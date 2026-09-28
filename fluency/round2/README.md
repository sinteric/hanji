# hanji fluency test, round 2 (DESIGN.md §6, §10.7)

Round 1 ([../README.md](../README.md), [../RESULTS.md](../RESULTS.md)) hit the ceiling. Opus and Sonnet got
72/72 valid and landed under every candidate for three reasons:
- the briefs spelled out every merge;
- the style list always offered a matching style;
- the seeds were small (662–1,305 characters).

Round 2 keeps round 1's unit shape, answer format and scorer CLI, and makes the tasks hard enough that
models should fail somewhere, without favouring either candidate. Round 1 files are not changed. This kit copies
and adapts round 1's code.

## Decisions

| decision | A | B |
|---|---|---|
| `merge` (re-test) | pipe tables; a table with a merged cell is HTML `<table>` with `rowspan`/`colspan` | every table is a pipe table; `^^` = merged into the cell above, `\|\|` = the cell to the left extends here (§5.2) |
| `styleattr` (re-test) | `<div style="Name">`, and `<table style="Name">` … `</table>` around a pipe table | the same with `class="Name"` |
| `slides` (re-test) | `<slide layout="…">` with `<title>`, `<body>`, `<left>`, `<right>`, `<notes>`, `<shape>` lines | Slidev-style exactly as §5.3 defines it now, including `<shape>` lines after the slots |
| `tablestyle` (new, §10.7) | a line `<table style="Name">`, the pipe table unchanged, a line `</table>` | a line `{style="Name"}` directly before the header row of the pipe table |

In `tablestyle`, the tables use the `^^`/`||` markers and paragraphs use `<div style="Name">`, as in §5.2. Both
candidates share the same text for these parts. The two table-style forms are defined exactly:

- **A.** `<table style="Name">` stands alone on its line, directly before the header row. `</table>` stands alone
  on its line, directly after the last row. Nothing else is between them, not even a blank line. `<table>` takes
  no other attribute. Errors: `unclosed_tag` (no `</table>`) and `wrapper_content` (a blank line, text, a second
  table or HTML rows inside).
- **B.** `{style="Name"}` stands alone on its line, directly before the header row, with no blank line between
  them. The braces hold only `style="Name"`. Nothing closes the table. Errors: `attr_orphan` (the line is not
  followed directly by a table row), `attr_form` (for example `{.Name}` or `{}`), and `invented_attr` (another
  key).

The subjects see the guides in `guides/<decision>-<cand>.md`. Within a decision, A and B share every common
sentence. Their candidate parts are about the same length and never mention each other:

| decision | A chars | B chars |
|---|---|---|
| merge | 1,304 | 1,282 |
| styleattr | 1,279 | 1,279 |
| tablestyle | 2,106 | 2,079 |
| slides | 1,574 | 1,637 |

## What makes it hard

- **Seeds of 3,000–6,000 characters.** The range is 3,015–4,983 over the 24 seeds. The seeds are realistic Korean
  office documents:
  - merge: a regional sales report, a development weekly update, and a public-agency budget request (hwpx);
  - styleattr: executive minutes, a logistics-center proposal, and a municipal notice with Korean style names;
  - tablestyle: a supplier-evaluation report, a training plan with Korean style names, and a complaint report;
  - slides: a 21-slide sales-strategy deck, a 23-slide KPI review, and a 23-slide customer proposal with Korean
    layout names.

  Each seed has several similar tables or slides and a lot of repeated text: the same branch names in three
  tables, identical header rows under identical style lines, `질문 시 설명` as the notes of five slides, and two
  `출처` shapes with the same text. An exact-match `old` has to include unique context.
- **Merges described by meaning.** For example: "each 구분 should appear once for all of its sessions", "the
  직급 column should show each 직급 once", or "교통 and 교통정책과 should each still appear once for all 교통
  사업".
- **Structural edits where the syntaxes differ.** Each item gives the task id and the unit:
  - insert a row inside a merged group: at the end (report1-e1), in the middle (report2-e3), and at the start,
    which moves the origin cell (report3-e4);
  - delete the row that holds a merged cell's text (report1-e2, report2-e4);
  - add a column inside a colspan group (report1-e3), or a column that widens the colspan of a total row
    (report3-e1);
  - split a merged cell horizontally (report2-e1) or vertically (report3-e2);
  - merge a 2×2 block (report1-e4, report2-e2);
  - in `tablestyle`: split a styled table in two (form1-e3), join two styled tables (form3-e4), move a styled table
    with its section (form2-e4), add a styled merged table (form2-e2), style or unstyle a table (form1-e1,
    form1-e4, form3-e2), and add a row at the end of a styled table (form3-e1).
