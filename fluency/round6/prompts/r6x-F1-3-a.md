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
<format default font=Arial size=10pt color=#000000/>

<sheet name="Exp1" range="A1:L26">

<format range="A1" valign=middle align=left font="Times New Roman" size=12pt bold/>
<format range="B1:I3" valign=middle align=center font="Times New Roman" bold/>
<format range="A2:A3" valign=middle align=left font="Times New Roman" bold/>
<format range="A4:J4" valign=middle align=center font="Times New Roman" size=14pt bold/>
<format range="A5:J5" valign=middle align=center font="Times New Roman" size=14pt/>
<format range="A6:J6" border-bottom="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=14pt/>
<format range="A7:C7" border="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="D7" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="E7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="F7:J7" border="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="A8:J8" border="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="A9:I10" border="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="J9:J10" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt bold/>
<format range="K9:L10" valign=middle align=center font="Times New Roman"/>
<format range="A11" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=left font="Times New Roman" size=12pt/>
<format range="B11:C11" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="D11:E11" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="F11" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="G11" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="H11" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="I11:I21" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="J11" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="A12:A21" border-left="0.75pt solid #000000" valign=middle align=left font="Times New Roman" size=12pt/>
<format range="B12:C21" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="D12:E21" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="F12:F21" border-right="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="G12:G21" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="H12:H21" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="J12:J21" border-right="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="A22" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=left font="Times New Roman" size=12pt/>
<format range="B22:C22" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="D22:E22" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="F22" border-bottom="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="G22:I22" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="J22" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="A23" valign=middle align=left font="Times New Roman" size=12pt/>
<format range="B23:J23" valign=middle align=center font="Times New Roman" size=12pt/>
<format range="A24" valign=middle font="Times New Roman"/>
<format range="A25" valign=middle align=left font="Times New Roman"/>
<format range="G25" valign=middle align=center font="Times New Roman"/>
<format range="A26:D26" align=left font="Times New Roman"/>

<keep id="k8a8r" kind="merged-cells" summary="A26:D26, A5:J5, A4:J4, B7:C7, D7:E7, F7:G7, A6:J6"/>

<data sheet="Exp1" range="A1:L20">
| row | A | B | C | D | E | F | G | H | I | J | K | L |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | (ii) Examples Showing the Effects of the Budget changes on Different Categories of Single and Married Taxpayers |  |  |  |  |  |  |  |  |  |  |  |
| 2 |  |  |  |  |  |  |  |  |  |  |  |  |
| 3 |  |  |  |  |  |  |  |  |  |  |  |  |
| 4 | EXAMPLE 1 |  |  |  |  |  |  |  |  |  |  |  |
| 5 | Married couple, one income, no children taxed under PAYE  |  |  |  |  |  |  |  |  |  |  |  |
| 6 | Full rate PRSI contributor |  |  |  |  |  |  |  |  |  |  |  |
| 7 | GROSS | PRSI Liability |  | Levy Liability |  | Tax Liability |  | Total | Total | Gain as % of  |  |  |
| 8 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Gain | Gain | Net Income |  |  |
| 9 |  |  |  |  |  |  |  | (Per Year) | (Per Week) (a) |  |  |  |
| 10 | € | € | € | € | € | € | € | € | € | % |  |  |
| 11 | 17,500 | 436 | 0 | 0 | 0 | 0 | 0 | 436 | 8 | 2.6 |  |  |
| 12 | 25,000 (b) | 734 | 734 | 499 | 0 | 242 | 0 | 741 | 14 | 3.2 |  |  |
| 13 | 30,000 | 936 | 936 | 600 | 600 | 1,250 | 720 | 530 | 10 | 1.9 |  |  |
| 14 | 35,000 | 1,136 | 1,136 | 700 | 700 | 2,250 | 1,720 | 530 | 10 | 1.7 |  |  |
| 15 | 40,000 | 1,336 | 1,336 | 800 | 800 | 3,250 | 2,720 | 530 | 10 | 1.5 |  |  |
| 16 | 45,000 | 1,536 | 1,536 | 900 | 900 | 5,130 | 4,140 | 990 | 19 | 2.6 |  |  |
| 17 | 50,000 | 1,615 | 1,693 | 1,000 | 1,000 | 7,230 | 6,190 | 962 | 19 | 2.4 |  |  |
| 18 | 60,000 | 1,656 | 1,734 | 1,200 | 1,200 | 11,430 | 10,290 | 1,062 | 20 | 2.3 |  |  |
| 19 | 80,000 | 1,707 | 1,789 | 1,600 | 1,600 | 19,830 | 18,490 | 1,257 | 24 | 2.2 |  |  |
| 20 | 100,000 | 1,737 | 1,820 | 2,000 | 2,000 | 28,230 | 26,690 | 1,457 | 28 | 2.1 |  |  |
</data>
</sheet>

<sheet name="Exp2 (2)" range="A4:N34">

<format range="A4:M4" align=center font="Times New Roman" size=26pt bold/>
<format range="A5:M5" align=center font="Times New Roman" size=26pt/>
<format range="N5" font="Times New Roman" size=14pt/>
<format range="A6:M6" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=26pt/>
<format range="A7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="B7" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="C7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="D7" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="E7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="F7" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="G7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="H7:M7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="A8:A9" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="B8:G8" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="H8:M8" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="B9:E9" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="F9:K9" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="L9:M9" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="A10:K10" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="L10" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="M10" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="A11:A21" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=20pt/>
<format range="B11:M11" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="B12:C12" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="D12" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="E12" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="F12" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="G12:M12" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="B13:L21" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="M13:M21" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="A22" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=20pt/>
<format range="B22:L22" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="M22" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="A23:K23" align=center font="Times New Roman" size=14pt/>
<format range="L23:M24" font="Times New Roman" size=14pt/>
<format range="A24" valign=middle font="Times New Roman" size=16pt/>
<format range="B24" align=center font="Times New Roman" size=16pt/>
<format range="C24:K24" align=center font="Times New Roman" size=14pt/>
<format range="A25:A31" align=left font="Times New Roman" size=16pt/>
<format range="B25:B31" font="Times New Roman" size=16pt/>
<format range="C25:M31" font="Times New Roman" size=14pt/>
<format range="A32:B32" align=left font="Times New Roman" size=16pt/>
<format range="C32:K32" font="Times New Roman" size=14pt/>
<format range="A33" align=center font="Times New Roman" size=18pt/>
<format range="B33" font="Times New Roman" size=18pt/>
<format range="F33:F34" align=center font="Times New Roman"/>
<format range="G33:H33" font="Times New Roman"/>
<format range="K33" font="Times New Roman"/>

