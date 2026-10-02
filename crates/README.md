# hanji crates

The Rust workspace for the Document type (docx, hwpx), the Presentation
type (pptx) and the Spreadsheet type (xlsx). The spec is [DESIGN.md](../DESIGN.md): §4 architecture, §5.1–5.4 format, §8 safety,
§9 verification, §10 decisions. None of the library crates does filesystem,
network or thread I/O: bytes in, bytes out. They build and pass their tests on
native targets and on wasm32 (§10.4).

| Crate | What |
|---|---|
| `hanji-format` | The text format (§5.1–5.3). A typed AST (flat inline units with marks, plus link and field spans), a parser that keeps a source map for exact-span edits, a canonical serializer, and validator errors written for the model (line, column, expected form, allowed names). `pres.rs` reads and writes Presentations: `layout:` lines, `::slot::` markers checked against the file's layouts, `<shape>` lines. `sheet.rs` reads and writes a Spreadsheet's structure (`<sheet>`, `<table>` blocks, `<chart/>`, placeholders) and writes the row window; `ops.rs` reads the JSON range operations and checks them against the structure; `formula.rs` tokenizes Excel formulas (structured references in short and file form, the §8 checks); `chars.rs` names invisible, whitespace-variant, private-use and control characters in errors, and finds where an `old` would match but for them. Formatting (§5.1, §5.2 F2): `vocab.rs` spells the vocabulary's values, `props.rs` is a property set in canonical key order, `styled.rs` resolves style lines and the lifting rules; the parser reads the style section, `{…}` and `[text]{…}` into per-element properties that differ from the style, and the serializer lifts them back |
| `hanji-core` | The `Engine` trait (§7). The resolved model and style set. The remainder store (§10.3): a flat list of typed entries, with the namespace map stored once. Re-anchoring: a port of difflib, block alignment, design C's document-level diff, and the exact-span edit API. Entries are refused when they would land in two paragraphs or on one of several identical blocks. `Block::Head` anchors slides, slots and shapes (`presentation.rs`); the `TextModel` trait runs edits and re-anchoring for either grammar, and design C pairs a head with where most of its content went. `cells.rs`: cell-range anchors (A1 ranges, range lists, where a range goes when rows are inserted into or deleted from a span of columns) |
| `hanji-core` (revisions) | `revision.rs`: `Kept`, what an edit's alignment keeps unchanged (units, paragraph marks, tables, placeholders); maps compose over a chain of edits. The docx tracked-change export writes the rest as inserted or deleted |
| `hanji-core` (lists) | `plan_lists`: which list definition and level each list item takes at export (§5.2), behind a small `ListDefs` trait each engine implements |
| `hanji-package` | Plumbing the XML engines share: zip parts read and written with their metadata, a lossless XML tree over quick-xml with the canonical form GetPut compares, and OPC helpers (`opc.rs`: relationships, targets, reachable parts, content types) |
| `hanji-docx` | The docx engine. Splits `word/document.xml` at the XML level and copies every other part through byte for byte. Numbered paragraphs are list items (a list is one numId and one kind at the margin); `numbering.rs` reads their kind and gives new items and lists their numbering. A table without a `{style}` line has the style a new table is written with: the default table style, or Table Grid when that draws no borders. Formatting (§5.2 F2, `format.rs`): the style section from `styles.xml` and the theme, paragraph, run and cell properties as they differ from the style, and a table's own `w:jc`/`w:tblInd` as `table-align`/`table-indent`, written back child by child; style-line edits and new styles rewrite `styles.xml`. §8: removes and reports active and remote content on import, and lists what to surface before export. Tracked changes (§10.2, `track.rs`): an export option, off by default, that writes the edits since import as `w:ins`/`w:del` in Word's shapes, from the exact edit spans (a `History` of the edits) or a design C rewrite whose alignment needed no choice among identical blocks; what it cannot write faithfully is refused with the reason. `examples/dump.rs` prints a file's model text |
| `hanji-hwpx` | The hwpx engine (OWPML, KS X 6101). Splits `Contents/section*.xml` at the XML level and copies every other part through byte for byte, `header.xml` too unless the text needs a new character or paragraph shape. Paragraph styles by their own (Korean) names, outline styles as `#`, bold/italic/underline/strikeout from the character shapes, tables with merges and multi-paragraph cells, side-by-side tables as separate blocks (§10.8), bullets and numbering as list items, tracked changes as read-only placeholders (§10.2). §8: removes scripts, embedded OLE objects and linked files. `examples/dump_hwpx.rs` prints a file's model text |
| `hanji-pptx` | The pptx engine. Splits each slide and notes page at the XML level into a skeleton (with stand-ins for the placeholders and text shapes the text shows) and text entries, and copies every other part through byte for byte. Layout names and slots come from the layouts' placeholders (`deck.rs`), bullets from the inheritance chain up to the master text styles. Objects that are not placeholders (pictures, charts, tables, groups) are `<keep/>` lines in z-order. Slides can be added from a layout, deleted (their notes and unreachable parts go; a slide something else links to is refused) and moved; `sldIdLst`, sections, rels and content types follow. §8 (`safety.rs`): macros, ActiveX, OLE objects (their preview picture stays), program and macro click actions, linked media, images and objects, other external relationships. `examples/dump_pptx.rs` prints a file's model text |
| `hanji-xlsx` | The xlsx engine (§5.4). The structure (sheets, tables, column types, formats and formulas, placeholders) is text; cells are read through row windows (`view.rs`) and written by range operations (`ops.rs`). Each worksheet is a skeleton plus its rows as byte ranges, parsed only when read or changed (`store.rs`); merged cells, conditional formats, data validations and hyperlinks are range entries (`hanji_core::cells`). Values as displayed (`numfmt.rs`). Cell formatting (round 6 part D, `format.rs`): each cell style's font, fill, border and alignment in the vocabulary, shown as read-only `<format range …/>` lines (one per rectangle of equal cells, `hanji_format::cellfmt`) and a `<format default …/>` line for Normal, and written by the `format` operation as copies of the cells' styles with the written keys changed. Row shifts move formulas, tables, entries, defined names, drawings, notes and pivot sources, and refuse what they would tear (`shift.rs`). Cached values: IronCalc computes the formulas whose inputs changed, over the rows they read, and writes only their `<v>`; what it cannot compute keeps its value and the file asks to be recalculated when opened (`calc.rs`). §8 (`safety.rs`): macros and macro sheets, external links, DDE, OLE, ActiveX, connections and query tables, fetching formulas. `examples/dump_xlsx.rs` prints a file's structure and windows, `examples/apply_ops.rs` applies an operation list |
| `hanji-store` | Documents and their revisions, and the operations of the CLI (§4): open, new, read (partial views by line range, section or slide range, and row windows, §2 rule 11), exact-span edits and whole-file rewrites against a known revision, range operations, validate, export behind the §8 surface list, re-import of a person's edits (rule 7, an edit against the revision before it merged by lines or refused), history and diff. Each revision is its model text and its remainder; storage is a small key–value trait (`MemStorage`, and `FsStorage` on native builds). The blank packages a new file starts from are in `blank/`. Errors are written for the model and reuse the validator's. `guide.md` is the format summary a model reads |
| `hanji-preview` | The preview (§2 rules 4–5, §7, §7.1), PPTX and bounded XLSX windows: the bytes a revision exports, rendered by rpptx 0.12.1 to one SVG per slide, an HTML viewer and PNG (resvg). `prep.rs`: engine-compat transforms on the preview's copy (an empty theme `a:ea` takes the Hangul face, runs split where the script changes, `p:timing` dropped). `fonts.rs`: each requested face looked up in `--font-dir`/`HANJI_FONT_DIR`, the deck's embedded fonts, system fonts and the faces compiled in, by name, then by `fonts/aliases.toml` (metric twin, substitutes, class). `sfnt.rs`: the face rpptx lays out with, renamed per request and given a 1.2 em line. `subset.rs`: fonts cut per page (allsorts, `cmap` kept). The `fonts` report: substituted, drawn as requested, missing glyphs. `render_pptx_with_fonts`/`render_pptx_with_resolver`: byte-only native/WASM document jobs; `render_page`: SVG/PNG buffers with diagnostics. `store.rs` and host font discovery are optional native adapters (`host-fonts`, enabled by default) |
| `hanji-cli` | The `hanji` binary: each operation as a subcommand, text for people or `--json`; `hanji preview` writes the preview, `hanji guide` prints `guide.md`. Native only. Packaged with a skill as a Claude Code and Codex plugin in [`plugins/hanji`](../plugins/hanji/README.md) |
| `hanji-testkit` | The corpus harness the engines run (not published): the prototype's E1–E10 edits (P1–P9 for pptx), oracle and scoring, GetPut, PutGet and well-formedness |

