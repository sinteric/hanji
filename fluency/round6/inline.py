"""Inline text of one paragraph: hanji's marks plus the candidates' run spans `[text]{attrs}`.

tokenize(s) -> tokens
  ('c', ch, raw)      a character of the text (raw is its spelling: an escape, or <br/> for a line break)
  ('m', kind, raw)    a mark toggle: kind in bold, italic, strike, u+ (open <u>), u- (close </u>)
  ('x', raw)          zero-width markup kept as it is (<keep/>, <field …>, </field>, footnote refs, math)
  ('[',) (']', attrs) a run span (only when spans=True)
Marks never cross a span in canonical text: render_spans splits a mark at a span edge when it would."""
import re

import vocab as V

KEEP_RE = re.compile(r'<keep\b[^>]*/>|<field\b[^>]*>|</field>|\[\^[^\]]+\]|<math>.*?</math>')
SPAN_RE = re.compile(r'\](\{([^{}]*)\})')


def tokenize(s, spans=False):
    out = []
    i = 0
    n = len(s)
    while i < n:
        ch = s[i]
        if ch == '\\' and i + 1 < n:
            out.append(('c', s[i + 1], s[i:i + 2]))
            i += 2
            continue
        m = KEEP_RE.match(s, i)
        if m:
            out.append(('x', m.group(0)))
            i = m.end()
            continue
        if s.startswith('<br/>', i):
            out.append(('c', '\n', '<br/>'))
            i += 5
            continue
        if s.startswith('<u>', i):
            out.append(('m', 'u+', '<u>'))
            i += 3
            continue
        if s.startswith('</u>', i):
            out.append(('m', 'u-', '</u>'))
            i += 4
            continue
        if s.startswith('~~', i):
            out.append(('m', 'strike', '~~'))
            i += 2
            continue
        if ch == '*':
            j = i
            while j < n and s[j] == '*':
                j += 1
            k = j - i
            for _ in range(k // 2):
                out.append(('m', 'bold', '**'))
            if k % 2:
                out.append(('m', 'italic', '*'))
            i = j
            continue
        if spans and ch == '[':
            close = find_span_close(s, i)
            if close is not None:
                out.append(('[',))
                inner_end, attrs, end = close
                out.extend(tokenize(s[i + 1:inner_end], False))
                out.append((']', attrs))
                i = end
                continue
        if spans and ch == '{' and re.match(r'\{\s*(%s)(=|\s|\})' % '|'.join(sorted(V.KEYS, key=len, reverse=True)), s[i:]):
            raise V.VocabError('a `{` inside the text starts nothing here: run formatting is written '
                               '[text]{key=value}, paragraph formatting as {key=value} at the end of the line; write '
                               '\\{ for a literal brace')
        out.append(('c', ch, ch))
        i += 1
    return out


def find_span_close(s, i):
    """At s[i] == '[': -> (index of ']', attrs text, end) when a `](`…`)` span closes it, else None."""
    depth = 0
    j = i + 1
    while j < len(s):
        c = s[j]
        if c == '\\':
            j += 2
            continue
        if c == '[':
            return None
        if c == ']':
            if s.startswith(']{', j):
                k = s.find('}', j + 2)
                if k < 0:
                    return None
                return j, s[j + 2:k], k + 1
            return None
        j += 1
    return None


def plain(tokens):
    return ''.join(t[1] for t in tokens if t[0] == 'c')


def zero_width(tokens):
    return [t[1] for t in tokens if t[0] == 'x']


def flags_runs(tokens, span_parse=None):
    """-> list of (char, flags set, span attrs dict) per character, from marks and spans."""
    state = set()
    span = None
    out = []
    for t in tokens:
        if t[0] == 'c':
            out.append((t[1], frozenset(state), span))
        elif t[0] == 'm':
            k = t[1]
            if k == 'u+':
                state.add('underline')
            elif k == 'u-':
                state.discard('underline')
            elif k == 'bold+italic':
                for f in ('bold', 'italic'):
                    state.symmetric_difference_update({f})
            else:
                state.symmetric_difference_update({k})
        elif t[0] == '[':
            span = {}
        elif t[0] == ']':
            span = None
            # attrs were attached when the span opened; see runs_of
    return out


def runs_of(s, spans=True, where=''):
    """Inline text -> (plain text, [(text, flags frozenset, span props dict)] merged, zero-width markup list)."""
    toks = tokenize(s, spans=spans)
    state = set()
    cur_span = None
    chars = []
    stack = []
    for t in toks:
        if t[0] == 'c':
            chars.append((t[1], frozenset(state), cur_span))
        elif t[0] == 'm':
            k = t[1]
            if k == 'u+':
                state.add('underline')
            elif k == 'u-':
                state.discard('underline')
            elif k == 'bold+italic':
                for f in ('bold', 'italic'):
                    state.symmetric_difference_update({f})
            else:
                state.symmetric_difference_update({k})
        elif t[0] == '[':
            stack.append(len(chars))
            cur_span = {}
        elif t[0] == ']':
            props = V.parse_attrs(t[1], where)
            bad = [k for k in props if k not in V.CHAR_KEYS]
            if bad:
                raise V.VocabError('%sa run span [text]{…} holds text properties only (%s), not %s'
                                   % (where, ', '.join(V.CHAR_KEYS), ', '.join(bad)))
            start = stack.pop()
            for idx in range(start, len(chars)):
                c, f, _ = chars[idx]
                chars[idx] = (c, f, tuple(sorted(props.items())))
            cur_span = None
    runs = []
    for c, f, sp in chars:
        sp = sp if isinstance(sp, tuple) else ()
        if runs and runs[-1][1] == f and runs[-1][2] == sp:
            runs[-1] = (runs[-1][0] + c, f, sp)
        else:
            runs.append((c, f, sp))
    return ''.join(c for c, _, _ in chars), [(t, f, dict(sp)) for t, f, sp in runs], zero_width(toks)


def strip_spans(s):
    """Inline text without its run spans: `[ab]{color=red}` -> `ab`."""
    out = []
    i = 0
    while i < len(s):
        if s[i] == '\\' and i + 1 < len(s):
            out.append(s[i:i + 2])
            i += 2
            continue
        if s[i] == '[':
            c = find_span_close(s, i)
            if c is not None:
                inner_end, _, end = c
                out.append(s[i + 1:inner_end])
                i = end
                continue
        out.append(s[i])
        i += 1
    return ''.join(out)


def render_spans(s, segments):
    """Insert run spans into hanji inline text s (which has none).

    segments: [(start, end, attrs text)] over the plain text, non-overlapping and sorted. The paragraph's text is
    written again from its characters: a span edge closes the marks open there and reopens them inside (or after)
    the span, so marks never cross a span."""
    if not segments:
        return s
    toks = tokenize(s)
    chars = []      # (raw, flags)
    zw = {}         # plain index -> [raw] of zero-width markup before that char
    state = []
    for t in toks:
        if t[0] == 'c':
            chars.append((t[2], tuple(state)))
        elif t[0] == 'm':
            for kk in mark_kinds(t[1]):
                if kk in state and t[1] != 'u+':
                    state.remove(kk)
                elif t[1] != 'u-':
                    state.append(kk)
        else:
            zw.setdefault(len(chars), []).append(t[1])
    starts = {a: (b, attrs) for a, b, attrs in segments}
    out = []
    cur = []
    span_end = None

    def close_all():
        for kk in reversed(cur):
            out.append(CLOSE[kk])
        cur.clear()

    def set_flags(fl):
        # close what is not wanted (from the top), open what is missing
        while cur and (cur[-1] not in fl or any(x not in fl for x in cur)):
            kk = cur.pop()
            out.append(CLOSE[kk])
        for kk in ORDER_M:
            if kk in fl and kk not in cur:
                out.append(OPEN[kk])
                cur.append(kk)

    for i in range(len(chars) + 1):
        if span_end is not None and i == span_end:
            close_all()
            out.append(']{%s}' % span_attrs)
            span_end = None
        if i in starts and i < len(chars):
            close_all()
            out.append('[')
            span_end, span_attrs = starts[i]
        for r in zw.get(i, []):
            out.append(r)
        if i == len(chars):
            break
        raw, fl = chars[i]
        set_flags(fl)
        if span_end is not None and raw in ('[', ']'):
            raw = '\\' + raw
        out.append(raw)
    close_all()
    return ''.join(out)


OPEN = {'bold': '**', 'italic': '*', 'strike': '~~', 'underline': '<u>'}
CLOSE = {'bold': '**', 'italic': '*', 'strike': '~~', 'underline': '</u>'}
ORDER_M = ('bold', 'italic', 'strike', 'underline')


def mark_kinds(k):
    if k in ('u+', 'u-'):
        return ['underline']
    if k == 'bold+italic':
        return ['bold', 'italic']
    return [k]
