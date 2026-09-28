"""In-memory workbook model for round 4 of the hanji fluency test (DESIGN.md §5.4, §8, §10 item 6).

A workbook is  {"sheets": [{"name", "tables": [table]}]}
a table is     {"name", "anchor": "A1", "columns": [{"name", "type", "format", "formula"}], "rows": [row]}
a row is       {"orig": <sheet row number in the seed, or None for a row added since>, "cells": [value, ...]}
a value is     None (empty) | str (text, or a date "YYYY-MM-DD") | int | float
               the cells of a formula column are always None; their values are computed.

The table's header is on the anchor's row, so data row i (0-based) is sheet row anchor_row + 1 + i.

This module holds everything the three write shapes and the two read views share: types and formats, the
structured-reference formula language and its evaluator, the range operations (write shape A), the code API
and its sandbox (write shape C), the text window and its parser (write shape B, read view A) and the compressed
index view (read view B). Standard library only.
"""
import ast
import copy
import json
import re
import sys
import time
from decimal import Decimal, ROUND_HALF_UP

TYPES = ('text', 'number', 'date')
FORMATS = {
    'text': ('@',),
    'number': ('#,##0', '#,##0.0', '0', '0.0', '0.0%', '0000', '00000', '000000'),
    'date': ('yyyy-mm', 'yyyy-mm-dd'),
}
FUNCTIONS = ('SUM', 'SUMIFS', 'COUNTIFS', 'AVERAGE', 'MIN', 'MAX', 'ROUND', 'ABS', 'IF', 'IFERROR')
FETCH_FUNCTIONS = ('WEBSERVICE', 'FILTERXML', 'HYPERLINK', 'IMPORTDATA', 'IMPORTXML', 'IMPORTHTML', 'RTD', 'CALL',
                   'REGISTER.ID', 'STOCKHISTORY')
MAX_NEW_ROWS = 50
A1_RE = re.compile(r'^\$?([A-Z]{1,3})\$?([1-9]\d*)$')
DATE_RE = re.compile(r'^(\d{4})-(\d{2})(?:-(\d{2}))?$')


class WbError(Exception):
    def __init__(self, flag, msg):
        super().__init__(msg)
        self.flag = flag
        self.msg = msg


# ---------------------------------------------------------------- addresses

def col_letter(i):
    s = ''
    i += 1
    while i:
        i, r = divmod(i - 1, 26)
        s = chr(65 + r) + s
    return s


def col_index(letters):
    n = 0
    for ch in letters:
        n = n * 26 + ord(ch) - 64
    return n - 1


def parse_addr(a):
    m = A1_RE.match(a.strip().upper()) if isinstance(a, str) else None
    if not m:
        raise WbError('bad_range', '%r is not a cell address such as D70.' % (a,))
    return col_index(m.group(1)), int(m.group(2))


def anchor_rc(t):
    return parse_addr(t['anchor'])


def table_range(t):
    c0, r0 = anchor_rc(t)
    return '%s%d:%s%d' % (col_letter(c0), r0, col_letter(c0 + len(t['columns']) - 1), r0 + len(t['rows']))


# ---------------------------------------------------------------- values and formats

def dec_places(fmt):
    return {'#,##0.0': 1, '0.0': 1, '0.0%': 3}.get(fmt, 0)


def round_half_up(v, d):
    q = Decimal(repr(v)).quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    return q


def nv(v):
    """Value as compared: numbers normalised (int when integral, else 9 decimals)."""
    if isinstance(v, bool):
        return v
    if isinstance(v, float):
        if v != v or v in (float('inf'), float('-inf')):
            return str(v)
        r = round(v, 9)
        return int(r) if r == int(r) else r
    return v


def norm_date(s):
    m = DATE_RE.match(s.strip()) if isinstance(s, str) else None
    if not m:
        return None
    y, mo, d = int(m.group(1)), int(m.group(2)), int(m.group(3) or 1)
    if not (1 <= mo <= 12 and 1 <= d <= 31):
        return None
    return '%04d-%02d-%02d' % (y, mo, d)


def looks_a1(s):
    return bool(re.search(r'(?<![A-Za-z\[])\$?[A-Za-z]{1,3}\$?\d+(?![\w\]])', re.sub(r'"[^"]*"|\[[^\]]*\]', '', s)))


def check_value(col, v, where=''):
    """A value written into a (non-formula) cell of col; returns it normalised or raises WbError."""
    if v is None or v == '':
        return None
    at = (' (%s)' % where) if where else ''
    if isinstance(v, str) and v.lstrip().startswith('='):
        if looks_a1(v):
            raise WbError('a1_formula', 'a value%s starts with "=" and uses A1 cell references: %r. A value is never '
                                        'a formula; make the column a formula column with structured references '
                                        '([@Column], Table[Column]) instead.' % (at, v))
        raise WbError('formula_in_cell', 'a value%s starts with "=": %r. Text that starts with "=" is not accepted; a '
                                         'formula belongs to a formula column.' % (at, v))
    t = col['type']
    if t == 'number':
        if isinstance(v, bool) or not isinstance(v, (int, float)):
            raise WbError('type_mismatch', 'column %s is a number column; %r%s is not a number.' % (col['name'], v, at))
        return nv(float(v)) if isinstance(v, float) else v
    if t == 'date':
        d = norm_date(v) if isinstance(v, str) else None
        if d is None:
            raise WbError('type_mismatch', 'column %s is a date column; write a date as "YYYY-MM-DD" (or "YYYY-MM" for '
                                           'the first of the month), not %r%s.' % (col['name'], v, at))
        return d
    if not isinstance(v, str):
        raise WbError('type_mismatch', 'column %s is a text column; write %r%s as a string.' % (col['name'], v, at))
    return v


def display(v, col):
    """A value as the column's format shows it."""
    if v is None:
        return ''
    if isinstance(v, Err):
        return str(v)
    t, f = col['type'], col['format']
    if t == 'text':
        return v if isinstance(v, str) else fmt_number(v, '0')
    if t == 'date':
        return v[:7] if f == 'yyyy-mm' else v
    return fmt_number(v, f)


def fmt_number(v, f):
    if not isinstance(v, (int, float)) or isinstance(v, bool):
        return str(v)
    if f == '0.0%':
        q = round_half_up(v * 100, 1)
        return '%s%%' % q
    if re.fullmatch(r'0{4,}', f):
        q = int(round_half_up(v, 0))
        return ('-' if q < 0 else '') + str(abs(q)).rjust(len(f), '0')
    d = dec_places(f)
    q = round_half_up(v, d)
    if f.startswith('#,##0'):
        return '{:,.{}f}'.format(q, d)
    return '{:.{}f}'.format(q, d)


def raw(v, col):
    """A value as the index view writes it: text as is, dates yyyy-mm-dd, numbers ungrouped at display precision."""
    if isinstance(v, Err):
        return str(v)
    t = col['type']
    if t in ('text', 'date'):
        return v
    d = dec_places(col['format'])
    return '{:.{}f}'.format(round_half_up(v, d), d)


NUM_DISPLAY = re.compile(r'^[-+]?(\d{1,3}(,\d{3})+|\d+)(\.\d+)?%?$')


def parse_display(s, col, where=''):
    """A cell of the text window back to a value of col's type (the inverse of display)."""
    s = s.strip()
    if s == '':
        return None
    if s.startswith('='):
        return check_value(col, s, where)
    t = col['type']
    at = (' (%s)' % where) if where else ''
    if t == 'text':
        return s
    if t == 'date':
        d = norm_date(s)
        if d is None:
            raise WbError('type_mismatch', 'column %s is a date column; %r%s is not a date (YYYY-MM-DD or YYYY-MM).'
                          % (col['name'], s, at))
        return d
    if not NUM_DISPLAY.match(s):
        raise WbError('type_mismatch', 'column %s is a number column; %r%s is not a number.' % (col['name'], s, at))
    pct = s.endswith('%')
    x = Decimal(s.rstrip('%').replace(',', ''))
    if pct:
        x = x / 100
    return nv(float(x)) if x != int(x) else int(x)


