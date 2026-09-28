# Spike: tracked-change export (DESIGN.md §10 item 2)

**Question.** Can rdocx (docx) and rhwp (hwp/hwpx) write insertions and
deletions, with an author and a date, that open cleanly? If they can, a model
edit could reach the exported file as reviewable tracked changes instead of as
direct changes.

**Short answer.**

- **docx: yes, if hanji writes the revision XML itself.** rdocx has no API for
  writing revisions. It can generate them from two documents
  (`Document::compare_with_options`), but that path fails too often to be the
  export path. When hanji writes `w:ins`/`w:del`/`w:pPrChange` directly, rdocx
  and LibreOffice both read the result, and Accept All / Reject All give the
  expected text in 6 of 6 files (with one LibreOffice field-recompute
  exception, noted below).
- **hwpx: not now.** rhwp does not model OWPML track changes. It opens and
  renders a file that has them, but it shows deleted text as ordinary text.
  Saving through rhwp drops every mark without reporting a loss, and rhwp's own
  round-trip loss gate still passes.
- **Not tested: Word and Hancom themselves.** Every result here comes from
  rdocx, rhwp and LibreOffice 24.2. That check is deferred (§9 Validity).

Recommendation (§10 item 2): **direct changes only for now**. Tracked changes
come later as a docx-only **export option**, written by hanji-docx itself
(route B below). See the end of this file.

## Setup

| Tool | Version |
|---|---|
| rdocx | 0.14.0 from crates.io, `default-features = false` |
| rhwp | 0.8.6, git `680111ec7bea2fe11110de18c3676ba5a1cf7847` (`cargo build --release --bin rhwp`) |
| LibreOffice | 24.2.7.2, headless, driven over UNO (`python3-uno`) |
| Rust / Python | 1.94.1 / 3.11, lxml 6.1.3, PyMuPDF (only for looking at PDFs) |

Inputs:

- docx: the 13 files in `../remainder/corpus` (see its `SOURCES.md`).
- hwpx:
  - `hwpx/report.json` turned into a small Korean report by `rhwp scaffold`.
  - `mel-001.hwpx`, a Hancom Office 2021 document from rhwp's samples,
    fetched at a pinned commit by `hwpx/fetch.sh`.

The crate here is standalone (`[workspace]` in its own `Cargo.toml`), is not
part of the hanji workspace, and changes nothing under `crates/`.

```sh
cargo build --release                         # target/release/tcspike (rdocx driver)
python3 make_direct.py                        # route A input: hanji-style direct exports, E0–E10
python3 inject.py                             # route B: revision XML written by hand
python3 validate.py --granularity run         # route A + B checks -> results/docx-run.json  (~5 min)
python3 validate.py --granularity word        #                     -> results/docx-word.json
hwpx/fetch.sh && RHWP=/path/to/rhwp python3 hwpx_spike.py   # -> results/hwpx.json
python3 summarize.py                          # -> results/RESULTS.md (all numbers below)
```

`out/` (every generated file) is git-ignored. A rerun regenerates it.

## docx / rdocx

### What the rdocx API offers

| Need | rdocx 0.14.0 |
|---|---|
| Write an insertion or deletion run with author and date | **No API.** Backlog item F-291 "Tracked insertion and deletion authoring" in `docs/hld/14-development-backlog.md` is not built. |
| Generate revisions from two documents | `Document::compare(&edited, author, rfc3339)` and `compare_with_options(.., &ComparisonOptions { granularity: Run \| Word \| Character, .. })` (`crates/rdocx/src/comparison.rs:226,236`). Writes `w:ins`, `w:del`, `w:moveFrom`/`w:moveTo` and `w:pPrChange`, each with `w:id`, `w:author` and `w:date`. It commits only after checking that accepting and rejecting its result reproduce the two inputs. |
| Read revisions | `Document::revisions()` → `RevisionRef { id, author, timestamp, kind }` (main story only) |
| Resolve revisions | `accept_all`, `reject_all`, and the same two filtered by author, by date range or by id (`crates/rdocx/src/revision.rs:103–140`) |
| Turn on "track changes" for the reviewer | `set_track_revisions(true)` (`w:trackRevisions` in settings) |
| Render revisions | `to_pdf_with_options(RenderOptions { revision_view: RevisionView::Tracked })` |
| Inject raw XML | No public entry point into a paragraph. Revision XML already in a file is kept as captured bytes ("the captured revision subtree remains the sole serialization source", `rdocx-oxml/src/revision.rs`), so it survives open and save. |

