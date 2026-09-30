"""The round 6 formatting vocabulary: one small, fixed set of properties for docx, hwpx, pptx and xlsx.

A property set is a dict {key: canonical value string}. Values are parsed and written here, so every candidate and
every format shares one spelling:

  lengths      12pt, 0.34pt (points, at most two decimals; a bare number is points)
  colours      #RRGGBB, or a theme colour: tx1 bg1 tx2 bg2 accent1..accent6 hlink (dk1, lt1, text1, … read as
               aliases), optionally lighter or darker: accent1+40% (lighter 40%), accent1-25% (darker 25%), or
               accent1* (another transform, kept while left as written); /55% is opacity
  borders      none | "<width> <style> <colour>", style one of solid dashed dotted double dash-dot dash-dot-dot
               (shown-only styles, kept when left as shown: triple thin-thick thick-thin wave 3d)
  fill         a colour, or the shown-only tokens pattern / gradient / picture (kept when left as shown)
  line-spacing 160% (proportional), 14pt (exact), "at-least 14pt"
  align        left center right justify distribute     valign  top middle bottom
  flags        bold italic underline strike (bare = on, bold=no = off)

Standard library only."""
import colorsys
import re

PARA_KEYS = ('align', 'indent-left', 'indent-right', 'first-line', 'space-before', 'space-after', 'line-spacing')
CHAR_KEYS = ('font', 'size', 'color', 'bold', 'italic', 'underline', 'strike')
BOX_KEYS = ('fill', 'border', 'border-top', 'border-right', 'border-bottom', 'border-left', 'valign')
SIDES = ('border-top', 'border-right', 'border-bottom', 'border-left')
FLAGS = ('bold', 'italic', 'underline', 'strike')
XLSX_KEYS = ('indent',)
ORDER = BOX_KEYS + PARA_KEYS + CHAR_KEYS + XLSX_KEYS
KEYS = set(ORDER) | {'style'}
LENGTH_KEYS = ('indent-left', 'indent-right', 'first-line', 'space-before', 'space-after', 'size')

THEME = ('tx1', 'bg1', 'tx2', 'bg2', 'accent1', 'accent2', 'accent3', 'accent4', 'accent5', 'accent6', 'hlink')
THEME_ALIAS = {'dk1': 'tx1', 'lt1': 'bg1', 'dk2': 'tx2', 'lt2': 'bg2', 'text1': 'tx1', 'background1': 'bg1',
               'text2': 'tx2', 'background2': 'bg2', 'hyperlink': 'hlink', 'folHlink': 'hlink'}
BORDER_STYLES = ('solid', 'dashed', 'dotted', 'double', 'dash-dot', 'dash-dot-dot')
SHOWN_ONLY_STYLES = ('triple', 'thin-thick', 'thick-thin', 'wave', '3d')
FILL_TOKENS = ('pattern', 'gradient', 'picture')
ALIGN = ('left', 'center', 'right', 'justify', 'distribute')
VALIGN = ('top', 'middle', 'bottom')


class VocabError(ValueError):
    pass


def num(x):
    """Canonical number: at most two decimals, no trailing zeros."""
    s = ('%.2f' % float(x)).rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s


def pt(x):
    return num(x) + 'pt'


def parse_len(v, key='length'):
    m = re.fullmatch(r'\s*(-?\d+(?:\.\d+)?)\s*(pt)?\s*', str(v))
    if not m:
        raise VocabError('%s: expected a length in points such as 10pt, got %r' % (key, v))
    return float(m.group(1))


# ---------------------------------------------------------------- colours

COLOR_RE = re.compile(r'(#[0-9A-Fa-f]{6}|[a-zA-Z]+\d?)(?:([+-])(\d{1,3})%|(\*))?(?:/(\d{1,3})%)?')


