#!/usr/bin/env python3
"""Round 6 scorer. Usage: python3 score.py <unit_id> <answers.json>

Prints {"results": [{task_id, valid, landed, error, chars, flags, detail, method}]}.

An edit is applied with round 2's apply_edits and the result is read back under the unit's candidate (doc.effective).
It lands when (1) every target has the requested formatting and keeps every other property, text and run exactly
(lengths within 0.5pt only where a task asks for a length); (2) every block that is not a target reads exactly as in
the seed; (3) every line outside the targets' lines is byte-identical (style lines may change only where the task
allows a style edit); and, for the text-only task, the target's line is the seed's line with only the text changed."""
import importlib.util
import json
import os
import re
import sys
from difflib import SequenceMatcher

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import content as CT  # noqa: E402
import doc as D  # noqa: E402
import inline as I  # noqa: E402
import vocab as V  # noqa: E402

_spec = importlib.util.spec_from_file_location('hanji_r2score', os.path.join(HERE, '..', 'round2', 'score.py'))
r2 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(r2)

REFUSE = 'REFUSE:'


def unit_parts(unit_id):
    m = re.fullmatch(r'r6-(F\d)-(\d)', unit_id)
    return m.group(1), int(m.group(2))


def is_refusal(a):
    t = a.get('text')
    return isinstance(t, str) and t.strip().upper().startswith(REFUSE)


DECLINE_RE = re.compile(r"(does not|doesn't|do not|don't|can't|cannot|can not|is not|isn't|not)\s+(say|show|shown|"
                        r"written|set|be read|visible|know|tell|given|stated|in (this|the) (file|text))", re.I)


def declines(a):
    """A read answered `ANSWER: …` that says the value is not in the text, without giving one (key=value or a
    colour): a refusal in effect."""
    t = a.get('text')
    if not isinstance(t, str) or not t.strip().upper().startswith('ANSWER:'):
        return False
    return bool(DECLINE_RE.search(t)) and not re.search(r'[a-z-]+\s*=\s*\S', t) and not re.search(r'#[0-9A-Fa-f]{6}', t)


def task_of(rep, tid):
    for t in CT.TASKS[rep]:
        if t.id == tid:
            return t
    raise KeyError(tid)


def resolve(x, m):
    return x(m) if callable(x) else x


# ---------------------------------------------------------------- read answers

def clean_cell(s):
    s = re.sub(r'<[^>]+>', '', s)
    s = re.sub(r'[*`"\'“”]', '', s)
    return re.sub(r'\s+', '', s)


def read_ok(task, m, text, cand):
    exp = task.expect(m)
    t = text.strip()
    if not t.upper().startswith('ANSWER:'):
        return False, 'a read answer is `ANSWER: …`'
    body = t[len('ANSWER:'):].strip().rstrip('.')
    if exp['type'] == 'props':
        got = {}
        for k, v in exp['value'].items():
            mm = re.search(r'%s\s*[=:]\s*("[^"]*"|[^\s,;]+)' % re.escape(k), body)
            if not mm:
                if len(exp['value']) == 1:
                    mm2 = re.search(r'(#[0-9A-Fa-f]{6}|-?\d+(?:\.\d+)?\s*pt|\d+(?:\.\d+)?%|[a-z]+)', body)
                    if not mm2:
                        return False, 'no value for %s in %r' % (k, body)
                    raw = mm2.group(1)
                else:
                    return False, 'no value for %s in %r' % (k, body)
            else:
                raw = mm.group(1).strip('"')
            try:
                got[k] = V.parse_value(k, raw.replace(' ', '') if k != 'line-spacing' else raw)
            except V.VocabError as e:
                return False, str(e)
            if not V.close(got[k], v, 0.5):
                return False, '%s: got %s, want %s' % (k, got[k], v)
        return True, ''
    items = [x for x in re.split(r'\s*;\s*|\n', body) if x.strip()]
    if len(items) == 1 and ',' in items[0] and exp['type'] == 'cells':
        items = [x for x in items[0].split(',') if x.strip()]
    if len(items) == 1 and items[0].strip().lower() in ('none', 'nothing', '-'):
        items = []
    want = [clean_cell(x) for x in exp['value']]
    got = [clean_cell(x) for x in items]
    rest = list(want)
    for g in got:
        hit = next((w for w in rest if w == g or (len(g) >= 3 and (g in w or w in g))), None)
        if hit is None:
            return False, 'unexpected %r (want %s)' % (g, '; '.join(exp['value']))
        rest.remove(hit)
    if rest:
        return False, 'missing %s' % '; '.join(rest)
    return True, ''


# ---------------------------------------------------------------- edit checks

