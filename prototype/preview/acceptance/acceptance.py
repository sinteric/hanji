#!/usr/bin/env python3
"""Bounded, local acceptance checks. No renderer, app or baseline mutation."""
import argparse
import base64
from collections import Counter
import hashlib
import html
import json
import math
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import unicodedata
import xml.etree.ElementTree as ET

VERSION = 1
MAX_FILE = 64 * 1024 * 1024
MAX_SVG = 16 * 1024 * 1024
MAX_PAGES = 512  # harness budget, not a Hanji product limit
HERE = Path(__file__).resolve().parent


class NativeBlocked(ValueError):
    """Unavailable or unsuitable native evidence, independent of regression."""


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def bounded(path, limit=MAX_FILE):
    path = Path(path)
    if path.stat().st_size > limit:
        raise ValueError(f'harness byte budget exceeded: {path.name}')
    return path.read_bytes()


def png_size(data):
    if len(data) < 24 or data[:8] != b'\x89PNG\r\n\x1a\n' or data[12:16] != b'IHDR':
        raise ValueError('PNG IHDR dimensions required')
    size = struct.unpack_from('>II',data,16)
    if not all(size):
        raise ValueError('positive PNG dimensions required')
    return size


def under(root, name):
    root = Path(root).resolve()
    path = (root / name).resolve()
    if not path.is_relative_to(root):
        raise ValueError('artifact path escapes its root')
    return path


def load(path):
    return json.loads(bounded(path))


def norm(text):
    return ''.join(unicodedata.normalize('NFC', text).split())


def local(tag):
    return tag.rsplit('}', 1)[-1]


def drawn(element):
    return (element.text or '') + ''.join(
        drawn(c) + (c.tail or '') for c in element if local(c.tag) not in ('title', 'desc'))


def length_pt(value):
    match = re.fullmatch(r'\s*([-+\d.eE]+)\s*(pt|px|in|mm|cm)?\s*', value)
    if not match:
        raise ValueError('unsupported SVG physical length')
    return float(match[1]) * {'pt': 1, 'px': .75, 'in': 72, 'mm': 72/25.4, 'cm': 72/2.54, None: .75}[match[2]]


IDENTITY = (1, 0, 0, 1, 0, 0)


def multiply(a, b):
    return (a[0]*b[0]+a[2]*b[1], a[1]*b[0]+a[3]*b[1],
            a[0]*b[2]+a[2]*b[3], a[1]*b[2]+a[3]*b[3],
            a[0]*b[4]+a[2]*b[5]+a[4], a[1]*b[4]+a[3]*b[5]+a[5])


def transform(value):
    result = IDENTITY
    end = 0
    for match in re.finditer(r'(matrix|translate|scale|rotate)\s*\(([^)]*)\)', value):
        if value[end:match.start()].strip(' ,\t\r\n'):
            raise ValueError('unsupported SVG transform')
        name, args = match[1], [float(x) for x in re.split(r'[\s,]+', match[2].strip())]
        if name == 'matrix' and len(args) == 6:
            next_matrix = tuple(args)
        elif name == 'translate' and len(args) in (1, 2):
            next_matrix = (1, 0, 0, 1, args[0], args[1] if len(args) == 2 else 0)
        elif name == 'scale' and len(args) in (1, 2):
            next_matrix = (args[0], 0, 0, args[-1], 0, 0)
        elif name == 'rotate' and len(args) in (1, 3):
            angle = math.radians(args[0]); c, s = math.cos(angle), math.sin(angle)
            next_matrix = (c, s, -s, c, 0, 0)
            if len(args) == 3:
                x, y = args[1:]
                next_matrix = multiply(multiply((1, 0, 0, 1, x, y), next_matrix), (1, 0, 0, 1, -x, -y))
        else:
            raise ValueError('unsupported SVG transform arguments')
        result = multiply(result, next_matrix); end = match.end()
    if value[end:].strip():
        raise ValueError('unsupported SVG transform')
    return result


def point(matrix, x, y):
    a, b, c, d, e, f = matrix
    return (a*x+c*y+e, b*x+d*y+f)


def rect(matrix, box):
    x, y, w, h = box
    corners = [point(matrix, a, b) for a, b in [(x,y), (x+w,y), (x,y+h), (x+w,y+h)]]
    xs, ys = zip(*corners)
    return [min(xs), min(ys), max(xs), max(ys)]


def contained(inner, outer, tolerance=.5):
    return inner[0] >= outer[0]-tolerance and inner[1] >= outer[1]-tolerance and inner[2] <= outer[2]+tolerance and inner[3] <= outer[3]+tolerance


def intersection(a, b):
    return max(0, min(a[2],b[2])-max(a[0],b[0])) * max(0, min(a[3],b[3])-max(a[1],b[1]))


