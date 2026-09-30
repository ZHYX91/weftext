---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：1bde8ca5-50fd-41ec-a146-b31d951e46c2。

# D3 身份、引用、所有权和生命周期

候选状态：D6-FA-r01；部分联合候选；未接受、未激活、未实现。固定 S 的 D3 wire11/Result9 与既有 v9/v10 saved decision 只作为历史兼容来源。本后像定义新决议的 D3 wire12；它必须与 D6-FA-r01、后续 D4/D5/D7/D8/D9/D10 owner 后像共同接受后才可激活。本文不授权产品实现、发布或 A2。

## 1. 范围、冻结输入与非目标

D3 继续唯一拥有：

- WorkspaceRef、NodeRef、ResourceRef、AnnotationRef 的 identity algebra；
- Node parent/order 的逻辑语义；
- Node/Resource/Annotation 的 live、trashed、tombstoned 生命周期；
- Locator、anchor、reference lifecycle、copy/fork/continue/import identity 规则；
- D3 operation request、receipt、resolver outcome 和 fail-closed family。

D6 唯一拥有 CommitDomain/2、ReplicaEpoch、ChangeId/1、Frontier/1、SourceVersion/2、InputDescriptor/2、PreparedIntent/2、文件安装、P seal、ContentCompletionProof、ConflictRecord 和 execution responsibility。D3 只消费这些 exact 类型，不重新定义同名对象或建立第二可写真相。

D2 exact source、D4 typed schema/relations、D5 structure、D7 Query/Action、
  D8 editor、D9 conversion 和 D10 external execution 仍由各 owner 定义。

## 2. 设计目标

1. 物理文件路径、索引行、mtime、digest、同步 provider ID 都不是内容 identity。
2. 同一逻辑 Workspace 的多个已登记副本保留同一 durable refs，同时允许各自离线普通内容操作。
3. 普通副本内容资格与 global execution responsibility 分离。
4. parent/order、lifecycle、tombstone 和 no-reuse 事实必须可移植，不能只在可删 I。
5. replica_local 成功只证明该提交域内的可靠内容决议；它不冒充所有副本联合当前态或 D7 全集证明。
6. managed_atomic 保留需要完整 closure、完整正/负范围和强恢复语义的操作。
7. 外部变化、placeholder、partial transport 和 ABA 不得通过相同 hash 被折叠为旧版本连续。
8. saved decision replay 与 current source state 永远分开。

## 3. 身份代数与术语分域

WorkspaceId、AuthorityInstanceId、NodeId、ResourceId、AnnotationId 继续使用 canonical lowercase RFC 4122 UUIDv4。D3Integer 继续是数学整数 0..2^63-1，禁止 bool、浮点、指数、负值和 wrap。

NodeRef 由 WorkspaceId + NodeId 构成。ResourceRef/AnnotationRef 必须编码 owner NodeRef 与 owner-local ID。完整 typed ref 才可比较；裸 UUID、path、title、digest 或 provider key 不参与 identity equality。

Document 没有独立 durable ID；以 owning NodeRef 寻址。Document element、heading、paragraph、
  table row/cell、Field occurrence、Saved Query/View occurrence 都不是第四类 entity。

ForeignIdentityKey、SourceBinding、OriginBinding、Provenance、LogicalOccurrenceKey、
  ResultRowHandle、ActionEvidence、OperationId、AuditEventId 都不 coercion 为 content ref。

## 4. wire12、提交域与幂等账本

### 4.1 D6 类型消费

wire12 直接嵌入并按 D6 decoder 验证 CommitDomain/2、Frontier/1 和 InputDescriptor/2。它们的 canonical bytes 使用 D6 已冻结规则；D3 不接受“看起来相同”的私有对象。

CommitDomain.replica 的 replicaEpoch 必须是当前 active ReplicaRecord；CommitDomain.server 的 authorityInstanceId 必须等于当前托管 authority。D6 先证明 domain/back-end/fence 连续性，D3 才访问 identity ledger。

### 4.2 OperationId ledger key

新 wire12 ledger key 唯一为：

    (boundWorkspaceRef.workspaceId, D3-CJ/3(commitDomain), operationId)

protocolOwner=D3 与同一 D6 v2 control ledger 一起保存。相同 Workspace 与 OperationId 在不同 CommitDomain 中是独立 key；相同 key 只能有一个 request fingerprint/decision。

状态仍为 unseen → rejected 或 unseen → planned → committed | terminal_failed。rejected、committed、terminal_failed bytes 不可改写；planned 只能恢复原计划。

不同 fingerprint 使用同一 key 固定 operation_id_conflict，不读取或回显原请求内容。

