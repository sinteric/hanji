"""model blocks + placed remainder entries -> document.xml bytes."""
import copy
from collections import defaultdict

from lxml import etree

from . import model as M
from .ooxml import qn, local, wval, XML_NS

RPR_ORDER = ["rStyle", "rFonts", "b", "bCs", "i", "iCs", "caps", "smallCaps", "strike", "dstrike",
             "outline", "shadow", "emboss", "imprint", "noProof", "snapToGrid", "vanish", "webHidden",
             "color", "spacing", "w", "kern", "position", "sz", "szCs", "highlight", "u", "effect",
             "bdr", "shd", "fitText", "vertAlign", "rtl", "cs", "em", "lang", "eastAsianLayout",
             "specVanish", "oMath"]
TCPR_ORDER = ["cnfStyle", "tcW", "gridSpan", "hMerge", "vMerge", "tcBorders", "shd", "noWrap",
              "tcMar", "textDirection", "tcFitText", "vAlign", "hideMark"]


def W_(name, **attrs):
    el = etree.Element(qn("w:" + name))
    for k, v in attrs.items():
        el.set(qn("w:" + k), v)
    return el


def insert_ordered(parent, el, order):
    rank = order.index(local(el)) if local(el) in order else len(order)
    for idx, c in enumerate(parent):
        if local(c) in order and order.index(local(c)) > rank:
            parent.insert(idx, el)
            return
    parent.append(el)


def set_flag(rpr, name, want):
    cur = rpr.find(qn("w:" + name))
    on = cur is not None and wval(cur) not in ("0", "false", "off")
    if on == want:
        return
    if cur is not None:
        rpr.remove(cur)
    if want:
        insert_ordered(rpr, W_(name), RPR_ORDER)


class ExportError(Exception):
    pass


