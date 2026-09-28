A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items (a nested item is indented two spaces), `**bold**` and `*italic*`. A blank line only separates blocks; it is never a paragraph itself. Inside a paragraph, `<br/>` is a line break: the text goes on in the same paragraph. A line holding only `<pagebreak/>` is a page break. Besides Markdown, only the tags and markers described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. Colours, fonts, sizes, spacing, alignment, borders and shading cannot be written directly. A paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. A style name is written exactly as listed, spaces included.

### Tables

A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column, merged or not. Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column. A span over three columns is `|||`, with no space between the pipes.
- A merged area is a rectangle; for two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. The text of a merged cell is written once, in its top-left cell. An empty cell that is not merged is written with a space, `|  |`. `^^` never appears in the first row, and `||` never starts a row.

A table with a style has a line `{style="Name"}` directly before it, with no blank line between; the braces hold nothing but `style="Name"`, and nothing closes the table. A table without that line has the default table style.

Example ("서울" covers two rows, "합계" covers two columns; the table has the style Grid Table 4):

```
{style="Grid Table 4"}
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```

### Cells with several paragraphs

A table cell holds one or more paragraphs, all written on the line of its row. Inside a cell, `<p/>` ends one paragraph and starts the next one, which has the default style; `<p style="Name"/>` does the same and gives the next paragraph the paragraph style Name.

- The first paragraph of a cell has no tag in front of it, unless it has a style: then the cell begins with `<p style="Name"/>`, directly followed by its text.
- Every paragraph in a cell has text, so a cell never ends with a tag. `<p/>` is not `<br/>`: `<br/>` breaks the line inside one paragraph, while `<p/>` starts a new paragraph.
- `<p/>` and `<p style="Name"/>` are single tags; there is no `</p>`, and `<p>` takes no other attribute. They are written only inside table cells, and a paragraph in a cell never uses `<div>`.
- A covered cell still holds only `^^` or `||`. The text of a merged cell, all of its paragraphs, is written once, in its top-left cell.
- Every table is a pipe table, whatever its cells hold; only the cells that need it contain these tags.

Example (the 내용 cell of 서울 holds two paragraphs, the second in the style 표 참고; its 비고 cell holds one paragraph in that style and covers the row below; the 내용 cell of 부산 holds two paragraphs with the default style):

```
{style="표 눈금"}
| 지역 | 내용 | 비고 |
|---|---|---|
| 서울 | 강남·종로 2곳<p style="표 참고"/>종로는 10월 개점 | <p style="표 참고"/>임차 |
| 부산 | 해운대 1곳<p/>서면 1곳 | ^^ |
```