class FontMetrics:
    """Read cmap/advances plus bounded glyph evidence, not raster/native fidelity."""
    def __init__(self, data):
        if len(data) < 12 or data[:4] not in (b'OTTO', b'\0\1\0\0'):
            raise ValueError('unsupported embedded font container')
        self.tables = {}
        for i in range(struct.unpack_from('>H', data, 4)[0]):
            tag, _, offset, size = struct.unpack_from('>4sIII', data, 12+i*16)
            if offset+size > len(data):
                raise ValueError('font table exceeds bytes')
            self.tables[tag.decode()] = data[offset:offset+size]
        self.units = struct.unpack_from('>H', self.tables['head'], 18)[0]
        self.metrics = struct.unpack_from('>H', self.tables['hhea'], 34)[0]
        # PR70 uses this same OpenType flag. SVG aliases and CLI coverage
        # reports cannot erase the generic-symbol semantics of embedded bytes.
        self.generic_reasons = []
        if struct.unpack_from('>H',self.tables['head'],16)[0] & (1 << 14):
            self.generic_reasons.append('head.flags bit 14: generic LastResort symbols')
        names = self.tables.get('name',b'')
        if names:
            _,count,start = struct.unpack_from('>3H',names)
            for i in range(count):
                platform,_,_,name_id,size,offset = struct.unpack_from('>6H',names,6+12*i)
                if name_id not in (1,4,6,16):
                    continue
                raw = names[start+offset:start+offset+size]
                name = raw.decode('utf-16-be' if platform in (0,3) else 'mac_roman',errors='replace')
                if re.sub('[^a-z]','',name.lower()).startswith('lastresort'):
                    self.generic_reasons.append('embedded LastResort name: '+name)
        self.glyph_cache = {}
        cmap = self.tables['cmap']; candidates = []
        for i in range(struct.unpack_from('>H', cmap, 2)[0]):
            platform, encoding, offset = struct.unpack_from('>HHI', cmap, 4+8*i)
            form = struct.unpack_from('>H', cmap, offset)[0]
            if platform in (0, 3) and form in (4, 12):
                candidates.append((form, cmap[offset:]))
        if not candidates or not self.units or not self.metrics:
            raise ValueError('font lacks supported metrics/cmap')
        self.form, self.cmap = max(candidates, key=lambda x: x[0])

    def glyph(self, char):
        cp = ord(char); c = self.cmap; glyph = 0
        if self.form == 12:
            for i in range(struct.unpack_from('>I', c, 12)[0]):
                start, end, first = struct.unpack_from('>III', c, 16+12*i)
                if start <= cp <= end:
                    glyph = first+cp-start; break
        else:
            n = struct.unpack_from('>H', c, 6)[0]//2
            for i in range(n):
                end = struct.unpack_from('>H', c, 14+2*i)[0]
                start = struct.unpack_from('>H', c, 16+2*n+2*i)[0]
                if start <= cp <= end:
                    delta = struct.unpack_from('>h', c, 16+4*n+2*i)[0]
                    offset = struct.unpack_from('>H', c, 16+6*n+2*i)[0]
                    glyph = (cp+delta)&65535 if offset == 0 else struct.unpack_from('>H', c, 16+6*n+2*i+offset+2*(cp-start))[0]
                    if offset and glyph:
                        glyph = (glyph+delta)&65535
                    break
        return glyph

    def advance(self, char, size):
        glyph = self.glyph(char)
        if not glyph:
            return None
        advance = struct.unpack_from('>H', self.tables['hmtx'], min(glyph,self.metrics-1)*4)[0]
        return advance*size/self.units

    def outline(self, glyph):
        """Inspect TrueType records only; CFF/bitmap/SVG outlines stay unverified."""
        if not all(t in self.tables for t in ('glyf','loca','maxp')):
            return None
        count = struct.unpack_from('>H',self.tables['maxp'],4)[0]
        if not 0 <= glyph < count:
            raise ValueError('cmap glyph exceeds maxp glyph count')
        fmt = struct.unpack_from('>h',self.tables['head'],50)[0]
        if fmt not in (0,1):
            raise ValueError('unsupported loca offset format')
        code,width,mult = ('H',2,2) if fmt == 0 else ('I',4,1)
        first,last = struct.unpack_from('>2'+code,self.tables['loca'],glyph*width)
        first *= mult; last *= mult
        if not 0 <= first <= last <= len(self.tables['glyf']):
            raise ValueError('glyph outline exceeds glyf bytes')
        return self.tables['glyf'][first:last]

    def glyph_evidence(self, char):
        if self.generic_reasons:
            return 'failed','; '.join(self.generic_reasons)
        glyph = self.glyph(char)
        key = (glyph,char.isalnum())
        if key in self.glyph_cache:
            return self.glyph_cache[key]
        result = self.inspect_glyph(glyph,char.isalnum(),())
        self.glyph_cache[key] = result
        return result

    def inspect_glyph(self, glyph, letter_digit, visiting):
        if glyph in visiting:
            return 'failed','cyclic composite outline'
        if len(visiting) >= 16:
            return 'blocked','composite depth exceeds inspection budget'
        if not glyph:
            result = ('failed','missing cmap glyph')
        else:
            try:
                outline = self.outline(glyph)
                if outline is None:
                    result = ('blocked','outline container is outside TrueType inspection coverage')
                elif len(outline) < 10:
                    result = ('empty','nonprinting component') if visiting else ('failed','mapped glyph has no outline record')
                else:
                    contours,x1,y1,x2,y2 = struct.unpack_from('>5h',outline)
                    if contours == 0 or x1 >= x2 or y1 >= y2:
                        result = ('empty','nonprinting component') if visiting else ('failed','mapped glyph has no nonempty outline bounds')
                    elif contours < 0:
                        result = self.inspect_components(outline,letter_digit,(*visiting,glyph))
                    elif contours > 4096:
                        result = ('blocked','simple contour count exceeds inspection budget')
                    else:
                        shape = simple_shape(outline)
                        xs,ys = zip(*shape[2])
                        notdef = self.outline(0)
                        if min(xs) == max(xs) or min(ys) == max(ys):
                            result = ('empty','nonprinting component') if visiting else ('failed','mapped simple outline has no two-dimensional points')
                        elif letter_digit and notdef and struct.unpack_from('>h',notdef)[0] > 0 and shape == simple_shape(notdef):
                            # Compare contours, not padding or hint instructions.
                            # Authored square/replacement symbols are not failed
                            # solely because they resemble .notdef.
                            result = ('failed','mapped letter/digit duplicates the .notdef tofu outline')
                        else:
                            result = ('passed','nonempty simple TrueType outline; semantic/raster review unverified')
            except (ValueError,struct.error,KeyError,IndexError) as error:
                result = ('failed','invalid outline data: '+str(error))
        return result

    def inspect_components(self, data, letter_digit, visiting):
        cursor = 10; children = []
        for _ in range(128):
            flags,glyph = struct.unpack_from('>2H',data,cursor); cursor += 4
            cursor += 4 if flags & 1 else 2
            size = 4 if flags & 128 else 2 if flags & 64 else 1 if flags & 8 else 0
            values = struct.unpack_from('>'+str(size)+'h',data,cursor) if size else ()
            cursor += 2*size
            if cursor > len(data):
                raise ValueError('truncated composite arguments')
            determinant = values[0]*values[3]-values[1]*values[2] if size == 4 else values[0]*values[-1] if size else 1
            if determinant == 0:
                return 'failed','composite transform collapses the outline'
            if not glyph and letter_digit:
                return 'failed','mapped letter/digit composite uses .notdef tofu'
            children.append(self.inspect_glyph(glyph,letter_digit,visiting))
            if not flags & 32:
                break
        else:
            return 'blocked','composite component count exceeds inspection budget'
        if flags & 256:
            length = struct.unpack_from('>H',data,cursor)[0]
            if cursor+2+length > len(data):
                raise ValueError('truncated composite instructions')
        bad = next((r for r in children if r[0] == 'failed'),None)
        unknown = next((r for r in children if r[0] == 'blocked'),None)
        return bad or unknown or (('passed','nonempty TrueType component outlines; semantic/raster review unverified')
                                  if any(r[0] == 'passed' for r in children) else ('failed','composite has no drawing components'))


