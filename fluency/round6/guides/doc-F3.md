A document file is plain text. It begins with a front matter block between two `---` lines; keep it as it is.

### Text

- One line is one paragraph. `#` to `######` start a heading of that level. `- ` starts a bullet item and `1. ` a numbered item; a nested item is indented under its parent (two spaces under `- `).
- `<div style="Name">text</div>` is a paragraph in the named style. A line holding only `<p/>` or `<p style="Name"/>` is an empty paragraph.
- Inside a paragraph: `**bold**`, `*italic*`, `<u>underline</u>`, `~~strike~~`, `<br/>` (a line break). A literal `*`, `[`, `]` or `{` in text is written `\*`, `\[`, `\]`, `\{`.
- `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a footnote, a comment). Leave every one exactly as it is. `<pagebreak/>` is a page break.

### Tables

A table is a pipe table: a header row, a `|---|` line, then the other rows, one line per row, one cell per column. `^^` as a whole cell means the cell is merged into the one above; `||` (two pipes with nothing between) means the cell to the left extends into this column. In a cell, `<p/>` starts another paragraph and `<p style="Name"/>` starts one in that style; a cell that begins with `<p style="Name"/>` has its first paragraph in that style. An empty cell is `|  |`.

### Formatting

Formatting comes from named styles, listed once at the top of the file. Only styles can be read and changed here.

- **The style lines.** After the front matter, each style the document uses has a line `<style name="Name" …/>`. The first is the default style: its line is complete, and a property it leaves out is 0pt, none or off (and `align` is left). Every other style line holds only what differs from the default style; a property it leaves out is the default style's. A heading's style is its level's, a `<div>` names its style, a list item names it in its `{…}` (`{style="Name"}`, the only thing such a `{…}` holds), a cell paragraph after `<p style="Name"/>` is in that style, and any other paragraph is in the default style. Change a style line to change every paragraph in that style.
- **Direct formatting is not shown.** Formatting set on one paragraph, cell or stretch of text (a colour on a word, a cell's fill or borders, one paragraph's indent) is kept for you exactly as it is, but it is not in this text and cannot be read or changed here. Bold, italic, underline and strike are the marks `**`, `*`, `<u>`, `~~`, and can be changed.
- **Changing a paragraph's look** is done by giving it another style (`<div style="Name">`, a list item's `style`, a heading level) or by changing its style's line.

### The vocabulary

A style line holds `key=value` pairs separated by spaces; a value with a space is quoted. Lengths are points (`12pt`); colours are `#RRGGBB`.

- paragraph: `align` (left, center, right, justify, distribute), `indent-left`, `indent-right`, `first-line` (a positive value indents the first line, a negative one is a hanging indent), `space-before`, `space-after`, `line-spacing` (`160%`, `14pt` exact, `"at-least 14pt"`), `fill`, `border-top` / `-right` / `-bottom` / `-left`
- text: `font`, `size`, `color`, `bold`
- a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`

Tables, cells and rows have no formatting of their own here; a table's `{style="Name"}` line names its table style. New styles cannot be created. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow.

Example:

```
<style name="바탕글" align=justify line-spacing=160% font=바탕 size=10pt color=#000000/>
<style name="개요 1" size=16pt bold/>
<style name="본문" first-line=10pt/>

# 개요
<div style="본문">본문 문단입니다. **강조** 부분이 있습니다.</div>
| 구분 | 내용 |
|---|---|
| 합계 | 215 |
```
