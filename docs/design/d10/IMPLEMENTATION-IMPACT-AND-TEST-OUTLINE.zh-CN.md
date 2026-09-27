---
source_language: zh-CN
translation_status: source
---

[English](IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md)

# D10 实现影响与测试轮廓

revision: D10-r04-final-review-fixes-2026-09-28；状态：candidate。本文件描述未来实现义务和证据门，不表示当前仓库已经实现 Agent、自动化、Connector、MCP、standing approval 或 D10 runtime。本文不授权修改产品代码；当前 PR 只包含设计材料。

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

## 2. 数据与存储影响

### 2.1 Authority Store

需要新增受管 D10 control records，但不能增加另一 author commit root。逻辑上至少包括：

- activation、trust、catalog records；
- Delegation Lease、LeaseRunUse 与 Standing Approval；
- Automation Definition、occurrence claim、Run/step 与 terminal dedup proof；
- ApprovalUse、approval-count reservation 与 PlannedDecisionApproval；
- budget/cost account 与 reservation；
- ExternalEffectIntent、attempt、reconciliation evidence；
- Audit Started、terminal link 与本地 protected spool metadata；
- SecretRef/account generation metadata，不含 secret bytes。

这些记录应与 Workspace/authority identity、fence、current principal 和 version/CAS 一起管理。SQL table layout、index 和 GC 由实现决定，但不得改变候选的 version、atomicity、replay、masking、retention 语义。

D6 author ledger 仍是 Workspace+OperationId 唯一 author decision namespace。D10 Run、Approval、LeaseRunUse 和 ExternalEffect 记录不得在恢复时生成第二个“作者 committed”事实。

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

## 9. Budget 与费用实现义务

D10 cost engine 必须做多账户原子 reservation，而不是先读余额后分别扣减。至少验证 Run、DelegationLease、Automation、deployment 四层上限，并与 D6 原 work/attempt budget 独立累计。

Money 只能使用同 account currency 与 Counter microUnits。pricing rule 必须版本化并冻结：

- 固定收费；
- token/unit 线性收费时，最大输入、输出和工作上限能推导有限最大费用；
- 无法给出有限上限的动态价格不提供 hard ceiling。

reservation 状态必须耐久且终态含义唯一：

- `released`：能证明 billable execution 或 billable send 从未开始；
- `settled(actual)`：收费尝试已经开始且最终计费可证，actual 可以为 0；实际发送后可靠账单为零必须是 `settled(0)`；
- `uncertain`：收费尝试可能开始但最终费用不可证，继续占用原上限。

已经发送请求不能因为“最终收费为零”写成 released。author terminal_failed 也不能自动释放费用；只有证明对应收费尝试从未开始才可 released。overcharge anomaly 冻结 capability 并进入管理恢复，不能改写历史 reservation 使其“合法”。

真实 provider 测试至少包括：发送前取消→released；实际发送且账单0→settled(0)；正常成功计费；error/retry billing；delayed invoice；missing usage；usage disagreement；uncertain crash recovery；over-ceiling simulation；currency mismatch。每次 retry 独立 reservation，幂等效果不等于免费重试。
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

新增 D6 `approval_unavailable` 是 closed-enum 协调扩展；测试必须证明旧 capability profile 永远不会产生它，只有 consumer 明确支持修订 profile 时 capability 才 available。不能在 D10 adapter 内把这个 D6 code 重新包装成 approval_required。

D3/D7/D8/D9 原 error 同样必须逐字沿原 owner 传递。诊断 UI 可以在另一个受权 control read 中解释状态，但不能通过改变正式 error wire 泄露隐藏数据。
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
3. 两个同值 Entry 的 standing approval，禁止自动选 first；
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

每个 case 同时给正例和 mutant/negative，不能只比较字符串日志。任何未实际运行的 case 在 evidence 表中保持 pending。
## 16. 完成门

作者实现计划只有在以下条件都明确记录后才可交独立评审：

- CANDIDATE、TERMINOLOGY、SCENARIO-DISPOSITIONS、UPSTREAM-AMENDMENTS 与本文互相一致；
- TERMINOLOGY §13 对全部当前 D10 受控概念逐项完成 Intake §8.5.1 的 13 项 mapping，继承名称引用原 owner，没有 TODO/“留实现决定”占位；
- 48/48 upstream input coverage 保持；
- D6/D7 amendment 明确为未激活提案；
- 没有把 unsupported/deferred 写成 available；
- 自动 author commit 只限 single_field_member profile；
- External effect unknown、cost uncertain、audit failure、cancel/planned 恢复均有单一规范结论；
- D1 surface/reason、D3 identity、D4 Registry、D8 confirmation、D9 worker/publication 不被暗改；
- 任何实际运行证据精确分层，pending 项不被写成 pass。

这些是候选完整性门，不是独立 Gate verdict。
