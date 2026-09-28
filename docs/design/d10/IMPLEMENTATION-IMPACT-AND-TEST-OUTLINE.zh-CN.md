---
source_language: zh-CN
translation_status: source
---

[English](IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md)

# D10 实现影响与测试轮廓

revision: D10-r08-joint-review-fixes-2026-09-28；状态：与最终作者候选同步的完整 R08 实现/测试义务。这些是设计与未来证据要求，不是已执行产品测试。固定 R07 的11项 finding 全部保持开放，等待固定 R08 候选的 fresh 完整独立联合评审。

## 1. 实现切片与状态所有者

目标实现保持“窄控制面 + 专用 executor”，不建立一个包含所有领域语义的通用 tool host。

```text
Desktop / CLI local                     Server / WebUI remote
        |                                      |
        v                                      v
  D10 Broker / Scheduler                D10 Broker / Scheduler
        |                                      |
        +---- Core-managed D10 control adapters+
        |              |
        |              +--> D6 Policy / ledger / author commit
        |              +--> D4 Registry validation/evolution
        |              +--> D7 Action prepare/effects
        |              +--> D8 edit proposal/confirmation
        |              +--> D9 conversion/export
        |
        +--> Agent runtime
        +--> Model Adapter
        +--> Tool Adapter / MCP Adapter
        +--> Connector
        +--> managed transport / secret injection
```

Broker 不读写 authority DB 表，不解释 D2/D4 source，也不接受自由 callback 写入。Core-managed control adapter 是所有 durable delegation、approval、ActivationBinding、occurrence claim、ApprovalUse 和 author-submit 资格的唯一入口。

建议实现顺序：

1. S1：实现 D10 严格 value/control decoder、受管 token tag、ActivationBinding/Capability Catalog，以及 publisher/namespace 信任校验。
2. S2：DelegationLease、ContextBundle、egress、SecretRef、audit spool 与原子预算/费用 reservation。
3. S3：Run/Automation Definition、occurrence claim、serial scheduler、cancel/restart。
4. S4：Model/Tool/MCP adapters 与 runtime isolation；先只 read/compute，不开放 external mutation。
5. S5：ExternalEffectIntent、send fence、idempotency/reconciliation、credential rotation。
6. S6：协调实现 UPSTREAM-AMENDMENTS 中 D6/D7 standing-approval 分支；在此之前无人值守 author commit 继续 unavailable。
7. S7：Connector 的具名 read-only profile；只有已有 SourceBinding/OriginBinding closed adapter 的写回才可逐 profile 开放。
8. S8：完成 Desktop/CLI/Server/WebUI 端侧接线、诊断、audit/export/retention；Mobile 只验证明确不可用的 capability。
9. S9：清理旧原型/别名/自由 JSON/tool callback、更新公开规范；只有真实实施/平台证据完成后才能公开声称支持。

### 1.1 R08 控制合同实施切片

[CONTROL-CONTRACT](CONTROL-CONTRACT.zh-CN.md) 是本纲要消费的 R08 management/current/history/author-adapter/external-effect/stop wire 唯一规范 owner；实现不得另选授权、重放、记账、frozen-request 或 stop 语义。

- Workspace 自助控制由 proposed D6 Policy/2 `d10_control_self` 授权；Workspace 管理由原 `policy_admin`，Registry activation 另验 `registry_admin`。deployment trust/account/secret/pricing/grant 由 D10 DeploymentControlPolicy 管理，不得借 Workspace admin 或 issuer admin。
- 会影响作者提交的 Workspace 控制变更继续使用原 D6 authority store、`PreparedIntent`、决议/receipt 与作者提交点。`DeploymentControlDecision` 只保存部署控制结果；host adapter 不得写作者 source。
- `ControlPrepareBinding/1` 使用 `(scope incarnation, principal, requestId)` 稳定键和完整规范意图字节。恢复顺序固定为：先验证当前可见性/权限，再比较同键完整输入，再重放已保存决议；只有尚无决议时才检查当前 expected revision 和执行资格。
- configuration revision 与 usageRevision 分离；control ID/incarnation 永不复用。retire/archive 不能删掉 planned、unknown、uncertain、evidence 或 dedup 仍引用的记录。
- `ResourceUseGrant/1` 的 cost/secret/egress/external-effect 四类分别实现；grant renew/revision 不清零 spent/held/attempt/use，换 grant 也不清除旧 reservation 和实际 account liability。
- fixed S profile/2 的旧非 Field 集合必须由测试常量逐字固定；profile/3 才增加 `d10_control_self`。升级、replay、replacement、continue/failover 不能改既有 family profile。
- PackageManifest dependency 在具体 dependent Contribution 内解析。Contribution availability 单独计算，connector unavailable 不得级联停用同包 schema/template/pack。

这些是合同实现义务，不表示本作者阶段已经运行真实 Core/OS/provider 实现测试。

## 2. 数据与存储影响

### 2.1 Authority Store

需要新增受管 D10 control records，但不能增加另一 author commit root。逻辑上至少包括：

- activation、trust、catalog records；
- Delegation Lease、LeaseRunUse 与 Standing Approval；
- Automation Definition、occurrence claim、Run/step 与 terminal dedup proof；
- ApprovalUse、approval-count reservation 与 PlannedDecisionApproval；
- budget/cost account 与 reservation；
- immutable ExternalEffectIntent、ExternalExecutionBinding send-attempt association 与 reconciliation evidence；
- 把一个 Run step 绑定到原 D7 PreparedActionBinding 与 D6 request 的受保护 D10AuthorPreparationLink；
- 完整 ControlPrepareBinding canonical intent B 与另行受权的 public history summary；历史 public delivery 不再保存另一份 A/B/M；
- exact-target ExecutionStopLatch、StopOwner/result association 与预留 safety-sequence capacity；
- Audit Started、terminal link 与本地 protected spool metadata；
- SecretRef/account generation metadata，不含 secret bytes。

这些记录应与 Workspace/authority identity、fence、current principal 和 version/CAS 一起管理。SQL table layout、index 和 GC 由实现决定，但不得改变候选的 version、atomicity、replay、masking、retention 语义。

D6 author ledger 仍是 Workspace+OperationId 唯一 author decision namespace。D10 Run、Approval、LeaseRunUse 和 ExternalEffect 记录不得在恢复时生成第二个“作者 committed”事实。

