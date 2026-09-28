"""Re-anchoring the remainder after an edit.

Designs (all share block alignment and placeholder lookup; they differ in how an
entry inside an *edited* paragraph finds its new offset):

  A         block anchor + frozen offset: an entry keeps the offset it had in its
            block; offsets are clamped to the new length.
  A-strict  block anchor, refuse on change: an offset-anchored entry in a block
            whose text changed is orphaned (refused with a reason).
  B         run-range anchor, re-anchored by a character diff of the old and new
            block text (difflib), with fixed affinity rules.

The oracle is B's placement rule fed with the *true* edit (the exact span and
the true block map) instead of a diff: it states where each entry is meant to
land under the conventions below, which follow Word's own editing behaviour:
  - replaced text takes the run of the first replaced character;
  - text inserted at a run boundary joins the run on its left;
  - a range start keeps right affinity, a range end left affinity (a range
    does not grow when text is inserted exactly at its edge);
  - bookmarks, comment ranges and permission ranges in a deleted paragraph
    move to the start of the next surviving block (never vanish);
  - run formatting, paragraph properties and proofing marks go with deleted text
    (and are listed as removed).
"""
import difflib
from dataclasses import dataclass

from . import model as M
from .importer import DURABLE_MARKERS

OFFSET_KINDS = ("run", "marker", "rmarker", "wrap")


@dataclass
class Outcome:
    entry: object      # original entry
    status: str        # placed | removed | orphan
    placed: object = None
    reason: str = ""


# ------------------------------------------------------------------ signatures

def full_sig(b):
    if isinstance(b, M.Para):
        return ("P", b.style, b.chars, tuple(b.bold), tuple(b.italic))
    if isinstance(b, M.Table):
        return ("T", tuple(tuple((c.marker, None if c.marker else full_sig(c.para)) for c in r) for r in b.rows))
    return ("K", b.n)


def content(b):
    if isinstance(b, M.Para):
        return b.chars
    if isinstance(b, M.Table):
        return "\u0001".join("\u0002".join(c.marker or c.para.chars for c in r) for r in b.rows)
    return "\u0003%d" % b.n


def kind_of(b):
    return type(b).__name__


def align(old, new):
    """old block index -> new block index (or None). Diff on whole blocks, then pair
    moved blocks (identical), then pair edited blocks inside each replaced stretch."""
    osig, nsig = [full_sig(b) for b in old], [full_sig(b) for b in new]
    sm = difflib.SequenceMatcher(None, osig, nsig, autojunk=False)
    bmap, used = {}, set()
    stretches = []
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            for k in range(i2 - i1):
                bmap[i1 + k] = j1 + k
                used.add(j1 + k)
        else:
            stretches.append((list(range(i1, i2)), list(range(j1, j2))))
    # moves: identical blocks that the sequence diff saw as delete + insert
    free_new = [j for _, js in stretches for j in js]
    for olds, _ in stretches:
        for i in olds:
            for j in free_new:
                if j not in used and nsig[j] == osig[i]:
                    bmap[i] = j; used.add(j); break
    def ratio(i, j):
        return difflib.SequenceMatcher(None, content(old[i]), content(new[j]), autojunk=False).ratio()

    def pairable(i, j):
        return kind_of(old[i]) == kind_of(new[j]) and not isinstance(old[i], M.KeepBlock)

    # restyled or moved-and-restyled: same content anywhere
    for olds, _ in stretches:
        for i in olds:
            if i in bmap:
                continue
            for j in free_new:
                if j not in used and pairable(i, j) and content(old[i]) == content(new[j]):
                    bmap[i] = j; used.add(j); break
    # edits in place: inside each replaced stretch, same count -> pair in order when similar enough
    for olds, news in stretches:
        o = [i for i in olds if i not in bmap]
        n = [j for j in news if j not in used]
        if len(o) == len(n):
            for i, j in zip(o, n):
                if pairable(i, j) and ratio(i, j) >= 0.4:
                    bmap[i] = j; used.add(j)
    # anything left: best similar block anywhere (moved and edited), greedy
    left_o = [i for olds, _ in stretches for i in olds if i not in bmap]
    left_n = [j for j in free_new if j not in used]
    cands = sorted(((ratio(i, j), i, j) for i in left_o for j in left_n if pairable(i, j)), reverse=True)
    for r, i, j in cands:
        if r < 0.5:
            break
        if i not in bmap and j not in used:
            bmap[i] = j; used.add(j)
    # keep blocks are identified by their placeholder id wherever they went
    npos = {b.n: j for j, b in enumerate(new) if isinstance(b, M.KeepBlock)}
    for i, b in enumerate(old):
        if isinstance(b, M.KeepBlock):
            bmap[i] = npos.get(b.n)
    for i in range(len(old)):
        bmap.setdefault(i, None)
    return bmap


