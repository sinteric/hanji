#!/usr/bin/env python3
"""Round 6 part D (Spreadsheets): F1 range lines, written by the `format` range operation. A short trial.

The text is today's text of a corpus workbook with the formatting shown as grid.py's F1 lines: one
`<format range="B4:C4" …/>` line per rectangle of cells whose effective formatting differs from the workbook's default
(Normal), plus one `<format default …/>` line at the top. Edits are range operations, not text edits:

    {"op": "format", "range": "Sheet!A1:D1", "set": {"fill": "#D9D9D9", "bold": true}}

`set` gives each cell of the range the properties written and leaves the others; `border` sets the four sides of
every cell, `outline` the outer edges of the range. A width snaps to the nearest Excel border (hair 0.25pt, thin
0.75pt, medium 1.5pt, thick 2.25pt). Theme colours stay names (`accent2+80%`).

Usage: python3 xlsx_kit.py build | score <unit> <answers.json> | selftest"""
import copy
import json
import os
import re
import sys
import xml.etree.ElementTree as ET
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)

import grid as G  # noqa: E402
import vocab as V  # noqa: E402

CORPUS = os.path.join(ROOT, 'crates', 'hanji-xlsx', 'corpus')
SEEDS = {1: 'simple-monthly-budget.xlsx', 2: 'korean-sales-lo.xlsx', 3: 'Tables.xlsx'}
KEYS = ('fill', 'border-top', 'border-right', 'border-bottom', 'border-left', 'align', 'valign', 'indent', 'font',
        'size', 'color', 'bold', 'italic', 'underline', 'strike')
ALIGN_X = ('general',) + V.ALIGN
SNAP = {'solid': (0.75, 1.5, 2.25), 'dashed': (0.75, 1.5), 'dotted': (0.25, 0.75), 'double': (2.25,),
        'dash-dot': (0.75, 1.5), 'dash-dot-dot': (0.75, 1.5)}


class OpError(Exception):
    pass


# ================================================================ the workbook model

class Book:
    def __init__(self, rep):
        self.rep = rep
        self.path = os.path.join(CORPUS, SEEDS[rep])
        z = zipfile.ZipFile(self.path)
        self.st = G.Styles(z.read('xl/styles.xml'))
        self.base = full(self.st.style_xfs[0] if self.st.style_xfs else self.st.xfs[0])
        wb = ET.fromstring(z.read('xl/workbook.xml'))
        rels = ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))
        rid = {r.get('Id'): r.get('Target') for r in rels}
        self.sheets = []
        self.cells = {}
        for s in wb.find(G.S + 'sheets'):
            t = rid[s.get(G.R + 'id')]
            t = t[1:] if t.startswith('/') else 'xl/' + t
            name = s.get('name')
            self.sheets.append(name)
            cs = {}
            if t in z.namelist():
                for c in ET.fromstring(z.read(t)).iter(G.S + 'c'):
                    k = int(c.get('s', '0'))
                    if k:
                        cs[G.ref_rc(c.get('r'))] = full(self.st.xfs[k] if k < len(self.st.xfs) else {}, self.base)
            self.cells[name] = cs
        th = [n for n in z.namelist() if n.startswith('xl/theme/')]
        x = z.read(th[0]).decode('utf-8') if th else ''
        self.palette = {}
        for k, name in (('dk1', 'tx1'), ('lt1', 'bg1'), ('dk2', 'tx2'), ('lt2', 'bg2'), ('accent1', 'accent1'),
                        ('accent2', 'accent2'), ('accent3', 'accent3'), ('accent4', 'accent4'),
                        ('accent5', 'accent5'), ('accent6', 'accent6'), ('hlink', 'hlink')):
            m = re.search(r'<a:%s>.*?(?:val|lastClr)="([0-9A-Fa-f]{6})"' % k, x, re.S)
            if m:
                self.palette[name] = '#' + m.group(1).upper()

    def get(self, sheet, rc):
        return self.cells[sheet].get(rc, self.base)

    def text(self):
        s = SEEDS[self.rep]
        today = open(os.path.join(HERE, 'data', 'today', s + '.txt'), encoding='utf-8').read()
        return G.render(self.path, today, 'F1')


def full(p, base=None):
    out = dict(base or {})
    for k in KEYS:
        if k in p:
            out[k] = p[k]
        elif base is None:
            out[k] = G.DEFAULT_XL.get(k, '')
    return out


# ================================================================ ranges and the format operation

