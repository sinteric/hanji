# hanji

hanji reads Word, PowerPoint, Excel and Hancom files (docx, pptx, xlsx,
hwpx) as a Markdown-like text, and writes your edits to that text back into
the original file. Anything the text does not show (formatting, pictures,
charts, comments, fields) is kept in the file. It is built for coding agents,
and it also works as a command-line tool.

Two lens laws hold for every file:

- **GetPut**: open a file and export it without an edit, and every part is
  XML-equivalent to the original.
- **PutGet**: after an edit, reading the file again shows exactly the text
  that was written.

When an edit cannot keep something, hanji refuses the edit and gives the
reason. It never drops content without saying so. Content it cannot edit
shows in the text as a `<keep/>` placeholder. You can move or delete a
placeholder, but you cannot change it or create a new one.

The design and its reasoning are in [DESIGN.md](DESIGN.md).

## Example

Open a file, read it, edit an exact span and export the result. The file is
`prototype/remainder/corpus/korean-report.docx`:

```console
$ hanji open korean-report.docx
opened korean-report.docx as korean-report (docx document), revision 1, 42 lines
surface before export: comment at comment 0: 박서준: 지방 수치 재확인 필요

$ hanji read korean-report --lines 1:30
---
type: document
format: docx
schema: 1
---
<style name="Normal" line-spacing=100% font="Noto Sans CJK KR" size=12pt color=#000000/>
<style name="Heading 1" size=16pt bold/>
…

# 3분기 영업 보고

매출은 전년 대비 **12%** 증가했다.<keep id="khc1l" kind="footnote" summary="footnote: 내부 집계 기준."/> 신규 고객은 34곳이며, 그중 [21곳이 수도권]{color=#C00000}이다.
…
<keep id="kml28" kind="drawing" summary="조직도, 상자 5개"/>

| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |

$ hanji edit korean-report --rev 1 --old '| ^^ | 종로 | 95 |' --new '| ^^ | 종로 | 98 |'
korean-report: revision 2 (from 1)

$ hanji export korean-report out.docx
error (surfaced_not_acknowledged): the file would carry content nobody may have reviewed (§8):
- comment at comment 0: 박서준: 지방 수치 재확인 필요
…

$ hanji export korean-report out.docx --acknowledge-surfaced
wrote out.docx (11726 bytes) from revision 2 of korean-report; digest fnv1a64:…
```

A placeholder that is not in the file is refused:

```console
$ hanji edit korean-report --rev 2 --old '<keep id="khc1l" kind="footnote" summary="footnote: 내부 집계 기준."/>' \
    --new '<keep id="zzzzz" kind="footnote" summary="x"/>'
error (invalid): the text is not valid:
line 14, column 24: placeholder id="zzzzz" is not in this file. Placeholders come from the file: keep, move or delete them, but never create one.
```

A Document can be exported in its other format, docx ↔ hwpx. The text and
what the target can write cross; the export lists what did not (§5.5):

```console
$ hanji export korean-report out.hwpx --format hwpx --acknowledge-surfaced
wrote out.hwpx (7821 bytes) from revision 2 of korean-report; digest fnv1a64:…
converted docx → hwpx. Not carried across:
- placeholders dropped, 4 (comment ×1, drawing ×1, footnote ×1, tracked-insert ×1):
    comment ku25a (line 18): comment: 지방 수치 재확인 필요
    …
- properties hwpx cannot write, dropped:
    table-align=left table-indent=0pt ×1 (line 26): hwpx has no table position (table-align, table-indent): the table sits where hwpx puts it
- styles hwpx has under its own definition (its values differ):
    Normal → 바탕글: align left → justify, line-spacing 100% → 160%, font Noto Sans CJK KR → 함초롬바탕, size 12pt → 10pt
    Heading 1 → 개요 1: size 16pt → 10pt, bold yes → no
    …
- what the file held beyond the text, left behind:
    …
```

