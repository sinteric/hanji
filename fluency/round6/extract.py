"""Reads the formatting of real files into the round 6 vocabulary (vocab.py).

extract_hwpx(path) / extract_docx(path) -> Doc:
  styles    {name: {'para': props, 'char': props}}  every vocabulary property, resolved (effective)
  default   the default paragraph style's name (바탕글 / Normal)
  palette   theme colours {text1: #RRGGBB, ...} (docx; hwpx stores colours as RGB)
  items     document order, outside tables: ('p', Para) and ('t', Table)
Para: style, para (effective props), runs [(text, effective char props)], text
Table: cells {(row, col): Cell}; Cell: box (effective box props), paras [Para]
Nested tables, text boxes, footnotes, headers and footers are skipped: the text shows them as placeholders.

extract_pptx(path) / extract_xlsx(path) are in canvas.py and grid.py.

Units: hwpx HWPUNIT (1 pt = 100; paragraph margins from the HwpUnitChar branch of hp:switch, which the line
segments confirm: the default branch holds twice the value), border widths in mm; docx twips (1 pt = 20),
half-points, eighths of a point. Standard library only."""
import re
import xml.etree.ElementTree as ET
import zipfile

import vocab as V


def local(tag):
    return tag.rsplit('}', 1)[-1]


def kids(e, name):
    return [c for c in e if local(c.tag) == name]


def kid(e, name):
    for c in e:
        if local(c.tag) == name:
            return c
    return None


def find(e, name):
    for c in e.iter():
        if local(c.tag) == name:
            return c
    return None


class Para:
    def __init__(self, style, para, runs):
        self.style, self.para, self.runs = style, para, runs
        self.text = ''.join(t for t, _ in runs)

    def __repr__(self):
        return 'Para(%s, %r)' % (self.style, self.text[:30])


class Cell:
    def __init__(self, box, paras, span=(1, 1)):
        self.box, self.paras, self.span = box, paras, span


class Table:
    def __init__(self, cells, box=None, style=None):
        self.cells, self.box, self.style = cells, box or {}, style


class Doc:
    def __init__(self, fmt, styles, default, palette, items):
        self.fmt, self.styles, self.default, self.palette, self.items = fmt, styles, default, palette, items


NO_BOX = {'fill': 'none', 'border-top': 'none', 'border-right': 'none', 'border-bottom': 'none',
          'border-left': 'none', 'valign': 'top'}

# ================================================================ hwpx

HWPX_ALIGN = {'JUSTIFY': 'justify', 'LEFT': 'left', 'RIGHT': 'right', 'CENTER': 'center', 'DISTRIBUTE': 'distribute',
              'DISTRIBUTE_SPACE': 'distribute'}
HWPX_LINE = {'SOLID': 'solid', 'DASH': 'dashed', 'DOT': 'dotted', 'DASH_DOT': 'dash-dot',
             'DASH_DOT_DOT': 'dash-dot-dot', 'DOUBLE_SLIM': 'double', 'LONG_DASH': 'dashed', 'CIRCLE': 'dotted',
             'SLIM_THICK': 'thin-thick', 'THICK_SLIM': 'thick-thin', 'SLIM_THICK_SLIM': 'triple', 'WAVE': 'wave',
             'DOUBLE_WAVE': 'wave', 'THICK_3D': '3d', 'THICK_3D_REVERS': '3d', '3D': '3d', '3D_REVERS': '3d'}
HWPX_VALIGN = {'TOP': 'top', 'CENTER': 'middle', 'BOTTOM': 'bottom'}
MM = 72 / 25.4


def hwpx_color(c):
    if not c or c.lower() == 'none':
        return None
    return '#' + c.lstrip('#').upper()[-6:]


