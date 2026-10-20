"""Actual hosted Native/BibTeX/CSL/RIS import and separately granted consumers.

Local APIs only provision the owned destination and its admitted Schema/root.
All import effects, original-request recovery and result reads use hosted HTTP.
"""
import base64
import copy
import hashlib
import io
import json
import sys
import unittest
import zipfile

from test_hosted import uid, digest
from test_hosted_history import HostedHistory
from test_hosted_media import HostedMedia
from test_local_features import tiny_png


def native_upload(schema, *, derived=False, tags=False, query=False):
    """Two linked native nodes, one exact registered PNG, and no local custody."""
    workspace, first, second, resource = (uid() for _ in range(4))
    files = {
        '.weftext-schema.json': schema.encode(),
        'One/One.typ': (
            f'#metadata((id: "{first}", types: ("task",), status: "open")) <weftext>\n'
            '= Imported One\nNATIVE ORIGINAL ONE\n'
            f'#link("weftext-node://{workspace}/{second}")[Imported Two]\n'
            '#image("pixel.png", width: 12pt)\n').encode(),
        'Two/Two.typ': (
            f'#metadata((id: "{second}", types: ())) <weftext>\n'
            '= Imported Two\nNATIVE ORIGINAL TWO\n').encode(),
        'One/pixel.png': tiny_png(),
        'One/weftext.resources.json': json.dumps({
            'version': 1, 'resources': [{'resourceId': resource, 'path': 'pixel.png'}]
        }).encode(),
    }
    roles = {
        '.weftext-schema.json': {'role': 'schema'},
        'One/One.typ': {'role': 'source', 'nodeId': first},
        'Two/Two.typ': {'role': 'source', 'nodeId': second},
        'One/pixel.png': {'role': 'resource', 'ownerNodeId': first, 'resourceId': resource},
        'One/weftext.resources.json': {'role': 'resourceManifest', 'ownerNodeId': first},
    }
    if tags:
        parent, child = uid(), uid()
        files['.weftext-tags.json'] = json.dumps({'version': 2, 'tags': [
            {'tagId': parent, 'displayName': 'same/name', 'parentTagId': None},
            {'tagId': child, 'displayName': 'same/name', 'parentTagId': parent},
        ]}).encode()
        roles['.weftext-tags.json'] = {'role': 'tags'}
        reference = f'weftext-tag://{workspace}/{child}'
        files['One/One.typ'] = files['One/One.typ'].replace(
            b'status: "open"',
            f'status: "open", tags: ((ref: "{reference}"), (text: "{reference}"))'.encode())
    if query:
        schema_revision = uid()
        roles['.weftext-schema.json']['revision'] = schema_revision
        expression = (f"tagInSubtree(tagRef('weftext-tag://{workspace}/{child}'), "
                      f"tagRef('weftext-tag://{workspace}/{parent}'))" if tags else 'true')
        declaration = (
            f'\n#metadata((kind: "weftext.query", key: "{uid()}", '
            f'binding: (schema: "weftext-config://{workspace}/schema", '
            f'revision: "{schema_revision}", language: "weftext.query.cel.v1"), '
            f'where: {json.dumps(expression)}, '
            'select: ((name: "value", expr: "true"),))) <weftext-query-imported>\n').encode()
        first_line, body = files['One/One.typ'].split(b'\n', 1)
        files['One/One.typ'] = first_line + b'\n' + declaration + body
    if derived:
        files['One/weftext.citations.bib'] = b'@article{retainedOnly,title={Derived input}}\n'
        roles['One/weftext.citations.bib'] = {'role': 'generatedBibliography', 'ownerNodeId': first}
    manifest = dict(format='weftext.native-package', version=1, packageId=uid(),
                    workspaceId=workspace, scope='library', schemaPath='.weftext-schema.json',
                    losses=[], entries={path: {**roles[path], 'bytes': len(value),
                        'sha256': hashlib.sha256(value).hexdigest()} for path, value in files.items()})
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('manifest.json', json.dumps(manifest))
        for path, value in files.items():
            archive.writestr(path, value)
    return buffer.getvalue(), manifest


