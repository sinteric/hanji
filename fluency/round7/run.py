#!/usr/bin/env python3
"""Sends round 7 prompts to the subjects. Usage:

    python3 run.py <workdir> [unit_id …] [--models=sonnet55,haiku55,sonnet46]
    python3 run.py <workdir> --fix

Each prompt goes to a fresh `claude -p` conversation whose only tool is Read, started in an empty directory that
holds nothing but the prompt as prompt.md (round 4's lesson: a long prompt is read from its own file, the same way
for every model). The answer is saved as runs/<unit>/<label>-first.json (a code fence stripped). Token counts go
to runs/<unit>/<label>-first.usage.json. With --fix, every unit whose first answer has an invalid task gets one
fresh conversation: the prompt, the first answer and the validator's errors (runs/<unit>/<label>-fix.json).

The keys of MODELS are the short labels used in file names. The values are the ids passed to
`claude -p --model <id>`. The owner's box sets MODELS if the aliases below do not match the local CLI.
"""
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

# ---------------------------------------------------------------------------
# CLI model ids passed to `claude -p --model <id>`.
# The owner's box sets these if the aliases below do not match the local CLI.
# File-name labels (the dict keys) stay as they are: sonnet55, haiku55, sonnet46.
# ---------------------------------------------------------------------------
MODELS = {
    'sonnet55': 'claude-sonnet-5-5',
    'haiku55': 'claude-haiku-5-5',
    'sonnet46': 'claude-sonnet-4-6',
}

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


def parse_models(argv, default=None):
    labels = list((default or MODELS).keys())
    for a in argv:
        if a.startswith('--models='):
            labels = [x.strip() for x in a.split('=', 1)[1].split(',') if x.strip()]
    if '--models' in argv:
        i = argv.index('--models')
        if i + 1 < len(argv) and not argv[i + 1].startswith('--'):
            labels = [x.strip() for x in argv[i + 1].split(',') if x.strip()]
    if not labels:
        labels = list(MODELS)
    unknown = [l for l in labels if l not in MODELS]
    if unknown:
        sys.exit('unknown model label %s (known: %s)' % (', '.join(unknown), ', '.join(MODELS)))
    return labels


def call(workdir, key, model_id, prompt, message):
    d = os.path.join(workdir, '%s-%s' % (key, model_id.replace('/', '_')))
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, 'prompt.md'), 'w', encoding='utf-8').write(prompt)
    env = dict(os.environ, CLAUDE_CODE_MAX_OUTPUT_TOKENS='64000')
    p = subprocess.run(['claude', '-p', message, '--model', model_id, '--tools', 'Read',
                        '--output-format', 'json'],
                       cwd=d, capture_output=True, text=True, env=env, timeout=3600)
    try:
        out = json.loads(p.stdout)
    except ValueError:
        return None, {'error': (p.stdout or p.stderr)[-2000:]}
    usage = out.get('usage', {})
    meta = {'input_tokens': usage.get('input_tokens', 0) + usage.get('cache_read_input_tokens', 0) +
            usage.get('cache_creation_input_tokens', 0), 'output_tokens': usage.get('output_tokens', 0),
            'num_turns': out.get('num_turns'), 'is_error': out.get('is_error'),
            'duration_ms': out.get('duration_ms'), 'model': model_id}
    return out.get('result', ''), meta


def run_one(workdir, unit, label):
    prompt = open(os.path.join(HERE, 'prompts', unit + '.md'), encoding='utf-8').read()
    res, meta = call(workdir, unit, MODELS[label], prompt, MESSAGE)
    rd = os.path.join(HERE, 'runs', unit)
    os.makedirs(rd, exist_ok=True)
    json.dump(meta, open(os.path.join(rd, '%s-first.usage.json' % label), 'w'), indent=1)
    if res is None:
        print('FAILED', unit, label, meta)
        return
    txt = strip_fence(res)
    try:
        ans = json.loads(txt)
    except ValueError:
        ans = {'answers': [], 'unparsed': res}
    json.dump(ans, open(os.path.join(rd, '%s-first.json' % label), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)
    print('done', unit, label, meta.get('output_tokens'), 'tokens out')


def fix_one(workdir, unit, label):
    import score as S
    rd = os.path.join(HERE, 'runs', unit)
    first_path = os.path.join(rd, '%s-first.json' % label)
    if not os.path.exists(first_path):
        return
    first = json.load(open(first_path, encoding='utf-8'))
    res = S.score_answer(unit, first)
    bad = [r for r in res if not r['valid'] and r['flags'] != ['missing_answer']]
    if not bad:
        return
    ids = {r['task_id'] for r in bad}
    prompt = open(os.path.join(HERE, 'prompts', unit + '.md'), encoding='utf-8').read()
    errs = '\n'.join('- `%s`: %s' % (r['task_id'], r['error']) for r in bad)
    body = (prompt + '\n\n## Your answer\n\n```\n' + json.dumps(first, ensure_ascii=False) + '\n```\n\n'
            '## Validator errors\n\nThese answers could not be applied or read back:\n\n' + errs + '\n\n'
            'Reply with the whole JSON object again, every task included, with these answers corrected.\n')
    json.dump({'message': FIX, 'errors': errs},
              open(os.path.join(rd, '%s-fix-message.json' % label), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)
    out, meta = call(workdir, '%s-fix' % unit, MODELS[label], body, FIX)
    json.dump(meta, open(os.path.join(rd, '%s-fix.usage.json' % label), 'w'), indent=1)
    if out is None:
        print('FAILED fix', unit, label, meta)
        return
    try:
        ans = json.loads(strip_fence(out))
    except ValueError:
        print('FAILED fix parse', unit, label)
        return
    fixed = {a['task_id']: a for a in first.get('answers', []) if isinstance(a, dict)}
    for a in ans.get('answers', []):
        if isinstance(a, dict) and a.get('task_id') in ids:
            fixed[a['task_id']] = a
    json.dump({'answers': list(fixed.values())},
              open(os.path.join(rd, '%s-fix.json' % label), 'w', encoding='utf-8'),
              ensure_ascii=False, indent=1)
    print('fixed', unit, label, sorted(ids))


def main():
    argv = sys.argv[1:]
    args = [a for a in argv if not a.startswith('--') and a != '']
    # drop the value of --models if it was passed as a separate token
    if '--models' in argv:
        i = argv.index('--models')
        if i + 1 < len(argv) and not argv[i + 1].startswith('--') and argv[i + 1] in args:
            args.remove(argv[i + 1])
    if not args:
        print('usage: python3 run.py <workdir> [unit_id …] [--models=sonnet55,haiku55,sonnet46] [--fix]',
              file=sys.stderr)
        sys.exit(2)
    workdir = args[0]
    labels = parse_models(argv)
    idx = json.load(open(os.path.join(HERE, 'data', 'index.json'), encoding='utf-8'))
    units = args[1:] or [x['unit_id'] for x in idx]
    if '--fix' in argv:
        with ThreadPoolExecutor(int(os.environ.get('JOBS', '6'))) as ex:
            list(ex.map(lambda ul: fix_one(workdir, *ul), [(u, l) for u in units for l in labels]))
        return
    jobs = [(u, l) for u in units for l in labels]
    with ThreadPoolExecutor(int(os.environ.get('JOBS', '6'))) as ex:
        list(ex.map(lambda j: run_one(workdir, *j), jobs))


if __name__ == '__main__':
    main()