def simple_shape(data):
    """Bounded TrueType simple contours, omitting hint instructions and padding."""
    contours = struct.unpack_from('>h',data)[0]
    if not 0 < contours <= 4096:
        raise ValueError('simple contour count outside inspection budget')
    ends = struct.unpack_from('>'+str(contours)+'H',data,10)
    if any(a >= b for a,b in zip(ends,ends[1:])):
        raise ValueError('invalid simple contour endpoints')
    count = ends[-1]+1; cursor = 10+2*contours
    instructions = struct.unpack_from('>H',data,cursor)[0]
    cursor += 2+instructions
    flags = []
    while len(flags) < count:
        flag = data[cursor]; cursor += 1
        repeat = 1
        if flag & 8:
            repeat += data[cursor]; cursor += 1
        if len(flags)+repeat > count:
            raise ValueError('simple point flags exceed endpoint count')
        flags.extend([flag]*repeat)
    coordinates = []
    for short,same in ((2,16),(4,32)):
        values = []; value = 0
        for flag in flags:
            if flag & short:
                delta = data[cursor] * (1 if flag & same else -1); cursor += 1
            elif flag & same:
                delta = 0
            else:
                delta = struct.unpack_from('>h',data,cursor)[0]; cursor += 2
            value += delta; values.append(value)
        coordinates.append(values)
    return ends,tuple(f & 1 for f in flags),tuple(zip(*coordinates))


