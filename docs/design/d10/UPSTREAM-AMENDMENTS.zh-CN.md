---
source_language: zh-CN
translation_status: source
---

[English](UPSTREAM-AMENDMENTS.md)

# D10 配套 D6/D7 上游协调修订提案

revision: D10-r01-candidate-2026-09-25；状态：candidate upstream amendment proposal，尚未共同接受或协调激活。固定上游输入提交为 `f205831c848729f7ddbc3ba0cf32b689459c0c98`。当前 D1–D9 快照继续权威；本文只给独立评审一份完整、可机械比较的未来配套文本，不修改快照，也不授权产品提前实现无人值守作者提交。

## 1. 修订目的和不变边界

现有 D7 已冻结 fresh Action prepare、完整 preview/effects、原 D3/D6 request、重放和 unknown recovery；现有 D6 已冻结 current authorization、planned/commit CAS、唯一 author ledger 与最终 author commit point。缺口是：D7 默认叙述用户在 preview 后确认，D6 没有定义一份未来 Standing Approval 怎样在最终事务前被机械核验、计数、撤销和恢复。

本提案只开放 D10 candidate 定义的一个窄 profile：existing Node + one Field + 当前完整 Field 恰一 Entry + 一个 existing scalar member 的原 D7 `set_field_member`。其它 D7/D8/D3 author mutations 继续逐次交互确认。

以下均保持不变：

- D3 wireVersion11、Result/9、mode、identity/lifecycle stages 与 receipts；
- D6 `d6_commit_request`、`d6_commit_receipt`、`d6_error` 外形和 Workspace+OperationId ledger key；
- D7 `ActionSpec`、`d7_action_prepare`、`d7_action_prepared` 与 `PreparedActionBinding/2` exact shape；
- D7 `EffectManifest/1`、EffectBytes、delivery epoch 与 preview/committed transport；
- D8 `PreparedEditBinding/1`、Draft/IME/explicit confirmation；
- D4 Registry、D7 Narrow Field Qualification 与 D6 Policy/ObservationScope；
- 原 current authorization、deny precedence、non-disclosure、authority/fence、dependency CAS、replay 和 planned recovery。

因此本修订不触发 D3 镜像、D7 binding version 3、D6 wire version 2 或新的 protocolOwner。

## 2. D6 Storage Transactions Permissions and Sync — 拟新增条款

### 2.1 在权限模型后新增：D10 委托和批准不是 D6 权限来源

建议在 D6 主文权限模型中、current `PrincipalContext` 与 generation/revocation 规则之后新增以下完整规范文本：

> **D10 delegation/approval consumption.** D10 可由 host/Core 受管控制域提供当前 DelegationLease 与 approval evidence，但它们只能收窄已经由本节 D6 Policy、ObservationScope、state disclosure 与实际 footprint 授予的资格，绝不能增加 capability、把 deny 改成 allow 或替代当前授权。普通请求、Agent、worker、Connector、MCP server 与客户端 JSON 均不能自报 principal、delegation、standing approval、approval count 或 proof。
>
> 每个使用 D10 委托的 plan、commit、saved receipt delivery、result/effects delivery 仍先执行本文件已有 current principal/delegation/policy/generation 门。DelegationLease 过期或撤销阻止新的受保护步骤，但不把已 committed decision 改写为失败，也不把已经线性化发送到外部系统的效果回滚。旧 approval 或 capability probe 永远不是持续授权票据。
>
> 只有 D10/1 coordinated contract 明确列出的 author-submit profile 才允许 Standing Approval 替代本次交互确认；当前首版仅为 D7 `set_field_member` 的 `single_field_member` profile。其它 D3/D6/D7/D8 author intent 必须沿原确认合同。D10 approval 无法使一个原本不合法、不可观察、语义冲突、超预算或 stale 的 D6/D7 plan 成为合法。

### 2.2 在 author commit point 前新增：ApprovalUse 是附加授权证据

建议在 D6 主文唯一 author commit point 与 planning/commit dependency 重验规则附近新增以下完整文本：

