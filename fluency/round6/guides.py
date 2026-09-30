"""The syntax guide each subject reads: a shared part (hanji's Document text) and one part per candidate.

The candidate parts have the same structure (what is shown, where it attaches, the vocabulary, what cannot be
written, an example) and never mention each other."""

SHARED = """A document file is plain text. It begins with a front matter block between two `---` lines; keep it as it is.

### Text

- One line is one paragraph. `#` to `######` start a heading of that level. `- ` starts a bullet item and `1. ` a numbered item; a nested item is indented under its parent (two spaces under `- `).
- `<div style="Name">text</div>` is a paragraph in the named style. A line holding only `<p/>` or `<p style="Name"/>` is an empty paragraph.
- Inside a paragraph: `**bold**`, `*italic*`, `<u>underline</u>`, `~~strike~~`, `<br/>` (a line break). A literal `*`, `[`, `]` or `{` in text is written `\\*`, `\\[`, `\\]`, `\\{`.
- `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a footnote, a comment). Leave every one exactly as it is. `<pagebreak/>` is a page break.

### Tables

A table is a pipe table: a header row, a `|---|` line, then the other rows, one line per row, one cell per column. `^^` as a whole cell means the cell is merged into the one above; `||` (two pipes with nothing between) means the cell to the left extends into this column. In a cell, `<p/>` starts another paragraph and `<p style="Name"/>` starts one in that style; a cell that begins with `<p style="Name"/>` has its first paragraph in that style. An empty cell is `|  |`.
"""

F1 = """### Formatting

Formatting is shown where it applies, in a small fixed vocabulary, so every paragraph, cell and stretch of text reads on its own.

- **The default line.** The line `<style name="…" …/>` after the front matter is the document's default formatting: what a paragraph has when nothing else is written. A property it leaves out is 0pt, none or off (and `align` is left).
- **Paragraph:** `{…}` at the end of the paragraph's line (after `</div>` for a `<div>`) is its complete formatting: every property that differs from the default line. A property left out has the default line's value. A list item's `{…}` also names its style, `style="Name"`. A paragraph without `{…}` has the default formatting. A new paragraph written without `{…}` gets the formatting of its style (its heading level, its `<div>` style, or its list item's `style`).
- **Text in a paragraph:** `[text]{…}` gives that stretch the text properties written (font, size, color); the rest of the paragraph has the paragraph's. Bold, italic, underline and strike are the marks `**`, `*`, `<u>`, `~~`.
- **Cell:** `{…}` at the very start of a cell is the cell's formatting: fill, borders and vertical alignment. A cell's paragraphs have their own `{…}` at their end. `{…}` after a row's last `|` applies to every cell of that row. The line `{…}` just before a table applies to every cell and every cell paragraph of the table. A cell's own value wins over its row's, and a row's over the table line's; a table line may also hold `style="Name"`.

### The vocabulary

`key=value` pairs separated by spaces; a value with a space is quoted. Lengths are points (`12pt`); colours are `#RRGGBB`.

- paragraph: `align` (left, center, right, justify, distribute), `indent-left`, `indent-right`, `first-line` (a positive value indents the first line, a negative one is a hanging indent), `space-before`, `space-after`, `line-spacing` (`160%`, `14pt` exact, `"at-least 14pt"`), `fill`, `border-top` / `-right` / `-bottom` / `-left`
- text: `font`, `size`, `color`, `bold`
- cell: `fill`, `border` (all four sides) or `border-top` / `-right` / `-bottom` / `-left`, `valign` (top, middle, bottom)
- a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`

`fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written; they can be replaced by a colour but not written or changed. Border styles other than the four above (triple, wave, 3d) are kept while left as written. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow.

Example:

```
<style name="바탕글" align=justify line-spacing=160% font=바탕 size=10pt color=#000000/>

# 개요 {size=16pt bold}
본문 문단입니다. [강조]{color=#C00000} 부분이 있습니다. {first-line=10pt}
{border="0.5pt solid #000000"}
| {fill=#D9D9D9} 구분 {align=center} | {fill=#D9D9D9} 내용 {align=center} |
|---|---|
| 합계 | 215 | {border-top="1.5pt double #000000"}
```
"""

F2 = """### Formatting

Formatting comes from named styles, listed once at the top of the file; each paragraph, cell and stretch of text shows only what differs from its style.

- **The style lines.** After the front matter, each style the document uses has a line `<style name="Name" …/>`. The first is the default style: its line is complete, and a property it leaves out is 0pt, none or off (and `align` is left). Every other style line holds only what differs from the default style; a property it leaves out is the default style's. A heading's style is its level's, a `<div>` names its style, a list item names it in its `{…}` (`style="Name"`), and any other paragraph is in the default style. Change a style line to change every paragraph in that style that does not set the property itself.
- **Paragraph:** `{…}` at the end of the paragraph's line (after `</div>` for a `<div>`) is its own formatting: the properties that differ from its style. A property left out is the style's.
- **Text in a paragraph:** `[text]{…}` gives that stretch the text properties written (font, size, color); the rest of the paragraph has the paragraph's. Bold, italic, underline and strike are the marks `**`, `*`, `<u>`, `~~`.
- **Cell:** `{…}` at the very start of a cell is the cell's formatting: fill, borders and vertical alignment. A cell's paragraphs have their own `{…}` at their end. `{…}` after a row's last `|` applies to every cell of that row. The line `{…}` just before a table applies to every cell and every cell paragraph of the table. A cell's own value wins over its row's, and a row's over the table line's; a table line may also hold `style="Name"`.

### The vocabulary

`key=value` pairs separated by spaces; a value with a space is quoted. Lengths are points (`12pt`); colours are `#RRGGBB`.

- paragraph: `align` (left, center, right, justify, distribute), `indent-left`, `indent-right`, `first-line` (a positive value indents the first line, a negative one is a hanging indent), `space-before`, `space-after`, `line-spacing` (`160%`, `14pt` exact, `"at-least 14pt"`), `fill`, `border-top` / `-right` / `-bottom` / `-left`
- text: `font`, `size`, `color`, `bold`
- cell: `fill`, `border` (all four sides) or `border-top` / `-right` / `-bottom` / `-left`, `valign` (top, middle, bottom)
- a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`

`fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written; they can be replaced by a colour but not written or changed. Border styles other than the four above (triple, wave, 3d) are kept while left as written. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow. New styles cannot be created.

Example:

```
<style name="바탕글" align=justify line-spacing=160% font=바탕 size=10pt color=#000000/>
<style name="개요 1" size=16pt bold/>

# 개요
본문 문단입니다. [강조]{color=#C00000} 부분이 있습니다. {first-line=10pt}
{border="0.5pt solid #000000"}
| {fill=#D9D9D9} 구분 {align=center} | {fill=#D9D9D9} 내용 {align=center} |
|---|---|
| 합계 | 215 | {border-top="1.5pt double #000000"}
```
"""

F3 = """### Formatting

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
"""

GUIDES = {'F1': SHARED + '\n' + F1, 'F2': SHARED + '\n' + F2, 'F3': SHARED + '\n' + F3}
