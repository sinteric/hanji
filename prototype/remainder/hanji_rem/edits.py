"""The scripted edits (§10 item 3 task list). Each picks its target in the file
automatically, applies the edit to a copy of the model blocks, and returns the
true block map and the true character opcodes for the oracle."""
import copy
import re
from dataclasses import dataclass, field

from . import model as M
from .anchoring import span_ops

VISIBLE_RPR = ("color", "sz", "highlight", "u", "shd", "strike", "caps", "vertAlign")
PREFERRED_STYLES = ["Quote", "Intense Quote", "List Paragraph", "Body Text", "Subtitle", "Title", "Note", "Quotations"]


@dataclass
class Edit:
    name: str
    what: str
    blocks: list
    bmap: dict
    true_ops: dict = field(default_factory=dict)
    target: tuple = None
    touched: set = field(default_factory=set)   # old block indices the edit is about
    xmap: dict = field(default_factory=dict)     # split/merge: old path -> [(lo, hi, new path, shift)]
    lenient: tuple = ()                          # paths whose ppr has no single right answer


def ident(n):
    return {i: i for i in range(n)}


def paras(blocks):
    for i, b in enumerate(blocks):
        if isinstance(b, M.Para):
            yield (i,), b
        elif isinstance(b, M.Table):
            for r, row in enumerate(b.rows):
                for c, cell in enumerate(row):
                    if cell.marker is None:
                        yield (i, r, c), cell.para


def get_para(blocks, path):
    b = blocks[path[0]]
    return b if len(path) == 1 else b.rows[path[1]][path[2]].para


def entries_at(doc, path):
    return [e for e in doc.entries if e.path == path]


def clean(p, s, e):
    """No placeholder, tab or break inside the span: the model never edits those."""
    return all(M.keep_num(ch) is None and ch not in "\t\n" for ch in p.chars[s:e])


def replace_span(doc, path, s, e, new, name, what):
    blocks = copy.deepcopy(doc.blocks)
    p = get_para(blocks, path)
    src = s if e > s else max(s - 1, 0)
    b = p.bold[src] if p.chars else False
    it = p.italic[src] if p.chars else False
    n = len(p.chars)
    p.chars = p.chars[:s] + new + p.chars[e:]
    p.bold = p.bold[:s] + [b] * len(new) + p.bold[e:]
    p.italic = p.italic[:s] + [it] * len(new) + p.italic[e:]
    return Edit(name, what, blocks, ident(len(blocks)), {path: span_ops(n, s, e, len(new))}, path, {path[0]})


def figure_or_word(p):
    for m in re.finditer(r"\d+", p.chars):
        if clean(p, m.start(), m.end()):
            return m.start(), m.end(), str(int(m.group()) * 10 + 5)
    for m in re.finditer(r"\w{3,}", p.chars):
        if clean(p, m.start(), m.end()) and not m.group().isdigit():
            return m.start(), m.end(), m.group() + "-2"
    return None


def e1_figure(doc):
    """Change a figure inside a paragraph that has a comment or bookmark."""
    best = None
    for path, p in paras(doc.blocks):
        ms = [e for e in entries_at(doc, path) if e.kind == "marker" and e.extra["tag"].startswith(("comment", "bookmark"))]
        if not ms:
            continue
        f = figure_or_word(p)
        if not f:
            continue
        score = (any(e.extra["tag"].startswith("comment") for e in ms), any(m.start >= f[1] for m in ms),
                 f[2].isdigit())
        if best is None or score > best[0]:
            best = (score, path, f)
    if not best:
        return None
    _, path, (s, e, new) = best
    p = get_para(doc.blocks, path)
    return replace_span(doc, path, s, e, new, "E1 figure",
                        "change %r to %r in a paragraph with a %s" % (p.chars[s:e], new,
                        "comment" if best[0][0] else "bookmark"))


def has_drawing(doc, b):
    if isinstance(b, M.KeepBlock):
        return doc.keeps[b.n].kind in ("drawing", "table") and any(
            e.extra.get("n") == b.n and (b"drawing" in _x(e) or b"pict" in _x(e)) for e in doc.entries if e.kind == "bkeep")
    ps = [b] if isinstance(b, M.Para) else [c.para for r in b.rows for c in r if c.para is not None]
    return any(doc.keeps[M.keep_num(ch)].kind == "drawing" for p in ps for ch in p.chars if M.keep_num(ch) is not None)


def _x(e):
    from lxml import etree
    return b"".join(etree.tostring(x) for x in e.payload)


def e2_insert_before_drawing(doc):
    for i, b in enumerate(doc.blocks):
        if has_drawing(doc, b):
            blocks = copy.deepcopy(doc.blocks)
            text = "Inserted paragraph before the drawing."
            blocks.insert(i, M.Para(doc.styles.default, text, [False] * len(text), [False] * len(text)))
            bmap = {k: (k if k < i else k + 1) for k in range(len(doc.blocks))}
            return Edit("E2 insert before drawing", "new paragraph before block %d" % i, blocks, bmap, {}, (i,), {i})
    return None


