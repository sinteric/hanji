#!/usr/bin/env python3
"""Run every measurement on the corpus and write results/.

  python3 run_all.py            # GetPut, edits x designs, PutGet
  python3 run_all.py --soffice  # also convert every exported file to PDF with LibreOffice
"""
import glob
import re
import json
import os
import shutil
import subprocess
import sys
import time
from types import SimpleNamespace
from collections import Counter

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from hanji_rem import model as M  # noqa: E402
from hanji_rem.anchoring import place, placed_entries  # noqa: E402
from hanji_rem.check import score, xml_diff  # noqa: E402
from hanji_rem.edits import ALL  # noqa: E402
from hanji_rem.compose import combined  # noqa: E402
from hanji_rem.exporter import export_xml  # noqa: E402
from hanji_rem.importer import import_docx  # noqa: E402
from hanji_rem.ooxml import DOC_PART, canon, parse, part, read_docx, write_docx  # noqa: E402

DESIGNS = ["A", "A-strict", "B", "C"]
RES = os.path.join(HERE, "results")
OUT = os.path.join(RES, "out")


def remainder_jsonl(doc):
    lines = []
    for e in doc.entries:
        lines.append(json.dumps({
            "id": e.id, "kind": e.kind,
            "anchor": {"block": list(e.path), "start": e.start, "end": e.end, "seq": e.seq},
            "xml": "".join(etree.tostring(x, encoding="unicode") for x in e.payload),
        }, ensure_ascii=False))
    return "\n".join(lines) + "\n"


def roundtrip(doc, text, design, bmap=None, true_ops=None):
    blocks = M.parse(text, doc.keeps, doc.styles)
    outcomes = place(doc, blocks, design, bmap, true_ops)
    xml, issues = export_xml(doc, blocks, placed_entries(outcomes))
    return blocks, outcomes, xml, issues