def parse_color(v, key='color'):
    """#RRGGBB or a theme colour, lighter (+NN%) or darker (-NN%), `*` for another transform the file keeps, and
    /NN% for opacity: accent1+40%, accent1*, #FF0000/55%."""
    s = str(v).strip()
    m = COLOR_RE.fullmatch(s)
    if m:
        base = m.group(1)
        if base.startswith('#'):
            if m.group(2) or m.group(4):
                raise VocabError('%s: lighter/darker (+NN%%/-NN%%) and `*` apply to theme colours, not to %s'
                                 % (key, base))
            out = '#' + base[1:].upper()
        else:
            base = THEME_ALIAS.get(base, base)
            if base not in THEME:
                m = None
            else:
                out = base
                if m.group(2):
                    if not 0 < int(m.group(3)) < 100:
                        raise VocabError('%s: a theme colour is lighter or darker by 1%% to 99%%, got %r' % (key, v))
                    out += '%s%d%%' % (m.group(2), int(m.group(3)))
                elif m.group(4):
                    out += '*'
        if m:
            if m.group(5):
                if not 0 <= int(m.group(5)) <= 100:
                    raise VocabError('%s: opacity is 0%% to 100%%, got %r' % (key, v))
                if int(m.group(5)) != 100:
                    out += '/%d%%' % int(m.group(5))
            return out
    raise VocabError('%s: expected a colour #RRGGBB or a theme colour (%s; accent1+40%% is lighter, accent1-25%% '
                     'darker), got %r' % (key, ', '.join(THEME), v))


def hex_rgb(h):
    return tuple(int(h[i:i + 2], 16) / 255 for i in (1, 3, 5))


def rgb_hex(r, g, b):
    return '#%02X%02X%02X' % tuple(max(0, min(255, round(c * 255))) for c in (r, g, b))


def modify(hexcol, pct):
    """Office's lighter/darker: in HSL, lighter p% gives L' = L(1-p) + p, darker p% gives L' = L(1-p)."""
    r, g, b = hex_rgb(hexcol)
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    p = abs(pct) / 100
    l = l * (1 - p) + p if pct > 0 else l * (1 - p)
    return rgb_hex(*colorsys.hls_to_rgb(h, l, s))


def resolve_color(c, palette):
    """-> #RRGGBB for a canonical colour, with the file's theme palette {name: #RRGGBB} (opacity dropped; a `*`
    transform is read as the plain theme colour)."""
    if c is None:
        return c
    c = c.split('/')[0].rstrip('*')
    if c.startswith('#'):
        return c
    m = re.fullmatch(r'([a-z]+\d?)(?:([+-])(\d+)%)?', c)
    base = palette.get(m.group(1), '#000000')
    if m.group(2):
        return modify(base, int(m.group(3)) * (1 if m.group(2) == '+' else -1))
    return base


def theme_mod(name, lighter=0.0, darker=0.0):
    """A theme colour with a lighter/darker fraction (0..1) -> canonical text."""
    if lighter > 0.005:
        return '%s+%d%%' % (name, round(lighter * 100))
    if darker > 0.005:
        return '%s-%d%%' % (name, round(darker * 100))
    return name


def hls(c, palette=None):
    h = resolve_color(c, palette or {})
    r, g, b = hex_rgb(h)
    hh, l, s = colorsys.rgb_to_hls(r, g, b)
    return hh * 360, l, s


# ---------------------------------------------------------------- borders

def parse_border(v, key='border'):
    s = str(v).strip()
    if s == 'none':
        return 'none'
    parts = s.split()
    if len(parts) != 3:
        raise VocabError('%s: expected "none" or "<width>pt <style> <colour>", e.g. "0.5pt solid #000000", got %r'
                         % (key, v))
    w = parse_len(parts[0], key)
    if w <= 0:
        raise VocabError('%s: a border width is above 0pt; write none for no border' % key)
    if parts[1] not in BORDER_STYLES + SHOWN_ONLY_STYLES:
        raise VocabError('%s: border style is one of %s, got %r' % (key, ', '.join(BORDER_STYLES), parts[1]))
    return '%s %s %s' % (pt(w), parts[1], parse_color(parts[2], key))


def border_width(b):
    return 0.0 if not b or b == 'none' else float(b.split()[0][:-2])


def border_parts(b):
    if not b or b == 'none':
        return None
    w, st, c = b.split()
    return float(w[:-2]), st, c


# ---------------------------------------------------------------- values

