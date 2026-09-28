#!/usr/bin/env python3
"""Self-test: gold answers must all be valid+landed; broken answers must be caught with the expected flag;
hand-written alternative correct answers (not equal to gold text) must pass. Run: python3 selftest.py"""
import copy
import json
import os
import subprocess
import sys
import tempfile

import score

HERE = os.path.dirname(os.path.abspath(__file__))
UNIT_IDS = ['%s-%s-%d' % (d, c, r) for d in ('merge', 'styleattr', 'slides') for c in 'AB' for r in (1, 2, 3)]


def gold(uid):
    with open(os.path.join(HERE, 'data', 'gold', uid + '.json'), encoding='utf-8') as f:
        return json.load(f)


def one(uid, tid, answer):
    answer = dict(answer, task_id=tid)
    res = score.score(uid, {'answers': [answer]})['results']
    return [r for r in res if r['task_id'] == tid][0]


def mod(uid, tid, a, b):
    """Gold answer for tid with a -> b replaced in its text / in the edits' new strings."""
    g = {x['task_id']: x for x in gold(uid)['answers']}[tid]
    g = copy.deepcopy(g)
    if 'text' in g:
        assert a in g['text'], (uid, tid, a)
        g['text'] = g['text'].replace(a, b, 1)
    else:
        hit = False
        for e in g['edits']:
            if a in e['new']:
                e['new'] = e['new'].replace(a, b, 1)
                hit = True
                break
        assert hit, (uid, tid, a)
    return g


def E(*pairs):
    return {'edits': [{'old': o, 'new': n} for o, n in pairs]}


