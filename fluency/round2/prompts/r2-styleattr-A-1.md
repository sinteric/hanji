You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts, sizes, alignment, borders and highlighting cannot be written directly.

- A paragraph with a style is `<div style="Name">text</div>`, on one line.
- A table with a style is a line `<table style="Name">`, then a pipe table, then a line `</table>`. A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. Nothing else is between the two tag lines, not even a blank line.
- A paragraph or a pipe table without a tag has the default style.

The value of `style` is exactly one style name, written as listed, spaces included. A paragraph takes a paragraph style and a table a table style. `<div>` and `<table>` take no other attribute.

Example:

```
<div style="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table style="Grid Table 4">
| 지역 | 매출 |
|---|---|
| 수도권 | 1,204 |
</table>
```

## Names available in this file

Paragraph styles:
- `Body Text` — ordinary body paragraph
- `Body Text Indent` — body paragraph indented 10 mm, for sub-points under the paragraph before it
- `Body Text 2` — body paragraph with double line spacing, for draft wording under review
- `Quote` — italic quotation, indented on both sides
- `Intense Quote` — bold italic quotation between two thin rules, for decisions recorded word for word
- `Note` — small gray text for side remarks
- `Note Heading` — bold label line that introduces a note
- `Caption` — 9 pt label directly above a table
- `Signature` — right-aligned sign-off line at the end of a document

Table styles:
- `Table Grid` — thin black lines around every cell
- `Grid Table 4` — gray header row, gray banded rows
- `Grid Table 4 Accent 1` — blue header row, light blue banded rows
- `Plain Table 1` — light horizontal lines only

## The file: memo-1.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 2026년 9월 경영회의 회의록

<div style="Body Text">일시: 2026년 9월 24일(목) 10:00 ~ 11:40 / 장소: 본사 12층 대회의실</div>

## 1. 참석자

<div style="Caption">표 1. 참석자</div>

<table style="Table Grid">
| 구분 | 성명 | 소속 |
|---|---|---|
| 의장 | 김정호 | 대표이사 |
| 참석 | 이수민 | 경영지원본부 |
| 참석 | 박태윤 | 영업본부 |
| 참석 | 최은영 | 생산본부 |
| 배석 | 박소영 | 경영지원팀 |
</table>

<div style="Note">생산본부장은 10시 30분부터 참석했다.</div>

## 2. 안건 1: 3분기 실적 점검

<div style="Body Text">3분기 매출은 6,203백만 원으로 목표 대비 97%를 달성했다.</div>

<div style="Body Text Indent">수도권 매출은 목표를 넘었으나 호남권은 목표 대비 89%에 그쳤다.</div>

<div style="Body Text Indent">서비스본부는 유지보수 계약 해지 2건의 영향으로 목표 대비 94%에 머물렀다.</div>

<div style="Body Text 2">(초안) 4분기 목표는 3분기 실적을 반영하여 10월 경영회의에서 재조정한다.</div>

<div style="Note Heading">참고</div>

<div style="Note">세부 내용은 첨부 자료 참조.</div>

<div style="Caption">표 2. 3분기 본부별 실적 (단위: 백만 원)</div>

<table style="Grid Table 4">
| 본부 | 목표 | 실적 | 달성률 |
|---|---|---|---|
| 영업본부 | 4,200 | 4,080 | 97% |
| 생산본부 | 1,450 | 1,420 | 98% |
| 서비스본부 | 750 | 703 | 94% |
</table>

<div style="Body Text">4분기에는 호남권 판촉과 서비스본부 계약 갱신에 집중한다.</div>

## 3. 안건 2: 2027년 예산 편성 방향

<div style="Body Text">2027년 예산은 2026년 대비 5% 이내 증액을 원칙으로 한다.</div>

인건비는 동결하되, 신규 채용 5명분은 별도로 반영한다.

<div style="Body Text">마케팅 예산은 온라인 채널 중심으로 재배분한다.</div>

<div style="Body Text 2">(초안) 시설 예산은 본사 노후 설비 교체를 2028년으로 미루는 안을 전제로 한다.</div>

<div style="Note Heading">참고</div>

<div style="Note">세부 내용은 첨부 자료 참조.</div>

<div style="Caption">표 3. 2027년 예산 편성 방향 (단위: 백만 원)</div>

