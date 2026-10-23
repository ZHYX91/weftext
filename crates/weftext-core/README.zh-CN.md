# 本地 Core

当前 Core 由所有客户端共用，承载普通文件／管理笔记的打开、保存、Draft、库存、显式操作、原回执恢复、历史、资源和同步接纳。作者写入只有 `Intent → Plan → OperationJournal → Receipt` 一条路径。当前存储合同为 `weftext.qualified-fs.v2`，与原生导出包版本独立；物理身份和保留元数据均带平台标签。旧无标签物理记录明确拒绝，原字节保留供显式导入／转换，不自动重置设备身份或补造历史连续性。

Windows 本地档位继续使用真实 NTFS 句柄与 share mode：保留父目录闭包，同一 leaf handle 核查对象／原字节、写入／截断、flush、回读和决议。handle-relative no-replace 移动保留原对象；同字节替换不恢复旧 basis。移动源内的目标缺失事实通过原持有目录检查，不按路径重开已受保护的源。Windows 上 `BoundFile` 仍由原 NT entry handle 和已有 guard closure 负责，包括最终父目录的原 canonical witness 交接；不因跨平台抽象保留一个阻碍交接的重复 DELETE 父句柄。

[Linux 受控服务档位](../../docs/LINUX.zh-CN.md) 使用显式 `BackendContext::controlled_service` 与 `Core::with_backend`。管理员创建的全新空 root、不同 OS service/client/worker 身份、私有权限及实际 root FD 锁共同建立边界；同 UID 目录、API 自报和 advisory lock 不能代替 OS 隔离。服务进程核 real/effective/saved UID/GID、空附加组、无 capabilities 和独占 service group。客户端从未获得 root inode 的直接写权限，保留资料只经显式 create-only 导入。Linux `BoundFile` 持续保留 root／parent FD 和准确 entry name；`openat2`、opaque handle／FSID、严格枚举、no-replace 与分别 file／directory fsync 均为实际 Linux 实现，不模拟 Windows 字段或用 inode／ctime 回退。其他未合格平台仍明确 unavailable。

每次初始 guard 均显式接收所属 Core 的 backend；不通过 ambient registration 接纳其他 Core 的 root。私有读取、缓存事实、空目录结果与缺失事实共用 enrollment fence，不把读取失败猜作缺失。原已提交 birth／revision／Plan 事实缺失或损坏仍报错，不把受管理 Node 改判为普通文件。

设备状态属于原物理 private profile。v2 设备记录采用明确的 Windows DPAPI／SID 或 Linux 受控服务 HMAC-SHA256 提供方；Linux 密钥保存在管理员配置树中，不进入 profile、库或备份。复制 profile、换机器／enrollment 或篡改认证数据不能接续原设备身份。已知外平台保留原 ID、返回非当前资格；不认识的格式拒绝且保原件。用户请求的偏好、Draft 和授权状态仍有各自原作用域，不因同设备而合并。

新建库只接受尚不存在的目标；新文件 create-only，目录和文件的实际出生对象持有到原决议。Windows 使用直接返回新目录 handle 的调用；Linux 仅在管理员登记且连续隔离的服务 namespace 内允许 `mkdirat → constrained open` 等价步骤，并同步目录项。源／Schema／Query 读范围阻止相冲突的写入；移动保原对象，复制冻结准确输入并生成新身份。Trash／恢复保持原对象，不永久删除或扩大原确认范围。

普通 `.typ` 文件不注入管理元数据。`A/A.typ` 只有声明 ID 与真实已提交 birth/object binding 匹配才接纳；相同文件名、UUID、marker 或字节不授予历史／写入资格。库存的 observed／partial 与完整 admitted cut 分开；预算或枚举错误不伪装完整空结果。原生源、Schema、资源、作者顺序、标签及导出消费者共用实际类型与物理见证。

Draft 和活动日志位于作者树之外的私有目录，但 Linux 的两者仍在同一已接纳 service root 中。完整临时记录写入并 flush 后再发布，私有替换明确 replace，公开 record 名称 create-only。Linux 同步两个移动目录，文件 flush 不代替目录项耐久。不同 data-dir 不能绕过同一 root 的 writer 协调。失去资格／磁盘满／第三态／未知结果保留原 operationId、Draft 和日志，不分配新请求重发。

副本接收保留稳定的作者引用，但在目标平台重新接纳本地物理对象；历史 Windows 物理证据不转换为 Linux 连续性。完整 owner backup 冻结库和属于该库的原私有恢复材料；设备密钥、机器状态和其他库正文不进入它。内容原生导出与完整恢复 backup 仍为不同产品，不能以导出包重新建立丢失的执行责任。

正式合同见 [CORE](../../docs/design/CORE.zh-CN.md)，实际范围和历史候选资格见 [MVP](../../docs/MVP.zh-CN.md)。本源码中的后端实现与实际产物验收分开；机制检查、WSL 命令行和旧 Windows 结果不建立新候选／Linux 桌面／Mobile／macOS／真实断电验收。尚未隔离的非 Windows 产品 worker 必须明确 unavailable。

检查沿用产品入口 `python scripts/check.py --models`、Rust formatting、`cargo clippy -p weftext-core --all-targets -- -D warnings` 和 `cargo test -p weftext-core`。Linux 的显式部署测试和所需身份／环境变量见 Linux 指南，实际以 service UID 运行；root 测试不计作其权限验收。检查结果归对应源码与产物的原候选记录。