rdocx is not on the import/export path. Its typed model (`CT_P`, `CT_RPr`,
hyperlinks as run spans) does not expose the original `pPr`/`rPr` fragments,
bookmarks and wrappers that the remainder stores. It remains the candidate for
the docx preview. rpptx is not on the pptx import/export path for the same
reason: it rewrites the parts it opens through its typed model. Adding a slide
from a layout, the one thing it would have saved, is done at the XML level.
It renders the pptx preview (`hanji-preview`). `Engine::render` in hanji-core
stays a stub; the reusable preview library has its own document-byte/font-byte
entry points on native and wasm32 targets. Its optional `host-fonts` adapter
retains font directories, system discovery and stored-file output on native targets.

### Embedding preview

Use `hanji-preview` with `default-features = false` for a byte-only build.
`render_pptx_with_fonts(package, &FontOptions)` accepts caller-owned font bytes
and optional aliases, then considers package-embedded and bundled faces.
`FontData::face_index` selects a face in a font collection. A custom Rust
`FontResolver` can supply a complete selection/fallback policy through
`render_pptx_with_resolver`; resolver calls remain inside the document job.

```rust,ignore
use hanji_preview::{FontData, FontOptions, PageFormat};
let options = FontOptions { fonts: vec![FontData::new(font_bytes)], ..Default::default() };
let preview = hanji_preview::render_pptx_with_fonts(&package_bytes, &options)?;
let page = preview.render_page(0, PageFormat::Png { dpi: 144.0 })?;
// page.data is an owned PNG/SVG buffer; page.diagnostics has stable source paths.
```

