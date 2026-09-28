"""Package and XML helpers: read/write a .docx, canonicalise XML for GetPut."""
import copy
import zipfile

from lxml import etree

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
WP = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
XML_NS = "http://www.w3.org/XML/1998/namespace"

DOC_PART = "word/document.xml"


def qn(tag):
    """'w:p' -> '{ns}p'."""
    pfx, local = tag.split(":")
    ns = {"w": W, "r": R, "mc": MC, "wp": WP, "xml": XML_NS}[pfx]
    return "{%s}%s" % (ns, local)


def local(el):
    t = el.tag
    if not isinstance(t, str):  # comment / PI
        return "#" + type(el).__name__
    return t.split("}", 1)[1] if "}" in t else t


def is_w(el, name):
    return el.tag == "{%s}%s" % (W, name)


def wval(el, attr="val"):
    if el is None:
        return None
    return el.get("{%s}%s" % (W, attr))


PARSER = etree.XMLParser(remove_blank_text=False, huge_tree=True, resolve_entities=False)


def read_docx(path):
    """Return (ordered list of (ZipInfo, bytes))."""
    with zipfile.ZipFile(path) as z:
        return [(i, z.read(i.filename)) for i in z.infolist()]


def part(parts, name):
    for info, data in parts:
        if info.filename == name:
            return data
    return None


def write_docx(path, parts, replacements):
    """Copy every part through unchanged except those in `replacements`."""
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for info, data in parts:
            data = replacements.get(info.filename, data)
            zi = zipfile.ZipInfo(info.filename, date_time=info.date_time)
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.external_attr = info.external_attr
            z.writestr(zi, data)


def parse(data):
    return etree.fromstring(data, PARSER)


def _strip_ws(el):
    """Drop whitespace-only text/tail outside w:t / w:instrText (pretty-print noise)."""
    keep_text = {"t", "instrText", "delText", "delInstrText"}
    for e in el.iter():
        if not isinstance(e.tag, str):
            continue
        if local(e) not in keep_text and e.text is not None and not e.text.strip() and len(e):
            e.text = None
        if e.tail is not None and not e.tail.strip():
            e.tail = None


def canon(el_or_bytes):
    """C14N 2.0 of an element (or document bytes) after dropping inter-element whitespace."""
    el = parse(el_or_bytes) if isinstance(el_or_bytes, (bytes, bytearray)) else copy.deepcopy(el_or_bytes)
    _strip_ws(el)
    return etree.tostring(el, method="c14n2", with_comments=False)


def fp(*els):
    """Fingerprint of one or more elements (canonical XML, joined)."""
    out = []
    for e in els:
        if e is None:
            out.append("-")
        elif isinstance(e, str):
            out.append(e)
        else:
            out.append(canon(e).decode("utf-8"))
    return "|".join(out)


def text_of(el, limit=60):
    ts = []
    for t in el.iter(qn("w:t"), qn("w:delText"), qn("w:instrText")):
        if t.text:
            ts.append(t.text)
    s = " ".join("".join(ts).split())
    return s[:limit] + ("…" if len(s) > limit else "")
