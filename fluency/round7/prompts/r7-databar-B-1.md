You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 6 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A workbook file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The text is the workbook's structure; cells are not in it. Each sheet is a `<sheet name="…" range="A1:G20">` … `</sheet>` block. Inside a sheet, in this order as they appear:

- A table is a `<table name="…" range="A1:D12">` block whose body is a pipe table of columns (column, type, format, formula). A column's type is text, number, date or mixed.
- `<merge range="A1:G1"/>`, `<hyperlink range="H4" target="https://example.com"/>`, `<note range="A3" author="작성자" text="…"/>` (a newline in the text is `&#10;`), `<validation range="B2:B20" type="list" formula1="&quot;강남,서초&quot;"/>`, and `<conditional-format range="F4:F12" type="cellIs" operator="lessThan" formula="3000000" dxf="fill=#FFC7CE"/>` are one line each. Line order of conditional formats is the sheet-wide priority order.
- `<chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>` is a chart. `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a pivot table); leave every one exactly as it is.

A sheet's `range` is its used range and cannot be edited here. Besides the tags described below, there are no other tags or attributes.

### Data bars

A data-bar conditional format is a `<conditional-format>` line with `type="dataBar"`. It paints a bar in each cell of its range, from a minimum to a maximum, in one fill colour.

- Required: `range` (an A1 cell or range), `type="dataBar"`, `min-type`, `max-type`, `color`.
- `min-type` and `max-type` are `min` (the smallest value in the range), `max` (the largest), `num` (a number, or a defined name), `percent` (0–100), `percentile` (0–100), `formula` (an Excel formula). When the type needs a value, `min` or `max` holds it: `min-type="num" min="0"`, `max-type="num" max="월간수입합계"`, `max-type="percent" max="100"`, `min-type="formula" min="=A1"`. `min` and `max` types write no value.
- `color` is `#RRGGBB` or a theme colour by name (`accent1`, `accent4`, `tx1`).
- Other conditional formats stay as they are: `type="cellIs"` with `operator`, `formula` and `dxf`. Line order is still the sheet-wide priority order.
- Nothing else can be expressed: a color scale, an icon set, a gradient bar, `showValue`, an axis position, a border on the bar. A request for one of those is refused.

Example (a bar on the 실적 column from 0 to the named total, Office's blue; a second bar on 달성률 as 0–100%):

```
<conditional-format range="E4:E15" type="dataBar" min-type="num" min="0" max-type="num" max="월간수입합계" color="#638EC6"/>
<conditional-format range="G4:G15" type="dataBar" min-type="min" max-type="max" color="#FFC7CE"/>
```

## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
---
type: spreadsheet
format: xlsx
schema: 1
---

<sheet name="예산" range="A1:G20">

<table name="월별" range="A1:G16">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy-mm |  |
| 수입 | number | #,##0 |  |
| 지출 | number | #,##0 |  |
| 잔액 | number | #,##0 | =[@수입]-[@지출] |
| 실적 | number | #,##0 |  |
| 목표 | number | #,##0 |  |
| 달성률 | number | 0% |  |
</table>

<conditional-format range="D2:D16" type="cellIs" operator="lessThan" formula="0" dxf="fill=#FFC7CE"/>

<conditional-format range="E4:E16" type="dataBar" min-type="num" min="0" max-type="num" max="월간수입합계" color="#638EC6"/>

<conditional-format range="G4:G16" type="dataBar" min-type="min" max-type="max" color="#FFC7CE"/>

<keep id="k1" kind="chart" summary="수입·지출 막대"/>

</sheet>
=== FILE END ===

## Tasks

1. `budget1-q1` (read): The 실적 column (E) has a data bar. What are its min, max and fill? Answer as `ANSWER: min=<…> max=<…> fill=<…>` using the file's own tokens.
2. `budget1-w` (write): Write a new workbook with exactly one sheet 요약, range A1:C6, containing: a table 요약표 on A1:C4 with columns 지표 (text), 값 (number, format #,##0), 비율 (number, format 0%); one data-bar rule on B2:B4 whose minimum is the number 0, whose maximum is the largest value in the range, and whose fill is #638EC6. No other items.
3. `budget1-e1` (edit): Change the data bar on the 실적 column so its range is E4:E18. Do not change the data bar on 달성률 (G), and do not change the cellIs rule on 잔액.
4. `budget1-e2` (edit): Change the fill colour of the 실적 data bar to #63BE7B. Leave the 달성률 bar's colour as it is.
5. `budget1-e3` (edit): Change the 실적 data bar so its minimum is the smallest value in the range and its maximum is the largest value in the range (the auto min/max types, no number). Keep its fill.
6. `budget1-r` (edit): Add a 3-color scale on F4:F16 (red-yellow-green) and an icon set on G4:G16.

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Write tasks: answer with `text`: the complete new file, including its front matter.
- Edit tasks: answer with exact text edits. Each edit is `{"old": "...", "new": "..."}`: `old` is copied exactly from the file (same spaces and characters) and occurs exactly once in it, and `new` replaces it. Edits apply in order. Change only what the task asks for; every other line must stay exactly as it is.
- If a task cannot be done in this file format, do not edit: answer `REFUSE: <one sentence why>` and `"edits": []`. Refuse only when the task cannot be done as asked.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "…", "text": "ANSWER: …"},
  {"task_id": "…", "text": "<the complete new file>"},
  {"task_id": "…", "edits": [{"old": "…", "new": "…"}]},
  {"task_id": "…", "text": "REFUSE: …", "edits": []}
]}
```

One entry per task, in the order of the tasks.
