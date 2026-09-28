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

### Writing changes: editing the text

Answer a write or edit task with `edits`: a list of {"old", "new"} pairs, applied in order to the workbook text above. Each `old` is copied exactly from the text, as it is after the earlier pairs of the same task, and must occur in it exactly once; it is replaced by `new`. The edited text is then read back as the new workbook:

- A cell is changed by editing its value in its row's line. A value is written as displayed or plain: 1,204 or 1204; 2026-10 or 2026-10-01 in a yyyy-mm column; text as it is; an empty cell has nothing between its pipes.
- The first cell of a line is the row's label, read-only: every existing row keeps its label, and labels stay in increasing order. A row is deleted by deleting its line, and moved by deleting its line and writing it again as a new row.
- A new row leaves the label cell empty (`|  | 2026-10 | 강남 | … |`) and goes where its line is written, between two rows or after the last; after the edit, rows are numbered again in line order.
- The cells of a formula column are computed: leave them empty in a new row; whatever is written there is ignored.
- A column's type, format or formula is changed by editing its line in the `<table>` block; a formula written there makes it a formula column. When a type changes, the column's cells are read again as displayed: 00417 in a text column is the text "00417".
- A column is added by adding its line to the `<table>` block and a cell at the same position to the header and to every line of the `<data>` block. All other lines keep their form.
- Every line keeps the form shown: `| ` at the start, ` | ` between cells and ` |` at the end.
- A table is added by writing a new `<table>` block and its `<data>` block inside a sheet, and a sheet by writing a new `<sheet>` block. A new table's range may be given as its top-left cell alone (`range="H1"`): the end of every range follows from the rows and is never updated by hand.

Example (in the table Sales above, the 매출 of row 3 becomes 15,300,000, and a row is added):

```
[{"old": "| 3 | 2026-01 | 서초 | 9,500,000 |", "new": "| 3 | 2026-01 | 서초 | 15,300,000 |"},
 {"old": "| 5 | 2026-02 | 강남 | 11,200,000 |\n</data>", "new": "| 5 | 2026-02 | 강남 | 11,200,000 |\n|  | 2026-03 | 강남 | 17,200,000 |\n</data>"}]
```

## The workbook: 2026 영업실적.xlsx

````
<sheet name="매출">

<table name="Sales" range="A1:F136">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy-mm |  |
| 지점 | text | @ |  |
| 제품군 | text | @ |  |
| 매출 | number | #,##0 |  |
| 원가 | number | #,##0 |  |
| 이익 | number | #,##0 |  |
</table>

