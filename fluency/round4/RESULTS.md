# Fluency test results, round 4

## Round 4 — 2026-09-28

**Subjects:** Claude Opus (`claude-opus-5-5`) and Claude Sonnet (`claude-sonnet-5`). Each was sent all 15 blind
units: `readview` 2 candidates × 3 workbooks × 5 questions (30 tasks per model) and `writeshape` 3 candidates × 3
workbooks × 6 tasks (54 tasks per model), 168 answers in all. No tools, a fresh conversation per unit, as in
[README.md](README.md#running-a-round). No first answer was invalid, so the fix round was not used.

**Sonnet's `writeshape` answers are void.** In all nine `writeshape` units Sonnet did not receive the prompt: the
relay that ran the round passed it a pointer to `prompts/r4-writeshape-*.md` (23,251–23,560 chars, the only prompts
over 20,000) instead of the prompt text, and told it not to read files. Sonnet refused every task and said why, for
example `r4-writeshape-A-1` `sales1-w`: "REFUSE: … the actual spreadsheet content (workbook text, structural rules,
and task definitions) was never inlined here — only a reference to an external prompt file … that I was told not to
open." The scorer counts these as 18 valid and 3 landed per candidate (15 `wrong_refusal`; the 3 `-e5` refusals
land only because everything was refused). They measure the relay, not the write shape, and are left out of every
table and verdict below. Sonnet's six `readview` prompts (14,777–18,832 chars) arrived whole and its answers are
real. Opus's 15 units are all real answers.

Answers: `runs/<unit>/opus-first.json` and `runs/<unit>/sonnet-first.json` (the nine void Sonnet `writeshape` files
are kept as returned; a re-run replaces them). Re-score any of them with `python3 score.py <unit> <file>`; a
re-score of every file reproduces the numbers here. The kit's self-test passes (`python3 selftest.py`: 84 gold
answers valid and landed; broken readview 8/8 and writeshape 28/28 caught (A 8, B 9, C 11); alternatives 4/4 and
16/16 pass; cli ok).

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
| Sonnet | 15 | 15 | void | void | void |

### Landed

Answers whose value (`readview`) or resulting workbook (`writeshape`) equals the intended one, first try:

| model | readview A | readview B | writeshape A | writeshape B | writeshape C |
|---|---|---|---|---|---|
| Opus | 15 | 15 | 18 | 18 | 18 |
| Sonnet | 15 | **14** | void | void | void |

Opus: 84/84. Sonnet: 29/30 on `readview`; `writeshape` not measured.

### Fix rate after one validator error

No first answer was invalid in any real cell (`invalid`: 0), so the fix round had nothing to fix and the fix rate is
not measured this round. The one miss (Sonnet, `readview` B) is a valid answer with the wrong value, which no
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

Per workbook the index view saves 22% on `sales1` (12,689 → 9,902), 3% on `roster2` (11,102 → 10,774) and 15% on
`budget3` (12,138 → 10,312): it compresses repeated labels and blank rows and pays a row number on every unique
value, so a roster of names and phone numbers barely shrinks. Read answers are the same in both views
(`ANSWER: <value>`).

Opus's `writeshape` answers, by task kind, summed over the three workbooks (A / B / C):

| task | A (ops) | B (edits) | C (code) |
|---|---|---|---|
| `-w` new table with a formula column | 1,560 | 1,675 | 1,516 |
| `-e1` one cell next to a near-duplicate | 168 | 426 | 105 |
| `-e2` append a month of rows | 1,314 | 1,058 | 1,189 |
| `-e3` fill a formula down | 268 | 192 | 160 |
| `-e4` number column → text IDs | 259 | 142 | 130 |

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
number format, so rows cannot be coloured red", A-3). No Opus `wrong_refusal` or `should_refuse`.

### Failure patterns

There is one real miss in round 4, plus the void Sonnet `writeshape` units:

1. **Adjacent column line in the index view** (Sonnet, `r4-readview-B-3` `budget3-q1`, flag `wrong_answer`). Asked
   for the 2분기 of 개발's 지급수수료, Sonnet answered `ANSWER: 15,710,000`, detail "answered '15,710,000'.". The row
   is 44 (지급수수료 is `8,26,44,62,80` on the `C 항목` line, 개발 is 38–54 on `B 부서`); the right value is on the
   next line, `E 2분기 | … | 12510000: 44 | …`, and `15710000: 44` is the `D 1분기` entry. Sonnet found the row and
   read it from the neighbouring column's line. In view A the same row is one line,
   `| 44 | 0471 | 개발 | 지급수수료 | 15,710,000 | 12,510,000 | 15,010,000 | 14,340,000 | 57,570,000 |`, and both
   models answered 12,510,000 under A. The mechanism is B's: a cell is the crossing of two row sets (항목 and 부서)
   and then a lookup on a third line among 80 entries of like-sized numbers, so an off-by-one line gives a
   plausible wrong figure with no validation error.
