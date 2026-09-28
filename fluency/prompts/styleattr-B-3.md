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
- `본문` — 기본 본문
- `참고 문단` — 회색 상자의 참고 사항
- `강조 문단` — 빨간색 굵은 큰 글씨의 강조 문단
- `인용 문단` — 들여 쓴 기울임 인용
- `붙임 목록` — 첨부 목록 문단
- `발신 명의` — 가운데 정렬 큰 글씨의 발신 명의

Table styles:
- `기본 표` — 기본 격자
- `격자 표 4` — 머리글 행이 진한 격자
- `목록 표 강조 2` — 주황 머리글의 목록형 표

## The file: document-3.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 개인정보 처리방침 개정 안내

수신: 전 부서장 / 참조: 정보보호위원회

## 1. 개정 사유

개인정보 보호법 시행령 개정(2026. 9. 15. 시행)에 따라 처리방침을 개정합니다.

<div class="인용 문단">개인정보처리자는 처리 목적이 달성된 개인정보를 지체 없이 파기하여야 한다.</div>

## 2. 주요 개정 내용

| 구분 | 현행 | 개정 |
|---|---|---|
| 보유 기간 | 회원 탈퇴 후 1년 | 회원 탈퇴 후 6개월 |
| 파기 절차 | 분기별 일괄 파기 | 월별 일괄 파기 |
| 위탁 업체 | 3곳 | 4곳 |

<div class="참고 문단">위탁 업체 추가: 한결고객센터(고객 상담)</div>

<div class="참고 문단">개정 방침은 2026년 10월 1일부터 적용합니다.</div>

## 3. 부서별 조치 사항

1. 보유 중인 탈퇴 회원 정보 점검 (9월 30일까지)
1. 위탁 계약서 개인정보 조항 갱신 (10월 15일까지)
1. 부서 내 교육 실시 후 결과 보고 (10월 31일까지)

기한 내 미조치 부서는 정보보호위원회에 보고됩니다.

<div class="붙임 목록">붙임: 개정 처리방침 전문 1부. 끝.</div>

<div class="발신 명의">정보보호책임자</div>
````

## Tasks

### document3-w (write)

Write a new document with, in this order: a heading level 1 "사내 동호회 지원 안내"; a paragraph in style 본문 "2027년 동호회 활동비 지원 신청을 받습니다."; a paragraph in style 강조 문단 "신청 마감: 12월 5일(금) 18:00"; a table in style 격자 표 4 with the header row 구분 / 지원 금액 and the rows 정기 활동 / 월 10만 원 and 대회 참가 / 회당 30만 원; and a paragraph in style 발신 명의 "경영지원팀장".

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

### document3-e1 (edit)

Give the table under "2. 주요 개정 내용" the table style 목록 표 강조 2. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### document3-e2 (edit)

Make the sentence "기한 내 미조치 부서는 정보보호위원회에 보고됩니다." red, bold and big. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### document3-e3 (edit)

Right after the quoted paragraph "개인정보처리자는 처리 목적이 달성된 개인정보를 지체 없이 파기하여야 한다.", add a new paragraph in style 참고 문단 reading "관련 조항: 개인정보 보호법 제21조".

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
