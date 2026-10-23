# Linux 受控服务文件后端

此页说明当前 v0.7 源码中的受控服务档位，不能替代同一候选产物的 Core／HTTP／同步／恢复验收。普通 Linux 用户的开放本地写入、Linux 桌面、macOS、Mobile 与真实断电均不由此建立。当前 Core 存储合同为 `weftext.qualified-fs.v2`，与原生导出包版本独立；旧无标签物理证据明确拒绝并保留原件，不自动接纳或重置设备身份。

管理员预先提供专属非登录 service 账号、不同的 client／worker OS 账号、原生 ext4 或 XFS 上 root 管理员拥有的父目录。enrollment 工具在其中 create-new 空服务根目录，客户端此前从未有该 inode 的写入权限；不能把旧目录 chown／改权限后当作已隔离，因为这不能撤销旧 FD 或 mapping。真实资料保原件，经 Core create-only 显式导入新 inode。服务根属于 service UID，权限为 `0700`；现存目录项须完整可枚举、由服务拥有、无其他用户权限，不含符号链接、硬链接、特殊项、非 UTF-8 名或跨挂载。路径字符串、自报 UID、同用户 `flock` 或 `0700` 不能替代不同的 OS 写入权限。

`scripts/linux_service_enrollment.py` 由 64 位 Linux root 管理员运行，只接受预置账号和管理员目录，服务根目录必须尚不存在。`--root`、`--key`、`--output` 为绝对路径；配置和密钥的每层父目录须由 root 拥有且不可由其他用户写入。工具 create-new 空根目录并仅生成随机 enrollment UUID、实际 FSID／opaque handle、当前机器身份摘要和 32 字节认证密钥；已有文件不覆盖，账号、组、ACL 和安全策略不修改。密钥保存在库和私有 profile 之外，root:service-group `0440`；enrollment 为 root `0444`。发生部分失败保留已创建对象供核查，不删除后盲重试。

服务进程须以准确的 real／effective／saved service UID 和 primary GID、空 supplementary groups 和无 capabilities 运行；service 主组不能包含其他账号。worker 同样使用独立非登录账号。客户端和 worker 不能继承服务凭据、密钥或库描述符。当前非 Windows worker 启动器若尚无独立资格，应保持明确 unavailable，不能把未隔离进程当成合格 worker。

轻量 `weftext-enrollment` 是管理员配置和 tagged `PhysicalIdentity` 的唯一实现。Core 与受控 worker broker 共用 `inspect_enrollment` 返回的不可伪造 `ValidatedEnrollment`；检查配置的精确字节摘要、机器、实际本地账号、根目录与父目录 opaque identity 和密钥元数据，inspection 不读取密钥内容、不取得 writer 锁。`check_current` 持续核实配置摘要及原父目录／根目录绑定；管理员替换坐标或改变权限后，已有 FD 不继续授予写入。实际 service UID 的 `read_service_key` 才能取得内存中自动清零的密钥。

Core 的显式入口为：

```rust
let backend = weftext_core::BackendContext::controlled_service(enrollment_path)?;
let core = weftext_core::Core::with_backend(private_data_directory, backend)?;
```

私有目录和作者库均位于同一个已接纳 root 下；调用入口本身不授予 OS 权限。`Core::new` 不在 Linux 自动取得任意本地路径写权。每个真实 root FD 上的独占锁协调不同 data-dir 的实例，两个独立 writer 不能以不同私有目录绕过它。该锁只协调已经由 OS 隔离的服务身份。

目录解析使用 Linux `openat2` BENEATH／NO_SYMLINKS／NO_MAGICLINKS／NO_XDEV；bind mount 也拒绝。文件以实际 FSID 与 opaque handle 定位身份，不用 inode／ctime 回退。`BoundFile` 连续保留 entry、parent 和 root 描述符；placement 只开 `O_PATH`，source／dependency read lease 阻止冲突写入及目录项移动。移动只用 no-replace，私有替换明确使用 replace；文件、源目录、目标目录和原日志决议分别同步。错误／预算截断不返回完整空集合；未知效果保留原 operationId。

设备身份使用带平台标签的提供方。Windows 保留原 DPAPI／SID 机制；Linux 通过标准 HMAC-SHA256 验证实际 machine、enrollment、service UID、controlled-root 和 private-profile 物理绑定。密钥不进入 profile、备份或进程参数内容。复制 profile、换 enrollment／机器或篡改认证数据不能取得原设备资格；已知外平台记录保留原 ID 并返回 `is_current=false`，未知格式明确拒绝，不生成替代 ID。

`backend::platform::tests::controlled_service_core_round_trip_and_negative_boundaries` 为显式 ignored 的部署集成测试，须以真实 service UID 运行，并提供 `WEFTEXT_LINUX_TEST_ENROLLMENT`、`WEFTEXT_LINUX_TEST_ALTERNATE_ENROLLMENT`、`WEFTEXT_LINUX_TEST_BAD_MACHINE_ENROLLMENT` 和位于库外、明确拥有的 `WEFTEXT_LINUX_TEST_QUARANTINE`。测试保留拒绝对象和原字节，不递归清理。root 身份测试不计作服务权限验收；小系统调用检查、WSL 命令行结果和文档模型也不计作其他平台或真实断电验收。

每次初始 guard 都接收所属 Core 的 `BackendContext`；没有 ambient root registry，另一个并存 Core 的接纳不能授权访问其 root。私有 policy、journal、birth、history 和 Draft 读取均保留 bound descriptor，并持续核对同一 enrollment fence。存在性检查区分实际缺失与不支持项、枚举失败或资格撤销。commit record 必须匹配原准确 Plan 和耐久顺序；已提交 Node 的 birth／revision 事实缺失或损坏始终报错，不把受管理源降为普通文件。

原同步应用／目录传输集成测试也在 Linux 使用三份独立 enrollment：`WEFTEXT_LINUX_TEST_PEER_A`、`WEFTEXT_LINUX_TEST_PEER_B`、`WEFTEXT_LINUX_TEST_PEER_C` 均指定管理员登记文件。以真实服务身份、空 supplementary groups、五组 capability 全空、`NoNewPrivs=1`、umask `0077`，并用 `--include-ignored`、`--test-threads=1` 运行测试产物。`WEFTEXT_LINUX_TEST_RETAINED_FIXTURES` 必须是三个 root 之外、明确拥有的私有目录。每个 Core 独立持有其 enrollment lease；测试通过显式 create-only 传输通道复制 portable records。完成或失败的 fixture tree 以 no-replace 移入保留目录，持久记录原路径／目标路径的 planned／completed 回执，不递归删除。

两项独立管理员撤销测试需要全新管理员所有 fixture 配置与有界 harness，由 `WEFTEXT_LINUX_TEST_FENCE_ENROLLMENT`、`WEFTEXT_LINUX_TEST_FENCE_SIGNALS` 指定。普通服务测试用 `--skip administrator_revocation_` 排除它们，再分别用独立全新配置运行准确测试名；只修改该全新配置。已打开 Core 必须拒绝 policy 读取、operation status、缓存 snapshot／style 事实与后续写入，并保留原 source／decision 字节。没有 operation 的 Core 在撤销后也必须拒绝返回空 recovery／catalog；不修改已有 enrollment 或密钥文件。
