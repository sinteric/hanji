"""Round 5: the Presentation model with geometry, and its four candidate syntaxes.

The semantic model is what every candidate reads to and writes from. Geometry is in EMU (914,400 per inch,
12,700 per point, 360,000 per cm), exactly as a pptx stores it:

- deck: {'fm': [(key, value)], 'W', 'H', 'layouts': {name: [(slot, box)]}, 'slides': [slide]}
- slide: {'layout', 'objs': [obj, ...] back to front (z-order), 'notes': [para] or None, 'src': seed index}
- obj:
  - slot  {'t': 'slot', 'slot', 'box': (x, y, w, h) or None (inherited from the layout), 'rot', 'paras'}
  - shape {'t': 'shape', 'id', 'name', 'box', 'rot', 'flip', 'paras'}
  - keep  {'t': 'keep', 'id', 'kind', 'summary', 'src', 'box', 'rot'}
  - line  {'t': 'line', 'id', 'name', 'a': (x, y), 'b': (x, y)}
  - group {'t': 'group', 'id', 'name', 'children': [obj]}   (its box is the union of its children)

A para is one Markdown line ('- 매출', '  - 수도권', '1. 목표', '' for an empty paragraph).

Candidates (fluency/round5/README.md):
- A  canvas-first, points: every object is a line or block with its box, slots included.
- Ap A with percent of the slide (the units probe).
- B  today's slots plus an optional box: a slot shows a box only when it is not the layout's.
- C  A's structure on a 12 x 12 grid of cells (`cells="B1:K2"`).

Standard library only.
"""
import math
import re

PT = 12700
CM = 360000
CANDIDATES = ('A', 'Ap', 'B', 'C')
GRID = 12


def rhu(x):
    """Round half up (display rounding is the same on every platform)."""
    return int(math.floor(x + 0.5))


def fnum(v):
    s = '%.1f' % v
    return s[:-2] if s.endswith('.0') else s


def colname(i):
    return 'ABCDEFGHIJKL'[i]


# ---------------------------------------------------------------- units

class Units:
    """Numbers for one candidate: how an EMU value is shown and how a written number becomes EMU.

    The keep rule (GetPut on rounded numbers): a written number equal to how the stored value is shown keeps the
    stored value exactly; any other number is converted exactly."""
    grid = False
    attr = 'box'

    def __init__(self, W, H):
        self.W, self.H = W, H

    def L(self, axis):
        return self.W if axis == 'x' else self.H


class PtUnits(Units):
    name = 'pt'

    def disp(self, v, axis):
        return rhu(v / PT)

    def emu(self, n, axis):
        return int(round(n * PT))

    def s(self, n):
        return str(n)


class PctUnits(Units):
    name = 'pct'

    def disp(self, v, axis):
        return rhu(v / self.L(axis) * 1000) / 10

    def emu(self, n, axis):
        return int(round(n / 100 * self.L(axis)))

    def s(self, n):
        return fnum(n)


class GridUnits(Units):
    name = 'grid'
    grid = True
    attr = 'cells'

    def __init__(self, W, H):
        super().__init__(W, H)
        self.cw, self.rh = W / GRID, H / GRID

    def lines(self, b):
        x, y, w, h = b
        l, r = rhu(x / self.cw), rhu((x + w) / self.cw)
        t, bt = rhu(y / self.rh), rhu((y + h) / self.rh)
        l, r, t, bt = (max(0, min(GRID, v)) for v in (l, r, t, bt))
        if r <= l:
            l, r = (l, l + 1) if l < GRID else (GRID - 1, GRID)
        if bt <= t:
            t, bt = (t, t + 1) if t < GRID else (GRID - 1, GRID)
        return l, t, r, bt

    def show_box(self, b):
        l, t, r, bt = self.lines(b)
        a, z = '%s%d' % (colname(l), t + 1), '%s%d' % (colname(r - 1), bt)
        return a if a == z else a + ':' + z

    def cell(self, p):
        c = max(0, min(GRID - 1, int(math.floor(p[0] / self.cw))))
        r = max(0, min(GRID - 1, int(math.floor(p[1] / self.rh))))
        return c, r

    def show_pt(self, p):
        c, r = self.cell(p)
        return '%s%d' % (colname(c), r + 1)


def units_for(cand, W, H):
    return {'A': PtUnits, 'B': PtUnits, 'Ap': PctUnits, 'C': GridUnits}[cand](W, H)


AX = ('x', 'y', 'x', 'y')


