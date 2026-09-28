# Fluency test results, round 4

## Round 4 — 2026-09-28

**Subjects:** Claude Opus (`claude-opus-5-5`) and Claude Sonnet (`claude-sonnet-5`). Each was sent all 15 blind
units: `readview` 2 candidates × 3 workbooks × 5 questions (30 tasks per model) and `writeshape` 3 candidates × 3
workbooks × 6 tasks (54 tasks per model), 168 answers in all. No tools, a fresh conversation per unit, as in
[README.md](README.md#running-a-round). Opus had no invalid first answer; Sonnet had five, all in `writeshape`, and
each was sent back once with its validator error (the fix round).

**Sonnet's `writeshape` units were re-run.** In the first run the nine `writeshape` prompts (23,251–23,560 chars,
the only ones over 20,000) reached Sonnet as a pointer to `prompts/<unit>.md` instead of the text, with an
instruction not to read files, so it refused every task ("the actual spreadsheet content … was never inlined here").
Those answers measured the relay, not the write shape, and are discarded. The re-run told each Sonnet subject to read
only its own prompt file, `prompts/<unit>.md`, which is how Opus's `writeshape` subjects had received the same
prompts in the first run, so both models now saw identical text by the same route. Sonnet's six `readview` prompts
(14,777–18,832 chars) had arrived whole the first time and were not re-run; Opus was not re-run.

