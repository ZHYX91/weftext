---
source_language: zh-CN
translation_status: source
---

[English](UPSTREAM-AMENDMENTS.md)

# D10 配套上游协调修订提案

revision: D10-FA-r01-2026-10-02；状态：协调作者候选，未接受、未激活、未实现。最近一次完整历史 R08 评审绑定 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 与 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，结论 REVISE（P0=0、P1=3、P2=8）。十一项历史最终处置仍为 OPEN。具名修订已有限定独立复核，实际跨 owner 整合及 fresh 全局接受仍未完成；REVIEW-DISPOSITIONS 区分各层证据。

## 1. 修订目的和不变边界

现有 D7 已冻结 fresh Action prepare、完整 preview/effects、原 D3/D6 request、重放和 unknown recovery；现有 D6 已冻结 current authorization、planned/commit CAS、唯一 author ledger 与最终 author commit point。D10 需要补足三类原上游尚未定义的组合语义：

1. Standing Approval 在进入 D6 后发生竞争失效时，D6 必须有唯一可表达且不产生永久拒绝的错误。
2. 原 D6 decision 已经 planned、原 preview transport 已过期时，当前有权用户必须能只读重新查阅**原保存 preview**，而不是重新 prepare。
3. ApprovalUse 次数 reservation 在 authoritative `terminal_failed` 时必须有唯一终态，且与费用 reservation 分开。

本提案仍只开放 D10 candidate 定义的一个窄自动作者 profile：existing Node + one Field + 当前完整 Field 恰一 Entry + 一个 existing scalar member 的原 D7 `set_field_member`。其它 D7/D8/D3 author mutations 继续逐次交互确认。

以下保持不变：

- D3 wireVersion12、Result/9、mode、identity/lifecycle stages 与 receipts；
- D6 `d6_commit_request` 与 `d6_commit_receipt` 当前 wireVersion=2 exact shape、DecisionKey/2 `(Workspace, CommitDomain, OperationId)` ledger key 与唯一 author commit point；
- D7 `ActionSpec`、`d7_action_prepare`、`d7_action_prepared` 与 `PreparedActionBinding/3` exact shape；
- D7 `EffectManifest/2`、EffectBytes item/schema 与 committed transport；
- D8 `PreparedEditBinding/2`、Draft/IME/explicit confirmation；
- D4 Registry、D7 Narrow Field Qualification 与 D6 Policy/ObservationScope；
- 原当前授权、deny 优先级、信息不披露、authority/fence、dependency CAS、replay 与 planned 恢复规则。

D6 `d6_error` object shape 与 disposition 集合保持，但第一份共同公开 unattended author-submit 合同的 code 闭集需要协调加入 `approval_unavailable` 与 `execution_stopped`。它们都是必要的 closed-enum 扩展，不伪装成既有错误，也不建立匿名 old/new D10 error profile。D7 Preview/Effects 需要增加 planned-only 只读恢复入口；原 `d7_effects_resolve/open` 继续 committed-only。本轮当前协调提议 Policy/3 新增 `d10_control_self`、固定 bootstrap profile/2 的原非 Field 集合并新增显式 profile/3。以上都只是设计提案；独立接受/协调激活不等于产品已经实现或 D1 capability 已可用。

## 2. D6 Storage Transactions Permissions and Sync — 拟新增条款

### 2.1 D10 delegation/approval 不是 D6 权限来源

建议在 D6 主文 current `PrincipalContext`、generation 与撤权规则之后新增：

> **D10 delegation/approval consumption.** D10 可由 host/Core 受管控制域提供当前 DelegationLease、StandingApproval 与 PlannedDecisionApproval 证据，但它们只能收窄已经由 D6 Policy、ObservationScope、state disclosure 与实际 footprint 授予的资格，不能增加 capability、把 deny 改成 allow 或替代 current authorization。普通请求、Agent、worker、Connector、MCP server 与客户端 JSON 均不能自报 principal、delegation、approval、approval count 或 proof。
>
> 每个使用 D10 委托的 plan、commit、saved receipt delivery、result/effects delivery 仍先执行 D6 既有 current principal/delegation/policy/generation 门。DelegationLease 过期、撤销或时间连续性不可证明会阻止新的受保护步骤。`maxRuns` 耗尽只阻止**尚无 LeaseRunUse 的新 Run**；已经有完整 LeaseRunUse 的同 Run 后续步骤和原 planned 恢复不因 remaining=0 被再次拒绝，但仍须通过当前授权、准确 leaseRevision、可信时间、ActivationBinding、批准和预算门。以上任何控制失败都不把已 committed decision 改写为失败，也不回滚已经线性化发送的外部效果。旧 approval 或 capability probe 永远不是持续授权票据。
>
> 只有 D10/1 coordinated contract 明确列出的 author-submit profile 才允许 Standing Approval 替代本次交互确认；首版仅为 D7 `set_field_member` 的 `single_field_member` profile。其它 D3/D6/D7/D8 author intent 沿原确认合同。D10 approval 无法使原本非法、不可观察、stale、超预算或语义冲突的 plan 成为合法。

### 2.2 ApprovalUse 是附加授权证据，并显式允许 raw no-op

建议在 D6 主文唯一 author commit point 和 planning/commit 依赖重验附近新增：

> 对采用 D10 Standing Approval 的 D6-owned Action，D7 prepare 完成后，Core 必须依据已保存的 PreparedActionBinding/3、完整 owner_fields preview、实际 MutationFootprint、current D6 authorization 及 D10 当前控制记录，独立构造受管 `ApprovalUse/1`。客户端仍只持原 `d6_commit_request`；不得向 request 增加 approved、approvalId、delegation、proof、budget override 或 effect override。
>
> `ApprovalUse/1` 不可变地绑定 approvalId/revision、Run/step、原完整 canonical D6 request、planToken、对应 PreparedActionBinding、完整 preview semantic binding、实际 footprint proof、DelegationLease/ActivationBinding、approval-count reservation 和全部适用 budget/cost reservations。它是附加授权证据，不是第二 author plan、第二 ledger、receipt 或 capability token。
>
> Core 只能为 `single_field_member` 建立 ApprovalUse。共同前提是：当前完整 Field 恰一 Entry；原 Action 为 `set_field_member`；owner、FieldId、memberPath、memberType 与 envelope 准确匹配；value 满足 closed constraint；D7 Narrow Field Qualification 成功；完整 preview 已形成且可披露。实际结果只有两个允许分支：
>
> 1. **member-change**：MutationFootprint 恰只包含该 existing scalar member 的改变，公开 owner_fields preview 有准确 `field_change`；
> 2. **raw-no-op**：原 Action 仍准确解析到同一唯一 Entry/existing member，请求 typed value 与当前值相等，并且完整 proposed source bytes 与 before 逐字相同。此时 MutationFootprint 必须为空、preview 不列 `field_change`、最终 D6 receipt 的 `sourceVersions` 为空；不得伪造 effect、footprint 或 revision。
>
> 任一 occurrenceKey、其它 member、qualifier、note、provenance、其它 Entry、正文/title/coreKind/Facet、Ref/relation、identity/lifecycle/placement 或 D6 控制状态变化都使 Standing Approval 不适用。Core 不选择第一条、preferred 或同值 Entry，也不把失败降格到整 Entry/整 source 写入。
>
> 若 raw-no-op 最终按原 D6 规则形成 committed decision，它仍消费一次 approval successful-commit count；same saved decision replay 不再次消费。

### 2.3 D6 内批准竞争的唯一错误

建议在 D6 主文和 Control Interfaces 同时增加以下错误合同：

