---
source_language: zh-CN
translation_status: source
---

[English](CANDIDATE.md)

# D10 Agent、自动化与外部能力候选

revision: D10-r01-candidate-2026-09-25；状态：candidate，等待独立审查与协调激活。本候选以固定上游输入提交 `f205831c848729f7ddbc3ba0cf32b689459c0c98` 的 D1–D9 为权威前提，已完整读取 `docs/design/inputs.json` 所列 48/48 份输入。本文是作者候选，不表示 Gate 通过、产品已经实现、D6/D7 已修改或可以开始 A2。

## 1. 选择与问题边界

D10 选择 **Narrow Delegation Broker + Typed Contribution Catalog + Specialized Executors**：共享一个窄的安全控制面，Agent、automation scheduler、connector/tool adapter、model adapter 与 D9 conversion worker 保持各自封闭协议。Broker 只协调身份后的委托、贡献可用性、secret 引用、预算、审计、Run 生命周期与外部副作用恢复；它不解释作者领域模型，不保存第二份作者事实，也不获得 Core 外的工作区提交路径。

Core 继续是唯一作者事务权威。作者内容、D3 identity/lifecycle、D4 typed schema、D5 collection、D6 policy/transaction/recovery、D7 Query/View/Action、D8 Draft/confirmation 与 D9 conversion/publication 的所有既有权威均保持。D10 新增的控制记录只能决定某个外部能力是否可尝试、某个受限委托是否仍有效、某个外部请求是否已知完成；它不能用自身状态声明作者提交成功。

本代必须同时支持本地 Desktop/CLI 与托管 Server。WebUI 只能经 Server 发起或管理这些能力。Mobile 首期不运行 Agent、automation、connector、conversion，不管理这些能力的 credential，也不提供 Agent/automation 审批入口；它只能读取经过合法工作区提交产生的普通结果。

## 2. 非目标

本代不定义通用 workflow DAG、任意脚本系统、任意 JSON patch、第二查询语言、第二权限系统、第二作者 ledger、跨文件系统与工作区的分布式原子事务、实时协作协议、Mobile Agent、任意远程 shell、自动发现后即执行的 MCP 工具、长期可移植 transcript、通用双向同步语义或所有外部系统的写回协议。

D10 不把 D9 worker 升级成通用插件宿主，不给 D7 ActionSpec 增加自由 tool/action kind，不改变 D4 Registry 的 schema 或累计演进规则，不增加 D3 identity kind，也不把 package、Run、approval、tool call、provider ID、cursor、etag、UID、RECURRENCE-ID 或 transcript 变成 EntityRef。

## 3. 全局不变量

1. 任一作者修改最终仍只有原 D3 或 D6 author commit point；D10 没有第三提交入口。
2. read、content egress、workspace mutation、external side effect 与 secret use 是五个独立授权维度，任何一个都不蕴含另一个。
3. Web page、Document、tool result、MCP descriptor、model output、template text 与 provider response 都是不可信数据，不能扩大 principal、delegation、tool allowlist、egress recipient、network、file、process、secret、budget 或 approval。
4. D10 的控制身份不是内容身份。RunId、AutomationId、ApprovalId、ExternalEffectId 和 AuditEventId 都不能进入 EntityRef 或替代 D3 Ref。
5. 安装、namespace ownership、运行可用性和作者事实正交。disable、uninstall、failed upgrade、credential rotation 或 provider outage 不删除、默认化或重写作者事实。
6. 外部副作用与 Core transaction 不是原子整体。产品必须分别报告作者结果和外部结果。
7. capability discovery 不是授权票据；每个实际受保护步骤重新验证 D1 capability、当前 D6 权限、D10 委托及本步骤的附加授权。
8. 所有 resume/replay 使用原固定输入、原 OperationId 或原外部请求标识；unknown outcome 不能靠新 ID 重做。
9. 缓存、context bundle、preview、tool result 和已生成 staging 都受各自 current authorization/generation 门，不因“已经算过”继续交付。
10. 有限模型、CI、真实 Core、OS sandbox、真实协议服务和 UI/设备证据必须分层报告；任何一个子层通过都不表示产品支持已经完成。

