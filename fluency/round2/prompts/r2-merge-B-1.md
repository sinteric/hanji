You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Tables

Every table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. This holds for merged tables too.

Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column.
- A merged area is a rectangle. For two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. An empty cell that is not merged is written with a space, `|  |`.
- The text of a merged cell is written once, in its top-left cell; covered cells hold only a marker. `^^` never appears in the first row, and `||` never starts a row.

Example ("서울" covers two rows, "합계" covers two columns):

```
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```

## Names available in this file

This file uses no named styles or layouts.

## The file: report-1.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 2026년 3분기 지역별 영업 실적 보고

작성: 영업기획팀 / 보고일: 2026-10-05 / 배포: 영업본부장, 각 권역장

## 1. 요약

3분기 전사 매출은 6,203백만 원으로 전년 동기 대비 4.7% 증가했다. 수도권과 충청권이 성장을 이끌었고, 호남권은 전년 수준에 머물렀다.

- 수도권: 2,637백만 원 (전년 동기 대비 +6.5%)
- 영남권: 1,946백만 원 (전년 동기 대비 +3.6%)
- 호남권: 788백만 원 (전년 동기 대비 +0.6%)
- 충청권: 832백만 원 (전년 동기 대비 +6.3%)

## 2. 권역별 월별 매출 (단위: 백만 원)

| 권역 | 지점 | 7월 | 8월 | 9월 | 합계 |
|---|---|---|---|---|---|
| 수도권 | 서울 본점 | 412 | 398 | 455 | 1,265 |
| ^^ | 경기 지점 | 287 | 301 | 296 | 884 |
| ^^ | 인천 지점 | 156 | 162 | 170 | 488 |
| 영남권 | 부산 지점 | 301 | 287 | 312 | 900 |
| ^^ | 대구 지점 | 198 | 205 | 211 | 614 |
| ^^ | 울산 지점 | 143 | 150 | 139 | 432 |
| 호남권 | 광주 지점 | 162 | 156 | 170 | 488 |
| ^^ | 전주 지점 | 97 | 104 | 99 | 300 |
| 충청권 | 대전 지점 | 176 | 181 | 190 | 547 |
| ^^ | 청주 지점 | 88 | 95 | 102 | 285 |
| 전사 합계 || 2,020 | 2,039 | 2,144 | 6,203 |

8월 수치는 반품 정산 후 확정치이다.

9월에는 추석 선물 세트 판매로 수도권과 영남권 매출이 크게 늘었다.

권역별 수치는 지점 확정 매출 기준이며, 본사 직판 매출은 서울 본점에 포함했다.

## 3. 권역별 전년 대비 실적 (단위: 백만 원)

| 권역 | 지점 | 3분기 매출 || 증감률 |
|---|---|---|---|---|
| ^^ | ^^ | 전년 | 금년 | ^^ |
| 수도권 | 서울 본점 | 1,180 | 1,265 | +7.2% |
| ^^ | 경기 지점 | 842 | 884 | +5.0% |
| ^^ | 인천 지점 | 455 | 488 | +7.3% |
| 영남권 | 부산 지점 | 861 | 900 | +4.5% |
| ^^ | 대구 지점 | 590 | 614 | +4.1% |
| ^^ | 울산 지점 | 428 | 432 | +0.9% |
| 호남권 | 광주 지점 | 482 | 488 | +1.2% |
| ^^ | 전주 지점 | 301 | 300 | -0.3% |
| 충청권 | 대전 지점 | 512 | 547 | +6.8% |
| ^^ | 청주 지점 | 271 | 285 | +5.2% |
| 전사 합계 || 5,922 | 6,203 | +4.7% |

증감률은 전년 동기 대비이며 소수점 둘째 자리에서 반올림했다.

울산 지점은 부산 지점과의 통합을 앞두고 영업 인력을 재배치하고 있다.

## 4. 제품군별 매출 (단위: 백만 원)

| 제품군 | 품목 | 수도권 | 영남권 | 호남권 | 충청권 |
|---|---|---|---|---|---|
| 생활용품 | 세제 | 820 | 610 | 240 | 250 |
| ^^ | 주방용품 | 612 | 488 | 170 | 180 |
| 식품 | 음료 | 540 | 402 | 150 | 160 |
| ^^ | 간편식 | 455 | 301 | 120 | 130 |
| 신제품 | 건강기능식품 | 128 | 88 | 64 | 72 |
| ^^ | 반려동물용품 | 82 | 57 | 44 | 40 |

신제품은 7월 출시 품목으로, 권역별 수치는 잠정치이다.

생활용품과 식품이 전체 매출의 75%를 차지했으며, 신제품 비중은 7% 수준이다.

## 5. 주요 거래처 현황