Answers: `runs/<unit>/<model>-first.json`, and `runs/<unit>/sonnet-fix.json` for the four units where a Sonnet
answer was sent back (the re-run's files replace the void ones). Re-score any of them with
`python3 score.py <unit> <file>`; a re-score of every file reproduces the numbers here. The kit's self-test passes
(`python3 selftest.py`: 84 gold answers valid and landed; broken readview 8/8 and writeshape 28/28 caught (A 8,
B 9, C 11); alternatives 4/4 and 16/16 pass; cli ok).

### What round 4 decides

§10 item 6: **spreadsheet cell data**, how the model reads cells (`readview`) and how it writes them (`writeshape`).
Both sit on the §5.4 structure text (`<sheet>`, `<table name range>`, one line per column with type, format and
formula), identical in every unit. Every `writeshape` unit reads the cells through readview A, so only the write
shape varies. Exact definitions are in [README.md](README.md#decisions); the wording each subject saw is in
`guides/<decision>-<candidate>.md`.

| decision | A | B | C |
|---|---|---|---|
| `readview` | plain window: per table a `<data>` pipe table with a leading `row` column, every row (blank ones too), values as displayed | compressed index (SheetCompressor style): per table the data rows and blank rows as anchors, then per column an inverted index `value: rows` of raw values, format stated once | – |
| `writeshape` | range operations: a JSON list of ops from a closed set of ten (`set`, `append_rows`, `fill_formula`, `set_type`, `add_table`, …) | editable range text: exact `{old, new}` edits on the window, row labels read-only | code: Python against a small workbook API (`wb`, `table(...)`, `rows`, `append`, `fill_formula`, `set_type`, …) in a sandbox |

The three workbooks (150–190 data rows over 3 sheets each: `sales1` 2026 영업실적, `roster2` 지점 인사명부,
`budget3` 2027 부서별 예산) carry the traps §5.4 and §8 care about: near-duplicate rows with the same figure, a
number column holding leading-zero IDs, a typed column with one wrong row, subtotal and blank rows inside a table,
and one refusal per `writeshape` replicate (a `WEBSERVICE` column, 714 placeholder rows, rows coloured red).

### First-try validity

Valid answers per cell (15 per `readview` cell, 18 per `writeshape` cell):

| model | readview A | readview B | writeshape A | writeshape B | writeshape C |
|---|---|---|---|---|---|
| Opus | 15 | 15 | 18 | 18 | 18 |
| Sonnet | 15 | 15 | 18 | **15** | **16** |

### Landed

Answers whose value (`readview`) or resulting workbook (`writeshape`) equals the intended one, first try:

| model | readview A | readview B | writeshape A | writeshape B | writeshape C |
|---|---|---|---|---|---|
| Opus | 15 | 15 | 18 | 18 | 18 |
| Sonnet | 15 | **14** | 18 | **15** | **16** |

Opus: 84/84. Sonnet: 78/84 first try (29/30 `readview`, 49/54 `writeshape`), 83/84 after the fix round. Every
valid `writeshape` answer landed; Sonnet's five `writeshape` misses are all invalid answers the validator caught.

### Fix rate after one validator error

| model | writeshape A | writeshape B | writeshape C |
|---|---|---|---|
| Opus | – (0 invalid) | – (0 invalid) | – (0 invalid) |
| Sonnet | – (0 invalid) | 3/3 fixed, 3/3 landed | 2/2 fixed, 2/2 landed |

All five sent-back answers were fixed in one round and landed, each by the change the error named (−23 to +1 chars;
the −23 is `roster2-w`, where Sonnet also emptied the formula cells it had filled with computed values). The one miss that remains (Sonnet, `readview` B) is a valid answer with the wrong value, which no
validator can catch.

### Size

Answer size in characters, summed over the tasks of each cell (`chars`: the answer text, the JSON op list, the code,
or the sum of `old` + `new`), and the input cost of the read view (`view_chars`, summed over the three workbooks):

| model | decision | A | B | C |
|---|---|---|---|---|
| both | readview input (`view_chars`) | 35,929 | 30,988 (−14%) | – |
| Opus | readview answers | 188 | 188 | – |
| Sonnet | readview answers | 188 | 188 | – |
| Opus | writeshape answers | 3,897 | 3,869 (−1%) | 3,417 (−12%) |
| Opus | writeshape without the 3 refusals | 3,569 | 3,493 (−2%) | 3,100 (−13%) |
| Sonnet | writeshape answers (first try) | 3,995 | 4,114 (+3%) | 3,770 (−6%) |
| Sonnet | writeshape without the 3 refusals | 3,569 | 3,731 (+5%) | 3,410 (−4%) |

Per workbook the index view saves 22% on `sales1` (12,689 → 9,902), 3% on `roster2` (11,102 → 10,774) and 15% on
`budget3` (12,138 → 10,312): it compresses repeated labels and blank rows and pays a row number on every unique
value, so a roster of names and phone numbers barely shrinks. Read answers are the same in both views
(`ANSWER: <value>`).

`writeshape` answers by task kind, summed over the three workbooks, Opus / Sonnet (first try):

| task | A (ops) | B (edits) | C (code) |
|---|---|---|---|
| `-w` new table with a formula column | 1,560 / 1,560 | 1,675 / 1,741 | 1,516 / 1,584 |
| `-e1` one cell next to a near-duplicate | 168 / 168 | 426 / 426 | 105 / 213 |
| `-e2` append a month of rows | 1,314 / 1,314 | 1,058 / 1,106 | 1,189 / 1,329 |
| `-e3` fill a formula down | 268 / 268 | 192 / 316 | 160 / 159 |
| `-e4` number column → text IDs | 259 / 259 | 142 / 142 | 130 / 125 |

Under A the two models wrote the same ops to the character on every non-refusal task. The differences under B and C
are strategy, not the shape: for `roster2-e1` under C Sonnet found 김서연's row by its keys in a loop
(`for row in t.rows: if row["이름"] == "김서연" and row["지점"] == "대구" and …`, 139 chars) where Opus wrote
`wb["교육"].cell("G20").value = 88`, and for `sales1-e3` under B it added a second edit to a formula cell of row 64,
which is ignored.

This is the shape each candidate was expected to have: B (edits) is largest for one cell, because the `old` is the
whole row line (`| 73 | 2026-05 | 서초 | 가전 | 18,240,000 | 12,630,000 | 5,610,000 |`) where A writes
`{"op": "set", "range": "매출!D73", "values": [[18420000]]}` and C `wb["매출"].cell("D73").value = 18420000`; B is
smallest for appended rows (one pipe line per row, no keys). C is smallest or close to it everywhere, and for
column-level changes B's one spec line (`| 사번 | number | 00000 |  |` → `| 사번 | text | @ |  |`) and C's one call
(`table("Staff").set_type("사번", "text", "@")`) are about half the size of A's JSON op.

### Refusals

All nine Opus refusal tasks refused with the right rule, under every write shape: `WEBSERVICE` ("WEBSERVICE is a
data-fetching function, and the rules do not allow it in formulas", C-1), 714 rows ("Adding 714 rows exceeds the
50-row limit per answer; bulk rows must come from an import", A-2) and red rows ("Formatting is limited to type and
number format, so rows cannot be coloured red", A-3). Sonnet's nine did the same (for example B-2: "Adding 714 rows
would exceed the documentation's limit of no more than 50 added rows per answer"). No `wrong_refusal` or
`should_refuse` from either model.

### Failure patterns

Six real misses in round 4, all Sonnet's: one silent wrong value in `readview` B and five invalid `writeshape`
answers in two patterns, one per candidate. All five invalid ones were caught by the validator and fixed in one round.

1. **Adjacent column line in the index view** (Sonnet, `r4-readview-B-3` `budget3-q1`, flag `wrong_answer`). Asked
   for the 2분기 of 개발's 지급수수료, Sonnet answered `ANSWER: 15,710,000`, detail "answered '15,710,000'.". The row
   is 44 (지급수수료 is `8,26,44,62,80` on the `C 항목` line, 개발 is 38–54 on `B 부서`); the right value is on the
   next line, `E 2분기 | … | 12510000: 44 | …`, and `15710000: 44` is the `D 1분기` entry. Sonnet found the row and
   read it from the neighbouring column's line. In view A the same row is one line,
   `| 44 | 0471 | 개발 | 지급수수료 | 15,710,000 | 12,510,000 | 15,010,000 | 14,340,000 | 57,570,000 |`, and both
   models answered 12,510,000 under A. The mechanism is B's: a cell is the crossing of two row sets (항목 and 부서)
   and then a lookup on a third line among 80 entries of like-sized numbers, so an off-by-one line gives a
   plausible wrong figure with no validation error.
2. **Row labels written on a new table** (Sonnet, write shape B, `-w` in all three workbooks, flag
   `row_labels_edited`, 3 of 18). Asked to add a table with a formula column, Sonnet wrote the new table's `<data>`
   block with the rows numbered from the sheet row after the header, `| 2 | 2026-08 | 강남 | 1,240,000 |  |`,
   `| 3 | 2026-08 | 서초 | 860,000 |  |`, … (`sales1-w`; `roster2-w` and `budget3-w` the same), and was refused
   with "line 166: row label '2' was changed or invented. Row labels are read-only: keep each existing row's label,
   in order, and leave the row cell empty on a new row." The guide says a new row leaves the label empty, but its only
   example is a row added to an existing table, and its line on adding a table does not repeat it; in a table that
   does not exist yet, numbering the rows by the sheet rows they will occupy is a natural reading. In `roster2-w` it also typed computed values into the
   formula column (`| 2 | 서울본점 | 2026-08 | 412 | 24.2 |`), which the rules ignore. Each fix emptied the label
   cells (`|  | 2026-08 | 강남 | 1,240,000 |  |`) and landed. Same task, same mistake, three separate
   conversations: a B-specific trap, not noise in the sense of a random slip. Appended rows in existing tables
   (`-e2`) were written with empty labels every time.
3. **Formula without its `=`** (Sonnet, write shape C, both in `r4-writeshape-C-2`, flag `bad_formula`, 2 of 18).
   `sheet.add_table("Overtime", "G1", [ … {"name": "1인당", …, "formula": "[@초과근무]/COUNTIFS(Roster[지점],[@지점])"}])`
   ("column 1인당: a formula starts with "=".") and `table("Headcount").fill_formula("결원", "[@정원]-[@현원]")`
   ("fill_formula: the formula is text starting with "=", not '[@정원]-[@현원]'."); each fix added the `=` and
   landed. In `sales1` and `budget3` Sonnet wrote `"=[@매출]-[@원가]"` and `"=SUM([@[1분기]:[4분기]])"` under the same
   shape, so this is one conversation's habit, and both misses are correlated. The C guide's API lines give no formula
   example (A's shows `"formula": "=[@매출]-[@원가]"`), so the guide may share the blame; the formulas themselves
   were right.

Nothing failed on what round 4 was built to provoke, in any answer: no near-duplicate row hit instead of the
asked one (`wrong_cell`: 0, including 서초 가전 18,240,000 in 2026-05 and 2026-06 and the two 김서연 rows), no ID
losing its zeros when read (`03609`, `068328`, `0431` were right under both views, and view B shows the first one
raw as `3609`) or after a type change (`id_lost_zeros`: 0), no A1 formula (`a1_formula`: 0; every `fill_formula` and
new formula column used structured references such as `=[@[충원 인원]]*[@[1인당 연봉]]` and
`=[@반품액]/SUMIFS(Sales[매출],Sales[월],[@월],Sales[지점],[@지점])`), no unmatched or ambiguous `old`
(`edit_no_match`, `edit_ambiguous`: 0), no sandbox or runtime error under C
(`sandbox_violation`, `runtime_error`, `syntax_error`: 0), no bad op or JSON under A (`bad_op`, `bad_json`,
`bad_range`: 0), and no `type_mismatch`, `bulk_typed` or `fetch_function`. The only flags are the three
`row_labels_edited` and two `bad_formula` above.

### Decisions

| decision | round 4 | verdict |
|---|---|---|
| read view | A 30/30, B 29/30 (both models); B −14% input (−3% to −22% by workbook); B's one miss is B-specific and silent | **A (plain window)**. The 30 vs 29 is within noise; A is chosen on the kind of error, not the score |
| write shape | first try A 36/36, B 33/36, C 34/36 (Opus 18/18/18, Sonnet 18/15/16); all 54 land after one fix on both models; size vs A: B −1% / +3%, C −12% / −6% (Opus / Sonnet) | **A (range operations)**. The only shape with no first-try error on either model; the validity gaps and C's size lead are within noise |

- **readview: A.** 29/30 against 30/30 is one task in one conversation, well within noise, so the scores do not
  decide it. The kind of error does: B's miss is the lookup only the index view asks for (find the row from two row
  lists, then read a third line), and it produced a plausible wrong figure that no validator can flag, the worst kind
  of error for §8; A's one line per row cannot fail that way. B's 14% input saving (3% to 22% by workbook) is real
  but modest at this size, and small next to a silent wrong answer. The saving grows with repeated labels and empty
  runs, which large real sheets have, and the round did not test sheets where the whole window does not fit, so the
  question reopens at scale (see Next round), not here.
- **writeshape: A.** On Opus the three shapes tie (18/18 each, no flag). On Sonnet A is 18/18, B 15/18 and C 16/18
  first try; as a comparison of cells that is within noise (A vs B pooled over both models, 36/36 against 33/36:
  Fisher two-sided p ≈ 0.24; A vs C higher still), and every miss was caught and fixed in one round, so all three
  shapes end at 54/54 landed. What separates them is that B's and C's misses are systematic, not random: B's
  row-label rule tripped Sonnet on the same task in all three workbooks, and C's formulas need an `=` that the C API
  does not show. A, where the same rules sit in explicit op fields, drew no error on either model, and the two models
  wrote identical ops on every non-refusal task. Size does not overturn this: B is no smaller than A overall (−1% on
  Opus, +3% on Sonnet), and C's lead (−12% and −6%) is within what strategy alone moves (Sonnet's one keyed loop made
  C larger than A on `-e1`). B's two real advantages, appended rows and column changes, and C's
  general compactness remain true by task kind. The design notes in
  [README.md](README.md#design-notes-for-the-decision-not-measured-here) still apply: A and C need a cell address
  for a single cell (neither model got one wrong here), and C also needs the sandbox, which A does not. If a later
  round with other models shows A's errors, C is the runner-up; B's new-table labels should be fixed in its guide
  before it is compared again.

### Limits

- **Claude models only**, Opus and Sonnet. GPT and Gemini were not run, so §6's "at least Claude, GPT, Gemini" is
  still not met.
- **Few tasks per cell.** 15 per `readview` cell and 18 per `writeshape` cell, so one task is 6.7 or 5.6 points,
  and the tasks of a unit share one conversation, so their errors are correlated.
- **One run per prompt** per model. Sonnet's `writeshape` answers come from a re-run (see the top); the fix round
  was exercised only by Sonnet's five invalid answers.
- **An in-memory workbook model**, not a real xlsx engine: `wb.py` evaluates a formula subset (ten functions,
  structured references only) and results are compared by value; no workbook was round-tripped through a real xlsx
  file or a spreadsheet application.
- **Workbooks of a few hundred rows, not 100k.** 150–190 data rows per workbook, generated by `content.py` (realistic
  in shape, not taken from a corpus). Both views show every row here; at 100k rows neither can, and the question
  that decides a read view at scale (which rows to show, and how the model asks for more) is not tested.
- `chars` stands in for tokens and also measures strategy (how wide an `old` a model picks, whether it loops or
  lists).

### Next round

- Deliver long prompts the same way to every subject (the file route both models used for `writeshape`), and check
  before scoring that no subject refused for a missing prompt.
- A `readview` round on large sheets (thousands to 100k rows) where the view is a window or an index over a subset,
  with lookups that cross column lines, to see whether B's miss repeats and whether its saving grows.
- Haiku; GPT and Gemini when keys exist; repeated runs per prompt.
