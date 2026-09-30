"""Flow documents (docx, hwpx) in the three candidates.

A Model is hanji's text of a real file (from the engine's dump) with the file's formatting attached:
  front      the front matter lines
  styles     {name: {'para', 'char'}} effective properties of each paragraph style (every key)
  default    the default style's name;  box0: a cell's properties when nothing is set (per format)
  heading    {level: style name} for `#` lines;  order: the style names a style section lists
  blocks     Raw(line) | P | T
P: kind (heading/list/div/plain/empty), prefix, style, inline (hanji's inline text, no spans), para (effective),
   runs [(text, char props)] (effective, per run), level
T: pre (the table line's existing style name or None), rows [[Cover('^^'|'||') | C]], C: box, paras [CP]
CP (a paragraph in a cell) is a P with kind 'cell' and `tag` (<p style="X"/> or '' when it starts the cell).

render(model, cand) writes F1, F2 or F3; parse(text, cand, truth) reads any of them back to effective properties
(truth supplies what the text does not show: F1's hidden style definitions, F3's hidden direct formatting)."""
import re
from difflib import SequenceMatcher

import inline as I
import vocab as V

CANDS = ('F1', 'F2', 'F2s', 'F3')
TWO = ('F2', 'F2s')                        # F2s: F2 plus a defaults line per section (<pagebreak/> to <pagebreak/>)
CHARV = ('font', 'size', 'color')          # char properties written in braces and spans
FLAGS = V.FLAGS
PKEYS = V.PARA_KEYS + ('fill',) + V.SIDES   # paragraph properties (fill/borders: paragraph shading and borders)
BOXK = ('fill',) + V.SIDES + ('valign',)


class ParseError(Exception):
    def __init__(self, line, msg):
        super().__init__('line %d: %s' % (line, msg))
        self.line, self.msg = line, msg


class Raw:
    def __init__(self, line):
        self.line = line

    def key(self):
        return 'R:' + self.line


class P:
    def __init__(self, kind, prefix, style, inline, para=None, runs=None, tag='', brace=None, level=0):
        self.kind, self.prefix, self.style, self.inline = kind, prefix, style, inline
        self.para, self.runs, self.tag, self.brace, self.level = para or {}, runs or [], tag, brace, level
        self.plain = I.runs_of(inline, spans=False)[0] if inline is not None else ''

    def key(self):
        return 'P:' + self.kind + ':' + re.sub(r'\s+', '', self.plain)


class Cover:
    def __init__(self, m):
        self.m = m


class C:
    def __init__(self, box, paras, brace=None):
        self.box, self.paras, self.brace = box, paras, brace


class T:
    def __init__(self, pre, rows, brace=None):
        self.pre, self.rows, self.brace = pre, rows, brace

    def cells(self):
        for r, row in enumerate(self.rows):
            for c, x in enumerate(row):
                if isinstance(x, C):
                    yield (r, c), x

    def key(self):
        return 'T:' + '|'.join(re.sub(r'\s+', '', p.plain) for _, c in self.cells() for p in c.paras)[:200]


class Model:
    def __init__(self, fmt, front, styles, default, box0, heading, blocks, order=None, palette=None):
        self.fmt, self.front, self.styles, self.default, self.box0 = fmt, front, styles, default, box0
        self.heading, self.blocks, self.order, self.palette = heading, blocks, order or [], palette or {}


# ================================================================ reading lines

HEAD_RE = re.compile(r'^(#{1,6}) (.*)$')
LIST_RE = re.compile(r'^( *)(- |1\. )(.*)$')
DIV_RE = re.compile(r'^<div style="([^"]*)">(.*)</div>$')
EMPTY_RE = re.compile(r'^<p(?: style="([^"]*)")?/>$')
PTAG_RE = re.compile(r'<p(?: style="([^"]*)")?/>')
STYLE_LINE_RE = re.compile(r'^<style name="([^"]*)"(.*?)/>$')
DEFAULTS_RE = re.compile(r'^<defaults(.*?)/>$')
SECK = V.PARA_KEYS + CHARV                  # what a section's defaults line may hold
BRACE_END_RE = re.compile(r'\{([^{}]*)\}$')


def split_row(line, lineno):
    """`| a | b || c |` -> ['a', 'b', None(||), 'c'] with '^^' kept; cells are unstripped-by-one-space."""
    if not (line.startswith('|') and line.endswith('|')):
        raise ParseError(lineno, 'a table row starts and ends with `|`')
    cells = []
    cur = []
    i = 1
    n = len(line)
    while i < n:
        ch = line[i]
        if ch == '\\' and i + 1 < n:
            cur.append(line[i:i + 2])
            i += 2
            continue
        if ch == '<':
            m = I.KEEP_RE.match(line, i) or PTAG_RE.match(line, i)
            if m:
                cur.append(m.group(0))
                i = m.end()
                continue
        if ch == '[':
            c = I.find_span_close(line, i)
            if c is not None:
                cur.append(line[i:c[2]])
                i = c[2]
                continue
        if ch == '{':
            k = line.find('}', i)
            if k > 0:
                cur.append(line[i:k + 1])
                i = k + 1
                continue
        if ch == '|':
            cells.append(''.join(cur))
            cur = []
            i += 1
            continue
        cur.append(ch)
        i += 1
    out = []
    for c in cells:
        if c == '':
            out.append(None)
        elif c.strip() == '^^':
            out.append('^^')
        else:
            if not (c.startswith(' ') and c.endswith(' ')) and c.strip():
                raise ParseError(lineno, 'a table cell is written `| text |`, with one space on each side')
            out.append(c[1:-1] if len(c) >= 2 else '')
    return out


