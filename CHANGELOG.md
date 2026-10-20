# Changelog

## Unreleased

- Stop the formula tokenizer from panicking on an unclosed `[` in model-written
  formulas and in imported XLSX defined names or cell formulas.
- Advance the rdocx/rpptx/oxml-layout family to sinteric/rdocx main `66611b3`
  (S94, including zip 9.0.3+, plus S93 and sinteric/rdocx#20).
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
