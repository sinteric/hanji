#!/usr/bin/env python3
"""Round 5 scorer. Usage: python3 score.py <unit_id> <answers.json>

Prints {"results": [{task_id, valid, landed, error, chars, flags, detail}]}. Edits are applied with round 2's
apply_edits; the result is parsed by deck.py against the seed (so unchanged rounded numbers keep their exact
EMU) and checked by the task's check in content.py: geometry within content.TOL (2 pt), and every object the task
does not name exactly as in the seed."""
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import content as C  # noqa: E402
import deck as D  # noqa: E402

_spec = importlib.util.spec_from_file_location('hanji_r2score', os.path.join(HERE, '..', 'round2', 'score.py'))
r2 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(r2)

REFUSE = 'REFUSE:'


def ctx_for(rep, with_seed=True):
    d = C.SEEDS[rep]
    return D.Ctx(d['W'], d['H'], d['layouts'], seed=C.deck_copy(d) if with_seed else None)


def task_spec(rep, tid):
    short = tid.split('-', 1)[1]
    for t in C.TASKS[rep] + C.TASKS_X[rep]:
        if t[0] == short:
            return t
    raise KeyError(tid)


def is_refusal(a):
    t = a.get('text')
    return isinstance(t, str) and t.strip().upper().startswith(REFUSE)


def read_answer(text, rep, sidx):
    """-> (set of keys or None, message)."""
    t = text.strip()
    if not t.upper().startswith('ANSWER:'):
        return None, 'a read answer is `ANSWER: <objects, comma-separated>`'
    body = t[len('ANSWER:'):].strip().strip('.')
    if body.lower() in ('none', 'nothing', '-', ''):
        return set(), ''
    slide = C.SEEDS[rep]['slides'][sidx]
    lab = {}
    for o in D.flat(slide['objs']):
        for l in C.labels(o):
            lab[l.lower()] = D.okey(o)
    out = set()
    for item in re.split(r'[,;\n]| and ', body):
        it = item.strip().strip('`"\'').strip()
        if not it:
            continue
        cands = [it] + [p.strip().strip('`"\'') for p in re.findall(r'\(([^)]*)\)', it)] + \
                [re.sub(r'\s*\([^)]*\)', '', it).strip().strip('`"\'')]
        cands += [re.sub(r'^(the\s+)?(slot|shape|line|picture|keep|table|text box)\s+', '', c, flags=re.I)
                  for c in list(cands)]
        key = None
        for c in cands:
            c = c.strip().lower()
            if c.startswith('id='):
                c = c[3:].strip('"')
            if c in lab:
                key = lab[c]
                break
        if key is None:
            return None, 'unknown object %r' % it
        out.add(key)
    return out, ''


def score_task(unit, t, a):
    rep, cand, tid = unit['replicate'], unit['candidate'], t['task_id']
    spec = task_spec(rep, tid)
    _, kind, _, payload = spec
    res = {'task_id': tid, 'valid': False, 'landed': False, 'error': '', 'chars': 0, 'flags': [], 'detail': ''}
    if a is None:
        res['flags'].append('missing_answer')
        return res
    if kind == 'read':
        text = a.get('text') if isinstance(a.get('text'), str) else ''
        res['chars'] = len(text)
        sidx, want = payload
        got, msg = read_answer(text, rep, sidx)
        if got is None:
            res['error'] = msg
            res['flags'].append('answer_form' if 'ANSWER' in msg else 'unknown_object')
            return res
        res['valid'] = True
        if got == set(want):
            res['landed'] = True
        else:
            res['flags'].append('wrong_answer')
            res['detail'] = 'answered %s, expected %s' % (sorted(got), sorted(want))
        return res
    gold_fn, check = payload
    refusal = gold_fn is None
    if is_refusal(a):
        res['chars'] = len(a['text'])
        if a.get('edits'):
            res['flags'].append('bad_refusal')
            res['error'] = 'a refusal has "edits": [].'
            return res
        res['valid'] = True
        if refusal:
            res['landed'] = True
        else:
            res['flags'].append('wrong_refusal')
        return res
    seed_text = D.render(C.deck_copy(C.SEEDS[rep]), cand)
    if kind == 'write':
        text = a.get('text')
        if not isinstance(text, str):
            res['flags'].append('wrong_answer_kind')
            return res
        res['chars'] = len(text)
        deck, errs, _ = D.parse(text, cand, ctx_for(rep, with_seed=False))
    else:
        edits = a.get('edits')
        if not isinstance(edits, list) or not edits:
            res['flags'].append('wrong_answer_kind' if a.get('text') else 'no_change')
            return res
        res['chars'] = sum(len(e.get('old', '')) + len(e.get('new', '')) for e in edits if isinstance(e, dict))
        new_text, flag, msg = r2.apply_edits(seed_text, edits)
        if flag:
            res['flags'].append(flag)
            res['error'] = msg
            return res
        deck, errs, _ = D.parse(new_text, cand, ctx_for(rep))
    if errs:
        res['flags'].append(errs[0][0])
        res['error'] = errs[0][1]
        return res
    res['valid'] = True
    if refusal:
        res['flags'].append('should_refuse')
        return res
    detail = check(C.deck_copy(C.SEEDS[rep]), deck)
    if detail is None:
        res['landed'] = True
    else:
        res['flags'].append('not_landed')
        res['detail'] = detail
    return res


def load_unit(uid):
    with open(os.path.join(HERE, 'data', 'units', uid + '.json'), encoding='utf-8') as f:
        return json.load(f)


def score(uid, answers):
    unit = load_unit(uid)
    by = {}
    for a in answers.get('answers', []):
        if isinstance(a, dict) and 'task_id' in a:
            by.setdefault(a['task_id'], a)
    return [score_task(unit, t, by.get(t['task_id'])) for t in unit['tasks']]


def main(argv):
    if len(argv) != 3:
        print(__doc__)
        return 2
    with open(argv[2], encoding='utf-8') as f:
        answers = json.load(f)
    print(json.dumps({'results': score(argv[1], answers)}, ensure_ascii=False, indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
