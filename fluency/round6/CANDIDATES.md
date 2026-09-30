# Round 6 candidates: direct formatting in the text

Today the text shows formatting by name only (`style="Name"`, `{style="Name"}` before a table, `<p style="Name"/>`)
plus `**bold**` and `*italic*`. Fill, borders, colour, font size, alignment and paragraph layout live in the
remainder, so an agent cannot read or change them. The owner (2026-09-30) says users will edit styling like colour
and border style, and that this is important. This file sets out the candidates on real samples from the repo's
corpora, one vocabulary shared by all four formats, and how each candidate attaches, keeps and refuses. The fluency
measurements are in [RESULTS.md](RESULTS.md).

The kinds differ (owner, 2026-09-30): docx and hwpx are text flows that repeat formatting over hundreds of
paragraphs, pptx is a canvas, and xlsx is a grid. The candidates are judged per kind: (a) flow documents, (b)
Presentation shapes, (c) Spreadsheet ranges.

## The candidates

| | flow documents (docx, hwpx) | Presentations | Spreadsheets |
|---|---|---|---|
| **F1** effective, always | every paragraph, cell, run shows its formatting as it looks, from the fixed vocabulary; the one default line after the front matter says what "nothing written" is | every object's tag or slot marker shows its fill, outline and text properties, whether set on the object or taken from the theme or layout (the analogue of `box`, round 5 A) | `<format range="B4:C4" …/>` lines for every rectangle of cells whose formatting is not Normal's |
| **F2** overrides + styles | a style section (`<style name="개요 3" …/>` lines, editable) and, on each paragraph, cell and run, only what differs from its style | tested as **F2o** (the pptx canvas audit's proposal): an object shows only what it sets itself; theme-style and layout values are not shown | named cell styles as style lines; a range shows `style="Name"` and what it sets beyond it |
| **F3** styles only | the style section, editable; direct formatting stays hidden in the remainder | – | ranges show `style="Name"` only |

A fourth candidate, automatic styles (every distinct direct combination named, as ODF's `P1`/`T1` and hwpx's
`charPr`/`paraPr` ids do), was considered and not built: it moves every value one lookup away (the owner's objection
to B in round 5), and an edit of one paragraph through a shared automatic style silently changes every paragraph
that shares it.

## The vocabulary (one for all four formats)

`key=value` pairs, space-separated, a value with spaces quoted. Not CSS: round 1 found models do not write CSS, and a
CSS-like `style="color:red"` is refused by the validator with the list of keys.

| group | keys | values |
|---|---|---|
| paragraph | `align`, `indent-left`, `indent-right`, `first-line`, `space-before`, `space-after`, `line-spacing` | `left center right justify distribute`; lengths in pt; `first-line` positive = first-line indent, negative = hanging; `line-spacing` `160%`, `14pt` (exact), `"at-least 14pt"`, `"gap 4pt"` (hwpx's 여백만 지정, shown, writable only in hwpx) |
| text | `font`, `size`, `color`, `bold`, `italic`, `underline`, `strike` | the font of the paragraph's Korean (East Asian) text; flags bare (`bold`) or `bold=no`; in running text the flags stay the marks `**`, `*`, `<u>`, `~~` |
| box (cell, shape, paragraph shading) | `fill`, `border`, `border-top` / `-right` / `-bottom` / `-left`, `valign` | a border is `"<width>pt <style> <colour>"` or `none`; styles `solid dashed dotted double dash-dot dash-dot-dot`; `border` = four equal sides (canonical form writes it when they are) |
| xlsx only | `indent` | Excel's indent in levels (a pt value is refused) |

- **Lengths:** points, at most two decimals, `pt` written (`0.34pt`, `20pt`). Two decimals because hwpx border
  widths 0.1 mm and 0.12 mm are 0.28pt and 0.34pt; one decimal would show both as 0.3pt. Box numbers keep round 5's
  unitless whole points; formatting lengths carry the unit because a bare `size=20` next to `line-spacing=160%`
  reads ambiguously.
- **Colours:** `#RRGGBB`; theme colours by their OOXML names `tx1 bg1 tx2 bg2 accent1`…`accent6 hlink` (`dk1`,
  `lt1`, `text1`, `background1` read as aliases). `accent1+40%` is Office's "Lighter 40%" (pptx `lumMod`/`lumOff`,
  docx `themeTint`, xlsx `tint`), `accent1-25%` "Darker 25%"; `accent1*` is a theme colour with any other transform
  (`shade`, `satMod`, a non-preset `lumMod`), kept exactly while left as written; `/55%` is opacity. Theme colours are
  kept, not resolved: they are how modern decks and Office themes are built (the pptx audit), a theme change must
  still recolour them, and resolving would turn every theme colour into a fixed one on the first edit.
- **Shown, not writable:** `fill=gradient`, `fill=pattern`, `fill=picture`, border styles `triple`, `thin-thick`,
  `thick-thin`, `wave`, `3d`, and `accent1*`. Left as written they keep the stored XML; they may be replaced by a
  writable value; writing one where none is stored is refused (rule 1).
- **Refused (not expressible):** gradients and patterns as values, diagonal cell borders (hwpx `slash`/`backSlash`,
  docx `tl2br`), shadows, glow and other effects, per-script fonts beyond the Korean one, character spacing and
  width ratio, text highlight. They stay in the remainder, and a request for one is refused with the reason.

## Where formatting attaches, and noise

| element | syntax | holds |
|---|---|---|
| paragraph (plain, heading, list item, `<div>`) | ` {…}` at the end of its line (after `</div>`) | paragraph keys and the text keys every run of it shares; a list item's `style="Name"` too |
| run | `[text]{…}` (Pandoc's bracketed span) | text keys that differ from the paragraph's |
| table | `{…}` line before the header row (today's `{style="Name"}` line) | `style` plus, per key, the value **more than half** of the cells (or of the cells' paragraphs) have |
| row | ` {…}` after the row's last `\|` | per key, the value more than half of the row's cells have, when it differs from the table line |
| cell | `{…}` at the very start of the cell | what differs from the row and table lines; each cell paragraph ends in its own `{…}` |
| shape, line, slot | attributes on the tag or `::slot …::` marker, after `box` | box keys and the text keys the object's paragraphs share; a paragraph's own `{…}` before `<p/>`/`</shape>` or at the end of its slot line |
| xlsx range | `<format range="A1:D1" …/>` lines in the `<sheet>` block; written by a `format` range operation | the cell keys of every cell in the rectangle |

Noise rules, all canonical form (re-applied on every write, like table padding):

1. **Lifting.** A property every run of a paragraph shares is on the paragraph, not on its runs; a property most
   cells share is on the table line, most of a row on the row. On fdi-2025q2.hwpx (a real press release, 38,163
   chars today) row lifting alone took F1 from 167,253 to 74,969 chars.
2. **Merge.** Adjacent runs with equal properties are one run whatever the XML's run boundaries (the remainder keeps
   the boundaries, as it keeps rsids).
3. **Marks stay marks.** Bold, italic, underline and strike in running text stay `**`, `*`, `<u>`, `~~`, so today's
   text does not change where nothing else is set.
4. **Empty paragraphs** (`<p/>` lines, spaces only) show no formatting: their size and spacing stay in the remainder.
5. **Absent means default.** F1: absent is the default line's value (or none); F2: the style's.

## The samples

### hwpx: the title banner (footnote-01.hwpx)

The last table of footnote-01.hwpx is one cell. Its `borderFill` 8: `faceColor="#FFF0C3"`, top `SOLID 0.12 mm
#000000`, bottom `SOLID 1.0 mm #7F7F7F`, left and right `NONE`; the run `charPr` 19: 20pt, bold, font 나눔고딕; the
paragraph `paraPr` 8: centred, 160%. Today: `| **3D 프린팅 기술의 미래와 전망** |`.

```
F1, F2   | {fill=#FFF0C3 border-top="0.34pt solid #000000" border-bottom="2.83pt solid #7F7F7F"} **3D 프린팅 기술의 미래와 전망** {align=center font=나눔고딕 size=20pt} |
F3       | **3D 프린팅 기술의 미래와 전망** |
```

F1 and F2 coincide: hwpx has no cell styles, and the paragraph is in 바탕글, so everything is direct. The table's
own `borderFillIDRef="3"` (all sides 0.12 mm) is not drawn with `cellSpacing="0"` and stays in the remainder.

The body text of the same file, where styles matter (개요 2/3/4 carry indent and size):

```
F1   - 사회적 변화 예상 {style="개요 2" indent-left=10pt space-before=15pt space-after=5pt font=휴먼명조 size=15pt bold}
       - 시제품 제작 시간과 비용 절감 및 이를 통한 제품 혁신의 가속화 {style="개요 3" indent-left=20pt space-before=5pt font=휴먼명조 size=15pt}
F2   <style name="개요 3" indent-left=20pt space-before=5pt font=휴먼명조 size=15pt/>        (once, at the top)
     - 사회적 변화 예상 {style="개요 2" space-before=15pt space-after=5pt}
       - 시제품 제작 시간과 비용 절감 및 이를 통한 제품 혁신의 가속화 {style="개요 3"}
F3   the same style lines;  - 사회적 변화 예상 {style="개요 2"}
```

The large real document, mel-001.hwpx (고용노동부 업무보고, 1,717 paragraphs), is formatted almost entirely directly:
a body paragraph with its hanging indent and a run of smaller text reads the same under F1 and F2.

```
 [ㅇ  **(노동절 입법)**]{size=15pt} … [(｢노동절 제정에 관한 법률｣ 개정, 11.11)]{size=13pt} {first-line=-58.96pt space-before=20pt line-spacing=162% font=함초롬바탕}
```

### docx: a shaded table (docx4j-tables.docx, "Table width")

`w:shd w:fill="FFC000"`, `w:themeFill="background1"`, `w:themeFill="text2" w:themeFillTint="1A"`, and
`w:tblBorders` single `sz=8` in `background1` shaded `1A`:

```
F1, F2   {border="0.5pt solid bg1-90%"}
         | {fill=bg1} A | {fill=#FFC000} |
         |---|---|
         | {fill=tx2+90%} C | ^^ |
F3       | A |  |  …
```

Word styles differ from hwpx: korean-report.docx's headings take size and bold from Heading 2, so F1 writes
`## 지역별 현황 {size=13pt bold}` on each and F2 writes `## 지역별 현황` under `<style name="Heading 2" size=13pt bold/>`.
Runs coloured directly show in both: `[15% 성장]{size=14pt color=#1F4E79}`.

### pptx: cards and connectors (the audit's synthetic modern pitch deck, and shapes.pptx)

```
F1    <shape id="s8" name="Rounded Rectangle 7" box="354 130 260 320" fill=#FFFFFF border=none/>
      <line id="s7" name="Connector 6" from="172 256" to="300 256" border="2pt solid accent1"/>
F2o   <shape id="s8" name="Rounded Rectangle 7" box="354 130 260 320" fill=#FFFFFF border=none/>
      <line id="s7" name="Connector 6" from="172 256" to="300 256"/>
```

The connector's outline comes from its `p:style` `lnRef idx="2"` (theme line 2pt, `accent1`): F1 shows it, F2o does
not. shapes.pptx slide 6 shows the theme tints and the `*` mark (`lnRef` `shade val="50000"`):

```
F1    <shape id="s9" name="Rectangle 8" box="174 72 72 72" fill=accent1+80% border="2pt solid accent1*"/>
F2o   <shape id="s9" name="Rectangle 8" box="174 72 72 72" fill=accent1+80%/>
```

A slot shows what the layout and master give on its marker under F1:
`::title box="36 22 648 90" align=center font=Calibri size=44pt color=tx1::`.

### xlsx: a styled range (simple-monthly-budget.xlsx)

```
F1    <format range="E4" fill=bg2-10% border-top="2.25pt solid bg1" border-left="2.25pt solid bg1"/>
      <format range="F4:G4" fill=bg2-10% border-top="2.25pt solid bg1"/>
      <format range="B4" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" font=Georgia size=10pt color=accent1-25% italic/>
F2    <style name="Heading 2" border-top="1.5pt solid bg2-10%" border-bottom="1.5pt solid bg2-10%" … italic/>
      <format range="B4" style="Heading 2" valign=middle/>
F3    <format range="B4:C4" style="Heading 2"/>
```

Excel border styles show as widths (thin 0.75pt, medium 1.5pt, thick 2.25pt, double 2.25pt double, hair 0.25pt
dotted); a written width snaps to the nearest Excel style and the returned text shows it.

## Keeping the file: GetPut, PutGet, refusals, propagation

- **GetPut (byte-exact where unchanged).** The keep rule of `box`: a value left as shown keeps the stored XML. A
  paragraph whose `{…}` is unchanged keeps its `pPr`/`paraPrIDRef` and its runs' `rPr`/`charPrIDRef`; a cell whose
  `{…}` (and row and table lines) is unchanged keeps its `tcPr`/`borderFillIDRef`; an unchanged tag keeps `spPr` and
  `p:style`. Lifting is only a way of writing: moving a value between the table line and cells, when the effective
  values stay equal, changes nothing in the file. Rounded values (0.34pt for 0.12 mm) keep the stored value when left
  as shown, like a box's EMU.
- **A changed value** writes that property only, as direct formatting on the element: docx a `w:shd`/`w:tcBorders`
  side/`w:rPr` child; hwpx a new `borderFill`, `charPr` or `paraPr` cloned from the element's and changed in one
  field (hanji-hwpx already adds shapes to header.xml); pptx `spPr`/`a:ln`/`a:rPr` over the `p:style` reference,
  which stays. Removing a key reverts it: F1 to the default line (or none), F2 to the style (the direct property is
  removed).
- **PutGet.** The write returns the canonical text: lifting recomputed, `border` folded, lengths rounded, snapped
  widths shown (hwpx: 0.1 0.12 0.15 0.2 0.25 0.3 0.4 0.5 0.6 0.7 1.0 1.5 2.0 3.0 4.0 5.0 mm; docx: eighths of a point,
  0.25–12pt; xlsx: the Excel styles). The kit checks it on the text: every seed, every candidate reads its own
  rendering back to the same effective formatting (28 of 29 flow corpus files, and every part A seed; the miss is
  below).
- **Style edits propagate (F2, F3).** A style line changes the style's definition (docx `w:style`, hwpx the style's
  `paraPr`/`charPr`); every paragraph in that style that does not override the property changes with it, and the
  returned text shows them unchanged (they carry no override). A style line is relative to the default style, so an
  edit of the default line reaches every style that does not set the property: the kit's "indent every body
  paragraph" task on korean-report.docx catches an edit of Normal that also indents headings and table cells.
  Creating a style is refused for now.
