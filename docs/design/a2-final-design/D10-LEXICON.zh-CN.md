---
source_language: zh-CN
translation_status: source
---

[English](D10-LEXICON.md)

# D10 术语与命名

## A2 D10 current 名称、真实继任类型与历史别名

完整保留原 §§1–15 的受控名称、逐概念 13 字段和反例。本文 `firstFreeze`、旧 Concept、36+1/17-kind 数量只是来源资格，不能据此重注册 D6/D7/D8/D9-owned 名称或阻断真实 D9 Annotation/View typed helper。最新 owner 分派由 [D10-UPSTREAM §8.2](D10-UPSTREAM.zh-CN.md) 与 [D10-CONTROL §0](D10-CONTROL.zh-CN.md) 自足列明：D3 Request13、D6 Key3/Proof3/Descriptor3（十五成员）、D7 PAB4/Effect3、D8 PreparedEditBinding3、D10 ControlBinding3/ExecutionRecord3/Schedule2、D9 ExportPlan4/Catalog3/Selection2/Projection2/Loss2/Receipt4/PrintReceipt1。`PackageId` 不等于 D4 Namespace，`FieldId` 不等于任意 ToolValue，`PublicationReceipt`、`D9PrintReceipt` 及 `D6` 作者回执永远不同。明确排除假造 Resolver12、新 `adoption_binding` 和凭名称推 identity/来源的 alias；UI locale/CLI 和第一方模块代码只是尚未实现的候选映射。实际历史 decoder 原样保留。

---


revision: D10-FA-r01-2026-10-02；状态：协调作者候选，未接受、未激活、未实现。最近一次完整历史 R08 评审绑定 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 与 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，结论 REVISE（P0=0、P1=3、P2=8）。十一项历史最终处置仍为 OPEN。具名修订已有限定独立复核，实际跨 owner 整合及 fresh 全局接受仍未完成；D10-REVIEW 区分各层证据。

## 1. 命名原则

- 作者内容、身份、定位、来源、Field、Facet、Query、Action、Draft、conversion、publication 等词继续由其原阶段拥有。
- D10 控制身份不能借用 D3 EntityRef/Locator 命名；局部 ordinal、token、ID 也不能被描述为内容身份。
- “plugin”只作为用户/历史泛称，不作为 canonical wire family。受控文档应区分 Extension Package、Contribution、Connector、Tool Adapter、Model Adapter、Pack 与 Bundled Module。
- 裸 “Provider” 易与 D9 Conversion Provider 冲突；D10 必须写成 Model Provider、External Service、Connector Provider 或直接写 Contribution/Adapter。
- “Registry”在 D10 正文默认指 D4 semantic Registry；D10 自身使用 Capability Catalog，不另建“plugin registry”。
- “approval”“authorization”“delegation”“confirmation”分域：D6 Policy 是工作区授权；Delegation Lease 只能减权；Standing Approval 是有限预授权；D8 confirmation 是当前完整预览的人机确认。
- “source”必须加限定：作者 source、外部输入、工具结果、提供方状态、包资产或 SourceBinding 证据；提供方状态不是 author source。

## 2. D10 新概念

| conceptId | 中文 / English | owner | 定义 | 明确排除 |
| --- | --- | --- | --- | --- |
| weftext.term.extension-package | 扩展包 / Extension Package | D10 | 具名版本、内容摘要、publisher、依赖和贡献目录组成的分发/升级单元 | 不是权限单元、namespace owner、作者对象 |
| weftext.term.contribution | 贡献项 / Contribution | D10 | package 中可单独接纳、启停和授权的一个纯数据或可执行能力 | 不等于整个 package 的权限 |
| weftext.term.pack | 扩展包数据包 / Pack | D10 | 无任意宿主权限的声明、规则、schema 或数据 contribution 集合，归属一个父领域/extension point | 不是 executable Extension、Connector、作者 Profile 或 D4 Registry |
| weftext.term.bundled-module | 内置模块 / Bundled Module | D1 产品归属 + D10 lifecycle consumption | 第一方用户可见产品模块，可提供领域 extension points 和 UX | 不是 Core identity、package owner 或作者数据库 |
| weftext.term.agent-session | Agent 会话 / Agent Session | D10 | Run 的交互式 Agent 模式，绑定一个受管任务、上下文与工具集合 | 不是作者实体、永久 principal 或独立 ledger |
| weftext.term.parent-extension-dependency | 父扩展依赖 / Parent Extension Dependency | D10 | domain Pack/Contribution 对父 domain、extension point 和兼容版本范围的必需依赖；激活时解析并绑定 | UI 隐藏不是依赖失效，已接受 D4 定义历史也不是运行时激活 |
| weftext.term.capability-catalog | 能力目录 / Capability Catalog | D10 | 当前激活部署中的 contribution/runtime 可用性和 host privilege 描述 | 不是 D4 Registry、D1 capability response 或作者 schema |
| weftext.term.activation-binding | 激活绑定 / Activation Binding | D10 | 将 current D4 RegistryBinding、Capability Catalog digest 与 trust revision 固定为一个受管代际的记录 | 不是 D4 semantic generation 的替代 |
| weftext.term.publisher-identity | 发布者身份 / Publisher Identity | D10 | 由接纳 trust root 证明的 package 签名主体 | 不是 namespace ownership 本身 |
| weftext.term.namespace-claim | 命名空间所有权声明 / Namespace Claim | D10→D4 proof | 绑定允许的 D4 命名空间 owner tuple，并由第一方信任根或已接纳发布者身份认证的带证据声明 | 安装顺序、display name、enablement 均不是 claim |
| weftext.term.delegation-lease | 委托租约 / Delegation Lease | D10 | 对当前 D6 principal 权限作进一步有限收窄的任务/运行授权边界，含有限 maxRuns | 不增加 D6 capability，不可多级转授 |
| weftext.term.lease-run-use | 运行准入消费 / Lease Run Use | D10 | 某 Run 第一次获准进入受保护执行时对同一 leaseId 谱系消费一次 maxRuns 的耐久防重事实 | queued claim 不是消费；失败、取消、崩溃后不退款 |
| weftext.term.standing-approval | 持续批准 / Standing Approval | D10 | 对有限、可机械判定的未来操作集合给予有期限/次数/预算上限的批准 | 不是永久同意、自然语言目标或 D6 Policy |
| weftext.term.approval-use | 批准使用记录 / Approval Use | D10 + proposed D6 amendment | 把一份已准备请求、完整预览、实际 footprint 与一个批准的单次消费或预留绑定；count 状态可为 unreserved、reserved、consumed、released_terminal | 不是 author plan、receipt 或第三 ledger |
| weftext.term.planned-decision-approval | 已计划决议交互批准 / Planned Decision Approval | D10 + proposed D7/D6 amendment | 用户在新的受保护 recovery preview 中完整查阅原 planned 语义后，对 exact 原 request 给出有限一次性交互授权 | 不重新 prepare、不换 OperationId、不复活已冲突 plan |
| weftext.term.automation-definition | 自动化定义 / Automation Definition | D10 | 一个受管、版本化的单 invocation 调度定义 | 不是通用 workflow DAG 或作者 Document |
| weftext.term.automation-occurrence | 自动化发生项 / Automation Occurrence | D10 | 由 Automation 订阅代际与精确原发生 UTC 坐标确定的一次调度机会 | 不等于作者 recurrence occurrence identity |
| weftext.term.run | 运行 / Run | D10 | Agent 或 Automation 的受管执行记录及 step outcomes | Run completed 不等于 author committed |
| weftext.term.context-bundle | 上下文包 / Context Bundle | D10 | 当前受权、最小化、版本绑定、面向特定 recipient 的模型/工具上下文 | 不是 author snapshot 或可转授权 token |
| weftext.term.tool-value-profile | 工具值配置 / Tool Value Profile | D10 | D10 自有 closed ToolValueProfile/ToolType/ToolValue 工具参数/结果代数 | 不是任意 JSON Schema、D7 TypeSpec alias 或 D4 Field value 全集 |
| weftext.term.input-slot | 输入槽 / Input Slot | D10 | 单次 invocation 使用、绑定 exact bytes/用途/recipient 的受限文件输入句柄 | 不是路径、ResourceRef 或跨调用 file handle |
| weftext.term.tool-adapter | 工具适配器 / Tool Adapter | D10 | 把一个已接纳外部 tool protocol 映射到 Tool Value 与明确 effect class 的 adapter | 不是作者 Action adapter |
| weftext.term.mcp-adapter | MCP 适配器 / MCP Adapter | D10 | Tool Adapter 使用 MCP 作为运输/发现协议的具体类型 | MCP annotations/prompts 不构成 Weftext 权限 |
| weftext.term.model-adapter | 模型适配器 / Model Adapter | D10 | 将受管 ContextBundle 与模型 API/本地模型 transport 连接的 adapter | 模型输出不是 author authority |
| weftext.term.connector | 连接器 / Connector | D10 | 具名外部系统的协议、账户、cursor/version 与 sync control adapter | provider ID/cursor 不是 EntityRef |
| weftext.term.secret-reference | 凭据引用 / Secret Reference | D10 | 对 OS/Server secret store 中实际 credential 的受管引用及 generation | 不是 secret bytes、author source 或 model input |
| weftext.term.external-effect-intent | 外部效果意图 / External Effect Intent | D10 | 已冻结 target/account/request/idempotency/approval/budget 的一次外部 mutation 请求 | 不是 D6 transaction 或 rollback |
| weftext.term.external-outcome-unknown | 外部结果未知 / External Outcome Unknown | D10 | 无法证明外部 mutation 是否发生或是否完整发生的持久状态 | 不是 failed 或 cancelled |
| weftext.term.cost-reservation | 费用预留 / Cost Reservation | D10 | 一笔 billable attempt 对恰一个实际 cost account/grant/pricing/currency 的有限上限 reservation；准入可同时检查 Run/Lease/Automation/Workspace/deployment 多层 ceiling | 不是 billing fact、effect idempotency，也不是一份跨多个实际账户的 reservation |
| weftext.term.audit-started | 审计开始记录 / Audit Started Record | D10 | protected step 真正执行前耐久写入的最小安全审计事实 | 不代表 step 已成功 |
| weftext.term.money | 费用值 / Money | D10 | currency + Counter microUnits 的 exact cost 表示 | 不做隐式 FX，不使用 binary float |

