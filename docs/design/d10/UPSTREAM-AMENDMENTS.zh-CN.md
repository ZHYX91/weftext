---
source_language: zh-CN
translation_status: source
---

[English](UPSTREAM-AMENDMENTS.md)

# D10 配套 D6/D7 上游协调修订提案

revision: D10-r04-final-review-fixes-2026-09-28；状态：candidate upstream amendment proposal，尚未共同接受或协调激活。固定上游输入提交为 `f205831c848729f7ddbc3ba0cf32b689459c0c98`。当前 D1–D9 快照继续权威；本文只给独立评审一份完整未来配套文本，不修改快照，也不授权产品提前实现无人值守作者提交。

## 1. 修订目的和不变边界

现有 D7 已冻结 fresh Action prepare、完整 preview/effects、原 D3/D6 request、重放和 unknown recovery；现有 D6 已冻结 current authorization、planned/commit CAS、唯一 author ledger 与最终 author commit point。D10 需要补足三类原上游尚未定义的组合语义：

1. Standing Approval 在进入 D6 后发生竞争失效时，D6 必须有唯一可表达且不产生永久拒绝的错误。
2. 原 D6 decision 已经 planned、原 preview transport 已过期时，当前有权用户必须能只读重新查阅**原保存 preview**，而不是重新 prepare。
3. ApprovalUse 次数 reservation 在 authoritative `terminal_failed` 时必须有唯一终态，且与费用 reservation 分开。

本提案仍只开放 D10 candidate 定义的一个窄自动作者 profile：existing Node + one Field + 当前完整 Field 恰一 Entry + 一个 existing scalar member 的原 D7 `set_field_member`。其它 D7/D8/D3 author mutations 继续逐次交互确认。

以下保持不变：

- D3 wireVersion11、Result/9、mode、identity/lifecycle stages 与 receipts；
- D6 `d6_commit_request` 与 `d6_commit_receipt` exact shape、Workspace+OperationId ledger key 与唯一 author commit point；
- D7 `ActionSpec`、`d7_action_prepare`、`d7_action_prepared` 与 `PreparedActionBinding/2` exact shape；
- D7 `EffectManifest/1`、EffectBytes item/schema 与 committed transport；
- D8 `PreparedEditBinding/1`、Draft/IME/explicit confirmation；
- D4 Registry、D7 Narrow Field Qualification 与 D6 Policy/ObservationScope；
- 原当前授权、deny 优先级、信息不披露、authority/fence、dependency CAS、replay 与 planned 恢复规则。

D6 `d6_error` object shape 与 disposition 集合保持，但第一份共同公开 unattended author-submit 合同的 code 闭集需要协调加入 `approval_unavailable` 与 `execution_stopped`。它们都是必要的 closed-enum 扩展，不伪装成既有错误，也不建立匿名 old/new D10 error profile。D7 Preview/Effects 需要增加 planned-only 只读恢复入口；原 `d7_effects_resolve/open` 继续 committed-only。R05 还提议 Policy/2 新增 `d10_control_self`、固定 bootstrap profile/2 的原非 Field 集合并新增显式 profile/3。以上都只是设计提案；独立接受/协调激活不等于产品已经实现或 D1 capability 已可用。

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