def parse_range(book, s):
    if not isinstance(s, str):
        raise OpError('range: a string such as "Sheet!A1:D4"')
    sheet = None
    if '!' in s:
        sheet, s = s.rsplit('!', 1)
        sheet = sheet.strip()
        if sheet.startswith("'") and sheet.endswith("'"):
            sheet = sheet[1:-1].replace("''", "'")
    elif len(book.sheets) == 1:
        sheet = book.sheets[0]
    else:
        raise OpError('range %r: name the sheet, as "Sheet!A1:D4" (the sheets are %s)' % (s, ', '.join(book.sheets)))
    if sheet not in book.sheets:
        raise OpError('range: there is no sheet %r (the sheets are %s)' % (sheet, ', '.join(book.sheets)))
    m = re.fullmatch(r'\$?([A-Z]{1,3})\$?(\d+)(?::\$?([A-Z]{1,3})\$?(\d+))?', s.strip())
    if not m:
        raise OpError('range %r: write cells as A1 or A1:D4' % s)
    r1, c1 = G.ref_rc(m.group(1) + m.group(2))
    r2, c2 = G.ref_rc(m.group(3) + m.group(4)) if m.group(3) else (r1, c1)
    if r2 < r1 or c2 < c1:
        raise OpError('range %r: the first cell is the top-left one' % s)
    return sheet, (min(r1, r2), min(c1, c2), max(r1, r2), max(c1, c2))


def snap_border(b):
    if b == 'none':
        return b
    w, st, col = V.border_parts(b)
    if st not in SNAP:
        raise OpError('border style %r cannot be written in a workbook (write solid, dashed, dotted or double)' % st)
    w2 = min(SNAP[st], key=lambda x: abs(x - w))
    return '%s %s %s' % (V.pt(w2), st, col)


def parse_set(key, v):
    if key in V.FLAGS:
        if v in (True, 'yes', 'true', 'on'):
            return 'yes'
        if v in (False, 'no', 'false', 'off'):
            return 'no'
        raise OpError('%s: true or false, got %r' % (key, v))
    if key == 'align':
        if v not in ALIGN_X:
            raise OpError('align: one of %s, got %r' % (', '.join(ALIGN_X), v))
        return v
    if key == 'indent' and isinstance(v, int):
        v = str(v)
    if key in ('size',) and isinstance(v, (int, float)):
        v = '%spt' % v
    if not isinstance(v, str):
        raise OpError('%s: a string value, got %r' % (key, v))
    try:
        out = V.parse_value(key, v)
    except V.VocabError as e:
        raise OpError(str(e))
    if key == 'fill' and out in V.FILL_TOKENS:
        raise OpError('fill=%s is shown but cannot be written; write a colour or none' % out)
    if key in V.SIDES:
        out = snap_border(out)
    return out


def apply_ops(book, ops):
    """-> new cells {sheet: {rc: props}}; raises OpError."""
    if not isinstance(ops, list) or not ops:
        raise OpError('an edit answer is {"task_id", "ops": [{"op": "format", "range": …, "set": {…}}, …]}')
    cells = copy.deepcopy(book.cells)
    for n, op in enumerate(ops):
        try:
            if not isinstance(op, dict) or op.get('op') != 'format':
                raise OpError('the only operation for formatting is {"op": "format", "range": …, "set": {…}}')
            extra = set(op) - {'op', 'range', 'set'}
            if extra:
                raise OpError('unknown field %s (an op has op, range and set)' % ', '.join(sorted(extra)))
            sheet, (r1, c1, r2, c2) = parse_range(book, op.get('range'))
            st = op.get('set')
            if not isinstance(st, dict) or not st:
                raise OpError('set: an object of properties, such as {"fill": "#D9D9D9"}')
            sets = {}
            outline = None
            for k, v in st.items():
                if k == 'outline':
                    outline = parse_set('border', v)
                    outline = snap_border(outline)
                elif k == 'border':
                    b = snap_border(parse_set('border', v))
                    for s in V.SIDES:
                        sets[s] = b
                elif k in KEYS:
                    sets[k] = parse_set(k, v)
                else:
                    raise OpError('unknown property %r; the properties are %s, border, outline'
                                  % (k, ', '.join(KEYS)))
            for r in range(r1, r2 + 1):
                for c in range(c1, c2 + 1):
                    p = dict(cells[sheet].get((r, c), book.base))
                    p.update(sets)
                    if outline is not None:
                        if r == r1:
                            p['border-top'] = outline
                        if r == r2:
                            p['border-bottom'] = outline
                        if c == c1:
                            p['border-left'] = outline
                        if c == c2:
                            p['border-right'] = outline
                    cells[sheet][(r, c)] = p
        except OpError as e:
            raise OpError('op %d: %s' % (n + 1, e))
    return cells


