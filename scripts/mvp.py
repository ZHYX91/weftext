"""Build, check or package the local MVP. No publishing and no workspace-private inputs."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
NPM = shutil.which('npm.cmd' if os.name == 'nt' else 'npm') or 'npm'
IGNORED = {'.git', 'node_modules', 'target', 'dist', '.workspace', '__pycache__'}
VERSION = tomllib.loads((ROOT / 'crates/weftext-app/Cargo.toml').read_text(encoding='utf-8'))['package']['version']


def run(*args, capture=False, env=None):
    return subprocess.run([str(x) for x in args], cwd=ROOT, check=True,
                          capture_output=capture, text=True, encoding='utf-8', env=env).stdout


def build(release):
    run(NPM, '--prefix', 'app', 'ci', '--ignore-scripts')
    run(NPM, '--prefix', 'app', 'run', 'build')
    args = ['cargo', 'build', '--locked', '-p', 'weftext-app',
            '--message-format=json-render-diagnostics']
    if release:
        args.append('--release')
    executable = None
    # Cargo owns the output location, including target-dir and target overrides.
    # Never check or package an old default-target executable after a new build.
    with subprocess.Popen(args, cwd=ROOT, stdout=subprocess.PIPE,
                          text=True, encoding='utf-8') as process:
        for line in process.stdout:
            message = json.loads(line)
            if (message.get('reason') == 'compiler-artifact'
                    and message.get('target', {}).get('name') == 'weftext'
                    and 'bin' in message['target']['kind']
                    and message.get('executable')):
                executable = Path(message['executable'])
        if process.wait():
            raise subprocess.CalledProcessError(process.returncode, args)
    if executable is None or not executable.is_file():
        raise RuntimeError('Cargo did not report the built Weftext executable')
    print(f'Built executable: {executable}')
    return executable


def source_files(target_directory):
    for directory, children, names in os.walk(ROOT):
        children[:] = [name for name in children if name not in IGNORED
                       and Path(directory) / name != target_directory]
        for name in names:
            p = Path(directory) / name
            if p.suffix not in {'.pyc', '.pyo'}:
                yield p


def package(executable):
    # Refuse to overwrite an existing candidate; a tested artifact keeps its identity.
    import datetime
    metadata = json.loads(run('cargo', 'metadata', '--locked', '--format-version', '1', capture=True))
    target_directory = Path(metadata['target_directory'])
    if (target_directory.is_relative_to(ROOT)
            and not target_directory.is_relative_to(ROOT / 'target')):
        raise RuntimeError('Packaging requires target/ or a Cargo target directory outside the source tree')
    stamp = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    destination = ROOT / 'dist' / f'weftext-{VERSION}-local-{stamp}'
    destination.mkdir(parents=True, exist_ok=False)
    shutil.copy2(executable, destination / executable.name)
    shutil.copy2(ROOT / 'app/example.typ', destination / 'example.typ')
    for name in ['LICENSE', 'NOTICE']:
        shutil.copy2(ROOT / name, destination / name)
    for lang in ['', '.zh-CN']:
        guide = (ROOT / f'docs/MVP{lang}.md').read_text(encoding='utf-8')
        guide = re.sub(r'^---\n.*?\n---\n', '', guide, flags=re.S)
        guide = re.sub(r'\[([^]]+)\]\([^)]+\)', r'\1 (source.zip/docs)', guide)
        (destination / f'README{lang}.txt').write_text(guide, encoding='utf-8')
    with zipfile.ZipFile(destination / 'source.zip', 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as archive:
        for file in source_files(target_directory):
            archive.write(file, file.relative_to(ROOT))
    inventory = []
    licenses = destination / 'third-party'

    def collect(name, version, folder, license_id, repository, ecosystem):
        label = f'{ecosystem}-{name.replace("/", "-").replace("@", "")}-{version}'
        copied = []
        for directory, children, names in os.walk(folder):
            children[:] = [n for n in children if n not in IGNORED]
            for filename in names:
                if filename.lower().startswith(('license', 'licence', 'copying', 'notice', 'ofl')):
                    file = Path(directory) / filename
                    relative = file.relative_to(folder)
                    target = licenses / label / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(file, target)
                    copied.append(str(target.relative_to(destination)))
        inventory.append({'ecosystem': ecosystem, 'name': name, 'version': version,
                          'license': license_id, 'repository': repository, 'licenseFiles': copied})

    # Conservative build inventory includes transitive and test/build dependencies.
    # It does not claim every listed package is shipped or that system fonts are bundled.
    for item in metadata['packages']:
        if item.get('source') or '/vendor/' in Path(item['manifest_path']).as_posix():
            collect(item['name'], item['version'], Path(item['manifest_path']).parent,
                    item.get('license'), item.get('repository'), 'cargo')
    lock = json.loads((ROOT / 'app/package-lock.json').read_text(encoding='utf-8'))
    for relative, item in lock['packages'].items():
        if not relative:
            continue
        folder = ROOT / 'app' / relative
        if not folder.exists():
            continue  # Optional dependencies for another build platform.
        manifest = json.loads((folder / 'package.json').read_text(encoding='utf-8'))
        collect(manifest['name'], manifest['version'], folder, manifest.get('license'),
                manifest.get('repository'), 'npm')
    fonts = ROOT / 'resources/fonts'
    collect('weftext-offline-fonts', VERSION, fonts, 'See the exact per-font manifests and original notices', None, 'assets')
    for name in ['manifest.json', 'builtin-manifest.json', 'web-manifest.json']:
        shutil.copy2(fonts / name, destination / f'FONTS-{name}')
    (destination / 'DEPENDENCIES.json').write_text(json.dumps({
        'scope': 'Installed build dependency inventory including test/build tools, the local Typst HTML patch, and compiler/browser font assets. Only the exact font manifests describe bundled fonts; unrelated system fonts are not copied.',
        'packages': inventory}, ensure_ascii=False, indent=2), encoding='utf-8')
    binary = destination / executable.name
    (destination / 'BUILD.json').write_text(json.dumps({
        'version': f'{VERSION}-local', 'platform': sys.platform,
        'profile': executable.parent.name, 'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'sourceSha256': hashlib.sha256((destination / 'source.zip').read_bytes()).hexdigest(),
        'status': 'local candidate, not a published release',
        'source': 'source.zip contains this working-tree source, including uncommitted changes',
        'recognition': {
            'runtimeIncluded': False,
            'setup': 'source.zip/scripts/recognition/README.md',
            'runtimeLockSha256': hashlib.sha256((ROOT / 'resources/recognition/runtime-lock.json').read_bytes()).hexdigest(),
            'embeddedScripts': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in
                                (ROOT / 'scripts/recognition').glob('*.py')
                                if p.name in {'worker.py', 'engine.py', 'install.py', 'schemes.py', 'docx_reader.py', 'product_worker.py'}},
        },
    }, indent=2), encoding='utf-8')
    archive = str(destination) + '.zip'
    # Registry license files can carry an epoch timestamp; ZIP's lower bound is 1980.
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, strict_timestamps=False) as bundle:
        for file in destination.rglob('*'):
            if file.is_file():
                bundle.write(file, file.relative_to(destination.parent))
    print(f'Local candidate: {destination}\nArchive: {archive}')
    return destination


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['build', 'check', 'package'])
    parser.add_argument('--release', action='store_true', help='Use the optimized Rust profile')
    args = parser.parse_args()
    exe = build(args.release)
    if args.action in {'check', 'package'}:
        run(sys.executable, 'scripts/check.py', '--models')
        run(sys.executable, 'scripts/recognition/test_schemes.py')
        run(sys.executable, '-S', 'scripts/recognition/test_docx_reader.py')
        run(sys.executable, 'resources/fonts/verify.py')
        run(NPM, '--prefix', 'app', 'test')
        run('cargo', 'fmt', '--all', '--', '--check')
        run('cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--', '-D', 'warnings')
        run('cargo', 'test', '--locked', '--workspace', env={**os.environ, 'WEFTEXT_TEST_EXECUTABLE': str(exe)})
        run(sys.executable, 'scripts/test_mvp.py', exe)
        run(sys.executable, 'scripts/test_commit_delivery.py', exe)
        run(sys.executable, 'scripts/test_local_features.py', exe)
        run(sys.executable, 'scripts/test_outputs.py', exe)
        run(sys.executable, 'scripts/test_v05_completion.py', exe)
        run(sys.executable, 'scripts/test_review_interchange.py', exe)
        run(sys.executable, 'scripts/test_hosted.py', exe)
        # Includes source/editor/field boundaries and separately authorized review.
        run(sys.executable, 'scripts/test_hosted_review.py', exe)
        run(sys.executable, 'scripts/test_hosted_office.py', exe)
        run(sys.executable, 'scripts/test_hosted_citations_outputs.py', exe)
        run(sys.executable, 'scripts/test_v061_authoring.py', exe)
        run(sys.executable, 'scripts/test_everyday.py', exe)
        run(sys.executable, 'scripts/test_query_organization.py', exe)
        run(sys.executable, 'scripts/test_hierarchical_tags.py', exe)
        run(sys.executable, 'scripts/test_link_diagnostics.py', exe)
        run(sys.executable, 'scripts/test_v061_processing.py', exe)
        run(sys.executable, 'scripts/test_v061_processing_integration.py', exe)
        run(sys.executable, 'scripts/test_recognition.py', exe)
        run(sys.executable, 'scripts/test_hosted_owner_backup.py', exe)
        run(sys.executable, 'scripts/test_hosted_content_import.py', exe)
        run(sys.executable, 'scripts/test_hosted_query.py', exe)
        run(sys.executable, 'scripts/test_hosted_providers.py', exe)
        run(sys.executable, 'scripts/test_hosted_scopes.py', exe)
        run(sys.executable, 'scripts/test_hosted_library.py', exe)
        run(sys.executable, 'scripts/test_hosted_media.py', exe)
        run(sys.executable, 'scripts/test_hosted_resource_collections.py', exe)
        run(sys.executable, 'scripts/test_hosted_definition.py', exe)
        run(sys.executable, 'scripts/test_hosted_history.py', exe)
        run(sys.executable, 'scripts/test_hosted_navigation_fragments.py', exe)
        run(sys.executable, 'scripts/test_hosted_structure.py', exe)
        run(sys.executable, 'scripts/test_hosted_knowledge_links.py', exe)
        run(sys.executable, 'scripts/test_hosted_navigation_registry.py', exe)
        run(sys.executable, 'scripts/test_sync.py', exe)
        run(sys.executable, 'scripts/test_sync_compound.py', exe)
        run(sys.executable, 'scripts/test_sync_revalidation.py', exe)
        run(sys.executable, 'scripts/test_sync_structure.py', exe)
        run(sys.executable, 'scripts/test_sync_placement.py', exe)
        run(sys.executable, 'scripts/test_sync_retirement.py', exe)
        run(sys.executable, 'scripts/test_sync_history.py', exe)
        run(sys.executable, 'scripts/test_remote_sync.py', exe)
    if args.action == 'package':
        package(exe)
