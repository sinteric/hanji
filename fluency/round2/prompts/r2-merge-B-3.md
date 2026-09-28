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

## The file: report-3.hj.md

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---

# 2027년도 정보화사업 예산 요구서

제출 부서: ○○시 스마트도시국 / 제출일: 2026. 9. 30. / 담당: 정보화기획팀

## 1. 요구 개요

2027년도 정보화사업 예산은 총 4,860백만 원으로, 2026년 대비 370백만 원(8.2%) 증액을 요구한다.

- 증액 사유: 노후 장비 교체, 스마트 교통 확대, 행정정보 DB 이중화
- 감액 사업: 민원 키오스크 유지보수 (계약 만료), 돌봄 서비스 플랫폼 (구축 완료)

요구액 중 국비 보조는 1,210백만 원, 시비는 3,650백만 원이다.

## 2. 분야별 사업 예산 (단위: 백만 원)

| 분야 | 사업명 | 담당 부서 | 2026 예산 | 2027 요구 | 증감 |
|---|---|---|---|---|---|
| 행정 | 전자결재 고도화 | 기획예산과 | 320 | 410 | +90 |
| ^^ | 민원 키오스크 유지보수 | ^^ | 180 | 120 | -60 |
| ^^ | 행정정보 DB 이중화 | ^^ | 250 | 300 | +50 |
| 교통 | 스마트 교차로 확대 | 교통정책과 | 900 | 1,150 | +250 |
| ^^ | 버스정보시스템 교체 | ^^ | 620 | 700 | +80 |
| 안전 | CCTV 통합관제 증설 | 안전총괄과 | 1,050 | 1,120 | +70 |
| ^^ | 재난문자 연계 개선 | 재난대응과 | 170 | 160 | -10 |
| 복지 | 돌봄 서비스 플랫폼 | 복지정책과 | 1,000 | 900 | -100 |
| 합계 ||| 4,490 | 4,860 | +370 |

증감은 2027 요구액에서 2026 예산을 뺀 금액이다.

## 3. 분기별 집행 계획 (단위: 백만 원)

| 분야 | 상반기 || 하반기 || 연간 |
|---|---|---|---|---|---|
| ^^ | 1분기 | 2분기 | 3분기 | 4분기 | ^^ |
| 행정 | 210 | 250 | 230 | 140 | 830 |
| 교통 | 450 | 520 | 480 | 400 | 1,850 |
| 안전 | 300 | 340 | 360 | 280 | 1,280 |
| 복지 | 200 | 250 | 250 | 200 | 900 |
| 합계 | 1,160 | 1,360 | 1,320 | 1,020 | 4,860 |

집행 계획은 조달 일정과 국비 교부 시기를 반영했다.

교통 분야는 2분기에 스마트 교차로 공사가 몰려 집행액이 가장 크다.

복지 분야는 운영비만 남아 분기별 집행액이 고르게 나뉜다.

## 4. 인건비 및 운영비 (단위: 백만 원)

| 직급 | 구분 | 인원 | 금액 |
|---|---|---|---|
| 5급 | 정규직 | 2 | 180 |
| 6급 | 정규직 | 4 | 280 |
| 6급 | 기간제 | 1 | 55 |
| 7급 | 정규직 | 3 | 180 |
| 7급 | 기간제 | 2 | 96 |
| 운영비 ||| 145 |

기간제 인원은 2027년 1월 채용 예정이며, 인건비는 12개월분으로 산정했다.

## 5. 추진 일정

| 단계 | 기간 | 내용 |
|---|---|---|
| 준비 | 2027. 1. ~ 2. | 사업계획 수립, 발주 준비 |
| ^^ | 2027. 3. | 조달 발주 |
| 구축 | 2027. 4. ~ 9. | 시스템 구축 및 시험 |
| 운영 | 2027. 10. ~ 12. | 안정화 및 성과 점검 |

단계별 일정은 조달 발주 결과에 따라 조정될 수 있다.

## 6. 성과 지표

| 분야 | 지표 | 2026 목표 | 2027 목표 | 측정 방법 |
|---|---|---|---|---|
| 행정 | 전자결재 처리 시간 | 2.0일 | 1.5일 | 시스템 로그 |
| ^^ | 민원 온라인 처리율 | 75% | 80% | 민원 통계 |
| 교통 | 교차로 평균 대기 시간 | 90초 | 80초 | 신호 제어 데이터 |
| ^^ | 버스 도착 정보 정확도 | 92% | 95% | 표본 조사 |
| 안전 | 관제 CCTV 대수 | 1,800대 | 2,100대 | 관제센터 집계 |
| 복지 | 돌봄 플랫폼 이용자 | 3,000명 | 4,000명 | 가입자 통계 |

