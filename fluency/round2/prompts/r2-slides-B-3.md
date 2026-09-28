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
- `제목 슬라이드`: title, body
- `제목 및 내용`: title, body
- `콘텐츠 2개`: title, left, right
- `구역 머리글`: title, body
- `제목만`: title

## The file: deck-3.hj.md

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---

layout: 제목 슬라이드
::title::
한빛유통 스마트 물류 솔루션 제안
::body::
㈜누리시스템 | 2026년 10월

---

layout: 제목 및 내용
::title::
제안 개요
::body::
- 현황 진단
- 제안 솔루션
- 기대 효과
- 추진 일정
- 투자 비용

---

layout: 제목 및 내용
::title::
제안사 소개
::body::
- ㈜누리시스템: 물류 자동화 전문 기업 (2009년 설립)
- 물류센터 자동화 구축 42건
- 유통·제조 고객 28곳
::notes::
회사 소개는 1분 이내

---

layout: 구역 머리글
::title::
현황 진단
::body::
한빛유통 물류센터 3곳

---

layout: 콘텐츠 2개
::title::
현황과 과제
::left::
- 현황: 수작업 분류 60%
- 현황: 출고 지연 월 120건
::right::
- 과제: 분류 자동화
- 과제: 실시간 재고 가시성
::notes::
현장 인터뷰 결과 기반

---

layout: 제목 및 내용
::title::
분류 작업 현황
::body::
- 수작업 분류 인력 60명 (3교대)
- 시간당 분류 1,800건
- 오분류율 1.4%
- 성수기 초과 근무 월 평균 38시간
::notes::
현장 인터뷰 결과 기반

---

layout: 제목 및 내용
::title::
센터별 현황
::body::
- 이천 센터: 일 출고 8,000건
- 용인 센터: 일 출고 5,500건
- 칠곡 센터: 일 출고 4,200건
<shape id="s5" name="출처">출처: 한빛유통 내부 자료 (2026. 9.)</shape>

---

layout: 제목 및 내용
::title::
고객 요구 사항
::body::
- 분류 작업 자동화로 인력 의존도 축소
- 센터 3곳 재고 실시간 통합 조회
- 기존 WMS와 연동 (추가 개발 최소화)
- 성수기 전 1단계 가동
- 현장 작업자 교육 지원
::notes::
9월 워크숍에서 확인한 요구 사항

---

layout: 구역 머리글
::title::
제안 솔루션
::body::
자동 분류 + 재고 관제

---

layout: 콘텐츠 2개
::title::
도입 전후 비교
::left::
- 도입 후
- 분류 인력 25명
- 출고 지연 월 20건 이하
::right::
- 도입 전
- 분류 인력 60명
- 출고 지연 월 120건
::notes::
인력 재배치 방안은 질문 시 설명

---

layout: 제목 및 내용
::title::
솔루션 구성
::body::
- 자동 분류기 (시간당 6,000건)
- 재고 관제 대시보드
- WMS 연동 모듈
- 모바일 검수 앱
::notes::
데모 영상 2분

---

layout: 콘텐츠 2개
::title::
적용 사례
::left::
- A사 물류센터
- 분류 인력 45% 절감
- 출고 지연 80% 감소
::right::
- B사 물류센터
- 재고 정확도 99.7%
- 구축 기간 4개월
::notes::
사례 고객사 이름은 공개 동의 후 사용
<shape id="s9" name="출처">출처: ㈜누리시스템 구축 실적 (2025)</shape>

---

layout: 콘텐츠 2개
::title::
도입 방식 비교
::left::
- 구매형
- 초기 투자 18억 원
- 설비 자산 보유
::right::
- 임대형
- 월 4,500만 원 (5년 약정)
- 유지보수 포함
::notes::
고객 선호 방식 확인 필요

---

layout: 제목 및 내용
::title::
기대 효과
::body::
- 분류 인력 60명 → 25명
- 출고 지연 월 120건 → 20건 이하
- 재고 정확도 97% → 99.5%
::notes::
인력 재배치 방안은 질문 시 설명

