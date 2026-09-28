---
status: prototype results
date: 2026-09-28
answers: DESIGN.md §10 item 3 — the remainder's storage shape and anchor granularity
scope: Document type, docx only; Python + lxml, not an engine
tested with: 13 .docx (12 real test documents, 1 synthetic Korean); LibreOffice 24.2 headless; Word NOT tested
---

# Remainder storage and anchoring: prototype results

## The question

§4 says import moves everything the model does not represent into the
remainder, anchored to "the model element it belongs to (block, run range,
slide, shape, cell range)", and export puts every entry back at its anchor. §10
item 3 asks what the remainder looks like when stored, and how fine its anchors
must be, so that entries survive edits (rule 1) or are refused with a reason,
and are never dropped silently.

## What was built

The smallest import/export that exercises the remainder
([hanji_rem/](hanji_rem/) and [run_all.py](run_all.py), about 2,600 lines):

- **Import** `word/document.xml` → model text (a §5.2 subset) + remainder entries.
  The model text has headings (`#` for the file's own `heading N` styles),
  `<div style="Name">` for other paragraph styles, `**bold**`, `*italic*`, `<br/>`,
  GFM pipe tables with `^^`/`||` merges, and `<keep id kind summary/>` placeholders,
  both block and inline.
- **Export** model + remainder → `document.xml`. Every other package part is
  copied through byte for byte.
- **Scripted edits**: E1–E9 are applied one at a time to the model; E10 applies
  all of them in one revision. Each edit picks its own target in the file.
- **Checks**: GetPut, the outcome of every remainder entry after every edit, PutGet,
  well-formedness, and LibreOffice PDF conversion
  ([run_all.py](run_all.py), results in [results/RESULTS.md](results/RESULTS.md)).

Simplifications, deliberate because they do not change the anchoring question:
footnote references, comments, fields, content controls, tracked changes and
math are all placeholders here. §5 models some of them as `[^1]`, `<field>`
and `<ins>`/`<del>`, but they would anchor the same way. List numbering (`numPr`)
and character styles (`rStyle`) stay in the remainder. Headers, footers and
footnote text are not imported: those parts are copied through.

## Storage shape (common to every design)

One flat list of typed entries per document revision:

```
{id, kind, xml: [fragment, …], anchor: {block: [b] | [b, row] | [b, row, col], start, end, seq}}
```

| Kind | What it holds | Anchor |
|---|---|---|
| `keep` / `bkeep` | an object the model sees as `<keep/>`: drawing, OLE object, field, footnote/comment reference, tracked change, content control, math; or a whole block (block sdt, a table that cannot be a pipe table) | the placeholder itself: its id in the text |
| `ppr` | `w:p` attributes + `pPr` minus `pStyle`: spacing, numbering, **section breaks** (`sectPr`), paragraph-mark formatting | the paragraph |
| `run` | **direct formatting**: one entry per `w:r`, holding its attributes and its `rPr` minus `b`/`i` (colour, size, fonts, `lang`, `rStyle`, rsids) | character range `[start, end)` in the paragraph |
| `marker` / `rmarker` | zero-width elements: bookmark and comment-range starts/ends, permission ranges, proofing marks, `lastRenderedPageBreak` | a character position |
| `wrap` | inline wrappers whose text the model edits: hyperlink, smartTag, customXml | character range |
| `bmarker` | a zero-width element between blocks (a body-level bookmark) | before a block |
| `tbl` / `tr` / `tc` | table, row and cell properties (merges are regenerated from `^^`/`||`) | table / row / cell |
| `tail` | the body's final `sectPr` | the document |

- Offsets count characters in the paragraph's *anchor string*: its text, with
  each inline placeholder as one character. Bold and italic belong to the model
  (per character), so they are not part of the anchor.
- `seq` (document order at import) orders zero-width things that share a
  position, for example a `bookmarkEnd` inside a hyperlink versus just after it.
  Without it GetPut fails.
- Placeholders anchor themselves. Because the id is in the text, an edit
  cannot lose them: every design scored 0 lost over 128 inline and 30 block
  placeholders.
- Size: across the 13 files `document.xml` is 710 KB and the model text 30 KB.
  The remainder's XML is 941 KB when each fragment carries its own `xmlns`
  declarations, and 660 KB (0.93×) when the namespace map is stored once per
  document. Store it once.

## The designs compared

All four designs share the storage shape above. They also share block
alignment: difflib over block signatures, then detection of moved and restyled
blocks, then pairing by similarity inside each changed stretch and across the
document. They find placeholders by id, and they apply the same rules to a
deleted block. They differ only in how an offset-anchored entry (`run`,
`marker`, `wrap`) finds its place after its paragraph's text changes:

| Design | Anchor | After an edit |
|---|---|---|
| **A** block + offset | the block, and the offset it had | offset kept, clamped to the new length |
| **A-strict** | the block, and the offset it had | any offset entry in a changed block is refused (orphaned, with a reason) |
| **B** run range, block diff | character range in the block | character diff of the old and new paragraph text |
| **C** run range, document diff | character range in the block | blocks aligned unchanged keep their anchors; every other paragraph, old and new, goes into one document-level stream, diffed as a whole, so text can carry its entries into another paragraph |

**How the outcome is judged.** The exported file is re-imported, and each
entry is looked up by its XML fingerprint at its intended anchor. The oracle
states the intended anchor: B's placement rule, fed the true edit (the exact
span, and the true block and paragraph maps) instead of a diff. Its
conventions follow Word's editing behaviour:

