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

| 항목 | 세부 내역 | 금액 | 집행 시기 |
|---|---|---|---|
| 마케팅 | 온라인 광고 | 1,500 | 11월 |
| ^^ | 오프라인 시식 행사 | 800 | ^^ |
| ^^ | 인플루언서 협업 | 500 | 12월 |
| 제작 | 패키지 디자인 | 300 | 10월 |
| ^^ | 초도 생산 | 2,400 | 10월 |
| 운영 | 물류 대행 | 500 | 11월~12월 |
| 합계 || 6,000 | - |

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
