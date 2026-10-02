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

Subcommands: `open`, `new`, `read`, `edit`, `write`, `ops`, `validate`,
`export`, `preview`, `reimport`, `history`, `diff`, `list`, `guide`.
`hanji preview DOC` (pptx) writes an HTML viewer of the revision as it would
export (`--format svg` or `png` for one file per slide) and says which fonts
it had to substitute; `--font-dir DIR` or `$HANJI_FONT_DIR` adds fonts. Add `--json` for
machine-readable output. Documents and their revisions are kept in `.hanji/`,
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

You do not need a Rust toolchain for either plugin. The skill tells the agent
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
| `hanji export DOC PATH` | Write a revision to a file. Refused until surfaced content (comments, hidden text, metadata) is acknowledged (`--acknowledge-surfaced`); `--tracked-changes` for docx |
| `hanji reimport DOC PATH` | Bring back a file a person edited in Office or Hancom as a new revision, and merge or refuse concurrent edits |
| `hanji history DOC`, `hanji diff DOC A B` | List a document's revisions, and show the diff between two of them |
| `hanji preview DOC` | Render a presentation to check the look: an HTML viewer, or SVG or PNG per slide, with the fonts it substituted |
| `hanji list`, `hanji guide` | List the stored documents; print the format summary for agents |

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

Version 0.1.0. The format has `schema: 1` and may change before 1.0.

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
- Preview of docx, hwpx and xlsx (pptx previews; Korean text needs a
  Korean font installed or in `--font-dir`).

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
