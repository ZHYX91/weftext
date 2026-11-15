"""Pure ownership guards only; never starts simctl, Xcode or any OS process."""
from pathlib import Path
from contextlib import nullcontext
import errno
import hashlib
import importlib.util
import json
import plistlib
import subprocess
import tempfile
import unittest
from unittest.mock import call, patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("weftext_apple_entry", ROOT / "scripts" / "apple.py")
ENTRY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ENTRY)


class SimulatorOwnershipTests(unittest.TestCase):
    UDID = "A9642E97-FC21-4E4B-B384-DA49AEAC78BF"
    NAME = "Weftext-39b8e279-5736-4da5-b7b4-b8511781b934"

    def device(self, **changes):
        value = {"udid": self.UDID, "name": self.NAME, "deviceTypeIdentifier": ENTRY.SIMULATOR_DEVICE_TYPE,
                 "isAvailable": True, "state": "Shutdown"}
        value.update(changes)
        return value

    def inventory(self, device=None, runtime=None):
        return {"devices": {runtime or ENTRY.SIMULATOR_RUNTIME: [device or self.device()]}}

    def test_actual_original_tuple_required_before_any_owned_action(self):
        self.assertEqual(ENTRY.owned_device(self.inventory(), self.UDID, self.NAME)["state"], "Shutdown")
        for changed in [{"name": "other"}, {"deviceTypeIdentifier": "other"}, {"isAvailable": False},
                        {"deviceTypeIdentifier": None}]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                ENTRY.owned_device(self.inventory(self.device(**changed)), self.UDID, self.NAME)
        with self.assertRaises(ValueError):
            ENTRY.owned_device(self.inventory(runtime="foreign-runtime"), self.UDID, self.NAME)

    def test_duplicate_udid_cannot_be_hidden_in_another_runtime(self):
        inventory = self.inventory()
        inventory["devices"]["foreign-runtime"] = [self.device(name="other")]
        with self.assertRaises(ValueError):
            ENTRY.owned_device(inventory, self.UDID, self.NAME)

    def test_ambiguous_create_reconciles_only_new_exact_original_tuple(self):
        before = {"devices": {ENTRY.SIMULATOR_RUNTIME: []}}
        self.assertEqual(ENTRY.reconcile_created_device(self.inventory(), before, self.NAME), self.UDID)
        with self.assertRaises(ValueError):
            ENTRY.reconcile_created_device(self.inventory(), self.inventory(), self.NAME)
        with self.assertRaises(ValueError):
            ENTRY.reconcile_created_device(before, before, self.NAME)
        ambiguous = self.inventory()
        ambiguous["devices"][ENTRY.SIMULATOR_RUNTIME].append(self.device(udid="F9A74953-0178-493A-A59C-BB6C58A23D28"))
        with self.assertRaises(ValueError):
            ENTRY.reconcile_created_device(ambiguous, before, self.NAME)

    def test_uuid_aliases_are_not_os_identity_receipts(self):
        self.assertEqual(ENTRY.simulator_udid(self.UDID.lower()), self.UDID)
        for alias in ["{" + self.UDID + "}", self.UDID.replace("-", ""), " " + self.UDID, None]:
            with self.subTest(alias=alias), self.assertRaises((ValueError, TypeError)):
                ENTRY.simulator_udid(alias)

    def test_duplicate_inventory_keys_are_not_silently_rebound(self):
        with self.assertRaises(ValueError):
            ENTRY.strict_inventory('{"devices":{},"devices":{"different":[]}}')

    def test_actual_host_architecture_is_not_assumed_from_the_workflow_label(self):
        for architecture in ["arm64", "x86_64"]:
            self.assertEqual(ENTRY.host_architecture(architecture + "\n"), architecture)
        for invalid in ["", "amd64", "arm64 x86_64", "aarch64"]:
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                ENTRY.host_architecture(invalid)


