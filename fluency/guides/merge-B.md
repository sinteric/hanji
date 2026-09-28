A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Tables

Every table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row. Every row has one cell per column, merged or not.

Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column.
- A merged area is a rectangle. For two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. An empty cell that is not merged is written with a space, `|  |`.
- The text of a merged cell is written once, in its top-left cell; the covered cells hold only a marker, never text.

Example ("서울" covers two rows, "합계" covers two columns):

```
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```
