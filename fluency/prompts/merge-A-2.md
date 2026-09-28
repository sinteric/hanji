You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and four tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Tables

A simple table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row.

A table with any merged cell is written as `<table>` … `</table>` with one `<tr>` per line. Header cells are `<th>`, the others `<td>`.

- `rowspan="N"`: the cell extends down over N rows. In the rows below, that cell is left out.
- `colspan="N"`: the cell extends right over N columns.
- A cell may have both. `<table>` and `<tr>` take no attributes, and cells take no others.

Every row covers the same number of columns as the header: the cells written, plus their colspans, plus the cells still covered by a rowspan from above.

Example ("서울" covers two rows, "합계" covers two columns):

```
<table>
<tr><th>지역</th><th>지점</th><th>매출</th></tr>
<tr><td rowspan="2">서울</td><td>강남</td><td>120</td></tr>
<tr><td>종로</td><td>95</td></tr>
<tr><td colspan="2">합계</td><td>215</td></tr>
</table>
```

## Names available in this file

This file uses no named styles or layouts.

## The file: report-2.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 개발본부 주간 업무 보고 (2026년 9월 4주차)

작성: 개발기획파트 / 기간: 9월 21일 ~ 9월 25일

## 1. 금주 요약

- 모바일 앱 4.2 버전 출시 완료
- 결제 모듈 장애 1건 발생, 2시간 내 복구
- 추석 연휴 대비 비상 연락망 점검

## 2. 팀별 실적 및 계획

<table>
<tr><th>팀</th><th>업무</th><th>금주 실적</th><th>차주 계획</th><th>진척률</th></tr>
<tr><td rowspan="3">개발1팀</td><td>모바일 앱 4.2</td><td>출시 완료</td><td>모니터링</td><td>100%</td></tr>
<tr><td>푸시 알림 개선</td><td>설계 검토</td><td>개발 착수</td><td>20%</td></tr>
<tr><td>결제 모듈 안정화</td><td>장애 원인 분석</td><td>재발 방지 패치</td><td>60%</td></tr>
<tr><td rowspan="2">개발2팀</td><td>관리자 웹 개편</td><td>화면 개발</td><td>화면 개발</td><td>60%</td></tr>
<tr><td>API 문서화</td><td colspan="2">휴가로 일정 순연 (9/29~10/2)</td><td>40%</td></tr>
<tr><td rowspan="2">QA팀</td><td>4.2 회귀 테스트</td><td>완료</td><td>없음</td><td>100%</td></tr>
<tr><td>결제 모듈 검증</td><td>테스트 케이스 작성</td><td>검증 수행</td><td>20%</td></tr>
</table>

진척률은 팀장 확인 기준이다.

## 3. 이슈 및 요청 사항

| 구분 | 내용 | 요청 부서 | 기한 |
|---|---|---|---|
| 장애 | 결제 모듈 타임아웃 재발 방지 | 개발1팀 | 10월 2일 |
| 인력 | QA 인력 1명 추가 지원 | QA팀 | 10월 10일 |
| 일정 | 관리자 웹 오픈 일정 확정 | 개발2팀 | 10월 2일 |

## 4. 차주 일정

- 9월 29일(월) 결제 모듈 패치 배포
- 10월 1일(수) 관리자 웹 중간 리뷰
````

## Tasks

### report2-w (write)

Write a new document. Its content is a heading level 1 "10월 당직 근무표", then one table with the columns 주차, 요일, 주간 담당, 야간 담당 and these three rows (cells separated by " / "):
1. 1주차 / 월~수 / 박지훈 / 최유진
2. 1주차 / 목~금 / 박지훈 / 정하늘
3. 2주차 / 월~금 / 외부 위탁 (한결시큐리티)
Merged cells: the two 1주차 cells are one cell; the two 박지훈 cells are one cell; in row 3, "외부 위탁 (한결시큐리티)" is one cell covering the 주간 담당 and 야간 담당 columns. After the table, a paragraph "비상 연락: 경영지원팀 내선 2020".

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

### report2-e1 (edit)

In the table under "2. 팀별 실적 및 계획", the row 관리자 웹 개편 has 화면 개발 in both the 금주 실적 and 차주 계획 columns. Merge these two cells into one cell reading 화면 개발 (계속).

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### report2-e2 (edit)

In the table under "2. 팀별 실적 및 계획", add a column 비고 between 차주 계획 and 진척률, with these values: 모바일 앱 4.2 "-"; 푸시 알림 개선 "디자인팀 협업"; 결제 모듈 안정화 "장애 보고서 첨부"; 관리자 웹 개편 "-"; API 문서화 "-"; 4.2 회귀 테스트 "-"; 결제 모듈 검증 "QA 인력 요청 중".

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### report2-e3 (edit)

In the table under "2. 팀별 실적 및 계획", change the 진척률 of 결제 모듈 안정화 from 60% to 70%. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