> 在正式 `d6_commit_request` 进入 D6 之前，D10 adapter 可以依据已获权控制状态返回 D10 approval_required、approval_expired、delegation_expired 或 delegation_exhausted。该 probe 不消除后续竞争。
>
> 具名 D10 路径把受保护批准依赖关联原 planToken 并进入 D6 后，若该依赖缺失或不可用，而原 D6 current authorization/ObservationScope 与适用业务可见性仍成立，使用新增 `d6_error.code="approval_unavailable"`，disposition 固定 `preflight`。仅有两个 closed profile：single-field-member action 的 Standing Approval，以及下述原 external-consent control plan 的可信确认；均不创造客户端可自报的通用 approval 字段。
>
> 原 D6 permission failure 仍为 `not_visible/preflight`；业务 dependency、semantic、budget、authority/integrity 错误仍返回原 code/disposition。不得用 `approval_unavailable` 遮蔽它们。
>
> 对 unseen request，`approval_unavailable/preflight` 不写 author ledger、recorded rejection 或 planned；原 prepare 若仍有效，可在新的合法批准下重试，或回到交互路径。对已 planned request，同一错误保持 ledger=planned 并保留原 plan/pins/reservations；不能写 `semantic_rejected` 或 `terminal_failed`。
>
> D10 adapter 必须逐字传递这个 D6 error，不包装成 D10 approval error。调用端若要显示“批准已撤销/过期/次数被抢占”等细分原因，必须另经当前受权的 D10 control read，不能从 D6 author error 偷带控制细节。
>
> 这是第一份共同公开 unattended author-submit D6-Control/2 error 闭集的一部分。固定 S 的 D6 是尚未发布的设计合同，不构成必须运行时兼容的旧 D10 profile。能够进入该正式 branch 的组件组合必须支持同一闭集；不兼容组合在 D1 capability/version gate 前拒绝。Policy/1/2、bootstrap profile/1/2 与历史 saved decision decoder 均保持。

external-consent 控制路径另消费 CONTROL-CONTRACT §7 拥有的准确受保护 `ExternalConsentConfirmation/1`。原 D6 control plan 必须将原 ControlPrepareBinding、完整 immutable review 和可信当前确认关联为准入依赖，planning 及 final commit 都检查；直接提交原请求不能绕过。提交前可信界面等待本次用户事件，不定义新的公共 Workspace-commit 响应。进入 D6 后，若原授权和业务门已满足而确认缺失/不可用，使用同一 `approval_unavailable/preflight` 规则，保留 planned 和全部原恢复 pins。当前披露拒绝保持 not_visible 优先。已保存 consent 先重放，再考虑新确认门。这里不为外部写入增加 Standing Approval profile，不增加作者请求字段或第二决议；外部 transport 仍有独立 send fence。

### 2.4 planning CAS 原子 reserve

建议扩展 D6 planned CAS：

> 对带 ApprovalUse 的 unseen D6 request，原步骤6业务、authorization、dependency、semantic、budget 验证完成后、写 planned 前，planning CAS 还必须比较 current approval/delegation/activation revision、ApprovalUse 与 request/plan/preview/footprint 的准确绑定、approval 的时间与 revoked 状态、`consumed + reserved < maxSuccessfulCommits`，以及所有适用 budget/cost reservation 的旧版本。
>
> 只有全部成立，才能在同一 database write transaction 写 planned、完整固定 plan/pins、ApprovalUse count `unreserved→reserved` 与相关资源 reservation。两个 request 竞争最后一个 approval count 或最后预算时至多一个 winner；loser 回原 restart gate，不能把先前读取的余额当 current fact。
>
> same DecisionKey/2 `(Workspace, CommitDomain, OperationId)` 的 canonical replay 始终关联同一 reservation，不能重复 reserve。未进入 planned 的临时 ApprovalUse 可以按准备寿命清理；一旦 planned 引用，approval reservation 与完整恢复 pins 按原 ledger recovery 生命周期保留，不受 preview TTL、Agent session 结束或普通 cache GC 清理。

### 2.5 final author commit、terminal release 与 replay

建议扩展 D6 final transaction 和 recovery：

> 对带 ApprovalUse 的 planned decision，最终 author commit 仍逐项验证 current D6 authorization、authority/fence、原业务 dependencies 与完整 plan；此外比较 ApprovalUse 仍绑定同一 request/plan，以及 current standing/supplemental approval 是否允许本次**新的** author submit。
>
> 全部通过时，author payload/control effects、D6 committed decision、canonical receipt bytes、approval count `reserved→consumed`、ApprovalUse terminal link、适用 D6 budget control effects 与必要 audit link 在同一 final author transaction 发布。raw-no-op committed 同样 consumed。事务回滚时这些全部回到原 planned/reserved。
>
> 若原 D6 ledger 在完整 continuity 和 current authorization 下证明该 planned decision **确定永不提交**，并按既有规则写 authoritative `terminal_failed`，同一个 abort transaction 必须把 ApprovalUse count reservation 从 `reserved→released_terminal`。released_terminal 不计入 maxSuccessfulCommits 的 reserved/consumed 合计，但历史不可删除；terminal replay 不能再次 release。
>
> 只有 authoritative terminal_failed 能释放 approval count。Run cancel、preview/approval TTL、临时撤权、Lease 过期、attempt/work 暂停、authority/continuity 暂不可证均不是 abort，reservation 保留。
>
> approval count 与费用 reservation 完全分域。author terminal_failed 不自动退款已发生、可能发生或费用未知的模型/工具/外部调用；费用按 D10 settled/released/uncertain 合同独立处理。
>
> 已 committed decision 的 delivery/replay 不要求历史 Standing Approval 仍未过期，只要求原 saved decision 连续且当前调用方满足原协议的 current authorization。lost receipt 返回原 bytes，不再次消费 approval count、budget 或 source revision。

## 3. D6 Control Interfaces — 拟新增/替换条款

### 3.1 PreparedIntent producer 分类

在当前 D6 Control §3.6 的受管 PreparedIntent producer 边界补充：

> D10 Broker 本身不是 PreparedIntent producer。D10 Agent/Automation 的工作区修改只能调用已有 D7/D8/Core closed adapter。对于 `single_field_member`，D7 prepare 先按原合同生成 PreparedActionBinding/3、PreparedIntent、preview 和原 `d6_commit_request`；随后 Core-managed D10 approval adapter 才可从受保护记录建立 ApprovalUse。D10 runtime 不得直接构造 PreparedIntent、MutationFootprint、source bytes 或 proof。
>
> ApprovalUse 不改变 `d6_commit_request` exact members、canonical request key、planToken tag、protocolOwner 或 ledger key。它通过受保护内部关联绑定原 planToken；外部 caller 没有可追加 approval 字段。

### 3.2 实际 D6 Control §5 边界上的增补

当前 D6 owner 的算法有十四个有序步骤；旧 S 的八步编号不是当前算法。闭合解码、当前披露/ObservationScope、域/fence/custody 以及完整原 DecisionKey/request 比较均先于新的 D10 业务门。步骤4中，已保存结果按当前结果权限重放，先于任何新的批准或 stop 资格；planned 只恢复原计划、pins 和预留。

unseen 步骤5中，受权解析 planToken 后，仅通过受保护最小映射定位具名 approval/Run/confirmation 要求。映射缺失、audience/Workspace 错误保持原 not_visible 规则；原当前授权通过后才读取完整保护记录。步骤6先保留原业务、dependency、semantic、budget 错误，再核对精确 single_field_member 效果分支、当前 approval/delegation/activation、适用的完整外部确认及计次/费用预留候选。其它前提均成立而具名 approval dependency 不可消费时返回 approval_unavailable/preflight，unseen 不写决议。

步骤8 planning CAS 原子保存原固定计划/pins，同时把 ApprovalUse count 从 unreserved→reserved 并写入适用预留。CAS 失败者重新进入原授权/域/key 路径，不能沿用旧计数。步骤9–11保持原严格安装 notice、真实前后像、受管屏障、安装溯源与恢复义务；受保护 D10 步骤不选择 observed_only。

步骤12是唯一 seal/作者提交点。实际 P 事务在提交原决议及 reserved→consumed 前，重新验证原 written-after/unwritten-before 依赖、当前权限、精确受保护 D10 资格与不可逆 stop。只有原权威终止证明允许同一 abort 事务改为 reserved→released_terminal。物理安装后、seal 前 stop 赢不证明从未写入 bytes：保留 B/N、受管屏障与实际溯源，只能沿原已证明的 rollback/abort 路径处理；未知安装、third state 或不可证明 custody 继续 paused/recovery_unknown 并保留全部责任。步骤13–14发布/交付同一保存决议，不再确认、安装源、计费或消费次数。共同接受前必须把这些边界落实到实际 D6 owner 正文，本提案本身不能替代。

### 3.3 当前 D6 /2 回执与错误的精确扩展

将 `d6_error.code` 闭集扩展为：

```text
invalid_request
unsupported_version
not_visible
domain_unavailable
integrity_conflict
operation_id_conflict
plan_expired
source_unavailable
proof_unavailable
dependency_conflict
semantic_rejected
budget_exceeded
install_unavailable
conflict
conflict_changed
state_unavailable
owner_update_required
effects_unavailable
transaction_aborted
approval_unavailable
execution_stopped
```