def cells_of(book, spec):
    """'Sheet!A1:B2' -> [(sheet, rc)]"""
    sheet, (r1, c1, r2, c2) = parse_range(book, spec)
    return [(sheet, (r, c)) for r in range(r1, r2 + 1) for c in range(c1, c2 + 1)]


# ================================================================ tasks

def is_grey(c):
    return c is not None and c.upper() in ('#D9D9D9', '#D8D8D8', '#DDDDDD', '#D3D3D3', '#E0E0E0', '#CCCCCC', '#C0C0C0',
                                           '#BFBFBF', '#DCDCDC', '#F2F2F2', '#E7E6E6', '#D0CECE')


def is_navy(c):
    if not c or not c.startswith('#'):
        return False
    h, l, s = V.hls(c)
    return 200 <= h <= 250 and l <= 0.35 and s >= 0.3


def eq(v):
    return lambda x: x == v


def border_is(width, style, colpred):
    def f(b):
        p = V.border_parts(b)
        return p is not None and abs(p[0] - width) < 0.05 and p[1] == style and colpred(p[2])
    return f


def white(c):
    return c is not None and (c.upper() == '#FFFFFF' or c == 'bg1')


class Task:
    def __init__(self, tid, kind, text, **kw):
        self.id, self.kind, self.text = tid, kind, text
        self.__dict__.update(kw)


def cell_props(book, spec, *keys):
    sheet, rc = cells_of(book, spec)[0]
    p = book.get(sheet, rc)
    return {k: p[k] for k in keys}


def uniform(ranges, change):
    """targets: every cell of the ranges gets `change` ({key: pred})."""
    return lambda b: {x: change for r in ranges for x in cells_of(b, r)}


def outline_of(rng, pred):
    def f(b):
        out = {}
        sheet, (r1, c1, r2, c2) = parse_range(b, rng)
        for r in range(r1, r2 + 1):
            for c in range(c1, c2 + 1):
                ch = {}
                if r == r1:
                    ch['border-top'] = pred
                if r == r2:
                    ch['border-bottom'] = pred
                if c == c1:
                    ch['border-left'] = pred
                if c == c2:
                    ch['border-right'] = pred
                if ch:
                    out[(sheet, (r, c))] = ch
        return out
    return f


def tasks_1():
    S = 'Simple Monthly Budget'
    t = []
    t.append(Task('xb-q1', 'read', 'What is the fill of cell F4? Answer `ANSWER: fill=<colour>`, the colour as the file '
                  'writes it.', expect=lambda b: {'type': 'props', 'value': cell_props(b, S + '!F4', 'fill')}))
    t.append(Task('xb-q2', 'read', 'What are the font, size and colour of the Amount header cell C4? Answer `ANSWER: '
                  'font=<name> size=<pt> color=<colour>`.',
                  expect=lambda b: {'type': 'props', 'value': cell_props(b, S + '!C4', 'font', 'size', 'color')}))
    t.append(Task('xb-q3', 'read', 'Which cells of the sheet have an indent? Answer `ANSWER: <cell>; <cell>` (cells or '
                  'ranges).', expect=lambda b: {'type': 'cellset', 'value': sorted(
                      '%s%d' % (G.col_letters(c), r + 1) for (r, c), p in b.cells[S].items() if p['indent'] != '0')}))
    t.append(Task('xb-e1', 'edit', 'Fill the header cells of the expenses table (B10:C10) with the theme colour accent2, '
                  '80% lighter. Keep it a theme colour, not a hex value.',
                  targets=uniform([S + '!B10:C10'], {'fill': eq('accent2+80%')}),
                  gold=[{'op': 'format', 'range': S + '!B10:C10', 'set': {'fill': 'accent2+80%'}}]))
    t.append(Task('xb-e2', 'edit', 'Put a 2pt navy outline around the income table (B4:C7): the outer edges only.',
                  targets=outline_of(S + '!B4:C7', border_is(2.25, 'solid', is_navy)),
                  gold=[{'op': 'format', 'range': S + '!B4:C7', 'set': {'outline': '2pt solid #1F3864'}}]))
    t.append(Task('xb-e3', 'edit', 'Make the balance figure in H4 bold, in the theme colour accent2.',
                  targets=uniform([S + '!H4'], {'bold': eq('yes'), 'color': eq('accent2')}),
                  gold=[{'op': 'format', 'range': S + '!H4', 'set': {'bold': True, 'color': 'accent2'}}]))
    t.append(Task('xb-e4', 'edit', 'Indent the item names of the expenses table (B11:B23) by one level.',
                  targets=uniform([S + '!B11:B23'], {'indent': eq('1')}),
                  gold=[{'op': 'format', 'range': S + '!B11:B23', 'set': {'indent': 1}}]))
    t.append(Task('xb-e5', 'edit', 'Remove the fill of the balance box E4:G5 (no fill); keep its borders.',
                  targets=uniform([S + '!E4:G5'], {'fill': eq('none')}),
                  gold=[{'op': 'format', 'range': S + '!E4:G5', 'set': {'fill': 'none'}}]))
    t.append(Task('xb-e6', 'refuse', 'Give the title cell A1 a gradient fill from accent1 to accent2.'))
    return t


