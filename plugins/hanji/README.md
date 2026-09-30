# hanji plugin for Claude Code and Codex

One plugin directory for both tools: the `hanji` MCP server (the ten
`hanji_*` tools of `crates/hanji-mcp`) and the `office-documents` skill,
which tells the agent when to reach for them and how to work with them.

| File | Read by |
|---|---|
| `.claude-plugin/plugin.json` | Claude Code |
| `.codex-plugin/plugin.json` | Codex (it prefers this over `.claude-plugin/`) |
| `.mcp.json` | Claude Code: `sh ${CLAUDE_PLUGIN_ROOT}/scripts/hanji-mcp` |
| `codex.mcp.json` | Codex: `sh ./scripts/hanji-mcp` with the plugin as working directory, since Codex does not substitute a plugin-root variable |
| `skills/office-documents/SKILL.md` | both: the one copy of the skill |
| `scripts/hanji-mcp` | both: the launcher |
| `../../.claude-plugin/marketplace.json` | both: the marketplace at the repository root |

## Install

Claude Code:

```sh
claude plugin marketplace add sinteric/hanji
claude plugin install hanji@hanji
```

or, in a session, `/plugin marketplace add sinteric/hanji` and then
`/plugin install hanji@hanji`. The skill is `/hanji:office-documents`, and
`/mcp` lists the plugin's `hanji` server.

Codex (CLI, IDE extension and app):

```sh
codex plugin marketplace add sinteric/hanji
codex plugin add hanji@hanji
```

`codex mcp list` then shows `hanji`, and the skill is `hanji:office-documents`.

## The server binary

No Rust toolchain is needed. The launcher runs, in order:

1. `$HANJI_MCP_BIN`, if set;
2. `hanji-mcp` on `PATH`, for example after
   `cargo install --locked --git https://github.com/sinteric/hanji hanji-mcp`;
3. the release binary for this platform (Linux x86_64 and arm64, static;
   macOS x86_64 and arm64; Windows x86_64 under Git Bash), downloaded once
   from the GitHub release `v<version>` into
   `${XDG_CACHE_HOME:-~/.cache}/hanji/<version>/` and checked against the
   release's `SHA256SUMS`. It needs `curl` or `wget`, and `tar`.

The release is made by `.github/workflows/release.yml` when a `v*` tag that
matches the plugin's version is pushed. Until the tag exists, use 1 or 2.

Documents are kept in `$HANJI_STORE`, else `~/.hanji`. Under Codex the
server's working directory is the plugin's directory, so paths are given to
the tools as absolute paths (the skill says so).

## Without the plugin

Any MCP client can run the server directly. Codex's `~/.codex/config.toml`:

```toml
[mcp_servers.hanji]
command = "hanji-mcp"
```

Claude Code: `claude mcp add hanji -- hanji-mcp`.

## Changing it

- Try a local checkout: `claude --plugin-dir plugins/hanji`, or
  `codex plugin marketplace add .` then `codex plugin add hanji@hanji`.
- `scripts/check-plugins.py` (CI job `plugins`) checks that the manifests,
  the marketplace, the workspace version and the launcher agree, and that the
  server answers `initialize` and `tools/list` from both tools' commands. Put a
  built `hanji-mcp` on `PATH` first.
- A new version: bump `version` in `Cargo.toml`, both `plugin.json` files and
  `scripts/hanji-mcp` together (the check fails otherwise), merge, then push
  the tag `v<version>`.
