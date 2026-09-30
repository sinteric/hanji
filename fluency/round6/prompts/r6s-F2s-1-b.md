You work with office files stored as plain text. Below are the syntax documentation, the file itself, and 3 tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

A document file is plain text. It begins with a front matter block between two `---` lines; keep it as it is.

### Text

- One line is one paragraph. `#` to `######` start a heading of that level. `- ` starts a bullet item and `1. ` a numbered item; a nested item is indented under its parent (two spaces under `- `).
- `<div style="Name">text</div>` is a paragraph in the named style. A line holding only `<p/>` or `<p style="Name"/>` is an empty paragraph.
- Inside a paragraph: `**bold**`, `*italic*`, `<u>underline</u>`, `~~strike~~`, `<br/>` (a line break). A literal `*`, `[`, `]` or `{` in text is written `\*`, `\[`, `\]`, `\{`.
- `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (a picture, a footnote, a comment). Leave every one exactly as it is. `<pagebreak/>` is a page break.

### Tables

A table is a pipe table: a header row, a `|---|` line, then the other rows, one line per row, one cell per column. `^^` as a whole cell means the cell is merged into the one above; `||` (two pipes with nothing between) means the cell to the left extends into this column. In a cell, `<p/>` starts another paragraph and `<p style="Name"/>` starts one in that style; a cell that begins with `<p style="Name"/>` has its first paragraph in that style. An empty cell is `|  |`.

### Formatting

Formatting comes from named styles, listed once at the top of the file; a defaults line holds what most paragraphs of a page share; each paragraph, cell and stretch of text shows only what differs from those.

- **The style lines.** After the front matter, each style the document uses has a line `<style name="Name" …/>`. The first is the default style: its line is complete, and a property it leaves out is 0pt, none or off (and `align` is left). Every other style line holds only what differs from the default style; a property it leaves out is the default style's. A heading's style is its level's, a `<div>` names its style, a list item names it in its `{…}` (`style="Name"`), and any other paragraph is in the default style. Change a style line to change every paragraph in that style that does not set the property itself (for the default style: nor has it from a defaults line).
- **The defaults lines.** A line `<defaults …/>` holds the formatting most paragraphs share from that line to the next `<defaults …/>` line (usually a page's worth, after a `<pagebreak/>`); `<defaults/>` holds nothing. A paragraph's property is, first found: its own `{…}`, the table line (in a table), its style's line when the style is not the default style and the line sets the property, the defaults line in force, the default style's line. So a paragraph in the default style takes the defaults line's values, and another style's own values win over them. A change to a defaults line changes every paragraph after it, up to the next one, that does not set the property itself or through its style.
- **Paragraph:** `{…}` at the end of the paragraph's line (after `</div>` for a `<div>`) is its own formatting: the properties that differ from what its style and the defaults line give it. A property left out comes from them, in the order above.
- **Text in a paragraph:** `[text]{…}` gives that stretch the text properties written (font, size, color); the rest of the paragraph has the paragraph's. Bold, italic, underline and strike are the marks `**`, `*`, `<u>`, `~~`.
- **Cell:** `{…}` at the very start of a cell is the cell's formatting: fill, borders and vertical alignment. A cell's paragraphs have their own `{…}` at their end. `{…}` after a row's last `|` applies to every cell of that row. The line `{…}` just before a table applies to every cell and every cell paragraph of the table. A cell's own value wins over its row's, and a row's over the table line's; a table line may also hold `style="Name"`.

### The vocabulary

`key=value` pairs separated by spaces; a value with a space is quoted. Lengths are points (`12pt`); colours are `#RRGGBB`.

- paragraph: `align` (left, center, right, justify, distribute), `indent-left`, `indent-right`, `first-line` (a positive value indents the first line, a negative one is a hanging indent), `space-before`, `space-after`, `line-spacing` (`160%`, `14pt` exact, `"at-least 14pt"`), `fill`, `border-top` / `-right` / `-bottom` / `-left`
- text: `font`, `size`, `color`, `bold`
- cell: `fill`, `border` (all four sides) or `border-top` / `-right` / `-bottom` / `-left`, `valign` (top, middle, bottom)
- a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`

`fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written; they can be replaced by a colour but not written or changed. Border styles other than the four above (triple, wave, 3d) are kept while left as written. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow. A new style is a new style line, after the others, with a name no other style has (a name that is already a style is an error); like the other lines it holds what differs from the default style. A paragraph takes it like any style: `<div style="Name">…</div>`, a list item's `style="Name"`, `<p style="Name"/>` in a cell.

Example:

```
<style name="바탕글" align=justify line-spacing=160% font=바탕 size=10pt color=#000000/>
<style name="개요 1" size=16pt bold/>

<defaults line-spacing=170% size=11pt/>
# 개요
본문 문단입니다. [강조]{color=#C00000} 부분이 있습니다. {first-line=10pt}
{border="0.5pt solid #000000"}
| {fill=#D9D9D9} 구분 {align=center} | {fill=#D9D9D9} 내용 {align=center} |
|---|---|
| 합계 | 215 | {border-top="1.5pt double #000000"}
```

## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
---
type: document
format: hwpx
schema: 1
---

<style name="바탕글" align=justify line-spacing=160% font=함초롬바탕 size=10pt color=#000000/>
<style name="개요 2" indent-left=10pt space-before=5pt font=휴먼명조 size=15pt bold/>
<style name="개요 3" indent-left=20pt space-before=5pt font=휴먼명조 size=15pt/>
<style name="개요 4" indent-left=30pt space-before=3pt font=휴먼명조 size=14pt/>

<defaults line-spacing=170% font=HY견명조 size=18pt/>
| {fill=#E3F8FF border-top="0.34pt solid #000000" border-bottom="2.83pt solid #7F7F7F"} 3D 프린팅 기술의 등장과 {align=center line-spacing=160%}<p/>기술 발전에 따른 문제점과 정부의 대처 방안 {align=center line-spacing=160% size=20pt} |
|---|

{border-bottom="0.28pt solid #999999" border-left="0.28pt solid #999999" line-spacing=160%}
| {fill=#F2F2F2 border-top="0.28pt solid #999999" border-right="0.28pt solid #999999"} **Ⅰ** {align=center} |  | {border-left=none} 3D 프린팅 기술의 등장 |
|---|---|---|

- 개념 {style="개요 2" space-before=25pt space-after=5pt}
  - 플라스틱 액체<keep id="kd1j9" kind="footnote" summary="footnote: 플라스틱 액체란"/>와 같은 원료를 사출해 3차원 모양의 고체 물질을 자유롭게 찍어내는 기술 {style="개요 3"}
  - 산업용 샘플을 찍어내던 것에서 발전해 시계, 신발, 휴대전화 케이스, 자동차 부속품까지 출력 {style="개요 3"}
  - 3D기술을 활용하면 비용 효율성을 높일 수 있기 때문에 변화가 빠른 제조업 분야에 활용도가 높고 일본, 미국 등에서는 본격 상용화 {style="개요 3" indent-left=19pt}
  - 소비재, 의료, 교육, 건축, 자동차, 항공/우주 관련 산업에도 도입 {style="개요 3"}
- 3D 프린팅<keep id="kjgcj" kind="footnote" summary="footnote: 3D 프린팅은 플라스틱 액체와 같은 원료"/> 기술의 장점 {style="개요 2" space-before=25pt space-after=5pt}
  - 기존의 틀을 만들어 찍어내는 방법은 하나의 물건을 만드는데 매우 비용이 많이 들지만 틀 없이 원료를 한 겹씩 쌓아서 물건을 만들기 때문에 다품종 소량 생산이 가능 {style="개요 3"}
  - 아무리 복잡한 모양이라도 간편하게 만들어 낼 수 있기 때문에 정교하고 복잡한 모양도 한 번에 인쇄 가능 {style="개요 3"}
    - 3D 프린터로 만들 수 있는 물건은 사실상 무궁무진하다고 볼 수 있음 {style="개요 4"}

<pagebreak/>
<defaults font=HY견명조 size=16pt/>

{border-bottom="0.28pt solid #999999" border-left="0.28pt solid #999999"}
| {fill=#F2F2F2 border-top="0.28pt solid #999999" border-right="0.28pt solid #999999"} **Ⅱ** {align=center} |  | {border-left=none} 기술발전에 따른 예상되는 사회적 변화<keep id="kurr4" kind="footnote" summary="footnote: 표안의 각주"/>와 문제점 {indent-left=-1pt} |
|---|---|---|

- 사회적 변화 예상 {style="개요 2" space-before=15pt space-after=5pt}
  - 시제품 제작 시간과 비용 절감 및 이를 통한 제품 혁신의 가속화 {style="개요 3"}
  - 제품 생산에 있어 짧은 Setup 시간, 공구 작동의 오차 감소에 따른 생산성 향상 {style="개요 3"}
  - 금형의 냉각회로, 항공기 부품 등 제품 수명 주기가 길고 재료비가 비싸며, 제품 형상이 복잡한 분야에서 그 잠재력을 최대한 활용 {style="개요 3"}
  - 상용화 단계에 있는 3D 바이오프린터 기술의 완성도가 높아져 개인 맞춤형 조직, 장기 생산 가능으로 수명 연장 {style="개요 3"}
    - 이미 보청기 개발, 샴쌍둥이<keep id="k6nvg" kind="footnote" summary="footnote: 썀쌍둥이란 샴이란"/> 분리 수술, 인공 뼈 이식, 의족 등에서 3D 프린터 활용 사례가 확인 {style="개요 4"}
- 문제점 분석 {style="개요 2" space-before=15pt space-after=10pt}
  - (일반 접근성) 3D 프린팅에 대한 높은 관심에 비해 일반 국민이 이를 체험‧활용할 수 있는 장비 등의 인프라 부족 {style="개요 3"}
  - (인력부족) 3D 프린팅의 도입‧보급이 확대되고 있으나, 이를 활용할 수 있는 인력이 부족하고, 관련 교육, 인재양성 체계 등 미흡 {style="개요 3"}
    - 모델링, 데이터 검증, 유지보수 등 3D 프린팅 각 분야별 전문 인력이 필수적이고 교육프로그램 및 강사형 인재 부족으로 인력수급 불균형 우려 {style="개요 4"}
  - (범죄에 이용되는 사례) 집에서도 설계도만 있으면 손쉽게 총기류를 만들 수 있다는 점에서 위험성이 제기 {style="개요 3"}
    - 일본에서 3D 프린터로 권총을 만든 남자가 체포, 해외 사이트에서 총 설계도의 데이터를 다운로드하고 자체제작으로 조립 {style="개요 4"}
    - 자금력 있는 범죄 조직이 3D 프린터를 사용해 대량의 불법 마약을 대량 생산할 수 있다고 이미 예상 {style="개요 4"}
    - 의약품의 레시피만 알면 이론적으로 집에서 약을 만들 수도 있어 가짜 의약품 유통의 우려 {style="개요 4"}
- 기대효과<keep id="km2j4" kind="footnote" summary="footnote: 여기서의 기대효과"/> {style="개요 2" space-before=15pt space-after=10pt}
  - 제조업, 의료, IT 분야 등 다방면에서 기술 패러다임을 바꾸며, 산업혁신을 이끌 것으로 기대되고 있음 {style="개요 3"}
    - 맞춤형 보청기나 의족, 의수 제작, 인공 장기 제작까지 사용될 수도 있을 것으로 기대 {style="개요 4"}
  - 최근 3D 프린팅 기술에 관심이 집중되는 이유는 3D 프린터 가격이 낮아져 대중화 될 것으로 전망 {style="개요 3"}
    - 시제품 제작에만 그쳤던 3D 프린팅 기술이 집에서의 개인화, 맞춤화된 장난감<keep id="kuni3" kind="footnote" summary="footnote: 커스터마이징된 장난감"/>, 액세서리 제작에 사용 {style="개요 4"}
- 제도적 대처 방안 {style="개요 2" space-before=15pt space-after=10pt}
  - 기술의 대중화에 있어 가장 큰 과제는 표준화<keep id="kevsc" kind="footnote" summary="footnote: 다양한 업체에서 만들어지는 제품의 재료 표준화"/>이며, 장기적으로 보면 모든 상품을 쉽게 복제할 수 있는 저작권 보호 제도 마련 시급 {style="개요 3"}
  - 지식재산권법<keep id="km0kd" kind="footnote" summary="footnote: 지식재산법의 재정"/>, 제조물책임법<keep id="ke22v" kind="footnote" summary="footnote: 제조되는 물건들에 대한 관련 법들"/>, 총포도검법 등 정미 및 바이오메디컬 적용 규정 도입 {style="개요 3"}
  - 산업생태계 전반에서 동반성장을 하도록 하려면 불법․무단제조 제품의 유통과 판매 등에 대한 선제적인 제도정비 {style="개요 3"}
  - 기존 산업과의 갈등을 줄이기 위한 범정부 차원의 노력 {style="개요 3"}
  - 전문인력 양성 및 기술 수용성 확산 {style="개요 3"}
    - 저가의 장비 및 S/W 보급, 3D 프린팅 교육과정 신설 및 교육 확대, 올바른 이용 홍보가 필요 {style="개요 4"}

<pagebreak/>
<defaults/>

| {fill=#FFF0C3 border-top="0.34pt solid #000000" border-bottom="2.83pt solid #7F7F7F"} **3D 프린팅 기술의 미래와 전망** {align=center font=나눔고딕 size=20pt} |
|---|

<pagebreak/>

<p/>
=== FILE END ===

## Tasks

1. `fn-e5` (edit): Change the style 개요 3 so that all of its paragraphs turn navy (#1F3864).
2. `fn-e8` (edit): Indent the first line of every body paragraph (the second- and third-level list items) by 10pt.
3. `fn-e10` (edit): Create a new style named "개요 3 강조" that looks exactly like 개요 3 but with dark red text (#C00000), and apply it to the two list items "(일반 접근성)…" and "(인력부족)…". Their other formatting stays as it is.

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Edit tasks: answer with exact text edits. Each edit is `{"old": "...", "new": "..."}`: `old` is copied exactly from the file (same spaces and characters) and occurs exactly once in it, and `new` replaces it. Edits apply in order. Change only what the task asks for; every other line must stay exactly as it is.
- If a task cannot be done in this file format, do not edit: answer `REFUSE: <one sentence why>`.

## Answer format

Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "…", "text": "ANSWER: …"},
  {"task_id": "…", "edits": [{"old": "…", "new": "…"}]},
  {"task_id": "…", "text": "REFUSE: …"}
]}
```

One entry per task, in the order of the tasks.
