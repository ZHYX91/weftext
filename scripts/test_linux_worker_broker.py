"""Explicit, opt-in actual Linux broker boundary test; never installs/configures it.

Requires a separately reviewed root-owned fixture policy/image/cgroup deployment.
This is a mechanism check, not product or whole-v0.7 acceptance.
"""
import argparse
import array
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import signal
import select
import socket
import subprocess
import sys
import time

CASE_SECONDS = 8.0
MAX_STREAM_BYTES = 65536
MAX_RESULT_BYTES = 524288


class CaseTimeout(TimeoutError):
    pass


class Deadline:
    def __init__(self, end):
        self.end = end

    @classmethod
    def after(cls, seconds):
        return cls(time.monotonic() + seconds)

    def remaining(self, stage):
        remaining = self.end - time.monotonic()
        if remaining <= 0:
            raise CaseTimeout(f"case deadline at {stage}")
        return remaining

    def poll(self, descriptors, stage):
        poller = select.poll()
        for fd, events in descriptors.items():
            poller.register(fd, events)
        while True:
            remaining = self.remaining(stage)
            try:
                ready = poller.poll(max(1, int(remaining * 1000)))
            except InterruptedError:
                continue
            if ready:
                return ready


def read_some(fd, deadline, stage):
    os.set_blocking(fd, False)
    while True:
        deadline.poll({fd: select.POLLIN}, stage)
        try:
            return os.read(fd, 65536)
        except BlockingIOError:
            pass


def write_all(fd, data, deadline, stage):
    os.set_blocking(fd, False)
    view = memoryview(data)
    while view:
        deadline.poll({fd: select.POLLOUT}, stage)
        try:
            written = os.write(fd, view)
        except BlockingIOError:
            continue
        if written <= 0:
            raise OSError(f"short write at {stage}")
        view = view[written:]


def drain_pipes(buffers, deadline, limit=MAX_STREAM_BYTES):
    """Drain all owned pipes together, including stderr, under one deadline."""
    pending = dict(buffers)
    for fd in pending:
        os.set_blocking(fd, False)
    while pending:
        for fd, _ in deadline.poll({fd: select.POLLIN for fd in pending}, "pipe drain"):
            try:
                chunk = os.read(fd, min(65536, limit + 1 - len(pending[fd])))
            except BlockingIOError:
                continue
            if not chunk:
                del pending[fd]
                continue
            pending[fd].extend(chunk)
            if len(pending[fd]) > limit:
                raise ValueError("owned pipe output budget exceeded")


class OwnedClient:
    """The helper is this process's unreaped child; pidfd binds kill/wait."""
    def __init__(self, pid):
        self.pid = pid
        self.status = None
        self.pidfd = os.pidfd_open(pid)
        self.kill_sent = False

    def poll_status(self):
        if self.status is None:
            found, status = os.waitpid(self.pid, os.WNOHANG)
            if found == self.pid:
                self.status = status
        return self.status

    def wait(self, deadline):
        while self.status is None:
            deadline.remaining("owned client reap")
            if self.poll_status() is not None:
                break
            deadline.poll({self.pidfd: select.POLLIN}, "owned client reap")
        return self.status

    def kill(self):
        if self.poll_status() is None:
            try:
                signal.pidfd_send_signal(self.pidfd, signal.SIGKILL)
                self.kill_sent = True
            except ProcessLookupError:
                pass

    def close(self):
        os.close(self.pidfd)


def root_peer_rejection(outcome, exit_code):
    """Only root's negative may close before launch; never admit a started job."""
    credentials = outcome.get("clientCredentials", {})
    if credentials.get("Uid") != ["0"] * 4 or credentials.get("Gid") != ["0"] * 4:
        return False
    if any(key in outcome for key in ["started", "receipt", "rawReceiptHex"]):
        return False
    if outcome.get("rawStartedHex", "") != "":
        return False
    if exit_code == 0:
        return (outcome.get("phase") == "started reply" and outcome.get("rejected") is True
                and outcome.get("rawStartedHex") == "" and "error" not in outcome)
    return (exit_code == 1 and outcome.get("phase") in ["launch", "started reply"]
            and "error" in outcome and (outcome.get("clientErrorType"), outcome.get("clientErrno"))
            in [("BrokenPipeError", errno.EPIPE), ("ConnectionResetError", errno.ECONNRESET)])


