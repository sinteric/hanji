You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 8 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A workbook file is plain text. It describes the workbook's structure; the cell values are shown in `<data>` windows.

### Text

- `<sheet name="…" range="A1:H23">` … `</sheet>` is one sheet and its used range.
- `<table name="…" range="…">` lists a table's columns (name, type, number format, formula). `<data …>` shows cell values: a pipe table whose first column is the sheet row number, then one column per sheet column or table column.
- `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (merged cells, charts, conditional formats, notes, validations); leave it as it is.

### Formatting

Formatting is shown as range lines, in a small fixed vocabulary.

- The line `<format default …/>` at the top is the workbook's default cell formatting (the Normal style): every cell has it unless a range line says otherwise. A property it leaves out is none, off, general (alignment), bottom (vertical alignment) or 0 (indent).
- In a sheet, `<format range="B4:C7" key=value …/>` gives every cell of that rectangle the properties written, beyond the default. A cell is in at most one range line; a property its line leaves out is the default's. A cell in no range line has the default formatting.

### The vocabulary

`key=value` pairs separated by spaces; a value with a space is quoted.

- `fill` (a colour or `none`), `border-top` / `-right` / `-bottom` / `-left` (a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`), `align` (general, left, center, right, justify, distribute), `valign` (top, middle, bottom), `indent` (a whole number of indent levels), `font`, `size` (pt), `color`, `bold`, `italic`, `underline`, `strike` (a flag written alone means on).
- A colour is `#RRGGBB` or a theme colour by name: `tx1`, `bg1`, `tx2`, `bg2`, `accent1` … `accent6`, `hlink`; `+N%` after a theme colour means N% lighter and `-N%` N% darker (`bg2-10%`, `accent2+80%`). A theme colour follows the workbook's theme.
- Excel border widths are hair 0.25pt (dotted), thin 0.75pt, medium 1.5pt and thick 2.25pt; `double` is 2.25pt.
- `fill=gradient` and `fill=pattern` are shown but cannot be written. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow, a conditional format.

### Changing formatting

The file is not edited as text. Formatting is written by range operations, applied in order:

```
{"op": "format", "range": "Sheet!A1:D1", "set": {"fill": "#D9D9D9", "bold": true, "border-bottom": "0.75pt solid #000000"}}
```

- `range` is `Sheet!A1` or `Sheet!A1:D4` (a sheet name with spaces or brackets is quoted: `'Exp2 (2)'!A1`).
- `set` gives every cell of the range the properties written, with the same keys and values as the vocabulary (`true` / `false` for the flags), and leaves every other property of those cells as it is. `"fill": "none"` removes a fill and `"border-left": "none"` a border.
- Two keys are for operations only: `border` sets all four sides of every cell of the range, and `outline` sets only the outer edges of the range (the top of its first row, the bottom of its last row, the left of its first column, the right of its last column).
- A border width snaps to the nearest Excel width (2pt becomes 2.25pt); theme colours stay theme colours.

## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
<format default font=Calibri size=11pt color=tx1/>

<sheet name="매출" range="A1:H48">

<format range="A1:F1" align=center font=Cambria size=14pt color=#000000 bold/>
<format range="A3:F3" font="WenQuanYi Zen Hei"/>
<format range="H3:H4" font="WenQuanYi Zen Hei"/>
<format range="B4:C48" font="WenQuanYi Zen Hei"/>

<table name="Sales" range="A3:F48">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy\-mm |  |
| 지점 | text | General |  |
| 제품군 | text | General |  |
| 매출 | number | #,##0 |  |
| 원가 | number | #,##0 |  |
| 이익 | number | #,##0 |  |
</table>

<data table="Sales" rows="4:23">
| row | 월 | 지점 | 제품군 | 매출 | 원가 | 이익 |
|---|---|---|---|---|---|---|
| 4 | 2026-01 | 강남 | 가전 | 10,269,000 | 7,410,000 | 2,859,000 |
| 5 | 2026-01 | 강남 | 모바일 | 8,761,000 | 6,480,000 | 2,281,000 |
| 6 | 2026-01 | 강남 | 생활 | 6,408,000 | 4,560,000 | 1,848,000 |
| 7 | 2026-01 | 서초 | 가전 | 13,825,000 | 9,730,000 | 4,095,000 |
| 8 | 2026-01 | 서초 | 모바일 | 17,393,000 | 11,620,000 | 5,773,000 |
| 9 | 2026-01 | 서초 | 생활 | 4,149,000 | 2,890,000 | 1,259,000 |
| 10 | 2026-01 | 송파 | 가전 | 13,552,000 | 9,900,000 | 3,652,000 |
| 11 | 2026-01 | 송파 | 모바일 | 18,665,000 | 13,340,000 | 5,325,000 |
| 12 | 2026-01 | 송파 | 생활 | 12,200,000 | 8,700,000 | 3,500,000 |
| 13 | 2026-01 | 분당 | 가전 | 6,168,000 | 4,750,000 | 1,418,000 |
| 14 | 2026-01 | 분당 | 모바일 | 15,428,000 | 11,610,000 | 3,818,000 |
| 15 | 2026-01 | 분당 | 생활 | 7,045,000 | 5,060,000 | 1,985,000 |
| 16 | 2026-01 | 일산 | 가전 | 6,051,000 | 4,120,000 | 1,931,000 |
| 17 | 2026-01 | 일산 | 모바일 | 14,419,000 | 10,060,000 | 4,359,000 |
| 18 | 2026-01 | 일산 | 생활 | 18,843,000 | 12,760,000 | 6,083,000 |
| 19 | 2026-02 | 강남 | 가전 | 4,146,000 | 2,800,000 | 1,346,000 |
| 20 | 2026-02 | 강남 | 모바일 | 17,364,000 | 12,590,000 | 4,774,000 |
| 21 | 2026-02 | 강남 | 생활 | 17,694,000 | 12,640,000 | 5,054,000 |
| 22 | 2026-02 | 서초 | 가전 | 15,530,000 | 11,500,000 | 4,030,000 |
| 23 | 2026-02 | 서초 | 모바일 | 16,148,000 | 11,220,000 | 4,928,000 |
</data>

