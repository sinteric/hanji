# Syntax guides given to the subject model. Within a decision the shared part is identical for A and B;
# the candidate parts are written to the same length and level of detail and never mention each other.

DOC_BASICS = """\
A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes."""

SLIDE_BASICS = """\
A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes."""

GUIDES = {}

GUIDES['merge', 'A'] = DOC_BASICS + """

### Tables

A simple table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row.

A table with any merged cell is written as `<table>` … `</table>` with one `<tr>` per line. Header cells are `<th>`, the others `<td>`.

- `rowspan="N"`: the cell extends down over N rows. In the rows below, that cell is left out.
- `colspan="N"`: the cell extends right over N columns.
- A cell may have both. `<table>` and `<tr>` take no attributes, and cells take no others.

Every row covers the same number of columns as the header: the cells written, plus their colspans, plus the cells still covered by a rowspan from above.

Example ("서울" covers two rows, "합계" covers two columns):

```
<table>
<tr><th>지역</th><th>지점</th><th>매출</th></tr>
<tr><td rowspan="2">서울</td><td>강남</td><td>120</td></tr>
<tr><td>종로</td><td>95</td></tr>
<tr><td colspan="2">합계</td><td>215</td></tr>
</table>
```"""

GUIDES['merge', 'B'] = DOC_BASICS + """

### Tables

Every table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row. Every row has one cell per column, merged or not.

Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column.
- A merged area is a rectangle. For two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. An empty cell that is not merged is written with a space, `|  |`.
- The text of a merged cell is written once, in its top-left cell; the covered cells hold only a marker, never text.

Example ("서울" covers two rows, "합계" covers two columns):

```
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```"""

_STYLE = """

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts and sizes cannot be written directly.

- A paragraph with a style is `<div ATTR="Name">text</div>`, on one line.
- A table with a style is `<table ATTR="Name">`, then one `<tr>` per line with `<th>` header cells and `<td>` cells, then `</table>`.
- A paragraph or a pipe table without a tag has the default style.

The value of `ATTR` is exactly one style name, written as listed, spaces included. `<div>` and `<table>` take no other attribute.

Example:

```
<div ATTR="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table ATTR="Grid Table 4">
<tr><th>지역</th><th>매출</th></tr>
<tr><td>수도권</td><td>1,204</td></tr>
</table>
```"""

GUIDES['styleattr', 'A'] = DOC_BASICS + _STYLE.replace('ATTR', 'style')
GUIDES['styleattr', 'B'] = DOC_BASICS + _STYLE.replace('ATTR', 'class')

GUIDES['slides', 'A'] = SLIDE_BASICS + """

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
```"""

GUIDES['slides', 'B'] = SLIDE_BASICS + """

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
```"""
