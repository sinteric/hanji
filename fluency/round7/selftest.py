#!/usr/bin/env python3
"""Round 7 self-test: gold answers land, broken answers are caught, alternative correct answers pass.
Run: python3 selftest.py (after build.py). Ends with SELFTEST PASSED."""
import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import score as S  # noqa: E402

fails = []


def gold(uid):
    return json.load(open(os.path.join(HERE, 'data', 'gold', uid + '.json'), encoding='utf-8'))


def seed(uid):
    return open(os.path.join(HERE, 'seeds', uid + '.txt'), encoding='utf-8').read()


def one(uid, tid, answer):
    res = S.score_answer(uid, {'answers': [dict(answer, task_id=tid)]})
    return next(r for r in res if r['task_id'] == tid)


def expect(name, uid, tid, answer, valid, landed, flag=None):
    r = one(uid, tid, answer)
    ok = r['valid'] == valid and r['landed'] == landed and (flag is None or flag in r['flags'])
    if not ok:
        fails.append((name, uid, tid, r))
    return ok


def line_with(text, *needles):
    for ln in text.split('\n'):
        if all(n in ln for n in needles):
            return ln
    raise KeyError(needles)


def edit(old, new):
    return {'edits': [{'old': old, 'new': new}]}


def main():
    idx = json.load(open(os.path.join(HERE, 'data', 'index.json'), encoding='utf-8'))
    n_gold = 0
    for u in idx:
        uid = u['unit_id']
        for r in S.score_answer(uid, gold(uid)):
            n_gold += 1
            if not (r['valid'] and r['landed'] and not r['flags']):
                fails.append(('gold', uid, r['task_id'], r['error'], r['detail'], r['flags']))
    print('gold answers:', n_gold)

    # ---- broken: the other candidate's tags, invented attrs, wrong span, ambiguous old
    A1, B1 = 'r7-breaks-A-1', 'r7-breaks-B-1'
    ta, tb = seed(A1), seed(B1)
    # A: using B's <br type="page"/>
    ln = line_with(ta, '수도권 매출은', '첨부 1')
    expect('A rejects br type=page', A1, 'report1-e3',
           edit(ln, ln.replace('<pagebreak/>', '<br type="page"/>')), False, False, 'invented_attr')
    # B: using A's <columnbreak/>
    ln = line_with(tb, '인천 0곳')
    expect('B rejects columnbreak', B1, 'report1-e2',
           edit(ln, ln.replace('<br type="column"/>', '<columnbreak/>')), False, False, 'unknown_tag')
    # invented type
    ln = line_with(tb, '인천 0곳')
    expect('B rejects type=section', B1, 'report1-e2',
           edit(ln, ln.replace('type="column"', 'type="section"')), False, False, 'invented_attr')
    # wrong paragraph (the other +12%)
    ln = line_with(ta, '영업본부는')
    expect('wrong +12% paragraph', A1, 'report1-e3',
           edit(ln, ln.replace('전년 대비 +12%', '전년 대비 +11%')), True, False, 'not_landed')
    # ambiguous old
    expect('ambiguous old', A1, 'report1-e3',
           edit('전년 대비 +12%', '전년 대비 +13%'), False, False, 'edit_ambiguous')
    # refuse a doable task
    expect('wrong refusal', A1, 'report1-e1',
           {'text': 'REFUSE: I cannot find the paragraph.', 'edits': []}, True, False, 'wrong_refusal')
    # not refusing the refusal task
    expect('not refused', A1, 'report1-r',
           edit('<pagebreak/>\n\n# 부록', '<pagebreak/>\n\n<sectionbreak/>\n\n# 부록'), False, False)

    SA, SB = 'r7-shapes-A-1', 'r7-shapes-B-1'
    sa, sb = seed(SA), seed(SB)
    tip = '아래에 입력하면 표가 늘어납니다. 마지막 행에서 Enter.'
    # A: text= attribute (B's form)
    ln = line_with(sa, '입력 안내')
    expect('A rejects text attr', SA, 'kpi1-e2',
           edit(ln, '<shape name="입력 안내" anchor="B28:D30" text="%s"/>' % tip),
           False, False)
    # B: element body (A's form)
    ln = line_with(sb, '입력 안내')
    expect('B rejects element body', SB, 'kpi1-e2',
           edit(ln, '<shape name="입력 안내" anchor="B28:D30">%s</shape>' % tip),
           False, False)
    # move the wrong shape
    ln = line_with(sa, '매출 배지')
    expect('moved the badge not the tip', SA, 'kpi1-e2',
           edit(ln, ln.replace('F2:G3', 'B28:D30')), True, False, 'not_landed')
    # invented attr
    ln = line_with(sa, '입력 안내')
    expect('invented rot', SA, 'kpi1-e2',
           edit(ln, ln.replace('anchor="B25:D27"', 'anchor="B28:D30" rot="15"')), False, False, 'invented_attr')

    DA, DB = 'r7-databar-A-1', 'r7-databar-B-1'
    da, db = seed(DA), seed(DB)
    # A: min-type (B's form)
    ln = line_with(da, 'type="dataBar"', '월간수입합계')
    expect('A rejects min-type', DA, 'budget1-e3',
           edit(ln, ln.replace('min="num:0" max="num:월간수입합계"',
                               'min-type="min" max-type="max"')), False, False, 'invented_attr')
    # B: compact min="min"
    ln = line_with(db, 'type="dataBar"', '월간수입합계')
    expect('B rejects compact min', DB, 'budget1-e3',
           edit(ln, ln.replace('min-type="num" min="0" max-type="num" max="월간수입합계"',
                               'min="min" max="max"')), False, False)
    # wrong bar (G not E)
    ln = line_with(da, 'G4:G16')
    expect('wrong data bar', DA, 'budget1-e2',
           edit(ln, ln.replace('#FFC7CE', '#63BE7B')), True, False, 'not_landed')
    # color scale
    ln = line_with(da, 'type="dataBar"', '월간수입합계')
    expect('colorScale refused as invalid', DA, 'budget1-e2',
           edit(ln, ln.replace('type="dataBar"', 'type="colorScale"')), False, False)

    # ---- alternatives that still land
    ln = line_with(ta, '수도권 매출은', '첨부 1')
    expect('wider old still lands', A1, 'report1-e3',
           edit('입니다.<pagebreak/>첨부', '입니다. 첨부'), True, True)
    ln = line_with(sa, '매출 배지')
    expect('retext with more context', SA, 'kpi1-e3',
           edit(ln, ln.replace('매출 +12%', '매출 +15%')), True, True)
    ln = line_with(da, 'E4:E16')
    expect('range edit as written', DA, 'budget1-e1',
           edit('range="E4:E16"', 'range="E4:E18"'), True, True)

    # ---- CLI
    gpath = os.path.join(HERE, 'data', 'gold', 'r7-breaks-A-1.json')
    p = subprocess.run([sys.executable, os.path.join(HERE, 'score.py'), 'r7-breaks-A-1', gpath],
                       capture_output=True, text=True)
    if p.returncode != 0:
        fails.append(('cli', p.stderr))
    else:
        out = json.loads(p.stdout)
        if not all(r['landed'] for r in out['results']):
            fails.append(('cli landed', out))

    # guides same length, never mention each other
    for dec in ('breaks', 'shapes', 'databar'):
        a = open(os.path.join(HERE, 'guides', '%s-A.md' % dec), encoding='utf-8').read()
        b = open(os.path.join(HERE, 'guides', '%s-B.md' % dec), encoding='utf-8').read()
        if len(a) != len(b):
            fails.append(('guide length', dec, len(a), len(b)))
        if 'candidate A' in b.lower() or 'candidate B' in a.lower():
            fails.append(('guides mention each other', dec))

    # rebuild is byte-identical
    def tree_hash():
        h = hashlib.sha256()
        for rel in ('data', 'seeds', 'prompts', 'guides'):
            root = os.path.join(HERE, rel)
            for dirpath, _, files in os.walk(root):
                for f in sorted(files):
                    p = os.path.join(dirpath, f)
                    h.update(os.path.relpath(p, HERE).encode())
                    h.update(open(p, 'rb').read())
        return h.digest()

    before = tree_hash()
    p = subprocess.run([sys.executable, os.path.join(HERE, 'build.py')], cwd=HERE,
                       capture_output=True, text=True)
    if p.returncode != 0:
        fails.append(('rebuild', p.stderr[-2000:]))
    elif tree_hash() != before:
        fails.append(('rebuild not deterministic',))

    print('fails:', len(fails))
    for f in fails:
        print(' ', f)
    if fails:
        sys.exit(1)
    print('SELFTEST PASSED')


if __name__ == '__main__':
    main()