`approval_unavailable` 只允许 disposition=`preflight`。它表示 coordinated D10 author-submit 的 approval dependency 在进入 D6 后不可消费，但原 D6 author permission/ObservationScope 与适用业务前提没有更早失败。它不生成 recorded rejection，不泄露 approval 内部原因。

`execution_stopped` 同样只允许 disposition=`preflight`，且 current authorization/ObservationScope 必须先通过。它只表示本 request 绑定的不可逆 ExecutionStopLatch 已在线性化顺序中阻止新的 author submit；不得替代 `not_visible`、approval 错误、temporary disable、Lease expiry 或业务 dependency。unseen 不写 ledger；planned 按 §3.5 的严格 authoritative-abort 条件处理。

当前 preflight/recorded/paused/terminal 四种 disposition 与原 key/权限/availability 规则不变；`dependency_conflict|semantic_rejected|budget_exceeded` 仍只在原步骤6可 recorded；planned 后永久 abort 仍只有 `transaction_aborted|terminal`。不兼容的组件组合在 D1 gate 拒绝；真实旧 wire1 决议保留原 decoder，不因新闭集而重编码。

Standing Approval 不增加 receipt member。D10 UI 可以从受权 Run/ApprovalUse control state 显示使用来源，但它不是 author receipt。

### 3.4 D10 control state 与 Lease Run admission

ActivationBinding、DelegationLease、StandingApprovalEnvelope、AutomationDefinition、LeaseRunUse、PlannedDecisionApproval 等耐久 D10 控制状态只能由 Core 管理的封闭 control adapter 修改，并具有独立 control revision/CAS、当前 principal authorization、audit 与 replay 合同。它们不能经作者 source、provider callback、Agent JSON 或 `d6_commit_request` 自由 payload 修改。

`maxRuns` 消费发生在第一次受保护执行前的 D10 Run-admission CAS，不进入 D6 author ledger。CAS 按同一 `leaseId` 谱系累计历史，写 `LeaseRunUse/1`；同 Run restart 不重复消费，准入后的 failed/cancelled/crash 不退款。只有一个**尚无 LeaseRunUse 的新 Run**在累计消费已达到 `maxRuns` 时才返回 D10 `delegation_exhausted`。已经有完整 `LeaseRunUse/1` 的同 Run 后续受保护步骤和原 planned request 恢复不再比较 remaining count，也不再次消费；即使 `maxRuns=1` 且累计已经为 1，也不能把该同 Run 当成新 Run 拒绝。它们仍必须逐步验证当前授权、准确 `leaseRevision`、可信时间、ActivationBinding、批准和预算；撤销、过期、revision/binding 变化或连续性不可证明仍会阻止执行。

### 3.5 D10 Workspace 自助管理、bootstrap profile/3 与不可逆 stop

这是对固定 S D6 Control 的显式协调修订，不改写 S 快照。

当前 Policy/3 capability union 增加一个无参数值 d10_control_self，只允许 workspace scope。它允许当前主体通过 R05 CONTROL-CONTRACT 的 closed Workspace control adapter 管理自己拥有的有限 Automation/Lease/Approval/Run 控制记录；它不蕴含 source/Field read-write、policy_admin、registry_admin、binding_admin、repair、commit_sequence_state 或任何 deployment resource。其它能力也不蕴含它；deny 继续优先。

Workspace D10 control adapter 是受管 PreparedIntent producer，但只接受 CONTROL-CONTRACT §7 的 automation_configure、consent、state、workspace_limits、activation 五类 Workspace body。它不能接受 deployment_put、cost_reconcile、secret bytes 或自由 callback。producer 从真实 current principal、完整 closed body、受权读取和 stable prepare binding 生成原 d6_commit_request；external caller 仍不能声明 principal、authorized、effect 或 writer。涉及作者 payload 的最终 mutation 仍只能来自原 D7/D8/D3 adapter；普通 D10 control mutation 不能构造 author source bytes。

固定 S profile/2 的“全部非 Field capability”在本 amendment 中冻结为 S 当时的受控集合 `workspace_state, entity_state, locator_state, source_read, source_write, body_write, node_control, node_create, resource_read, resource_write, annotation_read, annotation_write, lifecycle, registry_admin, binding_admin, policy_admin, export, repair, audit, source_envelope_state, commit_sequence_state`。以后新增 capability 不自动进入 profile/2。

新增 d6_bootstrap_profile wireVersion=3；成员仍为 kind,wireVersion,profileRevision,registrySeedBinding,newSeriesMultiplicity,initialPeriodScope。profile/3 明确生成 initialPolicy.version=3；creator 初始 Workspace grant 为上述固定 profile/2 非 Field 集合、当前 D6 的 replica_register/replica_retire/conflict_read/conflict_resolve/execution_custody_admin/structure_state/portable_frontier_state、d10_control_self，以及 S 原规则从 target Registry 生成的全部 Field read/write，deny 为空。实际新 WorkspaceBootstrapPlan/2 与 D7 bootstrap 投影按 CONTROL-CONTRACT §12 和真实 D6 owner 协调；不扩原 /1。

只有当前 administer_issuer 的显式 issuer-profile 管理操作才能把以后新 family 切换为 profile/3。既有 family 保存的 profile/1/2 副本、replacement、WorkspaceBootstrapPlan、saved decisions、replay/continue/failover 不重算、不补 grant。既有 Workspace 取得 d10_control_self 只能由当前 policy_admin 走原 Policy 修改事务显式安装；Field 权限主体不能自授。

d6_error.code 的第一份共同公开 closed set 同时增加 execution_stopped，只允许 disposition=preflight。对尚未 planned 的 D10-bound author request：先通过 current authorization/ObservationScope，再检查受保护 RunBinding 对应不可逆 stop latch；stop 已线性化时返回 execution_stopped/preflight，不写 author decision。

D6 final author commit 必须在真正持有 authority-store 写序列化的最终事务内再次验证同一 RunBinding 的 stop latches；不得只在 prepare 或事务外 check。stop 先线性化则本次新 author commit 不发生；commit 先线性化则保存原 committed decision，之后 stop 不倒推撤销。

latch transition 本身继续由 CONTROL-CONTRACT §11 冻结的专用 D10 safety transaction 拥有。它在同一物理 Authority Store serialization domain 中使用预留 latch/result/sequence capacity，但既不是 `d10_control_prepare`、也不是 `d6_commit_request`，更不增加 D6 public store-incarnation API。D6 只在 planning/final recovery 消费受保护 latch association；普通 D6 budget、author commitSequence 或 D10 configuration Counter 耗尽都不能成为停止已预留 target 的前置条件。

对 already planned request，stop 本身不能直接写 terminal。只有 current authorization、authority/continuity、原完整 plan/RunBinding 和不可逆 stop 均可证明，并由原 D6 recovery 证明该计划确定永不提交时，才能在原 author ledger 中写 transaction_aborted/terminal；同一 abort transaction 按既定 ApprovalUse 规则释放 approval-count reservation。临时 disable、Lease expiry、普通 Run cancel、暂时撤权或 clock/state 暂不可证都不是该证明，planned 保持 planned。费用 reservation 不因 author abort 自动释放。

stop 不阻断当前获权的 authoritative abort、cost settlement、evidence/audit retention 或 reference-safe cleanup；这些操作不会恢复 executor 或 author-write 权限。

External send 竞争继续归 D10 transport boundary，而不是 D6：首次不可逆 handoff 前，同一 safety serialization/fence 重验 frozen ExternalExecutionBinding、stop、approval/grant/secret/cost binding，并耐久记录 send-attempt evidence 与 holds。send 先完成保留已发送事实；stop 先完成阻止发送。D9 worker 保持原 no-network 默认，不因本 D10 amendment 获得 transport capability。

## 4. D7 Execution and Action Interfaces — 拟新增/替换条款

### 4.1 ActionSpec 默认确认句

将 §5 当前“user 看到 preview 后通过原 D3/D6 提交入口确认”的语义替换为：

> ActionSpec 默认确认仍是：当前用户取得并验证完整 preview 后，明确发送 prepare 返回的原 D3/D6 request。只有 D10/1 coordinated `single_field_member` profile 可以在没有本次人机点击时继续：Core 必须从这次 fresh D7 prepare 的完整 PreparedActionBinding/3、EffectManifest/EffectBytes、实际 MutationFootprint 和 current authorization 机械证明有效 ApprovalUse。这个例外不创建 D7 confirmation token、不修改 request，也不表示用户逐字阅读 preview。其它 ActionSpec kind 和 D3-owned intent 继续原交互确认。

