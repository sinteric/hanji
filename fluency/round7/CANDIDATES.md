# Round 7 candidates: inline breaks, xlsx shapes/groups, dataBar CF

Today three kinds of content stay `<keep/>` (docs/keep-census.md). The owner approved adding syntax for each,
but each must be decided by a fluency test first. This file shows the candidates on the corpus samples that
produce those keeps. The measurements are in [RESULTS.md](RESULTS.md).

## 1. DOCX inline page and column breaks (`breaks`)

`<pagebreak/>` is already a **block**: a line of its own, a page break between paragraphs ([guide.md](../../crates/hanji-store/src/guide.md), DESIGN.md §5.2). `<br/>` is a line break inside a paragraph. An inline `w:br type=page` mixed with text (a TOC heading in tdf154481.docx) and an inline `w:br type=column` (before the repeated "test" in tdf124637_sectionMargin.docx) stay `<keep kind="break"/>`, because they are not a whole-paragraph page break and not a line break.

**A.** Dedicated tags inside the paragraph, the block `<pagebreak/>` unchanged:

```
수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.<pagebreak/>첨부 1의 지점별 표는 10월 말 확정치입니다.
서울 14곳, 경기 7곳.<columnbreak/>부산 5곳, 대구 3곳.
<pagebreak/>
# 부록
```

**B.** Reuse `<br/>` with a `type`, the block `<pagebreak/>` unchanged:

```
수도권 매출은 전년 대비 +12% 늘었고, 지방은 +4%입니다.<br type="page"/>첨부 1의 지점별 표는 10월 말 확정치입니다.
서울 14곳, 경기 7곳.<br type="column"/>부산 5곳, 대구 3곳.
<pagebreak/>
# 부록
```

A line holding only `<pagebreak/>` is still the block form under both. A section break, an odd-page break and a change of column count stay refused (DESIGN.md §10.5).

## 2. XLSX drawing shapes and groups (`shapes`)

xlsx drawings today are placeholders: `shape` (simple-monthly-budget.xlsx, the "Data Entry Tip" callout whose two-cell anchor is B25:D27; WithDrawing.xlsx, TextBox 8) and `group` (ExcelWithAttachments.xlsm, Group 1 of a rectangle and a text box). Charts and pictures stay `<keep/>`. The existing sheet lines are self-closing (`<hyperlink/>`, `<note/>`, `<validation/>`, `<conditional-format/>`).

**A.** A shape's text is the element's body, as Presentation `<shape>` is; a group nests its shapes, as pptx `<group>` does:

```
<shape name="입력 안내" anchor="B25:D27">아래에 입력하면 표가 늘어납니다.</shape>
<group name="신호등">
<shape name="매출 배지" anchor="F2:G3">매출 +12%</shape>
<shape name="이익 배지" anchor="F4:G5">이익 +8%</shape>
</group>
```

**B.** Every shape is a self-closing line in the style of `<note/>`; membership is `group="Name"`:

```
<shape name="입력 안내" anchor="B25:D27" text="아래에 입력하면 표가 늘어납니다."/>
<shape name="매출 배지" anchor="F2:G3" text="매출 +12%" group="신호등"/>
<shape name="이익 배지" anchor="F4:G5" text="이익 +8%" group="신호등"/>
```

`anchor` is the A1 range of Excel's two-cell anchor (from-cell:to-cell). A picture, a chart, a connector, rotation, a fill and a shadow stay refused.

## 3. XLSX dataBar conditional formats (`databar`)

`cellIs` and `expression` are already `<conditional-format/>` lines. A `dataBar` stays a placeholder: simple-monthly-budget.xlsx E4 is `type=dataBar`, `cfvo type=num val=0` / `type=num val=TotalMonthlyIncome`, colour theme 4 (Office blue `#638EC6`).

**A.** Attributes on the existing tag; min/max are one token (`min`, `max`, `num:0`, `percent:100`, `formula:=A1`); fill is the cell-format colour:

```
<conditional-format range="E4:E16" type="dataBar" min="num:0" max="num:월간수입합계" fill="#638EC6"/>
```

**B.** Split type and value, Excel's own names (`min-type` / `max-type` / `color`):

```
<conditional-format range="E4:E16" type="dataBar" min-type="num" min="0" max-type="num" max="월간수입합계" color="#638EC6"/>
```

A color scale, an icon set, a gradient bar, `showValue` and a data-bar axis stay refused.
