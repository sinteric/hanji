#!/usr/bin/env python3
"""Builds round 4 units, gold answers and subject prompts from content.py. Run: python3 build.py
make_edits is round 3's (../round3/build.py); rounds 1-3 are not changed."""
import importlib.util
import json
import os
from difflib import SequenceMatcher

import content as C
import wb as W
from guides import GUIDES, CANDIDATE_PART

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location('hanji_r2score', os.path.join(HERE, '..', 'round2', 'score.py'))
r2 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(r2)

OUT = {k: os.path.join(HERE, *p) for k, p in {
    'units': ('data', 'units'), 'gold': ('data', 'gold'), 'views': ('views',), 'prompts': ('prompts',),
    'guides': ('guides',)}.items()}
DECISIONS = {'readview': 'AB', 'writeshape': 'ABC'}
ROWS_MIN, ROWS_MAX = 150, 400


# ---------------------------------------------------------------- gold edits by line diff (round 3's make_edits)

def make_edits(a_text, b_text):
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
            if old.strip() and r2.count_occ(''.join(cur), old) == 1:
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

INTRO = {
    'readview': 'You work with spreadsheet workbooks shown as text. Below are the documentation of the text form, '
                'the workbook itself, and five questions about it. Answer every question from the workbook text '
                'alone.',
    'writeshape': 'You work with spreadsheet workbooks shown as text. Below are the documentation of the text form '
                  'and of how changes are written, the workbook itself, and six tasks. Do each task on its own: every '
                  'task starts from the workbook exactly as shown, not from the result of another task.',
}

HOW = {
    'readview': 'Each question is answered with `text`: `ANSWER: ` followed by the value, exactly as asked (one '
                'value, or a short list separated by commas). Nothing else goes into the text.',
    ('writeshape', 'A'): 'The write task and each edit task are answered with `text`: the JSON list of operations '
                         'that makes the change.',
    ('writeshape', 'B'): 'The write task and each edit task are answered with `edits`: the list of {"old", "new"} '
                         'pairs that makes the change.',
    ('writeshape', 'C'): 'The write task and each edit task are answered with `text`: the Python code that makes the '
                         'change.',
}

REFUSAL = {
    'readview': 'If a question cannot be answered from the workbook text, do not guess: answer it with `"text": '
                '"REFUSE: <one sentence saying why>"`. Refuse only when the question cannot be answered; every other '
                'question gets its `ANSWER:`.',
    'writeshape': 'Do only what the documentation above allows. If a task asks for something it cannot express or '
                  'does not allow, do not approximate it: refuse that task by answering it with `"text": "REFUSE: '
                  '<one sentence saying why>"` and `"edits": []`. Refuse only when the task cannot be done as asked; '
                  'every other task gets its answer.',
}

FORMAT = {
    'readview': '{"answers": [\n  {"task_id": "<id of a question>", "text": "ANSWER: <value>"},\n'
                '  {"task_id": "<id of a question you refuse>", "text": "REFUSE: <reason>"}\n]}',
    ('writeshape', 'A'): '{"answers": [\n  {"task_id": "<id of a task>", "text": [{"op": "…", …}, …]},\n'
                         '  {"task_id": "<id of a task you refuse>", "text": "REFUSE: <reason>", "edits": []}\n]}',
    ('writeshape', 'B'): '{"answers": [\n  {"task_id": "<id of a task>", "edits": [{"old": "<exact text from the '
                         'workbook>", "new": "<replacement>"}]},\n'
                         '  {"task_id": "<id of a task you refuse>", "text": "REFUSE: <reason>", "edits": []}\n]}',
    ('writeshape', 'C'): '{"answers": [\n  {"task_id": "<id of a task>", "text": "<Python code>"},\n'
                         '  {"task_id": "<id of a task you refuse>", "text": "REFUSE: <reason>", "edits": []}\n]}',
}


