# Spike: preview fidelity (DESIGN.md §4 rule 5, §9)

**Question.** Which renderer should hanji's preview use, and how is rule 5
("preview fidelity above 90% of what the native application shows") measured?

**Short answer.**

- **Raw per-page SSIM, §9's metric until now, cannot decide this.** A blank
  page scores 0.868 against the native pages and a 1 px shift 0.919. rhwp's
  render of file 25 page 1 is near-identical to Hancom's layout but scores
  0.57 because its substitute font draws different glyphs.
- **Rule 5 scores layout instead:** where each word lands, the line it
  starts, the page it is on, and whether the text is there. Glyph shapes do
  not enter the score. Calibrated below: native vs itself 1.000, blank pages
  0.000, a 1 px shift 0.987, 5 pt 0.898, 10 pt 0.734. rhwp 25 scores 0.948
  and rdocx 04 (5 pages for Word's 2) 0.702.
- **Preview ships the in-binary engines only:** rdocx (docx), rpptx (pptx)
  and rhwp (hwpx), drawing SVG per page. xlsx becomes an HTML grid (later).
  No LibreOffice. Decided with the owner, 2026-10-01 (decisions below).
- **No engine meets the proposed threshold today.** Corpus layout scores: rhwp
  0.836 on hwpx, rpptx 0.791 on pptx (0.904 without deck 12, which it refuses
  to open), rdocx 0.619 on docx. LibreOffice reaches 0.924 on pptx and fails
  hwpx (0.285).
- **Fonts are the main lever for the look; engine bugs are separate.** The
  layout metric is built to tell the two apart (next steps, item 1).
- **5 of 9 docx baseline PDFs are unusable for geometry.** Word printed them
  in its markup view (01, 02, 05, 06, 07: page shrunk to ~70%, comment pane on
  the right). The scorer detects this and rescales, but those PDFs should be
  re-exported with Review → No Markup.

## Setup

| | |
|---|---|
| Inputs | `baseline/`: 30 hanji exports (hanji `0024cc3`) of corpus files: 9 docx, 8 pptx, 5 xlsx, 8 hwpx; untouched, edited (E10 / pset) and new. `MANIFEST.json` gives source, edits and SHA-256 |
| Native PDFs | Word, PowerPoint and Excel 16.113.1 and Hancom Office 12.30.0 on macOS 27.0, exported by the owner per `baseline/CHECKLIST.md`. No repair prompts. **Not committed** (`native/`, git-ignored) |
| LibreOffice | 24.2.7.2 (Ubuntu noble: `libreoffice-writer`, `-calc`, `-impress`; `libreoffice-core` alone fails with "source file could not be loaded") + `libreoffice-h2orestart` 0.6.1 for hwpx |
| Fonts | 73 files from noble packages: Noto Sans/Serif CJK, Nanum, UnFonts, Baekmuk, Carlito, Caladea, Liberation (`fonts/FONTS.md`). **No HCR Batang/Dotum (함초롬)**: hancom.com is blocked by this environment's proxy (CONNECT 403). No Malgun Gothic, Batang, Calibri, Aptos. Every MS and Hancom name maps to a substitute (`fonts/fonts.conf`, `engines/aliases.txt`) |
| Rasters | 96 DPI (pymupdf for PDFs, resvg 0.48.1 for engine SVGs) |
| Python | 3.11, pymupdf 1.28.2, scikit-image 0.26, rapidfuzz 3.14, cairosvg 2.9 |

Candidates:

| Candidate | Formats | Output |
|---|---|---|
| LibreOffice | docx, pptx, xlsx | PDF |
| LibreOffice + H2Orestart | hwpx | PDF |
| rdocx 0.14.0 | docx | SVG per page (`engines/rdocx`) |
| rpptx 0.12.1 | pptx | SVG per slide (`engines/rpptx`) |
| rhwp 0.8.6 (`680111e`) | hwpx | SVG per page (`engines/rhwp`) |

## Why raw SSIM fails

Round 1 scored every page with SSIM (grayscale, 96 DPI). Round-1 mean raw
SSIM: LibreOffice docx 0.865, pptx 0.949, xlsx 0.832, hwpx (H2O) 0.586; rdocx
0.801; rpptx 0.837 (0.956 on the 7 decks it rendered); rhwp 0.801.

| Probe | Raw SSIM |
|---|---|
| native vs a blank white page | 0.868 |
| native vs itself shifted 1 px | 0.919 |
| rhwp 25 p1, same layout, substitute glyphs | 0.571 (p2–p6: 0.61–0.94) |
| rhwp 25 p2 drawn with Noto, then with Nanum (same SVG) | 0.781 → 0.712 |

SSIM is dominated by white paper and by glyph shape. A blank page beats a
correct page drawn in another font, so the number does not rank renderers.
The content-masked SSIM (SSIM averaged over pixels with ink in either image)
fixes the blank page, but it still punishes glyph shape (rhwp 25 p2 0.515 with
Noto, 0.363 with Nanum, same layout). It stays as the secondary signal.

## Layout metric (`layout_score.py`)

1. **Glyphs.** Each source becomes glyphs (character, page, origin x,
   baseline y, font size) in page-relative pt. PDFs (native and LibreOffice)
   use pymupdf's char origins. Engine SVGs use every `<text>`: its `x`/`y`
   lists (or `textLength` spread), every ancestor `transform`, and the root
   `viewBox` scaled to pt (rdocx/rpptx: pt viewBox; rhwp: px, one `<text>`
   per character). Text is NFKC-normalised and whitespace dropped, and every
   bullet or list glyph (including Symbol-font private-use bullets) becomes
   one token `•`.
2. **Words** come from the native PDF (pymupdf's segmentation: whitespace and
   line ends; Korean words are eojeol). Candidates are not segmented: rhwp
   writes no spaces and PowerPoint's PDFs drop them, so the alignment runs on
   characters.
3. **Alignment.** One LCS over the whole document (rapidfuzz Indel), keeping
   runs of ≥ 2 characters or whole native words. Then two more passes over
   the leftovers (runs ≥ 4) catch blocks emitted in another order (headers,
   text boxes).
4. **Lines.** Per page, glyphs are banded by baseline (≤ 0.3 em apart), and a
   band splits where the gap between glyph origins exceeds 2.5 em. The same
   rule runs on both sides. A word is a line start when its first glyph
   starts a line.
5. **Submetrics**, each 0..1:
   - **T** text: character F1 = 2·matched / (native + candidate characters), so missing and extra text both count.
   - **P** page: share of matched native words (≥ half their characters matched) whose counterpart is on the same page.
   - **L** line breaks: F1 of the line-start flags over matched words, so a break added and a break lost both count.
   - **W** position: mean over same-page matched words of 1 − min(1, d / (2% of the page diagonal)). d is the distance between the centroids of the matched glyph origins. 2% of A4 is 20.6 pt.
   - **layout = (T·P·L·W)^¼** per file, over the whole document.
   - **Per page:** (T_k·L_k·W_k)^⅓, where T_k counts only matches that stay on page k.
   - **F** font identity (**proposed**, not in `layout` yet): share of matched characters drawn in the font family the native PDF used. See next steps, item 1(g).
   - **W_rel** (diagnostic): W after removing each page's median offset. A high W_rel with a low W means the page moved as a whole.
6. **Baseline guard.** A native page printed in Word's markup view is
   detected by the grey (0.949) comment pane. The pane's text is dropped and
   the page is mapped back to full size: scale = pane height / sheet height,
   top = pane top. Checked against LibreOffice on 02/07: fitted scale within
   0.3%, offset under 2 pt. 01/06 still sit 5–10 pt off, so treat them as
   provisional until they are re-exported.

**Why a geometric mean.** A preview that loses half the text, or puts every
word on the wrong page, must not score well because its other submetrics are
perfect. The product falls when any factor falls, and a blank page is 0. An
arithmetic mean gives 0.75 to a page with all text on the right lines but
everything 25 pt off. The fourth root keeps moderate defects readable: T =
0.5 alone gives 0.84. Equal weights: no data here says one factor matters more.

**Not covered.** Text-free content (shapes, pictures, fills, borders, chart
marks) has no words. Content-SSIM covers it as the secondary check. A page
with no native text (11 p3, p6; 18/21 p4; 29 p3) is judged by content-SSIM
alone. A coarse box metric for drawing objects (SVG `rect`/`path`/`image`
bounds vs pymupdf `get_drawings`/`get_images`) is a cheap next step and is
not built.

## Calibration (`calibrate.py`, `results/calibration.json`)

| Probe | Layout | Notes |
|---|---|---|
| native vs itself (30 files) | **1.000** | |
| blank pages | **0.000** | raw SSIM 0.868 |
| every glyph shifted 1 px (0.75 pt, both axes) | 0.987 (min page 0.975) | raw SSIM 0.919 |
| shifted 2 px | 0.973 (0.949) | |
| shifted 5 pt (both axes) / 5 pt in x only | 0.898 (0.801) / 0.931 (0.869) | |
| shifted 10 pt | 0.734 (0.307) | |
| rhwp 25: Hancom's layout, substitute glyphs | **0.948**; pages 0.96 0.99 0.98 0.94 **0.00** 0.97 | p5: rhwp drops the "[별지 제4호 서식]" line, so the page sits 34 pt high (W_rel 0.98) |
| rhwp 25 p1–p3, the same SVG drawn with Noto and with Nanum | identical (0.961, 0.992, 0.978) | raw SSIM p2 0.781 → 0.712, content-SSIM 0.515 → 0.363: **glyph substitution moves SSIM, not layout** |
| rdocx 04 (5 pages for Word's 2) | **0.702**; pages 0.60 0.79 0 0 0 | |
| LibreOffice 09 with Korean → NanumGothic instead of Noto Sans CJK KR | 0.712 vs LibreOffice default (T=P=L=1, W=0.26) | same line breaks, shorter line height: a **metric** change, scored as layout |
| LibreOffice 24 with Nanum instead of Noto Serif | 2 pages (Hancom: 2; LO+Noto: 3); 0.401 vs native (LO+Noto 0.224) | the font's metrics decide the reflow |

The metric separates the two cases rule 5 needs. Glyph shape alone (rhwp 25)
leaves the score unchanged. A different layout (rdocx 04) or a font with
different metrics (LO + Nanum on 09) scores lower.

## Results (`results/summary.md`)

Corpus = per format; file score = layout over the whole document;
content-SSIM and raw SSIM are round 1's. "clean" leaves out the five docx with
markup-view baselines (01, 02, 05, 06, 07). Their content-SSIM and raw SSIM
were scored against the shrunken pages and are not meaningful.

| Candidate | Format | Files | Layout | T | P | L | W | Content-SSIM | Page count off | Files ≥ 0.85 | Pages ≥ 0.80 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| rdocx | docx | 9 | **0.619** | .967 | .961 | .936 | .222 | .462 | 2 (04: 5/2, 05: 4/3) | 0% | 7% |
| rdocx | docx clean | 4 | 0.607 | .971 | .947 | .905 | .229 | .168 | 1 | 0% | 10% |
| rpptx | pptx | 8 | **0.791** | .871 | .875 | .866 | .611 | .469 | 1 (12 refused) | 62% | 60% |
| rpptx | pptx w/o 12 | 7 | 0.904 | .996 | 1.00 | .990 | .698 | .535 | 0 | 71% | 68% |
| rhwp | hwpx | 8 | **0.836** | .990 | .851 | .951 | .713 | .396 | 1 (28: 6/7) | 62% | 60% |
| LibreOffice | docx | 9 | 0.822 | .988 | .989 | .983 | .535 | .537 | 1 (05: 4/3) | 44% | 58% |
| LibreOffice | docx clean | 4 | 0.781 | 1.00 | .991 | .961 | .472 | .363 | 0 | 50% | 43% |
| LibreOffice | pptx | 8 | 0.924 | .998 | 1.00 | .982 | .760 | .535 | 0 | 75% | 81% |
| LibreOffice | xlsx | 5 | 0.585 | .932 | .725 | .941 | .333 | .235 | 1 (20: 15/21) | 40% | 9% |
| LibreOffice + H2O | hwpx | 8 | 0.285 | .927 | .551 | .643 | .071 | .157 | 5 | 0% | 0% |

T is high everywhere: every engine draws the text. Layout separates the
candidates on **W** (where the text lands) and **P** (pagination). F, computed
for the LibreOffice PDFs, is 0.00 on all 30 files: no file's requested font
exists in this environment (Calibri → Carlito, 맑은 고딕 → Noto, 함초롬바탕 →
Noto Serif).

Worst three per engine (pictures in `diffs/`, native left):

- **rdocx.**
  - **09 (0.377).** The table is stretched to full width instead of autofit, and line height is compressed, so content drifts 35 pt up (W 0.02). LibreOffice also scores 0.666 here.
  - **02 (0.461) and 07 (0.482).** The header is drawn over the title and paragraph spacing is larger, so pages drift 43 pt (W_rel 0.63). Both rest on markup-view baselines.
  - **03 (0.587, `diffs/rdocx-03-p2.png`).** Table cell spacing (and the table background it shows) is not drawn, and column widths differ. Rows come out shorter, and p2/p3 content sits 37/174 pt high.
  - **04 (0.702, `diffs/rdocx-04-p1.png`).** List and paragraph spacing is too large (5 pages for 2), and the text box is drawn without its border in a different place.
  - **05 (0.664).** The VML watermark is dropped, OfficeMath is not rendered, and the header and footer overlap the body (4 pages for 3).
- **rpptx.**
  - **12 (0).** The deck is refused: `open: malformed PresentationML part /ppt/slides/slide1.xml: invalid value: duplicate p:attrName` (a schema-valid `p:attrNameLst` in an animation).
  - **15 (0.809) and 10 (0.838, `diffs/rpptx-10-p2.png`).** Bullet line spacing is larger and the indent wider, so body text drifts 20 pt down by the last bullet (W 0.43–0.49).
  - **17 (0.872).** On p4 the smaller "· Thank you" run is drawn at the title size and the title sits 18 pt off (page 0.27).
- **rhwp.**
  - **28 (0.329, `diffs/rhwp-28-p1.png`).** 6 pages for Hancom's 7, and the body starts 41 pt higher on p1 (top margin / header space), so pagination drifts (P 0.27).
  - **26 (0.719).** The unedited source of 28: the body sits 25 pt high on p1/p3 and 174 pt high on p5 (W_rel 0.62–1.00, so whole blocks move). Line breaks mostly agree (L 0.92).
  - **29 (0.827).** Hancom's p3 holds only the logos, while rhwp's p3 holds the whole press-release body (P 0.58).
  - **25 (0.948, `diffs/rhwp-25-p1.png`, `diffs/rhwp-25-p5.png`).** Near-identical except p5's dropped first line. Also from round 1: `LAYOUT_OVERFLOW: page=1, sec=2, col=0, para=4, type=Table, first=false, y=1038.3, bottom=1028.0, overflow=10.3px`.

LibreOffice for reference: pptx is the best row here, but 13 and 16 place a text
box 37 pt off on their last slide. xlsx breaks pages differently from Excel (20: 15 pages for
21). H2Orestart reflows every hwpx (23: 14 pages for 9; 24: 3 pages for 2 with
Noto Serif, 2 with Nanum).

## Engine notes (from round 1, `results/*/engine_results.jsonl`)

| | rdocx 0.14.0 | rpptx 0.12.1 | rhwp 0.8.6 (`680111e`) |
|---|---|---|---|
| Licence | MIT OR Apache-2.0 | MIT OR Apache-2.0 (same maintainer and repo as rdocx) | MIT |
| Build | `default-features = false`; 3m27s cold, no `-sys` crates, MSRV 1.93 | `render`, `default-template`; 4m13s cold | `default-features = false`; 6m00s cold; `blake3` needs a C compiler at build time; wasm-bindgen compiled in on native; brings its own usvg/resvg 0.45 next to 0.48 |
| Binary cost (stripped, over a 4.4 MB resvg+serde_json baseline) | +19 MB, of which 8.6 MB is bundled fonts | +18 MB, mostly shared with rdocx (`oxml-*`) | +10.6 MB, no fonts |
| Run deps | libc/libm/libgcc_s only | same | same |
| SVG | yes (`render_page_to_svg`); the CLI lays out with caller fonts and lowers with a copy of rdocx's `svg.rs` | no SVG API: rpptx's `LayoutResult` is the same `oxml-layout` type, lowered by the same `svg.rs` copy | yes, layer SVG (`RenderProfile::Screen`) |
| Text in SVG | `<text>` per glyph run, fonts embedded whole as base64 `@font-face`: **47–81 MB per page** with a CJK `.ttc`, so they need subsetting | same, 26–54 MB per Korean slide | `<text>` per character, positions from rhwp's built-in metric DB; **no fonts embedded** (0.2–0.8 MB/page): the look depends on the viewer's fonts |
| Fonts | caller fonts + aliases accepted; Hangul reaches Noto Sans CJK **SC** by coverage fallback, not via `w:eastAsia`; `.ttc` face index is lost in SVG (browser shows face 0) | public render is deterministic only (bundled + embedded fonts, no CJK: every Korean glyph is tofu). Caller fonts need `layout_presentation_with_font_manager`, which **drops per-paragraph text direction** | layout needs no font files; the chain of families per `<text>` is resolved by the viewer |
| Speed | 0.3–1.9 s per file (04: 4.3 s) | 0.65–5 s per deck | SVG 0.003–0.08 s per document |
| Gaps seen | 04/05 page counts; 05 OfficeMath and VML watermark; 02/07 one non-PNG/JPEG image omitted | deck 12 refused (above); connector line styles; `hlinksldjump` | 28 pagination; 25 p5 dropped line; drops hwpx track-change marks (prototype/tracked-changes) |

## Proposed rule-5 threshold

Per format corpus (each of docx, pptx, hwpx; xlsx separately, below):

1. Mean file layout score **≥ 0.90**. That is roughly every word 5 pt off at most (0.898 calibrated); a 1–2 px error scores 0.97–0.99.
2. **≥ 80%** of files score **≥ 0.85**, which lies between rhwp 25 (0.948) and rdocx 04 (0.702).
3. Page count exact on **≥ 90%** of files. On today's 8–9-file corpora that means every file, which is intended: a wrong page count is the most visible failure.
4. **≥ 90%** of pages score **≥ 0.80** (a 5 pt shift: min page 0.80). A page with no native text counts when its content-SSIM is ≥ 0.80. Shapes-only slides 11 p3 and p6 score 0.92–0.97 in every candidate.
5. Font identity (**proposed**, item 1(g)): F ≥ 0.99 over text whose requested font is available to the run (org or user font dir, installed, or bundled). Text in an unavailable font is excluded from F but must appear in the substitution list.

Content-SSIM is reported and reviewed, not gated. On text pages it stays
glyph-bound (0.1–0.6 for good layouts).

Today: **no engine passes.**

| Engine | Mean | Files ≥ .85 | Pages exact | Pages ≥ .80 |
|---|---|---|---|---|
| rhwp | 0.836 | 62% | 88% | 60% |
| rpptx | 0.791 (0.904 without 12) | 62% | 88% | 60% |
| rdocx | 0.619 | 0% | 78% | 7% |
| LibreOffice pptx | 0.924 | 75% | 100% | 81% |

LibreOffice pptx is the nearest. That is the expected spot for a mature
renderer, short on W.

**xlsx.** There is no engine, and the planned preview is an HTML grid, not
pages. Rule 5 for xlsx should therefore compare **displayed values and
formats** with Excel's PDF text:

- For each printed cell, the shown string: number format, date format, rounding, the `####` overflow rule, and text that spills into the next cell.
- Its row/column order and the sheet it is on, aligned like T and P above.
- Fill, font weight, borders and merged ranges from the grid's own model.

Page geometry does not apply. LibreOffice xlsx (0.585 here) is listed only as
a reference. Not built.

## Decisions (with the owner, 2026-10-01)

- **(a)** Rule 5 scores layout (this metric, content-SSIM secondary), not raw SSIM.
- **(b)** Preview ships the in-binary engines only: rdocx for docx, rpptx for pptx, rhwp for hwpx, each as SVG per page with fonts subset and embedded by hanji. xlsx is an HTML grid, later. No LibreOffice: it is a reference in this spike, not a preview engine. It is also a large external dependency, and on hwpx (H2Orestart) it is the worst candidate.
- **(c)** This spike is committed without the native PDFs. Only six downscaled diff pictures are included.

## Next steps

1. **Font system.** Fonts decide most of the look. SeongUk noted that apart
   from fonts, rhwp's 25 looks like Hancom's, and the numbers agree: layout
   0.948, raw SSIM 0.57–0.94. Engine bugs (rdocx 04/05, rhwp 28, rpptx 12) are
   a separate problem, and the layout metric is built to separate the two.
   Glyph shape does not move it; metrics and engine layout do.
   1. **Name resolution.** One alias table from document font names
      (함초롬바탕/돋움, 맑은 고딕, 바탕, 굴림, Calibri, Cambria, Aptos, Arial,
      Times New Roman, …) to the fonts available. This spike's table is
      `fonts/fonts.conf` / `engines/aliases.txt`. Shared by the three engines
      so that rdocx/rpptx stop reaching Hangul through Noto CJK SC coverage
      fallback.
   2. **Layout metrics.** Break lines with the original font's metrics, or a
      metric-compatible substitute (Carlito = Calibri, Caladea = Cambria,
      Liberation = Arial/Times/Courier), even when the glyphs come from
      another font. rhwp already lays out from built-in metrics, which is why
      25 kept Hancom's layout with Noto glyphs. LibreOffice with Noto Serif on
      24 reflowed to 3 pages instead of Hancom's 2 (with Nanum: 2).
   3. **Bundled set.** Noto Sans/Serif CJK KR, Carlito, Caladea and
      Liberation. 함초롬바탕/돋움 too, if Hancom's licence allows
      redistribution. That is **unverified**: its download page could not be
      reached from here. The user's installed fonts come before the bundled
      ones.
   4. **Per-page subsetting and embedding in the SVG.** Today's 47–81 MB
      pages come from whole `.ttc` embedding. rhwp embeds nothing, so its
      look depends on the viewer.

   Because some Korean organisations allow only one or two mandated fonts,
   font **identity** matters, not just layout:

   - **(e) No silent substitution.** Resolution order: an org/user font
     directory (a CLI `--font-dir` flag and an MCP server config entry), then
     installed system fonts, then the bundled substitutes.
   - **(f) Substitutions are visible.** The HTML viewer marks substituted
     text and lists each substitution, e.g. "휴먼명조 → Noto Serif CJK KR, N
     chars". The preview CLI/MCP result returns the same list. rhwp's chain
     per `<text>` and rdocx/rpptx's resolved faces are the inputs.
   - **(g) Font-identity sub-score F** in rule 5 (defined above, computed
     here for PDF candidates: 0.00 on all 30 LibreOffice files). A
     layout-perfect page in the wrong font then cannot pass. The engines must
     report the resolved face per run for F to be computed on SVG.
   - **(h) Optional font policy.** `hanji validate` checks a document's fonts
     against an allowed list.
2. **Re-export the five markup-view docx PDFs** (01, 02, 05, 06, 07) with
   Review → No Markup (the view rdocx renders as `Accepted`), and add that
   step to `baseline/CHECKLIST.md`. Then rescore docx.
3. **Engine bugs, reported upstream with these files.**
   - rpptx: 12's `duplicate p:attrName` rejection; run-level size on 17; bullet spacing on 10/15; a public caller-font render that keeps text direction.
   - rdocx: 04/05 pagination and spacing; cell spacing on 03; table autofit on 09; VML watermark and OfficeMath on 05.
   - rhwp: 28 pagination; 25 p5's dropped first line; 26's body offset.
4. **Drawing-object check.** A coarse box metric for shapes, pictures and
   fills (SVG elements vs pymupdf drawings), to cover what the word metric
   cannot see.
5. **xlsx grid check** as described above, once the HTML grid exists.
6. **Wire `layout_score.py` into a test** that renders the baseline with the
   hanji preview path. The rule-5 thresholds above stay **proposed** until
   then.