## 3. 现任 fresh 名称与真实已记录历史

现任 A2 D10 fresh 名称：`D10AuthorPreparationLink/2`、`ApprovalUse/2`、`ControlPrepareBinding/3`、`D10InteractiveRunStartRequest/1`、`D10InteractiveRunResultRequest/1`、`D10InteractiveRunStarted/1`。原 D7 fresh 拥有 `PreparedActionBinding/4`、`EffectManifest/3`、`EffectBytes/3`；D8 拥有 `PreparedEditBinding/3`；D6 拥有 `WorkspaceBootstrapProfile/4`、`WorkspaceBootstrapPlan/4`、`WorkspaceTrustGenesis/2`。这是**现任首次生产**类型，并非真实历史 `D10AuthorPreparationLink/1`、`ApprovalUse/1`、`ControlPrepareBinding/2`、`PreparedActionBinding/3`、`EffectManifest/2`、`EffectBytes/2`、`PreparedEditBinding/2`、Profile1–3/Plan1/Plan3 的 alias。真实历史记录保留原 request/OperationId、decoder、完整 pin 字节、preview proof 与错误次序。唯一 D7 Registry 负责按版本分派二者。

其他未变 D10 受控名称：`ActivationBinding/1`、`DelegationLease/1`、`LeaseRunUse/1`、`StandingApprovalEnvelope/1`、`PlannedDecisionApproval/1`、`ToolValueProfile/1`、`ToolType/1`、`ToolValue/1`、`InputSlot`、`AutomationOccurrenceKey/1`、`ExternalEffectIntent/1`、`Money/1`、`CostBudgetAttribution/1`、`CostBudgetLayer/1`。未变 D3/D4/D6 继承名称：`RegistrySnapshot/1`、`RegistryBinding/1`、`PrincipalContext`、`ObservationScope`、`PreparedIntent`、`ActionSpec`、`SourceBinding`、`OriginBinding`、`Provenance`、`SourceVersion`、`OperationId`。D10 不定义 alias。

**责任分支与版本证据：**新自动 core_field_member D7 PAB4 + Manifest3/EffectBytes3 必须绑定/pin 真实 Automation 的 Link2/ApprovalUse2/Standing Approval 并按原作者恢复；fresh interactive step 则按 `preparedFormat` 使用 D3 Request13 或 D6 request2、D7 PAB4 或 D8 PreparedEditBinding3、受保护 preparedRecordPin/recoveryPins 和逐次全量 preview 后本人认证明确确认，不需要 Link2/ApprovalUse2。真实记录的 Link1/PAB3/ApprovalUse1 或历史 interactive prepared binding 只按自身 request/semantic preview/pins/decoder 重放，不归一化为新版本。Link1 冒名 Link2、PAB3 当作 PAB4 解码、Manifest2 以 Manifest3 域重新哈希或 Link2 指向 PAB3-only pin，均按原披露/custody/version 错误次序拒绝。新 Run 仅来自可信到场 Core §7.1；历史 origin 不从显示文字猜测。

## 4. Package、module、pack 与 plugin

**Extension Package**是安装、升级、签名与资产完整性单位，可以包含一个或多个 Contribution。权限、依赖和运行可用性逐 Contribution 计算，拒绝某个网络 Connector 不得自动禁用同 package 的纯数据 contribution。

**Bundled Module**表示第一方产品组织/UI 形态，例如 Calendar、People、Organizations、Library 等 D1 已有模块方向。module 可以提供领域 extension point，但不成为新的 namespace owner class、作者数据库或 Core identity。

**Pack**只用于没有任意 code/process/network/secret/file host privilege 的声明式语义、规则、schema 或数据 contribution。一个需要执行代码、网络、凭据或复杂运行行为的“pack”必须在受控面重新分类为 executable Contribution/Connector，而不是靠名称规避 runtime gate。

每个 domain Pack Contribution 必须归属一个 primary parent domain/extension point，并声明 required compatible version range；这项依赖是 D10 Capability Catalog 的激活依赖，不是 UI 层级。parent UI 只隐藏入口时，若 parent semantic capability 仍 active compatible，则 Pack activation 不变；真正 parent missing、disabled、incompatible 或 surface unsupported 才使 dependent contribution inactive。package 本身仍可保持 installed/verified，配置、来源和历史 binding 可恢复。

Pack 的**语义定义保留**与**运行激活**分开。已经进入 D4 semantic ledger 的 schema/Field/Facet 定义与历史不能因为 Pack 或 parent UI/module disable/uninstall 被删除；当前 RegistryBinding 若仍证明 definitions complete，Core 可以继续解释已有作者 facts，但这不意味着 Pack 的 View/Action/rule/connector 仍 active。definitions 无法证明时按 D4 unavailable/raw-preservation 处理。

**plugin**仅为用户可理解的泛称或历史文本。controlled schema、CLI/API、测试 fixture 和候选设计不得用 plugin 替代具体类别；它也不能成为 Pack、Module、Connector、Provider 的共同 wire kind。

## 5. Registry、Catalog 与 capability

**D4 Registry**持有 semantic namespace、Field/Facet/Relation/Calendar/unit 等作者语义，并按 D4 累计 evolution 规则演进。D4 Registry 的 complete/unavailable、generation、digest、tombstone 和 migration 历史不能被 D10 package enablement 重写。

**Capability Catalog**持有 contribution/runtime 的当前部署描述、parent extension-point 依赖解析和 host privilege；它不能定义 Field、Facet 或关系语义。Catalog change 与 Registry change 可以由同一个 Activation Binding 协调，但两者不是同一对象。

**Parent Extension Dependency**只描述 parent domain、extension point、required version range 与当前解析结果；它不是 D4 schema ownership，也不是 UI 可见性。resolved parent binding 必须进入 current Catalog digest，parent version/binding 改变后旧 dependent activation 失效并要求 successor ActivationBinding。

**D1 capability**描述某个正式产品端在当前 release/current subject 下是否可提供产品能力，并使用 D1 固定 unavailable reasons。D10 dependency state 只作为 D1 capability 判断的一项下游事实，不新增平行 reason。更高优先级项仍先应用；若 parent dependency 本身是首个阻塞原因，则 missing→`missing_component`、disabled→`not_configured`、incompatible→`incompatible_version`、surface unsupported→`unsupported_surface`。UI hidden-only 不产生 unavailable reason。
## 6. Provider 术语消歧

D9 的 **Conversion Provider** / **Route** 保持 D9 owner，不被 D10 重命名。

D10 中：
- 模型服务写 **Model Provider**；
- 外部 SaaS/系统写 **External Service**；
- 协议实现写 **Connector** 或 **Tool Adapter**；
- 只有上下文明确且不会与 D9 混淆时，自然语言才可简写 provider。

“provider unavailable”不新增为 D10 capability reason；正式 capability 仍使用 D1 的 `missing_component`、`not_configured`、`offline`、`incompatible_version`、`temporarily_unavailable` 等原 reason。

