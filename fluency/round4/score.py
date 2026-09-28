#!/usr/bin/env python3
"""Scorer for round 4 of the hanji fluency test kit (DESIGN.md §10 item 6).

usage: python3 score.py <unit_id> <answers.json>

answers.json: {"answers": [{"task_id": ..., "text": ...} | {"task_id": ..., "edits": [{"old", "new"}]}]}
              readview:     "text": "ANSWER: <value>"
              writeshape A: "text": a JSON list of operations (or a string holding one)
              writeshape B: "edits": [{"old", "new"}] applied to the workbook text
              writeshape C: "text": Python code
              a task may be refused: {"task_id": ..., "text": "REFUSE: <reason>", "edits": []}
prints:       {"results": [{"task_id", "valid", "landed", "error", "chars", "flags", "detail"}]}

Every write shape is applied to one in-memory workbook model (wb.py: sheets -> tables -> typed columns -> rows,
formulas as structured-reference text). valid: the answer parses or runs and the result is a well-typed workbook
(types, formats, structured formulas that evaluate without an error, no overlapping tables, at most 50 new rows).
landed: the result's semantic model (formula columns compared by their computed values) equals the gold's.
A read answer is compared after normalising whitespace and digit grouping only.

Reuses round 2's edit application (../round2/score.py) by import; rounds 1-3 are not changed.
"""
import importlib.util
import json
import os
import re
import sys

import wb as W

HERE = os.path.dirname(os.path.abspath(__file__))
UNITS = os.path.join(HERE, 'data', 'units')
GOLD = os.path.join(HERE, 'data', 'gold')

_spec = importlib.util.spec_from_file_location('hanji_r2score', os.path.join(HERE, '..', 'round2', 'score.py'))
r2 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(r2)
apply_edits = r2.apply_edits
first_diff = r2.first_diff
REFUSE_PREFIX = 'REFUSE:'
ANSWER_RE = re.compile(r'^\s*ANSWER\s*:\s*(.*?)\s*$', re.I | re.S)


def load_unit(unit_id):
    with open(os.path.join(UNITS, unit_id + '.json'), encoding='utf-8') as f:
        return json.load(f)


def load_gold(unit_id):
    with open(os.path.join(GOLD, unit_id + '.json'), encoding='utf-8') as f:
        return {a['task_id']: a for a in json.load(f)['answers']}


def is_refusal(a):
    t = a.get('text')
    return isinstance(t, str) and t.strip().upper().startswith(REFUSE_PREFIX)


# ---------------------------------------------------------------- read answers

def norm_answer(s):
    """Whitespace and digit grouping only: runs of whitespace become one space (none next to a comma), and a comma
    between digits that is followed by exactly three digits is dropped."""
    s = re.sub(r'\s+', ' ', s.strip())
    s = re.sub(r'\s*,\s*', ',', s)
    prev = None
    while prev != s:
        prev = s
        s = re.sub(r'(?<=\d),(?=\d{3}(?!\d))', '', s)
    return s


def score_question(task, a, g, res):
    text = a.get('text')
    if not isinstance(text, str):
        res['error'] = 'answer a question with "text": "ANSWER: <value>".'
        res['flags'] = ['wrong_answer_kind']
        return
    res['chars'] = len(text)
    if is_refusal(a):
        res['valid'] = True
        res['flags'] = ['wrong_refusal']
        res['detail'] = 'refused a question that the workbook answers.'
        return
    m = ANSWER_RE.match(text)
    if not m or not m.group(1):
        res['error'] = 'the text must be "ANSWER: <value>" (or "REFUSE: <reason>").'
        res['flags'] = ['answer_form']
        return
    res['valid'] = True
    got = norm_answer(m.group(1))
    accept = [norm_answer(x) for x in g['accept']]
    if got in accept:
        res['landed'] = True
        return
    flags = {'wrong_answer'}
    for x in accept:
        if x.startswith('0') and got == x.lstrip('0'):
            flags.add('id_lost_zeros')
    res['flags'] = sorted(flags)
    res['detail'] = 'answered %r.' % m.group(1)


# ---------------------------------------------------------------- write answers

