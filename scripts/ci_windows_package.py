"""CI identities and original evidence around the unchanged product entry."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import zipfile

import mvp
from _cli_serve import _io_path
from package_inputs import is_reparse, parse_json, read_checked, sha256

ROOT = Path(__file__).resolve().parents[1]


def write_json(path, value):
    with _io_path(path).open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def output(name, value):
    value = str(value)
    if '\r' in value or '\n' in value:
        raise ValueError('CI scalar output contains a newline')
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8', newline='\n') as stream:
        stream.write(f'{name}={value}\n')


def ordinary(path):
    for parent in (path, *path.parents):
        if _io_path(parent).exists() and is_reparse(_io_path(parent).lstat()):
            raise ValueError('CI evidence path contains a reparse point')
    return path


def children(path):
    ordinary(path)
    return sorted(item.name for item in _io_path(path).iterdir()) if _io_path(path).exists() else []


def new_children(path, previous):
    return [path / name for name in children(path) if name not in previous]


def artifact_identity(artifacts):
    if not isinstance(artifacts, mvp.QualifiedArtifacts) or artifacts.receipt_path is None:
        raise ValueError('CI requires the new qualified Windows batch')
    return {'receipt': str(artifacts.receipt_path), 'batch': str(artifacts.receipt_path.parent),
            'sourceRoot': str(artifacts.source_root), 'profile': artifacts.profile,
            'buildId': parse_json(artifacts.catalog)['buildId'],
            'artifactCatalogSha256': sha256(artifacts.catalog)}


def initialize():
    environment = dict(os.environ)
    target = mvp.validate_target(ROOT, mvp.metadata(ROOT, environment)['target_directory'])
    evidence = Path(os.environ['RUNNER_TEMP']) / (
        'weftext-ci-' + os.environ['GITHUB_RUN_ID'] + '-' + os.environ['GITHUB_RUN_ATTEMPT'])
    ordinary(evidence)
    evidence.mkdir(exist_ok=False)
    (evidence / 'temp').mkdir()
    output('evidence', evidence)
    output('temp', evidence / 'temp')
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
    fixed = ROOT / 'target/test-fixtures/hierarchical-tags'
    provenance = {'schemaVersion': 1, 'scope': 'CI identities; never cached check results',
        'repository': os.environ['GITHUB_REPOSITORY'], 'runId': os.environ['GITHUB_RUN_ID'],
        'runAttempt': os.environ['GITHUB_RUN_ATTEMPT'], 'githubSha': os.environ['GITHUB_SHA'],
        'checkoutCommit': git('rev-parse', 'HEAD'), 'checkoutTree': git('rev-parse', 'HEAD^{tree}'),
        'targetRoot': str(target), 'checkRootsBefore': children(target / 'weftext-checks'),
        'batchesBefore': children(target / 'weftext-build-artifacts'),
        'distBefore': children(ROOT / 'dist'),
        'fixedControlRootsBefore': {str(fixed): children(fixed)}}
    write_json(evidence / 'provenance.json', provenance)


def verify_receipt(evidence, artifacts):
    """Run the original receipt CLI in a fresh process on this package batch."""
    source, receipt = artifacts.source_root, artifacts.receipt_path
    python = Path(sys.executable)
    if not python.is_absolute() or not source.is_absolute() or not receipt.is_absolute():
        raise ValueError('The receipt child requires the selected absolute Python, source and receipt')
    command = [str(python), str(source / 'scripts/mvp.py'), 'check', '--release',
               '--group', 'cli', '--artifacts', str(receipt)]
    environment = {**os.environ, **artifacts.build_environment}
    stdout, stderr = evidence / 'receipt-cli.stdout.bin', evidence / 'receipt-cli.stderr.bin'
    record = {**artifact_identity(artifacts), 'schemaVersion': 1,
        'scope': 'independent receipt and CLI check; never replaces canonical full checks',
        'command': command, 'cwd': str(source), 'python': str(python),
        'stdout': str(stdout), 'stderr': str(stderr), 'exitCode': None,
        'startedUtc': datetime.now(timezone.utc).isoformat(), 'status': 'invoking'}
    write_json(evidence / 'receipt-cli-intent.json', record)
    try:
        with _io_path(stdout).open('xb') as out, _io_path(stderr).open('xb') as err:
            with subprocess.Popen(command, cwd=source, env=environment,
                    stdin=subprocess.DEVNULL, stdout=out, stderr=err) as process:
                record['pid'] = process.pid
                interrupted, previous, deferred = None, None, False
                try:
                    while True:
                        try:
                            record['exitCode'] = process.wait()
                            break
                        except (KeyboardInterrupt, SystemExit):
                            interrupted = interrupted or sys.exc_info()
                            if not deferred and threading.current_thread() is threading.main_thread():
                                try:
                                    previous = signal.signal(signal.SIGINT, signal.SIG_IGN)
                                except Exception as error:
                                    interrupted[1].add_note(f'Could not defer later SIGINT: {error!r}')
                                else:
                                    deferred = True
                finally:
                    if deferred:
                        try:
                            signal.signal(signal.SIGINT, previous)
                        except Exception as error:
                            interrupted[1].add_note(f'Could not restore SIGINT handler: {error!r}')
                if interrupted is not None:
                    raise interrupted[1].with_traceback(interrupted[2])
        if record['exitCode']:
            raise subprocess.CalledProcessError(record['exitCode'], command)
        record['status'] = 'passedPartialCliOnly'
    except BaseException as error:
        record['status'] = 'failedOriginalsRetained'
        record['error'] = repr(error)
        raise
    finally:
        record['endedUtc'] = datetime.now(timezone.utc).isoformat()
        primary = sys.exception()
        try:
            write_json(evidence / 'receipt-cli-result.json', record)
        except Exception as error:
            if primary is None:
                raise
            primary.add_note(f'Receipt child result write also failed: {error!r}')


def stream_identity(path):
    digest, count = hashlib.sha256(), 0
    ordinary(path)
    with _io_path(path).open('rb') as stream:
        while block := stream.read(1024 * 1024):
            count += len(block)
            digest.update(block)
    return {'bytes': count, 'sha256': digest.hexdigest()}


def match_package(dist, previous, identity):
    """Only the just-returned full package can qualify an upload."""
    candidates = new_children(dist, previous)
    if len(candidates) != 1:
        raise ValueError('Canonical package must create exactly one new candidate')
    destination = candidates[0]
    ordinary(destination)
    if _io_path(destination / 'INCOMPLETE.json').exists():
        raise ValueError('Incomplete package cannot qualify')
    receipt = parse_json(read_checked(destination / 'PACKAGE.json', destination))
    if any(receipt[key] != identity[key] for key in ('buildId', 'artifactCatalogSha256')):
        raise ValueError('Package is not the exact returned product batch')
    if any(receipt[key] is not False for key in ('signed', 'installed', 'upgradeQualified', 'dataPreservationQualified')):
        raise ValueError('CI generates unsigned review artifacts only')
    containers = receipt['containers']
    if (not isinstance(containers, list) or len(containers) != 2
            or any(not isinstance(row, dict) or set(row) != {'path', 'bytes', 'sha256'} for row in containers)
            or {Path(row['path']).suffix for row in containers} != {'.zip', '.msix'}
            or any(Path(row['path']).name != row['path'] or '\\' in row['path'] or ':' in row['path']
                   for row in containers)):
        raise ValueError('Unexpected package container names')
    for row in containers:
        if stream_identity(destination / row['path']) != {key: row[key] for key in ('bytes', 'sha256')}:
            raise ValueError('Package container changed after canonical generation')
    build_bytes = read_checked(destination / 'payload/BUILD.json', destination)
    build_rows = [row for row in receipt['commonPayload'] if row['path'] == 'BUILD.json']
    if build_rows != [{'path': 'BUILD.json', 'bytes': len(build_bytes), 'sha256': sha256(build_bytes)}]:
        raise ValueError('BUILD original bytes changed after canonical generation')
    build = parse_json(build_bytes)
    if any(build[key] != identity[key] for key in ('buildId', 'artifactCatalogSha256')):
        raise ValueError('BUILD does not match the exact product batch')
    return destination, receipt


def package(evidence, preparation):
    provenance = parse_json(read_checked(evidence / 'provenance.json', evidence))
    prepared = parse_json(read_checked(preparation, preparation.parent))
    if prepared['status'] != 'preparedInputsOnly':
        raise ValueError('Windows packaging inputs were not prepared')
    original_build = mvp.build
    def observed_build(*args, **kwargs):
        # Validate this canonical batch independently before returning it to the
        # original full check. No original group or PASS result is replaced.
        artifacts = original_build(*args, **kwargs)
        identity = artifact_identity(artifacts)
        if str(artifacts.target_directory) != provenance['targetRoot'] or artifacts.source_root == ROOT:
            raise ValueError('Package must use its new frozen source and this attempt target')
        fixed = artifacts.source_root / 'target/test-fixtures/hierarchical-tags'
        identity['fixedControlRootsBefore'] = {str(fixed): children(fixed)}
        write_json(evidence / 'package-batch.json', identity)
        write_json(evidence / 'cargo-metadata.json', artifacts.metadata)
        verify_receipt(evidence, artifacts)
        return artifacts
    try:
        mvp.build = observed_build
        artifacts = mvp.main(['package', '--release', '--windows-sdk', prepared['sdk']['directory'],
            '--offline-materials', prepared['materialsDirectory'], '--msvc-linker', prepared['linker']['path']])
    finally:
        mvp.build = original_build
    identity = artifact_identity(artifacts)
    destination, receipt = match_package(ROOT / 'dist', provenance['distBefore'], identity)
    write_json(evidence / 'package-success.json', {**identity, 'directory': str(destination),
        'containers': receipt['containers'], 'scope': 'full canonical package succeeded; unsigned and uninstalled'})


def archive_controls(evidence, roots):
    def traversal_failed(error):
        raise error
    with zipfile.ZipFile(_io_path(evidence / 'process-controls.zip'), 'x', zipfile.ZIP_DEFLATED) as archive:
        for label, root in roots:
            ordinary(root)
            if not _io_path(root).is_dir():
                raise ValueError('CI process evidence root is not a directory')
            # Encode only filesystem access. Logical paths and original control
            # bytes retain their exact spelling, including in the ZIP inventory.
            for directory, dirs, names in os.walk(_io_path(root), followlinks=False, onerror=traversal_failed):
                io_folder = Path(directory)
                dirs[:] = [name for name in dirs if not is_reparse((io_folder / name).lstat())]
                relative = io_folder.relative_to(_io_path(root))
                if not any(part.startswith(('server-control-', 'owner-control-')) or part == 'cli-results'
                           for part in relative.parts):
                    continue
                for name in sorted(names):
                    path = io_folder / name
                    if is_reparse(path.lstat()):
                        raise ValueError('CI process evidence file is a reparse point')
                    archive.write(path, (Path(label) / relative / name).as_posix())


def collect(evidence):
    provenance = parse_json(read_checked(evidence / 'provenance.json', evidence))
    target = Path(provenance['targetRoot'])
    new_checks = new_children(target / 'weftext-checks', provenance['checkRootsBefore'])
    roots = [('http-temp', evidence / 'temp'), *[(path.name, path) for path in new_checks]]
    fixed_roots = dict(provenance['fixedControlRootsBefore'])
    if _io_path(evidence / 'package-batch.json').exists():
        package_batch = parse_json(read_checked(evidence / 'package-batch.json', evidence))
        fixed_roots.update(package_batch['fixedControlRootsBefore'])
    for fixed, previous in fixed_roots.items():
        roots.extend(('hierarchical-tags-' + path.name, path) for path in new_children(Path(fixed), previous))
    write_json(evidence / 'process-roots.json', {'temporaryRoot': str(evidence / 'temp'),
        'newCheckRoots': list(map(str, new_checks)),
        'selectedRoots': [{'label': label, 'path': str(root)} for label, root in roots]})
    archive_controls(evidence, roots)
    paths = [evidence / '*.json', evidence / 'process-controls.zip',
        evidence / 'receipt-cli.stdout.bin', evidence / 'receipt-cli.stderr.bin',
        *new_children(target / 'weftext-build-artifacts', provenance['batchesBefore'])]
    new_dist = new_children(ROOT / 'dist', provenance['distBefore'])
    success = evidence / 'package-success.json'
    if _io_path(success).exists():
        record = parse_json(read_checked(success, evidence))
        destination, receipt = match_package(ROOT / 'dist', provenance['distBefore'], record)
        paths.extend([destination / row['path'] for row in receipt['containers']])
        paths.extend(destination / name for name in ('PACKAGE.json', 'payload/BUILD.json', 'sdk-logs',
            'AppxManifest.xml', 'mapping.txt', 'priconfig.xml', 'resource-input'))
    else:
        # Never label a partial SDK output as a candidate. Preserve it for diagnosis.
        paths.extend(new_dist)
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8', newline='\n') as stream:
        stream.write('paths<<WEFTEXT_CI_PATHS\n')
        for path in paths:
            if any(char in str(path) for char in '\r\n'):
                raise ValueError('CI upload path contains a newline')
            stream.write(str(path) + '\n')
        stream.write('WEFTEXT_CI_PATHS\n')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('init', 'package', 'collect'))
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--preparation', type=Path)
    args = parser.parse_args(argv)
    if args.action == 'init':
        initialize()
    else:
        if args.evidence is None or args.action == 'package' and args.preparation is None:
            parser.error('This phase requires explicit evidence/preparation paths')
        if args.action == 'package':
            package(args.evidence, args.preparation)
        else:
            collect(args.evidence)


if __name__ == '__main__':
    main()