## 7. Identity、source 与 provenance 消歧

EntityRef、NodeRef、ResourceRef、AnnotationRef、Locator 与 OperationId 均由原上游拥有。

RunId、AutomationId、ApprovalId、ExternalEffectId、AuditEventId、tool call ID、package ID、provider account ID 都是控制/外部身份，不能进入 EntityRef。

author source 是 D2/D3/D6 认可的作者 payload；外部输入、ContextBundle、工具结果、模型输出、包资产、transcript、连接器缓存和提供方响应都不是 author source。

D3 Provenance 只是来源证据，不授予 authority。D10 package/provider origin 同样不能转化为写权限。SourceBinding/OriginBinding 是 D3 绑定语义；Connector 自己的 cursor/etag 不得借名“binding”后绕过它们。

## 8. Approval、confirmation、authorization 与 delegation

**authorization**：当前 D6 Policy、ObservationScope、authority/cut 与适用上游权限门共同给出的当前工作区资格。

**delegation**：Delegation Lease 对当前 authorization 的进一步收窄；它不是授权来源。maxRuns 是同一 leaseId 谱系允许进入受保护执行的 Run 总次数，不按 leaseRevision、重启或 scheduler cache 重建清零。

**Lease Run Use**：Run 在第一次受保护步骤前通过原子准入 CAS 时写入的耐久消费事实。同一个 Run 恢复复用原记录；准入后即使失败或取消也不返还次数。

**interactive confirmation**：当前完整 preview 后由用户明确确认原 request；D8/D7 保持默认路径。

**Standing Approval**：仅对候选规定的机械 envelope 预先允许未来相同类型操作，并仍需 fresh prepare、preview 与 current authorization。

**ApprovalUse**：一份已准备 request 实际消费 Standing Approval 的受管绑定；不能由客户端自报。其次数 reservation 在 D6 planned 时才成为 reserved，commit 后 consumed，只有 authoritative terminal_failed 才能原子转为 released_terminal。

**Planned Decision Approval**：原 D6 decision 已经 planned、旧 preview transport 已过期或 Standing Approval 不再可用时，用户通过新的只读 planned-preview recovery epoch 完整查阅原保存语义后，对 exact 原 request 给出的有限一次性交互授权。D7 owner 按原实际保存的 prepared version 分派：现任 PAB4/Manifest3/EffectBytes3 或真实历史 PAB3/Manifest2/EffectBytes2，分别用自身 closed decoder、完整原 bytes/preview digest 与 pins；外层 planned-preview wire 不变。它不改变 request、OperationId、PreparedActionBinding 或 target，也不能使已经发生确定 dependency conflict 的 plan 复活。

**approval-required / approval-expired**：只属于进入正式 D6 请求之前的 D10 控制结果。若 D10 自动路径已经进入 D6 后 approval 发生竞争失效，配套修订使用 D6 approval_unavailable/preflight，不能再伪装成 D10 前置错误。
## 9. Secret、context、egress 与 external effect

Secret Reference 只指 secret store 中的 credential；secret bytes 不进入 ToolValue、ContextBundle 或普通日志。

Context Bundle 是一次模型/工具调用的实际受权上下文。持有 ContextBundle 不授新的 read，也不允许换 recipient。

egress 指数据离开当前受信 Core/host boundary 到具名 model/tool/connector/external service；network access 只是 transport 能力，不能自动授权任意 egress recipient。

external side effect 指改变外部系统状态的请求。读取外部数据和写外部数据使用不同 effect class；“read-only tool”只在 Weftext 本地接纳 contract 下成立，不能信任远端 self-annotation。

External Outcome Unknown 表示已无法证明 external request 的结果；不能称 timeout、failure 或 cancellation。费用 released 只表示能证明收费执行从未开始；已经发送但可靠账单为零属于 settled(0)。

## 10. Audit、transcript、log 与 evidence

Audit 是恢复与安全所需的受保护事实；Transcript 是可选的人机/model 对话记录；ordinary log/telemetry 是运行观测。三者保留、权限与导出规则不同。

Audit Started Record 只证明 protected step 已进入执行边界，不证明其成功。真正 author 成功仍只看 D3/D6 receipt；external 成功看具名 adapter 规定的完整 success evidence。

测试 report、CI log、model witness、screenshot 和 protocol trace 都是 evidence，不是 authority。候选必须标出它们证明的有限范围。

费用预算归属是每份 reservation 的受保护事实来源，支撑真实 Run/Lease/可选 Automation/Workspace/grant/account 货币投影。准确名称为 `CostBudgetAttribution/1`、`CostBudgetLayer/1`；code type CostBudgetAttribution/CostBudgetLayer、variable cost_budget_attribution/cost_budget_layer 归 D10 cost control。它不是新账户、公共账务 payload、第二笔 charge 或独立余额账本，没有用户控件/locale 或历史 alias。不可变原归属在改配置后仍保留，并驱动全部原层结算；当前显示名或配置不能替代。此辅助合同依据 D10-CONTROL §10 补全既有 Cost Reservation 概念。

## 11. 受控禁止混称

- 不把 Capability Catalog 称 D4 Registry。
- 不把 package install 称 namespace registration 成功。
- 不把 Enable/Disable 称作者事实创建/删除。
- 不把 Run completed 称 committed。
- 不把 model/tool proposal 称 Draft，除非实际进入 D8 Draft。
- 不把 Preview 称 receipt。
- 不把 ApprovalUse 称 author plan。
- 不把 ExternalEffectIntent 称 transaction。
- 不把 compensation 称 rollback。
- 不把 outcome_unknown 称 failed。
- 不把 cancel Run 称 cancel D6 planned decision。
- 不把 SecretRef 称 credential bytes。
- 不把 ToolValue text 中的 UUID/Ref spelling 称 EntityRef。
- 不把 MCP prompt/tool annotation 称 policy。
- 不把 finite model pass 称 product support。

## 12. 术语门完成条件

独立审查前应机械扫描 D10 controlled headings、类型名、error code、state、candidate interfaces、CLI/API 示例与双语 pair，确保上表 owned names 一致；扫描不得把历史引用、用户正文、反例或第三方自然语言误判为受控 positive surface。

独立 reviewer 仍需人工检查 Registry/Catalog、Provider、source/binding、approval/authorization、Run/transaction、audit/transcript 等高风险概念是否在上下文中保持上述分域。本文状态为 candidate，不自证 terminology gate 通过。

## 13. 逐概念结构化映射

本节是 Intake §8.5.1 要求的机械可追踪 Lexicon 产物。表中的“未公开 IPC”“无直接 CLI”等是明确结论，不表示将命名推迟给实现；后续若要公开新的 wire/API/CLI 名称，必须先修订本表。locale key 是候选受控映射，当前 PR 不实现资源文件。继承自 D1–D9 的名称继续由原 owner 冻结，D10 只引用，不建立 alias。

