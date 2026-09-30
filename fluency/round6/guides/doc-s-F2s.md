A document file is plain text. It begins with a front matter block between two `---` lines; keep it as it is.

### Text

- One line is one paragraph. `#` to `######` start a heading of that level. `- ` starts a bullet item and `1. ` a numbered item; a nested item is indented under its parent (two spaces under `- `).
- `<div style="Name">text</div>` is a paragraph in the named style. A line holding only `<p/>` or `<p style="Name"/>` is an empty paragraph.
- Inside a paragraph: `**bold**`, `*italic*`, `<u>underline</u>`, `~~strike~~`, `<br/>` (a line break). A literal `*`, `[`, `]` or `{` in text is written `\*`, `\[`, `\]`, `\{`.
- `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a footnote, a comment). Leave every one exactly as it is. `<pagebreak/>` is a page break.

### Tables

A table is a pipe table: a header row, a `|---|` line, then the other rows, one line per row, one cell per column. `^^` as a whole cell means the cell is merged into the one above; `||` (two pipes with nothing between) means the cell to the left extends into this column. In a cell, `<p/>` starts another paragraph and `<p style="Name"/>` starts one in that style; a cell that begins with `<p style="Name"/>` has its first paragraph in that style. An empty cell is `|  |`.

### Formatting

Formatting comes from named styles, listed once at the top of the file; a defaults line holds what most paragraphs of a page share; each paragraph, cell and stretch of text shows only what differs from those.

- **The style lines.** After the front matter, each style the document uses has a line `<style name="Name" …/>`. The first is the default style: its line is complete, and a property it leaves out is 0pt, none or off (and `align` is left). Every other style line holds only what differs from the default style; a property it leaves out is the default style's. A heading's style is its level's, a `<div>` names its style, a list item names it in its `{…}` (`style="Name"`), and any other paragraph is in the default style. Change a style line to change every paragraph in that style that does not set the property itself (for the default style: nor has it from a defaults line).
- **The defaults lines.** A line `<defaults …/>` holds the formatting most paragraphs share from that line to the next `<defaults …/>` line (usually a page's worth, after a `<pagebreak/>`); `<defaults/>` holds nothing. A paragraph's property is, first found: its own `{…}`, the table line (in a table), its style's line when the style is not the default style and the line sets the property, the defaults line in force, the default style's line. So a paragraph in the default style takes the defaults line's values, and another style's own values win over them. A change to a defaults line changes every paragraph after it, up to the next one, that does not set the property itself or through its style.
- **Paragraph:** `{…}` at the end of the paragraph's line (after `</div>` for a `<div>`) is its own formatting: the properties that differ from what its style and the defaults line give it. A property left out comes from them, in the order above.
- **Text in a paragraph:** `[text]{…}` gives that stretch the text properties written (font, size, color); the rest of the paragraph has the paragraph's. Bold, italic, underline and strike are the marks `**`, `*`, `<u>`, `~~`.
- **Cell:** `{…}` at the very start of a cell is the cell's formatting: fill, borders and vertical alignment. A cell's paragraphs have their own `{…}` at their end. `{…}` after a row's last `|` applies to every cell of that row. The line `{…}` just before a table applies to every cell and every cell paragraph of the table. A cell's own value wins over its row's, and a row's over the table line's; a table line may also hold `style="Name"`.

### The vocabulary

`key=value` pairs separated by spaces; a value with a space is quoted. Lengths are points (`12pt`); colours are `#RRGGBB`.

- paragraph: `align` (left, center, right, justify, distribute), `indent-left`, `indent-right`, `first-line` (a positive value indents the first line, a negative one is a hanging indent), `space-before`, `space-after`, `line-spacing` (`160%`, `14pt` exact, `"at-least 14pt"`), `fill`, `border-top` / `-right` / `-bottom` / `-left`
- text: `font`, `size`, `color`, `bold`
- cell: `fill`, `border` (all four sides) or `border-top` / `-right` / `-bottom` / `-left`, `valign` (top, middle, bottom)
- a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`

`fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written; they can be replaced by a colour but not written or changed. Border styles other than the four above (triple, wave, 3d) are kept while left as written. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow. A new style is a new style line, after the others, with a name no other style has (a name that is already a style is an error); like the other lines it holds what differs from the default style. A paragraph takes it like any style: `<div style="Name">…</div>`, a list item's `style="Name"`, `<p style="Name"/>` in a cell.

Example:

```
<style name="바탕글" align=justify line-spacing=160% font=바탕 size=10pt color=#000000/>
<style name="개요 1" size=16pt bold/>

<defaults line-spacing=170% size=11pt/>
# 개요
본문 문단입니다. [강조]{color=#C00000} 부분이 있습니다. {first-line=10pt}
{border="0.5pt solid #000000"}
| {fill=#D9D9D9} 구분 {align=center} | {fill=#D9D9D9} 내용 {align=center} |
|---|---|
| 합계 | 215 | {border-top="1.5pt double #000000"}
```
