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

D6 唯一拥有 DecisionKey/2、CommitDomain/2、ReplicaEpoch、ChangeId/1、Frontier/2、SourceVersion/2、SourceObservation/1、SourceVersionRef/1、ObservationScope/2、DependencyProof/2、OwnerInputBinding/2、InputDescriptor/2、PreparedIntent/2、D3DecisionCompanion/2、文件安装、P seal、ContentCompletionProof/2、ConflictRecord 和 execution responsibility。D3 只消费这些 exact 类型，不重新定义同名对象或建立第二可写真相。

D2 exact source、D4 typed schema/relations、D5 structure、D7 Query/Action、
  D8 editor、D9 conversion 和 D10 external execution 仍由各 owner 定义。

## 2. 设计目标

1. 物理文件路径、索引行、mtime、digest、同步 provider ID 都不是内容 identity。
2. 同一逻辑 Workspace 的多个已登记副本保留同一 durable refs，同时允许各自离线普通内容操作。
3. 普通副本内容资格与 global execution responsibility 分离。
4. parent/order、lifecycle、tombstone 和 no-reuse 事实必须可移植，不能只在可删 I。
5. replica_local 只缩小本次 D3 语义证明范围；所有 D3 identity/structure/lifecycle 作者安装仍固定 WriteProtection=strict。它不冒充所有副本联合当前态或 D7 全集证明。
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

### 4.1 D6 类型消费与 D3-native owner input

wire12 直接消费并按 D6 owner decoder 验证 DecisionKey/2、CommitDomain/2、Frontier/2、ObservationScope/2、DependencyProof/2、SourceObservation/1、OwnerInputBinding/2 和 InputDescriptor/2。D3 不接受私有近似对象，也不把 D6 类型复制成第二 owner。

InputDescriptor/2.ownerInput 必须满足 protocolOwner=D3、ownerKind=d3_identity_operation/12。canonicalDescriptorBytes 解码为 D3 自有闭合描述符：

~~~json
{
  "kind":"d3_identity_input",
  "version":12,
  "mode":<D3Mode>,
  "writeProtection":"strict",
  "expectedAuthority":<ExpectedAuthority?>,
  "workspaceProposal":<WorkspaceAllocationProposal?>,
  "intent":<D3Intent>
}
~~~

带 ? 的成员只在原 mode 矩阵允许时出现，不适用时必须省略。preparationBinding 属于 D7 交叉 owner 输入，不进入这个 D3-native descriptor；它仍由 wire12 顶层和未来 D7 准备记录独立绑定。ownerInput.pinRefs 只承载 D3 descriptor 中大型不可变输入的 typed slot；完整 source/currentness 仍通过 InputDescriptor sourceInputs 的 SourceObservation/1 与受保护 pins 证明。

CommitDomain.replica 的 replicaEpoch 必须是当前 active ReplicaRecord；CommitDomain.server 的 authorityInstanceId 必须等于当前托管 authority。D6 先证明 domain、backend、P continuity 和当前 observation/dependency 资格，D3 才访问 identity ledger。

### 4.2 DecisionKey/2 与 OperationId ledger

新 wire12 的唯一持久决议键是 D6 DecisionKey/2。其 workspaceRef 必须 byte-equal boundWorkspaceRef，commitDomain 与 operationId 必须等于请求同名成员。P 的实际索引仍为：

    (boundWorkspaceRef.workspaceId, D3-CJ/3(commitDomain), operationId)

protocolOwner=D3 表示 D3 是该键的主 decision owner；它不进入 DecisionKey，也不允许同一键再形成 D6 主 decision。相同 Workspace 与 OperationId 在不同 CommitDomain 中是独立键；相同 DecisionKey 只能有一个 canonical request fingerprint 和一个 decision。