> 对采用 D10 Standing Approval 的 D6-owned Action，Core 必须在 D7 prepare 完成后，依据已经保存的 PreparedActionBinding/2、完整 owner_fields preview、实际 MutationFootprint、当前 D6 authorization 及 D10 当前控制记录，独立构造一份受管 `ApprovalUse/1`。客户端只持原 `d6_commit_request`；不得向 request 增加 `approved`、approvalId、delegation、proof、budget override 或 effect override。
>
> `ApprovalUse/1` 至少不可变地绑定 approvalId/revision、Run/step、原完整 canonical D6 request、planToken、对应 PreparedActionBinding 记录、完整 preview semantic binding、实际 footprint proof、DelegationLease/ActivationBinding、approval-count reservation 和全部适用 budget/cost reservations。它是当前 author-submit 的附加授权证据，不是第二 author plan、第二 ledger、receipt 或 capability token。
>
> Core 只能为 D10 `single_field_member` profile 建立 ApprovalUse：当前完整 Field 必须恰一 Entry；Action 必须是原 `set_field_member`；owner、FieldId、memberPath、memberType 与 envelope exact-equal；value 满足 envelope 的 closed constraint；D7 Narrow Field Qualification 成功；完整 preview 已经形成且可交付；实际 footprint 只改变该 existing Entry 的一个 existing scalar member。occurrenceKey、其它 members、qualifiers、note、provenance、其它 Entries、body/title/coreKind/Facet、Ref/relation、identity/lifecycle/placement、D6 control state 任一改变都使 standing approval 不适用。Core 不选择 first/preferred/same-value Entry，也不把失败降格到 whole-entry/source write。
>
> Standing Approval 不能隐藏 upstream error。若 Action/Field/preview/D6 permission 本身失败，返回原 D7/D6 error；若业务计划合法但本次没有可消费的 approval，D10 caller 得到其自身的 approval_required/expired 控制结果并可转交互确认，不把它记录成 D6 semantic_rejected。

### 2.3 在 planned CAS 中新增：原子 reserve

建议把 D6 主文对 planned CAS 的规范扩展为：

> 对带 ApprovalUse 的 unseen D6 request，原步骤6全部业务/authorization/dependency 验证完成后、写 planned 前，planning CAS 还必须比较当前 approval/delegation/activation revision、原 ApprovalUse 与 request/plan/preview/footprint 的 exact binding、approval 的有效时间和未撤销状态、`committed + reserved < maxSuccessfulCommits`，以及所有 budget/cost reservation 的可用旧版本。只有全部成立，才能在同一 database write transaction 中写 planned、完整固定 plan/pins、ApprovalUse reserved、approval-count reserved 与相关 resource reservations。
>
> 两个并发 request 竞争最后一个 approval count 或最后预算时，至多一个 CAS winner；loser 回到原 D6 restart gate，不能把旧余额当作新的事实。reserved 不是 committed，也不提前修改作者 source。相同 Workspace+OperationId 的 canonical request 重放永远关联同一 reservation，不能重复占用。
>
> 未形成 planned 的 prepare/preview 记录在过期后可释放自己的临时资源；一旦 planned 引用了 ApprovalUse/reservations，它们按原 ledger recovery 生命周期保留，不能因 preview TTL、Agent session 结束或普通 cache GC 回收。

### 2.4 在最终 author commit 中新增：原子 consume

建议在 D6 主文唯一 final database transaction 描述中新增：

