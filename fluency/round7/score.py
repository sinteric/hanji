#!/usr/bin/env python3
"""Round 7 scorer. Usage: python3 score.py <unit_id> <answers.json>

Prints {"results": [{task_id, valid, landed, error, chars, flags, detail}]}.

Each candidate is parsed into one semantic model per decision (inlines for breaks; sheet items for shapes and
data bars). Edits are applied with round 2's apply_edits. A write lands when the parsed file equals the brief;
an edit lands when the parsed file equals the gold parse; a read lands when its ANSWER matches the expected
tokens (whitespace-collapsed); a refusal lands when the task is the refusal task.
"""
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
UNITS = os.path.join(HERE, 'data', 'units')
GOLD = os.path.join(HERE, 'data', 'gold')

_spec = importlib.util.spec_from_file_location('hanji_r2score', os.path.join(HERE, '..', 'round2', 'score.py'))
r2 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(r2)

count_occ = r2.count_occ
apply_edits = r2.apply_edits
first_diff = r2.first_diff
ntext = r2.ntext
parse_attrs = r2.parse_attrs
MAX_ERRORS = r2.MAX_ERRORS
REFUSE = 'REFUSE:'

TAG_RE = re.compile(r'<\s*(/?)\s*([A-Za-z][\w-]*)([^<>]*?)(/?)\s*>')
FM_KEYS = ['type', 'format', 'template', 'schema']
CFVO = ('min', 'max', 'num', 'percent', 'percentile', 'formula')
THEME = ('tx1', 'bg1', 'tx2', 'bg2', 'accent1', 'accent2', 'accent3', 'accent4', 'accent5', 'accent6', 'hlink')
CELLIS_OPS = ('between', 'notBetween', 'equal', 'notEqual', 'greaterThan', 'lessThan',
              'greaterThanOrEqual', 'lessThanOrEqual')
A1 = re.compile(r'^\$?([A-Za-z]{1,3})\$?(\d+)(?::\$?([A-Za-z]{1,3})\$?(\d+))?$')


def col_n(s):
    n = 0
    for ch in s.upper():
        n = n * 26 + (ord(ch) - 64)
    return n


def col_s(n):
    out = []
    while n:
        n, r = divmod(n - 1, 26)
        out.append(chr(65 + r))
    return ''.join(reversed(out))


def canon_range(s):
    m = A1.fullmatch((s or '').strip())
    if not m:
        return None
    c1, r1 = col_n(m.group(1)), int(m.group(2))
    if m.group(3):
        c2, r2 = col_n(m.group(3)), int(m.group(4))
        if (r2, c2) < (r1, c1):
            c1, r1, c2, r2 = c2, r2, c1, r1
        if (r1, c1) == (r2, c2):
            return '%s%d' % (col_s(c1), r1)
        return '%s%d:%s%d' % (col_s(c1), r1, col_s(c2), r2)
    return '%s%d' % (col_s(c1), r1)


def canon_color(s):
    if not s:
        return None
    t = s.strip()
    if t.startswith('#') and len(t) == 7 and re.fullmatch(r'#[0-9A-Fa-f]{6}', t):
        return t.upper()
    if t.lower() in THEME:
        return t.lower()
    return None


class Ctx:
    def __init__(self, unit):
        self.unit = unit
        self.decision = unit['decision']
        self.cand = unit['candidate']
        self.errors = []
        self.flags = set()

    def add(self, flag, line, col, msg):
        self.flags.add(flag)
        self.errors.append((flag, line, col, msg))

    def error_text(self):
        out = ['line %d, column %d: %s' % (l, c, m) for _, l, c, m in self.errors[:MAX_ERRORS]]
        if len(self.errors) > MAX_ERRORS:
            out.append('(%d more errors)' % (len(self.errors) - MAX_ERRORS))
        return '\n'.join(out)

    def hint(self):
        if self.decision == 'breaks':
            if self.cand == 'A':
                return ('Inside a paragraph, <pagebreak/> is a page break and <columnbreak/> is a column break. '
                        'A line holding only <pagebreak/> is a page break between paragraphs. <br/> is a line break.')
            return ('Inside a paragraph, <br type="page"/> is a page break and <br type="column"/> is a column '
                    'break. A line holding only <pagebreak/> is a page break between paragraphs. <br/> is a line break.')
        if self.decision == 'shapes':
            if self.cand == 'A':
                return ('A shape is <shape name="…" anchor="B2:D5">text</shape>; a group is <group name="…"> … '
                        '</group>. Pictures, charts, rotation and fills cannot be written.')
            return ('A shape is <shape name="…" anchor="B2:D5" text="…"/>; a grouped shape adds group="Name". '
                    'Pictures, charts, rotation and fills cannot be written.')
        if self.cand == 'A':
            return ('A data bar is <conditional-format range type="dataBar" min max fill>. min/max are min, max, '
                    'num:N, percent:N, percentile:N or formula:=…. Color scales and icon sets cannot be written.')
        return ('A data bar is <conditional-format range type="dataBar" min-type max-type color>, with min/max '
                'values when the type needs one. Color scales and icon sets cannot be written.')


