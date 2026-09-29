"""Writes korean-deck.pptx, the corpus's synthetic Korean deck (CC0-1.0), with
python-pptx 1.0.2 from its default template: the DESIGN.md §5.3 example
grown to seven slides (title, bullets with nesting and bold, two content,
a text box, a table, a picture, a numbered list, a hyperlink and a coloured
run, notes). Run: pip install python-pptx==1.0.2 && python3 make_korean.py.
Regenerating gives the same content, not the same bytes (zip timestamps)."""
import datetime, io
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.oxml.ns import qn
from PIL import Image

prs = Presentation()
L = {l.name: l for l in prs.slide_layouts}

def ko(run):
    rpr = run._r.get_or_add_rPr()
    rpr.set("lang", "ko-KR")
    rpr.set("altLang", "en-US")

def para(tf, text, level=0, first=False, bold=None):
    p = tf.paragraphs[0] if first else tf.add_paragraph()
    p.level = level
    parts = [(text, False)] if bold is None else [(text[: text.index(bold)], False), (bold, True), (text[text.index(bold) + len(bold):], False)]
    for t, b in parts:
        if not t:
            continue
        r = p.add_run()
        r.text = t
        r.font.bold = True if b else None
        ko(r)
    return p

def notes(slide, text):
    tf = slide.notes_slide.notes_text_frame
    tf.text = text
    for r in tf.paragraphs[0].runs:
        ko(r)

s = prs.slides.add_slide(L["Title Slide"])
s.shapes.title.text = "3분기 영업 보고"
s.placeholders[1].text = "영업본부 · 2026년 10월"
for sh in s.placeholders:
    for r in sh.text_frame.paragraphs[0].runs:
        ko(r)
notes(s, "인사말 후 목차를 소개한다.")

s = prs.slides.add_slide(L["Title and Content"])
s.shapes.title.text = "핵심 지표"
ko(s.shapes.title.text_frame.paragraphs[0].runs[0])
tf = s.placeholders[1].text_frame
para(tf, "매출 12% 증가", first=True, bold="12% 증가")
para(tf, "신규 고객 34곳")
para(tf, "수도권 21곳", level=1)
para(tf, "지방 13곳", level=1)
para(tf, "영업이익률 8.4%")
notes(s, "전년 대비 증가폭을 강조한다.")

s = prs.slides.add_slide(L["Two Content"])
s.shapes.title.text = "지역별 현황"
ko(s.shapes.title.text_frame.paragraphs[0].runs[0])
para(s.placeholders[1].text_frame, "수도권 21곳", first=True)
para(s.placeholders[1].text_frame, "서울 14곳", level=1)
para(s.placeholders[2].text_frame, "지방 13곳", first=True)
para(s.placeholders[2].text_frame, "부산 5곳", level=1)
tb = s.shapes.add_textbox(Inches(0.5), Inches(6.6), Inches(4), Inches(0.4))
tb.name = "출처"
para(tb.text_frame, "출처: 내부 집계", first=True)

s = prs.slides.add_slide(L["Title Only"])
s.shapes.title.text = "분기별 매출"
ko(s.shapes.title.text_frame.paragraphs[0].runs[0])
rows = [("분기", "매출", "증감"), ("1분기", "1,120", "+3%"), ("2분기", "1,180", "+5%"), ("3분기", "1,204", "+12%")]
tbl = s.shapes.add_table(4, 3, Inches(1), Inches(2), Inches(8), Inches(2)).table
for i, row in enumerate(rows):
    for j, v in enumerate(row):
        tbl.cell(i, j).text = v
buf = io.BytesIO()
Image.new("RGB", (64, 32), (40, 90, 160)).save(buf, "PNG")
buf.seek(0)
s.shapes.add_picture(buf, Inches(8), Inches(5.5), Inches(1), Inches(0.5))
notes(s, "표는 잠정치이며 10월 말 확정된다.")

s = prs.slides.add_slide(L["Title and Content"])
s.shapes.title.text = "다음 분기 계획"
ko(s.shapes.title.text_frame.paragraphs[0].runs[0])
tf = s.placeholders[1].text_frame
for k, t in enumerate(["신규 지점 3곳 개설", "온라인 채널 확대", "고객 만족도 조사"]):
    p = para(tf, t, first=(k == 0))
    ppr = p._p.get_or_add_pPr()
    num = ppr.makeelement(qn("a:buAutoNum"), {"type": "arabicPeriod"})
    ppr.append(num)
p = tf.add_paragraph()
r = p.add_run()
r.text = "자세한 일정은 "
ko(r)
r = p.add_run()
r.text = "사내 게시판"
r.hyperlink.address = "https://www.korea.kr/"
ko(r)
r = p.add_run()
r.text = " 참고 (긴급)"
r.font.color.rgb = RGBColor(0xC0, 0x00, 0x00)
r.font.size = Pt(20)
ko(r)

s = prs.slides.add_slide(L["Section Header"])
s.shapes.title.text = "부록"
ko(s.shapes.title.text_frame.paragraphs[0].runs[0])
s.placeholders[1].text = "세부 자료"
ko(s.placeholders[1].text_frame.paragraphs[0].runs[0])

s = prs.slides.add_slide(L["Title and Content"])
s.shapes.title.text = "감사합니다"
ko(s.shapes.title.text_frame.paragraphs[0].runs[0])

cp = prs.core_properties
cp.author = "hanji"
cp.last_modified_by = "hanji"
cp.title = "3분기 영업 보고"
cp.created = cp.modified = datetime.datetime(2026, 9, 29, 0, 0, 0)
prs.save("korean-deck.pptx")
