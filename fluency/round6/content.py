"""Round 6 part A (flow documents): the three seeds and their tasks, worded the same for every candidate.

Seeds are real corpus files: hanji's text of each (data/today/, from the engine's dump) with the file's formatting
attached (extract.py, doc.attach). A task names its targets in the seed; its check is on effective formatting, read
back from the answer under the candidate (doc.effective), plus a line check: every line outside the targets is
unchanged. `gold(m, cand)` changes a copy of the seed model the way a correct answer would; build.py renders it."""
import copy
import os
import re

import doc as D
import inline as I
import vocab as V

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..'))

SEEDS = {
    1: {'name': 'footnote-01', 'fmt': 'hwpx', 'path': 'crates/hanji-hwpx/corpus/footnote-01.hwpx'},
    2: {'name': 'mel-001', 'fmt': 'hwpx', 'path': 'crates/hanji-hwpx/corpus/mel-001.hwpx'},
    3: {'name': 'korean-report', 'fmt': 'docx', 'path': 'prototype/remainder/corpus/korean-report.docx'},
}


def today_path(rep):
    s = SEEDS[rep]
    return os.path.join(HERE, 'data', 'today', '%s.%s.txt' % (s['name'], s['fmt']))


_cache = {}


def seed_model(rep):
    if rep not in _cache:
        import extract as X
        s = SEEDS[rep]
        text = open(today_path(rep), encoding='utf-8').read()
        m = D.attach(text, X.extract(os.path.join(ROOT, s['path'])), s['fmt'])
        D.normalize(m)
        _cache[rep] = m
    return copy.deepcopy(_cache[rep])


# ================================================================ locating things in a model (non-Raw block index)

def nr(m):
    return [b for b in m.blocks if not isinstance(b, D.Raw)]


def banner(m, text):
    for i, b in enumerate(nr(m)):
        if isinstance(b, D.T):
            cells = list(b.cells())
            if len(cells) == 1 and text in ''.join(p.plain for p in cells[0][1].paras):
                return i
    raise KeyError(text)


def table(m, first_text, nth=0):
    k = 0
    for i, b in enumerate(nr(m)):
        if isinstance(b, D.T) and len(list(b.cells())) > 1:
            txt = ' '.join(p.plain for _, c in b.cells() for p in c.paras)
            if first_text in txt.replace(' ', '') or first_text in txt:
                if k == nth:
                    return i
                k += 1
    raise KeyError(first_text)


def para(m, start):
    for i, b in enumerate(nr(m)):
        if isinstance(b, D.P) and re.sub(r'\s+', ' ', b.plain.strip()).startswith(start):
            return i
    raise KeyError(start)


def paras(m, pred):
    return [i for i, b in enumerate(nr(m)) if isinstance(b, D.P) and b.plain.strip() and pred(b)]


def cell_paras(m, pred):
    out = []
    for i, b in enumerate(nr(m)):
        if isinstance(b, D.T):
            for rc, c in b.cells():
                for k, p in enumerate(c.paras):
                    if p.plain.strip() and pred(p):
                        out.append((i, rc, k))
    return out


def first_row_cells(m, i):
    b = nr(m)[i]
    return [(i, rc) for rc, c in b.cells() if rc[0] == 0]


# ================================================================ colour predicates (a request in words)

def is_light_blue(c):
    if not c or not c.startswith('#'):
        return False
    h, l, s = V.hls(c)
    return 180 <= h <= 250 and l >= 0.65 and s >= 0.15


def is_red(c):
    if not c or not c.startswith('#'):
        return False
    h, l, s = V.hls(c)
    return (h <= 15 or h >= 345) and s >= 0.5 and 0.2 <= l <= 0.65


def exact(v):
    return lambda x: x is not None and x.upper() == v.upper()


def border_color(pred, like):
    """The border keeps width and style (those of `like`) and takes a colour pred accepts."""
    w, st, _ = V.border_parts(like)

    def f(b):
        p = V.border_parts(b)
        return p is not None and abs(p[0] - w) < 0.05 and p[1] == st and pred(p[2])
    return f


def length_is(v):
    return lambda x: x is not None and x.endswith('pt') and abs(float(x[:-2]) - v) <= 0.5


# ================================================================ model edits for gold answers