### 4.2 standing-approval eligibility，包括 raw no-op

在 §6 prepare/commit 前新增：

> D7 prepare 永远不知道或信任 caller 自报 approval。它仍完整执行原 closed intent decode、潜在 observation、authority/cut、source/selector/definition/rule/dependency、proposed source/MutationFootprint、D2/D4/D5/D7 gates 与 immutable prepare/preview。只有全部成功后，Core-managed D10 adapter 才尝试匹配。
>
> 当前唯一 profile 要求 ActionSpec.intent.kind 恰为 `set_field_member`，当前完整 Field 恰一 Entry，memberPath 指向 existing scalar member，owner/Field/member type/value constraint 精确匹配，Narrow Field Qualification 成功。实际效果只允许：
>
> - member-change：MutationFootprint 恰为所选 member，owner_fields preview 有准确 `field_change`；
> - raw-no-op：请求 value 已等于 current typed member，完整 proposed source 与 before bytes 相同；MutationFootprint 为空，preview 不列 `field_change`。
>
> eligibility=false 不改变原 Action 合法性。可交互提交的 Action 转 awaiting_confirmation；原 Action 本身非法则返回原 D7 error。D7 不为 automation 增加更宽 parser、selector、permission 或 semantic fallback。

### 4.3 replay 和 restart

新增：

> Standing Approval 不改变 D7 OperationId/replay。fresh automation occurrence 必须 fresh prepare；同一 committed request 丢 receipt 只能重发原 request。D10/Automation restart 不得因为原 preview token 过期就生成语义相同的新 OperationId。
>
> 原 ActionEvidence、FieldSelection、result epoch、source revision、preview delivery epoch 失效规则保持。Standing Approval 不能复活 stale evidence/selector。若 decision 尚未 planned 且需要重新 prepare，就产生新 request/OperationId 和新的 approval use；旧 mechanical approval 不沿用。

### 4.4 不可逆 stop 与 D7 prepare 的绑定

D7 ActionSpec、PreparedActionBinding/3、EffectManifest/EffectBytes wire 不增加 caller-supplied stop 字段。对于 D10 自动 author-submit，Core 在 prepare 成功后用受保护内部关联把 planToken 绑定到当前 Run/Automation/Lease 与其 ExecutionStopLatch refs；调用方不能删除、替换或自报这些 refs。final submit 由本提案 §3.5 所指定的实际 D6 seal consumer 在真正 transaction 内重验 stop。D7 交互路径本身不因 stop 获得新的自动确认能力；stop 只会阻止仍未线性化的后台 submit。

## 5. D7 Preview and Effects Transport — planned 恢复扩展

### 5.1 新 planned-preview 只读入口

现有 `d7_effects_resolve` 保持 committed-only，planned/rejected/terminal/unseen 仍返回 `effects_unavailable`。现有 `d7_effects_open` 继续只接受 committed `effectsToken`。

新增：

```text
d7_planned_preview_open {
  wireVersion: 2,
  kind: "d7_planned_preview_open",
  protocolOwner: "D6",
  request: <original wireVersion=2 d6_commit_request, with its complete DecisionKey/2>
}

d7_planned_preview_opened {
  wireVersion: 2,
  kind: "d7_planned_preview_opened",
  request,
  previewToken,
  previewCursorToken,
  previewManifest
}
```

它不是 prepare、commit、resolve 或 author state mutation。输入必须是原完整 D6 request，不接受只有 OperationId、planToken 或 approvalId 的 ledger 探测。

唯一顺序：

1. closed decode；
2. current authenticated audience 与受保护最小定位映射；
3. 原 D6 current authorization、原 ObservationScope 与完整 preview disclosure 资格；
4. current authority/custody/ledger continuity；
5. 同 key canonical request byte-equal 且 ledger state=planned；
6. 原 PreparedActionBinding/3、其 semantic preview 与全部必要 pins 完整可证；
7. 建立新的有限 recovery delivery epoch，返回 fresh action_preview token/cursor/header。

新 epoch 的语义 item、排序、EffectBytes payload 和 profile 必须逐字来自原 planned 保存的 immutable preview，不得重跑 Query、重新解析 current definition、重新选择 target、重新生成 proposed source 或切换 Registry。它只重新签发运输句柄。

新 recovery epoch 有独立有限 TTL；它不延长或复活旧 preview token/cursor。page 和 EffectBytes 继续使用原 D7 transport。恢复 epoch 过期使用 `preview_expired`；epoch 已证明失效用 `reset_required`；pin/continuity 不可证、request/state 不匹配用 `effects_unavailable`；更早 authorization failure 仍 `not_visible`。不新增新的 effects error enum。

### 5.2 交互批准消费边界

只有在同一个 recovery epoch 下完整取得 manifest、所有 pages 与全部必要 EffectBytes，并按原 decoder 验证无洞后，UI 才可创建 D10 `PlannedDecisionApproval/1`。

该 record 绑定 exact 原 request、原 planned decision、原 immutable preview semantic record、current granting principal/DelegationLease/ActivationBinding、有限授予时间。它不绑定短命 preview token 本身，不修改 request、OperationId、PreparedActionBinding、plan 或 target。

最终提交仍走原 D6 request 和 §2/§3 的 current authorization、approval、dependency、authority/fence 检查。新的交互批准只解决“本次用户已重新查阅并授权原 plan”，不能使确定 dependency conflict、stale target 或损坏 pins 合法。

若原 plan 已有 standing ApprovalUse count reservation，该 reservation 在人工恢复期间继续保留；成功 author commit 仍 `reserved→consumed`，只有 authoritative terminal_failed 才 `reserved→released_terminal`。不得在人工点击时提前释放给另一个 Run。

## 6. D3 Terminology and Naming Lexicon — 最小勘误提案

固定 U 的 D3 词表在 `weftext.term.origin-binding` 的约第 377/379 行与 `weftext.term.adopt` 的约第 496/498 行之间存在一处内部 owned-name 冲突：`weftext.term.origin-binding` 已拥有 code 名 `OriginBinding` 与变量名 `origin_binding`；但 `weftext.term.adopt` 的 wire/API 说明又写“variables `adoption_binding` 必须映射 OriginBinding”，并把 `adoption_binding` 登记进 Adopt 的 `owned-names.codeConventions`。这会让同一个关联绑定概念同时出现第二个变量命名约定。

建议只做以下最小勘误，不改变任何 identity、wire、capability、操作语义或已冻结 OriginBinding 类型：

1. `weftext.term.adopt` 的 wire/API 改为：D3 不冻结 mode；future API/code verb 只使用 `adopt_*`。Adopt 若创建或重建 foreign-object 关联，其绑定值使用既有 D3 `OriginBinding(ForeignIdentityKey, NodeRef)` 类型，代码变量/字段名称使用既有 `origin_binding`；Adopt 不定义 `adoption_binding`。
2. Adopt 的 `owned-names.codeConventions` 从 `["adopt_*","adoption_binding"]` 改为 `["adopt_*"]`。
3. `weftext.term.origin-binding` 的 owned-names 保持 `OriginBinding` / `origin_binding`，继续唯一拥有这一绑定名称族。
4. 删除 `adoption_binding` 约定，不提供 compatibility alias、双读、迁移别名或第二变量名；它也不进入 public wire、CLI、locale、identity 或 capability。

正向验证：Adopt 代码路径可以叫 `adopt_*`，但其关联绑定值的类型和变量必须解析到 `weftext.term.origin-binding` 的 `OriginBinding` / `origin_binding`。反向验证：受控正向源码、API/schema、fixture、术语 registry 中不得把 `adoption_binding` 归到 Adopt，也不得把它作为 OriginBinding 的兼容 alias；扫描器不能用豁免掩盖残留。

这是对 D3 词表的最小协调勘误提案。固定 U 快照仍未修改；独立复核和协调接受之前，本文不能声称该勘误已经生效。

## 7. Pack parent lifecycle — 无需新增 D1/D4 wire 修订

CANDIDATE §6.1 对 Pack parent domain/extension point 的 lifecycle 收敛不要求修改固定 D1/D4 wire，理由如下：