성과 지표는 2027년 12월 말 기준으로 측정해 2028년 2월에 공개한다.

## 7. 사업별 요구 사유

전자결재 고도화: 2016년 도입한 전자결재 시스템을 클라우드로 전환하고 모바일 결재 기능을 추가한다.

스마트 교차로 확대: 2026년 시범 운영한 12개 교차로에서 대기 시간이 평균 14% 줄어, 2027년 30개소로 확대한다.

CCTV 통합관제 증설: 어린이 보호구역과 하천변 사각지대 해소를 위해 300대를 추가한다.

민원 키오스크 유지보수: 2026년 12월에 계약이 끝나고, 2027년에는 무상 유지보수 기간이 적용된다.

행정정보 DB 이중화: 재해 복구 센터에 실시간 복제 DB를 구축한다.

버스정보시스템 교체: 2015년 설치한 정류장 안내기 180대 중 120대를 교체한다.

재난문자 연계 개선: 기존 연계 모듈의 유지보수 계약을 단가 인하 조건으로 갱신한다.

돌봄 서비스 플랫폼: 구축이 완료되어 2027년에는 운영비만 요구한다.

## 8. 연차별 투자 계획 (단위: 백만 원)

| 분야 | 사업명 | 2027 | 2028 | 2029 |
|---|---|---|---|---|
| 행정 | 전자결재 고도화 | 410 | 150 | 150 |
| ^^ | 행정정보 DB 이중화 | 300 | 80 | 80 |
| 교통 | 스마트 교차로 확대 | 1,150 | 1,200 | 600 |
| ^^ | 버스정보시스템 교체 | 700 | 100 | 100 |
| 안전 | CCTV 통합관제 증설 | 1,120 | 900 | 500 |
| 합계 || 3,680 | 2,430 | 1,430 |

2028년 이후 금액은 중기지방재정계획 반영 전 추정치이다.

연차별 투자 계획은 2027년 3월 정보화위원회 심의 후 확정한다.

붙임: 사업별 산출 내역서 1부. 끝.
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### report3-w (write)

Write a new document: a heading level 1 "2026년 하반기 청사 시설 점검 계획"; then one table with the columns 구역, 층, 점검 항목, 점검일, 담당 for these rows, in this order (구역 / 층 / 점검 항목 / 점검일 / 담당):
- 본관 / 1~3층 / 소방 설비 / 10월 12일 / 시설관리팀
- 본관 / 1~3층 / 전기 설비 / 10월 13일 / 시설관리팀
- 본관 / 4~6층 / 소방 설비 / 10월 14일 / 시설관리팀
- 별관 / 전층 / 소방 설비 / 10월 19일 / 외부 위탁
- 별관 / 전층 / 승강기 / 10월 20일 / 외부 위탁
In the table, each 구역 should appear once for its rows; within a 구역, a 층 should appear once for consecutive rows with the same 층, and a 담당 once for the rows of that 구역 with the same 담당. The 점검 항목 and 점검일 cells are never combined. After the table, a paragraph "점검 결과는 10월 30일까지 시설관리팀에 제출한다."

Start the new file with this front matter:

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---
````

### report3-e1 (edit)

In the table under "2. 분야별 사업 예산 (단위: 백만 원)", add a column 비고 right after 사업명. Its entries: 전자결재 고도화 "클라우드 전환"; 민원 키오스크 유지보수 "계약 만료"; 스마트 교차로 확대 "국비 50%"; CCTV 통합관제 증설 "국비 30%". The other 사업 have no 비고: leave their cells empty. In the 합계 row, the 합계 label keeps covering every column to the left of 2026 예산, the new 비고 column included.

### report3-e2 (edit)

In the table under "2. 분야별 사업 예산 (단위: 백만 원)", the 담당 부서 of 행정정보 DB 이중화 is now 정보통신과. 기획예산과 stays the 담당 부서 of the other two 행정 사업. Change nothing else.

### report3-e3 (edit)

In the table under "4. 인건비 및 운영비 (단위: 백만 원)", the 직급 column should show each 직급 once for its rows. Change nothing else.

### report3-e4 (edit)

In the table under "2. 분야별 사업 예산 (단위: 백만 원)", add the 사업 "교통 빅데이터 분석" (2026 예산 0, 2027 요구 230, 증감 +230) as the first 교통 사업, before 스마트 교차로 확대. It is handled by 교통정책과 like the other 교통 사업: 교통 and 교통정책과 should each still appear once for all 교통 사업. Leave the 합계 row as it is.

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