def hwpx_border(e):
    if e is None or e.get('type', 'NONE') == 'NONE':
        return 'none'
    w = float(e.get('width', '0.1 mm').split()[0]) * MM
    col = hwpx_color(e.get('color')) or '#000000'
    return '%s %s %s' % (V.pt(w), HWPX_LINE.get(e.get('type'), 'solid'), col)


def hwpx_borderfill(bf):
    p = {}
    for side, tag in (('border-top', 'topBorder'), ('border-right', 'rightBorder'), ('border-bottom', 'bottomBorder'),
                      ('border-left', 'leftBorder')):
        p[side] = hwpx_border(kid(bf, tag)) if bf is not None else 'none'
    p['fill'] = 'none'
    fb = find(bf, 'fillBrush') if bf is not None else None
    if fb is not None:
        wb = kid(fb, 'winBrush')
        if kid(fb, 'gradation') is not None:
            p['fill'] = 'gradient'
        elif kid(fb, 'imgBrush') is not None:
            p['fill'] = 'picture'
        elif wb is not None:
            if wb.get('hatchStyle') not in (None, 'NONE', '-1'):
                p['fill'] = 'pattern'
            else:
                p['fill'] = hwpx_color(wb.get('faceColor')) or 'none'
    return p


def hwpx_margin(pp):
    sw = find(pp, 'switch')
    src = None
    if sw is not None:
        case = kid(sw, 'case')
        src = find(case, 'margin') if case is not None else None
    if src is None:
        src = find(pp, 'margin')
    vals = {}
    for k in ('intent', 'left', 'right', 'prev', 'next'):
        e = kid(src, k) if src is not None else None
        vals[k] = int(e.get('value', '0')) / 100 if e is not None else 0.0
    ls = None
    if sw is not None and kid(sw, 'case') is not None:
        ls = find(kid(sw, 'case'), 'lineSpacing')
    if ls is None:
        ls = find(pp, 'lineSpacing')
    return vals, ls


def hwpx_line_spacing(ls):
    if ls is None:
        return '160%'
    t, v = ls.get('type', 'PERCENT'), int(ls.get('value', '160'))
    if t == 'PERCENT':
        return V.num(v) + '%'
    if t == 'FIXED':
        return V.pt(v / 100)
    if t == 'AT_LEAST':
        return 'at-least ' + V.pt(v / 100)
    return 'gap ' + V.pt(v / 100)


def hwpx_parapr(pp, bfs):
    m, ls = hwpx_margin(pp)
    al = find(pp, 'align')
    p = {'align': HWPX_ALIGN.get(al.get('horizontal') if al is not None else 'JUSTIFY', 'justify'),
         'indent-left': V.pt(m['left']), 'indent-right': V.pt(m['right']), 'first-line': V.pt(m['intent']),
         'space-before': V.pt(m['prev']), 'space-after': V.pt(m['next']), 'line-spacing': hwpx_line_spacing(ls)}
    b = find(pp, 'border')
    box = hwpx_borderfill(bfs.get(b.get('borderFillIDRef'))) if b is not None else dict(NO_BOX)
    for k in ('fill',) + V.SIDES:
        p[k] = box.get(k, 'none')
    return p


def hwpx_charpr(cp, fonts):
    fr = kid(cp, 'fontRef')
    face = fonts.get(fr.get('hangul', '0')) if fr is not None else None
    ul = kid(cp, 'underline')
    so = kid(cp, 'strikeout')
    return {'font': face or '', 'size': V.pt(int(cp.get('height', '1000')) / 100),
            'color': hwpx_color(cp.get('textColor')) or '#000000',
            'bold': 'yes' if kid(cp, 'bold') is not None else 'no',
            'italic': 'yes' if kid(cp, 'italic') is not None else 'no',
            'underline': 'yes' if ul is not None and ul.get('type', 'NONE') != 'NONE' else 'no',
            'strike': 'yes' if so is not None and so.get('shape', 'NONE') not in ('NONE', '', '3D') else 'no'}