def tasks_2():
    t = []
    t.append(Task('ks-q1', 'read', 'In sheet 매출, what are the font and the size of the title A1? Answer `ANSWER: '
                  'font=<name> size=<pt>`.', expect=lambda b: {'type': 'props', 'value': cell_props(b, '매출!A1', 'font',
                                                                                                    'size')}))
    t.append(Task('ks-q2', 'read', 'In sheet 매출, what is the horizontal alignment of cell D1? Answer `ANSWER: '
                  'align=<value>`.', expect=lambda b: {'type': 'props', 'value': cell_props(b, '매출!D1', 'align')}))
    t.append(Task('ks-q3', 'read', 'In sheet 담당자, which cells use the font WenQuanYi Zen Hei? Answer `ANSWER: '
                  '<range>; <range>`.', expect=lambda b: {'type': 'cellset', 'value': sorted(
                      '%s%d' % (G.col_letters(c), r + 1) for (r, c), p in b.cells['담당자'].items()
                      if p['font'] == 'WenQuanYi Zen Hei')}))
    t.append(Task('ks-e1', 'edit', 'In sheet 매출, make the header row of the Sales table (A3:F3) bold and centred, with '
                  'a light grey fill (#D9D9D9).',
                  targets=uniform(['매출!A3:F3'], {'bold': eq('yes'), 'align': eq('center'), 'fill': is_grey}),
                  gold=[{'op': 'format', 'range': '매출!A3:F3', 'set': {'bold': True, 'align': 'center',
                                                                       'fill': '#D9D9D9'}}]))
    t.append(Task('ks-e2', 'edit', 'In sheet 매출, colour the 이익 figures (F4:F48) with the theme colour accent6.',
                  targets=uniform(['매출!F4:F48'], {'color': eq('accent6')}),
                  gold=[{'op': 'format', 'range': '매출!F4:F48', 'set': {'color': 'accent6'}}]))
    t.append(Task('ks-e3', 'edit', 'In sheet 매출, put a thin black bottom border (0.75pt solid) under the Sales table\'s '
                  'header row A3:F3.',
                  targets=uniform(['매출!A3:F3'], {'border-bottom': border_is(0.75, 'solid', eq('#000000'))}),
                  gold=[{'op': 'format', 'range': '매출!A3:F3', 'set': {'border-bottom': '0.75pt solid #000000'}}]))
    t.append(Task('ks-e4', 'edit', 'In sheet 담당자, give the header row A1:E1 the theme colour accent1 as its fill, with '
                  'white bold text.',
                  targets=uniform(['담당자!A1:E1'], {'fill': eq('accent1'), 'color': white, 'bold': eq('yes')}),
                  gold=[{'op': 'format', 'range': '담당자!A1:E1', 'set': {'fill': 'accent1', 'color': 'bg1',
                                                                          'bold': True}}]))
    t.append(Task('ks-e5', 'refuse', 'In sheet 매출, fill F4:F48 with a diagonal-stripe pattern.'))
    return t


