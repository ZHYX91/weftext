"""Real Server DOC conversions and original-ID recovery, without desktop input.

Usage: python scripts/test_hosted_legacy_office.py EXE NEW_EVIDENCE_DIRECTORY [NEW_WORK_DIRECTORY] [explicit|default|long]
The executable, library, policy, data, conversion processes and artifacts belong
to this one run. No Word/WPS automation or synthetic conversion receipt is used.
"""
from _creation_result import created_document
import base64
import hashlib
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import secrets
import sys
import unittest
import xml.etree.ElementTree as ET
import zipfile
from urllib.error import HTTPError
from urllib.request import Request, urlopen

import test_hosted
from test_hosted import uid, digest, Server as BaseServer
from test_hosted_office import HostedOffice
from windows_package import load_contract


EXE = Path(sys.argv[1]).resolve()
EVIDENCE = Path(sys.argv[2]).resolve()
RUN_ROOT = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else EVIDENCE
MODE = sys.argv[4] if len(sys.argv) > 4 else 'explicit'
HOST_PATHS = load_contract((Path(__file__).resolve().parents[1] / 'packaging/windows/host-contract.json').read_bytes())['paths']
if MODE not in {'explicit', 'default', 'long'}:
    raise ValueError('Mode must be explicit, default or long')
test_hosted.EXE = EXE


class Server(BaseServer):
    def call(self, route, data=None, token=None, headers=None):
        # The external converter has a 90 s product deadline. A first private
        # profile may build host caches; the client must wait for that bounded
        # receipt rather than interrupt it at the generic harness's 40 s.
        auth = {'Authorization': 'Bearer ' + (token or '')} if self.hosted else {'X-Weftext-Token': self.token}
        request = Request(self.url + route, data=None if data is None else json.dumps(data).encode(),
                          headers={'Content-Type': 'application/json', **auth, **(headers or {})})
        try:
            response = urlopen(request, timeout=130)
        except HTTPError as error:
            response = error
        with response:
            raw = response.read()
            value = json.loads(raw) if 'json' in response.headers.get('Content-Type', '') else raw
            return response.status, value, response.headers


