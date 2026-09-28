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

### Empty paragraphs

Documents use empty paragraphs for spacing, often several in a row and sometimes with a style. An empty paragraph with the default style is a line holding only `<p/>`; an empty paragraph with a paragraph style is a line holding only `<p style="Name"/>`.

- Each empty paragraph is its own line: three empty paragraphs are three such lines, even when they are identical. Consecutive empty paragraphs are on consecutive lines, with no blank line between them; a blank line separates the group from the blocks before and after it, as for any block.
- `<p/>` and `<p style="Name"/>` are single tags; there is no `</p>`, and `<p>` takes no other attribute. The tag stands alone on its line, never inside a line of text or a table cell.
- A blank line is not an empty paragraph, and neither is a line holding only `<br/>`.

Example (two empty paragraphs with the default style after the heading, and one in the style 좁은 간격 after the table):

```
# 알림

<p/>
<p/>

행사 일정은 다음과 같다.

| 구분 | 일시 |
|---|---|
| 1차 | 3월 |

<p style="좁은 간격"/>

문의는 총무과로 한다.
```

## Names available in this file

Paragraph styles:
- `Title` — large centered document title
- `Caption` — title line directly above a table
- `Spacer` — low empty line between a table and the text after it
- `Signature` — right-aligned date and signature line
- `Note` — small gray note

Table styles:
- `Table Grid` — thin black lines around all cells
- `Grid Table 4` — gray header row, gray banded rows

## The file: board-2.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

<div style="Title">제12기 제4차 이사회 의사록</div>

<p/>
<p/>

일시: 2026년 10월 27일(화) 16:00

장소: 본사 12층 이사회실

출석 이사: 5명 중 5명 (사외이사 2명 포함) / 출석 감사: 1명

<p/>

## 1. 개회

의장인 대표이사 김정훈은 정관 제31조에 따라 이사회가 적법하게 성립되었음을 알리고 개회를 선언하였다.

<p/>

## 2. 보고 사항

재무 담당 이사 박현주는 2026년 3분기 경영 실적을 다음과 같이 보고하였다.

<div style="Caption">표 1. 3분기 경영 실적 (단위: 억 원)</div>

{style="Table Grid"}
| 구분 | 3분기 | 전년 동기 | 증감률 |
|---|---|---|---|
| 매출액 | 412 | 385 | +7.0% |
| 영업이익 | 38 | 31 | +22.6% |
| 당기순이익 | 27 | 24 | +12.5% |

<p style="Spacer"/>

매출액은 신규 거래처 확대로 늘었으나, 원자재 가격 상승으로 매출원가율이 1.2%포인트 높아졌다. 영업이익은 판매관리비 절감 효과로 전년 동기보다 7억 원 늘었다.

이사들은 보고 내용을 확인하였으며, 사외이사 정유진은 원자재 가격 상승에 따른 4분기 원가 관리 방안을 다음 회의에서 보고해 줄 것을 요청하였다.

<p/>

감사 오미경은 2026년 3분기 내부 감사 결과를 보고하였다. 구매 업무와 법인카드 사용을 점검한 결과 중대한 지적 사항은 없었으며, 구매 요청서 결재 누락 2건은 담당 부서에 시정을 요구하였다고 밝혔다.

이사들은 감사 결과를 확인하고, 결재 누락이 반복되지 않도록 전자 결재 시스템의 필수 결재 설정을 점검할 것을 요청하였다.

<p/>

## 3. 의결 사항

제1호 의안: 2027년 사업계획 승인의 건

의장은 2027년 사업계획안의 주요 내용을 설명하고 심의를 요청하였다. 사업계획안은 매출액 1,750억 원, 영업이익 160억 원을 목표로 하며, 평택 공장 3라인 증설에 120억 원을 투자하는 내용을 담고 있다.

사외이사 정유진은 설비 투자 증가에 따른 차입금 규모를 물었으며, 재무 담당 이사 박현주는 투자금의 절반은 자체 자금으로, 나머지는 시설 자금 대출로 조달할 계획이라고 답변하였다.

<div style="Caption">표 2. 2027년 사업계획 주요 지표 (단위: 억 원)</div>

{style="Table Grid"}
| 구분 | 2026년 전망 | 2027년 계획 | 증감률 |
|---|---|---|---|
| 매출액 | 1,610 | 1,750 | +8.7% |
| 영업이익 | 142 | 160 | +12.7% |
| 설비 투자 | 85 | 120 | +41.2% |

<p style="Spacer"/>

심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.

<p/>

제2호 의안: 지점 개설의 건

