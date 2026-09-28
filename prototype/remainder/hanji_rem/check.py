"""Measurements: GetPut, per-edit outcome per design, PutGet."""
from collections import Counter, defaultdict


from .ooxml import canon, parse, local

COUNTED = ("ppr", "run", "marker", "rmarker", "wrap", "keep", "bkeep", "bmarker", "tbl", "tr", "tc", "tail")


def xml_diff(a_bytes, b_bytes, limit=3):
    """First differences between two canonical documents, as short reasons."""
    a, b = parse(canon(a_bytes)), parse(canon(b_bytes))
    ea, eb = list(a.iter()), list(b.iter())
    out = []

    def path(e):
        p = []
        while e is not None:
            p.append(local(e)); e = e.getparent()
        return "/".join(reversed(p[:-1]))[-80:]

    k = 0
    while k < min(len(ea), len(eb)) and len(out) < limit:
        x, y = ea[k], eb[k]
        if x.tag != y.tag:
            out.append("element #%d: %s became %s at %s" % (k, local(x), local(y), path(x)))
            break
        if dict(x.attrib) != dict(y.attrib):
            out.append("attributes of %s differ: %s vs %s" % (path(x), dict(x.attrib), dict(y.attrib)))
        elif (x.text or "") != (y.text or ""):
            out.append("text of %s: %r vs %r" % (path(x), (x.text or "")[:30], (y.text or "")[:30]))
        k += 1
    if not out and len(ea) != len(eb):
        out.append("element count %d vs %d" % (len(ea), len(eb)))
    return out


class Observed:
    def __init__(self, doc):
        self.cov = defaultdict(dict)
        self.multi = Counter()
        for e in doc.entries:
            if e.kind == "run" and e.end > e.start:
                for c in range(e.start, e.end):
                    self.cov[e.path][c] = e.fp
            else:
                self.multi[self.key(e)] += 1

    @staticmethod
    def key(e):
        if e.kind in ("marker", "rmarker", "keep") or (e.kind == "run" and e.end == e.start):
            return (e.kind, e.fp, e.path, e.start)
        if e.kind == "wrap":
            return (e.kind, e.fp, e.path, e.start, e.end)
        return (e.kind, e.fp, e.path)

    def take(self, e):
        pieces = e.extra.get("pieces") or ([(e.path, e.start, e.end)] if e.kind == "run" and e.end > e.start else None)
        if pieces:
            return all(self.cov.get(path, {}).get(c) == e.fp for path, s, en in pieces for c in range(s, en))
        k = self.key(e)
        if self.multi[k] > 0:
            self.multi[k] -= 1
            return True
        return False

    def has_fp(self, e):
        if e.kind == "run" and e.end > e.start:
            return any(e.fp in d.values() for d in self.cov.values())
        return any(k[1] == e.fp and k[0] == e.kind and v > 0 for k, v in self.multi.items())


def score(doc, expected, got, observed_doc, touched=()):
    """Classify every non-trivial remainder entry: landed / orphaned / lost (misplaced or dropped)."""
    obs = Observed(observed_doc)
    tally = Counter()
    examples = []
    order = sorted(doc.entries, key=lambda e: (0 if e.kind in ("run",) else 1, e.id))
    for e in order:
        if e.kind not in COUNTED or not e.fp:
            continue
        x, g = expected[e.id], got[e.id]
        if x.reason == "lenient":  # paragraph properties after a split/merge: no single right answer
            tally["excluded"] += 1
            continue
        if x.status == "placed":
            if obs.take(x.placed):
                res = "landed"
            elif g.status == "orphan":
                res = "orphaned"
            elif g.status == "removed":
                res = "lost:dropped"
            else:
                res = "lost:misplaced" if obs.has_fp(x.placed) else "lost:missing"
        else:  # the oracle removes it with its text
            if g.status == "removed":
                res = "landed"
                tally["removed-with-text"] += 1
            elif g.status == "orphan":
                res = "orphaned"
            else:
                res = "lost:misplaced"
        tally[res] += 1
        if e.path and e.path[0] in touched:
            tally["touched:" + res.split(":")[0]] += 1
        tally["kind:%s:%s" % (e.kind, res.split(":")[0])] += 1
        if res.startswith("lost") and len(examples) < 6:
            examples.append("%s %s @%s[%s,%s] → expected %s, design %s (%s)" % (
                res, e.kind, e.path, e.start, e.end,
                (x.placed.path, x.placed.start, x.placed.end) if x.placed else x.status,
                (g.placed.path, g.placed.start, g.placed.end) if g.placed else g.status, g.reason))
    return tally, examples