### 4.3 wire12 identity operation request

新决议只接受以下顶层闭合对象：

~~~json
{
  "wireVersion":12,
  "kind":"identity_operation_request",
  "operationId":"uuid-v4",
  "boundWorkspaceRef":<WorkspaceRef>,
  "commitDomain":<D6 CommitDomain/2>,
  "guarantee":"replica_local|managed_atomic",
  "expectedFrontier":<D6 Frontier/1>,
  "inputDescriptor":<D6 InputDescriptor/2>,
  "mode":<D3Mode>,
  "expectedAuthority":<ExpectedAuthority?>,
  "workspaceProposal":<WorkspaceAllocationProposal?>,
  "preparationBinding":<PreparationBinding?>,
  "intent":<D3Intent>
}
~~~

带 ? 的成员只在矩阵允许时出现；不适用必须省略而不是 null。所有其它未知成员拒绝。

inputDescriptor.workspaceRef、commitDomain、expectedFrontier、guarantee 必须与顶层逐字 canonical-equal；intentKind 必须声明 D3 identity operation v12。ownerInput 的 canonical descriptor 由 D3 v12 owner 产生，大型 Document/Resource/Annotation bytes 只能以 D6 PinRef typed slot存在。canonical request 不永久内嵌完整正文或附件。

完整输入相等必须比较 canonical InputDescriptor、owner descriptor 与每个引用 exact pin。相同 sha256 本身不证明相同 input，也不能证明 ABA 连续。

### 4.4 requestFingerprint

D3-CJ/3 与 SHA-256 规则保持不变。fingerprint body 包含除顶层 operationId 外的全部规范请求字段；workspaceProposal 仍以其完整 canonical digest 进入。

任一 commitDomain、guarantee、expectedFrontier、InputDescriptor、authority expectation、proposal、preparationBinding、intent 或 plan 变化都改变 fingerprint。

### 4.5 guarantee 与 mode 矩阵

| mode | replica_local | managed_atomic |
|---|---|---|
| create_node | 允许 | 允许 |
| move_node | 允许 | 允许 |
| reorder_node | 允许 | 允许 |
| trash | 允许 | 允许 |
| create_workspace | 禁止 | 允许 |
| create_resource/create_annotation | 禁止 | 允许 |
| copy_node_subtree/copy_resource/copy_annotation | 禁止 | 允许 |
| fork_workspace/continue_workspace | 禁止 | 允许 |
| restore/purge | 禁止 | 允许 |
| import_new | 禁止 | 允许 |

replica_local 禁止 workspaceProposal、expectedAuthority 与 preparationBinding。它由 D6 active replica、portable registry/policy 和文件后端资格提供 commit-domain fence。

managed_atomic 沿用 wire11 的 expectedAuthority 模式：create/fork 使用 create；continue 使用 continue；其余使用 existing。create/fork 继续使用 proposal P1/P2 与 TargetLedgerCustody；server CommitDomain 还必须与 expectedAuthority 当前实例一致。

D7-mediated wire12 preparation 仍等待 D7 owner afterimage。preparationBinding 的 outer shape 保留原 token wrapper，但当前 partial candidate 不允许它产生 wire12 managed success；
  D6 preparation capability 返回 owner_update_required，零 D3 ledger decision。D3-native typed operations可以省略 preparationBinding。

### 4.6 allocation 与 UUID 不复用

同一个 CommitDomain 内，Core 在 P 的 allocation history 上 reserve/burn fresh IDs。
  portable metadata 保存 accepted birth、tombstone 和 no-reuse facts。

两个离线 replica 极低概率独立 mint 同一完整 typed ref 时，不按“先同步者”或 hash 自动选 winner。接收端形成 identity_collision；在冲突解决前没有联合 canonical entity。显式解决可保留一个 birth claim，并把另一分支内容通过 fresh-copy/import 产生新 ID。losing claim 永不被静默 rekey。

已 accepted tombstone 的 ID 永不复用。burned reservation 也不能在同一提交域再次分配。

### 4.7 SourceVersion/2 与 ABA

D3 Locator、selector 和 prepared evidence 的 currentness 必须同时绑定 D6 SourceVersion/2。裸 revision Counter 不可跨 CommitDomain 比较。

observationEpoch 变化使旧 locator/preparation/action evidence 失效；A→B→A 即使最终 bytes/digest 相同，也不能恢复旧连续性。sourceOccurrenceKey 的更高层连续性仍由 D10 owner决定，D3 不按 path/hash猜测。

### 4.8 legacy v9/v10/v11