def main(soffice=False):
    shutil.rmtree(OUT, ignore_errors=True)
    for d in ("model", "remainder"):
        os.makedirs(os.path.join(RES, d), exist_ok=True)
    os.makedirs(OUT, exist_ok=True)
    files = sorted(glob.glob(os.path.join(HERE, "corpus", "*.docx")))
    report = {"files": {}, "totals": {d: Counter() for d in DESIGNS}, "per_edit": {}}
    for path in files:
        name = os.path.basename(path)
        stem = name[:-5]
        t0 = time.time()
        parts = read_docx(path)
        orig = part(parts, DOC_PART)
        doc = import_docx(parts)
        text = M.serialize(doc.blocks, doc.keeps, doc.styles, name)
        open(os.path.join(RES, "model", stem + ".md"), "w").write(text)
        rem = remainder_jsonl(doc)
        open(os.path.join(RES, "remainder", stem + ".jsonl"), "w").write(rem)
        kinds = Counter(e.kind for e in doc.entries)
        fr = {"entries": dict(Counter(e.kind for e in doc.entries if e.fp)), "entries_all": dict(kinds), "counted_entries": sum(1 for e in doc.entries if e.fp),
              "keeps_inline": sum(1 for k in doc.keeps.values() if not k.block),
              "keeps_block": sum(1 for k in doc.keeps.values() if k.block),
              "keep_kinds": dict(Counter(k.kind for k in doc.keeps.values())),
              "tables_modelled": doc.stats["tables_modelled"], "tables_kept": doc.stats["tables_kept"],
              "kept_reasons": doc.stats["kept_reasons"],
              "document_xml_bytes": len(orig), "model_text_bytes": len(text.encode()),
              "remainder_jsonl_bytes": len(rem.encode()), "blocks": len(doc.blocks),
              "remainder_xml_bytes": sum(len(etree.tostring(x)) for e in doc.entries for x in e.payload),
              "remainder_xml_bytes_no_xmlns": sum(len(re.sub(rb' xmlns(:\w+)?="[^"]*"', b"", etree.tostring(x)))
                                                  for e in doc.entries for x in e.payload)}
        # model text round trip
        again = M.serialize(M.parse(text, doc.keeps, doc.styles), doc.keeps, doc.styles, name)
        fr["text_reparse_equal"] = again == text
        # GetPut (no edit) through the whole path: text -> parse -> place -> export
        _, _, xml, issues = roundtrip(doc, text, "B")
        same = canon(orig) == canon(xml)
        fr["getput"] = {"document.xml": "equal" if same else "differs",
                        "why": [] if same else xml_diff(orig, xml),
                        "other_parts": "copied byte-for-byte (%d parts)" % (len(parts) - 1),
                        "issues": issues}
        gp_path = os.path.join(OUT, stem, "getput.docx")
        os.makedirs(os.path.dirname(gp_path), exist_ok=True)
        write_docx(gp_path, parts, {DOC_PART: xml})
        # edits
        fr["edits"] = []
        jobs = []
        for fn in ALL:
            ed = fn(doc)
            if ed is None:
                fr["edits"].append({"edit": fn.__name__, "status": "n/a (no target in this file)"})
                continue
            new_text = M.serialize(ed.blocks, doc.keeps, doc.styles, name)
            written = M.parse(new_text, doc.keeps, doc.styles)
            expected = place(doc, written, "B", ed.bmap, ed.true_ops, ed.xmap, ed.lenient)
            jobs.append((ed.name, ed.what, new_text, expected, ed.touched))
        fblocks, fexp, steps = combined(doc)
        jobs.append(("E10 all edits in one revision", "; ".join(steps), M.serialize(fblocks, doc.keeps, doc.styles, name),
                     fexp, set(range(len(doc.blocks)))))
        for ename, what, new_text, expected, touched in jobs:
            ed = SimpleNamespace(name=ename, what=what, touched=touched)
            row = {"edit": ed.name, "what": ed.what, "designs": {}}
            for design in DESIGNS:
                blocks, got, xml, issues = roundtrip(doc, new_text, design)
                fn_out = os.path.join(OUT, stem, "%s-%s.docx" % (ed.name.split()[0], design))
                write_docx(fn_out, parts, {DOC_PART: xml})
                try:
                    parse(xml)
                    wf = True
                except etree.XMLSyntaxError:
                    wf = False
                re_doc = import_docx(read_docx(fn_out))
                re_text = M.serialize(re_doc.blocks, re_doc.keeps, re_doc.styles, name)
                putget = M.normalise_ids(re_text) == M.normalise_ids(new_text)
                tally, examples = score(doc, expected, got, re_doc, ed.touched)
                orphans = [o.reason for o in got.values() if o.status == "orphan"]
                row["designs"][design] = {
                    "landed": tally["landed"], "orphaned": tally["orphaned"],
                    "lost": sum(v for k, v in tally.items() if k.startswith("lost")),
                    "lost_detail": {k: v for k, v in tally.items() if k.startswith("lost")},
                    "removed_with_text": tally["removed-with-text"],
                    "touched": {k: tally["touched:" + k] for k in ("landed", "orphaned", "lost")},
                    "by_kind": {k[5:]: v for k, v in tally.items() if k.startswith("kind:") and not k.endswith(":landed")},
                    "examples": examples, "orphan_reasons": dict(Counter(orphans)),
                    "putget": putget, "well_formed": wf, "export_issues": issues,
                }
                t = report["totals"][design]
                for k in ("landed", "orphaned", "lost"):
                    t[k] += row["designs"][design][k]
                for k in ("landed", "orphaned", "lost"):
                    t["touched_" + k] += tally["touched:" + k]
                t["putget_ok"] += putget
                t["runs"] += 1
                pe = report["per_edit"].setdefault(ed.name, {d: Counter() for d in DESIGNS})
                for k in ("landed", "orphaned", "lost"):
                    pe[design][k] += row["designs"][design][k]
                    pe[design]["touched_" + k] += tally["touched:" + k]
            fr["edits"].append(row)
        fr["seconds"] = round(time.time() - t0, 2)
        report["files"][name] = fr
        gp = fr["getput"]["document.xml"]
        print("%-32s blocks=%4d entries=%5d getput=%s  %s" % (name, len(doc.blocks), len(doc.entries), gp,
              " ".join("%s:%d/%d/%d" % (d, report["totals"][d]["landed"], report["totals"][d]["orphaned"],
                                        report["totals"][d]["lost"]) for d in DESIGNS)))
    if soffice:
        report["soffice"] = run_soffice()
    report["totals"] = {d: dict(c) for d, c in report["totals"].items()}
    report["per_edit"] = {k: {d: dict(c) for d, c in v.items()} for k, v in report["per_edit"].items()}
    json.dump(report, open(os.path.join(RES, "results.json"), "w"), indent=1, ensure_ascii=False, default=str)
    write_tables(report)


