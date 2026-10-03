# Preview acceptance contract, v1

This harness inventories the 30 historical inputs, the 12-case v0.3.0 release
subset, and four deterministic synthetic inputs. It audits saved SVG/PNG/HTML
or runs an **existing** Hanji executable into a fresh local directory. Python
3.11's standard library is sufficient for regression checks. Native PDF text
ingest uses an already-installed `pdftotext`; missing tools block that layer.
It never installs software, invokes office applications, changes app settings,
approves baselines, opens a browser, or promotes a support profile.

## Contract and ownership

`corpus-v1.json` is versioned metadata: source bytes/hash, format, features,
origin, rights status, support profile, reference app/version/OS, reference font
identities, observed output dimensions, diagnostic expectations, embedded font
subset hashes, and output hashes. Full native font/PDF hashes are **null** when
unavailable. The original app-version file records PowerPoint/Excel as
`6.113.1`; SPIKE.md says `16.113.1`. Neither is silently corrected or confirmed.

Self-rendered regression hashes are explicitly labelled. They can detect a
changed renderer/font environment; they cannot prove Word, PowerPoint, Hancom
or Excel fidelity. Historical composite images in `diffs/` are inventory
evidence, not standalone native pages. Historical page counts are retained
separately from current regression counts, including DOCX 05's 3 versus 5 and
HWPX 28's 7 versus 6. Old Word markup exports 01/02/05/06/07 are unusable as
clean layout oracles and need No Markup exports.

Existing real inputs stay in their existing tracked locations. Their original
redistribution rights are unverified by this work; only metadata is added.
Local validation is authorized. No real input, private/native export, font
binary, or new renderer output is added to the PR. Synthetic content is newly
authored over Hanji's MIT-licensed blank parts at the pinned main commit;
`generate_synthetic.py` records that provenance. Generated inputs are ignored.
Do not publish local artifacts without separately establishing their rights.

Main preview implementation owns renderer/resolver/diagnostic/XLSX fixes.
This directory owns the acceptance contract. No Cargo or preview source change
is required to use it. `preview-acceptance.yml` independently tests this Python
contract and all source hashes; it does not build a renderer or assert fidelity.

## Layers and outcomes

Every layer reports `passed`, `failed`, `blocked`, or `unrun`. Exit codes are
0 for the requested checks passing, 1 for failure, and 2 for blocked coverage.
`--require-native` makes missing native evidence a nonzero result. A regression
pass can coexist with blocked native acceptance. `fidelity_promoted` is always
false: this report is evidence for an explicit later support-profile decision.

1. Input hash before/after capture and audit; outputs never overwrite evidence.
2. Complete consecutive page/slide enumeration, SVG physical dimensions,
   corresponding 96-DPI PNG dimensions, and all pages in HTML. A page/window
   omission fails independently of text or image similarity.
3. Owner-curated, page-specific critical text and exact cached/display values.
   Uncurated real cases are blocked, not credited with complete text coverage.
   Native PDFs add per-page normalized character-count coverage: missing text
   cannot be outweighed by whitespace or an aggregate image score.
4. Diagnostic allow/require patterns and reported missing characters. New or
   disappearing diagnostics require review. Every emitted character is also
   checked against its actual embedded SFNT cmap; missing subset characters
   fail even if `cells[].display` or the JSON glyph report is complete.
   `font_coverage` retains that codepoint/advance contract. Independently,
   `font_glyph_evidence` rejects embedded generic LastResort faces, empty mapped
   outlines, invalid TrueType data and letter/digit outlines duplicating
   `.notdef` tofu. A clean CLI report and nonzero cmap cannot rescue these.
5. SVG IDs/fragment resolution and document-wide HTML ID uniqueness. Critical
   text advance boxes, declared bounds, rectangular clip origins, and declared
   pairs of non-overlapping semantic regions are checked independently.
6. Owner-provided native source/app/export/font/output hashes and manual visual
   review of the **exact candidate hashes**. PDF text is re-extracted from its
   verified file during audit; editable sidecar text is not trusted as a PDF
   oracle. Locally generated opacity overlays are unapproved review aids.

