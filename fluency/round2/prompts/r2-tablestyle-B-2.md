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
- `본문` — 기본 본문
- `표 제목` — 표 바로 위의 표 제목 줄
- `참고` — 작은 회색 글씨의 참고 사항

Table styles:
- `표 눈금` — 모든 칸에 가는 검은 실선
- `눈금 표 4` — 회색 머리글 행, 회색 줄무늬 행
- `눈금 표 4 - 강조색 1` — 파란 머리글 행, 연한 파란 줄무늬 행
- `일반 표 1` — 가는 가로선만, 세로선 없음

## The file: form-2.hj.md

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---

# 2027년 임직원 교육 훈련 계획

<div style="본문">주관: 인재개발팀 / 시행: 2027. 1. 1. ~ 12. 31. / 대상: 전 임직원 420명</div>

## 1. 기본 방향

<div style="본문">2027년 교육은 법정 의무 교육, 직무 교육, 리더십 교육, 외부 위탁 교육으로 나누어 운영한다.</div>

- 법정 의무 교육 이수율 100% 유지
- 직무 교육 1인당 연 20시간 이상
- 리더십 교육 대상 확대 (팀장 → 파트장)

<div style="본문">교육 시간은 1인 기준 연간 최소 이수 시간으로 관리하며, 부서장은 분기마다 이수 현황을 점검한다.</div>

<div style="본문">교육 과정별 세부 일정은 매 분기 시작 2주 전에 공지한다.</div>

<div style="본문">2026년 이수율은 법정 의무 교육 100%, 직무 교육 87%, 리더십 교육 92%였다.</div>

## 2. 법정 의무 교육

<div style="본문">법정 의무 교육은 관련 법령에 따라 전 직원이 이수해야 하며, 온라인 과정으로도 운영한다.</div>

<div style="표 제목">표 1. 법정 의무 교육</div>

{style="눈금 표 4"}
| 분야 | 과정명 | 시기 | 대상 | 시간 |
|---|---|---|---|---|
| 안전 | 산업안전보건 교육 | 분기별 | 전 직원 | 12 |
| ^^ | 소방 안전 교육 | 5월 | ^^ | 2 |
| 인권 | 성희롱 예방 교육 | 3월 | ^^ | 1 |
| ^^ | 장애인 인식 개선 교육 | 9월 | ^^ | 1 |
| 정보 | 개인정보 보호 교육 | 6월 | ^^ | 2 |

<div style="참고">법정 의무 교육 미이수자는 인사 평가에 반영한다.</div>

<div style="본문">신규 입사자는 입사 후 1개월 안에 법정 의무 교육을 이수한다.</div>

## 3. 직무 교육

<div style="본문">직무 교육은 본부별 수요 조사 결과를 반영해 편성했으며, 공통 과정은 전 직원이 신청할 수 있다.</div>

<div style="표 제목">표 2. 직무 교육</div>

{style="눈금 표 4"}
| 분야 | 과정명 | 시기 | 대상 | 시간 |
|---|---|---|---|---|
| 영업 | 협상 실무 | 4월 | 영업본부 | 16 |
| ^^ | 고객 데이터 분석 | 7월 | ^^ | 12 |
| 생산 | 품질 관리 심화 | 5월 | 생산본부 | 16 |
| ^^ | 설비 예방 정비 | 8월 | ^^ | 8 |
| 공통 | 보고서 작성 | 수시 | 전 직원 | 4 |

<div style="참고">직무 교육 시간은 1인 기준이다.</div>

<div style="본문">영업본부와 생산본부 과정은 본부별 교육 담당자가 운영을 지원한다.</div>

## 4. 리더십 및 외부 위탁 교육

<div style="표 제목">표 3. 리더십 교육</div>

{style="일반 표 1"}
| 과정명 | 대상 | 시기 | 시간 |
|---|---|---|---|
| 신임 팀장 과정 | 신임 팀장 | 3월 | 24 |
| 팀장 역량 심화 | 팀장 | 6월 | 16 |
| 파트장 코칭 과정 | 파트장 | ^^ | ^^ |

<div style="참고">리더십 과정은 사외 강사와 사내 임원이 함께 진행한다.</div>

<div style="본문">팀장 역량 심화와 파트장 코칭 과정은 6월에 함께 운영하며, 시간은 과정별 16시간이다.</div>

<div style="본문">리더십 교육은 사내에서, 전문 과정은 외부 기관에 위탁해 운영한다. 외부 위탁 과정은 다음과 같다.</div>

## 5. 교육 예산

