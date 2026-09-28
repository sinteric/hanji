A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts and sizes cannot be written directly.

- A paragraph with a style is `<div style="Name">text</div>`, on one line.
- A table with a style is `<table style="Name">`, then one `<tr>` per line with `<th>` header cells and `<td>` cells, then `</table>`.
- A paragraph or a pipe table without a tag has the default style.

The value of `style` is exactly one style name, written as listed, spaces included. `<div>` and `<table>` take no other attribute.

Example:

```
<div style="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table style="Grid Table 4">
<tr><th>지역</th><th>매출</th></tr>
<tr><td>수도권</td><td>1,204</td></tr>
</table>
```
