hanji edits office files (docx, hwpx, pptx, xlsx) as text. A file becomes a document with revisions; you read and edit its text, and everything the text does not show (layout, formatting, pictures, comments) is kept for you and put back on export.

How to work
1. Open a file (or make a new one). You get a doc_id and revision 1. Tell the person about any surfaced items it reports (comments, hidden text, tracked deletions, metadata).
2. Read before every edit. A read returns the current revision; edits name it. A large file comes in parts: use its outline and read by lines, section (heading text) or slides.
3. Edit with exact spans: `old` is copied exactly from what you read (spaces, line breaks and markers included) and must occur exactly once in the whole revision, so add surrounding text when it repeats. Several edits go in one list, applied in order. For large changes, write the whole text instead. Spreadsheet cells change only through range operations.
4. A refused edit changes nothing and says why: the line and column, the expected form, the allowed names, or how often `old` occurs. Fix that and retry; never resend the same edit unchanged. If the revision is stale, read again.
5. Export to a path. If it lists surfaced items, show them to the person and export again with acknowledge_surfaced only once they agree. For a docx a person will review, export with tracked_changes: your edits since the file was opened come out as Word's tracked changes by "hanji (model edit)"; what cannot be tracked (moving a placeholder, deleting a field or another author's change, adding or removing a section break) is refused with the reason: export without tracked_changes instead.
6. If the person changed the exported file in Office or Hancom, re-import it, then read again: their changes are the new revision.

The format (schema 1)
Every text starts with front matter:
---
type: document
format: docx
schema: 1
---
(`type` is document, presentation or spreadsheet; `format` is docx or hwpx, pptx, xlsx; an optional `template:` line.)
Canonical form: one paragraph per line (no hard wraps), a blank line between blocks, no table padding, `1.` for every numbered item. hanji stores text in this form; read again after an edit that reports `canonicalized`.
Inline: **bold**, *italic*, ~~strike~~, <u>underline</u>, <br/> (a line break inside a paragraph).
Formatting is by the file's own style and layout names only: no colours, fonts or sizes. If no style fits a request, say so; do not invent one.
<keep id="k3" kind="drawing" summary="…"/> stands for something the text does not model (a picture, chart, footnote, comment, merged cells, …). Leave it as it is; you may move it or delete its line, never create or change one.

Document (docx, hwpx)
- `#` … `######`: the file's Heading 1–6.
- <div style="Name">text</div>: a paragraph in style Name (one style name exactly as the file lists it, spaces included).
- <p/> on its own line: an empty paragraph; <p style="Name"/>: an empty one in style Name.
- `- item` bullets, `1. item` numbered. Nest by indenting to the parent's text: 2 spaces under `- `, 3 under `1. `. A blank line ends a list; so does an item at the margin of the other kind.
- Tables are pipe tables, one line per row, one cell per column in every row:
  | 지역 | 지점 | 매출 |
  |---|---|---|
  | 서울 | 강남 | 120 |
  | ^^ | 종로 | 95 |
  | 합계 || 215 |
  `^^` as the whole cell: merged into the cell above. `||` (no space between the pipes): the cell to the left extends into this column; `|||` spans three. Text goes in the top-left cell of a merge only. In a cell, <p/> starts another paragraph. A line {style="Name"} right before the header row sets the table style; without it a table has the file's default.
- <pagebreak/> on its own line.