That leaves two ways to get tracked changes: A, let rdocx compare the unedited
export with the edited one; B, have hanji write the revision XML in its own
exporter and use rdocx only to read and check the result.

### Route A: `compare_with_options(E0, Ek)` after hanji's direct export

`make_direct.py` runs the remainder prototype (design C) and exports the
unedited file (E0) and each scripted edit E1–E10 as direct changes. This gives
107 exports over 13 files. `tcspike compare` then records Ek against E0 as
revisions by `hanji (model edit)`, dated `2026-09-28T00:00:00Z`.

| | Run granularity | Word granularity |
|---|---|---|
| tracked file written | **40 / 107** | 38 / 107 |
| refused: input already has revisions (5 of 13 files: delins, korean-report, sample-docx, tdf154481, testWORD_2006ml) | 45 | 45 |
| refused: structure (paragraph split/merge boundary, final paragraph, field, moved block, "acceptance does not reproduce") | 20 | 24 |
| returned OK but wrote **no** revision (restyle silently lost) | 2 (Bug54849 E5, loadAndSave E5) | 0 |
| E10 (all edits in one revision) tracked | **0 / 8** | 0 / 8 |
| well-formed (all parts) | 34 / 40 | 32 / 38 |
| no whitespace dropped (`w:t` with edge spaces but no `xml:space="preserve"`) | 40 / 40 | **31 / 38** |
| rdocx re-opens, `revisions()` lists all, save + re-open keeps all, bytes identical | 40 / 40 | 38 / 38 |
| rdocx `accept_all` text == edited, `reject_all` text == unedited | 40 / 40 | 38 / 38 |
| LibreOffice opens | 34 / 40 | 32 / 38 |
| LibreOffice lists ≥ 1 tracked change | 32 / 34 | 30 / 32 |
| LibreOffice Accept All / Reject All text == edited / unedited (both) | 33 / 34 | 22 / 32 (accept 24, reject 25) |

Tracked files per edit kind, Run granularity, over the 8 files without their
own revisions:

- E1 figure: 6/6
- E2 insert paragraph: 5/6
- E3 delete paragraph: 4/5
- E4 move section: 2/7
- E5 restyle: 4/7
- E6 table cell: 4/4
- E7 edit across runs: 6/6
- E8 split: 4/7
- E9 merge: 5/6
- E10: 0/8

What went wrong, with evidence (`results/RESULTS.md` has every file):

1. **Files that already have revisions are refused.** The error is
   `document comparison requires inputs without existing modeled revisions`,
   and it hits 5 of 13 files. Documents under review are exactly the ones
   likely to carry revisions already.
2. **Real revisions touch several places, and compare refuses those.** E10
   fails in 8 of 8 files, with errors such as `comparison needs an adjacent
   paragraph for a final paragraph change`, `cannot revise paragraph boundary
   structures` and `correlated 3 of 5 physical paragraph runs`.
3. **Word granularity drops spaces.** It splits text into word, space and word
   `w:t` elements and writes the space ones as `<w:t> </w:t>` with no
   `xml:space="preserve"`.
   - LibreOffice then collapses them: after Accept All, "Row 15 Col 1" reads
     "Row15Col1" (testWORD_various E6, fdo76098 E7/E9).
   - Word handles `w:t` whitespace by the same XML rule. This is an inference:
     Word was not run.
   - Run granularity does not have this bug, but it is coarse: it deletes the
     whole run and inserts the whole new run (`<w:delText>subscript
     </w:delText>` then `<w:t>subscripEDITED</w:t>`).
4. **rdocx save breaks a part it does not need to touch.** Plain
   `Document::open` + `save` of `loadAndSave.docx` writes a `word/styles.xml`
   that is not well-formed.
   - It drops `xmlns:w14` from the root but keeps `<w14:ligatures>` in
     `rPrDefault`.
   - LibreOffice then refuses the file ("source file could not be loaded").
     This caused all 6 well-formedness failures.
   - This is not specific to tracked changes. It also means route A cannot keep
     hanji's GetPut law, because hanji-docx copies other parts byte for byte.
5. **Moves and format changes do not always show in LibreOffice.** A
   `w:moveFrom`/`w:moveTo` pair (WordWithAttachments E4) and one `w:pPrChange`
   imported into LibreOffice as no redline at all.
6. **rdocx's Korean rendering needs fonts.** The tracked-view PDF shows the
   revision decorations (underline, strike, change bars). With
   `default-features = false`, though, the bundled fonts have no Hangul, so
   Korean text renders blank. Use `system-fonts` or supply fonts.

### Route B: hanji writes the revision XML, rdocx and LibreOffice check it

