#!/usr/bin/env python3
"""hwpx part of the spike: write OWPML track-change markup directly and see what
rhwp (and LibreOffice) do with it.

    RHWP=/path/to/rhwp python3 hwpx_spike.py      # -> out/hwpx/*, results/hwpx.json

rhwp has no API that writes tracked changes (see SPIKE.md), so the markup is
written here at the XML level, the way hanji's own exporter would:

  Contents/header.xml, end of <hh:refList>:
    <hh:trackChanges itemCnt="n"><hh:trackChange type="Insert|Delete" date=… authorID="1" hide="0" id="k"/>…
    <hh:trackChangeAuthors itemCnt="1"><hh:trackChangeAuthor name=… mark="1" color="#FF0000" id="1"/>
  Contents/section0.xml, inside <hp:t> (TrackChangeTag, ParaList XML schema.xml:294-299, 2808-2812):
    <hp:deleteBegin Id="k" TcId="k" paraend="0"/>old<hp:deleteEnd Id="k" TcId="k" paraend="0"/>
    <hp:insertBegin Id="k" TcId="k" paraend="0"/>new<hp:insertEnd Id="k" TcId="k" paraend="0"/>
  paraend="1" on the end tag when the paragraph end is part of the change.

Inputs: report.hwpx (made by `rhwp scaffold hwpx/report.json`) and, if present,
mel-001.hwpx (Hancom Office 2021 file from rhwp's samples, fetched by
hwpx/fetch.sh). Edits: H1 replace a figure inside a paragraph (delete + insert),
H2 inserted paragraph, H3 changed table cell, H4 deleted paragraph.
"""
import copy
import json
import os
import re
import shutil
import subprocess
import zipfile

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out", "hwpx")
RES = os.path.join(HERE, "results")
RHWP = os.environ.get("RHWP", "rhwp")
HP = "http://www.hancom.co.kr/hwpml/2011/paragraph"
HH = "http://www.hancom.co.kr/hwpml/2011/head"
hp = lambda t: "{%s}%s" % (HP, t)  # noqa: E731
hh = lambda t: "{%s}%s" % (HH, t)  # noqa: E731
AUTHOR, DATE = "hanji (model edit)", "2026-09-28T00:00:00Z"
TRACK_TAGS = ("insertBegin", "insertEnd", "deleteBegin", "deleteEnd")


class Changes:
    def __init__(self):
        self.items = []  # (id, type)

    def new(self, kind):
        k = len(self.items) + 1
        self.items.append((k, kind))
        return k


def tag(name, k, paraend=False):
    e = etree.Element(hp(name))
    e.set("Id", str(k))
    e.set("TcId", str(k))
    e.set("paraend", "1" if paraend else "0")
    return e


def plain_t(p):
    """hp:t elements of direct runs of p holding only text."""
    return [t for r in p.findall(hp("run")) for t in r.findall(hp("t")) if len(t) == 0 and (t.text or "").strip()]


def text(p):
    return "".join("".join(t.itertext()) for t in p.iter(hp("t")))


def top_paras(sec):
    return [p for p in sec.findall(hp("p")) if p.find(".//" + hp("secPr")) is None
            and p.find(".//" + hp("tbl")) is None and plain_t(p)]


def h1_replace_figure(sec, ch, log):
    for p in top_paras(sec):
        if len(text(p)) < 15:
            continue  # skip headings and dates: a figure inside a sentence
        for t in plain_t(p):
            m = re.search(r"\d[\d,.]*%?", t.text)
            if not m:
                continue
            old = m.group(0)
            new = re.sub(r"\d+", lambda d: str(int(d.group(0)) + 3), old, count=1)
            before, after = t.text[:m.start()], t.text[m.end():]
            kd, ki = ch.new("Delete"), ch.new("Insert")
            t.text = before
            db, de, ib, ie = tag("deleteBegin", kd), tag("deleteEnd", kd), tag("insertBegin", ki), tag("insertEnd", ki)
            db.tail, de.tail, ib.tail, ie.tail = old, None, new, after
            t.extend([db, de, ib, ie])
            return log.append({"edit": "H1", "what": "%r -> %r inside %r" % (old, new, (before + old + after)[:50])})
    log.append({"edit": "H1", "what": "n/a"})


