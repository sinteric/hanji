You work with office files stored as plain text. Below are the syntax documentation, the names available in the file, the file itself, and five tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items (a nested item is indented two spaces), `**bold**` and `*italic*`. A blank line only separates blocks; it is never a paragraph itself. Inside a paragraph, `<br/>` is a line break: the text goes on in the same paragraph. A line holding only `<pagebreak/>` is a page break. Besides Markdown, only the tags and markers described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. Colours, fonts, sizes, spacing, alignment, borders and shading cannot be written directly. A paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. A style name is written exactly as listed, spaces included.

### Tables

A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column, merged or not. Merged cells are written with two markers, placed in the cells that are covered:

- `^^` as the whole content of a cell: this cell is merged into the cell above it.
- `||`, two pipes with nothing between them: the cell to the left extends into this column. A span over three columns is `|||`, with no space between the pipes.
- A merged area is a rectangle; for two rows by two columns, write the text in the top-left cell followed by `||`, and `^^ ||` in the row below. The text of a merged cell is written once, in its top-left cell. An empty cell that is not merged is written with a space, `|  |`. `^^` never appears in the first row, and `||` never starts a row.

A table with a style has a line `{style="Name"}` directly before it, with no blank line between; the braces hold nothing but `style="Name"`, and nothing closes the table. A table without that line has the default table style.

Example ("서울" covers two rows, "합계" covers two columns; the table has the style Grid Table 4):

```
{style="Grid Table 4"}
| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |
```

### Cells with several paragraphs

A table cell holds one or more paragraphs. A table in which any cell holds more than one paragraph, or a paragraph with a style, is written as a list table; every other table stays a pipe table.

- A list table begins with a line `{list-table}`; its `{style="Name"}` line, if it has one, comes directly before that line. Without the `{list-table}` line, the lines below it would be an ordinary list.
- Each row is a line holding only `-`, followed by one line `  - ` (two spaces, `-`, a space) per cell, in column order. The first row is the header row, and every row has one cell per column.
- A cell's first paragraph is on its `  - ` line. Each further paragraph of that cell is its own line, indented four spaces. A paragraph with a style is written there as `<div style="Name">text</div>`. `<br/>` still breaks the line inside one paragraph.
- A covered cell is a `  - ` line holding only `^^` or `||`. The text of a merged cell, all of its paragraphs, is written once, in its top-left cell.
- A list table has no blank line inside; the first blank line ends it.

Example (the 내용 cell of 서울 holds two paragraphs, the second in the style 표 참고; its 비고 cell holds one paragraph in that style and covers the row below; the 내용 cell of 부산 holds two paragraphs with the default style):

```
{style="표 눈금"}
{list-table}
-
  - 지역
  - 내용
  - 비고
-
  - 서울
  - 강남·종로 2곳
    <div style="표 참고">종로는 10월 개점</div>
  - <div style="표 참고">임차</div>
-
  - 부산
  - 해운대 1곳
    서면 1곳
  - ^^
```

## Names available in this file

Paragraph styles:
- `표 제목` — 표 바로 위의 표 제목 줄
- `참고` — 본문 아래의 작은 회색 글씨 참고 사항
- `표 참고` — 표 칸 안의 작은 글씨 참고 사항
- `표 강조` — 표 칸 안에서 굵은 글씨로 강조하는 줄
- `발신 명의` — 가운데 정렬된 크고 굵은 기관장 명의

Table styles:
- `표 눈금` — 모든 칸에 가는 검은 실선
- `눈금 표 4` — 회색 머리글 행, 회색 줄무늬 행
- `일반 표 1` — 가는 가로선만, 세로선 없음

## The file: notice-1.hj.md

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---

# 2027년 청년 창업 지원사업 참여자 모집 공고

○○시는 지역 청년의 창업을 돕기 위하여 「2027년 청년 창업 지원사업」에 참여할 예비 창업자와 초기 창업자를 다음과 같이 모집합니다.

2027년 1월 13일

<div style="발신 명의">○○시장</div>

## 1. 사업 개요