def key(dec, cand):
    return dec if dec == 'readview' else (dec, cand)


def prompt_for(unit, spec):
    dec, cand = unit['decision'], unit['candidate']
    fence = '````'
    out = [INTRO[dec]]
    out.append('\n## Documentation\n\n' + GUIDES[dec, cand])
    out.append('\n## The workbook: %s\n\n%s\n%s%s' % (spec['file'], fence, unit['view'], fence))
    out.append('\n## Tasks\n\n' + HOW[key(dec, cand)])
    for t in unit['tasks']:
        kind = {'question': 'question', 'write': 'write', 'edit': 'edit'}[t['type']]
        out.append('\n### %s (%s)\n\n%s' % (t['task_id'], kind, t['instruction']))
    out.append('\n## Tasks that cannot be done\n\n' + REFUSAL[dec])
    out.append('\n## Answer format\n\nReply with one JSON object and nothing else:\n\n```\n%s\n```'
               % FORMAT[key(dec, cand)])
    return '\n'.join(out) + '\n'


# ---------------------------------------------------------------- gold per write shape

def gold_answer(cand, task, seed_book, seed_view):
    """-> (answer, resulting Book). A: ops; C: code; B: edits derived from the ops result."""
    if task['ops'] is None:
        return {'text': 'REFUSE: %s.' % task['refuse'], 'edits': []}, None
    ba = W.Book(seed_book)
    W.run_ops(ba, json.dumps(task['ops'], ensure_ascii=False))
    ba.validate()
    if cand == 'A':
        return {'text': task['ops']}, ba
    if cand == 'C':
        bc = W.Book(seed_book)
        W.run_code(bc, task['code'])
        bc.validate()
        return {'text': task['code']}, bc
    seed_ranges = {t['name']: W.table_range(t) for s in seed_book['sheets'] for t in s['tables']}
    target = W.window_text(ba, labels='orig', ranges=seed_ranges, blank_new_formulas=True)
    edits = make_edits(seed_view, target)
    text, flag, _ = r2.apply_edits(seed_view, edits)
    assert flag is None and text == target
    bb = W.parse_window(text, W.Book(seed_book))
    bb.validate()
    return {'edits': edits}, bb


def check_index(book, text):
    """The index view is lossless: every cell's raw value, and only those, can be read back from it."""
    got = {}
    table = None
    for line in text.split('\n'):
        if line.startswith('<index '):
            table = line.split('table="')[1].split('"')[0]
            continue
        if line == '</index>':
            table = None
            continue
        if table is None:
            continue
        parts = line.split(' | ')
        letter = parts[0].split(' ')[0]
        for e in parts[1:]:
            v, rows = e.rsplit(': ', 1)
            for run in rows.split(','):
                a, _, z = run.partition(':')
                for r in range(int(a), int(z or a) + 1):
                    k = (table, letter, r)
                    assert k not in got, k
                    got[k] = v
    want = {}
    ev = book.ev()
    for s in book.d['sheets']:
        for t in s['tables']:
            c0, r0 = W.anchor_rc(t)
            for ci, c in enumerate(t['columns']):
                for ri, v in enumerate(ev.column(t, ci)):
                    if v is not None:
                        want[t['name'], W.col_letter(c0 + ci), r0 + 1 + ri] = W.raw(v, c)
    assert got == want, 'index view is not lossless'


