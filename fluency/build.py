#!/usr/bin/env python3
"""Builds seeds, tasks, gold answers and subject prompts from content.py. Run: python3 build.py"""
import json
import os
from difflib import SequenceMatcher

import content as C
import score
from guides import GUIDES

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = {k: os.path.join(HERE, *p) for k, p in {
    'units': ('data', 'units'), 'gold': ('data', 'gold'), 'seeds': ('seeds',), 'prompts': ('prompts',),
    'guides': ('guides',)}.items()}


# ---------------------------------------------------------------- renderers

def fm_text(fm):
    return '---\n' + ''.join('%s: %s\n' % (k, v) for k, v in fm.items()) + '---\n'


def span(c):
    return (c, 1, 1) if isinstance(c, str) else c


def has_merge(rows):
    return any(span(c)[1] > 1 or span(c)[2] > 1 for row in rows for c in row)


def html_table(rows, attr=None, style=None):
    out = ['<table%s>' % (' %s="%s"' % (attr, style) if style else '')]
    for r, row in enumerate(rows):
        tag = 'th' if r == 0 else 'td'
        cells = []
        for c in row:
            t, rs, cs = span(c)
            a = (' rowspan="%d"' % rs if rs > 1 else '') + (' colspan="%d"' % cs if cs > 1 else '')
            cells.append('<%s%s>%s</%s>' % (tag, a, t, tag))
        out.append('<tr>%s</tr>' % ''.join(cells))
    out.append('</table>')
    return out


def pipe_table(rows):
    occ = {}
    for r, row in enumerate(rows):
        c = 0
        for cell in row:
            while (r, c) in occ:
                c += 1
            t, rs, cs = span(cell)
            for dr in range(rs):
                for dc in range(cs):
                    occ[(r + dr, c + dc)] = (r, c, t)
            c += cs
    R = len(rows)
    W = max(c for (r, c) in occ if r == 0) + 1
    out = []
    for r in range(R):
        s = '|'
        for c in range(W):
            o_r, o_c, t = occ[(r, c)]
            if (o_r, o_c) == (r, c):
                s += ' %s |' % (t if t else ' ')
            elif r == o_r or c != o_c:
                s += '|'
            else:
                s += ' ^^ |'
        out.append(s)
        if r == 0:
            out.append('|' + '---|' * W)
    return out


def render_doc(unit, blocks, cand):
    dec = unit['decision']
    attr = {'A': 'style', 'B': 'class'}[cand] if dec == 'styleattr' else None
    parts = []
    prev = None
    for b in blocks:
        k = b[0]
        if k == 'h':
            lines = ['#' * b[1] + ' ' + b[2]]
        elif k == 'p':
            lines = [b[1]]
        elif k == 'div':
            lines = ['<div %s="%s">%s</div>' % (attr, b[1], b[2])]
        elif k == 'ul':
            lines = ['- ' + b[1]]
        elif k == 'ol':
            lines = ['1. ' + b[1]]
        elif k == 'table':
            style, rows = b[1], b[2]
            if style:
                lines = html_table(rows, attr, style)
            elif has_merge(rows):
                if dec != 'merge':
                    raise ValueError('merged table outside the merge decision')
                lines = html_table(rows) if cand == 'A' else pipe_table(rows)
            else:
                lines = pipe_table(rows)
        else:
            raise ValueError(k)
        if parts and not (k in ('ul', 'ol') and prev == k):
            parts.append('')
        parts.extend(lines)
        prev = k
    return fm_text(unit['fm']) + '\n' + '\n'.join(parts) + '\n'


def render_deck(unit, slides, cand):
    out = []
    for i, (layout, slots) in enumerate(slides):
        if cand == 'A':
            if i:
                out.append('')
            out.append('<slide layout="%s">' % layout)
            for name, lines in slots:
                if len(lines) == 1:
                    out.append('<%s>%s</%s>' % (name, lines[0], name))
                else:
                    out += ['<%s>' % name] + lines + ['</%s>' % name]
            out.append('</slide>')
        else:
            if i:
                out += ['', '---', '']
            out.append('layout: %s' % layout)
            for name, lines in slots:
                out += ['::%s::' % name] + lines
    return fm_text(unit['fm']) + '\n' + '\n'.join(out) + '\n'


def render(unit, model, cand):
    if unit['decision'] == 'slides':
        return render_deck(unit, model, cand)
    return render_doc(unit, model, cand)


# ---------------------------------------------------------------- gold edits by line diff

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

def names_section(dec, names_desc):
    if dec == 'merge':
        return 'This file uses no named styles or layouts.'
    if dec == 'styleattr':
        out = ['Paragraph styles:']
        out += ['- `%s` — %s' % nd for nd in names_desc['paragraph_styles']]
        out += ['', 'Table styles:']
        out += ['- `%s` — %s' % nd for nd in names_desc['table_styles']]
        return '\n'.join(out)
    out = ['Layouts and their slots (every layout also has `notes`):']
    out += ['- `%s`: %s' % (k, ', '.join(v)) for k, v in names_desc['layouts'].items()]
    return '\n'.join(out)


ANSWER_FORMAT = """\
Reply with one JSON object and nothing else:

```
{"answers": [
  {"task_id": "<id of a write task>", "text": "<the complete new file>"},
  {"task_id": "<id of an edit task>", "edits": [{"old": "<exact text from the file>", "new": "<replacement>"}]}
]}
```"""