- 사업 기간: 2027. 3. 1. ~ 2027. 11. 30. (9개월)
- 지원 규모: 40개 팀 내외, 팀당 최대 3,000만 원
- 지원 분야: 제조, 지식서비스, 콘텐츠, 바이오, 친환경 소재 등 전 분야
- 지원 내용: 사업화 자금, 창업 교육, 전문가 멘토링, 입주 공간
  - 입주 공간은 ○○청년창업센터 2~4층에 있으며 최대 20개 팀이 입주할 수 있습니다.
- 신청 기간: 2027. 1. 20.(수) ~ 2. 10.(수) 18:00

올해는 지역 특화 산업(바이오, 콘텐츠, 친환경 소재) 분야 창업 팀을 전체 선정 인원의 30% 이상 선발하며, 선정된 팀은 협약 기간 동안 분기마다 사업 진행 상황을 보고해야 합니다.

## 2. 신청 자격

신청일 현재 다음 요건을 모두 갖춘 사람이 신청할 수 있습니다.

<div style="표 제목">표 1. 신청 자격</div>

{style="표 눈금"}
{list-table}
-
  - 구분
  - 요건
  - 비고
-
  - 연령
  - 공고일 기준 만 19세 이상 39세 이하
    병역을 마친 사람은 복무 기간(최대 6년)을 연령 계산에서 뺍니다.
  - <div style="표 참고">1987. 1. 13. 이후 출생자</div>
-
  - 거주
  - 공고일 현재 ○○시에 주민등록을 둔 사람
  - <div style="표 참고">공동 대표는 전원이 요건을 갖추어야 합니다.</div>
    <div style="표 참고">법인은 본점 소재지 기준</div>
-
  - 창업
  - 예비 창업자 또는 창업 3년 이내 기업의 대표
    창업일은 사업자등록증의 개업일(법인은 법인 등기일) 기준
  - ^^
-
  - 제외 대상
  - 국세·지방세 체납자
    같은 아이템으로 다른 정부 창업 지원사업을 수행 중인 사람
    휴업 또는 폐업 중인 기업의 대표
  - <div style="표 참고">선정 후에 확인되어도 선정을 취소합니다.</div>

<div style="참고">자격 요건은 공고일 기준으로 판단하며, 제출한 증빙 서류로 확인합니다. 요건을 갖추지 못한 것이 확인되면 평가 대상에서 제외합니다.</div>

## 3. 지원 내용

<div style="표 제목">표 2. 지원 내용</div>

{style="눈금 표 4"}
{list-table}
-
  - 구분
  - 지원 내용
  - 지원 한도
  - 비고
-
  - 사업화 자금
  - 시제품 제작·마케팅 비용, 지식재산권 출원 비용
    자부담 10% 이상
  - 팀당 최대 3,000만 원
  - <div style="표 참고">인건비는 총액의 30% 이내</div>
-
  - 창업 교육
  - 창업 기초 과정 (20시간)
    분야별 심화 과정 (16시간)
  - 전액 지원
  - <div style="표 참고">기초 과정은 80% 이상 출석해야 합니다.</div>
-
  - 멘토링
  - 분야별 전문가 1:1 멘토링 (월 2회)
    투자 유치 설명회 참가
  - ^^
  - <div style="표 참고">멘토는 센터가 배정합니다.</div>
-
  - 입주 공간
  - ○○청년창업센터 개별 사무실
    회의실과 장비실 공동 이용
  - 최대 20개 팀
  - <div style="표 참고">관리비 월 5만 원은 입주 기업이 부담합니다.</div>

<div style="참고">사업화 자금은 협약을 체결한 뒤 두 번에 나누어 지급합니다. 2차 자금은 중간 점검을 통과한 팀에만 지급합니다.</div>

## 4. 신청 방법 및 제출 서류

- 신청 방법: ○○시 청년포털에서 온라인으로 접수 (방문·우편 접수 불가)
- 제출 서류는 모두 PDF 파일로 올립니다.

<div style="표 제목">표 3. 제출 서류</div>

{style="표 눈금"}
| 서류 | 대상 | 비고 |
|---|---|---|
| 사업 신청서 | 전원 | 지정 서식 |
| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 |
| 주민등록초본 | 전원 | 공고일 이후 발급분 |
| 사업자등록증 사본 | 기창업자 | 해당자만 |

## 5. 선정 절차

