#!/usr/bin/env python3
"""Keeps the sinteric/rhwp and sinteric/rdocx forks in step with upstream.

    scripts/sync-forks.py status [--json]
    scripts/sync-forks.py merge FORK
    scripts/sync-forks.py check FORK
    scripts/sync-forks.py pr-body FORK
    scripts/sync-forks.py pin FORK REV [--also PKG@VER]...
    scripts/sync-forks.py watch

The daily scheduled task (docs/fork-sync.md) drives these. Only git is used,
so `status` is cheap and does nothing on a day upstream has not moved.

- status: for each fork, whether upstream `main` is already merged into the
  fork's `main`, whether an unmerged `sync/upstream-main-*` branch already
  carries it, and how Hanji's pin (Cargo.toml on Hanji's origin/main) relates
  to the fork's `main`. Prints one `action` per fork.
- merge: branches `sync/upstream-main-YYYYMMDD` off the fork's origin/main and
  merges upstream `main` into it. Never rebases. Exit 2 leaves the merge in
  progress with its conflicts listed, for a person or agent to resolve.
- check: the fork's own quick checks (FORKS below). Hanji's CI on the pin
  bump is the full gate. Results go to .git/sync-forks-check.json.
- pr-body: the sync PR's description: upstream range, the fork-only commits
  the merge keeps, files both sides changed, and the check results.
- pin: points every Hanji workspace dependency on the fork at REV and updates
  Cargo.lock for them (and for any --also package a version conflict names).
- watch: checks the rhwp checkout for the known upstream bugs in WATCHED that
  hold a Hanji change, by rendering their inputs the way hanji-preview does.
  Prints one line per bug, `fixed` or `still present`.

A fork's checkout is $HANJI_SYNC_<NAME>_DIR, else a sibling of this
repository (../rhwp, ../rdocx), cloned there when missing. For a rehearsal,
HANJI_SYNC_NO_FETCH=1 uses the refs already in that checkout.
"""
import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

FORKS = {
    "rhwp": {
        "fork": "sinteric/rhwp",
        "upstream": "edwardkim/rhwp",
        # Hanji builds rhwp without default features. The two test modules
        # are the fork's own contracts: TAC spacing at page starts and the
        # Rust 1.99 lint fixes. rhwp compiles its tests into generated suites,
        # so `suite:` names a module and check() finds the suite that holds it.
        "checks": [
            ["cargo", "check", "--locked", "--lib", "--no-default-features"],
            "suite:issue_598_footnote_marker_nav",
            "suite:hanji_rust199_compatibility",
        ],
    },
    "rdocx": {
        "fork": "sinteric/rdocx",
        "upstream": "tensorbee/rdocx",
        # The crates Hanji links, and the hash harness that gates every
        # rdocx PR (its CLAUDE.md). The rdocx and rpptx facades are compiled,
        # not tested: their unit tests take 45 minutes on 4 cores and need
        # rdocx CI's pinned Poppler and HarfBuzz tools. Tests named for the
        # pinned corpus are skipped, since its download hosts are not all
        # reachable from a sandbox. `harness` passes when
        # only the entries in harness_deltas differ from upstream's baseline.
        "checks": [
            ["cargo", "fmt", "--all", "--check"],
            ["cargo", "test", "--locked", "-p", "oxml-layout", "-p", "rdocx-layout",
             "-p", "rpptx-layout", "-p", "rpptx-render", "--", "--skip", "corpus"],
            ["cargo", "check", "--locked", "-p", "rdocx", "-p", "rpptx", "--all-targets"],
            "harness",
        ],
        # The fork's patches change the feature_showcase PDF (seen at #24, the
        # S96 sync, and at main 571ebef). Upstream's own main matches its
        # baseline. A new entry here needs a reason.
        "harness_deltas": {"feature_showcase:pdf/bytes", "feature_showcase:pdf/pages"},
    },
}

SYNC_PREFIX = "sync/upstream-main-"

# Upstream rhwp bugs a Hanji change waits on. Each renders `input` (relative to
# Hanji's root) with hanji-preview's rhwp calls and counts rhwp's table
# overlaps on `page` (1-based). The bug is fixed when that count is zero.
WATCHED = [
    {
        "name": "hy-002 inline table overlaps the next paragraph",
        "holds": "sinteric/hanji#155 (rhwp 0.9.1 bump)",
        "input": "prototype/preview/baseline/29-hwpx-edited-hy-002-e10.hwpx",
        "page": 2,
    },
]

