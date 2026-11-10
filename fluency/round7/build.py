#!/usr/bin/env python3
"""Builds round 7: seeds in every candidate, units, gold answers and subject prompts. Run: python3 build.py

Asserts that A and B of a seed parse to the same semantic model (GetPut on the text), that gold answers land,
and that the two candidate guides of a decision are the same length.
"""
import json
import os
from difflib import SequenceMatcher

import content as C
import score as S
from guides import GUIDES

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = {k: os.path.join(HERE, *p) for k, p in {
    'units': ('data', 'units'), 'gold': ('data', 'gold'), 'seeds': ('seeds',), 'prompts': ('prompts',),
    'guides': ('guides',)}.items()}

HEADER = """You work with office files stored as plain text. Below are the syntax documentation, the file itself, and {n} tasks. Do each task on its own: every edit task starts from the file exactly as shown, not from the result of another task.

## Syntax documentation

{guide}
## The file

The file is between the two lines `=== FILE START ===` and `=== FILE END ===` (they are not part of it).

=== FILE START ===
{file}=== FILE END ===

## Tasks

{tasks}

## Rules

- Read tasks: answer in the form the task asks for, starting with `ANSWER:`.
- Write tasks: answer with `text`: the complete new file, including its front matter.
- Edit tasks: answer with exact text edits. Each edit is `{{"old": "...", "new": "..."}}`: `old` is copied exactly from the file (same spaces and characters) and occurs exactly once in it, and `new` replaces it. Edits apply in order. Change only what the task asks for; every other line must stay exactly as it is.
- If a task cannot be done in this file format, do not edit: answer `REFUSE: <one sentence why>` and `"edits": []`. Refuse only when the task cannot be done as asked.

## Answer format

Reply with one JSON object and nothing else:

```
{{"answers": [
  {{"task_id": "…", "text": "ANSWER: …"}},
  {{"task_id": "…", "text": "<the complete new file>"}},
  {{"task_id": "…", "edits": [{{"old": "…", "new": "…"}}]}},
  {{"task_id": "…", "text": "REFUSE: …", "edits": []}}
]}}
```

One entry per task, in the order of the tasks.
"""


def make_edits(a_text, b_text):
    """Round 3's gold-edit derivation: a line diff, each `old` widened until it occurs exactly once."""
    a = a_text.splitlines(keepends=True)
    b = b_text.splitlines(keepends=True)
    ops = []
    for op in SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        if op[0] == 'equal':
            continue
        if ops and op[1] - ops[-1][2] <= 2:
            ops[-1] = ('replace', ops[-1][1], op[2], ops[-1][3], op[4])
        else:
            ops.append(op)
    cur = a[:]
    off = 0
    edits = []
    for idx, (tag, i1, i2, j1, j2) in enumerate(ops):
        s, e = i1 + off, i2 + off
        limit = (ops[idx + 1][1] + off) if idx + 1 < len(ops) else len(cur)
        ss, ee = s, e
        back = True
        while True:
            old = ''.join(cur[ss:ee])
            if old.strip() and S.count_occ(''.join(cur), old) == 1:
                break
            if back and ss > 0:
                ss -= 1
            elif ee < limit:
                ee += 1
            elif ss > 0:
                ss -= 1
            else:
                raise RuntimeError('cannot make a unique edit')
            back = not back
        new_lines = cur[ss:s] + b[j1:j2] + cur[e:ee]
        edits.append({'old': ''.join(cur[ss:ee]), 'new': ''.join(new_lines)})
        cur[ss:ee] = new_lines
        off += (j2 - j1) - (i2 - i1)
    assert ''.join(cur) == b_text
    return edits


def model_of(spec, decision):
    if decision == 'breaks':
        return {'fm': spec['fm'], 'blocks': [
            dict(b, text=S.inlines_of(b['text'])) if 'text' in b else dict(b) for b in spec['doc']]}
    return C.cp(spec['doc'])


def write_model(spec, decision):
    if decision == 'breaks':
        return {'fm': spec['fm'], 'blocks': [
            dict(b, text=S.inlines_of(b['text'])) if 'text' in b else dict(b) for b in spec['write_doc']]}
    return C.cp(spec['write_doc'])


def task_list(spec, cand):
    out = []
    rid, rtext = spec['read']
    expect = spec.get('read_expect') or spec['read_expect_' + cand.lower()]
    out.append({'task_id': rid, 'kind': 'read', 'text': rtext, 'expect': expect})
    prefix = spec['edits'][0][0].rsplit('-', 1)[0]  # report1 / kpi1 / budget1
    out.append({'task_id': prefix + '-w', 'kind': 'write', 'text': spec['write_instruction']})
    for tid, instruction, _fn in spec['edits']:
        out.append({'task_id': tid, 'kind': 'edit', 'text': instruction})
    rid2, rtext2, _reason = spec['refuse']
    out.append({'task_id': rid2, 'kind': 'refuse', 'text': rtext2})
    return out


def fmt_tasks(tasks):
    labels = {'read': 'read', 'write': 'write', 'edit': 'edit', 'refuse': 'edit'}
    return '\n'.join('%d. `%s` (%s): %s' % (k + 1, t['task_id'], labels[t['kind']], t['text'])
                     for k, t in enumerate(tasks))


