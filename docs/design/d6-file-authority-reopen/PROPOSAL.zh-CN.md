---
source_language: zh-CN
translation_status: source
---

[English](PROPOSAL.md)

# D6-FA-r01 文件权威重开联合候选说明

状态：**candidate / partial coordinated candidate / 未接受 / 未激活 / 未实现**。

固定上游仍为 S=7e18168dad3e6d120fce0dd607dc10fa7894e252；旧 D10 作者候选基线仍为 C8=d99f053b9386c9c9e1664251fdec9f00e33fac2c。本目录只保存 D6-FA-r01 的候选后像，不修改 snapshots、inputs 或原 D10 十八份。只有全部真实 replacement owner、D10 消费修订、fresh 独立联合审查和协调接受完成后，才能讨论激活。

本文件只做路由、版本与差异说明。规范操作语义唯一归 owners/ 下对应 owner 后像，本文件不是第二份事务、冲突、身份或权限协议。

## 1. 已确定产品取舍

1. Document 当前 exact source 由普通 .adoc 文件唯一承载；Resource 当前 bytes 由普通资源文件唯一承载。
2. 设备本地可重建索引使用同步目录外的独立 SQLite，可删除重建，不保存全库完整当前正文或完整 AST 镜像。
3. 不可从笔记重建的执行控制事实使用另一份同步目录外的耐久 SQLite，保存原 decision/receipt、事务恢复、unknown、批准/claim、Money/预算责任和必要 pins。
4. 可移植身份、parent/order、lifecycle、共享配置/ACL/trust 由库内可移植元数据承载；D3 是 identity/placement/lifecycle 逻辑 owner，D6 是其物理存储/事务 owner。
5. 多台登记设备可离线普通编辑、创建、移动、重排和 Trash，之后显式处理 source/placement/lifecycle/policy 冲突。
6. 文件复制不复制 Automation、批准次数、Money、external unknown 或 stop 的消费资格。
7. 内网 Server 保留多人同时编辑；多会话可并行持有 Draft、读取和准备，同一托管后端仍只有一个持久提交 holder。
8. 实时共同文本编辑仍在 G2 之后；本候选不声称 OT/CRDT 已实现，也不采用每键作者提交。

## 2. 当前已生成 replacements

replacements.json 现在只登记实际存在的十份 fixed-S owner replacement：

第一批 D6 五份：
- d6-storage-transactions-permissions-and-sync
- d6-control-interfaces
- d6-terminology-and-naming-lexicon
- d6-terminology-registry
- d6-implementation-impact-and-test-outline

第二批 D1/D3 五份：
- d1-product-surface-and-capability-boundary
- d1-implementation-impact-and-test-outline
- d3-identity-references-ownership-and-lifecycle
- d3-terminology-and-naming-lexicon
- d3-implementation-impact-and-test-outline

机器清单只记录 fixed S sourcePath/sourceBlob、实际输出路径、消费者和真实版本差异；不存在的未来后像不得预登记。

## 3. 核心差异路由

| 主题 | fixed S 假设 | D6-FA-r01 当前 owner |
|---|---|---|
| 当前 Document/Resource bytes | authority SQLite 内完整 bytes | D6 Storage：普通文件是当前作者字节；P/I不保存全库 current body |
| identity / parent / order / lifecycle | 与 payload/control 同一数据库 current | D3 仍为逻辑 owner；D6 portable metadata 只作物理承载 |
| 操作账本 | WorkspaceId+OperationId | D3 wire12 / D6 v2：WorkspaceId + CommitDomain + OperationId |
| 多设备普通写 | continuity 不足时全局只读/fork/reconciliation | D1/D3/D6：每个 active replicaEpoch 可作限定 ordinary operation，冲突显式记录 |
| author commit | 单 SQLite 事务发布 payload/control/receipt | D6：文件安装 → P seal → portable publication；可靠保存与便携发布分开 |
| SourceVersion | Ref+Counter/store incarnation | D6 SourceVersion/2 绑定 CommitDomain、observationEpoch、revision、ChangeId |
| partial index | 不足时完整扫描或 unavailable | 无关普通保存不等待全集；complete Query/Action仍不能用缺口索引 |
| external file edit | checkout proposal only | 文件本身是 current bytes；观察缺口提升 observationEpoch，ABA不靠hash续认 |
| pins/effects | decision 可长期保留完整 source pins | purpose-bound pins + last-reference/unknown protection；旧承诺不追溯删除 |
| 局部生命周期 | 强完整范围 | D3 wire12 区分 replica_local Trash 与 managed_atomic restore/purge |
| collaboration | Server 单提交持有者；实时后续 | D1：多人会话与单持久提交 holder 分层；实时仍 post-G2 |

