"""document.xml -> model blocks + remainder entries.

Remainder entry kinds (payload = the XML the model does not represent):
  ppr      paragraph: w:p attributes + pPr (pStyle is modelled, the rest is not)   anchor: block
  run      one w:r: attributes + rPr (bold/italic are modelled)                   anchor: [start,end)
  marker   zero-width inline element (bookmark, comment range, proofErr, perm)  anchor: pos
  rmarker  zero-width element inside a run (lastRenderedPageBreak)              anchor: pos (+ its run)
  wrap     inline wrapper (hyperlink, smartTag, customXml, dir, bdo)            anchor: [start,end)
  keep     inline object shown as <keep/> (drawing, field, footnote ref, ins…)  anchor: its placeholder
  bkeep    block shown as <keep/> (block sdt, unmodellable table, …)            anchor: its placeholder
  bmarker  zero-width element between blocks (body-level bookmark …)           anchor: before block
  tbl/tr/tc table, row and cell properties                                      anchor: table/row/cell
  tail     body sectPr and anything after the last block                        anchor: document
"""
import copy
import itertools
from dataclasses import dataclass, field

from . import model as M
from .ooxml import qn, local, is_w, wval, parse, part, fp, text_of, DOC_PART

WRAPPERS = {"hyperlink", "smartTag", "customXml", "dir", "bdo"}
MARKERS = {"bookmarkStart", "bookmarkEnd", "commentRangeStart", "commentRangeEnd", "proofErr",
           "permStart", "permEnd", "moveFromRangeStart", "moveFromRangeEnd", "moveToRangeStart",
           "moveToRangeEnd", "customXmlInsRangeStart", "customXmlInsRangeEnd",
           "customXmlDelRangeStart", "customXmlDelRangeEnd", "customXmlMoveFromRangeStart",
           "customXmlMoveFromRangeEnd", "customXmlMoveToRangeStart", "customXmlMoveToRangeEnd"}
# markers that mean something to a person and must never vanish with nearby text
DURABLE_MARKERS = {"bookmarkStart", "bookmarkEnd", "commentRangeStart", "commentRangeEnd",
                   "permStart", "permEnd", "moveFromRangeStart", "moveFromRangeEnd",
                   "moveToRangeStart", "moveToRangeEnd"}
RUN_MARKERS = {"lastRenderedPageBreak"}
KEEP_KINDS = {"drawing": "drawing", "pict": "drawing", "object": "object", "AlternateContent": "drawing",
              "footnoteReference": "footnote", "endnoteReference": "endnote",
              "commentReference": "comment", "fldSimple": "field", "fldChar": "field-part",
              "instrText": "field-part", "ins": "tracked-insert", "del": "tracked-delete",
              "moveFrom": "tracked-move", "moveTo": "tracked-move", "sdt": "content-control",
              "oMath": "math", "oMathPara": "math", "sym": "symbol", "ptab": "tab",
              "noBreakHyphen": "hyphen", "softHyphen": "hyphen", "cr": "break", "br": "break"}

ON = lambda el: el is not None and wval(el) not in ("0", "false", "off")


@dataclass
class Entry:
    id: int
    kind: str
    payload: list          # list of lxml elements (deep copies)
    fp: str
    path: tuple            # (bi,) | (bi, r) | (bi, r, c)
    start: int = None
    end: int = None
    seq: float = 0
    extra: dict = field(default_factory=dict)

    @property
    def empty(self):
        return self.fp in ("", "<w:r/>")

    def clone(self, **kw):
        e = copy.copy(self)
        e.extra = dict(self.extra)
        for k, v in kw.items():
            setattr(e, k, v)
        return e


@dataclass
class Doc:
    parts: list
    root: object          # document root element with body emptied (namespaces, attributes)
    body_attrs: dict
    styles: M.Styles
    blocks: list
    keeps: dict           # n -> KeepInfo
    entries: list
    stats: dict


