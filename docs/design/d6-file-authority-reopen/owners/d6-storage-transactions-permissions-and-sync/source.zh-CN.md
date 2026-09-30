---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：`c99e6e2b-dc0c-4a3b-ab35-75a85d12ef99`。

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 中 revision05/D7/D9 的生效或候选标签只作为历史来源保留，不支配本后像。稳定文档 ID 保持不变。只有本重开所需全部 owner 后像、D10 消费者、fresh 独立联合审查和协调接受全部完成后，才可讨论激活；禁止半包启用新版受管成功。

# D6 Storage Transactions Permissions and Sync

## 1. 范围、权威分层与非目标

D6-FA-r01 的首要变化是把当前作者字节、可移植控制事实、不可重建执行责任、可重建索引和设备草稿分为不同逻辑 owner；任一物理介质都不得因方便而成为第二可写真相。

当前逻辑 owner 闭合如下：

| 事实 | 唯一逻辑 owner | 物理承载 | 可复制性 |
|---|---|---|---|
| Document 当前 exact source | 对应普通 .adoc 文件 | 用户可见普通文件 | 可由普通同步器复制字节 |
| Resource 当前 bytes | 对应普通资源文件 | 用户可见普通文件 | 可复制 |
| Node/Resource/Annotation identity、分配、不复用事实 | Portable Workspace Metadata | 库内可移植元数据 | 可复制 |
| Node parent/order、lifecycle、Trash 恢复 membership、文件绑定 | Portable Workspace Metadata | 库内可移植元数据 | 可复制 |
| Annotation 当前 closed value | Portable Workspace Metadata 中的普通数据记录 | 库内可移植数据文件 | 可复制 |
| 共享 Registry/config、ACL/policy、trust 声明 | Portable Workspace Metadata | 库内可移植控制文件；凭据/私钥除外 | 可复制并按 trust 规则验证 |
| 原 request/decision/receipt、事务恢复、unknown、批准/claim/Money、必要 pins | Durable Control Store | 库外耐久 SQLite + 私有 pin 区 | 不由普通文件同步复制消费资格 |
| metadata/search/query candidate/OCR 等派生缓存 | Derived Index Store | 库外设备本地 SQLite | 可删除重建，不同步 |
| Draft、input log、Selection、IME、未提交协作状态 | Draft Store / collaborative session runtime | 设备或 Server 私有区 | 默认不作为作者内容同步 |

Portable Workspace Metadata 以下简称 portable metadata，Durable Control Store 以下简称 durable control，Derived Index Store 以下简称 derived index；这些简称仅为本文叙述，不新增内容实体域。

Document/Resource 的 current bytes 与 portable metadata 共同形成文件型工作区，但两者职责互斥：metadata 可以指向准确当前文件、保存身份/结构/配置及版本链，不得复制完整 current body 成为另一作者源；durable control 可以保存某次原 decision 的必要 before/after pins，却不得保存全库 current body 作为读取回退。索引和 Draft 永远不是作者权威。

本稿不修改 D2 AsciiDoc Profile2 的 source 语法，不新增 D3 EntityRef，不引入 D5 持久 Record，不冻结 OT/CRDT 算法，不实现产品代码，不改变既有发布支持矩阵。实时共同文本编辑仍服从 D1 的 G2 后续排期。

## 2. Portable Workspace Metadata 与物理文件绑定

### 2.1 Portable metadata 的 current truth

一个文件型 Workspace 至少具有一个受管 portable metadata 根。首代建议物理目录名为 .weftext-meta；该名称属于 D6 文件后端，不是 D2 作者语法。portable metadata 中每个事实只有一个 current 逻辑 owner：

- Workspace identity、root NodeRef、从未复用的 identity birth/tombstone/burn-portable 摘要；
- NodeRef 到 Document 普通文件的当前 FileBinding；
- ResourceRef 到资源普通文件的当前 FileBinding；
- 每个 live Node 的 parent 与 sibling order；order 以每个 parent 的有序 child Ref 列表作为唯一 current 表示，child.parent 与 ordinal 由该列表机械导出，禁止两份可独立修改表示；
- Trash forest、restore membership 与 original-location hint；
- Annotation current closed value 及 owner；
- portable Registry binding、共享 series/scope config、共享 ACL/policy/trust 声明；
- Replica registration、ChangeRecord、Frontier、InstallationNotice、ContentCompletionProof、ConflictRecord；
- 必须可携带的 source semantic state 与 observation epoch。

