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
| wire12 | CommitDomain、Frontier、InputDescriptor、guarantee 的 closed request/receipt | D3-CJ/3、D3Integer、Ref/Locator词法不变 |
| scoped ledger | WorkspaceId + CommitDomain + OperationId | v9/v10/v11 saved ledger原样重放 |
| replica_local | create_node、move、reorder、Trash 的局部可靠语义 | 不升级为完整全集证明 |
| managed_atomic | restore/purge/copy/fork/continue/import与完整 closure | 不自动降级 local |
| resolver | conflict/incomplete/placeholder 与 SourceVersion/2 currentness | 不泄露隐藏 branch |
| portable identity | birth/parent-order/lifecycle/tombstone/no-reuse | path/I/P不是逻辑owner |
| preparation | D7 backing schema仍由D7 owner | 不复制 PreparedActionBinding/3 |
| sync conflicts | D6 ConflictRecord + D3 typed resolution | 无 LWW、mtime winner |

## 2. decoder 与版本路由

实施必须先按 wireVersion 分流：

- v9/v10/v11：仅已保存 decision、planned recovery、receipt/error/outcome replay；禁止新 decision。
- v12：只接受 D6-FA-r01 request shape。
- 其它版本：unsupported wire，不猜兼容。

v12 decoder必须验证 CommitDomain/2、Frontier/1、InputDescriptor/2 由 D6 owner decoder 成功；
  再检查 Workspace/domain/guarantee/inputDescriptor cross-field equality。

same OperationId 在两个 CommitDomain可独立存在；同 domain 同 key 不同 fingerprint固定 operation_id_conflict。

## 3. canonical request 与 pins

wire12 canonical request不能永久嵌入完整 Document/Resource/Annotation bytes。
  D3 owner descriptor只保存 closed semantic descriptor和 typed PinRef slots。

实施测试必须证明：

1. 相同 InputDescriptor canonical bytes但任一 exact pin不同→不是同输入。
2. 仅 sha256相同而 source binding/provenance不同→不能恢复原输入。
3. planned恢复复用原 pin，不重新读取 current source后伪造“等价 request”。
4. old v11 saved request保持原 bytes，不迁移到 InputDescriptor/2。
5. pin cleanup服从 D6 last-reference/retention，不能删除 planned/unknown/conflict/approval-money所需证据。

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

### 5.1 existing source edit

由 D6 source-save owner测试；D3只提供 current live owner/identity条件。D2-invalid proposal拒绝普通 save；external invalid bytes进入repair读路径。

### 5.2 create_node

正例必须证明：active replica、current parent/ancestor、完整destination sibling list、
  fresh allocation、D2 valid、local typed facts、safe install、portable birth+placement。

负例：

- hidden/missing sibling range；
- parent处于 conflict/placeholder；
- fresh ID与remote birth冲突；
- local typed fact invalid；
- backend只有hash-then-replace。

### 5.3 move/reorder

正例覆盖 same-parent、cross-parent、no-op final index、compound movement。
  完整 old/new sibling lists与 ancestor cycle proof必需。

负例覆盖 duplicate final index、cycle、hidden sibling、
  concurrent move head、parent lifecycle conflict。

### 5.4 Trash

正例覆盖完整 local subtree/owner/reply membership、
  Trash sibling order、policy、safe metadata install。

若 inbound全集未证明，SemanticState必须 semantic_pending(inbound)。receipt不能列一个伪造的global referenceLifecycleTransitions全集。

## 6. managed_atomic 强门

managed_atomic fixture至少覆盖：

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

任一完整 range、authority/custody、D4/D5/D7 owner version或 pin continuity缺失时失败，不降级 replica_local。

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

## 9. purge frontier 与 tombstone

purge fixture必须锁定：

- current replica registry revision；
- active replica set；
- required Frontier；
- target Trash change；
-全部已知source/lifecycle/placement heads；
- inbound/reference complete proof；
- D4/D5 complete obligations；
- allocation/tombstone history。

少一个 active replica frontier→暂停/冲突，零 payload deletion。

retire replica后可重新评估 purge；retired epoch不可重新 active。旧文件后来接入必须 fresh ReplicaEpoch，并由 tombstone阻止原 ref复活。

从未登记设备不进入 active set；同步服务“已完成”不算 ack。

