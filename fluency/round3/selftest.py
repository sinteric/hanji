#!/usr/bin/env python3
"""Round 3 self-test: gold answers must all be valid+landed with no flags; broken answers must be caught with the
expected flag; alternative correct answers (not equal to gold) must pass. Run: python3 selftest.py"""
import copy
import json
import os
import subprocess
import sys
import tempfile

import score

HERE = os.path.dirname(os.path.abspath(__file__))
DECISIONS = ('cellpara', 'emptypara')
UNIT_IDS = ['r3-%s-%s-%d' % (d, c, r) for d in DECISIONS for c in 'AB' for r in (1, 2, 3)]
MIN_BROKEN, MIN_ALT = 4, 2


def gold(uid):
    with open(os.path.join(HERE, 'data', 'gold', uid + '.json'), encoding='utf-8') as f:
        return json.load(f)


def seed(uid):
    return score.load_unit(uid)['seed']


def one(uid, tid, answer):
    answer = dict(answer, task_id=tid)
    res = score.score(uid, {'answers': [answer]})['results']
    return [r for r in res if r['task_id'] == tid][0]


def G(uid, tid):
    return copy.deepcopy({x['task_id']: x for x in gold(uid)['answers']}[tid])


def mod(uid, tid, a, b):
    """Gold answer for tid with a -> b replaced once, in its text or in the first edit's new string holding a."""
    g = G(uid, tid)
    if 'text' in g and not g.get('edits'):
        assert a in g['text'], (uid, tid, a)
        g['text'] = g['text'].replace(a, b, 1)
        return g
    for e in g['edits']:
        if a in e['new']:
            e['new'] = e['new'].replace(a, b, 1)
            return g
    raise AssertionError((uid, tid, a))


def E(*pairs):
    return {'edits': [{'old': o, 'new': n} for o, n in pairs]}


def S(uid, *pairs):
    """Edits against the seed; asserts each old occurs in the seed (as a guard against typos in this file)."""
    s = seed(uid)
    for o, _ in pairs:
        assert o in s, (uid, o)
    return E(*pairs)


def REFUSE(reason='this cannot be expressed with the syntax and styles of this file.'):
    return {'text': 'REFUSE: ' + reason, 'edits': []}


CA1, CB1, CA2, CB2, CA3, CB3 = ('r3-cellpara-%s-%d' % (c, r) for r in (1, 2, 3) for c in 'AB')
EA1, EB1, EA2, EB2, EA3, EB3 = ('r3-emptypara-%s-%d' % (c, r) for r in (1, 2, 3) for c in 'AB')

B1_TABLE3 = ('| 서류 | 대상 | 비고 |\n|---|---|---|\n| 사업 신청서 | 전원 | 지정 서식 |\n'
             '| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 |\n| 주민등록초본 | 전원 | 공고일 이후 발급분 |\n'
             '| 사업자등록증 사본 | 기창업자 | 해당자만 |\n')
B1_LIST3 = ('-\n  - 서류\n  - 대상\n  - 비고\n-\n  - 사업 신청서\n  - 전원\n  - 지정 서식\n-\n  - 사업계획서\n  - 전원\n'
            '  - 지정 서식, 20쪽 이내\n    <div style="표 참고">사업비 집행 계획 포함</div>\n-\n  - 주민등록초본\n  - 전원\n'
            '  - 공고일 이후 발급분\n-\n  - 사업자등록증 사본\n  - 기창업자\n  - 해당자만\n')