文件路径、标题、内容摘要、mtime、inode/file-id 都不是 D3 identity。FileBinding 只说明在一个 portable version 中哪个普通文件承载指定 Ref 的当前 bytes；外部 rename/move 可经协调更新 FileBinding，不改变 Ref。Node parent/order 不从文件夹层级猜测；文件夹可作为 UI/存储映射，但不能替代结构 owner。

Portable metadata 的单个记录必须采用版本化 closed format、确定排序和完整 source-of-truth 关系。一个事实不得同时在两个 sidecar 中可独立写。分片只影响物理布局，读取器必须能通过 workspace root 和版本化目录恢复唯一 current record set。

### 2.2 文件对象与外部修改

D6 的文件安装证据绑定 trusted FileObjectBinding，而不是只绑定 sha256。FileObjectBinding 是 host 可信内部类型，至少固定 backend identity、规范相对路径、object generation/identity（后端支持时）、observationEpoch、byteLength、digest 及 absent/present 分支。digest 用于完整性和比较，不单独构成 CAS 或 identity。

外部程序可以直接编辑 .adoc/Resource。Core 观察到任何下列事件时必须提升受影响观察域的 observationEpoch 并使旧 SourceVersion、locator/map/PreparedIntent/cut 失效：

- object identity/generation 变化；
- bytes、长度或 metadata 与受管 binding 不符；
- watcher/journal gap；
- 文件被删除、替换、重命名且不能由 portable metadata 的已知 ChangeRecord 解释；
- provider placeholder 变为 materialized 或反向变化；
- 后端声明其事件连续性丢失。

相同 digest 不能证明 A→B→A 未发生；观察缺口后必须提升 observationEpoch，即使最终 bytes 与旧值相同。外部 current bytes 若 strict UTF-8 或 D2 parse 无效，仍作为真实外部原始字节保留；Node identity 不消失，但 D2 projection unavailable，状态为 external_invalid。Core 不用旧 pin 或 index 覆盖它，也不声称该状态是 Core author commit。

## 3. Durable Control Store、Derived Index 与 Draft

### 3.1 Durable control

每个可执行 CommitDomain 具有库外 durable control SQLite。它只保存不可从 current files/portable metadata 安全重建的执行控制事实：

- canonical request、fingerprint/input descriptor、decision/receipt/error；
- planned/terminal recovery、attempt/budget/lease、reservation 与 exact execution owner；
- installation recovery state、write set 与 portable-publication state；
- ApprovalUse、claim、Money/费用谱系、external request/send/result unknown、stop responsibility；
- PreparedIntent/PreparedAction/PreparedEdit 所需的准确 input descriptors 与必要 pins；
- ImportJob/Export publication 等受管作业控制；
- 当前 execution authority/custody/fence。

运行中的 control.sqlite3、WAL、SHM、私有 pin staging 均不得置于普通同步目录，也不得由网盘 merge。SQLite 单写者只限定一个物理数据库写事务的并发，不限制产品同时有多个用户、Draft、read、prepare 或不同文档编辑会话。

durable control 不是 portable metadata 的 current owner。它可以引用某次 portable metadata/source 的版本并保存恢复所需副本；恢复只能完成原 plan 绑定的 current version，不得从 control DB 推导一套独立最新 parent/order/policy 覆盖 portable metadata。

### 3.2 Derived index

Derived index 是设备本地 SQLite，状态闭集 building|ready|unavailable。每个层级记录完整构建 inventory、parser/profile/Registry/search/OCR version、输入 SourceVersion/Frontier 与连续失效消费位置。ready 只代表该层及范围覆盖；部分/旧 index 不能证明空范围或全集。

允许层级至少为：inventory/metadata、D2 parse projections、typed Field/relation candidates、search candidates、attachment extraction、OCR。不得用方便理由将全库 current source 或完整 AST 永久复制到 index。contentless FTS/gram 可以保存候选 token，但仍受权限、覆盖和回读语义约束。

删除整个 derived index 后，Workspace identity/structure/policy、original decisions、ApprovalUse/Money 等不变；重建不得 mint identity、重新消费 approval 或重放 external effect。

### 3.3 Draft

Draft 与 D8 原 state machine 保持设备/session 私有。Draft 保存用户 proposal、input log、selection 与 base binding；它没有 author revision、ChangeId、D3 locator authority 或 committed status。普通 cache 清理不能静默删除 dirty Draft。durable save 与 portable publication 是独立可读状态，UI 不得以 Draft persistence、worker success、HTTP 200 或 sync provider upload 代替它们。

