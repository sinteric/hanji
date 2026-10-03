#!/usr/bin/env python3
"""Generate small deterministic MIT test inputs from pinned Hanji blank parts.

No native reference or renderer golden is generated. Inputs are never overwritten.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import zipfile

BASE = 'e2182b04959008d2c2ef9d478dd0e5e68f28ef67'


def fixtures(repo):
    result = {}
    for kind in ('docx','pptx','hwpx','xlsx'):
        prefix = f'crates/hanji-store/blank/{kind}/'
        names = subprocess.check_output(['git','-C',str(repo),'ls-tree','-r','--name-only',BASE,'--',prefix],text=True).splitlines()
        parts = {name.removeprefix(prefix):subprocess.check_output(['git','-C',str(repo),'show',f'{BASE}:{name}']) for name in names}
        if not parts:
            raise ValueError('pinned Hanji blank inputs unavailable')
        if kind == 'docx':
            key = 'word/document.xml'
            parts[key] = parts[key].replace(b'<w:p/>',b'<w:p><w:r><w:t>HANJI DOCX 42</w:t></w:r></w:p>')
        elif kind == 'pptx':
            key = 'ppt/slides/slide1.xml'
            text = b'<a:p><a:r><a:rPr lang="en-US" sz="2400"/><a:t>HANJI PPTX 42</a:t></a:r></a:p>'
            parts[key] = parts[key].replace(b'<a:p><a:endParaRPr lang="ko-KR" altLang="en-US"/></a:p>',text,1)
        elif kind == 'hwpx':
            key = 'Contents/section0.xml'
            parts[key] = parts[key].replace(b'<hp:run charPrIDRef="0"/>',b'<hp:run charPrIDRef="0"><hp:t>HANJI HWPX 42</hp:t></hp:run>')
            # Author the Latin drawing request, so this smoke fixture does not
            # require proprietary Korean fonts or a platform's font discovery.
            parts['Contents/header.xml'] = parts['Contents/header.xml'].replace('함초롬바탕'.encode(),b'Arial')
        else:
            key = 'xl/worksheets/sheet1.xml'
            cells = b'<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>HANJI XLSX</t></is></c><c r="B1"><v>42</v></c></row><row r="2"><c r="A2"><f>B1</f><v>42</v></c></row></sheetData>'
            parts[key] = parts[key].replace(b'<sheetData/>',cells).replace(b'ref="A1"',b'ref="A1:B2"')
        target = io.BytesIO()
        with zipfile.ZipFile(target,'w',compression=zipfile.ZIP_STORED) as archive:
            for name,data in sorted(parts.items(),key=lambda x: (x[0] != 'mimetype',x[0])):
                info = zipfile.ZipInfo(name,(2026,1,1,0,0,0)); info.external_attr = 0o644 << 16
                archive.writestr(info,data)
        result[f'synthetic-{kind}-v1.{kind}'] = target.getvalue()
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--repo',type=Path,default=Path(__file__).resolve().parents[3])
    p.add_argument('--out',type=Path,required=True)
    args = p.parse_args(); generated = fixtures(args.repo); args.out.mkdir(parents=True,exist_ok=True)
    for name,data in generated.items():
        target = args.out/name
        if target.exists():
            raise ValueError('input exists; generator never overwrites '+name)
        target.write_bytes(data)
    print(json.dumps({name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()} for name,data in generated.items()},indent=2))


if __name__ == '__main__':
    main()