def parse_value(key, v):
    """-> canonical value string, or raises VocabError."""
    if key == 'style':
        return str(v)
    if key in FLAGS:
        if v in (True, None, '', 'yes', 'on', 'true'):
            return 'yes'
        if v in ('no', 'off', 'false'):
            return 'no'
        raise VocabError('%s: write %s alone to turn it on, or %s=no to turn it off' % (key, key, key))
    if key in ('color',):
        return parse_color(v, key)
    if key == 'fill':
        s = str(v).strip()
        if s in FILL_TOKENS:
            return s
        if s == 'none':
            return 'none'
        return parse_color(s, key)
    if key == 'border' or key in SIDES:
        return parse_border(v, key)
    if key == 'size':
        x = parse_len(v, key)
        if x < 0:
            raise VocabError('size: 0pt or more')
        return pt(x)
    if key in LENGTH_KEYS:
        return pt(parse_len(v, key))
    if key == 'line-spacing':
        s = str(v).strip()
        m = re.fullmatch(r'(\d+(?:\.\d+)?)%', s)
        if m:
            return num(m.group(1)) + '%'
        m = re.fullmatch(r'at-least\s+(\d+(?:\.\d+)?)\s*(pt)?', s)
        if m:
            return 'at-least ' + pt(m.group(1))
        m = re.fullmatch(r'gap\s+(\d+(?:\.\d+)?)\s*(pt)?', s)
        if m:
            return 'gap ' + pt(m.group(1))
        return pt(parse_len(s, key))
    if key == 'align':
        if v not in ALIGN:
            raise VocabError('align: one of %s, got %r' % (', '.join(ALIGN), v))
        return v
    if key == 'valign':
        if v not in VALIGN:
            raise VocabError('valign: one of %s, got %r' % (', '.join(VALIGN), v))
        return v
    if key == 'font':
        return str(v)
    if key == 'indent':
        if not re.fullmatch(r'\d{1,3}', str(v)):
            raise VocabError('indent: a whole number of indent levels (xlsx), got %r' % v)
        return str(int(v))
    raise VocabError('unknown property %r; the properties are %s' % (key, ', '.join(ORDER)))


def length(props, key):
    v = props.get(key)
    return None if v is None else float(v[:-2])


# ---------------------------------------------------------------- attribute lists

ATTR_RE = re.compile(r'\s*([a-z][a-z0-9-]*)(?:=("([^"]*)"|[^\s"}]+))?')


def parse_attrs(s, where=''):
    """'fill=#FFF0C3 border-bottom="2.83pt solid #7F7F7F" bold' -> {key: canonical}. Raises VocabError."""
    out = {}
    pos = 0
    s = s.strip()
    while pos < len(s):
        m = ATTR_RE.match(s, pos)
        if not m or m.end() == pos:
            raise VocabError('%scannot read the attributes at %r; write key=value pairs separated by spaces, with '
                             'quotes around a value that holds spaces' % (where, s[pos:pos + 30]))
        key = m.group(1)
        raw = m.group(3) if m.group(3) is not None else m.group(2)
        if key not in KEYS:
            raise VocabError('%sunknown property %r; the properties are %s' % (where, key, ', '.join(ORDER)))
        if key in out:
            raise VocabError('%sproperty %r written twice' % (where, key))
        if raw is None and key not in FLAGS:
            raise VocabError('%s%s needs a value (%s=...)' % (where, key, key))
        try:
            out[key] = parse_value(key, raw)
        except VocabError as e:
            raise VocabError(where + str(e))
        pos = m.end()
    return expand(out)


def expand(p):
    """`border` is shorthand for the four sides."""
    if 'border' in p:
        b = p.pop('border')
        for s in SIDES:
            p.setdefault(s, b)
    return p


def quote(v):
    return '"%s"' % v if (' ' in v or '}' in v or '{' in v) else v


def fmt_attrs(p):
    """Canonical text of a property set (no braces). Four equal sides are written as `border`."""
    q = dict(p)
    sides = [q.get(s) for s in SIDES]
    if all(x is not None for x in sides) and len(set(sides)) == 1:
        for s in SIDES:
            q.pop(s)
        q['border'] = sides[0]
    out = []
    for k in ('style',) + ORDER:
        if k not in q:
            continue
        v = q[k]
        if k in FLAGS:
            out.append(k if v == 'yes' else k + '=no')
        elif k == 'style':
            out.append('style="%s"' % v)
        else:
            out.append('%s=%s' % (k, quote(v)))
    return ' '.join(out)


def diff(eff, base):
    """Properties in eff that differ from base (a key absent from eff but set in base is not a difference
    here: callers decide how absence reads)."""
    return {k: v for k, v in eff.items() if base.get(k) != v}


def close(a, b, tol=0.6):
    """Two canonical values equal, lengths within tol pt."""
    if a == b:
        return True
    if a is None or b is None:
        return False
    try:
        fa, fb = float(a[:-2]), float(b[:-2])
        return a.endswith('pt') and b.endswith('pt') and abs(fa - fb) <= tol
    except ValueError:
        return False
