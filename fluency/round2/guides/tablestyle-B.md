A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. Colours, fonts, sizes, alignment, borders and widths cannot be written directly. A paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. A style name is written exactly as listed, spaces included.

### Tables

Every table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. This holds for merged tables too. Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column.
- A merged area is a rectangle; for two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. An empty cell that is not merged is written with a space, `|  |`. The text of a merged cell is written once, in its top-left cell. `^^` never appears in the first row, and `||` never starts a row.

### Table styles

A table with a style has a line `{style="Name"}` directly before its header row: no blank line or other text is between them. The line gives that one table its style; the pipe table itself is unchanged, and nothing closes it. The braces hold nothing but `style="Name"`, and the line belongs to no other block. A pipe table without such a line has the default table style.

Example ("서울" covers two rows, "합계" covers two columns; the table has the style Grid Table 4):

```
{style="Grid Table 4"}
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```