# (unit, task, answer, expected flag) -- each must be caught (not valid+landed) and carry the flag
BROKEN = [
    # merge / A
    ('merge-A-1', 'report1-e2', lambda: mod('merge-A-1', 'report1-e2', '<td>884</td><td>842</td>', '<td>884</td>'),
     'counted_span_wrong'),
    ('merge-A-1', 'report1-e3', lambda: E(('287', '293')), 'edit_ambiguous'),
    ('merge-A-1', 'report1-e1', lambda: E(('| 대성상사 | 김민수 |', '| 대성상사 | ^^ |')), 'not_landed'),
    ('merge-A-1', 'report1-w', lambda: mod('merge-A-1', 'report1-w', 'rowspan="2">본사', 'rowspan="3">본사'),
     'counted_span_wrong'),
    ('merge-A-3', 'report3-e2', lambda: mod('merge-A-3', 'report3-e2', 'colspan="3">합계', 'colspan="2">합계'),
     'counted_span_wrong'),
    ('merge-A-2', 'report2-e1', lambda: mod('merge-A-2', 'report2-e1', 'colspan="2">화면', 'span="2">화면'),
     'invented_attr'),
    # merge / B
    ('merge-B-1', 'report1-e2', lambda: mod('merge-B-1', 'report1-e2', '| ^^ | 경기 지점', '| 경기 지점'),
     'row_width_mismatch'),
    ('merge-B-1', 'report1-w', lambda: mod('merge-B-1', 'report1-w', '| 구분 |', '| ^^ |'), 'marker_misplaced'),
    ('merge-B-1', 'report1-e1', lambda: gold('merge-A-1')['answers'][1], 'unknown_tag'),
    ('merge-B-1', 'report1-e3', lambda: E(('|부산 지점|301|287|', '|부산 지점|301|293|')), 'edit_no_match'),
    ('merge-B-3', 'report3-e2', lambda: mod('merge-B-3', 'report3-e2', '| 합계 ||| 6,000', '| 합계 || 6,000'),
     'row_width_mismatch'),
    ('merge-B-2', 'report2-e1', lambda: E(('| 관리자 웹 개편 | 화면 개발 | 화면 개발 |', '| 관리자 웹 개편 | 화면 개발 (계속) | |')),
     'not_landed'),
    # styleattr / A
    ('styleattr-A-1', 'document1-e2', lambda: E(('재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.\n',
                                                   '<div style="color:red; font-weight:bold; font-size:16pt">재발 방지를 '
                                                   '위해 전 직원 보안 교육을 10월 중 실시한다.</div>\n')), 'css_in_attr'),
    ('styleattr-A-1', 'document1-e2', lambda: E(('재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.',
                                                   '<span style="color:red">**재발 방지를 위해 전 직원 보안 교육을 10월 중 '
                                                   '실시한다.**</span>')), 'unknown_tag'),
    ('styleattr-A-1', 'document1-e3', lambda: mod('styleattr-A-1', 'document1-e3', 'style="Intense Quote"',
                                                    'style="Intense"'), 'split_name'),
    ('styleattr-A-1', 'document1-e2', lambda: mod('styleattr-A-1', 'document1-e2', 'style="Alert Box"',
                                                    'class="Alert Box"'), 'invented_attr'),
    ('styleattr-A-3', 'document3-e2', lambda: mod('styleattr-A-3', 'document3-e2', 'style="강조 문단"',
                                                    'style="빨간 굵은 글씨"'), 'unknown_style'),
    ('styleattr-A-1', 'document1-e1', lambda: E(('<div style="Note">', '<div style="Body Text Indent">')),
     'edit_ambiguous'),
    # styleattr / B
    ('styleattr-B-1', 'document1-e2', lambda: mod('styleattr-B-1', 'document1-e2', 'class="Alert Box"',
                                                    'class="Alert"'), 'split_name'),
    ('styleattr-B-1', 'document1-e2', lambda: mod('styleattr-B-1', 'document1-e2', 'class="Alert Box"',
                                                    'style="Alert Box"'), 'invented_attr'),
    ('styleattr-B-1', 'document1-e2', lambda: mod('styleattr-B-1', 'document1-e2', 'class="Alert Box"',
                                                    'class="red bold large"'), 'css_in_attr'),
    ('styleattr-B-2', 'document2-e1', lambda: mod('styleattr-B-2', 'document2-e1', 'class="Plain Table 1"',
                                                    'class="Plain-Table-1"'), 'split_name'),
    ('styleattr-B-3', 'document3-w', lambda: mod('styleattr-B-3', 'document3-w', 'class="격자 표 4"',
                                                   'class="격자"'), 'split_name'),
    ('styleattr-B-2', 'document2-e2', lambda: mod('styleattr-B-2', 'document2-e2', 'class="Key Message"',
                                                    'class="Note"'), 'not_landed'),
    # slides / A
    ('slides-A-1', 'deck1-e1', lambda: mod('slides-A-1', 'deck1-e1', 'layout="Two Content"',
                                              'layout="Two Columns"'), 'unknown_layout'),
    ('slides-A-1', 'deck1-w', lambda: mod('slides-A-1', 'deck1-w', '<body>영업기획팀</body>',
                                             '<subtitle>영업기획팀</subtitle>'), 'unknown_slot'),
    ('slides-A-1', 'deck1-e3', lambda: E(('전년 대비 +12%', '전년 대비 +10%')), 'edit_ambiguous'),
    ('slides-A-2', 'deck2-e1', lambda: mod('slides-A-2', 'deck2-e1', '<body>\n- 콜센터', '<left>\n- 콜센터'),
     'unknown_slot'),
    ('slides-A-3', 'deck3-e1', lambda: mod('slides-A-3', 'deck3-e1', '<slide layout="Section Header">',
                                              '<slide layout="Section Header" transition="fade">'), 'invented_attr'),
    # slides / B
    ('slides-B-1', 'deck1-e1', lambda: mod('slides-B-1', 'deck1-e1', 'layout: Two Content\n',
                                              'layout: Two Content\n---\n'), 'missing_layout'),
    ('slides-B-1', 'deck1-w', lambda: mod('slides-B-1', 'deck1-w', '::body::\n영업기획팀',
                                             '::subtitle::\n영업기획팀'), 'unknown_slot'),
    ('slides-B-3', 'deck3-e1', lambda: mod('slides-B-3', 'deck3-e1', 'layout: Section Header\n',
                                              'layout: Section Header\ntransition: fade\n'), 'invented_attr'),
    ('slides-B-2', 'deck2-w', lambda: mod('slides-B-2', 'deck2-w', 'layout: 구역 머리글\n', ''),
     'missing_layout'),
    ('slides-B-1', 'deck1-e3', lambda: E(('전년 대비 +12%', '전년 대비 +10%')), 'edit_ambiguous'),
    ('slides-B-2', 'deck2-e2', lambda: E(('- 부품사업부 원가 상승 영향\n', '')), 'not_landed'),
]

