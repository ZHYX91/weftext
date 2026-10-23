"""Same-checkout frozen artifact identity, never a cache of check results."""
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tomllib
import uuid

from cargo_cache import digest_json, tool_identity
from package_inputs import (BuildArtifacts, inventory, is_reparse, parse_json,
                            read_checked, sha256, snapshot, validate_target)
from windows_package import artifact_catalog, load_contract

RECEIPT = 'build-input-receipt.json'
SCOPE = 'same-checkout-frozen-artifacts-not-reproducible-build-or-check-results'
GROUPS = ('source', 'scripts', 'frontend', 'cli', 'rust', 'http')
NPM_CONFIG_KEYS = ('userconfig', 'globalconfig', 'prefix', 'script-shell',
                   'ignore-scripts', 'omit', 'include', 'install-strategy',
                   'legacy-peer-deps', 'strict-peer-deps', 'engine-strict',
                   'bin-links', 'node-options', 'cpu', 'os', 'libc', 'force')
ENVIRONMENT_KEYS = (
    'RUSTUP_TOOLCHAIN', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTC', 'RUSTC_WRAPPER',
    'RUSTC_WORKSPACE_WRAPPER', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
    'RUSTDOCFLAGS', 'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_INCREMENTAL',
    'CARGO_BUILD_TARGET', 'CARGO_BUILD_JOBS', 'CARGO_BUILD_RUSTC',
    'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER',
    'CARGO_BUILD_RUSTFLAGS', 'CARGO_BUILD_RUSTDOCFLAGS',
    'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER',
    'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS',
    'WEFTEXT_WINDOWS_SDK_BIN', 'WindowsSdkVerBinPath', 'ProgramFiles(x86)',
    'LIB', 'LIBPATH', 'INCLUDE', 'CC', 'CXX', 'AR', 'CL', '_CL_', 'LINK', '_LINK_',
    'NODE_ENV', 'NODE_OPTIONS', 'NODE_PATH', 'WEFTEXT_API_PORT',
    *(f'CARGO_PROFILE_{profile}_{key}'
      for profile in ('DEV', 'RELEASE', 'TEST', 'BENCH')
      for key in ('OPT_LEVEL', 'DEBUG', 'SPLIT_DEBUGINFO', 'STRIP',
                  'DEBUG_ASSERTIONS', 'OVERFLOW_CHECKS', 'LTO', 'PANIC',
                  'INCREMENTAL', 'CODEGEN_UNITS', 'RPATH')),
)


def _same(left, right):
    # Python's True == 1 must not admit mistyped JSON facts.
    return digest_json(left) == digest_json(right)


def _closed(value, keys, label):
    if type(value) is not dict or set(value) != set(keys):
        raise ValueError(f'Invalid closed {label}')


def _file(path, root=None):
    path = Path(path).absolute()
    data = read_checked(path, path.parent if root is None else root)
    return {'bytes': len(data), 'sha256': sha256(data)}


def _output(command, root, environment):
    result = subprocess.run([str(arg) for arg in command], cwd=root, env=environment,
                            stdin=subprocess.DEVNULL, capture_output=True, check=True,
                            timeout=60)
    if len(result.stdout) > 1024 * 1024:
        raise ValueError('Tool identity output exceeds its budget')
    return result.stdout.decode('utf-8').strip()


def selected_node(environment):
    name = 'node.exe' if os.name == 'nt' else 'node'
    selected = shutil.which(name, path=environment.get('PATH'))
    if not selected:
        raise ValueError('Node is required for this selected check group')
    return Path(selected).resolve(strict=True)


