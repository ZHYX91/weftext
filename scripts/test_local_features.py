"""Real HTTP acceptance for the local v0.2-v0.4 feature slices.

Only disposable files are used. Fixtures and server logs are retained after the
run, including failures, so a failing transaction can be inspected before any
workspace cleanup. No caller-supplied library or private data directory is used.
This checks server behavior; IME, accessibility and shared client Undo require
the separate client acceptance and are not established by these tests.
"""
import argparse
import base64
import ctypes
import hashlib
import io
import json
from pathlib import Path
import queue
import re
import struct
import subprocess
import sys
import tempfile
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen
import uuid
import wave
import zlib


EXE = None
ARTIFACT_PARENT = None


def uid():
    return str(uuid.uuid4())


def sha(source):
    return hashlib.sha256(source.encode('utf-8')).hexdigest()


def units(source):
    return len(source.encode('utf-16-le')) // 2


def text(value):
    return {'kind': 'text', 'value': value}


def record(**members):
    return {'kind': 'record', 'value': [{'key': key, 'value': value}
                                      for key, value in members.items()]}


def array(*values):
    return {'kind': 'array', 'value': list(values)}


def tiny_png():
    def chunk(kind, payload):
        return (struct.pack('>I', len(payload)) + kind + payload
                + struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff))
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', 2, 2, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(b'\x00\xff\x00\x00\x00\xff\x00' * 2))
            + chunk(b'IEND', b''))


def walk(node):
    if isinstance(node, dict):
        yield node
        for child in node.get('children', []):
            yield from walk(child)