## 4. 竞争性完整方案与选择

| 方案 | 结构 | 主要优点 | 主要失败风险 | 本候选裁决 |
| --- | --- | --- | --- | --- |
| A | 共享 broker/control plane + 专用 Agent/automation/connector | 安全与恢复规则可复用 | 公共层容易膨胀为万能工具系统 | revise |
| B | 单一 capability host + manifest 驱动所有执行 | 运行模型看似统一 | manifest 变成第二 Query/Action/schema 与大攻击面 | reject |
| C | Agent、automation、connector 各自完整控制系统 | 领域隔离直接 | secret、撤权、预算、审计、unknown recovery 重复且漂移 | reject |
| D | 窄 Broker + typed Catalog + 专用 executor；工作区安全核验仍在 Core | 共用安全控制而不共用领域执行语义 | 需要严格禁止 Broker 获得业务/作者权威 | select |

方案 D 的核心取舍是：允许少量明确重复的专用 executor 状态机，换取不建立开放式“万能工具 host”。只有身份、委托、安装信任、预算、审计、Run 与外部效果恢复进入共用层；领域输入和提交继续由既有 owner 解码和验证。

## 5. 权威与耐久状态

D10 控制状态位于与 D6 Authority Store 绑定的受管控制域，由 Core/Server Core 的封闭 adapter 修改。Broker、package 代码、Agent、connector 和 MCP server 均不取得数据库、工作区目录、author store、SQL 或直接文件写句柄。

| 状态 | 权威 | 规则 |
| --- | --- | --- |
| 作者 source、Ref、revision、D3/D6 decision | D3/D6 | 原事务、CAS、fence、receipt 不变 |
| D4 Registry semantic ledger | D4 消费、D10 认证来源 | 累计演进，不允许倒退删除历史 |
| D10 Capability Catalog 与 ActivationBinding | D10 受管控制域 | 只描述贡献/运行可用性，不解释作者值 |
| Delegation Lease、Standing Approval、Automation Definition | D10 受管控制域 | 减权、有限寿命、版本化 |
| Run、step、occurrence claim、ApprovalUse、cost reservation | D10 运行控制记录 | 不能宣称作者 commit |
| ExternalEffectIntent 与恢复证据 | D10 运行控制记录 | 与 Core receipt 分域 |
| credential 原值 | OS/Server secret store | D10 只保存 SecretRef 与 generation |
| package/runtime immutable assets | 受管资产区 | exact digest/version；暂存不等于激活 |
| transient context、model/tool output、transcript | 受限 pins/cache | 不能替代 author/control ledger |

所有影响工作区操作资格的 durable record 必须通过 Core 管理的 closed adapter 修改。普通运行 telemetry 可以独立运输，但不得被恢复器当作批准、授权、费用或效果事实。

## 6. Registry、Catalog 与激活

D4 Registry 仍是唯一 semantic namespace/schema 权威。D10 证明 package/publisher/namespace claim、安装资产与贡献来源，随后把完整候选 `RegistrySnapshot/1`、`RegistryBinding/1` 交给 D4 既有验证、演进和 catalog load；D10 不增加 Registry member，也不改变 D4 Field/Facet/Relation/Calendar/Unit 语义。

D10 另维护 Capability Catalog，记录每个 executable 或纯数据 contribution 的 exact package digest/version、contribution kind/version、runtime profile、platform/architecture、dependencies、host privileges、network/egress class、secret requirement、cost profile 与 D1 capability ID。Catalog 不能保存另一份 FieldDefinition 或用 display name/安装顺序证明 namespace owner。

```text
ActivationBinding/1 {
  workspaceRef,
  activationGeneration,
  registryBinding,
  capabilityCatalogDigest,
  trustRevision
}
```

一次激活固定顺序为：stage immutable assets → 验证 publisher/namespace/dependency → 构造 candidate Registry 与 Catalog → 完整 D4 evolution/load validation → 验证运行资产可达 → 在同一受管事务切换 ActivationBinding。失败前继续旧 binding，失败后不能出现半份 Catalog 或半份 Registry。