def show_box(u, b):
    if u.grid:
        return u.show_box(b)
    return ' '.join(u.s(u.disp(v, a)) for v, a in zip(b, AX))


def show_pt(u, p):
    if u.grid:
        return u.show_pt(p)
    return ' '.join(u.s(u.disp(v, a)) for v, a in zip(p, AX))


# ---------------------------------------------------------------- geometry helpers

def union(boxes):
    boxes = list(boxes)
    x0 = min(b[0] for b in boxes)
    y0 = min(b[1] for b in boxes)
    x1 = max(b[0] + b[2] for b in boxes)
    y1 = max(b[1] + b[3] for b in boxes)
    return (x0, y0, x1 - x0, y1 - y0)


def obj_box(o, layouts=None, layout=None):
    """The box an object occupies on the slide (a line: the rectangle its two ends span)."""
    if o['t'] == 'line':
        (ax, ay), (bx, by) = o['a'], o['b']
        return (min(ax, bx), min(ay, by), abs(ax - bx), abs(ay - by))
    if o['t'] == 'group':
        return union(obj_box(c) for c in o['children'])
    if o['t'] == 'slot' and o['box'] is None:
        return dict(layouts[layout])[o['slot']]
    return o['box']


def flat(objs):
    for o in objs:
        yield o
        if o['t'] == 'group':
            yield from flat(o['children'])


def okey(o):
    return ('slot', o['slot']) if o['t'] == 'slot' else (o['t'], o.get('id'))


# ---------------------------------------------------------------- rendering

def q(v):
    assert '"' not in v, v
    return '"%s"' % v


def attrs(pairs):
    return ''.join(' %s=%s' % (k, q(v)) for k, v in pairs if v is not None and v != '')


def geo_attrs(u, o):
    out = []
    if o.get('rot'):
        out.append(('rot', fnum(o['rot'])))
    if o.get('flip'):
        out.append(('flip', o['flip']))
    return out


def front_matter(deck, cand):
    lines = ['---'] + ['%s: %s' % kv for kv in deck['fm']]
    wpt, hpt = rhu(deck['W'] / PT), rhu(deck['H'] / PT)
    lines.append('size: %d x %d pt' % (wpt, hpt))
    if cand == 'C':
        lines.append('grid: %d x %d cells of %s x %s pt' % (GRID, GRID, fnum(wpt / GRID), fnum(hpt / GRID)))
    lines.append('---')
    return lines


def render(deck, cand):
    u = units_for(cand, deck['W'], deck['H'])
    out = front_matter(deck, cand)
    out.append('')
    for i, s in enumerate(deck['slides']):
        if i:
            out += ['', '---', '']
        out.append('layout: %s' % s['layout'])
        for o in s['objs']:
            out += render_obj(o, s, deck, cand, u)
        if s.get('notes') is not None:
            out.append('::notes::')
            out += [p if p else '<p/>' for p in s['notes']]
    return '\n'.join(out) + '\n'


def body(paras):
    return [p if p else '<p/>' for p in paras]


def render_obj(o, s, deck, cand, u):
    t = o['t']
    ga = geo_attrs(u, o)
    if t == 'slot':
        if cand == 'B':
            box = None if o['box'] is None else show_box(u, o['box'])
        else:
            box = show_box(u, o['box'] if o['box'] is not None else dict(deck['layouts'][s['layout']])[o['slot']])
        return ['::%s%s::' % (o['slot'], attrs([(u.attr, box)] + ga))] + body(o['paras'])
    if t == 'shape':
        a = attrs([('id', o.get('id')), ('name', o.get('name')), (u.attr, show_box(u, o['box']))] + ga)
        if cand == 'B':
            if not o['paras']:
                return ['<shape%s/>' % a]
            return ['<shape%s>%s</shape>' % (a, '<p/>'.join(o['paras']))]
        return ['::shape%s::' % a] + body(o['paras'])
    if t == 'keep':
        a = attrs([('id', o.get('id')), ('kind', o['kind']), ('summary', o.get('summary')), ('src', o.get('src')),
                   (u.attr, show_box(u, o['box']))] + ga)
        return ['<keep%s/>' % a]
    if t == 'line':
        a = attrs([('id', o.get('id')), ('name', o.get('name')), ('from', show_pt(u, o['a'])), ('to', show_pt(u, o['b']))])
        return ['<line%s/>' % a]
    if t == 'group':
        a = attrs([('id', o.get('id')), ('name', o.get('name')), (u.attr, show_box(u, obj_box(o)))])
        out = ['<group%s>' % a]
        for c in o['children']:
            out += render_obj(c, s, deck, cand, u)
        return out + ['</group>']
    raise ValueError(t)