control history 持久化保留两种不同表示。内部 `ControlPrepareBinding.canonicalIntentBytes` 保存完整 canonical B，stable-key equality 比较完整 bytes。public `ControlPreparedHistory/1` 只含 `intentDigest`、`automation_configure|consent|state|workspace_limits|activation|deployment_put|cost_reconcile` 七值之一、allocated refs 与既有 affected/resource-use summary。

历史 result 永不返回嵌套作者请求 A、完整 B、生成 submit request M 或可复用 prepare token。首次 prepare 只有在当前披露覆盖完整 B 与嵌套 A 后才可返回 M。r5 historical result 与 r6 current exact-ref view 分别授权、分别投影；applied linkage 不可证明时固定 `state_unavailable`，绝不 prepared。

ApprovalUse count history 必须耐久记录 unreserved、reserved、consumed、released_terminal 的单向转移；只有 D6 authoritative terminal_failed 的同一 abort transaction 可以产生 released_terminal。Run cancel、TTL、临时撤权和 Lease 过期均不能靠 GC 释放该 reservation。

Occurrence claim 和 terminal dedup proof 可以压缩，但压缩后仍必须证明同一 AutomationOccurrenceKey 已经关联原 Run/terminal outcome；当 definition revision 仍可能重扫或恢复时，不能因为删掉明细行而重新执行同一个 occurrence。

PlannedDecisionApproval 是对 exact 原 planned request 的一次性交互授权，必须绑定原保存 preview semantic record；transport token 本身只是短命交付句柄，不能成为批准权威。
### 2.2 Secret Store

Desktop/CLI 使用符合平台能力的 OS secret store；Server 使用部署提供的受保护 secret store/KMS 等价物。实现必须证明：

- secret bytes 不写 author DB、普通日志、crash dump fixture、transcript、export；
- SecretRef 不能在另一个 account/contribution/audience 下重放；
- rotation generation、disable/revoke、restart、backup/restore 行为可验证；
- Server 多前端不会让浏览器取得 raw secret。

若所声称平台无法提供合格 secret store，该依赖 secret 的 contribution 为 unavailable；不能回退到明文配置文件或环境变量继承。

### 2.3 Immutable assets

package、manifest、definition、runtime、model、font/other dependency 按 exact digest/version 存于不可变资产区。激活前完整校验，激活后不能就地覆盖相同 identity 的 bytes。GC 只能删除不再被 current/historical binding、planned decision、unknown effect 或 audit/recovery pin 引用的资产。

`activation.current` 是派生 current-read 值，不是回写历史 ActivationBinding 的 revision。测试在同一个受权 cut 读取 active selector 与 candidate binding，CAS 准确 current predecessor selector，并原子发布 successor selector/binding。

旧 generation/revision 变成 non-current 后仍保持 byte-stable。A→B→A selector race 不能让 stale predecessor CAS 成功。

## 3. 激活、Registry 与 package 实施义务

Package verifier 实现 SHA-256 目录绑定、Ed25519 package signature、PublisherIdentity、NamespaceClaim、dependency closure 与平台约束。具体 key storage、revocation list distribution 和管理员 UX 可由实现选择，但必须满足以下规范行为：

- 自签不产生 namespace ownership；
- reserved namespace 永不被 third-party claim；
- key rotation 保持 publisher continuity；
- revoke 后新的 trusted context/execution 失效；
- 历史 D4 semantic ledger、saved decision、author source 不删除；
- ActivationBinding 切换要么旧、要么完整新，不能半态；
- semantic rollback 只能作为 successor activation，不能倒退 Registry history。

D4 candidate Registry 仍必须通过原 `RegistrySnapshot/1`、`RegistryBinding/1`、evolution proof 与 catalog loader。不能为了实现方便把 D10 package metadata 塞进 D4 closed snapshot。

### 3.1 Pack parent dependency 与 lifecycle 测试

实现必须把 package 安装、Contribution 激活、D4 definition retention 与 UI visibility 分开建模。domain Pack Contribution 在 Catalog 中保存 primary `parentDomainId`、`extensionPointId`、`requiredVersionRange` 和 activation 时解析的准确 parent binding；parent binding 进入 Catalog digest。父版本变化后旧 dependent binding 失效，只有 successor ActivationBinding 可以重新激活。

至少验证下列组合，并保持 D1 固定 reason 优先级：

| 条件 | Contribution | 预期 capability projection | schema/作者事实 |
| --- | --- | --- | --- |
| parent missing | inactive | 高优先级 reason 不成立时 `missing_component` | history/raw source 保留；typed interpretation 由 D4 complete/unavailable 决定 |
| parent present but explicitly disabled | inactive | `not_configured` | 同上；不能删除 definitions |
| parent incompatible | inactive | `incompatible_version` | old history retained；不得继续跑旧规则 |
| unsupported surface | inactive on that surface | `unsupported_surface` | portable facts 按该 surface 的 Core/D4 能力处理 |
| UI hidden only | activation unchanged | 无新增 unavailable reason | 完全不改变 RegistryBinding/作者 facts |
| parent ready/compatible | may activate | 继续经过 D1/D6/D10 其它门 | current RegistryBinding 解释 |

测试必须同时覆盖两类 Pack：Calendar rule/data Pack 在 parent inactive 时不得产生派生 rule/View/cache 或 connector 行为；Organizations schema Pack 在其已接受 definitions 仍由 current D4 binding 完整证明时可继续解释已有作者 facts，但不得因 retained schema 偷开 domain-specific View/Action/Assign/connector。若 definitions state=unavailable，raw source 保留，typed operation fail closed。

GC/uninstall 测试必须证明 package config/source、accepted semantic ledger、tombstone/migration 和恢复所需 immutable assets 不因 UI/module disable 被误删。测试还必须验证 parent version/binding 变化会使旧 dependent Catalog binding 失效，而不是在运行中跟随 latest。

补充 S 的 D4 reference catalog 现在属于实施测试输入：61 个 Field、7 个 Facet、27 个 relation Field，以及 Calendar 的跨 Field `union_variant_equal` 等真实约束必须参与 D7 Narrow Field Qualification。测试必须至少包含 `people/phone` 正向构造、一个 relation Field 负例、`calendar/range` 或 `calendar/recurrence` 的跨字段约束负例，以及未知 constraint constructor 负例；不能硬编码一个“安全 FieldId 列表”绕过 Registry 图证明。

## 4. Agent、Tool 与 MCP 实施义务

Agent runtime 只持 Run-local state、ContextBundle refs 和已允许 tool catalog，不缓存 ambient workspace authority。Context builder 必须从 Core 当前授权输入产生 exact source/result pins，并在 egress 前再次匹配 recipient。

