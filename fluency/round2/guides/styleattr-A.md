A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts, sizes, alignment, borders and highlighting cannot be written directly.

- A paragraph with a style is `<div style="Name">text</div>`, on one line.
- A table with a style is a line `<table style="Name">`, then a pipe table, then a line `</table>`. A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. Nothing else is between the two tag lines, not even a blank line.
- A paragraph or a pipe table without a tag has the default style.

The value of `style` is exactly one style name, written as listed, spaces included. A paragraph takes a paragraph style and a table a table style. `<div>` and `<table>` take no other attribute.

Example:

```
<div style="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table style="Grid Table 4">
| 지역 | 매출 |
|---|---|
| 수도권 | 1,204 |
</table>
```