# ---------------------------------------------------------------- formulas (structured references only)

class Err(str):
    """An Excel error value such as #DIV/0!."""


NAME_CH = r'A-Za-z_ㄱ-ㆎ가-힣'
TOK = re.compile(r'''
 (?P<ws>\s+)
|(?P<str>"(?:[^"]|"")*")
|(?P<this>\[@(?:\[[^\[\]]+\](?::\[[^\[\]]+\])?|[^\[\]]+)\])
|(?P<tcol>[%(n)s][%(n)s0-9.]*\[(?:\[[^\[\]]+\]|@\[[^\[\]]+\](?::\[[^\[\]]+\])?|@[^\[\]]+|[^\[\]@]+)\])
|(?P<func>[A-Za-z_][A-Za-z0-9_.]*(?=\s*\())
|(?P<sheetref>(?:'[^']+'|[^\s!'"(),:=+\-*/&<>]+)!\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?)
|(?P<a1>\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?(?![\w\[]))
|(?P<num>\d+(?:\.\d+)?)
|(?P<bool>(?:TRUE|FALSE)(?![\w\[]))
|(?P<op><>|<=|>=|[-+*/^&=<>(),%%:])
|(?P<name>[^\s()\[\],+\-*/^&=<>:"]+)
''' % {'n': NAME_CH}, re.X)


def tokenize(f):
    if not isinstance(f, str) or not f.strip().startswith('='):
        raise WbError('bad_formula', 'a formula is text that starts with "=", not %r.' % (f,))
    s = f.strip()[1:]
    out, i = [], 0
    while i < len(s):
        m = TOK.match(s, i)
        if not m:
            raise WbError('bad_formula', 'formula %s: cannot read %r.' % (f, s[i:i + 12]))
        i = m.end()
        k = m.lastgroup
        if k == 'ws':
            continue
        out.append((k, m.group()))
    return out


def unbracket(s):
    s = s.strip()
    if s.startswith('[') and s.endswith(']'):
        s = s[1:-1]
    return s.strip()


class FParser:
    def __init__(self, f, table, book):
        self.f, self.toks, self.i = f, tokenize(f), 0
        self.table, self.book = table, book

    def peek(self):
        return self.toks[self.i] if self.i < len(self.toks) else (None, None)

    def take(self, val=None):
        k, v = self.peek()
        if k is None or (val is not None and v != val):
            raise WbError('bad_formula', 'formula %s: expected %s.' % (self.f, repr(val) if val else 'more'))
        self.i += 1
        return k, v

    def parse(self):
        if not self.toks:
            raise WbError('bad_formula', 'formula %s is empty.' % self.f)
        node = self.cmp()
        if self.i != len(self.toks):
            raise WbError('bad_formula', 'formula %s: unexpected %r.' % (self.f, self.peek()[1]))
        return node

    def cmp(self):
        a = self.cat()
        while self.peek()[1] in ('=', '<>', '<', '>', '<=', '>='):
            op = self.take()[1]
            a = ('bin', op, a, self.cat())
        return a

    def cat(self):
        a = self.add()
        while self.peek()[1] == '&':
            self.take()
            a = ('bin', '&', a, self.add())
        return a

    def add(self):
        a = self.mul()
        while self.peek()[1] in ('+', '-'):
            op = self.take()[1]
            a = ('bin', op, a, self.mul())
        return a

    def mul(self):
        a = self.pow()
        while self.peek()[1] in ('*', '/'):
            op = self.take()[1]
            a = ('bin', op, a, self.pow())
        return a

    def pow(self):
        a = self.unary()
        while self.peek()[1] == '^':
            self.take()
            a = ('bin', '^', a, self.unary())
        return a

    def unary(self):
        if self.peek()[1] in ('-', '+'):
            op = self.take()[1]
            a = self.unary()
            return ('neg', a) if op == '-' else a
        a = self.primary()
        while self.peek()[1] == '%':
            self.take()
            a = ('bin', '/', a, ('num', 100))
        return a

    def col_of(self, tname, cname):
        t = self.book.find_table(tname)
        if t is None:
            raise WbError('unknown_ref', 'formula %s: there is no table %s.' % (self.f, tname))
        names = [c['name'] for c in t['columns']]
        if cname not in names:
            raise WbError('unknown_ref', 'formula %s: table %s has no column %s (columns: %s).'
                          % (self.f, tname, cname, ', '.join(names)))
        return names.index(cname)

    def this_node(self, inner):
        m = re.fullmatch(r'\[([^\[\]]+)\]:\[([^\[\]]+)\]', inner)
        if m:
            a = self.col_of(self.table['name'], m.group(1).strip())
            b = self.col_of(self.table['name'], m.group(2).strip())
            if b < a:
                a, b = b, a
            return ('thisrange', a, b)
        return ('this', self.col_of(self.table['name'], unbracket(inner)))

    def primary(self):
        k, v = self.take()
        if k == 'num':
            return ('num', float(v) if '.' in v else int(v))
        if k == 'str':
            return ('str', v[1:-1].replace('""', '"'))
        if k == 'bool':
            return ('bool', v == 'TRUE')
        if k in ('a1', 'sheetref'):
            raise WbError('a1_formula', 'formula %s uses the A1 reference %s. Use structured references only: '
                                        '[@Column] for this row, Table[Column] for a whole column.' % (self.f, v))
        if k == 'this':
            return self.this_node(v[2:-1])
        if k == 'tcol':
            tname, rest = v.split('[', 1)
            rest = '[' + rest
            inner = rest[1:-1]
            if inner.startswith('@'):
                if tname != self.table['name']:
                    raise WbError('bad_formula', 'formula %s: %s refers to this row of another table.' % (self.f, v))
                return self.this_node(inner[1:])
            if inner.startswith('#'):
                raise WbError('bad_formula', 'formula %s: special items such as %s are not supported.' % (self.f, v))
            return ('col', tname, self.col_of(tname, unbracket(inner)))
        if k == 'func':
            name = v.upper()
            if name in FETCH_FUNCTIONS:
                raise WbError('fetch_function', 'formula %s uses %s, a function that fetches data; functions that '
                                                'fetch are not allowed.' % (self.f, name))
            if name not in FUNCTIONS:
                raise WbError('unknown_function', 'formula %s uses %s; the functions are %s.'
                              % (self.f, name, ', '.join(FUNCTIONS)))
            self.take('(')
            args = []
            if self.peek()[1] != ')':
                args.append(self.cmp())
                while self.peek()[1] == ',':
                    self.take()
                    args.append(self.cmp())
            self.take(')')
            ar = {'SUMIFS': lambda n: n >= 3 and n % 2 == 1, 'COUNTIFS': lambda n: n >= 2 and n % 2 == 0,
                  'ROUND': lambda n: n == 2, 'ABS': lambda n: n == 1, 'IF': lambda n: n in (2, 3),
                  'IFERROR': lambda n: n == 2}.get(name, lambda n: n >= 1)
            if not ar(len(args)):
                raise WbError('bad_formula', 'formula %s: %s has the wrong number of arguments.' % (self.f, name))
            return ('func', name, args)
        if k == 'op' and v == '(':
            a = self.cmp()
            self.take(')')
            return a
        if k == 'name':
            if A1_RE.match(v.upper()):
                raise WbError('a1_formula', 'formula %s uses the A1 reference %s.' % (self.f, v))
            raise WbError('unknown_ref', 'formula %s: %s is not a function, number or structured reference.'
                          % (self.f, v))
        raise WbError('bad_formula', 'formula %s: unexpected %r.' % (self.f, v))


