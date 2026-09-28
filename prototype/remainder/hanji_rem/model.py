"""Model text (the §5.2 Document subset this prototype needs) and its parser.

A paragraph's content is held as an *anchor string*: its text, with every
inline placeholder as one private-use character, plus per-character bold and
italic flags. Remainder anchors are offsets into that string.
"""
import copy
import re
from dataclasses import dataclass, field

KEEP_BASE = 0xF0000  # Supplementary Private Use Area-A: one char per keep


def keep_char(n):
    return chr(KEEP_BASE + n)


def keep_num(ch):
    o = ord(ch)
    return o - KEEP_BASE if o >= KEEP_BASE else None


@dataclass
class Para:
    style: str
    chars: str = ""
    bold: list = field(default_factory=list)
    italic: list = field(default_factory=list)

    def copy(self):
        return copy.deepcopy(self)


@dataclass
class Cell:
    marker: str = None  # None, '^^' (merged into cell above), '||' (cell to the left extends)
    para: Para = None


@dataclass
class Table:
    rows: list  # list[list[Cell]]


@dataclass
class KeepBlock:
    n: int


@dataclass
class KeepInfo:
    n: int
    kind: str
    summary: str
    block: bool

    @property
    def id(self):
        return "k%d" % self.n


class ModelError(Exception):
    pass


# ---------------------------------------------------------------- serialise

ESC = set("\\*_<[]|~`^")


def _esc(ch):
    return "\\" + ch if ch in ESC else ch


def _attr(s):
    return s.replace("&", "&amp;").replace('"', "&quot;").replace("<", "&lt;")


def keep_tag(k):
    return '<keep id="%s" kind="%s" summary="%s"/>' % (k.id, _attr(k.kind), _attr(k.summary))


def inline(p, keeps):
    out = []
    b = i = False
    for idx, ch in enumerate(p.chars):
        nb, ni = p.bold[idx], p.italic[idx]
        if ni != i and i:
            out.append("*"); i = False
        if nb != b:
            out.append("**"); b = nb
        if ni != i:
            out.append("*"); i = ni
        n = keep_num(ch)
        if n is not None:
            out.append(keep_tag(keeps[n]))
        elif ch == "\n":
            out.append("<br/>")
        else:
            out.append(_esc(ch))
    if i:
        out.append("*")
    if b:
        out.append("**")
    return "".join(out)


HEADING_RE = re.compile(r"^heading ([1-6])$", re.I)


class Styles:
    def __init__(self, by_id, default_id):
        self.by_id = by_id  # id -> name (paragraph styles)
        self.by_name = {}
        for k, v in by_id.items():
            self.by_name.setdefault(v, k)
        self.default_id = default_id
        self.default = by_id.get(default_id, "Normal")
        self.heading = {}
        for sid, name in by_id.items():
            m = HEADING_RE.match(name)
            if m:
                self.heading.setdefault(int(m.group(1)), name)

    def level(self, name):
        for lv, n in self.heading.items():
            if n == name:
                return lv
        return None


def para_line(p, keeps, styles):
    body = inline(p, keeps)
    lv = styles.level(p.style)
    if p.chars and lv:
        return "#" * lv + " " + body
    if p.chars and p.style == styles.default:
        # a plain line: escape a leading block marker
        if body[0] in "#>-+=":
            body = "\\" + body
        body = re.sub(r"^(\d+)([.)])", r"\1\\\2", body)
        if body.startswith(" ") or body.startswith("\t"):
            return '<div style="%s">%s</div>' % (_attr(p.style), body)
        # a plain paragraph made only of a block-keep-looking tag is still inline:
        # the parser tells them apart by the keep registry.
        return body
    return '<div style="%s">%s</div>' % (_attr(p.style), body)


def cell_text(c, keeps):
    if c.marker == "||":
        return None
    if c.marker == "^^":
        return "^^"
    return inline(c.para, keeps)


def table_lines(t, keeps):
    lines = []
    for ri, row in enumerate(t.rows):
        s = "|"
        for c in row:
            txt = cell_text(c, keeps)
            s += "|" if txt is None else " %s |" % txt
        lines.append(s)
        if ri == 0:
            lines.append("|" + "---|" * len(row))
    return lines


def serialize(blocks, keeps, styles, template="source.docx"):
    out = ["---", "type: document", "format: docx", "template: %s" % template, "schema: 1", "---"]
    for b in blocks:
        out.append("")
        if isinstance(b, Para):
            out.append(para_line(b, keeps, styles))
        elif isinstance(b, Table):
            out.extend(table_lines(b, keeps))
        elif isinstance(b, KeepBlock):
            out.append(keep_tag(keeps[b.n]))
    return "\n".join(out) + "\n"


# ------------------------------------------------------------------- parse

