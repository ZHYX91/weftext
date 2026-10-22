"""Unprivileged, bounded I/O/owned-child regressions; starts no product daemon."""
import errno
import json
import os
import signal
import socket
import subprocess
import sys
import time
import unittest

import test_linux_worker_broker as harness


@unittest.skipUnless(sys.platform == "linux", "Linux pidfd/poll helper tests")
class HarnessDeadlineTests(unittest.TestCase):
    def test_read_without_data_obeys_deadline(self):
        read, write = os.pipe()
        try:
            started = time.monotonic()
            with self.assertRaises(harness.CaseTimeout):
                harness.read_some(read, harness.Deadline.after(.05), "unit blocked read")
            self.assertLess(time.monotonic() - started, .5)
        finally:
            os.close(read)
            os.close(write)

    def test_full_pipe_write_obeys_same_deadline(self):
        read, write = os.pipe()
        try:
            os.set_blocking(write, False)
            while True:
                try:
                    os.write(write, b"x" * 65536)
                except BlockingIOError:
                    break
            with self.assertRaises(harness.CaseTimeout):
                harness.write_all(write, b"blocked", harness.Deadline.after(.05), "unit blocked write")
        finally:
            os.close(read)
            os.close(write)

    def test_stdout_and_stderr_are_drained_together(self):
        out_read, out_write = os.pipe()
        err_read, err_write = os.pipe()
        try:
            os.write(out_write, b"stdout")
            os.write(err_write, b"stderr")
            os.close(out_write)
            os.close(err_write)
            out_write = err_write = None
            out, error = bytearray(), bytearray()
            harness.drain_pipes({out_read: out, err_read: error}, harness.Deadline.after(.5))
            self.assertEqual(out, b"stdout")
            self.assertEqual(error, b"stderr")
        finally:
            for fd in [out_read, out_write, err_read, err_write]:
                if fd is not None:
                    os.close(fd)

    def collect_fixture(self, action, **expected):
        read, write = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(read)
            if action != "silent":
                report = {"ok": True}
                if action in ["permission", "wrong-error"]:
                    report = {"error": "intentional connect failure", "phase": "connect",
                              "clientErrorType": "PermissionError" if action == "permission" else "OSError",
                              "clientErrno": errno.EACCES}
                os.write(write, b"invalid-json" if action == "malformed" else json.dumps(report).encode())
                os.close(write)
            if action in ["silent", "hang-after-report"]:
                while True:
                    time.sleep(10)
            if action == "signal-after-report":
                signal.signal(signal.SIGTERM, signal.SIG_DFL)
                os.kill(os.getpid(), signal.SIGTERM)
                os._exit(99)
            os._exit(23 if action == "nonzero-after-report" else
                     1 if action in ["permission", "wrong-error"] else 0)
        os.close(write)
        started = time.monotonic()
        result = harness.collect_client(pid, read, harness.Deadline.after(.8), .2, **expected)
        self.assertLess(time.monotonic() - started, 1.5)
        self.assertTrue(result["clientReaped"], result)
        with self.assertRaises(ChildProcessError):
            os.waitpid(pid, os.WNOHANG)
        return result

    def test_honest_client_report_has_actual_exit_status(self):
        result = self.collect_fixture("honest")
        self.assertTrue(result["ok"])
        self.assertEqual(result["clientWaitStatus"], 0)
        self.assertTrue(result["clientExitAccepted"])
        self.assertFalse(result["forcedClientCleanup"])

    def test_valid_report_then_nonzero_exit_is_rejected(self):
        result = self.collect_fixture("nonzero-after-report")
        self.assertTrue(result["ok"])
        self.assertEqual(result["clientExitCode"], 23)
        self.assertFalse(result["clientExitAccepted"])
        self.assertIn("error", result)
        self.assertEqual(json.loads(bytes.fromhex(result["rawResultHex"])), {"ok": True})
        self.assertFalse(result["forcedClientCleanup"])

    def test_valid_report_then_signal_exit_is_rejected(self):
        result = self.collect_fixture("signal-after-report")
        self.assertTrue(result["ok"])
        self.assertEqual(result["clientExitCode"], -signal.SIGTERM)
        self.assertFalse(result["clientExitAccepted"])
        self.assertIn("error", result)
        self.assertFalse(result["forcedClientCleanup"])

    def test_expected_permission_failure_requires_exact_error_and_exit(self):
        expected_error = {"clientErrorType": "PermissionError", "clientErrno": errno.EACCES,
                          "phase": "connect"}
        expected = {"expected_exit_code": 1, "expected_error": expected_error}
        accepted = self.collect_fixture("permission", **expected)
        self.assertTrue(accepted["clientExitAccepted"])
        self.assertEqual(accepted["clientExitCode"], 1)
        for action in ["wrong-error", "honest"]:
            with self.subTest(action=action):
                rejected = self.collect_fixture(action, **expected)
                self.assertFalse(rejected["clientExitAccepted"])
                self.assertIn("error", rejected)

    def collect_closed_peer_fixture(self, *, expect_root=False, eof=False):
        read, write = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(read)
            # Transport is real and ordinary. Root metadata is a classifier
            # fixture, never a claim that this test changed OS credentials.
            outcome = {"clientCredentials": {"Uid": ["0"] * 4, "Gid": ["0"] * 4},
                       "simulatedRootCredentials": True, "actualTestUid": os.getuid(),
                       "phase": "started reply" if eof else "launch"}
            connection, peer = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
            peer.close()
            exit_code = 0
            try:
                connection.settimeout(.3)
                if eof:
                    raw = connection.recv(4097)
                    outcome["rejected"] = harness.decode_started(raw, outcome) is None
                else:
                    connection.sendmsg([b'{"version":1,"kind":"render","timeoutMs":1}'])
            except OSError as error:
                outcome.update(error=repr(error), clientErrorType=type(error).__name__, clientErrno=error.errno)
                exit_code = 1
            finally:
                connection.close()
            os.write(write, json.dumps(outcome).encode())
            os.close(write)
            os._exit(exit_code)
        os.close(write)
        result = harness.collect_client(pid, read, harness.Deadline.after(.8), .2,
                                        expect_root_peer_rejection=expect_root)
        self.assertTrue(result["clientReaped"], result)
        self.assertFalse(result["forcedClientCleanup"], result)
        self.assertFalse(result["forcedClientKill"], result)
        with self.assertRaises(ChildProcessError):
            os.waitpid(pid, os.WNOHANG)
        return result

    def test_closed_peer_before_send_is_only_accepted_for_explicit_root_negative(self):
        for expect_root in [True, False]:
            with self.subTest(expect_root=expect_root):
                result = self.collect_closed_peer_fixture(expect_root=expect_root)
                self.assertEqual(result["clientErrorType"], "BrokenPipeError")
                self.assertEqual(result["clientErrno"], errno.EPIPE)
                self.assertEqual(result["clientWaitStatus"], 256)
                self.assertEqual(result["clientExitCode"], 1)
                self.assertEqual(result["clientExitAccepted"], expect_root)
                raw = json.loads(bytes.fromhex(result["rawResultHex"]))
                self.assertEqual(raw["clientErrno"], errno.EPIPE)
                self.assertIn("error", raw)
                self.assertEqual(raw["actualTestUid"], os.getuid())
                self.assertTrue(raw["simulatedRootCredentials"])
                self.assertNotIn("started", result)
                self.assertNotIn("receipt", result)

    def test_root_negative_eof_before_started_preserves_exit_zero_and_raw(self):
        result = self.collect_closed_peer_fixture(expect_root=True, eof=True)
        self.assertTrue(result["rootPeerRejectionAccepted"])
        self.assertEqual(result["clientWaitStatus"], 0)
        self.assertEqual(result["rawStartedHex"], "")
        self.assertTrue(result["rejected"])

    def test_root_close_classifier_rejects_late_wrong_peer_and_started_cases(self):
        report = {"clientCredentials": {"Uid": ["0"] * 4, "Gid": ["0"] * 4},
                  "phase": "launch", "error": "synthetic reset", "clientErrorType": "ConnectionResetError",
                  "clientErrno": errno.ECONNRESET}
        self.assertTrue(harness.root_peer_rejection(report, 1))
        for change, exit_code in [
            ({"phase": "input"}, 1), ({"phase": "receipt"}, 1),
            ({"clientCredentials": {"Uid": ["981"] * 4, "Gid": ["981"] * 4}}, 1),
            ({"clientErrorType": "TimeoutError", "clientErrno": errno.ETIMEDOUT}, 1),
            ({"clientErrno": errno.EACCES}, 1), ({"started": {}}, 1),
            ({"receipt": None}, 1), ({"rawReceiptHex": ""}, 1),
            ({"rawStartedHex": b'{"version":1}'.hex()}, 1), ({}, 23), ({}, -signal.SIGTERM),
        ]:
            with self.subTest(change=change, exit_code=exit_code):
                self.assertFalse(harness.root_peer_rejection(dict(report, **change), exit_code))

    def test_root_negative_expectation_cannot_be_selected_for_service_uid(self):
        with self.assertRaisesRegex(ValueError, "explicit root negative"):
            harness.raw_session({}, b"{}", 981, 981, expect_root_peer_rejection=True)

    def test_result_pipe_timeout_is_failure_and_owned_child_is_reaped(self):
        result = self.collect_fixture("silent")
        self.assertTrue(result["caseTimedOut"])
        self.assertTrue(result["forcedClientKill"])
        self.assertEqual(os.waitstatus_to_exitcode(result["clientWaitStatus"]), -signal.SIGKILL)
        self.assertNotIn("receipt", result)

    def test_reported_but_hung_client_is_not_counted_completed(self):
        result = self.collect_fixture("hang-after-report")
        self.assertTrue(result["caseTimedOut"])
        self.assertTrue(result["forcedClientKill"])
        self.assertIn("error", result)

    def test_malformed_report_keeps_failure_bytes(self):
        result = self.collect_fixture("malformed")
        self.assertIn("error", result)
        self.assertEqual(bytes.fromhex(result["rawResultHex"]), b"invalid-json")
        self.assertTrue(result["forcedClientCleanup"])

    def test_bad_started_reply_preserves_exact_packet_before_decode(self):
        for raw in [b"{bad-json", b"\xff", b"[]", b'{"version":1,"version":1}']:
            with self.subTest(raw=raw):
                outcome = {}
                with self.assertRaises((ValueError, UnicodeDecodeError)):
                    harness.decode_started(raw, outcome)
                self.assertEqual(bytes.fromhex(outcome["rawStartedHex"]), raw)

    def test_valid_started_and_eof_preserve_original_packet(self):
        for raw, expected in [(b'{"version":1}', {"version": 1}), (b"", None)]:
            with self.subTest(raw=raw):
                outcome = {}
                self.assertEqual(harness.decode_started(raw, outcome), expected)
                self.assertEqual(bytes.fromhex(outcome["rawStartedHex"]), raw)