> 对带 ApprovalUse 的 planned decision，最终 author commit 仍逐项执行当前 D6 authorization、authority/fence、原业务 dependencies 与完整原 plan 检查；此外比较 ApprovalUse 仍绑定同一 plan/request、reserved record 完整、当前 delegation/approval 没有使本次新 author submit 失去资格。Standing Approval 的 expiry/revoke 只影响尚未线性化的 author submit；最终 commit 若在 expiry/revoke 的受管事务之前取得 author decision CAS 可完成，否则保持 planned/blocked，不把撤权记录成永久业务 rejection。
>
> 全部通过时，作者 payload/control effects、D6 committed decision、canonical receipt bytes、approval-count 从 reserved→consumed、ApprovalUse terminal link、适用 budget settlement/control effects 与必要 audit link 在同一 final author transaction 发布。raw source no-op 如果原 D6 规则产生 committed decision，也算一次 successful commit 并消费一次 approval count；相同 saved decision 的 replay 不再消费。
>
> 如果 final author transaction 回滚，作者 source、receipt、approval count consumed 和 settlement 全部回滚到原 planned/reserved 状态；不能出现“approval 已消耗但 author 未提交”或反向半态。后续派生 audit aggregation/telemetry 失败不能邀请重复 author request；恢复只看原 ledger/control record。

### 2.5 在 journal/recovery 中新增：撤销、过期与补批准

建议在 D6 主文 recovery 表和 planned 恢复规则后新增：

> planned decision 不因 Run cancel、DelegationLease/Standing Approval 到期或撤销自动变成 terminal_failed。恢复时先执行原 current authorization/non-disclosure，再验证原 plan/dependencies 和本次提交所需 approval evidence。当前 approval 不足时保持 planned/blocked；不生成新 OperationId，不重新跑 Query/重新选择 target，也不修改原 request 或 PreparedActionBinding。
>
> 用户可以针对这份 exact 原 planned decision 给出新的、一次性的明确交互授权；Core 将其作为新的附加 authorization evidence 绑定原 plan，而不是把原 StandingApprovalEnvelope 替换成另一份 envelope。原业务依赖若已发生确定冲突，新的 approval 也不能复活 plan，仍按本文件既有 authoritative abort 条件处理。
>
> 已 committed decision 在 delivery/replay 时不要求旧 Standing Approval 仍未过期；只要求原 saved decision 连续且当前调用者仍有原协议要求的 current authorization。lost receipt 重放返回原 bytes，不增加 approval count、budget spend 或 author revision。

## 3. D6 Control Interfaces — 拟新增/替换条款

### 3.1 §2 PreparedIntent 与提交入口新增 producer 分类

建议在“有效 adapter 可生成受管 PreparedIntent”段之后新增：

> D10 Broker 本身不是 PreparedIntent producer。D10 Agent/Automation 若提出工作区修改，只能调用已有 D7/D8/Core closed adapter。对于 D7 `single_field_member` standing-approval profile，D7 prepare 先按原合同生成完整 PreparedActionBinding/2、PreparedIntent、preview 与原 `d6_commit_request`；随后 Core-managed D10 approval adapter 才可以读取该受保护 prepare record并建立 ApprovalUse。不得让 D10 runtime 直接生成 PreparedIntent、MutationFootprint、source bytes 或 proof。
>
> ApprovalUse 的存在不改变 `d6_commit_request` exact members、canonical request key、planToken tag、protocolOwner 或 ledger key。它通过 planToken/受保护内部记录与原 request 唯一关联；外部 caller 没有可追加的 approval field。

### 3.2 §2 唯一顺序的完整增补

对 D10 standing-approval profile，原 D6 步骤1–8 保持，只有步骤5–8增加以下附加检查：

> **步骤5附加：** 解析本 principal 的 planToken 后，如果其 PreparedIntent 标记需要 D10 Standing Approval，先只通过已授权最小 control mapping 确定 approval profile 和 ApprovalUse 定位；missing/wrong-audience/wrong-workspace 仍 not_visible。完整 ApprovalUse 必须在当前 D6 authorization/ObservationScope 已通过以后才能读取。
>
> **步骤6附加：** 在原业务语义/依赖/预算验证之外，Core 验证 D10 `single_field_member` eligibility、current approval/delegation/activation、完整 preview/footprint 与 count/cost reservation candidate。业务错误仍用原 D6/D7 owner error；仅 D10 approval 不可消费而业务计划本身仍合法时，调用它的 D10 adapter 返回 approval_required/approval_expired，不把该状态保存成 D6 recorded rejection。
>
> **步骤7附加：** planned CAS 原子 reserve ApprovalUse、approval count 与 budgets，和完整 plan/pins 同事务保存。CAS loser 回原第3步；任何 reservation 不得提前产生 author effect。
>
> **步骤8附加：** final author commit 再验证 current authorization 和 ApprovalUse 的最终提交资格，并同事务 consume approval count/settle适用受管预算。失败/崩溃按 D6 原 planned recovery，不能另建 D10 author decision。