v9/v10/v11 saved request、proposal、planned、receipt、error 和 PreparedActionBinding/1,
  /2 继续用原 decoder、fingerprint、authorization、custody、pin retention 与 byte-equivalent replay。

禁止：

- 把旧 request重编码成 wire12；
- 给旧 receipt补 CommitDomain/ChangeId；
- 用新 retention删除旧协议保证仍存在的 evidence；
- 用 semantic_pending 重解释旧 complete gate；
- 因当前文件与旧 pin hash 相同而恢复旧版本资格。

新 decision 不接受旧 wire；旧 saved decision 不接受新语义字段。

## 5. Workspace、Node、owner 与有序结构

### 5.1 Workspace 与 replica

WorkspaceRef 仍是 identity namespace；replicaEpoch 不是 Workspace identity。Replica registration 保留 WorkspaceId 和已有 Node/Resource/Annotation refs，只增加一个新的 D6 CommitDomain。

注册 replica 不调用 continue_workspace，不分配新的 WorkspaceId/AuthorityInstanceId，不接管 global execution responsibility，也不声明旧设备停止。

### 5.2 Node parent/order

Workspace 的 live Node 仍构成从 root 全连接的有序树。root parent/ordinal 为 null；非 root live Node 必须恰有一个 live parent。同一 parent 的 siblingOrdinal 从 0 连续、唯一。

逻辑 parent/order owner 仍是 D3，物理承载由 D6 portable metadata 实现。path、目录树、索引排序、文件枚举顺序、mtime 都不能成为第二 owner。

move 保持 NodeRef。title/path/label 变化不改 child identity。删除 parent 若要保留 children，必须先显式 move，不能产生 live orphan。

### 5.3 Resource 与 Annotation owner

Resource/Annotation owner 不可变。跨 owner 语义只能 fresh-copy。owner Trash/purge 的 closure、reply 无环、Annotation target owner-local 等既有规则保持。

同 owner 的 Resource bytes replace、Annotation body/target edit保持 ref，但产生新 SourceVersion/2。

### 5.4 结构冲突

并发 placement heads 不自动按 UUID、路径、ordinal 或 mtime修复。D6 ConflictRecord.kind=placement_concurrent 定位完整 heads；普通 resolver 不选择一个隐藏 winner。

冲突分支仍可被明确打开和继续编辑，但未指定 branch 的结构读取不能声称得到唯一 canonical tree。

## 6. Locator、anchor、exact source 与外部字节

DocumentElementLocator、DocumentRangeLocator、ResourceRegionLocator 与 l1 token 的词法、owner、revision、span、
  range、base64url 和 D3-CJ/3 规则保持不变。AuthorAnchorAddress 仍是 current-revision symbolic address，不是 entity。

wire12 consumer 另外绑定 SourceVersion/2 与 CommitDomain；same text、
  same span 或 same token payload 不能跨 observationEpoch 恢复 currentness。

D2 valid managed source 才能形成普通 D3 author mutation。外部 physical-invalid 或 D2-invalid bytes 保留为真实文件字节，通过 D6 current-source/repair 入口读取和诊断；它不编码为 SemanticState，不产生成功 D3 author receipt，也不由旧 pin覆盖。

## 7. lifecycle：普通 Trash 与强 restore/purge

### 7.1 共同状态机

Node、Resource、Annotation 仍使用 unallocated → live → trashed → live|tombstoned。tombstoned 是终态；Document 跟随 Node。

Trash 仍保留 typed refs、payload 和恢复信息。purge 删除 payload并发布最小 tombstone，不能从 Weftext restore。

### 7.2 replica_local Trash

replica_local Trash 必须完整证明：

- target 与实际 subtree/owner-local/reply membership；
- 受影响 live/Trash sibling lists 的连续唯一 ordinal；
- 当前 policy 与该 closure 的 lifecycle write 权；
- 被修改 portable metadata components 的安全安装资格。

它不要求为普通离线删除先扫描所有其它副本和全库所有 inbound references。未证明的 inbound/cross-object obligation 必须出现在 D6 SemanticState.semantic_pending 中，至少含 inbound；receipt 不得把其局部 referenceLifecycleTransitions 宣称为全 Workspace 完整枚举。

远端 source 到达后形成 delete/edit 或 lifecycle_concurrent 时显式冲突；不得默认 delete-wins 或 edit-wins。

### 7.3 managed_atomic Trash

managed_atomic Trash 可在完整正/负引用范围已证明时产生 complete_semantics，并保留 wire11 的完整 suspended/reference lifecycle 证据。

### 7.4 restore

restore 只允许 managed_atomic。它保持 identity，精确消费 Trash graph、restore location、owner/reply closure、当前引用与完整 required dependencies。