| 거래처 | 담당 | 계약 상태 | 비고 |
|---|---|---|---|
| 한빛유통 | 김민수 | 갱신 완료 | 3년 계약 |
| 대성상사 | ^^ | 갱신 협의 중 | 단가 조정 요청 |
| 동해물산 | 이서연 | 신규 | 9월 첫 발주 |
| 서해식품 | ^^ | 갱신 협의 중 | 단가 조정 요청 |
| 남부상회 | 박준호 | 갱신 완료 | 2년 계약 |

갱신 협의 중인 2곳은 모두 단가 조정을 요청했으며, 10월 말까지 대응 방안을 마련한다.

## 6. 권역별 영업 인력 (9월 말 기준)

| 권역 | 지점 | 정원 | 현원 | 비고 |
|---|---|---|---|---|
| 수도권 | 서울 본점 | 18 | 17 | 충원 예정 |
| ^^ | 경기 지점 | 12 | 12 | - |
| ^^ | 인천 지점 | 8 | 7 | 충원 예정 |
| 영남권 | 부산 지점 | 11 | 11 | - |
| ^^ | 대구 지점 | 8 | 8 | - |
| ^^ | 울산 지점 | 6 | 5 | 충원 예정 |
| 호남권 | 광주 지점 | 7 | 7 | - |
| ^^ | 전주 지점 | 5 | 5 | - |
| 충청권 | 대전 지점 | 7 | 6 | 충원 예정 |
| ^^ | 청주 지점 | 5 | 5 | - |
| 합계 || 87 | 83 | 4명 충원 예정 |

10월 중 수도권 2명, 영남권 1명, 충청권 1명을 충원한다.

## 7. 권역장 의견

- 수도권: 대형마트 입점 확대로 4분기에도 성장세 유지 전망
- 영남권: 부산 지점 물류 거점 이전 후 배송 지연 감소
- 호남권: 판촉 행사 효과는 10월 이후 반영될 전망
- 충청권: 청주 지점 신규 거래처 2곳 계약 진행 중
- 공통: 4분기 물류비 인상분의 단가 반영 여부 결정 필요

## 8. 향후 계획

- 4분기 수도권 신규 거래처 5곳 확보
- 호남권 판촉 행사 2회 진행 (광주, 전주)
- 단가 조정 요청 거래처 대응 방안 수립 (10월 말)
- 청주 지점 신규 거래처 계약 마무리 (11월)
- 신제품 권역별 판매 목표 수립 (11월 초)

첨부: 지점별 월간 실적표 1부.

문의: 영업기획팀 김지현 (내선 2314)
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### report1-w (write)

Write a new document: a heading level 1 "2026년 하반기 신입사원 교육 일정"; a paragraph "대상: 2026년 하반기 입사자 24명 / 주관: 인사팀"; then one table with the columns 구분, 과정, 일시, 장소 for these sessions, in this order (구분 / 과정 / 일시 / 장소):
- 공통 교육 / 회사 소개 / 10월 6일 오전 / 본사 대강당
- 공통 교육 / 정보보안 / 10월 6일 오후 / 본사 대강당
- 공통 교육 / 윤리 경영 / 10월 7일 오전 / 본사 3층 교육장
- 직무 교육 / 영업 실무 / 10월 8일 / 영업본부 회의실
- 직무 교육 / 품질 관리 / 10월 9일 / 생산본부 회의실
In the table, each 구분 should appear once for all of its sessions, and where consecutive sessions are in the same place, that 장소 should appear once for them; no other cells are combined. The table ends with one more row whose only content, "교육 불참 시 팀장 승인 필요", runs across the whole width of the table. After the table, a paragraph "문의: 인사팀 교육담당 (내선 1102)".

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### report1-e1 (edit)

수도권 opened a new branch. In the table under "2. 권역별 월별 매출 (단위: 백만 원)", add 수원 지점 as the last 수도권 branch, right after 인천 지점, with 7월 64, 8월 71, 9월 83 and 합계 218. It belongs to 수도권 like the other 수도권 branches. Leave the 전사 합계 row as it is.

### report1-e2 (edit)

In the table under "3. 권역별 전년 대비 실적 (단위: 백만 원)", remove the row of 부산 지점 (the branch was merged into 울산 지점). 영남권 must still label its remaining branches. Change nothing else.

### report1-e3 (edit)

In the table under "3. 권역별 전년 대비 실적 (단위: 백만 원)", add a 목표 column to the 3분기 매출 group, as its last column right after 금년, so that 3분기 매출 stands over 전년, 금년 and 목표. The 목표 values: 서울 본점 1,250; 경기 지점 900; 인천 지점 480; 부산 지점 920; 대구 지점 600; 울산 지점 450; 광주 지점 500; 전주 지점 310; 대전 지점 530; 청주 지점 290; 전사 합계 6,230.

### report1-e4 (edit)

In the table under "4. 제품군별 매출 (단위: 백만 원)", the 호남권 and 충청권 figures of the two 신제품 items are not reported separately yet. Replace those four figures with a single cell reading "합산 집계 중". Change nothing else.

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