### 3.3 §3 回执与错误不改 wire

建议新增：

> Standing Approval 不增加 `d6_commit_receipt` 或 `d6_error` member/code。Author protocol caller 看到的 commit/replay 仍是原 D6 bytes。D10 UI/Broker 可以从受权的 D10 Run/ApprovalUse 状态同时呈现“本次由 standing approval 消费”，但这属于控制证据，不回写 receipt。
>
> approval_required、approval_expired、delegation_expired、control_conflict 等 D10 控制错误只由 D10 control adapter 返回；一旦正式提交进入 `d6_commit_request` 处理，原 D6 error/disposition 优先且不包装。撤权导致的 current author permission failure继续是原 not_visible，而不是 approval error。

### 3.4 新增受管 D10 控制意图的一般边界

建议在 D6 Control Interfaces 受管配置章节加入以下 owner-neutral 原则，而不在本修订冻结所有 D10 public wire：

> D10 的 ActivationBinding、DelegationLease、StandingApprovalEnvelope、AutomationDefinition 等 durable control state 只能由 Core-managed closed control adapter 修改，并具有独立 control revision/CAS、current principal authorization、完整 preview/audit 与 replay contract。它们不能通过 `d6_commit_request` 的自由 payload、普通 author source、provider callback 或 Agent JSON 修改。
>
> 本 D10 candidate 只冻结这些对象的语义和与 author submit 的组合；具体 public IPC/HTTP envelope 留给实现接口，但不得减少 closed decode、version/CAS、non-disclosure、audit 和 current-authorization 义务。任何未来决定把某个 D10 control mutation 纳入 D6 通用管理 request，都必须使用已有唯一 ledger/authoritative control transaction，不得建立无 ledger 管理旁路。

## 4. D7 Execution and Action Interfaces — 拟替换/新增条款

### 4.1 替换 ActionSpec 总述中的确认句

固定上游 D7 Execution/Action §5 中当前语义为“user 看到 preview 后通过原 D3/D6 提交入口确认”。建议完整替换为：

> ActionSpec 的默认提交确认仍是：当前用户取得并验证完整 preview 后，明确发送准备阶段返回的原 D3/D6 request。只有 D10/1 coordinated contract 的 `single_field_member` profile 可以在没有本次人机点击时继续：Core 必须从这次 fresh D7 prepare 的完整 PreparedActionBinding/2、EffectManifest/EffectBytes、实际 MutationFootprint 和 current authorization 机械证明一份有效 ApprovalUse。这个例外不创建 D7 confirmation token、不修改 request，也不表示用户逐字阅读了 preview。其它 ActionSpec kind 和 D3-owned intent 继续要求原交互确认，除非未来新的 coordinated contract 明确逐项开放。

### 4.2 在 §6 prepare/commit 前新增 standing-approval eligibility

建议新增以下完整条款：

> **D10 standing-approval eligibility.** D7 prepare 永远不知道或信任 caller 自报的 approval。它仍按原顺序完成 closed intent decode、潜在 observation、authority/cut、source/selector/definition/rule/dependency、proposed source/MutationFootprint、D2/D4/D5/D7 gates 与 immutable prepare/preview。只有上述全部成功后，Core-managed D10 adapter 才可尝试匹配。
>
> 当前唯一 profile 要求 ActionSpec.intent.kind 恰为 `set_field_member`；selector 指向当前完整 Field 中恰一 Entry；memberPath 每段是静态 object member name 且终点为 existing scalar member；owner/Field/member type 与 StandingApprovalEnvelope exact match；value 通过原 TypedLiteral/D4 decoder 与 envelope valueConstraint；Narrow Field Qualification 对同 RegistryBinding/current source 成功；MutationFootprint 恰只包含该 member value 改变。任何增加/删除 Entry、改变 occurrenceKey/qualifier/note/provenance、其它 member、Facet/body/title/coreKind、relation/ref、identity/lifecycle/placement/control、或需要 full-source preview 的效果都使 eligibility=false。
>
> eligibility=false 本身不改变 D7 Action 的合法性。若原 Action 可以交互提交，则 D10 caller 转 awaiting_confirmation；如果原 Action 本身不合法，返回原 D7 error。D7 不定义“为了自动化而更宽松”的 parser、Field selector、permission 或 semantic fallback。