2. **The relay, not the model** (Sonnet, all nine `writeshape` units, 45 `wrong_refusal`). See the note at the top;
   for example `r4-writeshape-C-3` `budget3-w`: "REFUSE: The actual task content (the workbook text of 2027 부서별
   예산.xlsx, the code documentation, and the specific question for this subtask) was not included in what was
   relayed to me." These are the correct response to an empty prompt and say nothing about the candidates.

Nothing failed on what round 4 was built to provoke, in any real answer: no near-duplicate row hit instead of the
asked one (`wrong_cell`: 0, including 서초 가전 18,240,000 in 2026-05 and 2026-06 and the two 김서연 rows), no ID
losing its zeros when read (`03609`, `068328`, `0431` were right under both views, and view B shows the first one
raw as `3609`) or after a type change (`id_lost_zeros`: 0), no A1 formula (`a1_formula`: 0; every `fill_formula` and
new formula column used structured references such as `=[@[충원 인원]]*[@[1인당 연봉]]` and
`=[@반품액]/SUMIFS(Sales[매출],Sales[월],[@월],Sales[지점],[@지점])`), no edited row label or unmatched `old`
(`row_labels_edited`, `edit_no_match`, `edit_ambiguous`: 0), no sandbox or runtime error under C
(`sandbox_violation`, `runtime_error`, `syntax_error`: 0), no bad op or JSON under A (`bad_op`, `bad_json`,
`bad_range`: 0), and no `type_mismatch`, `bulk_typed` or `fetch_function`.

### Decisions

| decision | round 4 | verdict |
|---|---|---|
| read view | A 30/30, B 29/30 (both models); B −14% input (−3% to −22% by workbook); B's one miss is B-specific | **none yet**: within noise on correctness; B is smaller, A had no miss |
| write shape | Opus 18/18 under A, B and C, no flag of any kind; Sonnet not measured; C −12% in size, B −1% | **none yet**: a three-way tie on one model; re-run Sonnet first |

- **readview.** 29/30 against 30/30 is one task in one conversation, well within noise. It is not a random miss,
  though: it is the lookup that only the index view asks for (find the row from two row lists, then read a third
  line), and it produced a plausible wrong figure that no validator can flag, the worst kind of error for §8.
  Against that, B costs 14% less input on these workbooks, and the saving grows with repeated labels and empty
  runs, which is what large real sheets have. The round does not separate the two: at 150–190 rows the whole window
  fits easily, and neither view was tested where the choice matters (see Limits).
- **writeshape.** Opus landed all 54 answers under every shape, with all the traps avoided, so correctness does
  not separate the shapes on this model. The size differences are real but mostly by task kind (B cheap for
  appended rows and column changes, dear for one cell; C cheap almost everywhere) and C's 12% lead is on one model's
  answers. The design notes in [README.md](README.md#design-notes-for-the-decision-not-measured-here) still apply:
  B needs no addresses but exact pipe lines, A and C need a cell address for a single cell, and C also needs the
  sandbox. In rounds 2 and 3 each model had misses the other did not, so one model's tie is not enough to
  decide.

### Limits

- **Claude models only**, Opus and Sonnet. GPT and Gemini were not run, so §6's "at least Claude, GPT, Gemini" is
  still not met, and Sonnet's `writeshape` half is missing (see above), so `writeshape` rests on Opus alone.
- **Few tasks per cell.** 15 per `readview` cell and 18 per `writeshape` cell, so one task is 6.7 or 5.6 points,
  and the tasks of a unit share one conversation, so their errors are correlated.
- **One run per prompt** per model. The fix round was not exercised (no invalid answer).
- **An in-memory workbook model**, not a real xlsx engine: `wb.py` evaluates a formula subset (ten functions,
  structured references only) and results are compared by value; no workbook was round-tripped through a real xlsx
  file or a spreadsheet application.
- **Workbooks of a few hundred rows, not 100k.** 150–190 data rows per workbook, generated by `content.py` (realistic
  in shape, not taken from a corpus). Both views show every row here; at 100k rows neither can, and the question
  that decides a read view at scale (which rows to show, and how the model asks for more) is not tested.
- `chars` stands in for tokens and also measures strategy (how wide an `old` a model picks, whether it loops or
  lists).

### Next round

- Re-run Sonnet on the nine `writeshape` prompts, with each prompt sent as the whole user message, and add it to
  this file; then decide `writeshape`.
- A `readview` round on large sheets (thousands to 100k rows) where the view is a window or an index over a subset,
  with lookups that cross column lines, to see whether B's miss repeats and whether its saving grows.
- Haiku; GPT and Gemini when keys exist; repeated runs per prompt.
