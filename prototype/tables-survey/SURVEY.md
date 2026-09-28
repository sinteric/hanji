---
status: survey results
date: 2026-09-28
answers: DESIGN.md §10 item 8, the open remainder — how often real documents hold the table shapes pipe tables cannot express
scope: Document type; 261 real files (94 Korean .hwpx, 167 English .docx); main text part only
---

# Table shapes in real documents: survey results

## The question

§10 item 8 leaves three table shapes without a syntax: rows that start or end
short (`w:gridBefore`/`w:gridAfter`), nested tables, and row-level content
controls. Such a table falls back to an uneditable block placeholder. The
remainder prototype met 3 of them in 29 tables, but those were feature-dense
test files. This survey counts the shapes in real documents.

## Sources and licences

No document is committed. [manifest.csv](manifest.csv) records each file's
original URL, fetch URL, licence, date and sha256.
[fetch.py](fetch.py) downloads the files into `corpus/` (git-ignored) and
checks each hash. [build_manifest.py](build_manifest.py) shows how the manifest
was built.

| Collection | Files | Language, format | Licence | Date |
|---|---|---|---|---|
| korea.kr press releases (보도자료) from 32 ministries and agencies, mirrored as `corpus_latest100` in [wavelen-jw/GovPress_PDF_MD](https://github.com/wavelen-jw/GovPress_PDF_MD) at `ed43231` | 94 used of 99 | Korean, .hwpx | 공공누리 제1유형 (KOGL Type 1, attribution) for the text, per korea.kr. Images may belong to third parties. The mirror repo's MIT licence covers its code only. Not redistributed. | 2026-04-10 to 04-13 (approval date) |
| [GovDocs1](https://digitalcorpora.org/corpora/file-corpora/files/) (Digital Corpora), `by_type/docx.zip`: every .docx in the corpus, crawled from US government web servers (`usg=YES`) | 162 used of 163 | English, .docx | US federal works are public domain (17 U.S.C. §105). A few hosts are contractor or state sites (e.g. fnal.gov, a state university), so the licence of every file is not confirmed. Not redistributed. | retrieved 2008–2009 |
| [18F/handbook](https://github.com/18F/handbook) `downloads/*.docx` (4) and [usds/playbook](https://github.com/usds/playbook) TechFAR Handbook (1) | 5 | English, .docx | US Government work, public domain. 18F adds a CC0 1.0 waiver. | commits 2025-10 and 2026-06 |

Excluded, and not in the manifest:

- 4 of the 99 korea.kr files are HWP 5.0 binaries renamed `.hwpx`.
- 1 is an HTML error page.
- 1 GovDocs1 file (641559) is 162 bytes of padding, not a zip.

**Blocked hosts.** The session's egress proxy refused a CONNECT to every
Korean government host tried, with `curl: (56) CONNECT tunnel failed, response
403`. The hosts were www.data.go.kr, www.korea.kr, www.kogl.or.kr, www.gov.kr,
open.go.kr, www.opengov.go.kr, www.alio.go.kr, www.law.go.kr, www.nl.go.kr,
www.archives.go.kr, and the ministry sites moef, mois, molit, msit and
me.go.kr. The same error came from kdi.re.kr, nia.or.kr, www.gov.uk,
assets.publishing.service.gov.uk, www.govinfo.gov, nist.gov, cdc.gov, epa.gov,
nasa.gov, data.gov.uk, catalog.data.gov, canada.ca, legislation.gov.uk,
op.europa.eu, un.org, worldbank.org, oecd.org, who.int, zenodo.org, osf.io,
figshare.com, huggingface.co, web.archive.org, archive.org, commoncrawl.org,
downloads.digitalcorpora.org, commons.wikimedia.org and sourceforge.net.
Reachable: github.com, raw.githubusercontent.com and
digitalcorpora.s3.amazonaws.com. So the Korean files come from a GitHub mirror
of korea.kr, and their licence was checked through web search, not on
korea.kr itself.

## Method

[survey.py](survey.py) opens each file and classifies every top-level table.
For .docx that means every `w:tbl` in `word/document.xml`. For .hwpx it means
every `hp:tbl` in `Contents/section*.xml`. A table inside a cell is counted
with its outer table.

Each table gets the first category that applies, in this order:

1. **nested**: a cell holds a table.
2. **row-cc**: a `w:sdt` or `w:customXml` wraps rows (`tbl > sdt`) or cells
   (`tr > sdt`).
3. **grid**: a row starts or ends short. In docx that is `gridBefore`/`gridAfter`,
   or cells that stop before the grid width. In HWPX it is a grid slot that no
   cell covers.
4. **other**: anything else §5.2 cannot express, for example `hMerge`, text in
   a covered cell, or a vertical merge that is not a rectangle.
5. **multipara**: `<p/>` cells, supported since round 3.
6. **merged**: `^^`/`||` only.
7. **plain**.

Every shape a table has is also listed as a flag.

The docx side reuses the remainder prototype's classifier
(`Importer.table_reason` in [../remainder/hanji_rem/importer.py](../remainder/hanji_rem/importer.py)).
It records the classifier's first reason as `proto_reason` and checks that
reason against the full scan. All 506 docx tables agree, once the scan is
mapped back to the prototype's stricter rules. The scan departs from the
prototype on purpose in two ways. It collects every shape, not just the first.
And it treats zero-width markers as transparent: bookmarks, proofing marks, and
permission and comment ranges. The remainder can anchor those between rows or
cells without new syntax.

Results: [results/tables.csv](results/tables.csv) (one row per table, dimensions
and flags, no document text), [results/docs.csv](results/docs.csv),
[results/summary.md](results/summary.md).

## Frequencies

Per top-level table:

| Category | Korean hwpx | English docx | All |
|---|---|---|---|
| nested | 8 (1.0%) | 27 (5.3%) | 35 (2.6%) |
| row-level content control | 0 | 0 | 0 |
| gridBefore/gridAfter (grid) | 0 | 13 (2.6%) | 13 (1.0%) |
| other | 0 | 0 | 0 |
| **unsupported, total** | **8 (1.0%)** | **40 (7.9%)** | **48 (3.6%)** |
| multi-paragraph cells (`<p/>`) | 284 (34.3%) | 175 (34.6%) | 459 (34.4%) |
| `^^`/`||` merges only | 74 (8.9%) | 49 (9.7%) | 123 (9.2%) |
| plain | 463 (55.9%) | 242 (47.8%) | 705 (52.8%) |
| **tables** | 829 | 506 | 1,335 |

Per document (a document counts under every category it has a table of):

| Documents with | Korean hwpx | English docx | All |
|---|---|---|---|
| any table | 94 (100%) | 81 (48.5%) | 175 (67.0%) |
| a nested table | 4 (4.3%) | 5 (3.0%) | 9 (3.4%) |
| a row-level content control | 0 | 0 | 0 |
| gridBefore/gridAfter | 0 | 8 (4.8%) | 8 (3.1%) |
| **any unsupported shape** | **4 (4.3%)** | **12 (7.2%)** | **16 (6.1%)** |
| … among documents with tables | 4 of 94 (4.3%) | 12 of 81 (14.8%) | 16 of 175 (9.1%) |
| multi-paragraph cells | 83 (88.3%) | 59 (35.3%) | 142 (54.4%) |
| **documents** | 94 | 167 | 261 |

The unsupported tables are concentrated in a few documents:

- One GovDocs1 file holds 18 of the 35 nested tables.
- Three files (633495, 632014 and koreakr-156754209) hold 25 of the 35.
- 28 of the 35 nested outer tables have a single column. They are layout
  frames, not data grids.

Row-level content controls did not appear at all. The only structural content
control is one `tr > sdt` wrapping one cell, and it sits in a table that is
already nested (example C1). Every gridBefore/gridAfter table comes from 2007–09
GovDocs1 files. None comes from the 94 HWPX files. HWPX tables always covered
their full `rowCnt × colCnt` grid.

Things that are not shapes but matter for import (flag counts):

- **Markers in table structure**, all in docx:
  - 79 of 506 tables (15.6%) have a bookmark or similar between `tblGrid` and the
    first row.
  - 86 have one between cells, and 72 between the paragraphs of a cell.
  - The prototype's classifier refuses all of these. 77 tables become
    placeholders for that reason alone ("table-level bookmarkStart").
- **HWPX tables sharing their anchor paragraph.** An HWPX table is an object
  inside a paragraph.
  - 28 tables (14 pairs) sit two to a paragraph, side by side. Most are the
    contact boxes (담당 부서 / 책임자 / 담당자) at the end of a press release.
  - 4 tables share their paragraph with text of its own.
  - 7 tables have a field in a cell.
- **1×1 "box" tables**: 114 in HWPX (13.8%) and 21 in docx. A box table is a
  framed note used as a sidebar. It is a legal one-cell pipe table, and it is
  where most of the Korean `<p/>` cells are.

## Examples

Anonymised. Row and column counts are the table's grid.

**gridBefore / gridAfter**

- G1. A procurement guide's contract-type comparison matrix (GovDocs1 037025),
  three tables of 7 × 7. The first six rows fill the grid. The last row, a
  "not for use with…" note, has three cells that cover six columns, and
  `gridAfter=1` leaves the seventh column empty. The same shape repeats in all
  three tables. It is a pipe table plus one short row.
- G2. A field-survey interview form (GovDocs1 280856), 5 × 8. The header rows
  span the grid. Each check-box row lists two or four `[ ]` + site-name pairs,
  so its row ends 2 or 5 columns short (`gridAfter=2`, `gridAfter=5`). This
  is form layout. The missing slots have no border.
- G3. A rank-insignia chart (GovDocs1 116734), 6 × 7. Rows of three insignia
  alternate with rows of two. The two-insignia rows start one column in
  (`gridBefore=1`) and end one or two short (`gridAfter`), which centres
  them. This one really needs the offset: shown as empty cells, the rows would
  lose their centring.
- G4. A contact block (GovDocs1 523860), 2 × 3. Its first row is one empty cell
  plus `gridAfter=2`, a spacer.

**Nested tables**

- N1. Korean press release, "titled frame" (koreakr-156754209 table 19;
  koreakr-156754252 table 99 has the same shape). The outer table is 3 × 3 or
  4 × 3.
  - The top row is a tab: empty corner cells around a merged title cell,
    `< 선투자 인정 기준 >`.
  - The bottom row is one full-width merged cell holding 12–17 paragraphs and
    one or two data tables (3 × 3 and 6 × 4; 3 × 6).
  - The outer table is decoration. The inner tables are the data.
- N2. Korean press release, box around a table (koreakr-156754209 table 6;
  also 156754095 and 156754235). A 1 × 1 box with 4–7 `□`/`○` paragraphs of
  explanation, and a data table inside it: 4 × 5 (programme budget), or a
  small 1 × 1 or 1 × 2 table.
- N3. A clinical reference card deck (GovDocs1 633495). 18 one-column, 4-row
  tables, one per card. The rows alternate heading ("VITALS", "DESCRIPTION")
  and body, and each body cell holds a small key–value table.
- N4. A regional programme directory (GovDocs1 017097). One 5 × 1 table lays
  out the whole page and holds 47 inner tables, one per programme (department,
  support contact, mailing address).
- C1. A laboratory memo header (GovDocs1 623902), 7 × 2. The rows are To /
  From / CC / Date / Re / Comments. In the From row, a content control wraps
  the value cell (`tr > sdt`, aliases "Author" and "Company", bound to
  document properties). The Comments cell holds a nested table. This is the
  corpus's only content control in table structure, and it wraps a cell, not
  a row.

**Row-level content controls**: none in 261 documents. Their usual source is a
Word *repeating section* (`tbl > sdt` over `tr`), which mostly appears in
forms built in Word 2013 or later. Neither collection has such forms (see
Limits).

## Recommendation

**No shape needs syntax now; the block placeholder is enough.**

| Shape | Share of tables | Recommendation |
|---|---|---|
| Row-level content controls | never seen | No syntax. |
| gridBefore/gridAfter | 1.0% overall, 0% in Korean | No syntax. |
| Nested tables | 2.6% overall, 1.0% in Korean | No syntax. |

The table shape real documents need, multi-paragraph cells, is already
supported. Without it, 34% of tables would be placeholders, against 3.6% for
all three open shapes together.

Two changes that need no syntax would each gain more than any new syntax:

1. **Markers in table structure go to the remainder**, like body-level
   `bmarker` entries. Today a bookmark before the first row turns 15% of docx
   tables into placeholders: 79 tables, against 48 for the three open shapes
   together.
2. **The HWPX importer lays out an anchor paragraph that holds tables as
   separate blocks.** It keeps the shared anchor, and any text around it, in
   the remainder, so the 28 side-by-side contact boxes stay pipe tables.

Shapes that did not reach the corpus stay placeholders. If a later corpus shows
one is common, the smallest syntax consistent with §5.2 is one of these:

- **gridBefore/gridAfter**: a whole-cell local marker for "no cell in this
  slot". Following `^^`, it would be written only in covered positions, allowed
  only in a run at the start or end of a row, and distinct from an empty cell
  `|  |`. A single tag in the `<p/>` family fits, for example `| <gap/> |`.
  Showing the slots as plain empty cells, with `gridBefore`/`gridAfter` kept in
  the row's `tr` entry, would need no syntax. But it would break G3's centring
  as soon as someone typed into a slot, and it would make an empty cell mean
  two things.
- **Nested tables**: no new syntax, but a finer placeholder. The outer table
  stays a pipe table, and each inner table becomes an inline
  `<keep kind="table"/>` in its cell. Or, for the one-column frames that are 28
  of the 35 cases (N1–N4), the inner tables are lifted out as ordinary pipe
  tables and the frame goes to the remainder. Pick between the two only when
  there is a corpus where it matters.
- **Content control around rows or cells**: stays a placeholder. It carries
  data binding (C1), which §5.2 has no model for.

## Limits

- **Corpus is smaller and narrower than planned.** 261 files instead of
  100–300 from many sources. Because egress was blocked (above), and the scope
  was then cut for speed, only three collections were used:
  - The Korean side is one genre, ministry press releases, from four days in
    April 2026. It has no forms (신청서, 서식), no statutory annex forms and no
    long reports, which are where nested and irregular tables would be
    expected.
  - There are no Korean .docx at all.
  - The English side is 97% GovDocs1, crawled in 2008–09 and written mostly by
    Word 2007. Newer features, such as repeating-section content controls,
    cannot appear in it.
- Only the main text part is scanned: `word/document.xml` and HWPX
  `section*.xml`. Headers, footers, footnotes and comments are not, and neither
  are .hwp binaries (4 files excluded).
- Classification is structural. Word and Hancom were not opened. Whether a
  gridAfter slot is visible (bordered) or a nested table is layout or data was
  judged from the XML and the text, for the examples only.
- The docx licence is not confirmed file by file. Some GovDocs1 hosts are
  contractors or states. The korea.kr licence was checked through web search,
  not on korea.kr, which was blocked. So no document is committed.
- HWPX has no `sdt`. Its nearest equivalents, fields (누름틀), were counted only
  as a flag: 7 tables.
