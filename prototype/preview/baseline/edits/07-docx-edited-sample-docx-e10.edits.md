# What 07-docx-sample-docx-e10.docx changed

Every scripted edit (E1–E9, and the formatting edits F1–F4) in one revision, as direct changes. Source: `sample-docx.docx`; compare with `05-docx-sample-docx-original.docx`.

## The edits

- E1 figure (change "10" to "105" in a paragraph with a bookmark)
- E2 insert before drawing (new paragraph before block 45)
- E3 delete formatted paragraph (delete block 27 ("Font styles 18 point"))
- E4 move section (move the section at block 41 ("Images") (23 blocks) before the one at block 30 ("Bullets & numbering"))
- E5 restyle (block 1 ("This is a document exhibiting "…) → style "List Paragraph")
- E6 table cell (no merged cell in any modelled table: append ' (rev.)' to cell [0, 0, 0])
- E7 edit across a run boundary (replace "s.  " (straddles two differently formatted runs) with 'EDITED')
- E8 split paragraph (split block 23 ("A short paragraph with 105 poi"…) at offset 8 of 66)
- E9 merge paragraphs (join block 23 ("A short") and block 24 ("paragraph with 105 points spac"…))
- F1 first-line indent (first-line=10pt on body paragraphs block 4 ("Some text."))
- F2 restyle (style "heading 1" (paragraphs block 2 ("This is style Heading 1"), block 5 ("Tables") and 4 more) → color=#1F4E79 space-before=18pt)
- F3 new style (new style "Callout" (fill=#FFF2CC border-left="2.25pt solid #C00000") given to block 12 ("Left indent"))
- F4 cell fill (fill=#DDEBF7 on the header row of the table at block 7 (cells "Cell text (rev.)" to the last))

## Model text, before and after (unified diff; `-` original, `+` edited)

```diff
--- original (model text)
+++ edited (model text)
@@ -5,25 +5,26 @@
 ---
 <style name="Normal" indent-left=4.3pt indent-right=4.3pt line-spacing=100% font=Calibri size=11pt color=#000000/>
 <style name="Title" border-bottom="1pt solid accent1" space-after=15pt font="?? ??" size=26pt color=tx2-25%/>
-<style name="heading 1" space-before=24pt font="?? ??" size=14pt color=accent1-25% bold/>
 <style name="List Paragraph" indent-left=36pt/>
+<style name="heading 1" space-before=18pt font="?? ??" size=14pt color=#1F4E79 bold/>
+<style name="Callout" fill=#FFF2CC border-left="2.25pt solid #C00000" indent-left=10pt/>
 
 <div style="Title">Docx sample document</div>
 
-This is a document exhibiting basic docx features.  
+<div style="List Paragraph">This is a document exhibiting basic docx featureEDITED</div>
 
 # This is style Heading 1
 
 <p/>
 
-Some text.
+Some text. {first-line=10pt}
 
 # Tables
 
 <p/>
 
 {indent-left=0pt}
-| Cell text |  | {fill=bg1-15%} Shaded grey |
+| Cell text (rev.) |  | Shaded grey | {fill=#DDEBF7}
 |---|---|---|
 | Vertical merge |  | {fill=bg1-15%} Shaded grey |
 | ^^ |  |  |
@@ -37,7 +38,7 @@
 
 <p/>
 
-Left indent
+<div style="Callout">Left indent</div>
 
 Centred  {align=center}
 
@@ -58,7 +59,7 @@
 
 Normal {align=justify}
 
-A short paragraph with 10 points spacing before, 20 points after. {align=justify indent-left=4.25pt indent-right=4.25pt space-before=10pt space-after=20pt}
+A short paragraph with 105 points spacing before, 20 points after. {align=justify indent-left=4.25pt indent-right=4.25pt space-before=10pt space-after=20pt}
 
 # Run properties
 
@@ -66,38 +67,19 @@
 
 Font styles [Aerial Black]{font="Arial Black"} {align=justify}
 
-Font styles [18 point]{size=18pt} {align=justify}
-
 Font styles **bold** {align=justify}
 
 Font styles *italic* {align=justify}
 
 Font styles <u>underline</u> {align=justify}
 
-# Bullets & numbering
-
-<p/>
-
-Bullets {align=justify}
-
-- Level 1 {align=justify}
-  - Level 2 {align=justify}
-
-<p/>
-
-Numbering {align=justify}
-
-1. Level 1 {align=justify}
-   1. Level 2 {align=justify}
-      1. Level 3 {align=justify}
-
-<p/>
-
 # Images
 
 <p/>
 
 Jpeg:
+
+Inserted paragraph before the drawing.
 
 <keep id="kerl8" kind="drawing" summary="C:\Documents and Settings\Jason Harrop\My Documents\tmp-test-docs\pangolin.jpeg"/>
 
@@ -134,3 +116,22 @@
 This line contains a soft return<br/>and here it continues {indent-left=0pt}
 
 <p/>
+
+# Bullets & numbering
+
+<p/>
+
+Bullets {align=justify}
+
+- Level 1 {align=justify}
+  - Level 2 {align=justify}
+
+<p/>
+
+Numbering {align=justify}
+
+1. Level 1 {align=justify}
+   1. Level 2 {align=justify}
+      1. Level 3 {align=justify}
+
+<p/>
```
