You work with spreadsheet workbooks shown as text. Below are the documentation of the text form and of how changes are written, the workbook itself, and six tasks. Do each task on its own: every task starts from the workbook exactly as shown, not from the result of another task.

## Documentation

A workbook is shown as text, sheet by sheet. Each sheet is a `<sheet name="…">` … `</sheet>` block holding its tables. Each table first has a `<table name="…" range="…">` … `</table>` block. `range` is the table's area in A1 notation: its first row is the header row (the column names), so the first data row is the next sheet row, and the columns take the range's letters in order. In `range="A1:F153"` the first column is A and the sixth is F, and the data rows are 2 to 153. Each line of the block is one column: its name, its type, its number format and, for a formula column, its formula.

- Types: `text`, `number` or `date`. Every cell of a column holds a value of that type, or is empty.
- Number formats say how a value is displayed: `#,##0` (1234567 displays 1,234,567), `#,##0.0` and `0.0` (one decimal), `0` (a whole number), `0.0%` (0.923 displays 92.3%), `0000`, `00000` and `000000` (a whole number padded with leading zeros: 417 in `00000` displays 00417), `yyyy-mm` (the date 2026-03-01 displays 2026-03), `yyyy-mm-dd`, and `@` for text, displayed as it is.
- A formula column computes every row from one formula written with structured references: `[@매출]` is this row's 매출, and `Sales[매출]` is the whole 매출 column of the table Sales. Its values are computed, never typed.
- A row whose cells are all empty is a blank row; its formula cells are empty too. Tables often hold blank rows between blocks, and subtotal rows (소계) written like data rows.

### Cell data

After its `<table>` block, each table's cells follow in a `<data table="…">` … `</data>` block. It is a pipe table whose header row is `row` followed by the table's column names, and then one line per data row: every row of the table, in sheet order, blank rows included.

- The first cell of a line is the row's sheet row number.
- The other cells are the row's values, one per column in the column order, each displayed with its column's number format: 12,000,000 in `#,##0` is the number 12000000, and 00417 in `00000` is the number 417 shown with leading zeros.
- An empty cell has nothing between its pipes, and a blank row is a line of empty cells after its row number.
- Rows are never left out or merged: a table with data rows 2 to 153 has 152 lines, one per sheet row.
- A cell's address is its column's letter and the row number: the third column of the line starting `| 3 |` is cell C3.
- The tables of a sheet follow one another, each with its own two blocks.

Example (a table Sales at A1 with the columns 월, 지점 and 매출):

```
<data table="Sales">
| row | 월 | 지점 | 매출 |
|---|---|---|---|
| 2 | 2026-01 | 강남 | 12,000,000 |
| 3 | 2026-01 | 서초 | 9,500,000 |
| 4 |  |  |  |
| 5 | 2026-02 | 강남 | 11,200,000 |
</data>
```

Here row 4 is blank, and the 매출 of 서초 in 2026-01 is 9,500,000, in cell C3.

### Rules for every change

- Types are never guessed from how a value looks: IDs, codes and phone numbers are text, even when they hold only digits. When a column's type changes, its values are converted as they are displayed.
- A formula column's cells are computed from its formula on every row; they are never set one by one. Formulas use only structured references: `[@Column]` or `[@[Column name]]` for a cell of this row, `[@[First]:[Last]]` for this row's cells from one column to another (inside SUM), and `Table[Column]` for a whole column of a table. The functions are SUM, SUMIFS, COUNTIFS, AVERAGE, MIN, MAX, ROUND, ABS, IF and IFERROR. A1 references such as D2, $D$2, D2:D9 or 매출!D2 are not allowed in formulas.
- A value that starts with `=` is not accepted: a formula belongs to a formula column.
- Functions that fetch data (WEBSERVICE, FILTERXML, HYPERLINK and the like) are not allowed.
- Formatting is by type and number format only, from the formats listed above; colours, fonts, fills, borders and column widths cannot be set.
- No answer adds more than 50 rows in total: bulk rows come from an import, not from rows you type or generate.
- A table name is letters, digits, `_` and `.`, starting with a letter, and unique in the workbook. Tables on one sheet never overlap.

### Writing changes: range operations

Answer a write or edit task with `text`: a JSON list of operations, applied in order. There are exactly ten operations: objects with `"op"` and the keys shown, all required unless marked optional:

