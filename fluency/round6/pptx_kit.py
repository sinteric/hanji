#!/usr/bin/env python3
"""Round 6 part B: formatting on Presentation objects. Usage:
  python3 pptx_kit.py build            seeds, units, gold, prompts (checks every gold answer lands)
  python3 pptx_kit.py score <unit> <answers.json>

Two candidates, on three decks (the pptx canvas audit's two samples and its Korean synthetic deck):
  F1   every object shows its effective formatting inline, like its box (theme style and layout values included)
  F2o  an object shows only what it sets itself; what it takes from the theme's shape style or the layout is not
       shown (the audit's proposal)
The text is hanji's (engine dump) with the formatting added by canvas.py. Checks are on the text: every object
keeps its attributes, text and run formatting except what the task changes, and every line outside the targets is
byte-identical. Expected values come from F1's text (the effective formatting)."""
import json
import os
import re
import sys
from difflib import SequenceMatcher

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import canvas  # noqa: E402
import doc as D  # noqa: E402
import inline as I  # noqa: E402
import score as S  # noqa: E402
import vocab as V  # noqa: E402

CANDS = ('F1', 'F2o')
DECKS = {
    1: {'name': 'synth-modern-pitch', 'desc': 'an 8-slide English pitch deck on a free canvas (synthetic, CC0)'},
    2: {'name': 'synth-korean-report', 'desc': 'a 6-slide Korean business report deck (synthetic, CC0)'},
    3: {'name': 'onlyoffice-sample', 'desc': 'the ONLYOFFICE sample presentation, 7 slides, 210 objects'},
}
FMT = set(V.ORDER)
NONFMT = {'id', 'name', 'box', 'from', 'to', 'rot', 'flip', 'kind', 'summary', 'src', 'style'}


def deck_path(rep):
    return os.path.join(HERE, 'data', 'decks', DECKS[rep]['name'] + '.pptx')


def today(rep):
    return open(os.path.join(HERE, 'data', 'today', DECKS[rep]['name'] + '.pptx.txt'), encoding='utf-8').read()


# ---------------------------------------------------------------- reading the text

class PErr(Exception):
    def __init__(self, line, msg):
        super().__init__('line %d: %s' % (line, msg))


TAG_ATTR = re.compile(r'\s*([a-z][a-z-]*)(?:=("([^"]*)"|[^\s"]+))?')


def tag_attrs(s, no):
    out = {}
    pos = 0
    s = s.strip()
    while pos < len(s):
        m = TAG_ATTR.match(s, pos)
        if not m or m.end() == pos:
            raise PErr(no, 'cannot read the attributes at %r' % s[pos:pos + 30])
        k = m.group(1)
        v = m.group(3) if m.group(3) is not None else m.group(2)
        if k in out:
            raise PErr(no, 'attribute %r written twice' % k)
        if k in FMT or k in V.FLAGS:
            try:
                out[k] = V.parse_value(k, v)
            except V.VocabError as e:
                raise PErr(no, str(e))
        elif k in NONFMT:
            out[k] = v
        else:
            raise PErr(no, 'unknown attribute %r; the formatting properties are %s' % (k, ', '.join(V.ORDER)))
        pos = m.end()
    return V.expand(out)


def para_of(piece, no):
    txt, brace = D.take_brace_end(piece, set(V.PARA_KEYS) | set(D.CHARV) | set(V.FLAGS) | {'fill'} | set(V.SIDES),
                                  no, 'paragraph formatting')
    try:
        plain, runs, zw = I.runs_of(txt, spans=True)
    except V.VocabError as e:
        raise PErr(no, str(e))
    return {'plain': plain, 'brace': brace or {}, 'runs': runs, 'raw': txt}