def refs_of(node, out):
    if node[0] == 'this':
        out.add(('this', node[1]))
    elif node[0] == 'thisrange':
        for c in range(node[1], node[2] + 1):
            out.add(('this', c))
    elif node[0] == 'col':
        out.add(('col', node[1], node[2]))
    elif node[0] == 'func':
        for a in node[2]:
            refs_of(a, out)
    elif node[0] == 'bin':
        refs_of(node[2], out)
        refs_of(node[3], out)
    elif node[0] == 'neg':
        refs_of(node[1], out)
    return out


def to_num(v):
    if isinstance(v, Err):
        return v
    if v is None:
        return 0
    if isinstance(v, bool):
        return 1 if v else 0
    if isinstance(v, (int, float)):
        return v
    try:
        return float(v)
    except ValueError:
        return Err('#VALUE!')


def crit_match(v, c):
    if isinstance(c, str):
        m = re.match(r'^(<>|>=|<=|=|>|<)(.*)$', c)
        op, rhs = (m.group(1), m.group(2)) if m else ('=', c)
        try:
            rn = float(rhs)
        except ValueError:
            rn = None
        if rn is not None and isinstance(v, (int, float)) and not isinstance(v, bool):
            lhs, r = v, rn
        else:
            lhs, r = ('' if v is None else str(v)), rhs
            if op in ('>', '<', '>=', '<='):
                return False
            lhs, r = lhs.lower(), r.lower()
    else:
        op, r, lhs = '=', c, v
        if isinstance(v, str) or isinstance(c, str) or v is None:
            return v == c
    return {'=': lhs == r, '<>': lhs != r, '>': lhs > r, '<': lhs < r, '>=': lhs >= r, '<=': lhs <= r}[op]


class Evaluator:
    def __init__(self, book):
        self.book = book
        self.cache = {}
        self.visiting = set()
        self.asts = {}

    def ast(self, t, ci):
        f = t['columns'][ci]['formula']
        key = (t['name'], f)
        if key not in self.asts:
            self.asts[key] = FParser(f, t, self.book).parse()
        return self.asts[key]

    def column(self, t, ci):
        key = (t['name'], ci)
        if key in self.cache:
            return self.cache[key]
        col = t['columns'][ci]
        if not col.get('formula'):
            vals = [r['cells'][ci] for r in t['rows']]
            self.cache[key] = vals
            return vals
        if key in self.visiting:
            raise WbError('circular_ref', 'the formula of %s[%s] refers to itself.' % (t['name'], col['name']))
        self.visiting.add(key)
        node = self.ast(t, ci)
        nonformula = [k for k, c in enumerate(t['columns']) if not c.get('formula')]
        vals = []
        for ri, r in enumerate(t['rows']):
            if all(r['cells'][k] is None for k in nonformula):
                vals.append(None)       # a blank row stays blank
                continue
            v = self.ev(node, t, ri)
            if isinstance(v, float):
                v = nv(v)
            vals.append(v)
        self.visiting.discard(key)
        self.cache[key] = vals
        return vals

    def ev(self, n, t, ri):
        k = n[0]
        if k in ('num', 'str', 'bool'):
            return n[1]
        if k == 'this':
            return self.column(t, n[1])[ri]
        if k == 'thisrange':
            return [self.column(t, c)[ri] for c in range(n[1], n[2] + 1)]
        if k == 'col':
            return list(self.column(self.book.find_table(n[1]), n[2]))
        if k == 'neg':
            a = to_num(self.scalar(self.ev(n[1], t, ri)))
            return a if isinstance(a, Err) else -a
        if k == 'bin':
            op = n[1]
            a, b = self.scalar(self.ev(n[2], t, ri)), self.scalar(self.ev(n[3], t, ri))
            if isinstance(a, Err):
                return a
            if isinstance(b, Err):
                return b
            if op == '&':
                return ('' if a is None else display_plain(a)) + ('' if b is None else display_plain(b))
            if op in ('=', '<>', '<', '>', '<=', '>='):
                return crit_match(a, op + ('' if b is None else str(b))) if not isinstance(b, (int, float)) \
                    else crit_match(to_num(a), op + repr(b))
            a, b = to_num(a), to_num(b)
            if isinstance(a, Err):
                return a
            if isinstance(b, Err):
                return b
            if op == '+':
                return a + b
            if op == '-':
                return a - b
            if op == '*':
                return a * b
            if op == '/':
                return Err('#DIV/0!') if b == 0 else a / b
            if op == '^':
                return a ** b
        if k == 'func':
            return self.func(n[1], n[2], t, ri)
        raise WbError('bad_formula', 'cannot evaluate %r' % (n,))

    def scalar(self, v):
        return Err('#VALUE!') if isinstance(v, list) else v

    def func(self, name, args, t, ri):
        if name == 'IFERROR':
            a = self.scalar(self.ev(args[0], t, ri))
            return self.scalar(self.ev(args[1], t, ri)) if isinstance(a, Err) else a
        if name == 'IF':
            c = self.scalar(self.ev(args[0], t, ri))
            if isinstance(c, Err):
                return c
            c = to_num(c)
            if isinstance(c, Err):
                return c
            if c:
                return self.scalar(self.ev(args[1], t, ri))
            return self.scalar(self.ev(args[2], t, ri)) if len(args) > 2 else False
        vals = [self.ev(a, t, ri) for a in args]
        if name in ('SUM', 'AVERAGE', 'MIN', 'MAX'):
            nums = []
            for v in vals:
                if isinstance(v, list):
                    for x in v:
                        if isinstance(x, Err):
                            return x
                        if isinstance(x, (int, float)) and not isinstance(x, bool):
                            nums.append(x)
                else:
                    x = to_num(v)
                    if isinstance(x, Err):
                        return x
                    nums.append(x)
            if name == 'SUM':
                return sum(nums)
            if not nums:
                return Err('#DIV/0!') if name == 'AVERAGE' else 0
            return {'AVERAGE': lambda: sum(nums) / len(nums), 'MIN': lambda: min(nums), 'MAX': lambda: max(nums)}[name]()
        if name in ('SUMIFS', 'COUNTIFS'):
            if name == 'SUMIFS':
                target, pairs = vals[0], vals[1:]
            else:
                target, pairs = None, vals
            ranges = pairs[0::2]
            crits = [self.scalar(c) for c in pairs[1::2]]
            if not all(isinstance(r, list) for r in ranges) or (target is not None and not isinstance(target, list)):
                return Err('#VALUE!')
            n = len(ranges[0])
            if any(len(r) != n for r in ranges) or (target is not None and len(target) != n):
                return Err('#VALUE!')
            total = 0
            for i in range(n):
                if all(crit_match(r[i], c) for r, c in zip(ranges, crits)):
                    if target is None:
                        total += 1
                    elif isinstance(target[i], (int, float)) and not isinstance(target[i], bool):
                        total += target[i]
            return total
        if name == 'ROUND':
            a, d = to_num(self.scalar(vals[0])), to_num(self.scalar(vals[1]))
            if isinstance(a, Err):
                return a
            q = round_half_up(a, int(d)) if a >= 0 else -round_half_up(-a, int(d))
            return nv(float(q))
        if name == 'ABS':
            a = to_num(self.scalar(vals[0]))
            return a if isinstance(a, Err) else abs(a)
        raise WbError('bad_formula', 'unknown function %s' % name)


def display_plain(v):
    if isinstance(v, float) and v == int(v):
        return str(int(v))
    return str(v)


# ---------------------------------------------------------------- the workbook

