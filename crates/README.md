# hanji crates

The Rust workspace for the Document type (docx, hwpx) and the Presentation
type (pptx). The spec is [DESIGN.md](../DESIGN.md): §4 architecture, §5.1–5.3 format, §8 safety,
§9 verification, §10 decisions. None of the library crates does filesystem,
network or thread I/O: bytes in, bytes out. They build and pass their tests on
native targets and on wasm32 (§10.4).

| Crate | What |
|---|---|
| `hanji-format` | The text format (§5.1–5.3). A typed AST (flat inline units with marks, plus link and field spans), a parser that keeps a source map for exact-span edits, a canonical serializer, and validator errors written for the model (line, column, expected form, allowed names). `pres.rs` reads and writes Presentations: `layout:` lines, `::slot::` markers checked against the file's layouts, `<shape>` lines |
| `hanji-core` | The `Engine` trait (§7). The resolved model and style set. The remainder store (§10.3): a flat list of typed entries, with the namespace map stored once. Re-anchoring: a port of difflib, block alignment, design C's document-level diff, and the exact-span edit API. Entries are refused when they would land in two paragraphs or on one of several identical blocks. `Block::Head` anchors slides, slots and shapes (`presentation.rs`); the `TextModel` trait runs edits and re-anchoring for either grammar, and design C pairs a head with where most of its content went |
| `hanji-core` (lists) | `plan_lists`: which list definition and level each list item takes at export (§5.2), behind a small `ListDefs` trait each engine implements |
| `hanji-package` | Plumbing the XML engines share: zip parts read and written with their metadata, a lossless XML tree over quick-xml with the canonical form GetPut compares, and OPC helpers (`opc.rs`: relationships, targets, reachable parts, content types) |
| `hanji-docx` | The docx engine. Splits `word/document.xml` at the XML level and copies every other part through byte for byte. Numbered paragraphs are list items; `numbering.rs` reads their kind and gives new items and lists their numbering. §8: removes and reports active and remote content on import, and lists what to surface before export. `examples/dump.rs` prints a file's model text |
| `hanji-hwpx` | The hwpx engine (OWPML, KS X 6101). Splits `Contents/section*.xml` at the XML level and copies every other part through byte for byte, `header.xml` too unless the text needs a new character or paragraph shape. Paragraph styles by their own (Korean) names, outline styles as `#`, bold/italic/underline/strikeout from the character shapes, tables with merges and multi-paragraph cells, side-by-side tables as separate blocks (§10.8), bullets and numbering as list items, tracked changes as read-only placeholders (§10.2). §8: removes scripts, embedded OLE objects and linked files. `examples/dump_hwpx.rs` prints a file's model text |
| `hanji-pptx` | The pptx engine. Splits each slide and notes page at the XML level into a skeleton (with stand-ins for the placeholders and text shapes the text shows) and text entries, and copies every other part through byte for byte. Layout names and slots come from the layouts' placeholders (`deck.rs`), bullets from the inheritance chain up to the master text styles. Slides can be added from a layout, deleted (their notes and unreachable parts go; a slide something else links to is refused) and moved; `sldIdLst`, sections, rels and content types follow. §8 (`safety.rs`): macros, ActiveX, OLE objects (their preview picture stays), program and macro click actions, linked media, images and objects, other external relationships. `examples/dump_pptx.rs` prints a file's model text |
| `hanji-testkit` | The corpus harness the engines run (not published): the prototype's E1–E10 edits (P1–P9 for pptx), oracle and scoring, GetPut, PutGet and well-formedness |

rdocx is not on the import/export path. Its typed model (`CT_P`, `CT_RPr`,
hyperlinks as run spans) does not expose the original `pPr`/`rPr` fragments,
bookmarks and wrappers that the remainder stores. It remains a candidate for
rendering, which is a stub for now. rpptx is not on the pptx path for the same
reason: it rewrites the parts it opens through its typed model. Adding a slide
from a layout, the one thing it would have saved, is done at the XML level.

rhwp is not on the hwpx import/export path either: it parses into its own
document model and writes the package from it, which drops what the model
does not hold (tracked changes, without a loss report). `hanji-hwpx/validate`
uses it to re-open every export.

## Tests