Document creation and page output are coarse library jobs, with no CLI dependency.
Dropping the `Preview` releases the layout/font caches. The job is synchronous;
the embedding application chooses its worker, isolate or native bridge. No
platform ABI, FRB binding or Flutter SVG renderer is selected here. Existing
native `Options`, `render_pptx` and `store` remain available with the default
`host-fonts` feature. Invalid supplied fonts are configuration errors; renderer
fallbacks remain diagnostics. `render_page` validates the index, and PNG DPI
must be finite and positive.

`cargo run -p hanji-preview --no-default-features --example portable` exercises
document bytes, caller font bytes, SVG/PNG page buffers and diagnostics using
compiled-in test inputs. It runs on native and WASI without font files or
environment variables. Browser WASM still needs JavaScript glue from its
`wasm-bindgen` dependencies and an application adapter; a successful build or
WASI run alone does not verify browser/Flutter Web integration.

### XLSX worksheet windows

`hanji_preview::xlsx::open_xlsx(bytes, XlsxOptions)` reads the existing sparse
worksheet model. `Workbook::sheets` lists names, visibility and worksheet kind;
`render_window_with_fonts(sheet_index, "A1:H40", &FontOptions)` or
`render_window_with_resolver` returns one independently owned `Window`.
`Window::render(PageFormat)` uses the shared SVG/PNG/font pipeline and returns
all window diagnostics with the buffer. `Window::html` is a static standalone
viewer. The library has no filesystem, network, macro execution or formula
recalculation on this path. Native adapters in `hanji_preview::store` export
stored revisions or read an unstored file, select one bounded window, and
write its output. `hanji preview DOC --sheet NAME --range A1:H40` uses those
adapters; `--sheet-index N` is 1-based, while library/JSON indexes are 0-based.
CLI defaults select the first visible worksheet and `A1:L40`, and JSON includes
the sheet inventory, cells/cache status, dimensions, fonts and diagnostics.