<keep id="kr2q2" kind="merged-cells" summary="A32:B32, A5:M5, A4:M4, B7:C7, D7:E7, F7:G7, A6:M6"/>

<data sheet="Exp2 (2)" range="A4:N23">
| row | A | B | C | D | E | F | G | H | I | J | K | L | M | N |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 4 | Example 2 |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 5 | Married couple, one income, two children (under the age of six) taxed under PAYE |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 6 | Full rate PRSI contributor |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 7 | GROSS | PRSI Liability |  | Levy Liabilty |  |  Tax Liability |  | Child Benefit | Total | Gain as % of | Total Gain where | Total Gain where | Gain including FIS |  |
| 8 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Increase (a) | Gain (b) | Net Income (c) | FIS applies (d) | FIS applies (e) | as % of Net Income (f) |  |
| 9 |  |  |  |  |  |  |  |  |  |  | (Per Year) | (Per Week) |  |  |
| 10 | € | € | € | € | € | € | € | € | € | % | € | € | % |  |
| 11 | 17,500 | 436 | 0 | 0 | 0 | 0 | 0 | 180 | 1,116 | 5.0 | 2,104 | 40 | 7.6 |  |
| 12 | 25,000 (g) | 734 | 734 | 499 | 0 | 0 | 0 | 180 | 1,179 | 4.1 | 2,115 | 41 | 6.9 |  |
| 13 | 30,000 | 936 | 936 | 600 | 600 | 480 | 0 | 180 | 1,160 | 3.5 | 2,200 | 42 | 6.7 |  |
| 14 | 35,000 | 1,136 | 1,136 | 700 | 700 | 1,480 | 950 | 180 | 1,210 | 3.3 | 1,210 | 23 | 3.3 |  |
| 15 | 40,000 | 1,336 | 1,336 | 800 | 800 | 2,480 | 1,950 | 180 | 1,210 | 3.0 | 1,210 | 23 | 3.0 |  |
| 16 | 45,000 | 1,536 | 1,536 | 900 | 900 | 4,360 | 3,370 | 180 | 1,670 | 3.9 | 1,670 | 32 | 3.9 |  |
| 17 | 50,000 | 1,615 | 1,693 | 1,000 | 1,000 | 6,460 | 5,420 | 180 | 1,642 | 3.6 | 1,642 | 32 | 3.6 |  |
| 18 | 60,000 | 1,656 | 1,734 | 1,200 | 1,200 | 10,660 | 9,520 | 180 | 1,742 | 3.4 | 1,742 | 34 | 3.4 |  |
| 19 | 80,000 | 1,707 | 1,789 | 1,600 | 1,600 | 19,060 | 17,720 | 180 | 1,937 | 3.1 | 1,937 | 37 | 3.1 |  |
| 20 | 100,000 | 1,737 | 1,820 | 2,000 | 2,000 | 27,460 | 25,920 | 180 | 2,137 | 2.9 | 2,137 | 41 | 2.9 |  |
| 21 | 120,000 | 1,757 | 1,840 | 2,400 | 2,500 | 35,860 | 34,120 | 180 | 2,238 | 2.6 | 2,238 | 43 | 2.6 |  |
| 22 | 200,000 | 1,798 | 1,886 | 4,000 | 4,500 | 69,460 | 66,920 | 180 | 2,633 | 2.0 | 2,633 | 51 | 2.0 |  |
| 23 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp3 (2)" range="A1:N31">

<format range="A1:M1" align=center font="Times New Roman" size=26pt bold/>
<format range="A2:M2" align=center font="Times New Roman" size=26pt/>
<format range="A3:M3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=26pt/>
<format range="A4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="B4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="C4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="D4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="H4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" font="Times New Roman" size=20pt bold/>
<format range="I4:L4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="M4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="N4:N29" font="Times New Roman" size=12pt/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="B5:H5" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="I5:L5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="M5" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="F6:L6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="M6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="A7:L7" border="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="M7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=20pt bold/>
<format range="A8:K8" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="L8:M8" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="A9:M18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="A19:M19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=20pt/>
<format range="A20:K20" align=center font="Times New Roman" size=14pt/>
<format range="L20:M22" font="Times New Roman" size=14pt/>
<format range="A21" valign=middle font="Times New Roman" size=16pt/>
<format range="B21:K21" align=center font="Times New Roman" size=14pt/>
<format range="A22:A29" align=left font="Times New Roman" size=16pt/>
<format range="B22:F22" font="Times New Roman" size=14pt/>
<format range="G22:K22" align=center font="Times New Roman" size=14pt/>
<format range="B23:M29" font="Times New Roman" size=14pt/>
<format range="A30" align=center font="Times New Roman" size=12pt/>
<format range="B30:N30" font="Times New Roman" size=12pt/>
<format range="F31" align=center font="Times New Roman"/>

<keep id="kacg6" kind="merged-cells" summary="A1:M1, B4:C4, D4:E4, A2:M2, A3:M3, F4:G4"/>