| stable concept/term ID | 中文正式名 | English formal name | 精确定义 | owner/layer | 排除边界 | canonical wire/API/manifest/schema | code type/function/variable/namespace | CLI/UI label + locale key | 允许简称 | 禁止/退役/历史 alias | 正例 / 反例 | 首次冻结、状态、迁移/删除目标 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| weftext.term.extension-package | 扩展包 | Extension Package | 具名版本、内容摘要、publisher、依赖和 contributions 的分发/升级单元 | D10 package lifecycle | 不是权限单元、namespace owner 或作者对象 | 闭合公共值 `PackageManifest/1` 位于 `DeploymentValue/1` 的 `package` 分支，通过 `d10_control_prepare` 的 `deployment_put` 及公共 `package_state` 投影承载；无独立扩展包 RPC | type `ExtensionPackage`; variable `extension_package`; package ID 不进入 EntityRef | UI“扩展包”; locale `term.extensionPackage`（候选映射，资源未实现） | 无 | 禁用裸 `plugin` 作为受控同义词；历史用户口语可出现 | 正：一个包含 schema+connector；反：一次安装获得全部权限 | 首次冻结 `D10-r01`；`r04` 补全映射；无公开旧 wire，发布前删除泛化 plugin 受控面 |
| weftext.term.contribution | 贡献项 | Contribution | package 中独立接纳、启停、授权和依赖解析的纯数据或可执行能力 | D10 capability lifecycle | 不继承整个 package 权限，不是 D1 capability response | 闭合公共值 `Contribution/1` 位于 `PackageManifest/1.contributions`，由同一扩展包控制请求及公共投影承载；无独立贡献项 RPC | type `Contribution`; variable `contribution`; canonical contribution ID | 通常无独立 CLI；设置 UI“贡献项”; locale `term.contribution` | 无 | 禁止把 package/plugin/provider 当同义词 | 正：同包 data pack 可用而 connector denied；反：whole-package grant | `D10-r01`; 无旧兼容 alias |
| weftext.term.pack | 扩展包数据包 | Pack | 无任意代码、网络、secret、文件权限的声明/规则/schema/data contribution 集合，绑定父 domain extension point | D10 extension taxonomy + parent domain owner | 不是 executable Extension/Connector，也不是作者 Profile | 公共 `PackageManifest/1` 中的 `Contribution/1.kind="pack"`，必需 `role="primary_parent"` 的 `ContributionDependency/1`；无独立 Pack RPC | type `PackContribution`; variable `pack`; namespace 仍由 D4 owner 规则决定 | UI 按父域显示“扩展包”; locale `term.pack` | pack | 禁用“Weftext pack”作为无父域正式名；`plugin` 仅历史/口语 | 正：Calendar Holiday Schedule pack；反：带网络凭据仍称 pack | `D10-r04` 显式冻结父域生命周期；既有候选 Pack 名保留，无 wire 迁移 |
| weftext.term.bundled-module | 内置模块 | Bundled Module | 第一方用户可见产品组织抽象，提供领域 UX 和 extension points；其关闭不改变 Core 身份或作者 source | D10 package/product mapping；D1 只拥有 surface/capability availability | 不是 package identity、D4 namespace owner、Core contract 或作者数据库 | 本代不声称 D1 已有 module ID；具体四模块使用 §14.2 的 PackageId + module contribution 映射 | 候选 code/namespace 见 §14.2，均未实现；不造第二 D4 namespace | 正式中英 label 与候选 locale 见 §14.2；locale resource 未实现；无独立 CLI verb | module（文档限定） | 禁止把 module 与 Extension Package/Pack 混称，也禁止把 D4 ownerId 当 module ID | 正：Calendar UI 隐藏但 portable author facts 保留；反：关闭 UI 删除 schema | `D10-r04` 冻结三轴 lifecycle；`D10-r05` 补 package/module/schema 映射；无 legacy runtime ID |
| weftext.term.capability-catalog | 能力目录 | Capability Catalog | 当前 ActivationBinding 下 contribution/runtime、依赖解析和宿主权限的部署描述 | D10 control domain | 不是 D4 Registry、D1 capability response 或作者 schema | 受保护派生目录；公共 `ActivationBinding/1.capabilityCatalogDigest` 与已解析 `ContributionBinding/1` 标识其接纳 cut；无公开全目录枚举 RPC | type `CapabilityCatalog`; variable `capability_catalog` | 无直接普通用户 label；管理诊断“能力目录”; locale `term.capabilityCatalog` | Catalog（仅 D10 上下文） | 禁止裸 Registry/plugin registry | 正：记录 parent dependency=disabled；反：保存第二份 FieldDefinition | `D10-r01`; `r04` 扩展 parent dependency；无旧 wire |
| weftext.term.parent-extension-dependency | 父扩展依赖 | Parent Extension Dependency | 一个 Contribution 对精确父 package/contribution/extension-point tuple 及兼容 contract version range 的必需依赖，在激活时解析 | D10 Capability Catalog; parent semantic owner remains domain module | 不是 UI 可见性、D4 semantic history 或授权 grant | 公共闭合 `ContributionDependency/1` 位于 `Contribution/1.dependencies`：role、parent 的 packageId/contributionId/extensionPointId、requiredContractRange；精确已解析父 binding 进入受保护 Catalog | type `ParentExtensionDependency`; variable `parent_extension_dependency` | 通常无直接 UI；诊断“父扩展依赖”; locale `term.parentExtensionDependency` | parent dependency（文档） | 禁止用 UI hidden/安装顺序代替依赖状态 | 正：Calendar pack pin 到 compatible extension point；反：Calendar disabled 时 holiday contribution 仍运行 | 首次冻结 `D10-r04`；当前公共载体为 ContributionDependency/1；无旧别名 |
| weftext.term.activation-binding | 激活绑定 | Activation Binding | 把当前 D4 RegistryBinding、Capability Catalog digest 与 trust revision 固定为一个受管激活代 | D10 control domain + D4 consumption boundary | 不是 D4 semantic generation 或 runtime health | closed public value `ActivationBinding/1`；由 Lease/Approval/Run current projection 和 activation control 承载；无独立 RPC | type `ActivationBinding`; variable `activation_binding` | 无普通 CLI/UI label；诊断“激活绑定” | 无 | 禁止无版本 current/latest 指针 | 正：同 cut selector 指向 successor binding；反：半 Registry/半 Catalog | `D10-r01`; R08 明确 public carrier；历史 binding 保留 |
| weftext.term.publisher-identity | 发布者身份 | Publisher Identity | 由接纳 trust root 证明的 package 签名主体 | D10 package trust | 不是 namespace ownership、用户 principal 或 Node identity | 信任控制使用 `DeploymentValue/1` 的 trust.publisherId/publicKey/proof 与公共 `trust_state`；`PackageManifest/1.publisherKeyId` 选择已接纳签名密钥；没有名为 PublisherIdentity 的公共对象 | type `PublisherIdentity`; variable `publisher_identity` | 管理 UI“发布者”; locale `term.publisherIdentity` | publisher（限定上下文） | 禁止把 signature-valid 当 namespace owner | 正：key rotation continuity；反：self-signed 首装 claim reserved namespace | `D10-r01`; 无兼容 alias |
| weftext.term.namespace-claim | 命名空间所有权声明 | Namespace Claim | 绑定完整 D4 命名空间 owner tuple 的带证据声明：已验证第一方信任根证明对应的精确保留第一方 tuple，或已接纳发布者身份证明允许的发布者 tuple | D10 trust proof consumed by D4 owner gate | 不是安装、enablement、display name | 闭合公共值 `NamespaceClaim/1` 位于 `DeploymentValue/1` 的 `trust` 分支及公共 `trust_state` 投影；D4 namespace row 形状不改 | type `NamespaceClaim`; variable `namespace_claim` | 管理 UI“命名空间声明”; locale `term.namespaceClaim` | 无 | 禁止 namespace registration=install | 正：`people`、`first_party`、`weftext.people` 及绑定该 tuple 的有效证明；反：声明 `core`、`wf` 或 `user`，冒充其他保留 tuple，或两个发布者拥有同一命名空间 | `D10-r01`; 无旧 alias |
| weftext.term.delegation-lease | 委托租约 | Delegation Lease | 进一步收窄 D6 principal 的有限任务/Run 授权，含寿命、scope、maxRuns 与预算账户 | D10 delegation control | 不是 D6 Policy，不可增加权限或多级转授 | 受保护 `DelegationLease/1`；公共创建/更新由 `automation_configure` 中的 `LeaseSpec/1` 承载，公共查阅使用 `lease_state`；无独立 lease RPC | type `DelegationLease`; variable `delegation_lease` | UI“委托”; locale `term.delegationLease` | Lease（D10 限定） | 禁止 permanent grant/session-wide allow 作为同义词 | 正：maxRuns=1 同 Run 多步；反：新 Run 在耗尽后继续 | `D10-r01`; `r02/r04` 收敛 maxRuns；无 legacy alias |
| weftext.term.lease-run-use | 运行准入消费 | Lease Run Use | Run 第一次进入受保护执行时对 leaseId 谱系消费一次 maxRuns 的耐久防重事实 | D10 run admission control | 不是 occurrence claim、作者 ledger 或每-step 计数 | 受保护 `LeaseRunUse/1`；公共 `run_state.admission` 与 `lease_state` 累计计数仅投影准入事实，不返回完整保护记录；无独立 RPC | type `LeaseRunUse`; variable `lease_run_use` | 无直接 UI；诊断“运行准入”; locale `term.leaseRunUse` | 无 | 禁止 run counter/step counter 混称 | 正：同 Run planned 恢复复用；反：remaining=0 再消费一次 | 首次冻结 `D10-r02`; `r04` 明确 same-Run continuation；无旧 alias |
| weftext.term.standing-approval | 持续批准 | Standing Approval | 对有限可机械判定的未来操作集合给予期限/次数/预算上限的预授权 | D10 approval control + proposed D6 consumption | 不是永久同意、Policy 或自然语言目标 | 受保护 `StandingApprovalEnvelope/1`；公共 `StandingApprovalSpec/1` 由 `automation_configure.standing` 承载，`approval_state` 投影当前状态；无独立 approval RPC | type `StandingApprovalEnvelope`; variable `standing_approval` | UI“持续批准”; locale `term.standingApproval` | 无 | 禁止 always allow/remember forever | 正：single_field_member envelope；反：整源自由写 | `D10-r01`; `r02` 收窄并配套 D6/D7；未激活 |
| weftext.term.approval-use | 批准使用记录 | Approval Use | 把准备请求、完整 preview、footprint 与一次批准 reservation/consumption 绑定 | D10 control + proposed D6 amendment | 不是 author plan、receipt 或第三 ledger | 现任内部类型 `ApprovalUse/2`（真实历史 /1 保留）；公共 IPC 未冻结 | type `ApprovalUse`; variable `approval_use` | 无直接 UI；可显示“本次使用持续批准”; locale `term.approvalUse` | 无 | 禁止 approved=true 客户端字段 | 正：reserved→consumed；反：TTL 自动释放 | 首次冻结 `D10-r01`; `r02` 增 released_terminal；未激活 |
| weftext.term.planned-decision-approval | 已计划决议交互批准 | Planned Decision Approval | 用户完整重读原 planned 保存 preview 后，对 exact 原 request 给出的有限一次性交互授权 | D10 control + proposed D6/D7 amendment | 不 reprepare、不换 OperationId、不复活冲突 plan | 受保护 `PlannedDecisionApproval/1`；公共 `ConsentSpec/1` 的 planned 分支承载原 request 与完整 preview 确认绑定；`planned_approval_state` 是受权脱敏当前投影；planned-preview 恢复运输归 D7 | type `PlannedDecisionApproval`; variable `planned_decision_approval` | UI“确认原计划”; locale `term.plannedDecisionApproval` | 无 | 禁止把 fresh prepare 当恢复 | 正：旧 token 过期后查原 preview；反：重新 Query 换 target | 首次冻结 `D10-r02`; 未激活，无 legacy alias |
| weftext.term.automation-definition | 自动化定义 | Automation Definition | 版本化的单 invocation 调度定义 | D10 scheduler control | 不是通用 DAG、脚本或作者 Document | `AutomationSpec/1` 是 `automation_configure` 的 closed public value；`core_field_member` arm 只承载 `FieldMemberTask/1` | type `AutomationDefinition`; variable `automation_definition` | UI“自动化” | Automation | 禁止 workflow/script/free callback | 正：具名 field-member task；反：ToolValue text 当 NodeRef | `D10-r01`; R08 冻结专用作者 invocation arm |
| weftext.term.automation-occurrence | 自动化发生项 | Automation Occurrence | 由 Automation 订阅代际与精确原发生 UTC 坐标确定的调度机会 | D10 scheduler control | 不是作者 recurrence identity 或新 Node | 公共 `run_state.origin` 承载闭合 `AutomationOccurrenceKey/1`；`automation_state` 显示订阅代际/原发生下界；`automation_configure` 使用 `AutomationScheduleUpdate/1`，准确 shape 归 D10-CONTROL §16 | type `AutomationOccurrenceKey`; variable `automation_occurrence_key` | 通常无独立 label；诊断“发生项”; locale `term.automationOccurrence` | occurrence（限定 Automation） | 禁止 recurrence Node/Run 同义 | 正：terminal K 重扫仍原 Run；反：enable 重建第二 Run | `D10-r01`; `r02` 补 terminal dedup |
| weftext.term.run | 运行 | Run | Agent 或 Automation 的受管执行记录与 step outcomes | D10 runtime control | 不是 author transaction 或 receipt | 受保护 Run 记录；公共 `ControlRef<run>/1`、带 `RunOrigin/1` 的 `run_state`、有限 state control body 与紧急停止 API 只公开各自闭合投影；不公开完整 Run 记录 | type `Run`; variable `run` | UI“运行”; locale `term.run` | Run | 禁止 job=author transaction | 正：cancelled Run 含已 committed step；反：Run completed=author committed | `D10-r01`; 无旧 alias |
| weftext.term.agent-session | Agent 会话 | Agent Session | Run 的交互 Agent 模式，绑定当前任务、ContextBundle 与工具 allowlist | D10 Agent runtime | 不是 principal、Document、长期授权 | 受保护 Run 的交互模式；公共 `run_state.origin` 使用 `RunOrigin/1` 的 interactive 分支；无独立 AgentSession wire 对象或 session RPC | type `AgentSession`; variable `agent_session` | UI“Agent 会话”; locale `term.agentSession` | Agent session | 禁止 chat=authority/session=grant | 正：一次交互 Run；反：会话自动继承全 Workspace | 首次冻结 `D10-r04` 映射；语义来自 r01 Candidate |
| weftext.term.context-bundle | 上下文包 | Context Bundle | 面向特定 recipient 的当前受权、最小化、版本绑定输入 | D10 context/egress control | 不是 author snapshot、授权 token 或 secret container | 公共 IPC 未冻结；受控概念 `ContextBundle` | type `ContextBundle`; variable `context_bundle` | 通常不直接显示；诊断“上下文”; locale `term.contextBundle` | 无 | 禁止 prompt=context authority | 正：Model A 的 bundle 不能改发 Model B；反：ambient workspace | `D10-r01`; 无 legacy alias |
| weftext.term.tool-value-profile | 工具值配置 | Tool Value Profile | D10 自有 closed ToolValueProfile/ToolType/ToolValue 代数 | D10 Tool Adapter | 不是任意 JSON Schema、D7 TypeSpec/TypedLiteral alias 或 D4 全值域 | 受控 `ToolValueProfile/1`、`ToolType/1`、`ToolValue/1`；公共 transport 仍由 adapter 决定 | types `ToolValueProfile`、`ToolType`、`ToolValue`; variables `tool_value_profile`、`tool_type`、`tool_value` | 无直接 UI；开发诊断“工具值”; locale `term.toolValueProfile` | ToolValue | 禁止 any/open map/float 或 D7 alias fallback | 正：ToolType 下 exact int64 ToolValue；反：JS double 或把 D7 TypeSpec 当同一 wire | `D10-r01`; R08 保持三个 D10 名称分开登记；无兼容 fallback |
| weftext.term.input-slot | 输入槽 | Input Slot | 单次 invocation 的 exact bytes/media/purpose/recipient 文件输入句柄 | D10 Tool/Model runtime | 不是 path、ResourceRef 或跨调用 file handle | 受控概念 `InputSlot`; 公共 IPC 未冻结 | type `InputSlot`; variable `input_slot` | 无直接 UI；可显示“输入文件”; locale `term.inputSlot` | 无 | 禁止 filepath/path handle | 正：exact attachment bytes；反：传 `C:\Users\...` | `D10-r01`; 无 legacy alias |
| weftext.term.tool-adapter | 工具适配器 | Tool Adapter | 把已接纳外部工具协议映射到 ToolValue 和 effect class | D10 external capability adapter | 不是 D7 author Action adapter | 公共包中 `Contribution/1.kind="tool"` 绑定已接纳 descriptor asset；D10 ToolValueProfile 拥有闭合值代数，已接纳外部 adapter 拥有其运输；Tool Adapter 不是 ContributionKind 字面量 | type `ToolAdapter`; variable `tool_adapter` | 管理 UI“工具适配器”; locale `term.toolAdapter` | tool adapter | 禁止 tool=permission | 正：本地接纳删除工具为 mutation；反：信任远端 readOnly | `D10-r01`; 无 legacy alias |
| weftext.term.mcp-adapter | MCP 适配器 | MCP Adapter | 使用 MCP 作为发现/运输协议的 Tool Adapter | D10 Tool Adapter | MCP metadata 不是 policy/identity/permission | 公共 IPC 使用外部 MCP；Weftext 受控类别 `MCP Adapter` | type `McpAdapter`; variable `mcp_adapter` | UI“MCP 适配器”; locale `term.mcpAdapter` | MCP | 禁止 MCP server=trusted principal | 正：schema drift pending；反：发现即热执行 | `D10-r01`; 无旧 alias |
| weftext.term.model-adapter | 模型适配器 | Model Adapter | 连接受管 ContextBundle 与模型 transport 的 adapter | D10 Agent runtime | 模型不是 author authority 或 secret store | 公共包中 `Contribution/1.kind="model"` 绑定已接纳 descriptor asset；已接纳 adapter 拥有模型运输；Model Adapter 不是 ContributionKind 字面量 | type `ModelAdapter`; variable `model_adapter` | UI“模型”; locale `term.modelAdapter` | model adapter | 禁止 model provider=author | 正：recipient-specific egress；反：换 provider 沿用 grant | `D10-r01`; 无 legacy alias |
| weftext.term.connector | 连接器 | Connector | 具名外部系统的协议、账户、cursor/version 与 sync control adapter | D10 connector control + D3/D9 mappings | 不是 author source、EntityRef 或通用 sync authority | 公共包中 `Contribution/1.kind="connector"` 绑定已接纳 descriptor asset；外部账户 control/current projection 使用其精确 ContributionBinding；Connector 是语义标签，不是新 RPC | type `Connector`; variable `connector` | UI“连接器”; locale `term.connector` | 无 | 禁止 provider/cursor=identity | 正：cursor 在 control state；反：cursor 写作者 YAML | `D10-r01`; 无 legacy alias |
| weftext.term.secret-reference | 凭据引用 | Secret Reference | 指向 OS/Server secret store 中 credential 的受管引用及 generation | D10 secret control | 不是 secret bytes、作者 source、ToolValue | 公共 `ControlRef<secret>/1` 与 `Binding<secret>/1` 定位受权凭据记录；`secret_state` 仅公开闭合脱敏元数据，`SecretStageTicket/1` 是受保护暂存引用；这些值不含凭据明文 | type `ControlRef<secret>` / `Binding<secret>`; variable `secret_ref`；SecretRef 仅为行文简称 | UI“凭据”; locale `term.secretReference` | SecretRef | 禁止 token/credential bytes 当引用 | 正：transport 注入；反：进入 prompt/log | `D10-r01`; 无 secret compatibility fallback |
| weftext.term.external-effect-intent | 外部效果意图 | External Effect Intent | 冻结 target/account/request/idempotency 的 immutable 外部 mutation 请求 | D10 external-effect control | 不是 D6 transaction、author receipt 或 rollback | 完整 `ExternalEffectIntent/1`、`ExternalExecutionBinding/1` 仅内部受保护；public current carrier 为 `ExternalEffectCurrentView/1` | types `ExternalEffectIntent`,`ExternalExecutionBinding`,`ExternalEffectCurrentView` | UI“外部操作” | effect intent | 禁止把 lifecycle revision 当 request identity | 正：prepared→submitting 保持 requestDigest；反：unknown 换 key 重发 | `D10-r01`; R08 闭合 internal/public projection |
| weftext.term.external-outcome-unknown | 外部结果未知 | External Outcome Unknown | 不能证明外部 mutation 是否发生或完整发生的持久结果状态 | D10 external-effect recovery | 不是 timeout、failed 或 cancelled | 受控状态 `outcome_unknown`; 外部 adapter 必须保留 | enum `ExternalOutcome::Unknown`; variable `outcome_unknown` | UI“结果未知”; locale `term.externalOutcomeUnknown` | unknown（仅外部效果上下文） | 禁止 timeout/failure/cancel 同义 | 正：send 后 response 丢失；反：自动当 failed 重试 | `D10-r01`; 无 legacy alias |
| weftext.term.cost-reservation | 费用预留 | Cost Reservation | 一个 billable attempt/account/grant/pricing/currency 的有限上限；多层 policy ceiling 在准入时检查但不是其它实际账户 | D10 cost control | 不是账单事实、approval count、effect idempotency 或单份多账户 reservation | D10-CONTROL §10：一份 `CostReservation/1`，`reserved→settled\|released\|uncertain`、`uncertain→settled\|released`；只有 settled/released 终态 | type `CostReservation`; variable `cost_reservation` | UI“费用预留”; locale `term.costReservation`（候选，未实现） | 无 | 禁止 multi-account reservation / released=zero bill / uncertain=terminal | 正：已发送零账单 `settled(0)`；反：一份 reservation 同时记两个账户 | `D10-r01`; R07 明确单 actual account；无 legacy alias |
| weftext.term.audit-started | 审计开始记录 | Audit Started Record | 受保护步骤真正执行前耐久写入的最小安全审计事实 | D10 audit control | 不是 success、receipt 或普通 telemetry | 公共 IPC 未冻结；受控事件语义 `audit_started` | type `AuditStartedRecord`; variable `audit_started` | 通常无直接 UI；审计查看器 label“开始”; locale `term.auditStarted` | 无 | 禁止 log line=authoritative audit | 正：本地 spool 后执行；反：collector offline 即假失败 | `D10-r01`; 无 legacy alias |
| weftext.term.money | 费用值 | Money | currency + Counter microUnits 的 exact 费用表示 | D10 cost value | 不做隐式 FX 或 binary float | closed public value `Money/1`；由 BudgetCaps、ResourcePermission、ControlPreview/History、reservation/current projection 承载；无独立 RPC | type `Money`; variable `money` | UI 按 currency 格式化 | 无 | 禁止 float money / implicit conversion | 正：public preview maximum；反：跨币种自动相加 | `D10-r01`; R08 明确 public value carrier |

