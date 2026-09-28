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

## The file: report-3.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

# 신제품 '하루견과' 출시 마케팅 제안서

제안 부서: 마케팅전략팀 / 제안일: 2026-09-28

## 1. 제안 배경

간편 건강 간식 시장은 최근 3년간 연평균 11% 성장했다.

당사 견과류 제품군은 대형마트 매출 비중이 높아 온라인 채널 보완이 필요하다.

## 2. 소요 예산 (단위: 만 원)

<table>
<tr><th>항목</th><th>세부 내역</th><th>금액</th><th>집행 시기</th></tr>
<tr><td rowspan="3">마케팅</td><td>온라인 광고</td><td>1,500</td><td rowspan="2">11월</td></tr>
<tr><td>오프라인 시식 행사</td><td>800</td></tr>
<tr><td>인플루언서 협업</td><td>500</td><td>12월</td></tr>
<tr><td rowspan="2">제작</td><td>패키지 디자인</td><td>300</td><td>10월</td></tr>
<tr><td>초도 생산</td><td>2,400</td><td>10월</td></tr>
<tr><td>운영</td><td>물류 대행</td><td>500</td><td>11월~12월</td></tr>
<tr><td colspan="2">합계</td><td>6,000</td><td>-</td></tr>
</table>

예산은 2026년 하반기 마케팅 예비비에서 집행한다.

## 3. 추진 일정

| 단계 | 기간 | 주요 내용 |
|---|---|---|
| 준비 | 10월 1일 ~ 10월 31일 | 패키지 확정, 초도 생산 |
| 출시 | 11월 1일 ~ 11월 15일 | 온라인 광고, 시식 행사 |
| 확산 | 11월 16일 ~ 12월 31일 | 인플루언서 협업, 재구매 쿠폰 |

## 4. 기대 효과

- 출시 3개월 매출 4억 원
- 온라인 매출 비중 25%까지 확대
````

## Tasks

### report3-w (write)

Write a new document. Its content is a heading level 1 "교육비 지원 기준", a paragraph "2027년 1월 1일부터 적용한다.", and then one table with the columns 구분, 대상, 지원 한도, 비고 and these four rows (cells separated by " / "):
1. 직무 교육 / 전 직원 / 연 200만 원 / 사전 승인 필요
2. 직무 교육 / 팀장 이상 / 연 300만 원 / 사전 승인 필요
3. 자격증 / 전 직원 / 응시료 전액 / 합격 시 지급
4. 어학 / 전 직원 / 월 10만 원 / 합격 시 지급
Merged cells: the two 직무 교육 cells are one cell; the two 사전 승인 필요 cells are one cell; the two 합격 시 지급 cells are one cell. The 전 직원 cells stay separate.

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

### report3-e1 (edit)

In the table under "2. 소요 예산", the 집행 시기 cells of 패키지 디자인 and 초도 생산 both read 10월. Merge these two cells into one cell reading 10월. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### report3-e2 (edit)

In the table under "2. 소요 예산", add a column 담당 부서 right after 세부 내역. For the three 마케팅 rows it is one merged cell reading 마케팅전략팀; 패키지 디자인: 디자인팀; 초도 생산: 생산관리팀; 물류 대행: SCM팀. The 합계 cell in the last row then covers the 항목, 세부 내역 and 담당 부서 columns.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

### report3-e3 (edit)

In the table under "2. 소요 예산", change the 금액 of 인플루언서 협업 from 500 to 650. Change nothing else.

Answer with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```