状态仍为 unseen → rejected 或 unseen → planned → committed | terminal_failed。rejected、committed、terminal_failed bytes 不可改写；planned 只能恢复原计划。不同 fingerprint 使用同一键固定 operation_id_conflict，不读取或回显原请求内容。

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
  "expectedFrontier":<D6 Frontier/2>,
  "inputDescriptor":<D6 InputDescriptor/2>,
  "mode":<D3Mode>,
  "expectedAuthority":<ExpectedAuthority?>,
  "workspaceProposal":<WorkspaceAllocationProposal?>,
  "preparationBinding":<PreparationBinding?>,
  "intent":<D3Intent>
}
~~~

inputDescriptor.workspaceRef、commitDomain、expectedFrontier、guarantee 必须与顶层逐字 canonical-equal；intentKind 必须是 D3 identity operation v12。ownerInput 必须逐字等于 §4.1 的 D3-native descriptor；其中 writeProtection 固定 strict，任何 observed_only 值都在 D3 ledger 访问前拒绝。

InputDescriptor.sourceInputs 绑定完整 SourceObservation/1，而不是裸 SourceVersion 或 revision。controlInputs 与 dependencyProof 使用 D6 closed DependencyKey/stamp。大型 Document/Resource/Annotation bytes 只能由 typed PinRef/2 slot承载，canonical request 不永久内嵌完整正文或附件。

完整输入相等比较 InputDescriptor、D3-native owner descriptor、每个 exact pin、SourceObservation 和 DependencyProof。相同 sha256 不证明相同输入，也不能证明 ABA 连续。

### 4.4 requestFingerprint

D3-CJ/3 与 SHA-256 规则保持不变。fingerprint body 包含除顶层 operationId 外的全部规范请求字段；workspaceProposal 仍以完整 canonical digest 进入。

任一 commitDomain、guarantee、expectedFrontier、frontierPolicy、ObservationScope、SourceObservation、DependencyProof、owner descriptor、authority expectation、proposal、preparationBinding、intent 或 plan 变化都改变 fingerprint。strict 不能通过修改旧 plan 降级为另一请求。

### 4.5 guarantee、Frontier policy 与 mode 矩阵

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

所有格都固定 WriteProtection=strict。replica_local 表示 local semantic scope，不表示弱文件安装；它不得使用 observed_only。

replica_local 的 create_node、move_node、reorder_node、trash 固定使用 frontierPolicy=scope_dependencies 和 ObservationScope/2 local_structure。无关 Frontier/2 扩展只能在原 source/control/auth 正负依赖逐项重验后继续，不能重选 subject、parent、ordinal、closure 或 proposed after。

所有 managed_atomic D3 mode 固定 frontierPolicy=exact。create_workspace 与 fork_workspace 使用 prepared_workspace 观察上界；其余 managed_atomic identity/lifecycle mode 使用 workspace_constraints。完整正负范围、当前权限和 owner 版本仍按原强门证明。

replica_local 禁止 workspaceProposal、expectedAuthority 与 preparationBinding。managed_atomic 沿用 wire11 expectedAuthority 模式：create/fork 使用 create；continue 使用 continue；其余使用 existing。create/fork 继续使用 proposal P1/P2 与 TargetLedgerCustody，server CommitDomain 还必须与 expectedAuthority 当前实例一致。

D7-mediated wire12 preparation 仍等待 D7 owner afterimage。preparationBinding 的 outer token wrapper 保留；在 D7 consumer完成前，它返回 owner_update_required 且零 D3 ledger decision。D3-native typed operations可以省略 preparationBinding。

### 4.6 allocation 与 UUID 不复用

同一个 CommitDomain 内，Core 在 P 的 allocation history 上 reserve/burn fresh IDs。
  portable metadata 保存 accepted birth、tombstone 和 no-reuse facts。

两个离线 replica 极低概率独立 mint 同一完整 typed ref 时，不按“先同步者”或 hash 自动选 winner。接收端形成 identity_collision；在冲突解决前没有联合 canonical entity。显式解决可保留一个 birth claim，并把另一分支内容通过 fresh-copy/import 产生新 ID。losing claim 永不被静默 rekey。

