---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# 本地 Core

v0.3/v0.4 扩展保留已有 `weftext.local-mvp.v1` 库及完整私有日志，不重置实际资料。`prepare_operation` 冻结请求、随机 PlanId、完整原像/目标字节、真实文件/目录对象及预览；`execute_operation(operationId, planId)` 只执行原计划。同 ID 改输入拒绝；已提交返回原回执，进入安装后未知结果只查询和人工保留，不重新执行。准备本身不安装用户文件。

已实现同卷受管节点改名、移动、应用回收与恢复。目录 namespace 使用去重的 root→leaf 句柄闭包，源目录首次即申请 DELETE，保 no-share-delete 到决议；通过实际句柄 no-replace 改名。节点改名先改 canonical leaf，再移动外目录，保留原 NodeId 和物理文件身份；中间故障保完整计划并屏障两侧路径。回收将原目录移入 `.weftext-trash/<operationId>`，不删除字节；恢复目标占用时冲突。回收源在 Core 中只读。

`read_snapshot` 同时持有有限选中源/config/resource 的全部 share-zero 句柄，返回当前 opaque baseId。后续写入必须再次匹配实际对象、stamp、原字节和父对象；同字节替换不能复活旧依据。`UpdateFiles` 将源与独立批注等成员放入一个原日志、一个提交决议；半写时全部相关成员进入 reader barrier。普通外部工具仍可能看到部分安装，不能把这一逻辑原子性称为文件系统原子替换。

`admitted_snapshot` 的成员域是此唯一 Core 私有日志中的已提交出生事实，扣除当前实际拥有的回收树；全部选中笔记与 schema 的句柄在冻结期间共存。缺失、重复、排除或损坏的已纳管成员使 `complete=false`。原始文件发现的 `rawComplete/rawPending` 独立返回；新 raw 文件不凭 metadata 自动加入已纳管笔记快照。`prepare_admitted_operation` 将完整快照成员及源/schema 原依据作为同一动作依赖，执行前重新核对成员和实际文件，不将 Query 结果变成写权限。

库 schema/settings 使用真实 config role、准确原字节和同一操作日志；样式是实际 `Styles/Styles.typ` 节点拥有的 `library-style.typ` Resource，settings 只选其稳定 ResourceRef。首次样式应用以一个决议创建节点、module、资源 manifest 和设置；样式控件/原生源码共享该文件。资源导入 create-only，manifest 和资源字节同事务，随机 ResourceId 必须匹配私有已提交出生对象；外来 manifest UUID 不自动接纳。

复制固定成员、实际当前 schema（或明确缺省的 bundled schema）、after blobs 和 fresh Node/Resource/Annotation/definition 映射，调用 Knowledge 纯规划器改写官方 AST 及 schema 可证明的类型引用、current/pinned 引用，保留普通文件字节。闭包不可证明时默认拒绝；显式 `allowLosses` 保留原始副本和准确损失。已注册模板实例使用相同受管子树复制规划器，改写自身引用与资源身份，并将准确源/config/resource 依据冻结在原计划。新决议记录准确 after-stamp 与随机 retained revision；实际旧日志没有此证据时明确观察状态，独立批注可在原事务内显式接纳当前源依据。

备份固定所选完整库树（包括 ignored 普通字节与回收内容）及相关原始 journal、Draft、草稿历史和移动消费标记；不带出共享私有目录中其他库或普通文件的记录。目标 create-only，`backup.json` 记录版本、准确摘要与恢复边界。活动 singleton 锁和备份自身新操作不属于输入截面。预算为 5000 文件、64 MiB，超过或遇到不可读成员整体失败。新物理目录/设备仍需显式重新绑定；没有自动跨设备 restore/migration。保原件，不能重置私有状态或拿备份 marker 激活外来库。

`admitted_compile_snapshot(root, lifecycle)` 将已纳管源/schema 与有界原生文件输入、准确选中样式 module、拥有者和 manifest 一起冻结。编译器只读返回的库相对字节映射，未知依赖明确失败，不从后来磁盘或包状态补读。此不可变本地截面用于 Query 渲染产品，不宣称 causal 或持续当前的文件系统覆盖。

仍未实现完整历史 SourceSchemaBasis/causal LogicalCut、跨设备同步/托管权限、自动跨设备备份恢复、所有未知 Typst 构造的语义 transfer、真实断电或所有 filesystem/filter 组合资格。上述本地能力有界且由真实 NTFS 测试核对；不能据此宣称整个 v0.4 或后续同步协议完成。

此 crate 实现 Windows 本地 NTFS 上的打开、准确 UTF-8 源保存、create-only 另存、私有恢复稿、未决日志、有限库存枚举及显式新建库/笔记。它使用官方 Typst AST 检查声明。普通 `.typ` 不注入 metadata；随机 session/base token 不等同于文件 ID、Node ID 或历史版本。

