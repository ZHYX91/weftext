"""Real hosted resource delivery, media review and explicit-input consumers.

Only owned disposable fixtures are created. The local host provisions admitted
resources; all tested consumer requests cross the hosted HTTP boundary.
"""
import base64
import copy
import hashlib
import io
import json
from pathlib import Path
import sys
import time
import unittest
import wave
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor

from test_hosted import uid
from test_hosted_query import HostedQuery
from test_hosted_review import HostedReview
from test_hosted_office import HostedOffice
from test_hosted_scopes import HostedScopes
from test_hosted_library import visible_html_text
from test_local_features import tiny_png


class HostedMedia(HostedQuery):
    prepare_review = HostedReview.prepare_review
    prepare_annotation = HostedReview.prepare_annotation
    commit_annotation = HostedReview.commit_annotation
    commit_local = HostedScopes.commit_local
    operation_local = HostedScopes.operation_local

    def prepare_media(self, query=False):
        if query:
            self.prepare_query()
        self.prepare_review()
        wav_bytes = io.BytesIO()
        with wave.open(wav_bytes, 'wb') as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(8000)
            wav.writeframes(b'\0\0' * 8000)
        self.payloads = {
            'image': ('region.png', tiny_png()),
            'audio': ('interval.wav', wav_bytes.getvalue()),
            'video': ('video.mp4', (Path(__file__).resolve().parents[1] /
                       'crates/weftext-app/tests/fixtures/media-blue.mp4').read_bytes()),
            'aux': ('supplement.typ', b'AUX_NATIVE_MEDIA_MARKER'),
            'binary': ('owned.bin', bytes(range(256)) * 800),
            'empty': ('empty.bin', b''),
        }

        def provision(api):
            doc = api('/api/open', {'root': str(self.root),
                                   'path': str(self.root/'Shared/Shared.typ')})
            paged = api('/api/paged', {'sessionId': doc['sessionId'],
                        'source': 'PDF page one\n#pagebreak()\nPDF page two\n', 'serial': 1})
            self.assertTrue(paged['ok'], paged)
            self.payloads['pdf'] = ('pages.pdf', base64.b64decode(paged['pdfBase64']))
            for name, payload in self.payloads.values():
                plan = api('/api/resources/prepare-import', {'sessionId': doc['sessionId'],
                    'name': name, 'bytes': base64.b64encode(payload).decode(), 'operationId': uid()})
                api('/api/operations/commit', {'operationId': plan['operationId'], 'planId': plan['planId']})
            records = api('/api/resources/list', {'sessionId': doc['sessionId']})['resources']
            by_name = {r['name']: r for r in records}
            self.assets = {asset: by_name[name] for asset, (name, _) in self.payloads.items()}
            self.config['resources'] = {}
            for asset, descriptor in self.assets.items():
                workspace = descriptor['resource'].split('://', 1)[1].split('/')[0]
                self.config['resources'][asset] = {
                    'owner': 'Shared', 'expectedOwner': {'workspaceId': workspace,
                        'nodeId': self.docs['Shared']['nodeId']},
                    'path': 'Shared/' + self.payloads[asset][0], 'title': self.payloads[asset][0],
                    'reference': descriptor['resource']}
        self.local(provision)

        def grant(users):
            users['alice']['resources'] = {a: {'read': True, 'download': True} for a in self.assets}
            # Annotation and resource authority work without owner source access.
            users['bob']['grants']['Shared'] = {'annotations': {'read': True, 'write': True, 'target': True}}
            users['bob']['resources'] = {a: {'read': True} for a in self.assets}
        self.policy(grant)

    def descriptor(self, asset='image', actor='alice'):
        return next(r for r in self.call('media', actor=actor)['resources'] if r['asset'] == asset)

    def media_review(self, asset='image', actor='alice'):
        return self.call('review/read', {'resource': 'Shared', 'asset': asset}, actor)

    def whole(self, asset='image'):
        d = self.descriptor(asset)
        return {'kind': 'wholeResource', 'asset': asset, 'resource': d['resource'], 'revision': d['revision']}

    def original(self, asset='image', actor='alice', download=False):
        d = self.descriptor(asset, actor)
        return self.call('media/download/prepare' if download else 'media/read',
                         {'asset': asset, 'revision': d['revision']}, actor)

    def deliver_media(self, ticket, actor='alice', download=False):
        output = bytearray()
        while True:
            chunk = self.call('media/download/chunk' if download else 'media/chunk',
                             {'ticket': ticket['ticket'], 'offset': len(output)}, actor)
            block = base64.b64decode(chunk['bytesBase64'])
            self.assertLessEqual(len(block), 65536)
            self.assertEqual(chunk['offset'], len(output))
            self.assertEqual(chunk['length'], len(block))
            self.assertEqual(chunk['total'], ticket['bytes'])
            output.extend(block)
            if chunk['done']:
                break
        self.assertEqual(hashlib.sha256(output).hexdigest(), ticket['sha256'])
        return bytes(output)

    def hold_leaf(self, asset):
        leaf = self.root/self.config['resources'][asset]['path']
        held = leaf.with_name('OwnedHeld-' + leaf.name)
        self.assertTrue(leaf.resolve().is_relative_to(self.root.resolve()))
        self.assertFalse(held.exists())
        leaf.rename(held)
        return leaf, held

    def test_media_default_closed_exact_delivery_and_hidden_siblings(self):
        self.prepare_media()
        self.assertEqual(self.call('media', actor='reader')['resources'], [])
        known = self.call('media/read', {'asset': 'image', 'revision': 'hidden'}, 'reader', 404)
        self.assertEqual(known, self.call('media/read', {'asset': 'invented', 'revision': 'hidden'}, 'reader', 404))
        self.call('open', {'resource': 'Shared'}, 'bob', 404)
        ticket = self.original(actor='bob')
        self.assertEqual(self.deliver_media(ticket, 'bob'), self.payloads['image'][1])
        self.assertNotIn(str(self.root), json.dumps(ticket))
        # A broken ungranted sibling cannot participate in selected resource reading.
        self.policy(lambda users: users['bob']['resources'].pop('video'))
        self.hold_leaf('video')
        self.assertEqual(self.deliver_media(self.original(actor='bob'), 'bob'), self.payloads['image'][1])
        catalog = self.call('media', actor='bob')
        self.assertEqual(catalog['coverage'], 'complete')
        self.assertNotIn('video.mp4', json.dumps(catalog))

    def test_media_copy_rights_ticket_purpose_actor_and_revocation(self):
        self.prepare_media()
        view = self.original('binary', 'bob')
        self.assertEqual(self.deliver_media(view, 'bob'), self.payloads['binary'][1])
        self.call('media/download/prepare', {'asset': 'binary', 'revision': view['revision']}, 'bob', 404)
        self.call('media/download/chunk', {'ticket': view['ticket'], 'offset': 0}, 'bob', 410)
        self.call('media/chunk', {'ticket': view['ticket'], 'offset': 0}, 'alice', 410)
        copy_ticket = self.original('binary', download=True)
        self.assertEqual(self.deliver_media(copy_ticket, download=True), self.payloads['binary'][1])
        self.assertEqual(self.deliver_media(self.original('empty', download=True), download=True), b'')
        self.call('media/chunk', {'ticket': copy_ticket['ticket'], 'offset': 0}, status=410)
        self.policy(lambda users: users['bob']['resources']['binary'].update(read=False))
        self.call('media/chunk', {'ticket': view['ticket'], 'offset': 65536}, 'bob', 410)
        self.policy(lambda users: users['bob']['resources']['binary'].update(read=True))
        self.call('media/chunk', {'ticket': view['ticket'], 'offset': 0}, 'bob', 410)
        self.assertEqual(self.deliver_media(self.original('binary', 'bob'), 'bob'), self.payloads['binary'][1])

    def test_media_native_facts_valid_regions_and_forged_geometry(self):
        self.prepare_media()
        image = self.descriptor('image'); audio = self.descriptor('audio')
        pdf = self.descriptor('pdf'); video = self.descriptor('video')
        self.assertEqual(image['metadata'], {'kind': 'image', 'width': 2, 'height': 2})
        self.assertEqual(audio['metadata']['durationTick'], 8000)
        self.assertEqual(len(pdf['metadata']['pages']), 2)
        self.assertEqual(video['metadata']['kind'], 'video')
        targets = [
            {**self.whole('image'), 'kind': 'imageRegion', 'width': 2, 'height': 2,
                'x': 0, 'y': 0, 'rectWidth': 1, 'rectHeight': 2},
            {**self.whole('audio'), 'kind': 'mediaInterval', 'timebase': 8000,
                'timebaseDenominator': 1, 'durationTick': 8000, 'startTick': 1000, 'endTick': 2000},
            {**self.whole('pdf'), 'kind': 'pdfRegion', 'pageIndex': 1,
                **pdf['metadata']['pages'][1], 'x': 0, 'y': 0, 'rectWidth': 10, 'rectHeight': 10},
            {**self.whole('video'), 'kind': 'mediaInterval',
                'timebase': video['metadata']['timebase']['numerator'],
                'timebaseDenominator': video['metadata']['timebase']['denominator'],
                'durationTick': video['metadata']['durationTick'], 'startTick': 0,
                'endTick': max(1, video['metadata']['durationTick']//2)},
            self.whole('binary'),
        ]
        original = (self.root/'Shared/Shared.typ').read_bytes()
        for target in targets:
            p, _ = self.prepare_annotation(self.media_review(target['asset'], 'bob'),
                {'kind': 'annotationCreate', 'body': '真实资源 ' + target['asset'], 'target': target}, 'bob')
            self.assertEqual(self.commit_annotation(p, actor='bob')['status'], 'committed')
        sidecar = self.root/'Shared/weftext.annotations.json'
        before = sidecar.read_bytes()
        for target, changes, status in [(targets[0], {'width': 999}, 409),
                                         (targets[1], {'timebase': 44100}, 409),
                                         (targets[2], {'pageIndex': 99}, 400),
                                         (targets[0], {'resource': audio['resource']}, 400)]:
            profile = self.media_review(target['asset'], 'bob')
            self.call('review/prepare', {'resource': 'Shared', 'basis': profile['basis'],
                'operationId': uid(), 'intent': {'kind': 'annotationCreate', 'body': '伪造',
                    'target': {**target, **changes}}}, 'bob', status)
        self.assertEqual(sidecar.read_bytes(), before)
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), original)
        binary = self.descriptor('binary')
        self.assertFalse(binary['annotationReady']); self.assertTrue(binary['wholeAnnotationReady'])
        self.policy(lambda users: users['bob'].update(resources={}))
        discussion = self.call('review/read', {'resource': 'Shared'}, 'bob')
        self.assertEqual(len(discussion['annotations']), len(targets))
        self.assertTrue(all('resourceLocation' not in a for a in discussion['annotations']))
        reply, _ = self.prepare_annotation(discussion, {'kind': 'annotationReply',
            'annotationId': discussion['annotations'][0]['id'], 'body': '独立讨论无需重读媒体'}, 'bob')
        self.assertEqual(self.commit_annotation(reply, actor='bob')['status'], 'committed')

    def test_media_original_plan_restart_idempotency_and_source_draft(self):
        self.prepare_media()
        source = self.open()
        self.call('draft', {'sessionId': source['sessionId'], 'source': source['source']+'\nPRIVATE_DRAFT'})
        plan, request = self.prepare_annotation(self.media_review('image', 'bob'),
            {'kind': 'annotationCreate', 'body': '原操作唯一讨论', 'target': self.whole()}, 'bob')
        self.server.close(); self.start()
        recovered = self.call('review/prepare', request, 'bob')
        self.assertEqual(recovered['planId'], plan['planId'])
        self.assertEqual(self.commit_annotation(recovered, actor='bob')['status'], 'committed')
        self.assertEqual(self.commit_annotation(plan, actor='bob')['status'], 'committed')
        self.assertEqual(self.call('review/prepare', request, 'bob')['status'], 'committed')
        self.call('review/prepare', {**request, 'intent': {**request['intent'], 'body': '不同意图'}}, 'bob', 409)
        self.assertEqual(self.call('review/operation', {'resource': 'Shared',
            'operationId': plan['operationId']})['status'], 'notRegistered')
        self.assertEqual(len(self.media_review('image', 'bob')['annotations']), 1)
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), source['source'])
        self.assertIn('PRIVATE_DRAFT', self.open()['recoveredDraft'])

    def test_media_changed_original_blocks_tickets_and_persisted_plan(self):
        self.prepare_media()
        ticket = self.original()
        plan, _ = self.prepare_annotation(self.media_review(), {'kind': 'annotationCreate',
            'body': '旧依据不得提交', 'target': self.whole()})
        leaf = self.root/self.config['resources']['image']['path']
        leaf.write_bytes(self.payloads['image'][1] + b'changed')
        self.call('media/chunk', {'ticket': ticket['ticket'], 'offset': 0}, status=410)
        self.call('media/read', {'asset': 'image', 'revision': ticket['revision']}, status=409)
        self.server.close(); self.start()
        self.commit_annotation(plan, status=409)
        self.assertFalse((self.root/'Shared/weftext.annotations.json').exists())
        self.assertEqual(leaf.read_bytes(), self.payloads['image'][1] + b'changed')

    def test_media_same_bytes_replacement_is_not_original_identity(self):
        self.prepare_media()
        ticket = self.original()
        leaf, held = self.hold_leaf('image')
        leaf.write_bytes(held.read_bytes())
        self.call('media/read', {'asset': 'image', 'revision': ticket['revision']}, status=409)
        self.call('media/chunk', {'ticket': ticket['ticket'], 'offset': 0}, status=410)
        self.assertEqual(self.descriptor()['status'], 'unavailable')
        self.assertEqual(self.call('media')['coverage'], 'partial')
        self.assertEqual(held.read_bytes(), self.payloads['image'][1])

    def test_media_stable_refs_and_real_pdf_fragment_validation(self):
        self.prepare_media()
        pdf = self.descriptor('pdf')
        request = {'reference': pdf['resource']+'#page=2', 'revision': pdf['revision']}
        resolved = self.call('media/resolve', request, 'bob')
        self.assertEqual(resolved['viewerTarget']['resourceFragment'], {'status': 'found', 'pageIndex': 1})
        self.assertEqual(self.call('media/resolve', {**request, 'reference': pdf['resource']+'#page=99'})
                         ['viewerTarget']['resourceFragment']['status'], 'missing')
        self.assertEqual(self.call('media/resolve', {'reference': self.assets['image']['resource']+'#page=2'})
                         ['viewerTarget']['resourceFragment']['status'], 'unavailable')
        self.call('media/resolve', request, 'reader', 404)
        self.call('media/resolve', {**request, 'revision': 'not-current'}, status=409)
        self.assertNotIn(str(self.root), json.dumps(resolved))

    def test_media_explicit_inputs_native_query_editor_and_frozen_office(self):
        self.prepare_media(query=True)
        doc = self.open()
        self.save(doc, doc['source']+'\n#include "supplement.typ"\n')
        self.server.close(); self.config['files']['Shared']['inputs'] = ['aux']; self.start()
        self.grant_query(resources=('Shared',), products=('body',))
        static = self.query({'select': [self.column()]}, resources=['Shared'])
        self.assertEqual(len(static['result']['rows']), 1)
        self.query({'select': [self.column('body')]}, fields=[], products=['body'], resources=['Shared'], status=404)
        self.policy(lambda users: users['fields'].update(resources={'aux': {'read': True}}))
        native = self.query({'select': [self.column('body')]}, fields=[], products=['body'], resources=['Shared'])
        self.assertIn('AUX_NATIVE_MEDIA_MARKER', json.dumps(native))
        doc = self.open()
        rendered = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('AUX_NATIVE_MEDIA_MARKER', visible_html_text(rendered['html']))
        HostedOffice.prepare_office(self)
        request = {'outputLocale': 'zh-CN', 'requestId': uid(), 'resource': 'Shared', 'source': doc['source'], 'format': 'docx',
                   'reviewMode': 'none', 'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}}
        output = self.call('office/export/prepare', request)
        leaf = self.root/self.config['resources']['aux']['path']
        leaf.write_text('LATER_AUX_CONTENT', 'utf-8')
        parts = HostedOffice.parts(HostedOffice.deliver(self, output))
        text = ''.join(ET.fromstring(parts['word/document.xml']).itertext())
        self.assertIn('AUX_NATIVE_MEDIA_MARKER', text); self.assertNotIn('LATER_AUX_CONTENT', text)
        self.policy(lambda users: users['alice']['resources']['aux'].update(download=False))
        self.call('office/export/chunk', {'requestId': output['requestId'], 'offset': 0}, status=409)

    def test_media_render_does_not_deliver_after_original_input_changes(self):
        self.prepare_media()
        self.server.close(); self.config['files']['Shared']['inputs'] = ['aux']; self.start()
        doc = self.open()
        source = doc['source']+'\n#include "supplement.typ"\n#for i in range(12000) [slow paragraph #parbreak()]'
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.server.call, '/api/hosted/preview',
                {'sessionId': doc['sessionId'], 'source': source}, self.tokens['alice'])
            time.sleep(0.2)
            (self.root/self.config['resources']['aux']['path']).write_text('CHANGED_DURING_COMPILE', 'utf-8')
            status, response, _ = pending.result(timeout=40)
        self.assertEqual(status, 409, str(response)[:1000])
        self.assertNotIn('html', response)

    def test_media_proofread_original_dependencies_survive_restart(self):
        self.prepare_media()
        doc = self.open()
        self.save(doc, doc['source']+'\n错别字\n#include "supplement.typ"\n')
        self.server.close(); self.config['files']['Shared']['inputs'] = ['aux']; self.start()
        options = dict(characters=True, typos=True, pairs=True, sensitive=True,
            allowlist=[], selection=None, rules=[dict(id='media.plan', version='1',
            category='typo', text='错别字', message='请核对', replacement='正确字', scope={'universal': True})])
        profile = self.call('review/read', {'resource': 'Shared'})
        scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        findings = [f['id'] for f in scan['findings'] if f['sourceRange']]
        self.assertTrue(findings)
        plan, request = self.prepare_annotation(profile, {'annotationLocale': 'zh-CN', 'kind': 'annotationProofread',
            'options': options, 'findings': findings, 'scanId': scan['scanId'], 'ignore': False})
        source = (self.root/'Shared/Shared.typ').read_bytes()
        self.server.close(); self.start()
        self.assertEqual(self.call('review/operation', {'resource': 'Shared',
            'operationId': request['operationId']})['planId'], plan['planId'])
        (self.root/self.config['resources']['aux']['path']).write_text('CHANGED_INPUT', 'utf-8')
        self.commit_annotation(plan, status=409)
        self.assertFalse((self.root/'Shared/weftext.annotations.json').exists())
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), source)

    def test_media_trash_inputs_partial_missing_and_no_restore_fallback(self):
        self.prepare_media(query=True)
        doc = self.open()
        self.save(doc, doc['source']+'\n#include "supplement.typ"\n')
        self.grant_query(resources=('Shared',), products=('body',))
        def readonly(users):
            users['fields'].update(resources={'aux': {'read': True}}, queryScopes={'trash': True})
            for user in users.values():
                grant = user.get('grants', {}).get('Shared')
                if grant is not None:
                    grant.update(write=False)
                    grant.pop('annotations', None)
                    grant.pop('office', None)
                    for field in grant.get('fields', {}).values():
                        field['write'] = False
        self.policy(readonly)
        self.config['users'] = copy.deepcopy(self.call('admin/policy', actor='admin')['users'])
        def trash(api):
            receipt = self.operation_local(api, {'kind': 'trashNode', 'source': str(self.root/'Shared')})
            target = Path(receipt['path'])
            self.trash_operation = receipt['operationId']
            self.config['files']['Shared'].update(path=(target/'Shared.typ').relative_to(self.root).as_posix(),
                editable=False, importable=False, lifecycle='trash', inputs=['aux'],
                expectedNode=self.config['resources']['aux']['expectedOwner'])
            for asset, resource in self.config['resources'].items():
                resource.update(path=(target/self.payloads[asset][0]).relative_to(self.root).as_posix(), lifecycle='trash')
        self.local(trash)
        plan = {'scope': {'lifecycle': 'trash', 'descendants': True},
                'select': [self.column('body')]}
        result = self.query(plan, fields=[], products=['body'], resources=['Shared'])
        self.assertEqual(result['result']['coverage'], 'complete', result)
        self.assertIn('AUX_NATIVE_MEDIA_MARKER', json.dumps(result))
        self.assertTrue(result['result']['rows'][0]['readOnly'])
        self.assertEqual(self.deliver_media(self.original()), self.payloads['image'][1])
        self.call('review/read', {'resource': 'Shared', 'asset': 'image'}, status=404)
        leaf, held = self.hold_leaf('aux')
        partial = self.query(plan, fields=[], products=['body'], resources=['Shared'])
        self.assertEqual(partial['result']['coverage'], 'partial', partial)
        rows = partial['result']['rows']
        self.assertEqual(len(rows), 1, partial)
        self.assertEqual(rows[0]['cells']['status']['state'], 'unavailable')
        # Restoring the original object is owned fixture maintenance, not admission.
        held.rename(leaf)
        def restore(api):
            self.operation_local(api, {'kind': 'restoreNode', 'trashOperationId': self.trash_operation,
                                       'parent': str(self.root), 'name': 'Restored'})
        self.local(restore)
        self.call('media/resolve', {'reference': self.assets['image']['resource']}, status=409)
        self.query(plan, fields=[], products=['body'], resources=['Shared'], status=409)
        self.assertTrue((self.root/'Restored/region.png').exists())


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    suite = unittest.TestSuite(HostedMedia(name) for name in sorted(HostedMedia.__dict__)
                              if name.startswith('test_media_') and (not selected or name in selected))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
