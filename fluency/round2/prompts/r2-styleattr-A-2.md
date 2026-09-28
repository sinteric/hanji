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
- `Key Message` — large bold statement of the main point of a section
- `Key Message Small` — the same statement at body size, for a secondary main point
- `Callout` — boxed paragraph on a light gray background
- `Callout Warning` — boxed paragraph with an orange border, for risks
- `Footnote Text` — small text for sources and remarks
- `Caption` — 9 pt label directly above a table

Table styles:
- `Light List` — only a thin line under the header row
- `Light List Accent 2` — only a thin orange line under the header row
- `Medium Shading 1` — dark header row, shaded rows
- `Medium Shading 1 Accent 2` — orange header row, shaded rows

## The file: memo-2.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 이천 통합 물류센터 구축 제안서

<div style="Body Text">제안 부서: 물류혁신TF / 제출일: 2026년 9월 30일 / 보고 대상: 경영위원회</div>

## 1. 추진 배경

<div style="Key Message">분산된 3개 창고를 하나로 통합해 연간 물류비 18%를 절감한다.</div>

<div style="Body Text">현재 수도권 물량은 이천, 용인, 광주(경기) 3개 임차 창고에서 나누어 처리하고 있다.</div>

<div style="Body Text">창고 간 재고 이동이 월 1,200건에 달하고, 출고 리드타임은 평균 2일이다.</div>

<div style="Body Text">온라인 주문 증가로 2025년 이후 일 출고 건수가 해마다 15%씩 늘고 있다.</div>

<div style="Callout">임차 계약 3건 중 2건이 2027년 6월에 만료된다.</div>

<div style="Caption">표 1. 현행 창고 현황</div>

<table style="Light List">
| 창고 | 면적 (㎡) | 일 출고 (건) | 계약 만료 |
|---|---|---|---|
| 이천 | 9,900 | 8,000 | 2027년 6월 |
| 용인 | 6,600 | 5,500 | 2027년 6월 |
| 광주(경기) | 4,300 | 3,200 | 2028년 12월 |
</table>

## 2. 추진 방안

<div style="Key Message Small">이천 부지에 연면적 23,000㎡ 규모의 통합 센터를 신축한다.</div>

<div style="Body Text">자동 분류기와 WMS를 도입하고, 용인·광주 창고는 계약 만료 시 반납한다.</div>

<div style="Body Text">신축 센터는 지상 4층 규모로, 1~2층은 입출고 구역, 3~4층은 보관 구역으로 쓴다.</div>

<div style="Footnote Text">연면적은 기본 설계 전 추정치이다.</div>

공사 기간 중 성수기 물량이 몰리면 출고 지연이 생길 수 있다.

<div style="Caption">표 2. 투자 계획 (단위: 억 원)</div>

<table style="Medium Shading 1">
| 항목 | 2027년 | 2028년 | 합계 |
|---|---|---|---|
| 토지·건축 | 180 | 60 | 240 |
| 설비 | 40 | 55 | 95 |
| 시스템 | 12 | 18 | 30 |
| 합계 | 232 | 133 | 365 |
</table>

<div style="Footnote Text">금액은 VAT 별도, 설계 전 추정치이다.</div>

## 3. 기대 효과

<div style="Body Text">통합 후 연간 물류비 18% 절감과 출고 리드타임 단축이 예상된다.</div>

<div style="Caption">표 3. 기대 효과</div>

<table style="Medium Shading 1">
| 항목 | 현행 | 통합 후 |
|---|---|---|
| 연간 물류비 | 142억 원 | 116억 원 |
| 출고 리드타임 | 2일 | 1일 |
| 창고 간 재고 이동 | 월 1,200건 | 없음 |
</table>

<div style="Callout">물류비 절감액은 연간 약 26억 원으로 추정된다.</div>

<div style="Footnote Text">금액은 VAT 별도, 설계 전 추정치이다.</div>

## 4. 추진 일정

<div style="Key Message Small">2028년 6월 통합 센터 가동을 목표로 한다.</div>

<div style="Caption">표 4. 추진 일정</div>

<table style="Light List">
| 단계 | 기간 | 내용 |
|---|---|---|
| 설계 | 2027년 1월 ~ 4월 | 기본·실시 설계 |
| 시공 | 2027년 5월 ~ 2028년 4월 | 건축 및 설비 공사 |
| 이전 | 2028년 5월 ~ 6월 | 재고 이전 및 시범 운영 |
</table>

<div style="Body Text">이전 기간에는 기존 이천 창고 운영을 유지해 출고 공백을 막는다.</div>

<div style="Callout">이사회 승인 후 설계 발주까지 약 2개월이 소요된다.</div>

## 5. 위험 요인 및 대응

<div style="Callout Warning">인허가가 지연되면 착공이 최대 3개월 늦어질 수 있다.</div>

<div style="Body Text">인허가는 설계 단계부터 이천시와 사전 협의를 진행해 지연 가능성을 줄인다.</div>

<div style="Caption">표 5. 위험 요인별 대응</div>

<table style="Light List">
| 위험 요인 | 영향 | 대응 |
|---|---|---|
| 인허가 지연 | 착공 지연 | 설계 단계 사전 협의 |
| 공사비 상승 | 투자비 증가 | 주요 자재 단가 계약 |
| 성수기 출고 지연 | 고객 불만 | 임시 창고 확보 |
</table>

## 6. 요청 사항

<div style="Key Message Small">경영위원회에 투자 계획 승인을 요청한다.</div>

<div style="Body Text">승인 시 10월 중 설계 용역을 발주하고, 11월에 이천시와 인허가 사전 협의를 시작한다.</div>

## 7. 검토 의견

<div style="Body Text">재무팀: 2027년 차입 한도 안에서 조달할 수 있다.</div>

<div style="Body Text">법무팀: 임차 계약에 중도 해지 조항은 없으며, 만료 시 원상 복구 의무가 있다.</div>

<div style="Body Text">인사팀: 용인 창고 인력 32명은 이천 센터로 전환 배치한다.</div>

<div style="Footnote Text">금액은 VAT 별도, 설계 전 추정치이다.</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### memo2-w (write)

Write a new document, in this order: a heading level 1 "용인 창고 반납 계획"; the main point of the document, "용인 창고는 2027년 6월 계약 만료와 함께 반납한다.", as a large bold statement; the ordinary body paragraph "재고는 2027년 4월부터 이천 센터로 단계적으로 옮긴다."; the label "표 1. 반납 일정" in the file's style for a label above a table; directly after it a table whose only line is a thin orange line under the header row, with the columns 단계, 시기 and the rows 재고 이전 / 2027년 4월 ~ 5월 and 원상 복구 / 2027년 6월; the source line "자료: 물류혁신TF" in small text for sources; and the risk "원상 복구 비용이 보증금을 넘을 수 있다." in a box with an orange border.

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### memo2-e1 (edit)

The sentence "공사 기간 중 성수기 물량이 몰리면 출고 지연이 생길 수 있다." describes a risk. Give it the file's style for risks. Change nothing else.

### memo2-e2 (edit)

Give the table under "2. 추진 방안" the orange version of its current style. Change nothing else.

### memo2-e3 (edit)

In the paragraph under "3. 기대 효과", highlight the phrase "연간 물류비 18% 절감" in yellow.

### memo2-e4 (edit)

Right after the heading "3. 기대 효과", add the paragraph "출고 리드타임을 2일에서 1일로 줄인다." as a secondary main-point statement, at body size.

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