- **hwpx paragraph margins** are read from the `hp:switch` case `HwpUnitChar` branch, which the line segments
  confirm (paraPr 14: case `left 2000`, default `4000`; its lines start at `horzpos 2000`); a write sets both
  branches, the default at twice the value. 1pt = 100 HWPUNIT.

| vocabulary | docx | hwpx | pptx | xlsx |
|---|---|---|---|---|
| `align` | `w:jc` | `hh:align horizontal` | `a:pPr algn` | `alignment horizontal` |
| `indent-left` / `-right` | `w:ind left/right` (twips; `leftChars` read, converted) | `hc:left`/`hc:right` | `marL` (EMU) | – (`indent` levels) |
| `first-line` | `w:ind firstLine` / `hanging` (negative) | `hc:intent` | `indent` | – |
| `space-before` / `-after` | `w:spacing before/after` | `hc:prev`/`hc:next` | `a:spcBef`/`a:spcAft spcPts` | – |
| `line-spacing` | `w:spacing line` + `lineRule` (auto = %/240) | `hh:lineSpacing` PERCENT/FIXED/AT_LEAST/BETWEEN_LINES | `a:lnSpc spcPct`/`spcPts` | – |
| `font` / `size` / `color` | `w:rFonts eastAsia`, `w:sz` (half-pt), `w:color`/`themeColor` | `hh:fontRef hangul`, `height` (1/100 pt), `textColor` | `a:ea`/`a:latin`, `sz` (1/100 pt), `a:solidFill` | `font name`, `sz`, `color rgb/theme/tint` |
| `fill` | `w:shd fill`/`themeFill` (+ `pattern`) | `hc:winBrush faceColor` (+ hatch, gradation, image) | `a:solidFill`/`a:noFill`, `p:style fillRef` | `patternFill solid fgColor` |
| `border-*` | `w:tcBorders`, `w:tblBorders`, `w:pBdr` (`sz` eighths) | `hh:leftBorder`… `type width color` | `a:ln w`, `a:prstDash`, `p:style lnRef` | `border left/right/top/bottom style color` |
| `valign` | `w:vAlign` | `hp:subList vertAlign` | `a:bodyPr anchor` | `alignment vertical` |