@unittest.skipUnless(sys.platform == "linux", "Linux ordinary subprocess terminal statuses")
class DaemonTerminalStatusTests(unittest.TestCase):
    def stand_in(self, disposition):
        process = subprocess.Popen([sys.executable, "-B", "-c",
                                    "import signal,sys; " + disposition +
                                    "; print('ready', flush=True); signal.pause()"],
                                   stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                   stderr=subprocess.DEVNULL, close_fds=True)
        self.addCleanup(self.reap_stand_in, process)
        self.assertEqual(harness.read_some(process.stdout.fileno(), harness.Deadline.after(1),
                                           "ordinary stand-in ready"), b"ready\n")
        return process

    @staticmethod
    def reap_stand_in(process):
        if process.poll() is None:
            process.kill()
        process.wait(timeout=1)
        if process.stdout is not None:
            process.stdout.close()

    def test_graceful_zero_exit_is_accepted(self):
        process = self.stand_in("signal.signal(signal.SIGTERM, lambda *_:sys.exit(0))")
        self.assertIsNone(harness.stop_daemon(process, timeout=1))
        self.assertEqual(process.returncode, 0)

    def test_actual_nonzero_exit_is_rejected_after_reap(self):
        process = subprocess.Popen([sys.executable, "-B", "-c", "raise SystemExit(9)"],
                                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, close_fds=True)
        self.addCleanup(self.reap_stand_in, process)
        self.assertEqual(process.wait(timeout=1), 9)
        self.assertIsInstance(harness.stop_daemon(process, timeout=1), RuntimeError)
        self.assertEqual(process.returncode, 9)

    def test_actual_signal_exit_is_rejected_after_reap(self):
        process = self.stand_in("signal.signal(signal.SIGTERM, signal.SIG_DFL)")
        self.assertIsInstance(harness.stop_daemon(process, timeout=1), RuntimeError)
        self.assertEqual(process.returncode, -signal.SIGTERM)

    def test_shutdown_timeout_does_not_claim_reaped(self):
        process = self.stand_in("signal.signal(signal.SIGTERM, signal.SIG_IGN)")
        self.assertIsInstance(harness.stop_daemon(process, timeout=.05), subprocess.TimeoutExpired)
        self.assertIsNone(process.returncode)
        self.assertIsNone(process.poll())


if __name__ == "__main__":
    unittest.main()