restore 与另一个副本的 edit/move/delete head冲突时先形成/解决 ConflictRecord，不能按路径猜原位置或偷偷新建 ID。

### 7.5 purge 与 tombstone

purge 只允许 managed_atomic，并要求：

1. target 已 trashed；
2. 完整 owner/reply/subtree closure；
3. 全部适用 inbound refs与 D4/D5 强约束已证明；
4. portable replica registry revision固定；
5. 每个 active 已登记 replica 的已接纳 Frontier 对 purge required frontier 达到规定覆盖；
6. retired replica 不再算 active ack，但其旧 epoch 永不可恢复 active；
7. tombstone/no-reuse 与 ContentCompletionProof 同步发布。

从未登记、系统不可证明曾参与该 Workspace 的设备不形成永久阻塞者。若以后拿旧目录接入，它必须注册新的 ReplicaEpoch，并用当前 tombstone/ConflictRecord 协调；不得以旧文件复活 tombstoned ref。

“同步服务显示完成”不是无 inbound ref、无离线 head 或 purge coverage 的证明。

## 8. Create、Move、Copy、Fork、Continue 与 Import

### 8.1 create_node

replica_local create_node 需要：

- active replica CommitDomain；
- 当前 parent/ancestor chain 和完整 destination sibling list；
- fresh NodeId reservation；
- D2-valid source；
- 当前 Registry 下本次 source 的 local typed facts验证；
- portable identity birth、placement 和 source components 的安全安装。

跨对象 relation/unique/calendar/collection/inbound 等未证明义务进入 semantic_pending。它们不能被下游当完整 D4/D5 成功。

managed_atomic create_node 还可要求 complete_semantics 和 D7/D4/D5完整准备。

### 8.2 move/reorder

replica_local move/reorder 保持 NodeRef，需要 old/new parent、完整两侧 sibling list、cycle 所需 ancestor chain、当前 placement/policy 资格。最终 destinationOrdinal 仍指 commit 后 final index；数组输入顺序不决定结果。

并发 move/move 或 move 与 parent delete产生 placement/lifecycle conflict，不能自动挂 root或按 last writer 选位置。

### 8.3 copy 与 owner-local fresh identity

copy_node_subtree、copy_resource、copy_annotation 仅 managed_atomic。wire11 的 identityMap、
  owner rewrite、reference slot、Annotation target/reply、omission closure和 fresh identity语义全部保留。

相同 bytes、title 或 digest 不产生 identity equality。跨 owner Resource/Annotation 必须 fresh。

### 8.4 Workspace fork 与 continue

fork/continue 仅 managed_atomic。

continue 只用于同一 authority lineage 的 exclusive continuation/failover/disaster recovery，需要完整 control ledger、
  allocation/burn/tombstone、saved decisions 和旧 authority fence。Replica registration 绝不走 continue。

fork 创建 fresh WorkspaceId/AuthorityInstanceId 和所有 mapped content IDs；source 与 target没有共同提交 authority。

### 8.5 import/export re-entry

formal backup 的 continue/fork、identity-bearing bundle、
  ordinary format import 和 current Trash restore 仍按显式 artifact class区分，不从 path、digest、root UUID 或文件名猜模式。

ordinary format/worker IR 不保留 source content identity。
  partial identity bundle不在线查询 source Workspace authority。

## 9. replica registration、新设备、丢 I 与丢 P

### 9.1 新设备接入

用户把完整普通文件和可移植 metadata 带到新设备后：

1. 先验证 WorkspaceRef、portable trust、replica registry、birth/tombstone/structure records 与实际 files。
2. 通过 D6 replica registration mint 新 ReplicaEpoch。
3. 新 CommitDomain 只获得普通内容资格；已有 refs 保持。
4. 不导入旧设备 active P/WAL/SHM。
5. 不取得 Automation/ApprovalUse/Money/external unknown 的消费资格。

### 9.2 丢 I

I 可全部删除。Core 从 F/M 渐进重建 inventory/parser/search/OCR，不重新 mint identity、parent/order、lifecycle、receipt 或 conflict。

### 9.3 丢 P

某 replica 的 P 丢失后，原 OperationId decision、planned install、external unknown、approval/Money 不能从当前文件重建。

该 replicaEpoch 的 ledger continuity不可证明，因此不得继续在原 CommitDomain 建立新 decision。受权修复必须先检查 portable InstallationNotice/ContentCompletionProof 与实际 components；有未决范围时形成 recovery/conflict。

安全协调完成后可 retire 旧 ReplicaEpoch 并注册新 epoch 继续普通内容。global execution responsibility 仍保持 unavailable，直到专门 custody takeover 完整证明旧 holder 被 fence；不能因新 replica 注册自动重置额度。

