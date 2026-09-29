"""Round 5 guides: a shared part, identical for every candidate, and one candidate part each.

The candidate parts have the same structure (geometry and units, slots, shapes, other objects, new objects, an
example on the same slide) and never mention each other."""

COMMON_HEAD = """\
A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. \
`size` in the front matter is the slide's width and height. Slides follow, separated by a line holding only `---`. \
The first line of every slide is `layout: Name`, with Name written as listed, spaces included. Besides what is \
described below, there are no other tags, markers or attributes.

### Text

Text is Markdown: one line per paragraph, `- ` bullets and `1. ` numbered items (a nested item is indented to its \
parent's text: two spaces under `- `), `**bold**`, `*italic*`, `<u>underline</u>`. A line holding only `<p/>` is an \
empty paragraph.

### Objects and z-order

A slide is a list of objects written back to front: an object written later is drawn on top of the ones before \
it. Speaker notes come last, after a line `::notes::`; they are not an object and have no position.

- **Slots** are the layout's placeholders (title, body, left, right, …). Use only the slots of the slide's layout \
and leave out a slot you do not fill; each slot appears at most once per slide.
- **Shapes** are text boxes and drawn shapes (rectangles, arrows, freeforms); a shape may hold no text.
- **Pictures, charts and tables** are `<keep id="…" kind="…" summary="…" …/>` lines. Their content is kept for \
you and cannot be read or changed here; you may move, resize or delete one, never change its `id`, `kind` or \
`summary`.
- **Lines** (lines, arrows and connectors) are `<line …/>` lines.
- **Groups**: a `<group …>` line, the group's objects, then a line `</group>`.

"""

COMMON_TAIL = """
### Changing a deck

- Change text by editing it. Delete an object by deleting its line(s); delete a slide by deleting it and one of \
the `---` lines around it.
- A new slide is written like any other, from one of the listed layouts.
- Objects you add have no `id`; ids are given when the file is saved. Never invent an id.
- Formatting (colours, fills, fonts, font sizes, line styles) is not in this file and cannot be changed here.
"""

PARTS = {}

PARTS['A'] = """\
### Positions and sizes

Every object shows where it is. `box="x y w h"` is its left edge, top edge, width and height, in points (pt) from \
the slide's top-left corner: 72 pt = 1 inch = 2.54 cm; x grows to the right, y downwards. Numbers are shown \
rounded to whole points; a number you leave as it is keeps its exact value, a number you change is used as \
written. `rot="15"` turns an object 15 degrees clockwise about its centre (its box is the unturned one); \
`flip="h"` or `flip="v"` mirrors it.

- **Slot:** a marker line `::title box="36 22 648 90"::`, then its text on the lines after it, up to the next \
object or `---`. The box shown is where the slot is; it is the layout's box unless someone moved the slot. Leave \
it as it is to keep the slot there; in a new slide write the marker without a box, `::title::`, and the slot \
sits where its layout puts it.
- **Shape:** a marker line `::shape id="s5" name="출처" box="36 475 288 29"::`, then its text on the lines \
after it (none for a shape without text).
- **Picture, chart, table:** `<keep id="k2" kind="picture" summary="map.png" box="400 300 200 150"/>`.
- **Line:** `<line id="s6" name="Arrow 5" from="84 144" to="252 144"/>`, its two ends as `x y`.
- **Group:** `<group id="g5" name="Group 4" box="120 108 258 152">`; its objects' boxes are on the slide, like \
any other. The group's box is the box around its objects: change it to move or resize the whole group, or change \
the objects themselves, not both.
- **New objects:** a text box is `::shape box="…"::` followed by its text; a picture from a file is \
`<keep kind="picture" src="images/map.png" box="…"/>`; a line is `<line from="…" to="…"/>`.

Example (a 720 x 540 pt slide):

```
layout: Two Content
::title box="36 22 648 90"::
지역별 현황
::left box="36 126 318 356"::
- 수도권 21곳
::right box="366 126 318 356"::
- 지방 13곳
<keep id="k7" kind="picture" summary="지도" box="560 20 120 80"/>
::shape id="s4" name="출처" box="36 490 288 29"::
출처: 내부 집계
::notes::
전년 대비 강조
```
"""

PARTS['Ap'] = """\
### Positions and sizes

Every object shows where it is. `box="x y w h"` is its left edge, top edge, width and height, in percent of the \
slide from its top-left corner: x and w are percent of the slide's width, y and h percent of its height (see \
`size`); x grows to the right, y downwards. Numbers are shown rounded to one decimal; a number you leave as it is \
keeps its exact value, a number you change is used as written. `rot="15"` turns an object 15 degrees clockwise \
about its centre (its box is the unturned one); `flip="h"` or `flip="v"` mirrors it.

- **Slot:** a marker line `::title box="5 4 90 16.7"::`, then its text on the lines after it, up to the next \
object or `---`. The box shown is where the slot is; it is the layout's box unless someone moved the slot. Leave \
it as it is to keep the slot there; in a new slide write the marker without a box, `::title::`, and the slot \
sits where its layout puts it.
- **Shape:** a marker line `::shape id="s5" name="출처" box="5 88 40 5.4"::`, then its text on the lines after \
it (none for a shape without text).
- **Picture, chart, table:** `<keep id="k2" kind="picture" summary="map.png" box="55.6 55.6 27.8 27.8"/>`.
- **Line:** `<line id="s6" name="Arrow 5" from="11.7 26.7" to="35 26.7"/>`, its two ends as `x y`.
- **Group:** `<group id="g5" name="Group 4" box="16.7 20 35.8 28.1">`; its objects' boxes are on the slide, like \
any other. The group's box is the box around its objects: change it to move or resize the whole group, or change \
the objects themselves, not both.
- **New objects:** a text box is `::shape box="…"::` followed by its text; a picture from a file is \
`<keep kind="picture" src="images/map.png" box="…"/>`; a line is `<line from="…" to="…"/>`.

Example (a 720 x 540 pt slide):

```
layout: Two Content
::title box="5 4 90 16.7"::
지역별 현황
::left box="5 23.3 44.2 66"::
- 수도권 21곳
::right box="50.8 23.3 44.2 66"::
- 지방 13곳
<keep id="k7" kind="picture" summary="지도" box="77.8 3.7 16.7 14.8"/>
::shape id="s4" name="출처" box="5 90.7 40 5.4"::
출처: 내부 집계
::notes::
전년 대비 강조
```
"""

