---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：04f16e0f-a8ca-4d4a-8d75-007e46d44975。

# D3 Implementation Impact and Test Outline

候选状态：D6-FA-r01；部分联合候选；未接受、未激活、未实现。固定 S 的 D3 Impact 作为历史来源；本文件只定义未来实施与验收，不授权源码、依赖、发布、部署或 A2。

## 1. 实施切片

| 切片 | 新版职责 | 保留边界 |
|---|---|---|
| wire12 | G0-B 闭合 D3 请求与输入，消费 DecisionKey/2、Frontier/2、InputDescriptor/2 与 D3-native owner input | D3-CJ/3、D3Integer、Ref/Locator 词法保持不变；receipt/companion 仍按真实 owner |
| scoped ledger | D6 DecisionKey/2 + protocolOwner=D3，同一 P 仅一个主 decision | v9/v10/v11 saved ledger原样重放 |
| replica_local | create_node、move、reorder、Trash 的局部语义证明；安装仍 strict | 不升级为 observed_only 或完整全集证明 |
| managed_atomic | restore/purge/copy/fork/continue/import与完整 closure | 不自动降级 local |
| resolver | conflict/incomplete/placeholder 与完整 SourceObservation/1 currentness | 不按裸 revision、生产 domain 或相同 hash 猜 current |
| portable identity | birth/parent-order/lifecycle/tombstone/no-reuse | path/I/P不是逻辑owner |
| preparation | D7 backing schema仍由D7 owner | 不复制 PreparedActionBinding/3 |
| sync conflicts | D6 ConflictRecord + D3 typed resolution | 无 LWW、mtime winner |

## 2. decoder 与版本路由

实施必须先按 wireVersion 分流：

- v9/v10/v11：仅已保存 decision、planned recovery、receipt/error/outcome replay；禁止新 decision，原 bytes、原 fingerprint、原 pin retention、原 authority/custody 连续性全部保留。
- v12：只接受 G0-B 与 A 的 D6 owner afterimage 共同定义的 request/owner-input shape。
- 其它版本：unsupported wire，不猜兼容。

v12 decoder 必须先调用 D6 owner decoder 验证 DecisionKey/2、CommitDomain/2、Frontier/2、ObservationScope/2、DependencyProof/2、OwnerInputBinding/2、InputDescriptor/2 和 SourceObservation/1。
ownerInput 必须是 protocolOwner=D3、ownerKind=d3_identity_operation/12；descriptor 中 writeProtection 固定 strict。
随后分两组检查 cross-field equality：
- Workspace、domain、guarantee、expectedFrontier；
- frontierPolicy、observationScope、owner descriptor 与 request。

same OperationId 在两个 CommitDomain 可以独立存在；同一个 DecisionKey 上不同 protocol owner 或不同 fingerprint 固定 operation_id_conflict。

## 3. canonical request 与 pins

wire12 canonical request不能永久嵌入完整 Document/Resource/Annotation bytes。D3-native owner descriptor固定为 d3_identity_operation/12，只保存 mode、strict WriteProtection、适用 expectedAuthority/workspaceProposal 与 intent；大型不可变输入只用 typed PinRef slots。Source currentness 另由 InputDescriptor.sourceInputs 的完整 SourceObservation/1 绑定。

实施测试必须证明：

1. 相同 InputDescriptor canonical bytes但任一 exact pin、SourceObservation 或 DependencyProof不同→不是同输入。
2. 仅 sha256相同、生产 SourceVersion相同但 observationEpoch 或 FileObjectBinding不同→不能恢复原输入。
3. planned恢复复用原 pin/observation/dependency，不重新读取 current source后伪造“等价 request”。
4. old v11 saved request保持原 bytes，不迁移到 InputDescriptor/2。
5. pin cleanup服从 D6 last-reference/retention，不能删除 planned/unknown/conflict/approval-money所需证据。
6. strict request不能通过修改 owner descriptor、frontierPolicy 或 plan 原地降级。

## 4. replica registration 与 control loss

### 4.1 新设备

正式 fixture：Workspace W 的 portable F/M 完整到设备 B，B 没有 A 的 P/I。B 验证 portable metadata 后注册 fresh ReplicaEpoch，保留 W 及全部 refs。B 可在新 CommitDomain普通编辑，但 execution responsibility remains unavailable。

反例：把 B 当 continue_workspace，或复制 A 的 control.sqlite3/WAL/SHM后让 A/B同时消费同一 approval/Money。

### 4.2 丢 I

删除 I 后：

- ref/resolver/parent/order/lifecycle从 F/M恢复；
- Query/search progressive rebuild；
- 不生成新 birth、receipt、tombstone；
- 不重置 ChangeId/Frontier。