1. D1 已经冻结 `unsupported_surface|not_in_release|policy_denied|user_action_required|missing_component|not_configured|offline|incompatible_version|temporarily_unavailable` 的 closed reason 和固定优先级。D10 只把 parent dependency 的实际阻塞事实映射到这些既有 reason；不新增 `parent_missing` 等产品级 reason。
2. D4 已经冻结 `RegistrySnapshot/1`、`RegistryBinding/1`、namespace definitions 的 `complete|unavailable`、raw-source preservation、generation+digest 双绑定，以及 tombstone/migration/semantic contribution 的累计 ledger。Pack/parent disable 不能借 D10 改写这些规则。
3. Parent Extension Dependency 只存在 D10 Capability Catalog/ActivationBinding 的部署控制面：domain contribution 声明 parent domain、extension point 和 version range；激活时解析 exact parent binding 并写入 Catalog digest。它不进入 D4 Field/Facet schema，也不成为作者事实。
4. UI visibility 继续只是产品投影。仅隐藏父模块 UI 不修改 D1 capability、D4 RegistryBinding 或 D10 contribution activation。

因此 parent missing/disabled/incompatible/unsupported surface 时，dependent runtime/view/action/rule contribution inactive；已接受 D4 definitions/history/raw author source 按原 D4 complete/unavailable 合同保留。若 exact definitions 仍 complete，Core 可解释已有作者事实，但这不授权新的 domain behavior。若 definitions 不可证明，按原 `provider_or_schema_unavailable`/retained raw 路径处理。

只有未来方案要求以下任一事项时才必须重开上游：把 parent dependency 成员加入 `RegistrySnapshot/1`；新增 D1 unavailable reason；让 UI disable 改写 schema availability；允许 disabled parent 下 dependent contribution 继续产生领域派生语义；或改变 D4 semantic-ledger retention。当前候选全部拒绝这些路线。

## 8. D8 / D9 术语配套修订与行为不变边界

R06 不给 D8 增加无人值守编辑分支，也不改变 D9 转换、模板或发布行为；但 Mandatory Intake §8.5.1 要求由**原 owner**补全受控术语元数据，因此本节取代此前“D8/D9 完全无需修订”的说法。以下仅是 D8/D9 owner 词表的规范性配套提案，尚未共同接受。

### 8.1 D8 owner 词表追加条款

D8 现有九个 concept ID、Editor Interfaces 的 12 个公开 request/response kind 与内部 `d8_prepared_edit_binding` 均保持原 wire。D8 owner 追加如下命名元数据；不改 IME、Write/Read、作者源、确认、Undo 或错误合同。

共同规则：九个 concept 的原 `firstFreeze` 继续是已接受的 D8 r03 concept contract。R08 只提议追加 naming/interface-owner metadata；该后续 metadata 有自己的 R08 candidate 来源，不能倒称 S 原行已经包含它。除 `Direction Preference` 的既有方向选择器和 Edit Draft 的既有状态短语外，本代不冻结新的 CLI verb、独立 UI 控件名或 locale key；写“无”就是规范结论，不是留实现决定。没有已发布旧 alias；实现采用这些名称时，必须从 D8 editor adapter 的受控 type/kind registry、fixtures、help/locales 中原子删除任何未列出的受控 alias，禁止双读。历史证据和用户正文不迁移。

| concept ID | 正式中英名；owner/layer | 定义 / 排除 | owned names 与 kind 归属 | CLI / UI / locale | 简称、alias、正反例与迁移目标 |
| --- | --- | --- | --- | --- | --- |
| `weftext.term.edit_session` | 编辑会话 / Edit Session；D8 UI state | 当前编辑交互会话；非 Workspace、Document identity 或提交权威 | code type `EditSession`；不拥有独立 wire kind；`d8_document_read/document` 仍是 D8 envelope 且嵌套 D2/D6 owner 数据 | CLI 无；不新增独立 UI label/locale key；界面继续显示 Source/Write/Read 模式 | D8 文档可简称 session；禁止 EditorSession wire alias；删除目标为 editor state registry 中未列受控 alias |
| `weftext.term.edit_draft` | 编辑草稿 / Edit Draft；D8 proposal state | 非权威用户提案与 `draftSerial`；非已保存作者源 | code type `Draft`；`d8_draft_project`、`d8_draft_text_replace`、`d8_draft_write` 输入归此；响应投影另归 Draft Projection | CLI 无；UI 继续使用已冻结“草稿已保存于此设备/待提交”等状态短语；不新增 locale key | D8 上下文可简称 Draft；禁止 author draft/source alias；删除目标为 draft-state alias/fixtures/help |
| `weftext.term.draft_projection` | 草稿投影 / Draft Projection；D8 Core projection | Core 对一次 proposal 的可丢弃投影；非 D2 payload、Locator | code/wire `d8_draft_projection`；`d8_draft_text_replaced`、`d8_draft_written` 也返回该 projection | CLI 无；无独立 UI label/locale key，renderer 只消费结构 | 可简称 projection 仅限 D8；禁止 document_snapshot alias；删除未列 projection wire/type alias |
| `weftext.term.draft_edit_map` | 草稿编辑映射 / Draft Edit Map；D8 Core mapping | flow/path/segment/plainRegion/site 的 proposal-local 坐标；非持久身份/Locator | code type `DraftEditMap`；由 `d8_draft_projection` 内 `editMap` 承载，文本 replace/write 请求消费其 binding | CLI/UI/locale 均无新增 | 可简称 edit map；禁止 locator/parser alias；删除 editor map registry 中未列 alias |
| `weftext.term.prepared_edit_binding` | 编辑准备绑定 / Prepared Edit Binding；D8 protected Core record | D8 immutable prepare 记录；非 D7 PreparedActionBinding、ActionSpec 或第三 ledger | type `PreparedEditBinding/2`；internal kind `d8_prepared_edit_binding`；`d8_edit_prepare/edit_prepared` 和 `d8_undo_prepare` 消费/产生其受保护结果 | CLI/UI/locale 无新增 | 简称 Prepared Edit Binding；禁止 prepared-action alias；删除未列 binding wrapper/fixture 名 |
| `weftext.term.composition_transaction` | 组合输入事务 / Composition Transaction；D8 UI input state | IME begin/update/commit/cancel 分组；非 D6 author transaction/receipt | code type `CompositionTransaction`；无 D8 JSON kind，新旧 input events 只映射到该 UI state | CLI/UI label/locale 无新增；辅助技术沿现有输入状态说明 | D8 上下文可简称 composition；禁止 commit transaction alias；删除 UI state registry 中未列 alias |
| `weftext.term.caret_affinity` | 光标亲和位置 / Caret Affinity；D8 layout state | 同 logical point 的 `upstream|downstream` visual side；非 source offset/Locator | code enum `CaretAffinity`；作为 layout/caret state 成员，不新增 request kind | CLI 无；无独立 label/locale key，必要辅助说明按当前 caret 状态生成 | 可简称 affinity；禁止 source-side identity alias；删除 layout enum 未列 alias |
| `weftext.term.layout_epoch` | 布局代 / Layout Epoch；D8 layout state | 同次 shaping/wrap/hit-test 的可丢弃世代；非 result/auth/author revision | code type `LayoutEpoch`；无 public D8 JSON kind | CLI/UI/locale 无新增 | 可简称 epoch 仅限布局上下文；禁止 result epoch alias；删除 layout cache 未列 alias |
| `weftext.term.direction_preference` | 方向偏好 / Direction Preference；D8 presentation | device/session 的 `ltr|rtl|auto` 呈现偏好；非 locale、作者方向或 Query 排序 | code type `DirectionPreference`；值来自 Direction §2 的 shell/document/session preference，不新增 author wire | CLI 无；UI 继续使用现有方向选择器和 `ltr|rtl|auto` 选择值；不冻结新的概念 locale key | 可简称 direction preference；禁止 locale/author-dir alias；删除任何把偏好写回作者 source 的受控路径 |

kind 归属是完整单值映射：每个 kind 恰有一个 interface owner，多个 kind 可以共享该 owner。例如 d8_draft_project 与 d8_draft_projection 同归 D8 Draft Projection Interface，不要求反向唯一。“technical interface owner”表示负责解析并返回该消息的 D8 接口合同，不是第十个 domain concept；领域概念只作为 consumed/returned 数据单独列出：