def cell_map(ot, nt):
    """(r, c) in the old table -> (r, c) in the new one, or None."""
    if len(ot.rows) == len(nt.rows) and all(len(a) == len(b) for a, b in zip(ot.rows, nt.rows)):
        return lambda r, c: (r, c)
    osig = ["\u0002".join(c.marker or c.para.chars for c in r) for r in ot.rows]
    nsig = ["\u0002".join(c.marker or c.para.chars for c in r) for r in nt.rows]
    rm = {}
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, osig, nsig, autojunk=False).get_opcodes():
        if tag == "equal" or (tag == "replace" and i2 - i1 == j2 - j1):
            for k in range(i2 - i1):
                rm[i1 + k] = j1 + k
    same_cols = len({len(r) for r in ot.rows} | {len(r) for r in nt.rows}) == 1

    def f(r, c):
        if r not in rm or not same_cols:
            return None
        return (rm[r], c)
    return f


# ------------------------------------------------------------------ char mapping

def diff_ops(a, b):
    return difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes()


def span_ops(n_old, s, e, new_len):
    """Opcodes for replacing old[s:e] with a string of length new_len."""
    ops = []
    if s > 0:
        ops.append(("equal", 0, s, 0, s))
    if e > s or new_len:
        tag = "replace" if e > s and new_len else ("delete" if e > s else "insert")
        ops.append((tag, s, e, s, s + new_len))
    if e < n_old:
        ops.append(("equal", e, n_old, s + new_len, s + new_len + n_old - e))
    return ops


def map_pos(ops, p, right):
    cands = []
    for tag, i1, i2, j1, j2 in ops:
        if not (i1 <= p <= i2):
            continue
        if tag == "equal":
            cands.append(j1 + (p - i1))
        elif tag == "insert":
            cands += [j1, j2]
        else:
            if p == i1:
                cands.append(j1)
            elif p == i2:
                cands.append(j2)
            else:
                cands.append(j2 if right else j1)
    if not cands:
        return None
    return max(cands) if right else min(cands)


def owners_after(ops, old_owner, new_len):
    """Per new char: the old char whose run it joins."""
    out = [None] * new_len
    for tag, i1, i2, j1, j2 in ops:
        for j in range(j1, j2):
            if tag == "equal":
                out[j] = (i1 + (j - j1), True)
            elif tag == "replace":
                out[j] = (i1, False)
            elif tag == "insert":
                out[j] = (i1 - 1 if i1 > 0 else i1, False)
    return out


def resolve(old_owner, k):
    """Owner of old char k; a character outside any run (a placeholder) defers to
    the nearest run on its left, then on its right."""
    if k is None or not old_owner:
        return None
    k = min(k, len(old_owner) - 1)
    for x in range(k, -1, -1):
        if old_owner[x] is not None:
            return old_owner[x]
    for x in range(k + 1, len(old_owner)):
        if old_owner[x] is not None:
            return old_owner[x]
    return None


def resolve_in(old_owner, k, lo, hi):
    """Like resolve(), bounded to one old paragraph's characters [lo, hi)."""
    if hi <= lo:
        return None
    k = max(lo, min(k, hi - 1))
    for x in list(range(k, lo - 1, -1)) + list(range(k + 1, hi)):
        if old_owner[x] is not None:
            return old_owner[x]
    return None


# ------------------------------------------------------------------ document stream (design C)

SEP = " "