def bibliography_upload(format, title='Imported Publication'):
    if format == 'bibtex':
        return f'@article{{originalKey,title={{{title}}},author={{Alpha, Author}},year={{2024}}}}\n'.encode()
    if format == 'csl-json':
        return json.dumps([{'id': 'originalKey', 'type': 'article-journal', 'title': title,
                            'author': [{'family': 'Alpha', 'given': 'Author'}]}]).encode()
    if format == 'ris':
        return f'TY  - JOUR\r\nTI  - {title}\r\nAU  - Alpha, Author\r\nER  - \r\n'.encode()
    raise ValueError(format)


class HostedContentImport(HostedHistory):
    def prepare_imports(self, *, install_schema=True):
        def provision(api):
            schema = api('/api/data/schema', {'root': str(self.root)})
            self.schema_source = schema['source']
            if install_schema:
                plan = api('/api/data/schema/prepare', {
                    'root': str(self.root), 'source': schema['source'], 'beforeSource': schema['source'],
                    'beforeRevision': schema['revision'], 'operationId': uid()})
                api('/api/data/schema/install', {'operationId': plan['operationId'], 'planId': plan['planId']})
            admitted = api('/api/navigation/admit-directory', {
                'root': str(self.root), 'path': str(self.root), 'operationId': uid()})
            plan = admitted['preview']
            api('/api/operations/commit', {'operationId': plan['operationId'], 'planId': plan['planId']})
            self.config['directories'] = {'import-root': {
                'path': '', 'title': 'Import destination', 'reference': admitted['targetRef']}}
            self.config['contentImportSlots'] = {
                name: {'parent': 'import-root', 'name': name, 'title': name,
                       'inputs': [name + '/*']}
                for name in ['Native', 'Bib', 'Csl', 'Ris', 'Retry', 'Loss', 'Denied']}
        self.local(provision)

    def grant_import(self, slots=None, actor='alice', *, members=None, resources=None, schema=True):
        slots = slots or list(self.config['contentImportSlots'])
        def edit(users):
            grants = users[actor].setdefault('contentImports', {})
            for slot in slots:
                grants[slot] = {'imports': {'native': True, 'bibliography': True},
                                'members': copy.deepcopy(members or {}),
                                'resources': copy.deepcopy(resources or {})}
            users[actor].setdefault('library', {})['schema'] = {'read': schema}
        self.policy(edit)

    def grant_shared_import(self, *, tag_read=True, tag_create=True, schema_write=False, status=200):
        def edit(users):
            user = users['alice']
            user.setdefault('navigation', {})['tags'] = {'read': tag_read, 'create': tag_create}
            user.setdefault('library', {})['schema'] = {'read': True, 'write': schema_write}
        if status == 200:
            self.policy(edit)
        else:
            old = self.call('admin/policy', actor='admin')
            users = copy.deepcopy(old['users'])
            edit(users)
            self.call('admin/policy', {'expectedGeneration': old['generation'], 'users': users}, 'admin', status)

    def request(self, slot='Native', format='native', payload=None):
        if payload is None:
            payload = native_upload(self.schema_source)[0] if format == 'native' else bibliography_upload(format)
        return dict(slot=slot, operationId=uid(), format=format,
                    bytesBase64=base64.b64encode(payload).decode())

    def inspect(self, request, actor='alice', status=200):
        return self.call('import/inspect', {k: request[k] for k in ('slot', 'format', 'bytesBase64')}, actor, status)

    def prepare(self, request, actor='alice', status=200):
        result = self.call('import/prepare', request, actor, status)
        self.no_physical_paths(result.decode('utf-8') if isinstance(result, bytes) else result)
        return result

    def commit(self, plan, actor='alice', status=200, *, accept_losses=True):
        return self.call('import/commit', {**{k: plan[k] for k in ('slot', 'operationId', 'planId')},
            'acceptLosses': accept_losses}, actor, status)

    def execute(self, request, actor='alice'):
        plan = self.prepare(request, actor)
        self.assertEqual(plan['status'], 'prepared', plan)
        result = self.commit(plan, actor)
        self.assertEqual(result['status'], 'committed', result)
        self.no_physical_paths(result)
        return result

    def original(self, request, route='operation', actor='alice', status=200):
        return self.call('import/' + route, {k: request[k] for k in ('slot', 'operationId')}, actor, status)

    def member_list(self, slot='Native', actor='alice', status=200):
        return self.call('import/members', {'slot': slot}, actor, status)

    def test_content_import_default_denial_schema_and_closed_input(self):
        self.prepare_imports()
        request = self.request()
        self.assertEqual(self.call('import')['targets'], [])
        self.prepare(request, status=404)
        self.grant_import(schema=False)
        self.inspect(request, status=404)
        self.prepare(request, status=404)
        self.grant_import()
        for extra in [{'root': str(self.root)}, {'destination': str(self.root / 'Other')},
                      {'choices': []}, {'nodeId': uid()}]:
            self.prepare({**request, **extra}, status=422)
        self.inspect({**request, 'bytesBase64': 'not base64'}, status=400)
        self.inspect(self.request(payload=b'not a ZIP'), status=400)
        self.assertFalse((self.root / 'Native').exists())
        result = self.execute(request)
        self.assertEqual(len(result['nodeRefs']), 2)
        self.member_list(status=404)
        member = 'Native/' + result['nodeRefs'][0]['nodeId']
        self.call('open', {'resource': member}, status=404)
        self.assertEqual(self.call('media')['resources'], [])
        self.assertEqual(self.call('import', actor='bob')['targets'], [])

    def test_content_import_native_tags_do_not_inherit_shared_rights_from_destination_slot(self):
        self.prepare_imports()
        self.grant_import(members={'read': True})
        payload, manifest = native_upload(self.schema_source, tags=True)
        request = self.request(payload=payload)
        self.grant_shared_import(tag_read=False, tag_create=True, status=400)
        for read, create in [(False, False), (True, False)]:
            self.grant_shared_import(tag_read=read, tag_create=create)
            self.inspect(request, status=404)
            self.prepare(request, status=404)
            self.assertEqual(self.original(request)['status'], 'notRegistered')
            self.assertFalse((self.root / 'Native').exists())
            self.assertFalse((self.root / '.weftext-tags.json').exists())
        self.grant_shared_import()
        prepared = self.prepare(request)
        self.assertTrue(prepared['commitAllowed'])
        self.assertEqual(set(prepared['preview']['sharedWrites']), {'tags.read', 'tags.create'})
        # A matching installed Schema causes no Schema write and grants no
        # unrelated rename/move action merely because it imports a hierarchy.
        policy = self.call('admin/policy', actor='admin')
        self.assertFalse(policy['users']['alice']['library']['schema']['write'])
        self.assertFalse(policy['users']['alice']['navigation']['tags']['move'])
        self.assertFalse((self.root / 'Native').exists())
        result = self.commit(prepared)
        self.assertEqual(result['status'], 'committed')
        self.assertEqual(self.call('admin/policy', actor='admin'), policy)
        registry = self.call('navigation/registry')['tags']
        self.assertEqual(registry['coverage'], 'complete')
        self.assertEqual(len(registry['items']), 2)
        parent = next(tag for tag in registry['items'] if tag['parentTagId'] is None)
        child = next(tag for tag in registry['items'] if tag['parentTagId'] is not None)
        self.assertEqual(child['parentTagId'], parent['tagId'])
        with zipfile.ZipFile(io.BytesIO(payload)) as archive:
            original_ids = {tag['tagId'] for tag in json.loads(archive.read('.weftext-tags.json'))['tags']}
        self.assertTrue(original_ids.isdisjoint({tag['tagId'] for tag in registry['items']}))
        self.assertEqual((self.root / 'Native/.weftext-import-originals/original.weftext').read_bytes(), payload)
        members = [self.open(member['resource']) for member in self.member_list()['members']]
        one = next(member for member in members if 'NATIVE ORIGINAL ONE' in member['source'])
        self.assertIn(f'ref: "{child["reference"]}"', one['source'])
        self.assertIn(f'text: "weftext-tag://{manifest["workspaceId"]}/', one['source'])
        hidden = self.call('navigation/registry', actor='bob')['tags']
        self.assertFalse(hidden['read'])
        self.assertEqual(hidden['items'], [])

    def test_content_import_native_schema_initialization_and_shared_role_revoke_fence(self):
        self.prepare_imports(install_schema=False)
        self.grant_import(members={'read': True})
        self.grant_shared_import()
        payload, manifest = native_upload(self.schema_source, tags=True, query=True)
        denied = self.request(payload=payload)
        self.inspect(denied, status=404)
        self.prepare(denied, status=404)
        self.assertEqual(self.original(denied)['status'], 'notRegistered')
        self.assertFalse((self.root / '.weftext-schema.json').exists())
        self.assertFalse((self.root / '.weftext-tags.json').exists())
        for revoked in ['tags.read', 'tags.create', 'schema.write']:
            self.grant_shared_import(schema_write=True)
            request = self.request(payload=payload)
            prepared = self.prepare(request)
            self.assertEqual(set(prepared['preview']['sharedWrites']),
                             {'tags.read', 'tags.create', 'schema.write'})
            options = {'schema_write': True}
            options[{'tags.read': 'tag_read', 'tags.create': 'tag_create',
                     'schema.write': 'schema_write'}[revoked]] = False
            if revoked == 'tags.read':
                options['tag_create'] = False
            self.grant_shared_import(**options)
            retained = self.original(request)
            self.assertEqual(retained['status'], 'prepared')
            self.assertFalse(retained['commitAllowed'])
            self.assertIsNone(retained['planId'])
            self.assertEqual(retained['preview'], {})
            self.assertEqual(retained['nodeRefs'], [])
            self.commit(prepared, status=409)
            self.grant_shared_import(schema_write=True)
            self.commit(prepared, status=409)
            self.assertFalse(self.prepare(request)['commitAllowed'])
            self.assertEqual(self.original(request, 'settle')['status'], 'cancelled')
            self.assertFalse((self.root / 'Native').exists())
            self.assertFalse((self.root / '.weftext-schema.json').exists())
            self.assertFalse((self.root / '.weftext-tags.json').exists())
        request = self.request(payload=payload)
        prepared = self.prepare(request)
        self.server.close()
        self.start()
        replay = self.prepare(request)
        self.assertEqual(replay['planId'], prepared['planId'])
        self.assertTrue(replay['commitAllowed'])
        result = self.commit(replay)
        self.assertEqual(result['status'], 'committed')
        self.assertTrue((self.root / '.weftext-schema.json').exists())
        self.assertTrue(self.call('library/read', {'role': 'schema'})['installed'])
        self.assertEqual(self.call('navigation/registry')['tags']['coverage'], 'complete')
        members = [self.open(member['resource']) for member in self.member_list()['members']]
        one = next(member for member in members if 'NATIVE ORIGINAL ONE' in member['source'])
        workspace = result['nodeRefs'][0]['workspaceId']
        self.assertIn(f'binding: (schema: "weftext-config://{workspace}/schema"', one['source'])
        self.assertNotIn(f'binding: (schema: "weftext-config://{manifest["workspaceId"]}/schema"', one['source'])
        self.assertEqual((self.root / 'Native/.weftext-import-originals/original.weftext').read_bytes(), payload)

    def test_content_import_native_members_remap_resources_and_actual_consumers(self):
        self.prepare_imports()
        rights = {'read': True, 'write': True, 'history': True,
                  'fields': {'tasks/status': {'read': True, 'write': True}},
                  'office': {'export': True, 'download': True}}
        self.grant_import(members=rights, resources={'read': True, 'download': True})
        payload, manifest = native_upload(self.schema_source)
        before = self.open()
        self.call('draft', {'sessionId': before['sessionId'], 'source': before['source'] + '\nKEEP PERSONAL DRAFT'})
        policy = self.call('admin/policy', actor='admin')
        result = self.execute(self.request(payload=payload))
        self.assertEqual(self.call('admin/policy', actor='admin'), policy)
        self.assertEqual((self.root / 'Native/.weftext-import-originals/original.weftext').read_bytes(), payload)
        members = self.member_list()
        self.assertEqual(members['coverage'], 'complete')
        self.assertEqual(len(members['members']), 2)
        self.assertTrue(all(m['canOpen'] for m in members['members']))
        old_ids = {e['nodeId'] for e in manifest['entries'].values() if 'nodeId' in e}
        self.assertTrue(old_ids.isdisjoint({r['nodeId'] for r in result['nodeRefs']}))
        opened = [self.open(m['resource']) for m in members['members']]
        one = next(d for d in opened if 'NATIVE ORIGINAL ONE' in d['source'])
        two = next(d for d in opened if 'NATIVE ORIGINAL TWO' in d['source'])
        ref = self.call('navigation/ref', {'sessionId': two['sessionId']})['reference']
        self.assertIn(ref, one['source'])
        self.assertNotIn(manifest['workspaceId'], one['source'])
        profile = self.call('fields/read', {'resource': one['resource'], 'fieldId': 'tasks/status'})
        self.assertTrue(profile['basis'])
        query = self.call('query', {'resources': [one['resource']], 'fields': ['tasks/status'],
            'plan': {'select': [{'name': 'status', 'expr': 'status'}]}})
        self.assertEqual(query['result']['rows'][0]['entityKey'], one['resource'])
        self.assertEqual(query['result']['rows'][0]['cells']['status']['value']['value'], 'open')
        media = self.call('media')
        image = next(r for r in media['resources'] if r.get('contentType') == 'image/png')
        self.assertTrue(image['asset'].startswith(one['resource'] + '/'))
        ticket = self.call('media/download/prepare', {'asset': image['asset'], 'revision': image['revision']})
        self.assertEqual(HostedMedia.deliver_media(self, ticket, download=True), tiny_png())
        preview = self.call('preview', {'sessionId': one['sessionId'], 'source': one['source']})
        self.assertTrue(preview['ok'], preview)
        self.assertIn('<img', preview['html'])
        basis = dict(sessionId=one['sessionId'], source=one['source'], sourceHash=digest(one['source']), serial=1)
        self.call('editor/map', basis)
        links = self.call('knowledge-links/read', basis)
        edge = next(e for e in links['links']['outgoing'] if e['occurrence']['target'] == ref)
        self.assertEqual(edge['resolution']['target']['resource'], two['resource'])
        literal = self.call('knowledge-links/copy', {
            'basis': basis, 'snapshot': links['basis']['snapshot'], 'form': 'original',
            'occurrence': {k: edge['occurrence'][k] for k in ('from', 'to', 'target')}})
        self.assertEqual(literal['address'], ref)
        self.assertTrue(literal['literal'])
        self.save(one, one['source'] + '\nSAVED IMPORTED MEMBER')
        self.assertIn('SAVED IMPORTED MEMBER', self.open(one['resource'])['source'])
        self.assertTrue(self.call('history/list', {'sessionId': self.open(one['resource'])['sessionId']})['versions'])
        self.assertIn('KEEP PERSONAL DRAFT', self.open()['recoveredDraft'])
        self.call('open', {'resource': one['resource']}, 'bob', 404)

    def test_content_import_bibliography_three_formats_field_only_original_resources(self):
        self.prepare_imports()
        field_only = {'fields': {'library/publication': {'read': True}},
                      'citations': {'use': True, 'export': True, 'download': True},
                      'outputs': {'bibliography': {'export': True, 'download': True}}}
        self.grant_import(members=field_only, resources={'read': True, 'download': True})
        for slot, format, filename in [('Bib', 'bibtex', 'original.bib'),
                                      ('Csl', 'csl-json', 'original.csl.json'), ('Ris', 'ris', 'original.ris')]:
            payload = bibliography_upload(format, 'Publication ' + slot)
            request = self.request(slot, format, payload)
            self.assertTrue(self.inspect(request)['canPrepare'])
            result = self.execute(request)
            self.assertEqual(len(result['nodeRefs']), 1)
            member = self.member_list(slot)['members'][0]
            self.assertFalse(member['canOpen'])
            self.call('open', {'resource': member['resource']}, status=404)
            field = self.call('fields/read', {'resource': member['resource'], 'fieldId': 'library/publication'})
            self.assertIn('Publication ' + slot, json.dumps(field))
            row = self.call('query', {'resources': [member['resource']], 'fields': ['library/publication'],
                'plan': {'select': [{'name': 'publication', 'expr': 'publication'}]}})['result']['rows'][0]
            self.assertEqual(row['entityKey'], member['resource'])
            media = self.call('media')['resources']
            original = next(r for r in media if r.get('name') == filename and r['asset'].startswith(member['resource'] + '/'))
            ticket = self.call('media/download/prepare', {'asset': original['asset'], 'revision': original['revision']})
            self.assertEqual(HostedMedia.deliver_media(self, ticket, download=True), payload)
            self.assertIn(member['resource'], [w['resource'] for w in self.call('citations')['works']])
        self.assertFalse(any(d['resource'].startswith(('Bib/', 'Csl/', 'Ris/')) for d in self.call('documents')['documents']))
        self.assertEqual(self.call('citations', actor='bob')['works'], [])

    def test_content_import_structure_uses_explicit_original_source_rights(self):
        self.prepare_imports()
        self.grant_import(members={'read': True, 'write': True})
        self.execute(self.request())
        members = self.member_list()['members']
        resource = members[0]['resource']
        original = self.open(resource)['source']
        request = {'resource': resource, 'source': resource,
                   'operationId': uid(), 'kind': 'trash'}
        self.assertEqual(self.call('structure')['sources'], [])
        self.call('structure/prepare', request, status=404)
        self.policy(lambda users: users['alice'].update(structure={
            'sources': {'Native/*': {'trash': True}}, 'destinations': {}}))
        available = self.call('structure')['sources']
        self.assertEqual({row['resource'] for row in available},
                         {row['resource'] for row in members})
        self.assertTrue(all(row['actions'] == ['trash'] for row in available))
        prepared = self.call('structure/prepare', request)
        result = self.call('structure/commit', {'resource': resource,
            'operationId': request['operationId'], 'planId': prepared['planId'],
            'acceptLosses': False})
        self.assertEqual(result['status'], 'committed')
        status = self.call('structure/operation', {
            'resource': resource, 'operationId': request['operationId']})
        self.assertEqual(status['status'], 'committed')
        self.assertTrue(any(path.read_bytes() == original.encode('utf-8')
                            for path in (self.root / '.weftext-trash').rglob('*.typ')))
        self.assertEqual(self.member_list()['coverage'], 'partial')
        self.call('structure/prepare', {**request, 'operationId': uid()}, status=409)
        self.assertEqual(self.call('structure', actor='bob')['sources'], [])

    def test_content_import_original_plan_restart_actor_fence_and_changed_upload(self):
        self.prepare_imports()
        self.grant_import()
        request = self.request('Retry')
        prepared = self.prepare(request)
        self.server.close(); self.start()
        replay = self.prepare(request)
        self.assertEqual(replay['planId'], prepared['planId'])
        self.prepare({**self.request('Retry'), 'operationId': request['operationId']}, status=409)
        self.assertEqual(self.commit(replay)['status'], 'committed')
        original = (self.root / 'Retry/.weftext-import-originals/original.weftext').read_bytes()
        self.server.close(); self.start()
        self.assertEqual(self.original(request)['status'], 'committed')
        self.assertEqual(self.prepare(request)['status'], 'committed')
        self.assertEqual(self.commit(prepared)['status'], 'committed')
        self.assertEqual((self.root / 'Retry/.weftext-import-originals/original.weftext').read_bytes(), original)
        self.grant_import(actor='bob')
        self.assertEqual(self.original(request, actor='bob')['status'], 'notRegistered')
        self.assertEqual(self.original(request, 'settle', actor='bob')['status'], 'cancelled')
        self.assertEqual(self.original(request)['status'], 'committed')
        unknown = self.request('Denied')
        self.assertEqual(self.original(unknown, 'settle')['status'], 'cancelled')
        self.assertEqual(self.prepare(unknown)['status'], 'cancelled')
        self.assertFalse((self.root / 'Denied').exists())

    def test_content_import_actual_losses_require_explicit_acceptance(self):
        self.prepare_imports(); self.grant_import(members={'read': True})
        payload, _ = native_upload(self.schema_source, derived=True)
        request = self.request('Loss', payload=payload)
        plan = self.prepare(request)
        self.assertIn('generatedBibliographyNotInstalled', json.dumps(plan['preview']['losses']))
        self.commit(plan, status=409, accept_losses=False)
        self.assertFalse((self.root / 'Loss').exists())
        self.assertEqual(self.commit(plan)['status'], 'committed')
        self.assertFalse((self.root / 'Loss/One/weftext.citations.bib').exists())
        self.assertEqual((self.root / 'Loss/.weftext-import-originals/original.weftext').read_bytes(), payload)

    def test_content_import_upload_conflicts_are_reported_without_partial_work_creation(self):
        self.prepare_imports(); self.grant_import()
        publication = {'kind': 'record', 'value': [
            {'key': 'kind', 'value': {'kind': 'text', 'value': 'article'}},
            {'key': 'title', 'value': {'kind': 'text', 'value': 'Preserved exact title'}}]}
        row = {'id': 'originalKey', 'type': 'article-journal', 'title': 'Externally changed title',
               'weftext': {'version': 1, 'publication': publication}}
        payload = json.dumps([row]).encode()
        request = self.request('Csl', 'csl-json', payload)
        inspected = self.inspect(request)
        self.assertFalse(inspected['canPrepare'])
        conflicts = inspected['records'][0]['conflicts']
        title = next(c for c in conflicts if c['key'] == 'title')
        self.assertEqual(title['current']['value'], 'Preserved exact title')
        self.assertEqual(title['imported']['value'], 'Externally changed title')
        self.prepare(request, status=409)
        self.assertFalse((self.root / 'Csl').exists())
        self.assertEqual(self.original(request)['status'], 'notRegistered')
        self.assertEqual(base64.b64decode(request['bytesBase64']), payload)

    def test_content_import_revocation_regrant_and_destination_rebind_stale_original(self):
        self.prepare_imports(); self.grant_import()
        first = self.request(); prepared = self.prepare(first)
        self.policy(lambda users: users['alice']['contentImports']['Native']['imports'].update(native=False))
        self.commit(prepared, status=409)
        self.grant_import()
        self.commit(prepared, status=409)
        self.assertFalse(self.prepare(first)['commitAllowed'])
        self.assertFalse((self.root / 'Native').exists())
        # A fresh current-epoch decision proves config rebind independently.
        second = self.request('Retry'); second_plan = self.prepare(second)
        self.server.close()
        self.config['contentImportSlots']['Retry']['name'] = 'DifferentDestination'
        self.start()
        self.commit(second_plan, status=409)
        self.assertFalse(self.prepare(second)['commitAllowed'])
        self.assertFalse((self.root / 'Retry').exists())
        self.assertFalse((self.root / 'DifferentDestination').exists())

    def test_content_import_same_byte_replacement_remains_partial_without_namesake_admission(self):
        self.prepare_imports(); self.grant_import(members={'read': True}, resources={'read': True})
        result = self.execute(self.request())
        before = self.member_list()
        path = self.root / 'Native/One/One.typ'
        original = path.read_bytes()
        retained = path.with_name('retained-original.typ')
        path.rename(retained); path.write_bytes(original)
        after = self.member_list()
        self.assertEqual(len(after['members']), len(before['members']))
        self.assertEqual(after['coverage'], 'partial')
        unavailable = [m for m in after['members'] if m['status'] == 'unavailable']
        self.assertTrue(unavailable)
        for row in unavailable:
            self.call('open', {'resource': row['resource']}, status=409)
        self.assertEqual({m['nodeRef']['nodeId'] for m in after['members']},
                         {r['nodeId'] for r in result['nodeRefs']})
        self.assertEqual(retained.read_bytes(), original)


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    available = {name for name in HostedContentImport.__dict__ if name.startswith('test_content_import_')}
    if selected - available:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(selected - available)))
    suite = unittest.TestSuite(HostedContentImport(name) for name in sorted(selected or available))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