def hwpx_text(t):
    out = [t.text or '']
    for c in t:
        n = local(c.tag)
        if n == 'tab':
            out.append('\t')
        elif n == 'lineBreak':
            out.append('\n')
        out.append(c.tail or '')
    return ''.join(out)


SKIP_IN_RUN = {'ctrl', 'pic', 'rect', 'ellipse', 'line', 'container', 'polygon', 'curve', 'arc', 'equation', 'ole',
               'textart', 'connectLine', 'video', 'chart', 'secPr', 'compose', 'dutmal', 'btn', 'radioBtn',
               'checkBtn', 'comboBox', 'edit', 'listBox', 'scrollBar'}


class HwpxHeader:
    def __init__(self, xml):
        root = ET.fromstring(xml)
        self.fonts = {}
        for ff in root.iter():
            if local(ff.tag) == 'fontface' and ff.get('lang') == 'HANGUL':
                for f in kids(ff, 'font'):
                    self.fonts[f.get('id')] = f.get('face')
        self.bfs = {e.get('id'): e for e in root.iter() if local(e.tag) == 'borderFill'}
        self.parapr = {e.get('id'): hwpx_parapr(e, self.bfs) for e in root.iter() if local(e.tag) == 'paraPr'}
        self.charpr = {e.get('id'): hwpx_charpr(e, self.fonts) for e in root.iter() if local(e.tag) == 'charPr'}
        self.styles = {}
        self.style_names = {}
        for e in root.iter():
            if local(e.tag) == 'style' and e.get('type', 'PARA') == 'PARA':
                self.style_names[e.get('id')] = e.get('name')
                self.styles[e.get('name')] = {'para': dict(self.parapr.get(e.get('paraPrIDRef'), {})),
                                              'char': dict(self.charpr.get(e.get('charPrIDRef'), {}))}
        self.default = self.style_names.get('0')


def hwpx_para(p, H, tables_out, in_cell):
    style = H.style_names.get(p.get('styleIDRef'), H.default)
    runs = []
    for r in kids(p, 'run'):
        cp = H.charpr.get(r.get('charPrIDRef'), {})
        for c in r:
            n = local(c.tag)
            if n == 't':
                runs.append((hwpx_text(c), dict(cp)))
            elif n == 'tbl':
                if not in_cell:
                    tables_out.append(hwpx_table(c, H))
    para = dict(H.parapr.get(p.get('paraPrIDRef'), {}))
    merged = []
    for t, pr in runs:
        if merged and merged[-1][1] == pr:
            merged[-1] = (merged[-1][0] + t, pr)
        elif t:
            merged.append((t, pr))
    if not merged and runs:
        merged = [('', runs[0][1])]
    if not runs:
        r0 = kid(p, 'run')
        merged = [('', dict(H.charpr.get(r0.get('charPrIDRef'), {})))] if r0 is not None else [('', {})]
    return Para(style, para, merged)


def hwpx_table(tbl, H):
    cells = {}
    for tr in kids(tbl, 'tr'):
        for tc in kids(tr, 'tc'):
            addr = kid(tc, 'cellAddr')
            span = kid(tc, 'cellSpan')
            sub = kid(tc, 'subList')
            box = hwpx_borderfill(H.bfs.get(tc.get('borderFillIDRef')))
            box['valign'] = HWPX_VALIGN.get(sub.get('vertAlign', 'CENTER') if sub is not None else 'CENTER', 'middle')
            paras = [hwpx_para(p, H, [], True) for p in kids(sub, 'p')] if sub is not None else []
            rc = (int(addr.get('rowAddr')), int(addr.get('colAddr')))
            cells[rc] = Cell(box, paras, (int(span.get('rowSpan', '1')), int(span.get('colSpan', '1'))))
    return Table(cells)