- **Style requests with no matching style.** The correct answer to these is a refusal:
  - "make it red" (memo1-e2);
  - "highlight in yellow" (memo2-e3);
  - "a red box" (memo3-e2);
  - "a red header row" (form1-e2);
  - "thicker lines" (form2-e3);
  - "full page width, equal columns" (form3-e3).

  Next to them are tempting requests that *do* have a matching style and must not be refused: "right-align the
  last line as a sign-off" (memo1-e4), and "center it and make it large, it is the sender's name" (memo3-e4).
- **Multi-word names and near-duplicates.** Examples: `Grid Table 4` / `Grid Table 4 Accent 1`,
  `Body Text` / `Body Text Indent` / `Body Text 2`, `Key Message` / `Key Message Small`, `Callout` /
  `Callout Warning`, `Medium Grid 3` / `Medium Grid 3 - Accent 1` / `Light Grid - Accent 1`, `본문 들여쓰기` /
  `본문 들여쓰기 2`, and `눈금 표 4` / `눈금 표 4 - 강조색 1`. Styles are always asked for by description, never
  by name.
- **Slides:**
  - split one slide into two, with the notes staying on the first slide (deck1-e1) or moving to the second
    (deck2-e3);
  - split a two-content slide into two single-content slides that keep its shape (deck3-e3);
  - move a slide (deck2-e1, deck3-e2);
  - move a bullet from the left slot to the right slot, where a notes slot and a shape follow the right slot
    (deck1-e2), or swap the two columns (deck3-e1);
  - add a two-content slide in the middle (deck1-e4) or at the very end (deck2-e2);
  - edit, add or remove notes whose text repeats on other slides (deck1-e3, deck2-e4 on a slide with a shape,
    deck3-e4).

## Units

A unit is one (decision, candidate, replicate), with an id like `r2-merge-A-1`. There are 4 × 2 × 3 = 24 units,
each with 5 tasks: 1 write task (`<stem><rep>-w`) and 4 edit tasks (`<stem><rep>-e1` … `-e4`). The task ids are
neutral: `report`, `memo`, `form`, `deck`. Both candidates of a replicate share the same seed content, the same
task wording and the same intended result. `build.py` asserts that the A and B renderings of the seed, of every
target and of every write gold parse to identical semantic models.

Refusal tasks: `memo1-e2`, `memo2-e3`, `memo3-e2`, `form1-e2`, `form2-e3`, `form3-e3`. `data/index.json` lists
them per unit. `merge` and `slides` have no refusal task, but their prompts carry the same refusal rule, so
over-refusal shows there too.

## Answer format and refusals

The answer format is the same as round 1:

```
{"answers": [{"task_id", "text"} | {"task_id", "edits": [{"old", "new"}]}]}
```

Every prompt adds this rule. If an edit task asks for something the syntax and the names cannot express, the
answer is `{"task_id": …, "text": "REFUSE: <reason>", "edits": []}`. The scorer handles refusals as follows:
- A refusal where the gold is a refusal is valid and landed. The reason text is not scored.
- A refusal where the gold is an edit is valid but not landed (`wrong_refusal`).
- A refusal that also carries edits is invalid (`bad_refusal`).
- An answer that edits where the gold is a refusal is invalid if it writes CSS, direct formatting, an invented
  style name, tag or attribute (`css_in_attr`, `unknown_tag`, `unknown_style`, `split_name`, `invented_attr`,
  `attr_form`). If it only uses existing styles, it is valid but not landed (`should_refuse`).

## Layout

- `content.py`: seeds, task wording and intended results in one form that belongs to neither syntax. Tables are
  dense grids with `^^` / `<<` markers, and each edit is a function that changes a copy of the seed. A refusal
  edit is `(instruction, None, reason)`.