def parse(text):
    """-> list of objects: {key, kind, attrs, paras, lines}; key = (slide, id or slot)."""
    lines = text.split('\n')
    end = lines.index('---', 1)
    objs = []
    slide = 0
    cur = None
    for i in range(end + 1, len(lines)):
        ln = lines[i]
        no = i + 1
        if ln == '---':
            slide += 1
            cur = None
            continue
        if ln.startswith('layout:') or not ln.strip():
            continue
        if ln == '::notes::':
            cur = {'key': (slide, 'notes'), 'kind': 'notes', 'attrs': {}, 'paras': [], 'lines': [i]}
            objs.append(cur)
            continue
        m = re.match(r'^::(\w+)(.*)::$', ln)
        if m:
            cur = {'key': (slide, 'slot:' + m.group(1)), 'kind': 'slot', 'attrs': tag_attrs(m.group(2), no),
                   'paras': [], 'lines': [i]}
            objs.append(cur)
            continue
        m = re.match(r'^<(shape|line|keep|group)\b(.*?)(/?)>(.*)$', ln)
        if m and not (m.group(1) == 'shape' and not m.group(3) and not m.group(4).endswith('</shape>')):
            kind, inner, selfc, rest = m.groups()
            if kind == 'shape' and not selfc:
                # the attributes end at the first `>` outside quotes
                full = ln[len('<shape'):]
                q = False
                for k, ch in enumerate(full):
                    if ch == '"':
                        q = not q
                    elif ch == '>' and not q:
                        inner, rest = full[:k], full[k + 1:]
                        break
                if not rest.endswith('</shape>'):
                    raise PErr(no, 'a shape is one line: <shape …>text</shape>')
                body = rest[:-len('</shape>')]
            else:
                body = None
                if selfc == '' and kind in ('line', 'keep'):
                    raise PErr(no, '<%s …/> closes itself' % kind)
            attrs = tag_attrs(inner.rstrip('/'), no)
            key = (slide, 'id:' + attrs['id']) if 'id' in attrs else (slide, 'new:%d' % i)
            cur = None
            o = {'key': key, 'kind': kind, 'attrs': attrs, 'paras': [], 'lines': [i]}
            if body is not None:
                o['paras'] = [para_of(p, no) for p in body.split('<p/>')]
            objs.append(o)
            continue
        if ln == '</group>':
            continue
        if cur is not None:
            cur['paras'].append(para_of(ln, no))
            cur['lines'].append(i)
            continue
        raise PErr(no, 'this line is not an object, a slot marker or slot text: %r' % ln[:60])
    return objs


def char_props(o):
    """Per character: (char, props) from the object's text properties, the paragraph's {…}, spans and marks."""
    top = {k: v for k, v in o['attrs'].items() if k in D.CHARV or k in V.FLAGS}
    out = []
    for p in o['paras']:
        base = dict(top)
        base.update({k: v for k, v in p['brace'].items() if k in D.CHARV or k in V.FLAGS})
        for t, fl, sp in p['runs']:
            pr = dict(base)
            for f in fl:
                pr[f] = 'yes'
            pr.update(sp)
            out += [(ch, pr) for ch in t]
        out.append(('\n', {}))
    return out


def para_props(o):
    return [{k: v for k, v in p['brace'].items() if k in V.PARA_KEYS} for p in o['paras']]


# ---------------------------------------------------------------- predicates

def is_navy(c):
    if not c or not c.startswith('#'):
        return False
    h, l, s = V.hls(c)
    return 200 <= h <= 250 and l <= 0.4 and s >= 0.3


def is_coral(c):
    if not c or not c.startswith('#'):
        return False
    h, l, s = V.hls(c)
    return (h <= 25 or h >= 350) and s >= 0.6 and 0.55 <= l <= 0.82


def border_is(width, style, colpred):
    def f(b):
        p = V.border_parts(b)
        return p is not None and abs(p[0] - width) < 0.05 and p[1] == style and colpred(p[2])
    return f


# ---------------------------------------------------------------- tasks

class Task:
    def __init__(self, tid, kind, text, **kw):
        self.id, self.kind, self.text = tid, kind, text
        self.__dict__.update(kw)


def obj(objs, key):
    for o in objs:
        if o['key'] == key:
            return o
    raise KeyError(key)


def by_name(objs, slide, name):
    for o in objs:
        if o['key'][0] == slide and o['attrs'].get('name') == name:
            return o['key']
    raise KeyError(name)


def fmt_of(o):
    return {k: v for k, v in o['attrs'].items() if k in FMT}


