import json, sys, subprocess, glob, os
eng, native = sys.argv[1], sys.argv[2]
def npages(f):
    pdf = os.path.join(native, f.rsplit('.',1)[0] + '.pdf')
    try:
        out = subprocess.run(['pdfinfo', pdf], capture_output=True, text=True).stdout
        return int([l for l in out.splitlines() if l.startswith('Pages')][0].split()[1])
    except Exception: return '?'
print("| File | OK | Pages (native PDF) | Page px | Time s (total; open / layout or svg / png) | SVG MB/page max | `<text>` / `<path>` | Warnings (count) | Fonts |")
print("|---|---|---|---|---|---|---|---|---|")
for l in open(f"{eng}/results.jsonl"):
    r = json.loads(l)
    f = r['file']
    if not r['ok']:
        print(f"| {f} | **no** | – ({npages(f)}) | – | {r['total_s']:.2f} | – | – | `{r['error']}` | – |"); continue
    pg = r['pages']
    sizes = sorted({f"{p['w_px']}×{p['h_px']}" for p in pg})
    t = f"{r['total_s']:.2f}; " + (f"{r['open_s']:.2f} / {r['layout_s']:.2f}" if 'layout_s' in r else f"{r['open_s']:.2f} / {r['svg_s']:.2f} / {r['png_s']:.2f}")
    mb = max(p['svg_bytes'] for p in pg)/1e6
    te = sum(p['text_elems'] for p in pg); pe = sum(p['path_elems'] for p in pg)
    w = {}
    for k in ('layout_diagnostics','svg_diagnostics'):
        for m,c in r.get(k,{}).items():
            m = m.replace('resolve: ','')
            w[m] = max(w.get(m,0), c)
    if r.get('overflow_cell_lines'): w['overflow cell lines'] = r['overflow_cell_lines']
    ws = '; '.join(f"{m} ({c})" for m,c in w.items()) or '–'
    if 'fonts_used' in r:
        fams = sorted({x.replace(' bold','').replace(' italic','') for x in r['fonts_used']})
        fs = ', '.join(fams) if len(fams) < 12 else f"{len(fams)} families loaded (incl. {', '.join(fams[:4])}…)"
    else:
        fs = '; '.join(f"{k} → {v['resolved'].split(' -> ')[-1]}" for k,v in r['font_resolution'].items())
    n = r['page_count']; nn = npages(f)
    pc = f"{n} ({nn})" + ("" if n == nn else " **≠**")
    print(f"| {f} | yes | {pc} | {', '.join(sizes)} | {t} | {mb:.1f} | {te} / {pe} | {ws} | {fs} |")
