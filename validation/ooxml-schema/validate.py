#!/usr/bin/env python3
"""Validates Office Open XML packages (.pptx .pptm .docx .docm .xlsx .xlsm) against the
ISO/IEC 29500 transitional schemas, plus rules outside the schemas that Office enforces.

    validation/ooxml-schema/fetch.sh                      # the schemas, once (into xsd/)
    python3 validation/ooxml-schema/validate.py PATH...   # files or directories; needs lxml

Each XML part whose root is in a transitional namespace is validated after markup
compatibility processing (ISO/IEC 29500-3) as an application that knows only the
transitional namespaces does it: mc:AlternateContent keeps its first mc:Choice whose
Requires it knows, else mc:Fallback; elements and attributes in mc:Ignorable namespaces
are dropped. xml:space is dropped too: XML allows it on any element and Excel writes it
where the schemas do not declare it (shared strings). One schema erratum is patched
(ERRATA below). Extension lists (extLst) are lax in the schemas, so their content is not
checked. Parts in other namespaces (Office 2010+ extension parts, custom XML, VML) are
listed as not checked.

Beyond the schemas, per package:
  - every part has a content type, and every content-type override names a part;
  - every internal relationship target exists, and every r:id, r:embed, r:link, … in a
    part names a relationship of that part;
  - mc:Ignorable and mc:Choice Requires prefixes are declared;
  - presentations: slide ids unique; master and layout ids unique across the deck;
    shape ids (cNvPr id) unique per slide, layout and master; every animation target
    (p:spTgt) and build entry (p:bldP) names a shape of its slide;
  - embedded packages (a chart's workbook, an embedded document) are checked the same way.

A finding is reported as `file: part: message`. The allowlist (allowlist.txt beside this
script, or --allow FILE) lists known deviations in third-party files, each with a reason;
the script exits 1 when any finding is not allowlisted.
"""
import argparse
import fnmatch
import io
import os
import posixpath
import re
import shutil
import sys
import tempfile
import zipfile

from lxml import etree

HERE = os.path.dirname(os.path.abspath(__file__))
EXTS = (".pptx", ".pptm", ".potx", ".ppsx", ".docx", ".docm", ".dotx", ".xlsx", ".xlsm", ".xltx")

MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
P = "http://schemas.openxmlformats.org/presentationml/2006/main"
PR = "http://schemas.openxmlformats.org/package/2006/relationships"
CT = "http://schemas.openxmlformats.org/package/2006/content-types"
XML_NS = "http://www.w3.org/XML/1998/namespace"
DC = "http://purl.org/dc/elements/1.1/"
DCTERMS = "http://purl.org/dc/terms/"

# Stand-ins for schemas the set imports but does not contain: xml:lang and xml:space, and the
# Dublin Core elements the core properties part uses (checked loosely; W3CDTF as the OPC
# rules define it for dcterms:created and dcterms:modified).
STUBS = {
    "xml.xsd": f"""<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="{XML_NS}">
  <xs:attribute name="lang" type="xs:language"/>
  <xs:attribute name="space"><xs:simpleType><xs:restriction base="xs:NCName">
    <xs:enumeration value="default"/><xs:enumeration value="preserve"/></xs:restriction></xs:simpleType></xs:attribute>
</xs:schema>""",
    "dc.xsd": f"""<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="{DC}" elementFormDefault="qualified">
""" + "".join(f'  <xs:element name="{n}" type="xs:anyType"/>\n' for n in (
        "title", "creator", "subject", "description", "publisher", "contributor", "date", "type",
        "format", "identifier", "source", "language", "relation", "coverage", "rights")) + "</xs:schema>",
    "dcterms.xsd": f"""<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns="{DCTERMS}" targetNamespace="{DCTERMS}" elementFormDefault="qualified">
  <xs:element name="created" type="xs:anyType"/>
  <xs:element name="modified" type="xs:anyType"/>
  <xs:simpleType name="W3CDTF"><xs:union memberTypes="xs:gYear xs:gYearMonth xs:date xs:dateTime"/></xs:simpleType>
</xs:schema>""",
}


