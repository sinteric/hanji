#!/usr/bin/env python3
"""Sends round 6 prompts to the subjects. Usage: python3 run.py <workdir> [unit-part …] [--models opus,sonnet]
[--fix]

Each prompt goes to a fresh `claude -p` conversation whose only tool is Read, started in an empty directory that
holds nothing but the prompt as prompt.md (round 4's lesson: a long prompt is read from its own file, the same way
for both models). The message is MESSAGE below. The answer is saved as runs/<unit>/<model>-<part>.json (a code
fence stripped), and runs/<unit>/<model>-first.json merges the two parts. Token counts go to
runs/<unit>/<model>-<part>.usage.json. With --fix, every unit whose first answer has an invalid task gets one fresh
conversation: the prompt, the first answer and the validator's errors (runs/<unit>/<model>-fix.json)."""
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

MESSAGE = ('Read the file prompt.md in the current directory and do what it says. Read only that file. It may be '
           'long: read it in parts if you need to, until you have read all of it. Reply with the JSON object only.')

FIX = ('Read the file prompt.md in the current directory: it holds a task prompt, then the answer you gave, then the '
       'validator\'s errors for it. Read only that file, all of it. Reply with a corrected JSON object for the '
       'whole prompt, and nothing else.')


def strip_fence(t):
    t = t.strip()
    m = re.search(r'```(?:json)?\s*\n(.*?)\n```', t, re.S)
    if m:
        return m.group(1)
    i = t.find('{')
    j = t.rfind('}')
    return t[i:j + 1] if i >= 0 and j > i else t


def call(workdir, key, model, prompt, message):
    d = os.path.join(workdir, '%s-%s' % (key, model))
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, 'prompt.md'), 'w', encoding='utf-8').write(prompt)
    env = dict(os.environ, CLAUDE_CODE_MAX_OUTPUT_TOKENS='64000')
    p = subprocess.run(['claude', '-p', message, '--model', model, '--tools', 'Read', '--output-format', 'json'],
                       cwd=d, capture_output=True, text=True, env=env, timeout=3600)
    try:
        out = json.loads(p.stdout)
    except ValueError:
        return None, {'error': (p.stdout or p.stderr)[-2000:]}
    usage = out.get('usage', {})
    meta = {'input_tokens': usage.get('input_tokens', 0) + usage.get('cache_read_input_tokens', 0) +
            usage.get('cache_creation_input_tokens', 0), 'output_tokens': usage.get('output_tokens', 0),
            'num_turns': out.get('num_turns'), 'is_error': out.get('is_error'),
            'duration_ms': out.get('duration_ms')}
    return out.get('result', ''), meta


