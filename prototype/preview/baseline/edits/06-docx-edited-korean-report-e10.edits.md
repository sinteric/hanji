# What 03-docx-korean-report-e10.docx changed

Every scripted edit (E1–E9, and the formatting edits F1–F4) in one revision, as direct changes. Source: `korean-report.docx`; compare with `01-docx-korean-report-original.docx`.

## The edits

- E1 figure (change "1" to "15" in a paragraph with a comment)
- E2 insert before drawing (new paragraph before block 6)
- E3 delete formatted paragraph (delete block 1 ("매출은 전년 대비 12% 증가했다. 신규 고객은 34곳"…))
- E4 move section (move the section at block 9 ("향후 계획") (5 blocks) before the one at block 3 ("지역별 현황"))
- E5 restyle (block 2 ("지방 지점의 매출은 15,204억 원으로 15% 성장했"…) → style "Body Text")
- E6 table cell (cell next to a merged cell: append ' (rev.)' to cell [2, 1, 0])
- E8 split paragraph (split block 2 ("지방 지점의 매출은 15,204억 원으로 15% 성장했"…) at offset 19 of 33)
- E9 merge paragraphs (join block 2 ("지방 지점의 매출은 15,204억") and block 3 ("원으로 15% 성장했다."))
- F1 first-line indent (first-line=10pt on body paragraphs block 4 ("4분기에는 부산과 대구에 지점을 연다."))
- F2 restyle (style "Heading 2" (paragraphs block 3 ("향후 계획"), block 8 ("지역별 현황")) → color=#1F4E79 space-before=18pt)
- F3 new style (new style "Callout" (fill=#FFF2CC border-left="2.25pt solid #C00000") given to block 6 ("부록: 지점 목록"))
- F4 cell fill (fill=#DDEBF7 on the header row of the table at block 12 (cells "지역" to the last))

## Model text, before and after (unified diff; `-` original, `+` edited)

```diff
--- original (model text)
+++ edited (model text)
@@ -6,37 +6,39 @@
 <style name="Normal" line-spacing=100% font="Noto Sans CJK KR" size=12pt color=#000000/>
 <style name="Heading 1" size=16pt bold/>
 <style name="Note" fill=#EEEEEE/>
-<style name="Heading 2" size=13pt bold/>
+<style name="Body Text" space-after=7pt line-spacing=115%/>
+<style name="Heading 2" space-before=18pt size=13pt color=#1F4E79 bold/>
 <style name="Block Quotation" indent-left=28.35pt/>
+<style name="Callout" fill=#FFF2CC border-left="2.25pt solid #C00000" indent-left=10pt/>
 
 # 3분기 영업 보고
 
-매출은 전년 대비 **12%** 증가했다.<keep id="khc1l" kind="footnote" summary="footnote: 내부 집계 기준."/> 신규 고객은 34곳이며, 그중 [21곳이 수도권]{color=#C00000}이다.
-
 <div style="Note">신규 고객 34곳 중 21곳이 수도권.</div>
 
-지방 지점의 매출은 1,204억 원으로 [15% 성장]{size=14pt color=#1F4E79}했다.<keep id="ku25a" kind="comment" summary="comment: 지방 수치 재확인 필요"/>
+<div style="Body Text">지방 지점의 매출은 15,204억 원으로 [15% 성장]{size=14pt color=#1F4E79}했다.<keep id="ku25a" kind="comment" summary="comment: 지방 수치 재확인 필요"/></div>
+
+## 향후 계획
+
+4분기에는 [부산]{color=#C00000}과 [대구]{color=#C00000}에 지점을 연다. {first-line=10pt}
+
+<div style="Block Quotation">“고객이 있는 곳에 지점이 있어야 한다.” — 영업본부장</div>
+
+<div style="Callout">부록: 지점 목록</div>
+
+서울 강남, 서울 종로, 경기 분당, 인천 송도.
 
 ## 지역별 현황
 
 아래 표는 지점별 매출을 정리한 것이다.
 
+Inserted paragraph before the drawing.
+
 <keep id="kml28" kind="drawing" summary="조직도, 상자 5개"/>
 
-| 지역 | 지점 | 매출 |
+| 지역 | 지점 | 매출 | {fill=#DDEBF7}
 |---|---|---|
 | 서울 | 강남 | 120 |
-| ^^ | 종로 | 95 |
+| ^^ | 종로 (rev.) | 95 |
 | 합계 || 215 |
 
 강남 지점은 *전년 대비* <keep id="k83hu" kind="tracked-insert" summary="ins by 김민지: 8%"/>성장했다.
-
-## 향후 계획
-
-4분기에는 [부산]{color=#C00000}과 [대구]{color=#C00000}에 지점을 연다.
-
-<div style="Block Quotation">“고객이 있는 곳에 지점이 있어야 한다.” — 영업본부장</div>
-
-부록: 지점 목록
-
-서울 강남, 서울 종로, 경기 분당, 인천 송도.
```