<data sheet="Exp3 (2)" range="A1:N20">
| row | A | B | C | D | E | F | G | H | I | J | K | L | M | N |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Example 3 |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 2 |    Married couple, two incomes, two children (under the age of six) taxed under PAYE  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 3 |        Full rate PRSI contributors |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI liability |  | Levy Liability |  |  Tax Liability |  | Child Benefit | Total | Gain as % of | Total Gain where | Total Gain where | Gain including FIS |  |
| 5 | INCOME | Existing | Proposed | Exisitng | Proposed | Existing | Proposed | Increase (a) | Gain (b) | Net Income (c) | FIS applies (d) | FIS applies | as % of Net Income (f) |  |
| 6 |  |  |  |  |  |  |  |  |  |  | (Per Year) | (Per Week) (e) |  |  |
| 7 | € | € | € | € | € | € | € | € | € | % | € | € | % |  |
| 8 | 25,000 | 386 | 0 | 0 | 0 | 0 | 0 | 180 | 1,066 | 3.6 | 2,106 | 4 1 | 6.8 |  |
| 9 | 30,000 | 516 | 516 | 0 | 0 | 0 | 0 | 180 | 680 | 2.0 | 680 | 13 | 2.0 |  |
| 10 | 35,000 | 646 | 646 | 0 | 0 | 760 | 0 | 180 | 1,440 | 3.7 | 1,440 | 28 | 3.7 |  |
| 11 | 40,000 | 776 | 776 | 520 | 520 | 1,760 | 960 | 180 | 1,480 | 3.5 | 1,480 | 28 | 3.5 |  |
| 12 | 45,000 | 1,272 | 906 | 585 | 585 | 2,760 | 1,960 | 180 | 1,846 | 4.1 | 1,846 | 36 | 4.1 |  |
| 13 | 50,000 | 1,472 | 1,036 | 650 | 650 | 3,760 | 2,960 | 180 | 1,916 | 3.9 | 1,916 | 37 | 3.9 |  |
| 14 | 60,000 | 1,872 | 1,872 | 780 | 780 | 5,760 | 4,960 | 180 | 1,480 | 2.6 | 1,480 | 28 | 2.6 |  |
| 15 | 70,000 | 2,272 | 2,272 | 1,400 | 910 | 9,080 | 7,485 | 180 | 2,765 | 4.4 | 2,765 | 53 | 4.4 |  |
| 16 | 80,000 | 2,481 | 2,559 | 1,600 | 1,600 | 13,280 | 11,480 | 180 | 2,402 | 3.5 | 2,402 | 46 | 3.5 |  |
| 17 | 100,000 | 2,807 | 2,888 | 2,000 | 2,000 | 21,680 | 19,680 | 180 | 2,599 | 3.3 | 2,599 | 50 | 3.3 |  |
| 18 | 120,000 | 3,118 | 3,200 | 2,400 | 2,400 | 30,080 | 27,880 | 180 | 2,798 | 3.1 | 2,798 | 54 | 3.1 |  |
| 19 | 200,000 | 3,454 | 3,614 | 4,000 | 4,150 | 63,680 | 60,680 | 180 | 3,370 | 2.5 | 3,370 | 65 | 2.5 |  |
| 20 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp4" range="A1:K24">

<format range="A1:J1" align=center font="Times New Roman" size=14pt bold/>
<format range="A2:J2" align=center font="Times New Roman" size=14pt/>
<format range="A3:J3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="A4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="C4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="D4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H4:J4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B5:F5" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G5" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="I5" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F6:H6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="I6:J6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A7:J7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A8:A15" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B8:H8" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="I8" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J8:J12" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="B9:I12" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="K12:K20" border-left="0.75pt solid #000000" font="Times New Roman"/>
<format range="B13:E13" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="F13:I13" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J13:J19" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="B14:D14" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="E14:I14" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="B15" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="C15:I15" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A16:A19" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B16:I19" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B20:I20" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J20" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A21" align=left font="Times New Roman" size=12pt/>
<format range="B21:J21" align=center font="Times New Roman" size=12pt/>
<format range="K21" font="Times New Roman"/>
<format range="A22" valign=middle font="Times New Roman"/>
<format range="B22:H23" font="Times New Roman" size=12pt/>
<format range="A23:A24" font="Times New Roman"/>
<format range="B24:C24" font="Times New Roman" size=12pt/>
<format range="D24" align=center font="Times New Roman" size=12pt/>
<format range="E24:H24" font="Times New Roman" size=12pt/>

<keep id="kdttv" kind="merged-cells" summary="A1:J1, F4:G4, B4:C4, D4:E4, A2:J2, A3:J3"/>

<data sheet="Exp4" range="A1:K20">
| row | A | B | C | D | E | F | G | H | I | J | K |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 |                     EXAMPLE 4 |  |  |  |  |  |  |  |  |  |  |
| 2 |                        Single person taxed under PAYE  |  |  |  |  |  |  |  |  |  |  |
| 3 |                              Full  rate PRSI contributor |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  | Tax Liability |  | Total | Total  | Gain as % of  |  |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Gain | Gain | Net Income |  |
| 6 |  |  |  |  |  |  |  | (Per Year) | (Per Week) (a) |  |  |
| 7 | € | € | € | € | € | € | € | € | € | % |  |
| 8 | 17,500 | 436 | 0 | 0 | 0 | 380 | 0 | 816 | 16 | 4.9 |  |
| 9 | 25,000 (b) | 734 | 734 | 499 | 0 | 1,872 | 1,472 | 899 | 17 | 4.1 |  |
| 10 | 30,000 | 936 | 936 | 600 | 600 | 2,880 | 2,480 | 400 | 8 | 1.6 |  |
| 11 | 35,000 | 1,136 | 1,136 | 700 | 700 | 4,540 | 3,690 | 850 | 16 | 3.0 |  |
| 12 | 40,000 | 1,336 | 1,336 | 800 | 800 | 6,640 | 5,740 | 900 | 17 | 2.9 |  |
| 13 | 45,000 | 1,536 | 1,536 | 900 | 900 | 8,740 | 7,790 | 950 | 18 | 2.8 |  |
| 14 | 50,000 | 1,615 | 1,693 | 1,000 | 1,000 | 10,840 | 9,840 | 922 | 18 | 2.5 |  |
| 15 | 55,000 | 1,638 | 1,713 | 1,100 | 1,100 | 12,940 | 11,890 | 975 | 19 | 2.5 |  |
| 16 | 60,000 | 1,656 | 1,734 | 1,200 | 1,200 | 15,040 | 13,940 | 1,022 | 20 | 2.4 |  |
| 17 | 80,000 | 1,707 | 1,789 | 1,600 | 1,600 | 23,440 | 22,140 | 1,217 | 23 | 2.3 |  |
| 18 | 100,000 | 1,737 | 1,820 | 2,000 | 2,000 | 31,840 | 30,340 | 1,417 | 27 | 2.2 |  |
| 19 | 120,000 | 1,757 | 1,840 | 2,400 | 2,500 | 40,240 | 38,540 | 1,518 | 29 | 2.0 |  |
| 20 | 200,000 | 1,798 | 1,886 | 4,000 | 4,500 | 73,840 | 71,340 | 1,913 | 37 | 1.6 |  |
</data>
</sheet>

<sheet name="Exp5" range="A1:K28">

<format range="A1:J1" align=center font="Times New Roman" size=14pt bold/>
<format range="A2:J2" align=center font="Times New Roman" size=14pt/>
<format range="A3:J3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="A4:E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H4:I4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="K4:K19" align=center font="Times New Roman" size=12pt/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H5:I5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J5:J6" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F6:I6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A7:H7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="I7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A8:A17" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B8:I17" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J8:J18" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A18" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B18:H18" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="I18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B19:I19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20" align=left font="Times New Roman" size=12pt/>
<format range="B20:K20" align=center font="Times New Roman" size=12pt/>
<format range="A21:A23" font="Times New Roman"/>
<format range="H21:H28" font="Times New Roman"/>
<format range="D23" align=center font="Times New Roman"/>

