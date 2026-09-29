#!/usr/bin/env python3
"""LibreOffice's Accept All / Reject All on the docx corpus test's tracked exports (DESIGN.md §10.2).

    cargo test --release -p hanji-docx --test corpus corpus_tracked   # writes target/tmp/docx-tracked-out
    python3 crates/hanji-docx/validate/lo_review.py [DIR]            # needs soffice, python3-uno, lxml

For every tracked export E…-{C,exact}.docx, over UNO (headless):
  - LibreOffice lists the author's changes as redlines;
  - its Accept All gives the same paragraph text as its Accept All of the
    test harness's accepted file (…accepted.docx), and its Reject All the same
    as its Reject All of the harness's rejected file. Doing the same to both
    sides also resolves the revisions the imported file had of its own.
"""
import glob
import os
import re
import subprocess
import sys
import tempfile
import time
import zipfile

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
DIR = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else os.path.join(HERE, "..", "..", "..", "target", "tmp", "docx-tracked-out")
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
AUTHOR = "hanji (model edit)"


def w(t):
    return "{%s}%s" % (W, t)


def para_texts(path):
    root = etree.fromstring(zipfile.ZipFile(path).read("word/document.xml"))
    out = []
    for p in root.find(w("body")).iter(w("p")):
        s = [("\t" if e.tag == w("tab") else (e.text or "")) for e in p.iter(w("t"), w("tab"))
             if e.tag != w("tab") or e.getparent().tag == w("r")]
        out.append("".join(s))
    return out


class LO:
    def __init__(self, profile):
        import uno
        self.proc = subprocess.Popen(["soffice", "--headless", "--invisible", "--norestore",
                                      "-env:UserInstallation=file://" + profile,
                                      "--accept=pipe,name=hanjilo;urp;"],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        local = uno.getComponentContext()
        resolver = local.ServiceManager.createInstanceWithContext("com.sun.star.bridge.UnoUrlResolver", local)
        for _ in range(60):
            try:
                ctx = resolver.resolve("uno:pipe,name=hanjilo;urp;StarOffice.ComponentContext")
                break
            except Exception:
                time.sleep(1)
        else:
            raise RuntimeError("soffice did not start")
        smgr = ctx.ServiceManager
        self.desktop = smgr.createInstanceWithContext("com.sun.star.frame.Desktop", ctx)
        self.dispatcher = smgr.createInstanceWithContext("com.sun.star.frame.DispatchHelper", ctx)

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

    def redlines(self, path):
        doc = self._load(path)
        try:
            n = 0
            en = doc.getRedlines().createEnumeration()
            while en.hasMoreElements():
                n += en.nextElement().getPropertyValue("RedlineAuthor") == AUTHOR
            return n
        finally:
            doc.close(True)

    def resolved(self, src, action, dst):
        import uno
        doc = self._load(src)
        try:
            self.dispatcher.executeDispatch(doc.getCurrentController().getFrame(), action, "", 0, ())
            doc.storeToURL(uno.systemPathToFileUrl(os.path.abspath(dst)), self._pv(FilterName="MS Word 2007 XML"))
        finally:
            doc.close(True)
        return para_texts(dst)

    def close(self):
        try:
            self.desktop.terminate()
        except Exception:
            pass
        self.proc.wait(timeout=60)


def main():
    only = sys.argv[2] if len(sys.argv) > 2 else ""
    files = sorted(f for f in glob.glob(os.path.join(DIR, "*", "E*.docx"))
                   if os.path.basename(f).count(".") == 1 and only in f)
    tmp = tempfile.mkdtemp(prefix="hanji-lo-review-")
    lo = LO(os.path.join(tmp, "profile"))
    n = {"files": 0, "opened": 0, "listed": 0, "accept": 0, "reject": 0}
    problems = []
    digits = lambda ps: [re.sub(r"\d", "0", x) for x in ps]  # noqa: E731
    try:
        for f in files:
            n["files"] += 1
            name = os.path.relpath(f, DIR)
            try:
                listed = lo.redlines(f)
            except Exception as e:  # noqa: BLE001
                problems.append("%s: does not open: %r" % (name, e))
                continue
            n["opened"] += 1
            if listed:
                n["listed"] += 1
            else:
                problems.append("%s: no redline by %s" % (name, AUTHOR))
            stem = os.path.join(tmp, name.replace(os.sep, "_")[:-5])
            for key, action, ref in (("accept", ".uno:AcceptAllTrackedChanges", "accepted"),
                                     ("reject", ".uno:RejectAllTrackedChanges", "rejected")):
                try:
                    got = lo.resolved(f, action, stem + "." + key + ".docx")
                    want = lo.resolved(f[:-5] + "." + ref + ".docx", action, stem + "." + ref + ".docx")
                except Exception as e:  # noqa: BLE001
                    problems.append("%s: %s: %r" % (name, key, e))
                    continue
                if got == want:
                    n[key] += 1
                elif digits(got) == digits(want):
                    n[key] += 1
                    problems.append("%s: %s equal once field digits are ignored (LibreOffice recomputes fields)"
                                    % (name, key))
                else:
                    g, v = digits(got), digits(want)
                    k = next((i for i, (a, b) in enumerate(zip(g, v)) if a != b), min(len(got), len(want)))
                    problems.append("%s: %s differs at paragraph %d: %r / %r" % (
                        name, key, k + 1, got[k] if k < len(got) else None, want[k] if k < len(want) else None))
    finally:
        lo.close()
    print("LibreOffice over %(files)d tracked exports: opened %(opened)d, lists the author's changes %(listed)d, "
          "Accept All = harness %(accept)d, Reject All = harness %(reject)d" % n)
    for p in problems:
        print("  " + p)


if __name__ == "__main__":
    main()
