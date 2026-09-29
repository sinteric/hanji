"""Round 5 seeds, tasks and checks, written once in the semantic model of deck.py (EMU), for every candidate.

Seeds 1 and 2 are the two pptx decks validation/office-kit uses (crates/hanji-pptx/corpus/korean-deck.pptx and
shapes.pptx): every object, its text and its geometry as the file stores it (read with python-pptx, 2026-09-29),
including the connectors and textless shapes today's text does not show. Seed 3 is written for this round: a
16:9 Korean product deck on the stock Office 16:9 layouts, with an overridden title box, a rotated shape, a
group holding lines, a chart and two logos with the same summary.
"""
from deck import PT, CM, flat, obj_box, okey, norm_para, norm_text

TOL = 2 * PT   # geometry within 2 pt (0.7 mm) counts as exact

# ---------------------------------------------------------------- layouts

T43 = (457200, 274638, 8229600, 1143000)
LAYOUTS_43 = {
    'Title Slide': [('title', (685800, 2130425, 7772400, 1470025)), ('subtitle', (1371600, 3886200, 6400800, 1752600))],
    'Title and Content': [('title', T43), ('body', (457200, 1600200, 8229600, 4525963))],
    'Section Header': [('title', (722313, 4406900, 7772400, 1362075)), ('body', (722313, 2906713, 7772400, 1500187))],
    'Two Content': [('title', T43), ('left', (457200, 1600200, 4038600, 4525963)),
                    ('right', (4648200, 1600200, 4038600, 4525963))],
    'Comparison': [('title', T43), ('body', (457200, 1535113, 4040188, 639762)),
                   ('body2', (457200, 2174875, 4040188, 3951288)), ('body3', (4645025, 1535113, 4041775, 639762)),
                   ('body4', (4645025, 2174875, 4041775, 3951288))],
    'Title Only': [('title', T43)],
    'Blank': [],
    'Content with Caption': [('title', (457200, 273050, 3008313, 1162050)),
                             ('right', (3575050, 273050, 5111750, 5853113)),
                             ('left', (457200, 1435100, 3008313, 4691063))],
    'Picture with Caption': [('title', (1792288, 4800600, 5486400, 566738)),
                             ('picture', (1792288, 612775, 5486400, 4114800)),
                             ('body', (1792288, 5367338, 5486400, 804862))],
    'Title and Vertical Text': [('title', T43), ('body', (457200, 1600200, 8229600, 4525963))],
    'Vertical Title and Text': [('title', (6629400, 274638, 2057400, 5851525)),
                                ('body', (457200, 274638, 6019800, 5851525))],
}

T169 = (838200, 365125, 10515600, 1325563)
LAYOUTS_169 = {
    'Title Slide': [('title', (1524000, 1122363, 9144000, 2387600)), ('subtitle', (1524000, 3602038, 9144000, 1655762))],
    'Title and Content': [('title', T169), ('body', (838200, 1825625, 10515600, 4351338))],
    'Section Header': [('title', (831850, 1709738, 10515600, 2852737)), ('body', (831850, 4589463, 10515600, 1500187))],
    'Two Content': [('title', T169), ('left', (838200, 1825625, 5181600, 4351338)),
                    ('right', (6172200, 1825625, 5181600, 4351338))],
    'Comparison': [('title', (839788, 365125, 10515600, 1325563)), ('body', (839788, 1681163, 5157787, 823912)),
                   ('body2', (839788, 2505075, 5157787, 3684588)), ('body3', (6172200, 1681163, 5183188, 823912)),
                   ('body4', (6172200, 2505075, 5183188, 3684588))],
    'Title Only': [('title', T169)],
    'Blank': [],
    'Content with Caption': [('title', (839788, 457200, 3932237, 1600200)),
                             ('right', (5183188, 987425, 6172200, 4873625)),
                             ('left', (839788, 2057400, 3932237, 3811588))],
    'Picture with Caption': [('title', (839788, 457200, 3932237, 1600200)),
                             ('picture', (5183188, 987425, 6172200, 4873625)),
                             ('body', (839788, 2057400, 3932237, 3811588))],
}


def slot(name, paras, box=None, rot=0):
    return {'t': 'slot', 'slot': name, 'box': box, 'rot': rot, 'paras': list(paras)}


def shape(id_, name, box, paras=(), rot=0, flip=None):
    return {'t': 'shape', 'id': id_, 'name': name, 'box': box, 'rot': rot, 'flip': flip, 'paras': list(paras)}


def keep(id_, kind, summary, box, rot=0):
    return {'t': 'keep', 'id': id_, 'kind': kind, 'summary': summary, 'src': None, 'box': box, 'rot': rot}


def line(id_, name, a, b):
    return {'t': 'line', 'id': id_, 'name': name, 'a': a, 'b': b}


def group(id_, name, children):
    return {'t': 'group', 'id': id_, 'name': name, 'children': children}


def slide(layout, objs, notes=None):
    return {'layout': layout, 'objs': objs, 'notes': notes, 'src': None}


# ---------------------------------------------------------------- seed 1: korean-deck.pptx (real)