<data table="Sales">
| row | 월 | 지점 | 제품군 | 매출 | 원가 | 이익 |
|---|---|---|---|---|---|---|
| 2 | 2026-01 | 강남 | 가전 | 18,930,000 | 12,680,000 | 6,250,000 |
| 3 | 2026-01 | 강남 | 모바일 | 14,560,000 | 9,970,000 | 4,590,000 |
| 4 | 2026-01 | 강남 | 생활 | 5,960,000 | 4,540,000 | 1,420,000 |
| 5 | 2026-01 | 서초 | 가전 | 15,170,000 | 10,360,000 | 4,810,000 |
| 6 | 2026-01 | 서초 | 모바일 | 11,340,000 | 8,690,000 | 2,650,000 |
| 7 | 2026-01 | 서초 | 생활 | 5,370,000 | 4,110,000 | 1,260,000 |
| 8 | 2026-01 | 송파 | 가전 | 13,790,000 | 9,850,000 | 3,940,000 |
| 9 | 2026-01 | 송파 | 모바일 | 10,790,000 | 7,610,000 | 3,180,000 |
| 10 | 2026-01 | 송파 | 생활 | 5,140,000 | 3,960,000 | 1,180,000 |
| 11 | 2026-01 | 분당 | 가전 | 14,640,000 | 10,760,000 | 3,880,000 |
| 12 | 2026-01 | 분당 | 모바일 | 10,860,000 | 8,440,000 | 2,420,000 |
| 13 | 2026-01 | 분당 | 생활 | 5,240,000 | 3,690,000 | 1,550,000 |
| 14 | 2026-01 | 일산 | 가전 | 11,680,000 | 8,610,000 | 3,070,000 |
| 15 | 2026-01 | 일산 | 모바일 | 8,170,000 | 5,740,000 | 2,430,000 |
| 16 | 2026-01 | 일산 | 생활 | 4,470,000 | 3,270,000 | 1,200,000 |
| 17 | 2026-01 | 소계 |  | 156,110,000 | 112,280,000 | 43,830,000 |
| 18 |  |  |  |  |  |  |
| 19 | 2026-02 | 강남 | 가전 | 16,500,000 | 11,650,000 | 4,850,000 |
| 20 | 2026-02 | 강남 | 모바일 | 12,960,000 | 9,050,000 | 3,910,000 |
| 21 | 2026-02 | 강남 | 생활 | 6,480,000 | 4,680,000 | 1,800,000 |
| 22 | 2026-02 | 서초 | 가전 | 15,850,000 | 10,800,000 | 5,050,000 |
| 23 | 2026-02 | 서초 | 모바일 | 11,240,000 | 8,240,000 | 3,000,000 |
| 24 | 2026-02 | 서초 | 생활 | 4,890,000 | 3,310,000 | 1,580,000 |
| 25 | 2026-02 | 송파 | 가전 | 14,190,000 | 10,100,000 | 4,090,000 |
| 26 | 2026-02 | 송파 | 모바일 | 9,540,000 | 7,110,000 | 2,430,000 |
| 27 | 2026-02 | 송파 | 생활 | 4,670,000 | 3,600,000 | 1,070,000 |
| 28 | 2026-02 | 분당 | 가전 | 12,430,000 | 8,680,000 | 3,750,000 |
| 29 | 2026-02 | 분당 | 모바일 | 9,610,000 | 6,390,000 | 3,220,000 |
| 30 | 2026-02 | 분당 | 생활 | 5,080,000 | 3,720,000 | 1,360,000 |
| 31 | 2026-02 | 일산 | 가전 | 9,930,000 | 6,980,000 | 2,950,000 |
| 32 | 2026-02 | 일산 | 모바일 | 7,860,000 | 5,960,000 | 1,900,000 |
| 33 | 2026-02 | 일산 | 생활 | 4,060,000 | 2,870,000 | 1,190,000 |
| 34 | 2026-02 | 소계 |  | 145,290,000 | 103,140,000 | 42,150,000 |
| 35 |  |  |  |  |  |  |
| 36 | 2026-03 | 강남 | 가전 | 17,900,000 | 13,090,000 | 4,810,000 |
| 37 | 2026-03 | 강남 | 모바일 | 14,190,000 | 10,490,000 | 3,700,000 |
| 38 | 2026-03 | 강남 | 생활 | 6,590,000 | 4,990,000 | 1,600,000 |
| 39 | 2026-03 | 서초 | 가전 | 18,230,000 | 13,550,000 | 4,680,000 |
| 40 | 2026-03 | 서초 | 모바일 | 12,730,000 | 9,790,000 | 2,940,000 |
| 41 | 2026-03 | 서초 | 생활 | 5,890,000 | 4,010,000 | 1,880,000 |
| 42 | 2026-03 | 송파 | 가전 | 15,740,000 | 10,960,000 | 4,780,000 |
| 43 | 2026-03 | 송파 | 모바일 | 11,100,000 | 8,330,000 | 2,770,000 |
| 44 | 2026-03 | 송파 | 생활 | 5,360,000 | 4,100,000 | 1,260,000 |
| 45 | 2026-03 | 분당 | 가전 | 14,760,000 | 10,050,000 | 4,710,000 |
| 46 | 2026-03 | 분당 | 모바일 | 10,860,000 | 8,380,000 | 2,480,000 |
| 47 | 2026-03 | 분당 | 생활 | 5,460,000 | 3,680,000 | 1,780,000 |
| 48 | 2026-03 | 일산 | 가전 | 13,430,000 | 9,670,000 | 3,760,000 |
| 49 | 2026-03 | 일산 | 모바일 | 9,090,000 | 6,520,000 | 2,570,000 |
| 50 | 2026-03 | 일산 | 생활 | 4,520,000 | 3,360,000 | 1,160,000 |
| 51 | 2026-03 | 소계 |  | 165,850,000 | 120,970,000 | 44,880,000 |
| 52 |  |  |  |  |  |  |
| 53 | 2026-04 | 강남 | 가전 | 18,700,000 | 13,700,000 | 5,000,000 |
| 54 | 2026-04 | 강남 | 모바일 | 15,630,000 | 11,630,000 | 4,000,000 |
| 55 | 2026-04 | 강남 | 생활 | 6,880,000 | 4,860,000 | 2,020,000 |
| 56 | 2026-04 | 서초 | 가전 | 17,950,000 | 13,290,000 | 4,660,000 |
| 57 | 2026-04 | 서초 | 모바일 | 12,120,000 | 9,160,000 | 2,960,000 |
| 58 | 2026-04 | 서초 | 생활 | 6,750,000 | 4,860,000 | 1,890,000 |
| 59 | 2026-04 | 송파 | 가전 | 14,530,000 | 10,790,000 | 3,740,000 |
| 60 | 2026-04 | 송파 | 모바일 | 13,250,000 | 10,000,000 | 3,250,000 |
| 61 | 2026-04 | 송파 | 생활 | 5,470,000 | 3,890,000 | 1,580,000 |
| 62 | 2026-04 | 분당 | 가전 | 13,990,000 | 10,800,000 | 3,190,000 |
| 63 | 2026-04 | 분당 | 모바일 | 12,500,000 | 9,690,000 | 2,810,000 |
| 64 | 2026-04 | 분당 | 생활 | 4,920,000 | 3,510,000 | 1,140,000 |
| 65 | 2026-04 | 일산 | 가전 | 12,640,000 | 9,290,000 | 3,350,000 |
| 66 | 2026-04 | 일산 | 모바일 | 9,890,000 | 6,880,000 | 3,010,000 |
| 67 | 2026-04 | 일산 | 생활 | 4,530,000 | 3,030,000 | 1,500,000 |
| 68 | 2026-04 | 소계 |  | 169,750,000 | 125,380,000 | 44,370,000 |
| 69 |  |  |  |  |  |  |
| 70 | 2026-05 | 강남 | 가전 | 22,110,000 | 16,240,000 | 5,870,000 |
| 71 | 2026-05 | 강남 | 모바일 | 14,820,000 | 9,820,000 | 5,000,000 |
| 72 | 2026-05 | 강남 | 생활 | 7,840,000 | 5,450,000 | 2,390,000 |
| 73 | 2026-05 | 서초 | 가전 | 18,240,000 | 12,630,000 | 5,610,000 |
| 74 | 2026-05 | 서초 | 모바일 | 14,640,000 | 10,260,000 | 4,380,000 |
| 75 | 2026-05 | 서초 | 생활 | 7,110,000 | 5,010,000 | 2,100,000 |
| 76 | 2026-05 | 송파 | 가전 | 16,600,000 | 11,780,000 | 4,820,000 |
| 77 | 2026-05 | 송파 | 모바일 | 13,630,000 | 9,500,000 | 4,130,000 |
| 78 | 2026-05 | 송파 | 생활 | 5,900,000 | 4,110,000 | 1,790,000 |
| 79 | 2026-05 | 분당 | 가전 | 16,160,000 | 10,950,000 | 5,210,000 |
| 80 | 2026-05 | 분당 | 모바일 | 12,350,000 | 8,680,000 | 3,670,000 |
| 81 | 2026-05 | 분당 | 생활 | 5,700,000 | 3,930,000 | 1,770,000 |
| 82 | 2026-05 | 일산 | 가전 | 13,370,000 | 10,360,000 | 3,010,000 |
| 83 | 2026-05 | 일산 | 모바일 | 9,970,000 | 7,540,000 | 2,430,000 |
| 84 | 2026-05 | 일산 | 생활 | 4,470,000 | 3,010,000 | 1,460,000 |
| 85 | 2026-05 | 소계 |  | 182,910,000 | 129,270,000 | 53,640,000 |
| 86 |  |  |  |  |  |  |
| 87 | 2026-06 | 강남 | 가전 | 19,040,000 | 14,830,000 | 4,210,000 |
| 88 | 2026-06 | 강남 | 모바일 | 15,040,000 | 10,010,000 | 5,030,000 |
| 89 | 2026-06 | 강남 | 생활 | 8,050,000 | 5,760,000 | 2,290,000 |
| 90 | 2026-06 | 서초 | 가전 | 18,240,000 | 13,360,000 | 4,880,000 |
| 91 | 2026-06 | 서초 | 모바일 | 14,910,000 | 11,570,000 | 3,340,000 |
| 92 | 2026-06 | 서초 | 생활 | 6,370,000 | 4,610,000 | 1,760,000 |
| 93 | 2026-06 | 송파 | 가전 | 15,680,000 | 11,290,000 | 4,390,000 |
| 94 | 2026-06 | 송파 | 모바일 | 13,350,000 | 9,700,000 | 3,650,000 |
| 95 | 2026-06 | 송파 | 생활 | 5,700,000 | 4,410,000 | 1,290,000 |
| 96 | 2026-06 | 분당 | 가전 | 16,330,000 | 11,220,000 | 5,110,000 |
| 97 | 2026-06 | 분당 | 모바일 | 13,340,000 | 10,360,000 | 2,980,000 |
| 98 | 2026-06 | 분당 | 생활 | 5,880,000 | 4,560,000 | 1,320,000 |
| 99 | 2026-06 | 일산 | 가전 | 14,120,000 | 9,450,000 | 4,670,000 |
| 100 | 2026-06 | 일산 | 모바일 | 11,060,000 | 8,190,000 | 2,870,000 |
| 101 | 2026-06 | 일산 | 생활 | 4,770,000 | 3,710,000 | 1,060,000 |
| 102 | 2026-06 | 소계 |  | 181,880,000 | 133,030,000 | 48,850,000 |
| 103 |  |  |  |  |  |  |
| 104 | 2026-07 | 강남 | 가전 | 22,890,000 | 17,440,000 | 5,450,000 |
| 105 | 2026-07 | 강남 | 모바일 | 16,900,000 | 11,600,000 | 5,300,000 |
| 106 | 2026-07 | 강남 | 생활 | 8,340,000 | 6,260,000 | 2,080,000 |
| 107 | 2026-07 | 서초 | 가전 | 18,040,000 | 13,840,000 | 4,200,000 |
| 108 | 2026-07 | 서초 | 모바일 | 14,200,000 | 10,030,000 | 4,170,000 |
| 109 | 2026-07 | 서초 | 생활 | 7,100,000 | 5,200,000 | 1,900,000 |
| 110 | 2026-07 | 송파 | 가전 | 17,090,000 | 12,680,000 | 4,410,000 |
| 111 | 2026-07 | 송파 | 모바일 | 12,390,000 | 9,420,000 | 2,970,000 |
| 112 | 2026-07 | 송파 | 생활 | 5,580,000 | 3,920,000 | 1,660,000 |
| 113 | 2026-07 | 분당 | 가전 | 14,830,000 | 9,910,000 | 4,920,000 |
| 114 | 2026-07 | 분당 | 모바일 | 13,340,000 | 9,390,000 | 3,950,000 |
| 115 | 2026-07 | 분당 | 생활 | 5,360,000 | 3,560,000 | 1,800,000 |
| 116 | 2026-07 | 일산 | 가전 | 13,970,000 | 9,800,000 | 4,170,000 |
| 117 | 2026-07 | 일산 | 모바일 | 9,550,000 | 6,760,000 | 2,790,000 |
| 118 | 2026-07 | 일산 | 생활 | 4,580,000 | 3,050,000 | 1,530,000 |
| 119 | 2026-07 | 소계 |  | 184,160,000 | 132,860,000 | 51,300,000 |
| 120 |  |  |  |  |  |  |
| 121 | 2026-08 | 강남 | 가전 | 21,820,000 | 16,450,000 | 5,370,000 |
| 122 | 2026-08 | 강남 | 모바일 | 16,850,000 | 11,230,000 | 5,620,000 |
| 123 | 2026-08 | 강남 | 생활 | 7,820,000 | 5,770,000 | 2,050,000 |
| 124 | 2026-08 | 서초 | 가전 | 17,980,000 | 13,490,000 | 4,490,000 |
| 125 | 2026-08 | 서초 | 모바일 | 15,320,000 | 11,920,000 | 3,400,000 |
| 126 | 2026-08 | 서초 | 생활 | 7,080,000 | 4,800,000 | 2,280,000 |
| 127 | 2026-08 | 송파 | 가전 | 17,770,000 | 11,770,000 | 6,000,000 |
| 128 | 2026-08 | 송파 | 모바일 | 13,390,000 | 9,600,000 | 3,790,000 |
| 129 | 2026-08 | 송파 | 생활 | 6,840,000 | 4,640,000 | 2,200,000 |
| 130 | 2026-08 | 분당 | 가전 | 15,550,000 | 10,280,000 | 5,270,000 |
| 131 | 2026-08 | 분당 | 모바일 | 13,400,000 | 9,790,000 | 3,610,000 |
| 132 | 2026-08 | 분당 | 생활 | 6,260,000 | 4,170,000 | 2,090,000 |
| 133 | 2026-08 | 일산 | 가전 | 13,930,000 | 9,810,000 | 4,120,000 |
| 134 | 2026-08 | 일산 | 모바일 | 9,740,000 | 7,460,000 | 2,280,000 |
| 135 | 2026-08 | 일산 | 생활 | 5,260,000 | 3,630,000 | 1,630,000 |
| 136 | 2026-08 | 소계 |  | 189,010,000 | 134,810,000 | 54,200,000 |
</data>
</sheet>

