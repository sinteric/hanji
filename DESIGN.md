---
status: draft
date: 2026-09-29
measured: library and project facts below were checked 2026-09-28/29 against crates.io, npm, PyPI, GitHub and project docs; the §5 syntax choices were measured by fluency rounds 1–4 (2026-09-28, two Claude models) and the Presentation geometry by round 5 (2026-09-29, the same models); direct formatting (§10 item 10, proposed) by round 6 (2026-09-30, the same models); the remainder anchoring (§10.3) by the remainder prototype (2026-09-28, 13 docx); nothing here has yet been opened in real Office by this project
---

# hanji — office documents for LLM agents (design)

A text format and a toolchain that let a language model **read, write and edit
office documents** — Documents, Presentations and Spreadsheets, new or
existing — with export and preview, **without losing what the format does not
model**.

The name is 한지, Korean mulberry paper, known for lasting ("지천년 견오백" —
paper a thousand years, silk five hundred): rule 1 below. Package names,
checked 2026-09-29: `hanji` is free on crates.io and PyPI; on npm it is taken
by an unrelated CLI library, so JavaScript packages use a scope.

## 1. The problem

Agents are asked to draft a report in the company template, update last
month's deck with this month's numbers, fill a Korean government form, or fix
a spreadsheet. Today each of those is one of:

- **Generate from scratch** (pandoc, docx/pptxgenjs builders): no existing
  files, no templates beyond styles, and the result is thrown away the moment
  a person polishes it in Office.
- **Edit the package XML directly**: the model reverse-engineers OOXML every
  time; slow, token-heavy, and a single misplaced element produces Word's
  "unreadable content" prompt.