class Exporter:
    def __init__(self, doc, blocks, entries):
        self.doc = doc
        self.styles = doc.styles
        self.blocks = blocks
        self.by = defaultdict(lambda: defaultdict(list))
        self.keep = {}
        self.tail = []
        self.issues = []
        for e in entries:
            if e.kind == "tail":
                self.tail.append(e)
            elif e.kind in ("keep", "bkeep"):
                self.keep[e.extra["n"]] = e
            else:
                self.by[e.path][e.kind].append(e)

    def run(self):
        root = copy.deepcopy(self.doc.root)
        body = root.find(qn("w:body"))
        for bi, b in enumerate(self.blocks):
            for m in sorted(self.by[(bi,)]["bmarker"], key=lambda e: e.seq):
                body.append(copy.deepcopy(m.payload[0]))
            if isinstance(b, M.Para):
                body.append(self.para(b, (bi,)))
            elif isinstance(b, M.Table):
                body.append(self.table(b, bi))
            else:
                k = self.keep.get(b.n)
                if k is None:
                    raise ExportError("block placeholder k%d has no remainder entry" % b.n)
                for el in k.payload:
                    body.append(copy.deepcopy(el))
        for e in sorted(self.tail, key=lambda e: e.seq):
            body.append(copy.deepcopy(e.payload[0]))
        return etree.tostring(root, xml_declaration=True, encoding="UTF-8", standalone=True)

    # ------------------------------------------------------------ paragraph
    def ppr(self, p, path):
        pp = self.by[path]["ppr"]
        new_sid = self.styles.by_name.get(p.style)
        if new_sid is None:
            raise ExportError("%s: unknown style %r" % (path, p.style))
        if pp:
            e = pp[0]
            pel = copy.deepcopy(e.payload[0])
            pPr = copy.deepcopy(e.payload[1]) if len(e.payload) > 1 else None
            if self.styles.by_id.get(e.extra["style_id"], e.extra["style_id"]) != p.style:
                if pPr is None:
                    pPr = W_("pPr")
                old = pPr.find(qn("w:pStyle"))
                if old is not None:
                    pPr.remove(old)
                if new_sid != self.styles.default_id:
                    pPr.insert(0, W_("pStyle", val=new_sid))
        else:
            pel = W_("p")
            pPr = None
            if new_sid != self.styles.default_id:
                pPr = W_("pPr")
                pPr.append(W_("pStyle", val=new_sid))
        if pPr is not None:
            pel.append(pPr)
        return pel

    def para(self, p, path):
        pel = self.ppr(p, path)
        ents = self.by[path]
        n = len(p.chars)
        owner = [None] * n
        zruns = []
        for r in ents["run"]:
            if r.start == r.end:
                zruns.append(r)
            for c in range(max(0, r.start), min(n, r.end)):
                owner[c] = r
        pkeep = {}
        for c, ch in enumerate(p.chars):
            k = M.keep_num(ch)
            if k is not None:
                ke = self.keep.get(k)
                if ke is None:
                    raise ExportError("placeholder k%d has no remainder entry" % k)
                if not ke.extra.get("in_run"):
                    pkeep[c] = ke
        bounds = {m.start for m in ents["marker"]} | {z.start for z in zruns}
        for w in ents["wrap"]:
            bounds |= {w.start, w.end}
        segs = []
        c = 0
        while c < n:
            if c in pkeep:
                c += 1
                continue
            a = c
            key = (id(owner[c]), p.bold[c], p.italic[c])
            c += 1
            while c < n and c not in pkeep and c not in bounds and (id(owner[c]), p.bold[c], p.italic[c]) == key:
                c += 1
            segs.append((a, c, owner[a]))
        # run-internal markers go into the segment of their own run when it still covers them
        rm_at = defaultdict(list)
        for m in ents["rmarker"]:
            target = None
            for si, s in enumerate(segs):
                if s[2] is not None and s[2].id == m.extra["run"] and s[0] <= m.start <= s[1]:
                    target = si; break
            if target is None:
                for si, s in enumerate(segs):
                    if s[0] <= m.start < s[1]:
                        target = si; break
            if target is None and segs and segs[-1][1] == m.start:
                target = len(segs) - 1
            if target is None:
                for z in zruns:
                    if z.id == m.extra["run"]:
                        target = ("z", z.id); break
            if target is None:
                self.issues.append("rmarker at %s:%d had no run to live in" % (path, m.start))
                continue
            rm_at[target].append(m)
        items = []
        for w in ents["wrap"]:
            items.append((w.start, 0, w.seq, "open", w))
            items.append((w.end, 0, w.extra["seq_close"], "close", w))
        for m in ents["marker"]:
            items.append((m.start, 0, m.seq, "marker", m))
        for z in zruns:
            items.append((z.start, 0, z.seq, "zrun", z))
        for c, k in pkeep.items():
            items.append((c, 1, k.seq, "pkeep", k))
        for si, s in enumerate(segs):
            items.append((s[0], 1, s[2].seq if s[2] is not None else float("inf"), "seg", si))
        items.sort(key=lambda t: (t[0], t[1], t[2]))
        stack = [pel]
        open_ids = []
        for pos, _, _, what, obj in items:
            parent = stack[-1]
            if what == "open":
                el = copy.deepcopy(obj.payload[0])
                parent.append(el)
                stack.append(el)
                open_ids.append(obj.id)
            elif what == "close":
                if obj.id not in open_ids:
                    self.issues.append("wrapper %d closed before it opened" % obj.id)
                    continue
                while open_ids and open_ids[-1] != obj.id:
                    self.issues.append("wrapper %d mis-nested" % open_ids[-1])
                    open_ids.pop(); stack.pop()
                open_ids.pop(); stack.pop()
            elif what == "marker":
                parent.append(copy.deepcopy(obj.payload[0]))
            elif what == "pkeep":
                for el in obj.payload:
                    parent.append(copy.deepcopy(el))
            elif what == "zrun":
                r = copy.deepcopy(obj.payload[0])
                for el in obj.payload[1:]:
                    r.append(copy.deepcopy(el))
                for m in rm_at.get(("z", obj.id), []):
                    r.append(copy.deepcopy(m.payload[0]))
                parent.append(r)
            else:
                parent.append(self.run_el(p, segs[obj], rm_at.get(obj, [])))
        return pel

    def run_el(self, p, seg, rmarkers):
        a, b, own = seg
        if own is not None:
            r = copy.deepcopy(own.payload[0])
            rpr = copy.deepcopy(own.payload[1]) if len(own.payload) > 1 else None
            t_attrs = own.extra.get("t_attrs", [])
        else:
            r, rpr, t_attrs = W_("r"), None, []
        want_b, want_i = p.bold[a], p.italic[a]
        if rpr is None and (want_b or want_i):
            rpr = W_("rPr")
        if rpr is not None:
            set_flag(rpr, "b", want_b)
            set_flag(rpr, "i", want_i)
            r.append(rpr)
        rm = defaultdict(list)
        for m in rmarkers:
            rm[m.start].append(m)
        buf = []
        tcount = [0]

        def flush():
            if not buf:
                return
            s = "".join(buf)
            t = W_("t")
            attrs, orig = t_attrs[tcount[0]] if tcount[0] < len(t_attrs) else (t_attrs[-1][0] if t_attrs else {}, None)
            for k, v in attrs.items():
                t.set(k, v)
            # a w:t keeps its attributes as they were; changed text that needs it gets preserve
            if s != orig and (s != s.strip() or "  " in s) and t.get("{%s}space" % XML_NS) is None:
                t.set("{%s}space" % XML_NS, "preserve")
            t.text = s
            r.append(t)
            tcount[0] += 1
            buf.clear()

        for c in range(a, b):
            for m in sorted(rm.get(c, []), key=lambda e: e.seq):
                flush(); r.append(copy.deepcopy(m.payload[0]))
            ch = p.chars[c]
            k = M.keep_num(ch)
            if k is not None:
                flush()
                for el in self.keep[k].payload:
                    r.append(copy.deepcopy(el))
            elif ch == "\t":
                flush(); r.append(W_("tab"))
            elif ch == "\n":
                flush(); r.append(W_("br"))
            else:
                buf.append(ch)
        flush()
        for m in sorted(rm.get(b, []), key=lambda e: e.seq):
            r.append(copy.deepcopy(m.payload[0]))
        return r

    # ------------------------------------------------------------ table
    def table(self, t, bi):
        te = self.by[(bi,)]["tbl"]
        if te:
            tbl = copy.deepcopy(te[0].payload[0])
            for el in te[0].payload[1:]:
                tbl.append(copy.deepcopy(el))
        else:
            tbl = W_("tbl")
            tbl.append(W_("tblPr"))
            grid = W_("tblGrid")
            for _ in t.rows[0]:
                grid.append(W_("gridCol", w="2000"))
            tbl.append(grid)
        for ri, row in enumerate(t.rows):
            re_ = self.by[(bi, ri)]["tr"]
            if re_:
                tr = copy.deepcopy(re_[0].payload[0])
                for el in re_[0].payload[1:]:
                    tr.append(copy.deepcopy(el))
            else:
                tr = W_("tr")
            ci = 0
            while ci < len(row):
                cell = row[ci]
                span = 1
                while ci + span < len(row) and row[ci + span].marker == "||":
                    span += 1
                if cell.marker == "||":
                    raise ExportError("row %d: || with no cell to its left" % ri)
                path = (bi, ri, ci)
                ce = self.by[path]["tc"]
                if ce:
                    tc = copy.deepcopy(ce[0].payload[0])
                    tcpr = copy.deepcopy(ce[0].payload[1]) if len(ce[0].payload) > 1 else None
                else:
                    tc, tcpr = W_("tc"), None
                below = t.rows[ri + 1][ci] if ri + 1 < len(t.rows) and ci < len(t.rows[ri + 1]) else None
                tcpr = self.merge_props(tcpr, span, cell.marker == "^^", below is not None and below.marker == "^^")
                if tcpr is not None:
                    tc.append(tcpr)
                para = cell.para
                if para is None:  # '^^' cell: its (empty) paragraph lives in the remainder
                    pp = self.by[path]["ppr"]
                    sid = pp[0].extra["style_id"] if pp else self.styles.default_id
                    para = M.Para(self.styles.by_id.get(sid, sid), "", [], [])
                tc.append(self.para(para, path))
                tr.append(tc)
                ci += span
            tbl.append(tr)
        return tbl

    def merge_props(self, tcpr, span, covered, top):
        def get(name):
            return tcpr.find(qn("w:" + name)) if tcpr is not None else None
        gs = get("gridSpan")
        cur_span = int(wval(gs)) if gs is not None else 1
        vm = get("vMerge")
        cur_v = None if vm is None else ("restart" if wval(vm) == "restart" else "continue")
        want_v = "continue" if covered else ("restart" if top else None)
        if cur_span == span and cur_v == want_v:
            return tcpr
        if tcpr is None:
            tcpr = W_("tcPr")
        if cur_span != span:
            if gs is not None:
                tcpr.remove(gs)
            if span > 1:
                insert_ordered(tcpr, W_("gridSpan", val=str(span)), TCPR_ORDER)
        if cur_v != want_v:
            if vm is not None:
                tcpr.remove(vm)
            if want_v == "continue":
                insert_ordered(tcpr, W_("vMerge"), TCPR_ORDER)
            elif want_v == "restart":
                insert_ordered(tcpr, W_("vMerge", val="restart"), TCPR_ORDER)
        return tcpr


def export_xml(doc, blocks, entries):
    ex = Exporter(doc, blocks, entries)
    return ex.run(), ex.issues