ToolValue decoder/encoder 必须逐类型证明 exact 语义；严禁：

- JS double 代替 arbitrary integer/decimal；
- JSON null 同时表示 Optional.none、missing member 和 invalid；
- additionalProperties=true 的自由 map；
- 未界定 recursive schema；
- 任意 path、URL fetch、shell command、secret/token 作为通用值；
- 远端 MCP annotation 自动决定 effect class。

MCP adapter 测试需要恶意 server：descriptor 前后漂移、tool name 冲突、schema 变更、extra field、巨大递归 schema、伪 readOnly、prompt injection、资源内容带 tool 指令、断流/重复响应、超预算结果。发现变化只能 pending/reject/reset，不得在一个 Run 中热换 schema。


### 4.1 R08 Core author-adapter、closed-rule 与 SearchContribution conformance

`CoreFieldMemberAdapterDescriptor/1` 与 `FieldMemberTask/1` 测试必须穿过真实第一方 `weftext.automation/set-field-member` contribution，不能使用 generic ToolValue callback。task 拥有具体 D3 NodeRef、D4 FieldId、1..7 existing memberPath 与原 D7 TypedLiteral；Standing Approval 是独立 allow-rule，绝不提供这些 task 坐标。把 ToolValue text/title/path/model output 当 NodeRef、FieldId、occurrenceKey 或 control token 必须拒绝。

每次自动尝试都运行真实链：current Narrow Field Qualification → 完整 current Field read → exactly-one Entry selection → 重建原 D7 FieldSelector `{owner,fieldId,expectedRevision,occurrenceKey,rawEntrySource}` → 原 `set_field_member` Action → 真实 D7 prepare/完整 preview/effects/MutationFootprint → 机械比较 Standing Approval → ApprovalUse → 原 D6 request/plan/final/replay。`D10AuthorPreparationLink/1` 必须在提交前与原 PreparedActionBinding 原子耐久。冷启动只能恢复该原 request；只有证明从未保存时才能重新 prepare；continuity unknown 为 `state_unavailable`；planned/submitted-unknown 绝不新建 replacement OperationId。

Standing Approval rule 测试继续覆盖 0/64 enum、完整 TypedLiteral 排序唯一、同型 numeric bounds、0/65528 UTF-8 text 与 CR/LF 拒绝、1..7 static D4 member path 加 alias depth、semantic-code scope+code equality，以及 required/present-optional D7 bridge。保留原 D2/D4 预算：raw Entry 65,528 bytes、header 65,536 bytes、最多 256 header lines、32 carriers、8,192 entries、1,048,576 checked bytes，以及原 alias-depth/schema/source limits。完整 source 验证只是内部证明，不因此向窄主体披露完整 source。

规范正例是只有一个 Entry 且 optional `label` 已 present 的 `people/phone`。task 输入使用原 D7 Optional TypedLiteral，`state:"some"`，完整 resolved contribution-set scope 为 `people/other|people/personal|people/work`；approval enum 可以有意更窄。present `personal→work` 是真实 member change；present `work→work` 只有完整 proposed source byte-equal 且 MutationFootprint/field_change/sourceVersions 全空时才自动通过。`optional.none` 不自动批准。第二条同值 phone 存在时自动 exactly-one 不适用，但普通 interactive D7 路径仍能用真实 occurrenceKey/rawEntrySource selector 选择第二条。

Control-result/current 测试实例化全部 19 个 `ControlRecordKind` current projection，断言 exact scope/lifecycle，区分 configuration revision 与独立 usageRevision，并精确枚举七种 public control-history operation kind。内部完整 B bytes 继续是 stable-key equality 来源；public history 永不返回 A/B/M。首次 prepare 只有在当前披露覆盖 B 与嵌套 A 时才可交付 M。r5 成功/丢响应→r6 更新后，`d10_control_result` 只返回原 r5 summary/receipt/deltas，`d10_control_read` 另返回 current r6；两个入口都先执行各自 current authorization。applied continuity 不明固定 `state_unavailable`，绝不 prepared。secret current read 永不含 plaintext。

SearchContribution 集成直接消费真实 D7 Query Algebra descriptor，并使用真实 NFC text 正例，例如 accepted 第一方 `people/search-name`，终点是既有 `people/name.text`。测试覆盖合法 `people→(first_party,weftext.people)` NamespaceClaim tuple + 独立 proof、descriptor digest、current Registry/textPath、同 cut 完整 Catalog set，以及第一方 install→activation→原 D7 Query consumption。负例包括 runtime-only discovery、wrong owner、digest mismatch、重复 contributionId、D10 contractVersion 不变而 D7 version 改变、漏掉 active descriptor、selected Field/provider unavailable、script/network/extra member、缺 D6 `field_read`、successor Catalog/selector invalidation。Pack 测试另保留 parentDomainId、extensionPointId、compatible parent version/binding 三轴和合法第一方 activation 正例。不得用永久 deny-only 路径冒充这些正向能力。

## 5. Runtime 与 OS sandbox

每个可执行 contribution 的支持声明必须逐平台绑定真实 sandbox 证据。最低检查：

- no workspace/authority-store mount；
- no user home/browser profile/SSH agent/system clipboard；
- no inherited arbitrary environment secret；
- child process tree 受控且 cancel/crash 后可确认终止；
- network 默认关闭；只允许 managed transport；
- InputSlot 只读、output/temp 私有；
- CPU、memory、disk、process-count、wall-time 预算；
- symlink/hardlink/device/reparse/path alias 拒绝；
- stdout/stderr 有界并脱敏；
- runtime crash 不改变 author state。

Windows、macOS、Linux 分开验收；存在 container/sandbox 名称不算通过。Server 所宣称的 architecture/platform 也必须逐行有相同等级证据。

## 6. Automation 与 scheduler 实施义务

Scheduler 必须持久化 definition revision、有限调度 horizon、sourceOccurrenceKey、claim owner、Run identity、LeaseRunUse 关联，以及真实的 skipped/started/terminal outcome。首版采用 serial 语义：

