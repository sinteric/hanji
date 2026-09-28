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
- `Light Grid` — thin gray lines, bold header row
- `Light Grid - Accent 1` — thin blue lines, bold blue header row
- `Medium Grid 3` — dark gray header row, shaded rows, white lines
- `Medium Grid 3 - Accent 1` — dark blue header row, shaded blue rows, white lines

## The file: form-3.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 2026년 9월 고객 불만 처리 현황

<div style="Body Text">작성: 고객지원센터 / 집계 기간: 9월 1일 ~ 9월 30일 / 보고 대상: 고객경험본부장</div>

## 1. 요약

<div style="Body Text">9월 접수 건수는 412건으로 전월 대비 6% 감소했고, 평균 처리 기간은 2.4일이었다.</div>

<div style="Body Text">처리 중인 건은 17건이며, 모두 10월 첫째 주 안에 종결할 예정이다.</div>

- 배송 지연 불만이 전체의 49%
- 채팅 접수 비중 확대 (24% → 28%)
- 환불 요청 처리 기간 목표 미달 (목표 1.5일, 실적 1.8일)

## 2. 유형별 접수 현황

<div style="Caption">표 1. 오프라인 채널 접수 (단위: 건)</div>

{style="Light Grid - Accent 1"}
| 채널 | 유형 | 접수 | 처리 완료 | 처리 중 |
|---|---|---|---|---|
| 매장 | 배송 지연 | 42 | 40 | 2 |
| ^^ | 제품 불량 | 35 | 35 | 0 |
| ^^ | 환불 요청 | 28 | 26 | 2 |
| 콜센터 | 배송 지연 | 61 | 58 | 3 |
| ^^ | 제품 불량 | 30 | 29 | 1 |
| ^^ | 환불 요청 | 22 | 22 | 0 |

<div style="Body Text">온라인 채널 접수는 다음과 같다.</div>

<div style="Caption">표 2. 온라인 채널 접수 (단위: 건)</div>

{style="Light Grid - Accent 1"}
| 채널 | 유형 | 접수 | 처리 완료 | 처리 중 |
|---|---|---|---|---|
| 홈페이지 | 배송 지연 | 38 | 36 | 2 |
| ^^ | 제품 불량 | 25 | 25 | 0 |
| ^^ | 환불 요청 | 17 | 15 | 2 |
| 채팅 | 배송 지연 | 60 | 57 | 3 |
| ^^ | 제품 불량 | 32 | 32 | 0 |
| ^^ | 환불 요청 | 22 | 20 | 2 |

<div style="Note">처리 중 건수는 9월 30일 18시 기준이다.</div>

<div style="Body Text">오프라인 접수는 218건, 온라인 접수는 194건이다.</div>

<div style="Body Text">채팅 접수는 114건으로, 처음으로 콜센터 접수(113건)를 넘었다.</div>

## 3. 처리 기간

<div style="Caption">표 3. 유형별 평균 처리 기간 (단위: 일)</div>

{style="Light Grid"}
| 유형 | 채널 구분 | 8월 | 9월 | 목표 |
|---|---|---|---|---|
| 배송 지연 | 오프라인 | 2.8 | 2.5 | 2.0 |
| ^^ | 온라인 | 2.2 | 1.9 | ^^ |
| 제품 불량 | 오프라인 | 3.1 | 2.9 | 2.5 |
| ^^ | 온라인 | 2.7 | 2.6 | ^^ |
| 환불 요청 | 오프라인 | 2.0 | 2.1 | 1.5 |
| ^^ | 온라인 | 1.6 | 1.5 | ^^ |

<div style="Note">목표는 2026년 고객 서비스 기준에 따른 값이다.</div>

<div style="Body Text">환불 요청은 오프라인 처리 기간이 늘어 목표를 넘었다.</div>

<div style="Body Text">배송 지연과 제품 불량은 온라인 채널에서 목표 대비 0.1일 이내로 근접했다.</div>

## 4. 재발 방지 대책

<div style="Body Text">배송 지연 대책은 물류팀과 고객지원센터가 함께 추진한다.</div>

