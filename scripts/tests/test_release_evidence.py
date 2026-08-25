from __future__ import annotations

import importlib.util
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "release_evidence.py"
SPEC = importlib.util.spec_from_file_location("release_evidence", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
release_evidence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_evidence)


class ReleaseEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixtures = Path(__file__).parent / "fixtures"

    def test_checked_in_repository_passes_static_policy(self) -> None:
        repo = Path(__file__).resolve().parents[2]
        versions = release_evidence.validate_source_policy(repo)
        self.assertEqual(versions["releaseVersion"], "0.1.0")
        self.assertEqual(versions["rust"], "1.98.0")
        trust = release_evidence.load_release_trust(repo, versions["releaseVersion"])
        self.assertEqual(trust["cosignVersion"], "3.0.6")
        self.assertTrue(trust["identity"].endswith("@refs/tags/v0.1.0"))

    def test_direct_npm_ranges_are_rejected(self) -> None:
        with self.assertRaisesRegex(release_evidence.GateError, "exact version"):
            release_evidence.validate_direct_npm_spec("^4.1.11", "vitest")

    def test_docling_worker_has_a_separate_exact_source_policy(self) -> None:
        repo = Path(__file__).resolve().parents[2]
        worker_rust = release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")
        self.assertEqual(worker_rust, "1.98.0")
        self.assertIn(
            Path("workers/weftext-docling-lite/Cargo.lock"),
            release_evidence.SOURCE_DIGEST_FILES,
        )

    def test_release_schemas_bind_every_required_supply_chain_scope(self) -> None:
        repo = Path(__file__).resolve().parents[2]
        release_evidence.validate_checked_in_schemas(repo)
        for name in ("release-input.schema.json", "release-evidence.schema.json"):
            schema = json.loads((repo / "release" / name).read_text(encoding="utf-8"))
            self.assertEqual(
                set(schema["$defs"]["scope"]["enum"]),
                release_evidence.REQUIRED_SUPPLY_CHAIN_SCOPES,
            )

    def test_docling_worker_rejects_a_direct_compatible_version_range(self) -> None:
        source_repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            destination = repo / release_evidence.DOCLING_WORKER_ROOT
            shutil.copytree(
                source_repo / release_evidence.DOCLING_WORKER_ROOT,
                destination,
                ignore=shutil.ignore_patterns("target"),
            )
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            shutil.copy2(
                source_repo / "crates/weftext-import/docling-lite-assets.lock.json",
                asset_lock,
            )
            manifest_path = destination / "Cargo.toml"
            manifest = manifest_path.read_text(encoding="utf-8")
            changed, replacements = re.subn(
                r'(serde\s*=\s*\{\s*version\s*=\s*)"=[^"]+"',
                r'\1"1.0"',
                manifest,
                count=1,
            )
            self.assertEqual(replacements, 1)
            manifest_path.write_text(changed, encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "exact registry version"):
                release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")

    def test_docling_worker_rejects_a_target_specific_version_range(self) -> None:
        source_repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            destination = repo / release_evidence.DOCLING_WORKER_ROOT
            shutil.copytree(
                source_repo / release_evidence.DOCLING_WORKER_ROOT,
                destination,
                ignore=shutil.ignore_patterns("target"),
            )
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            shutil.copy2(
                source_repo / "crates/weftext-import/docling-lite-assets.lock.json",
                asset_lock,
            )
            manifest_path = destination / "Cargo.toml"
            manifest_path.write_text(
                manifest_path.read_text(encoding="utf-8")
                + "\n[target.'cfg(windows)'.dependencies]\nlibc = \"0.2\"\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "exact registry version"):
                release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")

    def test_docling_worker_rejects_stale_binary_build_evidence(self) -> None:
        source_repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            destination = repo / release_evidence.DOCLING_WORKER_ROOT
            shutil.copytree(
                source_repo / release_evidence.DOCLING_WORKER_ROOT,
                destination,
                ignore=shutil.ignore_patterns("target"),
            )
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            shutil.copy2(
                source_repo / "crates/weftext-import/docling-lite-assets.lock.json",
                asset_lock,
            )
            evidence_path = repo / release_evidence.DOCLING_WORKER_BUILD_EVIDENCE
            evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
            evidence["nativeRuntime"]["sha256"] = "0" * 64
            evidence_path.write_text(json.dumps(evidence), encoding="utf-8")
            with self.assertRaisesRegex(
                release_evidence.GateError, "CPU-only native runtime"
            ):
                release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")

    def test_docling_worker_rejects_directml_in_the_attested_import_table(self) -> None:
        source_repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            destination = repo / release_evidence.DOCLING_WORKER_ROOT
            shutil.copytree(
                source_repo / release_evidence.DOCLING_WORKER_ROOT,
                destination,
                ignore=shutil.ignore_patterns("target"),
            )
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            shutil.copy2(
                source_repo / "crates/weftext-import/docling-lite-assets.lock.json",
                asset_lock,
            )
            evidence_path = repo / release_evidence.DOCLING_WORKER_BUILD_EVIDENCE
            evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
            evidence["nativeRuntime"]["workerImports"] = sorted(
                evidence["nativeRuntime"]["workerImports"] + ["directml.dll"]
            )
            evidence["externalRuntimeImports"]["windowsSystem"] = sorted(
                evidence["externalRuntimeImports"]["windowsSystem"] + ["directml.dll"]
            )
            evidence_path.write_text(json.dumps(evidence), encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "CPU-only native runtime"):
                release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")

    def test_docling_worker_rejects_a_build_environment_that_can_download_native_code(self) -> None:
        source_repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            destination = repo / release_evidence.DOCLING_WORKER_ROOT
            shutil.copytree(
                source_repo / release_evidence.DOCLING_WORKER_ROOT,
                destination,
                ignore=shutil.ignore_patterns("target"),
            )
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            shutil.copy2(
                source_repo / "crates/weftext-import/docling-lite-assets.lock.json",
                asset_lock,
            )
            profile_path = destination / "release-profile.json"
            profile = json.loads(profile_path.read_text(encoding="utf-8"))
            profile["reviewedNativeRuntime"]["buildEnvironment"]["ORT_SKIP_DOWNLOAD"] = "0"
            profile_path.write_text(json.dumps(profile), encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "Microsoft CPU archive"):
                release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")

    def test_docling_worker_rejects_a_success_envelope_drift(self) -> None:
        source_repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            destination = repo / release_evidence.DOCLING_WORKER_ROOT
            shutil.copytree(
                source_repo / release_evidence.DOCLING_WORKER_ROOT,
                destination,
                ignore=shutil.ignore_patterns("target"),
            )
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            shutil.copy2(
                source_repo / "crates/weftext-import/docling-lite-assets.lock.json",
                asset_lock,
            )
            source_path = destination / "src/lib.rs"
            source = source_path.read_text(encoding="utf-8")
            changed, replacements = re.subn(
                r"WorkerOutput::Completed\(document\) => serde_json::to_vec\(document\)",
                "WorkerOutput::Completed(document) => serialize_success_envelope(document)",
                source,
                count=1,
            )
            self.assertEqual(replacements, 1)
            source_path.write_text(changed, encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "raw DoclingDocument"):
                release_evidence.validate_docling_worker_source_policy(repo, "0.1.0")

    def test_floating_github_runner_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            workflows = repo / ".github/workflows"
            workflows.mkdir(parents=True)
            (workflows / "ci.yml").write_text(
                "jobs:\n  test:\n    runs-on: ubuntu-latest\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(release_evidence.GateError, "may not use -latest"):
                release_evidence.validate_workflow_action_pins(repo)

    def test_dangling_release_schema_reference_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            release = repo / "release"
            release.mkdir()
            schema = {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "additionalProperties": False,
                "$defs": {},
                "properties": {"bad": {"$ref": "#/$defs/missing"}},
            }
            for name in (
                "release-input.schema.json",
                "release-evidence.schema.json",
                "test-evidence.schema.json",
            ):
                (release / name).write_text(json.dumps(schema), encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "unresolved"):
                release_evidence.validate_checked_in_schemas(repo)

    def test_unpinned_container_base_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            docker_directory = repo / "crates/weftext-server/deploy"
            docker_directory.mkdir(parents=True)
            (docker_directory / "Dockerfile").write_text(
                "FROM debian:bookworm-slim\n", encoding="utf-8"
            )
            (docker_directory / "compose.same-host.yaml").write_text(
                "services:\n  server:\n    image: example/server:latest\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(release_evidence.GateError, "non-digest-pinned"):
                release_evidence.validate_docker_policy(repo)

    def test_mutable_container_package_install_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            docker_directory = repo / "crates/weftext-server/deploy"
            docker_directory.mkdir(parents=True)
            (docker_directory / "Dockerfile").write_text(
                "FROM debian@sha256:" + "0" * 64 + "\nRUN apt-get update && apt-get install curl\n",
                encoding="utf-8",
            )
            (docker_directory / "compose.same-host.yaml").write_text(
                "services:\n  server:\n    image: example/server@sha256:" + "1" * 64 + "\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "package-manager"):
                release_evidence.validate_docker_policy(repo)

    def test_floating_dockerfile_frontend_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            docker_directory = repo / "crates/weftext-server/deploy"
            docker_directory.mkdir(parents=True)
            (docker_directory / "Dockerfile").write_text(
                "# syntax=docker/dockerfile:1\n"
                "FROM debian@sha256:" + "0" * 64 + "\n",
                encoding="utf-8",
            )
            (docker_directory / "compose.same-host.yaml").write_text(
                "services:\n  server:\n    image: example/server@sha256:" + "1" * 64 + "\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "Dockerfile frontend"):
                release_evidence.validate_docker_policy(repo)

    def test_reviewed_server_and_proxy_image_templates_are_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            docker_directory = repo / "crates/weftext-server/deploy"
            docker_directory.mkdir(parents=True)
            (docker_directory / "Dockerfile").write_text(
                "FROM scratch\n", encoding="utf-8"
            )
            (docker_directory / "compose.same-host.yaml").write_text(
                "services:\n"
                "  server:\n"
                "    image: ${WEFTEXT_SERVER_IMAGE_REPOSITORY:?required}@sha256:${WEFTEXT_SERVER_IMAGE_DIGEST:?required}\n"
                "  proxy:\n"
                "    image: ${WEFTEXT_PROXY_IMAGE_REPOSITORY:?required}@sha256:${WEFTEXT_PROXY_IMAGE_DIGEST:?required}\n",
                encoding="utf-8",
            )
            release_evidence.validate_docker_policy(repo)

    def test_mismatched_image_template_authorities_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            docker_directory = repo / "crates/weftext-server/deploy"
            docker_directory.mkdir(parents=True)
            (docker_directory / "Dockerfile").write_text(
                "FROM scratch\n", encoding="utf-8"
            )
            (docker_directory / "compose.same-host.yaml").write_text(
                "services:\n"
                "  server:\n"
                "    image: ${WEFTEXT_SERVER_IMAGE_REPOSITORY:?required}@sha256:${WEFTEXT_PROXY_IMAGE_DIGEST:?required}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "not immutable"):
                release_evidence.validate_docker_policy(repo)

    def test_incomplete_packaged_asset_lock_blocks_release(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            lock.parent.mkdir(parents=True)
            lock.write_text(
                json.dumps(
                    {
                        "lockVersion": "weftext.docling-lite-assets.v1",
                        "doclingReleaseCommit": "0" * 40,
                        "completeForExecution": False,
                        "missingForExecution": ["reviewed worker binary"],
                        "artifacts": [],
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "incomplete for release"):
                release_evidence.validate_packaged_asset_locks(repo)

    def test_cyclonedx_records_exact_generator_and_release(self) -> None:
        result = release_evidence.validate_cyclonedx(
            self.fixtures / "cyclonedx-valid.json",
            "0.1.0",
            {"name": "test-sbom", "version": "1.2.3"},
            "fixture",
        )
        self.assertEqual(result, {"name": "test-sbom", "version": "1.2.3"})

    def test_cyclonedx_without_recorded_generator_is_rejected(self) -> None:
        with self.assertRaisesRegex(release_evidence.GateError, "does not record generator"):
            release_evidence.validate_cyclonedx(
                self.fixtures / "cyclonedx-missing-generator.json",
                "0.1.0",
                {"name": "test-sbom", "version": "1.2.3"},
                "fixture",
            )

    def test_cyclonedx_must_cover_every_locked_component(self) -> None:
        with self.assertRaisesRegex(release_evidence.GateError, "omits 1 locked"):
            release_evidence.validate_sbom_coverage(
                self.fixtures / "cyclonedx-valid.json",
                {("serde", "1.0.0"), ("sha2", "0.10.0")},
                "fixture",
            )

    def test_nested_npm_locator_maps_to_actual_package_name(self) -> None:
        self.assertEqual(
            release_evidence.npm_package_name_from_locator(
                "node_modules/parent/node_modules/@scope/child"
            ),
            "@scope/child",
        )

    def test_release_assembly_rejects_wrong_rust_toolchain(self) -> None:
        with (
            mock.patch.object(release_evidence.shutil, "which", return_value="cargo"),
            mock.patch.object(
                release_evidence, "run_checked", return_value="cargo 1.96.0 (fixture)\n"
            ),
        ):
            with self.assertRaisesRegex(release_evidence.GateError, "version mismatch"):
                release_evidence.detect_rust_toolchain("1.98.0", Path.cwd())

    def test_source_manifest_binds_git_file_mode(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            repo = Path(raw_directory)
            subprocess.run(["git", "init", "--quiet"], cwd=repo, check=True)
            script = repo / "verify.sh"
            script.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            subprocess.run(["git", "add", "verify.sh"], cwd=repo, check=True)
            subprocess.run(
                ["git", "update-index", "--chmod=+x", "verify.sh"], cwd=repo, check=True
            )
            manifest = release_evidence.build_source_manifest(repo, "0" * 40)
            self.assertEqual(manifest["files"][0]["gitMode"], "100755")

    def test_structural_sigstore_fixture_is_not_treated_as_crypto_verification(self) -> None:
        # This checks only the prerequisite bundle shape.  verify-release always
        # invokes cosign afterwards and therefore cannot accept this test fixture.
        release_evidence.validate_sigstore_bundle_structure(
            self.fixtures / "sigstore-structure-only.json", "fixture"
        )

    def test_missing_transparency_log_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            path = Path(raw_directory) / "bundle.json"
            bundle = json.loads(
                (self.fixtures / "sigstore-structure-only.json").read_text(encoding="utf-8")
            )
            bundle["verificationMaterial"]["tlogEntries"] = []
            path.write_text(json.dumps(bundle), encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "transparency-log"):
                release_evidence.validate_sigstore_bundle_structure(path, "fixture")

    def test_unfinished_narrative_slot_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            path = Path(raw_directory) / "rollback.txt"
            path.write_text(
                "Rollback ownership and recovery commands are documented here. "
                "TODO verify the drill.",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "placeholder"):
                release_evidence.validate_text_evidence(path, "rollback")

    def test_notice_must_name_every_sbom_component(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            path = Path(raw_directory) / "NOTICE.txt"
            path.write_text(
                "Complete third-party notices include serde@1.0.0 and its terms.\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release_evidence.GateError, "sha2@0.10.0"):
                release_evidence.validate_notice_coverage(
                    path, {("serde", "1.0.0"), ("sha2", "0.10.0")}, "notice"
                )

    def test_structured_test_evidence_binds_layer_version_and_commit(self) -> None:
        release_evidence.validate_test_evidence_document(
            self.fixtures / "test-evidence-valid.json",
            "core-source",
            "0.1.0",
            "0" * 40,
            "fixture",
        )

    def test_manual_acceptance_requires_human_executor(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            path = Path(raw_directory) / "manual.json"
            document = json.loads(
                (self.fixtures / "test-evidence-valid.json").read_text(encoding="utf-8")
            )
            document["layer"] = "manual-accessibility-daily-use"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "human executor"):
                release_evidence.validate_test_evidence_document(
                    path,
                    "manual-accessibility-daily-use",
                    "0.1.0",
                    "0" * 40,
                    "fixture",
                )

    def test_release_input_rejects_missing_backup_slot(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            path = Path(raw_directory) / "input.json"
            document = {
                "schemaVersion": release_evidence.RELEASE_INPUT_SCHEMA,
                "releaseVersion": "0.1.0",
                "sourceCommit": "0" * 40,
                "artifacts": [],
                "sboms": [],
                "noticeFiles": [],
                "testEvidence": [],
                "knownLimitations": {},
                "migrationPath": {},
                "rollbackProcedure": {},
            }
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaisesRegex(release_evidence.GateError, "backupCompatibility"):
                release_evidence.parse_release_input(path, "0.1.0")

    def test_complete_gate_assembles_evidence_only_after_external_verification(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            temporary = Path(raw_directory)
            repo = temporary / "repo"
            release_bundle = temporary / "bundle"
            output = temporary / "verified-output"
            second_output = temporary / "verified-output-repeat"
            repo.mkdir()
            release_bundle.mkdir()
            (repo / "Cargo.lock").write_text("locked fixture\n", encoding="utf-8")
            worker_lock = repo / release_evidence.DOCLING_WORKER_ROOT / "Cargo.lock"
            worker_lock.parent.mkdir(parents=True)
            worker_lock.write_text("isolated worker lock fixture\n", encoding="utf-8")
            asset_lock = repo / "crates/weftext-import/docling-lite-assets.lock.json"
            asset_lock.parent.mkdir(parents=True)
            asset_lock.write_text("{}\n", encoding="utf-8")

            def write_bundle_file(relative: str, content: bytes) -> tuple[str, str]:
                path = release_bundle / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
                return relative, hashlib.sha256(content).hexdigest()

            signature_bytes = (
                self.fixtures / "sigstore-structure-only.json"
            ).read_bytes()
            signature_path, signature_digest = write_bundle_file(
                "signatures/fixture.sigstore.json", signature_bytes
            )
            artifacts = []
            for component in sorted(release_evidence.REQUIRED_ARTIFACT_COMPONENTS):
                artifact_path, artifact_digest = write_bundle_file(
                    f"artifacts/{component}.bin", f"signed {component} fixture\n".encode()
                )
                artifacts.append(
                    {
                        "name": f"weftext-{component}",
                        "component": component,
                        "platform": "fixture-platform",
                        "version": "0.1.0",
                        "signatureBundle": signature_path,
                        "signatureBundleSha256": signature_digest,
                        **(
                            {
                                "ociReference": (
                                    "registry.example.test/weftext/server@sha256:" + "3" * 64
                                )
                            }
                            if component == "server-container"
                            else {
                                "path": artifact_path,
                                "expectedSha256": artifact_digest,
                            }
                        ),
                    }
                )

            sboms = []
            for index, scope in enumerate(sorted(release_evidence.REQUIRED_SUPPLY_CHAIN_SCOPES)):
                generator_name = release_evidence.SBOM_GENERATORS[scope]
                bom = {
                    "bomFormat": "CycloneDX",
                    "specVersion": "1.6",
                    "serialNumber": (
                        f"urn:uuid:123e4567-e89b-42d3-a456-{index:012x}"
                    ),
                    "version": 1,
                    "metadata": {
                        "component": {
                            "bom-ref": f"pkg:generic/weftext-{scope}@0.1.0",
                            "type": "application",
                            "name": f"weftext-{scope}",
                            "version": "0.1.0",
                        },
                        "tools": {
                            "components": [
                                {
                                    "type": "application",
                                    "name": generator_name,
                                    "version": "1.2.3",
                                }
                            ]
                        },
                    },
                    "components": [
                        {
                            "bom-ref": f"pkg:generic/fixture-{scope}@1.0.0",
                            "type": "library",
                            "name": f"fixture-{scope}",
                            "version": "1.0.0",
                        }
                    ],
                }
                sbom_path, sbom_digest = write_bundle_file(
                    f"sbom/{scope}.cdx.json",
                    (json.dumps(bom, sort_keys=True) + "\n").encode(),
                )
                sboms.append(
                    {
                        "scope": scope,
                        "path": sbom_path,
                        "expectedSha256": sbom_digest,
                        "generator": {
                            "name": generator_name,
                            "version": "1.2.3",
                        },
                    }
                )

            notices = []
            for scope in sorted(release_evidence.REQUIRED_SUPPLY_CHAIN_SCOPES):
                notice_path, notice_digest = write_bundle_file(
                    f"notices/{scope}.txt",
                    (
                        f"Third-party notice evidence for {scope}; all dependency "
                        "license texts and attributions were reviewed for this fixture. "
                        f"fixture-{scope}@1.0.0\n"
                    ).encode(),
                )
                notices.append(
                    {
                        "scope": scope,
                        "path": notice_path,
                        "expectedSha256": notice_digest,
                    }
                )

            source_commit = "0" * 40
            tests = []
            for layer in sorted(release_evidence.REQUIRED_TEST_LAYERS):
                test_document = {
                    "schemaVersion": release_evidence.TEST_EVIDENCE_SCHEMA,
                    "layer": layer,
                    "releaseVersion": "0.1.0",
                    "sourceCommit": source_commit,
                    "result": "passed",
                    "startedAt": "2026-08-24T10:00:00Z",
                    "completedAt": "2026-08-24T10:05:00Z",
                    "executor": {
                        "kind": (
                            "human" if layer == "manual-accessibility-daily-use" else "ci"
                        ),
                        "identity": "fixture evidence executor",
                        "environment": "bounded fixture environment",
                    },
                    "summary": (
                        f"Complete fixture summary for the {layer} acceptance layer "
                        "with a passed result bound to this exact source."
                    ),
                    "checks": [
                        {
                            "name": "bounded fixture check",
                            "result": "passed",
                            "details": "The fixture exercises deterministic evidence assembly.",
                        }
                    ],
                }
                test_path, test_digest = write_bundle_file(
                    f"tests/{layer}.json",
                    (json.dumps(test_document, sort_keys=True) + "\n").encode(),
                )
                tests.append(
                    {
                        "layer": layer,
                        "path": test_path,
                        "expectedSha256": test_digest,
                    }
                )

            narrative = {}
            for field in (
                "knownLimitations",
                "migrationPath",
                "backupCompatibility",
                "rollbackProcedure",
            ):
                path, digest = write_bundle_file(
                    f"narrative/{field}.txt",
                    (
                        f"Complete {field} fixture evidence with accountable ownership, "
                        "verified commands, expected results, and recovery boundaries.\n"
                    ).encode(),
                )
                narrative[field] = {"path": path, "expectedSha256": digest}

            release_input = {
                "schemaVersion": release_evidence.RELEASE_INPUT_SCHEMA,
                "releaseVersion": "0.1.0",
                "sourceCommit": source_commit,
                "artifacts": artifacts,
                "sboms": sboms,
                "noticeFiles": notices,
                "testEvidence": tests,
                **narrative,
            }
            input_path = release_bundle / "release-input.json"
            input_path.write_text(json.dumps(release_input), encoding="utf-8")

            license_report = {
                "schemaVersion": release_evidence.LICENSE_REPORT_SCHEMA,
                "ecosystem": "fixture",
                "packages": [{"name": "fixture", "version": "1.0.0"}],
            }
            with (
                mock.patch.object(
                    release_evidence,
                    "validate_source_policy",
                    return_value={
                        "releaseVersion": "0.1.0",
                        "rust": "1.98.0",
                        "node": "22.13.0",
                    },
                ),
                mock.patch.object(
                    release_evidence,
                    "load_release_trust",
                    return_value={
                        "cosignVersion": "3.0.6",
                        "identity": "trusted fixture identity",
                        "issuer": "https://token.actions.githubusercontent.com",
                    },
                ),
                mock.patch.object(
                    release_evidence,
                    "git_release_state",
                    return_value=(source_commit, 1_777_000_000),
                ),
                mock.patch.object(release_evidence, "validate_packaged_asset_locks"),
                mock.patch.object(release_evidence, "detect_rust_toolchain"),
                mock.patch.object(
                    release_evidence,
                    "detect_cosign",
                    return_value=("cosign", "3.0.6"),
                ),
                mock.patch.object(release_evidence, "verify_cosign_bundle") as blob_verifier,
                mock.patch.object(release_evidence, "verify_cosign_oci") as oci_verifier,
                mock.patch.object(
                    release_evidence,
                    "build_source_manifest",
                    return_value={
                        "schemaVersion": release_evidence.SOURCE_MANIFEST_SCHEMA,
                        "sourceCommit": source_commit,
                        "treeSha256": "2" * 64,
                        "files": [],
                    },
                ),
                mock.patch.object(
                    release_evidence, "cargo_license_report", return_value=license_report
                ),
                mock.patch.object(
                    release_evidence, "npm_license_report", return_value=license_report
                ),
                mock.patch.object(
                    release_evidence,
                    "expected_sbom_components",
                    side_effect=lambda _repo, scope: {(f"fixture-{scope}", "1.0.0")},
                ),
                mock.patch.object(
                    release_evidence,
                    "package_input_paths",
                    return_value=[Path("Cargo.lock")],
                ),
                mock.patch.object(release_evidence, "PACKAGE_LOCKS", ()),
            ):
                manifest = release_evidence.verify_release(
                    repo, input_path, output
                )
                repeated_manifest = release_evidence.verify_release(
                    repo, input_path, second_output
                )

            self.assertEqual(blob_verifier.call_count, 2 * (len(artifacts) - 1))
            self.assertEqual(oci_verifier.call_count, 2)
            self.assertTrue(manifest.is_file())
            self.assertEqual(manifest.read_bytes(), repeated_manifest.read_bytes())
            generated = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(generated["sourceCommit"], source_commit)
            self.assertEqual(len(generated["artifacts"]), len(artifacts))
            self.assertRegex(generated["releaseSetSha256"], r"^[0-9a-f]{64}$")
            self.assertTrue((output / "cargo-licenses.json").is_file())
            self.assertTrue((output / "docling-worker-cargo-licenses.json").is_file())
            self.assertTrue((output / "release-input.json").is_file())
            self.assertTrue((output / "sboms/cargo.cdx.json").is_file())
            self.assertTrue(
                (output / f"signature-bundles/{signature_digest}.sigstore.json").is_file()
            )
            release_set_digest = generated.pop("releaseSetSha256")
            self.assertEqual(
                release_set_digest,
                hashlib.sha256(
                    release_evidence.canonical_json_bytes(generated)
                ).hexdigest(),
            )
            self.assertEqual(
                generated["generatedFiles"]["cargoLicensesSha256"],
                hashlib.sha256((output / "cargo-licenses.json").read_bytes()).hexdigest(),
            )
            self.assertEqual(
                generated["generatedFiles"]["doclingWorkerCargoLicensesSha256"],
                hashlib.sha256(
                    (output / "docling-worker-cargo-licenses.json").read_bytes()
                ).hexdigest(),
            )


if __name__ == "__main__":
    unittest.main()