# ---------------------------------------------------------------- front matter

def split_fm(text, ctx):
    lines = text.splitlines(keepends=True)
    if not lines or lines[0].strip() != '---':
        ctx.add('no_front_matter', 1, 1, 'the file begins with a front matter block between two --- lines.')
        return {}, text
    i = 1
    fm = {}
    while i < len(lines) and lines[i].strip() != '---':
        ln = lines[i].rstrip('\n')
        if ln.strip() and ':' in ln:
            k, v = ln.split(':', 1)
            fm[k.strip()] = v.strip()
        i += 1
    if i >= len(lines):
        ctx.add('front_matter', 1, 1, 'front matter is not closed by a --- line.')
        return fm, text
    return fm, ''.join(lines[i + 1:])


def fm_text(fm):
    keys = [k for k in FM_KEYS if k in fm] + [k for k in fm if k not in FM_KEYS]
    return '---\n' + ''.join('%s: %s\n' % (k, fm[k]) for k in keys) + '---\n'


# ---------------------------------------------------------------- inlines (breaks)

def parse_inlines(s, ln, ctx, allow_block_pagebreak=False):
    """Parse a paragraph's text into [['t', …], ['br'], ['page'], ['col'], …]."""
    out, pos = [], 0
    for m in TAG_RE.finditer(s):
        if m.start() > pos:
            out.append(['t', s[pos:m.start()]])
        closing, name, attrs, selfcl = m.group(1), m.group(2).lower(), parse_attrs(m.group(3)), m.group(4)
        extra = set(a.lower() for a in attrs) 
        if name == 'br':
            ty = (attrs.get('type') or '').lower()
            extra -= {'type'}
            if extra:
                ctx.add('invented_attr', ln, m.start() + 1,
                        '<br> takes no attribute other than type (page or column). ' + ctx.hint())
            if ctx.decision == 'breaks' and ctx.cand == 'A':
                if ty:
                    ctx.add('invented_attr', ln, m.start() + 1,
                            '<br/> is a line break and takes no attribute. ' + ctx.hint())
                else:
                    out.append(['br'])
            else:
                if ty in ('page', 'column'):
                    out.append(['page'] if ty == 'page' else ['col'])
                elif ty:
                    ctx.add('invented_attr', ln, m.start() + 1,
                            'type is page or column, not %r. ' % ty + ctx.hint())
                else:
                    out.append(['br'])
        elif name == 'pagebreak':
            if extra:
                ctx.add('invented_attr', ln, m.start() + 1, '<pagebreak/> takes no attribute.')
            if ctx.decision == 'breaks' and ctx.cand == 'A':
                out.append(['page'])
            elif allow_block_pagebreak:
                out.append(['page'])
            else:
                ctx.add('unknown_tag', ln, m.start() + 1,
                        '<pagebreak/> on its own line is a page break between paragraphs; inside a paragraph use '
                        'the inline form. ' + ctx.hint())
        elif name == 'columnbreak':
            if ctx.decision == 'breaks' and ctx.cand == 'A':
                if extra:
                    ctx.add('invented_attr', ln, m.start() + 1, '<columnbreak/> takes no attribute.')
                out.append(['col'])
            else:
                ctx.add('unknown_tag', ln, m.start() + 1, 'unknown tag <columnbreak>. ' + ctx.hint())
        elif name in ('u', 'keep'):
            out.append(['t', m.group(0)])
        else:
            ctx.add('unknown_tag', ln, m.start() + 1, 'unknown tag <%s>. %s' % (name, ctx.hint()))
        if closing:
            ctx.add('unclosed_tag', ln, m.start() + 1, 'this tag is a single tag, closed by />.')
        pos = m.end()
    if pos < len(s):
        out.append(['t', s[pos:]])
    return merge_text(out)


def merge_text(parts):
    out = []
    for p in parts:
        if p[0] == 't' and out and out[-1][0] == 't':
            out[-1][1] += p[1]
        elif p[0] == 't' and p[1] == '':
            continue
        else:
            out.append(p)
    return out


def render_inlines(parts, cand):
    out = []
    for p in parts:
        p = list(p)
        if p[0] == 't':
            out.append(p[1])
        elif p[0] == 'br':
            out.append('<br/>')
        elif p[0] == 'page':
            out.append('<pagebreak/>' if cand == 'A' else '<br type="page"/>')
        elif p[0] == 'col':
            out.append('<columnbreak/>' if cand == 'A' else '<br type="column"/>')
    return ''.join(out)


def inlines_of(x):
    return [list(p) for p in x]


# ---------------------------------------------------------------- documents