## 4. CommitDomain、ReplicaEpoch、ChangeId、Frontier 与 SourceVersion

closed wire 形状由 Control Interfaces 拥有；本节冻结其存储语义。

CommitDomain 区分 replica 与 server。replica domain 由 WorkspaceRef+ReplicaEpoch 唯一；server domain 由 WorkspaceRef+当前托管 AuthorityInstanceId 唯一。Operation ledger v2 key 为 WorkspaceId、完整 CommitDomain canonical bytes、OperationId。不同 domain 可合法复用相同 OperationId UUID；同一 domain 内 D3/D6 protocol owner 仍必须互斥。旧 v1 ledger key 及 D3 v9/v10/v11 saved decisions 继续按旧 decoder 和连续性规则履约，不迁移为 v2 key。

ReplicaEpoch 只由 Core 在显式 replica registration 中 mint，canonical UUIDv4，永不在同 Workspace 重用。注册 replica 只授该副本在当前共享 policy/trust 下的普通内容域资格；它不是 D3 continue、AuthorityInstanceId 或 global execution custody。复制 portable files 不自动产生 ReplicaEpoch；恢复旧副本记录也不能复活已经 retired 的 epoch。

ChangeId 为完整 CommitDomain+单调 changeSequence。每个 domain 首个 content change sequence=1，checked increment，MAX 后拒绝新 content change，不 wrap。ChangeRecord 保存直接因果前驱 Frontier、实际 write set、semantic state 与 completion proof 绑定。ChangeId 不是 EntityRef、OperationId 或 source occurrence identity。

Frontier 是每 CommitDomain 至多一个最大已接纳 ChangeId 的规范向量；entries 按 CommitDomain canonical bytes 排序、无重复。空 Frontier 仅用于 workspace genesis/尚无 change。一个 replica 接纳远端 change 时形成本域新的同步接纳 decision/ChangeId，并把远端 ChangeId 作为因果前驱；不得把远端 OperationId 原样放入本域 ledger 当作已执行 operation。

SourceVersion/2 绑定完整 EntityRef、CommitDomain、observationEpoch、source revision 与造成当前 source 的 ChangeId。revision 在该 domain+entity 的受管 source 修改上 checked increment；raw no-op 不增 source revision。外部未知变化提升 observationEpoch 并产生 external observation branch，不得靠 digest 将旧 SourceVersion 复活。跨 CommitDomain 数字 revision 没有相等语义；比较必须逐项完整相等。

## 5. 授权、普通保存与完整语义资格

### 5.1 三层资格

D6 冻结三类互不蕴含的资格：

1. ordinary replica content qualification：证明本次普通 source/create/move/reorder/Trash 所实际触及的 source/identity/structure/policy 和安装后端；
2. complete semantic/action qualification：证明 D4/D5/D7 指定动作真实需要的完整正/负范围、完整 cut、授权与语义；
3. global execution responsibility：证明 Automation、ApprovalUse、claim、Money、external unknown、stop 等执行域只有连续责任持有者。

无 global execution responsibility 只暂停对应执行能力，不永久禁止 ordinary content。building index 只影响需要其完整证明的动作，不永久禁止 ordinary content。反之，ordinary content success 不升级为全集 Query/Action proof。

### 5.2 ordinary 证明范围

普通完整 Document edit 至少绑定：目标 SourceVersion/2、完整当前 source、entity lifecycle、当前 policy、actual MutationFootprint、D2 完整 parse、所有被实际修改的本地 typed facts及其当前 Registry 定义。未修改的原 bytes 必须逐字保持。若变换触及需要完整 cross-object proof 的 D4/D5 条件而当前不能证明，Core 可以在显式 ordinary-save profile 下得到 semantic_pending；不能把它记录为 complete semantics。

create Node 绑定：destination parent、完整目标 sibling list、必要祖先 chain/cycle proof、新 identity reservation、完整新 Document source、实际 local typed admission、old/new policy scope。move/reorder 绑定 subject、old/new parent、两边完整 sibling list、必要 ancestor/cycle proof、旧新授权范围；未涉及的全库 source 不作为前置。ordinary Trash 绑定明确 subtree/owner-local closure、相关 live/Trash sibling lists、restore membership 与权限；它只证明本地 lifecycle effect，不伪造完整全库 inbound-reference enumeration。

restore、purge、relation mutation、unique Calendar/config、collection create/remove、all_result/bulk 等若上游语义要求完整范围，则继续走 complete qualification；无法取得完整范围时不得降级成同一强 Action 的 ordinary success。用户若只想保留 source bytes，可另发明确 ordinary-save request，其结果 semantic_pending，强 Action 仍未成功。

