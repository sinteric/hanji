# Corpus sources

Twenty-two files: twenty test decks from two open-source projects, and two
synthetic decks written for this project. All are committed; `fetch.sh`
re-downloads the twenty third-party files at pinned commits and checks their
SHA-256, and `make_korean.py` and `make_turns.py` regenerate the synthetic ones.

Licence note: as for the docx and hwpx corpora, each file is redistributed
under the licence of the repository it is test data in. Neither project gives
per-file provenance (several of Apache POI's are attachments to public bug
reports), so "licence" below means "distributed by that project under this
licence". Apache POI's decks named after web sites (crawled real-world files)
were left out, as were its fuzzing cases.

- **python-pptx** (MIT), [scanny/python-pptx](https://github.com/scanny/python-pptx)
  `features/steps/test_files/` at commit `278b47b1dedd5b46ee84c286e77cdfb0bf4594be`.
- **Apache POI** (Apache-2.0), [apache/poi](https://github.com/apache/poi)
  `test-data/slideshow/` at commit `93b6a820f4aaefd7b062615f843799947872cb24`.

| File | Source | Slides | What it exercises |
|---|---|---|---|
| act-props.pptm | python-pptx | 5 | macros (`vbaProject.bin`), embedded OLE objects, click actions (jump to a slide, run a program, a macro or an object's verb, play media, open a file, hyperlinks), a custom show, line breaks; 18 text boxes |
| ext-rels.pptx | python-pptx | 1 | an external relationship, a hyperlink |
| ph-populated-placeholders.pptx | python-pptx | 9 | placeholders filled with a picture, clip art, a table, a chart and SmartArt (slots holding an object); nine layouts |
| ph-unpopulated-placeholders.pptx | python-pptx | 9 | empty placeholders (left out of the text, kept) |
| prs-notes.pptx | python-pptx | 1 | a notes page |
| sld-notes.pptx | python-pptx | 2 | a slide with notes and one without |
| shp-access-ole-object.pptx | python-pptx | 1 | embedded OLE objects in `mc:AlternateContent` |
| shp-groupshape.pptx | python-pptx | 1 | a group shape |
| shp-shapes.pptx | python-pptx | 2 | a chart, SmartArt, a group, a table and pictures on one slide, with placeholders |
| tbl-cell.pptx | python-pptx | 3 | tables only (three slides the text cannot tell apart) |
| txt-text.pptx | python-pptx | 2 | a field, a hyperlink, line breaks, a table |
| txt-font-props.pptx | python-pptx | 5 | run formatting in text boxes: bold and italic set on and off, four underline styles, size, languages |
| SampleShow.pptx | Apache POI | 2 | title and bullet slides with notes, a run in another font |
| layouts.pptx | Apache POI | 10 | ten of the default layouts, a picture, a slide-number field, bulleted text boxes |
| with_japanese.pptx | Apache POI | 1 | Japanese text, an animation, a table, hyperlinks |
| shapes.pptx | Apache POI | 6 | a hyperlink to another slide, groups, tables, a picture |
| 45545_Comment.pptx | Apache POI | 11 | comments, transitions on every slide, animations, OLE objects, notes on every slide |
| SimpleMacro.pptm | Apache POI | 1 | macros |
| EmbeddedVideo.pptx | Apache POI | 1 | an embedded video and its playback animation |
| 60810.pptx | Apache POI | 28 | sections, SmartArt, media, 17 notes pages, 175 bulleted paragraphs |
| korean-deck.pptx | synthetic: `make_korean.py` (python-pptx 1.0.2, its default template) | 7 | Korean text with `lang="ko-KR"`: the DESIGN.md §5.3 example grown to seven slides — title, nested bullets with bold, two content, a text box, a table, a picture, a numbered list, a hyperlink and a coloured run, notes |
| turns-deck.pptx | synthetic: `make_turns.py` (python-pptx 1.0.2, its default template, XML then set by hand; CC0-1.0) | 2 | geometry as other applications store it, which GetPut leaves alone: rotations stored negative (`rot="-5400000"`, as Google Slides writes) and past a full turn, `flipH="true"`, a turned connector, a turned object in a group; a picture in `mc:AlternateContent` at the top of a slide (as a 3D model or ink is stored), its box not whole points |

## Korean files

No openly licensed Korean .pptx was found in the two projects searched
(python-pptx, Apache POI; LibreOffice's `sd/qa` decks were not searched).
`korean-deck.pptx` fills the gap
and is labelled synthetic wherever it is reported (CC0-1.0, written for this
project). Regenerating it gives the same content but not the same bytes (zip
timestamps); the committed copy is the one measured.

## Audit decks (`audit/`)

Five decks from the pptx canvas audit (2026-09-30), kept apart from the
twenty-one above so the geometry tests' counts stay theirs: designed real
decks and two synthetic modern ones, which the pptx tests run through GetPut,
the edit set and PutGet (`audit_decks_getput_putget_remainder`), and on
which the audit's coverage tally is measured. `fetch.sh` re-downloads the
three third-party ones and checks their SHA-256; `make_synth.py` regenerates
the synthetic ones. The synthetic decks pass the transitional-schema check
(`validation/ooxml-schema`). The pitch deck was regenerated on 2026-09-30:
its first version had an empty animation list on slide 3 and negative chart
axis ids, and PowerPoint repaired it on opening. Crawled decks the audit also read (from Apache POI's
crawled-file set) are not committed.

| File | Source | Licence | Slides | What it exercises |
|---|---|---|---|---|
| onlyoffice-sample.pptx | [ONLYOFFICE/document-templates](https://github.com/ONLYOFFICE/document-templates) `sample/sample.pptx` at `0a52fb76fa53fddf7d70da28878537cde3d29421` | Apache-2.0 | 7 | a designed template: 210 objects, nested groups, theme fills and outlines from `p:style`, gradients, vertical text |
| o09_Performance_typical.pptx | [dotnet/Open-XML-SDK](https://github.com/dotnet/Open-XML-SDK) `test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/o09_Performance_typical.pptx` at `431ab05cf160248cc3885a4a766026d4f8243792` | MIT | 11 | a "typical" Office deck: charts, tables, pictures (GIF, TIFF, WMF), pictures in groups, freeforms, a hand-drawn diagram |
| slide-section-test.pptx | [LibreOffice/core](https://github.com/LibreOffice/core) `sd/qa/unit/data/pptx/slide-section-test.pptx` at `7642fc49b32eaac1e1ab8639df991902fd9a48ff` | MPL-2.0 | 13 | sections; 150 shapes filled and outlined from the theme |
| synth-modern-pitch.pptx | synthetic: `make_synth.py` (python-pptx 1.0.2) | CC0-1.0 | 8 | a modern pitch deck on Blank layouts: full-bleed photo with an overlay, cropped and masked pictures (ellipse, rounded rectangle), rounded cards with shadows, freeform icons, nested groups, rotated shapes, gradients, connectors, a chart, a table, transitions, an animation |
| synth-korean-report.pptx | synthetic: `make_synth.py` (python-pptx 1.0.2) | CC0-1.0 | 6 | a Korean report deck (맑은 고딕): process chevrons, an org chart with elbow connectors, a masked photo, a doughnut chart, KPI cards |

Licences: ONLYOFFICE's document-templates repository is Apache-2.0, Open XML
SDK is MIT, LibreOffice core is MPL-2.0 (its test documents are
redistributed under it, as the docx corpus's LibreOffice files are).