DECK1 = {
    'file': 'korean-deck.pptx', 'fm': [('type', 'presentation'), ('format', 'pptx'), ('schema', '1')],
    'W': 9144000, 'H': 6858000, 'layouts': LAYOUTS_43,
    'slides': [
        slide('Title Slide', [slot('title', ['3분기 영업 보고']), slot('subtitle', ['영업본부 · 2026년 10월'])],
              ['인사말 후 목차를 소개한다.']),
        slide('Title and Content', [slot('title', ['핵심 지표']),
                                    slot('body', ['- 매출 **12% 증가**', '- 신규 고객 34곳', '  - 수도권 21곳',
                                                  '  - 지방 13곳', '- 영업이익률 8.4%'])],
              ['전년 대비 증가폭을 강조한다.']),
        slide('Two Content', [slot('title', ['지역별 현황']), slot('left', ['- 수도권 21곳', '  - 서울 14곳']),
                              slot('right', ['- 지방 13곳', '  - 부산 5곳']),
                              shape('s5', '출처', (457200, 6035040, 3657600, 365760), ['출처: 내부 집계'])]),
        slide('Title Only', [slot('title', ['분기별 매출']),
                             keep('kb2br', 'table', 'Table 2: 분기 매출 증감 1분기 1,120 +3% 2분기 1,180 +5% 3분기 1,204 +12%',
                                  (914400, 1828800, 7315200, 1828800)),
                             keep('k04k9', 'picture', 'image.png', (7315200, 5029200, 914400, 457200))],
              ['표는 잠정치이며 10월 말 확정된다.']),
        slide('Title and Content', [slot('title', ['다음 분기 계획']),
                                    slot('body', ['1. 신규 지점 3곳 개설', '1. 온라인 채널 확대', '1. 고객 만족도 조사',
                                                  '- 자세한 일정은 사내 게시판 참고 (긴급)'])]),
        slide('Section Header', [slot('title', ['부록']), slot('body', ['세부 자료'])]),
        slide('Title and Content', [slot('title', ['감사합니다'])]),
    ]}

# ---------------------------------------------------------------- seed 2: shapes.pptx (real, Apache POI test data)

DECK2 = {
    'file': 'shapes.pptx', 'fm': [('type', 'presentation'), ('format', 'pptx'), ('schema', '1')],
    'W': 9144000, 'H': 6858000, 'layouts': LAYOUTS_43,
    'slides': [
        slide('Blank', [
            shape('s4', 'TextBox 3', (914400, 914400, 2286000, 369332), ['Learning PPTX']),
            line('s6', 'Straight Connector 5', (1066800, 1828800), (3200400, 1828800)),
            shape('s7', 'Freeform 6', (595961, 2674961, 2353994, 1733266), ['Cloud']),
            keep('kqld8', 'picture', 'Picture 1', (5105400, 990600, 1828800, 1676400)),
            keep('kmtnf', 'table', 'Table 2: Column1 Column2 Column3 data1 data2 data3',
                 (3810000, 4724400, 4724400, 1219200)),
            line('s8', 'Straight Arrow Connector 7', (5943600, 4648200), (5943600, 2743200)),
            line('s10', 'Elbow Connector 9', (2362200, 3200400), (5105400, 1828800)),
        ]),
        slide('Title Slide', [slot('title', ['PPTX <u>Title</u>']),
                              slot('subtitle', ['**Subtitle**', '', 'And second line'])]),
        slide('Blank', [group('g5', 'Group 4', [
            shape('s2', 'Rectangle 1', (1524000, 1371600, 1752600, 762000)),
            shape('s3', 'Oval 2', (3886200, 1905000, 914400, 914400)),
            shape('s4', 'Right Arrow 3', (1828800, 2819400, 978408, 484632)),
        ])]),
        slide('Blank', [keep('k5mk0', 'table',
                             'Table 1: header1 header2 header3 A1 B1 C1 A2 A3 B3 and C3 are merged A4 A5',
                             (1524000, 1397000, 6096000, 2225040))]),
        slide('Title Only', [keep('k4akl', 'table',
                                  'Table 1: Link Type Target URI Web Page http://poi.apache.org/ Place in this docu…',
                                  (685800, 2133600, 7848600, 1854200)),
                             slot('title', ['Hyperlinks'])]),
        slide('Blank', [shape('s2', 'Rectangle 1', (838200, 914400, 914400, 914400)),
                        shape('s9', 'Rectangle 8', (2209800, 914400, 914400, 914400)),
                        shape('s10', 'Rectangle 9', (3429000, 914400, 914400, 914400)),
                        shape('s11', 'Rectangle 10', (4648200, 914400, 914400, 914400)),
                        shape('s12', 'Rectangle 11', (5943600, 914400, 914400, 914400)),
                        shape('s13', 'Rectangle 12', (7543800, 914400, 914400, 914400))]),
    ]}

# ---------------------------------------------------------------- seed 3: a 16:9 Korean product deck (written for round 5)

