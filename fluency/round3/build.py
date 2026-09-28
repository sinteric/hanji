#!/usr/bin/env python3
"""Builds round 3 seeds, tasks, gold answers and subject prompts from content.py. Run: python3 build.py
Adapted from ../round2/build.py; round 1 and round 2 files are not changed."""
import json
import os
import re
from difflib import SequenceMatcher

import content as C
import score
from guides import GUIDES

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = {k: os.path.join(HERE, *p) for k, p in {
    'units': ('data', 'units'), 'gold': ('data', 'gold'), 'seeds': ('seeds',), 'prompts': ('prompts',),
    'guides': ('guides',)}.items()}
DECISIONS = ('cellpara', 'emptypara')
SEED_MIN, SEED_MAX = 3000, 6000


# ---------------------------------------------------------------- renderers

def fm_text(fm):
    return '---\n' + ''.join('%s: %s\n' % (k, v) for k, v in fm.items()) + '---\n'


def regions(rows):
    """Dense rows ('^^', '<<' markers) -> (origin map, R, W)."""
    R, W = len(rows), len(rows[0])
    origin = {}
    for r in range(R):
        assert len(rows[r]) == W, ('row width', rows[r])
        for c in range(W):
            t = rows[r][c]
            if t == '^^':
                origin[r, c] = origin[r - 1, c]
            elif t == '<<':
                origin[r, c] = origin[r, c - 1]
            else:
                origin[r, c] = (r, c)
    groups = {}
    for p, o in origin.items():
        groups.setdefault(o, []).append(p)
    for o, ps in groups.items():
        rs = max(p[0] for p in ps) - o[0] + 1
        cs = max(p[1] for p in ps) - o[1] + 1
        assert len(ps) == rs * cs and min(ps) == o, ('not a rectangle', o, rows)
    return origin, R, W


def needs_list(rows):
    for row in rows:
        for t in row:
            if t in ('^^', '<<') or isinstance(t, str):
                continue
            ps = C.paras(t)
            if len(ps) > 1 or ps[0][0]:
                return True
    return False


def cell_A(t):
    out = ''
    for k, (s, x) in enumerate(C.paras(t)):
        assert x, 'a cell paragraph has text'
        if s:
            out += '<p style="%s"/>' % s
        elif k:
            out += '<p/>'
        out += x
    return out


def cell_plain(t):
    ps = C.paras(t)
    assert len(ps) == 1 and ps[0][0] is None, t
    return ps[0][1]


def pipe_table(rows, fmt):
    origin, R, W = regions(rows)
    out = []
    for r in range(R):
        s = '|'
        for c in range(W):
            o_r, o_c = origin[r, c]
            if (o_r, o_c) == (r, c):
                t = fmt(rows[r][c])
                s += (' %s |' % t) if t else '  |'
            elif r == o_r or c != o_c:
                s += '|'
            else:
                s += ' ^^ |'
        out.append(s)
        if r == 0:
            out.append('|' + '---|' * W)
    return out


def list_table(rows):
    origin, R, W = regions(rows)
    out = ['{list-table}']
    for r in range(R):
        out.append('-')
        for c in range(W):
            o_r, o_c = origin[r, c]
            if (o_r, o_c) != (r, c):
                out.append('  - ' + ('||' if (r == o_r or c != o_c) else '^^'))
                continue
            ps = C.paras(rows[r][c])
            if ps == [(None, '')]:
                out.append('  -')
                continue
            for k, (s, x) in enumerate(ps):
                assert x, 'a cell paragraph has text'
                line = ('<div style="%s">%s</div>' % (s, x)) if s else x
                out.append(('  - ' if k == 0 else '    ') + line)
    return out


def render_table(dec, cand, style, rows):
    pre = ['{style="%s"}' % style] if style else []
    if dec == 'cellpara' and cand == 'A':
        return pre + pipe_table(rows, cell_A)
    if dec == 'cellpara' and needs_list(rows):
        return pre + list_table(rows)
    return pre + pipe_table(rows, cell_plain)


def render_empty(dec, cand, style):
    assert dec == 'emptypara'
    if cand == 'A':
        return '<p style="%s"/>' % style if style else '<p/>'
    return '<div style="%s"></div>' % style if style else '<div></div>'


