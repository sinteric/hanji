#!/usr/bin/env python3
"""Checks the Claude Code and Codex plugin in plugins/hanji (CI runs it).

    scripts/check-plugins.py [--codex-mcp-json FILE]

- The manifests and the marketplace parse; their names and versions agree
  with each other, with the workspace version and with the launcher's
  version (it downloads that release).
- Each skill's SKILL.md has front matter whose name is its directory's.
- The MCP server starts from each plugin's configured command, as each tool
  resolves it (Claude Code: ${CLAUDE_PLUGIN_ROOT} substituted, the project as
  working directory; Codex: the plugin root as working directory, and only
  the environment Codex passes), and answers initialize and tools/list with
  the tools crates/hanji-mcp defines.
- With --codex-mcp-json, the same for the server config Codex itself
  resolved (`codex mcp get hanji --json` after installing the plugin).

The launcher picks hanji-mcp from PATH, $HANJI_MCP_BIN or the release
download, so put a built hanji-mcp on PATH (or set HANJI_MCP_DOWNLOAD_BASE)
before running this.
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import threading

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PLUGIN = os.path.join(ROOT, "plugins", "hanji")
# The variables Codex passes to a stdio MCP server (codex-rs/rmcp-client
# DEFAULT_ENV_VARS), besides the server's own env_vars.
CODEX_ENV = ["HOME", "LOGNAME", "PATH", "SHELL", "USER", "LANG", "LC_ALL", "TERM", "TMPDIR", "TZ"]

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


def launcher_version():
    text = open(os.path.join(PLUGIN, "scripts", "hanji-mcp"), encoding="utf-8").read()
    return re.search(r"^version=(\S+)$", text, re.M).group(1)


def expected_tools():
    src = open(os.path.join(ROOT, "crates", "hanji-mcp", "src", "main.rs"), encoding="utf-8").read()
    return set(re.findall(r"async fn (hanji_\w+)\(", src))


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
    ok(f"skills: {', '.join(names)}")


def handshake(label, command, args, cwd, env):
    """Starts the server and checks initialize and tools/list."""
    try:
        p = subprocess.Popen(
            [command, *args],
            cwd=cwd,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            # MCP is UTF-8 JSON; the locale's encoding (cp1252 on Windows) is not.
            encoding="utf-8",
        )
    except OSError as e:
        fail(f"{label}: cannot start {command} {args}: {e}")
        return
    err = []
    threading.Thread(target=lambda: err.extend(p.stderr), daemon=True).start()
    timer = threading.Timer(120, p.kill)
    timer.start()

    def call(id, method, params):
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": id, "method": method, "params": params}) + "\n")
        p.stdin.flush()
        for line in p.stdout:
            msg = json.loads(line)
            if msg.get("id") == id:
                return msg
        raise EOFError("the server closed stdout")

    try:
        init = call(1, "initialize", {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "check-plugins", "version": "1"},
        })
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        p.stdin.flush()
        tools = call(2, "tools/list", {})
    except (EOFError, ValueError, BrokenPipeError) as e:
        p.kill()
        p.wait()
        fail(f"{label}: {e}; stderr: {''.join(err).strip()}")
        return
    finally:
        timer.cancel()
    p.stdin.close()
    p.wait(timeout=30)
    name = init.get("result", {}).get("serverInfo", {}).get("name")
    got = {t["name"] for t in tools.get("result", {}).get("tools", [])}
    want = expected_tools()
    if name != "hanji" or got != want:
        fail(f"{label}: server {name!r}, tools {sorted(got)}; expected hanji with {sorted(want)}")
        return
    ok(f"{label}: initialize and tools/list answer {len(got)} tools")


def codex_env(server):
    if os.name == "nt":
        # Codex passes Windows' core variables there; not mirrored here.
        return dict(os.environ, **(server.get("env") or {}))
    names = CODEX_ENV + [v if isinstance(v, str) else v["name"] for v in server.get("env_vars") or []]
    env = {k: os.environ[k] for k in names if k in os.environ}
    env.update(server.get("env") or {})
    return env


def check_servers(codex_json):
    project = tempfile.mkdtemp(prefix="hanji-plugin-project-")
    store = tempfile.mkdtemp(prefix="hanji-plugin-store-")
    os.environ["HANJI_STORE"] = store

    claude = load("plugins/hanji/.mcp.json")["mcpServers"]["hanji"]
    sub = lambda s: s.replace("${CLAUDE_PLUGIN_ROOT}", PLUGIN)
    env = dict(os.environ, CLAUDE_PLUGIN_ROOT=PLUGIN)
    handshake("Claude Code .mcp.json", sub(claude["command"]), [sub(a) for a in claude.get("args", [])], project, env)

    codex = load("plugins/hanji/codex.mcp.json")["mcpServers"]["hanji"]
    cwd = os.path.normpath(os.path.join(PLUGIN, codex.get("cwd", ".")))
    handshake("Codex codex.mcp.json", codex["command"], codex.get("args", []), cwd, codex_env(codex))

    if codex_json:
        server = json.load(open(codex_json, encoding="utf-8"))["transport"]
        handshake("Codex (as `codex mcp get` resolved it)", server["command"], server.get("args") or [], server["cwd"], codex_env(server))


def main():
    codex_json = None
    argv = sys.argv[1:]
    if argv[:1] == ["--codex-mcp-json"]:
        codex_json = argv[1]
    elif argv:
        sys.exit(__doc__)
    check_static()
    check_servers(codex_json)
    if failures:
        sys.exit(f"{len(failures)} plugin check(s) failed")


if __name__ == "__main__":
    main()
