You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Tables

A simple table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column.

A table with any merged cell is written as `<table>` … `</table>` with one `<tr>` per line. Cells of the first row are `<th>`, the others `<td>`; an empty cell is `<td></td>`.

- `rowspan="N"`: the cell extends down over N rows. In the rows below, that cell is left out.
- `colspan="N"`: the cell extends right over N columns.
- A cell may have both. `<table>` and `<tr>` take no attributes, and cells take no others.

Every row covers the same number of columns as the first row: the cells written, plus their colspans, plus the cells still covered by a rowspan from above.

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

작성: 개발기획파트 / 기간: 9월 21일 ~ 9월 25일 / 보고 대상: 개발본부장

## 1. 금주 요약

- 모바일 앱 4.2 버전 출시 완료 (9월 23일)
- 결제 모듈 장애 1건 발생, 2시간 내 복구
- 추석 연휴 대비 비상 연락망 점검 완료
- 데이터팀 결원 2명 채용 공고 게시
- 관리자 웹 개편 화면 개발 60% 진행
- 검색 속도 개선을 위한 인덱스 재설계 착수

## 2. 팀별 실적 및 계획

<table>
<tr><th>팀</th><th>업무</th><th>금주 실적</th><th>차주 계획</th><th>진척률</th></tr>
<tr><td rowspan="3">개발1팀</td><td>모바일 앱 4.2</td><td>출시 완료</td><td>모니터링</td><td>100%</td></tr>
<tr><td>푸시 알림 개선</td><td>설계 검토</td><td>개발 착수</td><td>20%</td></tr>
<tr><td>결제 모듈 안정화</td><td>장애 원인 분석</td><td>재발 방지 패치</td><td>60%</td></tr>
<tr><td rowspan="3">개발2팀</td><td>관리자 웹 개편</td><td>화면 개발</td><td>화면 개발</td><td>60%</td></tr>
<tr><td>API 문서화</td><td colspan="2">휴가로 일정 순연 (9/29~10/2)</td><td>40%</td></tr>
<tr><td>검색 속도 개선</td><td>인덱스 재설계</td><td>성능 테스트</td><td>50%</td></tr>
<tr><td rowspan="2">QA팀</td><td>4.2 회귀 테스트</td><td>완료</td><td>없음</td><td>100%</td></tr>
<tr><td>결제 모듈 검증</td><td>테스트 케이스 작성</td><td>검증 수행</td><td>20%</td></tr>
</table>

진척률은 팀장 확인 기준이다.

관리자 웹 개편은 화면 개발이 2주 연속 이어지고 있어 오픈 일정 재검토가 필요하다.

## 3. 팀별 인력 현황 (9월 25일 기준)

<table>
<tr><th>본부</th><th>팀</th><th>정원</th><th>현원</th><th>결원</th></tr>
<tr><td rowspan="3">개발본부</td><td>개발1팀</td><td>12</td><td>11</td><td>1</td></tr>
<tr><td>개발2팀</td><td>10</td><td>10</td><td>0</td></tr>
<tr><td>QA팀</td><td>6</td><td>5</td><td>1</td></tr>
<tr><td rowspan="3">기술지원본부</td><td>인프라팀</td><td>8</td><td>7</td><td>1</td></tr>
<tr><td>보안팀</td><td>5</td><td>5</td><td>0</td></tr>
<tr><td>데이터팀</td><td>6</td><td>4</td><td>2</td></tr>
<tr><td colspan="2">합계</td><td>47</td><td>42</td><td>5</td></tr>
</table>

정원은 2026년 조직 개편 기준이며, 파견 인력은 현원에 포함하지 않았다.

데이터팀 결원 2명은 10월 채용으로 충원할 예정이며, 그동안 개발1팀이 데이터 추출 업무를 지원한다.

## 4. 이슈 및 요청 사항

<table>
<tr><th>구분</th><th>내용</th><th>요청 부서</th><th>기한</th></tr>
<tr><td>장애</td><td>결제 모듈 타임아웃 재발 방지</td><td>개발1팀</td><td>10월 2일</td></tr>
<tr><td rowspan="2">인력</td><td>QA 인력 1명 추가 지원</td><td>QA팀</td><td rowspan="2">10월 10일</td></tr>
<tr><td>데이터팀 결원 2명 채용</td><td>데이터팀</td></tr>
<tr><td>일정</td><td>관리자 웹 오픈 일정 확정</td><td>개발2팀</td><td>10월 2일</td></tr>
</table>

인력 요청 2건은 10월 10일 인사위원회에서 함께 검토한다.

## 5. 차주 일정

<table>
<tr><th>일자</th><th>시간</th><th>내용</th><th>참석</th></tr>
<tr><td rowspan="2">9월 29일(월)</td><td>10:00</td><td>결제 모듈 패치 배포</td><td>개발1팀, QA팀</td></tr>
<tr><td>15:00</td><td>주간 회의</td><td>전 팀장</td></tr>
<tr><td>10월 1일(수)</td><td>14:00</td><td>관리자 웹 중간 리뷰</td><td>개발2팀, 기획파트</td></tr>
<tr><td rowspan="2">10월 2일(목)</td><td>종일</td><td>연휴 전 시스템 점검</td><td>전 팀</td></tr>
<tr><td>16:00</td><td>연휴 비상 연락망 최종 확인</td><td>전 팀장</td></tr>
</table>

10월 2일 점검은 전 팀이 참여하며, 점검 결과는 10월 5일까지 공유한다.

## 6. 9월 장애 및 배포 현황

