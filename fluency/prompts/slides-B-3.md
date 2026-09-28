You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and four tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes.

### Slides

- Slides are separated by a line containing only `---`. The first slide begins right after the front matter.
- The first line of every slide is `layout: Name`, written as listed, spaces included. There are no other `key: value` lines.
- Then each slot begins with a marker line named after the slot: `::title::`, `::body::`, `::left::`, `::right::`, and `::notes::` for speaker notes (every layout has notes). Its text is on the lines after the marker, up to the next marker or `---`.
- Use only the slots of the slide's layout and leave out a slot you do not fill. All text is inside a slot.

Example (front matter and two slides):

```
---
type: presentation
format: pptx
template: org/deck
schema: 1
---

layout: Title and Content
::title::
핵심 지표
::body::
- 매출 **12% 증가**
- 신규 고객 34곳
::notes::
전년 대비 강조

---

layout: Two Content
::title::
지역별 현황
::left::
- 수도권 21곳
::right::
- 지방 13곳
```

## Names available in this file

Layouts and their slots (every layout also has `notes`):
- `Title Slide`: title, body
- `Title and Content`: title, body
- `Two Content`: title, left, right
- `Section Header`: title, body
- `Title Only`: title

## The file: deck-3.hj.md

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---

layout: Title Slide
::title::
한빛유통 물류 자동화 제안
::body::
스마트로지스 영업1팀 / 2026-10-12

---

layout: Title and Content
::title::
고객 현황
::body::
- 일 출고량 12,000박스
- 피크 시즌 출고 지연 7%
- 수작업 분류 인력 60명

---

layout: Title and Content
::title::
제안 개요
::body::
- 자동 분류기 2기 도입
- 출고 지연 2% 이하
- 분류 인력 60명 → 25명
::notes::
인력 재배치 방안은 질문 시 설명

---

layout: Two Content
::title::
도입 전후 비교
::left::
- 도입 전
- 출고 지연 7%
- 분류 인력 60명
::right::
- 도입 후
- 출고 지연 2% 이하
- 분류 인력 25명

---

layout: Title and Content
::title::
투자 및 회수
::body::
- 총 투자비 18억 원
- 연간 절감액 5억 원
- 회수 기간 3.6년
::notes::
리스 조건 별도 안내 가능

---

layout: Title Only
::title::
감사합니다
````

## Tasks

### deck3-w (write)

Write a new presentation with exactly three slides:
1. Layout Title Slide; title "고객 세미나 결과 보고"; body text "마케팅팀".
2. Layout Title and Content; title "참석 현황"; body bullets "참석 기업 42곳", "참석자 118명", "만족도 4.6점".
3. Layout Title and Content; title "후속 조치"; body bullets "상담 요청 기업 12곳 방문", "발표 자료 공유"; speaker notes "방문 일정은 영업팀과 조율".

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

### deck3-e1 (edit)

Before the last slide ("감사합니다"), add a slide with layout Section Header: title "다음 단계"; body text "현장 실사 일정 협의 (10월 넷째 주)".

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### deck3-e2 (edit)

On slide 3 ("제안 개요"), move the speaker note "인력 재배치 방안은 질문 시 설명" into the body as a new last bullet; the slide then has no notes.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### deck3-e3 (edit)

On slide 4 ("도입 전후 비교"), change "출고 지연 2% 이하" in the right column to "출고 지연 1.5% 이하". Slide 3 keeps its figure.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
