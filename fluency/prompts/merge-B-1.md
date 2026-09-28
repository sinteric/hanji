You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and four tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items, `**bold**` and `*italic*`. Besides Markdown, only the tags described below exist; there are no other tags or attributes.

### Tables

Every table is a pipe table: a header row, a delimiter row with one `---` per column, then one line per row. Every row has one cell per column, merged or not.

Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column.
- A merged area is a rectangle. For two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. An empty cell that is not merged is written with a space, `|  |`.
- The text of a merged cell is written once, in its top-left cell; the covered cells hold only a marker, never text.

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

작성: 영업기획팀 / 보고일: 2026-10-05

## 1. 요약

3분기 전사 매출은 5,012백만 원으로 전년 동기 대비 4.8% 증가했다.

수도권과 영남권이 성장을 이끌었고, 호남권과 제주는 전년 수준에 머물렀다.

## 2. 지역별 실적 (단위: 백만 원)

| 구분 || 7월 | 8월 | 9월 | 합계 |
|---|---|---|---|---|---|
| 수도권 | 서울 본점 | 412 | 398 | 455 | 1,265 |
| ^^ | 경기 지점 | 287 | 301 | 296 | 884 |
| ^^ | 인천 지점 | 156 | 162 | 170 | 488 |
| 영남권 | 부산 지점 | 301 | 287 | 312 | 900 |
| ^^ | 대구 지점 | 198 | 205 | 211 | 614 |
| 호남권 | 광주 지점 | 162 | 156 | 170 | 488 |
| 제주 | 제주 지점 | 124 | 131 | 118 | 373 |
| 전사 합계 || 1,640 | 1,640 | 1,732 | 5,012 |

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
