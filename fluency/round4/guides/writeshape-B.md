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
