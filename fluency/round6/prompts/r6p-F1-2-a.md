You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 8 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. `size` in the front matter is the slide's width and height in points. Slides follow, separated by a line holding only `---`. The first line of every slide is `layout: Name`.

### Objects

A slide is a list of objects written back to front: an object written later is drawn on top. Every object shows where it is: `box="x y w h"` is its left edge, top edge, width and height in points from the slide's top-left corner; `rot` turns it, `flip` mirrors it. Leave ids, names and boxes as they are.

- **Slots** are the layout's placeholders: a marker line `::title box="…"::`, then the slot's text, one line per paragraph (`- ` for a bullet), up to the next object or `---`.
- **Shapes** are one line each, `<shape id="s4" name="…" box="…">text</shape>`, or `<shape … />` without text; `<p/>` starts the shape's next paragraph.
- **Lines and connectors** are `<line id="…" name="…" from="x y" to="x y"/>`.
- **Groups**: a `<group …>` line, the group's objects, then `</group>`.
- **Pictures, charts, tables** are `<keep …/>` lines: kept for you, never changed here.
- Text: `**bold**`, `*italic*`, `<u>underline</u>`, `<br/>` a line break. Speaker notes follow `::notes::`.

### Formatting

Every object shows how it looks: its formatting is written on its tag (a shape, a line) or its slot marker, whether the object sets it itself or takes it from the theme or the layout. A property that is not written is not there: no `fill` means no fill, no `border` means no outline.

- Text properties shared by all of an object's text are on its tag or marker; a paragraph's own are in `{…}` at its end (before `<p/>` or `</shape>` in a shape, at the end of the line in a slot); `[text]{…}` gives a stretch of text its own `size`, `color` or `font`.
- To change how an object looks, change or add the property on its tag.

Example: `<shape id="s3" name="Card" box="64 130 260 320" fill=accent1 border="2pt solid accent1-50%" size=18pt color=#FFFFFF>**Cities** {size=24pt}<p/>Low-emission [zones]{color=#FF6B5B} in 40 cities</shape>`

### The vocabulary

`key=value` pairs separated by spaces, on the object's tag or slot marker; a value with a space is quoted. Lengths are points (`12pt`).