- **Operation APIs over the file** (Office.js, SuperDoc, rhwp's MCP tools): the
  file stays intact, but the model addresses content by index/ID through many
  tool calls and cannot simply rewrite a paragraph.

None of them gives the model what it is actually fluent in — text it can read,
write and edit — while guaranteeing that everything else in the file survives.

## 2. Ground rules

1. **Preserve what is not supported.** Anything in a file the format does not
   model survives every edit cycle — by the model, by the system, or by a
   person in Office — unless it is unsafe (§8). When something cannot be
   preserved, the operation refuses and says why; it never drops silently.
2. **LLMs read, write and edit it fluently.** Fluency is a design input, not a
   test at the end (§6).
3. **Existing and new files follow one path.** A new file is an existing file
   that starts from a blank or template package.
4. **Export and preview.** Preview renders exactly the bytes export would
   produce.
5. **Preview fidelity above 90%** of what the native application shows, for
   both new and existing files. Past the point where fidelity costs messy work,
   people polish in Office and rule 1 keeps their polish.
6. **Rules 3–5 are implementation work, but the design must make them
   possible now.** Rule 1 and rule 5 together decide the architecture (§4).

Found while testing the rules, and adopted:

7. **A person's edits come back in.** A file polished in Office is re-imported;
   changes to modelled parts appear in the text, and a concurrent model edit
   is merged (on the text) or refused — never guessed.
8. **The model sees what it cannot edit.** Unmodelled content appears in the
   text as a placeholder the model keeps; deleting one is an explicit act.
9. **Preserved content can be unsafe.** Active and remote content is
   neutralised on import; hidden content is surfaced before export (§8).
10. **Format scope is decided up front.** Ruled 2026-09-28 — in: docx,
    pptx, xlsx, hwp/hwpx; later or out: odt/odp/ods, legacy binary
    doc/xls/ppt, Google formats (reachable by exporting them to OOXML).
11. **Large files need partial views** — a 100-page Document or a 100k-row
    Spreadsheet is never one text blob in a prompt.

## 3. Terms and non-goals

**Office types:** **Document** (word-processing; ODF "text document", OOXML
WordprocessingML; reflowable), **Presentation** (deck), **Spreadsheet**
(workbook). Markdown and Typst are not office types and are out of scope.

| Term | Meaning |
|---|---|
| **model** | The canonical text of one office file in this project's format — what the model and people edit |
| **remainder** | Everything an imported file holds that the model does not represent, stored as structured entries anchored to model elements (not as a zip) |
| **home format** | The package format a file exports to (docx, hwpx, pptx, xlsx) |
| **template** | The package a new file starts from; supplies styles, layouts, theme, page setup |
| **placeholder** | `<keep …/>` in the text — a visible handle on a remainder entry |
| **import / export** | package → model + remainder / model + remainder → package |

**Non-goals**

- **No conversion across types.** Presentation → Document (or the reverse) is
  not supported. *Generation* is different and fine: a model writes a new
  Presentation using a Document as a source.
- Not an Office editor, not a pixel-perfect renderer, not a macro runtime.
- Conversion *within* a type (docx ↔ hwpx) is allowed, with a loss report for
  whatever the remainder cannot carry across.

## 4. Architecture — the model is the truth, the remainder keeps the rest

```
            import (split)                    export (recombine)
package ─────────────────────► model text ──────────────────────► package ──► preview (render)
                         └───► remainder  ───┘        ▲
                                                      │ edits (model: edit/apply_patch/write; person: editor)
```

- **What is stored:** the model (canonical text, revisioned) + the remainder +
  home format + template. The package is not stored as the truth.
  *Why not the package:* reading or editing it needs an unpacking layer (zip,
  parts, relationships) at every step, and diffs, merges and reviews of a
  package are unreadable. Text is what the model edits and what merges well.
  *Counter-evidence to keep in mind:* SuperDoc moved the other way (V1 used a
  ProseMirror tree as truth; V2 is OOXML-native, because "a DOCX is a package
  of related XML parts … rather than one editor tree"). Their reason is an
  interactive editor with collaboration over the whole package; ours is agent
  text editing plus a remainder. The remainder's anchoring (§5) is where that
  choice will be tested hardest. §10.3 gives the chosen anchoring and its
  measured residual losses ([REMAINDER.md](prototype/remainder/REMAINDER.md)).
- **Import** parses the package with a format engine, emits the model text,
  and moves everything unmodelled into the remainder, anchored to the model
  element it belongs to (block, run range, slide, shape, cell range).
- **Export** writes the model through the engine and re-inserts every
  remainder entry at its anchor. **Preview** renders the exported package
  with the engine's renderer — same bytes, so preview and export cannot drift.
- **Identity without IDs in the text.** Edits are exact spans against a known
  revision (read-before-edit), so the system knows which blocks and runs
  changed and re-anchors remainder entries by diff. Only placeholders carry
  visible IDs. A whole-file rewrite is aligned block-by-block by diff.
- **Human polish.** A file edited in Office is re-imported as a new revision:
  modelled changes show up as a text diff; remainder changes are kept
  silently. A model edit made in between is rebased as a text merge, or
  refused when its target no longer exists.
- **Laws (from bidirectional transformations / lenses), as tests:**
  - *GetPut:* import then export with no edit returns the original —
    XML-equivalent per part (canonicalised), since byte identity does not
    survive a split and recombine. Geometry is exact too: a Presentation shows
    each box rounded, and a number left as shown keeps the stored EMU, so an
    unchanged `a:xfrm` is not touched (§5.3).
  - *PutGet:* after any edit, importing the exported file shows exactly the
    text that was written, in canonical form (§5.1). The write returns that
    text, so the model sees, for example, rounded numbers, ids on new objects,
    and a group's objects after the group was moved.

## 5. The format

### 5.1 Rules for all three types

- **Markdown for prose; a small closed set of HTML-like tags for what markdown
  cannot express.** Every tag has a schema; the set is closed, extensible only
  by a host through a registered schema (e.g. a citation tag).
- **Canonical form, re-applied on every write:** no hard wrapping (one
  paragraph = one line), no table padding, `1.` for every ordered item, fixed
  attribute order, numbers as shown (whole points, §5.3). *Why:* a formatter that re-pads a table or renumbers a list
  turns a one-cell edit into a whole-table rewrite, and the model's next exact
  `old → new` edit stops matching.
- **Local, never counted.** Nothing requires the author to count columns,
  spans or items.
- **Formatting by name only** — the file's own paragraph/character/table
  styles, slide layouts, cell styles and number formats. Direct formatting
  (a colour, a font size) is remainder: preserved for people, not edited by
  the model. *Proposed replacement (§10 item 10, round 6):* formatting is
  named styles plus a small fixed vocabulary of direct properties, one for
  all four formats, written as `key=value` attributes (not CSS):
  - paragraph `align`, `indent-left`, `indent-right`, `first-line` (negative
    is a hanging indent), `space-before`, `space-after`, `line-spacing`
    (`160%`, `14pt`, `"at-least 14pt"`); text `font`, `size`, `color`,
    `bold`, `italic`, `underline`, `strike` (in running text the marks
    `**`, `*`, `<u>`, `~~`); box `fill`, `border` or `border-top`/`-right`/
    `-bottom`/`-left` (`"<width>pt <style> <colour>"`, style solid, dashed,
    dotted, double, dash-dot, dash-dot-dot, or `none`), `valign`; xlsx
    `indent` in levels.
  - Lengths in points with `pt`, at most two decimals. Colours `#RRGGBB` or
    theme names `tx1 bg1 tx2 bg2 accent1`…`accent6 hlink`, kept as names:
    `accent1+40%` lighter, `accent1-25%` darker (Office's presets),
    `accent1*` any other transform, `/55%` opacity.
  - Shown and kept, not writable: `fill=gradient|pattern|picture`, border
    styles `triple`, `thin-thick`, `thick-thin`, `wave`, `3d`, and `accent1*`;
    left as written they keep the stored XML, and they may be replaced by a
    writable value. Refused: gradients and patterns as values, diagonal
    cell borders, shadows and other effects, highlight, character spacing.
  - One spelling: `hanji-format`'s `vocab` module parses every value a
    text writes and writes it back canonically (lengths in hundredths of a
    point, `0.34pt`; colours with their tint and opacity; borders; fills;
    line spacing; flags; `key=value` lists, a value quoted only when it
    holds a space), so each format's engine maps the same values to its
    XML.
  - Keep rule, as for `box` (§5.3): a value left as shown keeps the stored
    XML; a changed value writes that property only, as direct formatting.
    The write returns the canonical text (rounded and snapped values shown,
    §5.2 lifting re-applied).
  - Per kind (owner, 2026-09-30): flow documents show named styles and the
    direct properties that differ from them (§5.2); Presentations show
    every object's effective formatting inline (§5.3); Spreadsheets show
    formatted ranges (§5.4).
- **Placeholders:** `<keep id="k3" kind="drawing" summary="org chart, 5 boxes"/>`
  (block or inline). The model may move or delete one explicitly, and in a
  Presentation resize it by its box (§5.3); it never alters its content, and
  never creates one. (A Presentation's pictures are not placeholders: they
  are `<picture/>` lines, §5.3.)
- **Validator errors are written for the model:** line and column, the
  expected form, and the allowed names (styles, layouts, fields). Every
  error names the characters a reader cannot see or tell from a space by
  code point, as `⟨U+2007 FIGURE SPACE⟩` or `⟨U+F076 private use⟩`
  (whitespace variants, invisible and format characters, private use,
  controls), and an `old` that misses only by them is told where it would
  match. The text keeps them as they are: marking them in the read view
  would change the text edits match against (GetPut).
- **Front matter** names the type, home format, template and schema version.
  Every file carries its schema version; migrations are part of the toolchain
  from day one.

**Shared inline core:** text, `**bold**`, `*italic*`, `~~strike~~`, `<u>`,
links, footnote references `[^1]`, math `$…$` or `<math>…</math>` (→ OMML /
HWP equation), line break `<br/>`, `<field name=…>value</field>` (Word
content controls, HWP form fields), inline `<keep/>`.

- **Line break:** `<br/>` is a line break inside one paragraph (docx `w:br`,
  the HWP line break). It is valid in paragraphs, headings, list items and
  table cells, and it never starts a paragraph: a line holding only `<br/>`
  is a paragraph holding one line break.
- **Math:** `$…$` opens at a `$` followed by a non-space and closes at the
  next `$` that follows a non-space and is not followed by a digit (pandoc's
  rule), so `$5 and $10` is text. Math that `$…$` cannot hold is written
  `<math>…</math>`, its body raw text up to `</math>` on the same line. That
  covers math directly followed by a digit (`<math>x</math>5`, since `$x$5` is
  text), math starting or ending with a space, and math containing `$`.
  Canonical form writes `$…$` wherever it reads back and `<math>` only
  otherwise. *Why:* it is the smallest rule that keeps `$5 and $10` as text.
  It needs no new escape, the tag joins the existing tag set, and the common
  `$x^2$` is unchanged.

### 5.2 Document

```
---
type: document
format: docx            # or hwpx
template: org/report
schema: 1
---
# 3분기 영업 보고
매출은 전년 대비 **12%** 증가했다.[^1]

<div style="Note">신규 고객 34곳 중 21곳이 수도권.</div>

- 신규 고객 34곳
  - 수도권 21곳
1. 다음 분기 목표

<p/>
<p style="좁은 간격"/>

| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |

{style="Grid Table 4"}
| 지역 | 매출 | 증감 |
|---|---|---|
| 수도권 | 1,204 | +15% |
| 지방 | 812 | +4%<p/>부산 신규 2곳<p style="표 참고"/>잠정치 |

<field name="작성자">홍길동</field>
<keep id="k3" kind="drawing" summary="조직도, 상자 5개"/>
<pagebreak/>

[^1]: 내부 집계 기준.
```

- `#`–`######` map to the file's own Heading 1–6; `style` names come from the
  file's style set (names in any language).
- `style="Name"` holds exactly one style name, written as listed, spaces
  included (`style="Grid Table 4"`); never CSS. A styled paragraph is
  `<div style="Name">text</div>` on one line.
- Lists: GFM `- ` (bullet) and `1. ` (numbered) items, one paragraph per line.
  - A nested item is indented to its parent's text: 2 spaces under `- `, 3
    under `1. `. That is the content column, so nothing is counted. A list
    starts at the margin and nests one level at a time.
  - Canonical form writes `1.` for every numbered item. Consecutive items are
    consecutive lines. A blank line ends a list, so two lists are separated by
    one, and an item written after one starts a new list. An item at the
    margin of the other kind (`1.` after `- ` items, or `- ` after `1.`)
    starts a new list too, as in GFM; canonical form puts the blank line
    before it.
  - The text says only bullet or numbered, and the level. The item's paragraph
    style, list definition and number format stay in the remainder.
  - A new item takes the numbering of its nearest sibling: at its level, or the
    same list one level off. A new list with no sibling takes the template's
    default bullet or decimal list (a numbered one restarts at 1). It is
    refused, with the reason, if the template has neither.
  - An item is a single paragraph; `<br/>` breaks a line inside it. A paragraph
    continuing an item without a number of its own is an ordinary paragraph
    after the list, and its layout stays in the remainder.
  - Inside a table cell, items are not written as list lines: a numbered cell
    paragraph is an ordinary cell paragraph, and its numbering stays in the
    remainder. A numbered heading stays a heading (`#`), with its numbering
    in the remainder.
  - *Why:* both cases keep the one-line forms (pipe rows, `#` lines) and lose
    nothing. A list inside a cell would need its own syntax, which §6 has not
    tested.
- Every table is a GFM pipe table, merged or not. Every row has one cell per
  column; merged cells are written with local markers in the covered cells:
  - `^^` as the whole cell: merged into the cell above.
  - `||` (two pipes, nothing between): the cell to the left extends into this
    column.
  - A span over three columns is `|||`, adjacent pipes with no space between
    them: `| 합계 ||| 215 |`. `|| ||` is two separate cells.
  - A merged area is a rectangle. Its text is written once, in its top-left
    cell; covered cells hold only a marker. Nothing is counted.
  - A 2×2 merge is the text + `||` in the first row and `^^ ||` in the row
    below: `| 합산 || 10 |` then `| ^^ || 20 |`.
  - An empty unmerged cell is `|  |`. `^^` in the header row and `||` at the
    start of a row are errors.
- Paragraphs without Markdown form (§6 round 3). `<p/>` has one meaning: it
  starts a paragraph; `<p style="Name"/>` starts one in style Name. Single
  tags: no `</p>`, no other attribute; no `<div>` in a cell or for an empty
  paragraph.
  - Empty paragraph: a line holding only `<p/>` or `<p style="Name"/>` is a
    paragraph with no text. Each is its own line, identical ones included:
    consecutive ones are consecutive lines, and canonical form keeps them so;
    a blank line separates the group from other blocks. A blank line or a
    `<br/>` line is not an empty paragraph.
  - Multi-paragraph cell: tables stay pipe tables, one line per row. The
    cell's text starts its first paragraph and each `<p/>` starts another:
    `| 부산 | 해운대 1곳<p/>서면 1곳 |`. A tag written first in a cell starts
    the first paragraph itself, so a cell that begins with
    `<p style="Name"/>` has its first paragraph in that style (canonical form
    drops a leading plain `<p/>`). `<br/>` is a line break inside one
    paragraph, not a new paragraph. A merged cell's paragraphs are all in its
    top-left cell; covered cells hold only `^^` or `||`.
  - So `<p/><p/>` in a cell gives an empty paragraph there: `a<p/><p/>b` is
    `a`, an empty paragraph, `b`; `a<p/>` ends with one. Not tested: round 3
    tested the two rules apart and never an empty paragraph inside a cell,
    and its kit's scorer still rejects one (`empty_cell_para`) and a `<p/>`
    line in `cellpara` units (`p_outside_cell`); the kit must be aligned
    before the next round.
- Table style (§6 round 2): a `{style="Name"}` line directly before the
  header row, no blank line between; the braces hold only `style="Name"`;
  nothing closes the table. A pipe table without the line has the default
  table style, the one a new table gets (docx: the file's default table
  style, or Table Grid when that draws no borders, as Word's Normal Table
  does); a line naming it is written as no line.
- **Formatting (proposed, §10 item 10, round 6 F2).** After the front matter,
  one line per style the text uses, the default first:
  `<style name="바탕글" align=justify line-spacing=160% font=함초롬바탕 size=10pt color=#000000/>`,
  then `<style name="개요 3" indent-left=20pt space-before=5pt size=15pt/>`.
  The default line is complete (a property it leaves out is 0pt, none or
  off); every other line holds what differs from the default style. Direct
  formatting is written where it applies and holds only what differs from
  the element's style:
  - a paragraph (heading, list item, `<div>`, plain line): ` {…}` at the end
    of its line, `- 항목 {style="개요 3" first-line=10pt}`; a list item names
    its style there, unless it is in the file's list style (docx: List
    Paragraph, the style Word gives a new item; elsewhere the default
    style). A property left out is the style's.
  - a run: `[15% 성장]{size=14pt color=#1F4E79}`.
  - a cell: `{…}` at the very start of the cell; each cell paragraph ends in
    its own `{…}`: `| {fill=#FFF0C3 border-bottom="2.83pt solid #7F7F7F"} **3D 프린팅 기술의 미래와 전망** {align=center size=20pt} |`.
  - a row: ` {…}` after its last `|`; the table: the `{style="Name" …}`
    line before the header row.
  - Canonical form lifts what is shared: a property every run of a paragraph
    has is on the paragraph; a value more than half of a row's cells (or of
    a table's cells and cell paragraphs) have is on the row (or table) line,
    and a cell writes only what differs. Adjacent equal runs merge. Empty
    paragraphs show no formatting.
  - Changing a style line changes every paragraph in that style that does not
    set the property itself; the returned text shows them unchanged.
  - A new style is a new style line with a name no style has, holding what
    differs from the default style; a paragraph takes it like any style
    (`<div style="Name">`, a list item's `style="Name"`, `<p style="Name"/>`
    in a cell). A name that is already a style is refused, with the line
    named. docx writes a `w:style` (`basedOn` the default paragraph style),
    hwpx a `hh:style` with its own `paraPr` and `charPr` in `header.xml`
    (decided 2026-09-30; round 6 part C: 4/4 per model after one fix round,
    duplicates refused 8/8).
  - No section defaults: a `<defaults …/>` line per page (F2s: the values
    most of a page's paragraphs share, between the default style and a named
    style's own values) was measured and not adopted: it cut hwpx text by 6%
    (2.16× to 2.02× today's), read as well as F2 (all 10 reads per model that
    resolve defaults, style and override), and landed one answer fewer
    (49/52 against 50/52 after one fix round, a look-alike space in `old`),
    so it did not meet the bar of correctness at least F2's (round 6 part C).
  - docx (built, 2026-09-30): the default style's line is docDefaults with
    the default paragraph style over it; another style's values follow
    `basedOn`, theme fonts and colours by name (`themeTint`/`themeShade` as
    `+N%`/`-N%`). A paragraph shows its `w:pPr` (`w:jc`, `w:ind`,
    `w:spacing`, `w:shd`, `w:pBdr`) and a run its `w:rPr` (`w:rFonts`,
    `w:sz`, `w:color`, the marks) as they differ from the paragraph's
    style; character styles stay in the remainder, unshown. A cell shows
    its `w:tcPr` `w:shd`/`w:tcBorders`/`w:vAlign` only: what the table
    style or `w:tblBorders` draws is the table style's, not the cell's. An
    empty paragraph keeps its stored `w:pPr` (the text cannot show it) and
    shows it once it has text. A value left as shown keeps its XML; a
    changed one rewrites only its child, in schema order, keeping unknown
    attributes and siblings; a value Word cannot hold (`/NN%` opacity,
    `line-spacing` as a gap) is refused. A changed style line rewrites that
    `w:style` so its values are the line's, and a new one is a
    `w:customStyle` `w:style` based on the default style, its id from the
    name's ASCII letters and digits. The line of a style the text does not
    use is not shown; changing it (a line with other values, compared in
    canonical form) is refused, so a style is used first and then changed.
    A tracked-changes export (§10.2) writes paragraph and run property
    changes as `w:pPrChange`/`w:rPrChange` and refuses a style line or cell
    box change. A remainder fingerprint leaves out what the text shows.
  - hwpx (built, 2026-09-30): the default style's line is style 0's
    (바탕글) `paraPr` and `charPr`; another style's is what its shapes set
    beyond it. A paragraph shows what its `paraPr` sets beyond its style's
    (`hh:align`, the margins and line spacing of the `HwpUnitChar` branch of
    `hp:switch`, in HWPUNIT; the default branch, and a shape without the
    switch, hold twice those lengths, and both branches are written; the
    fill and borders of its `hh:border` border fill); a run, its `charPr`'s
    height, `textColor` and Hangul font face (a written font sets each
    language that had the same face, and is added to the fonts of a
    language that lacks it); a cell, its `tc` border fill and its
    `subList vertAlign` (unset is centred). A list item shows its indent
    too, and names its style; a new item, or one the text moves to another
    level, takes its level's indent from an item there unless its text
    writes one (the returned text shows it). A strikeout `shape="3D"` is
    none, as Hancom writes it. A value left as shown keeps its shape id
    and XML; a changed one points the element at a copy of its own shape
    changed in that field only, reused when `header.xml` already has an
    equal one, and a paragraph whose shapes change drops its layout cache
    (`hp:linesegarray`, #27), whose key now covers each unit's `charPr`. A
    changed style line points that `hh:style` at such copies, and each of
    its paragraphs changes with it; a new style is an `hh:style` whose
    shapes copy the default style's. A border width snaps to Hancom's
    (0.1–5 mm) and a percent line spacing to whole percents; a new table
    whose text gives its cells no box takes the file's first cell's look
    (Hancom's, solid 0.12 mm, in a file without a table); both show in the
    returned text. hwpx has no theme colours, opacity or table styles:
    those are refused. The remainder fingerprint of a shape leaves out what
    the text shows, and a table's paragraph its layout cache.
  - *Why F2:* round 6 tied F1 (every element shows its effective
    formatting) on every direct task, and only F2 could change a style (6/6,
    one line against every paragraph); F2's style lookups landed 10/10, on an
    84k-char file too; F3 (styles only) could not do 21 of 39 tasks and
    misread hidden formatting as the style's. Part C repeated F2 on both hwpx
    seeds with the new-style guide: 25/26 per model after one fix round, the
    miss a look-alike character, not formatting.
- Headers, footers and section setup come from the template or the remainder
  at first; exposing their text is a later extension.
- Tracked changes and comments (`<ins>`, `<del>`, `<comment>`) are read-only
  projections at first.

### 5.3 Presentation

A slide is a canvas: its objects in z-order, each where it is. Every object shows
its box, slots included, so a slide reads on its own (§6 round 5, candidate A).

```
---
type: presentation
format: pptx
template: org/deck
schema: 1
size: 720 x 540 pt
---

layout: Title and Content
::title box="36 22 648 90" font=Calibri size=44pt color=tx1::
핵심 지표
::body box="36 126 648 356" font=Calibri size=32pt color=tx1::
- 매출 **12% 증가**
- 신규 고객 34곳
::notes::
전년 대비 강조

---

layout: Two Content
::title box="36 22 504 90"::
지역별 현황
::left box="36 126 318 356"::
- 수도권 21곳
::right box="366 126 318 356"::
- 지방 13곳
<picture id="s7" name="지도" box="560 20 124 80" src="media/image1.png" alt="지역별 지도"/>
<shape id="s4" name="출처" box="36 490 288 29">출처: 내부 집계<p/>2026년 9월</shape>
<line id="s6" name="화살표" from="330 504" to="366 504"/>
```

- Slidev-style. The first slide begins right after the file's front matter;
  later slides are separated by a line containing only `---`. `size` in the
  front matter is the slide's width and height in points.
- The first line of every slide is `layout: Name`, as listed, spaces included
  (quotes allowed). No other `key: value` lines.
- Real Slidev closes the per-slide front matter with a second `---`. Here that
  line starts an empty, layout-less slide and is an error.
- **Objects and z-order.** A slide is its objects written back to front, slots
  included: an object written later is drawn on top. Moving a line is the
  z-order edit. `::notes::` comes last and is not an object. Every object on
  the slide is shown, shapes without text and connectors included (rule 8).
- **Geometry.** `box="x y w h"` is the left edge, top edge, width and height in
  points from the slide's top-left corner (72 pt = 1 inch = 2.54 cm; x grows to
  the right, y downwards). Points are the unit, decided 2026-09-29; a cm view
  or cm input can be added later as a conversion on top of them. Numbers are
  shown as whole points; a written number may have decimals. `rot="15"` turns an object 15° clockwise about its centre,
  and its box is the unturned one, as `a:xfrm` stores it. `flip="h"`, `"v"` or
  `"hv"` mirrors it. Both are left out when there is no rotation or flip.
  A rotation is shown from 0 to 359; one a file stores negative or past a full
  turn (`rot="-5400000"`, as Google Slides writes) is shown within one turn
  (`rot="270"`) and, like any number left as shown, keeps its stored value.
  - A number left as it is shown keeps the exact stored value; a changed number
    is used as written (1 pt = 12,700 EMU). A box whose four numbers are
    unchanged leaves its `a:xfrm` untouched (GetPut, §4).
  - Same number, same position: a changed (or new) box's x, y, w or h that
    another box on the slide (a group's own, not its objects') shows for the
    same number, left as shown there, takes that box's exact EMU: the nearest
    in z-order, the one beneath on a tie. An edge aligned to a shown value
    lands exactly on it.
  - *Why points:* round 5 measured points, percent of the slide and a 12 × 12
    grid. Every request given in cm landed in points. Percent's one miss was a
    cm-to-percent conversion on a 16:9 slide, where x and y have different
    scales. The grid could not write 11 of 30 tasks, and it hid overlaps and
    misalignments smaller than a cell.
- **Slots.** Each slot starts with a marker line — `::title::`, `::body::`,
  `::left::`, `::right::` — and its text is the lines after it, up to the next
  object or `---`. Only the layout's slots appear; an unfilled slot is left out.
  - Every slot shows its box on its marker, `::title box="36 22 648 90"::`:
    its own when the slide stores one (`a:xfrm` in its `p:spPr`), else the one
    it inherits from the layout (or the master). The text does not say which;
    the remainder does. A slide therefore reads on its own, without the
    layout list.
  - A box left as shown keeps what the slide stores: an inherited box writes
    nothing back (the placeholder stays without `a:xfrm`, so GetPut is
    byte-exact), and an own box keeps its exact EMU. A changed box gives the
    slot an `a:xfrm` of its own, the unchanged numbers keeping their exact
    inherited values. A marker without a box (`::title::`) removes the slot's
    own box: it sits where its layout puts it, and the text the write returns
    shows that box.
  - Changing a slide's layout keeps every slot where its box says: the text
    is where the slide is, so a slot whose box was inherited gets it as its
    own. To put a slot where the new layout puts it, write its marker without
    a box (or with the new layout's box).
  - Slot names are the layout's placeholder types: `title` (centred or not),
    `subtitle`, `body`, `picture`, `chart`, `table`, `diagram`, `media`,
    `clipart`, `date`, `footer`, `number`. Two body placeholders are `left` and
    `right`, by position; three or more are `body`, `body2`, `body3`. A second
    placeholder of one name is `picture2`. A layout name the file uses twice is
    written `Name (2)`.
  - A slot holds paragraphs, list items, `<p/>` and placeholders. Its list items
    form one list. No headings, styles, tables, page breaks or footnotes: those
    come from the layout. An empty marker is an error. Deleting a slot's text
    leaves the layout's empty placeholder; a placeholder holding only spaces
    counts as empty and is left out.
  - A placeholder filled with a picture, chart or table is its slot's text: the
    slot holds that `<keep/>` alone, and its box is on the marker.
  - *Why every slot shows its box (A over B):* round 5 tied A (every slot's
    box shown) and B (a slot's box shown only when the slide moved it) at the
    ceiling, with B 11% smaller in answers and 10% in input. Round 5's decks
    were small. In a large deck, an agent under B must look each slot up in
    the layout list to know where it is, and can misjudge how the slide looks;
    A makes each slide self-contained, and the owner chose it for that
    (2026-09-29) at the cost of the 11%.
- **Shapes.** A shape that is not a placeholder is one line,
  `<shape id="s4" name="출처" box="…">text</shape>`, with its text editable;
  `<p/>` starts its next paragraph. Its paragraphs are never list items
  (bullets stay in the remainder). A shape without text is
  `<shape id="s9" name="Oval 8" box="…" kind="ellipse"/>`. Its preset
  shape, fill and outline are shown (below). *Proposed (§10 item 10, round 6 F1):* its
  effective formatting follows `box` on the tag, whether the shape sets it
  or takes it from the theme's `p:style` or the layout, like `box` itself:
  `<shape id="s8" name="Card" box="354 130 260 320" fill=accent1 border="2pt solid accent1-50%" size=18pt color=#FFFFFF>…</shape>`;
  a slot's on its marker (`::title box="…" size=44pt color=tx1::`), a line's
  outline as `border` on `<line/>`. A paragraph's own properties end its
  paragraph (`{…}` before `<p/>` or `</shape>`, or at the end of a slot
  line); a run's are `[text]{…}`. A property left out is not there (no
  fill, no outline). A changed value writes `spPr`/`a:ln`/`a:rPr` on the
  object and keeps its `p:style`. *Why:* round 6 measured showing only what
  the object sets (the pptx canvas audit's proposal): a quarter of the tasks
  (a read of an inherited value, an edit that must keep one) could not be
  done, and one was answered wrong; F1 did all 48, at 1.27× today's text
  against 1.14× on the corpus.
- **Text formatting** (built: `font`, `size`, `color`; the paragraph
  properties are still proposed). Every run of a slot or shape
  shows the font, size and colour it has, from the run's `a:rPr` over what
  it inherits: the shape's list style and `p:style` font, its layout and
  master placeholder, the master's title, body or other text style, the
  presentation's default text style, and last the theme's minor font, 18 pt
  and `tx1`. What all of an object's shown text shares is on its tag or
  marker, what a paragraph's shares beyond that ends the paragraph
  (`{size=32pt}`), and the rest is `[text]{…}` (lifting, as §5.2). Spaces and
  line breaks carry no formatting of their own in the text: they take the
  text around them. `font` is the East Asian font where the run has one,
  else the Latin one; writing it sets `a:latin` (and `a:ea` where the run
  shows an East Asian font). Colours are §5.1's: a scheme colour's
  `lumMod`/`lumOff` in whole percents is `accent1+40%`/`accent1-25%`, other
  adjustments `accent1*`, `a:alpha` `/NN%`; a gradient, pattern or picture
  fill is shown and kept. Notes show no formatting. A group's shapes show
  theirs as a slide shape does (see Groups).
  - A value left as shown keeps the run's XML. A changed one is written on
    the run's `a:rPr` in schema order, other children kept; a value equal to
    what the run inherits removes the run's own, so writing a title back at
    its layout's 44 pt leaves the file as it was. A property left out takes
    what the run inherits, and the write shows it again.
  - Changing a slide's layout keeps the formatting the text shows, as it
    keeps boxes: a run whose size or colour came from the old layout gets it
    as its own (a kept colour copied as the old layout stores it). To take
    the new layout's, leave the property out.
  - A run's fingerprint leaves out what the text shows of it (size, fill,
    fonts), so a restyled run is found again. Refused: a kept colour
    (`accent1*`, `gradient`) written where the run does not show it, an
    unknown key (`colour`), a value outside the grammar.
- **Fill** (built). A slot or shape shows the fill it has as `fill=` after
  its box (`<shape id="s9" name="Oval 8" box="…" kind="ellipse" fill=accent1+40%/>`,
  `::title box="…" fill=accent2::`): its `p:spPr`'s own, else its
  `p:style` `a:fillRef` into the theme's fill styles (the style's `phClr`
  the reference's colour, its adjustments kept), else, for a placeholder,
  its layout's and master's placeholder's. No fill shows nothing, and a fill
  left out is none; a gradient, pattern or picture fill is shown
  (`fill=gradient`) and kept while left as shown, as is a colour the style
  adjusts other than by Office's tints (`fill=accent1*`).
  - A fill left as shown keeps the XML. A changed one is set in `p:spPr`
    after the geometry and before the outline, other children kept; one equal
    to what the style or layout gives removes the shape's own; leaving a
    style's fill out writes `a:noFill`. A bare marker (`::title::`) takes
    its layout's fill as it takes its box. A layout change keeps the fill the
    text shows (a kept one copied from the old layout).
  - The shape's fingerprint leaves its fill out, as its geometry. Refused:
    writing a kept fill anew, `background=` and other spellings (the key is
    `fill`).
- **Outline** (built). A slot, shape or line shows its outline as
  `border="<width>pt <style> <colour>"` after its fill, and a line its
  arrowheads as `start=` and `end=` (`triangle`, `stealth`, `diamond`,
  `oval`, `arrow`) at its `from` and `to` ends:
  `<line id="s8" name="…" from="…" to="…" border="0.75pt solid accent1*" end=arrow/>`.
  The outline is the object's own `a:ln` over its `p:style` `a:lnRef` into
  the theme's line styles (the style's `phClr` the reference's colour), over,
  for a placeholder, its layout's and master's placeholder's; a width nothing
  sets is PowerPoint's 0.75 pt. Dashes show as §5.1's styles (`dashed`,
  `dotted`, `dash-dot`, `dash-dot-dot`; `double` for a double line); a
  gradient or pattern line shows no border and is kept while left so, and
  `triple`, `thin-thick` and `thick-thin` are shown and kept.
  - A value left as shown keeps the XML. A changed one writes only the
    components that changed, in `a:ln`'s schema order, other children kept:
    a new width is `w`, a new colour the line's fill, a new style
    `a:prstDash` (or `cmpd`), an arrowhead `a:headEnd`/`a:tailEnd`'s
    `type`. An outline equal to what the style or layout gives removes the
    object's own `a:ln`; leaving a style's outline out writes `a:noFill`. A
    new line is drawn `1pt solid tx1` unless its text says otherwise, and a
    bare marker takes its layout's outline.
  - The object's fingerprint leaves its outline out. Refused: a kept colour
    or style written anew, `outline=`, `stroke=` and other spellings (the key
    is `border`), `fill=` on a line (its colour is its border's), and
    arrowheads on a shape or slot.
- **Preset shape** (built). A slot, shape or line shows its DrawingML
  preset as `kind="…"` right after its box (`kind="roundRect"`,
  `kind="chevron"`, a connector's `kind="bentConnector3"`), and the
  preset's adjustments, when the file sets them, as `adj="…"`: one number
  for a preset whose one guide is `adj` (`adj="16667"`), else its guides by
  name (`adj="adj1=50000 adj2=50000"`), in the file's units (thousandths of
  a percent of the shape's size for most, as PowerPoint stores them). The
  preset is the object's own `a:prstGeom`, else, for a placeholder, its
  layout's and master's placeholder's. A rectangle, and a straight line
  (`line`, `straightConnector1`), show no kind; adjustments left out are
  the preset's own. Custom geometry (`a:custGeom`) shows as
  `kind="custom"`: its paths are kept as the file has them, and a preset
  written in its place replaces them. Guides that are formulas rather than
  values are kept and not shown.
  - A kind and adjustments left as shown keep the XML. A changed kind
    writes `a:prstGeom`'s `prst` and the adjustments the text gives (none:
    the new preset's own); changed adjustments rewrite `a:avLst`. A kind
    left out writes `rect`, or `straightConnector1` for a connector (a
    `line` stays one). One equal to what the layout gives removes a
    placeholder's own. A new text box or line may take a kind, and a bare
    marker takes its layout's.
  - The object's fingerprint leaves its preset out. Refused: a kind that is
    not a DrawingML preset (with the nearest names), adjustments that are
    not numbers or `name=number` pairs, `kind="custom"` on a shape that is
    not custom (it cannot be written anew), `adj` on custom geometry, and
    `shape=`, `preset=` and other spellings (the key is `kind`).
- **Effects** (built). A slot, shape or line shows what it draws beyond its
  fill and outline as a summary, `effects="…"`, last on its tag: the names
  of its effects in the file's order, from `shadow` (an outer or preset
  shadow), `inner-shadow`, `glow`, `soft-edges`, `reflection`, `blur` and
  `fill-overlay`, or `custom` for an effect graph (`a:effectDag`):
  `<shape id="s3" name="Card" box="…" fill=#FFFFFF effects="shadow"/>`. They
  are the object's own `a:effectLst`, else its `p:style` `a:effectRef` into
  the theme's effect styles, else, for a placeholder, its layout's and
  master's placeholder's. No effects show nothing. The summary does not
  show an effect's settings (a shadow's blur, distance or colour), and 3-D
  (`a:scene3d`, `a:sp3d`) is not an effect here.
  - Effects left as shown keep the XML. Leaving them out writes none: the
    object's own `a:effectLst` goes, or an empty one is written over a
    style's or layout's. `effects="shadow"` on an object without a shadow
    writes PowerPoint's preset outer shadow (Offset: Bottom Right, black at
    40%). Effects equal to what the style or layout gives remove the
    object's own.
  - The object's fingerprint leaves its effects out. Refused: any other
    change (adding a glow, a shadow to a list with other effects), a name
    outside the list, and `shadow=`, `effect=` and other spellings (the key
    is `effects`).
- **Pictures** are a line each, `<picture id="s7" name="지도" box="…"
  src="media/image1.png" crop="10 0 5 0" mask="ellipse" alt="…"/>`, in this
  attribute order; `crop`, `mask` and `alt` are left out when the picture has
  none (§6 round 6: an unwritten value is not there).
  - `src` is the image: a part of the package, named from the presentation's
    folder (`ppt/media/image1.png` is `media/image1.png`). Another picture's
    `src` shows the same image: the slide gets a relationship to it (or reuses
    the one it has), and an image nothing names any more goes with its
    relationship.
  - `crop="l t r b"` is `a:srcRect`: the percent of the image cut off at its
    left, top, right and bottom edges, shown with at most one decimal (a
    negative value leaves space beside the image). Like a box, a value left as
    shown keeps the stored thousandths of a percent exactly; a changed one is
    written as given, an unchanged `crop` leaves `a:srcRect` untouched, and no
    `crop` removes it. Left and right together, and top and bottom together,
    stay under 100.
  - `mask` is the preset shape the picture is cut to (its `a:prstGeom`), one
    of DrawingML's preset names (`ellipse`, `roundRect`, `hexagon`, …); no
    `mask` is a rectangle. A new mask takes its preset's own adjustments.
  - `alt` is the alternative text (`p:cNvPr descr`).
  - Each of them, changed, is written into its own XML only (`r:embed`,
    `a:srcRect`, `a:prstGeom`, `descr`), everything else in the picture kept;
    left as shown, the picture's XML is not touched (GetPut). The picture's
    id and name never change; its outline, effects and fill stay in the
    remainder.
  - A picture the text cannot hold whole stays a `<keep kind="picture"/>`
    line: one with artistic effects or a duotone, a video or sound shown as
    a picture, a linked image, one cut to a custom shape, one in
    `mc:AlternateContent`, and a picture placeholder's picture (its slot's
    text).
- **Charts, tables** and other objects the format does not model are
  a `<keep id kind summary box/>` line each (§5.1). They may be moved, resized,
  reordered or deleted, and their `id`, `kind` and `summary` never change;
  the id is made from the object without its geometry, so a moved or resized
  object keeps it. Resizing a table scales its column widths and row heights;
  a picture, moved or resized, keeps its crop. An object stored in more than one form
  (`mc:AlternateContent`) is shown with its box and not moved here. Deleting one removes its parts; refused while an animation plays
  on it.
- **Lines and connectors** are `<line id="s6" name="…" from="x y" to="x y"/>`:
  the two ends, not a box. Writing them rewrites `a:off`, `a:ext` and the flips.
  Its outline, arrowheads and kind (a straight, bent or curved connector)
  are shown (Outline and Preset shape, above), and its ends' attachments
  as below.
  - *Attached connectors* (built). A connector's end attached to an object
    the text shows by id (its `stCxn` or `endCxn` names a shape, picture or
    group, top-level or in a group) shows as that object's id and
    connection site instead of a point: `to="s2.1"`. An end attached to a
    slot or a `<keep/>` object shows its point, as before. A turned line
    shows its ends where they are on the slide, through its rotation.
    - When the object moves or is resized, or the text attaches the end to
      another site or object, the end is put on the site again: the site
      from the object's preset (`rect`, `roundRect`, `diamond` and the
      flowchart process and decision shapes: the middles of the sides, top,
      left, bottom, right; `ellipse`: eight round it from the top) through
      its flips, rotation and group, and the connector's box from its ends,
      its rotation kept. A straight connector lands exactly; an elbow
      (`bentConnector2`, `bentConnector3`) is routed by its ends, its bend
      back at the midpoint. An end neither moved nor re-attached keeps its
      stored point, so an unchanged connector stays byte-exact.
    - An end written as a point comes loose (its `stCxn`/`endCxn` goes). A
      new line may be drawn attached, `<line from="s4.2" to="s2.1"/>`.
    - The connector's fingerprint leaves its attachments out. Refused, with
      the connector and the reason named: rerouting a curved or other
      connector, an object whose sites are not known (another preset,
      custom geometry, a group), a site it does not have, an object not on
      the slide, one in a turned or flipped group, and moving an object a
      connector inside a group or a `<keep/>` is attached to, unless that
      connector's end is written in the same edit.
- **Groups** are a `<group id name box>` line, the group's objects, then
  `</group>`. Its objects show slide coordinates (through the group's child
  offset and extent), and the group's box is the box around them. Changing the
  group's box moves or scales the whole group, and canonical form writes its
  objects' new boxes. Changing its objects moves the group's box with them. A
  group box that disagrees with changed objects is refused. An object in a
  group is written as at the slide's level, a picture as `<picture/>`, another
  object as `<keep id="s12" kind summary box/>` with its shape id. A group's
  shapes show their text, text formatting and fill as a slide's shapes do,
  inheriting as shapes that are not placeholders, and all three may change:
  an unchanged paragraph keeps its XML; a changed one keeps its `a:pPr` and
  `a:endParaRPr`, and each character takes the run of the old character it
  aligns with (a character-level diff), marks and formatting set where the
  text changes them; paragraphs added follow the last one. A paragraph
  holding what the text does not show (a field) is refused. The group's
  fingerprint leaves its shapes' paragraphs and fills out. Otherwise only
  the boxes of a group's objects change here: their names and number stay
  (refused with the reason). A rotated or flipped group is one
  `<keep kind="group" … box/>`. Groups are never created or ungrouped here.
- **New objects** are written without `id`; the export gives ids and names
  (`TextBox 4`, `Straight Connector 10`), and the file read back shows them.
  They are:
  - a text box, `<shape box="…">text</shape>`;
  - a picture, `<picture box="…" src="…"/>` (with `crop`, `mask` and `alt` if
    wanted), showing an image of the package or a file the host hands the
    write by that name (the engine's `export_with_files`; the CLI and MCP
    server do not hand files over yet, so there such a write is refused with
    the reason). A host's file must be a PNG, JPEG, GIF or BMP image; it
    becomes a new image part (`media/image5.png`), which the text read back
    shows as the picture's `src`. A new picture in a picture slot is not
    built yet;
  - a line, `<line from="…" to="…"/>`.

  Nothing else is created: a new `<keep/>` of another kind, a new group or an
  invented id is an error.
- **New slides and decks from a template** are written with `layout:` and bare
  slot markers, with no geometry: every slot sits where its layout puts it,
  and the text the write returns shows each slot's box. The old, geometry-free
  form still reads (a shape written without its box keeps its box); its bare
  markers put an existing slot back in its layout's place.

### 5.4 Spreadsheet

A grid does not fit a text view, so the Spreadsheet splits in two:

- **Workbook structure as text** — sheets, tables, column types and formats,
  formulas, charts, placeholders:

  ```
  ---
  type: spreadsheet
  format: xlsx
  schema: 1
  ---

  <sheet name="매출" range="A1:G1201">

  <table name="Sales" range="A1:D1201">
  | column | type | format | formula |
  |---|---|---|---|
  | 월 | date | yyyy-mm |  |
  | 매출 | number | #,##0 |  |
  | 원가 | number | #,##0 |  |
  | 이익 | number | #,##0 | =[@매출]-[@원가] |
  </table>

  <chart type="bar" data="Sales[월],Sales[이익]" title="월별 이익"/>

  <keep id="k1" kind="data-validation" summary="B2:B1201"/>
  </sheet>
  ```

  *Proposed (§10 item 10):* formatted cells are `<format range="E4:H4"
  fill=bg2-10% border-top="2.25pt solid bg1" bold/>` lines in the sheet
  block, one per rectangle of cells with the same effective formatting that
  is not Normal's, `style="Name"` on ranges in a named cell style; they are
  written by a `format` range operation
  (`{"op": "format", "range": "매출!A1:D1", "set": {"fill": "#D9D9D9", "bold": true}}`),
  which sets the properties written on every cell of the range and keeps
  the rest; `border` sets all four sides of every cell and `outline` the
  outer edges of the range. A `<format default font=Calibri size=11pt color=tx1/>`
  line at the top is Normal's, so a cell in no range line reads on its own.
  Excel border styles show as widths (hair 0.25pt, thin 0.75pt, medium
  1.5pt, thick 2.25pt, double 2.25pt) and a written width snaps to the
  nearest one; theme colours stay names (`accent2+80%`). Round 6 part D, on
  three corpus workbooks: 25/25 per model on the first try, every op valid.

  Built (2026-09-30), and settled there:
  - The default line comes after the front matter, before the first sheet:
    the Normal style's (`cellStyles` `builtinId="0"`, its `cellStyleXfs`
    entry) font, size and colour, and any other key where Normal is not
    none, off, general, bottom or indent 0. A sheet's lines come right
    after its `<sheet>` line, ordered by top-left cell; each is a run of
    equal cells along a row, stacked over consecutive rows with the same
    columns. A line shows a cell's own style (`s`) only: column and row
    styles of cells the sheet does not hold are not shown.
  - Values: the theme index as Excel numbers it (0 `bg1`, 1 `tx1`, 2 `bg2`,
    3 `tx2`, 4–9 the accents, 10 `hlink`, 11 `folHlink`); a `tint` within
    0.05% of a whole percent is `+N%`/`-N%`, any other `*`; an `rgb` colour
    and the legacy `indexed` palette as `#RRGGBB`; a font colour left
    automatic and a solid fill without a colour as `#000000`; a font without
    a name shows no `font`. `centerContinuous` shows as `center`;
    `slantDashDot` as `1.5pt dash-dot`.
  - The lines are read-only: a text whose lines differ from the workbook's
    is refused with the operation to use. The `format` operation
    changes a cell's style to a copy with the written keys changed (a new
    font, fill or border only when no existing one is equal; a new `xf`
    only when none is), so number formats, wrap, rotation, protection and
    diagonal borders stay; setting `font` drops the font's theme `scheme`,
    which would win over the name; an indent on general, centred or
    justified text makes it left-aligned, as Excel does. Cells in the
    range that the sheet does not hold are made; at most 100,000 cells per
    operation. Refused: a gradient, pattern or picture fill, `/NN%`
    opacity, a `*` colour, border styles other than the six, `style`
    (shown, not written yet), and a key of another kind with a hint
    (`first-line` → `indent`).

  `range` on `<sheet>` is the used range, tables included. A column's type
  is `text`, `number`, `date` or `mixed` (cells of more than one kind, or
  empty cells in General format). Merges, validations, conditional formats,
  notes, pictures and pivot tables are `<keep/>` placeholders (rule 8). A
  syntax for validations is still open; until then they are
  placeholders, and a value that breaks one is refused.

