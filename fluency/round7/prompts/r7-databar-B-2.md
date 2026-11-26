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

<sheet name="달성" range="A1:F18">

<table name="지역" range="A1:F14">
| column | type | format | formula |
|---|---|---|---|
| 지역 | text |  |  |
| 매출 | number | #,##0 |  |
| 목표 | number | #,##0 |  |
| 달성률 | number | 0% |  |
| 전년 | number | 0% |  |
| 증감 | number | 0% |  |
</table>

<conditional-format range="D2:D14" type="dataBar" min-type="num" min="0" max-type="percent" max="100" color="#638EC6"/>

<conditional-format range="E2:E14" type="dataBar" min-type="num" min="0" max-type="percent" max="100" color="#63BE7B"/>

<conditional-format range="F2:F14" type="cellIs" operator="lessThan" formula="0" dxf="fill=#FFC7CE"/>

</sheet>
=== FILE END ===

## Tasks

1. `rate2-q1` (read): Two columns have a data bar from 0 to 100 percent. What is the fill of the 달성률 column (D), and what is the fill of the 전년 column (E)? Answer as `ANSWER: D=<…> | E=<…>`.
2. `rate2-w` (write): Write a new workbook with exactly one sheet 요약, range A1:C6, containing: a table 요약표 on A1:C4 with columns 지역 (text), 달성률 (number, format 0%), 전년 (number, format 0%); one data-bar rule on B2:B4 whose minimum is the number 0, whose maximum is 100 percent, and whose fill is #63BE7B. No other items.
3. `rate2-e1` (edit): Change the 달성률 data bar (column D) so its range is D2:D16. Do not change the 전년 bar on E.
4. `rate2-e2` (edit): Change the fill of the 달성률 data bar to #FFC7CE. Leave the 전년 bar green.
5. `rate2-e3` (edit): Change the 달성률 data bar so its minimum is 0 percent and its maximum is 100 percent (percent types, not num). Keep its fill and range.
6. `rate2-r` (edit): Make the 달성률 data bar a gradient bar (the bar fades) and hide the cell values (showValue off).

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
