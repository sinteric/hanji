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

Documents use empty paragraphs for spacing, often several in a row and sometimes with a style. An empty paragraph with the default style is a line holding only `<div></div>`; an empty paragraph with a paragraph style is a line holding only `<div style="Name"></div>`.

- Each empty paragraph is its own line: three empty paragraphs are three such lines, even when they are identical. Consecutive empty paragraphs are on consecutive lines, with no blank line between them; a blank line separates the group from the blocks before and after it, as for any block.
- Nothing is between `<div …>` and `</div>`, not even a space. `<div></div>` is the only `<div>` without a style. The pair stands alone on its line, never inside a line of text or a table cell.
- A blank line is not an empty paragraph, and neither is a line holding only `<br/>`.

Example (two empty paragraphs with the default style after the heading, and one in the style 좁은 간격 after the table):

```
# 알림

<div></div>
<div></div>

행사 일정은 다음과 같다.

| 구분 | 일시 |
|---|---|
| 1차 | 3월 |

<div style="좁은 간격"></div>

문의는 총무과로 한다.
```

## Names available in this file

Paragraph styles:
- `표지 제목` — 표지 가운데의 크고 굵은 제목
- `표지 정보` — 표지 아래쪽의 작성 기관·날짜 줄
- `표 제목` — 표 바로 위의 표 제목 줄
- `참고` — 작은 회색 글씨의 참고 사항
- `표 아래 간격` — 표 바로 아래에 두는 높이가 낮은 빈 줄

Table styles:
- `표 눈금` — 모든 칸에 가는 검은 실선
- `눈금 표 4 - 강조색 1` — 파란 머리글 행, 연한 파란 줄무늬 행

## The file: report-3.hj.md

````
---
type: document
format: docx
template: org/report
schema: 1
---

<div></div>
<div></div>
<div></div>
<div></div>
<div></div>
<div></div>
<div></div>

<div style="표지 제목">2026년 하반기 사업장 정기 안전 점검 결과 보고서</div>

<div></div>
<div></div>
<div></div>
<div></div>
<div></div>

<div style="표지 정보">2026. 11.</div>

<div style="표지 정보">안전환경팀</div>

<div></div>
<div></div>
<div></div>
<div></div>
<div></div>
<div></div>

# 1. 점검 개요

산업안전보건법에 따른 위험성 평가의 후속 조치로, 전 사업장을 대상으로 하반기 정기 안전 점검을 실시하였다.

- 점검 기간: 2026. 10. 12. ~ 10. 30.
- 점검 대상: 본사, 평택 공장, 이천 물류센터
- 점검 인원: 안전환경팀 4명, 외부 전문기관 2명
- 점검 방법: 현장 확인, 관리 대장 검토, 작업자 면담
- 점검 기준: 사내 안전 점검 기준서(2026년 개정판)와 외부 전문기관 점검표

이번 점검은 상반기 점검에서 반복 지적된 항목(비상 대피로, 적재 상태)을 중점적으로 확인하였으며, 평택 공장은 신규 설치한 3라인 설비를 점검 대상에 추가하였다.

<div></div>

<div style="표 제목">표 1. 사업장별 점검 항목</div>

{style="표 눈금"}
| 사업장 | 점검 항목 | 점검일 |
|---|---|---|
| 본사 | 소방 설비, 비상 대피로, 전기 설비 | 10. 12. |
| 평택 공장 | 기계 방호 장치, 화학물질 보관, 소방 설비 | 10. 19. ~ 10. 21. |
| 이천 물류센터 | 지게차 운행, 적재 상태, 비상 대피로 | 10. 28. ~ 10. 30. |

<div style="표 아래 간격"></div>

# 2. 점검 결과

지적 사항은 모두 23건으로, 상반기(31건)보다 8건 줄었다. 중대 위험은 평택 공장에서 1건이 확인되어 즉시 조치하였다.

<div style="표 제목">표 2. 사업장별 지적 사항 (단위: 건)</div>

{style="눈금 표 4 - 강조색 1"}
| 사업장 | 중대 | 일반 | 경미 | 합계 |
|---|---|---|---|---|
| 본사 | 0 | 2 | 3 | 5 |
| 평택 공장 | 1 | 6 | 4 | 11 |
| 이천 물류센터 | 0 | 3 | 4 | 7 |
| 합계 | 1 | 11 | 11 | 23 |

<div style="표 아래 간격"></div>

상반기와 비교하면 소방 설비와 전기 설비 분야의 지적은 크게 줄었으나, 기계 방호 장치 분야는 3건에서 5건으로 늘었다. 신규 설비의 방호 장치 점검 절차가 아직 정착되지 않은 것이 원인으로 보인다.

사업장별로는 평택 공장의 지적 사항이 가장 많았으며, 이 가운데 4건은 신규 설치한 3라인에서 나왔다. 이천 물류센터는 적재 관련 지적이 상반기 5건에서 3건으로 줄었다.

주요 지적 사항은 다음과 같다.

- 평택 공장 2라인 프레스 방호 덮개 파손 (중대, 10. 19. 즉시 교체)
- 이천 물류센터 랙 상단 적재 높이 기준 초과 (일반)
- 본사 3층 비상구 앞 물품 적치 (일반)