# Errata in the published transitional schemas, where they reject what Office itself writes and
# reads. Applied to the copies before loading: (file, text, replacement, why).
ERRATA = [
    ("dml-main.xsd",
     """<xsd:simpleType name="ST_TextBulletSizePercent">
    <xsd:restriction base="xsd:string">
      <xsd:pattern value="0*((2[5-9])|([3-9][0-9])|([1-3][0-9][0-9])|400)%"/>
    </xsd:restriction>
  </xsd:simpleType>""",
     """<xsd:simpleType name="ST_TextBulletSizePercent">
    <xsd:union memberTypes="ST_TextBulletSizeDecimal">
      <xsd:simpleType><xsd:restriction base="xsd:string">
        <xsd:pattern value="0*((2[5-9])|([3-9][0-9])|([1-3][0-9][0-9])|400)%"/>
      </xsd:restriction></xsd:simpleType>
    </xsd:union>
  </xsd:simpleType>""",
     "a:buSzPct: Office writes thousandths of a percent (80000), as ECMA-376 1st edition defined it; "
     "the transitional schema kept only the strict '80%' form, unlike ST_Percentage beside it"),
]


def load_schema(xsd_dir):
    """One schema over every namespace of the set: copies the files, gives each xsd:import
    the location of its namespace's file (the published files have none), and imports all."""
    files = {}
    for f in sorted(os.listdir(xsd_dir)):
        if f.endswith(".xsd"):
            text = open(os.path.join(xsd_dir, f), encoding="utf-8").read()
            ns = re.search(r'targetNamespace="([^"]+)"', text).group(1)
            files[ns] = (f, text)
    if len(files) < 25:
        sys.exit(f"{xsd_dir}: {len(files)} schemas; run {HERE}/fetch.sh first")
    for name, text in STUBS.items():
        ns = re.search(r'targetNamespace="([^"]+)"', text).group(1)
        files[ns] = (name, text)
    tmp = tempfile.mkdtemp(prefix="ooxml-xsd-")
    try:
        def locate(m):
            ns = m.group(2)
            if ns not in files:
                return m.group(0)
            attrs = re.sub(r'\s*schemaLocation="[^"]*"', "", m.group(0)[:-2 if m.group(0).endswith("/>") else -1])
            return f'{attrs} schemaLocation="{files[ns][0]}"/>'
        for name, old, new, _ in ERRATA:
            ns = next(k for k, v in files.items() if v[0] == name)
            if old not in files[ns][1]:
                sys.exit(f"{name}: erratum text not found; update ERRATA in {__file__}")
            files[ns] = (name, files[ns][1].replace(old, new))
        for ns, (name, text) in files.items():
            text = re.sub(r'<(xs|xsd):import\b[^>]*?namespace="([^"]+)"[^>]*?/>', locate, text, flags=re.S)
            open(os.path.join(tmp, name), "w", encoding="utf-8").write(text)
        wrapper = '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:hanji:all">\n'
        wrapper += "".join(f'  <xs:import namespace="{ns}" schemaLocation="{n}"/>\n' for ns, (n, _) in files.items())
        wrapper += "</xs:schema>\n"
        open(os.path.join(tmp, "all.xsd"), "w").write(wrapper)
        schema = etree.XMLSchema(etree.parse(os.path.join(tmp, "all.xsd")))
    finally:
        shutil.rmtree(tmp)
    known = set(files) - {DC, DCTERMS}
    return schema, known


# ------------------------------------------------------------------ markup compatibility


