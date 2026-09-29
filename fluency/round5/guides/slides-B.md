A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. `size` in the front matter is the slide's width and height. Slides follow, separated by a line holding only `---`. The first line of every slide is `layout: Name`, with Name written as listed, spaces included. Besides what is described below, there are no other tags, markers or attributes.

### Text

Text is Markdown: one line per paragraph, `- ` bullets and `1. ` numbered items (a nested item is indented to its parent's text: two spaces under `- `), `**bold**`, `*italic*`, `<u>underline</u>`. A line holding only `<p/>` is an empty paragraph.

### Objects and z-order

A slide is a list of objects written back to front: an object written later is drawn on top of the ones before it. Speaker notes come last, after a line `::notes::`; they are not an object and have no position.

- **Slots** are the layout's placeholders (title, body, left, right, …). Use only the slots of the slide's layout and leave out a slot you do not fill; each slot appears at most once per slide.
- **Shapes** are text boxes and drawn shapes (rectangles, arrows, freeforms); a shape may hold no text.
- **Pictures, charts and tables** are `<keep id="…" kind="…" summary="…" …/>` lines. Their content is kept for you and cannot be read or changed here; you may move, resize or delete one, never change its `id`, `kind` or `summary`.
- **Lines** (lines, arrows and connectors) are `<line …/>` lines.
- **Groups**: a `<group …>` line, the group's objects, then a line `</group>`.

### Positions and sizes

A slot sits where its layout puts it, and shows no position: the layout list gives each slot's box. Every other object shows where it is. `box="x y w h"` is its left edge, top edge, width and height, in points (pt) from the slide's top-left corner: 72 pt = 1 inch = 2.54 cm; x grows to the right, y downwards. Numbers are shown rounded to whole points; a number you leave as it is keeps its exact value, a number you change is used as written. `rot="15"` turns an object 15 degrees clockwise about its centre (its box is the unturned one); `flip="h"` or `flip="v"` mirrors it.

- **Slot:** a marker line `::title::`, then its text on the lines after it, up to the next object or `---`. A slot someone moved away from its layout's box shows its own box: `::title box="36 60 648 90"::`. Add a box to move a slot; remove it to put the slot back where the layout puts it.
- **Shape:** one line, `<shape id="s5" name="출처" box="36 475 288 29">text</shape>`; `<p/>` starts its next paragraph. A shape without text is `<shape id="s9" name="Oval 8" box="…"/>`.
- **Picture, chart, table:** `<keep id="k2" kind="picture" summary="map.png" box="400 300 200 150"/>`.
- **Line:** `<line id="s6" name="Arrow 5" from="84 144" to="252 144"/>`, its two ends as `x y`.
- **Group:** `<group id="g5" name="Group 4" box="120 108 258 152">`; its objects' boxes are on the slide, like any other. The group's box is the box around its objects: change it to move or resize the whole group, or change the objects themselves, not both.
- **New objects:** a text box is `<shape box="…">text</shape>`; a picture from a file is `<keep kind="picture" src="images/map.png" box="…"/>`; a line is `<line from="…" to="…"/>`.

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

### Changing a deck

- Change text by editing it. Delete an object by deleting its line(s); delete a slide by deleting it and one of the `---` lines around it.
- A new slide is written like any other, from one of the listed layouts.
- Objects you add have no `id`; ids are given when the file is saved. Never invent an id.
- Formatting (colours, fills, fonts, font sizes, line styles) is not in this file and cannot be changed here.
