"""Actual local HTTP commit/delivery and original-attempt recovery, in owned TEST data.

All evidence and original files are retained. Processes stop by an exact typed
owner shutdown; this test does not certify UI shutdown or power loss.
"""
import argparse
import hashlib
import json
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

        cancelled = test_hosted.uid()
        call('/api/operations/settle', fence({'operationId': cancelled}, 'newNode'))
        blocked = call('/api/note/create', {**create, 'name': 'NeverCreated', 'operationId': cancelled}, 409)
        assert blocked['error']['code'] == 'operationCancelled' and not (root / 'NeverCreated').exists()
        backup = call('/api/operations/prepare', {'root': str(root), 'operationId': test_hosted.uid(), 'intent': {'kind': 'backupLibrary', 'destination': str(evidence / 'TEST-backup')}})
        assert call('/api/operations/commit', {'operationId': backup['operationId'], 'planId': backup['planId']})['status'] == 'committed'
        assert not (evidence / 'TEST-backup/private/journal' / f'{cancelled}.resolution.json').exists()
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