def mce(root, known, findings, part):
    """Markup compatibility processing in place, as an application that knows `known`."""

    def prefixes_to_ns(el, value, what):
        out = []
        for pfx in value.split():
            ns = el.nsmap.get(pfx)
            if ns is None:
                findings.append((part, f"{what} names prefix {pfx!r}, which is not declared (line {el.sourceline})"))
            else:
                out.append(ns)
        return out

    def walk(el, ignorable):
        ig = el.get(f"{{{MC}}}Ignorable")
        if ig is not None:
            ignorable = ignorable | set(prefixes_to_ns(el, ig, "mc:Ignorable"))
            del el.attrib[f"{{{MC}}}Ignorable"]
        el.attrib.pop(f"{{{XML_NS}}}space", None)
        for a in list(el.attrib):
            if a.startswith("{"):
                ns = a[1:].split("}")[0]
                if ns == MC:
                    del el.attrib[a]
                elif ns in ignorable and ns not in known:
                    del el.attrib[a]
        again = True
        while again:  # a kept Choice or Fallback can itself hold AlternateContent
            again = resolve_children(el, ignorable)
        for child in el:
            # An extension's content is lax in the schemas and read only by applications that
            # know its namespace, so it is left as written.
            if isinstance(child.tag, str) and not (etree.QName(child).localname == "ext" and etree.QName(el).localname == "extLst"):
                walk(child, ignorable)

    def resolve_children(el, ignorable):
        again = False
        for child in list(el):
            if not isinstance(child.tag, str):
                continue
            ns = etree.QName(child).namespace
            if ns == MC and etree.QName(child).localname == "AlternateContent":
                again = True
                chosen = None
                for alt in child:
                    if not isinstance(alt.tag, str):
                        continue
                    ln = etree.QName(alt).localname
                    if ln == "Choice":
                        req = prefixes_to_ns(alt, alt.get("Requires", ""), "mc:Choice Requires")
                        if chosen is None and req and all(r in known for r in req):
                            chosen = alt
                    elif ln == "Fallback" and chosen is None:
                        chosen = alt
                idx = el.index(child)
                el.remove(child)
                if chosen is not None:
                    for i, c in enumerate(list(chosen)):
                        el.insert(idx + i, c)
                continue
            if ns in ignorable and ns not in known:
                el.remove(child)
        return again

    walk(root, frozenset())


# ------------------------------------------------------------------ package checks


def resolve(base_part, target):
    if target.startswith("/"):
        return posixpath.normpath(target.lstrip("/"))
    return posixpath.normpath(posixpath.join(posixpath.dirname(base_part), target))


def rels_name(part):
    d, f = posixpath.split(part)
    return posixpath.join(d, "_rels", f + ".rels")


def rels_source(rels):
    d, f = posixpath.split(rels)
    return posixpath.join(posixpath.dirname(d), f[: -len(".rels")])


