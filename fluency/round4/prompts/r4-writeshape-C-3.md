You work with spreadsheet workbooks shown as text. Below are the documentation of the text form and of how changes are written, the workbook itself, and six tasks. Do each task on its own: every task starts from the workbook exactly as shown, not from the result of another task.

## Documentation

A workbook is shown as text, sheet by sheet. Each sheet is a `<sheet name="…">` … `</sheet>` block holding its tables. Each table first has a `<table name="…" range="…">` … `</table>` block. `range` is the table's area in A1 notation: its first row is the header row (the column names), so the first data row is the next sheet row, and the columns take the range's letters in order. In `range="A1:F153"` the first column is A and the sixth is F, and the data rows are 2 to 153. Each line of the block is one column: its name, its type, its number format and, for a formula column, its formula.

- Types: `text`, `number` or `date`. Every cell of a column holds a value of that type, or is empty.
- Number formats say how a value is displayed: `#,##0` (1234567 displays 1,234,567), `#,##0.0` and `0.0` (one decimal), `0` (a whole number), `0.0%` (0.923 displays 92.3%), `0000`, `00000` and `000000` (a whole number padded with leading zeros: 417 in `00000` displays 00417), `yyyy-mm` (the date 2026-03-01 displays 2026-03), `yyyy-mm-dd`, and `@` for text, displayed as it is.
- A formula column computes every row from one formula written with structured references: `[@매출]` is this row's 매출, and `Sales[매출]` is the whole 매출 column of the table Sales. Its values are computed, never typed.
- A row whose cells are all empty is a blank row; its formula cells are empty too. Tables often hold blank rows between blocks, and subtotal rows (소계) written like data rows.

### Cell data

After its `<table>` block, each table's cells follow in a `<data table="…">` … `</data>` block. It is a pipe table whose header row is `row` followed by the table's column names, and then one line per data row: every row of the table, in sheet order, blank rows included.

- The first cell of a line is the row's sheet row number.
- The other cells are the row's values, one per column in the column order, each displayed with its column's number format: 12,000,000 in `#,##0` is the number 12000000, and 00417 in `00000` is the number 417 shown with leading zeros.
- An empty cell has nothing between its pipes, and a blank row is a line of empty cells after its row number.
- Rows are never left out or merged: a table with data rows 2 to 153 has 152 lines, one per sheet row.
- A cell's address is its column's letter and the row number: the third column of the line starting `| 3 |` is cell C3.
- The tables of a sheet follow one another, each with its own two blocks.

Example (a table Sales at A1 with the columns 월, 지점 and 매출):

```
<data table="Sales">
| row | 월 | 지점 | 매출 |
|---|---|---|---|
| 2 | 2026-01 | 강남 | 12,000,000 |
| 3 | 2026-01 | 서초 | 9,500,000 |
| 4 |  |  |  |
| 5 | 2026-02 | 강남 | 11,200,000 |
</data>
```

Here row 4 is blank, and the 매출 of 서초 in 2026-01 is 9,500,000, in cell C3.

### Rules for every change

- Types are never guessed from how a value looks: IDs, codes and phone numbers are text, even when they hold only digits. When a column's type changes, its values are converted as they are displayed.
- A formula column's cells are computed from its formula on every row; they are never set one by one. Formulas use only structured references: `[@Column]` or `[@[Column name]]` for a cell of this row, `[@[First]:[Last]]` for this row's cells from one column to another (inside SUM), and `Table[Column]` for a whole column of a table. The functions are SUM, SUMIFS, COUNTIFS, AVERAGE, MIN, MAX, ROUND, ABS, IF and IFERROR. A1 references such as D2, $D$2, D2:D9 or 매출!D2 are not allowed in formulas.
- A value that starts with `=` is not accepted: a formula belongs to a formula column.
- Functions that fetch data (WEBSERVICE, FILTERXML, HYPERLINK and the like) are not allowed.
- Formatting is by type and number format only, from the formats listed above; colours, fonts, fills, borders and column widths cannot be set.
- No answer adds more than 50 rows in total: bulk rows come from an import, not from rows you type or generate.
- A table name is letters, digits, `_` and `.`, starting with a letter, and unique in the workbook. Tables on one sheet never overlap.

### Writing changes: code

