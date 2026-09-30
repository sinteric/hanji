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

An object shows the formatting it sets itself, on its tag (a shape, a line) or its slot marker. What it takes from elsewhere, the theme's shape style or the layout's text styles, is not written here: a property that is not written comes from there, and may be none.

- Text properties the object sets for all of its text are on its tag or marker; a paragraph's own are in `{…}` at its end (before `<p/>` or `</shape>` in a shape, at the end of the line in a slot); `[text]{…}` gives a stretch of text its own `size`, `color` or `font`.
- To change how an object looks, write the property on its tag: the object then sets it itself.

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
<keep id="k63h9" kind="picture" summary="image.jpg" box="0 0 960 540"/>
<shape id="s3" name="Rectangle 2" box="0 0 960 540" fill=#101B3A/55% border=none/>
<shape id="s4" name="TextBox 3" box="64 200 700 90" fill=none font="Pretendard ExtraBold" size=54pt color=#FFFFFF>**Northwind Mobility**</shape>
<shape id="s5" name="TextBox 4" box="64 290 600 40" fill=none font=Pretendard size=22pt color=#CFE8FF>Series A · 2026</shape>
<group id="g8" name="Logo" box="64 64 134 36">
<shape id="s6" name="Oval 5" box="64 64 36 36" fill=#14B8A6 border=none/>
<shape id="s7" name="Rectangle 6" box="108 72 90 20" fill=#FFFFFF border=none/>
</group>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="64 40 600 60" fill=none font=Pretendard size=40pt color=#101B3A>**Agenda**</shape>
<shape id="s3" name="Oval 2" box="100 220 72 72" fill=accent1 border=none size=28pt color=#FFFFFF>1</shape>
<shape id="s4" name="TextBox 3" box="70 310 132 40" fill=none align=center font=Pretendard size=18pt color=#222222>Problem</shape>
<shape id="s5" name="Oval 4" box="300 220 72 72" fill=accent1 border=none size=28pt color=#FFFFFF>2</shape>
<shape id="s6" name="TextBox 5" box="270 310 132 40" fill=none align=center font=Pretendard size=18pt color=#222222>Solution</shape>
<line id="s7" name="Connector 6" from="172 256" to="300 256"/>
<shape id="s8" name="Oval 7" box="500 220 72 72" fill=accent1 border=none size=28pt color=#FFFFFF>3</shape>
<shape id="s9" name="TextBox 8" box="470 310 132 40" fill=none align=center font=Pretendard size=18pt color=#222222>Market</shape>
<line id="s10" name="Connector 9" from="372 256" to="500 256"/>
<shape id="s11" name="Oval 10" box="700 220 72 72" fill=accent1 border=none size=28pt color=#FFFFFF>4</shape>
<shape id="s12" name="TextBox 11" box="670 310 132 40" fill=none align=center font=Pretendard size=18pt color=#222222>Traction</shape>
<line id="s13" name="Connector 12" from="572 256" to="700 256"/>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="64 40 800 60" fill=none font=Pretendard size=40pt color=#101B3A>**Why now**</shape>
<group id="g7" name="Card 1" box="64 130 260 320">
<shape id="s3" name="Rounded Rectangle 2" box="64 130 260 320" fill=#FFFFFF border=none/>
<shape id="s4" name="Freeform 3" box="88 154 44 44" fill=#FF6B5B border=none/>
<shape id="s5" name="TextBox 4" box="88 214 212 40" fill=none font=Pretendard size=24pt color=#101B3A>**Cities**</shape>
<shape id="s6" name="TextBox 5" box="88 260 212 120" fill=none font=Pretendard size=16pt color=#555555>Low-emission zones in 40 cities</shape>
</group>
<group id="g12" name="Card 2" box="354 130 260 320">
<shape id="s8" name="Rounded Rectangle 7" box="354 130 260 320" fill=#FFFFFF border=none/>
<shape id="s9" name="Freeform 8" box="378 154 44 44" fill=#FF6B5B border=none/>
<shape id="s10" name="TextBox 9" box="378 214 212 40" fill=none font=Pretendard size=24pt color=#101B3A>**Costs**</shape>
<shape id="s11" name="TextBox 10" box="378 260 212 120" fill=none font=Pretendard size=16pt color=#555555>Battery cost down 60% in 5 years</shape>
</group>
<group id="g17" name="Card 3" box="644 130 260 320">
<shape id="s13" name="Rounded Rectangle 12" box="644 130 260 320" fill=#FFFFFF border=none/>
<shape id="s14" name="Freeform 13" box="668 154 44 44" fill=#FF6B5B border=none/>
<shape id="s15" name="TextBox 14" box="668 214 212 40" fill=none font=Pretendard size=24pt color=#101B3A>**People**</shape>
<shape id="s16" name="TextBox 15" box="668 260 212 120" fill=none font=Pretendard size=16pt color=#555555>Two thirds want fewer cars</shape>
</group>