## 4. 三层资格

- ordinary replica content：普通 source save、create_node、局部 move/reorder、Trash 只要求真实触及的 source/identity/structure/policy 范围与合格安装原语。
- complete semantic/action proof：relation、unique、Calendar、collection、全集 bulk、restore/purge/copy/fork/import 等继续要求其真实完整正负范围。
- global execution responsibility：Automation、ApprovalUse、claim、Money、external unknown、stop 需要独立连续且已 fence 的执行责任。

semantic_pending 只表达 D2-valid source 已可靠保存、局部 typed facts 已过局部门，但列出的跨对象 obligations 尚未证明。它不是 D4/D5/D7 complete success。

## 5. 已完成版本切换

D6 已生成：
- Control wire 1 -> 2
- PreparedIntent 1 -> 2
- Policy 2 -> 3
- SourceVersion 1 -> 2

D3 本批已生成：
- identity operation wire 11 -> 12
- identity_change_receipt 11 -> 12
- resolver context/outcome 11 -> 12
- operation ledger key从 WorkspaceId+OperationId 变为 WorkspaceId+D3-CJ/3(CommitDomain)+OperationId
- replica_local 与 managed_atomic mode矩阵
- replica registration 与 continue 分离
- purge active-replica Frontier gate
- saved v9/v10/v11 原 decoder/bytes/gates继续履约

D3-CJ/3、D3Integer、Ref/Locator词法、Annotation Value/3、Result/9 不因 wire12 自动升级版本。

新 D3/D6 只完成自身 owner 后像；D4/D5/D7/D8/D9/D10 consumer尚未齐，因此**不得半包激活或产生 coordinated managed-success 产品结果**。

## 6. 剩余必须补齐的 owner

| Owner | 必须协调的真实变化 |
|---|---|
| D4 schema/relations | semantic_pending 对 relation/unique/Calendar/cross-object type 的精确消费矩阵 |
| D5 structures | native/bulk/collection 的局部与 complete 范围 |
| D7 Query/Action | CommitDomain/frontier/SourceVersion 消费、新 Prepared 版本、effects pin 保留、部分索引门禁 |
| D8 editor | Source/Live/Read、Live 三种标记策略、Draft/IME/Undo/selection、多会话与 collaboration checkpoint |
| D9 import/export | scoped pins、ImportJob、精确 export inputs、publication 与新 request/effects consumer |
| D10 | recipient/target/payload 审批、sourceOccurrenceKey 连续性、Money 谱系、Run/Lease/Automation/deployment 执行责任 |

这些文件未实际生成前不进入 replacements.json。

## 7. 首开、大库和多人边界

候选分别测量 T_first_open、T_first_edit、T_first_reliable_save、T_full_search_ready、T_OCR_ready。普通可靠保存不等待无关全库索引/OCR；complete Query/Action 不消费 incomplete index。没有性能实测前不承诺具体秒数。

Server 多用户可同时编辑同一或不同文档；持久提交排序不是前台单用户锁。G2 后实时协作仍需单独实现证据。

## 8. D10 旧终审状态

固定 B13 继续是 **REVISE**，术语/双语 FAIL，P0=0、P1=3、P2=8，共 11 OPEN：

- P1：R08-B13-P1-01、R08-B13-P1-02、R08-B13-P1-03
- P2：R08-B01-P2-01、R08-B02-P2-01、R08-B02-P2-02、R08-B02-P2-03、R08-B05-P2-01、R08-B11-P2-01、R08-B12-P2-01、R08-B13-P2-01

本候选不关闭、重分类或独立接受任何 finding。后继不可变候选仍须 fresh 完整联合审查：新提案、D10 十八份、固定 S49 与全部真实 replacement owner 后像。作者文档检查和 CI 不是独立接受。
