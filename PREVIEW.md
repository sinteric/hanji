# Preview support

Hanji previews the bytes its selected revision would export. File-path previews
stay unstored and leave the input file unchanged. `--rev N` selects a stored
revision; a file path is previewed as supplied. All formats below offer HTML,
SVG and PNG. A successful render does not establish the design's native
application fidelity target.

| Input | Native CLI | Portable library | Fidelity and limits |
|---|---|---|---|
| PPTX | `hanji preview deck.pptx --format png --out DIR` | `render_pptx_with_fonts`, `render_pptx_with_resolver`, or `render_document_with_fonts(DocumentKind::Pptx, ...)` | Every slide. Groups retain engine line breaks/origins; ordinary glyphs use the selected drawing font and preserve compatible authored spacing. Rich group runs retain their original font/cluster positioning with a warning. Charts and other advanced visuals can be placeholders or approximations. Rendering diagnostics identify reported omissions. |
| DOCX | `hanji preview report.docx --format svg --out DIR` | `docx::render_with_fonts` or `render_document_with_fonts(DocumentKind::Docx, ...)` | Experimental, every page. Paragraphs, tables, images, headings, sections and headers/footers are exercised by actual files. Word pagination, vertical text, OfficeMath and legacy VML features can differ or be omitted. |
| HWPX | `hanji preview plan.hwpx --format html --out DIR` | `hwpx::render_with_fonts` or `render_document_with_fonts(DocumentKind::Hwpx, ...)`, with feature `hwpx` | Experimental, every page. rhwp supplies source/table geometry, images and controls; Hanji selects drawing fonts and embeds subsets. Substitution does not trigger exact-font reflow. Table overflow/overlap counters are reported; the integration does not have comprehensive diagnostics for every unsupported Hancom control. |
| XLSX/XLSM | `hanji preview book.xlsx --sheet NAME --range A1:H40 --format png --out DIR` | `xlsx::open_xlsx`, then `render_window_with_fonts` / `render_window_with_resolver` | One bounded worksheet window, not printed pages. Cached formula values only. No formula calculation, macros or external-link refresh. Charts, images, conditional formatting and print layout are omitted with diagnostics. |
| Binary HWP, HML, legacy DOC/PPT/XLS, ODF | No preview entry point | No document-kind variant | These input formats are outside this preview integration. HWPX is a ZIP/XML format, not binary HWP. |

rhwp itself can open binary HWP and HML. Hanji's HWPX adapter explicitly
requires the ZIP/XML header, and its import/store format model excludes those
inputs. A future preview-only adapter could use rhwp's parsers; that would
require separate input routing and validation rather than removing a renderer
page limit or adding editing support.

Output names retain `<doc>-r<rev>-slide-N` for PPTX and use
`<doc>-r<rev>-page-N` for DOCX/HWPX. HTML has one viewer file. The JSON result
retains `slides` for PPTX and adds `pages`; DOCX/HWPX omit `slides`. XLSX names
include the sheet number and canonical window range.

## Library jobs and dependencies

`render_document_with_fonts(kind, document_bytes, &FontOptions)` returns an
owned `DocumentPreview`. It exposes `page_count`, `page_info`, `render_page`,
`html`, `fonts`, `warnings` and `diagnostics`. `render_page` returns an owned
SVG string or PNG buffer and stable page diagnostics. Indexes are zero-based;
PNG DPI must be finite and positive. The native CLI is an adapter over these
library jobs. Existing PPTX entry points remain available.

The default features are `host-fonts` and `hwpx`. Use
`default-features = false` for PPTX/DOCX/XLSX jobs with caller font bytes and
bundled fallbacks; add `features = ["hwpx"]` for portable HWPX. The byte APIs
do not discover host fonts. Native `Options` can load `--font-dir`,
`HANJI_FONT_DIR` and installed fonts. Korean text needs an appropriate font;
the output reports actual missing characters when one is unavailable.

PPTX/DOCX share one immutable git source for rdocx 0.14.0 and the rpptx /
oxml-layout 0.12.1 family. HWPX uses rhwp 0.8.6 from the sinteric fork.
`Cargo.toml` records exact revision pins and `Cargo.lock` records the resolved
source. A crate's version alone does not identify these fork patches.

Native and WASM compile/runtime checks have separate purposes. Browser WASM
requires JavaScript glue for wasm-bindgen dependencies and an application
adapter; compilation or a WASI test does not validate browser or Flutter Web
integration. No platform bridge is shipped by this change.
Local release validation uses Rust 1.99.0. The declared crate minimum Rust
versions are retained and were not separately validated by these runs.