def paras_of(blocks):
    """(path, Para or None) for every paragraph position in document order; block
    keeps take one slot so the stream keeps their place."""
    for j, b in enumerate(blocks):
        if isinstance(b, M.Para):
            yield (j,), b
        elif isinstance(b, M.Table):
            for r, row in enumerate(b.rows):
                for c, cell in enumerate(row):
                    if cell.marker == "||":
                        continue
                    yield (j, r, c), cell.para if cell.para is not None else M.Para("", "", [], [])
        else:
            yield (j,), None


class Stream:
    def __init__(self, items):
        """items: [(path, Para or None)] in document order."""
        self.base, self.lens, self.order, self.kind = {}, {}, [], {}
        buf = []
        for path, p in items:
            self.base[path] = len(buf)
            txt = p.chars if p is not None else "\u0003"
            self.lens[path] = len(txt)
            self.kind[path] = "para" if p is not None else "keep"
            self.order.append(path)
            buf.extend(txt)
            buf.append(SEP)
        self.text = "".join(buf)
        self.starts = [self.base[p] for p in self.order]

    def locate(self, g):
        """global gap -> (path, local); a gap in a block keep moves to the next paragraph."""
        import bisect
        k = bisect.bisect_right(self.starts, g) - 1
        k = max(k, 0)
        while k < len(self.order) and self.kind[self.order[k]] != "para":
            k += 1
        if k >= len(self.order):
            return None
        path = self.order[k]
        return path, max(0, min(g - self.base[path], self.lens[path]))

    def char_at(self, g):
        import bisect
        k = bisect.bisect_right(self.starts, g) - 1
        path = self.order[k]
        loc = g - self.base[path]
        return (path, loc) if loc < self.lens[path] and self.kind[path] == "para" else None


# ------------------------------------------------------------------ placement