class Book:
    def __init__(self, data):
        self.d = copy.deepcopy(data)
        self._ev = None

    # ---- lookup
    def sheets(self):
        return self.d['sheets']

    def find_sheet(self, name):
        for s in self.d['sheets']:
            if s['name'] == name:
                return s
        return None

    def sheet(self, name):
        s = self.find_sheet(name)
        if s is None:
            raise WbError('unknown_ref', 'there is no sheet %r (sheets: %s).'
                          % (name, ', '.join(x['name'] for x in self.d['sheets'])))
        return s

    def find_table(self, name):
        for s in self.d['sheets']:
            for t in s['tables']:
                if t['name'] == name:
                    return t
        return None

    def sheet_of(self, t):
        for s in self.d['sheets']:
            if any(x is t for x in s['tables']):
                return s
        return None

    def table(self, name):
        t = self.find_table(name)
        if t is None:
            raise WbError('unknown_ref', 'there is no table %r (tables: %s).'
                          % (name, ', '.join(x['name'] for s in self.d['sheets'] for x in s['tables'])))
        return t

    def column(self, t, name):
        names = [c['name'] for c in t['columns']]
        if name not in names:
            raise WbError('unknown_ref', 'table %s has no column %r (columns: %s).' % (t['name'], name, ', '.join(names)))
        return names.index(name)

    def changed(self):
        self._ev = None

    def ev(self):
        if self._ev is None:
            self._ev = Evaluator(self)
        return self._ev

    def value(self, t, ri, ci):
        return self.ev().column(t, ci)[ri]

    # ---- cell addressing
    def locate(self, sheet_name, addr):
        s = self.sheet(sheet_name)
        c, r = parse_addr(addr)
        for t in s['tables']:
            c0, r0 = anchor_rc(t)
            if c0 <= c < c0 + len(t['columns']) and r0 <= r <= r0 + len(t['rows']):
                if r == r0:
                    raise WbError('header_cell', '%s!%s is the header of table %s; header cells are column names, '
                                                 'changed only through the column definitions.' % (sheet_name, addr, t['name']))
                return t, r - r0 - 1, c - c0
        raise WbError('outside_table', '%s!%s is not a data cell of any table on sheet %s.' % (sheet_name, addr, sheet_name))

    def set_cell(self, t, ri, ci, v, where=''):
        col = t['columns'][ci]
        if col.get('formula'):
            raise WbError('formula_cell', '%s[%s] is a formula column; its cells are computed and cannot be set%s.'
                          % (t['name'], col['name'], (' (%s)' % where) if where else ''))
        t['rows'][ri]['cells'][ci] = check_value(col, v, where)
        self.changed()

    def row_from_dict(self, t, d, where):
        if not isinstance(d, dict):
            raise WbError('bad_op', '%s: a row is an object {column: value}, not %r.' % (where, d))
        cells = [None] * len(t['columns'])
        for k, v in d.items():
            ci = self.column(t, k)
            col = t['columns'][ci]
            if col.get('formula'):
                if v is None or v == '':
                    continue
                raise WbError('formula_cell', '%s: %s[%s] is a formula column; leave it out of new rows.'
                              % (where, t['name'], k))
            cells[ci] = check_value(col, v, where)
        return {'orig': None, 'cells': cells}

    # ---- range operations (write shape A; the code API calls these too)
    def op_set(self, rng, values):
        if not isinstance(rng, str) or '!' not in rng:
            raise WbError('bad_range', 'a range names its sheet: "Sheet!D70" or "Sheet!D70:E71", not %r.' % (rng,))
        sh, a = rng.rsplit('!', 1)
        sh = sh.strip().strip("'")
        parts = a.split(':')
        if len(parts) > 2:
            raise WbError('bad_range', 'bad range %r.' % rng)
        c1, r1 = parse_addr(parts[0])
        c2, r2 = parse_addr(parts[-1])
        if c2 < c1 or r2 < r1:
            raise WbError('bad_range', 'range %r runs backwards.' % rng)
        h, w = r2 - r1 + 1, c2 - c1 + 1
        if not (isinstance(values, list) and len(values) == h and all(isinstance(x, list) and len(x) == w for x in values)):
            raise WbError('bad_op', 'set: values must be a list of %d row(s) of %d value(s) each, the shape of %s.'
                          % (h, w, rng))
        for i in range(h):
            for j in range(w):
                addr = col_letter(c1 + j) + str(r1 + i)
                t, ri, ci = self.locate(sh, addr)
                self.set_cell(t, ri, ci, values[i][j], '%s!%s' % (sh, addr))

    def op_append_rows(self, tname, rows):
        t = self.table(tname)
        if not isinstance(rows, list):
            raise WbError('bad_op', 'append_rows: rows must be a list of objects.')
        for k, d in enumerate(rows):
            t['rows'].append(self.row_from_dict(t, d, 'append_rows row %d' % (k + 1)))
        self.changed()

    def data_row(self, t, r, what, allow_end=False):
        c0, r0 = anchor_rc(t)
        if isinstance(r, bool) or not isinstance(r, int):
            raise WbError('bad_op', '%s: a row is a sheet row number, not %r.' % (what, r))
        lo, hi = r0 + 1, r0 + len(t['rows']) + (1 if allow_end else 0)
        if not lo <= r <= hi:
            raise WbError('bad_range', '%s: row %d is not a data row of %s (rows %d-%d).'
                          % (what, r, t['name'], lo, r0 + len(t['rows'])))
        return r - lo

    def op_insert_rows(self, tname, before, rows):
        t = self.table(tname)
        i = self.data_row(t, before, 'insert_rows', allow_end=True)
        if not isinstance(rows, list):
            raise WbError('bad_op', 'insert_rows: rows must be a list of objects.')
        new = [self.row_from_dict(t, d, 'insert_rows row %d' % (k + 1)) for k, d in enumerate(rows)]
        t['rows'][i:i] = new
        self.changed()

    def op_delete_rows(self, tname, rows):
        t = self.table(tname)
        if isinstance(rows, int) and not isinstance(rows, bool):
            a = b = rows
        elif isinstance(rows, str) and re.fullmatch(r'\s*\d+\s*(:\s*\d+\s*)?', rows):
            p = [int(x) for x in rows.split(':')]
            a, b = p[0], p[-1]
        else:
            raise WbError('bad_op', 'delete_rows: rows is "57" or "57:58" (sheet row numbers), not %r.' % (rows,))
        i, j = self.data_row(t, a, 'delete_rows'), self.data_row(t, b, 'delete_rows')
        if j < i:
            raise WbError('bad_range', 'delete_rows: %r runs backwards.' % rows)
        del t['rows'][i:j + 1]
        self.changed()

    def op_fill_formula(self, tname, column, formula):
        t = self.table(tname)
        ci = self.column(t, column)
        if not isinstance(formula, str) or not formula.strip().startswith('='):
            raise WbError('bad_formula', 'fill_formula: the formula is text starting with "=", not %r.' % (formula,))
        old = t['columns'][ci].get('formula', '')
        t['columns'][ci]['formula'] = formula.strip()
        try:
            FParser(formula.strip(), t, self).parse()
        except WbError:
            t['columns'][ci]['formula'] = old
            raise
        for r in t['rows']:
            r['cells'][ci] = None
        self.changed()

    def op_set_type(self, tname, column, typ, fmt=None):
        t = self.table(tname)
        ci = self.column(t, column)
        col = t['columns'][ci]
        if typ not in TYPES:
            raise WbError('bad_type', 'set_type: the type is text, number or date, not %r.' % (typ,))
        if fmt is None or fmt == '':
            if typ == 'text':
                fmt = '@'
            elif typ == col['type']:
                fmt = col['format']
            else:
                raise WbError('bad_format', 'set_type: a %s column needs a format (%s).' % (typ, ', '.join(FORMATS[typ])))
        if fmt not in FORMATS[typ]:
            raise WbError('bad_format', 'set_type: %r is not a %s format (%s).' % (fmt, typ, ', '.join(FORMATS[typ])))
        new = {'name': col['name'], 'type': typ, 'format': fmt, 'formula': col.get('formula', '')}
        if not col.get('formula'):
            for ri, r in enumerate(t['rows']):
                v = r['cells'][ci]
                if v is None or col['type'] == typ:
                    continue
                if typ == 'text':
                    r['cells'][ci] = display(v, col)
                elif col['type'] == 'text':
                    r['cells'][ci] = parse_display(v, new, '%s row %d' % (t['name'], anchor_rc(t)[1] + 1 + ri))
                else:
                    raise WbError('type_mismatch', 'set_type: a %s column cannot become a %s column directly.'
                                  % (col['type'], typ))
        t['columns'][ci] = new
        self.changed()

    def op_sort(self, tname, keys):
        t = self.table(tname)
        if not isinstance(keys, list) or not keys:
            raise WbError('bad_op', 'sort: keys is a non-empty list of {"column", "order"}.')
        ks = []
        for k in keys:
            if isinstance(k, str):
                k = {'column': k}
            elif isinstance(k, (list, tuple)) and len(k) == 2:
                k = {'column': k[0], 'order': k[1]}
            if not isinstance(k, dict) or 'column' not in k or k.get('order', 'asc') not in ('asc', 'desc'):
                raise WbError('bad_op', 'sort: each key is {"column": name, "order": "asc" | "desc"}, not %r.' % (k,))
            ks.append((self.column(t, k['column']), k.get('order', 'asc') == 'desc'))
        vals = {ci: self.ev().column(t, ci) for ci, _ in ks}
        idx = list(range(len(t['rows'])))
        for ci, desc in reversed(ks):
            present = [i for i in idx if vals[ci][i] is not None]
            empty = [i for i in idx if vals[ci][i] is None]
            present.sort(key=lambda i: (isinstance(vals[ci][i], str), vals[ci][i]), reverse=desc)
            idx = present + empty
        t['rows'] = [t['rows'][i] for i in idx]
        self.changed()

    def op_add_column(self, tname, column):
        t = self.table(tname)
        if not isinstance(column, dict) or not isinstance(column.get('name'), str) or not column['name'].strip():
            raise WbError('bad_op', 'add_column: column is {"name", "type", "format"[, "formula"]}, not %r.' % (column,))
        unknown = set(column) - {'name', 'type', 'format', 'formula'}
        if unknown:
            raise WbError('bad_op', 'add_column: unknown column key(s) %s.' % ', '.join(sorted(unknown)))
        if column['name'].strip() in [c['name'] for c in t['columns']]:
            raise WbError('duplicate_name', 'table %s already has a column %s.' % (t['name'], column['name']))
        c = make_column(column['name'], column.get('type'), column.get('format'), column.get('formula') or '')
        t['columns'].append(c)
        for r in t['rows']:
            r['cells'].append(None)
        if c['formula']:
            try:
                FParser(c['formula'], t, self).parse()
            except WbError:
                t['columns'].pop()
                for r in t['rows']:
                    r['cells'].pop()
                raise
        self.changed()

    def op_add_sheet(self, name):
        if not isinstance(name, str) or not name.strip():
            raise WbError('bad_op', 'add_sheet: name is a non-empty string.')
        if self.find_sheet(name):
            raise WbError('duplicate_name', 'there is already a sheet %r.' % name)
        self.d['sheets'].append({'name': name, 'tables': []})

    def op_add_table(self, sheet, name, anchor, columns, rows=None):
        s = self.sheet(sheet)
        if not isinstance(name, str) or not re.fullmatch(r'[%s][%s0-9_.]*' % (NAME_CH, NAME_CH), name or ''):
            raise WbError('bad_op', 'add_table: %r is not a table name (letters, digits, _ and ., starting with a '
                                    'letter).' % (name,))
        if self.find_table(name):
            raise WbError('duplicate_name', 'there is already a table %r.' % name)
        parse_addr(anchor)
        if not isinstance(columns, list) or not columns:
            raise WbError('bad_op', 'add_table: columns is a non-empty list of {"name", "type", "format", "formula"}.')
        cols = []
        for c in columns:
            if not isinstance(c, dict) or not isinstance(c.get('name'), str) or not c['name'].strip():
                raise WbError('bad_op', 'add_table: each column is {"name", "type", "format"[, "formula"]}, not %r.' % (c,))
            unknown = set(c) - {'name', 'type', 'format', 'formula'}
            if unknown:
                raise WbError('bad_op', 'add_table: unknown column key(s) %s.' % ', '.join(sorted(unknown)))
            cols.append(make_column(c['name'], c.get('type'), c.get('format'), c.get('formula') or ''))
        t = {'name': name, 'anchor': anchor.strip().upper().replace('$', ''), 'columns': cols, 'rows': []}
        s['tables'].append(t)
        self.changed()
        for k, d in enumerate(rows or []):
            t['rows'].append(self.row_from_dict(t, d, 'add_table row %d' % (k + 1)))
        for ci, c in enumerate(cols):
            if c['formula']:
                FParser(c['formula'], t, self).parse()
        self.changed()

    # ---- whole-workbook check
    def validate(self):
        names = [s['name'] for s in self.d['sheets']]
        if len(set(names)) != len(names):
            raise WbError('duplicate_name', 'two sheets have the same name.')
        tnames = [t['name'] for s in self.d['sheets'] for t in s['tables']]
        dup = sorted({n for n in tnames if tnames.count(n) > 1})
        if dup:
            raise WbError('duplicate_name', 'two tables are named %s.' % dup[0])
        for s in self.d['sheets']:
            rects = []
            for t in s['tables']:
                cn = [c['name'] for c in t['columns']]
                if len(set(cn)) != len(cn):
                    raise WbError('duplicate_name', 'table %s has two columns with the same name.' % t['name'])
                for c in t['columns']:
                    make_column(c['name'], c['type'], c['format'], c.get('formula', ''))
                c0, r0 = anchor_rc(t)
                rects.append((t['name'], c0, r0, c0 + len(cn) - 1, r0 + len(t['rows'])))
                for ri, r in enumerate(t['rows']):
                    if len(r['cells']) != len(cn):
                        raise WbError('row_width_mismatch', 'a row of %s has %d cells for %d columns.'
                                      % (t['name'], len(r['cells']), len(cn)))
                    for ci, c in enumerate(t['columns']):
                        v = r['cells'][ci]
                        if c.get('formula'):
                            if v is not None:
                                raise WbError('formula_cell', 'a cell of formula column %s holds a value.' % c['name'])
                            continue
                        check_value(c, v, '%s!%s%d' % (s['name'], col_letter(c0 + ci), r0 + 1 + ri))
            for i in range(len(rects)):
                for j in range(i + 1, len(rects)):
                    a, b = rects[i], rects[j]
                    if a[1] <= b[3] and b[1] <= a[3] and a[2] <= b[4] and b[2] <= a[4]:
                        raise WbError('table_overlap', 'tables %s and %s overlap on sheet %s.' % (a[0], b[0], s['name']))
        self.changed()
        ev = self.ev()
        for s in self.d['sheets']:
            for t in s['tables']:
                c0, r0 = anchor_rc(t)
                for ci, c in enumerate(t['columns']):
                    if not c.get('formula'):
                        continue
                    for ri, v in enumerate(ev.column(t, ci)):
                        where = '%s!%s%d' % (s['name'], col_letter(c0 + ci), r0 + 1 + ri)
                        if isinstance(v, Err):
                            raise WbError('formula_error', 'the formula of %s[%s] gives %s in %s.'
                                          % (t['name'], c['name'], v, where))
                        if v is None:
                            continue
                        if c['type'] == 'number' and (isinstance(v, bool) or not isinstance(v, (int, float))):
                            raise WbError('type_mismatch', 'the formula of number column %s[%s] gives %r in %s.'
                                          % (t['name'], c['name'], v, where))
                        if c['type'] == 'date' and not (isinstance(v, str) and norm_date(v)):
                            raise WbError('type_mismatch', 'the formula of date column %s[%s] gives %r in %s.'
                                          % (t['name'], c['name'], v, where))

    def new_rows(self):
        return sum(1 for s in self.d['sheets'] for t in s['tables'] for r in t['rows'] if r['orig'] is None)

    def semantic(self):
        ev = self.ev()
        out = []
        for s in self.d['sheets']:
            ts = []
            for t in sorted(s['tables'], key=lambda x: x['name']):
                cols = [[c['name'], c['type'], c['format'], bool(c.get('formula'))] for c in t['columns']]
                colvals = [ev.column(t, ci) for ci in range(len(t['columns']))]
                rows = [[nv(colvals[ci][ri]) for ci in range(len(cols))] for ri in range(len(t['rows']))]
                ts.append({'name': t['name'], 'anchor': t['anchor'], 'columns': cols, 'rows': rows})
            out.append({'name': s['name'], 'tables': ts})
        return {'sheets': out}


