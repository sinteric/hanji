#!/usr/bin/env python3
"""Round 5 self-test. Run after build.py: python3 selftest.py (must end with SELFTEST PASSED)."""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import content as C  # noqa: E402
import deck as D  # noqa: E402
import score as S  # noqa: E402

fails = []


def gold(uid):
    with open(os.path.join(HERE, 'data', 'gold', uid + '.json'), encoding='utf-8') as f:
        return json.load(f)['answers']


def one(uid, answer):
    unit = S.load_unit(uid)
    t = [t for t in unit['tasks'] if t['task_id'] == answer['task_id']][0]
    return S.score_task(unit, t, answer)


def seed(uid):
    return S.load_unit(uid)['seed']


def expect(name, uid, answer, valid, landed, flag=None):
    r = one(uid, answer)
    ok = r['valid'] == valid and r['landed'] == landed and (flag is None or flag in r['flags'])
    if not ok:
        fails.append((name, r))
    return ok


# 1. gold answers: valid and landed, except the tasks a candidate cannot land by construction
with open(os.path.join(HERE, 'data', 'index.json'), encoding='utf-8') as f:
    index = json.load(f)
n_gold = n_unreach = 0
for u in index:
    res = S.score(u['unit_id'], {'answers': gold(u['unit_id'])})
    for r in res:
        n_gold += 1
        if r['task_id'] in u['unreachable']:
            n_unreach += 1
            if r['landed']:
                fails.append(('unreachable gold landed', r))
        elif not (r['valid'] and r['landed'] and not r['flags']):
            fails.append(('gold', u['unit_id'], r))
print('gold: %d answers, %d unreachable by construction (all C)' % (n_gold, n_unreach))
assert all(u['candidate'] == 'C' or not u['unreachable'] for u in index)