- **Cell data** is read through a row window of one table or range: a pipe
  table whose first column is the read-only sheet row number, then one cell
  per column with the value as displayed (`12,000,000`, `00417`), blank rows
  included. A large sheet is read one window at a time (rule 11); the whole
  sheet is never assumed to fit. `rows` picks the sheet rows of a table's
  window; cells outside tables are read by range, one column per letter:

  ```
  <data table="Sales" rows="2:3">
  | row | 월 | 매출 | 원가 | 이익 |
  |---|---|---|---|---|
  | 2 | 2026-01 | 12,000,000 | 8,400,000 | 3,600,000 |
  | 3 | 2026-02 | 11,200,000 | 7,900,000 | 3,300,000 |
  </data>
  <data sheet="매출" range="F1:G2">
  | row | F | G |
  |---|---|---|
  | 1 | 목표 | 150,000,000 |
  | 2 |  |  |
  </data>
  ```

- Cell data is written by **range operations**: a JSON list of ops from a
  closed set, applied in order — `set`, `append_rows`, `insert_rows`,
  `delete_rows`, `fill_formula`, `set_type`, `add_column`, `sort`,
  `add_table`, `add_sheet`. The model never types bulk rows. A value that
  starts with `=` is stored as text, with a notice (§8); formulas are written
  only as a column's `formula` or by `fill_formula`.

  ```json
  [{"op": "set", "range": "매출!B73", "values": [[18420000]]},
   {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-08", "매출": 12400000}]}]
  ```

