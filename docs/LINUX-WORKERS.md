---
source_language: zh-CN
translation_of: LINUX-WORKERS.zh-CN.md
translation_status: synced
---

[简体中文](LINUX-WORKERS.zh-CN.md)

# Controlled Linux worker boundary

`weftext-worker-broker` is a separate administrator-started process and narrow client library for existing Render / Notes subprocess protocols used by Linux `controlledHosted`. It creates no second Core, router, data-root enrollment or rendering pipeline. Ordinary `endpointCLI` still accesses Core through the same authorized service. This mechanism alone establishes no Linux Desktop, recognition runtime, Office adapter or whole-v0.7 availability. Source and targeted checks are not installed-product acceptance.

Core and the broker share the sole `weftext-enrollment::inspect_enrollment` validator. Its sealed `ValidatedEnrollment` supplies the actual administrator enrollment, service/worker UID/GID, native physical root identity and configuration digest. The broker reads no device-authentication key. Enrollment retains the administrator-created empty-root qualification; changing ownership of an existing client-writable tree cannot create that qualification. Service and worker use distinct dedicated non-root identities, with no same-UID fallback.

## Fixed protocol and client

Administrator policy is bounded JSON rejecting duplicate and unknown fields. These paths are deployment templates: the administrator reviews each actual value and SHA256. Requests cannot supply paths or replace policy.

```json
{
  "version": 1,
  "enrollment": "/etc/weftext/enrollment.json",
  "enrollmentSha256": "<64 lowercase hexadecimal characters>",
  "enrollmentId": "<the same Core enrollment UUID>",
  "socket": "/var/lib/weftext-workers/run/broker.sock",
  "stateRoot": "/var/lib/weftext-workers/state",
  "cgroupRoot": "/sys/fs/cgroup/weftext-workers/workers",
  "image": "/usr/libexec/weftext/weftext",
  "imageSha256": "<64 lowercase hexadecimal characters>"
}
```

Policy, image and every ancestor must be root-owned, without group/other writes, symlinks or unqualified POSIX ACLs. The image must be a singly linked ELF without set-id bits or file capabilities. The broker hashes while copying, then seals the snapshot using `MFD_EXEC` and `F_SEAL_WRITE/GROW/SHRINK/SEAL`. Execution uses that same immutable FD, avoiding content changes between verification and exec during an in-place update. Missing kernel support returns `Unsupported`. A worker's `current_exe` therefore refers to a memfd; fixed internal dispatch cannot discover resources relative to the original executable directory. Every admission rechecks Core enrollment; policy digest and UUID bind exactly to that enrollment. Policy creates no independent UID, key or data-root authority.

Only the enrolled Core service identity may use the client:

```rust,ignore
let broker = BrokerClient::connect(&core_enrollment_path, &administrator_policy_path)?;
let mut worker = broker.launch(WorkerKind::Render, Duration::from_secs(20))?;
let stdin = worker.take_stdin();
let stdout = worker.take_stdout();
let stderr = worker.take_stderr();
// Write/drain concurrently; flush and drop stdin after the complete input for EOF.
let receipt = worker.wait()?; // Use cancel_and_wait() on failure paths.
```

`WorkerKind::Render` executes exactly `--render-worker`; `Notes` executes exactly `--notes-edit-worker`. Both enter the existing early internal dispatch of the product image. `Recognition` returns typed `Unavailable`: there is no approved Linux Python/runtime/script combination. Requests cannot select arbitrary programs, argv, environment, cwd, URIs or additional descriptors. App's sole `owned_process` adapter owns the 96 MiB input, 24 MiB stdout, 256 KiB stderr and concurrent-drain budgets. Broker admission allows at most 20 seconds.

Transport uses `AF_UNIX/SOCK_SEQPACKET` with at most 4096 bytes per frame. The first frame is exactly `{version,kind,timeoutMs}`, accompanied by exactly three distinct, correctly directed anonymous pipe FDs. Regular files, named FIFOs, duplicate pipes, wrong directions, truncated ancillary data and extra FDs are rejected. The broker authenticates actual service UID/GID using `SO_PEERCRED`; the client authenticates the root peer. After admission only strict `{version:1,cancel:true}` is accepted. Cancellation is never addressed by a claimed PID or client-supplied identity.

## Atomic containment and completion