已 accepted tombstone 的 ID 永不复用。burned reservation 也不能在同一提交域再次分配。

### 4.7 SourceObservation/1、SourceVersion/2 与 ABA

D3 Locator、selector 和 prepared evidence 的 currentness 绑定完整 D6 SourceObservation/1。SourceVersion/2 仍表示实际来源版本及其生产 CommitDomain；生产 domain 可以与当前 operation 的 observerDomain 不同，不能因二者不同误报 stale 或 domain mismatch。

当前 operation 只要求 SourceObservation.observerDomain 等于 operation CommitDomain、entityRef 与实际 owner一致，并验证完整 FileObjectBinding、observationEpoch 与 evidence pins。D4/D5 后续 source-bearing inner revision也必须对应该 observation 内实际 SourceVersion，而不是按“revision数字+当前domain”猜版本。

SourceVersionRef/1 或任何 expectedSourceToken 都必须选择整个受保护 SourceObservation。相同生产 SourceVersion 在 observationEpoch 变化、watcher gap、external replace 或不连续 rematerialize 后不再是旧输入；A→B→A 即使最终 bytes/digest 相同，也不能恢复旧 locator、preparation 或 ActionEvidence。

sourceOccurrenceKey 的更高层连续性仍由 D10 owner决定，D3 不按 path/hash猜测。

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

D3 primary receipt 保留 wire11 的十二类 effect arrays 和各 mode 的精确语义。
同一 DecisionKey 还增加 effect class；portable variant 的闭合形状如下：