PROBE_MAIN = r"""
fn main() {
    let path = std::env::args().nth(1).expect("input");
    let bytes = std::fs::read(&path).expect("read input");
    let document = rhwp::document_core::DocumentCore::from_bytes(&bytes).expect("open");
    for index in 0..document.page_count() {
        document
            .render_page_svg_layer_with_profile_native(index, rhwp::paint::RenderProfile::Screen)
            .expect("render");
        println!("{} {}", index + 1, document.take_table_overlaps().len());
    }
}
"""


def git(repo, *args, check=True):
    out = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)
    if check and out.returncode != 0:
        sys.exit(f"git {' '.join(args)} failed in {repo}:\n{out.stderr.strip()}")
    return out.stdout.strip()


def is_ancestor(repo, a, b):
    return subprocess.run(["git", "-C", repo, "merge-base", "--is-ancestor", a, b]).returncode == 0


def url(slug):
    return f"https://github.com/{slug}"


def fork_dir(name):
    path = os.environ.get(f"HANJI_SYNC_{name.upper()}_DIR") or os.path.join(os.path.dirname(ROOT), name)
    if not os.path.isdir(os.path.join(path, ".git")):
        subprocess.run(["git", "clone", "--quiet", url(FORKS[name]["fork"]), path], check=True)
    return path


def fetch(name):
    """Fetches the fork's main and sync branches and upstream main; returns the fork dir."""
    repo = fork_dir(name)
    if os.environ.get("HANJI_SYNC_NO_FETCH"):
        return repo
    git(repo, "fetch", "--quiet", "--prune", "origin", "+refs/heads/main:refs/remotes/origin/main",
        f"+refs/heads/{SYNC_PREFIX}*:refs/remotes/origin/{SYNC_PREFIX}*")
    git(repo, "fetch", "--quiet", url(FORKS[name]["upstream"]), "+refs/heads/main:refs/remotes/upstream/main")
    return repo


def hanji_pins(name):
    """The revs Hanji's origin/main pins for the fork, by package."""
    git(ROOT, "fetch", "--quiet", "origin", "+refs/heads/main:refs/remotes/origin/main")
    toml = git(ROOT, "show", "origin/main:Cargo.toml")
    pattern = re.compile(r'^([\w-]+) = \{ git = "' + re.escape(url(FORKS[name]["fork"])) + r'", rev = "([0-9a-f]+)"', re.M)
    return dict(pattern.findall(toml))


def status(name):
    repo = fetch(name)
    up = git(repo, "rev-parse", "refs/remotes/upstream/main")
    main = git(repo, "rev-parse", "refs/remotes/origin/main")
    merged = is_ancestor(repo, up, main)
    pending = []
    for line in git(repo, "for-each-ref", "--format=%(refname:short) %(objectname)",
                    f"refs/remotes/origin/{SYNC_PREFIX}*").splitlines():
        branch, sha = line.split()
        if not is_ancestor(repo, sha, main):
            pending.append({"branch": branch.removeprefix("origin/"), "head": sha,
                            "has_upstream": is_ancestor(repo, up, sha)})
    pins = hanji_pins(name)
    revs = set(pins.values())
    pin = revs.pop() if len(revs) == 1 else None
    if pin is None:
        pin_state = "inconsistent"
    else:
        git(repo, "fetch", "--quiet", "origin", pin, check=False)
        # A fork commit that only touches its own CI (.github/) changes nothing Hanji builds.
        same = subprocess.run(["git", "-C", repo, "diff", "--quiet", pin, "refs/remotes/origin/main",
                               "--", ".", ":(exclude).github"]).returncode == 0
        if same:
            pin_state = "current"
        elif is_ancestor(repo, pin, main):
            pin_state = "behind"
        elif any(is_ancestor(repo, pin, p["head"]) for p in pending):
            pin_state = "on-sync-branch"
        else:
            pin_state = "off-main"
    if merged:
        action = "none"
    elif any(p["has_upstream"] for p in pending):
        action = "review-sync-pr"
    else:
        action = "merge-upstream"
    base = git(repo, "merge-base", up, main)
    return {
        "fork": FORKS[name]["fork"], "upstream": FORKS[name]["upstream"],
        "upstream_main": up, "fork_main": main, "upstream_merged": merged,
        "upstream_new_commits": int(git(repo, "rev-list", "--count", "--first-parent", f"{base}..{up}")),
        "pending_sync_branches": pending, "hanji_pin": pin, "hanji_pins": pins,
        "hanji_pin_state": pin_state, "action": action,
        "bump_hanji_pin": pin_state in ("behind", "off-main", "inconsistent"),
    }