def svg_page(path):
    data = bounded(path, MAX_SVG)
    if b'<!DOCTYPE' in data.upper() or b'<!ENTITY' in data.upper():
        raise ValueError('SVG DTD/entities are outside the harness contract')
    root = ET.fromstring(data)
    if local(root.tag) != 'svg':
        raise ValueError('SVG root required')
    vb = [float(x) for x in re.split(r'[ ,]+', root.attrib['viewBox'].strip())]
    if len(vb) != 4 or not all(math.isfinite(x) for x in vb) or min(vb[2:]) <= 0:
        raise ValueError('invalid SVG viewBox')
    width = length_pt(root.attrib.get('width', str(vb[2])))
    height = length_pt(root.attrib.get('height', str(vb[3])))
    if not all(math.isfinite(x) and x > 0 for x in (width,height)):
        raise ValueError('invalid SVG dimensions')
    scale = (width/vb[2], 0, 0, height/vb[3], -vb[0]*width/vb[2], -vb[1]*height/vb[3])
    ids = {}; refs = []; fonts = {}; font_hashes = {}; issues = []
    for e in root.iter():
        if 'id' in e.attrib:
            if e.attrib['id'] in ids:
                issues.append('duplicate SVG ID '+e.attrib['id'])
            ids[e.attrib['id']] = e
        for key, value in e.attrib.items():
            refs.extend(re.findall(r'url\(\s*[\"\']?#([^\s\"\')]+)', value, re.I))
            if local(key) == 'href' and value.startswith('#'):
                refs.append(value[1:])
        if local(e.tag) == 'style':
            for block in re.findall(r'@font-face\s*\{([^}]*)\}', e.text or ''):
                family = re.search(r'font-family\s*:\s*[\"\']?([^;\"\']+)', block)
                encoded = re.search(r'data:font/[^;]+;base64,([^\"\')\s]+)', block)
                if family and encoded:
                    raw = base64.b64decode(encoded[1], validate=True)
                    font_hashes[family[1]] = hashlib.sha256(raw).hexdigest()
                    try:
                        fonts[family[1]] = FontMetrics(raw)
                    except (ValueError, KeyError, struct.error):
                        issues.append('unsupported embedded font metrics '+family[1])
    issues.extend('unresolved SVG fragment '+r for r in refs if r not in ids)
    chars = []; clip_issues = []; embedded_missing = set(); unverified_fonts = set(); glyph_evidence = {}

    def walk(e, matrix, clips, inherited):
        tag = local(e.tag)
        if tag in ('defs', 'style', 'title', 'desc'):
            return
        matrix = multiply(matrix, transform(e.attrib.get('transform','')))
        attrs = {**inherited, **e.attrib}
        clip = re.fullmatch(r'url\(#([^)]*)\)', e.attrib.get('clip-path',''))
        if clip and clip[1] in ids:
            definition = ids[clip[1]]
            children = list(definition)
            if definition.attrib.get('clipPathUnits','userSpaceOnUse') == 'userSpaceOnUse' and len(children) == 1 and local(children[0].tag) == 'rect':
                c = children[0]; cm = multiply(matrix, transform(c.attrib.get('transform','')))
                clips = clips + [rect(cm,[float(c.attrib.get(k,0)) for k in ('x','y','width','height')])]
            else:
                clip_issues.append('unsupported clip geometry '+clip[1])
        if tag == 'text':
            text = drawn(e); size = float(attrs.get('font-size',12))
            x = [float(v) for v in re.split(r'[ ,]+', attrs.get('x','0').strip())]
            y = float(attrs.get('y','0').split()[0]); face = fonts.get(attrs.get('font-family',''))
            positioned_children = any(local(c.tag) == 'tspan' for c in e.iter() if c is not e)
            if text.strip() and face is None:
                unverified_fonts.add(attrs.get('font-family','(unspecified)'))
            for i, char in enumerate(text):
                if not char.isspace():
                    advance = face.advance(char,size) if face else None
                    evidence = face.glyph_evidence(char) if face else ('blocked','drawing face is unavailable')
                    glyph_evidence[(attrs.get('font-family',''),char)] = evidence
                    if face and advance is None:
                        embedded_missing.add(f'U+{ord(char):04X}')
                    box = rect(matrix,[x[min(i,len(x)-1)],y-size,advance or 0,size*1.25])
                    chars.append({'char': unicodedata.normalize('NFC',char), 'box':box, 'origin':point(matrix,x[min(i,len(x)-1)],y), 'clips':clips,
                                  'metrics_verified': advance is not None and len(x) == len(text) and not positioned_children})
            return
        for child in e:
            walk(child,matrix,clips,attrs)
    walk(root,scale,[],{})
    return {'width_pt':width,'height_pt':height,'text':''.join(c['char'] for c in chars),
            'chars':chars,'font_subset_sha256':font_hashes,'embedded_missing_glyphs':sorted(embedded_missing),'unverified_fonts':sorted(unverified_fonts),'issues':issues,'clip_issues':clip_issues,
            'glyph_evidence':[{'face':family,'char':f'U+{ord(char):04X}','status':result[0],'reason':result[1]} for (family,char),result in sorted(glyph_evidence.items())],
            'sha256':hashlib.sha256(data).hexdigest(),'path':str(path)}


def critical_box(page, value):
    text = norm(value); start = page['text'].find(text)
    if start < 0:
        return None
    chars = page['chars'][start:start+len(text)]
    if not chars or not all(c['metrics_verified'] for c in chars):
        return None
    boxes = [c['box'] for c in chars]
    return [min(b[0] for b in boxes),min(b[1] for b in boxes),max(b[2] for b in boxes),max(b[3] for b in boxes)]


def layer(name, status, details):
    return {'layer':name,'status':status,'details':details}


def verify_reference(metadata, root, fixture):
    """Reject wrong sources and unknown native provenance before extraction."""
    if metadata.get('schema_version') != VERSION or metadata.get('kind') != 'native-application':
        raise ValueError('native-application schema_version 1 required; regression goldens are not native references')
    if metadata.get('fixture_id') != fixture['id'] or metadata.get('source_sha256') != fixture['input']['sha256']:
        raise ValueError('native reference belongs to different source bytes')
    app = metadata.get('app',{})
    if app.get('name') != fixture['native_reference']['app'] or not app.get('version') or app.get('version_confirmed') is not True or not app.get('os'):
        raise ValueError('confirmed native app/version and OS required')
    if metadata.get('rights',{}).get('local_validation') is not True:
        raise ValueError('owner permission for local reference validation required')
    mode = metadata.get('export',{}).get('mode')
    expected = {'docx':'no-markup','pptx':'full-slides','hwpx':'document-pages','xlsx':'worksheet-window'}[fixture['format']]
    if mode != expected:
        raise ValueError('native export mode must be '+expected)
    if any(metadata['export'].get(k) is not False for k in ('repair_prompt','fields_updated')):
        raise ValueError('native export must confirm no repair prompt and no updated fields')
    if fixture['format'] == 'xlsx':
        for key in ('sheet','range'):
            if metadata['export'].get(key) != fixture['window'][key]:
                raise ValueError('native worksheet window does not match '+key)
    for record in metadata.get('files',[]) + metadata.get('fonts',[]):
        if not re.fullmatch('[a-f0-9]{64}',record.get('sha256','')):
            raise ValueError('native file/font SHA-256 required')
        path = under(root,record['path']); bounded(path)
        if sha(path) != record['sha256']:
            raise ValueError('native file/font hash mismatch: '+record['path'])
    if not metadata.get('files') or not metadata.get('fonts'):
        raise ValueError('native output files and original font identities/hashes required')
    for font in metadata['fonts']:
        if not font.get('family') or (Path(font['path']).suffix.lower() == '.ttc' and (type(font.get('face_index')) is not int or font['face_index'] < 0)):
            raise ValueError('original font family and collection face index required')
    pdfs = [r for r in metadata['files'] if r.get('role') == 'native-pdf']
    if fixture['format'] != 'xlsx' and len(pdfs) != 1:
        raise ValueError('exactly one native PDF required')
    images = [r for r in metadata['files'] if r.get('role') == 'native-page-png']
    if not images or any(type(r.get('page')) is not int or r['page'] < 1 for r in images) or len({r['page'] for r in images}) != len(images):
        raise ValueError('unique indexed native page/window PNGs required')
    for image in images:
        png_size(bounded(under(root,image['path'])))
    return metadata