Subcommands: `open`, `new`, `read`, `edit`, `write`, `ops`, `validate`,
`export`, `preview`, `reimport`, `history`, `diff`, `list`, `guide`.
`hanji preview DOC` writes an HTML viewer of the revision as it would
export. PPTX writes one slide per SVG/PNG; experimental DOCX and HWPX
write one page per SVG/PNG (`--format svg` or `png`) and says which fonts
it had to substitute; `--font-dir DIR` or `$HANJI_FONT_DIR` adds fonts. Rendering
fallbacks and font-embedding failures are warnings; `--json` also returns their
`diagnostics` with source paths. The preview library exposes the same diagnostics
before any output is requested. Add `--json` for machine-readable output.
Shared PPTX/DOCX output also preflights decoded PNG/JPEG image resources: the
defaults are 64 MiB per image/page and 1 GiB across the document. Images that
exceed a budget are omitted with diagnostics in SVG, HTML and PNG. Library
callers can configure these budgets; see [embedded image limits](crates/hanji-preview/IMAGE_LIMITS.md).
For XLSX, `hanji preview DOC --sheet '매출' --range A1:H40 --format png
--out /tmp/hanji-preview` writes one worksheet window. A `.xlsx`/`.xlsm`
file can be previewed directly without opening or storing it. Stored documents
use the exported bytes of the current revision, or `--rev N`; the original
file is not read again. The default selects the first visible worksheet and
`A1:L40`, regardless of the sheet's used range. Choose an exact name with
`--sheet NAME`, or a 1-based workbook sheet number with `--sheet-index N`;
explicit selection can include a hidden worksheet. JSON sheet indexes are
0-based, matching the library. Output names include the sheet number and
canonical range, e.g. `book-r1-sheet-1-A1-H40.png`.

XLSX windows show cached formula results without evaluating formulas or
executing macros or external links. JSON `cells` reports `formula_result` as
`not-formula`, `cached-unverified`, `cached-possibly-stale`, or `missing`;
missing results display `#UNEVALUATED`. Charts, images, conditional formatting,
print layout and other omitted features have diagnostics. Column widths and
text wrapping are approximate. Default library budgets are 512 rows, 128
columns, 32,768 cells, 2 MiB of window text and 16,777,216 PNG pixels. Clipped
cells have a diagnostic with their address; narrow numeric columns can hide
leading digits, while JSON `cells[].display` retains the complete value.
The CLI uses these budgets and a 64 MiB raw/unpacked package limit. A window cutting a
merged cell is refused with the complete merge address. Request a smaller
window on a budget refusal; there is no silent truncation. For example:

```sh
cargo run --locked -p hanji-cli -- preview crates/hanji-xlsx/corpus/korean-sales.xlsx --sheet '매출' --range A1:H20 --format svg --out /tmp/hanji-xlsx-svg
cargo run --locked -p hanji-cli -- --json preview crates/hanji-xlsx/corpus/korean-sales.xlsx --sheet-index 1 --range A1:H20 --format png --out /tmp/hanji-xlsx-png
cargo run --locked -p hanji-cli -- preview crates/hanji-xlsx/corpus/korean-sales.xlsx --range A1:H20 --format html --out /tmp/hanji-xlsx-html
```

PPTX, DOCX and HWPX render every page; there is no product page or image-size
cap. Reported engine omissions, actual missing glyphs and font subset
failures remain visible in warnings and JSON. DOCX pagination and vertical text can differ
from Word. HWPX retains rhwp source/table geometry rather than reflowing with
the selected drawing fonts. These previews do not establish the design
fidelity target. See [the preview support matrix](PREVIEW.md) for APIs,
limits, diagnostics and validation scope.

PPTX/DOCX diagnostic inspection does not serialize SVG or encode image data.
Unique page/viewer font subsets are validated before output; identical
character sets share buffers in a 16 MiB performance cache. Validated subsets
that do not fit are recreated when requested, without limiting output. HWPX
preparation obtains each SVG from rhwp and prepares page/viewer subsets. All
three retain their complete layout or page data in memory; preparation is
not incremental.
Documents and their revisions are kept in `.hanji/`,
or in the directory named by `--store` or `$HANJI_STORE`. `hanji guide` prints
the format summary that agents read.

## Install

