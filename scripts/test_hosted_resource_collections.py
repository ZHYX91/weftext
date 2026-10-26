"""Real hosted resource production and explicitly granted collection inputs.

All libraries/uploads are owned fixtures. Only Core fixture preparation uses
the local host; collection, import and consumer assertions use hosted HTTP.
"""
import base64
import copy
import hashlib
import json
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from owned_worker_observer import (OwnedWorkerObservation, event_time_ns, timed_call,
    apply_inflight_mutation, verify_inflight_change)
from pathlib import Path

from test_hosted import uid
from test_hosted_media import HostedMedia
from test_hosted_office import HostedOffice
from test_hosted_library import visible_html_text
from test_local_features import tiny_png
from test_outputs import CT, P, R, W, package, unpack


def image_docx():
    """Owned OPC fixture with one inline image, matching the native producer."""
    drawing = ('<w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0">'
        '<wp:extent cx="19050" cy="19050"/><wp:docPr id="1" name="Owned image"/>'
        '<wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr>'
        '<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">'
        '<pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="owned.png"/><pic:cNvPicPr/></pic:nvPicPr>'
        '<pic:blipFill><a:blip r:embed="image"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>'
        '<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="19050" cy="19050"/></a:xfrm>'
        '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>'
        '</a:graphicData></a:graphic></wp:inline></w:drawing>')
    return package({
        '[Content_Types].xml': f'<Types xmlns="{CT}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
        '_rels/.rels': f'<Relationships xmlns="{P}"><Relationship Id="officeDocument" Type="{R}/officeDocument" Target="word/document.xml"/></Relationships>',
        'word/document.xml': f'<w:document xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body><w:p><w:r><w:t>Imported collection image</w:t></w:r></w:p><w:p><w:r>{drawing}</w:r></w:p><w:sectPr/></w:body></w:document>',
        'word/_rels/document.xml.rels': f'<Relationships xmlns="{P}"><Relationship Id="image" Type="{R}/image" Target="media/owned.png"/></Relationships>',
        'word/media/owned.png': tiny_png(),
    })