| 유형 | 대책 | 담당 | 기한 |
|---|---|---|---|
| 배송 지연 | 권역별 배송 협력사 추가 | 물류팀 | 10월 31일 |
| ^^ | 지연 예상 시 사전 문자 안내 | 고객지원센터 | 10월 15일 |
| 제품 불량 | 입고 검사 기준 강화 | 품질팀 | 11월 15일 |
| 환불 요청 | 환불 처리 단계 축소 (5단계 → 3단계) | 고객지원센터 | 10월 31일 |
| 공통 | 상담 품질 교육 (전 상담원) | ^^ | 11월 30일 |

<div style="Note">대책별 진행 상황은 매주 월요일 점검한다.</div>

<div style="Body Text">10월 말에 대책별 효과를 1차 점검해 11월 보고에 포함한다.</div>

## 5. 채널별 고객 만족도

<div style="Caption">표 4. 채널별 만족도 (5점 만점)</div>

{style="Medium Grid 3"}
| 채널 구분 | 채널 | 8월 | 9월 | 증감 |
|---|---|---|---|---|
| 오프라인 | 매장 | 4.1 | 4.2 | +0.1 |
| ^^ | 콜센터 | 3.8 | 3.9 | +0.1 |
| 온라인 | 홈페이지 | 3.9 | 3.9 | 0.0 |
| ^^ | 채팅 | 4.0 | 4.3 | +0.3 |

<div style="Note">만족도는 처리 완료 고객 대상 문자 설문 결과이다.</div>

<div style="Body Text">채팅 만족도가 가장 크게 올랐으며, 콜센터 만족도는 여전히 가장 낮다.</div>

## 6. 10월 중점 관리

- 추석 연휴 전후 배송 지연 문의 대응 인력 20% 증원
- 채팅 상담 응답 시간 1분 이내 유지
- 환불 처리 단계 축소안 10월 31일 시행
- 콜센터 상담원 대상 불만 고객 응대 교육 (10월 2주차)

## 7. 우수 상담원

{style="Light Grid"}
| 채널 | 상담원 | 처리 건수 | 만족도 |
|---|---|---|---|
| 콜센터 | 김하늘 | 312 | 4.8 |
| ^^ | 이준서 | 298 | 4.7 |
| 채팅 | 박서윤 | 405 | 4.9 |

<div style="Note">우수 상담원은 10월 월례 조회에서 시상한다.</div>

<div style="Body Text">다음 보고는 11월 첫째 주에 한다.</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### form3-w (write)

Write a new document, in this order: a heading level 1 "2026년 10월 상담원 교육 일정"; the label "표 1. 교육 일정" in the file's style for a label above a table; directly after it a table with thin gray lines and a bold header row, with the columns 주차, 과정, 대상, 시간 for these rows, in this order (주차 / 과정 / 대상 / 시간):
- 1주차 / 응대 화법 / 신입 상담원 / 4시간
- 1주차 / 불만 고객 응대 / 신입 상담원 / 4시간
- 2주차 / 환불 규정 개정 안내 / 전 상담원 / 2시간
- 2주차 / 채팅 상담 도구 / 전 상담원 / 2시간
- 3주차 / 사례 발표회 (전 상담원 참석), written once across the 과정, 대상 and 시간 columns
In the table, each 주차 should appear once for its rows, and within a 주차 a 대상 or a 시간 that repeats should appear once (each column on its own). After the table, the side remark "교육은 매주 수요일 14시에 시작한다." in small gray text.

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### form3-e1 (edit)

In the table under "3. 처리 기간", add a row for the new 채널 구분 "모바일 앱" of 환불 요청, right after its 온라인 row, with 8월 "-" and 9월 1.2. It belongs to 환불 요청, and its 목표 is the same 1.5 as the other 환불 요청 rows, shown once for all of them.

### form3-e2 (edit)

Give the table under "4. 재발 방지 대책" the table style with a dark blue header row. Change nothing else.

### form3-e3 (edit)

Make the table under "3. 처리 기간" span the full page width, with equal column widths.

### form3-e4 (edit)

Join the two tables under "2. 유형별 접수 현황" into one table in which the 온라인 rows directly follow the 오프라인 rows. Remove the paragraph "온라인 채널 접수는 다음과 같다.", the label "표 2. 온라인 채널 접수 (단위: 건)" and the header row of the second table. Change the first label to "표 1. 채널별 접수 (단위: 건)". The joined table keeps the style of the first table.

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