Geometry coverage is deliberately bounded. Fonts use cmap/hmtx advances, not
glyph ink outlines. A font-size envelope crossing a clip while its origins
remain inside is **blocked ink verification**, not a proven clipping failure.
Complex clips, arbitrary drawing-path collisions, shaping clusters, `tspan`
positioning and non-supported transforms require further verification. This
does not certify general picture/table/shape overlap or browser layout.
The glyph evidence check aligns with PR #70's OpenType
[`head.flags` bit 14](https://learn.microsoft.com/en-us/typography/opentype/spec/head):
generic codepoint-range symbols do not establish character support. It also
checks embedded family/full/PostScript names for LastResort, independently of
SVG aliases and JSON family claims. TrueType checks use
[`glyf`/`loca` records](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf),
simple contour points and bounded composite references. Empty positioning
components are allowed when another valid component draws the glyph. Tofu
comparison ignores hint bytes/padding and targets letters/digits; an authored
square or replacement symbol is not rejected solely for resembling `.notdef`.

Nonempty outlines are **structural drawing evidence**, not proof that arbitrary
contours have the correct character shape or that PNG/browser rasterization is
correct. CFF, bitmap/SVG glyph containers and inspection-budget exhaustion
remain blocked, not credited as verified ink. Native source/font provenance,
critical text/geometry and exact-candidate visual review are still required.
The additional glyph tests use tiny authored in-memory font/outline doubles,
not native references or fonts copied from installed applications.

The harness budget is 512 pages, 64 MiB per file, and 16 MiB per SVG; these are
tool budgets, not Hanji product caps. Capture refuses under 256 MiB free disk.
Glyph inspection is bounded to 4,096 simple contours, 16 composite levels and
128 components per glyph, with at most 4,096 outline record inspections per
character before blocking repeated reference work; these are harness budgets.

XLSX is one named, bounded cached-value **worksheet window**, not workbook
print pagination. A historical entire-workbook Excel PDF is not a matching
window reference. Cached values are never recalculated or external links
refreshed by this harness.

## Local commands

Run from the repository root. Use a new output name for every run.

```sh
python3 -m unittest discover -s prototype/preview/acceptance -v
python3 prototype/preview/acceptance/generate_synthetic.py \
  --out prototype/preview/acceptance/synthetic-inputs
python3 prototype/preview/acceptance/acceptance.py inventory
python3 prototype/preview/acceptance/acceptance.py capture \
  --hanji /path/to/existing/hanji --fixture synthetic-docx-v1 \
  --fixture synthetic-pptx-v1 --fixture synthetic-hwpx-v1 \
  --fixture synthetic-xlsx-v1 --out /tmp/hanji-capture-001
python3 prototype/preview/acceptance/acceptance.py audit \
  --artifacts /tmp/hanji-capture-001 --fixture synthetic-docx-v1 \
  --fixture synthetic-pptx-v1 --fixture synthetic-hwpx-v1 \
  --fixture synthetic-xlsx-v1 --out /tmp/hanji-audit-001.json
```

`audit --artifacts DIR` also accepts the prior release acceptance tree with
`<source-stem>/{svg,png,html}/result.json`. It remaps saved absolute filenames to
the supplied artifact root; it cannot follow arbitrary result-file paths.
`--hash-goldens` requests exact comparison to the observed v0.3.0 output hashes.
Leave it off when evaluating a deliberately changed renderer or font set;
semantic failures still fail. Changing golden metadata requires a reviewed diff,
explicitly preserving the distinction from native references.

## Native reference intake

Start with `reference-v1.example.json`. Place owner-authorized exports and fonts
outside Git, in a directory named after the fixture ID. Hash actual original
font bytes (including collection face identity in owner notes); subset hashes
and family names do not substitute for original font identities. Record exact
app version/build/OS and confirm it. Do not update source fields/links, accept a
repair-and-save prompt, or save back to the input. Record any repair prompt.

```sh
python3 prototype/preview/acceptance/acceptance.py ingest-native \
  --metadata /private/local/fixture/owner-metadata.json \
  --root /private/local/fixture --out /private/local/fixture/reference.json
python3 prototype/preview/acceptance/acceptance.py audit \
  --artifacts /tmp/hanji-capture-002 --references /private/local \
  --fixture synthetic-docx-v1 --require-native \
  --overlays /tmp/hanji-overlays-002 --out /tmp/hanji-audit-002.json
```

DOCX uses `no-markup`; PPTX `full-slides`; HWPX `document-pages`. XLSX uses
`worksheet-window`, with exact `sheet`/`range`, owner-verified dimensions/text
and cached-cell sidecar. Native PNGs need calibrated page/window geometry;
do not stretch or crop evidence to conceal mismatches. PDF image-only pages
need owner transcription/critical assertions; no OCR or fabricated text is
introduced. Missing extraction/reference/font data remains blocked. An owner
can subsequently supply `visual_review.decision`, reviewer identity and the
exact `candidate_output_sha256` map after reviewing the local evidence. Intake
and overlay creation never fill that approval.

## Priority missing exports

For the exact source hashes in `corpus-v1.json`, supply these fresh exports,
their per-page/window 96-DPI PNGs, original font hashes and confirmed app builds:

- **Word, No Markup PDF:** 01, 03, 04, 05, 08 (filenames from baseline CHECKLIST.md).
- **PowerPoint, full-slide PDF:** 12 and 14, including 14 slide 6's chart legend.
- **Hancom, document PDF:** 24, 26, 28 and 30, including every table/footnote page.
- **Excel, native worksheet viewport:** 18, `매출!A1:H20`, with cached/display
  cell values and calibrated viewport bounds. An entire-workbook print PDF
  cannot satisfy this contract.

Also export the four generated synthetic inputs in their respective native
apps. Later extend to the remaining 18 historical cases after provenance and
critical assertions are reviewed. No available native app export is invented
or reattempted through the earlier timed-out Word automation.