## 10. 同步、并发版本与冲突

### 10.1 D6 ConflictRecord 是唯一 portable 冲突记录

D3 不建立第二 conflict database。D6 ConflictKey/ConflictId/ConflictRecord 定位 source_concurrent、placement_concurrent、
  lifecycle_concurrent、identity_collision、policy_concurrent、incomplete_transport 和 placeholder。

D3 拥有 placement/lifecycle/identity 的 typed resolution semantics；D6 conflict_prepare 的 owner_resolution 只能路由到这些 D3 语义，不接受自由 JSON patch。

### 10.2 两端编辑同一 Document

两个并发 SourceVersion/2 heads 都保留。用户可明确选择一头或提供完整 merge source；三方 Base 必须由因果前驱证明，不能按内容相同猜 Base。

merge 产生新的 ChangeId；旧 heads保留历史，不覆盖其 bytes。

### 10.3 create/move 与另一端 edit

相同 birth record 且因果链成立时，正文 edit与独立 placement change可以在重新验证 policy/structure 后组合。若 birth 或 portable metadata缺失，状态是 incomplete，不新 mint identity。

### 10.4 delete/edit

Trash 与并发 edit形成 lifecycle_concurrent。双方 payload 与 lifecycle head 都保留；显式 resolution 决定保留 Trash、恢复 live 或 fresh-copy 内容。无 delete-wins/edit-wins默认。

### 10.5 partial metadata 与 placeholder

正文先到、placement/identity后到，或反之，均是 incomplete_transport。按需下载未物化是 placeholder/not_materialized，不是空 source、not_found 或 deletion。

### 10.6 duplicate identity

相同 typed ref 出现两个不相容 birth claims 是 identity_collision。普通 resolver不选 winner。解决必须保留一条 canonical claim，并将另一分支内容显式 fresh-copy/import；不得原地改 ID 后假装同一历史。

### 10.7 ABA 与观察缺口

watcher gap、外部 replace、placeholder materialization 或同步不连续会推进 observationEpoch。相同 digest不能证明原安装归属、SourceVersion连续或 old locator current。

## 11. Resolver 与可见性

### 11.1 wire12 resolver context

所有新 resolver request 必须显式绑定 WorkspaceRef、CommitDomain 和 expected Frontier；D6 先执行 state-disclosure 与 domain qualification，再由 D3 解析 typed ref/locator。

### 11.2 Entity outcome

wire12 Entity outcome 保留 resolved、trashed、tombstoned、not_found、not_visible、workspace_unavailable、invalid，并增加三种非生命周期状态：

- conflicted：该 typed ref 当前存在未解决 identity/placement/lifecycle/source conflict，不能选择 canonical branch；
- incomplete：portable components/records未完整到达；
- placeholder：已知实体/资源存在但当前 bytes未物化。

这些状态不附另一 branch 的隐藏 bytes。读取具体 ConflictRecord 或 source仍走 D6当前授权入口。

### 11.3 Locator outcome

Locator 先传播 owner/entity 的
  conflicted/incomplete/placeholder/trashed/tombstoned/not_found/not_visible/workspace_unavailable。
  只有 canonical live owner 和 locator disclosure成立后才判断 resolved/stale/anchor ambiguous。

stale 不 fuzzy reanchor。D8 可以请求 proposal，但新 locator由 Core 针对新 SourceVersion签发。

### 11.4 non-disclosure order

顺序为：closed decode → workspace/domain binding → current state disclosure → domain/backend continuity
  → conflict/incomplete state → exact identity lifecycle → locator disclosure → revision/coordinate。

无权限时不以 conflict数量、placeholder、tombstone 或 stale状态泄露存在性。

## 12. wire12 receipt、D6 seal 与历史结果

### 12.1 identity_change_receipt v12

D3 receipt 延续 wire11 的十二 effect arrays与 per-mode exact semantics，并增加提交域绑定：

~~~json
{
  "wireVersion":12,
  "kind":"identity_change_receipt",
  "operationId":"uuid-v4",
  "targetWorkspaceRef":<WorkspaceRef>,
  "commitDomain":<D6 CommitDomain/2>,
  "changeId":<D6 ChangeId/1>,
  "guarantee":"replica_local|managed_atomic",
  "semanticState":<D6 SemanticState/1>,
  "mode":<D3Mode>,
  "identityMap":[],
  "allocated":[],
  "resultAllocations":[],
  "resultLifecycles":[],
  "initialPlacements":[],
  "trashPlacements":[],
  "preserved":[],
  "tombstoned":[],
  "rewrittenReferences":[],
  "referenceLifecycleTransitions":[],
  "structuralChanges":[],
  "omittedObjects":[],
  "result":"committed"
}
~~~