<div style="표 제목">표 4. 교육 예산 (단위: 천 원)</div>

{style="눈금 표 4 - 강조색 1"}
| 구분 | 항목 | 금액 |
|---|---|---|
| 법정 의무 교육 | 강사료 | 12,000 |
| ^^ | 교재비 | 3,000 |
| 직무 교육 | 강사료 | 48,000 |
| ^^ | 교재비 | 9,500 |
| 리더십 교육 | 위탁 교육비 | 36,000 |
| 합계 || 108,500 |

<div style="참고">예산은 2027년 사업계획 확정 시 조정될 수 있다.</div>

<div style="참고">외부 위탁 과정의 교육비는 직무 교육 예산에서 집행한다.</div>

<div style="본문">2027년 교육 예산은 2026년 대비 6% 늘어난 108,500천 원이다.</div>

## 6. 교육 평가 및 사후 관리

<div style="본문">모든 과정은 만족도와 현업 적용도를 평가하고, 결과를 다음 연도 계획에 반영한다.</div>

<div style="표 제목">표 5. 평가 방법</div>

{style="눈금 표 4"}
| 구분 | 평가 항목 | 시기 | 방법 |
|---|---|---|---|
| 과정 평가 | 만족도 | 과정 종료 직후 | 설문 |
| ^^ | 학습 성취도 | ^^ | 사후 시험 |
| 현업 적용 | 적용도 | 종료 3개월 후 | 설문 |
| ^^ | 성과 기여 | 종료 6개월 후 | 팀장 면담 |

<div style="참고">평가 결과는 인재개발팀이 분기별로 보고한다.</div>

<div style="본문">현업 적용도는 교육 대상자와 소속 팀장이 함께 평가한다.</div>

<div style="본문">만족도 4.0점 미만 과정은 다음 연도에 내용과 강사를 다시 검토한다.</div>

## 7. 행정 사항

- 교육 신청은 사내 교육 시스템에서 받는다.
- 외부 교육 참가자는 결과 보고서를 1주일 안에 제출한다.
- 교육 시간은 근무 시간으로 인정한다.
- 교육 불참 시 부서장 승인을 받아 다음 차수로 이월한다.
- 사외 교육 비용은 사전 승인된 과정만 지원한다.
- 교육 관련 문의: 인재개발팀 (내선 4120)

<div style="본문">끝.</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### form2-w (write)

Write a new document, in this order: a heading level 1 "2027년 신입사원 입문 교육"; the ordinary body paragraph "대상: 2027년 상반기 신입사원 / 장소: 연수원"; the line "표 1. 입문 교육 일정" in the file's style for a table's title line; directly after it a table with a gray header row and gray banded rows, with the columns 일차, 시간, 과정, 강사 for these rows, in this order (일차 / 시간 / 과정 / 강사):
- 1일차 / 오전 / 회사 소개 / 인재개발팀
- 1일차 / 오후 / 조직 문화 / 인재개발팀
- 2일차 / 오전 / 법정 의무 교육 / 외부 강사
- 2일차 / 오후 / 법정 의무 교육 / 외부 강사
- 3일차 / 종일 / 현업 부서 견학, written once across the 과정 and 강사 columns
In the table, each 일차 should appear once for its rows, and within an 일차 a 과정 or a 강사 that repeats on its rows should appear once (each column on its own). After the table, the side remark "교육 일정은 인원에 따라 조정될 수 있다." in small gray text.

Start the new file with this front matter:

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---
````

### form2-e1 (edit)

Give the 직무 교육 table (under "3. 직무 교육") the blue-header version of its current style. Change nothing else.

### form2-e2 (edit)

Right after the paragraph that ends "외부 위탁 과정은 다음과 같다.", add a table with light horizontal lines only, with the columns 과정명, 위탁 기관, 기간 and these rows, in this order: 재무 회계 실무 / 한국생산성본부 / 5월 (3일); 데이터 분석 입문 / 한국생산성본부 / 6월 (2일); 프로젝트 관리 / 대한상공회의소 / 9월 (3일). The 위탁 기관 should appear once for consecutive rows with the same 기관; no other cells are combined.

### form2-e3 (edit)

Make the lines of the table under "2. 법정 의무 교육" thicker so that it prints clearly.

### form2-e4 (edit)

Move the heading "5. 교육 예산" together with everything under it, up to the heading "6. 교육 평가 및 사후 관리" (its title line, table, notes and paragraph), so that it comes right before the heading "2. 법정 의무 교육". The table keeps its style. Do not change any text, including the heading numbers.

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
