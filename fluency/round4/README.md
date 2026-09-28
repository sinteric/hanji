# hanji fluency test, round 4 (DESIGN.md §10 item 6)

§10 item 6 leaves open **Spreadsheet cell-data operations: the API shape and the compressed read view**. §5.4
already fixes the workbook structure as text (`<sheet>`, `<table name range>`, one line per column with type,
format and formula), structured references in formulas, and the rule that the model never types bulk rows. It does
not say how the model reads the cells or how it writes them. Round 4 measures both, on the same three workbooks:

- `readview`: how the cells are shown (2 candidates). Every task is a question answered by reading only.
- `writeshape`: how a change is written (3 candidates). Every unit gets the same read view (readview A) so that
  only the write shape varies.

Round 4 keeps round 3's unit shape, answer JSON, refusal rule, result fields and scorer CLI. Rounds 1–3 are not
changed. `score.py` and `build.py` import round 2's scorer for edit application (`apply_edits`, `count_occ`,
`first_diff`), and `build.py` reuses round 3's `make_edits` for the text-edit gold.

## Decisions

| decision | A | B | C |
|---|---|---|---|
| `readview` | a plain window: each table's `<data>` block is a pipe table with a leading `row` column, every row, values as displayed | a compressed index in the SheetCompressor style: per table, the data rows and blank rows as anchors, then per column an inverted index of distinct raw values to runs of rows, with the format stated once | – |
| `writeshape` | range operations: `text` holds a JSON list of ops from a closed set of ten | editable range text: `edits` of exact `{old, new}` pairs on the window, row labels read-only | code: `text` holds Python against a small workbook API, run in a sandbox |

Both read views and all three write shapes sit on the same structure text (§5.4), identical in every unit. The
shared part of the guides (structure, types, formats, formulas, and for `writeshape` the window and the rules for
every change) is identical within a decision; the candidate parts have the same structure (a definition, the rules
as a list, one example on the same three-row Sales table) and never mention each other.

### readview, exactly

Both views are rendered by `wb.py` from the same in-memory workbook, deterministically.

- **A (`window_text`).** After each `<table>` block, a `<data table="Name">` block: the header row `| row | col1 | … |`,
  a delimiter row, then one line per data row in sheet order, blank rows included (`| 18 |  |  | … |`). The first
  cell is the sheet row number; every other cell is the value **as displayed** by its column's format
  (`18,930,000`, `92.3%`, `2026-03`, `00417`). Formula columns show their computed values.