- `guides.py`: the guides, rendered to `guides/*.md`.
- `build.py`: renders and checks everything. It writes:
  - `data/units/<unit>.json`
  - `data/gold/<unit>.json`
  - `seeds/<unit>.txt`
  - `prompts/<unit>.md`
  - `data/index.json`

  It derives gold edits by a line diff, with each `old` widened until it occurs exactly once (the same method as
  round 1), and it asserts that every seed is 3,000–6,000 characters.
- `score.py`: the scorer. `selftest.py`: the self-test.

The kit uses only the standard library (Python 3.11 or later). The build is deterministic: rebuilding rewrites the
generated files byte for byte.

```
cd fluency/round2
python3 build.py && python3 selftest.py      # must end with SELFTEST PASSED
python3 score.py r2-merge-B-1 answers.json
```

## Scoring

The scorer works as in round 1. It parses the answer into a common semantic model:
- Documents: the front matter plus blocks. A table is `{style, grid}`, and a grid lists
  `[r, c, text, rowspan, colspan, is_header]` for each origin cell. `is_header` means the cell starts in the first
  row, so `<th>` versus `<td>` is not scored.
- Presentations: slides as `{layout, slots, shapes: [[id, name, text]]}`. Slot order and shape position inside a
  slide are not scored.

`landed` compares the result of an edit to the gold result. For a write task, it checks that the brief's blocks
appear in order (documents) or that the slides equal the brief exactly (presentations). For each task the scorer
prints `{"results":[{task_id, valid, landed, error, chars, flags, detail}]}`.

Round 2 adds these flags to round 1's:

| flag | meaning |
|---|---|
| `wrapper_content` | something other than a pipe table inside `<table …>` … `</table>` |
| `wrapper_form` | the `<table …>` line is not alone on its line |
| `attr_orphan` | a `{style="Name"}` line is not directly followed by a table row |
| `attr_form` | a malformed `{style="Name"}` line |
| `shape_form` | a malformed shape line |
| `invented_shape` | a shape id that is not in the seed; a write task has no shapes |
| `duplicate_shape` | the same shape written twice |
| `should_refuse` | edited where the gold is a refusal |
| `wrong_refusal` | refused where the gold is an edit |
| `bad_refusal` | a refusal that also carries edits |

## Self-test

`python3 selftest.py` checks the following:
- All 120 gold answers (24 units × 5 tasks) are valid and landed, with no flags.
- 57 broken answers are caught, each with its expected flag: merge 15, styleattr 15, tablestyle 14, slides 13, and
  at least 3 per decision under each candidate. Examples:
  - a rowspan not widened after an inserted row;
  - a deleted origin row whose label was not moved, so in B the `^^` silently joins the group above;
  - a colspan not widened, a covered cell left in, and `||` miscounted;
  - `style="color:red"`, `<span style>`, `<mark>`, an invented style, and an existing but wrong style where the
    answer should be a refusal;
  - `class="Indent"` and `class="Grid Table 4 Accent"`;
  - `wrapper_content`, `unclosed_tag`, `attr_orphan`, `{.Name}` and `{style="color: red"}`;
  - a split table whose second half lost its style;
  - an `old` that is ambiguous because of the repeated text;
  - the Slidev closing `---`, a bullet placed after a shape line, a missing slide separator, an invented shape,
    and wrong refusals.
- 14 correct answers written differently from the gold pass, at least 3 per decision. Examples: a 2×2 merge
  written with `^^ ^^`, minimal edits, a refusal worded differently, notes written after a shape line, and
  `* ` bullets.
- The command-line round trip passes.

## Running a round

This is the same procedure as round 1. For each unit in `data/index.json`, send `prompts/<unit>.md` as the whole
user message, in a fresh conversation with no tools. Save the JSON reply as `runs/<unit>/<model>-first.json` and
score it. Once, send `error` back for each invalid task and save the reply as `<model>-fix.json`. Tally per
decision and candidate: valid, landed, flags, `should_refuse` / `wrong_refusal`, and the sum of `chars`.

## Limits

- The descriptions of styles and layouts are the subject's only guide to which named style fits a request. A
  refusal gold assumes that no listed description matches, and the wording was chosen so that none does.
- Write tasks check the brief's blocks in order and allow extra blocks. Edit tasks require the exact intended
  result.
- `chars` stands in for tokens, as in round 1.