PARTS['B'] = """\
### Positions and sizes

A slot sits where its layout puts it, and shows no position: the layout list gives each slot's box. Every other \
object shows where it is. `box="x y w h"` is its left edge, top edge, width and height, in points (pt) from the \
slide's top-left corner: 72 pt = 1 inch = 2.54 cm; x grows to the right, y downwards. Numbers are shown rounded \
to whole points; a number you leave as it is keeps its exact value, a number you change is used as written. \
`rot="15"` turns an object 15 degrees clockwise about its centre (its box is the unturned one); `flip="h"` or \
`flip="v"` mirrors it.

- **Slot:** a marker line `::title::`, then its text on the lines after it, up to the next object or `---`. A \
slot someone moved away from its layout's box shows its own box: `::title box="36 60 648 90"::`. Add a box to \
move a slot; remove it to put the slot back where the layout puts it.
- **Shape:** one line, `<shape id="s5" name="출처" box="36 475 288 29">text</shape>`; `<p/>` starts its next \
paragraph. A shape without text is `<shape id="s9" name="Oval 8" box="…"/>`.
- **Picture, chart, table:** `<keep id="k2" kind="picture" summary="map.png" box="400 300 200 150"/>`.
- **Line:** `<line id="s6" name="Arrow 5" from="84 144" to="252 144"/>`, its two ends as `x y`.
- **Group:** `<group id="g5" name="Group 4" box="120 108 258 152">`; its objects' boxes are on the slide, like \
any other. The group's box is the box around its objects: change it to move or resize the whole group, or change \
the objects themselves, not both.
- **New objects:** a text box is `<shape box="…">text</shape>`; a picture from a file is \
`<keep kind="picture" src="images/map.png" box="…"/>`; a line is `<line from="…" to="…"/>`.

Example (a 720 x 540 pt slide):

```
layout: Two Content
::title::
지역별 현황
::left::
- 수도권 21곳
::right::
- 지방 13곳
<keep id="k7" kind="picture" summary="지도" box="560 20 120 80"/>
<shape id="s4" name="출처" box="36 490 288 29">출처: 내부 집계</shape>
::notes::
전년 대비 강조
```
"""

PARTS['C'] = """\
### Positions and sizes

The slide is a grid of 12 columns, `A` to `L` from left to right, and 12 rows, `1` to `12` from top to bottom \
(see `grid` in the front matter for the size of a cell). Every object shows where it is: `cells="B1:K2"` is the \
range of cells it covers, from its top-left cell to its bottom-right cell, as in a spreadsheet; one cell is \
`cells="K10"`. Edges are shown at the nearest grid line. An edge you leave as it is keeps its exact place; moving \
an edge by some cells moves it by exactly that many cells; a new object sits exactly on the grid. `rot="15"` \
turns an object 15 degrees clockwise about its centre (its cells are the unturned box); `flip="h"` or `flip="v"` \
mirrors it.

- **Slot:** a marker line `::title cells="A1:L2"::`, then its text on the lines after it, up to the next object \
or `---`. The cells shown are where the slot is; they are the layout's unless someone moved the slot. Leave them \
as they are to keep the slot there; in a new slide write the marker without cells, `::title::`, and the slot sits \
where its layout puts it.
- **Shape:** a marker line `::shape id="s5" name="출처" cells="B11:E11"::`, then its text on the lines after it \
(none for a shape without text).
- **Picture, chart, table:** `<keep id="k2" kind="picture" summary="map.png" cells="H7:J9"/>`.
- **Line:** `<line id="s6" name="Arrow 5" from="B4" to="E4"/>`, the cells its two ends are in (a new line runs \
between cell centres).
- **Group:** `<group id="g5" name="Group 4" cells="C3:H6">`; its objects' cells are on the slide, like any \
other. The group's cells are the cells around its objects: change them to move or resize the whole group, or \
change the objects themselves, not both.
- **New objects:** a text box is `::shape cells="…"::` followed by its text; a picture from a file is \
`<keep kind="picture" src="images/map.png" cells="…"/>`; a line is `<line from="…" to="…"/>`.

Example (a 720 x 540 pt slide, cells of 60 x 45 pt):

```
layout: Two Content
::title cells="B1:K2"::
지역별 현황
::left cells="B4:F11"::
- 수도권 21곳
::right cells="G4:K11"::
- 지방 13곳
<keep id="k7" kind="picture" summary="지도" cells="J1:K2"/>
::shape id="s4" name="출처" cells="B12:E12"::
출처: 내부 집계
::notes::
전년 대비 강조
```
"""

GUIDES = {c: COMMON_HEAD + PARTS[c] + COMMON_TAIL for c in PARTS}