- **B (`index_text`).** After each `<table>` block, an `<index table="Name" rows="2:153" blank="18,35,…">` block
  (the structural anchors: the data rows, and the blank rows as single rows or first:last runs; `blank` is left
  out when there are none). Then one line per column: `<letter> <name>`, then ` | `-separated entries
  `value: rows`. Each distinct value appears once, in order of first appearance going down; its rows are merged
  into first:last runs where consecutive, comma-separated otherwise (the inverted index; a column of one repeated
  label collapses to one entry). Empty cells are not listed. Values are **raw**, and the format is aggregated
  into the `<table>` block: text as is, dates `yyyy-mm-dd`, numbers without digit grouping, rounded to the
  precision their format displays (`0.0%` keeps three decimals). So 417 in a `00000` column is written `417`,
  displayed `00417`.

  B is lossless (`build.py` asserts that every cell's raw value, and only those, reads back from it) and drops
  nothing SheetCompressor would drop: its lossy steps (keeping only rows near anchors, replacing numbers by format
  tokens) would make exact-value questions unanswerable. Row-only addresses are used because the column letter is
  on the line.

View sizes (chars; `view_chars` in `data/index.json` and in every unit):

| workbook | rows | A | B | B/A |
|---|---|---|---|---|
| `sales1` (2026 영업실적, 3 sheets) | 190 | 12,689 | 9,902 | 0.78 |
| `roster2` (지점 인사명부, 3 sheets) | 155 | 11,102 | 10,774 | 0.97 |
| `budget3` (2027 부서별 예산, 3 sheets) | 152 | 12,138 | 10,312 | 0.85 |

B compresses repeated labels (월, 지점, 부서, 직급, 항목) and blank rows, and costs more than A on unique values
(IDs, names, phone numbers, amounts), where each value carries its row number: the roster, mostly unique columns,
barely shrinks.

### writeshape, exactly

All three write to the same in-memory model and are scored on the result, not the form.

- **A: range operations (`run_ops`).** `text` is a JSON list (a string holding the list is also accepted),
  applied in order. Exactly ten ops, keys all required unless marked optional:
  `set(range "Sheet!D70" | "Sheet!D70:E71", values [[…]] in the range's shape)`,
  `append_rows(table, rows [{column: value}])`, `insert_rows(table, before, rows)`,
  `delete_rows(table, rows "57" | "57:58")`, `fill_formula(table, column, formula)`,
  `set_type(table, column, type[, format])`, `add_column(table, column {name, type, format[, formula]})`,
  `sort(table, keys [{column, order}])`, `add_table(sheet, name, anchor, columns[, rows])`, `add_sheet(name)`.
  Values are JSON: string, number, `"YYYY-MM-DD"` (or `"YYYY-MM"`), null. Row numbers are sheet rows at the time
  the op runs. An unknown op or key is `bad_op`.
- **B: editable range text (`parse_window`).** `edits` are exact `{old, new}` pairs applied to the window text (as
  in Documents; round 2's `apply_edits`: `edit_no_match`, `edit_ambiguous`). The edited text is parsed back:
  cells are read as displayed or plain (`18,420,000` or `18420000`; `2026-10` or `2026-10-01`); the first cell
  of a data line is a **read-only label**: every labelled line must carry one of that table's seed labels, in
  strictly increasing order, else `row_labels_edited`; a new row leaves the label empty and goes where its line
  is; rows are renumbered from line order. Formula-column cells are ignored (computed). Column type, format and
  formula are edited in the `<table>` block; a type change re-reads the cells as displayed. Only the start of a
  `range` is used; its end follows from the rows (a new table may write `range="H1"`).
- **C: code (`run_code`).** `text` is Python run with `wb` and `table` in scope: `wb[sheet]`, `wb.table(name)`,
  `wb.add_sheet(name)`; `sheet.cell("D70").value`, `sheet.add_table(name, anchor, columns, rows=None)`,
  `sheet.table(name)`; `t.rows` (`row["col"]` get/set, `row.row`), `t.columns`, `t.append({…})`,
  `t.insert(before, {…})`, `t.delete_rows(first, last=None)`, `t.fill_formula(col, f)`,
  `t.set_type(col, type, format=None)`, `t.add_column(name, type, format=None, formula=None)`, `t.sort(keys)`.
  Sandbox: the AST is checked first (no `import`, no attribute or name starting with `_`, no `open`, `eval`,
  `exec`, `getattr`, `globals`, `type`, … , no `class`/`global`), builtins are a fixed safe list, and a trace
  hook stops the code after 5 s or 2,000,000 line events. Any of these is `sandbox_violation`. An API call that
  breaks a rule raises with the same flag as the matching op (so `a1_formula`, `type_mismatch`, … mean the same
  under A and C); another exception is `runtime_error`.

Rules shared by all three (in every guide, and checked on every result):

- Types are never guessed (§8): IDs, codes and phone numbers are text; a type change converts values **as
  displayed** (417 in `00000` → `"00417"`). A value of the wrong JSON type is `type_mismatch`.
- A formula column's cells are computed, never set (`formula_cell`). Formulas use structured references only:
  `[@Col]`, `[@[Col name]]`, `[@[First]:[Last]]`, `Table[Col]`, `Table[@Col]`; functions SUM, SUMIFS,
  COUNTIFS, AVERAGE, MIN, MAX, ROUND, ABS, IF, IFERROR. Any A1 reference (`D2`, `$D$2`, `D2:D9`, `매출!D2`) is
  `a1_formula`; an unknown table or column `unknown_ref`; an unknown function `unknown_function`.
- A value starting with `=` is refused (§8 formula injection): `a1_formula` when it holds A1 references (the
  "A1 copies" failure), else `formula_in_cell`.
- Fetching functions (WEBSERVICE, FILTERXML, HYPERLINK, IMPORTDATA, RTD, …) are `fetch_function`.
- Formatting is by type and number format only (§5.1): the closed format list is `#,##0`, `#,##0.0`, `0`, `0.0`,
  `0.0%`, `0000`, `00000`, `000000`, `yyyy-mm`, `yyyy-mm-dd`, `@` (`bad_format` otherwise).
- No answer adds more than 50 rows (§5.4, the model never types bulk rows): `bulk_typed`, whatever the shape (a
  JSON list, typed lines, or a loop).
- Table names are unique and tables on a sheet never overlap (`duplicate_name`, `table_overlap`); a formula that
  evaluates to an error is `formula_error`.

Guide sizes (chars; candidate part in brackets): readview A 2,784 (1,319), B 2,810 (1,345); writeshape A 6,477
(2,394), B 6,373 (2,290), C 6,365 (2,282).

## The workbooks

Generated by `content.py` with fixed seeds (standard `random`), 150–400 data rows across 3 sheets each:

- **`sales1` — 2026 영업실적.xlsx.** 매출/`Sales` (월 × 5 지점 × 3 제품군, a 소계 row per month and a blank row
  between months, 135 rows; 이익 typed by hand, one row with two digits swapped); KPI/`KPI` (40 rows, 달성률 a
  formula `=[@실적]/[@목표]`); 담당자/`Staff` (15 rows, 사번 wrongly typed as a number with format `00000`, 휴대전화
  text). The 매출 of 서초 가전 is 18,240,000 in both 2026-05 and 2026-06 (near-duplicate rows).
- **`roster2` — 지점 인사명부.xlsx.** 명부/`Roster` (6 지점 × 17 people with a blank row between branches, 107 rows;
  사번 wrongly a number with format `000000`; 휴대전화 and 내선 `0412`-style text); 교육/`Training` (30 rows, 사번
  correctly text); 요약/`Headcount` (18 rows; one 현원 disagrees with the roster). 김서연 works in both 부산 and 대구,
  and both took 개인정보 보호 on the same day, on adjacent rows; 이민준 appears twice too.
- **`budget3` — 2027 부서별 예산.xlsx.** 예산/`Budget` (5 부서 × 16 accounts, a 소계 row and a blank row per
  department, 89 rows; 계정코드 wrongly a number with format `0000`; 합계 typed, one row off by 1,000,000; 복리후생비
  repeated in every department, and its 3분기 is the same figure for 영업 and 개발); 인건비/`Payroll` (25 rows, 인건비
  a formula `=[@인원]*[@[1인당 연봉]]`); 집행/`Spend` (3 months with blank rows between, 38 rows; 계정코드 text).

## Tasks

A unit is one (decision, candidate, replicate), id `r4-<decision>-<candidate>-<rep>`: readview 2 × 3 and
writeshape 3 × 3, 15 units. Replicate *r* uses workbook *r*; all candidates of a decision and replicate share the
workbook, the task wording and the intended result.

**readview**, 5 questions per replicate (`<stem>-q1` … `-q5`), answered as `"text": "ANSWER: <value>"`:

| id | kind | sales1 | roster2 | budget3 |
|---|---|---|---|---|
| q1 | one cell by row labels | 매출 of 송파 모바일 2026-07 | 점수 of 부산 김서연 in 개인정보 보호 (the 대구 one is on the next row) | 2분기 of 개발 지급수수료 |
| q2 | rows matching a condition | KPI rows with 달성률 < 95.0% | 대구 people with 직급 대리 | Spend rows of 2027-02 with 집행액 > 5,000,000 |
| q3 | the row that disagrees with its parts | the Sales row whose 이익 ≠ 매출 − 원가 | the Headcount row whose 현원 ≠ its Roster count | the Budget row whose 합계 ≠ 1분기+…+4분기 |
| q4 | an ID, leading zeros exact | Staff 사번 (a number shown `00000`: B shows `3609`) | Training 사번 (text) | Spend 계정코드 (text) |
| q5 | the range a formula must cover | 원가 of the 2026-03 branch rows, no 소계 | 이름 of the 광주 people | 4분기 of 영업's accounts, no 소계 |

**writeshape**, 6 tasks per replicate: 1 write (`-w`), 4 edits (`-e1` … `-e4`) and 1 refusal (`-e5`):

| id | sales1 | roster2 | budget3 |
|---|---|---|---|
| w: new table, 3 typed columns + a structured formula column | `Returns` at 매출!H1, 반품률 = 반품액 / SUMIFS(Sales[매출], …) | `Overtime` at 요약!G1, 1인당 = 초과근무 / COUNTIFS(Roster[지점], …) | `Hiring` at 인건비!H1, 추가 인건비 = 충원 인원 × 1인당 연봉 |
| e1: one figure by labels, next to near-duplicates | 매출 of 서초 가전 2026-05 (2026-06 has the same figure) | 점수 of 대구 김서연 (부산 김서연 on the row above) | 3분기 of 개발 복리후생비 (영업 has the same figure) |
| e2: append a month of rows | KPI 2026-09, 5 rows | Training 2026-09, 4 rows with text 사번 | Spend 2027-04, 5 rows with text 계정코드, one 비고 empty |
| e3: fill a formula down, structured not A1 | Sales 이익 = 매출 − 원가 | Headcount 결원 = 정원 − 현원 | Budget 합계 = SUM(1분기:4분기) |
| e4: fix a number column holding leading-zero IDs | Staff 사번 → text | Roster 사번 → text | Budget 계정코드 → text |
| e5: refusal | a 환율 column with `=WEBSERVICE(…)` | add 714 placeholder rows to Training (7 per person) | colour rows over 100,000,000 red |

Write briefs name the sheet, table name, top-left cell, every column's type and format, the formula in words, and
every row. The gold is written once as ops and once as code in `content.py`; the text-edit gold is derived from the
ops result by a line diff (each `old` widened until it occurs once, as in rounds 1–3), keeping the seed's `range`
text and leaving formula cells of new rows empty. `build.py` asserts that the three golds give the same workbook
for every task, that every task changes the workbook, and that the window parses back to the seed.

## Answer format and refusals

As in rounds 2 and 3:

```
{"answers": [{"task_id", "text"} | {"task_id", "edits": [{"old", "new"}]}]}
```

readview: `"text": "ANSWER: <value>"`; writeshape A: `"text": [ops]`; B: `"edits"`; C: `"text": "<code>"`. A task
that cannot be done as asked is answered `{"task_id": …, "text": "REFUSE: <reason>", "edits": []}`. A refusal
where the gold refuses is valid and landed; where the gold does not, it is valid but not landed (`wrong_refusal`);
with edits attached it is invalid (`bad_refusal`). An answer where the gold refuses is invalid if it breaks a rule
(`fetch_function`, `bulk_typed`, …, plus `should_refuse`), and otherwise valid but not landed (`should_refuse`).
Readview questions never refuse in the gold; the prompt still states the rule.

## Scoring

`python3 score.py <unit_id> <answers.json>` prints `{"results": [{task_id, valid, landed, error, chars, flags,
detail}]}`.

- **readview.** `valid`: the text is `ANSWER: <value>` (prefix case-insensitive). `landed`: the value equals an
  accepted value after normalising **whitespace and digit grouping only** (runs of whitespace to one space, none
  next to a comma; a comma between digits followed by exactly three digits dropped). `chars` is the answer length;
  the input cost is `view_chars`.
- **writeshape.** `valid`: the answer parses or runs, and the result is a well-typed workbook (every rule above).
  `landed`: the result's semantic model equals the gold's. The model is sheets (in order) → tables (by name) →
  anchor, columns `[name, type, format, is_formula]`, and rows of values, with formula columns compared by their
  **computed values**, so any formula equivalent on the data lands (`=-[@원가]+[@매출]`, SUMIFS criteria in any
  order, `SUM([@[1분기]:[4분기]])` or four `+`). Row labels and range text are not in the model. `chars`: the answer
  text length (A: the JSON list), or the sum of `old` and `new` (B).

| flag | meaning |
|---|---|
| `wrong_answer` | a read answer that is not the value asked for |
| `answer_form` | a read answer without `ANSWER:` |
| `id_lost_zeros` | an ID or code lost its leading zeros (a read answer `3609` for `03609`, or a text cell `"34884"` for `"034884"`) |
| `wrong_cell` | a valid result that differs from the gold in cells of tables of the same shape (the edit hit another cell, e.g. a near-duplicate row) |
| `formula_missing` | a column that should be a formula column is not (values typed instead) |
| `no_change` | the result equals the seed |
| `a1_formula` | an A1 reference in a formula, or an A1 formula typed into a cell |
| `formula_in_cell` | another value starting with `=` |
| `fetch_function` | WEBSERVICE and other fetching functions |
| `bulk_typed` | more than 50 rows added |
| `row_labels_edited` | (B) a row label changed, invented, reordered or put on a new row |
| `edit_no_match`, `edit_ambiguous` | (B) as in rounds 2–3 |
| `window_form`, `data_header_mismatch`, `row_width_mismatch` | (B) the edited text does not read back as a workbook |
| `sandbox_violation` | (C) import, file, network, introspection, `_` names, or the time limit |
| `runtime_error`, `syntax_error` | (C) the code fails |
| `bad_json`, `bad_op`, `bad_range` | (A) not a list of known ops with their keys, or a bad address |
| `type_mismatch`, `bad_type`, `bad_format`, `formula_cell`, `header_cell`, `outside_table`, `unknown_ref`, `unknown_function`, `bad_formula`, `formula_error`, `circular_ref`, `duplicate_name`, `table_overlap` | rule checks shared by all shapes |
| `not_landed`, `should_refuse`, `wrong_refusal`, `bad_refusal`, `missing_answer`, `wrong_answer_kind` | as in rounds 2–3 |

`wrong_answer`, `id_lost_zeros` (read), `not_landed`, `wrong_cell`, `id_lost_zeros` (write), `formula_missing`,
`no_change`, `should_refuse` and `wrong_refusal` are diagnoses of a valid answer; the others are validation errors
(the `error` text is written for the model: the line, the cell or the op, and the allowed form).

## Self-test

`python3 selftest.py` must end with `SELFTEST PASSED`. It checks:

- all 84 gold answers (6 × 5 questions + 9 × 6 tasks) are valid and landed, with no flags;
- 36 broken answers are caught with their expected flag: readview 8 (a wrong value, a near-duplicate's value
  (the other 김서연), a wrong row, a range one row too wide, no `ANSWER:`, a refusal, and an ID without its zeros
  under A and B) and writeshape 28 (A 8, B 9, C 11), among them: the near-duplicate row set instead of the asked
  one (A, B, C: `wrong_cell`), A1 formulas in `fill_formula`, `add_table` and code, and A1 copies typed into
  cells (B), IDs losing zeros after a type change (A, B, C), invented labels on new rows and a changed label (B),
  an `old` with a missing space and an ambiguous `old` (B), `import os`, `open`, `().__class__` and an endless
  loop (C), 1,008 rows by op, 60 typed lines and 714 rows by a loop (`bulk_typed`), WEBSERVICE by op and by code
  (`fetch_function`), a number where a text code is due (`type_mismatch`), a colour done as a text column
  (`should_refuse`), and wrong refusals;
- 20 correct answers written differently from the gold pass: readview 4 (ungrouped digits, lower-case prefix
  with extra spaces, no space after the colon) and writeshape 16 (a two-cell `set`, a reordered formula, SUMIFS
  criteria swapped with `YYYY-MM` dates and the op list given as a JSON string, `set_type` without a format, a
  `+` formula for `SUM`, a refusal worded differently; a spec-line-only formula edit, new rows with plain numbers
  and junk in formula cells, `range="H1"`, an empty text format, a shorter unique `old`; a label-based row loop,
  an explicit `@` format, `add_table` with a comprehension and `[[지점]]`, `SUM([@[1분기]:[4분기]])` without the
  table name, appends from tuples with `None`);
- the command-line round trip.

## Running a round

As in rounds 1–3: for each unit in `data/index.json`, send `prompts/<unit>.md` as the whole user message in a
fresh conversation with no tools, save the JSON reply as `runs/<unit>/<model>-first.json` and score it. Once, send
`error` back for each invalid task and save the reply as `<model>-fix.json`. Tally per decision and candidate:
valid, landed, flags (especially `wrong_answer`, `id_lost_zeros`, `wrong_cell`, `a1_formula`,
`row_labels_edited`, `edit_no_match`, `sandbox_violation`, `bulk_typed`), `should_refuse` / `wrong_refusal`, the
sum of `chars` and, for readview, `view_chars` as the input cost.

## Layout

- `wb.py`: the workbook model: types and formats, the formula tokenizer, parser and evaluator, the ten range
  operations, the code API and sandbox, the two renderings (`window_text`, `index_text`) and the window parser.
- `content.py`: the three workbooks, questions and tasks, with gold as ops and as code.
- `guides.py`: the guides, rendered to `guides/*.md`.
- `build.py`: renders and checks everything; writes `data/units/<unit>.json` (with the seed workbook and its view),
  `data/gold/<unit>.json`, `views/<unit>.txt`, `prompts/<unit>.md` and `data/index.json`.
- `score.py`: the scorer. `selftest.py`: the self-test.

Standard library only (Python 3.11 or later); the build is deterministic (a rebuild rewrites the generated files
byte for byte).

```
cd fluency/round4
python3 build.py && python3 selftest.py      # must end with SELFTEST PASSED
python3 score.py r4-writeshape-C-2 answers.json
```

## Design notes for the decision (not measured here)

- **Size is not neutral in either direction.** B's view is 3–22% smaller than A's, depending on how much of a
  workbook is repeated labels and blank rows; a real workbook with long runs of empty cells would favour B more,
  a contact list less. Under writeshape, B's gold answers are short for cell and column edits (one line) and long
  for new tables and appended rows (every row typed as a line), A's and C's the reverse.
- **What each shape makes the model count.** A needs a cell address for `set` (column letter from the range and
  the column order, row from the window); B needs none (the line carries the label) but must keep labels and
  pipe form exact; C can find a row by its labels in a loop and never needs an address.
- **Safety is in the model, not the shape.** Every §8 rule is checked on the resulting workbook, so a shape cannot
  bypass it; C additionally needs the sandbox, which A and B do not.
- **Not in the model.** Row order within `sort` ties, cell styles, charts, validation and merged cells are out of
  scope; so are totals rows and `#This Row`-style special items.

## Limits

- 15 tasks per readview cell and 18 per writeshape cell (3 replicates), 1 refusal per writeshape replicate; one
  task is 6.7 or 5.6 points.
- The formula language is a subset (ten functions, structured references only), evaluated by `wb.py`, not by a
  spreadsheet engine; results are compared by value, so an equivalent formula that the subset cannot parse fails
  (`unknown_function`).
- The window parser accepts `|`-separated cells with any spacing and a delimiter row of dashes; the index view and
  the window never contain `|` or `: ` inside a value (asserted).
- Workbooks are generated, realistic in shape (Korean labels, blank rows, subtotals, repeated labels, typed-by-hand
  errors) but not taken from a corpus.
- `chars` stands in for tokens, as before.
