#!/usr/bin/env python3
"""Layout fidelity of a preview render against the native-app PDFs (DESIGN.md §9 rule 5).

usage: layout_score.py <candidate> [--native DIR] [--out DIR] [--files 01,02] [--shift DX,DY] [--blank]

<candidate> is one of
  - a render dir holding NN-page-K.svg (engine output; rdocx/rpptx/rhwp),
  - a dir holding NN-*.pdf (e.g. LibreOffice PDFs),
  - the word `native` (the native PDFs against themselves; with --shift/--blank for calibration).
--native  dir searched recursively for NN-*.pdf from Word/PowerPoint/Excel/Hancom (default ./native).
--out     where layout.json / layout_files.csv / layout_pages.csv go (default: <candidate>/layout,
          or ./results/native-<variant> for `native`).

Method (see SPIKE.md):
 1. Glyphs. Every source becomes a list of glyphs (char, page, origin x, baseline y, font size) in
    page-relative pt, top-left origin. PDFs: pymupdf rawdict char origins. SVG: every <text>
    element, its x/y lists (or textLength spread), with all ancestor transforms applied and the
    root viewBox scaled to pt. Text is NFKC-normalised, whitespace dropped, and every bullet or
    list glyph (incl. Symbol-font PUA bullets) becomes the token '•'.
 2. Words come from the native PDF (pymupdf word segmentation: whitespace and line ends).
    Candidates are not segmented: the alignment runs over characters, so a candidate that draws
    one <text> per glyph, or a PDF that drops spaces, aligns the same way.
 3. Alignment. Whole-document LCS (rapidfuzz Indel) of the native and candidate char streams;
    an equal run is kept if it is >= 2 chars or covers a whole native word. Then two more LCS
    passes over the leftovers (runs >= 4) pick up blocks emitted in a different order
    (headers, footers, text boxes).
 4. Lines. Per page, glyphs are banded by baseline (gap <= 0.3 em) and each band is split where
    the gap between glyph origins exceeds 2.5 em. The same rule runs on native and candidate.
    A word is a line start when its first glyph starts a line.
 5. Submetrics (0..1):
    T  text   = char F1 = 2*matched / (native chars + candidate chars)
    P  page   = matched native words (>= half their chars matched) whose counterpart is on the same page
    L  lines  = F1 of line-start flags over matched words whose first char is matched
    W  where  = mean over same-page matched words of 1 - min(1, d / (2% of page diagonal)),
                d = distance between the centroids of the matched glyph origins
    F  font   = PROPOSED, not in `layout` yet: share of matched chars drawn in the same font family as
                the native PDF (subset prefix and style suffix stripped, no aliasing). Computed only when
                the candidate names its drawn fonts (PDF candidates); SVG engines must report resolved faces.
    (W_rel, diagnostic only: W after removing each page's median offset; high W_rel with low W
     means the page is shifted as a whole, low W_rel means the layout itself differs)
    layout    = (T*P*L*W) ** (1/4)            (per file, over the whole document)
 Baseline guard: a native page printed in Word's markup view (grey comment pane on the right,
 page shrunk) has the pane's text dropped and is mapped back onto the full page with the scale and
 offset the pane implies (baseline_note says so). Re-export such PDFs with Review > No Markup.
    page k    = (T_k*L_k*W_k) ** (1/3), T_k counting only matches that stay on page k
"""
import sys, os, re, glob, json, csv, math, argparse, unicodedata
import xml.etree.ElementTree as ET
import numpy as np
import pymupdf
from rapidfuzz.distance import Indel

HERE = os.path.dirname(os.path.abspath(__file__))
BULLETS = set('•◦▪▫■□●○◆◇►▶➢➤✓✔⁃∙·‣⦁⦾⦿') | {chr(c) for c in (0xF0B7, 0xF0A7, 0xF076, 0xF0D8, 0xF0FC, 0xF06E, 0xF0A8, 0xF0E0, 0xF0B2, 0xF02D)}
SVG_NS = '{http://www.w3.org/2000/svg}'


def norm_chars(s):
    out = []
    for ch in s:
        if ch in BULLETS:
            out.append('•'); continue
        n = unicodedata.normalize('NFKC', ch)
        for c in n:
            if c.isspace() or unicodedata.category(c) in ('Cc', 'Cf', 'Zs'):
                continue
            out.append('•' if c in BULLETS else c)
    return out