有限 Field 权限仍须使用 D7/D6 的静态独立性证明；不得先读取隐藏 sibling/Field/constraint 再根据实际值决定是否允许。parent/order 的局部证明也须在读取 sibling list 前具备对应结构观察资格。

### 5.3 SemanticState

受管 source 状态分为：

- complete_semantics：D2 valid，所有本次适用 D4/D5/结构/控制语义已在声明 scope 上完整证明；
- semantic_pending：D2 valid，本次实际修改的 local typed facts 已通过，但一个或多个跨对象/全集 obligation 尚未证明；
- external_invalid：current external bytes 无法通过 strict UTF-8/D2，因此没有成功 Core author decision；只可 Source/repair/read-raw 路径消费。

semantic_pending 可以作为普通可靠保存的 current source，但不能作为以下能力的合格输入：需要完整 D4 relation/unique/Calendar invariant 的 mutation；需要 complete query cut 的 D7 ActionEvidence/all_result/post-query；自动化/Agent 自动写；purge；任何声称“整个 Workspace 约束已验证”的导出或审计。只做 exact source read、Source editor、原文件搜索或明确局部 projection 的消费者可以在自身授权下使用，并必须向用户显示 pending 状态。

这一变化要求 D4/D5/D7 后续真实 owner afterimage；在那些消费者完成前，新 semantic_pending managed success 不得激活。

## 6. 文件安装能力与可靠保存

### 6.1 BackendQualification

每个 FileBinding 所在 backend 必须公开受信能力，而不是由 UI 猜测。已有文件的 reliable save 只允许以下任一安装原语：

- conditional_replace：后端以受信 object generation/etag/file identity 作真正 compare-and-replace，只有 expected object 完整匹配时才替换；
- exclusive_write_window：host 能证明从最后完整 before 验证到安装完成期间，所有属于本威胁模型的 writer 都无法修改/replace/delete 该目标；仅 advisory lock 不算；
- create_only：只用于 expected absent 的新文件，提供原子 create-if-absent。

安装还必须支持 staged bytes 的完整耐久、目标文件耐久和目录项/rename 耐久证明。若平台只提供“最后 hash 再 replace”，这不是 conditional_replace。若无法证明安全安装，就保留 current file 与 Draft/after branch，返回 conflict 或 install_unavailable，绝不报告 reliable save success。

外部未合作程序可能不遵守 Weftext lock。D6 的保证只覆盖 BackendQualification 明确证明的 writer 集；一旦 backend 无法排除 race，本次既有文件安装不得进入 success。发现 unknown competing bytes 时必须保留它们，不覆盖、不删除、不用 after digest 猜是本次写入。

### 6.2 Guarantee

普通 commit 的 guarantee 闭集：

- replica_local：证明本 CommitDomain 的固定 write set 已在合格文件原语下可靠安装并 durable sealed；不声称其它离线 replica 没有并发版本；
- managed_atomic：证明整个受管 operation 的所有受管作者/portable-control write 位于同一排他安装/发布屏障，且读者只看到 old cut 或 sealed new cut。普通外部文件读者不保证跨多文件瞬时原子可见。

managed_atomic 不把多个 rename 称为文件系统全局事务。跨文件操作使用受管发布屏障、per-component install evidence 和 durable-control seal；外部读者仍可能在中间看到物理文件，产品不得宣传为任意外部工具的瞬时多文件快照。

## 7. Prepared、安装、seal 与 portable publication

### 7.1 InputDescriptor 与 pins

D6 v2 canonical commit request 保持小型控制请求，不内嵌完整 Document/Resource bytes。PreparedIntent/2 保存 InputDescriptor/2：准确 Workspace/CommitDomain、intent kind、before SourceVersion/metadata versions、
  expected Frontier、actual scope/profile、write-set descriptors、
    Registry/policy/rule versions和其拥有者规范输入。完整 source/bytes 由 PinDirectory 中 purpose-bound pins 承载。

input equality 要求 descriptor canonical bytes 与其所引用 exact pins/owners 完整一致；相同 sha256 不足以证明相同输入。修改任一 source、mapping、policy、frontier、scope 或 owner request 都需要新 prepare/OperationId，除非原协议明确为 same canonical replay。

### 7.2 Pin 分类

每个 pin 必须记录 purpose、owner request/job、byteLength、payload type、source version、
  created control revision、last-reference policy、retention class、capacity account。允许长期保护的类别仅为：