sourceWorkspaceRef、destinationOwnerRef、issuer/target authority、
  continuation、artifactClass等 mode-specific members 保留 wire11 exact matrix。

changeId.commitDomain 必须等于 receipt.commitDomain，并与 D6 seal-time receipt同一 decision绑定。D3 receipt不复制 reliableSaveState/portablePublicationState：D6 receipt固定 reliable + publication pending；publication完成后由 D6 decision-state读取 current published 状态，旧 D3/D6 receipt bytes都不改。

replica_local receipt 的 effect arrays 对本请求已证明的 local scope完整；若 SemanticState=semantic_pending，它明确**不宣称**对应 obligation 的全 Workspace closure已经枚举。managed_atomic + complete_semantics 才能满足要求全集的旧强 consumer。

### 12.2 r5 replay 与 current r6

replay 原 r5 committed request时返回原 r5 D3/D6 receipt bytes；它不能把 current r6 SourceVersion、Frontier 或 publication state写回旧 receipt。

current r6 通过 D6 current-source/portable state读取。历史 decision 与 current state是两个接口。

### 12.3 安装与 P seal

D3 只声明 planned identity/structure/lifecycle poststate；文件安装由 D6 使用 FileObjectBinding/InstallCapability 执行。hash-then-replace 不是 CAS，多文件 rename也不是跨文件瞬时原子保证。

written components 在 installed verification 与 planned poststate比较；未写 dependency仍与 before/cut 比较。撤权、third-state、不能安全恢复 before、backend unavailable 都保持 paused/conflict/recovery_unknown，不能伪造 committed。

P seal成功才有 D3 committed decision。ContentCompletionProof 发布失败时 decision仍可靠已保存但 portable publication pending。

## 13. fail-closed 顺序与错误

wire12 D3 family 保留 wire11 的 identity_not_visible、
  identity_authority_unavailable、workspace_integrity_conflict、
  operation_id_conflict、owner_mismatch、root_operation_forbidden、
  identity_not_resolvable、entity_not_live、entity_not_restorable、
  invalid_ordinal、orphan_creation、structural_cycle、invalid_locator、
  stale_locator、identity_collision、cross_workspace_identity_preservation、
  identity_map_incomplete、operation_precondition_failed、
  inbound_reference_conflict、identity_commit_aborted 等既有闭集。

新请求进入 D3 ledger前还有 D6-owned前门：

1. D6 closed decode与 current principal最低披露；
2. CommitDomain/replica/server qualification 与 backend fence；
3. open ConflictRecord、placeholder/incomplete transport、P continuity；
4. InputDescriptor/pin可取与 owner version；
5. 然后进入 D3 原 stage3 以后对应的 identity gates。

D6前门失败返回 D6 v2 error，不伪造 D3 family，也不读取 D3 ledger key。

进入 D3 后，原授权优先于存在性；authority/custody availability优先于 reachable integrity；不同 fingerprint只在 ledger continuity已证明后返回 operation_id_conflict。早门失败不探测晚门。

replica_local 不运行 proposal P1/P2/TargetLedgerCustody；managed_atomic create/fork/continue及 server authority继续运行原强门。

planned后只有证明原 plan 永不提交且安装残留安全处置完成，才能 terminal_failed + identity_commit_aborted。容量不足、撤权或恢复不确定不是这种证明。

## 14. operation-applicable 资格与 semantic_pending

### 14.1 ordinary Document edit

existing Document 普通 source edit归 D6 source-save，不使用 D3 identity_operation_request。D3只提供 owner/lifecycle/identity前提。D2必须 valid；local typed facts必须通过当前 Registry的局部门。

### 14.2 replica_local create/move/reorder/Trash

允许局部证明范围见§7–§8。成功可以返回 complete_semantics 或 semantic_pending。

semantic_pending 的 relation/unique/calendar/collection/inbound/cross_object_type obligation 不能被以下 consumer 当成完整成功：

- D7 complete Query 的负范围证明；
- result/all_result 派生写集；
- bulk/collection postcondition；
- 自动化触发的“全集无匹配/唯一”判断；
- restore/purge/copy/fork/import strong path；
- D4/D5 需要跨对象完整约束的 Action。

D4/D5 完整 owner afterimage 尚未生成时，这些 consumer 返回 owner_update_required/所属版本不可用；不得把 pending降级为空关系或合法全集。

### 14.3 managed_atomic

managed_atomic 必须证明完整 operation-applicable D3 closure和对应 D4/D5/D7依赖。缺 complete proof 时整体失败，不自动降级 replica_local。