def visible_rpr(e):
    if e.kind != "run" or len(e.payload) < 2:
        return False
    names = {c.tag.split("}")[1] for c in e.payload[1] if isinstance(c.tag, str)}
    return bool(names & set(VISIBLE_RPR))


def e3_delete_formatted(doc):
    for i, b in enumerate(doc.blocks):
        if not isinstance(b, M.Para) or not b.chars:
            continue
        ents = entries_at(doc, (i,))
        ppr = [e for e in ents if e.kind == "ppr"]
        if ppr and len(ppr[0].payload) > 1 and ppr[0].payload[1].find(".//{*}sectPr") is not None:
            continue
        if any(visible_rpr(e) for e in ents):
            blocks = copy.deepcopy(doc.blocks)
            del blocks[i]
            bmap = {k: (k if k < i else (None if k == i else k - 1)) for k in range(len(doc.blocks))}
            return Edit("E3 delete formatted paragraph", "delete block %d (%r…)" % (i, b.chars[:30]), blocks, bmap, {}, (i,), {i})
    return None


def e4_move_section(doc):
    styles = doc.styles
    heads = [(i, styles.level(b.style)) for i, b in enumerate(doc.blocks)
             if isinstance(b, M.Para) and b.chars and styles.level(b.style)]
    n = len(doc.blocks)
    # the final paragraph often carries the document's last section break: never move past it
    secs = []
    for k, (i, lv) in enumerate(heads):
        end = next((j for j, l2 in heads[k + 1:] if l2 <= lv), n)
        secs.append((i, end, lv))
    pair = None
    for a in secs:
        for b in secs:
            if a[1] == b[0] and a[2] == b[2]:
                pair = (a, b)
    if pair:
        (a0, a1, _), (b0, b1, _) = pair
        what = "move the section at block %d (%d blocks) before the one at block %d" % (b0, b1 - b0, a0)
    else:
        body = [i for i, b in enumerate(doc.blocks) if isinstance(b, M.Para) and b.chars]
        if len(body) < 4:
            return None
        b0, b1 = body[-3], body[-1]
        a0 = body[0]
        what = "no same-level headings: move blocks %d–%d before block %d" % (b0, b1 - 1, a0)
    order = list(range(a0)) + list(range(b0, b1)) + list(range(a0, b0)) + list(range(b1, n))
    blocks = [copy.deepcopy(doc.blocks[k]) for k in order]
    bmap = {old: new for new, old in enumerate(order)}
    return Edit("E4 move section", what, blocks, bmap, {}, (b0,), set(range(a0, b1)))


def e5_restyle(doc):
    names = [n for sid, n in doc.styles.by_id.items() if n != doc.styles.default and not doc.styles.level(n)]
    used = {b.style for b in doc.blocks if isinstance(b, M.Para)}
    pick = next((s for s in PREFERRED_STYLES if s in names), None) or next((s for s in sorted(names) if s in used), None) \
        or (sorted(names)[0] if names else None)
    if not pick:
        return None
    for i, b in enumerate(doc.blocks):
        if isinstance(b, M.Para) and b.chars and b.style == doc.styles.default:
            ents = entries_at(doc, (i,))
            if any(e.kind in ("run", "marker") and e.fp for e in ents):
                blocks = copy.deepcopy(doc.blocks)
                blocks[i].style = pick
                return Edit("E5 restyle", "block %d → style %r" % (i, pick), blocks, ident(len(blocks)), {}, (i,), {i})
    return None


def e6_cell_next_to_merge(doc):
    fallback = None
    for i, b in enumerate(doc.blocks):
        if not isinstance(b, M.Table):
            continue
        R = b.rows
        for r, row in enumerate(R):
            for c, cell in enumerate(row):
                if cell.marker or not cell.para.chars:
                    continue
                is_anchor = (c + 1 < len(row) and row[c + 1].marker == "||") or \
                            (r + 1 < len(R) and R[r + 1][c].marker == "^^")
                nb = [(r, c - 1), (r, c + 1), (r - 1, c), (r + 1, c)]
                adj = any(0 <= y < len(R) and 0 <= x < len(R[y]) and R[y][x].marker for y, x in nb)
                if adj and not is_anchor:
                    return _cell_edit(doc, (i, r, c), "cell next to a merged cell")
                if fallback is None and (adj or is_anchor):
                    fallback = ((i, r, c), "merged anchor cell (no free neighbour)")
                elif fallback is None:
                    fallback = ((i, r, c), "no merged cell in any modelled table")
    if fallback:
        return _cell_edit(doc, *fallback)
    return None