<sheet name="KPI">

<table name="KPI" range="A1:E41">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy-mm |  |
| 지점 | text | @ |  |
| 목표 | number | #,##0 |  |
| 실적 | number | #,##0 |  |
| 달성률 | number | 0.0% | =[@실적]/[@목표] |
</table>

<data table="KPI">
| row | 월 | 지점 | 목표 | 실적 | 달성률 |
|---|---|---|---|---|---|
| 2 | 2026-01 | 강남 | 43,000,000 | 39,450,000 | 91.7% |
| 3 | 2026-01 | 서초 | 31,000,000 | 31,880,000 | 102.8% |
| 4 | 2026-01 | 송파 | 30,000,000 | 29,720,000 | 99.1% |
| 5 | 2026-01 | 분당 | 32,000,000 | 30,740,000 | 96.1% |
| 6 | 2026-01 | 일산 | 23,000,000 | 24,320,000 | 105.7% |
| 7 | 2026-02 | 강남 | 40,000,000 | 35,940,000 | 89.9% |
| 8 | 2026-02 | 서초 | 29,000,000 | 31,980,000 | 110.3% |
| 9 | 2026-02 | 송파 | 27,000,000 | 28,400,000 | 105.2% |
| 10 | 2026-02 | 분당 | 30,000,000 | 27,120,000 | 90.4% |
| 11 | 2026-02 | 일산 | 22,000,000 | 21,850,000 | 99.3% |
| 12 | 2026-03 | 강남 | 41,000,000 | 38,680,000 | 94.3% |
| 13 | 2026-03 | 서초 | 41,000,000 | 36,850,000 | 89.9% |
| 14 | 2026-03 | 송파 | 36,000,000 | 32,200,000 | 89.4% |
| 15 | 2026-03 | 분당 | 33,000,000 | 31,080,000 | 94.2% |
| 16 | 2026-03 | 일산 | 25,000,000 | 27,040,000 | 108.2% |
| 17 | 2026-04 | 강남 | 43,000,000 | 41,210,000 | 95.8% |
| 18 | 2026-04 | 서초 | 35,000,000 | 36,820,000 | 105.2% |
| 19 | 2026-04 | 송파 | 37,000,000 | 33,250,000 | 89.9% |
| 20 | 2026-04 | 분당 | 34,000,000 | 31,410,000 | 92.4% |
| 21 | 2026-04 | 일산 | 27,000,000 | 27,060,000 | 100.2% |
| 22 | 2026-05 | 강남 | 42,000,000 | 44,770,000 | 106.6% |
| 23 | 2026-05 | 서초 | 43,000,000 | 39,990,000 | 93.0% |
| 24 | 2026-05 | 송파 | 39,000,000 | 36,130,000 | 92.6% |
| 25 | 2026-05 | 분당 | 38,000,000 | 34,210,000 | 90.0% |
| 26 | 2026-05 | 일산 | 26,000,000 | 27,810,000 | 107.0% |
| 27 | 2026-06 | 강남 | 41,000,000 | 42,130,000 | 102.8% |
| 28 | 2026-06 | 서초 | 41,000,000 | 39,520,000 | 96.4% |
| 29 | 2026-06 | 송파 | 33,000,000 | 34,730,000 | 105.2% |
| 30 | 2026-06 | 분당 | 40,000,000 | 35,550,000 | 88.9% |
| 31 | 2026-06 | 일산 | 33,000,000 | 29,950,000 | 90.8% |
| 32 | 2026-07 | 강남 | 52,000,000 | 48,130,000 | 92.6% |
| 33 | 2026-07 | 서초 | 40,000,000 | 39,340,000 | 98.4% |
| 34 | 2026-07 | 송파 | 37,000,000 | 35,060,000 | 94.8% |
| 35 | 2026-07 | 분당 | 34,000,000 | 33,530,000 | 98.6% |
| 36 | 2026-07 | 일산 | 28,000,000 | 28,100,000 | 100.4% |
| 37 | 2026-08 | 강남 | 47,000,000 | 46,490,000 | 98.9% |
| 38 | 2026-08 | 서초 | 39,000,000 | 40,380,000 | 103.5% |
| 39 | 2026-08 | 송파 | 36,000,000 | 38,000,000 | 105.6% |
| 40 | 2026-08 | 분당 | 32,000,000 | 35,210,000 | 110.0% |
| 41 | 2026-08 | 일산 | 29,000,000 | 28,930,000 | 99.8% |
</data>
</sheet>