暂时 offline/health failure 不产生新 semantic generation。真正 package、definition、runtime contract 或 trust 变化才产生后继 binding。未激活升级失败可丢弃 staging；已激活后的“回滚”必须是后继激活，不能把 D4 semantic ledger 指针倒退或删除中间历史。

disable/uninstall 可使 executable contribution unavailable，也可使需要该 provider 才能证明的定义进入原 D4 unavailable 分支，但必须保留作者 raw source、累计 tombstone/migration 与必要的已接纳语义证据；不能把 unavailable 解释为空集合或删除事实。

## 7. Publisher、namespace 与 package 信任

本代 package authenticity 采用 SHA-256 完整内容绑定与应用层 Ed25519 签名。这不是平台发行签名、公证或应用商店身份，不改变 D1 的发行约束。

PublisherIdentity、NamespaceClaim 与 PackageManifest 三层分开：publisher root/key 证明“谁签了”，NamespaceClaim 证明其是否拥有某个 publisher namespace，PackageManifest 证明该 package 资产目录与贡献项来自已接纳 publisher。自签 package 不取得 namespace ownership；首次安装也不会自动创建 claim。

D4 已冻结的 core、first-party、workspace_user 与 reserved namespace 规则保持。publisher 只能 claim 未保留 namespace，且一个 active namespace 恰一 owner。安装顺序、enablement、localized label、package display name、下载站点或签名“有效”均不是 owner proof。

key rotation 必须由原 publisher 连续性证明和当前 trust policy 接纳；key loss 的恢复是明确管理员动作。trust revocation 阻止新的相关执行与可信 context 构造，但不删除历史签名、已保存 decision 或 D4 semantic history。

## 8. Principal、委托与授权交集

D10 不定义与 D6 Policy 并行的权限代数。Delegation Lease 只是在当前 authenticated principal/D6 Policy 上进一步收窄。

```text
DelegationLease/1 {
  leaseId,
  leaseRevision,
  workspaceRef,
  principal,
  automationOrRun,
  activationBinding,
  capabilityAllowlist,
  readScope,
  egressRecipients,
  externalEffectClasses,
  secretUsages,
  notBefore,
  notAfter,
  maxRuns,
  budgetAccounts
}
```

首版只允许用户/管理员向一个具名 Automation 或 Run 作一层 delegation；Agent、tool、connector 不得再次把权限转授另一主体。子步骤仍在原 Run 内，并受原 Lease 与预算限制。

有效资格为：D1 capability ∩ current D6 Policy/ObservationScope ∩ DelegationLease ∩ contribution deployment policy ∩ egress grant ∩ secret-use grant ∩ external-effect approval ∩ current ActivationBinding ∩ budgets。任何一项失败都不能由另一项补足。

Lease 的 readScope 不能虚构 D6 不存在的 Field ref-set 权限。Core 先按 D6 原授权取得合法读/写范围，D10 再用 exact owner/Field/context 约束收窄。过期、撤销或 generation 变化阻止新受保护步骤；已 committed author decision 和已经线性化发送的外部请求不被倒推回滚。

## 9. Context、出站与提示注入

Agent 没有 ambient workspace。每个 context request 明确选择 workspace、typed source、最大范围、目的、目标模型/工具 recipient 和预算。Core/Server 在当前授权下构造 immutable ContextBundle，绑定精确 source versions/result epoch/authorization generation 与实际选出的 bytes；上下文选择遵守最小必要和数据最小化。

Context readable 不等于允许 egress。发送到 Model A、Connector B 或 remote MCP C 分别是不同 recipient authorization；切换 provider、endpoint、account 或 remote origin 必须重新匹配。

优先级固定：host/product policy 与受管 delegation/approval 是控制输入；用户明确任务是业务输入；Document、web page、email、tool result、MCP description/prompt/resource、model output 都是不可信数据。任何来自不可信数据的“忽略系统规则”“调用更多工具”“发送 secret”“批准下一步”均只能作为普通文本，不改变控制状态。