### 4.3 在 replay/恢复段新增

建议新增：

> D10 standing approval 不改变 D7 OperationId/replay规则。fresh automation occurrence 必须 fresh prepare；同一已提交 request 的 lost receipt 只能重发原 request。ApprovalUse/count/budget 的 replay/恢复由 D6 coordinated amendment 管理，D7 不能因 Agent/automation restart 重新签发一个语义相同但 OperationId 不同的 Action。
>
> 原 D7 preview delivery epoch、EffectBytes、ActionEvidence、FieldSelection、result epoch 与 source revision 的失效规则保持；Standing Approval 不能复活 stale evidence/selector/preview。若必须重新 prepare，产生新 request/OperationId，并重新消费新的 approval use；旧点击/旧 mechanical approval 不可沿用。

## 5. D8 与 D9 的明确非修订说明

D8 不增加 unattended edit branch。Document/Annotation edit、dirty Draft、IME、current serial、完整 preview 和 explicit confirmation 原样保留。Agent/Automation 只能给 D8 提 proposal；不能伪造 EditSession 或 human origin。

D9 不修改 Provider/Route、worker sandbox、ExportPlan、LossReport、PublicationReceipt 或 external publisher。D10 外部 transport/Connector 不授 D9 worker 网络，D9 publication confirmation 不授 Standing Approval author write。

## 6. 激活与版本兼容

这些 amendment 只有在独立审查接受、总控协调裁决，并与 D10 candidate 一起激活后才生效。激活前：

- 产品可以实现/read-only D10 Broker、Agent proposal、Automation scheduling、Tool/MCP、Connector read 和 external effect control；
- 所有 D7/D8 author proposal 仍使用现有逐次确认；
- `single_field_member` unattended author submit capability 必须显示 not_in_release 或其它真实 D1 unavailable 状态，不得以 private flag 绕过。

激活后仍不修改旧 saved D3/D6 decisions 的 decoder/bytes。已提交历史无需回填 ApprovalUse。尚未形成 decision 的旧 D7 prepare 不自动获得 standing approval；用户必须 fresh prepare 或按当前协调版本明确支持的 exact recovery 路径处理。

## 7. 独立评审必须攻击的联合反例

1. 两个同值 Field Entries，自动批准不能挑 first。
2. Approval count=1 两个 Run 同时 planning，至多一方 planned/reserved。
3. Approval reserve 后 process crash，restart 不重复 reserve。
4. D6 planned 后 approval revoke/expiry，不自动 terminal_failed，不偷换新 envelope。
5. final commit 与 approval revoke 同时发生，存在唯一线性化结果。
6. author commit 成功、receipt 丢失、approval 已过期，重放原 receipt 不再扣次数。
7. raw no-op committed 正确扣一次，重放不再扣。
8. stale FieldSelection、A→B→A、Registry generation change 不能被 standing approval 复活。
9. footprint 偷改 note/provenance/Facet/第二 member 必须拒绝自动批准。
10. D8 dirty Draft 与合法 background author commit 组合，Draft 不被覆盖。
11. D3 create/lifecycle Action 不能借 single_field_member envelope 自动提交。
12. D10 control error 不能包装/泄漏原 D6 not_visible 与隐藏事实。

这些反例的作者候选定义不等于测试通过；真实证据仍按 Implementation Impact 分层。
