# hanji fluency test, round 6: direct formatting (DESIGN.md §5.1–§5.4, §10 item 10, proposed)

Today the text shows formatting by name only. Round 6 measures showing fill, borders, colour, size, alignment and
paragraph layout in the text, per kind of file: flow documents (docx, hwpx), Presentation shapes, Spreadsheet
ranges. The candidates, the vocabulary, the samples on real corpus files and the mapping to the four formats are in
[CANDIDATES.md](CANDIDATES.md). The results are in [RESULTS.md](RESULTS.md).

| part | candidates | seeds | tasks |
|---|---|---|---|
| A, flow documents | F1 effective always; F2 overrides + editable style section; F3 style section only | footnote-01.hwpx (report with a title banner, 2,708 chars today), mel-001.hwpx (ministry work report, 1,717 paragraphs, 32,752 chars today, 81–84k chars with formatting), korean-report.docx | 13 each: 4 reads, 2 single edits, many-place edits, restyle a set of headings, a style edit, a text-only edit that must keep formatting byte-identical, a new table with a header fill, a first-line indent on every body paragraph, a refusal |
| B, Presentations | F1 effective inline; F2o own values only (the pptx canvas audit's proposal) | the audit's synthetic modern pitch deck and Korean report deck (CC0), the ONLYOFFICE sample deck (210 objects) | 8 each: a read of an inherited value, a read of a direct one, a card fill to accent2, a 2pt navy outline, a 24pt coral span, connectors 3pt keeping their colour, a text-only edit, a refusal |

## The kit

- `vocab.py`: the vocabulary: parsing and canonical writing of every value (lengths, colours with theme names,
  lighter/darker, `*` and opacity, borders, line spacing), attribute lists.
- `extract.py`: reads a real docx or hwpx into the vocabulary: styles resolved through `basedOn`, docDefaults and
  theme; paragraphs, runs, table cells (table style, `tblBorders`, `tcBorders`, `shd`; hwpx `borderFill`, `charPr`,
  `paraPr` with the `HwpUnitChar` margins).
- `inline.py`: a paragraph's inline text: hanji's marks plus `[text]{…}` spans; writing spans so marks never cross
  them.
- `doc.py`: flow documents. `attach` puts a file's formatting on hanji's text of it (the engine's dump), block by
  block; `render(model, cand)` writes F1, F2 or F3 (lifting, merging, canonical order); `effective(text, cand,
  truth)` reads any of them back to effective formatting (F1's hidden style definitions and F3's hidden direct
  formatting come from the seed, F3's by character alignment as the remainder re-anchors it).
- `canvas.py`, `grid.py`: Presentations and workbooks, rendered on today's text (theme, `p:style`, layout and master
  text styles; `styles.xml` with named cell styles).
- `content.py`: part A's seeds and tasks, each with its targets, the check on effective formatting and a gold
  transformation of the seed model. `build.py` renders seeds in every candidate, asserts GetPut on the text, renders
  every gold answer and checks that it lands, and records what F3 cannot do (`unreachable`). `score.py` scores an
  answer file. `selftest.py`: gold, broken and alternative answers.
- `pptx_kit.py`: part B (build, gold, score) on the text.
- `measure.py`: the size of each candidate on every corpus file (data/measure.json).
- `run.py`: sends the prompts; `summarize.py`: the tables in RESULTS.md, re-scored from `runs/`.

Standard library only (Python 3.11); round 2's `apply_edits` is imported from `../round2/score.py`.

```
cd fluency/round6
python3 build.py && python3 pptx_kit.py build && python3 selftest.py   # ends with SELFTEST PASSED
python3 score.py r6-F2-2 runs/r6-F2-2/sonnet-first.json
python3 pptx_kit.py score r6p-F2o-3 runs/r6p-F2o-3/opus-first.json
python3 summarize.py
```

Today's text of each seed is in `data/today/` (the engine's `dump`, `dump_hwpx`, `dump_pptx` examples at the spike's
commit). `measure.py <dir>` needs the dump of every corpus file in `<dir>` as `<file>.txt`, made the same way:
`cargo run -p hanji-hwpx --example dump_hwpx -- <file> > <dir>/<file>.txt` (and `dump`, `dump_pptx`, `dump_xlsx`).
The two synthetic decks are in `data/decks/` (CC0, written for the pptx canvas audit). The ONLYOFFICE sample deck is
not committed: its text (`data/today/onlyoffice-sample.pptx.txt`) and seeds are, and `pptx_kit.py build` needs the
deck at `data/decks/onlyoffice-sample.pptx` to re-render them.

## Scoring

- **valid:** the edits apply (each `old` occurs exactly once) and the text parses under the candidate: known keys,
  values in the vocabulary, no brace where the candidate has none, known style names.
- **landed (part A):** read back to effective formatting, every target has the requested properties (colours in
  words by hue: light blue, red, navy; exact where the task gives a hex) and keeps every other property, its text and
  its runs; every other block reads exactly as in the seed; every line outside the targets is byte-identical (style
  lines may change where the task allows a style edit); the text-only task's line is the seed's line with only the
  text changed. A new table must follow the named paragraph with the header fill and no fill below.
- **landed (part B):** on the text, per object: the target has the requested values, every other attribute, text
  and run formatting of every object is unchanged, and lines outside the targets are byte-identical.
- **unreachable:** a task the candidate's text cannot do or answer: F3 has no direct formatting; F2o does not show
  an inherited value, so a read of it, or an edit that must keep it, cannot be done. A refusal, or a read answer
  saying the value is not shown, is the right answer and is counted apart. Under F1 a request to "change the style"
  has no style to change: a refusal there (`no_style_refused`) is also counted apart.
- **chars:** the answer text, or the sum of `old` + `new`, as in rounds 1–5.

## Runs

Each prompt went to Opus and Sonnet in a fresh `claude -p` conversation whose only tool was Read, started in an
empty directory holding only the prompt as `prompt.md`; the message told the subject to read that file, and only
that file, in parts if needed (round 4's lesson, used for every prompt so both models and all sizes arrive the same
way). Part A was sent as two prompts per unit with the same file (reads, single edits and the refusal; then the
many-place, style and new-table edits), so no answer outgrows its output. Answers are in
`runs/<unit>/<model>-first.json` (the two parts merged; each part in `<model>-a.json`, `-b.json`, token counts in
`*.usage.json`). The one fix round (part A, the units with an invalid answer) is `<model>-fix.json`, sent as a fresh
conversation with the prompt, the first answer and the validator's errors, which name the closest line of the file
and any character in it that looks like a space but is not.
