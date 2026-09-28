# hanji fluency test, round 3 (DESIGN.md §10 item 8)

§10 item 8 asks for a syntax for **multi-paragraph table cells** and for **empty paragraphs**. In the remainder
prototype ([../../prototype/remainder/REMAINDER.md](../../prototype/remainder/REMAINDER.md)), 12 of 29 tables
could not be pipe tables and fell back to an uneditable block placeholder, 9 of them because of multi-paragraph
cells. Empty paragraphs had no Markdown form (the prototype wrote `<div style="Normal"></div>`), and runs of
identical empty paragraphs are where identity-by-diff failed: 17 of the 18 `ppr` losses under design C came
from empty paragraphs whose properties swapped when a rewrite moved some of them.

Round 3 keeps round 2's unit shape, answer format, refusal rule and scorer CLI. Round 1 and round 2 files are not
changed. `score.py` imports round 2's scorer for the shared pieces (front matter, inline tags, style-name
checks, the `{style="Name"}` line, edit application), and `build.py` is adapted from round 2's.

2026-09-28: the scorer was aligned with DESIGN.md §5.2 (commit 6a3530b): under both A candidates `<p/>` only starts a paragraph, so an empty paragraph inside a cell (`a<p/><p/>b`, `a<p/>`) and a `<p/>` line in `cellpara` units are accepted; no published round 3 result changed.

## Decisions

| decision | A | B |
|---|---|---|
| `cellpara` | every table stays a pipe table; inside a cell, `<p/>` starts the next paragraph and `<p style="Name"/>` starts the next paragraph with a style | a table that needs it becomes a list table: a `{list-table}` line, then one `-` line per row and one `  - ` line per cell, with further paragraphs of a cell on lines indented four spaces |
| `emptypara` | a line holding only `<p/>` or `<p style="Name"/>` | a line holding only `<div></div>` or `<div style="Name"></div>` |

Both candidates of both decisions sit on the syntax already decided in rounds 1 and 2 (§5.2): pipe tables with
`^^` / `||` merges (the guides now show `|||` too), the `{style="Name"}` table-style line, `<div style="Name">`
for a styled paragraph, one paragraph per line, `<br/>` as a line break, and a `<pagebreak/>` line. This shared
part is identical in the A and B guides of a decision.

### cellpara, exactly

- **A.** A pipe-table cell holds one or more paragraphs on the row's line. `<p/>` ends a paragraph and starts the
  next one, which has the default style; `<p style="Name"/>` does the same and gives the next paragraph that
  paragraph style. The first paragraph has no tag unless it has a style; then the cell begins with
  `<p style="Name"/>`. Every cell paragraph has text. `<p …/>` is one self-closing tag (no `</p>`), takes only
  `style`, and appears only inside table cells; a cell paragraph never uses `<div>`. Covered cells hold only `^^` /
  `||`, and a merged cell's paragraphs are written once, in its top-left cell.

  ```
  | 서울 | 강남·종로 2곳<p style="표 참고"/>종로는 10월 개점 | <p style="표 참고"/>임차 |
  ```

  Errors: `cell_para_form` (`<p>…</p>`), `empty_cell_para` (a cell ending with a tag, or `<p/><p/>`), `cell_div`
  (`<div>` inside a cell), `p_outside_cell` (`<p/>` in a paragraph line or on a line of its own).
- **B.** A table in which any cell holds more than one paragraph, or a paragraph with a style, is a list table
  (MyST `list-table`-like); every other table stays a pipe table. The parser knows a list is a table by its
  **leading `{list-table}` line**: without it the same lines are an ordinary list, so ordinary lists, nested ones
  included, are unaffected.
  - The `{style="Name"}` line, if any, comes directly before the `{list-table}` line.
  - Each row is a line holding only `-`. Each cell is one line starting with exactly two spaces, `-` and a space;
    it holds the cell's first paragraph. Each further paragraph of that cell is its own line, indented exactly
    four spaces. A styled paragraph is `<div style="Name">text</div>` in either position.
  - The first row is the header row. Every row has one cell per column.
  - A covered cell is a cell line holding only `^^` or `||`. An empty unmerged cell is `  -`.
  - There is no blank line inside; the first blank line ends the table.

  ```
  {style="표 눈금"}
  {list-table}
  -
    - 지역
    - 내용
  -
    - 서울
    - 강남·종로 2곳
      <div style="표 참고">종로는 10월 개점</div>
  ```

  Errors: `list_table_form` (a row line with text such as MyST's compact `- - cell`, wrong indentation, a stray
  line), `list_parsed_as_list` (a list-table body without its `{list-table}` line, or cut by a blank line, which
  reads as an ordinary list whose items have no text), `row_width_mismatch`, `marker_misplaced` (a covered cell
  with a further paragraph), `attr_form` (`{list-table style="…"}`), `cell_div` (`<div>` inside a pipe-table cell).

