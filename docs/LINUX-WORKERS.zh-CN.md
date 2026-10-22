---
source_language: zh-CN
translation_status: source
---

[English](LINUX-WORKERS.md)

# Linux 受控 worker 边界

`weftext-worker-broker` 是单独的管理员启动进程及窄客户端库，只为 Linux 的 `controlledHosted` 服务提供已有 Render / Notes 子进程协议。它不实现第二个 Core、路由器、数据根注册或渲染流水线。普通 `endpointCLI` 仍通过同一已授权服务访问 Core。Linux Desktop、recognition runtime、Office 适配器和整个 v0.7 的可用性均不能由这个机制单独证明。当前源码与局部检查不是安装验收。

Core 与 broker 共同使用 `weftext-enrollment` 的唯一 `inspect_enrollment` 验证器。不可由调用者构造的 `ValidatedEnrollment` 给出真实管理员注册、服务与 worker 的实际 UID/GID、根的原生物理身份及配置摘要；broker 不读取设备认证密钥。注册保留管理员新建空根的资格，不能把既有客户端可写树改属后当作新根。服务与 worker 必须是不同的专属非 root 身份；没有同 UID 回退。

## 固定协议与客户端

管理员策略是有界、拒绝重复和未知字段的 JSON。以下路径是部署模板，实际值及 SHA256 由管理员逐一审核；不能从请求传入路径或替换策略。

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

策略、映像及每级祖先须为 root 所有、没有组/其他写入、符号链接或未限定 POSIX ACL；映像须为单链接、无 set-id / file-capability 的 ELF。broker 边复制边校验 SHA256，再以 `MFD_EXEC` 及 `F_SEAL_WRITE/GROW/SHRINK/SEAL` 封存快照，实际执行同一不可变快照 FD，避免原位更新时校验与执行的内容窗口。内核不支持则 `Unsupported`。worker 的 `current_exe` 因而是 memfd，固定内部分派不能依赖原程序目录发现资源。每次接纳重新核对 Core 注册；策略的摘要和 UUID 必须精确绑定该注册。策略不另设 UID、密钥或数据根权威。

客户端仅能以注册的 Core 服务身份调用：

```rust,ignore
let broker = BrokerClient::connect(&core_enrollment_path, &administrator_policy_path)?;
let mut worker = broker.launch(WorkerKind::Render, Duration::from_secs(20))?;
let stdin = worker.take_stdin();
let stdout = worker.take_stdout();
let stderr = worker.take_stderr();
// 并行写入/排空；完整输入后 flush 并 drop stdin，向 worker 发送 EOF。
let receipt = worker.wait()?; // 失败路径使用 cancel_and_wait()。
```

`WorkerKind::Render` 恰执行 `--render-worker`，`Notes` 恰执行 `--notes-edit-worker`；两者直接进入产品映像的既有内部分派。`Recognition` 返回 typed `Unavailable`，因为尚无管理员批准的 Linux Python/runtime/script 组合。不能请求任意程序、argv、环境、工作目录、URI 或额外描述符。App 的唯一 `owned_process` 适配器持有输入 96 MiB、stdout 24 MiB、stderr 256 KiB 和并发排空预算；broker 的接纳超时最长为 20 秒。

传输采用 `AF_UNIX/SOCK_SEQPACKET`，每帧最多 4096 bytes。首帧恰为 `{version,kind,timeoutMs}`，并携带恰三个独立、方向正确的匿名 pipe FD。常规文件、命名 FIFO、重复 pipe、错误方向、截断辅助数据及额外 FD 均拒绝。broker 用 `SO_PEERCRED` 核对真实连接者的服务 UID/GID；客户端核对 root peer。任务接纳后只接受严格 `{version:1,cancel:true}`，不用 PID 或客户端声称的身份来寻址取消。

## 原子包含与收尾

管理员提前准备独占 cgroup v2 子树并启用 `cpu memory pids`。任何 mkdir 之前，分别检查私有 state entries 与 cgroup 子目录的固定 256 项残留预算；达到预算即停止接纳并要求管理员检查，失败残留也受限。每次任务新建唯一的 root 所有 cgroup；在出生前写入 `memory.max=536870912`、`memory.swap.max=0`、`memory.oom.group=1`、`pids.max=32` 和 `cpu.max=100000 100000`。任务用 `clone3(CLONE_INTO_CGROUP | CLONE_PIDFD | CLONE_NEWNET | CLONE_NEWNS | CLONE_NEWIPC)` 原子出生，不能在进入包含边界前执行不可信产品工作。缺少这个内核能力或必要控制器时失败关闭；不退回 fork 后搬迁、进程组或声称的 PID。