def set_cell(m, i, rc, **box):
    c = dict(nr(m)[i].cells())[rc]
    for k, v in box.items():
        c.box[k.replace('_', '-')] = v


def set_para(p, para=None, char=None):
    for k, v in (para or {}).items():
        p.para[k] = v
    if char:
        p.runs = [(t, dict(pr, **char)) for t, pr in p.runs]


def make_bold(m, p):
    if all(pr.get('bold') == 'yes' for t, pr in p.runs if t.strip()):
        return
    p.inline = '**' + p.inline.replace('**', '') + '**'
    D.normalize_flags(m, p)


def edit_style(m, name, key, value):
    """A style's property changes, and so does every paragraph in it that had the style's value."""
    st = m.styles[name]
    part = 'para' if key in D.PKEYS else 'char'
    old = st[part].get(key)
    st[part][key] = value
    if name == m.default:
        for n, s in m.styles.items():
            if n != name and s[part].get(key) == old:
                s[part][key] = value
    for p in D.all_paras(m):
        pst = p.style or m.default
        if pst != name and not (name == m.default and m.styles[pst][part].get(key) == value):
            continue
        if part == 'para':
            if p.para.get(key) == old:
                p.para[key] = value
        else:
            p.runs = [(t, dict(pr, **{key: value}) if pr.get(key) == old else pr) for t, pr in p.runs]


def insert_after(m, i, blocks):
    b = nr(m)[i]
    k = m.blocks.index(b)
    m.blocks[k + 1:k + 1] = [D.Raw('')] + blocks


def new_table(m, header, rows, header_box):
    dflt = m.styles[m.default]

    def cp(text, bold=False):
        inl = '**%s**' % text if bold else text
        p = D.P('cell', '', m.default, inl)
        p.para = dict(dflt['para'])
        p.runs = [(p.plain, dict(dflt['char']))]
        D.normalize_flags(m, p)
        return p
    trows = []
    for r, cells in enumerate([header] + rows):
        row = []
        for text in cells:
            box = dict(m.box0)
            if r == 0:
                box.update(header_box)
            row.append(D.C(box, [cp(text, r == 0)]))
        trows.append(row)
    return D.T(None, trows)


# ================================================================ tasks

class Task:
    """kind: read | edit | refuse.
    read:  expect(m) -> {'type': 'props'|'cells'|'texts', 'value': ...}; refuse_in: candidates where the property
           is not in the text (the right answer is a refusal).
    edit:  targets(m) -> list of refs ('P', i) / ('C', i, rc) / ('CP', i, rc, k); change: {prop: value or pred};
           box: {prop: pred} for cells; text: (old, new) for a text change; styles: style lines may change;
           new: a check for one added table; gold(m, cand) -> changes m (None: the candidate cannot)."""

    def __init__(self, tid, kind, text, **kw):
        self.id, self.kind, self.text = tid, kind, text
        self.__dict__.update(kw)


def T_(tid, kind, text, **kw):
    return Task(tid, kind, text, **kw)


NAVY = '#1F3864'
DARKRED = '#C00000'
GREY = '#D9D9D9'


