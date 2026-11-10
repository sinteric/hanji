# Fluency test results, round 7

## Round 7 — 2026-11-10

**Subjects:** Claude Sonnet 5.5 (`claude-sonnet-5-5`, files `sonnet55-*.json`), Claude Haiku 5.5
(`claude-haiku-5-5`, files `haiku55-*.json`), Claude Sonnet 4.6 (`claude-sonnet-4-6`, files `sonnet46-*.json`).
Blind, each unit in a fresh `claude -p` conversation whose only tool was Read (the prompt read from its own file,
[README.md](README.md#runs)). 18 units, 114 tasks × 3 models = 342 answers. The three CLI ids were probed on the
owner's box (`claude -p --model <id> "say ok"`) and accepted as written in `run.py`'s `MODELS` map; the map was
not changed. Every first-try call returned parsed JSON with `is_error` false. There were **no invalid first
answers** (342/342 valid), so the fix round (`python3 run.py … --fix`) had nothing to send back.

Re-score any answer with `python3 score.py <unit> <file>`; `python3 summarize.py` reproduces every table here.
The self-test must pass (`python3 build.py && python3 selftest.py` ends with `SELFTEST PASSED`).

At run time, **breaks e1 and shapes e1 missed 18/18** for both candidates: gold kept a space after the period
(`보고합니다. <pagebreak/>`) and appended a new shape before `</sheet>`, while every model put the break next to
the period without that space and the shape next to the existing callout. That is gold/scorer strictness, not
syntax. The scorer now treats whitespace next to an inline break as equivalent, and compares drawing items by
name rather than sheet order. Tables below are re-scored from `runs/` after that kit fix (no new subject runs);
the 36 e1 answers land, and seven e2 answers that only dropped a space next to the moved column break land too.

### What round 7 decides

Three keep-reducing syntaxes, each by a fluency test first ([CANDIDATES.md](CANDIDATES.md)):

| decision | A | B |
|---|---|---|
| `breaks` | inline `<pagebreak/>` / `<columnbreak/>` inside the paragraph | `<br type="page"/>` / `<br type="column"/>` (reusing `<br/>`) |
| `shapes` | `<shape name="…" anchor="B2:D5">text</shape>` and `<group name="…">…</group>` | `<shape name="…" anchor="B2:D5" text="…"/>` with `group="Name"` (xlsx line style) |
| `databar` | `<conditional-format type="dataBar" min="min" max="max" fill="#638EC6"/>` | `<conditional-format type="dataBar" min-type="min" max-type="num" max="100" color="#638EC6"/>` |

### Landed

| model | decision | cand | landed, first try | after one fix round | invalid, first try | wrong refusals | answer chars | tokens in | tokens out |
|---|---|---|---|---|---|---|---|---|---|
| sonnet55 | breaks | A | 15 / 18 | 15 / 18 | 0 | 0 | 1,519 | 499,806 | 3,640 |
| sonnet55 | breaks | B | 14 / 18 | 14 / 18 | 0 | 0 | 1,684 | 499,743 | 3,631 |
| sonnet55 | shapes | A | 21 / 21 | 21 / 21 | 0 | 0 | 2,753 | 498,465 | 3,322 |
| sonnet55 | shapes | B | 21 / 21 | 21 / 21 | 0 | 0 | 2,607 | 498,359 | 3,230 |
| sonnet55 | databar | A | 18 / 18 | 18 / 18 | 0 | 0 | 2,078 | 497,763 | 2,541 |
| sonnet55 | databar | B | 16 / 18 | 16 / 18 | 0 | 0 | 2,134 | 497,939 | 2,682 |
| haiku55 | breaks | A | 15 / 18 | 15 / 18 | 0 | 0 | 1,577 | 504,529 | 10,469 |
| haiku55 | breaks | B | 16 / 18 | 16 / 18 | 0 | 0 | 1,632 | 504,396 | 9,526 |
| haiku55 | shapes | A | 21 / 21 | 21 / 21 | 0 | 0 | 2,870 | 503,118 | 8,297 |
| haiku55 | shapes | B | 21 / 21 | 21 / 21 | 0 | 0 | 3,153 | 503,120 | 9,132 |
| haiku55 | databar | A | 18 / 18 | 18 / 18 | 0 | 0 | 2,198 | 502,551 | 6,180 |
| haiku55 | databar | B | 16 / 18 | 16 / 18 | 0 | 0 | 2,218 | 502,718 | 7,924 |
| sonnet46 | breaks | A | 15 / 18 | 15 / 18 | 0 | 0 | 1,833 | 408,406 | 13,105 |
| sonnet46 | breaks | B | 15 / 18 | 15 / 18 | 0 | 0 | 1,905 | 408,445 | 13,806 |
| sonnet46 | shapes | A | 21 / 21 | 21 / 21 | 0 | 0 | 3,257 | 407,102 | 9,354 |
| sonnet46 | shapes | B | 21 / 21 | 21 / 21 | 0 | 0 | 3,387 | 406,994 | 11,339 |
| sonnet46 | databar | A | 18 / 18 | 18 / 18 | 0 | 0 | 3,179 | 406,448 | 7,411 |
| sonnet46 | databar | B | 16 / 18 | 16 / 18 | 0 | 0 | 3,628 | 406,633 | 7,866 |

Per-task landed (first try, three seeds; L landed, x missed, r wrong refusal):

### breaks

| task | sonnet55 A | sonnet55 B | haiku55 A | haiku55 B | sonnet46 A | sonnet46 B |
|---|---|---|---|---|---|---|
| q1 | xLx | xxx | xLx | xLx | xLx | xLx |
| w | LLL | LLL | LLL | LLL | LLL | LLL |
| e1 | LLL | LLL | LLL | LLL | LLL | LLL |
| e2 | xLL | xLL | LxL | LLL | xLL | xLL |
| e3 | LLL | LLL | LLL | LLL | LLL | LLL |
| r | LLL | LLL | LLL | LLL | LLL | LLL |

### shapes

| task | sonnet55 A | sonnet55 B | haiku55 A | haiku55 B | sonnet46 A | sonnet46 B |
|---|---|---|---|---|---|---|
| q1 | LLL | LLL | LLL | LLL | LLL | LLL |
| w | LLL | LLL | LLL | LLL | LLL | LLL |
| e1 | LLL | LLL | LLL | LLL | LLL | LLL |
| e2 | LLL | LLL | LLL | LLL | LLL | LLL |
| e3 | LLL | LLL | LLL | LLL | LLL | LLL |
| e4 | LLL | LLL | LLL | LLL | LLL | LLL |
| r | LLL | LLL | LLL | LLL | LLL | LLL |

### databar

| task | sonnet55 A | sonnet55 B | haiku55 A | haiku55 B | sonnet46 A | sonnet46 B |
|---|---|---|---|---|---|---|
| q1 | LLL | xLx | LLL | xLx | LLL | xLx |
| w | LLL | LLL | LLL | LLL | LLL | LLL |
| e1 | LLL | LLL | LLL | LLL | LLL | LLL |
| e2 | LLL | LLL | LLL | LLL | LLL | LLL |
| e3 | LLL | LLL | LLL | LLL | LLL | LLL |
| r | LLL | LLL | LLL | LLL | LLL | LLL |

### Fix rate after one validator error

No first-try answer was invalid, on any model or candidate (0/342). The fix round was run and sent nothing back.
Every miss below is a valid answer that did not land (`not_landed` or, for a write, none occurred).

### Size

Answer chars, summed over each decision's tasks (three seeds, first try):

| model | breaks A | breaks B | shapes A | shapes B | databar A | databar B |
|---|---|---|---|---|---|---|
| sonnet55 | 1,519 | 1,684 (+11%) | 2,753 | 2,607 (−5%) | 2,078 | 2,134 (+3%) |
| haiku55 | 1,577 | 1,632 (+3%) | 2,870 | 3,153 (+10%) | 2,198 | 2,218 (+1%) |
| sonnet46 | 1,833 | 1,905 (+4%) | 3,257 | 3,387 (+4%) | 3,179 | 3,628 (+14%) |
| all | 4,929 | 5,221 (+6%) | 8,880 | 9,147 (+3%) | 7,455 | 7,980 (+7%) |

Input, the seed file in each syntax:

| seed | A | B |
|---|---|---|
| breaks 1 / 2 / 3 | 920 / 1,041 / 844 | 930 / 1,051 / 854 |
| breaks all | 2,805 | 2,835 (+1%) |
| shapes 1 / 2 / 3 | 945 / 720 / 660 | 944 / 737 / 660 |
| shapes all | 2,325 | 2,341 (+1%) |
| databar 1 / 2 / 3 | 726 / 637 / 609 | 760 / 683 / 655 |
| databar all | 1,972 | 2,098 (+6%) |

### Failure patterns

1. **A trailing space before an inserted break** (`breaks` e1, all 18 answers at run time, both candidates). The
   task is to insert an inline page break after the first sentence (after “보고합니다.”). Every answer put the
   break in that sentence and applied cleanly, but wrote `보고합니다.<pagebreak/>` (or `<br type="page"/>`) and ate
   the space after the period. Gold kept the space (`보고합니다. <pagebreak/>`). The seed and the guide have no
   space around an inline break, so this was a kit bug, not syntax, and did not separate A from B. The scorer now
   ignores whitespace next to a page/column break; re-scored from `runs/`, e1 is 18/18.

2. **Five-word quotes that ran long** (`breaks` q1). The read asks for the five words immediately after the page
   break and after the column break. On seeds 1 and 3 every model, under both candidates, added words past the fifth
   (`첨부 1의 지점별 표는 10월` for `첨부 1의 지점별 표`; `인쇄물 마감은 출시일 11월` for `인쇄물 마감은 출시일`).
   Seed 2 usually landed. The one extra miss is sonnet55 under B on seed 2, a trailing period. These are tight token
   matches, not a confusion of `<pagebreak/>` with `<br type="page"/>`.

3. **Punctuation around a moved column break** (`breaks` e2). Gold moves the break to sit after `대구 3곳.` (or the
   matching phrase on the other seeds), period included. At run time, misses kept a comma, dropped a space after
   the break, or joined two sentences without a space. The space-next-to-the-break cases land after the same kit
   fix as e1. Five of 18 still miss: a comma where gold has a period (A and B seed 1, sonnet55 and sonnet46), or
   two sentences joined without a space (haiku55 A seed 2: `늘립니다.업종은`). No answer moved the wrong break or
   invented a tag. After the kit fix, 13/18 landed (A 6, B 7).

4. **A new shape in the right place, wrong list order** (`shapes` e1, all 18 answers at run time, both candidates).
   Every model added a shape with the requested name, anchor and text, next to the existing callout. Gold's edit
   is `old: "\n</sheet>\n"`, so the new shape lands after the group and the picture keep. The task does not ask
   for a list position; this was a kit bug, not syntax. The scorer now compares drawing items by name, not sheet
   order (group membership is still checked). Re-scored from `runs/`, e1 is 18/18, and shapes is 63/63 under both
   A and B.

5. **B's data-bar attribute names were not produced on a read** (`databar` B q1, 6 of 9). Asked to report min, max
   and colour, every model under A answered in A's form and landed 9/9 (`min=num:0 max=num:월간수입합계 fill=#638EC6`).
   Under B, seeds 1 and 3 (the tasks that require the type tokens) were answered in A's vocabulary:
   `min=0 max=월간수입합계 fill=#638EC6` or `min=percentile 10 max=percentile 90 fill=#638EC6`, never
   `min-type=` / `max-type=` / `color=`. Seed 2 only asked for the two fills (`D=#638EC6 | E=#63BE7B`) and landed
   3/3 under B. Writes and edits under B all landed: once the guide is in front of them, models can write
   `min-type` and `color`, but they do not reach for those names when reading.

Writes and refusals landed 18/18 on every decision and candidate. No `wrong_refusal`, no invented tag, no
`edit_no_match`.

### Decisions

| decision | official landed (3 models) | first-try invalid | size (all models) | verdict |
|---|---|---|---|---|
| `breaks` | A 45/54, B 45/54 | 0 / 0 | A answers −6%, input −1% | **A**: `<pagebreak/>` / `<columnbreak/>` |
| `shapes` | A 63/63, B 63/63 | 0 / 0 | A answers −3%, input ~equal | **A**: `<shape>…</shape>` and `<group>` |
| `databar` | A 54/54, B 48/54 | 0 / 0 | A answers −7%, input −6% | **A**: `min` / `max` / `fill` |

At run time, before the kit fix, this was A 32/54 vs B 33/54 on breaks and 54/63 each on shapes (e1 0/18 on both
decisions). The 36 e1 misses were the space-after-period and `</sheet>` insertion comparisons above, not syntax.
After re-scoring, breaks is a landed tie and shapes is at the ceiling; the three verdicts do not change.

- **breaks: A.** Correctness is a tie (after the kit fix, 45/54 each: B is still ahead by one e2 punctuation
  landing, A by one q1). Remaining misses are an extra word on a five-word quote, a comma where gold has a period,
  or two sentences joined without a space — never the tag name, and never a section break attempted as an inline
  one (the refusal was 18/18). First-try validity does not decide. A is 6% smaller in answers on every model and
  slightly smaller in input, and it does not overload `<br/>`. Adopt A's dedicated tags.

- **shapes: A.** A and B tie at 21/21 on every model after the kit fix (at run time 18/21, the only miss item
  order on "add a shape", identical under both). Move, retext, resize, the group, the write and the
  picture/connector/rotation refusal all landed on the day. Correctness does not decide. Size is mixed per model
  (sonnet55 slightly smaller on B; haiku55 and sonnet46 smaller on A) and 3% smaller for A overall; input is the
  same. Adopt A's element-with-body and `<group>` wrapper, which is also how a grouped shape is written in the seed.

- **databar: A, clearly.** A is at the ceiling on all three models (54/54). B drops the same two reads on all three
  (48/54), because models describe a data bar with `min=` / `fill=` even when the file in front of them uses
  `min-type=` / `color=`. Edits and writes do not save B: they landed, but the read is the task that shows which
  names are fluent. A is also shorter in both answers and input.

### Limits

- 18 tasks per breaks/databar cell and 21 per shapes cell, one run per prompt, three Claude models (Sonnet 5.5,
  Haiku 5.5, Sonnet 4.6). No weaker model, and no second try except the empty fix round.
- At run time the scorer treated a missing space before a break, and a correctly added shape in a different list
  position, as misses (e1 0/18 on both decisions). Those were kit bugs, now corrected; tables above are re-scored
  from the same `runs/`. Verdicts unchanged.
- `chars` stands in for tokens, as before. Seeds are modelled on corpus files, not the files themselves.
