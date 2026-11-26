"""Round 7 syntax guides: a shared part per decision, identical for A and B, and one candidate part each.

The candidate parts have the same structure (what the tags are, where they attach, what cannot be written, an
example on the same content) and never mention each other. build.py asserts they are the same length.
"""

# ---------------------------------------------------------------- documents (breaks)

DOC_BASICS = """\
A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is \
Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items (a nested item is indented two spaces), \
`**bold**` and `*italic*`. A blank line only separates blocks; it is never a paragraph itself. Inside a paragraph, \
`<br/>` is a line break: the text goes on in the same paragraph. A line holding only `<pagebreak/>` is a page break \
between paragraphs. `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a chart); \
leave every one exactly as it is. Besides Markdown, only the tags and markers described below exist; there are no \
other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. A \
paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. \
A style name is written exactly as listed, spaces included.

### Tables

A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one \
cell per column. `^^` as the whole cell means the cell is merged into the one above; `||` (two pipes with nothing \
between) means the cell to the left extends into this column. An empty unmerged cell is `|  |`.
"""

BREAKS_A = """
### Breaks inside a paragraph

A page break or a column break can sit in the middle of a paragraph, between two stretches of that paragraph's \
text. `<pagebreak/>` starts a new page at that point; `<columnbreak/>` starts a new column at that point. The text \
before the tag and the text after it stay one paragraph.

- Write the tag where the break falls, with no space inside it: `앞 문장.<pagebreak/>뒤 문장`, \
`서울 14곳.<columnbreak/>부산 5곳`.
- `<pagebreak/>` and `<columnbreak/>` take no attribute. They are single tags; there is no `</pagebreak>` and no \
`</columnbreak>`.
- A line that holds only `<pagebreak/>` is still a page break *between* paragraphs, as above. A line that holds \
only `<columnbreak/>` is a paragraph whose only content is a column break.
- `<br/>` is still a line break inside the paragraph (the text continues on the next line of the same page and \
column). It takes no attribute.
- Nothing else can be expressed: a section break, an odd-page or even-page break, a change of column count, a \
change of margins. A request for one of those is refused.

Example (a page break in the middle of the 핵심 지표 paragraph, a column break in the 지역 줄, then a page break \
between the body and the 부록 heading):

```
수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.<pagebreak/>첨부 1의 지점별 표는 10월 말 확정치입니다.

서울 14곳, 경기 7곳.<columnbreak/>부산 5곳, 대구 3곳.

<pagebreak/>

# 부록
```
"""

BREAKS_B = """
### Breaks inside a paragraph

A page break or a column break can sit in the middle of a paragraph, between two stretches of that paragraph's \
text. `<br type="page"/>` starts a new page at that point; `<br type="column"/>` starts a new column at that point. \
The text before the tag and the text after it stay one paragraph.

- Write the tag where the break falls, with no space inside it: `앞 문장.<br type="page"/>뒤 문장`, \
`서울 14곳.<br type="column"/>부산 5곳`.
- `<br>` takes only `type`, and `type` is `page` or `column`. It is a single tag; there is no `</br>`. A `<br/>` \
with no `type` is still a line break inside the paragraph (the text continues on the next line of the same page \
and column).
- A line that holds only `<pagebreak/>` is still a page break *between* paragraphs, as above. A line that holds \
only `<br type="column"/>` is a paragraph whose only content is a column break.
- Nothing else can be expressed: a section break, an odd-page or even-page break, a change of column count, a \
change of margins. A request for one of those is refused.

Example (a page break in the middle of the 핵심 지표 paragraph, a column break in the 지역 줄, then a page break \
between the body and the 부록 heading):

```
수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.<br type="page"/>첨부 1의 지점별 표는 10월 말 확정치입니다.

서울 14곳, 경기 7곳.<br type="column"/>부산 5곳, 대구 3곳.

<pagebreak/>

# 부록
```
"""

# ---------------------------------------------------------------- spreadsheets (shapes, databar)