def cmd_status(args):
    report = {name: status(name) for name in FORKS}
    if args.json:
        print(json.dumps(report, indent=2))
        return
    for name, s in report.items():
        print(f"{name}: action={s['action']} upstream {s['upstream_main'][:9]} "
              f"({'merged' if s['upstream_merged'] else str(s['upstream_new_commits']) + ' new'}), "
              f"fork main {s['fork_main'][:9]}, Hanji pin {str(s['hanji_pin'])[:9]} {s['hanji_pin_state']}")
        for p in s["pending_sync_branches"]:
            print(f"  unmerged {p['branch']} {p['head'][:9]}{' (has upstream main)' if p['has_upstream'] else ''}")


def cmd_merge(args):
    name = args.fork
    repo = fetch(name)
    if git(repo, "status", "--porcelain", "--untracked-files=no"):
        sys.exit(f"{repo} has uncommitted changes")
    stamp = datetime.date.today().strftime("%Y%m%d")
    taken = set(git(repo, "for-each-ref", "--format=%(refname:short)", "refs/remotes/origin/").split())
    branch, n = SYNC_PREFIX + stamp, 2
    while f"origin/{branch}" in taken:
        branch, n = f"{SYNC_PREFIX}{stamp}-{n}", n + 1
    up = git(repo, "rev-parse", "refs/remotes/upstream/main")
    git(repo, "checkout", "--quiet", "-B", branch, "refs/remotes/origin/main")
    message = f"Merge {FORKS[name]['upstream']} main {up[:9]} into {branch}"
    out = subprocess.run(["git", "-C", repo, "merge", "--no-ff", "-m", message, "refs/remotes/upstream/main"],
                         capture_output=True, text=True)
    print(f"branch {branch}")
    if out.returncode != 0:
        conflicts = git(repo, "diff", "--name-only", "--diff-filter=U").splitlines()
        if not conflicts:
            sys.exit(out.stdout + out.stderr)
        print("conflicts:\n" + "\n".join(f"  {c}" for c in conflicts))
        print("Resolve them, keep the fork's patches, then `git commit --no-edit` in " + repo)
        sys.exit(2)
    print(f"merged cleanly: {git(repo, 'rev-parse', 'HEAD')}")


def suite_test(repo, module):
    """The cargo test command for one module of rhwp's generated test suites."""
    subprocess.run(["node", "scripts/rust-test-suite-manifest.mjs", "--prepare"], cwd=repo, check=True,
                   stdout=subprocess.DEVNULL)
    generated = os.path.join(repo, "tests", "generated")
    for file in sorted(os.listdir(generated)):
        if file.endswith(".rs") and re.search(rf"^mod {module};$", open(os.path.join(generated, file)).read(), re.M):
            return ["cargo", "test", "--locked", "--test", file[:-3], f"{module}::"]
    sys.exit(f"no generated suite holds the test module {module}")


def harness(repo, env, known):
    """rdocx's hash harness, passing when only the fork's known entries differ."""
    out = subprocess.run(["python3", "scripts/hash_harness.py", "--check"], cwd=repo, env=env,
                         capture_output=True, text=True)
    print(out.stdout[-2000:] + out.stderr, flush=True)
    if out.returncode == 0:
        return 0
    if "output delta detected" not in out.stderr:
        return out.returncode
    changed = set(re.findall(r"^  - \w+: (\S+)", out.stderr, re.M))
    unknown = changed - known
    if unknown:
        print("new output deltas beyond the fork's known ones: " + ", ".join(sorted(unknown)))
        return 1
    print("only the fork's known output deltas: " + ", ".join(sorted(changed)))
    return 0