class LocalFeatures(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(tempfile.mkdtemp(prefix='weftext-features-', dir=ARTIFACT_PARENT))
        cls.data_dir = cls.root / 'private-state'
        cls.stderr = (cls.root / 'server.stderr.log').open('w', encoding='utf-8')
        cls.stdout = (cls.root / 'server.stdout.log').open('w', encoding='utf-8')
        cls.process = subprocess.Popen(
            [str(EXE), '--no-browser', '--data-dir', str(cls.data_dir)],
            stdout=subprocess.PIPE, stderr=cls.stderr, text=True, encoding='utf-8',
            creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        lines = queue.Queue()

        def read_lines():
            for line in cls.process.stdout:
                # The token is deliberately omitted from the retained log.
                cls.stdout.write(re.sub(r'#token=\w+', '#token=<redacted>', line))
                cls.stdout.flush()
                lines.put(line)
            lines.put(None)

        cls.reader = threading.Thread(target=read_lines, daemon=True)
        cls.reader.start()
        try:
            while True:
                line = lines.get(timeout=40)
                if line is None:
                    raise RuntimeError('Application exited before printing its local URL')
                match = re.search(r'(http://127\.0\.0\.1:\d+)/#token=(\w+)', line)
                if match:
                    cls.url, cls.token = match.groups()
                    break
        except Exception:
            cls.process.terminate()
            cls.process.wait(timeout=10)
            cls.stderr.close()
            cls.stdout.close()
            print(f'Retained failed startup fixtures: {cls.root}', file=sys.stderr)
            raise
        print(f'Acceptance fixtures (retained): {cls.root}', flush=True)

    @classmethod
    def tearDownClass(cls):
        cls.process.terminate()
        cls.process.wait(timeout=15)
        cls.reader.join(timeout=2)
        cls.stderr.close()
        cls.stdout.close()
        print(f'Retained acceptance fixtures: {cls.root}', flush=True)

    def raw(self, path, data=None, status=200, headers=None, token=True):
        headers = {'Content-Type': 'application/json', **(headers or {})}
        if token:
            headers.setdefault('X-Weftext-Token', self.token)
        request = Request(self.url + path,
                          data=None if data is None else json.dumps(data, ensure_ascii=False).encode(),
                          headers=headers)
        try:
            with urlopen(request, timeout=75) as response:
                code, body, response_headers = response.status, response.read(), response.headers
        except HTTPError as response:
            code, body, response_headers = response.code, response.read(), response.headers
        expected = (status,) if isinstance(status, int) else status
        self.assertIn(code, expected,
                      f'{path}: HTTP {code}, expected {expected}: {body.decode(errors="replace")[:2500]}')
        return code, body, response_headers

    def api(self, path, data=None, status=200, headers=None, token=True):
        _, body, _ = self.raw(path, data, status, headers, token)
        return json.loads(body)

    def library(self):
        root = self.root / uid()
        library = self.api('/api/library/create', {'path': str(root), 'operationId': uid()})
        return root, library

    def note(self, root, name, source=None, parent=None):
        doc = self.api('/api/note/create', {'root': str(root), 'parent': str(parent or root),
                                          'name': name, 'operationId': uid()})
        if source is not None:
            metadata = doc['source'].split('\n', 1)[0]
            doc = self.saved(doc, metadata + '\n\n' + source, root)
        return doc

    def saved(self, doc, source, root=None):
        receipt = self.api('/api/save', {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                                         'operationId': uid(), 'source': source})
        self.assertEqual(receipt['status'], 'saved')
        self.assertEqual(Path(doc['path']).read_bytes(), source.encode())
        return self.api('/api/open', {'path': doc['path'], **({'root': str(root)} if root else {})})

    def ordinary(self, source):
        folder = self.root / uid()
        folder.mkdir()
        path = folder / '普通 中文.typ'
        path.write_bytes(source.encode())
        return self.api('/api/open', {'path': str(path), 'root': str(folder)})

    def snapshot(self, doc, source=None, serial=1):
        source = doc['source'] if source is None else source
        return {'sessionId': doc['sessionId'], 'source': source, 'serial': serial,
                'sourceHash': sha(source)}

    def context(self, doc, source=None, serial=1, dirty=False):
        return {'sessionId': doc['sessionId'], 'source': doc['source'] if source is None else source,
                'serial': serial, 'baseId': doc['baseId'], 'dirty': dirty}

    def assert_range(self, source, address):
        raw = source.encode()[address['from']:address['to']].decode()
        self.assertEqual(raw, address['raw'])
        self.assertEqual(units(source.encode()[:address['from']].decode()), address['utf16From'])
        self.assertEqual(units(source.encode()[:address['to']].decode()), address['utf16To'])

    def assert_patches(self, before, output, data=False):
        byte_result = before.encode()
        utf16_result = before.encode('utf-16-le')
        for patch in reversed(output['patches']):
            bfrom, bto = (patch['byteFrom'], patch['byteTo']) if data else (patch['from'], patch['to'])
            ufrom, uto = (patch['from'], patch['to']) if data else (patch['utf16From'], patch['utf16To'])
            insert = patch['text'] if data else patch['insert']
            self.assertEqual(before.encode()[bfrom:bto].decode(), patch['expected'])
            self.assertEqual(before.encode('utf-16-le')[ufrom * 2:uto * 2].decode('utf-16-le'),
                             patch['expected'])
            byte_result = byte_result[:bfrom] + insert.encode() + byte_result[bto:]
            utf16_result = (utf16_result[:ufrom * 2] + insert.encode('utf-16-le')
                            + utf16_result[uto * 2:])
        self.assertEqual(byte_result.decode(), output['source'])
        self.assertEqual(utf16_result.decode('utf-16-le'), output['source'])
        expected_hash = sha(before) if data else sha(output['source'])
        self.assertEqual(output['sourceHash'], expected_hash)

    def edit(self, doc, source, serial, intent):
        snap = self.snapshot(doc, source, serial)
        mapping = self.api('/api/editor-map', snap)
        self.assertEqual(mapping['sourceHash'], sha(source))
        output = self.api('/api/edit', {**snap, 'intent': intent(mapping)})
        self.assert_patches(source, output)
        return output['source'], output['map']

    def prepare(self, root, intent, status=200, operation_id=None):
        return self.api('/api/operations/prepare', {'root': str(root),
                        'operationId': operation_id or uid(), 'intent': intent}, status=status)

    def commit(self, preview, context=None, status=200):
        request = {'operationId': preview['operationId'], 'planId': preview['planId']}
        if context:
            request['context'] = context
        result = self.api('/api/operations/commit', request, status=status)
        if status == 200:
            self.assertEqual(result['status'], 'committed')
        return result

    def test_01_startup_token_and_origin_gate(self):
        self.api('/api/bootstrap', status=403, token=False)
        self.api('/api/bootstrap', status=403, headers={'X-Weftext-Token': 'invalid'})
        self.api('/api/bootstrap', status=403, headers={'Origin': 'https://example.invalid'})
        self.api('/api/bootstrap', status=403, headers={'Host': 'example.invalid'})
        self.api('/api/operations/prepare', {'root': str(self.root), 'operationId': uid(),
                 'intent': {'kind': 'newNode', 'parent': str(self.root), 'name': '未经授权'}},
                 status=403, token=False)
        self.assertFalse((self.root / '未经授权').exists())
        bootstrap = self.api('/api/bootstrap')
        self.assertTrue(bootstrap['capabilities']['nativeTypst'])
        self.assertEqual(bootstrap['platform'], 'windows')
        self.assertEqual(sys.platform, 'win32')
        filesystem = ctypes.create_unicode_buffer(32)
        result = ctypes.windll.kernel32.GetVolumeInformationW(
            str(self.root.anchor), None, 0, None, None, None, filesystem, len(filesystem))
        self.assertTrue(result, 'Could not identify the disposable fixture filesystem')
        self.assertEqual(filesystem.value, 'NTFS')

    def test_02_native_exact_utf16_edit_and_stale_rejection(self):
        source = '= 标题🌿\r\n\r\n正文 *强调🌿* 与 _斜体_。\r\n'
        doc = self.ordinary(source)
        snap = self.snapshot(doc, serial=4)
        mapping = self.api('/api/editor-map', snap)
        self.assertTrue(mapping['syntaxValid'])
        region = next(r for r in mapping['literalRegions'] if r['text'] == '强调🌿')
        self.assert_range(source, region['range'])
        output = self.api('/api/edit', {**snap, 'intent': {'kind': 'replaceLiteral',
                          'range': region['range'], 'text': '中文*文字🌿'}})
        self.assert_patches(source, output)
        self.assertIn('中文\\*文字🌿', output['source'])
        self.assertEqual(Path(doc['path']).read_bytes(), source.encode())
        forged = {**region['range'], 'utf16From': region['range']['utf16From'] + 1}
        refusal = self.api('/api/edit', {**snap, 'intent': {'kind': 'replaceLiteral',
                           'range': forged, 'text': '坏范围'}}, status=422)
        self.assertTrue(refusal['error']['details']['sourcePreserved'])
        self.api('/api/editor-map', self.snapshot(doc, output['source'], 5))
        stale = self.api('/api/edit', {**snap, 'intent': {'kind': 'replaceLiteral',
                         'range': region['range'], 'text': '迟到结果'}}, status=409)
        self.assertEqual(stale['error']['code'], 'stale_draft')
        mismatch = self.api('/api/editor-map', {**self.snapshot(doc, output['source'], 6),
                            'sourceHash': sha(source)}, status=409)
        self.assertEqual(mismatch['error']['code'], 'source_hash_mismatch')
        self.saved(doc, output['source'])

    def test_03_native_controls_deep_heading_and_complex_preservation(self):
        doc = self.ordinary('= 一节\n\n正文🌿\n')
        source, _ = self.edit(doc, doc['source'], 1,
                             lambda m: {'kind': 'setTitle', 'text': '文章标题🌿'})
        source, _ = self.edit(doc, source, 2,
                             lambda m: {'kind': 'setSubtitle', 'text': '明确副标题'})
        source, _ = self.edit(doc, source, 3, lambda m: {
            'kind': 'setHeadingLevel', 'range': next(s['range'] for s in m['structures']
                                                   if s['kind'] == 'heading'), 'level': 9})
        source, mapping = self.edit(doc, source, 4, lambda m: {
            'kind': 'setRunIn', 'range': next(s['range'] for s in m['structures']
                                            if s['kind'] == 'heading'), 'enabled': True})
        heading = next(s for s in mapping['structures'] if s['kind'] == 'heading')
        self.assertEqual(heading['level'], 9)
        self.assertTrue(heading['runIn'])
        self.assertEqual(mapping['title']['text'], '文章标题🌿')
        self.assertEqual(mapping['subtitle']['text'], '明确副标题')
        for serial, block in enumerate([
                {'kind': 'bulletList', 'items': ['首项🌿', '次项']},
                {'kind': 'numberedList', 'items': ['第一', '第二']},
                {'kind': 'math', 'text': '"总价" = "单价" times "数量"', 'display': True},
                {'kind': 'table', 'columns': 2, 'cells': ['甲', '乙🌿', '丙', '丁']}], 5):
            source, mapping = self.edit(doc, source, serial, lambda m, block=block: {
                'kind': 'insertBlock', 'at': m['bodyInsertion']['from'], 'block': block})
        table = next(s for s in mapping['structures'] if s['kind'] == 'table')
        self.assertTrue(table['table']['gridWritable'])
        source, mapping = self.edit(doc, source, 9, lambda m: {
            'kind': 'tableEdit', 'range': next(s['range'] for s in m['structures']
                                             if s['kind'] == 'table'),
            'action': {'kind': 'moveRow', 'from': 0, 'to': 1}})
        table = next(s for s in mapping['structures'] if s['kind'] == 'table')
        self.assertEqual([c['raw'] for c in table['table']['cells']], ['丙', '丁', '甲', '乙🌿'])
        source, _ = self.edit(doc, source, 10, lambda m: {
            'kind': 'setTableCell', 'range': next(s for s in m['structures']
                                               if s['kind'] == 'table')['table']['cells'][0],
            'text': '安全*单元格'})
        compiled = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': source, 'serial': 11})
        self.assertTrue(compiled['ok'], compiled['diagnostics'])
        visible = ''.join(n.get('text') or '' for n in walk(compiled['tree']))
        self.assertIn('安全*单元格', visible)
        self.assertIn('首项🌿', visible)
        self.assertTrue(any(n.get('tag') == 'math' for n in walk(compiled['tree'])))
        complex_source = '#table(columns: 2, table.cell(colspan: 2)[原生合并单元格🌿])\n'
        snap = self.snapshot(doc, complex_source, 12)
        mapping = self.api('/api/editor-map', snap)
        table = next(s for s in mapping['structures'] if s['kind'] == 'table')
        self.assertFalse(table['table']['gridWritable'])
        self.assertTrue(table['table']['topologyWritable'])
        inserted = self.api('/api/edit', {**snap, 'intent': {'kind': 'tableEdit', 'range': table['range'],
                                         'action': {'kind': 'insertRow', 'index': 0}}})
        logical = next(s for s in inserted['map']['structures']
                       if s['kind'] == 'table')['table']['logicalCells']
        self.assertEqual([(c['row'], c['column'], c['colspan']) for c in logical],
                         [(0, 0, 1), (0, 1, 1), (1, 0, 2)])
        self.assertEqual(logical[-1]['body']['raw'], '原生合并单元格🌿')
        compiled = self.api('/api/render', {'sessionId': doc['sessionId'],
                            'source': inserted['source'], 'serial': 13})
        self.assertTrue(compiled['ok'], compiled['diagnostics'])
        self.assertTrue(any(n.get('attrs', {}).get('colspan') == '2' for n in walk(compiled['tree'])))
        dynamic_source = '#table(columns: 2, ..([动态甲], [动态乙]))\n'
        snap = self.snapshot(doc, dynamic_source, 14)
        mapping = self.api('/api/editor-map', snap)
        table = next(s for s in mapping['structures'] if s['kind'] == 'table')
        self.api('/api/edit', {**snap, 'intent': {'kind': 'tableEdit', 'range': table['range'],
                              'action': {'kind': 'insertRow', 'index': 0}}}, status=422)
        compiled = self.api('/api/render', {'sessionId': doc['sessionId'],
                            'source': dynamic_source, 'serial': 14})
        self.assertTrue(compiled['ok'], compiled['diagnostics'])
        self.assertEqual(Path(doc['path']).read_text(encoding='utf-8'), doc['source'])

    def test_04_native_paged_pdf_and_default_chinese_fonts(self):
        source = ('= 中文与繁體\n\n正文与 *强调*。\n\n- 中文列表\n\n'
                  '#table(columns: 2, [中文], [繁體])\n\n`中文代码`\n\n'
                  '$ "总价" = "单价" times "数量" $\n\n#pagebreak()\n第二页\n')
        doc = self.ordinary(source)
        fonts = self.api('/api/fonts')
        self.assertTrue(fonts['fonts'])
        paged = self.api('/api/paged', {'sessionId': doc['sessionId'], 'source': source, 'serial': 1})
        self.assertTrue(paged['ok'], paged['diagnostics'])
        self.assertEqual(paged['target'], 'paged')
        self.assertEqual(paged['pageCount'], 2)
        pdf = base64.b64decode(paged['pdfBase64'], validate=True)
        self.assertTrue(pdf.startswith(b'%PDF-'), pdf[:80])
        self.assertIn(b'%%EOF', pdf[-128:])
        self.assertRegex(pdf, rb'/Type\s*/Catalog')
        self.assertRegex(pdf, rb'/FontFile[23]?\b')
        self.assertGreater(len(pdf), 2000)
        (Path(doc['path']).parent / 'actual-native.pdf').write_bytes(pdf)
        self.assertTrue(any('Noto Serif CJK' in f for f in paged['fontFamilies']))
        self.assertFalse(any('missing glyph' in d['message'].lower() for d in paged['diagnostics']))
        self.assertEqual(Path(doc['path']).read_bytes(), source.encode())

    def test_05_styles_review_apply_render_override_and_conflict(self):
        root, _ = self.library()
        doc = self.note(root, '样式验收', '中文样式正文\n')
        before = self.api('/api/styles/read', {'sessionId': doc['sessionId']})
        original = Path(doc['path']).read_bytes()
        baseline = self.api('/api/render', {'sessionId': doc['sessionId'],
                            'source': doc['source'], 'serial': 1})
        self.assertTrue(baseline['ok'], baseline['diagnostics'])
        request = {'sessionId': doc['sessionId'], 'operationId': uid(), 'baseId': before['baseId'],
                   'changes': {'text.size': '12pt'}, 'enabled': True}
        plan = self.api('/api/styles/prepare', request)
        self.assertEqual(plan['beforeSource'], before['source'])
        self.assertIn('size: 12pt', plan['afterSource'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        installed = self.api('/api/styles/apply', {'sessionId': doc['sessionId'],
                             'operationId': plan['operationId'], 'planId': plan['planId']})
        self.assertTrue(installed['installed'])
        self.assertTrue(installed['enabled'])
        self.assertEqual(Path(installed['path']).read_text(encoding='utf-8'), plan['afterSource'])
        styled = self.api('/api/render', {'sessionId': doc['sessionId'],
                          'source': doc['source'], 'serial': 2})
        self.assertTrue(styled['ok'], styled['diagnostics'])
        self.assertNotEqual(styled['environmentKey'], baseline['environmentKey'])
        self.assertNotEqual(styled['html'], baseline['html'])
        self.assertIn('12pt', styled['html'])
        update = self.api('/api/styles/prepare', {'sessionId': doc['sessionId'],
                          'operationId': uid(), 'baseId': installed['baseId'],
                          'changes': {'text.size': '18pt'}})
        self.assertEqual(update['beforeSource'], plan['afterSource'])
        self.assertIn('size: 18pt', update['afterSource'])
        self.assertEqual(Path(installed['path']).read_text(encoding='utf-8'), plan['afterSource'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        updated = self.api('/api/styles/apply', {'sessionId': doc['sessionId'],
                           'operationId': update['operationId'], 'planId': update['planId']})
        self.assertTrue(updated['installed'])
        self.assertTrue(updated['enabled'])
        self.assertEqual(updated['path'], installed['path'])
        self.assertEqual(Path(updated['path']).read_text(encoding='utf-8'), update['afterSource'])
        restyled = self.api('/api/render', {'sessionId': doc['sessionId'],
                            'source': doc['source'], 'serial': 3})
        self.assertTrue(restyled['ok'], restyled['diagnostics'])
        self.assertNotEqual(restyled['environmentKey'], styled['environmentKey'])
        self.assertNotEqual(restyled['html'], styled['html'])
        self.assertIn('18pt', restyled['html'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        override = doc['source'] + '\n#text(size: 9pt)[笔记局部覆盖]\n'
        result = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': override, 'serial': 4})
        self.assertTrue(result['ok'], result['diagnostics'])
        self.assertIn('9pt', result['html'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        prepared = self.api('/api/styles/prepare', {'sessionId': doc['sessionId'],
                            'operationId': uid(), 'baseId': updated['baseId'],
                            'changes': {'text.size': '20pt'}})
        style_path = Path(updated['path'])
        external = style_path.read_bytes() + b'\n// external edit\n'
        style_path.write_bytes(external)
        self.api('/api/styles/apply', {'sessionId': doc['sessionId'],
                 'operationId': prepared['operationId'], 'planId': prepared['planId']}, status=409)
        self.assertEqual(style_path.read_bytes(), external)

    def install_amount_schema(self, root):
        old = self.api('/api/data/schema', {'root': str(root)})
        schema = json.loads(old['source'])
        schema['fields'].append({'id': 'acceptance/amount', 'key': 'amount',
                                 'shape': {'kind': 'scalar', 'semantic': 'decimal'},
                                 'nullable': False, 'default': {'kind': 'absent'}})
        task = next(t for t in schema['types'] if t['id'] == 'tasks/task')
        task['fields'].append({'id': 'acceptance/amount', 'required': False})
        source = json.dumps(schema, ensure_ascii=False, indent=2)
        request = {'root': str(root), 'operationId': uid(), 'beforeSource': old['source'],
                   'beforeRevision': old['revision'], 'source': source}
        plan = self.api('/api/data/schema/prepare', request)
        self.assertFalse((root / '.weftext-schema.json').exists())
        installed = self.api('/api/data/schema/install', {'operationId': plan['operationId'],
                                                         'planId': plan['planId']})
        self.assertEqual(installed['status'], 'committed')
        current = self.api('/api/data/schema', {'root': str(root)})
        self.assertEqual(current['source'], source)
        self.api('/api/data/schema/prepare', {**request, 'operationId': uid()}, status=409)
        return current

    def capture(self, root, name, types, fields, body='真实正文🌿', title=None, operation_id=None):
        request = {'root': str(root), 'parent': str(root), 'name': name, 'types': types,
                   'fields': fields, 'body': body, 'title': title, 'operationId': operation_id or uid()}
        return self.api('/api/data/capture', request), request

    def test_06_schema_capture_fields_query_exact_values_and_same_item(self):
        root, _ = self.library()
        self.install_amount_schema(root)
        amount = {'kind': 'decimal', 'value': '9007199254740993.01'}
        due = {'kind': 'date', 'value': '2026-10-13'}
        doc, request = self.capture(root, '精确任务', ['task'],
                                    {'status': text('open'), 'amount': amount, 'due': due})
        retry = self.api('/api/data/capture', request)
        self.assertEqual(retry['nodeId'], doc['nodeId'])
        self.assertEqual(retry['path'], doc['path'])
        self.assertIn('真实正文🌿', Path(doc['path']).read_text(encoding='utf-8'))
        projected = self.api('/api/data/project', self.snapshot(doc))['projection']
        fields = {f['key']: f for f in projected['fields']}
        self.assertEqual(fields['amount']['cell']['value'], amount)
        self.assertEqual(fields['due']['cell']['value'], due)
        self.assertEqual(fields['priority']['cell']['state'], 'absent')
        changed = self.api('/api/data/edit', {**self.snapshot(doc, serial=2),
                           'key': 'status', 'value': text('done')})
        self.assert_patches(doc['source'], changed, data=True)
        self.assertEqual(changed['patchUnit'], 'utf16')
        self.assertIn('真实正文🌿', changed['source'])
        doc = self.saved(doc, changed['source'], root)
        query = self.api('/api/query/run', {'root': str(root), 'plan': {
            'types': ['task'], 'where': "amount > decimal('9007199254740993.00') && due == date('2026-10-13') && status == 'done'",
            'select': [{'name': 'amount', 'expr': 'amount'}, {'name': 'due', 'expr': 'due'}]}})
        self.assertEqual(len(query['rows']), 1)
        self.assertEqual(query['rows'][0]['cells']['amount']['value'], amount)
        self.assertEqual(query['rows'][0]['cells']['due']['value'], due)
        self.assertEqual(query['snapshotMode'], 'admittedFrozen')
        self.assertEqual(query['coverage'], 'complete')
        self.assertEqual(query['freshness'], 'stale')
        self.assertFalse(query['externalPending'])
        self.assertEqual(query['rawScope']['pendingCount'], 0)
        self.assertFalse(query['access']['resultGrantsWriteAccess'])
        self.api('/api/query/run', {'root': str(root), 'plan': {
                 'where': 'amount > 9007199254740993.0'}}, status=400)
        self.api('/api/data/capture', {**request, 'operationId': uid(), 'name': '缺少状态',
                 'fields': {'amount': amount}}, status=409)
        self.assertFalse((root / '缺少状态').exists())
        good = array(record(number=text('001-010'), label=text('work'), note=text('first')),
                     record(number=text('001-010'), label=text('work'), note=text('duplicate')))
        bad = array(record(number=text('001-010'), label=text('personal')),
                    record(number=text('999'), label=text('work')))
        person, _ = self.capture(root, '同项匹配', ['person'], {'phones': good})
        self.capture(root, '跨项不匹配', ['person'], {'phones': bad})
        projection = self.api('/api/data/project', self.snapshot(person))['projection']
        phones = next(f for f in projection['fields'] if f['key'] == 'phones')
        self.assertEqual(phones['cell']['value'], good)
        result = self.api('/api/query/run', {'root': str(root), 'plan': {
            'types': ['person'], 'where': "phones.exists(p, p.number == '001-010' && p.label == 'work')",
            'select': [{'name': 'phones', 'expr': 'phones'}]}})
        self.assertEqual([r['path'] for r in result['rows']], [person['path']])
        self.assertEqual(result['rows'][0]['cells']['phones']['value'], good)

    def test_07_overlapping_mentions_explicit_target_and_link_conversion(self):
        root, lib = self.library()
        mountain = self.note(root, '山一', '#title[白云山]\n')
        second = self.note(root, '山二', '#title[白云山]\n')
        self.note(root, '单山', '#title[山]\n')
        self.note(root, '道路', '#title[山南路]\n')
        source = '= 起点\n\n白云山南路\n'
        doc = self.note(root, '起点', source)
        context = self.context(doc)
        render = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        self.assertTrue(render['ok'], render['diagnostics'])
        knowledge = self.api('/api/knowledge/read', {'context': context})
        fragment = next(f for f in knowledge['mentions']['fragments'] if f['text'] == '白云山南路')
        spans = {(h['startScalar'], h['endScalar']): h for h in fragment['hits']}
        self.assertIn((0, 3), spans)
        self.assertIn((2, 3), spans)
        self.assertIn((2, 5), spans)
        self.assertEqual({t['nodeId'] for t in spans[0, 3]['targets']},
                         {mountain['nodeId'], second['nodeId']})
        intent = {'kind': 'mentionConvert', 'generation': knowledge['mentions']['generation'],
                  'fragmentId': fragment['id'], 'startScalar': 0, 'endScalar': 3,
                  'target': {'workspaceId': lib['workspaceId'], 'nodeId': second['nodeId']}}
        preview = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(), 'intent': intent})
        self.assertTrue(preview['draftOnly'])
        self.assertIn(second['nodeId'], preview['source'])
        self.assertIn('[白云山]', preview['source'])
        self.assertIn('南路', preview['source'])
        self.assertEqual(Path(doc['path']).read_text(encoding='utf-8'), doc['source'])
        self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
                 'intent': {**intent, 'target': {'workspaceId': lib['workspaceId'], 'nodeId': uid()}}}, status=409)
        doc = self.saved(doc, preview['source'], root)
        context = self.context(doc)
        current = self.api('/api/knowledge/read', {'context': context})
        edge = current['links']['outgoing'][0]
        self.assertEqual(edge['resolution']['target']['nodeId'], second['nodeId'])
        copied = self.api('/api/knowledge/link-copy', {'context': context,
                          'occurrence': edge['occurrence'], 'form': 'relative'})
        self.assertEqual(copied['status'], 'resolved')
        self.assertIn('[白云山]', copied['fullLink'])
        converted = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'linkConvert', 'generation': current['links']['generation'],
                       'occurrence': edge['occurrence'], 'form': 'relative'}})
        self.assertTrue(converted['draftOnly'])
        self.assertIn(copied['address'], converted['source'])
        selected = self.api('/api/open', {'path': second['path'], 'root': str(root)})
        backlinks = self.api('/api/knowledge/read', {'context': self.context(selected)})['links']['incomingId']
        self.assertEqual([e['source']['nodeId'] for e in backlinks], [doc['nodeId']])
        self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
                 'intent': {**intent, 'generation': 'stale'}}, status=409)

    def suggestion(self, doc, needle='原文🌿'):
        start = doc['source'].index(needle)
        context = self.context(doc)
        preview = self.api('/api/knowledge/prepare', {'operationId': uid(), 'context': context,
            'intent': {'kind': 'annotationCreate', 'purpose': 'suggestion', 'body': '建议讨论',
                       'replacement': '修订🌿', 'target': {'kind': 'sourceRange',
                       'from': units(doc['source'][:start]), 'to': units(doc['source'][:start + len(needle)]),
                       'visibleExcerpt': needle}}})
        receipt = self.commit(preview, context)
        doc = receipt['document']
        current = self.api('/api/knowledge/read', {'context': self.context(doc)})
        annotation = next(a for a in current['annotations'] if a['body'] == '建议讨论')
        self.assertEqual(annotation['targetResolution'], 'exact')
        self.assertEqual(annotation['suggestionState'], 'pending')
        return doc, annotation

    def test_08_annotation_accept_is_atomic_and_replayable(self):
        root, _ = self.library()
        doc = self.note(root, '建议验收', '= 批注\n\n前缀🌿 原文🌿 后缀。\n')
        doc, annotation = self.suggestion(doc)
        context = self.context(doc, serial=2)
        preview = self.api('/api/knowledge/prepare', {'operationId': uid(), 'context': context,
            'intent': {'kind': 'annotationAccept', 'annotationId': annotation['id']}})
        self.assertEqual(len(preview['changes']), 2)
        self.assertIn('前缀🌿 修订🌿 后缀。', preview['source'])
        self.assertIn('原文🌿', Path(doc['path']).read_text(encoding='utf-8'))
        receipt = self.commit(preview, context)
        replay = self.commit(preview, context)
        self.assertEqual(replay['status'], receipt['status'])
        source = Path(doc['path']).read_text(encoding='utf-8')
        self.assertEqual(source, preview['source'])
        opened = self.api('/api/open', {'path': doc['path'], 'root': str(root)})
        annotations = self.api('/api/knowledge/read', {'context': self.context(opened)})['annotations']
        accepted = next(a for a in annotations if a['id'] == annotation['id'])
        self.assertEqual(accepted['suggestionState'], 'accepted')
        self.assertEqual(accepted['body'], '建议讨论')
        raw = json.loads(Path(doc['path']).with_name('weftext.annotations.json').read_text(encoding='utf-8'))
        self.assertEqual(next(a for a in raw['annotations'] if a['annotationId'] == annotation['id'])
                         ['suggestion']['state'], 'accepted')

    def test_09_annotation_source_conflict_preserves_discussion_and_pending_state(self):
        root, _ = self.library()
        doc = self.note(root, '批注冲突', '= 批注\n\n原文🌿\n')
        doc, annotation = self.suggestion(doc)
        context = self.context(doc, serial=2)
        preview = self.api('/api/knowledge/prepare', {'operationId': uid(), 'context': context,
            'intent': {'kind': 'annotationAccept', 'annotationId': annotation['id']}})
        annotation_path = Path(doc['path']).with_name('weftext.annotations.json')
        discussion = annotation_path.read_bytes()
        external = doc['source'].replace('原文🌿', '外部改稿🌿').encode()
        Path(doc['path']).write_bytes(external)
        self.commit(preview, context, status=409)
        self.assertEqual(Path(doc['path']).read_bytes(), external)
        self.assertEqual(annotation_path.read_bytes(), discussion)
        opened = self.api('/api/open', {'path': doc['path'], 'root': str(root)})
        current = self.api('/api/knowledge/read', {'context': self.context(opened)})
        preserved = next(a for a in current['annotations'] if a['id'] == annotation['id'])
        self.assertEqual(preserved['body'], '建议讨论')
        self.assertEqual(preserved['suggestionState'], 'pending')
        self.assertIn(preserved['targetResolution'], ('stale', 'invalid', 'unavailable'))
        self.assertFalse(preserved['verified'])

    def test_10_rename_move_identity_trash_restore_and_exact_backup(self):
        root, lib = self.library()
        doc = self.note(root, '原名', '= 稳定身份\n\n保留字节🌿\n')
        original = Path(doc['path']).read_bytes()
        id_address = f"weftext-node://{lib['workspaceId']}/{doc['nodeId']}"
        referring = self.note(root, '引用', f'= 引用\n\n#link("{id_address}")[稳定目标]\n')
        rename = self.prepare(root, {'kind': 'renameNode', 'source': str(Path(doc['path']).parent), 'name': '新名'})
        self.assertEqual(rename['nodeMap'][doc['nodeId']], doc['nodeId'])
        renamed = self.commit(rename)
        path = Path(renamed['path']) / '新名.typ'
        self.assertEqual(path.read_bytes(), original)
        self.assertFalse(Path(doc['path']).exists())
        parent = self.note(root, '容器')
        moved = self.commit(self.prepare(root, {'kind': 'moveNode', 'source': str(path.parent),
                                             'parent': str(Path(parent['path']).parent)}))
        path = Path(moved['path']) / '新名.typ'
        self.assertEqual(path.read_bytes(), original)
        resolved = self.api('/api/link/resolve', {'sessionId': referring['sessionId'], 'target': id_address})
        self.assertEqual(Path(resolved['path']), path)
        trash = self.prepare(root, {'kind': 'trashNode', 'source': str(path.parent)})
        trashed = self.commit(trash)
        self.assertFalse(path.exists())
        self.assertEqual((Path(trashed['path']) / '新名.typ').read_bytes(), original)
        missing = self.api('/api/link/resolve', {'sessionId': referring['sessionId'],
                           'target': id_address}, status=409)
        self.assertEqual(missing['error']['code'], 'missing')
        restored = self.commit(self.prepare(root, {'kind': 'restoreNode', 'trashOperationId': trash['operationId'],
                                                  'parent': str(root), 'name': '恢复名'}))
        restored_path = Path(restored['path']) / '恢复名.typ'
        self.assertEqual(restored_path.read_bytes(), original)
        resolve = self.api('/api/data/resolve', {'root': str(root), 'entityKey': id_address})
        self.assertEqual(Path(resolve['path']), restored_path)
        expected = {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}
        destination = self.root / f'backup-{uid()}'
        backup = self.prepare(root, {'kind': 'backupLibrary', 'destination': str(destination)})
        self.assertFalse(destination.exists())
        receipt = self.commit(backup)
        self.assertEqual(Path(receipt['path']), destination)
        self.assertEqual(self.commit(backup)['path'], receipt['path'])
        actual = {str(p.relative_to(destination / 'library')): p.read_bytes()
                  for p in (destination / 'library').rglob('*') if p.is_file()}
        self.assertEqual(actual, expected)
        manifest = json.loads((destination / 'backup.json').read_text(encoding='utf-8'))
        self.assertEqual(manifest['workspaceId'], lib['workspaceId'])
        self.assertTrue(any(f['path'].startswith('private') for f in manifest['files']))
        for item in manifest['files']:
            payload = (destination / item['path']).read_bytes()
            self.assertEqual(len(payload), item['bytes'])
            self.assertEqual(hashlib.sha256(payload).hexdigest(), item['sha256'])
        self.api('/api/operations/prepare', {'root': str(root), 'operationId': uid(),
                 'intent': {'kind': 'backupLibrary', 'destination': str(root / 'recursive')}}, status=409)

    def test_11_complete_copy_new_ids_resources_and_internal_links(self):
        root, lib = self.library()
        parent = self.note(root, '树', '= 树\n\n主体🌿\n')
        child = self.note(root, '子节点', '= 子\n\n子正文\n', Path(parent['path']).parent)
        child_id = f"weftext-node://{lib['workspaceId']}/{child['nodeId']}"
        parent_id = f"weftext-node://{lib['workspaceId']}/{parent['nodeId']}"
        parent = self.saved(parent, parent['source'] + f'\n#link("{child_id}")[子节点]\n', root)
        child = self.saved(child, child['source'] + f'\n#link("{parent_id}")[父节点]\n', root)
        payload = b'\x00\x01fixture\xff\r\n'
        imported = self.api('/api/resources/prepare-import', {'sessionId': parent['sessionId'],
            'name': 'owned.bin', 'bytes': base64.b64encode(payload).decode(), 'operationId': uid()})
        self.commit(imported)
        resource = Path(parent['path']).with_name('owned.bin')
        self.assertEqual(resource.read_bytes(), payload)
        resource_id = self.api('/api/resources/list', {'sessionId': parent['sessionId']})['resources'][0]['resourceId']
        context = self.context(parent)
        comment = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'annotationCreate', 'body': '随完整副本保留的讨论',
                       'target': {'kind': 'wholeDocument'}}})
        self.commit(comment, context)
        original_annotations = json.loads(Path(parent['path']).with_name(
            'weftext.annotations.json').read_text(encoding='utf-8'))['annotations']
        original_parent, original_child = Path(parent['path']).read_bytes(), Path(child['path']).read_bytes()
        plan = self.prepare(root, {'kind': 'copyNode', 'source': str(Path(parent['path']).parent),
                                  'parent': str(root), 'name': '独立副本'})
        self.assertEqual(set(plan['nodeMap']), {parent['nodeId'], child['nodeId']})
        self.assertEqual(len(set(plan['nodeMap'].values())), 2)
        self.assertTrue(set(plan['nodeMap']).isdisjoint(plan['nodeMap'].values()))
        self.assertFalse((root / '独立副本').exists())
        receipt = self.commit(plan)
        replay = self.commit(plan)
        self.assertEqual({k: v for k, v in replay.items() if k != 'document'},
                         {k: v for k, v in receipt.items() if k != 'document'})
        copy_root = Path(receipt['path'])
        copied_parent = self.api('/api/open', {'path': str(copy_root / '独立副本.typ'), 'root': str(root)})
        copied_child = self.api('/api/open', {'path': str(copy_root / '子节点' / '子节点.typ'), 'root': str(root)})
        self.assertEqual(copied_parent['nodeId'], plan['nodeMap'][parent['nodeId']])
        self.assertEqual(copied_child['nodeId'], plan['nodeMap'][child['nodeId']])
        self.assertIn(plan['nodeMap'][child['nodeId']], copied_parent['source'])
        self.assertNotIn(child_id, copied_parent['source'])
        self.assertIn(plan['nodeMap'][parent['nodeId']], copied_child['source'])
        self.assertNotIn(parent_id, copied_child['source'])
        self.assertEqual((copy_root / 'owned.bin').read_bytes(), resource.read_bytes())
        copied_resources = json.loads((copy_root / 'weftext.resources.json').read_text(encoding='utf-8'))
        self.assertNotEqual(copied_resources['resources'][0]['resourceId'], resource_id)
        actual_resource = self.api('/api/resources/list', {'sessionId': copied_parent['sessionId']})['resources'][0]
        self.assertIsNotNone(actual_resource['revision'])
        _, delivered, _ = self.raw('/api/resources/read', {'sessionId': copied_parent['sessionId'],
            'resource': actual_resource['resource'], 'revision': actual_resource['revision']})
        self.assertEqual(delivered, payload)
        copied_annotations = json.loads((copy_root / 'weftext.annotations.json').read_text(
            encoding='utf-8'))['annotations']
        self.assertEqual(copied_annotations[0]['body'], original_annotations[0]['body'])
        self.assertNotEqual(copied_annotations[0]['annotationId'], original_annotations[0]['annotationId'])
        self.assertEqual(copied_annotations[0]['target']['node']['nodeId'], copied_parent['nodeId'])
        self.assertEqual(Path(parent['path']).read_bytes(), original_parent)
        self.assertEqual(Path(child['path']).read_bytes(), original_child)
        graph = self.api('/api/knowledge/read', {'context': self.context(copied_parent)})
        self.assertEqual(graph['links']['outgoing'][0]['resolution']['target']['nodeId'], copied_child['nodeId'])
        raced = self.prepare(root, {'kind': 'copyNode', 'source': str(Path(parent['path']).parent),
                                   'parent': str(root), 'name': '竞争副本'})
        external = payload + b'external resource change'
        resource.write_bytes(external)
        self.commit(raced, status=409)
        self.assertFalse((root / '竞争副本').exists())
        self.assertEqual(resource.read_bytes(), external)
        self.assertEqual((copy_root / 'owned.bin').read_bytes(), payload)

    def test_12_registered_media_exact_regions_intervals_and_version_conflict(self):
        root, _ = self.library()
        doc = self.note(root, '媒体批注', '= 媒体\n\n保留讨论。\n')

        def chunk(kind, payload):
            return (struct.pack('>I', len(payload)) + kind + payload
                    + struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff))

        png = (b'\x89PNG\r\n\x1a\n'
               + chunk(b'IHDR', struct.pack('>IIBBBBB', 2, 2, 8, 2, 0, 0, 0))
               + chunk(b'IDAT', zlib.compress(b'\x00\xff\x00\x00\x00\xff\x00' * 2))
               + chunk(b'IEND', b''))
        wav_file = io.BytesIO()
        with wave.open(wav_file, 'wb') as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(8000)
            wav.writeframes(b'\x00\x00' * 8000)
        for name, payload in [('region.png', png), ('interval.wav', wav_file.getvalue())]:
            prepared = self.api('/api/resources/prepare-import', {'sessionId': doc['sessionId'],
                'name': name, 'bytes': base64.b64encode(payload).decode(), 'operationId': uid()})
            self.assertFalse(Path(doc['path']).with_name(name).exists())
            self.commit(prepared)
            self.assertEqual(Path(doc['path']).with_name(name).read_bytes(), payload)
        resources = {r['name']: r for r in self.api('/api/resources/list',
                     {'sessionId': doc['sessionId']})['resources']}
        image, audio = resources['region.png'], resources['interval.wav']
        self.assertTrue(image['annotationReady'])
        self.assertEqual(image['metadata'], {'kind': 'image', 'width': 2, 'height': 2})
        self.assertEqual(audio['metadata']['durationTick'], 8000)
        self.assertEqual(audio['metadata']['timebase'], {'numerator': 8000, 'denominator': 1})
        self.assertEqual(audio['metadata']['durationTick'] * audio['metadata']['timebase']['denominator']
                         / audio['metadata']['timebase']['numerator'], 1)
        _, payload, headers = self.raw('/api/resources/read', {'sessionId': doc['sessionId'],
            'resource': image['resource'], 'revision': image['revision']})
        self.assertEqual(payload, png)
        self.assertEqual(headers.get_content_type(), 'image/png')
        targets = [
            {'kind': 'imageRegion', 'resource': image['resource'], 'revision': image['revision'],
             'width': 2, 'height': 2, 'x': 0, 'y': 0, 'rectWidth': 1, 'rectHeight': 2},
            {'kind': 'mediaInterval', 'resource': audio['resource'], 'revision': audio['revision'],
             'timebase': 8000, 'timebaseDenominator': 1, 'durationTick': 8000,
             'startTick': 2000, 'endTick': 4000}]
        for index, target in enumerate(targets):
            context = self.context(doc, serial=index + 1)
            preview = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
                'intent': {'kind': 'annotationCreate', 'body': f'区域讨论{index}', 'target': target}})
            self.assertEqual(len(preview['changes']), 3)
            self.commit(preview, context)
        annotation_path = Path(doc['path']).with_name('weftext.annotations.json')
        records = json.loads(annotation_path.read_text(encoding='utf-8'))['annotations']
        image_target = next(a['target'] for a in records if a['body'] == '区域讨论0')
        interval_target = next(a['target'] for a in records if a['body'] == '区域讨论1')
        self.assertEqual(image_target['kind'], 'imageRegion')
        self.assertEqual(image_target['rect'], {'x': 0, 'y': 0, 'width': 1, 'height': 2})
        self.assertEqual(interval_target['kind'], 'mediaInterval')
        self.assertEqual(interval_target['startTick'], 2000)
        self.assertEqual(interval_target['endTick'], 4000)
        self.assertEqual(interval_target['timebase'], audio['metadata']['timebase'])
        before = annotation_path.read_bytes()
        context = self.context(doc, serial=3)
        self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'annotationCreate', 'body': '伪造几何',
                       'target': {**targets[0], 'width': 999}}}, status=409)
        self.assertEqual(annotation_path.read_bytes(), before)
        Path(doc['path']).with_name('region.png').write_bytes(png + b'external')
        self.api('/api/resources/read', {'sessionId': doc['sessionId'],
            'resource': image['resource'], 'revision': image['revision']}, status=409)
        self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'annotationCreate', 'body': '过期资源', 'target': targets[0]}}, status=409)
        self.assertEqual(annotation_path.read_bytes(), before)

    def test_13_template_preview_matches_created_native_source(self):
        root, _ = self.library()
        doc = self.note(root, '模板入口')
        context = self.context(doc)
        parameters = {'title': '#panic("literal title")🌿', 'body': '正文 *显式原生强调*。'}
        preview = self.api('/api/knowledge/template-preview', {'context': context,
            'templateId': 'native-note', 'parameters': parameters})
        self.assertIsNotNone(preview['tree'], preview['diagnostics'])
        self.assertIn('\\#panic', preview['source'])
        source_visible = ''.join(n.get('text') or '' for n in walk(preview['tree']))
        self.assertIn('#panic("literal title")🌿', source_visible)
        prepared = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'templateCreate', 'templateId': 'native-note',
                       'parent': str(root), 'name': '由模板创建', 'parameters': parameters}})
        self.assertFalse((root / '由模板创建').exists())
        receipt = self.commit(prepared, context)
        created = self.api('/api/open', {'path': str(Path(receipt['path']) / '由模板创建.typ'),
                                        'root': str(root)})
        self.assertEqual(created['nodeId'], prepared['newNodeId'])
        self.assertEqual(created['source'].split('\n', 1)[1], preview['source'].split('\n', 1)[1])
        self.assertNotIn('00000000-0000-4000-8000-000000000000', created['source'])
        self.api('/api/knowledge/template-preview', {'context': context,
            'templateId': 'native-note', 'parameters': {**parameters, 'unknown': '拒绝未知参数'}}, status=409)

    def test_14_cross_inline_selection_and_soft_wrap_preserve_visible_text(self):
        source = '前缀 🌿*粗体文字* 和 _斜体文字_ 后缀'
        doc = self.ordinary(source)
        snap = self.snapshot(doc)
        self.api('/api/editor-map', snap)
        start = source.index('体文字')
        end = source.index('体文字_ 后缀') + len('体文字')
        selected = {'from': len(source[:start].encode()), 'to': len(source[:end].encode()),
                    'utf16From': units(source[:start]), 'utf16To': units(source[:end]),
                    'raw': source[start:end], 'writable': True}
        changed = self.api('/api/edit', {**snap, 'intent': {'kind': 'setInline',
                           'range': selected, 'format': 'strike'}})
        self.assert_patches(source, changed)
        self.assertTrue(changed['source'].startswith('前缀 🌿'))
        self.assertTrue(changed['source'].endswith(' 后缀'))
        self.assertIn('#strike[', changed['source'])
        render = self.api('/api/render', {'sessionId': doc['sessionId'],
                          'source': changed['source'], 'serial': 2})
        self.assertTrue(render['ok'], render['diagnostics'])
        self.assertEqual(''.join(n.get('text') or '' for n in walk(render['tree'])),
                         '前缀 🌿粗体文字 和 斜体文字 后缀')
        wrapped = '第一段🌿\r\n同段换行\r\n\r\n第二段'
        snap = self.snapshot(doc, wrapped, 3)
        mapping = self.api('/api/editor-map', snap)
        region = mapping['literalRegions'][0]
        self.assert_range(wrapped, region['range'])
        appended = self.api('/api/edit', {**snap, 'intent': {'kind': 'replaceLiteral',
                            'range': region['range'], 'text': region['text'] + '追加'}})
        self.assert_patches(wrapped, appended)
        self.assertEqual(appended['source'], wrapped + '追加')
        self.assertEqual(len(appended['patches']), 1)
        self.assertEqual(appended['patches'][0]['from'], len(wrapped.encode()))
        self.assertEqual(Path(doc['path']).read_bytes(), source.encode())

    def test_15_query_native_products_use_frozen_final_output(self):
        root, lib = self.library()
        source = ('#title[#("冻结" + "原生标题🌿")]\n\n'
                  '= 源码一级标题不是文章标题\n\n'
                  '#show "SOURCE_SENTINEL": [实际显示🌿]\n'
                  'SOURCE_SENTINEL\n\n#("计算" + "正文")\n\n'
                  '#heading(level: 9)[生成深层标题]\n')
        native = self.note(root, '编译结果', source)
        absent = self.note(root, '没有文章标题', '= 文件中只有一级标题\n\n无标题正文\n')
        failed = self.note(root, '编译失败', '#panic("有意失败")\n\n= 不能猜测的标题\n猜测正文\n')
        originals = {doc['path']: Path(doc['path']).read_bytes() for doc in (native, absent, failed)}
        result = self.api('/api/query/run', {'root': str(root), 'plan': {
            'select': [{'name': name, 'expr': name} for name in ('title', 'body', 'headings')]}})
        rows = {row['path']: row for row in result['rows']}
        self.assertEqual(set(rows), set(originals))
        cells = rows[native['path']]['cells']
        self.assertEqual(cells['title']['value'], text('冻结原生标题🌿'))
        body = cells['body']['value']['value']
        self.assertIn('实际显示🌿', body)
        self.assertIn('计算正文', body)
        self.assertNotIn('SOURCE_SENTINEL', body)
        self.assertNotIn('#heading', body)
        headings = [{member['key']: member['value'] for member in heading['value']}
                    for heading in cells['headings']['value']['value']]
        self.assertTrue(any(h['level'] == {'kind': 'int', 'value': '9'}
                            and h['text'] == text('生成深层标题') for h in headings))
        self.assertEqual(rows[absent['path']]['cells']['title']['state'], 'absent')
        for name in ('title', 'body'):
            self.assertEqual(rows[failed['path']]['cells'][name]['state'], 'unavailable')
        self.assertEqual(result['snapshotMode'], 'admittedFrozen')
        self.assertEqual(result['coverage'], 'partial')
        frozen = next(v for v in result['inputVersions'] if v.get('role') == 'nativeCompile')
        self.assertEqual(frozen['target'], 'html')
        self.assertTrue(any(v.get('role') == 'nativeProduct'
                            and v['entityKey'] == f"weftext-node://{lib['workspaceId']}/{native['nodeId']}"
                            and v['target'] == 'html' for v in result['inputVersions']))
        for path, before in originals.items():
            self.assertEqual(Path(path).read_bytes(), before)

    def test_16_registered_pdf_four_link_forms_resolve_same_version_and_page(self):
        root, lib = self.library()
        doc = self.note(root, '资源地址')
        paged = self.api('/api/paged', {'sessionId': doc['sessionId'],
            'source': 'PDF 第一页\n#pagebreak()\nPDF 第二页\n', 'serial': 1})
        self.assertTrue(paged['ok'], paged['diagnostics'])
        self.assertEqual(paged['pageCount'], 2)
        pdf = base64.b64decode(paged['pdfBase64'], validate=True)
        imported = self.api('/api/resources/prepare-import', {'sessionId': doc['sessionId'],
            'name': '材料 中文.pdf', 'bytes': base64.b64encode(pdf).decode(), 'operationId': uid()})
        self.commit(imported)
        resource = self.api('/api/resources/list', {'sessionId': doc['sessionId']})['resources'][0]
        self.assertEqual(resource['metadata']['kind'], 'pdf')
        self.assertEqual(len(resource['metadata']['pages']), 2)
        self.assertIsNotNone(resource['revision'])
        source = doc['source'] + f'\n#link("{resource["resource"]}#page=2")[准确第二页]\n'
        doc = self.saved(doc, source, root)
        context = self.context(doc)
        graph = self.api('/api/knowledge/read', {'context': context})
        occurrence = graph['links']['outgoing'][0]['occurrence']
        addresses = {}
        for form in ('id', 'relative', 'workspace', 'file'):
            with self.subTest(form=form):
                copied = self.api('/api/knowledge/link-copy', {'context': context,
                                  'occurrence': occurrence, 'form': form})
                self.assertEqual(copied['status'], 'resolved')
                self.assertFalse(copied['literal'])
                self.assertIn('[准确第二页]', copied['fullLink'])
                addresses[form] = copied['address']
                resolved = self.api('/api/link/resolve', {'sessionId': doc['sessionId'],
                                    'target': copied['address']})
                self.assertEqual(resolved['resourceRef'], resource['resource'])
                self.assertEqual(resolved['revision'], resource['revision'])
                self.assertEqual(resolved['ownerEntityKey'],
                                 f"weftext-node://{lib['workspaceId']}/{doc['nodeId']}")
                self.assertEqual(Path(resolved['path']), Path(doc['path']).with_name('材料 中文.pdf'))
                self.assertEqual(resolved['fragment'], 'page=2')
                self.assertEqual(resolved['resourceFragment'], {'status': 'found', 'pageIndex': 1})
                self.assertEqual(resolved['resolution'], 'found')
        self.assertEqual(len(set(addresses.values())), 4)
        self.assertTrue(addresses['id'].startswith('weftext-resource://'))
        self.assertTrue(addresses['workspace'].startswith('weftext-workspace://'))
        self.assertTrue(addresses['file'].startswith('file:///'))
        self.assertNotIn('://', addresses['relative'])
        missing = self.api('/api/link/resolve', {'sessionId': doc['sessionId'],
                           'target': resource['resource'] + '#page=3'})
        self.assertEqual(missing['resourceRef'], resource['resource'])
        self.assertEqual(missing['resourceFragment']['status'], 'missing')
        self.assertEqual(Path(doc['path']).with_name('材料 中文.pdf').read_bytes(), pdf)

    def test_17_registered_native_template_copies_real_resource_dependency(self):
        root, lib = self.library()
        source = ('// UTF-16 槽前偏移🌿\n#title[标题槽]\n\n正文槽\n\n'
                  '#image("依赖.png", width: 20pt)\n')
        doc = self.note(root, '自定义模板', source)
        png = tiny_png()
        self.commit(self.api('/api/resources/prepare-import', {'sessionId': doc['sessionId'],
            'name': '依赖.png', 'bytes': base64.b64encode(png).decode(), 'operationId': uid()}))
        original_resource = self.api('/api/resources/list', {'sessionId': doc['sessionId']})['resources'][0]
        context = self.context(doc)
        slots = []
        for name, marker in [('title', '标题槽'), ('body', '正文槽')]:
            start = doc['source'].index(marker)
            slots.append({'name': name, 'kind': 'markup', 'from': units(doc['source'][:start]),
                          'to': units(doc['source'][:start + len(marker)])})
        registered = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'templateRegister', 'title': '实际原生模板',
                       'description': '准确文字槽与登记图片依赖', 'parameters': slots}})
        template_id = registered['preview']['templateAfter']['id']
        self.commit(registered, context)
        doc = self.api('/api/open', {'path': doc['path'], 'root': str(root)})
        context = self.context(doc)
        original_source = Path(doc['path']).read_bytes()
        catalog = self.api('/api/knowledge/read', {'context': context})['templates']
        definition = next(t for t in catalog if t['id'] == template_id)
        self.assertTrue(definition['available'])
        self.assertTrue(definition['sourceRevision'])
        self.assertEqual(definition['sourceNode']['nodeId'], doc['nodeId'])
        parameters = {'title': '#panic("literal")🌿', 'body': '实例正文🌿'}
        preview = self.api('/api/knowledge/template-preview', {'context': context,
            'templateId': template_id, 'parameters': parameters})
        self.assertTrue(preview['ok'], preview['diagnostics'])
        self.assertTrue(any(n.get('tag') == 'img' for n in walk(preview['tree'])))
        self.assertIn('\\#panic', preview['source'])
        prepared = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'templateCreate', 'templateId': template_id,
                       'parent': str(root), 'name': '有依赖的实例', 'parameters': parameters}})
        actual_preview = prepared['preview']['template']
        self.assertTrue(actual_preview['ok'], actual_preview['diagnostics'])
        self.assertTrue(any(n.get('tag') == 'img' for n in walk(actual_preview['tree'])))
        self.assertFalse((root / '有依赖的实例').exists())
        receipt = self.commit(prepared, context)
        created = self.api('/api/data/open', {'root': str(root),
            'entityKey': f"weftext-node://{lib['workspaceId']}/{prepared['newNodeId']}"})
        self.assertEqual(created['source'], actual_preview['source'])
        self.assertEqual(Path(created['path']).parent, Path(receipt['path']))
        copied_resource = self.api('/api/resources/list', {'sessionId': created['sessionId']})['resources'][0]
        self.assertNotEqual(copied_resource['resourceId'], original_resource['resourceId'])
        self.assertEqual(copied_resource['nodeId'], created['nodeId'])
        self.assertIsNotNone(copied_resource['revision'])
        _, delivered, _ = self.raw('/api/resources/read', {'sessionId': created['sessionId'],
            'resource': copied_resource['resource'], 'revision': copied_resource['revision']})
        self.assertEqual(delivered, png)
        rendered = self.api('/api/render', {'sessionId': created['sessionId'],
                            'source': created['source'], 'serial': 1})
        self.assertTrue(rendered['ok'], rendered['diagnostics'])
        self.assertTrue(any(n.get('tag') == 'img' for n in walk(rendered['tree'])))
        self.assertEqual(Path(doc['path']).read_bytes(), original_source)
        self.assertEqual(Path(doc['path']).with_name('依赖.png').read_bytes(), png)
        raced = self.api('/api/knowledge/prepare', {'context': context, 'operationId': uid(),
            'intent': {'kind': 'templateCreate', 'templateId': template_id,
                       'parent': str(root), 'name': '过期依赖实例', 'parameters': parameters}})
        Path(doc['path']).with_name('依赖.png').write_bytes(png + b'external change')
        self.commit(raced, context, status=409)
        self.assertFalse((root / '过期依赖实例').exists())
        self.assertEqual(Path(created['path']).with_name('依赖.png').read_bytes(), png)

    def test_18_navigation_directory_tag_identity_and_strong_open(self):
        root, lib = self.library()
        directory = root / '未配对容器'
        directory.mkdir()
        target = self.api('/api/navigation/capture', {'root': str(root), 'path': str(directory)})
        self.assertEqual(target['status'], 'unmanaged')
        admission = self.api('/api/navigation/admit-directory', {'root': str(root),
            'path': str(directory), 'operationId': uid()})
        self.assertFalse((directory / 'weftext.directory.json').exists())
        self.commit(admission['preview'])
        directory_ref = admission['targetRef']
        resolved = self.api('/api/navigation/resolve', {'root': str(root), 'targetRef': directory_ref})
        self.assertEqual(resolved['status'], 'managed')
        self.assertEqual(Path(resolved['path']), directory)
        doc = self.note(root, '导航原名', '= 导航正文\n', directory)
        self.note(root, '范围外条目')
        registry = self.api('/api/navigation/tags', {'root': str(root)})
        first = self.api('/api/navigation/create-tag', {'root': str(root), 'parentRef': None, 'displayName': '同名标签',
                         'baseId': registry['baseId'], 'operationId': uid()})
        self.commit(first['preview'])
        registry = self.api('/api/navigation/tags', {'root': str(root)})
        second = self.api('/api/navigation/create-tag', {'root': str(root), 'parentRef': None, 'displayName': '同名标签',
                          'baseId': registry['baseId'], 'operationId': uid()})
        self.commit(second['preview'])
        self.assertNotEqual(first['targetRef'], second['targetRef'])
        source = (f'#metadata((id: "{doc["nodeId"]}", types: (), '
                  f'tags: ((ref: "{first["targetRef"]}"), (text: "同名标签")))) <weftext>\n\n'
                  '#title[导航条目]\n导航正文\n')
        doc = self.saved(doc, source, root)
        original = Path(doc['path']).read_bytes()
        registry = self.api('/api/navigation/tags', {'root': str(root)})
        renamed_tag = self.api('/api/navigation/rename-tag', {'root': str(root),
            'tagRef': first['targetRef'], 'displayName': '标签新显示名',
            'baseId': registry['baseId'], 'operationId': uid()})
        self.commit(renamed_tag['preview'])
        self.assertEqual(renamed_tag['targetRef'], first['targetRef'])
        labels = {tag['tagRef']: tag['displayName'] for tag in
                  self.api('/api/navigation/tags', {'root': str(root)})['tags']}
        self.assertEqual(labels[first['targetRef']], '标签新显示名')
        self.assertEqual(labels[second['targetRef']], '同名标签')
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        node_ref = f"weftext-node://{lib['workspaceId']}/{doc['nodeId']}"
        self.commit(self.prepare(root, {'kind': 'renameNode',
                    'source': str(Path(doc['path']).parent), 'name': '导航新名'}))
        self.assertFalse(Path(doc['path']).exists())
        query = self.api('/api/query/run', {'root': str(root), 'plan': {
            'scope': {'directory': directory_ref, 'descendants': True},
            'select': [{'name': 'self', 'expr': 'self'}]}})
        self.assertEqual([row['entityKey'] for row in query['rows']], [node_ref])
        opened = self.api('/api/data/open', {'root': str(root), 'entityKey': node_ref})
        self.assertEqual(opened['nodeId'], doc['nodeId'])
        self.assertEqual(Path(opened['path']), directory / '导航新名' / '导航新名.typ')
        self.assertEqual(Path(opened['path']).read_bytes(), original)
        self.assertEqual(query['rows'][0]['path'], opened['path'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path, nargs='?')
    parser.add_argument('--executable', type=Path, dest='executable_option')
    parser.add_argument('--artifacts-dir', type=Path,
                        help='Parent for a new owned disposable fixture directory; no existing files are changed')
    parser.add_argument('--only', action='append', default=[], metavar='TEST',
                        help='Run selected unittest names (repeatable)')
    args = parser.parse_args()
    if bool(args.executable) == bool(args.executable_option):
        parser.error('Supply one executable, either positional or --executable')
    EXE = (args.executable or args.executable_option).resolve(strict=True)
    if args.artifacts_dir:
        ARTIFACT_PARENT = args.artifacts_dir.resolve(strict=True)
    result = unittest.main(argv=[__file__, *args.only], verbosity=2, exit=False).result
    if hasattr(LocalFeatures, 'root'):
        (LocalFeatures.root / 'results.json').write_text(json.dumps({
            'executable': str(EXE), 'testsRun': result.testsRun,
            'failures': [{'test': str(test), 'traceback': trace} for test, trace in result.failures],
            'errors': [{'test': str(test), 'traceback': trace} for test, trace in result.errors],
            'successful': result.wasSuccessful()}, ensure_ascii=False, indent=2), encoding='utf-8')
    sys.exit(0 if result.wasSuccessful() else 1)