def tasks(rep):
    """Each task: targets(truth objs) -> keys; box: {key: pred}; span: (key, para, word, {k: pred}); retext."""
    t = []
    T = Task
    if rep == 1:
        t.append(T('pp-q1', 'read', 'On the Agenda slide, what is the outline (width, line style and colour) of the '
                   'connector "Connector 6"? Answer `ANSWER: border="<width> <style> <colour>"`.',
                   expect=lambda o: {'border-top': fmt_of(obj(o, (1, 'id:s7'))).get('border-top')}, ref=(1, 'id:s7')))
        t.append(T('pp-q2', 'read', 'On the Agenda slide, which shapes are filled accent1? Answer `ANSWER: <name>; '
                   '<name>`.', expect_names=lambda o: [x['attrs']['name'] for x in o if x['key'][0] == 1 and
                                                         fmt_of(x).get('fill') == 'accent1']))
        t.append(T('pp-e1', 'edit', 'Change the fill of Card 2\'s background, "Rounded Rectangle 7", to accent2.',
                   targets=lambda o: [by_name(o, 2, 'Rounded Rectangle 7')], box={'fill': lambda v: v == 'accent2'}))
        t.append(T('pp-e2', 'edit', 'Give Card 3\'s background, "Rounded Rectangle 12", a 2pt navy outline.',
                   targets=lambda o: [by_name(o, 2, 'Rounded Rectangle 12')],
                   box={s: border_is(2, 'solid', is_navy) for s in V.SIDES}))
        t.append(T('pp-e3', 'edit', 'In the text "Battery cost down 60% in 5 years", make "60%" 24pt and coral.',
                   targets=lambda o: [by_name(o, 2, 'TextBox 10')],
                   span=('60%', {'size': lambda v: v == '24pt', 'color': is_coral})))
        t.append(T('pp-e4', 'edit', 'Make the three connectors on the Agenda slide 3pt wide, keeping their colour and '
                   'line style.',
                   targets=lambda o: [x['key'] for x in o if x['key'][0] == 1 and x['kind'] == 'line'],
                   box_rel=lambda f: {s: border_is(3, V.border_parts(f[s])[1], lambda c, w=V.border_parts(f[s])[2]:
                                                   c == w) for s in V.SIDES}))
        t.append(T('pp-e5', 'edit', 'Change "Series A · 2026" to "Series B · 2026", keeping its formatting exactly as '
                   'it is.', targets=lambda o: [by_name(o, 0, 'TextBox 4')], retext=('Series A', 'Series B')))
        t.append(T('pp-e6', 'refuse', 'Give Card 1\'s background, "Rounded Rectangle 2", a soft drop shadow.'))
    elif rep == 2:
        t.append(T('pp-q1', 'read', 'On the 조직 구성 slide, what is the outline (width, line style and colour) of the '
                   'connectors? Answer `ANSWER: border="<width> <style> <colour>"`.',
                   expect=lambda o: {'border-top': fmt_of(obj(o, (4, 'id:s5'))).get('border-top')}, ref=(4, 'id:s5')))
        t.append(T('pp-q2', 'read', 'On the 추진 절차 slide, which shapes are filled accent2? Answer `ANSWER: <name>; '
                   '<name>`.', expect_names=lambda o: [x['attrs']['name'] for x in o if x['key'][0] == 2 and
                                                         fmt_of(x).get('fill') == 'accent2']))
        t.append(T('pp-e1', 'edit', 'Change the fill of the card "Rounded Rectangle 3" (+12%) to accent2.',
                   targets=lambda o: [by_name(o, 1, 'Rounded Rectangle 3')], box={'fill': lambda v: v == 'accent2'}))
        t.append(T('pp-e2', 'edit', 'Give the shape "Oval 4" (VS) a 2pt navy outline.',
                   targets=lambda o: [by_name(o, 3, 'Oval 4')],
                   box={s: border_is(2, 'solid', is_navy) for s in V.SIDES}))
        t.append(T('pp-e3', 'edit', 'In "10월 착수 예정", make "10월" 24pt and coral.',
                   targets=lambda o: [by_name(o, 2, 'Rectangular Callout 7')],
                   span=('10월', {'size': lambda v: v == '24pt', 'color': is_coral})))
        t.append(T('pp-e4', 'edit', 'Make the three connectors on the 조직 구성 slide 3pt wide, keeping their colour '
                   'and line style.',
                   targets=lambda o: [x['key'] for x in o if x['key'][0] == 4 and x['kind'] == 'line'],
                   box_rel=lambda f: {s: border_is(3, V.border_parts(f[s])[1], lambda c, w=V.border_parts(f[s])[2]:
                                                   c == w) for s in V.SIDES}))
        t.append(T('pp-e5', 'edit', 'Change "신규 4곳" to "신규 5곳", keeping its formatting exactly as it is.',
                   targets=lambda o: [(3, 'slot:right')], retext=('신규 4곳', '신규 5곳')))
        t.append(T('pp-e6', 'refuse', 'Make the gradient of the left panel on the first slide run from navy to '
                   'teal.'))
    else:
        t.append(T('pp-q1', 'read', 'On slide 1, what is the fill of the shape s780022854? Answer `ANSWER: '
                   'fill=<colour>`.', expect=lambda o: {'fill': fmt_of(obj(o, (0, 'id:s780022854'))).get('fill')},
                   ref=(0, 'id:s780022854')))
        t.append(T('pp-q2', 'read', 'On slide 1, which shapes inside the group g752157556 have a 1.5pt outline? '
                   'Answer `ANSWER: <id>; <id>` with the shape ids.',
                   expect_names=lambda o: onlyoffice_q2(o)))
        t.append(T('pp-e1', 'edit', 'On slide 2, change the fill of the shape s304069585 to accent2.',
                   targets=lambda o: [(1, 'id:s304069585')], box={'fill': lambda v: v == 'accent2'}))
        t.append(T('pp-e2', 'edit', 'On slide 3, give the shape "Slide text" a 2pt navy outline.',
                   targets=lambda o: [by_name(o, 2, 'Slide text')],
                   box={s: border_is(2, 'solid', is_navy) for s in V.SIDES}))
        t.append(T('pp-e3', 'edit', 'In the slide 3 title "What are presentations in editors?", make "editors" '
                   '24pt and coral.', targets=lambda o: [(2, 'slot:title')],
                   span=('editors', {'size': lambda v: v == '24pt', 'color': is_coral})))
        t.append(T('pp-e4', 'edit', 'On slide 1, make the two small lines s721421238 and s918234874 twice as thick, '
                   'keeping their colour and line style.',
                   targets=lambda o: [(0, 'id:s721421238'), (0, 'id:s918234874')],
                   box_rel=lambda f: {s: border_is(2 * V.border_parts(f[s])[0], V.border_parts(f[s])[1],
                                                   lambda c, w=V.border_parts(f[s])[2]: c == w) for s in V.SIDES}))
        # slide 1 holds "Presentation Editor" three times (the title slot, a large title shape and a small label):
        # changing any of them, or all, lands
        t.append(T('pp-e5', 'edit', 'On slide 1, change "Presentation Editor" to "Presentation Studio", keeping its '
                   'formatting exactly as it is.', targets=lambda o: [(0, 'id:s1179962678'), (0, 'id:s1696527000'),
                                                                      (0, 'slot:title')],
                   retext=('Presentation Editor', 'Presentation Studio'), retext_one=True))
        t.append(T('pp-e6', 'refuse', 'On slide 2, give the shape "Slide title" a soft drop shadow.'))
    return t