# ---------------------------------------------------------------- parsing

SLOT_NAMES = {'title', 'subtitle', 'body', 'left', 'right', 'picture', 'chart', 'table', 'diagram', 'media',
              'clipart', 'date', 'footer', 'number'}
ATTR_RE = re.compile(r'\s*([A-Za-z_:-]+)\s*=\s*"([^"]*)"')
MARKER_RE = re.compile(r'^::\s*([A-Za-z][A-Za-z0-9_-]*)(.*?)::\s*$')
TAG_RE = re.compile(r'^<(keep|line|shape|group)\b(.*?)(/?)>(.*)$')
NUM_RE = re.compile(r'^-?\d+(?:\.\d+)?$')
CELL_RE = re.compile(r'^([A-La-l])(\d{1,2})$')
KEEP_KINDS = {'picture', 'chart', 'table', 'group', 'diagram', 'media', 'object', 'ink'}


class Err(Exception):
    def __init__(self, flag, line, msg):
        super().__init__(msg)
        self.flag, self.line, self.msg = flag, line, msg


def parse_attrs(s, ln):
    out, pos = {}, 0
    s = s.rstrip()
    while pos < len(s):
        m = ATTR_RE.match(s, pos)
        if not m:
            rest = s[pos:].strip()
            if not rest:
                break
            raise Err('bad_attr', ln, 'line %d: expected attributes written name="value", found %r.' % (ln, rest[:30]))
        if m.group(1) in out:
            raise Err('bad_attr', ln, 'line %d: attribute %s is written twice.' % (ln, m.group(1)))
        out[m.group(1)] = m.group(2)
        pos = m.end()
    return out


ALLOWED = {
    'slot': {'box', 'cells', 'rot', 'flip'},
    'shape': {'id', 'name', 'box', 'cells', 'rot', 'flip'},
    'keep': {'id', 'kind', 'summary', 'src', 'box', 'cells', 'rot', 'flip'},
    'line': {'id', 'name', 'from', 'to'},
    'group': {'id', 'name', 'box', 'cells'},
}


def check_allowed(kind, a, u, ln):
    allowed = set(ALLOWED[kind])
    allowed.discard('cells' if not u.grid else 'box')
    for k in a:
        if k not in allowed:
            raise Err('invented_attr', ln, 'line %d: %s has no attribute %s; its attributes are %s.'
                      % (ln, kind, k, ', '.join(sorted(allowed))))