def collect_client(pid, result_read, deadline, reserve, *, expected_exit_code=0, expected_error=None,
                   expect_root_peer_rejection=False):
    """Reserve the final slice of the same case budget for exact kill/reap."""
    try:
        owned = OwnedClient(pid)
    except OSError as error:
        # This is our direct, unreaped fork result under default SIGCHLD, not a
        # supplied PID. No other code reaps it, so it cannot be reused here.
        os.kill(pid, signal.SIGKILL)
        status = None
        while time.monotonic() < deadline.end:
            found, value = os.waitpid(pid, os.WNOHANG)
            if found == pid:
                status = value
                break
            time.sleep(min(.005, max(0, deadline.end - time.monotonic())))
        os.close(result_read)
        return {"error": f"pidfd unavailable: {error!r}", "clientPid": pid,
                "clientWaitStatus": status, "clientReaped": status is not None,
                "forcedClientKill": True, "cleanupUnknown": status is None}
    raw = bytearray()
    outcome = {}
    failed = False
    try:
        try:
            drain_pipes({result_read: raw}, Deadline(deadline.end - reserve), MAX_RESULT_BYTES)
            outcome = json.loads(raw, object_pairs_hook=unique)
            if not isinstance(outcome, dict):
                raise ValueError("client report must be an object")
        except BaseException as error:
            outcome = {"error": repr(error), "rawResultHex": raw.hex(),
                       "caseTimedOut": isinstance(error, TimeoutError)}
            failed = True
        if failed:
            owned.kill()
        try:
            status = owned.wait(deadline if failed else Deadline(deadline.end - reserve / 2))
        except CaseTimeout as error:
            outcome.update(error=repr(error), caseTimedOut=True)
            failed = True
            owned.kill()
            try:
                status = owned.wait(deadline)
            except CaseTimeout:
                status = owned.poll_status()
        outcome.update(clientPid=pid, clientWaitStatus=status,
                       clientReaped=status is not None, forcedClientKill=owned.kill_sent,
                       forcedClientCleanup=failed,
                       caseDeadlineMonotonic=deadline.end)
        if status is None:
            outcome.update(error="owned helper cleanup unknown after case deadline", cleanupUnknown=True)
        outcome["rawResultHex"] = raw.hex()
        outcome["expectedClientExitCode"] = [0, 1] if expect_root_peer_rejection else expected_exit_code
        outcome["clientExitCode"] = os.waitstatus_to_exitcode(status) if status is not None else None
        accepted = status is not None and os.WIFEXITED(status) and os.WEXITSTATUS(status) == expected_exit_code
        if expected_error is not None:
            accepted = accepted and all(outcome.get(key) == value for key, value in expected_error.items())
        if expect_root_peer_rejection:
            accepted = (status is not None and os.WIFEXITED(status)
                        and root_peer_rejection(outcome, outcome["clientExitCode"]))
            outcome["rootPeerRejectionAccepted"] = accepted
        outcome["clientExitAccepted"] = accepted
        if not accepted:
            if "error" in outcome:
                outcome["priorError"] = outcome["error"]
            outcome["error"] = f"owned helper terminal status rejected: actual={outcome['clientExitCode']}, expected={outcome['expectedClientExitCode']}"
        return outcome
    finally:
        os.close(result_read)
        owned.close()