LOGO = (10972800, 228600, 990600, 495300)
DECK3 = {
    'file': 'product-deck (synthetic)',
    'fm': [('type', 'presentation'), ('format', 'pptx'), ('template', 'org/product-deck'), ('schema', '1')],
    'W': 12192000, 'H': 6858000, 'layouts': LAYOUTS_169,
    'slides': [
        slide('Title Slide', [slot('title', ['2027 신제품 출시 계획']), slot('subtitle', ['마케팅본부 · 2026년 11월']),
                              keep('k3ftw', 'picture', 'logo.png', LOGO)],
              ['일정과 수치는 내부 검토용']),
        slide('Title and Content', [
            slot('title', ['시장 현황'], box=(838200, 365125, 9601200, 1325563)),
            slot('body', ['- 국내 스마트홈 시장 4.2조 원 (전년 대비 +12%)', '- 주요 경쟁사 3곳 신제품 출시',
                          '  - A사: 2027년 3월', '  - B사: 2027년 5월', '- 1인 가구 비중 34%']),
            shape('s4', 'NEW 배지', (9448800, 1600200, 1143000, 457200), ['NEW'], rot=15),
            keep('k7hqa', 'picture', 'logo.png', LOGO)]),
        slide('Title Only', [
            slot('title', ['출시 일정']),
            group('g5', '타임라인', [
                shape('s6', '기획 단계', (1219200, 2286000, 2438400, 914400), ['기획']),
                line('s7', '화살표 1', (3657600, 2743200), (4572000, 2743200)),
                shape('s8', '개발 단계', (4572000, 2286000, 2438400, 914400), ['개발']),
                line('s9', '화살표 2', (7010400, 2743200), (7924800, 2743200)),
                shape('s10', '출시 단계', (7924800, 2286000, 2438400, 914400), ['출시']),
            ]),
            shape('s11', '주석', (1219200, 4572000, 6096000, 369332), ['일정은 내부 검토 중이며 변동될 수 있음'])],
            ['각 단계는 분기 단위']),
        slide('Two Content', [slot('title', ['제품 비교']),
                              slot('left', ['- 스마트 허브 S1', '  - 가격 19만 9천 원', '  - 음성 인식 지원']),
                              slot('right', ['- 스마트 허브 S1 Pro', '  - 가격 29만 9천 원', '  - 음성 인식 지원',
                                             '  - 카메라 내장'])],
              ['가격은 부가세 포함']),
        slide('Title Only', [slot('title', ['예상 매출']),
                             keep('k2m8c', 'chart', 'Chart 3: 분기별 예상 매출 (억 원)', (1524000, 2133600, 6096000, 3810000)),
                             shape('s13', '1분기 라벨', (8001000, 2286000, 2286000, 369332), ['1분기 120억']),
                             shape('s14', '2분기 라벨', (8128000, 2971800, 2286000, 369332), ['2분기 180억'])]),
        slide('Section Header', [slot('title', ['부록']), slot('body', ['세부 자료'])]),
    ]}

SEEDS = {1: DECK1, 2: DECK2, 3: DECK3}
FILENAMES = {1: 'korean-deck.hj.md', 2: 'shapes.hj.md', 3: 'product-deck.hj.md'}


def deck_copy(d):
    import copy
    return copy.deepcopy({k: v for k, v in d.items() if k != 'file'})


# ---------------------------------------------------------------- check helpers

def lay_box(deck, layout, name):
    return dict(deck['layouts'][layout])[name]


def res_slide(res, sidx):
    for s in res['slides']:
        if s.get('src') == sidx:
            return s
    return None


def find(slide_, key):
    for o in flat(slide_['objs']):
        if okey(o) == key:
            return o
    return None


def box_of(deck, slide_, o):
    return obj_box(o, deck['layouts'], slide_['layout'])


def shallow(o):
    if o['t'] == 'group':
        return ('group', o.get('id'), o.get('name'), tuple(okey(c) for c in o['children']))
    from deck import canon_obj
    return canon_obj(o)


def others_same(seed, res, changed=(), new_objs=None, new_slide_at=None):
    """Everything not named in `changed` ({(slide index, key)}) is exactly as in the seed. new_objs: {slide index:
    count} of new objects allowed; new_slide_at: the result index of the one new slide allowed."""
    new_objs = new_objs or {}
    changed = set(changed)
    srcs = [s.get('src') for s in res['slides']]
    want = list(range(len(seed['slides'])))
    if new_slide_at is not None:
        want = want[:new_slide_at] + [None] + want[new_slide_at:]
    if srcs != want:
        return 'the slides are not the seed\'s slides in order (%s)' % srcs
    for si, ss in enumerate(seed['slides']):
        rs = res_slide(res, si)
        if rs['layout'] != ss['layout']:
            return 'slide %d: layout changed' % (si + 1)
        if (rs.get('notes') is None) != (ss.get('notes') is None) or (
                ss.get('notes') is not None and [norm_para(p) for p in rs['notes']] != [norm_para(p) for p in ss['notes']]):
            return 'slide %d: notes changed' % (si + 1)
        skeys = [okey(o) for o in flat(ss['objs'])]
        rkeys = [okey(o) for o in flat(rs['objs'])]
        new = [k for k in rkeys if k not in skeys or k[1] is None]
        if len(new) != new_objs.get(si, 0):
            return 'slide %d: %d new object(s), expected %d' % (si + 1, len(new), new_objs.get(si, 0))
        kept = [k for k in rkeys if k in skeys and k[1] is not None]
        if kept != [k for k in skeys if k in kept] or set(kept) != set(skeys):
            missing = [k for k in skeys if k not in kept]
            return 'slide %d: objects deleted or reordered (%s)' % (si + 1, missing or 'order')
        for o in flat(ss['objs']):
            k = okey(o)
            if (si, k) in changed:
                continue
            ro = find(rs, k)
            if shallow(ro) != shallow(o):
                return 'slide %d: %s %s changed' % (si + 1, k[0], k[1])
    return None


def near(a, b):
    return abs(a - b) <= TOL


def box_is(got, want, what):
    names = ('left', 'top', 'width', 'height')
    for n, g, w in zip(names, got, want):
        if w is not None and not near(g, w):
            return '%s: %s is %.1f pt, expected %.1f pt' % (what, n, g / PT, w / PT)
    return None