def pdf_pages(path):
    data = open(path, "rb").read()
    return len(re.findall(rb"/Type\s*/Page[^s]", data))


def run_soffice():
    """Convert the originals and every exported file to PDF (one LibreOffice call per
    source file); record failures and page counts against the original."""
    work = os.path.join(RES, "pdf-work")
    shutil.rmtree(work, ignore_errors=True)
    res = {"converted": 0, "failed": [], "page_count_differs": [], "total": 0, "version": ""}
    try:
        res["version"] = subprocess.run(["soffice", "--version"], capture_output=True, text=True).stdout.strip()
    except OSError as e:
        res["error"] = str(e)
        return res
    for d in sorted(glob.glob(os.path.join(OUT, "*"))):
        stem = os.path.basename(d)
        os.makedirs(work)
        shutil.copy(os.path.join(HERE, "corpus", stem + ".docx"), os.path.join(work, "ORIGINAL.docx"))
        for f in glob.glob(os.path.join(d, "*.docx")):
            shutil.copy(f, work)
        files = sorted(glob.glob(os.path.join(work, "*.docx")))
        subprocess.run(["soffice", "--headless", "--convert-to", "pdf", "--outdir", work] + files,
                       capture_output=True, text=True, timeout=1800)
        orig_pdf = os.path.join(work, "ORIGINAL.pdf")
        base = pdf_pages(orig_pdf) if os.path.exists(orig_pdf) else None
        for f in files:
            name = os.path.basename(f)[:-5]
            if name == "ORIGINAL":
                continue
            res["total"] += 1
            pdf = os.path.join(work, name + ".pdf")
            if os.path.exists(pdf) and os.path.getsize(pdf) > 0:
                res["converted"] += 1
                n = pdf_pages(pdf)
                if base is not None and name == "getput" and n != base:
                    res["page_count_differs"].append("%s/%s: %d pages vs original %d" % (stem, name, n, base))
            else:
                res["failed"].append({"file": "%s/%s" % (stem, name)})
        res.setdefault("original_pages", {})[stem] = base
        shutil.rmtree(work)
    return res