def render_doc(unit, blocks, cand):
    dec = unit['decision']
    parts = []
    prev = None
    tight = {('ul', 'ul'), ('ul', 'ul2'), ('ul2', 'ul'), ('ul2', 'ul2'), ('e', 'e')}
    for b in blocks:
        k = b[0]
        if k == 'h':
            lines = ['#' * b[1] + ' ' + b[2]]
        elif k == 'p':
            assert not re.match(r'^(\d+[.)]|[-*+])(\s|$)|^[#|<{]', b[1]), ('plain paragraph reads as another block', b[1])
            lines = [b[1]]
        elif k == 'div':
            lines = ['<div style="%s">%s</div>' % (b[1], b[2])]
        elif k == 'e':
            lines = [render_empty(dec, cand, b[1])]
        elif k == 'ul':
            lines = ['- ' + b[1]]
        elif k == 'ul2':
            lines = ['  - ' + b[1]]
        elif k == 'table':
            lines = render_table(dec, cand, b[1], b[2])
        elif k == 'pagebreak':
            lines = ['<pagebreak/>']
        else:
            raise ValueError(k)
        if parts and (prev, k) not in tight:
            parts.append('')
        parts.extend(lines)
        prev = k
    return fm_text(unit['fm']) + '\n' + '\n'.join(parts) + '\n'


# ---------------------------------------------------------------- gold edits by line diff (as rounds 1 and 2)

def make_edits(a_text, b_text):
    a = a_text.splitlines(keepends=True)
    b = b_text.splitlines(keepends=True)
    ops = []
    for op in SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        if op[0] == 'equal':
            continue
        if ops and op[1] - ops[-1][2] <= 2:   # join hunks separated by at most two unchanged lines
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
            if old.strip() and score.count_occ(''.join(cur), old) == 1:
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


# ---------------------------------------------------------------- prompts

def names_section(names_desc):
    out = ['Paragraph styles:']
    out += ['- `%s` — %s' % nd for nd in names_desc['paragraph_styles']]
    out += ['', 'Table styles:']
    out += ['- `%s` — %s' % nd for nd in names_desc['table_styles']]
    return '\n'.join(out)


REFUSAL_RULE = """\
Do only what the syntax documentation and the names above can express. If an edit task asks for something they cannot express, do not approximate it, and do not invent a name, tag or attribute: refuse that task by answering it with `"text": "REFUSE: <one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; every other task gets its edits."""

ANSWER_FORMAT = """\
Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]},
  {"task_id": "<id of an edit task you refuse>", "text": "REFUSE: <reason>", "edits": []}
]}
```"""


def prompt_for(unit, names_desc, filename):
    dec = unit['decision']
    fence = '````'
    out = []
    out.append('You work with office files stored as plain text. Below are the syntax documentation, the names '
               'available in the file, the file itself, and five tasks. Do each task on its own: every edit task '
               'starts from the file exactly as shown, not from the result of another task.')
    out.append('\n## Syntax documentation\n\n' + GUIDES[dec, unit['candidate']])
    out.append('\n## Names available in this file\n\n' + names_section(names_desc))
    out.append('\n## The file: %s\n\n%s\n%s%s' % (filename, fence, unit['seed'], fence))
    out.append('\n## Tasks\n\nA write task is answered with `text`: the complete new file. An edit task is answered '
               'with `edits`: a list of {"old", "new"} pairs, applied in order to the file above. Each `old` is copied '
               'exactly from the file (as it is after the earlier pairs of that task) and must occur in it exactly '
               'once; it is replaced by `new`.')
    for t in unit['tasks']:
        if t['type'] == 'write':
            out.append('\n### %s (write)\n\n%s\n\nStart the new file with this front matter:\n\n%s\n%s%s'
                       % (t['task_id'], t['instruction'], fence, t['front_matter'] + '\n', fence))
        else:
            out.append('\n### %s (edit)\n\n%s' % (t['task_id'], t['instruction']))
    out.append('\n## Tasks that cannot be done\n\n' + REFUSAL_RULE)
    out.append('\n## Answer format\n\n' + ANSWER_FORMAT)
    return '\n'.join(out) + '\n'


# ---------------------------------------------------------------- main

