You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes.

### Slides

- Slides are separated by a line containing only `---`. The first slide begins right after the front matter.
- The first line of every slide is `layout: Name`, written as listed, spaces included (quotes allowed). There are no other `key: value` lines, and no `---` after the layout line.
- Then each slot begins with a marker line named after the slot: `::title::`, `::body::`, `::left::`, `::right::`, and `::notes::` for speaker notes (every layout has notes). Its text is on the lines after the marker, up to the next marker or `---`.
- Use only the slots of the slide's layout and leave out a slot you do not fill. All text is inside a slot.
- A shape of the file that is not a slot is its own line `<shape id="…" name="…">text</shape>` after the slots; like a marker, it ends the slot before it. You may change its text, move it or delete it, but never add a shape.

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
<shape id="s4" name="출처">출처: 내부 집계</shape>
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

layout: Title Slide
::title::
2026년 하반기 영업 전략
::body::
영업기획팀 | 2026년 7월 3일
::notes::
인사 후 30초 안에 핵심 요약

---

layout: Title and Content
::title::
목차
::body::
- 상반기 실적 요약
- 시장 환경
- 하반기 목표
- 실행 계획
- 요청 사항

---

layout: Section Header
::title::
상반기 실적 요약
::body::
1월 ~ 6월 누계

---

layout: Title and Content
::title::
상반기 핵심 지표
::body::
- 매출 1조 2,480억 원 (전년 대비 +12%)
- 영업이익 1,340억 원 (전년 대비 +9%)
- 신규 고객 412곳
- 고객 유지율 91%
::notes::
전년 대비 수치는 재무팀 확정치
<shape id="s4" name="출처">출처: 재무팀 결산 (2026. 7. 1.)</shape>

---

layout: Two Content
::title::
권역별 매출
::left::
- 수도권 5,120억 원
- 영남권 3,260억 원
- 충청권 1,480억 원
::right::
- 호남권 1,390억 원
- 강원권 610억 원
- 제주 620억 원
::notes::
권역 구분은 2026년 조직 기준
<shape id="s5" name="단위">단위: 억 원, 상반기 누계</shape>

---

layout: Title and Content
::title::
상반기 성과 요인
::body::
- 수도권 대형 거래처 3곳 신규 계약
- 온라인 채널 매출 +31%
- 생활용품 매출 전년 대비 +12%

---

layout: Two Content
::title::
상반기 채널별 매출
::left::
- 오프라인 7,740억 원 (전년 대비 +5%)
- 대형마트 3,900억 원
- 대리점 3,840억 원
::right::
- 온라인 4,740억 원 (전년 대비 +31%)
- 자사몰 2,100억 원
- 오픈마켓 2,640억 원
::notes::
온라인 비중 38%로 확대

---

layout: Section Header
::title::
시장 환경
::body::
하반기 전망

---

layout: Title and Content
::title::
하반기 시장 전망
::body::
- 소비 심리 완만한 회복 예상
- 원자재 가격 안정세
- 온라인 채널 경쟁 심화
- 경쟁사 A 가격 인하 (평균 -5%)
- 경쟁사 B 신제품 2종 출시 예정
- 물류비 상승 지속 (+4%)
- 환율 변동성 확대
- 정부 소비 촉진 정책 (10월)
::notes::
경쟁사 동향은 질문 시 설명

---

layout: Title and Content
::title::
경쟁사 동향
::body::
- 경쟁사 A: 가격 인하 (평균 -5%), 대형마트 행사 확대
- 경쟁사 B: 신제품 2종 출시 예정 (9월)
- 경쟁사 C: 온라인 전용 브랜드 출시
::notes::
경쟁사 동향은 질문 시 설명

---

layout: Section Header
::title::
하반기 계획
::body::
목표와 실행

---

layout: Title and Content
::title::
하반기 목표
::body::
- 매출 1조 3,600억 원 (전년 대비 +12%)
- 신규 고객 450곳
- 고객 유지율 92%
::notes::
목표는 이사회 승인 전 잠정치

---

