You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and four tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts and sizes cannot be written directly.

- A paragraph with a style is `<div class="Name">text</div>`, on one line.
- A table with a style is `<table class="Name">`, then one `<tr>` per line with `<th>` header cells and `<td>` cells, then `</table>`.
- A paragraph or a pipe table without a tag has the default style.

The value of `class` is exactly one style name, written as listed, spaces included. `<div>` and `<table>` take no other attribute.

Example:

```
<div class="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table class="Grid Table 4">
<tr><th>지역</th><th>매출</th></tr>
<tr><td>수도권</td><td>1,204</td></tr>
</table>
```

## Names available in this file

Paragraph styles:
- `Subtitle` — 제목 아래 회색 부제
- `Key Message` — 파란색 굵은 16pt 강조 문단
- `Note` — 회색 배경의 참고 문단
- `Block Text` — 테두리 상자 안의 본문
- `Source Note` — 작은 기울임 글씨의 출처 표기

Table styles:
- `Table Grid` — 기본 격자
- `Grid Table 4 Accent 5` — 청록 머리글의 격자
- `Plain Table 1` — 가는 선만 있는 표

## The file: document-2.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 스마트 물류센터 구축 제안서

<div class="Subtitle">경기 이천 제2물류센터 자동화 사업</div>

## 1. 추진 배경

온라인 주문량이 최근 2년간 2.3배 증가해 기존 센터의 처리 한계에 도달했다.

피크 시즌 출고 지연률은 7.8%로 목표치 3%를 크게 웃돌았다.

<div class="Source Note">출처: 2026년 상반기 물류 운영 보고서</div>

## 2. 제안 내용

<div class="Block Text">자동 분류기(소터) 2기와 AMR 40대를 도입해 출고 공정을 자동화한다.</div>

<div class="Block Text">WMS를 클라우드형으로 전환해 재고 정확도를 99.5% 이상으로 유지한다.</div>

도입 후 시간당 처리량은 4,000박스에서 9,000박스로 늘어난다.

<table class="Grid Table 4 Accent 5">
<tr><th>구분</th><th>현재</th><th>도입 후</th></tr>
<tr><td>시간당 처리량</td><td>4,000박스</td><td>9,000박스</td></tr>
<tr><td>출고 지연률</td><td>7.8%</td><td>2% 이하</td></tr>
<tr><td>필요 인력</td><td>120명</td><td>85명</td></tr>
</table>

<div class="Source Note">출처: 설비 공급사 제안 자료 (2026-08)</div>

## 3. 투자 및 일정

총 투자비는 86억 원이며, 2027년 3월 착공해 2027년 10월 가동을 목표로 한다.

투자비 회수 기간은 약 4.2년으로 예상된다.

<div class="Note">세부 견적은 공급사 2곳의 최종 제안서 접수 후 확정한다.</div>
````

## Tasks

### document2-w (write)

Write a new document with, in this order: a heading level 1 "물류센터 견학 일정 안내"; a paragraph in style Subtitle "협력사 대상 / 2026년 11월"; a paragraph in the default style "견학은 회차별 20명 이내로 운영합니다."; a table in style Grid Table 4 Accent 5 with the header row 회차 / 일시 / 대상 and the rows 1회차 / 11월 5일 14:00 / 신규 협력사 and 2회차 / 11월 12일 14:00 / 기존 협력사; and a paragraph in style Source Note "문의: 물류기획팀 내선 3120".

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

Answer with `text`: the complete new file.

### document2-e1 (edit)

Give the comparison table (구분 / 현재 / 도입 후) the table style Plain Table 1 instead of its current style. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### document2-e2 (edit)

Make the sentence "도입 후 시간당 처리량은 4,000박스에서 9,000박스로 늘어난다." stand out in bold, blue, 16pt text. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### document2-e3 (edit)

Right after the first 출처 paragraph ("출처: 2026년 상반기 물류 운영 보고서"), add a new paragraph in style Block Text reading "센터 확장 없이 처리 능력을 두 배 이상 늘리는 것이 이번 제안의 목표이다."

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
