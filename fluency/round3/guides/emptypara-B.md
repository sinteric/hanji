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

### Empty paragraphs

Documents use empty paragraphs for spacing, often several in a row and sometimes with a style. An empty paragraph with the default style is a line holding only `<div></div>`; an empty paragraph with a paragraph style is a line holding only `<div style="Name"></div>`.

- Each empty paragraph is its own line: three empty paragraphs are three such lines, even when they are identical. Consecutive empty paragraphs are on consecutive lines, with no blank line between them; a blank line separates the group from the blocks before and after it, as for any block.
- Nothing is between `<div …>` and `</div>`, not even a space. `<div></div>` is the only `<div>` without a style. The pair stands alone on its line, never inside a line of text or a table cell.
- A blank line is not an empty paragraph, and neither is a line holding only `<br/>`.

Example (two empty paragraphs with the default style after the heading, and one in the style 좁은 간격 after the table):

```
# 알림

<div></div>
<div></div>

행사 일정은 다음과 같다.

| 구분 | 일시 |
|---|---|
| 1차 | 3월 |

<div style="좁은 간격"></div>

문의는 총무과로 한다.
```
