"""Two synthetic modern-style decks (CC0-1.0, written for hanji's pptx canvas
audit and kept in the corpus, see ../SOURCES.md) that mimic
designed templates: Blank-layout free canvas, full-bleed photos with overlays,
rounded cards with shadows, freeform icons, nested groups, rotated/flipped
shapes, gradients, cropped/masked pictures, connectors, a chart, a table,
transitions and an entrance animation. python-pptx 1.0.2.

Run: pip install python-pptx==1.0.2 pillow && python3 make_synth.py .
Regenerating gives the same content, not the same bytes (zip timestamps). The output must pass
validation/ooxml-schema/validate.py (the transitional schemas): the first version wrote an
empty animation list and negative chart axis ids, and PowerPoint repaired the pitch deck."""
import io, copy, random
from PIL import Image, ImageDraw, ImageFilter
from pptx import Presentation
from pptx.util import Pt, Emu
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE, MSO_CONNECTOR
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.chart.data import CategoryChartData
from pptx.enum.chart import XL_CHART_TYPE
from pptx.oxml.ns import qn
from lxml import etree

random.seed(7)
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'


def photo(w, h, hue):
    im = Image.new('RGB', (w, h))
    d = ImageDraw.Draw(im)
    for y in range(h):
        c = (int(hue[0] * (1 - y / h) + 20), int(hue[1] * (1 - y / h) + 30), int(hue[2] * (y / h) + 40))
        d.line([(0, y), (w, y)], fill=c)
    for _ in range(25):
        x, y, r = random.randint(0, w), random.randint(0, h), random.randint(10, 80)
        d.ellipse([x - r, y - r, x + r, y + r], fill=(random.randint(100, 255),) * 3)
    im = im.filter(ImageFilter.GaussianBlur(6))
    b = io.BytesIO()
    im.save(b, 'JPEG', quality=70)
    b.seek(0)
    return b


def textbox(s, x, y, w, h, text, size=18, color=(0x22, 0x22, 0x22), bold=False, font='Pretendard', align=None, anchor=None):
    tb = s.shapes.add_textbox(Pt(x), Pt(y), Pt(w), Pt(h))
    tf = tb.text_frame
    tf.word_wrap = True
    if anchor:
        tf.vertical_anchor = anchor
    for i, line in enumerate(text.split('\n')):
        p = tf.paragraphs[0] if i == 0 else tf.add_paragraph()
        r = p.add_run()
        r.text = line
        r.font.size = Pt(size)
        r.font.bold = bold
        r.font.name = font
        r.font.color.rgb = RGBColor(*color)
        if align:
            p.alignment = align
    return tb


def shape(s, kind, x, y, w, h, fill=None, line=None, text=None, size=14, tcolor=(255, 255, 255), theme=None):
    sh = s.shapes.add_shape(kind, Pt(x), Pt(y), Pt(w), Pt(h))
    if fill is None and theme is None:
        sh.fill.background()
    elif theme is not None:
        sh.fill.solid()
        sh.fill.fore_color.theme_color = theme
    else:
        sh.fill.solid()
        sh.fill.fore_color.rgb = RGBColor(*fill)
    if line is None:
        sh.line.fill.background()
    else:
        sh.line.color.rgb = RGBColor(*line)
        sh.line.width = Pt(1.5)
    if text:
        sh.text_frame.text = text
        for p in sh.text_frame.paragraphs:
            for r in p.runs:
                r.font.size = Pt(size)
                r.font.color.rgb = RGBColor(*tcolor)
    return sh


def shadow(sh):
    spPr = sh._element.spPr
    eff = etree.SubElement(spPr, qn('a:effectLst'))
    o = etree.SubElement(eff, qn('a:outerShdw'), blurRad='190500', dist='38100', dir='5400000', algn='t', rotWithShape='0')
    c = etree.SubElement(o, qn('a:srgbClr'), val='000000')
    etree.SubElement(c, qn('a:alpha'), val='25000')


def alpha(sh, a):
    sf = sh._element.spPr.find(qn('a:solidFill'))
    clr = sf[0]
    etree.SubElement(clr, qn('a:alpha'), val=str(a))


def gradient(sh, c1, c2, ang=5400000):
    spPr = sh._element.spPr
    for f in spPr.findall(qn('a:solidFill')) + spPr.findall(qn('a:noFill')):
        spPr.remove(f)
    g = etree.Element(qn('a:gradFill'), rotWithShape='1')
    gl = etree.SubElement(g, qn('a:gsLst'))
    for pos, c in ((0, c1), (100000, c2)):
        gs = etree.SubElement(gl, qn('a:gs'), pos=str(pos))
        etree.SubElement(gs, qn('a:srgbClr'), val=c)
    etree.SubElement(g, qn('a:lin'), ang=str(ang), scaled='0')
    geom = spPr.find(qn('a:prstGeom'))
    geom.addnext(g)