def tasks_1():
    """footnote-01.hwpx: a Korean report with a title banner, section header tables and outline lists."""
    t = []
    B = '3D 프린팅 기술의 미래와 전망'
    t.append(T_('fn-q1', 'read', 'What colour is the fill of the banner "%s"? Answer as `ANSWER: fill=<colour>`.' % B,
                expect=lambda m: {'type': 'props', 'value': {'fill': cellbox(m, banner(m, B))['fill']}},
                refuse_in=('F3',)))
    t.append(T_('fn-q2', 'read', 'Which table cells in the file have a bottom border thicker than 1pt? Answer '
                '`ANSWER: <cell text>; <cell text>` (the text of each such cell, or `none`).',
                expect=lambda m: {'type': 'cells', 'value': thick_bottom_cells(m, 1.0)}, refuse_in=('F3',)))
    t.append(T_('fn-q3', 'read', 'What are the left indent and the first-line indent of the paragraph "산업용 샘플을 '
                '찍어내던 것에서 발전해…"? Answer `ANSWER: indent-left=<pt> first-line=<pt>`.',
                expect=lambda m: {'type': 'props', 'value': pick(eff_para(m, para(m, '산업용 샘플을')),
                                                                 'indent-left', 'first-line')}))
    t.append(T_('fn-q4', 'read', 'What is the font size of the paragraph "3D 프린터로 만들 수 있는 물건은…"? Answer '
                '`ANSWER: size=<pt>`.',
                expect=lambda m: {'type': 'props', 'value': {'size': eff_char(m, para(m, '3D 프린터로 만들'))['size']}}))
    t.append(T_('fn-e1', 'edit', 'Change the fill of the banner "%s" to light blue.' % B,
                targets=lambda m: [('C', banner(m, B), (0, 0))], box={'fill': is_light_blue},
                gold=lambda m, c: None if c == 'F3' else set_cell(m, banner(m, B), (0, 0), fill='#DDEBF7')))
    t.append(T_('fn-e2', 'edit', 'Make the bottom border of the banner "%s" red, keeping its width and line style.'
                % B,
                targets=lambda m: [('C', banner(m, B), (0, 0))],
                box=lambda m: {'border-bottom': border_color(is_red, cellbox(m, banner(m, B))['border-bottom'])},
                gold=lambda m, c: None if c == 'F3' else set_cell(
                    m, banner(m, B), (0, 0), border_bottom=recolor(cellbox(m, banner(m, B))['border-bottom'],
                                                                  '#FF0000'))))
    t.append(T_('fn-e3', 'edit', 'In every table, make the cells of the first row bold with a grey fill (%s).' % GREY,
                targets=lambda m: all_first_rows(m), box={'fill': exact(GREY)}, change={'bold': 'yes'},
                gold=lambda m, c: None if c == 'F3' else gold_first_rows(m)))
    L1 = ('개념', '3D 프린팅 기술의 장점', '사회적 변화 예상', '문제점 분석', '기대효과', '제도적 대처 방안')
    t.append(T_('fn-e4', 'edit', 'Make the six top-level list items (%s) dark red (%s) and 16pt.'
                % (', '.join(L1), DARKRED),
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.kind == 'list' and p.level == 0)],
                change={'color': exact(DARKRED), 'size': length_is(16)}, styles=True,
                gold=lambda m, c: gold_paras(m, c, paras(m, lambda p: p.kind == 'list' and p.level == 0),
                                             {'color': DARKRED, 'size': '16pt'}, style='개요 2')))
    t.append(T_('fn-e5', 'edit', 'Change the style 개요 3 so that all of its paragraphs turn navy (%s).' % NAVY,
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.style == '개요 3')],
                change={'color': exact(NAVY)}, styles=True, consistency='개요 3',
                gold=lambda m, c: gold_paras(m, c, paras(m, lambda p: p.style == '개요 3'), {'color': NAVY},
                                             style='개요 3')))
    t.append(T_('fn-e6', 'edit', 'In the list item "시제품 제작 시간과 비용 절감…", change "시간" to "기간". Change '
                'nothing else: its formatting must stay exactly as it is.',
                targets=lambda m: [('P', para(m, '시제품 제작 시간과'))], retext=('시제품 제작 시간', '시제품 제작 기간'),
                gold=lambda m, c: gold_text(m, para(m, '시제품 제작 시간과'), '시제품 제작 시간', '시제품 제작 기간')))
    t.append(T_('fn-e7', 'edit', 'After the list item "저가의 장비 및 S/W 보급…", add a table with a header row '
                '구분 | 내용 whose cells have a grey fill (%s), and one row 교육 | 교육과정 신설.' % GREY,
                targets=lambda m: [], new={'after': lambda m: para(m, '저가의 장비'),
                                           'header': ['구분', '내용'], 'rows': [['교육', '교육과정 신설']],
                                           'fill': exact(GREY)},
                gold=lambda m, c: None if c == 'F3' else insert_after(
                    m, para(m, '저가의 장비'), [new_table(m, ['구분', '내용'], [['교육', '교육과정 신설']],
                                                     {'fill': GREY})])))
    t.append(T_('fn-e8', 'edit', 'Indent the first line of every body paragraph (the second- and third-level list '
                'items) by 10pt.',
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.kind == 'list' and p.level in (1, 2))],
                change_para={'first-line': length_is(10)}, styles=True,
                gold=lambda m, c: gold_paras(m, c, paras(m, lambda p: p.kind == 'list' and p.level in (1, 2)),
                                             {'first-line': '10pt'}, style=('개요 3', '개요 4'))))
    t.append(T_('fn-e9', 'refuse', 'Give the banner "%s" a gradient fill that runs from yellow to orange.' % B))
    return t