def apply_gold_fn(spec, decision, fn):
    if decision == 'breaks':
        d = C.cp(spec['doc'])
        fn(d)
        return {'fm': spec['fm'], 'blocks': [
            dict(b, text=S.inlines_of(b['text'])) if 'text' in b else dict(b) for b in d]}
    b = C.cp(spec['doc'])
    fn(b)
    return b


def main():
    for d in OUT.values():
        os.makedirs(d, exist_ok=True)
    for (dec, cand), g in GUIDES.items():
        open(os.path.join(OUT['guides'], '%s-%s.md' % (dec, cand)), 'w', encoding='utf-8').write(g)
        assert 'candidate B' not in g.lower() and 'candidate A' not in g.lower()
        if dec == 'breaks':
            # A may mention <br/> (line break) but not type=; B may not mention columnbreak
            if cand == 'B':
                assert '<columnbreak' not in g
            if cand == 'A':
                assert 'type="page"' not in g and 'type="column"' not in g
    index = []
    for dec in C.DECISIONS:
        la, lb = len(GUIDES[dec, 'A']), len(GUIDES[dec, 'B'])
        assert la == lb, '%s guides differ: A=%d B=%d' % (dec, la, lb)
        print('guide %-8s A=B=%d chars' % (dec, la))
        for rep in (1, 2, 3):
            spec = C.spec(dec, rep)
            seed_m = model_of(spec, dec)
            texts = {}
            parsed = {}
            for cand in 'AB':
                unit_stub = {'decision': dec, 'candidate': cand}
                t = S.render(seed_m, cand, dec)
                m, ctx = S.parse(t, unit_stub)
                assert not ctx.errors, (dec, cand, rep, ctx.error_text())
                t2 = S.render(m, cand, dec)
                assert t == t2, (dec, cand, rep, 'GetPut on the text failed')
                texts[cand] = t
                parsed[cand] = S.semantic(m, dec)
            assert parsed['A'] == parsed['B'], (dec, rep, S.first_diff(parsed['A'], parsed['B']))
            write_m = write_model(spec, dec)
            for cand in 'AB':
                uid = 'r7-%s-%s-%d' % (dec, cand, rep)
                unit_stub = {'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep}
                tasks = task_list(spec, cand)
                gold = []
                # read
                gold.append({'task_id': tasks[0]['task_id'], 'text': tasks[0]['expect']})
                # write
                wtext = S.render(write_m, cand, dec)
                wm, wctx = S.parse(wtext, unit_stub)
                assert not wctx.errors, (uid, 'write', wctx.error_text())
                gold.append({'task_id': tasks[1]['task_id'], 'text': wtext})
                # edits
                for tdef, ed in zip(tasks[2:-1], spec['edits']):
                    target = apply_gold_fn(spec, dec, ed[2])
                    ttext = S.render(target, cand, dec)
                    edits = make_edits(texts[cand], ttext)
                    applied, flag, msg = S.apply_edits(texts[cand], edits)
                    assert flag is None and applied == ttext, (uid, tdef['task_id'], msg)
                    gold.append({'task_id': tdef['task_id'], 'edits': edits})
                # refuse
                gold.append({'task_id': tasks[-1]['task_id'],
                             'text': 'REFUSE: %s.' % spec['refuse'][2], 'edits': []})
                unit = dict(unit_stub, names=spec.get('names') or {}, fm=spec['fm'],
                            seed=texts[cand], tasks=tasks, unreachable=[])
                json.dump(unit, open(os.path.join(OUT['units'], uid + '.json'), 'w', encoding='utf-8'),
                          ensure_ascii=False, indent=1)
                json.dump({'answers': gold}, open(os.path.join(OUT['gold'], uid + '.json'), 'w', encoding='utf-8'),
                          ensure_ascii=False, indent=1)
                open(os.path.join(OUT['seeds'], uid + '.txt'), 'w', encoding='utf-8').write(texts[cand])
                prompt = HEADER.format(n=len(tasks), guide=GUIDES[dec, cand], file=texts[cand],
                                       tasks=fmt_tasks(tasks))
                open(os.path.join(OUT['prompts'], uid + '.md'), 'w', encoding='utf-8').write(prompt)
                res = S.score_answer(uid, {'answers': gold})
                for r in res:
                    assert r['valid'] and r['landed'] and not r['flags'], (
                        'gold does not land: %s %s: %s %s %s' % (
                            uid, r['task_id'], r['flags'], r['error'], r['detail']))
                index.append({'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep,
                              'prompt_file': 'prompts/%s.md' % uid, 'prompt_chars': len(prompt),
                              'seed_chars': len(texts[cand]), 'n_tasks': len(tasks),
                              'tasks': [t['task_id'] for t in tasks]})
                print('%-22s seed %5d  prompt %6d' % (uid, len(texts[cand]), len(prompt)))
    json.dump(index, open(os.path.join(HERE, 'data', 'index.json'), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)
    print('built %d units' % len(index))


if __name__ == '__main__':
    main()