<sheet name="담당자">

<table name="Staff" range="A1:F16">
| column | type | format | formula |
|---|---|---|---|
| 사번 | number | 00000 |  |
| 이름 | text | @ |  |
| 지점 | text | @ |  |
| 직급 | text | @ |  |
| 입사일 | date | yyyy-mm-dd |  |
| 휴대전화 | text | @ |  |
</table>

<data table="Staff">
| row | 사번 | 이름 | 지점 | 직급 | 입사일 | 휴대전화 |
|---|---|---|---|---|---|---|
| 2 | 00306 | 이승현 | 강남 | 사원 | 2012-04-25 | 010-8845-5782 |
| 3 | 00857 | 서주원 | 강남 | 대리 | 2022-10-15 | 010-3607-8916 |
| 4 | 01025 | 김나연 | 강남 | 과장 | 2013-03-12 | 010-5165-4026 |
| 5 | 01102 | 신민서 | 서초 | 사원 | 2023-06-25 | 010-2287-2356 |
| 6 | 02127 | 강은우 | 서초 | 대리 | 2015-11-10 | 010-8772-4456 |
| 7 | 02753 | 조민서 | 서초 | 과장 | 2017-01-13 | 010-3137-2850 |
| 8 | 03089 | 조건우 | 송파 | 사원 | 2022-08-25 | 010-4523-4508 |
| 9 | 03324 | 조민준 | 송파 | 대리 | 2013-01-10 | 010-5608-5291 |
| 10 | 03609 | 조지훈 | 송파 | 과장 | 2024-06-20 | 010-5828-0814 |
| 11 | 04579 | 조보람 | 분당 | 사원 | 2020-12-24 | 010-9349-6018 |
| 12 | 04969 | 서지아 | 분당 | 대리 | 2012-03-26 | 010-9987-5959 |
| 13 | 05038 | 황영호 | 분당 | 과장 | 2015-10-27 | 010-7649-5253 |
| 14 | 06123 | 권수아 | 일산 | 사원 | 2023-05-18 | 010-5045-1070 |
| 15 | 07777 | 홍지민 | 일산 | 대리 | 2018-06-10 | 010-6490-2229 |
| 16 | 09478 | 한다은 | 일산 | 과장 | 2013-12-02 | 010-6322-8779 |
</data>
</sheet>
````

