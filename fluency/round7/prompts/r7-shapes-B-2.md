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

<sheet name="지역매출" range="A1:H28">

<table name="지역" range="A1:E16">
| column | type | format | formula |
|---|---|---|---|
| 지역 | text |  |  |
| 지점 | text |  |  |
| 매출 | number | #,##0 |  |
| 목표 | number | #,##0 |  |
| 달성 | number | 0% |  |
</table>

<note range="E2" author="작성자" text="부산 해운대는 목표 미달 (잠정)."/>

<shape name="서울 콜아웃" anchor="B18:D20" text="서울 14곳, 전년 대비 +12%"/>

<shape name="부산 콜아웃" anchor="B22:D24" text="목표 미달"/>

<shape name="수도권 배지" anchor="F2:G3" text="수도권 21곳" group="권역 배지"/>
<shape name="지방 배지" anchor="F4:G5" text="지방 13곳" group="권역 배지"/>
<shape name="합계 배지" anchor="F6:G7" text="합계 34곳" group="권역 배지"/>

<keep id="k2" kind="chart" summary="지역별 막대"/>

</sheet>
=== FILE END ===

## Tasks

1. `region2-q1` (read): Which shape is named 부산 콜아웃, what is its anchor, and what is its text? Answer as `ANSWER: <anchor> | <text>`.
2. `region2-w` (write): Write a new workbook with exactly one sheet 메모, range A1:D8, containing: a table 메모표 on A1:B3 with columns 지역 (text) and 상태 (text); a shape named 경고 anchored C2:D3 text "목표 미달"; a group named 등 of two shapes, 녹 at C5:D6 text "녹" and 적 at C7:D8 text "적". No other items.
3. `region2-e1` (edit): Add a shape named 대구 콜아웃, anchored B26:D28, with text "대구 3곳, 전년 대비 +4%". Do not change 서울 콜아웃.
4. `region2-e2` (edit): Move 부산 콜아웃 so its anchor is E22:G24. Keep its text "목표 미달". Do not edit the note that also says 목표 미달.
5. `region2-e3` (edit): Change the text of 부산 콜아웃 to "목표 달성". Do not edit the note that says 목표 미달 (잠정).
6. `region2-e4` (edit): Resize 서울 콜아웃 so its anchor is B18:E21. Keep its text.
7. `region2-r` (edit): Connect 서울 콜아웃 to 부산 콜아웃 with an elbow connector, and give 수도권 배지 a drop shadow.

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