def drop_client_identity(uid, gid):
    """Only the future explicit root harness calls this trusted fixture setup."""
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    if uid != 0:
        for cap in range(64):
            if libc.prctl(24, cap, 0, 0, 0) < 0 and ctypes.get_errno() != errno.EINVAL:
                raise OSError(ctypes.get_errno(), "PR_CAPBSET_DROP")
        if libc.prctl(47, 4, 0, 0, 0) < 0:
            raise OSError(ctypes.get_errno(), "PR_CAP_AMBIENT_CLEAR_ALL")
    os.setgroups([])
    os.setresgid(gid, gid, gid)
    os.setresuid(uid, uid, uid)
    if uid != 0:
        class Header(ctypes.Structure):
            _fields_ = [("version", ctypes.c_uint32), ("pid", ctypes.c_int)]
        class Data(ctypes.Structure):
            _fields_ = [("effective", ctypes.c_uint32), ("permitted", ctypes.c_uint32),
                        ("inheritable", ctypes.c_uint32)]
        header, data = Header(0x20080522, 0), (Data * 2)()
        libc.capset.argtypes = [ctypes.POINTER(Header), ctypes.POINTER(Data)]
        if libc.capset(ctypes.byref(header), data) < 0 or libc.prctl(38, 1, 0, 0, 0) < 0:
            raise OSError(ctypes.get_errno(), "clear capabilities / NoNewPrivs")
    prefixes = ("Uid:", "Gid:", "Groups:", "Cap", "NoNewPrivs:")
    return {line.split()[0].rstrip(":"): line.split()[1:]
            for line in Path("/proc/self/status").read_text().splitlines() if line.startswith(prefixes)}


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field {key}")
        result[key] = value
    return result


def decode_started(raw, outcome):
    # Capture the actual packet even if UTF-8/JSON/type validation fails.
    outcome["rawStartedHex"] = raw.hex()
    if not raw:
        return None
    started = json.loads(raw, object_pairs_hook=unique)
    require(isinstance(started, dict), "started reply must be an object")
    return started


def stop_daemon(daemon, timeout=45):
    """Reaping and a clean exit are separate obligations; preserve both facts."""
    try:
        daemon.send_signal(signal.SIGTERM)
        code = daemon.wait(timeout=timeout)
    except (subprocess.TimeoutExpired, OSError) as error:
        return error
    if code != 0:
        return RuntimeError(f"daemon terminal status rejected: {code}")
    return None


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def in_directory(path, directory):
    path = Path(path)
    if not path.is_absolute() or path.resolve() != path or not path.is_relative_to(directory):
        raise ValueError(f"unapproved/noncanonical test path: {path}")
    return path


