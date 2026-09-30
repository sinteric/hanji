#!/usr/bin/env python3
"""Tables for RESULTS.md from runs/: python3 summarize.py. Re-scores every answer file (first try and fix round)."""
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import pptx_kit as PK  # noqa: E402
import score as S  # noqa: E402

MODELS = ('opus', 'sonnet')
OK_REFUSALS = ('unreachable_refused', 'no_style_refused')


def load(unit, model, kind):
    f = os.path.join(HERE, 'runs', unit, '%s-%s.json' % (model, kind))
    return json.load(open(f, encoding='utf-8')) if os.path.exists(f) else None


def usage(unit, model):
    tot = {'in': 0, 'out': 0}
    for part in ('a', 'b'):
        f = os.path.join(HERE, 'runs', unit, '%s-%s.usage.json' % (model, part))
        if os.path.exists(f):
            u = json.load(open(f))
            tot['in'] += u.get('input_tokens', 0)
            tot['out'] += u.get('output_tokens', 0)
    return tot


def results(unit, model, scorer):
    first = load(unit, model, 'first')
    if first is None:
        return None, None
    r1 = scorer(unit, first)
    fix = load(unit, model, 'fix')
    r2 = None
    if fix is not None:
        merged = {a['task_id']: a for a in first['answers'] if isinstance(a, dict)}
        merged.update({a['task_id']: a for a in fix['answers'] if isinstance(a, dict)})
        r2 = scorer(unit, {'answers': list(merged.values())})
    return r1, r2


def part_a():
    cands = ('F1', 'F2', 'F3')
    rows = []
    out = defaultdict(dict)
    per_task = defaultdict(lambda: defaultdict(list))
    chars = defaultdict(lambda: defaultdict(int))
    common = defaultdict(lambda: defaultdict(int))
    methods = defaultdict(list)
    tok = defaultdict(lambda: {'in': 0, 'out': 0})
    for model in MODELS:
        for c in cands:
            agg = {'reach': 0, 'landed1': 0, 'landed2': 0, 'valid1': 0, 'invalid1': 0, 'unreach': 0,
                   'refused': 0, 'attempted': 0, 'wrong_ref': 0, 'silent': []}
            for rep in (1, 2, 3):
                unit = 'r6-%s-%d' % (c, rep)
                r1, r2 = results(unit, model, S.score_answer)
                if r1 is None:
                    continue
                u = usage(unit, model)
                tok[(model, c)]['in'] += u['in']
                tok[(model, c)]['out'] += u['out']
                meta = json.load(open(os.path.join(HERE, 'data', 'units', unit + '.json'), encoding='utf-8'))
                un = set(meta['unreachable'])
                for k, x in enumerate(r1):
                    tid = x['task_id']
                    chars[(model, c)][rep] += x['chars']
                    kind = tid.split('-', 1)[1]
                    per_task[kind][(model, c)].append(x)
                    if x.get('method'):
                        methods[(model, c)].append((tid, x['method']))
                    if 'no_style_refused' in x['flags']:
                        methods[(model, c)].append((tid, 'refused'))
                    if tid in un or 'no_style_refused' in x['flags']:
                        agg['unreach'] += 1
                        if set(x['flags']) & set(OK_REFUSALS):
                            agg['refused'] += 1
                        else:
                            agg['attempted'] += 1
                            agg['silent'].append(tid)
                        continue
                    agg['reach'] += 1
                    agg['landed1'] += x['landed']
                    agg['valid1'] += x['valid']
                    agg['invalid1'] += (not x['valid'])
                    if 'wrong_refusal' in x['flags']:
                        agg['wrong_ref'] += 1
                    x2 = r2[k] if r2 else x
                    agg['landed2'] += x2['landed']
            out[model][c] = agg
    return out, per_task, chars, methods, tok