def tasks_3():
    t = []
    t.append(Task('tb-q1', 'read', 'In sheet Exp1, what are the four borders of cell E7? Answer `ANSWER: border-top=… '
                  'border-right=… border-bottom=… border-left=…`.',
                  expect=lambda b: {'type': 'props', 'value': cell_props(b, 'Exp1!E7', *V.SIDES)}))
    t.append(Task('tb-q2', 'read', 'In sheet Exp1, what are the font size and the horizontal alignment of cell C15? '
                  'Answer `ANSWER: size=<pt> align=<value>`.',
                  expect=lambda b: {'type': 'props', 'value': cell_props(b, 'Exp1!C15', 'size', 'align')}))
    t.append(Task('tb-q3', 'read', 'In sheet Exp8, is cell A1 bold, and what is its font size? Answer `ANSWER: '
                  'bold=<yes|no> size=<pt>`.',
                  expect=lambda b: {'type': 'props', 'value': cell_props(b, 'Exp8!A1', 'bold', 'size')}))
    t.append(Task('tb-e1', 'edit', 'In sheet Exp1, fill the header row A7:J7 with light grey (#D9D9D9); keep its '
                  'borders.', targets=uniform(['Exp1!A7:J7'], {'fill': is_grey}),
                  gold=[{'op': 'format', 'range': 'Exp1!A7:J7', 'set': {'fill': '#D9D9D9'}}]))
    t.append(Task('tb-e2', 'edit', 'In sheet Exp1, put a thick black outline (2.25pt solid) around the block A7:J22.',
                  targets=outline_of('Exp1!A7:J22', border_is(2.25, 'solid', eq('#000000'))),
                  gold=[{'op': 'format', 'range': 'Exp1!A7:J22', 'set': {'outline': '2.25pt solid #000000'}}]))
    t.append(Task('tb-e3', 'edit', 'In every one of the twelve sheets, make cell A1 14pt.',
                  targets=lambda b: {(s, (0, 0)): {'size': eq('14pt')} for s in b.sheets},
                  gold=lambda b: [{'op': 'format', 'range': "'%s'!A1" % s, 'set': {'size': '14pt'}} for s in b.sheets]))
    t.append(Task('tb-e4', 'edit', 'In sheet Exp4, set the font of the whole used range (A1:K24) to Arial.',
                  targets=uniform(['Exp4!A1:K24'], {'font': eq('Arial')}),
                  gold=[{'op': 'format', 'range': 'Exp4!A1:K24', 'set': {'font': 'Arial'}}]))
    t.append(Task('tb-e5', 'refuse', 'In sheet Exp1, draw a diagonal line across cell A7.'))
    return t


TASKS = {1: tasks_1(), 2: tasks_2(), 3: tasks_3()}


# ================================================================ scoring

def expand_cells(book, sheet, body):
    out = set()
    for item in re.split(r'[;,\s]+', body):
        item = item.strip().strip('.`"\'')
        if not item:
            continue
        if '!' in item:
            item = item.rsplit('!', 1)[1]
        m = re.fullmatch(r'\$?([A-Z]{1,3})\$?(\d+)(?::\$?([A-Z]{1,3})\$?(\d+))?', item)
        if not m:
            continue
        r1, c1 = G.ref_rc(m.group(1) + m.group(2))
        r2, c2 = G.ref_rc(m.group(3) + m.group(4)) if m.group(3) else (r1, c1)
        for r in range(r1, r2 + 1):
            for c in range(c1, c2 + 1):
                out.add('%s%d' % (G.col_letters(c), r + 1))
    return out


def read_ok(book, task, text):
    exp = task.expect(book)
    t = text.strip()
    if not t.upper().startswith('ANSWER:'):
        return False, 'a read answer is `ANSWER: …`'
    body = t[7:].strip().rstrip('.')
    if exp['type'] == 'cellset':
        got = expand_cells(book, None, body)
        want = set(exp['value'])
        if got != want:
            return False, 'got %s, want %s' % (sorted(got), sorted(want))
        return True, ''
    for k, v in exp['value'].items():
        mm = re.search(r'(?<![a-z-])%s\s*[=:]\s*("[^"]*"|-?[\d.]+pt\s+[a-z-]+\s+[^\s,;]+|[^\s,;]+)'
                       % re.escape(k), body)
        if not mm:
            return False, 'no value for %s in %r' % (k, body)
        raw = mm.group(1).strip('"')
        try:
            g = V.parse_value(k, raw) if k != 'align' else raw
            if k in V.SIDES and g != 'none':
                g = snap_border(g)
        except (V.VocabError, OpError) as e:
            return False, str(e)
        want = v
        if k == 'color' or k == 'fill':
            ok = g == want or (g.startswith('#') and want.startswith('#') and g.upper() == want.upper())
        elif k == 'font':
            ok = g.strip() == want.strip()
        else:
            ok = V.close(g, want, 0.05) if hasattr(V, 'close') else g == want
        if not ok:
            return False, '%s: got %s, want %s' % (k, g, want)
    return True, ''


def check(book, task, cells):
    targets = task.targets(book)
    errs = []
    for sheet in book.sheets:
        keys = set(book.cells[sheet]) | set(cells[sheet])
        keys |= {rc for (s, rc) in targets if s == sheet}
        for rc in sorted(keys):
            before = book.get(sheet, rc)
            after = cells[sheet].get(rc, book.base)
            want = targets.get((sheet, rc), {})
            ref = '%s!%s%d' % (sheet, G.col_letters(rc[1]), rc[0] + 1)
            for k in KEYS:
                if k in want:
                    if not want[k](after.get(k)):
                        errs.append('%s: %s is %s' % (ref, k, after.get(k)))
                elif after.get(k) != before.get(k):
                    errs.append('%s: %s changed: %s -> %s' % (ref, k, before.get(k), after.get(k)))
            if len(errs) > 3:
                return errs
    return errs


