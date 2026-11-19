#!/usr/bin/env python3
"""Measure the spike's PDFs: text, fonts, tags, outline, links, PDF/A and
visual match with Hanji's preview PNGs.

usage: analyze.py <out-dir> [<verapdf-lib-dir>]
Writes <out-dir>/analysis.json and <out-dir>/compare/*.png.
"""
import glob
import json
import os
import re
import subprocess
import sys
import unicodedata

import numpy as np
from PIL import Image, ImageFilter
from pypdf import PdfReader

OUT = sys.argv[1]
VERA = sys.argv[2] if len(sys.argv) > 2 else None
DPI = 96


def run(*args):
    return subprocess.run(args, capture_output=True, text=True).stdout


def fonts(pdf):
    rows = run("pdffonts", pdf).splitlines()[2:]
    out = []
    for r in rows:
        parts = r.split()
        # name type... emb sub uni object ID
        out.append({"name": parts[0], "emb": parts[-5], "sub": parts[-4], "uni": parts[-3]})
    return out


def text(pdf):
    return run("pdftotext", "-layout", pdf, "-")


def norm(t):
    t = unicodedata.normalize("NFC", t)
    return re.sub(r"\s+", "", t)


def hangul(t):
    return sum(1 for c in t if "가" <= c <= "힣")


def outline_count(reader):
    def walk(o):
        n = 0
        for x in o:
            n += walk(x) if isinstance(x, list) else 1
        return n
    try:
        return walk(reader.outline)
    except Exception:
        return 0


def links(reader):
    n = 0
    for p in reader.pages:
        for a in p.get("/Annots") or []:
            a = a.get_object()
            if a.get("/Subtype") == "/Link":
                n += 1
    return n


def tagged(reader):
    root = reader.trailer["/Root"]
    mark = root.get("/MarkInfo")
    return bool(root.get("/StructTreeRoot")) and bool(mark and mark.get("/Marked"))


def verapdf(pdf, flavour):
    if not VERA:
        return None
    cp = ":".join(glob.glob(os.path.join(VERA, "*.jar")))
    r = subprocess.run(
        ["java", "-cp", cp, "org.verapdf.apps.GreenfieldCliWrapper", "--flavour", flavour,
         "--format", "json", pdf],
        capture_output=True, text=True)
    try:
        j = json.loads(r.stdout)
        job = j["report"]["jobs"][0]
        v = job["validationResult"][0] if isinstance(job["validationResult"], list) else job["validationResult"]
        d = v["details"]
        rules = sorted({f'{x["specification"].split()[-1] if False else ""}{x["clause"]}-{x["testNumber"]}: {x["description"][:90]}'
                        for x in d.get("ruleSummaries", [])})
        return {"compliant": v["compliant"], "failed_rules": d["failedRules"],
                "failed_checks": d["failedChecks"], "rules": rules[:12]}
    except Exception as e:  # noqa: BLE001
        return {"error": (r.stderr or r.stdout)[-400:]}


def raster(pdf, prefix):
    subprocess.run(["pdftoppm", "-r", str(DPI), "-png", pdf, prefix], check=True)
    return sorted(glob.glob(prefix + "*.png"))


def diff(a_path, b_path):
    # A light blur first, so anti-aliasing differences between resvg and
    # poppler do not count; what is left is placement and missing ink.
    a = Image.open(a_path).convert("L").filter(ImageFilter.GaussianBlur(1.2))
    b = Image.open(b_path).convert("L").filter(ImageFilter.GaussianBlur(1.2))
    if a.size != b.size:
        b = b.resize(a.size)
    a = np.asarray(a, dtype=np.int16)
    b = np.asarray(b, dtype=np.int16)
    d = np.abs(a - b)
    return {
        "mean_abs": round(float(d.mean()), 2),
        "pct_px_gt40": round(float((d > 40).mean() * 100), 3),
        # Of the pixels either side inks, the share that differ strongly.

    }