`inject.py` edits `word/document.xml` with lxml. That is what hanji-docx's
XML-level exporter would do. It leaves every other part as it is and uses ids
above the file's largest `w:id`. The XML it writes:

```xml
<!-- X1 deleted run inside a paragraph: split the run, keep its rPr -->
<w:r><w:rPr>…</w:rPr><w:t>아래</w:t></w:r>
<w:del w:id="…" w:author="hanji (model edit)" w:date="2026-09-28T00:00:00Z">
  <w:r><w:rPr>…</w:rPr><w:delText xml:space="preserve"> 표는</w:delText></w:r></w:del>
<w:r><w:rPr>…</w:rPr><w:t xml:space="preserve"> 지점별 …</w:t></w:r>

<!-- X2 inserted paragraph, Word's own shape: the mark that ends the paragraph
     BEFORE it is the inserted one; the new paragraph takes that paragraph's pPr -->
<w:p><w:pPr>…<w:rPr><w:ins w:id="…" w:author="…" w:date="…"/></w:rPr></w:pPr>…old text…</w:p>
<w:p><w:pPr>…copy…</w:pPr><w:ins w:id="…" w:author="…" w:date="…"><w:r><w:t>모델이 추가한 문단입니다.</w:t></w:r></w:ins></w:p>

<!-- X3 changed table cell: delete the old run, insert the new one -->
<w:tc>…<w:p><w:del …><w:r><w:rPr>…</w:rPr><w:delText>지역</w:delText></w:r></w:del>
             <w:ins …><w:r><w:rPr>…</w:rPr><w:t>1,204</w:t></w:r></w:ins></w:p></w:tc>

<!-- X4 deleted paragraph: every run in w:del, and the paragraph mark deleted -->
<w:p><w:pPr><w:rPr><w:del …/></w:rPr></w:pPr><w:del …><w:r><w:delText>…</w:delText></w:r></w:del></w:p>

<!-- X5 restyle -->
<w:pPr><w:pStyle w:val="Heading2"/><w:pPrChange …><w:pPr><w:pStyle w:val="Heading1"/></w:pPr></w:pPrChange></w:pPr>
```

| file | edits | well-formed | rdocx `revisions()` | rdocx save keeps revision XML | rdocx accept / reject text | LibreOffice redlines | LibreOffice accept / reject text |
|---|---|---|---|---|---|---|---|
| Bug54849 | X2 X3 | yes | del 1 ins 3 | 4/4 | yes / yes | Insert 2 Delete 1 | yes / yes |
| docx4j-tables | X1 X2 X3 X5 | yes | del 2 ins 3 pPrChange 1 | 6/6 | yes / yes | Delete 2 Insert 2 ParagraphFormat 1 | yes / yes |
| korean-report | X1–X5 | yes | del 10 ins 4 pPrChange 1 | 15/15 | yes / yes | ParagraphFormat 1 Insert 3 Delete 3 | yes / yes |
| loadAndSave | X1–X5 | yes | del 5 ins 3 pPrChange 1 | 9/9 | yes / yes | ParagraphFormat 1 Delete 3 Insert 2 | yes / yes |
| tdf154481 | X1 X2 X4 X5 | yes | del 2 ins 4 pPrChange 1 | 15/15 | yes / yes | ParagraphFormat 1 Insert 2 Delete 1 | yes / no* |
| testWORD_various | X1–X4 | yes | del 5 ins 3 | 8/8 | yes / yes | Delete 3 Insert 2 | yes / yes |

Notes on the table:

- Counts include the revisions the file already had (korean-report and
  tdf154481). Those coexist with hanji's.
- \* In tdf154481, rejecting *all* revisions also rejects the file's own
  earlier revisions. LibreOffice then recomputes REF fields to "Error:
  Reference source not found". The difference is not in hanji's marks.
- LibreOffice's PDF export with changes shown
  (`out/lo/B-korean-report.changes.pdf`) shows the insertions underlined, the
  deletions struck through, and change bars. LibreOffice reports
  `hanji (model edit)` as the author of every change hanji wrote.

A pitfall this found: the first version of X2 marked the **new** paragraph's
mark as inserted. When a table followed it, rdocx `reject_all` refused with
`paragraph mark removal requires an adjacent paragraph`, and LibreOffice's
Reject All left an empty paragraph behind. Moving the mark to the preceding
paragraph, which is what Word writes for "Enter, then type", fixed both.
Details like this are where the effort goes.

## hwp / hwpx / rhwp

### What rhwp has (source at `680111ec`)

