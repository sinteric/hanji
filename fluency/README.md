# hanji fluency-test kit (DESIGN.md §6)

The fluency test is the method in [DESIGN.md §6](../DESIGN.md#6-fluency--designed-for-then-measured): give
a subject model real Korean office files and tasks in a candidate syntax, and measure first-try validity,
whether an edit lands on the intended span, the fix rate after one validator error, and size. It exists
because rule 2 makes fluency a design input: where §5 had two candidate syntaxes, the test decides.

This kit decides the three format choices that were open in §5/§10.1. Each choice is tested by giving a
subject model the same documents and tasks in candidate syntax A or candidate syntax B. Round 1 results
and the decisions are in [RESULTS.md](RESULTS.md). Round 2 (larger seeds, harder tasks, and the new table-style
decision of §10.7) has its own kit in [round2/](round2/README.md); its results are in
[round2/RESULTS.md](round2/RESULTS.md). Round 3 (multi-paragraph table cells and empty paragraphs, §10.8) has its
own kit in [round3/](round3/README.md); its results are in [round3/RESULTS.md](round3/RESULTS.md). Round 4
(spreadsheet cell data: the read view and the write shape, §10.6) has its own kit in [round4/](round4/README.md);
its results are in [round4/RESULTS.md](round4/RESULTS.md).

| decision | A | B |
|---|---|---|
| `merge` | HTML-like `<table>` with `rowspan`/`colspan` (simple tables stay pipe tables) | pipe table with local markers: `^^` = merged into the cell above, `\|\|` = cell to the left extends here |
| `styleattr` | `<div style="Name">`, `<table style="Name">` | `<div class="Name">`, `<table class="Name">` (the whole value is one name) |
| `slides` | `<slide layout="…">` with `<title>`, `<body>`, `<left>`, `<right>`, `<notes>` | Slidev-style (see below) |

The exact wording each subject saw is in `guides/<decision>-<candidate>.md`.

A **unit** is one (decision, candidate, replicate). There are 3 × 2 × 3 = 18 units, for example `merge-A-1`.

## Layout

- `content.py`: seed documents, task wording and intended results, written once in a form that belongs to
  neither syntax. Edit tasks are small functions that change a copy of the seed.
- `guides.py`: the syntax guide for each candidate. The Markdown basics are identical for A and B. The
  candidate parts are about the same length (merge 1226/1213 chars, styleattr 1050/1050, slides
  1258/1316) and never mention each other. They are rendered into `guides/*.md`.
- `build.py`: renders every seed, target and write-gold into both syntaxes. It asserts that the A and B
  renderings parse to *identical* semantic models. It derives gold edits by a line diff, with each `old`
  widened until it occurs exactly once. Then it writes:
  - `data/units/<unit>.json`: the seed, allowed names, and tasks (write tasks also carry `expect`, the brief's
    semantic checks)
  - `data/gold/<unit>.json`: gold answers in the answers format
  - `seeds/<decision>-<rep>-<cand>.txt`: the seed files
  - `prompts/<unit>.md`: the self-contained subject prompts
  - `data/index.json`
- `score.py`: the scorer. `selftest.py`: the self-test.
- `runs/<unit>/`: the subjects' answers (see "Runs" below).

All paths are relative to the kit directory, so the kit runs from any checkout. It uses only the standard
library (Python 3.11 or later). Rebuild and self-test:

```
cd fluency
python3 build.py && python3 selftest.py
```

`build.py` is deterministic: rebuilding an unchanged `content.py`/`guides.py` rewrites the generated files
byte for byte, so `git status` stays clean. The self-test must end with `SELFTEST PASSED`.

## Seeds and tasks

Each seed is realistic Korean office content:
- merge: a quarterly regional sales report, a development weekly update, a product-launch marketing proposal.
- styleattr: executive meeting minutes, a logistics-center proposal, and an official notice. The notice uses
  Korean style names with spaces.
- slides: an H2 sales-strategy deck, a Q3 KPI review (Korean PowerPoint layout names), and a customer
  proposal deck.

The seeds repeat text on purpose. Examples: `287 | 301` next to `301 | 287`, two `488` totals, two
near-identical Note paragraphs, and `전년 대비 +12%` on three slides.

Every unit has 4 tasks, worded the same for A and B. Task ids are neutral: `report1-w`, `document2-e1`,
`deck3-e3`.

- `*-w` (write): write a new file from a brief. The answer is `text`.
- `*-e1..e3` (edit): the answer is `edits` (`{old,new}` pairs applied in order to the seed, where each `old`
  must occur exactly once).
  - merge: merge two cells (vertically in a plain table, horizontally in a merged table, or vertically in a
    merged table); add a column (last column, middle column next to a colspan, or a new column with its own
    rowspan that widens the total row's colspan from 2 to 3); change one figure that also appears elsewhere.
  - styleattr: restyle a block (a paragraph, a table, or a plain pipe table that must become `<table …>`); a
    tempting request ("red, bold and large", "bold blue 16pt") that only a described named style
    satisfies; add a block whose style name contains a space.
  - slides: add a slide from a layout; move content between slots (body→notes, right→left, notes→body);
    change one bullet whose text repeats on other slides.

Merge seeds carry no table styles, so the merge test does not get mixed up with the styleattr choice.

## Candidate B slides, defined exactly

The file's front matter comes first. The first slide begins right after its closing `---`, and later slides
are separated by a line containing only `---`. The first non-blank line of each slide is `layout: Name`.
Quotes around Name are allowed, and no other `key: value` lines are allowed. Slot markers are lines of the
form `::title::`, `::body::`, `::left::`, `::right::` or `::notes::`. A slot's text is the lines after its
marker, up to the next marker or `---`.

Real Slidev writes a closing `---` after the per-slide front matter. Here that closing `---` would start an
empty or layout-less slide, and the scorer reports it as `missing_layout` or `empty_slide`.

## Scoring (`python3 score.py <unit_id> <answers.json>`)

```
python3 score.py merge-B-1 runs/merge-B-1/opus-first.json
```

The answers file is the subject's reply: `{"answers": [{"task_id", "text"} | {"task_id", "edits"}]}`, as
the prompt's "Answer format" asks.

The scorer parses the candidate syntax into a common semantic model:
- Documents: the front matter plus blocks. Blocks are headings, paragraphs (style name + text), list items,
  and tables (style + grid). A grid is `[r, c, text, rowspan, colspan, is_header]` for each origin cell.
  Plain pipe tables and `<table>`s both map to grids.
- Presentations: slides as `{layout, slots: {slot: [lines]}}`.

Text is compared with whitespace collapsed and `**` removed. Slot bullets `*`/`+` count as `-`.

For each task the scorer prints `{"results":[{task_id, valid, landed, error, chars, flags, detail}]}`:
- `valid`: every edit applied, the file parses, and every tag, attribute, style, layout and slot name is
  allowed.
- `error`: validator messages written for the model. Each gives the line and column, the expected form, and
  the allowed names. Line numbers refer to the file after the edits.
- `landed`: for edit tasks, the semantic model equals that of the gold result. For write tasks, the brief's
  blocks appear in order (documents) or the slides equal the brief exactly (presentations).
- `chars`: the length of `text`, or the sum of the lengths of each `old` and `new`.
- `detail`: the first semantic difference when an answer is valid but did not land.
- `flags`: `edit_no_match`, `edit_ambiguous`, `bad_edit`, `counted_span_wrong`, `row_width_mismatch`,
  `marker_misplaced`, `merge_not_rectangular`, `pipe_row`, `pipe_delimiter`, `text_outside_cell`,
  `css_in_attr`, `unknown_style`, `split_name`, `invented_attr`, `missing_attr`, `unknown_tag`,
  `unclosed_tag`, `div_form`, `unknown_layout`, `unknown_slot`, `duplicate_slot`, `missing_layout`,
  `empty_slide`, `text_outside_slide`, `text_outside_slot`, `front_matter`, `no_front_matter`,
  `no_change`, `not_landed`, `brief_unmet`, `missing_answer`, `wrong_answer_kind`.

Scoring rules:
- If an edit pair fails to apply, the task is invalid and scoring stops at that pair.
- If a write answer has no front matter, the scorer adds the given front matter and flags `no_front_matter`
  instead of failing the answer.
- The fix rate after one validator error (a §6 metric) is not in this kit. To measure it, send `error` back
  to the model and score the second answer the same way.

## Self-test (`python3 selftest.py`)

The self-test passes:
- All 72 gold answers (18 units × 4 tasks) are valid and landed, with no flags.
- It runs 35 deliberately broken answers (12 merge, 12 styleattr, 11 slides, covering both candidates). Each
  one is caught with its expected flag. Examples: miscounted rowspans and colspans, a row missing a `^^`,
  `^^` in the header, an HTML table in B, `style="color:red…"`, `<span style=…>`, `class="Alert"` and
  `class="Plain-Table-1"`, the wrong attribute, an unknown layout or slot, `transition: fade`, the Slidev
  closing `---`, ambiguous `old` strings, and valid edits that landed on the wrong span.
- 10 correct answers written differently from the gold (minimal edits, a different choice of span) pass.
- The command-line round trip also passes.

## Runs

`runs/<unit_id>/<model>-first.json` holds one subject's first answer to `prompts/<unit_id>.md`, saved
exactly as returned (the JSON object only). Round 1 has two files per unit:

- `opus-first.json`: Claude Opus (`claude-opus-5-5`)
- `sonnet-first.json`: Claude Sonnet (`claude-sonnet-5`)

A second answer after a validator error would be `<model>-fix.json`; round 1 needed none.

## Running a new round with another model

1. Pick a short label for the model, e.g. `haiku` or `gpt`.
2. For each of the 18 units in `data/index.json`, send `prompts/<unit_id>.md` to the model as the whole
   user message, in a fresh conversation, with no tools and no other context. The unit is blind: the model
   sees one candidate only and never the other one or another unit.
3. Save the JSON object it returns as `runs/<unit_id>/<label>-first.json`. If the reply wraps the JSON in
   a code fence or adds prose, strip that and note it.
4. Score each file: `python3 score.py <unit_id> runs/<unit_id>/<label>-first.json`.
5. For any task with `valid: false`, send the task's `error` back in the same conversation once and save
   the reply as `runs/<unit_id>/<label>-fix.json`; score it the same way. That gives the fix rate.
6. Tally per decision and candidate: valid, landed, flags, and the sum of `chars`. Add the round to
   `RESULTS.md`.

To change the tasks (for a harder round), edit `content.py` and `guides.py`, rebuild, and self-test. A
rebuild changes the prompts, so answers from an earlier round belong to the earlier kit revision.