- planned/installation recovery 的 before/after；
- unresolved conflict branch 与 merge base；
- external unknown/approval/Money 责任所需冻结证据；
- ImportJob/ExportPlan/Query result 等其 owner 协议明确要求的 pin；
- 明确用户 history/backup policy。

打开/解析/索引 Workspace 不能自动为每份 current source 建永久 pin。decision 不再引用、无 unknown/冲突/用户保留且达到 retention policy 后，大 effects bytes 可以失效；canonical request、decision state、receipt/error、费用/批准/claim 责任和最小 input descriptor 继续耐久。旧 D6/D7 协议已经承诺永久/decision-lifetime pins 的 saved decision 不追溯删 pin；新 retention 只适用于新版本记录。历史 effects pin 已合法回收后读取返回 effects_unavailable，不能回读 current file 冒充旧 after。

容量不足在开始新 installation 前失败或暂停，不删除 protected pin。planned/unknown pin 的最后引用不能由 TTL、preview expiry 或用户关闭窗口解除。

### 7.3 状态机

每个 v2 content decision 保存独立 DecisionState、InstallationState、ReliableSaveState、PortablePublicationState：

DecisionState：unseen → rejected | planned → committed | terminal_failed。
InstallationState：prepared → planned → installing(k) → installed；
  另有 conflict | recovery_unknown | paused_authorization | paused_capacity。
ReliableSaveState：not_saved | reliable。
PortablePublicationState：not_published | pending | published | conflict。

顺序固定：

1. prepare：冻结 InputDescriptor、exact write set、
  before/after pins、语义 proof、preview、budget；无 author effect。
2. planning：先证明 required pins durable/capacity reserved；P 事务保存 canonical request、
  fixed plan、reservation、installation recovery description 与 planned。
3. portable installation notice：在修改任何 portable current 文件前，写入并 durable flush InstallationNotice，列 ChangeId、operation binding、
  guarantee、before Frontier、component/write-set descriptors；
    它不含 approval/Money/external payload，也不是 completed decision。
4. install：每项 staged after 先完整写入并 durable flush，再通过 BackendQualification 允许的 create_only/conditional_replace/exclusive window 安装；每个 component 保存实际 FileObjectBinding/outcome。move/Trash 先确保可恢复目标已 durable，不先永久删除 current。
5. installed verification：以安装后的**自身 write set 指定 poststate**为 expected 值核对每个实际 component；不得再次要求这些目标仍等于原 before。对未写的 positive/negative dependencies、当前 policy/auth、Registry/rules、Frontier 范围和 concurrent control facts重验；自产生 ChangeId/SourceVersion 只与 fixed plan 的 planned poststate 比较。
6. seal：全部已写 component 等于 planned poststate、未写依赖仍成立、current authorization 允许 seal 时，在 P 单一 durable transaction 写 committed decision、canonical receipt、ReliableSaveState=reliable、effects metadata、execution charges/approval consumption（适用）、outbox。此 P durable seal 是本次受管 decision 的提交点。
7. portable publication：从 saved decision 生成 ContentCompletionProof，写入 portable metadata 并 durable flush，
  推进 Frontier，解除受管 portable publication barrier，PortablePublicationState=published。

第 6 步成功而第 7 步失败时，decision 和 reliable save 已成功；portable publication=pending。恢复只补同一 completion proof，不重写 source、不新建 OperationId、不重复收费。远端 replica 在完整 proof+components 到齐前只看到 incomplete transport，不能把文件先到达当 committed portable version。

撤权发生在 seal 前：停止 seal。若可以在 BackendQualification 保护下安全恢复 exact before，则恢复并 flush；如果不能证明安全恢复，保留 before/after/现文件及 recovery evidence，进入 paused_authorization 或 recovery_unknown，不覆盖第三种 bytes。撤权不将一次未知安装写成永久业务 rejection。

## 8. 崩溃恢复与历史结果

恢复总则：先读 P 决议，再读取 installation notice、write-set bindings 与实际文件。客户端超时、文件存在、mtime、digest 或 provider 状态都不能替代 P。

每个 component 分类只允许：

- exact_before：FileObjectBinding/bytes 与原 plan before 完整匹配；
- exact_after：与该 plan 固定 poststate 完整匹配，并可证明安装归属；
- third_state：其它 bytes/object/placeholder/缺失；
- unavailable：后端无法安全读取或证明。

相同 bytes 但不能证明安装归属时不能从 third/unavailable 提升到 exact_after。

故障规则：

