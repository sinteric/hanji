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

A table cell holds one or more paragraphs. A table in which any cell holds more than one paragraph, or a paragraph with a style, is written as a list table; every other table stays a pipe table.

- A list table begins with a line `{list-table}`; its `{style="Name"}` line, if it has one, comes directly before that line. Without the `{list-table}` line, the lines below it would be an ordinary list.
- Each row is a line holding only `-`, followed by one line `  - ` (two spaces, `-`, a space) per cell, in column order. The first row is the header row, and every row has one cell per column.
- A cell's first paragraph is on its `  - ` line. Each further paragraph of that cell is its own line, indented four spaces. A paragraph with a style is written there as `<div style="Name">text</div>`. `<br/>` still breaks the line inside one paragraph.
- A covered cell is a `  - ` line holding only `^^` or `||`. The text of a merged cell, all of its paragraphs, is written once, in its top-left cell.
- A list table has no blank line inside; the first blank line ends it.

Example (the 내용 cell of 서울 holds two paragraphs, the second in the style 표 참고; its 비고 cell holds one paragraph in that style and covers the row below; the 내용 cell of 부산 holds two paragraphs with the default style):

```
{style="표 눈금"}
{list-table}
-
  - 지역
  - 내용
  - 비고
-
  - 서울
  - 강남·종로 2곳
    <div style="표 참고">종로는 10월 개점</div>
  - <div style="표 참고">임차</div>
-
  - 부산
  - 해운대 1곳
    서면 1곳
  - ^^
```
