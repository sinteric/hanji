# Corpus sources

Forty-six workbooks: twenty-eight test files of Apache POI, sixteen
Excel-written reference files of rust_xlsxwriter, and two synthetic Korean
workbooks written for this project (one as openpyxl writes it, one as
LibreOffice saves it again). All are committed; `fetch.sh` re-downloads the
forty-four third-party files at pinned commits and checks their SHA-256, and
`make_korean.py` regenerates the Korean ones.

Licence note: as for the other corpora, each file is redistributed under the
licence of the repository it is test data in. Neither project gives per-file
provenance (many of Apache POI's are attachments to public bug reports), so
"licence" below means "distributed by that project under this licence". Apache
POI's files named after web sites (crawled real-world files) and its fuzzing
cases were left out.

- **Apache POI** (Apache-2.0), [apache/poi](https://github.com/apache/poi)
  `test-data/spreadsheet/` at commit `93b6a820f4aaefd7b062615f843799947872cb24`
  (the same commit as the pptx corpus). One file is renamed:
  `54084 - Greek - beyond BMP.xlsx` → `54084-Greek-beyond-BMP.xlsx`.
- **rust_xlsxwriter** (MIT OR Apache-2.0),
  [jmcnamara/rust_xlsxwriter](https://github.com/jmcnamara/rust_xlsxwriter)
  `tests/input/` at commit `72dae65fc232bd0c5c66ccb03a2b0359f2cf2ea5`. These
  are the files Excel itself saved, which the project compares its output
  against; they are committed here with an `rx_` prefix.

| File | Source | Sheets | What it exercises |
|---|---|---|---|
| 50867_with_table.xlsx | Apache POI | 3 | a table; empty sheets |
| 54084-Greek-beyond-BMP.xlsx | Apache POI | 3 | text outside the Basic Multilingual Plane |
| 57893-many-merges.xlsx | Apache POI | 1 | 50,000 merged ranges (850 KB): range entries at scale |
| DataValidations-49244.xlsx | Apache POI | 1 | 52 data validations of every kind, merged cells |
| DateFormatTests.xlsx | Apache POI | 2 | 45 formulas, date and time formats (the display check) |
| ExcelPivotTableSample.xlsx | Apache POI | 3 | a table feeding two pivot tables |
| ExcelTables.xlsx | Apache POI | 1 | a table with a calculated column written with A1 references |
| ExcelWithAttachments.xlsm | Apache POI | 3 | macros (`vbaProject.bin`), embedded OLE objects, WordArt, a group, notes, hyperlinks, Cyrillic and Uzbek text |
| GeneralFormatTests.xlsx | Apache POI | 2 | 72 formulas; the General format (the display check) |
| InlineStrings.xlsx | Apache POI | 3 | inline strings (`t="inlineStr"`), formulas |
| SampleSS.xlsx | Apache POI | 3 | direct formatting, a formula |
| SimpleMacro.xlsm | Apache POI | 3 | macros |
| SimpleWithComments.xlsx | Apache POI | 3 | notes |
| StructuredReferences.xlsx | Apache POI | 2 | structured references to a table named `\_Prime.1` with a column `calc=#*#` (escapes) |
| Tables.xlsx | Apache POI | 12 | twelve sheets of forms: merged cells on every sheet, formulas |
| WithChart.xlsx | Apache POI | 3 | a chart |
| WithConditionalFormatting.xlsx | Apache POI | 3 | a conditional format |
| WithDrawing.xlsx | Apache POI | 3 | five pictures and a shape |
| WithTable.xlsx | Apache POI | 3 | a table |
| WithThreeCharts.xlsx | Apache POI | 3 | three charts, formulas |
| WithVariousData.xlsx | Apache POI | 3 | hyperlinks, notes, numbers and text |
| link-external-workbook-b.xlsx | Apache POI | 1 | a formula reading another workbook (`externalLink`, an external `externalLinkPath`) |
| shared_formulas.xlsx | Apache POI | 1 | 40 cells of shared formulas |
| sharedhyperlink.xlsx | Apache POI | 1 | one hyperlink over a range |
| simple-monthly-budget.xlsx | Apache POI | 1 | two tables, a chart, a shape, merged cells, a conditional format |
| table-sample.xlsx | Apache POI | 1 | a table with a totals row and calculated columns (`[#Totals]`, `[@[a]:[b]]`) |
| unicodeSheetName.xlsx | Apache POI | 1 | a sheet name in Chinese |
| xlookup.xlsx | Apache POI | 3 | XLOOKUP (`_xlfn.`) |
| rx_array_formula01.xlsx | rust_xlsxwriter | 1 | an array formula |
| rx_autofilter01.xlsx | rust_xlsxwriter | 1 | a sheet filter |
| rx_chart_bar01.xlsx | rust_xlsxwriter | 1 | a bar chart |
| rx_comment01.xlsx | rust_xlsxwriter | 1 | a note (VML) |
| rx_cond_format01.xlsx | rust_xlsxwriter | 1 | a conditional format |
| rx_data_validation01.xlsx | rust_xlsxwriter | 1 | a data validation |
| rx_defined_name01.xlsx | rust_xlsxwriter | 3 | defined names, print areas and titles |
| rx_dynamic_array01.xlsx | rust_xlsxwriter | 1 | a dynamic array formula (`cm`, metadata part) |
| rx_hyperlink01.xlsx | rust_xlsxwriter | 1 | a hyperlink |
| rx_image01.xlsx | rust_xlsxwriter | 1 | a picture |
| rx_macro01.xlsm | rust_xlsxwriter | 1 | macros |
| rx_merge_range01.xlsx | rust_xlsxwriter | 1 | a merged range |
| rx_rich_string01.xlsx | rust_xlsxwriter | 1 | a rich string (runs) |
| rx_table01.xlsx | rust_xlsxwriter | 1 | an empty table |
| rx_table05.xlsx | rust_xlsxwriter | 1 | a table on a sheet with a picture, a note and a hyperlink |
| rx_table14.xlsx | rust_xlsxwriter | 1 | a table with number formats that end in a space |
| korean-sales.xlsx | synthetic: `make_korean.py` (openpyxl 3.1.5) | 3 | Korean text: the DESIGN.md §5.4 example as a real workbook — a table with a calculated column (no cached values, as openpyxl writes it), leading-zero IDs and phone numbers as text, dates, a merged title, a data validation, a conditional format, a hyperlink, a note, a bar chart, SUMIFS over the table |
| korean-sales-lo.xlsx | synthetic: the same, saved by LibreOffice 24.2 | 3 | the same, as LibreOffice writes it: cached values computed, the calculated column's definition dropped, number formats re-escaped (`yyyy\-mm`) |

## Korean files

No openly licensed Korean .xlsx was found in the two projects searched
(Apache POI, rust_xlsxwriter; openpyxl's and LibreOffice's test files were not
searched). `korean-sales.xlsx` and `korean-sales-lo.xlsx` fill the gap and are
labelled synthetic wherever they are reported (CC0-1.0, written for this
project). The round-4 workbooks (`fluency/round4`) are built as xlsx at test
time from their seeds (`tests/round4.rs`), and cover the Korean table layouts
the fluency test used. `tests/large.rs` generates its 100,000-row workbook the
same way.
