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

<table>
<tr><th>권역</th><th>지점</th><th>7월</th><th>8월</th><th>9월</th><th>합계</th></tr>
<tr><td rowspan="3">수도권</td><td>서울 본점</td><td>412</td><td>398</td><td>455</td><td>1,265</td></tr>
<tr><td>경기 지점</td><td>287</td><td>301</td><td>296</td><td>884</td></tr>
<tr><td>인천 지점</td><td>156</td><td>162</td><td>170</td><td>488</td></tr>
<tr><td rowspan="3">영남권</td><td>부산 지점</td><td>301</td><td>287</td><td>312</td><td>900</td></tr>
<tr><td>대구 지점</td><td>198</td><td>205</td><td>211</td><td>614</td></tr>
<tr><td>울산 지점</td><td>143</td><td>150</td><td>139</td><td>432</td></tr>
<tr><td rowspan="2">호남권</td><td>광주 지점</td><td>162</td><td>156</td><td>170</td><td>488</td></tr>
<tr><td>전주 지점</td><td>97</td><td>104</td><td>99</td><td>300</td></tr>
<tr><td rowspan="2">충청권</td><td>대전 지점</td><td>176</td><td>181</td><td>190</td><td>547</td></tr>
<tr><td>청주 지점</td><td>88</td><td>95</td><td>102</td><td>285</td></tr>
<tr><td colspan="2">전사 합계</td><td>2,020</td><td>2,039</td><td>2,144</td><td>6,203</td></tr>
</table>

8월 수치는 반품 정산 후 확정치이다.

9월에는 추석 선물 세트 판매로 수도권과 영남권 매출이 크게 늘었다.

권역별 수치는 지점 확정 매출 기준이며, 본사 직판 매출은 서울 본점에 포함했다.

## 3. 권역별 전년 대비 실적 (단위: 백만 원)

<table>
<tr><th rowspan="2">권역</th><th rowspan="2">지점</th><th colspan="2">3분기 매출</th><th rowspan="2">증감률</th></tr>
<tr><td>전년</td><td>금년</td></tr>
<tr><td rowspan="3">수도권</td><td>서울 본점</td><td>1,180</td><td>1,265</td><td>+7.2%</td></tr>
<tr><td>경기 지점</td><td>842</td><td>884</td><td>+5.0%</td></tr>
<tr><td>인천 지점</td><td>455</td><td>488</td><td>+7.3%</td></tr>
<tr><td rowspan="3">영남권</td><td>부산 지점</td><td>861</td><td>900</td><td>+4.5%</td></tr>
<tr><td>대구 지점</td><td>590</td><td>614</td><td>+4.1%</td></tr>
<tr><td>울산 지점</td><td>428</td><td>432</td><td>+0.9%</td></tr>
<tr><td rowspan="2">호남권</td><td>광주 지점</td><td>482</td><td>488</td><td>+1.2%</td></tr>
<tr><td>전주 지점</td><td>301</td><td>300</td><td>-0.3%</td></tr>
<tr><td rowspan="2">충청권</td><td>대전 지점</td><td>512</td><td>547</td><td>+6.8%</td></tr>
<tr><td>청주 지점</td><td>271</td><td>285</td><td>+5.2%</td></tr>
<tr><td colspan="2">전사 합계</td><td>5,922</td><td>6,203</td><td>+4.7%</td></tr>
</table>

증감률은 전년 동기 대비이며 소수점 둘째 자리에서 반올림했다.

울산 지점은 부산 지점과의 통합을 앞두고 영업 인력을 재배치하고 있다.

## 4. 제품군별 매출 (단위: 백만 원)

<table>
<tr><th>제품군</th><th>품목</th><th>수도권</th><th>영남권</th><th>호남권</th><th>충청권</th></tr>
<tr><td rowspan="2">생활용품</td><td>세제</td><td>820</td><td>610</td><td>240</td><td>250</td></tr>
<tr><td>주방용품</td><td>612</td><td>488</td><td>170</td><td>180</td></tr>
<tr><td rowspan="2">식품</td><td>음료</td><td>540</td><td>402</td><td>150</td><td>160</td></tr>
<tr><td>간편식</td><td>455</td><td>301</td><td>120</td><td>130</td></tr>
<tr><td rowspan="2">신제품</td><td>건강기능식품</td><td>128</td><td>88</td><td>64</td><td>72</td></tr>
<tr><td>반려동물용품</td><td>82</td><td>57</td><td>44</td><td>40</td></tr>
</table>

신제품은 7월 출시 품목으로, 권역별 수치는 잠정치이다.

생활용품과 식품이 전체 매출의 75%를 차지했으며, 신제품 비중은 7% 수준이다.

## 5. 주요 거래처 현황

<table>
<tr><th>거래처</th><th>담당</th><th>계약 상태</th><th>비고</th></tr>
<tr><td>한빛유통</td><td rowspan="2">김민수</td><td>갱신 완료</td><td>3년 계약</td></tr>
<tr><td>대성상사</td><td>갱신 협의 중</td><td>단가 조정 요청</td></tr>
<tr><td>동해물산</td><td rowspan="2">이서연</td><td>신규</td><td>9월 첫 발주</td></tr>
<tr><td>서해식품</td><td>갱신 협의 중</td><td>단가 조정 요청</td></tr>
<tr><td>남부상회</td><td>박준호</td><td>갱신 완료</td><td>2년 계약</td></tr>
</table>

갱신 협의 중인 2곳은 모두 단가 조정을 요청했으며, 10월 말까지 대응 방안을 마련한다.

## 6. 권역별 영업 인력 (9월 말 기준)

<table>
<tr><th>권역</th><th>지점</th><th>정원</th><th>현원</th><th>비고</th></tr>
<tr><td rowspan="3">수도권</td><td>서울 본점</td><td>18</td><td>17</td><td>충원 예정</td></tr>
<tr><td>경기 지점</td><td>12</td><td>12</td><td>-</td></tr>
<tr><td>인천 지점</td><td>8</td><td>7</td><td>충원 예정</td></tr>
<tr><td rowspan="3">영남권</td><td>부산 지점</td><td>11</td><td>11</td><td>-</td></tr>
<tr><td>대구 지점</td><td>8</td><td>8</td><td>-</td></tr>
<tr><td>울산 지점</td><td>6</td><td>5</td><td>충원 예정</td></tr>
<tr><td rowspan="2">호남권</td><td>광주 지점</td><td>7</td><td>7</td><td>-</td></tr>
<tr><td>전주 지점</td><td>5</td><td>5</td><td>-</td></tr>
<tr><td rowspan="2">충청권</td><td>대전 지점</td><td>7</td><td>6</td><td>충원 예정</td></tr>
<tr><td>청주 지점</td><td>5</td><td>5</td><td>-</td></tr>
<tr><td colspan="2">합계</td><td>87</td><td>83</td><td>4명 충원 예정</td></tr>
</table>

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
