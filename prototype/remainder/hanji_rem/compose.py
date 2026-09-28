"""E10: every scripted edit in one revision (what a whole-file rewrite looks like to
the system). The oracle is composed step by step; the designs see only the
original and the final text."""
import itertools
from dataclasses import replace

from . import model as M
from .anchoring import Outcome, place
from .edits import ALL


def combined(doc):
    cur = doc
    origin = {e.id: e.id for e in doc.entries}
    ids = itertools.count(10 ** 6)
    gone = {}
    steps = []
    for fn in ALL:
        ed = fn(cur)
        if ed is None:
            continue
        text = M.serialize(ed.blocks, doc.keeps, doc.styles)
        written = M.parse(text, doc.keeps, doc.styles)
        outs = place(cur, written, "B", ed.bmap, ed.true_ops, ed.xmap, ed.lenient)
        nxt = []
        for eid, o in outs.items():
            oid = origin[eid]
            if o.reason == "lenient":
                gone[oid] = "lenient"
            if o.status != "placed":
                gone.setdefault(oid, o.status)
                continue
            pieces = o.placed.extra.get("pieces")
            if not pieces:
                nxt.append(o.placed)
                continue
            for k, (path, s, en) in enumerate(pieces):
                x = o.placed.clone(path=path, start=s, end=en)
                x.extra.pop("pieces", None)
                if k:
                    x.id = next(ids)
                    origin[x.id] = oid
                nxt.append(x)
        cur = replace(cur, blocks=written, entries=nxt)
        steps.append("%s (%s)" % (ed.name, ed.what))
    expected = {}
    live = {}
    for e in cur.entries:
        live.setdefault(origin[e.id], []).append(e)
    for e in doc.entries:
        if gone.get(e.id) == "lenient":
            expected[e.id] = Outcome(e, "removed", reason="lenient")
        elif e.id in live:
            parts = live[e.id]
            p0 = parts[0].clone()
            if len(parts) > 1:
                p0.extra["pieces"] = [(x.path, x.start, x.end) for x in parts]
            expected[e.id] = Outcome(e, "placed", p0)
        else:
            expected[e.id] = Outcome(e, gone.get(e.id, "removed"))
    return cur.blocks, expected, steps
