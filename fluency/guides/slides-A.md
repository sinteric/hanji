A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes.

### Slides

- A slide is `<slide layout="Name">` … `</slide>`. The layout name is written as listed, spaces included.
- Inside, each slot is a tag named after the slot: `<title>`, `<body>`, `<left>`, `<right>`, and `<notes>` for speaker notes (every layout has notes). A slot's text is either on one line, `<title>핵심 지표</title>`, or on the lines between `<body>` and `</body>`.
- Use only the slots of the slide's layout and leave out a slot you do not fill. All text is inside a slot.

Example (front matter and two slides):

```
---
type: presentation
format: pptx
template: org/deck
schema: 1
---

<slide layout="Title and Content">
<title>핵심 지표</title>
<body>
- 매출 **12% 증가**
- 신규 고객 34곳
</body>
<notes>전년 대비 강조</notes>
</slide>

<slide layout="Two Content">
<title>지역별 현황</title>
<left>- 수도권 21곳</left>
<right>- 지방 13곳</right>
</slide>
```