Answer a write or edit task with `text`: Python code that changes the workbook through the objects below. It runs in a sandbox: no imports, files or network, no names starting with `_`, and a time limit of 5 seconds. The builtins are range, len, list, dict, str, int, float, round, min, max, sum, enumerate, zip, sorted, abs, tuple, set, any, all, reversed, bool and isinstance; print does nothing. A call that breaks a rule stops the code with an error.

- `wb[name]` is a sheet; `wb.table(name)` or `table(name)` is a table; `wb.add_sheet(name)` adds an empty sheet at the end.
- `sheet.cell("B7").value` reads or sets a data cell of a table, not in a formula column. `sheet.add_table(name, anchor, columns, rows=None)` adds a table whose header row starts at anchor; columns is a list of {"name", "type", "format"}, with "formula" only for a formula column; rows as in append.
- `t.rows` is the table's rows in order: `row["매출"]` reads or sets a cell, and `row.row` is its sheet row. `t.columns` is the column names.
- `t.append({column: value})` adds one row after the last; columns left out are empty, and formula columns are always left out. `t.insert(before, {…})` adds one row before sheet row `before`. `t.delete_rows(first, last=None)` deletes sheet rows first to last.
- `t.fill_formula(column, formula)` makes the column a formula column; its cells are computed on every row.
- `t.set_type(column, type, format=None)` changes a column's type and format (format optional for text). Values are converted as displayed: 417 in format 00000 becomes the text "00417".
- `t.add_column(name, type, format=None, formula=None)` adds a column after the last; formula only for a formula column.
- `t.sort(keys)` sorts the rows; keys is a list of column names or (column, "desc") pairs.

Values: text is a str, a number an int or float (1204, never "1,204"), a date the str "YYYY-MM-DD" ("YYYY-MM" for the first of the month; dates read back as "YYYY-MM-DD"), and None an empty cell. Row numbers are sheet rows as the workbook is when the call runs.

Example (in the table Sales above, the 매출 of row 3 becomes 15,300,000, and a row is added):

```
wb["매출"].cell("C3").value = 15300000
table("Sales").append({"월": "2026-03", "지점": "강남", "매출": 17200000})
```

## The workbook: 2027 부서별 예산.xlsx

````
<sheet name="예산">

<table name="Budget" range="A1:H90">
| column | type | format | formula |
|---|---|---|---|
| 계정코드 | number | 0000 |  |
| 부서 | text | @ |  |
| 항목 | text | @ |  |
| 1분기 | number | #,##0 |  |
| 2분기 | number | #,##0 |  |
| 3분기 | number | #,##0 |  |
| 4분기 | number | #,##0 |  |
| 합계 | number | #,##0 |  |
</table>