- 无 planned：只清理已证明无引用 staging，current 不变。
- planned、未 install：恢复同一 plan/reservation；不重新 sample identity。
- installing：对每 component 分类。only before/after 且安装来源连续可证时恢复同一 plan；出现 third_state 就保留现文件、pins，进入 conflict/recovery_unknown。
- 全部 after 但 seal 未知：先读 P。P committed 则重放 receipt；P planned 只恢复同一 plan，不能凭 files 猜 committed。
- committed 但 response 丢失：当前 replay authorization 通过后返回保存的原 receipt；不再写 files、Frontier 或费用。
- committed、portable publication pending：只恢复 ContentCompletionProof。
- derived index/outbox 更新失败：重建/补消费，不回滚 author decision。

历史 r5 receipt 与 current r6 source 分开：replay r5 只返回 r5 saved bytes；current read 使用 r6 SourceVersion/Frontier。Undo/restore 如需 old bytes 必须走新的 plan/preview，不能让 receipt replay 倒退 current。

## 9. 同步、接纳与冲突

### 9.1 Replica registration 不等于 execution takeover

新设备获得完整 portable Workspace 时，先验证 workspace identity、portable metadata chain、policy/trust 与可用 current components，再显式 register 新 ReplicaEpoch。注册只建立 ordinary content CommitDomain。它不接管旧 P 的 ApprovalUse、claim、Money、external unknown 或 Automation lease，也不使用 D3 continue_workspace。

执行域接管必须另有完整 continuity proof，证明原 decision/receipt、charges、unknown、claims、standing approvals、stop state 全部连续且旧执行者已失效；无法证明则该执行能力暂停，但普通 replica content 继续。

P 丢失时，原 execution decisions/unknown 不能从 files 重建。若 portable installation notice 指示某范围可能有未决安装，则该范围先进入 reconciliation；无安装疑点的普通 source 仍可在新 ReplicaEpoch 下工作。建立空 control DB 不退款、不补 approval、不重发 external request。

### 9.2 Transport completeness

同步 provider 只运输 ordinary files 和 portable metadata immutable/versioned records；
  不运输活动 control.sqlite3/WAL/SHM、derived index 或 Draft。

接收端只有在某 ChangeId 的 InstallationNotice、ContentCompletionProof 和全部 listed component bytes/metadata 到齐且互相验证后才接纳该 change 到本域 Frontier。先到 Document 后到 sidecar、先到 sidecar 后到 Resource、placeholder 尚未 materialized 都是 incomplete，不是空值、删除或成功 commit。

未下载 placeholder 的状态是 not_materialized。需要读取其 bytes 的 operation 返回 source_unavailable/对应 owner unavailable；不能用 size=0 或 not_found 代替。

### 9.3 ConflictRecord

同步检测到并发 source、placement、lifecycle、identity 或 policy heads 时建立稳定 ConflictRecord。ConflictKey 至少绑定 WorkspaceRef、closed conflict kind、受影响 Ref 集、所有并发 head ChangeId；Ref 和 heads 规范排序、唯一。ConflictId 是对完整 canonical ConflictKey 做域分离 SHA-256 得到的地址，完整 key 仍必须保存并比较，hash 不能代替证据。

state 闭集 open → resolution_prepared → resolved；新 head 出现使旧准备 superseded 并产生关联后的新 ConflictKey。resolution request 必须绑定 exact current key/heads 和 owner-specific resolution；最终写仍编译成原 D3/D6 typed request，不存在 conflict bypass transaction。

至少覆盖：

- source/source：保留两个 head 和共同 base；非重叠也只可生成明确 merge proposal，不后台自动 commit；
- create/create：两个不同 identity 均保留；同 parent order 冲突显式解决，不按 UUID/mtime 伪造用户顺序；
- move/edit：identity 相同且维度真正独立时可组合，但须重新验证 destination structure/policy；否则冲突；
- move/move 或 cycle：不自动选一个 parent，不挂 root；
- Trash/edit：保留 edit bytes 与 Trash intent，不默认 delete-wins；
- purge/restore：已证明 tombstone 不复活；遗留 bytes 只能作为 fresh-copy 输入；
- duplicate Ref/birth mismatch：identity_collision，普通 resolver 不选“第一个”；解决可保留一边 identity、另一边通过 D3 fresh copy/adopt 流程；
- policy conflict：不做 allow union；在显式解决前按不扩大权限的安全交集/deny 优先提供受限读取，管理写暂停；
- sidecar/body partial：incomplete，不 mint identity；
- placeholder：not_materialized。

