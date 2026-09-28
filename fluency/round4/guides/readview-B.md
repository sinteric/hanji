A workbook is shown as text, sheet by sheet. Each sheet is a `<sheet name="…">` … `</sheet>` block holding its tables. Each table first has a `<table name="…" range="…">` … `</table>` block. `range` is the table's area in A1 notation: its first row is the header row (the column names), so the first data row is the next sheet row, and the columns take the range's letters in order. In `range="A1:F153"` the first column is A and the sixth is F, and the data rows are 2 to 153. Each line of the block is one column: its name, its type, its number format and, for a formula column, its formula.

- Types: `text`, `number` or `date`. Every cell of a column holds a value of that type, or is empty.
- Number formats say how a value is displayed: `#,##0` (1234567 displays 1,234,567), `#,##0.0` and `0.0` (one decimal), `0` (a whole number), `0.0%` (0.923 displays 92.3%), `0000`, `00000` and `000000` (a whole number padded with leading zeros: 417 in `00000` displays 00417), `yyyy-mm` (the date 2026-03-01 displays 2026-03), `yyyy-mm-dd`, and `@` for text, displayed as it is.
- A formula column computes every row from one formula written with structured references: `[@매출]` is this row's 매출, and `Sales[매출]` is the whole 매출 column of the table Sales. Its values are computed, never typed.
- A row whose cells are all empty is a blank row; its formula cells are empty too. Tables often hold blank rows between blocks, and subtotal rows (소계) written like data rows.

### Cell data

After its `<table>` block, each table's cells follow in an `<index table="…" rows="…" blank="…">` … `</index>` block. `rows` gives the table's data rows as first:last, and `blank` its blank rows (left out if none). Then one line per column: its letter and name, then its entries, each after ` | `.

- An entry is `value: rows`: one distinct value of the column, then every sheet row where the column holds it. A run of consecutive rows is written first:last; separate rows and runs are separated by commas.
- Entries are in the order their values first appear going down. An empty cell is not listed: a row missing from a column's line is empty in that column.
- Values are raw: text as it is, dates as yyyy-mm-dd, numbers without digit grouping, rounded as their format displays them (`0.0%` keeps three decimals: 0.923). The format in the `<table>` block says how a value displays: 12000000 in `#,##0` is 12,000,000, and 417 in `00000` is 00417.
- A cell's address is its column's letter and the row number.

Example (a table Sales at A1 with the columns 월, 지점 and 매출):

```
<index table="Sales" rows="2:5" blank="4">
A 월 | 2026-01-01: 2:3 | 2026-02-01: 5
B 지점 | 강남: 2,5 | 서초: 3
C 매출 | 12000000: 2 | 9500000: 3 | 11200000: 5
</index>
```

Here row 4 is blank, and the 매출 of 서초 in 2026-01 is 9,500,000 (as displayed), in cell C3.
