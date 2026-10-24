"""Real hosted v0.6.1 shared-dictionary scans and frozen-operation acceptance.

Supply a newly built executable explicitly. Every scan uses the actual rendering
and proofreading workers. Owned libraries, state, HTTP records and process logs
are retained; this suite does not establish browser, Office or IME acceptance.
Only the tests declared below run, including through unittest discovery.
"""
import uuid
from _creation_result import created_document
import argparse
import copy
import hashlib
import json
from pathlib import Path
import secrets
import sys
import tempfile
import unittest

import test_hosted as hosted_harness
from test_hosted_library import HostedLibrary


ARTIFACT_PARENT = None
EXECUTABLE = None
MARKER = 'LCSharedDictionaryToken'
MESSAGE = 'OWNED_PRIVATE_SHARED_DICTIONARY_SENTINEL'


def retained_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')


def dictionary(version='1'):
    return {'schemaVersion': 1, 'defaults': {
        'version': '1', 'languages': [], 'scheme': 'general',
        'inputFormat': 'typst', 'documentType': 'prose', 'dependencies': []},
        'rules': [{'id': 'owned.shared.dictionary', 'version': version,
                   'category': 'typo', 'text': MARKER, 'message': MESSAGE + ':' + version,
                   'replacement': 'LC_REVIEWED', 'scope': {'universal': True}}],
        'scopedAllowlist': []}


class EvidenceServer(hosted_harness.Server):
    def __init__(self, args, prefix, hosted=False):
        self.prefix = prefix
        self.closed = False
        super().__init__(args, hosted=hosted)

    def close(self):
        if self.closed:
            return
        stdout, stderr = super().close()
        self.closed = True
        # Hosted output has no bearer tokens; bootstrap's startup URL was
        # consumed by the harness and its remaining output is redacted too.
        if self.token:
            stdout = stdout.replace(self.token, '<redacted>')
            stderr = stderr.replace(self.token, '<redacted>')
        Path(str(self.prefix) + '.stdout.log').write_text(stdout, encoding='utf-8')
        Path(str(self.prefix) + '.stderr.log').write_text(stderr, encoding='utf-8')


