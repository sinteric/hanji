#!/usr/bin/env python3
"""Approach B: write w:ins / w:del / w:pPrChange directly into word/document.xml
(what hanji's own XML-level exporter would do), without any engine.

    python3 inject.py   # writes out/inject/<stem>.docx and out/inject/<stem>.json

Five edits per file, each picked automatically:
  X1 deleted run inside a paragraph   <w:del><w:r><w:delText>…</w:delText></w:r></w:del>
  X2 inserted paragraph               previous paragraph's mark <w:pPr><w:rPr><w:ins/></w:rPr></w:pPr>,
                                      new <w:p> with that paragraph's pPr and <w:ins><w:r>…</w:r></w:ins>
  X3 changed table cell               <w:del>old run</w:del><w:ins>new run</w:ins> in the cell's first paragraph
  X4 deleted paragraph                every run in <w:del>, paragraph mark <w:pPr><w:rPr><w:del/></w:rPr></w:pPr>
  X5 restyle                          <w:pPr><w:pStyle new/><w:pPrChange><w:pPr><w:pStyle old/></w:pPr></w:pPrChange></w:pPr>
"""
import copy
import glob
import json
import os
import sys
import zipfile

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, "..", "remainder", "corpus")
OUT = os.path.join(HERE, "out", "inject")
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
w = lambda t: "{%s}%s" % (W, t)  # noqa: E731
AUTHOR, DATE = "hanji (model edit)", "2026-09-28T00:00:00Z"
FILES = ["korean-report", "testWORD_various", "loadAndSave", "docx4j-tables", "Bug54849", "tdf154481"]


class Ids:
    def __init__(self, root):
        used = [int(v) for v in root.xpath("//@w:id", namespaces={"w": W}) if v.lstrip("-").isdigit()]
        self.n = max(used + [0]) + 1000

    def __call__(self):
        self.n += 1
        return str(self.n)


def mark(tag, ids, **extra):
    e = etree.Element(w(tag))
    e.set(w("id"), ids())
    e.set(w("author"), AUTHOR)
    e.set(w("date"), DATE)
    for k, v in extra.items():
        e.set(w(k), v)
    return e


def text_of(p):
    return "".join(t.text or "" for t in p.iter(w("t")))


def in_table(p):
    return any(a.tag == w("tc") for a in p.iterancestors())


def plain_runs(p):
    """Direct-child runs holding only rPr + one w:t (safe to split)."""
    out = []
    for r in p.findall(w("r")):
        kids = [k for k in r if k.tag != w("rPr")]
        if len(kids) == 1 and kids[0].tag == w("t") and kids[0].text:
            out.append(r)
    return out


def body_paras(body, table):
    return [p for p in body.iter(w("p")) if in_table(p) == table and not p.xpath(".//w:ins|.//w:del|.//w:moveFrom|.//w:moveTo", namespaces={"w": W})]


def ensure(parent, tag, first=False):
    e = parent.find(w(tag))
    if e is None:
        e = etree.Element(w(tag))
        parent.insert(0, e) if first else parent.append(e)
    return e


def ppr(p):
    return ensure(p, "pPr", first=True)


def set_t(t, s):
    t.text = s
    if s != s.strip():
        t.set("{http://www.w3.org/XML/1998/namespace}space", "preserve")


def x1_delete_inside(body, ids, log):
    for p in body_paras(body, False):
        for r in plain_runs(p):
            s = r.find(w("t")).text
            words = s.split(" ")
            if len(words) < 3 or len(s) < 12:
                continue
            mid = " " + words[1]
            i = s.index(mid)
            pre, post = s[:i], s[i + len(mid):]
            r_pre, r_del, r_post = copy.deepcopy(r), copy.deepcopy(r), copy.deepcopy(r)
            set_t(r_pre.find(w("t")), pre)
            set_t(r_post.find(w("t")), post)
            t = r_del.find(w("t"))
            t.tag = w("delText")
            set_t(t, mid)
            d = mark("del", ids)
            d.append(r_del)
            parent = r.getparent()
            k = parent.index(r)
            parent.remove(r)
            for j, e in enumerate([r_pre, d, r_post]):
                parent.insert(k + j, e)
            log.append({"edit": "X1", "what": "delete %r inside %r" % (mid, s[:60])})
            return
    log.append({"edit": "X1", "what": "n/a"})