| weftext.term.core-field-member-adapter | Core 字段成员适配器 | Core Field Member Adapter | 将一个受限 Automation task 确定映射到原 D7 set_field_member prepare/submit 的具名第一方 Core adapter | D10 automation/Core adapter | 不是通用 Tool callback、脚本或新的 D7 Action kind | descriptor 为 `CoreFieldMemberAdapterDescriptor/1`；accepted Contribution 为 `weftext.automation/set-field-member`；不新增独立 endpoint | types 为 `CoreFieldMemberAdapterDescriptor`,`FieldMemberTask` | 不新增独立 CLI | 字段成员适配器 | 禁止 ToolValue→Ref 强制转换或自由 callback | 正例：people/phone.label 的 Optional semantic-code task；反例：任意 source patch | 首次提议 D10-r08-joint-review-fixes-2026-09-28；待独立复核 |
| weftext.term.control-history-projection | 控制历史投影 | Control History Projection | 已保存 D10 control history 的受权 public 摘要，不重放嵌套作者请求 bytes | D10 control history | 不是完整 B、嵌套 A、生成 M 或第二 decision ledger | `ControlPreparedHistory/1`,`ControlPreviewSummary/1`,`ControlAffectedChange/1`,`ControlResourceUse/1` 由 `d10_control_result` 承载 | 对应 types | 仅诊断/history UI | 无 | 禁止 history 返回 A/B/M | 正：r6 current 下读 r5 history；反：重建 M | 首次提议 D10-r08-joint-review-fixes-2026-09-28；待独立复核 |
| weftext.term.emergency-stop-result | 紧急停止结果 | Emergency Stop Result | exact-target 不可逆 safety latch receipt 或读取时点已证明 open 的结果 | D10 safety control | 不是普通 control prepare、D6 author receipt、cancel 或 rollback | `d10_emergency_stop`,`d10_emergency_stop_result`；`D10EmergencyStopReceipt/1` 或 `d10_emergency_stop_not_applied`；current 用 `ExecutionStopLatchView/1` | stop result types | UI“已停止”/“本次检查时未停止” | stop result | 禁止 stop_address/prepared-control alias | 正：r5→r6 后重放丢失响应；反：探测 hidden target | 首次提议 D10-r08-joint-review-fixes-2026-09-28；待独立复核 |