In both, **only tables that need multi-paragraph (or styled) cells use the candidate form**. In A that costs
nothing: the tags go into the cells that need them. In B the whole table switches form, so two tasks test the
switch: `notice1-e1` gives a cell of a plain pipe table a second paragraph (B must rewrite the table as a list
table), and `form3-e2` joins the only multi-paragraph cell of a list table (B may keep the list table or go back
to a pipe table; both are scored as landed, because the scorer compares meaning, not form).

### emptypara, exactly

- **A.** An empty paragraph is a line holding only `<p/>` (default style) or `<p style="Name"/>`. One
  self-closing tag, only `style`, never inside a line of text or a table cell. Errors: `empty_para_form`
  (`<p></p>`, `<p/>` inside text, or `<div style="Name"></div>` used for an empty paragraph).
- **B.** An empty paragraph is a line holding only `<div></div>` (default style) or `<div style="Name"></div>`.
  `<div></div>` is the only `<div>` without a style (a `<div>` with text still needs one, as in §5.2). Errors:
  `empty_para_form` (`<div/>`), `missing_attr` (`<div>text</div>`), `unknown_tag` (`<p/>`).
- **Canonical form, both:** every empty paragraph is its own line, even when identical to its neighbours;
  consecutive empty paragraphs are on consecutive lines with no blank line between them, and a blank line
  separates the group from the blocks around it. A blank line is never a paragraph, and neither is a line
  holding only `<br/>`. The parser treats blank lines as separators only, so a run written with blank lines
  between its members means the same.

## Content borrowed from the corpus

Cell shapes come from the real multi-paragraph cells of `prototype/remainder/corpus` (read from each
`word/document.xml`: 45 multi-paragraph cells in 3 files):

| corpus cell | shape | where it is used |
|---|---|---|
| `form_footnotes.docx`, t1 r2 c1 "03 - Date of birth" / "Day Month Year"; t1 r4 c1 "05 - Sex" / "male female" | a label or value line followed by a second line of the same item | 연락처 (전화 / 휴대전화), 연령 (요건 / 병역 예외), 이용 일시 (일시 / note) |
| `form_footnotes.docx`, t1 r12 c0: "11 – Highest level of education" + one paragraph per option | a cell of options or items, one per paragraph | 시설명 (`[ ] 대공연장` / `[√] 소공연장` / `[ ] 전시실`), 제외 대상 (three items), 수집 항목 |
| `form_footnotes.docx`, t5 r0 c0: "INSTRUCTIONS" (Heading5) + four plain items | a styled first line followed by plain items | 준수 사항 (`표 소제목` line + ① ② ③) |
| `form_footnotes.docx`, t2 r2 c2 "F - Validade" + a `Header`-styled line; t1 r2 c1 | a notes cell whose later line has another style | 비고 / 결정 사항 / 배포 cells ending in a `표 참고` / `Table Note` line |
| `fdo76098.docx`, t0 r0 c2 (vMerge) and t0 r3 c1 (gridSpan 2), two paragraphs each; most `form_footnotes.docx` multi-paragraph cells span columns | multi-paragraph cells inside or next to merged cells | 비고 of 거주/창업 (a merged cell with two styled paragraphs), 지원 한도 `^^` next to multi-paragraph cells, 준수 사항 `||` label next to a four-paragraph cell, 결과 `||` in `form3-w` |
| `form_footnotes.docx`: the same fill line or note in many cells | the same paragraph in several cells | `※ 해당 시 작성` in two cells (`form3-e1` must hit one) |

Not borrowed: cells holding **empty** paragraphs (`docx4j-tables.docx` t4: text + two empty paragraphs;
`form_footnotes.docx` t2 r3 "H - Observações" + two empty). Both decisions would need to combine to express
them (an empty paragraph inside a cell), so this round keeps the two apart: cell paragraphs always have text, and
the `emptypara` seeds have no multi-paragraph cells.

