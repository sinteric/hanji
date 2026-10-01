#!/usr/bin/env python3
"""Score a preview render dir against the native-app PDFs (DESIGN.md §9: per-page SSIM).

usage: score.py <render_dir> [--native native-png-dir] [--no-diffs]

Round 1's metric, kept as the secondary signal (content-SSIM) next to layout_score.py.

render_dir holds NN-page-K.png or NN-page-K.svg (K from 1); PNG wins if both exist. Writes into render_dir:
  score.json        full result (per file, per page, per format, overall)
  score_pages.csv   one row per page
  score_files.csv   one row per file
  diffs/NN-worst-pK.png  native | render | abs-diff for the worst page of each file

Secondary metrics (because a blank white page already scores ~0.87 raw SSIM against these docs):
  ssim_content = mean of the SSIM map over pixels where either image has ink (gray<245), dilated 9px
  ssim_norm    = (ssim - ssim(native, blank)) / (1 - ssim(native, blank)), clipped to [0,1]
Rules: grayscale; render resized to the native page's pixel size (aspect mismatch >2% is
recorded, still compared); SSIM via skimage, data_range=255; unmatched pages score 0;
file score = mean over max(n_native, n_render) pages. A file with no render pages at all is
'no_render': it scores 0 if the engine rendered other files of that format, and is left out
of the format entirely if the engine rendered nothing of that format (format unsupported).
"""
import sys, os, re, glob, json, csv, io, argparse
import numpy as np
from PIL import Image, ImageOps
from skimage.metrics import structural_similarity
from scipy.ndimage import maximum_filter

INK = 245  # gray < INK counts as content
_blank = {}


def metrics(A, B, key):
    """raw SSIM (spec), content-masked SSIM, blank-normalised SSIM."""
    s, S = structural_similarity(A, B, data_range=255, full=True)
    m = maximum_filter((A < INK) | (B < INK), size=9)
    sc = float(S[m].mean()) if m.any() else 1.0
    if key not in _blank:
        _blank[key] = float(structural_similarity(A, np.full_like(A, 255), data_range=255))
    b = _blank[key]
    sn = 1.0 if b >= 1 else max(0.0, (s - b) / (1 - b))
    return float(s), sc, sn

HERE = os.path.dirname(os.path.abspath(__file__))
# file number -> format / name, from the inputs in baseline/ (NN-<format>-....<ext>)
FMT, NAMES = {}, {}
for p in glob.glob(os.path.join(HERE, 'baseline', '[0-9][0-9]-*.*')):
    b = os.path.basename(p)
    FMT[b[:2]] = b.split('-')[1]
    NAMES[b[:2]] = b.rsplit('.', 1)[0]


def pages(d):
    out = {}
    for p in os.listdir(d):
        m = re.fullmatch(r'(\d\d)-page-(\d+)\.(png|svg)', p)
        if m:  # prefer PNG when both NN-page-K.png and .svg exist
            cur = out.setdefault(m[1], {}).get(int(m[2]))
            if cur is None or (m[3] == 'png' and cur.endswith('.svg')):
                out[m[1]][int(m[2])] = os.path.join(d, p)
    return out


def load(path, size=None):
    if path.endswith('.svg'):
        import cairosvg
        kw = {'output_width': size[0], 'output_height': size[1]} if size else {}
        png = cairosvg.svg2png(url=path, background_color='white', **kw)
        im = Image.open(io.BytesIO(png))
    else:
        im = Image.open(path)
    if im.mode in ('RGBA', 'LA', 'P'):
        im = im.convert('RGBA')
        bg = Image.new('RGBA', im.size, (255, 255, 255, 255))
        im = Image.alpha_composite(bg, im)
    return im.convert('L')