# 2. broken answers are caught
A1, B1, Ap2, C2, A2, B2, A3, B3 = 'r5-A-1', 'r5-B-1', 'r5-Ap-2', 'r5-C-2', 'r5-A-2', 'r5-B-2', 'r5-A-3', 'r5-B-3'
k = 0
broken = [
    ('keep summary changed', A1, {'task_id': 'deck1-e4', 'edits': [
        {'old': 'summary="image.png" box="576 396 72 36"', 'new': 'summary="logo.png" box="72 396 72 36"'}]},
     False, False, 'keep_altered'),
    ('invented id on a new text box', A1, {'task_id': 'deck1-e3', 'edits': [
        {'old': '분기별 매출\n<keep', 'new': '분기별 매출\n::shape id="s9" box="36 112 648 28"::\n단위: 억 원\n<keep'}]},
     False, False, 'unknown_id'),
    ('::shape:: marker in B', B1, {'task_id': 'deck1-e3', 'edits': [
        {'old': '분기별 매출\n<keep', 'new': '분기별 매출\n::shape box="36 112 648 28"::\n단위: 억 원\n<keep'}]},
     False, False, 'unknown_marker'),
    ('<shape> tag in A', A1, {'task_id': 'deck1-e3', 'edits': [
        {'old': '분기별 매출\n<keep', 'new': '분기별 매출\n<shape box="36 112 648 28">단위: 억 원</shape>\n<keep'}]},
     False, False, 'unknown_tag'),
    ('the table moved too', A1, {'task_id': 'deck1-e4', 'edits': [
        {'old': 'box="72 144 576 144"/>\n<keep id="k04k9" kind="picture" summary="image.png" box="576 396 72 36"',
         'new': 'box="80 144 576 144"/>\n<keep id="k04k9" kind="picture" summary="image.png" box="72 396 72 36"'}]},
     True, False, 'not_landed'),
    ('the repeated bullet on the wrong slide', B1, {'task_id': 'deck1-e5', 'edits': [
        {'old': '- 지방 13곳\n  - 부산', 'new': '- 지방 14곳\n  - 부산'}]}, True, False, 'not_landed'),
    ('ambiguous old', B1, {'task_id': 'deck1-e5', 'edits': [{'old': '지방 13곳', 'new': '지방 14곳'}]},
     False, False, 'edit_ambiguous'),
    ('a new deck with its own boxes', A1, {'task_id': 'deck1-w', 'text': D.render({
        'fm': [('type', 'presentation'), ('format', 'pptx'), ('schema', '1')], 'W': 9144000, 'H': 6858000,
        'layouts': C.LAYOUTS_43, 'slides': C.TASKS[1][9][3][0](C.deck_copy(C.DECK1))['slides']}, 'A').replace(
        '::title box="54 168 612 116"::', '::title box="54 150 612 116"::')}, True, False, 'not_landed'),
    ('points written in the percent candidate', Ap2, {'task_id': 'deck2-e1', 'edits': [
        {'old': 'name="TextBox 3" box="10 13.3', 'new': 'name="TextBox 3" box="157 13.3'}]}, True, False, 'not_landed'),
    ('grid: one cell is not 3 cm', C2, {'task_id': 'deck2-e1', 'edits': [
        {'old': 'name="TextBox 3" cells="B3:D3"', 'new': 'name="TextBox 3" cells="C3:E3"'}]}, True, False,
     'not_landed'),
    ('read: a wrong object', A2, {'task_id': 'deck2-q1', 'text': 'ANSWER: s10, kmtnf'}, True, False, 'wrong_answer'),
    ('read: no ANSWER:', A2, {'task_id': 'deck2-q1', 'text': 's10'}, False, False, 'answer_form'),
    ('read: an unknown object', A3, {'task_id': 'deck3-q1', 'text': 'ANSWER: title, s99'}, False, False,
     'unknown_object'),
    ('formatting done anyway', A2, {'task_id': 'deck2-e7', 'edits': [
        {'old': 'Learning PPTX', 'new': '**Learning PPTX**'}]}, True, False, 'should_refuse'),
    ('refusing a doable task', A2, {'task_id': 'deck2-e1', 'text': 'REFUSE: no.', 'edits': []}, True, False,
     'wrong_refusal'),
    ('refusal with edits', A2, {'task_id': 'deck2-e7', 'text': 'REFUSE: formatting.', 'edits': [
        {'old': 'Learning PPTX', 'new': 'x'}]}, False, False, 'bad_refusal'),
    ('group box and a child both changed, inconsistently', B2, {'task_id': 'deck2-e4', 'edits': [
        {'old': 'box="120 108 258 152">', 'new': 'box="100 108 258 152">'},
        {'old': 'name="Right Arrow 3" box="144 222 77 38"', 'new': 'name="Right Arrow 3" box="120 222 77 38"'}]},
     False, False, 'group_box_conflict'),
    ('a box of three numbers', A2, {'task_id': 'deck2-e2', 'edits': [
        {'old': 'box="402 78 144 132"', 'new': 'box="402 78 72"'}]}, False, False, 'bad_box'),
    ('an unknown slot', B3, {'task_id': 'deck3-e6', 'edits': [
        {'old': '가격은 부가세 포함\n', 'new': '가격은 부가세 포함\n\n---\n\nlayout: Comparison\n::title::\n출시 채널 비교\n'
         '::left::\n온라인\n'}]}, False, False, 'unknown_slot'),
    ('logo on slide 1 resized instead of slide 2', A3, {'task_id': 'deck3-e2', 'edits': [
        {'old': 'id="k3ftw" kind="picture" summary="logo.png" box="864 18 78 39"',
         'new': 'id="k3ftw" kind="picture" summary="logo.png" box="825 18 117 59"'}]}, True, False, 'not_landed'),
    ('text box overlapping the chart', A3, {'task_id': 'deck3-e3', 'edits': [
        {'old': '예상 매출\n<keep', 'new': '예상 매출\n::shape box="66 133 828 40"::\n단위: 억 원 (2027년 계획)\n<keep'}]},
     True, False, 'not_landed'),
]
for name, uid, ans, v, l, flag in broken:
    k += expect(name, uid, ans, v, l, flag)
print('broken: %d/%d caught' % (k, len(broken)))

