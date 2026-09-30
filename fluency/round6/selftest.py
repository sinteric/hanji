#!/usr/bin/env python3
"""Round 6 self-test: gold answers land (build.py asserts it too), broken answers are caught, and alternative
correct answers pass. Run: python3 selftest.py (after build.py). Ends with SELFTEST PASSED."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import score as S  # noqa: E402


def seed(uid):
    return open(os.path.join(HERE, 'seeds', uid + '.txt'), encoding='utf-8').read()


def line_with(text, *needles):
    for ln in text.split('\n'):
        if all(n in ln for n in needles):
            return ln
    raise KeyError(needles)


def one(uid, tid, answer):
    res = S.score_answer(uid, {'answers': [dict(answer, task_id=tid)]})
    return next(r for r in res if r['task_id'] == tid)


def edit(old, new):
    return {'edits': [{'old': old, 'new': new}]}


def main():
    fails = []
    n_gold = 0
    import pptx_kit as PK
    for f in sorted(os.listdir(os.path.join(HERE, 'data', 'gold'))):
        uid = f[:-5]
        if uid.startswith('r6x-'):
            continue    # part D has its own: python3 xlsx_kit.py selftest
        unit = json.load(open(os.path.join(HERE, 'data', 'units', f), encoding='utf-8'))
        gold = json.load(open(os.path.join(HERE, 'data', 'gold', f), encoding='utf-8'))
        scorer = PK.score_answer if uid.startswith('r6p-') else S.score_answer
        for r in scorer(uid, gold):
            if uid.startswith('r6p-') and r['task_id'] in unit['unreachable']:
                continue
            n_gold += 1
            ok = r['landed'] or (r['task_id'] in unit['unreachable'] and 'unreachable_refused' in r['flags'])
            if not ok:
                fails.append(('gold', uid, r['task_id'], r['error'], r['detail']))
    print('gold answers:', n_gold)

    broken = []   # (uid, task, answer, why)
    good = []
    B = '3D 프린팅 기술의 미래와 전망'
    for c in ('F1', 'F2'):
        uid = 'r6-%s-1' % c
        t = seed(uid)
        ban = line_with(t, B, 'fill=#FFF0C3')
        first = line_with(t, '3D 프린팅 기술의 등장과', 'fill=#E3F8FF')
        broken.append((uid, 'fn-e1', edit(first, first.replace('#E3F8FF', '#DDEBF7')), 'the wrong banner'))
        broken.append((uid, 'fn-e1', edit(ban, ban.replace('#FFF0C3', '#FF0000')), 'red is not light blue'))
        broken.append((uid, 'fn-e1', edit(ban, ban.replace('#FFF0C3', '#DDEBF7').replace('size=20pt', 'size=22pt')),
                       'a size changed too'))
        good.append((uid, 'fn-e1', edit(ban, ban.replace('#FFF0C3', '#BDD7EE')), 'another light blue'))
        broken.append((uid, 'fn-e2', edit(ban, ban.replace('2.83pt solid #7F7F7F', '1pt solid #FF0000')),
                       'width changed'))
        good.append((uid, 'fn-e2', edit(ban, ban.replace('2.83pt solid #7F7F7F', '2.83pt solid #C00000')),
                     'a darker red'))
        broken.append((uid, 'fn-e1', edit(ban, ban.replace('fill=#FFF0C3', 'colour=#DDEBF7')), 'unknown key'))
        broken.append((uid, 'fn-e1', edit(ban, ban.replace('fill=#FFF0C3', 'fill=lightblue')), 'a colour name'))
        broken.append((uid, 'fn-e1', edit(ban, ban.replace('{fill=#FFF0C3', '{style="background:#DDEBF7"')),
                       'CSS'))
        item = line_with(t, '시제품 제작 시간과')
        broken.append((uid, 'fn-e6', edit(item, item.replace('시간', '기간').replace(' {style="개요 3"', ' {style="개요 4"')
                                           if c == 'F2' else item.replace('시간', '기간').replace('style="개요 3" ', '')),
                       'formatting changed with the text'))
        other = line_with(t, '문제점 분석')
        broken.append((uid, 'fn-e4', edit(other, other.replace('문제점 분석', '문제점  분석')), 'text changed'))
    t = seed('r6-F2-1')
    s2 = line_with(t, '<style name="개요 2"')
    s3 = line_with(t, '<style name="개요 3"')
    broken.append(('r6-F2-1', 'fn-e5', edit(s2, s2.replace('/>', ' color=#1F3864/>')), 'the wrong style'))
    good.append(('r6-F2-1', 'fn-e5', edit(s3, s3.replace('/>', ' color=#1F3864/>')), 'the style line'))
    # F2: the same by direct formatting on every paragraph of 개요 3
    edits = []
    for ln in t.split('\n'):
        if '{style="개요 3"' in ln:
            edits.append({'old': ln, 'new': ln[:-1] + ' color=#1F3864}'})
    good.append(('r6-F2-1', 'fn-e5', {'edits': edits}, 'direct on every paragraph'))
    # header row fill by the row's {…}
    t1 = seed('r6-F1-3')
    hdr = line_with(t1, '| 지역 | 지점 | 매출 |')
    good.append(('r6-F1-3', 'kr-e3', edit(hdr, '| **지역** | **지점** | **매출** | {fill=#D9D9D9}'), 'row brace'))
    broken.append(('r6-F1-3', 'kr-e3', edit(hdr, '| 지역 | 지점 | 매출 | {fill=#D9D9D9}'), 'not bold'))
    # docx F2: the Normal style edit indents headings and cells too
    t2 = seed('r6-F2-3')
    nl = line_with(t2, '<style name="Normal"')
    broken.append(('r6-F2-3', 'kr-e8', edit(nl, nl.replace('line-spacing', 'first-line=10pt line-spacing')),
                   'the default style reaches headings and cells'))
    broken.append(('r6-F2-3', 'kr-q2', {'text': 'ANSWER: 21곳이 수도권; 부산'}, 'one missing'))
    good.append(('r6-F2-3', 'kr-q2', {'text': 'ANSWER: 21곳이 수도권; 부산; 대구'}, 'all three'))
    good.append(('r6-F3-3', 'kr-q2', {'text': 'REFUSE: run colours are not shown'}, 'unreachable refusal'))
    # F3: a style edit lands, a brace is a parse error
    t3 = seed('r6-F3-2')
    st = line_with(t3, '<style name="표가운데"')
    good.append(('r6-F3-2', 'mel-e5', edit(st, st.replace('/>', ' color=#1F3864/>')), 'F3 style line'))
    ln = line_with(t3, '노동이 존중받는 일터')
    broken.append(('r6-F3-2', 'mel-e1', edit(ln, ln.replace('| ', '| {fill=#DDEBF7} ', 1)), 'brace in F3'))
    # refusal task answered with an edit
    broken.append(('r6-F1-1', 'fn-e9', edit(ban if False else line_with(seed('r6-F1-1'), B),
                                             line_with(seed('r6-F1-1'), B).replace('#FFF0C3', 'gradient')),
                   'should refuse'))
    # part B (presentations)
    import pptx_kit as PK
    p1 = seed('r6p-F1-1')
    card = line_with(p1, 'name="Rounded Rectangle 7"')
    other = line_with(p1, 'name="Rounded Rectangle 2"')
    pb = []
    pb.append(('r6p-F1-1', 'pp-e1', edit(other, other.replace('fill=#FFFFFF', 'fill=accent2')), 'the wrong card', False))
    pb.append(('r6p-F1-1', 'pp-e1', edit(card, card.replace('fill=#FFFFFF', 'fill=accent2')), 'the card', True))
    pb.append(('r6p-F1-1', 'pp-e1', edit(card, card.replace('fill=#FFFFFF', 'fill=accent2 size=20pt')),
               'a size added too', False))
    tb = line_with(p1, 'Battery cost down 60%')
    pb.append(('r6p-F1-1', 'pp-e3', edit(tb, tb.replace('Battery', '[Battery]{size=24pt color=#FF7F50}')),
               'the wrong word', False))
    pb.append(('r6p-F1-1', 'pp-e3', edit(tb, tb.replace('60%', '[60%]{size=24pt color=#FF6F61}')), 'coral', True))
    pb.append(('r6p-F1-1', 'pp-e3', edit(tb, tb.replace('60%', '[60%]{size=24pt color=#1F3864}')), 'navy', False))
    con = line_with(p1, 'name="Connector 6"')
    p2 = seed('r6p-F2o-1')
    con2 = line_with(p2, 'name="Connector 6"')
    pb.append(('r6p-F1-1', 'pp-e2', edit(line_with(p1, 'name="Rounded Rectangle 12"'),
                                         line_with(p1, 'name="Rounded Rectangle 12"').replace(
                                             'border=none', 'border="2pt solid #000080"')), 'navy outline', True))
    pb.append(('r6p-F2o-1', 'pp-e2', edit(line_with(p2, 'name="Rounded Rectangle 12"'),
                                          line_with(p2, 'name="Rounded Rectangle 12"').replace(
                                              'border=none', 'border="2pt solid #000080"')), 'navy outline', True))
    ts = line_with(p1, 'Series A')
    pb.append(('r6p-F1-1', 'pp-e5', edit(ts, ts.replace('Series A', 'Series B').replace('size=22pt', 'size=20pt')),
               'formatting changed with the text', False))
    # part C (section defaults, new styles)
    s1 = seed('r6s-F2s-1')
    d1 = line_with(s1, '<defaults line-spacing=170%')
    item = line_with(s1, '3D기술을 활용하면')
    cons = line_with(s1, '소비재, 의료')
    broken.append(('r6s-F2s-1', 'fn-q5', {'text': 'ANSWER: line-spacing=160%'}, 'the default style, not the defaults'))
    broken.append(('r6s-F2s-1', 'fn-q7', {'text': 'ANSWER: font=휴먼명조 size=15pt'}, 'another style\'s values'))
    broken.append(('r6s-F2s-1', 'fn-e12', edit(d1, d1.replace('170%', '200%')), 'the defaults line reaches others'))
    good.append(('r6s-F2s-1', 'fn-e12', edit(item, item.replace('indent-left=19pt', 'line-spacing=200% indent-left=19pt')),
                 'the item\'s own brace'))
    good.append(('r6s-F2s-1', 'fn-n1', edit(cons, cons[:-1] + ' line-spacing=170%}'), 'written out, unchanged'))
    broken.append(('r6s-F2s-1', 'fn-n1', edit(cons, cons[:-1] + ' line-spacing=160%}'), 'changed'))
    broken.append(('r6s-F2s-1', 'fn-n1', {'text': 'ANSWER: line-spacing=170%'}, 'neither unchanged nor an edit'))
    st4 = line_with(s1, '<style name="개요 4"')
    broken.append(('r6s-F2s-1', 'fn-e10', edit(st4, st4 + '\n<style name="개요 3"/>'), 'a duplicate name'))
    ga = line_with(s1, '(일반 접근성)')
    gb = line_with(s1, '(인력부족)')
    broken.append(('r6s-F2s-1', 'fn-e10', {'edits': [
        {'old': st4, 'new': st4 + '\n<style name="개요 3 강조" indent-left=20pt space-before=5pt font=휴먼명조 size=15pt/>'},
        {'old': ga, 'new': ga.replace('"개요 3"', '"개요 3 강조" color=#C00000')},
        {'old': gb, 'new': gb.replace('"개요 3"', '"개요 3 강조" color=#C00000')}]}, 'the colour on the items'))
    good.append(('r6s-F2s-1', 'fn-e10', {'edits': [
        {'old': st4, 'new': st4 + '\n<style name="개요 3 강조" indent-left=20pt space-before=5pt font=휴먼명조 '
                                  'size=15pt color=#C00000/>'},
        {'old': ga, 'new': ga.replace('"개요 3"', '"개요 3 강조"')},
        {'old': gb, 'new': gb.replace('"개요 3"', '"개요 3 강조"')}]}, 'the new style'))
    broken.append(('r6s-F2s-1', 'fn-e11', edit(st4, st4.replace('size=14pt', 'size=12pt')), 'should refuse'))
    s2 = seed('r6s-F2s-2')
    root = line_with(s2, '(근본 대책)')
    dl = [ln for ln in s2.split('\n') if ln.startswith('<defaults') and 'line-spacing=162%' in ln][0]
    broken.append(('r6s-F2s-2', 'mel-e12', edit(dl, dl.replace('space-before=20pt', 'space-before=12pt')),
                   'the defaults line reaches others'))
    good.append(('r6s-F2s-2', 'mel-e12', edit(root, root + ' {space-before=12pt}'), 'its own brace'))
    broken.append(('r6s-F2s-2', 'mel-q4', {'text': 'ANSWER: first-line=0pt'}, 'the default style, not the defaults'))
    broken.append(('r6s-F2s-2', 'mel-q7', {'text': 'ANSWER: line-spacing=130%'}, 'the defaults over the style'))
    for uid, tid, ans, why, want in pb:
        r = next(x for x in PK.score_answer(uid, {'answers': [dict(ans, task_id=tid)]}) if x['task_id'] == tid)
        if r['landed'] != want:
            fails.append(('pptx', uid, tid, why, r['flags'], r['error'] or r['detail']))
        else:
            print('  %s %-10s %-7s %-35s %s' % ('passes' if want else 'caught', uid, tid, why, r['flags']))
    r = next(x for x in PK.score_answer('r6p-F2o-1', {'answers': [{'task_id': 'pp-q1', 'text':
                                                                   'ANSWER: border="1pt solid #000000"'}]}))
    if 'unreachable_attempted' not in r['flags']:
        fails.append(('pptx', 'hidden read guessed', r['flags']))
    nb = ng = 0
    for uid, tid, ans, why in broken:
        r = one(uid, tid, ans)
        nb += 1
        if r['landed']:
            fails.append(('broken passed', uid, tid, why))
        else:
            print('  caught %-10s %-7s %-45s %s %s' % (uid, tid, why, r['flags'], (r['error'] or r['detail'])[:90]))
    for uid, tid, ans, why in good:
        r = one(uid, tid, ans)
        ng += 1
        ok = r['landed'] or 'unreachable_refused' in r['flags']
        if not ok:
            fails.append(('alternative failed', uid, tid, why, r['error'], r['detail']))
        else:
            print('  passes %-10s %-7s %-45s %s' % (uid, tid, why, r['method'] or ''))
    print('broken caught: %d/%d; alternatives pass: %d/%d' % (nb - sum(1 for f in fails if f[0] == 'broken passed'),
                                                            nb, ng - sum(1 for f in fails if f[0] ==
                                                                         'alternative failed'), ng))
    if fails:
        for f in fails:
            print('FAIL', f)
        sys.exit(1)
    print('SELFTEST PASSED')


if __name__ == '__main__':
    main()
