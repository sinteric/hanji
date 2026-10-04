"""Owned deterministic images and minimal image-only Office packages.

Large declarations intentionally retain a tiny pixel payload. They are metadata
fixtures, never inputs to an unbounded raster decode. No third-party assets/tools.
"""
from pathlib import Path
import struct
import zlib
import zipfile

root = Path(__file__).resolve().parent

def chunk(name, data):
    return struct.pack(">I", len(data)) + name + data + struct.pack(">I", zlib.crc32(name + data))

def png(width, height):
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"\0\xff\0\0\xff")) + chunk(b"IEND", b"")

for name, width, height in [("red", 1, 1), ("huge", 16384, 16384), ("zero", 0, 1), ("overflow", 4294967295, 4294967295), ("rounding", 16777217, 1)]:
    (root / f"{name}.png").write_bytes(png(width, height))

def segment(marker, data):
    return bytes([255, marker]) + struct.pack(">H", len(data) + 2) + data

def jpeg(width, height):
    # Three neutral YCbCr blocks: zero DC and EOB codes, yielding gray 128.
    data = b"\xff\xd8" + segment(0xdb, b"\0" + bytes([1]) * 64)
    data += segment(0xc0, struct.pack(">BHHB", 8, height, width, 3) + bytes([1,0x11,0,2,0x11,0,3,0x11,0]))
    data += segment(0xc4, b"\0" + bytes([1] + [0] * 15) + b"\0" + b"\x10" + bytes([1] + [0] * 15) + b"\0")
    return data + segment(0xda, bytes([3,1,0,2,0,3,0,0,63,0])) + b"\x03\xff\xd9"

(root / "gray.jpg").write_bytes(jpeg(1, 1))
(root / "huge.jpg").write_bytes(jpeg(16384, 16384))

rel_ns = "http://schemas.openxmlformats.org/package/2006/relationships"
office_rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"
ct_ns = "http://schemas.openxmlformats.org/package/2006/content-types"
dml = "http://schemas.openxmlformats.org/drawingml/2006/main"
pml = "http://schemas.openxmlformats.org/presentationml/2006/main"
rns = office_rel[:-1]

def rels(entries):
    return f'<Relationships xmlns="{rel_ns}">' + "".join(f'<Relationship Id="{rid}" Type="{office_rel}{kind}" Target="{target}"/>' for rid, kind, target in entries) + '</Relationships>'

def types(overrides):
    return f'<Types xmlns="{ct_ns}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/>' + "".join(f'<Override PartName="/{name}" ContentType="application/vnd.openxmlformats-officedocument.{kind}"/>' for name, kind in overrides) + '</Types>'

group = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
picture = '<p:pic><p:nvPicPr><p:cNvPr id="2" name="Owned red pixel"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>'
namespaces = f'xmlns:p="{pml}" xmlns:a="{dml}" xmlns:r="{rns}"'
clrmap = '<p:clrMap accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" bg1="lt1" bg2="lt2" folHlink="folHlink" hlink="hlink" tx1="dk1" tx2="dk2"/>'
style = '<a:lvl1pPr><a:defRPr sz="1800"/></a:lvl1pPr>'
colors = "".join(f'<a:{name}><a:srgbClr val="{color}"/></a:{name}>' for name, color in [('dk1','000000'),('lt1','FFFFFF'),('dk2','000000'),('lt2','FFFFFF'),('accent1','FF0000'),('accent2','00FF00'),('accent3','0000FF'),('accent4','FFFF00'),('accent5','FF00FF'),('accent6','00FFFF'),('hlink','0000FF'),('folHlink','800080')])
font = '<a:latin typeface="Arial"/><a:ea typeface=""/><a:cs typeface=""/>'
solid = '<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>'
lines = ('<a:ln w="9525">' + solid + '<a:prstDash val="solid"/></a:ln>') * 3
effects = '<a:effectStyle><a:effectLst/></a:effectStyle>' * 3
theme = f'<a:theme xmlns:a="{dml}" name="Owned"><a:themeElements><a:clrScheme name="Owned">{colors}</a:clrScheme><a:fontScheme name="Owned"><a:majorFont>{font}</a:majorFont><a:minorFont>{font}</a:minorFont></a:fontScheme><a:fmtScheme name="Owned"><a:fillStyleLst>{solid*3}</a:fillStyleLst><a:lnStyleLst>{lines}</a:lnStyleLst><a:effectStyleLst>{effects}</a:effectStyleLst><a:bgFillStyleLst>{solid*3}</a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>'
parts = {
    '[Content_Types].xml': types([('ppt/presentation.xml','presentationml.presentation.main+xml'),('ppt/slides/slide1.xml','presentationml.slide+xml'),('ppt/slideLayouts/slideLayout1.xml','presentationml.slideLayout+xml'),('ppt/slideMasters/slideMaster1.xml','presentationml.slideMaster+xml'),('ppt/theme/theme1.xml','theme+xml')]),
    '_rels/.rels': rels([('rId1','officeDocument','ppt/presentation.xml')]),
    'ppt/presentation.xml': f'<p:presentation {namespaces}><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="914400" cy="914400"/><p:notesSz cx="914400" cy="914400"/></p:presentation>',
    'ppt/_rels/presentation.xml.rels': rels([('rId1','slideMaster','slideMasters/slideMaster1.xml'),('rId2','slide','slides/slide1.xml')]),
    'ppt/slides/slide1.xml': f'<p:sld {namespaces}><p:cSld><p:spTree>{group}{picture}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>',
    'ppt/slides/_rels/slide1.xml.rels': rels([('rId1','slideLayout','../slideLayouts/slideLayout1.xml'),('rId2','image','../media/image.png')]),
    'ppt/slideLayouts/slideLayout1.xml': f'<p:sldLayout {namespaces} type="blank"><p:cSld><p:spTree>{group}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>',
    'ppt/slideLayouts/_rels/slideLayout1.xml.rels': rels([('rId1','slideMaster','../slideMasters/slideMaster1.xml')]),
    'ppt/slideMasters/slideMaster1.xml': f'<p:sldMaster {namespaces}><p:cSld><p:spTree>{group}</p:spTree></p:cSld>{clrmap}<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle>{style}</p:titleStyle><p:bodyStyle>{style}</p:bodyStyle><p:otherStyle>{style}</p:otherStyle></p:txStyles></p:sldMaster>',
    'ppt/slideMasters/_rels/slideMaster1.xml.rels': rels([('rId1','slideLayout','../slideLayouts/slideLayout1.xml'),('rId2','theme','../theme/theme1.xml')]),
    'ppt/theme/theme1.xml': theme,
    'ppt/media/image.png': (root/'red.png').read_bytes(),
}

def write_package(name, parts):
    with zipfile.ZipFile(root/name, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for path, content in sorted(parts.items()):
            info = zipfile.ZipInfo(path, (2020,1,1,0,0,0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, content.encode() if isinstance(content,str) else content)

write_package('owned.pptx', parts)
wml = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
wp = 'http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing'
pic = 'http://schemas.openxmlformats.org/drawingml/2006/picture'
drawing = f'<wp:inline><wp:extent cx="914400" cy="914400"/><wp:docPr id="1" name="Owned red pixel"/><a:graphic><a:graphicData uri="{pic}"><pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="Owned"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId1"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline>'
write_package('owned.docx', {
    '[Content_Types].xml': types([('word/document.xml','wordprocessingml.document.main+xml')]),
    '_rels/.rels': rels([('rId1','officeDocument','word/document.xml')]),
    'word/document.xml': f'<w:document xmlns:w="{wml}" xmlns:r="{rns}" xmlns:a="{dml}" xmlns:wp="{wp}" xmlns:pic="{pic}"><w:body><w:p><w:r><w:drawing>{drawing}</w:drawing></w:r></w:p><w:sectPr><w:pgSz w:w="2880" w:h="2880"/><w:pgMar w:top="0" w:right="0" w:bottom="0" w:left="0" w:header="0" w:footer="0" w:gutter="0"/></w:sectPr></w:body></w:document>',
    'word/_rels/document.xml.rels': rels([('rId1','image','media/image.png')]),
    'word/media/image.png': (root/'red.png').read_bytes(),
})
