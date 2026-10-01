# What 52-xlsx-korean-sales-ops.xlsx changed

Range operations that change the inputs of formulas, and format operations (fill, borders, font) on the ranges they name. Source: `korean-sales.xlsx`; compare with `50-xlsx-korean-sales-original.xlsx`.

## The edits

- Operations: `[{"op": "set", "range": "매출!D5", "values": [[18420000]]}, {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-04", "지점": "강남", "제품군": "가전", "매출": 12400000, "원가": 9000000}]}, {"op": "format", "range": "매출!A3:F3", "set": {"fill": "#D9D9D9", "bold": true, "border-bottom": "0.75pt solid #000000"}}]`
- Values the export holds: formula results as computed by hanji, except the cells hanji left to Excel (listed below, if any). A formula cell left blank here had no cached value in the source; Excel computes it when it opens the file.

```
<data sheet="요약" range="A1:B8">
| row | A | B |
|---|---|---|
| 1 | 지점 | 합계 |
| 2 | 강남 | 112,071,000 |
| 3 | 서초 | 111,527,000 |
| 4 | 송파 | 108,272,000 |
| 5 | 분당 | 104,836,000 |
| 6 | 일산 | 92,774,000 |
| 7 |  |  |
| 8 | 총계 | 529,480,000 |
</data>
```


```
<data sheet="매출" range="A3:F6">
| row | A | B | C | D | E | F |
|---|---|---|---|---|---|---|
| 3 | 월 | 지점 | 제품군 | 매출 | 원가 | 이익 |
| 4 | 2026-01 | 강남 | 가전 | 10,269,000 | 7,410,000 |  |
| 5 | 2026-01 | 강남 | 모바일 | 18,420,000 | 6,480,000 | 11,940,000 |
| 6 | 2026-01 | 강남 | 생활 | 6,408,000 | 4,560,000 |  |
</data>
```


## Cell values and model text, before and after (unified diff; the row windows above, then the structure)

```diff
--- original (model text)
+++ edited (model text)
@@ -2,13 +2,13 @@
 | row | A | B |
 |---|---|---|
 | 1 | 지점 | 합계 |
-| 2 | 강남 |  |
-| 3 | 서초 |  |
-| 4 | 송파 |  |
-| 5 | 분당 |  |
-| 6 | 일산 |  |
+| 2 | 강남 | 112,071,000 |
+| 3 | 서초 | 111,527,000 |
+| 4 | 송파 | 108,272,000 |
+| 5 | 분당 | 104,836,000 |
+| 6 | 일산 | 92,774,000 |
 | 7 |  |  |
-| 8 | 총계 |  |
+| 8 | 총계 | 529,480,000 |
 </data>
 
 <data sheet="매출" range="A3:F6">
@@ -16,7 +16,7 @@
 |---|---|---|---|---|---|---|
 | 3 | 월 | 지점 | 제품군 | 매출 | 원가 | 이익 |
 | 4 | 2026-01 | 강남 | 가전 | 10,269,000 | 7,410,000 |  |
-| 5 | 2026-01 | 강남 | 모바일 | 8,761,000 | 6,480,000 |  |
+| 5 | 2026-01 | 강남 | 모바일 | 18,420,000 | 6,480,000 | 11,940,000 |
 | 6 | 2026-01 | 강남 | 생활 | 6,408,000 | 4,560,000 |  |
 </data>
 
@@ -28,11 +28,12 @@
 
 <format default font=Calibri size=11pt color=tx1/>
 
-<sheet name="매출" range="A1:H48">
+<sheet name="매출" range="A1:H49">
 
 <format range="A1" align=center size=14pt color=#000000 bold/>
+<format range="A3:F3" fill=#D9D9D9 border-bottom="0.75pt solid #000000" bold/>
 
-<table name="Sales" range="A3:F48">
+<table name="Sales" range="A3:F49">
 | column | type | format | formula |
 |---|---|---|---|
 | 월 | date | yyyy-mm |  |
@@ -45,9 +46,9 @@
 
 <keep id="k8a8r" kind="merged-cells" summary="A1:F1"/>
 
-<keep id="k0k51" kind="conditional-format" summary="F4:F48"/>
+<keep id="k0k51" kind="conditional-format" summary="F4:F49"/>
 
-<keep id="kqjeg" kind="data-validation" summary="B4:B48"/>
+<keep id="kqjeg" kind="data-validation" summary="B4:B49"/>
 
 <keep id="kn7g2" kind="hyperlinks" summary="H4"/>
 
```