SHEET_BASICS = """\
A workbook file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The \
text is the workbook's structure; cells are not in it. Each sheet is a `<sheet name="…" range="A1:G20">` … \
`</sheet>` block. Inside a sheet, in this order as they appear:

- A table is a `<table name="…" range="A1:D12">` block whose body is a pipe table of columns (column, type, \
format, formula). A column's type is text, number, date or mixed.
- `<merge range="A1:G1"/>`, `<hyperlink range="H4" target="https://example.com"/>`, `<note range="A3" \
author="작성자" text="…"/>` (a newline in the text is `&#10;`), `<validation range="B2:B20" type="list" \
formula1="&quot;강남,서초&quot;"/>`, and `<conditional-format range="F4:F12" type="cellIs" operator="lessThan" \
formula="3000000" dxf="fill=#FFC7CE"/>` are one line each. Line order of conditional formats is the sheet-wide \
priority order.
- `<chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>` is a chart. `<keep id="…" kind="…" \
summary="…"/>` stands for content kept for you (a picture, a pivot table); leave every one exactly as it is.

A sheet's `range` is its used range and cannot be edited here. Besides the tags described below, there are no \
other tags or attributes.
"""

SHAPES_A = """
### Drawing shapes

A drawing shape that holds ordinary text is a line in the sheet. `<shape name="…" anchor="B2:D5">text</shape>` \
names the shape, the cells it sits on, and its text. `anchor` is an A1 range: the top-left cell of the shape to \
the bottom-right cell (Excel's two-cell anchor). A shape without text is `<shape name="…" anchor="B2:D5"/>`.

- A group of shapes is a `<group name="…">` line, then each of its shapes, then a line `</group>`. Shapes inside \
the group are written the same way as shapes outside it. Do not add, remove or rename a group's shapes by editing \
another group's lines; add or delete a `<shape>` line inside this group, or move the whole `<group>` … `</group>` \
block.
- Change a shape's text by editing the text between the tags. Move or resize it by changing `anchor`. Delete it \
by deleting its line (or the group's block). A new shape is a new `<shape name="…" anchor="…">text</shape>` line; \
do not invent an `id`.
- `name` and `anchor` are required. `anchor` is a cell or a range (`B25`, `B25:D27`). Nothing else can be \
expressed: a picture, a chart, a connector, rotation, a fill, a shadow, an arrow. A request for one of those is \
refused.

Example (a tip over B25:D27, then a group of two KPI badges):

```
<shape name="입력 안내" anchor="B25:D27">아래에 입력하면 표가 늘어납니다.</shape>
<group name="신호등">
<shape name="매출 배지" anchor="F2:G3">매출 +12%</shape>
<shape name="이익 배지" anchor="F4:G5">이익 +8%</shape>
</group>
```
"""

SHAPES_B = """
### Drawing shapes

A drawing shape that holds ordinary text is a line in the sheet, written like a note. `<shape name="…" \
anchor="B2:D5" text="…"/>` names the shape, the cells it sits on, and its text. `anchor` is an A1 range: the \
top-left cell of the shape to the bottom-right cell (Excel's two-cell anchor). A shape without text omits \
`text`.

- A shape that belongs to a group adds `group="Name"` on the same line. Consecutive lines with the same \
`group` are that group, in that order. Change membership by adding or removing `group`; do not split one \
group's lines around another object.
- Change a shape's text by editing `text`. Move or resize it by changing `anchor`. Delete it by deleting its \
line. A new shape is a new `<shape name="…" anchor="…" text="…"/>` line; do not invent an `id`.
- `name` and `anchor` are required. `anchor` is a cell or a range (`B25`, `B25:D27`). `text` and `group` are \
optional. Nothing else can be expressed: a picture, a chart, a connector, rotation, a fill, a shadow, an arrow. \
A request for one of those is refused.

Example (a tip over B25:D27, then a group of two KPI badges):

```
<shape name="입력 안내" anchor="B25:D27" text="아래에 입력하면 표가 늘어납니다."/>
<shape name="매출 배지" anchor="F2:G3" text="매출 +12%" group="신호등"/>
<shape name="이익 배지" anchor="F4:G5" text="이익 +8%" group="신호등"/>
```
"""