<keep id="k41pm" kind="merged-cells" summary="A1:J1, F4:G4, B4:C4, D4:E4, A2:J2, A3:J3"/>

<data sheet="Exp5" range="A1:K20">
| row | A | B | C | D | E | F | G | H | I | J | K |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 5 |  |  |  |  |  |  |  |  |  |  |
| 2 |    Married couple, one income, no children taxed under PAYE  |  |  |  |  |  |  |  |  |  |  |
| 3 |    Modified rate PRSI contributor |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  | Tax Liability |  | Total | Total | Gain as % of  |  |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Gain | Gain | Net Income |  |
| 6 |  |  |  |  |  |  |  | (Per Year) | (Per Week) (a) |  |  |
| 7 | € | € | € | € | € | € | € | € | € | % |  |
| 8 | 17,500 | 145 | 0 | 0 | 0 | 0 | 0 | 145 | 3 | 0.8 |  |
| 9 | 25,000 (b) | 212 | 212 | 499 | 0 | 242 | 0 | 741 | 14 | 3.1 |  |
| 10 | 30,000 | 258 | 258 | 600 | 600 | 1,170 | 640 | 530 | 10 | 1.9 |  |
| 11 | 35,000 | 303 | 303 | 700 | 700 | 2,170 | 1,640 | 530 | 10 | 1.7 |  |
| 12 | 40,000 | 348 | 348 | 800 | 800 | 3,170 | 2,640 | 530 | 10 | 1.5 |  |
| 13 | 45,000 | 393 | 393 | 900 | 900 | 5,050 | 4,060 | 990 | 19 | 2.6 |  |
| 14 | 50,000 | 408 | 427 | 1,000 | 1,000 | 7,150 | 6,110 | 1,021 | 20 | 2.5 |  |
| 15 | 60,000 | 410 | 429 | 1,200 | 1,200 | 11,350 | 10,210 | 1,121 | 22 | 2.4 |  |
| 16 | 80,000 | 412 | 432 | 1,600 | 1,600 | 19,750 | 18,410 | 1,320 | 25 | 2.3 |  |
| 17 | 100,000 | 414 | 433 | 2,000 | 2,000 | 28,150 | 26,610 | 1,520 | 29 | 2.2 |  |
| 18 | 120,000 | 414 | 434 | 2,400 | 2,500 | 36,550 | 34,810 | 1,620 | 31 | 2.0 |  |
| 19 | 200,000 | 416 | 436 | 4,000 | 4,500 | 70,150 | 67,610 | 2,020 | 39 | 1.6 |  |
| 20 |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp6 (2)" range="A1:N32">

<format range="A1:M1" align=center font="Times New Roman" size=24pt bold/>
<format range="A2:M2" align=center font="Times New Roman" size=24pt/>
<format range="A3:M3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=24pt/>
<format range="A4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="B4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="C4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="D4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="H4:M4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="N4:N18" border-left="0.75pt solid #000000" font="Times New Roman" size=12pt/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="H5:M5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="F6:L6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="M6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="A7:L7" border="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="M7" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="A8:A17" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=18pt/>
<format range="B8:M17" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="A18" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=18pt/>
<format range="B18:L18" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="M18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="A19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=18pt/>
<format range="B19:M19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="N19:N20" font="Times New Roman" size=12pt/>
<format range="A20" align=left font="Times New Roman" size=18pt/>
<format range="B20:M20" align=center font="Times New Roman" size=18pt/>
<format range="A21" valign=middle font="Times New Roman" size=15pt/>
<format range="B21:K21" align=center font="Times New Roman" size=14pt/>
<format range="L21" align=center font="Times New Roman" size=12pt/>
<format range="M21:N21" font="Times New Roman" size=12pt/>
<format range="A22:A29" align=left font="Times New Roman" size=15pt/>
<format range="B22:K27" font="Times New Roman" size=14pt/>
<format range="L22:N27" font="Times New Roman" size=12pt/>
<format range="B28:L29" font="Times New Roman" size=14pt/>
<format range="M28:N29" font="Times New Roman" size=12pt/>
<format range="A30" font="Times New Roman"/>
<format range="L30:L31" font="Times New Roman"/>
<format range="I31" font="Times New Roman"/>
<format range="F32:G32" align=center font="Times New Roman"/>

<keep id="k7nfe" kind="merged-cells" summary="B4:C4, D4:E4, A1:M1, A2:M2, A3:M3, F4:G4"/>

<data sheet="Exp6 (2)" range="A1:N20">
| row | A | B | C | D | E | F | G | H | I | J | K | L | M | N |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 6 |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 2 |    Married couple, one income, two children (under the age of 6) taxed under PAYE  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 3 |    Modified rate PRSI contributor |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  |  Tax Liability |  | Child Benefit | Total | Gain as % of | Total Gain where | Total Gain where | Gain including FIS |  |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Increase (a) | Gain (b) | Net Income (c) | FIS applies (d) | FIS applies (e) | as % of Net Income (f) |  |
| 6 |  |  |  |  |  |  |  |  |  |  | (Per Year) | (Per Week) |  |  |
| 7 | € | € | € | € | € | € | € | € | € | % | € | € | % |  |
| 8 | 17,500 | 145 | 0 | 0 | 0 | 0 | 0 | 180 | 825 | 3.7 | 2,021 | 39 | 7.2 |  |
| 9 | 25,000 (g) | 212 | 212 | 499 | 0 | 0 | 0 | 180 | 1,179 | 4.0 | 2,115 | 41 | 6.9 |  |
| 10 | 30,000 | 258 | 258 | 600 | 600 | 480 | 0 | 180 | 1,160 | 3.4 | 1,160 | 22 | 3.4 |  |
| 11 | 35,000 | 303 | 303 | 700 | 700 | 1,480 | 950 | 180 | 1,210 | 3.2 | 1,210 | 23 | 3.2 |  |
| 12 | 40,000 | 348 | 348 | 800 | 800 | 2,480 | 1,950 | 180 | 1,210 | 2.9 | 1,210 | 23 | 2.9 |  |
| 13 | 45,000 | 393 | 393 | 900 | 900 | 4,360 | 3,370 | 180 | 1,670 | 3.8 | 1,670 | 32 | 3.8 |  |
| 14 | 50,000 | 408 | 427 | 1,000 | 1,000 | 6,460 | 5,420 | 180 | 1,701 | 3.6 | 1,701 | 33 | 3.6 |  |
| 15 | 60,000 | 410 | 429 | 1,200 | 1,200 | 10,660 | 9,520 | 180 | 1,801 | 3.4 | 1,801 | 35 | 3.4 |  |
| 16 | 80,000 | 412 | 432 | 1,600 | 1,600 | 19,060 | 17,720 | 180 | 2,000 | 3.1 | 2,000 | 38 | 3.1 |  |
| 17 | 100,000 | 414 | 433 | 2,000 | 2,000 | 27,460 | 25,920 | 180 | 2,200 | 2.9 | 2,200 | 42 | 2.9 |  |
| 18 | 120,000 | 414 | 434 | 2,400 | 2,500 | 35,860 | 34,120 | 180 | 2,301 | 2.7 | 2,301 | 44 | 2.7 |  |
| 19 | 200,000 | 416 | 436 | 4,000 | 4,500 | 69,460 | 66,920 | 180 | 2,701 | 2.1 | 2,701 | 52 | 2.1 |  |
| 20 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp7 (2)" range="A1:M29">