def cmd_check(args):
    name = args.fork
    repo = fork_dir(name)
    env = dict(os.environ)
    # rdocx's CI runs its tests with this stack; some debug-build tests need it.
    env.setdefault("RUST_MIN_STACK", "8388608")
    # As in Hanji's CI: no debug info or incremental cache, which keeps a
    # fork's target directory to a few GB in a sandbox with a disk allowance.
    for key in ("CARGO_INCREMENTAL", "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG"):
        env.setdefault(key, "0")
    env.setdefault("CARGO_TARGET_DIR", os.path.join(os.path.expanduser("~"), ".cache", "hanji-sync-target", name))
    results, failed = [], False
    for command in FORKS[name]["checks"]:
        start = time.time()
        if command == "harness":
            command = ["python3", "scripts/hash_harness.py", "--check"]
            print("$ " + " ".join(command), flush=True)
            code = harness(repo, env, FORKS[name]["harness_deltas"])
        else:
            if isinstance(command, str):
                command = suite_test(repo, command.removeprefix("suite:"))
            print("$ " + " ".join(command), flush=True)
            code = subprocess.run(command, cwd=repo, env=env).returncode
        results.append({"command": " ".join(command), "exit": code, "seconds": round(time.time() - start)})
        failed |= code != 0
    head = git(repo, "rev-parse", "HEAD")
    with open(os.path.join(repo, ".git", "sync-forks-check.json"), "w") as f:
        json.dump({"head": head, "results": results}, f, indent=2)
    for r in results:
        print(f"{'ok  ' if r['exit'] == 0 else 'FAIL'} {r['seconds']:>5}s  {r['command']}")
    sys.exit(1 if failed else 0)


def cmd_pr_body(args):
    name = args.fork
    repo = fork_dir(name)
    up_slug = FORKS[name]["upstream"]
    head = git(repo, "rev-parse", "HEAD")
    up = git(repo, "rev-parse", "refs/remotes/upstream/main")
    old = git(repo, "merge-base", "refs/remotes/origin/main", up)
    upstream_log = git(repo, "log", "--first-parent", "--format=%h %s", f"{old}..{up}").splitlines()
    upstream_total = git(repo, "rev-list", "--count", f"{old}..{up}")
    kept = git(repo, "log", "--no-merges", "--reverse", "--format=%h %s", f"{up}..{head}").splitlines()
    upstream_files = set(git(repo, "diff", "--name-only", old, up).splitlines())
    fork_files = set(git(repo, "diff", "--name-only", up, head).splitlines())
    both = sorted(upstream_files & fork_files)
    lines = [
        f"Merges `{up_slug}` `main` at [`{up[:9]}`]({url(up_slug)}/commit/{up}) into the fork. "
        f"Upstream range: [`{old[:9]}..{up[:9]}`]({url(up_slug)}/compare/{old}...{up}), "
        f"{len(upstream_log)} first-parent commits ({upstream_total} in all). Merge only, no rebase.",
        "",
        "Merge this with a merge commit, not squash, so the Hanji pin to this branch's head stays on `main`.",
        "",
        "## Upstream changes",
        "",
        *[f"- {c}" for c in upstream_log[:40]],
        *([f"- ... and {len(upstream_log) - 40} more"] if len(upstream_log) > 40 else []),
        "",
        f"## Fork commits kept ({len(kept)})",
        "",
        *[f"- {c}" for c in kept],
        "",
        f"## Files changed on both sides ({len(both)})",
        "",
        *([f"- `{f}`" for f in both] or ["None. The fork patches and this upstream range do not touch the same files."]),
        "",
        "## Checks",
        "",
    ]
    report = os.path.join(repo, ".git", "sync-forks-check.json")
    data = json.load(open(report)) if os.path.exists(report) else None
    if data and data["head"] == head:
        lines += ["| Result | Time | Command |", "|---|---|---|"]
        lines += [f"| {'pass' if r['exit'] == 0 else 'FAIL'} | {r['seconds']}s | `{r['command']}` |" for r in data["results"]]
    else:
        lines.append("Not run at this head.")
    lines += ["", "Hanji's CI on the matching pin bump PR in sinteric/hanji is the full gate."]
    print("\n".join(lines))