# (unit, task, answer factory, expected flag) -- each must be caught (not valid+landed) and carry the flag
BROKEN = [
    # ---------------- cellpara / A (<p/> inside pipe cells)
    (CA1, 'notice1-e2', lambda: mod(CA1, 'notice1-e2', '시제품 제작·마케팅 비용<p/>지식재산권',
                                    '시제품 제작·마케팅 비용<br/>지식재산권'), 'br_for_p'),
    (CA1, 'notice1-e1', lambda: mod(CA1, 'notice1-e1', '<p style="표 참고"/>사업비 집행 계획 포함',
                                    '<p style="표 참고">사업비 집행 계획 포함</p>'), 'cell_para_form'),
    (CA2, 'minutes2-e1', lambda: mod(CA2, 'minutes2-e1', '<p style="Table Emphasis"/>간편 로그인만 3.3에 포함',
                                     '<div style="Table Emphasis">간편 로그인만 3.3에 포함</div>'), 'cell_div'),
    (CA1, 'notice1-e4', lambda: mod(CA1, 'notice1-e4', '<p/>분야별 전문가 1:1 멘토링 (월 2회)<p/>투자 유치 설명회 참가', ''),
     'cell_para_lost'),
    (CA3, 'form3-e1', lambda: S(CA3, ('<p style="표 참고"/>※ 해당 시 작성', '')), 'edit_ambiguous'),
    # a cell ending in <p/> ends with an empty paragraph (§5.2): valid, but not the brief
    (CA2, 'minutes2-w', lambda: mod(CA2, 'minutes2-w', '| 10월 중 조치 |', '| 10월 중 조치<p/> |'), 'wrong_empty_para'),
    (CA3, 'form3-e3', lambda: S(CA3, ('<p/>② 허가받은', '<p style="6pt"/>② 허가받은')), 'css_in_attr'),
    (CA1, 'notice1-w', lambda: mod(CA1, 'notice1-w', '<div style="참고">교육 장소는',
                                   '<p style="참고"/>교육 장소는'), 'p_outside_cell'),
    (CA2, 'minutes2-e2', lambda: G(CB2, 'minutes2-e2'), 'edit_no_match'),
    # ---------------- cellpara / B (list tables)
    (CB1, 'notice1-e1', lambda: S(CB1, ('| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 |',
                                        '| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 <div style="표 참고">사업비 집행 계획 포함</div> |')),
     'cell_div'),
    (CB1, 'notice1-e1', lambda: S(CB1, ('| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 |',
                                        '| 사업계획서 | 전원 | 지정 서식, 20쪽 이내<br/>사업비 집행 계획 포함 |')),
     'br_for_p'),
    (CB1, 'notice1-e1', lambda: S(CB1, (B1_TABLE3, B1_LIST3)), 'list_parsed_as_list'),
    (CB2, 'minutes2-e4', lambda: mod(CB2, 'minutes2-e4', '    품질팀\n-\n', '    품질팀\n\n-\n'), 'list_parsed_as_list'),
    (CB2, 'minutes2-e4', lambda: mod(CB2, 'minutes2-e4', '  - 품질팀 초안 채택\n    <div style="Table Note">11월 1일부터 시행</div>\n',
                                     ''), 'row_width_mismatch'),
    (CB3, 'form3-e2', lambda: S(CB3, ('  - 가. 행사 계획서 1부\n    나. 단체 소개서 1부 (단체인 경우만)\n'
                                      '    <div style="표 참고">※ 해당 시 작성</div>\n',
                                      '  - 가. 행사 계획서 1부, 나. 단체 소개서 1부 (단체인 경우만)\n  <div style="표 참고">※ 해당 시 작성</div>\n')),
     'list_table_form'),
    (CB3, 'form3-e1', lambda: S(CB3, ('    <div style="표 참고">※ 해당 시 작성</div>\n', '')), 'edit_ambiguous'),
    (CB2, 'minutes2-e3', lambda: S(CB2, ('  - 개발2팀\n    품질팀\n',
                                         '  - <div style="Table Emphasis">개발2팀</div>\n    품질팀\n')),
     'should_refuse'),
    (CB3, 'form3-e4', lambda: mod(CB3, 'form3-e4', '  - ^^\n  - ||\n  - ② ', '  - ^^\n    ② 허가받은 용도\n  - ||\n  - ② '),
     'marker_misplaced'),
    (CB1, 'notice1-e4', lambda: mod(CB1, 'notice1-e4', '    분야별 전문가 1:1 멘토링 (월 2회)\n    투자 유치 설명회 참가\n', ''),
     'cell_para_lost'),
    # ---------------- emptypara / A (<p/> lines)
    (EA1, 'letter1-e1', lambda: S(EA1, ('<p/>\n<p/>\n<p/>\n', '<p/>\n<p/>\n')), 'edit_ambiguous'),
    (EA1, 'letter1-e2', lambda: mod(EA1, 'letter1-e2', '<p style="좁은 간격"/>', '<div style="좁은 간격"></div>'),
     'empty_para_form'),
    (EA1, 'letter1-e4', lambda: mod(EA1, 'letter1-e4', '<pagebreak/>', '<p/>\n<pagebreak/>'), 'wrong_empty_para'),
    (EA2, 'board2-e2', lambda: S(EA2, (
        '기명날인한다.\n\n<p/>\n<p/>\n<p/>\n', '기명날인한다.\n\n(첨부: 2027년 사업계획서 1부)\n\n<p/>\n<p/>\n')),
     'wrong_empty_para'),
    (EA2, 'board2-e1', lambda: mod(EA2, 'board2-e1', '<p style="Spacer"/>\n<p/>\n', '<p style="Spacer"/>\n\n'),
     'wrong_empty_para'),
    (EA3, 'report3-w', lambda: mod(EA3, 'report3-w', '<p/>\n<p/>\n<p/>\n<p/>\n<p/>\n\n<div style="표지 정보">',
                                   '<br/>\n<br/>\n<br/>\n<br/>\n<br/>\n\n<div style="표지 정보">'), 'br_for_p'),
    (EA3, 'report3-e4', lambda: S(EA3, ('---\n\n<p/>\n', '---\n\n<p style="height: 6pt"/>\n')), 'css_in_attr'),
    (EA1, 'letter1-w', lambda: mod(EA1, 'letter1-w', '<p style="좁은 간격"/>', '<p style="좁은 간격"></p>'),
     'empty_para_form'),
    (EA3, 'report3-e2', lambda: mod(EA3, 'report3-e2', '<p style="표 아래 간격"/>', '<p style="표 아래 간격" height="6pt"/>'),
     'invented_attr'),
    # ---------------- emptypara / B (<div></div> lines)
    (EB1, 'letter1-e1', lambda: S(EB1, ('<div></div>\n<div></div>\n<div></div>\n', '<div></div>\n<div></div>\n')),
     'edit_ambiguous'),
    (EB1, 'letter1-e2', lambda: mod(EB1, 'letter1-e2', '<div style="좁은 간격"></div>', '<p style="좁은 간격"/>'),
     'unknown_tag'),
    (EB3, 'report3-e3', lambda: S(EB3, ('---\n\n<div></div>\n<div></div>\n<div></div>\n<div></div>\n',
                                        '---\n\n<div></div>\n')), 'wrong_empty_para'),
    (EB2, 'board2-e3', lambda: S(EB2, ('| 설비 투자 | 85 | 120 | +41.2% |\n\n<div style="Spacer"></div>\n\n'
                                       '심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.\n\n<div></div>\n\n',
                                       '| 설비 투자 | 85 | 120 | +41.2% |\n\n'
                                       '심의 결과 출석 이사 전원의 찬성으로 원안대로 승인하였다.\n\n')), 'wrong_empty_para'),
    (EB2, 'board2-w', lambda: mod(EB2, 'board2-w', '<div style="Spacer"></div>', '<div style="Spacer"/>'),
     'empty_para_form'),
    (EB2, 'board2-e1', lambda: REFUSE('an empty paragraph cannot be added here.'), 'wrong_refusal'),
    (EB3, 'report3-e4', lambda: S(EB3, ('---\n\n<div></div>\n', '---\n\n<div style="표 아래 간격"></div>\n')),
     'should_refuse'),
    (EB1, 'letter1-e4', lambda: G(EA1, 'letter1-e4'), 'edit_no_match'),
]

