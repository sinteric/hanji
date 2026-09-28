# Remainder anchoring prototype (DESIGN.md §10 item 3)

A small Python import/export for .docx that splits `word/document.xml` into model
text (a §5.2 subset) and remainder entries, then puts them back together. Four
ways of anchoring the remainder are compared on a 13-file corpus under scripted
edits. Findings and recommendation: **[REMAINDER.md](REMAINDER.md)**. Generated
numbers: [results/RESULTS.md](results/RESULTS.md).

This is not an engine and does not claim Word compatibility: Word was not tested.

## Rerun

```sh
pip install lxml                  # tested with lxml 6.1.3, Python 3.11
python3 run_all.py                # GetPut, 10 edits x 4 designs, PutGet (~1 min)
python3 run_all.py --soffice      # also converts every exported file to PDF with LibreOffice
corpus/fetch.sh                   # optional: re-download the 12 third-party files, check SHA-256
corpus/make_korean.sh             # optional: regenerate the synthetic Korean file (needs soffice)
```

`--soffice` needs `soffice` with the Writer component. On Ubuntu, install it with
`apt-get install libreoffice-writer`: `libreoffice-core` alone fails with "source
file could not be loaded".

## Layout

| Path | What |
|---|---|
| `hanji_rem/ooxml.py` | package read/write (other parts copied byte for byte), C14N canonicalisation, fingerprints |
| `hanji_rem/model.py` | model text: blocks, serialiser, parser (`#`, `<div style>`, `**`/`*`, `<br/>`, pipe tables with `^^`/`\|\|`, `<keep/>`) |
| `hanji_rem/importer.py` | document.xml → blocks + remainder entries (kinds are documented at the top of the file) |
| `hanji_rem/exporter.py` | blocks + placed entries → document.xml |
| `hanji_rem/anchoring.py` | block alignment, designs A / A-strict / B / C, and the oracle |
| `hanji_rem/edits.py` | scripted edits E1–E9 (each picks its own target in the file) |
| `hanji_rem/compose.py` | E10: all edits in one revision, with the oracle composed step by step |
| `hanji_rem/check.py` | outcome scoring (landed / orphaned / lost), XML diff for GetPut |
| `run_all.py` | runs everything and writes `results/` |
| `corpus/` | the 13 .docx, `SOURCES.md` (source URL and licence per file), `fetch.sh`, the Korean source `korean-report.fodt` |
| `results/RESULTS.md`, `results/results.json` | generated numbers |
| `results/model/*.md` | the model text of every corpus file |
| `results/remainder/*.jsonl` | the stored remainder of two files, as a sample of the storage shape (the run writes all 13; the rest are git-ignored) |

`results/out/` (every exported .docx, 35 MB) is git-ignored; a rerun regenerates it.