<format range="A1:M1" align=center font="Times New Roman" size=22pt bold/>
<format range="A2:M2" align=center font="Times New Roman" size=22pt/>
<format range="A3:M3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=22pt/>
<format range="A4:E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="H4:M4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="H5:M5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="F6:M6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="A7:M7" border="0.75pt solid #000000" align=center font="Times New Roman" size=18pt bold/>
<format range="A8:M18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="A19:M19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=18pt/>
<format range="A20:L20" align=center font="Times New Roman" size=12pt/>
<format range="M20:M21" font="Times New Roman" size=12pt/>
<format range="A21" font="Times New Roman" size=15pt/>
<format range="B21:J21" align=center font="Times New Roman" size=14pt/>
<format range="K21:L21" align=center font="Times New Roman" size=12pt/>
<format range="A22:A29" align=left font="Times New Roman" size=15pt/>
<format range="B22:J29" font="Times New Roman" size=14pt/>
<format range="K22:M29" font="Times New Roman" size=12pt/>

<keep id="k6jff" kind="merged-cells" summary="A1:M1, B4:C4, D4:E4, A2:M2, A3:M3, F4:G4"/>

<data sheet="Exp7 (2)" range="A1:M20">
| row | A | B | C | D | E | F | G | H | I | J | K | L | M |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 7 |  |  |  |  |  |  |  |  |  |  |  |  |
| 2 |    Married couple, two incomes, two children (under the age of 6) |  |  |  |  |  |  |  |  |  |  |  |  |
| 3 |    Modified rate PRSI contributors |  |  |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  | Tax Liability |  | Child Benefit | Total | Gain as % of | Total Gain where | Total Gain where | Gain including FIS |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Increase (a) | Gain (b) | Net Income (c) | FIS applies (d) | FIS applies (e) | as % of Net Income (f) |
| 6 |  |  |  |  |  |  |  |  |  |  | (Per Year) | (Per Week) |  |
| 7 | € | € | € | € | € | € | € | € | € | % | € |  | % |
| 8 | 20,000 | 0 | 0 | 0 | 0 | 0 | 0 | 180 | 680 | 2.7 | 1,928 | 37 | 6.6 |
| 9 | 30,000 | 163 | 163 | 0 | 0 | 0 | 0 | 180 | 680 | 1.9 | 680 | 13 | 1.9 |
| 10 | 35,000 | 193 | 193 | 0 | 0 | 760 | 0 | 180 | 1,440 | 3.7 | 1,440 | 28 | 3.7 |
| 11 | 40,000 | 222 | 222 | 520 | 520 | 1,760 | 960 | 180 | 1,480 | 3.5 | 1,480 | 28 | 3.5 |
| 12 | 45,000 | 381 | 251 | 585 | 585 | 2,760 | 1,960 | 180 | 1,610 | 3.5 | 1,610 | 31 | 3.5 |
| 13 | 50,000 | 426 | 280 | 650 | 650 | 3,760 | 2,960 | 180 | 1,625 | 3.2 | 1,625 | 31 | 3.2 |
| 14 | 60,000 | 516 | 516 | 780 | 780 | 5,760 | 4,960 | 180 | 1,480 | 2.6 | 1,480 | 28 | 2.6 |
| 15 | 70,000 | 606 | 606 | 1,400 | 910 | 9,080 | 7,485 | 180 | 2,765 | 4.3 | 2,765 | 53 | 4.3 |
| 16 | 80,000 | 648 | 668 | 1,600 | 1,600 | 13,280 | 11,480 | 180 | 2,461 | 3.5 | 2,461 | 47 | 3.5 |
| 17 | 100,000 | 713 | 733 | 2,000 | 2,000 | 21,680 | 19,680 | 180 | 2,661 | 3.3 | 2,661 | 51 | 3.3 |
| 18 | 120,000 | 778 | 797 | 2,400 | 2,400 | 30,080 | 27,880 | 180 | 2,860 | 3.1 | 2,860 | 55 | 3.1 |
| 19 | 200,000 | 826 | 865 | 4,000 | 4,150 | 63,680 | 60,680 | 180 | 3,492 | 2.6 | 3,492 | 67 | 2.6 |
| 20 |  |  |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp8" range="A1:K23">

<format range="A1:J1" align=center font="Times New Roman" size=14pt bold/>
<format range="K1" align=center font="Times New Roman"/>
<format range="A2:J2" align=center font="Times New Roman" size=14pt/>
<format range="A3:J3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="K3" align=center font="Times New Roman"/>
<format range="A4:E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H4:I4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H5:I5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J5:J6" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G6:H6" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="I6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A7:G7" border="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="H7:I7" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="J7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="A8:A18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B8:I18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J8:J18" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B19:I19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20:H20" align=center font="Times New Roman" size=14pt/>
<format range="I20:J20" font="Times New Roman" size=14pt/>
<format range="A21" font="Times New Roman"/>
<format range="E21:J22" font="Times New Roman" size=14pt/>
<format range="A22" align=left font="Times New Roman"/>
<format range="A23:D23" align=left font="Times New Roman"/>

