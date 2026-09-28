"""Classify every table in the survey corpus by the shape hanji's pipe tables need.

    python3 survey.py [corpus_dir]      # default ./corpus (see fetch.py)

Writes results/tables.csv (one row per top-level table), results/docs.csv (one
row per document) and results/summary.md (the frequency tables in SURVEY.md).

Categories, most blocking first; a table gets the first that applies, and every
shape it has is also listed in `flags`:

  nested      a cell holds another table
  row-cc      a content control (w:sdt) or customXml wraps rows (tbl > sdt) or
              cells (tr > sdt) — "row-level content control"
  grid        a row starts or ends short: w:gridBefore / w:gridAfter, or cells
              that do not reach the grid width (implicit); HWPX: grid slots no
              cell covers
  other       anything else a pipe table cannot say (named in `flags`)
  multipara   some cell has more than one paragraph (<p/>, supported since §6
              round 3)
  merged      ^^ / || merges only
  plain       a plain pipe table

docx reuses the remainder prototype's classifier
(../remainder/hanji_rem/importer.py, Importer.table_reason): its first reason is
recorded as `proto_reason` and checked against the full scan here. The scan
differs from it in two deliberate ways: it finds *every* shape rather than the
first, and it treats zero-width markers (bookmarks, proofing marks, permission
and comment ranges) as transparent, since the remainder can anchor those
between rows or cells without a new syntax.
"""
import collections
import csv
import glob
import os
import re
import sys
import zipfile

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "remainder"))
from hanji_rem.importer import Importer, MARKERS  # noqa: E402
from hanji_rem.ooxml import qn, local, wval, parse, DOC_PART  # noqa: E402

ORDER = ["nested", "row-cc", "grid", "other", "multipara", "merged", "plain"]
MARK = MARKERS | {"lastRenderedPageBreak"}
HP = "http://www.hancom.co.kr/hwpml/2011/paragraph"
hp = lambda n: "{%s}%s" % (HP, n)


def primary(flags):
    keys = {f.split(":")[0] for f in flags}
    for c in ORDER:
        if c in keys:
            return c
    return "plain"