def run_one(workdir, unit, part, model):
    prompt = open(os.path.join(HERE, 'prompts', '%s-%s.md' % (unit, part)), encoding='utf-8').read()
    res, meta = call(workdir, '%s-%s' % (unit, part), model, prompt, MESSAGE)
    rd = os.path.join(HERE, 'runs', unit)
    os.makedirs(rd, exist_ok=True)
    json.dump(meta, open(os.path.join(rd, '%s-%s.usage.json' % (model, part)), 'w'), indent=1)
    if res is None:
        print('FAILED', unit, part, model, meta)
        return
    txt = strip_fence(res)
    try:
        ans = json.loads(txt)
    except ValueError:
        ans = {'answers': [], 'unparsed': res}
    json.dump(ans, open(os.path.join(rd, '%s-%s.json' % (model, part)), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)
    print('done', unit, part, model, meta.get('output_tokens'), 'tokens out')


def merge(unit, model):
    rd = os.path.join(HERE, 'runs', unit)
    answers = []
    for part in ('a', 'b'):
        f = os.path.join(rd, '%s-%s.json' % (model, part))
        if os.path.exists(f):
            answers += json.load(open(f, encoding='utf-8')).get('answers', [])
    json.dump({'answers': answers}, open(os.path.join(rd, '%s-first.json' % model), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)


def fix_one(workdir, unit, model):
    import score as S
    if unit.startswith('r6p-'):
        import pptx_kit as S
    if unit.startswith('r6x-'):
        import xlsx_kit as S
    rd = os.path.join(HERE, 'runs', unit)
    first = json.load(open(os.path.join(rd, '%s-first.json' % model), encoding='utf-8'))
    res = S.score_answer(unit, first)
    bad = [r for r in res if not r['valid'] and r['flags'] != ['missing'] and 'unreachable' not in ' '.join(r['flags'])]
    if not bad:
        return
    ids = {r['task_id'] for r in bad}
    parts = {p for p, tids in PARTS_OF(unit).items() if ids & set(tids)}
    fixed = {a['task_id']: a for a in first['answers'] if isinstance(a, dict)}
    for part in sorted(parts):
        prompt = open(os.path.join(HERE, 'prompts', '%s-%s.md' % (unit, part)), encoding='utf-8').read()
        mine = json.load(open(os.path.join(rd, '%s-%s.json' % (model, part)), encoding='utf-8'))
        errs = '\n'.join('- `%s`: %s' % (r['task_id'], r['error']) for r in bad if r['task_id'] in PARTS_OF(unit)[part])
        body = (prompt + '\n\n## Your answer\n\n```\n' + json.dumps(mine, ensure_ascii=False) + '\n```\n\n'
                '## Validator errors\n\nThese answers could not be applied or read back:\n\n' + errs + '\n\n'
                'Reply with the whole JSON object again, every task included, with these answers corrected.\n')
        json.dump({'message': FIX, 'errors': errs}, open(os.path.join(rd, '%s-fix-message-%s.json' % (model, part)),
                                                          'w', encoding='utf-8'), ensure_ascii=False, indent=1)
        out, meta = call(workdir, '%s-%s-fix' % (unit, part), model, body, FIX)
        json.dump(meta, open(os.path.join(rd, '%s-fix-%s.usage.json' % (model, part)), 'w'), indent=1)
        if out is None:
            continue
        try:
            ans = json.loads(strip_fence(out))
        except ValueError:
            continue
        for a in ans.get('answers', []):
            if isinstance(a, dict) and a.get('task_id') in ids:
                fixed[a['task_id']] = a
    json.dump({'answers': list(fixed.values())}, open(os.path.join(rd, '%s-fix.json' % model), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)
    print('fixed', unit, model, sorted(ids))


def PARTS_OF(unit):
    idx = json.load(open(os.path.join(HERE, 'data', os.environ.get('INDEX', 'index.json'))))
    return {x['part']: x['tasks'] for x in idx if x['unit_id'] == unit}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    workdir = args[0]
    models = ['opus', 'sonnet']
    for a in sys.argv:
        if a.startswith('--models='):
            models = a.split('=', 1)[1].split(',')
    idx = json.load(open(os.path.join(HERE, 'data', os.environ.get('INDEX', 'index.json'))))
    todo = args[1:] or ['%s-%s' % (x['unit_id'], x['part']) for x in idx]
    if '--fix' in sys.argv:
        units = sorted({t.rsplit('-', 1)[0] for t in todo})
        with ThreadPoolExecutor(int(os.environ.get('JOBS', '6'))) as ex:
            list(ex.map(lambda um: fix_one(workdir, *um), [(u, m) for u in units for m in models]))
        return
    jobs = [(t.rsplit('-', 1)[0], t.rsplit('-', 1)[1], m) for t in todo for m in models]
    with ThreadPoolExecutor(int(os.environ.get('JOBS', '6'))) as ex:
        list(ex.map(lambda j: run_one(workdir, *j), jobs))
    for u in sorted({j[0] for j in jobs}):
        for m in models:
            merge(u, m)


if __name__ == '__main__':
    main()
