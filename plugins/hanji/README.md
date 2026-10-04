# hanji plugin for Claude Code and Codex

One plugin directory for both tools: the `office-documents` skill, which
tells the agent when to reach for hanji and how to work with it, and a
launcher for the `hanji` command line (`crates/hanji-cli`) that the agent
runs. The plugin has no MCP server.

| File | Read by |
|---|---|
| `.claude-plugin/plugin.json` | Claude Code |
| `.codex-plugin/plugin.json` | Codex (it prefers this over `.claude-plugin/`) |
| `skills/office-documents/SKILL.md` | both: the one copy of the skill |
| `skills/office-documents/scripts/hanji` | both: the launcher the skill runs |
| `../../.claude-plugin/marketplace.json` | both: the marketplace at the repository root |

## Install

Claude Code:

```sh
claude plugin marketplace add sinteric/hanji
claude plugin install hanji@hanji
```

or, in a session, `/plugin marketplace add sinteric/hanji` and then
`/plugin install hanji@hanji`. The skill is `/hanji:office-documents`.

Codex (CLI, IDE extension and app):

```sh
codex plugin marketplace add sinteric/hanji
codex plugin add hanji@hanji
```

The skill is `hanji:office-documents`.

## How the agent runs hanji

The skill tells the agent to run `sh <skill directory>/scripts/hanji
<subcommand> …`:

- Claude Code substitutes `${CLAUDE_SKILL_DIR}` in a skill's text with the
  skill's directory, so the skill names the launcher as
  `${CLAUDE_SKILL_DIR}/scripts/hanji`.
- Codex resolves a path that a skill gives relative to the directory of its
  `SKILL.md`; the skill tells the agent to use that directory where the
  variable is not filled in.

The plugin has no top-level `bin/`: Claude Code would put it on the Bash
tool's `PATH`, but claude.ai and Cowork refuse a plugin that has one.

## The binary

For a plugin version with published release assets, no Rust toolchain is
needed. The launcher runs, in order:

1. `$HANJI_BIN`, if set;
2. `hanji` on `PATH`, for example after
   `cargo install --locked --git https://github.com/sinteric/hanji hanji-cli`;
3. the release binary for this platform (Linux x86_64 and arm64, static;
   macOS x86_64 and arm64; Windows x86_64 under Git Bash), downloaded once
   from the GitHub release `v<version>` into
   `${XDG_CACHE_HOME:-~/.cache}/hanji/<version>/` and checked against the
   release's `SHA256SUMS`. It needs `curl` or `wget`, and `tar`.

The download happens on the first command, which waits for it (the launcher
says so on stderr); later commands run the cached binary at once. On failure,
the launcher prints the failed URL and guidance for its cause. HTTP 404
means the release or the requested asset is unavailable; other HTTP errors
retain their status. Connection failures keep the network-access guidance
(github.com and release-assets.githubusercontent.com). The skill tells the
agent to show that line to the person.

The release is made by `.github/workflows/release.yml` when a `v*` tag that
matches the plugin's version is pushed. A main-branch plugin can be ahead of
the published releases. Until the matching release assets are available,
use a source build or a plugin version with published assets. The launcher
keeps its version pin and checksum verification; it never downloads an older
version automatically.

For a source checkout matching the plugin, run from the repository root:

```sh
cargo build --locked -p hanji-cli
export HANJI_BIN="$PWD/target/debug/hanji"
sh plugins/hanji/skills/office-documents/scripts/hanji --version
```

On Windows under Git Bash, use `target/debug/hanji.exe`. Set `HANJI_BIN` in
the environment of the Claude Code or Codex process that will run the
launcher. The absolute path works even when commands run in another project.

Documents are kept in `.hanji/` in the directory the agent runs hanji from
(the project's), or in `$HANJI_STORE`. Paths are given to hanji as absolute
paths (the skill says so).

## Without the plugin

Install `hanji` (a release binary, or `cargo install` as above) and give the
agent the format with `hanji guide`; `hanji --help` lists the commands.

## Changing it

- Try a local checkout: `claude --plugin-dir plugins/hanji`, or
  `codex plugin marketplace add .` then `codex plugin add hanji@hanji`.
- `python3 scripts/test-launcher.py` checks download failures, the source
  build override, checksums and caching using fake curl/wget responses. It
  needs no Rust build or network access and runs in CI's `plugins` job.
- `scripts/check-plugins.py` (CI job `plugins`) checks that the manifests,
  the marketplace, the workspace version and the launcher agree, that the
  plugin ships no MCP server and no top-level `bin/`, and that the launcher
  runs the CLI of this version (`--version`, `guide`, new, read, export), in
  the checkout and in each copy Claude Code and Codex installed. Put a built
  `hanji` on `PATH` first (`cargo build -p hanji-cli`, then
  `PATH="$PWD/target/debug:$PATH"`), or set `HANJI_DOWNLOAD_BASE` to a
  URL (`file:///…` works) that holds release archives and their `SHA256SUMS`.
- A new version: bump `version` in `Cargo.toml`, both `plugin.json` files and
  `skills/office-documents/scripts/hanji` together (the check fails
  otherwise), merge, then push the tag `v<version>`.