def overlaps(b1, b2):
    ix = min(b1[0] + b1[2], b2[0] + b2[2]) - max(b1[0], b2[0])
    iy = min(b1[1] + b1[3], b2[1] + b2[3]) - max(b1[1], b2[1])
    return ix > TOL and iy > TOL


# ---------------------------------------------------------------- task makers
# Each edit task: gold(seed) -> gold deck (exact EMU), check(seed, res) -> None (landed) or the first difference.

def t_geom(sidx, key, want_fn, what):
    def gold(seed):
        d = deck_copy(seed)
        o = find(d['slides'][sidx], key)
        o['box'] = tuple(int(round(v)) for v in want_fn(seed))
        return d

    def check(seed, res):
        err = others_same(seed, res, changed={(sidx, key)} | parents(seed, sidx, key))
        if err:
            return err
        o = find(res_slide(res, sidx), key)
        return box_is(o['box'], want_fn(seed), what)
    return gold, check


def parents(seed, sidx, key):
    out = set()
    for o in seed['slides'][sidx]['objs']:
        if o['t'] == 'group' and any(okey(c) == key for c in flat(o['children'])):
            out.add((sidx, okey(o)))
    return out


def t_textbox(sidx, text, below=('slot', 'title')):
    def title_box(seed):
        s = seed['slides'][sidx]
        return box_of(seed, s, find(s, below))

    def want(seed):
        x, y, w, h = title_box(seed)
        return (x, y + h, w, int(0.9 * CM))

    def gold(seed):
        d = deck_copy(seed)
        s = d['slides'][sidx]
        idx = [i for i, o in enumerate(s['objs']) if okey(o) == below][0]
        s['objs'].insert(idx + 1, {'t': 'shape', 'id': None, 'name': None, 'box': want(seed), 'rot': 0, 'flip': None,
                                   'paras': [text]})
        return d

    def check(seed, res):
        err = others_same(seed, res, new_objs={sidx: 1})
        if err:
            return err
        rs = res_slide(res, sidx)
        new = [o for o in flat(rs['objs']) if o['t'] != 'slot' and o.get('id') is None]
        o = new[0]
        if o['t'] != 'shape':
            return 'the new object is a %s, not a text box' % o['t']
        if [norm_para(p) for p in o['paras']] != [norm_para(text)]:
            return 'the new text box holds %r' % o['paras']
        x, y, w, h = want(seed)
        e = box_is(o['box'], (x, y, w, None), 'the new text box')
        if e:
            return e
        if not (0.7 * CM - TOL <= o['box'][3] <= 1.1 * CM + TOL):
            return 'the new text box is %.2f cm tall' % (o['box'][3] / CM)
        for other in rs['objs']:
            if other is o:
                continue
            if overlaps(o['box'], box_of(res, rs, other)):
                return 'the new text box overlaps %s %s' % okey(other)
        return None
    return gold, check


def t_text(sidx, key, old, new):
    def gold(seed):
        d = deck_copy(seed)
        o = find(d['slides'][sidx], key)
        o['paras'] = [p.replace(old, new) if norm_text(p).endswith(norm_text(old)) else p for p in o['paras']]
        assert o['paras'] != find(seed['slides'][sidx], key)['paras']
        return d

    def check(seed, res):
        err = others_same(seed, res, changed={(sidx, key)})
        if err:
            return err
        g = find(gold(seed)['slides'][sidx], key)
        o = find(res_slide(res, sidx), key)
        if [norm_para(p) for p in o['paras']] != [norm_para(p) for p in g['paras']]:
            return 'the text is %r' % o['paras']
        so = find(seed['slides'][sidx], key)
        if o.get('box') != so.get('box'):
            return 'the box of the edited object changed'
        return None
    return gold, check


def t_comparison(after, title, lh, lb, rh, rb):
    def gold(seed):
        d = deck_copy(seed)
        d['slides'].insert(after, slide('Comparison', [slot('title', [title]), slot('body', [lh]),
                                                       slot('body2', ['- ' + b for b in lb]), slot('body3', [rh]),
                                                       slot('body4', ['- ' + b for b in rb])]))
        return d

    def check(seed, res):
        err = others_same(seed, res, new_slide_at=after)
        if err:
            return err
        s = res['slides'][after]
        return slide_is(s, ('Comparison', {'title': [title], 'body': [lh], 'body2': ['- ' + b for b in lb],
                                           'body3': [rh], 'body4': ['- ' + b for b in rb]}, None), heading=('body', 'body3'))
    return gold, check


def slide_is(s, want, heading=()):
    layout, slots, notes = want
    if s['layout'] != layout:
        return 'layout is %r, expected %r' % (s['layout'], layout)
    extra = [o for o in s['objs'] if o['t'] != 'slot']
    if extra:
        return 'the slide has a %s besides its slots' % extra[0]['t']
    got = {o['slot']: o for o in s['objs']}
    if set(got) != set(slots):
        return 'slots are %s, expected %s' % (sorted(got), sorted(slots))
    for name, paras in slots.items():
        g = [norm_para(p) for p in got[name]['paras']]
        if name in heading and len(g) == 1 and g[0].startswith('- '):
            g = [g[0][2:]]
        if g != [norm_para(p) for p in paras]:
            return '::%s:: holds %r, expected %r' % (name, got[name]['paras'], paras)
        if got[name]['box'] is not None:
            return '::%s:: was given its own box (the layout\'s was asked for)' % name
    if (s.get('notes') is None) != (notes is None) or (
            notes is not None and [norm_para(p) for p in s['notes']] != [norm_para(p) for p in notes]):
        return 'notes are %r, expected %r' % (s.get('notes'), notes)
    return None