def make_column(name, typ, fmt, formula=''):
    if typ not in TYPES:
        raise WbError('bad_type', 'column %s: the type is text, number or date, not %r.' % (name, typ))
    if typ == 'text' and fmt in (None, ''):
        fmt = '@'
    if fmt not in FORMATS[typ]:
        raise WbError('bad_format', 'column %s: %r is not a %s format (%s).' % (name, fmt, typ, ', '.join(FORMATS[typ])))
    if formula and not (isinstance(formula, str) and formula.strip().startswith('=')):
        raise WbError('bad_formula', 'column %s: a formula starts with "=".' % name)
    return {'name': name.strip(), 'type': typ, 'format': fmt, 'formula': (formula or '').strip()}


# ---------------------------------------------------------------- write shape A: range operations

OPS = {
    'set': (('range', 'values'), ()),
    'append_rows': (('table', 'rows'), ()),
    'insert_rows': (('table', 'before', 'rows'), ()),
    'delete_rows': (('table', 'rows'), ()),
    'fill_formula': (('table', 'column', 'formula'), ()),
    'set_type': (('table', 'column', 'type'), ('format',)),
    'sort': (('table', 'keys'), ()),
    'add_column': (('table', 'column'), ()),
    'add_table': (('sheet', 'name', 'anchor', 'columns'), ('rows',)),
    'add_sheet': (('name',), ()),
}


