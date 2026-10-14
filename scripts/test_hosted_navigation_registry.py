"""Directory/tag consumers and original actor transactions on owned HTTP fixtures."""
import json
import secrets
import sys
import unittest

from test_hosted import digest, uid
from test_hosted_history import HostedHistory


class HostedNavigationRegistry(HostedHistory):
    def setUp(self):
        super().setUp()
        visible = self.open('Shared')
        hidden = self.open('Secret')
        self.visible_ref = self.call('navigation/ref', {'sessionId': visible['sessionId']})['reference']
        self.hidden_ref = self.call('navigation/ref', {'sessionId': hidden['sessionId']})['reference']
        self.server.close()
        (self.root/'PublicContainer').mkdir()
        (self.root/'PrivateContainer').mkdir()
        self.config['directories'] = {
            'VisibleFolder': {'path': 'Shared', 'title': 'Visible directory', 'reference': self.visible_ref},
            'HiddenFolder': {'path': 'Secret', 'title': 'Hidden directory', 'reference': self.hidden_ref}}
        self.config['navigation'] = {'directoryAdmissions': {
            'PublicAdmission': {'path': 'PublicContainer', 'title': 'Public admission'},
            'HiddenAdmission': {'path': 'PrivateContainer', 'title': 'Private admission'}}}
        self.config['users']['alice']['navigation'] = {'directories': ['VisibleFolder'],
            'admitDirectories': ['PublicAdmission'], 'tags': {'read': True, 'create': True, 'rename': True}}
        self.config['users']['bob']['navigation'] = {'admitDirectories': ['PublicAdmission'], 'tags': {'read': True}}
        self.config['users']['reader']['queryScopes'] = {'directories': ['VisibleFolder']}
        self.tokens['navOnly'] = secrets.token_urlsafe(32)
        self.config['users']['navOnly'] = {'enabled': True, 'tokenSha256': digest(self.tokens['navOnly']),
            'grants': {}, 'navigation': {'directories': ['VisibleFolder']}}
        self.start()
        def install_navigation(users):
            for actor in ['alice', 'bob']:
                users[actor]['navigation'] = self.config['users'][actor]['navigation']
            users['reader']['queryScopes'] = self.config['users']['reader']['queryScopes']
            users['navOnly'] = self.config['users']['navOnly']
        self.policy(install_navigation)

    def registry(self, actor='alice'):
        result = self.call('navigation/registry', actor=actor)
        self.no_physical_paths(result)
        return result

    def prepare_navigation(self, intent, actor='alice', operation=None, status=200):
        request = {'operationId': operation or uid(), 'intent': intent}
        result = self.call('navigation/registry/prepare', request, actor, status)
        self.no_physical_paths(result)
        return result, request

    def commit_navigation(self, prepared, actor='alice', status=200):
        request = {key: prepared[key] for key in ['operationId', 'planId']}
        result = self.call('navigation/registry/commit', request, actor, status)
        self.no_physical_paths(result)
        return result

    def admitted(self, receipt, actor='alice', status=200):
        return self.call('navigation/registry/resolve', {'kind': 'admittedDirectory', 'slot': 'PublicAdmission',
            'operationId': receipt['operationId'], 'reference': receipt['navigationRef']}, actor, status)

    def test_registry_read_roles_are_independent_and_hidden_selectors_never_probe_source_or_path(self):
        catalog = self.registry()
        self.assertEqual([row['directory'] for row in catalog['directories']], ['VisibleFolder'])
        self.assertEqual([row['slot'] for row in catalog['admissionSlots']], ['PublicAdmission'])
        self.assertNotIn('HiddenFolder', json.dumps(catalog))
        self.assertNotIn('HiddenAdmission', json.dumps(catalog))
        readonly = self.registry('navOnly')
        self.assertFalse(readonly['tags']['read'])
        self.assertEqual(readonly['tags']['items'], [])
        self.assertIsNone(readonly['tags']['basis'])
        result = self.call('navigation/registry/resolve', {'kind': 'directory', 'directory': 'VisibleFolder'}, 'navOnly')
        self.assertEqual(result['reference'], self.visible_ref)
        self.assertEqual(result['title'], 'Visible directory')
        self.call('open', {'resource': 'Shared'}, 'navOnly', 404)
        reader = self.registry('reader')
        self.assertEqual(reader['directories'], [])
        self.call('navigation/registry/resolve', {'kind': 'directory', 'directory': 'VisibleFolder'}, 'reader', 404)
        hidden = self.call('navigation/registry/resolve', {'kind': 'directory', 'directory': 'HiddenFolder'}, status=404)
        missing = self.call('navigation/registry/resolve', {'kind': 'directory', 'directory': 'NotThere'}, status=404)
        self.assertEqual(hidden, missing)
        self.call('navigation/registry/resolve', {'kind': 'tag', 'reference': f'weftext-tag://{uid()}/{uid()}'}, 'navOnly', 404)
        # Caller-authored filesystem selectors never enter the typed planner.
        self.call('navigation/registry/prepare', {'operationId': uid(), 'intent': {'kind': 'admitDirectory',
            'slot': 'PublicAdmission', 'path': str(self.root/'PrivateContainer')}}, status=422)

    def test_registry_directory_original_receipt_retry_restart_peer_and_namesake_are_exact(self):
        prepared, request = self.prepare_navigation({'kind': 'admitDirectory', 'slot': 'PublicAdmission'})
        self.assertTrue(prepared['commitAllowed'])
        self.assertTrue(prepared['navigationRef'].startswith('weftext-directory://'))
        self.assertEqual(self.call('navigation/registry/prepare', request), prepared)
        committed = self.commit_navigation(prepared)
        self.assertEqual(committed['navigationRef'], prepared['navigationRef'])
        self.assertTrue(committed['result']['canResolve'])
        self.assertEqual(self.admitted(committed)['title'], 'Public admission')
        self.admitted(committed, 'bob', 404)
        peer = self.call('navigation/registry/operation', {'operationId': request['operationId']}, 'bob')
        self.assertEqual(peer['status'], 'notRegistered')
        self.assertIsNone(peer['navigationRef'])
        self.server.close()
        self.start()
        self.assertEqual(self.call('navigation/registry/prepare', request), committed)
        self.assertEqual(self.admitted(committed)['reference'], prepared['navigationRef'])
        self.server.close()
        (self.root/'PublicContainer').rename(self.root/'OriginalContainer')
        (self.root/'PublicContainer').mkdir()
        (self.root/'PublicContainer/weftext.directory.json').write_bytes((self.root/'OriginalContainer/weftext.directory.json').read_bytes())
        self.start()
        self.admitted(committed, status=404)
        current = self.call('navigation/registry/operation', {'operationId': request['operationId']})
        self.assertEqual(current['status'], 'committed')
        self.assertIsNone(current['result'])

    def test_registry_unknown_fence_revoke_epoch_and_rebound_slot_do_not_commit_another_intent(self):
        unknown = uid()
        settled = self.call('navigation/registry/settle', {'operationId': unknown})
        self.assertEqual(settled['status'], 'cancelled')
        fenced, _ = self.prepare_navigation({'kind': 'admitDirectory', 'slot': 'PublicAdmission'}, operation=unknown)
        self.assertEqual(fenced['status'], 'cancelled')
        self.assertIsNone(fenced['navigationRef'])
        self.assertFalse((self.root/'PublicContainer/weftext.directory.json').exists())
        prepared, request = self.prepare_navigation({'kind': 'admitDirectory', 'slot': 'PublicAdmission'})
        self.policy(lambda users: users['alice']['navigation'].update(admitDirectories=[]))
        stale = self.call('navigation/registry/operation', {'operationId': request['operationId']})
        self.assertEqual(stale['status'], 'prepared')
        self.assertFalse(stale['commitAllowed'])
        self.assertIsNone(stale['planId'])
        self.assertIsNone(stale['navigationRef'])
        self.commit_navigation(prepared, status=409)
        self.assertFalse((self.root/'PublicContainer/weftext.directory.json').exists())
        self.policy(lambda users: users['alice']['navigation'].update(admitDirectories=['PublicAdmission']))
        self.commit_navigation(prepared, status=409)
        self.call('navigation/registry/settle', {'operationId': request['operationId']})
        # Use a fresh original plan in the current epoch, so the following
        # rejection proves exact configuration binding rather than old epoch.
        prepared, request = self.prepare_navigation({'kind': 'admitDirectory', 'slot': 'PublicAdmission'})
        self.server.close()
        (self.root/'ReboundContainer').mkdir()
        self.config['navigation']['directoryAdmissions']['PublicAdmission']['path'] = 'ReboundContainer'
        self.start()
        self.commit_navigation(prepared, status=409)
        self.call('navigation/registry/settle', {'operationId': request['operationId']})
        self.assertFalse((self.root/'PublicContainer/weftext.directory.json').exists())
        self.assertFalse((self.root/'ReboundContainer/weftext.directory.json').exists())
        self.assertFalse((self.root/'PrivateContainer/weftext.directory.json').exists())

    def test_registry_tag_basis_permissions_stale_registry_and_original_ids_survive_restart(self):
        basis = self.registry()['tags']['basis']
        self.assertTrue(basis)
        self.assertIsNone(self.registry('bob')['tags']['basis'])
        intent = {'kind': 'createTag', 'basis': basis, 'displayName': 'Same name'}
        self.prepare_navigation(intent, 'bob', status=404)
        prepared, request = self.prepare_navigation(intent)
        self.assertTrue(prepared['navigationRef'].startswith('weftext-tag://'))
        self.assertEqual(self.call('navigation/registry/prepare', request), prepared)
        committed = self.commit_navigation(prepared)
        self.assertEqual(committed['result']['title'], 'Same name')
        self.prepare_navigation({'kind': 'createTag', 'basis': basis, 'displayName': 'Stale basis'}, status=409)
        current = self.registry()['tags']
        reference = current['items'][0]['reference']
        rename, rename_request = self.prepare_navigation({'kind': 'renameTag', 'basis': current['basis'],
            'reference': reference, 'displayName': 'New name'})
        renamed = self.commit_navigation(rename)
        self.assertEqual(renamed['navigationRef'], reference)
        self.assertEqual(self.call('navigation/registry/resolve', {'kind': 'tag', 'reference': reference}, 'bob')['title'], 'New name')
        self.server.close()
        self.start()
        self.assertEqual(self.call('navigation/registry/prepare', rename_request), renamed)
        self.assertEqual(self.call('navigation/registry/operation', {'operationId': request['operationId']})['navigationRef'], reference)

    def test_registry_stolen_tag_basis_unknown_fence_and_copied_tag_registry_do_not_create_bindings(self):
        self.policy(lambda users: users['bob']['navigation']['tags'].update(create=True))
        basis = self.registry()['tags']['basis']
        self.prepare_navigation({'kind': 'createTag', 'basis': basis, 'displayName': 'Foreign basis'}, 'bob', status=409)
        operation = uid()
        self.call('navigation/registry/settle', {'operationId': operation})
        fenced, _ = self.prepare_navigation({'kind': 'createTag', 'basis': basis, 'displayName': 'Fenced'}, operation=operation)
        self.assertEqual(fenced['status'], 'cancelled')
        self.assertEqual(self.registry()['tags']['items'], [])
        prepared, _ = self.prepare_navigation({'kind': 'createTag', 'basis': self.registry()['tags']['basis'], 'displayName': 'Original tag'})
        committed = self.commit_navigation(prepared)
        self.server.close()
        original = self.root/'.weftext-tags.json'
        original.rename(self.root/'previous-registry.json')
        original.write_bytes((self.root/'previous-registry.json').read_bytes())
        self.start()
        tags = self.registry()['tags']
        self.assertEqual(tags['coverage'], 'partial')
        self.assertEqual(tags['items'], [])
        self.assertIsNone(tags['basis'])
        self.call('navigation/registry/resolve', {'kind': 'tag', 'reference': committed['navigationRef']}, status=404)

    def test_registry_explicit_root_admission_receipt_and_root_reader_do_not_gain_query_or_source(self):
        self.server.close()
        self.config['navigation']['directoryAdmissions']['RootAdmission'] = {'path': '', 'title': 'Library root'}
        self.config['users']['alice']['navigation']['admitDirectories'].append('RootAdmission')
        self.start()
        self.policy(lambda users: users['alice']['navigation']['admitDirectories'].append('RootAdmission'))
        prepared, _ = self.prepare_navigation({'kind': 'admitDirectory', 'slot': 'RootAdmission'})
        receipt = self.commit_navigation(prepared)
        self.assertEqual(receipt['result']['title'], 'Library root')
        self.assertTrue(receipt['result']['canResolve'])
        self.call('navigation/registry/resolve', {'kind': 'admittedDirectory', 'slot': 'RootAdmission',
            'operationId': receipt['operationId'], 'reference': receipt['navigationRef']})
        self.server.close()
        self.config['directories']['RootDirectory'] = {'path': '', 'title': 'Root read', 'reference': receipt['navigationRef']}
        self.config['users']['navOnly']['navigation']['directories'].append('RootDirectory')
        self.start()
        self.policy(lambda users: users['navOnly']['navigation']['directories'].append('RootDirectory'))
        resolved = self.call('navigation/registry/resolve', {'kind': 'directory', 'directory': 'RootDirectory'}, 'navOnly')
        self.assertEqual(resolved['title'], 'Root read')
        self.assertEqual(resolved['reference'], receipt['navigationRef'])
        self.assertIn('RootDirectory', {row['directory'] for row in self.registry('navOnly')['directories']})
        self.call('open', {'resource': 'Shared'}, 'navOnly', 404)
        self.call('navigation/registry/resolve', {'kind': 'directory', 'directory': 'RootDirectory'}, 'reader', 404)
        self.assertNotIn('RootDirectory', self.config['users']['navOnly'].get('queryScopes', {}).get('directories', []))


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    suite = unittest.TestSuite(HostedNavigationRegistry(name) for name in sorted(HostedNavigationRegistry.__dict__)
        if name.startswith('test_registry_') and (not selected or name in selected))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
