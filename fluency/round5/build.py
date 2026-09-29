#!/usr/bin/env python3
"""Builds round 5: seeds in every candidate syntax, units, gold answers and subject prompts. Run: python3 build.py

Asserts that every candidate reads its own rendering of every seed back to exactly the seed (GetPut on the text:
unchanged rounded numbers keep their EMU), that every gold answer is valid, and records which tasks a candidate
cannot land by construction (`unreachable`: its best gold misses the check, e.g. a grid that cannot move by 3 cm)."""
import json
import os
from difflib import SequenceMatcher

import content as C
import deck as D
import score as S
from guides import GUIDES, PARTS

PART2 = ('A', 'Ap', 'B')
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = {k: os.path.join(HERE, *p) for k, p in {
    'units': ('data', 'units'), 'gold': ('data', 'gold'), 'seeds': ('seeds',), 'prompts': ('prompts',),
    'guides': ('guides',)}.items()}


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


def layout_list(deck, cand):
    u = D.units_for(cand, deck['W'], deck['H'])
    out = ['Layouts and their slots, each with the box the layout gives it (every layout also has `notes`):']
    for name, slots in deck['layouts'].items():
        if slots:
            out.append('- `%s`: %s' % (name, ', '.join('%s (%s)' % (s, D.show_box(u, b)) for s, b in slots)))
        else:
            out.append('- `%s`: no slots' % name)
    return '\n'.join(out)


INTRO = ('You work with office files stored as plain text. Below are the syntax documentation, the names available '
         'in the file, the file itself, and %s tasks. Do each task on its own: every edit task starts from the '
         'file exactly as shown, not from the result of another task.')
NUMBER = {6: 'six', 10: 'ten'}

REFUSAL_RULE = """\
Do only what the syntax documentation and the names above can express. If an edit task asks for something they \
cannot express, do not approximate it, and do not invent a name, tag or attribute: refuse that task by answering it \
with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as \
asked; every other task gets its answer."""

ANSWER_FORMAT = """\
Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a read task>", "text": "ANSWER: <objects, comma-separated>"},
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]},
  {"task_id": "<id of an edit task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```"""


def front_matter_text(deck, cand):
    return '\n'.join(D.front_matter(deck, cand)) + '\n'


def prompt_for(unit, deck, cand, filename):
    fence = '````'
    out = [INTRO % NUMBER[len(unit['tasks'])], '\n## Syntax documentation\n\n' + GUIDES[cand],
           '\n## Names available in this file\n\n' + layout_list(deck, cand),
           '\n## The file: %s\n\n%s\n%s%s' % (filename, fence, unit['seed'], fence),
           '\n## Tasks\n\nA read task is answered with `text`: %s A write task is answered with `text`: the complete '
           'new file. An edit task is answered with `edits`: a list of {"old", "new"} pairs, applied in order to the '
           'file above. Each `old` is copied exactly from the file (as it is after the earlier pairs of that task) and '
           'must occur in it exactly once; it is replaced by `new`.' % C.READ_HOW]
    for t in unit['tasks']:
        if t['type'] == 'write':
            out.append('\n### %s (write)\n\n%s\n\nStart the new file with this front matter:\n\n%s\n%s%s'
                       % (t['task_id'], t['instruction'], fence, front_matter_text(deck, cand), fence))
        else:
            out.append('\n### %s (%s)\n\n%s' % (t['task_id'], t['type'], t['instruction']))
    out.append('\n## Tasks that cannot be done\n\n' + REFUSAL_RULE)
    out.append('\n## Answer format\n\n' + ANSWER_FORMAT)
    return '\n'.join(out) + '\n'


