# hanji fluency test, round 7: inline breaks, xlsx shape/group, dataBar CF (DESIGN.md §6)

Today three kinds of content stay `<keep/>`. The owner approved adding syntax for each, but each must be decided
by a fluency test first. Round 7 measures two candidates per kind. The candidates on the corpus samples that
produce those keeps are in [CANDIDATES.md](CANDIDATES.md). The results are in [RESULTS.md](RESULTS.md).

| decision | A | B | seeds | tasks |
|---|---|---|---|---|
| `breaks` | inline `<pagebreak/>` / `<columnbreak/>` | `<br type="page"/>` / `<br type="column"/>` | Korean quarterly report, ministry work report, product-launch proposal (page and column breaks in the middle of a paragraph; modelled on tdf154481.docx and tdf124637_sectionMargin.docx) | 6 each: a read, a write from a brief, add / move / remove an inline break, a refusal (section break) |
| `shapes` | `<shape name anchor>text</shape>` and `<group>…</group>` | `<shape name anchor text group/>` (xlsx line style) | KPI workbook, regional sales dashboard, budget-execution sheet (callouts and grouped badges; modelled on simple-monthly-budget.xlsx, WithDrawing.xlsx, ExcelWithAttachments.xlsm) | 7 each: a read, a write, add / move / retext / resize a shape, a refusal (picture, connector, rotation) |
| `databar` | `min="num:0" max="max" fill="#638EC6"` on `<conditional-format type="dataBar">` | `min-type` / `max-type` / `color` on the same tag | Monthly budget, regional attainment, KPI scores (data bars; modelled on simple-monthly-budget.xlsx E4) | 6 each: a read, a write, change range / colour / min-max, a refusal (color scale, icon set) |

The candidate guides are the same length and never mention each other. Seeds repeat text on purpose (전년 대비 +12%, 서울 14곳, 목표 미달, 집행률 82%, two similar data bars).

## The kit

- `content.py`: seed documents and workbooks, task wording and intended results, written once in a form that belongs to neither syntax. Edit tasks are small functions that change a copy of the seed.
- `guides.py`: the syntax guide for each candidate. The shared part of a decision is identical for A and B. The candidate parts have the same structure and never mention each other; they are the same length. They are rendered into `guides/*.md`.
- `build.py`: renders every seed, write-gold and edit-gold into both syntaxes. It asserts that the A and B renderings parse to *identical* semantic models (GetPut on the text). It derives gold edits by a line diff, with each `old` widened until it occurs exactly once. Then it writes `data/units/`, `data/gold/`, `seeds/`, `prompts/`, `data/index.json`.
- `score.py`: the scorer. `selftest.py`: the self-test.
- `run.py`: sends the prompts. `summarize.py`: the tables in RESULTS.md, re-scored from `runs/`.

Standard library only (Python 3.11); round 2's `apply_edits` is imported from `../round2/score.py`. Rebuild and self-test:

```
cd fluency/round7
python3 build.py && python3 selftest.py
```

`build.py` is deterministic: rebuilding an unchanged `content.py`/`guides.py` rewrites the generated files byte for byte, so `git status` stays clean. The self-test must end with `SELFTEST PASSED`.

There are **18 units** (`r7-<decision>-<A|B>-<1|2|3>`), **18 prompts** (one file per unit), **114 tasks** (breaks 6×6=36, shapes 6×7=42, databar 6×6=36).

## Scoring (`python3 score.py <unit_id> <answers.json>`)

```
python3 score.py r7-breaks-A-1 runs/r7-breaks-A-1/sonnet55-first.json
```

The answers file is the subject's reply: `{"answers": [{"task_id", "text"} | {"task_id", "edits"}]}`.

- **valid:** the edits apply (each `old` occurs exactly once) and the text parses under the candidate: known tags and attributes, A1 anchors, min/max tokens, colours `#RRGGBB` or a theme name.
- **landed:** a read matches the expected tokens (whitespace collapsed); a write or edit parses to the same semantic model as the gold (inlines for breaks, with whitespace next to a page/column break not compared; drawing shapes compared by name, not sheet order; sheet items for data bars); a refusal of the refusal task.
- **chars:** the answer text, or the sum of `old` + `new`, as in rounds 1–6.
- **flags:** `edit_no_match`, `edit_ambiguous`, `bad_edit`, `unknown_tag`, `invented_attr`, `missing_attr`, `unclosed_tag`, `shape_form`, `bad_anchor`, `bad_range`, `bad_cfvo`, `bad_color`, `no_change`, `not_landed`, `brief_unmet`, `wrong_refusal`, `not_refused`, `missing_answer`, `wrong_answer_kind`.

## Runs

The subject runs will **not** happen on the kit author's VM. The owner's box runs `run.py` with `claude -p` (Sonnet 5.5, Haiku 5.5, Sonnet 4.6), only the Read tool, a fresh conversation per prompt, the same way [round 6](../round6/README.md) describes.

`run.py` is standalone: Python 3.11 stdlib and the `claude` CLI on PATH. It writes `runs/<unit>/<label>-first.json`. The CLI model ids are the values of `MODELS` at the top of `run.py`; the keys are the file-name labels. The owner's box sets those ids if the aliases do not match the local CLI.

### Exact commands (owner's box)

From a checkout with `claude` on PATH, after `python3 build.py && python3 selftest.py`:

```
cd fluency/round7

# first try, all 18 units × 3 models. JOBS (default 6) is the parallel `claude -p` count.
# --models takes the file-name labels; each is passed to claude as --model MODELS[label].
python3 run.py /tmp/hanji-r7
python3 run.py /tmp/hanji-r7 --models=sonnet55,haiku55,sonnet46

# a subset
python3 run.py /tmp/hanji-r7 r7-breaks-A-1 r7-shapes-B-2 --models=haiku55

# score one file
python3 score.py r7-breaks-A-1 runs/r7-breaks-A-1/sonnet55-first.json

# fix round: every unit whose first answer has an invalid task, one fresh conversation
# (prompt + first answer + validator errors) → runs/<unit>/<label>-fix.json
python3 run.py /tmp/hanji-r7 --fix
python3 run.py /tmp/hanji-r7 --fix --models=sonnet55,haiku55,sonnet46

# tables for RESULTS.md
python3 summarize.py
```

Each prompt is copied to an empty directory as `prompt.md`. The message tells the subject to read that file, and only that file, in parts if needed. Answers are stored as the JSON object only (a wrapping code fence stripped). Usage goes to `<label>-first.usage.json` / `<label>-fix.usage.json`.

Default `MODELS` in `run.py` (edit this map on the box if needed):

| label (file name) | CLI id (`claude -p --model`) |
|---|---|
| `sonnet55` | `claude-sonnet-5-5` |
| `haiku55` | `claude-haiku-5-5` |
| `sonnet46` | `claude-sonnet-4-6` |
