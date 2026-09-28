#!/usr/bin/env python3
"""Round 4 self-test: gold answers must all be valid+landed with no flags; broken answers must be caught with the
expected flag; alternative correct answers (not equal to gold) must pass. Run: python3 selftest.py"""
import copy
import json
import os
import subprocess
import sys
import tempfile

import score

HERE = os.path.dirname(os.path.abspath(__file__))
DECISIONS = {'readview': 'AB', 'writeshape': 'ABC'}
UNIT_IDS = ['r4-%s-%s-%d' % (d, c, r) for d, cs in DECISIONS.items() for c in cs for r in (1, 2, 3)]
MIN_BROKEN, MIN_ALT = 4, 2


def gold(uid):
    with open(os.path.join(HERE, 'data', 'gold', uid + '.json'), encoding='utf-8') as f:
        return json.load(f)


def G(uid, tid):
    return copy.deepcopy({x['task_id']: x for x in gold(uid)['answers']}[tid])


def view(uid):
    return score.load_unit(uid)['view']


def one(uid, tid, answer):
    answer = dict(answer, task_id=tid)
    res = score.score(uid, {'answers': [answer]})['results']
    return [r for r in res if r['task_id'] == tid][0]


def T(s):
    return {'text': s}


def OPS(*ops):
    return {'text': list(ops)}


def E(uid, *pairs):
    """Edits against the view; asserts each old occurs in it (a guard against typos in this file)."""
    v = view(uid)
    for o, _ in pairs:
        assert o in v, (uid, o)
    return {'edits': [{'old': o, 'new': n} for o, n in pairs]}


def REFUSE(reason='this cannot be done with this workbook.'):
    return {'text': 'REFUSE: ' + reason, 'edits': []}


def line_of(uid, start):
    ls = [x for x in view(uid).split('\n') if x.startswith(start)]
    assert len(ls) == 1, (uid, start, len(ls))
    return ls[0]


def last_row_line(uid, tname):
    ls = view(uid).split('\n')
    i = ls.index('<data table="%s">' % tname)
    j = ls.index('</data>', i)
    return ls[j - 1]


def cell(line, k, v):
    """line with its k-th cell (0 = row label) replaced by v."""
    cells = line.strip()[1:-1].split('|')
    cells[k] = ' %s ' % v
    return '|' + '|'.join(cells) + '|'


RA1, RA2, RA3, RB1, RB2, RB3 = ('r4-readview-%s-%d' % (c, r) for c in 'AB' for r in (1, 2, 3))
WA1, WA2, WA3, WB1, WB2, WB3, WC1, WC2, WC3 = ('r4-writeshape-%s-%d' % (c, r) for c in 'ABC' for r in (1, 2, 3))

L73 = '| 73 | 2026-05 | 서초 | 가전 | 18,240,000 | 12,630,000 | 5,610,000 |'
L90 = '| 90 | 2026-06 | 서초 | 가전 | 18,240,000 | 13,360,000 | 4,880,000 |'
KIM_D = '| 20 | 068328 | 김서연 | 대구 | 개인정보 보호 | 2026-07-14 | 2 | 84 |'
KIM_B = '| 19 | 064477 | 김서연 | 부산 | 개인정보 보호 | 2026-07-14 | 2 | 91 |'
B39 = lambda: line_of(WB3, '| 39 | 0421 | 개발 | 복리후생비 |')
B2 = lambda: line_of(WB3, '| 2 | 0411 | 경영지원 | 급여 |')
R2 = '| 2 | 034884 | 정은우 | 서울본점 | 영업 | 대리 | 2023-07-17 | 010-5924-7131 | 0371 |'
KPI_NEW = [('강남', 50000000, 48720000), ('서초', 41000000, 42180000), ('송파', 37000000, 35930000),
           ('분당', 35000000, 36110000), ('일산', 29000000, 27560000)]
LOOP_1008 = ('t = table("Training")\nfor r in table("Roster").rows:\n    if r["이름"]:\n        for c in range(7):\n'
             '            t.append({"사번": "%06d" % r["사번"], "이름": r["이름"], "지점": r["지점"], '
             '"과정": "2027 과정 " + str(c + 1)})\n')

