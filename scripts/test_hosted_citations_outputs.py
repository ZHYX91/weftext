"""Real HTTP citation and frozen-output acceptance on owned libraries.

These cases exercise the hosted boundary; ZIP inspection is not Word/WPS GUI
acceptance. No user documents or desktop input are used.
"""
import uuid
import io
import base64
import json
import sys
import subprocess
import unittest
import zipfile
import xml.etree.ElementTree as ET

from test_hosted import EXE, digest, uid
from test_hosted_office import HostedOffice
from test_hosted_query import HostedQuery


class HostedCitationsOutputs(HostedOffice, HostedQuery):
    PRIVATE = 'PRIVATE_WORK_BODY_NEVER_IN_PUBLICATION_OUTPUT'

    def prepare_citations(self):
        doc = self.open('Other')
        node = self.docs['Other']['nodeId']
        self.work_source = (
            f'#metadata((id: "{node}", types: ("paper",), publication: '
            '(kind: "article", title: "Granted Publication", contributors: '
            '((agent: (text: "Author Alpha"), roles: ("author",)), '
            '(agent: (text: "Author Beta"), roles: ("author",)),)))) <weftext>\n'
            + self.PRIVATE + '\n')
        self.save(doc, self.work_source)
        self.work_ref = self.call('navigation/ref', {'sessionId': self.open('Other')['sessionId']})['reference']
        def grants(users):
            users['alice']['grants']['Other'] = {
                'fields': {'library/publication': {'read': True}},
                'citations': {'use': True, 'export': True, 'download': True},
                'outputs': {'bibliography': {'export': True, 'download': True}}}
            users['alice']['grants']['Shared'].update(
                office={'export': True, 'download': True},
                outputs={'pdf': {'export': True, 'download': True},
                         'bibliography': {'export': True, 'download': True}})
        self.policy(grants)

    def basis(self, doc=None, source=None, serial=1, actor='alice'):
        doc = doc or self.open(actor=actor)
        source = doc['source'] if source is None else source
        basis = dict(sessionId=doc['sessionId'], source=source,
                     serial=serial, sourceHash=digest(source))
        self.call('editor/map', basis, actor)
        return {**basis, 'baseId': doc['baseId']}

    @staticmethod
    def request(format, **extra):
        return {'outputLocale': 'zh-CN', 'requestId': uid(), 'format': format, 'reviewMode': 'none',
                    'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}, **extra}

    def prepare_output(self, req, actor='alice', status=200):
        return self.call('export/prepare', req, actor, status)

    def proposal(self, basis, key='actualKey', actor='alice', status=200):
        return self.call('citations/propose', {k: v for k, v in
            {**basis, 'key': key, 'work': 'Other'}.items() if k != 'baseId'}, actor, status)

    def assert_publication_only(self, value):
        text = value if isinstance(value, str) else json.dumps(value, ensure_ascii=False)
        self.assertIn('Granted Publication', text)
        for hidden in [self.PRIVATE, digest(self.work_source), str(self.root), 'HIDDEN_PAYLOAD_987']:
            self.assertNotIn(hidden, text)

    def apply_utf16(self, original, proposal):
        units = original.encode('utf-16-le')
        for patch in sorted(proposal['patches'], key=lambda p: p['from'], reverse=True):
            self.assertEqual(units[patch['from']*2:patch['to']*2].decode('utf-16-le'), patch['expected'])
            units = units[:patch['from']*2]+patch['text'].encode('utf-16-le')+units[patch['to']*2:]
        self.assertEqual(units.decode('utf-16-le'), proposal['source'])

    def test_native_citation_controls_reuse_forms_actual_products_and_stale_basis(self):
        self.prepare_citations()
        doc = self.open()
        original = (self.root/'Shared/Shared.typ').read_bytes()
        source = doc['source']+'\n正文 😀\n'
        basis = self.basis(doc, source, 10)
        raw = {key: value for key, value in basis.items() if key != 'baseId'}
        profile = self.call('citations/editor', raw)['profile']
        self.assert_publication_only(profile)
        self.assertTrue(any(choice['reference'] == self.work_ref and choice['citationReady']
                            for choice in profile['choices']))
        intent = dict(kind='nativeCitationInsert', at=len(source[:source.index('正文')].encode()),
                      work=self.work_ref, key='paper2026', form='prose',
                      supplement='第 12 页 ] #panic("must stay plain text")')
        proposal = self.call('citations/edit', {**raw, 'generation': profile['generation'], 'intent': intent})
        self.apply_utf16(source, proposal)
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), original)
        source = proposal['source']
        basis = self.basis(doc, source, 11)
        raw = {key: value for key, value in basis.items() if key != 'baseId'}
        profile = self.call('citations/editor', raw)['profile']
        self.assertEqual(len(profile['editor']['bindings']), 1)
        self.assertEqual(profile['editor']['bindings'][0]['work'], self.work_ref)
        self.assertEqual(profile['displayStatus'], 'nativeCompileAvailable')
        self.assertTrue(any(item['key'] == 'paper2026' and item['form'] == 'prose'
                            for item in profile['displayed']))
        self.assert_publication_only(profile)
        second = {**raw, 'generation': profile['generation'], 'intent': dict(
            kind='nativeCitationInsert', at=len(source[:source.index('正文')].encode()),
            work=self.work_ref, form='none', supplement='')}
        proposal = self.call('citations/edit', second)
        self.apply_utf16(source, proposal)
        basis2 = self.basis(doc, proposal['source'], 12)
        final = self.call('citations/editor', {key: value for key, value in basis2.items() if key != 'baseId'})['profile']
        self.assertEqual(len(final['editor']['bindings']), 1)
        self.assertEqual(len(final['editor']['citations']), 2)
        self.assertTrue(any(item['visibility'] == 'bibliographyOnly' for item in final['displayed']))
        self.call('citations/edit', second, status=409)
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), original)

    def test_citation_projection_exact_proposal_and_same_saved_identity(self):
        self.prepare_citations()
        def readonly(users):
            for user in users.values():
                if 'Other' in user['grants']:
                    user['grants']['Other']['write'] = False
        self.policy(readonly)
        self.config['users'] = self.call('admin/policy', actor='admin')['users']
        self.server.close(); self.config['files']['Other']['editable'] = False; self.start()
        self.call('open', {'resource': 'Other'}, status=404)
        works = self.call('citations')['works']
        self.assertEqual([(w['resource'], w['reference']) for w in works], [('Other', self.work_ref)])
        self.assertEqual(self.call('citations', actor='bob')['works'], [])
        doc = self.open(); before = (self.root/'Shared/Shared.typ').read_bytes()
        basis = self.basis(doc, doc['source']+'\nUNSAVED_INPUT😀', 7)
        proposed = self.proposal(basis)
        self.assertEqual(proposed['serial'], 7)
        self.assertEqual(proposed['sourceHash'], basis['sourceHash'])
        self.assertIn(self.work_ref, proposed['source'])
        self.assertIn('@actualKey', proposed['source'])
        self.assertNotIn(self.PRIVATE, proposed['source'])
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), before)
        # Returned UTF-16 edits must reproduce the exact proposed draft.
        self.apply_utf16(basis['source'], proposed)
        self.basis(doc, basis['source']+' newer', 8)
        self.basis(doc, basis['source'], 9)
        self.proposal(basis, status=409)
        self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': doc['baseId'], 'inputRevision': 1, 'sessionId': doc['sessionId'], 'source': proposed['source']})
        self.save(doc, proposed['source'])
        reopened = self.open()
        identity = self.call('navigation/ref', {'sessionId': reopened['sessionId']})['nodeRef']
        self.assertEqual(identity['nodeId'], self.docs['Shared']['nodeId'])
        second = self.proposal(self.basis(reopened, serial=1), key='secondKey')
        self.apply_utf16(reopened['source'], second)
        self.assertIn('@actualKey', second['source'])
        self.assertIn('@secondKey', second['source'])
        self.assertTrue(self.call('preview', {'sessionId': reopened['sessionId'], 'source': second['source']})['ok'])
        minimal = f'#metadata((id: "{identity["nodeId"]}", types: ())) <weftext>'
        minimal_proposal = self.proposal(self.basis(reopened, minimal, 2))
        self.apply_utf16(minimal, minimal_proposal)
        self.assertTrue(self.call('preview', {'sessionId': reopened['sessionId'], 'source': minimal_proposal['source']})['ok'])
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), proposed['source'])

    def test_bibliography_formats_freeze_publication_without_source_read(self):
        self.prepare_citations()
        frozen = []
        for format in ['bibtex', 'csl-json', 'ris']:
            req = self.request(format, bibliographyScope='selectedWorks', works=['Other'])
            result = self.prepare_output(req)
            self.assertEqual(result['status'], 'prepared')
            self.assert_publication_only(result['preview'])
            frozen.append((req, result))
        bob = self.open('Other', 'bob')
        self.save(bob, bob['source'].replace('Granted Publication', 'Newer Publication'), 'bob')
        for req, result in frozen:
            text = self.deliver(result).decode('utf-8')
            self.assert_publication_only(text)
            self.assertNotIn('Newer Publication', text)
            self.assertLess(text.index('Author Alpha'), text.index('Author Beta'))
            self.assertEqual(self.prepare_output(req)['digest'], result['digest'])
            self.prepare_output({**req, 'works': ['Other', 'Secret']}, status=409)
        req, result = frozen[0]
        self.assertEqual(self.call('export/status', {'requestId': req['requestId']}, 'bob')['status'], 'notRegistered')
        self.server.close(); self.start()
        self.assertEqual(self.prepare_output(req)['status'], 'expired')
        self.call('export/chunk', {'requestId': req['requestId'], 'offset': 0}, status=409)

    def test_bibliography_separate_grants_and_revocation_every_delivery(self):
        self.prepare_citations()
        req = self.request('bibtex', bibliographyScope='selectedWorks', works=['Other'])
        result = self.prepare_output(req)
        self.policy(lambda u: u['alice']['grants']['Other']['citations'].update(download=False))
        expired = self.call('export/confirm', {'requestId': result['requestId'], 'digest': result['digest'], 'acceptLosses': True})
        self.assertEqual(expired['status'], 'expired')
        self.call('export/chunk', {'requestId': result['requestId'], 'offset': 0}, status=409)
        # Preparing and delivering are independently granted.
        pending = self.prepare_output({**req, 'requestId': uid()})
        self.call('export/confirm', {'requestId': pending['requestId'], 'digest': pending['digest'], 'acceptLosses': True}, status=404)
        self.policy(lambda u: u['alice']['grants']['Other']['fields']['library/publication'].update(read=False))
        self.assertEqual(self.call('citations')['works'], [])
        self.prepare_output({**req, 'requestId': uid()}, status=404)
        self.prepare_output({**req, 'requestId': uid(), 'works': ['NotConfigured']}, status=404)

    def test_pdf_exact_draft_and_docx_share_generated_citation_input(self):
        self.prepare_citations()
        doc = self.open(); basis = self.basis(doc, doc['source']+'\nCaptured unsaved body', 3)
        proposed = self.proposal(basis)
        exact = self.basis(doc, proposed['source'], 4)
        pdf_req = self.request('pdf', resource='Shared', **exact)
        missing = {k: v for k, v in pdf_req.items() if k != 'serial'}
        self.prepare_output(missing, status=400)
        rendered = self.call('preview', {k: exact[k] for k in ['sessionId', 'source']})
        self.assertTrue(rendered['ok'], rendered)
        pdf = self.prepare_output(pdf_req)
        word = self.prepare_output(self.request('docx', resource='Shared', **exact))
        self.assertTrue(self.deliver(pdf).startswith(b'%PDF-'))
        xml = self.parts(self.deliver(word))['word/document.xml']
        text = ''.join(ET.fromstring(xml).itertext())
        self.assertIn('Captured unsaved body', text)
        self.assertIn('Granted Publication', text)
        self.assertNotIn(self.PRIVATE, text)
        self.assertNotIn('Captured unsaved body', (self.root/'Shared/Shared.typ').read_text('utf-8'))
        self.basis(doc, exact['source']+' next', 5)
        self.prepare_output({**pdf_req, 'requestId': uid()}, status=409)

    def test_generated_input_collision_preserves_real_file_without_registering_output(self):
        self.prepare_citations()
        doc = self.open(); proposed = self.proposal(self.basis(doc))
        exact = self.basis(doc, proposed['source'], 2)
        real = self.root/'Shared/weftext.citations.bib'
        real.write_bytes(b'RAW_REAL_BIB_MUST_SURVIVE')
        req = self.request('pdf', resource='Shared', **exact)
        self.prepare_output(req, status=400)
        self.assertEqual(real.read_bytes(), b'RAW_REAL_BIB_MUST_SURVIVE')
        self.assertNotIn('weftext.citations', (self.root/'Shared/Shared.typ').read_text('utf-8'))
        # Input qualification fails before an output execution is registered.
        self.assertEqual(self.call('export/status', {'requestId': req['requestId']})['status'], 'notRegistered')
        self.prepare_output(req, status=400)

    def test_native_whole_library_explicit_grant_and_saved_generated_bib(self):
        self.prepare_citations()
        doc = self.open(); proposed = self.proposal(self.basis(doc))
        self.save(doc, proposed['source'])
        doc = self.open()
        self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': doc['baseId'], 'inputRevision': 1, 'sessionId': doc['sessionId'], 'source': doc['source']+'\nPRIVATE_UNSAVED_NATIVE_DRAFT'})
        (self.root/'raw-private.txt').write_text('RAW_UNADMITTED_MUST_NOT_EXPORT', 'utf-8')
        req = self.request('native')
        self.prepare_output(req, status=404)
        self.server.close(); self.config['nativeWholeLibraryExport'] = True; self.start()
        self.prepare_output({**req, 'requestId': uid()}, status=404)
        self.policy(lambda u: u['alice'].update(nativeWholeLibrary={'read': True, 'export': True, 'download': True}))
        result = self.prepare_output(self.request('native'))
        self.assertEqual(result['preview']['scope'], 'savedWholeLibrary')
        with zipfile.ZipFile(io.BytesIO(self.deliver(result))) as archive:
            manifest = json.loads(archive.read('manifest.json'))
            entries = manifest['entries']
            generated = [(p, e) for p, e in entries.items() if e['role'] == 'generatedBibliography']
            self.assertEqual(len(generated), 1)
            path, entry = generated[0]
            self.assertTrue(path.endswith('Shared/weftext.citations.bib'))
            for key in ['nodeId', 'resourceId', 'revision']:
                self.assertNotIn(key, entry)
            self.assertEqual(entry['ownerNodeId'], self.docs['Shared']['nodeId'])
            self.assert_publication_only(archive.read(path).decode('utf-8'))
            all_bytes = b'\n'.join(archive.read(p) for p in archive.namelist())
            self.assertNotIn(b'RAW_UNADMITTED_MUST_NOT_EXPORT', all_bytes)
            self.assertNotIn(b'PRIVATE_UNSAVED_NATIVE_DRAFT', all_bytes)
            # A fresh isolated compiler receives only the delivered package.
            # It cannot obtain missing inputs from the live hosted library.
            frozen = {p: base64.b64encode(archive.read(p)).decode() for p in entries}
            worker = dict(source=archive.read('Shared/Shared.typ').decode('utf-8'),
                          path='Shared/Shared.typ', root=str(self.folder/'offline-package-root'),
                          paged=True, frozen_files=frozen)
        (self.folder/'offline-package-root').mkdir()
        compiled = subprocess.run([str(EXE), '--render-worker'], input=json.dumps(worker),
            capture_output=True, encoding='utf-8', timeout=40,
            creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        (self.folder/'offline-compile.stdout.json').write_text(compiled.stdout, 'utf-8')
        (self.folder/'offline-compile.stderr.log').write_text(compiled.stderr, 'utf-8')
        self.assertEqual(compiled.returncode, 0, compiled.stderr)
        paged = json.loads(compiled.stdout)
        self.assertTrue(paged['ok'], paged)
        self.assertGreater(paged['pageCount'], 0)
        self.policy(lambda u: u['alice']['nativeWholeLibrary'].update(download=False))
        self.call('export/chunk', {'requestId': result['requestId'], 'offset': 0}, status=409)

    def test_field_only_xlsx_uses_cells_without_exposing_private_source(self):
        self.prepare_fields()
        self.policy(lambda u: u['fields']['grants']['Shared'].update(office={'export': True, 'download': True}))
        req = self.request('xlsx', resource='Shared')
        req['bindings'] = {'Status': {'kind': 'field', 'field': 'tasks/status'}}
        result = self.prepare_output(req, 'fields')
        # Invoke the original fields helper, not the citation proposal method.
        from test_hosted_fields import HostedFields
        self.call('fields/write', HostedFields.proposal(self, self.field(), 'done'), 'fields')
        xml = '\n'.join(self.parts(self.deliver(result, 'fields')).values())
        self.assertIn('open', xml)
        for hidden in ['初始共同正文', 'hidden-value', 'HIDDEN_PAYLOAD_987', 'tasks/due', str(self.root)]:
            self.assertNotIn(hidden, xml)
        req['bindings']['Status']['field'] = 'tasks/due'
        self.prepare_output({**req, 'requestId': uid()}, 'fields', 404)

    def test_field_only_docx_template_uses_original_field_value(self):
        self.prepare_office()
        original = self.deliver(self.output()[0])
        template_bytes = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(original)) as original_zip, zipfile.ZipFile(template_bytes, 'w') as template:
            for entry in original_zip.infolist():
                data = original_zip.read(entry.filename)
                if entry.filename == 'word/document.xml':
                    xml = data.decode('utf-8')
                    start = xml.index('<w:body>') + len('<w:body>')
                    end = xml.index('<w:sectPr>')
                    control = '<w:sdt><w:sdtPr><w:tag w:val="Status"/></w:sdtPr><w:sdtContent><w:p><w:r><w:t>OLD_SLOT</w:t></w:r></w:p></w:sdtContent></w:sdt>'
                    data = (xml[:start]+control+xml[end:]).encode('utf-8')
                template.writestr(entry.filename, data)
        self.prepare_fields()
        self.policy(lambda u: u['fields']['grants']['Shared'].update(office={'export': True, 'download': True}))
        req = self.request('docx', resource='Shared', template={'name': 'owned.docx', 'bytesBase64': base64.b64encode(template_bytes.getvalue()).decode()})
        req['bindings'] = {'Status': {'kind': 'field', 'field': 'tasks/status'}}
        result = self.prepare_output(req, 'fields')
        from test_hosted_fields import HostedFields
        self.call('fields/write', HostedFields.proposal(self, self.field(), 'done'), 'fields')
        text = ''.join(ET.fromstring(self.parts(self.deliver(result, 'fields'))['word/document.xml']).itertext())
        self.assertIn('open', text)
        for hidden in ['done', 'OLD_SLOT', '初始共同正文', 'hidden-value']:
            self.assertNotIn(hidden, text)

    def test_embedded_query_outputs_bind_exact_draft_and_frozen_result(self):
        self.prepare_query()
        definition = self.add_definition({'where': 'status == "open"', 'select': [self.column()]})
        def grant(users):
            for name in ['Shared', 'Other']:
                users['alice']['grants'][name].update(
                    fields={'tasks/status': {'read': True}},
                    office={'export': True, 'download': True},
                    outputs={'pdf': {'export': True, 'download': True}})
        self.policy(grant)
        self.grant_query(actor='alice', definitions=True)
        doc = self.open('Other')
        exact = self.basis(doc, doc['source']+'\nExact query draft', 4)
        basis = {k: v for k, v in exact.items() if k != 'baseId'}
        found = self.call('query/embedded/discover', basis)
        occurrence = next(o for o in found['occurrences'] if o['key'] == definition['definition']['key'])
        query = self.call('query/embedded/run', {**basis, 'occurrenceId': occurrence['id']})
        self.assertEqual([r['entityKey'] for r in query['result']['rows']], ['Shared'])
        selected = [{'occurrenceId': occurrence['id'], 'queryId': query['result']['queryId']}]
        word_req = self.request('docx', resource='Other', embeddedQueries=selected, **exact)
        word = self.prepare_output(word_req)
        pdf = self.prepare_output(self.request('pdf', resource='Other', embeddedQueries=selected, **exact))
        shared = self.open()
        self.save(shared, shared['source'].replace('status: "open"', 'status: "cancelled"'))
        text = ''.join(ET.fromstring(self.parts(self.deliver(word))['word/document.xml']).itertext())
        self.assertIn('Exact query draft', text)
        self.assertIn('open', text)
        self.assertNotIn('cancelled', text)
        self.assertTrue(self.deliver(pdf).startswith(b'%PDF-'))
        self.basis(doc, exact['source'], 5)
        self.prepare_output({**word_req, 'requestId': uid()}, status=409)


if __name__ == '__main__':
    suite = unittest.TestSuite(HostedCitationsOutputs(name) for name in
        sorted(vars(HostedCitationsOutputs)) if name.startswith('test_'))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