def h2_insert_para(sec, ch, log):
    paras = top_paras(sec)
    if len(paras) < 2:
        return log.append({"edit": "H2", "what": "n/a"})
    anchor = paras[1]
    p = copy.deepcopy(anchor)
    runs = p.findall(hp("run"))
    for r in runs[1:]:
        p.remove(r)
    for k in list(runs[0]):
        runs[0].remove(k)
    t = etree.SubElement(runs[0], hp("t"))
    k = ch.new("Insert")
    b, e = tag("insertBegin", k), tag("insertEnd", k, paraend=True)
    b.tail = "모델이 추가한 문단입니다."
    t.extend([b, e])
    anchor.addnext(p)
    log.append({"edit": "H2", "what": "insert a paragraph after %r" % text(anchor)[:40]})


def h3_cell(sec, ch, log):
    for tc in sec.iter(hp("tc")):
        for p in tc.iter(hp("p")):
            ts = plain_t(p)
            if ts:
                t = ts[0]
                old = t.text
                kd, ki = ch.new("Delete"), ch.new("Insert")
                db, de, ib, ie = tag("deleteBegin", kd), tag("deleteEnd", kd), tag("insertBegin", ki), tag("insertEnd", ki)
                t.text = None
                db.tail, ib.tail = old, "1,204"
                t.extend([db, de, ib, ie])
                return log.append({"edit": "H3", "what": "table cell %r -> '1,204'" % old[:30]})
    log.append({"edit": "H3", "what": "n/a"})


def h4_delete_para(sec, ch, log):
    paras = top_paras(sec)
    if len(paras) < 3:
        return log.append({"edit": "H4", "what": "n/a"})
    p = paras[-1]
    ts = [t for t in p.iter(hp("t"))]
    k = ch.new("Delete")
    b = tag("deleteBegin", k)
    b.tail = ts[0].text
    ts[0].text = None
    ts[0].insert(0, b)
    ts[-1].append(tag("deleteEnd", k, paraend=True))
    log.append({"edit": "H4", "what": "delete paragraph %r" % text(p)[:40]})


def add_header(root, ch):
    ref = root.find(hh("refList"))
    for name in ("trackChanges", "trackChangeAuthors"):
        old = ref.find(hh(name))
        if old is not None:
            ref.remove(old)  # none in the inputs; replace to keep the example simple
    tcs = etree.SubElement(ref, hh("trackChanges"), itemCnt=str(len(ch.items)))
    for k, kind in ch.items:
        etree.SubElement(tcs, hh("trackChange"), type=kind, date=DATE, authorID="1", hide="0", id=str(k))
    au = etree.SubElement(ref, hh("trackChangeAuthors"), itemCnt="1")
    etree.SubElement(au, hh("trackChangeAuthor"), name=AUTHOR, mark="1", color="#FF0000", id="1")


def inject(src, dst):
    zin = zipfile.ZipFile(src)
    sec = etree.fromstring(zin.read("Contents/section0.xml"))
    head = etree.fromstring(zin.read("Contents/header.xml"))
    ch, log = Changes(), []
    h1_replace_figure(sec, ch, log)
    h2_insert_para(sec, ch, log)
    h3_cell(sec, ch, log)
    h4_delete_para(sec, ch, log)
    add_header(head, ch)
    new = {"Contents/section0.xml": etree.tostring(sec, xml_declaration=True, encoding="UTF-8", standalone=True),
           "Contents/header.xml": etree.tostring(head, xml_declaration=True, encoding="UTF-8", standalone=True)}
    with zipfile.ZipFile(dst, "w") as zout:
        for it in zin.infolist():  # mimetype stays first and stored
            data = new.get(it.filename, zin.read(it.filename))
            zout.writestr(it, data, compress_type=it.compress_type)
    return log, len(ch.items)