def take_brace_end(s, allow, lineno, what):
    """-> (s without a trailing {attrs}, props or None)."""
    m = BRACE_END_RE.search(s)
    if not m or (m.start() > 0 and s[m.start() - 1] == '\\') or (m.start() > 0 and s[m.start() - 1] == ']'):
        return s, None
    body = m.group(1)
    if body.strip() and not re.match(r'\s*(%s)(=|\s|$)' % '|'.join(sorted(V.KEYS, key=len, reverse=True)), body):
        return s, None
    try:
        props = V.parse_attrs(body)
    except V.VocabError as e:
        raise ParseError(lineno, '%s: %s' % (what, e))
    bad = [k for k in props if k not in allow]
    if bad:
        raise ParseError(lineno, '%s: %s cannot be set here (allowed: %s)' % (what, ', '.join(sorted(bad)),
                                                                             ', '.join(sorted(allow))))
    head = s[:m.start()]
    if head.endswith(' '):
        head = head[:-1]
    return head, props


def lines_to_blocks(text, cand=None):
    """-> (front lines, style lines {name: props}, style order, blocks). cand None = hanji's text (no braces)."""
    lines = text.split('\n')
    if lines and lines[-1] == '':
        lines = lines[:-1]
    if not lines or lines[0] != '---':
        raise ParseError(1, 'the file starts with its front matter between two `---` lines')
    end = lines.index('---', 1)
    front = lines[:end + 1]
    i = end + 1
    styles = {}
    order = []
    blocks = []
    style_spans = {}
    braces = cand in ('F1', 'F2', 'F2s', 'F3')
    para_allow = set(V.PARA_KEYS + ('fill',) + V.SIDES + CHARV + FLAGS)
    if cand == 'F3':
        para_allow = set()
    while i < len(lines):
        ln = lines[i]
        no = i + 1
        m = STYLE_LINE_RE.match(ln)
        if m and cand:
            name = m.group(1)
            try:
                props = V.parse_attrs(m.group(2))
            except V.VocabError as e:
                raise ParseError(no, 'style %s: %s' % (name, e))
            bad = [k for k in props if k in ('style', 'valign')]
            if bad:
                raise ParseError(no, 'a style line holds paragraph and text properties, not %s' % ', '.join(bad))
            if name in styles:
                raise ParseError(no, 'style %r is defined twice' % name)
            styles[name] = props
            order.append(name)
            style_spans[name] = i
            i += 1
            continue
        if ln.startswith('<style') and cand:
            raise ParseError(no, 'a style line is <style name="Name" key=value …/>')
        md = DEFAULTS_RE.match(ln) if cand == 'F2s' else None
        if md:
            try:
                props = V.parse_attrs(md.group(1))
            except V.VocabError as e:
                raise ParseError(no, 'defaults line: %s' % e)
            bad = [k for k in props if k not in SECK]
            if bad:
                raise ParseError(no, 'a defaults line holds paragraph and text properties (%s), not %s'
                                 % (', '.join(SECK), ', '.join(bad)))
            r = Raw(ln)
            r.defaults = props
            r.span = (i, i + 1)
            blocks.append(r)
            i += 1
            continue
        if ln.startswith('<defaults') and cand:
            raise ParseError(no, 'there is no <defaults …/> line in this file' if cand != 'F2s'
                             else 'a defaults line is <defaults key=value …/> on its own line')
        if ln.startswith('|') or (braces and ln.startswith('{') and i + 1 < len(lines)
                                  and lines[i + 1].startswith('|')) or (ln.startswith('{style=') and i + 1 < len(lines)
                                                                       and lines[i + 1].startswith('|')):
            pre = None
            brace = None
            start = i
            if ln.startswith('{'):
                if not ln.endswith('}'):
                    raise ParseError(no, 'a table line is {key=value …} on its own line')
                try:
                    props = V.parse_attrs(ln[1:-1])
                except V.VocabError as e:
                    raise ParseError(no, 'table line: %s' % e)
                pre = props.pop('style', None)
                bad = [k for k in props if k not in BOXK and k not in TLP and k not in FLAGS]
                if bad:
                    raise ParseError(no, 'the table line holds a style, cell properties and paragraph properties, '
                                     'not %s' % ', '.join(bad))
                if props and cand == 'F3':
                    raise ParseError(no, 'this file has no cell formatting; the table line holds only style="Name"')
                brace = props or None
                i += 1
            rows = []
            rbr = []
            first = True
            while i < len(lines) and lines[i].startswith('|'):
                no = i + 1
                if not first and re.fullmatch(r'\|(?:-+\|)+', lines[i]) and len(rows) == 1:
                    i += 1
                    continue
                rowline = lines[i]
                rbrace = None
                mm = re.search(r'\|\s\{([^{}]*)\}$', rowline)
                if mm and cand:
                    try:
                        rbrace = V.parse_attrs(mm.group(1))
                    except V.VocabError as e:
                        raise ParseError(no, 'row formatting: %s' % e)
                    bad = [k for k in rbrace if k not in BOXK]
                    if bad:
                        raise ParseError(no, 'a row\'s {…} after its last `|` holds cell properties (%s), not %s'
                                         % (', '.join(BOXK), ', '.join(bad)))
                    if cand == 'F3':
                        raise ParseError(no, 'this file has no cell formatting')
                    rowline = rowline[:mm.start() + 1]
                cells = split_row(rowline, no)
                row = []
                for c in cells:
                    if c is None:
                        row.append(Cover('||'))
                    elif c == '^^':
                        row.append(Cover('^^'))
                    else:
                        row.append(parse_cell(c, no, cand, para_allow))
                rows.append(row)
                rbr.append(rbrace or {})
                first = False
                i += 1
            widths = {len(r) for r in rows}
            if len(widths) > 1:
                raise ParseError(no, 'every row of a table has one cell per column (rows have %s cells)'
                                 % '/'.join(str(w) for w in sorted(widths)))
            t = T(pre, rows, brace)
            t.rowb = rbr
            t.span = (start, i)
            blocks.append(t)
            continue
        if ln.strip() == '' or ln == '<pagebreak/>' or re.fullmatch(r'(<keep\b[^>]*/>)+', ln):
            r = Raw(ln)
            r.span = (i, i + 1)
            blocks.append(r)
            i += 1
            continue
        p = parse_para_line(ln, no, cand, para_allow)
        p.span = (i, i + 1)
        blocks.append(p)
        i += 1
    lines_to_blocks.style_spans = style_spans
    return front, styles, order, blocks