| kind | 唯一 technical interface owner | consumes / operates on | returns / domain concepts |
| --- | --- | --- | --- |
| `d8_document_read` | D8 Document Read Interface（Editor Interfaces §2；文档读取接口） | 消费 D3 NodeRef 与当前 D6 读取授权 | 返回 `d8_document`；D2 document_snapshot 语义继续归 D2 |
| `d8_document` | D8 Document Read Interface（Editor Interfaces §2；文档读取接口） | 表示准确 read request 的结果 | 承载 D2 document_snapshot 与 D8 Draft/selection shell projection；不创建新 identity concept |
| `d8_draft_project` | D8 Draft Projection Interface（Editor Interfaces §3.1；草稿投影接口） | 消费 Edit Draft 与当前 author cut | 返回 `d8_draft_projection` |
| `d8_draft_projection` | D8 Draft Projection Interface（Editor Interfaces §3.1；草稿投影接口） | 消费一份 Edit Draft proposal | 返回携带 Draft Edit Map 的 Draft Projection |
| `d8_draft_text_replace` | D8 Draft Text Replace Interface（Editor Interfaces §3.3；草稿文本替换接口） | 消费 Edit Draft 与 Draft Edit Map 的 segment/range binding | 返回 `d8_draft_text_replaced` |
| `d8_draft_text_replaced` | D8 Draft Text Replace Interface（Editor Interfaces §3.3；草稿文本替换接口） | 表示准确 replacement 的结果 | 返回 Draft Projection，本响应没有 caret 成员；Draft Edit Map 只作为 projection 内消费/返回的数据，不与 kind 共 owner |
| `d8_draft_write` | D8 Draft Write Interface（Editor Interfaces §3.4；草稿写入接口） | 消费 Edit Draft 与 Draft Edit Map 给出的写入位置绑定 | 返回 `d8_draft_written` |
| `d8_draft_written` | D8 Draft Write Interface（Editor Interfaces §3.4；草稿写入接口） | 表示准确 write 的结果 | 返回 Draft Projection 与 caret；Draft Edit Map 是数据，不是 kind owner |
| `d8_edit_prepare` | D8 Edit Prepare Interface（Editor Interfaces §4；编辑准备接口） | 消费 Edit Draft，并按 Prepared Edit Binding 规则构造准备态 | 返回 `d8_edit_prepared`；嵌套 D6 commit request 继续归 D6 |
| `d8_edit_prepared` | D8 Edit Prepare Interface（Editor Interfaces §4；编辑准备接口） | 消费受保护的 Prepared Edit Binding 结果 | 返回 preview/commit envelope；不声明 author success |
| `d8_undo_prepare` | D8 Undo Prepare Interface（Editor Interfaces §7；撤销准备接口） | 消费当前 bytes/revision 与原 receipt/effects | 返回 `d8_edit_prepared`；只准备 inverse edit，不拥有 Undo history |
| `d8_editor_error` | D8 Editor Error Interface（Editor Interfaces §6；编辑错误接口） | 表示 D8 public interface 的失败 | 返回 closed D8 editor error family；不创建 durable concept ID |
| `d8_prepared_edit_binding` | D8 Prepared Edit Binding Internal Interface（Editor Interfaces §4.2；准备绑定内部接口） | 消费不可变 prepared request/preview/authorization coordinates | 返回受保护 `PreparedEditBinding/2`；绝不是 public response kind |

`document|annotation` 继续只是 D8 intent 局部 discriminator，不是实体 kind。上述补充不增加任何 D8 wire member、error、IME 状态或自动确认路径。

### 8.2 D9 owner 词表追加条款

本命名补充自身不修改 D9 行为或 wire；下表消费实际 D9 文件权威后像的当前版本。Workspace 接口与模板/导出生产端显式采用 /2，纯工件 Worker 保持 /1，真实旧决议继续原 decoder/恢复，版本化语义归实际 D9 owner。下表把 D9 lexicon 中的分组词拆成稳定 concept ID，并把八份 D9 来源新增的主要受控 type/profile 归到唯一 owner。所有 D9-owned 行的原 concept-contract `firstFreeze` 继续是已接受 D9 r04 合同；`weftext.term.import-job` 是明确继承例外：D6 已拥有 concept、names、locale 与历史 firstFreeze，D9 只消费它。R08 提议追加 naming 与逐 kind technical-interface-owner metadata，并具有独立 R08 candidate 来源；不能倒称这些逐项 ID/owner 已经作为 S 原行存在。除 D9 已冻结的语义流程 `prepare→inspect→publish/state/cancel` 外，不新增 per-concept CLI verb、独立 UI 控件名或 locale key；内部概念明确写无。不存在已发布兼容 alias，迁移只清理未发布旧受控名称。

迁移删除目标代码：M-import=旧 source ID/`ImportIr`/YAML proposal decoder、fixtures、help、generated samples；M-route=自由 command/fallback/provider alias 与 route inventory；M-template=旧 attr/record/H1–H9/formula-reorder/宽泛 view 模板 parser/help/samples；M-export=自由 binding dictionary、rowHandle identity、author-snapshot export、generic author-receipt alias；M-region=D9 私有 Locator kind/opaque registry identity；M-none=没有具体旧原型，仅删除实现中未列受控 alias。历史研究文本不迁移。