### 13.1 继承名称与碰撞裁决

| 名称/概念 | 原 owner | D10 使用规则 | rejected collision / negative gate |
| --- | --- | --- | --- |
| `RegistrySnapshot/1`、`RegistryBinding/1`、Field/Facet | D4 | 逐字复用；D10 只认证来源并提供完整 binding | Capability Catalog 不得改名为 Registry，不保存第二份 FieldDefinition |
| `EntityRef`、`NodeRef`、`SourceBinding`、`OriginBinding`、`Provenance` | D3 | 逐字复用；控制 ID/provider ID 不进入这些类型 | `adoption_binding` 不得成为 OriginBinding alias；B10-01 见 amendment |
| `PrincipalContext`、`ObservationScope`、D6 Policy/error | D6 | D10 delegation 只收窄，不替换 | Agent permission / approval 不得覆盖 D6 deny |
| `ActionSpec`、fresh `PreparedActionBinding/4`、`EffectManifest/3`、`EffectBytes/3`（真实历史 PAB3/Manifest2/EffectBytes2 保留） | D7 | 原 prepare/preview/commit 依实际版本 | tool plan/AgentPlan 不是第二 Action wire |
| Draft / `PreparedEditBinding/2` | D8 | 人工编辑仍走 Draft/confirmation | model proposal 不得叫 Draft，除非真实进入 D8 Draft |
| Conversion Provider / Route / PublicationReceipt | D9 | 继续限定为 conversion/publication owner | D10 裸 Provider 不得覆盖 D9 Provider |
| author source / Provenance | D2/D3 | source 是作者 payload，Provenance 是非授权来源证据 | external input/provider state/tool result 不得叫 author source |
| D1 capability / unavailable reason | D1 | 产品可用性继续只用 D1 closed reasons 和固定优先级 | D10 dependency state 不得新增平行产品 availability reason |


