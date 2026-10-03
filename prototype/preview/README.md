# Preview fidelity spike (DESIGN.md §4 rule 5, §9)

Findings, numbers and the proposed rule-5 threshold: **[SPIKE.md](SPIKE.md)**. Per-candidate tables:
[results/summary.md](results/summary.md). Calibration: [results/calibration.json](results/calibration.json).

The checked-in scores and comparison pictures describe the original spike's
engine versions. The render CLIs now consume immutable fork revisions recorded
in their manifests and lockfiles. Rerunning them produces new candidate output;
it does not update those historical scores without the native reference PDFs.
Current production preview support and release validation are documented in
[PREVIEW.md](../../PREVIEW.md).

## Layout

| Path | What |
|---|---|
| `baseline/` | the 30 input files (hanji `0024cc3` exports of corpus files), `MANIFEST.json` (source, kind, edits, SHA-256), `CHECKLIST.md` (how the native PDFs were made), `VERSIONS.txt` and `checklist.csv` as returned with the PDFs, `edits/` |
| `layout_score.py` | the rule-5 layout metric (T, P, L, W, layout; F proposed) |
| `score.py` | round 1's per-page SSIM; its content-masked SSIM is the secondary signal |
| `calibrate.py` | self / blank / shift / font-substitution / glyph-only calibration → `results/calibration.json` |
| `report.py` | merges both scores → `results/summary.md`, `results/summary.json` |
| `raster.py`, `intake.py`, `make_diffs.py` | PDF → 96-DPI PNG; native PDF page counts and fonts; the six pictures in `diffs/` |
| `render.sh` | LibreOffice PDFs and the engine renders |
| `engines/{rdocx,rpptx,rhwp}` | the render CLIs (standalone crates, each with its own `[workspace]`; not part of the hanji workspace). `rdocx/src/svg.rs` and `rpptx/src/svg.rs` are a copy of rdocx 0.14.0's `src/svg.rs` (MIT OR Apache-2.0, provenance header at the top). `engines/baseline` is the resvg + serde_json binary used to measure each engine's binary-size cost |
| `fonts/fonts.conf`, `fonts/FONTS.md` | the fontconfig alias table and where each font comes from (no font files) |
| `results/<candidate>/` | `layout.json`, `layout_files.csv`, `layout_pages.csv` (this round), `score.json`, `score_*.csv` (round 1), `engine_results.jsonl` (per-file engine log) |

## Rerun

Local, git-ignored inputs (see `.gitignore`): the native PDFs are **not** in the repo.

```sh
# 1. the 30 PDFs exported by Word / PowerPoint / Excel / Hancom (baseline/CHECKLIST.md), any subdir
mkdir native && cp /path/to/NN-*.pdf native/
# 2. Python 3.11+: pymupdf 1.28, scikit-image 0.26, scipy, pillow, cairosvg 2.9, rapidfuzz 3.14
python3 -m venv venv && venv/bin/pip install pymupdf scikit-image scipy pillow cairosvg rapidfuzz
# 3. fonts into fonts/files/ (fonts/FONTS.md); LibreOffice 24.2 with libreoffice-writer/-calc/-impress
#    and libreoffice-h2orestart (only for the LibreOffice reference rows); Rust (rdocx/rpptx need 1.93+)
PY=venv/bin/python ./render.sh                    # lo-pdf/, lo-pdf-hwpx/, renders/<candidate>/ (~15 min cold)
for f in native/*.pdf; do venv/bin/python raster.py "$f" native-png "$(basename "$f" | cut -c1-2)"; done
for c in libreoffice libreoffice-h2orestart rdocx rpptx rhwp; do
  venv/bin/python score.py renders/$c --no-diffs && cp renders/$c/score*.json renders/$c/score_*.csv results/$c/
done
venv/bin/python layout_score.py lo-pdf        --out results/libreoffice
venv/bin/python layout_score.py lo-pdf-hwpx   --out results/libreoffice-h2orestart
for e in rdocx rpptx rhwp; do venv/bin/python layout_score.py renders/$e --out results/$e; done
venv/bin/python report.py                         # results/summary.md
FONTCONFIG_FILE=$PWD/fonts/fonts.conf venv/bin/python calibrate.py   # results/calibration.json
```

The font-substitution calibration also needs `fontsub/ref/` (the default LibreOffice PDFs of 01, 09, 10, 24)
and `fontsub/nanum/` (the same four converted with `FONTCONFIG_FILE=fonts/fonts-nanum.conf`); `calibrate.py`
skips it when they are missing.

The baseline inputs can be regenerated from hanji `0024cc3` and the office-kit edit lists in
`baseline/edits/`; `MANIFEST.json` gives each file's source and SHA-256.