### 4.3 丢 P

删除/损坏 P 后：

- old replicaEpoch 不得继续发新 decision；
- portable InstallationNotice圈定可能的未决范围；
- current files不能重建 original decision/unknown/Money；
- 受权修复后 retire旧 epoch并注册新 epoch；
- execution responsibility需要独立 takeover proof。

## 5. replica_local 资格矩阵

replica_local 只改变 D3 的语义证明范围，不改变安装保护级别。create_node、move_node、reorder_node、Trash 的 owner descriptor 一律 WriteProtection=strict，frontierPolicy=scope_dependencies，ObservationScope/2=local_structure。observed_only 只属于 D6 人工单一既有 live Document source-save，任何 D3 mode 选择它都必须在 ledger 前失败。

### 5.1 existing source edit

existing Document 普通正文保存仍由 D6 source-save owner测试；D3只提供 current live owner/identity条件。D2-invalid proposal拒绝普通 save；external invalid bytes进入repair读路径。narrow Field/body 权限不能通过 D3 action 或 observed_only 获得整文件替换。

### 5.2 create_node

正例证明分为三组：
- active replica、current parent/ancestor、完整 destination sibling list；
- fresh allocation、D2 valid、local typed facts；
- strict installation、portable birth+placement、当前 SourceObservation/DependencyProof。

负例：
- hidden/missing sibling range；
- parent处于 conflict/placeholder；
- fresh ID与remote birth冲突；
- local typed fact invalid；
- strict backend qualification不可用；
- 无关 Frontier/2 扩展后原 dependency重验失败。

### 5.3 move/reorder

正例覆盖 same-parent、cross-parent、no-op final index、compound movement。完整 old/new sibling lists、ancestor cycle proof、当前权限与原 scope_dependencies 范围都必需。

负例分为两组：
- duplicate final index、cycle、hidden sibling、concurrent move head；
- parent lifecycle conflict、observationEpoch变化或已观察竞争。

### 5.4 Trash

正例必须同时覆盖：
- 完整 local subtree/owner/reply membership 与 Trash sibling order；
- policy、strict metadata installation、相关 SourceObservation/DependencyProof。

若 inbound全集未证明，SemanticState必须 semantic_pending(inbound)。receipt不能列伪造的 global referenceLifecycleTransitions全集。已观察 Trash/edit 竞争必须显式 conflict，不得把 observed_only 当 delete-wins/edit-wins。

## 6. managed_atomic 强门

managed_atomic 所有 D3 mode 固定 WriteProtection=strict、frontierPolicy=exact，并使用完整 observation/dependency scope。fixture至少覆盖：

- full Trash with complete inbound range；
- restore original/explicit location；
- purge with replica frontier；
- same-Workspace copy；
- owner-local copy；
- cross-Workspace transfer；
- fork；
- continue/failover；
- identity-bearing import；
- ordinary import。

任一完整 range、payload materialization、authority/custody、D4/D5/D7 owner version、SourceObservation/DependencyProof 或 pin continuity缺失时失败，不降级 replica_local，也不改用 observed_only。

## 7. sync/conflict 矩阵

必须建立以下 concurrent fixture：

| 场景 | 预期 |
|---|---|
| source/source | source_concurrent，保留两heads |
| create/create不同ID | 两个birth均保留；同parent顺序可形成placement conflict |
| create/create同typed ref | identity_collision，无自动winner |
| move/edit | 因果可证明且无冲突时组合，否则显式 conflict |
| move/move | placement_concurrent |
| Trash/edit | lifecycle_concurrent |
| Trash/restore | lifecycle_concurrent或强门重证 |
| body先到metadata后到 | incomplete_transport |
| metadata先到body后到 | incomplete_transport/placeholder |
|按需下载未物化 | placeholder，不是not_found |
| policy两端变化 | policy_concurrent，禁止 grant union |
| ABA watcher gap | observationEpoch推进，旧evidence失效 |

ConflictId稳定性还要验证：相同完整 ConflictKey产生同ID；新增 head产生新 record并supersede旧 open/prepared record；resolved历史不原地改写。

## 8. identity collision 与 no-reuse

离线 replica的 UUIDv4 独立 mint collision必须有确定验收：

1. 两个 birth claim 均已本地 reliable。
2. 同步后无 canonical winner。
3. resolver返回 conflicted。
4. 解决入口明确选择一条 claim。
5. losing payload通过 fresh-copy/import取得新 ID。
6. 原 collision ref不被历史重写。
7. tombstoned ID永不复活。
8. retired replica带回旧bytes不能恢复旧 birth为live。