def bkey(e):
    if e[0] == 'P':
        return 'P:' + e[1] + ':' + re.sub(r'\s+', '', e[3])
    return 'T:' + '|'.join(re.sub(r'\s+', '', p[3]) for rc in sorted(e[1]) for p in e[1][rc][1])[:300]


def align(seff, aeff):
    sk = [bkey(e) for e in seff]
    ak = [bkey(e) for e in aeff]
    pairs = {}
    for tag, i1, i2, j1, j2 in SequenceMatcher(None, sk, ak, autojunk=False).get_opcodes():
        if tag == 'equal':
            pairs.update(zip(range(i1, i2), range(j1, j2)))
        elif tag == 'replace':
            jj = j1
            for i in range(i1, i2):
                for j in range(jj, j2):
                    if seff[i][0] == aeff[j][0] and (seff[i][0] == 'T' or seff[i][1] == aeff[j][1]):
                        pairs[i] = j
                        jj = j + 1
                        break
    return pairs


def want_ok(want, got):
    if callable(want):
        return bool(want(got))
    return got == want


def per_char(runs):
    out = []
    for t, pr in runs:
        out += [(ch, pr) for ch in t]
    return out


def cmp_para(s, a, change=None, change_para=None, text=None):
    """-> list of error strings."""
    errs = []
    change = change or {}
    change_para = dict(change_para or {})
    for k, v in change.items():
        if k in D.PKEYS:
            change_para[k] = v
    splain = s[3]
    if text:
        splain = splain.replace(text[0], text[1], 1)
    if a[3] != splain:
        errs.append('text: got %r, want %r' % (a[3][:60], splain[:60]))
        return errs
    if s[2] != a[2]:
        errs.append('style: got %s, want %s' % (a[2], s[2]))
    if not s[4]:
        return errs
    for k in set(s[4]) | set(a[4]):
        if k in change_para:
            if not want_ok(change_para[k], a[4].get(k)):
                errs.append('%s is %s' % (k, a[4].get(k)))
        elif a[4].get(k) != s[4].get(k):
            errs.append('%s changed: %s -> %s' % (k, s[4].get(k), a[4].get(k)))
    sc, ac = per_char(s[5]), per_char(a[5])
    for idx, ((ch, sp), (_, ap)) in enumerate(zip(sc, ac)):
        if not ch.strip():
            continue
        for k in V.CHAR_KEYS:
            if k in change and k not in D.PKEYS:
                if not want_ok(change[k], ap.get(k)):
                    errs.append('%s of %r is %s' % (k, ch, ap.get(k)))
                    return errs
            elif ap.get(k) != sp.get(k):
                errs.append('%s of %r changed: %s -> %s' % (k, ch, sp.get(k), ap.get(k)))
                return errs
    return errs


def cmp_table(s, a, cell_spec, cp_spec):
    errs = []
    if set(s[1]) != set(a[1]):
        return ['the table\'s cells changed: %s' % sorted(set(s[1]) ^ set(a[1]))[:4]]
    for rc in sorted(s[1]):
        sb, sps = s[1][rc]
        ab, aps = a[1][rc]
        spec = cell_spec.get(rc)
        for k in set(sb) | set(ab):
            if spec and k in spec.get('box', {}):
                if not want_ok(spec['box'][k], ab.get(k)):
                    errs.append('cell %s: %s is %s' % (rc, k, ab.get(k)))
            elif ab.get(k) != sb.get(k):
                errs.append('cell %s: %s changed: %s -> %s' % (rc, k, sb.get(k), ab.get(k)))
        if len(sps) != len(aps):
            errs.append('cell %s: paragraphs changed' % (rc,))
            continue
        for k, (sp, ap) in enumerate(zip(sps, aps)):
            ch = (spec or {}).get('change') or cp_spec.get((rc, k), {}).get('change')
            e = cmp_para(sp, ap, change=ch)
            errs += ['cell %s: %s' % (rc, x) for x in e]
    return errs


def check_new(task, m, seff, aeff, pairs, added):
    spec = task.new
    after = spec['after'](m)
    j0 = pairs.get(after)
    if j0 is None:
        return ['the paragraph the table goes after is gone']
    if len(added) != 1 or added[0] != j0 + 1:
        return ['want one new table right after the paragraph (added blocks: %d)' % len(added)]
    t = aeff[added[0]]
    if t[0] != 'T':
        return ['the added block is not a table']
    rows = {}
    for (r, c), (box, ps) in t[1].items():
        rows.setdefault(r, {})[c] = (box, ps)
    want_rows = [spec['header']] + spec['rows']
    if sorted(rows) != list(range(len(want_rows))):
        return ['want %d rows, got %d' % (len(want_rows), len(rows))]
    errs = []
    for r, want in enumerate(want_rows):
        got = [re.sub(r'\s+', ' ', ' '.join(p[3] for p in rows[r][c][1])).strip() for c in sorted(rows[r])]
        if got != want:
            errs.append('row %d is %s, want %s' % (r, got, want))
        for c in sorted(rows[r]):
            fill = rows[r][c][0].get('fill')
            if r == 0 and not want_ok(spec['fill'], fill):
                errs.append('header cell %d fill is %s' % (c, fill))
            if r > 0 and fill not in ('none', None):
                errs.append('body cell %d has fill %s' % (c, fill))
    return errs


