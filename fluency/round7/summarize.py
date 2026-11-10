#!/usr/bin/env python3
"""Tables for RESULTS.md from runs/: python3 summarize.py. Re-scores every answer file (first try and fix round).

Reports per model label (sonnet55, haiku55, sonnet46) and per decision/candidate. With no runs yet, prints a
placeholder so the kit still runs.
"""
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import score as S  # noqa: E402
from run import MODELS  # noqa: E402

DECISIONS = ('breaks', 'shapes', 'databar')
CANDS = ('A', 'B')


def load(unit, label, kind):
    f = os.path.join(HERE, 'runs', unit, '%s-%s.json' % (label, kind))
    return json.load(open(f, encoding='utf-8')) if os.path.exists(f) else None


def usage(unit, label):
    tot = {'in': 0, 'out': 0}
    for kind in ('first', 'fix'):
        f = os.path.join(HERE, 'runs', unit, '%s-%s.usage.json' % (label, kind))
        if os.path.exists(f):
            u = json.load(open(f))
            tot['in'] += u.get('input_tokens', 0)
            tot['out'] += u.get('output_tokens', 0)
    return tot


def results(unit, label):
    first = load(unit, label, 'first')
    if first is None:
        return None, None
    r1 = S.score_answer(unit, first)
    fix = load(unit, label, 'fix')
    r2 = None
    if fix is not None:
        merged = {a['task_id']: a for a in first.get('answers', []) if isinstance(a, dict)}
        merged.update({a['task_id']: a for a in fix.get('answers', []) if isinstance(a, dict)})
        r2 = S.score_answer(unit, {'answers': list(merged.values())})
    return r1, r2


def main():
    idx = json.load(open(os.path.join(HERE, 'data', 'index.json'), encoding='utf-8'))
    labels = list(MODELS)
    any_run = False
    print('# Fluency test results, round 7\n')
    print('## Round 7 — (to be filled after the runs)\n')
    print('**Subjects:** Sonnet 5.5 (`%s`, files `sonnet55-*.json`), Haiku 5.5 (`%s`, files `haiku55-*.json`), '
          'Sonnet 4.6 (`%s`, files `sonnet46-*.json`). Blind, each unit in a fresh `claude -p` conversation whose '
          'only tool was Read (the prompt read from its own file). Re-score with `python3 score.py <unit> <file>`; '
          '`python3 summarize.py` reproduces every table here.\n' % (
              MODELS['sonnet55'], MODELS['haiku55'], MODELS['sonnet46']))
    print('| model | decision | cand | landed, first try | after one fix round | invalid, first try | '
          'wrong refusals | answer chars | tokens in | tokens out |')
    print('|---|---|---|---|---|---|---|---|---|---|')
    per_task = defaultdict(lambda: defaultdict(list))
    for label in labels:
        for dec in DECISIONS:
            for cand in CANDS:
                n = l1 = l2 = inv = wrong = ch = tin = tout = 0
                for u in idx:
                    if u['decision'] != dec or u['candidate'] != cand:
                        continue
                    r1, r2 = results(u['unit_id'], label)
                    if r1 is None:
                        continue
                    any_run = True
                    r2 = r2 or r1
                    us = usage(u['unit_id'], label)
                    tin += us['in']
                    tout += us['out']
                    for x, y in zip(r1, r2):
                        n += 1
                        l1 += x['landed']
                        l2 += y['landed']
                        inv += not x['valid']
                        wrong += 'wrong_refusal' in x['flags']
                        ch += x['chars']
                        per_task[(dec, x['task_id'].rsplit('-', 1)[-1])][(label, cand)].append(x)
                cell = '%d / %d' % (l1, n) if n else '— / —'
                cell2 = '%d / %d' % (l2, n) if n else '— / —'
                print('| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |' % (
                    label, dec, cand, cell, cell2, inv if n else '—', wrong if n else '—',
                    format(ch, ',') if n else '—', format(tin, ',') if n else '—',
                    format(tout, ',') if n else '—'))
    if not any_run:
        print('\nNo `runs/<unit>/<label>-first.json` files yet. After the owner\'s box runs `run.py`, re-run '
              '`python3 summarize.py` and paste the tables into RESULTS.md.\n')
        return
    print('\nPer-task landed (first try, three seeds; L landed, x missed, r wrong refusal):\n')
    kinds = []
    seen = set()
    for dec, short in per_task:
        if (dec, short) not in seen:
            kinds.append((dec, short))
            seen.add((dec, short))
    for dec in DECISIONS:
        rows = [(d, s) for d, s in kinds if d == dec]
        if not rows:
            continue
        print('### %s\n' % dec)
        print('| task | ' + ' | '.join('%s %s' % (m, c) for m in labels for c in CANDS) + ' |')
        print('|---|' + '---|' * (len(labels) * 2))
        for d, short in rows:
            cells = []
            for m in labels:
                for c in CANDS:
                    xs = per_task[(d, short)][(m, c)]
                    cells.append(''.join(
                        'L' if x['landed'] else ('r' if 'wrong_refusal' in x['flags'] else 'x') for x in xs) or '—')
            print('| %s | %s |' % (short, ' | '.join(cells)))
        print()


if __name__ == '__main__':
    main()
