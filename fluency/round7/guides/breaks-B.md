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
 