- `{"op": "set", "range": "Sheet1!B7", "values": [[1204]]}`: set cells. `range` is Sheet!Cell or Sheet!First:Last, `values` a list of rows of values in its shape; each cell is a table's data cell, not in a formula column.
- `{"op": "append_rows", "table": T, "rows": [{"월": "2026-04", "매출": 9800000}]}`: add rows after the last row. A row is {column: value}; columns left out are empty, and formula columns are always left out.
- `{"op": "insert_rows", "table": T, "before": 19, "rows": [...]}`: add rows before sheet row 19, a data row of T or the row after its last.
- `{"op": "delete_rows", "table": T, "rows": "18:19"}`: delete sheet rows 18 to 19 (`"18"` for one row).
- `{"op": "fill_formula", "table": T, "column": C, "formula": "=[@매출]-[@원가]"}`: make C a formula column; its cells are computed on every row.
- `{"op": "set_type", "table": T, "column": C, "type": "text", "format": "@"}`: change a column's type and format (format optional for text). Values are converted as displayed: 417 in format 00000 becomes the text "00417".
- `{"op": "add_column", "table": T, "column": {"name": C, "type": "text", "format": "@"}}`: add a column after the last; `"formula"` only for a formula column.
- `{"op": "sort", "table": T, "keys": [{"column": C, "order": "asc"}]}`: sort the rows; order is "asc" or "desc".
- `{"op": "add_table", "sheet": S, "name": N, "anchor": "H1", "columns": [{"name": C, "type": "number", "format": "#,##0"}], "rows": [...]}`: add a table with its header row at anchor; a formula column also has `"formula"`; `"rows"` optional, as in append_rows.
- `{"op": "add_sheet", "name": S}`: add an empty sheet at the end.

Values are JSON: text is a string, a number is a JSON number (1204, never "1,204"), a date is "YYYY-MM-DD" ("YYYY-MM" for the first of the month), and null is an empty cell. Row numbers are sheet rows at the time the operation runs, after the earlier ones.

Example (in the table Sales above, the 매출 of row 3 becomes 15,300,000, and a row is added):

```
[{"op": "set", "range": "매출!C3", "values": [[15300000]]},
 {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-03", "지점": "강남", "매출": 17200000}]}]
```

## The workbook: 지점 인사명부.xlsx

````
<sheet name="명부">

<table name="Roster" range="A1:H108">
| column | type | format | formula |
|---|---|---|---|
| 사번 | number | 000000 |  |
| 이름 | text | @ |  |
| 지점 | text | @ |  |
| 부서 | text | @ |  |
| 직급 | text | @ |  |
| 입사일 | date | yyyy-mm-dd |  |
| 휴대전화 | text | @ |  |
| 내선 | text | @ |  |
</table>

