#!/usr/bin/env python3
"""calibrate.py -> results/calibration.json (+ a markdown table on stdout)

Calibration of layout_score.py (SPIKE.md "Calibration"):
  self / blank / shifted   the native PDFs against themselves, against empty pages, and with every
                           glyph moved by 1 px, 2 px (96 DPI), 5 pt and 10 pt
  good / bad               rhwp 25 and rdocx 04, read from results/<engine>/layout.json
  font metrics             LibreOffice with its Korean fonts swapped for Nanum (fonts/fonts-nanum.conf):
                           fontsub/nanum/*.pdf against the default LibreOffice PDFs (fontsub/ref/*.pdf)
  glyphs only              rhwp 25's SVGs rasterised with Noto and with Nanum: same SVG, so the same
                           layout score; raw and content SSIM against Hancom's pages change
Steps whose inputs are missing are skipped.
"""
import os, sys, json, glob, subprocess, math
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import layout_score as ls

out = {}
nat = ls.find_native(os.path.join(HERE, 'native'))
docs = {nn: ls.pdf_glyphs(p, {} if os.path.basename(p).split('-')[1] == 'docx' else None) for nn, p in sorted(nat.items())}


def variant(name, f):
    rs = []
    for nn, (pg, g, w) in docs.items():
        rs.append(ls.score_doc(pg, g, w, list(pg), f(g)))
    pages = [p['layout'] for r in rs for p in r['pages'] if p['layout'] is not None]
    out[name] = {'mean_layout': round(float(np.mean([r['layout'] for r in rs])), 4), 'min_page': round(min(pages), 4),
                 'W': round(float(np.mean([r['W'] for r in rs])), 4)}


if docs:
    variant('self', lambda g: [dict(x) for x in g])
    variant('blank', lambda g: [])
    for nm, dx, dy in (('shift_1px', .75, .75), ('shift_2px', 1.5, 1.5), ('shift_5pt', 5, 5), ('shift_5pt_x', 5, 0), ('shift_10pt', 10, 10)):
        variant(nm, lambda g, dx=dx, dy=dy: [dict(x, x=x['x'] + dx, y=x['y'] + dy) for x in g])

for eng, nn in (('rhwp', '25'), ('rdocx', '04')):
    p = os.path.join(HERE, 'results', eng, 'layout.json')
    if os.path.exists(p):
        f = [f for f in json.load(open(p))['files'] if f['file'] == nn][0]
        out[f'{eng}_{nn}'] = {k: f[k] for k in ('n_native_pages', 'n_render_pages', 'T', 'P', 'L', 'W', 'W_rel', 'layout')}
        out[f'{eng}_{nn}']['pages'] = [p['layout'] for p in f['pages']]

fs = os.path.join(HERE, 'fontsub')
if glob.glob(os.path.join(fs, 'nanum', '*.pdf')) and glob.glob(os.path.join(fs, 'ref', '*.pdf')):
    ref = ls.find_native(os.path.join(fs, 'ref'))
    for p in sorted(glob.glob(os.path.join(fs, 'nanum', '*.pdf'))):
        nn = os.path.basename(p)[:2]
        cp, cg, _ = ls.pdf_glyphs(p)
        row = {}
        for tag, src in (('vs_lo_default', ref.get(nn)), ('vs_native', nat.get(nn))):
            if src:
                npg, ng, nw = ls.pdf_glyphs(src, {} if 'docx' in src else None)
                r = ls.score_doc(npg, ng, nw, cp, cg)
                row[tag] = {k: r[k] for k in ('n_native_pages', 'n_render_pages', 'T', 'P', 'L', 'W', 'layout')}
        if nn in ref and nn in nat:
            npg, ng, nw = ls.pdf_glyphs(nat[nn], {} if 'docx' in nat[nn] else None)
            cp0, cg0, _ = ls.pdf_glyphs(ref[nn])
            r = ls.score_doc(npg, ng, nw, cp0, cg0)
            row['lo_default_vs_native'] = {k: r[k] for k in ('n_native_pages', 'n_render_pages', 'layout')}
        out[f'lo_fontsub_{nn}'] = row

# glyphs only: same rhwp SVG, two font sets
svgs = sorted(glob.glob(os.path.join(HERE, 'renders', 'rhwp', '25-page-[123].svg')))
confs = {'noto': os.path.join(HERE, 'fonts', 'fonts.conf'), 'nanum': os.path.join(HERE, 'fonts', 'fonts-nanum.conf')}
if svgs and all(os.path.exists(c) for c in confs.values()) and os.path.isdir(os.path.join(HERE, 'native-png')):
    import score
    from PIL import Image
    tmp = os.path.join(HERE, 'out', 'glyphs'); os.makedirs(tmp, exist_ok=True)
    res = {}
    for svg in svgs:
        k = int(svg.rsplit('-', 1)[1].split('.')[0])
        A = np.asarray(score.load(os.path.join(HERE, 'native-png', f'25-page-{k}.png')))
        for name, conf in confs.items():
            png = os.path.join(tmp, f'{name}-25-{k}.png')
            subprocess.run([sys.executable, '-c', 'import sys,cairosvg; cairosvg.svg2png(url=sys.argv[1], write_to=sys.argv[2], output_width=int(sys.argv[3]), output_height=int(sys.argv[4]), background_color="white")',
                            svg, png, str(A.shape[1]), str(A.shape[0])], check=True, env=dict(os.environ, FONTCONFIG_FILE=conf))
            B = np.asarray(score.load(png))
            s, sc, _ = score.metrics(A, B, (name, k))
            res.setdefault(name, {})[k] = {'raw_ssim': round(s, 3), 'content_ssim': round(sc, 3)}
    lj = os.path.join(HERE, 'results', 'rhwp', 'layout.json')
    lay = {p['page']: p['layout'] for p in [f for f in json.load(open(lj))['files'] if f['file'] == '25'][0]['pages']} if os.path.exists(lj) else {}
    out['rhwp_25_glyphs_only'] = {'layout_by_page (same SVG, both font sets)': {k: lay.get(k) for k in (1, 2, 3)}, **res}

json.dump(out, open(os.path.join(HERE, 'results', 'calibration.json'), 'w'), indent=1, ensure_ascii=False)
print(json.dumps(out, indent=1, ensure_ascii=False))
