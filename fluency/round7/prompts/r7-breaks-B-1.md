You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 6 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. The body is Markdown: `#` headings, one line per paragraph, `- ` and `1. ` list items (a nested item is indented two spaces), `**bold**` and `*italic*`. A blank line only separates blocks; it is never a paragraph itself. Inside a paragraph, `<br/>` is a line break: the text goes on in the same paragraph. A line holding only `<pagebreak/>` is a page break between paragraphs. `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a chart); leave every one exactly as it is. Besides Markdown, only the tags and markers described below exist; there are no other tags or attributes.

### Styles

Formatting is by name only: a paragraph or a table takes one of the file's own styles, listed with the file. A paragraph with a style is `<div style="Name">text</div>`, on one line; a paragraph without it has the default style. A style name is written exactly as listed, spaces included.

### Tables

A pipe table is a header row, a delimiter row with one `---` per column, then one line per row; every row has one cell per column. `^^` as the whole cell means the cell is merged into the one above; `||` (two pipes with nothing between) means the cell to the left extends into this column. An empty unmerged cell is `|  |`.

### Breaks inside a paragraph

A page break or a column break can sit in the middle of a paragraph, between two stretches of that paragraph's text. `<br type="page"/>` starts a new page at that point; `<br type="column"/>` starts a new column at that point. The text before the tag and the text after it stay one paragraph.

- Write the tag where the break falls, with no space inside it: `앞 문장.<br type="page"/>뒤 문장`, `서울 14곳.<br type="column"/>부산 5곳`.
- `<br>` takes only `type`, and `type` is `page` or `column`. It is a single tag; there is no `</br>`. A `<br/>` with no `type` is still a line break inside the paragraph (the text continues on the next line of the same page and column).
- A line that holds only `<pagebreak/>` is still a page break *between* paragraphs, as above. A line that holds only `<br type="column"/>` is a paragraph whose only content is a column break.
- Nothing else can be expressed: a section break, an odd-page or even-page break, a change of column count, a change of margins. A request for one of those is refused.

Example (a page break in the middle of the 핵심 지표 paragraph, a column break in the 지역 줄, then a page break between the body and the 부록 heading):

```
수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.<br type="page"/>첨부 1의 지점별 표는 10월 말 확정치입니다.

서울 14곳, 경기 7곳.<br type="column"/>부산 5곳, 대구 3곳.

<pagebreak/>

# 부록
```
 
## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
---
type: document
format: docx
template: org/report
schema: 1
---

# 3분기 영업 보고

영업본부는 2026년 3분기 실적을 다음과 같이 보고합니다. 매출은 전년 대비 +12% 증가했고, 신규 고객 34곳 중 21곳이 수도권입니다.

## 핵심 지표

<div style="표 제목">표 1. 지역별 매출 (억 원)</div>

{style="표 눈금"}
| 지역 | 지점 | 매출 | 증감 |
|---|---|---|---|
| 서울 | 강남 | 120 | +12% |
| 서울 | 종로 | 95 | +8% |
| 부산 | 해운대 | 88 | +4% |
| 합계 | — | 303 | +9% |

수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.<br type="page"/>첨부 1의 지점별 표는 10월 말 확정치이며, 잠정치와 다를 수 있습니다.

## 지역별 현황

서울 14곳, 경기 7곳, 인천 0곳.<br type="column"/>부산 5곳, 대구 3곳, 광주 2곳, 그 외 3곳.

신규 고객 중 서울 14곳은 강남·종로에 집중되어 있고, 지방 13곳 중 부산이 5곳입니다.

- 전년 대비 +12% 증가는 수도권 대형 거래 3건이 이끌었습니다.
- 지방은 전년 대비 +4%로, 부산 해운대 지점이 가장 컸습니다.
  - 해운대 지점 매출 88억은 잠정치입니다.

<div style="Note">표의 +12%와 본문의 전년 대비 +12%는 같은 내부 집계입니다.</div>

<keep id="k3" kind="drawing" summary="조직도, 상자 5개"/>

<pagebreak/>

# 부록

첨부 1. 지점별 매출 원표는 영업지원팀에 있습니다. 서울 14곳의 명단은 별도 파일입니다.

문의는 영업본부 기획팀 (☎ 02-000-1234)으로 하시기 바랍니다.
=== FILE END ===

## Tasks

1. `report1-q1` (read): In the file, the paragraph that reports 수도권 매출 and 지방 증감 has an inline page break. Quote the five words immediately after that page break, then the five words immediately after the inline column break in the 지역별 현황 paragraph. Answer as `ANSWER: <page-break words> | <column-break words>`.
2. `report1-w` (write): Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 "주간 영업 메모"; a default-style paragraph "강남 지점 매출은 잠정치 12억입니다." then an inline page break then "종로 지점은 같은 주 9억입니다."; a default-style paragraph "서울 안건." then an inline column break then "부산 안건."; a page break between paragraphs; a default-style paragraph "다음 회의는 금요일 10시입니다."
3. `report1-e1` (edit): In the opening paragraph (the one that begins 영업본부는), insert an inline page break after the first sentence (after "보고합니다."). Do not change the other page break, and do not touch the other "전년 대비 +12%" sentence.
4. `report1-e2` (edit): In the paragraph that lists 서울 14곳, 경기 7곳, 인천 0곳 and then 부산 5곳, move the inline column break so it falls after "대구 3곳." instead of after "인천 0곳.". Keep every city count. Do not edit the other paragraph that also says 서울 14곳.
5. `report1-e3` (edit): Remove the inline page break from the paragraph that begins 수도권 매출은 전년 대비 +12%. Keep that paragraph as one paragraph (the two stretches become one, with a space where the break was). Do not remove the page break between paragraphs before 부록, and do not edit the opening paragraph that also says 전년 대비 +12%.
6. `report1-r` (edit): Insert a next-page section break before the 부록 heading so that heading starts a new section with 2.5 cm margins and two newspaper columns for the rest of the file.

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Write tasks: answer with `text`: the complete new file, including its front matter.
- Edit tasks: answer with exact text edits. Each edit is `{"old": "...", "new": "..."}`: `old` is copied exactly from the file (same spaces and characters) and occurs exactly once in it, and `new` replaces it. Edits apply in order. Change only what the task asks for; every other line must stay exactly as it is.
- If a task cannot be done in this file format, do not edit: answer `REFUSE: <one sentence why>` and `"edits": []`. Refuse only when the task cannot be done as asked.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "…", "text": "ANSWER: …"},
  {"task_id": "…", "text": "<the complete new file>"},
  {"task_id": "…", "edits": [{"old": "…", "new": "…"}]},
  {"task_id": "…", "text": "REFUSE: …", "edits": []}
]}
```

One entry per task, in the order of the tasks.
