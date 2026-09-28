You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and four tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts and sizes cannot be written directly.

- A paragraph with a style is `<div style="Name">text</div>`, on one line.
- A table with a style is `<table style="Name">`, then one `<tr>` per line with `<th>` header cells and `<td>` cells, then `</table>`.
- A paragraph or a pipe table without a tag has the default style.

The value of `style` is exactly one style name, written as listed, spaces included. `<div>` and `<table>` take no other attribute.

Example:

```
<div style="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table style="Grid Table 4">
<tr><th>지역</th><th>매출</th></tr>
<tr><td>수도권</td><td>1,204</td></tr>
</table>
```

## Names available in this file

Paragraph styles:
- `Note` — 회색 배경의 참고 문단
- `Decision` — 굵은 글씨의 결정 사항 문단
- `Alert Box` — 빨간색 굵은 큰 글씨의 경고 문단
- `Body Text Indent` — 들여 쓴 본문
- `Caption` — 작은 회색 글씨의 설명 문단
- `Intense Quote` — 위아래 테두리가 있는 강조 인용 문단

Table styles:
- `Table Grid` — 기본 격자
- `Grid Table 4` — 머리글 행이 진한 격자
- `List Table 3 Accent 1` — 파란 머리글의 목록형 표

## The file: document-1.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 제38차 주간 경영회의 회의록

<div style="Caption">일시: 2026-09-21(월) 09:00 / 장소: 본사 12층 대회의실</div>

## 1. 참석자

대표이사, 경영지원본부장, 영업본부장, 연구소장, 재무팀장

## 2. 안건별 논의

### 안건 1. 4분기 채용 계획

<div style="Body Text Indent">연구소 개발 인력 6명, 영업본부 2명 충원을 요청함.</div>

<div style="Note">인건비 증가분은 4분기 예산 범위 내에서 집행.</div>

<div style="Decision">결정: 연구소 6명 우선 채용, 영업본부는 11월 재검토.</div>

### 안건 2. 판교 사무실 이전

<div style="Body Text Indent">판교 사무실 임대 계약이 2027년 2월 만료됨.</div>

<div style="Note">이전 비용은 4분기 예산 범위 내에서 집행.</div>

<div style="Decision">결정: 후보지 3곳 실사 후 10월 회의에서 확정.</div>

### 안건 3. 보안 점검 결과

정보보안 점검 결과 외부 반출 의심 건 1건이 확인됨.

재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.

## 3. 후속 조치

<table style="Grid Table 4">
<tr><th>조치 사항</th><th>담당</th><th>기한</th></tr>
<tr><td>채용 공고 게시</td><td>경영지원본부</td><td>9월 25일</td></tr>
<tr><td>후보지 실사</td><td>경영지원본부</td><td>10월 10일</td></tr>
<tr><td>보안 교육 계획 수립</td><td>정보보안팀</td><td>9월 30일</td></tr>
</table>

<div style="Caption">다음 회의: 2026-09-28(월) 09:00</div>
````

## Tasks

### document1-w (write)

Write a new document with, in this order: a heading level 1 "10월 전사 워크숍 안내"; a paragraph in the default style "10월 17일(금) 전 직원 대상 워크숍을 진행합니다."; a paragraph in style Note "장소: 가평 한빛연수원 / 버스 출발: 본사 정문 08:00"; a paragraph in style Alert Box "불참자는 10월 10일까지 팀장에게 사유를 제출해야 합니다."; and a table in style List Table 3 Accent 1 with the header row 시간 / 프로그램 and the rows 10:00 / 대표이사 인사말, 11:00 / 팀별 발표, 14:00 / 팀 빌딩.

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

### document1-e1 (edit)

Under 안건 2, the paragraph "이전 비용은 4분기 예산 범위 내에서 집행." should have the style Body Text Indent instead of its current style. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### document1-e2 (edit)

The CEO asked: make the sentence "재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다." red, bold and large so that it stands out. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### document1-e3 (edit)

Right after the paragraph "재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.", add a new paragraph in style Intense Quote reading "보안 교육 미이수자는 사내 시스템 접속이 제한된다."

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