def syntactic(text, cand, u):
    """Phase 1: the file as raw slides of raw objects (attributes still text). Raises Err."""
    lines = text.split('\n')
    if lines and lines[-1] == '':
        lines.pop()
    i = 0
    while i < len(lines) and not lines[i].strip():
        i += 1
    fm = []
    if i < len(lines) and lines[i].strip() == '---':
        j = i + 1
        while j < len(lines) and lines[j].strip() != '---':
            if ':' not in lines[j]:
                raise Err('front_matter', j + 1, 'line %d: front matter lines are key: value.' % (j + 1))
            k, v = lines[j].split(':', 1)
            fm.append((k.strip(), v.strip()))
            j += 1
        if j >= len(lines):
            raise Err('front_matter', i + 1, 'line %d: the front matter is not closed by a --- line.' % (i + 1))
        i = j + 1
    else:
        raise Err('no_front_matter', i + 1, 'line %d: the file must begin with its front matter between --- lines.'
                  % (i + 1))
    slides, cur, stack = [], None, []
    body_target = None   # the list a text line goes to (a slot's, a shape's, or the notes' paras)

    def new_slide(ln):
        return {'layout': None, 'objs': [], 'notes': None, 'line': ln}

    pending = True  # expecting a layout line
    for k in range(i, len(lines)):
        ln = k + 1
        raw = lines[k]
        s = raw.rstrip()
        st = s.strip()
        if st == '---':
            if stack:
                raise Err('unclosed_tag', ln, 'line %d: <group> opened on line %d is not closed by </group> before ---.'
                          % (ln, stack[-1]['line']))
            if cur is None or cur['layout'] is None:
                raise Err('empty_slide', ln, 'line %d: a --- line must follow a slide; a slide starts with its '
                          'layout: line.' % ln)
            slides.append(cur)
            cur, pending, body_target = None, True, None
            continue
        if not st:
            continue
        if pending:
            m = re.match(r'^layout\s*:\s*(.+)$', st)
            if not m:
                raise Err('missing_layout', ln, 'line %d: the first line of a slide is `layout: Name`.' % ln)
            name = m.group(1).strip()
            if len(name) >= 2 and name[0] == name[-1] and name[0] in '"\'':
                name = name[1:-1]
            cur = new_slide(ln)
            cur['layout'] = name
            pending = False
            continue
        if re.match(r'^[a-z_]+\s*:\s', st) and body_target is None:
            raise Err('front_matter', ln, 'line %d: a slide has no key: value lines besides layout:.' % ln)
        m = MARKER_RE.match(st)
        container = stack[-1]['children'] if stack else cur['objs']
        if m:
            name, rest = m.group(1), m.group(2)
            a = parse_attrs(rest, ln)
            if name == 'notes':
                if a:
                    raise Err('invented_attr', ln, 'line %d: ::notes:: takes no attributes.' % ln)
                if stack:
                    raise Err('unclosed_tag', ln, 'line %d: close the <group> before ::notes::.' % ln)
                if cur['notes'] is not None:
                    raise Err('duplicate_slot', ln, 'line %d: a slide has one ::notes::.' % ln)
                cur['notes'] = []
                body_target = cur['notes']
                cur['notes_line'] = ln
                continue
            if cur['notes'] is not None:
                raise Err('after_notes', ln, 'line %d: ::notes:: comes last on a slide.' % ln)
            if name == 'shape':
                if cand == 'B':
                    raise Err('unknown_marker', ln, 'line %d: a shape is one line <shape …>text</shape>, not a '
                              '::shape:: marker.' % ln)
                check_allowed('shape', a, u, ln)
                o = {'t': 'shape', 'a': a, 'paras': [], 'line': ln}
                container.append(o)
                body_target = o['paras']
                continue
            if stack:
                raise Err('slot_in_group', ln, 'line %d: a slot cannot be inside a <group>.' % ln)
            check_allowed('slot', a, u, ln)
            o = {'t': 'slot', 'slot': name, 'a': a, 'paras': [], 'line': ln}
            container.append(o)
            body_target = o['paras']
            continue
        if st.startswith('</group'):
            if st != '</group>':
                raise Err('bad_tag', ln, 'line %d: a group is closed by a line holding only </group>.' % ln)
            if not stack:
                raise Err('bad_tag', ln, 'line %d: </group> without an open <group>.' % ln)
            stack.pop()
            body_target = None
            continue
        m = TAG_RE.match(st)
        if m:
            tag, rest, selfclose, tail = m.groups()
            if tag == 'shape' and cand != 'B':
                raise Err('unknown_tag', ln, 'line %d: a shape is a ::shape …:: marker followed by its text.' % ln)
            if tag == 'shape':
                a = parse_attrs(rest, ln)
                check_allowed('shape', a, u, ln)
                if selfclose:
                    if tail.strip():
                        raise Err('bad_tag', ln, 'line %d: nothing may follow <shape …/> on its line.' % ln)
                    paras = []
                else:
                    if not tail.endswith('</shape>'):
                        raise Err('unclosed_tag', ln, 'line %d: a shape is one line <shape …>text</shape>.' % ln)
                    inner = tail[:-len('</shape>')]
                    if '</shape>' in inner or '<shape' in inner:
                        raise Err('bad_tag', ln, 'line %d: one shape per line.' % ln)
                    paras = inner.split('<p/>')
                    if all(not p.strip() for p in paras):
                        raise Err('empty_shape', ln, 'line %d: a shape without text is written <shape …/>.' % ln)
                container.append({'t': 'shape', 'a': a, 'paras': paras, 'line': ln})
                body_target = None
                continue
            if tag == 'group':
                if selfclose or tail.strip():
                    raise Err('bad_tag', ln, 'line %d: a group is a <group …> line, its objects, then </group>.' % ln)
                a = parse_attrs(rest, ln)
                check_allowed('group', a, u, ln)
                g = {'t': 'group', 'a': a, 'children': [], 'line': ln}
                container.append(g)
                stack.append(g)
                body_target = None
                continue
            if not selfclose or tail.strip():
                raise Err('bad_tag', ln, 'line %d: <%s …/> is one self-closing tag on its own line.' % (ln, tag))
            a = parse_attrs(rest, ln)
            check_allowed(tag, a, u, ln)
            container.append({'t': tag, 'a': a, 'line': ln})
            body_target = None
            continue
        if st.startswith('<') and re.match(r'^<\s*/?\s*[a-zA-Z]', st) and not st.startswith('<p/>') \
                and not st.startswith('<u>') and not st.startswith('<br'):
            raise Err('unknown_tag', ln, 'line %d: unknown tag %r; the tags are <keep/>, <line/>, <group>%s.'
                      % (ln, st[:20], ', <shape>' if cand == 'B' else ''))
        if body_target is None:
            raise Err('text_outside_slot', ln, 'line %d: text must follow a slot marker%s.'
                      % (ln, '' if cand == 'B' else ' or a ::shape:: marker'))
        body_target.append('' if st == '<p/>' else s)
    if pending and not slides:
        raise Err('empty_slide', len(lines), 'the file has no slide.')
    if not pending:
        if stack:
            raise Err('unclosed_tag', stack[-1]['line'], 'line %d: <group> is not closed by </group>.'
                      % stack[-1]['line'])
        slides.append(cur)
    elif slides:
        raise Err('empty_slide', len(lines), 'the file ends with a --- line; remove it.')
    return fm, slides


