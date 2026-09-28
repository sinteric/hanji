#!/usr/bin/env python3
"""Steps 2-3 for docx: make tracked files and validate them.

    cargo build --release          # builds target/release/tcspike (rdocx 0.14.0)
    python3 make_direct.py         # hanji-style direct exports (route A input)
    python3 inject.py              # hand-written revision XML (route B)
    python3 validate.py [--granularity run|word] [--no-lo]   # -> results/docx-<g>.json

Route A: rdocx `Document::compare_with_options(E0, Ek, author, date, opts)`
with `--granularity run` (default) or `word`. E0 is the unedited hanji
export, Ek the direct-change export of edit k.
Route B: out/inject/<stem>.docx, revision XML written by inject.py.

Checks per tracked file:
  wf        every XML part parses (lxml)
  rdocx     re-open; revisions() count/author/date; save + re-open keeps them;
            accept_all / reject_all succeed
  text      paragraph text of rdocx accept_all == edited text, reject_all == base
            text, and both equal this script's own accept/reject projection
  LO        LibreOffice (UNO, headless): redline count/types/author; Accept All
            and Reject All give the same paragraph text as LibreOffice's own
            round trip of the edited and base files; a PDF with changes shown
"""
import glob
import json
import os
import re
import subprocess
import sys
import time
import zipfile

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, "..", "remainder", "corpus")
OUT = os.path.join(HERE, "out")
RES = os.path.join(HERE, "results")
BIN = os.path.join(HERE, "target", "release", "tcspike")
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
w = lambda t: "{%s}%s" % (W, t)  # noqa: E731
AUTHOR = "hanji (model edit)"


# ---------------------------------------------------------------- text views
def _inside(e, tags, stop):
    for a in e.iterancestors():
        if a is stop:
            return False
        if a.tag in tags:
            return True
    return False


def para_texts(path, view="plain"):
    """Body paragraph texts in document order. view: plain | accept | reject."""
    root = etree.fromstring(zipfile.ZipFile(path).read("word/document.xml"))
    gone = {"accept": (w("del"), w("moveFrom")), "reject": (w("ins"), w("moveTo"))}.get(view, ())
    mark_gone = {"accept": (w("del"), w("moveFrom")), "reject": (w("ins"), w("moveTo"))}.get(view, ())
    out, carry = [], ""
    for p in root.find(w("body")).iter(w("p")):
        s = []
        for e in p.iter(w("t"), w("delText"), w("tab")):
            if any(a.tag == w("p") for a in e.iterancestors() if a is not p and _is_desc(a, p)):
                continue  # nested paragraph (text box): counted on its own
            if e.tag == w("delText") and view != "reject":
                continue
            if e.tag == w("tab") and e.getparent().tag != w("r"):
                continue  # tab stop definition, not a tab character
            if gone and _inside(e, gone, p):
                continue
            s.append("\t" if e.tag == w("tab") else (e.text or ""))
        text = carry + "".join(s)
        rpr = p.find(w("pPr") + "/" + w("rPr"))
        if rpr is not None and any(rpr.find(t) is not None for t in mark_gone):
            carry = text  # paragraph mark removed in this view: joins the next paragraph
            continue
        carry = ""
        out.append(text)
    if carry:
        out.append(carry)
    return out


def _is_desc(a, p):
    return any(x is p for x in a.iterancestors())


def diff_count(a, b):
    import difflib
    return sum(max(i2 - i1, j2 - j1) for op, i1, i2, j1, j2 in
               difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes() if op != "equal")


def well_formed(path):
    bad = []
    z = zipfile.ZipFile(path)
    for n in z.namelist():
        if n.endswith(".xml") or n.endswith(".rels"):
            try:
                etree.fromstring(z.read(n))
            except etree.XMLSyntaxError as e:
                bad.append("%s: %s" % (n, e))
    return bad


def ws_unpreserved(path):
    """w:t / w:delText inside a revision wrapper whose text starts or ends with
    whitespace but lacks xml:space="preserve": Word and LibreOffice drop that
    whitespace. (Only revision content counts: the corpus has its own cases.)"""
    root = etree.fromstring(zipfile.ZipFile(path).read("word/document.xml"))
    xs = "{http://www.w3.org/XML/1998/namespace}space"
    wrappers = (w("ins"), w("del"), w("moveFrom"), w("moveTo"))
    return sum(1 for e in root.iter(w("t"), w("delText"))
               if e.text and e.text != e.text.strip() and e.get(xs) != "preserve"
               and any(a.tag in wrappers for a in e.iterancestors()))