def onlyoffice_q2(o):
    # the shapes listed between the group's line and its </group>: found by the text order
    out = []
    inside = False
    depth = 0
    for x in o:
        pass
    return ONLY_Q2


ONLY_Q2 = []


def group_members(text, gid):
    lines = text.split('\n')
    out = []
    depth = None
    for ln in lines:
        if depth is None:
            if ln.startswith('<group id="%s"' % gid):
                depth = 1
            continue
        if ln.startswith('<group '):
            depth += 1
        elif ln == '</group>':
            depth -= 1
            if depth == 0:
                break
        else:
            m = re.match(r'<shape id="([^"]+)"', ln)
            if m:
                out.append((m.group(1), ln))
    return out


# ---------------------------------------------------------------- scoring

def check_edit(task, seed_text, ans_text, truth):
    so = parse(seed_text)
    ao = parse(ans_text)
    sk = [o['key'] for o in so]
    ak = [o['key'] for o in ao]
    if sk != ak:
        extra = [k for k in ak if k not in sk]
        gone = [k for k in sk if k not in ak]
        return False, ['objects_changed'], 'objects added %s, removed %s' % (extra[:3], gone[:3])
    targets = set(task.targets(truth))
    allowed = set()
    changed_one = []
    tfmt = {o['key']: fmt_of(o) for o in truth}
    for s, a in zip(so, ao):
        if s['key'] in targets:
            allowed.update(s['lines'])
        nf_s = {k: v for k, v in s['attrs'].items() if k in NONFMT}
        nf_a = {k: v for k, v in a['attrs'].items() if k in NONFMT}
        if nf_s != nf_a:
            return False, ['not_landed'], '%s: %s changed' % (s['key'], sorted(set(nf_s.items()) ^ set(nf_a.items())))
        fs, fa = fmt_of(s), fmt_of(a)
        spec = {}
        if s['key'] in targets:
            spec = dict(getattr(task, 'box', {}) or {})
            if getattr(task, 'box_rel', None):
                spec.update(task.box_rel(tfmt[s['key']]))
        for k in set(fs) | set(fa):
            if k in spec:
                if not spec[k](fa.get(k)):
                    return False, ['not_landed'], '%s: %s is %s' % (s['key'], k, fa.get(k))
            elif fs.get(k) != fa.get(k):
                return False, ['not_landed'], '%s: %s changed %s -> %s' % (s['key'], k, fs.get(k), fa.get(k))
        for k in spec:
            if k not in fa and not spec[k](None):
                return False, ['not_landed'], '%s: %s is not set' % (s['key'], k)
        sp, ap = [p['plain'] for p in s['paras']], [p['plain'] for p in a['paras']]
        rt = getattr(task, 'retext', None) if s['key'] in targets else None
        if rt and getattr(task, 'retext_one', False) and sp == ap:
            rt = None
        elif rt and getattr(task, 'retext_one', False):
            changed_one.append(s['key'])
        if rt:
            sp = [x.replace(rt[0], rt[1], 1) for x in sp]
        if sp != ap:
            return False, ['not_landed'], '%s: text %r, want %r' % (s['key'], ap[:2], sp[:2])
        if para_props(s) != para_props(a):
            return False, ['not_landed'], '%s: paragraph formatting changed' % (s['key'],)
        cs, ca = char_props(s), char_props(a)
        span = getattr(task, 'span', None) if s['key'] in targets else None
        word_at = set()
        if span:
            full = ''.join(ch for ch, _ in cs)
            k0 = full.find(span[0])
            word_at = set(range(k0, k0 + len(span[0])))
        for idx, ((ch, p1), (_, p2)) in enumerate(zip(cs, ca)):
            if not ch.strip():
                continue
            if idx in word_at:
                for k, pred in span[1].items():
                    if not pred(p2.get(k)):
                        return False, ['not_landed'], '%s: %s of %r is %s' % (s['key'], k, ch, p2.get(k))
                rest1 = {k: v for k, v in p1.items() if k not in span[1]}
                rest2 = {k: v for k, v in p2.items() if k not in span[1]}
                if rest1 != rest2:
                    return False, ['not_landed'], '%s: %r changed %s' % (s['key'], ch, rest2)
            elif p1 != p2:
                return False, ['not_landed'], '%s: text formatting of %r changed: %s -> %s' % (s['key'], ch, p1, p2)
    if getattr(task, 'retext_one', False) and not changed_one:
        return False, ['not_landed'], 'the text was not changed'
    lc = S.line_check(seed_text, ans_text, allowed, set())
    if lc:
        return False, ['untouched_line_changed'], lc
    if getattr(task, 'retext', None):
        tl = [seed_text.split('\n')[i] for o in so if o['key'] in targets and
              (not getattr(task, 'retext_one', False) or o['key'] in changed_one) for i in o['lines']]
        al = ans_text.split('\n')
        for ln in tl:
            if task.retext[0] in ln and ln.replace(task.retext[0], task.retext[1], 1) not in al:
                return False, ['formatting_changed'], 'the line must stay %r with only the text changed' % ln[:100]
    return True, [], ''