def parse_doc(body, ctx):
    blocks, lines = [], body.splitlines()
    i, n = 0, len(lines)
    while i < n:
        raw = lines[i]
        ln = i + 1
        s = raw.strip()
        if s == '':
            i += 1
            continue
        if s == '<pagebreak/>' or re.fullmatch(r'<\s*pagebreak\s*/\s*>', s, re.I):
            blocks.append({'k': 'pagebreak'})
            i += 1
            continue
        km = re.fullmatch(r'<\s*keep\b([^<>]*)/\s*>', s, re.I)
        if km:
            a = parse_attrs(km.group(1))
            blocks.append({'k': 'keep', 'id': a.get('id', ''), 'kind': a.get('kind', ''),
                           'summary': a.get('summary', '')})
            i += 1
            continue
        if s.startswith('|') and i + 1 < n and re.match(r'^\|?\s*---', lines[i + 1].strip()):
            header = [c.strip() for c in s.strip('|').split('|')]
            i += 2
            rows = []
            while i < n and lines[i].strip().startswith('|'):
                rows.append([c.strip() for c in lines[i].strip().strip('|').split('|')])
                i += 1
            style = None
            if blocks and blocks[-1].get('k') == 'styleline':
                style = blocks.pop()['style']
            blocks.append({'k': 'table', 'style': style, 'header': header, 'rows': rows})
            continue
        sm = re.fullmatch(r'\{style="([^"]+)"\}', s)
        if sm:
            blocks.append({'k': 'styleline', 'style': sm.group(1)})
            i += 1
            continue
        hm = re.match(r'^(#{1,6})\s+(.*)$', s)
        if hm:
            blocks.append({'k': 'h', 'level': len(hm.group(1)),
                           'text': parse_inlines(hm.group(2), ln, ctx)})
            i += 1
            continue
        dm = re.fullmatch(r'<\s*div\s+style="([^"]+)"\s*>(.*)</\s*div\s*>', s, re.I | re.S)
        if dm:
            blocks.append({'k': 'p', 'style': dm.group(1), 'text': parse_inlines(dm.group(2), ln, ctx)})
            i += 1
            continue
        if raw.startswith('  - '):
            blocks.append({'k': 'ul2', 'text': parse_inlines(raw[4:], ln, ctx)})
            i += 1
            continue
        if s.startswith('- '):
            blocks.append({'k': 'ul', 'text': parse_inlines(s[2:], ln, ctx)})
            i += 1
            continue
        om = re.match(r'^1\.\s+(.*)$', s)
        if om:
            blocks.append({'k': 'ol', 'text': parse_inlines(om.group(1), ln, ctx)})
            i += 1
            continue
        if ctx.cand == 'A' and re.fullmatch(r'<\s*columnbreak\s*/\s*>', s, re.I):
            blocks.append({'k': 'p', 'style': None, 'text': [['col']]})
            i += 1
            continue
        if ctx.cand == 'B' and re.fullmatch(r'<\s*br\s+type="column"\s*/\s*>', s, re.I):
            blocks.append({'k': 'p', 'style': None, 'text': [['col']]})
            i += 1
            continue
        blocks.append({'k': 'p', 'style': None, 'text': parse_inlines(s, ln, ctx)})
        i += 1
    return [b for b in blocks if b.get('k') != 'styleline' or False]


def render_doc(model, cand):
    fm = model['fm'] if 'fm' in model else model.get('front', {})
    lines = [fm_text(fm).rstrip('\n'), '']
    prev = None
    for b in model['blocks']:
        k = b['k']
        if k == 'pagebreak':
            if prev not in (None, 'pagebreak'):
                lines.append('')
            lines.append('<pagebreak/>')
            lines.append('')
            prev = 'pagebreak'
            continue
        if prev == 'list' and k in ('ul', 'ul2', 'ol'):
            pass
        elif prev is not None:
            if not (lines and lines[-1] == ''):
                lines.append('')
        if k == 'h':
            lines.append('#' * b['level'] + ' ' + render_inlines(inlines_of(b['text']), cand))
            prev = 'h'
        elif k == 'p':
            t = render_inlines(inlines_of(b['text']), cand)
            if b.get('style'):
                lines.append('<div style="%s">%s</div>' % (b['style'], t))
            else:
                lines.append(t)
            prev = 'p'
        elif k == 'ul':
            lines.append('- ' + render_inlines(inlines_of(b['text']), cand))
            prev = 'list'
        elif k == 'ul2':
            lines.append('  - ' + render_inlines(inlines_of(b['text']), cand))
            prev = 'list'
        elif k == 'ol':
            lines.append('1. ' + render_inlines(inlines_of(b['text']), cand))
            prev = 'list'
        elif k == 'keep':
            lines.append('<keep id="%s" kind="%s" summary="%s"/>' % (b['id'], b['kind'], b['summary']))
            prev = 'keep'
        elif k == 'table':
            if b.get('style'):
                lines.append('{style="%s"}' % b['style'])
            hdr = b['header']
            lines.append('| ' + ' | '.join(hdr) + ' |')
            lines.append('|' + '|'.join(['---'] * len(hdr)) + '|')
            for row in b['rows']:
                lines.append('| ' + ' | '.join(row) + ' |')
            prev = 'table'
    text = '\n'.join(lines).strip() + '\n'
    return text


