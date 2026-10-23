"""Build/check the product or generate an unsigned, qualified Windows candidate."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib
import uuid

from cargo_cache import CargoCache
from package_inputs import BuildArtifacts, collect_artifact, read_checked, require_checkout, sha256, snapshot, validate_target
from windows_package import artifact_catalog, load_contract, msix_version, package_windows, pe_info, sdk_tools
from windows_materials import freeze_materials

ROOT = Path(__file__).resolve().parents[1]
NPM = shutil.which('npm.cmd' if os.name == 'nt' else 'npm') or 'npm'
NODE = shutil.which('node.exe' if os.name == 'nt' else 'node') or 'node'
CHECK_SCRIPTS = [
    'test_mvp.py', 'test_commit_delivery.py', 'test_local_features.py', 'test_outputs.py',
    'test_v05_completion.py', 'test_review_interchange.py', 'test_hosted.py',
    'test_hosted_review.py', 'test_hosted_office.py', 'test_hosted_citations_outputs.py',
    'test_v061_authoring.py', 'test_everyday.py', 'test_query_organization.py',
    'test_hierarchical_tags.py', 'test_link_diagnostics.py', 'test_v061_processing.py',
    'test_v061_processing_integration.py', 'test_recognition.py', 'test_hosted_owner_backup.py',
    'test_hosted_content_import.py', 'test_hosted_query.py', 'test_hosted_providers.py',
    'test_hosted_scopes.py', 'test_hosted_library.py', 'test_hosted_media.py',
    'test_hosted_resource_collections.py', 'test_hosted_definition.py', 'test_hosted_history.py',
    'test_hosted_navigation_fragments.py', 'test_hosted_structure.py',
    'test_hosted_knowledge_links.py', 'test_hosted_navigation_registry.py',
    'test_sync.py', 'test_sync_compound.py', 'test_sync_revalidation.py',
    'test_sync_structure.py', 'test_sync_placement.py', 'test_sync_retirement.py',
    'test_sync_history.py', 'test_remote_sync.py',
]


def run(*args, capture=False, env=None, cwd=ROOT):
    return subprocess.run([str(value) for value in args], cwd=cwd, check=True,
                          capture_output=capture, text=True, encoding='utf-8', env=env).stdout


def metadata(cwd, env, full=False):
    args = ['cargo', 'metadata', '--locked', '--format-version', '1']
    if full:
        args.append('--offline')
    else:
        args.append('--no-deps')
    return json.loads(run(*args, capture=True, cwd=cwd, env=env))


def build(release, *, package_input=False, linker=None, windows_sdk=None):
    environment = dict(os.environ)
    initial = metadata(ROOT, environment)
    target = validate_target(ROOT, initial['target_directory'])
    environment['CARGO_TARGET_DIR'] = str(target)
    if windows_sdk is not None:
        environment['WEFTEXT_WINDOWS_SDK_BIN'] = str(Path(windows_sdk).absolute())
    if package_input:
        environment['CARGO_BUILD_TARGET'] = 'x86_64-pc-windows-msvc'
    frozen = snapshot(ROOT, [target])
    source_root = ROOT
    if package_input:
        # A before/after comparison of an edited checkout alone is not an atomic
        # source snapshot. Package builds use an owned, fixed public-source copy.
        source_root = target / 'weftext-package-inputs' / str(uuid.uuid4()) / 'source'
        source_root.mkdir(parents=True, exist_ok=False)
        for name, data in frozen.items():
            path = source_root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(data)
    contract = load_contract(frozen['packaging/windows/host-contract.json'])
    linker_version = None
    linker_bytes = None
    if linker is not None:
        from windows_trust import file_version
        linker = Path(linker).absolute()
        if linker.name.lower() != 'link.exe':
            raise ValueError('The packaging linker must be an explicit installed MSVC link.exe')
        linker_bytes = read_checked(linker, linker.parent)
        if pe_info(linker_bytes)['machine'] != 0x8664:
            raise ValueError('Use the actual x64 MSVC linker for the x64 package build')
        linker_version = file_version(linker)
        environment['CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER'] = str(linker)
    run(NPM, '--prefix', 'app', 'ci', '--ignore-scripts', cwd=source_root, env=environment)
    run(NPM, '--prefix', 'app', 'run', 'build', cwd=source_root, env=environment)
    frontend = snapshot(source_root / 'app/dist')
    local_metadata = metadata(source_root, environment)
    app_manifest = source_root / 'crates/weftext-app/Cargo.toml'
    package_id = next(package['id'] for package in local_metadata['packages']
                      if Path(package['manifest_path']) == app_manifest)
    args = ['cargo', 'build', '--locked', '-p', 'weftext-app', '--bins',
            '--message-format=json-render-diagnostics']
    if package_input:
        args.extend(['--target', 'x86_64-pc-windows-msvc'])
    if release:
        args.append('--release')
    paths, participants = {}, set()
    def compiled(message):
        collect_artifact(message, package_id, contract['roles'], paths)
        participants.add(message['package_id'])
    # Keep the product target lock through qualification and copying, so another
    # product invocation cannot replace mutable Cargo outputs before freezing.
    with CargoCache(source_root, target, environment) as cache:
        cache.run(args, on_message=compiled)
        if set(paths) != {'desktop', 'cli'}:
            raise ValueError('Cargo did not report both real Weftext desktop and CLI executables')
        binaries = {role: read_checked(path, target) for role, path in paths.items()}
        complete_metadata = metadata(source_root, environment, full=True)
        if validate_target(source_root, complete_metadata['target_directory']) != target:
            raise ValueError('Cargo target directory changed during the build')
        if snapshot(source_root, [target]) != frozen or snapshot(source_root / 'app/dist') != frontend:
            raise ValueError('Build inputs changed during compilation; originals retained, package refused')
        if linker is not None and read_checked(linker, linker.parent) != linker_bytes:
            raise ValueError('MSVC linker changed during the build')
        catalog_path = catalog = None
        if os.name == 'nt':
            catalog = artifact_catalog(frozen, binaries, contract)
            artifact_root = target / 'weftext-build-artifacts' / str(uuid.uuid4())
            artifact_root.mkdir(parents=True, exist_ok=False)
            paths = {}
            for role, policy in contract['roles'].items():
                path = artifact_root / policy['filename']
                with path.open('xb') as stream:
                    stream.write(binaries[role])
                paths[role] = path
            catalog_path = artifact_root / 'artifact-catalog.json'
            with catalog_path.open('xb') as stream:
                stream.write(catalog)
    result = BuildArtifacts(source_root, paths, binaries, frozen, frontend, complete_metadata,
                            frozenset(participants), target, 'release' if release else 'debug', linker_version,
                            {key: environment[key] for key in ('CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET',
                             'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER', 'WEFTEXT_WINDOWS_SDK_BIN') if key in environment},
                            catalog_path, catalog)
    for role, path in paths.items():
        print(f'Built {role}: {path} (SHA-256 {sha256(binaries[role])})', flush=True)
    if catalog_path is not None:
        print(f'Artifact catalog: {catalog_path}', flush=True)
    if package_input:
        print(f'Frozen package source: {source_root}')
    return result


def check(artifacts):
    root, exe = artifacts.source_root, artifacts.paths['cli']
    environment = {**os.environ, **artifacts.build_environment,
                   'WEFTEXT_TEST_EXECUTABLE': str(exe)}
    def checked(*args, **kwargs):
        return run(*args, cwd=root, env=environment, **kwargs)
    checked(sys.executable, 'scripts/check.py', '--models')
    check_root = artifacts.target_directory / 'weftext-checks' / str(uuid.uuid4())
    check_root.mkdir(parents=True, exist_ok=False)
    print(f'Product check root: {check_root}', flush=True)
    checked(sys.executable, 'scripts/test_cargo_cache.py', '--fixture-root', check_root / 'cargo-inputs')
    checked(sys.executable, 'scripts/test_windows_package.py')
    checked(sys.executable, 'scripts/test_cli_serve.py')
    checked(NODE, '--test', 'scripts/test_cli_serve.mjs')
    checked(sys.executable, 'scripts/recognition/test_schemes.py')
    checked(sys.executable, '-S', 'scripts/recognition/test_docx_reader.py')
    checked(sys.executable, 'resources/fonts/verify.py')
    checked(NPM, '--prefix', 'app', 'test')
    checked('cargo', 'fmt', '--all', '--', '--check')
    checked(sys.executable, 'scripts/test_cli_contract.py',
            '--gui-executable', artifacts.paths['desktop'], '--cli-executable', artifacts.paths['cli'],
            '--test-root', check_root / 'cli', '--output-dir', check_root / 'cli-results')
    with CargoCache(root, artifacts.target_directory, environment) as cache:
        cache.run(['cargo', 'clippy', '--locked', '--workspace', '--all-targets',
                   '--message-format=json-render-diagnostics', '--', '-D', 'warnings'])
        cache.run(['cargo', 'test', '--locked', '--workspace', '--message-format=json-render-diagnostics'])
    for script in CHECK_SCRIPTS:
        checked(sys.executable, 'scripts/' + script, exe)
    artifacts.verify_inputs(root)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['build', 'check', 'package'])
    parser.add_argument('--release', action='store_true', help='Use the optimized Rust profile')
    parser.add_argument('--windows-sdk', type=Path, help='x64 Windows SDK bin directory; package only')
    parser.add_argument('--offline-materials', type=Path, help='Directory containing exact repository-locked offline installers')
    parser.add_argument('--msvc-linker', type=Path, help='Installed x64 MSVC link.exe used explicitly by the package build')
    args = parser.parse_args(argv)
    tools = materials = contract = None
    if args.action == 'package':
        if os.name != 'nt' or not all((args.windows_sdk, args.offline_materials, args.msvc_linker)):
            parser.error('package requires Windows, --windows-sdk, --offline-materials and --msvc-linker')
        require_checkout(ROOT)
        contract = load_contract(read_checked(ROOT / 'packaging/windows/host-contract.json', ROOT))
        version = tomllib.loads(read_checked(ROOT / 'crates/weftext-app/Cargo.toml', ROOT).decode('utf-8'))['package']['version']
        msix_version(version)
        tools = sdk_tools(args.windows_sdk)
        materials = freeze_materials(ROOT, args.offline_materials, contract)
    elif any((args.windows_sdk, args.offline_materials, args.msvc_linker)):
        parser.error('Windows packaging inputs are accepted only for package')
    artifacts = build(args.release, package_input=args.action == 'package', linker=args.msvc_linker,
                      windows_sdk=args.windows_sdk)
    if args.action in {'check', 'package'}:
        check(artifacts)
    if args.action == 'package':
        source = artifacts.source_root
        if load_contract(artifacts.source['packaging/windows/host-contract.json']) != contract:
            raise ValueError('Host contract changed after package preflight')
        if sha256(artifacts.source['packaging/windows/offline-materials.json']) != materials['lockSha256']:
            raise ValueError('Offline material lock changed after package preflight')
        version = tomllib.loads(artifacts.source['crates/weftext-app/Cargo.toml'].decode('utf-8'))['package']['version']
        package_windows(source, artifacts, version, contract, materials, tools, output_root=ROOT)


if __name__ == '__main__':
    main()