- The compressed view (SpreadsheetLLM-style anchors and a value index) is not
  the default: it is retested on large sheets as a later option for overview
  reads (§6 round 4).
- Formulas use Excel structured references (`Table[Column]`, `[@Column]`) —
  the formula dialect models already know.
- Formula results are computed by an engine before export (§7), so viewers
  that do not recalculate do not show 0.

## 6. Fluency — designed for, then measured

What a model (self-report, to be verified) handles well: CommonMark/GFM;
a few HTML-like tags; small JSON. Where it fails:

- **Counting and arithmetic** — `rowspan`/`colspan` grids, pipe-table column
  counts, list renumbering, unique IDs across a long file.
- **Syntax against habit** — e.g. Djot's `*strong*` (models write `**bold**`);
  Typst function names that changed across versions.
- **Prose inside JSON strings** (`\n`, `\"`).
- **Coordinates** (slide EMUs) — hopeless. Points are not: round 5 below
  measured boxes in points, percent and grid cells.
- **Undocumented attributes** — models invent plausible ones.
- **Exact-match editing breaks** on padded tables, repeated text, and
  auto-renumbered lists.

**Fluency test** (decides every "candidate A/B" in §5):

- Corpus: real files people make — Korean reports, minutes, proposals, weekly
  updates, government forms (hwp/hwpx), sales decks, KPI workbooks.
