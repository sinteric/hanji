# hanji crates

The Rust workspace for the Document type, docx only. The spec is
[DESIGN.md](../DESIGN.md): §4 architecture, §5.1–5.2 format, §8 safety,
§9 verification, §10 decisions. None of the library crates does filesystem,
network or thread I/O: bytes in, bytes out. They build and pass their tests on
native targets and on wasm32 (§10.4).

| Crate | What |
|---|---|
| `hanji-format` | The text format (§5.1, §5.2). A typed AST (flat inline units with marks, plus link and field spans), a parser that keeps a source map for exact-span edits, a canonical serializer, and validator errors written for the model (line, column, expected form, allowed names) |
| `hanji-core` | The `Engine` trait (§7). The resolved model and style set. The remainder store (§10.3): a flat list of typed entries, with the namespace map stored once. Re-anchoring: a port of difflib, block alignment, design C's document-level diff, and the exact-span edit API. Entries are refused when they would land in two paragraphs or on one of several identical blocks |
| `hanji-docx` | The docx engine. Splits `word/document.xml` at the XML level (a lossless tree over quick-xml) and copies every other part through byte for byte. §8: removes and reports active and remote content on import, and lists what to surface before export. `examples/dump.rs` prints a file's model text |

rdocx is not on the import/export path. Its typed model (`CT_P`, `CT_RPr`,
hyperlinks as run spans) does not expose the original `pPr`/`rPr` fragments,
bookmarks and wrappers that the remainder stores. It remains a candidate for
rendering, which is a stub for now.

## Tests

```sh
cargo test --workspace
HANJI_REPORT=1 cargo test --release -p hanji-docx --test corpus -- --nocapture   # numbers per file and edit
HANJI_SOFFICE=1 cargo test --release -p hanji-docx --test corpus -- --nocapture  # also convert every export with LibreOffice
cargo build --target wasm32-unknown-unknown --workspace
# the same tests on wasm32 (wasmtime, or `pip install wasmtime` and the bundled runner):
CARGO_TARGET_WASM32_WASIP1_RUNNER=scripts/wasi-run.py cargo test --target wasm32-wasip1 --workspace
```

The corpus tests (`hanji-docx/tests/corpus.rs`) read the prototype's 13 files
from `prototype/remainder/corpus/`. They port its E1–E10 edits, oracle and
scoring, and also run on wasm32-wasip1: the runner preopens the workspace.
The LibreOffice conversion runs natively only.

## Deferred

- §10.2: tracked changes. `<ins>`, `<del>` and comments stay placeholders.
- §10.5: header, footer and footnote text. Those parts are copied through, and
  `[^1]`, links, `<field>` and `$math$` parse but the docx engine refuses them
  on export.
- §10.8 leftovers: `gridBefore`/`gridAfter`, nested tables and row-level
  content controls stay block placeholders (4 of the corpus's 29 tables).
- §4 rule 5: rendering and preview.
- §9 validity in Word: exports are checked for well-formed XML and a
  LibreOffice PDF conversion only.
- An embedded macro-enabled workbook behind a chart (`.xlsm` in
  `word/embeddings`) is not neutralised; only OLE objects, ActiveX, `vbaProject`,
  fetching fields and external non-hyperlink relationships are.
- Lists (`- `, `1. `) are not in §5.2 yet; the parser rejects them with a hint.