def _cell_edit(doc, path, why):
    p = get_para(doc.blocks, path)
    for m in re.finditer(r"\d+", p.chars):
        if clean(p, m.start(), m.end()):
            new = str(int(m.group()) * 10 + 5)
            return replace_span(doc, path, m.start(), m.end(), new, "E6 table cell",
                                "%s: %r → %r in cell %s" % (why, m.group(), new, path[1:]))
    n = len(p.chars)
    return replace_span(doc, path, n, n, " (rev.)", "E6 table cell", "%s: append ' (rev.)' to cell %s" % (why, path[1:]))


def e7_overlap_run(doc):
    cands = []
    for path, p in paras(doc.blocks):
        runs = sorted([e for e in entries_at(doc, path) if e.kind == "run" and e.end > e.start], key=lambda e: e.start)
        for x, y in zip(runs, runs[1:]):
            if x.end != y.start or x.fp == y.fp or not (x.fp or y.fp):
                continue
            b = x.end
            if x.end - x.start >= 2 and y.end - y.start >= 2 and clean(p, b - 2, b + 2):
                vis = visible_rpr(x) or visible_rpr(y)
                cands.append(((len(path) == 1, vis), path, b))
    if not cands:
        return None
    cands.sort(key=lambda t: t[0], reverse=True)
    _, path, b = cands[0]
    p = get_para(doc.blocks, path)
    return replace_span(doc, path, b - 2, b + 2, "EDITED", "E7 edit across a run boundary",
                        "replace %r (straddles two differently formatted runs) with 'EDITED'" % p.chars[b - 2:b + 2])


def _has_sect(doc, i):
    ppr = [e for e in entries_at(doc, (i,)) if e.kind == "ppr"]
    return bool(ppr and len(ppr[0].payload) > 1 and ppr[0].payload[1].find(".//{*}sectPr") is not None)


def e8_split(doc):
    """Split a paragraph in two (Enter in the middle), through formatted text."""
    best = None
    for i, b in enumerate(doc.blocks):
        if not isinstance(b, M.Para) or len(b.chars) < 12:
            continue
        ents = [e for e in entries_at(doc, (i,)) if e.fp and e.kind in ("run", "marker", "wrap")]
        runs = [e for e in ents if e.kind == "run" and e.end - e.start >= 4]
        n = len(b.chars)
        for k in range(2, n - 2):
            if b.chars[k - 1] != " " or not clean(b, k - 1, k + 1):
                continue
            straddle = any(r.start < k < r.end for r in runs)
            both = any(e.start < k for e in ents) and any(e.start >= k for e in ents)
            score = (straddle and both, both, -abs(k - n // 2))
            if best is None or score > best[0]:
                best = (score, i, k)
    if not best:
        return None
    _, i, k = best
    blocks = copy.deepcopy(doc.blocks)
    p = blocks[i]
    n = len(p.chars)
    q = M.Para(p.style, p.chars[k:], p.bold[k:], p.italic[k:])
    p.chars, p.bold, p.italic = p.chars[:k], p.bold[:k], p.italic[:k]
    blocks.insert(i + 1, q)
    bmap = {x: (x if x <= i else x + 1) for x in range(len(doc.blocks))}
    xmap = {(i,): [(0, k, (i,), 0), (k, n, (i + 1,), 0)]}
    return Edit("E8 split paragraph", "split block %d at offset %d of %d" % (i, k, n), blocks, bmap, {}, (i,), {i},
                xmap, ((i,),))


def e9_merge(doc):
    """Join two paragraphs (delete the paragraph mark between them)."""
    best = None
    for i in range(len(doc.blocks) - 1):
        a, b = doc.blocks[i], doc.blocks[i + 1]
        if not (isinstance(a, M.Para) and isinstance(b, M.Para) and a.chars and b.chars) or _has_sect(doc, i):
            continue
        ents = [e for e in entries_at(doc, (i + 1,)) if e.fp and e.kind in ("run", "marker", "wrap")]
        if not ents:
            continue
        score = (a.style == b.style, any(e.kind == "marker" for e in ents), len(ents))
        if best is None or score > best[0]:
            best = (score, i)
    if not best:
        return None
    i = best[1]
    blocks = copy.deepcopy(doc.blocks)
    a, b = blocks[i], blocks[i + 1]
    n1, n2 = len(a.chars), len(b.chars)
    a.chars, a.bold, a.italic = a.chars + b.chars, a.bold + b.bold, a.italic + b.italic
    del blocks[i + 1]
    bmap = {x: (x if x <= i else (None if x == i + 1 else x - 1)) for x in range(len(doc.blocks))}
    xmap = {(i,): [(0, n1, (i,), 0)], (i + 1,): [(0, n2, (i,), n1)]}
    return Edit("E9 merge paragraphs", "join blocks %d and %d" % (i, i + 1), blocks, bmap, {}, (i,), {i, i + 1},
                xmap, ((i,), (i + 1,)))


ALL = [e1_figure, e2_insert_before_drawing, e3_delete_formatted, e4_move_section, e5_restyle,
       e6_cell_next_to_merge, e7_overlap_run, e8_split, e9_merge]
