# hanji crates

Rust workspace for the first implementation slice (Document type, docx only).
See [DESIGN.md](../DESIGN.md) §4, §5, §8, §9, §10.

| Crate | What |
|---|---|
| `hanji-format` | The text format (§5.1, §5.2): typed AST, parser with source map, canonical serializer, validator errors for the model |

## Tests

```sh
cargo test --workspace
cargo build --target wasm32-unknown-unknown --workspace
# the same tests on wasm32 (needs wasmtime, or `pip install wasmtime` and the bundled runner):
CARGO_TARGET_WASM32_WASIP1_RUNNER=scripts/wasi-run.py cargo test --target wasm32-wasip1 --workspace
```
