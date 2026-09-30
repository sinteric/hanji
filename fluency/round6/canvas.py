"""Presentations (pptx) in the candidates: formatting on slide objects, shown on today's text (render only; the
fluency round measured flow documents, so no reader is needed here).

  F1  every object shows its effective formatting inline, like its box: fill and border on the shape tag or slot
      marker, the text properties its paragraphs share there too, a paragraph's own at the end of its line
      (`{…}`, before `<p/>` or `</shape>` in a one-line shape), a run's as `[text]{…}`.
  F2  the same places hold only what the slide itself sets; a style section lists what the theme's shape style
      and the master's text styles give (`<style name="shape" …/>`, `title`, `body`), and a shape the theme style
      draws says so with `style="shape"`.
  F3  the style section only.

Theme colours stay theme colours: `accent1`, `accent1+40%` (lighter), `accent1-25%` (darker). Units: points."""
import os
import re
import xml.etree.ElementTree as ET
import zipfile

import vocab as V

A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
PNS = '{http://schemas.openxmlformats.org/presentationml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
EMU = 12700
SCHEME = {'tx1': 'tx1', 'bg1': 'bg1', 'tx2': 'tx2', 'bg2': 'bg2', 'dk1': 'tx1',
          'lt1': 'bg1', 'dk2': 'tx2', 'lt2': 'bg2', 'hlink': 'hlink', 'folHlink': 'hlink',
          'phClr': None}
DASH = {'solid': 'solid', 'dash': 'dashed', 'lgDash': 'dashed', 'sysDash': 'dashed', 'dot': 'dotted',
        'sysDot': 'dotted', 'dashDot': 'dash-dot', 'lgDashDot': 'dash-dot', 'sysDashDot': 'dash-dot',
        'lgDashDotDot': 'dash-dot-dot', 'sysDashDotDot': 'dash-dot-dot'}
ALGN = {'l': 'left', 'ctr': 'center', 'r': 'right', 'just': 'justify', 'dist': 'distribute'}


def local(t):
    return t.rsplit('}', 1)[-1]


def color_of(e, ph=None):
    """A colour element parent (solidFill, lnRef, …) -> canonical colour."""
    if e is None:
        return None
    for c in e:
        n = local(c.tag)
        if n == 'srgbClr':
            al = c.find(A + 'alpha')
            out = '#' + c.get('val').upper()
            if al is not None and int(al.get('val')) < 99500:
                out += '/%d%%' % round(int(al.get('val')) / 1000)
            return out
        if n in ('schemeClr', 'sysClr', 'prstClr'):
            if n == 'sysClr':
                return '#' + (c.get('lastClr') or '000000').upper()
            if n == 'prstClr':
                return {'black': '#000000', 'white': '#FFFFFF', 'red': '#FF0000'}.get(c.get('val'), '#000000')
            name = SCHEME.get(c.get('val'), c.get('val'))
            if name is None:
                return ph
            mods = {local(m.tag): int(m.get('val')) / 100000 for m in c}
            alpha = mods.pop('alpha', None)
            lm, lo = mods.pop('lumMod', None), mods.pop('lumOff', None)
            if mods:
                out = name + '*'          # shade, tint, satMod, …: kept as written
            elif lo and lm is not None and abs(lm + lo - 1) < 0.001:
                out = V.theme_mod(name, lighter=lo)
            elif lm is not None and not lo:
                out = V.theme_mod(name, darker=1 - lm)
            elif lm is not None or lo is not None:
                out = name + '*'
            else:
                out = name
            if alpha is not None and alpha < 0.995:
                out += '/%d%%' % round(alpha * 100)
            return out
    return None


def fill_of(sppr, ph=None):
    if sppr is None:
        return None
    for c in sppr:
        n = local(c.tag)
        if n == 'noFill':
            return 'none'
        if n == 'solidFill':
            return color_of(c, ph)
        if n == 'gradFill':
            return 'gradient'
        if n == 'blipFill':
            return 'picture'
        if n == 'pattFill':
            return 'pattern'
    return None


def line_of(ln, ph=None, w_default=None):
    if ln is None:
        return None
    if ln.find(A + 'noFill') is not None:
        return 'none'
    w = int(ln.get('w', w_default or 9525)) / EMU
    col = color_of(ln.find(A + 'solidFill'), ph)
    if col is None and ln.find(A + 'solidFill') is None and ph is None:
        return None
    dash = ln.find(A + 'prstDash')
    st = DASH.get(dash.get('val'), 'solid') if dash is not None else 'solid'
    if ln.get('cmpd') in ('dbl',):
        st = 'double'
    return '%s %s %s' % (V.pt(w), st, col or ph or '#000000')


