# Fluency test results, round 2

## Round 2 — 2026-09-28

**Subjects:** Claude Opus (`claude-opus-5-5`) and Claude Sonnet (`claude-sonnet-5`). Each did all 24 blind
units: 4 decisions × A/B × 3 Korean seeds × 5 tasks = 120 tasks per model, 240 answers in all. No tools, a
fresh conversation per unit, as in [README.md](README.md#running-a-round). One fix round on validator errors was
allowed; Sonnet needed it three times, Opus never.

Answers: `runs/<unit>/opus-first.json`, `runs/<unit>/sonnet-first.json`, and `runs/<unit>/sonnet-fix.json` for
the three units with a fix round (a fix file answers only the task that was invalid, so the scorer reports the
other tasks of that file as `missing_answer`). Re-score any of them with `python3 score.py <unit> <file>`. The
kit's self-test passes (`python3 selftest.py`: 120 gold answers, 57 broken answers caught, 14 alternatives
pass).

### How round 2 differs from round 1

Round 1 ([../RESULTS.md](../RESULTS.md)) hit the ceiling: 144/144 valid and landed. Round 2 keeps the unit shape,
the answer format and the scorer CLI, and changes the following:

| | round 1 | round 2 |
|---|---|---|
| decisions | merge, styleattr, slides | the same three again, plus `tablestyle` (§10.7) |
| units × tasks | 18 × 4 | 24 × 5 |
| seed size | 662–1,305 chars | 3,015–4,983 chars, with repeated text so `old` needs context |
| merges | spelled out cell by cell | described by meaning ("each 구분 once for all its sessions") |
| structural edits | merge two cells, add a column | also insert/delete rows inside merged groups, split cells, 2×2 merges, split/join/move styled tables, split/move slides |
| styling requests | a matching style always existed | 6 requests with no matching style, where the right answer is `REFUSE`, next to tempting ones that do match |
| style names | a few | near-duplicates: `Body Text` / `Body Text Indent` / `Body Text 2`, `눈금 표 4` / `눈금 표 4 - 강조색 1` |
| slides | slots only | also `<shape>` lines, and notes text repeated on up to five slides |