---

layout: Blank
<shape id="s2" name="Rectangle 1" box="0 0 960 180" fill=gradient border=none/>
<shape id="s3" name="TextBox 2" box="64 60 800 60" fill=none font=Pretendard size=40pt color=#FFFFFF>**Traction**</shape>
<shape id="s4" name="TextBox 3" box="64 220 260 100" fill=none font=Pretendard size=72pt color=#FF6B5B>**12k**</shape>
<shape id="s5" name="TextBox 4" box="64 320 260 40" fill=none font=Pretendard size=18pt color=#222222>riders / day</shape>
<shape id="s6" name="Chevron 5" box="264 420 40 40" rot="270" fill=#101B3A border=none/>
<shape id="s7" name="TextBox 6" box="364 220 260 100" fill=none font=Pretendard size=72pt color=#FF6B5B>**3.4x**</shape>
<shape id="s8" name="TextBox 7" box="364 320 260 40" fill=none font=Pretendard size=18pt color=#222222>YoY growth</shape>
<shape id="s9" name="Chevron 8" box="564 420 40 40" rot="270" fill=#101B3A border=none/>
<shape id="s10" name="TextBox 9" box="664 220 260 100" fill=none font=Pretendard size=72pt color=#FF6B5B>**92%**</shape>
<shape id="s11" name="TextBox 10" box="664 320 260 40" fill=none font=Pretendard size=18pt color=#222222>retention</shape>
<shape id="s12" name="Chevron 11" box="864 420 40 40" rot="270" fill=#101B3A border=none/>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="64 40 800 60" fill=none font=Pretendard size=40pt color=#101B3A>**In the field**</shape>
<keep id="k12ki" kind="picture" summary="image.jpg" box="64 120 400 300"/>
<keep id="kf30k" kind="picture" summary="image.jpg" box="500 120 180 180"/>
<keep id="krg9t" kind="picture" summary="image.jpg" box="710 120 200 150"/>
<shape id="s6" name="TextBox 5" box="64 430 400 30" fill=none font=Pretendard size=14pt color=#777777>Depot, Incheon</shape>
<shape id="s7" name="TextBox 6" box="500 310 180 30" fill=none align=center font=Pretendard size=14pt color=#777777>Driver onboarding</shape>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="64 40 800 60" fill=none font=Pretendard size=40pt color=#101B3A>**Roadmap**</shape>
<line id="s3" name="Connector 2" from="80 280" to="880 280" border="3pt solid #101B3A"/>
<shape id="s4" name="Diamond 3" box="110 265 30 30" fill=#14B8A6 border=none/>
<shape id="s5" name="TextBox 4" box="75 220 100 30" fill=none align=center font=Pretendard size=16pt color=#101B3A>**Q1**</shape>
<shape id="s6" name="TextBox 5" box="75 305 100 30" fill=none align=center font=Pretendard size=14pt color=#222222>Pilot</shape>
<shape id="s7" name="Diamond 6" box="280 265 30 30" fill=#14B8A6 border=none/>
<shape id="s8" name="TextBox 7" box="245 220 100 30" fill=none align=center font=Pretendard size=16pt color=#101B3A>**Q2**</shape>
<shape id="s9" name="TextBox 8" box="245 305 100 30" fill=none align=center font=Pretendard size=14pt color=#222222>Seoul</shape>
<shape id="s10" name="Diamond 9" box="450 265 30 30" fill=#14B8A6 border=none/>
<shape id="s11" name="TextBox 10" box="415 220 100 30" fill=none align=center font=Pretendard size=16pt color=#101B3A>**Q3**</shape>
<shape id="s12" name="TextBox 11" box="415 305 100 30" fill=none align=center font=Pretendard size=14pt color=#222222>Busan</shape>
<shape id="s13" name="Diamond 12" box="620 265 30 30" fill=#14B8A6 border=none/>
<shape id="s14" name="TextBox 13" box="585 220 100 30" fill=none align=center font=Pretendard size=16pt color=#101B3A>**Q4**</shape>
<shape id="s15" name="TextBox 14" box="585 305 100 30" fill=none align=center font=Pretendard size=14pt color=#222222>Tokyo</shape>
<shape id="s16" name="Diamond 15" box="790 265 30 30" fill=#FF6B5B border=none/>
<shape id="s17" name="TextBox 16" box="755 220 100 30" fill=none align=center font=Pretendard size=16pt color=#101B3A>**2027**</shape>
<shape id="s18" name="TextBox 17" box="755 305 100 30" fill=none align=center font=Pretendard size=14pt color=#222222>IPO?</shape>
<shape id="s19" name="Curved Right Arrow 18" box="820 360 60 90" flip="h" fill=#FF6B5B border=none/>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="64 40 800 60" fill=none font=Pretendard size=40pt color=#101B3A>**Unit economics**</shape>
<keep id="kthd3" kind="chart" summary="Chart 2" box="64 120 480 340"/>
<keep id="kc83c" kind="table" summary="Table 3: Metric Value CAC $18 LTV $240 Payback 4 mo" box="580 140 320 160"/>