~~~json
{
  "wireVersion":12,
  "kind":"identity_change_receipt",
  "operationId":"uuid-v4",
  "targetWorkspaceRef":<WorkspaceRef>,
  "commitDomain":<D6 CommitDomain/2>,
  "effectClass":"portable",
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

sourceWorkspaceRef、destinationOwnerRef、issuer/target authority、continuation、artifactClass 等 mode-specific members 继续使用 wire11 的精确矩阵。
effectClass 的闭合取值为 portable|control_only|no_op。
portable 必须带 changeId 和 semanticState，ChangeId 只在 seal 时分配。
control_only/no_op 省略 changeId 与 semanticState；同一 decision 的 D6 sourceVersions 为空，Frontier/2 不推进。
no_op 的十二类 effect arrays 全空；equal-byte external managed admission 不是 no_op。

所有 D3 mode 固定 WriteProtection=strict。D3 receipt 不复制 ReliableSaveState、InstallationState 或 PortablePublicationState；这些由 D6 decision state 给出。
portable D6 effect metadata 只列真实改变 source 的 SourceVersionRef/1。纯 placement/lifecycle metadata change 可以没有 sourceVersions，但仍获得 content ChangeId。

D3 primary receipt 与 D3DecisionCompanion/2 在同一个 P seal transaction 保存。
companion 的 DecisionKey 等于本 request，domainCommitSequence 是该 CommitDomain 的决议序，effectsToken 绑定同一 decision。
它不是第二个成功 receipt，也不建立第二 ledger。

replica_local receipt 的 effect arrays 对实际证明的 local scope 完整；SemanticState=semantic_pending 不宣称全 Workspace closure。
只有 managed_atomic + complete_semantics 才满足旧强 consumer。

### 12.2 r5 replay 与 current r6

replay 原 r5 committed request时返回原 r5 D3/D6 receipt bytes；它不能把 current r6 SourceVersion、Frontier 或 publication state写回旧 receipt。

current r6 通过 D6 current-source/portable state读取。历史 decision 与 current state是两个接口。

### 12.3 严格安装、P seal 与发布

D3只声明planned identity/structure/lifecycle poststate；所有D3 mode的installationPlan固定WriteProtection=strict。D6使用strict create_only、conditional_replace或真正exclusive_write_window安装portable components；D3不接受observed_replace，也不把人工普通source-save的observed_only资格带入结构/lifecycle。

written components与planned poststate比较；未写dependency仍与before/cut比较。撤权、third_state、已观察竞争、不能安全恢复before、backend unavailable或安装归属不明都保持paused/conflict/recovery_unknown，不能伪造committed。

P seal是唯一D3 author decision commit point。portable effect在该transaction内才分配ChangeId/SourceVersion，并同时保存D3 primary receipt、D3DecisionCompanion/2、D6 decision/effects state和适用charge/outbox。control_only/no_op不分配content ChangeId。同一DecisionKey重放只返回原保存结果，不重装source/metadata、不增加domainCommitSequence，也不重复ApprovalUse/Money。

只有portable effect生成ContentCompletionProof/2并推进Frontier/2。proof发布失败时committed decision不回滚，PortablePublicationState=pending；恢复只发布同一proof，不重新运行D3 plan。control_only/no_op publication为not_applicable。

## 13. fail-closed 顺序与错误

wire12 保留 wire11 的 D3 错误 family。
第一组前半为 identity_not_visible、identity_authority_unavailable、workspace_integrity_conflict。
第一组后半为 operation_id_conflict、owner_mismatch、root_operation_forbidden。
第二组前半为 identity_not_resolvable、entity_not_live、entity_not_restorable、invalid_ordinal。
第二组后半为 orphan_creation、structural_cycle、invalid_locator、stale_locator。
第三组前半为 identity_collision、cross_workspace_identity_preservation、identity_map_incomplete。
第三组后半为 operation_precondition_failed、inbound_reference_conflict、identity_commit_aborted。

访问 D3 DecisionKey 前先走 D6 前门：
1. closed decode、当前主体最低披露、ObservationScope/2 资格；
2. CommitDomain、backend fence、P continuity、protocolOwner=D3；
3. OwnerInputBinding/2 必须是 d3_identity_operation/12，且 descriptor/request cross-field equality 成立；
4. SourceObservation/1、DependencyProof/2、pins、owner version 可取；
5. exact 要求 Frontier/2 完整相等；scope_dependencies 只允许无关非回退扩展并重验原依赖；
6. ConflictRecord、placeholder/incomplete transport 或 availability 问题不能伪装成业务 rejection；
7. 全部通过后才进入 D3 原 identity gates 与 DecisionKey lookup。

D6 前门失败使用 D6 v2 closed error。
其中 not_visible、domain_unavailable、integrity_conflict 属于前门结果。
source_unavailable、proof_unavailable、dependency_conflict、install_unavailable 也属于 D6 闭合结果。
这些错误不发明 D3 family，也不读取 D3 ledger。
D3-native owner 已在 G0-B 闭合；缺 D4/D5/D7 consumer 的强路径仍可在其版本门返回 owner_update_required。

进入 D3 后，原授权优先于存在性，authority/custody availability 优先于 reachable integrity；不同 fingerprint 只在 ledger continuity 已证明后返回 operation_id_conflict。
replica_local 不运行 proposal P1/P2/TargetLedgerCustody；managed_atomic 强门保持。
scope_dependencies 只重验原 request/dependency scope，不重选 identity、placement、closure 或 after。

planned 后只有证明原 plan 永不提交且安装残留安全处置完成，才能 terminal_failed + identity_commit_aborted。
容量不足、撤权、unknown install 或恢复不确定都不是这种证明。

## 14. operation-applicable 资格与 semantic_pending

### 14.1 ordinary Document edit

existing Document 普通 source edit归 D6 source-save，不使用 D3 identity_operation_request。D3只提供 owner/lifecycle/identity前提。D2必须 valid；local typed facts必须通过当前 Registry的局部门。

### 14.2 replica_local create/move/reorder/Trash

允许局部语义证明范围见§7–§8。replica_local只缩小依赖范围，所有create/move/reorder/Trash安装仍必须WriteProtection=strict；observed_only只属于D6人工单Document source-save，不能被D3 request选择。成功可以返回complete_semantics或semantic_pending。

semantic_pending 的 relation/unique/calendar/collection/inbound/cross_object_type obligation 不能被以下 consumer 当成完整成功：

- D7 complete Query 的负范围证明；
- result/all_result 派生写集；
- bulk/collection postcondition；
- 自动化触发的“全集无匹配/唯一”判断；
- restore/purge/copy/fork/import strong path；
- D4/D5 需要跨对象完整约束的 Action。

D4/D5既有afterimage尚未消费G0-A/G0-B的SourceObservation/Frontier/WriteProtection合同；C批前这些consumer返回owner_update_required/所属版本不可用，不得把pending降级为空关系或合法全集。

### 14.3 managed_atomic

managed_atomic 必须证明完整 operation-applicable D3 closure和对应 D4/D5/D7依赖。缺 complete proof 时整体失败，不自动降级 replica_local。

## 15. purge、Frontier/2、物化范围与离线副本

purge preparation 把 current ReplicaRecord active set、各 active replica 已接纳 Frontier/2、相关 SourceObservation/1 和完整 DependencyProof/2 作为 D6 受保护输入。
Frontier/2 只证明 sealed 因果前缀，不证明 payload 物化、placeholder 下载、inbound 扫描或全集约束完成。

required frontier 至少覆盖 target 进入 Trash 的 ChangeId。
它还必须覆盖已知 lifecycle/placement/source heads 以及 tombstone/allocation history。
真正 purge 还必须对相关 source/lifecycle/placement/inbound/reference scope 取得实际物化与完整正负证明。

即使 Frontier 数值满足，required payload 不可读或 DependencyProof 不完整时，purge 仍暂停/不可用，零 payload deletion。
同步 provider 的“已完成”不能替代这些证明。
管理员可按当前 policy/trust 和 execution responsibility 显式 retire 永久离线 replica；retired epoch 不复活。
旧目录重入使用新 ReplicaEpoch，并与当前 SourceObservation、tombstone、ConflictRecord 协调。

生产 SourceVersion 的 CommitDomain 可以与当前 purge observerDomain 不同；只要 SourceObservation 当前有效就不能误拒绝。
相反，相同生产版本在 observationEpoch 变化后旧输入必须失效。

## 16. 对 D4–D10 的稳定边界

### 16.1 D4/D5

D4 ref-valued fields仍用 typed refs。relation、unique、calendar 和 cross-object type 对 semantic_pending 的精确消费矩阵必须在 D4 afterimage冻结。

D5 native/bulk/collection 的完整范围同理必须版本化；D3不从部分索引猜完整性。

### 16.2 D6

D6 的 physical paths、portable metadata records、P/I 与 strict safe install 继续由 D6 拥有。
DecisionKey/2、Frontier/2、SourceVersion/2、SourceObservation/1、DependencyProof/2、D3DecisionCompanion/2 由 D6 owner afterimage 定义。
ConflictRecord 和 execution responsibility 同样归 D6；D3 不提供 generic patch 绕过 D6，也不把 P 变成 parent/order 的逻辑 owner。

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
- DecisionKey/2 + protocolOwner=D3 的单一 P ledger lookup；
- d3_identity_operation/12 OwnerInputBinding 与 InputDescriptor/PinRef exact comparison；
- Frontier/2 policy、ObservationScope/2、SourceObservation/1 与 DependencyProof/2 currentness；
- replica-local fresh ID allocation与portable birth/tombstone；
- placement/lifecycle conflict projection；
- SourceVersion/2-aware locator/evidence invalidation；
- local Trash versus managed restore/purge；
- resolver conflict/incomplete/placeholder states；
- legacy v9/v10/v11 replay router；
- D6 protocolOwner=D3 decision-state integration、同 P D3DecisionCompanion/2 与 effectClass 分支；
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
