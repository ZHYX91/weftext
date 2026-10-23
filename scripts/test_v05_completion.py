"""Real HTTP acceptance for the v0.5 local timeline and body Query adapters.

Only newly owned disposable libraries are used. Fixtures, journal witnesses and
redacted server logs are retained, including failures. No native window, host
input or clipboard is used. These checks establish HTTP and exact-source
behavior; they do not establish the LHT/IQV browser interaction clauses,
five-view controls, focus, touch, keyboard, Undo or late browser result handling.
"""
from _cli_serve import launch_serve
import argparse
import copy
import hashlib
import io
import json
from pathlib import Path
import sys
import time
import unittest
import zipfile
import xml.etree.ElementTree as ET

import test_local_features as features


# Separate Python/Rust system-clock observations on Windows can differ by a few
# milliseconds in resolution. This bounds that observation error; the timeline
# must still report the exact timestamp and sequence from its original commit.
CLOCK_OBSERVATION_TOLERANCE_MS = 50


class V05Completion(features.LocalFeatures):
    def reopen(self, doc, root):
        return self.api('/api/open', {'path': doc['path'], 'root': str(root)})

    def timeline(self, doc):
        return self.api('/api/history/list', {'sessionId': doc['sessionId']})

    def history_read(self, doc, version_id, status=200):
        return self.api('/api/history/read', {'sessionId': doc['sessionId'],
                        'versionId': version_id}, status=status)

    def history_versions(self, name='历史正文'):
        root, library = self.library()
        doc = self.note(root, name)
        metadata = doc['source'].split('\n', 1)[0]
        versions = []
        for index in range(1, 4):
            source = metadata + f'\r\n\r\n= 保存 v{index}🌿\r\n\r\n准确正文 {index}。\r\n'
            request = {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                       'operationId': features.uid(), 'source': source}
            start = int(time.time() * 1000)
            receipt = self.api('/api/save', request)
            end = int(time.time() * 1000)
            self.assertEqual(receipt['status'], 'saved')
            self.assertEqual(Path(doc['path']).read_bytes(), source.encode())
            versions.append({'id': receipt['operationId'], 'revision': receipt['baseId'],
                             'source': source, 'start': start, 'end': end})
            self.assertNotIn('documentError', receipt)
            self.assertTrue(receipt['documentSourceMatchesCommit'])
            current = receipt['document']
            self.assertEqual(current['sessionId'], doc['sessionId'])
            self.assertEqual(current['source'], source)
            self.assertEqual(current['baseId'], receipt['baseId'])
            doc = current
        return root, library, doc, versions

    def restore_request(self, doc, version_id, draft_source=None, action='none'):
        return {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                'versionId': version_id, 'operationId': features.uid(),
                'draftAction': action, 'draftSource': draft_source}

    def install_schema(self, root):
        before = self.api('/api/data/schema', {'root': str(root)})
        preview = self.api('/api/data/schema/prepare', {
            'root': str(root), 'operationId': features.uid(), 'source': before['source'],
            'beforeSource': before['source'], 'beforeRevision': before['revision']})
        result = self.api('/api/data/schema/install', {
            'operationId': preview['operationId'], 'planId': preview['planId']})
        self.assertEqual(result['status'], 'committed')
        return self.api('/api/data/schema', {'root': str(root)})

    def definition(self, doc, plan, label, key=None, *, serial, next_serial):
        self.assertGreater(next_serial, serial)
        result = self.api('/api/query/definition/save', {
            **self.snapshot(doc, serial=serial), 'baseId': doc['baseId'],
            'operationId': features.uid(), 'key': key, 'label': label, 'plan': plan})
        self.assertEqual(Path(doc['path']).read_bytes(), result['document']['source'].encode())
        self.assertEqual(result['document']['sessionId'], doc['sessionId'])
        self.map_source(result['document'], result['document']['source'], serial=next_serial)
        return result

    @staticmethod
    def status_plan(status='open'):
        return {'types': ['task'], 'where': f"status == '{status}'",
                'select': [{'name': 'status', 'expr': 'status'}]}

    def query_fixture(self, multiple=False):
        root, library = self.library()
        self.install_schema(root)
        task, _ = self.capture(root, '任务🌿', ['task'], {'tasks/status': features.text('open')})
        self.capture(root, '已完成任务', ['task'], {'tasks/status': features.text('done')})
        owner = self.note(root, '查询正文', '= 查询🌿\n\n前文不可丢。\n')
        first = self.definition(owner, self.status_plan(), 'weftext-query-first', serial=0, next_serial=1)
        owner = first['document']
        if multiple:
            second = self.definition(owner, self.status_plan('done'), 'weftext-query-second', serial=1, next_serial=2)
            owner = second['document']
        return root, library, task, owner, first

    def discover(self, doc, source=None, *, serial):
        result = self.api('/api/query/embedded/discover', self.snapshot(doc, source, serial))
        self.assertEqual(result['sourceHash'], features.sha(result['source']))
        self.assertEqual(result['serial'], serial)
        for occurrence in result['occurrences']:
            self.assert_range(result['source'], occurrence['range'])
        return result

    def run_embedded(self, doc, source, occurrence_id, serial, status=200):
        return self.api('/api/query/embedded/run', {
            **self.snapshot(doc, source, serial), 'occurrenceId': occurrence_id}, status=status)

    def edit_embedded(self, doc, source, occurrence_id, serial, status=200, **change):
        return self.api('/api/query/embedded/edit', {
            **self.snapshot(doc, source, serial), 'occurrenceId': occurrence_id, **change},
            status=status)

    @staticmethod
    def occurrence(discovery, label):
        return next(item for item in discovery['occurrences'] if item['label'] == label)

    def assert_result_parity(self, embedded, direct):
        # Run/cut/output handles differ. These fields describe actual values and
        # the executor's coverage/state/access conclusions for the same input.
        for field in ['rows', 'coverage', 'freshness', 'diagnostics', 'totalKnown',
                      'limited', 'aggregateExact', 'runtime', 'snapshotMode',
                      'externalPending', 'rawScope', 'access']:
            self.assertEqual(embedded[field], direct[field], field)

    @classmethod
    def restart_owned_server(cls):
        """Drain the exact owner, then restart against its private journal."""
        cls.owner.stop()
        cls.owner = launch_serve(features.EXE, ['--data-dir', str(cls.data_dir)], log_prefix=cls.root / 'server')
        cls.process = cls.owner.process
        cls.url, cls.token = cls.owner.ready['base'], cls.owner.ready['token']

    def test_01_new_endpoints_require_token_and_same_origin(self):
        paths = ['/api/history/list', '/api/history/read', '/api/history/compare',
                 '/api/history/restore', '/api/history/status',
                 '/api/query/embedded/discover', '/api/query/embedded/run',
                 '/api/query/embedded/edit']
        before = {path: path.read_bytes() for path in self.data_dir.rglob('*.json')}
        for path in paths:
            with self.subTest(path=path):
                for headers, token in [({}, False), ({'X-Weftext-Token': 'invalid'}, True),
                                       ({'Origin': 'https://example.invalid'}, True)]:
                    refusal = self.api(path, {}, status=403, headers=headers, token=token)
                    self.assertEqual(refusal['error']['code'], 'session_token')
        self.assertEqual({path: path.read_bytes() for path in self.data_dir.rglob('*.json')}, before)

    def test_02_saved_versions_exact_time_read_compare_and_draft_preservation(self):
        root, library, doc, versions = self.history_versions()
        timeline = self.timeline(doc)
        self.assertEqual(timeline['nodeId'], doc['nodeId'])
        self.assertEqual(timeline['workspaceId'], library['workspaceId'])
        self.assertEqual(timeline['currentness'], 'saved')
        self.assertEqual(timeline['currentSavedVersionId'], versions[-1]['id'])
        indexed = {entry['versionId']: entry for entry in timeline['versions']}
        sequences = []
        for version in versions:
            row = indexed[version['id']]
            self.assertEqual(row['status'], 'available')
            self.assertEqual(row['revisionId'], version['revision'])
            self.assertEqual(row['sha256'], features.sha(version['source']))
            self.assertEqual(row['byteLength'], len(version['source'].encode()))
            self.assertEqual(row['timeBasis'], 'commitClock')
            commit_path = self.data_dir / 'journal' / f"{version['id']}.commit.json"
            commit = json.loads(commit_path.read_bytes())
            self.assertEqual(row['savedAtUnixMs'], commit['savedAtUnixMs'])
            self.assertEqual(row['commitSequence'], commit['commitSequence'])
            self.assertGreaterEqual(row['savedAtUnixMs'],
                                    version['start'] - CLOCK_OBSERVATION_TOLERANCE_MS)
            self.assertLessEqual(row['savedAtUnixMs'],
                                 version['end'] + CLOCK_OBSERVATION_TOLERANCE_MS)
            sequences.append(row['commitSequence'])
            read = self.history_read(doc, version['id'])
            self.assertEqual(read['source'], version['source'])
            self.assertTrue(read['readOnly'])
            self.assertEqual(read['previewStatus'], 'sourceOnly')
        self.assertEqual(sequences, sorted(set(sequences)))
        draft = doc['source'] + '\n尚未保存的草稿🌿。\n'
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': draft})
        before = Path(doc['path']).read_bytes()
        compared = self.api('/api/history/compare', {
            'sessionId': doc['sessionId'], 'versionId': versions[0]['id']})
        self.assertEqual(compared['comparison'], 'currentSaved')
        self.assertEqual(compared['selected']['source'], versions[0]['source'])
        self.assertEqual(compared['current']['source'], versions[-1]['source'])
        self.assertFalse(compared['equal'])
        self.assertEqual(Path(doc['path']).read_bytes(), before)
        reopened = self.reopen(doc, root)
        self.assertEqual(reopened['recoveredDraft'], draft)
        after = self.timeline(reopened)
        self.assertEqual(after['versions'], timeline['versions'])
        self.assertNotIn(features.sha(draft), [row['sha256'] for row in after['versions']])

    def test_03_restore_requires_draft_protection_and_is_one_new_replayable_save(self):
        root, _, doc, versions = self.history_versions('恢复保护')
        persisted = doc['source'] + '\n已持久化但未保存的草稿。\n'
        buffer = persisted + '缓冲区更新🌿。\n'
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': persisted})
        before = self.timeline(doc)
        request = self.restore_request(doc, versions[0]['id'], buffer)
        refusal = self.api('/api/history/restore', request, status=409)
        self.assertEqual(refusal['error']['code'], 'draftProtectionRequired')
        self.assertEqual(Path(doc['path']).read_bytes(), versions[-1]['source'].encode())
        self.assertEqual(self.timeline(doc)['versions'], before['versions'])
        self.assertEqual(self.reopen(doc, root)['recoveredDraft'], persisted)
        request['draftAction'] = 'retain'
        envelope = self.api('/api/history/restore', request)
        self.assertEqual(envelope['status'], 'committed')
        self.assertEqual(envelope['operationId'], request['operationId'])
        restored = envelope['result']
        self.assertEqual(envelope['receipt'], restored['receipt'])
        self.assertEqual(restored['source'], versions[0]['source'])
        self.assertEqual(restored['versionId'], versions[0]['id'])
        self.assertEqual(restored['receipt']['status'], 'saved')
        self.assertEqual(restored['receipt']['operationId'], request['operationId'])
        self.assertNotEqual(restored['receipt']['baseId'], versions[0]['revision'])
        self.assertEqual({item['source'] for item in restored['protectedDrafts']}, {persisted, buffer})
        self.assertTrue(all(item['baseKnown'] for item in restored['protectedDrafts']))
        self.assertEqual(Path(doc['path']).read_bytes(), versions[0]['source'].encode())
        after = self.timeline(doc)
        self.assertEqual(len(after['versions']), len(before['versions']) + 1)
        self.assertEqual(after['currentSavedVersionId'], request['operationId'])
        for version in versions:
            self.assertEqual(self.history_read(doc, version['id'])['source'], version['source'])
        self.assertTrue({persisted, buffer}.issubset({item['source'] for item in after['protectedDrafts']}))
        # Simulate a lost response by querying and replaying the original request.
        status = self.api('/api/history/status', {'operationId': request['operationId']})
        self.assertEqual(status['status'], 'committed')
        self.assertEqual(status['result'], restored)
        self.assertEqual(self.api('/api/history/restore', request), envelope)
        self.assertEqual(self.timeline(doc)['versions'], after['versions'])
        changed_retry = self.api('/api/history/restore', {
            **request, 'versionId': versions[1]['id']}, status=409)
        self.assertEqual(changed_retry['error']['code'], 'operationConflict')
        self.assertEqual(Path(doc['path']).read_bytes(), versions[0]['source'].encode())
        stale_base = self.api('/api/history/restore', {
            **request, 'operationId': features.uid(), 'versionId': versions[1]['id']}, status=409)
        self.assertEqual(stale_base['error']['code'], 'staleBase')
        self.assertEqual(self.timeline(doc)['versions'], after['versions'])
        # Keep the original exact plan/effects and temporarily park its durable
        # decision witness. An uncertain original operation must be reported
        # and block new history access/retry effects until it is reconciled.
        commit = self.data_dir / 'journal' / f"{request['operationId']}.commit.json"
        parked_commit = commit.with_suffix('.json.owned-parked')
        commit.rename(parked_commit)
        try:
            pending = self.api('/api/history/status', {'operationId': request['operationId']})
            self.assertEqual(pending['status'], 'recoveryPending')
            self.assertNotIn('result', pending)
            refusal = self.api('/api/history/restore', request, status=409)
            self.assertEqual(refusal['error']['code'], 'recoveryPending')
            refusal = self.api('/api/history/list', {'sessionId': doc['sessionId']}, status=409)
            self.assertEqual(refusal['error']['code'], 'recoveryPending')
            self.assertEqual(Path(doc['path']).read_bytes(), versions[0]['source'].encode())
        finally:
            parked_commit.rename(commit)
        self.assertEqual(self.timeline(doc)['versions'], after['versions'])

    def test_04_restore_external_conflict_preserves_external_source_and_draft(self):
        root, _, doc, versions = self.history_versions('外部竞争')
        buffer = doc['source'] + '\n等待恢复时的本地草稿🌿。\n'
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': buffer})
        request = self.restore_request(doc, versions[0]['id'], buffer, 'retain')
        external = versions[-1]['source'] + '\n外部编辑的准确字节。\n'
        Path(doc['path']).write_bytes(external.encode())
        refusal = self.api('/api/history/restore', request, status=409)
        self.assertEqual(refusal['error']['code'], 'staleBase')
        self.assertEqual(Path(doc['path']).read_bytes(), external.encode())
        self.assertEqual(self.history_read(doc, versions[0]['id'])['source'], versions[0]['source'])
        timeline = self.timeline(doc)
        self.assertEqual(timeline['currentness'], 'external')
        self.assertIsNone(timeline['currentSavedVersionId'])
        self.assertNotIn(request['operationId'], [item['versionId'] for item in timeline['versions']])
        self.assertEqual(self.api('/api/history/status', {
            'operationId': request['operationId']})['status'], 'missing')
        compared = self.api('/api/history/compare', {
            'sessionId': doc['sessionId'], 'versionId': versions[0]['id']}, status=409)
        self.assertEqual(compared['error']['code'], 'currentSavedUnavailable')
        reopened = self.reopen(doc, root)
        self.assertTrue(reopened['recoveredDraftConflict'])
        self.assertEqual(reopened['recoveredDraft'], buffer)
        self.assertEqual(reopened['recoveredDraftBaseSource'], versions[-1]['source'])

    def test_05_history_identity_survives_application_move_restart_and_copy_isolation(self):
        root, _, doc, versions = self.history_versions('历史原名')
        original_ids = {item['versionId'] for item in self.timeline(doc)['versions']}
        renamed = self.commit(self.prepare(root, {'kind': 'renameNode',
            'source': str(Path(doc['path']).parent), 'name': '历史新名'}))
        renamed_path = Path(renamed['path']) / '历史新名.typ'
        parent = self.note(root, '历史容器')
        moved = self.commit(self.prepare(root, {'kind': 'moveNode',
            'source': str(renamed_path.parent), 'parent': str(Path(parent['path']).parent)}))
        moved_path = Path(moved['path']) / '历史新名.typ'
        self.restart_owned_server()
        moved_doc = self.api('/api/open', {'path': str(moved_path), 'root': str(root)})
        self.assertEqual(moved_doc['nodeId'], doc['nodeId'])
        self.assertEqual({item['versionId'] for item in self.timeline(moved_doc)['versions']}, original_ids)
        self.assertEqual(self.history_read(moved_doc, versions[0]['id'])['source'], versions[0]['source'])
        copied = self.commit(self.prepare(root, {'kind': 'copyNode',
            'source': str(moved_path.parent), 'parent': str(root), 'name': '历史新名'}))
        copy_doc = self.api('/api/open', {'path': str(Path(copied['path']) / '历史新名.typ'),
                                        'root': str(root)})
        self.assertNotEqual(copy_doc['nodeId'], moved_doc['nodeId'])
        copied_ids = {item['versionId'] for item in self.timeline(copy_doc)['versions']}
        self.assertTrue(original_ids.isdisjoint(copied_ids))
        denial = self.history_read(copy_doc, versions[0]['id'], status=409)
        self.assertEqual(denial['error']['code'], 'historyUnavailable')
        other_root, _ = self.library()
        same_name = self.note(other_root, '历史新名', '同名但独立。\n')
        self.assertTrue(original_ids.isdisjoint(
            item['versionId'] for item in self.timeline(same_name)['versions']))
        self.assertEqual(Path(moved_doc['path']).read_bytes(), versions[-1]['source'].encode())

    def test_06_retention_missing_exact_witness_and_unknown_or_rolled_back_clock(self):
        root, _, doc, versions = self.history_versions('保留范围')
        self.install_schema(root)
        original = Path(doc['path']).read_bytes()
        timeline = self.timeline(doc)
        self.assertFalse(timeline['retention']['complete'])
        self.assertEqual(timeline['retention']['knownVersions'], len(timeline['versions']))
        self.assertEqual(timeline['retention']['availableVersions'], len(timeline['versions']))
        # Preserve the actual witness under another owned filename, then restore
        # it. No record or fixture is permanently deleted.
        witness = self.data_dir / 'journal' / f"{versions[0]['id']}.observations.json"
        parked = witness.with_suffix('.json.owned-parked')
        witness.rename(parked)
        try:
            missing = self.timeline(doc)
            row = next(item for item in missing['versions'] if item['versionId'] == versions[0]['id'])
            self.assertEqual(row['status'], 'missing')
            self.assertIsNone(row['sha256'])
            self.assertEqual(missing['retention']['availableVersions'], len(timeline['versions']) - 1)
            refusal = self.history_read(doc, versions[0]['id'], status=409)
            self.assertEqual(refusal['error']['code'], 'historyUnavailable')
            self.assertEqual(self.history_read(doc, versions[1]['id'])['source'], versions[1]['source'])
        finally:
            parked.rename(witness)
        # Exact source remains readable when the original semantic config is
        # absent; the API reports sourceOnly instead of substituting current.
        schema = root / '.weftext-schema.json'
        parked_schema = root / '.weftext-schema.json.owned-parked'
        schema.rename(parked_schema)
        try:
            read = self.history_read(doc, versions[0]['id'])
            self.assertEqual(read['source'], versions[0]['source'])
            self.assertEqual(read['previewStatus'], 'sourceOnly')
        finally:
            parked_schema.rename(schema)
        commits = [self.data_dir / 'journal' / f"{version['id']}.commit.json" for version in versions]
        exact_commits = [path.read_bytes() for path in commits]
        try:
            records = [json.loads(value) for value in exact_commits]
            sequences = [record['commitSequence'] for record in records]
            records[0]['savedAtUnixMs'] = 2000
            records[1]['savedAtUnixMs'] = 1000
            records[2]['savedAtUnixMs'] = 1000
            for path, record in zip(commits, records):
                path.write_text(json.dumps(record), encoding='utf-8')
            rows = [item for item in self.timeline(doc)['versions'] if item['versionId'] in
                    {version['id'] for version in versions}]
            self.assertEqual([row['versionId'] for row in rows],
                             [versions[0]['id'], versions[2]['id'], versions[1]['id']])
            self.assertEqual({row['versionId']: row['commitSequence'] for row in rows},
                             {version['id']: sequence for version, sequence in zip(versions, sequences)})
            records[0].pop('savedAtUnixMs')
            commits[0].write_text(json.dumps(records[0]), encoding='utf-8')
            unknown_time = next(item for item in self.timeline(doc)['versions']
                          if item['versionId'] == versions[0]['id'])
            self.assertEqual(unknown_time['timeBasis'], 'unknown')
            self.assertIsNone(unknown_time['savedAtUnixMs'])
            self.assertEqual(unknown_time['commitSequence'], sequences[0])
            self.assertEqual(self.history_read(doc, versions[0]['id'])['source'], versions[0]['source'])
            records[0].pop('commitSequence')
            commits[0].write_text(json.dumps(records[0]), encoding='utf-8')
            invalid = self.api('/api/history/list', {'sessionId': doc['sessionId']}, status=409)
            self.assertEqual(invalid['error']['code'], 'invalidState')
            self.assertEqual(invalid['error']['explanation'], {
                'key': 'error.core.journal.originalCommitEvidenceInvalid', 'params': {}})
            self.assertEqual(self.history_read(doc, versions[0]['id'], status=409)['error']['code'],
                             'invalidState')
        finally:
            for path, value in zip(commits, exact_commits):
                path.write_bytes(value)
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        refusal = self.history_read(doc, features.uid(), status=409)
        self.assertEqual(refusal['error']['code'], 'historyUnavailable')

    def test_07_external_rename_and_replacement_cannot_join_history_by_name(self):
        root, _, doc, versions = self.history_versions('外部改名前')
        old_path = Path(doc['path'])
        new_folder = old_path.parent.with_name('外部改名后')
        old_path.parent.rename(new_folder)
        renamed_leaf = new_folder / old_path.name
        new_path = new_folder / '外部改名后.typ'
        renamed_leaf.rename(new_path)
        renamed = self.api('/api/open', {'path': str(new_path), 'root': str(root)})
        refusal = self.api('/api/history/list', {'sessionId': renamed['sessionId']}, status=409)
        self.assertEqual(refusal['error']['code'], 'identityPending')
        self.assertEqual(new_path.read_bytes(), versions[-1]['source'].encode())
        # A replacement bearing identical metadata/source is a different file.
        root2, _, second, saved = self.history_versions('外部替换')
        path = Path(second['path'])
        original = path.read_bytes()
        path.rename(path.with_suffix('.typ.owned-original'))
        path.write_bytes(original)
        replacement = self.api('/api/open', {'path': str(path), 'root': str(root2)})
        refusal = self.api('/api/history/list', {'sessionId': replacement['sessionId']}, status=409)
        self.assertIn(refusal['error']['code'], {'historyUnavailable', 'identityPending'})
        self.assertEqual(path.read_bytes(), original)
        self.assertEqual(path.with_suffix('.typ.owned-original').read_bytes(), original)

    def test_08_draft_persistence_failure_refuses_restore_without_source_effect(self):
        from test_sync_compound import physical_identity

        root, _, doc, versions = self.history_versions('草稿写入失败')
        draft = doc['source'] + '\n需要保护的准确草稿🌿。\n'
        self.api('/api/draft', {'sessionId': doc['sessionId'], 'source': draft})
        source_identity = physical_identity(Path(doc['path']))
        expected_identity = {'platform': 'windows',
                             **dict(zip(('volume', 'index', 'created'), source_identity))}
        candidates = []
        for path in (self.data_dir / 'drafts').glob('*.json'):
            original = path.read_bytes()
            record = json.loads(original)
            self.assertIn(record['scope']['kind'], {'deviceUnassigned', 'library'})
            if (record['path'] == doc['path'] and record['source'] == draft
                    and record['identity'] == expected_identity):
                candidates.append((path, original))
        self.assertEqual(len(candidates), 1, 'The exact original Draft primary must be unique')
        draft_file, draft_bytes = candidates[0]
        self.assertTrue(draft_file.name.startswith('object-'))
        parked = draft_file.with_suffix('.json.owned-parked')
        obstruction = draft_file.with_suffix('.json.owned-obstruction')
        draft_file.rename(parked)
        draft_file.mkdir()
        request = self.restore_request(doc, versions[0]['id'], draft, 'retain')
        try:
            failure = self.api('/api/history/restore', request, status=409)
            self.assertIn(failure['error']['code'], {'io', 'permissionDenied', 'exists', 'invalidState'})
            self.assertEqual(Path(doc['path']).read_bytes(), versions[-1]['source'].encode())
            self.assertEqual(self.api('/api/history/status', {
                'operationId': request['operationId']})['status'], 'missing')
        finally:
            draft_file.rename(obstruction)
            parked.rename(draft_file)
        self.assertEqual(draft_file.read_bytes(), draft_bytes)
        self.assertEqual(physical_identity(Path(doc['path'])), source_identity)
        self.assertEqual(self.reopen(doc, root)['recoveredDraft'], draft)
        self.assertNotIn(request['operationId'], [item['versionId'] for item in self.timeline(doc)['versions']])

    def test_09_multiple_embedded_occurrences_exact_ranges_and_same_native_cel_executor(self):
        root, _, task, doc, _ = self.query_fixture(multiple=True)
        original = Path(doc['path']).read_bytes()
        draft = doc['source'] + '\n末尾尚未保存🌿。\n'
        discovery = self.discover(doc, draft, serial=7)
        self.assertTrue(discovery['syntaxValid'])
        self.assertEqual(len(discovery['occurrences']), 2)
        self.assertEqual(len({item['id'] for item in discovery['occurrences']}), 2)
        first = self.occurrence(discovery, 'weftext-query-first')
        second = self.occurrence(discovery, 'weftext-query-second')
        self.assertTrue(first['range']['to'] <= second['range']['from']
                        or second['range']['to'] <= first['range']['from'],
                        'Distinct native Query carriers must have disjoint exact ranges')
        for occurrence in [first, second]:
            self.assertTrue(occurrence['writable'])
            self.assertTrue(occurrence['builderSupported'])
            self.assertTrue(occurrence['runnable'])
            actual = self.run_embedded(doc, draft, occurrence['id'], serial=7)
            self.assertEqual(actual['sourceHash'], features.sha(draft))
            self.assertEqual(actual['occurrenceId'], occurrence['id'])
            self.assertEqual(actual['serial'], 7)
            self.assertTrue(actual['draftPreview'])
            direct = self.api('/api/query/run', {'root': str(root), 'plan': occurrence['plan']})
            self.assert_result_parity(actual['result'], direct)
            self.assertFalse(actual['result']['access']['resultGrantsWriteAccess'])
        open_rows = self.run_embedded(doc, draft, first['id'], serial=7)['result']['rows']
        self.assertEqual([row['path'] for row in open_rows], [task['path']])
        rendered = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': draft, 'serial': 7})
        self.assertTrue(rendered['ok'], rendered['diagnostics'])
        labels = [node.get('attrs', {}).get('data-weftext-query-label') for node in features.walk(rendered['tree'])]
        self.assertIn('weftext-query-first', labels)
        self.assertIn('weftext-query-second', labels)
        self.assertEqual(Path(doc['path']).read_bytes(), original)

    def test_10_builder_edits_only_selected_draft_range_and_rejects_stale_snapshots(self):
        _, _, _, doc, _ = self.query_fixture(multiple=True)
        original = Path(doc['path']).read_bytes()
        source = doc['source'] + '\n输入缓冲区🌿。\n'
        discovery = self.discover(doc, source, serial=3)
        first = self.occurrence(discovery, 'weftext-query-first')
        second = self.occurrence(discovery, 'weftext-query-second')
        plan = copy.deepcopy(first['plan'])
        plan['where'] = "status == 'done'"
        output = self.edit_embedded(doc, source, first['id'], serial=3, plan=plan)
        self.assert_patches(source, output, data=True)
        self.assertEqual(len(output['patches']), 1)
        patch = output['patches'][0]
        self.assertEqual(patch['byteFrom'], first['range']['from'])
        self.assertEqual(patch['byteTo'], first['range']['to'])
        self.assertEqual(source.encode()[:patch['byteFrom']], output['source'].encode()[:patch['byteFrom']])
        self.assertTrue(output['source'].endswith(source.encode()[patch['byteTo']:].decode()))
        self.assertIn(second['range']['raw'], output['source'])
        self.assertEqual(output['source'].count(second['range']['raw']), 1)
        next_discovery = self.discover(doc, output['source'], serial=4)
        changed = self.occurrence(next_discovery, first['label'])
        unchanged = self.occurrence(next_discovery, second['label'])
        self.assertEqual(changed['plan']['where'], "status == 'done'")
        self.assertEqual(changed['key'], first['key'])
        self.assertEqual(changed['binding'], first['binding'])
        self.assertEqual(unchanged['plan'], second['plan'])
        stale = self.edit_embedded(doc, source, first['id'], serial=3, plan=plan, status=409)
        self.assertEqual(stale['error']['code'], 'stale_draft')
        expired = self.edit_embedded(doc, output['source'], first['id'], serial=4, plan=plan, status=409)
        self.assertEqual(expired['error']['code'], 'embeddedQueryUnavailable')
        wrong_hash = self.api('/api/query/embedded/discover', {
            **self.snapshot(doc, output['source'], serial=5), 'sourceHash': features.sha(source)}, status=409)
        self.assertEqual(wrong_hash['error']['code'], 'source_hash_mismatch')
        # Temporarily invalid direct source input is an exact Draft proposal.
        replacement = changed['range']['raw'] + '\n#metadata(('
        invalid = self.edit_embedded(doc, output['source'], changed['id'], serial=4, replacement=replacement)
        self.assert_patches(output['source'], invalid, data=True)
        self.assertIn(replacement, invalid['source'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)

    def test_11_complex_duplicate_invalid_and_ordinary_query_boundaries_preserve_source(self):
        root, _, _, doc, _ = self.query_fixture()
        grouped_plan = {'types': ['task'], 'group': {
            'keys': [{'name': 'status', 'expr': 'status'}],
            'aggregates': [{'name': 'count', 'op': 'count'}]}}
        grouped = self.definition(doc, grouped_plan, 'weftext-query-grouped', serial=1, next_serial=2)
        doc = grouped['document']
        grouped_query = self.occurrence(self.discover(doc, serial=2), 'weftext-query-grouped')
        self.assertTrue(grouped_query['builderSupported'])
        grouped_edit = self.edit_embedded(doc, doc['source'], grouped_query['id'],
            plan={**grouped_query['plan'], 'take': 10}, serial=2)
        self.assert_patches(doc['source'], grouped_edit, data=True)
        self.assertEqual(Path(doc['path']).read_bytes(), doc['source'].encode())
        # Simple grouping is now supported. A distinct producer remains an
        # advanced source-only plan, preserving this negative boundary.
        complex_plan = {**self.status_plan(), 'advanced': {
            'distinct': [{'name': 'status', 'expr': 'status'}]}}
        saved = self.definition(doc, complex_plan, 'weftext-query-complex', serial=2, next_serial=3)
        doc = saved['document']
        original = Path(doc['path']).read_bytes()
        discovery = self.discover(doc, serial=3)
        complex_query = self.occurrence(discovery, 'weftext-query-complex')
        self.assertFalse(complex_query['builderSupported'])
        self.assertTrue(complex_query['runnable'])
        self.assertTrue(complex_query['controlReason'])
        actual = self.run_embedded(doc, doc['source'], complex_query['id'], serial=3)
        direct = self.api('/api/query/run', {'root': str(root), 'plan': complex_query['plan']})
        self.assert_result_parity(actual['result'], direct)
        refusal = self.edit_embedded(doc, doc['source'], complex_query['id'],
                                     plan=self.status_plan(), status=409, serial=3)
        self.assertEqual(refusal['error']['code'], 'embeddedQueryBuilderUnsupported')
        # Duplicate native keys with different labels do not create a guessed
        # source target or silently rewrite either occurrence.
        simple = self.occurrence(discovery, 'weftext-query-first')
        duplicate = simple['range']['raw'].replace('weftext-query-first', 'weftext-query-duplicate')
        duplicate_source = doc['source'] + '\n' + duplicate + '\n'
        duplicated = self.discover(doc, duplicate_source, serial=4)
        targets = [item for item in duplicated['occurrences'] if item['key'] == simple['key']]
        self.assertEqual(len(targets), 2)
        self.assertTrue(all(not item['writable'] and not item['runnable'] for item in targets))
        refusal = self.edit_embedded(doc, duplicate_source, targets[0]['id'], serial=4,
                                     replacement=targets[0]['range']['raw'], status=409)
        self.assertEqual(refusal['error']['code'], 'embeddedQueryUnavailable')
        invalid_source = doc['source'] + '\n#let incomplete = ('
        invalid = self.discover(doc, invalid_source, serial=5)
        self.assertFalse(invalid['syntaxValid'])
        self.assertTrue(all(not item['writable'] for item in invalid['occurrences']))
        self.assertEqual(invalid['source'], invalid_source)
        invalid_target = self.occurrence(invalid, 'weftext-query-first')
        failure = self.run_embedded(doc, invalid_source, invalid_target['id'], serial=5, status=409)
        self.assertEqual(failure['error']['code'], 'embeddedQueryUnavailable')
        ordinary = self.ordinary(doc['source'])
        plain = self.discover(ordinary, serial=1)
        first = self.occurrence(plain, 'weftext-query-first')
        failure = self.run_embedded(ordinary, ordinary['source'], first['id'], status=409, serial=1)
        self.assertEqual(failure['error']['code'], 'queryUnavailable')
        failure = self.edit_embedded(ordinary, ordinary['source'], first['id'],
                                     plan=first['plan'], status=409, serial=1)
        self.assertEqual(failure['error']['code'], 'queryUnavailable')
        self.assertEqual(Path(ordinary['path']).read_bytes(), ordinary['source'].encode())
        self.assertEqual(Path(doc['path']).read_bytes(), original)

    def test_12_pinned_calls_use_selected_owner_revision_and_never_write_owner(self):
        root, library, task, owner, first = self.query_fixture()
        key = first['definition']['key']
        revision = first['definitionRevision']
        second = self.definition(owner, self.status_plan('done'), 'weftext-query-first', key=key, serial=1, next_serial=2)
        owner = second['document']
        owner_ref = f"weftext-node://{library['workspaceId']}/{owner['nodeId']}"
        caller = self.note(root, '调用者', '调用者前后文🌿。\n')
        call_plan = {'advanced': {'call': {'owner': owner_ref,
            'selection': {'mode': 'pinned', 'key': key, 'revision': revision}}}}
        called = self.definition(caller, call_plan, 'weftext-query-call', serial=0, next_serial=1)
        caller = called['document']
        owner_before = Path(owner['path']).read_bytes()
        caller_before = Path(caller['path']).read_bytes()
        discovery = self.discover(caller, serial=1)
        occurrence = self.occurrence(discovery, 'weftext-query-call')
        self.assertEqual(occurrence['kind'], 'call')
        self.assertEqual(occurrence['call']['owner'], owner_ref)
        self.assertEqual(occurrence['call']['selection']['revision'], revision)
        self.assertTrue(occurrence['builderSupported'])
        result = self.run_embedded(caller, caller['source'], occurrence['id'], serial=1)['result']
        self.assertEqual([row['path'] for row in result['rows']], [task['path']])
        self.assertFalse(result['access']['resultGrantsWriteAccess'])
        definitions = [item for item in result['inputVersions']
                       if item.get('role') == 'definition' and item.get('owner') == owner_ref]
        self.assertEqual(len(definitions), 1)
        provenance = definitions[0]
        self.assertEqual(provenance['definitionRevision'], revision)
        self.assertEqual(provenance['sourceHash'], features.sha(first['document']['source']))
        self.assertFalse(provenance['draftPreview'])
        current_plan = json.loads(json.dumps(occurrence['plan']))
        current_plan['advanced']['call']['selection'] = {'mode': 'current', 'key': key}
        current_edit = self.edit_embedded(caller, caller['source'], occurrence['id'],
                                         plan=current_plan, serial=1)
        self.assert_patches(caller['source'], current_edit, data=True)
        self.assertRegex(current_edit['source'], r'(?:"mode"|mode):\s*"current"')
        self.assertIn(owner_ref, current_edit['source'])
        self.assertEqual(Path(owner['path']).read_bytes(), owner_before)
        self.assertEqual(Path(caller['path']).read_bytes(), caller_before)
        owner_discovery = self.discover(owner, serial=2)
        owner_occurrence = self.occurrence(owner_discovery, 'weftext-query-first')
        borrowed_range = self.edit_embedded(caller, caller['source'], owner_occurrence['id'],
            replacement=owner_occurrence['range']['raw'], status=409, serial=1)
        self.assertEqual(borrowed_range['error']['code'], 'embeddedQueryUnavailable')
        # Even explicit source editing is an exact caller Draft range. It may
        # alter the invocation, and cannot mutate the owner's retained source.
        edited = self.edit_embedded(caller, caller['source'], occurrence['id'],
            replacement=occurrence['range']['raw'] + ' /* 调用处草稿🌿 */', serial=1)
        self.assert_patches(caller['source'], edited, data=True)
        self.assertEqual(Path(owner['path']).read_bytes(), owner_before)
        self.assertEqual(Path(caller['path']).read_bytes(), caller_before)
        history = self.timeline(owner)
        old = next(item for item in history['versions'] if item['revisionId'] == revision)
        self.assertEqual(self.history_read(owner, old['versionId'])['source'], first['document']['source'])
        # Pinned definition history still queries newly saved current data.
        changed = self.api('/api/data/edit', {**self.snapshot(task, serial=2),
            'key': 'status', 'value': features.text('done')})
        self.map_source(task, changed['source'], serial=3)
        task = self.saved(task, changed['source'], root)
        actual = self.run_embedded(caller, caller['source'], occurrence['id'], serial=1)['result']
        self.assertEqual(actual['rows'], [])
        self.assertEqual(actual['coverage'], 'complete')
        self.assertEqual(Path(owner['path']).read_bytes(), owner_before)

    def test_13_embedded_empty_partial_error_and_external_pending_are_distinct(self):
        root, _, _, doc, _ = self.query_fixture()
        original = Path(doc['path']).read_bytes()
        discovery = self.discover(doc, serial=1)
        occurrence = self.occurrence(discovery, 'weftext-query-first')
        empty_plan = copy.deepcopy(occurrence['plan'])
        empty_plan['where'] = 'false'
        empty_edit = self.edit_embedded(doc, doc['source'], occurrence['id'], plan=empty_plan, serial=1)
        empty_discovery = self.discover(doc, empty_edit['source'], serial=2)
        empty_occurrence = self.occurrence(empty_discovery, occurrence['label'])
        empty = self.run_embedded(doc, empty_edit['source'], empty_occurrence['id'], serial=2)['result']
        self.assertEqual(empty['rows'], [])
        self.assertEqual(empty['coverage'], 'complete')
        self.assertEqual(empty['totalKnown'], 0)
        self.assertFalse(empty['externalPending'])
        self.assertEqual(empty['freshness'], 'stale')
        partial_plan = copy.deepcopy(empty_occurrence['plan'])
        partial_plan['where'] = "priority == 'high'"
        partial_edit = self.edit_embedded(doc, empty_edit['source'], empty_occurrence['id'],
                                          serial=2, plan=partial_plan)
        partial_discovery = self.discover(doc, partial_edit['source'], serial=3)
        partial_occurrence = self.occurrence(partial_discovery, occurrence['label'])
        partial = self.run_embedded(doc, partial_edit['source'], partial_occurrence['id'], serial=3)['result']
        self.assertEqual(partial['rows'], [])
        self.assertEqual(partial['coverage'], 'partial')
        self.assertTrue(partial['diagnostics'])
        self.assert_result_parity(partial, self.api('/api/query/run', {
            'root': str(root), 'plan': partial_occurrence['plan']}))
        invalid_raw = partial_occurrence['range']['raw'].replace("priority == 'high'", 'unknown_field == true')
        invalid_edit = self.edit_embedded(doc, partial_edit['source'], partial_occurrence['id'],
                                          serial=3, replacement=invalid_raw)
        invalid_discovery = self.discover(doc, invalid_edit['source'], serial=4)
        invalid_occurrence = self.occurrence(invalid_discovery, occurrence['label'])
        error = self.run_embedded(doc, invalid_edit['source'], invalid_occurrence['id'], serial=4, status=400)
        self.assertEqual(error['error']['code'], 'invalid_data')
        # A discovered ordinary file is pending data, not an admitted member
        # that can make a supposedly complete scope silently expand.
        pending_folder = root / '外部未接纳'
        pending_folder.mkdir()
        pending = pending_folder / '外部未接纳.typ'
        pending.write_text('外部待接纳正文。\n', encoding='utf-8')
        fresh_discovery = self.discover(doc, empty_edit['source'], serial=5)
        fresh_occurrence = self.occurrence(fresh_discovery, occurrence['label'])
        pending_result = self.run_embedded(doc, empty_edit['source'], fresh_occurrence['id'], serial=5)['result']
        self.assertEqual(pending_result['rows'], [])
        self.assertTrue(pending_result['externalPending'])
        self.assertGreater(pending_result['rawScope']['pendingCount'], 0)
        self.assertFalse(pending_result['access']['resultGrantsWriteAccess'])
        self.assertEqual(Path(doc['path']).read_bytes(), original)

    def test_14_docx_export_reports_unmaterialized_embedded_results_and_preserves_source(self):
        _, _, _, doc, _ = self.query_fixture(multiple=True)
        original = Path(doc['path']).read_bytes()
        draft = doc['source'] + '\n输出所用的未保存正文🌿。\n'
        discovery = self.discover(doc, draft, serial=6)
        self.assertEqual(len(discovery['occurrences']), 2)
        request = {'outputLocale': 'zh-CN', 'requestId': features.uid(), 'sessionId': doc['sessionId'],
                   'baseId': doc['baseId'], 'source': draft, 'serial': 6,
                   'format': 'docx', 'stylePolicy': 'template'}
        output = self.api('/api/export/prepare', request)
        self.assertEqual(output['status'], 'prepared')
        self.assertEqual(output['requestId'], request['requestId'])
        self.assertTrue(any(loss['code'] == 'embedded_query_results_not_materialized'
                            for loss in output['losses']), output['losses'])
        self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': False}, status=409)
        self.assertEqual(Path(doc['path']).read_bytes(), original)
        confirmed = self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': True})
        self.assertEqual(confirmed['status'], 'confirmed')
        _, payload, headers = self.raw('/api/export/download', {'id': output['id']})
        self.assertTrue(payload.startswith(b'PK'))
        self.assertEqual(len(payload), output['length'])
        self.assertEqual(hashlib.sha256(payload).hexdigest(), output['digest'])
        self.assertEqual(headers['Cache-Control'], 'no-store')
        products = self.root / 'output-products'
        products.mkdir(exist_ok=True)
        (products / 'embedded-query-export.docx').write_bytes(payload)
        (products / 'embedded-query-export.typ').write_bytes(draft.encode())
        (products / 'embedded-query-export-preview.json').write_text(
            json.dumps(output, ensure_ascii=False, indent=2), encoding='utf-8')
        self.assertEqual(Path(doc['path']).read_bytes(), original)


    def materialized_fixture(self):
        root, _, _, doc, _ = self.query_fixture(multiple=True)
        occurrences = self.discover(doc, serial=2)['occurrences']
        source = doc['source']
        for item in occurrences:
            source = source.replace(item['range']['raw'], '')
        source += '\n\n输出前文。\n\n' + occurrences[0]['range']['raw'] + '\n\n输出中段。\n\n' + occurrences[1]['range']['raw'] + '\n\n输出后文。\n'
        discovery = self.discover(doc, source, serial=10)
        selections = []
        for item in discovery['occurrences']:
            result = self.run_embedded(doc, source, item['id'], serial=10)['result']
            selections.append({'occurrenceId': item['id'], 'queryId': result['queryId']})
        return root, doc, source, selections

    def materialized_export(self, doc, source, selections, fmt, name, **options):
        request = {'outputLocale': 'zh-CN', 'requestId': features.uid(), 'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                   'source': source, 'serial': 10, 'format': fmt, 'embeddedQueries': selections, **options}
        output = self.api('/api/export/prepare', request)
        self.assertEqual(output['status'], 'prepared', output)
        self.assertEqual(self.api('/api/export/prepare', request), output)
        self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': True})
        _, payload, _ = self.raw('/api/export/download', {'id': output['id']})
        self.assertEqual(hashlib.sha256(payload).hexdigest(), output['digest'])
        products = self.root / 'output-products'; products.mkdir(exist_ok=True)
        (products / f'{name}.{fmt}').write_bytes(payload)
        (products / f'{name}.json').write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding='utf-8')
        self.assertEqual(Path(doc['path']).read_bytes(), doc['source'].encode())
        return output, payload

    def test_15_docx_embedded_results_keep_body_order_and_frozen_rows(self):
        root, doc, source, selections = self.materialized_fixture()
        self.capture(root, '查询之后新增，不应混入结果', ['task'], {'tasks/status': features.text('open')})
        output, payload = self.materialized_export(doc, source, selections, 'docx', 'embedded-frozen-body')
        self.assertFalse(any(l['code'] == 'embedded_query_results_not_materialized' for l in output['losses']))
        self.assertEqual([q['rowCount'] for q in output['preview']['embeddedQueries']], [1, 1])
        with zipfile.ZipFile(io.BytesIO(payload)) as archive:
            xml = ET.fromstring(archive.read('word/document.xml'))
        ns = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
        text = ''.join(t.text or '' for t in xml.iter(ns + 't'))
        self.assertLess(text.index('输出前文'), text.index('查询 '))
        self.assertLess(text.index('查询 '), text.index('输出中段'))
        self.assertLess(text.index('输出中段'), text.rindex('查询 '))
        self.assertLess(text.rindex('查询 '), text.index('输出后文'))
        tables = list(xml.iter(ns + 'tbl'))
        self.assertEqual(len(tables), 2)
        self.assertEqual([len(list(t.iter(ns + 'tr'))) for t in tables], [2, 2])
        self.assertIn('open', text); self.assertIn('done', text)
        # Template body uses the same materialization, preserving its untouched footer.
        import test_outputs as outputs
        template, _, _ = outputs.docx_template()
        self.materialized_export(doc, source, selections, 'docx', 'embedded-template-body',
                                 template={'name': 'template.docx', 'bytesBase64': outputs.b64(template)},
                                 bindings={'BodySlot': {'kind': 'body'}, 'TitleSlot': {'kind': 'text', 'text': '冻结查询报告'},
                                           'LabelSlot': {'kind': 'text', 'text': '保留模板样式'}})

    def test_16_native_pdf_materializes_exact_results_without_saving(self):
        _, doc, source, selections = self.materialized_fixture()
        output, payload = self.materialized_export(doc, source, selections, 'pdf', 'embedded-frozen-pdf')
        self.assertTrue(payload.startswith(b'%PDF-'))
        self.assertEqual(output['preview']['target'], 'paged')
        self.assertGreater(output['preview']['pageCount'], 0)
        self.assertEqual(len(output['preview']['embeddedQueries']), 2)
        self.assertFalse(any(l['code'] == 'embedded_query_results_not_materialized' for l in output['losses']))

    def test_17_body_output_rejects_cross_query_duplicate_and_old_draft_results(self):
        _, doc, source, selections = self.materialized_fixture()
        request = {'outputLocale': 'zh-CN', 'sessionId': doc['sessionId'], 'baseId': doc['baseId'], 'source': source,
                   'serial': 10, 'format': 'docx', 'embeddedQueries': selections}
        swapped = copy.deepcopy(selections); swapped[0]['queryId'] = selections[1]['queryId']
        for bad in [swapped, [selections[0], selections[0]]]:
            output = self.api('/api/export/prepare', {**request, 'requestId': features.uid(), 'embeddedQueries': bad})
            self.assertEqual(output['status'], 'failed', output)
        self.discover(doc, source, serial=11)
        output = self.api('/api/export/prepare', {**request, 'requestId': features.uid(), 'serial': 11})
        self.assertEqual(output['status'], 'failed', output)
        self.assertEqual(Path(doc['path']).read_bytes(), doc['source'].encode())

    def test_18_partial_and_error_results_are_visible_in_body_output(self):
        _, doc, source, _ = self.materialized_fixture()
        occurrence = self.discover(doc, source, serial=10)['occurrences'][0]
        plan = copy.deepcopy(occurrence['plan']); plan['where'] = "priority == 'high'"
        edited = self.edit_embedded(doc, source, occurrence['id'], serial=10, plan=plan)
        discovery = self.discover(doc, edited['source'], serial=11)
        chosen = self.occurrence(discovery, occurrence['label'])
        result = self.run_embedded(doc, edited['source'], chosen['id'], serial=11)['result']
        self.assertEqual(result['coverage'], 'partial'); self.assertTrue(result['diagnostics'])
        for fmt in ['docx', 'pdf']:
            output = self.api('/api/export/prepare', {'outputLocale': 'zh-CN', 'requestId': features.uid(), 'sessionId': doc['sessionId'],
                'baseId': doc['baseId'], 'source': edited['source'], 'serial': 11, 'format': fmt,
                'embeddedQueries': [{'occurrenceId': chosen['id'], 'queryId': result['queryId']}]})
            self.assertEqual(output['status'], 'prepared', output)
            self.assertTrue(any(l['code'] == 'query_incomplete' for l in output['losses']))
            self.assertTrue(any(l['code'] == 'embedded_query_results_not_materialized' for l in output['losses']))
            self.assertEqual(output['preview']['embeddedQueries'][0]['coverage'], 'partial')
            self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': True})
            _, payload, _ = self.raw('/api/export/download', {'id': output['id']})
            folder = self.root / 'output-products'; folder.mkdir(exist_ok=True)
            (folder / ('partial-result.' + fmt)).write_bytes(payload)
            if fmt == 'docx':
                with zipfile.ZipFile(io.BytesIO(payload)) as archive:
                    xml = ET.fromstring(archive.read('word/document.xml'))
                text = ''.join(xml.itertext())
                self.assertIn('partial', text); self.assertIn('不能据此断言全库为空', text)
                self.assertIn('诊断', text)


    def test_19_inline_query_boundaries_and_pdf_binding_guard(self):
        _, _, _, doc, _ = self.query_fixture()
        occurrence = self.discover(doc, serial=1)['occurrences'][0]
        raw = occurrence['range']['raw']
        base = doc['source'].replace(raw, '')
        for serial, fmt, prefix, expected in [
            (10, 'docx', '普通前文 *加粗前文* ', 'prepared'),
            (11, 'docx', '#emph[样式内部 ', 'failed'),
            (12, 'pdf', '#let table = (..args) => [替代布局]\n普通前文 ', 'failed'),
        ]:
            source = base + '\n\n' + prefix + raw + ' 后文' + (']' if serial == 11 else '')
            current = self.discover(doc, source, serial=serial)['occurrences'][0]
            if serial == 11:
                self.assertFalse(current['runnable'])
                self.run_embedded(doc, source, current['id'], serial=serial, status=409)
                continue
            selections = []
            if serial == 10:
                result = self.run_embedded(doc, source, current['id'], serial=serial)['result']
                selections = [{'occurrenceId': current['id'], 'queryId': result['queryId']}]
            output = self.api('/api/export/prepare', {'outputLocale': 'zh-CN', 'requestId': features.uid(), 'sessionId': doc['sessionId'],
                'baseId': doc['baseId'], 'source': source, 'serial': serial, 'format': fmt,
                'embeddedQueries': selections})
            self.assertEqual(output['status'], expected, output)
            if expected == 'prepared':
                self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': True})
                _, payload, _ = self.raw('/api/export/download', {'id': output['id']})
                with zipfile.ZipFile(io.BytesIO(payload)) as archive:
                    xml = ET.fromstring(archive.read('word/document.xml'))
                ns = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
                body = xml.find(ns + 'body')
                table = next(i for i, child in enumerate(body) if child.tag == ns + 'tbl')
                self.assertIn('加粗前文', ''.join(body[table - 2].itertext()))
                self.assertTrue(list(body[table - 2].iter(ns + 'b')))
                self.assertIn('后文', ''.join(body[table + 1].itertext()))
            self.assertEqual(Path(doc['path']).read_bytes(), doc['source'].encode())


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('--artifacts', '--artifacts-dir', type=Path, dest='artifacts')
    parser.add_argument('--only', action='append', default=[], metavar='TEST')
    args = parser.parse_args()
    features.EXE = args.executable.resolve(strict=True)
    executable_digest = hashlib.sha256(features.EXE.read_bytes()).hexdigest()
    if args.artifacts:
        args.artifacts.mkdir(parents=True, exist_ok=True)
        features.ARTIFACT_PARENT = args.artifacts.resolve(strict=True)
    names = sorted(name for name in V05Completion.__dict__ if name.startswith('test_'))
    if args.only:
        selected = {name.split('.')[-1] for name in args.only}
        unknown = selected.difference(names)
        if unknown:
            parser.error(f'Unknown v0.5 scenarios: {sorted(unknown)}')
        names = [name for name in names if name in selected]
    # LocalFeatures supplies helpers only. Its inherited scenarios are not rerun.
    suite = unittest.TestSuite(V05Completion(name) for name in names)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if hasattr(V05Completion, 'root'):
        (V05Completion.root / 'results.json').write_text(json.dumps({
            'executable': str(features.EXE), 'executableSha256AtStart': executable_digest,
            'testsRun': result.testsRun, 'selectedScenarios': names,
            'failures': [{'test': str(test), 'traceback': trace} for test, trace in result.failures],
            'errors': [{'test': str(test), 'traceback': trace} for test, trace in result.errors],
            'skipped': [{'test': str(test), 'reason': reason} for test, reason in result.skipped],
            'successful': result.wasSuccessful(),
            'scope': 'HTTP acceptance using newly owned disposable local libraries and retained real Core journals',
            'browserInteraction': 'Not established: LHT/IQV view controls, focus, keyboard/touch, shared Undo, cancellation UI and late browser result handling require separate browser acceptance',
            'recoveryPendingInjection': 'Owned restore commit witness was temporarily parked and restored to check pending status/access/retry refusal; actual process crash and power-loss were not injected',
            'multiplayerHistory': 'Not run; v0.6 authorization/synchronization scenarios remain outside this local candidate',
            'hostInput': 'None; owned console server runs serve with CREATE_NO_WINDOW',
            'fixturesRetained': str(V05Completion.root),
        }, ensure_ascii=False, indent=2), encoding='utf-8')
    sys.exit(0 if result.wasSuccessful() else 1)