def check_package(data, label, schema, known, stats, depth=0):
    """Findings as (part, message) for one package's bytes."""
    findings = []
    try:
        z = zipfile.ZipFile(io.BytesIO(data))
    except zipfile.BadZipFile as e:
        return [("", f"not a zip: {e}")]
    names = [n for n in z.namelist() if not n.endswith("/")]
    parts = set(names)
    lower = {n.lower(): n for n in names}

    # content types
    if "[Content_Types].xml" not in parts:
        return [("", "no [Content_Types].xml")]
    ctx = etree.fromstring(z.read("[Content_Types].xml"))
    defaults = {d.get("Extension", "").lower(): d.get("ContentType") for d in ctx.iter(f"{{{CT}}}Default")}
    overrides = {o.get("PartName", "").lstrip("/").lower(): o.get("ContentType") for o in ctx.iter(f"{{{CT}}}Override")}
    for pn in overrides:
        if pn not in lower:
            findings.append(("[Content_Types].xml", f"override for /{pn}, which is not in the package"))

    def ctype(n):
        if n.lower() in overrides:
            return overrides[n.lower()]
        return defaults.get(n.rsplit(".", 1)[-1].lower() if "." in n else "")

    for n in names:
        if n != "[Content_Types].xml" and ctype(n) is None:
            findings.append((n, "part has no content type"))

    # relationships
    rels_of = {}
    for n in names:
        if not n.endswith(".rels"):
            continue
        try:
            rx = etree.fromstring(z.read(n))
        except etree.XMLSyntaxError:
            continue  # reported below
        src = rels_source(n)
        if src and src not in parts:
            findings.append((n, f"relationships of /{src}, which is not in the package"))
        ids = {}
        for r in rx.iter(f"{{{PR}}}Relationship"):
            ids[r.get("Id")] = r
            if r.get("TargetMode") == "External":
                continue
            t = r.get("Target", "")
            t = t.split("#", 1)[0]
            if not t:
                continue
            tgt = resolve(src, t)
            if tgt not in parts:
                hint = " (case differs)" if tgt.lower() in lower else ""
                findings.append((n, f"{r.get('Id')} -> {r.get('Target')}: target /{tgt} is not in the package{hint}"))
        rels_of[src] = ids

    # parts
    embedded = []
    for n in sorted(names):
        ct = ctype(n) or ""
        is_xml = n.endswith(".xml") or n.endswith(".rels") or ct.endswith("+xml") or ct.endswith("/xml")
        if n.lower().endswith(EXTS) and depth < 2:
            embedded.append(n)
            continue
        if not is_xml or n.endswith(".vml"):
            if n.endswith(".vml"):
                stats["unchecked"].add("VML (legacy drawing)")
            continue
        try:
            root = etree.fromstring(z.read(n), etree.XMLParser(huge_tree=True))
        except etree.XMLSyntaxError as e:
            findings.append((n, f"not well-formed XML: {e}"))
            continue
        ns = etree.QName(root).namespace
        # relationship references
        ids = rels_of.get(n, {})
        for el in root.iter():
            if not isinstance(el.tag, str):
                continue
            for a, v in el.attrib.items():
                if a.startswith(f"{{{R}}}") and v and v not in ids:
                    findings.append((n, f"{etree.QName(el).localname}/@r:{a.split('}')[1]}={v!r} names no relationship of this part (line {el.sourceline})"))
        if ns in known:
            mce(root, known, findings, n)
            root = etree.fromstring(etree.tostring(root))  # fresh line numbers are lost; errors name paths
            stats["checked"] += 1
            if not schema.validate(root):
                seen = set()
                for err in schema.error_log:
                    msg = re.sub(r"^Element '", "'", err.message)
                    path = err.path or ""
                    key = (msg,)
                    if key in seen:
                        continue
                    seen.add(key)
                    findings.append((n, f"schema: {msg}" + (f" at {path}" if path else "")))
                    if len(seen) >= 20:
                        findings.append((n, "schema: more errors not shown"))
                        break
            if ns == P:
                findings += [(n, m) for m in pml_rules(n, root)]
        else:
            stats["unchecked"].add(ns or f"(no namespace: {etree.QName(root).localname})")

    # presentation-wide ids
    if "ppt/presentation.xml" in parts:
        findings += deck_ids(z, parts, rels_of)

    for n in embedded:
        sub = check_package(z.read(n), f"{label}!{n}", schema, known, stats, depth + 1)
        findings += [(f"{n}!{p}", m) for p, m in sub]
    return findings


def pml_rules(part, root):
    """Shape ids unique per slide, layout or master; animation targets name shapes."""
    out = []
    ids = {}
    for el in root.iter(f"{{{P}}}cNvPr"):
        i = el.get("id")
        if i in ids:
            out.append(f"shape id {i} used twice ({ids[i]!r} and {el.get('name')!r})")
        ids[i] = el.get("name")
    timing = root.find(f"{{{P}}}timing")
    if timing is not None:
        for t in timing.iter(f"{{{P}}}spTgt"):
            if t.get("spid") not in ids:
                out.append(f"animation target spid={t.get('spid')} is no shape of this slide")
        for b in timing.iter(f"{{{P}}}bldP", f"{{{P}}}bldDgm", f"{{{P}}}bldOleChart", f"{{{P}}}bldGraphic"):
            if b.get("spid") not in ids:
                out.append(f"build list {etree.QName(b).localname} spid={b.get('spid')} is no shape of this slide")
    return out