可信子进程设置只执行固定的系统调用：把三个 pipe 置于 stdio，保留 exec FD 和 setup 回执 FD 到 exec，关闭其余 FD；仅在独立 mount namespace 内递归设置根为 readonly/nosuid/nodev，以刚新建的私有目录挂载 64 MiB / 4096 inode、nosuid/nodev/noexec 的 tmpfs 并进入它，避免向宿主 `/tmp` 等位置累计写入或重开宿主设备；清空附加组及 real/effective/saved UID/GID，清空所有 capabilities，启用 `no_new_privs`、禁 core dump 和空环境。独立网络/IPC 命名空间以及 seccomp 禁止创建所有 socket、io_uring 网络旁路、跨进程 FD 获取/ptrace 与命名空间/新旧挂载 API。服务 Core 根和 root 管理密钥继续由操作系统 DAC 隔离；worker 既没有这些路径的读写权，也没有继承 Core/客户端的写 handle 或映射。scratch 随私有 namespace 消亡；收尾只对 held parent/name 与原 inode 匹配的新建空 cwd 做 `AT_REMOVEDIR`，非空或换身份时保留日志和预算占用。它不递归清理内容，不把 cwd 残留误记成进程未回收，也不修改宿主全局 mount。

daemon 持有内核 pidfd 与该任务 cgroup，自己作为 subreaper。自然退出、超时、取消、断开或坏控制帧均进入同一 `cgroup.kill` 及回收路径，包含 fork、`setsid` 和父进程退出后仍运行的后代。仅当真实 pidfd 已退出、`cgroup.events` 的 `populated=0` 且自有子进程已 wait/reap，才生成绑定随机 job ID 的回执，保留真实根进程退出码/信号。内核拒绝收尾时保持所有权并停止接纳后续任务，不发完成回执。

`try_wait`、`wait`、`cancel_and_wait` 只在取得该完整回执后报告完成。daemon 断链、错误回执或未知状态返回 `Unknown`；`Drop` 发送取消通知和断开，不代表已经完成。App 必须保留实际任务收尾 lease，直到 `cancel_and_wait` 回执。正常 SIGTERM/INT/QUIT 会先收尾活动任务。部署宿主仍须以独占的服务 cgroup 和 `KillMode=control-group` 等实际 supervisor 管理 daemon 及其整个子树；SIGKILL、宿主崩溃或断电不能伪造成客户端完成回执。已有 socket 不自动覆盖，仅自动移除这个 daemon 自己绑定且身份仍匹配的 socket。

## 检查与部署边界

普通代码检查使用产品仓库入口及 `cargo test -p weftext-worker-broker`。`boundary-probe` 是显式 feature 下的敌对测试 ELF，绝不作为产品 runtime 或安装包交付。`scripts/test_linux_worker_broker.py` 是需显式 `--run-approved` 的真实 Linux root 机制测试，它不安装组件、创建用户、授予 cgroup 或修改系统安全设置。必须先审核具体源码、映像摘要、策略、原注册及专属 `/etc`、`/var/lib`、cgroup 路径，再单独批准部署和测试命令。

机制测试覆盖严格帧、peer 混淆、FD 注入、映像/摘要/祖先错误、进入包含边界后才执行、fork/setsid/孤儿、超时/取消/断开回收、真实 UID/GID/caps/组/NNP、环境和网络、Core 根/密钥 DAC 及额外写 FD 关闭。实际原始日志、源码/产物 SHA256 和未验项归原任务证据；不把这些局部机制测试记作 Linux Desktop、整个产品或 v0.7 验收。部署仍须验证真实 Render / Notes 产品映像、App 唯一适配器及同一受控 Core 消费链。

`python3 -B scripts/test_linux_worker_broker_harness.py -v` 只检查普通 pipe、poll 和自有 helper 子进程，不启动产品 daemon 或改变 UID。实际部署 harness 的每个 case 共用一个 8 秒绝对截止时间，预留最后 0.5 秒给报告与精确回收；socket、stdin 写入、stdout/stderr 并行排空、result pipe 和 wait 都按剩余时间等待。helper 通过 held pidfd 取消，只有实际 waitpid 状态才记 `clientReaped`。超时、强制收尾、坏报告或未知状态均保留失败/原 bytes，不算 case 完成。合法 service fixture 实际清空五组 capabilities（含 bounding）、附加组并启用 NNP1；root peer 负例保持真实 UID0。daemon 45 秒内无法证明退出时记录 `cleanupUnknown`，保留仍持有自有树的 daemon 供管理员检查。