def file_hash(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def visible_html(html):
    class Text(HTMLParser):
        def __init__(self):
            super().__init__()
            self.pieces = []

        def handle_data(self, data):
            self.pieces.append(data)
    parser = Text()
    parser.feed(html)
    return ''.join(parser.pieces)


class HostedLegacyOffice(HostedOffice):
    @classmethod
    def setUpClass(cls):
        if EVIDENCE.exists():
            raise RuntimeError('Evidence directory must be new; existing originals stay intact')
        EVIDENCE.mkdir(parents=True)
        if RUN_ROOT != EVIDENCE:
            if RUN_ROOT.exists():
                raise RuntimeError('Work directory must also be new; no original is overwritten')
            RUN_ROOT.mkdir(parents=True)
        cls.evidence = EVIDENCE
        cls.summary = {'executable': str(EXE), 'executableSha256': file_hash(EXE),
                       'actualExecution': 'owned Windows Server process; its private LibreOffice profile',
                       'workDirectory': str(RUN_ROOT), 'dataDirectoryMode': MODE,
                       'wordOrWpsStarted': False, 'cases': []}
        if MODE == 'default':
            # Exercise the actual default data-dir resolver while keeping all
            # writes in this new owned fixture, away from a real user's state.
            os.environ['LOCALAPPDATA'] = str(RUN_ROOT)

    @classmethod
    def tearDownClass(cls):
        (EVIDENCE / 'summary.json').write_text(json.dumps(cls.summary, ensure_ascii=False, indent=2), 'utf-8')

    def setUp(self):
        # Keep this Windows fixture short: CreateProcess's cwd does not accept
        # arbitrary extended-length paths even when file IO supports them.
        self.folder = EVIDENCE
        self.data = RUN_ROOT / HOST_PATHS['persistentRelativeRoot'] / HOST_PATHS['coreRelativePath'] if MODE == 'default' else RUN_ROOT / 's'
        self.root = RUN_ROOT / 'l'
        local = Server([ *self.data_args()], expected_default_data_dir=self.data if MODE == 'default' else None,
                       evidence_parent=RUN_ROOT if MODE == 'default' else None)
        try:
            self.assertEqual(local.call('/api/library/create', {'path': str(self.root), 'operationId': uid()})[0], 200)
            code, doc, _ = local.call('/api/note/create', {'root': str(self.root), 'parent': str(self.root), 'name': 'Shared', 'operationId': uid()})
            self.assertEqual(code, 200, doc)
            doc = created_document(doc)
            code, _, _ = local.call('/api/save', {'sessionId': doc['sessionId'], 'baseId': doc['baseId'], 'source': doc['source'] + '初始共同正文', 'operationId': uid()})
            self.assertEqual(code, 200)
        finally:
            local.close()
        self.tokens = {actor: secrets.token_urlsafe(32) for actor in ['alice', 'bob', 'admin']}
        rights = {'read': True, 'write': True, 'history': True, 'download': True}
        self.config = {'deployment': 'legacy-http-acceptance', 'bind': '127.0.0.1:0', 'root': str(self.root),
                       'adminTokenSha256': digest(self.tokens['admin']),
                       'files': {'Shared': {'path': 'Shared/Shared.typ', 'title': 'Shared', 'editable': True}},
                       'users': {actor: {'enabled': True, 'tokenSha256': digest(self.tokens[actor]), 'grants': {'Shared': rights.copy()}}
                                 for actor in ['alice', 'bob']}}
        self.config_path = EVIDENCE / 'server.json'
        self.start()

    def data_args(self):
        return [] if MODE == 'default' else ['--data-dir', str(self.data)]

    def start(self):
        self.config_path.write_text(json.dumps(self.config), 'utf-8')
        self.server = Server([ *self.data_args(), '--server-config', str(self.config_path)], hosted=True,
                             expected_default_data_dir=self.data if MODE == 'default' else None,
                             evidence_parent=RUN_ROOT if MODE == 'default' else None)

    def call(self, route, data=None, actor='alice', status=200, headers=None):
        result = super().call(route, data, actor, status, headers)
        # HTTP bodies exposed to an actor cannot reveal deployment disk paths.
        encoded = json.dumps(result, ensure_ascii=False) if not isinstance(result, bytes) else ''
        self.assertNotIn(str(self.root), encoded)
        self.assertNotIn(str(self.data), encoded)
        return result

    def receipt_files(self):
        root = self.data / 'legacy-office'
        return {p.relative_to(root).as_posix(): {'sha256': file_hash(p), 'mtimeNs': p.stat().st_mtime_ns}
                for p in root.rglob('completed.json')}

    def restart(self):
        self.server.close()
        self.start()

    def test_real_doc_http_export_import_original_recovery_and_revocation(self):
        self.server.close()
        self.config['files']['ImportedDoc'] = dict(path='ImportedDoc/document/document.typ', title='DOC import', editable=True, importable=True)
        self.start()

        def grants(users):
            users['alice']['grants']['Shared']['office'] = {'export': True, 'download': True}
            users['bob']['grants']['Shared']['office'] = {'export': True, 'download': True}
            users['alice']['grants']['ImportedDoc'] = {'read': True, 'write': True, 'office': {'doc': {'import': True}}}
        self.policy(grants)
        source = self.open()['source'] + '\n\nREAL_HOSTED_DOC_BODY 中文 English 20261018\n'
        request = dict(requestId=uid(), resource='Shared', source=source, format='doc',
                       reviewMode='none', includeResolved=False, stylePolicy='template', bindings={}, legacyBackend='libreoffice')
        self.call('office/export/prepare', request, status=404)
        self.call('office/export/prepare', request, 'bob', 404)
        # Ordinary DOCX remains usable without any old-format grant.
        ordinary, _ = self.output({**request, 'requestId': uid(), 'format': 'docx', 'legacyBackend': None}, 'bob')
        self.assertEqual(ordinary['status'], 'prepared')
        self.assertTrue(self.deliver(ordinary, 'bob').startswith(b'PK'))
        self.policy(lambda users: users['alice']['grants']['Shared']['office'].update(
            doc={'export': True, 'download': True}, wps={'export': True, 'download': True}))
        capabilities = self.call('office/legacy-capabilities')
        for backend in capabilities['backends']:
            for direction in backend['directions']:
                if direction['direction'] in ['wpsToDocx', 'docxToWps']:
                    self.assertFalse(direction['available'])
        self.summary['capabilities'] = capabilities
        (self.folder / 'capabilities.json').write_text(json.dumps(capabilities, ensure_ascii=False, indent=2), 'utf-8')

        prepared = self.call('office/export/prepare', request)
        self.assertEqual(prepared['status'], 'prepared', prepared)
        self.assertTrue(prepared['filename'].endswith('.doc'))
        self.assertEqual(prepared['contentType'], 'application/msword')
        original_receipts = self.receipt_files()
        self.assertEqual(len(original_receipts), 1)
        self.restart()
        recovered = self.call('office/export/status', {'requestId': request['requestId']})
        self.assertEqual(recovered['status'], 'prepared', recovered)
        for key in ['digest', 'length', 'filename', 'contentType']:
            self.assertEqual(recovered[key], prepared[key])
        self.assertEqual(self.receipt_files(), original_receipts)
        self.assertEqual(self.call('office/export/prepare', request)['digest'], prepared['digest'])
        self.call('office/export/prepare', {**request, 'source': source + ' changed'}, status=409)
        if recovered['losses']:
            self.call('office/export/confirm', {'requestId': request['requestId'], 'digest': prepared['digest'], 'acceptLosses': False}, status=400)
        artifact = self.deliver(recovered)
        self.assertEqual(artifact[:8], bytes.fromhex('d0cf11e0a1b11ae1'))
        self.assertFalse(zipfile.is_zipfile(self.folder / (prepared['requestId'] + '.doc')))
        self.restart()
        confirmed = self.call('office/export/status', {'requestId': request['requestId']})
        self.assertEqual(confirmed['status'], 'confirmed', confirmed)
        self.assertEqual(self.deliver(confirmed), artifact)
        self.assertEqual(self.receipt_files(), original_receipts)
        self.summary['cases'].append({'case': 'actual DOC export prepare/restart/confirm/download/restart/download',
                                     'requestId': request['requestId'], 'length': len(artifact),
                                     'sha256': hashlib.sha256(artifact).hexdigest(), 'signature': artifact[:8].hex(),
                                     'noExternalRerun': True, 'receiptFiles': original_receipts, 'losses': prepared['losses'],
                                     'legacyConversion': prepared.get('preview', {}).get('legacyConversion', prepared.get('preview', {}).get('legacy'))})

        upload = dict(resource='ImportedDoc', operationId=uid(), format='doc', legacyBackend='libreoffice',
                      bytesBase64=base64.b64encode(artifact).decode())
        self.call('office/import/prepare', upload, 'bob', 404)
        self.call('office/import/inspect', {'resource': 'ImportedDoc', 'format': 'docx', 'bytesBase64': ordinary_bytes(artifact)}, status=404)
        slots = self.call('office/import-targets')['targets']
        imported_slot = next(slot for slot in slots if slot['resource'] == 'ImportedDoc')
        self.assertEqual(imported_slot['formats'], ['doc'])
        plan = self.call('office/import/prepare', upload)
        self.assertEqual(plan['status'], 'prepared', plan)
        self.assertFalse((self.root / 'ImportedDoc').exists())
        all_receipts = self.receipt_files()
        self.assertEqual(len(all_receipts), 2)
        self.restart()
        resumed = self.call('office/import/prepare', upload)
        self.assertEqual(resumed['planId'], plan['planId'])
        self.assertEqual(resumed['preview'], plan['preview'])
        self.assertEqual(self.receipt_files(), all_receipts)
        self.call('office/import/prepare', {**upload, 'bytesBase64': base64.b64encode(artifact + b' changed').decode()}, status=409)
        commit = dict(resource='ImportedDoc', operationId=upload['operationId'], planId=plan['planId'], acceptLosses=True)
        if plan['preview'].get('losses'):
            self.call('office/import/commit', {**commit, 'acceptLosses': False}, status=409)
        self.assertEqual(self.call('office/import/commit', commit)['status'], 'committed')
        imported = self.open('ImportedDoc')
        source_after = imported['source']
        self.assertFalse(any(line.lstrip().startswith('- ') for line in source_after.splitlines()),
                         'These ordinary paragraphs must not become list items through LibreOffice numId=0')
        rendered = self.call('preview', {'sessionId': imported['sessionId'], 'source': source_after})
        self.assertTrue(rendered['ok'], rendered)
        native_text = visible_html(rendered['html'])
        self.assertIn('REAL_HOSTED_DOC_BODY', native_text)
        self.assertIn('中文', native_text)
        retained_doc = list((self.root / 'ImportedDoc').rglob('*.doc'))
        retained_docx = list((self.root / 'ImportedDoc').rglob('*.docx'))
        self.assertEqual(len(retained_doc), 1)
        self.assertEqual(retained_doc[0].read_bytes(), artifact)
        self.assertEqual(len(retained_docx), 1)
        self.assertTrue(zipfile.is_zipfile(retained_docx[0]))
        with zipfile.ZipFile(retained_docx[0]) as intermediate:
            body = ET.fromstring(intermediate.read('word/document.xml'))
        intermediate_text = ''.join(body.itertext())
        self.assertIn('REAL_HOSTED_DOC_BODY', intermediate_text)
        self.assertIn('中文', intermediate_text)
        manifest = json.loads((self.root / 'ImportedDoc/document/weftext.resources.json').read_text('utf-8'))
        registered = {entry['path'] for entry in manifest['resources']}
        self.assertIn(retained_doc[0].relative_to(self.root / 'ImportedDoc/document').as_posix(), registered)
        self.assertIn(retained_docx[0].relative_to(self.root / 'ImportedDoc/document').as_posix(), registered)
        self.restart()
        self.assertEqual(self.call('office/import/operation', {'resource': 'ImportedDoc', 'operationId': upload['operationId']})['status'], 'committed')
        self.assertEqual(self.call('office/import/commit', commit)['status'], 'committed')
        self.assertEqual(self.receipt_files(), all_receipts)
        self.summary['cases'].append({'case': 'actual DOC upload/convert/plan/restart/exact plan/commit/restart',
                                     'operationId': upload['operationId'], 'planId': plan['planId'], 'noExternalRerun': True,
                                     'originalDoc': {'relative': retained_doc[0].relative_to(self.root).as_posix(), 'sha256': file_hash(retained_doc[0])},
                                     'intermediateDocx': {'relative': retained_docx[0].relative_to(self.root).as_posix(), 'sha256': file_hash(retained_docx[0])},
                                     'losses': plan['preview'].get('losses'), 'sourceContainsExactFixtureText': True,
                                     'intermediateBodyText': intermediate_text, 'nativeRenderedText': native_text,
                                     'nativeSourceSha256': hashlib.sha256(source_after.encode()).hexdigest(),
                                     'ordinaryParagraphsDidNotBecomeLists': True, 'originalAndIntermediateRegistered': True})

        # Removing only this format's delivery grant denies every old chunk,
        # including after restart; source reads remain independently usable.
        self.policy(lambda users: users['alice']['grants']['Shared']['office']['doc'].update(download=False))
        denied = self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, status=409)
        self.assertNotIn('bytesBase64', denied)
        expired = self.call('office/export/status', {'requestId': request['requestId']})
        self.assertEqual(expired['status'], 'expired')
        self.assertNotIn('digest', expired)
        self.assertTrue(self.open()['source'])
        self.restart()
        denied_again = self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, status=409)
        self.assertNotIn('bytesBase64', denied_again)
        wps = self.call('office/export/prepare', {**request, 'requestId': uid(), 'format': 'wps'})
        self.assertEqual(wps['status'], 'failed')
        self.assertEqual(wps['preview']['legacy']['status'], 'unavailable')
        self.assertIsNone(wps['preview']['legacy']['generated'])
        self.assertNotIn('digest', wps)
        self.summary['cases'].append({'case': 'independent DOC grants, revoke download and restart delivery denial; WPS unavailable',
                                     'revokedStatus': expired['status'], 'deniedBytesDelivered': False, 'ordinaryDocxWithoutDocGrant': True,
                                     'docImportDidNotGrantDocxInspect': True, 'wpsNativeSuccessClaimed': False})

    def test_long_data_path_is_refused_before_start_and_original_result_is_recoverable(self):
        self.policy(lambda users: users['alice']['grants']['Shared'].update(
            office={'export': True, 'download': True, 'doc': {'export': True, 'download': True}}))
        source = self.open()['source'] + '\nLONG_PATH_EXACT_ORIGINAL\n'
        request = dict(requestId=uid(), resource='Shared', source=source, format='doc',
                       reviewMode='none', includeResolved=False, stylePolicy='template', bindings={}, legacyBackend='libreoffice')
        refused = self.call('office/export/prepare', request)
        self.assertEqual(refused['status'], 'failed', refused)
        conversion = refused['preview']['legacy']
        self.assertEqual(conversion['status'], 'unavailable')
        self.assertEqual(conversion['code'], 'executionPathTooLong')
        self.assertIsNone(conversion['backendVersion'])
        receipts = self.receipt_files()
        self.assertEqual(len(receipts), 1)
        originals = list((self.data / 'legacy-office' / 'runs').rglob('original.docx'))
        self.assertTrue(originals)
        self.assertTrue(all(zipfile.is_zipfile(original) for original in originals))
        self.assertEqual(list((self.data / 'legacy-office' / 'runs').rglob('profile')), [])
        self.assertEqual(list((self.data / 'legacy-office' / 'runs').rglob('converter.log')), [])
        self.restart()
        recovered = self.call('office/export/prepare', request)
        self.assertEqual(recovered['status'], 'failed')
        self.assertEqual(recovered['preview']['legacy']['code'], 'executionPathTooLong')
        self.assertEqual(self.receipt_files(), receipts)
        ordinary, _ = self.output({**request, 'requestId': uid(), 'format': 'docx', 'legacyBackend': None})
        self.assertEqual(ordinary['status'], 'prepared')
        self.assertTrue(self.deliver(ordinary).startswith(b'PK'))
        self.summary['cases'].append({'case': 'long data path explicit preflight refusal; no conversion profile/log; exact original and same-ID receipt retained',
                                     'requestId': request['requestId'], 'code': 'executionPathTooLong',
                                     'ordinaryDocxUsable': True, 'noExternalRerun': True,
                                     'retainedOriginals': [{'relative': path.relative_to(self.data).as_posix(), 'sha256': file_hash(path)} for path in originals]})


def ordinary_bytes(data):
    return base64.b64encode(data).decode()


if __name__ == '__main__':
    method = 'test_long_data_path_is_refused_before_start_and_original_result_is_recoverable' if MODE == 'long' else 'test_real_doc_http_export_import_original_recovery_and_revocation'
    suite = unittest.TestSuite([HostedLegacyOffice(method)])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if 'summary' in HostedLegacyOffice.__dict__:
        marker = EVIDENCE / 'summary.json'
        summary = json.loads(marker.read_text('utf-8')) if marker.exists() else {}
        summary['passed'] = result.wasSuccessful()
        summary['testsRun'] = result.testsRun
        summary['failures'] = len(result.failures)
        summary['errors'] = len(result.errors)
        marker.write_text(json.dumps(summary, ensure_ascii=False, indent=2), 'utf-8')
    sys.exit(not result.wasSuccessful())