def verify_native_geometry(native, pages, root):
    if not 0 < len(pages) <= MAX_PAGES:
        raise NativeBlocked('native page enumeration exceeds harness budget or is absent')
    images = {r['page']:r for r in native['files'] if r.get('role') == 'native-page-png'}
    if set(images) != set(range(1,len(pages)+1)):
        raise NativeBlocked('native PNGs must enumerate every page/window')
    for i,page in enumerate(pages,1):
        if page.get('index') != i or not isinstance(page.get('text'),str):
            raise NativeBlocked('native page index and owner/extracted text required')
        dims = [page[k] for k in ('width_pt','height_pt')]
        if not all(isinstance(x,(int,float)) and math.isfinite(x) and x > 0 for x in dims):
            raise NativeBlocked('positive finite native dimensions required')
        size = png_size(bounded(under(root,images[i]['path'])))
        if any(abs(a-round(b*4/3)) > 1 for a,b in zip(size,dims)):
            raise NativeBlocked('native PNG must preserve calibrated 96-DPI page/window geometry')


def ingest(metadata_path, corpus, reference_root, output):
    metadata = load(metadata_path)
    fixture = next(f for f in corpus['fixtures'] if f['id'] == metadata['fixture_id'])
    verify_reference(metadata,reference_root,fixture)
    pdfs = [r for r in metadata['files'] if r.get('role') == 'native-pdf']
    if fixture['format'] != 'xlsx':
        pdf = under(reference_root,pdfs[0]['path'])
        metadata['pages'] = extract_pdf_pages(pdf)
        metadata['extraction'] = {'tool':'pdftotext -bbox-layout','pdf_sha256':sha(pdf)}
    elif not metadata.get('pages') or not metadata.get('cells'):
        raise ValueError('worksheet screenshot needs owner-verified page dimensions/text and cached-value cell sidecar')
    verify_native_geometry(metadata,metadata['pages'],reference_root)
    write_json(output,metadata)


def extract_pdf_pages(pdf):
    if not shutil.which('pdftotext'):
        raise NativeBlocked('native PDF text extraction blocked: pdftotext unavailable; no installation attempted')
    try:
        run = subprocess.run(['pdftotext','-bbox-layout',str(pdf),'-'],capture_output=True,timeout=60,check=True)
    except subprocess.SubprocessError as error:
        raise NativeBlocked('native PDF text extraction unavailable: '+str(error)) from error
    if len(run.stdout) > MAX_FILE:
        raise ValueError('PDF text exceeds harness budget')
    tree = ET.fromstring(run.stdout)
    return [{'index':i,'width_pt':float(p.attrib['width']),'height_pt':float(p.attrib['height']),
             'text':' '.join(drawn(w) for w in p.iter() if local(w.tag) == 'word')}
            for i,p in enumerate((e for e in tree.iter() if local(e.tag) == 'page'),1)]


def capture(fixtures, repo, binary, out, font_dir=None):
    """Run an existing CLI against unchanged inputs into a fresh local directory."""
    if out.exists():
        raise ValueError('capture directory exists; evidence is never overwritten')
    ancestor = out.parent
    while not ancestor.exists():
        ancestor = ancestor.parent
    if shutil.disk_usage(ancestor).free < 256 * 1024 * 1024:
        raise ValueError('capture blocked: less than 256 MiB free; no cache deletion attempted')
    binary = binary.resolve(); bounded(binary)
    version = subprocess.run([str(binary),'--version'],capture_output=True,text=True,timeout=10,check=True).stdout.strip()
    out.mkdir(parents=True); runs = []
    for fixture in fixtures:
        source = under(repo,fixture['input']['path']); bounded(source)
        before = sha(source)
        if before != fixture['input']['sha256']:
            raise ValueError('capture refuses changed input '+fixture['id'])
        for kind in ('svg','png','html'):
            folder = out/fixture.get('artifact_stem',source.stem)/kind
            command = [str(binary),'--store',str(out/'store'),'--json','preview',str(source),'--format',kind,'--out',str(folder)]
            if fixture['format'] == 'xlsx':
                command += ['--sheet',fixture['window']['sheet'],'--range',fixture['window']['range']]
            if font_dir:
                command += ['--font-dir',str(font_dir)]
            run = subprocess.run(command,capture_output=True,text=True,timeout=60)
            if len(run.stdout.encode()) > MAX_FILE:
                raise ValueError('CLI JSON exceeds harness byte budget')
            report = json.loads(run.stdout) if run.stdout else {'error':run.stderr[:4096]}
            write_json(folder/'result.json',report)
            after = sha(source)
            runs.append({'fixture_id':fixture['id'],'output':kind,'exit_code':run.returncode,
                         'input_before_sha256':before,'input_after_sha256':after,'source_unchanged':before == after})
            if before != after:
                raise ValueError('CLI mutated original input '+fixture['id'])
    record = {'schema_version':VERSION,'kind':'self-rendered-regression-capture','binary_sha256':sha(binary),'version':version,'runs':runs}
    if font_dir:
        record['supplied_font_files'] = [{'name':p.name,'sha256':sha(p)} for p in sorted(font_dir.rglob('*')) if p.suffix.lower() in ('.ttf','.otf','.ttc')]
    write_json(out/'capture.json',record)
    return record


