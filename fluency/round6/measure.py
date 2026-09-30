#!/usr/bin/env python3
"""Size cost of each candidate on the whole corpus: the candidate's text over today's text, per file.

Usage: python3 measure.py <dump dir>    (the dump dir holds today's text of every corpus file, <name>.<ext>.txt, as the
engines' examples print it: see README.md). Writes data/measure.json and prints a table.

Flow documents (docx, hwpx): doc.attach + render, with GetPut checked on each (every candidate reads its own text
back to F1's effective formatting). Presentations and workbooks: canvas.py and grid.py."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..'))
sys.path.insert(0, HERE)

import doc as D  # noqa: E402
import extract as X  # noqa: E402

CORPORA = {
    'hwpx': 'crates/hanji-hwpx/corpus',
    'docx': 'prototype/remainder/corpus',
    'pptx': 'crates/hanji-pptx/corpus',
    'xlsx': 'crates/hanji-xlsx/corpus',
}


def flow(dumps):
    rows = []
    for fmt in ('hwpx', 'docx'):
        d = os.path.join(ROOT, CORPORA[fmt])
        for f in sorted(os.listdir(d)):
            if not f.endswith('.' + fmt):
                continue
            tp = os.path.join(dumps, f + '.txt')
            if not os.path.exists(tp) or os.path.getsize(tp) == 0:
                continue
            text = open(tp, encoding='utf-8').read()
            row = {'file': f, 'format': fmt, 'today': len(text)}
            try:
                m = D.attach(text, X.extract(os.path.join(d, f)), fmt)
                D.normalize(m)
                truth = [e for e in D.effective(D.render(m, 'F1'), 'F1', m)[0] if e[0] != 'R']
                for c in D.CANDS:
                    t = D.render(m, c)
                    row[c] = len(t)
                    e = [x for x in D.effective(t, c, m)[0] if x[0] != 'R']
                    row['getput_' + c] = e == truth
                row['blocks'] = m.stats['blocks']
                row['matched'] = m.stats['matched']
            except Exception as ex:  # a file the kit cannot read is reported, not hidden
                row['error'] = '%s: %s' % (type(ex).__name__, str(ex)[:120])
            rows.append(row)
    return rows


def other(dumps):
    rows = []
    try:
        import canvas
        rows += canvas.measure(dumps, os.path.join(ROOT, CORPORA['pptx']))
    except ImportError:
        pass
    try:
        import grid
        rows += grid.measure(dumps, os.path.join(ROOT, CORPORA['xlsx']))
    except ImportError:
        pass
    return rows


def main():
    dumps = sys.argv[1]
    rows = flow(dumps) + other(dumps)
    json.dump(rows, open(os.path.join(HERE, 'data', 'measure.json'), 'w', encoding='utf-8'), indent=1)
    tot = {}
    print('%-44s %-5s %8s %8s %8s %8s  %s' % ('file', 'fmt', 'today', 'F1', 'F2', 'F3', 'getput'))
    for r in rows:
        if 'error' in r:
            print('%-44s %-5s %8d  error: %s' % (r['file'], r['format'], r['today'], r['error']))
            continue
        cands = [c for c in ('F1', 'F2', 'F2o', 'F3') if c in r]
        print('%-44s %-5s %8d %s  %s' % (r['file'][:44], r['format'], r['today'],
                                         ' '.join('%8s' % ('%.2f' % (r[c] / r['today'])) for c in cands),
                                         all(r.get('getput_' + c, True) for c in cands)))
        t = tot.setdefault(r['format'], {'today': 0, 'n': 0})
        t['today'] += r['today']
        t['n'] += 1
        for c in cands:
            t[c] = t.get(c, 0) + r[c]
    print()
    for fmt, t in tot.items():
        print('%-5s %d files, today %d chars: %s' % (fmt, t['n'], t['today'], ', '.join(
            '%s %.2fx' % (c, t[c] / t['today']) for c in ('F1', 'F2', 'F2o', 'F3') if c in t)))


if __name__ == '__main__':
    main()