class SharedProcessingIntegration(HostedLibrary):
    @classmethod
    def setUpClass(cls):
        cls.evidence = Path(tempfile.mkdtemp(prefix='weftext-v061-processing-integration-',
                                             dir=ARTIFACT_PARENT))
        retained_json(cls.evidence / 'candidate.json', {
            'executable': str(EXECUTABLE),
            'sha256': hashlib.sha256(EXECUTABLE.read_bytes()).hexdigest()})
        print(f'Shared processing fixtures (retained): {cls.evidence}', flush=True)

    def record(self, route, request, code, response, actor):
        with (self.folder / 'http.ndjson').open('a', encoding='utf-8') as output:
            output.write(json.dumps({'actor': actor, 'route': route, 'request': request,
                                     'status': code, 'response': response},
                                    ensure_ascii=False) + '\n')

    def setUp(self):
        self.folder = self.evidence / self._testMethodName
        self.folder.mkdir()
        self.data = self.folder / 'state'
        self.root = self.folder / 'library'
        self.server = None
        local = EvidenceServer([ '--data-dir', str(self.data)],
                               self.folder / 'bootstrap')
        try:
            def call(route, request):
                code, result, _ = local.call(route, request)
                self.record(route, request, code, result, 'owned-bootstrap')
                self.assertEqual(code, 200, result)
                return result
            call('/api/library/create', {'path': str(self.root), 'operationId': hosted_harness.uid()})
            self.docs = {}
            for name, text in (('Shared', MARKER), ('Other', '独立的原稿😀')):
                doc = created_document(call('/api/note/create', {'root': str(self.root), 'parent': str(self.root),
                    'name': name, 'operationId': hosted_harness.uid()}))
                source = doc['source'] + '\n\n' + text + '\n'
                call('/api/save', {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                    'source': source, 'operationId': hosted_harness.uid()})
                self.docs[name] = doc
        finally:
            local.close()
        self.tokens = {actor: secrets.token_urlsafe(32) for actor in ('alice', 'bob', 'admin')}
        full = {'read': True, 'write': True, 'history': True, 'download': True,
                'annotations': {'read': True, 'write': True, 'target': True,
                                'accept': True, 'proofread': True}}
        self.config = {'deployment': 'v061-processing-integration', 'bind': '127.0.0.1:0',
            'root': str(self.root), 'adminTokenSha256': hosted_harness.digest(self.tokens['admin']),
            'files': {name: {'path': f'{name}/{name}.typ', 'title': name, 'editable': True}
                      for name in self.docs}, 'users': {
                'alice': {'enabled': True, 'tokenSha256': hosted_harness.digest(self.tokens['alice']),
                    'grants': {name: copy.deepcopy(full) for name in self.docs},
                    'library': {'settings': {'read': True, 'write': True}, 'styleUse': False}},
                'bob': {'enabled': True, 'tokenSha256': hosted_harness.digest(self.tokens['bob']),
                    'grants': {'Shared': copy.deepcopy(full)}, 'library': {'styleUse': False}}}}
        self.config_path = self.folder / 'server.json'
        self.start_number = 0
        self.start()

    def tearDown(self):
        if self.server is not None:
            self.server.close()

    def start(self):
        retained_json(self.config_path, self.config)
        self.start_number += 1
        self.server = EvidenceServer([ '--data-dir', str(self.data),
            '--server-config', str(self.config_path)],
            self.folder / f'hosted-{self.start_number:02d}', hosted=True)

    def call(self, route, data=None, actor='alice', status=200, headers=None):
        code, result, returned = self.server.call('/api/hosted/' + route, data,
                                                  self.tokens.get(actor, 'x' * 40), headers)
        self.record('/api/hosted/' + route, data, code, result, actor)
        self.assertEqual(code, status, str(result)[:1500])
        if code == 200:
            self.assertEqual(returned.get('Cache-Control'), 'no-store')
        return result

    def install_dictionary(self, version='1'):
        current = self.library_read('settings')
        settings = json.loads(current['source'])
        settings['processing'] = dictionary(version)
        source = json.dumps(settings, ensure_ascii=False)
        plan, request = self.library_prepare('settings', source, basis=current)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        self.assertEqual(self.call('library/prepare', request)['status'], 'committed')
        return self.library_read('settings')

    def options(self, profile):
        document = json.loads(profile['source'])['processing']
        return {'characters': False, 'typos': True, 'pairs': False, 'sensitive': False,
            'rules': document['rules'], 'allowlist': [],
            'scopedAllowlist': document['scopedAllowlist'], 'processing': document['defaults'],
            'processingOrigin': {'source': 'shared-library', 'baseId': profile['basis']},
            'selection': None}

    def scan(self, options, actor='alice'):
        profile = self.call('review/read', {'resource': 'Shared'}, actor)
        result = self.call('review/proofread', {'resource': 'Shared',
            'basis': profile['basis'], 'options': options}, actor)
        matches = [finding for finding in result['findings']
                   if finding['ruleId'] == 'owned.shared.dictionary']
        self.assertEqual(len(matches), 1, result)
        finding = matches[0]
        self.assertEqual(finding['messageOrigin'], 'author')
        self.assertEqual(finding['authorMessage'], options['rules'][0]['message'])
        self.assertNotIn('message', finding)
        self.assertEqual(finding['replacement'], 'LC_REVIEWED')
        self.assertTrue(finding['sourceRange'], finding)
        self.assertTrue(finding['executable'], finding)
        self.assertNotIn(str(self.root), json.dumps(result))
        self.assertNotIn(str(self.data), json.dumps(result))
        return profile, result

    def request(self, profile, result, options):
        return {'resource': 'Shared', 'basis': profile['basis'],
            'operationId': hosted_harness.uid(), 'intent': {'annotationLocale': 'zh-CN', 'kind': 'annotationProofread',
                'scanId': result['scanId'], 'options': options,
                'findings': [finding['id'] for finding in result['findings']
                             if finding['ruleId'] == 'owned.shared.dictionary'], 'ignore': False}}

    def operation(self, request, actor='alice'):
        return self.call('review/operation', {'resource': 'Shared',
            'operationId': request['operationId']}, actor)

    def commit_review(self, plan, status=200):
        return self.call('review/commit', {key: plan[key]
            for key in ('resource', 'operationId', 'planId')}, status=status)

    def originals(self):
        return {name: (self.root / name / (name + '.typ')).read_bytes() for name in self.docs}

    def replace_settings(self, label):
        path = self.root / '.weftext-settings.json'
        before = path.read_bytes()
        retained = self.folder / f'{label}.retained-settings.json'
        path.rename(retained)
        path.write_bytes(before)
        self.assertEqual(retained.read_bytes(), path.read_bytes())
        self.assertNotEqual(retained.stat().st_ino, path.stat().st_ino)
        return path, before

    def assert_hidden_pending(self, status):
        self.assertEqual(status['status'], 'prepared')
        self.assertEqual(status['preview'], {})
        self.assertFalse(status['commitAllowed'])
        self.assertIsNone(status.get('planId'))

    def test_v061_shared_dictionary_real_worker_and_original_plan_recovery(self):
        options = self.options(self.install_dictionary())
        original = self.originals()
        draft_doc = self.open()
        draft = draft_doc['source'] + '\n个人未保存草稿😀'
        self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': draft_doc['baseId'], 'inputRevision': 1, 'sessionId': draft_doc['sessionId'], 'source': draft})
        profile, result = self.scan(options)
        request = self.request(profile, result, options)
        plan = self.call('review/prepare', request)
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['preview']['count'], 1)
        self.assertTrue(plan['commitAllowed'])
        self.assertEqual(self.call('review/prepare', request)['planId'], plan['planId'])
        self.assertEqual(self.operation(request)['planId'], plan['planId'])
        self.assertEqual(self.operation(request, 'bob')['status'], 'notRegistered')
        self.assertFalse((self.root / 'Shared/weftext.annotations.json').exists())
        self.server.close()
        self.start()
        recovered = self.operation(request)
        self.assertEqual(recovered['planId'], plan['planId'])
        self.assertEqual(recovered['preview'], plan['preview'])
        receipt = self.commit_review(recovered)
        self.assertEqual(receipt['status'], 'committed')
        sidecar = self.root / 'Shared/weftext.annotations.json'
        written = sidecar.read_bytes()
        annotations = self.call('review/read', {'resource': 'Shared'})['annotations']
        self.assertEqual(len(annotations), 1)
        self.assertEqual(annotations[0]['body'], MESSAGE + ':1')
        self.assertEqual(annotations[0]['replacement'], 'LC_REVIEWED')
        self.assertEqual(self.commit_review(plan)['status'], 'committed')
        self.server.close()
        self.start()
        self.assertEqual(self.call('review/prepare', request)['status'], 'committed')
        self.assertEqual(self.commit_review(plan)['status'], 'committed')
        self.assertEqual(sidecar.read_bytes(), written)
        self.assertEqual(self.originals(), original)
        self.assertEqual(self.open()['recoveredDraft'], draft)

    def test_v061_private_shared_dictionary_requires_own_current_settings_read(self):
        options = self.options(self.install_dictionary())
        original = self.originals()
        private = self.call('library/read', {'role': 'settings'}, 'bob', 404)
        self.assertNotIn(MESSAGE, json.dumps(private))
        bob = self.call('review/read', {'resource': 'Shared'}, 'bob')
        denied = self.call('review/proofread', {'resource': 'Shared', 'basis': bob['basis'],
            'options': options}, 'bob', 404)
        self.assertNotIn('findings', denied)
        self.assertNotIn(MESSAGE, json.dumps(denied))
        # Even equal current role rights cannot transfer Alice's opaque token.
        self.grant_library(roles=('settings',), actor='bob', write=False, style_use=False)
        bob = self.call('review/read', {'resource': 'Shared'}, 'bob')
        denied = self.call('review/proofread', {'resource': 'Shared', 'basis': bob['basis'],
            'options': options}, 'bob', 409)
        self.assertNotIn('findings', denied)
        own_options = self.options(self.library_read('settings', 'bob'))
        self.scan(own_options, 'bob')
        profile, result = self.scan(options)
        request = self.request(profile, result, options)
        self.policy(lambda users: users['alice']['library'].update(
            settings={'read': False, 'write': False}))
        fresh = self.call('review/read', {'resource': 'Shared'})
        self.call('review/proofread', {'resource': 'Shared', 'basis': fresh['basis'],
            'options': options}, status=404)
        self.call('review/prepare', request, status=409)
        self.assertEqual(self.operation(request)['status'], 'notRegistered')
        self.assertEqual(self.originals(), original)
        self.assertFalse((self.root / 'Shared/weftext.annotations.json').exists())

    def test_v061_original_settings_token_rejects_same_bytes_replacement_before_scan(self):
        options = self.options(self.install_dictionary())
        original = self.originals()
        profile = self.call('review/read', {'resource': 'Shared'})
        path, before = self.replace_settings('before-scan')
        denied = self.call('review/proofread', {'resource': 'Shared', 'basis': profile['basis'],
            'options': options}, status=409)
        self.assertNotIn('findings', denied)
        self.assertEqual(path.read_bytes(), before)
        # Only a new explicit settings read may form a new scan; the old token
        # cannot be repaired by any newly captured same-byte snapshot.
        self.scan(self.options(self.library_read('settings')))
        self.assertEqual(self.originals(), original)
        self.assertFalse((self.root / 'Shared/weftext.annotations.json').exists())

    def test_v061_changed_dictionary_invalidates_scan_and_prepared_plan(self):
        options = self.options(self.install_dictionary('1'))
        original = self.originals()
        profile, result = self.scan(options)
        stale = self.request(profile, result, options)
        updated = self.install_dictionary('2')
        self.call('review/prepare', stale, status=409)
        self.assertEqual(self.operation(stale)['status'], 'notRegistered')
        # An existing native settings file need not have an accepted Core
        # revision. Keep this supported observed-input branch in the same
        # focused case: content hashes alone cannot detect its replacement.
        observed = json.loads(updated['source'])
        observed['ownedExternalSettingsFixture'] = 'observed-without-accepted-revision'
        retained_json(self.root / '.weftext-settings.json', observed)
        newer = self.options(self.library_read('settings'))
        profile, result = self.scan(newer)
        request = self.request(profile, result, newer)
        plan = self.call('review/prepare', request)
        self.assertTrue(plan['commitAllowed'])
        path, before = self.replace_settings('after-prepare')
        self.assert_hidden_pending(self.operation(request))
        self.commit_review(plan, status=409)
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(self.originals(), original)
        self.assertFalse((self.root / 'Shared/weftext.annotations.json').exists())