- Tasks: write from a brief; and edit — change one figure in a table, add a
  column, merge two cells, move a section, restyle a block, fill a form field,
  add a slide from a layout.
- Models: at least Claude, GPT (with `apply_patch`), Gemini.
- Metrics: first-try validity; edit lands on the intended span; fix rate after
  one validator error; tokens.
- Open choices it decides: merged cells as HTML spans vs local markers
  (`^^` / `||`); the style attribute name (`style` invites CSS such as
  `color:red`, `class` splits "Heading 1" on the space); Presentation syntax A
  vs B.

**Round 1 results (2026-09-28)** — kit and answers in [fluency/](fluency/),
details in [fluency/RESULTS.md](fluency/RESULTS.md). Claude Opus and Sonnet,
18 blind units each (3 choices × A/B × 3 Korean seeds × 4 tasks), no tools.

| Choice | Opus chars A / B | Sonnet chars A / B | Decided |
|---|---|---|---|
| Merged cells | 3,435 / 2,893 | 6,689 / 3,383 | B: `^^` / `\|\|` markers |
| Style attribute | 2,527 / 2,527 | 2,578 / 2,551 | A: `style` (tie, kept) |
| Presentation | 2,410 / 1,887 | 2,837 / 2,256 | B: Slidev-style |

- Both models: 72/72 valid and landed on the first try under every
  candidate; the fix round was never needed. **Correctness tied; the choices
  rest on size and design fit** (nothing counted, no closing tags), not on
  errors avoided.
- "Make it red, bold and large": both models chose the named Alert Box style
  under both attributes; no CSS was written.
- The tasks hit the ceiling: briefs spelled out the merges and the style list
  offered a matching style. Next round: larger files, merges implied not
  spelled out, no matching style to escape to, weaker models (Haiku), GPT and
  Gemini (not run: no API keys).

**Round 2 results (2026-09-28)** — kit in [fluency/round2/](fluency/round2/),
details in [fluency/round2/RESULTS.md](fluency/round2/RESULTS.md). Same two
models, 24 blind units each (4 choices × A/B × 3 Korean seeds × 5 tasks), no
tools, one fix round. What changed: seeds of 3,015–4,983 chars (round 1:
662–1,305) with repeated text; merges described by meaning; structural edits
(rows inside merged groups, splits, 2×2 merges, split/join/move styled tables
and slides); 6 styling requests with no matching style (right answer:
refuse); a fourth choice, table style (§10.7): A `<table style>` … `</table>`
wrapper vs B `{style="Name"}` line.