| concept ID | 正式中英名；owner | canonical type/profile/kind 归属 | 定义 / 排除 | CLI/UI/locale；简称/alias | 迁移目标 |
| --- | --- | --- | --- | --- | --- |
| `weftext.term.source-artifact` | 来源工件 / Source Artifact；D9 input control | `SourceArtifact` descriptor；`d9_probe` 等输入 | host pin 的外部 bytes；非 Node、SourceBinding、identity | 无独立 CLI/locale；UI 可显示来源工件；禁止 path/hash identity | M-import |
| `weftext.term.import-ir` | 导入中间表示 / Import IR；D9 conversion | `ImportIR/1`, format `weftext.conversion-ir` | 有界 typed conversion IR；非第三方 AST/作者源 | 无直接 UI/locale；简称 IR 限 D9；禁旧 ImportIr alias | M-import |
| `weftext.term.source-location` | 来源位置 / Source Location；D9 IR evidence | `SourceLocation` closed union | 输入版本内坐标；非 D3 Locator/写 target | 无 CLI/UI/locale；禁止 Locator alias | M-import |
| `weftext.term.import-observation` | 导入观察 / Import Observation；D9 IR evidence | `Observation` exact object | origins/confidence/method 提取证据；非 provenance 授权 | 无 CLI/UI/locale；简称 observation | M-import |
| `weftext.term.conversion-provider` | 转换提供方 / Conversion Provider；D9 host registry | Provider installed record | 经版本/依赖/许可/沙箱审查的适配器；非 Model Provider | 管理 UI 可显示签名提供方名称；不冻结 locale key/CLI verb；裸 Provider 仅 D9 上下文 | M-route |
| `weftext.term.conversion-route` | 转换路由 / Conversion Route；D9 host registry | routeId/profileId + fixed Provider chain | 有限有序无循环流水线；非自由 fallback | 管理 UI 可显示 Route；无新 locale/CLI | M-route |
| `weftext.term.import-mapping` | 导入映射 / Import Mapping；D9 Core mapping | `ImportMapping/1` | Core 明确结构/字段转换选择；非 Query/Action | 分析 UI 可显示映射；无独立 locale key/CLI | M-import |
| `weftext.term.mapping-proposal` | 映射提案 / Mapping Proposal；D9 proposal | `d9_import_analysis` 中固定 proposal 语义 | prepare 前完整提案；非 author plan/patch | UI 通过 import analysis 展示；无独立 locale/CLI | M-import |
| `weftext.term.import-job` | 导入作业 / Import Job；**owner D6，D9 消费** | 完整继承 D6 owned names `ImportJob`、`importJob`、`stageInput`、`planAtomicGroups`、`commitImportBatch`；D9 `d9_import_*` 只观察/操作该 D6 record | 固定有限 groups/batches；非第二 ledger/全 job 原子事务 | 继承 D6 UI/locale `storage.import_job`；firstFreeze 保持 `D6 revision05-observation-bootstrap candidate; not activated`；无 D9 兼容 alias | M-import |
| `weftext.term.coupling-group` | 耦合组 / Coupling Group；D9 mapping | ConversionInput/prepare 内 group | 不可拆作者组；非 UI page/worker process | 无 CLI/UI/locale | M-import |
| `weftext.term.import-batch` | 导入批次 / Import Batch；D9 mapping | 一个原 D3/D6 atomic request | 有限提交批次；非任意 1000 条切块 | UI 可显示批次进度但无新 locale key | M-import |
| `weftext.term.conversion-input` | 转换输入证据 / Conversion Input；D9 控制域 | `ConversionInput/2` ZIP/profile | 绑定原始输入、IR、映射、损失和路由；非身份或作者源 | 无直接 UI/CLI/locale | M-import |
| `weftext.term.worker-invocation` | Worker 调用记录 / Worker Invocation；D9 Worker 控制域 | `WorkerInvocation/1` | 固定作业、步骤、路由、输入、选项和预算；没有路径或命令字段 | 无 UI/CLI/locale；Worker 不得自报授权 | M-route |
| `weftext.term.template-recipe` | 模板配方 / Template Recipe；D9 template | `TemplateRecipe/2`, format `weftext.node-template` | 对 D2 Template 的一次 fresh construction 配方；非第二 Template identity | Template UI 可显示配方；无独立 locale/CLI | M-template |
| `weftext.term.template-construction-input` | 模板构造输入 / Template Construction Input；D9 control | `TemplateConstructionInput/2` | 固定 pins/recipe/params/resource/loss 证据；非 author source | 无直接 UI/CLI/locale | M-template |
| `weftext.term.office-template` | Office 模板 / Office Template；D9 template | 普通 Office template profile | 普通文本模板 bytes；非宏/脚本 | UI 可称 Office 模板；无固定 locale key/CLI | M-template |
| `weftext.term.template-placeholder` | 模板占位符 / Template Placeholder；D9 template | Templates token/binding contract | 值插入位置；非 content control/named range | 用户可见 placeholder 文本；无独立 locale key | M-template |
| `weftext.term.style-directive` | 样式指令 / Style Directive；D9 template | Templates style directive | 可见 style 样板；非执行命令 | UI 可预览，不冻结 CLI/locale | M-template |
| `weftext.term.repeat-band` | 重复带 / Repeat Band；D9 template | 单完整 row/column repeat contract | dataset 重复结构；非 Excel Table 增强层 | UI 可显示重复区域；无独立 locale/CLI | M-template |
| `weftext.term.render-snapshot` | 渲染快照 / Render Snapshot；D9 export | Templates `RenderSnapshot` finite union | 已授权显式渲染投影；非 author snapshot/import | 无直接 CLI/locale；inspect UI 可展示 | M-export |
| `weftext.term.d7-result-pin` | D7 结果固定证据 / D7 Result Pin；D9 control over D7 | `D7ResultPin` 内部 pin；嵌套 `TerminalSchema`/V 继续归 D7 | 固定完整 D7 终态结果/epoch/auth/cut；非 rowHandle identity | 无 UI/CLI/locale；禁止 D7 result identity alias | M-export |
| `weftext.term.export-plan` | 导出计划 / Export Plan；D9 export control | `ExportPlan/2` | 当前输出不可变准备记录；非 D6 PreparedIntent/author ledger | export prepare/inspect UI 消费；无独立 locale key | M-export |
| `weftext.term.export-input-catalog` | 导出输入目录 / Export Input Catalog；D9 export | `ExportInputCatalog/2` | Plan 的完整受权输入目录；非作者 source | 无独立 UI/CLI/locale | M-export |
| `weftext.term.export-content-selection` | 导出内容选择 / Export Content Selection；D9 export | `ExportContentSelection/1` | body/bibliography 明确消费选择；非授权 grant | inspect UI 显示选择；无 locale key | M-export |
| `weftext.term.export-projection` | 导出投影 / Export Projection；D9 export | `ExportProjection/1` | bindings/datasets 的有限 Core 投影；非自由 dictionary | 无独立 locale/CLI | M-export |
| `weftext.term.export-input-location` | 导出输入位置 / Export Input Location；D9 evidence | `input|source_range|annotation|query_cell|query_scalar|template_range` closed union | Plan 内证据位置；非 Locator/identity | 无 UI/CLI/locale | M-export |
| `weftext.term.export-loss-location` | 导出损失位置 / Export Loss Location；D9 evidence | ExportInputLocation + `binding|dataset_cell|block` | 同 Plan 的损失定位；非跨 Plan 地址 | 无 UI/CLI/locale | M-export |
| `weftext.term.export-loss-report` | 导出损失报告 / Export Loss Report；D9 export | `ExportLossReport/1`, format `weftext.export-loss` | 固定提案损失报告；非 import LossReport | inspect UI 显示完整报告；无独立 locale key | M-export |
| `weftext.term.export-blocks` | 导出块投影 / Export Blocks；D9 export compiler | internal `ExportBlocks` | D2 已知结构的只读编译结果；非新 author block ID | 无 UI/CLI/locale | M-export |
| `weftext.term.staged-output` | 暂存输出 / Staged Output；D9 publication control | validated staging record | 未发布完整输出；非 author revision | publish/state UI 消费；无独立 locale key | M-export |
| `weftext.term.publication-receipt` | 外部发布回执 / Publication Receipt；D9 publication | `PublicationReceipt/2` | 外部文件发布事实，只绑定 plan/output/destination | UI 可显示“已发布”；无独立 locale key；**禁止 D3/D6 author receipt alias** | M-export |
| `weftext.term.import-loss-report` | 导入损失报告 / Import Loss Report；D9 import/template | `LossReport/1` | 导入/Node Template 固定损失选择；非 export report | analysis UI 显示；无独立 locale key | M-import |
| `weftext.term.import-loss-issue` | 导入问题 / Import Loss Issue；D9 IR | IR issue exact object | 原始观察问题；非安全批准或完整性证明 | analysis UI 可显示 message；无独立 locale key | M-import |
| `weftext.term.template-loss-location` | 模板损失位置 / Template Loss Location；D9 模板域 | `template_source|template_annotation` union | 模板输入固定证据或明确省略证据；非 D3 Locator | 无 UI/CLI/locale | M-template |
| `weftext.term.image-physical-size` | 图片物理尺寸 / Image Physical Size；D9 image profile | profile `image_physical_size/1` | 可验证源物理事实/量化；非宿主 DPI 猜测 | inspect UI 可显示尺寸；无独立 locale key | M-none |
| `weftext.term.image-size-selection` | 图片尺寸选择 / Image Size Selection；D9 export | `imageSizes` Plan member | 用户冻结的布局选择；非源物理事实 | export UI 可显示选择；无独立 locale key | M-none |
| `weftext.term.region-body` | 区域几何体 / Region Body；D9 geometry | `RegionBody` + profile `d9rg1` | 非身份几何事实；非完整 Locator | 无 UI/CLI/locale | M-region |

继承 owner 必须显式保持：

- D2 `Template` meta-kind 仍拥有 Node Template 的作者身份；D9 只拥有 `TemplateRecipe/2`/construction evidence。
- D7 `TerminalSchema`、V 值代数和 `PreparedActionBinding/3` 继续归 D7；`D7ResultPin` 只 pin 这些既有事实。
- D3 `SourceBinding`、`ForeignIdentityKey`、`OriginBinding`、`ResourceRegionLocator/l1` 与 D3/D6 author receipt 继续归原 owner。D9 的 `RegionBody/d9rg1` 只是内层几何，`PublicationReceipt/2` 只能表示外部发布。
- D6 `SourceVersion`、`BudgetBinding`、Token 与 job/commit 权威不改。

D9 owner lexicon 还为17个 public kind 逐项记录唯一 technical interface owner。这些 interface-owner label 不创建17个新的 domain concept；上表36个 D9-owned concept 继续构成领域词表，D6 ImportJob 仍是唯一继承自 D6 的 concept。