<data table="Roster">
| row | 사번 | 이름 | 지점 | 부서 | 직급 | 입사일 | 휴대전화 | 내선 |
|---|---|---|---|---|---|---|---|---|
| 2 | 034884 | 정은우 | 서울본점 | 영업 | 대리 | 2023-07-17 | 010-5924-7131 | 0371 |
| 3 | 092166 | 오은지 | 서울본점 | 영업 | 사원 | 2026-04-18 | 010-8604-2358 | 2677 |
| 4 | 007757 | 안태호 | 서울본점 | 영업 | 부장 | 2020-02-11 | 010-5943-1901 | 0398 |
| 5 | 052008 | 송우진 | 서울본점 | 영업 | 부장 | 2010-09-19 | 010-6059-5450 | 1816 |
| 6 | 065879 | 한선우 | 서울본점 | 영업 | 사원 | 2010-01-25 | 010-9011-0528 | 1141 |
| 7 | 052045 | 장보람 | 서울본점 | 영업 | 사원 | 2015-12-26 | 010-3455-1696 | 0617 |
| 8 | 003533 | 신서윤 | 서울본점 | 영업 | 대리 | 2012-05-02 | 010-9863-9126 | 0572 |
| 9 | 072904 | 홍하린 | 서울본점 | 운영 | 사원 | 2025-11-03 | 010-9699-9586 | 1077 |
| 10 | 066020 | 류정우 | 서울본점 | 운영 | 사원 | 2020-06-05 | 010-3456-6584 | 0972 |
| 11 | 062435 | 이민준 | 서울본점 | 운영 | 차장 | 2023-12-21 | 010-3440-2843 | 0437 |
| 12 | 006820 | 황유진 | 서울본점 | 운영 | 과장 | 2021-12-27 | 010-8382-4904 | 1950 |
| 13 | 065533 | 정현우 | 서울본점 | 운영 | 사원 | 2024-06-15 | 010-5528-6534 | 0568 |
| 14 | 044159 | 서성민 | 서울본점 | 운영 | 차장 | 2017-04-08 | 010-6695-2864 | 2048 |
| 15 | 048662 | 송주원 | 서울본점 | 지원 | 과장 | 2015-11-08 | 010-5043-5590 | 3883 |
| 16 | 042385 | 강가은 | 서울본점 | 지원 | 과장 | 2017-11-10 | 010-6704-7510 | 3621 |
| 17 | 017335 | 정선우 | 서울본점 | 지원 | 과장 | 2010-07-24 | 010-2175-9711 | 2766 |
| 18 | 071613 | 안종민 | 서울본점 | 지원 | 차장 | 2020-07-04 | 010-2730-3030 | 2311 |
| 19 |  |  |  |  |  |  |  |  |
| 20 | 097926 | 임도윤 | 부산 | 영업 | 과장 | 2015-09-25 | 010-5600-2996 | 0232 |
| 21 | 015548 | 오경수 | 부산 | 영업 | 사원 | 2009-01-03 | 010-7259-6428 | 0286 |
| 22 | 001395 | 신종민 | 부산 | 영업 | 사원 | 2026-03-17 | 010-8475-2034 | 3336 |
| 23 | 064477 | 김서연 | 부산 | 영업 | 사원 | 2011-03-26 | 010-3353-1356 | 3173 |
| 24 | 086105 | 홍건우 | 부산 | 영업 | 사원 | 2008-01-23 | 010-8588-7830 | 1882 |
| 25 | 002424 | 임가은 | 부산 | 영업 | 사원 | 2012-01-21 | 010-4880-1038 | 0118 |
| 26 | 013149 | 홍우진 | 부산 | 영업 | 사원 | 2017-04-26 | 010-2775-1556 | 2665 |
| 27 | 089984 | 권경수 | 부산 | 운영 | 사원 | 2021-01-23 | 010-5420-0629 | 1830 |
| 28 | 082742 | 홍보람 | 부산 | 운영 | 차장 | 2022-09-28 | 010-8571-6451 | 3802 |
| 29 | 009319 | 권민준 | 부산 | 운영 | 대리 | 2026-05-08 | 010-9705-2846 | 2745 |
| 30 | 057765 | 오수빈 | 부산 | 운영 | 사원 | 2014-04-25 | 010-8316-9254 | 1609 |
| 31 | 004397 | 송은우 | 부산 | 운영 | 차장 | 2018-01-08 | 010-7492-9789 | 3186 |
| 32 | 028666 | 이선우 | 부산 | 운영 | 차장 | 2024-04-02 | 010-6668-3891 | 3829 |
| 33 | 025920 | 송종민 | 부산 | 지원 | 과장 | 2012-01-28 | 010-6110-8126 | 2431 |
| 34 | 024631 | 조시우 | 부산 | 지원 | 과장 | 2009-10-15 | 010-8559-1097 | 0186 |
| 35 | 010741 | 홍하준 | 부산 | 지원 | 대리 | 2025-12-07 | 010-7465-6279 | 2960 |
| 36 | 081928 | 한가은 | 부산 | 지원 | 차장 | 2021-12-03 | 010-6224-2099 | 2673 |
| 37 |  |  |  |  |  |  |  |  |
| 38 | 010394 | 이지민 | 대구 | 영업 | 대리 | 2025-11-10 | 010-8511-8118 | 2355 |
| 39 | 041852 | 최동현 | 대구 | 영업 | 대리 | 2020-06-24 | 010-9059-2461 | 1824 |
| 40 | 091175 | 황재원 | 대구 | 영업 | 대리 | 2017-08-21 | 010-4947-8165 | 0119 |
| 41 | 092098 | 최서윤 | 대구 | 영업 | 부장 | 2021-01-05 | 010-7243-3976 | 0654 |
| 42 | 050701 | 안현우 | 대구 | 영업 | 대리 | 2019-03-28 | 010-4049-9809 | 3645 |
| 43 | 068328 | 김서연 | 대구 | 영업 | 사원 | 2012-10-21 | 010-9885-2634 | 1919 |
| 44 | 010607 | 최예은 | 대구 | 영업 | 대리 | 2012-05-12 | 010-3722-4012 | 0724 |
| 45 | 073559 | 임현우 | 대구 | 운영 | 과장 | 2008-05-20 | 010-6959-9826 | 3928 |
| 46 | 088563 | 김은지 | 대구 | 운영 | 대리 | 2015-09-07 | 010-3014-6889 | 2827 |
| 47 | 059752 | 박승현 | 대구 | 운영 | 대리 | 2009-11-21 | 010-3786-5870 | 0368 |
| 48 | 033401 | 송경수 | 대구 | 운영 | 사원 | 2013-04-15 | 010-8718-5961 | 3791 |
| 49 | 055220 | 장영호 | 대구 | 운영 | 사원 | 2009-11-21 | 010-7949-4016 | 1071 |
| 50 | 095396 | 서보람 | 대구 | 운영 | 대리 | 2015-07-09 | 010-4039-3432 | 0342 |
| 51 | 061380 | 황서윤 | 대구 | 지원 | 부장 | 2024-10-05 | 010-9576-0911 | 3799 |
| 52 | 019684 | 장혜진 | 대구 | 지원 | 사원 | 2020-09-21 | 010-5071-7166 | 0760 |
| 53 | 061692 | 조연우 | 대구 | 지원 | 과장 | 2008-08-13 | 010-6677-1333 | 1820 |
| 54 | 091717 | 김미경 | 대구 | 지원 | 사원 | 2022-10-04 | 010-4721-1918 | 1305 |
| 55 |  |  |  |  |  |  |  |  |
| 56 | 014531 | 장지민 | 광주 | 영업 | 사원 | 2014-01-12 | 010-5624-6273 | 2577 |
| 57 | 095370 | 오태호 | 광주 | 영업 | 사원 | 2008-01-25 | 010-6373-9833 | 0853 |
| 58 | 059501 | 윤하준 | 광주 | 영업 | 과장 | 2017-08-13 | 010-5263-0383 | 0319 |
| 59 | 037389 | 임윤서 | 광주 | 영업 | 사원 | 2019-12-11 | 010-6823-1213 | 0887 |
| 60 | 014326 | 윤연우 | 광주 | 영업 | 대리 | 2020-07-18 | 010-2263-6802 | 2671 |
| 61 | 015179 | 임선우 | 광주 | 영업 | 사원 | 2017-12-16 | 010-2913-1109 | 0494 |
| 62 | 075461 | 오준서 | 광주 | 영업 | 부장 | 2011-10-18 | 010-2029-1484 | 2655 |
| 63 | 092144 | 오지호 | 광주 | 운영 | 사원 | 2009-01-13 | 010-9735-1642 | 0473 |
| 64 | 081731 | 정수빈 | 광주 | 운영 | 대리 | 2025-09-18 | 010-6487-2143 | 0939 |
| 65 | 089512 | 윤준서 | 광주 | 운영 | 대리 | 2022-03-28 | 010-9161-0844 | 2374 |
| 66 | 064909 | 윤은우 | 광주 | 운영 | 사원 | 2022-11-05 | 010-3912-8167 | 3260 |
| 67 | 080655 | 한다은 | 광주 | 운영 | 대리 | 2018-08-09 | 010-3743-2771 | 3737 |
| 68 | 085719 | 류윤서 | 광주 | 운영 | 부장 | 2015-05-03 | 010-5130-5426 | 1378 |
| 69 | 019105 | 황지민 | 광주 | 지원 | 차장 | 2016-08-11 | 010-3036-4544 | 2780 |
| 70 | 009570 | 신영호 | 광주 | 지원 | 대리 | 2018-10-10 | 010-7011-8563 | 3948 |
| 71 | 037851 | 황우진 | 광주 | 지원 | 과장 | 2021-11-22 | 010-7534-0161 | 2320 |
| 72 | 003943 | 황태호 | 광주 | 지원 | 사원 | 2021-04-21 | 010-6911-1960 | 2056 |
| 73 |  |  |  |  |  |  |  |  |
| 74 | 014825 | 송정우 | 대전 | 영업 | 사원 | 2015-08-20 | 010-2685-1634 | 3695 |
| 75 | 087105 | 박소율 | 대전 | 영업 | 사원 | 2011-08-28 | 010-6148-4167 | 3816 |
| 76 | 094872 | 홍은우 | 대전 | 영업 | 사원 | 2008-12-03 | 010-8837-9585 | 0570 |
| 77 | 076544 | 안서현 | 대전 | 영업 | 차장 | 2026-08-06 | 010-6150-3598 | 0771 |
| 78 | 021857 | 조유진 | 대전 | 영업 | 대리 | 2013-11-06 | 010-7930-3158 | 2324 |
| 79 | 070896 | 장가은 | 대전 | 영업 | 과장 | 2009-10-03 | 010-6838-7246 | 3169 |
| 80 | 072605 | 황승현 | 대전 | 영업 | 사원 | 2008-09-12 | 010-8418-7373 | 2954 |
| 81 | 055703 | 이미경 | 대전 | 운영 | 사원 | 2020-08-08 | 010-7048-8449 | 2475 |
| 82 | 060777 | 정예준 | 대전 | 운영 | 대리 | 2017-08-24 | 010-6435-0719 | 0309 |
| 83 | 003104 | 강태윤 | 대전 | 운영 | 차장 | 2020-04-05 | 010-4582-2322 | 2872 |
| 84 | 041726 | 김하린 | 대전 | 운영 | 사원 | 2018-06-22 | 010-5606-5771 | 2511 |
| 85 | 060576 | 윤선우 | 대전 | 운영 | 사원 | 2015-04-16 | 010-7980-1261 | 1239 |
| 86 | 031176 | 신시우 | 대전 | 운영 | 차장 | 2018-10-22 | 010-3013-3730 | 0532 |
| 87 | 035379 | 윤다은 | 대전 | 지원 | 과장 | 2020-04-20 | 010-2253-9299 | 0820 |
| 88 | 043922 | 안지민 | 대전 | 지원 | 부장 | 2015-12-16 | 010-3220-4195 | 2694 |
| 89 | 054513 | 권지호 | 대전 | 지원 | 사원 | 2015-01-24 | 010-7683-0187 | 1731 |
| 90 | 077303 | 류지우 | 대전 | 지원 | 대리 | 2014-06-24 | 010-7649-0421 | 2813 |
| 91 |  |  |  |  |  |  |  |  |
| 92 | 094217 | 홍미경 | 인천 | 영업 | 부장 | 2014-08-02 | 010-9468-2268 | 2370 |
| 93 | 018592 | 송연우 | 인천 | 영업 | 과장 | 2020-06-22 | 010-3984-0030 | 3072 |
| 94 | 070964 | 권상훈 | 인천 | 영업 | 부장 | 2017-04-12 | 010-4871-5220 | 0838 |
| 95 | 072225 | 최하준 | 인천 | 영업 | 사원 | 2025-12-02 | 010-2575-4819 | 0905 |
| 96 | 069646 | 장현우 | 인천 | 영업 | 대리 | 2014-01-14 | 010-8067-3927 | 0563 |
| 97 | 010589 | 안준서 | 인천 | 영업 | 사원 | 2025-05-04 | 010-5700-6723 | 0892 |
| 98 | 054517 | 박나연 | 인천 | 영업 | 과장 | 2012-03-03 | 010-5913-4180 | 3158 |
| 99 | 032091 | 최민서 | 인천 | 운영 | 부장 | 2018-06-12 | 010-4605-9495 | 2644 |
| 100 | 043569 | 박준서 | 인천 | 운영 | 사원 | 2025-06-22 | 010-6613-6799 | 2227 |
| 101 | 050806 | 최현우 | 인천 | 운영 | 과장 | 2010-12-25 | 010-5328-1028 | 1586 |
| 102 | 063479 | 홍서연 | 인천 | 운영 | 사원 | 2014-05-13 | 010-2608-1236 | 0678 |
| 103 | 081816 | 조은우 | 인천 | 운영 | 대리 | 2010-09-05 | 010-4764-7467 | 1480 |
| 104 | 070234 | 서윤서 | 인천 | 운영 | 사원 | 2024-10-04 | 010-3772-4131 | 1280 |
| 105 | 003686 | 최민준 | 인천 | 지원 | 사원 | 2019-08-04 | 010-3896-7486 | 2231 |
| 106 | 016216 | 정서윤 | 인천 | 지원 | 사원 | 2014-05-28 | 010-2648-9278 | 2620 |
| 107 | 034225 | 이민준 | 인천 | 지원 | 대리 | 2020-06-14 | 010-8213-3887 | 0697 |
| 108 | 081447 | 서현우 | 인천 | 지원 | 사원 | 2017-09-16 | 010-3189-0366 | 1979 |
</data>
</sheet>