def is_wide(c):
    return unicodedata.east_asian_width(c) in 'WF'


# ---------------------------------------------------------------- glyph sources
def font_family(name):
    """'ABCDEF+TimesNewRomanPS-BoldMT' -> 'timesnewroman' (identity check only, no aliasing)"""
    n = re.sub(r'^[A-Z]{6}\+', '', name or '')
    n = re.split(r'[-,]', n)[0]
    n = re.sub(r'(PSMT|PS|MT|Regular|Bold|Italic|Oblique|Light|Medium|Roman|Extra|Semi|Demi|Black|Thin)+$', '', n)
    return re.sub(r'[\s_]', '', n).lower()


def markup_pane(pg):
    """Word 'markup' print view: page shrunk to the left, a grey (0.949) comment pane on the right."""
    for dr in pg.get_drawings():
        f, r = dr.get('fill'), dr['rect']
        if f and all(abs(c - 0.949) < 0.01 for c in f) and r.height > 0.5 * pg.rect.height and r.x1 > 0.9 * pg.rect.width and r.width > 0.2 * pg.rect.width:
            return r
    return None


def pdf_glyphs(path, flags=None):
    """-> pages: list of (w, h), glyphs: list of dicts, words: list of lists of glyph indices.
    flags (dict, optional) receives {page: pane rect} for pages printed in Word's markup view.
    On those pages the glyphs inside the pane (comment balloons) are dropped and the rest are mapped
    back onto the full page: Word shrinks the page by s = pane height / sheet height and puts its
    top at the pane's top (checked against LibreOffice: fitted scale within 0.3%, offset < 2 pt)."""
    d = pymupdf.open(path)
    pages, glyphs, words = [], [], []
    for pno, pg in enumerate(d, 1):
        r = pg.rect
        pages.append((r.width, r.height))
        pane = markup_pane(pg) if flags is not None else None
        if pane is not None:
            flags[pno] = pane
        raw = pg.get_text('rawdict', flags=pymupdf.TEXT_PRESERVE_WHITESPACE | pymupdf.TEXT_PRESERVE_LIGATURES)
        for b in raw['blocks']:
            for ln in b.get('lines', []):
                cur = []
                for sp in ln['spans']:
                    fs = sp['size']; fam = font_family(sp['font'])
                    for ch in sp['chars']:
                        if pane is not None and ch['origin'][0] >= pane.x0:
                            continue
                        cs = norm_chars(ch['c'])
                        if not cs:
                            if cur: words.append(cur); cur = []
                            continue
                        for c in cs:
                            cur.append(len(glyphs))
                            x, y, f_ = ch['origin'][0] - r.x0, ch['origin'][1] - r.y0, fs
                            if pane is not None:
                                k_ = pane.height / r.height
                                x, y, f_ = x / k_, (y - pane.y0) / k_, fs / k_
                            glyphs.append({'c': c, 'p': pno, 'x': x, 'y': y, 'fs': f_, 'font': fam})
                if cur: words.append(cur)
    return pages, glyphs, words


def parse_len(v):
    if v is None:
        return None
    m = re.match(r'\s*([-+0-9.eE]+)\s*(px|pt|mm|cm|in)?', v)
    if not m:
        return None
    x, u = float(m[1]), m[2] or 'px'
    return x * {'px': 0.75, 'pt': 1.0, 'mm': 72 / 25.4, 'cm': 72 / 2.54, 'in': 72.0}[u]  # -> pt


def parse_transform(s):
    M = np.eye(3)
    for name, args in re.findall(r'(\w+)\s*\(([^)]*)\)', s or ''):
        a = [float(v) for v in re.split(r'[\s,]+', args.strip()) if v]
        if name == 'matrix':
            T = np.array([[a[0], a[2], a[4]], [a[1], a[3], a[5]], [0, 0, 1]])
        elif name == 'translate':
            T = np.array([[1, 0, a[0]], [0, 1, a[1] if len(a) > 1 else 0], [0, 0, 1]])
        elif name == 'scale':
            T = np.diag([a[0], a[1] if len(a) > 1 else a[0], 1])
        elif name == 'rotate':
            t = math.radians(a[0]); c, s_ = math.cos(t), math.sin(t)
            T = np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])
            if len(a) == 3:
                T = np.array([[1, 0, a[1]], [0, 1, a[2]], [0, 0, 1]]) @ T @ np.array([[1, 0, -a[1]], [0, 1, -a[2]], [0, 0, 1]])
        else:
            continue
        M = M @ T
    return M


