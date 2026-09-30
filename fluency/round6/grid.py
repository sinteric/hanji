"""Workbooks (xlsx) in the candidates: cell formatting as range lines in the sheet structure (render only).

  F1  `<format range="B4:C4" fill=… bold border-bottom=…/>` for every rectangle of cells whose effective formatting
      is not the default (the Normal style), inside the sheet block, and one `<format default …/>` line at the top
      (Normal's font, size and colour); a range operation `format` writes it (xlsx_kit.py).
  F2  the named cell styles as `<style name="Heading 1" …/>` lines; a range shows `style="Name"` and what its cells
      set beyond the style.
  F3  `<format range style="Name"/>` for cells in a style other than Normal, and the style lines.

Excel border styles are shown in the vocabulary (thin 0.75pt solid, medium 1.5pt solid, thick 2.25pt solid, double
2.25pt double, hair 0.25pt dotted, dashed 0.75pt dashed, …); a written width snaps to the nearest Excel style, and
the text the write returns shows it. Indent is Excel's `indent` in levels."""
import os
import re
import xml.etree.ElementTree as ET
import zipfile

import vocab as V

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
THEME_IDX = ['bg1', 'tx1', 'bg2', 'tx2', 'accent1', 'accent2', 'accent3', 'accent4', 'accent5',
             'accent6', 'hlink']
XL_BORDER = {'thin': (0.75, 'solid'), 'medium': (1.5, 'solid'), 'thick': (2.25, 'solid'), 'double': (2.25, 'double'),
             'hair': (0.25, 'dotted'), 'dotted': (0.75, 'dotted'), 'dashed': (0.75, 'dashed'),
             'mediumDashed': (1.5, 'dashed'), 'dashDot': (0.75, 'dash-dot'), 'mediumDashDot': (1.5, 'dash-dot'),
             'dashDotDot': (0.75, 'dash-dot-dot'), 'mediumDashDotDot': (1.5, 'dash-dot-dot'),
             'slantDashDot': (1.5, 'dash-dot')}
INDEXED = {8: '#000000', 9: '#FFFFFF', 10: '#FF0000', 11: '#00FF00', 12: '#0000FF', 13: '#FFFF00', 22: '#C0C0C0',
           23: '#808080', 64: '#000000', 65: '#FFFFFF'}


def xcolor(e):
    if e is None:
        return None
    if e.get('rgb'):
        return '#' + e.get('rgb')[-6:].upper()
    if e.get('theme') is not None:
        name = THEME_IDX[int(e.get('theme'))] if int(e.get('theme')) < len(THEME_IDX) else 'tx1'
        t = float(e.get('tint', '0'))
        return V.theme_mod(name, lighter=t) if t > 0 else V.theme_mod(name, darker=-t)
    if e.get('indexed') is not None:
        return INDEXED.get(int(e.get('indexed')), '#000000')
    if e.get('auto'):
        return '#000000'
    return None