Empty-paragraph patterns come from the same corpus: 226 of 471 body-level paragraphs are empty; runs of 2–4 are
common and runs of 7, 10, 17 and 19 occur (`WordWithAttachments.docx`, `testWORD_2006ml.docx`, `tdf154481.docx`),
typically as spacing before a signature or to push the next part onto a new page; some are styled (`Normal` in
`fdo76098.docx` and `testWORD_various.docx`, `Header` in `form_footnotes.docx`), and 12 carry a section break.
The seeds use runs of 1–7, styled spacing lines after tables (`좁은 간격`, `Spacer`, `표 아래 간격`), runs that push
an attachment page or the report body to a new page, and a signature block with an empty paragraph between each
line.

## What makes it hard

- **Seeds of 3,000–6,000 characters** (3,023–3,408 over the 12 seeds), realistic Korean office documents:
  - `cellpara`: a youth start-up support notice (hwpx; 5 tables, 11 multi-paragraph cells), weekly minutes of a
    product division (docx; 4 tables, 15), a public-facility rental application form (hwpx; 5 tables, 13);
  - `emptypara`: an education-office official letter (공문) with two attachment pages (hwpx; 33 empty
    paragraphs), board-meeting minutes with a signature block (docx; 25), and a safety-inspection report with a
    cover page (docx; 30).
- **Identity.** Several identical things where the edit must hit one:
  - two identical runs of five empty paragraphs, each pushing an attachment page (`letter1-e4`), and runs of
    three that are also substrings of those runs (`letter1-e1`);
  - three identical `Spacer` lines after three tables (`board2-e1`), and "replace the middle of three identical
    empty paragraphs" (`board2-e2`, where the position does matter);
  - runs of 7, 5 and 6 empty paragraphs on one cover page, where a run of 6 or 5 is a substring of the run of 7
    (`report3-e1`, `report3-e3`);
  - `※ 해당 시 작성` in two cells (`form3-e1`).

  An exact-match `old` has to carry unique context (`edit_ambiguous` otherwise).
- **Structural edits on cells:** add a paragraph to a cell of a simple table (`notice1-e1`), split a cell
  paragraph (`notice1-e2`), merge two cells that both hold two paragraphs, next to an existing `^^`
  (`notice1-e4`), restyle one cell paragraph (`minutes2-e1`), move a paragraph between cells (`minutes2-e2`), add
  a row whose cells hold several paragraphs, one of them styled (`minutes2-e4`), delete one of two identical
  paragraphs (`form3-e1`), join a cell's paragraphs (`form3-e2`), and split a four-paragraph cell into three rows
  while a two-column label grows into a 3×2 merged area (`form3-e4`).
- **Structural edits on empty paragraphs:** delete one (`letter1-e1`), restyle one (`letter1-e2`), replace a run
  with `<pagebreak/>` (`letter1-e4`, `report3-e1`), insert one (`board2-e1`, `report3-e2`), turn one into text
  (`board2-e2`), delete every empty paragraph in one section, styled ones included (`board2-e3`), and delete two of
  seven (`report3-e3`).
- **Write tasks** list every block, empty paragraphs included, and are scored exactly (round 2 allowed extra
  blocks; here an extra or missing empty paragraph is the error being measured).
- **Refusals**, one per replicate, next to tasks that look similar but can be done: shade a row (`notice1-e3`),
  center text in cells (`minutes2-e3`), 6pt between cell paragraphs (`form3-e3`), 24pt instead of empty paragraphs
  (`letter1-e3`), 5 mm high empty paragraphs (`board2-e4`), 6pt high empty paragraphs when a "low empty line" style
  exists (`report3-e4`).
- Briefs name styles by description only, and say "with the default style" for a plain paragraph (round 2's one
  Opus miss was `본문` against the default style; these seeds list no body style).

## Units

A unit is one (decision, candidate, replicate), with an id like `r3-cellpara-A-1`: 2 × 2 × 3 = 12 units, each with
5 tasks: 1 write task (`<stem><rep>-w`) and 4 edit tasks (`-e1` … `-e4`). Stems: `notice`, `minutes`, `form`
(`cellpara`) and `letter`, `board`, `report` (`emptypara`). Both candidates of a replicate share the seed content,
the task wording and the intended result; `build.py` asserts that the A and B renderings of every seed, target
and write gold parse to identical semantic models.

Refusal tasks: `notice1-e3`, `minutes2-e3`, `form3-e3`, `letter1-e3`, `board2-e4`, `report3-e4` (listed per unit in
`data/index.json`).

## Answer format and refusals

As in round 2:

