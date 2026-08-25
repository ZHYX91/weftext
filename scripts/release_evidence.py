#!/usr/bin/env python3
"""Fail-closed source policy and release-evidence gate for Weftext.

The source-policy command is network-free.  The verify-release command consumes
already-built artifacts and externally generated evidence; it never creates a
signature or an SBOM.  Artifact signatures are accepted only after cosign has
verified a certificate-bearing Sigstore bundle against an exact identity and
issuer derived from the checked-in release trust policy.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from typing import Any, Iterable, NoReturn


RELEASE_INPUT_SCHEMA = "weftext.release-input.v1"
RELEASE_EVIDENCE_SCHEMA = "weftext.release-evidence.v1"
SOURCE_MANIFEST_SCHEMA = "weftext.release-source-manifest.v1"
LICENSE_REPORT_SCHEMA = "weftext.dependency-licenses.v1"
TEST_EVIDENCE_SCHEMA = "weftext.test-evidence.v1"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
EXACT_VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
PLACEHOLDER_RE = re.compile(
    r"(?im)(?:\bTODO\b|\bTBD\b|\bFIXME\b|\bREPLACE(?:_ME)?\b|<[^>\r\n]+>)"
)
REQUIRED_ARTIFACT_COMPONENTS = frozenset(
    {"cli", "desktop", "server", "server-container", "webui"}
)
REQUIRED_SUPPLY_CHAIN_SCOPES = frozenset(
    {
        "cargo",
        "desktop-npm",
        "docling-worker-cargo",
        "server-container",
        "server-webui-npm",
        "webui-npm",
    }
)
SBOM_GENERATORS = {
    "cargo": "cargo-cyclonedx",
    "desktop-npm": "@cyclonedx/cyclonedx-npm",
    "docling-worker-cargo": "cargo-cyclonedx",
    "server-container": "syft",
    "server-webui-npm": "@cyclonedx/cyclonedx-npm",
    "webui-npm": "@cyclonedx/cyclonedx-npm",
}
REQUIRED_TEST_LAYERS = frozenset(
    {
        "backup-disaster-recovery",
        "cli",
        "collaboration-multi-client",
        "core-source",
        "desktop-packaged",
        "filesystem-integration",
        "manual-accessibility-daily-use",
        "server-container-packaged",
        "webui-supported-browser",
    }
)
PACKAGE_JSONS = (
    Path("apps/desktop/package.json"),
    Path("prototypes/webui/package.json"),
    Path("crates/weftext-server/webui/package.json"),
)
PACKAGE_LOCKS = tuple(path.with_name("package-lock.json") for path in PACKAGE_JSONS)
DOCLING_WORKER_ROOT = Path("workers/weftext-docling-lite")
DOCLING_WORKER_BUILD_EVIDENCE = (
    DOCLING_WORKER_ROOT / "release-evidence/x86_64-pc-windows-msvc.json"
)
DOCLING_WORKER_POLICY_FILES = (
    DOCLING_WORKER_ROOT / "Cargo.toml",
    DOCLING_WORKER_ROOT / "Cargo.lock",
    DOCLING_WORKER_ROOT / "rust-toolchain.toml",
    DOCLING_WORKER_ROOT / "src/lib.rs",
    DOCLING_WORKER_ROOT / "src/main.rs",
    DOCLING_WORKER_ROOT / "release-profile.json",
    DOCLING_WORKER_ROOT / "THIRD_PARTY_NOTICES.md",
    DOCLING_WORKER_ROOT / "scripts/build-pinned-release.ps1",
    DOCLING_WORKER_ROOT / "scripts/smoke-worker.ps1",
    DOCLING_WORKER_ROOT / "scripts/write-build-evidence.ps1",
    DOCLING_WORKER_BUILD_EVIDENCE,
)
SOURCE_DIGEST_FILES = (
    Path("Cargo.toml"),
    Path("Cargo.lock"),
    Path("rust-toolchain.toml"),
    *PACKAGE_JSONS,
    *PACKAGE_LOCKS,
    Path("apps/desktop/src-tauri/tauri.conf.json"),
    Path("crates/weftext-server/deploy/Dockerfile"),
    Path("crates/weftext-server/deploy/Dockerfile.dockerignore"),
    Path("crates/weftext-server/deploy/compose.same-host.yaml"),
    Path("crates/weftext-import/docling-lite-assets.lock.json"),
    *DOCLING_WORKER_POLICY_FILES,
    Path(".github/workflows/source-gate.yml"),
    Path(".github/workflows/release-evidence.yml"),
    Path("release/release-input.schema.json"),
    Path("release/release-evidence.schema.json"),
    Path("release/test-evidence.schema.json"),
    Path("release/docling-lite-release-profile.schema.json"),
    Path("release/docling-lite-build-evidence.schema.json"),
    Path("release/docling-lite-assets-lock.schema.json"),
    Path("release/trust-policy.json"),
    Path("scripts/release_evidence.py"),
)


class GateError(RuntimeError):
    """A release assertion could not be established."""


def fail(message: str) -> NoReturn:
    raise GateError(message)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"cannot read JSON {path}: {error}")


def read_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as stream:
            value = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot read TOML {path}: {error}")
    if not isinstance(value, dict):
        fail(f"TOML root must be a table: {path}")
    return value


def require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(f"{label} must be an object")
    return value


def require_array(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        fail(f"{label} must be an array")
    return value


def require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{label} must be a non-empty string")
    return value


def require_positive_integer(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        fail(f"{label} must be a positive integer")
    return value


def require_exact_keys(
    value: dict[str, Any], required: Iterable[str], optional: Iterable[str], label: str
) -> None:
    required_set = set(required)
    allowed = required_set | set(optional)
    missing = sorted(required_set - value.keys())
    unknown = sorted(value.keys() - allowed)
    if missing:
        fail(f"{label} is missing required fields: {', '.join(missing)}")
    if unknown:
        fail(f"{label} has unknown fields: {', '.join(unknown)}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as stream:
            while block := stream.read(1024 * 1024):
                digest.update(block)
    except OSError as error:
        fail(f"cannot hash {path}: {error}")
    return digest.hexdigest()


def canonical_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def formatted_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    ).encode("utf-8")


def write_json(path: Path, value: Any) -> None:
    path.write_bytes(formatted_json_bytes(value))


def run_checked(
    arguments: list[str], cwd: Path, *, description: str, env: dict[str, str] | None = None
) -> str:
    try:
        process = subprocess.run(
            arguments,
            cwd=cwd,
            env=env,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except OSError as error:
        fail(f"cannot run {description}: {error}")
    if process.returncode != 0:
        detail = (process.stderr or process.stdout).strip()
        fail(f"{description} failed ({process.returncode}): {detail}")
    return process.stdout


def normalize_rust_version(value: str) -> str:
    parts = value.split(".")
    if len(parts) == 2 and all(part.isdigit() for part in parts):
        return f"{value}.0"
    if EXACT_VERSION_RE.fullmatch(value):
        return value
    fail(f"Rust version is not exact: {value!r}")


def validate_direct_npm_spec(specification: Any, label: str) -> None:
    spec = require_string(specification, label)
    if not EXACT_VERSION_RE.fullmatch(spec):
        fail(f"{label} must use an exact version, found {spec!r}")


def validate_npm_lock(package_json_path: Path, lock_path: Path, release_version: str) -> None:
    package = require_object(read_json(package_json_path), str(package_json_path))
    lock = require_object(read_json(lock_path), str(lock_path))
    if package.get("version") != release_version:
        fail(f"{package_json_path} version does not match {release_version}")
    if lock.get("lockfileVersion") != 3:
        fail(f"{lock_path} must use lockfileVersion 3")
    packages = require_object(lock.get("packages"), f"{lock_path} packages")
    root = require_object(packages.get(""), f"{lock_path} root package")
    if root.get("version") != release_version or root.get("name") != package.get("name"):
        fail(f"{lock_path} root package does not match package.json")
    for section in (
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ):
        declared = package.get(section, {})
        if not isinstance(declared, dict):
            fail(f"{package_json_path} {section} must be an object")
        locked_declared = root.get(section, {})
        if declared != locked_declared:
            fail(f"{lock_path} root {section} is stale")
        for name, spec in declared.items():
            validate_direct_npm_spec(spec, f"{package_json_path} {section}.{name}")
    for locator, raw_entry in packages.items():
        if not locator.startswith("node_modules/"):
            continue
        entry = require_object(raw_entry, f"{lock_path} {locator}")
        require_string(entry.get("version"), f"{lock_path} {locator}.version")
        require_string(entry.get("license"), f"{lock_path} {locator}.license")
        if entry.get("link") is True:
            continue
        resolved = require_string(entry.get("resolved"), f"{lock_path} {locator}.resolved")
        if not resolved.startswith("https://registry.npmjs.org/"):
            fail(f"{lock_path} {locator} uses an unapproved registry URL")
        integrity = require_string(entry.get("integrity"), f"{lock_path} {locator}.integrity")
        if not re.fullmatch(r"sha512-[A-Za-z0-9+/]+={0,2}", integrity):
            fail(f"{lock_path} {locator} has an invalid sha512 integrity")


def iter_dependency_tables(document: dict[str, Any]) -> Iterable[tuple[str, dict[str, Any]]]:
    dependency_keys = {
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "workspace.dependencies",
    }

    def visit(value: Any, path: tuple[str, ...]) -> Iterable[tuple[str, dict[str, Any]]]:
        if not isinstance(value, dict):
            return
        joined = ".".join(path)
        if path and (path[-1] in dependency_keys or joined.endswith(".dependencies")):
            yield joined, value
        for key, child in value.items():
            if isinstance(child, dict):
                yield from visit(child, (*path, str(key)))

    yield from visit(document, ())


def validate_cargo_manifests(
    repo: Path, workspace: dict[str, Any]
) -> tuple[str, list[tuple[str, Path]]]:
    package_defaults = require_object(
        workspace.get("workspace", {}).get("package"), "workspace.package"
    )
    release_version = require_string(package_defaults.get("version"), "workspace.package.version")
    if not EXACT_VERSION_RE.fullmatch(release_version):
        fail("workspace.package.version must be exact")
    members_raw = require_array(workspace.get("workspace", {}).get("members"), "workspace.members")
    members: list[tuple[str, Path]] = []
    manifests = [repo / "Cargo.toml"]
    for member_value in members_raw:
        member = Path(require_string(member_value, "workspace member"))
        if (
            member.is_absolute()
            or ".." in member.parts
            or any(char in str(member) for char in "*?[")
        ):
            fail(f"workspace member must be an explicit in-repository path: {member}")
        manifest_path = repo / member / "Cargo.toml"
        manifests.append(manifest_path)
        manifest = read_toml(manifest_path)
        package = require_object(manifest.get("package"), f"{manifest_path} package")
        name = require_string(package.get("name"), f"{manifest_path} package.name")
        version = package.get("version")
        if version != {"workspace": True} and version != release_version:
            fail(f"{manifest_path} does not inherit release version {release_version}")
        members.append((name, manifest_path))
    repo_real = repo.resolve()
    for manifest_path in manifests:
        document = read_toml(manifest_path)
        for table_name, dependencies in iter_dependency_tables(document):
            for dependency_name, dependency in dependencies.items():
                label = f"{manifest_path} {table_name}.{dependency_name}"
                if isinstance(dependency, str):
                    if dependency.strip() == "*":
                        fail(f"{label} uses a floating wildcard")
                    continue
                if not isinstance(dependency, dict):
                    fail(f"{label} has an invalid dependency declaration")
                if "git" in dependency:
                    revision = dependency.get("rev")
                    if not isinstance(revision, str) or not COMMIT_RE.fullmatch(revision):
                        fail(f"{label} git dependency must pin a full commit rev")
                    if "branch" in dependency or "tag" in dependency:
                        fail(f"{label} git dependency may not use branch or tag")
                if "path" in dependency:
                    path = manifest_path.parent / require_string(
                        dependency["path"], f"{label}.path"
                    )
                    try:
                        path.resolve().relative_to(repo_real)
                    except (OSError, ValueError):
                        fail(f"{label} path dependency escapes the repository")
                if dependency.get("version") == "*":
                    fail(f"{label} uses a floating wildcard")
    return release_version, members


def validate_cargo_lock(repo: Path, release_version: str, member_names: set[str]) -> None:
    lock_path = repo / "Cargo.lock"
    lock = read_toml(lock_path)
    if lock.get("version") != 4:
        fail("Cargo.lock must use lock format version 4")
    seen_members: set[str] = set()
    for raw_package in require_array(lock.get("package"), "Cargo.lock package"):
        package = require_object(raw_package, "Cargo.lock package entry")
        name = require_string(package.get("name"), "Cargo.lock package.name")
        version = require_string(package.get("version"), f"Cargo.lock {name}.version")
        source = package.get("source")
        if source is None:
            if name in member_names:
                if version != release_version:
                    fail(f"Cargo.lock workspace package {name} has stale version {version}")
                seen_members.add(name)
            continue
        source_string = require_string(source, f"Cargo.lock {name}.source")
        if source_string.startswith("registry+"):
            checksum = require_string(package.get("checksum"), f"Cargo.lock {name}.checksum")
            if not SHA256_RE.fullmatch(checksum):
                fail(f"Cargo.lock {name} has an invalid checksum")
        elif source_string.startswith("git+"):
            if not re.search(r"#[0-9a-f]{40}$", source_string):
                fail(f"Cargo.lock {name} git source is not commit-pinned")
        else:
            fail(f"Cargo.lock {name} uses unsupported source {source_string!r}")
    missing = sorted(member_names - seen_members)
    if missing:
        fail(f"Cargo.lock is missing workspace members: {', '.join(missing)}")


def exact_registry_dependency_version(dependency: Any, label: str) -> tuple[str, str | None]:
    """Return an exact registry version and optional renamed package for one direct dependency."""
    package_name = None
    if isinstance(dependency, str):
        specification = dependency
    elif isinstance(dependency, dict):
        if any(key in dependency for key in ("git", "path", "registry")):
            fail(f"{label} must use the pinned crates.io registry and standalone lockfile")
        specification = require_string(dependency.get("version"), f"{label}.version")
        raw_package_name = dependency.get("package")
        if raw_package_name is not None:
            package_name = require_string(raw_package_name, f"{label}.package")
    else:
        fail(f"{label} has an invalid dependency declaration")
    if not specification.startswith("=") or not EXACT_VERSION_RE.fullmatch(specification[1:]):
        fail(f"{label} must pin one exact registry version with =")
    return specification[1:], package_name


def validate_docling_worker_source_policy(repo: Path, release_version: str) -> str:
    """Validate the separately-tooled Docling Lite worker and its release pins."""
    root = repo / DOCLING_WORKER_ROOT
    manifest_path = root / "Cargo.toml"
    manifest = read_toml(manifest_path)
    package = require_object(manifest.get("package"), "Docling worker package")
    if package.get("name") != "weftext-docling-lite":
        fail("Docling worker package name is not exact")
    if package.get("version") != release_version:
        fail("Docling worker package version does not match the Weftext release")
    if package.get("edition") != "2021" or package.get("publish") is not False:
        fail("Docling worker must remain a private Rust 2021 package")
    declared_worker_rust = normalize_rust_version(
        require_string(package.get("rust-version"), "Docling worker rust-version")
    )
    workspace = require_object(manifest.get("workspace"), "Docling worker standalone workspace")
    if workspace.get("resolver") != "2" or "members" in workspace:
        fail("Docling worker must remain a standalone resolver-2 workspace")
    lints = require_object(manifest.get("lints"), "Docling worker lints")
    rust_lints = require_object(lints.get("rust"), "Docling worker Rust lints")
    if rust_lints.get("unsafe_code") != "forbid":
        fail("Docling worker must forbid unsafe Rust")

    direct_versions: dict[str, str] = {}

    def collect_direct_dependencies(raw_dependencies: Any, label: str) -> None:
        dependencies = require_object(raw_dependencies, label)
        for dependency_name, dependency in dependencies.items():
            version, renamed_package = exact_registry_dependency_version(
                dependency, f"{label}.{dependency_name}"
            )
            package_name = renamed_package or dependency_name
            previous = direct_versions.get(package_name)
            if previous is not None and previous != version:
                fail(
                    f"Docling worker declares conflicting exact versions for {package_name}"
                )
            direct_versions[package_name] = version

    dependency_tables = ("dependencies", "dev-dependencies", "build-dependencies")
    for table_name in dependency_tables:
        collect_direct_dependencies(
            manifest.get(table_name, {}), f"Docling worker {table_name}"
        )
    targets = require_object(manifest.get("target", {}), "Docling worker target")
    for target_name, raw_target in targets.items():
        target = require_object(raw_target, f"Docling worker target.{target_name}")
        for table_name in dependency_tables:
            collect_direct_dependencies(
                target.get(table_name, {}),
                f"Docling worker target.{target_name}.{table_name}",
            )

    required_versions = {
        "docling": "0.52.2",
        "docling-core": "0.52.2",
        "docling-pdf": "0.52.2",
        "ort": "2.0.0-rc.12",
        "ort-sys": "2.0.0-rc.12",
    }
    for name, expected in required_versions.items():
        if direct_versions.get(name) != expected:
            fail(f"Docling worker must pin {name} exactly to {expected}")
    docling_dependency = require_object(
        require_object(manifest.get("dependencies"), "Docling worker dependencies").get("docling"),
        "Docling worker dependencies.docling",
    )
    if docling_dependency.get("default-features") is not False or docling_dependency.get(
        "features"
    ) != ["pdf"]:
        fail("Docling worker must enable only docling's pdf feature with defaults disabled")
    for name in ("docling-core", "docling-pdf", "ort", "ort-sys"):
        dependency = require_object(
            manifest["dependencies"].get(name), f"Docling worker dependencies.{name}"
        )
        if dependency.get("default-features") is not False:
            fail(f"Docling worker must disable default features for {name}")

    toolchain = require_object(
        read_toml(root / "rust-toolchain.toml").get("toolchain"),
        "Docling worker rust-toolchain",
    )
    worker_rust = normalize_rust_version(
        require_string(toolchain.get("channel"), "Docling worker toolchain channel")
    )
    if worker_rust != declared_worker_rust or toolchain.get("profile") != "minimal":
        fail(
            "Docling worker manifest and pinned toolchain must name the same Rust version"
        )

    lock = read_toml(root / "Cargo.lock")
    if lock.get("version") != 4:
        fail("Docling worker Cargo.lock must use lock format version 4")
    locked_registry_versions: set[tuple[str, str]] = set()
    saw_worker = False
    for raw_package in require_array(lock.get("package"), "Docling worker Cargo.lock package"):
        locked = require_object(raw_package, "Docling worker Cargo.lock package entry")
        name = require_string(locked.get("name"), "Docling worker Cargo.lock package.name")
        version = require_string(
            locked.get("version"), f"Docling worker Cargo.lock {name}.version"
        )
        source = locked.get("source")
        if source is None:
            if name == "weftext-docling-lite" and version == release_version:
                saw_worker = True
            continue
        source_string = require_string(source, f"Docling worker Cargo.lock {name}.source")
        if source_string != "registry+https://github.com/rust-lang/crates.io-index":
            fail(f"Docling worker Cargo.lock {name} uses an unapproved source")
        checksum = require_string(
            locked.get("checksum"), f"Docling worker Cargo.lock {name}.checksum"
        )
        if not SHA256_RE.fullmatch(checksum):
            fail(f"Docling worker Cargo.lock {name} has an invalid checksum")
        locked_registry_versions.add((name, version))
    if not saw_worker:
        fail("Docling worker Cargo.lock is missing the exact worker package")
    for name, version in direct_versions.items():
        if (name, version) not in locked_registry_versions:
            fail(f"Docling worker Cargo.lock does not bind direct dependency {name} {version}")

    profile = require_object(read_json(root / "release-profile.json"), "Docling release profile")
    require_exact_keys(
        profile,
        {
            "schemaVersion",
            "workerProtocolVersion",
            "workerPackageVersion",
            "rustToolchainVersion",
            "buildToolchain",
            "source",
            "exactPrereleasePins",
            "exactDirectRegistryDependencies",
            "profile",
            "reviewedNativeRuntime",
            "runtimeAssetLayout",
            "reviewedExtractedArtifacts",
            "isolation",
        },
        set(),
        "Docling release profile",
    )
    if profile.get("schemaVersion") != "weftext.docling-lite-release-profile.v2":
        fail("Docling release profile schema is unsupported")
    if profile.get("workerProtocolVersion") != "weftext.docling-lite-worker-json.v1":
        fail("Docling worker protocol version is unsupported")
    if profile.get("workerPackageVersion") != release_version:
        fail("Docling release profile package version is stale")
    if normalize_rust_version(
        require_string(profile.get("rustToolchainVersion"), "Docling rustToolchainVersion")
    ) != worker_rust:
        fail("Docling release profile does not bind the exact worker Rust version")
    source = require_object(profile.get("source"), "Docling release profile source")
    require_exact_keys(
        source,
        {"crate", "version", "releaseTag", "releaseCommit", "defaultFeatures", "features"},
        set(),
        "Docling release profile source",
    )
    if (
        source.get("crate") != "docling"
        or source.get("version") != required_versions["docling"]
        or source.get("releaseTag") != "v0.52.2"
        or not isinstance(source.get("releaseCommit"), str)
        or not COMMIT_RE.fullmatch(source["releaseCommit"])
        or source.get("defaultFeatures") is not False
        or source.get("features") != ["pdf"]
    ):
        fail("Docling release profile source pin is not exact")
    asset_lock = require_object(
        read_json(repo / "crates/weftext-import/docling-lite-assets.lock.json"),
        "Docling Lite asset lock",
    )
    if (
        asset_lock.get("doclingReleaseTag") != source.get("releaseTag")
        or asset_lock.get("doclingReleaseCommit") != source.get("releaseCommit")
        or asset_lock.get("documentSchemaName") != "DoclingDocument"
        or asset_lock.get("documentSchemaVersion") != "1.10.0"
    ):
        fail("Docling worker source and packaged-asset lock do not share one release authority")
    if (
        asset_lock.get("target") != "x86_64-unknown-linux-gnu"
        or asset_lock.get("completeForExecution") is not False
        or not require_array(
            asset_lock.get("missingForExecution"), "Docling Lite asset lock missing evidence"
        )
    ):
        fail("the Linux audit lock must remain incomplete and may not authorize Windows")
    prerelease_pins = require_object(
        profile.get("exactPrereleasePins"), "Docling exactPrereleasePins"
    )
    for name in ("docling-core", "docling-pdf", "ort", "ort-sys"):
        if prerelease_pins.get(name) != required_versions[name]:
            fail(f"Docling release profile does not bind {name}")
    runtime_profile = require_object(profile.get("profile"), "Docling runtime profile")
    require_exact_keys(
        runtime_profile,
        {
            "input",
            "successOutput",
            "failureOutput",
            "ocr",
            "layout",
            "tableFormer",
            "networkFeatures",
            "excludedFeatures",
        },
        set(),
        "Docling runtime profile",
    )
    if (
        runtime_profile.get("input") != "input/source.pdf"
        or runtime_profile.get("successOutput") != "raw DoclingDocument 1.10.0 JSON"
        or runtime_profile.get("failureOutput") != "typed Weftext worker failure JSON"
        or runtime_profile.get("tableFormer") is not False
        or runtime_profile.get("networkFeatures") is not False
        or "directml-execution-provider"
        not in require_array(runtime_profile.get("excludedFeatures"), "Docling excludedFeatures")
    ):
        fail("Docling Lite profile must exclude TableFormer and network features")
    worker_source = (root / "src/lib.rs").read_text(encoding="utf-8")
    if (
        "WorkerOutput::Completed(document) => serde_json::to_vec(document)"
        not in worker_source
        or "WorkerOutput::Failed(failure)" not in worker_source
        or "serde_json::to_vec(&WorkerResponse::failed(request, *failure))"
        not in worker_source
        or "WorkerResponse::completed" in worker_source
    ):
        fail(
            "Docling worker source must emit raw DoclingDocument success and typed failure JSON"
        )
    native_runtime = require_object(
        profile.get("reviewedNativeRuntime"), "Docling reviewed native runtime"
    )
    require_exact_keys(
        native_runtime,
        {
            "target",
            "component",
            "implementation",
            "version",
            "rustBindingCrate",
            "rustBindingVersion",
            "linkage",
            "installPath",
            "sourceArchive",
            "archiveFiles",
            "buildEnvironment",
            "forbiddenImports",
        },
        set(),
        "Docling reviewed native runtime",
    )
    expected_native_archive = {
        "artifact": "onnxruntime-win-x64-1.24.2.zip",
        "sourceUrl": "https://github.com/microsoft/onnxruntime/releases/download/v1.24.2/onnxruntime-win-x64-1.24.2.zip",
        "byteLength": 74075355,
        "sha256": "8e3e9c826375352e29cb2614fe44f3d7a4b0ff7b8028ad7a456af9d949a7e8b0",
    }
    expected_native_files = {
        "runtime_library": (
            "onnxruntime-win-x64-1.24.2/lib/onnxruntime.dll",
            14148680,
            "114947d633e6844ce3c4b51ef6678f776628571d08a5763859c61642c8dcca9c",
        ),
        "import_library": (
            "onnxruntime-win-x64-1.24.2/lib/onnxruntime.lib",
            2124,
            "2ec547a0e0e655fd60d549d23a3155a3ec47217f92de32e84df51866175a51ff",
        ),
    }
    archive_files: dict[str, tuple[str, int, str]] = {}
    for index, raw_file in enumerate(
        require_array(native_runtime.get("archiveFiles"), "Docling native archive files")
    ):
        file_pin = require_object(raw_file, f"Docling native archive files[{index}]")
        require_exact_keys(
            file_pin,
            {"path", "role", "byteLength", "sha256"},
            set(),
            f"Docling native archive files[{index}]",
        )
        role = require_string(file_pin.get("role"), f"Docling archive file {index} role")
        if role in archive_files:
            fail("Docling native archive repeats a file role")
        archive_files[role] = (
            require_string(file_pin.get("path"), f"Docling archive file {index} path"),
            require_positive_integer(
                file_pin.get("byteLength"), f"Docling archive file {index} byteLength"
            ),
            require_string(file_pin.get("sha256"), f"Docling archive file {index} sha256"),
        )
    expected_build_environment = {
        "ORT_LIB_PATH": "<extracted-archive-root>/onnxruntime-win-x64-1.24.2/lib",
        "ORT_PREFER_DYNAMIC_LINK": "1",
        "ORT_SKIP_DOWNLOAD": "1",
        "CARGO_NET_OFFLINE": "true",
    }
    if (
        native_runtime.get("target") != "x86_64-pc-windows-msvc"
        or native_runtime.get("component") != "onnx-runtime"
        or native_runtime.get("implementation") != "Microsoft ONNX Runtime"
        or native_runtime.get("version") != "1.24.2"
        or native_runtime.get("rustBindingCrate") != "ort"
        or native_runtime.get("rustBindingVersion") != required_versions["ort"]
        or native_runtime.get("linkage") != "dynamic"
        or native_runtime.get("installPath") != "onnxruntime.dll"
        or require_object(native_runtime.get("sourceArchive"), "Docling native archive")
        != expected_native_archive
        or archive_files != expected_native_files
        or require_object(
            native_runtime.get("buildEnvironment"), "Docling native build environment"
        )
        != expected_build_environment
        or set(
            require_array(native_runtime.get("forbiddenImports"), "Docling forbidden imports")
        )
        != {"d3d12.dll", "directml.dll"}
    ):
        fail("Docling worker native runtime is not the reviewed Microsoft CPU archive")
    isolation = require_object(profile.get("isolation"), "Docling isolation profile")
    require_exact_keys(
        isolation,
        {"providedByWorker", "requiredFromSupervisor"},
        set(),
        "Docling isolation profile",
    )
    if isolation.get("providedByWorker") is not False:
        fail("Docling worker may not claim the supervisor's OS isolation")
    required_isolation = {
        "deny-by-default network isolation",
        "memory limit",
        "filesystem allowlist",
        "process-tree containment and cancellation",
    }
    actual_isolation = {
        require_string(value, "Docling isolation need")
        for value in require_array(
            isolation.get("requiredFromSupervisor"), "Docling isolation needs"
        )
    }
    if actual_isolation != required_isolation:
        fail("Docling release profile must declare every required supervisor isolation control")

    pinned_builder = (root / "scripts/build-pinned-release.ps1").read_text(encoding="utf-8")
    evidence_writer = (root / "scripts/write-build-evidence.ps1").read_text(encoding="utf-8")
    for token in (
        "ORT_LIB_PATH",
        "ORT_PREFER_DYNAMIC_LINK",
        "ORT_SKIP_DOWNLOAD",
        "CARGO_NET_OFFLINE",
        "--locked",
        "--offline",
        "x86_64-pc-windows-msvc",
        "onnxruntime-win-x64-1.24.2",
    ):
        if token not in pinned_builder:
            fail(f"Docling pinned builder omits the native build control {token}")
    for token in ("dumpbin", "workerImports", "runtimeImports", "forbiddenImportsAbsent"):
        if token not in evidence_writer:
            fail(f"Docling evidence writer omits native import attestation {token}")

    evidence_label = "Docling worker Windows build evidence"
    evidence = require_object(read_json(repo / DOCLING_WORKER_BUILD_EVIDENCE), evidence_label)
    require_exact_keys(
        evidence,
        {
            "schemaVersion",
            "target",
            "toolchain",
            "doclingSource",
            "fixedLiteProfile",
            "buildEnvironment",
            "worker",
            "workerSourceFiles",
            "nativeRuntime",
            "externalRuntimeImports",
            "directRegistryDependencies",
            "cargoManifestSha256",
            "cargoLockSha256",
            "releaseProfileSha256",
            "runtimeAssets",
            "physicalPackageBytes",
            "packageComplete",
            "missingForPackage",
            "osSandboxEvidence",
            "sandboxComplete",
            "completeForExecution",
            "missingForExecution",
        },
        set(),
        evidence_label,
    )
    build_toolchain = require_object(profile.get("buildToolchain"), "Docling buildToolchain")
    require_exact_keys(
        build_toolchain,
        {"rustc", "cargo", "reviewedTarget"},
        set(),
        "Docling buildToolchain",
    )
    if (
        evidence.get("schemaVersion") != "weftext.docling-lite-build-evidence.v2"
        or evidence.get("target") != build_toolchain.get("reviewedTarget")
        or require_object(evidence.get("toolchain"), f"{evidence_label}.toolchain")
        != {
            "rustc": build_toolchain.get("rustc"),
            "cargo": build_toolchain.get("cargo"),
        }
        or require_object(evidence.get("doclingSource"), f"{evidence_label}.doclingSource")
        != source
        or require_object(
            evidence.get("fixedLiteProfile"), f"{evidence_label}.fixedLiteProfile"
        )
        != {
            "formats": ["pdf"],
            "ocrLanguage": "en",
            "layoutPrecision": "int8",
            "tableFormer": False,
            "networkFeatures": False,
        }
    ):
        fail("Docling worker build evidence differs from the reviewed source/profile/toolchain")
    expected_evidence_build_environment = {
        "cargoCommand": f"cargo +{worker_rust} build --release --locked --offline "
        "--target x86_64-pc-windows-msvc",
        **expected_build_environment,
    }
    if require_object(
        evidence.get("buildEnvironment"), f"{evidence_label}.buildEnvironment"
    ) != expected_evidence_build_environment:
        fail("Docling worker build evidence does not bind the offline native override")
    if not require_string(
        require_object(evidence["toolchain"], f"{evidence_label}.toolchain").get("rustc"),
        f"{evidence_label}.toolchain.rustc",
    ).startswith(f"rustc {worker_rust} "):
        fail("Docling worker build evidence does not use the exact worker Rust toolchain")

    worker = require_object(evidence.get("worker"), f"{evidence_label}.worker")
    require_exact_keys(
        worker,
        {"component", "installPath", "fileName", "byteLength", "sha256"},
        set(),
        f"{evidence_label}.worker",
    )
    worker_path = require_string(worker.get("installPath"), f"{evidence_label}.worker.installPath")
    worker_bytes = require_positive_integer(
        worker.get("byteLength"), f"{evidence_label}.worker.byteLength"
    )
    worker_digest = require_string(worker.get("sha256"), f"{evidence_label}.worker.sha256")
    if (
        worker.get("component") != "docling-rs"
        or worker.get("fileName") != worker_path
        or "/" in worker_path
        or "\\" in worker_path
        or not SHA256_RE.fullmatch(worker_digest)
    ):
        fail("Docling worker build evidence has an invalid binary binding")

    source_bindings: dict[str, tuple[int, str]] = {}
    for index, raw_source in enumerate(
        require_array(evidence.get("workerSourceFiles"), f"{evidence_label}.workerSourceFiles")
    ):
        source_binding = require_object(
            raw_source, f"{evidence_label}.workerSourceFiles[{index}]"
        )
        require_exact_keys(
            source_binding,
            {"path", "byteLength", "sha256"},
            set(),
            f"{evidence_label}.workerSourceFiles[{index}]",
        )
        source_path = require_string(
            source_binding.get("path"), f"{evidence_label}.workerSourceFiles[{index}].path"
        )
        if source_path in source_bindings:
            fail("Docling worker build evidence repeats a source file")
        source_bindings[source_path] = (
            require_positive_integer(
                source_binding.get("byteLength"),
                f"{evidence_label}.workerSourceFiles[{index}].byteLength",
            ),
            require_string(
                source_binding.get("sha256"),
                f"{evidence_label}.workerSourceFiles[{index}].sha256",
            ),
        )
    expected_source_bindings = {
        relative: (
            (root / relative).stat().st_size,
            sha256_file(root / relative),
        )
        for relative in ("src/lib.rs", "src/main.rs")
    }
    if source_bindings != expected_source_bindings:
        fail("Docling worker build evidence does not bind the exact worker source")

    evidence_dependencies: dict[str, str] = {}
    for index, raw_dependency in enumerate(
        require_array(
            evidence.get("directRegistryDependencies"),
            f"{evidence_label}.directRegistryDependencies",
        )
    ):
        dependency = require_object(
            raw_dependency, f"{evidence_label}.directRegistryDependencies[{index}]"
        )
        require_exact_keys(
            dependency,
            {"name", "version"},
            set(),
            f"{evidence_label}.directRegistryDependencies[{index}]",
        )
        name = require_string(
            dependency.get("name"),
            f"{evidence_label}.directRegistryDependencies[{index}].name",
        )
        version = require_string(
            dependency.get("version"),
            f"{evidence_label}.directRegistryDependencies[{index}].version",
        )
        if name in evidence_dependencies:
            fail("Docling worker build evidence repeats a direct dependency")
        evidence_dependencies[name] = version
    profile_dependencies = require_object(
        profile.get("exactDirectRegistryDependencies"),
        "Docling exactDirectRegistryDependencies",
    )
    if evidence_dependencies != direct_versions or evidence_dependencies != profile_dependencies:
        fail("Docling worker build evidence does not bind every exact direct dependency")

    for field, relative in (
        ("cargoManifestSha256", DOCLING_WORKER_ROOT / "Cargo.toml"),
        ("cargoLockSha256", DOCLING_WORKER_ROOT / "Cargo.lock"),
        ("releaseProfileSha256", DOCLING_WORKER_ROOT / "release-profile.json"),
    ):
        digest = require_string(evidence.get(field), f"{evidence_label}.{field}")
        if digest != sha256_file(repo / relative):
            fail(f"Docling worker build evidence {field} is stale")

    evidence_native = require_object(
        evidence.get("nativeRuntime"), f"{evidence_label}.nativeRuntime"
    )
    require_exact_keys(
        evidence_native,
        {
            "component",
            "implementation",
            "version",
            "linkage",
            "installPath",
            "byteLength",
            "sha256",
            "rustBinding",
            "sourceArchive",
            "importLibrary",
            "workerImports",
            "runtimeImports",
            "forbiddenImportsAbsent",
        },
        set(),
        f"{evidence_label}.nativeRuntime",
    )
    runtime_file = expected_native_files["runtime_library"]
    import_file = expected_native_files["import_library"]
    expected_rust_binding = {
        "crate": "ort",
        "version": required_versions["ort"],
        "sysCrate": "ort-sys",
        "sysVersion": required_versions["ort-sys"],
    }
    expected_import_library = {
        "archivePath": import_file[0],
        "byteLength": import_file[1],
        "sha256": import_file[2],
    }

    def checked_imports(value: Any, label: str) -> list[str]:
        imports = [require_string(item, label) for item in require_array(value, label)]
        if (
            imports != sorted(set(imports))
            or any(item != item.lower() or not item.endswith(".dll") for item in imports)
        ):
            fail(f"{label} must be a sorted unique lowercase DLL inventory")
        return imports

    worker_imports = checked_imports(
        evidence_native.get("workerImports"), f"{evidence_label}.nativeRuntime.workerImports"
    )
    runtime_imports = checked_imports(
        evidence_native.get("runtimeImports"), f"{evidence_label}.nativeRuntime.runtimeImports"
    )
    forbidden_imports = {"d3d12.dll", "directml.dll"}
    if (
        evidence_native.get("component") != "onnx-runtime"
        or evidence_native.get("implementation") != "Microsoft ONNX Runtime"
        or evidence_native.get("version") != "1.24.2"
        or evidence_native.get("linkage") != "dynamic"
        or evidence_native.get("installPath") != "onnxruntime.dll"
        or evidence_native.get("byteLength") != runtime_file[1]
        or evidence_native.get("sha256") != runtime_file[2]
        or require_object(
            evidence_native.get("rustBinding"), f"{evidence_label}.nativeRuntime.rustBinding"
        )
        != expected_rust_binding
        or require_object(
            evidence_native.get("sourceArchive"),
            f"{evidence_label}.nativeRuntime.sourceArchive",
        )
        != expected_native_archive
        or require_object(
            evidence_native.get("importLibrary"),
            f"{evidence_label}.nativeRuntime.importLibrary",
        )
        != expected_import_library
        or "onnxruntime.dll" not in worker_imports
        or forbidden_imports.intersection(worker_imports)
        or forbidden_imports.intersection(runtime_imports)
        or set(
            checked_imports(
                evidence_native.get("forbiddenImportsAbsent"),
                f"{evidence_label}.nativeRuntime.forbiddenImportsAbsent",
            )
        )
        != forbidden_imports
    ):
        fail("Docling worker evidence does not bind the reviewed CPU-only native runtime")

    all_external_imports = set(worker_imports) | set(runtime_imports)
    packaged_imports = {"onnxruntime.dll"}
    unbound_runtime_imports = {
        item
        for item in all_external_imports
        if re.match(r"^(?:msvcp|vcruntime)[0-9_]*\.dll$", item)
        or item.startswith("api-ms-win-crt-")
    }
    windows_system_imports = all_external_imports - packaged_imports - unbound_runtime_imports
    external_imports = require_object(
        evidence.get("externalRuntimeImports"), f"{evidence_label}.externalRuntimeImports"
    )
    require_exact_keys(
        external_imports,
        {"packaged", "windowsSystem", "unboundMicrosoftRuntime"},
        set(),
        f"{evidence_label}.externalRuntimeImports",
    )
    if (
        set(
            checked_imports(
                external_imports.get("packaged"),
                f"{evidence_label}.externalRuntimeImports.packaged",
            )
        )
        != packaged_imports
        or set(
            checked_imports(
                external_imports.get("windowsSystem"),
                f"{evidence_label}.externalRuntimeImports.windowsSystem",
            )
        )
        != windows_system_imports
        or set(
            checked_imports(
                external_imports.get("unboundMicrosoftRuntime"),
                f"{evidence_label}.externalRuntimeImports.unboundMicrosoftRuntime",
            )
        )
        != unbound_runtime_imports
        or not unbound_runtime_imports
    ):
        fail("Docling worker external native import classification is incomplete")

    expected_asset_paths: dict[str, str] = {}
    reviewed_target = require_string(evidence.get("target"), f"{evidence_label}.target")
    for index, raw_layout in enumerate(
        require_array(profile.get("runtimeAssetLayout"), "Docling runtimeAssetLayout")
    ):
        layout = require_object(raw_layout, f"Docling runtimeAssetLayout[{index}]")
        require_exact_keys(
            layout,
            {"component"},
            {"path", "pathByTarget"},
            f"Docling runtimeAssetLayout[{index}]",
        )
        component = require_string(
            layout.get("component"), f"Docling runtimeAssetLayout[{index}].component"
        )
        if component in expected_asset_paths:
            fail("Docling runtime asset layout repeats a component")
        path = layout.get("path")
        if path is None:
            path = require_object(
                layout.get("pathByTarget"),
                f"Docling runtimeAssetLayout[{index}].pathByTarget",
            ).get(reviewed_target)
        elif layout.get("pathByTarget") is not None:
            fail("Docling runtime asset layout may have one path authority only")
        expected_asset_paths[component] = require_string(
            path, f"Docling runtimeAssetLayout[{index}] reviewed path"
        )
    if expected_asset_paths != {
        "pdfium": ".pdfium/lib/pdfium.dll",
        "layout-int8": "models/layout_heron_int8.onnx",
        "pp-ocr": "models/ocr_rec_en.onnx",
        "ocr-dictionary": "models/en_dict.txt",
    }:
        fail("Docling Windows runtime asset layout differs from the reviewed package")

    pinned_assets: dict[str, tuple[int, str]] = {}
    for index, raw_reviewed in enumerate(
        require_array(
            profile.get("reviewedExtractedArtifacts"),
            "Docling reviewedExtractedArtifacts",
        )
    ):
        reviewed = require_object(raw_reviewed, f"Docling reviewedExtractedArtifacts[{index}]")
        require_exact_keys(
            reviewed,
            {"component", "target", "byteLength", "sha256"},
            set(),
            f"Docling reviewedExtractedArtifacts[{index}]",
        )
        if reviewed.get("target") not in {"all", reviewed_target}:
            continue
        component = require_string(
            reviewed.get("component"),
            f"Docling reviewedExtractedArtifacts[{index}].component",
        )
        if component in pinned_assets:
            fail("Docling reviewed Windows asset pins repeat a component")
        pinned_assets[component] = (
            require_positive_integer(
                reviewed.get("byteLength"),
                f"Docling reviewedExtractedArtifacts[{index}].byteLength",
            ),
            require_string(
                reviewed.get("sha256"),
                f"Docling reviewedExtractedArtifacts[{index}].sha256",
            ),
        )
    if pinned_assets != {
        "pdfium": (
            7261184,
            "3aabcd60cec7c2bae8e40d63110b6b53dfe657015f268496fdf9ef9460cbe4d5",
        ),
        "layout-int8": (
            68543846,
            "5c7a4685c838b485069b81847f2c9330f7ffc488aefff7a8ceb7f7968c95e410",
        ),
        "pp-ocr": (
            8967018,
            "ef7abd8bd3629ae57ea2c28b425c1bd258a871b93fd2fe7c433946ade9b5d9ea",
        ),
        "ocr-dictionary": (
            190,
            "5662df9d2d03f0e8ca0d3b0649d6acbab904b6a14b3d3521463c71c37c668ce3",
        ),
    }:
        fail("Docling reviewed Windows runtime asset pins are incomplete")

    runtime_assets: dict[str, tuple[str, int, str]] = {}
    for index, raw_asset in enumerate(
        require_array(evidence.get("runtimeAssets"), f"{evidence_label}.runtimeAssets")
    ):
        asset = require_object(raw_asset, f"{evidence_label}.runtimeAssets[{index}]")
        require_exact_keys(
            asset,
            {"component", "installPath", "byteLength", "sha256"},
            set(),
            f"{evidence_label}.runtimeAssets[{index}]",
        )
        component = require_string(
            asset.get("component"), f"{evidence_label}.runtimeAssets[{index}].component"
        )
        if component in runtime_assets:
            fail("Docling worker build evidence repeats a runtime asset")
        runtime_assets[component] = (
            require_string(
                asset.get("installPath"),
                f"{evidence_label}.runtimeAssets[{index}].installPath",
            ),
            require_positive_integer(
                asset.get("byteLength"),
                f"{evidence_label}.runtimeAssets[{index}].byteLength",
            ),
            require_string(
                asset.get("sha256"), f"{evidence_label}.runtimeAssets[{index}].sha256"
            ),
        )
    if set(runtime_assets) != set(expected_asset_paths):
        fail("Docling worker build evidence runtime asset inventory is incomplete")
    for component, (path, byte_length, digest) in runtime_assets.items():
        if (
            path != expected_asset_paths[component]
            or pinned_assets.get(component) != (byte_length, digest)
            or not SHA256_RE.fullmatch(digest)
        ):
            fail(f"Docling worker build evidence does not match the {component} pin")
    physical_package_bytes = require_positive_integer(
        evidence.get("physicalPackageBytes"), f"{evidence_label}.physicalPackageBytes"
    )
    native_install_path = require_string(
        evidence_native.get("installPath"), f"{evidence_label}.nativeRuntime.installPath"
    )
    native_bytes = require_positive_integer(
        evidence_native.get("byteLength"), f"{evidence_label}.nativeRuntime.byteLength"
    )
    unique_physical_files = {worker_path: worker_bytes, native_install_path: native_bytes}
    if len(unique_physical_files) != 2:
        fail("Docling worker and native runtime may not alias one installed file")
    for path, byte_length, _ in runtime_assets.values():
        if path in unique_physical_files:
            fail("Docling worker build evidence aliases an external asset to the worker")
        unique_physical_files[path] = byte_length
    if physical_package_bytes != sum(unique_physical_files.values()):
        fail("Docling worker build evidence physicalPackageBytes is inconsistent")
    package_missing = [
        require_string(value, f"{evidence_label}.missingForPackage")
        for value in require_array(
            evidence.get("missingForPackage"), f"{evidence_label}.missingForPackage"
        )
    ]
    execution_missing = [
        require_string(value, f"{evidence_label}.missingForExecution")
        for value in require_array(
            evidence.get("missingForExecution"), f"{evidence_label}.missingForExecution"
        )
    ]
    if (
        evidence.get("packageComplete") is not False
        or not package_missing
        or not any("Visual C++" in value for value in package_missing if isinstance(value, str))
        or evidence.get("osSandboxEvidence") is not None
        or evidence.get("sandboxComplete") is not False
        or evidence.get("completeForExecution")
        is not (evidence.get("packageComplete") and evidence.get("sandboxComplete"))
        or not execution_missing
        or not set(package_missing).issubset(set(execution_missing))
        or not any("sandbox" in value for value in execution_missing if isinstance(value, str))
    ):
        fail("Docling worker target evidence makes an unsupported completeness claim")

    notices = (root / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
    for component in ("docling.rs", "ONNX Runtime", "PDFium", "PP-OCR", "Heron"):
        if component.casefold() not in notices.casefold():
            fail(f"Docling worker third-party notices omit {component}")
    if "1.24.2" not in notices or "dynamic" not in notices.casefold():
        fail("Docling worker notices confuse the Rust binding and native ONNX Runtime")
    return worker_rust


def validate_versions_and_toolchains(repo: Path) -> dict[str, str]:
    workspace = read_toml(repo / "Cargo.toml")
    release_version, members = validate_cargo_manifests(repo, workspace)
    validate_cargo_lock(repo, release_version, {name for name, _ in members})
    worker_rust = validate_docling_worker_source_policy(repo, release_version)
    package_defaults = require_object(workspace["workspace"]["package"], "workspace.package")
    declared_rust = normalize_rust_version(
        require_string(package_defaults.get("rust-version"), "rust-version")
    )
    toolchain = read_toml(repo / "rust-toolchain.toml").get("toolchain")
    toolchain_table = require_object(toolchain, "rust-toolchain toolchain")
    pinned_rust = normalize_rust_version(
        require_string(toolchain_table.get("channel"), "rust-toolchain channel")
    )
    if declared_rust != pinned_rust:
        fail("workspace rust-version must exactly match the pinned Rust toolchain")
    components = toolchain_table.get("components")
    if not isinstance(components, list) or not {"clippy", "rustfmt"}.issubset(components):
        fail("rust-toolchain must pin clippy and rustfmt")
    for package_path, lock_path in zip(PACKAGE_JSONS, PACKAGE_LOCKS):
        validate_npm_lock(repo / package_path, repo / lock_path, release_version)
    tauri = require_object(
        read_json(repo / "apps/desktop/src-tauri/tauri.conf.json"), "tauri.conf.json"
    )
    if tauri.get("version") != release_version:
        fail("Tauri package version does not match workspace release version")
    workflow = (repo / ".github/workflows/source-gate.yml").read_text(encoding="utf-8")
    if not re.search(
        rf"(?m)^\s*rustup toolchain install {re.escape(pinned_rust)}(?:\s|$)", workflow
    ):
        fail("source gate does not install the exact pinned Rust toolchain")
    required_rust_checks = (
        f"cargo +{pinned_rust} fmt --all -- --check",
        f"cargo +{pinned_rust} clippy --workspace --all-targets -- -D warnings",
        f"cargo +{pinned_rust} test --workspace --locked",
        f"cargo +{pinned_rust} check --workspace --all-targets --locked",
    )
    for command in required_rust_checks:
        if not re.search(
            rf"(?m)^\s*(?:run:\s*)?{re.escape(command)}\s*$", workflow
        ):
            fail(f"source gate is missing required Rust check: {command}")
    worker_checks = (
        f"rustup toolchain install {worker_rust} --profile minimal --component clippy --component rustfmt",
        f"cargo +{worker_rust} fetch --locked",
        f"cargo +{worker_rust} fmt --all -- --check",
        f"cargo +{worker_rust} check --all-targets --locked --offline",
        f"cargo +{worker_rust} clippy --all-targets --locked --offline -- -D warnings",
        f"cargo +{worker_rust} check --all-targets --locked --offline --target x86_64-pc-windows-msvc",
        f"cargo +{worker_rust} test --all-targets --locked --offline --target x86_64-pc-windows-msvc",
        f"cargo +{worker_rust} clippy --all-targets --locked --offline --target x86_64-pc-windows-msvc -- -D warnings",
        f"cargo +{worker_rust} build --release --locked --offline --target x86_64-pc-windows-msvc",
    )
    for command in worker_checks:
        if not re.search(rf"(?m)^\s*(?:run:\s*)?{re.escape(command)}\s*$", workflow):
            fail(f"source gate is missing required Docling worker check: {command}")
    for token in (
        "onnxruntime-win-x64-1.24.2.zip",
        "8e3e9c826375352e29cb2614fe44f3d7a4b0ff7b8028ad7a456af9d949a7e8b0",
        "114947d633e6844ce3c4b51ef6678f776628571d08a5763859c61642c8dcca9c",
        "2ec547a0e0e655fd60d549d23a3155a3ec47217f92de32e84df51866175a51ff",
        "ORT_PREFER_DYNAMIC_LINK=1",
        "ORT_SKIP_DOWNLOAD=1",
        "CARGO_NET_OFFLINE=true",
    ):
        if token not in workflow:
            fail(f"source gate does not bind the reviewed Docling native runtime: {token}")
    release_workflow = (repo / ".github/workflows/release-evidence.yml").read_text(
        encoding="utf-8"
    )
    required_worker_materialization = (
        f"rustup toolchain install {worker_rust} --profile minimal",
        f"cargo +{worker_rust} fetch --locked --manifest-path "
        f"{DOCLING_WORKER_ROOT.as_posix()}/Cargo.toml",
    )
    for command in required_worker_materialization:
        if not re.search(rf"(?m)^\s*{re.escape(command)}\s*$", release_workflow):
            fail(f"release evidence workflow does not materialize the worker lock: {command}")
    if len(re.findall(r"(?m)^\s*node-version:\s*22\.13\.0\s*$", workflow)) < 2:
        fail("source gate must pin Node.js 22.13.0 for both npm jobs")
    if not re.search(
        r"(?m)^\s*python3 scripts/release_evidence\.py policy(?:\s|$)", workflow
    ):
        fail("source gate does not run the release source-policy gate")
    dockerfile = (repo / "crates/weftext-server/deploy/Dockerfile").read_text(
        encoding="utf-8"
    )
    if f"FROM rust:{pinned_rust}-bookworm@sha256:" not in dockerfile:
        fail("Server container is not built with the exact pinned Rust toolchain image")
    return {
        "releaseVersion": release_version,
        "rust": pinned_rust,
        "node": "22.13.0",
    }


def load_release_trust(repo: Path, release_version: str) -> dict[str, str]:
    path = repo / "release/trust-policy.json"
    policy = require_object(read_json(path), "release trust policy")
    require_exact_keys(
        policy,
        {
            "schemaVersion",
            "cosignVersion",
            "certificateOidcIssuer",
            "certificateIdentityTemplate",
        },
        set(),
        "release trust policy",
    )
    if policy["schemaVersion"] != "weftext.release-trust.v1":
        fail("release trust policy schema is unsupported")
    cosign_version = require_string(policy["cosignVersion"], "trust cosignVersion")
    if not EXACT_VERSION_RE.fullmatch(cosign_version):
        fail("trust cosignVersion must be exact")
    issuer = require_string(policy["certificateOidcIssuer"], "trust certificateOidcIssuer")
    if issuer != "https://token.actions.githubusercontent.com":
        fail("release signatures must use the checked GitHub Actions OIDC issuer")
    template = require_string(
        policy["certificateIdentityTemplate"], "trust certificateIdentityTemplate"
    )
    if template.count("{releaseVersion}") != 1:
        fail("certificateIdentityTemplate must contain exactly one {releaseVersion}")
    identity = template.replace("{releaseVersion}", release_version)
    if not re.fullmatch(
        r"https://github\.com/ZHYX91/weftext/\.github/workflows/"
        r"release-artifacts\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?",
        identity,
    ):
        fail("release certificate identity is outside the trusted workflow and tag boundary")
    verifier_workflow = (repo / ".github/workflows/release-evidence.yml").read_text(
        encoding="utf-8"
    )
    if f"cosign-release: v{cosign_version}" not in verifier_workflow:
        fail("release-evidence workflow does not install the trusted cosign version")
    pinned_rust = normalize_rust_version(
        require_string(
            read_toml(repo / "rust-toolchain.toml")["toolchain"]["channel"],
            "rust-toolchain channel",
        )
    )
    if not re.search(
        rf"(?m)^\s*rustup toolchain install {re.escape(pinned_rust)} "
        r"--profile minimal\s*$",
        verifier_workflow,
    ):
        fail("release-evidence workflow does not install the locked Rust metadata toolchain")
    if not re.search(
        rf"(?m)^\s*cargo \+{re.escape(pinned_rust)} fetch --locked\s*$",
        verifier_workflow,
    ):
        fail("release-evidence workflow does not materialize only locked Cargo metadata")
    return {"cosignVersion": cosign_version, "identity": identity, "issuer": issuer}


def validate_docker_policy(repo: Path) -> None:
    excluded_directories = {".git", "legacy", "node_modules", "target"}
    dockerfiles = sorted(
        path
        for path in repo.rglob("Dockerfile*")
        if not path.name.endswith(".dockerignore")
        and not any(
            part in excluded_directories or part.startswith(".tmp-")
            for part in path.relative_to(repo).parts[:-1]
        )
    )
    if not dockerfiles:
        fail("no active Dockerfile was found")
    from_pattern = re.compile(r"(?im)^\s*FROM(?:\s+--platform=\S+)?\s+(\S+)")
    for path in dockerfiles:
        text = path.read_text(encoding="utf-8")
        syntax_directives = re.findall(r"(?im)^\s*#\s*syntax\s*=\s*(\S+)", text)
        for reference in syntax_directives:
            if not re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", reference):
                fail(f"{path} uses a non-digest-pinned Dockerfile frontend: {reference}")
        if re.search(
            r"(?i)\b(?:apt(?:-get)?\s+(?:update|install)|apk\s+add|dnf\s+install|yum\s+install)\b",
            text,
        ):
            fail(f"{path} performs a mutable package-manager install")
        references = from_pattern.findall(text)
        if not references:
            fail(f"{path} has no FROM instruction")
        aliases: set[str] = set()
        for line in text.splitlines():
            match = re.match(
                r"(?i)^\s*FROM(?:\s+--platform=\S+)?\s+(\S+)(?:\s+AS\s+(\S+))?\s*$", line
            )
            if not match:
                continue
            reference, alias = match.groups()
            if reference != "scratch" and reference not in aliases:
                if not re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", reference):
                    fail(f"{path} uses a non-digest-pinned base image: {reference}")
            if alias:
                aliases.add(alias)
    production_composes = sorted(
        path
        for path in (repo / "crates/weftext-server/deploy").glob("compose*.y*ml")
        if ".dev." not in path.name and ".test." not in path.name
    )
    immutable_template = re.compile(
        r"^\$\{WEFTEXT_(?P<image>SERVER|PROXY)_IMAGE_REPOSITORY:\?[^}]+\}@sha256:"
        r"\$\{WEFTEXT_(?P=image)_IMAGE_DIGEST:\?[^}]+\}$"
    )
    for path in production_composes:
        text = path.read_text(encoding="utf-8")
        if re.search(r"(?m)^\s*build\s*:", text):
            fail(f"production compose may not build mutable local source: {path}")
        images = re.findall(r"(?m)^\s*image\s*:\s*[\"']?([^\"'\r\n]+)[\"']?\s*$", text)
        if not images:
            fail(f"production compose has no image: {path}")
        for image in images:
            if not (
                re.fullmatch(r"[^\s@]+@sha256:[0-9a-f]{64}", image)
                or immutable_template.fullmatch(image)
            ):
                fail(f"production compose image is not immutable: {path}: {image}")


def validate_workflow_action_pins(repo: Path) -> None:
    action_pattern = re.compile(r"(?m)^\s*-?\s*uses:\s*([^\s#]+)")
    for path in sorted((repo / ".github/workflows").glob("*.y*ml")):
        text = path.read_text(encoding="utf-8")
        if re.search(r"\b(?:ubuntu|windows|macos)-latest\b", text, flags=re.IGNORECASE):
            fail(f"GitHub runner OS labels may not use -latest: {path}")
        for reference in action_pattern.findall(text):
            if reference.startswith("./"):
                continue
            if not re.fullmatch(r"[^@]+@[0-9a-f]{40}", reference):
                fail(f"GitHub Action must pin a full commit: {path}: {reference}")


def validate_checked_in_schemas(repo: Path) -> None:
    for relative in (
        Path("release/release-input.schema.json"),
        Path("release/release-evidence.schema.json"),
        Path("release/test-evidence.schema.json"),
        Path("release/docling-lite-release-profile.schema.json"),
        Path("release/docling-lite-build-evidence.schema.json"),
        Path("release/docling-lite-assets-lock.schema.json"),
    ):
        document = require_object(read_json(repo / relative), str(relative))
        if document.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
            fail(f"{relative} must use JSON Schema draft 2020-12")
        if document.get("type") != "object" or document.get("additionalProperties") is not False:
            fail(f"{relative} must define a closed top-level object")
        definitions = require_object(document.get("$defs"), f"{relative} $defs")

        def visit(value: Any) -> None:
            if isinstance(value, dict):
                reference = value.get("$ref")
                if reference is not None:
                    name = require_string(reference, f"{relative} $ref").removeprefix(
                        "#/$defs/"
                    )
                    if reference != f"#/$defs/{name}" or name not in definitions:
                        fail(f"{relative} has an unresolved or external $ref: {reference}")
                for child in value.values():
                    visit(child)
            elif isinstance(value, list):
                for child in value:
                    visit(child)

        visit(document)
        if relative.name in {"release-input.schema.json", "release-evidence.schema.json"}:
            scope = require_object(definitions.get("scope"), f"{relative} $defs.scope")
            scopes = require_array(scope.get("enum"), f"{relative} $defs.scope.enum")
            if set(scopes) != REQUIRED_SUPPLY_CHAIN_SCOPES or len(scopes) != len(
                REQUIRED_SUPPLY_CHAIN_SCOPES
            ):
                fail(f"{relative} supply-chain scopes do not match the release verifier")


def validate_source_policy(repo: Path) -> dict[str, str]:
    repo = repo.resolve()
    required = [repo / path for path in SOURCE_DIGEST_FILES]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        fail(f"required release inputs are missing: {', '.join(missing)}")
    versions = validate_versions_and_toolchains(repo)
    load_release_trust(repo, versions["releaseVersion"])
    validate_checked_in_schemas(repo)
    validate_docker_policy(repo)
    validate_workflow_action_pins(repo)
    return versions


def validate_packaged_asset_locks(repo: Path) -> None:
    path = repo / "crates/weftext-import/docling-lite-assets.lock.json"
    document = require_object(read_json(path), "Docling Lite asset lock")
    if document.get("lockVersion") != "weftext.docling-lite-assets.v1":
        fail("Docling Lite asset lock schema is unsupported")
    commit = document.get("doclingReleaseCommit")
    if not isinstance(commit, str) or not COMMIT_RE.fullmatch(commit):
        fail("Docling Lite release commit is not immutable")
    if document.get("completeForExecution") is not True:
        missing = document.get("missingForExecution", [])
        details = ", ".join(value for value in missing if isinstance(value, str))
        fail(f"Docling Lite packaged assets are incomplete for release: {details}")
    target_evidence = require_object(
        read_json(repo / DOCLING_WORKER_BUILD_EVIDENCE),
        "Docling Lite target execution evidence",
    )
    if (
        target_evidence.get("packageComplete") is not True
        or target_evidence.get("sandboxComplete") is not True
        or target_evidence.get("completeForExecution") is not True
        or target_evidence.get("osSandboxEvidence") is None
    ):
        fail("Docling Lite may be released only with complete target package and OS sandbox evidence")
    artifacts = require_array(document.get("artifacts"), "Docling Lite artifacts")
    if not artifacts:
        fail("Docling Lite asset lock contains no artifacts")
    for index, raw_artifact in enumerate(artifacts):
        artifact = require_object(raw_artifact, f"Docling Lite artifacts[{index}]")
        digest = require_string(
            artifact.get("sha256"), f"Docling Lite artifacts[{index}].sha256"
        )
        if not SHA256_RE.fullmatch(digest):
            fail(f"Docling Lite artifacts[{index}] has an invalid SHA-256")
        require_string(artifact.get("license"), f"Docling Lite artifacts[{index}].license")
        require_string(artifact.get("noticeId"), f"Docling Lite artifacts[{index}].noticeId")


def package_input_paths(repo: Path) -> list[Path]:
    paths = set(SOURCE_DIGEST_FILES)
    for manifest in repo.rglob("Cargo.toml"):
        relative = manifest.relative_to(repo)
        if "legacy" in relative.parts or "target" in relative.parts:
            continue
        paths.add(relative)
    return sorted(paths, key=lambda path: path.as_posix().encode("utf-8"))


def ensure_file_beneath(
    base: Path, raw_path: Any, label: str, *, max_bytes: int | None = None
) -> Path:
    relative = Path(require_string(raw_path, label))
    if relative.is_absolute() or ".." in relative.parts:
        fail(f"{label} must be a relative path beneath the release-input directory")
    candidate = base / relative
    current = base
    for part in relative.parts:
        current = current / part
        if current.is_symlink():
            fail(f"{label} may not traverse a symlink: {relative}")
    if not candidate.is_file():
        fail(f"{label} is not a regular file: {relative}")
    if max_bytes is not None and candidate.stat().st_size > max_bytes:
        fail(f"{label} exceeds {max_bytes} bytes")
    return candidate


def verify_expected_file(
    base: Path, value: dict[str, Any], label: str, *, max_bytes: int | None = None
) -> tuple[Path, str]:
    require_exact_keys(value, {"path", "expectedSha256"}, set(), label)
    path = ensure_file_beneath(base, value["path"], f"{label}.path", max_bytes=max_bytes)
    expected = require_string(value["expectedSha256"], f"{label}.expectedSha256")
    if not SHA256_RE.fullmatch(expected):
        fail(f"{label}.expectedSha256 must be lowercase SHA-256")
    actual = sha256_file(path)
    if actual != expected:
        fail(f"{label} digest mismatch: expected {expected}, found {actual}")
    return path, actual


def validate_text_evidence(path: Path, label: str) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        fail(f"{label} must be UTF-8 text: {error}")
    if len("".join(text.split())) < 40:
        fail(f"{label} is too short to be release evidence")
    placeholder = PLACEHOLDER_RE.search(text)
    if placeholder:
        fail(f"{label} contains an unfinished placeholder: {placeholder.group(0)!r}")


def validate_notice_coverage(
    path: Path, components: set[tuple[str, str]], label: str
) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        fail(f"{label} must be UTF-8 text: {error}")
    missing = sorted(
        f"{name}@{version}"
        for name, version in components
        if f"{name}@{version}" not in text
    )
    if missing:
        examples = ", ".join(missing[:12])
        suffix = " ..." if len(missing) > 12 else ""
        fail(
            f"{label} omits {len(missing)} SBOM component notice tokens: "
            f"{examples}{suffix}"
        )


def parse_utc_timestamp(value: Any, label: str) -> datetime:
    text = require_string(value, label)
    if not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", text):
        fail(f"{label} must be a whole-second UTC timestamp")
    try:
        parsed = datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError as error:
        fail(f"{label} is invalid: {error}")
    return parsed


def validate_test_evidence_document(
    path: Path, layer: str, release_version: str, source_commit: str, label: str
) -> None:
    document = require_object(read_json(path), label)
    require_exact_keys(
        document,
        {
            "schemaVersion",
            "layer",
            "releaseVersion",
            "sourceCommit",
            "result",
            "startedAt",
            "completedAt",
            "executor",
            "summary",
            "checks",
        },
        set(),
        label,
    )
    if document["schemaVersion"] != TEST_EVIDENCE_SCHEMA:
        fail(f"{label} schema must be {TEST_EVIDENCE_SCHEMA}")
    if document["layer"] != layer:
        fail(f"{label} layer does not match its release-input slot")
    if document["releaseVersion"] != release_version or document["sourceCommit"] != source_commit:
        fail(f"{label} is not evidence for this exact release source")
    if document["result"] != "passed":
        fail(f"{label} result must be passed")
    started = parse_utc_timestamp(document["startedAt"], f"{label}.startedAt")
    completed = parse_utc_timestamp(document["completedAt"], f"{label}.completedAt")
    if completed < started:
        fail(f"{label} completedAt precedes startedAt")
    executor = require_object(document["executor"], f"{label}.executor")
    require_exact_keys(executor, {"kind", "identity", "environment"}, set(), f"{label}.executor")
    if executor["kind"] not in {"ci", "human"}:
        fail(f"{label}.executor.kind must be ci or human")
    if layer == "manual-accessibility-daily-use" and executor["kind"] != "human":
        fail(f"{label} requires a human executor")
    require_string(executor["identity"], f"{label}.executor.identity")
    require_string(executor["environment"], f"{label}.executor.environment")
    summary = require_string(document["summary"], f"{label}.summary")
    if len("".join(summary.split())) < 40 or PLACEHOLDER_RE.search(summary):
        fail(f"{label}.summary is incomplete")
    checks = require_array(document["checks"], f"{label}.checks")
    if not checks:
        fail(f"{label}.checks must not be empty")
    names: set[str] = set()
    for index, raw_check in enumerate(checks):
        check_label = f"{label}.checks[{index}]"
        check = require_object(raw_check, check_label)
        require_exact_keys(check, {"name", "result", "details"}, set(), check_label)
        name = require_string(check["name"], f"{check_label}.name")
        if name in names:
            fail(f"{label} has duplicate check name {name!r}")
        names.add(name)
        if check["result"] != "passed":
            fail(f"{check_label}.result must be passed")
        details = require_string(check["details"], f"{check_label}.details")
        if len("".join(details.split())) < 20 or PLACEHOLDER_RE.search(details):
            fail(f"{check_label}.details is incomplete")


def validate_cyclonedx(
    path: Path, release_version: str, generator: dict[str, Any], label: str
) -> dict[str, str]:
    document = require_object(read_json(path), label)
    if document.get("bomFormat") != "CycloneDX" or document.get("version") != 1:
        fail(f"{label} is not a CycloneDX version-1 BOM")
    if document.get("specVersion") not in {"1.4", "1.5", "1.6", "1.7"}:
        fail(f"{label} uses an unsupported CycloneDX specVersion")
    serial = document.get("serialNumber")
    if not isinstance(serial, str) or not re.fullmatch(
        r"urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}",
        serial,
    ):
        fail(f"{label} must have a lowercase UUID serialNumber")
    metadata = require_object(document.get("metadata"), f"{label}.metadata")
    component = require_object(metadata.get("component"), f"{label}.metadata.component")
    if component.get("version") != release_version:
        fail(f"{label} metadata component version does not match the release")
    generator_name = require_string(generator.get("name"), f"{label}.generator.name")
    generator_version = require_string(generator.get("version"), f"{label}.generator.version")
    if not EXACT_VERSION_RE.fullmatch(generator_version):
        fail(f"{label} generator version must be exact")
    tools = metadata.get("tools", [])
    if isinstance(tools, dict):
        tools = tools.get("components", [])
    if not isinstance(tools, list):
        fail(f"{label}.metadata.tools must identify the generator")
    if not any(
        isinstance(tool, dict)
        and tool.get("name") == generator_name
        and tool.get("version") == generator_version
        for tool in tools
    ):
        fail(f"{label} does not record generator {generator_name} {generator_version}")
    components = document.get("components", [])
    if not isinstance(components, list):
        fail(f"{label}.components must be an array")
    for index, raw_component in enumerate(components):
        child = require_object(raw_component, f"{label}.components[{index}]")
        require_string(child.get("name"), f"{label}.components[{index}].name")
        require_string(child.get("version"), f"{label}.components[{index}].version")
        require_string(child.get("bom-ref"), f"{label}.components[{index}].bom-ref")
    return {"name": generator_name, "version": generator_version}


def cyclonedx_component_versions(path: Path, label: str) -> set[tuple[str, str]]:
    document = require_object(read_json(path), label)
    versions: set[tuple[str, str]] = set()
    for index, raw_component in enumerate(document.get("components", [])):
        component = require_object(raw_component, f"{label}.components[{index}]")
        name = require_string(component.get("name"), f"{label}.components[{index}].name")
        group = component.get("group")
        if isinstance(group, str) and group and not name.startswith(f"{group}/"):
            name = f"{group}/{name}"
        version = require_string(
            component.get("version"), f"{label}.components[{index}].version"
        )
        versions.add((name, version))
    return versions


def npm_package_name_from_locator(locator: str) -> str:
    """Return the package name from a package-lock v3 node_modules locator."""
    marker = "node_modules/"
    if not locator.startswith(marker):
        fail(f"npm package locator is invalid: {locator!r}")
    nested_marker = f"/{marker}"
    return locator.rsplit(nested_marker, 1)[-1].removeprefix(marker)


def expected_sbom_components(repo: Path, scope: str) -> set[tuple[str, str]] | None:
    if scope == "server-container":
        return None
    cargo_lock_by_scope = {
        "cargo": Path("Cargo.lock"),
        "docling-worker-cargo": DOCLING_WORKER_ROOT / "Cargo.lock",
    }
    if scope in cargo_lock_by_scope:
        lock = read_toml(repo / cargo_lock_by_scope[scope])
        return {
            (
                require_string(package.get("name"), "Cargo.lock package.name"),
                require_string(package.get("version"), "Cargo.lock package.version"),
            )
            for raw_package in require_array(lock.get("package"), "Cargo.lock package")
            for package in [require_object(raw_package, "Cargo.lock package entry")]
            if package.get("source") is not None
        }
    npm_lock_by_scope = {
        "desktop-npm": Path("apps/desktop/package-lock.json"),
        "server-webui-npm": Path("crates/weftext-server/webui/package-lock.json"),
        "webui-npm": Path("prototypes/webui/package-lock.json"),
    }
    lock_path = npm_lock_by_scope[scope]
    lock = require_object(read_json(repo / lock_path), str(lock_path))
    expected: set[tuple[str, str]] = set()
    for locator, raw_entry in require_object(lock.get("packages"), f"{lock_path} packages").items():
        if not locator.startswith("node_modules/"):
            continue
        entry = require_object(raw_entry, f"{lock_path} {locator}")
        expected.add(
            (
                npm_package_name_from_locator(locator),
                require_string(entry.get("version"), f"{lock_path} {locator}.version"),
            )
        )
    return expected


def validate_sbom_coverage(path: Path, expected: set[tuple[str, str]] | None, label: str) -> None:
    observed = cyclonedx_component_versions(path, label)
    if expected is None:
        if not observed:
            fail(f"{label} container SBOM contains no discovered components")
        return
    missing = sorted(expected - observed)
    if missing:
        examples = ", ".join(f"{name}@{version}" for name, version in missing[:12])
        suffix = " ..." if len(missing) > 12 else ""
        fail(
            f"{label} omits {len(missing)} locked components: {examples}{suffix}"
        )


def validate_sigstore_bundle_structure(path: Path, label: str) -> None:
    bundle = require_object(read_json(path), label)
    verification = require_object(
        bundle.get("verificationMaterial"), f"{label}.verificationMaterial"
    )
    chain = verification.get("x509CertificateChain")
    certificate = verification.get("certificate")
    if chain is not None:
        chain_object = require_object(chain, f"{label}.x509CertificateChain")
        certificates = require_array(chain_object.get("certificates"), f"{label}.certificates")
        if not certificates:
            fail(f"{label} certificate chain is empty")
        for index, raw_certificate in enumerate(certificates):
            certificate_object = require_object(raw_certificate, f"{label}.certificates[{index}]")
            require_string(
                certificate_object.get("rawBytes"),
                f"{label}.certificates[{index}].rawBytes",
            )
    elif certificate is not None:
        certificate_object = require_object(certificate, f"{label}.certificate")
        require_string(certificate_object.get("rawBytes"), f"{label}.certificate.rawBytes")
    else:
        fail(f"{label} has no embedded signing certificate")
    tlog_entries = require_array(verification.get("tlogEntries"), f"{label}.tlogEntries")
    if not tlog_entries:
        fail(f"{label} has no transparency-log inclusion evidence")
    require_object(bundle.get("messageSignature"), f"{label}.messageSignature")


def detect_cosign(expected_version: str, repo: Path) -> tuple[str, str]:
    if not EXACT_VERSION_RE.fullmatch(expected_version):
        fail("cosignVersion must be exact")
    executable = shutil.which("cosign")
    if executable is None:
        fail("cosign is required to verify external release signatures; it was not found")
    output = run_checked([executable, "version"], repo, description="cosign version")
    match = re.search(
        r"(?im)(?:GitVersion:\s*v?|cosign version\s+v?)([0-9]+\.[0-9]+\.[0-9]+)",
        output,
    )
    if not match:
        fail("could not determine the installed cosign version")
    actual = match.group(1)
    if actual != expected_version:
        fail(f"cosign version mismatch: expected {expected_version}, found {actual}")
    return executable, actual


def detect_rust_toolchain(expected_version: str, repo: Path) -> None:
    for tool in ("cargo", "rustc"):
        executable = shutil.which(tool)
        if executable is None:
            fail(f"{tool} is required to assemble release dependency evidence")
        output = run_checked([executable, "--version"], repo, description=f"{tool} version")
        match = re.match(rf"^{tool}\s+([0-9]+\.[0-9]+\.[0-9]+)(?:\s|$)", output)
        if not match:
            fail(f"could not determine the installed {tool} version")
        actual = match.group(1)
        if actual != expected_version:
            fail(f"{tool} version mismatch: expected {expected_version}, found {actual}")


def verify_cosign_bundle(
    executable: str,
    artifact: Path,
    bundle: Path,
    identity: str,
    issuer: str,
    repo: Path,
) -> None:
    if not identity.strip() or not issuer.startswith("https://"):
        fail("cosign certificate identity and HTTPS OIDC issuer must be explicit")
    run_checked(
        [
            executable,
            "verify-blob",
            "--bundle",
            str(bundle),
            "--certificate-identity",
            identity,
            "--certificate-oidc-issuer",
            issuer,
            str(artifact),
        ],
        repo,
        description=f"cosign verification for {artifact.name}",
    )


def verify_cosign_oci(
    executable: str,
    reference: str,
    bundle: Path,
    identity: str,
    issuer: str,
    repo: Path,
) -> None:
    if not re.fullmatch(
        r"[a-z0-9.-]+(?::[0-9]+)?/[a-z0-9._/-]+@sha256:[0-9a-f]{64}", reference
    ):
        fail("container artifact must use an immutable lowercase OCI digest reference")
    run_checked(
        [
            executable,
            "verify",
            "--bundle",
            str(bundle),
            "--certificate-identity",
            identity,
            "--certificate-oidc-issuer",
            issuer,
            reference,
        ],
        repo,
        description=f"cosign verification for {reference}",
    )


def git_release_state(repo: Path, expected_commit: str) -> tuple[str, int]:
    if not COMMIT_RE.fullmatch(expected_commit):
        fail("sourceCommit must be a lowercase full Git commit")
    head = run_checked(["git", "rev-parse", "HEAD"], repo, description="Git HEAD").strip()
    if head != expected_commit:
        fail(f"sourceCommit does not match HEAD: expected {expected_commit}, found {head}")
    status = run_checked(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        repo,
        description="Git clean-state check",
    )
    if status:
        fail("release verification requires a completely clean source checkout")
    timestamp_text = run_checked(
        ["git", "show", "-s", "--format=%ct", "HEAD"], repo, description="Git commit timestamp"
    ).strip()
    try:
        timestamp = int(timestamp_text)
    except ValueError:
        fail("Git commit timestamp is invalid")
    return head, timestamp


def build_source_manifest(repo: Path, source_commit: str) -> dict[str, Any]:
    listing = run_checked(["git", "ls-files", "-z"], repo, description="tracked source listing")
    paths = [Path(value) for value in listing.split("\0") if value]
    index = run_checked(["git", "ls-files", "-s", "-z"], repo, description="Git index listing")
    modes: dict[Path, str] = {}
    for record in (value for value in index.split("\0") if value):
        try:
            metadata, raw_path = record.split("\t", 1)
        except ValueError:
            fail("Git index contains an invalid entry")
        mode = metadata.split(" ", 1)[0]
        if mode in {"120000", "160000"}:
            fail("release source may not contain tracked symlinks or submodules")
        if mode not in {"100644", "100755"}:
            fail(f"release source contains unsupported Git mode {mode}")
        modes[Path(raw_path)] = mode
    files: list[dict[str, Any]] = []
    root_digest = hashlib.sha256()
    for relative in sorted(paths, key=lambda value: value.as_posix().encode("utf-8")):
        path = repo / relative
        if not path.is_file() or path.is_symlink():
            fail(f"tracked source is not a regular file: {relative.as_posix()}")
        digest = sha256_file(path)
        size = path.stat().st_size
        portable = relative.as_posix()
        mode = modes.get(relative)
        if mode is None:
            fail(f"tracked source is missing from the Git index: {portable}")
        entry = {"path": portable, "gitMode": mode, "sha256": digest, "size": size}
        files.append(entry)
        root_digest.update(canonical_json_bytes(entry))
    return {
        "schemaVersion": SOURCE_MANIFEST_SCHEMA,
        "sourceCommit": source_commit,
        "treeSha256": root_digest.hexdigest(),
        "files": files,
    }


def cargo_license_report(
    repo: Path, manifest_path: Path | None = None
) -> dict[str, Any]:
    cargo = shutil.which("cargo")
    if cargo is None:
        fail("cargo is required to collect the locked Rust license report")
    command = [cargo, "metadata", "--locked", "--offline", "--format-version", "1"]
    description = "locked offline Cargo metadata"
    if manifest_path is not None:
        command.extend(["--manifest-path", manifest_path.as_posix()])
        description = f"locked offline Cargo metadata for {manifest_path.as_posix()}"
    output = run_checked(
        command,
        repo,
        description=description,
    )
    try:
        metadata = json.loads(output)
    except json.JSONDecodeError as error:
        fail(f"cargo metadata returned invalid JSON: {error}")
    workspace_members = set(metadata.get("workspace_members", []))
    packages: list[dict[str, Any]] = []
    for raw_package in metadata.get("packages", []):
        package = require_object(raw_package, "cargo metadata package")
        if package.get("id") in workspace_members:
            continue
        name = require_string(package.get("name"), "cargo package name")
        version = require_string(package.get("version"), f"cargo package {name} version")
        license_expression = package.get("license")
        license_file = package.get("license_file")
        if not license_expression and not license_file:
            fail(f"Rust dependency {name} {version} declares no license or license_file")
        entry: dict[str, Any] = {
            "name": name,
            "version": version,
            "source": package.get("source"),
            "license": license_expression,
        }
        if license_file:
            license_path = Path(license_file)
            if not license_path.is_file() or license_path.is_symlink():
                fail(f"Rust dependency {name} license_file is unavailable")
            entry["licenseFile"] = {
                "name": license_path.name,
                "sha256": sha256_file(license_path),
            }
        packages.append(entry)
    packages.sort(key=lambda item: (item["name"], item["version"], item.get("source") or ""))
    if not packages:
        fail("Cargo license report contains no third-party dependencies")
    return {"schemaVersion": LICENSE_REPORT_SCHEMA, "ecosystem": "cargo", "packages": packages}


def npm_license_report(repo: Path) -> dict[str, Any]:
    packages: list[dict[str, Any]] = []
    for relative in PACKAGE_LOCKS:
        lock = require_object(read_json(repo / relative), str(relative))
        locked_packages = require_object(lock.get("packages"), f"{relative} packages")
        for locator, raw_entry in locked_packages.items():
            if not locator.startswith("node_modules/"):
                continue
            entry = require_object(raw_entry, f"{relative} {locator}")
            packages.append(
                {
                    "application": relative.parent.as_posix(),
                    "locator": locator,
                    "name": npm_package_name_from_locator(locator),
                    "version": require_string(
                        entry.get("version"), f"{relative} {locator}.version"
                    ),
                    "license": require_string(
                        entry.get("license"), f"{relative} {locator}.license"
                    ),
                    "integrity": require_string(
                        entry.get("integrity"), f"{relative} {locator}.integrity"
                    ),
                }
            )
    packages.sort(
        key=lambda item: (
            item["application"],
            item["name"],
            item["version"],
            item["locator"],
        )
    )
    if not packages:
        fail("npm license report contains no third-party dependencies")
    return {"schemaVersion": LICENSE_REPORT_SCHEMA, "ecosystem": "npm", "packages": packages}


def parse_release_input(input_path: Path, release_version: str) -> dict[str, Any]:
    document = require_object(read_json(input_path), "release input")
    require_exact_keys(
        document,
        {
            "schemaVersion",
            "releaseVersion",
            "sourceCommit",
            "artifacts",
            "sboms",
            "noticeFiles",
            "testEvidence",
            "knownLimitations",
            "migrationPath",
            "backupCompatibility",
            "rollbackProcedure",
        },
        set(),
        "release input",
    )
    if document["schemaVersion"] != RELEASE_INPUT_SCHEMA:
        fail(f"release input schema must be {RELEASE_INPUT_SCHEMA}")
    if document["releaseVersion"] != release_version:
        fail("release input version does not match the source release set")
    require_string(document["sourceCommit"], "sourceCommit")
    for field in ("artifacts", "sboms", "noticeFiles", "testEvidence"):
        require_array(document[field], field)
    for field in ("knownLimitations", "migrationPath", "backupCompatibility", "rollbackProcedure"):
        require_object(document[field], field)
    return document


def verify_release(repo: Path, input_path: Path, output_path: Path) -> Path:
    repo = repo.resolve()
    versions = validate_source_policy(repo)
    trust = load_release_trust(repo, versions["releaseVersion"])
    if not input_path.is_file() or input_path.is_symlink():
        fail("release input must be a regular, non-symlink file")
    if input_path.stat().st_size > 16 * 1024 * 1024:
        fail("release input exceeds 16777216 bytes")
    if output_path.is_symlink():
        fail("release evidence output may not replace a symlink")
    input_path = input_path.resolve()
    output_path = output_path.resolve()
    input_digest = sha256_file(input_path)
    document = parse_release_input(input_path, versions["releaseVersion"])
    source_commit, source_date_epoch = git_release_state(repo, document["sourceCommit"])
    validate_packaged_asset_locks(repo)
    input_base = input_path.resolve().parent
    try:
        output_path.relative_to(repo)
    except ValueError:
        pass
    else:
        fail("release evidence output must be outside the clean source checkout")
    if output_path.exists():
        fail("release evidence output already exists; refusing to overwrite it")

    detect_rust_toolchain(versions["rust"], repo)
    cosign, cosign_version = detect_cosign(trust["cosignVersion"], repo)
    preserved_files: dict[Path, tuple[Path, str]] = {
        Path("release-input.json"): (input_path, input_digest)
    }
    artifacts: list[dict[str, Any]] = []
    artifact_components: set[str] = set()
    artifact_names: set[str] = set()
    for index, raw_artifact in enumerate(document["artifacts"]):
        label = f"artifacts[{index}]"
        artifact = require_object(raw_artifact, label)
        require_exact_keys(
            artifact,
            {
                "name",
                "component",
                "platform",
                "version",
                "signatureBundle",
                "signatureBundleSha256",
            },
            {"path", "expectedSha256", "ociReference"},
            label,
        )
        name = require_string(artifact["name"], f"{label}.name")
        if name in artifact_names:
            fail(f"duplicate artifact name: {name}")
        artifact_names.add(name)
        component = require_string(artifact["component"], f"{label}.component")
        if component not in REQUIRED_ARTIFACT_COMPONENTS:
            fail(f"unsupported artifact component: {component}")
        artifact_components.add(component)
        if artifact["version"] != versions["releaseVersion"]:
            fail(f"{label}.version does not match release")
        artifact_path: Path | None = None
        expected: str | None = None
        oci_reference = artifact.get("ociReference")
        if component == "server-container":
            if "path" in artifact or "expectedSha256" in artifact:
                fail(f"{label} container authority is its signed OCI digest, not a local file")
        else:
            if oci_reference is not None:
                fail(f"{label}.ociReference is only valid for server-container")
            artifact_path = ensure_file_beneath(
                input_base, artifact.get("path"), f"{label}.path"
            )
            expected = require_string(
                artifact.get("expectedSha256"), f"{label}.expectedSha256"
            )
            if not SHA256_RE.fullmatch(expected) or sha256_file(artifact_path) != expected:
                fail(f"{label} artifact SHA-256 does not match")
        bundle_path = ensure_file_beneath(
            input_base,
            artifact["signatureBundle"],
            f"{label}.signatureBundle",
            max_bytes=16 * 1024 * 1024,
        )
        bundle_digest = require_string(
            artifact["signatureBundleSha256"], f"{label}.signatureBundleSha256"
        )
        if not SHA256_RE.fullmatch(bundle_digest) or sha256_file(bundle_path) != bundle_digest:
            fail(f"{label} signature bundle SHA-256 does not match")
        validate_sigstore_bundle_structure(bundle_path, f"{label} signature bundle")
        preserved_bundle = Path("signature-bundles") / f"{bundle_digest}.sigstore.json"
        preserved_files.setdefault(preserved_bundle, (bundle_path, bundle_digest))
        identity = trust["identity"]
        issuer = trust["issuer"]
        if component == "server-container":
            reference = require_string(oci_reference, f"{label}.ociReference")
            verify_cosign_oci(cosign, reference, bundle_path, identity, issuer, repo)
        else:
            reference = None
            assert artifact_path is not None
            verify_cosign_bundle(cosign, artifact_path, bundle_path, identity, issuer, repo)
        artifact_record: dict[str, Any] = {
            "name": name,
            "component": component,
            "platform": require_string(artifact["platform"], f"{label}.platform"),
            "version": versions["releaseVersion"],
            "signature": {
                "bundlePath": preserved_bundle.as_posix(),
                "bundleSha256": bundle_digest,
                "certificateIdentity": identity,
                "certificateOidcIssuer": issuer,
                "verifiedBy": f"cosign {cosign_version}",
                "verifiedSubject": reference if reference is not None else artifact["path"],
            },
        }
        if reference is not None:
            artifact_record["ociReference"] = reference
            artifact_record["sha256"] = reference.rsplit("@sha256:", 1)[1]
        else:
            assert artifact_path is not None and expected is not None
            artifact_record["path"] = artifact["path"]
            artifact_record["sha256"] = expected
            artifact_record["size"] = artifact_path.stat().st_size
        artifacts.append(artifact_record)
    missing_artifacts = sorted(REQUIRED_ARTIFACT_COMPONENTS - artifact_components)
    if missing_artifacts:
        fail(f"release set is missing artifacts: {', '.join(missing_artifacts)}")

    sboms: list[dict[str, Any]] = []
    sbom_scopes: set[str] = set()
    sbom_components: dict[str, set[tuple[str, str]]] = {}
    for index, raw_sbom in enumerate(document["sboms"]):
        label = f"sboms[{index}]"
        sbom = require_object(raw_sbom, label)
        require_exact_keys(
            sbom, {"scope", "path", "expectedSha256", "generator"}, set(), label
        )
        scope = require_string(sbom["scope"], f"{label}.scope")
        if scope not in REQUIRED_SUPPLY_CHAIN_SCOPES or scope in sbom_scopes:
            fail(f"invalid or duplicate SBOM scope: {scope}")
        sbom_scopes.add(scope)
        # generator is deliberately separate from the common evidence reference.
        sbom_ref = {"path": sbom["path"], "expectedSha256": sbom["expectedSha256"]}
        path, digest = verify_expected_file(input_base, sbom_ref, label, max_bytes=64 * 1024 * 1024)
        generator = require_object(sbom["generator"], f"{label}.generator")
        require_exact_keys(generator, {"name", "version"}, set(), f"{label}.generator")
        if generator.get("name") != SBOM_GENERATORS[scope]:
            fail(
                f"{label} must be generated by the approved {SBOM_GENERATORS[scope]} tool"
            )
        generator_record = validate_cyclonedx(
            path, versions["releaseVersion"], generator, label
        )
        validate_sbom_coverage(path, expected_sbom_components(repo, scope), label)
        sbom_components[scope] = cyclonedx_component_versions(path, label)
        preserved_path = Path("sboms") / f"{scope}.cdx.json"
        preserved_files[preserved_path] = (path, digest)
        sboms.append(
            {
                "scope": scope,
                "path": preserved_path.as_posix(),
                "sha256": digest,
                "generator": generator_record,
            }
        )
    missing_sboms = sorted(REQUIRED_SUPPLY_CHAIN_SCOPES - sbom_scopes)
    if missing_sboms:
        fail(f"release set is missing CycloneDX SBOMs: {', '.join(missing_sboms)}")

    notices: list[dict[str, str]] = []
    notice_scopes: set[str] = set()
    for index, raw_notice in enumerate(document["noticeFiles"]):
        label = f"noticeFiles[{index}]"
        notice = require_object(raw_notice, label)
        require_exact_keys(notice, {"scope", "path", "expectedSha256"}, set(), label)
        scope = require_string(notice["scope"], f"{label}.scope")
        if scope not in REQUIRED_SUPPLY_CHAIN_SCOPES or scope in notice_scopes:
            fail(f"invalid or duplicate notice scope: {scope}")
        notice_scopes.add(scope)
        path, digest = verify_expected_file(
            input_base,
            {"path": notice["path"], "expectedSha256": notice["expectedSha256"]},
            label,
            max_bytes=16 * 1024 * 1024,
        )
        validate_text_evidence(path, label)
        validate_notice_coverage(path, sbom_components[scope], label)
        preserved_path = Path("notices") / f"{scope}.txt"
        preserved_files[preserved_path] = (path, digest)
        notices.append({"scope": scope, "path": preserved_path.as_posix(), "sha256": digest})
    missing_notices = sorted(REQUIRED_SUPPLY_CHAIN_SCOPES - notice_scopes)
    if missing_notices:
        fail(f"release set is missing notices: {', '.join(missing_notices)}")

    tests: list[dict[str, str]] = []
    test_layers: set[str] = set()
    for index, raw_test in enumerate(document["testEvidence"]):
        label = f"testEvidence[{index}]"
        test = require_object(raw_test, label)
        require_exact_keys(test, {"layer", "path", "expectedSha256"}, set(), label)
        layer = require_string(test["layer"], f"{label}.layer")
        if layer not in REQUIRED_TEST_LAYERS or layer in test_layers:
            fail(f"invalid or duplicate test evidence layer: {layer}")
        test_layers.add(layer)
        path, digest = verify_expected_file(
            input_base,
            {"path": test["path"], "expectedSha256": test["expectedSha256"]},
            label,
            max_bytes=256 * 1024 * 1024,
        )
        validate_test_evidence_document(
            path, layer, versions["releaseVersion"], source_commit, label
        )
        preserved_path = Path("test-evidence") / f"{layer}.json"
        preserved_files[preserved_path] = (path, digest)
        tests.append({"layer": layer, "path": preserved_path.as_posix(), "sha256": digest})
    missing_tests = sorted(REQUIRED_TEST_LAYERS - test_layers)
    if missing_tests:
        fail(f"release set is missing test evidence: {', '.join(missing_tests)}")

    narrative: dict[str, dict[str, str]] = {}
    for field in ("knownLimitations", "migrationPath", "backupCompatibility", "rollbackProcedure"):
        reference = require_object(document[field], field)
        path, digest = verify_expected_file(
            input_base, reference, field, max_bytes=16 * 1024 * 1024
        )
        validate_text_evidence(path, field)
        preserved_path = Path("narrative") / f"{field}.txt"
        preserved_files[preserved_path] = (path, digest)
        narrative[field] = {"path": preserved_path.as_posix(), "sha256": digest}

    source_manifest = build_source_manifest(repo, source_commit)
    cargo_licenses = cargo_license_report(repo)
    docling_worker_cargo_licenses = cargo_license_report(
        repo, DOCLING_WORKER_ROOT / "Cargo.toml"
    )
    npm_licenses = npm_license_report(repo)
    generated_files = {
        "sourceManifestSha256": hashlib.sha256(
            formatted_json_bytes(source_manifest)
        ).hexdigest(),
        "cargoLicensesSha256": hashlib.sha256(
            formatted_json_bytes(cargo_licenses)
        ).hexdigest(),
        "doclingWorkerCargoLicensesSha256": hashlib.sha256(
            formatted_json_bytes(docling_worker_cargo_licenses)
        ).hexdigest(),
        "npmLicensesSha256": hashlib.sha256(formatted_json_bytes(npm_licenses)).hexdigest(),
    }
    lockfiles = [
        {"path": path.as_posix(), "sha256": sha256_file(repo / path)}
        for path in (
            Path("Cargo.lock"),
            DOCLING_WORKER_ROOT / "Cargo.lock",
            *PACKAGE_LOCKS,
            Path("crates/weftext-import/docling-lite-assets.lock.json"),
        )
    ]
    package_inputs = [
        {"path": path.as_posix(), "sha256": sha256_file(repo / path)}
        for path in package_input_paths(repo)
    ]
    evidence = {
        "schemaVersion": RELEASE_EVIDENCE_SCHEMA,
        "releaseVersion": versions["releaseVersion"],
        "sourceCommit": source_commit,
        "sourceDateEpoch": source_date_epoch,
        "releaseInput": {"path": "release-input.json", "sha256": input_digest},
        "sourceTreeSha256": source_manifest["treeSha256"],
        "toolchains": {
            "rust": versions["rust"],
            "node": versions["node"],
            "cosign": cosign_version,
        },
        "lockfiles": lockfiles,
        "packageInputs": package_inputs,
        "dependencyLicenses": {
            "cargo": "cargo-licenses.json",
            "doclingWorkerCargo": "docling-worker-cargo-licenses.json",
            "npm": "npm-licenses.json",
        },
        "sourceManifest": "source-files.json",
        "generatedFiles": generated_files,
        "artifacts": sorted(artifacts, key=lambda item: (item["component"], item["name"])),
        "sboms": sorted(sboms, key=lambda item: item["scope"]),
        "noticeFiles": sorted(notices, key=lambda item: item["scope"]),
        "testEvidence": sorted(tests, key=lambda item: item["layer"]),
        **narrative,
    }
    evidence["releaseSetSha256"] = hashlib.sha256(canonical_json_bytes(evidence)).hexdigest()

    output_parent = output_path.parent
    output_parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=f".{output_path.name}.tmp-", dir=output_parent))
    try:
        for relative, (source, expected_digest) in sorted(
            preserved_files.items(), key=lambda item: item[0].as_posix().encode("utf-8")
        ):
            destination = temporary / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
            if sha256_file(destination) != expected_digest:
                fail(f"preserved evidence digest changed while copying {relative.as_posix()}")
        write_json(temporary / "source-files.json", source_manifest)
        write_json(temporary / "cargo-licenses.json", cargo_licenses)
        write_json(
            temporary / "docling-worker-cargo-licenses.json",
            docling_worker_cargo_licenses,
        )
        write_json(temporary / "npm-licenses.json", npm_licenses)
        write_json(temporary / "release-evidence.json", evidence)
        os.replace(temporary, output_path)
    except BaseException:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    return output_path / "release-evidence.json"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    policy = commands.add_parser("policy", help="run the network-free source policy gate")
    policy.add_argument("--repo", type=Path, default=Path.cwd())
    verify = commands.add_parser(
        "verify-release", help="verify external release inputs and emit deterministic evidence"
    )
    verify.add_argument("--repo", type=Path, default=Path.cwd())
    verify.add_argument("--input", type=Path, required=True)
    verify.add_argument("--output", type=Path, required=True)
    return parser


def main(arguments: list[str] | None = None) -> int:
    options = build_parser().parse_args(arguments)
    try:
        if options.command == "policy":
            versions = validate_source_policy(options.repo)
            print(
                "release source policy passed: "
                f"version={versions['releaseVersion']} "
                f"rust={versions['rust']} "
                f"node={versions['node']}"
            )
        else:
            evidence_path = verify_release(options.repo, options.input, options.output)
            print(f"verified release evidence: {evidence_path}")
    except GateError as error:
        print(f"release evidence gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