class Styles:
    def __init__(self, xml):
        root = ET.fromstring(xml)
        self.fonts = []
        for f in root.find(S + 'fonts') or []:
            p = {}
            for tag, key in (('b', 'bold'), ('i', 'italic'), ('strike', 'strike')):
                e = f.find(S + tag)
                p[key] = 'yes' if e is not None and e.get('val', '1') not in ('0', 'false') else 'no'
            u = f.find(S + 'u')
            p['underline'] = 'yes' if u is not None and u.get('val', 'single') != 'none' else 'no'
            sz = f.find(S + 'sz')
            p['size'] = V.pt(float(sz.get('val'))) if sz is not None else '11pt'
            p['color'] = xcolor(f.find(S + 'color')) or '#000000'
            nm = f.find(S + 'name')
            p['font'] = nm.get('val') if nm is not None else ''
            self.fonts.append(p)
        self.fills = []
        for f in root.find(S + 'fills') or []:
            pf = f.find(S + 'patternFill')
            if f.find(S + 'gradientFill') is not None:
                self.fills.append('gradient')
            elif pf is None or pf.get('patternType') in (None, 'none'):
                self.fills.append('none')
            elif pf.get('patternType') == 'solid':
                self.fills.append(xcolor(pf.find(S + 'fgColor')) or '#000000')
            else:
                self.fills.append('pattern')
        self.borders = []
        for b in root.find(S + 'borders') or []:
            p = {}
            for side, tag in (('border-top', 'top'), ('border-right', 'right'), ('border-bottom', 'bottom'),
                              ('border-left', 'left')):
                e = b.find(S + tag)
                if e is None:
                    e = b.find(S + {'left': 'start', 'right': 'end'}.get(tag, tag))
                if e is None or not e.get('style'):
                    p[side] = 'none'
                else:
                    w, st = XL_BORDER.get(e.get('style'), (0.75, 'solid'))
                    p[side] = '%s %s %s' % (V.pt(w), st, xcolor(e.find(S + 'color')) or '#000000')
            self.borders.append(p)
        self.xfs = [self.xf(x) for x in (root.find(S + 'cellXfs') or [])]
        self.style_xfs = [self.xf(x) for x in (root.find(S + 'cellStyleXfs') or [])]
        self.cell_xf_style = [int(x.get('xfId', '0')) for x in (root.find(S + 'cellXfs') or [])]
        self.names = {}
        for cs in root.find(S + 'cellStyles') or []:
            self.names[int(cs.get('xfId', '0'))] = cs.get('name')

    def xf(self, x):
        p = {}
        fid, flid, bid = int(x.get('fontId', '0')), int(x.get('fillId', '0')), int(x.get('borderId', '0'))
        if fid < len(self.fonts):
            p.update(self.fonts[fid])
        p['fill'] = self.fills[flid] if flid < len(self.fills) else 'none'
        if bid < len(self.borders):
            p.update(self.borders[bid])
        al = x.find(S + 'alignment')
        p['align'] = {'center': 'center', 'right': 'right', 'left': 'left', 'justify': 'justify',
                      'distributed': 'distribute', 'centerContinuous': 'center'}.get(
            al.get('horizontal') if al is not None else None, 'general')
        p['valign'] = {'top': 'top', 'center': 'middle', 'bottom': 'bottom'}.get(
            al.get('vertical') if al is not None else None, 'bottom')
        p['indent'] = al.get('indent', '0') if al is not None else '0'
        return p


def col_letters(n):
    s = ''
    n += 1
    while n:
        n, r = divmod(n - 1, 26)
        s = chr(65 + r) + s
    return s


def ref_rc(ref):
    m = re.fullmatch(r'([A-Z]+)(\d+)', ref)
    c = 0
    for ch in m.group(1):
        c = c * 26 + ord(ch) - 64
    return int(m.group(2)) - 1, c - 1


def rects(cells):
    """cells: {(r, c): sig} -> [(r1, c1, r2, c2, sig)] covering equal signatures with rectangles."""
    rows = {}
    for (r, c), sig in cells.items():
        rows.setdefault(r, []).append((c, sig))
    runs = []
    for r in sorted(rows):
        cs = sorted(rows[r])
        start = None
        for k, (c, sig) in enumerate(cs):
            if start is None:
                start, prev, psig = c, c, sig
            elif c == prev + 1 and sig == psig:
                prev = c
            else:
                runs.append((r, start, prev, psig))
                start, prev, psig = c, c, sig
        if start is not None:
            runs.append((r, start, prev, psig))
    out = []
    open_ = {}
    for r, c1, c2, sig in runs:
        key = (c1, c2, sig)
        if key in open_ and open_[key][2] == r - 1:
            open_[key][2] = r
        else:
            if key in open_:
                out.append(tuple(open_[key]))
            open_[key] = [r, c1, r, c2, sig]
            open_[key] = [r, c1, r, c2, sig]
    # normalise: [r1, c1, r2, c2, sig]
    res = []
    for v in list(out) + list(open_.values()):
        r1, c1, r2, c2, sig = v[0], v[1], v[2], v[3], v[4]
        res.append((r1, c1, r2, c2, sig))
    return sorted(res)


def rng(r1, c1, r2, c2):
    a = '%s%d' % (col_letters(c1), r1 + 1)
    b = '%s%d' % (col_letters(c2), r2 + 1)
    return a if a == b else '%s:%s' % (a, b)


DEFAULT_XL = {'fill': 'none', 'border-top': 'none', 'border-right': 'none', 'border-bottom': 'none',
              'border-left': 'none', 'align': 'general', 'valign': 'bottom', 'indent': '0', 'bold': 'no',
              'italic': 'no', 'underline': 'no', 'strike': 'no'}


