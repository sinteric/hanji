# Corpus sources

Sixteen .hwpx files, all from the sample files of
[rhwp](https://github.com/edwardkim/rhwp) at commit
`680111ec7bea2fe11110de18c3676ba5a1cf7847` (v0.8.6, MIT), the engine
DESIGN.md §7 names for hwp/hwpx. All are committed; `fetch.sh` re-downloads
them at that commit and checks their SHA-256. One file is renamed (its
upstream name is Korean, with spaces).

Licence note: as for the docx corpus, each file is redistributed under the
licence of the repository it is test data in (rhwp: MIT). rhwp gives no
per-file provenance. From the files' own metadata and content, they are of
two kinds:

- **Test documents made for rhwp**, authored by its contributors (creator
  `edward`, `rhwp`, `planet` in `Contents/content.hpf`).
- **Public-sector documents**: two ministry press releases, a ministry work
  report, a government application form and three official-letter forms.
  Korean ministry press releases are published under 공공누리 제1유형 (KOGL
  Type 1, attribution) per korea.kr, which the table survey
  (prototype/tables-survey) also relied on. The forms' personal details are
  masked (`****`) in the files as rhwp ships them. Logos and pictures in them
  belong to their publishers.

Not used: rhwp's national exam papers (`exam_*`: third-party copyright),
its Seoul 정보소통광장 documents (`opengov/`: real names), and files whose
origin is unclear (a publisher's sample form, private notices).

| File | Upstream (`samples/hwpx/`) | Kind | What it exercises |
|---|---|---|---|
| basic-table-01.hwpx | same | rhwp test | a 3×4 table alone in its paragraph, empty paragraph |
| eq-002.hwpx | same | rhwp test | equations (`hp:equation`), tabs, line breaks |
| fdi-2025q2.hwpx | `2025년 2분기 해외직접투자 (최종).hwpx` | press release, 기획재정부 (Ministry of Economy and Finance), 2025-09-19 | 2 sections; 26 tables with merged and multi-paragraph cells, incl. two contact boxes side by side in one paragraph; bold runs; spaces-only paragraphs; logo picture |
| field-multipara-clickhere.hwpx | same | official letter form (소방서 공문), masked | click-here fields (누름틀) spanning paragraphs; a nested table; pictures |
| footnote-01.hwpx | same | rhwp test | footnotes; 35 bullet and numbered paragraphs (list items); tables |
| footnote-tbox-01.hwpx | same | rhwp test | a footnote inside a text box |
| form-01.hwpx | same | rhwp test | form controls (button, check box, radio, combo box, edit); click-here fields; scripts (`Scripts/`) |
| hcar-001.hwpx | same | government form: 2026 전기자동차 구매 지원신청서 (EV purchase subsidy application, 별지 제1호 서식) | 3 sections; tables side by side; nested tables; merged cells |
| hy-002.hwpx | same | press release, 해양수산부 (Ministry of Oceans and Fisheries), 2026-05-07 | tables, a multi-paragraph cell; pictures and shapes |
| issue1535_coanchored_float_exclusion.hwpx | same | rhwp test | three tables sharing a paragraph with its text |
| issue1948_cross_para_fieldend.hwpx | same | official report form (소방서), masked | a field whose end is in a later paragraph; a nested table |
| landscape-001.hwpx | same | rhwp test | a landscape section; one large table |
| mel-001.hwpx | same | work report, 고용노동부 (Ministry of Employment and Labor), 2025-12-11; the tracked-changes spike's sample | 1,717 paragraphs; 44 tables incl. a 44-column organisation chart; page breaks; tabs |
| para-001.hwpx | same | rhwp test | paragraph shapes and character shapes |
| table-text.hwpx | same | rhwp test | a table with text; scripts |
| tb-img-03.hwpx | same | rhwp test | a picture in a table cell |

No openly licensed hwpx file with tracked changes was found (the
tracked-changes spike: 0 of rhwp's 543 hwpx files have any), so the §10.2
tests write the marks into corpus files themselves (`tests/engine.rs`,
`tests/rhwp.rs`), in the shape the spike took from the OWPML schema.

HWP 5.0 binaries are out of scope here; rhwp's samples include the `.hwp`
originals of several of these files.