def main():
    for p in OUT.values():
        os.makedirs(p, exist_ok=True)
    for cand, g in GUIDES.items():
        with open(os.path.join(OUT['guides'], 'slides-%s.md' % cand), 'w', encoding='utf-8') as f:
            f.write(g)
    index = []
    for part, rep, cand in [(1, r, c) for r in C.SEEDS for c in D.CANDIDATES] + \
            [(2, r, c) for r in C.SEEDS for c in PART2]:
        seed = C.SEEDS[rep]
        stem = 'deck%d' % rep
        if True:
            deck = C.deck_copy(seed)
            text = D.render(deck, cand)
            # GetPut on the text: the candidate reads its own rendering back to exactly the seed
            back, errs, _ = D.parse(text, cand, S.ctx_for(rep))
            assert not errs, (rep, cand, errs)
            assert D.canon_deck(back) == D.canon_deck(deck), (rep, cand)
            # and it renders again byte for byte
            assert D.render(back, cand) == text, (rep, cand)
            with open(os.path.join(OUT['seeds'], '%s-%s.txt' % (stem, cand)), 'w', encoding='utf-8') as f:
                f.write(text)
            uid = ('r5-%s-%d' if part == 1 else 'r5x-%s-%d') % (cand, rep)
            unit = {'unit_id': uid, 'part': part, 'candidate': cand, 'replicate': rep, 'stem': stem, 'file': seed['file'],
                    'seed': text, 'seed_chars': len(text), 'tasks': []}
            gold, unreachable = [], []
            for tid_s, kind, instr, payload in (C.TASKS if part == 1 else C.TASKS_X)[rep]:
                tid = '%s-%s' % (stem, tid_s)
                unit['tasks'].append({'task_id': tid, 'type': kind, 'instruction': instr})
                if kind == 'read':
                    sidx, keys = payload
                    so = C.SEEDS[rep]['slides'][sidx]
                    names = []
                    for k in keys:
                        o = C.find(so, k)
                        names.append(o['slot'] if o['t'] == 'slot' else o['id'])
                    gold.append({'task_id': tid, 'text': 'ANSWER: ' + ', '.join(names)})
                    continue
                gfn, _ = payload
                if gfn is None:
                    gold.append({'task_id': tid, 'text': 'REFUSE: formatting and the content of kept objects '
                                 'cannot be changed here.', 'edits': []})
                    continue
                gdeck = gfn(C.deck_copy(seed))
                gtext = D.render(gdeck, cand)
                if kind == 'write':
                    gold.append({'task_id': tid, 'text': gtext})
                else:
                    gold.append({'task_id': tid, 'edits': make_edits(text, gtext)})
            unit_path = os.path.join(OUT['units'], uid + '.json')
            with open(unit_path, 'w', encoding='utf-8') as f:
                json.dump(unit, f, ensure_ascii=False, indent=1)
            results = S.score(uid, {'answers': gold})
            for r in results:
                assert r['valid'] or r['flags'] == ['no_change'], (uid, r)
                if r['flags'] == ['no_change']:
                    unreachable.append({'task_id': r['task_id'], 'detail': 'the result shows as the seed does: '
                                        'the syntax cannot write the change'})
                elif not r['landed']:
                    unreachable.append({'task_id': r['task_id'], 'detail': r['detail']})
            unit['unreachable'] = unreachable
            with open(unit_path, 'w', encoding='utf-8') as f:
                json.dump(unit, f, ensure_ascii=False, indent=1)
            with open(os.path.join(OUT['gold'], uid + '.json'), 'w', encoding='utf-8') as f:
                json.dump({'answers': gold}, f, ensure_ascii=False, indent=1)
            prompt = prompt_for(unit, deck, cand, C.FILENAMES[rep])
            with open(os.path.join(OUT['prompts'], uid + '.md'), 'w', encoding='utf-8') as f:
                f.write(prompt)
            index.append({'unit_id': uid, 'part': part, 'candidate': cand, 'replicate': rep, 'prompt_file': 'prompts/%s.md' % uid,
                          'prompt_chars': len(prompt), 'seed_chars': len(text),
                          'gold_chars': sum(r['chars'] for r in results),
                          'unreachable': [u['task_id'] for u in unreachable]})
    index.sort(key=lambda u: (u['part'], D.CANDIDATES.index(u['candidate']), u['replicate']))
    with open(os.path.join(HERE, 'data', 'index.json'), 'w', encoding='utf-8') as f:
        json.dump(index, f, ensure_ascii=False, indent=1)
    for cand in D.CANDIDATES:
        print('guide %-2s %5d chars (candidate part %d)' % (cand, len(GUIDES[cand]), len(PARTS[cand])))
    for u in index:
        print('%-9s seed %5d  prompt %6d  gold %5d  unreachable %s' % (
            u['unit_id'], u['seed_chars'], u['prompt_chars'], u['gold_chars'], ','.join(u['unreachable']) or '-'))
    print('built %d units' % len(index))


if __name__ == '__main__':
    main()