---

layout: Blank
<shape id="s2" name="TextBox 1" box="40 40 160 160" rot="8" fill=none font=Georgia size=200pt color=#14B8A6>**“**</shape>
<shape id="s3" name="TextBox 2" box="140 170 700 160" fill=none font="Pretendard Light" size=36pt color=#FFFFFF>We moved 30% of our staff out of cars in six months.</shape>
<shape id="s4" name="TextBox 3" box="140 350 600 30" fill=none font=Pretendard size=16pt color=#CFE8FF>— Head of Operations, a pilot customer</shape>

=== FILE END ===

## Tasks

1. `pp-q1` (read): On the Agenda slide, what is the outline (width, line style and colour) of the connector "Connector 6"? Answer `ANSWER: border="<width> <style> <colour>"`.
2. `pp-q2` (read): On the Agenda slide, which shapes are filled accent1? Answer `ANSWER: <name>; <name>`.
3. `pp-e1` (edit): Change the fill of Card 2's background, "Rounded Rectangle 7", to accent2.
4. `pp-e2` (edit): Give Card 3's background, "Rounded Rectangle 12", a 2pt navy outline.
5. `pp-e3` (edit): In the text "Battery cost down 60% in 5 years", make "60%" 24pt and coral.
6. `pp-e4` (edit): Make the three connectors on the Agenda slide 3pt wide, keeping their colour and line style.
7. `pp-e5` (edit): Change "Series A · 2026" to "Series B · 2026", keeping its formatting exactly as it is.
8. `pp-e6` (edit): Give Card 1's background, "Rounded Rectangle 2", a soft drop shadow.

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