- **Nothing that writes track changes.** No CLI `edit` subcommand, MCP tool
  or `rhwp capabilities` entry mentions track changes or revisions.
  `mydocs/tech/hwpx_hancom_reference.md` lists 변경 추적 (track changes) as
  "3단계 (협업 기능)", a later collaboration phase.
- **Reading HWPX:**
  - `hp:insertBegin`, `hp:insertEnd`, `hp:deleteBegin` and `hp:deleteEnd`
    inside `<hp:t>` fall into `_ => {}` in `read_text_content_with_tabs`
    (`src/parser/hwpx/section.rs`), so they are dropped. The text between the
    marks, including deleted text, becomes ordinary run text.
  - `<hh:trackChanges>` and `<hh:trackChangeAuthors>` in `header.xml` are not
    parsed.
  - Only `<hh:trackchageConfig flags>` (Hancom's own spelling) is carried
    through (`src/serializer/hwpx/header.rs:1311–1340`), because it affects
    pagination.
- **HWP 5.0:**
  - The DocInfo `HWPTAG_TRACKCHANGE` record is kept raw in `extra_records`.
  - The PARA_HEADER tail ("instanceId/변경추적 suffix") is kept raw
    (`document_ir_audit_1414.md`).
  - The body-side change marks are not modelled.
- **Loss reporting:** `ContentLossCode` has no track-change case. Only the
  DocLang export's loss report has a `TrackChanges` category.
- **Samples:** 0 of the 543 `.hwpx` files in the rhwp repository contain
  track-change markup. That leaves no Hancom-authored example to copy the exact
  shape from.

### What the spike wrote (OWPML, `hwpx_spike.py`)

This follows the schema in rhwp's `mydocs/manual/OWPML SCHEMA`: `TrackChangeTag`
in ParaList XML schema.xml lines 294–299 and 2808–2812, and `TrackChange` and
`TrackChangeAuthor` in Header XML schema.xml.

```xml
<!-- header.xml, end of <hh:refList> -->
<hh:trackChanges itemCnt="6">
  <hh:trackChange type="Delete" date="2026-09-28T00:00:00Z" authorID="1" hide="0" id="1"/>
  <hh:trackChange type="Insert" date="2026-09-28T00:00:00Z" authorID="1" hide="0" id="2"/> …</hh:trackChanges>
<hh:trackChangeAuthors itemCnt="1">
  <hh:trackChangeAuthor name="hanji (model edit)" mark="1" color="#FF0000" id="1"/></hh:trackChangeAuthors>

<!-- section0.xml, inside <hp:t>: H1 figure replaced inside a paragraph -->
<hp:t>매출은 전년 대비 <hp:deleteBegin Id="1" TcId="1" paraend="0"/>12%<hp:deleteEnd Id="1" TcId="1" paraend="0"/><hp:insertBegin Id="2" TcId="2" paraend="0"/>15%<hp:insertEnd Id="2" TcId="2" paraend="0"/> 증가했다. …</hp:t>
<!-- H2 inserted paragraph: new hp:p, insertEnd paraend="1"; H4 deleted paragraph: deleteEnd paraend="1" -->
```

The spike guessed three things that nothing here can check: that `Id` is the
pair and `TcId` refers to the header entry, the meaning of `paraend`, and
whether Hancom needs `charShapeID`/`paraShapeID`. There was no Hancom-authored
sample to compare against.

| Check (report.hwpx and mel-001.hwpx, same result) | Result |
|---|---|
| well-formed XML, all parts | yes |
| OWPML XSD validation | not done: the schema set in rhwp's repo does not load in libxml2 (`SectionType` unresolved) and targets the 2024 namespaces; the markup was checked against the schema text by hand |
| `rhwp info`, `export-svg`, `export-pdf` | open and render (1 and 20 pages) |
| `rhwp export-text`: deleted text | **shown as ordinary text**: both "12%" and "15%", and the deleted paragraph |
| rhwp save (`edit replace-text … -o`) | **12 → 0 marks, header entries gone, no content-loss report**; deleted text becomes permanent |
| `rhwp hwpx-roundtrip` (loss gate) | **PASS, diff=0**, although its output also has 0 marks: the gate compares IR, and the IR never had them |
| `rhwp convert` to HWP 5.0 | exit 0; deleted text is ordinary text |
| LibreOffice 24.2 | no HWPX import ("source file could not be loaded") |

So hanji could write this markup itself in an XML-level hwpx exporter. But in
this environment nothing can show that Hancom accepts it. And anything that
goes through rhwp (hanji's preview, a save) shows or makes permanent the deleted
text as if it were live. That breaks §4's "preview and export cannot drift": the
preview would show text that the reviewer sees as deleted.

## Result table

| Engine / route | writes ins/del + author + date | deleted run in a paragraph | inserted paragraph | changed table cell | opens cleanly (well-formed, re-opens in same engine) | LibreOffice shows it as tracked | Word / Hancom |
|---|---|---|---|---|---|---|---|
| rdocx API | no (read, accept/reject, render only) | – | – | – | – | – | not tested |
| rdocx `compare`, Run | yes, 40/107 exports | yes (E7) | yes (E2) | yes (E1, E6) | 34/40 (rdocx save breaks one file's styles.xml) | 32/34; accept and reject both right in 33/34 | not tested |
| rdocx `compare`, Word | yes, 38/107 | yes | yes | yes, but spaces lost | 32/38; 7 drop whitespace | 30/32; accept and reject both right in 22/32 | not tested |
| **hanji writes OOXML (route B)**, rdocx re-reads | yes, 6/6 files | yes | yes | yes | 6/6 | 6/6; 11/12 right (1 = field recompute) | not tested |
| rhwp API | no | – | – | – | – | – | not tested |
| hanji writes OWPML, rhwp reads | written; rhwp ignores it | shown as live text | shown as live text | shown as live text | opens and renders; any rhwp save drops every mark silently | LibreOffice cannot open hwpx | not tested |

## Effort and risk if hanji adopts tracked-change export

**docx, route B, in hanji-docx's exporter.** About 1–2 weeks for text runs,
paragraph insert/delete/split/merge, restyle and table cells, with tests.

- hanji already knows each edit as exact spans against a known revision (§4).
  That is the input route B needs; no diff has to be guessed.
- The marks map directly:
  - a span replacement becomes a split run plus `w:del` and `w:ins` with the
    same `rPr`;
  - paragraph changes become paragraph-mark `w:ins`/`w:del` in Word's shape;
  - a restyle becomes `w:pPrChange`.
- Not tried in this spike: row and column insert/delete (`w:trPr/w:ins`,
  `w:cellIns`), cell merges, and moves. Moves should be written as delete plus
  insert, not `w:moveFrom`/`w:moveTo`, since LibreOffice lost one move.
- The tests fit the existing laws. reject(export) must equal the previous
  revision (GetPut); accept(export) must equal the written text (PutGet).
  rdocx's `accept_all`/`reject_all` serve as an independent oracle; rdocx is
  not used to write.

Risks:

1. Word and Hancom were not run. The §9 no-repair-prompt test must pass in
   Word before this ships.
2. Edits that touch spans already inside `w:ins`/`w:del`, which §5.2 treats as
   read-only projections, need nesting rules. Refuse them at first.
3. Re-import: a second model edit on top of a pending revision.
4. §8 surfacing. A tracked export deliberately keeps deleted text in the file,
   so it must be surfaced before export.
5. Unique ids, and splitting runs next to fields, bookmarks and comments.

**docx, route A (rdocx compare).** Little code, but not usable as the export
path:

- it tracked 40 of 107 edits;
- it refuses 5 of 13 files for their existing revisions, and every multi-edit
  revision;
- Word granularity drops spaces, and Run granularity silently skips 2 restyles;
- rdocx rewrites every part, and in 1 of 13 files the rewritten `styles.xml`
  is not well-formed.

It could be an optional cross-check. Upstream issues are worth filing for the
namespace and whitespace bugs.

**hwpx.** No estimate yet: it is blocked on facts, not on effort.

- It needs a Hancom-authored sample with tracked changes, or Hancom itself, to
  settle what `Id`, `TcId` and `paraend` mean and how inserted paragraphs are
  stored.
- It needs either rhwp to model the marks (rhwp plans it as a later phase), or
  hanji to write the marks after rhwp and keep them out of rhwp's preview.
- Risk is high: rhwp's loss gate does not catch the drop, so a regression would
  be silent.

## Recommendation for §10 item 2

**Direct changes only for now.** Tracked changes should become a **docx export
option** ("export as tracked changes", off by default), not the default. The
default would work for one format and not the other: hwpx cannot do it
reliably, and a hwpx preview through rhwp would show deletions as live text. The
docx option should be built in hanji-docx as route B, and its preconditions are:

1. the docx GetPut tests pass (already the precondition in §10.2);
2. the export opens in Word with no repair prompt and lists every change with
   its author and date;
3. reject(export) == previous revision and accept(export) == PutGet text, over
   the corpus.

Leave hwpx on direct changes until a Hancom-authored tracked sample exists and
rhwp models the marks or reports their loss. Keep §5.2's read-only
`<ins>`/`<del>` projections as they are.