<keep id="kpsrp" kind="merged-cells" summary="A23:D23, A1:J1, A3:J3, B4:C4, D4:E4, F4:G4, A2:J2"/>

<data sheet="Exp8" range="A1:K20">
| row | A | B | C | D | E | F | G | H | I | J | K |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 8 |  |  |  |  |  |  |  |  |  |  |
| 2 | Single person taxed under PAYE  |  |  |  |  |  |  |  |  |  |  |
| 3 | Modified  rate PRSI contributor |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  | Tax Liability |  | Total | Total | Gain as % of  |  |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Gain | Gain | Net Income |  |
| 6 |  |  |  |  |  |  |  | (Per Year) | (Per Week) (a) |  |  |
| 7 | € | € | € | € | € | € | € | € | € | % |  |
| 8 | 17,500 | 145 | 0 | 0 | 0 | 380 | 0 | 525 | 10 | 3.1 |  |
| 9 | 25,000 (b) | 212 | 212 | 499 | 0 | 1,872 | 1,472 | 899 | 17 | 4.0 |  |
| 10 | 30,000 | 258 | 258 | 600 | 600 | 2,880 | 2,480 | 400 | 8 | 1.5 |  |
| 11 | 35,000 | 303 | 303 | 700 | 700 | 4,540 | 3,690 | 850 | 16 | 2.9 |  |
| 12 | 40,000 | 348 | 348 | 800 | 800 | 6,640 | 5,740 | 900 | 17 | 2.8 |  |
| 13 | 45,000 | 393 | 393 | 900 | 900 | 8,740 | 7,790 | 950 | 18 | 2.7 |  |
| 14 | 50,000 | 408 | 427 | 1,000 | 1,000 | 10,840 | 9,840 | 981 | 19 | 2.6 |  |
| 15 | 60,000 | 410 | 429 | 1,200 | 1,200 | 15,040 | 13,940 | 1,081 | 21 | 2.5 |  |
| 16 | 80,000 | 412 | 432 | 1,600 | 1,600 | 23,440 | 22,140 | 1,280 | 25 | 2.3 |  |
| 17 | 100,000 | 414 | 433 | 2,000 | 2,000 | 31,840 | 30,340 | 1,480 | 28 | 2.3 |  |
| 18 | 120,000 | 414 | 434 | 2,400 | 2,500 | 40,240 | 38,540 | 1,581 | 30 | 2.1 |  |
| 19 | 200,000 | 416 | 436 | 4,000 | 4,500 | 73,840 | 71,340 | 1,981 | 38 | 1.6 |  |
| 20 |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp9" range="A1:J24">

<format range="A1:J1" align=center font="Times New Roman" size=14pt bold/>
<format range="A2:J2" align=center font="Times New Roman" size=14pt/>
<format range="A3:J3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="A4:E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H4:I4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H5:I5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J5" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F6:H6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="I6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A7:I7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A8:A18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B8:I18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J8:J18" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B19:I19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20:J20" font="Times New Roman"/>
<format range="A21:H22" font="Times New Roman"/>
<format range="A23:C23" font="Times New Roman"/>
<format range="D23" align=center font="Times New Roman"/>
<format range="E23:H23" font="Times New Roman"/>
<format range="A24:H24" font="Times New Roman"/>

<keep id="kg7hp" kind="merged-cells" summary="B4:C4, D4:E4, A1:J1, A2:J2, A3:J3, F4:G4"/>

<data sheet="Exp9" range="A1:J20">
| row | A | B | C | D | E | F | G | H | I | J |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 9 |  |  |  |  |  |  |  |  |  |
| 2 |    Married couple, one income, no children   |  |  |  |  |  |  |  |  |  |
| 3 | Taxed under Schedule D |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  | Tax Liability |  | Total | Total | Gain as % of |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Gain | Gain | Net Income |
| 6 |  |  |  |  |  |  |  | (Per Year) | (Per Week) (a) |  |
| 7 | € | € | € | € | € | € | € | € | € | % |
| 8 | 17,500 | 525 | 525 | 0 | 0 | 240 | 0 | 240 | 5 | 1.4 |
| 9 | 25,000 (b) | 749 | 749 | 499 | 0 | 1,732 | 1,472 | 759 | 15 | 3.5 |
| 10 | 30,000 | 900 | 900 | 600 | 600 | 2,740 | 2,480 | 260 | 5 | 1.0 |
| 11 | 35,000 | 1,050 | 1,050 | 700 | 700 | 3,740 | 3,480 | 260 | 5 | 0.9 |
| 12 | 40,000 | 1,200 | 1,200 | 800 | 800 | 4,740 | 4,480 | 260 | 5 | 0.8 |
| 13 | 45,000 | 1,350 | 1,350 | 900 | 900 | 6,620 | 5,900 | 720 | 14 | 2.0 |
| 14 | 50,000 | 1,500 | 1,500 | 1,000 | 1,000 | 8,720 | 7,950 | 770 | 15 | 2.0 |
| 15 | 60,000 | 1,800 | 1,800 | 1,200 | 1,200 | 12,920 | 12,050 | 870 | 17 | 2.0 |
| 16 | 80,000 | 2,400 | 2,400 | 1,600 | 1,600 | 21,320 | 20,250 | 1,070 | 21 | 2.0 |
| 17 | 100,000 | 3,000 | 3,000 | 2,000 | 2,000 | 29,720 | 28,450 | 1,270 | 24 | 1.9 |
| 18 | 120,000 | 3,600 | 3,600 | 2,400 | 2,500 | 38,120 | 36,650 | 1,371 | 26 | 1.8 |
| 19 | 200,000 | 6,000 | 6,000 | 4,000 | 4,500 | 71,720 | 69,450 | 1,771 | 34 | 1.5 |
| 20 |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp10 (2)" range="A1:K26">

<format range="A1:K1" align=center font="Times New Roman" size=14pt bold/>
<format range="A2:K2" align=center font="Times New Roman" size=14pt/>
<format range="A3:K3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="A4:C4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="D4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F4:K4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B5:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F5:K5" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F6:J6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="K6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A7:K7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A8:A18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B8:K18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B19:K19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20:J20" align=center font="Times New Roman" size=12pt/>
<format range="A21" valign=middle font="Times New Roman"/>
<format range="B21:K21" align=center font="Times New Roman" size=12pt/>
<format range="A22:A26" align=left font="Times New Roman"/>
<format range="G22:H25" font="Times New Roman"/>
<format range="K22:K25" font="Times New Roman"/>
<format range="B26" font="Times New Roman" size=11pt/>
<format range="C26:J26" font="Times New Roman" size=13pt bold/>