Secret、credential、token、plan/effects handles、private environment 和未选择 workspace data 不得进入普通 ContextBundle。日志和 transcript 使用显式 redaction；redaction 失败的敏感值不允许发布到普通日志。

## 10. Tool Value Profile 与 MCP

D10 ToolValueProfile/1 复用 D7 TypeSpec/V 的受限子集：bool、exact text、int64、integer、decimal、Optional、closed object、bounded list 和 closed union。类型最大深度 16、object members 64、union arms 8、list maximum 4096、type description 64 KiB、单次完整参数或结果 8 MiB；实际 deployment/run budget 可以更窄。

首版 ToolValue 不含 EntityRef、Locator、ActionEvidence、plan/result/effects token、SecretRef、任意 file path capability、calendar/quantity 或开放 map。需要向外部服务发送一个作者 Ref 的可读字符串时，它只是经明确 egress 允许的 text，不自动重新获得 Core 能力。

文件使用独立 InputSlot，绑定本次 exact bytes、media type、用途、recipient 和预算；InputSlot 不是通用路径或可跨调用 file handle。

MCP 只是 Tool Adapter 的一个传输/适配协议。MCP tool name、description、JSON schema、readOnly annotation、prompt 和 resource 内容全部是不可信远端元数据，不能授权 workspace read、egress、secret 或 write。每个可调用 tool 必须已有本地接纳的 contribution binding 与确定性 schema adapter；运行时发现新 tool/schema 只产生待接纳描述，不热加入 Agent allowlist。

远端 JSON 与 ToolValue 的 adapter 必须证明 exact numeric、Optional/null、closed member 和 bounded collection 映射；不能证明则 unsupported，不使用 JS double、stringify fallback 或额外字段容忍。tool output 永不自动执行其中下一条 Action/URL/tool call；Agent 只能产生新的 closed proposal，再重新经过授权链。

## 11. Runtime 隔离

所有 executable contribution 默认没有 workspace mount、author DB、用户 home、browser profile、SSH agent、系统 clipboard、任意 environment secret、任意 file path 或直接网络。受信 host 只提供 exact InputSlots、只读已验证 runtime/dependencies、private output/temp、资源限额及受管 transport。

本地 process runtime 与 Server sandbox 必须分别有实际 OS 证据；只有 timeout/container 名称不证明隔离。无法证明所需平台的文件、network、process-tree、resource 与 cleanup 边界时，该 contribution 在对应平台 unavailable。

D9 conversion worker 保持其更窄“默认无网络”的合同。D10 connector/model/remote tool 的网络只能通过受管 transport，按 contribution、account、recipient 和目的显式授权；不能把“D10 有网络”反向授给 D9 worker。

## 12. Agent

AgentSession 是 Run 的交互模式，不是内容实体。典型步骤为：取得 ContextBundle → model invocation → 解析普通输出/typed tool proposal → 对每个 proposal 重新执行 capability/authorization → tool/model step 或生成 D7/D8 proposal → 用户或受限 standing approval 决定是否提交 → 读取原 receipt/result。

模型不能提交任意 source patch。工作区修改只能形成已有 D7 ActionSpec 或 D8 explicit edit proposal；不存在 closed adapter 的修改不能靠自然语言、tool JSON 或 MCP command 逃逸。

交互式 author mutation 默认需要完整当前 preview 后的明确确认。model/tool timeout、cancel 或解析失败没有作者效果。取消后迟到输出不得触发下一步；可作为受限 diagnostic 保存，但不能改变 Run state 中已经关闭的 step。

Run completed 只表示其所有已要求步骤达到各自 terminal 状态；它不等于任何特定 author/external effect 成功。UI/CLI 必须展示实际 per-step outcomes。

## 13. Automation 调度

首版 Automation 只调度一个已经接纳的 invocation，不提供通用 DAG、循环或自由脚本。definition 包含 capability invocation、schedule、DelegationLease、approval binding、budgets、queue limit 和 missed policy。

