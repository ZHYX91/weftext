"""Exercise the built local application against disposable files, never a real library."""
from _cli_serve import launch_serve
from _creation_result import created_document
import argparse
import json
from pathlib import Path
import tempfile
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen
import uuid

EXE = None


def uid():
    return str(uuid.uuid4())


def walk(node):
    if isinstance(node, dict):
        yield node
        for child in node.get('children', []):
            yield from walk(child)


class LocalApplication(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='weftext-mvp-test-')
        cls.root = Path(cls.temp.name)
        cls.owner = launch_serve(EXE, ['--data-dir', str(cls.root / 'state')])
        cls.process = cls.owner.process
        cls.url, cls.token = cls.owner.ready['base'], cls.owner.ready['token']

    @classmethod
    def tearDownClass(cls):
        cls.owner.stop()
        cls.temp.cleanup()

    def api(self, path, data=None, status=200, headers=None):
        request = Request(self.url + path,
                          data=None if data is None else json.dumps(data).encode(),
                          headers={'X-Weftext-Token': self.token, 'Content-Type': 'application/json',
                                   **(headers or {})})
        try:
            with urlopen(request, timeout=35) as response:
                code, body = response.status, response.read()
        except HTTPError as response:
            code, body = response.code, response.read()
        self.assertEqual(code, status, body.decode(errors='replace')[:2000])
        return json.loads(body)

    def ordinary(self, source='= 中文标题\n\n正文 *强调*。\n'):
        folder = self.root / uid()
        folder.mkdir()
        path = folder / '普通.typ'
        path.write_bytes(source.encode('utf-8'))
        return path, self.api('/api/open', {'path': str(path), 'root': str(folder)})

    def test_access_boundary(self):
        self.api('/api/bootstrap', headers={'X-Weftext-Token': 'wrong'}, status=403)
        self.api('/api/bootstrap', headers={'Origin': 'https://example.invalid'}, status=403)
        self.api('/api/bootstrap', headers={'Host': 'example.invalid'}, status=403)
        self.assertTrue(self.api('/api/bootstrap')['capabilities']['nativeTypst'])

    def test_exact_save_retry_and_external_conflict(self):
        path, doc = self.ordinary()
        source = '= 中文标题\r\n\r\n保留中文与 emoji 🌿。\r\n'
        req = {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
               'operationId': uid(), 'source': source}
        receipt = self.api('/api/save', req)
        self.assertEqual(receipt['status'], 'saved')
        self.assertEqual(path.read_bytes(), source.encode())
        self.assertEqual(self.api('/api/save', req), receipt)
        path.write_text('外部新稿', encoding='utf-8')
        result = self.api('/api/save', {**req, 'baseId': receipt['baseId'],
                         'operationId': uid(), 'source': '我的新稿'}, status=409)
        self.assertEqual(result['error']['code'], 'conflict')
        self.assertEqual(path.read_text(encoding='utf-8'), '外部新稿')

    def test_save_as_preserves_original_and_new_session(self):
        path, doc = self.ordinary()
        target = path.with_name('另存.typ')
        req = {'sessionId': doc['sessionId'], 'path': str(target),
               'operationId': uid(), 'source': '另存稿'}
        receipt = self.api('/api/save-as', req)
        self.assertEqual(self.api('/api/save-as', req), receipt)
        self.assertEqual(path.read_text(encoding='utf-8'), doc['source'])
        opened = self.api('/api/open', {'path': str(target)})
        self.api('/api/save', {'sessionId': opened['sessionId'], 'baseId': opened['baseId'],
                              'operationId': uid(), 'source': '另存后的继续编辑'})
        self.assertEqual(target.read_text(encoding='utf-8'), '另存后的继续编辑')
        self.assertEqual(path.read_text(encoding='utf-8'), doc['source'])

    def test_link_path_can_reopen_and_cannot_escape(self):
        path, doc = self.ordinary()
        target = path.with_name('链接 空格.typ')
        target.write_text('目标', encoding='utf-8')
        req = {'sessionId': doc['sessionId'], 'target': '链接%20空格.typ#part'}
        link = self.api('/api/link/resolve', req)
        self.assertEqual(link['fragment'], 'part')
        opened = self.api('/api/open', {'path': link['path'], 'root': link['root']})
        self.assertEqual(opened['source'], '目标')
        outside = self.root / f'{uid()}.typ'
        outside.write_text('outside', encoding='utf-8')
        self.api('/api/link/resolve', {**req, 'target': outside.as_uri()}, status=400)
        self.api('/api/link/resolve', {**req, 'target': 'javascript:alert(1)'}, status=400)

    def test_library_node_and_id_retry(self):
        root = self.root / uid()
        req = {'path': str(root), 'operationId': uid()}
        lib = self.api('/api/library/create', req)
        self.assertEqual(self.api('/api/library/create', req)['workspaceId'], lib['workspaceId'])
        create = {'root': str(root), 'parent': str(root), 'name': '第一篇', 'operationId': uid()}
        node = created_document(self.api('/api/note/create', create))
        replay = created_document(self.api('/api/note/create', create))
        self.assertEqual(replay['nodeId'], node['nodeId'])
        self.assertEqual(node['kind'], 'managed')
        self.assertEqual(Path(node['path']), root / '第一篇' / '第一篇.typ')
        self.api('/api/note/create', {**create, 'name': '另一篇'}, status=409)
        inv = self.api('/api/library/open', {'path': str(root)})
        admitted = [e for e in inv['entries'] if e['identityStatus'] == 'admitted']
        self.assertEqual(len(admitted), 1)
        link = self.api('/api/link/resolve', {'sessionId': node['sessionId'],
                        'target': f"weftext-node://{lib['workspaceId']}/{node['nodeId']}"})
        self.assertEqual(Path(link['path']), Path(node['path']))

    def test_native_preview_and_stale_edit(self):
        source = '= 中文标题\n\n正文 *强调*\n\n$ x^2 $\n\n#table(columns: 2, table.cell(colspan: 2)[合并])\n'
        _, doc = self.ordinary(source)
        req = {'sessionId': doc['sessionId'], 'source': source, 'serial': 1}
        result = self.api('/api/render', req)
        self.assertTrue(result['ok'], result['diagnostics'])
        tags = [n.get('tag') for n in walk(result['tree'])]
        self.assertIn('math', tags)
        self.assertTrue(any(n.get('attrs', {}).get('colspan') == '2' for n in walk(result['tree'])))
        # Arbitrary function calls have preview but no rich-write capability.
        # Exercise the explicitly supported literal profile separately.
        req = {**req, 'source': '= 中文标题\n\n正文 *强调*\n', 'serial': 2}
        result = self.api('/api/render', req)
        field = next(n for n in walk(result['tree']) if n.get('writable') and n.get('text') == '强调')
        edit = self.api('/api/edit-literal', {**req, 'nodeKey': field['key'], 'text': '中文*文字'})
        self.assertIn('中文\\*文字', edit['source'])
        self.api('/api/render', {**req, 'source': '= 新稿\n', 'serial': 3})
        self.api('/api/edit-literal', {**req, 'nodeKey': field['key'], 'text': '迟到编辑'}, status=400)

    def test_empty_body_and_invalid_source(self):
        _, doc = self.ordinary('')
        req = {'sessionId': doc['sessionId'], 'source': '', 'serial': 1}
        result = self.api('/api/render', req)
        self.assertTrue(result['sourceInfo']['bodyInsertion']['writable'])
        edit = self.api('/api/edit-literal', {**req, 'nodeKey': '@body', 'kind': 'heading',
                       'level': 9, 'text': '深标题'})
        self.assertTrue(edit['source'].startswith('========= 深标题'))
        invalid = self.api('/api/render', {**req, 'source': '#let x = (', 'serial': 2})
        self.assertFalse(invalid['ok'])
        self.assertTrue(invalid['diagnostics'])

    def test_recovered_draft_requires_explicit_merge(self):
        path, doc = self.ordinary('原稿')
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': '未保存草稿'})
        path.write_text('外部新稿', encoding='utf-8')
        opened = self.api('/api/open', {'path': str(path)})
        self.assertEqual(opened['recoveredDraftBaseSource'], '原稿')
        self.assertTrue(opened['recoveredDraftConflict'])
        req = {'sessionId': opened['sessionId'], 'baseId': opened['baseId'],
               'source': '未保存草稿', 'operationId': uid()}
        failure = self.api('/api/save', req, status=409)
        self.assertEqual(failure['error']['code'], 'draftConflict')
        merged = '外部新稿\n未保存草稿'
        self.api('/api/draft/resolve', {'sessionId': opened['sessionId'],
                                      'action': 'mergeRecovered', 'source': merged})
        self.api('/api/save', {**req, 'source': merged, 'operationId': uid()})
        self.assertEqual(path.read_text(encoding='utf-8'), merged)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('executable', type=Path)
    args = parser.parse_args()
    EXE = args.executable.resolve(strict=True)
    unittest.main(argv=[__file__], verbosity=2)