> 对采用 D10 Standing Approval 的 D6-owned Action，D7 prepare 完成后，Core 必须依据已保存的 PreparedActionBinding/2、完整 owner_fields preview、实际 MutationFootprint、current D6 authorization 及 D10 当前控制记录，独立构造受管 `ApprovalUse/1`。客户端仍只持原 `d6_commit_request`；不得向 request 增加 approved、approvalId、delegation、proof、budget override 或 effect override。
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
> 一旦 D10 自动路径已把 ApprovalUse 作为内部依赖关联原 planToken 并进入 D6，只有“批准依赖在竞争中不可用、而原 D6 current author authorization/ObservationScope 与适用业务可见性仍成立”这一情况使用新增 `d6_error.code="approval_unavailable"`，disposition 固定 `preflight`。
>
> 原 D6 permission failure 仍为 `not_visible/preflight`；业务 dependency、semantic、budget、authority/integrity 错误仍返回原 code/disposition。不得用 `approval_unavailable` 遮蔽它们。
>
> 对 unseen request，`approval_unavailable/preflight` 不写 author ledger、recorded rejection 或 planned；原 prepare 若仍有效，可在新的合法批准下重试，或回到交互路径。对已 planned request，同一错误保持 ledger=planned 并保留原 plan/pins/reservations；不能写 `semantic_rejected` 或 `terminal_failed`。
>
> D10 adapter 必须逐字传递这个 D6 error，不包装成 D10 approval error。调用端若要显示“批准已撤销/过期/次数被抢占”等细分原因，必须另经当前受权的 D10 control read，不能从 D6 author error 偷带控制细节。
>
> 这是第一份共同公开 unattended author-submit D6-Control/1 error 闭集的一部分。固定 S 的 D6 是尚未发布的设计合同，不构成必须运行时兼容的旧 D10 profile。能够进入该正式 branch 的组件组合必须支持同一闭集；不兼容组合在 D1 capability/version gate 前拒绝。Policy/1/2、bootstrap profile/1/2 与历史 saved decision decoder 均保持。

### 2.4 planning CAS 原子 reserve

建议扩展 D6 planned CAS：

> 对带 ApprovalUse 的 unseen D6 request，原步骤6业务、authorization、dependency、semantic、budget 验证完成后、写 planned 前，planning CAS 还必须比较 current approval/delegation/activation revision、ApprovalUse 与 request/plan/preview/footprint 的准确绑定、approval 的时间与 revoked 状态、`consumed + reserved < maxSuccessfulCommits`，以及所有适用 budget/cost reservation 的旧版本。
>
> 只有全部成立，才能在同一 database write transaction 写 planned、完整固定 plan/pins、ApprovalUse count `unreserved→reserved` 与相关资源 reservation。两个 request 竞争最后一个 approval count 或最后预算时至多一个 winner；loser 回原 restart gate，不能把先前读取的余额当 current fact。
>
> same Workspace+OperationId 的 canonical replay 始终关联同一 reservation，不能重复 reserve。未进入 planned 的临时 ApprovalUse 可以按准备寿命清理；一旦 planned 引用，approval reservation 与完整恢复 pins 按原 ledger recovery 生命周期保留，不受 preview TTL、Agent session 结束或普通 cache GC 清理。

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

在 §2 “有效 adapter 可生成受管 PreparedIntent”段之后新增：

> D10 Broker 本身不是 PreparedIntent producer。D10 Agent/Automation 的工作区修改只能调用已有 D7/D8/Core closed adapter。对于 `single_field_member`，D7 prepare 先按原合同生成 PreparedActionBinding/2、PreparedIntent、preview 和原 `d6_commit_request`；随后 Core-managed D10 approval adapter 才可从受保护记录建立 ApprovalUse。D10 runtime 不得直接构造 PreparedIntent、MutationFootprint、source bytes 或 proof。
>
> ApprovalUse 不改变 `d6_commit_request` exact members、canonical request key、planToken tag、protocolOwner 或 ledger key。它通过受保护内部关联绑定原 planToken；外部 caller 没有可追加 approval 字段。

### 3.2 §2 步骤5–8的完整增补

对 D10 standing-approval profile，原步骤1–8保持，只增加：