class Theme:
    def __init__(self, xml):
        root = ET.fromstring(xml)
        self.palette = {}
        cs = root.find('.//' + A + 'clrScheme')
        for c in cs if cs is not None else []:
            n = {'dk1': 'tx1', 'lt1': 'bg1', 'dk2': 'tx2', 'lt2': 'bg2'}.get(local(c.tag), local(c.tag))
            s = c.find(A + 'srgbClr')
            y = c.find(A + 'sysClr')
            v = s.get('val') if s is not None else (y.get('lastClr') if y is not None else None)
            if v and n in V.THEME:
                self.palette[n] = '#' + v.upper()
        self.lines = [int(x.get('w', '9525')) for x in root.iter(A + 'ln')][:3]
        mf = root.find('.//' + A + 'minorFont')
        self.minor = None
        if mf is not None:
            ea = mf.find(A + 'ea')
            lat = mf.find(A + 'latin')
            self.minor = (ea.get('typeface') if ea is not None and ea.get('typeface') else None) or \
                (lat.get('typeface') if lat is not None else None)


def style_props(st, theme):
    """p:style -> (fill, border, text colour)."""
    if st is None:
        return {}
    out = {}
    fr = st.find(A + 'fillRef')
    if fr is not None:
        out['fill'] = 'none' if fr.get('idx') == '0' else color_of(fr)
    lr = st.find(A + 'lnRef')
    if lr is not None:
        idx = int(lr.get('idx', '0'))
        if idx == 0:
            out['border'] = 'none'
        else:
            w = theme.lines[idx - 1] if idx - 1 < len(theme.lines) else 9525
            out['border'] = '%s solid %s' % (V.pt(w / EMU), color_of(lr) or '#000000')
    fo = st.find(A + 'fontRef')
    if fo is not None and color_of(fo):
        out['color'] = color_of(fo)
    return out


def ppr_props(ppr):
    p = {}
    if ppr is None:
        return p
    if ppr.get('algn'):
        p['align'] = ALGN.get(ppr.get('algn'), 'left')
    if ppr.get('marL') is not None:
        p['indent-left'] = V.pt(int(ppr.get('marL')) / EMU)
    if ppr.get('indent') is not None:
        p['first-line'] = V.pt(int(ppr.get('indent')) / EMU)
    for key, tag in (('space-before', 'spcBef'), ('space-after', 'spcAft')):
        e = ppr.find(A + tag)
        if e is not None and e.find(A + 'spcPts') is not None:
            p[key] = V.pt(int(e.find(A + 'spcPts').get('val')) / 100)
    ls = ppr.find(A + 'lnSpc')
    if ls is not None:
        if ls.find(A + 'spcPct') is not None:
            p['line-spacing'] = V.num(int(ls.find(A + 'spcPct').get('val')) / 1000) + '%'
        elif ls.find(A + 'spcPts') is not None:
            p['line-spacing'] = V.pt(int(ls.find(A + 'spcPts').get('val')) / 100)
    return p


def rpr_props(rpr):
    p = {}
    if rpr is None:
        return p
    if rpr.get('sz'):
        p['size'] = V.pt(int(rpr.get('sz')) / 100)
    c = color_of(rpr.find(A + 'solidFill'))
    if c:
        p['color'] = c
    for tag in ('ea', 'latin'):
        e = rpr.find(A + tag)
        if e is not None and e.get('typeface') and not e.get('typeface').startswith('+'):
            p['font'] = e.get('typeface')
            break
    return p


def lst_level1(lst):
    """lstStyle / txStyles element -> (para props, char props) of level 1."""
    if lst is None:
        return {}, {}
    l1 = lst.find(A + 'lvl1pPr')
    if l1 is None:
        return {}, {}
    return ppr_props(l1), rpr_props(l1.find(A + 'defRPr'))


class Obj:
    def __init__(self, oid, name, kind, ph, box, direct, styled, style_ref, paras):
        self.id, self.name, self.kind, self.ph = oid, name, kind, ph
        self.box, self.direct, self.styled, self.style_ref, self.paras = box, direct, styled, style_ref, paras


