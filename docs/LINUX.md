---
source_language: zh-CN
translation_of: LINUX.zh-CN.md
translation_status: synced
---

[简体中文](LINUX.zh-CN.md)

# Linux controlled-service filesystem backend

This describes the controlled-service profile in the current v0.7 source. It does not replace Core/HTTP/sync/recovery acceptance on the same candidate artifact. It establishes no open local Linux writer, Linux desktop, macOS, Mobile or actual power-loss acceptance. The Core storage contract is `weftext.qualified-fs.v2`, independently of the native package version. Old untagged physical evidence is rejected with originals retained, without automatic admission or device-identity reset.

An administrator first provisions a dedicated non-login service account, distinct client/worker OS accounts, a root-administrator-owned parent on native ext4/XFS. Enrollment creates a fresh empty service root: clients have never had write access to those inodes. Chowning or changing permissions on an old tree cannot revoke existing descriptors/mappings. Preserve real originals and explicitly import into new Core-created inodes. The service UID owns its root with mode `0700`. Existing entries must be completely enumerable, service-owned and private, without symbolic/hard links, special files, non-UTF8 names or crossed mounts. Path strings, asserted UIDs, same-user `flock` and `0700` cannot replace separate OS write authority.

Run `scripts/linux_service_enrollment.py` as a 64-bit Linux root administrator against already provisioned accounts/administrator directories; the service root must be absent. `--root`, `--key` and `--output` are absolute paths. Every configuration/key parent must be root-owned and not writable by others. The tool creates a fresh empty root and new records: random enrollment UUID, actual FSID/opaque handle, current machine-identity digest and a 32-byte authentication key. It does not replace existing records or change accounts, groups, ACLs or security policy. Keep the key outside libraries/private profiles with root:service-group `0440`, and enrollment root-owned `0444`. Partial failure retains created objects for inspection instead of deleting and blindly retrying.

Run the service with its exact real/effective/saved service UID and primary GID, empty supplementary groups and no capabilities; the service primary group excludes all other accounts. The worker also uses a separate non-login account. Clients/workers must not inherit service credentials, keys or library descriptors. A non-Windows worker launcher without separate qualification must remain explicitly unavailable.

The small `weftext-enrollment` crate uniquely implements administrator configuration and tagged `PhysicalIdentity`. Core and the controlled worker broker share `inspect_enrollment` and its unforgeable `ValidatedEnrollment`: exact configuration-byte digest, machine, actual local accounts, root/parent opaque identities and key metadata. Inspection reads no key content and takes no writer lock. `check_current` continuously verifies the configuration digest and original parent/root bindings, so retained descriptors do not continue granting writes after administrator coordinate replacement or permission changes. Only the actual service UID can obtain automatically zeroed in-memory key material through `read_service_key`.

The explicit Core entry is:

```rust
let backend = weftext_core::BackendContext::controlled_service(enrollment_path)?;
let core = weftext_core::Core::with_backend(private_data_directory, backend)?;
```

Private data and author libraries reside beneath the same admitted root. Calling this API grants no OS permissions. `Core::new` does not automatically authorize arbitrary local Linux paths. An exclusive lock on the actual root descriptor coordinates instances across different private data directories; separate data directories cannot create two independent writers. The lock coordinates an already OS-isolated service identity.

Resolution uses Linux `openat2` BENEATH/NO_SYMLINKS/NO_MAGICLINKS/NO_XDEV, rejecting bind mounts too. Identity uses actual FSID and opaque handles without inode/ctime fallback. `BoundFile` continuously retains entry, parent and root descriptors. Placement opens only `O_PATH`; source/dependency read leases exclude conflicting writes and namespace movement. Moves never replace; private replacement declares replacement explicitly. File, source directory, destination directory and original journal decisions are separately synchronized. Errors/budget truncation never become complete empty results; unknown effects retain their original operationId.

Device identity has tagged providers. Windows retains DPAPI/SID. Linux uses standard HMAC-SHA256 over actual machine, enrollment, service UID, controlled-root and private-profile physical bindings. Key material never enters profiles, backups or process arguments. Copied profiles, changed enrollment/machine and authentication tampering cannot obtain the original device qualification. A known foreign-platform record keeps its ID and returns `is_current=false`; unknown formats fail without allocating a replacement ID.

`backend::platform::tests::controlled_service_core_round_trip_and_negative_boundaries` is an explicitly ignored deployment integration test. Run it as the actual service UID with `WEFTEXT_LINUX_TEST_ENROLLMENT`, `WEFTEXT_LINUX_TEST_ALTERNATE_ENROLLMENT`, `WEFTEXT_LINUX_TEST_BAD_MACHINE_ENROLLMENT`, and an explicitly owned `WEFTEXT_LINUX_TEST_QUARANTINE` outside the library. Rejected objects/original bytes remain retained, without recursive cleanup. Running as root does not establish service-authority acceptance. Small syscall probes, WSL command-line results and documentation models establish no other-platform or actual power-loss acceptance.

Each initial guard receives its owning Core's `BackendContext`. There is no ambient root registry: another concurrently admitted Core cannot grant access to its root. Private policy, journal, birth, history and Draft reads use bound descriptors and the same continuing enrollment fence. Presence checks distinguish actual absence from unsupported entries, failed enumeration or revoked authority. Commit records require the original exact Plan and durable sequence. Missing or damaged committed Node birth/revision facts remain errors; they cannot turn managed source into an ordinary file.

The existing sync application/transport integration cases also run on Linux using three independently enrolled roots: `WEFTEXT_LINUX_TEST_PEER_A`, `WEFTEXT_LINUX_TEST_PEER_B`, and `WEFTEXT_LINUX_TEST_PEER_C`. These variables name administrator enrollment files. Run the test artifact as the actual service identity with no supplementary groups, all five capability sets empty, `NoNewPrivs=1`, umask `0077`, `--include-ignored` and `--test-threads=1`. Supply `WEFTEXT_LINUX_TEST_RETAINED_FIXTURES` as an owned private directory outside the three roots. Each Core owns its separate enrollment lease; the test's explicit create-only transport channel copies portable records between roots. Finished/failed fixture trees are moved without replacement into the retained directory with durable planned/completed path receipts, never recursively deleted.

The two separate administrator-revocation cases require fresh administrator-owned fixture configurations and bounded harnesses, supplied through `WEFTEXT_LINUX_TEST_FENCE_ENROLLMENT` and `WEFTEXT_LINUX_TEST_FENCE_SIGNALS`. Exclude them from the ordinary service run with `--skip administrator_revocation_`, then run each exact case with its own fresh configuration. Only that fresh configuration changes. An existing Core must reject policy reads, operation status, cached snapshot/style facts and further writes while preserving original source and decision bytes. A Core with no operations must also reject empty recovery/catalog results after revocation. These tests do not modify existing enrollment or key files.