{style="일반 표 1"}
| 단계 | 일정 | 내용 |
|---|---|---|
| 서류 평가 | 2월 중 | 사업계획서 평가 (모집 인원의 2배수 선정) |
| 발표 평가 | 3월 초 | 대면 발표 및 질의응답 |
| 최종 선정 | 3월 중 | 선정 결과 개별 통보 |

<div style="참고">일정은 사정에 따라 바뀔 수 있으며, 바뀐 일정은 청년포털에 공지합니다.</div>

<div style="표 제목">표 4. 평가 기준</div>

{style="표 눈금"}
{list-table}
-
  - 평가 항목
  - 세부 내용
  - 배점
-
  - 사업 아이템
  - 아이템의 독창성과 시장성
    경쟁 제품과의 차별성
  - 40
-
  - 사업화 역량
  - 대표자와 팀원의 전문성
    사업화 추진 계획의 구체성
  - 30
-
  - 지역 기여
  - 지역 고용 창출 계획
    <div style="표 참고">○○시 거주자를 채용할 계획이 있으면 2점을 더합니다.</div>
  - 20
-
  - 사업비 계획
  - 사업비 편성의 적정성
  - 10
-
  - 합계
  - ||
  - 100

<div style="참고">발표 평가는 서류 평가 점수와 합산하지 않고 발표 평가 점수만으로 최종 선정합니다.</div>

## 6. 유의 사항

- 신청서와 사업계획서의 내용이 사실과 다르면 선정을 취소하고 지급한 지원금을 돌려받습니다.
- 한 사람은 한 팀으로만 신청할 수 있으며, 공동 대표의 중복 신청도 허용하지 않습니다.
- 사업화 자금은 협약서에 정한 용도로만 써야 하며, 다른 용도로 쓰면 환수합니다.
- 선정된 팀은 협약 기간이 끝난 뒤 2년 동안 매년 경영 현황 조사에 응해야 합니다.

## 7. 문의

○○시청 일자리정책과 청년창업팀 (☎ 031-000-1234, 평일 09:00~18:00)

온라인 접수 시스템 오류는 ○○시 청년포털 운영팀 (☎ 031-000-5678)으로 문의하시기 바랍니다.
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### notice1-w (write)

Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 "2027년 청년 창업 교육 수강생 모집"; the paragraph "창업 교육 과정별 내용은 다음과 같습니다." with the default style; the line "표 1. 과정별 내용" in the file's style for a table's title line; directly after it a table with thin black lines around all cells, with the columns 과정, 내용, 비고 and these rows, in this order:
- 기초 과정: the 내용 cell has two paragraphs, "사업계획서 작성 (8시간)" and "창업 법률·세무 (12시간)"; the 비고 cell has one paragraph, "온라인 병행", in the style for small notes inside a table.
- 심화 과정: the 내용 cell has two paragraphs, "분야별 실습 (12시간)" and, in the style for small notes inside a table, "분야는 신청할 때 선택"; its 비고 is the same cell as the 비고 of 기초 과정 (one cell covering both rows).
- 수료 기준: "80% 이상 출석", one paragraph written once across the 내용 and 비고 columns.
After the table, the note "교육 장소는 ○○청년창업센터 3층입니다." in the style for small gray notes below body text. Paragraphs in table cells have the default style unless a style is named above.

Start the new file with this front matter:

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---
````

### notice1-e1 (edit)

In 표 3 (제출 서류), give the 비고 cell of 사업계획서 a second paragraph, "사업비 집행 계획 포함", in the style for small notes inside a table. Change nothing else.

### notice1-e2 (edit)

In 표 2 (지원 내용), the first paragraph of the 지원 내용 cell of 사업화 자금 is "시제품 제작·마케팅 비용, 지식재산권 출원 비용". Split it at the comma into two paragraphs, "시제품 제작·마케팅 비용" and "지식재산권 출원 비용" (the comma goes away); the cell's other paragraph stays after them. Change nothing else.

### notice1-e3 (edit)

In 표 1 (신청 자격), shade the whole 제외 대상 row in light gray so that it stands out.

### notice1-e4 (edit)

In 표 2 (지원 내용), merge the 지원 내용 cells of 창업 교육 and 멘토링 into one cell. The merged cell keeps all four paragraphs, those of 창업 교육 first. Change nothing else.

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