def side_by_side(nat, ren, out):
    h, w = nat.shape
    gap = np.full((h, 8), 128, np.uint8)
    diff = 255 - np.abs(nat.astype(int) - ren.astype(int)).astype(np.uint8)
    Image.fromarray(np.hstack([nat, gap, ren, gap, diff])).save(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('render_dir')
    ap.add_argument('--native', default=os.path.join(HERE, 'native-png'))
    ap.add_argument('--no-diffs', action='store_true')
    a = ap.parse_args()
    nat, ren = pages(a.native), pages(a.render_dir)
    os.makedirs(os.path.join(a.render_dir, 'diffs'), exist_ok=True)
    fmts_rendered = {FMT[n] for n in ren if n in FMT}
    files, page_rows = [], []
    for nn in sorted(nat):
        fmt = FMT.get(nn, '?')
        if fmt not in fmts_rendered:
            continue
        np_, rp = nat[nn], ren.get(nn, {})
        n = max(len(np_), max(rp) if rp else 0)
        scores, worst = [], None
        for k in range(1, n + 1):
            row = {'file': nn, 'name': NAMES.get(nn), 'format': fmt, 'page': k,
                   'native': k in np_, 'render': k in rp, 'ssim': 0.0, 'ssim_content': 0.0, 'ssim_norm': 0.0, 'aspect_diff': None, 'note': ''}
            if k in np_ and k in rp:
                ni = load(np_[k])
                ri = load(rp[k], ni.size)
                ar_n, ar_r = ni.size[0] / ni.size[1], ri.size[0] / ri.size[1]
                row['aspect_diff'] = round(abs(ar_r - ar_n) / ar_n, 4)
                if row['aspect_diff'] > 0.02:
                    row['note'] = f'aspect mismatch native {ni.size} render {ri.size}'
                if ri.size != ni.size:
                    ri = ri.resize(ni.size, Image.LANCZOS)
                A, B = np.asarray(ni), np.asarray(ri)
                row['ssim'], row['ssim_content'], row['ssim_norm'] = metrics(A, B, (nn, k))
                pair = (A, B)
            else:
                row['note'] = 'native only' if k in np_ else 'render only'
                if k in np_:
                    A = np.asarray(load(np_[k])); B = np.full_like(A, 255)
                else:
                    B = np.asarray(load(rp[k])); A = np.full_like(B, 255)
                pair = (A, B)
            scores.append(row['ssim'])
            if worst is None or row['ssim'] < worst[0]:
                worst = (row['ssim'], k, pair)
            page_rows.append(row)
        fs = float(np.mean(scores)) if scores else 0.0
        prs = [r for r in page_rows if r['file'] == nn]
        status = 'ok' if rp else 'no_render'
        files.append({'file': nn, 'name': NAMES.get(nn), 'format': fmt, 'n_native': len(np_),
                      'n_render': len(rp), 'page_mismatch': len(np_) != len(rp), 'score': round(fs, 4),
                      'score_content': round(float(np.mean([r['ssim_content'] for r in prs])), 4),
                      'score_norm': round(float(np.mean([r['ssim_norm'] for r in prs])), 4),
                      'worst_page': worst[1] if worst else None,
                      'worst_ssim': round(worst[0], 4) if worst else None, 'status': status})
        if worst and not a.no_diffs:
            A, B = worst[2]
            if A.shape != B.shape:
                B = np.asarray(Image.fromarray(B).resize(A.shape[::-1]))
            side_by_side(A, B, os.path.join(a.render_dir, 'diffs', f'{nn}-worst-p{worst[1]}.png'))

    def agg(fs, prs):
        if not fs:
            return None
        s = [r['ssim'] for r in prs]
        return {'files': len(fs), 'mean_file_score': round(float(np.mean([f['score'] for f in fs])), 4),
                'pages': len(s), 'mean_page_ssim': round(float(np.mean(s)), 4),
                'share_pages_ge_0.90': round(float(np.mean([x >= 0.90 for x in s])), 4),
                'share_pages_ge_0.80': round(float(np.mean([x >= 0.80 for x in s])), 4),
                'mean_file_score_content': round(float(np.mean([f['score_content'] for f in fs])), 4),
                'mean_file_score_norm': round(float(np.mean([f['score_norm'] for f in fs])), 4),
                'share_pages_content_ge_0.90': round(float(np.mean([r['ssim_content'] >= 0.90 for r in prs])), 4),
                'page_count_mismatches': sum(f['page_mismatch'] for f in fs),
                'no_render': sum(f['status'] == 'no_render' for f in fs)}
    per_fmt = {fm: agg([f for f in files if f['format'] == fm], [r for r in page_rows if r['format'] == fm])
               for fm in ['docx', 'pptx', 'xlsx', 'hwpx'] if fm in fmts_rendered}
    res = {'render_dir': os.path.relpath(a.render_dir, HERE), 'overall': agg(files, page_rows),
           'per_format': per_fmt, 'files': files, 'pages': page_rows}
    json.dump(res, open(os.path.join(a.render_dir, 'score.json'), 'w'), indent=1)
    for nm, rows in (('score_pages.csv', page_rows), ('score_files.csv', files)):
        with open(os.path.join(a.render_dir, nm), 'w', newline='') as fh:
            if rows:
                w = csv.DictWriter(fh, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
    print(json.dumps({'overall': res['overall'], 'per_format': per_fmt}, indent=1))
    for f in files:
        print(f"{f['file']} {f['format']} native={f['n_native']} render={f['n_render']} score={f['score']:.4f} content={f['score_content']:.4f} norm={f['score_norm']:.4f} worst=p{f['worst_page']}:{f['worst_ssim']} {f['status']}")


if __name__ == '__main__':
    main()