# ---------------------------------------------------------------- phase 2: slides matched to the seed's

def raw_ids(objs):
    out = set()
    for o in objs:
        if o['t'] != 'slot' and o['a'].get('id'):
            out.add(o['a']['id'])
        if o['t'] == 'group':
            out |= raw_ids(o['children'])
    return out


def seed_ids(objs):
    return {o['id'] for o in flat(objs) if o['t'] != 'slot' and o.get('id')}


def norm_text(s):
    return re.sub(r'\s+', ' ', s.replace('**', '')).strip()


def title_of(objs, raw):
    for o in objs:
        if o['t'] == 'slot' and o['slot'] == 'title':
            return norm_text(' '.join(o['paras']))
    return None


def sig_score(r, s):
    sc = 0
    if r['layout'] == s['layout']:
        sc += 2
    tr, ts = title_of(r['objs'], True), title_of(s['objs'], False)
    if tr is not None and tr == ts:
        sc += 4
    sc += len(raw_ids(r['objs']) & seed_ids(s['objs']))
    if r['notes'] is not None and s.get('notes') is not None and \
            norm_text(' '.join(r['notes'])) == norm_text(' '.join(s['notes'])):
        sc += 1
    return sc


def align(raw_slides, seed_slides):
    n, m = len(raw_slides), len(seed_slides)
    sc = [[sig_score(r, s) for s in seed_slides] for r in raw_slides]
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            best = max(dp[i - 1][j], dp[i][j - 1])
            if sc[i - 1][j - 1] >= 3:
                best = max(best, dp[i - 1][j - 1] + sc[i - 1][j - 1])
            dp[i][j] = best
    out = [None] * n
    i, j = n, m
    while i and j:
        if sc[i - 1][j - 1] >= 3 and dp[i][j] == dp[i - 1][j - 1] + sc[i - 1][j - 1]:
            out[i - 1] = j - 1
            i, j = i - 1, j - 1
        elif dp[i][j] == dp[i - 1][j]:
            i -= 1
        else:
            j -= 1
    return out


# ---------------------------------------------------------------- phase 3: geometry resolved against the seed

def nums(s, count, ln, what):
    parts = s.split()
    if len(parts) != count or not all(NUM_RE.match(p) for p in parts):
        raise Err('bad_box', ln, 'line %d: %s is %d numbers separated by spaces, e.g. %s.'
                  % (ln, what, count, '"36 22 648 90"' if count == 4 else '"84 144"'))
    return [float(p) for p in parts]


def resolve_box(u, s, ref, ln, kind_word='box'):
    """A written box (text) -> EMU, keeping every number (grid: every edge) that is shown unchanged."""
    if u.grid:
        l, t, r, b = parse_range(s, ln)
        if ref is not None:
            dl, dt, dr, db = u.lines(ref)
            x, y, w, h = ref
            L = x + (l - dl) * u.cw
            R = x + w + (r - dr) * u.cw
            T = y + (t - dt) * u.rh
            B = y + h + (b - db) * u.rh
        else:
            L, R, T, B = l * u.cw, r * u.cw, t * u.rh, b * u.rh
        L, R, T, B = (int(round(v)) for v in (L, R, T, B))
        if R < L or B < T:
            raise Err('bad_box', ln, 'line %d: the cells run from top-left to bottom-right.' % ln)
        return (L, T, R - L, B - T)
    ns = nums(s, 4, ln, kind_word)
    out = []
    for i, (n, a) in enumerate(zip(ns, AX)):
        if ref is not None and abs(n - u.disp(ref[i], a)) < 1e-9:
            out.append(ref[i])
        else:
            out.append(u.emu(n, a))
    if out[2] < 0 or out[3] < 0:
        raise Err('bad_box', ln, 'line %d: width and height are never negative.' % ln)
    return tuple(out)