## 15. purge、Frontier 与离线副本

purge preparation必须把当前 ReplicaRecord active set与每个 active replica已接纳 Frontier作为 D6 controlInputs；它们是完整正/负依赖。

required frontier 至少覆盖：target进入 Trash的ChangeId、所有已知 lifecycle/placement/source heads、相关 inbound/reference range proof以及当前 tombstone/allocation history。

每个 active replica必须已发布/接纳一个 head，证明它没有仍可合法提交的较旧未观察分支落在 required frontier之前。无法证明则 purge暂停/冲突，不删除payload。

管理员可显式 retire 永久离线 replica；retire 本身要经过当前 policy/trust与 execution responsibility检查。retired epoch 永不恢复 active。旧目录重新出现必须新注册 epoch并与 current tombstones协调。

这让 purge 不等待“任何可能存在但从未登记的设备”，同时也不把同步 provider 的“上传完成”误作全集证明。

## 16. 对 D4–D10 的稳定边界

### 16.1 D4/D5

D4 ref-valued fields仍用 typed refs。relation、unique、calendar 和 cross-object type 对 semantic_pending 的精确消费矩阵必须在 D4 afterimage冻结。

D5 native/bulk/collection 的完整范围同理必须版本化；D3不从部分索引猜完整性。

### 16.2 D6

D6 physical paths、portable metadata records、P/I、safe install、ConflictRecord、SourceVersion/2、
  frontier 和 execution responsibility 均按 D6 owner afterimage。D3不提供 generic patch绕过 D6，也不把 P变成 parent/order 的逻辑 owner。

### 16.3 D7

D7 Query/Action 需要新 CommitDomain/frontier/SourceVersion consumer。PreparedActionBinding/1,/2 只用于历史 saved decisions。

future D7 afterimage需要定义 wire12-compatible preparation record；在它接受前，带 preparationBinding 的新 managed request不得产生成功 decision。D3不在本文件复制一份 PreparedActionBinding/3 schema。

DefinitionTransfer、Result/9 Q segment等 D3-owned identity transfer algebra在 wire12继续使用；其 D7 payload decoder仍归 D7。

### 16.4 D8

D8 Source/Live/Read、Draft/IME/Undo/selection 和 Server collaboration checkpoint绑定 SourceVersion/2。
  显示模式切换不改 source或 identity。

### 16.5 D9

D9 import/template/export使用新 scoped pins和 CommitDomain；worker/IR ID仍不是 content identity。ImportJob unknown先恢复原 request，不能换 OperationId重做。

### 16.6 D10

D10 approval/claim/Money/sourceOccurrenceKey/stop存在 D6 ExecutionResponsibilityRecord 的连续域中。Replica registration 不复制、退款、重置或接管这些资源。

## 17. Server 多用户与 D3 序列化

Server可以同时服务多个主体和多个 Draft。D3 request的 principal/audience 来自 host/D6 protected context，不由另一会话代签。

不同 write sets可以并行 prepare；P/文件安装和 D3 decision按实际冲突关系线性化。同文档第二个旧 Base request得到 stale/conflict并保留 Draft。

未来实时协作只向 Core产生一个可验证 source/identity proposal；operation stream、presence或CRDT object都不是 NodeRef/SourceVersion/receipt。

## 18. 实现影响图

实现需要：

- D3 wire12 decoder/encoder/fingerprint；
- CommitDomain-scoped D3 ledger lookup；
- InputDescriptor/PinRef exact comparison；
- replica-local fresh ID allocation与portable birth/tombstone；
- placement/lifecycle conflict projection；
- SourceVersion/2-aware locator/evidence invalidation；
- local Trash versus managed restore/purge；
- resolver conflict/incomplete/placeholder states；
- legacy v9/v10/v11 replay router；
- D6 protocolOwner=D3 decision-state integration；
- D7/D8/D9 future consumer version gates。

不得用实现便利新增 path identity、global mutable current table、LWW、hidden ID remint或第二 receipt。

## 19. 测试与最小反例

至少覆盖：