```
{"answers": [{"task_id", "text"} | {"task_id", "edits": [{"old", "new"}]}]}
```

If an edit task asks for something the syntax and the names cannot express, the answer is
`{"task_id": …, "text": "REFUSE: <reason>", "edits": []}`. A refusal where the gold refuses is valid and landed;
where the gold edits, it is valid but not landed (`wrong_refusal`); with edits attached it is invalid
(`bad_refusal`). An edit where the gold refuses is invalid if it writes CSS, direct formatting or an invented name,
tag or attribute, and otherwise valid but not landed (`should_refuse`).

## Layout

- `content.py`: seeds, task wording and intended results in one form that belongs to neither syntax. Blocks
  include `['e', style]` for an empty paragraph; a table cell is a string or a list of `(style, text)` paragraphs
  (written ` // ` and `«Style»` in the table specs). Each edit is a function on a copy of the seed; a refusal edit
  is `(instruction, None, reason)`.
- `guides.py`: the guides, rendered to `guides/*.md`.
- `build.py`: renders and checks everything and writes `data/units/<unit>.json`, `data/gold/<unit>.json`,
  `seeds/<unit>.txt`, `prompts/<unit>.md` and `data/index.json`. Gold edits come from a line diff, each `old` widened
  until it occurs exactly once (as in rounds 1 and 2). It asserts that every seed is 3,000–6,000 characters.
- `score.py`: the scorer. `selftest.py`: the self-test.

Standard library only (Python 3.11 or later); the build is deterministic (a rebuild rewrites the generated files
byte for byte).

```
cd fluency/round3
python3 build.py && python3 selftest.py      # must end with SELFTEST PASSED
python3 score.py r3-cellpara-B-1 answers.json
```

Guide sizes (A / B): `cellpara` 3,658 / 3,727 chars, `emptypara` 3,286 / 3,330. Within a decision the shared part
is identical and the candidate parts have the same structure: a definition paragraph, five (cellpara) or three
(emptypara) rules, and one example with the same content.

## Scoring

The scorer parses an answer into one semantic model: the front matter and a list of blocks. A paragraph is
`{style, text}`; an **empty paragraph is a paragraph with text `""`** and its style (or null). A table is
`{style, grid}`, and each origin cell is `[r, c, paras, rowspan, colspan, is_header]` where **`paras` is the cell's
list of `[style, text]` paragraphs**. `<br/>` stays in the text as `<br/>`, so a line break never equals a
paragraph break. Pipe table versus list table is not in the model. `landed` compares the result of an edit to the
gold result; a write task must equal the brief's block list exactly. For each task the scorer prints
`{"results":[{task_id, valid, landed, error, chars, flags, detail}]}`.

Round 3 adds these flags to round 2's:

| flag | meaning |
|---|---|
| `cell_para_lost` | a cell of the result has fewer paragraphs than intended (paragraphs joined, dropped, or written with `<br/>`) |
| `br_for_p` | `<br/>` where a paragraph break was intended, or a line holding only `<br/>` where an empty paragraph was intended |
| `wrong_empty_para` | the result differs from the intended one only in its empty paragraphs (count, position or style) |
| `list_parsed_as_list` | lines that look like a list table are read as an ordinary list (no `{list-table}` line, or a blank line inside) |
| `list_table_form` | a malformed list table (row line with text, wrong indentation, stray line) |
| `cell_para_form` | `<p>` … `</p>` or another non-self-closing form inside a cell |
| `empty_cell_para` | a list-table cell paragraph `<div style="Name"></div>` with no text (cellpara B; under A an empty cell paragraph is valid, §5.2) |
| `cell_div` | `<div>` inside a pipe-table cell |
| `p_outside_cell` | `<p/>` inside a line of text outside a table cell (cellpara A; a `<p/>` line is an empty paragraph, §5.2) |
| `empty_para_form` | a malformed empty paragraph (`<p></p>`, `<p/>` inside text, `<div/>`, or the other tag pair) |

`cell_para_lost`, `br_for_p` and `wrong_empty_para` are diagnoses of a valid but wrong result (added to
`not_landed` / `brief_unmet`); the others are validation errors. `edit_ambiguous`, `edit_no_match`,
`row_width_mismatch`, `marker_misplaced`, `unknown_style`, `css_in_attr`, `invented_attr`, `attr_form`,
`attr_orphan`, `should_refuse`, `wrong_refusal` and `bad_refusal` work as in round 2.

## Self-test

`python3 selftest.py` checks:

