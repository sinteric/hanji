You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. Slide text is Markdown: one line per paragraph, `- ` bullets, `**bold**`. Each slide is built from one of the file's layouts, listed with the file; a layout has named slots, and no positions or sizes are ever written. Besides what is described below, there are no other tags, markers or attributes.

### Slides

- A slide is `<slide layout="Name">` … `</slide>`. The layout name is written as listed, spaces included. Every `<slide>` is closed by `</slide>` before the next slide begins.
- Inside, each slot is a tag named after the slot: `<title>`, `<body>`, `<left>`, `<right>`, and `<notes>` for speaker notes (every layout has notes). A slot's text is either on one line, `<title>핵심 지표</title>`, or on the lines between `<body>` and `</body>`.
- Use only the slots of the slide's layout and leave out a slot you do not fill. All text is inside a slot.
- A shape of the file that is not a slot is one line `<shape id="…" name="…">text</shape>` inside the slide, outside every slot tag. You may change its text, move it or delete it, but never add a shape.

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
<shape id="s4" name="출처">출처: 내부 집계</shape>
</slide>
```

## Names available in this file

Layouts and their slots (every layout also has `notes`):
- `Title Slide`: title, body
- `Title and Content`: title, body
- `Two Content`: title, left, right
- `Section Header`: title, body
- `Title Only`: title

## The file: deck-2.hj.md

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---

<slide layout="Title Slide">
<title>2026년 3분기 KPI 리뷰</title>
<body>경영기획실 | 2026. 10. 8.</body>
</slide>

<slide layout="Title and Content">
<title>리뷰 순서</title>
<body>
- 전사 KPI 요약
- 사업부별 실적
- 고객 지표
- 4분기 과제
</body>
</slide>

<slide layout="Title and Content">
<title>리뷰 기준</title>
<body>
- 기간: 2026년 7월 ~ 9월
- 실적: 사업부 보고 잠정치
- 목표: 2026년 사업계획 (1월 확정)
</body>
</slide>

<slide layout="Section Header">
<title>전사 KPI 요약</title>
<body>3분기 누계 기준</body>
</slide>

<slide layout="Title and Content">
<title>전사 KPI</title>
<body>
- 매출: 목표 대비 96%
- 영업이익: 목표 대비 102%
- 신규 수주: 목표 대비 88%
- 고객 만족도: 4.3 / 5.0
</body>
<shape id="s4" name="기준">기준: 3분기 누계, 잠정치</shape>
</slide>

<slide layout="Two Content">
<title>목표 대비 실적</title>
<left>- 목표 달성: 영업이익, 고객 만족도</left>
<right>- 목표 미달: 매출, 신규 수주</right>
<notes>미달 항목은 사업부별 실적에서 설명</notes>
</slide>

<slide layout="Title and Content">
<title>3분기 주요 이슈</title>
<body>
- 원자재 가격 상승 (+7%)
- 완성품 신제품 2종 조기 출시
- 서비스 계약 갱신률 94% 유지
- 부품 수주 2건 4분기로 이월
</body>
<notes>이월 수주는 4분기 매출에 반영</notes>
</slide>

<slide layout="Section Header">
<title>사업부별 실적</title>
<body>3분기 누계 기준</body>
</slide>

<slide layout="Title and Content">
<title>부품사업부</title>
<body>
- 매출: 목표 대비 91%
- 원가 상승 영향 (원자재 +7%)
- 신규 수주: 목표 대비 80%
</body>
<notes>원가 상승은 4분기까지 지속 전망</notes>
</slide>

<slide layout="Title and Content">
<title>완성품사업부</title>
<body>
- 매출: 목표 대비 101%
- 신제품 2종 조기 출시
- 신규 수주: 목표 대비 95%
</body>
<notes>질문 시 상세 설명</notes>
</slide>

<slide layout="Title and Content">
<title>서비스사업부</title>
<body>
- 매출: 목표 대비 98%
- 유지보수 계약 갱신률 94%
- 신규 수주: 목표 대비 90%
</body>
<notes>질문 시 상세 설명</notes>
</slide>

<slide layout="Two Content">
<title>사업부별 영업이익</title>
<left>
- 부품: 목표 대비 94%
- 완성품: 목표 대비 108%
</left>
<right>
- 서비스: 목표 대비 103%
- 전사: 목표 대비 102%
</right>
<notes>영업이익은 원가 절감 효과 반영</notes>
</slide>

<slide layout="Title and Content">
<title>사업부별 인력</title>
<body>
- 부품사업부 412명 (전년 대비 -3%)
- 완성품사업부 538명 (전년 대비 +2%)
- 서비스사업부 297명 (전년 대비 +5%)
</body>
<notes>질문 시 상세 설명</notes>
</slide>

<slide layout="Title and Content">
<title>재고 지표</title>
<body>
- 재고 회전율: 목표 6.0회, 실적 5.2회
- 장기 재고: 전년 대비 +8%
- 결품률: 1.2% (목표 1.0%)
</body>
<notes>질문 시 상세 설명</notes>
</slide>

<slide layout="Section Header">
<title>고객 지표</title>
<body>3분기 누계 기준</body>
</slide>

<slide layout="Two Content">
<title>고객 만족도</title>
<left>
- 3분기: 4.3 / 5.0
- 2분기: 4.1 / 5.0
</left>
<right>
- 불만 접수: 132건
- 평균 처리: 2.4일
</right>
<notes>질문 시 상세 설명</notes>
<shape id="s11" name="출처">출처: 고객지원센터 월간 보고</shape>
</slide>

<slide layout="Title and Content">
<title>불만 유형</title>
<body>
- 배송 지연 49%
- 제품 불량 22%
- 환불 요청 21%
- 기타 8%
</body>
<notes>질문 시 상세 설명</notes>
<shape id="s14" name="출처">출처: 고객지원센터 월간 보고</shape>
</slide>

<slide layout="Two Content">
<title>채널별 불만 처리</title>
<left>
- 오프라인: 평균 2.5일
- 오프라인 목표: 2.0일
</left>
<right>
- 온라인: 평균 1.9일
- 온라인 목표: 1.5일
- 채팅 비중 28%
</right>
<notes>질문 시 상세 설명</notes>
</slide>

<slide layout="Section Header">
<title>4분기 계획</title>
<body>과제와 일정</body>
</slide>

<slide layout="Title and Content">
<title>4분기 과제</title>
<body>
- 부품사업부 원가 절감 TF 운영
- 신규 수주 파이프라인 점검 (주 1회)
- 완성품 신제품 판촉 강화
- 서비스 계약 갱신 캠페인
- 고객 불만 처리 1.5일 이내
- 재고 회전율 개선
- 인력 재배치 검토
</body>
<notes>과제별 담당 임원은 다음 주 확정</notes>
</slide>

<slide layout="Title and Content">
<title>4분기 일정</title>
<body>
- 10월: 원가 절감 TF 구성
- 11월: 수주 파이프라인 중간 점검
- 12월: 연간 실적 예비 집계
</body>
<notes>일정은 경영회의 후 확정</notes>
</slide>

<slide layout="Two Content">
<title>요청 사항</title>
<left>- 원가 절감 TF 인력 3명</left>
<right>- 재고 관리 시스템 개선 예산 2억 원</right>
<notes>질문 시 상세 설명</notes>
</slide>

<slide layout="Title Only">
<title>Q&A</title>
</slide>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### deck2-w (write)

Write a new presentation with exactly these four slides, in this order:
1. Layout Section Header: title "4분기 과제 점검"; body "11월 경영회의".
2. Layout Two Content: title "과제 진행 현황"; left column bullets "완료: 원가 절감 TF 구성" and "완료: 갱신 캠페인 착수"; right column bullets "진행 중: 파이프라인 점검" and "지연: 재고 회전율 개선"; speaker notes "지연 과제는 담당 임원이 설명".
3. Layout Title and Content: title "다음 달 계획"; body bullets "재고 회전율 개선안 보고", "인력 재배치안 확정" and "12월 경영회의 준비".
4. Layout Title Only: title "Q&A".

Start the new file with this front matter:

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---
````

### deck2-e1 (edit)

Move the slide "고객 만족도" (with everything on it) so that it comes right after the slide "전사 KPI", before "목표 대비 실적". Change nothing else.

### deck2-e2 (edit)

At the very end of the deck, after "Q&A", add a slide with the two-column layout: title "부록: 사업부별 수주 현황"; left column bullets "부품: 목표 대비 80%" and "완성품: 목표 대비 95%"; right column bullets "서비스: 목표 대비 90%" and "전사: 목표 대비 88%".

### deck2-e3 (edit)

Split the slide "4분기 과제" into two slides with the same layout. The first keeps the title and the first four bullets; the second, right after it, has the title "4분기 과제 (계속)" and the remaining three bullets. The speaker notes move to the second slide; the first has none.

### deck2-e4 (edit)

Give the slide "전사 KPI" the speaker notes "수치는 잠정치, 10월 20일 확정". Change nothing else.

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