# 3. correct answers written differently from the gold
alts = [
    ('decimal points', A2, {'task_id': 'deck2-e1', 'edits': [
        {'old': 'name="TextBox 3" box="72 72', 'new': 'name="TextBox 3" box="157.04 72'}]}),
    ('names instead of ids', A2, {'task_id': 'deck2-q1', 'text': 'ANSWER: Elbow Connector 9'}),
    ('slot marker spelled', A3, {'task_id': 'deck3-q1', 'text': 'ANSWER: ::title::, body (the bullets)'}),
    ('heading as a bullet', B1, {'task_id': 'deck1-e6', 'edits': [
        {'old': '::right::\n- 지방 13곳\n  - 부산 5곳\n<shape id="s5" name="출처" box="36 475 288 29">출처: 내부 집계</shape>\n',
         'new': '::right::\n- 지방 13곳\n  - 부산 5곳\n<shape id="s5" name="출처" box="36 475 288 29">출처: 내부 집계</shape>\n'
                '\n---\n\nlayout: Comparison\n::title::\n권역별 전략\n::body::\n- 수도권\n::body2::\n- 신규 지점 2곳\n'
                '- 온라인 판촉 강화\n::body3::\n- 지방\n::body4::\n- 부산 거점 확대\n- 대리점 교육\n'}]}),
    ('new slide with the layout boxes written', A1, {'task_id': 'deck1-e6', 'edits': [
        {'old': '출처: 내부 집계\n', 'new': '출처: 내부 집계\n\n---\n\nlayout: Comparison\n::title box="36 22 648 90"::\n'
         '권역별 전략\n::body box="36 121 318 50"::\n수도권\n::body2 box="36 171 318 311"::\n- 신규 지점 2곳\n'
         '- 온라인 판촉 강화\n::body3 box="366 121 318 50"::\n지방\n::body4 box="366 171 318 311"::\n'
         '- 부산 거점 확대\n- 대리점 교육\n'}]}),
    ('group child moved in percent', Ap2, {'task_id': 'deck2-e4', 'edits': [
        {'old': 'name="Right Arrow 3" box="20 41.1', 'new': 'name="Right Arrow 3" box="16.7 41.1'}]}),
    ('text box written after the table', B3, {'task_id': 'deck3-e3', 'edits': [
        {'old': 'box="630 180 180 29">1분기 120억</shape>',
         'new': 'box="630 180 180 29">1분기 120억</shape>\n<shape box="66 134 828 28">단위: 억 원 (2027년 계획)</shape>'}]}),
    ('group moved by its box alone', 'r5x-B-2', {'task_id': 'deck2-x4', 'edits': [
        {'old': 'name="Group 4" box="120 108', 'new': 'name="Group 4" box="176.7 108'}]}),
    ('group centred by its box alone, percent', 'r5x-Ap-3', {'task_id': 'deck3-x4', 'edits': [
        {'old': 'name="타임라인" box="10 33.3', 'new': 'name="타임라인" box="12.5 33.3'}]}),
    ('slot box added in B', 'r5x-B-1', {'task_id': 'deck1-x2', 'edits': [
        {'old': '지역별 현황\n::left::', 'new': '지역별 현황\n::left box="36 126 261.3 356"::'}]}),
    ('z-order by moving the table line', 'r5x-A-1', {'task_id': 'deck1-x3', 'edits': [
        {'old': '<keep id="kb2br" kind="table" summary="Table 2: 분기 매출 증감 1분기 1,120 +3% 2분기 1,180 +5% 3분기 1,204 +12%" '
                'box="72 144 576 144"/>\n<keep id="k04k9" kind="picture" summary="image.png" box="576 396 72 36"/>',
         'new': '<keep id="k04k9" kind="picture" summary="image.png" box="576 396 72 36"/>\n<keep id="kb2br" kind="table" '
                'summary="Table 2: 분기 매출 증감 1분기 1,120 +3% 2분기 1,180 +5% 3분기 1,204 +12%" box="72 144 576 144"/>'}]}),
]
k = 0
for name, uid, ans in alts:
    k += expect(name, uid, ans, True, True)
print('alternatives: %d/%d pass' % (k, len(alts)))

# 4. model laws on the text: GetPut (checked in build.py) and a group move keeping every child's offset
d = C.deck_copy(C.DECK2)
for cand in D.CANDIDATES:
    t = D.render(d, cand)
    u = D.units_for(cand, d['W'], d['H'])
    old = D.show_box(u, D.obj_box(d['slides'][2]['objs'][0]))
    new = 'D3:G6' if cand == 'C' else ' '.join(D.fnum(float(x) + 10) if i == 0 else x
                                               for i, x in enumerate(old.split()))
    t2 = t.replace('name="Group 4" %s="%s"' % (u.attr, old), 'name="Group 4" %s="%s"' % (u.attr, new))
    assert t2 != t, cand
    back, errs, _ = D.parse(t2, cand, S.ctx_for(2))
    assert not errs, (cand, errs)
    g0, g1 = d['slides'][2]['objs'][0], back['slides'][2]['objs'][0]
    dx = g1['children'][0]['box'][0] - g0['children'][0]['box'][0]
    for c0, c1 in zip(g0['children'], g1['children']):
        if abs((c1['box'][0] - c0['box'][0]) - dx) > 1 or c1['box'][2] != c0['box'][2]:
            fails.append(('group move', cand, c0, c1))
    # PutGet holds on the canonical text: a group move is written back with its objects moved (the write returns
    # that text), and the canonical text reads back to itself
    canon = D.render(back, cand)
    again, errs, _ = D.parse(canon, cand, D.Ctx(d['W'], d['H'], d['layouts'], seed=back))
    if errs or D.render(again, cand) != canon or D.canon_deck(again) != D.canon_deck(back):
        fails.append(('putget', cand))
print('group move and PutGet: ok' if not [f for f in fails if f[0] in ('group move', 'putget')] else 'group move FAILED')

# 5. the command line
out = subprocess.run([sys.executable, os.path.join(HERE, 'score.py'), 'r5-A-1',
                      os.path.join(HERE, 'data', 'gold', 'r5-A-1.json')], capture_output=True, text=True)
assert out.returncode == 0 and all(r['landed'] for r in json.loads(out.stdout)['results'])
print('cli: ok')

if fails:
    for f in fails:
        print('FAIL', f)
    sys.exit(1)
print('SELFTEST PASSED')