def props_text(d):
    d = {k: v for k, v in d.items() if not (k == 'align' and v == 'general') and not (k == 'font' and not v)}
    return V.fmt_attrs(d)


def sheet_lines(st, xml, cand, base):
    cells = {}
    for c in ET.fromstring(xml).iter(S + 'c'):
        s = int(c.get('s', '0'))
        if s == 0 and cand != 'F2':
            continue
        eff = st.xfs[s] if s < len(st.xfs) else {}
        sid = st.cell_xf_style[s] if s < len(st.cell_xf_style) else 0
        sname = st.names.get(sid, 'Normal')
        if cand == 'F1':
            d = {k: v for k, v in eff.items() if base.get(k, DEFAULT_XL.get(k)) != v}
        elif cand == 'F2':
            sbase = st.style_xfs[sid] if sid < len(st.style_xfs) else base
            d = {k: v for k, v in eff.items() if sbase.get(k, DEFAULT_XL.get(k)) != v}
            if sname != 'Normal':
                d['style'] = sname
        else:
            d = {'style': sname} if sname != 'Normal' else {}
        if d:
            cells[ref_rc(c.get('r'))] = tuple(sorted(d.items()))
    out = []
    for r1, c1, r2, c2, sig in rects(cells):
        out.append('<format range="%s" %s/>' % (rng(r1, c1, r2, c2), props_text(dict(sig))))
    return out


def render(path, text, cand):
    z = zipfile.ZipFile(path)
    st = Styles(z.read('xl/styles.xml'))
    wb = ET.fromstring(z.read('xl/workbook.xml'))
    rels = ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))
    rid = {r.get('Id'): r.get('Target') for r in rels}
    sheets = {}
    for s in wb.find(S + 'sheets'):
        t = rid[s.get(R + 'id')]
        t = t[1:] if t.startswith('/') else 'xl/' + t
        sheets[s.get('name')] = t
    base = st.style_xfs[0] if st.style_xfs else (st.xfs[0] if st.xfs else {})
    out = []
    lines = text.split('\n')
    k = 0
    if cand == 'F1':
        # the workbook's default cell formatting (Normal), so a cell in no range line reads on its own
        d = {k2: v for k2, v in base.items() if k2 in ('font', 'size', 'color') and v}
        out += ['<format default %s/>' % V.fmt_attrs(d), '']
    if cand in ('F2', 'F3'):
        end = lines.index('---', 1) if lines and lines[0] == '---' and '---' in lines[1:] else -1
        out += lines[:end + 1] + ([''] if end >= 0 else [])
        for sid, name in sorted(st.names.items()):
            sx = st.style_xfs[sid] if sid < len(st.style_xfs) else {}
            if name == 'Normal':
                d = {k2: v for k2, v in sx.items() if v not in ('none', 'no', '0', 'general', 'bottom')}
            else:
                d = {k2: v for k2, v in sx.items() if base.get(k2) != v}
            out.append('<style name="%s"%s/>' % (name, (' ' + props_text(d)) if d else ''))
        out.append('')
        k = end + 1
    while k < len(lines):
        ln = lines[k]
        out.append(ln)
        m = re.match(r'^<sheet name="([^"]*)"', ln)
        if m and m.group(1) in sheets and sheets[m.group(1)] in z.namelist():
            fl = sheet_lines(st, z.read(sheets[m.group(1)]), cand, base)
            if fl:
                out.append('')
                out += fl
        k += 1
    return '\n'.join(out)


def measure(dumps, corpus):
    rows = []
    for f in sorted(os.listdir(corpus)):
        if not f.endswith('.xlsx'):
            continue
        tp = os.path.join(dumps, f + '.txt')
        if not os.path.exists(tp) or os.path.getsize(tp) == 0:
            continue
        text = open(tp, encoding='utf-8').read()
        row = {'file': f, 'format': 'xlsx', 'today': len(text)}
        try:
            for c in ('F1', 'F2', 'F3'):
                row[c] = len(render(os.path.join(corpus, f), text, c))
        except Exception as ex:
            row['error'] = '%s: %s' % (type(ex).__name__, str(ex)[:120])
        rows.append(row)
    return rows


if __name__ == '__main__':
    import sys
    print(render(sys.argv[1], open(sys.argv[2], encoding='utf-8').read(), sys.argv[3]))