def t_write(slides_want):
    def gold(seed):
        d = deck_copy(seed)
        d['slides'] = [slide(l, [slot(n, p) for n, p in sl.items()], no) for l, sl, no in slides_want]
        return d

    def check(seed, res):
        if len(res['slides']) != len(slides_want):
            return '%d slides, expected %d' % (len(res['slides']), len(slides_want))
        for i, (s, w) in enumerate(zip(res['slides'], slides_want)):
            e = slide_is(s, w)
            if e:
                return 'slide %d: %s' % (i + 1, e)
        return None
    return gold, check


def refuse():
    return None, None


# ---------------------------------------------------------------- read questions: accepted objects

def labels(o):
    if o['t'] == 'slot':
        return {o['slot'], '::%s::' % o['slot']}
    out = set()
    if o.get('id'):
        out.add(o['id'])
    if o.get('name'):
        out.add(o['name'])
    return out


def q_answer(sidx, keys):
    return sidx, keys


# ---------------------------------------------------------------- the tasks
# Task wording is the same for every candidate. Ids: deck<r>-q1..q2 (read), -e1..e7 (edit), -w (write).

def cm(v):
    return int(round(v * CM))


TASKS = {
    1: [
        ('q1', 'read', 'On slide 3 (지역별 현황), which objects\' boxes overlap the box of the text box 출처?',
         q_answer(2, [('slot', 'left')])),
        ('q2', 'read', 'On slide 4 (분기별 매출), which objects lie entirely inside the bottom-right quarter of the '
         'slide?', q_answer(3, [('keep', 'k04k9')])),
        ('e1', 'edit', 'On slide 3 (지역별 현황), move the text box 출처 to the right so that its right edge lines up '
         'with the right edge of the right column. Keep its size and its vertical position.',
         t_geom(2, ('shape', 's5'), lambda d: (4648200 + 4038600 - 3657600, 6035040, 3657600, 365760), 'the 출처 box')),
        ('e2', 'edit', 'On slide 4 (분기별 매출), make the picture twice as wide and twice as tall, keeping its '
         'bottom-right corner where it is.',
         t_geom(3, ('keep', 'k04k9'), lambda d: (8229600 - 1828800, 5486400 - 914400, 1828800, 914400), 'the picture')),
        ('e3', 'edit', 'On slide 4 (분기별 매출), add a text box with the text "단위: 억 원" directly under the title: '
         'its top edge on the title\'s bottom edge, its left and right edges on the title\'s, and between 0.7 cm and '
         '1.1 cm tall. It must not overlap any other object.', t_textbox(3, '단위: 억 원')),
        ('e4', 'edit', 'On slide 4 (분기별 매출), move the picture to the left so that its left edge lines up with the '
         'table\'s left edge. Keep its size and its vertical position.',
         t_geom(3, ('keep', 'k04k9'), lambda d: (914400, 5029200, 914400, 457200), 'the picture')),
        ('e5', 'edit', 'On slide 2 (핵심 지표), change the bullet "지방 13곳" to "지방 14곳". Change nothing else: every '
         'position and size stays as it is.', t_text(1, ('slot', 'body'), '지방 13곳', '지방 14곳')),
        ('e6', 'edit', 'Right after slide 3 (지역별 현황), add a slide with the Comparison layout: title "권역별 전략"; '
         'left heading "수도권", with the bullets "신규 지점 2곳" and "온라인 판촉 강화" under it; right heading "지방", '
         'with the bullets "부산 거점 확대" and "대리점 교육" under it. Leave every placeholder where the layout puts it.',
         t_comparison(3, '권역별 전략', '수도권', ['신규 지점 2곳', '온라인 판촉 강화'], '지방', ['부산 거점 확대', '대리점 교육'])),
        ('e7', 'edit', 'On slide 4 (분기별 매출), change the 3분기 figure in the table from 1,204 to 1,210.', refuse()),
        ('w', 'write', 'Write a new presentation with exactly these three slides, in this order, using the layouts '
         'listed above:\n1. Layout Title Slide: title "2026년 4분기 영업 계획"; subtitle "영업기획팀 · 2026년 10월".\n'
         '2. Layout Title and Content: title "4분기 목표"; body bullets "매출 1,300억 원" and "신규 고객 40곳"; speaker '
         'notes "목표는 10월 경영회의에서 확정".\n3. Layout Two Content: title "채널별 과제"; left column bullets '
         '"오프라인: 매장 판촉" and "오프라인: 대리점 교육"; right column bullets "온라인: 기획전" and "온라인: 멤버십 '
         '캠페인".\nEvery placeholder stays where its layout puts it.',
         t_write([('Title Slide', {'title': ['2026년 4분기 영업 계획'], 'subtitle': ['영업기획팀 · 2026년 10월']}, None),
                  ('Title and Content', {'title': ['4분기 목표'], 'body': ['- 매출 1,300억 원', '- 신규 고객 40곳']},
                   ['목표는 10월 경영회의에서 확정']),
                  ('Two Content', {'title': ['채널별 과제'], 'left': ['- 오프라인: 매장 판촉', '- 오프라인: 대리점 교육'],
                                   'right': ['- 온라인: 기획전', '- 온라인: 멤버십 캠페인']}, None)])),
    ],
    2: [
        ('q1', 'read', 'On slide 1, which objects\' boxes overlap the box of the shape "Freeform 6" (the one holding '
         '"Cloud")? A line\'s box is the rectangle its two ends span.', q_answer(0, [('line', 's10')])),
        ('q2', 'read', 'On slide 1, which objects lie entirely inside the top-right quarter of the slide?',
         q_answer(0, [('keep', 'kqld8')])),
        ('e1', 'edit', 'On slide 1, move the text box "TextBox 3" 3 cm to the right. Keep its size and its vertical '
         'position.', t_geom(0, ('shape', 's4'), lambda d: (914400 + cm(3), 914400, 2286000, 369332), 'TextBox 3')),
        ('e2', 'edit', 'On slide 1, make "Picture 1" half as wide, keeping its aspect ratio and its top-left corner.',
         t_geom(0, ('keep', 'kqld8'), lambda d: (5105400, 990600, 914400, 838200), 'Picture 1')),
        ('e3', 'edit', 'On slide 5 (Hyperlinks), add a text box with the text "Links checked in September 2026" '
         'directly under the title: its top edge on the title\'s bottom edge, its left and right edges on the '
         'title\'s, and between 0.7 cm and 1.1 cm tall. It must not overlap any other object.',
         t_textbox(4, 'Links checked in September 2026')),
        ('e4', 'edit', 'On slide 3, move "Right Arrow 3" (in the group) to the left so that its left edge lines up '
         'with the left edge of "Rectangle 1". Keep its size and its vertical position.',
         t_geom(2, ('shape', 's4'), lambda d: (1524000, 2819400, 978408, 484632), 'Right Arrow 3')),
        ('e5', 'edit', 'On slide 2, change the subtitle\'s last line "And second line" to "And a second line". '
         'Change nothing else: every position and size stays as it is.',
         t_text(1, ('slot', 'subtitle'), 'And second line', 'And a second line')),
        ('e6', 'edit', 'Right after slide 2, add a slide with the Comparison layout: title "Before and after"; left '
         'heading "Before", with the bullets "Positions in EMU" and "Hidden connectors" under it; right heading '
         '"After", with the bullets "Boxes in the text" and "Every object shown" under it. Leave every placeholder '
         'where the layout puts it.',
         t_comparison(2, 'Before and after', 'Before', ['Positions in EMU', 'Hidden connectors'], 'After',
                      ['Boxes in the text', 'Every object shown'])),
        ('e7', 'edit', 'On slide 1, make the text "Learning PPTX" red and 24 pt.', refuse()),
        ('w', 'write', 'Write a new presentation with exactly these three slides, in this order, using the layouts '
         'listed above:\n1. Layout Title Slide: title "Quarterly Review"; subtitle "Operations team, October 2026".\n'
         '2. Layout Title and Content: title "Highlights"; body bullets "Costs down 4%" and "Two new sites"; speaker '
         'notes "Mention the audit".\n3. Layout Two Content: title "Next steps"; left column bullets "Hire two '
         'engineers" and "Close the old site"; right column bullets "Open the Busan office" and "Review in March".\n'
         'Every placeholder stays where its layout puts it.',
         t_write([('Title Slide', {'title': ['Quarterly Review'], 'subtitle': ['Operations team, October 2026']}, None),
                  ('Title and Content', {'title': ['Highlights'], 'body': ['- Costs down 4%', '- Two new sites']},
                   ['Mention the audit']),
                  ('Two Content', {'title': ['Next steps'], 'left': ['- Hire two engineers', '- Close the old site'],
                                   'right': ['- Open the Busan office', '- Review in March']}, None)])),
    ],
    3: [
        ('q1', 'read', 'On slide 2 (시장 현황), which objects\' boxes overlap the box of the shape "NEW 배지"? Use the '
         'boxes as written, ignoring rotation.', q_answer(1, [('slot', 'title'), ('slot', 'body')])),
        ('q2', 'read', 'On slide 3 (출시 일정), which objects of the group 타임라인 lie entirely in the right half of '
         'the slide?', q_answer(2, [('line', 's9'), ('shape', 's10')])),
        ('e1', 'edit', 'On slide 3 (출시 일정), move the text box 주석 2 cm down. Keep its size and its horizontal '
         'position.', t_geom(2, ('shape', 's11'), lambda d: (1219200, 4572000 + cm(2), 6096000, 369332), '주석')),
        ('e2', 'edit', 'On slide 2 (시장 현황), make the logo picture 1.5 times as wide and 1.5 times as tall, keeping '
         'its top-right corner where it is.',
         t_geom(1, ('keep', 'k7hqa'), lambda d: (LOGO[0] + LOGO[2] - 1.5 * LOGO[2], LOGO[1], 1.5 * LOGO[2],
                                                 1.5 * LOGO[3]), 'the logo')),
        ('e3', 'edit', 'On slide 5 (예상 매출), add a text box with the text "단위: 억 원 (2027년 계획)" directly under '
         'the title: its top edge on the title\'s bottom edge, its left and right edges on the title\'s, and between '
         '0.7 cm and 1.1 cm tall. It must not overlap any other object.', t_textbox(4, '단위: 억 원 (2027년 계획)')),
        ('e4', 'edit', 'On slide 5 (예상 매출), move the text box "2분기 라벨" so that its left edge lines up with the '
         'left edge of "1분기 라벨". Keep its size and its vertical position.',
         t_geom(4, ('shape', 's14'), lambda d: (8001000, 2971800, 2286000, 369332), '2분기 라벨')),
        ('e5', 'edit', 'On slide 4 (제품 비교), in the right column, change "음성 인식 지원" to "음성 인식·제스처 지원". '
         'Change nothing else: every position and size stays as it is.',
         t_text(3, ('slot', 'right'), '음성 인식 지원', '음성 인식·제스처 지원')),
        ('e6', 'edit', 'Right after slide 4 (제품 비교), add a slide with the Comparison layout: title "출시 채널 비교"; '
         'left heading "온라인", with the bullets "자사몰 단독 판매" and "사전 예약 할인" under it; right heading '
         '"오프라인", with the bullets "가전 매장 체험존" and "통신사 대리점" under it. Leave every placeholder where the '
         'layout puts it.',
         t_comparison(4, '출시 채널 비교', '온라인', ['자사몰 단독 판매', '사전 예약 할인'], '오프라인',
                      ['가전 매장 체험존', '통신사 대리점'])),
        ('e7', 'edit', 'On slide 3 (출시 일정), fill the three boxes of the timeline with the brand blue #1E5AA8.',
         refuse()),
        ('w', 'write', 'Write a new presentation with exactly these three slides, in this order, using the layouts '
         'listed above:\n1. Layout Title Slide: title "2027 상반기 마케팅 계획"; subtitle "마케팅본부 · 2026년 12월".\n'
         '2. Layout Title and Content: title "핵심 목표"; body bullets "신규 고객 5만 명" and "재구매율 40%"; speaker '
         'notes "목표치는 1월 확정".\n3. Layout Section Header: title "세부 실행안"; body "채널별 계획".\n'
         'Every placeholder stays where its layout puts it.',
         t_write([('Title Slide', {'title': ['2027 상반기 마케팅 계획'], 'subtitle': ['마케팅본부 · 2026년 12월']}, None),
                  ('Title and Content', {'title': ['핵심 목표'], 'body': ['- 신규 고객 5만 명', '- 재구매율 40%']},
                   ['목표치는 1월 확정']),
                  ('Section Header', {'title': ['세부 실행안'], 'body': ['채널별 계획']}, None)])),
    ],
}

