"""Round 4 guides. Within a decision the shared part is identical and the candidate parts have the same structure:
a definition, the rules as a list, and one example on the same small table. No guide mentions another candidate."""

STRUCTURE = """\
A workbook is shown as text, sheet by sheet. Each sheet is a `<sheet name="…">` … `</sheet>` block holding its tables. Each table first has a `<table name="…" range="…">` … `</table>` block. `range` is the table's area in A1 notation: its first row is the header row (the column names), so the first data row is the next sheet row, and the columns take the range's letters in order. In `range="A1:F153"` the first column is A and the sixth is F, and the data rows are 2 to 153. Each line of the block is one column: its name, its type, its number format and, for a formula column, its formula.

- Types: `text`, `number` or `date`. Every cell of a column holds a value of that type, or is empty.
- Number formats say how a value is displayed: `#,##0` (1234567 displays 1,234,567), `#,##0.0` and `0.0` (one decimal), `0` (a whole number), `0.0%` (0.923 displays 92.3%), `0000`, `00000` and `000000` (a whole number padded with leading zeros: 417 in `00000` displays 00417), `yyyy-mm` (the date 2026-03-01 displays 2026-03), `yyyy-mm-dd`, and `@` for text, displayed as it is.
- A formula column computes every row from one formula written with structured references: `[@매출]` is this row's 매출, and `Sales[매출]` is the whole 매출 column of the table Sales. Its values are computed, never typed.
- A row whose cells are all empty is a blank row; its formula cells are empty too. Tables often hold blank rows between blocks, and subtotal rows (소계) written like data rows."""

WINDOW = """\
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

Here row 4 is blank, and the 매출 of 서초 in 2026-01 is 9,500,000, in cell C3."""

INDEX = """\
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

Here row 4 is blank, and the 매출 of 서초 in 2026-01 is 9,500,000 (as displayed), in cell C3."""

RULES = """\
### Rules for every change

- Types are never guessed from how a value looks: IDs, codes and phone numbers are text, even when they hold only digits. When a column's type changes, its values are converted as they are displayed.
- A formula column's cells are computed from its formula on every row; they are never set one by one. Formulas use only structured references: `[@Column]` or `[@[Column name]]` for a cell of this row, `[@[First]:[Last]]` for this row's cells from one column to another (inside SUM), and `Table[Column]` for a whole column of a table. The functions are SUM, SUMIFS, COUNTIFS, AVERAGE, MIN, MAX, ROUND, ABS, IF and IFERROR. A1 references such as D2, $D$2, D2:D9 or 매출!D2 are not allowed in formulas.
- A value that starts with `=` is not accepted: a formula belongs to a formula column.
- Functions that fetch data (WEBSERVICE, FILTERXML, HYPERLINK and the like) are not allowed.
- Formatting is by type and number format only, from the formats listed above; colours, fonts, fills, borders and column widths cannot be set.
- No answer adds more than 50 rows in total: bulk rows come from an import, not from rows you type or generate.
- A table name is letters, digits, `_` and `.`, starting with a letter, and unique in the workbook. Tables on one sheet never overlap."""

WRITE_A = """\
### Writing changes: range operations

Answer a write or edit task with `text`: a JSON list of operations, applied in order. There are exactly ten operations: objects with `"op"` and the keys shown, all required unless marked optional:

- `{"op": "set", "range": "Sheet1!B7", "values": [[1204]]}`: set cells. `range` is Sheet!Cell or Sheet!First:Last, `values` a list of rows of values in its shape; each cell is a table's data cell, not in a formula column.
- `{"op": "append_rows", "table": T, "rows": [{"월": "2026-04", "매출": 9800000}]}`: add rows after the last row. A row is {column: value}; columns left out are empty, and formula columns are always left out.
- `{"op": "insert_rows", "table": T, "before": 19, "rows": [...]}`: add rows before sheet row 19, a data row of T or the row after its last.
- `{"op": "delete_rows", "table": T, "rows": "18:19"}`: delete sheet rows 18 to 19 (`"18"` for one row).
- `{"op": "fill_formula", "table": T, "column": C, "formula": "=[@매출]-[@원가]"}`: make C a formula column; its cells are computed on every row.
- `{"op": "set_type", "table": T, "column": C, "type": "text", "format": "@"}`: change a column's type and format (format optional for text). Values are converted as displayed: 417 in format 00000 becomes the text "00417".
- `{"op": "add_column", "table": T, "column": {"name": C, "type": "text", "format": "@"}}`: add a column after the last; `"formula"` only for a formula column.
- `{"op": "sort", "table": T, "keys": [{"column": C, "order": "asc"}]}`: sort the rows; order is "asc" or "desc".
- `{"op": "add_table", "sheet": S, "name": N, "anchor": "H1", "columns": [{"name": C, "type": "number", "format": "#,##0"}], "rows": [...]}`: add a table with its header row at anchor; a formula column also has `"formula"`; `"rows"` optional, as in append_rows.
- `{"op": "add_sheet", "name": S}`: add an empty sheet at the end.

Values are JSON: text is a string, a number is a JSON number (1204, never "1,204"), a date is "YYYY-MM-DD" ("YYYY-MM" for the first of the month), and null is an empty cell. Row numbers are sheet rows at the time the operation runs, after the earlier ones.

Example (in the table Sales above, the 매출 of row 3 becomes 15,300,000, and a row is added):

```
[{"op": "set", "range": "매출!C3", "values": [[15300000]]},
 {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-03", "지점": "강남", "매출": 17200000}]}]
```"""

WRITE_B = """\
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
 {"old": "| 5 | 2026-02 | 강남 | 11,200,000 |\\n</data>", "new": "| 5 | 2026-02 | 강남 | 11,200,000 |\\n|  | 2026-03 | 강남 | 17,200,000 |\\n</data>"}]
```"""

WRITE_C = """\
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
```"""

GUIDES = {
    ('readview', 'A'): STRUCTURE + '\n\n' + WINDOW,
    ('readview', 'B'): STRUCTURE + '\n\n' + INDEX,
    ('writeshape', 'A'): STRUCTURE + '\n\n' + WINDOW + '\n\n' + RULES + '\n\n' + WRITE_A,
    ('writeshape', 'B'): STRUCTURE + '\n\n' + WINDOW + '\n\n' + RULES + '\n\n' + WRITE_B,
    ('writeshape', 'C'): STRUCTURE + '\n\n' + WINDOW + '\n\n' + RULES + '\n\n' + WRITE_C,
}
CANDIDATE_PART = {('readview', 'A'): WINDOW, ('readview', 'B'): INDEX,
                  ('writeshape', 'A'): WRITE_A, ('writeshape', 'B'): WRITE_B, ('writeshape', 'C'): WRITE_C}
