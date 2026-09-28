You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a block takes one of the file's own styles, listed with the file. Colours, fonts, sizes, alignment, borders and highlighting cannot be written directly.

- A paragraph with a style is `<div class="Name">text</div>`, on one line.
- A table with a style is a line `<table class="Name">`, then a pipe table, then a line `</table>`. A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. Nothing else is between the two tag lines, not even a blank line.
- A paragraph or a pipe table without a tag has the default style.

The value of `class` is exactly one style name, written as listed, spaces included. A paragraph takes a paragraph style and a table a table style. `<div>` and `<table>` take no other attribute.

Example:

```
<div class="Note">신규 고객 34곳 중 21곳이 수도권.</div>

<table class="Grid Table 4">
| 지역 | 매출 |
|---|---|
| 수도권 | 1,204 |
</table>
```

## Names available in this file

Paragraph styles:
- `본문` — 기본 본문
- `본문 들여쓰기` — 10mm 들여 쓴 본문: 번호 항목 아래의 가., 나. 항목
- `본문 들여쓰기 2` — 20mm 들여 쓴 본문: 가., 나. 항목 아래의 1), 2) 항목
- `강조 문단` — 위아래 가는 선 사이의 굵은 문단: 공고의 핵심 문장
- `참고 문단` — 작은 회색 글씨의 참고 사항
- `붙임` — 문서 끝의 붙임 목록 줄
- `발신 명의` — 가운데 정렬된 큰 글씨: 공고 끝의 발신 기관장 명의

Table styles:
- `표 눈금` — 모든 칸에 가는 검은 실선
- `눈금 표 4` — 회색 머리글 행, 회색 줄무늬 행
- `눈금 표 4 - 강조색 1` — 파란 머리글 행, 연한 파란 줄무늬 행
- `일반 표 1` — 가는 가로선만

## The file: memo-3.hj.md

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---

# 2026년 하반기 소상공인 디지털 전환 지원사업 공고

<div class="본문">○○시는 소상공인의 디지털 전환을 돕기 위해 다음과 같이 지원사업 참여자를 모집합니다.</div>

<div class="강조 문단">신청 기간: 2026. 10. 5.(월) ~ 10. 23.(금) 18:00</div>

## 1. 지원 대상

<div class="본문">공고일 현재 ○○시에 사업자 등록을 하고 영업 중인 소상공인</div>

<div class="본문 들여쓰기">가. 상시 근로자 5인 미만 (제조업 등은 10인 미만)</div>

<div class="본문 들여쓰기">나. 2025년 매출액 10억 원 이하</div>

<div class="본문 들여쓰기">다. 지방세 체납이 없는 사업자</div>

<div class="참고 문단">휴업 또는 폐업 중인 사업자는 제외합니다.</div>

<div class="참고 문단">공동 대표 사업자는 대표자 1명이 신청합니다.</div>

## 2. 지원 내용

<div class="본문">선정된 사업자에게 다음 중 하나를 지원합니다.</div>

<div class="본문">가. 키오스크·테이블 주문 기기 도입비 (최대 300만 원)</div>

<div class="본문">나. 온라인 쇼핑몰 입점 및 상세페이지 제작 (최대 200만 원)</div>

<div class="본문 들여쓰기 2">1) 자부담 20%는 선정 후 납부</div>

<div class="본문 들여쓰기 2">2) 동일 품목을 이미 지원받은 사업자는 제외</div>

<div class="본문 들여쓰기 2">3) 기기 설치 후 3개월 안에 사용 실적 제출</div>

<div class="참고 문단">지원 한도를 넘는 금액은 사업자가 부담합니다.</div>

표 1. 지원 유형별 선정 규모

<table class="눈금 표 4">
| 지원 유형 | 선정 규모 | 지원 한도 | 자부담 |
|---|---|---|---|
| 키오스크·테이블 주문 | 120곳 | 300만 원 | 20% |
| 온라인 쇼핑몰 입점 | 80곳 | 200만 원 | 20% |
| 합계 | 200곳 | - | - |
</table>

## 3. 신청 방법

<div class="본문">○○시 누리집에서 온라인으로 신청하거나, 시청 민원실을 방문하여 신청합니다.</div>

<div class="본문">신청서 서식은 ○○시 누리집 공고 게시물에 첨부되어 있습니다.</div>

<div class="본문 들여쓰기">가. 온라인: ○○시 누리집 > 알림마당 > 공고</div>

<div class="본문 들여쓰기">나. 방문: 시청 본관 1층 민원실 (평일 09:00 ~ 18:00)</div>

<div class="참고 문단">방문 신청 시 대리인은 위임장과 신분증을 함께 제출합니다.</div>

신청 기한을 넘긴 신청서는 접수하지 않습니다.

표 2. 제출 서류

