#!/usr/bin/env python3
"""Round 2 self-test: gold answers must all be valid+landed with no flags; broken answers must be caught with the
expected flag; alternative correct answers (not equal to gold) must pass. Run: python3 selftest.py"""
import copy
import json
import os
import subprocess
import sys
import tempfile

import score

HERE = os.path.dirname(os.path.abspath(__file__))
DECISIONS = ('merge', 'styleattr', 'tablestyle', 'slides')
UNIT_IDS = ['r2-%s-%s-%d' % (d, c, r) for d in DECISIONS for c in 'AB' for r in (1, 2, 3)]


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


def set_new(uid, tid, k, new):
    """Gold answer for tid with the new string of its k-th edit replaced."""
    g = G(uid, tid)
    g['edits'][k]['new'] = new
    return g


def E(*pairs):
    return {'edits': [{'old': o, 'new': n} for o, n in pairs]}


def S(uid, *pairs):
    """Edits against the seed; asserts each old occurs in the seed (as a guard against typos in this file)."""
    s = seed(uid)
    for o, _ in pairs:
        assert o in s, (uid, o)
    return E(*pairs)


def REFUSE(reason='this cannot be expressed with the styles of this file.'):
    return {'text': 'REFUSE: ' + reason, 'edits': []}


# (unit, task, answer factory, expected flag) -- each must be caught (not valid+landed) and carry the flag
BROKEN = [
    # ---------------- merge / A (HTML spans)
    ('r2-merge-A-1', 'report1-e1', lambda: mod('r2-merge-A-1', 'report1-e1', 'rowspan="4">수도권', 'rowspan="3">수도권'),
     'counted_span_wrong'),
    ('r2-merge-A-1', 'report1-e2', lambda: mod('r2-merge-A-1', 'report1-e2', '<td rowspan="2">영남권</td><td>대구 지점',
                                                 '<td>대구 지점'), 'counted_span_wrong'),
    ('r2-merge-A-1', 'report1-e3', lambda: mod('r2-merge-A-1', 'report1-e3', 'colspan="3">3분기', 'colspan="2">3분기'),
     'counted_span_wrong'),
    ('r2-merge-A-2', 'report2-e2', lambda: mod('r2-merge-A-2', 'report2-e2', '<tr><td>보안팀</td><td>5</td></tr>',
                                                 '<tr><td>보안팀</td><td>5</td><td>5</td></tr>'), 'counted_span_wrong'),
    ('r2-merge-A-3', 'report3-e4', lambda: mod('r2-merge-A-3', 'report3-e4', '<td rowspan="3">교통정책과</td>',
                                                 '<td rowspan="2">교통정책과</td>'), 'counted_span_wrong'),
    ('r2-merge-A-2', 'report2-w', lambda: mod('r2-merge-A-2', 'report2-w', 'colspan="2">외부', 'span="2">외부'),
     'invented_attr'),
    ('r2-merge-A-1', 'report1-e4', lambda: G('r2-merge-B-1', 'report1-e4'), 'edit_no_match'),
    # ---------------- merge / B (^^ / || markers)
    ('r2-merge-B-1', 'report1-e1', lambda: mod('r2-merge-B-1', 'report1-e1', '| ^^ | 수원 지점', '| 수원 지점'),
     'row_width_mismatch'),
    ('r2-merge-B-1', 'report1-e2', lambda: S('r2-merge-B-1', ('| 영남권 | 부산 지점 | 861 | 900 | +4.5% |\n', '')),
     'not_landed'),
    ('r2-merge-B-1', 'report1-e4', lambda: S('r2-merge-B-1', ('| 128 | 88 | 64 | 72 |', '| 128 | 88 | 합산 집계 중 ||')),
     'not_landed'),
    ('r2-merge-B-1', 'report1-w', lambda: mod('r2-merge-B-1', 'report1-w', '| 구분 |', '| ^^ |'), 'marker_misplaced'),
    ('r2-merge-B-3', 'report3-e1', lambda: mod('r2-merge-B-3', 'report3-e1', '| 합계 |||| 4,490', '| 합계 ||| 4,490'),
     'row_width_mismatch'),
    ('r2-merge-B-2', 'report2-e1', lambda: G('r2-merge-A-2', 'report2-e1'), 'edit_no_match'),
    ('r2-merge-B-1', 'report1-e3', lambda: S('r2-merge-B-1', ('| 488 |', '| 480 |')), 'edit_ambiguous'),
    ('r2-merge-B-2', 'report2-w', lambda: mod('r2-merge-B-2', 'report2-w', '| 외부 위탁 (한결시큐리티) ||',
                                                '| 외부 위탁 (한결시큐리티) |  |'), 'brief_unmet'),
    # ---------------- styleattr / A (style="Name")
    ('r2-styleattr-A-1', 'memo1-e2', lambda: S('r2-styleattr-A-1', (
        '\n보안 교육 미이수자는 10월 말까지 이수한다.\n',
        '\n<div style="color:red">보안 교육 미이수자는 10월 말까지 이수한다.</div>\n')), 'css_in_attr'),
    ('r2-styleattr-A-1', 'memo1-e2', lambda: S('r2-styleattr-A-1', (
        '보안 교육 미이수자는 10월 말까지 이수한다.',
        '<span style="color:red">**보안 교육 미이수자는 10월 말까지 이수한다.**</span>')), 'unknown_tag'),
    ('r2-styleattr-A-1', 'memo1-e2', lambda: S('r2-styleattr-A-1', (
        '\n보안 교육 미이수자는 10월 말까지 이수한다.\n',
        '\n<div style="Warning">보안 교육 미이수자는 10월 말까지 이수한다.</div>\n')), 'unknown_style'),
    ('r2-styleattr-A-1', 'memo1-e2', lambda: S('r2-styleattr-A-1', (
        '\n보안 교육 미이수자는 10월 말까지 이수한다.\n',
        '\n<div style="Intense Quote">보안 교육 미이수자는 10월 말까지 이수한다.</div>\n')), 'should_refuse'),
    ('r2-styleattr-A-2', 'memo2-e2', lambda: mod('r2-styleattr-A-2', 'memo2-e2', 'style="Medium Shading 1 Accent 2"',
                                                   'style="Medium Shading 1 Accent 2" border="0"'), 'invented_attr'),
    ('r2-styleattr-A-2', 'memo2-e4', lambda: mod('r2-styleattr-A-2', 'memo2-e4', 'style="Key Message Small"',
                                                   'style="Key Message"'), 'not_landed'),
    ('r2-styleattr-A-3', 'memo3-e3', lambda: mod('r2-styleattr-A-3', 'memo3-e3', '<table style="눈금 표 4 - 강조색 1">\n',
                                                   '<table style="눈금 표 4 - 강조색 1">\n\n'), 'wrapper_content'),
    ('r2-styleattr-A-1', 'memo1-e1', lambda: {'text': 'REFUSE: no blue style.', 'edits': [
        {'old': 'x', 'new': 'y'}]}, 'bad_refusal'),
    # ---------------- styleattr / B (class="Name")
    ('r2-styleattr-B-1', 'memo1-e1', lambda: mod('r2-styleattr-B-1', 'memo1-e1', 'class="Grid Table 4 Accent 1"',
                                                   'class="Grid Table 4 Accent"'), 'split_name'),
    ('r2-styleattr-B-1', 'memo1-e3', lambda: mod('r2-styleattr-B-1', 'memo1-e3', 'class="Body Text Indent"',
                                                   'class="Indent"'), 'split_name'),
    ('r2-styleattr-B-1', 'memo1-e4', lambda: mod('r2-styleattr-B-1', 'memo1-e4', 'class="Signature"',
                                                   'class="right"'), 'css_in_attr'),
    ('r2-styleattr-B-3', 'memo3-e1', lambda: mod('r2-styleattr-B-3', 'memo3-e1', 'class="본문 들여쓰기"',
                                                   'class="본문 들여쓰기 2"'), 'not_landed'),
    ('r2-styleattr-B-3', 'memo3-e4', lambda: REFUSE('no style centers text.'), 'wrong_refusal'),
    ('r2-styleattr-B-2', 'memo2-e3', lambda: S('r2-styleattr-B-2', (
        '연간 물류비 18% 절감과', '<mark>연간 물류비 18% 절감</mark>과')), 'unknown_tag'),
    ('r2-styleattr-B-2', 'memo2-e1', lambda: mod('r2-styleattr-B-2', 'memo2-e1', 'class="Callout Warning"',
                                                   'class="Callout Warning" style="border:orange"'), 'css_in_attr'),
    # ---------------- tablestyle / A (<table style> ... </table>)
    ('r2-tablestyle-A-1', 'form1-e1', lambda: {'edits': G('r2-tablestyle-A-1', 'form1-e1')['edits'][:1]},
     'unclosed_tag'),
    ('r2-tablestyle-A-1', 'form1-e1', lambda: mod('r2-tablestyle-A-1', 'form1-e1', '<table style="Grid Table 4">\n',
                                                    '<table style="Grid Table 4">\n\n'), 'wrapper_content'),
    ('r2-tablestyle-A-1', 'form1-e3', lambda: mod('r2-tablestyle-A-1', 'form1-e3', '</table>\n\n<div style="Caption">물류 분야</div>\n\n<table style="Grid Table 4">\n',
                                                    '<div style="Caption">물류 분야</div>\n'), 'wrapper_content'),
    ('r2-tablestyle-A-1', 'form1-e1', lambda: mod('r2-tablestyle-A-1', 'form1-e1', 'style="Grid Table 4"',
                                                    'style="Grid Table 4 Accent"'), 'split_name'),
    ('r2-tablestyle-A-2', 'form2-e1', lambda: S('r2-tablestyle-A-2', ('<table style="눈금 표 4">\n| 분야 | 과정명 |',
                                                                      '<table style="눈금 표 4 - 강조색 1">\n| 분야 | 과정명 |')),
     'edit_ambiguous'),
    ('r2-tablestyle-A-1', 'form1-e2', lambda: S('r2-tablestyle-A-1', (
        '<table style="Grid Table 4">\n| 분야 | 업체 | 품질',
        '<table style="Grid Table 4" color="red">\n| 분야 | 업체 | 품질')), 'css_in_attr'),
    ('r2-tablestyle-A-3', 'form3-e2', lambda: G('r2-tablestyle-B-3', 'form3-e2'), 'not_landed'),
    # ---------------- tablestyle / B ({style="Name"} line)
    ('r2-tablestyle-B-1', 'form1-e1', lambda: mod('r2-tablestyle-B-1', 'form1-e1', '{style="Grid Table 4"}\n',
                                                    '{style="Grid Table 4"}\n\n'), 'attr_orphan'),
    ('r2-tablestyle-B-1', 'form1-e1', lambda: mod('r2-tablestyle-B-1', 'form1-e1', '{style="Grid Table 4"}',
                                                    '{.Grid Table 4}'), 'attr_form'),
    ('r2-tablestyle-B-1', 'form1-e2', lambda: S('r2-tablestyle-B-1', (
        '{style="Grid Table 4"}\n| 분야 | 업체 | 품질', '{style="color: red"}\n| 분야 | 업체 | 품질')), 'css_in_attr'),
    ('r2-tablestyle-B-1', 'form1-e3', lambda: mod('r2-tablestyle-B-1', 'form1-e3', '{style="Grid Table 4"}\n', ''),
     'not_landed'),
    ('r2-tablestyle-B-3', 'form3-e4', lambda: set_new('r2-tablestyle-B-3', 'form3-e4', 1,
                                                        '{style="Light Grid - Accent 1"}\n'), 'pipe_delimiter'),
    ('r2-tablestyle-B-3', 'form3-e2', lambda: G('r2-tablestyle-A-3', 'form3-e2'), 'unknown_tag'),
    ('r2-tablestyle-B-2', 'form2-e3', lambda: S('r2-tablestyle-B-2', (
        '{style="눈금 표 4"}\n| 분야 | 과정명 | 시기 | 대상 | 시간 |\n|---|---|---|---|---|\n| 안전 |',
        '{style="눈금 표 4" border="2"}\n| 분야 | 과정명 | 시기 | 대상 | 시간 |\n|---|---|---|---|---|\n| 안전 |')),
     'invented_attr'),
    # ---------------- slides / A (<slide layout> tags)
    ('r2-slides-A-1', 'deck1-e1', lambda: mod('r2-slides-A-1', 'deck1-e1', '</slide>\n\n<slide layout="Title and Content">',
                                                '\n<slide layout="Title and Content">'), 'unclosed_tag'),
    ('r2-slides-A-1', 'deck1-e4', lambda: mod('r2-slides-A-1', 'deck1-e4', 'layout="Two Content"', 'layout="Two Columns"'),
     'unknown_layout'),
    ('r2-slides-A-2', 'deck2-e4', lambda: S('r2-slides-A-2', (
        '<shape id="s4" name="기준">기준: 3분기 누계, 잠정치</shape>\n',
        '<shape id="s4" name="기준">기준: 3분기 누계, 잠정치</shape>\n'
        '<shape id="s99" name="메모">수치는 잠정치, 10월 20일 확정</shape>\n')), 'invented_shape'),
    ('r2-slides-A-3', 'deck3-e3', lambda: mod('r2-slides-A-3', 'deck3-e3', '<body>\n- 초기 투자', '<left>\n- 초기 투자'),
     'unknown_slot'),
    ('r2-slides-A-2', 'deck2-e3', lambda: mod('r2-slides-A-2', 'deck2-e3', '<slide layout="Title and Content">',
                                                '<slide layout="Title and Content" transition="fade">'),
     'invented_attr'),
    ('r2-slides-A-1', 'deck1-e3', lambda: S('r2-slides-A-1', ('<notes>질문 시 설명</notes>',
                                                               '<notes>9월 추석 프로모션은 마케팅팀과 공동 진행</notes>')),
     'edit_ambiguous'),
    # ---------------- slides / B (Slidev-style)
    ('r2-slides-B-1', 'deck1-e1', lambda: mod('r2-slides-B-1', 'deck1-e1', 'layout: Title and Content\n',
                                                'layout: Title and Content\n---\n'), 'missing_layout'),
    ('r2-slides-B-1', 'deck1-e2', lambda: S('r2-slides-B-1', (
        '<shape id="s5" name="단위">단위: 억 원, 상반기 누계</shape>\n',
        '<shape id="s5" name="단위">단위: 억 원, 상반기 누계</shape>\n- 충청권 1,480억 원\n'),
        ('- 영남권 3,260억 원\n- 충청권 1,480억 원\n', '- 영남권 3,260억 원\n')), 'text_outside_slot'),
    ('r2-slides-B-1', 'deck1-e3', lambda: S('r2-slides-B-1', ('::notes::\n질문 시 설명', '::notes::\n9월 추석 프로모션은 '
                                                                                  '마케팅팀과 공동 진행')),
     'edit_ambiguous'),
    ('r2-slides-B-2', 'deck2-e2', lambda: S('r2-slides-B-2', ('::title::\nQ&A\n', '::title::\nQ&A\n'
                                                           'layout: Two Content\n::title::\n부록: 사업부별 수주 현황\n')),
     'duplicate_slot'),
    ('r2-slides-B-2', 'deck2-e1', lambda: mod('r2-slides-B-2', 'deck2-e1', '---\n', '---\n\n---\n'), 'empty_slide'),
    ('r2-slides-B-1', 'deck1-e1', lambda: G('r2-slides-A-1', 'deck1-e1'), 'edit_no_match'),
    ('r2-slides-B-3', 'deck3-e4', lambda: REFUSE('notes cannot be removed.'), 'wrong_refusal'),
]