def main():
    result = {}
    os.makedirs(os.path.join(OUT, "compare"), exist_ok=True)
    for report in sorted(glob.glob(os.path.join(OUT, "*.report.json"))):
        stem = os.path.basename(report)[: -len(".report.json")]
        rep = json.load(open(report))
        entry = {"pages": rep["pages"], "fonts_summary": rep["fonts_summary"]}
        texts = {}
        for variant in ["krilla", "krilla-pdfa2b", "krilla-pdfa2a", "krilla-pdfua1", "svg2pdf"]:
            pdf = os.path.join(OUT, f"{stem}.{variant}.pdf")
            if not os.path.exists(pdf):
                entry[variant] = {"written": False, "error": rep.get(variant, {}).get("error", "")[:600]}
                continue
            reader = PdfReader(pdf)
            t = text(pdf)
            texts[variant] = t
            fs = fonts(pdf)
            e = {
                "written": True,
                "bytes": os.path.getsize(pdf),
                "pages": len(reader.pages),
                "version": reader.pdf_header,
                "fonts": len(fs),
                "fonts_not_embedded": sum(1 for f in fs if f["emb"] != "yes"),
                "fonts_without_tounicode": sum(1 for f in fs if f["uni"] != "yes"),
                "type3_fonts": sum(1 for f in fs if f["name"] == "[none]"),
                "text_chars": len(norm(t)),
                "hangul_chars": hangul(t),
                "tagged": tagged(reader),
                "outline_entries": outline_count(reader),
                "links": links(reader),
                "ms": rep.get(variant, {}).get("ms"),
            }
            if variant == "krilla-pdfa2b":
                e["verapdf_2b"] = verapdf(pdf, "2b")
            if variant == "krilla-pdfa2a":
                e["verapdf_2a"] = verapdf(pdf, "2a")
            if variant == "krilla-pdfua1":
                e["verapdf_ua1"] = verapdf(pdf, "ua1")
            if variant == "krilla":
                e["verapdf_2b_plain"] = verapdf(pdf, "2b")
            if variant == "svg2pdf":
                e["verapdf_2b_plain"] = verapdf(pdf, "2b")
            entry[variant] = e

        # Same text in both routes?
        if "krilla" in texts and "svg2pdf" in texts:
            a, b = norm(texts["krilla"]), norm(texts["svg2pdf"])
            import difflib
            entry["text_similarity_krilla_vs_svg2pdf"] = round(
                difflib.SequenceMatcher(None, a[:20000], b[:20000], autojunk=False).ratio(), 4)
        with open(os.path.join(OUT, "compare", f"{stem}.krilla.txt"), "w") as f:
            f.write(texts.get("krilla", ""))
        with open(os.path.join(OUT, "compare", f"{stem}.svg2pdf.txt"), "w") as f:
            f.write(texts.get("svg2pdf", ""))

        # Visual: each PDF rasterised at the preview's DPI against the
        # preview PNG.
        previews = sorted(glob.glob(os.path.join(OUT, f"{stem}.svgpages", "p*.preview.png")))
        tmp = os.path.join(OUT, "compare", "tmp")
        os.makedirs(tmp, exist_ok=True)
        visual = {}
        for variant in ["krilla", "svg2pdf"]:
            pdf = os.path.join(OUT, f"{stem}.{variant}.pdf")
            if not os.path.exists(pdf):
                continue
            for old in glob.glob(os.path.join(tmp, f"{stem}.{variant}*")):
                os.remove(old)
            pages = raster(pdf, os.path.join(tmp, f"{stem}.{variant}"))
            per = [diff(p, q) for p, q in zip(previews, pages)]
            visual[variant] = {
                "pages_compared": len(per),
                "worst_pct_px_gt40": max((p["pct_px_gt40"] for p in per), default=None),
                "mean_pct_px_gt40": round(sum(p["pct_px_gt40"] for p in per) / len(per), 3) if per else None,
                "worst_mean_abs": max((p["mean_abs"] for p in per), default=None),
            }
            # Side by side for the first two pages: preview | krilla | svg2pdf.
            for i, (p, q) in enumerate(list(zip(previews, pages))[:2]):
                visual.setdefault("_pairs", {}).setdefault(i, []).append((variant, p, q))
        for i, items in visual.pop("_pairs", {}).items():
            imgs = [Image.open(items[0][1]).convert("RGB")] + [Image.open(q).convert("RGB") for _, _, q in items]
            h = max(im.height for im in imgs)
            w = sum(im.width for im in imgs) + 10 * (len(imgs) - 1)
            sheet = Image.new("RGB", (w, h), "#888")
            x = 0
            for im in imgs:
                sheet.paste(im, (x, 0))
                x += im.width + 10
            sheet.save(os.path.join(OUT, "compare", f"{stem}.p{i+1}.preview-krilla-svg2pdf.png"))
        entry["visual_vs_preview"] = visual
        result[stem] = entry
    json.dump(result, open(os.path.join(OUT, "analysis.json"), "w"), indent=2, ensure_ascii=False)
    print(json.dumps(result, indent=2, ensure_ascii=False))


main()