<sheet name="교육">

<table name="Training" range="A1:G31">
| column | type | format | formula |
|---|---|---|---|
| 사번 | text | @ |  |
| 이름 | text | @ |  |
| 지점 | text | @ |  |
| 과정 | text | @ |  |
| 이수일 | date | yyyy-mm-dd |  |
| 시간 | number | 0 |  |
| 점수 | number | 0 |  |
</table>

<data table="Training">
| row | 사번 | 이름 | 지점 | 과정 | 이수일 | 시간 | 점수 |
|---|---|---|---|---|---|---|---|
| 2 | 041726 | 김하린 | 대전 | 고객 응대 | 2026-06-02 | 2 | 98 |
| 3 | 081447 | 서현우 | 인천 | 고객 응대 | 2026-06-04 | 2 | 74 |
| 4 | 017335 | 정선우 | 서울본점 | 정보보안 기초 | 2026-06-05 | 3 | 69 |
| 5 | 018592 | 송연우 | 인천 | 직장 내 괴롭힘 예방 | 2026-06-06 | 1 | 86 |
| 6 | 003943 | 황태호 | 광주 | 고객 응대 | 2026-06-08 | 2 | 70 |
| 7 | 075461 | 오준서 | 광주 | 정보보안 기초 | 2026-06-09 | 3 | 83 |
| 8 | 010589 | 안준서 | 인천 | 정보보안 기초 | 2026-06-11 | 3 | 87 |
| 9 | 052008 | 송우진 | 서울본점 | 직장 내 괴롭힘 예방 | 2026-06-15 | 1 | 90 |
| 10 | 070896 | 장가은 | 대전 | 개인정보 보호 | 2026-06-17 | 2 | 85 |
| 11 | 021857 | 조유진 | 대전 | 개인정보 보호 | 2026-06-19 | 2 | 69 |
| 12 | 032091 | 최민서 | 인천 | 직장 내 괴롭힘 예방 | 2026-06-20 | 1 | 91 |
| 13 | 037389 | 임윤서 | 광주 | 직장 내 괴롭힘 예방 | 2026-06-22 | 1 | 79 |
| 14 | 050806 | 최현우 | 인천 | 고객 응대 | 2026-07-04 | 2 | 77 |
| 15 | 072605 | 황승현 | 대전 | 고객 응대 | 2026-07-07 | 2 | 80 |
| 16 | 060777 | 정예준 | 대전 | 산업안전 보건 | 2026-07-11 | 4 | 72 |
| 17 | 019684 | 장혜진 | 대구 | 고객 응대 | 2026-07-12 | 2 | 84 |
| 18 | 052045 | 장보람 | 서울본점 | 개인정보 보호 | 2026-07-13 | 2 | 71 |
| 19 | 064477 | 김서연 | 부산 | 개인정보 보호 | 2026-07-14 | 2 | 91 |
| 20 | 068328 | 김서연 | 대구 | 개인정보 보호 | 2026-07-14 | 2 | 84 |
| 21 | 094217 | 홍미경 | 인천 | 산업안전 보건 | 2026-07-15 | 4 | 68 |
| 22 | 063479 | 홍서연 | 인천 | 고객 응대 | 2026-07-16 | 2 | 71 |
| 23 | 085719 | 류윤서 | 광주 | 산업안전 보건 | 2026-07-18 | 4 | 93 |
| 24 | 072225 | 최하준 | 인천 | 고객 응대 | 2026-07-28 | 2 | 96 |
| 25 | 095370 | 오태호 | 광주 | 개인정보 보호 | 2026-08-04 | 2 | 79 |
| 26 | 024631 | 조시우 | 부산 | 개인정보 보호 | 2026-08-08 | 2 | 91 |
| 27 | 091175 | 황재원 | 대구 | 정보보안 기초 | 2026-08-10 | 3 | 75 |
| 28 | 057765 | 오수빈 | 부산 | 직장 내 괴롭힘 예방 | 2026-08-12 | 1 | 77 |
| 29 | 092144 | 오지호 | 광주 | 고객 응대 | 2026-08-18 | 2 | 99 |
| 30 | 010394 | 이지민 | 대구 | 고객 응대 | 2026-08-22 | 2 | 71 |
| 31 | 016216 | 정서윤 | 인천 | 정보보안 기초 | 2026-08-28 | 3 | 83 |
</data>
</sheet>