存在某个 ConflictRecord 不冻结整个 Workspace。用户可明确打开某分支并继续形成后继 branch；没有 branch 选择的 ordinary read 不得随机取一个 current。

### 9.4 Trash 与 purge

多设备 ordinary delete 只执行 Trash。local Trash receipt 证明本操作已知 closure、owner-local membership、placement 与实际 source effects；它不能伪造全 Workspace inbound reference enumeration。

permanent purge 保持强操作：要求完整相关 inbound/owner closure、complete semantic proof，并要求登记 replica 对 purge frontier 已确认，或显式 retire 无法参与的 replica。retired replica 以后带回旧 bytes 只能进入 conflict/reconciliation，不能恢复旧 ReplicaEpoch 或 tombstoned identity。Purge 不承诺从已经复制到其它设备/backup 的历史字节安全擦除。

## 10. Index、Query 完整性与大库启动

首开路径分阶段，不使用全库完整 hash 作为启动前提：

1. T_first_open：读取 workspace root/portable metadata 入口，列出已发现目录和活动目标；
2. T_first_edit：打开活动 source，建立 Draft；
3. T_first_reliable_save：活动目标通过 BackendQualification、install、seal，得到 ReliableSaveState=reliable；
4. T_full_search_ready：指定 search profile/范围的 complete coverage 成立；
5. T_OCR_ready：指定附件/模型/版本的 OCR 层完整或明确失败。

inventory/metadata 枚举、D2 parse、typed index、search candidates、attachment extraction、OCR 使用有界队列、有限在途 bytes、批量 index transaction 和可续建 checkpoint。10k/100k/1M 小文件与几十 GB 附件/正文必须分别实测；本架构不给秒数保证。

Exact source scan 是正确性 baseline。候选 index 命中后必须回读准确 file/source version 进行最终判断；召回不完整的 tokenizer/gram 不能用于 exact/NFC/regex 的 complete proof。D7 complete Query 需要完整 authorized execution 和负范围依赖；building/partial index 只能用于标明“已扫描范围”的探索视图，不能签发完整 ResultHandle/ActionEvidence。

## 11. Server 多用户与实时协作接入

Server 托管 Workspace 仍有唯一持久 CommitDomain/提交持有者，跨全部 Server 进程/实例/failover 必须 fence 旧 writer 对**durable control 和作者文件**的写能力。只让旧 SQLite transaction 失败、却允许旧进程继续 rename author files，不算 fencing。

多个认证用户可以并行：

- 读取不同/相同文档；
- 持有各自 Draft；
- 执行 D2 parse、D7 Query/prepare；
- 编辑不同文档；
- 编辑同一文档并在不同 base 上保留提案。

不同文档的 author commit 只锁其 write set 和真实依赖范围，按固定 key 顺序取得范围锁；P 的最终 seal transaction 可以短暂串行化 commitSequence。无关文档不得仅因一个全局 coarse sequence 改变就自动 abort。

同文档非实时阶段：A seal 新 version 后，B 的 Draft 不丢失；B 的旧 base prepare/commit 返回 stale/conflict，并进入 base/current/proposed 三方流程。

G2 后实时会话的 D6 接入合同现在冻结，但不冻结 OT/CRDT 算法：

- session 绑定 Workspace/Node、committed Base SourceVersion、
  sessionEpoch、participant principal/session 和有序 client input sequence；
- receive ack、broadcast、durable checkpoint 三者分离；receive/broadcast 不叫 saved；
- 参与完整 source session 需要自身完整 source_read；一个用户不能借另一参与者资格看到隐藏 bytes；
- IME preedit 不成为 shared author op；composition 最终确认至多形成一个 Draft input transaction；
- checkpoint 由显式 save/collaboration-checkpoint 触发，不能每键 author commit；
- checkpoint 对合成后的 exact source 重做当前每个尚未提交贡献者的 authorization、actual footprint、D2/D4/D5适用门和依赖；
- 撤权在 seal 前线性化获胜时，该主体未提交贡献不能借别人的身份提交；相关 input 保留成待重确认 proposal；
- committed update 只从 saved decision/outbox 广播；每个接收者再次经过当前读取 gate；
- event gap、reconnect、mapping proof缺失时 reset/rebase，不按同文字猜 selection；
- collaboration transient op log 只为活跃 session/恢复使用，受 TTL/capacity 限制，不成为第二 durable Document source。