```sh
cargo test --workspace
HANJI_REPORT=1 cargo test --release -p hanji-docx --test corpus -- --nocapture   # numbers per file and edit
HANJI_REPORT=1 cargo test --release -p hanji-hwpx --test corpus -- --nocapture   # the same for hwpx
HANJI_REPORT=1 cargo test --release -p hanji-pptx --test corpus -- --nocapture   # the same for pptx (edits P1–P9)
HANJI_SOFFICE=1 cargo test --release -p hanji-docx --test corpus -- --nocapture  # also convert every export with LibreOffice
HANJI_SOFFICE=1 cargo test --release -p hanji-pptx --test corpus -- --nocapture  # the same, checking PDF page count against the slide count
cargo build --target wasm32-unknown-unknown --workspace
# rhwp re-opens every hwpx export the corpus and engine tests wrote (native, not part of the workspace):
cargo run --release --manifest-path crates/hanji-hwpx/validate/Cargo.toml
# the same tests on wasm32 (wasmtime, or `pip install wasmtime` and the bundled runner):
CARGO_TARGET_WASM32_WASIP1_RUNNER=scripts/wasi-run.py cargo test --target wasm32-wasip1 --workspace
```

The corpus tests run the shared harness (`hanji-testkit`): the docx ones on
the prototype's 13 files in `prototype/remainder/corpus/`, the hwpx ones on
the 16 files in `hanji-hwpx/corpus/` (sources and licences in its
SOURCES.md), the pptx ones on the 21 decks in `hanji-pptx/corpus/` (its
SOURCES.md; `fetch.sh` checks their SHA-256). They port the prototype's E1–E10 edits, oracle and scoring, and
also run on wasm32-wasip1: the runner preopens the workspace. The LibreOffice
conversion runs natively only, and the rhwp check is a separate crate
(`hanji-hwpx/validate`) so the workspace does not build rhwp.

## Deferred

- §10.2: tracked changes. `<ins>`, `<del>` and comments stay placeholders.
- §10.5: header, footer and footnote text. Those parts are copied through, and
  `[^1]`, links, `<field>` and `$math$` parse but the docx engine refuses them
  on export.
- §10.8 leftovers: `gridBefore`/`gridAfter`, nested tables and row-level
  content controls stay block placeholders (4 of the corpus's 29 tables, 40
  of the table survey's 506 docx tables). Zero-width markers between rows,
  cells or cell paragraphs are remainder entries and do not block a table.
- §4 rule 5: rendering and preview.
- §9 validity in Word: exports are checked for well-formed XML and a
  LibreOffice PDF conversion only.
- Lists inside table cells and multi-paragraph list items (§5.2): a numbered
  cell paragraph stays an ordinary cell paragraph, and a continuation paragraph
  an ordinary paragraph; their numbering stays in the remainder.

hwpx, in addition:

- 누름틀 (click-here) and other fields: their begin and end controls are
  durable markers and the value is ordinary editable text; `<field name=…>`
  (§5.1) is not produced or accepted yet.
- Footnote and endnote bodies, header and footer text stay inside their
  placeholders or controls (§10.5).
- Merging existing cells is refused: hwpx has no cell under a merge, so the
  covered cell's properties would go. Merges in new rows are written.
- A new table takes the file's first table's layout and a new cell its
  nearest cell's properties; a file without a table cannot get one.
- A table that shares its paragraph with text stays a placeholder, so the
  text stays visible; spaces after a table do not count. Nested tables and
  tables with content after their rows (`hp:label`) stay placeholders.
- XML comments and processing instructions inside a section are refused on
  import, as in docx.
- §9 validity in Hancom: exports are re-opened with rhwp only; LibreOffice
  24.2 cannot open hwpx.
- HWP 5.0 (`.hwp`): rhwp can convert it to hwpx (all 25 of rhwp's Hancom
  `.hwp` samples then import and round-trip), but that puts rhwp's document
  model on the import path and exports hwpx, not hwp.

pptx, in addition:

- Pictures, charts, tables, SmartArt and groups that are not placeholders are
  not shown in the text (they stay in the slide's skeleton); a placeholder
  holding one is a slot whose content the text keeps as a placeholder.
- Text in shapes is `<shape>` paragraphs only: their bullets stay in the
  remainder, and bulleted shape paragraphs are not list items.
- Moving a text shape to another slide keeps its text but not what refers to
  its slide's relationships (hyperlinks, pictures in the text); moving such
  text is refused. Deleting a slide another slide, a custom show or a
  notes page links to is refused, as is deleting a shape an animation plays on.
- A new slide takes its layout's placeholders without geometry, as
  PowerPoint does; slides are added only from the file's layouts.
- Comments, hidden slides and shapes and document metadata are surfaced, not
  editable.
