"""Pure ownership guards only; never starts simctl, Xcode or any OS process."""
from pathlib import Path
from contextlib import nullcontext
import errno
import importlib.util
import json
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
    def exercise(self, waits, signals, expected=None, clock=None):
        process = CommandProcess(waits)
        record = {"commands": []}
        output = b"TEST original command bytes\n"

        def spawn(argv, **options):
            self.assertEqual(argv, ["TEST-owned-command"])
            self.assertTrue(options["start_new_session"])
            options["stdout"].write(output)
            options["stdout"].flush()
            return process

        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            with patch.object(ENTRY.subprocess, "Popen", side_effect=spawn), \
                    patch.object(ENTRY.os, "killpg", side_effect=signals, create=True) as group, \
                    patch.object(ENTRY.signal, "SIGKILL", 9, create=True), \
                    patch.object(ENTRY.time, "sleep"), \
                    (patch.object(ENTRY.time, "monotonic", side_effect=clock) if clock is not None else nullcontext()):
                failure = None
                if expected:
                    with self.assertRaises(expected) as raised:
                        ENTRY.run_command(["TEST-owned-command"], run, "runtimes", {}, 60, record, run)
                    failure = raised.exception
                else:
                    self.assertEqual(ENTRY.run_command(["TEST-owned-command"], run, "runtimes", {}, 60, record, run), output.decode())
                signals_sent = list(group.call_args_list)
            saved = json.loads((run / "result.json").read_text(encoding="utf-8"))
            self.assertEqual(saved, record)
            self.assertEqual((run / "runtimes.log").read_bytes(), output)
        self.assertEqual(process.wait_timeouts, [60, 10])
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


if __name__ == "__main__":
    unittest.main()