未来 OT/CRDT adapter 必须证明输入归属、合成结果、source mapping、selection mapping、重复/迟到处理和收敛；证明失败保留所有用户输入并停止该 checkpoint，不以单用户锁或静默丢 op 伪装成功。D1 原 G2 后续发布边界不因此提前。

## 12. History、backup、pins 与修复

Portable backup 由一个明确 Frontier 的 ordinary files + portable metadata closed snapshot 组成；它不自动包含可消费的 execution control authority。需要灾难恢复 global execution responsibility 时必须另有受保护 control backup，并在接管时证明旧执行者失效和 ledger/charges/unknown continuity。

History 可以保存旧 source/resource bytes，但它是明确 retention product，不是 current source。普通 purge resolver 不可从 history 复活同 identity；用户可明确从 history bytes 建 fresh content。tombstone/no-reuse facts按 D3 保留。

repair 只在显式 repair/audit 权限下运行。它可以：

- 检查 portable record graph、component digests、FileBinding、Frontier holes；
- 恢复 P 已证明的原 planned install；
- 将第三种 bytes 保存为 conflict branch；
- 重建 derived index；
- 重新 materialize缺失但有合法 backup 的 ordinary file。

repair 不可：

- 从 current file 猜原 receipt/ApprovalUse/Money；
- 以相同 digest 认定原 install 成功；
- 自动删除 conflicting bytes；
- 重新分配旧 Ref；
- 用 backup 绕过 purge/tombstone；
- 无 continuity 时重发 external effect。

## 13. 预算、资源与执行责任

BudgetBinding、attempt allowance 和 persistent charge 继续是执行控制事实。work batch 先 charge 后执行；crash 不退款；attempt/clock epoch 无法证明时保持 planned/paused，不伪造 terminal business failure。

Pin/capacity 预算至少分开：recovery pins、conflict pins、preview/query pins、import/export pins、user history。protected planned/unknown pins 不因 preview TTL 清理。容量不足时可以拒绝新 prepare/install，不能通过删除 last-reference pin 取得空间。

global execution responsibility 的最小 continuity 包括：

- canonical original request/decision/receipt/error；
- current lease/claim/standing approval consumption；
- Money budget lineage：Run、Lease、Automation、Workspace、
  deployment 及 reservation/charged/refund evidence；
- external request/send/result frozen binding 与 unknown state；
- stop/emergency state；
- sourceOccurrenceKey 等自动化连续性所需 evidence。

普通 replica registration、FileBinding、ChangeId/Frontier 不提供上述消费资格。sourceOccurrenceKey 不能从 path、line、same Field key、same digest 或“唯一候选”重建；只有受管连续 change chain 能证明延续。观察 gap 后依赖 continuity 的 automation 暂停并要求显式 rebind。stop 先 durable 阻止新派发/消费，再处理在途取消；无法证明 provider effect 已撤销时仍为 unknown。

## 14. 与 D3/D4/D5 的版本化协调边界

D3 fixed S 的 wire11、ledger key、stage order、continue/fork、Trash/purge/receipt 仍是当前历史规范。本候选要求后续 D3 wire12 明确承接 CommitDomain、replica-local profile、new ledger key、local Trash receipt、purge frontier 与 legacy replay。D6 不通过“通用 commit”绕过 D3 identity/lifecycle owner；在 D3 后像完成前，不得激活需要 D3 v12 的新 managed success。

D4/D5 的现有 complete operation gates 不能被 semantic_pending 偷偷视为已满足。本候选只冻结 storage ability to persist a D2-valid pending source；哪些 D4/D5 facts 可以 local-only 验证、哪些必须 complete，必须由后续 D4/D5 owner afterimage列出。未完成前，依赖这些新分支的 Action/automation 保持 unavailable。

旧 D6 wire1、Policy/1/2、SourceVersion/1、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1 及其 saved decisions/pins 按原 decoder/retention/continuity 规则恢复和重放。新版本不得修改旧 saved bytes 或用新 pin-retention 政策追溯删除旧协议证据。

## 15. 接受边界

D6-FA-r01 目前只是作者部分联合候选：

- 没有产品代码实现；
- 没有文件系统 conditional-replace/exclusive primitive 实机证明；
- 没有 Server multi-user/real-time conformance；
- 没有 10k/100k/1M 文件或几十 GB 性能数据；
- 没有 D3/D4/D5/D7/D8/D9/D10 全部 consumer afterimage；
- 没有 fresh 独立联合审查。

任何 documentation check/CI 只证明其明确检查项，不构成上述证据。完整后继候选须以固定 S 49 原输入、全部实际 replacement owner、新 D10 十八份共同接受。
