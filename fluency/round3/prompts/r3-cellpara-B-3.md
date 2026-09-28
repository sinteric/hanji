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
- `서식 번호` — 서식 맨 위의 작은 서식 번호 줄
- `안내` — 서식 제목 아래와 표 아래의 작은 안내 문구
- `표 참고` — 표 칸 안의 작은 글씨 참고 사항
- `표 소제목` — 표 칸 안의 굵은 소제목 줄
- `서명` — 오른쪽 정렬된 날짜·서명 줄
- `수신` — 왼쪽 정렬된 크고 굵은 수신 기관 줄

Table styles:
- `서식 표` — 굵은 바깥 테두리, 안쪽은 가는 실선
- `표 눈금` — 모든 칸에 가는 검은 실선
- `일반 표 1` — 가는 가로선만, 세로선 없음

## The file: form-3.hj.md

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---

<div style="서식 번호">[별지 제3호서식] (개정 2026. 7. 1.)</div>

# 공공시설 대관 신청서

<div style="안내">※ 뒤쪽의 작성 방법을 읽고 작성하시기 바라며, [ ]에는 해당되는 곳에 √ 표시를 합니다.</div>

{style="서식 표"}
{list-table}
-
  - 구분
  - 항목
  - 내용
-
  - 신청인
  - 성명(단체명)
  - 한빛문화예술협회
-
  - ^^
  - 대표자
  - 이서연
-
  - ^^
  - 주소
  - 서울특별시 ○○구 ○○로 12<br/>한빛빌딩 3층
-
  - ^^
  - 연락처
  - 전화: 02-000-1234
    휴대전화: 010-0000-5678
-
  - 이용 계획
  - 시설명
  - [ ] 대공연장
    [√] 소공연장
    [ ] 전시실
-
  - ^^
  - 이용 일시
  - 2026. 11. 21.(토) 13:00 ~ 18:00
    <div style="표 참고">리허설과 정리 시간 포함</div>
-
  - ^^
  - 행사명
  - 제5회 시민 합창 발표회
-
  - ^^
  - 행사 내용
  - 시민 합창단 4개 팀 공연
    관람객 약 250명 예상
    <div style="표 참고">입장료 없음</div>
-
  - ^^
  - 부대 설비
  - [√] 음향 [√] 조명 [ ] 영상
    <div style="표 참고">※ 해당 시 작성</div>
-
  - 준수 사항
  - ||
  - <div style="표 소제목">신청인은 다음 사항을 지킵니다.</div>
    ① 시설물을 훼손하거나 잃어버린 경우 원래대로 복구합니다.
    ② 허가받은 용도 외에는 시설을 사용하지 않습니다.
    ③ 행사가 끝난 뒤 1시간 안에 정리를 마칩니다.

위와 같이 공공시설 대관을 신청하며, 준수 사항을 지킬 것을 서약합니다.

<div style="서명">2026년 10월 20일</div>

<div style="서명">신청인 이서연 (서명 또는 인)</div>

<div style="수신">○○시 문화예술회관장 귀하</div>

## 첨부 서류 및 수수료

{style="표 눈금"}
{list-table}
-
  - 구분
  - 서류
  - 수수료
-
  - 신청인 제출 서류
  - 가. 행사 계획서 1부
    나. 단체 소개서 1부 (단체인 경우만)
    <div style="표 참고">※ 해당 시 작성</div>
  - 「○○시 문화예술회관 관리 조례」 별표 2에 따른 사용료
-
  - 담당 공무원 확인 사항
  - 법인 등기사항증명서 (법인인 경우만)
  - ^^

<div style="안내">※ 담당 공무원 확인 사항은 신청인이 확인에 동의하지 않으면 해당 서류를 직접 제출해야 합니다.</div>

## 개인정보 수집·이용 동의

문화예술회관은 대관 신청을 처리하기 위하여 다음과 같이 개인정보를 수집·이용합니다.

{style="서식 표"}
{list-table}
-
  - 구분
  - 내용
-
  - 수집 항목
  - 성명, 주소, 전화번호, 휴대전화번호
    <div style="표 참고">단체는 대표자의 정보를 수집합니다.</div>
-
  - 이용 목적
  - 대관 신청의 접수와 허가
    사용료 부과와 환불
    시설 이용 안내
-
  - 보유 기간
  - 대관 종료 후 3년
    <div style="표 참고">관계 법령에 따라 보존해야 하는 경우에는 그 기간</div>
-
  - 제3자 제공
  - 제공하지 않습니다.
    <div style="표 참고">법령에 특별한 규정이 있는 경우는 예외로 합니다.</div>
-
  - 동의 거부
  - 동의를 거부할 수 있습니다. 다만, 동의하지 않으면 대관 신청을 처리할 수 없습니다.

<div style="서명">위 내용에 동의합니다. [√] 동의함 [ ] 동의하지 않음</div>

## 사용료 감면