layout: Title and Content
::title::
권역별 하반기 목표
::body::
- 수도권 5,600억 원 (전년 대비 +12%)
- 영남권 3,500억 원 (전년 대비 +9%)
- 기타 권역 4,500억 원 (전년 대비 +14%)
::notes::
목표는 이사회 승인 전 잠정치

---

layout: Two Content
::title::
채널별 전략
::left::
- 오프라인: 대형마트 입점 확대
- 오프라인: 지역 대리점 교육
::right::
- 온라인: 자사몰 멤버십 개편
- 온라인: 라이브 커머스 월 2회
::notes::
질문 시 설명

---

layout: Title and Content
::title::
고객 유지 전략
::body::
- 멤버십 등급 개편 (4단계 → 3단계)
- 재구매 고객 쿠폰 자동 발송
- 이탈 고객 전화 상담 (월 500곳)
::notes::
질문 시 설명

---

layout: Title and Content
::title::
실행 계획
::body::
- 7월: 조직 개편 및 목표 배분
- 8월: 신규 거래처 제안
- 9월: 추석 프로모션
- 10월 ~ 12월: 연말 성수기 대응
::notes::
질문 시 설명

---

layout: Title and Content
::title::
하반기 프로모션
::body::
- 8월: 여름 정기 세일 (온라인 단독)
- 9월: 추석 선물 세트 사전 예약
- 11월: 블랙프라이데이 기획전
- 12월: 연말 멤버십 감사 행사
::notes::
질문 시 설명

---

layout: Title Only
::title::
요청 사항 요약
<shape id="s12" name="요약표">요청 사항 3건: 인력 2명, 예산 5억 원, 시스템 1건</shape>

---

layout: Title and Content
::title::
요청 사항
::body::
- 영업 인력 2명 충원
- 판촉 예산 5억 원 추가
- CRM 시스템 고도화
::notes::
질문 시 설명

---

layout: Two Content
::title::
기대 효과
::left::
- 매출 전년 대비 +12%
- 신규 고객 450곳
::right::
- 고객 유지율 92%
- 영업이익률 11%
<shape id="s16" name="주석">하반기 목표 기준 추정치</shape>

---

layout: Title Slide
::title::
감사합니다
::body::
문의: 영업기획팀 (내선 2314)
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### deck1-w (write)

Write a new presentation with exactly these four slides, in this order:
1. Layout Title Slide: title "2026년 4분기 영업 계획"; body "영업기획팀 | 2026년 10월 2일".
2. Layout Title and Content: title "4분기 목표"; body bullets "매출 3,900억 원" and "신규 고객 120곳"; speaker notes "목표는 10월 경영회의에서 확정".
3. Layout Two Content: title "채널별 과제"; left column bullets "오프라인: 연말 매대 확보" and "오프라인: 대리점 판촉 지원"; right column bullets "온라인: 블랙프라이데이 기획전" and "온라인: 멤버십 전환 캠페인".
4. Layout Title Only: title "질의응답".

Start the new file with this front matter:

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---
````

### deck1-e1 (edit)

Split the slide "하반기 시장 전망" into two slides with the same layout. The first keeps the title and the first four bullets; the second, right after it, has the title "하반기 시장 전망 (계속)" and the other four bullets. The speaker notes stay with the first slide only.

### deck1-e2 (edit)

On the slide "권역별 매출", the left column should list only 수도권 and 영남권. Move the bullet "충청권 1,480억 원" to the right column, as its last bullet. Change nothing else.

### deck1-e3 (edit)

On the slide "실행 계획", replace the speaker notes with "9월 추석 프로모션은 마케팅팀과 공동 진행". Leave the notes of every other slide as they are.

### deck1-e4 (edit)

Right after the slide "하반기 목표", add a slide with the two-column layout: title "목표 대비 리스크"; left column bullets "경쟁사 가격 인하" and "물류비 상승"; right column bullets "대응: 번들 상품 확대" and "대응: 물류 거점 통합"; speaker notes "리스크 대응 방안은 8월 중 확정".

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
