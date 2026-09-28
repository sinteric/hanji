A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes.

### Slides

- Slides are separated by a line containing only `---`. The first slide begins right after the front matter.
- The first line of every slide is `layout: Name`, written as listed, spaces included. There are no other `key: value` lines.
- Then each slot begins with a marker line named after the slot: `::title::`, `::body::`, `::left::`, `::right::`, and `::notes::` for speaker notes (every layout has notes). Its text is on the lines after the marker, up to the next marker or `---`.
- Use only the slots of the slide's layout and leave out a slot you do not fill. All text is inside a slot.

Example (front matter and two slides):

```
---
type: presentation
format: pptx
template: org/deck
schema: 1
---

layout: Title and Content
::title::
핵심 지표
::body::
- 매출 **12% 증가**
- 신규 고객 34곳
::notes::
전년 대비 강조

---

layout: Two Content
::title::
지역별 현황
::left::
- 수도권 21곳
::right::
- 지방 13곳
```