다음에 해당하는 행사는 사용료를 감면합니다. 감면을 받으려면 신청서와 함께 증빙 서류를 내야 합니다.

{style="표 눈금"}
{list-table}
-
  - 감면 구분
  - 대상
  - 감면율
-
  - 전액 면제
  - 시가 주최하거나 주관하는 행사
    국가기관·공공기관이 공익 목적으로 여는 행사
  - 100%
-
  - 일부 감면
  - 관내 등록 예술 단체의 정기 공연
    관내 학교의 교육 행사
    <div style="표 참고">단체별로 연 2회까지</div>
  - 50%
-
  - ^^
  - 장애인·노인 단체가 회원을 위해 여는 행사
  - 30%

<div style="안내">※ 감면 대상이 둘 이상에 해당하면 감면율이 가장 높은 하나만 적용합니다.</div>

<div style="안내">※ 감면을 받은 뒤 입장료를 받거나 영리 목적으로 시설을 사용한 것이 확인되면 감면한 사용료를 다시 내야 합니다.</div>

<div style="안내">※ 부대 설비 사용료는 감면 대상에 포함되지 않습니다.</div>

## 작성 방법

- 성명(단체명)란에는 단체가 신청하는 경우 단체 이름을 적습니다.
- 이용 일시에는 리허설과 정리 시간을 포함하여 적습니다.
- 부대 설비는 사용할 설비에 √ 표시를 합니다.
  - 영상 설비는 사용 7일 전까지 따로 협의해야 합니다.
- 사용료는 허가 통지를 받은 날부터 5일 안에 냅니다.
- 사용 허가를 받은 뒤 취소하면 사용일 10일 전까지는 사용료 전액을, 그 뒤에는 절반을 돌려받습니다.
- 허가받은 사용자는 사용 권리를 다른 사람에게 넘길 수 없습니다.
- 시설 안에서는 음식물을 먹을 수 없으며, 무대 장치를 설치하려면 미리 허가를 받아야 합니다.
- 행사 계획서에는 행사 목적, 프로그램, 예상 관람 인원, 안전 관리 계획을 적습니다.
- 관람객이 300명 이상이면 안전 관리 요원 배치 계획을 함께 냅니다.
- 신청서는 사용일 60일 전부터 10일 전까지 문화예술회관 누리집이나 방문으로 냅니다.

## 처리 절차

{style="일반 표 1"}
| 신청서 작성 | 접수 | 검토 | 허가 | 사용료 납부 |
|---|---|---|---|---|
| 신청인 | 문화예술회관 | 문화예술회관 | 문화예술회관장 | 신청인 |

<div style="안내">※ 처리 기간은 접수일부터 7일입니다. 다만, 같은 날짜에 신청이 겹치면 추첨으로 정합니다.</div>

<div style="안내">※ 허가 결과는 신청서에 적은 휴대전화로 문자 메시지를 보내 알려 드립니다.</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### form3-w (write)

Write a new document with exactly these blocks, in this order, and nothing else: the line "[별지 제5호서식]" in the style for the small form number at the top of a form; a heading level 1 "공공시설 사용 결과 보고서"; a table with a thick outer border and thin inner lines, with the columns 구분, 항목, 내용 and these rows, in this order:
- 사용자 / 단체명 / "한빛문화예술협회".
- the same 구분 cell as the row above (사용자 covers both rows) / 사용 일시 / a 내용 cell with two paragraphs, "2026. 11. 21.(토) 13:00 ~ 18:00" and, in the style for small notes inside a table, "실제 사용 시간 기준".
- 결과, written once across the 구분 and 항목 columns / a 내용 cell with three paragraphs: "관람객 238명", "시설 훼손 없음" and, in the style for small notes inside a table, "사진 5매 첨부".
After the table, the line "2026년 11월 24일" in the style for right-aligned date and signature lines. Paragraphs in table cells have the default style unless a style is named above.

Start the new file with this front matter:

````
---
type: document
format: hwpx
template: org/gov-form
schema: 1
---
````

### form3-e1 (edit)

Remove the note "※ 해당 시 작성" from the 부대 설비 cell of the application table. The same note elsewhere stays. Change nothing else.

### form3-e2 (edit)

In the table under "첨부 서류 및 수수료", make the 서류 cell of 신청인 제출 서류 a single paragraph with the default style: "가. 행사 계획서 1부, 나. 단체 소개서 1부 (단체인 경우만)". The note in that cell goes away. Change nothing else.

### form3-e3 (edit)

In the 준수 사항 cell of the application table, put 6pt of space between the pledges ①, ② and ③.

### form3-e4 (edit)

In the application table, split the 준수 사항 row into three rows: the first keeps the cell's bold heading line and ①, the second holds ② and the third holds ③, each as the only paragraph of its cell. The 준수 사항 label stays one cell across the 구분 and 항목 columns and now covers all three rows. Change nothing else.

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
