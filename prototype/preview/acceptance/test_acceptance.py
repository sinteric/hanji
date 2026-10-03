"""Adversarial unit fixtures are test doubles, never native/golden evidence."""
import base64
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

import acceptance as a


def metric_stub(letters=True):
    head = bytearray(20); struct.pack_into('>H',head,18,1000)
    hhea = bytearray(36); struct.pack_into('>H',hhea,34,96)
    # Format 4 with ASCII range and required sentinel; or SPACE only.
    start,end = (32,126) if letters else (32,32)
    sub = struct.pack('>7H',4,32,0,4,4,1,0)
    sub += struct.pack('>3H',end,65535,0)+struct.pack('>2H',start,65535)
    sub += struct.pack('>2h',1-start,1)+struct.pack('>2H',0,0)
    cmap = struct.pack('>2H2HI',0,1,0,3,12)+sub
    tables = {'head':bytes(head),'hhea':bytes(hhea),'hmtx':struct.pack('>HH',500,0)*96,'cmap':cmap}
    result = bytearray(struct.pack('>I4H',0x10000,4,0,0,0)); offset = 12+16*len(tables)
    for tag,data in tables.items():
        result += struct.pack('>4sIII',tag.encode(),0,offset,len(data)); offset += len(data)
    return bytes(result)+b''.join(tables.values())