def audit(fixture, repo, artifacts, reference_root=None, hashes=False, overlay=None):
    checks = []; input_path = under(repo,fixture['input']['path']); before = sha(input_path)
    valid = before == fixture['input']['sha256']
    checks.append(layer('source_hash','passed' if valid else 'failed',{'sha256':before}))
    folder = under(artifacts,fixture.get('artifact_stem',input_path.stem))
    if not valid or not (folder/'svg/result.json').exists():
        checks.append(layer('artifacts','blocked','matching input and saved SVG/PNG/HTML result.json are required'))
        return {'fixture_id':fixture['id'],'regression_status':'failed' if not valid else 'blocked','native_status':'blocked','fidelity_promoted':False,'checks':checks}
    reports = {kind:load(folder/kind/'result.json') for kind in ('svg','png','html')}
    files = {kind:[under(folder/kind,Path(name).name) for name in r['files']] for kind,r in reports.items()}
    count = reports['svg'].get('pages',reports['svg'].get('slides',1))
    if not isinstance(count,int) or not 0 < count <= MAX_PAGES:
        raise ValueError('candidate page enumeration exceeds harness budget')
    expected = fixture.get('regression',{})
    enum_ok = len(files['svg']) == len(files['png']) == count and len(files['html']) == 1
    enum_ok &= all(r.get('pages',r.get('slides',1)) == count for r in reports.values())
    enum_ok &= len(set(files['svg'])) == count and len(set(files['png'])) == count
    for kind in ('svg','png'):
        if fixture['format'] != 'xlsx':
            nums = [re.search(r'-(?:page|slide)-(\d+)\.'+kind+r'$',p.name) for p in files[kind]]
            enum_ok &= all(nums) and [int(m[1]) for m in nums if m] == list(range(1,count+1))
    if fixture['format'] == 'xlsx':
        enum_ok &= count == 1
        for report in reports.values():
            enum_ok &= report.get('range') == fixture['window']['range'] and report.get('sheet',{}).get('name') == fixture['window']['sheet']
    pages = [svg_page(p) for p in files['svg']]
    dims = [[p['width_pt'],p['height_pt']] for p in pages]
    enum_ok &= expected.get('pages') == count
    expected_dims = expected.get('dimensions_pt',[])
    dim_ok = len(dims) == len(expected_dims) and all(abs(a-b) < .02 for x,y in zip(dims,expected_dims) for a,b in zip(x,y))
    for index,path in enumerate(files['png']):
        size = png_size(bounded(path))
        if index >= len(dims):
            dim_ok = False; continue
        dim_ok &= all(abs(a-round(b*4/3)) <= 1 for a,b in zip(size,dims[index]))
    checks.append(layer('enumeration_dimensions','passed' if enum_ok and dim_ok else 'failed',{'pages':count,'dimensions_pt':dims,'expected_pages':expected.get('pages')}))
    html_bytes = bounded(files['html'][0]); html_text = html_bytes.decode()
    inline_ids = re.findall(r'\bid="([^"]+)"',html_text)
    structure = [issue for p in pages for issue in p['issues']]
    if len(inline_ids) != len(set(inline_ids)):
        structure.append('duplicate document-wide HTML IDs')
    if len(re.findall(r'<svg\b',html_text)) != count:
        structure.append('HTML does not enumerate all pages/windows')
    checks.append(layer('svg_html_structure','failed' if structure else 'passed',structure))
    critical = fixture.get('critical_text',[]); text_issues = []; boxes = {}; geometry = []
    for item in critical:
        i = item['page']-1; key = item['id']; value = norm(item['value'])
        hits = pages[i]['text'].count(value) if 0 <= i < len(pages) else 0
        if hits < item.get('minimum_count',1):
            text_issues.append(key+' missing from its required page/window')
        if 0 <= i < len(pages):
            box = critical_box(pages[i],value); boxes[key] = box
            if box is None:
                geometry.append(key+': advance geometry is unavailable')
            else:
                outer = item.get('bounds_pt',[0,0,*dims[i]])
                if not contained(box,outer):
                    geometry.append(key+': critical advance box exceeds required bounds')
                start = pages[i]['text'].find(value)
                for char in pages[i]['chars'][start:start+len(value)]:
                    ox,oy = char['origin']
                    if any(not contained([ox,oy,ox,oy],clip) for clip in char['clips']):
                        geometry.append(key+': critical text crosses a rectangular clip')
                    elif any(not contained(char['box'],clip) for clip in char['clips']):
                        geometry.append(key+': conservative font envelope crosses clip; actual ink verification required')
    cell_issues = []
    for address,value in fixture.get('critical_cells',{}).items():
        cell = next((c for c in reports['svg'].get('cells',[]) if c['address'] == address),None)
        if cell is None or cell.get('display') != value['display'] or cell.get('formula_result') != value['formula_result']:
            cell_issues.append(address+': cached/display value contract changed')
    checks.append(layer('critical_text_values','failed' if text_issues or cell_issues else 'passed' if critical else 'blocked',text_issues+cell_issues or ('critical assertions verified' if critical else 'owner-reviewed critical text assertions missing')))
    for pair in fixture.get('non_overlap',[]):
        if all(boxes.get(key) for key in pair) and intersection(boxes[pair[0]],boxes[pair[1]]) > .5:
            geometry.append('critical regions overlap: '+' / '.join(pair))
    geometry.extend(issue for p in pages for issue in p['clip_issues'])
    checks.append(layer('critical_geometry','failed' if any('exceeds' in s or 'critical text crosses' in s or 'overlap:' in s for s in geometry) else 'blocked' if geometry or not critical else 'passed',{'issues':geometry,'coverage':'critical advance boxes and rectangular clips; not general ink/path/shape collision detection'}))
    messages = {s for report in reports.values() for s in report.get('warnings',[])} | {d['message'] for report in reports.values() for d in report.get('diagnostics',[])}
    contract = expected.get('diagnostics',{})
    unknown = [s for s in messages if not any(re.search(p,s) for p in contract.get('allow',[]))]
    absent = [p for p in contract.get('required',[]) if not any(re.search(p,s) for s in messages)]
    checks.append(layer('diagnostics','failed' if unknown or absent else 'passed',{'unexpected':unknown,'missing_required':absent}))
    missing = sorted(f['char'] for f in reports['svg'].get('fonts',{}).get('missing_glyphs',[]))
    embedded_missing = sorted({c for p in pages for c in p['embedded_missing_glyphs']})
    unverified_fonts = sorted({f for p in pages for f in p['unverified_fonts']})
    checks.append(layer('font_coverage','failed' if embedded_missing or missing != sorted(expected.get('missing_glyphs',[])) else 'blocked' if unverified_fonts else 'passed',{'missing_glyphs':missing,'embedded_missing_glyphs':embedded_missing,'unverified_fonts':unverified_fonts,'embedded_subset_hashes':[p['font_subset_sha256'] for p in pages],'native_font_hashes_verified':False}))
    evidence = [{'page':i,**e} for i,p in enumerate(pages,1) for e in p['glyph_evidence']]
    checks.append(layer('font_glyph_evidence',status(evidence) if evidence else 'blocked',{
        'issues':[e for e in evidence if e['status'] != 'passed'],
        'checked_characters':len(evidence),'coverage':'embedded generic metadata and simple TrueType outlines; not rasterized ink or native character fidelity'}))
    actual_hashes = {str(p.relative_to(folder)):sha(p) for paths in files.values() for p in paths}
    golden = expected.get('output_sha256',{})
    hash_ok = bool(golden) and actual_hashes == golden
    checks.append(layer('self_rendered_golden_hashes','blocked' if hashes and not golden else 'passed' if hashes and hash_ok else 'failed' if hashes else 'unrun',{'enabled':hashes,'role':'regression only; not native fidelity'}))
    native = None; native_error = 'owner-provided native bundle is missing'
    if reference_root and (Path(reference_root)/fixture['id']/'reference.json').exists():
        ref_dir = under(reference_root,fixture['id'])
        try:
            native = verify_reference(load(ref_dir/'reference.json'),ref_dir,fixture)
            if fixture['format'] == 'xlsx':
                native_pages = native.get('pages',[])
            else:
                pdf = next(r for r in native['files'] if r.get('role') == 'native-pdf')
                native_pages = extract_pdf_pages(under(ref_dir,pdf['path']))
            verify_native_geometry(native,native_pages,ref_dir)
            native = {**native,'pages':native_pages}
        except (ValueError,KeyError,OSError,ET.ParseError) as error:
            native = None
            native_error = str(error)
    if native:
        problems = []
        if len(native_pages) != count:
            problems.append('native page/window count differs')
        for i,(candidate,reference) in enumerate(zip(pages,native_pages),1):
            if reference['index'] != i or any(abs(a-b) > .5 for a,b in zip(dims[i-1],[reference['width_pt'],reference['height_pt']])):
                problems.append(f'native page {i} dimensions/enumeration differs')
            # Exact normalized character counts are a hard coverage check. They
            # cannot be outweighed by visual similarity or whitespace area.
            if Counter(candidate['text']) != Counter(norm(reference['text'])):
                problems.append(f'native page {i} text coverage differs')
        for address,value in fixture.get('critical_cells',{}).items():
            if native.get('cells',{}).get(address) != value['display']:
                problems.append('native cached cell differs: '+address)
        review = native.get('visual_review',{})
        reviewed = review.get('decision') == 'accept' and bool(review.get('reviewer')) and review.get('candidate_output_sha256') == actual_hashes
        checks.append(layer('native_semantics','failed' if problems else 'passed',problems))
        checks.append(layer('native_visual_review','passed' if reviewed else 'blocked','explicit review of these exact candidate hashes required; no automatic baseline approval'))
        if overlay:
            write_overlays(fixture,pages,native,ref_dir,Path(overlay)/fixture['id'])
    else:
        checks.append(layer('native_reference','blocked',native_error))
    after = sha(input_path)
    checks.append(layer('source_immutability','passed' if before == after else 'failed',{'before':before,'after':after}))
    regression_checks = [c for c in checks if not c['layer'].startswith('native_') and c['status'] != 'unrun']
    regression_status = status(regression_checks)
    return {'fixture_id':fixture['id'],'regression_status':regression_status,
            'native_status':status([c for c in checks if c['layer'] != 'self_rendered_golden_hashes' or hashes]) if native else 'blocked','fidelity_promoted':False,
            'checks':checks,'candidate_output_sha256':actual_hashes}