def floats(v):
    return [float(t) for t in re.split(r'[\s,]+', v.strip()) if t] if v else []


def svg_page(path, pno):
    s = open(path, encoding='utf-8').read()
    s = re.sub(r'<style[^>]*>.*?</style>', '', s, flags=re.S)  # drops base64 @font-face blobs
    root = ET.fromstring(s)
    vb = floats(root.get('viewBox'))
    wpt, hpt = parse_len(root.get('width')), parse_len(root.get('height'))
    if vb:
        sx = (wpt / vb[2]) if wpt else 1.0
        sy = (hpt / vb[3]) if hpt else sx
        base = np.array([[sx, 0, -vb[0] * sx], [0, sy, -vb[1] * sy], [0, 0, 1]])
        size = (vb[2] * sx, vb[3] * sy)
    else:
        base = np.diag([0.75, 0.75, 1]); size = (wpt, hpt)
    glyphs = []

    def walk(el, M):
        M = M @ parse_transform(el.get('transform'))
        tag = el.tag.replace(SVG_NS, '')
        if tag in ('defs', 'clipPath', 'mask', 'symbol', 'style'):
            return
        if tag == 'text':
            emit(el, M); return
        for ch in el:
            walk(ch, M)

    def emit(el, M):
        txt = ''.join(el.itertext())
        if el.get('fill') == 'none' or not txt.strip():
            return
        fs = float(el.get('font-size') or 12)
        xs, ys = floats(el.get('x')) or [0.0], floats(el.get('y')) or [0.0]
        tl = el.get('textLength')
        n = len(txt)
        pos = []
        for i, ch in enumerate(txt):
            if i < len(xs):
                x = xs[i]
            elif tl and len(xs) == 1:
                x = xs[0] + float(tl) * i / n
            else:
                x = pos[-1][0] + fs * (1.0 if is_wide(txt[i - 1]) else 0.55)
            y = ys[i] if i < len(ys) else ys[-1]
            pos.append((x, y))
        scale = math.sqrt(abs(M[0, 0] * M[1, 1] - M[0, 1] * M[1, 0]))
        for (x, y), ch in zip(pos, txt):
            for c in norm_chars(ch):
                X, Y, _ = M @ np.array([x, y, 1.0])
                glyphs.append({'c': c, 'p': pno, 'x': float(X), 'y': float(Y), 'fs': fs * scale})

    walk(root, base)
    return size, glyphs


def svg_glyphs(paths):
    pages, glyphs = [], []
    for k, p in enumerate(paths, 1):
        size, g = svg_page(p, k)
        pages.append(size); glyphs += g
    return pages, glyphs


# ---------------------------------------------------------------- lines
def line_starts(glyphs):
    """set of glyph indices that start a line (per page: baseline bands, split on 2.5 em gaps)"""
    starts = set()
    by_page = {}
    for i, g in enumerate(glyphs):
        by_page.setdefault(g['p'], []).append(i)
    for idx in by_page.values():
        idx.sort(key=lambda i: (glyphs[i]['y'], glyphs[i]['x']))
        bands, cur = [], [idx[0]]
        for a, b in zip(idx, idx[1:]):
            ga, gb = glyphs[a], glyphs[b]
            if gb['y'] - ga['y'] > 0.3 * max(ga['fs'], gb['fs'], 1):
                bands.append(cur); cur = []
            cur.append(b)
        bands.append(cur)
        for band in bands:
            band.sort(key=lambda i: glyphs[i]['x'])
            starts.add(band[0])
            for a, b in zip(band, band[1:]):
                if glyphs[b]['x'] - glyphs[a]['x'] > 2.5 * max(glyphs[a]['fs'], glyphs[b]['fs'], 1):
                    starts.add(b)
    return starts