def icon(s, x, y, size, color):
    """A freeform (custGeom) icon, like the SVG icons designed decks paste in."""
    k = size / 10.0
    pts = [(5, 0), (6.2, 3.6), (10, 3.8), (7, 6.2), (8, 10), (5, 7.8), (2, 10), (3, 6.2), (0, 3.8), (3.8, 3.6)]
    fb = s.shapes.build_freeform(Pt(x + pts[0][0] * k), Pt(y + pts[0][1] * k), scale=1.0)
    fb.add_line_segments([(Pt(x + px * k), Pt(y + py * k)) for px, py in pts[1:]], close=True)
    sh = fb.convert_to_shape()
    sh.fill.solid()
    sh.fill.fore_color.rgb = RGBColor(*color)
    sh.line.fill.background()
    return sh


def transition(slide, kind='fade'):
    x = slide._element
    t = etree.SubElement(x, qn('p:transition'), spd='med')
    etree.SubElement(t, qn('p:' + kind))
    # p:transition must come after clrMapOvr and before timing
    x.remove(t)
    cm = x.find(qn('p:clrMapOvr'))
    (cm if cm is not None else x.find(qn('p:cSld'))).addnext(t)


def appear(slide, spids):
    """An entrance (fade) on click for each shape id. A timing tree needs at least one effect:
    an empty p:childTnLst is invalid, and PowerPoint repairs the file."""
    assert spids, 'no shapes to animate'
    pars = ''
    for n, sid in enumerate(spids):
        c = 10 + n * 10
        pars += f'''<p:par><p:cTn id="{c}" fill="hold"><p:stCondLst><p:cond delay="indefinite"/></p:stCondLst><p:childTnLst><p:par><p:cTn id="{c+1}" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:par><p:cTn id="{c+2}" presetID="10" presetClass="entr" presetSubtype="0" fill="hold" nodeType="clickEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:set><p:cBhvr><p:cTn id="{c+3}" dur="1" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid="{sid}"/></p:tgtEl><p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val="visible"/></p:to></p:set><p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="{c+4}" dur="500"/><p:tgtEl><p:spTgt spid="{sid}"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par>'''
    xml = f'''<p:timing xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst><p:seq concurrent="1" nextAc="seek"><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst>{pars}</p:childTnLst></p:cTn><p:prevCondLst><p:cond evt="onPrev" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst><p:nextCondLst><p:cond evt="onNext" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>'''
    slide._element.append(etree.fromstring(xml))


def chart(s, kind, x, y, w, h, cd):
    """python-pptx 1.0.2 writes negative axis ids (c:axId, c:crossAx), which the schema's
    xsd:unsignedInt rejects; they are made positive here."""
    gf = s.shapes.add_chart(kind, x, y, w, h, cd)
    for el in gf.chart._chartSpace.iter(qn('c:axId'), qn('c:crossAx')):
        el.set('val', str(abs(int(el.get('val')))))
    return gf


def mask(pic, prst='ellipse'):
    g = pic._element.spPr.find(qn('a:prstGeom'))
    g.set('prst', prst)


def rotate(sh, deg):
    sh.rotation = deg


def flip(sh, h=True):
    x = sh._element.spPr.find(qn('a:xfrm'))
    x.set('flipH' if h else 'flipV', '1')


def bg_gradient(slide, c1, c2):
    cSld = slide._element.find(qn('p:cSld'))
    bg = etree.fromstring(f'<p:bg xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="{A}"><p:bgPr><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="{c1}"/></a:gs><a:gs pos="100000"><a:srgbClr val="{c2}"/></a:gs></a:gsLst><a:lin ang="2700000"/></a:gradFill><a:effectLst/></p:bgPr></p:bg>')
    cSld.insert(0, bg)