def is_refusal(a):
    return isinstance(a.get('text'), str) and a['text'].strip().upper().startswith('REFUSE:')


def score_answer(unit_id, answers):
    rep = int(unit_id.rsplit('-', 1)[1])
    book = Book(rep)
    by_id = {a.get('task_id'): a for a in answers.get('answers', []) if isinstance(a, dict)}
    out = []
    for task in TASKS[rep]:
        a = by_id.get(task.id)
        res = {'task_id': task.id, 'valid': False, 'landed': False, 'error': None, 'chars': 0, 'flags': [],
               'detail': '', 'ops': 0}
        out.append(res)
        if a is None:
            res['error'] = 'no answer'
            res['flags'] = ['missing']
            continue
        res['chars'] = len(json.dumps(a.get('ops'), ensure_ascii=False)) if 'ops' in a else len(a.get('text') or '')
        if task.kind == 'refuse':
            res['valid'] = True
            res['landed'] = is_refusal(a)
            if not res['landed']:
                res['flags'] = ['should_refuse']
            continue
        if is_refusal(a):
            res['valid'] = True
            res['flags'] = ['wrong_refusal']
            res['detail'] = a['text'][:200]
            continue
        if task.kind == 'read':
            if not isinstance(a.get('text'), str):
                res['error'] = 'a read answer is {"task_id", "text": "ANSWER: …"}'
                continue
            res['valid'] = True
            res['landed'], why = read_ok(book, task, a['text'])
            if not res['landed']:
                res['flags'] = ['wrong_answer']
                res['detail'] = why
            continue
        try:
            cells = apply_ops(book, a.get('ops'))
        except OpError as e:
            res['error'] = str(e)
            res['flags'] = ['invalid_op']
            continue
        res['valid'] = True
        res['ops'] = len(a['ops'])
        errs = check(book, task, cells)
        res['landed'] = not errs
        if errs:
            res['flags'] = ['not_landed']
            res['detail'] = errs[0]
    return out


# ================================================================ guide, prompts, build

GUIDE = """A workbook file is plain text. It describes the workbook's structure; the cell values are shown in `<data>` windows.

### Text

- `<sheet name="…" range="A1:H23">` … `</sheet>` is one sheet and its used range.
- `<table name="…" range="…">` lists a table's columns (name, type, number format, formula). `<data …>` shows cell values: a pipe table whose first column is the sheet row number, then one column per sheet column or table column.
- `<keep id="…" kind="…" summary="…"/>` stands for content kept for you (merged cells, charts, conditional formats, notes, validations); leave it as it is.

### Formatting

Formatting is shown as range lines, in a small fixed vocabulary.

- The line `<format default …/>` at the top is the workbook's default cell formatting (the Normal style): every cell has it unless a range line says otherwise. A property it leaves out is none, off, general (alignment), bottom (vertical alignment) or 0 (indent).
- In a sheet, `<format range="B4:C7" key=value …/>` gives every cell of that rectangle the properties written, beyond the default. A cell is in at most one range line; a property its line leaves out is the default's. A cell in no range line has the default formatting.

### The vocabulary

`key=value` pairs separated by spaces; a value with a space is quoted.

- `fill` (a colour or `none`), `border-top` / `-right` / `-bottom` / `-left` (a border is `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`), `align` (general, left, center, right, justify, distribute), `valign` (top, middle, bottom), `indent` (a whole number of indent levels), `font`, `size` (pt), `color`, `bold`, `italic`, `underline`, `strike` (a flag written alone means on).
- A colour is `#RRGGBB` or a theme colour by name: `tx1`, `bg1`, `tx2`, `bg2`, `accent1` … `accent6`, `hlink`; `+N%` after a theme colour means N% lighter and `-N%` N% darker (`bg2-10%`, `accent2+80%`). A theme colour follows the workbook's theme.
- Excel border widths are hair 0.25pt (dotted), thin 0.75pt, medium 1.5pt and thick 2.25pt; `double` is 2.25pt.
- `fill=gradient` and `fill=pattern` are shown but cannot be written. Nothing else can be expressed: a gradient, a pattern, a diagonal line in a cell, a shadow, a conditional format.

### Changing formatting

The file is not edited as text. Formatting is written by range operations, applied in order:

```
{"op": "format", "range": "Sheet!A1:D1", "set": {"fill": "#D9D9D9", "bold": true, "border-bottom": "0.75pt solid #000000"}}
```

- `range` is `Sheet!A1` or `Sheet!A1:D4` (a sheet name with spaces or brackets is quoted: `'Exp2 (2)'!A1`).
- `set` gives every cell of the range the properties written, with the same keys and values as the vocabulary (`true` / `false` for the flags), and leaves every other property of those cells as it is. `"fill": "none"` removes a fill and `"border-left": "none"` a border.
- Two keys are for operations only: `border` sets all four sides of every cell of the range, and `outline` sets only the outer edges of the range (the top of its first row, the bottom of its last row, the left of its first column, the right of its last column).
- A border width snaps to the nearest Excel width (2pt becomes 2.25pt); theme colours stay theme colours.
"""