def extract_hwpx(path):
    z = zipfile.ZipFile(path)
    H = HwpxHeader(z.read('Contents/header.xml'))
    secs = sorted((n for n in z.namelist() if re.fullmatch(r'Contents/section\d+\.xml', n)),
                  key=lambda n: int(re.search(r'(\d+)', n.rsplit('/', 1)[1]).group(1)))
    items = []
    for s in secs:
        root = ET.fromstring(z.read(s))
        for p in kids(root, 'p'):
            tables = []
            para = hwpx_para(p, H, tables, False)
            items.append(('p', para))
            for t in tables:
                items.append(('t', t))
    return Doc('hwpx', H.styles, H.default, {}, items)


# ================================================================ docx

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
DOCX_ALIGN = {'left': 'left', 'start': 'left', 'center': 'center', 'right': 'right', 'end': 'right', 'both': 'justify',
              'distribute': 'distribute', 'thaiDistribute': 'distribute', 'lowKashida': 'justify',
              'mediumKashida': 'justify', 'highKashida': 'justify'}
DOCX_LINE = {'single': 'solid', 'thick': 'solid', 'dashed': 'dashed', 'dashSmallGap': 'dashed', 'dotted': 'dotted',
             'double': 'double', 'dotDash': 'dash-dot', 'dotDotDash': 'dash-dot-dot', 'triple': 'triple',
             'wave': 'wave', 'doubleWave': 'wave', 'threeDEmboss': '3d', 'threeDEngrave': '3d', 'outset': '3d',
             'inset': '3d'}
THEME_NAMES = {'dark1': 'tx1', 'light1': 'bg1', 'dark2': 'tx2', 'light2': 'bg2',
               'text1': 'tx1', 'background1': 'bg1', 'text2': 'tx2', 'background2': 'bg2',
               'accent1': 'accent1', 'accent2': 'accent2', 'accent3': 'accent3', 'accent4': 'accent4',
               'accent5': 'accent5', 'accent6': 'accent6', 'hyperlink': 'hlink', 'followedHyperlink': 'hlink'}


def wv(e, name, attr='val'):
    if e is None:
        return None
    c = e.find(W + name)
    return None if c is None else c.get(W + attr)


def docx_color(val, theme=None, tint=None, shade=None):
    if theme:
        name = THEME_NAMES.get(theme, 'tx1')
        if tint:
            return V.theme_mod(name, lighter=1 - int(tint, 16) / 255)
        if shade:
            return V.theme_mod(name, darker=1 - int(shade, 16) / 255)
        return name
    if not val or val == 'auto':
        return None
    return '#' + val.upper()


def docx_border(e):
    if e is None:
        return None
    v = e.get(W + 'val')
    if v in (None, 'nil', 'none'):
        return 'none'
    sz = int(e.get(W + 'sz', '4')) / 8
    col = docx_color(e.get(W + 'color'), e.get(W + 'themeColor'), e.get(W + 'themeTint'),
                     e.get(W + 'themeShade')) or '#000000'
    return '%s %s %s' % (V.pt(max(sz, 0.25)), DOCX_LINE.get(v, 'solid'), col)


def docx_shd(e):
    if e is None:
        return None
    v = e.get(W + 'val')
    if v == 'nil':
        return 'none'
    if v not in (None, 'clear', 'solid'):
        return 'pattern'
    fill = e.get(W + 'color') if v == 'solid' else e.get(W + 'fill')
    col = docx_color(fill, e.get(W + 'themeFill'), e.get(W + 'themeFillTint'), e.get(W + 'themeFillShade'))
    return col or 'none'