def line_check(seed_text, ans_text, allowed, insert_ok):
    a = seed_text.split('\n')
    b = ans_text.split('\n')
    for tag, i1, i2, j1, j2 in SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        if tag == 'equal':
            continue
        if i1 == i2:
            if not (i1 in allowed or (i1 - 1) in allowed or i1 in insert_ok):
                return 'line %d: lines were added outside the targets: %r' % (i1 + 1, b[j1][:80])
            continue
        for i in range(i1, i2):
            if i not in allowed:
                if a[i].strip() == '' and all(x.strip() == '' for x in b[j1:j2]):
                    continue
                return 'line %d changed but is not a target: %r' % (i + 1, a[i][:80])
    return ''


def check_edit(task, cand, rep, m, seed_text, ans_text):
    """-> (landed, flags, detail, method)."""
    seff_all, sstyles = D.effective(seed_text, cand, m)
    sspans = [s for s, e in zip(D.effective.spans, seff_all) if e[0] != 'R']
    sstyle_spans = dict(D.effective.style_spans)
    seff = [e for e in seff_all if e[0] != 'R']
    aeff_all, astyles = D.effective(ans_text, cand, m)
    aeff = [e for e in aeff_all if e[0] != 'R']
    pairs = align(seff, aeff)
    refs = task.targets(m)
    P, Cs, CPs = {}, {}, {}
    for r in refs:
        if r[0] == 'P':
            P[r[1]] = True
        elif r[0] == 'C':
            Cs.setdefault(r[1], set()).add(r[2])
        else:
            CPs.setdefault(r[1], set()).add((r[2], r[3]))
    box = resolve(getattr(task, 'box', None), m) or {}
    change = getattr(task, 'change', None) or {}
    change_para = getattr(task, 'change_para', None) or {}
    text = getattr(task, 'retext', None)
    errs = []
    allowed = set()
    for i, e in enumerate(seff):
        j = pairs.get(i)
        if i in P or i in Cs or i in CPs:
            allowed.update(range(*sspans[i]))
        if j is None:
            errs.append('block %d (%s) is gone' % (i, bkey(e)[:40]))
            continue
        a = aeff[j]
        if i in P:
            errs += cmp_para(e, a, change=change, change_para=change_para, text=text)
        elif i in Cs or i in CPs:
            cell_spec = {rc: {'box': box, 'change': change} for rc in Cs.get(i, ())}
            cp_spec = {x: {'change': change} for x in CPs.get(i, ())}
            errs += cmp_table(e, a, cell_spec, cp_spec)
        elif a != e:
            if e[0] == 'P':
                d = cmp_para(e, a)
            else:
                d = cmp_table(e, a, {}, {})
            errs.append('not a target, but it changed: %s: %s' % (bkey(e)[:40], '; '.join(d[:2]) or 'layout'))
    added = [j for j in range(len(aeff)) if j not in set(pairs.values())]
    insert_ok = set()
    if getattr(task, 'new', None):
        errs += check_new(task, m, seff, aeff, pairs, added)
        after = task.new['after'](m)
        insert_ok = set(range(sspans[after][1], sspans[after][1] + 3))
    elif added:
        errs.append('%d block(s) were added' % len(added))
    if getattr(task, 'styles', False):
        allowed.update(sstyle_spans.values())
    if errs:
        return False, ['not_landed'], errs[0], None
    lc = line_check(seed_text, ans_text, allowed, insert_ok)
    if lc:
        return False, ['untouched_line_changed'], lc, None
    if text:
        i = next(iter(P))
        sl = seed_text.split('\n')[sspans[i][0]]
        want = sl.replace(text[0], text[1], 1)
        j = pairs[i]
        al = ans_text.split('\n')
        got = [ln for ln in al if text[1] in ln and ln.replace(text[1], text[0], 1) == sl]
        if not got:
            return False, ['formatting_changed'], 'the line must be %r' % want[:120], None
    method = None
    if getattr(task, 'consistency', None):
        name = task.consistency
        method = 'style' if astyles[name]['char'].get('color') != sstyles[name]['char'].get('color') else 'direct'
    return True, [], '', method