class Placer:
    """Places every remainder entry of `old_doc` against `new_blocks`.

    design: 'A' | 'A-strict' | 'B' | 'C'. The oracle is 'B' with the true block map
    (`bmap`), true per-paragraph opcodes (`true_ops`) and, for split/merge, a true
    cross-paragraph map (`xmap`: old path -> [(lo, hi, new_path, shift)])."""

    def __init__(self, old_doc, new_blocks, design, bmap=None, true_ops=None, xmap=None, lenient=()):
        self.doc = old_doc
        self.new = new_blocks
        self.design = design
        self.bmap = bmap if bmap is not None else align(old_doc.blocks, new_blocks)
        self.true_ops = true_ops or {}
        self.xmap = xmap or {}
        self.lenient = set(lenient)
        self.out = {}
        self.handled = set()
        self._own = {}

    def result(self, e, status, placed=None, reason=""):
        if e.id in self.out and e.id in self.handled:
            return
        self.out[e.id] = Outcome(e, status, placed, reason)

    def next_surviving(self, i):
        for k in range(i + 1, len(self.doc.blocks)):
            if self.bmap.get(k) is not None:
                return self.bmap[k]
        return None

    def relocate(self, e, i, why):
        """A durable marker whose paragraph is gone: to the next surviving block."""
        j = self.next_surviving(i)
        if j is None:
            self.result(e, "placed", e.clone(kind="tail", path=(), start=None, end=None), why)
        else:
            self.result(e, "placed", e.clone(kind="bmarker", path=(j,), start=None, end=None), why)

    def run(self):
        by_path = {}
        for e in self.doc.entries:
            by_path.setdefault(e.path, []).append(e)
        self.by_path = by_path
        where = {}
        for j, b in enumerate(self.new):
            if isinstance(b, M.KeepBlock):
                where[b.n] = ((j,), None)
            for path, p in self.paras(b, j):
                for c, ch in enumerate(p.chars):
                    n = M.keep_num(ch)
                    if n is not None:
                        where[n] = (path, c)
        for e in self.doc.entries:
            if e.kind in ("keep", "bkeep"):
                n = e.extra["n"]
                if n in where:
                    path, c = where[n]
                    self.result(e, "placed", e.clone(path=path, start=c, end=None if c is None else c + 1))
                else:
                    self.result(e, "removed", reason="placeholder k%d deleted by the edit" % n)
            elif e.kind == "tail":
                self.result(e, "placed", e.clone())
        if self.xmap:
            self.cross_map()
        if self.design == "C":
            self.global_map()
        for i, ob in enumerate(self.doc.blocks):
            j = self.bmap.get(i)
            for e in by_path.get((i,), []):
                if e.kind == "bmarker":
                    if j is None:
                        self.relocate(e, i, "its block was deleted")
                    else:
                        self.result(e, "placed", e.clone(path=(j,)))
            if isinstance(ob, M.KeepBlock):
                continue
            if j is None:
                self.deleted_block(ob, i, by_path)
                continue
            nb = self.new[j]
            if isinstance(ob, M.Para):
                self.para(ob, (i,), nb, (j,), by_path)
            else:
                self.table(ob, i, nb, j, by_path)
        return self.out

    def paras(self, b, j):
        if isinstance(b, M.Para):
            yield (j,), b
        elif isinstance(b, M.Table):
            for r, row in enumerate(b.rows):
                for c, cell in enumerate(row):
                    if cell.para is not None:
                        yield (j, r, c), cell.para

    def offset_entries(self, path):
        return [e for e in self.by_path.get(path, []) if e.kind in OFFSET_KINDS]

    def deleted_block(self, ob, i, by_path):
        paths = [(i,)]
        if isinstance(ob, M.Table):
            paths = [p for p in by_path if len(p) >= 2 and p[0] == i] + [(i,)]
        for path in paths:
            for e in by_path.get(path, []):
                if e.kind in ("keep", "bkeep", "bmarker") or e.id in self.handled:
                    continue
                if e.kind == "ppr" and path in self.lenient:
                    self.result(e, "removed", reason="lenient")
                    self.out[e.id].reason = "lenient"
                    continue
                if e.kind == "marker" and e.extra.get("tag") in DURABLE_MARKERS:
                    self.relocate(e, i, "its paragraph was deleted")
                else:
                    self.result(e, "removed", reason="its block was deleted")

    def table(self, ot, i, nt, j, by_path):
        for e in by_path.get((i,), []):
            if e.kind == "tbl":
                self.result(e, "placed", e.clone(path=(j,)))
        cm = cell_map(ot, nt)
        for r, row in enumerate(ot.rows):
            nr = cm(r, 0)
            for e in by_path.get((i, r), []):
                if nr is None:
                    self.result(e, "orphan", reason="table rows changed shape")
                else:
                    self.result(e, "placed", e.clone(path=(j, nr[0])))
            for c, cell in enumerate(row):
                if cell.marker == "||":
                    continue
                path = (i, r, c)
                tgt = cm(r, c)
                ncell = nt.rows[tgt[0]][tgt[1]] if tgt else None
                if ncell is None or ncell.marker == "||":
                    for e in by_path.get(path, []):
                        if e.kind not in ("keep",) and e.id not in self.handled:
                            self.result(e, "orphan", reason="cell no longer exists")
                    continue
                npath = (j,) + tgt
                for e in by_path.get(path, []):
                    if e.kind == "tc":
                        self.result(e, "placed", e.clone(path=npath))
                op = cell.para if cell.para is not None else M.Para("", "", [], [])
                npara = ncell.para if ncell.para is not None else M.Para(op.style, "", [], [])
                self.para(op, path, npara, npath, by_path, skip=("tc",))

    def para(self, op, path, np_, npath, by_path, skip=()):
        ents = [e for e in by_path.get(path, []) if e.kind not in ("keep", "bkeep", "bmarker") + skip
                and e.id not in self.handled]
        same = op.chars == np_.chars
        ops = self.true_ops.get(path)
        if ops is None and not same and self.design in ("B", "C"):
            ops = diff_ops(op.chars, np_.chars)
        if same and ops is None:
            ops = [("equal", 0, len(op.chars), 0, len(op.chars))]
        for e in ents:
            if e.kind == "ppr":
                self.result(e, "placed", e.clone(path=npath))
                if path in self.lenient:
                    self.out[e.id].reason = "lenient"
            elif ops is not None:
                self.by_ops(e, ops, op, np_, path, npath)
            elif self.design == "A-strict":
                if e.kind == "run" and e.fp == "":
                    self.result(e, "removed", reason="plain run (nothing to keep)")
                else:
                    self.result(e, "orphan", reason="paragraph text changed; a block anchor cannot place it")
            else:  # A: frozen offsets
                self.frozen(e, op, np_, npath)

    def frozen(self, e, op, np_, npath):
        L = len(np_.chars)
        s = min(e.start, L)
        en = min(e.end, L) if e.end is not None else None
        if e.kind == "run" and e.end == len(op.chars):
            en = L  # the last run takes any text added at the end
        if e.kind in ("run", "wrap") and e.end > e.start and s == en:
            self.result(e, "removed", reason="its offset range fell past the end of the new text")
            return
        self.result(e, "placed", e.clone(path=npath, start=s, end=en))

    @staticmethod
    def right(e):
        return e.extra.get("tag", "").endswith("Start")

    def by_ops(self, e, ops, op, np_, path, npath):
        if e.kind in ("marker", "rmarker"):
            p = map_pos(ops, e.start, self.right(e))
            self.result(e, "placed", e.clone(path=npath, start=p, end=p))
            return
        if e.kind == "wrap":
            s, en = map_pos(ops, e.start, True), map_pos(ops, e.end, False)
            if e.end > e.start and en <= s:
                self.result(e, "removed", reason="all of its text was deleted")
                return
            self.result(e, "placed", e.clone(path=npath, start=s, end=max(s, en)))
            return
        if e.start == e.end:  # empty run: a zero-width thing
            p = map_pos(ops, e.start, False)
            self.result(e, "placed", e.clone(path=npath, start=p, end=p))
            return
        key = (path, npath)
        if key not in self._own:
            old_owner = [None] * len(op.chars)
            for x in self.by_path.get(path, []):
                if x.kind == "run" and x.end > x.start:
                    for c in range(x.start, x.end):
                        old_owner[c] = x.id
            src = owners_after(ops, old_owner, len(np_.chars))
            self._own[key] = [None if s is None else (old_owner[s[0]] if s[1] else resolve(old_owner, s[0]))
                              for s in src]
        own = self._own[key]
        self.place_run(e, [(npath, c) for c, o in enumerate(own) if o == e.id])

    def place_run(self, e, chars):
        """chars: [(path, local)] owned by run e in the new text -> one entry, maybe in pieces."""
        if not chars:
            self.result(e, "removed", reason="all of its text was deleted")
            return
        pieces = []
        for path, c in chars:
            if pieces and pieces[-1][0] == path and pieces[-1][2] == c:
                pieces[-1][2] = c + 1
            else:
                pieces.append([path, c, c + 1])
        pieces = [tuple(p) for p in pieces]
        p0 = pieces[0]
        extra = dict(e.extra)
        if len(pieces) > 1:
            extra["pieces"] = pieces
        self.result(e, "placed", e.clone(path=p0[0], start=p0[1], end=p0[2], extra=extra))

    # ---------------------------------------------------------- oracle: split / merge
    def cross_map(self):
        for path, segs in self.xmap.items():
            def mp(pos, right):
                hits = [(np_, pos - lo + sh) for lo, hi, np_, sh in segs if lo <= pos <= hi]
                return hits[-1] if right else hits[0]
            for e in self.offset_entries(path):
                self.handled.add(e.id)
                if e.kind in ("marker", "rmarker") or (e.kind == "run" and e.start == e.end):
                    np_, p = mp(e.start, self.right(e))
                    self.out[e.id] = Outcome(e, "placed", e.clone(path=np_, start=p, end=p))
                elif e.kind == "wrap":
                    (a, s), (b, en) = mp(e.start, True), mp(e.end, False)
                    if a != b:
                        self.out[e.id] = Outcome(e, "orphan", reason="a wrapper cannot span two paragraphs")
                    else:
                        self.out[e.id] = Outcome(e, "placed", e.clone(path=a, start=s, end=en))
                else:
                    chars = []
                    for c in range(e.start, e.end):
                        for lo, hi, np_, sh in segs:
                            if lo <= c < hi:
                                chars.append((np_, c - lo + sh)); break
                    self.place_run(e, chars)

    # ---------------------------------------------------------- design C
    def global_map(self):
        """Blocks the alignment matched unchanged keep their per-block anchors; every other
        body paragraph, old and new, goes into one document-level stream and is diffed as a
        whole, so text can carry its remainder across a split, a merge or a move-and-edit."""
        same_old, same_new = set(), set()
        for i, j in self.bmap.items():
            if j is not None and full_sig(self.doc.blocks[i])[:3] == full_sig(self.new[j])[:3] \
                    or (j is not None and not isinstance(self.doc.blocks[i], M.Para)):
                same_old.add(i); same_new.add(j)
        old = Stream([(p, b) for p, b in paras_of(self.doc.blocks) if len(p) == 1 and p[0] not in same_old and b is not None])
        new = Stream([(p, b) for p, b in paras_of(self.new) if len(p) == 1 and p[0] not in same_new and b is not None])
        if not old.order:
            return
        ops = diff_ops(old.text, new.text)
        survived = [False] * len(old.text)
        for tag, i1, i2, j1, j2 in ops:
            if tag == "equal" and (i2 - i1 >= 3 or i2 - i1 == len(old.text)):
                for k in range(i1, i2):
                    survived[k] = True
        old_owner = [None] * len(old.text)
        for e in self.doc.entries:
            if e.kind == "run" and e.end > e.start and e.path in old.base:
                b = old.base[e.path]
                for c in range(e.start, e.end):
                    old_owner[b + c] = e.id
        # per new char: its run
        new_owner = [None] * len(new.text)
        for tag, i1, i2, j1, j2 in ops:
            for j in range(j1, j2):
                if tag == "equal":
                    new_owner[j] = old_owner[i1 + (j - j1)]
                    continue
                src = i1 if tag == "replace" else i1 - 1
                if src < 0 or old.text[src] == SEP:
                    src = i1  # inserted at a paragraph start: join the run on the right
                loc = old.char_at(min(src, len(old.text) - 1)) if old.text else None
                if loc is None:
                    continue
                lo = old.base[loc[0]]
                new_owner[j] = resolve_in(old_owner, src, lo, lo + old.lens[loc[0]])
        owned = {}
        for j, o in enumerate(new_owner):
            if o is not None and new.text[j] != SEP:
                loc = new.char_at(j)
                if loc:
                    owned.setdefault(o, []).append(loc)
        for path in old.order:
            if old.kind[path] != "para":
                continue
            b, n = old.base[path], old.lens[path]
            if not any(survived[b:b + n]):
                continue  # wholly deleted (or moved unchanged): the per-block rules apply
            for e in self.offset_entries(path):
                self.handled.add(e.id)
                if e.kind == "run" and e.end > e.start:
                    self.place_run(e, owned.get(e.id, []))
                    continue
                if e.kind == "wrap":
                    s = new.locate(map_pos(ops, b + e.start, True))
                    en = new.locate(map_pos(ops, b + e.end, False))
                    if s is None or en is None or s[0] != en[0]:
                        self.out[e.id] = Outcome(e, "orphan", reason="its two ends now fall in different paragraphs")
                    elif e.end > e.start and en[1] <= s[1]:
                        self.out[e.id] = Outcome(e, "removed", reason="all of its text was deleted")
                    else:
                        self.out[e.id] = Outcome(e, "placed", e.clone(path=s[0], start=s[1], end=en[1]))
                    continue
                right = self.right(e)
                loc = new.locate(map_pos(ops, b + e.start, right))
                if loc is None:
                    self.out[e.id] = Outcome(e, "placed", e.clone(kind="tail", path=(), start=None, end=None))
                else:
                    self.out[e.id] = Outcome(e, "placed", e.clone(path=loc[0], start=loc[1], end=loc[1]))


def place(old_doc, new_blocks, design, bmap=None, true_ops=None, xmap=None, lenient=()):
    return Placer(old_doc, new_blocks, design, bmap, true_ops, xmap, lenient).run()


def placed_entries(outcomes):
    out = []
    for o in outcomes.values():
        if o.status != "placed":
            continue
        e = o.placed
        pieces = e.extra.get("pieces")
        if pieces:
            for k, (path, s, en) in enumerate(pieces):
                x = e.clone(path=path, start=s, end=en)
                x.extra.pop("pieces", None)
                if k:
                    x.seq = e.seq + 0.001 * k
                out.append(x)
        else:
            out.append(e)
    return out
