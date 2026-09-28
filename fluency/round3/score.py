#!/usr/bin/env python3
"""Scorer for round 3 of the hanji fluency test kit (DESIGN.md §10 item 8).

usage: python3 score.py <unit_id> <answers.json>

answers.json: {"answers": [{"task_id": ..., "text": ...} | {"task_id": ..., "edits": [{"old", "new"}]}]}
              an edit task may be refused: {"task_id": ..., "text": "REFUSE: <reason>", "edits": []}
prints:       {"results": [{"task_id", "valid", "landed", "error", "chars", "flags", "detail"}]}

Every candidate syntax is parsed into one common semantic model:
  Document -> {"fm": {...}, "blocks": [heading | paragraph(style, text) | list item | table(style, grid) | pagebreak]}
              an empty paragraph is a paragraph with text "" (and its style, or null)
              a table grid is {"rows", "cols", "cells": [[r, c, paras, rowspan, colspan, is_header], ...]}
              paras is the cell's list of paragraphs [[style or null, text], ...]

Reuses round 2's scorer (../round2/score.py) by import for the shared pieces (front matter, inline tags, style
names, the {style="Name"} line, edits); round 1 and round 2 files are not changed.

Under both A candidates <p/> has the single meaning of DESIGN.md §5.2 (as of 6a3530b): it starts a paragraph, and
<p style="Name"/> starts one in style Name. A line holding only the tag is an empty paragraph; inside a pipe cell
each tag starts the next paragraph of the cell, which may be empty (a<p/><p/>b is a, an empty paragraph, b; a<p/>
ends with one). A tag written first in a cell starts the first paragraph itself, so a leading plain <p/> changes
nothing (canonical form drops it) and a leading <p style="Name"/> styles the first paragraph.
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
MAX_ERRORS = r2.MAX_ERRORS
REFUSE_PREFIX = r2.REFUSE_PREFIX

BR_RE = re.compile(r'\s*<\s*br\s*/?\s*>\s*', re.I)
P_TAG_RE = re.compile(r'<\s*(/?)\s*p\b([^<>]*?)(/?)\s*>', re.I)
EMPTY_P_LINE = re.compile(r'^<\s*p\b([^<>]*?)/\s*>$', re.I)
DIV_LINE = re.compile(r'^<\s*div\b([^>]*)>(.*)<\s*/\s*div\s*>$', re.I | re.S)
PAGEBREAK_LINE = re.compile(r'^<\s*pagebreak\s*/?\s*>$', re.I)


def ntext(s):
    """Text as compared: whitespace collapsed, **bold** markers ignored, line breaks written <br/>."""
    return r2.ntext(BR_RE.sub('<br/>', s))


class Ctx:
    def __init__(self, unit):
        self.unit = unit
        self.decision = d = unit['decision']
        self.cand = c = unit['candidate']
        self.names = unit.get('names', {})
        self.errors = []
        self.flags = set()
        self.p_tags = c == 'A'                            # §5.2 <p/>: cell paragraphs and empty-paragraph lines
        self.cell_p = d == 'cellpara' and c == 'A'        # the candidate tested <p/> inside pipe cells
        self.list_tables = d == 'cellpara' and c == 'B'   # {list-table} blocks
        self.empty_p = d == 'emptypara' and c == 'A'      # the candidate tested a line <p/> / <p style="Name"/>
        self.empty_div = d == 'emptypara' and c == 'B'    # a line <div></div> / <div style="Name"></div>

    def add(self, flag, line, col, msg):
        self.flags.add(flag)
        self.errors.append((flag, line, col, msg))

    def tag_hint(self):
        base = ('Formatting is by style name only: <div style="Name">text</div> for a paragraph and a line '
                '{style="Name"} before a table; colours, fonts, sizes, spacing, alignment and borders cannot be '
                'written. <br/> is a line break inside a paragraph and a line <pagebreak/> is a page break.')
        if self.p_tags:
            return base + (' Inside a table cell, <p/> or <p style="Name"/> starts the next paragraph of the cell; '
                           'a line holding only <p/> or <p style="Name"/> is an empty paragraph.')
        if self.list_tables:
            return base + (' A table whose cells hold several paragraphs or a styled paragraph is a list table: a '
                           'line {list-table}, then for each row a line - and one line "  - " per cell.')
        return base + ' An empty paragraph is a line holding only <div></div> or <div style="Name"></div>.'

    def error_text(self):
        out = ['line %d, column %d: %s' % (l, c, m) for _, l, c, m in self.errors[:MAX_ERRORS]]
        if len(self.errors) > MAX_ERRORS:
            out.append('(%d more errors)' % (len(self.errors) - MAX_ERRORS))
        return '\n'.join(out)


# ---------------------------------------------------------------- inline text

def check_text(t, ln, col, ctx, in_cell=False):
    """Checks inline content; handles the candidate tags that are not inline tags of round 2."""
    for m in P_TAG_RE.finditer(t):
        if ctx.cell_p:
            ctx.add('p_outside_cell', ln, col + m.start(),
                    '<p/> and <p style="Name"/> start a paragraph inside a table cell, or are an empty paragraph as '
                    'a line of their own. Outside a table, a paragraph with text is its own line, and a styled one '
                    'is <div style="Name">text</div>.')
        elif ctx.empty_p:
            ctx.add('empty_para_form', ln, col + m.start(),
                    'an empty paragraph is a line holding only <p/> or <p style="Name"/>; the tag is never inside '
                    'a line of text (inside a table cell it starts the next paragraph of the cell).')
    if ctx.p_tags:
        t = P_TAG_RE.sub(' ', t)
    if in_cell and re.search(r'<\s*/?\s*div\b', t):
        if ctx.list_tables:
            msg = ('a pipe table cell holds one paragraph with the default style. A table whose cells hold a styled '
                   'paragraph or several paragraphs is written as a list table.')
        elif ctx.p_tags:
            msg = ('inside a table cell a paragraph never uses <div>; a cell paragraph with a style is written '
                   '<p style="Name"/> before its text.')
        else:
            msg = 'a table cell holds text only; <div> is not allowed inside a table cell.'
        ctx.add('cell_div', ln, col, msg)
        t = re.sub(r'<\s*/?\s*div\b[^>]*>', ' ', t)
    r2.check_inline(t, ln, col, ctx)


def para_style(attrs, ln, col, ctx, elem):
    """Attributes of <p .../> or an empty <div ...>: only style="Name" (optional). Returns the style or None."""
    for k, v in attrs.items():
        if k == 'style':
            continue
        ctx.add('invented_attr', ln, col, '<%s> has no attribute "%s"; its only attribute is style="Name", with one '
                'paragraph style of this file.' % (elem, k))
        if k in ('color', 'height', 'size', 'spacing', 'align', 'class') or r2.CSS_RE.search(v or ''):
            ctx.flags.add('css_in_attr')
    if 'style' in attrs:
        r2.check_style_value(attrs['style'], 'paragraph', 'style', ln, col, ctx)
        return attrs['style']
    return None


# ---------------------------------------------------------------- tables

LEFT, UP = r2.LEFT, r2.UP


def build_grid(toks, pos, ctx):
    """toks: rows of LEFT / UP / paras; pos: rows of (line, col). Returns sorted cells or None."""
    R, W = len(toks), len(toks[0])
    origin = {}
    ok = True
    for r in range(R):
        for c in range(W):
            t = toks[r][c]
            ln, col = pos[r][c]
            if t is LEFT:
                if c == 0:
                    ctx.add('marker_misplaced', ln, col, '|| at the start of a row: there is no cell to the left to '
                            'extend.')
                    ok = False
                    origin[r, c] = (r, c)
                else:
                    origin[r, c] = origin[r, c - 1]
            elif t is UP:
                if r == 0:
                    ctx.add('marker_misplaced', ln, col, '^^ in the header row: there is no cell above to merge into.')
                    ok = False
                    origin[r, c] = (r, c)
                else:
                    origin[r, c] = origin[r - 1, c]
            else:
                origin[r, c] = (r, c)
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
            ctx.add('merge_not_rectangular', pos[o[0]][o[1]][0], 1,
                    'the merged area of the cell in row %d, column %d is not a rectangle. Every cell it covers is ^^ '
                    '(the cell above belongs to it) or || (the cell to the left belongs to it).' % (o[0] + 1, o[1] + 1))
            return None
        cells.append([o[0], o[1], toks[o[0]][o[1]], rs, cs, o[0] == 0])
    return sorted(cells, key=lambda x: (x[0], x[1]))


def cell_paras_A(raw, ln, col, ctx):
    """A pipe cell under the §5.2 rule (both A candidates): the cell's text starts its first paragraph and each
    <p/> / <p style="Name"/> starts another, which may be empty. A tag written first starts the first paragraph
    itself: <p style="Name"/>x styles it, and a leading plain <p/> changes nothing (<p/>x is x)."""
    ms = list(P_TAG_RE.finditer(raw))
    texts, styles, last = [], [None], 0
    for m in ms:
        if m.group(1) == '/' or m.group(3) != '/':
            ctx.add('cell_para_form', ln, col + m.start(), 'inside a table cell the next paragraph starts with <p/> '
                    'or <p style="Name"/>, one self-closing tag; there is no <p> ... </p>.')
            return None
        styles.append(para_style(r2.parse_attrs(m.group(2)), ln, col + m.start(), ctx, 'p'))
        texts.append(raw[last:m.start()])
        last = m.end()
    texts.append(raw[last:])
    if len(texts) == 1:
        check_text(raw, ln, col, ctx, in_cell=True)
        return [[None, ntext(raw)]]
    if not texts[0].strip():
        texts, styles = texts[1:], styles[1:]      # the cell begins with the tag of its first paragraph
    paras = []
    for t, s in zip(texts, styles):
        check_text(t, ln, col, ctx, in_cell=True)
        paras.append([s, ntext(t)])
    return paras


def pipe_cell(raw, ln, col, ctx):
    if ctx.p_tags:
        return cell_paras_A(raw, ln, col, ctx)
    check_text(raw, ln, col, ctx, in_cell=True)
    return [[None, ntext(raw)]]


def parse_pipe_table(tlines, start_line, ctx):
    rows = []
    for k, raw in enumerate(tlines):
        ln = start_line + k
        s = raw.strip()
        lead = len(raw) - len(raw.lstrip())
        if len(s) < 2 or not s.endswith('|') or s.endswith('\\|'):
            ctx.add('pipe_row', ln, len(raw.rstrip()) + 1, 'a table row starts and ends with |.')
            return None
        rows.append((ln, lead, r2.split_row(s)))
    if len(rows) < 2:
        ctx.add('pipe_row', start_line, 1, 'a pipe table needs a header row, then a delimiter row such as |---|---|.')
        return None
    hdr, delim = rows[0], rows[1]
    W = len(hdr[2])
    for text, col in delim[2]:
        if not re.fullmatch(r'\s*:?-+:?\s*', text):
            ctx.add('pipe_delimiter', delim[0], delim[1] + col, 'the second line of a table is the delimiter row, '
                    'with one --- per column, such as |---|---|---|.')
            return None
    ok = True
    if len(delim[2]) != W:
        ctx.add('row_width_mismatch', delim[0], 1, 'the delimiter row has %d cells but the header row has %d. '
                'Write one --- per column.' % (len(delim[2]), W))
        ok = False
    grid_rows = [hdr] + rows[2:]
    for ln, lead, cells in grid_rows:
        if len(cells) != W:
            ctx.add('row_width_mismatch', ln, 1, 'this row has %d cells but the header row has %d. Every row has one '
                    'cell per column: a cell merged into the one above is written ^^, and a column covered by the '
                    'cell to its left is written || (nothing between the pipes).' % (len(cells), W))
            ok = False
    if not ok:
        return None
    toks, pos = [], []
    before = len(ctx.errors)
    for ln, lead, cells in grid_rows:
        row, prow = [], []
        for text, col in cells:
            if text == '':
                row.append(LEFT)
            elif text.strip() == '^^':
                row.append(UP)
            else:
                row.append(pipe_cell(text, ln, lead + col, ctx))
            prow.append((ln, lead + col))
        toks.append(row)
        pos.append(prow)
    if len(ctx.errors) > before or any(t is None for row in toks for t in row):
        return None
    cells = build_grid(toks, pos, ctx)
    if cells is None:
        return None
    return {'k': 'table', 'style': None, 'grid': {'rows': len(toks), 'cols': W, 'cells': cells}}


def pipe_extent(lines, i):
    j = i
    while j < len(lines) and lines[j].strip().startswith('|'):
        j += 1
    return j


LIST_FORM = ('A list table is a line {list-table}, then for each row a line holding only -, followed by one line '
             '"  - " (two spaces, -, a space) per cell; each further paragraph of a cell is its own line indented four '
             'spaces. There is no blank line inside a list table.')


def list_cell_para(t, ln, col, ctx):
    s = t.strip()
    dm = DIV_LINE.match(s)
    if dm:
        style = r2.check_block_attrs('div', r2.parse_attrs(dm.group(1)), 'style', ln, col, ctx)
        body = dm.group(2)
        if not body.strip():
            ctx.add('empty_cell_para', ln, col, 'every paragraph in a cell has text.')
            return None
        if re.search(r'<\s*div\b', body):
            ctx.add('div_form', ln, col, 'a <div> cannot contain another <div>.')
            return None
        check_text(body, ln, col, ctx)
        return [style, ntext(body)]
    if re.search(r'<\s*/?\s*div\b', s):
        ctx.add('div_form', ln, col, 'a paragraph with a style is the whole line <div style="Name">text</div>, with '
                'nothing before <div> or after </div>.')
        return None
    check_text(s, ln, col, ctx)
    return [None, ntext(s)]


def parse_list_table(lines, i, style, ctx):
    """lines[i] is the {list-table} line. Returns (block or None, next index)."""
    n = len(lines)
    end = i + 1
    while end < n and lines[end].strip():
        end += 1
    rows = []
    cur = None
    for k in range(i + 1, end):
        raw = lines[k].rstrip()
        ln = k + 1
        if raw == '-':
            rows.append((ln, []))
            cur = None
            continue
        if not rows:
            ctx.add('list_table_form', ln, 1, 'the first line after {list-table} is a line holding only -, which '
                    'starts the header row. ' + LIST_FORM)
            return None, end
        m = re.match(r'^  -(?: (.*))?$', raw)
        if m:
            cur = {'ln': ln, 'lines': [(ln, 5, m.group(1) or '')]}
            rows[-1][1].append(cur)
            continue
        m = re.match(r'^    (\S.*)$', raw)
        if m and cur is not None:
            cur['lines'].append((ln, 5, m.group(1)))
            continue
        if re.match(r'^-\s*\S', raw):
            msg = 'a row line holds only -; its cells are the "  - " lines below it. '
        elif re.match(r'^\s*[-*+]', raw.lstrip(' ')) and raw.lstrip(' ')[:1] in '-*+':
            msg = 'a cell line is indented exactly two spaces and starts with "- ": "  - text". '
        elif cur is None:
            msg = 'a paragraph line belongs to a cell and comes after that cell\'s "  - " line. '
        else:
            msg = 'each further paragraph of a cell is its own line, indented exactly four spaces. '
        ctx.add('list_table_form', ln, 1, msg + LIST_FORM)
        return None, end
    if not rows:
        ctx.add('list_table_form', i + 1, 1, 'the {list-table} line is followed by the rows of the table. ' + LIST_FORM)
        return None, end
    W = len(rows[0][1])
    ok = W > 0
    for ln, cells in rows:
        if len(cells) != W:
            ctx.add('row_width_mismatch', ln, 1, 'this row has %d cells but the header row has %d. Every row has one '
                    'cell per column: a cell merged into the one above is "  - ^^", and a column covered by the cell '
                    'to its left is "  - ||".' % (len(cells), W))
            ok = False
    if not ok:
        return None, end
    toks, pos = [], []
    before = len(ctx.errors)
    for ln, cells in rows:
        trow, prow = [], []
        for cell in cells:
            first = cell['lines'][0][2].strip()
            if first in ('^^', '||'):
                if len(cell['lines']) > 1:
                    ctx.add('marker_misplaced', cell['lines'][1][0], 1, 'a covered cell holds only its marker (^^ or '
                            '||) and no other paragraph; the text of a merged cell is written in its top-left cell.')
                trow.append(UP if first == '^^' else LEFT)
            elif len(cell['lines']) == 1 and not first:
                trow.append([[None, '']])
            else:
                paras = [list_cell_para(t, l, c, ctx) for l, c, t in cell['lines']]
                if any(p is None for p in paras):
                    trow.append(None)
                else:
                    trow.append(paras)
            prow.append((cell['ln'], 5))
        toks.append(trow)
        pos.append(prow)
    if len(ctx.errors) > before or any(t is None for row in toks for t in row):
        return None, end
    cells = build_grid(toks, pos, ctx)
    if cells is None:
        return None, end
    return {'k': 'table', 'style': style, 'grid': {'rows': len(toks), 'cols': W, 'cells': cells}}, end


# ---------------------------------------------------------------- document

def parse_document(text, ctx):
    lines = text.split('\n')
    fm, i = r2.parse_front_matter(lines, ctx, 'document')
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
        if s.startswith('{') and s.endswith('}'):
            inner = s[1:-1].strip()
            if ctx.list_tables and inner == 'list-table':
                b, i = parse_list_table(lines, i, None, ctx)
                if b:
                    blocks.append(b)
                continue
            if ctx.list_tables and inner.startswith('list-table'):
                ctx.add('attr_form', ln, col0, 'the line {list-table} holds nothing else. A table style is its own '
                        'line {style="Name"}, directly before the {list-table} line.')
                i += 1
                continue
            style = r2.parse_attr_line(s, ln, col0, ctx)
            nxt = lines[i + 1].strip() if i + 1 < n else ''
            if ctx.list_tables and nxt == '{list-table}':
                b, i = parse_list_table(lines, i + 1, style, ctx)
                if b:
                    blocks.append(b)
                continue
            if nxt.startswith('|'):
                j = pipe_extent(lines, i + 1)
                b = parse_pipe_table(lines[i + 1:j], ln + 1, ctx)
                if b:
                    b['style'] = style
                    blocks.append(b)
                i = j
                continue
            ctx.add('attr_orphan', ln, col0, 'a {style="Name"} line must be directly followed by the first line of '
                    'its table, with no blank line or other text between.')
            i += 1
            continue
        if re.fullmatch(r'-', s) or re.fullmatch(r'[-*+]\s+[-*+](\s.*)?', s):
            j = i + 1
            while j < n and lines[j].strip():
                j += 1
            if ctx.list_tables:
                msg = ('these lines are read as an ordinary list, and a list item has text. A list table begins with a '
                       'line {list-table} and has no blank line inside; ' + LIST_FORM)
            else:
                msg = 'a list item has text after "- ".'
            ctx.add('list_parsed_as_list', ln, col0, msg)
            i = j
            continue
        if s.startswith('<'):
            if PAGEBREAK_LINE.match(s):
                blocks.append({'k': 'pagebreak'})
                i += 1
                continue
            em = EMPTY_P_LINE.match(s)
            if em and ctx.p_tags:
                style = para_style(r2.parse_attrs(em.group(1)), ln, col0, ctx, 'p')
                blocks.append({'k': 'p', 'style': style, 'text': ''})
                i += 1
                continue
            if em is None and ctx.empty_p and re.match(r'^<\s*/?\s*p\b', s, re.I):
                ctx.add('empty_para_form', ln, col0, 'an empty paragraph is a line holding only <p/> or '
                        '<p style="Name"/>: one self-closing tag, no </p>, no text.')
                i += 1
                continue
            if ctx.empty_div and re.match(r'^<\s*div\b[^<>]*/\s*>$', s, re.I):
                ctx.add('empty_para_form', ln, col0, 'an empty paragraph is a line holding only <div></div> or '
                        '<div style="Name"></div>, an opening and a closing tag with nothing between them.')
                i += 1
                continue
            if re.match(r'^<\s*/?\s*div\b', s, re.I):
                dm = DIV_LINE.match(s)
                if not dm:
                    ctx.add('div_form', ln, col0, 'expected <div style="Name">text</div> on one line, with nothing '
                            'after </div>.')
                    i += 1
                    continue
                attrs = r2.parse_attrs(dm.group(1))
                body = dm.group(2)
                if not body.strip():
                    if ctx.empty_div:
                        style = para_style(attrs, ln, col0, ctx, 'div')
                        blocks.append({'k': 'p', 'style': style, 'text': ''})
                    elif ctx.empty_p:
                        ctx.add('empty_para_form', ln, col0, 'an empty paragraph is a line holding only <p/> or '
                                '<p style="Name"/>; a <div> holds the text of its paragraph.')
                    else:
                        ctx.add('div_form', ln, col0, 'a <div> holds the text of its paragraph.')
                    i += 1
                    continue
                style = r2.check_block_attrs('div', attrs, 'style', ln, col0, ctx)
                if re.search(r'<\s*div\b', body):
                    ctx.add('unclosed_tag', ln, col0, 'a <div> cannot contain another <div>.')
                check_text(body, ln, col0 + dm.start(2), ctx)
                blocks.append({'k': 'p', 'style': style, 'text': ntext(body)})
                i += 1
                continue
            km = re.match(r'^<\s*keep\b([^>]*?)/\s*>$', s)
            if km:
                blocks.append({'k': 'keep', 'attrs': r2.parse_attrs(km.group(1))})
                i += 1
                continue
            tm = re.match(r'^<\s*/?\s*([A-Za-z][\w-]*)([^>]*)', s)
            if tm and tm.group(1).lower() not in r2.INLINE_TAGS and tm.group(1).lower() != 'p':
                ctx.add('unknown_tag', ln, col0, '<%s> is not a tag of this format. %s' % (tm.group(1).lower(),
                                                                                         ctx.tag_hint()))
                attrs = r2.parse_attrs(tm.group(2))
                if 'style' in attrs or 'color' in attrs or any(r2.CSS_RE.search(v or '') for v in attrs.values()):
                    ctx.flags.add('css_in_attr')
                i += 1
                continue
        hm = re.match(r'^(#{1,6})\s+(.*)$', s)
        if hm:
            check_text(hm.group(2), ln, col0, ctx)
            blocks.append({'k': 'h', 'level': len(hm.group(1)), 'text': ntext(hm.group(2))})
            i += 1
            continue
        lm = re.match(r'^(\s*)([-*+]|\d+[.)])\s+(.*)$', raw)
        if lm:
            check_text(lm.group(3), ln, col0, ctx)
            blocks.append({'k': 'li', 'ordered': lm.group(2)[0].isdigit(), 'level': len(lm.group(1)) // 2,
                           'text': ntext(lm.group(3))})
            i += 1
            continue
        check_text(s, ln, col0, ctx)
        blocks.append({'k': 'p', 'style': None, 'text': ntext(s)})
        i += 1
    return {'fm': fm, 'blocks': blocks}


def parse(text, unit, _unused=()):
    ctx = Ctx(unit)
    model = parse_document(text.replace('\r\n', '\n'), ctx)
    return model, ctx


# ---------------------------------------------------------------- diagnosis of a wrong result

def is_empty_para(b):
    return b.get('k') == 'p' and b.get('text') == ''


def without_empty_paras(blocks):
    """The blocks with every empty paragraph left out, those inside table cells included."""
    out = []
    for b in blocks:
        if is_empty_para(b):
            continue
        if b.get('k') == 'table':
            g = b['grid']
            cells = [c[:2] + [[p for p in c[2] if p[1] != '']] + c[3:] for c in g['cells']]
            b = dict(b, grid=dict(g, cells=cells))
        out.append(b)
    return out


def diagnose(got_blocks, want_blocks):
    """Flags that name what went wrong in a valid result that differs from the intended one."""
    flags = set()
    if without_empty_paras(got_blocks) == without_empty_paras(want_blocks):
        flags.add('wrong_empty_para')
    if any(b.get('k') == 'p' and b.get('text', '').replace('<br/>', '').strip() == '' and b.get('text')
           for b in got_blocks):
        flags.add('br_for_p')
    gt = [b for b in got_blocks if b.get('k') == 'table']
    wt = [b for b in want_blocks if b.get('k') == 'table']
    for tg, tw in zip(gt, wt):
        gc = {(c[0], c[1]): c[2] for c in tg['grid']['cells']}
        for c in tw['grid']['cells']:
            wp, gp = c[2], gc.get((c[0], c[1]))
            if gp is None or len(wp) < 2 or len(gp) >= len(wp):
                continue
            flags.add('cell_para_lost')
            if any('<br/>' in t for _, t in gp):
                flags.add('br_for_p')
    return flags


# ---------------------------------------------------------------- scoring

def check_write(model, task):
    want = task['expect']['items']
    got = model['blocks']
    if got == want:
        return True, ''
    return False, 'blocks differ from the brief: ' + first_diff(got, want)


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
        gmodel = None
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
            model, ctx = parse(text, unit)
            if not gold_refuses:
                gtext, gflag, _ = apply_edits(unit['seed'], g['edits'])
                gmodel, gctx = parse(gtext, unit)
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
            model, ctx = parse(text, unit)
        res['valid'] = not ctx.errors
        res['error'] = ctx.error_text()
        res['flags'] = sorted(ctx.flags | extra)
        if not res['valid']:
            continue
        if task['type'] == 'edit':
            if gold_refuses:
                res['detail'] = ('nothing in this file\'s syntax or names does what was asked; the intended answer '
                                 'is a refusal.')
                res['flags'] = sorted(set(res['flags']) | {'should_refuse'})
                continue
            res['landed'] = model == gmodel
            if not res['landed']:
                res['detail'] = 'result differs from the intended edit: ' + first_diff(model, gmodel)
                res['flags'] = sorted(set(res['flags']) | {'not_landed'} | diagnose(model['blocks'],
                                                                                    gmodel['blocks']))
        else:
            ok, detail = check_write(model, task)
            res['landed'] = ok
            res['detail'] = detail
            if not ok:
                res['flags'] = sorted(set(res['flags']) | {'brief_unmet'} | diagnose(model['blocks'],
                                                                                     task['expect']['items']))
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
