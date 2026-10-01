#!/usr/bin/env python3
"""raster.py <pdf> <outdir> <NN> [dpi]  -> outdir/NN-page-K.png"""
import sys, os, pymupdf
pdf, out, nn = sys.argv[1:4]; dpi = int(sys.argv[4]) if len(sys.argv) > 4 else 96
os.makedirs(out, exist_ok=True)
d = pymupdf.open(pdf)
for i, pg in enumerate(d, 1):
    pg.get_pixmap(dpi=dpi, alpha=False).save(f"{out}/{nn}-page-{i}.png")
print(nn, d.page_count)