def main():
    for p in OUT.values():
        os.makedirs(p, exist_ok=True)
    for (dec, cand), g in GUIDES.items():
        with open(os.path.join(OUT['guides'], '%s-%s.md' % (dec, cand)), 'w', encoding='utf-8') as f:
            f.write(g + '\n')
    index = []
    for rep in (1, 2, 3):
        spec = C.WORKBOOKS[rep]()
        book = spec['book']
        B0 = W.Book(book)
        B0.validate()
        nrows = sum(len(t['rows']) for s in book['sheets'] for t in s['tables'])
        assert ROWS_MIN <= nrows <= ROWS_MAX, (rep, nrows)
        assert 1 <= len(book['sheets']) <= 3
        stem = '%s%d' % (spec['stem'], rep)
        window = W.window_text(B0)
        views = {'A': window, 'B': W.index_text(B0)}
        # the window parses back to the seed (the text form is lossless)
        assert W.parse_window(window, B0).semantic() == B0.semantic(), stem
        check_index(B0, views['B'])
        for dec, cands in DECISIONS.items():
            sems = {}
            for cand in cands:
                uid = 'r4-%s-%s-%d' % (dec, cand, rep)
                view = views[cand] if dec == 'readview' else window
                unit = {'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep, 'stem': stem,
                        'file': spec['file'], 'rows': nrows, 'view': view, 'view_chars': len(view), 'book': book}
                tasks, gold = [], []
                if dec == 'readview':
                    for qid, kind, text, accept in spec['questions']:
                        tid = '%s-%s' % (stem, qid)
                        tasks.append({'task_id': tid, 'type': 'question', 'kind': kind, 'instruction': text})
                        gold.append({'task_id': tid, 'text': 'ANSWER: %s' % accept[0], 'accept': accept})
                else:
                    sem = []
                    for t in spec['tasks']:
                        tid = '%s-%s' % (stem, t['id'])
                        tasks.append({'task_id': tid, 'type': t['type'], 'instruction': t['instruction']})
                        ans, res = gold_answer(cand, t, book, window)
                        gold.append(dict(ans, task_id=tid))
                        if res is not None:
                            assert res.semantic() != B0.semantic(), (uid, tid, 'changes nothing')
                        sem.append(None if res is None else res.semantic())
                    sems[cand] = sem
                unit['tasks'] = tasks
                with open(os.path.join(OUT['units'], uid + '.json'), 'w', encoding='utf-8') as f:
                    json.dump(unit, f, ensure_ascii=False, indent=1)
                with open(os.path.join(OUT['gold'], uid + '.json'), 'w', encoding='utf-8') as f:
                    json.dump({'answers': gold}, f, ensure_ascii=False, indent=1)
                with open(os.path.join(OUT['views'], uid + '.txt'), 'w', encoding='utf-8') as f:
                    f.write(view)
                prompt = prompt_for(unit, spec)
                with open(os.path.join(OUT['prompts'], uid + '.md'), 'w', encoding='utf-8') as f:
                    f.write(prompt)
                index.append({'unit_id': uid, 'decision': dec, 'candidate': cand, 'replicate': rep,
                              'prompt_file': 'prompts/%s.md' % uid, 'prompt_chars': len(prompt),
                              'view_chars': len(view), 'rows': nrows,
                              'refusal_tasks': [g['task_id'] for g in gold
                                                if isinstance(g.get('text'), str) and g['text'].startswith('REFUSE:')]})
            if dec == 'writeshape':
                # all three write shapes must give exactly the same workbook for every task
                assert sems['A'] == sems['B'] == sems['C'], (rep, [i for i, (a, b, c) in enumerate(
                    zip(sems['A'], sems['B'], sems['C'])) if not a == b == c])
    index.sort(key=lambda u: (list(DECISIONS).index(u['decision']), u['candidate'], u['replicate']))
    with open(os.path.join(HERE, 'data', 'index.json'), 'w', encoding='utf-8') as f:
        json.dump(index, f, ensure_ascii=False, indent=1)
    for (dec, cand), g in sorted(GUIDES.items()):
        print('guide %-10s %s  %5d chars (candidate part %d)' % (dec, cand, len(g), len(CANDIDATE_PART[dec, cand])))
    for u in index:
        print('%-18s rows %3d  view %6d  prompt %6d' % (u['unit_id'], u['rows'], u['view_chars'], u['prompt_chars']))
    print('built %d units' % len(index))


if __name__ == '__main__':
    main()