def strip_fence(text):
    s = text.strip()
    m = re.fullmatch(r'```[\w-]*\n(.*)\n```', s, re.S)
    return m.group(1) if m else s


def run_ops(book, text):
    try:
        ops = json.loads(strip_fence(text))
    except ValueError as e:
        raise WbError('bad_json', 'the answer text is not a JSON list of operations: %s.' % e)
    if not isinstance(ops, list):
        raise WbError('bad_json', 'the answer text must be a JSON list of operations.')
    for k, op in enumerate(ops, 1):
        if not isinstance(op, dict) or op.get('op') not in OPS:
            raise WbError('bad_op', 'operation %d: "op" is one of %s.' % (k, ', '.join(OPS)))
        req, opt = OPS[op['op']]
        missing = [a for a in req if a not in op]
        extra = [a for a in op if a != 'op' and a not in req + opt]
        if missing or extra:
            raise WbError('bad_op', 'operation %d (%s) takes %s%s; %s.' % (
                k, op['op'], ', '.join(req), (' and optionally ' + ', '.join(opt)) if opt else '',
                ('missing ' + ', '.join(missing)) if missing else ('unknown ' + ', '.join(extra))))
        args = [op[a] for a in req] + [op.get(a) for a in opt]
        try:
            getattr(book, 'op_' + op['op'])(*args)
        except WbError as e:
            raise WbError(e.flag, 'operation %d (%s): %s' % (k, op['op'], e.msg))


# ---------------------------------------------------------------- write shape C: code against a small API

FORBIDDEN_NAMES = {'open', 'eval', 'exec', 'compile', '__import__', 'getattr', 'setattr', 'delattr', 'globals',
                   'locals', 'vars', 'input', 'breakpoint', 'exit', 'quit', 'help', 'dir', 'type', 'object', 'super',
                   'memoryview', 'classmethod', 'staticmethod', 'property', 'os', 'sys', 'subprocess', 'socket',
                   'urllib', 'requests', 'importlib', 'builtins', 'file'}
SAFE_BUILTINS = {n: __builtins__[n] if isinstance(__builtins__, dict) else getattr(__builtins__, n) for n in (
    'range', 'len', 'list', 'dict', 'str', 'int', 'float', 'round', 'min', 'max', 'sum', 'enumerate', 'zip',
    'sorted', 'abs', 'tuple', 'set', 'any', 'all', 'reversed', 'bool', 'isinstance', 'ValueError', 'KeyError',
    'Exception')}
CODE_LINE_LIMIT = 2_000_000
CODE_TIME_LIMIT = 5.0


class ApiError(Exception):
    pass


def _wrap(fn):
    try:
        return fn()
    except WbError as e:
        raise ApiError(e.flag, e.msg)


class Cell:
    def __init__(self, book, sheet, addr):
        self.__dict__['_b'] = (book, sheet, addr)

    @property
    def value(self):
        book, sheet, addr = self.__dict__['_b']
        t, ri, ci = _wrap(lambda: book.locate(sheet, addr))
        return book.value(t, ri, ci)

    @value.setter
    def value(self, v):
        book, sheet, addr = self.__dict__['_b']
        _wrap(lambda: book.op_set('%s!%s' % (sheet, addr), [[v]]))


class Row:
    def __init__(self, book, t, r):
        self.__dict__['_b'] = (book, t, r)

    def _idx(self):
        book, t, r = self.__dict__['_b']
        for i, x in enumerate(t['rows']):
            if x is r:
                return i
        raise ApiError('runtime_error', 'this row has been deleted.')

    @property
    def row(self):
        book, t, r = self.__dict__['_b']
        return anchor_rc(t)[1] + 1 + self._idx()

    def __getitem__(self, col):
        book, t, r = self.__dict__['_b']
        ci = _wrap(lambda: book.column(t, col))
        return book.value(t, self._idx(), ci)

    def __setitem__(self, col, v):
        book, t, r = self.__dict__['_b']
        ci = _wrap(lambda: book.column(t, col))
        _wrap(lambda: book.set_cell(t, self._idx(), ci, v, '%s row %d' % (t['name'], self.row)))


class Table:
    def __init__(self, book, t):
        self.__dict__['_b'] = (book, t)

    @property
    def name(self):
        return self.__dict__['_b'][1]['name']

    @property
    def columns(self):
        return [c['name'] for c in self.__dict__['_b'][1]['columns']]

    @property
    def rows(self):
        book, t = self.__dict__['_b']
        return [Row(book, t, r) for r in t['rows']]

    def append(self, values):
        book, t = self.__dict__['_b']
        _wrap(lambda: book.op_append_rows(t['name'], [values]))

    def insert(self, before, values):
        book, t = self.__dict__['_b']
        _wrap(lambda: book.op_insert_rows(t['name'], before, [values]))

    def delete_rows(self, first, last=None):
        book, t = self.__dict__['_b']
        _wrap(lambda: book.op_delete_rows(t['name'], '%d:%d' % (first, first if last is None else last)
                                          if isinstance(first, int) and not isinstance(first, bool) else first))

    def fill_formula(self, column, formula):
        book, t = self.__dict__['_b']
        _wrap(lambda: book.op_fill_formula(t['name'], column, formula))

    def set_type(self, column, type, format=None):
        book, t = self.__dict__['_b']
        _wrap(lambda: book.op_set_type(t['name'], column, type, format))

    def add_column(self, name, type, format=None, formula=None):
        book, t = self.__dict__['_b']
        c = {'name': name, 'type': type}
        if format is not None:
            c['format'] = format
        if formula:
            c['formula'] = formula
        _wrap(lambda: book.op_add_column(t['name'], c))

    def sort(self, keys):
        book, t = self.__dict__['_b']
        _wrap(lambda: book.op_sort(t['name'], list(keys) if isinstance(keys, (list, tuple)) else [keys]))


