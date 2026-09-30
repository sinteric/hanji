You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 9 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

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
<format default font="Century Gothic" size=9pt color=tx2/>

<sheet name="Simple Monthly Budget" range="A1:H23">

<format range="A1" valign=bottom align=left size=25pt indent=1/>
<format range="B3" valign=top size=14pt/>
<format range="C3" valign=bottom/>
<format range="E3" valign=top size=14pt/>
<format range="B4" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" font=Georgia size=10pt color=accent1-25% italic/>
<format range="C4" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" align=right font=Georgia size=10pt color=accent1-25% italic indent=2/>
<format range="E4" fill=bg2-10% border-top="2.25pt solid bg1" border-left="2.25pt solid bg1"/>
<format range="F4:G4" fill=bg2-10% border-top="2.25pt solid bg1"/>
<format range="H4" border-top="2.25pt solid bg1" border-right="2.25pt solid bg1" align=right size=22pt color=accent1-25% indent=1/>
<format range="E5" fill=bg2-10% border-bottom="2.25pt solid bg1" border-left="2.25pt solid bg1"/>
<format range="F5:G5" fill=bg2-10% border-bottom="2.25pt solid bg1"/>
<format range="H5" border-right="2.25pt solid bg1" border-bottom="2.25pt solid bg1" align=right size=22pt color=accent1-25% indent=1/>
<format range="E7" valign=top size=14pt/>
<format range="F7:G7" valign=bottom/>
<format range="B8:C8" valign=bottom align=center/>
<format range="E8:H8" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" align=left font=Georgia size=10pt color=accent1-25% italic/>
<format range="B9" valign=top size=14pt/>
<format range="C9" valign=bottom/>
<format range="E9:H9" border-top="1.5pt solid bg2-10%" valign=top align=left size=16pt/>
<format range="B10" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" font=Georgia size=10pt color=accent1-25% italic/>
<format range="C10" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" align=right font=Georgia size=10pt color=accent1-25% italic indent=2/>
<format range="E10" valign=top size=15.75pt/>
<format range="F10:H10" valign=top size=16pt/>

<table name="tblIncome" range="B4:C7">
| column | type | format | formula |
|---|---|---|---|
| Item | text | General |  |
| Amount | number | "$"#,##0.00 |  |
</table>

<data table="tblIncome">
| row | Item | Amount |
|---|---|---|
| 5 | Income 1 | $2,500.00 |
| 6 | Income 2 | $1,000.00 |
| 7 | Other | $250.00 |
</data>

<table name="tblExpenses" range="B10:C23">
| column | type | format | formula |
|---|---|---|---|
| Item | text | General |  |
| Amount | number | "$"#,##0.00 |  |
</table>

<data table="tblExpenses">
| row | Item | Amount |
|---|---|---|
| 11 | Rent/mortgage | $800.00 |
| 12 | Electric | $120.00 |
| 13 | Gas | $50.00 |
| 14 | Cell phone | $45.00 |
| 15 | Groceries | $500.00 |
| 16 | Car payment | $273.00 |
| 17 | Auto expenses | $120.00 |
| 18 | Student loans | $50.00 |
| 19 | Credit cards | $100.00 |
| 20 | Auto Insurance | $78.00 |
| 21 | Personal care | $50.00 |
| 22 | Entertainment | $100.00 |
| 23 | Miscellaneous | $50.00 |
</data>

<keep id="k8a8r" kind="merged-cells" summary="H4:H5, B8:C8, E4:G5, G8:H8, G9:H9"/>

<keep id="k0k51" kind="conditional-format" summary="E4"/>

<keep id="kushr" kind="chart" summary="column chart: 'Simple Monthly Budget'!$E$9:$F$9 at E11"/>

<keep id="kr189" kind="shape" summary="Need to add more income entries? Start typing below the last entry and the table will automatically expand when you press Enter. at B25: Need to add more entrie…"/>
</sheet>


=== FILE END ===

## Tasks

1. `xb-q1` (read): What is the fill of cell F4? Answer `ANSWER: fill=<colour>`, the colour as the file writes it.
2. `xb-q2` (read): What are the font, size and colour of the Amount header cell C4? Answer `ANSWER: font=<name> size=<pt> color=<colour>`.
3. `xb-q3` (read): Which cells of the sheet have an indent? Answer `ANSWER: <cell>; <cell>` (cells or ranges).
4. `xb-e1` (edit): Fill the header cells of the expenses table (B10:C10) with the theme colour accent2, 80% lighter. Keep it a theme colour, not a hex value.
5. `xb-e2` (edit): Put a 2pt navy outline around the income table (B4:C7): the outer edges only.
6. `xb-e3` (edit): Make the balance figure in H4 bold, in the theme colour accent2.
7. `xb-e4` (edit): Indent the item names of the expenses table (B11:B23) by one level.
8. `xb-e5` (edit): Remove the fill of the balance box E4:G5 (no fill); keep its borders.
9. `xb-e6` (edit): Give the title cell A1 a gradient fill from accent1 to accent2.

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