# correct answers written differently from gold -- must be valid and landed
ALTERNATIVES = [
    ('merge-B-1', 'report1-e1', E(('| 대성상사 | 김민수 |', '| 대성상사 | ^^ |'))),
    ('merge-A-1', 'report1-e3', E(('<td>301</td><td>287</td>', '<td>301</td><td>293</td>'))),
    ('merge-B-1', 'report1-e3', E(('| 301 | 287 | 312 |', '| 301 | 293 | 312 |'))),
    ('merge-A-2', 'report2-e1', E(('<td>화면 개발</td><td>화면 개발</td>', '<td colspan="2">화면 개발 (계속)</td>'))),
    ('merge-B-2', 'report2-e1', E(('| 화면 개발 | 화면 개발 |', '| 화면 개발 (계속) ||'))),
    ('styleattr-B-1', 'document1-e2', E(('재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.\n',
                                           '<div class="Alert Box">재발 방지를 위해 전 직원 보안 교육을 10월 중 실시한다.</div>\n'))),
    ('styleattr-A-1', 'document1-e1', E(('<div style="Note">이전', '<div style="Body Text Indent">이전'))),
    ('slides-A-2', 'deck2-e3', E(('- 매출: 목표 대비 96%\n', '- 매출: 목표 대비 97%\n'))),
    ('slides-B-3', 'deck3-e3', E(('- 도입 후\n- 출고 지연 2% 이하', '- 도입 후\n- 출고 지연 1.5% 이하'))),
    ('slides-B-3', 'deck3-e2', E(('- 분류 인력 60명 → 25명\n::notes::\n인력 재배치 방안은 질문 시 설명\n',
                                     '- 분류 인력 60명 → 25명\n- 인력 재배치 방안은 질문 시 설명\n'))),
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
        per_dec.setdefault(uid.split('-')[0], []).append(caught)
        if not caught:
            fails += 1
            print('BROKEN NOT CAUGHT', uid, tid, flag, r)
        else:
            print('caught  %-15s %-15s %-20s %s' % (uid, tid, flag, (r['error'] or r['detail']).split('\n')[0][:110]))
    for d, v in per_dec.items():
        print('broken %-9s %d/%d caught' % (d, sum(v), len(v)))
    for uid, tid, ans in ALTERNATIVES:
        r = one(uid, tid, ans)
        if not (r['valid'] and r['landed']):
            fails += 1
            print('ALTERNATIVE FAIL', uid, tid, r)
    print('alternatives: %d checked' % len(ALTERNATIVES))
    # CLI round trip
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False, encoding='utf-8') as f:
        json.dump(gold('merge-B-2'), f, ensure_ascii=False)
    out = subprocess.run([sys.executable, os.path.join(HERE, 'score.py'), 'merge-B-2', f.name],
                         capture_output=True, text=True, check=True).stdout
    os.unlink(f.name)
    assert all(r['valid'] and r['landed'] for r in json.loads(out)['results'])
    print('cli: ok')
    print('SELFTEST %s' % ('PASSED' if fails == 0 else 'FAILED (%d)' % fails))
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
