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

## The file: report-1.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 2026년 3분기 지역별 영업 실적 보고

작성: 영업기획팀 / 보고일: 2026-10-05

## 1. 요약

3분기 전사 매출은 5,012백만 원으로 전년 동기 대비 4.8% 증가했다.

수도권과 영남권이 성장을 이끌었고, 호남권과 제주는 전년 수준에 머물렀다.

## 2. 지역별 실적 (단위: 백만 원)

<table>
<tr><th colspan="2">구분</th><th>7월</th><th>8월</th><th>9월</th><th>합계</th></tr>
<tr><td rowspan="3">수도권</td><td>서울 본점</td><td>412</td><td>398</td><td>455</td><td>1,265</td></tr>
<tr><td>경기 지점</td><td>287</td><td>301</td><td>296</td><td>884</td></tr>
<tr><td>인천 지점</td><td>156</td><td>162</td><td>170</td><td>488</td></tr>
<tr><td rowspan="2">영남권</td><td>부산 지점</td><td>301</td><td>287</td><td>312</td><td>900</td></tr>
<tr><td>대구 지점</td><td>198</td><td>205</td><td>211</td><td>614</td></tr>
<tr><td>호남권</td><td>광주 지점</td><td>162</td><td>156</td><td>170</td><td>488</td></tr>
<tr><td>제주</td><td>제주 지점</td><td>124</td><td>131</td><td>118</td><td>373</td></tr>
<tr><td colspan="2">전사 합계</td><td>1,640</td><td>1,640</td><td>1,732</td><td>5,012</td></tr>
</table>

8월 수치는 반품 정산 후 확정치이다.

## 3. 주요 거래처 현황

| 거래처 | 담당 | 계약 상태 | 비고 |
|---|---|---|---|
| 한빛유통 | 김민수 | 갱신 완료 | 3년 계약 |
| 대성상사 | 김민수 | 갱신 협의 중 | 단가 조정 요청 |
| 동해물산 | 이서연 | 신규 | 9월 첫 발주 |
| 서해식품 | 이서연 | 갱신 협의 중 | 단가 조정 요청 |

## 4. 향후 계획

- 4분기 수도권 신규 거래처 5곳 확보
- 호남권 판촉 행사 2회 진행
- 단가 조정 요청 거래처 대응 방안 수립
````

## Tasks

### report1-w (write)

Write a new document. Its content is a heading level 1 "하반기 신입사원 교육 일정", followed by one table with the columns 구분, 과정, 일시, 장소 and these five rows (cells separated by " / "):
1. 공통 교육 / 회사 소개 / 10월 6일 오전 / 본사 대강당
2. 공통 교육 / 정보보안 / 10월 6일 오후 / 본사 대강당
3. 직무 교육 / 영업 실무 / 10월 7일 / 영업본부 회의실
4. 직무 교육 / 품질 관리 / 10월 8일 / 생산본부 회의실
5. 비고: 교육 불참 시 팀장 승인 필요
Merged cells: the two 공통 교육 cells are one cell; the two 본사 대강당 cells are one cell; the two 직무 교육 cells are one cell; row 5 is a single cell covering all four columns.

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

### report1-e1 (edit)

In the table under "3. 주요 거래처 현황", the 담당 cells of 한빛유통 and 대성상사 both read 김민수. Merge these two cells into one cell reading 김민수. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### report1-e2 (edit)

In the table under "2. 지역별 실적", add a new last column with the header 전년 동기 and these values: 서울 본점 1,180; 경기 지점 842; 인천 지점 455; 부산 지점 861; 대구 지점 590; 광주 지점 502; 제주 지점 351; 전사 합계 row 4,781.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### report1-e3 (edit)

In the table under "2. 지역별 실적", correct the 8월 value of 부산 지점 from 287 to 293. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