# correct answers written differently from gold -- must be valid and landed
ALTERNATIVES = [
    # cellpara
    (CA1, 'notice1-e1', lambda: S(CA1, ('| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 |',
                                        '| 사업계획서 | 전원 | 지정 서식, 20쪽 이내 <p style="표 참고"/> 사업비 집행 계획 포함 |'))),
    (CA1, 'notice1-e4', lambda: S(CA1, ('분야별 심화 과정 (16시간) | 전액',
                                        '분야별 심화 과정 (16시간)<p/>분야별 전문가 1:1 멘토링 (월 2회)<p/>투자 유치 설명회 참가 | 전액'),
                                  ('| 멘토링 | 분야별 전문가 1:1 멘토링 (월 2회)<p/>투자 유치 설명회 참가 | ^^ |',
                                   '| 멘토링 | ^^ | ^^ |'))),
    (CB3, 'form3-e2', lambda: S(CB3, ('  - 가. 행사 계획서 1부\n    나. 단체 소개서 1부 (단체인 경우만)\n'
                                      '    <div style="표 참고">※ 해당 시 작성</div>\n',
                                      '  - 가. 행사 계획서 1부, 나. 단체 소개서 1부 (단체인 경우만)\n'))),
    (CB1, 'notice1-e1', lambda: S(CB1, ('{style="표 눈금"}\n' + B1_TABLE3, '{style="표 눈금"}\n{list-table}\n' + B1_LIST3))),
    (CB2, 'minutes2-e2', lambda: S(CB2, (
        '  - 간편 로그인 추가 요청이 가장 많음\n    오프라인 모드는 개발 기간이 6주 이상 필요\n'
        '    <div style="Table Note">고객 설문 1,204명 기준</div>\n  - 간편 로그인만 3.3에 포함\n    오프라인 모드는 3.4로 연기\n',
        '  - 간편 로그인 추가 요청이 가장 많음\n    <div style="Table Note">고객 설문 1,204명 기준</div>\n'
        '  - 간편 로그인만 3.3에 포함\n    오프라인 모드는 3.4로 연기\n    오프라인 모드는 개발 기간이 6주 이상 필요\n'))),
    (CA2, 'minutes2-e3', lambda: REFUSE('no style of this file centers text in a cell.')),
    # emptypara
    (EA1, 'letter1-e1', lambda: S(EA1, ('끝.</div>\n\n<p/>\n<p/>\n<p/>\n', '끝.</div>\n\n<p/>\n<p/>\n'))),
    (EB1, 'letter1-e4', lambda: S(EB1, ('<div></div>\n<div></div>\n<div></div>\n<div></div>\n<div></div>\n\n'
                                        '<div style="붙임 제목">붙임 2. 연수 신청서</div>',
                                        '<pagebreak/>\n\n<div style="붙임 제목">붙임 2. 연수 신청서</div>'))),
    (EA2, 'board2-e1', lambda: S(EA2, ('+41.2% |\n\n<p style="Spacer"/>\n', '+41.2% |\n\n<p style="Spacer"/>\n\n<p/>\n'))),
    (EB3, 'report3-e3', lambda: S(EB3, ('<div></div>\n<div></div>\n\n<div style="표지 제목">',
                                        '\n<div style="표지 제목">'))),
    (EA3, 'report3-e4', lambda: REFUSE('heights in points cannot be written.')),
    (EB2, 'board2-e2', lambda: S(EB2, ('기명날인한다.\n\n<div></div>\n<div></div>\n<div></div>\n',
                                       '기명날인한다.\n\n<div></div>\n(첨부: 2027년 사업계획서 1부)\n<div></div>\n'))),
]

