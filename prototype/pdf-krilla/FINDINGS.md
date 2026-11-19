# PDF export spike: krilla from the page display list

2026-11-19. Branch `claude/project-thread-8oev0w`. Not for merging.

## What was built

- `crates/hanji-preview/src/page.rs`: the display list (`page::Layout`, the
  one every format already lowers to) gained what a PDF needs and a preview
  drops: `Item::Link`, `Item::BeginMarked`/`EndMarked` around structure
  nodes, `Layout::structure()`, `Layout::headings()` and
  `Layout::face_data()`. `Glyphs` gained `stroke` (about 140 lines).
- `crates/hanji-preview/src/page_svg.rs`: HWPX text that rhwp draws bold as
  fill plus a same-colour stroke now stays text instead of being flattened to
  outlines (about 15 lines).
- `prototype/pdf-krilla/src/main.rs`: a krilla 0.8.2 writer over the display
  list (glyph runs as real text with the face Hanji chose, paths, gradients,
  images, groups, links, bookmarks, tag tree), about 650 lines, plus the page
  SVG to usvg to svg2pdf route for comparison. Its own Cargo workspace, so the
  main `Cargo.lock` is untouched.
- `prototype/pdf-krilla/analyze.py`: measures the PDFs with poppler, pypdf and
  pdfplumber.

Run: `SPIKE_FONT_DIR=<fonts> target/debug/pdf-krilla-spike <docx|pptx|hwpx> <file> <out>`.
Fonts: Noto Sans/Serif CJK, Nanum (Ubuntu noble packages) as caller fonts.

## Results (9 documents, 3 per format)

| | krilla (direct) | page SVG to svg2pdf |
|---|---|---|
| Real, searchable text | yes, every glyph run | yes, but re-shaped from `<text>` |
| Text vs unique glyphs on page | 100% (HWPX after the bold fix) | bold HWPX text duplicated (fdi: 410 extra chars); text under a PPTX shadow becomes an image (synth-korean-report: 36% of text lost) |
| Fonts embedded, subset, ToUnicode | all | all, but one copy per page |
| Korean | yes, with caller fonts | same |
| Links | DOCX and PPTX (13 links, rects over the right text) | none |
| Bookmarks | DOCX headings (3, 6, 4 entries) | none |
| Tagged PDF | DOCX (P, Hn, L, Table... from rdocx's structure tree) | none |
| PDF/A-2b (krilla's validator) | 7 of 9 pass | not attempted |
| PDF/A-2a, PDF/UA-1 | 2 of 3 DOCX pass | not possible |
| Visual vs preview PNG (pixels off after 1.2px blur) | 0 to 2.1% | 0 to 2.1% |
| Write time (debug build) | 2 to 640 ms | 160 ms to 9.3 s |
| Size | 34 KB to 280 KB | 1.1x to 2.6x larger |

PDF/A failures and why:
- PPTX and HWPX for 2a/UA: rpptx and the rhwp SVG path carry no structure, so
  nothing to tag (`MissingTagging`, `MissingDocumentOutline`).
- `ContainsNotDefGlyph`: sample-docx has a U+0085 control character drawn
  with .notdef, footnote-01 uses ❍ and ․ that no supplied font has. PDF/A
  forbids .notdef, so export needs a fallback face or must drop those glyphs.
- Without Korean caller fonts every Korean document fails the same way: the
  bundled faces have no Hangul.

veraPDF was not run: launching it was blocked in this environment, so PDF/A
here means krilla's own validator only.

## The page SVGs

The preview SVGs carry real `<text>` with per-character `x` and subset
`@font-face` data, not outlined glyphs. A reader re-shapes them, so glyph ids
are not preserved, and rhwp's stroked bold becomes two text objects in
svg2pdf. svg2pdf here is the 0.13 pdf-writer fork rhwp pins, not krilla.

## Gaps a production version must close

1. Structure for PPTX (slide title as heading, shapes in reading order) and
   HWPX (rhwp's paragraphs and outline levels, which the SVG path loses).
2. HWPX links and bookmarks: not in the SVG, so not in the display list.
3. Shadows: dropped (24 in one deck). Rasterise them under vector content.
4. HWPX draws one glyph per text object (rhwp emits per-character `<text>`):
   fine for copy and search, wasteful in size. Merging runs is cheap.
5. Variable-font coordinates: none in the samples, not handled.
6. .notdef policy for PDF/A, and a Korean face that ships with Hanji or is
   required from the caller.
7. Link annotations come one per run, should merge per link.

## Not used, but present

rdocx already has `oxml-pdf` (pdf-writer, PDF/A, tagging) over the same
`LayoutResult`, and rhwp has a Skia direct backend. Both are per-engine; the
krilla writer over Hanji's display list covers all three formats with one
path and matches the preview by construction.