def tasks_2():
    """mel-001.hwpx: a 19-page Korean ministry work report (고용노동부 업무보고), 1,717 paragraphs, 44 tables."""
    t = []
    BQ = '노동이 존중받는 일터'
    BE = '기금형 퇴직연금 활성화'
    t.append(T_('mel-q1', 'read', 'What colour is the fill of the banner "%s"? Answer `ANSWER: fill=<colour>`.' % BQ,
                expect=lambda m: {'type': 'props', 'value': {'fill': cellbox(m, banner(m, BQ))['fill']}},
                refuse_in=('F3',)))
    t.append(T_('mel-q2', 'read', 'In the budget table (예산 현황, the table whose first cell is "구  분"), which cells '
                'have a double bottom border? Answer `ANSWER: <cell text>; <cell text>`.',
                expect=lambda m: {'type': 'cells', 'value': double_bottom_cells(m, table(m, '2025 예산 (A)'))},
                refuse_in=('F3',)))
    t.append(T_('mel-q3', 'read', 'In the staff table (인원 현황), what are the alignment and the line spacing of the '
                'figure 7,850? Answer `ANSWER: align=<value> line-spacing=<value>`.',
                expect=lambda m: {'type': 'props', 'value': pick(eff_cellpara(m, '7,850'), 'align', 'line-spacing')}))
    t.append(T_('mel-q4', 'read', 'What is the first-line indent of the paragraph that begins "ㅇ (노동절 입법)"? '
                'Answer `ANSWER: first-line=<pt>`.',
                expect=lambda m: {'type': 'props', 'value': pick(eff_para(m, para(m, 'ㅇ (노동절 입법)')),
                                                                 'first-line')}, refuse_in=('F3',)))
    t.append(T_('mel-e1', 'edit', 'Change the fill of the banner "%s…" to light blue.' % BE,
                targets=lambda m: [('C', banner(m, BE), (0, 0))], box={'fill': is_light_blue},
                gold=lambda m, c: None if c == 'F3' else set_cell(m, banner(m, BE), (0, 0), fill='#DDEBF7')))
    t.append(T_('mel-e2', 'edit', 'Make the bottom border of the banner "%s…" red, keeping its width and line style.'
                % BE,
                targets=lambda m: [('C', banner(m, BE), (0, 0))],
                box=lambda m: {'border-bottom': border_color(is_red, cellbox(m, banner(m, BE))['border-bottom'])},
                gold=lambda m, c: None if c == 'F3' else set_cell(
                    m, banner(m, BE), (0, 0), border_bottom=recolor(cellbox(m, banner(m, BE))['border-bottom'],
                                                                   '#FF0000'))))
    t.append(T_('mel-e3', 'edit', 'In every schedule table (the small tables whose first row is 1분기 | 2분기 | 3분기 | '
                '4분기), make the cells of the first row bold with a grey fill (%s).' % GREY,
                targets=lambda m: schedule_first_rows(m), box={'fill': exact(GREY)}, change={'bold': 'yes'},
                gold=lambda m, c: None if c == 'F3' else gold_cells(m, schedule_first_rows(m))))
    H = ('1. ’25년 성과', '2. 정책 보완점', '1. 추진방향', '2. 추진전략')
    t.append(T_('mel-e4', 'edit', 'Make the four numbered headings (%s) dark red (%s) and 18pt.'
                % (', '.join('"%s…"' % h for h in H), DARKRED),
                targets=lambda m: [('P', para(m, h)) for h in H],
                change={'color': exact(DARKRED), 'size': length_is(18)}, styles=True,
                gold=lambda m, c: None if c == 'F3' else gold_paras(m, c, [para(m, h) for h in H],
                                                                     {'color': DARKRED, 'size': '18pt'})))
    t.append(T_('mel-e5', 'edit', 'Change the style 표가운데 so that all of its paragraphs turn navy (%s).' % NAVY,
                targets=lambda m: [('CP',) + x for x in cell_paras(m, lambda p: p.style == '표가운데')],
                change={'color': exact(NAVY)}, styles=True, consistency='표가운데',
                gold=lambda m, c: gold_cellparas(m, c, cell_paras(m, lambda p: p.style == '표가운데'),
                                                 {'color': NAVY}, style='표가운데')))
    t.append(T_('mel-e6', 'edit', 'In the paragraph that begins "ㅇ (현장 밀착관리)", change "1,200명" to "1,300명". Change '
                'nothing else: its formatting must stay exactly as it is.',
                targets=lambda m: [('P', para(m, 'ㅇ (현장 밀착관리)'))], retext=('1,200명', '1,300명'),
                gold=lambda m, c: gold_text(m, para(m, 'ㅇ (현장 밀착관리)'), '1,200명', '1,300명')))
    t.append(T_('mel-e7', 'edit', 'After the paragraph that begins "□ (미래세대 청년 고용 불안)", add a table with a '
                'header row 구분 | 내용 whose cells have a grey fill (%s), and one row 청년 | 일자리 첫걸음 보장제.'
                % GREY,
                targets=lambda m: [], new={'after': lambda m: para(m, '□ (미래세대 청년 고용 불안)'),
                                           'header': ['구분', '내용'], 'rows': [['청년', '일자리 첫걸음 보장제']],
                                           'fill': exact(GREY)},
                gold=lambda m, c: None if c == 'F3' else insert_after(
                    m, para(m, '□ (미래세대 청년 고용 불안)'),
                    [new_table(m, ['구분', '내용'], [['청년', '일자리 첫걸음 보장제']], {'fill': GREY})])))
    t.append(T_('mel-e8', 'edit', 'Set the first-line indent of every paragraph that begins with "□" to 10pt.',
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.plain.lstrip().startswith('□'))],
                change_para={'first-line': length_is(10)}, styles=True,
                gold=lambda m, c: None if c == 'F3' else gold_paras(
                    m, c, paras(m, lambda p: p.plain.lstrip().startswith('□')), {'first-line': '10pt'})))
    t.append(T_('mel-e9', 'refuse', 'Change the gradient of the banner "Ⅱ. ’25년 평가 및 향후 업무추진방향" so it runs '
                'from red to blue.'))
    return t