def ppr_props(ppr, size_pt):
    """Only what the pPr sets."""
    p = {}
    if ppr is None:
        return p
    jc = wv(ppr, 'jc')
    if jc:
        p['align'] = DOCX_ALIGN.get(jc, 'left')
    ind = ppr.find(W + 'ind')
    if ind is not None:
        def g(*names):
            for n in names:
                v = ind.get(W + n)
                if v is not None:
                    return int(v)
            return None
        ch = {k: ind.get(W + k) for k in ('leftChars', 'startChars', 'firstLineChars', 'hangingChars')}
        l = g('left', 'start')
        if ch['leftChars'] or ch['startChars']:
            l = int(ch['leftChars'] or ch['startChars']) / 100 * size_pt * 20
        if l is not None:
            p['indent-left'] = V.pt(l / 20)
        r = g('right', 'end')
        if r is not None:
            p['indent-right'] = V.pt(r / 20)
        fl, hg = g('firstLine'), g('hanging')
        if ch['firstLineChars']:
            fl = int(ch['firstLineChars']) / 100 * size_pt * 20
        if ch['hangingChars']:
            hg = int(ch['hangingChars']) / 100 * size_pt * 20
        if hg:
            p['first-line'] = V.pt(-hg / 20)
        elif fl is not None:
            p['first-line'] = V.pt(fl / 20)
    sp = ppr.find(W + 'spacing')
    if sp is not None:
        if sp.get(W + 'before') is not None:
            p['space-before'] = V.pt(int(sp.get(W + 'before')) / 20)
        if sp.get(W + 'after') is not None:
            p['space-after'] = V.pt(int(sp.get(W + 'after')) / 20)
        if sp.get(W + 'line') is not None:
            ln, rule = int(sp.get(W + 'line')), sp.get(W + 'lineRule', 'auto')
            p['line-spacing'] = (V.num(ln / 240 * 100) + '%' if rule == 'auto' else
                                 V.pt(ln / 20) if rule == 'exact' else 'at-least ' + V.pt(ln / 20))
    shd = docx_shd(ppr.find(W + 'shd'))
    if shd:
        p['fill'] = shd
    bdr = ppr.find(W + 'pBdr')
    if bdr is not None:
        for side, tag in (('border-top', 'top'), ('border-right', 'right'), ('border-bottom', 'bottom'),
                          ('border-left', 'left')):
            b = docx_border(bdr.find(W + tag))
            if b:
                p[side] = b
    return p


def rpr_props(rpr):
    p = {}
    if rpr is None:
        return p
    f = rpr.find(W + 'rFonts')
    if f is not None:
        face = f.get(W + 'eastAsia') or f.get(W + 'ascii') or f.get(W + 'hAnsi')
        if face:
            p['font'] = face
    sz = wv(rpr, 'sz')
    if sz:
        p['size'] = V.pt(int(sz) / 2)
    c = rpr.find(W + 'color')
    if c is not None:
        col = docx_color(c.get(W + 'val'), c.get(W + 'themeColor'), c.get(W + 'themeTint'), c.get(W + 'themeShade'))
        p['color'] = col or '#000000'
    for k, tag in (('bold', 'b'), ('italic', 'i'), ('strike', 'strike')):
        e = rpr.find(W + tag)
        if e is not None:
            p[k] = 'no' if e.get(W + 'val') in ('0', 'false', 'off') else 'yes'
    u = rpr.find(W + 'u')
    if u is not None:
        p['underline'] = 'no' if u.get(W + 'val') in ('none', None) else 'yes'
    return p


DOCX_PARA_DEFAULT = {'align': 'left', 'indent-left': '0pt', 'indent-right': '0pt', 'first-line': '0pt',
                     'space-before': '0pt', 'space-after': '0pt', 'line-spacing': '100%', 'fill': 'none',
                     'border-top': 'none', 'border-right': 'none', 'border-bottom': 'none', 'border-left': 'none'}
DOCX_CHAR_DEFAULT = {'font': 'Times New Roman', 'size': '10pt', 'color': '#000000', 'bold': 'no', 'italic': 'no',
                     'underline': 'no', 'strike': 'no'}


