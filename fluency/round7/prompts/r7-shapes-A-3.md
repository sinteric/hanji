You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 7 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

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

## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
---
type: spreadsheet
format: xlsx
schema: 1
---

<sheet name="집행" range="A1:G24">

<table name="사업" range="A1:D14">
| column | type | format | formula |
|---|---|---|---|
| 사업 | text |  |  |
| 예산 | number | #,##0 |  |
| 집행 | number | #,##0 |  |
| 집행률 | number | 0% |  |
</table>

<note range="D2" author="작성자" text="집행률 82%는 9월 말 기준."/>

<shape name="표 제목" anchor="A16:C17">2026 사업별 집행</shape>

<shape name="입력 안내" anchor="B20:D22">행을 추가하려면 마지막 사업명 아래칸에 입력합니다.</shape>

<group name="화살표">
<shape name="증가 화살표" anchor="F2:G3">집행률 82%</shape>
<shape name="주의 화살표" anchor="F4:G5">인건비 초과</shape>
</group>

<keep id="k3" kind="picture" summary="부서 직인"/>

</sheet>
=== FILE END ===

## Tasks

1. `exec3-q1` (read): The file repeats 집행률 82%. Which object is the shape (not the note) that holds those words, and what group is it in? Answer as `ANSWER: <shape name> | <group name>`.
2. `exec3-w` (write): Write a new workbook with exactly one sheet 메모, range A1:D8, containing: a table 메모표 on A1:B3 with columns 사업 (text) and 집행률 (number, format 0%); a shape named 제목 anchored C1:D2 text "주간 집행"; a group named 표시 of two shapes, 양호 at C4:D5 text "양호" and 주의 at C6:D7 text "주의". No other items.
3. `exec3-e1` (edit): Add a shape named 마감 안내, anchored B23:D24, with text "10월 마감은 11월 5일입니다."
4. `exec3-e2` (edit): Move 표 제목 so its anchor is A18:C19. Keep its text "2026 사업별 집행".
5. `exec3-e3` (edit): Change the text of 증가 화살표 to "집행률 85%". Do not edit the note that says 집행률 82%.
6. `exec3-e4` (edit): Resize 입력 안내 so its anchor is B20:E23. Keep its text.
7. `exec3-r` (edit): Turn 증가 화살표 into a right-arrow preset (kind=rightArrow) filled accent1, with a 2 pt outline.

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
