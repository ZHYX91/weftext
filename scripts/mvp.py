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


def run(*args, capture=False):
    return subprocess.run([str(x) for x in args], cwd=ROOT, check=True,
                          capture_output=capture, text=True, encoding='utf-8').stdout


def build(release):
    run(NPM, '--prefix', 'app', 'ci', '--ignore-scripts')
    run(NPM, '--prefix', 'app', 'run', 'build')
    args = ['cargo', 'build', '--locked', '-p', 'weftext-app']
    if release:
        args.append('--release')
    run(*args)
    return ROOT / 'target' / ('release' if release else 'debug') / ('weftext.exe' if os.name == 'nt' else 'weftext')


def source_files():
    for directory, children, names in os.walk(ROOT):
        children[:] = [name for name in children if name not in IGNORED]
        for name in names:
            p = Path(directory) / name
            if p.suffix not in {'.pyc', '.pyo'}:
                yield p


def package(executable):
    # Refuse to overwrite an existing candidate; a tested artifact keeps its identity.
    import datetime
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
        for file in source_files():
            archive.write(file, file.relative_to(ROOT))
    metadata = json.loads(run('cargo', 'metadata', '--locked', '--format-version', '1', capture=True))
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
        'status': 'local candidate, not a published release',
        'source': 'source.zip contains this working-tree source, including uncommitted changes',
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
        run(sys.executable, 'resources/fonts/verify.py')
        run(NPM, '--prefix', 'app', 'test')
        run('cargo', 'fmt', '--all', '--', '--check')
        run('cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--', '-D', 'warnings')
        run('cargo', 'test', '--locked', '--workspace')
        run(sys.executable, 'scripts/test_mvp.py', exe)
        run(sys.executable, 'scripts/test_local_features.py', exe)
        run(sys.executable, 'scripts/test_outputs.py', exe)
        run(sys.executable, 'scripts/test_v05_completion.py', exe)
        run(sys.executable, 'scripts/test_review_interchange.py', exe)
        # Includes source/editor/field boundaries and separately authorized review.
        run(sys.executable, 'scripts/test_hosted_review.py', exe)
        run(sys.executable, 'scripts/test_hosted_office.py', exe)
        run(sys.executable, 'scripts/test_hosted_query.py', exe)
        run(sys.executable, 'scripts/test_hosted_providers.py', exe)
        run(sys.executable, 'scripts/test_hosted_scopes.py', exe)
        run(sys.executable, 'scripts/test_hosted_library.py', exe)
        run(sys.executable, 'scripts/test_hosted_media.py', exe)
        run(sys.executable, 'scripts/test_hosted_definition.py', exe)
        run(sys.executable, 'scripts/test_hosted_history.py', exe)
        run(sys.executable, 'scripts/test_sync.py', exe)
        run(sys.executable, 'scripts/test_remote_sync.py', exe)
    if args.action == 'package':
        package(exe)