class Sheet:
    def __init__(self, book, s):
        self.__dict__['_b'] = (book, s)

    @property
    def name(self):
        return self.__dict__['_b'][1]['name']

    def cell(self, addr):
        book, s = self.__dict__['_b']
        _wrap(lambda: parse_addr(addr))
        return Cell(book, s['name'], addr)

    def table(self, name):
        book, s = self.__dict__['_b']
        for t in s['tables']:
            if t['name'] == name:
                return Table(book, t)
        raise ApiError('unknown_ref', 'sheet %s has no table %r.' % (s['name'], name))

    def add_table(self, name, anchor, columns, rows=None):
        book, s = self.__dict__['_b']
        _wrap(lambda: book.op_add_table(s['name'], name, anchor, columns, rows))
        return Table(book, book.find_table(name))


class Workbook:
    def __init__(self, book):
        self.__dict__['_b'] = book

    def __getitem__(self, name):
        book = self.__dict__['_b']
        return Sheet(book, _wrap(lambda: book.sheet(name)))

    @property
    def sheets(self):
        return [s['name'] for s in self.__dict__['_b'].d['sheets']]

    def table(self, name):
        book = self.__dict__['_b']
        return Table(book, _wrap(lambda: book.table(name)))

    def add_sheet(self, name):
        book = self.__dict__['_b']
        _wrap(lambda: book.op_add_sheet(name))
        return Sheet(book, book.find_sheet(name))


def check_code(code):
    try:
        tree = ast.parse(code)
    except SyntaxError as e:
        raise WbError('syntax_error', 'the code does not parse: %s (line %s).' % (e.msg, e.lineno))
    for node in ast.walk(tree):
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            raise WbError('sandbox_violation', 'line %d: imports are not allowed.' % node.lineno)
        if isinstance(node, ast.Attribute) and node.attr.startswith('_'):
            raise WbError('sandbox_violation', 'line %d: attributes starting with "_" are not allowed.' % node.lineno)
        if isinstance(node, ast.Name) and (node.id.startswith('__') or node.id in FORBIDDEN_NAMES):
            raise WbError('sandbox_violation', 'line %d: %s is not available (no files, network, imports or '
                                               'introspection).' % (node.lineno, node.id))
        if isinstance(node, (ast.ClassDef, ast.Global, ast.Nonlocal, ast.AsyncFunctionDef, ast.Await)):
            raise WbError('sandbox_violation', 'line %d: %s is not allowed.' % (node.lineno, type(node).__name__))
    return tree


def run_code(book, code):
    code = strip_fence(code)
    tree = check_code(code)
    wbo = Workbook(book)
    env = {'__builtins__': dict(SAFE_BUILTINS, print=lambda *a, **k: None), 'wb': wbo, 'table': wbo.table}
    counter = [0]
    start = time.monotonic()

    def tracer(frame, event, arg):
        if frame.f_code.co_filename == '<answer>':
            counter[0] += 1
            if counter[0] > CODE_LINE_LIMIT or time.monotonic() - start > CODE_TIME_LIMIT:
                raise TimeoutError()
            return tracer
        return None

    old = sys.gettrace()
    sys.settrace(tracer)
    try:
        exec(compile(tree, '<answer>', 'exec'), env)
    except TimeoutError:
        raise WbError('sandbox_violation', 'the code ran past the time limit (%.0f s).' % CODE_TIME_LIMIT)
    except ApiError as e:
        raise WbError(e.args[0], e.args[1])
    except WbError:
        raise
    except NameError as e:
        raise WbError('runtime_error', 'the code failed: %s.' % e)
    except Exception as e:
        raise WbError('runtime_error', 'the code failed: %s: %s.' % (type(e).__name__, e))
    finally:
        sys.settrace(old)


# ---------------------------------------------------------------- text renderings

def row_line(cells):
    return '| ' + ' | '.join(cells) + ' |'


def structure_lines(t, rng=None):
    out = ['<table name="%s" range="%s">' % (t['name'], rng or table_range(t)), row_line(['column', 'type', 'format', 'formula']),
           '|---|---|---|---|']
    for c in t['columns']:
        out.append(row_line([c['name'], c['type'], c['format'], c.get('formula', '')]))
    out.append('</table>')
    return out


def window_text(book, labels='orig', ranges=None, blank_new_formulas=False):
    """Read view A / the text of write shape B: structure plus every row as a pipe table with a row column.
    labels='orig' writes each row's original label (the seed's sheet row; empty for a row added since);
    labels='sheet' writes the current sheet row numbers. ranges maps a table name to the range text to write
    (the text form never needs a range end updated by hand); blank_new_formulas leaves the formula cells of added
    rows empty, as a person writing the text would."""
    ev = book.ev()
    parts = []
    for s in book.d['sheets']:
        parts.append('<sheet name="%s">' % s['name'])
        for t in s['tables']:
            parts.append('')
            parts.extend(structure_lines(t, (ranges or {}).get(t['name'])))
            parts.append('')
            c0, r0 = anchor_rc(t)
            cols = t['columns']
            parts.append('<data table="%s">' % t['name'])
            parts.append(row_line(['row'] + [c['name'] for c in cols]))
            parts.append('|' + '---|' * (len(cols) + 1))
            colvals = [ev.column(t, ci) for ci in range(len(cols))]
            for ri, r in enumerate(t['rows']):
                lab = r['orig'] if labels == 'orig' else r0 + 1 + ri
                blank_f = blank_new_formulas and r['orig'] is None
                parts.append(row_line(['' if lab is None else str(lab)] +
                                      [('' if blank_f and cols[ci].get('formula') else display(colvals[ci][ri], cols[ci]))
                                       for ci in range(len(cols))]))
            parts.append('</data>')
        parts.append('</sheet>')
        parts.append('')
    return '\n'.join(parts)


def ranges_of(rows):
    """Sorted row numbers -> 'a:b' runs."""
    out = []
    for r in rows:
        if out and out[-1][1] == r - 1:
            out[-1][1] = r
        else:
            out.append([r, r])
    return out


def index_text(book):
    """Read view B: structure plus, per table, an inverted index of each column (SheetCompressor-style)."""
    ev = book.ev()
    parts = []
    for s in book.d['sheets']:
        parts.append('<sheet name="%s">' % s['name'])
        for t in s['tables']:
            parts.append('')
            parts.extend(structure_lines(t))
            parts.append('')
            c0, r0 = anchor_rc(t)
            cols = t['columns']
            colvals = [ev.column(t, ci) for ci in range(len(cols))]
            n = len(t['rows'])
            blank = [r0 + 1 + ri for ri in range(n) if all(colvals[ci][ri] is None for ci in range(len(cols)))]
            attrs = 'table="%s"' % t['name']
            if n:
                attrs += ' rows="%d:%d"' % (r0 + 1, r0 + n)
            if blank:
                attrs += ' blank="%s"' % ','.join(('%d' % a) if a == b else ('%d:%d' % (a, b))
                                                   for a, b in ranges_of(blank))
            parts.append('<index %s>' % attrs)
            for ci, c in enumerate(cols):
                L = col_letter(c0 + ci)
                order, cells = [], {}
                for ri in range(n):
                    v = colvals[ci][ri]
                    if v is None:
                        continue
                    key = raw(v, c)
                    assert '|' not in key and ': ' not in key, key
                    if key not in cells:
                        order.append(key)
                        cells[key] = []
                    cells[key].append(r0 + 1 + ri)
                entries = []
                for key in order:
                    addrs = ','.join(('%d' % a) if a == b else ('%d:%d' % (a, b)) for a, b in ranges_of(cells[key]))
                    entries.append('%s: %s' % (key, addrs))
                parts.append(' | '.join(['%s %s' % (L, c['name'])] + entries))
            parts.append('</index>')
        parts.append('</sheet>')
        parts.append('')
    return '\n'.join(parts)