schedule 仅支持一次性 D4 ZonedInstant，或从明确的 Calendar recurrence/range source、有限 horizon 与 limit 产生 occurrence。Core 必须从实际 source 与 frozen D4 rule context 计算，不接受 executor 自报可信时间规则；无法得到确定 instant 的 date-only 输入不暗补午夜。

并发策略固定 serial。missed policy 只有 skip 和 run_once；run_once 只执行当前恢复窗口中最新一个遗漏 occurrence，其余记录 skipped。恢复窗口必须有有限 policy 上限，不能无限回放历史。

```text
AutomationOccurrenceKey/1 =
  (automationId, definitionRevision, sourceOccurrenceKey)
```

同一 key 最多拥有一个 active Run claim。enable/disable 只改变 control revision，不改变 definitionRevision，因此不能靠反复启用复制执行。definition 的 invocation、schedule、Lease、approval 或预算语义变化产生新 definitionRevision，并只接管明确 activation point 之后的 occurrence。

每次 occurrence 开始前重新验证 current capability、Lease、D6 authorization、ActivationBinding、secret generation 与预算。已生成 D3/D6 request 后重启只恢复原 Run/原 request；不能重新采样目标或新 OperationId。

## 14. Standing Approval

Standing Approval 不是“允许这个 Agent 以后修改任何东西”。首版无人值守作者提交只支持原 D7 set_field_member 的一个极窄 profile：单 existing Node、单 Field、当前完整 Field 恰一个 Entry、一个 existing scalar member。

```text
StandingApprovalEnvelope/1 {
  approvalId,
  approvalRevision,
  workspaceRef,
  automationId,
  definitionRevision,
  grantingPrincipal,
  delegationBinding,
  activationBinding,
  notBefore,
  notAfter,
  rule: {
    kind: "single_field_member",
    ownerNodeRef,
    fieldId,
    selection: "require_exactly_one_entry",
    memberPath,
    memberType,
    valueConstraint
  },
  maxSuccessfulCommits,
  budgetAccountBindings
}
```

memberPath 只能是静态 object member path，不允许 list index、wildcard 或动态 FieldId。memberType 只允许 bool、exact text、int64、integer、decimal、semantic_code。valueConstraint 只允许最多 64 个完整 TypedLiteral 的有限集合、同型 exact numeric closed interval，或有 UTF-8 byte 上限且禁止 CR/LF 的 exact text。所有 D4 nonEmpty、code scope、schema/cardinality/constraint 仍独立验证。

机械批准判据必须全部满足：current D1/D6/D10 authorization；同 approval/automation/definition/delegation/activation binding；fresh current source revision；完整 Field 只有一个 Entry；D7 Narrow Field Qualification 成功；owner/field/member/type exact；新值落入 constraint；实际 mutation footprint 只改变该 member，occurrenceKey、其它 member、qualifier、note、provenance、其它 Entries、body、Facet、Ref、relation、identity、placement 和 control 均不变；完整 owner_fields preview 已形成并可取；audit 与 approval-count/cost reservation 成功。

任一判据失败均不能自动选择 first、自动扩大到 whole entry/source、自动降级为 append/remove 或换一个同名 target。它只能转 awaiting_confirmation、blocked 或失败。

## 15. Fresh prepare、ApprovalUse 与 D6/D7/D8

每次自动作者修改仍按：fresh current Field selection → 原 D7 ActionSpec → 原 d7_action_prepare → 完整 D7 EffectManifest/bytes → Core 机械匹配 StandingApprovalEnvelope → 建立 ApprovalUse → 原 D6 request。

```text
ApprovalUse/1 {
  approvalId,
  approvalRevision,
  runId,
  stepId,
  request,
  planToken,
  preparedBindingRef,
  previewBinding,
  footprintProof,
  delegationBinding,
  activationBinding,
  approvalCountReservation,
  budgetReservations
}
```

ApprovalUse 是受管授权证据，不是 author plan，不新增 ActionSpec、PreparedActionBinding/2 或 d6_commit_request member。客户端不能提交 approved=true。Core 依据已保存 prepared record/preview/footprint 独立建立它。