HEADER = """You work with office files stored as plain text. Below are the syntax documentation, the file itself, and {n} tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

{guide}
## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
{file}
=== FILE END ===

## Tasks

{tasks}

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Edit tasks: answer with a list of range operations (`"ops"`), as the documentation describes. Change only what the task asks for.
- If a task cannot be done in this file format, answer `REFUSE: <one sentence why>`.

## Answer format

Reply with one JSON object and nothing else:

```
{{"answers": [
  {{"task_id": "…", "text": "ANSWER: …"}},
  {{"task_id": "…", "ops": [{{"op": "format", "range": "…", "set": {{…}}}}]}},
  {{"task_id": "…", "text": "REFUSE: …"}}
]}}
```

One entry per task, in the order of the tasks.
"""


def gold_of(book, t):
    if t.kind == 'read':
        exp = t.expect(book)
        if exp['type'] == 'cellset':
            return {'task_id': t.id, 'text': 'ANSWER: ' + '; '.join(exp['value'])}
        return {'task_id': t.id, 'text': 'ANSWER: ' + ' '.join('%s=%s' % (k, V.quote(v)) for k, v in
                                                            exp['value'].items())}
    if t.kind == 'refuse':
        return {'task_id': t.id, 'text': 'REFUSE: this cannot be expressed in a workbook here.'}
    return {'task_id': t.id, 'ops': t.gold(book) if callable(t.gold) else t.gold}


def build():
    index = []
    for rep in SEEDS:
        book = Book(rep)
        text = book.text()
        uid = 'r6x-F1-%d' % rep
        open(os.path.join(HERE, 'seeds', uid + '.txt'), 'w', encoding='utf-8').write(text)
        gold = {'answers': [gold_of(book, t) for t in TASKS[rep]]}
        json.dump(gold, open(os.path.join(HERE, 'data', 'gold', uid + '.json'), 'w', encoding='utf-8'),
                  ensure_ascii=False, indent=1)
        for r in score_answer(uid, gold):
            assert r['landed'], (uid, r)
        tasks = '\n'.join('%d. `%s` (%s): %s' % (k + 1, t.id, 'read' if t.kind == 'read' else 'edit', t.text)
                          for k, t in enumerate(TASKS[rep]))
        prompt = HEADER.format(n=len(TASKS[rep]), guide=GUIDE, file=text, tasks=tasks)
        open(os.path.join(HERE, 'prompts', uid + '-a.md'), 'w', encoding='utf-8').write(prompt)
        index.append({'unit_id': uid, 'part': 'a', 'candidate': 'F1', 'seed': rep, 'file_chars': len(text),
                      'prompt_chars': len(prompt), 'tasks': [t.id for t in TASKS[rep]], 'unreachable': []})
        print(uid, 'file', len(text), 'prompt', len(prompt))
    json.dump(index, open(os.path.join(HERE, 'data', 'index-x.json'), 'w', encoding='utf-8'), indent=1)