def tasks_3():
    """korean-report.docx: a short Korean sales report in Word styles (Heading 1/2, Note, Block Quotation)."""
    t = []
    t.append(T_('kr-q1', 'read', 'What colour is the shading (fill) of the note paragraph "신규 고객 34곳 중 21곳이 '
                '수도권."? Answer `ANSWER: fill=<colour>`.',
                expect=lambda m: {'type': 'props', 'value': pick(eff_para(m, para(m, '신규 고객 34곳 중')), 'fill')}))
    t.append(T_('kr-q2', 'read', 'Which words or phrases in the file are coloured %s? Answer `ANSWER: <text>; <text>`.'
                % DARKRED, expect=lambda m: {'type': 'texts', 'value': colored_runs(m, DARKRED)}, refuse_in=('F3',)))
    t.append(T_('kr-q3', 'read', 'What is the left indent of the quotation "“고객이 있는 곳에 지점이 있어야 한다.”…"? '
                'Answer `ANSWER: indent-left=<pt>`.',
                expect=lambda m: {'type': 'props', 'value': pick(eff_para(m, para(m, '“고객이')), 'indent-left')}))
    t.append(T_('kr-q4', 'read', 'What is the font size of the text "15% 성장"? Answer `ANSWER: size=<pt>`.',
                expect=lambda m: {'type': 'props', 'value': {'size': run_prop(m, '15% 성장', 'size')}},
                refuse_in=('F3',)))
    t.append(T_('kr-e1', 'edit', 'Change the shading (fill) of the note paragraph "신규 고객 34곳 중 21곳이 수도권." '
                'to light blue.',
                targets=lambda m: [('P', para(m, '신규 고객 34곳 중'))], change_para={'fill': is_light_blue},
                styles=True,
                gold=lambda m, c: gold_paras(m, c, [para(m, '신규 고객 34곳 중')], {'fill': '#DDEBF7'}, style='Note')))
    t.append(T_('kr-e2', 'edit', 'Give the cells of the table\'s last row (합계 | 215) a red bottom border, 1.5pt solid.',
                targets=lambda m: last_row_cells(m, table(m, '지역')),
                box={'border-bottom': lambda b: b is not None and b != 'none' and is_red(b.split()[2])
                     and abs(V.border_width(b) - 1.5) < 0.05 and b.split()[1] == 'solid'},
                gold=lambda m, c: None if c == 'F3' else [set_cell(m, i, rc, border_bottom='1.5pt solid #FF0000')
                                                          for _, i, rc in last_row_cells(m, table(m, '지역'))]))
    t.append(T_('kr-e3', 'edit', 'Make the cells of the table\'s header row (지역 | 지점 | 매출) bold with a grey fill (%s).'
                % GREY,
                targets=lambda m: [('C', i, rc) for i, rc in first_row_cells(m, table(m, '지역'))],
                box={'fill': exact(GREY)}, change={'bold': 'yes'},
                gold=lambda m, c: None if c == 'F3' else gold_cells(
                    m, [('C', i, rc) for i, rc in first_row_cells(m, table(m, '지역'))])))
    t.append(T_('kr-e4', 'edit', 'Make every heading (the # and ## lines) dark red (%s).' % DARKRED,
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.kind == 'heading')],
                change={'color': exact(DARKRED)}, styles=True,
                gold=lambda m, c: gold_paras(m, c, paras(m, lambda p: p.kind == 'heading'), {'color': DARKRED},
                                             style=('Heading 1', 'Heading 2'))))
    t.append(T_('kr-e5', 'edit', 'Change the style Heading 2 so that all Heading 2 paragraphs turn navy (%s).' % NAVY,
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.style == 'Heading 2')],
                change={'color': exact(NAVY)}, styles=True, consistency='Heading 2',
                gold=lambda m, c: gold_paras(m, c, paras(m, lambda p: p.style == 'Heading 2'), {'color': NAVY},
                                             style='Heading 2')))
    t.append(T_('kr-e6', 'edit', 'Change "15% 성장" to "16% 성장". Change nothing else: its formatting must stay exactly '
                'as it is.',
                targets=lambda m: [('P', para(m, '지방 지점의 매출은'))], retext=('15% 성장', '16% 성장'),
                gold=lambda m, c: gold_text(m, para(m, '지방 지점의 매출은'), '15% 성장', '16% 성장')))
    t.append(T_('kr-e7', 'edit', 'After the paragraph "4분기에는 부산과 대구에 지점을 연다.", add a table with a header '
                'row 도시 | 개점 whose cells have a grey fill (%s), and two rows 부산 | 10월 and 대구 | 11월.' % GREY,
                targets=lambda m: [], new={'after': lambda m: para(m, '4분기에는'),
                                           'header': ['도시', '개점'], 'rows': [['부산', '10월'], ['대구', '11월']],
                                           'fill': exact(GREY)},
                gold=lambda m, c: None if c == 'F3' else insert_after(
                    m, para(m, '4분기에는'), [new_table(m, ['도시', '개점'], [['부산', '10월'], ['대구', '11월']],
                                                    {'fill': GREY})])))
    t.append(T_('kr-e8', 'edit', 'Indent the first line of every body paragraph by 10pt: the plain paragraphs of the '
                'text, not the headings, the note, the quotation or the table.',
                targets=lambda m: [('P', i) for i in paras(m, lambda p: p.kind == 'plain')],
                change_para={'first-line': length_is(10)}, styles=True,
                gold=lambda m, c: None if c == 'F3' else gold_paras(
                    m, c, paras(m, lambda p: p.kind == 'plain'), {'first-line': '10pt'})))
    t.append(T_('kr-e9', 'refuse', 'Fill the header row of the table with a diagonal-stripe pattern.'))
    return t