def main():
    for p in OUT.values():
        os.makedirs(p, exist_ok=True)
    for (dec, cand), g in GUIDES.items():
        with open(os.path.join(OUT['guides'], '%s-%s.md' % (dec, cand)), 'w', encoding='utf-8') as f:
            f.write(g + '\n')
    index = []
    for dec in DECISIONS:
        for rep in (1, 2, 3):
            spec = C.UNITS[dec, rep]
            names_desc = spec['names']
            names = {k: [n for n, _ in v] for k, v in names_desc.items()}
            seed_model = spec['doc']
            targets = []
            for e in spec['edits']:
                if e[1] is None:
                    targets.append(None)
                    continue
                t = C.cp(seed_model)
                e[1](t)
                assert t != seed_model, (dec, rep, e[0][:40])
                targets.append(t)
            write_model = spec['write']['doc']
            stem = spec['stem']
            sem = {}
            for cand in 'AB':
                uid = 'r3-%s-%s-%d' % (dec, cand, rep)
                unit = {'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep, 'names': names,
                        'fm': spec['fm']}
                seed = render_doc(unit, seed_model, cand)
                assert SEED_MIN <= len(seed) <= SEED_MAX, (uid, 'seed length', len(seed))
                unit['seed'] = seed
                fm = fm_text(spec['fm']).rstrip('\n')
                tasks = []
                gold = []
                wtext = render_doc(unit, write_model, cand)
                wm, wctx = score.parse(wtext, unit)
                assert not wctx.errors, (uid, 'write', wctx.error_text())
                wid = '%s%d-w' % (stem, rep)
                tasks.append({'task_id': wid, 'type': 'write', 'instruction': spec['write']['instruction'],
                              'front_matter': fm, 'expect': {'mode': 'exact', 'items': wm['blocks']}})
                gold.append({'task_id': wid, 'text': wtext})
                sm, sctx = score.parse(seed, unit)
                assert not sctx.errors, (uid, 'seed', sctx.error_text())
                sems = [sm, wm]
                for k, (e, tmodel) in enumerate(zip(spec['edits'], targets), 1):
                    tid = '%s%d-e%d' % (stem, rep, k)
                    tasks.append({'task_id': tid, 'type': 'edit', 'instruction': e[0]})
                    if tmodel is None:
                        gold.append({'task_id': tid, 'text': 'REFUSE: %s.' % e[2], 'edits': []})
                        sems.append('refuse')
                        continue
                    ttext = render_doc(unit, tmodel, cand)
                    edits = make_edits(seed, ttext)
                    applied, flag, _ = score.apply_edits(seed, edits)
                    assert flag is None and applied == ttext, uid
                    tm, tctx = score.parse(ttext, unit)
                    assert not tctx.errors, (uid, tid, tctx.error_text())
                    assert tm != sm, (uid, tid, 'edit changes nothing')
                    sems.append(tm)
                    gold.append({'task_id': tid, 'edits': edits})
                sem[cand] = sems
                unit['tasks'] = tasks
                filename = '%s-%d.hj.md' % (stem, rep)
                with open(os.path.join(OUT['units'], uid + '.json'), 'w', encoding='utf-8') as f:
                    json.dump(unit, f, ensure_ascii=False, indent=1)
                with open(os.path.join(OUT['gold'], uid + '.json'), 'w', encoding='utf-8') as f:
                    json.dump({'answers': gold}, f, ensure_ascii=False, indent=1)
                with open(os.path.join(OUT['seeds'], uid + '.txt'), 'w', encoding='utf-8') as f:
                    f.write(seed)
                prompt = prompt_for(unit, names_desc, filename)
                with open(os.path.join(OUT['prompts'], uid + '.md'), 'w', encoding='utf-8') as f:
                    f.write(prompt)
                index.append({'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep,
                              'prompt_file': 'prompts/%s.md' % uid, 'prompt_chars': len(prompt),
                              'seed_chars': len(seed),
                              'refusal_tasks': [t['task_id'] for t, g in zip(tasks, gold)
                                                if g.get('text', '').startswith('REFUSE:')]})
            # both candidates must mean exactly the same thing
            assert sem['A'] == sem['B'], (dec, rep, score.first_diff(sem['A'], sem['B']))
    with open(os.path.join(HERE, 'data', 'index.json'), 'w', encoding='utf-8') as f:
        json.dump(index, f, ensure_ascii=False, indent=1)
    for dec in DECISIONS:
        la, lb = len(GUIDES[dec, 'A']), len(GUIDES[dec, 'B'])
        print('guide %-10s A=%d B=%d chars (B/A %.2f)' % (dec, la, lb, lb / la))
    for u in index:
        print('%-20s seed %5d  prompt %6d' % (u['unit_id'], u['seed_chars'], u['prompt_chars']))
    print('built %d units' % len(index))


if __name__ == '__main__':
    main()
