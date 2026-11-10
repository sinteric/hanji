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

# 한빛 노트 출시 제안서

제품기획팀은 한빛 노트의 11월 출시를 제안합니다. 핵심 기능 3가지는 공동 편집, 오프라인 저장, 한글 교정입니다.

## 일정

출시일 11월 20일을 기준으로 마케팅은 11월 6일부터, CS 교육은 11월 13일부터입니다.<br type="page"/>인쇄물 마감은 출시일 11월 20일 일주일 전인 11월 13일입니다.

<div style="표 제목">표 1. 채널별 목표</div>

{style="표 눈금"}
| 채널 | 1주 | 4주 | 비고 |
|---|---|---|---|
| 자사몰 | 1,200 | 4,800 | 얼리버드 |
| 오픈마켓 | 800 | 3,200 | 쿠폰 |
| 합계 | 2,000 | 8,000 | — |

## 메시지

왼쪽 칼럼은 기업 고객용입니다.<br type="column"/>오른쪽 칼럼은 개인 고객용입니다. 핵심 기능 3가지를 같은 순서로 적습니다.

핵심 기능 3가지를 포스터 상단에도 반복합니다. 출시일 11월 20일을 모든 인쇄물에 넣습니다.

- 공동 편집은 최대 20명.
- 오프라인 저장은 문서당 200쪽.
- 한글 교정은 맞춤법과 띄어쓰기.

<div style="Note">출시일 11월 20일은 이사회가 승인한 날짜이며, 바꾸려면 이사회를 다시 엽니다.</div>

<keep id="k2" kind="drawing" summary="제품 렌더링, 각도 3개"/>

<pagebreak/>

# 첨부

첨부 1. 가격표. 첨부 2. 경쟁 제품 비교. 핵심 기능 3가지의 상세 설명은 별도 위키에 있습니다.
=== FILE END ===

## Tasks

1. `launch3-q1` (read): The file repeats 출시일 11월 20일. In the paragraph that also names 마케팅 and CS 교육, what are the first four words after the inline page break? In the paragraph that names 왼쪽 칼럼, what are the first four words after the inline column break? Answer as `ANSWER: <page> | <column>`.
2. `launch3-w` (write): Write a new document with exactly these blocks, in this order, and nothing else: a heading level 1 "출시 당일 공지"; a default-style paragraph "한빛 노트 판매를 시작합니다." then an inline page break then "자사몰 쿠폰은 오늘 자정까지입니다."; a default-style paragraph "기업 고객 안내." then an inline column break then "개인 고객 안내."
3. `launch3-e1` (edit): In the opening paragraph (제품기획팀은), insert an inline page break after the first sentence (after "제안합니다."). Do not touch the other paragraph that contains 출시일 11월 20일 and already has a page break.
4. `launch3-e2` (edit): In the paragraph that begins 왼쪽 칼럼은 기업 고객용입니다, move the inline column break so it falls after "개인 고객용입니다." (just before "핵심 기능 3가지를 같은 순서로"). Do not edit the other 핵심 기능 3가지 sentence.
5. `launch3-e3` (edit): Remove the inline page break from the paragraph that begins 출시일 11월 20일을 기준으로 마케팅은. Keep it as one paragraph, with a space where the break was. Do not remove the page break between paragraphs before 첨부.
6. `launch3-r` (edit): Add a continuous section break after 메시지 so the rest of the file is two newspaper columns with a 1 cm gap, without starting a new page.

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
