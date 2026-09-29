#!/usr/bin/env python3
"""The expected results of round 4's gold range operations (fluency/round4,
write shape A), computed by the round's own workbook model (wb.py): for each
write and edit task, every table after the operations — its anchor, its
columns and every row as displayed. tests/round4.rs builds each seed as a
real xlsx, applies the same operations with hanji-xlsx and compares.
Run: python3 make_expected.py (writes expected.json next to it)."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
R4 = os.path.normpath(os.path.join(HERE, '..', '..', '..', '..', 'fluency', 'round4'))
sys.path.insert(0, R4)
import wb as W  # noqa: E402


def tables(book):
    ev = book.ev()
    out = []
    for s in book.d['sheets']:
        for t in s['tables']:
            c0, r0 = W.anchor_rc(t)
            cols = t['columns']
            vals = [ev.column(t, ci) for ci in range(len(cols))]
            rows = [[r0 + 1 + ri] + [W.display(vals[ci][ri], cols[ci]) for ci in range(len(cols))] for ri in range(len(t['rows']))]
            out.append({'sheet': s['name'], 'name': t['name'], 'range': W.table_range(t),
                        'columns': [[c['name'], c['type'], c['format'], bool(c.get('formula'))] for c in cols],
                        'rows': rows})
    return out


def main():
    out = {}
    for rep in (1, 2, 3):
        unit = json.load(open(os.path.join(R4, 'data', 'units', 'r4-writeshape-A-%d.json' % rep)))
        gold = json.load(open(os.path.join(R4, 'data', 'gold', 'r4-writeshape-A-%d.json' % rep)))
        tasks = []
        for a in gold['answers']:
            if isinstance(a.get('text'), str):
                tasks.append({'task_id': a['task_id'], 'refuse': a['text']})
                continue
            book = W.Book(unit['book'])
            W.run_ops(book, json.dumps(a['text'], ensure_ascii=False))
            book.validate()
            tasks.append({'task_id': a['task_id'], 'ops': a['text'], 'tables': tables(book)})
        out['r4-writeshape-A-%d' % rep] = {'seed': tables(W.Book(unit['book'])), 'tasks': tasks}
    with open(os.path.join(HERE, 'expected.json'), 'w') as f:
        json.dump(out, f, ensure_ascii=False, separators=(',', ':'))
        f.write('\n')


if __name__ == '__main__':
    main()