The administrator prepares an exclusive cgroup v2 subtree with `cpu memory pids` enabled. Before any mkdir, private state entries and cgroup child directories each have a fixed 256-entry residue budget. Admission stops for administrator inspection at that bound, including failed-setup residue. Each job gets a new unique root-owned cgroup, configured before birth with `memory.max=536870912`, `memory.swap.max=0`, `memory.oom.group=1`, `pids.max=32` and `cpu.max=100000 100000`. `clone3(CLONE_INTO_CGROUP | CLONE_PIDFD | CLONE_NEWNET | CLONE_NEWNS | CLONE_NEWIPC)` creates the worker atomically inside containment. Untrusted product work cannot execute beforehand. Missing kernel features or required controllers fail closed; there is no fork-then-move, process-group or claimed-PID fallback.

Trusted child setup performs fixed syscalls only: place the three pipes in stdio, retain exec/setup receipt FDs until exec, close all other FDs, recursively make root readonly/nosuid/nodev inside the private mount namespace, mount a 64 MiB / 4096-inode nosuid/nodev/noexec tmpfs at the newly created private cwd and enter it, clear supplementary groups, set all real/effective/saved UID/GID values, clear every capability, enable `no_new_privs`, disable core dumps and use an empty environment. This prevents accumulating host writes through `/tmp` or reopening host devices. Private network/IPC namespaces and seccomp deny all new sockets, io_uring bypass, cross-process FD acquisition/ptrace and namespace/old or new mount APIs. OS DAC continues to isolate the service Core root and administrator-managed key. Workers have neither path authority nor inherited Core/client write handles or mappings. Scratch disappears with the private namespace. Cleanup uses `AT_REMOVEDIR` only for the newly created empty cwd whose held parent/name and original inode still match. Nonempty or replaced entries remain logged against the residue budget. It neither recursively removes contents, misreports cwd residue as an unreaped process, nor changes global host mounts.

The daemon retains the actual kernel pidfd and job cgroup and acts as subreaper. Natural exit, timeout, cancel, disconnect and invalid control frames enter the same `cgroup.kill`/reap path, including fork, `setsid` and descendants outliving their root. A receipt bound to a random job ID is produced only after the actual pidfd exits, `cgroup.events` reports `populated=0` and owned children have been waited/reaped. It preserves the real root exit code/signal. If the kernel refuses cleanup, ownership is retained, later admission stops and no completion receipt is sent.

`try_wait`, `wait` and `cancel_and_wait` report completion only with that full receipt. Daemon disconnects, invalid receipts and uncertain state return `Unknown`. `Drop` sends cancellation/disconnect notifications and makes no completion claim. App must retain its actual cleanup lease until `cancel_and_wait` returns the receipt. Normal SIGTERM/INT/QUIT first reap the active job. Deployment must also use an exclusive supervised service cgroup, such as actual `KillMode=control-group` supervision, covering the daemon and its entire subtree. SIGKILL, host failure and power loss cannot become fabricated client completion receipts. Existing sockets are never overwritten; automatic removal is limited to this daemon's own bound socket with matching identity.

## Checks and deployment boundary

Normal source checks use repository entry points and `cargo test -p weftext-worker-broker`. The explicit `boundary-probe` feature builds a hostile test ELF; it is never a product runtime or shipped package. `scripts/test_linux_worker_broker.py` is an actual Linux root mechanism test requiring explicit `--run-approved`. It installs nothing, creates no users, grants no cgroup authority and changes no global security settings. Review concrete source, image digests, policy, original enrollment and exclusive `/etc`, `/var/lib` and cgroup paths before separately authorizing deployment and test commands.

Mechanism tests cover strict frames, peer confusion, FD injection, image/hash/ancestor mismatches, execution only after containment, fork/setsid/orphans, timeout/cancel/disconnect reaping, actual UID/GID/capabilities/groups/NNP, empty environment, network denial, Core-root/key DAC and inherited write-FD closure. Raw logs, source/artifact SHA256 and untested scope belong to the original task evidence. These tests do not establish Linux Desktop, whole-product or v0.7 acceptance. Deployment still needs the actual Render / Notes product image, App's sole adapter and the same controlled Core consumer chain.

`python3 -B scripts/test_linux_worker_broker_harness.py -v` exercises ordinary pipes, poll and owned helper children, without starting the product daemon or changing UID. Each actual deployment-harness case shares one eight-second absolute deadline, reserving its final half-second for reporting and exact reaping. Socket operations, stdin writes, concurrent stdout/stderr drains, result pipes and waits use the remaining time. Helpers are cancelled through held pidfds; only actual waitpid status sets `clientReaped`. Timeouts, forced cleanup, malformed reports and uncertain states preserve failure/raw bytes and never count as case completion. Legitimate service fixtures actually clear all five capability sets, including bounding, clear supplementary groups and enable NNP1; the root-peer negative stays real UID0. If daemon exit cannot be proven within 45 seconds, `cleanupUnknown` is recorded and the daemon retaining its owned tree remains for administrator inspection.