- 同一个 `AutomationOccurrenceKey/1` 至多有一个 Run identity 和一个 durable claim；
- terminal occurrence 在重启、重扫、disable→enable 和 scheduler 缓存重建后仍返回原 Run/outcome，不创建第二个 Run；
- enable/disable 不改变 definitionRevision，也不清除 claim/terminal proof；
- definition 语义改变生成新 revision 和明确 activation point；
- run_once 只取当前有限窗口最新遗漏 occurrence；
- occurrence claim 可以在 Run admission 前存在，queued/blocked 且未进入受保护执行的 Run 不消费 Lease `maxRuns`；
- 第一次受保护步骤前必须执行 Core-managed Run-admission CAS，准确验证 current leaseId/leaseRevision、可信时间、ActivationBinding、budgets 与同 leaseId 谱系累计消费，并原子写 `LeaseRunUse/1`；
- admission 成功后 failed、cancelled、crash 均不返还 run use；同 Run recovery 不重复消费；
- 已有完整 `LeaseRunUse/1` 的同 Run 后续 protected step 和原 planned request 恢复不再比较 remaining count；即使 `maxRuns=1` 且累计已为 1，也只复用原准入事实；
- 复用准入事实不等于免检：每一步仍验证当前授权、准确 leaseRevision、可信时间、ActivationBinding、适用批准和预算；撤销/过期/revision 或 binding 改变仍阻止；
- `maxRuns` 耗尽返回 D10 `delegation_exhausted`，保留当前 claim/Run blocked，不能为同一 occurrence 创建新 Run 绕过；
- source/rule/authorization/ActivationBinding 变化在新 step 前重验；
- trusted time 超过 Lease `notAfter` 后，不论 cleanup task 是否运行，新 step 和最终提交都拒绝；
- clock epoch/时间连续性不可证明时返回 `state_unavailable` 并暂停，不能把未知时间当作未过期。

dedup 历史允许压缩为 coverage proof/terminal summary，但压缩必须保持“这个 key 已经处理”的可验证事实。GC 策略不能成为重复执行协议。

测试不能只 mock scheduler 返回一行。至少实际制造：

1. 双进程或双 Server frontend 同时 claim 一个 K；
2. K terminal 后 restart、schedule rescan、disable→enable、cache rebuild；
3. claim 已有但第一次 protected step 前取消，验证不消费 `maxRuns`；
4. admission 后 model call 收费并失败，验证 run use 不退款，`maxRuns=1` 时下一 K 返回 `delegation_exhausted`；
5. admission CAS 后立即 crash，再恢复同 Run，验证不重复消费；
6. Run 暂停期间 Lease 到期，cleanup 不运行但可信时间推进，验证新 context/model/tool/external step 与最终 author submit 都拒绝；
7. clock continuity 丢失，验证 fail closed 为 `state_unavailable`，恢复可信时间后再按真实当前时间判断 expired/active；
8. source/rule generation 与 definition revision 改变，旧 K 不被新 revision 接管重跑；
9. `maxRuns=1` 的同 Run 第二个 protected step 在 remaining=0 时仍复用原 LeaseRunUse，而新 Run 被 `delegation_exhausted` 拒绝；
10. 同 Run 原 planned request 在 remaining=0 时恢复，验证原 LeaseRunUse 连续性后继续；缺记录/连续性不可证时为 `state_unavailable`，不能重新消费。
## 7. Standing Approval 协调实现

UPSTREAM-AMENDMENTS 是本切片前置条件。修订尚未共同接受时，不得把该分支隐藏在 Broker 中先上线。

Core 必须拥有 StandingApprovalEnvelope validator、ApprovalUse builder、planned-preview recovery transport 的只读验证、PlannedDecisionApproval builder，以及 approval-count 的原子 reserve/consume/released_terminal 逻辑。Broker 只能请求“尝试机械批准”或“打开原 planned preview”，不能提交审批结论、MutationFootprint 或作者结果。

必须从真实 D7 `set_field_member`、FieldSelection/Narrow Field Qualification、PreparedActionBinding/preview 和 D6 request/plan 运行完整路径。简化 JSON fixture 只能证明其自身 decoder，不能建立组合语义。

错误 owner 测试必须覆盖 D10→D6 边界：

- 进入 D6 前 approval 不可用：D10 `approval_required|approval_expired`；
- R1 初检批准有效，R2 抢占最后次数，R1 在 D6 只因 approval 失败：新增 D6 `approval_unavailable/preflight`，unseen 无 ledger decision；
- 已 planned 后 approval 撤销：同 D6 error，ledger 保持 planned；
- D6 permission 失效仍为原 `not_visible`；
- dependency/semantic/budget 冲突仍为原 owner code，不能被 approval error 遮蔽。

planned-preview recovery 必须从真实 planned 保存的 PreparedActionBinding/2、preview semantic record 与 pins 打开新有限 epoch。测试要证明旧 preview token 已过期也能在 current audience/ObservationScope/permission/continuity 下读原语义，同时 current Query/definition/target 漂移不会改变恢复内容。recovery token 过期不修改原 planned，也不复活旧 token。

核心竞争与恢复测试至少包括：

1. exactly-one Entry 在 prepare 后变为两个；
2. source A→B→A；
3. approval 最后一次两个 Run 同时 reserve；
4. approval revoke/expire 与 D6 planning CAS 竞争；
5. planning 后 current D6 write permission revoke/regrant；
6. commit 成功但 receipt 丢失；
7. changed-member 与 raw-no-op 两分支：no-op 时 MutationFootprint、field_change、sourceVersions 均为空，但 target/type/value/权限/依赖仍验证；
8. process 在 approval reserved、D6 planned、author commit 三点崩溃；
9. stale preview/cursor/EffectBytes delivery epoch；
10. 新客户端无旧副本，通过 planned recovery 完整查阅原 preview 后建立 PlannedDecisionApproval；
11. PlannedDecisionApproval 遇确定 dependency conflict 不能复活 plan；
12. mutant 偷改 note、provenance、另一 member、Facet 或 body；
13. authoritative terminal_failed 与 approval `reserved→released_terminal` 同事务；replay 不双释放；
14. cancel、TTL、临时撤权、Lease 过期不产生 released_terminal。

只有真实 D6 commit transaction 同时保存 author result 与 approval consumed 后才算一次自动提交完成。authoritative terminal_failed 的 approval release 也必须与原 abort 同事务证明。

首代有意不提供 supplemental `planned_approval` 或 `external_approval` 的独立提前 revoke state action。测试必须证明这个支持边界，而不是发明 generic state：有限时间边界、准确 Lease/Activation/request binding、当前 ResourceUseGrant、当前 D6 authorization 与不可逆 stop 都在对应 planned/send cut 重新检查。expiry 或上层 grant/Lease/stop 变化可以阻断新使用，但不能把 supplemental approval 改写成合成 revoked state；planned work 只能按原 authoritative-abort 规则释放。
## 8. External effect 与 Connector 实施义务