<keep id="k4t5h" kind="merged-cells" summary="B4:C4, D4:E4, A1:K1, A2:K2, A3:K3"/>

<data sheet="Exp10 (2)" range="A1:K20">
| row | A | B | C | D | E | F | G | H | I | J | K |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 10 |  |  |  |  |  |  |  |  |  |  |
| 2 |    Married couple, one income, two children (under the age of 6)   |  |  |  |  |  |  |  |  |  |  |
| 3 |    Taxed under Schedule D |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS |      PRSI Liability |  | Levy Liability |  |     Tax Liability |  | Child Benefit | Total | Total | Gain as % of |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Increase (a) | Gain (b) | Gain | Net Income (d) |
| 6 |  |  |  |  |  |  |  |  | (Per Year) | (Per Week) ( c) |  |
| 7 | € | € | € | € | € | € | € | € | € | € | % |
| 8 | 17,500 | 525 | 525 | 0 | 0 | 0 | 0 | 180 | 680 | 13 | 3.1 |
| 9 | 25,000 (e) | 749 | 749 | 499 | 0 | 962 | 702 | 180 | 1,439 | 28 | 5.2 |
| 10 | 30,000 | 900 | 900 | 600 | 600 | 1,970 | 1,710 | 180 | 940 | 18 | 3.0 |
| 11 | 35,000 | 1,050 | 1,050 | 700 | 700 | 2,970 | 2,710 | 180 | 940 | 18 | 2.7 |
| 12 | 40,000 | 1,200 | 1,200 | 800 | 800 | 3,970 | 3,710 | 180 | 940 | 18 | 2.4 |
| 13 | 45,000 | 1,350 | 1,350 | 900 | 900 | 5,850 | 5,130 | 180 | 1,400 | 27 | 3.3 |
| 14 | 50,000 | 1,500 | 1,500 | 1,000 | 1,000 | 7,950 | 7,180 | 180 | 1,450 | 28 | 3.3 |
| 15 | 60,000 | 1,800 | 1,800 | 1,200 | 1,200 | 12,150 | 11,280 | 180 | 1,550 | 30 | 3.1 |
| 16 | 80,000 | 2,400 | 2,400 | 1,600 | 1,600 | 20,550 | 19,480 | 180 | 1,750 | 34 | 2.9 |
| 17 | 100,000 | 3,000 | 3,000 | 2,000 | 2,000 | 28,950 | 27,680 | 180 | 1,950 | 38 | 2.7 |
| 18 | 120,000 | 3,600 | 3,600 | 2,400 | 2,500 | 37,350 | 35,880 | 180 | 2,051 | 39 | 2.5 |
| 19 | 200,000 | 6,000 | 6,000 | 4,000 | 4,500 | 70,950 | 68,680 | 180 | 2,451 | 47 | 2.0 |
| 20 |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp11 (2)" range="A1:Q26">

<format range="A1:K1" align=center font="Times New Roman" size=14pt bold/>
<format range="A2:K2" align=center font="Times New Roman" size=14pt/>
<format range="A3:K3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="A4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="C4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="D4" border-top="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H4:K4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="H5:K5" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="F6:I6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J6" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="K6" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A7:K7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A8:K18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A19:K19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20:J20" align=center font="Times New Roman" size=12pt/>
<format range="A21" align=left font="Times New Roman"/>
<format range="B21:H21" align=left font="Times New Roman" size=12pt/>
<format range="I21:Q21" align=left font="Times New Roman"/>
<format range="A22" valign=middle font="Times New Roman"/>
<format range="B22:K22" align=center font="Times New Roman" size=12pt/>
<format range="L22:Q26" align=left font="Times New Roman"/>
<format range="A23:A26" align=left font="Times New Roman"/>
<format range="B23:H26" font="Times New Roman" size=12pt/>
<format range="I23:K26" font="Times New Roman"/>

<keep id="kdpaq" kind="merged-cells" summary="A1:K1, F4:G4, A2:K2, A3:K3, B4:C4, D4:E4"/>

<data sheet="Exp11 (2)" range="A1:Q20">
| row | A | B | C | D | E | F | G | H | I | J | K | L | M | N | O | P | Q |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 11 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 2 | Married couple, two incomes, two children, (under the age of 6)   |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 3 | Taxed under Schedule D  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  |        Tax Liability |  | Child Benefit | Total | Total | Gain as % of |  |  |  |  |  |  |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Increase (a) | Gain (b) | Gain | Net Income (d) |  |  |  |  |  |  |
| 6 |  |  |  |  |  |  |  |  | (Per Year)  | (Per Week) (c ) |  |  |  |  |  |  |  |
| 7 | € | € | € | € | € | € | € | € | € |  | % |  |  |  |  |  |  |
| 8 | 20,000 | 643 | 643 | 0 | 0 | 740 | 480 | 180 | 940 | 18 | 4.0 |  |  |  |  |  |  |
| 9 | 30,000 | 900 | 900 | 0 | 0 | 2,740 | 2,480 | 180 | 940 | 18 | 3.0 |  |  |  |  |  |  |
| 10 | 35,000 | 1,050 | 1,050 | 0 | 0 | 3,740 | 3,480 | 180 | 940 | 18 | 2.7 |  |  |  |  |  |  |
| 11 | 40,000 | 1,200 | 1,200 | 520 | 520 | 4,740 | 4,480 | 180 | 940 | 18 | 2.4 |  |  |  |  |  |  |
| 12 | 45,000 | 1,350 | 1,350 | 585 | 585 | 5,740 | 5,480 | 180 | 940 | 18 | 2.2 |  |  |  |  |  |  |
| 13 | 50,000 | 1,500 | 1,500 | 650 | 650 | 6,740 | 6,480 | 180 | 940 | 18 | 2.0 |  |  |  |  |  |  |
| 14 | 60,000 | 1,800 | 1,800 | 780 | 780 | 8,740 | 8,480 | 180 | 940 | 18 | 1.7 |  |  |  |  |  |  |
| 15 | 70,000 | 2,100 | 2,100 | 1,400 | 910 | 12,060 | 11,005 | 180 | 2,225 | 43 | 3.7 |  |  |  |  |  |  |
| 16 | 80,000 | 2,400 | 2,400 | 1,600 | 1,600 | 16,260 | 15,000 | 180 | 1,940 | 37 | 3.0 |  |  |  |  |  |  |
| 17 | 100,000 | 3,000 | 3,000 | 2,000 | 2,000 | 24,660 | 23,200 | 180 | 2,140 | 41 | 2.8 |  |  |  |  |  |  |
| 18 | 120,000 | 3,600 | 3,600 | 2,400 | 2,400 | 33,060 | 31,400 | 180 | 2,340 | 45 | 2.7 |  |  |  |  |  |  |
| 19 | 200,000 | 6,000 | 6,000 | 4,000 | 4,150 | 66,660 | 64,200 | 180 | 2,991 | 58 | 2.3 |  |  |  |  |  |  |
| 20 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>