# ---------------------------------------------------------------- spreadsheets

def parse_cfvo_a(tok):
    if tok in ('min', 'max'):
        return [tok, None]
    if ':' in tok:
        ty, val = tok.split(':', 1)
        if ty in CFVO:
            return [ty, val]
    return None


def fmt_cfvo_a(pair):
    ty, val = pair[0], pair[1]
    return ty if val is None else '%s:%s' % (ty, val)


def parse_sheet_tag(s, ln, ctx):
    m = re.fullmatch(r'<\s*(/?)\s*([A-Za-z][\w-]*)([^<>]*?)(/?)\s*>', s.strip())
    if not m:
        return None
    return {'close': bool(m.group(1)), 'name': m.group(2).lower(), 'attrs': parse_attrs(m.group(3)),
            'self': bool(m.group(4)), 'raw': s.strip()}


def parse_shape_a(s, ln, ctx):
    m = re.fullmatch(r'<\s*shape\b([^<>]*)/>(\s*)$', s.strip(), re.I)
    if m:
        a = parse_attrs(m.group(1))
        return shape_from_attrs(a, '', ln, ctx, body=False)
    m = re.fullmatch(r'<\s*shape\b([^<>]*)>(.*)</\s*shape\s*>$', s.strip(), re.I | re.S)
    if not m:
        ctx.add('shape_form', ln, 1, 'a shape is <shape name="…" anchor="…">text</shape> or <shape name anchor/>. '
                + ctx.hint())
        return None
    return shape_from_attrs(parse_attrs(m.group(1)), m.group(2), ln, ctx, body=True)


def shape_from_attrs(a, text, ln, ctx, body):
    extra = set(k.lower() for k in a) - {'name', 'anchor', 'text', 'group'}
    if ctx.cand == 'A':
        extra -= set()
        if 'text' in a:
            ctx.add('invented_attr', ln, 1, 'a shape\'s text is written between the tags, not as text="…". '
                    + ctx.hint())
        if 'group' in a:
            ctx.add('invented_attr', ln, 1, 'a grouped shape is written inside <group name="…"> … </group>. '
                    + ctx.hint())
        extra -= {'text', 'group'}
    else:
        if body:
            ctx.add('shape_form', ln, 1, 'a shape is one self-closing line with text="…". ' + ctx.hint())
    if extra:
        ctx.add('invented_attr', ln, 1, 'unknown attribute %s. %s' % (', '.join(sorted(extra)), ctx.hint()))
    name, anchor = a.get('name'), a.get('anchor')
    if not name or not anchor:
        ctx.add('missing_attr', ln, 1, 'a shape needs name and anchor. ' + ctx.hint())
        return None
    rng = canon_range(anchor)
    if rng is None:
        ctx.add('bad_anchor', ln, 1, 'anchor is an A1 cell or range such as B25:D27, got %r.' % anchor)
        return None
    tx = text if ctx.cand == 'A' else (a.get('text') or '')
    g = None if ctx.cand == 'A' else (a.get('group') or None)
    return {'k': 'shape', 'name': name, 'anchor': rng, 'text': tx, 'group': g}


