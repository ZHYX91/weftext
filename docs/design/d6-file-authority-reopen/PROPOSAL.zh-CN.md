---
_weftext:
  id: "2ab7d0c7-23b0-4ef4-9fb2-cb4f5413e0c1"
---

# D6-FA-r01 文件权威重开联合候选说明

状态：**candidate / partial coordinated candidate / 未接受 / 未激活 / 未实现**。

本目录是固定上游 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` 上 D6 文件权威重开的候选工作区。当前 D10 作者候选仍固定以 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 为本轮写入前父提交。这里的后像不修改 `docs/design/snapshots/`、`docs/design/inputs.json` 或固定 S；只有在后续完整 owner 后像、D10 消费者修订、fresh 独立联合审查和项目协调接受全部完成后，才能讨论激活。

本文件只做路由、版本和差异说明。规范操作语义唯一归 `owners/` 下相应 owner 后像；本文件不得作为第二份事务、冲突或权限协议。

## 1. 已确定产品取舍

本候选以已确认的以下选择为前提，不再把它们标成待定：

1. Document 的当前 exact source 由普通 `.adoc` 文件唯一承载；Resource 的当前 bytes 由普通资源文件唯一承载。用户可以用外部编辑器修改，文件库可以通过普通同步工具搬运字节。
2. 设备本地可重建索引使用独立 SQLite，位于同步目录之外，可删除重建；索引不得保存全库完整当前正文或完整 AST 镜像。
3. 不可从笔记重建的执行控制事实使用另一份独立耐久 SQLite 控制库，同样位于同步目录之外。它保存原 decision/receipt、事务恢复、unknown 外部效果、批准消费、claim、Money/预算责任及必要 pins；它不是正文数据库。
4. 可移植身份、parent/order、lifecycle、共享配置/ACL/trust 等事实必须由库内可移植元数据唯一拥有，不能只在可删索引，也不能由本地控制 SQLite 建成第二套可写 current truth。
5. 多台设备可离线编辑、创建、移动和普通删除/Trash；同步后显式处理并发版本与结构冲突。复制文件库不复制 Automation、批准次数、Money 或 external-effect 的消费资格。
6. 内网 Server 必须保留多人同时编辑：多客户端可并行持有 Draft、读取、准备和编辑不同/相同文档；持久结果仍经唯一 Server/Core 提交边界排序发布。SQLite 的单写事务不等于产品只能一人编辑。
7. 实时共同文本编辑仍保持 D1 原路线：G2 后续，不在本候选提前宣称已经实现 OT/CRDT 或发布协作能力。

## 2. 本批实际 replacement

本批只生成五份 D6 固定 owner 的完整后像：

- `d6-storage-transactions-permissions-and-sync`
- `d6-control-interfaces`
- `d6-terminology-and-naming-lexicon`
- `d6-terminology-registry`
- `d6-implementation-impact-and-test-outline`

其固定 S source、source blob、输出路径和版本差异由同目录 `replacements.json` 唯一登记。不存在的未来后像不得登记到该机器清单。

## 3. 核心差异路由

| 主题 | 旧 S 假设 | D6-FA-r01 后像 owner |
|---|---|---|
| 当前 Document/Resource bytes | authority SQLite 内完整 bytes | Storage 主稿：普通文件是当前作者字节；控制库不保存全库 current body |
| identity / parent / order / lifecycle | 与 payload/control 同事务数据库 current | Storage 主稿：库内可移植元数据唯一 current；P 只保存 decision/恢复证据 |
| operation ledger | WorkspaceId+OperationId 且与单 authority store 连续 | Control/Storage：新 CommitDomain 进入 v2 key；旧 saved v1/v9-v11 不改 bytes |
| 多设备普通写入 | 双 writer 无 continuity 时只读/fork/reconciliation | Storage：每个 replicaEpoch 可作普通 replica-local 提交并产生 ChangeId/frontier；全局执行责任仍单独连续 |
| author commit | 单 SQLite 事务发布 payload/control/receipt | Storage：文件安装 + durable-control seal + portable publication 分阶段；可靠保存与便携发布分开 |
| source version | Ref+Counter / store incarnation | Control：SourceVersion/2 显式 CommitDomain、observationEpoch、revision、ChangeId |
| partial index | 不足时完整扫描或 unavailable | 保留；并明确不阻塞无关普通保存，且不能满足全集 Action |
| external file edit | checkout proposal，文件不是第二 authority | 文件本身即当前 source；外部变化提升 observation epoch，无法证明时冲突/不可用 |
| pins/effects | planned/terminal decision 可长期保留完整 source pins | Control/Storage：purpose-bound pins、last-reference/unknown 保护、容量与可失效历史 effects；旧协议承诺不追溯删除 |
| collaboration | Server 单提交持有者；实时在后续 | Storage：多人 Draft/prepare/广播接入合同，提交序列唯一；不提前冻结 OT/CRDT |

## 4. 分层资格，不互相替代

本候选把三类资格明确分离：

- **ordinary replica content**：普通正文、创建、局部 move/reorder、Trash 等只要求该操作真实触及的 source/identity/structure/policy 范围与明确 backend 安装资格。
- **complete semantic/action proof**：D4/D5 relation、unique、Calendar、集合 membership、全集 bulk、purge 等继续要求其真实完整正负范围；building/partial index 不能冒充全集证明。
- **global execution responsibility**：Automation、ApprovalUse、claim、Money、external unknown、stop 需要单一连续执行责任；该责任不可从文件同步或副本注册推导。

`semantic_pending` 只允许表达 D2 source 已合法保存且局部 typed 事实已完成对应门，但跨对象语义仍未证明。它不等于 D4/D5 全部约束通过，不可供需要 complete semantic state 的 Query/Action/automation 自动消费。此变化触及 D4/D5 operation-applicable 消费者，后续必须形成真实 owner 后像，不能由本 D6 proposal 单方面宣布上游约束已修改。

## 5. 版本切换

D6-FA-r01 采用显式版本化，不使用“contractMajor 2”作为万能回退：

- D6 Control wire `1 -> 2`：CommitDomain、SourceVersion/2、frontier、普通保存与冲突接口、Policy/3。
- D6 PreparedIntent `1 -> 2`：保存准确 InputDescriptor、PinDirectory 和本次 write set；canonical commit request 仍不内嵌完整正文。
- D6 Policy `2 -> 3`：新增副本注册/普通内容/冲突读取与解决所需能力；旧 Policy/1/2 按原 decoder。
- D6 SourceVersion `1 -> 2`：不再允许裸 revision 跨 CommitDomain 比较。
- D6 Effect/Prepared 下游需要新 consumer 版本；本批尚未生成 D7/D8/D9 后像，因此**D6-FA-r01 不能半包激活或产生新版受管成功**。
- D3 新 decision 预计需要 wire12 承接 CommitDomain 与 local-vs-complete profile；本批不写 D3，因此当前 D3 wire11 仍是 fixed S 的唯一实际定义。后续后像完成前，本文不得被解释为已经开放 D3 新 write。
- 历史 D3 v9/v10/v11、D6 wire1、D7 PreparedActionBinding/1,/2 和所有 saved decision 继续按各自原 decoder、fingerprint、saved bytes、授权/continuity gate 恢复或重放；不得补新字段或重编码。

## 6. 后续必须补齐的真实 owner

下列文件目前只是**待补清单**，不在 `replacements.json`，也不因本批存在而被视为已定义：

| Owner | 必须协调的真实变化 |
|---|---|
| D1 product surface | 外部文件变化后的副本资格；共享文件夹从“停止全部写”修订为“停止受影响旧 cut/安装范围”；保留 Server 单提交者、多人客户端和 G2 后实时路线 |
| D3 identity/lifecycle | operation ledger key、replica registration 与 continue 的分离、local Trash receipt、purge frontier、move/order conflict、wire12 与 legacy replay |
| D4 schema/relations | semantic_pending 对 operation-applicable / relation / uniqueness / Calendar 的精确可消费矩阵；不能把 pending 当 validated |
| D5 structures | native structure/bulk/collection 在局部 save 与 complete proof 下的界线 |
| D7 Query/Action | cut domain/frontier、PreparedActionBinding/3、effects pin retention、partial-index gate |
| D8 editor | Source/Live/Read、三种 Live 标记策略、Draft/IME/Undo/selection continuity、多人 session/checkpoint |
| D9 import/export | scoped pins、ImportJob、导出精确输入、publication 与新 request/effects 版本消费 |
| D10 | recipient/target/payload approval、sourceOccurrenceKey continuity、Money lineage、Run/Lease/Automation/deployment execution responsibility |

## 7. 首开与大库边界

D6 后像分别定义并要求测量：

- `T_first_open`
- `T_first_edit`
- `T_first_reliable_save`
- `T_full_search_ready`
- `T_OCR_ready`

普通可靠保存不得等待无关全库索引或 OCR；完整 Query/Action 仍不得使用缺口索引。首次打开不要求全库 hash，不保存全库 body/AST 到 I。性能目标和秒数须由后续实际实现基准给出，本候选没有运行性能测试。

## 8. D10 旧终审状态

固定旧 B13 结果继续是 **REVISE**，术语/双语 FAIL，P0=0、P1=3、P2=8，共十一 OPEN。本候选只提供相关基础修复面，不由作者关闭、重分类或宣称通过：

- P1：`R08-B13-P1-01`、`R08-B13-P1-02`、`R08-B13-P1-03`
- P2：`R08-B01-P2-01`、`R08-B02-P2-01`、`R08-B02-P2-02`、`R08-B02-P2-03`、`R08-B05-P2-01`、`R08-B11-P2-01`、`R08-B12-P2-01`、`R08-B13-P2-01`

后继不可变候选必须进行 fresh 完整独立联合审查：新提案、D10 十八份、原固定 S 49 份及全部真实 replacement owner 后像。作者文档检查和 CI 不等于独立接受。