`tablestyle` compares **A**, a `<table style="Name">` line and a `</table>` line around an unchanged pipe table
(DESIGN.md's current proposal), with **B**, a `{style="Name"}` line directly before the header row and nothing
closing the table. In both, tables use the `^^` / `||` markers and paragraphs use `<div style="Name">`.

### First-try validity

Valid answers out of 15 per cell:

| model | merge A | merge B | styleattr A | styleattr B | tablestyle A | tablestyle B | slides A | slides B |
|---|---|---|---|---|---|---|---|---|
| Opus | 15 | 15 | 15 | 15 | 15 | 15 | 15 | 15 |
| Sonnet | 15 | **14** | 15 | 15 | **14** | **14** | 15 | 15 |

### Landed

Answers out of 15 whose result equals the intended one (first try):

| model | merge A | merge B | styleattr A | styleattr B | tablestyle A | tablestyle B | slides A | slides B |
|---|---|---|---|---|---|---|---|---|
| Opus | 15 | 15 | 15 | 15 | **14** | 15 | 15 | 15 |
| Sonnet | 15 | **13** | 15 | 15 | **14** | **14** | 15 | 15 |

Opus: 119/120 landed. Sonnet: 116/120 landed. Two of the five misses were valid files with the wrong meaning,
which no validator error can point out.

### Fix rate after one validator error

| model | cell | invalid first | valid after fix | landed after fix |
|---|---|---|---|---|
| Sonnet | merge B | 1 | 1 | 1 |
| Sonnet | tablestyle A | 1 | 1 | 1 |
| Sonnet | tablestyle B | 1 | 1 | 1 |
| Opus | — | 0 | — | — |

3/3: every invalid answer became valid and landed after one error message.

### Size

Answer size in characters, summed over the 15 tasks of each cell (`chars`: the length of a written file, or the
sum of `old` + `new` of an edit), A / B and B relative to A:

| model | merge | styleattr | tablestyle | slides |
|---|---|---|---|---|
| Opus | 5,147 / 3,827 (−26%) | 2,930 / 2,887 (−1%) | 4,060 / 3,641 (−10%) | 4,710 / 3,909 (−17%) |
| Sonnet | 8,453 / 4,438 (−47%) | 3,260 / 3,382 (+4%) | 7,437 / 8,511 (+14%) | 7,612 / 5,308 (−30%) |

Sonnet's `tablestyle` B total is driven by one task: on `form2-e4` (move a section with its styled table) it
replaced the whole 1,786-character span between the two headings, 3,572 chars, where under A it deleted and
re-inserted the section, 922 chars. That is an edit strategy, not the syntax. Without that task, Sonnet's
`tablestyle` is 6,515 / 4,939 (−24%).

### Refusals

All 24 refusal answers (6 tasks × 2 candidates × 2 models) refused, under `style` and `class`, and under both
table-style forms. There was no `should_refuse` and no `wrong_refusal`: the tempting requests that do have a
matching style ("right-align the last line as a sign-off", "center it and make it large") were done with the
named style, and no `merge` or `slides` task was refused. No answer wrote CSS, direct formatting or an invented
style name anywhere (`css_in_attr`, `unknown_style`, `split_name`: 0).

### Failure patterns

All five first-try misses, by pattern:

1. **`||` written as a separate cell for a span over three or more columns** (Sonnet, 3 misses; all caught and
   fixed in one round). Instead of `|||`, Sonnet put a space between the covered markers, which makes every `||`
   an extra cell:
   - `r2-merge-B-1` `report1-e3` (add a 목표 column inside the 3분기 매출 colspan): the new header was
     `| 권역 | 지점 | 3분기 매출 || || 증감률 |`, 7 cells for a 6-column table. Flag `row_width_mismatch`: "line
     46, column 1: the delimiter row has 6 cells but the header row has 7."
   - `r2-tablestyle-A-3` and `r2-tablestyle-B-3` `form3-w` (a row written once across three columns): both
     candidates got the identical row `| 3주차 | 사례 발표회 (전 상담원 참석) | || | || |`. Flag
     `row_width_mismatch`: "line 19, column 1: this row has 8 cells but the header row has 4." The table-style
     line around it was right in both, so this is the merge marker, not the table-style form.

   The guides define `||` as "two pipes with nothing between them" and show only a two-column span
   (`| 합계 || 215 |`); none shows a three-column one.
2. **A 2×2 merge written as two horizontal merges** (Sonnet, `r2-merge-B-1` `report1-e4`; valid, not landed).
   Sonnet wrote `| 합산 집계 중 ||` in both rows instead of `^^ ||` in the second, so the text appears twice
   in two 1×2 cells. Flag `not_landed`: "grid: cells: [32] [3] got 1, want 2" (rowspan 1, not 2). The guide
   describes exactly this case ("`^^ ||` in the row below"). The file is valid, so the validator cannot catch it.
3. **The default paragraph style instead of `본문`** (Opus, `r2-tablestyle-A-2` `form2-w`; valid, not landed).
   Asked for "the ordinary body paragraph", Opus wrote a plain line, `대상: 2027년 상반기 신입사원 / 장소: 연수원`,
   not `<div style="본문">…</div>`. Flag `brief_unmet`. The guide says a paragraph without `<div>` "has the
   default style", and `본문` is listed as "기본 본문", so the brief is arguably ambiguous. Under B, Opus wrote
   `본문`. This is about paragraph styles, not table styles.

Nothing failed on the things round 2 was built to provoke in HTML: no miscounted `rowspan`/`colspan`
(`counted_span_wrong`: 0), no ambiguous or unmatched `old` despite the repeated text (`edit_ambiguous`,
`edit_no_match`: 0), and no `wrapper_content`, `unclosed_tag`, `attr_orphan` or `attr_form` in `tablestyle`.

### Decisions

| decision | round 1 pick | round 2 | verdict |
|---|---|---|---|
| merged cells | **B** `^^` / `\|\|` | Opus 15/15 under both, B −26%. Sonnet landed 15/15 under A and 13/15 under B, B −47% | **B holds**, with a caveat (below) |
| style attribute | **A** `style="Name"` | 60/60 under both; every refusal right; no CSS in `style=`, no split names in `class=`; size within ±4% | **A holds** (a tie again, kept by default) |
| Presentation | **B** Slidev-style | 60/60 under both; B −17% (Opus), −30% (Sonnet) | **B holds** |
| table style | — (A proposed in §10.7) | no candidate-specific error by either model; B −10% (Opus), −24% for Sonnet without the one outlier task; styling or unstyling a table is one line in B, two in A | **B** `{style="Name"}`, narrowly |

- **merge.** The difference between 15/15 and 13/15 is within noise at 15 tasks, but it is one-sided and it has
  a pattern: counting the `tablestyle` units, which also use the markers, Sonnet made 4 marker mistakes in 45
  tasks with markers and none in merge A's 15 tasks with HTML spans. Three of them were caught and fixed in one round; the
  2×2 one was silent. Round 1's pick holds on size, on fix rate, and because §6's feared failure,
  miscounted spans, did not show under A either. The guide should show a span over three columns (`|||`) and a
  2×2 example, and Sonnet's `merge` B should be re-run after that change.
- **styleattr.** Round 2 added requests with no matching style, which is where `style="…"` was expected to invite
  CSS. It did not happen under either candidate, so the tie stands and `style` stays.
- **slides.** Larger decks with shapes and repeated notes produced no errors under either syntax. B stays smaller.
- **tablestyle.** Correctness is a tie. Both models placed, split, joined, moved, added and removed table styles
  correctly under both forms. The three misses in these units are the merge-marker and paragraph-style patterns
  above, identical under A and B or unrelated to the table form. B wins on size where the form matters: to remove
  a style (`form1-e4`) Sonnet wrote 500 chars under A (it rewrote the whole table to drop `<table …>` and
  `</table>`) and 63 under B; Opus 112 and 43. The margin rests on size, as in round 1's merge and slides
  decisions, not on errors avoided. DESIGN.md §5.2 and §10.7 still show the A proposal; this round does not edit
  DESIGN.md.

### Limits

- **Few tasks.** 15 tasks per cell, so one task is 6.7 points. Every correctness difference in this round (at
  most 2 tasks in a cell) is within noise. The patterns, not the counts, are the finding.
- Only two Claude models. GPT and Gemini were not run (no API keys), so §6's "at least Claude, GPT, Gemini" is
  still not met. Weaker models (Haiku) were not run either.
- Each prompt was run once per model. The fix round was exercised only three times.
- `chars` stands in for tokens. It also measures edit strategy (how wide an `old` a model picks), as Sonnet's
  `form2-e4` shows, not only the syntax.
- The 2×2 miss and the `본문` miss are valid files with the wrong meaning; only the intended-result comparison
  found them. A real user would see them only in the rendered file.

### Next round

- A guide example for a span over three columns and for a 2×2 merge, then re-run `merge` B and `tablestyle` with
  Sonnet.
- Settle the default paragraph style against the listed body style (`본문`, "기본 본문") in the guides.
- Haiku; GPT (with `apply_patch`) and Gemini when keys exist.
- Repeated runs per prompt, to put error bars on the counts.
