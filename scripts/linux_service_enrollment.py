#!/usr/bin/env python3
"""Create an administrator-owned Linux service enrollment without replacing data.

Accounts and administrator directories must already be provisioned. This tool
creates a fresh empty service root and never adopts an existing inode/tree.
It does not modify accounts, ACLs or existing records.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import pwd
import grp
import stat
import sys
import uuid


class StatFs(ctypes.Structure):
    _fields_ = [
        ("kind", ctypes.c_long), ("block_size", ctypes.c_long),
        ("blocks", ctypes.c_ulong), ("blocks_free", ctypes.c_ulong),
        ("blocks_available", ctypes.c_ulong), ("files", ctypes.c_ulong),
        ("files_free", ctypes.c_ulong), ("fsid", ctypes.c_int * 2),
        ("name_length", ctypes.c_long), ("fragment_size", ctypes.c_long),
        ("flags", ctypes.c_long), ("spare", ctypes.c_long * 4),
    ]


class FileHandle(ctypes.Structure):
    _fields_ = [("size", ctypes.c_uint), ("kind", ctypes.c_int), ("opaque", ctypes.c_ubyte * 128)]


def open_directory(path: Path, administrator: bool) -> int:
    if not path.is_absolute() or any(part in (".", "..") for part in path.parts):
        raise ValueError("An absolute path without traversal is required")
    current = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        for component in path.parts[1:]:
            child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=current)
            os.close(current)
            current = child
            metadata = os.fstat(current)
            if administrator and (metadata.st_uid != 0 or metadata.st_mode & 0o022):
                raise ValueError("Every administrator directory must be root-owned and not writable by other users")
        return current
    except BaseException:
        os.close(current)
        raise


def inspect_identity(fd: int) -> dict:
    library = ctypes.CDLL(None, use_errno=True)
    filesystem = StatFs()
    if library.fstatfs(fd, ctypes.byref(filesystem)):
        raise OSError(ctypes.get_errno(), "fstatfs failed")
    if filesystem.kind not in (0xEF53, 0x58465342):
        raise ValueError("A native ext4 or XFS filesystem is required")
    handle = FileHandle(size=128)
    mount_id = ctypes.c_int()
    if library.name_to_handle_at(fd, b"", ctypes.byref(handle), ctypes.byref(mount_id), 0x1000):
        raise OSError(ctypes.get_errno(), "name_to_handle_at failed")
    if not 0 < handle.size <= 128:
        raise ValueError("Unsupported opaque handle length")
    return {"platform": "linux", "filesystem": list(filesystem.fsid), "handle_type": handle.kind, "handle": list(handle.opaque[:handle.size])}


def write_new(parent: int, name: str, data: bytes, mode: int, gid: int) -> None:
    descriptor = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o400, dir_fd=parent)
    try:
        if any(name in os.listxattr(descriptor) for name in ("system.posix_acl_access", "system.posix_acl_default")):
            raise ValueError("Inherited extended ACLs are unsupported; the new administrator-only partial record is retained")
        os.fchown(descriptor, 0, gid)
        os.fchmod(descriptor, mode)
        with os.fdopen(descriptor, "wb", closefd=False) as stream:
            stream.write(data)
            stream.flush()
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    os.fsync(parent)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--service-uid", type=int, required=True)
    parser.add_argument("--client-uid", type=int, required=True)
    parser.add_argument("--worker-uid", type=int, required=True)
    parser.add_argument("--key", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    if sys.platform != "linux" or ctypes.sizeof(ctypes.c_void_p) != 8 or os.geteuid() != 0:
        parser.error("Enrollment creation requires a 64-bit Linux root administrator")
    identities = [arguments.service_uid, arguments.client_uid, arguments.worker_uid]
    if min(identities) <= 0 or len(set(identities)) != 3:
        parser.error("Service, client and worker require distinct non-root UIDs")
    accounts = [pwd.getpwuid(uid) for uid in identities]
    service = accounts[0]
    if service.pw_gid == 0 or service.pw_shell not in ("/usr/sbin/nologin", "/sbin/nologin", "/bin/false"):
        parser.error("The service account must be a dedicated non-login account")
    worker = accounts[2]
    if worker.pw_gid == 0 or worker.pw_shell not in ("/usr/sbin/nologin", "/sbin/nologin", "/bin/false"):
        parser.error("The worker account must be a dedicated non-login account")
    if any(account.pw_gid == service.pw_gid and account.pw_uid != service.pw_uid for account in pwd.getpwall()):
        parser.error("The service primary group must exclude every other account")
    if any(member != service.pw_name for group in grp.getgrall() if group.gr_gid == service.pw_gid for member in group.gr_mem):
        parser.error("The service primary group must exclude every other account")
    if arguments.key.is_relative_to(arguments.root) or arguments.output.is_relative_to(arguments.root):
        parser.error("Administrator configuration and device key must be outside the service tree")
    if os.path.lexists(arguments.root) or os.path.lexists(arguments.key) or os.path.lexists(arguments.output):
        parser.error("Existing roots, keys and enrollments are preserved; choose absent explicit destinations")
    parent = open_directory(arguments.root.parent, True)
    key_parent = open_directory(arguments.key.parent, True)
    output_parent = open_directory(arguments.output.parent, True)
    os.mkdir(arguments.root.name, 0o700, dir_fd=parent)
    root = os.open(arguments.root.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
    try:
        metadata = os.fstat(root)
        if metadata.st_uid != 0 or stat.S_IMODE(metadata.st_mode) != 0o700:
            parser.error("The fresh root must remain administrator-owned until enrollment is durable")
        enrollment = {"version": 1, "rootBirth": "administrator-created-empty-v1", "root": str(arguments.root), "identity": inspect_identity(root),
                      "serviceUid": identities[0], "clientUid": identities[1], "workerUid": identities[2],
                      "enrollmentId": str(uuid.uuid4()), "machineId": hashlib.sha256(Path("/etc/machine-id").read_bytes()).hexdigest(), "keyFile": str(arguments.key)}
        write_new(key_parent, arguments.key.name, os.urandom(32), 0o440, service.pw_gid)
        write_new(output_parent, arguments.output.name, (json.dumps(enrollment) + "\n").encode(), 0o444, 0)
        os.fchown(root, service.pw_uid, service.pw_gid)
        os.fchmod(root, 0o700)
        os.fsync(root)
        os.fsync(parent)
        print(json.dumps({"enrollment": str(arguments.output), "key": str(arguments.key), "root": str(arguments.root), "secretPrinted": False}))
    finally:
        for descriptor in (output_parent, key_parent, root, parent):
            os.close(descriptor)


if __name__ == "__main__":
    main()
