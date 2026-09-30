# Fluency test results, round 6

## Round 6 — 2026-09-30

**Subjects:** Claude Opus and Claude Sonnet, blind, each unit in a fresh conversation whose only tool was Read (the
prompt read from its own file, [README.md](README.md#runs)). Part A (flow documents): 3 candidates × 3 seeds × 13
tasks, 117 answers per model. Part B (Presentations): 2 candidates × 3 decks × 8 tasks, 48 answers per model. 330
answers in all. Prompts were 6,575–91,090 chars; the two mel-001 prompts per candidate were read in parts (18–20
turns). One fix round, for part A's invalid answers.

Re-score any answer with `python3 score.py <unit> <file>` or `python3 pptx_kit.py score <unit> <file>`;
`python3 summarize.py` reproduces every table here. The self-test passes: 159 gold answers land (or are the right
refusal where the candidate cannot do the task), 24/24 broken part A answers and 6/6 broken part B answers are
caught, 14/14 alternative answers pass (a style edit or direct edits for a style task, a row brace for a header
fill, another light blue, a darker red, a navy `#000080`, another coral), and GetPut holds on the text for every seed.

### What round 6 decides

§10 item 10 (proposed): whether direct formatting is in the text, and how, per kind of file. The candidates, the
vocabulary and the samples are in [CANDIDATES.md](CANDIDATES.md).

| | F1 | F2 | F3 |
|---|---|---|---|
| flow documents | effective formatting on every paragraph, cell, run; one default line | style section + only what differs from the style | style section only |
| Presentations | effective formatting inline on every object (like `box`) | F2o: only what the object sets itself (the audit's proposal) | – |

### Part A, flow documents: landed

| model | candidate | reachable, first try | after one fix round | invalid, first try | cannot be done in the text: refused / attempted |
|---|---|---|---|---|---|
| Opus | F1 | 33 / 37 | 35 / 37 | 4 | 2 / 0 |
| Opus | F2 | 35 / 39 | 37 / 39 | 4 | – |
| Opus | F3 | 18 / 18 | 18 / 18 | 0 | 19 / **2** |
| Sonnet | F1 | 33 / 37 | 35 / 37 | 4 | 2 / 0 |
| Sonnet | F2 | 36 / 39 | 37 / 39 | 3 | – |
| Sonnet | F3 | 18 / 18 | 18 / 18 | 0 | 19 / **2** |

"Cannot be done": F3 cannot show or write direct formatting (21 of 39 tasks: every cell, run and one-paragraph
change, and the reads of them); F1 has no style to change, and both models refused "change the style 표가운데 / Heading
2" there, correctly (on footnote-01 both recoloured every 개요 3 paragraph instead). Per task (seeds 1 2 3; L landed,
x missed, r the right refusal, U a wrong answer where the text could not know):

| task | Opus F1 | Opus F2 | Opus F3 | Sonnet F1 | Sonnet F2 | Sonnet F3 |
|---|---|---|---|---|---|---|
| q1 read a banner's or note's fill | LLL | LLL | rrL | LLL | LLL | rrL |
| q2 read which cells have a thick / double bottom border, which runs are red | LLL | LLL | rrr | LLL | LLL | rrr |
| q3 read an indent or alignment that comes from a style | LLL | LLL | LLL | LLL | LLL | LLL |
| q4 read a size or first-line indent set on the paragraph or run | LLL | LLL | L**UU** | LLL | LLL | L**UU** |
| e1 banner fill to light blue | LxL | LxL | rrL | LxL | LxL | rrL |
| e2 banner bottom border red, width and style kept | LxL | LxL | rrr | LxL | LxL | rrr |
| e3 header rows bold with a grey fill (4–52 cells) | LxL | LxL | rrr | LxL | LxL | rrr |
| e4 restyle a set of headings | LLL | LLL | LrL | LLL | LLL | LrL |
| e5 change a style so all its paragraphs turn navy | Lrr | LLL | LLL | Lrr | LLL | LLL |
| e6 text only, formatting byte-identical | LLL | LLL | LLL | LLL | LLL | LLL |
| e7 new table with a header fill | LLL | LLL | rrr | LLL | LLL | rrr |
| e8 first-line indent 10pt on every body paragraph (up to 47) | LxL | LxL | Lrr | LxL | LLL | Lrr |
| e9 refusal (gradient, pattern) | LLL | LLL | LLL | LLL | LLL | LLL |

- **F1 and F2 tie on everything the text shows directly.** Every miss under F1 and F2 is on mel-001 and is the same
  miss under both: an `old` that did not match because the line holds characters that look like spaces (see
  failure 1). Nothing failed on the formatting syntax: no invalid key, value or brace in 156 F1/F2 answers.
- **Only F2 can edit a style**, and the style edit is the small answer: fn-e5 was 162 chars under F2 (one style
  line) and 3,616 (Opus) / 1,264 (Sonnet) under F1 (every paragraph); fn-e8, 328 against 2,860 / 1,946. On
  mel-001, where Hancom put every paragraph's formatting on the paragraph, F2 has no style to use and its answers
  are as long as F1's (e8: 5,340–5,578 against 5,026–5,196 chars).
- **The F2 lookup did not cost a read.** Five read tasks needed a style line (fn-q3, fn-q4, kr-q1, kr-q3, mel-q3 on
  the 84,442-char mel-001 file, where the 표가운데 line is 80 lines above the cell and the model read the file in
  parts): 10/10 under F2.
- **F3 hides, and hidden formatting is misread.** Asked for the first-line indent of "ㅇ (노동절 입법)" (−58.96pt,
  set on the paragraph) and the size of "15% 성장" (14pt, set on the run), both models answered from the default
  style (0pt, 12pt) under F3: 4 wrong answers no validator can catch. The other 19 unreachable tasks per model were
  refused with the right reason ("A cell's fill is direct formatting, which this file does not show").

### Part B, Presentations: landed

| model | candidate | reachable, first try | the value is not in the text: refused / attempted |
|---|---|---|---|
| Opus | F1 | 24 / 24 | – |
| Opus | F2o | 18 / 18 | 6 / 0 |
| Sonnet | F1 | 24 / 24 | – |
| Sonnet | F2o | 18 / 18 | 5 / **1** |

| task | Opus F1 | Opus F2o | Sonnet F1 | Sonnet F2o |
|---|---|---|---|---|
| q1 read an outline or fill the theme's shape style gives | LLL | rrr | LLL | rr**U** |
| q2 read which shapes are filled accent1 / accent2, which have a 1.5pt outline | LLL | LLL | LLL | LLL |
| e1 a card's fill to accent2 | LLL | LLL | LLL | LLL |
| e2 a 2pt navy outline | LLL | LLL | LLL | LLL |
| e3 a text span 24pt coral | LLL | LLL | LLL | LLL |
| e4 connectors 3pt (or twice as thick), keeping colour and style | LLL | rrr | LLL | rrr |
| e5 text only, formatting unchanged | LLL | LLL | LLL | LLL |
| e6 refusal (shadow, gradient) | LLL | LLL | LLL | LLL |

- F1 did all 48 on both models. F2o can do 36: its text does not show what a shape takes from the theme's
  `p:style` (the connectors' 2pt accent1 outline, the ONLYOFFICE shapes' accent1 fill), so a read of it and an edit
  that must keep it have no answer. Opus said so every time ("Connector 6 has no border written on its tag, so its
  outline comes from the theme's line style. This file doesn't show that style"). Sonnet did five times and once
  answered `fill=none` for a shape filled accent1: a plausible wrong fact.
- The three edits the audit asked for (fill to accent2, 2pt navy outline, 24pt coral span) landed 12/12 under both.
  `accent1*` was kept as written wherever a line was widened (ONLYOFFICE e4, F1).

### Fix rate after one validator error (part A)

| model | F1 | F2 | F3 |
|---|---|---|---|
| Opus | 2/4 | 2/4 | – (0 invalid) |
| Sonnet | 2/4 | 1/3 | – (0 invalid) |

Every invalid answer was an `old` that did not occur. The error named the closest line with its look-alike
characters escaped (` `); mel-e3 and mel-e8 were then fixed on both models. mel-e1 and mel-e2 stayed invalid:
the banner line also holds U+F076, a private-use symbol from a Hancom font, which the error did not name.

### Size

Answer chars, summed over each unit's tasks (F3's are small because it refuses half):

| model | candidate | footnote-01 | mel-001 | korean-report | all |
|---|---|---|---|---|---|
| Opus | F1 | 9,788 | 8,264 | 1,423 | 19,475 |
| Opus | F2 | 3,346 | 8,605 | 1,186 | 13,137 (−33%) |
| Opus | F3 | 1,660 | 1,602 | 1,260 | 4,522 |
| Sonnet | F1 | 6,662 | 8,002 | 1,373 | 16,037 |
| Sonnet | F2 | 3,291 | 14,488 | 1,318 | 19,097 (+19%) |
| Sonnet | F3 | 1,532 | 1,390 | 1,218 | 4,140 |

Sonnet's F2 total is larger for one answer: mel-e3 rewrote 13 schedule tables' whole header rows (7,386 chars; Opus
1,302). Part B: Opus F1 3,183 / F2o 2,976 (−7%), Sonnet 2,855 / 2,618 (−8%).

Input, the seed file in each candidate (today's text in brackets):

| seed | F1 | F2 | F3 |
|---|---|---|---|
| footnote-01.hwpx [2,708] | 6,235 (2.3×) | 4,664 (1.7×) | 3,552 (1.3×) |
| mel-001.hwpx [32,752] | 81,536 (2.5×) | 84,442 (2.6×) | 34,062 (1.0×) |
| korean-report.docx [721] | 978 (1.4×) | 1,058 (1.5×) | 980 (1.4×) |
| synth modern pitch deck | 8,053 | F2o 8,362 | – |
| synth Korean report deck | 4,691 | F2o 4,372 | – |
| ONLYOFFICE sample deck | 20,786 | F2o 20,444 | – |

On the whole corpus (CANDIDATES.md): hwpx F1 2.13×, F2 2.16×, F3 1.04×; docx 1.40×, 1.38×, 1.12×; pptx F1 1.27×,
F2o 1.14×; xlsx F1 2.48× (2.04× without one 40× outlier, median 1.00).

### Failure patterns

1. **Look-alike characters break exact `old` matching** (F1 and F2, both models, mel-001 e1, e2, e3, e8: all 15
   invalid answers). Korean government documents put U+2007 FIGURE SPACE after `□` and `ㅇ`, and Hancom symbol
   fonts put private-use characters (U+F076) in banners. They look like a space or nothing, the models wrote a space
   or nothing, and `old` did not occur. Naming the character in the error fixed U+2007; the error did not name
   U+F076. This is not a formatting matter: today's text has the same characters. The text should show such
   characters escaped, and the validator should name them (a follow-up for hanji-format).
2. **Hidden formatting read as the style's** (F3, both models, 4 answers): see part A.
3. **Hidden inherited value guessed** (F2o, Sonnet, 1 answer): `fill=none` for an accent1 fill from `p:style`.
4. **F1 cannot keep a document consistent by style** (by construction): a style edit under F1 is a refusal or an
   edit of every paragraph (4 refusals, 2 per-paragraph edits); nothing in the file then ties those paragraphs
   together.

Nothing failed on what round 6 was built to provoke, under F1 and F2: no CSS, no invented key, no `colour=`, no
colour name; spans, row braces and table lines were written as the guide shows; the text-only edits kept their
braces and spans byte-identical (12/12, including a span split `[**…**]{size=15pt}` on mel-001); no style edit
leaked into other styles, and the tempting edit of the default style for "every body paragraph" on korean-report was
not made (both models wrote direct first-line indents there under F2).

### Decisions, per kind

| kind | round 6 | proposed |
|---|---|---|
| flow documents (docx, hwpx) | F1 = F2 on every direct task (35 and 37 of 37–39 after the fix round, the same misses); only F2 edits a style (6/6, 10–20× smaller); F2's style lookups 10/10 on an 84k-char file; F3: 21/39 tasks impossible and 4 silent misreads | **F2**: named style + visible direct overrides + editable style section |
| Presentations (pptx) | F1 48/48; F2o 36/48 possible, 11 right refusals, 1 silent misread; F2o 7–8% smaller answers, input within 4% | **F1**: effective formatting inline on every object, like `box` |
| Spreadsheets (xlsx) | not measured by fluency; 4 of 43 corpus workbooks use a named cell style besides Normal | **F1** range lines (`<format range …/>`), written by a `format` range operation; a named style shown as `style="Name"` on its ranges |

- **Flow documents: F2**, as the owner's hypothesis. On what the text shows directly the two tie; on consistency
  only F2 can act, with the smallest answers; and the cost the owner feared for lookups did not appear: every
  style-line read landed, on the largest document too. What misled models was formatting that was *not in the text*
  (F3, F2o), not formatting written one place away. F2's size is F1's on hwpx (2.2×): Hancom files keep their
  formatting on the paragraph, so F2 shows it there as F1 does.
- **Presentations: F1.** Own-only (the audit's proposal) is the look-it-up-elsewhere option with nowhere to look:
  a quarter of the tasks cannot be done and one was answered wrong. F1's cost is small (1.27× against 1.14× on the
  corpus, the same answer size). This matches round 5's choice of A for geometry.
- **Spreadsheets: F1**, on design: a grid has no flow for styles to keep consistent, named cell styles are rare, and
  the range lines merge equal cells into rectangles.

### Limits

- Two Claude models, one run per prompt, three seeds per part; part A's seeds are two real hwpx files and one small
  real docx (no large Korean docx was in the corpora), part B's two synthetic decks and one real one.
- Formatting is read and written by the kit (`extract.py`, `doc.py`, `canvas.py`), not by hanji's engines, and not
  checked in Word, Hancom or PowerPoint. The kit's GetPut holds on the text for 28 of 29 flow corpus files.
- Part A's F1 is strong because hwpx formatting is direct; a large docx built on styles would give F2 more to show
  off, and F1 more to repeat. Row lifting, table lifting and run merging were fixed before the round; other noise
  rules (dropping a font equal to the document's commonest) were not tried.
- xlsx was not run; its design rests on the corpus and on part B's result.
- `chars` stands in for tokens, as before. The tokens read (cache included) were 212,841–762,699 per unit on
  mel-001, 27,465–37,523 on the small seeds.