def deck_ids(z, parts, rels_of):
    out = []
    prs = etree.fromstring(z.read("ppt/presentation.xml"))
    seen = {}
    for s in prs.iter(f"{{{P}}}sldId"):
        if s.get("id") in seen:
            out.append(("ppt/presentation.xml", f"slide id {s.get('id')} used twice"))
        seen[s.get("id")] = 1
    ml = {}
    for m in prs.iter(f"{{{P}}}sldMasterId"):
        ml.setdefault(m.get("id"), []).append("ppt/presentation.xml")
        rid = m.get(f"{{{R}}}id")
        rel = rels_of.get("ppt/presentation.xml", {}).get(rid)
        if rel is None:
            continue
        mp = resolve("ppt/presentation.xml", rel.get("Target"))
        if mp not in parts:
            continue
        mx = etree.fromstring(z.read(mp))
        for lid in mx.iter(f"{{{P}}}sldLayoutId"):
            ml.setdefault(lid.get("id"), []).append(mp)
    for i, where in ml.items():
        if len(where) > 1:
            out.append((where[-1], f"master/layout id {i} used {len(where)} times across the deck ({', '.join(sorted(set(where)))})"))
    return out


# ------------------------------------------------------------------ allowlist and main


def load_allow(path):
    """Lines `file-glob | part-glob | message-regex | reason` (fields split on " | "); # starts a comment."""
    rules = []
    if not path or not os.path.exists(path):
        return rules
    for i, line in enumerate(open(path, encoding="utf-8"), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        f = [x.strip() for x in line.split(" | ", 3)]
        if len(f) != 4 or not f[3]:
            sys.exit(f"{path}:{i}: want `file-glob | part-glob | message-regex | reason`")
        rules.append({"file": f[0], "part": f[1], "re": re.compile(f[2]), "why": f[3], "line": i, "used": 0})
    return rules


def allowed(rules, file, part, msg):
    base = os.path.basename(file).lower()
    for r in rules:
        if fnmatch.fnmatch(base, r["file"].lower()) and fnmatch.fnmatch(part, r["part"]) and r["re"].search(msg):
            r["used"] += 1
            return r
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("paths", nargs="+")
    ap.add_argument("--xsd", default=os.path.join(HERE, "xsd"))
    ap.add_argument("--allow", default=os.path.join(HERE, "allowlist.txt"))
    ap.add_argument("-q", "--quiet", action="store_true", help="print only findings that fail")
    a = ap.parse_args()
    schema, known = load_schema(a.xsd)
    rules = load_allow(a.allow)
    files = []
    for p in a.paths:
        if os.path.isdir(p):
            for d, _, fs in os.walk(p):
                files += [os.path.join(d, f) for f in fs if f.lower().endswith(EXTS)]
        else:
            files.append(p)
    files.sort()
    if not files:
        sys.exit("no packages found")
    stats = {"checked": 0, "unchecked": set()}
    bad = allow_n = 0
    failing = set()
    for f in files:
        findings = check_package(open(f, "rb").read(), f, schema, known, stats)
        for part, msg in findings:
            r = allowed(rules, f, part, msg)
            if r:
                allow_n += 1
                if not a.quiet:
                    print(f"allowed  {f}: {part}: {msg}  [allowlist line {r['line']}]")
            else:
                bad += 1
                failing.add(f)
                print(f"FAIL     {f}: {part}: {msg}")
    for r in rules:
        if not r["used"] and not a.quiet:
            print(f"note: allowlist line {r['line']} matched nothing ({r['file']} | {r['part']})")
    print(f"{len(files)} packages, {stats['checked']} XML parts schema-checked; "
          f"{bad} findings in {len(failing)} packages, {allow_n} allowlisted")
    if stats["unchecked"] and not a.quiet:
        print("not schema-checked (namespaces outside the transitional set): " + ", ".join(sorted(stats["unchecked"])))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