测试不能以“概率极低”代替语义。

## 9. purge Frontier/2、物化与 tombstone

purge fixture必须同时锁定：
- current replica registry revision；
- active replica set；
- required Frontier/2；
- target Trash ChangeId；
- 已知 source/lifecycle/placement heads；
- 相关 SourceObservation/1；
- inbound/reference complete DependencyProof/2；
- D4/D5 complete obligations；
- allocation/tombstone history。

Frontier/2 只证明 sealed causal prefix。少一个 active replica causality head→暂停/冲突；即使 head齐全，只要 placeholder未物化、required bytes不可读或 negative DependencyProof不完整，也必须暂停/不可用并保持零 payload deletion。

production SourceVersion.commitDomain 与当前 purge observerDomain可以不同；完整当前 SourceObservation 仍有效。相同生产版本但 observationEpoch变化时旧 input必须失效。

retire replica后可重新评估 purge；retired epoch不可重新 active。旧文件后来接入必须 fresh ReplicaEpoch，并由 current SourceObservation、tombstone、ConflictRecord阻止原 ref复活。从未登记设备不进入 active set；同步服务“已完成”不算 causal/materialization ack。

## 10. resolver 与 privacy

新 resolver测试覆盖每个 typed entity的：

resolved | trashed | tombstoned | conflicted | incomplete |
  placeholder | not_found | not_visible | workspace_unavailable | invalid。

无 state-disclosure时上述存在性差异全部遮蔽。locator只有 canonical live entity + locator disclosure后才进入 resolved/stale/anchor ambiguity。

同样 bytes/span/token 在 observationEpoch变化后不得恢复旧currentness。

## 11. D6 严格安装、同 P seal 与 D3 决议

所有 D3 identity/structure/lifecycle mode 都固定 WriteProtection=strict。故障注入覆盖：

1. P planned前崩溃；
2. pins/SourceObservation/DependencyProof durable后 planning事务未知；
3. InstallationNotice/2 前；
4. notice后、第一 component 前；
5. 每个 staging/flush/install/directory-flush 点；
6. 已观察 competing state；
7. installed verification；
8. P seal前；
9. seal后、ContentCompletionProof/2 前；
10. proof写入/flush中；
11. response delivery丢失。

每格只允许 D6 定义的 exact_before、exact_after_with_provenance、third_state、unavailable 与 D3 decision组合。已观察 race 固定 conflict/paused；是否安装或 provenance不明固定 recovery_unknown，不得通过相同 hash 猜 success。

written target核对 planned poststate；未写 dependency核对 original cut。strict 请求不能原地降级 observed_only，D3 也不能调用人工 source-save 的弱路径完成结构或 lifecycle effect。

P seal 是唯一 author-decision commit point：portable effect 此时才分配 ChangeId/SourceVersion，并在同一 transaction 保存 D3 primary receipt、D3DecisionCompanion/2、D6 state/effects和适用 charge/outbox。third_state 保留现文件/pins，不能覆盖或盲回滚。seal 后 publication失败不 terminalize已提交 decision，恢复只补同一 proof。

## 12. r5/r6、DecisionKey replay 与 no-op

golden sequence：
- O5 以 DecisionKey K portable committed，response丢失；
- O6 随后提交更新；
- retry O5 exact original request。

唯一结果：O5 返回原 D3 primary receipt 与同一 D3DecisionCompanion/2 绑定；current state另读 O6。不得重新安装 source/metadata，不再分配 ChangeId，不再次增加 domainCommitSequence，不重复 ApprovalUse/Money charge，也不倒退版本。

撤权时 O5 replay可以 not_visible，但 decision不改变；重获授权只恢复原 decision 交付。

另测三种 effectClass：
- portable：真实 F/M author change，seal时分配 ChangeId并推进 Frontier/2；sourceVersions只列真实改变 source。
- control_only：只有 P 控制变化，无 content ChangeId、无 Frontier推进、sourceVersions为空。
- no_op：十二类 D3 effect arrays全空，安装/保存/发布均 not_applicable；可有一次 decision序号，但没有 content ChangeId/source revision/Frontier推进。

外部同字节状态被显式接纳为 managed 状态时，来源/控制状态发生变化，不能伪装成 no_op。

## 13. Server 多用户

至少测试：

1. A/B不同文档同时 prepare；两边可完成。
2. A/B同文档同 Base；A先seal，B保留Draft并stale/conflict。
3. B撤权发生在B checkpoint前；B不能提交。
4. A已seal后A撤权；历史decision不回滚，后续交付按授权遮蔽。
5. Server restart/failover同时 fence P与file writer。
6. 旧实例只能读或拒写，不能rename author files。
7. 后端SQLite writer串行不阻断前端多个Draft/session。
8. 两个离线副本分别 move/Trash 同一结构范围时，保留并发 heads 并显式 conflict，不做 LWW。
9. P/I schema审计必须证明没有全库 current body/AST 镜像，也没有第二 parent/order authority。