READ_HOW = ('`"ANSWER: <objects, comma-separated>"`, naming each object by its id, or a slot by its slot name '
            '(e.g. `ANSWER: title, s4`), or `ANSWER: none` if there is none.')


# ---------------------------------------------------------------- part 2: harder geometry (A, Ap and B only)
# Part 1 put A, Ap and B at the ceiling on both models, so part 2 asks for what separates them: geometry of slots
# (inherited in B), several objects at once, spacing and centring arithmetic, z-order, a group, a rotation.

def t_set(sidx, wants, order=None, rots=None):
    """wants: {key: box} (exact EMU; a None component keeps the seed's). order: the slide's top-level keys after the
    edit. rots: {key: degrees}."""
    rots = rots or {}

    def full(seed, key, w):
        s = seed['slides'][sidx]
        b = box_of(seed, s, find(s, key))
        return tuple(int(round(b[i] if w[i] is None else w[i])) for i in range(4))

    def gold(seed):
        d = deck_copy(seed)
        s = d['slides'][sidx]
        for key, w in wants.items():
            o = find(s, key)
            b = full(seed, key, w)
            if o['t'] == 'line':
                b0 = box_of(seed, s, o)
                dx, dy = b[0] - b0[0], b[1] - b0[1]
                o['a'] = (o['a'][0] + dx, o['a'][1] + dy)
                o['b'] = (o['b'][0] + dx, o['b'][1] + dy)
            else:
                o['box'] = b
        for key, r in rots.items():
            find(s, key)['rot'] = r
        if order:
            by = {okey(o): o for o in s['objs']}
            s['objs'] = [by[k] for k in order]
        return d

    def check(seed, res):
        ch = set()
        for key in list(wants) + list(rots):
            ch |= {(sidx, key)} | parents(seed, sidx, key)
        err = others_same_unordered(seed, res, ch, sidx if order else None)
        if err:
            return err
        rs = res_slide(res, sidx)
        for key, w in wants.items():
            e = box_is(box_of(res, rs, find(rs, key)), full(seed, key, w), '%s %s' % key)
            if e:
                return e
        for key, r in rots.items():
            if abs((find(rs, key).get('rot') or 0) - r) > 0.5:
                return '%s %s: rot is %s' % (key[0], key[1], find(rs, key).get('rot'))
        if order and [okey(o) for o in rs['objs']] != list(order):
            return 'z-order is %s' % [okey(o)[1] for o in rs['objs']]
        return None
    return gold, check


