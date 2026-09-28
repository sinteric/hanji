# Corpus sources

Thirteen .docx files: twelve real test documents from four open-source projects,
and one synthetic Korean document written for this prototype. All are committed;
`fetch.sh` re-downloads the twelve third-party files at pinned commits and checks
their SHA-256, and `make_korean.sh` regenerates the Korean one.

Licence note: each file is redistributed under the licence of the repository it is
test data in. The upstream projects do not give per-file provenance for their test
documents (several are attachments to public bug reports), so "licence" below means
"distributed by that project under this licence".

| File | Source (pinned commit) | Licence | Unmodelled content it exercises |
|---|---|---|---|
| Bug54849.docx | [apache/poi `test-data/document/Bug54849.docx`](https://github.com/apache/poi/blob/942d95d85b15d0dfdb3bc9ba1b4f273f277757c8/test-data/document/Bug54849.docx) | Apache-2.0 | block and inline content controls, footnote, endnote, bookmark, header, row-level sdt in a table |
| WordWithAttachments.docx | [apache/poi `test-data/document/WordWithAttachments.docx`](https://github.com/apache/poi/blob/942d95d85b15d0dfdb3bc9ba1b4f273f277757c8/test-data/document/WordWithAttachments.docx) | Apache-2.0 | embedded OLE objects, VML, comments, bookmarks, header, table, Cyrillic text |
| delins.docx | [apache/poi `test-data/document/delins.docx`](https://github.com/apache/poi/blob/942d95d85b15d0dfdb3bc9ba1b4f273f277757c8/test-data/document/delins.docx) | Apache-2.0 | tracked insertions and deletions, fields, colour and size runs |
| form_footnotes.docx | [apache/poi `test-data/document/form_footnotes.docx`](https://github.com/apache/poi/blob/942d95d85b15d0dfdb3bc9ba1b4f273f277757c8/test-data/document/form_footnotes.docx) | Apache-2.0 | a form: legacy form fields, footnotes, drawings, nested-paragraph table cells |
| testWORD_2006ml.docx | [apache/tika `…/test-documents/testWORD_2006ml.docx`](https://github.com/apache/tika/blob/df85810303bdd51e4a7f5accbcfb0d659b922631/tika-parsers/tika-parsers-standard/tika-parsers-standard-modules/tika-parser-microsoft-module/src/test/resources/test-documents/testWORD_2006ml.docx) | Apache-2.0 | everything: TOC field, content controls, comments, tracked changes incl. moves, footnote, endnote, text box, OLE, math, hyperlinks, nested table |
| testWORD_various.docx | [apache/tika `…/test-documents/testWORD_various.docx`](https://github.com/apache/tika/blob/df85810303bdd51e4a7f5accbcfb0d659b922631/tika-parsers/tika-parsers-standard/tika-parsers-standard-modules/tika-parser-microsoft-module/src/test/resources/test-documents/testWORD_various.docx) | Apache-2.0 | direct formatting (fonts, sizes, colours, super/subscript), section breaks, footnote, field, drawing, header/footer, table |
| loadAndSave.docx | [plutext/docx4j `docx4j-core-tests/src/test/resources/loadAndSave.docx`](https://github.com/plutext/docx4j/blob/0e8e7633ef46012e0d3603339728b83ed21d5045/docx4j-core-tests/src/test/resources/loadAndSave.docx) | Apache-2.0 | content control, comment, footnote, endnote, math, drawings, fields, header |
| sample-docx.docx | [plutext/docx4j `docx4j-samples-docx4j/sample-docs/sample-docx.docx`](https://github.com/plutext/docx4j/blob/0e8e7633ef46012e0d3603339728b83ed21d5045/docx4j-samples-docx4j/sample-docs/sample-docx.docx) | Apache-2.0 | merged table cells (vMerge, gridSpan), tracked changes, drawings, header |
| docx4j-tables.docx (upstream name `tables.docx`) | [plutext/docx4j `docx4j-samples-docx4j/sample-docs/tables.docx`](https://github.com/plutext/docx4j/blob/0e8e7633ef46012e0d3603339728b83ed21d5045/docx4j-samples-docx4j/sample-docs/tables.docx) | Apache-2.0 | thirteen tables: merges, gridBefore/gridAfter, multi-paragraph cells, drawing |
| fdo76098.docx | [LibreOffice/core `sw/qa/extras/ooxmlexport/data/fdo76098.docx`](https://github.com/LibreOffice/core/blob/ccbc8791e125f7d508931bcdf33134b44d393d39/sw/qa/extras/ooxmlexport/data/fdo76098.docx) | MPL-2.0 | drawings (charts), section break in a paragraph, merged cells, fields, colour runs |
| tdf124637_sectionMargin.docx | [LibreOffice/core `sw/qa/extras/ooxmlexport/data/tdf124637_sectionMargin.docx`](https://github.com/LibreOffice/core/blob/ccbc8791e125f7d508931bcdf33134b44d393d39/sw/qa/extras/ooxmlexport/data/tdf124637_sectionMargin.docx) | MPL-2.0 | several sections (sectPr in paragraphs), footnote, content control, fields, headers |
| tdf154481.docx | [LibreOffice/core `sw/qa/extras/ooxmlexport/data/tdf154481.docx`](https://github.com/LibreOffice/core/blob/ccbc8791e125f7d508931bcdf33134b44d393d39/sw/qa/extras/ooxmlexport/data/tdf154481.docx) | MPL-2.0 | twelve comments, tracked changes, fields, content control, drawing, page breaks |
| korean-report.docx | synthetic: `korean-report.fodt` in this directory, converted by LibreOffice 24.2 (`make_korean.sh`) | CC0-1.0 (written for this project) | Korean text: headings, named styles, bold/italic, colour and size runs, bookmark, comment, footnote, tracked insertion, image, merged table (`^^` and `||`), landscape section, header/footer |

## Korean files

No openly licensed real-world Korean .docx was found. Searched: Apache POI, Apache
Tika, docx4j, LibreOffice core (`sw/qa`, `oox/qa`, `sd/qa`, `filter/qa`,
`writerperfect/qa`; 2,181 .docx), pandoc, python-docx, mammoth.js, docx2python,
koreader/test-data. The only files with Hangul were LibreOffice's
`tdf167721_chUnits*.docx` (a few Hangul characters in a numbering definition,
36 characters of English body text) and docx4j's `rFonts.docx` (an English spec
excerpt). `korean-report.docx` fills the gap and is labelled synthetic
everywhere it is reported. Korean public-sector documents (KOGL Type 1) are
mostly .hwp/.hwpx and were not searched further.

Regenerating `korean-report.docx` gives the same content but not the same bytes
(LibreOffice writes timestamps); the committed copy is the one measured.