| Choice | Opus landed A / B | Sonnet landed A / B | Opus chars A / B | Sonnet chars A / B | Verdict |
|---|---|---|---|---|---|
| Merged cells | 15 / 15 | 15 / 13 | 5,147 / 3,827 | 8,453 / 4,438 | B holds |
| Style attribute | 15 / 15 | 15 / 15 | 2,930 / 2,887 | 3,260 / 3,382 | A holds (tie) |
| Table style | 14 / 15 | 14 / 14 | 4,060 / 3,641 | 7,437 / 8,511 | B, narrowly |
| Presentation | 15 / 15 | 15 / 15 | 4,710 / 3,909 | 7,612 / 5,308 | B holds |

- Landed is of 15 per cell, first try. Opus 120/120 valid, 119 landed;
  Sonnet 117/120 valid, 116 landed. The 3 invalid answers (Sonnet: merge B,
  table style A, table style B) were valid and landed after one validator
  error. All 24 refusal answers refused; no over-refusal, no CSS, no invented
  style name.
- Failures: Sonnet wrote a span over three columns as spaced `|| ||` (each an
  extra cell; 3 misses, all caught), and a 2×2 merge as two horizontal merges
  (text + `||` in both rows; valid, so silent); §5.2 now shows both cases.
  Opus once read "the ordinary body paragraph" as a plain line, not the listed
  `본문` style.
- Round 1's picks hold. Table style: correctness tied under A and B; B wins on
  size only — −10% (Opus), −24% (Sonnet without one outlier edit that makes
  its total +14%); unstyling a table is one line in B, two in A.

**Round 3 results (2026-09-28)** — kit in [fluency/round3/](fluency/round3/),
details in [fluency/round3/RESULTS.md](fluency/round3/RESULTS.md). Same two
models, 12 blind units each (2 decisions × A/B × 3 Korean seeds × 5 tasks), no
tools, one fix round. It decides §10.8 on seeds of 3,023–3,408 chars with
11–15 multi-paragraph cells or 25–33 empty paragraphs, identical runs of
empty paragraphs where an edit must hit one, structural cell edits (split,
join, move, restyle, merge cell paragraphs), exactly scored writes and one
refusal per unit. Multi-paragraph cells: A `<p/>` / `<p style="Name"/>` inside
pipe cells vs B a `{list-table}` with indented cell lines. Empty paragraphs:
A a `<p/>` / `<p style="Name"/>` line vs B a `<div></div>` /
`<div style="Name"></div>` line.

| Decision | Opus valid A / B | Sonnet valid A / B | Opus chars A / B | Sonnet chars A / B | Verdict |
|---|---|---|---|---|---|
| Multi-paragraph cells | 15 / 10 | 14 / 15 | 2,875 / 3,574 | 3,402 / 3,920 | A: `<p/>` in pipe cells |
| Empty paragraphs | 15 / 15 | 15 / 15 | 3,148 / 3,770 | 3,930 / 4,627 | A: `<p/>` lines (tie, size) |

- Valid is of 15 per cell, first try; every valid answer landed. Cells: A
  29/30, B 25/30; all 6 invalid answers were valid and landed after one
  validator error. All 24 refusal answers refused; no CSS, invented attribute
  or invented style.
- Failures: Opus quoted list-table lines two spaces too deep in every B
  `old` of two units (`edit_no_match`, 5 misses; its `new` text was right),
  Sonnet once wrote a span to the last column as `|| |` (an extra cell), and
  nothing failed on the identity tasks (no ambiguous `old` or wrong count in
  runs of up to seven identical empty paragraphs).
- Cells: A, on B's indentation misses and size (B +15–24%; one new cell
  paragraph turns a pipe table into a list table, 4× the edit). Empty
  paragraphs: A, on size alone (B +18–21%). The two A rules were tested apart;
  §5.2 unifies them untested.

**Round 4 results (2026-09-28)** — kit in [fluency/round4/](fluency/round4/),
details in [fluency/round4/RESULTS.md](fluency/round4/RESULTS.md). Same two
models, 15 blind units each, no tools, one fix round. It decides §10.6 on three
generated Korean workbooks (150–190 data rows over 3 sheets) with
near-duplicate rows, leading-zero IDs in a number column, subtotal and blank
rows, and one refusal per write unit. Read view (3 workbooks × 5 questions):
A a plain window (pipe table, leading `row` column, values as displayed) vs
B a compressed index (SheetCompressor-style anchors, per column an inverted
index of values to rows). Write shape (3 workbooks × 6 tasks, cells read
through view A): A range operations vs B editable range text (`{old, new}`
edits on the window, row labels read-only) vs C Python against a small
workbook API in a sandbox.

| Decision | Opus landed A / B / C | Sonnet landed A / B / C | Opus chars A / B / C | Sonnet chars A / B / C | Verdict |
|---|---|---|---|---|---|
| Read view | 15 / 15 / – | 15 / 14 / – | 35,929 / 30,988 / – (input) | 35,929 / 30,988 / – (input) | A: plain window |
| Write shape | 18 / 18 / 18 | 18 / 15 / 16 | 3,897 / 3,869 / 3,417 | 3,995 / 4,114 / 3,770 | A: range operations |

- Landed is first try, of 15 per read cell and 18 per write cell. Opus
  84/84; Sonnet 78/84, and 83/84 after one fix round: its five invalid
  answers (write B 3, C 2) were fixed and landed. All 18 refusal answers
  refused with the right rule. Sonnet's first write-shape run is void — its
  nine prompts over 20,000 chars reached it as a file path, not text, so it
  refused every task — and was rerun from the prompt files, as Opus had them.
- Failures: in the compressed view Sonnet found the row and read its value
  from the neighbouring column's line (15,710,000 for 12,510,000; valid, so
  silent); in editable text it invented row labels on a new table
  (`row_labels_edited`, all three new-table tasks); in code it wrote two
  formulas without `=` (`bad_formula`, one conversation). Nothing failed on
  near-duplicate rows, leading-zero IDs or A1 formulas.
- Read view: A. 30/30 vs 29/30 is noise; A is chosen on the kind of error, as
  B's miss is a plausible wrong figure no validator catches. B's −14% input
  (−3% to −22% by workbook) is retested on large sheets. Write shape: A, the
  only shape with no first-try error on either model (36/36 vs B 33/36, C
  34/36; within noise). C is the runner-up and 6–12% smaller (Sonnet / Opus);
  B is no smaller than A (−1% / +3%, Opus / Sonnet).

**Round 5 results (2026-09-29)** — kit in [fluency/round5/](fluency/round5/),
candidates in [fluency/round5/CANDIDATES.md](fluency/round5/CANDIDATES.md),
details in [fluency/round5/RESULTS.md](fluency/round5/RESULTS.md). Same two
models, 21 blind units each, no tools, one fix round. It decides §10.9, geometry
in Presentations, on korean-deck.pptx and shapes.pptx (the office-kit decks,
with every object the files hold) and a 16:9 Korean deck written for the round.
The candidates are A, every object and slot with its `box` in points; Ap, A in
percent of the slide; B, slots from the layout (a box only when moved) and
every other object with its box in points; and C, A on a 12 × 12 grid of
cells. Part 1 (4 candidates × 3 decks × 10 tasks) asks for reads (overlap,
region), a move, a picture resize, a text box under the title, left alignment,
a bullet edit that must not touch geometry, a comparison slide, a new
three-slide deck and one refusal. Part 2 (A, Ap and B × 3 decks × 6 tasks) is
harder: slot geometry, z-order, centring, even spacing, a group move and a
rotation.

| Decision | Opus landed A / Ap / B / C | Sonnet landed A / Ap / B / C | Opus chars A / Ap / B | Sonnet chars A / Ap / B | Verdict |
|---|---|---|---|---|---|
| Format and unit | 48 / 47 / 48 / 17 | 48 / 48 / 48 / 17 | 6,325 / 6,261 / 5,650 | 5,868 / 5,886 / 5,236 | A and B tie; A chosen (§10.9), in points |

- Landed is first try, of 48 per A/Ap/B cell (both parts) and 30 per C cell
  (part 1). All 288 A/Ap/B answers were valid, and every A and B answer
  landed on both models. Ap's one miss (Opus) wrote 0.5 cm as 1.0 cm in
  percent of each axis on the 16:9 deck (valid, so silent). All 24 refusals
  refused; no box was written on any of the 48 new slides and decks; every cm
  request landed in points.
- C cannot write 11 of its 30 tasks (3 cm moves, a 0.7–1.1 cm text box, 1.5×
  scaling, aligning to an off-grid edge). Opus refused 9 of them and Sonnet 5,
  and the rest missed. C also misread two 7 pt overlaps that round to one grid
  line (4 of 4 answers wrong), and Sonnet called two misaligned pairs "already
  aligned". Sonnet's two invalid C answers were valid after one round and still
  missed.
- Format: A and B tie at the ceiling. On size and fit alone, as in round 1,
  B wins: it is 11% smaller in answers on both models and 10% smaller in
  input (21% on a deck of inherited placeholders), and a box on a slot says
  the slide stores its own, as the file does. A's self-contained read did not
  matter here: no inherited slot was misread in 14 tasks that needed the
  layout list. The owner chose A
  (§10.9): the decks were small, and in a large deck B's lookups in the layout
  list are where an agent would misjudge a slide. Unit: points (A and B
  192/192; percent 191/192, its miss a unit conversion). Grid: rejected.
- Limits: the ceiling (a harder or longer round could still separate A and B),
  one run per prompt, and geometry checked in the kit's model, not yet in
  hanji-pptx or PowerPoint.