# ---------------------------------------------------------------- write shape B: parse the edited window

SHEET_OPEN = re.compile(r'^<sheet\s+name="([^"]+)"\s*>$')
TABLE_OPEN = re.compile(r'^<table\s+name="([^"]+)"\s+range="([^"]+)"\s*>$')
DATA_OPEN = re.compile(r'^<data\s+table="([^"]+)"\s*>$')


def split_row(line):
    s = line.strip()
    if not (s.startswith('|') and s.endswith('|')) or len(s) < 2:
        return None
    return [c.strip() for c in s[1:-1].split('|')]


def is_delim(cells):
    return cells is not None and all(re.fullmatch(r':?-{3,}:?', c) for c in cells)


def parse_window(text, seed_book):
    """The edited window text of write shape B -> Book (rows carry their labels as orig). Raises WbError."""
    lines = text.split('\n')
    seed_labels = {t['name']: [r['orig'] for r in t['rows']] for s in seed_book.d['sheets'] for t in s['tables']}
    sheets = []
    i = 0
    cur = None
    specs = {}

    def err(flag, ln, msg):
        return WbError(flag, 'line %d: %s' % (ln + 1, msg))

    while i < len(lines):
        line = lines[i].strip()
        if not line:
            i += 1
            continue
        m = SHEET_OPEN.match(line)
        if m:
            if cur is not None:
                raise err('window_form', i, '<sheet> inside a sheet; close the previous sheet with </sheet>.')
            cur = {'name': m.group(1), 'tables': []}
            sheets.append(cur)
            i += 1
            continue
        if line == '</sheet>':
            if cur is None:
                raise err('window_form', i, '</sheet> without <sheet>.')
            for t in cur['tables']:
                if 'rows' not in t:
                    raise err('window_form', i, 'table %s has no <data table="%s"> block in its sheet.'
                              % (t['name'], t['name']))
            cur = None
            i += 1
            continue
        if cur is None:
            raise err('window_form', i, 'text outside a <sheet> block: %r.' % line[:40])
        m = TABLE_OPEN.match(line)
        if m:
            name, rng = m.group(1), m.group(2)
            rm = re.fullmatch(r'\$?([A-Z]{1,3})\$?([1-9]\d*)(:\$?[A-Z]{1,3}\$?\d+)?', rng.strip())
            if not rm:
                raise err('window_form', i, 'range="%s" is not a range such as A1:F153 (or its top-left cell, A1).'
                          % rng)
            j = i + 1
            rows = []
            while j < len(lines) and lines[j].strip() != '</table>':
                if not lines[j].strip():
                    raise err('window_form', j, 'blank line inside <table>; the column lines end with </table>.')
                rows.append((j, split_row(lines[j])))
                j += 1
            if j >= len(lines):
                raise err('window_form', i, '<table name="%s"> is not closed by </table>.' % name)
            if len(rows) < 2 or rows[0][1] != ['column', 'type', 'format', 'formula'] or not is_delim(rows[1][1]):
                raise err('window_form', i + 1, 'a <table> block starts with | column | type | format | formula | and '
                                                'a delimiter row.')
            cols = []
            for ln, cells in rows[2:]:
                if cells is None or len(cells) != 4:
                    raise err('row_width_mismatch', ln, 'a column line has four cells: name, type, format, formula.')
                try:
                    cols.append(make_column(cells[0], cells[1], cells[2], cells[3]))
                except WbError as e:
                    raise err(e.flag, ln, e.msg)
            t = {'name': name, 'anchor': rm.group(1) + rm.group(2), 'columns': cols}
            if name in specs:
                raise err('duplicate_name', i, 'two tables are named %s.' % name)
            specs[name] = t
            cur['tables'].append(t)
            i = j + 1
            continue
        m = DATA_OPEN.match(line)
        if m:
            name = m.group(1)
            t = specs.get(name)
            if t is None or not any(x is t for x in cur['tables']):
                raise err('window_form', i, '<data table="%s"> comes before or without its <table> block in this sheet.'
                          % name)
            if 'rows' in t:
                raise err('window_form', i, 'table %s has two <data> blocks.' % name)
            j = i + 1
            rows = []
            while j < len(lines) and lines[j].strip() != '</data>':
                if not lines[j].strip():
                    raise err('window_form', j, 'blank line inside <data>; the rows end with </data>.')
                rows.append((j, split_row(lines[j])))
                j += 1
            if j >= len(lines):
                raise err('window_form', i, '<data table="%s"> is not closed by </data>.' % name)
            want = ['row'] + [c['name'] for c in t['columns']]
            if len(rows) < 2 or rows[0][1] != want or not is_delim(rows[1][1]):
                raise err('data_header_mismatch', i + 1, 'the <data> block of %s starts with %s and a delimiter row '
                                                         '(the row column, then the columns of its <table> block in '
                                                         'order).' % (name, row_line(want)))
            allowed = seed_labels.get(name, [])
            allowed_set = set(allowed)
            last = 0
            out_rows = []
            c0, r0 = parse_addr(t['anchor'])
            for ln, cells in rows[2:]:
                if cells is None:
                    raise err('window_form', ln, 'a row line starts and ends with |.')
                if len(cells) != len(want):
                    raise err('row_width_mismatch', ln, 'this row has %d cells; %s has %d (the row column and %d '
                                                        'columns).' % (len(cells), name, len(want), len(want) - 1))
                lab = cells[0]
                if lab == '':
                    orig = None
                elif re.fullmatch(r'\d+', lab) and int(lab) in allowed_set and int(lab) > last:
                    orig = int(lab)
                    last = orig
                else:
                    raise err('row_labels_edited', ln, 'row label %r was changed or invented. Row labels are '
                                                       'read-only: keep each existing row\'s label, in order, and '
                                                       'leave the row cell empty on a new row.' % lab)
                vals = []
                for ci, c in enumerate(t['columns']):
                    if c['formula']:
                        vals.append(None)    # computed; whatever is written there is ignored
                        continue
                    try:
                        vals.append(parse_display(cells[ci + 1], c, 'line %d, column %s' % (ln + 1, c['name'])))
                    except WbError as e:
                        raise err(e.flag, ln, e.msg)
                out_rows.append({'orig': orig, 'cells': vals})
            t['rows'] = out_rows
            i = j + 1
            continue
        raise err('window_form', i, 'unexpected line %r; a sheet holds <table> … </table> and <data> … </data> '
                                    'blocks.' % line[:40])
    if cur is not None:
        raise WbError('window_form', 'the last <sheet> is not closed by </sheet>.')
    for s in sheets:
        for t in s['tables']:
            if 'rows' not in t:
                raise WbError('window_form', 'table %s has no <data> block.' % t['name'])
    b = Book({'sheets': sheets})
    for s in sheets:
        for t in s['tables']:
            for c in t['columns']:
                if c['formula']:
                    FParser(c['formula'], t, b).parse()
    return b
