"""Focused v0.6.1 processing/settings and proposal-preview HTTP acceptance.

Run against an explicitly supplied newly built executable. Only disposable owned
libraries are created; originals, personal Drafts, settings and logs are retained.
This script does not establish browser Undo, IME, Office or platform acceptance.
The explicit suite includes only the new tests, never inherited harness suites.
"""
from _cli_serve import launch_serve
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from concurrent.futures import ThreadPoolExecutor

import test_hosted as hosted_harness
import test_local_features as local_harness
from test_hosted_library import HostedLibrary
from _creation_result import library_fact


ARTIFACT_PARENT = None
SLOW_PROPOSAL = '\n#for i in range(3000) [proposal delay paragraph #parbreak()]\n'


def processing(region='CN'):
    return {'schemaVersion': 1, 'defaults': {
        'version': '1', 'languages': [{'language': 'zh', 'script': 'Hans', 'region': region}],
        'scheme': 'general', 'inputFormat': 'typst', 'documentType': 'prose',
        'dependencies': []}, 'rules': [], 'scopedAllowlist': []}


def retained_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')


class LocalProcessing(local_harness.LocalFeatures):
    @classmethod
    def restart_same_state(cls):
        """Restart the owned process, retaining its exact data directory and logs."""
        cls.owner.stop()
        cls.restart_number = getattr(cls, 'restart_number', 0) + 1
        prefix = cls.root / f'server.restart-{cls.restart_number}'
        cls.owner = launch_serve(local_harness.EXE, ['--data-dir', str(cls.data_dir)], log_prefix=prefix)
        cls.process = cls.owner.process
        cls.url, cls.token = cls.owner.ready['base'], cls.owner.ready['token']

    def processing_read(self, root):
        return self.api('/api/processing/read', {'root': str(root)})

    def processing_request(self, root, value=None, before=None):
        before = before if before is not None else self.processing_read(root)
        return {'root': str(root), 'operationId': local_harness.uid(),
                'baseId': before['baseId'], 'beforeSource': before['beforeSource'],
                'processing': processing() if value is None else value}

    def processing_apply(self, request, plan, status=200):
        return self.api('/api/processing/apply', {'request': request, 'planId': plan['planId']}, status)

    def creation_request(self, language='en', region=None):
        defaults = self.api('/api/library/create-defaults')
        # The endpoint exposes certified UTF-16 source ranges, not a second
        # locale authority. Patch the real template's two exact native slots.
        native = defaults['source'].encode('utf-16-le')
        changes = []
        for key, value in [('text.lang', json.dumps(language)),
                           ('text.region', 'none' if region is None else json.dumps(region))]:
            matches = [item for item in defaults['controls'] if item['key'] == key]
            self.assertEqual(len(matches), 1)
            slot = matches[0]
            self.assertTrue(slot['editable'])
            bounds = slot['range']
            start, end = bounds['utf16From'] * 2, bounds['utf16To'] * 2
            self.assertEqual(native[start:end].decode('utf-16-le'), bounds['raw'])
            changes.append((start, end, value.encode('utf-16-le')))
        for start, end, value in sorted(changes, reverse=True):
            native = native[:start] + value + native[end:]
        return {'path': str(self.root / ('initial-library-' + local_harness.uid())),
                'operationId': local_harness.uid(),
                'options': {'initialStyle': {'source': native.decode('utf-16-le'), 'enabled': True}}}

    def test_v061_library_creation_locale_one_plan_and_exact_retry(self):
        request = self.creation_request('en', None)
        root = Path(request['path'])
        receipt = self.api('/api/library/create', request)
        self.assertEqual(receipt['operationId'], request['operationId'])
        self.assertEqual(receipt['root'], str(root))
        style = root / 'Styles' / 'library-style.typ'
        self.assertEqual(style.read_bytes(), request['options']['initialStyle']['source'].encode())
        settings = json.loads((root / '.weftext-settings.json').read_text('utf-8'))
        self.assertEqual(set(settings), {'version', 'defaultStyle'})
        self.assertTrue(settings['defaultStyle']['enabled'])
        manifest = json.loads((root / 'Styles' / 'weftext.resources.json').read_text('utf-8'))
        self.assertEqual(len(manifest['resources']), 1)
        resource = manifest['resources'][0]
        self.assertEqual(settings['defaultStyle']['resource'],
                         f"weftext-resource://{receipt['workspaceId']}/{resource['resourceId']}")
        plan_path = self.data_dir / 'journal' / (request['operationId'] + '.plan.json')
        frozen = json.loads(plan_path.read_text('utf-8'))
        self.assertEqual(frozen['kind'], 'createLibrary')
        self.assertEqual(frozen['transaction']['request']['intent']['options'], request['options'])
        self.assertEqual(len(frozen['transaction']['preview']['nodeMap']), 1)
        self.assertEqual(len(frozen['transaction']['preview']['resourceMap']), 1)
        admitted = self.api('/api/open', {'path': str(root / 'Styles' / 'Styles.typ'), 'root': str(root)})
        self.assertEqual(admitted['kind'], 'managed')
        self.assertEqual(admitted['workspaceId'], receipt['workspaceId'])
        self.assertEqual(library_fact(self.api('/api/library/create', request)), library_fact(receipt))
        changed = copy.deepcopy(request)
        changed['options']['initialStyle']['source'] += '\n// changed frozen input'
        self.assertEqual(self.api('/api/library/create', changed, 409)['error']['code'], 'operationConflict')
        self.assertEqual(style.read_bytes(), request['options']['initialStyle']['source'].encode())
        self.restart_same_state()
        self.assertEqual(library_fact(self.api('/api/library/create', request)), library_fact(receipt))
        doc = self.note(root, '初始化语言', 'English colour. #text(lang: "zh", region: "CN")[中文局部]\n')
        current = self.api('/api/styles/read', {'sessionId': doc['sessionId']})
        self.assertEqual(current['source'], request['options']['initialStyle']['source'])
        self.assertEqual({c['key']: c['value'] for c in current['controls']}['text.region'], 'none')
        result = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        self.assertTrue(result['ok'], result.get('diagnostics'))
        langs = {n.get('attrs', {}).get('lang') for n in local_harness.walk(result['tree'])}
        self.assertIn('en', langs)
        self.assertIn('zh-CN', langs)
        self.assertNotIn('en-CN', langs)

    def test_v061_library_creation_cancellation_fences_unknown_invalid_and_rejected_inputs(self):
        for kind in ('unknown', 'invalid-path', 'occupied'):
            with self.subTest(kind=kind):
                request = self.creation_request('en', 'GB')
                root = Path(request['path'])
                original = None
                if kind == 'invalid-path':
                    request['path'] = 'relative-original-creation-request'
                    self.assertEqual(self.api('/api/library/create', request, 409)['error']['code'], 'invalidPath')
                elif kind == 'occupied':
                    root.mkdir()
                    original = root / 'original-user-content.txt'
                    original.write_bytes(b'preserve exact original user bytes\x00')
                    self.assertEqual(self.api('/api/library/create', request, 409)['error']['code'], 'exists')
                ended = self.api('/api/library/create-cancel', request)
                self.assertEqual(ended['operationId'], request['operationId'])
                self.assertEqual(ended['status'], 'resolved')
                journal = self.data_dir / 'journal'
                terminal_path = journal / (request['operationId'] + '.resolution.json')
                terminal_bytes = terminal_path.read_bytes()
                terminal = json.loads(terminal_bytes)
                self.assertEqual(set(terminal), {'operationId', 'action', 'attempt'})
                self.assertEqual(terminal['operationId'], request['operationId'])
                self.assertEqual(terminal['action'], 'cancelPrepared')
                self.assertNotIn('message', terminal)
                self.assertEqual(set(terminal['attempt']), {'requestKind', 'wireDigest'})
                self.assertEqual(terminal['attempt']['requestKind'], 'createLibrary')
                self.assertIsNotNone(re.fullmatch(r'[0-9a-f]{64}', terminal['attempt']['wireDigest']))
                self.assertEqual(list(journal.glob(request['operationId'] + '.*.json')), [terminal_path])
                self.restart_same_state()
                self.assertEqual(self.api('/api/recovery/inspect', {'operationId': request['operationId']})['status'], 'resolved')
                self.assertEqual(self.api('/api/library/create', request, 409)['error']['code'], 'operationCanceled')
                self.assertEqual(self.api('/api/library/create-cancel', request)['status'], 'resolved')
                changed = copy.deepcopy(request)
                changed['options']['initialStyle']['enabled'] = False
                self.assertEqual(self.api('/api/library/create-cancel', changed, 409)['error']['code'], 'operationConflict')
                self.assertEqual(terminal_path.read_bytes(), terminal_bytes)
                self.assertEqual(list(journal.glob(request['operationId'] + '.*.json')), [terminal_path])
                if original:
                    self.assertEqual(original.read_bytes(), b'preserve exact original user bytes\x00')
                    self.assertEqual(list(root.iterdir()), [original])
                else:
                    self.assertFalse(root.exists())

    def test_v061_library_creation_committed_cancel_and_replaced_root_preserve_objects(self):
        request = self.creation_request('zh', 'TW')
        root = Path(request['path'])
        receipt = self.api('/api/library/create', request)
        original = {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob('*') if p.is_file()}
        self.assertEqual(self.api('/api/library/create-cancel', request)['status'], 'committed')
        self.restart_same_state()
        self.assertEqual(self.api('/api/library/create-cancel', request)['status'], 'committed')
        retained = root.with_name(root.name + '-retained-original')
        root.rename(retained)
        shutil.copytree(retained, root)
        self.assertNotEqual(root.stat().st_ino, retained.stat().st_ino)
        unavailable = self.api('/api/library/create', request)
        self.assertEqual(library_fact(unavailable), library_fact(receipt))
        self.assertIn('deliveryError', unavailable)
        self.assertNotIn('library', unavailable)
        self.assertNotIn('rootDocument', unavailable)
        self.assertEqual({p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob('*') if p.is_file()}, original)
        self.assertEqual({p.relative_to(retained).as_posix(): p.read_bytes() for p in retained.rglob('*') if p.is_file()}, original)

    def test_v061_processing_original_operation_restart_and_unknown_members(self):
        root, _ = self.library()
        doc = self.note(root, '处理设置与草稿', '#set text(lang: "zh", region: "CN")\n原稿正文😀')
        original = Path(doc['path']).read_bytes()
        draft = doc['source'] + '\n独立的个人未保存草稿'
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': draft})
        settings = root / '.weftext-settings.json'
        unknown = {'version': 1, 'customExtension': {'keep': ['未知', 7, {'nested': True}]},
                   'customLocale': 'zh-Hant-HK'}
        retained_json(settings, unknown)
        original_settings = settings.read_bytes()
        before = self.processing_read(root)
        self.assertIsNone(before['processing'])
        self.assertEqual(before['beforeSource'].encode(), original_settings)
        request = self.processing_request(root, before=before)
        self.assertEqual(self.api('/api/processing/status', request)['status'], 'notRegistered')
        self.api('/api/processing/prepare', {**request, 'baseId': local_harness.uid()}, 409)
        plan = self.api('/api/processing/prepare', request)
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(settings.read_bytes(), original_settings)
        self.assertEqual(self.api('/api/processing/prepare', request)['planId'], plan['planId'])
        self.assertEqual(self.api('/api/processing/status', request)['planId'], plan['planId'])
        self.api('/api/processing/prepare', {**request, 'processing': processing('SG')}, 409)
        self.api('/api/processing/status', {**request, 'beforeSource': request['beforeSource'] + ' '}, 409)
        other_root, _ = self.library()
        self.api('/api/processing/prepare', {**request, 'root': str(other_root)}, 409)
        self.assertFalse((other_root / '.weftext-settings.json').exists())
        # A registered retry may reject a changed transient baseId or return
        # its exact old plan. It must never recapture a new effect/authority.
        code, raw, _ = self.raw('/api/processing/prepare',
            {**request, 'baseId': local_harness.uid()}, status=(200, 409))
        if code == 200:
            retried = json.loads(raw)
            self.assertEqual(retried['planId'], plan['planId'])
            self.assertEqual(retried['preview'], plan['preview'])
        self.processing_apply(request, {**plan, 'planId': local_harness.uid()}, 409)
        self.assertEqual(settings.read_bytes(), original_settings)
        self.restart_same_state()
        recovered = self.api('/api/processing/status', request)
        self.assertEqual(recovered['planId'], plan['planId'])
        self.assertEqual(recovered, plan)
        receipt = self.processing_apply(request, recovered)
        self.assertEqual(receipt['status'], 'committed')
        self.assertEqual(self.processing_apply(request, recovered), receipt)
        self.assertEqual(self.api('/api/processing/prepare', request)['status'], 'committed')
        after = json.loads(settings.read_text('utf-8'))
        self.assertEqual(after['processing'], processing())
        for key, value in unknown.items():
            self.assertEqual(after[key], value)
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        opened = self.api('/api/open', {'path': doc['path'], 'root': str(root)})
        self.assertEqual(opened['recoveredDraft'], draft)
        self.restart_same_state()
        self.assertEqual(self.api('/api/processing/status', request)['status'], 'committed')
        self.assertEqual(self.api('/api/open', {'path': doc['path'], 'root': str(root)})['recoveredDraft'], draft)

    def test_v061_processing_durable_cancel_unknown_prepared_and_stale_requests(self):
        for stage in ('unknown', 'prepared', 'stale'):
            with self.subTest(stage=stage):
                root, _ = self.library()
                request = self.processing_request(root)
                plan = None
                if stage == 'prepared':
                    plan = self.api('/api/processing/prepare', request)
                if stage == 'stale':
                    retained_json(root / '.weftext-settings.json', {'version': 1, 'external': '保留外部改动'})
                    self.api('/api/processing/prepare', request, 409)
                before = (root / '.weftext-settings.json').read_bytes() if (root / '.weftext-settings.json').exists() else None
                ended = self.api('/api/processing/settle', request)
                self.assertEqual(ended['status'], 'cancelled')
                self.restart_same_state()
                late = self.api('/api/processing/prepare', request)
                self.assertEqual(late['status'], 'cancelled')
                self.assertEqual(self.api('/api/processing/status', request)['status'], 'cancelled')
                if plan:
                    self.assertEqual(self.processing_apply(request, plan)['status'], 'cancelled')
                current = (root / '.weftext-settings.json').read_bytes() if (root / '.weftext-settings.json').exists() else None
                self.assertEqual(current, before)

    def test_v061_processing_settle_keeps_committed_original_and_rejects_changed_effect(self):
        root, _ = self.library()
        request = self.processing_request(root)
        plan = self.api('/api/processing/prepare', request)
        receipt = self.processing_apply(request, plan)
        self.assertEqual(receipt['status'], 'committed')
        path = root / '.weftext-settings.json'
        original = path.read_bytes()
        self.assertEqual(self.api('/api/processing/settle', request)['status'], 'committed')
        self.api('/api/processing/settle', {**request, 'processing': processing('GB')}, 409)
        self.restart_same_state()
        self.assertEqual(self.api('/api/processing/settle', request)['status'], 'committed')
        self.assertEqual(path.read_bytes(), original)

    def test_v061_processing_stale_settings_and_same_bytes_file_identity(self):
        for stage in ('before-prepare', 'after-prepare'):
            for replacement in ('changed-bytes', 'same-bytes-new-file'):
                with self.subTest(stage=stage, replacement=replacement):
                    root, _ = self.library()
                    path = root / '.weftext-settings.json'
                    retained_json(path, {'version': 1, 'external': '原始设置'})
                    original = path.read_bytes()
                    request = self.processing_request(root)
                    plan = self.api('/api/processing/prepare', request) if stage == 'after-prepare' else None
                    if replacement == 'changed-bytes':
                        retained_json(path, {'version': 1, 'external': '用户在外部改过'})
                    else:
                        # Preserve the old physical file as evidence; the replacement
                        # has identical bytes but a different object identity.
                        retained = root / 'settings.retained-original.json'
                        path.rename(retained)
                        path.write_bytes(original)
                        self.assertEqual(retained.read_bytes(), original)
                        self.assertNotEqual(path.stat().st_ino, retained.stat().st_ino)
                    external = path.read_bytes()
                    if plan is None:
                        self.api('/api/processing/prepare', request, 409)
                    else:
                        self.processing_apply(request, plan, 409)
                    self.assertEqual(path.read_bytes(), external)
                    self.assertNotIn('processing', json.loads(external))

    def test_v061_processing_invalid_input_and_explicit_reset_preserve_originals(self):
        root, _ = self.library()
        path = root / '.weftext-settings.json'
        for source in ('not json', '[]', '{"version":2}'):
            with self.subTest(source=source):
                path.write_text(source, encoding='utf-8')
                if source == 'not json':
                    self.api('/api/processing/read', {'root': str(root)}, 400)
                else:
                    before = self.processing_read(root)
                    self.assertIsInstance(before['warning'], str)
                    self.api('/api/processing/prepare', self.processing_request(root, before=before), 400)
                self.assertEqual(path.read_text('utf-8'), source)
        retained_json(path, {'version': 1, 'custom': {'keep': '未知成员'}, 'processing': {'futureVersion': 999}})
        before = self.processing_read(root)
        self.assertIsInstance(before['warning'], str)
        self.assertEqual(before['processing'], {'futureVersion': 999})
        invalid = processing()
        invalid['defaults']['languages'][0]['region'] = 'not-a-region'
        self.api('/api/processing/prepare', self.processing_request(root, invalid, before), 400)
        self.assertEqual(self.processing_read(root)['beforeSource'], before['beforeSource'])
        reset = {**self.processing_request(root, before=before), 'processing': None}
        plan = self.api('/api/processing/prepare', reset)
        self.assertEqual(self.processing_apply(reset, plan)['status'], 'committed')
        self.assertEqual(json.loads(path.read_text('utf-8')), {'version': 1, 'custom': {'keep': '未知成员'}})

    def test_v061_processing_legacy_unscoped_dictionary_requires_explicit_confirmation(self):
        root, _ = self.library()
        doc = self.note(root, '旧词表保留', 'LegacyTypoToken ([)]\n')
        original_source = Path(doc['path']).read_bytes()
        options = {'characters': True, 'typos': True, 'pairs': True, 'sensitive': False,
            'rules': [{'id': 'owned.legacy.unscoped', 'version': 'old-version', 'category': 'typo',
                'text': 'LegacyTypoToken', 'message': 'retained old rule', 'replacement': 'ReviewedToken'}],
            'allowlist': [')'], 'selection': None}
        exact_legacy = copy.deepcopy(options)
        context = self.context(doc)
        self.assertTrue(self.api('/api/render', {'sessionId': doc['sessionId'],
            'source': doc['source'], 'serial': 1})['ok'])
        legacy = self.api('/api/knowledge/proofread', {'context': context, 'options': options})
        self.assertEqual(legacy['coverage'], 'partial')
        self.assertTrue(any('确认' in warning for warning in legacy['warnings']))
        self.assertFalse(any(item['ruleId'] == 'owned.legacy.unscoped' for item in legacy['findings']))
        self.assertTrue(any(['owned.legacy.unscoped', 'unknown'] in item['skippedRules']
                            for item in legacy['fragments']))
        self.assertTrue(any(item['ruleId'] == 'builtin.crossed-pair' for item in legacy['findings']))
        self.assertEqual(options, exact_legacy)
        confirmed = copy.deepcopy(options)
        confirmed['rules'][0]['scope'] = {'universal': True}
        confirmed['allowlist'] = []
        confirmed['scopedAllowlist'] = [{'text': ')', 'scope': {'universal': True}}]
        after = self.api('/api/knowledge/proofread', {'context': context, 'options': confirmed})
        self.assertTrue(any(item['ruleId'] == 'owned.legacy.unscoped' for item in after['findings']))
        self.assertFalse(any(item['ruleId'] == 'builtin.crossed-pair' for item in after['findings']))
        self.assertEqual(Path(doc['path']).read_bytes(), original_source)
        self.assertFalse((Path(doc['path']).parent / 'weftext.annotations.json').exists())

    def test_v061_local_proposal_preserves_original_editor_version_render_cache_and_draft(self):
        doc = self.ordinary('= 原始标题\n\n原始正文😀\n')
        source = doc['source'] + '\n个人未保存输入'
        original = Path(doc['path']).read_bytes()
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': source})
        basis = {'sessionId': doc['sessionId'], 'source': source, 'serial': 17}
        mapping = self.api('/api/editor-map', basis)
        rendered = self.api('/api/render', basis)
        self.assertTrue(rendered['ok'], rendered.get('diagnostics'))
        proposal = source + '\n$ frac(1, 2) $\n提案独有文字'
        preview = self.api('/api/editor/preview', {**basis, 'proposalSource': proposal})
        self.assertTrue(preview['ok'], preview.get('diagnostics'))
        self.assertEqual((preview['source'], preview['serial']), (proposal, 17))
        region = next(item for item in mapping['literalRegions'] if '原始正文' in item['text'])
        edited = self.api('/api/edit', {**basis, 'sourceHash': mapping['sourceHash'],
            'intent': {'kind': 'replaceLiteral', 'range': region['range'], 'text': '仍可编辑原始范围'}})
        self.assertIn('仍可编辑原始范围', edited['source'])
        self.assertNotIn('提案独有文字', edited['source'])
        cached = self.api('/api/edit-literal', {**basis, 'nodeKey': '@body', 'text': '缓存仍属于原稿'})
        self.assertIn('缓存仍属于原稿', cached['source'])
        self.assertNotIn('提案独有文字', cached['source'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        self.assertEqual(self.api('/api/open', {'path': doc['path'], 'root': doc['root']})['recoveredDraft'], source)

    def test_v061_local_late_proposal_cannot_restore_stale_source(self):
        doc = self.ordinary('原稿正文\n')
        basis = {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 21}
        self.api('/api/editor-map', basis)
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.raw, '/api/editor/preview',
                {**basis, 'proposalSource': basis['source'] + SLOW_PROPOSAL}, (200, 409))
            time.sleep(.2)
            current = {**basis, 'serial': 22, 'source': basis['source'] + '\n后来输入'}
            mapping = self.api('/api/editor-map', current)
            code, raw, _ = pending.result(timeout=75)
        self.assertEqual(code, 409, raw.decode(errors='replace')[:1500])
        self.assertEqual(json.loads(raw)['error']['code'], 'stale_draft')
        region = next(item for item in mapping['literalRegions'] if '后来输入' in item['text'])
        result = self.api('/api/edit', {**current, 'sourceHash': mapping['sourceHash'],
            'intent': {'kind': 'replaceLiteral', 'range': region['range'], 'text': '当前草稿仍可改'}})
        self.assertIn('当前草稿仍可改', result['source'])
        self.assertEqual(Path(doc['path']).read_text('utf-8'), doc['source'])


class LoggedHostedServer(hosted_harness.Server):
    def __init__(self, args, prefix):
        self.log_prefix = prefix
        self.closed = False
        super().__init__(args, hosted=True)

    def close(self):
        if self.closed:
            return
        stdout, stderr = super().close()
        self.closed = True
        self.log_prefix.with_suffix('.stdout.log').write_text(stdout, encoding='utf-8')
        self.log_prefix.with_suffix('.stderr.log').write_text(stderr, encoding='utf-8')


class HostedProcessing(HostedLibrary):
    @classmethod
    def setUpClass(cls):
        cls.evidence = Path(tempfile.mkdtemp(prefix='weftext-v061-hosted-processing-', dir=ARTIFACT_PARENT))
        print(f'Hosted processing fixtures (retained): {cls.evidence}', flush=True)

    def start(self):
        retained_json(self.config_path, self.config)
        self.start_number = getattr(self, 'start_number', 0) + 1
        self.server = LoggedHostedServer([ '--data-dir', str(self.data),
            '--server-config', str(self.config_path)], self.folder / f'hosted-{self.start_number:02d}')

    def test_v061_hosted_processing_settings_use_original_role_operation_and_personal_draft(self):
        self.grant_library(roles=('settings',))
        originals = {name: (self.root / name / (name + '.typ')).read_bytes() for name in self.docs}
        doc = self.open()
        draft = doc['source'] + '\n个人草稿不属于处理设置'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': draft})
        before = self.library_read('settings')
        changed = json.loads(before['source'])
        changed.update(processing=processing(), customExtension={'keep': ['未知', 7]})
        source = json.dumps(changed, ensure_ascii=False)
        plan, request = self.library_prepare('settings', source, basis=before)
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(self.call('library/prepare', request)['planId'], plan['planId'])
        self.call('library/prepare', {**request, 'source': source + ' '}, status=409)
        self.server.close()
        self.start()
        operation = self.call('library/operation', {'role': 'settings', 'operationId': request['operationId']})
        self.assertEqual(operation['planId'], plan['planId'])
        self.assertEqual(self.library_commit(operation)['status'], 'committed')
        self.assertEqual(self.call('library/prepare', request)['status'], 'committed')
        self.assertEqual((self.root / '.weftext-settings.json').read_text('utf-8'), source)
        self.assertEqual(json.loads(self.library_read('settings')['source'])['customExtension'], changed['customExtension'])
        self.assertEqual(self.open()['recoveredDraft'], draft)
        for name, original in originals.items():
            self.assertEqual((self.root / name / (name + '.typ')).read_bytes(), original)
        self.grant_library(roles=('settings',), actor='bob')
        self.assertEqual(self.call('library/operation', {'role': 'settings', 'operationId': request['operationId']}, 'bob')['status'], 'notRegistered')
        current = self.library_read('settings')
        invalid = copy.deepcopy(changed)
        invalid['processing']['schemaVersion'] = 999
        self.call('library/prepare', {'role': 'settings', 'operationId': hosted_harness.uid(),
            'basis': current['basis'], 'source': json.dumps(invalid)}, status=400)
        self.assertEqual(self.library_read('settings')['source'], source)

    def test_v061_hosted_proposal_preserves_original_serial_and_exact_edit(self):
        doc = self.open()
        original = (self.root / 'Shared' / 'Shared.typ').read_bytes()
        source = doc['source'] + '\n原始可编辑范围😀'
        basis = {'sessionId': doc['sessionId'], 'source': source, 'serial': 27}
        self.call('draft', {'sessionId': doc['sessionId'], 'source': source})
        mapping = self.call('editor/map', basis)
        self.assertTrue(self.call('editor/render', basis)['ok'])
        proposal = source + '\n$ frac(1, 2) $\n只存在于提案'
        preview = self.call('editor/preview', {**basis, 'proposalSource': proposal})
        self.assertTrue(preview['ok'], preview)
        self.assertNotIn('diagnostics', preview)
        self.assertEqual((preview['source'], preview['serial']), (proposal, 27))
        self.assertNotIn(str(self.root), json.dumps(preview))
        self.assertFalse(any(node.get('writable') for node in local_harness.walk(preview['tree'])))
        region = next(item for item in mapping['literalRegions'] if '原始可编辑范围' in item['text'])
        edit = {**basis, 'sourceHash': mapping['sourceHash'], 'intent': {
            'kind': 'replaceLiteral', 'range': region['range'], 'text': '仍然编辑原稿'}}
        result = self.call('editor/edit', edit)
        self.assertIn('仍然编辑原稿', result['source'])
        self.assertNotIn('只存在于提案', result['source'])
        self.assertEqual(self.call('editor/map', basis)['sourceHash'], mapping['sourceHash'])
        self.call('editor/preview', {**basis, 'proposalSource': proposal}, 'bob', 410)
        self.assertEqual((self.root / 'Shared' / 'Shared.typ').read_bytes(), original)
        self.assertEqual(self.open()['recoveredDraft'], source)

    def test_v061_hosted_proposal_keeps_private_style_diagnostics_unavailable(self):
        for expected, expression in ((True, 'set text(fill: gradient.linear(red, blue))'),
                                     (False, 'panic("PRIVATE_STYLE_DIAGNOSTIC_SENTINEL")')):
            with self.subTest(expected=expected):
                self.grant_library(roles=('style',))
                style = '// PRIVATE_STYLE_SOURCE_SENTINEL\n#let library-style(body) = { ' + expression + '; body }'
                plan, _ = self.library_prepare('style', style, enabled=True)
                self.assertEqual(self.library_commit(plan)['status'], 'committed')
                self.grant_library(roles=(), style_use=True)
                self.call('library/read', {'role': 'style'}, status=404)
                doc = self.open()
                basis = {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 51}
                self.call('editor/map', basis)
                preview = self.call('editor/preview', {**basis, 'proposalSource': basis['source'] + '\n普通提案'})
                self.assertEqual(preview['ok'], expected, preview)
                self.assertNotIn('diagnostics', preview)
                encoded = json.dumps(preview, ensure_ascii=False).replace('\\\\', '\\')
                self.assertNotIn(str(self.root), encoded)
                self.assertNotIn(str(self.data), encoded)
                self.assertNotIn('PRIVATE_STYLE_SOURCE_SENTINEL', encoded)
                self.assertNotIn('PRIVATE_STYLE_DIAGNOSTIC_SENTINEL', encoded)

    def test_v061_hosted_late_proposal_rechecks_source_and_revoked_session(self):
        doc = self.open()
        basis = {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 31}
        self.call('editor/map', basis)
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.server.call, '/api/hosted/editor/preview',
                {**basis, 'proposalSource': basis['source'] + SLOW_PROPOSAL}, self.tokens['alice'])
            time.sleep(.2)
            self.call('editor/map', {**basis, 'serial': 32, 'source': basis['source'] + '\n新草稿'})
            code, result, _ = pending.result(timeout=45)
        self.assertEqual(code, 409, result)
        self.assertEqual(result['error']['code'], 'stale_draft')
        current = {**basis, 'serial': 33, 'source': basis['source'] + '\n当前获权草稿'}
        self.call('editor/map', current)
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.server.call, '/api/hosted/editor/preview',
                {**current, 'proposalSource': current['source'] + SLOW_PROPOSAL}, self.tokens['alice'])
            time.sleep(.2)
            self.policy(lambda users: users['alice']['grants']['Shared'].update(write=False))
            code, result, _ = pending.result(timeout=45)
        self.assertEqual(code, 410, result)
        self.call('editor/preview', {**current, 'proposalSource': current['source'] + '\n迟到提案'}, status=410)
        self.assertEqual((self.root / 'Shared' / 'Shared.typ').read_text('utf-8'), doc['source'])