def read_sp(sp, theme, inherit):
    nv = sp.find('.//' + PNS + 'cNvPr')
    oid, name = nv.get('id'), nv.get('name')
    ph = sp.find('.//' + PNS + 'ph')
    phkey = (ph.get('type', 'body'), ph.get('idx', '0')) if ph is not None else None
    sppr = sp.find(PNS + 'spPr')
    direct = {}
    f = fill_of(sppr)
    if f:
        direct['fill'] = f
    ln = line_of(sppr.find(A + 'ln') if sppr is not None else None)
    if ln:
        direct['border'] = ln
    st = style_props(sp.find(PNS + 'style'), theme)
    styled = dict(st)
    paras = []
    tb = sp.find(PNS + 'txBody')
    base_p, base_c = inherit(phkey) if phkey else ({}, {})
    if tb is not None:
        lp, lc = lst_level1(tb.find(A + 'lstStyle'))
        for p in tb.findall(A + 'p'):
            pp = dict(ppr_props(p.find(A + 'pPr')))
            runs = []
            for r in p:
                if local(r.tag) in ('r', 'fld'):
                    t = ''.join(x.text or '' for x in r.iter(A + 't'))
                    runs.append((t, rpr_props(r.find(A + 'rPr'))))
            paras.append({'para': pp, 'runs': runs, 'base_p': dict(base_p, **lp), 'base_c': dict(base_c, **lc)})
    return Obj(oid, name, 'sp' if local(sp.tag) == 'sp' else local(sp.tag), phkey, None, direct, styled,
               sp.find(PNS + 'style') is not None, paras)


def slide_objects(z, slide_path, theme, inherit):
    root = ET.fromstring(z.read(slide_path))
    tree = root.find('.//' + PNS + 'spTree')
    out = []

    def walk(e):
        for c in e:
            n = local(c.tag)
            if n in ('sp', 'cxnSp'):
                out.append(read_sp(c, theme, inherit))
            elif n == 'grpSp':
                walk(c)
    walk(tree)
    return out


def rels(z, part):
    d, f = part.rsplit('/', 1)
    rp = '%s/_rels/%s.rels' % (d, f)
    if rp not in z.namelist():
        return {}
    out = {}
    for r in ET.fromstring(z.read(rp)):
        t = r.get('Target')
        if not t.startswith('/'):
            t = os.path.normpath(os.path.join(d, t)).replace('\\', '/')
        else:
            t = t[1:]
        out[r.get('Type').rsplit('/', 1)[-1]] = t
    return out


class Deck:
    def __init__(self, path):
        z = zipfile.ZipFile(path)
        pres = ET.fromstring(z.read('ppt/presentation.xml'))
        prels = ET.fromstring(z.read('ppt/_rels/presentation.xml.rels'))
        rid = {r.get('Id'): r.get('Target') for r in prels}
        self.slides = []
        master_path = None
        theme = None
        for s in pres.find(PNS + 'sldIdLst') if pres.find(PNS + 'sldIdLst') is not None else []:
            sp = 'ppt/' + rid[s.get(R + 'id')].lstrip('/').replace('ppt/', '')
            lay = rels(z, sp).get('slideLayout')
            mas = rels(z, lay).get('slideMaster') if lay else None
            if theme is None and mas:
                th = rels(z, mas).get('theme')
                theme = Theme(z.read(th)) if th else None
                master_path = mas
            self.slides.append((sp, lay, mas))
        self.theme = theme or Theme(b'<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>')
        self.styles = {}
        if master_path:
            m = ET.fromstring(z.read(master_path))
            tx = m.find(PNS + 'txStyles')
            if tx is not None:
                for name, tag in (('title', 'titleStyle'), ('body', 'bodyStyle')):
                    p, c = lst_level1(tx.find(PNS + tag))
                    if 'font' not in c and self.theme.minor:
                        c['font'] = self.theme.minor
                    self.styles[name] = (p, c)
        self.z = z
        self.objs = []
        for sp, lay, mas in self.slides:
            self.objs.append(slide_objects(z, sp, self.theme, self.inherit_for(lay, mas)))

    def inherit_for(self, lay, mas):
        z = self.z
        cache = {}

        def ph_props(part):
            if part in cache:
                return cache[part]
            out = {}
            if part and part in z.namelist():
                root = ET.fromstring(z.read(part))
                for sp in root.iter(PNS + 'sp'):
                    ph = sp.find('.//' + PNS + 'ph')
                    if ph is None:
                        continue
                    tb = sp.find(PNS + 'txBody')
                    out[(ph.get('type', 'body'), ph.get('idx', '0'))] = lst_level1(tb.find(A + 'lstStyle')) \
                        if tb is not None else ({}, {})
            cache[part] = out
            return out

        def inherit(key):
            kind = 'title' if key[0] in ('title', 'ctrTitle') else 'body'
            p, c = [dict(x) for x in self.styles.get(kind, ({}, {}))]
            for part in (mas, lay):
                d = ph_props(part)
                hit = d.get(key) or next((v for (t, i), v in d.items() if t == key[0]), None)
                if hit:
                    p.update(hit[0])
                    c.update(hit[1])
            return p, c
        return inherit