# ---------------------------------------------------------------- docx
def docx_flags(tbl):
    flags = set()
    grid = tbl.find(qn("w:tblGrid"))
    gridw = len(grid.findall(qn("w:gridCol"))) if grid is not None else 0
    rows = []  # per row: list of (gc, span, vmerge) and (before, after)

    def row_children(parent):
        """Yield w:tr under tbl, looking through markers; flag wrappers."""
        for ch in parent:
            n = local(ch)
            if n == "tr":
                yield ch
            elif n in ("sdt", "customXml"):
                flags.add("row-cc:%s wraps rows" % n)
                content = ch.find(qn("w:sdtContent")) if n == "sdt" else ch
                if content is not None:
                    yield from row_children(content)
            elif n in MARK or n.startswith("#"):
                flags.add("marker:table-level")
            elif n not in ("tblPr", "tblGrid"):
                flags.add("other:table-level %s" % n)

    def cell_children(tr):
        for ch in tr:
            n = local(ch)
            if n == "tc":
                yield ch
            elif n in ("sdt", "customXml"):
                flags.add("row-cc:%s wraps cells" % n)
                content = ch.find(qn("w:sdtContent")) if n == "sdt" else ch
                if content is not None:
                    yield from cell_children(content)
            elif n in MARK or n.startswith("#"):
                flags.add("marker:row-level")
            elif n not in ("trPr", "tblPrEx"):
                flags.add("other:row-level %s" % n)

    for ri, tr in enumerate(row_children(tbl)):
        trpr = tr.find(qn("w:trPr"))
        gb = trpr.find(qn("w:gridBefore")) if trpr is not None else None
        ga = trpr.find(qn("w:gridAfter")) if trpr is not None else None
        before = int(wval(gb) or 0) if gb is not None else 0
        after = int(wval(ga) or 0) if ga is not None else 0
        if before:
            flags.add("grid:gridBefore")
        if after:
            flags.add("grid:gridAfter")
        cells, gc = [], before
        for tc in cell_children(tr):
            tcpr = tc.find(qn("w:tcPr"))
            gs = tcpr.find(qn("w:gridSpan")) if tcpr is not None else None
            span = int(wval(gs) or 1) if gs is not None else 1
            vm = tcpr.find(qn("w:vMerge")) if tcpr is not None else None
            vstate = None if vm is None else ("restart" if wval(vm) == "restart" else "cont")
            if tcpr is not None and tcpr.find(qn("w:hMerge")) is not None:
                flags.add("other:hMerge")
            if span > 1 or vstate:
                flags.add("merged")
            content = [c for c in tc if local(c) != "tcPr"]
            paras = 0
            for c in content:
                n = local(c)
                if n == "p":
                    paras += 1
                elif n == "tbl":
                    flags.add("nested")
                elif n in MARK or n.startswith("#"):
                    flags.add("marker:cell-level")
                elif n in ("sdt", "customXml"):
                    flags.add("other:block %s in cell" % n)
                    if c.find(".//" + qn("w:tbl")) is not None:
                        flags.add("nested")
                else:
                    flags.add("other:cell holds %s" % n)
            if paras > 1:
                flags.add("multipara")
            if vstate == "cont":
                if ri == 0:
                    flags.add("other:vMerge continue in first row")
                txt = "".join(t.text or "" for t in tc.iter(qn("w:t"))).strip()
                if txt:
                    flags.add("other:text in a covered cell")
            cells.append((gc, span, vstate))
            gc += span
        rows.append((cells, before, after, gc + after))
    if not rows:
        flags.add("other:empty table")
        return flags, 0, 0
    width = max([r[3] for r in rows] + [gridw])
    for cells, before, after, total in rows:
        if total < width:
            flags.add("grid:implicit short row")
        elif total > width:
            flags.add("other:row wider than grid")
    # a vertical merge must stay a rectangle (§5.2): the continuing cell sits
    # under a cell with the same start column and span
    for ri in range(1, len(rows)):
        above = {(gc, span) for gc, span, _ in rows[ri - 1][0]}
        for gc, span, v in rows[ri][0]:
            if v == "cont" and (gc, span) not in above:
                flags.add("other:non-rectangular vertical merge")
    return flags, len(rows), width


def docx_tables(path):
    z = zipfile.ZipFile(path)
    root = parse(z.read(DOC_PART))
    out = []
    for tbl in root.iter(qn("w:tbl")):
        anc = [local(a) for a in tbl.iterancestors()]
        if "tc" in anc:
            continue  # nested: counted with its outer table
        ctx = "textbox" if "txbxContent" in anc else ("block-sdt" if "sdtContent" in anc else "body")
        flags, nrows, ncols = docx_flags(tbl)
        inner = sum(1 for t in tbl.iter(qn("w:tbl")) if t is not tbl)
        proto = Importer.table_reason(None, tbl) or ""
        out.append(dict(context=ctx, rows=nrows, cols=ncols, flags=sorted(flags), inner=inner, proto=proto))
    return out