def svg(text='CODE42',x=10,y=30,extra='',letters=True):
    font = base64.b64encode(metric_stub(letters)).decode()
    positions = ' '.join(str(x+6*i) for i in range(len(text)))
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="100pt" height="80pt" viewBox="0 0 100 80">
<style>@font-face{{font-family:'unit-font';src:url('data:font/ttf;base64,{font}')}}</style>{extra}
<text font-family="unit-font" font-size="12" x="{positions}" y="{y}">{text}</text></svg>'''


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name); (self.root/'source.docx').write_bytes(b'unit-source')
        self.fixture = {'id':'unit','format':'docx','input':{'path':'source.docx','sha256':a.sha(self.root/'source.docx')},
                        'artifact_stem':'unit','rights':{'local_validation':True},'native_reference':{'app':'Word'},
                        'critical_text':[{'id':'body','page':1,'value':'CODE42'}],
                        'regression':{'pages':1,'dimensions_pt':[[100,80]],'diagnostics':{'allow':[],'required':[]},'missing_glyphs':[]}}
        self.folder = self.root/'artifacts/unit'; self.write_artifacts(svg())

    def write_artifacts(self,xml,report_extra=None):
        for kind in ('svg','png','html'):
            folder = self.folder/kind; folder.mkdir(parents=True,exist_ok=True)
            path = folder/('unit-r1-page-1.'+kind if kind != 'html' else 'unit-preview.html')
            data = xml.encode() if kind == 'svg' else ('<section>'+xml+'</section>').encode() if kind == 'html' else b'\x89PNG\r\n\x1a\n'+struct.pack('>I',13)+b'IHDR'+struct.pack('>II',133,107)
            path.write_bytes(data)
            report = {'files':[str(path)],'pages':1,'fonts':{'missing_glyphs':[]},'warnings':[]}
            report.update(report_extra or {})
            (folder/'result.json').write_text(json.dumps(report))

    def audit(self):
        return a.audit(self.fixture,self.root,self.root/'artifacts')

    def check(self,name):
        return next(c for c in self.audit()['checks'] if c['layer'] == name)

    def reference(self):
        (self.root/'native.pdf').write_bytes(b'unit-pdf-hash-double')
        (self.root/'font.ttf').write_bytes(metric_stub())
        (self.root/'native.png').write_bytes(b'\x89PNG\r\n\x1a\n'+struct.pack('>I',13)+b'IHDR'+struct.pack('>II',133,107))
        return {'schema_version':1,'kind':'native-application','fixture_id':'unit','source_sha256':self.fixture['input']['sha256'],
                'app':{'name':'Word','version':'unit-test-double','version_confirmed':True,'os':'unit-test-double'},
                'export':{'mode':'no-markup','repair_prompt':False,'fields_updated':False},'rights':{'local_validation':True},
                'files':[{'path':'native.pdf','sha256':a.sha(self.root/'native.pdf'),'role':'native-pdf'},
                         {'path':'native.png','sha256':a.sha(self.root/'native.png'),'role':'native-page-png','page':1}],
                'fonts':[{'path':'font.ttf','sha256':a.sha(self.root/'font.ttf'),'family':'unit-font'}]}

    def test_regression_can_pass_while_native_is_blocked(self):
        result = self.audit()
        self.assertEqual(result['regression_status'],'passed')
        self.assertEqual(result['native_status'],'blocked')
        self.assertFalse(result['fidelity_promoted'])

    def test_missing_critical_text_is_not_rescued_by_matching_images(self):
        self.write_artifacts(svg('CODE'))
        self.assertEqual(self.check('critical_text_values')['status'],'failed')

    def test_subset_missing_letters_fails_even_with_empty_missing_report(self):
        self.write_artifacts(svg(letters=False))
        result = self.check('font_coverage')
        self.assertEqual(result['status'],'failed')
        self.assertIn('U+0043',result['details']['embedded_missing_glyphs'])

    def test_critical_text_outside_page_fails(self):
        self.write_artifacts(svg(x=90))
        self.assertEqual(self.check('critical_geometry')['status'],'failed')

    def test_critical_clip_fails(self):
        xml = svg().replace('<text ','<g clip-path="url(#clip)"><text ').replace('</text>','</text></g>')
        xml = xml.replace('<style>','<defs><clipPath id="clip"><rect x="0" y="0" width="15" height="80"/></clipPath></defs><style>')
        self.write_artifacts(xml)
        self.assertEqual(self.check('critical_geometry')['status'],'failed')

    def test_overlap_between_declared_semantic_regions_fails(self):
        extra = '<text font-family="unit-font" font-size="12" x="10 16 22" y="30">XYZ</text>'
        self.write_artifacts(svg(extra=extra))
        self.fixture['critical_text'].append({'id':'other','page':1,'value':'XYZ'})
        self.fixture['non_overlap'] = [['body','other']]
        self.assertEqual(self.check('critical_geometry')['status'],'failed')

    def test_duplicate_ids_and_unresolved_reference_fail(self):
        self.write_artifacts(svg(extra='<g id="same"/><g id="same" clip-path="url(#missing)"/>'))
        self.assertEqual(self.check('svg_html_structure')['status'],'failed')

    def test_changed_input_hash_fails_before_reading_artifacts(self):
        (self.root/'source.docx').write_bytes(b'changed')
        self.assertEqual(self.audit()['regression_status'],'failed')

    def test_missing_second_page_fails_enumeration(self):
        self.fixture['regression']['pages'] = 2
        self.assertEqual(self.check('enumeration_dimensions')['status'],'failed')

    def test_new_diagnostic_requires_explicit_review(self):
        self.write_artifacts(svg(),{'warnings':['unsupported object silently dropped']})
        self.assertEqual(self.check('diagnostics')['status'],'failed')

    def test_html_only_diagnostic_is_not_hidden_by_svg_report(self):
        path = self.folder/'html/result.json'; report = a.load(path)
        report['warnings'] = ['HTML formatter discarded content']; path.write_text(json.dumps(report))
        self.assertEqual(self.check('diagnostics')['status'],'failed')

    def test_required_diagnostic_disappearing_also_needs_review(self):
        self.fixture['regression']['diagnostics']['required'] = ['cached value']
        self.assertEqual(self.check('diagnostics')['status'],'failed')

    def test_cached_value_must_match_display_and_cache_contract(self):
        self.fixture['format'] = 'xlsx'; self.fixture['window'] = {'sheet':'Sheet1','range':'A1:B2'}
        self.fixture['critical_cells'] = {'A1':{'display':'42','formula_result':'cached-unverified'}}
        self.write_artifacts(svg(),{'sheet':{'name':'Sheet1'},'range':'A1:B2','cells':[{'address':'A1','display':'41','formula_result':'cached-unverified'}]})
        self.assertEqual(self.check('critical_text_values')['status'],'failed')

    def test_wrong_native_source_is_rejected(self):
        ref = self.reference(); ref['source_sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError,'different source'):
            a.verify_reference(ref,self.root,self.fixture)

    def test_self_rendered_reference_is_rejected(self):
        ref = self.reference(); ref['kind'] = 'self-rendered-regression'
        with self.assertRaisesRegex(ValueError,'not native'):
            a.verify_reference(ref,self.root,self.fixture)

    def test_native_font_tampering_is_rejected(self):
        ref = self.reference(); (self.root/'font.ttf').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'hash mismatch'):
            a.verify_reference(ref,self.root,self.fixture)

    def test_workbook_print_pdf_is_not_a_window_reference(self):
        ref = self.reference(); fixture = deepcopy(self.fixture); fixture['format'] = 'xlsx'
        fixture['native_reference']['app'] = 'Excel'; ref['app']['name'] = 'Excel'; ref['export']['mode'] = 'workbook-print'
        with self.assertRaisesRegex(ValueError,'worksheet-window'):
            a.verify_reference(ref,self.root,fixture)

    def test_reference_app_version_must_be_confirmed(self):
        ref = self.reference(); ref['app']['version_confirmed'] = False
        with self.assertRaisesRegex(ValueError,'confirmed'):
            a.verify_reference(ref,self.root,self.fixture)

    def test_paths_cannot_escape_artifact_root(self):
        with self.assertRaisesRegex(ValueError,'escapes'):
            a.under(self.root,'../outside')

    def test_outputs_never_overwrite_evidence(self):
        path = self.root/'result.json'; a.write_json(path,{'original':True})
        with self.assertRaisesRegex(ValueError,'never overwritten'):
            a.write_json(path,{'original':False})

    def test_empty_golden_set_is_blocked_when_requested(self):
        r = a.audit(self.fixture,self.root,self.root/'artifacts',hashes=True)
        self.assertEqual(next(c['status'] for c in r['checks'] if c['layer'] == 'self_rendered_golden_hashes'),'blocked')

    def test_native_text_loss_and_stale_review_cannot_pass(self):
        ref = self.reference(); ref['pages'] = [{'index':1,'width_pt':100,'height_pt':80,'text':'CODE42 MISSING'}]
        ref['visual_review'] = {'decision':'accept','reviewer':'unit-owner','candidate_output_sha256':{'stale':'0'*64}}
        target = self.root/'refs/unit'; target.mkdir(parents=True)
        for name in ('native.pdf','native.png','font.ttf'):
            (target/name).write_bytes((self.root/name).read_bytes())
        a.write_json(target/'reference.json',ref)
        with patch('acceptance.extract_pdf_pages',return_value=ref['pages']):
            result = a.audit(self.fixture,self.root,self.root/'artifacts',reference_root=self.root/'refs')
        self.assertEqual(next(c['status'] for c in result['checks'] if c['layer'] == 'native_semantics'),'failed')
        self.assertEqual(next(c['status'] for c in result['checks'] if c['layer'] == 'native_visual_review'),'blocked')

    def test_native_text_is_extracted_again_from_the_hashed_pdf(self):
        ref = self.reference(); ref['pages'] = [{'index':1,'width_pt':100,'height_pt':80,'text':'CODE42'}]
        target = self.root/'refs/unit'; target.mkdir(parents=True)
        for name in ('native.pdf','native.png','font.ttf'):
            (target/name).write_bytes((self.root/name).read_bytes())
        a.write_json(target/'reference.json',ref)
        actual = [{'index':1,'width_pt':100,'height_pt':80,'text':'CODE42 MISSING'}]
        with patch('acceptance.extract_pdf_pages',return_value=actual) as extractor:
            result = a.audit(self.fixture,self.root,self.root/'artifacts',reference_root=self.root/'refs')
        extractor.assert_called_once()
        self.assertEqual(next(c['status'] for c in result['checks'] if c['layer'] == 'native_semantics'),'failed')

    def test_conservative_clip_envelope_blocks_without_claiming_ink_loss(self):
        xml = svg().replace('<text ','<g clip-path="url(#clip)"><text ').replace('</text>','</text></g>')
        xml = xml.replace('<style>','<defs><clipPath id="clip"><rect x="0" y="25" width="100" height="55"/></clipPath></defs><style>')
        self.write_artifacts(xml)
        self.assertEqual(self.check('critical_geometry')['status'],'blocked')

    def test_unknown_embedded_face_blocks_font_verification(self):
        self.write_artifacts(svg().replace('font-family="unit-font"','font-family="unknown-face"'))
        self.assertEqual(self.check('font_coverage')['status'],'blocked')

    def test_tspan_positions_cannot_be_credited_as_verified_geometry(self):
        self.write_artifacts(svg().replace('>CODE42</text>','><tspan x="99">CODE42</tspan></text>'))
        self.assertEqual(self.check('critical_geometry')['status'],'blocked')

    def test_native_missing_pdf_role_blocks_cleanly(self):
        ref = self.reference(); ref['files'] = [r for r in ref['files'] if r['role'] != 'native-pdf']
        with self.assertRaisesRegex(ValueError,'exactly one'):
            a.verify_reference(ref,self.root,self.fixture)

    def test_reference_with_updated_fields_is_not_original_source_oracle(self):
        ref = self.reference(); ref['export']['fields_updated'] = True
        with self.assertRaisesRegex(ValueError,'no updated fields'):
            a.verify_reference(ref,self.root,self.fixture)

    def test_native_raster_cannot_be_stretched_into_matching_page(self):
        ref = self.reference()
        pages = [{'index':1,'width_pt':200,'height_pt':80,'text':'CODE42'}]
        with self.assertRaisesRegex(a.NativeBlocked,'96-DPI'):
            a.verify_native_geometry(ref,pages,self.root)

    def test_native_pngs_must_enumerate_every_page(self):
        ref = self.reference()
        pages = [{'index':1,'width_pt':100,'height_pt':80,'text':'CODE42'},
                 {'index':2,'width_pt':100,'height_pt':80,'text':'CODE42'}]
        with self.assertRaisesRegex(a.NativeBlocked,'every page'):
            a.verify_native_geometry(ref,pages,self.root)

    def test_missing_pdf_tool_blocks_native_without_failing_regression(self):
        ref = self.reference(); target = self.root/'refs/unit'; target.mkdir(parents=True)
        for name in ('native.pdf','native.png','font.ttf'):
            (target/name).write_bytes((self.root/name).read_bytes())
        a.write_json(target/'reference.json',ref)
        with patch('acceptance.shutil.which',return_value=None):
            result = a.audit(self.fixture,self.root,self.root/'artifacts',reference_root=self.root/'refs')
        self.assertEqual(result['regression_status'],'passed')
        self.assertEqual(result['native_status'],'blocked')
        self.assertIn('pdftotext unavailable',next(c['details'] for c in result['checks'] if c['layer'] == 'native_reference'))

    def test_native_pass_requires_current_hash_review_and_never_promotes_support(self):
        ref = self.reference(); target = self.root/'refs/unit'; target.mkdir(parents=True)
        for name in ('native.pdf','native.png','font.ttf'):
            (target/name).write_bytes((self.root/name).read_bytes())
        ref['visual_review'] = {'decision':'accept','reviewer':'unit-owner','candidate_output_sha256':self.audit()['candidate_output_sha256']}
        a.write_json(target/'reference.json',ref)
        pages = [{'index':1,'width_pt':100,'height_pt':80,'text':'CODE42'}]
        with patch('acceptance.extract_pdf_pages',return_value=pages):
            result = a.audit(self.fixture,self.root,self.root/'artifacts',reference_root=self.root/'refs')
        self.assertEqual(result['native_status'],'passed')
        self.assertFalse(result['fidelity_promoted'])


if __name__ == '__main__':
    unittest.main()