每个可写 External Service 必须有具名 adapter profile，声明：

- exact request payload/value types；
- target/account binding；
- current read/write authorization classes；
- secret use；
- accepted success proof；
- failed_no_effect proof；
- idempotency key 生成、server scope、retention/window；
- safe reconciliation query；
- cost model；
- cancellation/timeout semantics。

缺任一项时可保留 read-only connector，mutation 对 unattended execution unavailable。不能用 HTTP method 名或 status 2xx 泛化成所有服务的成功/幂等合同。

发送栅栏需要故障注入：耐久 intent 之前、intent 已保存但尚未发送、系统写入或 HTTP 发送中、远端已接收但尚未响应、收到响应但 terminal audit 尚未耐久。任何无法证明的结果都保持 outcome_unknown。

Connector sync 若修改 SourceBinding/OriginBinding/watermark 必须另有 owner-stage closed adapter；普通 ExternalEffectIntent 或 single_field_member approval 不得直接写这些控制字段。

R08 external-effect 测试逐字消费 CONTROL-CONTRACT §7：
- `FrozenEffectBytes/1` 验证 byteLength/digest 与有限 payload/idempotency-proof bytes；proof 不是凭空新增 EvidenceTicket arm。
- 不可变的 `ExternalEffectIntent/1` 冻结外部效果引用、工作区、贡献项/账户、操作/目标、请求载荷与幂等绑定；生命周期修订属于另一个独立版本域。
- `ConsentSpec.external` 绑定 stable effect Ref + requestDigest，因此合法 `prepared→submitting` 不会自行使 consent 失效；任何 contribution/account/target/payload/idempotency 变化都要求新 effect intent 与 consent。
- `ExternalExecutionBinding/1` 在不可逆 send 前冻结 sendAttemptId、准确 Lease/approval/external-effect grant/egress grant、可选 secret generation 与 0..32 个可归属 cost reservation。sendAttemptId 永远不是 billableAttemptId；每份 reservation 都解析到自己的 billable attempt。
- public `ExternalEffectCurrentView/1` 只在授权通过后暴露状态、恢复方式、贡献项、账户、操作以及 requestDigest/targetDigest；永不返回请求载荷、目标 ToolValue、幂等键/证明、secret generation、approval record 或 reservation identities。
crash/reconcile mutant 尝试替换 frozen bytes、target、key、secret generation 或 effectId 必须被拒绝；`outcome_unknown` recovery 永远继续同一 immutable request。D9 conversion worker 保持 network=denied，不能借 D10 egress。D9 PublicationReceipt 只证明 publication；另存 Resource 仍需原作者协议。

## 9. Budget 与费用实现义务

费用实现逐字服从 CONTROL-CONTRACT §6、§9–§10。一份 `CostReservation/1` 永远只对应一个实际账户；Run/Lease/Automation/Workspace/deployment 多层额度在同一准入中检查，但不是多个账户的重复计费。真实可分别归属的多账户费用使用独立 reservation/evidence，禁止对同一费用重复计算。CostReservation 的状态机为 `reserved→settled(actual)|released|uncertain` 与 `uncertain→settled(actual)|released`；只有 settled/released 是终态，uncertain 保留完整 upper bound 并可由后续证据恢复。

实现必须保存 reservationId、attemptId、真实 cost account/grant、currency、pricing binding、upper bound、state/revision 和 evidence attribution。调用方没有 targetState 或手填 actual；final-bill / never-started 结论只能来自受信 EvidenceTicket adapter。

reconcile 在同一 authority transaction 中 CAS expected reservation revision，并原子保存 `CostSettlementDecision/1`、reservation state/revision、grant/account held/spent/available、evidence 与 audit link。same decision replay 不二次返额；两个不同 reconciler 竞争至多一个 winner。错误 attempt/account/currency、non-final 或不可唯一拆分 aggregate bill 保持 uncertain/full bound。

发送后的可靠零账单必须 `settled(0)`；只有 never-started proof 才 `released`。TTL、restart、author terminal_failed、Run terminal、管理员无证据填0或 effect idempotency 均不能释放。actual 超 upper bound 走已有 overcharge anomaly/freeze，不通过普通 settlement 提高 ceiling。

ResourceUseGrant 的配置和累计 usage 分开版本化。renew/revision 保留 spent/held/attempts；降低额度不得低于既有消耗+占用。变更 grantee/account/resource kind/currency 创建新 grantId，但旧 reservation 继续结算到旧 grant 与实际账户。

真实费用证据至少覆盖：reserve100→send→crash→uncertain→final bill20→settled(20) 只返80；sent+final0→settled(0)；never sent→released；并发 reconciler；commit 后 response loss replay；wrong attempt/account/currency；non-final/不可拆 aggregate；管理员无证据0；overcharge。未运行的 provider 测试保持 pending。


current-control 与 historical-result 持久化测试还必须证明 `Binding<lease>.revision==leaseRevision`、`Binding<approval>.revision==approvalRevision`、`Binding<grant>.revision==grantRevision`，reservation revision 与 deployment-policy revision 各自只有一套 CAS 权威；独立 usageRevision 变化不修改这些 configuration binding。`ControlDependencies.usageBindings` 只允许 lease/approval/grant/cost_account。

Trust 测试包含保留第一方 `people→(first_party,weftext.people)` 正例，并拒绝把随机 43 字符 D6 Token 当 `ownerId`。Package/search owner proof 使用 exact D4 tuple + 独立 EvidenceTicket；install order 与同名 package 不能替代。

## 10. Audit、retention 与 export

本地/Server protected audit spool 必须先于受保护操作耐久记录 started；remote collector 可以异步。实现必须有：

- append/sequence/integrity 或等价防篡改机制；
- 与 Workspace/principal/Run/step/effect/author request 的受限关联；
- secret redaction 与结构化 allowlist，不用日志后处理猜 secret；
- cancellation/revocation/emergency-stop 的 reserve；
- storage full、permission failure、corruption、collector offline 的不同状态；
- planned/unknown/uncertain 证据的 retention pin；
- 当前 `audit` 权限下的导出，不泄露普通用户不可见 source/secret。

候选不冻结具体 retention 天数；部署必须给有限、可见 policy，并且不能早于恢复/计费/安全 evidence 的最低存活条件删除。

## 11. Capability、错误 owner 与 conformance

D1 capability reason 和固定优先级必须从真实部署组合执行测试，特别是 policy_denied 与组件缺失/离线的重叠、offline 与 incompatible version、temporarily unavailable 与 offline 互斥，以及 Mobile unsupported_surface 高于后续安装状态。