<div></div>

# 3. 조치 계획

지적 사항은 위험 수준에 따라 조치 기한을 정하였다. 중대 위험은 발견 즉시, 일반은 30일, 경미는 45일 안에 조치한다.

<div style="표 제목">표 3. 지적 사항 조치 계획</div>

{style="표 눈금"}
| 구분 | 조치 내용 | 담당 | 기한 |
|---|---|---|---|
| 중대 | 방호 덮개 교체 및 전 라인 점검 | 평택 공장 생산팀 | 완료 |
| 일반 | 적재 기준 재교육, 비상구 상시 점검 | 각 사업장 관리팀 | 11. 30. |
| 경미 | 표지판 교체, 소화기 위치 조정 | 각 사업장 관리팀 | 12. 15. |

<div style="참고">조치 결과는 12월 안전보건위원회에 보고한다.</div>

일반 지적 사항 중 적재 기준 초과는 11월 첫 주에 해당 구역 작업자 전원을 대상으로 재교육을 실시하였고, 비상구 앞 적치는 관리팀이 매일 오후 순회 점검한다.

<div></div>

# 4. 안전 교육 실적

하반기 법정 안전 교육 이수율은 전 사업장 평균 97%로, 상반기(93%)보다 높아졌다. 미이수자는 11월 중 보충 교육을 받는다.

<div style="표 제목">표 4. 사업장별 안전 교육 이수율</div>

{style="표 눈금"}
| 사업장 | 대상 인원 | 이수 인원 | 이수율 |
|---|---|---|---|
| 본사 | 120 | 118 | 98% |
| 평택 공장 | 210 | 202 | 96% |
| 이천 물류센터 | 85 | 83 | 98% |

<div style="표 아래 간격"></div>

평택 공장은 교대 근무자의 교육 시간을 맞추기 어려워 야간 교육반을 따로 운영하였다. 이천 물류센터는 신규 입사자 12명에게 지게차 안전 교육을 따로 실시하였다.

2027년에는 사업장별 교육 담당자를 1명씩 지정하고, 교육 자료를 사내 교육 시스템에서 함께 쓰도록 할 계획이다.

<div></div>

# 5. 향후 계획

2027년 상반기 점검은 4월에 실시하며, 이천 물류센터는 지게차 운행 구역을 중점 점검한다. 평택 공장 3라인은 가동 6개월이 되는 3월에 설비 제조사와 함께 특별 점검을 실시한다.

점검 결과는 사내 게시판에 공개하고, 사업장별 안전 교육 자료로 활용한다. 반복 지적된 항목은 사업장장 평가에 반영하는 방안을 안전보건위원회에서 논의한다.

- 지게차 운행 구역에 보행자 통로를 따로 표시한다.
- 랙 적재 높이 기준을 현장에 게시하고, 월 1회 자체 점검한다.
- 비상구 앞 적치 금지 구역을 바닥에 표시한다.
- 신규 설비는 설치 후 1개월 안에 방호 장치 점검표를 작성하여 안전환경팀에 제출한다.

외부 전문기관의 점검 의견서는 안전환경팀이 보관하며, 요청하면 열람할 수 있다.

<div></div>
<div></div>

붙임: 사업장별 점검표 3부. 끝.

<div></div>
<div></div>
<div></div>

<div style="참고">작성: 안전환경팀 대리 서지훈 / 검토: 안전환경팀장 문경수 / 배포: 각 사업장장, 안전보건위원회</div>
````

## Tasks

A write task is answered with `text`: the complete new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly once; it is replaced by `new`.

### report3-w (write)

Write a new document with exactly these blocks, in this order, and nothing else: seven empty paragraphs with the default style; the cover title "2027년 상반기 사업장 정기 안전 점검 계획" in the style for the large cover title; five empty paragraphs with the default style; the lines "2027. 3." and "안전환경팀", each in the style for the cover's organisation and date lines; a page break; a heading level 1 "1. 점검 개요"; the paragraph "상반기 점검은 4월 한 달 동안 실시한다." with the default style; the line "표 1. 점검 일정" in the file's style for a table's title line; directly after it a table with a blue header row and light blue banded rows, with the columns 사업장, 점검일 and the rows 본사 / 4. 6. and 평택 공장 / 4. 13. ~ 4. 15.; the low empty line used directly below a table.

Start the new file with this front matter:

````
---
type: document
format: docx
template: org/report
schema: 1
---
````

### report3-e1 (edit)

The run of empty paragraphs after the cover's "안전환경팀" line pushes the report body to the next page. Replace that whole run with one page break. Change nothing else.

### report3-e2 (edit)

표 3 (지적 사항 조치 계획) is followed directly by the 참고 paragraph. Put one empty paragraph in the style used directly below tables between them, as after the other tables. Change nothing else.

### report3-e3 (edit)

The cover page has seven empty paragraphs before its title. Delete two of them, so that five remain. Change nothing else.

### report3-e4 (edit)

Make the empty paragraphs on the cover page 6pt high each, so that the title sits higher on the page.

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