# correct answers written differently from gold -- must be valid and landed
ALTERNATIVES = [
    # merge
    ('r2-merge-B-1', 'report1-e4', lambda: S('r2-merge-B-1', ('| 128 | 88 | 64 | 72 |\n| ^^ | 반려동물용품 | 82 | 57 | 44 | 40 |',
                                                             '| 128 | 88 | 합산 집계 중 ||\n| ^^ | 반려동물용품 | 82 | 57 | ^^ | ^^ |'))),
    ('r2-merge-B-3', 'report3-e2', lambda: S('r2-merge-B-3', ('DB 이중화 | ^^ |', 'DB 이중화 | 정보통신과 |'))),
    ('r2-merge-A-1', 'report1-e1', lambda: S('r2-merge-A-1', (
        '<td rowspan="3">수도권</td><td>서울 본점</td><td>412</td>', '<td rowspan="4">수도권</td><td>서울 본점</td><td>412</td>'),
        ('<tr><td>인천 지점</td><td>156</td><td>162</td><td>170</td><td>488</td></tr>\n',
         '<tr><td>인천 지점</td><td>156</td><td>162</td><td>170</td><td>488</td></tr>\n'
         '<tr><td>수원 지점</td><td>64</td><td>71</td><td>83</td><td>218</td></tr>\n'))),
    ('r2-merge-A-3', 'report3-e3', lambda: S('r2-merge-A-3', (
        '<tr><td>6급</td><td>정규직</td>', '<tr><td rowspan="2">6급</td><td>정규직</td>'),
        ('<tr><td>6급</td><td>기간제</td>', '<tr><td>기간제</td>'),
        ('<tr><td>7급</td><td>정규직</td>', '<tr><td rowspan="2">7급</td><td>정규직</td>'),
        ('<tr><td>7급</td><td>기간제</td>', '<tr><td>기간제</td>'))),
    # styleattr
    ('r2-styleattr-B-1', 'memo1-e2', lambda: REFUSE('there is no red style in this file.')),
    ('r2-styleattr-A-1', 'memo1-e3', lambda: S('r2-styleattr-A-1', (
        '\n인건비는 동결하되', '\n<div style="Body Text Indent">인건비는 동결하되'),
        ('별도로 반영한다.\n', '별도로 반영한다.</div>\n'),
        ('<div style="Body Text">마케팅 예산은', '<div style="Body Text Indent">마케팅 예산은'))),
    ('r2-styleattr-B-3', 'memo3-e4', lambda: S('r2-styleattr-B-3', ('\n○○시장\n', '\n<div class="발신 명의">○○시장</div>\n'))),
    # tablestyle
    ('r2-tablestyle-A-1', 'form1-e4', lambda: S('r2-tablestyle-A-1', (
        '<table style="Table Grid">\n| 구분 | 항목 |', '| 구분 | 항목 |'),
        ('| 합계 || 100 |  |\n</table>\n', '| 합계 || 100 |  |\n'))),
    ('r2-tablestyle-B-3', 'form3-e2', lambda: S('r2-tablestyle-B-3', (
        '| 유형 | 대책 |', '{style="Medium Grid 3 - Accent 1"}\n| 유형 | 대책 |'))),
    ('r2-tablestyle-B-1', 'form1-e3', lambda: S('r2-tablestyle-B-1', (
        '| 물류 | 동방로지스 | 35 |', '<div style="Caption">물류 분야</div>\n\n{style="Grid Table 4"}\n'
                              '| 분야 | 업체 | 품질 | 납기 | 가격 | 협력 | 총점 | 등급 |\n|---|---|---|---|---|---|---|---|\n'
                              '| 물류 | 동방로지스 | 35 |'))),
    ('r2-tablestyle-A-2', 'form2-e3', lambda: REFUSE('no table style of this file makes the lines thicker.')),
    # slides
    ('r2-slides-B-1', 'deck1-e2', lambda: S('r2-slides-B-1', (
        '- 충청권 1,480억 원\n::right::\n- 호남권 1,390억 원\n- 강원권 610억 원\n- 제주 620억 원\n',
        '::right::\n- 호남권 1,390억 원\n- 강원권 610억 원\n- 제주 620억 원\n* 충청권 1,480억 원\n'))),
    ('r2-slides-B-2', 'deck2-e4', lambda: S('r2-slides-B-2', (
        '<shape id="s4" name="기준">기준: 3분기 누계, 잠정치</shape>\n',
        '<shape id="s4" name="기준">기준: 3분기 누계, 잠정치</shape>\n::notes::\n수치는 잠정치, 10월 20일 확정\n'))),
    ('r2-slides-A-2', 'deck2-e4', lambda: S('r2-slides-A-2', (
        '<title>전사 KPI</title>\n', '<title>전사 KPI</title>\n<notes>\n수치는 잠정치, 10월 20일 확정\n</notes>\n'))),
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
            print('caught  %-19s %-11s %-19s %s' % (uid, tid, flag, (r['error'] or r['detail']).split('\n')[0][:100]))
    for d in DECISIONS:
        v = per_dec.get(d, [])
        print('broken %-10s %d/%d caught' % (d, sum(v), len(v)))
        if sum(v) < 3:
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
        if sum(v) < 1:
            fails += 1
    # CLI round trip
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False, encoding='utf-8') as f:
        json.dump(gold('r2-tablestyle-B-2'), f, ensure_ascii=False)
    out = subprocess.run([sys.executable, os.path.join(HERE, 'score.py'), 'r2-tablestyle-B-2', f.name],
                         capture_output=True, text=True, check=True).stdout
    os.unlink(f.name)
    assert all(r['valid'] and r['landed'] for r in json.loads(out)['results'])
    print('cli: ok')
    print('SELFTEST %s' % ('PASSED' if fails == 0 else 'FAILED (%d)' % fails))
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