class Importer:
    def __init__(self, parts):
        self.parts = parts
        self.ids = itertools.count(1)
        self.seq = itertools.count(1)
        self.keepnum = itertools.count(1)
        self.entries = []
        self.keeps = {}
        self.stats = {"tables_modelled": 0, "tables_kept": 0, "kept_reasons": {}}
        self.styles = self._styles()
        self.comments = self._notes("word/comments.xml", "comment")
        self.footnotes = self._notes("word/footnotes.xml", "footnote")
        self.endnotes = self._notes("word/endnotes.xml", "endnote")

    # -------------------------------------------------------------- context
    def _styles(self):
        by_id, default = {}, None
        data = part(self.parts, "word/styles.xml")
        if data:
            root = parse(data)
            for s in root.findall(qn("w:style")):
                if s.get(qn("w:type")) != "paragraph":
                    continue
                sid = s.get(qn("w:styleId"))
                n = s.find(qn("w:name"))
                by_id[sid] = wval(n) if n is not None else sid
                if s.get(qn("w:default")) in ("1", "true", "on"):
                    default = sid
        if default is None:
            default = "Normal"
            by_id.setdefault("Normal", "Normal")
        return M.Styles(by_id, default)

    def _notes(self, name, tag):
        data = part(self.parts, name)
        out = {}
        if data:
            for n in parse(data).findall(qn("w:" + tag)):
                out[n.get(qn("w:id"))] = text_of(n, 40)
        return out

    def entry(self, kind, payload, fpv, path, start=None, end=None, **extra):
        e = Entry(next(self.ids), kind, payload, fpv, path, start, end, next(self.seq), extra)
        self.entries.append(e)
        return e

    def new_keep(self, kind, summary, block):
        n = next(self.keepnum)
        self.keeps[n] = M.KeepInfo(n, kind, summary, block)
        return n

    # -------------------------------------------------------------- summaries
    def summary(self, els):
        el = els[0]
        name = local(el)
        if name in ("drawing", "pict", "AlternateContent", "object"):
            dp = el.find(".//" + qn("wp:docPr"))
            if dp is not None:
                return (dp.get("descr") or dp.get("name") or "drawing")[:60]
            return "vml shape" if name == "pict" else name
        if name == "footnoteReference":
            return "footnote: " + self.footnotes.get(el.get(qn("w:id")), "")
        if name == "endnoteReference":
            return "endnote: " + self.endnotes.get(el.get(qn("w:id")), "")
        if name == "commentReference":
            return "comment: " + self.comments.get(el.get(qn("w:id")), "")
        if name in ("ins", "del", "moveFrom", "moveTo"):
            return "%s by %s: %s" % (name, el.get(qn("w:author")) or "?", text_of(el, 40))
        if name == "sdt":
            alias = el.find(".//" + qn("w:alias"))
            return ((wval(alias) + ": ") if alias is not None else "") + text_of(el, 40)
        if name == "fldSimple":
            return "field %s → %s" % (" ".join((el.get(qn("w:instr")) or "").split())[:30], text_of(el, 25))
        if name == "r":  # a grouped complex field
            instr = " ".join("".join(t.text or "" for e in els for t in e.iter(qn("w:instrText"))).split())
            res = []
            depth = 0
            for e in els:
                for x in e.iter():
                    if is_w(x, "fldChar"):
                        t = x.get(qn("w:fldCharType"))
                        depth = 1 if t == "separate" else (0 if t in ("begin", "end") else depth)
                    elif is_w(x, "t") and depth and x.text:
                        res.append(x.text)
            return ("field %s → %s" % (instr[:30], "".join(res)[:25])).strip()
        if name == "tbl":
            return "table: " + text_of(el, 40)
        return name + (": " + text_of(el, 40) if text_of(el) else "")

    # -------------------------------------------------------------- document
    def run(self):
        root = parse(part(self.parts, DOC_PART))
        body = root.find(qn("w:body"))
        blocks = []
        pending = []  # body-level markers waiting for the next block
        tail = []
        children = list(body)
        for idx, el in enumerate(children):
            name = local(el)
            bi = len(blocks)
            if name == "sectPr" and idx == len(children) - 1:
                tail.append(el)
                continue
            if name in MARKERS:
                pending.append(el)
                continue
            for m in pending:
                self.entry("bmarker", [copy.deepcopy(m)], fp(m), (bi,))
            pending = []
            if name == "p":
                blocks.append(self.para(el, (bi,)))
            elif name == "tbl":
                blocks.append(self.table(el, bi))
            else:
                blocks.append(self.block_keep([el], (bi,), name))
        for m in pending + tail:
            self.entry("tail", [copy.deepcopy(m)], fp(m), ())
        for el in list(body):
            body.remove(el)
        body_attrs = dict(body.attrib)
        self.stats.update(blocks=len(blocks), entries=len(self.entries), keeps=len(self.keeps))
        return Doc(self.parts, root, body_attrs, self.styles, blocks, self.keeps, self.entries, self.stats)

    def block_keep(self, els, path, kind):
        n = self.new_keep({"sdt": "content-control", "tbl": "table"}.get(kind, kind), self.summary(els), True)
        self.entry("bkeep", [copy.deepcopy(e) for e in els], fp(*els), path, n=n)
        return M.KeepBlock(n)

    # -------------------------------------------------------------- tables
    def table_reason(self, tbl):
        ncols = None
        for ch in tbl:
            if local(ch) not in ("tblPr", "tblGrid", "tr"):
                return "table-level %s" % local(ch)
        for ri, tr in enumerate(tbl.findall(qn("w:tr"))):
            cols = 0
            for ch in tr:
                if local(ch) not in ("trPr", "tblPrEx", "tc"):
                    return "row-level %s" % local(ch)
            trpr = tr.find(qn("w:trPr"))
            if trpr is not None and (trpr.find(qn("w:gridBefore")) is not None or trpr.find(qn("w:gridAfter")) is not None):
                return "gridBefore/gridAfter"
            for tc in tr.findall(qn("w:tc")):
                ps = [c for c in tc if local(c) not in ("tcPr",)]
                if len(ps) != 1 or local(ps[0]) != "p":
                    return "cell with %d blocks" % len(ps) if all(local(p) == "p" for p in ps) else "cell holds %s" % ",".join(sorted({local(p) for p in ps if local(p) != 'p'}))
                tcpr = tc.find(qn("w:tcPr"))
                span = int(wval(tcpr.find(qn("w:gridSpan")))) if tcpr is not None and tcpr.find(qn("w:gridSpan")) is not None else 1
                vm = tcpr.find(qn("w:vMerge")) if tcpr is not None else None
                if vm is not None and wval(vm) != "restart":
                    if ri == 0:
                        return "vMerge continue in first row"
                    if text_of(ps[0]) or ps[0].find(".//" + qn("w:drawing")) is not None:
                        return "text in a covered cell"
                if tcpr is not None and tcpr.find(qn("w:hMerge")) is not None:
                    return "hMerge"
                cols += span
            if ncols is None:
                ncols = cols
            elif cols != ncols:
                return "ragged rows"
        if ncols is None or ncols == 0:
            return "empty table"
        return None

    def table(self, tbl, bi):
        reason = self.table_reason(tbl)
        if reason:
            self.stats["tables_kept"] += 1
            self.stats["kept_reasons"][reason] = self.stats["kept_reasons"].get(reason, 0) + 1
            return self.block_keep([tbl], (bi,), "tbl")
        self.stats["tables_modelled"] += 1
        head = [copy.deepcopy(c) for c in tbl if local(c) in ("tblPr", "tblGrid")]
        shell = copy.deepcopy(tbl)
        for c in list(shell):
            shell.remove(c)
        self.entry("tbl", [shell] + head, fp(shell, *head), (bi,))
        rows = []
        for ri, tr in enumerate(tbl.findall(qn("w:tr"))):
            trs = copy.deepcopy(tr)
            for c in list(trs):
                trs.remove(c)
            rhead = [copy.deepcopy(c) for c in tr if local(c) in ("tblPrEx", "trPr")]
            self.entry("tr", [trs] + rhead, fp(trs, *rhead), (bi, ri))
            row = []
            for tc in tr.findall(qn("w:tc")):
                gc = len(row)
                tcpr = tc.find(qn("w:tcPr"))
                tcs = copy.deepcopy(tc)
                for c in list(tcs):
                    tcs.remove(c)
                self.entry("tc", [tcs] + ([copy.deepcopy(tcpr)] if tcpr is not None else []), fp(tcs, tcpr), (bi, ri, gc))
                p = tc.find(qn("w:p"))
                para = self.para(p, (bi, ri, gc))
                vm = tcpr.find(qn("w:vMerge")) if tcpr is not None else None
                span = int(wval(tcpr.find(qn("w:gridSpan")))) if tcpr is not None and tcpr.find(qn("w:gridSpan")) is not None else 1
                row.append(M.Cell("^^", para) if vm is not None and wval(vm) != "restart" else M.Cell(None, para))
                for _ in range(span - 1):
                    row.append(M.Cell("||"))
            rows.append(row)
        return M.Table(rows)

    # -------------------------------------------------------------- paragraphs
    def para(self, p, path):
        ppr = p.find(qn("w:pPr"))
        pstyle = ppr.find(qn("w:pStyle")) if ppr is not None else None
        sid = wval(pstyle) if pstyle is not None else self.styles.default_id
        style = self.styles.by_id.get(sid, sid)
        shell = copy.deepcopy(p)
        for c in list(shell):
            shell.remove(c)
        rest = copy.deepcopy(ppr) if ppr is not None else None
        if rest is not None and rest.find(qn("w:pStyle")) is not None:
            rest.remove(rest.find(qn("w:pStyle")))
        if rest is not None and len(rest) == 0 and not rest.attrib:
            rest = None  # an empty pPr holds nothing unmodelled
        self.entry("ppr", [shell] + ([copy.deepcopy(ppr)] if ppr is not None else []),
                   "" if not shell.attrib and rest is None else fp(shell, rest),
                   path, style_id=sid)
        self.buf = {"chars": [], "bold": [], "italic": []}
        self.walk(p, path)
        b = self.buf
        return M.Para(style, "".join(b["chars"]), b["bold"], b["italic"])

    @property
    def pos(self):
        return len(self.buf["chars"])

    def push(self, ch, bold=False, italic=False):
        self.buf["chars"].append(ch)
        self.buf["bold"].append(bold)
        self.buf["italic"].append(italic)

    def field_groups(self, kids):
        """Complex fields whose begin..end lie among these siblings -> {start_idx: end_idx}."""
        groups = {}
        i = 0
        while i < len(kids):
            k = kids[i]
            if is_w(k, "r") and any(wval(f, "fldCharType") == "begin" for f in k.findall(qn("w:fldChar"))):
                depth = 0
                for j in range(i, len(kids)):
                    for f in kids[j].iter(qn("w:fldChar")):
                        t = f.get(qn("w:fldCharType"))
                        depth += 1 if t == "begin" else (-1 if t == "end" else 0)
                    if depth <= 0:
                        break
                if depth == 0 and all(is_w(x, "r") or local(x) in MARKERS for x in kids[i:j + 1]):
                    groups[i] = j
                    i = j + 1
                    continue
            i += 1
        return groups

    def walk(self, container, path):
        kids = [k for k in container if isinstance(k.tag, str) and not is_w(k, "pPr")]
        groups = self.field_groups(kids)
        i = 0
        while i < len(kids):
            k = kids[i]
            name = local(k)
            if i in groups:
                els = kids[i:groups[i] + 1]
                n = self.new_keep("field", self.summary(els), False)
                self.entry("keep", [copy.deepcopy(e) for e in els], fp(*els), path, self.pos, self.pos + 1, n=n, in_run=False)
                self.push(M.keep_char(n))
                i = groups[i] + 1
                continue
            if name == "r":
                self.wrun(k, path)
            elif name in WRAPPERS:
                shell = copy.deepcopy(k)
                inner = [c for c in shell if local(c) in ("smartTagPr", "customXmlPr")]
                for c in list(shell):
                    if c not in inner:
                        shell.remove(c)
                e = self.entry("wrap", [shell], fp(shell), path, self.pos, None)
                self.walk_wrapper(k, path)
                e.end = self.pos
                e.extra["seq_close"] = next(self.seq)
            elif name in MARKERS:
                self.entry("marker", [copy.deepcopy(k)], fp(k), path, self.pos, self.pos, tag=name)
            else:
                n = self.new_keep(KEEP_KINDS.get(name, name), self.summary([k]), False)
                self.entry("keep", [copy.deepcopy(k)], fp(k), path, self.pos, self.pos + 1, n=n, in_run=False)
                self.push(M.keep_char(n))
            i += 1

    def walk_wrapper(self, w, path):
        self.walk([x for x in w if local(x) not in ("smartTagPr", "customXmlPr")], path)

    def wrun(self, r, path):
        rpr = r.find(qn("w:rPr"))
        bold = ON(rpr.find(qn("w:b"))) if rpr is not None else False
        italic = ON(rpr.find(qn("w:i"))) if rpr is not None else False
        shell = copy.deepcopy(r)
        for c in list(shell):
            shell.remove(c)
        rest = copy.deepcopy(rpr) if rpr is not None else None
        if rest is not None:
            for t in ("w:b", "w:i"):
                if rest.find(qn(t)) is not None:
                    rest.remove(rest.find(qn(t)))
        start = self.pos
        e = self.entry("run", [shell] + ([copy.deepcopy(rpr)] if rpr is not None else []),
                       fp(shell, rest), path, start, None, t_attrs=[], bold0=bold, italic0=italic)
        if not shell.attrib and (rest is None or (len(rest) == 0 and not rest.attrib)):
            e.fp = ""  # nothing unmodelled in this run
        for c in r:
            if not isinstance(c.tag, str) or is_w(c, "rPr"):
                continue
            name = local(c)
            if name == "t":
                e.extra["t_attrs"].append((dict(c.attrib), c.text or ""))
                for ch in c.text or "":
                    self.push(ch, bold, italic)
            elif name == "tab" and not c.attrib:
                self.push("\t", bold, italic)
            elif name == "br" and not c.attrib:
                self.push("\n", bold, italic)
            elif name in RUN_MARKERS:
                self.entry("rmarker", [copy.deepcopy(c)], fp(c), path, self.pos, self.pos, run=e.id)
            else:
                n = self.new_keep(KEEP_KINDS.get(name, name), self.summary([c]), False)
                self.entry("keep", [copy.deepcopy(c)], fp(c), path, self.pos, self.pos + 1, n=n, in_run=True, run=e.id)
                self.push(M.keep_char(n), bold, italic)
        e.end = self.pos


def import_docx(parts):
    return Importer(parts).run()