@dataclass(frozen=True)
class NpmTool:
    node: Path
    cli: Path
    root: Path

    @classmethod
    def discover(cls, environment):
        node = selected_node(environment)
        name = 'npm.cmd' if os.name == 'nt' else 'npm'
        selected = shutil.which(name, path=environment.get('PATH'))
        if not selected:
            raise ValueError('A standard installed npm CLI is required')
        selected = Path(selected).absolute()
        if os.name == 'nt':
            # A standard adjacent installation only; never execute/interpret cmd.
            root = selected.parent / 'node_modules/npm'
            cli = root / 'bin/npm-cli.js'
        else:
            cli = selected.resolve(strict=True)
            root = cli.parent.parent
        manifest = parse_json(read_checked(root / 'package.json', root, max_bytes=1024 * 1024))
        if (type(manifest) is not dict or manifest.get('name') != 'npm'
                or type(manifest.get('version')) is not str
                or not re.fullmatch(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?', manifest['version'])
                or type(manifest.get('bin')) is not dict
                or manifest['bin'].get('npm') != 'bin/npm-cli.js'
                or cli != root / 'bin/npm-cli.js'):
            raise ValueError('Selected npm is not the standard declared real CLI')
        read_checked(cli, root)
        return cls(node, cli, root)

    def environment(self, environment):
        return {**environment, 'PATH': str(self.node.parent) + os.pathsep + environment.get('PATH', '')}

    def command(self, *args):
        return [str(self.node), str(self.cli), *map(str, args)]

    def identity(self, root, environment):
        # snapshot excludes node_modules; npm's bundled implementation must not.
        entries, total, folded = [], 0, set()
        if is_reparse(self.root.lstat()):
            raise ValueError('npm installation root is a reparse point')
        def traversal_failed(error):
            raise error
        for directory, children, names in os.walk(self.root, followlinks=False, onerror=traversal_failed):
            for name in children:
                if is_reparse((Path(directory) / name).lstat()):
                    raise ValueError('npm implementation contains a reparse directory')
            children.sort()
            for name in sorted(names):
                path = Path(directory) / name
                relative = path.relative_to(self.root).as_posix()
                if relative.casefold() in folded:
                    raise ValueError('npm implementation contains case-colliding names')
                folded.add(relative.casefold())
                data = read_checked(path, self.root, max_bytes=128 * 1024 * 1024 - total)
                total += len(data)
                entries.append({'path': relative, 'bytes': len(data), 'sha256': sha256(data)})
                if len(entries) > 20000:
                    raise ValueError('npm implementation exceeds its file budget')
        manifest = parse_json(read_checked(self.root / 'package.json', self.root))
        version = _output(self.command('--version'), root, environment)
        if version != manifest['version']:
            raise ValueError('Actual npm CLI version differs from its installed package')
        return {
            'node': {'path': str(self.node), 'version': _output([self.node, '--version'], root, environment),
                     **_file(self.node)},
            'npm': {'root': str(self.root), 'cli': {'path': 'bin/npm-cli.js', **_file(self.cli, self.root)},
                    'package': _file(self.root / 'package.json', self.root), 'version': version,
                    'implementation': {'files': len(entries), 'bytes': total,
                                       'sha256': digest_json(sorted(entries, key=lambda item: item['path']))}},
        }


def _config_file(path):
    path = Path(path).absolute()
    return {'path': str(path), 'identity': _file(path) if path.exists() else None}


def _absolute_setting(value, label):
    path = Path(value)
    if not path.is_absolute():
        raise ValueError(f'Frozen qualification requires an absolute {label}')
    return path


def _cargo_home(environment):
    return _absolute_setting(environment.get('CARGO_HOME', str(Path.home() / '.cargo')), 'CARGO_HOME')


def _direct_tool(value, environment, root=None):
    if '/' in value or '\\' in value:
        return _absolute_setting(value, 'tool path')
    # Relative PATH entries otherwise use the Python caller's directory for
    # observation and the Cargo working directory for execution.
    entries = environment.get('PATH', os.defpath).split(os.pathsep)
    for entry in entries:
        _absolute_setting(entry, 'tool search path entry')
    selected = shutil.which(value, path=os.pathsep.join(entries))
    if selected is None:
        raise ValueError('The explicitly selected tool is absent from PATH')
    path = _absolute_setting(selected, 'selected tool path')
    if path.parent not in {Path(entry) for entry in entries}:
        raise ValueError('Frozen qualification refuses implicit current-directory tool selection')
    if os.name == 'nt' and root is not None:
        # Cargo is invoked from root, which may differ from this Python caller.
        # Refuse a root shadow rather than observing one executable and running
        # another through Windows' implicit executable search.
        names = {value, *(value + suffix for suffix in
                         environment.get('PATHEXT', '.COM;.EXE;.BAT;.CMD').split(os.pathsep))}
        for name in names:
            shadow = Path(root) / name
            if shadow.exists() and not shadow.samefile(path):
                raise ValueError('Frozen qualification refuses source-root tool shadowing')
    return path


def _require_direct_cargo_tools(root, environment):
    """Qualify explicit tool overrides without implementing Cargo's config resolver.

    https://doc.rust-lang.org/cargo/reference/config.html#buildrustc
    Unknown include/conditional tool selection is refused, never treated as default.
    """
    root = Path(root).absolute()
    home = _cargo_home(environment)
    folders = {home, *(parent / '.cargo' for parent in (root, *root.parents))}
    overrides = {
        'rustc': ('RUSTC', 'CARGO_BUILD_RUSTC'),
        'rustc-wrapper': ('RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WRAPPER'),
        'rustc-workspace-wrapper': ('RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER'),
    }
    selection_keys = {key for pair in overrides.values() for key in pair}
    selection_keys.update(('PATH', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN',
                           'WEFTEXT_WINDOWS_SDK_BIN', 'WindowsSdkVerBinPath',
                           'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER'))
    for folder in sorted(folders):
        legacy, modern = folder / 'config', folder / 'config.toml'
        path = legacy if legacy.exists() else modern
        if not path.exists():
            continue
        config = tomllib.loads(read_checked(path, folder).decode('utf-8'))
        if 'include' in config:
            raise ValueError('Frozen qualification does not resolve Cargo config includes')
        if selection_keys.intersection(config.get('env', {})):
            raise ValueError('Use direct environment tool selections, not Cargo [env], for frozen qualification')
        build = config.get('build', {})
        for name, keys in overrides.items():
            if name in build and not any(key in environment for key in keys):
                if name == 'rustc' and build[name] == 'rustc':
                    continue
                raise ValueError(f'Qualify Cargo build.{name} through its explicit environment override')
        for target, policy in config.get('target', {}).items():
            if 'linker' not in policy:
                continue
            if target.startswith('cfg('):
                raise ValueError('Frozen qualification does not resolve conditional Cargo linker selection')
            if (target == 'x86_64-pc-windows-msvc'
                    and not environment.get('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER')):
                raise ValueError('Qualify the selected Cargo linker through its explicit environment override')


def declared_config(root, environment, npm):
    root = Path(root).absolute()
    _require_direct_cargo_tools(root, environment)
    values = _output(npm.command('--prefix', 'app', 'config', 'get', *NPM_CONFIG_KEYS), root, environment)
    npm_values = {}
    for line in values.splitlines():
        key, equal, value = line.partition('=')
        if not equal or key not in NPM_CONFIG_KEYS or key in npm_values:
            raise ValueError('npm returned an unsupported declared configuration shape')
        npm_values[key] = value
    if set(npm_values) != set(NPM_CONFIG_KEYS):
        raise ValueError('npm did not report the declared build configuration')
    cargo_home = _cargo_home(environment)
    paths = {parent / '.cargo' / name for parent in (root, *root.parents)
             for name in ('config', 'config.toml')}
    paths.update(cargo_home / name for name in ('config', 'config.toml'))
    files = [_config_file(path) for path in sorted(paths)]
    npm_paths = {npm.root / 'npmrc', root / '.npmrc', root / 'app/.npmrc',
                 Path(npm_values['userconfig']), Path(npm_values['globalconfig'])}
    files.extend(_config_file(path) for path in sorted(npm_paths))
    files.extend(_config_file(root / 'app' / name) for name in
                 ('.env', '.env.local', '.env.production', '.env.production.local'))
    explicit = {}
    for key in ('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER', 'RUSTC_WRAPPER',
                'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTC_WRAPPER',
                'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER'):
        value = environment.get(key)
        if value:
            path = _direct_tool(value, environment, root)
            explicit[key] = _file(path)
    sdk = environment.get('WEFTEXT_WINDOWS_SDK_BIN')
    sdk = sdk or environment.get('WindowsSdkVerBinPath')
    if sdk:
        directory = _absolute_setting(sdk, 'Windows SDK directory')
        rc = directory / 'rc.exe'
        if not rc.is_file() and not environment.get('WEFTEXT_WINDOWS_SDK_BIN'):
            rc = directory / 'x64/rc.exe'
        explicit['resourceCompiler'] = _file(rc)
    # Values are private comparison inputs, never emitted as env/PATH/config dumps.
    inputs = {'environment': {key: environment.get(key) for key in ENVIRONMENT_KEYS},
              'npmEffective': npm_values, 'files': files, 'explicitTools': explicit}
    return {'sha256': digest_json(inputs),
            'sdkObservation': 'explicit-resource-compiler' if sdk else 'implicit-unobserved'}


def qualify(root, environment, npm):
    _require_direct_cargo_tools(root, environment)
    if 'RUSTC' in environment:
        compiler = environment['RUSTC']
    else:
        compiler = environment.get('CARGO_BUILD_RUSTC', 'rustc')
    if not compiler:
        raise ValueError('The explicitly selected Rust compiler must not be empty')
    compiler = str(_direct_tool(compiler, environment, root))
    cargo = str(_direct_tool('cargo', environment, root))
    return {'tools': {'cargo': tool_identity(cargo, root, environment),
                      'rustc': tool_identity(compiler, root, environment),
                      **npm.identity(root, environment)},
            'declaredConfig': declared_config(root, environment, npm)}


@dataclass(frozen=True)
class QualifiedArtifacts(BuildArtifacts):
    receipt_path: Path | None = None
    qualification: dict | None = None


def _batch(path, target):
    path = Path(path).absolute()
    if path.name != RECEIPT or path.parent.parent != target / 'weftext-build-artifacts':
        raise ValueError('Receipt must name the standard file in its qualified build batch')
    try:
        batch = uuid.UUID(path.parent.name)
    except ValueError as error:
        raise ValueError('Build batch is not a canonical UUID') from error
    if batch.version != 4 or str(batch) != path.parent.name:
        raise ValueError('Build batch must be a canonical UUID4')
    # read_checked subsequently rejects every reparse ancestor inside target.
    return path


def _facts(artifacts):
    path = _batch(artifacts.receipt_path, artifacts.target_directory)
    contract = load_contract(artifacts.source['packaging/windows/host-contract.json'])
    if set(artifacts.paths) != {'desktop', 'cli'} or set(artifacts.binaries) != {'desktop', 'cli'}:
        raise ValueError('Receipt requires both complete executable roles')
    roles = {}
    for role, policy in contract['roles'].items():
        expected = path.parent / policy['filename']
        if artifacts.paths[role] != expected:
            raise ValueError('Executable role must remain in the same immutable batch')
        roles[role] = {'filename': policy['filename'], 'bytes': len(artifacts.binaries[role]),
                       'sha256': sha256(artifacts.binaries[role])}
    if artifacts.catalog_path != path.parent / 'artifact-catalog.json' or artifacts.catalog is None:
        raise ValueError('Receipt requires its same-batch native catalog')
    ids = sorted(artifacts.built_package_ids)
    packages = {item['id'] for item in artifacts.metadata['packages']}
    if not ids or any(type(item) is not str or not item or len(item) > 4096 for item in ids) or not set(ids) <= packages:
        raise ValueError('Built package IDs must be actual Cargo metadata participants')
    return {
        'schemaVersion': 1, 'scope': SCOPE, 'sourceRoot': str(artifacts.source_root.resolve(strict=True)),
        'targetRoot': str(artifacts.target_directory), 'batchId': path.parent.name,
        'profile': artifacts.profile, 'target': 'x86_64-pc-windows-msvc', 'features': [],
        'source': inventory(artifacts.source), 'frontend': inventory(artifacts.frontend),
        'roles': roles, 'catalog': {'filename': 'artifact-catalog.json',
                                  'bytes': len(artifacts.catalog), 'sha256': sha256(artifacts.catalog)},
        'metadataSha256': digest_json(artifacts.metadata), 'builtPackageIds': ids,
        'qualification': artifacts.qualification,
    }


def publish_receipt(artifacts):
    """Called only while creating a new build batch; no sign/upgrade-old-batch CLI."""
    artifacts.verify_inputs(artifacts.source_root)
    data = json.dumps(_facts(artifacts), indent=2, ensure_ascii=True).encode() + b'\n'
    path = artifacts.receipt_path
    staging = path.with_name(RECEIPT + '.writing')
    # A complete closed file is published atomically with create-new semantics.
    # No replace of an old receipt; interrupted staging remains in the owned batch.
    with staging.open('xb') as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    os.link(staging, path, follow_symlinks=False)
    staging.unlink()


def verify_qualified(artifacts, environment, npm):
    artifacts.verify_inputs(artifacts.source_root)
    actual = parse_json(read_checked(artifacts.receipt_path, artifacts.target_directory,
                                     max_bytes=32 * 1024 * 1024))
    if not _same(actual, _facts(artifacts)):
        raise ValueError('Frozen build receipt changed; rebuild the current inputs')
    if artifacts.receipt_path.with_name(RECEIPT + '.writing').exists():
        raise ValueError('Build receipt publication is incomplete')
    if not _same(qualify(artifacts.source_root, environment, npm), artifacts.qualification):
        raise ValueError('Top-level tools or declared build configuration changed; rebuild')


def load_receipt(path, root, metadata, profile, environment, npm):
    root = Path(root).absolute()
    target = validate_target(root, metadata['target_directory'])
    if not Path(path).is_absolute():
        raise ValueError('Supply an absolute build-input receipt path')
    path = _batch(path, target)
    if path.with_name(RECEIPT + '.writing').exists():
        raise ValueError('Build receipt publication is incomplete')
    value = parse_json(read_checked(path, target, max_bytes=32 * 1024 * 1024))
    _closed(value, ('schemaVersion', 'scope', 'sourceRoot', 'targetRoot', 'batchId',
                    'profile', 'target', 'features', 'source', 'frontend', 'roles',
                    'catalog', 'metadataSha256', 'builtPackageIds', 'qualification'), 'build receipt')
    if (type(value['schemaVersion']) is not int or value['schemaVersion'] != 1
            or value['scope'] != SCOPE or value['sourceRoot'] != str(root.resolve(strict=True))
            or value['targetRoot'] != str(target) or value['batchId'] != path.parent.name
            or value['profile'] != profile or value['target'] != 'x86_64-pc-windows-msvc'
            or not _same(value['features'], [])):
        raise ValueError('Receipt version, checkout, target, batch or profile does not match')
    _closed(value['roles'], ('desktop', 'cli'), 'receipt roles')
    _closed(value['qualification'], ('tools', 'declaredConfig'), 'receipt qualification')
    ids = value['builtPackageIds']
    if (type(ids) is not list or any(type(item) is not str or not item or len(item) > 4096 for item in ids)
            or ids != sorted(set(ids)) or not ids):
        raise ValueError('Invalid Cargo participants in receipt')
    source, frontend = snapshot(root, [target]), snapshot(root / 'app/dist')
    contract = load_contract(source['packaging/windows/host-contract.json'])
    paths = {role: path.parent / policy['filename'] for role, policy in contract['roles'].items()}
    binaries = {role: read_checked(executable, target) for role, executable in paths.items()}
    catalog_path = path.parent / 'artifact-catalog.json'
    catalog = read_checked(catalog_path, target)
    if artifact_catalog(source, binaries, contract) != catalog:
        raise ValueError('Native catalog does not match current source and executable roles')
    qualification = qualify(root, environment, npm)
    artifacts = QualifiedArtifacts(root, paths, binaries, source, frontend, metadata,
                                   frozenset(ids), target, profile,
                                   build_environment={'CARGO_TARGET_DIR': str(target)},
                                   catalog_path=catalog_path, catalog=catalog,
                                   receipt_path=path, qualification=qualification)
    if not _same(value, _facts(artifacts)):
        raise ValueError('Source, frontend, roles, metadata, tools or declared configuration changed; rebuild')
    artifacts.verify_inputs(root)
    return artifacts