# (unit, task, answer factory, expected flag) -- each must be caught (not valid+landed) and carry the flag
BROKEN = [
    # ---------------- readview
    (RA1, 'sales1-q1', lambda: T('ANSWER: 12,930,000'), 'wrong_answer'),
    (RA1, 'sales1-q2', lambda: T('16'), 'answer_form'),
    (RB1, 'sales1-q4', lambda: T('ANSWER: 3609'), 'id_lost_zeros'),
    (RA2, 'roster2-q4', lambda: T('ANSWER: 68328'), 'id_lost_zeros'),
    (RB2, 'roster2-q1', lambda: T('ANSWER: 84'), 'wrong_answer'),            # the other 김서연
    (RA3, 'budget3-q3', lambda: T('ANSWER: 65'), 'wrong_answer'),
    (RB3, 'budget3-q5', lambda: T('ANSWER: G19:G35'), 'wrong_answer'),       # includes the 급여 row of 경영지원's 소계
    (RB3, 'budget3-q2', lambda: REFUSE('the workbook does not say.'), 'wrong_refusal'),
    # ---------------- writeshape A: range operations
    (WA1, 'sales1-e1', lambda: OPS({'op': 'set', 'range': '매출!D90', 'values': [[18420000]]}), 'wrong_cell'),
    (WA1, 'sales1-e3', lambda: OPS({'op': 'fill_formula', 'table': 'Sales', 'column': '이익', 'formula': '=D2-E2'}),
     'a1_formula'),
    (WA1, 'sales1-e5', lambda: OPS({'op': 'add_column', 'table': 'KPI', 'column': {
        'name': '환율', 'type': 'number', 'format': '#,##0.0',
        'formula': '=WEBSERVICE("https://fx.example.com/rate?pair=USDKRW")'}}), 'fetch_function'),
    (WA2, 'roster2-e4', lambda: OPS({'op': 'set_type', 'table': 'Roster', 'column': '사번', 'type': 'text'},
                                    {'op': 'set', 'range': '명부!A2', 'values': [['34884']]}), 'id_lost_zeros'),
    (WA2, 'roster2-e5', lambda: OPS({'op': 'append_rows', 'table': 'Training',
                                     'rows': [{'이름': '신규%d' % i} for i in range(1008)]}), 'bulk_typed'),
    (WA3, 'budget3-w', lambda: OPS({'op': 'add_table', 'sheet': '인건비', 'name': 'Hiring', 'anchor': 'H1', 'columns': [
        {'name': '부서', 'type': 'text', 'format': '@'}, {'name': '충원 인원', 'type': 'number', 'format': '0'},
        {'name': '1인당 연봉', 'type': 'number', 'format': '#,##0'},
        {'name': '추가 인건비', 'type': 'number', 'format': '#,##0', 'formula': '=I2*J2'}], 'rows': [
        {'부서': '개발', '충원 인원': 3, '1인당 연봉': 52000000}, {'부서': '품질', '충원 인원': 2, '1인당 연봉': 46000000},
        {'부서': '영업', '충원 인원': 2, '1인당 연봉': 48500000}]}), 'a1_formula'),
    (WA3, 'budget3-e2', lambda: OPS({'op': 'append_rows', 'table': 'Spend', 'rows': [
        {'월': '2027-04', '계정코드': 441, '항목': '여비교통비', '부서': '영업', '집행액': 3420000}]}), 'type_mismatch'),
    (WA3, 'budget3-e4', lambda: REFUSE('a type cannot be changed.'), 'wrong_refusal'),
    # ---------------- writeshape B: editing the text
    (WB1, 'sales1-e1', lambda: E(WB1, (L90, L90.replace('18,240,000', '18,420,000'))), 'wrong_cell'),
    (WB1, 'sales1-e2', lambda: E(WB1, (last_row_line(WB1, 'KPI'), last_row_line(WB1, 'KPI') + ''.join(
        '\n| %d | 2026-09 | %s | %s | %s |  |' % (42 + i, b, '{:,}'.format(g), '{:,}'.format(a))
        for i, (b, g, a) in enumerate(KPI_NEW)))), 'row_labels_edited'),
    (WB1, 'sales1-e3', lambda: E(WB1, *[(line_of(WB1, p), cell(line_of(WB1, p), 6, '=D%d-E%d' % (r, r)))
                                        for r, p in ((2, '| 2 | 2026-01 | 강남 | 가전 |'),
                                                     (3, '| 3 | 2026-01 | 강남 | 모바일 |'))]), 'a1_formula'),
    (WB2, 'roster2-e1', lambda: {'edits': [{'old': KIM_D.replace('| 2 | 84 |', '| 2 | 84|'),
                                            'new': KIM_D.replace('84', '88')}]}, 'edit_no_match'),
    (WB2, 'roster2-e4', lambda: E(WB2, ('| 사번 | number | 000000 |  |', '| 사번 | text | @ |  |'),
                                  (R2, R2.replace('034884', '34884'))), 'id_lost_zeros'),
    (WB2, 'roster2-w', lambda: {'edits': [{'old': '</data>\n</sheet>', 'new': '</data>\n</sheet>'}]}, 'edit_ambiguous'),
    (WB3, 'budget3-e1', lambda: E(WB3, (B39(), cell(B39(), 0, '40'))), 'row_labels_edited'),
    (WB3, 'budget3-e3', lambda: E(WB3, (B2(), cell(B2(), 8, '=E2+F2+G2+H2'))), 'a1_formula'),
    # ---------------- writeshape C: code
    (WC1, 'sales1-e3', lambda: T('import os\ntable("Sales").fill_formula("이익", "=[@매출]-[@원가]")\n'),
     'sandbox_violation'),
    (WC1, 'sales1-e1', lambda: T('while True:\n    pass\n'), 'sandbox_violation'),
    (WC1, 'sales1-e4', lambda: T('t = table("Staff")\nvals = [r["사번"] for r in t.rows]\nt.set_type("사번", "text")\n'
                                 'for r, v in zip(t.rows, vals):\n    r["사번"] = str(v)\n'), 'id_lost_zeros'),
    (WC2, 'roster2-e3', lambda: T('open("/etc/passwd")\n'), 'sandbox_violation'),
    (WC2, 'roster2-e1', lambda: T('for r in table("Training").rows:\n    if r["이름"] == "김서연" and r["지점"] == "부산":\n'
                                  '        r["점수"] = 88\n'), 'wrong_cell'),
    (WC2, 'roster2-e5', lambda: T(LOOP_1008), 'bulk_typed'),
    (WC3, 'budget3-e1', lambda: T('x = ().__class__\n'), 'sandbox_violation'),
    (WC3, 'budget3-e3', lambda: T('table("Budget").fill_formula("합계", "=D2+E2+F2+G2")\n'), 'a1_formula'),
    (WC3, 'budget3-w', lambda: REFUSE('tables cannot be added.'), 'wrong_refusal'),
    (WC1, 'sales1-e5', lambda: T('table("KPI").add_column("환율", "number", "#,##0.0", '
                                 '\'=WEBSERVICE("https://fx.example.com/rate?pair=USDKRW")\')\n'), 'fetch_function'),
    (WC3, 'budget3-e5', lambda: T('t = table("Budget")\nt.add_column("강조", "text")\nfor r in t.rows:\n'
                                  '    if (r["합계"] or 0) > 100000000:\n        r["강조"] = "빨강"\n'), 'should_refuse'),
    (WB2, 'roster2-e5', lambda: E(WB2, (last_row_line(WB2, 'Training'), last_row_line(WB2, 'Training') + ''.join(
        '\n|  |  | 신규%d |  | 2027 과정 |  |  |  |' % i for i in range(60)))), 'bulk_typed'),
]