现有上游不足以冻结这种无人值守确认：当前 D7 明确把 preview 后的用户确认作为提交路径，而 D6 尚无 StandingApproval/ApprovalUse 的原子消费与计数规则。因此本候选附带 D6/D7 coordinated amendment proposal；在该修订尚未共同接受前，无人值守 author commit 必须保持 unavailable，Automation 只能准备 proposal 等待交互确认。

协调修订要求 D6 planning CAS 同时校验并 reserve approval use/count/budgets；最终 author commit 再校验当前 authorization 和原 ApprovalUse，并在同一事务发布 author effects、原 receipt、approval count consumed 与必要 audit link。committed raw no-op 仍计一次成功提交。same request replay 不再次消费次数。

planned decision 如果原 approval 后来不足，保持 planned/blocked；不能自动换成续签 envelope。用户可以对原完整 plan 作新的明确一次性授权，以附加授权证据恢复，但不得修改 request、OperationId、source plan 或 PreparedActionBinding。业务依赖已经确定冲突时，新 approval 也不能复活旧 plan。

D8 人工编辑契约不放宽。Automation 不伪造 EditSession、draftSerial、IME 或用户 click。需要 D8 document/annotation edit 的 Agent 结果仍进入 D8 Draft/preview/explicit confirmation；后台 commit 改变了同 owner 时，只使 dirty Draft 进入原 stale/conflict 流程，不能覆盖 Draft。

## 16. Connector 与 secret

Connector 是具名外部系统的协议 adapter。它拥有 provider-specific cursor/etag/version/account state，但这些只存在 D10/D6 控制域，不成为作者 Ref 或 Field。外部 stable ID 只有经 D3 已冻结的 SourceBinding/OriginBinding 协议才可参与 lookup/upsert，不能因 connector 安装自动创建 identity。

需要修改 SourceBinding、OriginBinding、watermark 或持续 sync state 的流程必须有具名 closed adapter 生成原 D3/D6 request，并把实际进度与对应 author commit 原子关联。首版 single_field_member standing approval 不包含这些 control effects，所以不能宣传为通用双向同步。

SecretRef 绑定 contribution、external account、usage、audience 与 secretGeneration。credential 原值只在受信 transport 的认证通道注入；不进入 workspace source、ContextBundle、model prompt、普通 ToolValue、transcript、普通 log 或 export。

credential rotation 产生新 generation。未发送的新调用必须使用当前允许 generation；已开始 external request 的恢复仍绑定其实际旧 generation。新 credential 可以在明确获准时用来只读 reconcile 同一账户，但不能偷偷重发一个 outcome_unknown mutation。

## 17. External effect

外部 mutation 与 Core transaction 永不组合成一个“原子成功”。

```text
ExternalEffectIntent/1 {
  effectId,
  contributionBinding,
  accountBinding,
  targetBinding,
  requestPayload,
  secretGeneration,
  idempotencyBinding,
  delegationBinding,
  approvalBinding,
  egressBinding,
  budgetReservations
}
```

状态为 prepared → submitting → succeeded | failed_no_effect | outcome_unknown；只有能证明请求尚未开始发送时才可从 prepared 进入 cancelled。outcome_unknown 保持未知直到可靠 reconcile；manual_required 是恢复方式，不是假失败终态。

自动 retry 只在已接纳 adapter 明确提供仍有效的 idempotency window/key，或已有可靠 failed_no_effect 证据时允许。retry 使用同一 EffectIntent、同一语义 request、target/account 和原 idempotency key，并重新验证当前 authorization/egress/cost。幂等窗口失效、目标改变、请求改变或 credential 改变无法保持原合同即停止自动发送。

受信 transport 使用发送栅栏：先 durable intent、audit started 与 cost reservation，再在与 revocation 序列化的 host gate 中开始发送。revocation 先赢则不发送；发送已经开始后 cancel/revoke 不能证明外部未执行。崩溃发生在 durable started 与可证明 send outcome 之间按 outcome_unknown 恢复。