# ---------------------------------------------------------------- checks
def run(*args, timeout=300):
    p = subprocess.run([RHWP, *args], capture_output=True, text=True, timeout=timeout)
    return p.returncode, p.stdout, p.stderr


def marks(path):
    z = zipfile.ZipFile(path)
    names = z.namelist()
    sec = b"".join(z.read(n) for n in names if n.startswith("Contents/section"))
    head = z.read("Contents/header.xml") if "Contents/header.xml" in names else b""
    return {"section_marks": sum(sec.count(b"<hp:" + t.encode()) for t in TRACK_TAGS),
            "header_trackChange": head.count(b"<hh:trackChange "),
            "header_trackChangeAuthor": head.count(b"<hh:trackChangeAuthor ")}


def well_formed(path):
    bad = []
    z = zipfile.ZipFile(path)
    for n in z.namelist():
        if n.endswith(".xml") or n.endswith(".hpf") or n.endswith(".rdf"):
            try:
                etree.fromstring(z.read(n))
            except etree.XMLSyntaxError as e:
                bad.append("%s: %s" % (n, e))
    return bad


def rhwp_text(path):
    code, out, err = run("export-text", path, "--json")
    if code:
        return None, (err or out)[-300:]
    try:
        env = json.loads(out)
    except json.JSONDecodeError:
        return None, out[:300]
    # the envelope holds per-page text; collect every string leaf
    strings = []

    def walk(v):
        if isinstance(v, str):
            strings.append(v)
        elif isinstance(v, dict):
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)
    walk(env)
    return "\n".join(strings), None