```rust,ignore
use hanji_preview::{xlsx::{open_xlsx, XlsxOptions}, FontOptions, PageFormat};
let mut workbook = open_xlsx(&package_bytes, XlsxOptions::default())?;
let window = workbook.render_window_with_fonts(0, "A1:H40", &FontOptions::default())?;
let image = window.render(PageFormat::Png { dpi: 96.0 })?;
// window.cells carries displayed values and explicit formula-cache status.
// window.fonts()/diagnostics() are available before requesting output.
```

The first projection draws cell values/number formats, basic font/fill/border
styles, simple alignment, character wrapping, merged ranges and stored sizes.
Hidden rows/columns collapse; hidden sheets remain explicitly addressable.
Formula caches are always unverified; recalculation flags/manual mode identify
possibly stale caches. Missing caches show `#UNEVALUATED`, including shared
followers with no cache. Empty cached strings remain empty. IronCalc exists for
explicit editing/recalculation but preview does not call it.

Column width uses a 7-pixel maximum-digit approximation. Row auto-fit, complex
shaping/bidi, rich-text run styling, theme/indexed/tinted colors, named style
inheritance, East Asian advance overrides, gradients, advanced borders, charts/images, conditional formatting,
table styles, filters, freeze panes and print pagination are incomplete and
reported where applicable. A window that cuts a merged range is refused with
its address; choose a complete range. No Excel fidelity or interactive Flutter
grid is claimed. The `xlsx` example exercises a deterministic byte-only job.

Default configurable budgets: 64 MiB inflated package; 512 rows, 128 columns
and 32,768 grid cells per window; 2 MiB displayed text (headers included);
4,096 intersecting merged ranges; 16,777,216 PNG pixels. Violations are explicit
errors, with no truncation. Visual font names and number-format codes also have
configurable 1,024-byte and 4,096-byte metadata budgets. Cells at any valid Excel address are
accessible by windows; the declared full-sheet dimension does not allocate a
full grid. Sparse indexing/shared strings/XML/font originals still consume
memory, so these workload budgets are not a whole-process memory guarantee.
Fonts use the existing 16 MiB retained subset cache and pre-output diagnostics.
Drop windows and the workbook to release their independently owned resources.