def part_b():
    cands = ('F1', 'F2o')
    out = defaultdict(dict)
    chars = defaultdict(lambda: defaultdict(int))
    tasks_rows = defaultdict(lambda: defaultdict(list))
    tok = defaultdict(lambda: {'in': 0, 'out': 0})
    for model in MODELS:
        for c in cands:
            agg = {'reach': 0, 'landed1': 0, 'landed2': 0, 'valid1': 0, 'invalid1': 0, 'unreach': 0, 'refused': 0,
                   'attempted': 0, 'wrong_ref': 0, 'silent': []}
            for rep in (1, 2, 3):
                unit = 'r6p-%s-%d' % (c, rep)
                r1, r2 = results(unit, model, PK.score_answer)
                if r1 is None:
                    continue
                u = usage(unit, model)
                tok[(model, c)]['in'] += u['in']
                tok[(model, c)]['out'] += u['out']
                meta = json.load(open(os.path.join(HERE, 'data', 'units', unit + '.json'), encoding='utf-8'))
                un = set(meta['unreachable'])
                for k, x in enumerate(r1):
                    chars[(model, c)][rep] += x['chars']
                    tasks_rows[x['task_id']][(model, c)].append(x)
                    if x['task_id'] in un:
                        agg['unreach'] += 1
                        if 'unreachable_refused' in x['flags']:
                            agg['refused'] += 1
                        else:
                            agg['attempted'] += 1
                            agg['silent'].append('%d:%s %s' % (rep, x['task_id'], x['detail'][:60]))
                        continue
                    agg['reach'] += 1
                    agg['landed1'] += x['landed']
                    agg['valid1'] += x['valid']
                    agg['invalid1'] += (not x['valid'])
                    agg['wrong_ref'] += 'wrong_refusal' in x['flags']
                    x2 = r2[k] if r2 else x
                    agg['landed2'] += x2['landed']
            out[model][c] = agg
    return out, tasks_rows, chars, tok


def part_c():
    """F2 against F2s (section defaults) and new styles, on the two hwpx seeds."""
    import content as CT
    print('\n## Part C (section defaults, new styles; hwpx seeds 1 and 2)\n')
    print('| model | cand | landed, first try | after one fix round | invalid first try | reads | new style (e10) | '
          'no-op (n1) | answer chars | tokens in |')
    print('|---|---|---|---|---|---|---|---|---|---|')
    for model in MODELS:
        for c in CT.CANDS_S:
            n = l1 = l2 = inv = rd = rdn = ns = nsn = nop = nopn = ch = 0
            tin = 0
            for rep in CT.SEEDS_S:
                u = 'r6s-%s-%d' % (c, rep)
                r1, r2 = results(u, model, S.score_answer)
                if r1 is None:
                    continue
                r2 = r2 or r1
                tin += usage(u, model)['in']
                for x, y, t in zip(r1, r2, CT.TASKS_S[rep]):
                    n += 1
                    l1 += x['landed']
                    l2 += y['landed']
                    inv += not x['valid']
                    ch += x['chars']
                    if t.kind == 'read':
                        rdn += 1
                        rd += x['landed']
                    if t.id.endswith('-e10'):
                        nsn += 1
                        ns += y['landed']
                    if t.kind == 'noop':
                        nopn += 1
                        nop += x['landed']
            print('| %s | %s | %d / %d | %d / %d | %d | %d / %d | %d / %d | %d / %d | %s | %s |' % (
                model.capitalize(), c, l1, n, l2, n, inv, rd, rdn, ns, nsn, nop, nopn, format(ch, ','),
                format(tin, ',')))
    print('\n| task | ' + ' | '.join('%s %s' % (m.capitalize(), c) for m in MODELS for c in CT.CANDS_S) + ' |')
    print('|---|' + '---|' * 4)
    for rep in CT.SEEDS_S:
        for k, t in enumerate(CT.TASKS_S[rep]):
            cells = []
            for model in MODELS:
                for c in CT.CANDS_S:
                    r1, r2 = results('r6s-%s-%d' % (c, rep), model, S.score_answer)
                    x, y = r1[k], (r2 or r1)[k]
                    cells.append('L' if x['landed'] else ('x→L' if y['landed'] else 'x'))
            print('| %s | %s |' % (t.id, ' | '.join(cells)))


def part_d():
    import xlsx_kit as XK
    print('\n## Part D (workbooks: F1 range lines and the format operation)\n')
    print('| model | landed, first try | after one fix round | invalid first try | reads | edits | refusals | '
          'ops | answer chars |')
    print('|---|---|---|---|---|---|---|---|---|')
    for model in MODELS:
        n = l1 = l2 = inv = ops = ch = 0
        kinds = defaultdict(lambda: [0, 0])
        for rep in XK.SEEDS:
            u = 'r6x-F1-%d' % rep
            r1, r2 = results(u, model, XK.score_answer)
            if r1 is None:
                continue
            r2 = r2 or r1
            for x, y, t in zip(r1, r2, XK.TASKS[rep]):
                n += 1
                l1 += x['landed']
                l2 += y['landed']
                inv += not x['valid']
                ops += x['ops']
                ch += x['chars']
                kinds[t.kind][0] += x['landed']
                kinds[t.kind][1] += 1
        print('| %s | %d / %d | %d / %d | %d | %d / %d | %d / %d | %d / %d | %d | %s |' % (
            model.capitalize(), l1, n, l2, n, inv, kinds['read'][0], kinds['read'][1], kinds['edit'][0],
            kinds['edit'][1], kinds['refuse'][0], kinds['refuse'][1], ops, format(ch, ',')))
    for model in MODELS:
        for rep in XK.SEEDS:
            r1, r2 = results('r6x-F1-%d' % rep, model, XK.score_answer)
            for x in r1 or []:
                if not x['landed']:
                    print('- %s %s %s: %s %s' % (model, rep, x['task_id'], x['flags'], (x['error'] or x['detail'])[:160]))