KEEP_RE = re.compile(r'<keep id="k(\d+)" kind="[^"]*" summary="[^"]*"/>')
DIV_RE = re.compile(r'^<div style="([^"]*)">(.*)</div>$')


def _unattr(s):
    return s.replace("&lt;", "<").replace("&quot;", '"').replace("&amp;", "&")


def parse_inline(s, keeps, style, where):
    chars, bold, italic = [], [], []
    b = i = False
    k = 0
    while k < len(s):
        ch = s[k]
        if ch == "\\" and k + 1 < len(s):
            chars.append(s[k + 1]); bold.append(b); italic.append(i); k += 2
            continue
        if ch == "*":
            j = k
            while j < len(s) and s[j] == "*":
                j += 1
            n = j - k
            if n % 2 == 1:
                i = not i
            if (n // 2) % 2 == 1:
                b = not b
            k = j
            continue
        if ch == "<":
            m = KEEP_RE.match(s, k)
            if m:
                n = int(m.group(1))
                if n not in keeps:
                    raise ModelError("%s: unknown placeholder k%d (placeholders are never created)" % (where, n))
                chars.append(keep_char(n)); bold.append(b); italic.append(i)
                k = m.end()
                continue
            if s.startswith("<br/>", k):
                chars.append("\n"); bold.append(b); italic.append(i); k += 5
                continue
            raise ModelError("%s col %d: unexpected '<' (write \\< for a literal)" % (where, k + 1))
        chars.append(ch); bold.append(b); italic.append(i); k += 1
    if b or i:
        raise ModelError("%s: unclosed ** or *" % where)
    return Para(style, "".join(chars), bold, italic)


def split_row(line):
    cells, cur, k = [], [], 1  # skip leading '|'
    if not line.startswith("|"):
        raise ModelError("table row must start with |")
    while k < len(line):
        ch = line[k]
        if ch == "\\" and k + 1 < len(line):
            cur.append(line[k:k + 2]); k += 2
            continue
        if ch == "|":
            cells.append("".join(cur)); cur = []
            k += 1
            continue
        cur.append(ch); k += 1
    if "".join(cur).strip():
        raise ModelError("table row must end with |")
    return cells


def parse_table(lines, keeps, styles, lineno):
    rows = []
    for li, line in enumerate(lines):
        if li == 1:
            if not re.match(r"^\|(-+\|)+$", line):
                raise ModelError("line %d: expected the |---| separator row" % (lineno + 1))
            continue
        row = []
        for ci, raw in enumerate(split_row(line)):
            if raw == "":
                if ci == 0:
                    raise ModelError("line %d: || at the start of a row" % (lineno + li))
                row.append(Cell("||"))
                continue
            txt = raw[1:] if raw.startswith(" ") else raw
            txt = txt[:-1] if txt.endswith(" ") else txt
            if txt == "^^":
                if not rows:
                    raise ModelError("line %d: ^^ in the header row" % (lineno + li))
                row.append(Cell("^^"))
                continue
            row.append(Cell(None, parse_inline(txt, keeps, styles.default, "line %d" % (lineno + li))))
        rows.append(row)
    return Table(rows)


def parse(text, keeps, styles):
    """Model text -> blocks. `keeps` maps keep number -> KeepInfo (from the remainder)."""
    lines = text.split("\n")
    k = 0
    if lines and lines[0] == "---":
        k = lines.index("---", 1) + 1
    blocks = []
    while k < len(lines):
        line = lines[k]
        if line == "":
            k += 1
            continue
        where = "line %d" % (k + 1)
        if line.startswith("|"):
            j = k
            while j < len(lines) and lines[j].startswith("|"):
                j += 1
            blocks.append(parse_table(lines[k:j], keeps, styles, k))
            k = j
            continue
        m = KEEP_RE.fullmatch(line)
        if m and int(m.group(1)) in keeps and keeps[int(m.group(1))].block:
            blocks.append(KeepBlock(int(m.group(1))))
            k += 1
            continue
        m = DIV_RE.match(line)
        if m:
            blocks.append(parse_inline(m.group(2), keeps, _unattr(m.group(1)), where))
        else:
            hm = re.match(r"^(#{1,6}) (.*)$", line)
            if hm:
                lv = len(hm.group(1))
                if lv not in styles.heading:
                    raise ModelError("%s: this file has no Heading %d style" % (where, lv))
                blocks.append(parse_inline(hm.group(2), keeps, styles.heading[lv], where))
            else:
                blocks.append(parse_inline(line, keeps, styles.default, where))
        k += 1
    return blocks


def normalise_ids(text):
    """For PutGet: keep numbers are assigned in document order on import."""
    return re.sub(r'<keep id="k\d+"', '<keep id="k?"', text)