<sheet name="Exp12" range="A1:M23">

<format range="A1:J1" align=center font="Times New Roman" size=14pt bold/>
<format range="A2:J2" align=center font="Times New Roman" size=14pt/>
<format range="A3:J3" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=14pt/>
<format range="A4:E4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="F4" border-top="0.75pt solid #000000" border-left="0.75pt solid #000000" font="Times New Roman" size=14pt bold/>
<format range="G4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" font="Times New Roman" size=14pt bold/>
<format range="H4:I4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="J4" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="A5:A6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="B5:G5" border="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="H5:I5" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="J5:J6" border-right="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="B6:E6" border="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="F6:I6" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=14pt bold/>
<format range="A7:I7" border="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="J7" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt bold/>
<format range="A8:A18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B8:H18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="I8:I17" align=center font="Times New Roman" size=12pt/>
<format range="J8" border-top="0.75pt solid #000000" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J9:J18" border-right="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="I18" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=left font="Times New Roman" size=12pt/>
<format range="B19:H19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="I19" border-bottom="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="J19" border-right="0.75pt solid #000000" border-bottom="0.75pt solid #000000" border-left="0.75pt solid #000000" align=center font="Times New Roman" size=12pt/>
<format range="A20:H20" align=center font="Times New Roman" size=12pt/>
<format range="A21" font="Times New Roman"/>
<format range="B21:H21" font="Times New Roman" size=12pt/>
<format range="A22" align=left font="Times New Roman"/>
<format range="B22:H22" font="Times New Roman"/>
<format range="A23" font="Times New Roman"/>

<keep id="k6fs9" kind="merged-cells" summary="B4:C4, D4:E4, A1:J1, A2:J2, A3:J3"/>

<data sheet="Exp12" range="A1:M20">
| row | A | B | C | D | E | F | G | H | I | J | K | L | M |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | EXAMPLE 12 |  |  |  |  |  |  |  |  |  |  |  |  |
| 2 | Single person  |  |  |  |  |  |  |  |  |  |  |  |  |
| 3 | Taxed under Schedule D |  |  |  |  |  |  |  |  |  |  |  |   |
| 4 | GROSS | PRSI Liability |  | Levy Liability |  |           Tax Liability |  | Total | Total  | Gain as % of  |  |  |  |
| 5 | INCOME | Existing | Proposed | Existing | Proposed | Existing | Proposed | Gain | Gain | Net Income |  |  |  |
| 6 |  |  |  |  |  |  |  | (Per Month) | (Per Week) (a) |  |  |  |  |
| 7 | € | € | € | € | € | € | € | € | € | % |  |  |  |
| 8 | 17,500 | 525 | 525 | 0 | 0 | 1,870 | 1,740 | 130 | 3 | 0.9 |  |  |  |
| 9 | 25,000 (b) | 749 | 749 | 499 | 0 | 3,362 | 3,232 | 629 | 12 | 3.1 |  |  |  |
| 10 | 30,000 | 900 | 900 | 600 | 600 | 4,370 | 4,240 | 130 | 3 | 0.5 |  |  |  |
| 11 | 35,000 | 1,050 | 1,050 | 700 | 700 | 6,030 | 5,450 | 580 | 11 | 2.1 |  |  |  |
| 12 | 40,000 | 1,200 | 1,200 | 800 | 800 | 8,130 | 7,500 | 630 | 12 | 2.1 |  |  |  |
| 13 | 45,000 | 1,350 | 1,350 | 900 | 900 | 10,230 | 9,550 | 680 | 13 | 2.1 |  |  |  |
| 14 | 50,000 | 1,500 | 1,500 | 1,000 | 1,000 | 12,330 | 11,600 | 730 | 14 | 2.1 |  |  |  |
| 15 | 60,000 | 1,800 | 1,800 | 1,200 | 1,200 | 16,530 | 15,700 | 830 | 16 | 2.1 |  |  |  |
| 16 | 80,000 | 2,400 | 2,400 | 1,600 | 1,600 | 24,930 | 23,900 | 1,030 | 20 | 2.0 |  |  |  |
| 17 | 100,000 | 3,000 | 3,000 | 2,000 | 2,000 | 33,330 | 32,100 | 1,230 | 24 | 2.0 |  |  |  |
| 18 | 120,000 | 3,600 | 3,600 | 2,400 | 2,500 | 41,730 | 40,300 | 1,331 | 26 | 1.8 |  |  |  |
| 19 | 200,000 | 6,000 | 6,000 | 4,000 | 4,500 | 75,330 | 73,100 | 1,731 | 33 | 1.5 |  |  |  |
| 20 |  |  |  |  |  |  |  |  |  |  |  |  |  |
</data>
</sheet>


=== FILE END ===

## Tasks

1. `tb-q1` (read): In sheet Exp1, what are the four borders of cell E7? Answer `ANSWER: border-top=… border-right=… border-bottom=… border-left=…`.
2. `tb-q2` (read): In sheet Exp1, what are the font size and the horizontal alignment of cell C15? Answer `ANSWER: size=<pt> align=<value>`.
3. `tb-q3` (read): In sheet Exp8, is cell A1 bold, and what is its font size? Answer `ANSWER: bold=<yes|no> size=<pt>`.
4. `tb-e1` (edit): In sheet Exp1, fill the header row A7:J7 with light grey (#D9D9D9); keep its borders.
5. `tb-e2` (edit): In sheet Exp1, put a thick black outline (2.25pt solid) around the block A7:J22.
6. `tb-e3` (edit): In every one of the twelve sheets, make cell A1 14pt.
7. `tb-e4` (edit): In sheet Exp4, set the font of the whole used range (A1:K24) to Arial.
8. `tb-e5` (edit): In sheet Exp1, draw a diagonal line across cell A7.

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