# ---------------------------------------------------------------- hwpx
def hwpx_flags(tbl):
    flags = set()
    R, C = int(tbl.get("rowCnt") or 0), int(tbl.get("colCnt") or 0)
    occ = collections.Counter()
    fields = 0
    for tr in tbl.findall(hp("tr")):
        for tc in tr.findall(hp("tc")):
            a, s = tc.find(hp("cellAddr")), tc.find(hp("cellSpan"))
            c0, r0 = int(a.get("colAddr")), int(a.get("rowAddr"))
            cs, rs = int(s.get("colSpan")), int(s.get("rowSpan"))
            if cs > 1 or rs > 1:
                flags.add("merged")
            for r in range(r0, r0 + rs):
                for c in range(c0, c0 + cs):
                    occ[(r, c)] += 1
            sub = tc.find(hp("subList"))
            paras = sub.findall(hp("p")) if sub is not None else []
            if len(paras) > 1:
                flags.add("multipara")
            if tc.find(".//" + hp("tbl")) is not None:
                flags.add("nested")
            fields += sum(1 for f in tc.iter(hp("fieldBegin")))
    if R == 0 or C == 0:
        flags.add("other:empty table")
    holes = [(r, c) for r in range(R) for c in range(C) if occ[(r, c)] == 0]
    if holes:
        flags.add("grid:uncovered grid slots")
    if any(v > 1 for v in occ.values()):
        flags.add("other:overlapping cells")
    if any(r >= R or c >= C for r, c in occ):
        flags.add("other:cell outside rowCnt/colCnt")
    if fields:
        flags.add("info:field in cell")
    # the paragraph that anchors the table also holds text of its own
    p = next((a for a in tbl.iterancestors(hp("p"))), None)
    if p is not None:
        own = [t.text or "" for run in p.findall(hp("run")) for t in run.findall(hp("t"))]
        if "".join(own).strip():
            flags.add("info:anchor paragraph has text")
        if sum(1 for run in p.findall(hp("run")) for _ in run.findall(hp("tbl"))) > 1:
            flags.add("info:anchor paragraph holds another table")
    return flags, R, C


def hwpx_tables(path):
    z = zipfile.ZipFile(path)
    out = []
    names = sorted((n for n in z.namelist() if re.match(r"Contents/section\d+\.xml$", n)),
                   key=lambda n: int(re.findall(r"\d+", n)[0]))
    for n in names:
        root = etree.fromstring(z.read(n), etree.XMLParser(huge_tree=True, resolve_entities=False))
        for tbl in root.iter(hp("tbl")):
            anc = [a.tag for a in tbl.iterancestors()]
            if hp("tc") in anc:
                continue
            ctx = "textbox" if hp("drawText") in anc else "body"
            flags, R, C = hwpx_flags(tbl)
            inner = sum(1 for t in tbl.iter(hp("tbl")) if t is not tbl)
            out.append(dict(context=ctx, rows=R, cols=C, flags=sorted(flags), inner=inner, proto=""))
    return out