### 13.2 追加继承 owner 门

| 名称/概念 | 原 owner | D10 消费规则 | 禁止碰撞 / 负向门 |
| --- | --- | --- | --- |
| `SearchContribution` / `weftext.term.search-contribution` | D7 Query Algebra / D7 terminology | D10 只认证纯数据 descriptor asset、namespace proof、activation generation、完整 Catalog 集和 Registry binding；D7 继续拥有 exact `{contributionId,version,fieldId,textPath,role}` 语义并执行 Query | D10 不得改名为 ViewSpec/search-provider、增加 script/network/read/write 权限或创建 alias 作者源 |
| `ImportJob`、`importJob`、`stageInput`、`planAtomicGroups`、`commitImportBatch` / `weftext.term.import-job` | D6 terminology registry | D9 `d9_import_*` 只消费 D6 durable job/control record；D10 最多承载所需 package/runtime dependency | D9/D10 不得把 ID、ownedNames、`storage.import_job` 或历史 firstFreeze 重新冻结为 D9 |

## 14. R05 控制、capability 与第一方 module 映射

本节补齐 D10-CONTROL 新增公共/内部受控概念的 Intake §8.5.1 映射。候选 code namespace 与 locale key 是设计映射，不是实现证据；标为“无公共 IPC”的内部记录不能由普通 caller 自由构造。

### 14.1 控制记录与恢复概念

| stable concept ID | 中文正式名 | English formal name | 精确定义 | owner/layer | 排除边界 | canonical wire/API/manifest/schema | code symbol convention | CLI/UI label + locale | 允许简称 | 禁止/历史 alias | 正例 / 反例 | 首次冻结、状态、迁移 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| weftext.term.control-prepare-binding | 控制准备绑定 | Control Prepare Binding | stable key 到完整 canonical intent、分配 ID、原 commit request、preview 和 dependency pins 的耐久准备绑定 | D10 control | 不是 author decision、receipt 或授权 token | fresh internal ControlPrepareBinding/3（历史 /2 原 decoder）；public prepare/result 由 D10-CONTROL §7–§8 | type ControlPrepareBinding；variable control_prepare_binding；namespace d10::control | 无普通 CLI；诊断“控制准备”；candidate locale term.controlPrepareBinding，未实现 | 无 | 禁止 request cache、second ledger | 正：响应丢失后取回同 prepare；反：同 key 新分配 OperationId | R05 candidate；无 legacy alias |
| weftext.term.interactive-run-start | 交互运行启动 | Interactive Run Start | 认证本人到场 Core 原子创建 Run/Run-target Lease/Stop | D10 Core runtime | 非第八 ControlBody/作者 P | `d10_interactive_run_start`、`d10_interactive_run_result`、`D10InteractiveRunStarted/1` | D10InteractiveRunStartRequest / interactive_run_start | attended Desktop/CLI/Server；locale `term.interactiveRunStart` 待实现 | 无 | 禁止 trusted_creator/approved=true | maxRuns1 无 Automation 原结果恢复；伪 principal 拒绝 | 当前 A2 候选；产品 UNRUN |
| weftext.term.deployment-control-policy | 部署控制策略 | Deployment Control Policy | 一个 store incarnation 的 H 管理员和具名 cost reconciler 权威 | D10 host control | 不是 D6 Workspace Policy、IssuerControlPolicy 或 Workspace grant | DeploymentControlPolicy/1；无作者 wire | type DeploymentControlPolicy；variable deployment_control_policy；namespace d10::control | 管理诊断“部署控制策略”；candidate locale term.deploymentControlPolicy，未实现 | 无 | 禁止 first-login-admin、workspace-admin alias | 正：部署 operator 显式建立；反：首个 HTTP 抢占 | R05 candidate；无 legacy alias |
| weftext.term.resource-use-grant | 资源使用授权 | Resource Use Grant | 对 cost/secret/egress/external-effect 之一的有限使用授权及累计 usage | D10 host/control | 不是资源管理权、作者权限或预付余额 | ResourceUseGrant/1 + ResourcePermission/1；D10-CONTROL §6 | type ResourceUseGrant；variable resource_use_grant；namespace d10::control | UI“资源使用授权”；candidate locale term.resourceUseGrant，未实现 | grant（D10 限定） | 禁止 whole-package grant、grant renewal resets quota | 正：用户可用 D 的有限费用份额；反：grant holder 改 D ceiling | R05 candidate；无 legacy alias |
| weftext.term.deployment-control-decision | 部署控制决议 | Deployment Control Decision | 同一 host control stable key 的不可变 applied outcome | D10 host control | 不是 D6 author receipt 或第二 author ledger | internal DeploymentControlDecision/1；由 host commit/result 消费 | type DeploymentControlDecision；variable deployment_control_decision；namespace d10::control | 无普通 UI；诊断“部署控制决议”；candidate locale term.deploymentControlDecision，未实现 | 无 | 禁止 author receipt | 正：lost response 重放；反：重放再次改账户 | R05 candidate；无 legacy alias |
| weftext.term.cost-settlement-decision | 费用结算决议 | Cost Settlement Decision | 证据绑定、追加式地把一个 reservation 结算为 settled(actual) 或 released | D10 cost control | 不是人工账务 adjustment、作者 ledger、effect receipt | internal CostSettlementDecision/1；D10-CONTROL §10 | type CostSettlementDecision；variable cost_settlement_decision；namespace d10::cost | UI“费用结算”；candidate locale term.costSettlementDecision，未实现 | 无 | 禁止 manual release、clear uncertain | 正：100→uncertain→bill20→settled20；反：管理员填0 | R05 candidate；无 legacy alias |
| weftext.term.secret-stage-ticket | 凭据暂存票据 | Secret Stage Ticket | trusted secret channel 对 immutable secret version 的账户/audience/use 绑定引用 | D10 secret control | 不是 secret bytes、普通 ToolValue 或 credential alias | SecretStageTicket/1；只由 d10_secret_stage 成功产生 | type SecretStageTicket；variable secret_stage_ticket；namespace d10::secret | 无普通 UI；诊断“凭据暂存”；candidate locale term.secretStageTicket，未实现 | 无 | 禁止 plaintext digest ticket | 正：同 secret stable retry 取回同 ticket；反：换 audience 复用 | R05 candidate；无 legacy alias |
| weftext.term.execution-stop-latch | 执行停止闩锁 | Execution Stop Latch | 与精确 automation/run incarnation 绑定的单调 open→stopped 安全控制事实 | D10 safety control；D6 final commit 消费 | 不是 rollback、cancel receipt、费用释放或永久删除 | internal ExecutionStopLatch/1；public d10_emergency_stop | type ExecutionStopLatch；variable execution_stop_latch；namespace d10::control | UI“紧急停止”；candidate locale term.executionStop，未实现 | stop（安全控制上下文） | 禁止 clear-stop、rollback alias | 正：预算耗尽仍能 stop；反：stop 回滚已 sent effect | R05 candidate；无 clear/migration alias |

### 14.2 四个第一方 module 的完整映射

共同规则：下列 PackageId 是 D10 package identity，不是 D4 SemanticNamespaceId；module/schema 是 package-local ContributionId。code/locale 是候选映射、未实现。没有独立 CLI verb；通用管理入口使用完整 PackageId+ContributionId。

