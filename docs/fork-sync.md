# Fork sync

Hanji renders previews with two forks: [sinteric/rhwp](https://github.com/sinteric/rhwp)
(from [edwardkim/rhwp](https://github.com/edwardkim/rhwp)) and
[sinteric/rdocx](https://github.com/sinteric/rdocx) (from
[tensorbee/rdocx](https://github.com/tensorbee/rdocx), which also holds rpptx and
oxml-layout). Each fork is upstream `main` plus a small set of Hanji patches.
A scheduled task brings upstream `main` into each fork once a day, and then
moves Hanji's pins in `Cargo.toml` to the result.

[`scripts/sync-forks.py`](../scripts/sync-forks.py) does the git work. Its
docstring has the commands. This page is the procedure, and the daily task
follows it.

## Every day

1. `python3 scripts/sync-forks.py status --json`. It reads only git refs and
   takes a few seconds. Each fork gets one `action`:
   - `none`: upstream `main` is already in the fork's `main`.
   - `merge-upstream`: upstream moved and no sync branch carries it yet.
   - `review-sync-pr`: an unmerged `sync/upstream-main-*` branch already has it.
     Its PR waits for review, so do nothing more for that fork.

   `bump_hanji_pin` is true when Hanji's pin is behind the fork's `main` (or off
   it). If every fork is `none` and no pin needs a bump, stop there.
2. Before opening anything, list the open PRs in the fork and in
   sinteric/hanji. Do not open a second sync PR for a fork, or a pin PR while
   another open Hanji PR already changes that fork's pins.

## When upstream moved

1. `python3 scripts/sync-forks.py merge FORK` branches
   `sync/upstream-main-YYYYMMDD` off the fork's `main` and merges upstream
   `main` into it. Merge only. Never rebase, never force-push.
2. Exit 2 means conflicts. Resolve each file so that upstream's change and the
   fork patch both survive. The fork patches are listed by `pr-body` under
   "Fork commits kept". Read the patch's own commit and test to see what it
   protects. When upstream fixed the same problem, prefer upstream's code and
   keep the fork's test. Then `git commit --no-edit`.
3. `python3 scripts/sync-forks.py check FORK` runs the fork's quick checks.
   Fix what breaks in a follow-up commit on the same branch.
4. Push the branch and open a PR to the fork's `main` with the body from
   `python3 scripts/sync-forks.py pr-body FORK`. Add a "Conflict resolution"
   section that says, for each conflicted file, what was kept from each side.
   If a check still fails, open it as a draft and say what fails.
5. For rhwp, `python3 scripts/sync-forks.py watch` with the sync branch
   checked out. It renders the inputs of the upstream bugs that hold a Hanji
   change (`WATCHED` in the script) and prints `fixed` or `still present` for
   each. Put the result in the fork PR, and tell Ryan when one turns `fixed`,
   naming the Hanji change it holds.
6. In Hanji, `python3 scripts/sync-forks.py pin FORK <sync branch head SHA>`,
   update the comment above the pins, run `cargo clippy --locked --workspace
   --all-targets -- -D warnings`, and fix Hanji for API changes in the same
   PR. If `cargo update` reports a version conflict, rerun `pin` with
   `--also NAME@VERSION` for the crate it names. Open the PR as
   `Bump FORK to <short SHA> (upstream sync)`, linking the fork PR. Hanji's CI
   on this PR is the full gate.

Skip the pin PR while another open Hanji PR moves the same fork's pin (step 2
of "Every day"). Today sinteric/hanji#155 does that for rhwp and waits on the
hy-002 bug that `watch` checks.

The pin points at the sync branch head, not at the fork's `main`, so Hanji's
CI tests the merge before anyone merges it. Merge the fork PR with a merge
commit (not squash): the pinned commit is then part of the fork's `main`, and
`status` reports the pin as current.

## When only the pin is behind

A fork PR merged without an upstream sync (a new Hanji patch, for example)
leaves Hanji's pin behind. Open the same Hanji pin PR with the fork's `main`
head, as in step 6.

## Disk

A cloud sandbox has a fixed disk allowance. `check` builds without debug info
(about 1.5 GB for rdocx). Cargo's git checkouts of rhwp grow large, so after a
pin, delete `~/.cargo/git/checkouts` if space is short. `~/.cache/hanji-sync-target`
is a build cache and is safe to delete.

## Never

- Merge a PR, push to `main`, or delete a branch. A person merges.
- Rebase or force-push a sync branch.
- Change the fork patches beyond what a conflict needs.

## The fork checks

| Fork | Checks (`FORKS` in the script) | Cold, 4 cores |
|---|---|---|
| rhwp | `cargo check --lib --no-default-features` (how Hanji builds it), and the fork's two contract test modules (TAC spacing at page starts, the Rust 1.99 lint fixes), found in rhwp's generated suites | about 6 min |
| rdocx | `cargo fmt --check`, the tests of oxml-layout, rdocx-layout, rpptx-layout and rpptx-render (corpus tests skipped), a build of the rdocx and rpptx facades, and the hash harness | about 4 min |

The rdocx facade unit tests are left out: they take about 45 minutes and need
the pinned Poppler and HarfBuzz tools of rdocx's own CI. The hash harness
passes when only `harness_deltas` differ from upstream's baseline. Today that
is the `feature_showcase` PDF, which the fork patches change. A new delta
fails the check and needs a reason before it joins the list.

The same checks run in each fork's `hanji-fork-checks.yml` workflow when
GitHub Actions are on for that fork. Keep the two lists in step.

## Commit messages

Follow each repository's own rules. rdocx forbids `Co-Authored-By` trailers,
em-dashes and prose semicolons in commit messages.
