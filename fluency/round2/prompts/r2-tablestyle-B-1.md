You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. Colours, fonts, sizes, alignment, borders and widths cannot be written directly. A paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. A style name is written exactly as listed, spaces included.

### Tables

Every table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. This holds for merged tables too. Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column.
- A merged area is a rectangle; for two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. An empty cell that is not merged is written with a space, `|  |`. The text of a merged cell is written once, in its top-left cell. `^^` never appears in the first row, and `||` never starts a row.

### Table styles

A table with a style has a line `{style="Name"}` directly before its header row: no blank line or other text is between them. The line gives that one table its style; the pipe table itself is unchanged, and nothing closes it. The braces hold nothing but `style="Name"`, and the line belongs to no other block. A pipe table without such a line has the default table style.

Example ("서울" covers two rows, "합계" covers two columns; the table has the style Grid Table 4):

```
{style="Grid Table 4"}
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```

## Names available in this file

Paragraph styles:
- `Body Text` — ordinary body paragraph
- `Note` — small gray text for side remarks
- `Caption` — 9 pt label directly above a table

Table styles:
- `Table Grid` — thin black lines around every cell
- `Grid Table 4` — gray header row, gray banded rows
- `Grid Table 4 Accent 1` — blue header row, light blue banded rows
- `List Table 3` — dark header row, no vertical lines

## The file: form-1.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 2026년 하반기 협력업체 평가 결과 보고

<div style="Body Text">작성: 구매팀 / 평가 기간: 2026. 7. 1. ~ 9. 30. / 대상: 협력업체 7곳</div>

## 1. 평가 기준

<div style="Body Text">평가는 정량 항목 80점과 정성 항목 20점으로 구성하며, 정성 항목은 평가위원 3인의 평균으로 산정한다.</div>

<div style="Caption">표 1. 평가 항목과 배점</div>

{style="Table Grid"}
| 구분 | 항목 | 배점 | 비고 |
|---|---|---|---|
| 품질 | 불량률 | 30 | 월 평균 |
| ^^ | 품질 인증 | 10 | ISO 9001 등 |
| 납기 | 납기 준수율 | 25 | 월 평균 |
| ^^ | 긴급 대응 | 10 | 요청 후 48시간 |
| 가격 | 단가 경쟁력 | 15 | 시장가 대비 |
| 협력 | 개선 제안 | 10 | 연간 건수 |
| 합계 || 100 |  |

<div style="Note">배점은 2026년 구매위원회 의결 기준이다.</div>

## 2. 업체별 점수

<div style="Body Text">업체별 점수는 평가위원회 심의를 거쳐 10월 8일 확정했다.</div>

<div style="Caption">표 2. 업체별 평가 점수</div>

{style="Grid Table 4"}
| 분야 | 업체 | 품질 | 납기 | 가격 | 협력 | 총점 | 등급 |
|---|---|---|---|---|---|---|---|
| 제조 | 대한정밀 | 36 | 33 | 13 | 8 | 90 | A |
| ^^ | 성진테크 | 34 | 30 | 12 | 7 | 83 | B |
| ^^ | 우림산업 | 30 | 28 | 14 | 6 | 78 | B |
| ^^ | 한결부품 | 25 | 22 | 11 | 5 | 63 | C |
| 물류 | 동방로지스 | 35 | 34 | 12 | 9 | 90 | A |
| ^^ | 서해운송 | 28 | 27 | 13 | 7 | 75 | B |
| ^^ | 누리물류 | 22 | 20 | 10 | 4 | 56 | D |

<div style="Note">총점 90점 이상 A, 75점 이상 B, 60점 이상 C, 60점 미만 D.</div>

<div style="Body Text">제조 분야 평균은 78.5점, 물류 분야 평균은 73.7점이다.</div>

<div style="Body Text">등급별로는 A 등급 2곳, B 등급 3곳, C 등급 1곳, D 등급 1곳이다.</div>

## 3. 등급별 조치

| 등급 | 조치 | 시기 | 담당 |
|---|---|---|---|
| A | 우수 협력업체 지정 | 11월 | 구매팀 |
| ^^ | 2027년 물량 우선 배정 | 12월 | ^^ |
| B | 정기 모니터링 | 분기 1회 | ^^ |
| C | 개선 계획서 제출 요구 | 10월 | 품질팀 |
| ^^ | 현장 점검 | 11월 | ^^ |
| D | 거래 중단 검토 | 12월 | 구매위원회 |

<div style="Body Text">C, D 등급 업체는 개선 계획 이행 여부를 다음 평가에 반영한다.</div>

<div style="Note">우수 협력업체 지정 기간은 1년이며, 다음 평가에서 A 등급을 유지하면 연장한다.</div>

