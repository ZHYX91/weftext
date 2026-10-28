"""Actual Office conversions with scoped hosted authority and durable IDs."""
import uuid
import base64
import hashlib
import io
import json
import sys
import time
import unittest
import zipfile
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor
from owned_worker_observer import OwnedWorkerObservation, event_time_ns, timed_call
from test_hosted import uid
from test_hosted_review import HostedReview


class HostedOffice(HostedReview):
    def prepare_office(self):
        self.policy(lambda users: [grant.update(office={'export': True, 'download': True})
                                  for grant in users['alice']['grants'].values()])

    def output_request(self, **extra):
        return {'outputLocale': 'zh-CN', 'requestId': uid(), 'resource': 'Shared', 'source': self.open()['source'], 'format': 'docx',
                    'reviewMode': 'none', 'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}, **extra}

    def output(self, request=None, actor='alice'):
        request = request or self.output_request()
        return self.call('office/export/prepare', request, actor), request

    def deliver(self, result, actor='alice'):
        request = {'requestId': result['requestId']}
        self.call('office/export/confirm', {**request, 'digest': result['digest'], 'acceptLosses': True}, actor)
        output = bytearray()
        while True:
            chunk = self.call('office/export/chunk', {**request, 'offset': len(output)}, actor)
            block = base64.b64decode(chunk['bytesBase64'])
            self.assertEqual(chunk['offset'], len(output)); self.assertEqual(chunk['length'], len(block))
            self.assertEqual(chunk['total'], result['length']); self.assertEqual(chunk['sha256'], result['digest'])
            output.extend(block)
            if chunk['done']: break
        self.assertEqual(hashlib.sha256(output).hexdigest(), result['digest'])
        (self.folder / (result['requestId'] + '.' + result['filename'].rsplit('.', 1)[-1])).write_bytes(output)
        return bytes(output)

    @staticmethod
    def parts(data):
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            return {name: archive.read(name).decode('utf-8') for name in archive.namelist() if name.endswith('.xml')}

    def test_office_frozen_draft_independent_output_rights_and_restart_fence(self):
        self.prepare_office()
        request = self.output_request(); request['source'] += '\nCAPTURED_UNSAVED_BODY'
        self.call('office/export/prepare', request, 'bob', 404)
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'bob')['status'], 'notRegistered')
        result, _ = self.output(request)
        self.assertEqual(result['status'], 'prepared')
        self.assertNotIn('CAPTURED_UNSAVED_BODY', (self.root/'Shared/Shared.typ').read_text('utf-8'))
        self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, status=409)
        self.call('office/export/confirm', {'requestId': request['requestId'], 'digest': '0'*64, 'acceptLosses': True}, status=400)
        doc = self.open(); self.save(doc, doc['source']+'\nNEWER_SAVED_BODY')
        artifact = self.deliver(result); parts = self.parts(artifact)
        word_text = ''.join(ET.fromstring(parts['word/document.xml']).itertext())
        self.assertIn('CAPTURED_UNSAVED_BODY', word_text)
        self.assertNotIn('NEWER_SAVED_BODY', word_text)
        self.assertEqual(self.output(request)[0]['digest'], result['digest'])
        self.call('office/export/prepare', {**request, 'source': request['source']+'different'}, status=409)
        self.policy(lambda users: users['reader']['grants']['Shared'].update(office={'export': True}))
        reader_request = {**request, 'requestId': uid()}
        reader = self.output(reader_request, 'reader')[0]
        self.call('office/export/confirm', {'requestId': reader['requestId'], 'digest': reader['digest'], 'acceptLosses': True}, 'reader', 404)
        # Source download permission is independent from Office download.
        self.policy(lambda users: users['alice']['grants']['Shared'].update(download=False))
        current = self.output()[0]; self.deliver(current)
        self.call('download/prepare', {'resource': 'Shared'}, status=404)
        self.server.close(); self.start()
        self.assertEqual(self.output(request)[0]['status'], 'expired')
        self.assertEqual(self.call('office/export/status', {'requestId': current['requestId']})['status'], 'expired')
        self.call('office/export/confirm', {'requestId': current['requestId'], 'digest': current['digest'], 'acceptLosses': True})

    def test_office_reviews_exact_saved_source_pending_changes_and_personal_draft(self):
        self.prepare_review(); self.prepare_office()
        annotation, _, _ = self.create_annotation('suggestion', '修订替换内容😀')
        reply, _ = self.prepare_annotation(self.review(), dict(kind='annotationReply', annotationId=annotation['id'], body='REPLY_PRESERVED'))
        self.commit_annotation(reply)
        source = self.open()['source']; original = (self.root/'Shared/weftext.annotations.json').read_bytes()
        doc = self.open(); self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': doc['baseId'], 'inputRevision': 1, 'sessionId': doc['sessionId'], 'source': source+'\nPERSONAL_INPUT'})
        request = {**self.output_request(), 'reviewMode': 'changes'}
        self.call('office/export/prepare', {**request, 'source': source+'dirty'}, status=400)
        result, _ = self.output(request); artifact = self.deliver(result); parts = self.parts(artifact)
        self.assertIn('保留这条讨论', parts['word/comments.xml']); self.assertIn('REPLY_PRESERVED', parts['word/comments.xml'])
        self.assertIn('w:ins', parts['word/document.xml']); self.assertIn('w:del', parts['word/document.xml'])
        self.assertIn('修订替换内容', parts['word/document.xml'])
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), source)
        self.assertEqual((self.root/'Shared/weftext.annotations.json').read_bytes(), original)
        self.assertIn('PERSONAL_INPUT', self.open()['recoveredDraft'])
        self.policy(lambda users: users['alice']['grants']['Shared']['annotations'].update(target=False, accept=False, proofread=False))
        self.call('office/export/prepare', {**request, 'requestId': uid()}, status=404)
        self.assertEqual(self.call('office/export/status', {'requestId': result['requestId']})['status'], 'expired')
        self.call('office/export/chunk', {'requestId': result['requestId'], 'offset': 0}, status=409)
        self.output({**request, 'requestId': uid(), 'reviewMode': 'none'})

    def test_office_field_only_xlsx_uses_frozen_ordered_result_and_restricted_schema(self):
        self.prepare_fields()
        self.policy(lambda users: [g.update(office={'export': True, 'download': True}) for g in users['fields']['grants'].values()])
        query = self.call('query', {'fields': ['tasks/status'], 'plan': {'select': [{'name': '状态', 'expr': 'status'}, {'name': '序号', 'expr': '1'}]}}, 'fields')
        self.assertEqual(query['columns'], ['状态', '序号'])
        request = {'outputLocale': 'zh-CN', 'requestId': uid(), 'format': 'xlsx', 'reviewMode': 'none', 'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}, 'queryId': query['queryId']}
        self.call('fields/write', self.proposal(self.field(), 'done'), 'fields')
        result, _ = self.output(request, 'fields'); workbook = self.deliver(result, 'fields'); parts = self.parts(workbook)
        xml = '\n'.join(parts.values())
        for hidden in ['初始共同正文', 'hidden-value', 'HIDDEN_PAYLOAD_987', 'tasks/due', str(self.root), 'PRIVATE_INPUT']:
            self.assertNotIn(hidden, xml)
        # The authorized enum definition legitimately contains all options;
        # only the actual exported data rows must retain the original value.
        data_xml = parts['xl/worksheets/weftext1.xml']
        self.assertIn('open', data_xml); self.assertNotIn('done', data_xml)
        self.assertEqual(result['preview']['query']['columns'], query['columns'])
        self.assertEqual(result['preview']['query']['rowCount'], 2)
        template_bytes = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(workbook)) as original, zipfile.ZipFile(template_bytes, 'w', zipfile.ZIP_DEFLATED) as template:
            for entry in original.infolist():
                data = original.read(entry.filename)
                if entry.filename == 'xl/workbook.xml':
                    names = '<definedNames><definedName name="FrozenRows">\'资料\'!$A$1:$B$3</definedName><definedName name="Caption">\'资料\'!$C$1</definedName></definedNames>'
                    data = data.decode('utf-8').replace('</workbook>', names + '</workbook>').encode('utf-8')
                template.writestr(entry.filename, data)
        encoded = base64.b64encode(template_bytes.getvalue()).decode()
        slots = self.call('office/template', dict(format='xlsx', bytesBase64=encoded), 'fields')['slots']
        self.assertTrue({'FrozenRows', 'Caption'}.issubset({s['name'] for s in slots}))
        templated, _ = self.output({**request, 'requestId': uid(), 'template': {'name': 'owned.xlsx', 'bytesBase64': encoded},
                                   'bindings': {'FrozenRows': {'kind': 'query'}, 'Caption': {'kind': 'text', 'text': '明确字面文本😀'}}}, 'fields')
        filled = self.parts(self.deliver(templated, 'fields'))['xl/worksheets/weftext1.xml']
        self.assertIn('明确字面文本😀', filled); self.assertIn('open', filled); self.assertNotIn('done', filled)
        self.call('office/export/prepare', {**request, 'requestId': uid(), 'source': 'hidden source'}, 'fields', 400)
        self.call('office/export/prepare', {**request, 'requestId': uid()}, 'alice', 410)
        self.policy(lambda users: users['fields']['grants']['Shared']['office'].update(download=False))
        self.assertEqual(self.call('office/export/status', {'requestId': result['requestId']}, 'fields')['status'], 'expired')
        self.call('office/export/chunk', {'requestId': result['requestId'], 'offset': 0}, 'fields', 409)
        empty = self.call('query', {'fields': ['tasks/status'], 'resources': [], 'plan': {}}, 'fields')
        empty_result, _ = self.output({**request, 'requestId': uid(), 'queryId': empty['queryId']}, 'fields')
        self.assertEqual(empty_result['preview']['query']['rowCount'], 0)
        # Empty results still consume an authorized field definition and output grant.
        self.policy(lambda users: [g['office'].update(export=False) for g in users['fields']['grants'].values()])
        empty = self.call('query', {'fields': ['tasks/status'], 'resources': [], 'plan': {}}, 'fields')
        self.call('office/export/prepare', {**request, 'requestId': uid(), 'queryId': empty['queryId']}, 'fields', 404)

    def test_office_inputs_requalification_and_failed_original_cannot_regenerate(self):
        self.prepare_office()
        self.server.close(); self.config['files']['Shared']['inputs'] = ['Secret']; self.start()
        self.policy(lambda users: users['bob']['grants']['Shared'].update(office={'export': True, 'download': True}))
        request = self.output_request()
        self.call('office/export/prepare', request, 'bob', 404)
        self.policy(lambda users: users['alice']['grants']['Secret']['office'].update(export=False))
        self.call('office/export/prepare', request, status=404)
        self.policy(lambda users: users['alice']['grants']['Secret']['office'].update(export=True))
        bad = {**request, 'requestId': uid(), 'source': '#panic("invalid capture")'}
        self.call('office/export/prepare', bad, status=409)
        self.assertEqual(self.call('office/export/status', {'requestId': bad['requestId']})['status'], 'failed')
        self.assertEqual(self.output(bad)[0]['status'], 'failed')
        self.call('office/export/prepare', {**bad, 'source': request['source']}, status=409)
        # Construct 20,000 native paragraph contents with bounded projection.
        # Fully expanding them would test capacity instead of late authorization.
        workload = ('\n#let inflight-paragraphs = range(20000).map(i => [paragraph#parbreak()])\n'
                    '#text(str(inflight-paragraphs.len()))\nparagraph')
        slow = {**request, 'requestId': uid(), 'source': request['source']+workload}
        deadline = time.monotonic() + 45
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted')
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/office/export/prepare', slow, self.tokens['alice'])
                worker = observation.wait(pending, deadline=deadline)
                observation.assert_inflight(pending)
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before change')
                self.policy(lambda users: users['alice']['grants']['Secret']['office'].update(export=False))
                changed = event_time_ns()
                (code, response, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        trace = {'submittedMonotonicNs': submitted, 'worker': worker,
                 'changeCompletedMonotonicNs': changed, 'requestCompletedMonotonicNs': completed,
                 'status': code, 'errorCode': response.get('error', {}).get('code'),
                 'explanationKey': response.get('error', {}).get('explanation', {}).get('key')}
        with (self.folder/'office-input-revocation-inflight.json').open('x', encoding='utf-8') as evidence:
            evidence.write(json.dumps(trace, ensure_ascii=False, indent=2))
        self.assertLessEqual(submitted, worker['observedMonotonicNs'])
        self.assertLess(worker['observedMonotonicNs'], changed)
        self.assertLess(changed, completed, 'Concurrent change must finish before request delivery')
        self.assertIn(code, [404, 409, 422]); self.assertNotIn('digest', response)
        self.assertEqual(code, 409, response)
        self.assertEqual(response['error']['code'], 'officeUnavailable')
        self.assertEqual(response['error']['explanation']['key'],
                         'error.api.hostedOffice.outputBasisChanged')

    def test_office_template_slots_exact_upload_and_route_budget(self):
        self.prepare_office()
        artifact = self.deliver(self.output()[0])
        target = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(artifact)) as original, zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as template:
            for entry in original.infolist():
                data = original.read(entry.filename)
                if entry.filename == 'word/document.xml':
                    xml = data.decode('utf-8')
                    start = xml.index('<w:body>') + len('<w:body>'); end = xml.index('<w:sectPr>')
                    controls = '<w:sdt><w:sdtPr><w:tag w:val="bodySlot"/></w:sdtPr><w:sdtContent><w:p><w:r><w:t>OLD_BODY</w:t></w:r></w:p></w:sdtContent></w:sdt>'
                    controls += '<w:sdt><w:sdtPr><w:tag w:val="noteSlot"/></w:sdtPr><w:sdtContent><w:p><w:r><w:t>OLD_NOTE</w:t></w:r></w:p></w:sdtContent></w:sdt>'
                    data = (xml[:start] + controls + xml[end:]).encode('utf-8')
                template.writestr(entry.filename, data)
            # A real valid ZIP over the ordinary 3 MiB body budget: the Office
            # route's scoped 24 MiB exception must not be overridden globally.
            template.writestr('owned-unused.bin', b'x' * (3 * 1024 * 1024), compress_type=zipfile.ZIP_STORED)
        encoded = base64.b64encode(target.getvalue()).decode()
        inspection = self.call('office/template', dict(format='docx', bytesBase64=encoded))
        self.assertEqual({s['name'] for s in inspection['slots']}, {'bodySlot', 'noteSlot'})
        request = {**self.output_request(), 'template': {'name': 'owned.docx', 'bytesBase64': encoded},
                   'bindings': {'bodySlot': {'kind': 'body'}, 'noteSlot': {'kind': 'text', 'text': '模板字面文本😀'}}}
        result, _ = self.output(request)
        if result['losses']:
            self.call('office/export/confirm', dict(requestId=result['requestId'], digest=result['digest'], acceptLosses=False), status=400)
        text = ''.join(ET.fromstring(self.parts(self.deliver(result))['word/document.xml']).itertext())
        self.assertIn('初始共同正文', text); self.assertIn('模板字面文本😀', text)
        self.assertNotIn('OLD_BODY', text); self.assertNotIn('OLD_NOTE', text)
        self.call('office/export/prepare', {**request, 'requestId': uid(), 'bindings': {'bodySlot': {'kind': 'field', 'text': 'tasks/status'}}}, status=400)

    def import_slot(self):
        self.server.close()
        self.config['files']['Imported'] = dict(path='Imported/document/document.typ', title='Imported', editable=True, importable=True)
        self.config['files']['Cancelled'] = dict(path='Cancelled/document/document.typ', title='Cancelled', editable=True, importable=True)
        self.start()
        self.policy(lambda users: users['alice']['grants'].update({r: {'read': True, 'write': True, 'office': {'import': True}, 'annotations': {'read': True, 'target': True}} for r in ['Imported', 'Cancelled']}))

    def test_office_import_original_plan_restart_loss_consent_and_create_only(self):
        self.prepare_review(); self.prepare_office(); self.create_annotation()
        result, _ = self.output({**self.output_request(), 'reviewMode': 'comments'})
        original = self.deliver(result)
        self.import_slot()
        targets = self.call('office/import-targets')['targets']; self.assertEqual(len(targets), 2)
        request = dict(resource='Imported', operationId=uid(), bytesBase64=base64.b64encode(original).decode())
        self.call('office/import/prepare', request, 'bob', 404)
        plan = self.call('office/import/prepare', request)
        self.assertEqual(plan['status'], 'prepared'); self.assertFalse((self.root/'Imported').exists())
        self.assertNotIn(str(self.root), json.dumps(plan))
        self.server.close(); self.start()
        same = self.call('office/import/prepare', request)
        self.assertEqual(same['planId'], plan['planId']); self.assertEqual(same['preview'], plan['preview'])
        commit = dict(resource='Imported', operationId=plan['operationId'], planId=plan['planId'], acceptLosses=True)
        if plan['preview'].get('losses'):
            self.call('office/import/commit', {**commit, 'acceptLosses': False}, status=409)
        self.assertEqual(self.call('office/import/commit', commit)['status'], 'committed')
        self.assertEqual(self.call('office/import/commit', commit)['status'], 'committed')
        self.assertEqual(self.call('office/import/prepare', request)['status'], 'committed')
        originals = list((self.root/'Imported').rglob('*.docx'))
        self.assertEqual(len(originals), 1); self.assertEqual(originals[0].read_bytes(), original)
        self.assertIn('共同正文', self.open('Imported')['source'])
        self.assertTrue((self.root/'Imported/document/weftext.annotations.json').exists())
        self.assertIn('保留这条讨论', json.dumps(self.review('Imported'), ensure_ascii=False))
        self.call('office/import/prepare', {**request, 'operationId': uid()}, status=409)
        self.call('office/export/prepare', {**self.output_request(), 'resource': 'Imported', 'source': self.open('Imported')['source']}, status=404)
        operation = dict(resource='Cancelled', operationId=uid())
        self.assertEqual(self.call('office/import/settle', operation)['status'], 'cancelled')
        self.assertEqual(self.call('office/import/prepare', {**request, **operation})['status'], 'cancelled')
        self.assertFalse((self.root/'Cancelled').exists())


if __name__ == '__main__':
    # Only the new cases here; complete inherited suites run through their entry points.
    suite = unittest.TestSuite(HostedOffice(name) for name in unittest.defaultTestLoader.getTestCaseNames(HostedOffice) if name.startswith('test_office_'))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