## 10. resolver 与 privacy

新 resolver测试覆盖每个 typed entity的：

resolved | trashed | tombstoned | conflicted | incomplete |
  placeholder | not_found | not_visible | workspace_unavailable | invalid。

无 state-disclosure时上述存在性差异全部遮蔽。locator只有 canonical live entity + locator disclosure后才进入 resolved/stale/anchor ambiguity。

同样 bytes/span/token 在 observationEpoch变化后不得恢复旧currentness。

## 11. D6安装与D3决议组合

故障注入覆盖：

1. P planned前崩溃；
2. pins durable后planning事务未知；
3. InstallationNotice前；
4. notice后、第一component前；
5. 每个 staged/flush/install/dir-flush点；
6. installed verification；
7. P seal前；
8. seal后、ContentCompletionProof前；
9. proof写入/flush中；
10. response delivery丢失。

每格只允许 D6 定义的 exact_before/exact_after/third_state/unavailable 与 D3 decision组合。

written target核对 planned poststate；未写 dependency核对 original cut。不得安装 after后又要求 target等于 before。

third_state保留现文件和pins；不能覆盖或盲回滚。P seal成功后 publication失败不 terminalize已提交 decision。

## 12. r5/r6 与 saved replay

golden sequence：

- O5可靠提交 r5，response丢失；
- O6随后提交 r6；
- retry O5 exact request。

唯一结果：O5返回原 r5 receipt bytes；current source读取 r6；无第二source write、无重复 ApprovalUse/Money charge、无版本回滚。

撤权时 O5 replay可以 not_visible，但 decision不改变；重获授权只恢复原decision交付。

## 13. Server 多用户

至少测试：

1. A/B不同文档同时 prepare；两边可完成。
2. A/B同文档同 Base；A先seal，B保留Draft并stale/conflict。
3. B撤权发生在B checkpoint前；B不能提交。
4. A已seal后A撤权；历史decision不回滚，后续交付按授权遮蔽。
5. Server restart/failover同时 fence P与file writer。
6. 旧实例只能读或拒写，不能rename author files。
7. 后端SQLite writer串行不阻断前端多个Draft/session。

实时协作算法尚未实现；相关测试只能是未来接口契约，不宣称通过。

## 14. semantic_pending consumer gate

在 D4/D5 afterimage完成前，以下测试固定拒绝 pending作为完整证明：

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

- wireVersion12 accept；0..11及未知对**新 decision**拒绝；
- saved v9/v10/v11 replay router；
- CommitDomain replica/server；
- same OperationId跨domain合法、同domain fingerprint conflict；
- InputDescriptor exact equality与pin mismatch；
- replica_local/managed_atomic mode矩阵；
- receipt新增domain/changeId/guarantee/semanticState；
- resolver新增 conflict/incomplete/placeholder；
- D3-CJ/3 permutation不变；
- Result/9、Annotation Value/3、Locator l1不升级数字。

历史 v9/v10/v11 corpus只能作为旧 decoder回归，不能把版本数字替换成12后声称新语义通过。

## 17. terminology gate

D3 Lexicon后像必须证明：

- fixed-S 42 conceptId exact set全部保留；
- 每条 ownedNames exact-set不变；
- firstFreeze不变；
- D6 imported names不被D3重新拥有；
- Preparation Binding/Definition Transfer/Definition Result Segment保留历史 firstFreeze；
- 新 technical field若没有真实 owner mapping则拒绝；
- retired identifiers与owned set仍互斥。

## 18. 后续 owner 与激活门

必须同批后续完成：

- D4 semantic_pending消费；
- D5 local/complete structure范围；
- D7 CommitDomain/frontier/Prepared新版本/effects；
- D8 Source/Live/Read与collaboration checkpoints；
- D9 ImportJob/export scoped pins；
- D10 execution responsibility子schema与sourceOccurrenceKey。

这些 owner未共同接受前，产品不得产生“wire12 coordinated managed success”的激活声明。

## 19. 验收结论边界

本文件是作者候选，不是独立评审。实际未来测试必须绑定固定 commit、真实平台/backend和输出工件；文字案例或文档 CI不等于实现通过。

旧 D10 B13 保持 REVISE、术语/双语 FAIL、3 P1 + 8 P2 共11 OPEN。