# forms DESIGN.md §5.2 gives <p/> under both A candidates -- each must parse without errors to the given blocks.
# (unit, body lines after the unit's front matter, expected blocks)
T = lambda *cells: {'k': 'table', 'style': None, 'grid': {'rows': 2, 'cols': 2, 'cells': [
    [0, 0, [[None, '구분']], 1, 1, True], [0, 1, [[None, '내용']], 1, 1, True],
    [1, 0, [[None, '가']], 1, 1, False], [1, 1, [list(p) for p in cells], 1, 1, False]]}}
TABLE = '| 구분 | 내용 |\n|---|---|\n| 가 | %s |'
P = lambda text, style=None: {'k': 'p', 'style': style, 'text': text}
ACCEPTED = [
    ('empty paragraph mid-cell', CA1, TABLE % 'a<p/><p/>b', [T((None, 'a'), (None, ''), (None, 'b'))]),
    ('styled empty paragraph mid-cell', CA1, TABLE % 'a<p style="표 참고"/><p/>b',
     [T((None, 'a'), ('표 참고', ''), (None, 'b'))]),
    ('trailing empty paragraph in a cell', CA1, TABLE % 'a<p/>', [T((None, 'a'), (None, ''))]),
    ('cell of two empty paragraphs', CA1, TABLE % '<p/><p/>', [T((None, ''), (None, ''))]),
    ('empty first paragraph, then text', CA1, TABLE % '<p/><p/>b', [T((None, ''), (None, 'b'))]),
    ('leading <p style/> styles the first paragraph', CA1, TABLE % '<p style="표 참고"/>a<p/>b',
     [T(('표 참고', 'a'), (None, 'b'))]),
    ('leading plain <p/> is dropped', CA1, TABLE % '<p/>a<p/>b', [T((None, 'a'), (None, 'b'))]),
    ('<p/> and <p style/> lines in a cellpara unit', CA1, '문단\n\n<p/>\n<p style="참고"/>\n\n' + TABLE % 'a',
     [P('문단'), P(''), P('', '참고'), T((None, 'a'))]),
    ('<p/> mid-cell in an emptypara unit', EA1, '<p style="좁은 간격"/>\n\n' + TABLE % 'a<p/><p/>b',
     [P('', '좁은 간격'), T((None, 'a'), (None, ''), (None, 'b'))]),
]


