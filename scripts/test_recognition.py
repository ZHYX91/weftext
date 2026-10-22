"""Recognition HTTP boundaries over newly generated, retained TEST material.

Usage: python scripts/test_recognition.py CANDIDATE.exe --artifacts-dir OWNED_DIR
Only the tests declared here run. Retention, request and recovery checks require
no models. Set WEFTEXT_RECOGNITION_RUNTIME explicitly for the additional real
text-PDF / DOCX worker checks; absent runtime cases are reported as skipped.
This entry does not establish browser, invoice quality, Office or OS IME passes.
"""
from _creation_result import created_document
import argparse
import base64
from contextlib import contextmanager
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import signal
import struct
import sys
import tempfile
import time
import unittest
import zipfile

import test_hosted as http
from test_hosted_content_import import HostedContentImport
from test_local_features import tiny_png


ARTIFACTS = None
RUNTIME = None
OFFICIAL = 'official-document-cn-mainland-hans@1'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def text_pdf(pages):
    """A self-contained ASCII TEST PDF, without a renderer or font package."""
    objects = [b'', b'']
    font = 3
    objects.append(b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>')
    kids = []
    for lines in pages:
        page_id = len(objects) + 1
        stream_id = page_id + 1
        kids.append(f'{page_id} 0 R')
        objects.append((f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 400] '
                        f'/Resources << /Font << /F1 {font} 0 R >> >> '
                        f'/Contents {stream_id} 0 R >>').encode())
        commands = ['BT /F1 18 Tf']
        for index, line in enumerate(lines):
            escaped = line.replace('\\', '\\\\').replace('(', '\\(').replace(')', '\\)')
            commands.append(f'1 0 0 1 30 {350-index*32} Tm ({escaped}) Tj')
        stream = ('\n'.join(commands) + '\nET\n').encode('ascii')
        objects.append(f'<< /Length {len(stream)} >>\nstream\n'.encode() + stream + b'endstream')
    objects[0] = b'<< /Type /Catalog /Pages 2 0 R >>'
    objects[1] = f'<< /Type /Pages /Count {len(kids)} /Kids [{" ".join(kids)}] >>'.encode()
    result = bytearray(b'%PDF-1.7\n% TEST fixture\n')
    positions = [0]
    for index, value in enumerate(objects, 1):
        positions.append(len(result))
        result.extend(f'{index} 0 obj\n'.encode() + value + b'\nendobj\n')
    xref = len(result)
    result.extend(f'xref\n0 {len(positions)}\n0000000000 65535 f \n'.encode())
    for position in positions[1:]:
        result.extend(f'{position:010d} 00000 n \n'.encode())
    result.extend(f'trailer\n<< /Size {len(positions)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode())
    return bytes(result)


def gray_jpeg():
    """Generate one 8-bit grayscale pixel with a minimal baseline JPEG stream."""
    def marker(code, payload):
        return bytes([255, code]) + struct.pack('>H', len(payload) + 2) + payload
    huffman = bytes([0]) + bytes([1] + [0] * 15) + bytes([0])
    huffman += bytes([16]) + bytes([1] + [0] * 15) + bytes([0])
    return (b'\xff\xd8' + marker(224, b'JFIF\0\x01\x01\0\0\x01\0\x01\0\0')
            + marker(219, bytes([0]) + bytes([1] * 64))
            + marker(192, b'\x08\0\x01\0\x01\x01\x01\x11\0')
            + marker(196, huffman) + marker(218, b'\x01\x01\0\0\x3f\0')
            + b'\x3f\xff\xd9')


def official_docx():
    """Actual OOXML paragraphs/styles; no client-supplied structure assertions."""
    namespace = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
    relationships = 'http://schemas.openxmlformats.org/package/2006/relationships'
    office = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/'
    parts = {
        '[Content_Types].xml': '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>',
        '_rels/.rels': f'<Relationships xmlns="{relationships}"><Relationship Id="r1" Type="{office}officeDocument" Target="word/document.xml"/></Relationships>',
        'word/_rels/document.xml.rels': f'<Relationships xmlns="{relationships}"><Relationship Id="s1" Type="{office}styles" Target="styles.xml"/></Relationships>',
        'word/styles.xml': f'<w:styles xmlns:w="{namespace}"><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style></w:styles>',
        'word/document.xml': f'<w:document xmlns:w="{namespace}"><w:body><w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>TEST 资料导入通知</w:t></w:r></w:p><w:p><w:r><w:t>发文机关：TEST OFFICE</w:t></w:r></w:p><w:p><w:r><w:t>成文日期：2026年10月19日</w:t></w:r></w:p><w:p><w:r><w:t>主送机关：TEST TEAM</w:t></w:r></w:p></w:body></w:document>',
    }
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        for name, value in parts.items():
            archive.writestr(name, value.encode('utf-8'))
    return output.getvalue()


@contextmanager
def runtime_environment(value):
    key = 'WEFTEXT_RECOGNITION_RUNTIME'
    previous = os.environ.get(key)
    if value is None:
        os.environ.pop(key, None)
    else:
        os.environ[key] = str(value)
    try:
        yield
    finally:
        if previous is None:
            os.environ.pop(key, None)
        else:
            os.environ[key] = previous


def request(target, payload, format='pdf', purpose='resource', **options):
    return {'jobId': http.uid(), 'target': target, 'format': format,
            'bytesBase64': base64.b64encode(payload).decode(),
            'options': {'purpose': purpose, 'languages': ['eng'], 'pages': [{'page': 1}],
                        'dpi': 144, 'scheme': 'none', **options}}


def review(job, purpose='resource'):
    result = job.get('result', {})
    blocks = result.get('docxBlocks', []) or [block for page in result.get('pages', [])
             for region in page.get('regions', []) for block in region.get('blocks', [])]
    return {'purpose': purpose, 'title': 'TEST recognition result',
            'blocks': [{'id': block['id'], 'text': block['text']} for block in blocks],
            'fields': [], 'acceptPartial': False}


def private_root(data):
    path = (data / 'recognition').resolve()
    return Path('\\\\?\\' + str(path)) if os.name == 'nt' else path


def stop_owned_worker(parent_pid):
    """Stop only the Python child of the server created by this test."""
    if os.name == 'nt':
        import ctypes
        from ctypes import wintypes

        class ProcessEntry(ctypes.Structure):
            _fields_ = [('size', wintypes.DWORD), ('usage', wintypes.DWORD),
                        ('pid', wintypes.DWORD), ('heap', ctypes.c_size_t),
                        ('module', wintypes.DWORD), ('threads', wintypes.DWORD),
                        ('parent', wintypes.DWORD), ('priority', wintypes.LONG),
                        ('flags', wintypes.DWORD), ('exe', wintypes.WCHAR * 260)]

        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
        kernel.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
        kernel.Process32FirstW.argtypes = [wintypes.HANDLE, ctypes.POINTER(ProcessEntry)]
        kernel.Process32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(ProcessEntry)]
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        kernel.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        snapshot = kernel.CreateToolhelp32Snapshot(2, 0)
        if snapshot == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        children = []
        try:
            entry = ProcessEntry()
            entry.size = ctypes.sizeof(entry)
            more = kernel.Process32FirstW(snapshot, ctypes.byref(entry))
            while more:
                if entry.parent == parent_pid and entry.exe.lower() == 'python.exe':
                    children.append(entry.pid)
                more = kernel.Process32NextW(snapshot, ctypes.byref(entry))
        finally:
            kernel.CloseHandle(snapshot)
        if len(children) != 1:
            raise AssertionError(f'Expected one owned Python child, got {children}')
        process = kernel.OpenProcess(0x1001, False, children[0])
        if not process:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            image = ctypes.create_unicode_buffer(32768)
            count = wintypes.DWORD(len(image))
            if not kernel.QueryFullProcessImageNameW(process, 0, image, ctypes.byref(count)):
                raise ctypes.WinError(ctypes.get_last_error())
            if Path(image.value).resolve() != (Path(RUNTIME) / 'venv/Scripts/python.exe').resolve():
                raise AssertionError('Owned child executable differs from the explicit runtime: ' + image.value)
            if not kernel.TerminateProcess(process, 7):
                raise ctypes.WinError(ctypes.get_last_error())
            return {'parentPid': parent_pid, 'workerPid': children[0], 'image': image.value}
        finally:
            kernel.CloseHandle(process)
    children = []
    for process in Path('/proc').iterdir():
        if not process.name.isdigit():
            continue
        try:
            parent = int((process / 'stat').read_text().rsplit(')', 1)[1].split()[1])
            command = (process / 'cmdline').read_bytes().split(b'\0')
            if parent == parent_pid and any(value.endswith(b'/product_worker.py') for value in command):
                children.append(int(process.name))
        except (OSError, ValueError, IndexError):
            continue
    if len(children) != 1:
        raise AssertionError(f'Expected one owned product worker, got {children}')
    os.kill(children[0], signal.SIGKILL)
    return {'parentPid': parent_pid, 'workerPid': children[0]}


