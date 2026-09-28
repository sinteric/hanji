You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and four tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes.

### Slides

- A slide is `<slide layout="Name">` … `</slide>`. The layout name is written as listed, spaces included.
- Inside, each slot is a tag named after the slot: `<title>`, `<body>`, `<left>`, `<right>`, and `<notes>` for speaker notes (every layout has notes). A slot's text is either on one line, `<title>핵심 지표</title>`, or on the lines between `<body>` and `</body>`.
- Use only the slots of the slide's layout and leave out a slot you do not fill. All text is inside a slot.

Example (front matter and two slides):

```
---
type: presentation
format: pptx
template: org/deck
schema: 1
---

<slide layout="Title and Content">
<title>핵심 지표</title>
<body>
- 매출 **12% 증가**
- 신규 고객 34곳
</body>
<notes>전년 대비 강조</notes>
</slide>

<slide layout="Two Content">
<title>지역별 현황</title>
<left>- 수도권 21곳</left>
<right>- 지방 13곳</right>
</slide>
```

## Names available in this file

Layouts and their slots (every layout also has `notes`):
- `Title Slide`: title, body
- `Title and Content`: title, body
- `Two Content`: title, left, right
- `Section Header`: title, body
- `Title Only`: title

## The file: deck-1.hj.md

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---

<slide layout="Title Slide">
<title>2026 하반기 영업 전략</title>
<body>영업본부 / 2026년 7월</body>
</slide>

<slide layout="Title and Content">
<title>상반기 실적 요약</title>
<body>
- 매출 482억 원, 전년 대비 +12%
- 신규 고객 34곳 확보
- 재계약률 91%
</body>
<notes>전년 대비 성장률을 먼저 강조</notes>
</slide>

<slide layout="Section Header">
<title>시장 환경</title>
<body>경쟁 심화와 가격 압박</body>
</slide>

<slide layout="Two Content">
<title>경쟁사 비교</title>
<left>
- 당사: 전년 대비 +12%
- 평균 납기 3일
- 전담 CS 운영
</left>
<right>
- A사: 전년 대비 +8%
- 평균 납기 5일
- 가격 할인 공세
</right>
</slide>

<slide layout="Title and Content">
<title>하반기 목표</title>
<body>
- 매출 530억 원, 전년 대비 +12%
- 신규 고객 40곳
- 재계약률 93%
</body>
<notes>목표치는 경영회의 승인 완료</notes>
</slide>

<slide layout="Title Only">
<title>감사합니다</title>
</slide>
````

## Tasks

### deck1-w (write)

Write a new presentation with exactly three slides:
1. Layout Title Slide; title "4분기 영업 회의"; body text "영업기획팀".
2. Layout Title and Content; title "4분기 중점 과제"; body with the bullets "연말 재고 소진", "대형 거래처 재계약", "신규 대리점 교육"; speaker notes "재고 소진이 최우선".
3. Layout Two Content; title "지역별 담당"; left column bullets "수도권: 김민수", "영남권: 이서연"; right column bullets "호남권: 박지훈", "제주: 최유진".

Start the new file with this front matter:

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---
````

Answer with `text`: the complete new file.

### deck1-e1 (edit)

After slide 5 ("하반기 목표") and before the last slide, add a slide with layout Two Content: title "실행 과제"; left column bullets "수도권 대리점 3곳 추가", "온라인 견적 채널 오픈"; right column bullets "재계약 고객 할인 5%", "분기별 고객 세미나".

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### deck1-e2 (edit)

On slide 2 ("상반기 실적 요약"), move the bullet "- 재계약률 91%" out of the body and into the speaker notes, as a new last line of the notes (keep it as the bullet line "- 재계약률 91%").

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### deck1-e3 (edit)

On slide 5 ("하반기 목표"), change "전년 대비 +12%" to "전년 대비 +10%". The same phrase on other slides stays as it is.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
