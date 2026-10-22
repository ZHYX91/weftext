"""Actual local HTTP commit/delivery and original-attempt recovery, in owned TEST data.

All evidence and original files are retained. Processes stop by an exact typed
owner shutdown; this test does not certify UI shutdown or power loss.
"""
import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path
import socket
import tempfile
import time
import traceback
import test_hosted
from _creation_result import created_document


def run(executable, evidence):
    evidence.mkdir(parents=True, exist_ok=False)
    state, root = evidence / 'state', evidence / 'TEST-library'
    test_hosted.EXE = executable
    server = None
    owned = []
    events = (evidence / 'http.jsonl').open('w', encoding='utf-8')
    result = {'executable': str(executable), 'executableSha256': hashlib.sha256(executable.read_bytes()).hexdigest(), 'passed': False}

    def start():
        nonlocal server
        server = test_hosted.Server([ '--data-dir', str(state)])
        owned.append({'pid': server.process.pid, 'url': server.url, 'startedAt': time.time()})

    def stop():
        nonlocal server
        if server is None:
            return
        pid = server.process.pid
        stdout, stderr = server.close()
        (evidence / f'{pid}-stdout.log').write_text(stdout, encoding='utf-8')
        (evidence / f'{pid}-stderr.log').write_text(stderr, encoding='utf-8')
        owned[-1]['exitCode'] = server.process.returncode
        port = int(server.url.rsplit(':', 1)[1])
        with socket.socket() as client:
            owned[-1]['listenerClosed'] = client.connect_ex(('127.0.0.1', port)) != 0
        assert owned[-1]['listenerClosed']
        server = None

    def call(route, body=None, expected=200):
        status, value, _ = server.call(route, body)
        if isinstance(value, bytes):
            value = value.decode('utf-8', errors='replace')
        # ASCII escapes retain even deliberately rejected lone UTF-16 surrogates.
        events.write(json.dumps({'route': route, 'request': body, 'status': status, 'response': value}, ensure_ascii=True) + '\n')
        events.flush()
        assert status == expected, (route, status, value)
        return value

    def fence(payload, kind):
        # Server.call sends exactly json.dumps(payload).encode() on this entry.
        wire = json.dumps(payload).encode('utf-8')
        return {'operationId': payload['operationId'], 'requestKind': kind, 'wireDigest': hashlib.sha256(wire).hexdigest()}

    try:
        start()
        call('/api/library/create', {'path': str(root), 'operationId': test_hosted.uid()})
        create = {'root': str(root), 'parent': str(root), 'name': 'Original', 'operationId': test_hosted.uid()}
        first = call('/api/note/create', create)
        doc = created_document(first, create['operationId'])
        assert created_document(call('/api/note/create', create))['nodeId'] == doc['nodeId']
        save = {'sessionId': doc['sessionId'], 'baseId': doc['baseId'], 'source': doc['source'] + '\nsubmitted S', 'operationId': test_hosted.uid()}
        receipt = call('/api/save', save)
        stop(); start()
        fact = call('/api/operations/status', {'operationId': save['operationId']})
        assert fact['status'] == 'committed' and fact['baseId'] == receipt['baseId']
        assert fact['committedSource'] == save['source'] and fact['documentSourceMatchesCommit'] is True
        fresh = fact['document']
        assert fresh['sessionId'] != doc['sessionId']
        waiting = save['source'] + '\nwaiting input Y'
        call('/api/save', {'sessionId': fresh['sessionId'], 'baseId': fresh['baseId'], 'source': waiting, 'operationId': test_hosted.uid()})
        later = call('/api/operations/status', {'operationId': save['operationId']})
        assert later['committedSource'] == save['source'] and later['document']['source'] == waiting
        assert later['documentSourceMatchesCommit'] is False
        assert Path(doc['path']).read_text('utf-8') == waiting

        save_as_path = evidence / 'TEST-save-as.typ'
        save_as = {'sessionId': later['document']['sessionId'], 'path': str(save_as_path), 'source': waiting, 'operationId': test_hosted.uid()}
        saved_as = call('/api/save-as', save_as)
        assert saved_as['status'] == 'saved' and saved_as['documentSourceMatchesCommit'] is True
        assert saved_as['document']['path'] == str(save_as_path)
        as_plan = state / 'journal' / f"{save_as['operationId']}.plan.json"
        as_commit = state / 'journal' / f"{save_as['operationId']}.commit.json"
        as_original_journal = (as_plan.read_bytes(), as_commit.read_bytes())
        as_original = save_as_path.read_bytes()
        as_physical = (save_as_path.stat().st_dev, save_as_path.stat().st_ino)
        retained_as = evidence / 'TEST-retained-save-as-original.typ'
        save_as_path.rename(retained_as)
        save_as_path.write_bytes(as_original)
        assert (save_as_path.stat().st_dev, save_as_path.stat().st_ino) != as_physical
        assert call('/api/open', {'sessionId': saved_as['document']['sessionId']}, 409)['error']['code'] == 'sourceIdentityChanged'
        call('/api/open', {'sessionId': saved_as['document']['sessionId'], 'path': str(save_as_path)}, 422)
        for route, payload in [('/api/operations/status', {'operationId': save_as['operationId']}), ('/api/save-as', save_as)]:
            unavailable = call(route, payload)
            assert unavailable['baseId'] == saved_as['baseId'] and unavailable.get('documentError')
            assert 'document' not in unavailable
        assert (as_plan.read_bytes(), as_commit.read_bytes()) == as_original_journal
        assert retained_as.read_bytes() == as_original
        save_as_path.rename(evidence / 'TEST-retained-save-as-substitute.typ')
        retained_as.rename(save_as_path)
        as_restored = call('/api/operations/status', {'operationId': save_as['operationId']})
        assert as_restored['baseId'] == saved_as['baseId'] and as_restored['documentSourceMatchesCommit'] is True
        qualified_refresh = call('/api/open', {'sessionId': saved_as['document']['sessionId']})
        assert qualified_refresh['source'] == waiting and qualified_refresh['sessionId'] != saved_as['document']['sessionId']

        # The rejected author JSON cannot be parsed a second time to settle it.
        invalid = {'sessionId': later['document']['sessionId'], 'baseId': later['document']['baseId'], 'source': waiting + '\ud800', 'operationId': test_hosted.uid()}
        call('/api/save', invalid, 400)
        assert call('/api/operations/status', {'operationId': invalid['operationId']})['status'] == 'unknown'
        invalid_fence = fence(invalid, 'save')
        for malformed_digest in ['a' * 63, 'g' * 64]:
            call('/api/operations/settle', {**invalid_fence, 'wireDigest': malformed_digest}, 400)
        assert not (state / 'journal' / f"{invalid['operationId']}.resolution.json").exists()
        assert call('/api/operations/settle', invalid_fence)['status'] == 'resolved'
        decision = json.loads((state / 'journal' / f"{invalid['operationId']}.resolution.json").read_text('utf-8'))
        assert decision['attempt']['wireDigest'] == invalid_fence['wireDigest']
        assert Path(doc['path']).read_text('utf-8') == waiting
        delayed = {**invalid, 'source': waiting + '\nlate corrected payload'}
        assert call('/api/save', delayed, 409)['error']['code'] == 'operationCancelled'
        new_save = {**delayed, 'operationId': test_hosted.uid()}
        assert call('/api/save', new_save)['status'] == 'saved'

        history_created = call('/api/note/create', {**create, 'name': 'History', 'operationId': test_hosted.uid()})
        history_doc = created_document(history_created)
        history_saved = call('/api/save', {'sessionId': history_doc['sessionId'], 'baseId': history_doc['baseId'], 'source': history_doc['source'] + '\nnewer history version', 'operationId': test_hosted.uid()})
        before_history = call('/api/history/list', {'sessionId': history_doc['sessionId']})
        restore = {'sessionId': history_doc['sessionId'], 'baseId': history_saved['baseId'], 'versionId': history_created['operationId'], 'operationId': test_hosted.uid(), 'draftAction': 'none'}
        assert call('/api/history/status', {'operationId': restore['operationId']})['status'] == 'missing'
        restored = call('/api/history/restore', restore)
        assert restored['status'] == 'committed' and restored['operationId'] == restore['operationId']
        assert restored['result']['source'] == history_doc['source'] and restored['receipt'] == restored['result']['receipt']
        # Exact missing -> intervening real commit -> settle sequence, without a
        # scheduling race. Generic settlement must not masquerade as history.
        settled = call('/api/operations/settle', fence(restore, 'save'))
        assert settled['status'] == 'committed' and settled['baseId'] == restored['receipt']['baseId'] and 'result' not in settled
        assert call('/api/history/status', {'operationId': restore['operationId']}) == restored
        history_dir = Path(history_created['path'])
        retained_history = evidence / 'TEST-retained-history'
        history_dir.rename(retained_history)
        unavailable = call('/api/history/restore', restore)
        assert unavailable['status'] == 'committed' and unavailable['receipt'] == restored['receipt']
        assert 'result' not in unavailable and unavailable.get('resultError')
        assert (retained_history / 'History.typ').read_text('utf-8') == history_doc['source']
        retained_history.rename(history_dir)
        assert call('/api/history/restore', restore) == restored
        after_history = call('/api/history/list', {'sessionId': history_doc['sessionId']})
        assert len(after_history['versions']) == len(before_history['versions']) + 1
        changed_restore = call('/api/history/restore', {**restore, 'versionId': test_hosted.uid()}, 409)
        assert changed_restore['error']['code'] == 'operationConflict'

        capture = {'root': str(root), 'parent': str(root), 'name': 'Capture', 'types': ['task'], 'title': None, 'fields': {'status': {'kind': 'text', 'value': 'open'}}, 'body': 'TEST capture', 'operationId': test_hosted.uid()}
        capture_first = call('/api/data/capture', capture)
        period = {'root': str(root), 'parent': str(root), 'name': 'Period', 'seriesKey': 'work-log', 'kind': 'week', 'key': '2020-W53', 'title': None, 'body': 'TEST period', 'operationId': test_hosted.uid()}
        period_first = call('/api/data/period/create', period)
        schema = root / '.weftext-schema.json'
        previous = schema.read_bytes() if schema.exists() else None
        schema.write_bytes(b'invalid current schema')
        for route, payload, original in [('/api/data/capture', capture, capture_first), ('/api/data/period/create', period, period_first)]:
            replay = call(route, payload)
            assert replay['status'] == 'committed' and replay['operationId'] == payload['operationId']
            assert replay['document']['nodeId'] == original['document']['nodeId']
        if previous is None:
            schema.unlink()
        else:
            schema.write_bytes(previous)

        # Actual postcommit open failure: preserve the original physical object
        # under a TEST name. A replay returns the same fact and creates no tree.
        original_dir = Path(first['path'])
        retained = root / 'retained-original'
        original_dir.rename(retained)
        missing = call('/api/operations/status', {'operationId': create['operationId']})
        assert missing['status'] == 'committed' and missing.get('documentError') and not missing.get('document')
        replay = call('/api/note/create', create)
        assert replay['status'] == 'committed' and replay['operationId'] == create['operationId']
        assert not original_dir.exists() and (retained / 'Original.typ').is_file()
        retained.rename(original_dir)

        # The original result follows its physical birth, not an old pathname.
        parent = created_document(call('/api/note/create', {**create, 'name': 'DeliveryParent', 'operationId': test_hosted.uid()}))
        original_request = {**create, 'name': 'DeliveryX', 'operationId': test_hosted.uid()}
        original_receipt = call('/api/note/create', original_request)
        original_x = created_document(original_receipt)
        x_bytes = Path(original_x['path']).read_bytes()
        identity = lambda file: [os.stat(file).st_dev, os.stat(file).st_ino]
        x_identity = identity(original_x['path'])
        journal_files = [state / 'journal' / f"{original_request['operationId']}.{suffix}.json" for suffix in ('plan', 'commit', 'births', 'materialized')]
        original_journal = {str(file): hashlib.sha256(file.read_bytes()).hexdigest() for file in journal_files if file.exists()}
        move = call('/api/operations/prepare', {'root': str(root), 'operationId': test_hosted.uid(), 'intent': {'kind': 'moveNode', 'source': original_receipt['path'], 'parent': str(Path(parent['path']).parent)}})
        moved = call('/api/operations/commit', {'operationId': move['operationId'], 'planId': move['planId']})
        y = created_document(call('/api/note/create', {**original_request, 'operationId': test_hosted.uid()}))
        for route, payload in [('/api/operations/status', {'operationId': original_request['operationId']}), ('/api/note/create', original_request)]:
            delivered = call(route, payload)
            assert all(delivered[key] == original_receipt[key] for key in ('operationId', 'planId', 'path', 'nodeIds', 'status', 'message'))
            assert delivered['document']['nodeId'] == original_x['nodeId'] != y['nodeId']
            assert identity(delivered['document']['path']) == x_identity != identity(y['path'])
            assert Path(delivered['document']['path']).read_bytes() == x_bytes
        actual_x = Path(moved['path']) / 'DeliveryX.typ'
        held_x = evidence / 'TEST-original-physical-X.typ'
        actual_x.rename(held_x); actual_x.write_bytes(x_bytes)
        substitute = call('/api/operations/status', {'operationId': original_request['operationId']})
        assert substitute['status'] == 'committed' and substitute.get('documentError') and 'document' not in substitute
        actual_x.rename(evidence / 'TEST-preserved-ID-substitute.typ'); held_x.rename(actual_x)
        assert {file: hashlib.sha256(Path(file).read_bytes()).hexdigest() for file in original_journal} == original_journal

        # Library creation retains its fact while the original root cannot be
        # delivered; copied controls in another physical directory do not count.
        library_request = {'path': str(evidence / 'TEST-library-result'), 'operationId': test_hosted.uid(), 'options': {'initialRoot': {'title': '', 'queries': []}}}
        library_receipt = call('/api/library/create', library_request)
        assert library_receipt['rootDocument']['workspaceId'] == library_receipt['workspaceId']
        library_root = Path(library_request['path']); held_library = evidence / 'TEST-preserved-original-library'
        library_root.rename(held_library); shutil.copytree(held_library, library_root)
        unavailable_library = call('/api/library/create', library_request)
        assert unavailable_library['status'] == 'committed' and unavailable_library.get('deliveryError') and 'rootDocument' not in unavailable_library
        library_root.rename(evidence / 'TEST-preserved-substitute-library'); held_library.rename(library_root)
        library_delivered = call('/api/operations/status', {'operationId': library_request['operationId']})
        assert library_delivered['rootDocument']['nodeId'] == library_receipt['rootDocument']['nodeId']

        schema_basis = call('/api/data/schema', {'root': str(root)})
        schema_plan = call('/api/data/schema/prepare', {'root': str(root), 'operationId': test_hosted.uid(), 'source': schema_basis['source'], 'beforeSource': schema_basis['source'], 'beforeRevision': schema_basis['revision']})
        call('/api/data/schema/install', {'operationId': schema_plan['operationId'], 'planId': schema_plan['planId']})
        owner = created_document(call('/api/note/create', {**create, 'name': 'DefinitionReplay', 'operationId': test_hosted.uid()}))
        definition_request = {'sessionId': owner['sessionId'], 'baseId': owner['baseId'], 'source': owner['source'], 'serial': 1, 'label': 'weftext-query-original', 'plan': {'types': [], 'select': [], 'order': []}, 'operationId': test_hosted.uid()}
        definition = call('/api/query/definition/save', definition_request)
        definition_bytes = Path(owner['path']).read_bytes()
        stop(); start()  # Neither a live Core Session nor App location survives.
        definition_replay = call('/api/query/definition/save', definition_request)
        assert definition_replay['status'] == 'committed' and definition_replay['baseId'] == definition['baseId']
        assert definition_replay['document']['sessionId'] != owner['sessionId']
        for field, value in [('sessionId', test_hosted.uid()), ('baseId', test_hosted.uid()), ('source', owner['source'] + '\nchanged')]:
            assert call('/api/query/definition/save', {**definition_request, field: value}, 409)['error']['code'] == 'operationConflict'
        assert Path(owner['path']).read_bytes() == definition_bytes

        setup_request = {'root': str(root), 'operationId': test_hosted.uid()}
        setup = call('/api/sync/enable', setup_request)
        setup_fact = call('/api/operations/status', {'operationId': setup_request['operationId']})
        assert setup_fact['status'] == 'committed' and setup_fact['kind'] == 'enableSync'
        assert setup_fact['identity'] == setup['identity'] and setup_fact['replicaId'] == setup['replicaId']
        assert call('/api/operations/settle', fence(setup_request, 'operation')) == setup_fact

        cancelled = test_hosted.uid()
        call('/api/operations/settle', fence({'operationId': cancelled}, 'newNode'))
        blocked = call('/api/note/create', {**create, 'name': 'NeverCreated', 'operationId': cancelled}, 409)
        assert blocked['error']['code'] == 'operationCancelled' and not (root / 'NeverCreated').exists()
        backup = call('/api/operations/prepare', {'root': str(root), 'operationId': test_hosted.uid(), 'intent': {'kind': 'backupLibrary', 'destination': str(evidence / 'TEST-backup')}})
        assert call('/api/operations/commit', {'operationId': backup['operationId'], 'planId': backup['planId']})['status'] == 'committed'
        assert not (evidence / 'TEST-backup/private/journal' / f'{cancelled}.resolution.json').exists()
        # Reusing A's old root path for independently admitted B must neither
        # include A's private facts nor let A's bounded foreign header block B.
        reused = evidence / 'TEST-reused-library-root'
        library_a = call('/api/library/create', {'path': str(reused), 'operationId': test_hosted.uid()})
        a_request = {'root': str(reused), 'parent': str(reused), 'name': 'A', 'operationId': test_hosted.uid()}
        a_receipt = call('/api/note/create', a_request); a_doc = created_document(a_receipt)
        call('/api/draft', {'sessionId': a_doc['sessionId'], 'source': a_doc['source'] + '\nA original private draft'})
        device_input = evidence / 'TEST-device-input.typ'; device_input.write_text('device input', encoding='utf-8')
        device_doc = call('/api/open', {'path': str(device_input)})
        nested = reused / 'nested'; nested.mkdir()
        ordinary_path = nested / 'ordinary.typ'
        ordinary_as = {'sessionId': device_doc['sessionId'], 'path': str(ordinary_path), 'source': 'A PRIVATE ordinary SaveAs', 'operationId': test_hosted.uid()}
        ordinary_result = call('/api/save-as', ordinary_as)
        ordinary = call('/api/open', {'path': str(ordinary_path)})
        ordinary_save = {'sessionId': ordinary['sessionId'], 'baseId': ordinary['baseId'], 'source': 'A PRIVATE ordinary Save S', 'operationId': test_hosted.uid()}
        call('/api/save', ordinary_save)
        call('/api/draft', {'sessionId': ordinary['sessionId'], 'source': 'A PRIVATE later input Y'})
        a_ids = set()
        for file in (state / 'journal').glob('*.plan.json'):
            saved_plan = json.loads(file.read_text('utf-8'))
            scope = saved_plan.get('fileSave', {}).get('scope', {})
            if saved_plan.get('workspaceId') == library_a['workspaceId'] or scope.get('binding', {}).get('workspaceId') == library_a['workspaceId']:
                a_ids.add(saved_plan['operationId'])
        assert {ordinary_as['operationId'], ordinary_save['operationId']} <= a_ids
        a_private = evidence / 'TEST-A-complete-backup'
        a_backup = call('/api/operations/prepare', {'root': str(reused), 'operationId': test_hosted.uid(), 'intent': {'kind': 'backupLibrary', 'destination': str(a_private)}})
        call('/api/operations/commit', {'operationId': a_backup['operationId'], 'planId': a_backup['planId']})
        assert all((a_private / 'private/journal' / f'{op}.plan.json').is_file() for op in (ordinary_as['operationId'], ordinary_save['operationId']))
        a_journal = {file: file.read_bytes() for file in (state / 'journal').glob('*.json') if file.name.split('.')[0] in a_ids}
        retained_a = evidence / 'TEST-preserved-A-root'; reused.rename(retained_a)
        library_b = call('/api/library/create', {'path': str(reused), 'operationId': test_hosted.uid()})
        assert library_b['workspaceId'] != library_a['workspaceId']
        call('/api/draft', {'sessionId': ordinary['sessionId'], 'source': 'A PRIVATE input after path reuse'})
        stop(); start()  # The original scope must survive a Core restart.
        b_request = {**a_request, 'name': 'B', 'operationId': test_hosted.uid()}
        b_doc = created_document(call('/api/note/create', b_request))
        call('/api/draft', {'sessionId': b_doc['sessionId'], 'source': b_doc['source'] + '\nB selected private draft'})
        b_backup = evidence / 'TEST-B-only-backup'
        b_plan = call('/api/operations/prepare', {'root': str(reused), 'operationId': test_hosted.uid(), 'intent': {'kind': 'backupLibrary', 'destination': str(b_backup)}})
        assert call('/api/operations/commit', {'operationId': b_plan['operationId'], 'planId': b_plan['planId']})['status'] == 'committed'
        for file, original in a_journal.items():
            assert file.read_bytes() == original and not (b_backup / 'private/journal' / file.name).exists()
        assert (retained_a / 'A/A.typ').read_text('utf-8') == a_doc['source']
        private_drafts = list((b_backup / 'private/drafts').glob('*.json'))
        assert private_drafts and all(json.loads(file.read_text('utf-8'))['scope']['binding']['workspaceId'] == library_b['workspaceId'] for file in private_drafts)
        orphan = test_hosted.uid()
        original_evidence = state / 'journal' / f'{orphan}.install.json'
        original_evidence.write_bytes(b'TEST retained unknown install evidence')
        blocked = call('/api/operations/settle', fence({'operationId': orphan}, 'operation'), 409)
        assert blocked['error']['code'] == 'resolutionUnavailable'
        alias = call('/api/operations/settle', fence({'operationId': orphan.upper()}, 'operation'), 409)
        assert alias['error']['code'] == 'invalidOperationId'
        assert not (state / 'journal' / f'{orphan}.resolution.json').exists()
        result['passed'] = True
    except BaseException:
        result['failure'] = traceback.format_exc()
        raise
    finally:
        try:
            stop()
        finally:
            events.close()
            result['ownedProcesses'] = owned
            (evidence / 'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
            inventory = []
            for file in sorted(evidence.rglob('*')):
                if file.is_file():
                    data = file.read_bytes()
                    inventory.append({'path': str(file.relative_to(evidence)), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
            (evidence / 'artifact-manifest.json').write_text(json.dumps(inventory, ensure_ascii=False, indent=2), encoding='utf-8')
            print('Retained commit/delivery evidence:', evidence, flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('executable', type=Path)
    parser.add_argument('evidence', type=Path, nargs='?')
    args = parser.parse_args()
    evidence = args.evidence.resolve() if args.evidence else Path(tempfile.mkdtemp(prefix='weftext-delivery-parent-')) / 'run'
    run(args.executable.resolve(), evidence)