<sheet name="요약">

<table name="Headcount" range="A1:E19">
| column | type | format | formula |
|---|---|---|---|
| 지점 | text | @ |  |
| 부서 | text | @ |  |
| 정원 | number | 0 |  |
| 현원 | number | 0 |  |
| 결원 | number | 0 |  |
</table>

<data table="Headcount">
| row | 지점 | 부서 | 정원 | 현원 | 결원 |
|---|---|---|---|---|---|
| 2 | 서울본점 | 영업 | 8 | 7 | 1 |
| 3 | 서울본점 | 운영 | 7 | 6 | 1 |
| 4 | 서울본점 | 지원 | 5 | 4 | 1 |
| 5 | 부산 | 영업 | 9 | 7 | 2 |
| 6 | 부산 | 운영 | 7 | 6 | 1 |
| 7 | 부산 | 지원 | 4 | 4 | 0 |
| 8 | 대구 | 영업 | 9 | 7 | 2 |
| 9 | 대구 | 운영 | 7 | 6 | 1 |
| 10 | 대구 | 지원 | 5 | 4 | 1 |
| 11 | 광주 | 영업 | 8 | 7 | 1 |
| 12 | 광주 | 운영 | 7 | 6 | 1 |
| 13 | 광주 | 지원 | 4 | 4 | 0 |
| 14 | 대전 | 영업 | 8 | 7 | 1 |
| 15 | 대전 | 운영 | 7 | 5 | 2 |
| 16 | 대전 | 지원 | 6 | 4 | 2 |
| 17 | 인천 | 영업 | 8 | 7 | 1 |
| 18 | 인천 | 운영 | 7 | 6 | 1 |
| 19 | 인천 | 지원 | 5 | 4 | 1 |
</data>
</sheet>
````