def write_tables(r):
    L = ["# Results (generated by run_all.py)", ""]
    L.append("## Totals per design (all files, all edits)")
    L.append("")
    L.append("| Design | landed | orphaned (refused, with reason) | lost (silent) | PutGet ok |")
    L.append("|---|---|---|---|---|")
    for d, t in r["totals"].items():
        tot = t.get("landed", 0) + t.get("orphaned", 0) + t.get("lost", 0)
        L.append("| %s | %d (%.1f%%) | %d (%.1f%%) | %d (%.1f%%) | %d/%d |" % (
            d, t.get("landed", 0), 100 * t.get("landed", 0) / tot, t.get("orphaned", 0),
            100 * t.get("orphaned", 0) / tot, t.get("lost", 0), 100 * t.get("lost", 0) / tot,
            t.get("putget_ok", 0), t.get("runs", 0)))
    L += ["", "## Totals per design, entries in the blocks each edit touches", "",
          "| Design | landed | orphaned | lost |", "|---|---|---|---|"]
    for d, t in r["totals"].items():
        tot = sum(t.get("touched_" + k, 0) for k in ("landed", "orphaned", "lost")) or 1
        L.append("| %s | " % d + " | ".join("%d (%.1f%%)" % (t.get("touched_" + k, 0), 100 * t.get("touched_" + k, 0) / tot)
                                         for k in ("landed", "orphaned", "lost")) + " |")
    for pre, title in (("", "all entries"), ("touched_", "entries in touched blocks")):
        L += ["", "## Per edit, %s (landed / orphaned / lost)" % title, "", "| Edit | " + " | ".join(r["totals"]) + " |",
              "|---|" + "---|" * len(r["totals"])]
        for ed, v in sorted(r["per_edit"].items(), key=lambda kv: int(kv[0].split()[0][1:])):
            L.append("| %s | " % ed + " | ".join("%d / %d / %d" % (v[d].get(pre + "landed", 0), v[d].get(pre + "orphaned", 0),
                                                                  v[d].get(pre + "lost", 0)) for d in r["totals"]) + " |")
    kinds = Counter()
    lost = {d: Counter() for d in r["totals"]}
    orph = {d: Counter() for d in r["totals"]}
    wf = Counter()
    for x in r["files"].values():
        for k, v in x["entries"].items():
            kinds[k] += v
        for row in x["edits"]:
            for d, v in row.get("designs", {}).items():
                wf[d] += v["well_formed"]
                for k, n in v["by_kind"].items():
                    kk, res = k.rsplit(":", 1)
                    (lost if res == "lost" else orph)[d][kk] += n
    L += ["", "## Where the failures are, by entry kind (lost / orphaned, all edits)", "",
          "| Entry kind | entries in corpus | " + " | ".join(r["totals"]) + " |", "|---|---|" + "---|" * len(r["totals"])]
    for k in sorted(kinds, key=lambda k: -kinds[k]):
        L.append("| %s | %d | " % (k, kinds[k]) + " | ".join("%d / %d" % (lost[d][k], orph[d][k]) for d in r["totals"]) + " |")
    L += ["", "Exported document.xml well-formed: " + ", ".join("%s %d/%d" % (d, wf[d], r["totals"][d].get("runs", 0)) for d in r["totals"])]
    L += ["", "## Per file", "",
          "| File | blocks | entries (counted) | keeps inline/block | tables modelled/kept | document.xml → model text + remainder XML (without per-fragment xmlns) bytes | GetPut document.xml |",
          "|---|---|---|---|---|---|---|"]
    for f, x in r["files"].items():
        L.append("| %s | %d | %d | %d / %d | %d / %d | %d → %d + %d (%d) | %s |" % (
            f, x["blocks"], x["counted_entries"], x["keeps_inline"], x["keeps_block"], x["tables_modelled"],
            x["tables_kept"], x["document_xml_bytes"], x["model_text_bytes"], x["remainder_xml_bytes"],
            x["remainder_xml_bytes_no_xmlns"], x["getput"]["document.xml"] + ("" if not x["getput"]["why"] else ": " + "; ".join(x["getput"]["why"])[:160])))
    L += ["", "## Per file and edit (landed / orphaned / lost; PutGet)", ""]
    for f, x in r["files"].items():
        L.append("### %s" % f)
        L.append("")
        L.append("| Edit | target | " + " | ".join(r["totals"]) + " |")
        L.append("|---|---|" + "---|" * len(r["totals"]))
        for row in x["edits"]:
            if "designs" not in row:
                L.append("| %s | %s |" % (row["edit"], row["status"]) + " |" * len(r["totals"]))
                continue
            L.append("| %s | %s | " % (row["edit"], row["what"].replace("|", "\\|")[:90]) + " | ".join(
                "%d / %d / %d (touched %d / %d / %d)%s" % (v["landed"], v["orphaned"], v["lost"], v["touched"]["landed"],
                v["touched"]["orphaned"], v["touched"]["lost"], "" if v["putget"] else " PutGet✗")
                for v in row["designs"].values()) + " |")
        L.append("")
    if "soffice" in r:
        s = r["soffice"]
        L += ["## LibreOffice (%s)" % s.get("version", ""), "",
              "%d of %d exported files converted to PDF (headless, one call per source file)." % (s["converted"], s["total"]),
              "GetPut exports whose page count differs from the original's: %d." % len(s["page_count_differs"]), ""]
        for f in s["failed"]:
            L.append("- failed: %s" % f["file"])
        for f in s["page_count_differs"]:
            L.append("- pages: %s" % f)
    open(os.path.join(RES, "RESULTS.md"), "w").write("\n".join(L) + "\n")


if __name__ == "__main__":
    main(soffice="--soffice" in sys.argv)
