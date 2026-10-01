# What 25-docx-blank-report-ko-new.docx changed

A new Korean report from hanji's blank docx (DESIGN.md §5.2's example). Source: `blank-report-ko.docx`.

## The edits

- Model text written:

```
# 3분기 영업 보고
매출은 전년 대비 **12%** 증가했다.

<div style="Quote">신규 고객 34곳 중 21곳이 수도권.</div>

- 신규 고객 34곳
  - 수도권 21곳

다음 분기:

1. 목표 매출 1,300억 원

<p/>

| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |

| 지역 | 매출 | 증감 |
|---|---|---|
| 수도권 | 1,204 | +15% |
| 지방 | 812 | +4%<p/>부산 신규 2곳 |
```


## Model text, blank package to new document (unified diff)

```diff
--- original (model text)
+++ edited (model text)
@@ -3,6 +3,28 @@
 format: docx
 schema: 1
 ---
-<style name="Normal" space-after=8pt line-spacing=107.92% font="맑은 고딕" size=11pt color=#000000/>
+# 3분기 영업 보고
+
+매출은 전년 대비 **12%** 증가했다.
+
+<div style="Quote">신규 고객 34곳 중 21곳이 수도권.</div>
+
+- 신규 고객 34곳
+  - 수도권 21곳
+
+다음 분기:
+
+1. 목표 매출 1,300억 원
 
 <p/>
+
+| 지역 | 지점 | 매출 |
+|---|---|---|
+| 서울 | 강남 | 120 |
+| ^^ | 종로 | 95 |
+| 합계 || 215 |
+
+| 지역 | 매출 | 증감 |
+|---|---|---|
+| 수도권 | 1,204 | +15% |
+| 지방 | 812 | +4%<p/>부산 신규 2곳 |
```