def selftest():
    fails = 0
    cases = [
        ('r6x-F1-1', 'xb-e1', {'ops': [{'op': 'format', 'range': 'Simple Monthly Budget!B10:C10',
                                        'set': {'fill': '#F9D0E4'}}]}, False, 'a hex, not the theme colour'),
        ('r6x-F1-1', 'xb-e2', {'ops': [{'op': 'format', 'range': 'Simple Monthly Budget!B4:C7',
                                        'set': {'border': '2pt solid #1F3864'}}]}, False, 'inner borders too'),
        ('r6x-F1-1', 'xb-e2', {'ops': [
            {'op': 'format', 'range': 'Simple Monthly Budget!B4:C4', 'set': {'border-top': '2.25pt solid #000080'}},
            {'op': 'format', 'range': 'Simple Monthly Budget!B7:C7', 'set': {'border-bottom': '2.25pt solid #000080'}},
            {'op': 'format', 'range': 'Simple Monthly Budget!B4:B7', 'set': {'border-left': '2.25pt solid #000080'}},
            {'op': 'format', 'range': 'Simple Monthly Budget!C4:C7', 'set': {'border-right': '2.25pt solid #000080'}}]},
         True, 'four sides by hand'),
        ('r6x-F1-1', 'xb-e5', {'ops': [{'op': 'format', 'range': 'Simple Monthly Budget!E4:G5',
                                        'set': {'fill': 'none', 'border': 'none'}}]}, False, 'borders removed'),
        ('r6x-F1-1', 'xb-q1', {'text': 'ANSWER: fill=#D6ECFF'}, False, 'the theme colour resolved'),
        ('r6x-F1-1', 'xb-e6', {'ops': [{'op': 'format', 'range': 'Simple Monthly Budget!A1',
                                        'set': {'fill': 'gradient'}}]}, False, 'should refuse'),
        ('r6x-F1-2', 'ks-e1', {'ops': [{'op': 'format', 'range': '매출!A3:F3', 'set': {'bold': True, 'align': 'center',
                                                                                 'fill': '#D9D9D9',
                                                                                 'font': 'Arial'}}]},
         False, 'the font changed too'),
        ('r6x-F1-2', 'ks-e4', {'ops': [{'op': 'format', 'range': '담당자!A1:E1', 'set': {'fill': 'accent1',
                                                                                  'color': '#FFFFFF', 'bold': True}}]},
         True, 'white as hex'),
        ('r6x-F1-2', 'ks-e2', {'ops': [{'op': 'format', 'range': 'F4:F48', 'set': {'color': 'accent6'}}]},
         False, 'no sheet named'),
        ('r6x-F1-2', 'ks-q3', {'text': 'ANSWER: A1:E1, B2:C9'}, True, 'ranges with a comma'),
        ('r6x-F1-2', 'ks-q2', {'text': 'ANSWER: align=general'}, False, 'D1 is in the merged title range'),
        ('r6x-F1-3', 'tb-e3', {'ops': [{'op': 'format', 'range': 'Exp1!A1', 'set': {'size': '14pt'}}]}, False,
         'one sheet only'),
        ('r6x-F1-3', 'tb-e1', {'ops': [{'op': 'format', 'range': 'Exp1!A7:J7', 'set': {'fill': 'gray'}}]}, False,
         'a colour name'),
        ('r6x-F1-3', 'tb-q1', {'text': 'ANSWER: border-top=0.75pt solid #000000 border-right=0.75pt solid #000000 '
                                        'border-bottom=0.75pt solid #000000 border-left=none'}, True, 'unquoted borders'),
        ('r6x-F1-3', 'tb-q1', {'text': 'ANSWER: border-top=0.75pt solid #000000 border-right=none '
                                        'border-bottom=0.75pt solid #000000 border-left=none'}, False, 'one side wrong'),
        ('r6x-F1-3', 'tb-e2', {'ops': [{'op': 'format', 'range': 'Exp1!A7:J22',
                                        'set': {'outline': '2pt solid #000000'}}]}, True, '2pt snaps to 2.25pt'),
    ]
    n_gold = 0
    for rep in SEEDS:
        uid = 'r6x-F1-%d' % rep
        for r in score_answer(uid, json.load(open(os.path.join(HERE, 'data', 'gold', uid + '.json'),
                                                  encoding='utf-8'))):
            n_gold += 1
            if not r['landed']:
                fails += 1
                print('FAIL gold', uid, r)
    for uid, tid, ans, want, why in cases:
        r = next(x for x in score_answer(uid, {'answers': [dict(ans, task_id=tid)]}) if x['task_id'] == tid)
        ok = r['landed'] == want
        fails += not ok
        print('  %s %-9s %-6s %-35s %s %s' % ('ok  ' if ok else 'FAIL', uid, tid, why, r['flags'],
                                             (r['error'] or r['detail'] or '')[:90]))
    print('gold answers: %d; cases: %d' % (n_gold, len(cases)))
    if fails:
        sys.exit(1)
    print('SELFTEST PASSED')


def main():
    cmd = sys.argv[1]
    if cmd == 'build':
        build()
    elif cmd == 'score':
        print(json.dumps({'results': score_answer(sys.argv[2], json.load(open(sys.argv[3], encoding='utf-8')))},
                         ensure_ascii=False, indent=1))
    elif cmd == 'selftest':
        selftest()


if __name__ == '__main__':
    main()