## Resource guards and renderer limitations

PPTX/DOCX/HWPX have no product page-count or image-size cap. They retain the
complete layout or SVG pages and fonts in memory. PNG output must fit valid
raster dimensions and available memory. HWPX output preparation obtains every
page from rhwp; it is not incremental. PPTX timing is removed on a preview
copy: previews are static, and animations are not played.

PPTX/DOCX validate font subsets and SVG lowering before output. Their 16 MiB
subset cache is a performance budget: uncached validated subsets are recreated
on demand, without refusing pages. HWPX prepares page and whole-viewer subsets.
A subset failure is a diagnostic; affected text can be missing. The missing
character list shows at most 100 distinct characters, while
`missing_glyphs_total` preserves the full distinct-character count. This is a
report-size limit, not a rendering limit.

XLSX default library budgets are 512 rows, 128 columns, 32,768 cells, 2 MiB of
window text and 16,777,216 PNG pixels. The CLI adds a 64 MiB raw/unpacked
package limit and defaults to the first visible worksheet and `A1:L40`.
Library callers can set budgets explicitly. Budget overruns refuse with a
reason rather than silently truncating. A window cutting a merged cell is
refused with the full merge address. These guards protect sparse-sheet and
raster allocations; removing them does not implement Excel print layout.

Inspect `fonts`, `warnings` and `diagnostics` before relying on a preview.
Diagnostics are nonfatal reported approximations, not a completeness guarantee
for all document features. Missing fonts, engine layout differences and
unsupported controls are renderer/integration work; accepting a format at the
CLI cannot fix their appearance.

## Validation evidence

The local release candidate rendered eight real DOCX/PPTX/XLSX corpus files
and four real HWPX files in all three output formats. Input hashes were checked
before and after. Visual inspection included Korean text, tables, images,
headers/footers, section/page breaks, headings, spacing and reported warnings.
The HWPX set includes an edited document and a newly created Korean plan.

These are actual renders, not fresh native Office comparisons. The connected
Mac's Word automation timed out before opening the temporary fixture, and no
native PDF was produced. The old spike's checked-in measurements remain
historical evidence: DOCX `05` had three native reference pages; the current
candidate renders five. HWPX `28` had seven native reference pages; the current
candidate renders six. The unavailable source PDFs cannot be rescored locally.
Those pagination differences prevent a blanket native-fidelity claim.

Observed limitations include missing Symbol/Gothic characters in DOCX `04`,
OfficeMath/VML diagnostics in DOCX `05`, an unsupported chart placeholder in
PPTX `12`, and clipping/missing formula caches in XLSX `18`. PPTX `14`, slide
6, now draws the 55/30/15 doughnut wedges in three theme colors and shows
구독, 라이선스 and 서비스 in the chart’s own legend. A real-file regression
checks the curved wedges and category labels separately from the slide’s
manual percentage legend. The legend’s Korean glyph IDs and 7.785-point advances now match its actual
9-point drawing font. Regressions cover mixed Latin/Korean labels, explicit
font/fallback cases and extra word spacing. Group line breaks/origins remain
engine geometry; rich multilingual clusters retain their original engine font
and positioning when substitution cannot preserve that contract. A coverage-based
Latin fallback restores digits and punctuation in HWPX `24`, reducing its
missing-character count from 17 to three (`U+FF62`, `U+FF63`, `U+FF65`). HWPX
`26` and `28` each retain four missing symbols; `30` has none. The drawing-font
correction preserves rhwp's geometry and page counts. These remaining issues
are acceptance evidence for keeping DOCX/HWPX experimental and addressing
targeted renderer issues next.

An additional DOCX regression exercises nine vertical table-alignment and
page-alignment combinations with tall headers and footers. It checks the
rendered table bounds against the page's usable story area. Vertical pages
currently reserve the largest applicable story band, which can leave extra
space when first/even/default headers or footers have different heights.

The real six-page HWPX footnote document `26` reuses clip IDs across pages
with different rectangle geometry. The HTML viewer namespaces page IDs and
local fragment references; a regression checks global ID uniqueness and
resolves each clip back to its page's original rectangle. Standalone SVG/PNG
pages retain their original identifiers and drawing behavior. Browser visual
validation of the combined viewer remains pending: this execution environment
exposes no supported browser preview workflow, and restricted file navigation
has not been bypassed.