def parse_cf(a, ln, ctx):
    ty = a.get('type', '')
    rng = canon_range(a.get('range', ''))
    if rng is None:
        ctx.add('bad_range', ln, 1, 'range is an A1 cell or range, got %r.' % a.get('range'))
        return None
    extra = set(k.lower() for k in a)
    if ty == 'cellIs':
        extra -= {'range', 'type', 'operator', 'formula', 'formula2', 'dxf', 'stopiftrue'}
        if extra:
            ctx.add('invented_attr', ln, 1, 'unknown attribute %s on cellIs.' % ', '.join(sorted(extra)))
        op = a.get('operator', '')
        if op and op not in CELLIS_OPS:
            ctx.add('invented_attr', ln, 1, 'operator is one of %s.' % ', '.join(CELLIS_OPS))
        return {'k': 'cf', 'type': 'cellIs', 'range': rng, 'operator': op, 'formula': a.get('formula', ''),
                'dxf': a.get('dxf', '')}
    if ty == 'dataBar':
        if ctx.decision != 'databar':
            ctx.add('unknown_tag', ln, 1, 'type="dataBar" is not used in this file. ' + ctx.hint())
            return None
        if ctx.cand == 'A':
            extra -= {'range', 'type', 'min', 'max', 'fill'}
            if extra:
                ctx.add('invented_attr', ln, 1, 'a data bar takes range, type, min, max, fill. ' + ctx.hint())
            mn = parse_cfvo_a(a.get('min', ''))
            mx = parse_cfvo_a(a.get('max', ''))
            fill = canon_color(a.get('fill', ''))
            if mn is None or mx is None:
                ctx.add('bad_cfvo', ln, 1,
                        'min and max are min, max, num:N, percent:N, percentile:N or formula:=…. ' + ctx.hint())
                return None
            if fill is None:
                ctx.add('bad_color', ln, 1, 'fill is #RRGGBB or a theme name. ' + ctx.hint())
                return None
            return {'k': 'cf', 'type': 'dataBar', 'range': rng, 'min': mn, 'max': mx, 'fill': fill}
        extra -= {'range', 'type', 'min-type', 'max-type', 'min', 'max', 'color'}
        if extra:
            ctx.add('invented_attr', ln, 1,
                    'a data bar takes range, type, min-type, max-type, color, and min/max when needed. ' + ctx.hint())
        mt, xt = a.get('min-type', ''), a.get('max-type', '')
        if mt not in CFVO or xt not in CFVO:
            ctx.add('bad_cfvo', ln, 1, 'min-type and max-type are min, max, num, percent, percentile or formula. '
                    + ctx.hint())
            return None
        mv, xv = a.get('min'), a.get('max')
        if mt in ('min', 'max') and mv:
            ctx.add('invented_attr', ln, 1, 'min/max types min and max take no value.')
        if xt in ('min', 'max') and xv:
            ctx.add('invented_attr', ln, 1, 'min/max types min and max take no value.')
        if mt not in ('min', 'max') and not mv:
            ctx.add('missing_attr', ln, 1, 'min-type %s needs min="…".' % mt)
            return None
        if xt not in ('min', 'max') and not xv:
            ctx.add('missing_attr', ln, 1, 'max-type %s needs max="…".' % xt)
            return None
        fill = canon_color(a.get('color', ''))
        if fill is None:
            ctx.add('bad_color', ln, 1, 'color is #RRGGBB or a theme name. ' + ctx.hint())
            return None
        return {'k': 'cf', 'type': 'dataBar', 'range': rng,
                'min': [mt, None if mt in ('min', 'max') else mv],
                'max': [xt, None if xt in ('min', 'max') else xv], 'fill': fill}
    ctx.add('invented_attr', ln, 1, 'conditional-format type is cellIs or dataBar. ' + ctx.hint())
    return None


def parse_table_block(lines, i, ln0, ctx):
    open_ = parse_sheet_tag(lines[i], ln0, ctx)
    a = open_['attrs']
    cols, i = [], i + 1
    # skip header + delimiter
    while i < len(lines) and lines[i].strip() == '':
        i += 1
    if i < len(lines) and lines[i].strip().startswith('|'):
        i += 1
    if i < len(lines) and re.match(r'^\|?\s*---', lines[i].strip()):
        i += 1
    while i < len(lines):
        s = lines[i].strip()
        if s.lower().startswith('</table'):
            i += 1
            break
        if s.startswith('|'):
            cells = [c.strip() for c in s.strip('|').split('|')]
            while len(cells) < 4:
                cells.append('')
            cols.append({'name': cells[0], 'type': cells[1], 'format': cells[2], 'formula': cells[3]})
        i += 1
    return {'k': 'table', 'name': a.get('name', ''), 'range': a.get('range', ''), 'cols': cols}, i