<keep id="k8a8r" kind="merged-cells" summary="A1:F1"/>

<keep id="k0k51" kind="conditional-format" summary="F4:F48"/>

<keep id="kqjeg" kind="data-validation" summary="B4:B48"/>

<keep id="kn7g2" kind="hyperlinks" summary="H4"/>

<keep id="k7mm3" kind="notes" summary="1 note at A3: 월은 매월 1일로 적습니다."/>

<keep id="kushr" kind="chart" summary="column chart “지점별 매출”: 매출!D3, 매출!$B$4:$B$18, 매출!$D$4:$D$18 at H5"/>
</sheet>

<sheet name="담당자" range="A1:E9">

<format range="A1:E1" font="WenQuanYi Zen Hei"/>
<format range="B2:C9" font="WenQuanYi Zen Hei"/>

<table name="Staff" range="A1:E9">
| column | type | format | formula |
|---|---|---|---|
| 사번 | text | @ |  |
| 이름 | text | General |  |
| 지점 | text | General |  |
| 휴대전화 | text | @ |  |
| 입사일 | date | yyyy\-mm\-dd |  |
</table>

<data table="Staff">
| row | 사번 | 이름 | 지점 | 휴대전화 | 입사일 |
|---|---|---|---|---|---|
| 2 | 18867 | 김서연 | 강남 | 010-5934-8996 | 2019-01-02 |
| 3 | 99596 | 이민준 | 서초 | 010-9992-9527 | 2020-02-02 |
| 4 | 13896 | 박지우 | 송파 | 010-1962-3448 | 2021-03-02 |
| 5 | 08024 | 최하윤 | 분당 | 010-5359-9589 | 2022-04-02 |
| 6 | 30984 | 정도윤 | 일산 | 010-3044-8862 | 2023-05-02 |
| 7 | 34711 | 강서준 | 강남 | 010-5237-7881 | 2024-06-02 |
| 8 | 86273 | 조하은 | 서초 | 010-5146-3144 | 2019-07-02 |
| 9 | 42851 | 윤지호 | 송파 | 010-3183-9028 | 2020-08-02 |
</data>
</sheet>

<sheet name="요약" range="A1:B8">

<format range="A1:B1" font="WenQuanYi Zen Hei"/>
<format range="A2:A6" font="WenQuanYi Zen Hei"/>
<format range="A8" font="WenQuanYi Zen Hei"/>

<data sheet="요약" range="A1:B8">
| row | A | B |
|---|---|---|
| 1 | 지점 | 합계 |
| 2 | 강남 | 90,012,000 |
| 3 | 서초 | 111,527,000 |
| 4 | 송파 | 108,272,000 |
| 5 | 분당 | 104,836,000 |
| 6 | 일산 | 92,774,000 |
| 7 |  |  |
| 8 | 총계 | 507,421,000 |
</data>
</sheet>


=== FILE END ===

## Tasks

1. `ks-q1` (read): In sheet 매출, what are the font and the size of the title A1? Answer `ANSWER: font=<name> size=<pt>`.
2. `ks-q2` (read): In sheet 매출, what is the horizontal alignment of cell D1? Answer `ANSWER: align=<value>`.
3. `ks-q3` (read): In sheet 담당자, which cells use the font WenQuanYi Zen Hei? Answer `ANSWER: <range>; <range>`.
4. `ks-e1` (edit): In sheet 매출, make the header row of the Sales table (A3:F3) bold and centred, with a light grey fill (#D9D9D9).
5. `ks-e2` (edit): In sheet 매출, colour the 이익 figures (F4:F48) with the theme colour accent6.
6. `ks-e3` (edit): In sheet 매출, put a thin black bottom border (0.75pt solid) under the Sales table's header row A3:F3.
7. `ks-e4` (edit): In sheet 담당자, give the header row A1:E1 the theme colour accent1 as its fill, with white bold text.
8. `ks-e5` (edit): In sheet 매출, fill F4:F48 with a diagonal-stripe pattern.

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Edit tasks: answer with a list of range operations (`"ops"`), as the documentation describes. Change only what the task asks for.
- If a task cannot be done in this file format, answer `REFUSE: <one sentence why>`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "…", "text": "ANSWER: …"},
  {"task_id": "…", "ops": [{"op": "format", "range": "…", "set": {…}}]},
  {"task_id": "…", "text": "REFUSE: …"}
]}
```

One entry per task, in the order of the tasks.