def check_accepted():
    fails = 0
    for name, uid, body, want in ACCEPTED:
        unit = score.load_unit(uid)
        fm = unit['seed'].split('---\n')[1]
        model, ctx = score.parse('---\n' + fm + '---\n\n' + body + '\n', unit)
        if ctx.errors or model['blocks'] != want:
            fails += 1
            print('ACCEPTED FAIL', uid, name, ctx.error_text() or score.first_diff(model['blocks'], want))
    # the same forms reach the scorer as valid answers: gold plus an empty paragraph added mid-cell, or plus a <p/>
    # line, is valid and diagnosed as a wrong empty paragraph
    for uid, tid, old, new in [(CA1, 'notice1-e2', '| 연령 | 공고일 기준 만 19세 이상 39세 이하<p/>',
                                '| 연령 | 공고일 기준 만 19세 이상 39세 이하<p/><p/>'),
                               (CA1, 'notice1-e2', '# 2027년 청년 창업 지원사업 참여자 모집 공고\n',
                                '# 2027년 청년 창업 지원사업 참여자 모집 공고\n\n<p/>\n')]:
        ans = G(uid, tid)
        ans['edits'].append({'old': old, 'new': new})
        r = one(uid, tid, ans)
        if not r['valid'] or r['flags'] != ['not_landed', 'wrong_empty_para']:
            fails += 1
            print('ACCEPTED FAIL (scored)', uid, tid, r)
    print('accepted: %d §5.2 forms parse, 2 scored answers valid' % len(ACCEPTED))
    return fails


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
    print('gold: %d tasks checked' % n_gold)
    per_dec = {}
    for uid, tid, make, flag in BROKEN:
        r = one(uid, tid, make())
        caught = not (r['valid'] and r['landed']) and flag in r['flags']
        per_dec.setdefault(uid.split('-')[1], []).append(caught)
        if not caught:
            fails += 1
            print('BROKEN NOT CAUGHT', uid, tid, flag, r)
        else:
            print('caught  %-19s %-12s %-20s %s' % (uid, tid, flag, (r['error'] or r['detail']).split('\n')[0][:90]))
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
        ok = r['valid'] and r['landed'] and not same
        alt_dec.setdefault(uid.split('-')[1], []).append(ok)
        if not ok:
            fails += 1
            print('ALTERNATIVE FAIL', uid, tid, 'same as gold' if same else r)
    for d in DECISIONS:
        v = alt_dec.get(d, [])
        print('alternatives %-10s %d/%d pass' % (d, sum(v), len(v)))
        if sum(v) < MIN_ALT:
            fails += 1
    fails += check_accepted()
    # CLI round trip
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False, encoding='utf-8') as f:
        json.dump(gold('r3-cellpara-B-2'), f, ensure_ascii=False)
    out = subprocess.run([sys.executable, os.path.join(HERE, 'score.py'), 'r3-cellpara-B-2', f.name],
                         capture_output=True, text=True, check=True).stdout
    os.unlink(f.name)
    assert all(r['valid'] and r['landed'] for r in json.loads(out)['results'])
    print('cli: ok')
    print('SELFTEST %s' % ('PASSED' if fails == 0 else 'FAILED (%d)' % fails))
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
