# Fluency test results, round 3

## Round 3 — 2026-09-28

**Subjects:** Claude Opus (`claude-opus-5-5`) and Claude Sonnet (`claude-sonnet-5`). Each did all 12 blind
units: 2 decisions × A/B × 3 Korean seeds × 5 tasks = 60 tasks per model, 120 answers in all. No tools, a fresh
conversation per unit, as in [README.md](README.md#running-a-round). One fix round on validator errors was
allowed; Opus needed it in two units (5 tasks), Sonnet in one (1 task).

Answers: `runs/<unit>/opus-first.json`, `runs/<unit>/sonnet-first.json`, and `runs/<unit>/<model>-fix.json` for
the three units with a fix round (`r3-cellpara-A-1` Sonnet, `r3-cellpara-B-1` and `r3-cellpara-B-2` Opus; a fix
file answers only the tasks that were invalid, so the scorer reports the other tasks of that file as
`missing_answer`). Re-score any of them with `python3 score.py <unit> <file>`. The kit's self-test passes
(`python3 selftest.py`: 60 gold answers, 36 broken answers caught, 12 alternatives pass).

### What round 3 decides

§10 item 8: a syntax for **multi-paragraph table cells** (`cellpara`) and for **empty paragraphs** (`emptypara`).
Both sit on the syntax decided in rounds 1 and 2 (pipe tables with `^^` / `||`, the `{style="Name"}` table-style
line, `<div style="Name">` for a styled paragraph). The exact definitions are in [README.md](README.md#decisions);
the wording each subject saw is in `guides/<decision>-<candidate>.md`.

| decision | A | B |
|---|---|---|
| `cellpara` | every table stays a pipe table; inside a cell `<p/>` starts the next paragraph, `<p style="Name"/>` starts the next paragraph with a style | a table that needs it becomes a list table: a `{list-table}` line, a `-` line per row, a `  - ` line per cell, further paragraphs of a cell on lines indented four spaces, `<div style="Name">` for a styled one |
| `emptypara` | a line holding only `<p/>` or `<p style="Name"/>` | a line holding only `<div></div>` or `<div style="Name"></div>` |

What makes it hard (details in [README.md](README.md#what-makes-it-hard)): seeds of 3,023–3,408 chars with 11–15
multi-paragraph cells or 25–33 empty paragraphs; identical runs of empty paragraphs where the edit must hit one;
structural cell edits (split, join, move and restyle cell paragraphs, merge two two-paragraph cells, split a
four-paragraph cell into rows); write tasks scored exactly, so one extra or missing empty paragraph fails; one
refusal per unit.

### First-try validity

Valid answers out of 15 per cell:

| model | cellpara A | cellpara B | emptypara A | emptypara B |
|---|---|---|---|---|
| Opus | 15 | **10** | 15 | 15 |
| Sonnet | **14** | 15 | 15 | 15 |

### Landed

Answers out of 15 whose result equals the intended one (first try):

| model | cellpara A | cellpara B | emptypara A | emptypara B |
|---|---|---|---|---|
| Opus | 15 | **10** | 15 | 15 |
| Sonnet | **14** | 15 | 15 | 15 |

Opus: 55/60 landed. Sonnet: 59/60. Every valid answer landed: there was no valid file with the wrong meaning
(`not_landed`, `brief_unmet`, `cell_para_lost`, `br_for_p`, `wrong_empty_para`: 0).

### Fix rate after one validator error

| model | cell | invalid first | valid after fix | landed after fix |
|---|---|---|---|---|
| Opus | cellpara B | 5 | 5 | 5 |
| Sonnet | cellpara A | 1 | 1 | 1 |
| both | emptypara A, B | 0 | — | — |

6/6: every invalid answer became valid and landed after one error message. After the fix round all four cells
are 30/30 landed (both models together).

### Size

Answer size in characters, summed over the 15 tasks of each cell (`chars`: the length of a written file, or the
sum of `old` + `new` of an edit; first answers, invalid ones included), A / B and B relative to A:

| model | cellpara | emptypara |
|---|---|---|
| Opus | 2,875 / 3,574 (+24%) | 3,148 / 3,770 (+20%) |
| Sonnet | 3,402 / 3,920 (+15%) | 3,930 / 4,627 (+18%) |

The refusal answers are reason text and do not depend on the syntax; without them (12 tasks per cell) B is
+27% (Opus) / +18% (Sonnet) in `cellpara` and +21% / +21% in `emptypara`. Part of the gap is built in: B's seeds
are 5–8% longer, and the three write tasks alone are +11% (`cellpara`) and +15% (`emptypara`) under B for both
models.

Where `cellpara` B costs most is the whole-table switch: `notice1-e1` (give one cell of a plain pipe table a
second paragraph) was 89 chars under A and 384 under B for Opus (55 / 384 Sonnet), because B rewrites the table as
a list table. On `form3-e2` (join the only multi-paragraph cell of a list table) Opus rewrote the table back to a
pipe table (414 chars; landed, since the scorer compares meaning), Sonnet kept the list table and changed one line
(121 chars). Where B was expected to win, a one-line `old` per cell paragraph, it did not: under A both models
quoted only the unique part of the row line (`minutes2-e1`: 69 / 57 chars under A against 76 / 72 under B), not the
whole row that the gold diff uses.

In `emptypara` the difference is the form itself (`<div style="좁은 간격"></div>` is 7 chars longer than
`<p style="좁은 간격"/>`, `<div></div>` 7 longer than `<p/>`), multiplied over runs of up to seven lines.

### Refusals

All 24 refusal answers (6 tasks × 2 candidates × 2 models) refused, with a reason naming the missing style: shade
one row, center text in cells, 6pt between cell paragraphs, 24pt instead of empty paragraphs, 5 mm high empty
paragraphs, and 6pt high empty paragraphs next to a "low empty line" style. No `should_refuse`, no
`wrong_refusal`, and no CSS, invented attribute or invented style anywhere (`css_in_attr`, `invented_attr`,
`unknown_style`: 0).

### Failure patterns

All six first-try misses, by pattern:

1. **List-table lines quoted two spaces too deep** (Opus, `cellpara` B, 5 misses in two units; all caught and fixed
   in one round). Every `old` that began inside a list table in `r3-cellpara-B-1` and `r3-cellpara-B-2` was
   copied with its cell lines indented two more spaces than the file, as if the table were nested one level: the
   file has `  - ` and four spaces, Opus quoted `    - ` and six (and once the row line `-` as `  -`):
   - `r3-cellpara-B-1` `notice1-e2` (split a cell paragraph): `old` was
     `    - 시제품 제작·마케팅 비용, 지식재산권 출원 비용` where the file has `  - 시제품 제작·마케팅 비용, …`.
     Flag `edit_no_match`: "edit 1: the "old" text does not occur in the file. It must be copied exactly from
     the file as it is after the earlier edits (same spaces, line breaks and characters)."
   - `r3-cellpara-B-1` `notice1-e4` (merge two two-paragraph cells next to `^^`): a 9-line `old` starting
     `    - 창업 기초 과정 (20시간)\n      분야별 심화 과정 (16시간)\n    - 전액 지원`, every cell line off by two.
     Flag `edit_no_match`.
   - `r3-cellpara-B-2` `minutes2-e4` (add a row): `old` was `  -\n    - 채용 계획`, the row line itself indented.
     Flag `edit_no_match`. The other two, `minutes2-e1` (`    - 간편 로그인만 3.3에 포함`) and `minutes2-e2`, are the
     same.

   The `new` text Opus wrote was indented correctly: its whole new list table in `notice1-e1` landed, and so did
   every fixed answer (`  - 시제품 제작·마케팅 비용, …`). In `r3-cellpara-B-3` its three list-table `old`s were
   quoted correctly. So Opus knows the form but, reading it back from a 3,000-char file, it reproduces
   significant leading whitespace wrongly; the pipe-table form of A has no leading whitespace to get wrong. The
   five misses sit in two conversations, so they are not five independent events.
2. **A trailing cell after `||`** (Sonnet, `r3-cellpara-A-1` `notice1-w`; caught and fixed in one round). Asked for
   "80% 이상 출석", one paragraph written once across the 내용 and 비고 columns, Sonnet wrote
   `| 수료 기준 | 80% 이상 출석 || |`, 4 cells for a 3-column table. Flag `row_width_mismatch`: "line 19, column
   1: this row has 4 cells but the header row has 3." The fix was `| 수료 기준 | 80% 이상 출석 ||`. This is round
   2's merge-marker pattern (`|| ||` for a wider span) in a new shape, not the `<p/>` form: the two `<p/>` and
   `<p style/>` cells in the rows above it were right.

Nothing failed on what round 3 was built to provoke: no ambiguous `old` despite identical runs of five, six and
seven empty paragraphs and a repeated `※ 해당 시 작성` note (`edit_ambiguous`: 0), no wrong count or position of
empty paragraphs (`wrong_empty_para`: 0), no `<br/>` for a paragraph break or a blank line for an empty paragraph
(`br_for_p`: 0), no lost cell paragraph when merging, joining or moving (`cell_para_lost`: 0), no list table read as
a list (`list_parsed_as_list`, `list_table_form`: 0), and no malformed tag (`cell_para_form`, `empty_cell_para`,
`cell_div`, `p_outside_cell`, `empty_para_form`: 0).

### Decisions

| decision | round 3 | verdict |
|---|---|---|
| multi-paragraph cells | A 29/30 first try, B 25/30; 30/30 landed after the fix round under both. B's 5 misses are B-specific (list-table indentation), A's 1 is the shared merge marker. B +15–24% in size, and under A a cell never forces a table rewrite | **A** `<p/>` / `<p style="Name"/>` in pipe cells |
| empty paragraphs | 60/60 under both, no flag of any kind. B +18–21% in size | **A** `<p/>` / `<p style="Name"/>` lines, on size (a tie on correctness) |

- **cellpara.** 10/15 against 15/15 for Opus is at the edge of noise: it is one-sided and has one mechanism, but
  the five misses come from two of the three B conversations and one run per prompt, and Sonnet went the other
  way (B 15/15, A 14/15 on an unrelated marker). All six misses were fixed in one round, so neither candidate
  produced a silent error. The case for A rests on three things together: its only failure was not about
  paragraphs; B adds significant indentation, which is exactly what exact-match `old` strings copy badly; and B is
  larger, most of all when one new paragraph turns a pipe table into a list table (`notice1-e1`, 4× the size).
- **emptypara.** Identical on correctness, including the identity tasks the round was built around, so the
  difference is size alone, B +18–21%, about half of it from B's longer seeds. That is a narrow margin, of the
  kind that decided round 1's merge and slides.
- **The combination.** A and A gives `<p/>` two roles: a paragraph break inside a cell and a whole-line empty
  paragraph. Round 3 kept the two apart, so it did not test them together, and it does not cover an empty
  paragraph inside a cell (a shape the corpus has; `a<p/><p/>b` is an `empty_cell_para` error under the current
  rule). DESIGN.md §10.8 needs a rule for that case when it records the decision; this round does not edit
  DESIGN.md.

### Limits

- **Few tasks.** 15 tasks per cell, so one task is 6.7 points, and the tasks of a unit share one conversation, so
  their errors are correlated (Opus's five misses in two units). Only the Opus `cellpara` gap is larger than one
  or two tasks.
- Only two Claude models. GPT and Gemini were not run, so §6's "at least Claude, GPT, Gemini" is still not met;
  Haiku was not run either.
- Each prompt was run once per model. The fix round was exercised six times.
- `chars` stands in for tokens and also measures edit strategy (how wide an `old` a model picks); B's seeds and
  written files are longer by construction (see Size).
- Empty paragraphs inside cells, and the two decisions combined, were not tested (see above).

### Next round

- Empty paragraphs inside cells under the chosen combination, with the corpus's shapes (`docx4j-tables.docx` t4,
  `form_footnotes.docx` t2 r3).
- Re-run `cellpara` B with Opus several times to see whether the indentation misses repeat, if B is reconsidered.
- A guide example of a span to the last column (`| a | b ||`), after Sonnet's `|| |`.
- Haiku; GPT and Gemini when keys exist; repeated runs per prompt.