class RecognitionChecks:
    def private_job(self, job_id):
        matches = list(private_root(self.data).glob('*/' + job_id + '/claim.json'))
        self.assertEqual(len(matches), 1)
        return matches[0].parent

    def assert_download(self, delivered, req, job):
        content = base64.b64decode(delivered['dataBase64'], validate=True)
        self.assertEqual(delivered['jobId'], req['jobId'])
        self.assertEqual(delivered['mediaType'], 'text/plain')
        self.assertEqual(delivered['sha256'], sha(content))
        companion = delivered['provenance']
        self.assertEqual(companion['fileName'], 'recognized-provenance.json')
        self.assertEqual(companion['mediaType'], 'application/json')
        raw = base64.b64decode(companion['dataBase64'], validate=True)
        self.assertEqual(companion['sha256'], sha(raw))
        metadata = json.loads(raw)
        self.assertEqual(metadata['authority'], 'readOnlyRecognitionProvenance')
        self.assertEqual(metadata['text'], 'rawRecognizedTextBeforeAuthorReview')
        self.assertEqual(metadata['textSha256'], delivered['sha256'])
        self.assertEqual(metadata['inputSha256'], sha(base64.b64decode(req['bytesBase64'])))
        self.assertEqual(metadata['inputSha256'], job['inputSha256'])
        self.assertEqual(metadata['options'], req['options'])
        self.assertEqual(metadata['optionsDigest'], job['optionsDigest'])
        self.assertEqual(metadata['resultDigest'], job['resultDigest'])
        claim = json.loads((self.private_job(req['jobId']) / 'claim.json').read_bytes())
        for key in ('implementationDigest', 'runtimeDigest', 'optionsDigest'):
            self.assertEqual(metadata[key], claim[key])
            self.assertRegex(metadata[key], '^[a-f0-9]{64}$')
        result = job['result']
        self.assertEqual(metadata['sourceScope'], 'selectedPagesAndRegions')
        self.assertEqual(metadata['status'], result['status'])
        self.assertEqual(metadata['pageCount'], result['pageCount'])
        self.assertEqual(metadata['selection'], result['selection'])
        self.assertEqual(metadata['runtime'], result['runtime'])
        self.assertEqual(metadata['losses'], result['losses'])
        self.assertEqual(metadata['pages'], [
            {'page': page['page'], 'status': page['status'], 'losses': page['losses'],
             'regions': [{key: region[key] for key in ('id', 'rect', 'method', 'status', 'losses')}
                         for region in page['regions']]}
            for page in result['pages']])
        self.assertEqual(metadata['structureRecovery'], 'basicRuledRectangularTablesOnly')
        return content

    def wait_job(self, route, started):
        deadline = time.monotonic() + 80
        value = started
        while value['status'] in ('pending', 'running') and time.monotonic() < deadline:
            time.sleep(.15)
            value = route('status', {'jobId': started['jobId']})
        self.assertNotIn(value['status'], ('pending', 'running'), value)
        return value

    def assert_tree(self, destination, payload, receipt):
        evidence_paths = list(destination.rglob('recognition-evidence.json'))
        self.assertEqual(len(evidence_paths), 1)
        evidence = json.loads(evidence_paths[0].read_bytes())
        self.assertEqual(evidence['inputSha256'], sha(payload))
        self.assertEqual(evidence['authority'], 'historicalAcceptedAtImportOnly')
        registry = json.loads((evidence_paths[0].parent / 'weftext.resources.json').read_bytes())
        originals = [row for row in registry['resources'] if row['path'].startswith('original.')]
        self.assertEqual(len(originals), 1)
        self.assertEqual((evidence_paths[0].parent / originals[0]['path']).read_bytes(), payload)
        self.assertEqual(evidence['originalResource'],
                         f'weftext-resource://{receipt["nodeRefs"][0]["workspaceId"]}/{originals[0]["resourceId"]}')
        self.assertEqual(len(list(destination.rglob('*.typ'))), 1)
        source = next(destination.rglob('document.typ'))
        self.assertIn(evidence['originalResource'], source.read_text('utf-8'))
        self.assertNotIn(destination.as_uri(), source.read_text('utf-8'))
        return source


