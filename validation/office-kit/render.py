#!/usr/bin/env python3
"""Before/after PNGs of the Office check kit's edited slides (run by the kit, see main.rs).

    python3 render.py MANIFEST KIT_DIR     # needs soffice (LibreOffice), pymupdf and pillow

MANIFEST lists each pptx `pset` file with its original, the model text before and after the
edits, and which original slide each slide of the file is. For every slide of the file whose
text changed (an object's text, box, fill, …, the layout, the notes, an object added, removed
or moved in the z-order, or a new slide), this writes `NN-…-pset-slideK.png`: the original
slide (left) and the file's slide (right), as LibreOffice renders them, with the boxes of the
objects that changed outlined in red. It adds a section listing them to the file's -edits.md.
Without LibreOffice or PyMuPDF it writes nothing and says so.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

WIDTH = 560  # px per slide image; the PNG is two of them side by side
RED = (220, 30, 30)


def slides(text):
    """The model text's slides as lists of lines, and the slide size in pt."""
    lines = text.split("\n")
    assert lines[0] == "---", "no front matter"
    end = lines.index("---", 1)
    m = re.search(r"^size: ([\d.]+) x ([\d.]+) pt$", "\n".join(lines[1:end]), re.M)
    size = (float(m.group(1)), float(m.group(2))) if m else (720.0, 540.0)
    out, cur = [], []
    for ln in lines[end + 1:]:
        if ln == "---":
            out.append(cur)
            cur = []
        else:
            cur.append(ln)
    out.append(cur)
    return out, size


def chunks(lines):
    """A slide's objects: key -> (text, box or None), in z-order."""
    out, order = {}, []
    key = None
    for ln in lines:
        s = ln.strip()
        head = None
        if s.startswith("layout:"):
            head = "layout"
        elif s.startswith("::") and s.endswith("::") and len(s) > 4:
            head = "slot:" + re.match(r"::([^\s:]+)", s).group(1)
        elif s.startswith("<") and not s.startswith("</"):
            m = re.search(r'\bid="([^"]+)"', s)
            head = "id:" + m.group(1) if m else "obj:" + s
        if head is not None:
            k, n = head, 2
            while k in out:
                k, n = f"{head}#{n}", n + 1
            key = k
            b = re.search(r'\bbox="([-\d. ]+)"', s)
            box = tuple(float(v) for v in b.group(1).split()) if b else None
            if head == "layout":
                label = "the layout"
            elif head.startswith("slot:"):
                label = f"the {head[5:]} slot"
            else:
                tag = re.match(r"<(\w+)", s)
                nm = re.search(r'\bname="([^"]*)"', s)
                kind = re.search(r'\bkind="([^"]*)"', s)
                label = (tag.group(1) if tag else "object") + (f' "{nm.group(1)}"' if nm else "")
                if tag and tag.group(1) == "keep":
                    label = f"the {kind.group(1) if kind else 'object'} (kept as it is)"
            out[key] = [ln, box if box and len(box) == 4 else None, label]
            order.append(key)
        elif key is not None:
            out[key][0] += "\n" + ln
    return out, order


def lcs(a, b):
    """Keys of a (and b) kept in the same relative order: a longest common subsequence."""
    n, m = len(a), len(b)
    t = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n - 1, -1, -1):
        for j in range(m - 1, -1, -1):
            t[i][j] = t[i + 1][j + 1] + 1 if a[i] == b[j] else max(t[i + 1][j], t[i][j + 1])
    i = j = 0
    keep = set()
    while i < n and j < m:
        if a[i] == b[j]:
            keep.add(a[i])
            i, j = i + 1, j + 1
        elif t[i + 1][j] >= t[i][j + 1]:
            i += 1
        else:
            j += 1
    return keep


def changes(before, after):
    """(changed at all, boxes to outline before, boxes to outline after, what changed)."""
    if before is None:
        a, order = chunks(after)
        return True, [], [a[k][1] for k in order if a[k][1]], ["a new slide"]
    b, bo = chunks(before)
    a, ao = chunks(after)
    common = [k for k in bo if k in a]
    kept = lcs(common, [k for k in ao if k in b])
    ob, oa, what = [], [], []
    for k in bo:
        if k not in a:
            what.append("removed " + b[k][2])
            if b[k][1]:
                ob.append(b[k][1])
    for k in ao:
        if k not in b:
            what.append("added " + a[k][2])
            if a[k][1]:
                oa.append(a[k][1])
        elif a[k][0] != b[k][0] or k not in kept:
            what.append(("moved in the z-order: " if a[k][0] == b[k][0] else "changed ") + a[k][2])
            if b[k][1]:
                ob.append(b[k][1])
            if a[k][1]:
                oa.append(a[k][1])
    return bool(what), ob, oa, what


def to_pdf(paths, out):
    """Every deck to PDF in one LibreOffice run (hidden slides included, so pages are slides)."""
    filt = 'pdf:impress_pdf_Export:{"ExportHiddenSlides":{"type":"boolean","value":"true"}}'
    prof = tempfile.mkdtemp(prefix="lo-profile-")
    try:
        subprocess.run(
            ["soffice", f"-env:UserInstallation=file://{prof}", "--headless", "--convert-to", filt, "--outdir", out, *paths],
            check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=600,
        )
    finally:
        shutil.rmtree(prof, ignore_errors=True)