<data table="Budget">
| row | 계정코드 | 부서 | 항목 | 1분기 | 2분기 | 3분기 | 4분기 | 합계 |
|---|---|---|---|---|---|---|---|---|
| 2 | 0411 | 경영지원 | 급여 | 115,310,000 | 130,280,000 | 110,130,000 | 115,740,000 | 471,460,000 |
| 3 | 0421 | 경영지원 | 복리후생비 | 11,690,000 | 13,850,000 | 11,160,000 | 12,330,000 | 49,030,000 |
| 4 | 0431 | 경영지원 | 교육훈련비 | 5,910,000 | 5,550,000 | 5,790,000 | 5,940,000 | 23,190,000 |
| 5 | 0441 | 경영지원 | 여비교통비 | 12,230,000 | 11,550,000 | 11,690,000 | 12,120,000 | 47,590,000 |
| 6 | 0451 | 경영지원 | 통신비 | 5,880,000 | 5,420,000 | 5,530,000 | 4,400,000 | 21,230,000 |
| 7 | 0461 | 경영지원 | 소모품비 | 5,160,000 | 4,770,000 | 5,610,000 | 4,290,000 | 19,830,000 |
| 8 | 0471 | 경영지원 | 지급수수료 | 11,730,000 | 14,390,000 | 14,540,000 | 11,890,000 | 52,550,000 |
| 9 | 0481 | 경영지원 | 광고선전비 | 6,390,000 | 6,140,000 | 6,090,000 | 6,670,000 | 25,290,000 |
| 10 | 0491 | 경영지원 | 차량유지비 | 4,670,000 | 3,990,000 | 3,940,000 | 3,560,000 | 16,160,000 |
| 11 | 0511 | 경영지원 | 임차료 | 40,510,000 | 36,240,000 | 35,800,000 | 36,420,000 | 148,970,000 |
| 12 | 0521 | 경영지원 | 수선비 | 5,030,000 | 5,330,000 | 5,060,000 | 5,000,000 | 20,420,000 |
| 13 | 0531 | 경영지원 | 보험료 | 8,850,000 | 9,350,000 | 9,760,000 | 11,070,000 | 39,030,000 |
| 14 | 0551 | 경영지원 | 회의비 | 2,340,000 | 2,520,000 | 2,110,000 | 2,560,000 | 9,530,000 |
| 15 | 0581 | 경영지원 | 감가상각비 | 17,220,000 | 20,450,000 | 18,460,000 | 17,580,000 | 73,710,000 |
| 16 | 0611 | 경영지원 | 외주가공비 | 13,050,000 | 15,990,000 | 13,870,000 | 16,750,000 | 59,660,000 |
| 17 | 0651 | 경영지원 | 잡비 | 1,420,000 | 1,470,000 | 1,410,000 | 1,390,000 | 5,690,000 |
| 18 |  | 경영지원 | 소계 | 267,390,000 | 287,290,000 | 260,950,000 | 267,710,000 | 1,083,340,000 |
| 19 |  |  |  |  |  |  |  |  |
| 20 | 0411 | 영업 | 급여 | 107,840,000 | 103,480,000 | 110,580,000 | 111,130,000 | 433,030,000 |
| 21 | 0421 | 영업 | 복리후생비 | 16,280,000 | 18,270,000 | 18,950,000 | 15,510,000 | 69,010,000 |
| 22 | 0431 | 영업 | 교육훈련비 | 9,310,000 | 8,980,000 | 7,930,000 | 7,100,000 | 33,320,000 |
| 23 | 0441 | 영업 | 여비교통비 | 17,920,000 | 20,730,000 | 21,920,000 | 22,310,000 | 82,880,000 |
| 24 | 0451 | 영업 | 통신비 | 5,410,000 | 5,410,000 | 6,270,000 | 6,240,000 | 23,330,000 |
| 25 | 0461 | 영업 | 소모품비 | 4,730,000 | 5,390,000 | 5,510,000 | 5,630,000 | 21,260,000 |
| 26 | 0471 | 영업 | 지급수수료 | 14,720,000 | 15,190,000 | 16,350,000 | 13,800,000 | 60,060,000 |
| 27 | 0481 | 영업 | 광고선전비 | 17,510,000 | 18,890,000 | 20,540,000 | 21,820,000 | 78,760,000 |
| 28 | 0491 | 영업 | 차량유지비 | 12,800,000 | 13,700,000 | 11,280,000 | 12,360,000 | 50,140,000 |
| 29 | 0511 | 영업 | 임차료 | 19,230,000 | 15,390,000 | 16,330,000 | 19,160,000 | 70,110,000 |
| 30 | 0521 | 영업 | 수선비 | 8,830,000 | 8,120,000 | 8,150,000 | 7,980,000 | 33,080,000 |
| 31 | 0531 | 영업 | 보험료 | 12,390,000 | 11,750,000 | 10,810,000 | 11,480,000 | 46,430,000 |
| 32 | 0551 | 영업 | 회의비 | 3,110,000 | 3,300,000 | 3,620,000 | 3,520,000 | 13,550,000 |
| 33 | 0581 | 영업 | 감가상각비 | 15,420,000 | 14,270,000 | 11,670,000 | 13,710,000 | 55,070,000 |
| 34 | 0611 | 영업 | 외주가공비 | 11,820,000 | 11,960,000 | 11,420,000 | 11,910,000 | 47,110,000 |
| 35 | 0651 | 영업 | 잡비 | 2,670,000 | 2,570,000 | 2,910,000 | 2,540,000 | 10,690,000 |
| 36 |  | 영업 | 소계 | 279,990,000 | 277,400,000 | 284,240,000 | 286,200,000 | 1,127,830,000 |
| 37 |  |  |  |  |  |  |  |  |
| 38 | 0411 | 개발 | 급여 | 178,920,000 | 176,000,000 | 177,490,000 | 204,610,000 | 737,020,000 |
| 39 | 0421 | 개발 | 복리후생비 | 24,050,000 | 22,250,000 | 18,950,000 | 23,240,000 | 88,490,000 |
| 40 | 0431 | 개발 | 교육훈련비 | 18,200,000 | 17,620,000 | 17,560,000 | 14,490,000 | 67,870,000 |
| 41 | 0441 | 개발 | 여비교통비 | 10,720,000 | 10,250,000 | 10,080,000 | 11,940,000 | 42,990,000 |
| 42 | 0451 | 개발 | 통신비 | 7,490,000 | 6,120,000 | 6,020,000 | 6,410,000 | 26,040,000 |
| 43 | 0461 | 개발 | 소모품비 | 6,170,000 | 6,590,000 | 5,480,000 | 7,200,000 | 25,440,000 |
| 44 | 0471 | 개발 | 지급수수료 | 15,710,000 | 12,510,000 | 15,010,000 | 14,340,000 | 57,570,000 |
| 45 | 0481 | 개발 | 광고선전비 | 2,940,000 | 3,090,000 | 3,280,000 | 3,360,000 | 12,670,000 |
| 46 | 0491 | 개발 | 차량유지비 | 7,310,000 | 6,770,000 | 7,740,000 | 6,470,000 | 28,290,000 |
| 47 | 0511 | 개발 | 임차료 | 23,410,000 | 22,500,000 | 23,220,000 | 22,880,000 | 92,010,000 |
| 48 | 0521 | 개발 | 수선비 | 12,070,000 | 12,600,000 | 11,590,000 | 13,310,000 | 49,570,000 |
| 49 | 0531 | 개발 | 보험료 | 14,340,000 | 16,800,000 | 18,810,000 | 18,830,000 | 68,780,000 |
| 50 | 0551 | 개발 | 회의비 | 4,810,000 | 4,640,000 | 4,250,000 | 4,570,000 | 18,270,000 |
| 51 | 0581 | 개발 | 감가상각비 | 28,390,000 | 31,550,000 | 25,790,000 | 30,670,000 | 116,400,000 |
| 52 | 0611 | 개발 | 외주가공비 | 25,000,000 | 24,400,000 | 23,900,000 | 24,990,000 | 98,290,000 |
| 53 | 0651 | 개발 | 잡비 | 2,510,000 | 2,730,000 | 2,310,000 | 2,600,000 | 10,150,000 |
| 54 |  | 개발 | 소계 | 382,040,000 | 376,420,000 | 371,480,000 | 409,910,000 | 1,539,850,000 |
| 55 |  |  |  |  |  |  |  |  |
| 56 | 0411 | 생산 | 급여 | 182,230,000 | 165,430,000 | 178,000,000 | 152,970,000 | 678,630,000 |
| 57 | 0421 | 생산 | 복리후생비 | 37,490,000 | 30,550,000 | 39,500,000 | 31,220,000 | 138,760,000 |
| 58 | 0431 | 생산 | 교육훈련비 | 14,830,000 | 12,800,000 | 12,790,000 | 13,640,000 | 54,060,000 |
| 59 | 0441 | 생산 | 여비교통비 | 15,390,000 | 14,160,000 | 15,780,000 | 14,290,000 | 59,620,000 |
| 60 | 0451 | 생산 | 통신비 | 7,970,000 | 6,150,000 | 7,070,000 | 7,950,000 | 29,140,000 |
| 61 | 0461 | 생산 | 소모품비 | 8,060,000 | 7,410,000 | 6,150,000 | 8,300,000 | 29,920,000 |
| 62 | 0471 | 생산 | 지급수수료 | 24,600,000 | 28,010,000 | 28,120,000 | 29,790,000 | 110,520,000 |
| 63 | 0481 | 생산 | 광고선전비 | 1,750,000 | 1,720,000 | 1,380,000 | 1,730,000 | 6,580,000 |
| 64 | 0491 | 생산 | 차량유지비 | 8,750,000 | 8,390,000 | 7,660,000 | 8,140,000 | 32,940,000 |
| 65 | 0511 | 생산 | 임차료 | 26,620,000 | 26,690,000 | 26,560,000 | 28,520,000 | 108,390,000 |
| 66 | 0521 | 생산 | 수선비 | 36,130,000 | 33,730,000 | 39,060,000 | 36,070,000 | 145,990,000 |
| 67 | 0531 | 생산 | 보험료 | 20,260,000 | 20,530,000 | 23,200,000 | 23,530,000 | 87,520,000 |
| 68 | 0551 | 생산 | 회의비 | 3,670,000 | 4,290,000 | 4,290,000 | 4,570,000 | 16,820,000 |
| 69 | 0581 | 생산 | 감가상각비 | 31,420,000 | 24,300,000 | 24,440,000 | 27,180,000 | 107,340,000 |
| 70 | 0611 | 생산 | 외주가공비 | 64,980,000 | 59,980,000 | 74,890,000 | 70,230,000 | 270,080,000 |
| 71 | 0651 | 생산 | 잡비 | 3,120,000 | 2,890,000 | 2,960,000 | 3,450,000 | 12,420,000 |
| 72 |  | 생산 | 소계 | 487,270,000 | 447,030,000 | 491,850,000 | 461,580,000 | 1,887,730,000 |
| 73 |  |  |  |  |  |  |  |  |
| 74 | 0411 | 품질 | 급여 | 68,900,000 | 63,030,000 | 77,730,000 | 72,040,000 | 281,700,000 |
| 75 | 0421 | 품질 | 복리후생비 | 12,360,000 | 12,440,000 | 13,000,000 | 11,920,000 | 49,720,000 |
| 76 | 0431 | 품질 | 교육훈련비 | 5,510,000 | 5,810,000 | 5,290,000 | 4,970,000 | 21,580,000 |
| 77 | 0441 | 품질 | 여비교통비 | 6,840,000 | 8,040,000 | 7,290,000 | 6,680,000 | 28,850,000 |
| 78 | 0451 | 품질 | 통신비 | 2,290,000 | 2,420,000 | 2,480,000 | 2,750,000 | 9,940,000 |
| 79 | 0461 | 품질 | 소모품비 | 5,530,000 | 5,720,000 | 5,210,000 | 5,000,000 | 21,460,000 |
| 80 | 0471 | 품질 | 지급수수료 | 9,010,000 | 11,190,000 | 10,960,000 | 11,220,000 | 42,380,000 |
| 81 | 0481 | 품질 | 광고선전비 | 1,000,000 | 900,000 | 800,000 | 770,000 | 3,470,000 |
| 82 | 0491 | 품질 | 차량유지비 | 2,470,000 | 2,590,000 | 2,280,000 | 2,410,000 | 9,750,000 |
| 83 | 0511 | 품질 | 임차료 | 14,270,000 | 15,190,000 | 17,130,000 | 13,180,000 | 59,770,000 |
| 84 | 0521 | 품질 | 수선비 | 7,260,000 | 7,170,000 | 5,910,000 | 7,360,000 | 27,700,000 |
| 85 | 0531 | 품질 | 보험료 | 6,920,000 | 7,180,000 | 6,590,000 | 6,080,000 | 26,770,000 |
| 86 | 0551 | 품질 | 회의비 | 1,960,000 | 1,790,000 | 2,210,000 | 2,010,000 | 7,970,000 |
| 87 | 0581 | 품질 | 감가상각비 | 9,230,000 | 8,750,000 | 6,920,000 | 7,780,000 | 32,680,000 |
| 88 | 0611 | 품질 | 외주가공비 | 6,580,000 | 8,130,000 | 7,180,000 | 7,070,000 | 28,960,000 |
| 89 | 0651 | 품질 | 잡비 | 1,160,000 | 1,220,000 | 1,350,000 | 1,170,000 | 4,900,000 |
| 90 |  | 품질 | 소계 | 161,290,000 | 161,570,000 | 172,330,000 | 162,410,000 | 657,600,000 |
</data>
</sheet>

