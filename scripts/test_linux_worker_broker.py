"""Explicit, opt-in actual Linux broker boundary test; never installs/configures it.

Requires a separately reviewed root-owned fixture policy/image/cgroup deployment.
This is a mechanism check, not product or whole-v0.7 acceptance.
"""
import argparse
import array
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field {key}")
        result[key] = value
    return result


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
                control=None, disconnect=False):
    """Run the socket client in an independently dropped OS identity."""
    result_read, result_write = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(result_read)
        try:
            os.setgroups([])
            os.setresgid(gid, gid, gid)
            os.setresuid(uid, uid, uid)
            connection = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
            connection.settimeout(8)
            connection.connect(config["socket"])
            pipe_in, input_writer = os.pipe()
            output_reader, pipe_out = os.pipe()
            error_reader, pipe_err = os.pipe()
            passed = [pipe_in, pipe_out, pipe_err]
            if descriptors == "regular":
                passed = [os.open("/etc/passwd", os.O_RDONLY) for _ in range(3)]
            if descriptors == "samePipe":
                passed = [pipe_in, pipe_out, pipe_out]
            if descriptors == "extra":
                passed.append(pipe_err)
            if descriptors == "none":
                passed = []
            rights = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", passed))] if passed else []
            connection.sendmsg([launch], rights)
            for fd in set([pipe_in, pipe_out, pipe_err] + passed):
                os.close(fd)
            try:
                reply = connection.recv(4097)
            except ConnectionResetError:
                reply = b""
            if not reply:
                outcome = {"rejected": True}
            else:
                started = json.loads(reply, object_pairs_hook=unique)
                payload = json.dumps({"action": action, "core": config["core"], "key": config["key"]}).encode()
                os.write(input_writer, payload)
                os.close(input_writer)
                if cancel or control or disconnect:
                    # Ensure the fixture has forked/setsid before cancellation.
                    first = os.read(output_reader, 4096)
                    if disconnect:
                        connection.close()
                    else:
                        connection.send(control or b'{"version":1,"cancel":true}')
                else:
                    first = b""
                raw_receipt = b"" if disconnect else connection.recv(4097)
                receipt = json.loads(raw_receipt, object_pairs_hook=unique) if raw_receipt else None
                chunks = [first]
                while True:
                    chunk = os.read(output_reader, 4096)
                    if not chunk:
                        break
                    chunks.append(chunk)
                errors = os.read(error_reader, 4096).decode(errors="replace")
                outcome = {"started": started, "receipt": receipt,
                           "probe": json.loads(b"".join(chunks)), "stderr": errors}
            os.write(result_write, json.dumps(outcome).encode())
            os._exit(0)
        except BaseException as error:
            os.write(result_write, json.dumps({"error": repr(error)}).encode())
            os._exit(1)
    os.close(result_write)
    data = bytearray()
    while True:
        chunk = os.read(result_read, 65536)
        if not chunk:
            break
        data.extend(chunk)
    os.close(result_read)
    _, status = os.waitpid(pid, 0)
    result = json.loads(data, object_pairs_hook=unique)
    result["clientWaitStatus"] = status
    return result


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
        result = subprocess.run([str(args.broker), str(path)], env={}, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=5, check=False)
        (args.evidence / f"{name}.log").write_bytes(result.stdout)
        assert result.returncode != 0 and message in result.stdout.decode(errors="replace"), result.stdout
        assert not Path(negative["socket"]).exists()
        cases.append({"case": name, "returncode": result.returncode, "stderr": result.stdout.decode(errors="replace")})
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
            assert test("rootPeerConfusion", good, uid=0, gid=0).get("rejected")
            ordinary = test("ordinaryPeer", good, uid=1000, gid=1000)
            assert ordinary.get("rejected") or "PermissionError" in ordinary.get("error", "")
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
            print(f"PASS: {len(cases)} actual broker mechanism cases; no whole-version/product claim")
        finally:
            daemon.send_signal(signal.SIGTERM)
            daemon.wait(timeout=45)
            os.close(sentinel_fd)
            (args.evidence / "identities.json").write_text(json.dumps({
                "brokerSha256": digest(args.broker), "imageSha256": policy["imageSha256"],
                "policySha256": digest(args.policy), "enrollmentSha256": digest(enrollment_path),
                "kernel": os.uname().release, "daemonExit": daemon.returncode,
            }, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