def parse_book(body, ctx):
    lines = body.splitlines()
    sheets, i, n = [], 0, len(lines)
    while i < n:
        s = lines[i].strip()
        ln = i + 1
        if s == '':
            i += 1
            continue
        tag = parse_sheet_tag(s, ln, ctx)
        if not tag:
            ctx.add('unknown_tag', ln, 1, 'expected a sheet tag. ' + ctx.hint())
            i += 1
            continue
        if tag['name'] != 'sheet' or tag['close']:
            ctx.add('unknown_tag', ln, 1, 'the body is a sequence of <sheet> blocks. ' + ctx.hint())
            i += 1
            continue
        sh = {'name': tag['attrs'].get('name', ''), 'range': tag['attrs'].get('range', ''), 'items': []}
        i += 1
        group_open = None
        while i < n:
            s = lines[i].strip()
            ln = i + 1
            if s == '':
                i += 1
                continue
            # A's <shape name anchor>text</shape> is two tags on one line; parse_sheet_tag is one tag.
            if ctx.decision == 'shapes' and re.match(r'<\s*shape\b', s, re.I):
                if ctx.cand == 'A':
                    it = parse_shape_a(s, ln, ctx)
                else:
                    tag = parse_sheet_tag(s, ln, ctx)
                    if not tag or not tag['self']:
                        ctx.add('shape_form', ln, 1, 'a shape is a self-closing line. ' + ctx.hint())
                        i += 1
                        continue
                    it = shape_from_attrs(tag['attrs'], '', ln, ctx, body=False)
                if it:
                    if group_open is not None:
                        it['group'] = group_open['name']
                        group_open['children'].append(it)
                    else:
                        sh['items'].append(it)
                i += 1
                continue
            tag = parse_sheet_tag(s, ln, ctx)
            if not tag:
                ctx.add('unknown_tag', ln, 1, 'unknown line. ' + ctx.hint())
                i += 1
                continue
            if tag['name'] == 'sheet' and tag['close']:
                i += 1
                break
            if tag['name'] == 'table' and not tag['close']:
                t, i = parse_table_block(lines, i, ln, ctx)
                sh['items'].append(t)
                continue
            if tag['name'] == 'group' and not tag['close'] and not tag['self']:
                if ctx.cand != 'A' or ctx.decision != 'shapes':
                    ctx.add('unknown_tag', ln, 1, 'unknown tag <group>. ' + ctx.hint())
                    i += 1
                    continue
                group_open = {'k': 'group', 'name': tag['attrs'].get('name', ''), 'children': []}
                i += 1
                continue
            if tag['name'] == 'group' and tag['close']:
                if group_open is None:
                    ctx.add('unclosed_tag', ln, 1, 'a closing </group> with no opening <group>.')
                else:
                    sh['items'].append(group_open)
                    group_open = None
                i += 1
                continue
            if tag['name'] == 'shape':
                ctx.add('unknown_tag', ln, 1, 'unknown tag <shape>. ' + ctx.hint())
                i += 1
                continue
            # self-closing structure lines
            a = tag['attrs']
            it = None
            if tag['name'] == 'merge':
                it = {'k': 'merge', 'range': a.get('range', '')}
            elif tag['name'] == 'hyperlink':
                it = {'k': 'hyperlink', 'range': a.get('range', ''), 'target': a.get('target', '')}
            elif tag['name'] == 'note':
                it = {'k': 'note', 'range': a.get('range', ''), 'author': a.get('author', ''),
                      'text': a.get('text', '')}
            elif tag['name'] == 'validation':
                it = {'k': 'validation', 'range': a.get('range', ''), 'type': a.get('type', ''),
                      'formula1': a.get('formula1', '')}
            elif tag['name'] == 'conditional-format':
                it = parse_cf(a, ln, ctx)
            elif tag['name'] == 'chart':
                it = {'k': 'chart', 'type': a.get('type', ''), 'data': a.get('data', ''),
                      'title': a.get('title', '')}
            elif tag['name'] == 'keep':
                it = {'k': 'keep', 'id': a.get('id', ''), 'kind': a.get('kind', ''),
                      'summary': a.get('summary', '')}
            else:
                ctx.add('unknown_tag', ln, 1, 'unknown tag <%s>. %s' % (tag['name'], ctx.hint()))
            if it:
                sh['items'].append(it)
            i += 1
        if group_open is not None:
            ctx.add('unclosed_tag', ln, 1, '<group> is not closed by </group>.')
            sh['items'].append(group_open)
        # B: coalesce consecutive shapes with the same group
        if ctx.decision == 'shapes' and ctx.cand == 'B':
            sh['items'] = coalesce_groups(sh['items'], ctx)
        sheets.append(sh)
    return sheets


def coalesce_groups(items, ctx):
    out, buf = [], None
    for it in items:
        if it.get('k') == 'shape' and it.get('group'):
            g = it['group']
            if buf and buf['name'] == g:
                buf['children'].append(dict(it, group=g))
            else:
                if buf:
                    out.append(buf)
                buf = {'k': 'group', 'name': g, 'children': [dict(it, group=g)]}
        else:
            if buf:
                out.append(buf)
                buf = None
            if it.get('k') == 'shape':
                it = dict(it, group=None)
            out.append(it)
    if buf:
        out.append(buf)
    return out


def render_shape_a(sh):
    if sh.get('text'):
        return '<shape name="%s" anchor="%s">%s</shape>' % (sh['name'], sh['anchor'], sh['text'])
    return '<shape name="%s" anchor="%s"/>' % (sh['name'], sh['anchor'])


def render_shape_b(sh, group=None):
    parts = ['name="%s"' % sh['name'], 'anchor="%s"' % sh['anchor']]
    if sh.get('text'):
        parts.append('text="%s"' % sh['text'])
    g = group or sh.get('group')
    if g:
        parts.append('group="%s"' % g)
    return '<shape %s/>' % ' '.join(parts)


