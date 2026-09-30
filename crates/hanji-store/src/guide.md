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
A docx text shows its formatting (below). In hwpx and pptx, formatting is by the file's own style and layout names only: no colours, fonts or sizes; if no style fits a request, say so, do not invent one.
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

Formatting (docx)
Formatting comes from named styles, listed once at the top; each paragraph, cell and stretch of text shows only what differs from its style.
- Style lines: after the front matter, one line `<style name="Name" …/>` per style the text uses. The first is the default style: its line is complete, and a property it leaves out is 0pt, none or off (`align` left). Every other line holds only what differs from the default style. A heading's style is its level's, a `<div>` names its style, a list item without `style="Name"` in its `{…}` is in the file's list style (List Paragraph), any other paragraph is in the default style. Change a style line to change every paragraph in that style that does not set the property itself.
- Paragraph: `{…}` at the end of its line (after `</div>`, or `- item {style="Name" first-line=10pt}`) holds what differs from its style.
- Text: `[text]{…}` gives that stretch the text properties written (font, size, color); the rest of the paragraph has the paragraph's. Bold, italic, underline and strike are the marks `**`, `*`, `<u>`, `~~`. A literal `{` in text is `\{`; in `[text]`, `[` and `]` are `\[`, `\]`.
- Cell: `{…}` at the very start of a cell holds its fill, borders and valign; each cell paragraph ends in its own `{…}`. `{…}` after a row's last `|` applies to every cell of the row; the line `{…}` just before the header row applies to every cell and cell paragraph of the table (it may hold `style="Name"`). A cell's own value wins over its row's, a row's over the table line's.
- Vocabulary: `key=value` pairs separated by spaces, a value with a space quoted; lengths in points (`12pt`); colours `#RRGGBB` or a theme colour by name (accent1, tx1, …, with `+N%` lighter or `-N%` darker). Paragraph: align (left, center, right, justify, distribute), indent-left, indent-right, first-line (negative: a hanging indent), space-before, space-after, line-spacing (`115%`, `14pt` exact, `"at-least 14pt"`), fill, border-top/-right/-bottom/-left. Text: font, size, color. Cell: fill, border (all four sides) or border-top/…, valign (top, middle, bottom). A border is `"<width>pt <style> <colour>"` (solid, dashed, dotted, double) or none.
- `fill=gradient`, `fill=pattern`, `fill=picture` and other border styles are kept while left as written; they can be replaced, not written. What the vocabulary cannot say (a gradient, a pattern, a shadow, a diagonal line) is refused with the reason.
- A new style is a new style line with a name no style has (a name that is already a style is refused); it holds what differs from the default style. A paragraph takes it like any style: `<div style="Name">…</div>`, a list item's `style="Name"`, `<p style="Name"/>` in a cell. The line of a style the text does not use is not shown: give a paragraph that style first to change it.
<style name="Normal" line-spacing=115% font=Calibri size=11pt color=#000000/>
<style name="Heading 1" space-before=12pt size=16pt color=accent1 bold/>
<style name="Callout" fill=#FFF2CC border-left="2.25pt solid #C00000" indent-left=10pt/>

# 개요

본문 문단입니다. [강조]{color=#C00000} 부분이 있습니다. {first-line=10pt}

<div style="Callout">잠정치입니다.</div>

{border="0.5pt solid #000000"}
| 구분 {align=center} | 내용 {align=center} | {fill=#D9D9D9}
|---|---|
| {fill=#FFF2CC} 서울 | 120 |
| 합계 | 215 | {border-top="1.5pt double #000000"}

Presentation (pptx)
Slides are separated by a line `---`. Each slide's first line is `layout: Name` (a layout of the file). Then slot markers on their own lines, each followed by its text: ::title::, ::subtitle::, ::body::, ::left:: and ::right:: (two content areas), ::notes:: (last), and the others the layout has. Only the layout's slots; leave an unfilled slot out; an empty marker is an error. Slots hold paragraphs, - / 1. items and <p/>; no headings, styles or tables.
<shape id="s4" name="출처">text<p/>more</shape> is an existing text shape: edit its text, delete its line to delete it; never create one.
<picture id="s7" name="Picture 6" box="x y w h" src="media/image1.png" crop="10 0 5 0" mask="ellipse" alt="…"/> is a picture (box in points from the slide's top-left corner): change its box, crop (percent cut off the left, top, right and bottom), mask (a preset shape such as ellipse or roundRect; none is a rectangle) or alt (its alternative text); src may name another picture's image. Keep its id and name.
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
