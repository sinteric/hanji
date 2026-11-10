A workbook file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The text is the workbook's structure; cells are not in it. Each sheet is a `<sheet name="…" range="A1:G20">` … `</sheet>` block. Inside a sheet, in this order as they appear:

- A table is a `<table name="…" range="A1:D12">` block whose body is a pipe table of columns (column, type, format, formula). A column's type is text, number, date or mixed.
- `<merge range="A1:G1"/>`, `<hyperlink range="H4" target="https://example.com"/>`, `<note range="A3" author="작성자" text="…"/>` (a newline in the text is `&#10;`), `<validation range="B2:B20" type="list" formula1="&quot;강남,서초&quot;"/>`, and `<conditional-format range="F4:F12" type="cellIs" operator="lessThan" formula="3000000" dxf="fill=#FFC7CE"/>` are one line each. Line order of conditional formats is the sheet-wide priority order.
- `<chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>` is a chart. `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a pivot table); leave every one exactly as it is.

A sheet's `range` is its used range and cannot be edited here. Besides the tags described below, there are no other tags or attributes.

### Data bars

A data-bar conditional format is a `<conditional-format>` line with `type="dataBar"`. It paints a bar in each cell of its range, from a minimum to a maximum, in one fill colour.

- Required: `range` (an A1 cell or range), `type="dataBar"`, `min`, `max`, `fill`.
- `min` and `max` each name a type, and a value when the type needs one, written as one token: `min` (the smallest value in the range), `max` (the largest), `num:0` (a number, or a defined name such as `num:월간수입합계`), `percent:50` (0–100), `percentile:90` (0–100), `formula:=A1` (an Excel formula).
- `fill` is `#RRGGBB` or a theme colour by name (`accent1`, `accent4`, `tx1`).
- Other conditional formats stay as they are: `type="cellIs"` with `operator`, `formula` and `dxf`. Line order is still the sheet-wide priority order.
- Nothing else can be expressed: a color scale, an icon set, a gradient bar, `showValue`, an axis position, a border on the bar. A request for one of those is refused.

Example (a bar on the 실적 column from 0 to the named total, Office's blue; a second bar on 달성률 as 0–100%):

```
<conditional-format range="E4:E15" type="dataBar" min="num:0" max="num:월간수입합계" fill="#638EC6"/>
<conditional-format range="G4:G15" type="dataBar" min="min" max="max" fill="#FFC7CE"/>
```
 The tags are written exactly as shown.                                                                                                                            