def render_cf(it, cand):
    if it['type'] == 'cellIs':
        return ('<conditional-format range="%s" type="cellIs" operator="%s" formula="%s" dxf="%s"/>'
                % (it['range'], it['operator'], it['formula'], it['dxf']))
    if cand == 'A':
        return ('<conditional-format range="%s" type="dataBar" min="%s" max="%s" fill="%s"/>'
                % (it['range'], fmt_cfvo_a(it['min']), fmt_cfvo_a(it['max']), it['fill']))
    mn, mx = it['min'], it['max']
    bits = ['range="%s"' % it['range'], 'type="dataBar"', 'min-type="%s"' % mn[0]]
    if mn[1] is not None:
        bits.append('min="%s"' % mn[1])
    bits.append('max-type="%s"' % mx[0])
    if mx[1] is not None:
        bits.append('max="%s"' % mx[1])
    bits.append('color="%s"' % it['fill'])
    return '<conditional-format %s/>' % ' '.join(bits)


def render_item(it, cand):
    k = it['k']
    if k == 'table':
        lines = ['<table name="%s" range="%s">' % (it['name'], it['range']),
                 '| column | type | format | formula |', '|---|---|---|---|']
        for c in it['cols']:
            lines.append('| %s | %s | %s | %s |' % (c['name'], c['type'], c.get('format', ''), c.get('formula', '')))
        lines.append('</table>')
        return '\n'.join(lines)
    if k == 'merge':
        return '<merge range="%s"/>' % it['range']
    if k == 'hyperlink':
        return '<hyperlink range="%s" target="%s"/>' % (it['range'], it['target'])
    if k == 'note':
        return '<note range="%s" author="%s" text="%s"/>' % (it['range'], it['author'], it['text'])
    if k == 'validation':
        return '<validation range="%s" type="%s" formula1="%s"/>' % (it['range'], it['type'], it['formula1'])
    if k == 'cf':
        return render_cf(it, cand)
    if k == 'chart':
        return '<chart type="%s" data="%s" title="%s"/>' % (it['type'], it['data'], it['title'])
    if k == 'keep':
        return '<keep id="%s" kind="%s" summary="%s"/>' % (it['id'], it['kind'], it['summary'])
    if k == 'shape':
        return render_shape_a(it) if cand == 'A' else render_shape_b(it)
    if k == 'group':
        if cand == 'A':
            body = '\n'.join(render_shape_a(ch) for ch in it['children'])
            return '<group name="%s">\n%s\n</group>' % (it['name'], body)
        return '\n'.join(render_shape_b(ch, it['name']) for ch in it['children'])
    raise ValueError(k)


def render_book(model, cand):
    lines = [fm_text(model['fm']).rstrip('\n'), '']
    for sh in model['sheets']:
        lines.append('<sheet name="%s" range="%s">' % (sh['name'], sh['range']))
        lines.append('')
        for it in sh['items']:
            lines.append(render_item(it, cand))
            lines.append('')
        lines.append('</sheet>')
        lines.append('')
    return '\n'.join(lines).rstrip() + '\n'


# ---------------------------------------------------------------- public render / parse / semantic

def content_to_model(doc, decision):
    """Normalise a content.py seed (tuples) to the parsed-model shape."""
    if decision == 'breaks':
        blocks = []
        for b in doc:
            b = dict(b)
            if 'text' in b:
                b['text'] = inlines_of(b['text'])
            blocks.append(b)
        return {'fm': None, 'blocks': blocks}  # fm filled by caller
    return doc


def render(model, cand, decision):
    if decision == 'breaks':
        return render_doc(model, cand)
    return render_book(model, cand)


def parse(text, unit):
    ctx = Ctx(unit)
    fm, body = split_fm(text, ctx)
    if unit['decision'] == 'breaks':
        blocks = parse_doc(body, ctx)
        return {'fm': fm, 'blocks': blocks}, ctx
    sheets = parse_book(body, ctx)
    return {'fm': fm, 'sheets': sheets}, ctx


def semantic(model, decision):
    """Drop fm formatting noise; keep comparable structure."""
    if decision == 'breaks':
        return {'blocks': model['blocks']}
    return {'sheets': model['sheets']}


def wrap_doc(fm, blocks):
    return {'fm': fm, 'blocks': blocks}


def wrap_book(doc):
    return doc


# ---------------------------------------------------------------- scoring

def is_refusal(a):
    t = a.get('text')
    return isinstance(t, str) and t.strip().upper().startswith(REFUSE)


def load_unit(unit_id):
    with open(os.path.join(UNITS, unit_id + '.json'), encoding='utf-8') as f:
        return json.load(f)


def load_gold(unit_id):
    with open(os.path.join(GOLD, unit_id + '.json'), encoding='utf-8') as f:
        return {a['task_id']: a for a in json.load(f)['answers']}


def read_ok(got, want):
    g = ntext(got)
    if g.upper().startswith('ANSWER:'):
        g = ntext(g[7:])
    w = ntext(want)
    if w.upper().startswith('ANSWER:'):
        w = ntext(w[7:])
    return g == w or g.replace(' ', '') == w.replace(' ', '')