## Tasks

The write task and each edit task are answered with `text`: the JSON list of operations that makes the change.

### roster2-w (write)

On sheet 요약, add a new table named Overtime whose top-left cell is G1, with four columns in this order: 지점 (text), 월 (a date, format yyyy-mm), 초과근무 (a number, format #,##0) and 1인당 (a number, format 0.0). 1인당 is a formula column: the row's 초과근무 divided by the number of rows of the table Roster whose 지점 is the row's 지점. The table has these rows, in this order: 서울본점, 2026-08, 412; 부산, 2026-08, 288; 대구, 2026-08, 305; 광주, 2026-08, 196. Change nothing else.

### roster2-e1 (edit)

In Training, the 점수 of 김서연 of the 대구 branch in the course 개인정보 보호 should be 88. Change only that cell.

### roster2-e2 (edit)

Append the September 2026 completions to Training, after its last row, in this order (사번, 이름, 지점, 과정, 이수일, 시간, 점수): 052008, 송우진, 서울본점, 고객 응대, 2026-09-03, 2, 86; 010607, 최예은, 대구, 산업안전 보건, 2026-09-10, 4, 92; 019105, 황지민, 광주, 개인정보 보호, 2026-09-17, 2, 79; 063479, 홍서연, 인천, 정보보안 기초, 2026-09-24, 3, 95. Change nothing else.

### roster2-e3 (edit)

In Headcount, 결원 was typed by hand. Make 결원 a formula column: 정원 minus 현원, on every row. Change nothing else.

### roster2-e4 (edit)

In Roster, 사번 holds employee IDs but is typed as a number with format 000000. Make it a text column, so that every ID keeps its leading zeros exactly as displayed now. Change nothing else.

### roster2-e5 (edit)

Add 714 rows to Training, one for every person in Roster and each of the 7 courses planned for 2027, with 이수일 and 점수 left empty, so that the HR team can fill them in later.

## Tasks that cannot be done

Do only what the documentation above allows. If a task asks for something it cannot express or does not allow, do not approximate it: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its answer.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a task>", "text": [{"op": "…", …}, …]},
  {"task_id": "<id of a task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```
