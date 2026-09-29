# Round 5 candidates: geometry in the Presentation format

DESIGN.md §5.3 built slides "from the layout's placeholders — never geometry": the text held `layout:`,
`::slot::` markers, `<shape id name>text</shape>` lines and `<keep id kind summary/>` lines, and every position and
size stayed in the remainder. Checked against real decks in PowerPoint, a canvas without positions and sizes is
too limited (2026-09-29). Round 5 redesigns the format with geometry in it and measures four candidates.

The full renderings of the two decks validation/office-kit uses (`crates/hanji-pptx/corpus/korean-deck.pptx`,
Korean, 7 slides; `shapes.pptx`, Apache POI test data, 6 slides) are in [seeds/](seeds/):
`deck1-<candidate>.txt` and `deck2-<candidate>.txt`; `deck3-*` is the synthetic 16:9 deck of the fluency test.

## What the current text leaves out

- **Objects it does not show at all.** `shapes.pptx` has 19 objects on its slides. Today's text shows 10: 3 slots,
  2 `<shape>` lines and 5 `<keep/>` lines (one of them the group). It does not show the 3 connectors on slide 1
  or the 6 textless rectangles on slide 6, and the group's three shapes are one `<keep/>`. So rule 8 ("the model
  sees what it cannot edit") does not hold for lines or textless shapes, and a model asked what overlaps the
  cloud on slide 1 cannot see the elbow connector that crosses it.
- **Placeholders moved off their layout.** Across the 21 corpus decks (108 slides), 49 of the 138 placeholders
  on slides (36%) have their own box, moved or resized away from the layout's. In `60810.pptx` it is 35 of 48.
  korean-deck has none (python-pptx writes none), which is why it looks clean today.
- Also in the corpus: 84 other shapes (19 without text), 25 pictures, 19 graphic frames (tables, charts, OLE), 4
  groups, 4 connectors, and no rotated object.

## The four candidates on the same slide

korean-deck, slide 3 (`Two Content`, 720 x 540 pt): the layout's three placeholders, all inherited, and one text
box that overlaps the left column by 7 pt.

**A: canvas-first, points.** Every object is a line or a block with its box, and slots show theirs too:

```
layout: Two Content
::title box="36 22 648 90"::
지역별 현황
::left box="36 126 318 356"::
- 수도권 21곳
  - 서울 14곳
::right box="366 126 318 356"::
- 지방 13곳
  - 부산 5곳
::shape id="s5" name="출처" box="36 475 288 29"::
출처: 내부 집계
```

**Ap: A in percent of the slide** (the units probe; x and w are % of the width, y and h % of the height):

```
::title box="5 4 90 16.7"::  …  ::shape id="s5" name="출처" box="5 88 40 5.3"::
```

**B: today's slots plus a box.** A slot shows a box only when the slide moved it off its layout (the layout list
gives every slot's box). Every other object always shows its box. Shapes stay one-line tags:

```
layout: Two Content
::title::
지역별 현황
::left::
- 수도권 21곳
  - 서울 14곳
::right::
- 지방 13곳
  - 부산 5곳
<shape id="s5" name="출처" box="36 475 288 29">출처: 내부 집계</shape>
```

**C: a 12 x 12 grid.** A's structure, with every box written as the cells it covers, spreadsheet style:

```
::title cells="B1:K2"::  ::left cells="B4:F11"::  ::right cells="G4:K11"::  ::shape id="s5" name="출처" cells="B12:E12"::
```

In C the left column's bottom edge (482 pt) and the text box's top edge (475 pt) both round to grid line 11
(495 pt), so the column ends where the box starts and the overlap is gone from the text. Both models answered "none" there (see RESULTS.md).

shapes.pptx slide 1 under B shows what the redesign adds for every candidate. Connectors are `<line from to/>`,
and pictures and tables are `<keep/>` with a box:

```
layout: Blank
<shape id="s4" name="TextBox 3" box="72 72 180 29">Learning PPTX</shape>
<line id="s6" name="Straight Connector 5" from="84 144" to="252 144"/>
<shape id="s7" name="Freeform 6" box="47 211 185 136">Cloud</shape>
<keep id="kqld8" kind="picture" summary="Picture 1" box="402 78 144 132"/>
<keep id="kmtnf" kind="table" summary="Table 2: Column1 Column2 Column3 data1 data2 data3" box="300 372 372 96"/>
<line id="s8" name="Straight Arrow Connector 7" from="468 366" to="468 216"/>
<line id="s10" name="Elbow Connector 9" from="186 252" to="402 144"/>
```

## Settled for every candidate