> **步骤5附加：** 解析本 principal 的 planToken 后，如果该自动提交路径已经关联 ApprovalUse，只通过已授权最小 control mapping 定位其 profile/record。missing、wrong audience、wrong Workspace 保持 `not_visible`。完整 ApprovalUse 必须在 current D6 authorization/ObservationScope 通过后读取。
>
> **步骤6附加：** 除原业务依赖、semantic、budget 验证外，验证 `single_field_member` 两个 effect 分支、current approval/delegation/activation 和 count/cost reservation candidate。若原 D6 permission/business 失败，返回原 owner error；只有这些均仍成立而 ApprovalUse 不再可消费时返回 `d6_error approval_unavailable/preflight`。对 unseen 不写 decision。
>
> **步骤7附加：** planned CAS 原子保存 plan/pins，并把 ApprovalUse count `unreserved→reserved`；CAS loser 回原步骤3。任何 reservation 不提前产生 author effect。
>
> **步骤8附加：** planned 恢复或 final author commit 前再次检查 current D6 authorization 和本次 approval eligibility。仅 approval 不可用时返回 `approval_unavailable/preflight` 并保持 planned；确定业务冲突仍沿原 terminal_failed 条件。成功 commit 同事务把 count `reserved→consumed`；authoritative terminal_failed 同事务把 count `reserved→released_terminal`。

### 3.3 §3 回执与错误的精确扩展

将 `d6_error.code` 闭集扩展为：

```text
invalid_request
not_visible
authority_unavailable
integrity_conflict
operation_id_conflict
plan_expired
approval_unavailable
execution_stopped
dependency_conflict
semantic_rejected
budget_exceeded
transaction_aborted
```

`approval_unavailable` 只允许 disposition=`preflight`。它表示 coordinated D10 author-submit 的 approval dependency 在进入 D6 后不可消费，但原 D6 author permission/ObservationScope 与适用业务前提没有更早失败。它不生成 recorded rejection，不泄露 approval 内部原因。

`execution_stopped` 同样只允许 disposition=`preflight`，且 current authorization/ObservationScope 必须先通过。它只表示本 request 绑定的不可逆 ExecutionStopLatch 已在线性化顺序中阻止新的 author submit；不得替代 `not_visible`、approval 错误、temporary disable、Lease expiry 或业务 dependency。unseen 不写 ledger；planned 按 §3.5 的严格 authoritative-abort 条件处理。

原三阶段/key/权限/availability 规则不变；`dependency_conflict|semantic_rejected|budget_exceeded` 仍只在原步骤6可 recorded；planned 后永久 abort 仍只有 `transaction_aborted|terminal`。一个旧 consumer 未协商 D10 standing-approval capability 时不能收到新增 enum。

Standing Approval 不增加 receipt member。D10 UI 可以从受权 Run/ApprovalUse control state 显示使用来源，但它不是 author receipt。

### 3.4 D10 control state 与 Lease Run admission

ActivationBinding、DelegationLease、StandingApprovalEnvelope、AutomationDefinition、LeaseRunUse、PlannedDecisionApproval 等耐久 D10 控制状态只能由 Core 管理的封闭 control adapter 修改，并具有独立 control revision/CAS、当前 principal authorization、audit 与 replay 合同。它们不能经作者 source、provider callback、Agent JSON 或 `d6_commit_request` 自由 payload 修改。

`maxRuns` 消费发生在第一次受保护执行前的 D10 Run-admission CAS，不进入 D6 author ledger。CAS 按同一 `leaseId` 谱系累计历史，写 `LeaseRunUse/1`；同 Run restart 不重复消费，准入后的 failed/cancelled/crash 不退款。只有一个**尚无 LeaseRunUse 的新 Run**在累计消费已达到 `maxRuns` 时才返回 D10 `delegation_exhausted`。已经有完整 `LeaseRunUse/1` 的同 Run 后续受保护步骤和原 planned request 恢复不再比较 remaining count，也不再次消费；即使 `maxRuns=1` 且累计已经为 1，也不能把该同 Run 当成新 Run 拒绝。它们仍必须逐步验证当前授权、准确 `leaseRevision`、可信时间、ActivationBinding、批准和预算；撤销、过期、revision/binding 变化或连续性不可证明仍会阻止执行。

### 3.5 D10 Workspace 自助管理、bootstrap profile/3 与不可逆 stop

