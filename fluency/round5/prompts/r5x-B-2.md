You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and six tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

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


## Names available in this file

Layouts and their slots, each with the box the layout gives it (every layout also has `notes`):
- `Title Slide`: title (54 168 612 116), subtitle (108 306 504 138)
- `Title and Content`: title (36 22 648 90), body (36 126 648 356)
- `Section Header`: title (57 347 612 107), body (57 229 612 118)
- `Two Content`: title (36 22 648 90), left (36 126 318 356), right (366 126 318 356)
- `Comparison`: title (36 22 648 90), body (36 121 318 50), body2 (36 171 318 311), body3 (366 121 318 50), body4 (366 171 318 311)
- `Title Only`: title (36 22 648 90)
- `Blank`: no slots
- `Content with Caption`: title (36 22 237 92), right (282 22 403 461), left (36 113 237 369)
- `Picture with Caption`: title (141 378 432 45), picture (141 48 432 324), body (141 423 432 63)
- `Title and Vertical Text`: title (36 22 648 90), body (36 126 648 356)
- `Vertical Title and Text`: title (522 22 162 461), body (36 22 474 461)

## The file: shapes.hj.md

````
---
type: presentation
format: pptx
schema: 1
size: 720 x 540 pt
---

layout: Blank
<shape id="s4" name="TextBox 3" box="72 72 180 29">Learning PPTX</shape>
<line id="s6" name="Straight Connector 5" from="84 144" to="252 144"/>
<shape id="s7" name="Freeform 6" box="47 211 185 136">Cloud</shape>
<keep id="kqld8" kind="picture" summary="Picture 1" box="402 78 144 132"/>
<keep id="kmtnf" kind="table" summary="Table 2: Column1 Column2 Column3 data1 data2 data3" box="300 372 372 96"/>
<line id="s8" name="Straight Arrow Connector 7" from="468 366" to="468 216"/>
<line id="s10" name="Elbow Connector 9" from="186 252" to="402 144"/>

---

layout: Title Slide
::title::
PPTX <u>Title</u>
::subtitle::
**Subtitle**
<p/>
And second line

---

layout: Blank
<group id="g5" name="Group 4" box="120 108 258 152">
<shape id="s2" name="Rectangle 1" box="120 108 138 60"/>
<shape id="s3" name="Oval 2" box="306 150 72 72"/>
<shape id="s4" name="Right Arrow 3" box="144 222 77 38"/>
</group>

---

layout: Blank
<keep id="k5mk0" kind="table" summary="Table 1: header1 header2 header3 A1 B1 C1 A2 A3 B3 and C3 are merged A4 A5" box="120 110 480 175"/>

---

layout: Title Only
<keep id="k4akl" kind="table" summary="Table 1: Link Type Target URI Web Page http://poi.apache.org/ Place in this docu…" box="54 168 618 146"/>
::title::
Hyperlinks

---

layout: Blank
<shape id="s2" name="Rectangle 1" box="66 72 72 72"/>
<shape id="s9" name="Rectangle 8" box="174 72 72 72"/>
<shape id="s10" name="Rectangle 9" box="270 72 72 72"/>
<shape id="s11" name="Rectangle 10" box="366 72 72 72"/>
<shape id="s12" name="Rectangle 11" box="468 72 72 72"/>
<shape id="s13" name="Rectangle 12" box="594 72 72 72"/>
````

## Tasks

A read task is answered with `text`: `"ANSWER: <objects, comma-separated>"`, naming each object by its id, or a slot by its slot name (e.g. `ANSWER: title, s4`), or `ANSWER: none` if there is none. A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### deck2-x1 (read)

On slide 6, which rectangle's centre is closest to the slide's horizontal centre?

### deck2-x2 (edit)

On slide 6, space the six rectangles evenly: keep the first and the last where they are, and move the four between them so that the gaps between neighbours are all equal. Keep every size and vertical position.

### deck2-x3 (edit)

On slide 1, bring the text box "TextBox 3" to the front, so that it is drawn on top of every other object. Change nothing else.

### deck2-x4 (edit)

On slide 3, move the whole group 2 cm to the right. Keep every size and vertical position.

### deck2-x5 (edit)

On slide 1, make the table 20% narrower, keeping its centre and its height.

### deck2-x6 (edit)

On slide 5 (Hyperlinks), make the title exactly as wide as the table, with its left edge on the table's left edge. Keep the title's top and height.

## Tasks that cannot be done

Do only what the syntax documentation and the names above can express. If an edit task asks for something they cannot express, do not approximate it, and do not invent a name, tag or attribute: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its answer.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a read task>", "text": "ANSWER: <objects, comma-separated>"},
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]},
  {"task_id": "<id of an edit task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```