# ---------------------------------------------------------------- alignment
def align(nat, cand, nat_words):
    """-> dict native glyph index -> candidate glyph index"""
    a = ''.join(g['c'] for g in nat)
    b = ''.join(g['c'] for g in cand)
    word_of = {}
    for wi, w in enumerate(nat_words):
        for gi in w:
            word_of[gi] = wi
    m = {}

    def run(ia, ib, min_len, whole_word_ok):
        sa = ''.join(a[i] for i in ia); sb = ''.join(b[j] for j in ib)
        for op in Indel.opcodes(sa, sb):
            if op.tag != 'equal':
                continue
            L = op.src_end - op.src_start
            keep = L >= min_len
            if not keep and whole_word_ok:
                gis = [ia[k] for k in range(op.src_start, op.src_end)]
                w = nat_words[word_of[gis[0]]] if gis[0] in word_of else []
                keep = len(w) == L and set(w) == set(gis)
            if keep:
                for k in range(L):
                    m[ia[op.src_start + k]] = ib[op.dest_start + k]

    run(list(range(len(a))), list(range(len(b))), 2, True)
    for _ in range(2):
        used = set(m.values())
        ia = [i for i in range(len(a)) if i not in m]
        ib = [j for j in range(len(b)) if j not in used]
        if not ia or not ib:
            break
        before = len(m)
        run(ia, ib, 4, False)
        if len(m) == before:
            break
    return m