| stable concept ID | 中文 / English | 定义 | owner/layer | 排除 | wire/manifest/schema | code convention | UI/locale | 简称 | 禁止/历史 alias | 正反例 | 首次冻结/状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `weftext.term.calendar-module` | 日历 / Calendar | 第一方日历产品组织，复用既有时间与日历语义，并提供两个子扩展点 | D10 package/product mapping；D4 semantic owner 保持 `calendar→first_party,weftext.calendar` | 非 Node kind、Facet 或 D4 namespace identity | PackageId `weftext.calendar`；module contribution `module`；schema contribution `schema`→`calendar/period-note`,`calendar/range-note`,`calendar/event`,`semanticMajor=1`；extension points `calendar-system`,`holiday-schedule` | `CalendarModuleContribution`；`calendar_module`；candidate namespace `d10::bundled::calendar` | label 日历/Calendar；candidate locale `module.calendar.title`，未实现 | Calendar | 禁止 Time/Diary/Chrono 作为 wire/package alias | 正：隐藏 module UI 不删除作者事实；反：package update 改写同 Facet digest | R05 candidate；无 runtime legacy ID |
| `weftext.term.library-module` | 文献库 / Library | 第一方文献作品产品组织 | D10 package/product mapping；D4 semantic owner 保持 `library→first_party,weftext.library` | 非文件库、Workspace 或 Work identity | PackageId `weftext.library`；module=`module`；schema=`schema`→`library/work`,`semanticMajor=1`；本代无 extension point | `LibraryModuleContribution`；`library_module`；candidate namespace `d10::bundled::library` | label 文献库/Library；candidate locale `module.library.title`，未实现 | Library | 禁止 References/Bibliography/Literature/Works 作 wire alias | 正：My Works 是派生 View；反：Reference 变成实体 owner | R05 candidate；无 runtime legacy ID |
| `weftext.term.people-module` | 人物 / People | 第一方人物交互与产品组织 | D10 package/product mapping；D4 semantic owner 保持 `people→first_party,weftext.people` | 非 Person Node identity 或 namespace owner | PackageId `weftext.people`；module=`module`；schema=`schema`→`people/person`,`semanticMajor=1`；本代无 extension point | `PeopleModuleContribution`；`people_module`；candidate namespace `d10::bundled::people` | label 人物/People；candidate locale `module.people.title`，未实现 | People | 禁止 PersonModuleId 冒充 D4 owner | 正：provider UI 不存在时仍保留姓名事实；反：停用 module 删除 Person facts | R05 candidate；无 runtime legacy ID |
| `weftext.term.organizations-module` | 组织 / Organizations | 第一方组织交互与产品组织，并提供子 schema-pack 扩展点 | D10 package/product mapping；D4 semantic owner 保持 `organizations→first_party,weftext.organizations` | 非 Organization Node identity 或国家 ontology | PackageId `weftext.organizations`；module=`module`；schema=`schema`→`organizations/organization`,`semanticMajor=1`；extension point `schema-pack` | `OrganizationsModuleContribution`；`organizations_module`；candidate namespace `d10::bundled::organizations` | label 组织/Organizations；candidate locale `module.organizations.title`，未实现 | Organizations | 禁止 Business 独立 module/package owner；禁止国家 pack 冒充全局 enum | 正：country pack inactive 时仍保留 accepted schema/raw；反：停用 module 删除 author facts | R05 candidate；无 runtime legacy ID |

### 14.3 D10 定义、D1 发现的 capability IDs

| stable concept ID | 中文正式名 | English formal name | 定义 | owner/layer | 排除 | canonical D1 capability ID | code convention | UI/locale | 简称 | 禁止 alias | 正反例 | 首次冻结/状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| weftext.term.automation-manage-capability | 自动化管理能力 | Automation Management Capability | 自助/受权管理有限 Automation control records | D10 semantics；D1 discovery/availability | 不授作者写或 deployment manage | automation.manage | AutomationManageCapability；automation_manage | 候选 label“自动化管理”；locale 未实现 | 无 | 禁止 design-accepted=available | 正：release+surface+policy 全过；反：Mobile 强行显示 | R05 candidate，尚未产品发布 |
| weftext.term.workspace-extensions-manage-capability | 工作区扩展管理能力 | Workspace Extensions Management Capability | 管理 Workspace activation/package contribution selection | D10 semantics；D1 discovery/availability | 不授 deployment trust 或 Registry 内容写权 | workspace.extensions.manage | WorkspaceExtensionsManageCapability；workspace_extensions_manage | 候选 label“工作区扩展”；locale 未实现 | 无 | 禁止 policy_admin 自动等于 runtime available | 正：W+registry_admin 激活；反：隐藏 UI 改 capability | R05 candidate，尚未产品发布 |
| `weftext.term.deployment-external-manage-capability` | 部署外部能力管理 | Deployment External Management Capability | 管理 host trust、account、secret、pricing 和 grant | D10 host；D1 discovery/availability | 不授 Workspace author read/write | `deployment.external.manage` | `DeploymentExternalManageCapability`；`deployment_external_manage` | 管理 label 候选；locale 未实现 | 无 | 禁止 issuer admin alias | 正：H 管理 secret；反：Workspace admin 改 pricing | R05 candidate，尚未产品发布 |
| weftext.term.automation-stop-capability | 自动化停止能力 | Automation Stop Capability | 对获权 automation/run 设置不可逆 stop latch | D10 safety；D1 discovery/availability | 非 rollback、cancel 或 settlement | automation.stop | AutomationStopCapability；automation_stop | label“紧急停止”；locale 未实现 | stop | 禁止 stop=rollback | 正：普通预算耗尽仍 stop；反：stop 删除 evidence | R05 candidate，尚未产品发布 |
| weftext.term.automation-author-submit-capability | 自动化作者提交能力 | Automation Author Submit Capability | 首版只允许 single_field_member Standing Approval 路径进入 D6 author submit | D10 profile semantics + D6 submit；D1 discovery/availability | 非通用 author-write、D8 Draft 或 create/delete | automation.author_submit | AutomationAuthorSubmitCapability；automation_author_submit | 候选 label“自动提交单字段值”；locale 未实现 | 无 | 禁止旧匿名 capability profile | 正：D1 门全过且 D6 closed error 已实现；反：仅设计接受即 available | R05 candidate，尚未产品发布 |

## 15. R06 上游 owner 术语引用

R06 不把 D8/D9 名称重新登记到 D10 owner。D10-UPSTREAM §8 是 D8/D9 原 owner 的配套词表提案；本节只固定 D10 消费时的反向引用。

D8 owner 概念恰为：

```text
weftext.term.edit_session
weftext.term.edit_draft
weftext.term.draft_projection
weftext.term.draft_edit_map
weftext.term.prepared_edit_binding
weftext.term.composition_transaction
weftext.term.caret_affinity
weftext.term.layout_epoch
weftext.term.direction_preference
```

D10 文档中的 Draft 必须明确指 D8 `weftext.term.edit_draft`；Prepared Edit Binding 指 D8 `weftext.term.prepared_edit_binding`；Direction Preference 指 D8 `weftext.term.direction_preference`。D8 的 13 个 kind 归属由 D10-UPSTREAM §8.1 owner 表决定，D10 不建立 alias。

D9 owner 新增/拆分概念恰为；`weftext.term.import-job` 明确排除，因为 D6 已拥有它：

```text
weftext.term.source-artifact
weftext.term.import-ir
weftext.term.source-location
weftext.term.import-observation
weftext.term.conversion-provider
weftext.term.conversion-route
weftext.term.import-mapping
weftext.term.mapping-proposal
weftext.term.coupling-group
weftext.term.import-batch
weftext.term.conversion-input
weftext.term.worker-invocation
weftext.term.template-recipe
weftext.term.template-construction-input
weftext.term.office-template
weftext.term.template-placeholder
weftext.term.style-directive
weftext.term.repeat-band
weftext.term.render-snapshot
weftext.term.d7-result-pin
weftext.term.export-plan
weftext.term.export-input-catalog
weftext.term.export-content-selection
weftext.term.export-projection
weftext.term.export-input-location
weftext.term.export-loss-location
weftext.term.export-loss-report
weftext.term.export-blocks
weftext.term.staged-output
weftext.term.publication-receipt
weftext.term.import-loss-report
weftext.term.import-loss-issue
weftext.term.template-loss-location
weftext.term.image-physical-size
weftext.term.image-size-selection
weftext.term.region-body
```

其中 D7 TerminalSchema/V、现任 `PreparedActionBinding/4`（实际历史 /3 保留），D3 `SourceBinding`/`ForeignIdentityKey`/`OriginBinding`/`ResourceRegionLocator/l1`，D2 Template meta-kind 以及 D3/D6 author receipt 都继续由原 owner 冻结。特别是 `weftext.term.publication-receipt` 只表示 D9 外部发布事实，绝不成为作者回执别名；`weftext.term.d7-result-pin` 只固定 D7 已有结果，不取得其 schema/value owner。

调度辅助类型归既有自动化发生项/定义概念：SourceOccurrenceKey/1、AutomationScheduleUpdate/1、ScheduleSelection/1 是 §16 明确公共 carrier 使用的闭合值；ScheduleSubscription/1、ScheduleSourceBinding/1、ScheduleRecurrenceEvidence/1、ScheduleOccurrenceProof/1、AutomationOccurrenceRecord/1 和 AutomationOccurrenceDisposition/1 是受保护内部类型，不新增用户控制项、CLI 或 locale。subscriptionGeneration 是同一 Automation 内的正 Counter；definitionRevision 只选择不可变执行定义，两者不能混称，horizon/Query K/source token 均非业务身份。armed 是未产生 Run/LeaseRunUse 的耐久前态，不是 queued Run；claimed/skipped 是不可逆已处理责任。真实旧记录保持原 decoder，不把候选旧 tuple 伪造为已部署兼容格式。
