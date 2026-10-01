# What 73-hwpx-hy-002-e10.hwpx changed

Every scripted edit (E1–E9, the formatting edits F1–F4, a banner's fill and a first-line indent on the body paragraphs) in one revision. Source: `hy-002.hwpx`; compare with `71-hwpx-hy-002-original.hwpx`.

## The edits

- E2 insert before drawing (new paragraph before block 0 (a table))
- E3 delete formatted paragraph (delete block 11 ("해양수산부(장관 황종우)는 「지속가능한 연근해어업 발전"…))
- E4 move section (no same-level headings: move blocks 16–29 before block 0 ("Inserted paragraph before the "…))
- E5 restyle (block 0 ("「해운법 일부개정법률안」은 국가가 운항결손액 전액을 보"…) → style "본문")
- E6 table cell (no merged cell in any modelled table: append ' (rev.)' to cell [0, 0, 0])
- E7 edit across a run boundary (replace "행 국가" (straddles two differently formatted runs) with 'EDITED')
- E8 split paragraph (split block 26 ("「지속가능한 연근해어업 발전법안」은 데이터에 기반하여 "…) at offset 179 of 356)
- E9 merge paragraphs (join block 26 ("「지속가능한 연근해어업 발전법안」은 데이터에 기반하여 "…) and block 27 ("어구·어법 제한, 금어기·금지체장 등 1,500여 건의"…))
- F1 first-line indent (first-line=10pt on body paragraphs block 2 ("황종우 해양수산부 장관은 “이번 본회의를 통과한 3개 "…))
- F3 new style (new style "Callout" (fill=#FFF2CC border-left="2.25pt solid #C00000") given to block 14 ("Inserted paragraph before the "…))
- F4 cell fill (fill=#DDEBF7 on the header row of the table at block 13 (cells "담당 부서 (rev.)" to the last))

## Model text, before and after (unified diff; `-` original, `+` edited)

```diff
--- original (model text)
+++ edited (model text)
@@ -4,6 +4,38 @@
 schema: 1
 ---
 <style name="바탕글" align=justify line-spacing=160% font=함초롬바탕 size=10pt color=#000000/>
+<style name="본문" indent-left=15pt/>
+<style name="Callout" fill=#FFF2CC border-left="1.98pt solid #C00000" indent-left=10pt/>
+
+<div style="본문">  「해운법 일부개정법률안」은 국가가 운항결손액 전액을 보전하는 현EDITED보조항로의 명칭을 ‘공영항로’로 변경하고, 공영항로 운영을 공공기관에 위탁할 수 있도록 제도를 개편하였다. 그간 정부는 수익성이 없어 민간이 운영을 포기하는 항로에 대해 국비로 여객선을 건조하여 민간 선사에 위탁·운영하고, 운영비와 손실을 국가가 보전하는 국가보조항로 제도를 추진해왔다. 하지만, 국가 보조에 따른 도덕적 해이, 선박 관리 미흡 등으로 정부재정 부담은 급증하는 반면 운항 안전성과 서비스 질 저하에 대한 우려가 계속 제기되었다. 이번 법 개정으로 선박검사, 운항관리 등 전문성을 갖춘 공공기관이 공영항로를 운영할 수 있게 되어 항로 운영의 공공성과 안전성을 강화하고, 섬 주민들의 해상교통 기본권을 더욱 탄탄히 보장할 수 있을 것으로 기대된다. 해양수산부는 안정적인 제도 정착을 위해 법이 시행되는 2027년에 일부 공영항로를 공공기관에 위탁하고, 2028년부터는 전체 공영항로를 공공기관에 위탁할 계획이다. </div> {line-spacing=140% font=휴먼명조 size=14pt}
+
+<p/>
+
+  황종우 해양수산부 장관은 “이번 본회의를 통과한 3개 법안은 어업분야의 낡은 규제 혁파, 북극항로라는 새로운 국가 성장동력 창출, 섬 주민들의 해상교통권 강화 등을 추진할 법적 기반을 마련하였다는 측면에서 의미가 크다.”라며, “특히 연근해어업 데이터를 바탕으로 기존의 낡은 투입 규제들을 과감히 폐지·조정해 나가는 등, 본회의를 통과한 법률안들의 하위법령 정비 및 차질없는 법령 시행을 위해 최선을 다하겠다.”라고 말했다.  {first-line=10pt line-spacing=140% font=휴먼명조 size=14pt}
+
+<p/>
+<p/>
+<p/>
+<p/>
+<p/>
+<p/>
+<p/>
+<p/>
+<p/>
+<p/>
+
+{border="0.34pt solid #000000" valign=middle font=돋움체}
+| {border-bottom=none valign=bottom} 담당 부서 (rev.) {align=center} | {border-bottom=none valign=bottom} 정책기획관 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 서은정 {align=right} | {border-left=none} (051-773-5160) {align=left} | {fill=#DDEBF7}
+|---|---|---|---|---|---|
+| {border-top=none valign=top} | {border-top=none border-bottom=none valign=top} 규제개혁법무담당관실 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 양동곤 {align=right} | {border-left=none} (051-773-5163) {align=left} |
+| {border-bottom=none valign=top} <연근해어업 발전법> {align=center} | {border-bottom=none valign=bottom} 어업자원정책관 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 서진희 {align=right} | {border-left=none} (051-773-5510) {align=left} |
+| {border-top=none valign=top} | {border-top=none border-bottom=none valign=top} 어업정책과 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 이석진 {align=right} | {border-left=none} (051-773-5511) {align=left} |
+| {border-bottom=none valign=top} <북극항로 특별법> {align=center} | {border-bottom=none valign=bottom} 북극항로추진본부 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 고송주 {align=right} | {border-left=none} (051-773-6310) {align=left} |
+| {border-top=none valign=top} | {border-top=none border-bottom=none valign=top} 기획지원과 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 차석근 {align=right} | {border-left=none} (051-773-6315) {align=left} |
+| {border-bottom=none valign=top} <해운법> {align=center} | {border-bottom=none valign=bottom} 해운물류국 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 심상철 {align=right} | {border-left=none} (051-773-5730) {align=left} |
+| {border-top=none valign=top} | {border-top=none valign=top} 연안해운과 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 김근령 {align=right} | {border-left=none} (051-773-5737) {align=left} |
+
+<div style="Callout">Inserted paragraph before the drawing.</div>
 
 {border-top="0.34pt solid #000000" border-bottom="0.34pt solid #000000" valign=middle align=center}
 | {border-left="0.34pt solid #000000"} <keep id="kjg34" kind="drawing" summary="picture"/>광 {font=돋움체 color=#FFFFFF} | 보도자료 {size=14pt} | {border-right="0.34pt solid #000000"} |
@@ -27,9 +59,6 @@
 
 <p/>
 <p/>
-
-  [해양수산부(장관 황종우)는 「지속가능한 연근해어업 발전법안」,「북극항로 활용 촉진 및 연관산업 육성에 관한 특별법안」등 2개 제정법률안과 「해운법 일부개정법률안」이 5월 7일]{size=14pt}[(목)]{size=12pt} [국회 본회의를 통과했다고 밝혔다.]{size=14pt} {line-spacing=140% font=휴먼명조}
-
 <p/>
 
   「지속가능한 연근해어업 발전법안」은 데이터에 기반하여 연근해 어업 행정 체계를 재정비하기 위해 조업 위치, 어종별 어획·양륙 실적 보고를 의무화하고 어획확인서･증명서 발급 근거를 명시하는 등 연근해어업 데이터를 확보할 수 있는 기틀을 마련하였다. 그간 우리 연근해어업은 118년 전에 제정된 「어업법」에 뿌리를 두고 어구·어법 제한, 금어기·금지체장 등 1,500여 건의 투입규제를 중심으로 관리되어 왔다. 이번 제정안이 시행되면 과학적인 어획 데이터를 바탕으로 어업 관리체계를 산출량 중심으로 전환하고, 기존 투입규제는 과감히 폐지·조정함으로써 어업인 부담을 완화하고 국내 수산업 경쟁력을 강화하는 데 크게 기여할 것으로 기대된다. {line-spacing=140% font=휴먼명조 size=14pt}
@@ -40,32 +69,4 @@
 
 <p/>
 
-  「해운법 일부개정법률안」은 국가가 운항결손액 전액을 보전하는 현행 국가보조항로의 명칭을 ‘공영항로’로 변경하고, 공영항로 운영을 공공기관에 위탁할 수 있도록 제도를 개편하였다. 그간 정부는 수익성이 없어 민간이 운영을 포기하는 항로에 대해 국비로 여객선을 건조하여 민간 선사에 위탁·운영하고, 운영비와 손실을 국가가 보전하는 국가보조항로 제도를 추진해왔다. 하지만, 국가 보조에 따른 도덕적 해이, 선박 관리 미흡 등으로 정부재정 부담은 급증하는 반면 운항 안전성과 서비스 질 저하에 대한 우려가 계속 제기되었다. 이번 법 개정으로 선박검사, 운항관리 등 전문성을 갖춘 공공기관이 공영항로를 운영할 수 있게 되어 항로 운영의 공공성과 안전성을 강화하고, 섬 주민들의 해상교통 기본권을 더욱 탄탄히 보장할 수 있을 것으로 기대된다. 해양수산부는 안정적인 제도 정착을 위해 법이 시행되는 2027년에 일부 공영항로를 공공기관에 위탁하고, 2028년부터는 전체 공영항로를 공공기관에 위탁할 계획이다.  {line-spacing=140% font=휴먼명조 size=14pt}
-
-<p/>
-
-  황종우 해양수산부 장관은 “이번 본회의를 통과한 3개 법안은 어업분야의 낡은 규제 혁파, 북극항로라는 새로운 국가 성장동력 창출, 섬 주민들의 해상교통권 강화 등을 추진할 법적 기반을 마련하였다는 측면에서 의미가 크다.”라며, “특히 연근해어업 데이터를 바탕으로 기존의 낡은 투입 규제들을 과감히 폐지·조정해 나가는 등, 본회의를 통과한 법률안들의 하위법령 정비 및 차질없는 법령 시행을 위해 최선을 다하겠다.”라고 말했다.  {line-spacing=140% font=휴먼명조 size=14pt}
-
-<p/>
-<p/>
-<p/>
-<p/>
-<p/>
-<p/>
-<p/>
-<p/>
-<p/>
-<p/>
-
-{border="0.34pt solid #000000" valign=middle font=돋움체}
-| {border-bottom=none valign=bottom} 담당 부서 {align=center} | {border-bottom=none valign=bottom} 정책기획관 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 서은정 {align=right} | {border-left=none} (051-773-5160) {align=left} |
-|---|---|---|---|---|---|
-| {border-top=none valign=top} | {border-top=none border-bottom=none valign=top} 규제개혁법무담당관실 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 양동곤 {align=right} | {border-left=none} (051-773-5163) {align=left} |
-| {border-bottom=none valign=top} <연근해어업 발전법> {align=center} | {border-bottom=none valign=bottom} 어업자원정책관 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 서진희 {align=right} | {border-left=none} (051-773-5510) {align=left} |
-| {border-top=none valign=top} | {border-top=none border-bottom=none valign=top} 어업정책과 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 이석진 {align=right} | {border-left=none} (051-773-5511) {align=left} |
-| {border-bottom=none valign=top} <북극항로 특별법> {align=center} | {border-bottom=none valign=bottom} 북극항로추진본부 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 고송주 {align=right} | {border-left=none} (051-773-6310) {align=left} |
-| {border-top=none valign=top} | {border-top=none border-bottom=none valign=top} 기획지원과 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 차석근 {align=right} | {border-left=none} (051-773-6315) {align=left} |
-| {border-bottom=none valign=top} <해운법> {align=center} | {border-bottom=none valign=bottom} 해운물류국 {align=center} | 책임자 {align=center} | {border-right=none} 과  장  {align=right} | {border-right=none border-left=none} 심상철 {align=right} | {border-left=none} (051-773-5730) {align=left} |
-| {border-top=none valign=top} | {border-top=none valign=top} 연안해운과 {align=center} | 담당자 {align=center} | {border-right=none} 사무관 {align=right} | {border-right=none border-left=none} 김근령 {align=right} | {border-left=none} (051-773-5737) {align=left} |
-
 <keep id="kv5eo" kind="drawing" summary="rect: 그림입니다. 원본 그림의 이름: CLP0000d5d00002.bmp 원본…"/>
```
