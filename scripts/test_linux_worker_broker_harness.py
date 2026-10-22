"""Unprivileged, bounded I/O/owned-child regressions; starts no product daemon."""
import json
import os
import signal
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

    def collect_fixture(self, action):
        read, write = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(read)
            if action != "silent":
                os.write(write, b"invalid-json" if action == "malformed" else json.dumps({"ok": True}).encode())
                os.close(write)
            if action in ["silent", "hang-after-report"]:
                while True:
                    time.sleep(10)
            os._exit(0)
        os.close(write)
        started = time.monotonic()
        result = harness.collect_client(pid, read, harness.Deadline.after(.8), .2)
        self.assertLess(time.monotonic() - started, 1.5)
        self.assertTrue(result["clientReaped"], result)
        with self.assertRaises(ChildProcessError):
            os.waitpid(pid, os.WNOHANG)
        return result

    def test_honest_client_report_has_actual_exit_status(self):
        result = self.collect_fixture("honest")
        self.assertTrue(result["ok"])
        self.assertEqual(result["clientWaitStatus"], 0)
        self.assertFalse(result["forcedClientCleanup"])

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


if __name__ == "__main__":
    unittest.main()