错误所有者必须机械验证，不能由一个通用 AgentError 吞并：

| 位置 | owner | 必须结果 |
| --- | --- | --- |
| D10 pre-submit，没有可用 Standing Approval | D10 | `approval_required` 或 `approval_expired` |
| Run admission 的 `maxRuns` 已耗尽 | D10 | `delegation_exhausted`，不进入 D6 |
| Lease 超时 | D10 | `delegation_expired`；可信时间不可证时 `state_unavailable` |
| 已进入 D6，author permission/ObservationScope 失败 | D6 | 原 `not_visible/preflight` |
| 已进入 D6，仅 approval dependency 竞争失效 | D6 coordinated extension | `approval_unavailable/preflight`；unseen 无 ledger，planned 保持 planned |
| D6 dependency/semantic/budget 失败 | D6 | 原 code/disposition |
| planned-preview recovery 读取失败 | D7 transport | 原 `d7_effects_error` code，不产生 author decision |
| committed effects 读取 | D7 transport | 原 `d7_effects_resolve/open` |

D10 control error 还必须验证 hidden object exists/missing 两个世界在 caller 尚无可见资格时都返回 `not_visible`。若 caller 已有本人 control record 可见性，才允许显示 expired、exhausted 或 conflict。

R05 不再定义匿名旧 D10 capability profile。第一份共同公开 unattended author-submit D6 closed enum 直接包含 `approval_unavailable` 与 proposed `execution_stopped`；能否运行该 capability 由 D1 正式 catalog 与真实 availability gates 决定。D10 adapter 不得把 D6 code 重新包装为 approval_required，也不得删除既有 Policy/bootstrap/saved-decision 兼容合同。

D3/D7/D8/D9 原 error 同样必须逐字沿原 owner 传递。诊断 UI 可以在另一个受权 control read 中解释状态，但不能通过改变正式 error wire 泄露隐藏数据。
### 11.1 R08 capability、stop 与公开合同门

首版 D10 候选 capability IDs 是 `automation.manage`、`workspace.extensions.manage`、`deployment.external.manage`、`automation.stop`、`automation.author_submit`。设计接受或 coordinated design activation 只冻结规范；实现只有在 ID 已进入选定 D1 contractMajor 正式 catalog，并且真实 release/surface/policy/principal/component/configuration/version/reachability/health 门通过时才能报告 available。

第一份共同公开 unattended author-submit D6 closed error set 直接包含 `approval_unavailable/preflight` 和 proposed `execution_stopped/preflight`。测试不得构造匿名“旧 capability profile”；同时必须保留 Policy/1/2、bootstrap profile/1/2、D1 bootstrap/contractMajor 和历史 saved-decision replay。

emergency stop conformance 消费 CONTROL §11 的专用 D10 同库 safety transaction，不能建模成普通 control prepare 或已冻结 D6 public API。exact target 固定 `requestId==target.id`；W/H 是同一 latch 的两个独立 current-authority 读取面。首次 enable/admission 前已经预留 target/latch/result slot 与一个 safety-sequence unit。MAX=2^63-1；普通 configuration Counter/budget/quota 耗尽不能消费 reserved safety capacity，也不能阻止已预留 target 的首次 stop；容量耗尽只能拒绝新 executable target 建立。

测试覆盖首次 open→stopped、immutable revision-2 receipt、只读 `d10_emergency_stop_result` 返回原 receipt 或读取时点已证明 open 的结果，以及顺序 `closed decode/D1 → current disclosure → authority/fence/custody → stable-key/target equality → continuity → transition/read → final disclosure`。hidden/wrong-scope/wrong-store 为 `not_visible`；authority 不可证明为 `authority_unavailable`；确定破坏为 `integrity_conflict`；已可见 same-key/different-target 为 `control_conflict`；latch/result continuity 不可证明为 `state_unavailable`，绝不 not-applied。receipt 丢失后 target r5→r6 仍重放原 receipt。

仍必须证明与 Run admission 共用 store serialization domain、D6 final commit 在真实 final author transaction 内重验 latch、external send fence 持有到首次不可逆 handoff。stop 后当前获权 authoritative abort、cost settlement、evidence/audit retention 与 reference-safe cleanup 继续可用。任何 stop 测试都不得伪造 prepare history、D6 author receipt 或第二成功账本。

## 12. 跨表面实施矩阵

| 能力 | Desktop local | CLI local | Server | WebUI | Mobile |
| --- | --- | --- | --- | --- | --- |
| D10 Broker/control | 本地 | 同一本地能力族 | 托管 | 仅 Server 客户端 | unavailable |
| Agent runtime | 能力可用时 | 同一本地能力族 | 能力可用时 | 仅发起/管理 Server | unavailable |
| Automation scheduler | 一个本地 scheduler | 管理同一 scheduler | 一个 authority-domain scheduler | 仅 Server 客户端 | unavailable |
| Connector/model/tool executor | 本地 sandbox/transport | 同一本地能力族 | Server sandbox/transport | 不直接运行 | unavailable |
| Secret management | OS secret store | 同本地 store | Server secret store | 只经 Server 管理 | unavailable |
| Standing-approval author submit | amendment 激活后 | 同 Core | amendment 激活后 | 仅经 Server | unavailable |
| 普通 committed facts | 原 Core | 原 Core | 原 Core | Server Core | 原 Mobile Core/Server |

表中“能力可用时”仍必须满足 D1 release/platform evidence；设计存在不产生 supported 声明。

## 13. 受控命名映射与实现替换

TERMINOLOGY §13 的逐概念结构化映射是当前 D10 命名权威，不是未来测试计划。实现、CLI/API/schema/manifest、UI label 与 locale resource 只能消费其中已经给出的 mapping；“未公开 IPC”“无直接 CLI”“无历史 alias”是受控结论，不能被实现自行补成另一个名字。

后续实现应成套替换任何旧自由 tool callback、任意 JSON argument、UI 自报 approval、进程继承环境 secret、extension/name 驱动的自动 tool dispatch、Agent 直写文件、cursor/provider state 混入 author source，以及违反 Terminology collision 表的 prototype。若旧原型未发布，不保留 serde alias、fallback parser 或双读双写。

每个 D10 concept 必须能从 stable concept ID 追到中英正式名、owner、wire/API/manifest/schema 状态、代码 type/function/variable/namespace、CLI/UI/locale、简称/别名、正反例和首次冻结/迁移目标。继承 D1–D9 名称只引用原 owner；实现不得重新登记 `Registry`、`OriginBinding`、D6 permission、D7 Action、D8 Draft 或 D9 Provider 的 D10 alias。