def read_ok(task, truth, text, cand):
    t = text.strip()
    if not t.upper().startswith('ANSWER:'):
        return False, 'a read answer is `ANSWER: …`'
    body = t[7:].strip().rstrip('.')
    if getattr(task, 'expect_names', None):
        want = sorted(task.expect_names(truth))
        got = sorted(x.strip().strip('"`') for x in re.split(r'[;,\n]', body) if x.strip())
        return (got == want), ('got %s, want %s' % (got, want) if got != want else '')
    exp = task.expect(truth)
    for k, v in exp.items():
        m = re.search(r'(?:border|%s)\s*=\s*("[^"]*"|\S+)' % re.escape(k), body)
        raw = (m.group(1).strip('"') if m else body.strip('"'))
        try:
            got = V.parse_value(k, raw)
        except V.VocabError as e:
            return False, str(e)
        if got != v:
            return False, '%s: got %s, want %s' % (k, got, v)
    return True, ''


def visible(task, texts, cand):
    """A read whose value the candidate's text does not show, or an edit that must keep a value it does not show:
    the right answer is a refusal."""
    if getattr(task, 'box_rel', None):
        objs = parse(texts[cand])
        return all('border-top' in fmt_of(obj(objs, k)) for k in task.targets(parse(texts['F1'])))
    ref = getattr(task, 'ref', None)
    if ref is None:
        return True
    o = obj(parse(texts[cand]), ref)
    return all(k in fmt_of(o) for k in task.expect(parse(texts['F1'])))


