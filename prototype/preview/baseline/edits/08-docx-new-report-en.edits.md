# What 24-docx-blank-report-en-new.docx changed

A new report from hanji's blank docx: headings, lists, emphasis, a merged table. Source: `blank-report-en.docx`.

## The edits

- Model text written:

```
# Quarterly sales report
Sales grew **12%** over the same quarter last year, and *34* new customers signed.

## Highlights
- New customers: 34
  - Capital region: 21
- Returning customers: 112

Next steps:

1. Keep the regional teams
1. Open two branches in Busan

## Figures
| Region | Branch | Sales |
|---|---|---|
| Seoul | Gangnam | 120 |
| ^^ | Jongno | 95 |
| Total || 215 |

<div style="Quote">Figures are preliminary.</div>
```


## Model text, blank package to new document (unified diff)

```diff
--- original (model text)
+++ edited (model text)
@@ -3,6 +3,27 @@
 format: docx
 schema: 1
 ---
-<style name="Normal" space-after=8pt line-spacing=107.92% font="맑은 고딕" size=11pt color=#000000/>
+# Quarterly sales report
 
-<p/>
+Sales grew **12%** over the same quarter last year, and *34* new customers signed.
+
+## Highlights
+
+- New customers: 34
+  - Capital region: 21
+- Returning customers: 112
+
+Next steps:
+
+1. Keep the regional teams
+1. Open two branches in Busan
+
+## Figures
+
+| Region | Branch | Sales |
+|---|---|---|
+| Seoul | Gangnam | 120 |
+| ^^ | Jongno | 95 |
+| Total || 215 |
+
+<div style="Quote">Figures are preliminary.</div>
```