Presentation (pptx)
Slides are separated by a line `---`. Each slide's first line is `layout: Name` (a layout of the file). Then slot markers on their own lines, each followed by its text: ::title::, ::subtitle::, ::body::, ::left:: and ::right:: (two content areas), ::notes:: (last), and the others the layout has. Only the layout's slots; leave an unfilled slot out; an empty marker is an error. Slots hold paragraphs, - / 1. items and <p/>; no headings, styles or tables.
<shape id="s4" name="출처">text<p/>more</shape> is an existing text shape: edit its text, delete its line to delete it; never create one.
Text shows its font, size and color: what an object's text shares on its marker or tag (::title font=Calibri size=44pt color=tx1::), a paragraph's own at its end (- 매출 증가 {size=28pt}), a run's as [text]{size=24pt color=#FF7F50}. Change a value to restyle; leave one out to take what the layout gives. Colours: #RRGGBB, a theme name (tx1, accent1…), accent1+40% lighter, accent1-25% darker, /50% opacity; a value with * (accent1*) or gradient is kept as the file has it.
<picture id="s7" name="Picture 6" box="x y w h" src="media/image1.png" crop="10 0 5 0" mask="ellipse" alt="…"/> is a picture (box in points from the slide's top-left corner): change its box, crop (percent cut off the left, top, right and bottom), mask (a preset shape such as ellipse or roundRect; none is a rectangle) or alt (its alternative text); src may name another picture's image. Keep its id and name.
A slot or shape shows its fill after its box (<shape id="s9" name="Oval 8" box="…" fill=accent1/>, ::title box="…" fill=accent2::); write fill=colour to fill it, leave fill out for none.
layout: Title and Content
::title::
핵심 지표
::body::
- 매출 **12%** 증가
  - 수도권 21곳
::notes::
전년 대비 강조

Spreadsheet (xlsx)
The text is the workbook's structure; cells are not in it.
<sheet name="매출" range="A1:G1201">

<table name="Sales" range="A1:D1201">
| column | type | format | formula |
|---|---|---|---|
| 월 | date | yyyy-mm |  |
| 매출 | number | #,##0 |  |
| 원가 | number | #,##0 |  |
| 이익 | number | #,##0 | =[@매출]-[@원가] |
</table>

<chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>
</sheet>
A sheet's `range` is its used range. A column's type is text, number, date or mixed. Editing the structure text can add sheets (at the end), tables and columns (at the end), and change a column's type, format or formula; it cannot rename, reorder or delete sheets, tables or columns, or change a `range` (those follow the cells).
Cells are read as a row window: a table and its sheet rows (rows "2:101"), or a sheet and a range ("A1:F50"). The window's first column is the sheet row number (read-only); values are as displayed.
Cells are written by range operations, a JSON list applied in order, all or nothing: set {range, values}, append_rows {table, rows}, insert_rows {table, before, rows}, delete_rows {table, rows: "6:7"}, fill_formula {table, column, formula}, set_type {table, column, type, format?}, add_column {table, column: {name, type, format?, formula?}}, sort {table, keys: [{column, order: "asc"|"desc"}]}, add_table {sheet, name, anchor, columns, rows?}, add_sheet {name}.
[{"op": "set", "range": "매출!B73", "values": [[18420000]]},
 {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-08", "매출": 12400000}]}]
Numbers are JSON numbers; dates are "YYYY-MM-DD" text (read as dates in date columns); IDs with leading zeros are text. A value that starts with = is stored as text: formulas are written only as a column's formula or with fill_formula, using structured references ([@매출], Sales[매출]). At most 50 new rows per call.
Formatting (xlsx) is shown as read-only lines. `<format default font=Calibri size=11pt color=tx1/>` after the front matter is the Normal style: every cell has it unless a range line says otherwise, and a key it leaves out is none, off, general alignment, bottom or indent 0. In a sheet, `<format range="B4:C7" fill=#D9D9D9 bold/>` gives every cell of that rectangle what it writes beyond the default (style="Name": the cells' named cell style); a cell is in at most one line. Keys: fill (a colour or none), border-top/-right/-bottom/-left (`"<width>pt <style> <colour>"`, style solid, dashed, dotted, double, dash-dot or dash-dot-dot, or none), align (general, left, center, right, justify, distribute), valign (top, middle, bottom), indent (levels), font, size, color, bold, italic, underline, strike. Colours are #RRGGBB or a theme colour by name (tx1, bg1, tx2, bg2, accent1 … accent6, hlink), `+N%` lighter, `-N%` darker. Excel's border widths are hair 0.25pt (dotted), thin 0.75pt, medium 1.5pt, thick 2.25pt; double is 2.25pt; a written width snaps to the nearest.
Formatting is written by format {range, set}: every cell of the range takes the keys `set` writes (flags true/false, size and indent as numbers) and keeps the rest; `border` sets all four sides of every cell, `outline` the outer edges of the range; at most 100000 cells.
[{"op": "format", "range": "매출!A1:D1", "set": {"fill": "accent2+80%", "bold": true, "border-bottom": "0.75pt solid #000000"}}]
fill=gradient and fill=pattern are shown but cannot be written, and nothing else can be said: no gradient, pattern, diagonal line, shadow or conditional format.