의장은 부산 지점 개설 안건을 상정하고 개설 개요를 설명하였다.

<div style="Caption">표 3. 지점 개설 개요</div>

{style="Grid Table 4"}
| 항목 | 내용 |
|---|---|
| 지점명 | 부산 지점 |
| 소재지 | 부산광역시 해운대구 센텀중앙로 00 |
| 개설 예정일 | 2027년 1월 4일 |
| 인원 | 6명 (영업 4명, 지원 2명) |

<p style="Spacer"/>

심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.

<p/>

제3호 의안: 내부회계관리규정 개정의 건

의장은 외부감사법 시행령 개정에 따라 내부회계관리규정 일부를 개정하는 안을 상정하였다.

<div style="Caption">표 4. 내부회계관리규정 주요 개정 내용</div>

{style="Table Grid"}
| 조항 | 현행 | 개정안 |
|---|---|---|
| 제7조 | 운영 실태 보고: 연 1회 | 운영 실태 보고: 반기 1회 |
| 제12조 | 평가 주체: 내부감사팀 | 평가 주체: 감사위원회 |
| 부칙 | (신설) | 2027년 1월 1일부터 시행 |

<p style="Spacer"/>

심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.

<p/>

제4호 의안: 2027년 임원 보수 한도 결정의 건

의장은 2027년 임원 보수 한도를 2026년과 같은 18억 원으로 정하는 안을 상정하였으며, 출석 이사 전원의 찬성으로 원안대로 승인하였다. 이 안건은 정기 주주총회에 상정한다.

<p/>

## 4. 폐회

의장은 기타 안건이 있는지 물었으며, 사외이사 한동우는 부산 지점 개설에 따른 인력 채용 계획을 다음 이사회에 보고해 줄 것을 요청하였다. 의장은 이를 받아들였다.

의장은 이상으로 회의 목적 사항의 심의를 모두 마쳤음을 알리고 17시 20분에 폐회를 선언하였다.

위 의사의 경과와 결과를 명확히 하기 위하여 이 의사록을 작성하고 출석한 이사와 감사가 기명날인한다.

<p/>
<p/>
<p/>

<div style="Signature">2026년 10월 27일</div>

<p/>

<div style="Signature">의장 대표이사 김정훈 (인)</div>

<p/>

<div style="Signature">사내이사 박현주 (인)</div>

<p/>

<div style="Signature">사내이사 이상민 (인)</div>

<p/>

<div style="Signature">사외이사 정유진 (인)</div>

<p/>

<div style="Signature">사외이사 한동우 (인)</div>

<p/>

<div style="Signature">감사 오미경 (인)</div>

<p/>
<p/>

<div style="Note">이 의사록은 원본 1부를 작성하여 본사 경영지원팀이 보관한다.</div>

<div style="Note">첨부: 1. 2027년 사업계획서 1부. 2. 부산 지점 개설 계획서 1부. 3. 내부회계관리규정 개정안 1부.</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### board2-w (write)

Write a new document with exactly these blocks, in this order, and nothing else: the title "이사회 소집 통지서" in the style for a large centered document title; two empty paragraphs with the default style; the paragraphs "일시: 2026년 10월 27일(화) 16:00" and "장소: 본사 12층 이사회실", both with the default style; the line "표 1. 부의 안건" in the file's style for a table's title line; directly after it a table with thin black lines around all cells, with the columns 안건, 내용 and the rows 제1호 의안 / 2027년 사업계획 승인의 건 and 제2호 의안 / 지점 개설의 건; the low empty line used between a table and the text after it; the paragraph "참석이 어려운 이사는 10월 23일까지 알려 주시기 바랍니다." with the default style; three empty paragraphs with the default style; the line "2026년 10월 20일" in the style for right-aligned date and signature lines; one empty paragraph with the default style; the line "대표이사 김정훈" in the same signature style.

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### board2-e1 (edit)

Directly after the low spacing line that follows 표 2 (2027년 사업계획 주요 지표), add one empty paragraph with the default style, so that both come before the next paragraph. Change nothing else.

### board2-e2 (edit)

Of the three empty paragraphs directly before the date line "2026년 10월 27일", replace the middle one with the paragraph "(첨부: 2027년 사업계획서 1부)" with the default style; the first and the third stay empty. Change nothing else.

### board2-e3 (edit)

Remove every empty paragraph in section 3, between the heading "3. 의결 사항" and the heading "4. 폐회", including the low spacing lines after the tables. Empty paragraphs elsewhere stay. Change nothing else.

### board2-e4 (edit)

Make each empty paragraph between the signature lines exactly 5 mm high.

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