def cmd_pin(args):
    name, rev = args.fork, args.rev
    if not re.fullmatch(r"[0-9a-f]{40}", rev):
        sys.exit("REV must be a full 40-character commit SHA")
    path = os.path.join(ROOT, "Cargo.toml")
    text = open(path).read()
    pattern = re.compile(r'^([\w-]+)( = \{ git = "' + re.escape(url(FORKS[name]["fork"])) + r'", rev = ")[0-9a-f]+"', re.M)
    packages = pattern.findall(text)
    if not packages:
        sys.exit(f"no workspace dependency on {FORKS[name]['fork']} in Cargo.toml")
    open(path, "w").write(pattern.sub(lambda m: f'{m.group(1)}{m.group(2)}{rev}"', text))
    update = ["cargo", "update"]
    for package in [p for p, _ in packages] + args.also:
        update += ["-p", package]
    env = dict(os.environ)
    # git's own client fetches the large rhwp repository much faster.
    env.setdefault("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
    if subprocess.run(update, cwd=ROOT, env=env).returncode != 0:
        sys.exit("cargo update failed. If it names a version conflict, rerun with --also NAME@VERSION "
                 "for the crate it names (the smallest update that resolves it).")
    lock = open(os.path.join(ROOT, "Cargo.lock")).read()
    stale = set(re.findall(re.escape(url(FORKS[name]["fork"])) + r"\?rev=([0-9a-f]+)", lock)) - {rev}
    if stale:
        sys.exit(f"Cargo.lock still has {FORKS[name]['fork']} at {', '.join(stale)}")
    print(f"pinned {', '.join(p for p, _ in packages)} to {rev}")
    print("Update the comment above the pins in Cargo.toml to say what the new rev carries.")


def cmd_watch(args):
    """Renders each WATCHED input with the rhwp checkout and reports its overlaps."""
    repo = fork_dir("rhwp")
    probe = os.path.join(os.path.expanduser("~"), ".cache", "hanji-sync-probe")
    os.makedirs(os.path.join(probe, "src"), exist_ok=True)
    with open(os.path.join(probe, "Cargo.toml"), "w") as f:
        f.write('[package]\nname = "hanji-sync-probe"\nversion = "0.0.0"\nedition = "2021"\n\n'
                f'[dependencies]\nrhwp = {{ path = {json.dumps(repo)}, default-features = false }}\n\n[workspace]\n')
    with open(os.path.join(probe, "src", "main.rs"), "w") as f:
        f.write(PROBE_MAIN)
    env = dict(os.environ)
    for key in ("CARGO_INCREMENTAL", "CARGO_PROFILE_DEV_DEBUG"):
        env.setdefault(key, "0")
    env.setdefault("CARGO_TARGET_DIR", os.path.join(os.path.expanduser("~"), ".cache", "hanji-sync-target", "rhwp"))
    head = git(repo, "rev-parse", "HEAD")
    for bug in WATCHED:
        out = subprocess.run(["cargo", "run", "--quiet", "--manifest-path", os.path.join(probe, "Cargo.toml"),
                              "--", os.path.join(ROOT, bug["input"])], cwd=repo, env=env,
                             capture_output=True, text=True)
        if out.returncode != 0:
            print(f"error   {bug['name']}: probe failed at rhwp {head[:9]}\n{out.stderr[-2000:]}")
            continue
        counts = dict(line.split() for line in out.stdout.splitlines())
        overlaps = int(counts.get(str(bug["page"]), "0"))
        state = "fixed" if overlaps == 0 else "still present"
        print(f"{state:<13} {bug['name']} (page {bug['page']}: {overlaps} table overlaps, rhwp {head[:9]}); "
              f"holds {bug['holds']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("status")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_status)
    for command, func in (("merge", cmd_merge), ("check", cmd_check), ("pr-body", cmd_pr_body)):
        p = sub.add_parser(command)
        p.add_argument("fork", choices=sorted(FORKS))
        p.set_defaults(func=func)
    p = sub.add_parser("pin")
    p.add_argument("fork", choices=sorted(FORKS))
    p.add_argument("rev")
    p.add_argument("--also", action="append", default=[], metavar="PKG@VER")
    p.set_defaults(func=cmd_pin)
    p = sub.add_parser("watch")
    p.set_defaults(func=cmd_watch)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