class DocxStyles:
    def __init__(self, xml, theme_xml):
        self.palette = docx_palette(theme_xml)
        self.minor = self.palette.pop('_minor', None)
        root = ET.fromstring(xml) if xml else ET.Element('x')
        dd = root.find(W + 'docDefaults')
        char = dict(DOCX_CHAR_DEFAULT)
        if self.minor:
            char['font'] = self.minor
        para = dict(DOCX_PARA_DEFAULT)
        if dd is not None:
            rpd = dd.find(W + 'rPrDefault/' + W + 'rPr')
            f = rpd.find(W + 'rFonts') if rpd is not None else None
            if f is not None and not (f.get(W + 'eastAsia') or f.get(W + 'ascii')) and self.minor:
                pass
            char.update(rpr_props(rpd))
            para.update(ppr_props(dd.find(W + 'pPrDefault/' + W + 'pPr'), 10))
        self.base = {'para': para, 'char': char}
        self.raw = {}
        self.names = {}
        self.types = {}
        self.default_para = None
        for s in root.findall(W + 'style'):
            sid = s.get(W + 'styleId')
            name = wv(s, 'name') or sid
            self.names[sid] = name
            self.types[sid] = s.get(W + 'type')
            if s.get(W + 'type') == 'paragraph' and s.get(W + 'default') in ('1', 'true'):
                self.default_para = sid
            self.raw[sid] = s
        self.cache = {}

    def resolve(self, sid, depth=0):
        """Effective {'para','char'} of a paragraph or character style (basedOn chain over the defaults)."""
        if sid in self.cache:
            return self.cache[sid]
        s = self.raw.get(sid)
        if s is None or depth > 20:
            return {'para': dict(self.base['para']), 'char': dict(self.base['char'])}
        based = wv(s, 'basedOn')
        base = self.resolve(based, depth + 1) if based else {'para': dict(self.base['para']),
                                                             'char': dict(self.base['char'])}
        char = dict(base['char'])
        char.update(rpr_props(s.find(W + 'rPr')))
        para = dict(base['para'])
        para.update(ppr_props(s.find(W + 'pPr'), float(char['size'][:-2])))
        out = {'para': para, 'char': char}
        self.cache[sid] = out
        return out

    def table_style(self, sid):
        """(box props for every cell, {conditional part: (box, char)}) from a table style and its basedOn."""
        s = self.raw.get(sid)
        if s is None:
            return {}, {}
        based = wv(s, 'basedOn')
        box, cond = self.table_style(based) if based else ({}, {})
        box = dict(box)
        cond = dict(cond)
        tp = s.find(W + 'tblPr')
        if tp is not None and tp.find(W + 'tblBorders') is not None:
            box['_tbl'] = tbl_borders(tp.find(W + 'tblBorders'))
        tc = s.find(W + 'tcPr')
        if tc is not None and docx_shd(tc.find(W + 'shd')):
            box['fill'] = docx_shd(tc.find(W + 'shd'))
        for part in s.findall(W + 'tblStylePr'):
            t = part.get(W + 'type')
            pbox = {}
            tcp = part.find(W + 'tcPr')
            if tcp is not None:
                if docx_shd(tcp.find(W + 'shd')):
                    pbox['fill'] = docx_shd(tcp.find(W + 'shd'))
                if tcp.find(W + 'tcBorders') is not None:
                    pbox.update(tc_borders(tcp.find(W + 'tcBorders')))
            cond[t] = (pbox, rpr_props(part.find(W + 'rPr')))
        return box, cond


def docx_palette(xml):
    pal = {}
    if not xml:
        return pal
    root = ET.fromstring(xml)
    A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
    cs = root.find('.//' + A + 'clrScheme')
    if cs is not None:
        for c in cs:
            n = local(c.tag)
            srgb = c.find(A + 'srgbClr')
            sysc = c.find(A + 'sysClr')
            val = srgb.get('val') if srgb is not None else (sysc.get('lastClr') if sysc is not None else None)
            name = {'dk1': 'tx1', 'lt1': 'bg1', 'dk2': 'tx2', 'lt2': 'bg2'}.get(n, n)
            if val and name in V.THEME:
                pal[name] = '#' + val.upper()
    mf = root.find('.//' + A + 'minorFont')
    if mf is not None:
        for f in mf.findall(A + 'font'):
            if f.get('script') == 'Hang':
                pal['_minor'] = f.get('typeface')
        if '_minor' not in pal and mf.find(A + 'latin') is not None:
            pal['_minor'] = mf.find(A + 'latin').get('typeface')
    return pal


