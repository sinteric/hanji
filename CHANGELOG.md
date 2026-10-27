# Changelog

## Unreleased

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