def parse_range(s, ln):
    s = s.strip()
    parts = s.split(':')
    if len(parts) not in (1, 2):
        raise Err('bad_box', ln, 'line %d: cells is a range of cells like "B1:K2" (columns A–L, rows 1–12).' % ln)
    cells = []
    for p in parts:
        m = CELL_RE.match(p.strip())
        if not m or not 1 <= int(m.group(2)) <= GRID:
            raise Err('bad_box', ln, 'line %d: cells is a range of cells like "B1:K2" (columns A–L, rows 1–12).' % ln)
        cells.append(('ABCDEFGHIJKL'.index(m.group(1).upper()), int(m.group(2)) - 1))
    (c0, r0), (c1, r1) = cells[0], cells[-1]
    if c1 < c0 or r1 < r0:
        raise Err('bad_box', ln, 'line %d: a range runs from its top-left cell to its bottom-right cell.' % ln)
    return c0, r0, c1 + 1, r1 + 1


def resolve_pt(u, s, ref, ln):
    if u.grid:
        m = CELL_RE.match(s.strip())
        if not m or not 1 <= int(m.group(2)) <= GRID:
            raise Err('bad_box', ln, 'line %d: a line end is one cell like "B4".' % ln)
        c, r = 'ABCDEFGHIJKL'.index(m.group(1).upper()), int(m.group(2)) - 1
        if ref is not None:
            dc, dr = u.cell(ref)
            return (int(round(ref[0] + (c - dc) * u.cw)), int(round(ref[1] + (r - dr) * u.rh)))
        return (int(round((c + 0.5) * u.cw)), int(round((r + 0.5) * u.rh)))
    ns = nums(s, 2, ln, 'a line end')
    out = []
    for i, (n, a) in enumerate(zip(ns, ('x', 'y'))):
        if ref is not None and abs(n - u.disp(ref[i], a)) < 1e-9:
            out.append(ref[i])
        else:
            out.append(u.emu(n, a))
    return tuple(out)


def rot_of(a, ln):
    r = a.get('rot')
    if r is None:
        return 0
    if not NUM_RE.match(r.strip()):
        raise Err('bad_attr', ln, 'line %d: rot is degrees clockwise, e.g. rot="15".' % ln)
    return float(r)


def flip_of(a, ln):
    f = a.get('flip')
    if f is None:
        return None
    if f not in ('h', 'v', 'hv'):
        raise Err('bad_attr', ln, 'line %d: flip is "h", "v" or "hv".' % ln)
    return f


class Ctx:
    """What a file is checked against: the slide size, layouts, and the seed deck (None for a new file)."""

    def __init__(self, W, H, layouts, seed=None, fm=None):
        self.W, self.H, self.layouts, self.seed, self.fm = W, H, layouts, seed, fm


def parse(text, cand, ctx):
    """Returns (deck, errors, notes). errors: [(flag, message)], empty when valid."""
    u = units_for(cand, ctx.W, ctx.H)
    try:
        fm, raw = syntactic(text, cand, u)
    except Err as e:
        return None, [(e.flag, e.msg)], []
    errors, notes = [], []
    seed_slides = ctx.seed['slides'] if ctx.seed else []
    amap = align(raw, seed_slides) if seed_slides else [None] * len(raw)
    slides = []
    for si, r in enumerate(raw):
        try:
            slides.append(resolve_slide(r, amap[si], seed_slides, ctx, cand, u, notes))
        except Err as e:
            errors.append((e.flag, e.msg))
    if errors:
        return None, errors[:1], notes
    deck = {'fm': [kv for kv in fm if kv[0] not in ('size', 'grid')], 'W': ctx.W, 'H': ctx.H,
            'layouts': ctx.layouts, 'slides': slides}
    return deck, [], notes