def page_image(doc, n, size, boxes, dashed):
    from PIL import Image, ImageDraw

    page = doc[n]
    zoom = WIDTH / page.rect.width
    pix = page.get_pixmap(matrix=__import__("pymupdf").Matrix(zoom, zoom), alpha=False)
    im = Image.frombytes("RGB", (pix.width, pix.height), pix.samples)
    d = ImageDraw.Draw(im)
    sx, sy = im.width / size[0], im.height / size[1]
    for x, y, w, h in boxes:
        r = [x * sx, y * sy, (x + max(w, 1)) * sx, (y + max(h, 1)) * sy]
        if dashed:
            dash(d, r)
        else:
            d.rectangle(r, outline=RED, width=3)
    return im


def dash(d, r, on=8, off=5):
    x0, y0, x1, y1 = r
    for (a, b, c, e) in ((x0, y0, x1, y0), (x1, y0, x1, y1), (x1, y1, x0, y1), (x0, y1, x0, y0)):
        length = max(abs(c - a), abs(e - b))
        t = 0.0
        while t < length:
            u = min(t + on, length)
            f = lambda s: (a + (c - a) * s / length, b + (e - b) * s / length)
            d.line([f(t), f(u)], fill=RED, width=3)
            t = u + off


def side_by_side(left, right, labels):
    from PIL import Image, ImageDraw, ImageFont

    pad, head = 12, 26
    w = left.width + right.width + 3 * pad
    h = max(left.height, right.height) + head + 2 * pad
    im = Image.new("RGB", (w, h), (255, 255, 255))
    try:
        font = ImageFont.load_default(size=16)
    except TypeError:
        font = ImageFont.load_default()
    d = ImageDraw.Draw(im)
    for k, (pic, lab) in enumerate(((left, labels[0]), (right, labels[1]))):
        x = pad + k * (left.width + pad)
        d.text((x, pad // 2), lab, fill=(30, 30, 30), font=font)
        im.paste(pic, (x, head + pad))
        d.rectangle([x - 1, head + pad - 1, x + pic.width, head + pad + pic.height], outline=(160, 160, 160))
    return im.convert("P", palette=Image.ADAPTIVE, colors=192)


def main():
    manifest, kit = sys.argv[1], sys.argv[2]
    try:
        import pymupdf  # noqa: F401
        from PIL import Image  # noqa: F401
    except ImportError as e:
        print(f"  no renders: {e}")
        return
    if not shutil.which("soffice"):
        print("  no renders: soffice (LibreOffice) not found")
        return
    jobs = json.load(open(manifest, encoding="utf-8"))
    import pymupdf

    tmp = tempfile.mkdtemp(prefix="kit-render-")
    try:
        files = sorted({j["file"] for j in jobs} | {j["original"] for j in jobs})
        to_pdf([os.path.join(kit, f) for f in files], tmp)
        total = 0
        for j in jobs:
            before, size = slides(j["before"])
            after, _ = slides(j["after"])
            pdf = lambda f: pymupdf.open(os.path.join(tmp, f.rsplit(".", 1)[0] + ".pdf"))
            try:
                po, pe = pdf(j["original"]), pdf(j["file"])
            except Exception as e:  # noqa: BLE001
                print(f"  {j['file']}: no PDF ({e})")
                continue
            if len(po) != len(before) or len(pe) != len(after):
                print(f"  {j['file']}: PDF pages {len(po)}/{len(pe)} are not the slides {len(before)}/{len(after)}; no renders")
                continue
            stem = j["file"].rsplit(".", 1)[0]
            made = []
            for k, o in enumerate(j["order"]):
                changed, ob, oa, what = changes(before[o - 1] if o else None, after[k])
                if not changed:
                    continue
                right = page_image(pe, k, size, oa, False)
                if o:
                    left = page_image(po, o - 1, size, ob, True)
                    lab = (f"before: original slide {o}", f"after: slide {k + 1} of this file")
                else:
                    from PIL import Image

                    left = Image.new("RGB", right.size, (240, 240, 240))
                    lab = ("before: (a new slide)", f"after: slide {k + 1} of this file")
                name = f"{stem}-slide{k + 1}.png"
                side_by_side(left, right, lab).save(os.path.join(kit, name), optimize=True)
                made.append((k + 1, o, name, what))
            total += len(made)
            if made and j.get("md"):
                add_section(os.path.join(kit, j["md"]), made)
            print(f"  {j['file']}: {len(made)} slides")
        print(f"  {total} renders")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def add_section(md_path, made):
    sec = [
        "## Before and after (LibreOffice renders)\n",
        "Each slide of this file that an edit changed, next to the original slide it came from. "
        "Red outlines mark the boxes of the objects that changed: solid on the right (where they are now), "
        "dashed on the left (where they were). These are LibreOffice renders, which can differ slightly from "
        "PowerPoint (fonts, text wrapping, effects): they show where to look, and PowerPoint is what counts.\n",
    ]
    for k, o, name, what in made:
        src = f"original slide {o}" if o else "a new slide"
        sec.append(f"- Slide {k} ({src}): {'; '.join(what)}. `{name}`\n\n  ![slide {k}]({name})\n")
    text = open(md_path, encoding="utf-8").read()
    at = text.find("\n## Model text")
    block = "\n".join(sec)
    text = text[:at] + "\n" + block + text[at:] if at >= 0 else text + "\n" + block
    open(md_path, "w", encoding="utf-8").write(text)


if __name__ == "__main__":
    main()