def prompt_for(unit, names_desc, filename):
    dec = unit['decision']
    fence = '````'
    out = []
    out.append('You work with office files stored as plain text. Below are the syntax documentation, the names '
               'available in the file, the file itself, and four tasks. Do each task on its own: every edit task '
               'starts from the file exactly as shown, not from the result of another task.')
    out.append('\n## Syntax documentation\n\n' + GUIDES[dec, unit['candidate']])
    out.append('\n## Names available in this file\n\n' + names_section(dec, names_desc))
    out.append('\n## The file: %s\n\n%s\n%s%s' % (filename, fence, unit['seed'], fence))
    out.append('\n## Tasks')
    for t in unit['tasks']:
        if t['type'] == 'write':
            out.append('\n### %s (write)\n\n%s\n\nStart the new file with this front matter:\n\n%s\n%s%s\n\n'
                       'Answer with `text`: the complete new file.'
                       % (t['task_id'], t['instruction'], fence, t['front_matter'] + '\n', fence))
        else:
            out.append('\n### %s (edit)\n\n%s\n\nAnswer with `edits`: a list of {"old", "new"} pairs, applied in '
                       'order to the file above. Each `old` is copied exactly from the file (as it is after the '
                       'earlier pairs of this task) and must occur in it exactly once; it is replaced by `new`.'
                       % (t['task_id'], t['instruction']))
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
    for (dec, rep), spec in sorted(C.UNITS.items()):
        names_desc = spec.get('names', {})
        names = {k: [n for n, _ in v] for k, v in names_desc.items() if k != 'layouts'}
        if 'layouts' in names_desc:
            names = {'layouts': names_desc['layouts']}
        seed_model = spec['doc'] if dec != 'slides' else spec['deck']
        targets = []
        for instr, fn in spec['edits']:
            t = C.cp(seed_model)
            fn(t)
            targets.append(t)
        write_model = spec['write']['doc'] if dec != 'slides' else spec['write']['deck']
        sem = {}
        stem = {'merge': 'report', 'styleattr': 'document', 'slides': 'deck'}[dec]
        for cand in 'AB':
            uid = '%s-%s-%d' % (dec, cand, rep)
            unit = {'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep, 'names': names,
                    'fm': spec['fm']}
            seed = render(unit, seed_model, cand)
            unit['seed'] = seed
            fm = fm_text(spec['fm']).rstrip('\n')
            tasks = []
            gold = []
            # write task
            wtext = render(unit, write_model, cand)
            wm, wctx = score.parse(wtext, unit)
            assert not wctx.errors, (uid, wctx.error_text())
            items = wm['slides'] if dec == 'slides' else wm['blocks']
            tasks.append({'task_id': '%s%d-w' % (stem, rep), 'type': 'write',
                          'instruction': spec['write']['instruction'], 'front_matter': fm,
                          'expect': {'mode': 'exact' if dec == 'slides' else 'subsequence', 'items': items}})
            gold.append({'task_id': '%s%d-w' % (stem, rep), 'text': wtext})
            sems = [('seed', score.parse(seed, unit)), ('write', (wm, wctx))]
            for k, ((instr, fn), tmodel) in enumerate(zip(spec['edits'], targets), 1):
                tid = '%s%d-e%d' % (stem, rep, k)
                ttext = render(unit, tmodel, cand)
                edits = make_edits(seed, ttext)
                applied, flag, _ = score.apply_edits(seed, edits)
                assert flag is None and applied == ttext, uid
                tm, tctx = score.parse(ttext, unit)
                assert not tctx.errors, (uid, tid, tctx.error_text())
                sems.append((tid, (tm, tctx)))
                tasks.append({'task_id': tid, 'type': 'edit', 'instruction': instr})
                gold.append({'task_id': tid, 'edits': edits})
            for name, (m, ctx) in sems:
                assert not ctx.errors, (uid, name, ctx.error_text())
            sem[cand] = [m for _, (m, _) in sems]
            unit['tasks'] = tasks
            ext = 'hj.md'
            filename = '%s-%d.%s' % (stem, rep, ext)
            with open(os.path.join(OUT['units'], uid + '.json'), 'w', encoding='utf-8') as f:
                json.dump(unit, f, ensure_ascii=False, indent=1)
            with open(os.path.join(OUT['gold'], uid + '.json'), 'w', encoding='utf-8') as f:
                json.dump({'answers': gold}, f, ensure_ascii=False, indent=1)
            with open(os.path.join(OUT['seeds'], '%s-%d-%s.txt' % (dec, rep, cand)), 'w', encoding='utf-8') as f:
                f.write(seed)
            prompt = prompt_for(unit, names_desc, filename)
            with open(os.path.join(OUT['prompts'], uid + '.md'), 'w', encoding='utf-8') as f:
                f.write(prompt)
            index.append({'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep,
                          'prompt_file': 'prompts/%s.md' % uid, 'prompt_chars': len(prompt),
                          'seed_chars': len(seed)})
        # both candidates must mean exactly the same thing
        assert sem['A'] == sem['B'], (dec, rep, score.first_diff(sem['A'], sem['B']))
    with open(os.path.join(HERE, 'data', 'index.json'), 'w', encoding='utf-8') as f:
        json.dump(index, f, ensure_ascii=False, indent=1)
    for dec in ('merge', 'styleattr', 'slides'):
        la, lb = len(GUIDES[dec, 'A']), len(GUIDES[dec, 'B'])
        print('guide %-9s A=%d B=%d chars (B/A %.2f)' % (dec, la, lb, lb / la))
    print('built %d units' % len(index))


if __name__ == '__main__':
    main()