CASES = (LocalProcessing, HostedProcessing)


def new_tests():
    return [(case, name) for case in CASES for name in sorted(vars(case))
            if name.startswith('test_v061_')]


def suite(only=()):
    return unittest.TestSuite(case(name) for case, name in new_tests()
        if not only or name in only or f'{case.__name__}.{name}' in only)


def load_tests(_loader, _tests, _pattern):
    """Also keep unittest discovery from collecting any inherited tests."""
    return suite()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path, nargs='?')
    parser.add_argument('--executable', type=Path, dest='executable_option')
    parser.add_argument('--artifacts-dir', type=Path)
    parser.add_argument('--only', action='append', default=[], metavar='TEST')
    parser.add_argument('--list', action='store_true', help='List only the new tests without starting any executable')
    args = parser.parse_args()
    names = {name for _, name in new_tests()} | {f'{case.__name__}.{name}' for case, name in new_tests()}
    if any(name not in names for name in args.only):
        parser.error('Unknown --only name; use --list for the focused tests')
    if args.list:
        print('\n'.join(f'{case.__name__}.{name}' for case, name in new_tests()))
        return 0
    if bool(args.executable) == bool(args.executable_option):
        parser.error('Supply exactly one newly built executable, positional or --executable')
    executable = (args.executable or args.executable_option).resolve(strict=True)
    global ARTIFACT_PARENT
    ARTIFACT_PARENT = args.artifacts_dir.resolve(strict=True) if args.artifacts_dir else None
    local_harness.EXE = executable
    local_harness.ARTIFACT_PARENT = ARTIFACT_PARENT
    hosted_harness.EXE = executable
    result = unittest.TextTestRunner(verbosity=2).run(suite(args.only))
    if hasattr(LocalProcessing, 'root'):
        with executable.open('rb') as executable_file:
            executable_hash = hashlib.file_digest(executable_file, 'sha256').hexdigest()
        retained_json(LocalProcessing.root / 'v061-processing-results.json', {
            'executable': str(executable),
            'sha256': executable_hash,
            'version': subprocess.check_output([str(executable), '--version'], text=True,
                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0)).strip(),
            'selectedTests': args.only or [f'{case.__name__}.{name}' for case, name in new_tests()],
            'testsRun': result.testsRun,
            'successful': result.wasSuccessful(),
            'hostedFixtures': str(getattr(HostedProcessing, 'evidence', 'not run')),
            'failures': [{'test': str(test), 'traceback': trace} for test, trace in result.failures],
            'errors': [{'test': str(test), 'traceback': trace} for test, trace in result.errors]})
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    sys.exit(main())