def resolve_slide(r, src, seed_slides, ctx, cand, u, notes):
    ln = r['line']
    if r['layout'] not in ctx.layouts:
        raise Err('unknown_layout', ln, 'line %d: layout %r is not one of this file\'s layouts: %s.'
                  % (ln, r['layout'], ', '.join(ctx.layouts)))
    lay = dict(ctx.layouts[r['layout']])
    seed = seed_slides[src] if src is not None else None
    seed_objs = {okey(o): o for o in flat(seed['objs'])} if seed else {}
    seen = set()
    objs = [resolve_obj(o, r, lay, seed_objs, seen, ctx, cand, u, notes) for o in r['objs']]
    notes_paras = None
    if r['notes'] is not None:
        notes_paras = strip_trailing(r['notes'])
        if not notes_paras:
            raise Err('empty_slot', r['notes_line'], 'line %d: ::notes:: without text; delete the marker to have '
                      'no notes.' % r['notes_line'])
    return {'layout': r['layout'], 'objs': objs, 'notes': notes_paras, 'src': src}


def strip_trailing(paras):
    ps = list(paras)
    while ps and ps[-1] == '':
        ps.pop()
    return ps


def resolve_obj(o, r, lay, seed_objs, seen, ctx, cand, u, notes):
    ln = o['line']
    a = o['a']
    t = o['t']
    geo = u.attr
    if t == 'slot':
        name = o['slot']
        if name not in lay:
            raise Err('unknown_slot', ln, 'line %d: ::%s:: is not a slot of layout %r; its slots are %s.'
                      % (ln, name, r['layout'], ', '.join('::%s::' % s for s in lay) or 'none'))
        if ('slot', name) in seen:
            raise Err('duplicate_slot', ln, 'line %d: ::%s:: appears twice on this slide.' % (ln, name))
        seen.add(('slot', name))
        paras = strip_trailing(o['paras'])
        if not paras:
            raise Err('empty_slot', ln, 'line %d: ::%s:: has no text; leave an unfilled slot out.' % (ln, name))
        sref = seed_objs.get(('slot', name))
        ref_box = sref['box'] if sref is not None and sref['box'] is not None else None
        box = None
        if geo in a:
            base = ref_box if ref_box is not None else lay[name]
            box = resolve_box(u, a[geo], base, ln)
            if ref_box is None and box == tuple(lay[name]):
                box = None
        elif ref_box is not None and cand != 'B':
            box = ref_box   # A, Ap, C: a slot written without a box keeps the one it had
        return {'t': 'slot', 'slot': name, 'box': box, 'rot': rot_of(a, ln), 'paras': paras}
    oid = a.get('id')
    sref = None
    if oid is not None:
        sref = seed_objs.get((t, oid))
        if sref is None:
            raise Err('unknown_id', ln, 'line %d: this slide has no %s with id "%s". A new object has no id; ids are '
                      'given when the file is saved.' % (ln, t, oid))
        if (t, oid) in seen:
            raise Err('duplicate_id', ln, 'line %d: id "%s" appears twice on this slide.' % (ln, oid))
        seen.add((t, oid))
    if t == 'shape':
        if geo not in a:
            if sref is None:
                raise Err('missing_box', ln, 'line %d: a new shape needs its %s.' % (ln, geo))
            box = sref['box']
        else:
            box = resolve_box(u, a[geo], sref['box'] if sref else None, ln)
        paras = strip_trailing(o['paras'])
        name = a.get('name', sref['name'] if sref else None)
        return {'t': 'shape', 'id': oid, 'name': name, 'box': box, 'rot': rot_of(a, ln), 'flip': flip_of(a, ln),
                'paras': paras}
    if t == 'keep':
        if sref is not None:
            for k in ('kind', 'summary'):
                if a.get(k) != sref.get(k):
                    raise Err('keep_altered', ln, 'line %d: a <keep/>\'s %s is never changed (it is %s).'
                              % (ln, k, q(sref.get(k) or '')))
            if 'src' in a:
                raise Err('keep_altered', ln, 'line %d: src is only for a new picture.' % ln)
        else:
            if a.get('kind') != 'picture' or not a.get('src'):
                raise Err('bad_new_keep', ln, 'line %d: the only new <keep/> is a picture from a file: '
                          '<keep kind="picture" src="file" %s="…"/>.' % (ln, geo))
            if 'summary' in a:
                raise Err('bad_new_keep', ln, 'line %d: a new picture has no summary; it is given when saved.' % ln)
        if geo not in a:
            if sref is None:
                raise Err('missing_box', ln, 'line %d: a new picture needs its %s.' % (ln, geo))
            box = sref['box']
        else:
            box = resolve_box(u, a[geo], sref['box'] if sref else None, ln)
        return {'t': 'keep', 'id': oid, 'kind': a.get('kind'), 'summary': a.get('summary'), 'src': a.get('src'),
                'box': box, 'rot': rot_of(a, ln)}
    if t == 'line':
        for k in ('from', 'to'):
            if k not in a:
                raise Err('missing_attr', ln, 'line %d: a line has from and to.' % ln)
        pa = resolve_pt(u, a['from'], sref['a'] if sref else None, ln)
        pb = resolve_pt(u, a['to'], sref['b'] if sref else None, ln)
        return {'t': 'line', 'id': oid, 'name': a.get('name', sref['name'] if sref else None), 'a': pa, 'b': pb}
    if t == 'group':
        if sref is None:
            raise Err('bad_new_group', ln, 'line %d: groups are never created here.' % ln)
        children = [resolve_obj(c, r, lay, seed_objs, seen, ctx, cand, u, notes) for c in o['children']]
        if not children:
            raise Err('empty_group', ln, 'line %d: a group without objects; delete the group\'s lines instead.' % ln)
        old_u = obj_box(sref)
        same_kids = [strip_obj(c) for c in children] == [strip_obj(c) for c in sref['children']]
        if geo in a:
            want = resolve_box(u, a[geo], old_u, ln)
            if want != old_u:
                if same_kids:
                    children = [transform(c, old_u, want) for c in children]
                else:
                    new_u = obj_box({'t': 'group', 'children': children})
                    tol = max(u.cw, u.rh) if u.grid else 2 * PT
                    if any(abs(p - q) > tol for p, q in zip(resolve_box(u, a[geo], new_u, ln), new_u)):
                        raise Err('group_box_conflict', ln, 'line %d: change the group\'s %s or its objects\' %s, '
                                  'not both: the objects give %s.' % (ln, geo, geo, q(show_box(u, new_u))))
        return {'t': 'group', 'id': oid, 'name': a.get('name', sref['name']), 'children': children}
    raise Err('unknown_tag', ln, 'line %d: unknown object.' % ln)


