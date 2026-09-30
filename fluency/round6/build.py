#!/usr/bin/env python3
"""Builds round 6 part A: seeds in every candidate, units, gold answers and subject prompts. Run: python3 build.py

Asserts GetPut on the text for every seed and candidate (the candidate's rendering reads back to the same effective
formatting as F1's, block by block), that every gold answer lands, and records the tasks a candidate cannot do by
construction (`unreachable`: F3 cannot show or write direct formatting)."""
import copy
import json
import os
from difflib import SequenceMatcher

import content as CT
import doc as D
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
- Edit tasks: answer with exact text edits. Each edit is `{{"old": "...", "new": "..."}}`: `old` is copied exactly from the file (same spaces and characters) and occurs exactly once in it, and `new` replaces it. Edits apply in order. Change only what the task asks for; every other line must stay exactly as it is.
- If a task cannot be done in this file format, do not edit: answer `REFUSE: <one sentence why>`.

## Answer format

Reply with one JSON object and nothing else:

```
{{"answers": [
  {{"task_id": "…", "text": "ANSWER: …"}},
  {{"task_id": "…", "edits": [{{"old": "…", "new": "…"}}]}},
  {{"task_id": "…", "text": "REFUSE: …"}}
]}}
```

One entry per task, in the order of the tasks.
"""


# Each unit is sent as two prompts with the same file: reads, single edits and the refusal (a); the edits that
# touch many places, the style edit and the new table (b). One prompt with all thirteen could outgrow an answer.
PARTS = {'a': ('q1', 'q2', 'q3', 'q4', 'e1', 'e2', 'e6', 'e9'), 'b': ('e3', 'e4', 'e5', 'e7', 'e8')}


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
            if old.strip() and S.r2.count_occ(''.join(cur), old) == 1:
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


def fmt_expect(e):
    if e['type'] == 'props':
        from vocab import fmt_attrs
        return 'ANSWER: ' + ' '.join('%s=%s' % (k, v if ' ' not in v else '"%s"' % v) for k, v in e['value'].items())
    return 'ANSWER: ' + ('; '.join(e['value']) if e['value'] else 'none')


def main():
    for d in OUT.values():
        os.makedirs(d, exist_ok=True)
    for c, g in GUIDES.items():
        open(os.path.join(OUT['guides'], 'doc-%s.md' % c), 'w', encoding='utf-8').write(g)
    index = []
    for rep in sorted(CT.SEEDS):
        m = CT.seed_model(rep)
        texts = {c: D.render(m, c) for c in D.CANDS}
        truth = [e for e in D.effective(texts['F1'], 'F1', m)[0] if e[0] != 'R']
        for c in D.CANDS:
            e = [x for x in D.effective(texts[c], c, m)[0] if x[0] != 'R']
            assert e == truth, 'GetPut on the text fails: seed %d under %s' % (rep, c)
            again = D.render(m, c)
            assert again == texts[c]
        for c in D.CANDS:
            uid = 'r6-%s-%d' % (c, rep)
            open(os.path.join(OUT['seeds'], uid + '.txt'), 'w', encoding='utf-8').write(texts[c])
            gold = []
            unreachable = []
            for t in CT.TASKS[rep]:
                if t.kind == 'read':
                    if c in getattr(t, 'refuse_in', ()):
                        unreachable.append(t.id)
                        gold.append({'task_id': t.id, 'text': 'REFUSE: that formatting is not shown in this file.'})
                    else:
                        gold.append({'task_id': t.id, 'text': fmt_expect(t.expect(m))})
                elif t.kind == 'refuse':
                    gold.append({'task_id': t.id, 'text': 'REFUSE: this format cannot express it.'})
                else:
                    m2 = CT.seed_model(rep)
                    r = t.gold(m2, c)
                    if r is None and c == 'F3' and not getattr(t, 'retext', None):
                        unreachable.append(t.id)
                        gold.append({'task_id': t.id, 'text': 'REFUSE: direct formatting cannot be written here.'})
                        continue
                    new = D.render(m2, c)
                    gold.append({'task_id': t.id, 'edits': make_edits(texts[c], new)})
            unit = {'unit_id': uid, 'candidate': c, 'seed': rep, 'name': CT.SEEDS[rep]['name'],
                    'format': CT.SEEDS[rep]['fmt'], 'file': texts[c], 'unreachable': unreachable,
                    'tasks': [{'task_id': t.id, 'kind': t.kind, 'text': t.text} for t in CT.TASKS[rep]]}
            json.dump(unit, open(os.path.join(OUT['units'], uid + '.json'), 'w', encoding='utf-8'),
                      ensure_ascii=False, indent=1)
            json.dump({'answers': gold}, open(os.path.join(OUT['gold'], uid + '.json'), 'w', encoding='utf-8'),
                      ensure_ascii=False, indent=1)
            res = S.score_answer(uid, {'answers': gold})
            for r in res:
                ok = r['landed'] or (r['task_id'] in unreachable and 'unreachable_refused' in r['flags'])
                assert ok, 'gold does not land: %s %s: %s %s' % (uid, r['task_id'], r['error'], r['detail'])
            for part, ids in PARTS.items():
                ts = [t for t in CT.TASKS[rep] if t.id.split('-', 1)[1] in ids]
                tasks = '\n'.join('%d. `%s` (%s): %s' % (k + 1, t.id, t.kind if t.kind != 'refuse' else 'edit',
                                                        t.text) for k, t in enumerate(ts))
                prompt = HEADER.format(n=len(ts), guide=GUIDES[c], file=texts[c], tasks=tasks)
                open(os.path.join(OUT['prompts'], '%s-%s.md' % (uid, part)), 'w', encoding='utf-8').write(prompt)
                index.append({'unit_id': uid, 'part': part, 'candidate': c, 'seed': rep, 'file_chars': len(texts[c]),
                              'prompt_chars': len(prompt), 'max_line': max(len(x) for x in prompt.split('\n')),
                              'tasks': [t.id for t in ts], 'unreachable': [x for x in unreachable
                                                                           if x in [t.id for t in ts]]})
                print(uid, part, 'file', len(texts[c]), 'prompt', len(prompt), 'max line',
                      max(len(x) for x in prompt.split('\n')), 'unreachable', len(index[-1]['unreachable']))
    json.dump(index, open(os.path.join(HERE, 'data', 'index.json'), 'w', encoding='utf-8'), indent=1)


if __name__ == '__main__':
    main()