def score_answer(unit, answers):
    global ONLY_Q2
    m = re.fullmatch(r'r6p-(F1|F2o)-(\d)', unit)
    cand, rep = m.group(1), int(m.group(2))
    if rep == 3:
        ONLY_Q2 = json.load(open(os.path.join(HERE, 'data', 'units', unit + '.json'), encoding='utf-8'))['q2']
    seed = open(os.path.join(HERE, 'seeds', unit + '.txt'), encoding='utf-8').read()
    truth = parse(open(os.path.join(HERE, 'seeds', 'r6p-F1-%d.txt' % rep), encoding='utf-8').read())
    meta = json.load(open(os.path.join(HERE, 'data', 'units', unit + '.json'), encoding='utf-8'))
    unreach = set(meta['unreachable'])
    by = {a.get('task_id'): a for a in answers.get('answers', []) if isinstance(a, dict)}
    out = []
    for t in tasks(rep):
        a = by.get(t.id)
        r = {'task_id': t.id, 'valid': False, 'landed': False, 'error': None, 'chars': 0, 'flags': [], 'detail': ''}
        out.append(r)
        if a is None:
            r['flags'] = ['missing']
            r['error'] = 'no answer'
            continue
        if isinstance(a.get('text'), str):
            r['chars'] = len(a['text'])
        if isinstance(a.get('edits'), list):
            r['chars'] = sum(len(e.get('old', '')) + len(e.get('new', '')) for e in a['edits'] if isinstance(e, dict))
        refusal = S.is_refusal(a) or (t.kind == 'read' and t.id in unreach and S.declines(a))
        if t.kind == 'refuse':
            r['valid'] = True
            r['landed'] = refusal
            r['flags'] = [] if refusal else ['should_refuse']
            continue
        if t.id in unreach:
            r['valid'] = True
            r['flags'] = ['unreachable_refused' if refusal else 'unreachable_attempted']
            if not refusal and t.kind == 'read':
                ok, why = read_ok(t, truth, a.get('text', ''), cand)
                r['detail'] = 'answered %s' % ('right (a guess)' if ok else 'wrong: ' + why)
            elif not refusal:
                txt, flag, msg = S.r2.apply_edits(seed, a.get('edits') or [])
                if not flag:
                    try:
                        ok, fl, det = check_edit(t, seed, txt, truth)
                        r['detail'] = 'landed (a guess)' if ok else det
                    except PErr as e:
                        r['detail'] = str(e)
            continue
        if refusal:
            r['valid'] = True
            r['flags'] = ['wrong_refusal']
            r['detail'] = a['text'][:200]
            continue
        if t.kind == 'read':
            r['valid'] = isinstance(a.get('text'), str)
            ok, why = read_ok(t, truth, a.get('text', ''), cand)
            r['landed'] = ok
            if not ok:
                r['flags'] = ['wrong_answer']
                r['detail'] = why
            continue
        edits = a.get('edits')
        if not isinstance(edits, list) or not edits:
            r['error'] = 'an edit answer is {"task_id", "edits": [{"old", "new"}, …]}'
            continue
        txt, flag, msg = S.r2.apply_edits(seed, edits)
        if flag:
            r['error'] = msg + S.no_match_hint(seed, edits, flag)
            r['flags'] = [flag]
            continue
        try:
            ok, fl, det = check_edit(t, seed, txt, truth)
        except PErr as e:
            r['error'] = str(e)
            r['flags'] = ['parse_error']
            continue
        r['valid'] = True
        r['landed'] = ok
        r['flags'] = fl
        r['detail'] = det
    return out


# ---------------------------------------------------------------- gold answers

GOLD_BOX = {'pp-e1': {'fill': 'accent2'}, 'pp-e2': {'border': '2pt solid #1F3864'}}


def retag(line, fmt):
    """A tag or slot marker line with its formatting attributes replaced by fmt (canonical order)."""
    m = re.match(r'^(::\w+|<\w+)(.*?)(::|/>|>)(.*)$', line)
    head, inner, close, rest = m.groups()
    if close == '>' and head.startswith('<'):
        # a shape with text: the attributes end at the first `>` outside quotes
        full = line[len(head):]
        q = False
        for k, ch in enumerate(full):
            if ch == '"':
                q = not q
            elif ch == '>' and not q:
                inner, rest = full[:k], full[k + 1:]
                break
    selfc = inner.endswith('/') and close == '>'
    keep = [mm.group(0).strip() for mm in TAG_ATTR.finditer(inner.rstrip('/'))
            if mm.group(1) in NONFMT and mm.group(0).strip()]
    attrs = ' '.join(keep + ([V.fmt_attrs(V.expand(dict(fmt)))] if fmt else []))
    return '%s %s%s%s' % (head, attrs, close, rest) if attrs else '%s%s%s' % (head, close, rest)


