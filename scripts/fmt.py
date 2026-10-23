#!/usr/bin/env python3
"""Format the workspace with the pinned nightly rustfmt (STYLE.md, "Formatting").

`python3 scripts/fmt.py` rewrites files; `--check` only reports, as CI does.
The compiler stays on the stable toolchain in rust-toolchain.toml: nightly is
used for rustfmt alone, because the guide's import and literal options exist
only there.
"""

import subprocess
import sys

# The one place the formatter's toolchain is named. Bump it deliberately: a new
# rustfmt can reformat the tree.
TOOLCHAIN = "nightly-2026-10-22"


def main() -> int:
    subprocess.run(
        ["rustup", "toolchain", "install", TOOLCHAIN, "--profile", "minimal", "--component", "rustfmt"],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    cmd = ["cargo", f"+{TOOLCHAIN}", "fmt", "--all"]
    if "--check" in sys.argv[1:]:
        cmd += ["--", "--check"]
    return subprocess.run(cmd).returncode


if __name__ == "__main__":
    sys.exit(main())
