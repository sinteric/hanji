# Local validation, 2026-10-03

Base: `e2182b04959008d2c2ef9d478dd0e5e68f28ef67` (main).
This is harness validation against saved v0.3.0 evidence, not acceptance of a
new renderer or proof of native Office/Hancom fidelity.

- **32 adversarial unit tests passed** using Python 3.11 and its standard
  library. Native PDF/font/image test doubles are explicitly labelled and are
  never entered into the corpus as genuine reference evidence.
- **34 of 34 source hashes matched:** 30 historical inputs, including the
  12-case release subset, plus four synthetic packages. Repeating synthetic
  generation in a fresh directory produced the same four hashes and sizes.
- An existing `hanji 0.3.0` executable with SHA-256
  `0648aa98fb96d5cc7910c1612386f86a30f46a9a831968bfbb2f6d275b696504`
  captured the four synthetic inputs in SVG, PNG and HTML: **12 successful
  invocations, with matching before/after input hashes**. No build or install
  was needed. The capture contains diagnostic reports and binary identity.
- Synthetic audit: **DOCX and PPTX passed regression checks; HWPX blocked
  conservative clip-envelope verification; XLSX failed embedded-font coverage**.
  HWPX's glyph origins are inside the rectangular clip; the conservative
  font-size envelope is insufficient evidence of actual ink clipping.
  XLSX's embedded cell font maps only SPACE and lacks the ten reported drawn
  characters. Complete `cells[].display`/cached-value JSON does not rescue
  missing rendered glyphs. No renderer fix is included here.
- All **12 saved release cases matched their recorded SVG/PNG/HTML hashes**.
  Four nevertheless failed the stronger drawn-font contract: DOCX 04
  (`U+10332/10339/1033A/1033F/10343/10344/3005/F0B7`), HWPX 24
  (`U+FF62/FF63/FF65`), and HWPX 26/28 (`U+2024/2027/274D/274F`). These
  missing glyphs are also reported by the saved CLI, rather than newly hidden
  by it. The other eight release cases remain blocked by uncurated critical
  assertions or unsupported clip geometry. Stable hashes are not fidelity.
- **Native acceptance is blocked for every fixture.** No verified native
  PDFs/window references, original font hashes or explicit current-candidate
  visual approvals were available. Historical first-page metadata and six
  composite comparison PNGs do not substitute for those inputs.

Local audit reports are kept outside Git. Their SHA-256 values identify the
specific reports without publishing renderer outputs, fonts or native exports:

| Report | SHA-256 |
| --- | --- |
| inventory-v1.json | `3d8aa47015d5a4fbbaf9598347229da416c613a3a3b542db9115a12d31625459` |
| capture-v1/capture.json | `fd967ad86bcb0e0f5c08556008ce394fd94b3026d90a6259c8a9945f49d9557b` |
| synthetic-audit-final.json | `51832af814bbc818192b0be3456a49c1c6d686b69677b50b849df3d33db0c095` |
| release-audit-final.json | `e723cdb71c5911b6ae825823f905076d1664fac3777c63c4ddc38f5680806942` |

The release-archive audit covers all 34 metadata entries: four failures and
30 blocked entries, including the 18 historical cases and four synthetics
without outputs in that particular archive. Its 12 golden-hash layers pass.
These findings do not validate general ink/path/shape collision detection,
browser rendering, native application layout, recalculated formulas, or
support profiles beyond the explicitly recorded experimental/synthetic ones.