def lifted(dicts):
    if not dicts:
        return {}
    out = {}
    for k in set().union(*dicts):
        vals = {d.get(k) for d in dicts}
        if len(vals) == 1 and None not in vals:
            out[k] = vals.pop()
    return out


def obj_text_props(o, cand):
    """-> (object-level text props, [(para props shown, [(text, span props)])]) for cand."""
    rows = []
    for p in o.paras:
        vis = [(t, r) for t, r in p['runs'] if t.strip()]
        if cand == 'F1':
            pe = dict(p['base_p'], **p['para'])
            runs = [(t, dict(p['base_c'], **r)) for t, r in p['runs']]
        else:
            pe = dict(p['para'])
            runs = list(p['runs'])
        rows.append((pe, runs, bool(vis)))
    txt = [r for r in rows if r[2]]
    top_c = lifted([lifted([r for t, r in runs if t.strip()]) for _, runs, _ in txt]) if txt else {}
    top_p = lifted([pe for pe, _, _ in txt]) if txt else {}
    top_p = {k: v for k, v in top_p.items() if v != ZERO.get(k)}
    out = []
    for pe, runs, vis in rows:
        own = {k: v for k, v in pe.items() if top_p.get(k, ZERO.get(k)) != v and vis}
        para_c = lifted([r for t, r in runs if t.strip()])
        own.update({k: v for k, v in para_c.items() if top_c.get(k) != v})
        spans = []
        for t, r in runs:
            d = {k: v for k, v in r.items() if dict(top_c, **{k2: v2 for k2, v2 in para_c.items()}).get(k) != v}
            spans.append((t, d))
        out.append((own, spans))
    return dict(top_p, **top_c), out


ZERO = {'align': 'left', 'indent-left': '0pt', 'indent-right': '0pt', 'first-line': '0pt', 'space-before': '0pt',
        'space-after': '0pt', 'line-spacing': '100%'}


def style_names(deck):
    """The theme shape styles the slides use, named by kind and first use: shape, shape 2, line, …"""
    names = {}
    for objs in deck.objs:
        for o in objs:
            if not o.styled:
                continue
            sig = tuple(sorted(o.styled.items()))
            base = 'line' if o.kind == 'cxnSp' else 'shape'
            if (base, sig) not in names:
                n = sum(1 for (b2, _) in names if b2 == base)
                names[(base, sig)] = base if n == 0 else '%s %d' % (base, n + 1)
    return names


def obj_style_name(deck, o):
    return style_names(deck).get(('line' if o.kind == 'cxnSp' else 'shape', tuple(sorted(o.styled.items()))))


def obj_box_props(o, cand):
    if cand == 'F3':
        return {}
    if cand == 'F1':
        d = dict(o.styled)
        d.pop('color', None)
        d.update(o.direct)
        # absent means none: `none` is written only where it overrides what the theme style would draw
        return {k: v for k, v in d.items() if v != 'none' or (k in o.styled and o.styled[k] != 'none')}
    return dict(o.direct)


SHAPE_RE = re.compile(r'^(<shape\b[^>]*?)(/?)>(.*?)(</shape>)?$')