umya-spreadsheet is not on the xlsx import/export path, for the same reason:
it reads a workbook into its model and writes every part again from it. The
engine does the package, the XML and the cells itself; IronCalc (0.7.1: later
versions need JavaScript's time zone on every wasm32 target) is used as a
calculator only, over cells the engine hands it, and never reads or writes
the file.

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
# tracked changes (§10.2): the corpus test above also writes target/tmp/docx-tracked-out; cross-check it outside the workspace:
cargo run --release --manifest-path crates/hanji-docx/validate/Cargo.toml       # rdocx's accept / reject by author
python3 crates/hanji-docx/validate/lo_review.py                                 # LibreOffice's Accept All / Reject All (UNO)
# §9 validity by hand: the Office check kit (target/office-kit/ and target/office-kit.zip, with CHECKLIST.md):
cargo run --release --manifest-path validation/office-kit/Cargo.toml
HANJI_SOFFICE=1 cargo test --release -p hanji-pptx --test corpus -- --nocapture  # the same, checking PDF page count against the slide count
HANJI_REPORT=1 cargo test --release -p hanji-xlsx -- --nocapture                 # xlsx: corpus, op set, round 4, the 100k-row sheet
HANJI_SOFFICE=1 cargo test --release -p hanji-xlsx -- --nocapture                # also compare LibreOffice's values with the windows
HANJI_TEST_FONT_DIR=DIR cargo test -p hanji-preview   # the Korean preview tests need a Hangul font (else /usr/share/fonts; skipped without one, off CI)
cargo build --target wasm32-unknown-unknown --workspace --exclude hanji-cli
# rhwp re-opens every hwpx export the corpus and engine tests wrote (native, not part of the workspace):
cargo run --release --manifest-path crates/hanji-hwpx/validate/Cargo.toml
# the same tests on wasm32 (wasmtime, or `pip install wasmtime` and the bundled runner):
CARGO_TARGET_WASM32_WASIP1_RUNNER=scripts/wasi-run.py cargo test --target wasm32-wasip1 --workspace --exclude hanji-cli
```

The corpus tests run the shared harness (`hanji-testkit`): the docx ones on
the prototype's 13 files in `prototype/remainder/corpus/`, the hwpx ones on
the 16 files in `hanji-hwpx/corpus/` (sources and licences in its
SOURCES.md), the pptx ones on the 21 decks in `hanji-pptx/corpus/` (its
SOURCES.md; `fetch.sh` checks their SHA-256), the xlsx ones on the 46
workbooks in `hanji-xlsx/corpus/` (the same). They port the prototype's E1–E10 edits, oracle and scoring (docx adds the
formatting edits F1–F4: a first-line indent, a restyled heading style, a
new style, a filled header row), and
also run on wasm32-wasip1: the runner preopens the workspace. The LibreOffice
conversion runs natively only, and the rhwp check is a separate crate
(`hanji-hwpx/validate`) so the workspace does not build rhwp.

## Deferred

- §10.2: tracked changes are read as placeholders (`<ins>`, `<del>`, comments)
  and written only by the docx export option. It refuses: deleting or moving
  a placeholder other than a picture, symbol or special character in a run
  (fields, other authors' tracked changes, note and comment references,
  content controls, kept tables), text inside a field's result, inserting or
  deleting a section break, changing a table's rows, merges, style or
  position (`table-align`, `table-indent`), an
  inserted or deleted paragraph with no paragraph after it to join, and a
  design C rewrite that chose among identical blocks. Moves are written as a
  deletion and an insertion, not `w:moveFrom`/`w:moveTo`. hwpx stays on
  direct changes. Word itself has not opened these files yet (§9).
- §10.5: header, footer and footnote text. Those parts are copied through, and
  `[^1]`, links, `<field>` and `$math$` parse but the docx engine refuses them
  on export.
- §10.8 leftovers: `gridBefore`/`gridAfter`, nested tables and row-level
  content controls stay block placeholders (4 of the corpus's 29 tables, 40
  of the table survey's 506 docx tables). Zero-width markers between rows,
  cells or cell paragraphs are remainder entries and do not block a table.
- §4 rule 5: the docx/hwpx preview and XLSX print/advanced visual fidelity. The pptx preview has no live
  reload, no bundled Korean font pack (§7.1) and no font policy yet.
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
- A table's own position (`table-align`, `table-indent` on its table line,
  docx `w:jc` and `w:tblInd`) is refused with the reason, as table styles
  are.
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

xlsx, in addition:

- Charts, pictures, shapes, notes and pivot tables are `<keep/>` lines: they
  can be deleted, and their anchors and data ranges follow row shifts, but a
  new `<chart>` line is refused. Pivot tables whose source changed refresh
  when the file is opened.
- The structure text cannot rename or delete a sheet, table or column, or
  change a table's range; range operations do the rest.
- Operations that would tear a merged area, an array formula, a data table,
  a filter or a pivot table are refused with the reason, as are values that
  break a data validation. Validations the engine cannot check (custom, date
  and time rules, bounds or lists that are formulas) refuse the write under
  their default error style and are reported under `warning` and
  `information`. Column inserts and deletes are not operations.
- 1904-date workbooks: formulas whose inputs changed are left to the
  application (IronCalc knows the 1900 system only).
- Rich text in new cells, cell styles beyond a column's number format, and
  threaded comments.

pptx, in addition:

- Pictures, charts, tables, SmartArt, groups and ink that are not
  placeholders are `<keep/>` lines among the slots, in z-order (rule 8): the
  text can move them within their slide or delete them (their parts go when
  nothing else uses them; refused when an animation plays on one), never
  change or create them. Connectors and shapes without text stay in the
  slide's skeleton unshown.
- Design C refuses (orphans, with the reason "ambiguous alignment") a slide
  it cannot place from the text: an empty slide, or one none of whose text
  is left, when a new slide could be it.
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