# ------------------------------------------------------------------ deck 1: pitch (Blank-layout free canvas)
def pitch(path):
    prs = Presentation()
    prs.slide_width, prs.slide_height = Pt(960), Pt(540)
    blank = prs.slide_layouts[6]
    NAVY, TEAL, CORAL, INK = (0x10, 0x1B, 0x3A), (0x14, 0xB8, 0xA6), (0xFF, 0x6B, 0x5B), (0x22, 0x22, 0x22)

    s = prs.slides.add_slide(blank)  # 1 cover
    s.shapes.add_picture(photo(960, 540, (40, 90, 160)), 0, 0, Pt(960), Pt(540))
    ov = shape(s, MSO_SHAPE.RECTANGLE, 0, 0, 960, 540, fill=NAVY)
    alpha(ov, 55000)
    t = textbox(s, 64, 200, 700, 90, 'Northwind Mobility', 54, (255, 255, 255), True, 'Pretendard ExtraBold')
    textbox(s, 64, 290, 600, 40, 'Series A · 2026', 22, (0xCF, 0xE8, 0xFF))
    g1 = shape(s, MSO_SHAPE.OVAL, 64, 64, 36, 36, fill=TEAL)
    g2 = shape(s, MSO_SHAPE.RECTANGLE, 108, 72, 90, 20, fill=(255, 255, 255))
    s.shapes.add_group_shape([g1, g2]).name = 'Logo'
    transition(s)

    s = prs.slides.add_slide(blank)  # 2 agenda with numbered circles and connectors
    textbox(s, 64, 40, 600, 60, 'Agenda', 40, NAVY, True)
    prev = None
    for i, lab in enumerate(['Problem', 'Solution', 'Market', 'Traction']):
        x = 100 + i * 200
        c = shape(s, MSO_SHAPE.OVAL, x, 220, 72, 72, theme=5, text=str(i + 1), size=28)
        textbox(s, x - 30, 310, 132, 40, lab, 18, INK, align=PP_ALIGN.CENTER)
        if prev is not None:
            cn = s.shapes.add_connector(MSO_CONNECTOR.STRAIGHT, prev.left + prev.width, Pt(256), c.left, Pt(256))
            cn.begin_connect(prev, 3)
            cn.end_connect(c, 1)
        prev = c
    transition(s, 'push')

    s = prs.slides.add_slide(blank)  # 3 three cards: rounded rect + shadow + icon + text, grouped
    textbox(s, 64, 40, 800, 60, 'Why now', 40, NAVY, True)
    for i, (h, b) in enumerate([('Cities', 'Low-emission zones in 40 cities'), ('Costs', 'Battery cost down 60% in 5 years'), ('People', 'Two thirds want fewer cars')]):
        x = 64 + i * 290
        card = shape(s, MSO_SHAPE.ROUNDED_RECTANGLE, x, 130, 260, 320, fill=(255, 255, 255))
        card.adjustments[0] = 0.08
        shadow(card)
        ic = icon(s, x + 24, 154, 44, CORAL)
        hd = textbox(s, x + 24, 214, 212, 40, h, 24, NAVY, True)
        bd = textbox(s, x + 24, 260, 212, 120, b, 16, (0x55, 0x55, 0x55))
        s.shapes.add_group_shape([card, ic, hd, bd]).name = 'Card %d' % (i + 1)

    s = prs.slides.add_slide(blank)  # 4 big numbers, chevrons (rotated), gradient band
    band = shape(s, MSO_SHAPE.RECTANGLE, 0, 0, 960, 180, fill=TEAL)
    gradient(band, '14B8A6', '0EA5E9', 0)
    textbox(s, 64, 60, 800, 60, 'Traction', 40, (255, 255, 255), True)
    for i, (n, l) in enumerate([('12k', 'riders / day'), ('3.4x', 'YoY growth'), ('92%', 'retention')]):
        x = 64 + i * 300
        textbox(s, x, 220, 260, 100, n, 72, CORAL, True, align=PP_ALIGN.LEFT)
        textbox(s, x, 320, 260, 40, l, 18, INK)
        ch = shape(s, MSO_SHAPE.CHEVRON, x + 200, 420, 40, 40, fill=NAVY)
        rotate(ch, -90)
    ids = [sh.shape_id for sh in s.shapes][3:6]
    appear(s, ids)

    s = prs.slides.add_slide(blank)  # 5 image grid: crop, ellipse mask, rounded mask, captions
    textbox(s, 64, 40, 800, 60, 'In the field', 40, NAVY, True)
    p1 = s.shapes.add_picture(photo(640, 480, (200, 120, 60)), Pt(64), Pt(120), Pt(400), Pt(300))
    p1.crop_left, p1.crop_right = 0.1, 0.05
    p2 = s.shapes.add_picture(photo(400, 400, (60, 160, 90)), Pt(500), Pt(120), Pt(180), Pt(180))
    mask(p2, 'ellipse')
    p3 = s.shapes.add_picture(photo(400, 300, (120, 60, 180)), Pt(710), Pt(120), Pt(200), Pt(150))
    mask(p3, 'roundRect')
    shadow(p3)
    textbox(s, 64, 430, 400, 30, 'Depot, Incheon', 14, (0x77, 0x77, 0x77))
    textbox(s, 500, 310, 180, 30, 'Driver onboarding', 14, (0x77, 0x77, 0x77), align=PP_ALIGN.CENTER)

    s = prs.slides.add_slide(blank)  # 6 timeline
    textbox(s, 64, 40, 800, 60, 'Roadmap', 40, NAVY, True)
    ln = s.shapes.add_connector(MSO_CONNECTOR.STRAIGHT, Pt(80), Pt(280), Pt(880), Pt(280))
    ln.line.color.rgb = RGBColor(*NAVY)
    ln.line.width = Pt(3)
    ln.line._get_or_add_ln().append(etree.fromstring(f'<a:tailEnd xmlns:a="{A}" type="triangle"/>'))
    for i, (q, lab) in enumerate([('Q1', 'Pilot'), ('Q2', 'Seoul'), ('Q3', 'Busan'), ('Q4', 'Tokyo'), ('2027', 'IPO?')]):
        x = 110 + i * 170
        shape(s, MSO_SHAPE.DIAMOND, x, 265, 30, 30, fill=CORAL if i == 4 else TEAL)
        textbox(s, x - 35, 220, 100, 30, q, 16, NAVY, True, align=PP_ALIGN.CENTER)
        textbox(s, x - 35, 305, 100, 30, lab, 14, INK, align=PP_ALIGN.CENTER)
    ar = shape(s, MSO_SHAPE.CURVED_RIGHT_ARROW, 820, 360, 60, 90, fill=CORAL)
    flip(ar)

    s = prs.slides.add_slide(blank)  # 7 chart + table
    textbox(s, 64, 40, 800, 60, 'Unit economics', 40, NAVY, True)
    cd = CategoryChartData()
    cd.categories = ['Q1', 'Q2', 'Q3', 'Q4']
    cd.add_series('Revenue', (1.2, 2.1, 3.3, 4.8))
    cd.add_series('Cost', (1.8, 2.0, 2.4, 2.9))
    chart(s, XL_CHART_TYPE.COLUMN_CLUSTERED, Pt(64), Pt(120), Pt(480), Pt(340), cd)
    tb = s.shapes.add_table(4, 2, Pt(580), Pt(140), Pt(320), Pt(160)).table
    for r, (a, b) in enumerate([('Metric', 'Value'), ('CAC', '$18'), ('LTV', '$240'), ('Payback', '4 mo')]):
        tb.cell(r, 0).text, tb.cell(r, 1).text = a, b

    s = prs.slides.add_slide(blank)  # 8 quote on gradient background, rotated quote mark
    bg_gradient(s, '101B3A', '1E3A8A')
    q = textbox(s, 40, 40, 160, 160, '“', 200, (0x14, 0xB8, 0xA6), True, 'Georgia')
    rotate(q, 8)
    textbox(s, 140, 170, 700, 160, 'We moved 30% of our staff out of cars in six months.', 36, (255, 255, 255), False, 'Pretendard Light')
    textbox(s, 140, 350, 600, 30, '— Head of Operations, a pilot customer', 16, (0xCF, 0xE8, 0xFF))
    transition(s)
    prs.save(path)