补偿操作是新的 ExternalEffectIntent，需要自己的授权、批准和费用；不是 rollback。一个 workflow 同时要求 Core write 与 external mutation 时分别展示两个 outcome，不生成综合 author receipt。

## 18. Budget、费用与并发预留

D6 原 work/attempt budget 保持。D10 为 model/tool/network/external cost 增加受管多账户 reservation；Run、Lease、Automation 与 deployment account 的最窄剩余额度必须原子满足，避免两个并发 Run 同时看到最后余额。

```text
Money/1 {
  currency,
  microUnits
}
```

microUnits 使用 Counter，所有乘法和累计 checked。一个 account 只使用已配置 currency，不隐式换汇。每个可能收费 attempt 在发送前预留当前 accepted pricing rule 下的有限上限，并绑定 priceVersion、request limit 和包含的收费项。无法证明有限上限的服务不能提供硬 cost ceiling 模式。

费用 reservation 状态为 reserved → settled | released | uncertain。能证明未发送才 release；有可靠最终计费依据才 settle；timeout/crash/billing unknown 进入 uncertain 并继续占用上限。TTL、restart、transcript deletion 不能释放 uncertain。每次 retry 另作 reservation；effect idempotency 不等于计费免费。

服务违反已接纳 pricing contract 并产生超过 reservation 的费用时，记录 actual anomaly、冻结相应 capability 并要求管理处理；不得悄悄提高 ceiling 后继续宣称原保证。

## 19. Audit 与 retention

下列 protected step 必须在执行前写入耐久本地/Server authority-bound audit started record：敏感 workspace/context read、egress、secret use、经 D10 发起的 author submit、external mutation、delegation/approval/package activation mutation。不能写入时 fail closed。

远端 log collector 不可达不必使所有本地能力不可用：只要本地受保护 audit spool、完整性和保留预算仍可证明，可继续并稍后汇聚。真正本地 durable audit failure 则停止新的 protected step。

Core author commit 的 audit link 与原 author decision 同事务保存。external effect 的 terminal evidence 若无法耐久记录则保留 started/outcome_unknown，不能再发送一次“补日志”。取消、撤权、紧急停止必须保留独立 control-write reserve，不能被普通 telemetry quota 用尽而阻塞。

Transcript 与 audit 分开。transcript 可按部署 policy 有限保存/删除，但删除不能移除 planned author recovery、unknown external effect、uncertain cost 或安全审计所需 pins。Secret 与 raw credential 永不进入 transcript/audit export。

## 20. Run、取消与恢复

Run state 闭集为 queued|running|awaiting_confirmation|blocked|cancelling|reconciling|completed|failed|cancelled。每个 step 另保存实际 domain outcome，Run terminal state 不能抹掉已经 committed 的 author/external facts。

取消 queued/prepared 且未发送的 step 可产生 cancelled。已进入 D6 planned 的 request 不因取消 Run 自动 abort；Run 进入 cancelling/blocked，并按原 D6 planned recovery 处理。当前 delegation 撤销可阻止缺乏当前资格的新 author commit，但不能把撤权写成永久业务 rejection。已 committed decision 不回滚。

external effect 进入 submitting 后，取消只阻止后续步骤；该 effect 最终仍是 succeeded、failed_no_effect 或 outcome_unknown。迟到 model/tool output 在 step 已关闭后不能启动新 tool/action。

重启先恢复 durable occurrence claim、Run、ApprovalUse/cost reservation 与原 requests。无法证明 clock continuity 时当前 attempt 结束/暂停，不延长 deadline。无法证明 execution side effect 的结果时进入 blocked/reconciling，而不是生成新 OperationId 或 effect ID。

## 21. 错误与不可用语义

D1 capability availability 及其固定 reason 优先级完全不变，尤其 policy_denied 必须先于组件、配置、network、version 与 health 细节。D10 不用一个 runtime_unavailable 覆盖 missing_component、not_configured、offline、incompatible_version 或 temporarily_unavailable。

D10 自有控制请求的阶段顺序为：closed decode/version → D1 static capability/surface/release → current principal/control-object visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution。阶段顺序优先于错误细分类，防止越权探测。