def tbl_borders(e):
    return {k: docx_border(e.find(W + k)) for k in ('top', 'left', 'bottom', 'right', 'insideH', 'insideV', 'start',
                                                    'end') if e.find(W + k) is not None}


def tc_borders(e):
    out = {}
    for side, tags in (('border-top', ('top',)), ('border-right', ('right', 'end')),
                       ('border-bottom', ('bottom',)), ('border-left', ('left', 'start'))):
        for t in tags:
            b = docx_border(e.find(W + t))
            if b:
                out[side] = b
                break
    return out


def docx_text(r):
    out = []
    for c in r:
        n = local(c.tag)
        if n in ('t', 'delText') and n == 't':
            out.append(c.text or '')
        elif n == 'tab':
            out.append('\t')
        elif n in ('br', 'cr'):
            if c.get(W + 'type') in (None, 'textWrapping'):
                out.append('\n')
        elif n == 'noBreakHyphen':
            out.append('-')
    return ''.join(out)


def docx_para(p, S, cond_char=None):
    ppr = p.find(W + 'pPr')
    sid = wv(ppr, 'pStyle') or S.default_para
    st = S.resolve(sid)
    char_base = dict(st['char'])
    if cond_char:
        char_base.update(cond_char)
    para = dict(st['para'])
    para.update(ppr_props(ppr, float(char_base['size'][:-2])))
    runs = []

    def walk(e):
        for c in e:
            n = local(c.tag)
            if n == 'r':
                rpr = c.find(W + 'rPr')
                cp = dict(char_base)
                rs = wv(rpr, 'rStyle')
                if rs:
                    cp.update(rpr_props(S.raw[rs].find(W + 'rPr')) if rs in S.raw else {})
                cp.update(rpr_props(rpr))
                t = docx_text(c)
                if t:
                    runs.append((t, cp))
            elif n in ('hyperlink', 'ins', 'smartTag', 'sdt', 'sdtContent', 'fldSimple', 'customXml', 'bdo', 'dir'):
                walk(c)
    walk(p)
    merged = []
    for t, pr in runs:
        if merged and merged[-1][1] == pr:
            merged[-1] = (merged[-1][0] + t, pr)
        else:
            merged.append((t, pr))
    if not merged:
        merged = [('', char_base)]
    return Para(S.names.get(sid, sid), para, merged)