def gold(task, text, truth, cand):
    objs = parse(text)
    lines = text.split('\n')
    if task.kind == 'read':
        if getattr(task, 'expect_names', None):
            return {'text': 'ANSWER: ' + '; '.join(task.expect_names(truth))}
        return {'text': 'ANSWER: ' + ' '.join('%s="%s"' % (k if k != 'border-top' else 'border', v)
                                              for k, v in task.expect(truth).items())}
    if task.kind == 'refuse':
        return {'text': 'REFUSE: this format cannot express it.'}
    new = list(lines)
    for key in task.targets(truth):
        o = obj(objs, key)
        i = o['lines'][0]
        f = fmt_of(o)
        if task.id in GOLD_BOX:
            f.update(V.expand(dict(GOLD_BOX[task.id])))
            new[i] = retag(lines[i], f)
        elif getattr(task, 'box_rel', None):
            ft = fmt_of(obj(truth, key))
            for sd in V.SIDES:
                w, st, col = V.border_parts(ft[sd])
                w2 = 3 if task.id == 'pp-e4' and DECK_REP[0] != 3 else 2 * w
                f[sd] = '%s %s %s' % (V.pt(w2), st, col)
            new[i] = retag(lines[i], f)
        elif getattr(task, 'span', None):
            word = task.span[0]
            for j in o['lines']:
                if word in new[j]:
                    new[j] = new[j].replace(word, '[%s]{size=24pt color=#FF7F50}' % word, 1)
                    break
        elif getattr(task, 'retext', None):
            if getattr(task, 'retext_one', False) and key != task.targets(truth)[0]:
                continue
            for j in o['lines']:
                if task.retext[0] in new[j]:
                    new[j] = new[j].replace(task.retext[0], task.retext[1], 1)
    return {'edits': __import__('build').make_edits(text, '\n'.join(new))}


DECK_REP = [0]


# ---------------------------------------------------------------- building

SHARED = """A presentation file is plain text. It begins with a front matter block between two `---` lines; keep it as it is. `size` in the front matter is the slide's width and height in points. Slides follow, separated by a line holding only `---`. The first line of every slide is `layout: Name`.

### Objects

A slide is a list of objects written back to front: an object written later is drawn on top. Every object shows where it is: `box="x y w h"` is its left edge, top edge, width and height in points from the slide's top-left corner; `rot` turns it, `flip` mirrors it. Leave ids, names and boxes as they are.

- **Slots** are the layout's placeholders: a marker line `::title box="…"::`, then the slot's text, one line per paragraph (`- ` for a bullet), up to the next object or `---`.
- **Shapes** are one line each, `<shape id="s4" name="…" box="…">text</shape>`, or `<shape … />` without text; `<p/>` starts the shape's next paragraph.
- **Lines and connectors** are `<line id="…" name="…" from="x y" to="x y"/>`.
- **Groups**: a `<group …>` line, the group's objects, then `</group>`.
- **Pictures, charts, tables** are `<keep …/>` lines: kept for you, never changed here.
- Text: `**bold**`, `*italic*`, `<u>underline</u>`, `<br/>` a line break. Speaker notes follow `::notes::`.
"""

VOCAB = """### The vocabulary

`key=value` pairs separated by spaces, on the object's tag or slot marker; a value with a space is quoted. Lengths are points (`12pt`).

- colours: `#RRGGBB`, or a theme colour `accent1` … `accent6`, `tx1`, `bg1`, `tx2`, `bg2`, `hlink`; `accent1+40%` is 40% lighter and `accent1-25%` 25% darker; `accent1*` is the theme colour with another adjustment the file keeps while it is left as written; `/55%` after a colour is its opacity
- shape and line: `fill` (a colour, or `none`), `border` (the outline: `"<width>pt <style> <colour>"` with style solid, dashed, dotted or double, or `none`)
- text: `font`, `size`, `color`, `bold`; paragraph: `align` (left, center, right, justify), `indent-left`, `first-line`, `space-before`, `space-after`, `line-spacing` (`90%`, `14pt`)
- `fill=gradient`, `fill=pattern` and `fill=picture` are kept as they are while left as written, and can be replaced by a colour; they cannot be written or changed. Shadows, glow, 3-D and other effects cannot be expressed.
"""

PART = {
    'F1': """### Formatting

Every object shows how it looks: its formatting is written on its tag (a shape, a line) or its slot marker, whether the object sets it itself or takes it from the theme or the layout. A property that is not written is not there: no `fill` means no fill, no `border` means no outline.

- Text properties shared by all of an object's text are on its tag or marker; a paragraph's own are in `{…}` at its end (before `<p/>` or `</shape>` in a shape, at the end of the line in a slot); `[text]{…}` gives a stretch of text its own `size`, `color` or `font`.
- To change how an object looks, change or add the property on its tag.

Example: `<shape id="s3" name="Card" box="64 130 260 320" fill=accent1 border="2pt solid accent1-50%" size=18pt color=#FFFFFF>**Cities** {size=24pt}<p/>Low-emission [zones]{color=#FF6B5B} in 40 cities</shape>`
""",
    'F2o': """### Formatting

An object shows the formatting it sets itself, on its tag (a shape, a line) or its slot marker. What it takes from elsewhere, the theme's shape style or the layout's text styles, is not written here: a property that is not written comes from there, and may be none.

- Text properties the object sets for all of its text are on its tag or marker; a paragraph's own are in `{…}` at its end (before `<p/>` or `</shape>` in a shape, at the end of the line in a slot); `[text]{…}` gives a stretch of text its own `size`, `color` or `font`.
- To change how an object looks, write the property on its tag: the object then sets it itself.

Example: `<shape id="s3" name="Card" box="64 130 260 320" fill=accent1 border="2pt solid accent1-50%" size=18pt color=#FFFFFF>**Cities** {size=24pt}<p/>Low-emission [zones]{color=#FF6B5B} in 40 cities</shape>`
""",
}


