# hanji — start here

- **What hanji is and how it is built** (the format, the architecture, the
  engines, safety and the open decisions) → [`DESIGN.md`](DESIGN.md).
- **Rust style** — the Google Rust Style Guide is the base; every rule in it is
  kept, changed or excluded in one table, each change and exclusion with its
  reason → [`STYLE.md`](STYLE.md).
- **Formatting** runs a pinned nightly rustfmt: `python3 scripts/fmt.py`, and
  CI checks it with `--check` ([`STYLE.md`](STYLE.md), "Use the formatting
  tool").