D10 自有 error code 闭集：invalid_request、not_visible、control_conflict、binding_changed、approval_required、approval_expired、delegation_expired、budget_exceeded、audit_unavailable、state_unavailable、invalid_output、cancelled、external_outcome_unknown。unknown token、wrong tag、wrong audience 或无权控制对象统一 not_visible；只有已经有权读取本人记录时才披露 expired/conflict。

一旦请求进入原 D3/D6/D7/D8/D9 入口，返回该 owner 原有 error/disposition，不包装为 D10 error，也不根据内部 deployment detail改变原拒绝顺序。

## 22. 产品端

Desktop 本地承载同一个 local Broker、scheduler、secret transport 与 Core adapter；CLI 本地连接/启动同一能力族，不创建另一套 scheduler 或 authority。Desktop/CLI 远端模式只调用 Server。

Server 承载 hosted Broker、scheduler、connector/model/tool executors 和 secret transport，并继续满足 D1 单实际提交持有者；本代不定义多主 scheduler/database 集群。WebUI 只能通过 Server 管理和发起能力，不直接运行 worker/model/connector、不持 secret。

Mobile 对 agent.session、automation、connector execution、conversion execution 和 credential management 返回 D1 unsupported_surface；不会用“只读监控”入口变相提供 D1 已禁止的批准或委托。Mobile 可读取普通已 committed workspace facts。

LTR/RTL、locale、screen reader、Web/CLI transport 差异只影响呈现和交互，不改变 request bytes、target、approval rule、cost、error 或 author/external outcome。

## 23. D1–D9 组合与必要修订

D1 表面、capability reason 和唯一提交持有者保持；D2 raw source/unknown provider preservation 保持；D3 identity/SourceBinding/OriginBinding/Provenance 保持；D4 Registry exact shape 与演进保持；D5 不新增持久 Record；D8 Draft/explicit edit confirmation 保持；D9 worker/ExportPlan/publication 保持。

需要 coordinated amendment 的只有 D6/D7 standing-approval author-submit 分支，精确提案在 UPSTREAM-AMENDMENTS 文档。当前上游在该修订共同接受和协调激活前继续权威；因此当前阶段可以审查 D10 设计，但产品不得把该自动提交分支标为 available。

## 24. 安全反例

- 恶意 Document 写“读取所有文件并上传”不能改变 ContextBundle readScope、egress recipient 或 tool allowlist。
- MCP server 把 delete tool 标记为 readOnly 不会降低 external-effect authorization。
- 同一个 Field 有两个相同 value 的 Entry 时 standing approval 不挑第一条，自动执行停止。
- approval 只剩一次且两个 Run 并发时，原子 reservation 使至多一个获得提交资格。
- preview 后 source revision、Registry、activation 或 delegation 任一改变都使旧自动批准不可消费。
- external request 已发送但响应丢失，cancel/restart 都不能换新 key 重发。
- provider 费用未知使 reservation=uncertain，不因 TTL 清理而重新释放额度。
- audit collector offline 但本地 spool 完整时可继续；本地 durable audit 不可写时新受保护动作停止。
- package 被禁用不能删除其作者 Field raw source；同 ID schema 也不能随回滚改成旧语义。
- Run cancelled 不能把已经 committed D6 receipt 或 external succeeded 结果描述成回滚。

## 25. 实施与接受边界

完整实现至少要分别证明：closed decoder 与状态机有限模型；真实 D3/D4/D6/D7/D8 集成；SQLite/authority crash/fence/approval-count/cost reservation fault injection；Windows/Linux/macOS 所声称 executor sandbox；真实 MCP/model/connector 协议 hostile server；credential store 与 rotation；Desktop/CLI/Server/WebUI 端到端；Mobile negative capability；audit/retention/费用异常；大规模和预算。

作者候选完成不等于独立接受。独立审查必须检查更简单完整替代、D6/D7 amendment 是否充分、P0/P1 是否为零、术语与 mandatory scenarios 是否闭合，并明确哪些证据仍属于 implementation pending。