- all 60 gold answers (12 units × 5 tasks) are valid and landed, with no flags;
- 36 broken answers are caught with their expected flag, `cellpara` 19 (9 under A, 10 under B) and `emptypara` 17
  (9 under A, 8 under B), for example: `<br/>` for a paragraph break (A and B), `<p>…</p>` in a cell, `<div>` in a
  cell (A and B), a merge that drops the second cell's paragraphs, a cell ending in `<p/>`, `<p style="6pt"/>`, a
  `<p style/>` line outside a table, a list table without its `{list-table}` line, a blank line inside a list table,
  a row with a missing cell, a continuation line indented two spaces, a covered cell with a paragraph, an `old`
  that is ambiguous because of the repeated note or the identical runs (A and B), a page break that leaves one
  empty paragraph behind, the first instead of the middle empty paragraph turned into text, a blank line written
  instead of an empty paragraph, `<br/>` lines for empty paragraphs, `<p style="height: 6pt"/>`, `<p></p>`, an
  invented `height` attribute, `<p/>` under B, `<div/>`, a `Spacer` line left in, one empty paragraph too many
  deleted, an existing style used where the answer is a refusal, a wrong refusal, and the other candidate's gold;
  a cell ending in `<p/>` is now valid (§5.2) and caught as `wrong_empty_para`;
- 9 §5.2 forms parse to the intended model under the A candidates: an empty paragraph mid-cell (plain and styled),
  a trailing one, a cell of two, an empty first paragraph, a leading `<p style/>`, a dropped leading `<p/>`, `<p/>`
  and `<p style/>` lines in a `cellpara` unit, and `<p/>` in a cell of an `emptypara` unit; two such answers score
  valid with `wrong_empty_para`;
- 12 correct answers written differently from the gold pass, 6 per decision: spaces around `<p style/>`, a merge
  done as two separate edits, a joined cell kept as a list table, a list-table conversion that also rewrites the
  style line, a move done as one wide edit, deleting a different one of identical empty paragraphs, a page break
  whose `old` carries the following title, an inserted empty paragraph set off by a blank line, a run in which the
  text sits on consecutive lines with its neighbours, and refusals worded differently;
- the command-line round trip.

## Running a round

As in rounds 1 and 2: for each unit in `data/index.json`, send `prompts/<unit>.md` as the whole user message in a
fresh conversation with no tools, save the JSON reply as `runs/<unit>/<model>-first.json` and score it. Once, send
`error` back for each invalid task and save the reply as `<model>-fix.json`. Tally per decision and candidate:
valid, landed, flags (especially `cell_para_lost`, `br_for_p`, `wrong_empty_para`, `list_parsed_as_list`,
`edit_ambiguous`), `should_refuse` / `wrong_refusal`, and the sum of `chars`.

## Design notes for the decision (not measured here)

- **Size is not neutral.** B's seeds are 5–8% longer (cellpara 3,065 / 3,251, 3,049 / 3,207, 3,027 / 3,278;
  emptypara 3,177 / 3,408, 3,023 / 3,198, 3,035 / 3,245), and an edit that makes a simple table need a
  multi-paragraph cell rewrites the whole table under cellpara B (`notice1-e1` gold: 91 chars under A, 356 under
  B). `chars` should be read with that in mind.
- **Combinations.** Choosing A for both decisions gives `<p/>` two meanings: a paragraph break inside a cell and an
  empty paragraph as a whole line. Choosing B for both keeps one element, `<div>`, for every styled or empty
  paragraph outside cell text. Choosing cellpara A with emptypara B, or the reverse, needs its own rule for an
  empty paragraph inside a cell, a shape the corpus has (see above) and this round leaves out.
- **Identity.** Neither empty-paragraph form gives an empty paragraph an identity; both rely on exact-match edits
  with unique context (§4 read-before-edit). The round measures whether models supply that context.

## Limits

- 15 tasks per cell (3 replicates × 5), 1 refusal among them; one task is 6.7 points.
- Cells never hold empty paragraphs, and `emptypara` seeds never hold multi-paragraph cells (see above).
- The scorer accepts either table form under cellpara B, blank lines inside a run of empty paragraphs, and
  whitespace between `<div>` and `</div>`: the result is scored by meaning, and a canonical-form formatter would
  normalise these. It does not accept the other candidate's tags.
- Style descriptions are the subject's only guide to which named style fits a request; the refusal golds assume
  no listed description matches.
- `chars` stands in for tokens, as before.