实时协作算法尚未实现；相关测试只能是未来接口契约，不宣称通过。

## 14. semantic_pending consumer gate

D4/D5 既有 afterimage 尚未消费 G0-A/G0-B 新 source/observation/frontier 合同；C 批完成前，以下测试固定拒绝 pending 作为完整证明：

- relation “没有任何目标”；
- unique “全库唯一”；
- Calendar完整展开；
- collection membership完整postcondition；
- inbound全集；
- cross-object type closure；
- D7 all_result/bulk；
- Automation依赖完整Query。

普通 source显示、Draft、局部编辑和明确branch读取可以消费 pending，但UI必须显示其未证明obligations而不是“全部有效”。

## 15. large-workspace 与 partial index

测试组合：

- I完全缺失；
- I只建metadata；
- candidate search 50%/99%/100% coverage；
- parser版本变化；
- OCR版本变化；
- watcher gap；
- 100万小文件。

普通 edit/create/move/Trash只等待其真实局部范围。strong Query/Action只有完整source scan或合格complete coverage才能成功。building index不把未扫描对象当empty。

## 16. wire12 canonical 与 legacy corpus

新 corpus必须独立覆盖：
- wireVersion12 accept；0..11及未知对新 decision拒绝；
- saved v9/v10/v11 replay router，原 bytes/fingerprint/pins/authority-custody语义不改；
- DecisionKey/2、protocolOwner=D3 与 same OperationId跨domain合法/同key fingerprint conflict；
- CommitDomain replica/server；
- Frontier/2 exact 与 scope_dependencies 两种 policy；
- ObservationScope/2 local_structure、workspace_constraints、prepared_workspace；
- d3_identity_operation/12 OwnerInputBinding closed descriptor 且 WriteProtection=strict；
- InputDescriptor exact equality、pin mismatch、SourceObservation/DependencyProof mismatch；
- replica_local/managed_atomic mode矩阵；
- production SourceVersion domain 与 observerDomain可不同；
- observationEpoch变化使同生产版本旧 evidence stale；
- receipt effectClass portable/control_only/no_op 与同 P D3DecisionCompanion/2；
- ChangeId只在 seal 分配；
- resolver新增 conflict/incomplete/placeholder；
- D3-CJ/3 permutation不变；
- Result/9、Annotation Value/3、Locator l1不升级数字。

历史 v9/v10/v11 corpus只做旧 decoder回归，不能把版本数字替换成12后声称新语义通过，也不因为“未发现部署记录”删除历史恢复合同。

## 17. terminology gate

D3 Lexicon后像必须证明：

- fixed-S 42 conceptId exact set全部保留；
- 每条 ownedNames exact-set不变；
- firstFreeze不变；
- D6 imported names（含 DecisionKey/2、Frontier/2、SourceObservation/1、OwnerInputBinding/2、D3DecisionCompanion/2、WriteProtection）不被D3重新拥有；
- Preparation Binding/Definition Transfer/Definition Result Segment保留历史 firstFreeze；
- 新 technical field若没有真实 owner mapping则拒绝；
- retired identifiers与owned set仍互斥。

## 18. 后续 owner 与激活门

G0-B 本批完成 D1 产品消费与 D3 wire12/terminology/impact 配套，但仍是 candidate/not activated/not implemented。

后续必须完成并共同接受：
- D4：SourceObservation/生产 SourceVersion 分离、Frontier/2、weak B→N 与 semantic_pending 消费；
- D5：native ordinary-save protection 与 local/complete structure范围；
- D7：CommitDomain/Frontier/SourceObservation complete cut、新 Prepared、effects；
- D8：Source/Live/Read与collaboration checkpoints；
- D9：ImportJob/export scoped pins；
- D10：execution responsibility子schema与sourceOccurrenceKey。

A+B 不构成部分激活。D4/D5 C 批和后续 owner未完成前，产品不得产生“wire12 coordinated managed success”的激活声明，也不得用 G0-B fixture 模拟尚未闭合 consumer。

## 19. 验收结论边界

本文件是作者候选，不是独立评审。实际未来测试必须绑定固定 commit、真实平台/backend和输出工件；文字案例或文档 CI不等于实现通过。

旧 D10 B13 保持 REVISE、术语/双语 FAIL、3 P1 + 8 P2 共11 OPEN。