def score_answer(unit_id, answers):
    unit = load_unit(unit_id)
    gold = load_gold(unit_id)
    by_id = {}
    for a in answers.get('answers', []):
        if isinstance(a, dict) and 'task_id' in a:
            by_id[a['task_id']] = a
    results = []
    for task in unit['tasks']:
        tid = task['task_id']
        a = by_id.get(tid)
        res = {'task_id': tid, 'valid': False, 'landed': False, 'error': '', 'chars': 0, 'flags': [], 'detail': ''}
        results.append(res)
        if a is None:
            res['error'] = 'no answer for this task.'
            res['flags'] = ['missing_answer']
            continue
        g = gold[tid]
        gold_refuses = is_refusal(g)
        if is_refusal(a):
            res['chars'] = len(a['text'])
            if task['kind'] != 'refuse' and a.get('edits'):
                res['error'] = 'a refusal is "text": "REFUSE: <reason>" with "edits": []; this answer has both.'
                res['flags'] = ['bad_refusal']
                continue
            res['valid'] = True
            if gold_refuses:
                res['landed'] = True
            else:
                res['flags'] = ['wrong_refusal']
                res['detail'] = 'refused a task that can be done with this file\'s syntax and names.'
            continue
        if gold_refuses:
            res['flags'] = ['not_refused']
            res['detail'] = 'this task cannot be done; the answer should be REFUSE: …'
            res['chars'] = len(a.get('text') or '') + sum(
                len(e.get('old') or '') + len(e.get('new') or '') for e in (a.get('edits') or [])
                if isinstance(e, dict))
            continue
        if task['kind'] == 'read':
            t = a.get('text')
            res['chars'] = len(t or '')
            if not isinstance(t, str) or not t.strip().upper().startswith('ANSWER:'):
                res['error'] = 'a read answer is `ANSWER: …`.'
                res['flags'] = ['wrong_answer_kind']
                continue
            res['valid'] = True
            if read_ok(t, task['expect']):
                res['landed'] = True
            else:
                res['flags'] = ['not_landed']
                res['detail'] = 'got %s, want %s' % (ntext(t)[:120], ntext(task['expect'])[:120])
            continue
        if task['kind'] == 'write':
            t = a.get('text')
            res['chars'] = len(t or '')
            if not isinstance(t, str):
                res['error'] = 'a write task is answered with "text": the complete new file.'
                res['flags'] = ['wrong_answer_kind']
                continue
            model, ctx = parse(t, unit)
            if ctx.errors:
                res['error'] = ctx.error_text()
                res['flags'] = sorted(ctx.flags)
                continue
            res['valid'] = True
            want, _ = parse(g['text'], unit)
            if semantic(model, unit['decision']) == semantic(want, unit['decision']):
                res['landed'] = True
            else:
                res['flags'] = ['brief_unmet']
                res['detail'] = first_diff(semantic(model, unit['decision']), semantic(want, unit['decision']))
            continue
        # edit
        edits = a.get('edits')
        if not isinstance(edits, list):
            res['error'] = ('this is an edit task: answer with "edits", a list of {"old", "new"} pairs '
                            '(or refuse it with "text": "REFUSE: <reason>" and "edits": []).')
            res['flags'] = ['wrong_answer_kind']
            res['chars'] = len(a.get('text') or '')
            continue
        res['chars'] = sum(len(e.get('old') or '') + len(e.get('new') or '') for e in edits if isinstance(e, dict))
        text, flag, msg = apply_edits(unit['seed'], edits)
        if flag:
            res['error'] = msg
            res['flags'] = [flag]
            continue
        model, ctx = parse(text, unit)
        if ctx.errors:
            res['error'] = ctx.error_text()
            res['flags'] = sorted(ctx.flags)
            continue
        res['valid'] = True
        if text == unit['seed']:
            res['flags'] = ['no_change']
            res['detail'] = 'the edits left the file unchanged.'
            continue
        gtext, gflag, _ = apply_edits(unit['seed'], g['edits'])
        want, wctx = parse(gtext, unit)
        if gflag or wctx.errors:
            raise RuntimeError('gold answer for %s is broken: %s' % (tid, wctx.error_text() if wctx.errors else gflag))
        if semantic(model, unit['decision']) == semantic(want, unit['decision']):
            res['landed'] = True
        else:
            res['flags'] = ['not_landed']
            res['detail'] = first_diff(semantic(model, unit['decision']), semantic(want, unit['decision']))
    return results


def main():
    if len(sys.argv) != 3:
        print('usage: python3 score.py <unit_id> <answers.json>', file=sys.stderr)
        sys.exit(2)
    answers = json.load(open(sys.argv[2], encoding='utf-8'))
    print(json.dumps({'results': score_answer(sys.argv[1], answers)}, ensure_ascii=False, indent=1))


if __name__ == '__main__':
    main()