| question | answer |
|---|---|
| z-order | The slide's objects in file order, back to front, slots included. Reordering lines is the z-order edit (tested: bring to front, send behind). `::notes::` comes last and is not an object |
| rotation, flip | `rot="15"` in degrees clockwise about the centre; the box is the unrotated one, as `a:xfrm` stores it. `flip="h"`, `"v"`, `"hv"`. Left out when 0 or none |
| groups | `<group id name box>`, its objects, `</group>`. Children show their boxes in slide coordinates, mapped through the group's `chOff/chExt`, so they read like any other object. The group's box is the union of its children. Changing the group box alone moves or scales the children: canonical form rewrites their boxes. Changing children re-derives the group box. Changing both is accepted only when the written group box matches the children (within 2 pt), and refused otherwise (`group_box_conflict`). A rotated or flipped group stays one `<keep kind="group">` with a box, because its children cannot be shown in slide coordinates without a rotation of their own |
| lines, connectors | `<line id name from="x y" to="x y"/>`: the two ends, derived from `off/ext/flipH/flipV`, which is easier to read than a zero-height box. Writing the ends rewrites `off`, `ext` and the flips. The arrowheads, the elbow path and the connection ids (`stCxn`/`endCxn`) stay in the remainder (see "refused") |
| pictures, charts, tables | `<keep id kind summary box/>`. The box may be edited to move or resize the object; `id`, `kind` and `summary` never. Resizing a table scales its `gridCol` widths and row heights in proportion; a chart's frame takes the new box; a picture keeps its crop |
| new objects | Text box: a shape without `id` (A/Ap/C `::shape box::` + text; B `<shape box>text</shape>`), `txBox="1"`, no fill, the master's `otherStyle`. Picture from a file: `<keep kind="picture" src="…" box/>`, the only new `<keep/>`. Line: `<line from to/>`. The write assigns ids and names (`TextBox 7`), and the read shows them |
| empty placeholders | Left out, as today. A placeholder without text shows nothing in a slide show; writing its marker creates it |
| new decks from a template | The agent writes `layout:` and bare slot markers (`::title::`), with no geometry. Every slot sits where its layout puts it. That is B's canonical form, and A, Ap and C accept it too. Tested: 3 new decks and 3 new comparison slides per model and candidate, 48 answers, and none wrote a box |

## Different per candidate

| | A | Ap | B | C |
|---|---|---|---|---|
| unit | points, whole numbers | percent of slide width / height, one decimal | points, whole numbers | 12 x 12 cells (60 x 45 pt on 4:3, 80 x 45 pt on 16:9) |
| slot box shown | always (inherited ones as the layout's) | always | only when the slide has its own `a:xfrm` | always |
| layout boxes needed to read a slide | no | no | yes (in the read's layout list) | no |
| shape syntax | `::shape …::` block, Markdown body | as A | one line, `<p/>` paragraphs (today's) | as A |
| keep rule for rounded numbers | per number: a number equal to how the stored value is shown keeps the stored EMU | as A | as A | per edge: an edge moved by k cells moves exactly k cells from its stored place; a new object sits on grid lines |
| what it cannot write | nothing geometric finer than 1 pt | nothing finer than 0.1% (0.72 pt x 0.54 pt) | as A | any edge not on its old offset + whole cells: 3 cm, 0.7–1.1 cm tall, aligning to an off-grid edge (11 of 30 tasks) |
| what it hides on read | nothing | nothing | nothing (inherited boxes are in the layout list) | overlaps and misalignments under half a cell (the 7 pt overlap above) |

## Units compared

The title box of korean-deck (`457200 274638 8229600 1143000` EMU) in each unit:

| unit | title box | step | reads as | for | against |
|---|---|---|---|---|---|
| EMU | `457200 274638 8229600 1143000` | exact | the file's own numbers | GetPut needs no rule | §6: hopeless for a model; 914,400 per inch |
| pt | `36 22 648 90` | 1 pt (0.35 mm) | whole numbers | the unit of font sizes; the Google Slides API's other unit; 72 per inch; both axes on one scale, so a square is `w = h` | cm requests need x 28.35 |
| cm | `1.27 0.76 22.86 3.18` | 0.01 cm | two decimals | what Korean PowerPoint's Size and Position pane shows | decimals on every number; not tested |
| percent | `5 4 90 16.7` | 0.1% (0.72 x 0.54 pt) | one decimal | "right half", "centred" read directly | x and y on different scales (a 16:9 square is `w=10 h=17.8`); cm needs the slide size per axis; the one A/Ap/B miss in round 5 was this conversion |
| grid 12 | `B1:K2` | a cell (60 x 45 pt) | spreadsheet ranges | layout-level placement, bento slides | cannot hold real decks' geometry; hides overlaps |

## Laws

- **GetPut** (import, then export with no edit, gives back the original). Import shows each box rounded
  and keeps the exact `a:off`/`a:ext` in the remainder, as it does today. Export compares each written number
  with how the stored value is shown. An equal number keeps the stored EMU; a different one is converted
  exactly (pt x 12,700). When all four are equal, the `a:xfrm` element is not touched, so GetPut stays
  byte-exact on geometry and not only XML-equivalent. An inherited slot stays without `a:xfrm`. `build.py`
  checks this on the text for every seed and candidate: the rendering reads back to the same EMU and renders
  again byte for byte.
- **PutGet** (after an edit, importing the exported file shows what was written) holds on the canonical text,
  which the write returns (§5.1: canonical form is re-applied on every write). Numbers are shown rounded (a
  written `157.04` reads back as `157`). A group-box edit reads back with the children's boxes moved. New objects
  read back with their ids. In A, Ap and C, a slot written without a box reads back with its layout's box. The
  self-test checks that the canonical text of an edit reads back to itself.
- **Refused, with the reason (rule 1):** changing a `<keep/>`'s `id`, `kind` or `summary`; a new `<keep/>`
  other than a picture from a file; a new group, or a group box that disagrees with changed children;
  editing inside a rotated group; a negative width or height; a slot the layout does not have; moving an object
  a connector is attached to (`stCxn`/`endCxn`) without moving that connector's end. Rerouting attached
  connectors is not modelled, so the edit is refused rather than leave a connector detached in PowerPoint.
