You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 7 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A workbook file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The text is the workbook's structure; cells are not in it. Each sheet is a `<sheet name="…" range="A1:G20">` … `</sheet>` block. Inside a sheet, in this order as they appear:

- A table is a `<table name="…" range="A1:D12">` block whose body is a pipe table of columns (column, type, format, formula). A column's type is text, number, date or mixed.
- `<merge range="A1:G1"/>`, `<hyperlink range="H4" target="https://example.com"/>`, `<note range="A3" author="작성자" text="…"/>` (a newline in the text is `&#10;`), `<validation range="B2:B20" type="list" formula1="&quot;강남,서초&quot;"/>`, and `<conditional-format range="F4:F12" type="cellIs" operator="lessThan" formula="3000000" dxf="fill=#FFC7CE"/>` are one line each. Line order of conditional formats is the sheet-wide priority order.
- `<chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>` is a chart. `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a pivot table); leave every one exactly as it is.

A sheet's `range` is its used range and cannot be edited here. Besides the tags described below, there are no other tags or attributes.

### Drawing shapes

A drawing shape that holds ordinary text is a line in the sheet, written like a note. `<shape name="…" anchor="B2:D5" text="…"/>` names the shape, the cells it sits on, and its text. `anchor` is an A1 range: the top-left cell of the shape to the bottom-right cell (Excel's two-cell anchor). A shape without text omits `text`.

- A shape that belongs to a group adds `group="Name"` on the same line. Consecutive lines with the same `group` are that group, in that order. Change membership by adding or removing `group`; do not split one group's lines around another object.
- Change a shape's text by editing `text`. Move or resize it by changing `anchor`. Delete it by deleting its line. A new shape is a new `<shape name="…" anchor="…" text="…"/>` line; do not invent an `id`.
- `name` and `anchor` are required. `anchor` is a cell or a range (`B25`, `B25:D27`). `text` and `group` are optional. Nothing else can be expressed: a picture, a chart, a connector, rotation, a fill, a shadow, an arrow. A request for one of those is refused.

Example (a tip over B25:D27, then a group of two KPI badges):

```
<shape name="입력 안내" anchor="B25:D27" text="아래에 입력하면 표가 늘어납니다."/>
<shape name="매출 배지" anchor="F2:G3" text="매출 +12%" group="신호등"/>
<shape name="이익 배지" anchor="F4:G5" text="이익 +8%" group="신호등"/>
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

<sheet name="월별수지" range="A1:G30">

<table name="수지" range="A1:D20">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy-mm |  |
| 수입 | number | #,##0 |  |
| 지출 | number | #,##0 |  |
| 잔액 | number | #,##0 | =[@수입]-[@지출] |
</table>

<merge range="A1:D1"/>

<note range="C3" author="작성자" text="지출은 카드 승인 기준입니다."/>

<hyperlink range="G2" target="https://example.com/budget"/>

<validation range="B2:B20" type="list" formula1="&quot;급여,부수입,환급&quot;"/>

<conditional-format range="D2:D20" type="cellIs" operator="lessThan" formula="0" dxf="fill=#FFC7CE"/>

<chart type="column" data="수지[월],수지[잔액]" title="월별 잔액"/>

<shape name="입력 안내" anchor="B25:D27" text="아래에 입력하면 표가 늘어납니다. 마지막 행에서 Enter."/>

<shape name="매출 배지" anchor="F2:G3" text="매출 +12%" group="신호등"/>
<shape name="이익 배지" anchor="F4:G5" text="이익 +8%" group="신호등"/>

<keep id="k1" kind="picture" summary="로고"/>

</sheet>
=== FILE END ===

## Tasks

1. `kpi1-q1` (read): What is the anchor of the shape 입력 안내, and which two shape names are in the group 신호등? Answer as `ANSWER: <anchor> | <name>, <name>` in the order they are written.
2. `kpi1-w` (write): Write a new workbook with exactly one sheet 메모, range A1:D8, containing: a table 메모표 on A1:B4 with columns 항목 (text) and 값 (text); a shape named 오늘, anchor C2:D3, text "오늘은 마감일입니다."; a group named 뱃지 of two shapes, 통과 at C5:D6 text "통과" and 보류 at C7:D8 text "보류". No other items. Use the front matter of a spreadsheet (type spreadsheet, format xlsx, schema 1).
3. `kpi1-e1` (edit): Add a shape named 환급 안내, anchored B28:D29, with text "환급은 다음 달 수입에 넣습니다." Leave the 입력 안내 shape where it is.
4. `kpi1-e2` (edit): Move the 입력 안내 shape so its anchor is B28:D30. Do not change its text. Do not move the 신호등 group.
5. `kpi1-e3` (edit): Change the text of 매출 배지 to "매출 +15%". Do not edit 이익 배지, and do not change any cell note.
6. `kpi1-e4` (edit): Resize 이익 배지 so its anchor is F4:H6. Keep its text and keep it in the 신호등 group.
7. `kpi1-r` (edit): Add a picture from files/org.png as a drawing over H2:J10, and rotate the 입력 안내 shape 15 degrees.

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