现有文件保存通过 `NtCreateFile` 取得实际目录祖先链的 no-share-delete guards，目录访问包含 LIST_DIRECTORY/READ_ATTRIBUTES；相邻目录和叶文件按父句柄相对名字打开。叶文件为读写 share-zero，通过同一句柄读取原对象和原字节，先刷新完整 before/after 日志，再写入、截断、刷新、回读，最后刷新原操作决议。绝对 DOS 路径只用于第一次定位卷根；后续解析不再沿可变路径字符串写入。卷 GUID/NTFS、对象类型、reparse/offline/recall 标志及叶文件单一 hard-link 被核对。目录库存通过受保护的真实目录句柄枚举，不继承 `.gitignore`。

日志和 Draft 位于调用方选择的应用私有目录，调用方应将其放在用户资料及同步根外。同一私有目录仅允许一个活跃 Core，Windows share-zero 锁及私有日志/稿件目录的祖先保护持有到 Core 释放。计划、出生事实和提交决议先写入并刷新独立临时文件，再以原子 no-replace 发布最终记录名；未发布的半写临时文件不成为可恢复操作。多个独立私有数据目录打开同一资料树不提供共同 reader barrier，属于未支持的运行方式。已保存的原 operationId/相同输入重放原结果；换输入拒绝。创建的 `_with_id` API 同样支持准确重试，不能在响应丢失后分配第二身份。

未决日志阻止相关文件成为新的读写基准。事务恢复显示每个实际成员的 before/after/current 字节、对象绑定和一致性。显式 `resumeOriginalPlan` 只在原对象/父目录绑定下完成可证明的混合 Update 原计划；`adoptAfterImages` 只接纳完整且可证明的原 after 对象。第三态或未知状态不能解除屏障，namespace 中间状态可见且不盲目重放。草稿按已提交子树移动在重启后找回，保留竞争方案与原基准；显式解决归档旧方案及已审阅的合并稿。

新建库只接受尚不存在的目标目录，不能初始化任何已有目录。新目录由 create-only 系统调用直接返回的原句柄保护；配置/marker 均 create-only 构造并保留保护到操作决议。配置准确声明 `weftext.local-mvp.v1`、schema/sync 未实现。激活同时要求本应用私有日志中的真实已提交初始化事实，不能仅凭 marker 或外来 UUID 激活。新笔记为 `A/A.typ`，默认标题使用原生 `document(title: [...])` 与 `title()` 展示，和章节 heading 分开；不迁移已有笔记或传入的模板原文。声明 ID 与私有 committed birth/object binding 匹配才接纳；外来复制、未知 ID、重复声明和配对损坏分别诊断，已知节点的损坏不会自动变成普通文件绕过身份约束。

`.weftext-rules` 的 MVP 子集为以下准确相对路径 JSON；忽略边界不显示，不管理边界仍可浏览普通文件，但节点发现/新建不会重入。普通缺少同名主文件的容器仍能向下浏览并发现未被排除的配对。

```json
{"version":1,"ignored":["隐藏目录"],"unmanaged":["普通资料"]}
```

不支持 glob、`.gitignore` 语义和其他规则格式；无法完整解释时报告警告和不完整状态，阻止结构创建。单篇源上限 2 MiB，库存上限 5000 项、正文总量 16 MiB，达到限额或遇到错误时 `complete=false`。库存是有界外部观察，不是全库原子快照、完整 typed Query 或同步事实。

未实现完整历史 SourceSchemaBasis、托管权限、跨设备同步、外来库自动接纳、永久删除或出版。只复制库不能恢复本应用的身份接纳和未决责任。安装原位写入不是崩溃原子；进程故障可留下部分字节。真实断电、所有卷/filter 组合和 Windows 文件 symlink 未经资格验证；网络盘、非 NTFS、reparse、云占位及硬链接目标拒绝，不降级为 read-hash-replace。

隔离验证入口：`cargo test -p weftext-core` 和 `cargo clippy -p weftext-core --all-targets -- -D warnings`。测试包含准确 BOM/换行与无效源保全、缩短/连续保存、外部同字节替换与新稿、陈旧基准和重试、恢复第三态/破损决议、busy writer/持久 writable mapping、祖先和叶替换保护、hard-link 拒绝、无 symlink 特权的 junction 拒绝、配对/普通浏览/忽略边界/重复身份及 create-only 冲突。文件 symlink 测试在 Windows 1314 特权错误时明确未执行，不计作其平台资格证据。

系统调用合同参考 [NtCreateFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile)、[NtQueryDirectoryFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntquerydirectoryfile) 与 [FILE_DIRECTORY_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_directory_information)。