<sheet name="인건비">

<table name="Payroll" range="A1:E26">
| column | type | format | formula |
|---|---|---|---|
| 부서 | text | @ |  |
| 직급 | text | @ |  |
| 인원 | number | 0 |  |
| 1인당 연봉 | number | #,##0 |  |
| 인건비 | number | #,##0 | =[@인원]*[@[1인당 연봉]] |
</table>

<data table="Payroll">
| row | 부서 | 직급 | 인원 | 1인당 연봉 | 인건비 |
|---|---|---|---|---|---|
| 2 | 경영지원 | 사원 | 14 | 38,000,000 | 532,000,000 |
| 3 | 경영지원 | 대리 | 11 | 46,200,000 | 508,200,000 |
| 4 | 경영지원 | 과장 | 3 | 57,500,000 | 172,500,000 |
| 5 | 경영지원 | 차장 | 2 | 71,800,000 | 143,600,000 |
| 6 | 경영지원 | 부장 | 6 | 89,900,000 | 539,400,000 |
| 7 | 영업 | 사원 | 10 | 36,300,000 | 363,000,000 |
| 8 | 영업 | 대리 | 1 | 49,100,000 | 49,100,000 |
| 9 | 영업 | 과장 | 1 | 60,300,000 | 60,300,000 |
| 10 | 영업 | 차장 | 6 | 67,100,000 | 402,600,000 |
| 11 | 영업 | 부장 | 6 | 88,800,000 | 532,800,000 |
| 12 | 개발 | 사원 | 4 | 36,900,000 | 147,600,000 |
| 13 | 개발 | 대리 | 8 | 46,100,000 | 368,800,000 |
| 14 | 개발 | 과장 | 5 | 56,000,000 | 280,000,000 |
| 15 | 개발 | 차장 | 6 | 67,400,000 | 404,400,000 |
| 16 | 개발 | 부장 | 6 | 90,200,000 | 541,200,000 |
| 17 | 생산 | 사원 | 11 | 36,700,000 | 403,700,000 |
| 18 | 생산 | 대리 | 4 | 49,100,000 | 196,400,000 |
| 19 | 생산 | 과장 | 2 | 60,900,000 | 121,800,000 |
| 20 | 생산 | 차장 | 5 | 74,700,000 | 373,500,000 |
| 21 | 생산 | 부장 | 4 | 87,400,000 | 349,600,000 |
| 22 | 품질 | 사원 | 6 | 38,900,000 | 233,400,000 |
| 23 | 품질 | 대리 | 6 | 47,800,000 | 286,800,000 |
| 24 | 품질 | 과장 | 5 | 57,800,000 | 289,000,000 |
| 25 | 품질 | 차장 | 5 | 74,800,000 | 374,000,000 |
| 26 | 품질 | 부장 | 3 | 89,100,000 | 267,300,000 |
</data>
</sheet>