TASKS = {1: tasks_1(), 2: tasks_2(), 3: tasks_3()}


# ================================================================ helpers for expectations and gold

def cellbox(m, i, rc=(0, 0)):
    return dict(nr(m)[i].cells())[rc].box


def eff_para(m, i):
    return nr(m)[i].para


def eff_char(m, i):
    return D.lifted(nr(m)[i].runs)


def eff_cellpara(m, text):
    for b in nr(m):
        if isinstance(b, D.T):
            for _, c in b.cells():
                for p in c.paras:
                    if p.plain.strip() == text:
                        return p.para
    raise KeyError(text)


def pick(d, *keys):
    return {k: d.get(k) for k in keys}


def thick_bottom_cells(m, w):
    out = []
    for b in nr(m):
        if isinstance(b, D.T):
            for _, c in b.cells():
                if V.border_width(c.box.get('border-bottom')) > w:
                    out.append(' '.join(p.plain for p in c.paras).strip())
    return out


def double_bottom_cells(m, i):
    out = []
    for _, c in nr(m)[i].cells():
        p = V.border_parts(c.box.get('border-bottom'))
        if p and p[1] == 'double':
            out.append(' '.join(x.plain for x in c.paras).strip())
    return out


def colored_runs(m, col):
    out = []
    for p in D.all_paras(m):
        for t, pr in p.runs:
            if pr.get('color') == col and t.strip():
                out.append(t.strip())
    return out


