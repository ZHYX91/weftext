"""Build/check the product or generate an unsigned, qualified Windows candidate."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib
import uuid

from cargo_cache import CargoCache, digest_json
from build_receipt import (GROUPS, NpmTool, QualifiedArtifacts, RECEIPT, load_receipt,
                           publish_receipt, qualify, selected_node, verify_qualified)
from package_inputs import BuildArtifacts, collect_artifact, read_checked, require_checkout, sha256, snapshot, validate_target
from windows_package import artifact_catalog, load_contract, msix_version, package_windows, pe_info, sdk_tools
from windows_materials import freeze_materials

ROOT = Path(__file__).resolve().parents[1]
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
    npm = NpmTool.discover(environment)
    environment = npm.environment(environment)
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
    qualification = qualify(source_root, environment, npm) if os.name == 'nt' else None
    run(*npm.command('--prefix', 'app', 'ci', '--ignore-scripts'), cwd=source_root, env=environment)
    run(*npm.command('--prefix', 'app', 'run', 'build'), cwd=source_root, env=environment)
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
    if qualification is not None and digest_json(qualify(source_root, environment, npm)) != digest_json(qualification):
        raise ValueError('Build tools or declared configuration changed during compilation')
    artifact_type = QualifiedArtifacts if catalog_path is not None else BuildArtifacts
    qualified = {} if catalog_path is None else {
        'receipt_path': catalog_path.parent / RECEIPT, 'qualification': qualification}
    result = artifact_type(source_root, paths, binaries, frozen, frontend, complete_metadata,
                            frozenset(participants), target, 'release' if release else 'debug', linker_version,
                            {key: environment[key] for key in ('CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET',
                             'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER', 'WEFTEXT_WINDOWS_SDK_BIN') if key in environment},
                            catalog_path, catalog, **qualified)
    if isinstance(result, QualifiedArtifacts):
        publish_receipt(result)
        print(f'Build input receipt: {result.receipt_path}', flush=True)
    for role, path in paths.items():
        print(f'Built {role}: {path} (SHA-256 {sha256(binaries[role])})', flush=True)
    if catalog_path is not None:
        print(f'Artifact catalog: {catalog_path}', flush=True)
    if package_input:
        print(f'Frozen package source: {source_root}')
    return result


def selected_groups(values=None):
    if values is None:
        return GROUPS
    if not values or any(value not in GROUPS for value in values):
        raise ValueError('Select at least one known check group')
    return tuple(group for group in GROUPS if group in values)


def check(artifacts=None, groups=None):
    groups = selected_groups(groups)
    root = ROOT if artifacts is None else artifacts.source_root
    environment = dict(os.environ)
    if artifacts is not None:
        environment.update(artifacts.build_environment)
        environment['WEFTEXT_TEST_EXECUTABLE'] = str(artifacts.paths['cli'])
    npm = None
    if 'frontend' in groups or isinstance(artifacts, QualifiedArtifacts):
        npm = NpmTool.discover(environment)
        environment = npm.environment(environment)
    node = None
    if 'scripts' in groups:
        node = npm.node if npm is not None else selected_node(environment)
        environment['PATH'] = str(node.parent) + os.pathsep + environment.get('PATH', '')
    if artifacts is None and any(group in groups for group in ('cli', 'rust', 'http')):
        raise ValueError('CLI, Rust and HTTP groups require qualified product artifacts')

    def checked(*args, **kwargs):
        return run(*args, cwd=root, env=environment, **kwargs)

    check_root = None
    if any(group in groups for group in ('scripts', 'cli')):
        target = (artifacts.target_directory if artifacts is not None else
                  validate_target(root, metadata(root, environment)['target_directory']))
        check_root = target / 'weftext-checks' / str(uuid.uuid4())
        check_root.mkdir(parents=True, exist_ok=False)
        print(f'Product check root: {check_root}', flush=True)

    def source():
        checked(sys.executable, 'scripts/check.py', '--models')

    def scripts():
        checked(sys.executable, 'scripts/test_cargo_cache.py', '--fixture-root', check_root / 'cargo-inputs')
        checked(sys.executable, 'scripts/test_windows_package.py')
        checked(sys.executable, 'scripts/test_build_receipt.py')
        checked(sys.executable, 'scripts/test_mvp_entry.py')
        checked(sys.executable, 'scripts/test_cli_contract_support.py')
        checked(sys.executable, 'scripts/test_cli_serve.py')
        checked(sys.executable, 'scripts/test_owned_worker_observer.py')
        checked(node, '--test', 'scripts/test_cli_serve.mjs')
        checked(sys.executable, 'scripts/recognition/test_schemes.py')
        checked(sys.executable, '-S', 'scripts/recognition/test_docx_reader.py')
        checked(sys.executable, 'resources/fonts/verify.py')

    def frontend():
        if artifacts is None:
            checked(*npm.command('--prefix', 'app', 'ci', '--ignore-scripts'))
        checked(*npm.command('--prefix', 'app', 'test'))
        if artifacts is None:
            checked(*npm.command('--prefix', 'app', 'run', 'build'))

    def formatting():
        checked('cargo', 'fmt', '--all', '--', '--check')

    def cli():
        checked(sys.executable, 'scripts/test_cli_contract.py',
                '--gui-executable', artifacts.paths['desktop'], '--cli-executable', artifacts.paths['cli'],
                '--test-root', check_root / 'cli', '--output-dir', check_root / 'cli-results')

    def rust():
        with CargoCache(root, artifacts.target_directory, environment) as cache:
            cache.run(['cargo', 'clippy', '--locked', '--workspace', '--all-targets',
                       '--message-format=json-render-diagnostics', '--', '-D', 'warnings'])
            cache.run(['cargo', 'test', '--locked', '--workspace', '--message-format=json-render-diagnostics'])

    def http():
        for script in CHECK_SCRIPTS:
            checked(sys.executable, 'scripts/' + script, artifacts.paths['cli'])

    def verify():
        if isinstance(artifacts, QualifiedArtifacts):
            verify_qualified(artifacts, environment, npm)
        elif artifacts is not None:
            artifacts.verify_inputs(root)

    # One step list owns both full and partial execution. Rust formatting retains
    # its original position before CLI; Clippy/tests retain their position after.
    steps = (('source', source), ('scripts', scripts), ('frontend', frontend),
             ('rust', formatting), ('cli', cli), ('rust', rust), ('http', http))
    for group, step in steps:
        if group not in groups:
            continue
        verify()
        try:
            step()
        finally:
            primary = sys.exception()
            try:
                verify()
            except Exception as error:
                if primary is None:
                    raise
                primary.add_note(f'Post-group frozen input verification also failed: {type(error).__name__}')
    verify()
    if groups == GROUPS:
        print('Full product checks passed on the current frozen artifacts.', flush=True)
    else:
        print('Partial checks passed (partial; no whole-product acceptance): ' + ', '.join(groups), flush=True)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['build', 'check', 'package'])
    parser.add_argument('--release', action='store_true', help='Use the optimized Rust profile')
    parser.add_argument('--group', action='append', choices=GROUPS, help='Run selected check groups; partial unless all groups run')
    parser.add_argument('--artifacts', type=Path, help='Absolute receipt for same-checkout frozen artifacts; check only, never reuses PASS')
    parser.add_argument('--windows-sdk', type=Path, help='x64 Windows SDK bin directory; package only')
    parser.add_argument('--offline-materials', type=Path, help='Directory containing exact repository-locked offline installers')
    parser.add_argument('--msvc-linker', type=Path, help='Installed x64 MSVC link.exe used explicitly by the package build')
    args = parser.parse_args(argv)
    if args.action != 'check' and (args.group is not None or args.artifacts is not None):
        parser.error('--group and --artifacts are accepted only for check; package always builds and checks fully')
    groups = selected_groups(args.group)
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
    if args.artifacts is not None:
        require_checkout(ROOT)
        environment = dict(os.environ)
        npm = NpmTool.discover(environment)
        environment = npm.environment(environment)
        initial = metadata(ROOT, environment)
        environment['CARGO_TARGET_DIR'] = str(validate_target(ROOT, initial['target_directory']))
        complete = metadata(ROOT, environment, full=True)
        artifacts = load_receipt(args.artifacts, ROOT, complete,
                                 'release' if args.release else 'debug', environment, npm)
    elif args.action == 'check' and not any(group in groups for group in ('cli', 'rust', 'http')):
        artifacts = None
    else:
        artifacts = build(args.release, package_input=args.action == 'package', linker=args.msvc_linker,
                          windows_sdk=args.windows_sdk)
    if args.action in {'check', 'package'}:
        check(artifacts, groups)
    if args.action == 'package':
        source = artifacts.source_root
        if load_contract(artifacts.source['packaging/windows/host-contract.json']) != contract:
            raise ValueError('Host contract changed after package preflight')
        if sha256(artifacts.source['packaging/windows/offline-materials.json']) != materials['lockSha256']:
            raise ValueError('Offline material lock changed after package preflight')
        version = tomllib.loads(artifacts.source['crates/weftext-app/Cargo.toml'].decode('utf-8'))['package']['version']
        package_windows(source, artifacts, version, contract, materials, tools, output_root=ROOT)
    return artifacts


if __name__ == '__main__':
    main()