<sheet name="집행">

<table name="Spend" range="A1:F39">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy-mm |  |
| 계정코드 | text | @ |  |
| 항목 | text | @ |  |
| 부서 | text | @ |  |
| 집행액 | number | #,##0 |  |
| 비고 | text | @ |  |
</table>

<data table="Spend">
| row | 월 | 계정코드 | 항목 | 부서 | 집행액 | 비고 |
|---|---|---|---|---|---|---|
| 2 | 2027-01 | 0521 | 수선비 | 생산 | 11,470,000 | 세금계산서 |
| 3 | 2027-01 | 0421 | 복리후생비 | 개발 | 3,540,000 | 법인카드 |
| 4 | 2027-01 | 0441 | 여비교통비 | 개발 | 8,800,000 |  |
| 5 | 2027-01 | 0441 | 여비교통비 | 영업 | 4,870,000 |  |
| 6 | 2027-01 | 0521 | 수선비 | 경영지원 | 11,400,000 | 계좌이체 |
| 7 | 2027-01 | 0461 | 소모품비 | 개발 | 8,020,000 |  |
| 8 | 2027-01 | 0611 | 외주가공비 | 품질 | 2,750,000 | 법인카드 |
| 9 | 2027-01 | 0431 | 교육훈련비 | 품질 | 4,670,000 | 계좌이체 |
| 10 | 2027-01 | 0611 | 외주가공비 | 영업 | 6,640,000 |  |
| 11 | 2027-01 | 0451 | 통신비 | 생산 | 10,510,000 |  |
| 12 | 2027-01 | 0461 | 소모품비 | 경영지원 | 9,770,000 |  |
| 13 | 2027-01 | 0451 | 통신비 | 개발 | 9,970,000 |  |
| 14 |  |  |  |  |  |  |
| 15 | 2027-02 | 0461 | 소모품비 | 품질 | 8,850,000 | 계좌이체 |
| 16 | 2027-02 | 0471 | 지급수수료 | 영업 | 2,670,000 | 법인카드 |
| 17 | 2027-02 | 0611 | 외주가공비 | 영업 | 720,000 | 법인카드 |
| 18 | 2027-02 | 0421 | 복리후생비 | 생산 | 5,100,000 | 계좌이체 |
| 19 | 2027-02 | 0481 | 광고선전비 | 품질 | 650,000 | 계좌이체 |
| 20 | 2027-02 | 0551 | 회의비 | 경영지원 | 1,730,000 | 법인카드 |
| 21 | 2027-02 | 0491 | 차량유지비 | 품질 | 3,400,000 | 법인카드 |
| 22 | 2027-02 | 0461 | 소모품비 | 영업 | 8,640,000 |  |
| 23 | 2027-02 | 0491 | 차량유지비 | 영업 | 8,100,000 |  |
| 24 | 2027-02 | 0471 | 지급수수료 | 품질 | 7,390,000 | 법인카드 |
| 25 | 2027-02 | 0531 | 보험료 | 품질 | 6,400,000 | 계좌이체 |
| 26 | 2027-02 | 0431 | 교육훈련비 | 품질 | 1,290,000 |  |
| 27 |  |  |  |  |  |  |
| 28 | 2027-03 | 0551 | 회의비 | 경영지원 | 1,650,000 |  |
| 29 | 2027-03 | 0481 | 광고선전비 | 영업 | 9,690,000 | 계좌이체 |
| 30 | 2027-03 | 0431 | 교육훈련비 | 개발 | 9,970,000 | 계좌이체 |
| 31 | 2027-03 | 0451 | 통신비 | 생산 | 2,620,000 | 계좌이체 |
| 32 | 2027-03 | 0431 | 교육훈련비 | 품질 | 6,080,000 | 세금계산서 |
| 33 | 2027-03 | 0441 | 여비교통비 | 생산 | 4,830,000 | 법인카드 |
| 34 | 2027-03 | 0491 | 차량유지비 | 품질 | 2,940,000 |  |
| 35 | 2027-03 | 0421 | 복리후생비 | 개발 | 4,010,000 | 법인카드 |
| 36 | 2027-03 | 0491 | 차량유지비 | 생산 | 11,380,000 | 법인카드 |
| 37 | 2027-03 | 0531 | 보험료 | 개발 | 2,130,000 | 법인카드 |
| 38 | 2027-03 | 0511 | 임차료 | 경영지원 | 9,590,000 | 계좌이체 |
| 39 | 2027-03 | 0521 | 수선비 | 품질 | 2,820,000 |  |
</data>
</sheet>
````

