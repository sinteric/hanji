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
- `제목 슬라이드`: title, body
- `제목 및 내용`: title, body
- `콘텐츠 2개`: title, left, right
- `구역 머리글`: title, body
- `제목만`: title

## The file: deck-2.hj.md

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---

layout: 제목 슬라이드
::title::
2026년 3분기 KPI 리뷰
::body::
경영기획실 / 2026-10-06

---

layout: 제목 및 내용
::title::
전사 KPI 달성률
::body::
- 매출: 목표 대비 96%
- 영업이익: 목표 대비 102%
- 고객 만족도: 목표 대비 99%
::notes::
영업이익 초과 달성 강조

---

layout: 콘텐츠 2개
::title::
사업부별 달성률
::left::
- 가전사업부: 목표 대비 104%
- 부품사업부: 목표 대비 91%
::right::
- 서비스사업부: 목표 대비 96%
- 해외사업부: 목표 대비 88%
- 부품사업부 원가 상승 영향

---

layout: 구역 머리글
::title::
4분기 계획
::body::
미달 KPI 집중 관리

---

layout: 제목 및 내용
::title::
4분기 중점 관리 KPI
::body::
- 매출: 목표 대비 96% → 100%
- 해외사업부: 목표 대비 88% → 95%
- 고객 만족도: 목표 대비 99% → 100%
::notes::
해외사업부 담당 임원 별도 보고

---

layout: 제목만
::title::
Q&A
````

## Tasks

### deck2-w (write)

Write a new presentation with exactly three slides:
1. Layout 제목 슬라이드; title "신입사원 온보딩"; body text "인사팀 / 2026년 10월".
2. Layout 구역 머리글; title "첫 주 일정"; body text "적응 교육과 부서 배치".
3. Layout 콘텐츠 2개; title "담당자 안내"; left column bullets "교육: 인사팀 한지민", "장비: IT지원팀 오세훈"; right column bullets "급여: 재무팀 윤가은", "복지: 총무팀 서준호"; speaker notes "연락처는 사내 메신저로 공유".

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

### deck2-e1 (edit)

After slide 2 ("전사 KPI 달성률"), add a slide with layout 제목 및 내용: title "고객 만족도 세부"; body bullets "콜센터 응답률 92%", "불만 처리 기간 2.1일"; speaker notes "콜센터 인력 충원 효과".

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### deck2-e2 (edit)

On slide 3 ("사업부별 달성률"), the bullet "부품사업부 원가 상승 영향" is in the right column. Move it to the end of the left column.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### deck2-e3 (edit)

On slide 2 ("전사 KPI 달성률"), change the 매출 bullet from "목표 대비 96%" to "목표 대비 97%". The same figure on slides 3 and 5 stays as it is.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