def run_prop(m, text, key):
    for p in D.all_paras(m):
        for t, pr in p.runs:
            if text in t:
                return pr.get(key)
    raise KeyError(text)


def recolor(b, col):
    w, st, _ = V.border_parts(b)
    return '%s %s %s' % (V.pt(w), st, col)


def all_first_rows(m):
    out = []
    for i, b in enumerate(nr(m)):
        if isinstance(b, D.T):
            out += [('C', i, rc) for rc, _ in b.cells() if rc[0] == 0]
    return out


def schedule_first_rows(m):
    out = []
    for i, b in enumerate(nr(m)):
        if isinstance(b, D.T):
            first = [c for rc, c in b.cells() if rc[0] == 0]
            if [' '.join(p.plain for p in c.paras).strip() for c in first] == ['1분기', '2분기', '3분기', '4분기']:
                out += [('C', i, rc) for rc, _ in b.cells() if rc[0] == 0]
    return out


def last_row_cells(m, i):
    b = nr(m)[i]
    last = max(rc[0] for rc, _ in b.cells())
    return [('C', i, rc) for rc, _ in b.cells() if rc[0] == last]


def gold_first_rows(m):
    gold_cells(m, all_first_rows(m))


def gold_cells(m, refs):
    for _, i, rc in refs:
        c = dict(nr(m)[i].cells())[rc]
        c.box['fill'] = GREY
        for p in c.paras:
            if p.plain.strip():
                make_bold(m, p)


def gold_paras(m, cand, idxs, props, style=None):
    """Direct formatting on each paragraph; under F2/F3 through the named style(s) when given (the style's
    paragraphs are exactly the targets). F3 has only the style route."""
    styles = (style,) if isinstance(style, str) else (style or ())
    if cand in ('F2', 'F3') and styles:
        for s in styles:
            for k, v in props.items():
                edit_style(m, s, k, v)
        return m
    if cand == 'F3':
        return None
    for i in idxs:
        p = nr(m)[i]
        set_para(p, {k: v for k, v in props.items() if k in D.PKEYS},
                 {k: v for k, v in props.items() if k not in D.PKEYS})
    return m


def gold_cellparas(m, cand, refs, props, style=None):
    if cand in ('F2', 'F3') and style:
        for k, v in props.items():
            edit_style(m, style, k, v)
        return m
    for i, rc, k in refs:
        p = dict(nr(m)[i].cells())[rc].paras[k]
        set_para(p, {k2: v for k2, v in props.items() if k2 in D.PKEYS},
                 {k2: v for k2, v in props.items() if k2 not in D.PKEYS})
    return m


def gold_text(m, i, old, new):
    p = nr(m)[i]
    assert len(old) == len(new)
    assert old in p.inline, (old, p.inline)
    p.inline = p.inline.replace(old, new, 1)
    pos = p.plain.index(old)
    p.plain = p.plain.replace(old, new, 1)
    flat = []
    for t, pr in p.runs:
        flat += [(ch, pr) for ch in t]
    flat = [(new[j - pos], pr) if pos <= j < pos + len(new) else (ch, pr) for j, (ch, pr) in enumerate(flat)]
    runs = []
    for ch, pr in flat:
        if runs and runs[-1][1] == pr:
            runs[-1] = (runs[-1][0] + ch, pr)
        else:
            runs.append((ch, pr))
    p.runs = runs
    return m