class SimulatorTestEnvironmentTests(unittest.TestCase):
    def test_resolved_tools_must_exist_inside_selected_xcode(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            xcode = root / "Xcode.app/Contents/Developer"
            tool = xcode / "usr/bin/simctl"
            tool.parent.mkdir(parents=True)
            tool.write_bytes(b"TEST tool only")
            self.assertEqual(ENTRY.qualified_tool_path(str(tool) + "\n", xcode, "simctl"), tool.resolve())
            foreign = root / "simctl"
            foreign.write_bytes(b"TEST foreign tool")
            for path, name in [(foreign, "simctl"), ("relative/simctl", "simctl"),
                               (tool, "lipo"), (tool, "unknown"),
                               (xcode / "usr/bin/lipo", "lipo")]:
                with self.subTest(path=path, name=name), self.assertRaises(ValueError):
                    ENTRY.qualified_tool_path(str(path), xcode, name)

    def fixture(self, directory):
        bundle = Path(directory) / "iOS 18.5.simruntime"
        library = bundle / "Contents/Resources/RuntimeRoot/System/Cryptexes/OS/usr/lib/swift/libswiftWebKit.dylib"
        library.parent.mkdir(parents=True)
        library.write_bytes(b"TEST original runtime overlay; not a native qualification")
        runtime = {"identifier": ENTRY.SIMULATOR_RUNTIME, "version": "18.5", "bundlePath": str(bundle)}
        return runtime, library

    def test_uses_original_runtime_and_only_changes_child_test_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime, library = self.fixture(directory)
            inherited = {"DEVELOPER_DIR": "TEST selected Xcode"}
            test_env, evidence = ENTRY.simulator_test_environment(runtime, inherited)
            self.assertEqual(inherited, {"DEVELOPER_DIR": "TEST selected Xcode"})
            self.assertEqual(test_env, {**inherited, "TEST_RUNNER_DYLD_FALLBACK_LIBRARY_PATH": str(library.parent)})
            self.assertEqual(evidence["sha256"], hashlib.sha256(library.read_bytes()).hexdigest())
            self.assertEqual(evidence["bytes"], library.stat().st_size)
            self.assertEqual(evidence["runtimeBundle"], runtime["bundlePath"])

    def test_missing_overlay_does_not_download_or_select_another_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime = {"identifier": ENTRY.SIMULATOR_RUNTIME, "version": "18.5", "bundlePath": directory}
            with self.assertRaisesRegex(ValueError, "no original WebKit"):
                ENTRY.simulator_test_environment(runtime, {})
            self.assertEqual(list(Path(directory).iterdir()), [])

    def test_foreign_runtime_relative_bundle_and_inherited_override_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime, _ = self.fixture(directory)
            for change in [{"identifier": "other"}, {"version": "18.6"},
                           {"bundlePath": "relative.simruntime"}, {"bundlePath": None}]:
                with self.subTest(change=change), self.assertRaises(ValueError):
                    ENTRY.simulator_test_environment({**runtime, **change}, {})
            with self.assertRaisesRegex(ValueError, "inherited"):
                ENTRY.simulator_test_environment(runtime, {"TEST_RUNNER_DYLD_FALLBACK_LIBRARY_PATH": "other"})


class CommandProcess:
    pid = 1243

    def __init__(self, waits):
        self.waits = list(waits)
        self.wait_timeouts = []
        self.returncode = None

    def wait(self, timeout):
        self.wait_timeouts.append(timeout)
        outcome = self.waits.pop(0)
        if isinstance(outcome, BaseException):
            raise outcome
        self.returncode = outcome
        return outcome

    def poll(self):
        return self.returncode


class CommandSupervisionTests(unittest.TestCase):
    def exercise(self, waits, signals, expected=None, clock=None, observation=False):
        process = CommandProcess(waits)
        record = {"commands": []}
        output = b"TEST original command bytes\n"
        argv = ["/bin/ps", "-axo", "pid,ppid,pgid,%cpu,rss,stat,comm"] if observation else ["TEST-owned-command"]
        label, timeout = ("processes-after-boot", 15) if observation else ("runtimes", 60)

        def spawn(argv, **options):
            self.assertEqual(argv, expected_argv)
            self.assertTrue(options["start_new_session"])
            options["stdout"].write(output)
            options["stdout"].flush()
            return process

        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            expected_argv = argv
            def invoke():
                if observation:
                    return ENTRY.observe_processes(run, {}, record, label)
                return ENTRY.run_command(argv, run, label, {}, timeout, record, run)
            with patch.object(ENTRY.subprocess, "Popen", side_effect=spawn), \
                    patch.object(ENTRY.os, "killpg", side_effect=signals, create=True) as group, \
                    patch.object(ENTRY.signal, "SIGKILL", 9, create=True), \
                    patch.object(ENTRY.time, "sleep"), \
                    (patch.object(ENTRY.time, "monotonic", side_effect=clock) if clock is not None else nullcontext()):
                failure = None
                if expected:
                    with self.assertRaises(expected) as raised:
                        invoke()
                    failure = raised.exception
                else:
                    self.assertEqual(invoke(), None if observation else output.decode())
                signals_sent = list(group.call_args_list)
            saved = json.loads((run / "result.json").read_text(encoding="utf-8"))
            self.assertEqual(saved, record)
            self.assertEqual((run / (label + ".log")).read_bytes(), output)
        self.assertEqual(process.wait_timeouts, [timeout, 10])
        return process, record["commands"][0], signals_sent, failure

    def test_deadline_and_denied_signal_keep_original_cause_and_reap_original_leader(self):
        denied = PermissionError(errno.EPERM, "TEST signal denied")
        process, entry, signals, failure = self.exercise(
            [subprocess.TimeoutExpired("TEST-owned-command", 60), 0], [None, denied], RuntimeError)
        self.assertEqual(signals, [call(process.pid, 0), call(process.pid, 9)])
        self.assertEqual(entry["terminationReason"], "deadline")
        self.assertEqual(entry["cleanupErrors"][0]["phase"], "groupSignal")
        self.assertEqual(entry["cleanupErrors"][0]["errno"], errno.EPERM)
        self.assertTrue(entry["ownedLeaderReaped"])
        self.assertFalse(entry["ownedGroupGone"])
        self.assertTrue(entry["cleanupUnknown"])
        self.assertEqual(entry["exitCode"], 0)
        self.assertEqual(entry["state"], "failed")
        self.assertIn("deadline", str(failure))
        self.assertIs(failure.__cause__, denied)

    def test_denied_probe_does_not_signal_or_skip_original_leader_wait(self):
        process, entry, signals, _ = self.exercise(
            [subprocess.TimeoutExpired("TEST-owned-command", 60), 0],
            [PermissionError(errno.EPERM, "TEST probe denied")], RuntimeError)
        self.assertEqual(signals, [call(process.pid, 0)])
        self.assertEqual(entry["terminationReason"], "deadline")
        self.assertEqual(entry["cleanupErrors"][0]["phase"], "groupProbe")
        self.assertTrue(entry["ownedLeaderReaped"])
        self.assertTrue(entry["cleanupUnknown"])
        self.assertEqual(entry["state"], "failed")

    def test_probe_after_signal_error_keeps_deadline_and_does_not_repeat_signal(self):
        process, entry, signals, _ = self.exercise(
            [subprocess.TimeoutExpired("TEST-owned-command", 60), 0],
            [None, None, OSError(errno.EIO, "TEST later probe failed")], RuntimeError,
            clock=[0, 1, 2, 3])
        self.assertEqual(signals, [call(process.pid, 0), call(process.pid, 9), call(process.pid, 0)])
        self.assertEqual(entry["terminationReason"], "deadline")
        self.assertEqual(entry["cleanupErrors"][0]["phase"], "groupProbeAfterSignal")
        self.assertTrue(entry["ownedLeaderReaped"])
        self.assertTrue(entry["cleanupUnknown"])

    def test_leader_wait_failure_records_both_cleanup_errors_without_overwriting_deadline(self):
        _, entry, _, _ = self.exercise(
            [subprocess.TimeoutExpired("TEST-owned-command", 60), subprocess.TimeoutExpired("TEST-owned-command", 10)],
            [None, PermissionError(errno.EPERM, "TEST signal denied")], RuntimeError)
        self.assertEqual(entry["terminationReason"], "deadline")
        self.assertEqual([error["phase"] for error in entry["cleanupErrors"]], ["groupSignal", "leaderWait"])
        self.assertFalse(entry["ownedLeaderReaped"])
        self.assertIsNone(entry["exitCode"])
        self.assertTrue(entry["cleanupUnknown"])
        self.assertEqual(entry["state"], "failed")

    def test_original_command_exception_survives_cleanup_permission_error(self):
        original = ValueError("TEST original command observation failed")
        _, entry, _, failure = self.exercise(
            [original, 7], [PermissionError(errno.EPERM, "TEST probe denied")], ValueError)
        self.assertIs(failure, original)
        self.assertEqual(entry["terminationReason"], "ValueError")
        self.assertEqual(entry["exitCode"], 7)
        self.assertEqual(entry["state"], "failed")

    def test_group_still_unverified_after_bounded_signal_wait_cannot_pass(self):
        process, entry, signals, _ = self.exercise(
            [subprocess.TimeoutExpired("TEST-owned-command", 60), 0],
            [None, None], RuntimeError, clock=[0, 1, 11, 12])
        self.assertEqual(signals, [call(process.pid, 0), call(process.pid, 9)])
        self.assertFalse(entry["ownedGroupGone"])
        self.assertTrue(entry["ownedLeaderReaped"])
        self.assertTrue(entry["cleanupUnknown"])
        self.assertEqual(entry["terminationReason"], "deadline")
        self.assertEqual(entry["state"], "failed")

    def test_known_gone_group_and_reaped_zero_leader_preserve_success(self):
        process, entry, signals, _ = self.exercise([0, 0], [ProcessLookupError()])
        self.assertEqual(signals, [call(process.pid, 0)])
        self.assertTrue(entry["ownedGroupGone"])
        self.assertTrue(entry["ownedLeaderReaped"])
        self.assertFalse(entry["cleanupUnknown"])
        self.assertNotIn("cleanupErrors", entry)
        self.assertEqual(entry["exitCode"], 0)
        self.assertEqual(entry["state"], "passed")

    def test_known_gone_group_cannot_promote_nonzero_command_to_success(self):
        _, entry, _, failure = self.exercise([7, 7], [ProcessLookupError()], RuntimeError)
        self.assertFalse(entry["cleanupUnknown"])
        self.assertEqual(entry["exitCode"], 7)
        self.assertEqual(entry["state"], "failed")
        self.assertIn("exit 7", str(failure))

    def test_process_snapshot_failure_continues_only_after_exact_process_retirement(self):
        for waits, signals in [([0, 0], [ProcessLookupError()]),
                ([7, 7], [ProcessLookupError()]),
                ([subprocess.TimeoutExpired("TEST ps", 15), -9], [None, None, ProcessLookupError()])]:
            with self.subTest(waits=waits):
                _, entry, _, failure = self.exercise(waits, signals, observation=True)
                self.assertIsNone(failure)
                self.assertFalse(entry["cleanupUnknown"])
                self.assertEqual(entry["state"], "passed" if waits[0] == 0 else "failed")
                if isinstance(waits[0], subprocess.TimeoutExpired):
                    self.assertEqual(entry["terminationReason"], "deadline")
                    self.assertEqual(entry["exitCode"], -9)

    def test_process_snapshot_unknown_cleanup_and_cancellation_still_propagate(self):
        self.exercise([subprocess.TimeoutExpired("TEST ps", 15), 0],
                      [PermissionError(errno.EPERM, "TEST probe denied")], RuntimeError, observation=True)
        for interrupted in (KeyboardInterrupt(), SystemExit(2)):
            with self.subTest(interrupted=type(interrupted).__name__):
                _, _, _, failure = self.exercise([interrupted, 0], [ProcessLookupError()],
                                                type(interrupted), observation=True)
                self.assertIs(failure, interrupted)

    def test_process_snapshot_cannot_swallow_unbound_or_incomplete_failure(self):
        exact = {"label": "processes-after-boot", "argv": ["/bin/ps", "-axo", "pid,ppid,pgid,%cpu,rss,stat,comm"],
                 "state": "failed", "cleanupUnknown": False, "ownedGroupGone": True, "ownedLeaderReaped": True}
        for change in ({"label": "simulator-after-boot"}, {"argv": ["xcodebuild"]},
                       {"state": "running"}, {"cleanupUnknown": None},
                       {"ownedGroupGone": False}, {"ownedLeaderReaped": False}):
            with self.subTest(change=change):
                record = {"commands": []}
                def fail(*_args, **_kwargs):
                    record["commands"].append({**exact, **change})
                    raise RuntimeError("TEST unqualified failure")
                with patch.object(ENTRY, "run_command", side_effect=fail), self.assertRaises(RuntimeError):
                    ENTRY.observe_processes(Path("TEST-run"), {}, record, "processes-after-boot")
        record = {"commands": [exact]}
        with patch.object(ENTRY, "run_command", side_effect=RuntimeError("TEST no new record")), \
                self.assertRaises(RuntimeError):
            ENTRY.observe_processes(Path("TEST-run"), {}, record, "processes-after-boot")


class AfterBootSamplingTests(unittest.TestCase):
    def wait_case(self, waits, sample_seconds=0, sample_failure=None):
        now = [0.0]
        process = CommandProcess([])
        calls = []
        outcomes = list(waits)
        def wait(timeout):
            calls.append(timeout)
            outcome, elapsed = outcomes.pop(0)
            now[0] += elapsed
            if isinstance(outcome, BaseException):
                raise outcome
            return outcome
        process.wait = wait
        record = {"commands": [{}]}
        def sample(argv, run, env, observed, label, timeout, cwd, **kwargs):
            self.assertEqual(argv, ["/usr/bin/sample", str(process.pid), "1", "10", "-mayDie"])
            self.assertEqual((label, timeout, kwargs), ("simulator-after-boot-sample", 3, {"retire_by": 30}))
            self.assertIs(observed, record)
            now[0] += sample_seconds
            if sample_failure:
                raise sample_failure
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            with patch.object(ENTRY.time, "monotonic", side_effect=lambda: now[0]), \
                    patch.object(ENTRY, "observe_retired_command", side_effect=sample) as sampled:
                try:
                    result = ENTRY.wait_after_boot(process, ["TEST simctl"], run, {}, 30,
                                                  record, run, record["commands"][0], 0)
                except BaseException as error:
                    result = error
            self.assertEqual(json.loads((run / "result.json").read_text(encoding="utf8")), record)
        return result, calls, sampled.call_count, record["commands"][0]

    def test_fast_command_never_samples_and_slow_command_keeps_absolute_deadline(self):
        result, calls, sampled, entry = self.wait_case([(0, 2)])
        self.assertEqual((result, calls, sampled), (0, [10], 0))
        self.assertNotIn("profiled", entry)
        result, calls, sampled, entry = self.wait_case(
            [(subprocess.TimeoutExpired("TEST simctl", 10), 10), (0, 5)], sample_seconds=2)
        self.assertEqual((result, calls, sampled), (0, [10, 18], 1))
        self.assertTrue(entry["profiled"])
        self.assertNotIn("terminationReason", entry)

    def test_sample_budget_exhaustion_and_interruption_never_grant_command_success(self):
        result, calls, sampled, _ = self.wait_case(
            [(subprocess.TimeoutExpired("TEST simctl", 10), 10)], sample_seconds=20)
        self.assertIsInstance(result, subprocess.TimeoutExpired)
        self.assertEqual((calls, sampled), ([10], 1))
        for error in (RuntimeError("TEST sample cleanup unknown"), KeyboardInterrupt(), SystemExit(2)):
            with self.subTest(error=type(error).__name__):
                result, calls, sampled, _ = self.wait_case(
                    [(subprocess.TimeoutExpired("TEST simctl", 10), 10)], sample_failure=error)
                self.assertIs(result, error)
                self.assertEqual((calls, sampled), ([10], 1))

    def test_unknown_sample_error_still_retires_the_original_simctl(self):
        process = CommandProcess([0])
        record = {"commands": []}
        failure = RuntimeError("TEST sample process cleanup unknown")
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            with patch.object(ENTRY.subprocess, "Popen", return_value=process), \
                    patch.object(ENTRY, "wait_after_boot", side_effect=failure), \
                    patch.object(ENTRY.os, "killpg", side_effect=ProcessLookupError(), create=True) as group:
                with self.assertRaises(RuntimeError) as raised:
                    ENTRY.run_command(["TEST simctl"], run, "simulator-after-boot", {}, 30, record, run)
            self.assertIs(raised.exception, failure)
            self.assertEqual(group.call_args_list, [call(process.pid, 0)])
            self.assertEqual(process.wait_timeouts, [10])
            self.assertEqual(record["commands"][0]["state"], "failed")
            self.assertTrue(record["commands"][0]["ownedLeaderReaped"])

    def test_sample_group_and_leader_cleanup_share_the_outer_deadline(self):
        for deadline in (15.0, 13.01):
            with self.subTest(deadline=deadline):
                now, pauses = [10.0], []
                process = CommandProcess([])
                def wait(timeout):
                    process.wait_timeouts.append(timeout)
                    now[0] += timeout
                    raise subprocess.TimeoutExpired("TEST sample", timeout)
                def sleep(seconds):
                    pauses.append(seconds)
                    now[0] = deadline
                process.wait = wait
                record = {"commands": []}
                with tempfile.TemporaryDirectory() as directory:
                    run = Path(directory)
                    with patch.object(ENTRY.subprocess, "Popen", return_value=process), \
                            patch.object(ENTRY.os, "killpg", return_value=None, create=True), \
                            patch.object(ENTRY.signal, "SIGKILL", 9, create=True), \
                            patch.object(ENTRY.time, "monotonic", side_effect=lambda: now[0]), \
                            patch.object(ENTRY.time, "sleep", side_effect=sleep):
                        with self.assertRaises(RuntimeError):
                            ENTRY.run_command(["TEST sample"], run, "sample", {}, 3, record, run, retire_by=deadline)
                    self.assertEqual(now[0], deadline)
                    self.assertEqual(process.wait_timeouts, [3, 0])
                    self.assertLessEqual(max(pauses), min(0.05, deadline - 13))
                    self.assertTrue(record["commands"][0]["cleanupUnknown"])
                    self.assertEqual(record["commands"][0]["terminationReason"], "deadline")

    def test_small_tool_script_bytes_are_recorded_without_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "simctl"
            raw = b"#!/bin/sh\nexit 73\n"
            path.write_bytes(raw)
            observed = ENTRY.selected_tool_record(path, path.resolve())
            self.assertEqual(observed["kind"], "script")
            self.assertEqual(observed["boundedScriptText"], raw.decode())
            self.assertEqual(observed["sha256"], hashlib.sha256(raw).hexdigest())
            self.assertEqual(observed["fileIdentity"]["bytes"], len(raw))
            self.assertFalse(observed["reportedIsSymlink"])
            path.write_bytes(b"#!" + b"x" * 4096)
            self.assertNotIn("boundedScriptText", ENTRY.selected_tool_record(path, path.resolve()))


class PlatformVersionMetadataTests(unittest.TestCase):
    def test_aliases_share_one_bounded_bundle_read_and_keep_their_mapping(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apps = root / "Applications"
            actual, alias = apps / "Xcode_26.3.app", apps / "Xcode.app"
            (actual / "Contents/Developer/usr/bin").mkdir(parents=True)
            alias.mkdir()
            (actual / "Contents/Info.plist").write_bytes(plistlib.dumps({"CFBundleShortVersionString": "26.3"}))
            (actual / "Contents/Developer/usr/bin/simctl").write_bytes(b'#!/bin/bash\nEXPECTED_VERSION="1051.17.8"\n')
            core = root / "CoreSimulator.framework/Versions/A/Resources/Info.plist"
            core.parent.mkdir(parents=True)
            core.write_bytes(plistlib.dumps({"CFBundleVersion": "1051.17.8"}))
            original_resolve = Path.resolve
            def resolve(path, *args, **kwargs):
                return actual / path.relative_to(alias) if path == alias or alias in path.parents else original_resolve(path, *args, **kwargs)
            record = {}
            with patch.object(Path, "resolve", autospec=True, side_effect=resolve), \
                    patch.object(ENTRY, "metadata_bytes", wraps=ENTRY.metadata_bytes) as reads:
                ENTRY.record_platform_versions(record, apps, core)
            inventory = record["installedXcodeMetadata"]
            self.assertEqual((inventory["candidateCount"], inventory["resolvedCandidateCount"]), (2, 1))
            self.assertEqual(len(inventory["entries"]), 1)
            self.assertEqual(set(inventory["entries"][0]["reportedPaths"]), {str(actual), str(alias)})
            self.assertEqual(reads.call_count, 3)  # Core plist, one Xcode plist, one wrapper.
            self.assertFalse(inventory["truncated"])

    def test_records_real_file_versions_and_comparisons_without_executing_tools(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apps = root / "Applications"
            core = root / "CoreSimulator.framework/Versions/A/Resources/Info.plist"
            core.parent.mkdir(parents=True)
            core.write_bytes(plistlib.dumps({"CFBundleVersion": "1051.17.8"}))
            for name, version, expected in [("Xcode_16.4.app", "16.4", "1010.15"),
                                            ("Xcode-beta.app", "26.3", "1051.17.8")]:
                app = apps / name
                (app / "Contents/Developer/usr/bin").mkdir(parents=True)
                (app / "Contents/Info.plist").write_bytes(plistlib.dumps({"CFBundleShortVersionString": version}))
                (app / "Contents/Developer/usr/bin/simctl").write_text(
                    '#!/bin/bash\nEXPECTED_VERSION="' + expected + '"\nexit 73\n', encoding="utf8")
            record = {"state": "TEST original state"}
            with patch.object(ENTRY.subprocess, "Popen") as spawn:
                ENTRY.record_platform_versions(record, apps, core)
            spawn.assert_not_called()
            self.assertEqual(record["state"], "TEST original state")
            self.assertEqual(record["coreSimulatorMetadata"]["cfBundleVersion"], "1051.17.8")
            self.assertEqual(record["coreSimulatorMetadata"]["source"]["sha256"], hashlib.sha256(core.read_bytes()).hexdigest())
            entries = {Path(entry["reportedPath"]).name: entry for entry in record["installedXcodeMetadata"]["entries"]}
            self.assertFalse(entries["Xcode_16.4.app"]["equalsInstalledCoreSimulator"])
            self.assertTrue(entries["Xcode-beta.app"]["equalsInstalledCoreSimulator"])
            self.assertEqual(entries["Xcode_16.4.app"]["status"], "observed")
            self.assertFalse(record["installedXcodeMetadata"]["selectionChanged"])

    def test_unavailable_or_ambiguous_metadata_never_becomes_a_qualification_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            apps = root / "Applications"
            for number in range(17):
                (apps / f"Xcode_{number}.app").mkdir(parents=True)
            record = {"state": "TEST unchanged"}
            ENTRY.record_platform_versions(record, apps, root / "absent.plist")
            self.assertEqual(record["state"], "TEST unchanged")
            self.assertEqual(record["coreSimulatorMetadata"]["status"], "unavailable")
            self.assertEqual(record["installedXcodeMetadata"]["candidateCount"], 17)
            self.assertEqual(len(record["installedXcodeMetadata"]["entries"]), 16)
            self.assertTrue(record["installedXcodeMetadata"]["truncated"])
            self.assertTrue(all(entry["status"] == "unavailable" for entry in record["installedXcodeMetadata"]["entries"]))
            oversized = root / "large.plist"
            oversized.write_bytes(b"x" * 65537)
            with self.assertRaises(ValueError):
                ENTRY.metadata_bytes(oversized, 65536, root)
            self.assertIsNone(ENTRY.wrapper_expected_version('EXPECTED_VERSION="1.2"\nEXPECTED_VERSION="3.4"'))

    def test_inner_metadata_link_cannot_read_outside_the_original_bundle(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "Xcode.app"
            app.mkdir()
            foreign = root / "outside.txt"
            foreign.write_text("TEST outside content must not be read", encoding="utf8")
            original_resolve = Path.resolve
            for relative in ("Contents/Info.plist", "Contents/Developer/usr/bin/simctl"):
                path = app / relative
                def resolve(current, *args, **kwargs):
                    return foreign if current == path else original_resolve(current, *args, **kwargs)
                with self.subTest(relative=relative), \
                        patch.object(Path, "resolve", autospec=True, side_effect=resolve), \
                        patch.object(Path, "open", autospec=True) as opened:
                    with self.assertRaisesRegex(ValueError, "outside its original bundle"):
                        ENTRY.metadata_bytes(path, 65536, app)
                opened.assert_not_called()


if __name__ == "__main__":
    unittest.main()
