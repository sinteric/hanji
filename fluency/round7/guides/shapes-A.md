A workbook file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The text is the workbook's structure; cells are not in it. Each sheet is a `<sheet name="…" range="A1:G20">` … `</sheet>` block. Inside a sheet, in this order as they appear:

- A table is a `<table name="…" range="A1:D12">` block whose body is a pipe table of columns (column, type, format, formula). A column's type is text, number, date or mixed.
- `<merge range="A1:G1"/>`, `<hyperlink range="H4" target="https://example.com"/>`, `<note range="A3" author="작성자" text="…"/>` (a newline in the text is `&#10;`), `<validation range="B2:B20" type="list" formula1="&quot;강남,서초&quot;"/>`, and `<conditional-format range="F4:F12" type="cellIs" operator="lessThan" formula="3000000" dxf="fill=#FFC7CE"/>` are one line each. Line order of conditional formats is the sheet-wide priority order.
- `<chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>` is a chart. `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a pivot table); leave every one exactly as it is.

A sheet's `range` is its used range and cannot be edited here. Besides the tags described below, there are no other tags or attributes.

### Drawing shapes

A drawing shape that holds ordinary text is a line in the sheet. `<shape name="…" anchor="B2:D5">text</shape>` names the shape, the cells it sits on, and its text. `anchor` is an A1 range: the top-left cell of the shape to the bottom-right cell (Excel's two-cell anchor). A shape without text is `<shape name="…" anchor="B2:D5"/>`.

- A group of shapes is a `<group name="…">` line, then each of its shapes, then a line `</group>`. Shapes inside the group are written the same way as shapes outside it. Do not add, remove or rename a group's shapes by editing another group's lines; add or delete a `<shape>` line inside this group, or move the whole `<group>` … `</group>` block.
- Change a shape's text by editing the text between the tags. Move or resize it by changing `anchor`. Delete it by deleting its line (or the group's block). A new shape is a new `<shape name="…" anchor="…">text</shape>` line; do not invent an `id`.
- `name` and `anchor` are required. `anchor` is a cell or a range (`B25`, `B25:D27`). Nothing else can be expressed: a picture, a chart, a connector, rotation, a fill, a shadow, an arrow. A request for one of those is refused.

Example (a tip over B25:D27, then a group of two KPI badges):

```
<shape name="입력 안내" anchor="B25:D27">아래에 입력하면 표가 늘어납니다.</shape>
<group name="신호등">
<shape name="매출 배지" anchor="F2:G3">매출 +12%</shape>
<shape name="이익 배지" anchor="F4:G5">이익 +8%</shape>
</group>
```