1. 两 replica 从同一 SourceVersion 离线编辑同一 Document，两个可靠 local decisions保留并同步成 source_concurrent。
2. A create/move Node，B 编辑同一 Node source；因果可证明时组合，metadata缺失时 incomplete。
3. A Trash，B edit；不自动 delete-wins/edit-wins。
4. source 与 portable metadata 分批到达，不把缺一侧当删除或 fresh identity。
5. 两个不相容 birth claim 使用同一 typed ref，形成 identity_collision，普通 resolver不选 winner。
6. A→B→A + observation gap，使旧 locator/prepared/evidence stale，即使 digest相同。
7. 文件安装在每个 stage崩溃；第三状态不覆盖，P seal后 publication未完仍显示 reliable + pending。
8. 删除 I 可重建；丢 P 不重建旧 decision/unknown/Money。
9. r5 committed replay返回 r5原 bytes，current r6另读。
10. Server同文档两用户 Draft并存，先提交者产生新版本，后者旧 Base不覆盖。
11. Server不同文档 prepare并发，不因无关 commit sequence误 abort。
12. wire9/wire10/wire11 saved decision按原 decoder重放；新 request只接受wire12。
13. partial index不能让 strong bulk/restore/purge或 complete Query通过。
14. replica registration 保持WorkspaceId/refs且不调用continue。
15. duplicate local UUID claim同步后显式 fresh-copy losing branch，不静默rekey。
16. purge在active replica frontier缺失时暂停；retired旧副本重入只能新ReplicaEpoch，tombstone不复活。
17. D2-invalid外部 bytes可repair/read但无成功D3 author receipt。
18. hash相同、FileObject provenance不同不能证明安装属于原plan。
19. revoked planned request不seal；已sealed request撤权只遮蔽交付，不改decision。
20. ContentCompletionProof失败不回滚sealed receipt，后续可恢复publication。
21. same OperationId在两个 CommitDomain独立合法；同domain不同fingerprint固定conflict。
22. semantic_pending不能被D7 all_result、Automation或unique negative当complete。
23. managed_atomic缺完整range时不得自动降级replica_local。
24. D7 Prepared1/2历史恢复不被wire12重新解释；Prepared3未定义时新binding路径明确不可用。
25. no body bytes永久嵌入wire12 canonical request；InputDescriptor+purpose pins完整比较且hash-only不足。

## 20. legacy compatibility 与联合激活

D3-CJ/3、D3Integer、Ref/Locator词法、Annotation Value/3、Result/9及既有 typed identity algebra保持。wire12只改变新决议的 domain/frontier/input/guarantee 和局部/强资格。

旧 v9/v10/v11 saved artifacts保留原 bytes、原 pin retention、原 authority/custody gates、原 receipt/error/outcome。它们不能被转换成 wire12 来获得 replica-local语义。

D3 wire12 当前只与已生成 D6-FA-r01 owner afterimage闭合。D4/D5/D7/D8/D9/D10 consumer afterimage未完成，因此本候选不能半包激活或生成“联合 v12 已通过”的产品 fixture。

## 21. D7 preparation 与定义转移

### 21.1 PreparedActionBinding ownership

PreparedActionBinding 当前规范 owner仍是 D7。固定 S 中 D3 §21B 对 /2 的全文镜像在本后像中退为历史来源，不再作为第二份可漂移 schema。

wire9/v10/v11 saved decisions继续按其原镜像/decoder履约。新 wire12 的 preparationBinding 外层仍只保存 kind + bindingToken；它不声明 backing record版本。

在 D7 wire12 consumer afterimage接受前，任何新 request携 preparationBinding 都在进入 D3 ledger前由 D6/D7 capability gate返回 owner_update_required。D3不猜 /3 字段、不把 /2自动升级，也不建立第二 preparation ledger。

### 21.2 DefinitionTransfer 与 Result/9

DefinitionTransfer、definitionTransfers 数组、Result/9 Q segment、
  D3-Symbolic-Result/9 B/M/N/E/S/C/Q partition继续由 D3 identity mutation algebra拥有；D7拥有 SavedQuery/View/DynamicBlock payload schema。

copy/fork/identity-bearing import若涉及这些已识别 payload，仍须完整 typed traversal、source/result occurrence binding和 candidate map物化；普通 text/CEL中的UUID字符串不当Ref。

### 21.3 exact input 与 pin lifetime

新 D7/D3 preparation最终必须生成 D6 InputDescriptor/2：canonical owner descriptor中大型 source/resource字段换成 typed PinRef，canonical request保持小型。

planned/unknown/conflict/approval-money最后引用保护的 pins不能按preview TTL删除；可失效历史 effects由D6/D7新版本明确定义。旧协议承诺永久/decision耐久的pins不追溯删除。

## 22. 候选接受边界

本文件是 D6-FA-r01 的 D3完整 owner afterimage，不是独立接受。D3术语与实施后像必须与本文同步，后续 D4/D5/D7/D8/D9/D10 owner必须消费新版本后才能形成不可变联合候选。

旧 D10 B13 继续是 REVISE、术语/双语 FAIL、3 P1 + 8 P2 共 11 OPEN；本文件不关闭任何项。