这是对固定 S D6 Control 的显式协调修订，不改写 S 快照。

Policy/2 capability union 增加一个无参数值 d10_control_self，只允许 workspace scope。它允许当前主体通过 R05 CONTROL-CONTRACT 的 closed Workspace control adapter 管理自己拥有的有限 Automation/Lease/Approval/Run 控制记录；它不蕴含 source/Field read-write、policy_admin、registry_admin、binding_admin、repair、commit_sequence_state 或任何 deployment resource。其它能力也不蕴含它；deny 继续优先。

Workspace D10 control adapter 是受管 PreparedIntent producer，但只接受 CONTROL-CONTRACT §7 的 automation_configure、consent、state、workspace_limits、activation 五类 Workspace body。它不能接受 deployment_put、cost_reconcile、secret bytes 或自由 callback。producer 从真实 current principal、完整 closed body、受权读取和 stable prepare binding 生成原 d6_commit_request；external caller 仍不能声明 principal、authorized、effect 或 writer。涉及作者 payload 的最终 mutation 仍只能来自原 D7/D8/D3 adapter；普通 D10 control mutation 不能构造 author source bytes。

固定 S profile/2 的“全部非 Field capability”在本 amendment 中冻结为 S 当时闭集：workspace_state、entity_state、locator_state、source_read、source_write、body_write、node_control、node_create、resource_read、resource_write、annotation_read、annotation_write、lifecycle、registry_admin、binding_admin、policy_admin、export、repair、audit、source_envelope_state、commit_sequence_state。以后新增 capability 不自动进入 profile/2。

新增 d6_bootstrap_profile wireVersion=3；成员仍为 kind,wireVersion,profileRevision,registrySeedBinding,newSeriesMultiplicity,initialPeriodScope。profile/3 的 initialPolicy.version 仍为 2；creator 初始 Workspace grant=上述固定 profile/2 非 Field 集合 + d10_control_self + S 原规则从 target Registry 生成的全部 Field read/write，deny 为空。

只有当前 administer_issuer 的显式 issuer-profile 管理操作才能把以后新 family 切换为 profile/3。既有 family 保存的 profile/1/2 副本、replacement、WorkspaceBootstrapPlan、saved decisions、replay/continue/failover 不重算、不补 grant。既有 Workspace 取得 d10_control_self 只能由当前 policy_admin 走原 Policy 修改事务显式安装；Field 权限主体不能自授。

d6_error.code 的第一份共同公开 closed set 同时增加 execution_stopped，只允许 disposition=preflight。对尚未 planned 的 D10-bound author request：先通过 current authorization/ObservationScope，再检查受保护 RunBinding 对应不可逆 stop latch；stop 已线性化时返回 execution_stopped/preflight，不写 author decision。

D6 final author commit 必须在真正持有 authority-store 写序列化的最终事务内再次验证同一 RunBinding 的 stop latches；不得只在 prepare 或事务外 check。stop 先线性化则本次新 author commit 不发生；commit 先线性化则保存原 committed decision，之后 stop 不倒推撤销。

对 already planned request，stop 本身不能直接写 terminal。只有 current authorization、authority/continuity、原完整 plan/RunBinding 和不可逆 stop 均可证明，并由原 D6 recovery 证明该计划确定永不提交时，才能在原 author ledger 中写 transaction_aborted/terminal；同一 abort transaction 按既定 ApprovalUse 规则释放 approval-count reservation。临时 disable、Lease expiry、普通 Run cancel、暂时撤权或 clock/state 暂不可证都不是该证明，planned 保持 planned。费用 reservation 不因 author abort 自动释放。

stop 不阻断当前获权的 authoritative abort、cost settlement、evidence/audit retention 或 reference-safe cleanup；这些操作不会恢复 executor 或 author-write 权限。

## 4. D7 Execution and Action Interfaces — 拟新增/替换条款

### 4.1 ActionSpec 默认确认句

