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

# 2026년 하반기 고용노동 업무보고

장관 주재 업무보고입니다. 핵심은 청년 일자리, 산업안전, 노동시간 단축 세 가지이며, 수치는 내부 집계 기준입니다.

## 1. 청년 일자리

청년 고용률은 45.2%로 전년 동기 44.1%보다 1.1%p 올랐습니다. 일경험 프로그램 참여자는 12,400명입니다.

다음 단계로 대학일자리+센터 30곳을 추가 지정합니다.<br type="page"/>지정 공고는 11월 둘째 주에 나갑니다.

<div style="표 제목">표 1. 일경험 프로그램 (명)</div>

{style="표 눈금"}
| 구분 | 상반기 | 하반기 | 합계 |
|---|---|---|---|
| 공공 | 3,200 | 3,400 | 6,600 |
| 민간 | 2,800 | 3,000 | 5,800 |
| 합계 | 6,000 | 6,400 | 12,400 |

## 2. 산업안전

사고사망자는 3분기 누적 457명으로 전년 동기 443명보다 14명 늘었습니다. 50인 미만 사업장에서 26명이 늘었습니다.

다음 단계로 초소규모 사업장 점검 횟수를 두 배로 늘립니다.<br type="column"/>업종은 지붕·축사, 벌목, 질식 위험 작업부터입니다.

다음 단계 일정은 12월 초 관계부처 회의에서 확정합니다. 다음 주 내부 검토가 있습니다.

- 50인 이상 사업장은 12명 감소, 50인 미만은 26명 증가.
- 한파·온열 등 새로운 위험 요인을 점검 항목에 넣습니다.

<div style="Note">「다음 단계」는 각 과제의 후속 조치이며, 문단마다 가리키는 과제가 다릅니다.</div>

<pagebreak/>

## 3. 노동시간

실노동시간 단축 지원법 입법은 3월, 패키지 입법은 5월을 목표로 합니다.

다음 단계로 야간노동자 실태조사를 6월까지 마칩니다.

<keep id="k1" kind="drawing" summary="조직도, 본부 5개"/>

문의는 기획조정실 (☎ 044-000-1234)입니다.
=== FILE END ===

## Tasks

1. `ministry2-q1` (read): The file uses the words 다음 단계 in several paragraphs. Which 다음 단계 paragraph has an inline page break, and which has an inline column break? Answer as `ANSWER: page: <the words after the page break, up to the period> | column: <the words after the column break, up to the period>`.
2. `ministry2-w` (write): Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 "산업안전 주간 메모"; a default-style paragraph "이번 주 점검은 50인 미만 3곳입니다." then an inline page break then "결과는 금요일에 올립니다."; a default-style paragraph "지붕·축사." then an inline column break then "벌목."; a default-style paragraph "다음 단계 안내는 별도 공지입니다."
3. `ministry2-e1` (edit): In the paragraph that begins 청년 고용률은 45.2%, insert an inline page break after the first sentence (after "올랐습니다."). Do not touch the 다음 단계 paragraph that already has a page break.
4. `ministry2-e2` (edit): In the paragraph about 초소규모 사업장 점검, move the inline column break so it falls just before "질식 위험 작업부터입니다." (after "벌목, "). Keep the rest of that paragraph. Do not edit the other 다음 단계 paragraphs.
5. `ministry2-e3` (edit): Remove the inline page break from the 다음 단계로 대학일자리+센터 paragraph. Keep it as one paragraph, with a space where the break was. Do not remove the page break between paragraphs before "3. 노동시간".
6. `ministry2-r` (edit): Insert an odd-page section break before "3. 노동시간" so that heading always starts on a right-hand page, and set that section to three columns.

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
