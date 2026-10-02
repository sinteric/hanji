#!/usr/bin/env python3
"""Checks the Claude Code and Codex plugin in plugins/hanji (CI runs it).

    scripts/check-plugins.py [--installed PLUGIN_DIR]...

- The manifests and the marketplace parse; their names and versions agree
  with each other, with the workspace version and with the launcher's
  version (it downloads that release).
- Each skill's SKILL.md has front matter whose name is its directory's.
- The plugin ships no MCP server (no .mcp.json, no codex.mcp.json, no
  mcpServers in a manifest or the marketplace entry) and no top-level bin/
  (claude.ai and Cowork refuse a plugin that has one).
- The skill's launcher, run as the skill tells the agent to (`sh
  <skill dir>/scripts/hanji …` from the project's directory), runs the hanji
  CLI of this version: `--version`, `guide` (the text of
  crates/hanji-store/src/guide.md), and new, read and export of a document.
- With --installed, the same for the launcher in each installed copy of the
  plugin (the directory Claude Code or Codex installed it to).

The launcher picks hanji from $HANJI_BIN, PATH or the release download, so
put a built hanji on PATH (or set HANJI_DOWNLOAD_BASE) before running this.
"""
import json
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PLUGIN = os.path.join(ROOT, "plugins", "hanji")
LAUNCHER = os.path.join("skills", "office-documents", "scripts", "hanji")

failures = []


def fail(msg):
    failures.append(msg)
    print(f"FAIL {msg}", file=sys.stderr)


def ok(msg):
    print(f"ok   {msg}")


def load(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as f:
        return json.load(f)


def workspace_version():
    text = open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8").read()
    m = re.search(r'\[workspace\.package\][^\[]*?\nversion\s*=\s*"([^"]+)"', text)
    return m.group(1)


def launcher_version(plugin=PLUGIN):
    text = open(os.path.join(plugin, LAUNCHER), encoding="utf-8").read()
    return re.search(r"^version=(\S+)$", text, re.M).group(1)


def check_static():
    claude = load("plugins/hanji/.claude-plugin/plugin.json")
    codex = load("plugins/hanji/.codex-plugin/plugin.json")
    market = load(".claude-plugin/marketplace.json")
    version = workspace_version()
    for what, v in [
        ("Claude Code plugin.json", claude.get("version")),
        ("Codex plugin.json", codex.get("version")),
        ("launcher", launcher_version()),
    ]:
        if v != version:
            fail(f"{what} version {v} is not the workspace version {version}")
    if claude["name"] != "hanji" or codex["name"] != "hanji":
        fail("both plugin.json files name the plugin hanji")
    entries = [p for p in market["plugins"] if p["name"] == "hanji"]
    if len(entries) != 1 or os.path.normpath(os.path.join(ROOT, entries[0]["source"])) != PLUGIN:
        fail("marketplace.json lists hanji once, with source ./plugins/hanji")
    ok(f"manifests and marketplace agree on hanji {version}")

    mcp = [f for f in (".mcp.json", "codex.mcp.json") if os.path.exists(os.path.join(PLUGIN, f))]
    mcp += [what for what, m in [("Claude Code plugin.json", claude), ("Codex plugin.json", codex)] + [
        ("the marketplace entry", e) for e in entries
    ] if "mcpServers" in m]
    bin_dir = os.path.exists(os.path.join(PLUGIN, "bin"))
    if mcp:
        fail(f"the plugin ships no MCP server, but has: {', '.join(mcp)}")
    if bin_dir:
        fail("the plugin has a top-level bin/, which claude.ai and Cowork refuse")
    if not mcp and not bin_dir:
        ok("no MCP server and no top-level bin/")

    skills = os.path.join(PLUGIN, "skills")
    names = sorted(os.listdir(skills))
    if not names:
        fail("no skills")
    for name in names:
        text = open(os.path.join(skills, name, "SKILL.md"), encoding="utf-8").read()
        m = re.match(r"---\n(.*?)\n---\n", text, re.S)
        front = dict(line.split(": ", 1) for line in m.group(1).splitlines()) if m else {}
        if front.get("name") != name or not front.get("description"):
            fail(f"skills/{name}/SKILL.md: front matter needs name: {name} and a description")
    text = open(os.path.join(skills, "office-documents", "SKILL.md"), encoding="utf-8").read()
    if "${CLAUDE_SKILL_DIR}/scripts/hanji" not in text:
        fail("skills/office-documents/SKILL.md does not name its launcher as ${CLAUDE_SKILL_DIR}/scripts/hanji")
    ok(f"skills: {', '.join(names)}")


def check_launcher(label, plugin):
    """Runs the launcher as the skill tells the agent to, from a project directory."""
    launcher = os.path.join(plugin, LAUNCHER)
    project = tempfile.mkdtemp(prefix="hanji-plugin-project-")
    env = dict(os.environ)
    env.pop("HANJI_STORE", None)

    def run(*args, timeout=300):
        try:
            return subprocess.run(
                ["sh", launcher, *args],
                cwd=project,
                env=env,
                capture_output=True,
                encoding="utf-8",
                timeout=timeout,
            )
        except (OSError, subprocess.TimeoutExpired) as e:
            fail(f"{label}: sh {launcher} {' '.join(args)}: {e}")
            return None

    want = launcher_version(plugin)
    # The first run may download the release binary: give it the time.
    p = run("--version")
    if p is None:
        return
    if p.returncode != 0 or p.stdout.split() != ["hanji", want]:
        fail(f"{label}: `hanji --version` printed {p.stdout.strip()!r} (status {p.returncode}), expected 'hanji {want}'; stderr: {p.stderr.strip()}")
        return
    guide = open(os.path.join(ROOT, "crates", "hanji-store", "src", "guide.md"), encoding="utf-8").read()
    p = run("guide", timeout=60)
    if p is None:
        return
    if p.returncode != 0 or p.stdout != guide:
        fail(f"{label}: `hanji guide` (status {p.returncode}) does not print guide.md; stderr: {p.stderr.strip()}")
        return
    out = os.path.join(project, "new.docx")
    steps = [
        ("--json", "new", "document"),
        ("--json", "read", "{doc}"),
        ("--json", "export", "{doc}", out),
    ]
    doc = None
    for step in steps:
        args = [a.format(doc=doc) for a in step]
        p = run(*args, timeout=60)
        if p is None:
            return
        try:
            result = json.loads(p.stdout)
        except ValueError:
            result = {}
        if p.returncode != 0 or "error" in result:
            fail(f"{label}: `hanji {' '.join(args)}` (status {p.returncode}): {p.stdout.strip()} {p.stderr.strip()}")
            return
        doc = doc or result.get("doc_id")
    if not os.path.isfile(out) or not os.path.isdir(os.path.join(project, ".hanji")):
        fail(f"{label}: export wrote no {out}, or the store is not .hanji/ in the project's directory")
        return
    ok(f"{label}: the launcher runs hanji {want} (--version, guide, new, read, export)")


def main():
    installed = []
    argv = sys.argv[1:]
    while argv[:1] == ["--installed"] and len(argv) > 1:
        installed.append(argv[1])
        argv = argv[2:]
    if argv:
        sys.exit(__doc__)
    check_static()
    check_launcher("plugins/hanji", PLUGIN)
    for plugin in installed:
        check_launcher(f"installed copy {plugin}", plugin)
    if failures:
        sys.exit(f"{len(failures)} plugin check(s) failed")


if __name__ == "__main__":
    main()