def revision_xml_kept(tracked, resaved):
    """Every w:ins/w:del/w:pPrChange element survives rdocx save byte for byte."""
    import re
    pat = re.compile(rb"<w:(?:ins|del|pPrChange|moveFrom|moveTo)\b[^>]*?(?:/>|>.*?</w:(?:ins|del|pPrChange|moveFrom|moveTo)>)", re.S)
    a = pat.findall(zipfile.ZipFile(tracked).read("word/document.xml"))
    b = set(pat.findall(zipfile.ZipFile(resaved).read("word/document.xml")))
    return sum(1 for x in a if x in b), len(a)


# ---------------------------------------------------------------- LibreOffice
class LO:
    def __init__(self):
        import uno  # noqa: F401
        self.proc = subprocess.Popen(["soffice", "--headless", "--invisible", "--norestore",
                                      "-env:UserInstallation=file://" + os.path.join(OUT, "lo-profile"),
                                      "--accept=pipe,name=tcspike;urp;"],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        import uno
        local = uno.getComponentContext()
        resolver = local.ServiceManager.createInstanceWithContext("com.sun.star.bridge.UnoUrlResolver", local)
        for _ in range(60):
            try:
                ctx = resolver.resolve("uno:pipe,name=tcspike;urp;StarOffice.ComponentContext")
                break
            except Exception:
                time.sleep(1)
        else:
            raise RuntimeError("soffice did not start")
        self.smgr = ctx.ServiceManager
        self.desktop = self.smgr.createInstanceWithContext("com.sun.star.frame.Desktop", ctx)
        self.dispatcher = self.smgr.createInstanceWithContext("com.sun.star.frame.DispatchHelper", ctx)

    @staticmethod
    def _pv(**kw):
        from com.sun.star.beans import PropertyValue
        out = []
        for k, v in kw.items():
            p = PropertyValue()
            p.Name, p.Value = k, v
            out.append(p)
        return tuple(out)

    def _load(self, path):
        import uno
        return self.desktop.loadComponentFromURL(uno.systemPathToFileUrl(os.path.abspath(path)), "_blank", 0,
                                                 self._pv(Hidden=True))

    def _store(self, doc, path, flt):
        import uno
        doc.storeToURL(uno.systemPathToFileUrl(os.path.abspath(path)), self._pv(FilterName=flt))

    def roundtrip(self, src, dst, action=None):
        doc = self._load(src)
        try:
            if action:
                self.dispatcher.executeDispatch(doc.getCurrentController().getFrame(), action, "", 0, ())
            self._store(doc, dst, "MS Word 2007 XML")
        finally:
            doc.close(True)

    def redlines(self, src, pdf=None):
        doc = self._load(src)
        try:
            kinds, authors = {}, set()
            en = doc.getRedlines().createEnumeration()
            while en.hasMoreElements():
                r = en.nextElement()
                t = r.getPropertyValue("RedlineType")
                kinds[t] = kinds.get(t, 0) + 1
                authors.add(r.getPropertyValue("RedlineAuthor"))
            if pdf:
                doc.setPropertyValue("ShowChanges", True)
                self._store(doc, pdf, "writer_pdf_Export")
            return {"count": sum(kinds.values()), "by_type": kinds, "authors": sorted(authors)}
        finally:
            doc.close(True)

    def close(self):
        try:
            self.desktop.terminate()
        except Exception:
            pass
        self.proc.wait(timeout=60)


def lo_check(lo, tracked, base, edited, stem):
    d = os.path.join(OUT, "lo")
    os.makedirs(d, exist_ok=True)
    r = {}
    try:
        pdf = os.path.join(d, stem + ".changes.pdf")
        r["redlines"] = lo.redlines(tracked, pdf)
        r["pdf"] = os.path.relpath(pdf, HERE)
        for name, src, act in (("acc", tracked, ".uno:AcceptAllTrackedChanges"),
                               ("rej", tracked, ".uno:RejectAllTrackedChanges"),
                               ("base", base, None), ("edited", edited, None)):
            lo.roundtrip(src, os.path.join(d, "%s.%s.docx" % (stem, name)), act)
        # LibreOffice keeps the revisions a base or edited file already has: project them
        views = {"acc": "plain", "rej": "plain", "base": "reject", "edited": "accept"}
        t = {n: para_texts(os.path.join(d, "%s.%s.docx" % (stem, n)), v) for n, v in views.items()}
        r["accept_matches_edited"] = t["acc"] == t["edited"]
        r["reject_matches_base"] = t["rej"] == t["base"]
        digits = lambda ps: [re.sub(r"\d", "0", x) for x in ps]  # noqa: E731
        # LibreOffice recomputes TIME / SEQ field results on every load
        r["accept_matches_ignoring_field_digits"] = digits(t["acc"]) == digits(t["edited"])
        r["reject_matches_ignoring_field_digits"] = digits(t["rej"]) == digits(t["base"])
        if not r["accept_matches_edited"]:
            r["accept_diff_paras"] = diff_count(t["acc"], t["edited"])
        if not r["reject_matches_base"]:
            r["reject_diff_paras"] = diff_count(t["rej"], t["base"])
        r["ok"] = True
    except Exception as e:  # noqa: BLE001
        r["ok"] = False
        r["error"] = repr(e)
    return r


# ---------------------------------------------------------------- driver
def tcspike(*args):
    p = subprocess.run([BIN, *args], capture_output=True, text=True, timeout=600)
    try:
        return json.loads(p.stdout)
    except json.JSONDecodeError:
        return {"ok": False, "stage": "crash", "error": (p.stderr or p.stdout)[-400:]}


def check(tracked, base, edited, stem, lo):
    prefix = os.path.join(OUT, "check", stem)
    os.makedirs(os.path.dirname(prefix), exist_ok=True)
    r = {"wf_errors": well_formed(tracked), "ws_unpreserved": ws_unpreserved(tracked)}
    r["rdocx"] = tcspike("check", tracked, prefix)
    if r["rdocx"].get("ok"):
        kept, total = revision_xml_kept(tracked, prefix + ".resaved.docx")
        r["rdocx"]["revision_xml_kept_on_save"] = "%d/%d" % (kept, total)
        acc, rej = prefix + ".accepted.docx", prefix + ".rejected.docx"
        if os.path.exists(acc) and os.path.exists(rej):
            # base: reject view (a base with its own revisions is rejected too); edited: accept view
            want_acc, want_rej = para_texts(edited, "accept"), para_texts(base, "reject")
            got_acc, got_rej = para_texts(acc), para_texts(rej)
            r["text"] = {
                "projection_accept_eq_edited": para_texts(tracked, "accept") == want_acc,
                "projection_reject_eq_base": para_texts(tracked, "reject") == want_rej,
                "rdocx_accept_eq_edited": got_acc == want_acc,
                "rdocx_reject_eq_base": got_rej == want_rej,
            }
            if got_acc != want_acc:
                r["text"]["accept_diff_paras"] = diff_count(got_acc, want_acc)
            if got_rej != want_rej:
                r["text"]["reject_diff_paras"] = diff_count(got_rej, want_rej)
    if lo:
        r["lo"] = lo_check(lo, tracked, base, edited, stem)
    return r


def main():
    use_lo = "--no-lo" not in sys.argv
    gran = sys.argv[sys.argv.index("--granularity") + 1] if "--granularity" in sys.argv else "run"
    lo = LO() if use_lo else None
    os.makedirs(RES, exist_ok=True)
    results = {"route_a": [], "route_b": []}
    try:
        manifest = json.load(open(os.path.join(OUT, "direct", "manifest.json")))
        for m in manifest:
            stem, e = m["file"], m["edit"]
            base = os.path.join(OUT, "direct", stem, "E0.docx")
            edited = os.path.join(OUT, "direct", stem, e + ".docx")
            tracked = os.path.join(OUT, "tracked-" + gran, "%s-%s.docx" % (stem, e))
            os.makedirs(os.path.dirname(tracked), exist_ok=True)
            row = {"file": stem, "edit": e, "what": m["what"]}
            c = tcspike("compare", base, edited, tracked, gran)
            row["granularity"] = gran
            row["compare"] = c if c.get("ok") else {"ok": False, "error": c.get("error", "")[:300]}
            if c.get("ok") and c["revisions"]["count"]:
                row.update(check(tracked, base, edited, "A%s-%s-%s" % (gran[0], stem, e), lo))
            results["route_a"].append(row)
            print("A", stem, e, "ok" if c.get("ok") else c.get("error", "")[:80], flush=True)
        for inj in sorted(glob.glob(os.path.join(OUT, "inject", "*.docx"))):
            stem = os.path.basename(inj)[:-5]
            base = os.path.join(CORPUS, stem + ".docx")
            row = {"file": stem, "edits": json.load(open(inj[:-5] + ".json"))}
            # the injected file's own accept view is the "edited" document for LibreOffice
            row.update(check(inj, base, inj, "B-" + stem, lo))
            # for route B the independent expectation is: reject view == original text
            results["route_b"].append(row)
            print("B", stem, "done", flush=True)
    finally:
        if lo:
            lo.close()
    results["granularity"] = gran
    json.dump(results, open(os.path.join(RES, "docx-%s.json" % gran), "w"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()
