You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and ten tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

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
- `Title Slide`: title (7.5 31.1 85 21.4), subtitle (15 56.7 70 25.6)
- `Title and Content`: title (5 4 90 16.7), body (5 23.3 90 66)
- `Section Header`: title (7.9 64.3 85 19.9), body (7.9 42.4 85 21.9)
- `Two Content`: title (5 4 90 16.7), left (5 23.3 44.2 66), right (50.8 23.3 44.2 66)
- `Comparison`: title (5 4 90 16.7), body (5 22.4 44.2 9.3), body2 (5 31.7 44.2 57.6), body3 (50.8 22.4 44.2 9.3), body4 (50.8 31.7 44.2 57.6)
- `Title Only`: title (5 4 90 16.7)
- `Blank`: no slots
- `Content with Caption`: title (5 4 32.9 16.9), right (39.1 4 55.9 85.3), left (5 20.9 32.9 68.4)
- `Picture with Caption`: title (19.6 70 60 8.3), picture (19.6 8.9 60 60), body (19.6 78.3 60 11.7)
- `Title and Vertical Text`: title (5 4 90 16.7), body (5 23.3 90 66)
- `Vertical Title and Text`: title (72.5 4 22.5 85.3), body (5 4 65.8 85.3)

## The file: korean-deck.hj.md

````
---
type: presentation
format: pptx
schema: 1
size: 720 x 540 pt
---

layout: Title Slide
::title box="7.5 31.1 85 21.4"::
3분기 영업 보고
::subtitle box="15 56.7 70 25.6"::
영업본부 · 2026년 10월
::notes::
인사말 후 목차를 소개한다.

---

layout: Title and Content
::title box="5 4 90 16.7"::
핵심 지표
::body box="5 23.3 90 66"::
- 매출 **12% 증가**
- 신규 고객 34곳
  - 수도권 21곳
  - 지방 13곳
- 영업이익률 8.4%
::notes::
전년 대비 증가폭을 강조한다.

---

layout: Two Content
::title box="5 4 90 16.7"::
지역별 현황
::left box="5 23.3 44.2 66"::
- 수도권 21곳
  - 서울 14곳
::right box="50.8 23.3 44.2 66"::
- 지방 13곳
  - 부산 5곳
::shape id="s5" name="출처" box="5 88 40 5.3"::
출처: 내부 집계

---

layout: Title Only
::title box="5 4 90 16.7"::
분기별 매출
<keep id="kb2br" kind="table" summary="Table 2: 분기 매출 증감 1분기 1,120 +3% 2분기 1,180 +5% 3분기 1,204 +12%" box="10 26.7 80 26.7"/>
<keep id="k04k9" kind="picture" summary="image.png" box="80 73.3 10 6.7"/>
::notes::
표는 잠정치이며 10월 말 확정된다.

---

layout: Title and Content
::title box="5 4 90 16.7"::
다음 분기 계획
::body box="5 23.3 90 66"::
1. 신규 지점 3곳 개설
1. 온라인 채널 확대
1. 고객 만족도 조사
- 자세한 일정은 사내 게시판 참고 (긴급)

---

layout: Section Header
::title box="7.9 64.3 85 19.9"::
부록
::body box="7.9 42.4 85 21.9"::
세부 자료

---

layout: Title and Content
::title box="5 4 90 16.7"::
감사합니다
````

## Tasks

A read task is answered with `text`: `"ANSWER: <objects, comma-separated>"`, naming each object by its id, or a slot by its slot name (e.g. `ANSWER: title, s4`), or `ANSWER: none` if there is none. A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### deck1-q1 (read)

On slide 3 (지역별 현황), which objects' boxes overlap the box of the text box 출처?

### deck1-q2 (read)

On slide 4 (분기별 매출), which objects lie entirely inside the bottom-right quarter of the slide?

### deck1-e1 (edit)

On slide 3 (지역별 현황), move the text box 출처 to the right so that its right edge lines up with the right edge of the right column. Keep its size and its vertical position.

### deck1-e2 (edit)

On slide 4 (분기별 매출), make the picture twice as wide and twice as tall, keeping its bottom-right corner where it is.

### deck1-e3 (edit)

On slide 4 (분기별 매출), add a text box with the text "단위: 억 원" directly under the title: its top edge on the title's bottom edge, its left and right edges on the title's, and between 0.7 cm and 1.1 cm tall. It must not overlap any other object.

### deck1-e4 (edit)

On slide 4 (분기별 매출), move the picture to the left so that its left edge lines up with the table's left edge. Keep its size and its vertical position.

### deck1-e5 (edit)

On slide 2 (핵심 지표), change the bullet "지방 13곳" to "지방 14곳". Change nothing else: every position and size stays as it is.

### deck1-e6 (edit)

Right after slide 3 (지역별 현황), add a slide with the Comparison layout: title "권역별 전략"; left heading "수도권", with the bullets "신규 지점 2곳" and "온라인 판촉 강화" under it; right heading "지방", with the bullets "부산 거점 확대" and "대리점 교육" under it. Leave every placeholder where the layout puts it.

### deck1-e7 (edit)

On slide 4 (분기별 매출), change the 3분기 figure in the table from 1,204 to 1,210.

### deck1-w (write)

Write a new presentation with exactly these three slides, in this order, using the layouts listed above:
1. Layout Title Slide: title "2026년 4분기 영업 계획"; subtitle "영업기획팀 · 2026년 10월".
2. Layout Title and Content: title "4분기 목표"; body bullets "매출 1,300억 원" and "신규 고객 40곳"; speaker notes "목표는 10월 경영회의에서 확정".
3. Layout Two Content: title "채널별 과제"; left column bullets "오프라인: 매장 판촉" and "오프라인: 대리점 교육"; right column bullets "온라인: 기획전" and "온라인: 멤버십 캠페인".
Every placeholder stays where its layout puts it.

Start the new file with this front matter:

````
---
type: presentation
format: pptx
schema: 1
size: 720 x 540 pt
---
````

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