def check(stem, base, tracked, log):
    d = os.path.join(OUT, stem)
    os.makedirs(d, exist_ok=True)
    c = {"edits": "; ".join("%s %s" % (x["edit"], x["what"]) for x in log)}
    c["well-formed XML (all parts)"] = well_formed(tracked) or "yes"
    c["marks written"] = marks(tracked)
    code, out, err = run("info", tracked)
    c["rhwp info opens it"] = "yes" if code == 0 else "no: " + (err or out)[-200:]
    txt, e = rhwp_text(tracked)
    if txt is None:
        c["rhwp export-text"] = "fails: " + e
    else:
        probes = {}
        for x in log:
            m = re.match(r"'(.+?)' -> '(.+?)'", x["what"])
            if x["edit"] == "H1" and m:
                probes["deleted figure %r still shown" % m.group(1)] = m.group(1) in txt
                probes["inserted figure %r shown" % m.group(2)] = m.group(2) in txt
            if x["edit"] == "H2":
                probes["inserted paragraph shown"] = "모델이 추가한 문단입니다." in txt
            if x["edit"] == "H4":
                probes["deleted paragraph still shown"] = x["what"].split("'")[1][:15] in txt
        c["rhwp export-text (deleted text is shown as live text?)"] = probes
    code, out, err = run("export-svg", tracked, "-o", os.path.join(d, "svg"))
    svgs = [f for f in os.listdir(os.path.join(d, "svg"))] if os.path.isdir(os.path.join(d, "svg")) else []
    c["rhwp export-svg renders"] = ("yes, %d page(s)" % len(svgs)) if code == 0 and svgs else "no: " + (err or out)[-200:]
    code, out, err = run("export-pdf", tracked, "-o", os.path.join(d, "tracked.pdf"))
    c["rhwp export-pdf renders"] = "yes" if code == 0 and os.path.exists(os.path.join(d, "tracked.pdf")) else "no: " + (err or out)[-200:]
    # rhwp save: an unrelated edit, so the whole document goes through rhwp's IR and serializer
    saved = os.path.join(d, "saved.hwpx")
    code, out, err = run("edit", "replace-text", tracked, "--find", "문단", "--replace", "문단", "-o", saved, "--json")
    if code == 0 and os.path.exists(saved):
        c["rhwp save (edit replace-text) keeps the marks"] = marks(saved)
        st, _ = rhwp_text(saved)
        if st is not None:
            c["after rhwp save, deleted text is ordinary text"] = all(
                x["what"].split("'")[1][:15] in st for x in log if x["edit"] in ("H1", "H4") and x["what"] != "n/a")
        try:
            env = json.loads(out)
            c["rhwp save reports content loss"] = env.get("contentLoss", env.get("data", {}).get("contentLoss", "no contentLoss field"))
        except json.JSONDecodeError:
            pass
    else:
        c["rhwp save (edit replace-text)"] = "fails: " + (err or out)[-300:]
    code, out, err = run("hwpx-roundtrip", tracked, "-o", os.path.join(d, "roundtrip"))
    c["rhwp hwpx-roundtrip (loss gate)"] = "exit %d: %s" % (code, (out or err).strip().replace("\n", " ")[-400:])
    rt = [os.path.join(dp, f) for dp, _, fs in os.walk(os.path.join(d, "roundtrip")) for f in fs if f.endswith(".hwpx")]
    if rt:
        c["rhwp hwpx-roundtrip output keeps the marks"] = marks(rt[0])
    code, out, err = run("ir-diff", base, tracked, "--json")
    try:
        env = json.loads(out)
        c["rhwp ir-diff base vs tracked"] = {"identical": env.get("identical"), "diffCount": env.get("diffCount")}
    except json.JSONDecodeError:
        c["rhwp ir-diff base vs tracked"] = "exit %d: %s" % (code, (out or err)[-300:])
    hwp = os.path.join(d, "converted.hwp")
    code, out, err = run("convert", tracked, hwp)
    c["rhwp convert to HWP5"] = "exit %d%s" % (code, "" if code == 0 else ": " + (err or out)[-200:])
    if code == 0 and os.path.exists(hwp):
        ht, _ = rhwp_text(hwp)
        c["HWP5 from convert: deleted text is ordinary text"] = ht is not None and all(
            x["what"].split("'")[1][:15] in ht for x in log if x["edit"] in ("H1", "H4") and x["what"] != "n/a")
    lo = os.path.join(d, "lo")
    os.makedirs(lo, exist_ok=True)
    p = subprocess.run(["soffice", "--headless", "-env:UserInstallation=file://" + os.path.join(OUT, "lo-profile"),
                        "--convert-to", "odt", "--outdir", lo, tracked], capture_output=True, text=True, timeout=300)
    odt = os.path.join(lo, os.path.basename(tracked)[:-5] + ".odt")
    c["LibreOffice 24.2 opens hwpx"] = ("yes" if os.path.exists(odt) else "no: " + (p.stdout + p.stderr).strip()[-200:])
    return c


def main():
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(RES, exist_ok=True)
    inputs = []
    base = os.path.join(OUT, "report.hwpx")
    code, out, err = run("scaffold", os.path.join(HERE, "hwpx", "report.json"), "-o", base)
    if code:
        raise SystemExit("rhwp scaffold failed: " + err + out)
    inputs.append(("report", base))
    mel = os.path.join(HERE, "hwpx", "mel-001.hwpx")
    if os.path.exists(mel):
        shutil.copy(mel, os.path.join(OUT, "mel-001.hwpx"))
        inputs.append(("mel-001", os.path.join(OUT, "mel-001.hwpx")))
    code, ver, _ = run("--version")
    rows = []
    for stem, src in inputs:
        dst = os.path.join(OUT, stem + ".tracked.hwpx")
        log, n = inject(src, dst)
        rows.append({"file": stem, "changes": n, "checks": check(stem, src, dst, log)})
        print(json.dumps(rows[-1], ensure_ascii=False, indent=1))
    json.dump({"rhwp": ver.strip(), "rows": rows}, open(os.path.join(RES, "hwpx.json"), "w"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()