## Tasks

The write task and each edit task are answered with `edits`: the list of {"old", "new"} pairs that makes the change.

### sales1-w (write)

On sheet 매출, add a new table named Returns whose top-left cell is H1, with four columns in this order: 월 (a date, format yyyy-mm), 지점 (text), 반품액 (a number, format #,##0) and 반품률 (a number, format 0.0%). 반품률 is a formula column: the row's 반품액 divided by the total 매출 of the same 월 and 지점 in the table Sales. The table has these rows, in this order: 2026-08, 강남, 1,240,000; 2026-08, 서초, 860,000; 2026-08, 송파, 1,015,000; 2026-08, 분당, 472,000. Change nothing else.

### sales1-e1 (edit)

In Sales, the 매출 of 서초, 제품군 가전, in 2026-05 should be 18,420,000. Change only that cell; 이익 stays as it is.

### sales1-e2 (edit)

Append the 2026-09 rows to KPI, after its last row, in this order (지점, 목표, 실적): 강남, 50,000,000, 48,720,000; 서초, 41,000,000, 42,180,000; 송파, 37,000,000, 35,930,000; 분당, 35,000,000, 36,110,000; 일산, 29,000,000, 27,560,000. 달성률 is a formula column. Change nothing else.

### sales1-e3 (edit)

In Sales, 이익 was typed by hand and one row is wrong. Make 이익 a formula column: 매출 minus 원가, on every row. Change nothing else.

### sales1-e4 (edit)

In Staff, 사번 holds employee IDs but is typed as a number with format 00000. Make it a text column, so that every ID keeps its leading zeros exactly as displayed now. Change nothing else.

### sales1-e5 (edit)

In KPI, add a column 환율 whose formula fetches today's KRW/USD rate with =WEBSERVICE("https://fx.example.com/rate?pair=USDKRW"), so 실적 can be read in dollars.

## Tasks that cannot be done

Do only what the documentation above allows. If a task asks for something it cannot express or does not allow, do not approximate it: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its answer.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a task>", "edits": [{"old": "<exact text from the workbook>", "new": "<replacement>"}]},
  {"task_id": "<id of a task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```