class HostedResourceCollections(HostedMedia):
    def prepare_collections(self, query=False):
        if query:
            self.prepare_query()
        self.prepare_review()
        owner = self.call('navigation/ref', {'sessionId': self.open()['sessionId']})['nodeRef']
        self.server.close()
        self.config['resourceCollections'] = {
            'attachments': {'owner': 'Shared', 'expectedOwner': owner, 'title': 'Shared attachments'}}
        self.start()
        def grant(users):
            users['alice']['resourceCollections'] = {'attachments': {'read': True, 'download': True, 'import': True}}
            users['bob']['grants'] = {'Shared': {'annotations': {'read': True, 'write': True, 'target': True}}}
            users['bob']['resourceCollections'] = {'attachments': {'import': True}}
        self.policy(grant)

    def import_request(self, name='new.png', payload=None):
        return {'collection': 'attachments', 'name': name,
                'bytesBase64': base64.b64encode(tiny_png() if payload is None else payload).decode(),
                'operationId': uid()}

    def import_prepare(self, request=None, actor='alice', status=200):
        request = request or self.import_request()
        return self.call('media/import/prepare', request, actor, status), request

    def import_commit(self, plan, actor='alice', status=200):
        return self.call('media/import/commit', {key: plan[key] for key in
                         ('collection', 'operationId', 'planId')}, actor, status)

    def import_asset(self, name='new.png', payload=None, actor='alice'):
        plan, request = self.import_prepare(self.import_request(name, payload), actor)
        committed = self.import_commit(plan, actor)
        self.assertEqual(committed['status'], 'committed')
        self.assertEqual(committed['resourceRefs'], plan['resourceRefs'])
        return committed['assets'][0], committed, request

    def enable_inputs(self):
        self.server.close()
        self.config['files']['Shared']['inputs'] = ['attachments/*']
        self.start()

    def prepare_import_slot_collection(self):
        self.prepare_review()
        HostedOffice.import_slot(self)
        self.server.close()
        self.config['resourceCollections'] = {'imported-assets': {
            'owner': 'Imported', 'fromImportSlot': True, 'title': 'Imported resources'}}
        self.config['files']['Imported']['inputs'] = ['imported-assets/*']
        self.start()
        def grant(users):
            users['alice']['resourceCollections'] = {'imported-assets': {
                'read': True, 'download': True, 'import': True}}
            users['alice']['grants']['Imported']['office'].update(export=True, download=True)
            users['bob']['grants'] = {}
            users['bob']['resourceCollections'] = {'imported-assets': {'read': True}}
        self.policy(grant)

    def install_image_docx(self):
        original = image_docx()
        request = {'resource': 'Imported', 'operationId': uid(),
                   'bytesBase64': base64.b64encode(original).decode()}
        plan = self.call('office/import/prepare', request)
        self.assertEqual(plan['status'], 'prepared')
        commit = {key: plan[key] for key in ('resource', 'operationId', 'planId')}
        self.assertEqual(self.call('office/import/commit', {**commit, 'acceptLosses': True})['status'], 'committed')
        self.assertEqual((self.root/'Imported/document/original.docx').read_bytes(), original)
        return original

    def test_collection_independent_producer_and_future_member_grants(self):
        self.prepare_collections()
        self.assertEqual(self.call('media', actor='reader')['resources'], [])
        self.assertEqual(self.call('media', actor='bob')['resources'], [])
        self.assertEqual(self.call('documents', actor='bob')['documents'], [])
        before = (self.root/'Shared/Shared.typ').read_bytes()
        plan, request = self.import_prepare(actor='bob')
        self.assertEqual(plan['status'], 'prepared')
        self.assertFalse((self.root/'Shared/new.png').exists())
        self.assertEqual(self.call('media', actor='alice')['resources'], [])
        receipt = self.import_commit(plan, 'bob')
        asset = receipt['assets'][0]
        self.assertTrue(asset.startswith('attachments/'))
        self.assertEqual((self.root/'Shared/new.png').read_bytes(), tiny_png())
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), before)
        self.assertEqual(self.call('media', actor='bob')['resources'], [])
        self.call('media/resolve', {'reference': receipt['resourceRefs'][0]}, 'bob', 404)
        descriptor = self.descriptor(asset)
        ticket = self.call('media/read', {'asset': asset, 'revision': descriptor['revision']})
        self.assertEqual(self.deliver_media(ticket), tiny_png())
        self.assertNotIn(str(self.root), json.dumps(receipt))
        self.assertEqual(self.call('media/import/prepare', request, 'bob')['resourceRefs'], receipt['resourceRefs'])
        self.import_prepare(self.import_request('denied.png'), 'reader', 404)

    def test_collection_original_operation_restart_settle_and_collision(self):
        self.prepare_collections()
        plan, request = self.import_prepare()
        self.server.close(); self.start()
        operation = {'collection': 'attachments', 'operationId': request['operationId']}
        old = self.call('media/import/operation', operation)
        self.assertEqual(old['planId'], plan['planId'])
        self.assertEqual(self.import_commit(old)['resourceRefs'], plan['resourceRefs'])
        self.assertEqual(self.import_commit(old)['status'], 'committed')
        self.import_prepare({**request, 'name': 'different.png'}, status=409)
        self.import_prepare(self.import_request(), status=409)
        unknown = self.import_request('cancelled.png')
        self.assertEqual(self.call('media/import/settle', {'collection': 'attachments',
            'operationId': unknown['operationId']})['status'], 'cancelled')
        late, _ = self.import_prepare(unknown)
        self.assertEqual(late['status'], 'cancelled')
        self.assertFalse((self.root/'Shared/cancelled.png').exists())

    def test_collection_deleted_or_unaccepted_manifest_is_not_new_empty(self):
        self.prepare_collections()
        asset, _, _ = self.import_asset()
        manifest = self.root/'Shared/weftext.resources.json'
        held = manifest.with_name('OwnedOriginalManifest.json')
        manifest.rename(held)
        self.assertEqual(self.call('media')['coverage'], 'partial')
        self.import_prepare(self.import_request('after-missing.png'), status=409)
        self.assertFalse(manifest.exists())
        self.assertTrue((self.root/'Shared/new.png').exists())
        held.rename(manifest)
        self.assertEqual(self.descriptor(asset)['status'], 'ready')
        original = manifest.read_bytes()
        manifest.write_bytes(original + b' ')
        self.assertEqual(self.call('media')['coverage'], 'partial')
        self.import_prepare(self.import_request('after-raw.png'), status=409)
        self.assertFalse((self.root/'Shared/after-raw.png').exists())

    def test_collection_import_revoke_does_not_grant_read_or_revive_original_plan(self):
        self.prepare_collections()
        plan, request = self.import_prepare(actor='bob')
        self.policy(lambda users: users['bob']['resourceCollections']['attachments'].update({'import': False}))
        self.import_commit(plan, 'bob', 404)
        self.policy(lambda users: users['bob']['resourceCollections']['attachments'].update({'import': True}))
        self.import_commit(plan, 'bob', 409)
        self.assertFalse((self.root/'Shared/new.png').exists())
        self.assertEqual(self.call('media', actor='bob')['resources'], [])
        original = self.call('media/import/operation', {'collection': 'attachments',
            'operationId': request['operationId']}, 'bob')
        self.assertFalse(original['commitAllowed'])

    def test_collection_dynamic_annotation_and_exact_download_permissions(self):
        self.prepare_collections()
        asset, _, _ = self.import_asset()
        self.policy(lambda users: users['bob']['resourceCollections']['attachments'].update(read=True))
        descriptor = self.descriptor(asset, 'bob')
        profile = self.media_review(asset, 'bob')
        target = {'kind': 'wholeResource', 'asset': asset, 'resource': descriptor['resource'],
                  'revision': descriptor['revision']}
        plan, _ = self.prepare_annotation(profile, {'kind': 'annotationCreate',
            'body': '动态集合资源批注', 'target': target}, actor='bob')
        self.commit_annotation(plan, actor='bob')
        self.call('media/download/prepare', {'asset': asset, 'revision': descriptor['revision']}, 'bob', 404)
        ticket = self.call('media/read', {'asset': asset, 'revision': descriptor['revision']}, 'bob')
        self.policy(lambda users: users['bob']['resourceCollections']['attachments'].update(read=False))
        self.call('media/chunk', {'ticket': ticket['ticket'], 'offset': 0}, 'bob', 410)
        discussion = self.call('review/read', {'resource': 'Shared'}, 'bob')
        self.assertEqual(len(discussion['annotations']), 1)
        self.assertNotIn('resourceLocation', discussion['annotations'][0])

    def test_collection_native_inputs_static_queries_and_frozen_office(self):
        self.prepare_collections(query=True)
        self.import_asset('supplement.typ', b'COLLECTION_NATIVE_MARKER')
        doc = self.open(); self.save(doc, doc['source']+'\n#include "supplement.typ"\n')
        self.enable_inputs()
        self.grant_query(resources=('Shared',), products=('body',))
        self.assertEqual(len(self.query({'select': [self.column()]}, resources=['Shared'])['result']['rows']), 1)
        self.query({'select': [self.column('body')]}, fields=[], products=['body'], resources=['Shared'], status=404)
        self.policy(lambda users: users['fields'].update(resourceCollections={'attachments': {'read': True}}))
        result = self.query({'select': [self.column('body')]}, fields=[], products=['body'], resources=['Shared'])
        self.assertIn('COLLECTION_NATIVE_MARKER', json.dumps(result))
        doc = self.open(); rendered = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertIn('COLLECTION_NATIVE_MARKER', visible_html_text(rendered['html']))
        HostedOffice.prepare_office(self)
        request = HostedOffice.output_request(self)
        output = self.call('office/export/prepare', request)
        original = HostedOffice.deliver(self, output)
        self.import_asset('later.bin', b'LATER_MEMBER')
        self.assertEqual(HostedOffice.deliver(self, output), original)
        self.policy(lambda users: users['alice']['resourceCollections']['attachments'].update(download=False))
        self.call('office/export/chunk', {'requestId': output['requestId'], 'offset': 0}, status=409)

    def test_collection_semantic_xlsx_keeps_independent_input_download_authority(self):
        self.prepare_collections(query=True)
        self.import_asset('supplement.typ', b'XLSX_COLLECTION_ORIGINAL')
        doc = self.open(); self.save(doc, doc['source']+'\n#include "supplement.typ"\n')
        self.enable_inputs()
        self.grant_query(resources=('Shared',), products=('body',))
        def grant(users):
            users['fields']['resourceCollections'] = {'attachments': {'read': True}}
            users['fields']['grants']['Shared']['office'] = {'export': True, 'download': True}
        self.policy(grant)
        def freeze():
            query = self.query({'select': [self.column('body')]}, fields=[], products=['body'], resources=['Shared'])
            self.assertIn('XLSX_COLLECTION_ORIGINAL', json.dumps(query))
            return {'outputLocale': 'zh-CN', 'requestId': uid(), 'format': 'xlsx', 'reviewMode': 'none', 'includeResolved': False,
                        'stylePolicy': 'template', 'bindings': {}, 'queryId': query['queryId']}
        self.call('office/export/prepare', freeze(), 'fields', 404)
        self.policy(lambda users: users['fields']['resourceCollections']['attachments'].update(download=True))
        output = self.call('office/export/prepare', freeze(), 'fields')
        original = HostedOffice.deliver(self, output, 'fields')
        self.assertIn('XLSX_COLLECTION_ORIGINAL', '\n'.join(HostedOffice.parts(original).values()))
        self.import_asset('later.bin', b'NOT_IN_FROZEN_XLSX')
        self.assertEqual(HostedOffice.deliver(self, output, 'fields'), original)
        self.policy(lambda users: users['fields']['resourceCollections']['attachments'].update(download=False))
        self.call('office/export/chunk', {'requestId': output['requestId'], 'offset': 0}, 'fields', 409)

    def test_collection_membership_change_fences_inflight_and_original_proofread_plan(self):
        self.prepare_collections()
        self.import_asset('supplement.typ', b'ORIGINAL_COLLECTION_INPUT')
        doc = self.open(); self.save(doc, doc['source']+'\n错别字\n#include "supplement.typ"\n')
        self.enable_inputs()
        doc = self.open()
        deadline = time.monotonic() + 40
        # Preparation is not publication. Keep it inside the original total
        # budget, but outside the worker/commit race so it cannot consume it.
        manifest = self.root/'Shared/weftext.resources.json'
        original_membership = manifest.read_bytes()
        members = json.loads(original_membership)['resources']
        payload = b'EXPLICIT_NEW_MEMBER'
        import_request = self.import_request('during-render.bin', payload)
        plan, _ = self.import_prepare(import_request)
        self.assertEqual(manifest.read_bytes(), original_membership)
        # Construct the same 20,000 native paragraphs; project their count and
        # fixed representative text, not a full expansion/capacity result.
        workload = ('\n#let inflight-paragraphs = range(20000).map(i => [delayed paragraph#parbreak()])\n'
                    '#text(str(inflight-paragraphs.len()))\ndelayed paragraph')
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted')
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/editor/render', {'sessionId': doc['sessionId'], 'source': doc['source']+workload, 'serial': 1}, self.tokens['alice'])
                worker = observation.wait(pending, deadline=deadline)
                observation.assert_inflight(pending)
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before change')
                committed, mutation = apply_inflight_mutation(observation, pending,
                    lambda: self.import_commit(plan), deadline)
                self.assertEqual(committed['operationId'], import_request['operationId'])
                self.assertEqual(committed['status'], 'committed')
                self.assertEqual(committed['resourceRefs'], plan['resourceRefs'])
                (code, response, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        self.assertLess(time.monotonic(), deadline, 'Original budget exhausted before membership readback')
        after = manifest.read_bytes()
        actual_members = json.loads(after)['resources']
        added = [member for member in actual_members if member not in members]
        self.assertEqual(len(actual_members),len(members)+1)
        self.assertTrue(all(member in actual_members for member in members))
        self.assertEqual(len(added),1); self.assertEqual(added[0]['path'],'during-render.bin')
        self.assertTrue(committed['resourceRefs'][0].endswith('/'+added[0]['resourceId']))
        descriptor = self.descriptor(committed['assets'][0])
        self.assertEqual(descriptor['resource'],committed['resourceRefs'][0])
        ticket=self.call('media/read',{'asset':committed['assets'][0],'revision':descriptor['revision']})
        self.assertEqual(self.deliver_media(ticket),payload)
        self.assertEqual((self.root/'Shared/during-render.bin').read_bytes(),payload)
        self.assertLess(time.monotonic(), deadline, 'Original budget exhausted during membership readback')
        trace = {'submittedMonotonicNs': submitted, 'worker': worker,
                 **mutation, 'requestCompletedMonotonicNs': completed,
                 'operationId':import_request['operationId'],'planId':plan['planId'],
                 'resourceRefs':committed['resourceRefs'],'addedMember':added[0],
                 'membershipBeforeSha256':hashlib.sha256(original_membership).hexdigest(),
                 'membershipAfterSha256':hashlib.sha256(after).hexdigest(),
                 'payloadSha256':hashlib.sha256(payload).hexdigest(),
                 'status': code, 'errorCode': response.get('error', {}).get('code'),
                 'explanationKey': response.get('error', {}).get('explanation', {}).get('key')}
        with (self.folder/'collection-membership-inflight.json').open('x', encoding='utf-8') as evidence:
            evidence.write(json.dumps(trace, ensure_ascii=False, indent=2))
        self.assertLessEqual(submitted, worker['observedMonotonicNs'])
        verify_inflight_change(trace)
        self.assertEqual(code, 409, response)
        self.assertEqual(response['error']['code'], 'staleEnvironment')
        self.assertEqual(response['error']['explanation']['key'],
                         'error.api.hostedEditor.authorizedDependenciesChanged')
        self.assertNotIn('tree', response)
        options = dict(characters=True, typos=True, pairs=True, sensitive=True,
            allowlist=[], selection=None, rules=[dict(id='collection.plan', version='1',
            category='typo', text='错别字', message='请核对', replacement='正确字', scope={'universal': True})])
        profile = self.call('review/read', {'resource': 'Shared'})
        scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        findings = [f['id'] for f in scan['findings'] if f['sourceRange']]
        self.assertTrue(findings)
        plan, request = self.prepare_annotation(profile, {'annotationLocale': 'zh-CN', 'kind': 'annotationProofread',
            'options': options, 'findings': findings, 'scanId': scan['scanId'], 'ignore': False})
        self.server.close(); self.start()
        self.assertEqual(self.call('review/operation', {'resource': 'Shared',
            'operationId': request['operationId']})['planId'], plan['planId'])
        self.import_asset('after-plan.bin', b'NEW_MEMBERSHIP_AFTER_RESTART')
        self.commit_annotation(plan, status=409)
        self.assertFalse((self.root/'Shared/weftext.annotations.json').exists())

    def test_collection_trash_is_readonly_and_missing_input_is_partial(self):
        self.prepare_collections(query=True)
        asset, _, _ = self.import_asset('supplement.typ', b'TRASH_COLLECTION_INPUT')
        doc = self.open(); self.save(doc, doc['source']+'\n#include "supplement.typ"\n')
        self.enable_inputs()
        self.grant_query(resources=('Shared',), products=('body',))
        def readonly(users):
            users['fields'].update(resourceCollections={'attachments': {'read': True}}, queryScopes={'trash': True})
            for user in users.values():
                if 'attachments' in user.get('resourceCollections', {}):
                    user['resourceCollections']['attachments']['import'] = False
                grant = user.get('grants', {}).get('Shared')
                if grant is not None:
                    grant.update(write=False)
                    grant.pop('annotations', None); grant.pop('office', None)
                    for field in grant.get('fields', {}).values(): field['write'] = False
        self.policy(readonly)
        self.config['users'] = copy.deepcopy(self.call('admin/policy', actor='admin')['users'])
        def trash(api):
            receipt = self.operation_local(api, {'kind': 'trashNode', 'source': str(self.root/'Shared')})
            self.trash_path = Path(receipt['path'])
            self.config['files']['Shared'].update(path=(self.trash_path/'Shared.typ').relative_to(self.root).as_posix(),
                editable=False, importable=False, lifecycle='trash',
                expectedNode=self.config['resourceCollections']['attachments']['expectedOwner'])
            self.config['resourceCollections']['attachments']['lifecycle'] = 'trash'
        self.local(trash)
        plan = {'scope': {'lifecycle': 'trash', 'descendants': True}, 'select': [self.column('body')]}
        result = self.query(plan, fields=[], products=['body'], resources=['Shared'])
        self.assertEqual(result['result']['coverage'], 'complete')
        self.assertIn('TRASH_COLLECTION_INPUT', json.dumps(result))
        self.assertEqual(self.descriptor(asset)['lifecycle'], 'trash')
        self.import_prepare(self.import_request('trash-write.bin'), status=404)
        leaf = self.trash_path/'supplement.typ'
        leaf.rename(self.trash_path/'OwnedMissingSupplement.typ')
        result = self.query(plan, fields=[], products=['body'], resources=['Shared'])
        self.assertEqual(result['result']['coverage'], 'partial')
        self.assertEqual(len(result['result']['rows']), 1)
        self.assertEqual(self.call('media')['coverage'], 'partial')

    def test_collection_docx_slot_original_producer_resources_and_consumers(self):
        self.prepare_import_slot_collection()
        before = self.call('media', actor='bob')
        self.assertEqual(before['resources'], [])
        self.assertEqual(before['collections'][0]['status'], 'unavailable')
        self.assertEqual(before['coverage'], 'partial')
        original = self.install_image_docx()
        self.server.close(); self.start()
        media = self.call('media', actor='bob')
        self.assertEqual(media['coverage'], 'complete')
        image = next(r for r in media['resources'] if r.get('contentType') == 'image/png')
        source_docx = next(r for r in media['resources'] if r.get('name') == 'original.docx')
        self.assertEqual(self.call('documents', actor='bob')['documents'], [])
        ticket = self.call('media/read', {'asset': image['asset'], 'revision': image['revision']}, 'bob')
        self.assertEqual(self.deliver_media(ticket, 'bob'), tiny_png())
        self.call('media/download/prepare', {'asset': image['asset'], 'revision': image['revision']}, 'bob', 404)
        ticket = self.call('media/download/prepare', {'asset': source_docx['asset'],
            'revision': source_docx['revision']})
        self.assertEqual(self.deliver_media(ticket, download=True), original)
        doc = self.open('Imported')
        self.assertIn('image-1.png', doc['source'])
        rendered = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertIn('<img', rendered['html'])
        request = {**HostedOffice.output_request(self), 'resource': 'Imported', 'source': doc['source']}
        output = self.call('office/export/prepare', request)
        parts = unpack(HostedOffice.deliver(self, output))
        self.assertIn(tiny_png(), [value for name, value in parts.items() if name.startswith('word/media/')])
        self.call('media/resolve', {'reference': image['resource']}, 'reader', 404)
        request = {**self.import_request('later.png'), 'collection': 'imported-assets'}
        plan, _ = self.import_prepare(request)
        self.assertEqual(self.import_commit(plan)['status'], 'committed')
        self.assertEqual((self.root/'Imported/document/later.png').read_bytes(), tiny_png())
        self.assertEqual((self.root/'Imported/document/original.docx').read_bytes(), original)
        self.policy(lambda users: users['alice']['resourceCollections']['imported-assets'].update(read=False, download=False))
        self.call('preview', {'sessionId': self.open('Imported')['sessionId'], 'source': doc['source']}, status=404)

    def test_collection_docx_slot_replacement_cannot_borrow_original_birth(self):
        self.prepare_import_slot_collection()
        self.install_image_docx()
        self.assertEqual(self.call('media')['coverage'], 'complete')
        path = self.root/'Imported/document/document.typ'
        retained = path.with_name('OwnedOriginalDocument.typ')
        original = path.read_bytes()
        path.rename(retained)
        path.write_bytes(original)
        catalog = self.call('media')
        self.assertEqual(catalog['coverage'], 'partial')
        self.assertEqual(catalog['collections'][0]['status'], 'unavailable')
        self.assertEqual(catalog['resources'], [])
        self.import_prepare({**self.import_request('not-admitted.png'), 'collection': 'imported-assets'}, status=409)
        self.assertFalse(path.with_name('not-admitted.png').exists())
        self.assertEqual(retained.read_bytes(), original)


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    suite = unittest.TestSuite(HostedResourceCollections(name) for name in sorted(HostedResourceCollections.__dict__)
                              if name.startswith('test_collection_') and (not selected or name in selected))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