HIRING_ROWS = [{'부서': '개발', '충원 인원': 3, '1인당 연봉': 52000000}, {'부서': '품질', '충원 인원': 2, '1인당 연봉': 46000000},
               {'부서': '영업', '충원 인원': 2, '1인당 연봉': 48500000}]


def returns_ops():
    g = G(WA1, 'sales1-w')
    op = g['text'][0]
    op['columns'][3]['formula'] = '=[@반품액]/SUMIFS(Sales[매출], Sales[지점], [@지점], Sales[월], [@월])'
    for r in op['rows']:
        r['월'] = r['월'][:7]
    return {'text': json.dumps(g['text'], ensure_ascii=False)}          # the list given as a JSON string


def kpi_rows_plain(uid):
    last = last_row_line(uid, 'KPI')
    return E(uid, (last + '\n</data>', last + ''.join('\n|  | 2026-09-01 | %s | %d | %d | 99.9%% |' % (b, g, a)
                                                        for b, g, a in KPI_NEW) + '\n</data>'))


def returns_edits(uid):
    g = G(uid, 'sales1-w')
    e = copy.deepcopy(g['edits'][0])
    e['new'] = e['new'].replace('range="H1:K5"', 'range="H1"')
    return {'edits': [e]}


# (unit, task, answer factory) -- written differently from the gold, must be valid and landed
ALTERNATIVES = [
    # ---------------- readview
    (RA1, 'sales1-q1', lambda: T('ANSWER: 12390000')),
    (RB1, 'sales1-q5', lambda: T('answer:   E36:E50 ')),
    (RA2, 'roster2-q4', lambda: T('ANSWER:068328')),
    (RB3, 'budget3-q1', lambda: T('ANSWER: ' + G(RB3, 'budget3-q1')['accept'][0].replace(',', ''))),
    # ---------------- writeshape A
    (WA1, 'sales1-e1', lambda: OPS({'op': 'set', 'range': '매출!D73:E73', 'values': [[18420000, 12630000]]})),
    (WA1, 'sales1-e3', lambda: OPS({'op': 'fill_formula', 'table': 'Sales', 'column': '이익',
                                    'formula': '=-[@원가]+[@매출]'})),
    (WA1, 'sales1-w', returns_ops),
    (WA2, 'roster2-e4', lambda: OPS({'op': 'set_type', 'table': 'Roster', 'column': '사번', 'type': 'text'})),
    (WA3, 'budget3-e3', lambda: OPS({'op': 'fill_formula', 'table': 'Budget', 'column': '합계',
                                     'formula': '=[@[1분기]]+[@[2분기]]+[@[3분기]]+[@[4분기]]'})),
    (WA3, 'budget3-e5', lambda: REFUSE('colours cannot be set; only types and number formats.')),
    # ---------------- writeshape B
    (WB1, 'sales1-e3', lambda: E(WB1, ('| 이익 | number | #,##0 |  |', '| 이익 | number | #,##0 | =[@매출]-[@원가] |'))),
    (WB1, 'sales1-e2', lambda: kpi_rows_plain(WB1)),
    (WB1, 'sales1-w', lambda: returns_edits(WB1)),
    (WB2, 'roster2-e1', lambda: E(WB2, ('김서연 | 대구 | 개인정보 보호 | 2026-07-14 | 2 | 84 |',
                                        '김서연 | 대구 | 개인정보 보호 | 2026-07-14 | 2 | 88 |'))),
    (WB3, 'budget3-e4', lambda: E(WB3, ('| 계정코드 | number | 0000 |  |', '| 계정코드 | text |  |  |'))),
    # ---------------- writeshape C
    (WC1, 'sales1-e1', lambda: T('for r in table("Sales").rows:\n    if r["월"] == "2026-05-01" and r["지점"] == "서초" '
                                 'and r["제품군"] == "가전":\n        r["매출"] = 18420000\n')),
    (WC1, 'sales1-e4', lambda: T('table("Staff").set_type("사번", "text", "@")')),
    (WC2, 'roster2-w', lambda: T('wb["요약"].add_table("Overtime", "G1", [\n'
                                 '    {"name": "지점", "type": "text"},\n'
                                 '    {"name": "월", "type": "date", "format": "yyyy-mm"},\n'
                                 '    {"name": "초과근무", "type": "number", "format": "#,##0"},\n'
                                 '    {"name": "1인당", "type": "number", "format": "0.0",\n'
                                 '     "formula": "=[@초과근무]/COUNTIFS(Roster[[지점]],[@[지점]])"}],\n'
                                 '    [{"지점": b, "월": "2026-08", "초과근무": h} for b, h in '
                                 '[("서울본점", 412), ("부산", 288), ("대구", 305), ("광주", 196)]])\n')),
    (WC3, 'budget3-e3', lambda: T('t = table("Budget")\nt.fill_formula("합계", "=SUM([@[1분기]:[4분기]])")\n')),
    (WC3, 'budget3-e2', lambda: T('t = wb["집행"].table("Spend")\nrows = [\n'
                                  ' ("0441", "여비교통비", "영업", 3420000, "법인카드"),\n'
                                  ' ("0431", "교육훈련비", "개발", 5800000, "세금계산서"),\n'
                                  ' ("0521", "수선비", "생산", 7150000, "세금계산서"),\n'
                                  ' ("0461", "소모품비", "품질", 640000, None),\n'
                                  ' ("0551", "회의비", "경영지원", 910000, "법인카드")]\n'
                                  'for c, i, d, v, n in rows:\n'
                                  '    t.append({"월": "2027-04", "계정코드": c, "항목": i, "부서": d, "집행액": v, '
                                  '"비고": n})\n')),
]