## 4. 전년 대비 점수 비교

<div style="Caption">표 3. 업체별 전년 대비 총점</div>

{style="Grid Table 4"}
| 분야 | 업체 | 2025년 하반기 | 2026년 하반기 | 증감 |
|---|---|---|---|---|
| 제조 | 대한정밀 | 88 | 90 | +2 |
| ^^ | 성진테크 | 85 | 83 | -2 |
| ^^ | 우림산업 | 74 | 78 | +4 |
| ^^ | 한결부품 | 70 | 63 | -7 |
| 물류 | 동방로지스 | 87 | 90 | +3 |
| ^^ | 서해운송 | 76 | 75 | -1 |
| ^^ | 누리물류 | 61 | 56 | -5 |

<div style="Note">한결부품과 누리물류는 2회 연속 점수가 하락했다.</div>

<div style="Body Text">전년 대비 점수가 오른 업체는 3곳, 내린 업체는 4곳이다.</div>

## 5. 향후 일정

{style="Grid Table 4"}
| 일정 | 내용 | 담당 |
|---|---|---|
| 10월 15일 | 결과 통보 | 구매팀 |
| 10월 31일 | 개선 계획서 접수 | 품질팀 |
| 11월 중 | 현장 점검 | ^^ |
| 12월 중 | 구매위원회 상정 | 구매팀 |
| 2027년 1월 | 상반기 평가 계획 수립 | ^^ |

<div style="Note">평가 결과에 이의가 있는 업체는 통보일로부터 7일 안에 재심을 신청할 수 있다.</div>

<div style="Note">업체별 평가표는 구매팀 공유 폴더에 있다.</div>

## 6. 평가위원

{style="Table Grid"}
| 구분 | 성명 | 소속 |
|---|---|---|
| 위원장 | 김태호 | 구매본부장 |
| 위원 | 이지은 | 품질팀장 |
| ^^ | 박성우 | 구매팀장 |
| ^^ | 최민정 | 재무팀 과장 |

<div style="Note">평가위원은 평가 대상 업체와 이해관계가 없음을 서약했다.</div>

## 7. 개선 계획서 제출 현황

{style="List Table 3"}
| 분야 | 업체 | 등급 | 제출 기한 | 상태 |
|---|---|---|---|---|
| 제조 | 한결부품 | C | 10월 31일 | 검토 중 |
| 물류 | 누리물류 | D | 10월 31일 | 보완 요청 |

<div style="Note">개선 계획서는 구매팀과 품질팀이 함께 검토한다.</div>

<div style="Body Text">보완 요청을 받은 업체는 11월 7일까지 다시 제출한다.</div>

<div style="Body Text">첨부: 업체별 평가표 7부.</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### form1-w (write)

Write a new document, in this order: a heading level 1 "협력업체 현장 점검 계획"; the ordinary body paragraph "점검 기간: 2026. 11. 2. ~ 11. 20."; the label "표 1. 점검 일정" in the file's style for a label above a table; directly after it a table with a blue header row and light blue banded rows, with the columns 분야, 업체, 점검일, 점검자 for these rows, in this order (분야 / 업체 / 점검일 / 점검자):
- 제조 / 성진테크 / 11월 3일 / 품질팀 김도현
- 제조 / 우림산업 / 11월 5일 / 품질팀 김도현
- 제조 / 한결부품 / 11월 10일 / 품질팀 오세린
- 물류 / 서해운송 / 11월 12일 / 구매팀 정민재
- 물류 / 누리물류 / 11월 19일 / 구매팀 정민재
In the table, each 분야 should appear once for its rows, and a 점검자 once for consecutive rows with the same 점검자; no other cells are combined. After the table, the side remark "점검 결과는 11월 27일까지 보고한다." in small gray text.

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### form1-e1 (edit)

Give the table under "3. 등급별 조치" the table style with a gray header row and gray banded rows. Change nothing else.

### form1-e2 (edit)

Make the header row of the table under "2. 업체별 점수" red.

### form1-e3 (edit)

Split the table under "2. 업체별 점수" into two tables: the 제조 rows stay in the first table, and the 물류 rows move to a second table that starts with a copy of the same header row. Between the two tables put the label "물류 분야" in the file's style for a label above a table. Both tables have the style of the original table. Everything else stays as it is.

### form1-e4 (edit)

The table under "1. 평가 기준" should have the default table style instead of its current style. Change nothing else.

## Tasks that cannot be done

Do only what the syntax documentation and the names above can express. If an edit task asks for something they cannot express, do not approximate it, and do not invent a name, tag or attribute: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its edits.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]},
  {"task_id": "<id of an edit task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```
