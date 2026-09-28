#!/usr/bin/env python3
"""Step 1: produce hanji-style *direct-change* exports with the remainder
prototype (design C), one per scripted edit E1-E9, plus E10 (all edits).

    python3 make_direct.py            # writes out/direct/<stem>/<Ek>.docx

These are what hanji exports today (no revision marks). The Rust `tcspike`
binary then turns (original, direct export) into a tracked-change file.
"""
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REM = os.path.join(HERE, "..", "remainder")
sys.path.insert(0, REM)

from hanji_rem import model as M  # noqa: E402
from hanji_rem.anchoring import place, placed_entries  # noqa: E402
from hanji_rem.compose import combined  # noqa: E402
from hanji_rem.edits import ALL  # noqa: E402
from hanji_rem.exporter import export_xml  # noqa: E402
from hanji_rem.importer import import_docx  # noqa: E402
from hanji_rem.ooxml import DOC_PART, read_docx, write_docx  # noqa: E402

OUT = os.path.join(HERE, "out", "direct")


def export(doc, parts, text, path):
    blocks = M.parse(text, doc.keeps, doc.styles)
    xml, _ = export_xml(doc, blocks, placed_entries(place(doc, blocks, "C")))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    write_docx(path, parts, {DOC_PART: xml})


def main():
    manifest = []
    for path in sorted(glob.glob(os.path.join(REM, "corpus", "*.docx"))):
        name = os.path.basename(path)
        stem = name[:-5]
        parts = read_docx(path)
        doc = import_docx(parts)
        text = M.serialize(doc.blocks, doc.keeps, doc.styles, name)
        export(doc, parts, text, os.path.join(OUT, stem, "E0.docx"))  # GetPut, no edit
        for fn in ALL:
            ed = fn(doc)
            if ed is None:
                continue
            key = ed.name.split()[0]
            export(doc, parts, M.serialize(ed.blocks, doc.keeps, doc.styles, name),
                   os.path.join(OUT, stem, key + ".docx"))
            manifest.append({"file": stem, "edit": key, "name": ed.name, "what": ed.what})
        fblocks, _, steps = combined(doc)
        export(doc, parts, M.serialize(fblocks, doc.keeps, doc.styles, name),
               os.path.join(OUT, stem, "E10.docx"))
        manifest.append({"file": stem, "edit": "E10", "name": "E10 all edits in one revision",
                         "what": "; ".join(steps)})
    with open(os.path.join(HERE, "out", "direct", "manifest.json"), "w") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=1)
    print(len(manifest), "direct exports")


if __name__ == "__main__":
    main()
