# hanji fluency test, round 5 (DESIGN.md §5.3, §10 item 9)

§5.3 made Presentations Slidev-style and "built from the layout's placeholders — never geometry". Checked against
real decks in PowerPoint, a canvas without positions and sizes is too limited, so the Presentation format is
redesigned with geometry in it. Round 5 measures four candidates. The design of each, shown on the two real decks
validation/office-kit uses, is in [CANDIDATES.md](CANDIDATES.md). The results are in [RESULTS.md](RESULTS.md).

| candidate | what the text holds |
|---|---|
| `A` | canvas-first, points: every object is a line or a block with `box="x y w h"` in pt, in z-order; slots show their box too (the layout's, unless moved); text bodies are Markdown after a `::slot::` or `::shape …::` marker |
| `Ap` | A with percent of the slide: x and w in % of the width, y and h in % of the height, one decimal (the units probe) |
| `B` | today's slots plus a box: a slot shows `box` only when the slide moved it off its layout (the layout list gives every slot's box); every other object always shows its box; shapes stay one-line `<shape …>text</shape>` |
| `C` | A's structure on a 12 x 12 grid: every box is the range of cells it covers, `cells="B1:K2"`; a moved edge moves by whole cells |

All four share everything else: z-order is file order; `rot` and `flip`; `<line from to/>` for lines and
connectors; `<group box>` … `</group>` with children in slide coordinates; `<keep id kind summary box/>`; new text
boxes, pictures (`src`) and lines without `id`. Every object is shown, including the textless shapes and
connectors today's text leaves out. The guides are in `guides/slides-<candidate>.md`. Their shared part is
identical, and the candidate parts have the same structure (units, slots, shapes, other objects, new objects, an
example) and never mention each other.

## The kit

- `deck.py`: the semantic model (geometry in EMU), a renderer and a parser for each candidate. It includes the
  keep rule for rounded numbers: a number (in C an edge) written as it is shown keeps the stored EMU. It also
  matches the slides of an answer to the seed's, and handles groups, lines, rotation and new objects.
- `content.py`: the three seeds as data, the tasks, and a check for each task. Seeds 1 and 2 are
  `crates/hanji-pptx/corpus/korean-deck.pptx` and `shapes.pptx`: every object as the file stores it (python-pptx,
  2026-09-29), so it has the connectors and textless rectangles too. Seed 3 is a 16:9 Korean product deck
  written for the round. It has an overridden title box, a rotated badge, a group holding two lines, a chart, and
  two logos with the same summary.
- `build.py`: renders each seed in every candidate and asserts GetPut on the text: the rendering reads back to
  exactly the seed's EMU and renders again byte for byte. It derives gold edits by a line diff (round 3's
  `make_edits`) and scores every gold answer. It records the tasks a candidate cannot land by construction
  (`unreachable`: the best gold misses the check). It writes `seeds/`, `data/units/`, `data/gold/`, `prompts/`
  and `data/index.json`.
- `score.py`: the scorer, `python3 score.py <unit_id> <answers.json>`. `selftest.py`: the self-test.

Standard library only; round 2's `apply_edits` is imported from `../round2/score.py`.

```
cd fluency/round5
python3 build.py && python3 selftest.py      # must end with SELFTEST PASSED
python3 score.py r5-B-2 runs/r5-B-2/sonnet-first.json
```

## Units and tasks

Part 1 has 12 units, `r5-<candidate>-<seed>`, one per candidate and seed. Each has 10 tasks, worded the same for
every candidate:

| id | task | seed 1 (korean-deck, 4:3) | seed 2 (shapes, 4:3) | seed 3 (product, 16:9) |
|---|---|---|---|---|
| q1 | read: which objects overlap X | the 출처 box (overlaps the inherited left column by 7 pt) | the cloud (crossed by the elbow connector) | the rotated NEW badge (overlaps the moved title and the inherited body) |
| q2 | read: what sits in a region | the bottom-right quarter | the top-right quarter | the group's objects in the right half |
| e1 | move a shape | right edge onto the right column's | 3 cm right | 2 cm down |
| e2 | resize a picture | twice as large, bottom-right corner fixed | half as wide, aspect kept | 1.5x, top-right corner fixed (one of two logos with the same summary) |
| e3 | add a text box under the title | 0.7–1.1 cm tall, title's width, no overlap | the same | the same |
| e4 | align two shapes left | picture onto the table's left edge | a group child onto another | two labels 10 pt apart |
| e5 | change bullet text, no geometry | a bullet repeated on another slide | the subtitle's last line | a line repeated in the other column |
| e6 | a two-column comparison slide | Comparison layout, headings and bullets in the right slots | the same | the same |
| e7 | refusal | a table figure | red and 24 pt | a fill colour |
| w | a new 3-slide deck from the layout list | Korean | English | Korean |

Part 2 has 9 units, `r5x-<candidate>-<seed>`, for A, Ap and B only. Each has 6 harder tasks (`x1`–`x6`). Part 1 put
those three at the ceiling on both models, so part 2 asks for what could separate them: slot geometry (inherited in
B, so B must add a box), a read that needs inherited boxes and a cm threshold, z-order changes, centring and even
spacing, a whole-group move, a table resized about its centre, and a rotation removed plus a corner-to-corner move.
C was left out of part 2 because part 1 already showed it cannot write most geometric edits (RESULTS.md).

Every prompt has the guide, the layout list with each slot's box (in the candidate's syntax), the file, the tasks,
the refusal rule and the answer format. The answer format is round 4's plus read answers,
`"text": "ANSWER: <objects>"`. Prompts are 8,590–10,859 chars, under round 4's 20,000-char limit, so each was sent
whole as the user message.

## Scoring

- **valid:** the edits apply (round 2's rules: each `old` occurs exactly once) and the file parses under the
  candidate. That means known layouts and slots, known attributes, no invented id, no altered `<keep/>`, and
  boxes of the right form. Errors are written for the model, with the line.
- **landed:** the task's check passes on the parsed result, in EMU. Geometry must be within 2 pt (`content.TOL`).
  Every object the task does not name must be exactly as in the seed: box, text, attributes and z-order, with the
  keep rule applied. The slides must match the seed's in order. A write or a new slide must match the brief with
  every slot at its layout's place. A read answer must name exactly the expected objects, by id, name or slot name.
- **chars:** the answer text, or the sum of `old` + `new`, as in rounds 1–4.
- **unreachable:** a task whose best gold the candidate cannot land (C only). For these a refusal is the honest
  answer. It is flagged `wrong_refusal` like any refusal of a doable task, and RESULTS.md counts it apart.

## Runs

Each prompt went to Opus and Sonnet as the whole user message of a fresh `claude -p` conversation, with no tools
(`--tools ""`), from an empty directory. Answers are in `runs/<unit>/<model>-first.json`, saved as returned, with a
code fence stripped where there was one. The one fix round (two Sonnet units under C) is in `<model>-fix.json`;
the message sent is in `<model>-fix-message.json`. It was sent as a fresh conversation: the prompt, the first answer,
then the validator's errors. The subjects' sessions shared one session id, inherited from the harness, so
`--resume` would not reach the right conversation; a first attempt that way was discarded.
