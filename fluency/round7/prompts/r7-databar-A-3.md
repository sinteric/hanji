You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 6 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

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
## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
---
type: spreadsheet
format: xlsx
schema: 1
---

<sheet name="KPI" range="A1:E16">

<table name="점수" range="A1:E12">
| column | type | format | formula |
|---|---|---|---|
| 팀 | text |  |  |
| 점수 | number | 0 |  |
| 목표 | number | 0 |  |
| 가중 | number | 0 |  |
| 등급 | text |  |  |
</table>

<conditional-format range="B2:B12" type="dataBar" min="percentile:10" max="percentile:90" fill="#638EC6"/>

<conditional-format range="C2:C12" type="dataBar" min="num:0" max="num:100" fill="#FFEB84"/>

<conditional-format range="B2:B12" type="cellIs" operator="greaterThan" formula="80" dxf="fill=#C6EFCE"/>

</sheet>
=== FILE END ===

## Tasks

1. `score3-q1` (read): The 점수 column (B) has a data bar. What are its min and max types (and values, if any) and its fill? Answer as `ANSWER: min=<…> max=<…> fill=<…>` using the file's own tokens.
2. `score3-w` (write): Write a new workbook with exactly one sheet 요약, range A1:C6, containing: a table 요약표 on A1:C4 with columns 팀 (text), 점수 (number, format 0), 목표 (number, format 0); one data-bar rule on B2:B4 whose minimum is the smallest value in the range, whose maximum is the largest, and whose fill is #FFEB84. No other items.
3. `score3-e1` (edit): Change the 점수 data bar so its range is B2:B14. Do not change the 목표 bar on C, and do not change the cellIs rule that also sits on B2:B12.
4. `score3-e2` (edit): Change the fill of the 점수 data bar to #FFC7CE. Leave the 목표 bar yellow.
5. `score3-e3` (edit): Change the 점수 data bar so its minimum is the number 0 and its maximum is the number 100 (num types). Keep its fill. Do not edit the cellIs rule that uses 80.
6. `score3-r` (edit): Add an icon set (3 arrows) on 등급 (E2:E12) and a data bar with a solid axis at the cell midpoint.

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