def main():
    a, per_task, chars, methods, tok = part_a()
    print('## Part A (flow documents)\n')
    print('| model | cand | reachable landed, first try | after one fix round | invalid first try | '
          'unreachable: refused / attempted | wrong refusals |')
    print('|---|---|---|---|---|---|---|')
    for model in MODELS:
        for c, g in a[model].items():
            print('| %s | %s | %d / %d | %d / %d | %d | %d / %d | %d |' % (
                model, c, g['landed1'], g['reach'], g['landed2'], g['reach'], g['invalid1'], g['refused'],
                g['attempted'], g['wrong_ref']))
    print()
    for model in MODELS:
        for c, g in a[model].items():
            if g['silent']:
                print('- %s %s attempted where it could not know: %s' % (model, c, ', '.join(g['silent'])))
    print('\nPer task (landed first try, both models, 3 seeds; U = unreachable, r = correct refusal):\n')
    kinds = sorted(per_task, key=lambda k: (k[0] != 'q', k))
    print('| task | ' + ' | '.join('%s %s' % (m, c) for m in MODELS for c in ('F1', 'F2', 'F3')) + ' |')
    print('|---|' + '---|' * 6)
    for k in kinds:
        cells = []
        for m in MODELS:
            for c in ('F1', 'F2', 'F3'):
                xs = per_task[k][(m, c)]
                s = ''.join('L' if x['landed'] else ('r' if set(x['flags']) & set(OK_REFUSALS) else
                                                     ('U' if 'unreachable_attempted' in x['flags'] else
                                                      'x')) for x in xs)
                cells.append(s)
        print('| %s | %s |' % (k, ' | '.join(cells)))
    print('\nAnswer chars (sum over tasks, per seed):\n')
    print('| model | cand | seed 1 | seed 2 | seed 3 | all | tokens in | tokens out |')
    print('|---|---|---|---|---|---|---|---|')
    for m in MODELS:
        for c in ('F1', 'F2', 'F3'):
            d = chars[(m, c)]
            print('| %s | %s | %d | %d | %d | %d | %d | %d |' % (m, c, d[1], d[2], d[3], sum(d.values()),
                                                               tok[(m, c)]['in'], tok[(m, c)]['out']))
    print('\nStyle-edit tasks (e5), how they were done:\n')
    for key, v in sorted(methods.items()):
        print('- %s %s: %s' % (key[0], key[1], ', '.join('%s %s' % x for x in v)))
    b, rows_b, chars_b, tok_b = part_b()
    print('\n## Part B (presentations)\n')
    print('| model | cand | reachable landed, first try | after fix | invalid | unreachable: refused / attempted |')
    print('|---|---|---|---|---|---|')
    for m in MODELS:
        for c, g in b[m].items():
            print('| %s | %s | %d / %d | %d / %d | %d | %d / %d |' % (m, c, g['landed1'], g['reach'], g['landed2'],
                                                                   g['reach'], g['invalid1'], g['refused'],
                                                                   g['attempted']))
    for m in MODELS:
        for c, g in b[m].items():
            if g['silent']:
                print('- %s %s attempted: %s' % (m, c, '; '.join(g['silent'])))
    print('\n| task | ' + ' | '.join('%s %s' % (m, c) for m in MODELS for c in ('F1', 'F2o')) + ' |')
    print('|---|' + '---|' * 4)
    for tid in sorted(rows_b):
        cells = []
        for m in MODELS:
            for c in ('F1', 'F2o'):
                xs = rows_b[tid][(m, c)]
                cells.append(''.join('L' if x['landed'] else ('r' if 'unreachable_refused' in x['flags'] else
                                                              ('U' if 'unreachable_attempted' in x['flags'] else 'x'))
                                     for x in xs))
        print('| %s | %s |' % (tid, ' | '.join(cells)))
    print('\n| model | cand | chars seed 1 | 2 | 3 | all | tokens in | tokens out |')
    print('|---|---|---|---|---|---|---|---|')
    for m in MODELS:
        for c in ('F1', 'F2o'):
            d = chars_b[(m, c)]
            print('| %s | %s | %d | %d | %d | %d | %d | %d |' % (m, c, d[1], d[2], d[3], sum(d.values()),
                                                               tok_b[(m, c)]['in'], tok_b[(m, c)]['out']))
    part_c()
    part_d()


if __name__ == '__main__':
    main()
