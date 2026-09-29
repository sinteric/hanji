# Fluency test results, round 5

## Round 5 — 2026-09-29

**Subjects:** Claude Opus and Claude Sonnet. Each got all 21 blind units: part 1, 4 candidates × 3 seeds × 10
tasks (120 tasks per model), and part 2, 3 candidates × 3 seeds × 6 harder tasks (54 tasks per model). That is 348
answers. There were no tools and a fresh conversation per unit, as in [README.md](README.md#runs). Prompts were
8,590–10,859 chars and arrived whole. Opus had no invalid answer. Sonnet had two, both under C, and each was sent
back once with its validator error (the fix round).

Answers are in `runs/<unit>/<model>-first.json`, and `runs/<unit>/sonnet-fix.json` for the two units sent back.
Re-score any of them with `python3 score.py <unit> <file>`; a re-score of every file reproduces the numbers here.
The self-test passes (`python3 selftest.py`): 174 gold answers, all valid, and landed except the 11 C cannot land
by construction; 21/21 broken answers caught; 11/11 alternative answers pass; the group move and PutGet on the
canonical text hold; cli ok.

### What round 5 decides

§10 item 9: **geometry in Presentations** — the candidate format, its unit, and whether a slot shows its box.
[CANDIDATES.md](CANDIDATES.md) has the four candidates on the two real decks and the rules they share.

| candidate | box of a slot | box of every other object | unit |
|---|---|---|---|
| A | always (the layout's unless moved) | always | pt |
| Ap | always | always | % of slide width / height |
| B | only when moved off the layout (the layout list gives the rest) | always | pt |
| C | always | always | 12 × 12 grid cells |

### Landed, first try

| model | part | A | Ap | B | C |
|---|---|---|---|---|---|
| Opus | 1 (of 30) | 30 | 30 | 30 | **17** |
| Opus | 2 (of 18) | 18 | **17** | 18 | – |
| Sonnet | 1 (of 30) | 30 | 30 | 30 | **17** |
| Sonnet | 2 (of 18) | 18 | 18 | 18 | – |

Every answer under A, Ap and B was valid: 288/288. Under C Opus had 30/30 valid and Sonnet 28/30. Under A and B
both models landed all 96 tasks. Ap lost one (Opus). C landed 17 of 30 on both models.

C, split by whether the task can be done at all on the grid (`unreachable`: the best gold misses the 2 pt check):

| model | reachable (19) landed | unreachable (11): refused | unreachable: attempted, missed |
|---|---|---|---|
| Opus | 17 | 9 | 2 |
| Sonnet | 17 | 5 (6 after the fix round) | 6 (5 after it) |

The C tasks that cannot land are 3 of 10 on seed 1, 4 on seed 2 and 4 on seed 3. They are every task that
asks for an edge not on a whole-cell step from where it was: 3 cm and 2 cm moves, a 0.7–1.1 cm tall text box
under a title at 111.6 pt, a picture scaled by 2 or 1.5, and aligning to an edge 12–36 pt off the grid.

### Fix rate after one validator error

| model | A | Ap | B | C |
|---|---|---|---|---|
| Opus | – (0 invalid) | – (0 invalid) | – (0 invalid) | – (0 invalid) |
| Sonnet | – (0 invalid) | – (0 invalid) | – (0 invalid) | 2/2 valid, 0/2 landed |

Both were unreachable tasks. One became a refusal: "Both left edges show as column C, so any offset between them is
smaller than a grid cell and cannot be read or set exactly". The other became a no-op edit (`old` = `new`).

### Size

Answer size in characters, summed over the tasks of each cell (the answer text, or the sum of `old` + `new`):

| model | part | A | Ap | B | C |
|---|---|---|---|---|---|
| Opus | 1 | 3,590 | 3,601 (+0.3%) | 3,365 (−6.3%) | 3,793 (+5.7%) |
| Sonnet | 1 | 2,949 | 3,211 (+8.9%) | 2,963 (+0.5%) | 3,277 (+11.1%) |
| Opus | 2 | 2,735 | 2,660 (−2.7%) | 2,285 (−16.5%) | – |
| Sonnet | 2 | 2,919 | 2,675 (−8.4%) | 2,273 (−22.1%) | – |
| Opus | both | 6,325 | 6,261 (−1.0%) | 5,650 (−10.7%) | – |
| Sonnet | both | 5,868 | 5,886 (+0.3%) | 5,236 (−10.8%) | – |

Input, the seed file in each syntax (the prompts also carry the same layout list with boxes for every candidate):

| seed | A | Ap | B | C |
|---|---|---|---|---|
| 1 korean-deck (14 inherited slots) | 1,241 | 1,240 | 981 (−21%) | 1,179 |
| 2 shapes (3 inherited slots, 16 other objects) | 1,749 | 1,813 | 1,688 (−3.5%) | 1,621 |
| 3 product (1 moved slot, 14 inherited) | 1,757 | 1,821 | 1,583 (−9.9%) | 1,622 |
| all | 4,747 | 4,874 (+2.7%) | 4,252 (−10.4%) | 4,422 (−6.8%) |

Where B's answers are smaller:

- **Slot geometry** (part 2 `x2`, `x5`, `x6`). B's `old` is the bare marker: `::left::` → `::left box="36 126
  261.31 356"::` (40 chars, both models). A's `old` carries the box (61 chars).
- **Everything else is anchoring.** On the tasks that edit a line written the same way in A and B (a `<keep/>` or
  `<line/>`), the gap is which `old` a model chose, not the syntax. Opus quoted the whole picture line under A in
  `deck1-e2` (147 chars) and a shorter span under B (79). Sonnet wrote 39 under both. So the 11% overall is part
  syntax (slot markers, 10% smaller input) and part noise.
- **Ap is no smaller than A.** One decimal per number offsets the shorter percentages.

### Refusals

All 24 refusal tasks refused, for the right reason, under every candidate and model. The three were a table
figure ("The table's content is kept opaque and cannot be read or changed in this file"), red and 24 pt ("Text
colour and font size are formatting, which this file format cannot express or change") and a fill colour. No
`should_refuse`. Outside C, no `wrong_refusal`. Under C, the refusals of unreachable tasks named the grid, correctly:
"A new object sits exactly on the grid, whose rows are 45 pt (about 1.59 cm) tall, so a text box between 0.7 cm
and 1.1 cm tall cannot be expressed" (Opus, `deck1-e3`).

### Failure patterns

1. **The grid hides small overlaps** (C, both models, `deck1-q1` and `deck3-q1`, flag `wrong_answer`, all 4 answers
   to those two tasks). The 출처 box overlaps the left column by 7 pt (475–482 pt). Both edges round to
   grid line 11, so in C the column is `B4:F11` and the box `B12:E12`, and both models answered `none`. The NEW
   badge overlaps the moved title by 7 pt. Both edges round to line 3, so both models answered `body` without
   `title`. These are valid answers with a wrong fact, and no validator can catch them.
2. **The grid hides misalignment** (C, Sonnet, `deck2-e4` and `deck3-e4`). Right Arrow 3 is 24 pt right of
   Rectangle 1 and the 2분기 label 10 pt right of the 1분기 label. Each pair shows the same column, so Sonnet
   answered "Already aligned: both left edges are in column C" and, on the other, empty edits. Opus refused both,
   saying the offset is below a cell.
3. **Near misses on the grid** (C, 6 answers). The attempts moved or resized by whole cells: a picture to `J9:K10`
   (left 516 pt, needed 504), a picture to `B10` (36 pt, needed 72), a note one row down (45 pt, needed 56.7 =
   2 cm), a logo to `K1:L2` (left 784, needed 825). Each is the nearest cell, and it misses.
4. **cm to percent** (Ap, Opus, `r5x-Ap-3` `deck3-x5`, `not_landed`). Asked to put two labels 0.5 cm below the chart
   and 0.5 cm apart on a 16:9 slide, Opus wrote a top of 91.95% (chart bottom 86.7% + 5.25%) and a gap of 2.95%.
   Both are 1.0 cm, not 0.5, on their own axis. The answer is valid, so the error is silent. The same task landed
   in points under A and B for both models (14.17 pt).

Nothing failed on what round 5 was built to provoke, under A and B:

- No inherited slot was misread in B: 14 read and edit tasks needed a slot's box from the layout list, including
  the overlap reads, the right column's right edge, the body's top-right corner, and the cm threshold in `x1`.
- No rounded number broke GetPut: every untouched object kept its exact EMU, checked in every landed answer.
- No `edit_no_match` or `edit_ambiguous`, including the repeated bullets `지방 13곳` and `음성 인식 지원` and the two
  identical logo lines, which differ only by id.
- No invented id or altered `<keep/>`, and no `group_box_conflict`. Group moves were written either by the group
  box alone or by the box and every child consistently.
- No box on any of the 48 new slides or decks. Every agent created slides without geometry.
- cm requests (3 cm, 2 cm, 1 cm, 0.5 cm, "0.7–1.1 cm tall", "within 2 cm") landed in points every time.

### Decisions

| decision | round 5 | verdict |
|---|---|---|
| format | A 96/96, B 96/96 (both models); Ap 95/96 Opus, 96/96 Sonnet; C 17/30 on both, 11 tasks unreachable, 2 silent wrong reads each | **B**: slots from the layout, every other object with its box |
| unit | pt 192/192 (A and B, both models); percent 191/192, its one miss the cm conversion; grid unreachable on 37% of part 1 | **points**, whole numbers shown, decimals accepted |
| slot box | shown only when moved: 10% smaller input and 11% smaller answers than always shown, no error either way | **only when the slide moved it**, with the used layouts' boxes in every read |

- **Format: B.** A and B tie at the ceiling on both models, on every kind of task in both parts. So correctness
  does not decide, and size and design fit do, as in round 1. B is 10–11% smaller in answers on both models and
  10% smaller in input (21% on a deck of inherited placeholders). Its saving is largest exactly where agents work
  most, on slides built from the layout. It is also truthful to the file. A box on a slot means the slide stores
  its own `a:xfrm`, so a slot at its layout's place looks different from one moved to the same place, and a layout
  change moves the first and keeps the second. A shows both the same way and needs a rule for which numbers are
  "inherited". A's advantage, a read without the layout list, did not show up: B misread no inherited slot. It
  would matter for a partial view that leaves the layout boxes out, so §5.3 requires every read to carry the boxes
  of the layouts it uses. B as tested is not the old format with a box added. Every object is shown, lines and
  textless shapes included (today's text leaves out 9 of the 19 objects in `shapes.pptx`). Every non-slot object
  always has its box, groups show their children, and pictures and text boxes can be added.
- **Unit: points.** The only miss among 288 A/Ap/B answers was a cm-to-percent conversion. In points every cm
  request landed. Points are whole numbers on one scale for both axes, and they are the unit of font sizes.
  Percent reads "right half" directly but puts x and y on different scales; on 16:9 a square is `w=10 h=17.8`.
  cm was not tested. It is what Korean PowerPoint's Size pane shows, at the cost of two decimals on every number.
- **Grid: rejected.** C cannot write 11 of its 30 part-1 tasks, and its reads are wrong where objects are within
  half a cell. The two silent wrong overlap answers and Sonnet's "already aligned" are the kind of error §8 cares
  about most. The grid remains a possible authoring aid (placing new objects by cells), not a representation of
  real decks.

### Limits

- 30 tasks per part-1 cell and 18 per part-2 cell, one run per prompt, two Claude models. A, Ap and B hit the
  ceiling in both parts, so the format decision rests on size and design, and a harder or longer round could still
  separate A from B. Candidates to try: a 40-slide deck read through a partial view without the layout list, a
  moved placeholder next to an inherited one, and weaker models.
- The seeds are two real decks and one written for the round. The corpus's decks with many moved placeholders
  (`60810.pptx`: 35 of 48 placeholders moved) were not used; there, B's markers carry boxes too and its input
  saving shrinks.
- Geometry is checked in the kit's own model (`deck.py`), not yet in `hanji-pptx` or in PowerPoint. Group scaling,
  table resizing, connector attachments and rotated groups are design, not measured.
- `chars` stands in for tokens, as before.
