---
status: draft
date: 2026-09-29
measured: library and project facts below were checked 2026-09-28/29 against crates.io, npm, PyPI, GitHub and project docs; the §5 syntax choices were measured by fluency rounds 1–4 (2026-09-28, two Claude models) and the Presentation geometry by round 5 (2026-09-29, the same models); the remainder anchoring (§10.3) by the remainder prototype (2026-09-28, 13 docx); nothing here has yet been opened in real Office by this project
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
  the model.
- **Placeholders:** `<keep id="k3" kind="drawing" summary="org chart, 5 boxes"/>`
  (block or inline). The model may move or delete one explicitly, and in a
  Presentation resize it by its box (§5.3); it never alters its content, and
  never creates one except a picture from a file (§5.3).
- **Validator errors are written for the model:** line and column, the
  expected form, and the allowed names (styles, layouts, fields).
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
::title box="36 22 648 90"::
핵심 지표
::body box="36 126 648 356"::
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
<keep id="k7" kind="picture" summary="지도" box="560 20 124 80"/>
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
  - A number left as it is shown keeps the exact stored value; a changed number
    is used as written (1 pt = 12,700 EMU). A box whose four numbers are
    unchanged leaves its `a:xfrm` untouched (GetPut, §4).
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
  - Changing a slide's layout moves every slot whose box was left as shown
    and inherited to the new layout's place; a slot with its own box keeps it.
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
  `<shape id="s9" name="Oval 8" box="…"/>`. Its outline, fill and preset
  geometry stay in the remainder.
- **Pictures, charts, tables** and other objects the format does not model are
  a `<keep id kind summary box/>` line each (§5.1). They may be moved, resized,
  reordered or deleted, and their `id`, `kind` and `summary` never change.
  Resizing a table scales its column widths and row heights; a picture keeps
  its crop. Deleting one removes its parts; refused while an animation plays
  on it.
- **Lines and connectors** are `<line id="s6" name="…" from="x y" to="x y"/>`:
  the two ends, not a box. Writing them rewrites `a:off`, `a:ext` and the flips.
  Arrowheads, the connector's path and its connection ids stay in the
  remainder.
  - *Attached connectors.* Moving or resizing an object that a connector is
    attached to (its `stCxn` or `endCxn` names the object) is refused, with the
    connector and the reason named, unless the same edit also rewrites every
    attached `<line>`. The write does not reroute connectors: rerouting is
    deferred (§10.9), and a refusal is better than a connector left detached
    in PowerPoint (rule 1).
- **Groups** are a `<group id name box>` line, the group's objects, then
  `</group>`. Its objects show slide coordinates (through the group's child
  offset and extent), and the group's box is the box around them. Changing the
  group's box moves or scales the whole group, and canonical form writes its
  objects' new boxes. Changing its objects moves the group's box with them. A
  group box that disagrees with changed objects is refused. A rotated or
  flipped group is one `<keep kind="group" … box/>`. Groups are never created or
  ungrouped here.
- **New objects** are written without `id`; the write gives ids and names, and
  the text it returns shows them. They are:
  - a text box, `<shape box="…">text</shape>`;
  - a picture from a file, `<keep kind="picture" src="…" box="…"/>`, or, in a
    picture slot, the slot's marker holding `<keep kind="picture" src="…"/>`,
    which takes the slot's box;
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
| Validity | Every export opens in Word / PowerPoint / Excel / Hancom with no repair prompt; schema validation alone is not enough (a schema-valid file Word rejected: office_oxide #208) |
| Rule 5 (fidelity) | Per-page SSIM of our preview against the native application's own PDF export — Word, PowerPoint, Excel, Hancom — not against LibreOffice |
| Presentation geometry | GetPut per object on the pptx corpus: every `a:xfrm` (and every absent one) unchanged after import → export; PutGet after box, z-order, group and new-object edits; the office-kit shows moved, resized and added objects where the text puts them, in PowerPoint |
| Rule 8 (the model sees it) | Per corpus deck, every object on a slide appears in the text: slots, shapes with or without text, lines, groups and their objects, `<keep/>` lines |
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
   and previous text across the corpus.
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
   landed as often, and is one line for a shape without text). It also fixes a rule 8 gap: the old
   text left out connectors and textless shapes (9 of the 19 objects in
   shapes.pptx). Across the 21 corpus decks, 36% of slide placeholders have
   their own box. A schema-1 text without boxes still reads unchanged.
   Deferred: rerouting attached connectors (an edit that moves an attached
   object without rewriting its connectors is refused, §5.3), editing inside
   rotated groups (kept whole), and cm as a view or input over points (the
   unit Korean PowerPoint shows; not measured).

## 11. Blind spots

- No library above has been opened in real Office or Hancom by this project;
  every defect and fidelity statement comes from project docs and issue
  trackers. This check is deliberately deferred until the build produces
  exports (2026-09-28).
- The fluency list in §6 is still mostly a model's self-report: five rounds
  ran on two Claude models only, 12–48 tasks per cell, one run per prompt.
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
- The 90% fidelity target is not yet a defined metric beyond "per-page SSIM
  against the native app's PDF"; the threshold per page and per corpus is
  unset.
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
