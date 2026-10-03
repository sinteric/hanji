# Changelog

## 0.3.0 — unreleased

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
  the pinned renderer, with a real Korean-deck regression.
- Preserve usable DOCX vertical-page bounds around tall headers and footers.
  Select HWPX Latin drawing fallbacks by actual character coverage so sparse
  Korean alias fonts do not lose digits and punctuation.
- Scope HWPX HTML page IDs and local SVG/CSS fragment references so repeated
  clip, paint and filter definitions do not bind across pages.
- Align the CLI/workspace and plugin version at 0.3.0. Document the preview
  support matrix and measured limitations in `PREVIEW.md`. DOCX/HWPX remain
  experimental; this release does not claim the native fidelity target.