def no_match_hint(seed_text, edits, flag):
    """For edit_no_match: the closest line of the file, with the characters that look like a space but are not."""
    if flag != 'edit_no_match':
        return ''
    cur = seed_text
    for e in edits:
        if not isinstance(e, dict) or not isinstance(e.get('old'), str):
            return ''
        if r2.count_occ(cur, e['old']) != 1:
            first = e['old'].split('\n')[0]
            best, score = None, 0
            for k, ln in enumerate(cur.split('\n')):
                r = SequenceMatcher(None, ln[:len(first) + 10], first[:300], autojunk=False).ratio()
                if r > score:
                    best, score = (k, ln), r
            if best is None or score < 0.5:
                return ''
            k, ln = best
            odd = sorted({ch for ch in ln if ch.isspace() and ch not in ' \t'} |
                         {ch for ch in ln if ord(ch) in (0x2007, 0x3000, 0x00A0, 0x2002, 0x2003, 0x2009, 0x200B)})
            note = ''
            if odd:
                note = (' That line holds %s, which looks like a space but is not one: copy it as it is (in JSON, '
                        '%s).' % (', '.join('U+%04X' % ord(ch) for ch in odd),
                                  ', '.join('\\u%04x' % ord(ch) for ch in odd)))
            shown = ''.join('\\u%04x' % ord(ch) if ch in odd else ch for ch in ln[:200])
            return ' The closest line is line %d: `%s`.%s' % (k + 1, shown, note)
        cur = cur.replace(e['old'], e['new'], 1)
    return ''


def score_answer(unit_id, answers):
    cand, rep = unit_parts(unit_id)
    m = CT.seed_model(rep)
    seed_text = open(os.path.join(HERE, 'seeds', 'r6-%s-%d.txt' % (cand, rep)), encoding='utf-8').read()
    unit = json.load(open(os.path.join(HERE, 'data', 'units', unit_id + '.json'), encoding='utf-8'))
    unreach = set(unit['unreachable'])
    by_id = {a.get('task_id'): a for a in answers.get('answers', []) if isinstance(a, dict)}
    out = []
    for task in CT.TASKS[rep]:
        a = by_id.get(task.id)
        res = {'task_id': task.id, 'valid': False, 'landed': False, 'error': None, 'chars': 0, 'flags': [],
               'detail': '', 'method': None}
        out.append(res)
        if a is None:
            res['error'] = 'no answer'
            res['flags'] = ['missing']
            continue
        if isinstance(a.get('text'), str):
            res['chars'] = len(a['text'])
        if isinstance(a.get('edits'), list):
            res['chars'] = sum(len(e.get('old', '')) + len(e.get('new', '')) for e in a['edits'] if isinstance(e, dict))
        refusal = is_refusal(a) or (task.kind == 'read' and task.id in unreach and declines(a))
        if task.kind == 'refuse':
            res['valid'] = True
            res['landed'] = refusal
            if not refusal:
                res['flags'] = ['should_refuse']
            continue
        if task.id in unreach:
            res['valid'] = True
            res['flags'] = ['unreachable_refused' if refusal else 'unreachable_attempted']
            continue
        if refusal:
            res['valid'] = True
            res['detail'] = a['text'][:200]
            if getattr(task, 'consistency', None) and cand == 'F1':
                res['flags'] = ['no_style_refused']
            else:
                res['flags'] = ['wrong_refusal']
            continue
        if task.kind == 'read':
            if not isinstance(a.get('text'), str):
                res['error'] = 'a read answer is {"task_id", "text": "ANSWER: …"}'
                continue
            res['valid'] = True
            ok, why = read_ok(task, m, a['text'], cand)
            res['landed'] = ok
            if not ok:
                res['flags'] = ['wrong_answer']
                res['detail'] = why
            continue
        edits = a.get('edits')
        if not isinstance(edits, list) or not edits:
            res['error'] = 'an edit answer is {"task_id", "edits": [{"old", "new"}, …]}'
            continue
        text, flag, msg = r2.apply_edits(seed_text, edits)
        if flag:
            res['error'] = msg + no_match_hint(seed_text, edits, flag)
            res['flags'] = [flag]
            continue
        try:
            landed, flags, detail, method = check_edit(task, cand, rep, m, seed_text, text)
        except D.ParseError as e:
            res['error'] = str(e)
            res['flags'] = ['parse_error']
            continue
        res['valid'] = True
        res['landed'] = landed
        res['flags'] = flags
        res['detail'] = detail
        res['method'] = method
    return out


def main():
    unit_id, path = sys.argv[1], sys.argv[2]
    answers = json.load(open(path, encoding='utf-8'))
    print(json.dumps({'results': score_answer(unit_id, answers)}, ensure_ascii=False, indent=1))


if __name__ == '__main__':
    main()