def strip_obj(o):
    return {k: v for k, v in o.items() if k != 'line'}


def transform(o, old, new):
    ox, oy, ow, oh = old
    nx, ny, nw, nh = new
    sx = nw / ow if ow else 1
    sy = nh / oh if oh else 1

    def P(p):
        return (int(round(nx + (p[0] - ox) * sx)), int(round(ny + (p[1] - oy) * sy)))
    o = dict(o)
    if o['t'] == 'line':
        o['a'], o['b'] = P(o['a']), P(o['b'])
    elif o['t'] == 'group':
        o['children'] = [transform(c, old, new) for c in o['children']]
    else:
        x, y, w, h = o['box']
        (x2, y2) = P((x, y))
        o['box'] = (x2, y2, int(round(w * sx)), int(round(h * sy)))
    return o


# ---------------------------------------------------------------- comparison

def canon_obj(o):
    t = o['t']
    if t == 'slot':
        return ('slot', o['slot'], o['box'], o.get('rot') or 0, tuple(norm_para(p) for p in o['paras']))
    if t == 'shape':
        return ('shape', o.get('id'), o.get('name'), o['box'], o.get('rot') or 0, o.get('flip'),
                tuple(norm_para(p) for p in o['paras']))
    if t == 'keep':
        return ('keep', o.get('id'), o['kind'], o.get('summary'), o.get('src'), o['box'], o.get('rot') or 0)
    if t == 'line':
        return ('line', o.get('id'), o.get('name'), o['a'], o['b'])
    if t == 'group':
        return ('group', o.get('id'), o.get('name'), tuple(canon_obj(c) for c in o['children']))
    raise ValueError(t)


def norm_para(p):
    s = p.rstrip()
    m = re.match(r'^(\s*)([*+])\s', s)
    if m:
        s = m.group(1) + '-' + s[m.end() - 1:]
    lead = len(s) - len(s.lstrip(' '))
    return ' ' * lead + norm_text(s)


def canon_slide(s):
    return (s['layout'], tuple(canon_obj(o) for o in s['objs']),
            None if s.get('notes') is None else tuple(norm_para(p) for p in s['notes']))


def canon_deck(d):
    return tuple(canon_slide(s) for s in d['slides'])
