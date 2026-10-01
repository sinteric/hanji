#!/usr/bin/env python3
"""intake.py: page count, page size and embedded fonts of each native PDF -> results/native_intake.json"""
import pymupdf, glob, os, re, json
HERE = os.path.dirname(os.path.abspath(__file__))
out = {}
for p in sorted(glob.glob(os.path.join(HERE, 'native', '**', '[0-9][0-9]-*.pdf'), recursive=True)):
    d = pymupdf.open(p); fonts = set()
    for pg in d:
        for f in pg.get_fonts(full=True):
            fonts.add(re.sub(r'^[A-Z]{6}\+', '', f[3]))
    r = d[0].rect
    out[os.path.basename(p)] = {'pages': d.page_count, 'size_pt': [round(r.width, 1), round(r.height, 1)], 'fonts': sorted(fonts)}
    print(os.path.basename(p)[:2], d.page_count, f"{r.width:.0f}x{r.height:.0f}", ', '.join(sorted(fonts)))
os.makedirs(os.path.join(HERE, 'results'), exist_ok=True)
json.dump(out, open(os.path.join(HERE, 'results', 'native_intake.json'), 'w'), indent=1, ensure_ascii=False)