def raw_session(config, launch, uid, gid, *, descriptors="pipes", action="exit", cancel=False,
                control=None, disconnect=False, expected_exit_code=0, expected_error=None,
                expect_root_peer_rejection=False):
    """Run the socket client in an independently dropped OS identity."""
    if expect_root_peer_rejection:
        require((uid, gid) == (0, 0) and expected_exit_code == 0 and expected_error is None,
                "root peer rejection expectation belongs only to the explicit root negative")
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "owned client requires default SIGCHLD/reaping")
    deadline = Deadline.after(CASE_SECONDS)
    reserve = min(.25, CASE_SECONDS / 4)
    io_deadline = Deadline(deadline.end - 2 * reserve)
    report_deadline = Deadline(deadline.end - reserve)
    result_read, result_write = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(result_read)
        fds = set()
        connection = None
        stdout, stderr = bytearray(), bytearray()
        outcome = {"phase": "identity"}
        exit_code = 0

        def close_fd(fd):
            fds.remove(fd)
            os.close(fd)

        try:
            outcome["clientCredentials"] = drop_client_identity(uid, gid)
            outcome["phase"] = "connect"
            connection = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
            connection.settimeout(io_deadline.remaining("connect"))
            connection.connect(config["socket"])
            pipe_in, input_writer = os.pipe()
            output_reader, pipe_out = os.pipe()
            error_reader, pipe_err = os.pipe()
            fds.update([pipe_in, input_writer, output_reader, pipe_out, error_reader, pipe_err])
            passed = [pipe_in, pipe_out, pipe_err]
            if descriptors == "regular":
                passed = [os.open("/etc/passwd", os.O_RDONLY) for _ in range(3)]
                fds.update(passed)
            if descriptors == "samePipe":
                passed = [pipe_in, pipe_out, pipe_out]
            if descriptors == "extra":
                passed.append(pipe_err)
            if descriptors == "none":
                passed = []
            rights = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", passed))] if passed else []
            outcome["phase"] = "launch"
            connection.settimeout(io_deadline.remaining("launch"))
            require(connection.sendmsg([launch], rights) == len(launch), "short launch frame")
            for fd in set([pipe_in, pipe_out, pipe_err] + passed):
                close_fd(fd)
            try:
                outcome["phase"] = "started reply"
                connection.settimeout(io_deadline.remaining("started reply"))
                reply = connection.recv(4097)
            except ConnectionResetError as error:
                outcome.update(startedReadError=repr(error), startedReadErrno=error.errno)
                reply = b""
            started = decode_started(reply, outcome)
            if started is None:
                outcome["rejected"] = True
            else:
                outcome.update(started=started, phase="input")
                payload = json.dumps({"action": action, "core": config["core"], "key": config["key"]}).encode()
                write_all(input_writer, payload, io_deadline, "worker stdin")
                close_fd(input_writer)
                if cancel or control or disconnect:
                    # Ensure the fixture has forked/setsid before cancellation.
                    outcome["phase"] = "fixture setsid barrier"
                    stdout.extend(read_some(output_reader, io_deadline, "fixture setsid barrier"))
                    require(bool(stdout), "fixture exited before cancellation barrier")
                    if disconnect:
                        connection.close()
                    else:
                        connection.settimeout(io_deadline.remaining("cancel"))
                        frame = control or b'{"version":1,"cancel":true}'
                        require(connection.send(frame) == len(frame), "short cancel frame")
                outcome["phase"] = "receipt"
                if not disconnect:
                    connection.settimeout(io_deadline.remaining("receipt"))
                raw_receipt = b"" if disconnect else connection.recv(4097)
                outcome["rawReceiptHex"] = raw_receipt.hex()
                receipt = json.loads(raw_receipt, object_pairs_hook=unique) if raw_receipt else None
                outcome.update(receipt=receipt, phase="stdout/stderr drain")
                drain_pipes({output_reader: stdout, error_reader: stderr}, io_deadline)
                outcome.update(probe=json.loads(stdout), stderr=stderr.decode(errors="replace"))
        except BaseException as error:
            outcome.update(error=repr(error), caseTimedOut=isinstance(error, TimeoutError),
                           clientErrorType=type(error).__name__, clientErrno=getattr(error, "errno", None),
                           stdoutHex=stdout.hex(), stderrHex=stderr.hex())
            exit_code = 1
        finally:
            if connection is not None:
                connection.close()
            for fd in fds:
                os.close(fd)
        try:
            write_all(result_write, json.dumps(outcome).encode(), report_deadline, "client report")
        except BaseException:
            exit_code = 1
        os.close(result_write)
        os._exit(exit_code)
    os.close(result_write)
    return collect_client(pid, result_read, deadline, reserve,
                          expected_exit_code=expected_exit_code, expected_error=expected_error,
                          expect_root_peer_rejection=expect_root_peer_rejection)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-approved", action="store_true")
    parser.add_argument("--policy", type=Path, required=True)
    parser.add_argument("--broker", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    if not args.run_approved or sys.platform != "linux" or os.getuid() != 0:
        parser.error("requires explicit --run-approved on the reviewed Linux root deployment")
    config_root = Path("/etc/weftext-test/v07-20261022")
    data_root = Path("/var/lib/weftext-test/v07-20261022")
    deployment_root = data_root / "worker-broker"
    require(in_directory(args.policy, config_root) == config_root / "worker-broker-fixture.json", "unexpected policy role path")
    require(in_directory(args.broker, deployment_root) == deployment_root / "bin/weftext-worker-broker", "unexpected broker role path")
    in_directory(args.evidence, deployment_root)
    require(args.evidence.parent == deployment_root and args.evidence.name.startswith("evidence-"), "evidence must be a new exclusive child")
    policy = json.loads(args.policy.read_bytes(), object_pairs_hook=unique)
    enrollment_path = in_directory(policy["enrollment"], config_root)
    require(enrollment_path == config_root / "enrollment.json", "unexpected Core enrollment path")
    enrollment = json.loads(enrollment_path.read_bytes(), object_pairs_hook=unique)
    require(digest(enrollment_path) == policy["enrollmentSha256"], "Core enrollment digest mismatch")
    require(enrollment["enrollmentId"] == policy["enrollmentId"], "Core enrollment ID mismatch")
    require([enrollment[key] for key in ["serviceUid", "workerUid", "clientUid"]] == [981, 982, 1000], "unexpected test principals")
    require(Path(enrollment["root"]) == data_root / "service-root", "unexpected Core root")
    require(Path(enrollment["keyFile"]) == config_root / "device-authentication.key", "unexpected Core key")
    require(in_directory(policy["image"], deployment_root) == deployment_root / "bin/weftext-worker-boundary-probe", "unexpected fixture role image")
    require(in_directory(policy["stateRoot"], deployment_root) == deployment_root / "state", "unexpected private state root")
    require(in_directory(policy["socket"], deployment_root) == deployment_root / "run/broker.sock", "unexpected fixture socket")
    require(Path(policy["cgroupRoot"]) == Path("/sys/fs/cgroup/weftext-test-v07-20261022/workers"), "unexpected owned workers cgroup")
    require(digest(Path(policy["image"])) == policy["imageSha256"], "fixture image digest mismatch")
    require(not Path(policy["socket"]).exists(), "refusing an existing daemon/socket")
    require(not any(p.is_dir() for p in Path(policy["cgroupRoot"]).iterdir()), "refusing prior cgroup child residue")
    args.evidence.mkdir(mode=0o700, parents=False, exist_ok=False)
    config = dict(policy, core=enrollment["root"], key=enrollment["keyFile"])
    cases = []
    # Administrator-owned negative fixtures only; never edit the qualified image,
    # Core root, key, original enrollment or original broker policy.
    bad_parent = args.evidence / "writable-parent"
    bad_parent.mkdir(mode=0o777)
    bad_parent.chmod(0o777)
    bad_image = bad_parent / "image"
    bad_image.write_bytes(Path(policy["image"]).read_bytes())
    bad_image.chmod(0o755)
    link_image = args.evidence / "symlink-image"
    link_image.symlink_to(policy["image"])
    for name, changes, message in [
        ("imageHash", {"imageSha256": "0" * 64}, "image type/hash mismatch"),
        ("imageAncestor", {"image": str(bad_image)}, "administrator object/ancestor"),
        ("imageSymlink", {"image": str(link_image)}, "administrator object/ancestor"),
        ("enrollmentBinding", {"enrollmentSha256": "0" * 64}, "policy binding differs"),
    ]:
        negative = dict(policy, **changes)
        negative["socket"] = str(Path(policy["socket"]).with_name(f"{name}.sock"))
        path = args.evidence / f"{name}.json"
        path.write_text(json.dumps(negative), encoding="utf-8")
        negative_log = args.evidence / f"{name}.log"
        with negative_log.open("wb") as log:
            process = subprocess.Popen([str(args.broker), str(path)], env={}, stdin=subprocess.DEVNULL,
                                       stdout=log, stderr=subprocess.STDOUT, close_fds=True)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                try:
                    process.wait(timeout=1)
                finally:
                    (args.evidence / f"{name}-timeout.json").write_text(json.dumps({
                        "failed": True, "pid": process.pid, "reaped": process.returncode is not None,
                        "waitStatus": process.returncode, "cleanupUnknown": process.returncode is None}), encoding="utf-8")
                raise RuntimeError(f"negative startup case timed out: {name}; retain log")
        raw_log = negative_log.read_bytes()
        assert process.returncode != 0 and message in raw_log.decode(errors="replace"), raw_log
        assert not Path(negative["socket"]).exists()
        cases.append({"case": name, "returncode": process.returncode, "stderr": raw_log.decode(errors="replace")})
    sentinel = args.evidence / "inherited-write-handle"
    sentinel.write_bytes(b"original sentinel; inherited descriptor must be closed\n")
    sentinel.chmod(0o600)
    sentinel_fd = os.open(sentinel, os.O_RDWR)
    sentinel_before = digest(sentinel)
    with (args.evidence / "daemon.log").open("wb") as log:
        daemon = subprocess.Popen([str(args.broker), str(args.policy)], stdin=subprocess.DEVNULL,
                                  stdout=log, stderr=subprocess.STDOUT, close_fds=True,
                                  pass_fds=(sentinel_fd,), env={}, cwd=str(data_root))
        try:
            deadline = time.monotonic() + 10
            while not Path(policy["socket"]).exists():
                if daemon.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("broker did not start; retain daemon.log")
                time.sleep(.02)

            def test(name, request, uid=981, gid=981, **kwargs):
                result = raw_session(config, request, uid, gid, **kwargs)
                cases.append({"case": name, "result": result})
                (args.evidence / "cases.json").write_text(json.dumps(cases, indent=2), encoding="utf-8")
                require(result.get("clientReaped"), f"owned client cleanup unknown: {name}")
                require(result.get("clientExitAccepted"), f"owned client terminal status rejected: {name}")
                require(not result.get("forcedClientCleanup") and not result.get("caseTimedOut"),
                        f"case deadline/forced helper cleanup is failure: {name}")
                credentials = result.get("clientCredentials", {})
                require(set(credentials.get("Uid", [])) == {str(uid)}, "client UID was not actually dropped")
                require(set(credentials.get("Gid", [])) == {str(gid)}, "client GID was not actually dropped")
                if uid == 981:
                    require(credentials.get("NoNewPrivs") == ["1"] and credentials.get("Groups") == [], "service fixture privilege state")
                    for cap in ["CapInh", "CapPrm", "CapEff", "CapAmb", "CapBnd"]:
                        require(len(credentials.get(cap, [])) == 1 and int(credentials[cap][0], 16) == 0,
                                f"service fixture {cap} is not zero")
                return result

            def launch(action="exit", timeout=2000, **kwargs):
                return test(action, json.dumps({"version": 1, "kind": "render", "timeoutMs": timeout}).encode(), action=action, **kwargs)

            for name, request in [
                ("duplicate", b'{"version":1,"version":1,"kind":"render","timeoutMs":1}'),
                ("unknown", b'{"version":1,"kind":"render","timeoutMs":1,"exe":"/bin/sh"}'),
                ("malformed", b'{'),
                ("oversize", b'x' * 4097),
                ("unknownKind", b'{"version":1,"kind":"shell","timeoutMs":1}'),
                ("recognitionUnavailable", b'{"version":1,"kind":"recognition","timeoutMs":1}'),
            ]:
                assert test(name, request).get("rejected"), name
            good = b'{"version":1,"kind":"render","timeoutMs":1000}'
            for fds in ["regular", "samePipe", "none", "extra"]:
                assert test(f"fd-{fds}", good, descriptors=fds).get("rejected"), fds
            assert test("rootPeerConfusion", good, uid=0, gid=0,
                        expect_root_peer_rejection=True).get("rootPeerRejectionAccepted")
            ordinary = test("ordinaryPeer", good, uid=1000, gid=1000, expected_exit_code=1,
                            expected_error={"clientErrorType": "PermissionError", "clientErrno": errno.EACCES, "phase": "connect"})
            assert ordinary["clientExitCode"] == 1
            for action, timeout, cancel in [("exit", 2000, False), ("orphan", 2000, False),
                                            ("timeout", 200, False), ("cancel", 2000, True),
                                            ("clone-orphan", 2000, False), ("orphan", 2000, False), ("orphan", 2000, False)]:
                result = launch(action, timeout, cancel=cancel)
                assert "error" not in result, result
                start, receipt, probe = result["started"], result["receipt"], result["probe"]
                assert start["jobId"] == receipt["jobId"] and start["pid"] == probe["pid"]
                assert start["jobId"] in probe["cgroup"], "execution occurred before containment"
                assert receipt["cgroupEmpty"] and receipt["pidfdExited"]
                assert probe["coreDenied"] and probe["keyDenied"] and probe["networkDenied"]
                assert probe["inherited"] == [] and probe["environmentCount"] == 0
                assert probe["readonlyRoot"] and probe["scratchWritable"] and probe["scratchBounded"]
                expected = "timeout" if action == "timeout" else "cancelled" if cancel else "exited"
                assert receipt["completion"] == expected
                if action in ["exit", "orphan", "clone-orphan"]:
                    assert receipt["exitCode"] == 23 and receipt["signal"] is None
                assert receipt["reaped"] >= (2 if action != "exit" else 1)
                assert not Path(policy["cgroupRoot"], start["jobId"]).exists()
                assert not Path(policy["stateRoot"], start["jobId"]).exists()
                for line in probe["status"]:
                    key, *values = line.split()
                    if key in ["Uid:", "Gid:"]:
                        assert set(values) == {"982"}, line
                    if key.startswith("Cap"):
                        assert int(values[0], 16) == 0, line
                    if key == "Groups:":
                        assert not values, line
                    if key == "NoNewPrivs:":
                        assert values == ["1"], line
            assert digest(sentinel) == sentinel_before
            for name, kwargs in [
                ("cancelDuplicate", {"control": b'{"version":1,"cancel":true,"cancel":true}'}),
                ("cancelUnknown", {"control": b'{"version":1,"cancel":true,"pid":1}'}),
                ("dropDisconnect", {"disconnect": True}),
            ]:
                result = test(name, good, action="cancel", **kwargs)
                assert result["receipt"] is None, "disconnect/malformed control cannot become a completion receipt"
                assert not Path(policy["cgroupRoot"], result["started"]["jobId"]).exists()
                assert not Path(policy["stateRoot"], result["started"]["jobId"]).exists()
                assert not Path("/proc", str(result["probe"]["pid"])).exists()
                assert not Path("/proc", str(result["probe"]["child"])).exists()
            job_ids = [case["result"]["started"]["jobId"] for case in cases
                       if "started" in case.get("result", {})]
            assert len(job_ids) == len(set(job_ids)), "job identity reused"
        finally:
            shutdown_error = None
            try:
                shutdown_error = stop_daemon(daemon)
            finally:
                os.close(sentinel_fd)
                (args.evidence / "identities.json").write_text(json.dumps({
                    "brokerSha256": digest(args.broker), "imageSha256": policy["imageSha256"],
                    "policySha256": digest(args.policy), "enrollmentSha256": digest(enrollment_path),
                    "kernel": os.uname().release, "daemonPid": daemon.pid, "daemonExit": daemon.returncode,
                    "daemonReaped": daemon.returncode is not None,
                    "daemonExitAccepted": shutdown_error is None and daemon.returncode == 0,
                    "cleanupUnknown": daemon.returncode is None, "shutdownError": repr(shutdown_error),
                }, indent=2), encoding="utf-8")
            if shutdown_error is not None:
                raise RuntimeError("daemon did not exit cleanly; retain identities.json and owned deployment") from shutdown_error
    print(f"PASS: {len(cases)} actual broker mechanism cases; no whole-version/product claim")


if __name__ == "__main__":
    main()
