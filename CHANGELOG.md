# Changelog

## Unreleased

- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx#22 (`944ce69`,
  on main `73894d2`): header and footer paragraphs with a positioned frame
  (`w:framePr`, such as the page-number gallery's) are placed at their anchor
  instead of taking flow room, contextual spacing drops space between
  paragraphs of one style, a PowerPoint layout placeholder inherits the master
  placeholder of its type, and chart labels take the chart's `c:txPr` with a
  one-series chart showing its automatic title.
- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx main `73894d2`
  (S95.4 / rdocx 0.16.4, rpptx and oxml-layout 0.14.4: Word legacy/modern
  form-field authoring, mail merge, relationship/style preservation, body
  read/edit bounds, PowerPoint publisher bounds, CJK line measurement,
  rendering changes, plus an upstream merge).

## 0.7.0 — 2026-10-28

- Show DOCX run-level tracked insertions and deletions (`w:ins` / `w:del`)
  whose inner text is one line as `<ins>…</ins>` / `<del>…</del>` (empty:
  `<ins/>` / `<del/>`) instead of `<keep kind="tracked-insert"/>` /
  `<keep kind="tracked-delete"/>` placeholders. Author, date, revision id
  and the original run XML stay as a `Wrap`. Editing the body rewrites
  `w:t` / `w:delText` and keeps those attributes; deleting the tag removes
  the mark and is reported; replacing it with ordinary text drops the mark
  and keeps the words. A new `<ins>` / `<del>` is refused (cannot invent a
  revision). Moves (`w:moveFrom` / `w:moveTo`), paragraph- or table-wrapping
  revisions, nested content the tags cannot hold, and paragraph-mark
  property revisions stay placeholders. hwpx and pptx refuse the tags.
  Corpus keeps: 315 → 284 (docx `tracked-insert` 17 → 0, `tracked-delete`
  14 → 0; `tracked-move` 2 unchanged).
- Show a Hancom equation (`hp:equation`) whose script is ordinary one-line
  text as `$…$` / `<math>…</math>` instead of a `<keep kind="math"/>`
  placeholder. Size, baseline, font, colour and the rest of the object stay
  as a `Wrap`. Editing the script writes it back to `hp:script` and keeps
  those attributes; Hancom's cached box (`hp:sz`) is left as stored, so an
  edited equation may display at its old size until Hancom lays the formula
  out again. Deleting the math removes the equation; a new `$…$` writes a
  new one (a guessed box). An empty script, one holding a newline, or one
  holding `</math>` stays a placeholder. docx OMML math is unchanged.
  Corpus keeps: 284 → 268 (hwpx `math` 16 → 0). The `number` keep in
  `eq-002.hwpx` (`hp:autoNum`) is left as-is.
- Show a single-paragraph DOCX field whose cached result holds a
  hyperlink (or another wrapper) and nested fields — a one-entry TOC's
  `PAGEREF` — as the same `Wrap` as #124/#130, including inside a shown
  content control. `begin`…`end` grouping no longer requires every sibling
  to be a run, so a blank leftover TOC that is only an empty hyperlink
  becomes one `<keep kind="field"/>` instead of four `field-part` keeps.
  Empty form fields, checkboxes, blank XE/TA, and parts that share a run
  with the result stay placeholders. An inline page break mixed with text
  (`w:br type=page` in a TOC heading) stays `<keep kind="break"/>`:
  `<pagebreak/>` is a paragraph. Corpus keeps: 319 → 315 (docx
  `field-part` 4 → 0; `field` 73 and `break` 2 unchanged).
- Show a DOCX content control (`w:sdt`) whose displayed value is text as
  ordinary paragraphs and runs instead of a `<keep kind="content-control"/>`
  placeholder: rich and plain text, a combo box or drop-down, a citation,
  a bibliography, and a `docPartObj` (heading TOC, bibliography gallery).
  The `w:sdt` / `w:sdtContent` shells and `w:sdtPr` / `w:sdtEndPr` stay as a
  `Wrap`. Editing the text keeps the control properties; deleting all of
  an inline control's text, or the first or last paragraph of a block
  control, removes it and is reported. Joining a paragraph onto a block
  control expands the control to the whole joined paragraph. Dates
  (`fullDate` in `sdtPr`), checkboxes, pictures, repeating sections,
  data-bound controls
  (`w:dataBinding`), and controls still showing placeholder text
  (`w:showingPlcHdr`) stay placeholders, as does a parent that contains
  any of those nested. Row- and cell-level controls the table model
  cannot hold stay inside a table placeholder. Comments inside a shown
  control stay `<comment>` balloons. Corpus keeps: 327 → 319
  (docx `content-control` 19 → 5; unwrapping a heading TOC `docPartObj`
  also surfaces inner leftovers the parent keep had hidden: `field` 72 →
  73, `field-part` 0 → 4, `break` 1 → 2).
- Show a DOCX comment whose body is ordinary text as
  `<comment>…</comment>` at the balloon instead of a
  `<keep kind="comment"/>` placeholder. The commented range is ordinary
  text (range start/end stay remainder markers); author, date, initials,
  threading and resolved state stay in `comments.xml` and the extended
  parts. Editing the body updates the comment part; editing the anchored
  text keeps the anchors; deleting the `<comment>` marker removes the
  comment and its extended-part entries and is reported; deleting the
  anchored range leaves a point comment. Replies, rich balloons (tables,
  images) and mixed runs stay placeholders. Tracked-change export refuses
  a change to the balloon. Corpus keeps: 345 → 327 (docx `comment` 18 → 0).
- Show a DOCX complex field whose cached result spans paragraphs (TOC,
  INDEX, TOA, and similar) as ordinary editable paragraphs instead of
  `<keep kind="field-part"/>` placeholders. The instruction stays as one
  `Wrap` remainder entry: `begin`…`separate` on the first paragraph and
  `end` on the last (`end_path`). Editing result text is allowed; adding
  or removing a paragraph strictly inside the range is written back when
  it can be placed faithfully; deleting the first or last paragraph
  removes the field and is reported; splitting a boundary paragraph
  keeps begin and end with the pieces that still hold them; reordering
  the ends is refused, as are tracked changes inside the range.
  Nested single-paragraph fields in the result (a TOC entry's `PAGEREF`)
  keep the existing wrap. Blank, unmatched, or shared-run spanning fields
  stay placeholders. Corpus keeps: 357 → 345 (docx `field-part` 12 → 0).
- Show a DOCX legacy form field's cached result as text when it is not
  blank (`FORMTEXT` / `FORMDROPDOWN` with `w:ffData`), the same way
  ordinary fields already are: editing it replaces the result and keeps
  the instruction, `ffData` and the form bookmark. Empty `FORMTEXT`
  (padding spaces) and `FORMCHECKBOX` (no cached result; state is in
  `ffData`) stay `<keep kind="field"/>` placeholders. Corpus keeps
  unchanged: 357 (docx `field` still 72 — the leftovers are empty form
  fields, checkboxes, and blank XE/TA).
- Corpus `<keep/>` placeholders went 357 → 268 since 0.6.0
  (`docs/keep-census.md`).

## 0.6.0 — 2026-10-26

**Breaking changes**

- Public library types that may grow now carry `#[non_exhaustive]` (Google
  Rust Style Guide; deferred from #122). Matches on those enums need a `_`
  arm (`&_` when matching a reference). Struct literals and functional
  record update (`Foo { x, ..Default::default() }`) from outside the
  defining crate no longer compile: use a constructor, or `Default` and
  then assign fields. Closed ontologies (`Format`, `DocType`, two-state
  enums, SVG path/stroke enums, `ColorBase` / `Tint`, `Tag`,
  `PreviewSource`, `DocumentKind`) are unmarked.

- Show XLSX merged cells as editable `<merge range="A1:B2"/>` lines instead of
  a `<keep kind="merged-cells"/>` placeholder. Each rectangle can be kept,
  added, removed or retargeted. Sheets with more than 1024 merges, or with
  overlapping or single-cell merges, stay placeholders. Corpus keeps: 374 →
  357 (`xlsx` merged-cells 18 → 1).
- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx main `45ac63f`
  (S95.2 and S95.3 / rdocx 0.16.3, rpptx and oxml-layout 0.14.3: Word
  content-control shells and typed payloads, custom XML data binding, template
  grammar, numbering navigation, PowerPoint authoring and table-row paint,
  bounded edit scans, stricter validation).
- Lay out PPTX, DOCX and HWPX pages as a display list (`hanji_preview::page`):
  glyph ids with their outlines, paths, images and groups, in hanji's own
  types, from a package or from a revision's text and remainder. For viewers
  that draw pages themselves instead of showing SVG or PNG.
- Read a DOCX field's cached result as text when it is text in one paragraph
  (page numbers, caption `SEQ` numbers, cross-references, `HYPERLINK` fields)
  instead of a `<keep kind="field"/>` placeholder. Editing it replaces the
  cached result and keeps the instruction; deleting all of it removes the
  field and is reported; splitting it across paragraphs, and tracked changes
  inside it, are refused. Legacy form fields, fields with no or a blank
  result, and fields spanning paragraphs stay placeholders. Corpus keeps:
  387 → 374 (DOCX `field` 85 → 72; `field-part` 12 unchanged).
- Count the `<keep/>` placeholders of every corpus file, with a committed
  baseline and a test that fails when it is stale. Since that baseline the
  count has gone 387 → 357.
- Adopt the Google Rust Style Guide as hanji's base, reformat the tree with
  the guide's rustfmt options, and check formatting in CI. `#[non_exhaustive]`
  on public library types, deferred from that adoption, is applied in this
  release (see Breaking changes).
- Show `###########` for XLSX date and time formats on serials past
  9999-12-31, as Excel does, instead of panicking on huge values (`1e300` as
  `yyyy-mm-dd`), and stop long `ss.000…` runs from showing float noise or the
  wrong date.
- Stop XLSX percent formats from panicking in display text and previews when
  scaling leaves f64's range (`0%` on `1e307`, or hundreds of `%` signs):
  the decimal point is moved in the digits instead.

## 0.5.0 — 2026-10-21

- Stop XLSX fraction formats with long denominators (`?/??????????`) from
  panicking or spinning for minutes per cell in display text and previews.
- Stop the formula tokenizer from panicking on an unclosed `[` in model-written
  formulas and in imported XLSX defined names or cell formulas.
- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx main `562aefb`
  (S95 and S95.11: Word authoring APIs, stricter Word/PowerPoint value
  validation, typed content controls, imported chart types). PPTX previews now
  also hide master shapes when the slide itself sets `showMasterSp="0"`.
- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx main `66611b3`
  (S94, including zip 9.0.3+, plus S93 and sinteric/rdocx#20).
- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx main `6725e96`
  (S92).
- Bump rhwp from sinteric/rhwp `260c6df` to main `d2fdbebf` (zip 9.0.4).
- Warn when a pie or doughnut category legend is omitted because the chart is
  too small, in DOCX and PPTX preview JSON/HTML and the CLI, without changing
  the source document.
- Bump the pinned Rust toolchain from 1.99.0 to 1.99.1 (`rust-toolchain.toml`,
  `.mise.toml`, and CI). The workspace `rust-version` remains 1.99.
- Move hanji-package, hanji-docx, the rdocx family, rhwp, and
  `validation/office-kit` from yanked zip 8.6.0 to zip 9.0.3+ (lockfiles resolve
  9.0.4; CI still uses `--locked`).
- Size the wasm32 linear stack at 16 MiB so S92 table-style parsing does not
  trap in debug wasmtime.

## 0.4.0 — 2026-10-14

- Export a Document in its other format (`docx` ↔ `hwpx`) through the CLI
  and store API, with a conversion report of what did not cross.
- Preserve PNG page content at very low resolutions. Bound decoded image
  resources in DOCX and shared preview output. Draw DOCX GIF first frames as
  static previews and diagnose animation as a first-frame approximation.
- Restore later-page vertical space for selected DOCX stories. Preserve DOCX
  shorthand-RGB watermarks and rounded PPTX preset corners through the pinned
  renderer.
- Preserve worksheet theme colors and tints, authored indentation, and
  ZWNJ/ZWJ joiners without fallback spacing or extra numeric overflow.
- Create missing DOCX style tables when paragraph-style edits require them.
  Keep each HWPX tab’s stored XML with that tab when nearby tabs change.
- Preserve CDATA text in DOCX and PPTX imports. Discover DOCX preview fonts
  from CDATA runs. Parse literal CDATA in the pinned renderer instead of
  treating markup as text.
- Refuse unchanged PPTX connectors whose targets were deleted. Detach a
  connector in the same export when its attached target is deleted and the
  endpoint is rewritten as a point.
- Refuse XLSX row insertions that would overflow ordinary cells. Update
  cross-sheet chart sources when worksheet rows move.
- Bump rhwp from sinteric/rhwp `fed548c` (0.8.6) to main `260c6df` (0.8.7 plus
  Hanji fork contracts). Advance the rdocx/rpptx/oxml-layout family through
  S90. Pin the workspace `rust-version` and toolchain at 1.99.
- Bound hosted CI concurrency and Rust cache writes. Manage plugin-validation
  tools through mise and pnpm.

## 0.3.0 — 2026-10-04

- Require `reimport --base-rev N` (the exported revision) or an explicit
  `--replace-head`. A stale base refuses before changing the store, including
  XLSX cell-only changes. Add checked re-import library APIs; retain the
  original methods as documented authoritative replacements.
- Add experimental DOCX and HWPX page previews through the reusable library
  and native CLI, with SVG, PNG and HTML output from exported revision bytes.
  File previews remain unstored and leave their source bytes unchanged.
- Add a shared document-byte/page-buffer API, font provenance, actual missing
  character reports and rendering diagnostics. Keep existing PPTX APIs and
  slide output names; add page counts/names for DOCX and HWPX.
- Include bounded XLSX worksheet windows in the CLI, with cached formula
  status, explicit resource budgets and diagnostics for omitted features.
- Use immutable fork sources for the compatible rdocx/rpptx/oxml-layout family
  and rhwp. Enable HWPX by default and cover it in byte-only CI checks.
- Correct varied pie/doughnut category colors and category legends through
  the pinned renderer, with a real Korean-deck regression. Rebind grouped
  ordinary glyphs with their drawing font’s IDs/advances while preserving
  compatible word spacing; retain rich cluster positioning with a warning.
- Preserve usable DOCX vertical-page bounds around tall headers and footers.
  Select HWPX Latin drawing fallbacks by actual character coverage so sparse
  Korean alias fonts do not lose digits and punctuation.
- Scope HWPX HTML page IDs and local SVG/CSS fragment references so repeated
  clip, paint and filter definitions do not bind across pages.
- Align the CLI/workspace and plugin version at 0.3.0. Document the preview
  support matrix and measured limitations in `PREVIEW.md`. DOCX/HWPX remain
  experimental; this release does not claim the native fidelity target.