<table>
<tr><th>주차</th><th>구분</th><th>서비스</th><th>건수</th><th>비고</th></tr>
<tr><td rowspan="2">1주차</td><td>장애</td><td>결제 모듈</td><td>0</td><td>-</td></tr>
<tr><td>배포</td><td>모바일 앱</td><td>2</td><td>4.1.3, 4.1.4</td></tr>
<tr><td rowspan="2">2주차</td><td>장애</td><td>관리자 웹</td><td>1</td><td>30분 내 복구</td></tr>
<tr><td>배포</td><td>관리자 웹</td><td>1</td><td>긴급 패치</td></tr>
<tr><td rowspan="2">3주차</td><td>장애</td><td>검색</td><td>1</td><td>인덱스 재구축</td></tr>
<tr><td>배포</td><td>모바일 앱</td><td>1</td><td>4.2 사전 배포</td></tr>
<tr><td rowspan="2">4주차</td><td>장애</td><td>결제 모듈</td><td>1</td><td>2시간 내 복구</td></tr>
<tr><td>배포</td><td>모바일 앱</td><td>1</td><td>4.2 정식 출시</td></tr>
<tr><td colspan="3">합계</td><td>8</td><td></td></tr>
</table>

장애 건수는 서비스 영향 30분 이상 기준이다.

4주차 결제 모듈 장애는 외부 결제 대행사의 응답 지연이 원인으로 확인되었다.

## 7. 10월 인력 운영 계획

<table>
<tr><th>팀</th><th>구분</th><th>인원</th><th>기간</th><th>비고</th></tr>
<tr><td rowspan="2">개발1팀</td><td>휴가</td><td>2</td><td>10월 5일 ~ 9일</td><td>추석 연휴 연계</td></tr>
<tr><td>교육</td><td>1</td><td>10월 14일 ~ 16일</td><td>보안 과정</td></tr>
<tr><td>개발2팀</td><td>휴가</td><td>1</td><td>9월 29일 ~ 10월 2일</td><td>API 문서화 담당</td></tr>
<tr><td>QA팀</td><td>지원</td><td>1</td><td>10월 한 달</td><td>외부 인력 (파견)</td></tr>
<tr><td>데이터팀</td><td>채용</td><td>2</td><td>10월 중</td><td>면접 진행 중</td></tr>
</table>

휴가 및 교육 일정은 팀장이 조정하며, 변경 시 개발기획파트에 알린다.

## 8. 코드 리뷰 현황

| 팀 | 요청 | 완료 | 대기 |
|---|---|---|---|
| 개발1팀 | 42 | 39 | 3 |
| 개발2팀 | 35 | 30 | 5 |
| QA팀 | 8 | 8 | 0 |
| 합계 | 85 | 77 | 8 |

대기 중인 리뷰 8건은 10월 1일까지 처리한다.

## 9. 건의 사항

- 결제 모듈 장애 대응 매뉴얼 개정 (개발1팀, 10월 중)
- QA 자동화 도구 라이선스 추가 구매 검토
- 관리자 웹 오픈 전 사용자 교육 일정 확정 필요
- 야간 배포 시 QA팀 대기 인력 수당 지급 기준 마련

## 10. 참고 사항

- 다음 주간 보고는 추석 연휴로 10월 2일(목)에 제출한다.
- 팀별 실적은 9월 25일 18시 기준이다.
- 장애 보고서는 사내 위키 장애 게시판에 게시했다.
- 코드 리뷰 대기 건수는 매주 금요일 17시 기준으로 집계한다.
- 인력 운영 계획은 10월 1일 팀장 회의에서 확정한다.

비상 연락: 개발기획파트 내선 3030 (연휴 기간 당직자 휴대전화로 전환)
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### report2-w (write)

Write a new document: a heading level 1 "2026년 10월 당직 근무표"; then one table with the columns 주차, 요일, 주간 담당, 야간 담당 for these rows, in this order (주차 / 요일 / 주간 담당 / 야간 담당):
- 1주차 / 월~수 / 박지훈 / 최유진
- 1주차 / 목~금 / 박지훈 / 정하늘
- 2주차 / 월~수 / 이도윤 / 최유진
- 2주차 / 목~금 / 이도윤 / 최유진
- 3주차 / 월~금 / 외부 위탁 (한결시큐리티), for both the day and the night duty
In the table, each 주차 should appear once for its rows, and within a 주차 a name that has the same duty on consecutive rows should appear once for those rows. In the 3주차 row, "외부 위탁 (한결시큐리티)" is written once and takes up both the 주간 담당 and 야간 담당 columns. After the table, a paragraph "비상 연락: 경영지원팀 내선 2020".

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### report2-e1 (edit)

In the table under "2. 팀별 실적 및 계획", the API 문서화 task now has separate entries for this week and next week: its 금주 실적 is "초안 50% 작성" and its 차주 계획 is "휴가로 일정 순연 (9/29~10/2)". Change nothing else.

### report2-e2 (edit)

In the table under "3. 팀별 인력 현황 (9월 25일 기준)", the 현원 and 결원 figures of 인프라팀 and 보안팀 are not final because of a reorganization. Show a single cell reading "조직 개편 중" in place of those four figures. Leave the 합계 row as it is.

### report2-e3 (edit)

In the table under "2. 팀별 실적 및 계획", 개발2팀 has a new task "SSO 연동" (금주 실적: 요건 정의, 차주 계획: 설계 착수, 진척률: 10%). Add it right after 관리자 웹 개편, as a task of 개발2팀.

### report2-e4 (edit)

The 모바일 앱 4.2 task is closed. Remove its row from the table under "2. 팀별 실적 및 계획"; 개발1팀 must still label its remaining tasks. Change nothing else.

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