def status(checks):
    return 'failed' if any(c['status'] == 'failed' for c in checks) else 'blocked' if any(c['status'] in ('blocked','unrun') for c in checks) else 'passed'


def write_overlays(fixture,pages,native,ref_root,out):
    """Local, unapproved visual aid; all content embedded, no network or browser."""
    out.mkdir(parents=True,exist_ok=True)
    for i,page in enumerate(pages,1):
        image = next((r for r in native['files'] if r.get('role') == 'native-page-png' and r.get('page') == i),None)
        if not image:
            continue
        reference = bounded(under(ref_root,image['path']),MAX_SVG)
        candidate = bounded(page['path'],MAX_SVG)
        native_page = native['pages'][i-1] if native.get('pages') else None
        if native_page and any(abs(a-b) > .5 for a,b in zip([page['width_pt'],page['height_pt']],[native_page['width_pt'],native_page['height_pt']])):
            continue  # Dimensions already fail independently; never stretch evidence.
        body = f'''<!doctype html><meta charset="utf-8"><title>{html.escape(fixture['id'])} page {i}</title>
<p>Native reference / candidate overlay. Review aid only; text and geometry failures remain hard failures.</p>
<label>Candidate opacity <input type="range" min="0" max="1" step="0.05" value="0.5" oninput="document.getElementById('candidate').style.opacity=this.value"></label>
<div style="position:relative;width:{page['width_pt']}pt;height:{page['height_pt']}pt">
<img style="position:absolute;width:100%;height:100%" src="data:image/png;base64,{base64.b64encode(reference).decode()}">
<img id="candidate" style="position:absolute;width:100%;height:100%;opacity:.5" src="data:image/svg+xml;base64,{base64.b64encode(candidate).decode()}"></div>'''
        target = out/f'page-{i}.html'
        if target.exists():
            raise ValueError('overlay already exists; original evidence is never overwritten')
        target.write_text(body)