<table class="눈금 표 4">
| 구분 | 서류 | 비고 |
|---|---|---|
| 필수 | 참여 신청서 | 붙임 서식 1 |
| 필수 | 사업자등록증 사본 | - |
| 필수 | 개인정보 수집 동의서 | 붙임 서식 2 |
| 해당자 | 부가가치세 과세표준증명 | 2025년 귀속 |
</table>

## 4. 선정 및 발표

<div class="본문">서류 심사 후 2026. 11. 13.(금) ○○시 누리집에 발표합니다.</div>

<div class="본문 들여쓰기">가. 평가 항목: 디지털 활용 계획, 매출 규모, 사업 지속성</div>

<div class="본문 들여쓰기">나. 동점 시 매출액이 적은 사업자를 우선 선정</div>

표 3. 평가 배점

<table class="일반 표 1">
| 평가 항목 | 배점 |
|---|---|
| 디지털 활용 계획 | 50 |
| 매출 규모 | 30 |
| 사업 지속성 | 20 |
| 합계 | 100 |
</table>

<div class="참고 문단">선정 결과는 개별 문자로도 안내합니다.</div>

<div class="참고 문단">선정 결과에 이의가 있는 경우 발표일로부터 7일 안에 신청할 수 있습니다.</div>

## 5. 유의 사항

<div class="본문">다음에 해당하는 경우 선정을 취소하고 지원금을 환수합니다.</div>

<div class="본문 들여쓰기">가. 거짓이나 부정한 방법으로 지원을 받은 경우</div>

<div class="본문 들여쓰기">나. 지원받은 기기를 1년 안에 처분하거나 폐업한 경우</div>

<div class="본문 들여쓰기 2">1) 폐업 시에는 남은 기간에 비례하여 환수</div>

<div class="본문 들여쓰기 2">2) 천재지변 등 불가피한 사유는 심의 후 면제</div>

<div class="참고 문단">지원 기기는 선정일로부터 1년간 사후 관리 대상입니다.</div>

표 4. 추진 일정

<table class="눈금 표 4">
| 단계 | 기간 | 내용 |
|---|---|---|
| 신청 접수 | 10. 5. ~ 10. 23. | 온라인·방문 접수 |
| 서류 심사 | 10. 26. ~ 11. 6. | 자격 요건 확인 |
| 결과 발표 | 11. 13. | 누리집 게시, 개별 문자 안내 |
| 협약 체결 | 11. 16. ~ 11. 27. | 협약서 작성, 자부담금 납부 |
</table>

## 6. 문의

<table class="일반 표 1">
| 구분 | 담당 부서 | 연락처 |
|---|---|---|
| 사업 내용 | 일자리경제과 | 031-000-1234 |
| 온라인 신청 | 정보통신과 | 031-000-5678 |
</table>

<div class="참고 문단">문의 전화는 평일 09:00 ~ 18:00에 받습니다.</div>

<div class="붙임">붙임  1. 참여 신청서 1부.</div>

<div class="붙임">2. 개인정보 수집 동의서 1부.  끝.</div>

○○시장
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### memo3-w (write)

Write a short notice as a new document, in this order: a heading level 1 "2026년 하반기 전통시장 주차장 무료 개방 안내"; the ordinary body paragraph "○○시는 전통시장 이용 편의를 위해 공영주차장을 무료로 개방합니다."; the key sentence of the notice, "개방 기간: 2026. 11. 1. ~ 12. 31.", in the file's style for it; the ordinary body paragraph "대상 주차장은 다음과 같습니다."; the items "가. 중앙시장 공영주차장" and "나. 동부시장 공영주차장" in the style for items under a numbered point; the item "1) 1일 최대 3시간" in the style for items under 가. and 나.; a table with thin black lines around every cell, with the columns 주차장, 주차면 and the rows 중앙시장 / 120면 and 동부시장 / 85면; the attachment line "붙임  주차장 위치도 1부.  끝." in the file's style for attachment lines; and the sender's name "○○시장" as the last line, in the file's style for it.

Start the new file with this front matter:

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---
````

### memo3-e1 (edit)

Under "2. 지원 내용", the lines "가. 키오스크·테이블 주문 기기 도입비 (최대 300만 원)" and "나. 온라인 쇼핑몰 입점 및 상세페이지 제작 (최대 200만 원)" are items under the numbered point. Give both the file's style for such items. Change nothing else.

### memo3-e2 (edit)

Put the sentence "신청 기한을 넘긴 신청서는 접수하지 않습니다." in a red box so that applicants notice it.

### memo3-e3 (edit)

Give the table of 제출 서류 (under "3. 신청 방법") the version of its current style with a blue header row. Change nothing else.

### memo3-e4 (edit)

Center the last line, "○○시장", and make it large: it is the sender's name at the end of the notice.

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
