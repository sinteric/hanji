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

Every object shows where it is. `box="x y w h"` is its left edge, top edge, width and height, in percent of the slide from its top-left corner: x and w are percent of the slide's width, y and h percent of its height (see `size`); x grows to the right, y downwards. Numbers are shown rounded to one decimal; a number you leave as it is keeps its exact value, a number you change is used as written. `rot="15"` turns an object 15 degrees clockwise about its centre (its box is the unturned one); `flip="h"` or `flip="v"` mirrors it.

- **Slot:** a marker line `::title box="5 4 90 16.7"::`, then its text on the lines after it, up to the next object or `---`. The box shown is where the slot is; it is the layout's box unless someone moved the slot. Leave it as it is to keep the slot there; in a new slide write the marker without a box, `::title::`, and the slot sits where its layout puts it.
- **Shape:** a marker line `::shape id="s5" name="출처" box="5 88 40 5.4"::`, then its text on the lines after it (none for a shape without text).
- **Picture, chart, table:** `<keep id="k2" kind="picture" summary="map.png" box="55.6 55.6 27.8 27.8"/>`.
- **Line:** `<line id="s6" name="Arrow 5" from="11.7 26.7" to="35 26.7"/>`, its two ends as `x y`.
- **Group:** `<group id="g5" name="Group 4" box="16.7 20 35.8 28.1">`; its objects' boxes are on the slide, like any other. The group's box is the box around its objects: change it to move or resize the whole group, or change the objects themselves, not both.
- **New objects:** a text box is `::shape box="…"::` followed by its text; a picture from a file is `<keep kind="picture" src="images/map.png" box="…"/>`; a line is `<line from="…" to="…"/>`.

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

### Changing a deck

- Change text by editing it. Delete an object by deleting its line(s); delete a slide by deleting it and one of the `---` lines around it.
- A new slide is written like any other, from one of the listed layouts.
- Objects you add have no `id`; ids are given when the file is saved. Never invent an id.
- Formatting (colours, fills, fonts, font sizes, line styles) is not in this file and cannot be changed here.


## Names available in this file

Layouts and their slots, each with the box the layout gives it (every layout also has `notes`):
- `Title Slide`: title (12.5 16.4 75 34.8), subtitle (12.5 52.5 75 24.1)
- `Title and Content`: title (6.9 5.3 86.3 19.3), body (6.9 26.6 86.3 63.4)
- `Section Header`: title (6.8 24.9 86.3 41.6), body (6.8 66.9 86.3 21.9)
- `Two Content`: title (6.9 5.3 86.3 19.3), left (6.9 26.6 42.5 63.4), right (50.6 26.6 42.5 63.4)
- `Comparison`: title (6.9 5.3 86.3 19.3), body (6.9 24.5 42.3 12), body2 (6.9 36.5 42.3 53.7), body3 (50.6 24.5 42.5 12), body4 (50.6 36.5 42.5 53.7)
- `Title Only`: title (6.9 5.3 86.3 19.3)
- `Blank`: no slots
- `Content with Caption`: title (6.9 6.7 32.3 23.3), right (42.5 14.4 50.6 71.1), left (6.9 30 32.3 55.6)
- `Picture with Caption`: title (6.9 6.7 32.3 23.3), picture (42.5 14.4 50.6 71.1), body (6.9 30 32.3 55.6)

## The file: product-deck.hj.md

````
---
type: presentation
format: pptx
template: org/product-deck
schema: 1
size: 960 x 540 pt
---

layout: Title Slide
::title box="12.5 16.4 75 34.8"::
2027 신제품 출시 계획
::subtitle box="12.5 52.5 75 24.1"::
마케팅본부 · 2026년 11월
<keep id="k3ftw" kind="picture" summary="logo.png" box="90 3.3 8.1 7.2"/>
::notes::
일정과 수치는 내부 검토용

---

layout: Title and Content
::title box="6.9 5.3 78.8 19.3"::
시장 현황
::body box="6.9 26.6 86.3 63.4"::
- 국내 스마트홈 시장 4.2조 원 (전년 대비 +12%)
- 주요 경쟁사 3곳 신제품 출시
  - A사: 2027년 3월
  - B사: 2027년 5월
- 1인 가구 비중 34%
::shape id="s4" name="NEW 배지" box="77.5 23.3 9.4 6.7" rot="15"::
NEW
<keep id="k7hqa" kind="picture" summary="logo.png" box="90 3.3 8.1 7.2"/>

---

layout: Title Only
::title box="6.9 5.3 86.3 19.3"::
출시 일정
<group id="g5" name="타임라인" box="10 33.3 75 13.3">
::shape id="s6" name="기획 단계" box="10 33.3 20 13.3"::
기획
<line id="s7" name="화살표 1" from="30 40" to="37.5 40"/>
::shape id="s8" name="개발 단계" box="37.5 33.3 20 13.3"::
개발
<line id="s9" name="화살표 2" from="57.5 40" to="65 40"/>
::shape id="s10" name="출시 단계" box="65 33.3 20 13.3"::
출시
</group>
::shape id="s11" name="주석" box="10 66.7 50 5.4"::
일정은 내부 검토 중이며 변동될 수 있음
::notes::
각 단계는 분기 단위

---

layout: Two Content
::title box="6.9 5.3 86.3 19.3"::
제품 비교
::left box="6.9 26.6 42.5 63.4"::
- 스마트 허브 S1
  - 가격 19만 9천 원
  - 음성 인식 지원
::right box="50.6 26.6 42.5 63.4"::
- 스마트 허브 S1 Pro
  - 가격 29만 9천 원
  - 음성 인식 지원
  - 카메라 내장
::notes::
가격은 부가세 포함

---

layout: Title Only
::title box="6.9 5.3 86.3 19.3"::
예상 매출
<keep id="k2m8c" kind="chart" summary="Chart 3: 분기별 예상 매출 (억 원)" box="12.5 31.1 50 55.6"/>
::shape id="s13" name="1분기 라벨" box="65.6 33.3 18.8 5.4"::
1분기 120억
::shape id="s14" name="2분기 라벨" box="66.7 43.3 18.8 5.4"::
2분기 180억

---

layout: Section Header
::title box="6.8 24.9 86.3 41.6"::
부록
::body box="6.8 66.9 86.3 21.9"::
세부 자료
````

## Tasks

A read task is answered with `text`: `"ANSWER: <objects, comma-separated>"`, naming each object by its id, or a slot by its slot name (e.g. `ANSWER: title, s4`), or `ANSWER: none` if there is none. A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### deck3-x1 (read)

On slide 2 (시장 현황), which objects are partly or wholly within 2 cm of the slide's right edge?

### deck3-x2 (edit)

On slide 4 (제품 비교), widen the right column so that its right edge is 1 cm from the slide's right edge, keeping its other three edges where they are.

### deck3-x3 (edit)

On slide 2 (시장 현황), send the shape "NEW 배지" behind the body, so that the body is drawn on top of it. Change nothing else.

### deck3-x4 (edit)

On slide 3 (출시 일정), centre the timeline group horizontally on the slide. Keep every size and vertical position.

### deck3-x5 (edit)

On slide 5 (예상 매출), put the two label text boxes side by side below the chart: "1분기 라벨" with its left edge on the chart's left edge and its top edge 0.5 cm below the chart's bottom edge; "2분기 라벨" to its right, with a gap of 0.5 cm between them and the same top edge. Keep both sizes.

### deck3-x6 (edit)

On slide 2 (시장 현황), turn the shape "NEW 배지" upright (no rotation) and move it so that its top-right corner is on the body's top-right corner. Keep its size.

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