def x2_insert_para(body, ids, log):
    """Word's own shape for "Enter at the end of P, then type": the paragraph mark
    that ends P is the inserted one, and the new paragraph takes P's original mark
    (a copy of P's pPr). Rejecting removes the new text and the inserted mark, so P
    keeps its own properties and nothing has to merge into a following table."""
    paras = [p for p in body_paras(body, False)
             if p.find(w("pPr")) is None or p.find(w("pPr")).find(w("sectPr")) is None]
    if not paras:
        return log.append({"edit": "X2", "what": "n/a"})
    anchor = paras[min(2, len(paras) - 1)]
    p = etree.Element(w("p"))
    if anchor.find(w("pPr")) is not None:
        p.append(copy.deepcopy(anchor.find(w("pPr"))))
    ensure(ppr(anchor), "rPr").append(mark("ins", ids))
    ins = mark("ins", ids)
    r = etree.SubElement(ins, w("r"))
    set_t(etree.SubElement(r, w("t")), "모델이 추가한 문단입니다. Paragraph inserted by the model.")
    p.append(ins)
    anchor.addnext(p)
    log.append({"edit": "X2", "what": "insert a paragraph after %r" % text_of(anchor)[:60]})


def x3_cell(body, ids, log):
    for p in body_paras(body, True):
        runs = plain_runs(p)
        if not runs or len(text_of(p)) < 1:
            continue
        r = runs[0]
        old = r.find(w("t")).text
        r_new = copy.deepcopy(r)
        set_t(r_new.find(w("t")), "1,204")
        r_old = copy.deepcopy(r)
        r_old.find(w("t")).tag = w("delText")
        d, i = mark("del", ids), mark("ins", ids)
        d.append(r_old)
        i.append(r_new)
        r.addprevious(d)
        r.addprevious(i)
        r.getparent().remove(r)
        log.append({"edit": "X3", "what": "table cell run %r -> '1,204'" % old[:40]})
        return
    log.append({"edit": "X3", "what": "n/a"})


def x4_delete_para(body, ids, log):
    paras = [p for p in body_paras(body, False) if plain_runs(p) and len(plain_runs(p)) == len(p.findall(w("r")))
             and p.getnext() is not None and p.getnext().tag == w("p")]
    if len(paras) < 4:
        return log.append({"edit": "X4", "what": "n/a"})
    p = paras[len(paras) // 2]
    for r in p.findall(w("r")):
        r.find(w("t")).tag = w("delText")
        d = mark("del", ids)
        r.addprevious(d)
        d.append(r)
    ensure(ppr(p), "rPr").append(mark("del", ids))
    log.append({"edit": "X4", "what": "delete paragraph %r" % text_of_del(p)[:60]})


def text_of_del(p):
    return "".join(t.text or "" for t in p.iter(w("delText")))


def x5_restyle(body, ids, styles, log):
    for p in body_paras(body, False):
        pp = p.find(w("pPr"))
        if pp is None or pp.find(w("sectPr")) is not None or not text_of(p):
            continue
        cur = pp.find(w("pStyle"))
        old_id = cur.get(w("val")) if cur is not None else None
        new_id = next((s for s in ("Quote", "IntenseQuote", "Heading2", "Title") if s in styles and s != old_id), None)
        if not new_id:
            break
        before = copy.deepcopy(pp)
        for k in list(before):
            if k.tag in (w("rPr"), w("sectPr"), w("pPrChange")):
                before.remove(k)
        if cur is None:
            cur = etree.Element(w("pStyle"))
            pp.insert(0, cur)
        cur.set(w("val"), new_id)
        ch = mark("pPrChange", ids)
        ch.append(before)
        # pPrChange is the last child of pPr (after rPr/sectPr per schema order it precedes nothing)
        pp.append(ch)
        log.append({"edit": "X5", "what": "restyle %r: %s -> %s" % (text_of(p)[:40], old_id, new_id)})
        return
    log.append({"edit": "X5", "what": "n/a"})


def main():
    os.makedirs(OUT, exist_ok=True)
    for stem in FILES:
        src = os.path.join(CORPUS, stem + ".docx")
        zin = zipfile.ZipFile(src)
        root = etree.fromstring(zin.read("word/document.xml"))
        styles = set(etree.fromstring(zin.read("word/styles.xml")).xpath("//w:style/@w:styleId", namespaces={"w": W}))
        body = root.find(w("body"))
        ids, log = Ids(root), []
        x1_delete_inside(body, ids, log)
        x2_insert_para(body, ids, log)
        x3_cell(body, ids, log)
        x4_delete_para(body, ids, log)
        x5_restyle(body, ids, styles, log)
        xml = etree.tostring(root, xml_declaration=True, encoding="UTF-8", standalone=True)
        dst = os.path.join(OUT, stem + ".docx")
        with zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as zout:
            for it in zin.infolist():
                zout.writestr(it, xml if it.filename == "word/document.xml" else zin.read(it.filename))
        json.dump(log, open(os.path.join(OUT, stem + ".json"), "w"), ensure_ascii=False, indent=1)
        print(stem, [l["edit"] + ("" if l["what"] != "n/a" else "(n/a)") for l in log])


if __name__ == "__main__":
    main()