def new_tests():
    return [name for name in sorted(vars(SharedProcessingIntegration))
            if name.startswith('test_v061_')]


def suite(only=()):
    return unittest.TestSuite(SharedProcessingIntegration(name) for name in new_tests()
        if not only or name in only or f'SharedProcessingIntegration.{name}' in only)


def load_tests(_loader, _tests, _pattern):
    return suite()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path, nargs='?')
    parser.add_argument('--executable', type=Path, dest='executable_option')
    parser.add_argument('--artifacts-dir', '--evidence-dir', dest='artifacts_dir', type=Path)
    parser.add_argument('--only', action='append', default=[], metavar='TEST')
    parser.add_argument('--list', action='store_true')
    args = parser.parse_args()
    allowed = set(new_tests()) | {f'SharedProcessingIntegration.{name}' for name in new_tests()}
    if any(name not in allowed for name in args.only):
        parser.error('Unknown --only name; use --list')
    if args.list:
        print('\n'.join(f'SharedProcessingIntegration.{name}' for name in new_tests()))
        return 0
    if bool(args.executable) == bool(args.executable_option):
        parser.error('Supply exactly one newly built executable, positional or --executable')
    global EXECUTABLE, ARTIFACT_PARENT
    EXECUTABLE = (args.executable or args.executable_option).resolve(strict=True)
    if not EXECUTABLE.is_file():
        parser.error('The executable must be a file')
    ARTIFACT_PARENT = args.artifacts_dir.resolve(strict=True) if args.artifacts_dir else None
    hosted_harness.EXE = EXECUTABLE
    result = unittest.TextTestRunner(verbosity=2).run(suite(args.only))
    if hasattr(SharedProcessingIntegration, 'evidence'):
        retained_json(SharedProcessingIntegration.evidence / 'results.json', {
            'executable': str(EXECUTABLE), 'testsRun': result.testsRun,
            'successful': result.wasSuccessful(),
            'failures': [{'test': str(test), 'traceback': trace} for test, trace in result.failures],
            'errors': [{'test': str(test), 'traceback': trace} for test, trace in result.errors]})
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    sys.exit(main())