def render(path, text, cand):
    """Today's pptx text -> the candidate's text."""
    deck = Deck(path)
    lines = text.split('\n')
    end = lines.index('---', 1)
    out = lines[:end + 1]
    if cand in ('F2', 'F3'):
        out.append('')
        for (base, sig), name in style_names(deck).items():
            out.append('<style name="%s" %s/>' % (name, V.fmt_attrs(V.expand(dict(sig)))))
        for n, (p, c) in deck.styles.items():
            d = dict(p, **c)
            out.append('<style name="%s"%s/>' % (n, (' ' + V.fmt_attrs(d)) if d else ''))
    slide = 0
    objs = {o.id: o for o in deck.objs[0]} if deck.objs else {}
    slots = [o for o in deck.objs[0] if o.ph] if deck.objs else []
    cur = None
    cur_para = 0
    i = end + 1
    while i < len(lines):
        ln = lines[i]
        if ln == '---':
            slide += 1
            objs = {o.id: o for o in deck.objs[slide]} if slide < len(deck.objs) else {}
            slots = [o for o in deck.objs[slide] if o.ph] if slide < len(deck.objs) else []
            cur = None
            out.append(ln)
            i += 1
            continue
        m = re.match(r'^::(\w+)( box="[^"]*")?::$', ln)
        if m and m.group(1) != 'notes':
            cur = pick_slot(slots, m.group(1))
            cur_para = 0
            if cur is not None and cand != 'F3':
                top, _ = obj_text_props(cur, cand)
                bx = obj_box_props(cur, cand)
                attrs = dict(bx, **top)
                ln = ln[:-2] + (' ' + V.fmt_attrs(V.expand(attrs)) if attrs else '') + '::'
            out.append(ln)
            i += 1
            continue
        if m:
            cur = None
            out.append(ln)
            i += 1
            continue
        sm = SHAPE_RE.match(ln)
        if sm:
            cur = None
            idm = re.search(r'id="s(\d+)"', sm.group(1))
            o = objs.get(idm.group(1)) if idm else None
            if o is not None and cand != 'F3':
                top, per = obj_text_props(o, cand)
                attrs = dict(obj_box_props(o, cand), **top)
                if cand == 'F2' and o.styled:
                    attrs = dict(attrs, style=obj_style_name(deck, o))
                head = sm.group(1) + (' ' + V.fmt_attrs(V.expand(attrs)) if attrs else '')
                if sm.group(2):
                    ln = head + '/>'
                else:
                    body = sm.group(3)
                    pieces = body.split('<p/>')
                    new = []
                    for k, piece in enumerate(pieces):
                        own = per[k][0] if k < len(per) else {}
                        new.append(piece + (' {%s}' % V.fmt_attrs(own) if own else ''))
                    ln = head + '>' + '<p/>'.join(new) + '</shape>'
            out.append(ln)
            i += 1
            continue
        lm = re.match(r'^(<line\b[^>]*?)/>$', ln)
        if lm:
            idm = re.search(r'id="s(\d+)"', lm.group(1))
            o = objs.get(idm.group(1)) if idm else None
            if o is not None and cand != 'F3':
                bx = obj_box_props(o, cand)
                if cand == 'F2' and o.styled:
                    bx = dict(bx, style=obj_style_name(deck, o))
                if bx:
                    ln = lm.group(1) + ' ' + V.fmt_attrs(V.expand(bx)) + '/>'
            out.append(ln)
            i += 1
            continue
        if cur is not None and ln.strip() and not ln.startswith('<keep') and cand != 'F3':
            _, per = obj_text_props(cur, cand)
            while cur_para < len(cur.paras) and not any(t.strip() for t, _ in cur.paras[cur_para]['runs']):
                cur_para += 1
            if cur_para < len(per):
                own, spans = per[cur_para]
                if own:
                    ln = ln + ' {%s}' % V.fmt_attrs(own)
            cur_para += 1
        out.append(ln)
        i += 1
    return '\n'.join(out) + ('\n' if not text.endswith('\n') else '')


def pick_slot(slots, name):
    want = {'title': ('title', 'ctrTitle'), 'subtitle': ('subTitle',), 'number': ('sldNum',), 'date': ('dt',),
            'footer': ('ftr',), 'picture': ('pic',), 'chart': ('chart',), 'table': ('tbl',)}.get(name)
    if want:
        return next((o for o in slots if o.ph[0] in want), None)
    bodies = [o for o in slots if o.ph[0] in ('body', 'obj') or o.ph[0] not in ('title', 'ctrTitle', 'subTitle', 'dt',
                                                                               'ftr', 'sldNum', 'pic', 'chart',
                                                                               'tbl', 'dgm', 'media', 'clipArt')]
    idx = {'body': 0, 'left': 0, 'right': 1, 'body2': 1, 'body3': 2}.get(name, 0)
    return bodies[idx] if idx < len(bodies) else None


def measure(dumps, corpus):
    rows = []
    for f in sorted(os.listdir(corpus)):
        if not f.endswith('.pptx'):
            continue
        tp = os.path.join(dumps, f + '.txt')
        if not os.path.exists(tp) or os.path.getsize(tp) == 0:
            continue
        text = open(tp, encoding='utf-8').read()
        row = {'file': f, 'format': 'pptx', 'today': len(text)}
        try:
            for c in ('F1', 'F2', 'F2o', 'F3'):
                row[c] = len(render(os.path.join(corpus, f), text, c))
        except Exception as ex:
            row['error'] = '%s: %s' % (type(ex).__name__, str(ex)[:120])
        rows.append(row)
    return rows


if __name__ == '__main__':
    import sys
    print(render(sys.argv[1], open(sys.argv[2], encoding='utf-8').read(), sys.argv[3]))