# ------------------------------------------------------------------ deck 2: Korean report / infographic
def report(path):
    prs = Presentation()
    prs.slide_width, prs.slide_height = Pt(960), Pt(540)
    title_only, blank, two = prs.slide_layouts[5], prs.slide_layouts[6], prs.slide_layouts[3]
    BLUE, GRAY, INK = (0x25, 0x63, 0xEB), (0xE5, 0xE7, 0xEB), (0x11, 0x18, 0x27)

    s = prs.slides.add_slide(blank)  # 1 cover with side band, big number, logo group (nested)
    band = shape(s, MSO_SHAPE.RECTANGLE, 0, 0, 320, 540, fill=BLUE)
    gradient(band, '2563EB', '1E40AF')
    textbox(s, 360, 180, 560, 80, '2026 하반기 사업 보고', 44, INK, True, '맑은 고딕')
    textbox(s, 360, 270, 560, 40, '전략기획실 · 2026년 9월', 20, (0x6B, 0x72, 0x80), font='맑은 고딕')
    a = shape(s, MSO_SHAPE.OVAL, 40, 40, 30, 30, fill=(255, 255, 255))
    b = shape(s, MSO_SHAPE.OVAL, 60, 40, 30, 30, fill=(0x93, 0xC5, 0xFD))
    inner = s.shapes.add_group_shape([a, b])
    c = textbox(s, 100, 42, 160, 30, '한빛모빌리티', 16, (255, 255, 255), True, '맑은 고딕')
    s.shapes.add_group_shape([inner, c]).name = '로고'

    s = prs.slides.add_slide(title_only)  # 2 title slot + KPI cards (roundRect w/ theme fill + text inside shape)
    s.shapes.title.text = '핵심 지표'
    for i, (n, l) in enumerate([('1,240억', '매출'), ('+12%', '전년 대비'), ('34곳', '신규 고객'), ('4.7', '고객 만족도')]):
        x = 40 + i * 225
        card = shape(s, MSO_SHAPE.ROUNDED_RECTANGLE, x, 150, 205, 150, fill=GRAY)
        tf = card.text_frame
        tf.text = n
        tf.paragraphs[0].runs[0].font.size = Pt(36)
        tf.paragraphs[0].runs[0].font.bold = True
        tf.paragraphs[0].runs[0].font.color.rgb = RGBColor(*BLUE)
        p = tf.add_paragraph()
        r = p.add_run()
        r.text = l
        r.font.size = Pt(16)
        r.font.color.rgb = RGBColor(*INK)
    textbox(s, 40, 330, 880, 60, '출처: 내부 집계 (2026년 9월 말 기준)', 12, (0x6B, 0x72, 0x80))

    s = prs.slides.add_slide(title_only)  # 3 process: chevrons with text, connectors, callout
    s.shapes.title.text = '추진 절차'
    for i, lab in enumerate(['기획', '설계', '구축', '검증', '확산']):
        shape(s, MSO_SHAPE.CHEVRON if i else MSO_SHAPE.PENTAGON, 40 + i * 180, 200, 190, 80, theme=5 if i % 2 else 6, text=lab, size=20)
    cl = shape(s, MSO_SHAPE.RECTANGULAR_CALLOUT, 560, 330, 240, 80, fill=(0xFE, 0xF3, 0xC7), line=(0xF5, 0x9E, 0x0B), text='10월 착수 예정', size=16, tcolor=INK)
    appear(s, [cl.shape_id])

    s = prs.slides.add_slide(two)  # 4 comparison layout with slots plus decorations
    s.shapes.title.text = '지역별 현황'
    s.placeholders[1].text = '수도권 21곳\n신규 9곳'
    s.placeholders[2].text = '지방 13곳\n신규 4곳'
    vs = shape(s, MSO_SHAPE.OVAL, 455, 250, 50, 50, fill=BLUE, text='VS', size=14)
    shadow(vs)

    s = prs.slides.add_slide(blank)  # 5 org-chart-like diagram built from shapes (not SmartArt) + elbow connectors
    textbox(s, 40, 30, 600, 50, '조직 구성', 32, INK, True, '맑은 고딕')
    top = shape(s, MSO_SHAPE.ROUNDED_RECTANGLE, 400, 100, 160, 50, fill=BLUE, text='대표이사', size=16)
    for i, lab in enumerate(['전략기획실', '사업본부', '기술연구소']):
        ch = shape(s, MSO_SHAPE.ROUNDED_RECTANGLE, 120 + i * 280, 230, 160, 50, fill=(255, 255, 255), line=BLUE, text=lab, size=16, tcolor=INK)
        cn = s.shapes.add_connector(MSO_CONNECTOR.ELBOW, 0, 0, 0, 0)
        cn.begin_connect(top, 2)
        cn.end_connect(ch, 0)
    ph = s.shapes.add_picture(photo(300, 300, (90, 90, 200)), Pt(820), Pt(400), Pt(90), Pt(90))
    mask(ph, 'ellipse')

    s = prs.slides.add_slide(title_only)  # 6 chart (doughnut) + legend made of shapes
    s.shapes.title.text = '매출 구성'
    cd = CategoryChartData()
    cd.categories = ['구독', '라이선스', '서비스']
    cd.add_series('비중', (0.55, 0.3, 0.15))
    chart(s, XL_CHART_TYPE.DOUGHNUT, Pt(60), Pt(130), Pt(360), Pt(360), cd)
    for i, (lab, col) in enumerate([('구독 55%', BLUE), ('라이선스 30%', (0xF5, 0x9E, 0x0B)), ('서비스 15%', (0x10, 0xB9, 0x81))]):
        shape(s, MSO_SHAPE.RECTANGLE, 500, 200 + i * 50, 20, 20, fill=col)
        textbox(s, 530, 195 + i * 50, 300, 30, lab, 18, INK, font='맑은 고딕')
    transition(s, 'wipe')
    prs.save(path)


if __name__ == '__main__':
    import sys
    pitch(sys.argv[1] + '/synth-modern-pitch.pptx')
    report(sys.argv[1] + '/synth-korean-report.pptx')