# ---------------------------------------------------------------- main
def main():
    corpus = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "corpus")
    manifest = {r["id"]: r for r in csv.DictReader(open(os.path.join(HERE, "manifest.csv")))}
    tables, docs = [], []
    for doc_id, m in manifest.items():
        path = os.path.join(corpus, m["format"], doc_id + "." + m["format"])
        ts = docx_tables(path) if m["format"] == "docx" else hwpx_tables(path)
        cnt = collections.Counter()
        for i, t in enumerate(ts):
            t.update(doc=doc_id, format=m["format"], lang=m["lang"], n=i, category=primary(t["flags"]))
            cnt[t["category"]] += 1
            tables.append(t)
        docs.append(dict(doc=doc_id, format=m["format"], lang=m["lang"], tables=len(ts),
                         inner=sum(t["inner"] for t in ts), **{c: cnt[c] for c in ORDER}))
    os.makedirs(os.path.join(HERE, "results"), exist_ok=True)
    with open(os.path.join(HERE, "results/tables.csv"), "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["doc", "format", "lang", "n", "context", "rows", "cols", "category", "flags", "inner_tables",
                    "proto_reason"])
        for t in tables:
            w.writerow([t["doc"], t["format"], t["lang"], t["n"], t["context"], t["rows"], t["cols"],
                        t["category"], ";".join(t["flags"]), t["inner"], t["proto"]])
    with open(os.path.join(HERE, "results/docs.csv"), "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(docs[0]))
        w.writeheader()
        w.writerows(docs)
    open(os.path.join(HERE, "results/summary.md"), "w").write(summary(tables, docs))
    print(open(os.path.join(HERE, "results/summary.md")).read())


def pct(a, b):
    return "%d (%.1f%%)" % (a, 100.0 * a / b) if b else "0"


def summary(tables, docs):
    groups = [("Korean hwpx", "hwpx", "ko"), ("English docx", "docx", "en")]
    out = ["# Table-shape survey: generated summary", ""]
    out.append("Per top-level table (primary category; nested tables counted with their outer table):")
    out.append("")
    out.append("| Category | " + " | ".join(g[0] for g in groups) + " | All |")
    out.append("|---|" + "---|" * (len(groups) + 1))
    sel = [[t for t in tables if (t["format"], t["lang"]) == g[1:]] for g in groups] + [tables]
    for c in ORDER:
        out.append("| %s | " % c + " | ".join(pct(sum(t["category"] == c for t in s), len(s)) for s in sel) + " |")
    out.append("| **tables** | " + " | ".join(str(len(s)) for s in sel) + " |")
    out.append("")
    out.append("Per document (a document counts once for every category it has at least one table of):")
    out.append("")
    dsel = [[d for d in docs if (d["format"], d["lang"]) == g[1:]] for g in groups] + [docs]
    out.append("| Documents with | " + " | ".join(g[0] for g in groups) + " | All |")
    out.append("|---|" + "---|" * (len(groups) + 1))
    out.append("| any table | " + " | ".join(pct(sum(d["tables"] > 0 for d in s), len(s)) for s in dsel) + " |")
    for c in ORDER:
        out.append("| %s | " % c + " | ".join(pct(sum(d[c] > 0 for d in s), len(s)) for s in dsel) + " |")
    out.append("| an unsupported shape (nested, row-cc, grid, other) | " + " | ".join(
        pct(sum(any(d[c] for c in ORDER[:4]) for d in s), len(s)) for s in dsel) + " |")
    out.append("| an unsupported shape, among documents with tables | " + " | ".join(
        pct(sum(any(d[c] for c in ORDER[:4]) for d in s), sum(d["tables"] > 0 for d in s)) for s in dsel) + " |")
    out.append("| **documents** | " + " | ".join(str(len(s)) for s in dsel) + " |")
    out.append("")
    out.append("Every flag (a table can have several):")
    out.append("")
    fc = [collections.Counter(f for t in s for f in t["flags"]) for s in sel]
    out.append("| Flag | " + " | ".join(g[0] for g in groups) + " | All |")
    out.append("|---|" + "---|" * (len(groups) + 1))
    for f in sorted(set(fc[-1])):
        out.append("| %s | " % f + " | ".join(str(c[f]) for c in fc) + " |")
    out.append("")
    out.append("Contexts: " + ", ".join("%s %d" % kv for kv in collections.Counter(
        (t["format"], t["context"]) for t in tables).items()))
    out.append("Inner (nested) tables: " + ", ".join(
        "%s %d" % (g[0], sum(t["inner"] for t in s)) for g, s in zip(groups, sel)))
    dx = [t for t in tables if t["format"] == "docx"]
    out.append("Prototype classifier (docx): %d of %d tables agree on pipe-table-or-not under the "
               "prototype's rules (multi-paragraph counted as not; markers as not)." % (
                   sum(_proto_agrees(t) for t in dx), len(dx)))
    out.append("Prototype first reasons (docx): " + ", ".join(
        "%s %d" % kv for kv in collections.Counter(t["proto"] or "(pipe table)" for t in dx).most_common()))
    out.append("")
    return "\n".join(out)


def _proto_agrees(t):
    """The prototype refuses multi-paragraph cells and markers in table structure; this scan
    does not. Map this scan back to the prototype's rules and compare."""
    keys = {f.split(":")[0] for f in t["flags"]}
    refused_here = bool(keys & {"nested", "row-cc", "grid", "other", "multipara", "marker"})
    return refused_here == bool(t["proto"])


if __name__ == "__main__":
    main()