class LocalRecognition(RecognitionChecks, unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.artifacts = Path(tempfile.mkdtemp(prefix='weftext-recognition-local-', dir=ARTIFACTS))
        print('Local recognition TEST artifacts:', cls.artifacts, flush=True)

    def setUp(self):
        needs_runtime = '_runtime_' in self._testMethodName
        if needs_runtime and not RUNTIME:
            self.skipTest('Set WEFTEXT_RECOGNITION_RUNTIME for the real worker cases')
        self.enterContext(runtime_environment(RUNTIME if needs_runtime else None))
        self.folder = self.artifacts / self._testMethodName.removeprefix('test_')
        self.folder.mkdir()
        self.root, self.data = self.folder / 'TEST-library', self.folder / 'state'
        self.server = http.Server([ '--data-dir', str(self.data)])
        self.addCleanup(lambda: self.server.close())
        self.api('/api/library/create', {'path': str(self.root), 'operationId': http.uid()})
        self.original = created_document(self.api('/api/note/create', {'root': str(self.root), 'parent': str(self.root), 'name': 'TEST-origin', 'operationId': http.uid()}))
        config = self.api('/api/data/schema', {'root': str(self.root)})
        plan = self.api('/api/data/schema/prepare', {'root': str(self.root), 'source': config['source'], 'beforeSource': config['source'], 'beforeRevision': config['revision'], 'operationId': http.uid()})
        self.api('/api/data/schema/install', {'operationId': plan['operationId'], 'planId': plan['planId']})

    def api(self, route, body=None, status=200):
        code, result, _ = self.server.call(route, body)
        logged = result.decode('utf-8', errors='replace') if isinstance(result, bytes) else result
        with (self.folder / 'http.ndjson').open('a', encoding='utf-8') as log:
            log.write(json.dumps({'route': route, 'status': code, 'response': logged}, ensure_ascii=False) + '\n')
        self.assertEqual(code, status, (route, result))
        return result

    def recognition(self, route, body=None, status=200):
        return self.api('/api/recognition/' + route, body, status)

    def target(self, name):
        return {'sessionId': self.original['sessionId'], 'parent': str(self.root), 'name': name}

    def commit(self, target, plan, accept=True, status=200):
        return self.recognition('commit', {'target': target, 'operationId': plan['operationId'], 'planId': plan['planId'], 'acceptLosses': accept}, status)

    def observe_checkpoint(self, req, label):
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            paths = [path for path in private_root(self.data).rglob('progress.json')
                     if path.parent.name == req['jobId']]
            if paths:
                snapshot = json.loads(paths[0].read_bytes())
                if 0 < len(snapshot['result']['pages']) < len(req['options']['pages']):
                    (self.folder / (label + '.json')).write_text(json.dumps(snapshot, ensure_ascii=False, indent=2), 'utf-8')
                    return paths[0].parent
            time.sleep(.025)
        self.fail('Did not observe a real intermediate page checkpoint')

    def assert_interrupted(self, recovered, req):
        self.assertEqual(recovered['status'], 'interrupted')
        result = recovered['result']
        self.assertEqual(result['status'], 'partial')
        self.assertFalse(result['workerFinalValidation'])
        self.assertEqual(result['pageCount'], 20)
        self.assertEqual({page['page'] for page in result['pages']}, set(range(1, 21)))
        self.assertIn('host-execution-interrupted', result['losses'])
        completed = [page for page in result['pages'] if page['status'] == 'complete']
        self.assertTrue(completed)
        self.assertTrue(any('TEST PAGE 1' in block['text'] for page in completed
                            for region in page['regions'] for block in region['blocks']))
        missing = [page for page in result['pages'] if page['status'] == 'failed']
        self.assertTrue(missing)
        self.assertTrue(all('unprocessed-after-interruption' in page['losses'] for page in missing))
        for accept in (False, True):
            selected = {**review(recovered, 'note'), 'acceptPartial': accept}
            operation = http.uid()
            self.recognition('prepare', {'target': req['target'], 'jobId': req['jobId'], 'operationId': operation, 'review': selected}, 409)
            self.recognition('cancel-import', {'target': req['target'], 'operationId': operation})
        self.assertFalse((self.root / req['target']['name']).exists())

    def test_local_retention_all_formats_without_models(self):
        capabilities = self.recognition('capabilities')
        self.assertFalse(capabilities['available'])
        self.assertEqual(capabilities['models'], [])
        for format, payload in [('pdf', text_pdf([['TEST retained PDF']])), ('png', tiny_png()), ('jpeg', gray_jpeg())]:
            with self.subTest(format=format):
                (self.folder / ('TEST-original.' + format)).write_bytes(payload)
                target = self.target('TEST-' + format)
                req = request(target, payload, format)
                job = self.wait_job(self.recognition, self.recognition('start', req))
                self.assertEqual(job['status'], 'complete')
                self.assertTrue(job['result']['retentionOnly'])
                self.assertEqual(job['inputSha256'], sha(payload))
                operation = http.uid()
                plan = self.recognition('prepare', {'target': target, 'jobId': req['jobId'], 'operationId': operation, 'review': review(job)})
                self.assertEqual(plan['status'], 'prepared')
                self.assertFalse((self.root / target['name']).exists())
                self.commit(target, plan, accept=False, status=409)
                receipt = self.commit(target, plan)
                self.assertEqual(receipt['status'], 'committed')
                self.assertEqual(self.commit(target, plan)['nodeRefs'], receipt['nodeRefs'])
                source = self.assert_tree(self.root / target['name'], payload, receipt)
                original_member = {'target': target, 'operationId': operation, 'node': receipt['nodeRefs'][0]}
                opened = self.recognition('open', original_member)
                self.assertEqual(opened['nodeId'], receipt['nodeRefs'][0]['nodeId'])
                if format == 'pdf':
                    original_source = source.read_bytes()
                    physical = (source.stat().st_dev, source.stat().st_ino)
                    parent = created_document(self.api('/api/note/create', {'root': str(self.root), 'parent': str(self.root), 'name': 'TEST-delivery-parent', 'operationId': http.uid()}))
                    move = self.api('/api/operations/prepare', {'root': str(self.root), 'operationId': http.uid(), 'intent': {'kind': 'moveNode', 'source': str(source.parent), 'parent': str(Path(parent['path']).parent)}})
                    self.api('/api/operations/commit', {'operationId': move['operationId'], 'planId': move['planId']})
                    y = created_document(self.api('/api/note/create', {'root': str(self.root), 'parent': str(source.parent.parent), 'name': source.parent.name, 'operationId': http.uid()}))
                    moved = self.recognition('open', original_member)
                    current = Path(moved['path'])
                    self.assertEqual(moved['nodeId'], opened['nodeId'])
                    self.assertNotEqual(moved['nodeId'], y['nodeId'])
                    self.assertEqual((current.stat().st_dev, current.stat().st_ino), physical)
                    self.assertEqual(current.read_bytes(), original_source)
                    self.recognition('open', {**original_member, 'node': {'workspaceId': y['workspaceId'], 'nodeId': y['nodeId']}}, 409)
                    journal = {p.name: p.read_bytes() for p in (self.data / 'journal').iterdir() if p.is_file()}
                    retained = self.folder / 'TEST-retained-recognition-original.typ'
                    current.rename(retained); current.write_bytes(original_source)
                    try:
                        self.recognition('open', original_member, 409)
                        self.assertEqual(self.recognition('operation', {'target': target, 'operationId': operation})['status'], 'committed')
                        self.assertEqual({p.name: p.read_bytes() for p in (self.data / 'journal').iterdir() if p.is_file()}, journal)
                        self.assertEqual(retained.read_bytes(), original_source)
                    finally:
                        current.rename(self.folder / 'TEST-retained-recognition-substitute.typ')
                        retained.rename(current)
                    self.assertEqual(self.recognition('open', original_member)['nodeId'], opened['nodeId'])
        self.assertEqual(Path(self.original['path']).read_text('utf-8'), self.original['source'])

    def test_local_closed_requests_and_frozen_job_identity(self):
        target = self.target('TEST-requests')
        payload = tiny_png()
        req = request(target, payload, 'png')
        first = self.wait_job(self.recognition, self.recognition('start', req))
        self.assertEqual(self.recognition('start', req)['resultDigest'], first['resultDigest'])
        for variant in [dict(bytesBase64=base64.b64encode(payload + b'TEST').decode()),
                        dict(target=self.target('TEST-other')), dict(options={**req['options'], 'dpi': 72})]:
            self.recognition('start', {**req, **variant}, 409)
        self.recognition('start', {**req, 'root': str(self.root)}, 422)
        self.recognition('start', {**req, 'jobId': http.uid(), 'target': {**target, 'inputPath': str(self.root)}}, 422)
        self.recognition('start', {**req, 'jobId': http.uid(), 'options': {**req['options'], 'command': 'TEST'}}, 400)
        self.recognition('start', {**req, 'jobId': http.uid(), 'bytesBase64': '!'}, 400)
        self.recognition('start', request(target, payload, 'png', 'note'), 409)
        self.assertFalse((self.root / target['name']).exists())

    def test_local_cancel_and_original_operation_restart(self):
        target = self.target('TEST-recovery')
        req = request(target, text_pdf([['TEST recovery']]))
        job = self.wait_job(self.recognition, self.recognition('start', req))
        cancelled = http.uid()
        ended = self.recognition('cancel-import', {'target': target, 'operationId': cancelled})
        self.assertEqual(ended['status'], 'cancelled')
        late = self.recognition('prepare', {'target': target, 'jobId': req['jobId'], 'operationId': cancelled, 'review': review(job)})
        self.assertEqual(late['status'], 'cancelled')
        self.assertFalse((self.root / target['name']).exists())
        prepared = self.recognition('prepare', {'target': target, 'jobId': req['jobId'], 'operationId': http.uid(), 'review': review(job)})
        self.server.close()
        self.server = http.Server([ '--data-dir', str(self.data)])
        opened = self.api('/api/open', {'root': str(self.root), 'path': self.original['path']})
        reconnected = {**target, 'sessionId': opened['sessionId']}
        status = self.recognition('operation', {'target': reconnected, 'operationId': prepared['operationId']})
        self.assertEqual(status['planId'], prepared['planId'])
        receipt = self.commit(reconnected, prepared)
        self.assertEqual(receipt['status'], 'committed')
        self.assertEqual(self.recognition('status', {'jobId': req['jobId']})['resultDigest'], job['resultDigest'])
        self.assert_tree(self.root / target['name'], text_pdf([['TEST recovery']]), receipt)
        self.recognition('cancel', {'jobId': req['jobId']})
        self.assertEqual(self.recognition('status', {'jobId': req['jobId']})['status'], 'cancelled')
        self.assertEqual(self.recognition('start', {**req, 'target': reconnected})['status'], 'cancelled')

    def test_local_runtime_text_preview_partial_consent_and_safe_source(self):
        literal = 'TEST #read("not-authorized.txt") <label> @unknown'
        payload = text_pdf([[literal, 'TEST FIRST PAGE'], ['TEST SECOND PAGE']])
        (self.folder / 'TEST-text.pdf').write_bytes(payload)
        target = self.target('TEST-extracted')
        req = request(target, payload, purpose='text', pages=[{'page': 1}, {'page': 3}])
        job = self.wait_job(self.recognition, self.recognition('start', req))
        self.assertEqual(job['status'], 'partial')
        self.assertEqual(job['result']['pageCount'], 2)
        self.assertEqual([page['page'] for page in job['result']['pages']], [1, 3])
        self.assertEqual(job['result']['pages'][1]['status'], 'failed')
        actual = '\n'.join(block['text'] for page in job['result']['pages'] for region in page['regions'] for block in region['blocks'])
        self.assertIn(literal, actual)
        self.assertNotIn('TEST SECOND PAGE', actual)
        preview = self.recognition('preview', {'jobId': req['jobId'], 'page': 1})
        pixels = base64.b64decode(preview['dataBase64'])
        self.assertTrue(pixels.startswith(b'\x89PNG\r\n\x1a\n'))
        self.assertGreater(preview['width'], 0)
        delivered = self.recognition('download', {'jobId': req['jobId']})
        text = self.assert_download(delivered, req, job)
        self.assertIn(literal, text.decode('utf-8'))
        selected = review(job, 'text')
        rejected = http.uid()
        self.recognition('prepare', {'target': target, 'jobId': req['jobId'], 'operationId': rejected, 'review': selected}, 409)
        self.recognition('cancel-import', {'target': target, 'operationId': rejected})
        selected['acceptPartial'] = True
        plan = self.recognition('prepare', {'target': target, 'jobId': req['jobId'], 'operationId': http.uid(), 'review': selected})
        self.commit(target, plan, accept=False, status=409)
        saved = self.commit(target, plan)
        source_path = self.assert_tree(self.root / target['name'], payload, saved)
        doc = self.api('/api/open', {'root': str(self.root), 'path': str(source_path)})
        self.assertIn('#text(', doc['source'])
        rendered = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('not-authorized.txt', rendered['html'])

    def test_local_runtime_cancel_does_not_publish_late_result(self):
        target = self.target('TEST-cancelled-worker')
        payload = text_pdf([[f'TEST PAGE {index}'] for index in range(1, 21)])
        req = request(target, payload, purpose='note', pages=[{'page': index} for index in range(1, 21)])
        started = self.recognition('start', req)
        self.assertIn(started['status'], ('pending', 'running'))
        cancelled = self.recognition('cancel', {'jobId': req['jobId']})
        self.assertEqual(cancelled['status'], 'cancelled')
        self.assertNotIn('result', cancelled)
        time.sleep(.3)
        self.assertEqual(self.recognition('status', {'jobId': req['jobId']})['status'], 'cancelled')
        self.recognition('prepare', {'target': target, 'jobId': req['jobId'], 'operationId': http.uid(), 'review': review(cancelled, 'note')}, 409)
        self.assertFalse((self.root / target['name']).exists())

        # Interrupt an actual product worker after its first durable page, rather
        # than manufacturing a progress file. A restart may recover completed
        # pages for viewing; an explicit new job must run before author import.
        target = self.target('TEST-interrupted-worker')
        req = request(target, payload, purpose='note', pages=[{'page': index} for index in range(1, 21)])
        self.recognition('start', req)
        job_dir = self.observe_checkpoint(req, 'observed-host-checkpoint')
        self.server.close()
        self.server = http.Server([ '--data-dir', str(self.data)])
        opened = self.api('/api/open', {'root': str(self.root), 'path': self.original['path']})
        target['sessionId'] = opened['sessionId']
        recovered = self.recognition('status', {'jobId': req['jobId']})
        self.assert_interrupted(recovered, req)
        cached_preview = self.recognition('preview', {'jobId': req['jobId'], 'page': 1})
        self.assertTrue(base64.b64decode(cached_preview['dataBase64']).startswith(b'\x89PNG\r\n\x1a\n'))
        self.assertTrue((job_dir / 'preview-1.json').is_file())
        # Explicit TEST-only input fault injection preserves both byte versions.
        original = (job_dir / 'input.bin').read_bytes()
        self.assertEqual(original, payload)
        (self.folder / 'TEST-input-before-change.pdf').write_bytes(original)
        changed = original + b'\n% TEST changed original bytes\n'
        (self.folder / 'TEST-input-after-change.pdf').write_bytes(changed)
        (job_dir / 'input.bin').write_bytes(changed)
        try:
            self.recognition('status', {'jobId': req['jobId']}, 409)
            self.recognition('preview', {'jobId': req['jobId'], 'page': 1}, 409)
            self.recognition('download', {'jobId': req['jobId']}, 409)
        finally:
            (job_dir / 'input.bin').write_bytes(original)
        self.assertEqual(self.recognition('status', {'jobId': req['jobId']})['resultDigest'], recovered['resultDigest'])
        self.assertEqual(self.recognition('preview', {'jobId': req['jobId'], 'page': 1}), cached_preview)

        crash_target = {**self.target('TEST-worker-crash'), 'sessionId': opened['sessionId']}
        crashed = request(crash_target, payload, purpose='note', pages=req['options']['pages'])
        self.recognition('start', crashed)
        self.observe_checkpoint(crashed, 'observed-worker-checkpoint')
        process = stop_owned_worker(self.server.process.pid)
        (self.folder / 'TEST-stopped-owned-worker.json').write_text(json.dumps(process, indent=2), 'utf-8')
        self.assertIsNone(self.server.process.poll(), 'Worker-only termination must leave the host alive')
        crashed_result = self.wait_job(self.recognition, {'jobId': crashed['jobId'], 'status': 'running'})
        self.assert_interrupted(crashed_result, crashed)
        rerun = request(target, payload, purpose='note', pages=[{'page': 1}])
        final = self.wait_job(self.recognition, self.recognition('start', rerun))
        self.assertEqual(final['status'], 'complete')
        self.assertNotEqual(final['jobId'], req['jobId'])
        plan = self.recognition('prepare', {'target': target, 'jobId': rerun['jobId'], 'operationId': http.uid(), 'review': review(final, 'note')})
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(self.recognition('cancel-import', {'target': target, 'operationId': plan['operationId']})['status'], 'cancelled')
        self.assertFalse((self.root / target['name']).exists())


class HostedRecognition(RecognitionChecks, HostedContentImport):
    @classmethod
    def setUpClass(cls):
        cls.evidence = Path(tempfile.mkdtemp(prefix='weftext-recognition-hosted-', dir=ARTIFACTS))
        print('Hosted recognition TEST artifacts:', cls.evidence, flush=True)

    def setUp(self):
        needs_runtime = '_runtime_' in self._testMethodName
        if needs_runtime and not RUNTIME:
            self.skipTest('Set WEFTEXT_RECOGNITION_RUNTIME for the real worker cases')
        self.enterContext(runtime_environment(RUNTIME if needs_runtime else None))
        super().setUp()
        self.prepare_imports()

    def recognition(self, route, body=None, status=200, actor='alice'):
        value = self.call('recognition/' + route, body, actor, status)
        with (self.folder / 'recognition.ndjson').open('a', encoding='utf-8') as log:
            log.write(json.dumps({'actor': actor, 'route': route, 'status': status, 'response': value}, ensure_ascii=False) + '\n')
        return value

    def grants(self, recognition, members=None, resources=None):
        self.grant_import(slots=['Native'], members=members or {}, resources=resources or {})
        self.policy(lambda users: users['alice']['contentImports']['Native'].update(recognition=recognition))

    def prepare_job(self, req, job, selected=None, status=200):
        return self.recognition('prepare', {'target': req['target'], 'jobId': req['jobId'], 'operationId': http.uid(), 'review': selected or review(job)}, status)

    def commit_job(self, req, plan):
        return self.recognition('commit', {'target': req['target'], 'operationId': plan['operationId'], 'planId': plan['planId'], 'acceptLosses': True})

    def test_hosted_denial_private_jobs_and_independent_consumers(self):
        payload, target = tiny_png(), {'slot': 'Native'}
        req = request(target, payload, 'png')
        self.assertEqual(self.recognition('capabilities')['targets'], [])
        self.recognition('start', req, 404)
        self.grants({'read': True})
        self.recognition('start', req, 404)
        self.grants({'run': True, 'read': True})
        job = self.wait_job(self.recognition, self.recognition('start', req))
        self.assertEqual(job['status'], 'complete')
        hidden = self.recognition('status', {'jobId': req['jobId']}, 404, 'bob')
        missing = self.recognition('status', {'jobId': http.uid()}, 404, 'bob')
        self.assertEqual(hidden, missing)
        self.prepare_job(req, job, status=404)
        self.recognition('download', {'jobId': req['jobId']}, 404)
        self.grants({'run': True, 'read': True, 'import': True})
        self.recognition('status', {'jobId': req['jobId']}, 409)
        req = request(target, payload, 'png')
        job = self.wait_job(self.recognition, self.recognition('start', req))
        plan = self.prepare_job(req, job)
        receipt = self.commit_job(req, plan)
        self.assertEqual(receipt['status'], 'committed')
        member = 'Native/' + receipt['nodeRefs'][0]['nodeId']
        self.call('open', {'resource': member}, status=404)
        self.assertFalse(self.call('media')['resources'])
        self.grants({'read': True}, members={'read': True, 'write': True, 'history': True})
        doc = self.open(member)
        self.assertEqual(doc['source'], self.open(member)['source'])
        self.assert_tree(self.root / 'Native', payload, receipt)
        self.recognition('start', request(target, payload, 'png'), 404)
        self.grants({})
        self.recognition('status', {'jobId': req['jobId']}, 404)
        self.grants({'read': True})
        self.recognition('status', {'jobId': req['jobId']}, 409)

    def test_hosted_runtime_download_and_import_are_separate(self):
        payload, target = text_pdf([['TEST PRIVATE EXTRACTED TEXT']]), {'slot': 'Native'}
        self.grants({'run': True, 'read': True, 'download': True})
        req = request(target, payload, purpose='text')
        job = self.wait_job(self.recognition, self.recognition('start', req))
        self.assertEqual(job['status'], 'complete')
        output = self.recognition('download', {'jobId': req['jobId']})
        content = self.assert_download(output, req, job)
        self.assertIn(b'TEST PRIVATE EXTRACTED TEXT', content)
        self.prepare_job(req, job, review(job, 'text'), 404)
        self.grants({'run': True, 'read': True, 'import': True})
        self.recognition('status', {'jobId': req['jobId']}, 409)
        req = request(target, payload, purpose='note')
        job = self.wait_job(self.recognition, self.recognition('start', req))
        self.recognition('download', {'jobId': req['jobId']}, 404)
        receipt = self.commit_job(req, self.prepare_job(req, job, review(job, 'note')))
        self.assertEqual(receipt['status'], 'committed')
        self.call('open', {'resource': 'Native/' + receipt['nodeRefs'][0]['nodeId']}, status=404)

    def test_hosted_runtime_actual_candidate_requires_mapped_field_rights(self):
        field, type_id = 'test-recognition/agency', 'test-recognition/official'
        def install(api):
            config = api('/api/data/schema', {'root': str(self.root)})
            schema = json.loads(config['source'])
            schema['fields'].append({'id': field, 'key': 'test_agency', 'shape': {'kind': 'scalar', 'semantic': 'text'}, 'nullable': False, 'default': {'kind': 'absent'}})
            schema['types'].append({'id': type_id, 'fields': [{'id': field, 'required': False}], 'requires': [], 'conflicts': [], 'constraints': []})
            plan = api('/api/data/schema/prepare', {'root': str(self.root), 'source': json.dumps(schema, ensure_ascii=False), 'beforeSource': config['source'], 'beforeRevision': config['revision'], 'operationId': http.uid()})
            api('/api/data/schema/install', {'operationId': plan['operationId'], 'planId': plan['planId']})
        self.local(install)
        payload = official_docx()
        (self.folder / 'TEST-official.docx').write_bytes(payload)
        target = {'slot': 'Native'}
        rights = {'run': True, 'read': True, 'import': True}
        self.grants(rights)
        options = {'scheme': OFFICIAL, 'language': 'zh', 'script': 'Hans', 'region': 'CN'}
        req = request(target, payload, 'docx', 'note', **options)
        job = self.wait_job(self.recognition, self.recognition('start', req))
        self.assertEqual(job['status'], 'complete')
        self.assertEqual(job['scheme']['fields']['issuingAgency']['value'], 'TEST OFFICE')
        selected = review(job, 'note')
        selected['fields'] = [{'key': 'issuingAgency', 'value': 'TEST OFFICE', 'confirmed': True, 'candidateIndex': 0, 'manualCorrection': False, 'targetField': field, 'targetType': type_id}]
        self.assertEqual(self.recognition('fields', {'target': target})['fields'], [])
        self.prepare_job(req, job, selected, 404)
        self.grants(rights, members={'fields': {field: {'read': True, 'write': True}}})
        req = request(target, payload, 'docx', 'note', **options)
        job = self.wait_job(self.recognition, self.recognition('start', req))
        available = self.recognition('fields', {'target': target})['fields']
        self.assertIn((field, type_id), [(row['id'], row['typeId']) for row in available])
        plan = self.prepare_job(req, job, selected)
        receipt = self.commit_job(req, plan)
        self.assertEqual(receipt['status'], 'committed')
        member = 'Native/' + receipt['nodeRefs'][0]['nodeId']
        self.call('open', {'resource': member}, status=404)
        profile = self.call('fields/read', {'resource': member, 'fieldId': field})
        self.assertIn('TEST OFFICE', json.dumps(profile))
        self.assertNotIn('source', profile)
        self.assert_tree(self.root / 'Native', payload, receipt)


def main():
    global ARTIFACTS, RUNTIME
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('--artifacts-dir', type=Path)
    parser.add_argument('--test', action='append', help='Exact test name; may be repeated')
    args = parser.parse_args()
    if not args.executable.is_file():
        parser.error('Supply an existing candidate executable')
    http.EXE = args.executable.resolve()
    if args.artifacts_dir:
        args.artifacts_dir.mkdir(parents=True, exist_ok=True)
        ARTIFACTS = args.artifacts_dir.resolve()
    else:
        ARTIFACTS = Path(tempfile.mkdtemp(prefix='weftext-recognition-'))
    RUNTIME = os.environ.get('WEFTEXT_RECOGNITION_RUNTIME')
    if RUNTIME and not Path(RUNTIME).is_absolute():
        parser.error('WEFTEXT_RECOGNITION_RUNTIME must be an explicit absolute path')
    classes = (LocalRecognition, HostedRecognition)
    declared = {name: cls for cls in classes for name in cls.__dict__ if name.startswith('test_')}
    selected = args.test or sorted(declared)
    if set(selected) - declared.keys():
        parser.error('Unknown test selection: ' + ', '.join(sorted(set(selected) - declared.keys())))
    manifest = {'scope': 'Recognition HTTP product boundaries; no browser or whole-version acceptance',
                'candidate': str(http.EXE), 'candidateSha256': sha(http.EXE.read_bytes()),
                'testSourceSha256': sha(Path(__file__).read_bytes()),
                'pythonExecutable': sys.executable, 'pythonVersion': sys.version,
                'platform': platform.platform(), 'recognitionRuntime': RUNTIME,
                'selected': selected}
    if RUNTIME and (Path(RUNTIME) / 'installation.json').is_file():
        installation = (Path(RUNTIME) / 'installation.json').read_bytes()
        manifest['runtimeInstallationSha256'] = sha(installation)
        manifest['runtimeLockSha256'] = json.loads(installation)['runtimeLockSha256']
    print('Recognition HTTP candidate:', http.EXE, 'SHA-256', manifest['candidateSha256'], flush=True)
    outcome = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(declared[name](name) for name in selected))
    manifest.update(run=outcome.testsRun, failures=len(outcome.failures), errors=len(outcome.errors),
                    skipped=[{'test': str(test), 'reason': reason} for test, reason in outcome.skipped],
                    passed=outcome.wasSuccessful())
    destination = ARTIFACTS / ('recognition-run-' + http.uid() + '.json')
    destination.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), 'utf-8')
    print('Recognition run manifest:', destination, flush=True)
    return 0 if outcome.wasSuccessful() else 1


if __name__ == '__main__':
    raise SystemExit(main())