- Replaced text takes the formatting of the first replaced character.
- Text inserted at a run boundary joins the run on its left.
- A range does not grow when text is inserted exactly at its edge.
- Bookmarks and comment ranges in a deleted paragraph move to the next block;
  they never vanish.
- Run formatting and paragraph properties go with deleted text, and the removal
  is reported.

Paragraph properties of the paragraphs a split or a merge touches have no
single right answer (which half keeps the paragraph mark?), so those entries are
excluded from scoring. Outcomes:

- *landed*: the entry is at its intended anchor, or it was removed together
  with its text and the removal was reported.
- *orphaned*: the design refused the entry with a reason (rule 1's refusal).
- *lost*: the entry is missing, or it sits at the wrong place in the exported
  file. Both are silent failures.

## Results

Corpus ([corpus/SOURCES.md](corpus/SOURCES.md)):

- 12 real test documents from Apache POI, Apache Tika, docx4j and LibreOffice
  (Apache-2.0 / MPL-2.0), and one synthetic Korean report (CC0; no openly
  licensed real Korean .docx was found).
- 518 blocks and 2,145 remainder entries, of which 1,740 hold something
  unmodelled. The other 405 are, for example, plain runs with no `rPr` and no
  attributes.
- 158 placeholders: 21 drawings, 19 content controls, 18 comments, 33 tracked
  changes (2 of them moves), 28 fields, 8 OLE objects and more.
- 29 tables: 17 are modelled as pipe tables and 12 are kept whole as a block
  placeholder.

**GetPut**: for 13 of 13 files, `document.xml` is canonically equal (C14N 2.0,
inter-element whitespace ignored) after the full path: import → model text →
parse → place → export. The other parts are copied byte for byte. Two bugs
found on the way were fixed:

- the exporter added `xml:space="preserve"` to `w:t` elements that had never
  had it;
- an empty `<w:pPr/>` changed its fingerprint.

**PutGet** holds for 107 of 107 exports under every design (placeholder ids are
normalised, because re-import numbers them in document order). **Validity**:
all 441 exported files (13 GetPut plus 107 × 4 designs) are well-formed. All
441 convert to PDF with LibreOffice 24.2 headless, and every GetPut export has
the same page count as its original. **Word was not tested**, and a clean
LibreOffice conversion does not show that Word would open a file without a
repair prompt (§9).

Remainder entries in the blocks each edit touches, over all files (the
untouched blocks landed 100% under every design):

| Design | landed | orphaned (refused, with reason) | lost (silent) |
|---|---|---|---|
| A — block + offset | 3,358 (92.2%) | 0 | **285 (7.8%)** |
| A-strict — block, refuse on change | 3,232 (88.7%) | **335 (9.2%)** | 76 (2.1%) |
| B — run range, block diff | 3,510 (96.3%) | 0 | 133 (3.7%) |
| C — run range, document diff | **3,600 (98.8%)** | 2 (0.1%) | **41 (1.1%)** |

Per edit (landed / orphaned / lost, touched blocks):

| Edit | A | A-strict | B | C |
|---|---|---|---|---|
| E1 change a figure in a paragraph with a comment/bookmark | 48 / 0 / 24 | 19 / 53 / 0 | 70 / 0 / 2 | 70 / 0 / 2 |
| E2 insert a paragraph before a drawing | 63 / 0 / 0 | 63 / 0 / 0 | 63 / 0 / 0 | 63 / 0 / 0 |
| E3 delete a paragraph holding a direct-formatted run | 50 / 0 / 0 | 50 / 0 / 0 | 50 / 0 / 0 | 50 / 0 / 0 |
| E4 move a section | 1,315 / 0 / 0 | 1,315 / 0 / 0 | 1,315 / 0 / 0 | 1,315 / 0 / 0 |
| E5 restyle a block | 78 / 0 / 0 | 78 / 0 / 0 | 78 / 0 / 0 | 78 / 0 / 0 |
| E6 edit a table cell next to a merged cell | 161 / 0 / 0 | 160 / 1 / 0 | 161 / 0 / 0 | 161 / 0 / 0 |
| E7 edit text across a formatting-run boundary | 35 / 0 / 20 | 14 / 41 / 0 | 54 / 0 / 1 | 54 / 0 / 1 |
| E8 split a paragraph (Enter in formatted text) | 11 / 0 / 54 | 5 / 60 / 0 | 33 / 0 / 32 | 64 / 1 / 0 |
| E9 join two paragraphs | 27 / 0 / 53 | 7 / 51 / 22 | 63 / 0 / 17 | 79 / 0 / 1 |
| E10 all of the above in one revision | 1,570 / 0 / 134 | 1,521 / 129 / 54 | 1,623 / 0 / 81 | 1,666 / 1 / 37 |

Losses by entry kind (all edits): `run` A 166, A-strict 30, B 66, C 6;
`marker` 88 / 27 / 43 / 16; `wrap` 11 / 1 / 4 / 0; `ppr` 18 in every design;
`keep`, `bkeep`, `tbl`, `tr` and `tc` 0 in every design. Some edits had no
target in some files, for example E6 in files without a modelled table. The
per-file tables in [results/RESULTS.md](results/RESULTS.md) list them.

## What broke, and why

1. **Block + offset (A) moves formatting silently.** When an edit changes the
   length of the text before a colour run, a bookmark or a comment range, the
   frozen offset now points at other text. E1 (`2023` → `20235`) shifts every
   later run and marker by one character. A split or merge (E8/E9) puts the
   offset in the wrong paragraph, or past its end. This is the "silently lost"
   failure rule 1 forbids, and nothing in A can detect it.
2. **Refusing on change (A-strict) is safe but refuses almost everything.** In
   real Word files nearly every run carries `rPr` (fonts, `lang`, rsids), so
   every text edit in such a paragraph is refused: 335 refusals. It also still
   loses 76 entries. When two paragraphs are joined, block alignment sees the
   second as deleted, so the design reports its formatting as "deleted with its
   text" while the text is still there, now unformatted.
3. **A diff inside one block (B) cannot follow text into another block.** When
   a paragraph is split or joined, the text that changes paragraphs loses its
   runs and markers (32 and 17 lost).
4. **A document-level diff over the changed blocks (C) fixes split and join**
   (0 and 1 lost). Three failure modes remain, and any text-diff anchoring
   shares them:
   - *Diff ambiguity at the edit boundary* (E1, E7: 3 lost). `2023` → `20235`
     diffs as "insert 5 after 2023", so a comment range that ended after the
     figure no longer covers the new last digit. `EDITED` replacing `h. I`
     aligns the `I` of `EDITED` with the old `I` and moves a run boundary by
     four characters.
     With the exact span the model wrote (§4: read-before-edit), the oracle is
     reached by construction.
   - *Identical blocks have no identity* (E10: 17 of the 18 `ppr` losses).
     Empty paragraphs, all `<div style="Normal"></div>` in the text, carry
     different `pPr` (spacing, rsids). When a rewrite moves some of them, the
     diff pairs the wrong ones and their properties swap. The same risk applies to any
     repeated line, and a section break stored in an empty paragraph's `pPr`
     could move with it.
   - *A short paragraph edited beyond recognition* (the 18th `ppr` loss):
     E7 turned `italic` into `iEDITEDc`, so alignment no longer paired the
     block, and its properties were reported as deleted.
   - *A wrapper whose ends land in different paragraphs* (a hyperlink cut by
     E8) cannot be placed. C refuses it with a reason (the 2 orphans). That is
     the correct rule-1 behaviour.
5. **Block-level operations were never the hard part.** Inserting, deleting,
   moving and restyling (E2–E5) landed 100% in every design, once block
   alignment detected moves and paired restyled blocks by content (added
   during this work, after the first run lost a moved table).

Findings about the format and the storage, independent of the design:

- **12 of 29 tables cannot be pipe tables** and fall back to one uneditable
  block placeholder: 9 have multi-paragraph cells, 2 use
  `gridBefore`/`gridAfter`, 1 nests a table, 1 has a row-level content control.
  §5.2 needs a form for multi-paragraph cells, or a table falls out of the
  model entirely.
- **Empty paragraphs have no Markdown form.** The prototype writes
  `<div style="Normal"></div>`. They are common in real files, and they are
  where identity fails (item 4).
- **rsid attributes are most of the remainder.** They make almost every run
  and paragraph an entry with a unique fingerprint. Word regenerates them, so
  dropping them would be harmless for Word, but it breaks canonical GetPut. The
  lens laws need a declared list of insignificant attributes if rsids are to be
  normalised.
- **Entries reference other parts.** A comment or footnote placeholder points
  into `comments.xml` or `footnotes.xml`, and a drawing points at a
  relationship. Deleting the placeholder leaves the other part dangling (the
  prototype copies parts through). Storage must record these cross-part edges,
  so that a deletion removes both sides or is refused.
- **Complex fields that span paragraphs** (a TOC) cannot be one placeholder;
  the prototype falls back to one placeholder per field part (12 of them), so
  the model sees three cryptic `<keep/>`s per field. A field-range entry kind
  is needed.
- **Serializer bug to avoid.** The prototype writes `**fox **`, with the space
  inside the closing marker. That is not CommonMark emphasis. A real
  serializer must move boundary spaces outside the markers.

## Recommendation for §10 item 3

Store the remainder as a flat list of typed entries per revision. Each entry
has a stable id, its XML fragments (with the namespace map stored once per
document) and an anchor at one of three granularities: **the placeholder id**
for objects the model sees; **the block** (paragraph, table, row, cell) for
block properties, section breaks included; and **a character range plus a
sequence number inside a paragraph** for direct formatting, bookmarks, comment
ranges and hyperlinks. Re-anchor by the exact edit span when the edit gives
one. For a whole-file rewrite, keep the anchors of blocks that align unchanged
and run one document-level character diff across all the changed blocks
(design C: 1.1% lost against 3.7% for a per-block diff and 7.8% for block
offsets). Refuse, with a reason, any entry whose two ends land in different
paragraphs, and any placement that has to choose among identical blocks or
pair a block the diff no longer recognises. Never
use block + offset anchors: they lose silently.

The remaining silent losses (diff ambiguity at the edit edge, duplicate blocks)
are the concrete form of the risk §4 names against SuperDoc. They need
exact-span edits, or a refusal when a diff alignment is not unique, before
rule 1 holds.

## Limits

- Python, not the real engine. Only `document.xml` is split. Headers, footers,
  footnote text and comment text are copied through, not modelled.
- 13 files, and they are feature-dense test documents rather than typical
  office files. The one Korean file is synthetic, generated by LibreOffice, not
  Word.
- The edits are scripted and their targets chosen automatically, one edit type
  per run except E10. Some edits had no target in some files. Human edits in
  Office (rule 7) and model-written rewrites were not tested.
- The oracle's conventions are this project's choice, modelled on Word's
  editing behaviour. A different convention would move some B/C "lost" entries
  to "landed", or the other way round.
- PutGet compares text with placeholder ids normalised.
- **Word was not tested.** Validity means well-formed XML and a clean
  LibreOffice 24.2 PDF conversion. Presentation and Spreadsheet anchors (shape,
  cell range) were not prototyped.