**Round 6 results (2026-09-30)** — kit in [fluency/round6/](fluency/round6/),
candidates in [fluency/round6/CANDIDATES.md](fluency/round6/CANDIDATES.md),
details in [fluency/round6/RESULTS.md](fluency/round6/RESULTS.md). Same two
models, blind, each prompt read from its own file with Read as the only tool,
one fix round. It proposes §10 item 10, direct formatting, per kind. Part A,
flow documents (3 candidates × 3 seeds × 13 tasks: footnote-01.hwpx, the
1,717-paragraph mel-001.hwpx, korean-report.docx): F1, effective formatting on
every element; F2, a style section plus only what differs from the style; F3,
the style section only. Part B, Presentations (2 candidates × 3 decks × 8
tasks: the pptx canvas audit's two synthetic decks and the ONLYOFFICE sample):
F1, effective formatting inline; F2o, only what the object sets itself.

| Part | Opus landed | Sonnet landed | Opus chars | Sonnet chars | Proposed |
|---|---|---|---|---|---|
| A: F1 / F2 / F3 | 35/37 / 37/39 / 18/18 (+21 impossible, 2 misread) | 35/37 / 37/39 / 18/18 (+21, 2 misread) | 19,475 / 13,137 / 4,522 | 16,037 / 19,097 / 4,140 | F2 for docx and hwpx |
| B: F1 / F2o | 24/24 / 18/18 (+6 impossible) | 24/24 / 18/18 (+6, 1 misread) | 3,183 / 2,976 | 2,855 / 2,618 | F1 for pptx |

- Landed is after the fix round, of the tasks the candidate's text can do.
  F1 and F2 tie on direct tasks with the same misses: an `old` that missed
  look-alike characters on mel-001 (U+2007 figure spaces, fixed once the
  error named them; a private-use U+F076, not named, not fixed). Only F2 can
  change a style (6/6; one line of 162 chars against every paragraph,
  1,264–3,616); under F1 both models refused 4 of the 6 style edits,
  correctly. F2's reads through a style line landed 10/10, on the 84k-char
  file too.
- The errors that matter are reads of formatting the text does not hold:
  F3 answered a paragraph's first-line indent and a run's size from the
  style (4 answers, no validator can catch them); F2o once gave `fill=none`
  for a shape filled accent1 by its theme style.
- Size on the corpus, over today's text: hwpx F1 2.13×, F2 2.16×, F3 1.04×
  (Hancom keeps formatting on each paragraph and cell, so F2 cannot factor
  it); docx 1.40×, 1.38×, 1.12×; pptx F1 1.27×, F2o 1.14×; xlsx F1 range
  lines 2.52× with the default line (2.08× without one outlier, median
  1.17). Row and table lifting took fdi-2025q2.hwpx from 4.4× to 2.0×.
- Part C (after the owner's answers, 2026-09-30), F2 against F2s on the two
  hwpx seeds, 13 tasks each: reads that resolve a page's defaults line, the
  style and the paragraph's own `{…}`, a no-op ("set it to 170%" where it
  already is), a single override, a new style applied to two paragraphs, and
  a new style with a name already taken (a refusal). Landed after one fix
  round: F2 25/26 per model, F2s 25/26 (Opus) and 24/26 (Sonnet); first
  try 23 and 24 against 23 and 23. Every miss, in both, is an `old` that
  dropped a U+2007 or U+F076 on mel-001; every read (20/20 per candidate),
  no-op, new style and refusal landed. F2s is 6% smaller on the hwpx corpus
  (2.02× against 2.16×; mel-001 76,393 against 84,442 chars) and not adopted
  (the bar was correctness at least F2's). New styles: adopted.
- Part D, Spreadsheets: F1 range lines and the `format` operation on three
  corpus workbooks (simple-monthly-budget, korean-sales-lo, the 12-sheet
  Tables.xlsx at 74,769 chars), 8–9 tasks each: reads through the range
  lines (a theme colour as written, borders, which cells have an indent),
  fills and font colours by theme name, an outline, one op per sheet across
  twelve sheets, refusals. Both models 25/25 on the first try, 24 ops each,
  none invalid; theme colours were written as names.
- Limits: formatting read, written and checked by the kit, not the engines
  or Office; no large docx; one run per prompt; part D at the ceiling.

## 7. Engines (surveyed 2026-09-28)

Engines work only at import, export and preview. Every engine is behind this
project's own interface so it can be replaced.

| Format | Engine | Key facts | Risk |
|---|---|---|---|
| hwp / hwpx | **rhwp** (Rust + wasm, MIT) | Parses HWP 5.0 / HWPX / HML; own layout; SVG/PNG/PDF; web editor; hwpctl-compatible API; **refuses a lossy save**; `hwp_ir_diff` checks no-loss; 3,400+ tests; 3,868★, 734 forks, several maintainers; used by Korean users in earnest | Pre-1.0 (v0.8.6) |
| docx | **rdocx** (Rust, MIT/Apache) | Open/create/edit/save; keeps unknown safe XML byte-for-byte (claim); own Word layout; PDF/PNG/SVG/HTML/MD | 7 months old; 1,656 of 1,716 commits by one maintainer; fidelity measured against **LibreOffice** (SSIM ≥ 0.95 on ≥ 80% of pages) on **5** documents, not against Word |
| docx | `docx` (npm, MIT) | Most-used builder (6.7M/week, release 2026-09-27); East Asian font + `w:lang`, OMML, header-row repeat | Builder only: no template input (placeholder patcher only); open "unreadable content" issues |
| docx | SuperDoc (JS) | OOXML-native engine with markdown/HTML projection, placeholders, diagnostics, source maps, revision guards, tracked-change writes | AGPL-3.0 or commercial licence |
| pptx | **rpptx** (Rust, MIT/Apache) | Opens a deck's own layouts; `add_slide(layout)`, placeholders, tables, pictures, native charts with embedded workbook, notes, autofit; renders frames and PDF; 50-deck render corpus | 2 months old, 356 downloads, same single maintainer as rdocx |
| pptx | pptxgenjs, python-pptx | Widely used | pptxgenjs: no commits since 2025-06, 8 open repair-dialog issues, no template loading; python-pptx: last release 2024-08 |
| xlsx (write) | **rust_xlsxwriter** (Rust, MIT/Apache) | Port of Python XlsxWriter; all repair reports fixed; row streaming; `wasm` feature | Write-only; no formula engine (writes 0 + recalc flag) |
| xlsx (edit) | umya-spreadsheet / ooxmlsdk + **IronCalc** | umya reads+writes; ooxmlsdk is schema-typed and keeps untouched parts; IronCalc evaluates formulas | umya: 8 open repair issues incl. "charts corrupted after opening and saving" (#281); IronCalc drops charts and validation on export |
| any OOXML | ooxmlsdk (Rust) | Port of .NET Open XML SDK; typed parts and elements generated from the schema; `create_from_template` | No helpers; pre-1.0, breaking releases |

Each engine must build and pass the same tests on a native target and on
wasm32 (§10 item 4).

Preview (decided 2026-10-01 by
[prototype/preview/SPIKE.md](prototype/preview/SPIKE.md)): the in-binary
engines render it, one SVG per page: rdocx for docx, rpptx for pptx, rhwp for
hwp/hwpx. hanji subsets the fonts used on each page and embeds them; today
rdocx/rpptx embed whole fonts (47–81 MB per Korean page) and rhwp none. xlsx
previews as an HTML grid (later). LibreOffice is not a preview engine; it
served only as a reference (worst on hwpx through H2Orestart). Found there:
rpptx 0.12.1 refuses a deck over a schema-valid animation list (`duplicate
p:attrName`), and its public render has no CJK fonts; rdocx 0.14 reaches
Hangul through Noto CJK SC coverage fallback, not the requested East Asian
font.

Found by [prototype/tracked-changes/SPIKE.md](prototype/tracked-changes/SPIKE.md):
rhwp 0.8.6 drops hwpx track-change marks on save without a loss report, and a
plain rdocx 0.14 open/save of one corpus file wrote a `styles.xml` that is not
well-formed (a dropped `xmlns:w14` declaration).

Not engines here: **pandoc** (GPL-2.0-or-later; md → docx/pptx only; no pptx
charts; 6 open corruption issues), **Typst** (PDF/PNG/SVG; HTML behind a
feature flag; no docx), **DocLang** (a read-oriented AI document format,
v0.7.3; "lossless … regarding content", not formatting).

## 8. Safety

- **Neutralised on import, never preserved:** macros (docm/xlsm/pptm), remote
  templates, external-link fields, DDE, `INCLUDEPICTURE`/linked images,
  OLE/ActiveX. Opening an exported file must fetch nothing.
- **Surfaced before export:** hidden text, comments, tracked deletions, author
  and document metadata — content nobody reviewed would otherwise leave with
  the file.
- **Spreadsheet formula injection:** text written by a model that starts with
  `=` is text unless it is in a formula field; functions that fetch
  (`WEBSERVICE`, `HYPERLINK` to data-carrying URLs) are refused or flagged.
- **No guessed types:** IDs and phone numbers with leading zeros stay text;
  numbers and dates are typed by the source, not inferred by the writer.
- **Korean text** carries the East Asian font and `lang` attributes.
- **Determinism:** the same model + remainder + template gives the same
  bytes (fixed timestamps, stable part order), so a digest identifies an
  export.

## 9. Verification

| Claim | Test |
|---|---|
| Rule 1 (preserve) | GetPut on a real corpus per engine: import → export with no edit is XML-equivalent per part |
| PutGet | After each fluency-test edit, re-import shows exactly the written text |
| Validity | Every export opens in Word / PowerPoint / Excel / Hancom with no repair prompt; schema validation alone is not enough (a schema-valid file Word rejected: office_oxide #208). Checked by hand for the office-kit in all four, 2026-09-29/30 (§11). CI validates every corpus source and kit file against the ISO/IEC 29500 transitional schemas plus the rules outside them that Office enforces (`validation/ooxml-schema`): necessary, not sufficient, and it caught the synthetic pitch deck PowerPoint repaired (kit v8, 44–46) |
| Rule 5 (fidelity) | Layout of our preview against the native application's own PDF export — Word, PowerPoint, Excel, Hancom — not against LibreOffice: words aligned over the whole document, scored on text present (T), page (P), line starts (L) and position (W), combined as (T·P·L·W)^¼ per file; content-masked SSIM as the secondary check for what has no text. Raw per-page SSIM is not used: a blank page scores 0.868 and a correct layout in a substitute font 0.57 ([prototype/preview/SPIKE.md](prototype/preview/SPIKE.md)). xlsx (an HTML grid) is checked on shown values and formats instead |
| Presentation geometry | GetPut per object on the pptx corpus: every `a:xfrm` (and every absent one) unchanged after import → export; PutGet after box, z-order, group and new-object edits; the office-kit shows moved, resized and added objects where the text puts them, in PowerPoint |
| Rule 8 (the model sees it) | Per corpus deck, every object on a slide appears in the text: slots, shapes with or without text, lines, groups and their objects, `<keep/>` lines |
| Direct formatting (proposed, §10 item 10) | GetPut per element on every corpus file: a formatting value left as shown leaves its `pPr`/`rPr`/`tcPr`, `paraPrIDRef`/`charPrIDRef`/`borderFillIDRef`, `spPr`/`p:style` and cell `s` untouched (the kit holds this on the text for 28 of 29 flow files); PutGet after fill, border, colour, size, indent and style-line edits, the returned text canonical (lifting, snapped widths); a new style line adds one style (docx `w:style`, hwpx `hh:style` with its `paraPr`/`charPr`) and a taken name is refused; a `format` op changes only the properties it writes, on the cells of its range; each vocabulary value opens as written in Word, Hancom, PowerPoint and Excel (the office-kit) |
| Rule 2 (fluency) | §6 |

## 10. Open decisions

1. ~~Style attribute name, merged-cell syntax, Presentation syntax~~ —
   decided by §6 round 1: `style="Name"`, `^^`/`||` markers, Slidev-style
   (§5.2, §5.3, §6).
2. ~~Whether model edits reach the exported file as **tracked changes**
   (reviewable in Word/Hancom) or as direct changes~~ — decided for now,
   2026-09-28, by
   [prototype/tracked-changes/SPIKE.md](prototype/tracked-changes/SPIKE.md):
   direct changes only; tracked changes stay read-only (§5.2). Later, tracked
   changes become a docx-only export option, off by default: hanji-docx writes
   the revision XML itself from the exact edit spans, marking an inserted
   paragraph on the mark of the paragraph before it, as Word does. rdocx has
   no API to write revisions and its compare is unreliable (40 of 107 edits
   tracked), so it only checks accept/reject all in tests; rhwp does not model
   hwpx track changes and drops them on save without a loss report, so hwpx
   stays on direct changes. Gate for the option: docx GetPut passes, the file
   opens in Word with no repair prompt, and accept/reject give back the edited
   and previous text across the corpus. Built (#16); checked by hand in Word
   on 2026-09-29 for the office-kit (§9, kit v2): all 19 docx files, the
   tracked ones included, open with no repair prompt, and Accept All / Reject
   All give the edited and the original text.
3. ~~The remainder's storage shape and anchor granularity (block, run range,
   shape, cell range)~~ — decided by
   [prototype/remainder/REMAINDER.md](prototype/remainder/REMAINDER.md): a flat
   list of typed entries per revision, namespace map stored once per document;
   anchors are a placeholder id, a block (paragraph/table/row/cell), or a
   character range + seq in a paragraph. Re-anchor by the exact edit span, else
   keep unchanged aligned blocks and run one document-level character diff over
   the changed ones (design C): over 13 docx, 98.8% landed, 1.1% silently lost
   (block + offset: 7.8%; never used). Refused: an entry whose ends land in
   different paragraphs; an ambiguous alignment (identical or unrecognised
   blocks).
4. ~~Where engines run: server, client (wasm), or both~~ — decided
   2026-09-28: both. The core stays I/O-free so one build serves a server and
   a wasm client; every engine is chosen and tested for both targets.
   rhwp, rdocx, rpptx and rust_xlsxwriter all build to wasm; risk for the
   client target: rdocx-wasm is unpublished.
5. ~~Header/footer and section text: when to expose~~ — deferred
   2026-09-28: header, footer and section text stay in the template or
   remainder (§5.2) until a corpus shows edits need them.
6. ~~Spreadsheet cell-data operations: API shape and the compressed read
   view~~ — decided by §6 round 4: plain row-window read view and range
   operations (§5.4); compressed view retested on large sheets later.
7. ~~How a table style attaches to a pipe table~~ — decided by §6 round 2:
   `{style="Name"}` line before the pipe table (§5.2); a narrow, size-only
   win.
8. ~~A syntax for multi-paragraph table cells and empty paragraphs~~ —
   decided by §6 round 3: `<p/>` paragraph starts in pipe cells and `<p/>`
   lines for empty paragraphs (§5.2). An empty paragraph inside a cell
   (`<p/><p/>`) follows from the two, untested. The table shapes round 3 did
   not cover are decided 2026-09-28 by
   [prototype/tables-survey/SURVEY.md](prototype/tables-survey/SURVEY.md):
   across 261 real documents (1,335 tables), nested tables are 2.6%,
   `gridBefore`/`gridAfter` 1.0% and row-level content controls 0%, so they
   stay an uneditable block placeholder for now. Import fixes it found: a
   bookmark or marker before the first row goes to the remainder instead of
   forcing a placeholder (15.6% of docx tables); side-by-side hwpx tables
   import as separate blocks. Limit: the Korean side is press releases only,
   and many source hosts were blocked.
9. ~~Geometry in Presentations~~ — decided 2026-09-29 by §6 round 5 and the
   owner: candidate A, in points, with the attached-connector refusal. Real
   decks checked in PowerPoint showed a canvas without positions and sizes to
   be too limited, so the old §5.3 ("never geometry") is replaced. Every
   object shows its box in points, slots included (their inherited box when
   the slide stores none), groups show their objects, and lines show their
   ends. Text boxes, pictures from a file and lines can be added (§5.3). A
   tied B (a slot's box shown only when moved) at the ceiling of round 5, and
   B is 11% smaller; the owner chose A because round 5's decks were small,
   and in a large deck an agent under B must look each slot up in the layout
   list and may misjudge how a slide looks. A slide under A reads on its own.
   Chosen over percent of the slide (one unit-conversion miss) and a 12 × 12
   grid (11 of 30 tasks unwritable, silent misreads). Shapes keep B's one-line
   `<shape>` form (round 5's A wrote them as `::shape::` blocks; B's form
   landed as often, and is one line for a shape without text). It also
   fixes a rule 8 gap: the old text left out connectors and textless shapes (9 of the 19 objects in
   shapes.pptx). Across the 21 corpus decks, 36% of slide placeholders have
   their own box. A schema-1 text without boxes still reads unchanged.
   Deferred: rerouting attached connectors (an edit that moves an attached
   object without rewriting its connectors is refused, §5.3), editing inside
   rotated groups (kept whole), pictures from a file, and cm as a view or
   input over points (the unit Korean PowerPoint shows; not measured).

10. **Direct formatting (proposed, 2026-09-30; docx and hwpx built, §5.2; xlsx built, §5.4)** — by §6 round 6, per kind:
    flow documents F2 (a style section plus visible direct overrides,
    §5.2), Presentations F1 (effective formatting inline on every object,
    §5.3), Spreadsheets range lines and a `format` operation (§5.4); one
    vocabulary for all four (§5.1). Replaces "formatting by name only" when
    accepted. Decided with the owner (2026-09-30): agents may create named
    styles, and a name already in use is refused (part C: 4/4 per model);
    per-page defaults lines (F2s) were tried for the size of Hancom files
    and not adopted, since they did not match F2's correctness (49/52
    against 50/52) for 6% less text; theme colours stay names, and the
    xlsx `format` operation landed 25/25 per model (part D). Open: Hancom
    files stay at 2.2× today's text (cell borders, fonts and sizes set on
    every paragraph and cell, which no style factors); look-alike and
    private-use characters caused round 6's only invalid answers and part
    C's only misses: validator errors now name them by code point (§4,
    #31), as round 6's fix round did for U+2007 but not U+F076, and F2s
    could be retried against that. The kit's formatting reader is its own, not the
    engines'.

## 11. Blind spots

- Real apps: only the office-kit (§9) has been opened in Word, PowerPoint,
  Excel and Hancom Office, by hand on 2026-09-29 and 2026-09-30; every kit
  file now opens with no repair prompt and passes its checks. Found on the
  way:
  - docx: none; all 19 files passed in Word (kit v2).
  - pptx (18 files): 37 and 40 hit the kit's slide-delete span bug (#21);
    28 and 31 failed as expected; on v4, 40 missed an align by 0.45 pt (the
    snap rule, #25). All passed on v6.
  - xlsx: 61 recalculated a binary-search XLOOKUP unlike Excel and showed
    stale labels, 55 showed stale labels (both #26).
  - hwpx: 64 raised a repair prompt in Hancom (a stale `linesegarray`
    textpos after a first-paragraph insert, #27); 73 lost a list level
    (#28). v6 passed, and on 2026-09-30 so did 73 in v7 (with #28's
    moved-item indent fix).

  Beyond the kit, the engines' corpora and every other library above are
  unchecked in real apps; their defect and fidelity statements still come
  from project docs and issue trackers.
- The fluency list in §6 is still mostly a model's self-report: six rounds
  ran on two Claude models only, 12–48 tasks per cell, one run per prompt.
  Round 6's formatting was read and checked by its kit, not by the engines,
  on three flow seeds (two hwpx, one small docx), three decks and three
  workbooks; its xlsx trial (part D) hit the ceiling.
  Round 5 put three of its four candidates at the ceiling on small decks, so
  the §5.3 geometry rests on design (a slide that reads on its own), and its geometry was checked in the
  kit's model (`deck.py`), not in hanji-pptx or PowerPoint. No round asked
  for a connector edit, so how often the attached-connector refusal (§5.3)
  blocks a real move is unmeasured.
  Round 1 hit the ceiling; rounds 2–4 produced a few failures (Sonnet's merge
  markers in rounds 2 and 3; Opus's list-table indentation in round 3;
  Sonnet's new-table row labels and `=`-less formulas in round 4), every
  invalid one fixed in one round; two, round 2's 2×2 merge and round 4's
  compressed-view read, were valid but wrong. Round 4 used an in-memory
  workbook model (`wb.py`), not a real xlsx engine, on workbooks of a few
  hundred rows (150–190 data rows), not 100k. GPT, Gemini and Haiku are still
  untested.
- The 90% fidelity target is now the layout metric of §9, but its threshold
  is **proposed**, not adopted
  ([prototype/preview/SPIKE.md](prototype/preview/SPIKE.md)): per format, a
  mean file layout score ≥ 0.90, ≥ 80% of files ≥ 0.85, the page count exact
  on ≥ 90% of files, and ≥ 90% of pages ≥ 0.80 (a page without text by
  content-SSIM ≥ 0.80). Calibration: a 1 px shift scores 0.987, 5 pt 0.898,
  a blank page 0. On 30 baseline files (2026-10-01) no engine meets it: rhwp
  0.836, rpptx 0.791 (it refuses one deck), rdocx 0.619. Five of the nine
  Word PDFs were printed in markup view and must be re-exported. Fonts are
  unchecked: no Hancom or Microsoft font was available, and a font-identity
  sub-score, visible substitutions and a font policy are proposed there too.
- rdocx/rpptx capability statements are the project's own claims.

## 12. References

- Syntax: [CommonMark](https://spec.commonmark.org), [GFM](https://github.github.com/gfm/),
  [Pandoc Markdown](https://pandoc.org/MANUAL.html#pandocs-markdown),
  [Djot](https://github.com/jgm/djot), [Markdoc](https://github.com/markdoc/markdoc),
  [MDX](https://mdxjs.com), [MyST](https://github.com/jupyter-book/mystmd)
  (one tree → docx/Typst/TeX/JATS; its docx exporter uses the `docx` npm package).
- Projection and preservation: [SuperDoc Document API](https://github.com/superdoc/docx-editor)
  (output projections, rich-content input, receipts);
  [rhwp](https://github.com/edwardkim/rhwp) (MCP tools, loss-safe save, IR diff).
- Round-trip laws: bidirectional transformations / lenses (Boomerang; lenses
  with complements).
- Document model: ECMA-376 WordprocessingML; HWPX / OWPML (KS X 6101);
  Pandoc's document tree as a checklist.
- Presentation: PresentationML placeholder types; Google Slides API layouts
  and placeholders; [Slidev](https://github.com/slidevjs/slidev),
  [Marp](https://github.com/marp-team/marp-core);
  [PPTAgent](https://arxiv.org/abs/2501.03936).
- Spreadsheet: [SpreadsheetLLM / SheetCompressor](https://arxiv.org/abs/2407.09025);
  Excel structured references;
  [Frictionless Table Schema](https://specs.frictionlessdata.io/table-schema/);
  [IronCalc](https://github.com/ironcalc/IronCalc).
- Engines: [rdocx / rpptx](https://github.com/tensorbee/rdocx),
  [ooxmlsdk](https://github.com/KaiserY/ooxmlsdk),
  [docx](https://github.com/dolanmiu/docx),
  [rust_xlsxwriter](https://github.com/jmcnamara/rust_xlsxwriter),
  [umya-spreadsheet](https://github.com/MathNya/umya-spreadsheet).