<table style="Grid Table 4">
| 항목 | 2026년 | 2027년 안 | 증감 |
|---|---|---|---|
| 인건비 | 2,100 | 2,180 | +80 |
| 마케팅 | 640 | 700 | +60 |
| 시설 | 380 | 350 | -30 |
| 합계 | 3,120 | 3,230 | +110 |
</table>

## 4. 결정 사항

<div style="Intense Quote">호남권 영업 강화를 위해 10월 중 권역 TF를 구성한다.</div>

<div style="Intense Quote">2027년 예산 초안은 10월 20일까지 경영지원본부에 제출한다.</div>

<div style="Intense Quote">4분기 목표 재조정안은 10월 경영회의에 상정한다.</div>

보안 교육 미이수자는 10월 말까지 이수한다.

## 5. 후속 조치

<div style="Caption">표 4. 후속 조치</div>

<table style="Grid Table 4">
| 조치 | 담당 | 기한 |
|---|---|---|
| 권역 TF 구성 | 영업본부 | 10월 15일 |
| 예산 초안 제출 | 각 본부 | 10월 20일 |
| 보안 교육 이수 점검 | 경영지원팀 | 10월 31일 |
| 4분기 목표 재조정안 작성 | 경영지원본부 | 10월 27일 |
</table>

<div style="Note Heading">참고</div>

<div style="Note">세부 내용은 첨부 자료 참조.</div>

## 6. 기타 보고

<div style="Body Text">정보보안팀은 9월 모의 해킹 점검 결과를 보고했다.</div>

<div style="Body Text Indent">외부 공개 서버 12대 중 2대에서 취약점이 발견되어 9월 30일까지 조치한다.</div>

<div style="Body Text Indent">보안 교육 이수율은 9월 말 기준 87%이다.</div>

<div style="Caption">표 5. 본부별 보안 교육 이수율</div>

<table style="Grid Table 4">
| 본부 | 대상 | 이수 | 이수율 |
|---|---|---|---|
| 영업본부 | 120 | 98 | 82% |
| 생산본부 | 140 | 126 | 90% |
| 경영지원본부 | 45 | 41 | 91% |
| 합계 | 305 | 265 | 87% |
</table>

<div style="Quote">보안은 한 사람의 실수로 무너진다. (대표이사 당부 사항)</div>

<div style="Note Heading">참고</div>

<div style="Note">세부 내용은 첨부 자료 참조.</div>

<div style="Body Text">다음 회의: 2026년 10월 29일(목) 10:00, 본사 12층 대회의실</div>

<div style="Note">회의 자료는 사내 게시판 경영회의 폴더에 게시했다.</div>

작성: 경영지원팀 박소영 / 확인: 경영지원본부장 이수민
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### memo1-w (write)

Write the minutes of a short meeting as a new document, in this order: a heading level 1 "2026년 10월 영업전략회의 회의록"; the line "일시: 2026년 10월 8일(목) 14:00 ~ 15:00" as an ordinary body paragraph; the label "표 1. 참석자" in the file's style for a label above a table; directly after it a table with thin black lines around every cell, with the columns 구분, 성명, 소속 and the rows 의장 / 박태윤 / 영업본부, 참석 / 정다은 / 영업기획팀, 참석 / 한도윤 / 온라인영업팀; the decision "온라인영업팀은 11월 프로모션 계획을 10월 22일까지 보고한다." recorded word for word in the file's style for that; the side remark "세부 일정은 별도 공지." in small gray text; and the sign-off "작성: 영업기획팀 정다은", right-aligned as the last line.

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### memo1-e1 (edit)

Give the table under "5. 후속 조치" the version of its current style that has a blue header row. Change nothing else.

### memo1-e2 (edit)

Make the sentence "보안 교육 미이수자는 10월 말까지 이수한다." red, so that nobody misses it.

### memo1-e3 (edit)

Under "3. 안건 2: 2027년 예산 편성 방향", the paragraphs "인건비는 동결하되, 신규 채용 5명분은 별도로 반영한다." and "마케팅 예산은 온라인 채널 중심으로 재배분한다." are sub-points of the paragraph before them. Give each of them the file's indented style for sub-points. Change nothing else.

### memo1-e4 (edit)

Right-align the last line, "작성: 경영지원팀 박소영 / 확인: 경영지원본부장 이수민", as the sign-off of the minutes.

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