---

layout: 제목 및 내용
::title::
운영 지원 체계
::body::
- 전담 PM 1명, 현장 엔지니어 2명 상주
- 24시간 원격 모니터링
- 장애 시 4시간 이내 현장 출동
- 분기별 운영 보고서 제출
::notes::
인력 재배치 방안은 질문 시 설명

---

layout: 구역 머리글
::title::
추진 계획
::body::
일정과 비용

---

layout: 제목 및 내용
::title::
추진 일정
::body::
- 1단계 (11월): 이천 센터 구축
- 2단계 (1월): 용인 센터 확대
- 3단계 (3월): 칠곡 센터 확대 및 안정화

---

layout: 콘텐츠 2개
::title::
단계별 범위
::left::
- 1단계: 이천 센터
- 자동 분류기 2대
- 재고 관제 대시보드 구축
::right::
- 2~3단계: 용인·칠곡 센터
- 자동 분류기 센터별 1대
- 모바일 검수 앱 확대

---

layout: 콘텐츠 2개
::title::
투자 비용
::left::
- 초기 투자: 18억 원
- 연간 운영비: 1.2억 원
::right::
- 회수 기간: 2.5년
- 연간 절감: 7.4억 원
<shape id="s11" name="주석">VAT 별도, 3개 센터 기준</shape>

---

layout: 제목 및 내용
::title::
유지보수 조건
::body::
- 무상 유지보수 1년
- 이후 연 1.2억 원 (연간 운영비에 포함)
- 소프트웨어 업데이트 연 2회
- 부품 교체는 실비 청구

---

layout: 제목 및 내용
::title::
다음 단계
::body::
- 11월 1주: 현장 실사
- 11월 3주: 상세 견적 제출
- 12월: 계약 및 착수
::notes::
현장 실사 일정은 한빛유통 물류팀과 협의

---

layout: 콘텐츠 2개
::title::
협력 체계
::left::
- 한빛유통: 물류팀 (현장 운영)
- 한빛유통: 정보팀 (WMS 연동)
::right::
- ㈜누리시스템: 구축 PM
- ㈜누리시스템: 설비·SW 엔지니어
::notes::
주간 회의는 매주 화요일

---

layout: 제목만
::title::
감사합니다
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### deck3-w (write)

Write a new presentation with exactly these four slides, in this order:
1. Layout 제목 슬라이드: title "한빛유통 1단계 구축 결과 보고"; body "㈜누리시스템 | 2027년 1월".
2. Layout 콘텐츠 2개: title "목표 대비 결과"; left column bullets "목표: 분류 인력 25명" and "목표: 출고 지연 월 20건 이하"; right column bullets "결과: 분류 인력 28명" and "결과: 출고 지연 월 17건"; speaker notes "인력은 2월까지 목표 달성 예정".
3. Layout 제목 및 내용: title "2단계 계획"; body bullets "용인 센터 확대 (2월 착수)" and "교육 일정: 2월 첫째 주".
4. Layout 제목만: title "감사합니다".

Start the new file with this front matter:

````
---
type: presentation
format: pptx
template: org/sales-deck
schema: 1
---
````

### deck3-e1 (edit)

On the slide "도입 전후 비교", the columns are the wrong way round: the 도입 전 bullets belong in the left column and the 도입 후 bullets in the right column. Swap the contents of the two columns. Change nothing else.

### deck3-e2 (edit)

Move the slide "추진 일정" so that it comes right after the slide "투자 비용". Change nothing else.

### deck3-e3 (edit)

Replace the slide "투자 비용" with two slides that use the layout 제목 및 내용: first "투자 비용", whose body is the bullets of the left column, then "투자 회수", whose body is the bullets of the right column. The shape of the original slide stays on the first of the two.

### deck3-e4 (edit)

Remove the speaker notes from the slide "기대 효과". Every other slide keeps its notes.

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