将 §5 当前“user 看到 preview 后通过原 D3/D6 提交入口确认”的语义替换为：

> ActionSpec 默认确认仍是：当前用户取得并验证完整 preview 后，明确发送 prepare 返回的原 D3/D6 request。只有 D10/1 coordinated `single_field_member` profile 可以在没有本次人机点击时继续：Core 必须从这次 fresh D7 prepare 的完整 PreparedActionBinding/2、EffectManifest/EffectBytes、实际 MutationFootprint 和 current authorization 机械证明有效 ApprovalUse。这个例外不创建 D7 confirmation token、不修改 request，也不表示用户逐字阅读 preview。其它 ActionSpec kind 和 D3-owned intent 继续原交互确认。

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

D7 ActionSpec、PreparedActionBinding/2、EffectManifest/EffectBytes wire 不增加 caller-supplied stop 字段。对于 D10 自动 author-submit，Core 在 prepare 成功后用受保护内部关联把 planToken 绑定到当前 Run/Automation/Lease 与其 ExecutionStopLatch refs；调用方不能删除、替换或自报这些 refs。final submit 由 D6 §3.5 在真正 transaction 内重验 stop。D7 交互路径本身不因 stop 获得新的自动确认能力；stop 只会阻止仍未线性化的后台 submit。

## 5. D7 Preview and Effects Transport — planned 恢复扩展

### 5.1 新 planned-preview 只读入口

现有 `d7_effects_resolve` 保持 committed-only，planned/rejected/terminal/unseen 仍返回 `effects_unavailable`。现有 `d7_effects_open` 继续只接受 committed `effectsToken`。

新增：

```text
d7_planned_preview_open {
  wireVersion: 1,
  kind: "d7_planned_preview_open",
  protocolOwner: "D6",
  request: <original d6_commit_request>
}

d7_planned_preview_opened {
  wireVersion: 1,
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
6. 原 PreparedActionBinding/2、其 semantic preview 与全部必要 pins 完整可证；
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

## 8. D8 与 D9 的明确非修订说明

D8 不增加无人值守编辑分支。Document/Annotation 编辑、dirty Draft、IME、current serial、完整 preview 和明确确认原样保留。Agent/Automation 只能向 D8 提出 proposal；不能伪造 EditSession 或人工来源。

D9 不修改 Provider/Route、worker sandbox、ExportPlan、LossReport、PublicationReceipt 或外部发布器。D10 的外部 transport/Connector 不授予 D9 worker 网络能力，D9 publication confirmation 也不授予 Standing Approval 作者写入权。

## 9. 激活与版本兼容

这些 amendment 只有在独立审查接受、总控协调裁决并与 D10 candidate 一起形成后续设计版本后，才改变规范；**设计接受/协调激活不是产品实现、发布或运行时 availability 证据**。

首版 D10 公开能力由 D10 定义、D1 发现/发布的候选 ID：automation.manage、workspace.extensions.manage、deployment.external.manage、automation.stop、automation.author_submit。它们必须先进入所选 D1 contractMajor 的正式 capability catalog，并继续经过 D1 已冻结的 release、surface、policy、principal、component/configuration、reachability/version-combination 与 health 门。unknown ID 仍为 D1 unsupported_feature；不允许的组件版本组合仍为 incompatible_version。D10/D6 不增加第二个产品级协商器。

第一份共同公开 unattended author-submit D6-Control/1 error 闭集在该能力真正发布时已经包含 approval_unavailable 与 execution_stopped。不存在旧 D10 author-submit error profile、enum fallback、migration parser 或双读写。这里删除的只是未发布 D10 profile 假设；S 已冻结的 Policy/1/2、bootstrap profile/1/2、IssuerControlPolicy、D1 bootstrap/contractMajor 以及历史 saved decision decoder 全部保留。

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

这些是作者修订后的审查靶点，不表示前两批独立问题已经关闭。只有后续独立复审才能改变其审查状态。
