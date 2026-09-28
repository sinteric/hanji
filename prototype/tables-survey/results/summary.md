# Table-shape survey: generated summary

Per top-level table (primary category; nested tables counted with their outer table):

| Category | Korean hwpx | English docx | All |
|---|---|---|---|
| nested | 8 (1.0%) | 27 (5.3%) | 35 (2.6%) |
| row-cc | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| grid | 0 (0.0%) | 13 (2.6%) | 13 (1.0%) |
| other | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| multipara | 284 (34.3%) | 175 (34.6%) | 459 (34.4%) |
| merged | 74 (8.9%) | 49 (9.7%) | 123 (9.2%) |
| plain | 463 (55.9%) | 242 (47.8%) | 705 (52.8%) |
| **tables** | 829 | 506 | 1335 |

Per document (a document counts once for every category it has at least one table of):

| Documents with | Korean hwpx | English docx | All |
|---|---|---|---|
| any table | 94 (100.0%) | 81 (48.5%) | 175 (67.0%) |
| nested | 4 (4.3%) | 5 (3.0%) | 9 (3.4%) |
| row-cc | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| grid | 0 (0.0%) | 8 (4.8%) | 8 (3.1%) |
| other | 0 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| multipara | 83 (88.3%) | 59 (35.3%) | 142 (54.4%) |
| merged | 39 (41.5%) | 15 (9.0%) | 54 (20.7%) |
| plain | 94 (100.0%) | 44 (26.3%) | 138 (52.9%) |
| an unsupported shape (nested, row-cc, grid, other) | 4 (4.3%) | 12 (7.2%) | 16 (6.1%) |
| an unsupported shape, among documents with tables | 4 (4.3%) | 12 (14.8%) | 16 (9.1%) |
| **documents** | 94 | 167 | 261 |

Every flag (a table can have several):

| Flag | Korean hwpx | English docx | All |
|---|---|---|---|
| grid:gridAfter | 0 | 12 | 12 |
| grid:gridBefore | 0 | 3 | 3 |
| info:anchor paragraph has text | 4 | 0 | 4 |
| info:anchor paragraph holds another table | 28 | 0 | 28 |
| info:field in cell | 7 | 0 | 7 |
| marker:cell-level | 0 | 72 | 72 |
| marker:row-level | 0 | 86 | 86 |
| marker:table-level | 0 | 79 | 79 |
| merged | 209 | 118 | 327 |
| multipara | 291 | 187 | 478 |
| nested | 8 | 27 | 35 |
| row-cc:sdt wraps cells | 0 | 1 | 1 |

Contexts: ('hwpx', 'body') 827, ('hwpx', 'textbox') 2, ('docx', 'body') 506
Inner (nested) tables: Korean hwpx 10, English docx 93
Prototype classifier (docx): 506 of 506 tables agree on pipe-table-or-not under the prototype's rules (multi-paragraph counted as not; markers as not).
Prototype first reasons (docx): (pipe table) 214, cell with 2 blocks 111, table-level bookmarkStart 77, cell with 3 blocks 29, cell holds tbl 26, cell with 4 blocks 13, gridBefore/gridAfter 7, cell with 5 blocks 7, cell with 9 blocks 4, cell with 7 blocks 4, cell with 10 blocks 3, cell with 6 blocks 2, cell with 13 blocks 2, table-level bookmarkEnd 2, row-level bookmarkStart 1, cell with 24 blocks 1, cell with 8 blocks 1, row-level sdt 1, cell with 12 blocks 1