- colours: `#RRGGBB`, or a theme colour `accent1` … `accent6`, `tx1`, `bg1`, `tx2`, `bg2`, `hlink`; `accent1+40%` is 40% lighter and `accent1-25%` 25% darker; `accent1*` is the theme colour with another adjustment the file keeps while it is left as written; `/55%` after a colour is its opacity
- shape and line: `fill` (a colour, or `none`), `border` (the outline: `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`)
- text: `font`, `size`, `color`, `bold`; paragraph: `align` (left, center, right, justify), `indent-left`, `first-line`, `space-before`, `space-after`, `line-spacing` (`90%`, `14pt`)
- `fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written, and can be replaced by a colour; they cannot be written or changed. Shadows, glow, 3-D and other effects cannot be expressed.

## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
---
type: presentation
format: pptx
schema: 1
size: 960 x 540 pt
---

layout: Blank
<shape id="s2" name="Rectangle 1" box="0 0 320 540" fill=gradient border=none/>
<shape id="s3" name="TextBox 2" box="360 180 560 80" font="맑은 고딕" size=44pt color=#111827>**2026 하반기 사업 보고**</shape>
<shape id="s4" name="TextBox 3" box="360 270 560 40" font="맑은 고딕" size=20pt color=#6B7280>전략기획실 · 2026년 9월</shape>
<group id="g9" name="로고" box="40 40 220 32">
<group id="g7" name="Group 6" box="40 40 50 30">
<shape id="s5" name="Oval 4" box="40 40 30 30" fill=#FFFFFF border=none/>
<shape id="s6" name="Oval 5" box="60 40 30 30" fill=#93C5FD border=none/>
</group>
<shape id="s8" name="TextBox 7" box="100 42 160 30" font="맑은 고딕" size=16pt color=#FFFFFF>**한빛모빌리티**</shape>
</group>

---

layout: Title Only
::title box="36 22 648 90" align=center font=Calibri size=44pt color=tx1::
핵심 지표
<shape id="s3" name="Rounded Rectangle 2" box="40 150 205 150" fill=#E5E7EB border=none>**1,240억** {size=36pt color=#2563EB}<p/>매출 {size=16pt color=#111827}</shape>
<shape id="s4" name="Rounded Rectangle 3" box="265 150 205 150" fill=#E5E7EB border=none>**+12%** {size=36pt color=#2563EB}<p/>전년 대비 {size=16pt color=#111827}</shape>
<shape id="s5" name="Rounded Rectangle 4" box="490 150 205 150" fill=#E5E7EB border=none>**34곳** {size=36pt color=#2563EB}<p/>신규 고객 {size=16pt color=#111827}</shape>
<shape id="s6" name="Rounded Rectangle 5" box="715 150 205 150" fill=#E5E7EB border=none>**4.7** {size=36pt color=#2563EB}<p/>고객 만족도 {size=16pt color=#111827}</shape>
<shape id="s7" name="TextBox 6" box="40 330 880 60" font=Pretendard size=12pt color=#6B7280>출처: 내부 집계 (2026년 9월 말 기준)</shape>

---

layout: Title Only
::title box="36 22 648 90" align=center font=Calibri size=44pt color=tx1::
추진 절차
<shape id="s3" name="Pentagon 2" box="40 200 190 80" fill=accent2 border=none size=20pt color=#FFFFFF>기획</shape>
<shape id="s4" name="Chevron 3" box="220 200 190 80" fill=accent1 border=none size=20pt color=#FFFFFF>설계</shape>
<shape id="s5" name="Chevron 4" box="400 200 190 80" fill=accent2 border=none size=20pt color=#FFFFFF>구축</shape>
<shape id="s6" name="Chevron 5" box="580 200 190 80" fill=accent1 border=none size=20pt color=#FFFFFF>검증</shape>
<shape id="s7" name="Chevron 6" box="760 200 190 80" fill=accent2 border=none size=20pt color=#FFFFFF>확산</shape>
<shape id="s8" name="Rectangular Callout 7" box="560 330 240 80" fill=#FEF3C7 border="1.5pt solid #F59E0B" size=16pt color=#111827>10월 착수 예정</shape>

---

layout: Two Content
::title box="36 22 648 90" align=center font=Calibri size=44pt color=tx1::
지역별 현황
::left box="36 126 318 356" indent-left=27pt first-line=-27pt font=Calibri size=28pt color=tx1::
- 수도권 21곳
- 신규 9곳
::right box="366 126 318 356" indent-left=27pt first-line=-27pt font=Calibri size=28pt color=tx1::
- 지방 13곳
- 신규 4곳
<shape id="s5" name="Oval 4" box="455 250 50 50" fill=#2563EB border=none size=14pt color=#FFFFFF>VS</shape>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="40 30 600 50" font="맑은 고딕" size=32pt color=#111827>**조직 구성**</shape>
<shape id="s3" name="Rounded Rectangle 2" box="400 100 160 50" fill=#2563EB border=none size=16pt color=#FFFFFF>대표이사</shape>
<shape id="s4" name="Rounded Rectangle 3" box="120 230 160 50" fill=#FFFFFF border="1.5pt solid #2563EB" size=16pt color=#111827>전략기획실</shape>
<line id="s5" name="Connector 4" from="480 150" to="200 230" border="2pt solid accent1"/>
<shape id="s6" name="Rounded Rectangle 5" box="400 230 160 50" fill=#FFFFFF border="1.5pt solid #2563EB" size=16pt color=#111827>사업본부</shape>
<line id="s7" name="Connector 6" from="480 150" to="480 230" border="2pt solid accent1"/>
<shape id="s8" name="Rounded Rectangle 7" box="680 230 160 50" fill=#FFFFFF border="1.5pt solid #2563EB" size=16pt color=#111827>기술연구소</shape>
<line id="s9" name="Connector 8" from="480 150" to="760 230" border="2pt solid accent1"/>
<keep id="kh44k" kind="picture" summary="image.jpg" box="820 400 90 90"/>

---

layout: Title Only
::title box="36 22 648 90" align=center font=Calibri size=44pt color=tx1::
매출 구성
<keep id="kthd3" kind="chart" summary="Chart 2" box="60 130 360 360"/>
<shape id="s4" name="Rectangle 3" box="500 200 20 20" fill=#2563EB border=none/>
<shape id="s5" name="TextBox 4" box="530 195 300 30" font="맑은 고딕" size=18pt color=#111827>구독 55%</shape>
<shape id="s6" name="Rectangle 5" box="500 250 20 20" fill=#F59E0B border=none/>
<shape id="s7" name="TextBox 6" box="530 245 300 30" font="맑은 고딕" size=18pt color=#111827>라이선스 30%</shape>
<shape id="s8" name="Rectangle 7" box="500 300 20 20" fill=#10B981 border=none/>
<shape id="s9" name="TextBox 8" box="530 295 300 30" font="맑은 고딕" size=18pt color=#111827>서비스 15%</shape>

=== FILE END ===

## Tasks

1. `pp-q1` (read): On the 조직 구성 slide, what is the outline (width, line style and colour) of the connectors? Answer `ANSWER: border="<width> <style> <colour>"`.
2. `pp-q2` (read): On the 추진 절차 slide, which shapes are filled accent2? Answer `ANSWER: <name>; <name>`.
3. `pp-e1` (edit): Change the fill of the card "Rounded Rectangle 3" (+12%) to accent2.
4. `pp-e2` (edit): Give the shape "Oval 4" (VS) a 2pt navy outline.
5. `pp-e3` (edit): In "10월 착수 예정", make "10월" 24pt and coral.
6. `pp-e4` (edit): Make the three connectors on the 조직 구성 slide 3pt wide, keeping their colour and line style.
7. `pp-e5` (edit): Change "신규 4곳" to "신규 5곳", keeping its formatting exactly as it is.
8. `pp-e6` (edit): Make the gradient of the left panel on the first slide run from navy to teal.

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Edit tasks: answer with exact text edits. Each edit is `{"old": "...", "new": "..."}`: `old` is copied exactly from the file (same spaces and characters) and occurs exactly once in it, and `new` replaces it. Edits apply in order. Change only what the task asks for; every other line must stay exactly as it is.
- If a task cannot be done in this file format, do not edit: answer `REFUSE: <one sentence why>`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "…", "text": "ANSWER: …"},
  {"task_id": "…", "edits": [{"old": "…", "new": "…"}]},
  {"task_id": "…", "text": "REFUSE: …"}
]}
```

One entry per task, in the order of the tasks.