## Tasks

The write task and each edit task are answered with `text`: the Python code that makes the change.

### budget3-w (write)

On sheet 인건비, add a new table named Hiring whose top-left cell is H1, with four columns in this order: 부서 (text), 충원 인원 (a number, format 0), 1인당 연봉 (a number, format #,##0) and 추가 인건비 (a number, format #,##0). 추가 인건비 is a formula column: the row's 충원 인원 times its 1인당 연봉. The table has these rows, in this order: 개발, 3, 52,000,000; 품질, 2, 46,000,000; 영업, 2, 48,500,000. Change nothing else.

### budget3-e1 (edit)

In Budget, the 3분기 amount of 개발's 복리후생비 should be 21,150,000. Change only that cell; 합계 stays as it is.

### budget3-e2 (edit)

Append the 2027-04 spending to Spend, directly after its last row, in this order (계정코드, 항목, 부서, 집행액, 비고): 0441, 여비교통비, 영업, 3,420,000, 법인카드; 0431, 교육훈련비, 개발, 5,800,000, 세금계산서; 0521, 수선비, 생산, 7,150,000, 세금계산서; 0461, 소모품비, 품질, 640,000, (empty); 0551, 회의비, 경영지원, 910,000, 법인카드. Change nothing else.

### budget3-e3 (edit)

In Budget, 합계 was typed by hand and one row is wrong. Make 합계 a formula column: the sum of 1분기 to 4분기, on every row. Change nothing else.

### budget3-e4 (edit)

In Budget, 계정코드 holds account codes but is typed as a number with format 0000. Make it a text column, so that every code keeps its leading zero exactly as displayed now. Change nothing else.

### budget3-e5 (edit)

In Budget, colour every row whose 합계 is over 100,000,000 red, so that the large accounts stand out.

## Tasks that cannot be done

Do only what the documentation above allows. If a task asks for something it cannot express or does not allow, do not approximate it: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its answer.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a task>", "text": "<Python code>"},
  {"task_id": "<id of a task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```