def apply_answer(unit, a):
    """-> (Book, chars) or raises W.WbError."""
    cand = unit['candidate']
    seed = W.Book(unit['book'])
    if cand == 'B':
        edits = a.get('edits')
        if not isinstance(edits, list):
            raise W.WbError('wrong_answer_kind', 'answer with "edits", a list of {"old", "new"} pairs (or refuse with '
                                                 '"text": "REFUSE: <reason>" and "edits": []).')
        text, flag, msg = apply_edits(unit['view'], edits)
        if flag:
            raise W.WbError(flag, msg)
        book = W.parse_window(text, seed)
    else:
        text = a.get('text')
        if cand == 'A' and isinstance(text, list):
            text = json.dumps(text, ensure_ascii=False)
        if not isinstance(text, str) or not text.strip():
            raise W.WbError('wrong_answer_kind', 'answer with "text": %s.' % (
                'a JSON list of operations' if cand == 'A' else 'Python code'))
        if cand == 'A':
            book = seed
            W.run_ops(book, text)
        else:
            book = seed
            W.run_code(book, text)
    n = book.new_rows()
    if n > W.MAX_NEW_ROWS:
        raise W.WbError('bulk_typed', 'the answer adds %d rows; no answer adds more than %d rows (bulk rows come from '
                                      'an import).' % (n, W.MAX_NEW_ROWS))
    book.validate()
    return book


def answer_chars(unit, a):
    if unit['candidate'] == 'B' and isinstance(a.get('edits'), list) and a['edits']:
        return sum(len(e.get('old') or '') + len(e.get('new') or '') for e in a['edits'] if isinstance(e, dict))
    t = a.get('text')
    if isinstance(t, list):
        return len(json.dumps(t, ensure_ascii=False))
    return len(t) if isinstance(t, str) else 0


def walk_cells(got, want):
    """Yield (table name, row, col, got value, want value) for tables of the same name and shape."""
    gt = {t['name']: t for s in got['sheets'] for t in s['tables']}
    for s in want['sheets']:
        for t in s['tables']:
            g = gt.get(t['name'])
            if not g or len(g['rows']) != len(t['rows']) or len(g['columns']) != len(t['columns']):
                continue
            for ri, (gr, wr) in enumerate(zip(g['rows'], t['rows'])):
                for ci, (gv, wv) in enumerate(zip(gr, wr)):
                    yield t['name'], ri, ci, gv, wv


def diagnose(got, want, seed):
    flags = set()
    if got == seed:
        flags.add('no_change')
    for _, _, _, gv, wv in walk_cells(got, want):
        if isinstance(wv, str) and wv.startswith('0') and wv.isdigit() and \
                (gv == wv.lstrip('0') or (isinstance(gv, int) and str(gv) == wv.lstrip('0'))):
            flags.add('id_lost_zeros')
    gcols = {(t['name'], c[0]): c for s in got['sheets'] for t in s['tables'] for c in t['columns']}
    for s in want['sheets']:
        for t in s['tables']:
            for c in t['columns']:
                g = gcols.get((t['name'], c[0]))
                if g and c[3] and not g[3]:
                    flags.add('formula_missing')
    if not flags:
        diff = [(n, ri, ci) for n, ri, ci, gv, wv in walk_cells(got, want) if gv != wv]
        if diff:
            flags.add('wrong_cell')
    return flags


def score_write(unit, task, a, g, res):
    gold_refuses = is_refusal(g)
    res['chars'] = answer_chars(unit, a)
    if is_refusal(a):
        res['chars'] = len(a['text'])
        if a.get('edits'):
            res['error'] = 'a refusal is "text": "REFUSE: <reason>" with "edits": []; this answer has both.'
            res['flags'] = ['bad_refusal']
            return
        res['valid'] = True
        if gold_refuses:
            res['landed'] = True
        else:
            res['flags'] = ['wrong_refusal']
            res['detail'] = 'refused a task that the documentation allows.'
        return
    try:
        book = apply_answer(unit, a)
    except W.WbError as e:
        res['error'] = e.msg
        res['flags'] = [e.flag]
        if gold_refuses:
            res['flags'].append('should_refuse')
        return
    res['valid'] = True
    got = book.semantic()
    seed = W.Book(unit['book']).semantic()
    if gold_refuses:
        res['flags'] = ['should_refuse']
        res['detail'] = 'the documentation does not allow what was asked; the intended answer is a refusal.'
        return
    want = apply_answer(unit, g).semantic()
    if got == want:
        res['landed'] = True
        return
    res['flags'] = sorted({'not_landed'} | diagnose(got, want, seed))
    res['detail'] = 'result differs from the intended workbook: ' + first_diff(got, want)


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
        if task['type'] == 'question':
            score_question(task, a, gold[tid], res)
        else:
            score_write(unit, task, a, gold[tid], res)
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
