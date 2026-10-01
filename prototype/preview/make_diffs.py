#!/usr/bin/env python3
"""make_diffs.py -> diffs/<engine>-NN-pK.png: native raster (left) | engine render (right), downscaled.
Kept few and small on purpose: the native rasters show glyphs of the fonts the native PDFs embed."""
import os
from PIL import Image
HERE = os.path.dirname(os.path.abspath(__file__))
PICKS = [('rhwp', '25', 1), ('rhwp', '25', 5), ('rhwp', '28', 1), ('rdocx', '04', 1), ('rdocx', '03', 2), ('rpptx', '10', 2)]
os.makedirs(os.path.join(HERE, 'diffs'), exist_ok=True)
for eng, nn, k in PICKS:
    a = Image.open(os.path.join(HERE, 'native-png', f'{nn}-page-{k}.png')).convert('RGB')
    b = Image.open(os.path.join(HERE, 'renders', eng, f'{nn}-page-{k}.png')).convert('RGB').resize(a.size)
    w, h = a.size
    o = Image.new('RGB', (2 * w + 8, h), (160, 160, 160)); o.paste(a, (0, 0)); o.paste(b, (w + 8, 0))
    s = 900 / o.width
    o = o.resize((900, int(o.height * s)), Image.LANCZOS).quantize(colors=64)
    p = os.path.join(HERE, 'diffs', f'{eng}-{nn}-p{k}.png'); o.save(p, optimize=True)
    print(p, os.path.getsize(p) // 1024, 'KB')
