"""Small stateless encoders, evidence reads and assertions for CLI acceptance."""
import datetime as dt
import hashlib
import json

MAX_LOG_READ = 26 * 1024 * 1024


class Nonrun(RuntimeError):
    """An explicit missing qualification, never silently treated as PASS."""

def utc():
    return dt.datetime.now(dt.timezone.utc).isoformat()

def digest(data):
    return hashlib.sha256(data).hexdigest()

def file_digest(path):
    with path.open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()

def require(condition, message):
    if not condition:
        raise AssertionError(message)

def unique_json(data):
    def pairs(items):
        out = {}
        for key, value in items:
            if key in out:
                raise ValueError('duplicate JSON key: ' + key)
            out[key] = value
        return out
    return json.loads(data.decode('utf-8'), object_pairs_hook=pairs)

def read_small(path):
    with path.open('rb') as file:
        data = file.read(MAX_LOG_READ + 1)
    require(len(data) <= MAX_LOG_READ, f'Output exceeded the bounded test read: {path}')
    return data