# ---------------------------------------------------------------- scoring
def gmean(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return None
    if min(xs) <= 0:
        return 0.0
    return float(math.exp(sum(math.log(x) for x in xs) / len(xs)))


def score_doc(npages, nat, words, cpages, cand):
    m = align(nat, cand, words)
    ns, cs = line_starts(nat) if nat else set(), line_starts(cand) if cand else set()
    diag = {k: math.hypot(*npages[k - 1]) for k in range(1, len(npages) + 1)}
    per_page = {k: {'n_nat': 0, 'n_cand': 0, 'm_same': 0, 'ls': [0, 0, 0], 'w': []} for k in range(1, max(len(npages), len(cpages)) + 1)}
    for g in nat:
        per_page[g['p']]['n_nat'] += 1
    for g in cand:
        per_page[g['p']]['n_cand'] += 1
    for i, j in m.items():
        if nat[i]['p'] == cand[j]['p']:
            per_page[nat[i]['p']]['m_same'] += 1
    off = {}  # per-page median offset of matched glyphs (diagnostic: uniform shift vs local disorder)
    for k in per_page:
        d = [(cand[j]['x'] - nat[i]['x'], cand[j]['y'] - nat[i]['y']) for i, j in m.items() if nat[i]['p'] == k and cand[j]['p'] == k]
        off[k] = tuple(np.median(np.array(d), 0)) if d else (0.0, 0.0)
    nw = len(words)
    matched_w = same_page = 0
    wrel = []
    tp = fn = fp = 0
    wscores = []
    for w in words:
        mm = [i for i in w if i in m]
        if len(mm) * 2 < len(w):
            continue
        matched_w += 1
        pn = nat[w[0]]['p']
        cps = [cand[m[i]]['p'] for i in mm]
        cp = max(set(cps), key=cps.count)
        pp = per_page[pn]
        if w[0] in m:
            a_, b_ = w[0] in ns, m[w[0]] in cs
            if a_ and b_: tp += 1; pp['ls'][0] += 1
            elif a_: fn += 1; pp['ls'][1] += 1
            elif b_: fp += 1; pp['ls'][2] += 1
        if cp != pn:
            continue
        same_page += 1
        sel = [i for i in mm if cand[m[i]]['p'] == pn]
        nx = np.mean([nat[i]['x'] for i in sel]); ny = np.mean([nat[i]['y'] for i in sel])
        cx = np.mean([cand[m[i]]['x'] for i in sel]); cy = np.mean([cand[m[i]]['y'] for i in sel])
        d = math.hypot(nx - cx, ny - cy)
        ws = 1 - min(1.0, d / (0.02 * diag[pn]))
        wscores.append(ws); pp['w'].append(ws)
        dr = math.hypot(nx - cx + off[pn][0], ny - cy + off[pn][1])
        wr = 1 - min(1.0, dr / (0.02 * diag[pn])); wrel.append(wr); pp.setdefault('wr', []).append(wr)
    N, C, M = len(nat), len(cand), len(m)
    fm = [(nat[i].get('font'), cand[j].get('font')) for i, j in m.items()]
    F = (sum(a == b for a, b in fm) / len(fm)) if fm and all(b for _, b in fm) else None
    T = 2 * M / (N + C) if N + C else None
    P = same_page / matched_w if matched_w else (0.0 if N else None)
    L = 2 * tp / (2 * tp + fn + fp) if (tp + fn + fp) else (0.0 if N else None)
    W = float(np.mean(wscores)) if wscores else (0.0 if N else None)
    pages = []
    for k, pp in sorted(per_page.items()):
        n, c, ms = pp['n_nat'], pp['n_cand'], pp['m_same']
        t = 2 * ms / (n + c) if n + c else None
        a_, b_, cc = pp['ls']
        l = 2 * a_ / (2 * a_ + b_ + cc) if (a_ + b_ + cc) else (None if not n else 0.0)
        w_ = float(np.mean(pp['w'])) if pp['w'] else (None if not n else 0.0)
        sc = gmean([t, l, w_]) if t is not None else None
        pages.append({'page': k, 'native': k <= len(npages), 'render': k <= len(cpages), 'n_native_chars': n, 'n_render_chars': c,
                      'T': rnd(t), 'L': rnd(l), 'W': rnd(w_), 'layout': rnd(sc),
                      'W_rel': rnd(np.mean(pp['wr'])) if pp.get('wr') else None, 'offset_pt': [round(float(v), 1) for v in off[k]]})
    return {'n_native_pages': len(npages), 'n_render_pages': len(cpages), 'page_diff': len(cpages) - len(npages),
            'native_chars': N, 'render_chars': C, 'matched_chars': M, 'native_words': nw, 'matched_words': matched_w,
            'word_recall': rnd(matched_w / nw if nw else None),
            'T': rnd(T), 'P': rnd(P), 'L': rnd(L), 'W': rnd(W), 'W_rel': rnd(np.mean(wrel)) if wrel else None, 'F': rnd(F), 'layout': rnd(gmean([T, P, L, W]) if N else None),
            'pages': pages}


def rnd(x):
    return None if x is None else round(float(x), 4)


# ---------------------------------------------------------------- driver
def find_native(d):
    out = {}
    for p in glob.glob(os.path.join(d, '**', '[0-9][0-9]-*.pdf'), recursive=True):
        b = os.path.basename(p)
        out[b[:2]] = p
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('candidate')
    ap.add_argument('--native', default=os.path.join(HERE, 'native'))
    ap.add_argument('--out')
    ap.add_argument('--files')
    ap.add_argument('--shift', help='DX,DY in pt (calibration, with candidate=native)')
    ap.add_argument('--blank', action='store_true', help='calibration: candidate has the native pages but no text')
    a = ap.parse_args()
    nat_pdfs = find_native(a.native)
    if not nat_pdfs:
        sys.exit(f'no NN-*.pdf under {a.native}')
    only = set(a.files.split(',')) if a.files else None
    variant = None
    if a.candidate == 'native':
        variant = 'blank' if a.blank else (f'shift{a.shift}' if a.shift else 'self')
        out = a.out or os.path.join(HERE, 'results', f'native-{variant}')
    else:
        out = a.out or os.path.join(a.candidate, 'layout')
    files = []
    svgs, cpdfs = {}, {}
    if variant is None:
        for p in os.listdir(a.candidate):
            mm = re.fullmatch(r'(\d\d)-page-(\d+)\.svg', p)
            if mm:
                svgs.setdefault(mm[1], {})[int(mm[2])] = os.path.join(a.candidate, p)
            elif re.fullmatch(r'\d\d-.*\.pdf', p):
                cpdfs[p[:2]] = os.path.join(a.candidate, p)
    fmts_rendered = {os.path.basename(nat_pdfs[n]).split('-')[1] for n in list(svgs) + list(cpdfs) if n in nat_pdfs}
    for nn in sorted(nat_pdfs):
        if only and nn not in only:
            continue
        name = os.path.basename(nat_pdfs[nn])[:-4]
        fmt = name.split('-')[1]
        markup = {}
        npages, nat, words = pdf_glyphs(nat_pdfs[nn], markup if fmt == 'docx' else None)
        if variant:
            cpages = list(npages)
            if variant == 'blank':
                cand = []
            else:
                dx, dy = (float(v) for v in (a.shift.split(',') if a.shift else (0, 0)))
                cand = [dict(g, x=g['x'] + dx, y=g['y'] + dy) for g in nat]
        elif nn in svgs:
            cpages, cand = svg_glyphs([svgs[nn][k] for k in sorted(svgs[nn])])
        elif nn in cpdfs:
            cpages, cand, _ = pdf_glyphs(cpdfs[nn])
        elif fmt in fmts_rendered:
            cpages, cand = [], []  # engine failed on this file: scores 0
        else:
            continue  # format not handled by this candidate
        r = score_doc(npages, nat, words, cpages, cand)
        r.update({'file': nn, 'name': name, 'format': fmt, 'status': 'ok' if cpages else 'no_render',
                  'baseline_note': f'Word markup view on {len(markup)}/{len(npages)} pages: rescaled' if markup else ''})
        files.append(r)
        print(f"{nn} {fmt} pages {r['n_native_pages']}->{r['n_render_pages']} T={r['T']} P={r['P']} L={r['L']} W={r['W']} layout={r['layout']} "
              f"chars {r['native_chars']}/{r['render_chars']}/{r['matched_chars']}", flush=True)

    def agg(fs):
        sc = [f['layout'] for f in fs if f['layout'] is not None]
        pg = [p['layout'] for f in fs for p in f['pages'] if p['layout'] is not None]
        if not sc:
            return None
        return {'files': len(fs), 'mean_layout': rnd(np.mean(sc)), 'min_layout': rnd(min(sc)),
                'share_files_ge_0.85': rnd(np.mean([s >= 0.85 for s in sc])),
                **{k: rnd(np.mean([f[k] for f in fs if f[k] is not None])) for k in ('T', 'P', 'L', 'W')},
                'pages_scored': len(pg), 'share_pages_ge_0.80': rnd(np.mean([s >= 0.80 for s in pg])) if pg else None,
                'min_page': rnd(min(pg)) if pg else None,
                'page_count_exact': rnd(np.mean([f['page_diff'] == 0 for f in fs])),
                'page_mismatches': sum(f['page_diff'] != 0 for f in fs), 'no_render': sum(f['status'] == 'no_render' for f in fs)}
    res = {'candidate': a.candidate if variant is None else f'native:{variant}', 'overall': agg(files),
           'per_format': {fm: agg([f for f in files if f['format'] == fm]) for fm in ('docx', 'pptx', 'xlsx', 'hwpx') if any(f['format'] == fm for f in files)},
           'files': files}
    os.makedirs(out, exist_ok=True)
    json.dump(res, open(os.path.join(out, 'layout.json'), 'w'), indent=1, ensure_ascii=False)
    cols = ['file', 'name', 'format', 'status', 'n_native_pages', 'n_render_pages', 'page_diff', 'native_chars', 'render_chars', 'matched_chars',
            'native_words', 'matched_words', 'word_recall', 'T', 'P', 'L', 'W', 'W_rel', 'F', 'layout', 'baseline_note']
    with open(os.path.join(out, 'layout_files.csv'), 'w', newline='') as fh:
        w = csv.writer(fh); w.writerow(cols); [w.writerow([f[c] for c in cols]) for f in files]
    with open(os.path.join(out, 'layout_pages.csv'), 'w', newline='') as fh:
        w = csv.writer(fh); pc = ['page', 'native', 'render', 'n_native_chars', 'n_render_chars', 'T', 'L', 'W', 'W_rel', 'offset_pt', 'layout']
        w.writerow(['file', 'format'] + pc)
        for f in files:
            for p in f['pages']:
                w.writerow([f['file'], f['format']] + [p[c] for c in pc])
    print(json.dumps({'overall': res['overall'], 'per_format': res['per_format']}, indent=1))


if __name__ == '__main__':
    main()