DATABAR_A = """
### Data bars

A data-bar conditional format is a `<conditional-format>` line with `type="dataBar"`. It paints a bar in each \
cell of its range, from a minimum to a maximum, in one fill colour.

- Required: `range` (an A1 cell or range), `type="dataBar"`, `min`, `max`, `fill`.
- `min` and `max` each name a type, and a value when the type needs one, written as one token: `min` (the \
smallest value in the range), `max` (the largest), `num:0` (a number, or a defined name such as \
`num:월간수입합계`), `percent:50` (0–100), `percentile:90` (0–100), `formula:=A1` (an Excel formula).
- `fill` is `#RRGGBB` or a theme colour by name (`accent1`, `accent4`, `tx1`).
- Other conditional formats stay as they are: `type="cellIs"` with `operator`, `formula` and `dxf`. Line order \
is still the sheet-wide priority order.
- Nothing else can be expressed: a color scale, an icon set, a gradient bar, `showValue`, an axis position, a \
border on the bar. A request for one of those is refused.

Example (a bar on the 실적 column from 0 to the named total, Office's blue; a second bar on 달성률 as 0–100%):

```
<conditional-format range="E4:E15" type="dataBar" min="num:0" max="num:월간수입합계" fill="#638EC6"/>
<conditional-format range="G4:G15" type="dataBar" min="min" max="max" fill="#FFC7CE"/>
```
"""

DATABAR_B = """
### Data bars

A data-bar conditional format is a `<conditional-format>` line with `type="dataBar"`. It paints a bar in each \
cell of its range, from a minimum to a maximum, in one fill colour.

- Required: `range` (an A1 cell or range), `type="dataBar"`, `min-type`, `max-type`, `color`.
- `min-type` and `max-type` are `min` (the smallest value in the range), `max` (the largest), `num` (a number, \
or a defined name), `percent` (0–100), `percentile` (0–100), `formula` (an Excel formula). When the type needs \
a value, `min` or `max` holds it: `min-type="num" min="0"`, `max-type="num" max="월간수입합계"`, \
`max-type="percent" max="100"`, `min-type="formula" min="=A1"`. `min` and `max` types write no value.
- `color` is `#RRGGBB` or a theme colour by name (`accent1`, `accent4`, `tx1`).
- Other conditional formats stay as they are: `type="cellIs"` with `operator`, `formula` and `dxf`. Line order \
is still the sheet-wide priority order.
- Nothing else can be expressed: a color scale, an icon set, a gradient bar, `showValue`, an axis position, a \
border on the bar. A request for one of those is refused.

Example (a bar on the 실적 column from 0 to the named total, Office's blue; a second bar on 달성률 as 0–100%):

```
<conditional-format range="E4:E15" type="dataBar" min-type="num" min="0" max-type="num" max="월간수입합계" color="#638EC6"/>
<conditional-format range="G4:G15" type="dataBar" min-type="min" max-type="max" color="#FFC7CE"/>
```
"""

GUIDES = {
    ('breaks', 'A'): DOC_BASICS + BREAKS_A,
    ('breaks', 'B'): DOC_BASICS + BREAKS_B,
    ('shapes', 'A'): SHEET_BASICS + SHAPES_A,
    ('shapes', 'B'): SHEET_BASICS + SHAPES_B,
    ('databar', 'A'): SHEET_BASICS + DATABAR_A,
    ('databar', 'B'): SHEET_BASICS + DATABAR_B,
}


def _balance():
    """Pad the shorter candidate of each pair with a trailing note so A and B are the same length."""
    for dec in ('breaks', 'shapes', 'databar'):
        a, b = GUIDES[dec, 'A'], GUIDES[dec, 'B']
        if len(a) == len(b):
            continue
        short, long_, key = (a, b, 'A') if len(a) < len(b) else (b, a, 'B')
        n = len(long_) - len(short)
        # A filler that is valid guide text and does not name the other candidate.
        pad = ' The tags are written exactly as shown.'
        extra = (n - len(pad)) * ' '
        if n < len(pad):
            pad = pad[:n]
            extra = ''
        GUIDES[dec, key] = short + pad + extra
        assert len(GUIDES[dec, 'A']) == len(GUIDES[dec, 'B']), (dec, len(GUIDES[dec, 'A']), len(GUIDES[dec, 'B']))


_balance()
