A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. `size` in the front matter is the slide's width and height in points. Slides follow, separated by a line holding only `---`. The first line of every slide is `layout: Name`.

### Objects

A slide is a list of objects written back to front: an object written later is drawn on top. Every object shows where it is: `box="x y w h"` is its left edge, top edge, width and height in points from the slide's top-left corner; `rot` turns it, `flip` mirrors it. Leave ids, names and boxes as they are.

- **Slots** are the layout's placeholders: a marker line `::title box="…"::`, then the slot's text, one line per paragraph (`- ` for a bullet), up to the next object or `---`.
- **Shapes** are one line each, `<shape id="s4" name="…" box="…">text</shape>`, or `<shape … />` without text; `<p/>` starts the shape's next paragraph.
- **Lines and connectors** are `<line id="…" name="…" from="x y" to="x y"/>`.
- **Groups**: a `<group …>` line, the group's objects, then `</group>`.
- **Pictures, charts, tables** are `<keep …/>` lines: kept for you, never changed here.
- Text: `**bold**`, `*italic*`, `<u>underline</u>`, `<br/>` a line break. Speaker notes follow `::notes::`.

### Formatting

Every object shows how it looks: its formatting is written on its tag (a shape, a line) or its slot marker, whether the object sets it itself or takes it from the theme or the layout. A property that is not written is not there: no `fill` means no fill, no `border` means no outline.

- Text properties shared by all of an object's text are on its tag or marker; a paragraph's own are in `{…}` at its end (before `<p/>` or `</shape>` in a shape, at the end of the line in a slot); `[text]{…}` gives a stretch of text its own `size`, `color` or `font`.
- To change how an object looks, change or add the property on its tag.

Example: `<shape id="s3" name="Card" box="64 130 260 320" fill=accent1 border="2pt solid accent1-50%" size=18pt color=#FFFFFF>**Cities** {size=24pt}<p/>Low-emission [zones]{color=#FF6B5B} in 40 cities</shape>`

### The vocabulary

`key=value` pairs separated by spaces, on the object's tag or slot marker; a value with a space is quoted. Lengths are points (`12pt`).

- colours: `#RRGGBB`, or a theme colour `accent1` … `accent6`, `tx1`, `bg1`, `tx2`, `bg2`, `hlink`; `accent1+40%` is 40% lighter and `accent1-25%` 25% darker; `accent1*` is the theme colour with another adjustment the file keeps while it is left as written; `/55%` after a colour is its opacity
- shape and line: `fill` (a colour, or `none`), `border` (the outline: `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`)
- text: `font`, `size`, `color`, `bold`; paragraph: `align` (left, center, right, justify), `indent-left`, `first-line`, `space-before`, `space-after`, `line-spacing` (`90%`, `14pt`)
- `fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written, and can be replaced by a colour; they cannot be written or changed. Shadows, glow, 3-D and other effects cannot be expressed.
