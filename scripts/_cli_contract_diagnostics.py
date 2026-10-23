"""Bounded observation of an already-failed CLI case; no product execution."""
from collections import deque
import json
import os
from pathlib import Path
import re
import sys

from _cli_contract_common import digest, file_digest, read_small, unique_json


def source_identity(entry_path, support_paths):
    """Bind the actual caller entry and loaded support sources, never this file alone."""
    entry = Path(entry_path).resolve(strict=True)
    supports = {}
    for supplied in support_paths:
        path = Path(supplied).resolve(strict=True)
        if path.name in supports:
            raise ValueError('Duplicate CLI support source name')
        supports[path.name] = {'path': str(path), 'sha256': file_digest(path)}
    return {'entry': {'path': str(entry), 'sha256': file_digest(entry)},
            'supportModules': supports}


def diagnostic_text(value, redactions, limit):
    # Apply complete-value redaction before cutting UTF-8, including at a boundary.
    for secret in redactions:
        value = value.replace(secret, '[REDACTED]')
    raw = value.encode('utf-8', errors='replace')
    return {'text': raw[-limit:].decode('utf-8', errors='replace'),
            'truncated': len(raw) > limit}


def _machine_record(value, stream):
    if not isinstance(value, dict) or type(value.get('schemaVersion')) is not int or value['schemaVersion'] != 1:
        return None
    error = value.get('error')
    if stream == 'stdout' and isinstance(value.get('result'), dict):
        error = value['result'].get('error')
    if not isinstance(error, dict):
        return None
    record = {}
    code = error.get('code')
    if isinstance(code, str) and re.fullmatch(r'[A-Za-z0-9_-]{1,80}', code):
        record['code'] = code
    if type(error.get('exitCode')) is int:
        record['exitCode'] = error['exitCode']
    for field, output in (('explanation', 'explanationKey'), ('description', 'descriptionKey')):
        message = error.get(field)
        key = message.get('key') if isinstance(message, dict) else None
        if isinstance(key, str) and re.fullmatch(r'[A-Za-z][A-Za-z0-9_.-]{0,199}', key):
            record[output] = key
    if stream == 'stdout' and type(value.get('httpStatus')) is int and 100 <= value['httpStatus'] <= 599:
        record['httpStatus'] = value['httpStatus']
    return record or None


def stream_diagnostic(path, stream, redactions):
    raw = read_small(path)
    records, count, unparsed = deque(maxlen=4), 0, 0
    for line in raw.splitlines():
        if not line.strip():
            continue
        try:
            if len(line) > 16384:
                raise ValueError('Diagnostic line exceeds the bounded parse')
            record = _machine_record(unique_json(line), stream)
            if record is None:
                raise ValueError('Not a versioned machine error')
            records.append(diagnostic_text(json.dumps(record, ensure_ascii=True), redactions, 1024))
            count += 1
        except (ValueError, UnicodeError):
            unparsed += 1
    return {'length': len(raw), 'sha256': digest(raw), 'machineErrorCount': count,
            'recentMachineErrors': list(records), 'unparsedLines': unparsed}


def failure_diagnostic(item, processes, output, redactions, sources, relay=None):
    """Only observe explicit current-case evidence; a broken sink cannot escape."""
    try:
        recent_processes = []
        for process in processes[-4:]:
            recent = {key: process[key] for key in (
                'label', 'exitCode', 'expectedExitCodes', 'timedOut',
                'elapsedSeconds', 'stdout', 'stderr') if key in process}
            for stream in ('stdout', 'stderr'):
                try:
                    recent[stream + 'Diagnostic'] = stream_diagnostic(
                        output / process[stream], stream, redactions)
                except (OSError, AssertionError) as error:
                    recent[stream + 'Unavailable'] = type(error).__name__
            recent_processes.append(recent)
        diagnostic = {'case': item['name'], 'status': item['status'],
            'resultPath': str(output / 'result.json'),
            'cliDriverSha256': sources['entry']['sha256'],
            'cliSupportSources': sources['supportModules'],
            'python': sys.version, 'checkoutCommit': os.environ.get('GITHUB_SHA'),
            'reason': diagnostic_text(item.get('error', item.get('reason', '')), redactions, 8192),
            'processes': recent_processes}
        if relay is not None:
            diagnostic['relay'] = {key: relay[key] for key in (
                'relayDrained', 'relayOwnedConnectionsRemaining',
                'relayListenerClosed', 'relayErrors') if key in relay}
            diagnostic['relay']['recentRequests'] = relay.get('relayRequests', [])[-4:]
        item['failureDiagnostic'] = diagnostic
        print('CLI failure diagnostic: ' + json.dumps(diagnostic, ensure_ascii=True),
              file=sys.stderr, flush=True)
    except Exception as error:
        try:
            print('CLI failure diagnostic unavailable: ' + type(error).__name__,
                  file=sys.stderr, flush=True)
        except Exception:
            pass  # Observation must not replace the original case/cleanup.