def others_same_unordered(seed, res, changed, reorder_slide):
    """others_same, but the slide `reorder_slide` may change its z-order (checked by the task)."""
    if reorder_slide is None:
        return others_same(seed, res, changed=changed)
    import copy
    res2 = copy.deepcopy(res)
    rs = res_slide(res2, reorder_slide)
    order = [okey(o) for o in seed['slides'][reorder_slide]['objs']]
    rs['objs'].sort(key=lambda o: order.index(okey(o)) if okey(o) in order else len(order))
    return others_same(seed, res2, changed=changed)


def shift(key_boxes, dx=0, dy=0):
    return {k: (b[0] + dx, b[1] + dy, None, None) for k, b in key_boxes}


G2 = DECK2['slides'][2]['objs'][0]['children']
G3 = DECK3['slides'][2]['objs'][1]['children']
L43 = dict(LAYOUTS_43)
L169 = dict(LAYOUTS_169)


def kids(children, dx):
    out = {}
    for c in children:
        out[okey(c)] = (obj_box(c)[0] + dx, None, None, None)
    return out


TASKS_X = {
    1: [
        ('x1', 'read', 'On slide 3 (지역별 현황), which objects reach lower than 16 cm from the top of the slide (their '
         'bottom edge is more than 16 cm from the top)?',
         q_answer(2, [('slot', 'left'), ('slot', 'right'), ('shape', 's5')])),
        ('x2', 'edit', 'On slide 3 (지역별 현황), make the left column narrower: move its right edge 2 cm to the left, '
         'keeping its other three edges where they are.',
         t_set(2, {('slot', 'left'): (None, None, 4038600 - cm(2), None)})),
        ('x3', 'edit', 'On slide 4 (분기별 매출), put the picture behind the table, so that the table is drawn on top '
         'of it. Change nothing else.',
         t_set(3, {}, order=[('slot', 'title'), ('keep', 'k04k9'), ('keep', 'kb2br')])),
        ('x4', 'edit', 'On slide 4 (분기별 매출), centre the picture horizontally on the slide, keeping its size and '
         'its vertical position.', t_set(3, {('keep', 'k04k9'): ((9144000 - 914400) / 2, None, None, None)})),
        ('x5', 'edit', 'On slide 2 (핵심 지표), move the title 1 cm down, keeping its size; and move the body\'s top '
         'edge 1 cm down as well, keeping the body\'s bottom edge where it is.',
         t_set(1, {('slot', 'title'): (None, T43[1] + cm(1), None, None),
                   ('slot', 'body'): (None, 1600200 + cm(1), None, 4525963 - cm(1))})),
        ('x6', 'edit', 'On slide 3 (지역별 현황), move the 출처 text box down so that its top edge is 0.5 cm below the '
         'left column\'s bottom edge. Keep its size and its horizontal position.',
         t_set(2, {('shape', 's5'): (None, 1600200 + 4525963 + cm(0.5), None, None)})),
    ],
    2: [
        ('x1', 'read', 'On slide 6, which rectangle\'s centre is closest to the slide\'s horizontal centre?',
         q_answer(5, [('shape', 's11')])),
        ('x2', 'edit', 'On slide 6, space the six rectangles evenly: keep the first and the last where they are, and '
         'move the four between them so that the gaps between neighbours are all equal. Keep every size and vertical '
         'position.',
         t_set(5, {('shape', k): (838200 + i * (7543800 - 838200) / 5, None, None, None)
                   for i, k in enumerate(['s2', 's9', 's10', 's11', 's12', 's13'])})),
        ('x3', 'edit', 'On slide 1, bring the text box "TextBox 3" to the front, so that it is drawn on top of every '
         'other object. Change nothing else.',
         t_set(0, {}, order=[('line', 's6'), ('shape', 's7'), ('keep', 'kqld8'), ('keep', 'kmtnf'), ('line', 's8'),
                             ('line', 's10'), ('shape', 's4')])),
        ('x4', 'edit', 'On slide 3, move the whole group 2 cm to the right. Keep every size and vertical position.',
         t_set(2, kids(G2, cm(2)))),
        ('x5', 'edit', 'On slide 1, make the table 20% narrower, keeping its centre and its height.',
         t_set(0, {('keep', 'kmtnf'): (3810000 + 0.1 * 4724400, None, 0.8 * 4724400, None)})),
        ('x6', 'edit', 'On slide 5 (Hyperlinks), make the title exactly as wide as the table, with its left edge on '
         'the table\'s left edge. Keep the title\'s top and height.',
         t_set(4, {('slot', 'title'): (685800, None, 7848600, None)})),
    ],
    3: [
        ('x1', 'read', 'On slide 2 (시장 현황), which objects are partly or wholly within 2 cm of the slide\'s right '
         'edge?', q_answer(1, [('keep', 'k7hqa')])),
        ('x2', 'edit', 'On slide 4 (제품 비교), widen the right column so that its right edge is 1 cm from the slide\'s '
         'right edge, keeping its other three edges where they are.',
         t_set(3, {('slot', 'right'): (None, None, 12192000 - cm(1) - 6172200, None)})),
        ('x3', 'edit', 'On slide 2 (시장 현황), send the shape "NEW 배지" behind the body, so that the body is drawn '
         'on top of it. Change nothing else.',
         t_set(1, {}, order=[('slot', 'title'), ('shape', 's4'), ('slot', 'body'), ('keep', 'k7hqa')])),
        ('x4', 'edit', 'On slide 3 (출시 일정), centre the timeline group horizontally on the slide. Keep every size and '
         'vertical position.', t_set(2, kids(G3, (12192000 - 9144000) / 2 - 1219200))),
        ('x5', 'edit', 'On slide 5 (예상 매출), put the two label text boxes side by side below the chart: "1분기 라벨" '
         'with its left edge on the chart\'s left edge and its top edge 0.5 cm below the chart\'s bottom edge; "2분기 '
         '라벨" to its right, with a gap of 0.5 cm between them and the same top edge. Keep both sizes.',
         t_set(4, {('shape', 's13'): (1524000, 2133600 + 3810000 + cm(0.5), None, None),
                   ('shape', 's14'): (1524000 + 2286000 + cm(0.5), 2133600 + 3810000 + cm(0.5), None, None)})),
        ('x6', 'edit', 'On slide 2 (시장 현황), turn the shape "NEW 배지" upright (no rotation) and move it so that its '
         'top-right corner is on the body\'s top-right corner. Keep its size.',
         t_set(1, {('shape', 's4'): (838200 + 10515600 - 1143000, 1825625, None, None)}, rots={('shape', 's4'): 0})),
    ],
}