| kind | 唯一 technical interface owner | consumes / operates on | returns / owner boundary |
| --- | --- | --- | --- |
| `d9_probe` | D9 Probe Interface（Main §4；探测接口） | 消费 SourceArtifact 与固定 Conversion Route candidate | 只发起探测，不产生 author effect |
| `d9_probe_result` | D9 Probe Interface（Main §4；探测接口） | 消费准确 probe evidence | 返回 probe/profile candidate；SourceArtifact/Route 仍为 D9 concept |
| `d9_convert` | D9 Conversion Start Interface（Main §4；转换启动接口） | 消费 SourceArtifact 与 accepted route/options/budget | 返回 `d9_conversion_started`；worker 继续受 sandbox 约束 |
| `d9_conversion_started` | D9 Conversion Start Interface（Main §4；转换启动接口） | 表示一次 accepted start | 返回 job token/status coordinate；不是 author identity |
| `d9_conversion_state` | D9 Conversion Job Interface（Main §4；转换作业接口） | 消费 conversion job token | 返回 `d9_conversion_state_result` |
| `d9_conversion_state_result` | D9 Conversion Job Interface（Main §4；转换作业接口） | 消费 WorkerInvocation/IR job state | 返回 state/validated IR result；不是 author receipt |
| `d9_conversion_cancel` | D9 Conversion Job Interface（Main §4；转换作业接口） | 消费 conversion job token | 请求取消；不改变已 committed 的作者事实 |
| `d9_import_analyze` | D9 Import Analysis Interface（Main §4；导入分析接口） | 消费 ImportIR 与 ImportMapping/Loss inputs | 返回 `d9_import_analysis` |
| `d9_import_analysis` | D9 Import Analysis Interface（Main §4；导入分析接口） | 消费固定分析切面 | 返回 MappingProposal 与损失/对象/组/批次目录 |
| `d9_import_choose` | D9 Import Analysis Interface（Main §4；导入分析接口） | 消费 analysis token 与明确 loss/mapping choices | 返回 successor analysis；不是 D6 commit |
| `d9_import_prepare` | D9 Import Preparation Interface（Main §4；导入准备接口） | 消费 fixed analysis 与继承 D6 ImportJob 的 group/batch control | 返回 `d9_import_prepared`；真实 author plan 继续归 D7/D3/D6 |
| `d9_import_prepared` | D9 Import Preparation Interface（Main §4；导入准备接口） | 消费准确 prepared batch | 包装/返回原 D7 prepared outcome；不创建新的 author receipt |
| `d9_import_next` | D9 Import Preparation Interface（Main §4；导入准备接口） | 消费当前继承 D6 ImportJob 与之前 authoritative batch outcome | 只为唯一下一有限 batch 返回 `d9_import_prepared`；完整/当前 job state 另由 `d9_import_state` 取得，本入口不新增 terminal success 分支 |
| `d9_import_state` | D9 Import Job State Interface（Main §4；导入作业状态接口） | **operate on D6-owned ImportJob** | 返回 `d9_import_state_result`；interface owner 是 D9，record/concept owner 仍为 D6 |
| `d9_import_state_result` | D9 Import Job State Interface（Main §4；导入作业状态接口） | 消费 D6 ImportJob 与关联原 receipts | 返回只读 progress/result projection；D3/D6 receipt 保留原 owner |
| `d9_template_analyze` | D9 Node Template Analysis Interface（Templates §6；节点模板分析接口） | 消费 D2 Template 与 D9 TemplateRecipe/TemplateConstructionInput | 返回 `d9_import_analysis`；D2 Template identity 继续归 D2 |
| `d9_error` | D9 Error Interface（Main §4；错误接口） | 表示 D9 public-interface failure | 返回 closed D9 error family；进入 D3/D6/D7 后继续原 owner error |

未知 kind、profile 或 version 仍拒绝。

这些 D8/D9 词表补充只完成命名门，不改变现有 wire、身份、权限、作者提交、IME、模板、转换、worker、导出或发布语义。

## 9. 激活与版本兼容

这些 amendment 只有在独立审查接受、总控协调裁决并与 D10 candidate 一起形成后续设计版本后，才改变规范；**设计接受/协调激活不是产品实现、发布或运行时 availability 证据**。

首版 D10 公开能力由 D10 定义、D1 发现/发布的候选 ID：automation.manage、workspace.extensions.manage、deployment.external.manage、automation.stop、automation.author_submit。它们必须先进入所选 D1 contractMajor 的正式 capability catalog，并继续经过 D1 已冻结的 release、surface、policy、principal、component/configuration、reachability/version-combination 与 health 门。unknown ID 仍为 D1 unsupported_feature；不允许的组件版本组合仍为 incompatible_version。D10/D6 不增加第二个产品级协商器。

第一份共同公开 unattended author-submit D6-Control/2 error 闭集在该能力真正发布时已经包含 approval_unavailable 与 execution_stopped。不存在旧 D10 author-submit error profile、enum fallback、migration parser 或双读写。这里删除的只是未发布 D10 profile 假设；S 已冻结的 Policy/1/2、bootstrap profile/1/2、IssuerControlPolicy、D1 bootstrap/contractMajor 以及历史 saved decision decoder 全部保留。

运行时发布前，所有 D7/D8 author proposal 仍使用当前逐次确认，planned-preview recovery 新入口和 unattended author submit 均保持真实 D1 unavailable。实现可以独立推进不涉及无人值守作者提交的 Broker、Agent proposal、Tool/MCP、Connector read 和 external-effect control，但每项仍须自己的 D1 capability 和真实实现证据。

历史 committed D3/D6 decision 继续按原 decoder/bytes 重放，不回填 ApprovalUse、stop 或 d10_control_self。已有 family 保留其固定 bootstrap profile；没有任何升级路径把 profile/2 静默解释为 profile/3。

## 10. 独立复审必须攻击的联合反例

1. R1 prepare 初检批准有效，R2 抢占最后次数；R1 进入 D6 后只因 approval 竞争失败，必须得到 `approval_unavailable/preflight`，unseen 无 ledger decision。
2. R1 已 planned 后 approval 撤销；同 error 保持 planned，不能 semantic_rejected/terminal_failed。
3. 新客户端无旧 preview 副本，但当前有权且连续性成立；能通过 `d7_planned_preview_open` 完整读取原 preview，而不是 reprepare。
4. recovery preview 不得重跑 Query/换 target；current definition 已漂移也只能显示原保存语义。
5. 新 PlannedDecisionApproval 不能复活确定 dependency conflict。
6. Approval count=1 两个 Run 同时 planning，至多一个 `reserved`。
7. reserved plan 后确定 dependency conflict 触发 authoritative terminal_failed；同事务变 `released_terminal`，replay 不二次 release。
8. cancel、TTL、临时撤权、Lease 过期均不能释放 approval reservation。
9. model/tool cost 已发生或 uncertain 时，author terminal_failed 不改变费用终态。
10. set_field_member 请求值已等于 current value：MutationFootprint/field_change/sourceVersions 为空，但原 target/type/value/permission/dependency 全部验证，committed 仍消费一次 approval count。
11. 两个同值 Field Entries 不能选择 first。
12. footprint 偷改 note/provenance/Facet/第二 member 必须拒绝自动批准。
13. author commit 成功、receipt 丢失、approval 后来过期，重放原 receipt 不再扣次数。
14. D8 dirty Draft 与合法 background author commit 组合，Draft 不被覆盖。
15. D3 create/lifecycle Action 不能借 `single_field_member` envelope 自动提交。
16. D10 control error 不能包装或泄漏原 D6 `not_visible` 与隐藏事实。
17. `maxRuns=1` 且同 Run 已有 LeaseRunUse 时，第二个受保护步骤和原 planned 恢复不能因 remaining=0 返回 `delegation_exhausted`；新 Run 才应被拒绝。
18. Adopt 正向代码只保留 `adopt_*`，关联绑定只使用 `OriginBinding` / `origin_binding`；`adoption_binding` 在正向术语/代码面必须不存在，也不能作为 alias。
19. profile/2 family 在软件升级后不得自动获得 d10_control_self；profile/3 只影响显式 issuer update 后的新 family，既有 Workspace 只能由当前 policy_admin 显式授予。
20. stop 与新 Run admission、D6 final author commit 的两种先后都必须只有一个线性化结果；stop 后仍允许受权 authoritative abort、cost settlement 和 evidence retention。
21. same stable Workspace-control request 已成功但响应丢失，随后对象 revision 改变；当前披露授权通过后必须重放旧结果，不能重复 mutation，也不能因 current revision 改变误拒绝历史成功。
22. D8 九个 concept ID 保持各自独立；十三个 kind 在 owner lexicon 中必须各有且仅有一个 technical interface owner，多个 kind 可以共享 owner，包括 d8_draft_project/d8_draft_projection。领域数据所有权另列，不要求反向唯一。任何 unlisted CLI/locale/wire alias 失败，且不得因此改变 IME/Write/Read/confirm/Undo。
23. D9 grouped lexicon 必须拆出稳定 concept ID；`PublicationReceipt/2` 只能映射外部发布事实，`D7ResultPin` 的 nested TerminalSchema/V 仍归 D7，PreparedActionBinding/SourceBinding/OriginBinding 等继承 owner 不得被 D9 重注册。

这些是作者修订后的审查靶点，不表示前两批独立问题已经关闭。只有后续独立复审才能改变其审查状态。