**Release binaries.** Each [release](https://github.com/sinteric/hanji/releases)
has `hanji-<version>-<target>.tar.gz` with the `hanji` binary for Linux
x86_64 and arm64 (static, musl), macOS x86_64 and arm64, and Windows x86_64,
plus `SHA256SUMS`. (v0.1.0 also had `hanji-mcp`, an MCP server, which has
since been removed: agents run the CLI.)

**From source** (Rust 1.93 or later):

```sh
cargo install --locked --git https://github.com/sinteric/hanji hanji-cli
```

**Claude Code plugin** (the `office-documents` skill and a launcher for the
`hanji` command):

```sh
/plugin marketplace add sinteric/hanji
/plugin install hanji@hanji
```

**Codex plugin**:

```sh
codex plugin marketplace add sinteric/hanji
codex plugin add hanji@hanji
```

You do not need a Rust toolchain for a plugin version with published release
assets. A main-branch plugin may need a source build until its matching
release is published; see the `HANJI_BIN` steps in
[plugins/hanji](plugins/hanji/README.md#the-binary). The skill tells the agent
to run `hanji` commands through its launcher, which uses `hanji` from `PATH`
if it is there. Otherwise its first run downloads the release binary for your
platform once and checks it against `SHA256SUMS`; the command waits for the
download. Later runs use the cached binary. For details, see
[plugins/hanji](plugins/hanji/README.md).

## Commands

| Command | What it does |
|---|---|
| `hanji open FILE` | Open a file. Prints the doc id, revision 1 and the safety report (what was removed, what to show the person before export) |
| `hanji new TYPE` | Create a new document from a blank file or a template (`--format`, `--template`) |
| `hanji read DOC` | Print a revision's text, or a part of it: `--lines`, `--section`, `--slides`, or spreadsheet row windows (`--table` with `--rows`, `--sheet` with `--range`) |
| `hanji edit DOC --rev N` | Edit exact spans (`--old`/`--new`, `--old-file`/`--new-file`, or a JSON list with `--edits`). Each `old` must occur exactly once, and the edit must be against the current revision |
| `hanji write DOC --rev N FILE` | Replace the whole text. hanji aligns the old and new text to find where unshown content goes |
| `hanji ops DOC --rev N OPS` | Write spreadsheet cells with JSON range operations (set, append/insert/delete rows, fill_formula, sort, add_table, format, …) |
| `hanji validate FILE` | Check a text against the grammar, and with `--doc` the document's style, layout and placeholder names |
| `hanji export DOC PATH` | Write a revision to a file. Refused until surfaced content (comments, hidden text, metadata) is acknowledged (`--acknowledge-surfaced`); `--tracked-changes` for docx; `--format hwpx` or `docx` writes a Document in its other format and lists what did not cross |
| `hanji reimport DOC PATH --base-rev N` | Bring back a file edited in Office or Hancom only if its exported base revision is still current; refuse if newer revisions were committed |
| `hanji history DOC`, `hanji diff DOC A B` | List a document's revisions, and show the diff between two of them |
| `hanji preview DOC` | Render PPTX slides, experimental DOCX/HWPX pages or a bounded XLSX worksheet window to HTML, SVG or PNG, with font and rendering diagnostics |
| `hanji list`, `hanji guide` | List the stored documents; print the format summary for agents |

For re-import, keep the revision printed by `export` and pass that number as
`--base-rev`. This is the file's base, not a newly read current revision;
hanji does not embed or infer export provenance. Re-import replaces the text
and remainder. If the head has advanced, reconcile the file with a fresh
export first. `--replace-head` is the explicit alternative for deliberately
replacing the current contents, including already committed edits. Pending
text edits submitted after a re-import can still be merged or refused;
already committed edits are not automatically merged with an imported file.
Library callers should use `reimport_bytes_at_revision` or
`reimport_at_revision`; the original `reimport_bytes` and `reimport` methods
retain authoritative replacement semantics for compatibility.

## Formats

| | docx | hwpx | pptx | xlsx |
|---|---|---|---|---|
| Text model | Document | Document | Presentation | Spreadsheet: the structure as text, cells as row windows |
| Edit text | yes | yes | slides, slots, text shapes, notes | range operations |
| Styles and direct formatting | yes | yes | fills, outlines, effects, spans | cell formats; `format` op |
| Tables | yes, with merges | yes, with merges | `<keep/>` | sheet tables |
| Lists | yes | yes | bullets from layouts | — |
| Add, delete, move | blocks | blocks | slides (from layouts); objects: move, resize, delete | rows, tables, sheets |
| Pictures, charts, other objects | `<keep/>` | `<keep/>` | `<keep/>` | `<keep/>`, follow row shifts |
| Tracked changes | read; optional on export | read | — | — |
| Formulas | — | — | — | yes; changed formulas are recalculated |
| Removed on open | macros, ActiveX, OLE, fetching fields, external links | scripts, OLE, linked files | macros, ActiveX, OLE, program actions, external links | macros, external links, DDE, connections |

The old binary formats (.doc, .ppt, .xls, .hwp) are not supported.

## What is verified

- **GetPut and PutGet** on the test files: 13 docx, 16 hwpx, 20 pptx plus 5
  audit decks, and 46 xlsx, taken from Apache POI, Apache Tika, docx4j,
  LibreOffice, python-pptx, rhwp, rust_xlsxwriter and others, plus synthetic
  Korean files. Sources and licences are in each corpus's `SOURCES.md`.
- **Opens in the real applications.** The Office check kit
  (`validation/office-kit`) is a set of edited and new files. It opened
  without a repair prompt and showed what its checklist expects in Word,
  PowerPoint, Excel and Hancom Office.
- **CI** runs rustfmt, clippy, the tests, wasm32 builds (the library crates do
  no I/O) with the tests on wasm32-wasip1, a check of every corpus file and
  kit file against the ISO/IEC 29500 transitional schemas
  (`validation/ooxml-schema`), and plugin installation in Claude Code and
  Codex.
- **Fluency.** Agents were tested on the format before it was fixed
  ([fluency/](fluency/README.md)).

## Status and limitations

Version 0.3.0 (release candidate). The format has `schema: 1` and may change before 1.0.

These are not supported yet:

- pptx: effects can only be removed or set to the preset shadow. Only
  two-stop linear gradients (`fill="linear 90 accent1 #FFFFFF"`) can be
  edited; other gradients are kept. Custom geometry (`kind="custom"`) is
  kept as the file draws it, or replaced by a preset; its paths cannot be
  edited. A turned or flipped group moves, resizes and turns as a whole,
  and its objects move only within its box. Pictures, charts and tables
  cannot be created.
- Links, fields, footnotes and math in documents. They are read as
  placeholders, and the docx engine refuses new ones on export.
- Editing header, footer and section text.
- Nested tables. They stay placeholders.
- hwpx: a table's own position (`table-align`, `table-indent`), which docx
  writes.
- xlsx: cell styles beyond formats, new charts, renaming or deleting sheets,
  tables and columns, column insert and delete.
- Native-application fidelity for all DOCX/HWPX features, and XLSX print
  layout and advanced visuals. DOCX/HWPX previews are experimental. XLSX
  previews are bounded read-only worksheet windows with explicit cached
  formula and unsupported-feature diagnostics. Korean text needs a supplied
  or installed Korean font.

For the details of each engine, see [crates/README.md](crates/README.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Third-party material in the repository and the binaries is
listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The test files
keep their own licences, which each corpus's `SOURCES.md` names.

Unless you explicitly state otherwise, any contribution you intentionally
submit for inclusion in the work, as defined in the Apache-2.0 license, is
dual licensed as above, without any additional terms or conditions.

## Plugin validation tooling

Rust development and CI use the compiler pinned in `.mise.toml` and
`rust-toolchain.toml`; direct Cargo commands also select its formatting, lint
and WebAssembly components. This development pin is independent of the
`rust-version` declaration in `Cargo.toml`.

The plugin CI uses the Node.js and pnpm versions in `.mise.toml` and the CLI
versions in `package.json`. From the repository root, run `mise trust`,
`mise install`, then `mise run install`. Run the local manifest checks with
`mise exec -- pnpm exec claude plugin validate --strict plugins/hanji` and
`mise exec -- pnpm exec claude plugin validate --strict .`. The Rust build
remains the prerequisite for checks that execute the hanji CLI.