def write_json(path,data):
    path = Path(path)
    if path.exists():
        raise ValueError('output exists; evidence is never overwritten')
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n')


def corpus_load(path):
    corpus = load(path)
    if corpus.get('schema_version') != VERSION or corpus.get('kind') != 'hanji-preview-corpus':
        raise ValueError('hanji-preview-corpus schema_version 1 required')
    ids = [f['id'] for f in corpus['fixtures']]
    if len(ids) != len(set(ids)) or any(not re.fullmatch(r'[a-zA-Z0-9_-]+',s) for s in ids):
        raise ValueError('unique safe fixture IDs required')
    for f in corpus['fixtures']:
        if f['format'] not in ('docx','pptx','hwpx','xlsx') or not re.fullmatch('[a-f0-9]{64}',f['input']['sha256']):
            raise ValueError('supported format and SHA-256 required')
        if f.get('rights',{}).get('local_validation') is not True:
            raise ValueError('local fixture-validation permission required')
        if f['format'] == 'xlsx' and not all(f.get('window',{}).get(k) for k in ('sheet','range')):
            raise ValueError('XLSX requires named sheet and bounded range')
    return corpus


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus',type=Path,default=HERE/'corpus-v1.json')
    parser.add_argument('--repo',type=Path,default=HERE.parents[2])
    sub = parser.add_subparsers(dest='command',required=True)
    inv = sub.add_parser('inventory'); inv.add_argument('--out',type=Path)
    test = sub.add_parser('audit'); test.add_argument('--artifacts',type=Path,required=True)
    test.add_argument('--fixture',action='append'); test.add_argument('--references',type=Path)
    test.add_argument('--hash-goldens',action='store_true'); test.add_argument('--overlays',type=Path)
    test.add_argument('--out',type=Path,required=True); test.add_argument('--require-native',action='store_true')
    run_parser = sub.add_parser('capture'); run_parser.add_argument('--hanji',type=Path,required=True)
    run_parser.add_argument('--fixture',action='append',required=True); run_parser.add_argument('--out',type=Path,required=True)
    run_parser.add_argument('--font-dir',type=Path)
    ingest_parser = sub.add_parser('ingest-native'); ingest_parser.add_argument('--metadata',type=Path,required=True)
    ingest_parser.add_argument('--root',type=Path,required=True); ingest_parser.add_argument('--out',type=Path,required=True)
    args = parser.parse_args(argv)
    try:
        corpus = corpus_load(args.corpus)
        if args.command == 'inventory':
            data = [{'id':f['id'],'input_matches':sha(under(args.repo,f['input']['path'])) == f['input']['sha256'],
                     'rights':f['rights'],'native_status':f['native_reference']['status']} for f in corpus['fixtures']]
            if args.out:
                write_json(args.out,data)
            print(json.dumps({'fixtures':len(data),'matching_inputs':sum(x['input_matches'] for x in data),'native_available':sum(x['native_status'] == 'available' for x in data)}))
            return 0 if all(x['input_matches'] for x in data) else 1
        if args.command == 'ingest-native':
            ingest(args.metadata,corpus,args.root,args.out); return 0
        fixtures = corpus['fixtures']
        if args.fixture:
            fixtures = [f for f in fixtures if f['id'] in args.fixture]
            if {f['id'] for f in fixtures} != set(args.fixture):
                raise ValueError('unknown fixture ID')
        if args.command == 'capture':
            record = capture(fixtures,args.repo,args.hanji,args.out,args.font_dir)
            print(json.dumps({'runs':len(record['runs']),'failed':sum(r['exit_code'] != 0 for r in record['runs'])}))
            return 0 if all(r['exit_code'] == 0 for r in record['runs']) else 1
        results = []
        for fixture in fixtures:
            try:
                results.append(audit(fixture,args.repo,args.artifacts,args.references,args.hash_goldens,args.overlays))
            except (ValueError,KeyError,OSError,ET.ParseError,struct.error,IndexError) as error:
                results.append({'fixture_id':fixture['id'],'regression_status':'failed','native_status':'blocked','error':str(error)})
        write_json(args.out,{'schema_version':VERSION,'corpus_version':corpus['corpus_version'],'results':results})
        summary = Counter(r['native_status' if args.require_native else 'regression_status'] for r in results)
        print(json.dumps(dict(summary)))
        return 1 if summary['failed'] else 2 if summary['blocked'] else 0
    except NativeBlocked as error:
        parser.exit(2,str(error)+'\n')
    except (ValueError,KeyError,OSError,StopIteration,subprocess.SubprocessError,ET.ParseError) as error:
        parser.exit(1,str(error)+'\n')


if __name__ == '__main__':
    sys.exit(main())