B10-01 的实现负向门：Adopt 代码路径只允许 `adopt_*` convention；关联绑定值使用 D3 既有 `OriginBinding` / `origin_binding`。受控正向源码、API/schema、fixture 和 terminology registry 中不得出现 `adoption_binding`，scanner 不增加豁免。

TERMINOLOGY §14 与 CONTROL-CONTRACT 还冻结 R05 新控制记录、五个 capability ID 和四个第一方 module/package/schema 映射。候选代码 symbol/namespace 和 locale key 只能作为尚未实现的 mapping 验证，不能被 CI “存在字符串”冒充实际代码或资源实现。PackageId、D4 SemanticNamespaceId、D4 namespace ownerId、FacetId 与 module ContributionId 必须按各自 owner 分型，不允许因为字符串相同合并。

R08 消费两组**原 owner** naming/interface gate。D8 继续恰有九个 domain concept 与十三个 kind；每个 kind 必须从 UPSTREAM-AMENDMENTS §8.1 解析到唯一 technical-interface owner。Draft、Draft Projection、Draft Edit Map、Prepared Edit Binding、D2 snapshot、D6/D7 value 可以被 consumes/returns，但绝不与 kind 共 owner。尤其 `d8_draft_text_replace/write` 归对应 D8 interface，同时使用 Draft Edit Map 坐标。该 mapping 不改变 D8 wire、IME、显式确认、PreparedEditBinding/1 或 Undo 语义。

dirty D8 Draft 不是全局 Core 写锁：合法后台 D7 author commit 仍可发生。D8 必须排队/投影 author update、保留本地 input/composition，并要求后续 edit 按原 D8 合同 rebase/reprepare 或报告 stale。Undo 只能反演自己的原 committed edit，不能回滚后来发生的后台 commit。

D9 保持 36 个 D9-owned naming row + 1 个继承 D6 ImportJob。17 个 public kind 都必须从 UPSTREAM-AMENDMENTS §8.2 解析到唯一 technical-interface owner；consumes/returns/operates-on 从不表示共 owner。`d9_import_state` 是 D9 interface，但 operates on D6-owned ImportJob；其原 concept ID、ownedNames、`storage.import_job` 与历史 firstFreeze 全部不变。PublicationReceipt 只证明 publication；D7ResultPin 内 TerminalSchema/V 仍归 D7；继承 D2/D3/D6/D7 名称保持原 owner。D9 worker 保持 no-network 默认。

这些 owner 词表补充只改变术语 registry/检查，不改变 D8/D9 wire 或产品能力。实现若尚无对应 UI/CLI/locale，本代“无新增”就是通过条件；不能为了通过命名门凭空创建资源。
## 14. 测试与证据分层

| 层 | 能证明 | 不能证明 |
| --- | --- | --- |
| D10 静态/文档检查 | 双语结构、controlled names、scenario/reference 完整性 | runtime 正确、安全隔离、真实协议 |
| 有限模型 | 指定状态机、reservation、claim、race 的有限反例 | 完整 Core、OS、provider、任意规模 |
| real Core conformance | D3–D8 decoder/permission/plan/commit/replay 实际组合 | OS sandbox、外部 provider 行为 |
| durable fault model | SQLite/fence/crash/restart/atomic reservation | 真实云/网络服务保证 |
| OS sandbox evidence | 特定 build/platform 的文件/network/process isolation | 其它平台/版本 |
| protocol-service evidence | 具名 MCP/model/connector 的实际 idempotency/schema/billing | 其它 provider 或未来版本 |
| UI/device evidence | Desktop/WebUI/CLI 交互、a11y、状态显示 | Core 内部正确性本身 |
| release evidence | 当前 package/platform/profile 可声明 Supported | 未覆盖组合 |

本候选作者阶段没有运行新的 D10 有限状态机模型，因此不存在可报告的 D10 模型 pass 数。现有 CI 只在提交后按实际结果报告，不能把文档绿灯写成产品 conformance。

## 15. 必须实现的最小 hostile/race corpus