def main():
    fails = 0
    n_gold = 0
    for uid in UNIT_IDS:
        res = score.score(uid, gold(uid))['results']
        for r in res:
            n_gold += 1
            if not (r['valid'] and r['landed']) or r['flags']:
                fails += 1
                print('GOLD FAIL', uid, r)
    print('gold: %d tasks checked, all valid and landed' % n_gold if not fails else 'gold: FAILURES')
    per_dec = {}
    for uid, tid, make, flag in BROKEN:
        r = one(uid, tid, make())
        caught = not (r['valid'] and r['landed']) and flag in r['flags']
        per_dec.setdefault(uid.split('-')[1], []).append(caught)
        if not caught:
            fails += 1
            print('BROKEN NOT CAUGHT', uid, tid, flag, r)
        else:
            print('caught  %-19s %-11s %-18s %s' % (uid, tid, flag, (r['error'] or r['detail']).split('\n')[0][:80]))
    for d in DECISIONS:
        v = per_dec.get(d, [])
        print('broken %-10s %d/%d caught' % (d, sum(v), len(v)))
        if sum(v) < MIN_BROKEN:
            fails += 1
    alt_dec = {}
    for uid, tid, make in ALTERNATIVES:
        ans = make()
        r = one(uid, tid, ans)
        g = G(uid, tid)
        same = ans.get('edits') == g.get('edits') and ans.get('text') == g.get('text')
        ok = r['valid'] and r['landed'] and not same and not r['flags']
        alt_dec.setdefault(uid.split('-')[1], []).append(ok)
        if not ok:
            fails += 1
            print('ALTERNATIVE FAIL', uid, tid, 'same as gold' if same else r)
        else:
            print('passes  %-19s %-11s' % (uid, tid))
    for d in DECISIONS:
        v = alt_dec.get(d, [])
        print('alternatives %-10s %d/%d pass' % (d, sum(v), len(v)))
        if sum(v) < MIN_ALT:
            fails += 1
    # CLI round trip
    for uid in ('r4-writeshape-B-2', 'r4-readview-B-3'):
        with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False, encoding='utf-8') as f:
            json.dump(gold(uid), f, ensure_ascii=False)
        out = subprocess.run([sys.executable, os.path.join(HERE, 'score.py'), uid, f.name],
                             capture_output=True, text=True, check=True).stdout
        os.unlink(f.name)
        assert all(r['valid'] and r['landed'] for r in json.loads(out)['results'])
    print('cli: ok')
    print('SELFTEST %s' % ('PASSED' if fails == 0 else 'FAILED (%d)' % fails))
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