def parse_para_line(ln, no, cand, allow):
    m = EMPTY_RE.match(ln)
    if m:
        return P('empty', '', m.group(1), '', tag=ln)
    body = ln
    brace = None
    if cand:
        a = set(allow) | {'style'}
        body, brace = take_brace_end(ln, a, no, 'paragraph formatting')
    m = HEAD_RE.match(body)
    if m:
        p = P('heading', m.group(1) + ' ', None, m.group(2), level=len(m.group(1)))
    else:
        m = LIST_RE.match(body)
        if m:
            p = P('list', m.group(1) + m.group(2), None, m.group(3), level=len(m.group(1)) // 2)
        else:
            m = DIV_RE.match(body)
            if m:
                p = P('div', '', m.group(1), m.group(2))
            else:
                p = P('plain', '', None, body)
    if brace and 'style' in brace:
        if p.kind != 'list':
            raise ParseError(no, 'style="Name" in a paragraph\'s {…} is for list items; a paragraph takes its style '
                             'as <div style="Name">…</div>')
        p.style = brace.pop('style')
    p.brace = brace
    try:
        I.runs_of(p.inline, spans=cand in ('F1',) + TWO)
    except V.VocabError as e:
        raise ParseError(no, str(e))
    if cand == 'F3' and re.search(r'\]\{', p.inline):
        raise ParseError(no, 'this file has no run formatting: [text]{…} is not part of it')
    return p


def parse_cell(c, no, cand, allow):
    brace = None
    s = c
    if cand and s.startswith('{'):
        k = s.find('}')
        try:
            brace = V.parse_attrs(s[1:k])
        except V.VocabError as e:
            raise ParseError(no, 'cell formatting: %s' % e)
        bad = [x for x in brace if x not in BOXK]
        if bad:
            raise ParseError(no, 'a cell\'s leading {…} holds cell properties (%s), not %s; paragraph properties go '
                             'in a {…} at the end of the paragraph' % (', '.join(BOXK), ', '.join(bad)))
        if cand == 'F3':
            raise ParseError(no, 'this file has no cell formatting')
        s = s[k + 1:]
        if s.startswith(' '):
            s = s[1:]
    paras = []
    pos = 0
    tag = ''
    style = None
    pieces = []
    for m in PTAG_RE.finditer(s):
        pieces.append((tag, style, s[pos:m.start()]))
        tag, style = m.group(0), m.group(1)
        pos = m.end()
    pieces.append((tag, style, s[pos:]))
    if pieces[0][2] == '' and len(pieces) > 1 and pieces[0][0] == '':
        pieces = pieces[1:]
        if pieces[0][0] == '<p/>':
            pieces[0] = ('', None, pieces[0][2])
    for tag, style, txt in pieces:
        b = None
        if cand and cand != 'F3':
            txt, b = take_brace_end(txt, allow, no, 'paragraph formatting')
        elif cand == 'F3' and BRACE_END_RE.search(txt) and re.search(r'\{\s*[a-z]', txt):
            raise ParseError(no, 'this file has no paragraph formatting in cells')
        p = P('cell', '', style, txt, tag=tag, brace=b)
        try:
            I.runs_of(txt, spans=cand in ('F1',) + TWO)
        except V.VocabError as e:
            raise ParseError(no, str(e))
        paras.append(p)
    return C({}, paras, brace)


# ================================================================ attaching a real file's formatting

def norm(s):
    return re.sub(r'\s+', '', s)


def attach(text, doc, fmt):
    """hanji's text + extract.Doc -> Model with the formatting attached, block by block."""
    front, _, _, blocks = lines_to_blocks(text, None)
    heading = {}
    if fmt == 'docx':
        for n in range(1, 7):
            heading[n] = 'Heading %d' % n if 'Heading %d' % n in doc.styles else ('heading %d' % n)
    else:
        for n in range(1, 7):
            heading[n] = '개요 %d' % n
    box0 = {'fill': 'none', 'border-top': 'none', 'border-right': 'none', 'border-bottom': 'none',
            'border-left': 'none', 'valign': 'middle' if fmt == 'hwpx' else 'top'}
    # sequence alignment over paragraphs and tables
    xs = []
    for kind, x in doc.items:
        if kind == 'p':
            xs.append(('P', norm(x.text), x))
        else:
            xs.append(('T', '|'.join(norm(p.text) for _, c in sorted(x.cells.items()) for p in c.paras)[:200], x))
    ts = []
    for b in blocks:
        if isinstance(b, P):
            ts.append(('P', norm(b.plain), b))
        elif isinstance(b, T):
            ts.append(('T', '|'.join(norm(p.plain) for _, c in b.cells() for p in c.paras)[:200], b))
    a = [k + ':' + s for k, s, _ in xs]
    bb = [k + ':' + s for k, s, _ in ts]
    sm = SequenceMatcher(None, a, bb, autojunk=False)
    pairs = []
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == 'equal':
            pairs += list(zip(range(i1, i2), range(j1, j2)))
        elif tag == 'replace':
            # pair by kind, in order, with a fuzzy text match
            jj = j1
            for i in range(i1, i2):
                for j in range(jj, j2):
                    if xs[i][0] == ts[j][0] and (xs[i][0] == 'T' or similar(xs[i][1], ts[j][1])):
                        pairs.append((i, j))
                        jj = j + 1
                        break
    matched = 0
    for i, j in pairs:
        x = xs[i][2]
        b = ts[j][2]
        if isinstance(b, P):
            fill_para(b, x, doc)
            matched += 1
        else:
            fill_table(b, x, doc)
            matched += 1
    # blocks left without a match take their style's formatting
    for _, _, b in ts:
        if isinstance(b, P) and not b.para:
            fill_default(b, doc, heading)
        elif isinstance(b, T):
            for _, c in b.cells():
                if not c.box:
                    c.box = dict(box0)
                for p in c.paras:
                    if not p.para:
                        fill_default(p, doc, heading)
    styles = {n: {'para': dict(s['para']), 'char': dict(s['char'])} for n, s in doc.styles.items()}
    # the heading styles `#` lines use, from the file
    seen = {}
    for b in blocks:
        if isinstance(b, P) and b.kind == 'heading' and b.style:
            seen.setdefault(b.level, {}).setdefault(b.style, 0)
            seen[b.level][b.style] += 1
    for lvl, d in seen.items():
        heading[lvl] = max(d, key=d.get)
    m = Model(fmt, front, styles, doc.default, box0, heading, blocks, palette=doc.palette)
    for b in blocks:
        if isinstance(b, P) and b.kind == 'heading':
            b.style = heading.get(b.level, b.style)
    for p in all_paras(m):
        normalize_flags(m, p)
    m.stats = {'blocks': len(ts), 'matched': matched}
    used = []
    for p in all_paras(m):
        if p.style and p.style not in used and p.style in styles:
            used.append(p.style)
    m.order = [doc.default] + [u for u in used if u != doc.default]
    return m


def normalize_flags(m, p):
    """A run's bold/italic/underline/strike are what the text can show: its marks, over its style's."""
    if p.kind == 'empty':
        p.runs = [('', dict(p.runs[0][1]) if p.runs else dict(style_eff(m, p.style or m.default)['char']))]
        return
    st = style_eff(m, p.style or m.default)['char']
    plain, mruns, _ = I.runs_of(p.inline, spans=False)
    flat = []
    for t, pr in p.runs:
        flat += [pr] * len(t)
    out = []
    pos = 0
    for t, fl, _ in mruns:
        for ch in t:
            pr = dict(flat[pos] if pos < len(flat) else st)
            for f in FLAGS:
                pr[f] = 'yes' if (f in fl or st.get(f) == 'yes') else 'no'
            if out and out[-1][1] == pr:
                out[-1] = (out[-1][0] + ch, pr)
            else:
                out.append((ch, pr))
            pos += 1
    p.runs = out or [('', {k: st.get(k) for k in V.CHAR_KEYS})]


def similar(x, y):
    if not x or not y:
        return x == y
    return SequenceMatcher(None, x, y, autojunk=False).ratio() > 0.6


def fill_default(p, doc, heading):
    st = p.style or (heading.get(p.level) if p.kind == 'heading' else doc.default)
    if st not in doc.styles:
        st = doc.default
    p.style = st
    p.para = dict(doc.styles[st]['para'])
    p.runs = [(p.plain, dict(doc.styles[st]['char']))]


def fill_para(b, x, doc):
    """The paragraph keeps the style its text names (a plain line: the default; <div>/<p style>: that one); a
    list item and a heading take the file's."""
    if b.kind in ('list', 'heading'):
        b.style = x.style if x.style in doc.styles else doc.default
    elif b.kind == 'plain':
        b.style = doc.default
    else:
        b.style = b.style if b.style in doc.styles else doc.default
    b.para = {k: x.para.get(k, 'none') for k in PKEYS}
    b.runs = map_runs(b.plain, x.runs, doc.styles[b.style]['char'])


def fill_table(b, x, doc):
    for rc, c in b.cells():
        xc = x.cells.get(rc)
        if xc is None:
            c.box = {}
            continue
        c.box = {k: xc.box.get(k, 'none') for k in BOXK}
        if len(xc.paras) == len(c.paras):
            pairs = list(zip(c.paras, xc.paras))
        else:
            pairs = []
            sm = SequenceMatcher(None, [norm(p.plain) for p in c.paras], [norm(p.text) for p in xc.paras],
                                 autojunk=False)
            for tag, i1, i2, j1, j2 in sm.get_opcodes():
                if tag in ('equal', 'replace'):
                    pairs += list(zip(c.paras[i1:i2], xc.paras[j1:j2]))
        for p, xp in pairs:
            fill_para(p, xp, doc)


def map_runs(plain, xruns, base):
    """XML runs -> runs over hanji's plain text (character alignment)."""
    xtext = ''.join(t for t, _ in xruns)
    owner = []
    for k, (t, pr) in enumerate(xruns):
        owner += [k] * len(t)
    out_props = [None] * len(plain)
    sm = SequenceMatcher(None, xtext, plain, autojunk=False)
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == 'equal':
            for d in range(i2 - i1):
                out_props[j1 + d] = xruns[owner[i1 + d]][1]
    last = xruns[0][1] if xruns else base
    for j in range(len(plain)):
        if out_props[j] is None:
            out_props[j] = last
        last = out_props[j]
    runs = []
    for ch, pr in zip(plain, out_props):
        pr = {k: pr.get(k, base.get(k)) for k in V.CHAR_KEYS}
        if runs and runs[-1][1] == pr:
            runs[-1] = (runs[-1][0] + ch, pr)
        else:
            runs.append((ch, pr))
    return runs or [('', dict(base))]


def all_paras(m):
    for b in m.blocks:
        if isinstance(b, P):
            yield b
        elif isinstance(b, T):
            for _, c in b.cells():
                yield from c.paras


# ================================================================ rendering

def lifted(runs):
    """Char properties shared by every run with visible text."""
    vis = [pr for t, pr in runs if t.strip()] or [pr for _, pr in runs]
    if not vis:
        return {}
    out = {}
    for k in V.CHAR_KEYS:
        vals = {pr.get(k) for pr in vis}
        if len(vals) == 1:
            out[k] = vals.pop()
    return out


def style_eff(m, name):
    s = m.styles.get(name) or m.styles[m.default]
    return s


TLP = tuple(k for k in V.PARA_KEYS) + CHARV   # paragraph properties a table line can hold


# ---------------------------------------------------------------- F2s: section defaults

def own_keys(styles, name, default):
    """The properties a style line shows: those that differ from the default style (none for the default)."""
    if name == default or name not in styles:
        return set()
    s, d = styles[name], styles[default]
    return {k for k in s['para'] if s['para'].get(k) != d['para'].get(k)} | \
        {k for k in s['char'] if s['char'].get(k) != d['char'].get(k)}


def layered(styles, default, name, sec, own=None):
    """F2s: the default style, under the section's defaults line, under the style's own properties."""
    d = styles[default]
    out = {'para': dict(d['para']), 'char': dict(d['char'])}
    for k, v in (sec or {}).items():
        (out['para'] if k in PKEYS else out['char'])[k] = v
    st = styles.get(name) or d
    for k in (own_keys(styles, name, default) if own is None else own):
        if k in st['para']:
            out['para'][k] = st['para'][k]
        elif k in st['char']:
            out['char'][k] = st['char'][k]
    return out


def sec_base(m, name, sec):
    return layered(m.styles, m.default, name or m.default, sec)


def section_starts(m):
    """Block index where each section starts: the first block, and the block after each <pagebreak/>."""
    starts = [0]
    for i, b in enumerate(m.blocks):
        if isinstance(b, Raw) and b.line == '<pagebreak/>' and i + 1 < len(m.blocks):
            starts.append(i + 1)
    return starts


def section_paras(m, lo, hi):
    for b in m.blocks[lo:hi]:
        if isinstance(b, P):
            yield b
        elif isinstance(b, T):
            for _, c in b.cells():
                yield from c.paras


SEC_RULE = 'plurality'   # 'majority': more than half; 'plurality': the commonest value, when more have it than the default
SEC_OVER = 'all'   # which paragraphs vote for a section's defaults: 'all' (cells too) or 'body'


def compute_sections(m):
    """Each section's defaults: for each property, the value more than half of the section's paragraphs with text
    have (two or more), among the paragraphs whose style does not set the property itself, when it is not the
    default style's. Stored on the model (m.secd) so an edit keeps them."""
    starts = section_starts(m)
    dflt = m.styles[m.default]
    m.secd = []
    for n, lo in enumerate(starts):
        hi = starts[n + 1] if n + 1 < len(starts) else len(m.blocks)
        ps = [p for p in section_paras(m, lo, hi) if p.plain.strip() and (SEC_OVER == 'all' or p.kind != 'cell')]
        d = {}
        for k in SECK:
            vals = []
            for p in ps:
                if k in own_keys(m.styles, p.style or m.default, m.default):
                    continue
                vals.append(p.para.get(k) if k in PKEYS else lifted(p.runs).get(k))
            ref = (dflt['para'] if k in PKEYS else dflt['char']).get(k)
            v, c = mode([x for x in vals if x != ref])
            if v is not None and c >= 2 and (2 * c > len(vals) if SEC_RULE == 'majority' else c > vals.count(ref)):
                d[k] = v
        m.secd.append(d)
    return m


def assign_sections(m):
    if not hasattr(m, 'secd'):
        compute_sections(m)
    starts = section_starts(m)
    m.sec_at = {lo: n for n, lo in enumerate(starts)}
    for n, lo in enumerate(starts):
        hi = starts[n + 1] if n + 1 < len(starts) else len(m.blocks)
        for p in section_paras(m, lo, hi):
            p.sec = m.secd[n] if n < len(m.secd) else {}


def para_brace(m, p, cand, tlp=None):
    """The paragraph's {…} (dict) under cand, or None. tlp: the table line's paragraph properties (cells)."""
    if p.kind == 'empty' or not p.plain.strip():
        return {'style': p.style} if p.kind == 'list' and p.style != m.default else None
    st = style_eff(m, p.style)
    dflt = style_eff(m, m.default)
    lift = lifted(p.runs)
    out = {}
    explicit = cand == 'F1' and bool(tlp)
    if cand == 'F1':
        base_p, base_c = dict(dflt['para']), dict(dflt['char'])
    elif cand == 'F2s':
        st = sec_base(m, p.style, getattr(p, 'sec', None))
        base_p, base_c = dict(st['para']), dict(st['char'])
    else:
        base_p, base_c = dict(st['para']), dict(st['char'])
    for k, v in (tlp or {}).items():
        (base_p if k in PKEYS else base_c)[k] = v
    if cand != 'F3':
        for k in PKEYS:
            if p.para.get(k) != base_p.get(k):
                out[k] = p.para.get(k)
        for k in CHARV:
            if k in lift and lift[k] != base_c.get(k):
                out[k] = lift[k]
        for f in FLAGS:
            if cand == 'F1' and lift.get(f) == 'yes' and st['char'].get(f) == 'yes' and dflt['char'].get(f) != 'yes':
                out[f] = 'yes'
            if cand in TWO and lift.get(f) == 'no' and st['char'].get(f) == 'yes':
                out[f] = 'no'
    if p.kind == 'list' and p.style != m.default:
        out['style'] = p.style
    if explicit:
        return out or None
    if cand == 'F1' and not out:
        same = all(p.para.get(k) == st['para'].get(k) for k in PKEYS) and \
            all(lift.get(k, st['char'].get(k)) == st['char'].get(k) for k in CHARV)
        return None if same else {}
    if cand == 'F1' and set(out) == {'style'}:
        return out
    return out or None


def ref_char(m, p, cand, br, tlp=None):
    """The text properties a run span is written against."""
    dflt = style_eff(m, m.default)['char']
    if cand == 'F1' and (br is not None or tlp):
        base = dict(dflt)
        for k, v in (tlp or {}).items():
            if k in CHARV:
                base[k] = v
        for f in FLAGS:
            base[f] = (br or {}).get(f, dflt.get(f))
    else:
        if cand == 'F2s':
            base = dict(sec_base(m, p.style, getattr(p, 'sec', None))['char'])
        else:
            base = dict(style_eff(m, p.style)['char'])
        for k, v in (tlp or {}).items():
            if k in CHARV:
                base[k] = v
    for k, v in (br or {}).items():
        if k in CHARV:
            base[k] = v
    return base


def span_segments(p, ref):
    """Runs whose text properties differ from the paragraph's -> [(start, end, attrs)]."""
    segs = []
    pos = 0
    for t, pr in p.runs:
        d = {k: pr.get(k) for k in CHARV if pr.get(k) != ref.get(k)}
        if d and t.strip():
            if segs and segs[-1][1] == pos and segs[-1][3] == d:
                segs[-1] = (segs[-1][0], pos + len(t), V.fmt_attrs(d), d)
            else:
                segs.append((pos, pos + len(t), V.fmt_attrs(d), d))
        pos += len(t)
    # a span never covers leading/trailing spaces
    out = []
    for a, b, s, d in segs:
        txt = p.plain[a:b]
        a2 = a + (len(txt) - len(txt.lstrip()))
        b2 = b - (len(txt) - len(txt.rstrip()))
        if a2 < b2:
            out.append((a2, b2, s))
    return out


def para_inline(m, p, cand, tlp=None):
    if cand == 'F3':
        return p.inline
    br = para_brace(m, p, cand, tlp)
    return I.render_spans(p.inline, span_segments(p, ref_char(m, p, cand, br, tlp)))


def brace_text(d):
    if d is None:
        return ''
    return ' {%s}' % V.fmt_attrs(d)


def render_para(m, p, cand):
    if p.kind == 'empty':
        return p.tag
    br = para_brace(m, p, cand)
    inl = para_inline(m, p, cand)
    if p.kind == 'div':
        return '<div style="%s">%s</div>%s' % (p.style, inl, brace_text(br))
    return p.prefix + inl + brace_text(br)


def para_needs_empty(m, p):
    return True


def mode(vals):
    best, cnt = None, 0
    for v in vals:
        c = vals.count(v)
        if c > cnt or (c == cnt and best is None):
            best, cnt = v, c
    return best, cnt


def table_line_props(m, t, cand):
    """-> (cell properties, cell-paragraph properties) the table line holds: for each property, the value at least
    half of the cells (or of the cells' paragraphs with text) have, when the table has two or more cells."""
    if cand == 'F3':
        return {}, {}
    cells = [c for _, c in t.cells() if c.box]
    if len(cells) < 2:
        return {}, {}
    box = {}
    for k in BOXK:
        v, n = mode([c.box.get(k) for c in cells])
        if v is not None and v != m.box0[k] and 2 * n > len(cells):
            box[k] = v
    paras = [p for c in cells for p in c.paras if p.plain.strip()]
    para = {}
    if len(paras) >= 2:
        dflt = style_eff(m, m.default)
        for k in TLP:
            vals = []
            for p in paras:
                lift = lifted(p.runs)
                eff = p.para.get(k) if k in PKEYS else lift.get(k)
                if cand == 'F1':
                    vals.append(eff)
                else:
                    st = sec_base(m, p.style, getattr(p, 'sec', None)) if cand == 'F2s' else style_eff(m, p.style)
                    ref = st['para'].get(k) if k in PKEYS else st['char'].get(k)
                    vals.append(eff if eff != ref else None)
            v, n = mode(vals)
            ref0 = (dflt['para'] if k in PKEYS else dflt['char']).get(k)
            if v is not None and 2 * n > len(paras) and (cand != 'F1' or v != ref0):
                para[k] = v
    return box, para


def row_line_props(m, row, tl, cand):
    """A row's {…} after its last `|`: for each cell property, the value at least half of the row's cells (two or
    more) have, when it differs from the table line's."""
    if cand == 'F3':
        return {}
    cells = [c for c in row if isinstance(c, C) and c.box]
    if len(cells) < 2:
        return {}
    out = {}
    for k in BOXK:
        v, n = mode([c.box.get(k) for c in cells])
        if v is not None and v != tl.get(k, m.box0[k]) and 2 * n > len(cells):
            out[k] = v
    return out


def render_cell(m, t, c, tl, cand, tlp=None):
    parts = []
    if cand != 'F3':
        d = {}
        for k in BOXK:
            v = c.box.get(k)
            if v is None:
                continue
            ref = tl.get(k, m.box0[k])
            if v != ref:
                d[k] = v
        if d:
            parts.append('{%s}' % V.fmt_attrs(d))
    body = []
    for idx, p in enumerate(c.paras):
        s = p.tag if (idx > 0 or p.tag not in ('', '<p/>')) else ''
        if idx > 0 and not s:
            s = '<p/>'
        br = para_brace(m, p, cand, tlp) if cand != 'F3' else None
        if br is not None and 'style' in br:
            br.pop('style')
        body.append(s + para_inline(m, p, cand, tlp) + brace_text(br))
    txt = ''.join(body)
    if parts:
        txt = parts[0] + (' ' + txt if txt else '')
    return ' %s ' % txt if txt else '  '


def render(m, cand):
    out = list(m.front)
    out.append('')
    names = m.order if cand in ('F2', 'F2s', 'F3') else [m.default]
    if cand == 'F2s':
        assign_sections(m)
    prev_d = {}
    dflt = style_eff(m, m.default)
    for n in names:
        s = style_eff(m, n)
        if n == m.default:
            d = {}
            for k in PKEYS:
                v = s['para'].get(k)
                if v not in (None, 'none', '0pt', 'left'):
                    d[k] = v
            for k in V.CHAR_KEYS:
                v = s['char'].get(k)
                if v not in (None, 'no', ''):
                    d[k] = v
        else:
            d = {k: v for k, v in s['para'].items() if k in PKEYS and v != dflt['para'].get(k)}
            d.update({k: v for k, v in s['char'].items() if v != dflt['char'].get(k)})
        out.append('<style name="%s"%s/>' % (n, (' ' + V.fmt_attrs(d)) if d else ''))
    out.append('')
    for bi, b in enumerate(m.blocks):
        if cand == 'F2s' and bi in m.sec_at:
            d = m.secd[m.sec_at[bi]]
            if d or prev_d:
                out.append('<defaults%s/>' % ((' ' + V.fmt_attrs(d)) if d else ''))
            prev_d = d
        if isinstance(b, Raw):
            out.append(b.line)
        elif isinstance(b, P):
            out.append(render_para(m, b, cand))
        else:
            tl, tlp = table_line_props(m, b, cand)
            head = dict(tl)
            head.update(tlp)
            if b.pre:
                head['style'] = b.pre
            if head:
                out.append('{%s}' % V.fmt_attrs(head))
            for r, row in enumerate(b.rows):
                rl = row_line_props(m, row, tl, cand)
                ref = dict(tl)
                ref.update(rl)
                cells = []
                for x in row:
                    if isinstance(x, Cover):
                        cells.append(None if x.m == '||' else ' ^^ ')
                    else:
                        cells.append(render_cell(m, b, x, ref, cand, tlp))
                line = '|'
                for cc in cells:
                    line += ('' if cc is None else cc) + '|'
                if rl:
                    line += ' {%s}' % V.fmt_attrs(rl)
                out.append(line)
                if r == 0:
                    out.append('|' + '---|' * len(row))
    # collapse the blank line after the style block when the text already starts with one
    txt = '\n'.join(out)
    txt = re.sub(r'\n\n\n+', '\n\n', txt)
    return txt + '\n'


# ================================================================ reading a candidate back to effective formatting

class Eff:
    """Effective formatting of every block of a text, for checks."""

    def __init__(self, blocks, styles):
        self.blocks, self.styles = blocks, styles


def style_defs_from_text(m, lines_styles, cand):
    """Style lines of a text -> {name: {'para','char'}} full effective, default first."""
    if cand == 'F1':
        base = {n: {'para': dict(s['para']), 'char': dict(s['char'])} for n, s in m.styles.items()}
        if m.default in lines_styles:
            d = full_default(lines_styles[m.default])
            base[m.default] = d
        return base
    out = {n: {'para': dict(s['para']), 'char': dict(s['char'])} for n, s in m.styles.items()}
    dflt = full_default(lines_styles.get(m.default, {}))
    out[m.default] = dflt
    for n, props in lines_styles.items():
        if n == m.default:
            continue
        s = {'para': dict(dflt['para']), 'char': dict(dflt['char'])}
        for k, v in props.items():
            (s['para'] if k in PKEYS else s['char'])[k] = v
        out[n] = s
    return out


def full_default(props):
    para = {k: '0pt' for k in ('indent-left', 'indent-right', 'first-line', 'space-before', 'space-after')}
    para.update({'align': 'left', 'line-spacing': '100%', 'fill': 'none'})
    para.update({s: 'none' for s in V.SIDES})
    char = {'font': '', 'size': '10pt', 'color': '#000000', 'bold': 'no', 'italic': 'no', 'underline': 'no',
            'strike': 'no'}
    for k, v in props.items():
        (para if k in PKEYS else char)[k] = v
    return {'para': para, 'char': char}


def effective(text, cand, truth):
    """Reads text under cand -> list of effective blocks: ('R', line) | ('P', kind, style, plain, para, runs,
    inline_without_spans) | ('T', {(r,c): (box, [P tuples])}). truth: the seed Model (hidden formatting)."""
    front, st_lines, order, blocks = lines_to_blocks(text, cand)
    styles = style_defs_from_text(truth, st_lines, cand)
    for n in st_lines:
        if n not in truth.styles and cand not in TWO:
            raise ParseError(1, 'style %r is not in this file; the styles are %s' % (n, ', '.join(truth.order)))
    if cand in TWO and truth.default not in st_lines:
        raise ParseError(1, 'the default style\'s line <style name="%s" …/> is missing' % truth.default)
    hidden = hidden_map(truth, blocks) if cand in ('F1', 'F3') else {}
    out = []
    sec = {}
    eff_para.own = {n: (set(v) if n != truth.default else set()) for n, v in st_lines.items()}
    for bi, b in enumerate(blocks):
        if isinstance(b, Raw):
            if getattr(b, 'defaults', None) is not None:
                sec = b.defaults
            out.append(('R', b.line))
        elif isinstance(b, P):
            out.append(eff_para(b, cand, styles, truth, hidden.get(id(b)), sec=sec))
        else:
            cells = {}
            tl = {k: v for k, v in (b.brace or {}).items() if k in BOXK}
            tlp = {k: v for k, v in (b.brace or {}).items() if k not in BOXK}
            for rc, c in b.cells():
                h = hidden.get(id(c))
                if cand == 'F3':
                    box = dict(h.box) if h is not None else dict(truth.box0)
                else:
                    box = dict(truth.box0)
                    box.update(tl)
                    box.update(b.rowb[rc[0]] if rc[0] < len(b.rowb) else {})
                    box.update(c.brace or {})
                ps = []
                for p in c.paras:
                    ps.append(eff_para(p, cand, styles, truth, hidden.get(id(p)), tlp, sec=sec))
                cells[rc] = (box, ps)
            out.append(('T', cells, b.pre))
    effective.spans = [b.span for b in blocks]
    effective.style_spans = dict(lines_to_blocks.style_spans)
    return out, styles


def eff_para(p, cand, styles, truth, hidden, tlp=None, sec=None):
    if p.kind == 'heading':
        style = truth.heading.get(p.level)
    elif p.kind in ('div', 'empty', 'cell'):
        style = p.style or truth.default
    elif p.kind == 'list':
        style = p.style or truth.default
    else:
        style = truth.default
    if style not in styles:
        if p.style and p.kind in ('div', 'cell', 'list', 'empty'):
            raise ParseError(p.span[0] + 1 if hasattr(p, 'span') else 0,
                             'style %r is not in this file; the styles are %s'
                             % (p.style, ', '.join(truth.order)))
        style = truth.default
    st = styles[style]
    plain, runs, zw = I.runs_of(p.inline, spans=cand in ('F1',) + TWO)
    base_inline = canon_inline(I.strip_spans(p.inline) if cand in ('F1',) + TWO else p.inline)
    if not plain.strip():
        return ('P', p.kind, style, plain, {}, [(plain, {})] if plain else [], base_inline, p.prefix, p.tag)
    if cand == 'F1':
        if p.brace is not None or tlp:
            d = styles[truth.default]
            br = dict(p.brace or {})
            br.pop('style', None)
            para = dict(d['para'])
            char = dict(d['char'])
            for k, v in list((tlp or {}).items()) + list(br.items()):
                (para if k in PKEYS else char)[k] = v
            for f in FLAGS:
                if f not in br and f not in (tlp or {}):
                    char[f] = d['char'].get(f, 'no')
        else:
            if hidden is not None and p.kind in ('list',):
                st = styles.get(hidden.style, st)
            para, char = dict(st['para']), dict(st['char'])
    elif cand in TWO:
        if cand == 'F2s':
            lay = layered(styles, truth.default, style, sec, eff_para.own.get(style, set()))
            para, char = lay['para'], lay['char']
        else:
            para, char = dict(st['para']), dict(st['char'])
        for k, v in list((tlp or {}).items()) + list((p.brace or {}).items()):
            if k == 'style':
                continue
            (para if k in PKEYS else char)[k] = v
    else:
        para, char = dict(st['para']), dict(st['char'])
        if hidden is not None:
            hst = truth.styles.get(hidden.style) or truth.styles[truth.default]
            for k in PKEYS:
                if hidden.para.get(k) != hst['para'].get(k):
                    para[k] = hidden.para.get(k)
            hl = lifted(hidden.runs)
            for k in CHARV:
                if k in hl and hl[k] != hst['char'].get(k):
                    char[k] = hl[k]
    eruns = []
    for t, fl, sp in runs:
        pr = dict(char)
        for f in FLAGS:
            if f in fl:
                pr[f] = 'yes'
        pr.update(sp)
        eruns.append((t, pr))
    if cand == 'F3' and hidden is not None:
        # the hidden run formatting follows the text, as the remainder re-anchors it (character alignment)
        hst = truth.styles.get(hidden.style) or truth.styles[truth.default]
        hl = lifted(hidden.runs)
        hflat = []
        for t, pr in hidden.runs:
            hflat += [pr] * len(t)
        src = [None] * len(plain)
        sm = SequenceMatcher(None, hidden.plain, plain, autojunk=False)
        for tag, i1, i2, j1, j2 in sm.get_opcodes():
            if tag == 'equal':
                for d in range(i2 - i1):
                    src[j1 + d] = i1 + d
            elif tag == 'replace':
                for d in range(j2 - j1):
                    src[j1 + d] = min(i1 + d, i2 - 1)
        last = None
        for k2 in range(len(src)):
            if src[k2] is None:
                src[k2] = last
            last = src[k2]
        flat = []
        for t, pr in eruns:
            flat += [pr] * len(t)
        for i2, pr in enumerate(flat):
            h = src[i2]
            if h is not None and h < len(hflat):
                pr = dict(pr)
                for k in CHARV:
                    hv = hflat[h].get(k)
                    if hv != hl.get(k, hst['char'].get(k)):
                        pr[k] = hv
                flat[i2] = pr
        cur = []
        for ch, pr in zip(plain, flat):
            if cur and cur[-1][1] == pr:
                cur[-1] = (cur[-1][0] + ch, pr)
            else:
                cur.append((ch, pr))
        eruns = cur
    return ('P', p.kind, style, plain, para, merge_runs(eruns), base_inline, p.prefix, p.tag)


def getput_view(effs):
    """Effective blocks with the text properties of white space dropped (a space between two differently formatted
    stretches takes whatever its paragraph has; no candidate shows it): what GetPut compares."""
    def para(e):
        if e[0] != 'P':
            return e
        flat = []
        for t, pr in e[5]:
            for ch in t:
                flat.append((ch, None if not ch.strip() else tuple(sorted(pr.items()))))
        return e[:5] + (flat,) + e[6:]
    out = []
    for e in effs:
        if e[0] == 'T':
            out.append(('T', {rc: (box, [para(p) for p in ps]) for rc, (box, ps) in e[1].items()}, e[2]))
        else:
            out.append(para(e))
    return out


def merge_runs(runs):
    out = []
    for t, pr in runs:
        if out and out[-1][1] == pr:
            out[-1] = (out[-1][0] + t, pr)
        elif t:
            out.append((t, pr))
    return out


def hidden_map(truth, blocks):
    """F3: blocks of an answer -> the seed block (P, or C) whose hidden formatting they keep."""
    tb = [b for b in truth.blocks if not isinstance(b, Raw)]
    ab = [b for b in blocks if not isinstance(b, Raw)]
    sm = SequenceMatcher(None, [x.key() for x in tb], [x.key() for x in ab], autojunk=False)
    out = {}
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag in ('equal', 'replace'):
            for x, y in zip(tb[i1:i2], ab[j1:j2]):
                if type(x) is not type(y):
                    continue
                if isinstance(x, P):
                    out[id(y)] = x
                else:
                    for rc, c in y.cells():
                        xc = dict(x.cells()).get(rc)
                        if xc is not None:
                            out[id(c)] = xc
                            for pa, pb in zip(xc.paras, c.paras):
                                out[id(pb)] = pa
    return out


def canon_inline(s):
    """Inline text with empty mark pairs removed (a mark split at a span edge joins again)."""
    prev = None
    while prev != s:
        prev = s
        s = s.replace('****', '').replace('~~~~', '').replace('</u><u>', '')
    return s.replace('\\[', '[').replace('\\]', ']')


def normalize(m):
    """The seed's truth is what F1 can show: render F1, read it back, and keep that formatting in the model, so
    every candidate starts from the same effective formatting (a span never covers bare spaces, for one)."""
    eff, _ = effective(render(m, 'F1'), 'F1', m)
    blocks = [b for b in m.blocks if not isinstance(b, Raw)]
    eff = [e for e in eff if e[0] != 'R']
    assert len(blocks) == len(eff), (len(blocks), len(eff))
    for b, e in zip(blocks, eff):
        if isinstance(b, P) and e[0] == 'P':
            if e[4]:
                b.para = dict(e[4])
                b.runs = [(t, dict(pr)) for t, pr in e[5]]
        elif isinstance(b, T) and e[0] == 'T':
            for rc, c in b.cells():
                box, ps = e[1][rc]
                c.box = dict(box)
                for p, pe in zip(c.paras, ps):
                    if pe[4]:
                        p.para = dict(pe[4])
                        p.runs = [(t, dict(pr)) for t, pr in pe[5]]
    return m