def docx_table(tbl, S):
    tpr = tbl.find(W + 'tblPr')
    tsid = wv(tpr, 'tblStyle')
    sbox, cond = S.table_style(tsid) if tsid else ({}, {})
    look = tpr.find(W + 'tblLook') if tpr is not None else None
    first_row = False
    if look is not None:
        if look.get(W + 'firstRow') is not None:
            first_row = look.get(W + 'firstRow') in ('1', 'true')
        elif look.get(W + 'val'):
            first_row = bool(int(look.get(W + 'val'), 16) & 0x20)
    tb = dict(sbox.get('_tbl', {}))
    if tpr is not None and tpr.find(W + 'tblBorders') is not None:
        tb.update(tbl_borders(tpr.find(W + 'tblBorders')))
    rows = tbl.findall(W + 'tr')
    grid = {}
    nrows = len(rows)
    for ri, tr in enumerate(rows):
        col = 0
        trpr = tr.find(W + 'trPr')
        gb = trpr.find(W + 'gridBefore') if trpr is not None else None
        if gb is not None:
            col += int(gb.get(W + 'val', '0'))
        tcs = tr.findall(W + 'tc')
        for ci, tc in enumerate(tcs):
            tcpr = tc.find(W + 'tcPr')
            span = int(wv(tcpr, 'gridSpan') or 1)
            vm = tcpr.find(W + 'vMerge') if tcpr is not None else None
            box = dict(NO_BOX)
            last_col = ci == len(tcs) - 1
            # table borders by position (outer or inside)
            for side, key in (('border-top', 'top' if ri == 0 else 'insideH'),
                              ('border-bottom', 'bottom' if ri == nrows - 1 else 'insideH'),
                              ('border-left', 'left' if col == 0 else 'insideV'),
                              ('border-right', 'right' if last_col else 'insideV')):
                b = tb.get(key) or (tb.get('start') if key == 'left' else tb.get('end') if key == 'right' else None)
                if b:
                    box[side] = b
            if sbox.get('fill'):
                box['fill'] = sbox['fill']
            cchar = None
            if first_row and ri == 0 and 'firstRow' in cond:
                box.update(cond['firstRow'][0])
                cchar = cond['firstRow'][1]
            if tcpr is not None:
                if tcpr.find(W + 'tcBorders') is not None:
                    box.update(tc_borders(tcpr.find(W + 'tcBorders')))
                shd = docx_shd(tcpr.find(W + 'shd'))
                if shd:
                    box['fill'] = shd
                va = wv(tcpr, 'vAlign')
                if va:
                    box['valign'] = {'top': 'top', 'center': 'middle', 'bottom': 'bottom'}.get(va, 'top')
            paras = [docx_para(p, S, cchar) for p in tc.findall(W + 'p')]
            if vm is not None and vm.get(W + 'val') in (None, 'continue'):
                pass  # covered: the text shows ^^
            else:
                grid[(ri, col)] = Cell(box, paras, (1, span))
            col += span
    return Table(grid, style=S.names.get(tsid, tsid) if tsid else None)


def extract_docx(path):
    z = zipfile.ZipFile(path)
    names = z.namelist()
    theme = next((n for n in names if re.fullmatch(r'word/theme/theme\d*\.xml', n)), None)
    S = DocxStyles(z.read('word/styles.xml') if 'word/styles.xml' in names else None,
                   z.read(theme) if theme else None)
    root = ET.fromstring(z.read('word/document.xml'))
    body = root.find(W + 'body')
    items = []

    def walk(e):
        for c in e:
            n = local(c.tag)
            if n == 'p':
                items.append(('p', docx_para(c, S)))
            elif n == 'tbl':
                items.append(('t', docx_table(c, S)))
            elif n in ('sdt', 'sdtContent', 'customXml'):
                walk(c)
    walk(body)
    styles = {}
    for sid, s in S.raw.items():
        if S.types.get(sid) == 'paragraph':
            styles[S.names[sid]] = S.resolve(sid)
    default = S.names.get(S.default_para, 'Normal')
    if default not in styles:
        styles[default] = {'para': dict(S.base['para']), 'char': dict(S.base['char'])}
    return Doc('docx', styles, default, S.palette, items)


def extract(path):
    if path.endswith('.hwpx'):
        return extract_hwpx(path)
    if path.endswith('.docx'):
        return extract_docx(path)
    raise ValueError(path)


if __name__ == '__main__':
    import sys
    d = extract(sys.argv[1])
    print(d.default, d.palette)
    for n, s in d.styles.items():
        print('STYLE', n, s)
    for kind, x in d.items[:int(sys.argv[2]) if len(sys.argv) > 2 else 40]:
        if kind == 'p':
            print('P', x.style, x.para, [(t[:20], r) for t, r in x.runs][:3])
        else:
            for rc, c in sorted(x.cells.items()):
                print('  C', rc, c.box, [(p.text[:20], p.para.get('align')) for p in c.paras])
