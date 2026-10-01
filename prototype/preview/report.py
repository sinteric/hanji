#!/usr/bin/env python3
"""report.py [results_dir] -> results/summary.md + results/summary.json

Merges results/<candidate>/layout.json (layout_score.py) with results/<candidate>/score.json
(score.py, round-1 SSIM; its per-file `score_content` is the content-SSIM) into per candidate x
format tables, a per-file table and the rule-5 check (thresholds below, proposed in SPIKE.md).
"""
import sys, os, json, glob
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
R = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, 'results')
CANDS = ['libreoffice', 'libreoffice-h2orestart', 'rdocx', 'rpptx', 'rhwp']
# proposed rule 5 (SPIKE.md): per corpus (= per format) and per page
# a page is good when layout >= 0.80, or, for a page with no native text, when content-SSIM >= 0.80
TH = {'corpus_mean': 0.90, 'file_min': 0.85, 'share_files': 0.80, 'page_exact': 0.90, 'page_good': 0.80, 'share_pages_good': 0.90}


def r3(x):
    return None if x is None else round(float(x), 3)


def rule5(fs):
    sc = [f['layout'] or 0.0 for f in fs]
    pg = []
    for f in fs:
        for p in f['pages']:
            if p['layout'] is not None:
                pg.append(p['layout'])
            elif p['native'] and not p['n_native_chars']:  # text-free page: content-SSIM decides
                pg.append(1.0 if (f.get('content_pages') or {}).get(p['page'], 0.0) >= TH['page_good'] else 0.0)
    c = {'mean': r3(np.mean(sc)), 'share_files_ge': r3(np.mean([s >= TH['file_min'] for s in sc])),
         'page_exact': r3(np.mean([f['page_diff'] == 0 for f in fs])),
         'share_pages_ge_080': r3(np.mean([p >= TH['page_good'] for p in pg])) if pg else 0.0,
         'min_page': r3(min(pg)) if pg else 0.0}
    c['pass'] = bool(c['mean'] >= TH['corpus_mean'] and c['share_files_ge'] >= TH['share_files'] and c['page_exact'] >= TH['page_exact']
                     and c['share_pages_ge_080'] >= TH['share_pages_good'])
    return c


def main():
    out, rows, files_rows = {}, [], []
    for cand in CANDS:
        lp = os.path.join(R, cand, 'layout.json')
        if not os.path.exists(lp):
            continue
        L = json.load(open(lp))
        sp = os.path.join(R, cand, 'score.json')
        SJ = json.load(open(sp)) if os.path.exists(sp) else {'files': [], 'pages': []}
        S = {f['file']: f for f in SJ['files']}
        for f in L['files']:
            s = S.get(f['file'], {})
            f['content_pages'] = {p['page']: p['ssim_content'] for p in SJ['pages'] if p['file'] == f['file']}
            f['content_ssim'] = s.get('score_content'); f['raw_ssim'] = s.get('score')
            files_rows.append((cand, f))
        for fmt in ('docx', 'pptx', 'xlsx', 'hwpx'):
            fs = [f for f in L['files'] if f['format'] == fmt]
            if not fs:
                continue
            for scope, sel in (('all', fs), ('clean', [f for f in fs if not f.get('baseline_note')])):
                if not sel or (scope == 'clean' and len(sel) == len(fs)):
                    continue
                m = lambda k: r3(np.mean([f[k] if f[k] is not None else 0.0 for f in sel]))
                row = {'candidate': cand, 'format': fmt, 'scope': scope, 'files': len(sel), 'layout': m('layout'),
                       'T': m('T'), 'P': m('P'), 'L': m('L'), 'W': m('W'), 'W_rel': m('W_rel'),
                       'content_ssim': r3(np.mean([f['content_ssim'] for f in sel if f['content_ssim'] is not None])) if any(f['content_ssim'] is not None for f in sel) else None,
                       'raw_ssim': r3(np.mean([f['raw_ssim'] for f in sel if f['raw_ssim'] is not None])) if any(f['raw_ssim'] is not None for f in sel) else None,
                       'page_mismatch': sum(f['page_diff'] != 0 for f in sel), 'no_render': sum(f['status'] == 'no_render' for f in sel),
                       'rule5': rule5(sel)}
                rows.append(row)
    out['thresholds'] = TH
    out['table'] = rows
    out['files'] = [{'candidate': c, **{k: f[k] for k in ('file', 'name', 'format', 'status', 'n_native_pages', 'n_render_pages', 'page_diff', 'T', 'P', 'L', 'W', 'W_rel', 'layout', 'content_ssim', 'raw_ssim', 'baseline_note')},
                     'worst_pages': sorted([(p['page'], p['layout'], p['offset_pt']) for p in f['pages'] if p['layout'] is not None], key=lambda t: t[1])[:2]} for c, f in files_rows]
    json.dump(out, open(os.path.join(R, 'summary.json'), 'w'), indent=1, ensure_ascii=False)
    md = ['| Candidate | Format | Scope | Files | Layout | T | P | L | W | W_rel | Content-SSIM | Raw SSIM | Page mismatch | No render | Rule 5 |',
          '|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|']
    for r in rows:
        c = r['rule5']
        md.append(f"| {r['candidate']} | {r['format']} | {r['scope']} | {r['files']} | **{r['layout']:.3f}** | {r['T']:.3f} | {r['P']:.3f} | {r['L']:.3f} | {r['W']:.3f} | {r['W_rel']:.3f} | "
                  f"{r['content_ssim'] if r['content_ssim'] is not None else '–'} | {r['raw_ssim'] if r['raw_ssim'] is not None else '–'} | {r['page_mismatch']} | {r['no_render']} | "
                  f"{'pass' if c['pass'] else 'fail'} (≥.85: {c['share_files_ge']:.0%}, pages exact {c['page_exact']:.0%}, pages ≥.80: {c['share_pages_ge_080']:.0%}) |")
    md += ['', '| Candidate | File | Pages (native→render) | T | P | L | W | W_rel | Layout | Content-SSIM | Raw SSIM | Note |', '|---|---|---|---|---|---|---|---|---|---|---|---|']
    for c, f in files_rows:
        g = lambda k: '–' if f[k] is None else f"{f[k]:.3f}"
        md.append(f"| {c} | {f['name']} | {f['n_native_pages']}→{f['n_render_pages']} | {g('T')} | {g('P')} | {g('L')} | {g('W')} | {g('W_rel')} | **{g('layout')}** | {g('content_ssim')} | {g('raw_ssim')} | {f.get('baseline_note') or ''} |")
    open(os.path.join(R, 'summary.md'), 'w').write('\n'.join(md) + '\n')
    print('\n'.join(md[:len(rows) + 2]))


if __name__ == '__main__':
    main()