1. prompt injection 请求扩大 read、egress、secret、tool 或 budget；
2. MCP descriptor/schema drift、伪 readOnly、巨大输出；
3. `FieldMemberTask/1` 真实 `people/phone.label` Optional semantic-code 路径：唯一 present `personal→work`、逐字 `work→work`、`optional.none` 拒绝，以及第二个同值 Entry 导致自动 exactly-one 不适用但 interactive D7 selection 仍可用；
4. approval-count N=1 双并发，至多一个 reserved；
5. R1 进入 D6 前批准有效、R2 抢占次数，R1 只因 approval 竞争失败时得到 D6 `approval_unavailable/preflight`；
6. planned 后 approval 撤销，保持 planned 而非 semantic rejection/terminal；
7. old preview token 过期，新客户端通过 planned-preview recovery 完整读取原语义；
8. recovery 时 current Query/definition 漂移，交付仍是原保存 preview；
9. PlannedDecisionApproval 遇确定 dependency conflict，不能复活 plan；
10. authoritative terminal_failed 原子 `reserved→released_terminal`，replay 不双释放；
11. raw no-op 自动路径：空 MutationFootprint/field_change/sourceVersions，仍验证原 Action/目标/权限/依赖并在 commit 后消费一次批准；
12. cost N=1 双并发；
13. 发送前取消费用 `released`、实际发送零账单 `settled(0)`、费用未知 `uncertain`；
14. revocation 与 context delivery、author planning、external send 三种竞争；
15. crash at occurrence claim、LeaseRunUse admission、ApprovalUse reserved、D6 planned、author commit；
16. `maxRuns=1`：R1 admission 后模型调用收费并失败，run use 不退款，R2/new K 得到 `delegation_exhausted`；
17. K terminal 后 restart/rescan/disable-enable/cache rebuild 不另起 Run；
18. paused Run 的 Lease 自然到期，cleanup 不运行但可信时间推进后新 step/final submit 拒绝；
19. clock continuity 丢失，固定 `state_unavailable`，恢复可信时间后再判 expired/active；
20. external unknown + idempotency window active/expired；
21. credential rotation + unknown mutation；
22. local audit failure 与 collector offline 分域；
23. package activation crash、failed upgrade、successor rollback；
24. D4 three-generation semantic revival attack；
25. cancelled Run 已有 author committed/external succeeded 结果；
26. dirty D8 Draft + background legal author commit；
27. Mobile upload/Agent/approval negative capability；
28. hidden object exists/missing non-disclosure；
29. provider billing uncertain/overcharge；
30. package disable 时 author raw unknown namespace 保留；
31. D4 reference catalog 的 `people/phone` 正向窄证明与 relation/cross-Field/未知 constructor 负例；
32. D3 词表正向映射：`adopt_*` 使用 `OriginBinding` / `origin_binding`；反向受控源码/API/schema/fixture/术语 registry 均不存在 `adoption_binding`，且 scanner 不允许豁免。
33. self-service P 有 `d10_control_self`、真实窄 Field 权限和 deployment cost grant 时可创建有限 Automation；缺任一资格则拒绝，不能把 Field 权限当 deployment account manage；
34. stable-key r5 成功/丢响应、随后 r6 更新：受保护完整 B 决定 equality；public historical r5 只给七-kind summary/receipt/deltas 且无 A/B/M，current read 另返回 r6；same key different complete B 返回 `control_conflict`；
35. 授权续期、revision 更新或新 grant 与旧 `uncertain` reservation 并存时，证明已消费、已占用、attempt 次数和账户义务都不会清零；
36. exact-target stop（`requestId==target.id`）分别与运行准入、D6 最终作者提交和外部发送竞争；同时覆盖首次 receipt、目标配置 r5→r6 后的丢响应查询、隐藏目标/连续性错误顺序、预留 MAX 边界，并证明 stop 不阻断费用结算、权威 abort 或证据清理；
37. profile/2 family 升级不获得 `d10_control_self`；profile/3 只作用 explicit issuer update 后新 family；
38. per-Contribution dependency：connector unavailable 时同包 schema/template/pack 不受牵连；
39. Calendar、Library、People、Organizations 的 D10 PackageId→module→schema 映射与 D4 namespace owner/Facet 保持类型分离，并验证第三方同名包不能冒充第一方 owner；
40. design accepted 但 release/surface/policy/version/health 任一门不满足时仍返回真实 D1 unavailable reason；
41. D8 九 concept / 十三 kind 的唯一 technical-interface-owner 映射；consumed Draft Edit Map 绝不共 owner；dirty Draft + 合法后台 D7 commit 必须保留输入并触发 stale/rebase 行为，Undo 不能回滚后续后台 commit；无额外 CLI/locale/wire alias；
42. D9 17-kind 唯一 technical-interface-owner 映射 + 36 个 D9-owned naming row + 1 个继承 D6 ImportJob；`d9_import_state` operates on ImportJob 但不拥有它，`PublicationReceipt/1` 不能成为 author receipt，`D7ResultPin` 不能取得 TerminalSchema/V owner，继承 D2/D3/D6/D7 名称不得重注册；
43. U12 分支模式：当前 ICS production profile unsupported 且零作者效果；无 D3 binding 的 ordinary-file profile 两次独立显式 import 各 fresh、同 canonical request recovery 重放；D3-bound 路径覆盖 never_bound/active_live/active_non_live/retired/conflict/miss，retired 只有显式 Adopt 才 fresh，UID 不能单独决策。
44. T02 discovery 记录/复用 pending admission，不让未接纳 tool callable；current Catalog bytes 不变，已有 prepared invocation 的 restart/uncertainty 只恢复原 admitted descriptor/request，不生成 replacement request。
45. E05 eventual-consistency readback miss 对同一 immutable effect/target/request bytes/原 key 保持 `outcome_unknown`；后续 readback 可以解析结果，但不能创建新 effectId/target/payload/key，proof 过期也不授权 resend。
46. D7 prepare 后冷启动恢复准确 `D10AuthorPreparationLink/1` 与原 D6 request；link continuity unknown 为 `state_unavailable`，只有证明从未保存的 prepare 才可重建。
47. external consent 绑定 effect Ref + requestDigest，因此合法 prepared→submitting lifecycle revision 不会使自身失效；任何 frozen request semantic 变化都要求新 effect/consent；sendAttemptId 与每个 billableAttemptId 始终分域。
48. 首代 supplemental planned/external approval 没有独立提前 revoke state action；time/binding/grant/authorization/stop gate 可阻断使用，但不伪造 generic revoked state。
49. `activation.current` 由同 cut selector 派生；stale selector CAS 失败，successor switch 原子完成，历史 ActivationBinding generation/revision 不因变成 non-current 而改变。

每个 case 同时给正例和 mutant/negative，不能只比较字符串日志。任何未实际运行的 case 在 evidence 表中保持 pending。
## 16. 完成门

作者实现计划只有在以下条件都明确记录后才可交独立评审：

- CANDIDATE、CONTROL-CONTRACT、TERMINOLOGY、SCENARIO-DISPOSITIONS、UPSTREAM-AMENDMENTS 与本文互相一致；
- TERMINOLOGY §13–§15 对 D10 自有与上游引用概念完成 Intake §8.5.1 映射；D8/D9 原 owner 的完整补充位于 UPSTREAM-AMENDMENTS §8，D10 不重复取得 owner；没有 TODO/“留实现决定”占位；
- 阅读证据分账：原作者 lineage 保留历史 S49/49；本接续作者亲自完成 S16/49 全文，D9 workers/export 与 templates 仅做依赖局部读取且不计全文；固定 R07 的独立评审另行完成 S49/49。三套证据互不替代；
- D6/D7/D8/D9/D3 配套 amendment 都明确为未激活提案；
- 没有把 unsupported/deferred 写成 available；
- 自动 author commit 只限 single_field_member profile；
- External effect unknown、cost uncertain、audit failure、cancel/planned 恢复均有单一规范结论；
- D1 surface/reason、D3 identity、D4 Registry、D8 confirmation、D9 worker/publication 不被暗改；
- 任何实际运行证据精确分层，pending 项不被写成 pass。

- 固定 R07 的完整独立终审历史结论仍为 REVISE（P0=0/P1=1/P2=10）；完整 R08 作者修订也不能靠作者声明关闭这 11 项 finding；
- 本纲要列的是待实现/待测试义务，不是 Core adapter、stop transaction、external transport、D8/D9 mapping 或 race corpus 已执行的证据；
- 固定 R08 候选只有在九对双语/18 exact path 全部一致、固定 S 与非 D10 路径不变、125 个 scenario ID/分类一致且适用 documentation/input gate 通过时才算作者候选完整；之后仍需 fresh 完整独立复核。

这些是候选完整性门，不是独立 Gate verdict。