def build():
    import build as B
    for d in ('seeds', 'prompts', os.path.join('data', 'units'), os.path.join('data', 'gold'), 'guides'):
        os.makedirs(os.path.join(HERE, d), exist_ok=True)
    index = []
    for c in CANDS:
        open(os.path.join(HERE, 'guides', 'deck-%s.md' % c), 'w', encoding='utf-8').write(
            SHARED + '\n' + PART[c] + '\n' + VOCAB)
    for rep in DECKS:
        texts = {c: canvas.render(deck_path(rep), today(rep), c) for c in CANDS}
        truth = parse(texts['F1'])
        global ONLY_Q2
        if rep == 3:
            ONLY_Q2 = [sid for sid, ln in group_members(texts['F1'], 'g752157556') if '1.5pt' in ln]
        for c in CANDS:
            uid = 'r6p-%s-%d' % (c, rep)
            open(os.path.join(HERE, 'seeds', uid + '.txt'), 'w', encoding='utf-8').write(texts[c])
            parse(texts[c])
            unreachable = [t.id for t in tasks(rep) if t.kind != 'refuse' and not visible(t, texts, c)]
            json.dump({'unit_id': uid, 'candidate': c, 'seed': rep, 'name': DECKS[rep]['name'], 'format': 'pptx',
                       'unreachable': unreachable, 'q2': ONLY_Q2 if rep == 3 else None,
                       'tasks': [{'task_id': t.id, 'kind': t.kind, 'text': t.text} for t in tasks(rep)]},
                      open(os.path.join(HERE, 'data', 'units', uid + '.json'), 'w', encoding='utf-8'),
                      ensure_ascii=False, indent=1)
            ts = '\n'.join('%d. `%s` (%s): %s' % (k + 1, t.id, 'edit' if t.kind == 'refuse' else t.kind, t.text)
                           for k, t in enumerate(tasks(rep)))
            prompt = B.HEADER.format(n=len(tasks(rep)), guide=SHARED + '\n' + PART[c] + '\n' + VOCAB,
                                     file=texts[c], tasks=ts)
            prompt = prompt.replace('A document file', 'A presentation file')
            open(os.path.join(HERE, 'prompts', uid + '-a.md'), 'w', encoding='utf-8').write(prompt)
            index.append({'unit_id': uid, 'part': 'a', 'candidate': c, 'seed': rep, 'file_chars': len(texts[c]),
                          'prompt_chars': len(prompt), 'max_line': max(len(x) for x in prompt.split('\n')),
                          'tasks': [t.id for t in tasks(rep)], 'unreachable': unreachable})
            DECK_REP[0] = rep
            g = {'answers': [dict(gold(t, texts[c], truth, c), task_id=t.id) for t in tasks(rep)
                             if t.id not in unreachable]}
            json.dump(g, open(os.path.join(HERE, 'data', 'gold', uid + '.json'), 'w', encoding='utf-8'),
                      ensure_ascii=False, indent=1)
            for r in score_answer(uid, g):
                if r['task_id'] in unreachable:
                    continue
                assert r['landed'], 'gold does not land: %s %s %s %s' % (uid, r['task_id'], r['error'], r['detail'])
            print(uid, 'file', len(texts[c]), 'prompt', len(prompt), 'unreachable', unreachable)
    json.dump(index, open(os.path.join(HERE, 'data', 'index-pptx.json'), 'w', encoding='utf-8'), indent=1)


if __name__ == '__main__':
    if sys.argv[1] == 'build':
        build()
    else:
        if int(re.search(r'-(\d)$', sys.argv[2]).group(1)) == 3:
            ONLY_Q2 = json.load(open(os.path.join(HERE, 'data', 'units', sys.argv[2] + '.json')))['q2']
        print(json.dumps({'results': score_answer(sys.argv[2], json.load(open(sys.argv[3], encoding='utf-8')))},
                         ensure_ascii=False, indent=1))
