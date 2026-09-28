#!/usr/bin/env python3
"""Scorer for round 2 of the hanji fluency test kit.

usage: python3 score.py <unit_id> <answers.json>

answers.json: {"answers": [{"task_id": ..., "text": ...} | {"task_id": ..., "edits": [{"old", "new"}]}]}
              an edit task may be refused: {"task_id": ..., "text": "REFUSE: <reason>", "edits": []}
prints:       {"results": [{"task_id", "valid", "landed", "error", "chars", "flags", "detail"}]}

Each candidate syntax is parsed into one common semantic model:
  Document     -> {"fm": {...}, "blocks": [heading | paragraph(style, text) | list item | table(style, grid)]}
                  a table grid is {"rows", "cols", "cells": [[r, c, text, rowspan, colspan, is_header], ...]}
                  (is_header: the cell starts in the first row)
  Presentation -> {"fm": {...}, "slides": [{"layout", "slots": {slot: [lines]}, "shapes": [[id, name, text]]}]}

Adapted from ../score.py (round 1); round 1 files are not changed.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
UNITS = os.path.join(HERE, 'data', 'units')
GOLD = os.path.join(HERE, 'data', 'gold')

INLINE_TAGS = {'u': set(), 'br': set(), 'field': {'name'}, 'keep': {'id', 'kind', 'summary'}}
TAG_RE = re.compile(r'<\s*(/?)\s*([A-Za-z][\w-]*)([^<>]*?)(/?)\s*>')
ATTR_RE = re.compile(r'([A-Za-z_:][\w:.-]*)(?:\s*=\s*(?:"([^"]*)"|\'([^\']*)\'|([^\s"\'>}]+)))?')
CSS_RE = re.compile(
    r'[:;#{}]|\b(colou?r|red|blue|green|yellow|orange|bold|bolder|italic|font|size|large|larger|big|small|'
    r'weight|underline|highlight|center|centre|right|left|align|background|border|thick|width|box|'
    r'\d+(\.\d+)?\s*(pt|px|em|rem|%))\b', re.I)
FM_KEYS = ['type', 'format', 'template', 'schema']
ALL_SLOTS = ['title', 'body', 'left', 'right', 'notes']
MAX_ERRORS = 6
REFUSE_PREFIX = 'REFUSE:'


def norm(s):
    return re.sub(r'\s+', ' ', s).strip()


def ntext(s):
    """Text as compared: whitespace collapsed, **bold** markers ignored."""
    return norm(s.replace('**', ''))


def parse_attrs(s):
    out = {}
    for m in ATTR_RE.finditer(s or ''):
        v = m.group(2) if m.group(2) is not None else (m.group(3) if m.group(3) is not None else m.group(4))
        out[m.group(1).lower()] = v if v is not None else ''
    return out


def quoted_list(names):
    return ', '.join('"%s"' % n for n in names)


class Ctx:
    def __init__(self, unit, shapes_allowed=()):
        self.unit = unit
        self.decision = d = unit['decision']
        self.cand = c = unit['candidate']
        self.names = unit.get('names', {})
        self.shapes_allowed = set(shapes_allowed)
        self.errors = []          # (flag, line, col, message)
        self.flags = set()
        # which constructs this candidate has
        self.markers = (d == 'merge' and c == 'B') or d == 'tablestyle'
        self.html_tables = d == 'merge' and c == 'A'
        self.div_attr = {'styleattr': 'style' if c == 'A' else 'class', 'tablestyle': 'style'}.get(d)
        self.wrap_attr = ('style' if c == 'A' else 'class') if d == 'styleattr' else (
            'style' if d == 'tablestyle' and c == 'A' else None)
        self.attr_line = d == 'tablestyle' and c == 'B'
        self.block_tags = {'keep', 'pagebreak'} if d != 'slides' else set()
        if self.html_tables or self.wrap_attr:
            self.block_tags.add('table')
        if self.div_attr:
            self.block_tags.add('div')

    def add(self, flag, line, col, msg):
        self.flags.add(flag)
        self.errors.append((flag, line, col, msg))

    def table_style_form(self):
        if self.wrap_attr:
            return 'a line <table %s="Name">, then the pipe table, then a line </table>' % self.wrap_attr
        if self.attr_line:
            return 'a line {style="Name"} directly before the header row of the pipe table'
        return None

    def tag_hint(self):
        d = self.decision
        if d == 'merge' and self.cand == 'A':
            return ('Tables are pipe tables, or <table> with <tr>, <th>, <td> (rowspan="N", colspan="N") '
                    'when cells are merged.')
        if d == 'merge':
            return ('Tables are pipe tables; a merged cell is written ^^ (merged into the cell above) '
                    'or || (the cell to the left extends into this column).')
        if d in ('styleattr', 'tablestyle'):
            return ('Formatting is by style name only: <div %s="Name">text</div> for a paragraph, and for a '
                    'table %s; colours, fonts, sizes and borders cannot be written.'
                    % (self.div_attr, self.table_style_form()))
        if self.cand == 'A':
            return ('A slide is <slide layout="Name"> ... </slide> with slot tags such as <title>, <body>, '
                    '<left>, <right>, <notes>, and shape lines <shape id=".." name="..">text</shape>.')
        return ('Slides are separated by a line ---, begin with a line layout: Name, and use slot markers '
                'such as ::title::, ::body::, ::left::, ::right::, ::notes::, and shape lines '
                '<shape id=".." name="..">text</shape>.')

    def error_text(self):
        out = []
        for flag, line, col, msg in self.errors[:MAX_ERRORS]:
            out.append('line %d, column %d: %s' % (line, col, msg))
        if len(self.errors) > MAX_ERRORS:
            out.append('(%d more errors)' % (len(self.errors) - MAX_ERRORS))
        return '\n'.join(out)


# ---------------------------------------------------------------- common pieces

def parse_front_matter(lines, ctx, want_type):
    form = ('a line ---, the lines type: ..., format: ..., template: ..., schema: ..., and a closing line ---. '
            'Keep the front matter as given.')
    if not lines or lines[0].strip() != '---':
        ctx.add('front_matter', 1, 1, 'The file must begin with its front matter block: ' + form)
        return {}, 0
    fm = {}
    for i in range(1, len(lines)):
        s = lines[i].strip()
        if s == '---':
            for k in FM_KEYS:
                if k not in fm:
                    ctx.add('front_matter', i + 1, 1, 'the front matter has no "%s:" line. It is %s' % (k, form))
            if fm.get('type') and fm['type'] != want_type:
                ctx.add('front_matter', 2, 1, 'this file is type: %s, not "%s".' % (want_type, fm['type']))
            return fm, i + 1
        m = re.match(r'^([A-Za-z_][\w-]*)\s*:\s*(.*)$', s)
        if m:
            fm[m.group(1)] = m.group(2).strip()
        elif s:
            ctx.add('front_matter', i + 1, 1, 'front matter lines are "key: value". It is ' + form)
    ctx.add('front_matter', 1, 1, 'the front matter is not closed by a line ---.')
    return fm, len(lines)


def check_inline(text, line, col0, ctx):
    for m in TAG_RE.finditer(text):
        name = m.group(2).lower()
        col = col0 + m.start()
        attrs = parse_attrs(m.group(3))
        if name not in INLINE_TAGS:
            ctx.add('unknown_tag', line, col, '<%s> is not a tag of this format. %s' % (name, ctx.tag_hint()))
            if 'style' in attrs or 'color' in attrs or any(CSS_RE.search(v or '') for v in attrs.values()):
                ctx.flags.add('css_in_attr')
            continue
        for a in attrs:
            if a not in INLINE_TAGS[name]:
                ctx.add('invented_attr', line, col, '<%s> has no attribute "%s".' % (name, a))
                if CSS_RE.search(attrs[a] or '') or a == 'style':
                    ctx.flags.add('css_in_attr')


def check_style_value(value, kind, attr, line, col, ctx):
    allowed = ctx.names.get('paragraph_styles' if kind == 'paragraph' else 'table_styles', [])
    other = ctx.names.get('table_styles' if kind == 'paragraph' else 'paragraph_styles', [])
    if value in allowed:
        return
    tail = (' The value is exactly one style name, spaces included. Allowed %s styles: %s.'
            % (kind, quoted_list(allowed)))
    if value in other:
        ctx.add('unknown_style', line, col, '%s="%s" is a %s style; a %s takes a %s style.%s'
                % (attr, value, 'table' if kind == 'paragraph' else 'paragraph', kind, kind, tail))
        return
    def squash(x):
        return re.sub(r'[\s_\-.]+', '', x).lower()
    toks = value.split()
    split = []
    for n in allowed:
        nt = n.split()
        if value.lower() == n.lower():
            continue
        if squash(value) == squash(n) or (toks and len(toks) < len(nt) and all(t in nt for t in toks)):
            split.append(n)
    case = [n for n in allowed if n.lower() == value.lower()]
    if case:
        ctx.add('unknown_style', line, col, '%s="%s" is not a %s style of this file; names are case-sensitive: '
                '"%s".%s' % (attr, value, kind, case[0], tail))
    elif split:
        ctx.add('split_name', line, col, '%s="%s" is not a %s style of this file; did you mean "%s"?%s'
                % (attr, value, kind, split[0], tail))
    elif CSS_RE.search(value):
        ctx.add('css_in_attr', line, col,
                '%s="%s" is direct formatting. Formatting is by style name only; colours, fonts, sizes and borders '
                'cannot be written. Use a %s style of this file, or refuse the task if none fits.%s'
                % (attr, value, kind, tail))
    else:
        ctx.add('unknown_style', line, col, '%s="%s" is not a %s style of this file.%s' % (attr, value, kind, tail))


def check_block_attrs(elem, attrs, attr, line, col, ctx):
    """Validates the attributes of <div> / <table>; attr is the one allowed attribute (or None).
    Returns the style name (or None)."""
    kind = 'paragraph' if elem == 'div' else 'table'
    for k, v in attrs.items():
        if attr and k == attr:
            continue
        if attr:
            msg = ('<%s> has no attribute "%s". Its only attribute is %s="Name", where Name is one %s style '
                   'of this file.' % (elem, k, attr, kind))
        else:
            msg = '<%s> takes no attributes; "%s" is not allowed here.' % (elem, k)
        ctx.add('invented_attr', line, col, msg)
        if k in ('style', 'color', 'bgcolor', 'border', 'width', 'align') or CSS_RE.search(v or ''):
            ctx.flags.add('css_in_attr')
    if attr and attr in attrs:
        check_style_value(attrs[attr], kind, attr, line, col, ctx)
        return attrs[attr]
    if attr:
        ctx.add('missing_attr', line, col, '<%s> needs %s="Name" with one %s style of this file.' % (elem, attr, kind))
    return None


# ---------------------------------------------------------------- tables

def build_html_grid(rows, ctx):
    """rows: [(line, [cell dict(text, rs, cs, line, col)])] -> grid or None."""
    occ = {}
    origins = []
    ok = True
    for r, (rline, cells) in enumerate(rows):
        c = 0
        for cell in cells:
            while (r, c) in occ:
                c += 1
            for dr in range(cell['rs']):
                for dc in range(cell['cs']):
                    p = (r + dr, c + dc)
                    if p in occ:
                        ctx.add('counted_span_wrong', cell['line'], cell['col'],
                                'the cell "%s" (row %d) overlaps a cell already covered by a rowspan or colspan. '
                                'In the rows below a rowspan, leave the covered cell out.' % (cell['text'], r + 1))
                        ok = False
                    occ[p] = True
            origins.append([r, c, cell['text'], cell['rs'], cell['cs'], r == 0])
            c += cell['cs']
    R = len(rows)
    if R == 0:
        ctx.add('counted_span_wrong', 1, 1, 'the table has no rows.')
        return None
    W = len([p for p in occ if p[0] == 0])
    for (o_r, o_c, text, rs, cs, h) in origins:
        if o_r + rs > R:
            ctx.add('counted_span_wrong', rows[o_r][0], 1,
                    'the rowspan="%d" of the cell "%s" (row %d) reaches past the last row of the table.'
                    % (rs, text, o_r + 1))
            ok = False
    for r in range(R):
        covered = sorted(c for (rr, c) in occ if rr == r)
        if covered != list(range(W)):
            ctx.add('counted_span_wrong', rows[r][0], 1,
                    'row %d of the table covers %d columns, but the first row covers %d. In each row, the cells '
                    'written, plus their colspan values, plus the cells still covered by a rowspan from the rows '
                    'above, must add up to the width of the table.' % (r + 1, len(covered), W))
            ok = False
    if not ok:
        return None
    return {'rows': R, 'cols': W, 'cells': sorted(origins)}


def parse_html_table(chunk, start_line, ctx):
    def lc(off):
        line = start_line + chunk.count('\n', 0, off)
        col = off - (chunk.rfind('\n', 0, off) + 1) + 1
        return line, col

    struct = {'table', 'thead', 'tbody', 'tfoot', 'tr', 'td', 'th'}
    toks = [m for m in TAG_RE.finditer(chunk) if m.group(2).lower() in struct]
    pos = 0
    rows = []
    cur_row = None
    cur_cell = None
    seen_open = False
    closed = False
    before = len(ctx.errors)
    for m in toks:
        closing = m.group(1) == '/'
        name = m.group(2).lower()
        between = chunk[pos:m.start()]
        if cur_cell is not None:
            l, c = lc(cur_cell['start'])
            check_inline(between, l, c, ctx)
            cur_cell['text'] = ntext(between)
            cur_cell = None
        elif between.strip():
            l, c = lc(pos + len(between) - len(between.lstrip()))
            ctx.add('text_outside_cell', l, c, 'text outside a cell: all text of a <table> is inside <th> or <td>.')
        attrs = parse_attrs(m.group(3))
        l, c = lc(m.start())
        pos = m.end()
        if name == 'table':
            if closing:
                closed = True
                break
            if seen_open:
                ctx.add('unclosed_tag', l, c, 'a <table> cannot contain another <table>.')
                continue
            seen_open = True
            check_block_attrs('table', attrs, None, l, c, ctx)
        elif name in ('thead', 'tbody', 'tfoot'):
            for k in attrs:
                ctx.add('invented_attr', l, c, '<%s> takes no attributes; "%s" is not allowed.' % (name, k))
        elif name == 'tr':
            if closing:
                cur_row = None
                continue
            for k in attrs:
                ctx.add('invented_attr', l, c, '<tr> takes no attributes; "%s" is not allowed.' % k)
            cur_row = (l, [])
            rows.append(cur_row)
        else:  # td / th
            if closing:
                continue
            if cur_row is None:
                ctx.add('text_outside_cell', l, c, '<%s> must be inside a <tr>.' % name)
                cur_row = (l, [])
                rows.append(cur_row)
            spans = {'rowspan': 1, 'colspan': 1}
            for k, v in attrs.items():
                if k in spans:
                    if v is not None and re.fullmatch(r'\s*[1-9]\d*\s*', v):
                        spans[k] = int(v)
                    else:
                        ctx.add('counted_span_wrong', l, c, '%s="%s" must be a whole number of 1 or more.' % (k, v))
                else:
                    ctx.add('invented_attr', l, c, '<%s> has no attribute "%s". A cell takes only rowspan="N" '
                            'and colspan="N".' % (name, k))
                    if k == 'style' or CSS_RE.search(v or ''):
                        ctx.flags.add('css_in_attr')
            cur_cell = {'text': '', 'rs': spans['rowspan'], 'cs': spans['colspan'],
                        'line': l, 'col': c, 'start': m.end()}
            cur_row[1].append(cur_cell)
    if not seen_open:
        ctx.add('unclosed_tag', start_line, 1, 'expected <table> here.')
        return None
    if not closed:
        ctx.add('unclosed_tag', start_line, 1, '<table> is not closed by </table>.')
        return None
    rest = chunk[pos:]
    if rest.strip():
        l, c = lc(pos)
        ctx.add('text_outside_cell', l, c, 'nothing may follow </table> on the same line.')
    if len(ctx.errors) > before:
        return None
    grid = build_html_grid(rows, ctx)
    if grid is None:
        return None
    return {'k': 'table', 'style': None, 'grid': grid}


def split_row(s):
    inner = s[1:-1]
    cells = []
    buf = ''
    start = 2
    i = 0
    while i < len(inner):
        ch = inner[i]
        if ch == '\\' and i + 1 < len(inner) and inner[i + 1] == '|':
            buf += '|'
            i += 2
            continue
        if ch == '|':
            cells.append((buf, start))
            buf = ''
            start = i + 3
            i += 1
            continue
        buf += ch
        i += 1
    cells.append((buf, start))
    return cells


LEFT, UP = object(), object()


def parse_pipe_table(tlines, start_line, ctx):
    markers = ctx.markers
    rows = []
    for k, raw in enumerate(tlines):
        ln = start_line + k
        s = raw.strip()
        lead = len(raw) - len(raw.lstrip())
        if len(s) < 2 or not s.endswith('|') or s.endswith('\\|'):
            ctx.add('pipe_row', ln, len(raw.rstrip()) + 1, 'a table row starts and ends with |.')
            return None
        rows.append((ln, lead, split_row(s)))
    if len(rows) < 2:
        ctx.add('pipe_row', start_line, 1, 'a pipe table needs a header row, then a delimiter row such as |---|---|.')
        return None
    hdr, delim = rows[0], rows[1]
    W = len(hdr[2])
    ok = True
    for text, col in delim[2]:
        if not re.fullmatch(r'\s*:?-+:?\s*', text):
            ctx.add('pipe_delimiter', delim[0], delim[1] + col, 'the second line of a table is the delimiter row, '
                    'with one --- per column, such as |---|---|---|.')
            return None
    if len(delim[2]) != W:
        ctx.add('row_width_mismatch', delim[0], 1, 'the delimiter row has %d cells but the header row has %d. '
                'Write one --- per column.' % (len(delim[2]), W))
        ok = False
    grid_rows = [hdr] + rows[2:]
    for ln, lead, cells in grid_rows:
        if len(cells) != W:
            if markers:
                msg = ('this row has %d cells but the header row has %d. Every row has one cell per column: a cell '
                       'merged into the one above is written ^^, and a column covered by the cell to its left is '
                       'written || (nothing between the pipes).' % (len(cells), W))
            else:
                msg = 'this row has %d cells but the header row has %d. Every row has one cell per column.' % (
                    len(cells), W)
            ctx.add('row_width_mismatch', ln, 1, msg)
            ok = False
    if not ok:
        return None
    toks = []
    for ln, lead, cells in grid_rows:
        row = []
        for text, col in cells:
            if markers and text == '':
                row.append(LEFT)
            elif markers and text.strip() == '^^':
                row.append(UP)
            else:
                check_inline(text, ln, lead + col, ctx)
                row.append(ntext(text))
        toks.append(row)
    R = len(toks)
    origin = {}
    for r in range(R):
        for c in range(W):
            t = toks[r][c]
            ln = grid_rows[r][0]
            col = grid_rows[r][1] + grid_rows[r][2][c][1]
            if t is LEFT:
                if c == 0:
                    ctx.add('marker_misplaced', ln, col, '|| at the start of a row: there is no cell to the left to '
                            'extend. An empty cell is written with a space: |  |.')
                    ok = False
                    origin[(r, c)] = (r, c)
                else:
                    origin[(r, c)] = origin[(r, c - 1)]
            elif t is UP:
                if r == 0:
                    ctx.add('marker_misplaced', ln, col, '^^ in the header row: there is no cell above to merge into.')
                    ok = False
                    origin[(r, c)] = (r, c)
                else:
                    origin[(r, c)] = origin[(r - 1, c)]
            else:
                origin[(r, c)] = (r, c)
    if not ok:
        return None
    regions = {}
    for p, o in origin.items():
        regions.setdefault(o, []).append(p)
    cells = []
    for o, ps in regions.items():
        rs = max(p[0] for p in ps) - min(p[0] for p in ps) + 1
        cs = max(p[1] for p in ps) - min(p[1] for p in ps) + 1
        if len(ps) != rs * cs or (min(p[0] for p in ps), min(p[1] for p in ps)) != o:
            t = toks[o[0]][o[1]]
            ctx.add('merge_not_rectangular', grid_rows[o[0]][0], 1,
                    'the merged area of the cell "%s" (row %d, column %d) is not a rectangle. Every cell it covers '
                    'is ^^ (the cell above belongs to it) or || (the cell to the left belongs to it).'
                    % (t if isinstance(t, str) else '', o[0] + 1, o[1] + 1))
            return None
        t = toks[o[0]][o[1]]
        cells.append([o[0], o[1], t if isinstance(t, str) else '', rs, cs, o[0] == 0])
    return {'k': 'table', 'style': None, 'grid': {'rows': R, 'cols': W, 'cells': sorted(cells)}}


def pipe_extent(lines, i):
    j = i
    while j < len(lines) and lines[j].strip().startswith('|'):
        j += 1
    return j


def parse_wrapped_table(lines, i, ctx):
    """<table ATTR="Name"> line, pipe table, </table> line. Returns (block or None, next index)."""
    n = len(lines)
    raw = lines[i]
    s = raw.strip()
    ln = i + 1
    col0 = len(raw) - len(raw.lstrip()) + 1
    form = ('A table with a style is %s; nothing else is between the two lines, not even a blank line.'
            % ctx.table_style_form())
    # find the closing line
    k = i + 1
    close = None
    while k < n:
        t = lines[k].strip().lower().replace(' ', '')
        if t == '</table>':
            close = k
            break
        if t.startswith('<table'):
            break
        k += 1
    m = re.match(r'^<\s*table\b([^>]*)>$', s)
    if not m:
        ctx.add('wrapper_form', ln, col0, 'the <table ...> line stands alone on its line. ' + form)
        return None, (close + 1 if close is not None else i + 1)
    style = check_block_attrs('table', parse_attrs(m.group(1)), ctx.wrap_attr, ln, col0, ctx)
    if close is None:
        ctx.add('unclosed_tag', ln, col0, '<table> opened here is not closed by a line </table>. ' + form)
        return None, pipe_extent(lines, i + 1)
    j = pipe_extent(lines, i + 1)
    if j == i + 1 or j != close:
        bad = j if j != i + 1 else i + 1
        ctx.add('wrapper_content', bad + 1, 1, 'between <table ...> and </table> goes exactly one pipe table: '
                'the header row directly after the <table ...> line and the </table> line directly after the last '
                'row. ' + form)
        return None, close + 1
    b = parse_pipe_table(lines[i + 1:j], i + 2, ctx)
    if b:
        b['style'] = style
    return b, close + 1


def parse_attr_line(s, ln, col0, ctx):
    """{style="Name"} -> style or None (errors recorded)."""
    inner = s.strip()[1:-1].strip()
    form = 'A table style line is exactly {style="Name"}, directly before the header row of the table.'
    m = re.fullmatch(r'style\s*=\s*"([^"]*)"', inner)
    if m:
        check_style_value(m.group(1), 'table', 'style', ln, col0, ctx)
        return m.group(1)
    toks = inner.split()
    if any(t.startswith('.') or t.startswith('#') for t in toks) or not inner:
        ctx.add('attr_form', ln, col0, 'the braces hold only style="Name". ' + form)
        return None
    attrs = parse_attrs(inner)
    if not attrs:
        ctx.add('attr_form', ln, col0, form)
        return None
    style = None
    for k, v in attrs.items():
        if k == 'style':
            check_style_value(v, 'table', 'style', ln, col0, ctx)
            style = v
        else:
            ctx.add('invented_attr', ln, col0, '"%s" is not allowed in a table style line. %s' % (k, form))
            if CSS_RE.search(v or '') or k in ('color', 'border', 'width', 'align', 'bgcolor'):
                ctx.flags.add('css_in_attr')
    if 'style' not in attrs:
        ctx.add('missing_attr', ln, col0, form)
    return style


# ---------------------------------------------------------------- document

def parse_document(text, ctx):
    lines = text.split('\n')
    fm, i = parse_front_matter(lines, ctx, 'document')
    blocks = []
    n = len(lines)
    while i < n:
        raw = lines[i]
        s = raw.strip()
        ln = i + 1
        col0 = len(raw) - len(raw.lstrip()) + 1
        if not s:
            i += 1
            continue
        if s.startswith('|'):
            j = pipe_extent(lines, i)
            b = parse_pipe_table(lines[i:j], ln, ctx)
            if b:
                blocks.append(b)
            i = j
            continue
        if ctx.attr_line and s.startswith('{') and s.endswith('}'):
            style = parse_attr_line(s, ln, col0, ctx)
            if i + 1 < n and lines[i + 1].strip().startswith('|'):
                j = pipe_extent(lines, i + 1)
                b = parse_pipe_table(lines[i + 1:j], ln + 1, ctx)
                if b:
                    b['style'] = style
                    blocks.append(b)
                i = j
            else:
                ctx.add('attr_orphan', ln, col0, 'a {style="Name"} line must be directly followed by the header row '
                        'of its table, with no blank line or other text between.')
                i += 1
            continue
        m = re.match(r'<\s*/?\s*([A-Za-z][\w-]*)', s)
        if m:
            tag = m.group(1).lower()
            if tag == 'table' and 'table' in ctx.block_tags:
                if s.startswith('</'):
                    ctx.add('wrapper_content', ln, col0, '</table> without a <table ...> line before its table.')
                    i += 1
                    continue
                if ctx.html_tables:
                    j = i
                    while j < n and '</table>' not in lines[j].lower().replace(' ', ''):
                        j += 1
                    if j >= n:
                        ctx.add('unclosed_tag', ln, col0, '<table> opened here is not closed by </table>.')
                        break
                    b = parse_html_table('\n'.join(lines[i:j + 1]), ln, ctx)
                    if b:
                        blocks.append(b)
                    i = j + 1
                    continue
                b, i = parse_wrapped_table(lines, i, ctx)
                if b:
                    blocks.append(b)
                continue
            if tag == 'div' and 'div' in ctx.block_tags:
                j = i
                while j < n and '</div>' not in lines[j]:
                    j += 1
                if j >= n:
                    ctx.add('unclosed_tag', ln, col0, '<div> opened here is not closed by </div>.')
                    break
                chunk = '\n'.join(lines[i:j + 1])
                dm = re.match(r'^\s*<\s*div\b([^>]*)>(.*)</div>\s*$', chunk, re.S)
                if not dm or j != i:
                    ctx.add('div_form', ln, col0, 'expected <div %s="Name">text</div> on one line, with nothing '
                            'after </div>.' % ctx.div_attr)
                else:
                    style = check_block_attrs('div', parse_attrs(dm.group(1)), ctx.div_attr, ln, col0, ctx)
                    body = dm.group(2)
                    if re.search(r'<\s*div\b', body):
                        ctx.add('unclosed_tag', ln, col0, 'a <div> cannot contain another <div>.')
                    check_inline(body, ln, col0 + dm.start(2), ctx)
                    blocks.append({'k': 'p', 'style': style, 'text': ntext(body)})
                i = j + 1
                continue
            if tag == 'keep' and 'keep' in ctx.block_tags:
                km = re.match(r'^<\s*keep\b([^>]*?)/\s*>$', s)
                if not km:
                    ctx.add('unclosed_tag', ln, col0, 'a placeholder is written <keep id="..." .../>.')
                else:
                    blocks.append({'k': 'keep', 'attrs': parse_attrs(km.group(1))})
                i += 1
                continue
            if tag == 'pagebreak' and 'pagebreak' in ctx.block_tags:
                blocks.append({'k': 'pagebreak'})
                i += 1
                continue
            if tag not in INLINE_TAGS:
                ctx.add('unknown_tag', ln, col0, '<%s> is not a tag of this format. %s' % (tag, ctx.tag_hint()))
                end = s.find('>')
                attrs = parse_attrs(s[m.end():end if end >= 0 else len(s)])
                if 'style' in attrs or 'color' in attrs or any(CSS_RE.search(v or '') for v in attrs.values()):
                    ctx.flags.add('css_in_attr')
                close = '</%s>' % tag
                j = i
                while j < n and close not in lines[j].lower():
                    j += 1
                i = (j + 1) if j < n else (i + 1)
                continue
        hm = re.match(r'^(#{1,6})\s+(.*)$', s)
        if hm:
            check_inline(hm.group(2), ln, col0, ctx)
            blocks.append({'k': 'h', 'level': len(hm.group(1)), 'text': ntext(hm.group(2))})
            i += 1
            continue
        lm = re.match(r'^(\s*)([-*+]|\d+[.)])\s+(.*)$', raw)
        if lm:
            check_inline(lm.group(3), ln, col0, ctx)
            blocks.append({'k': 'li', 'ordered': lm.group(2)[0].isdigit(), 'level': len(lm.group(1)) // 2,
                           'text': ntext(lm.group(3))})
            i += 1
            continue
        fm2 = re.match(r'^\[\^([^\]]+)\]:\s*(.*)$', s)
        if fm2:
            blocks.append({'k': 'fn', 'id': fm2.group(1), 'text': ntext(fm2.group(2))})
            i += 1
            continue
        check_inline(s, ln, col0, ctx)
        blocks.append({'k': 'p', 'style': None, 'text': ntext(s)})
        i += 1
    return {'fm': fm, 'blocks': blocks}


# ---------------------------------------------------------------- presentation

SHAPE_RE = re.compile(r'^<\s*shape\b([^>]*)>(.*)<\s*/\s*shape\s*>$')


def slot_line(l):
    l = norm(l)
    l = re.sub(r'^[*+]\s+', '- ', l)
    l = re.sub(r'^\d+[.)]\s+', '1. ', l)
    return ntext(l)


def check_slot(name, layout, slots, ln, col, ctx):
    layouts = ctx.names.get('layouts', {})
    if name not in ALL_SLOTS:
        allowed = (layouts.get(layout, []) + ['notes']) if layout in layouts else ALL_SLOTS
        ctx.add('unknown_slot', ln, col, '"%s" is not a slot. The slots of this slide are: %s.'
                % (name, ', '.join(allowed)))
        return False
    if layout in layouts and name != 'notes' and name not in layouts[layout]:
        ctx.add('unknown_slot', ln, col, 'layout "%s" has no slot "%s". Its slots are: %s.'
                % (layout, name, ', '.join(layouts[layout] + ['notes'])))
        return False
    if name in slots:
        ctx.add('duplicate_slot', ln, col, 'the slot "%s" appears twice in this slide; write it once.' % name)
        return False
    return True


def check_layout(layout, ln, col, ctx):
    layouts = ctx.names.get('layouts', {})
    if layout not in layouts:
        ctx.add('unknown_layout', ln, col, 'layout "%s" is not a layout of this file. Layouts: %s.'
                % (layout, quoted_list(list(layouts))))


def parse_shape(s, ln, ctx, seen_ids):
    m = SHAPE_RE.match(s.strip())
    if not m:
        ctx.add('shape_form', ln, 1, 'a shape is one line <shape id="..." name="...">text</shape>.')
        return None
    attrs = parse_attrs(m.group(1))
    for k in attrs:
        if k not in ('id', 'name'):
            ctx.add('invented_attr', ln, 1, '<shape> has no attribute "%s"; it has id and name only.' % k)
    if 'id' not in attrs or 'name' not in attrs:
        ctx.add('shape_form', ln, 1, 'a shape line keeps its id="..." and name="..." as they are in the file.')
        return None
    sid = attrs['id']
    if sid not in ctx.shapes_allowed:
        ctx.add('invented_shape', ln, 1, 'shape id="%s" is not a shape of this file. Shapes come from the '
                'imported file: keep them or delete them, but never add one.' % sid)
    if sid in seen_ids:
        ctx.add('duplicate_shape', ln, 1, 'shape id="%s" appears twice; each shape is written once.' % sid)
    seen_ids.add(sid)
    check_inline(m.group(2), ln, 1, ctx)
    return [sid, attrs['name'], ntext(m.group(2))]


def finish_slots(slots, ctx):
    out = {}
    for k, (ln, lines) in slots.items():
        vals = []
        for off, l in enumerate(lines):
            check_inline(l, ln + off, 1, ctx)
            if l.strip():
                vals.append(slot_line(l))
        if vals:
            out[k] = vals
    return out


def parse_slides_A(text, ctx):
    lines = text.split('\n')
    fm, i = parse_front_matter(lines, ctx, 'presentation')
    n = len(lines)
    slides = []
    seen_ids = set()
    while i < n:
        s = lines[i].strip()
        ln = i + 1
        if not s:
            i += 1
            continue
        m = re.match(r'^<\s*slide\b([^>]*)>$', s)
        if not m:
            ctx.add('text_outside_slide', ln, 1, 'expected <slide layout="Name">. Every slide is '
                    '<slide layout="Name"> ... </slide>, and all text is inside a slot tag of a slide.')
            i += 1
            continue
        attrs = parse_attrs(m.group(1))
        layout = attrs.get('layout')
        for k in attrs:
            if k != 'layout':
                ctx.add('invented_attr', ln, 1, '<slide> has no attribute "%s"; its only attribute is layout="Name".'
                        % k)
        if layout is None:
            ctx.add('missing_layout', ln, 1, '<slide> needs layout="Name". Layouts: %s.'
                    % quoted_list(list(ctx.names.get('layouts', {}))))
        else:
            check_layout(layout, ln, 1, ctx)
        i += 1
        slots = {}
        shapes = []
        closed = False
        while i < n:
            raw = lines[i]
            s = raw.strip()
            ln = i + 1
            if s == '</slide>':
                closed = True
                i += 1
                break
            if not s:
                i += 1
                continue
            if re.match(r'^<\s*slide\b', s):
                break
            if re.match(r'^<\s*shape\b', s):
                sh = parse_shape(s, ln, ctx, seen_ids)
                if sh:
                    shapes.append(sh)
                i += 1
                continue
            tm = re.match(r'^<\s*([A-Za-z][\w-]*)([^>]*)>(.*)$', s)
            if not tm or tm.group(1).lower() in INLINE_TAGS:
                ctx.add('text_outside_slot', ln, 1, 'text outside a slot. Put it inside a slot tag such as '
                        '<body> ... </body>.')
                i += 1
                continue
            name = tm.group(1).lower()
            for k in parse_attrs(tm.group(2)):
                ctx.add('invented_attr', ln, 1, '<%s> takes no attributes; "%s" is not allowed.' % (name, k))
            close = '</%s>' % name
            rest = tm.group(3)
            content = []
            start_ln = ln
            i += 1
            if rest.rstrip().endswith(close):
                content.append(rest.rstrip()[:-len(close)])
            else:
                if rest.strip():
                    content.append(rest)
                else:
                    start_ln = ln + 1
                ended = False
                while i < n:
                    l2 = lines[i].rstrip()
                    if l2.strip() == close:
                        ended = True
                        i += 1
                        break
                    if l2.endswith(close):
                        content.append(l2[:-len(close)])
                        ended = True
                        i += 1
                        break
                    if l2.strip() == '</slide>' or re.match(r'^\s*<\s*slide\b', l2):
                        break
                    content.append(l2)
                    i += 1
                if not ended:
                    ctx.add('unclosed_tag', ln, 1, '<%s> is not closed by %s.' % (name, close))
            if check_slot(name, layout, slots, ln, 1, ctx):
                slots[name] = (start_ln, content)
        if not closed:
            ctx.add('unclosed_tag', ln, 1, '<slide> is not closed by </slide>.')
        slides.append({'layout': layout, 'slots': finish_slots(slots, ctx), 'shapes': shapes})
    return {'fm': fm, 'slides': slides}


def parse_slides_B(text, ctx):
    lines = text.split('\n')
    fm, i = parse_front_matter(lines, ctx, 'presentation')
    n = len(lines)
    segs = []
    cur = []
    sep_lines = []
    for idx in range(i, n):
        if lines[idx].strip() == '---':
            segs.append(cur)
            sep_lines.append(idx + 1)
            cur = []
        else:
            cur.append((idx + 1, lines[idx]))
    segs.append(cur)
    slides = []
    seen_ids = set()
    layout_names = quoted_list(list(ctx.names.get('layouts', {})))
    for si, seg in enumerate(segs):
        nonblank = [(ln, l) for ln, l in seg if l.strip()]
        where = sep_lines[si] if si < len(sep_lines) else (sep_lines[-1] if sep_lines else i + 1)
        if not nonblank:
            ctx.add('empty_slide', where, 1, 'empty slide: nothing between two --- lines (or after the last one). '
                    'A slide is separated from the next by one line ---; its first line is layout: Name, with no '
                    'closing ---.')
            continue
        ln0, first = nonblank[0]
        m = re.match(r'^\s*layout\s*:\s*(.*?)\s*$', first)
        layout = None
        if not m:
            ctx.add('missing_layout', ln0, 1, 'each slide begins with a line layout: Name (right after the --- '
                    'that separates it from the previous slide; the first slide right after the front matter). '
                    'Layouts: %s.' % layout_names)
        else:
            layout = m.group(1).strip()
            if len(layout) >= 2 and layout[0] == layout[-1] and layout[0] in '"\'':
                layout = layout[1:-1]
            check_layout(layout, ln0, 1, ctx)
        cur_slot = None
        seen = {}
        shapes = []
        for ln, l in seg:
            if m and ln <= ln0:
                continue
            s = l.strip()
            mm = re.match(r'^::\s*([A-Za-z][\w-]*)\s*::\s*(.*)$', s)
            if mm:
                name = mm.group(1).lower()
                if check_slot(name, layout, seen, ln, 1, ctx):
                    seen[name] = (ln + 1, [])
                    cur_slot = name
                    if mm.group(2).strip():
                        seen[name] = (ln, [mm.group(2)])
                else:
                    cur_slot = None
                continue
            if re.match(r'^<\s*shape\b', s):
                sh = parse_shape(s, ln, ctx, seen_ids)
                if sh:
                    shapes.append(sh)
                cur_slot = None
                continue
            if cur_slot is None:
                if not s:
                    continue
                if re.match(r'^[A-Za-z_][\w-]*\s*:', s) and not seen and not shapes:
                    ctx.add('invented_attr', ln, 1, '"%s" is not allowed: a slide has one line layout: Name and '
                            'then slot markers.' % s.split(':')[0].strip())
                else:
                    ctx.add('text_outside_slot', ln, 1, 'text outside a slot. Text belongs after a marker line such '
                            'as ::body::; a shape line ends the slot before it.')
                continue
            seen[cur_slot][1].append(l)
        slides.append({'layout': layout, 'slots': finish_slots(seen, ctx), 'shapes': shapes})
    return {'fm': fm, 'slides': slides}


def parse(text, unit, shapes_allowed=()):
    ctx = Ctx(unit, shapes_allowed)
    text = text.replace('\r\n', '\n')
    if unit['decision'] == 'slides':
        model = parse_slides_A(text, ctx) if unit['candidate'] == 'A' else parse_slides_B(text, ctx)
    else:
        model = parse_document(text, ctx)
    return model, ctx


# ---------------------------------------------------------------- edits and scoring

def count_occ(hay, needle):
    if needle == '':
        return 2
    n, start = 0, 0
    while True:
        k = hay.find(needle, start)
        if k < 0:
            return n
        n += 1
        start = k + 1


def apply_edits(text, edits):
    """Returns (new_text, flag, message); flag is None when every pair applied."""
    cur = text
    for k, e in enumerate(edits, 1):
        if not isinstance(e, dict) or not isinstance(e.get('old'), str) or not isinstance(e.get('new'), str):
            return cur, 'bad_edit', 'edit %d: each edit is {"old": "...", "new": "..."} with two strings.' % k
        old = e['old']
        c = count_occ(cur, old)
        if c == 0:
            return cur, 'edit_no_match', ('edit %d: the "old" text does not occur in the file. It must be copied '
                                          'exactly from the file as it is after the earlier edits (same spaces, '
                                          'line breaks and characters).' % k)
        if c > 1:
            return cur, 'edit_ambiguous', ('edit %d: the "old" text occurs %s times in the file. Include enough '
                                           'surrounding text to make it occur exactly once.'
                                           % (k, 'many' if old == '' else c))
        cur = cur.replace(old, e['new'], 1)
    return cur, None, ''


def first_diff(a, b):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if a.get(k) != b.get(k):
                return '%s: %s' % (k, first_diff(a.get(k), b.get(k)))
        return ''
    if isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                return '[%d] %s' % (i, first_diff(x, y))
        if len(a) != len(b):
            return 'length %d vs %d' % (len(a), len(b))
        return ''
    return 'got %s, want %s' % (json.dumps(a, ensure_ascii=False)[:160], json.dumps(b, ensure_ascii=False)[:160])


def check_write(model, task, unit):
    exp = task['expect']
    if unit['decision'] == 'slides':
        got = model['slides']
        if got == exp['items']:
            return True, ''
        return False, 'slides differ from the brief: ' + first_diff(got, exp['items'])
    got = model['blocks']
    j = 0
    for want in exp['items']:
        while j < len(got) and got[j] != want:
            j += 1
        if j >= len(got):
            return False, 'required block not found (in order): ' + json.dumps(want, ensure_ascii=False)[:300]
        j += 1
    return True, ''


def is_refusal(a):
    t = a.get('text')
    return isinstance(t, str) and t.strip().upper().startswith(REFUSE_PREFIX)


def load_unit(unit_id):
    with open(os.path.join(UNITS, unit_id + '.json'), encoding='utf-8') as f:
        return json.load(f)


def load_gold(unit_id):
    with open(os.path.join(GOLD, unit_id + '.json'), encoding='utf-8') as f:
        return {a['task_id']: a for a in json.load(f)['answers']}


def score(unit_id, answers):
    unit = load_unit(unit_id)
    gold = load_gold(unit_id)
    shapes_seed = unit.get('shape_ids', [])
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
        extra = set()
        if is_refusal(a):
            res['chars'] = len(a['text'])
            if task['type'] == 'edit' and a.get('edits'):
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
        if task['type'] == 'edit':
            edits = a.get('edits')
            if not isinstance(edits, list):
                res['error'] = ('this is an edit task: answer with "edits", a list of {"old", "new"} pairs '
                                '(or refuse it with "text": "REFUSE: <reason>" and "edits": []).')
                res['flags'] = ['wrong_answer_kind']
                res['chars'] = len(a.get('text') or '')
                continue
            res['chars'] = sum(len(e.get('old') or '') + len(e.get('new') or '') for e in edits
                               if isinstance(e, dict))
            text, flag, msg = apply_edits(unit['seed'], edits)
            if flag:
                res['error'] = msg
                res['flags'] = [flag]
                continue
            model, ctx = parse(text, unit, shapes_seed)
            gmodel = None
            if not gold_refuses:
                gtext, gflag, _ = apply_edits(unit['seed'], g['edits'])
                gmodel, gctx = parse(gtext, unit, shapes_seed)
                if gflag or gctx.errors:
                    raise RuntimeError('gold answer for %s is broken' % tid)
            if text == unit['seed']:
                extra.add('no_change')
        else:
            text = a.get('text')
            if not isinstance(text, str):
                res['error'] = 'this is a write task: answer with "text", the complete new file.'
                res['flags'] = ['wrong_answer_kind']
                continue
            res['chars'] = len(text)
            if not text.lstrip().startswith('---'):
                extra.add('no_front_matter')
                text = task['front_matter'] + '\n' + text
            model, ctx = parse(text, unit, ())
        res['valid'] = not ctx.errors
        res['error'] = ctx.error_text()
        res['flags'] = sorted(ctx.flags | extra)
        if not res['valid']:
            continue
        if task['type'] == 'edit':
            if gold_refuses:
                res['landed'] = False
                res['detail'] = ('nothing in this file\'s syntax or names does what was asked; the intended answer '
                                 'is a refusal.')
                res['flags'] = sorted(set(res['flags']) | {'should_refuse'})
                continue
            res['landed'] = model == gmodel
            if not res['landed']:
                res['detail'] = 'result differs from the intended edit: ' + first_diff(model, gmodel)
                res['flags'] = sorted(set(res['flags']) | {'not_landed'})
        else:
            ok, detail = check_write(model, task, unit)
            res['landed'] = ok
            res['detail'] = detail
            if not ok:
                res['flags'] = sorted(set(res['flags']) | {'brief_unmet'})
    return {'results': results}


def main(argv):
    if len(argv) != 3:
        print(__doc__.strip().split('\n')[2], file=sys.stderr)
        return 2
    with open(argv[2], encoding='utf-8') as f:
        answers = json.load(f)
    print(json.dumps(score(argv[1], answers), ensure_ascii=False, indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