## Size on the corpus

The candidate's text over today's text, whole files (`measure.py`; data/measure.json):

| format (files) | F1 | F2 | F3 | median F1 / F2 / F3 |
|---|---|---|---|---|
| hwpx (16) | 2.13× | 2.16× | 1.04× | 1.87 / 1.77 / 1.13 |
| docx (13) | 1.40× | 1.38× | 1.12× | 1.38 / 1.37 / 1.17 |
| pptx (19) | 1.27× | F2o 1.14× (F2 with a section 1.23×) | 1.09× | – |
| xlsx (43) | 2.48× (2.04× without 57893-many-merges.xlsx, 40×) | 2.55× | 1.08× | 1.00 / 1.26 / 1.14 |

On hwpx, F2 is no smaller than F1: Hancom documents put nearly all formatting on the element (a `paraPrIDRef` and
`charPrIDRef` per paragraph, a `borderFill` per cell), not in styles. The cost is the formatting itself: cell borders
(34% of mel-001's formatting text), fonts and sizes on runs.

Kit limits: the flow reader is the kit's own (doc.py), not hanji-format; GetPut on the text fails on
form_footnotes.docx under F2 and F3 (two paragraphs: a span at a paragraph edge, a cell style whose name differs in
case), a kit bug, not a design one; pptx text styles are read at level 1 only; the pptx and xlsx renderers do not
read back (their fluency part checks the text).
