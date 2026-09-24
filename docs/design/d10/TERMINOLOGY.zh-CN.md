---
source_language: zh-CN
translation_status: source
---

[English](TERMINOLOGY.md)

# D10 术语与命名

revision: D10-r01-candidate-2026-09-25；状态：candidate。本词表只定义 D10 新增控制/扩展概念和 D1–D9 的消费边界，不重新分配任何上游 owned concept。自然语言、历史材料、测试描述和第三方产品用词不自动成为受控名称。

## 1. 命名原则

- 作者内容、身份、定位、来源、Field、Facet、Query、Action、Draft、conversion、publication 等词继续由其原阶段拥有。
- D10 控制身份不能借用 D3 EntityRef/Locator 命名；局部 ordinal、token、ID 也不能被描述为内容身份。
- “plugin”只作为用户/历史泛称，不作为 canonical wire family。受控文档应区分 Extension Package、Contribution、Connector、Tool Adapter、Model Adapter、Pack 与 Bundled Module。
- 裸 “Provider” 易与 D9 Conversion Provider 冲突；D10 必须写成 Model Provider、External Service、Connector Provider 或直接写 Contribution/Adapter。
- “Registry”在 D10 正文默认指 D4 semantic Registry；D10 自身使用 Capability Catalog，不另建“plugin registry”。
- “approval”“authorization”“delegation”“confirmation”分域：D6 Policy 是工作区授权；Delegation Lease 只能减权；Standing Approval 是有限预授权；D8 confirmation 是当前完整预览的人机确认。
- “source”必须加限定：author source、external input、tool result、provider state、package asset 或 source binding。Provider state 不是 author source。

## 2. D10 新概念

| conceptId | 中文 / English | owner | 定义 | 明确排除 |
| --- | --- | --- | --- | --- |
| weftext.term.extension-package | 扩展包 / Extension Package | D10 | 具名版本、内容摘要、publisher、依赖和贡献目录组成的分发/升级单元 | 不是权限单元、namespace owner、作者对象 |
| weftext.term.contribution | 贡献项 / Contribution | D10 | package 中可单独接纳、启停和授权的一个纯数据或可执行能力 | 不等于整个 package 的权限 |
| weftext.term.capability-catalog | 能力目录 / Capability Catalog | D10 | 当前激活部署中的 contribution/runtime 可用性和 host privilege 描述 | 不是 D4 Registry、D1 capability response 或作者 schema |
| weftext.term.activation-binding | 激活绑定 / Activation Binding | D10 | 将 current D4 RegistryBinding、Capability Catalog digest 与 trust revision 固定为一个受管代际的记录 | 不是 D4 semantic generation 的替代 |
| weftext.term.publisher-identity | 发布者身份 / Publisher Identity | D10 | 由接纳 trust root 证明的 package 签名主体 | 不是 namespace ownership 本身 |
| weftext.term.namespace-claim | 命名空间所有权声明 / Namespace Claim | D10→D4 proof | 把 publisher identity 与一个允许的 D4 publisher namespace 绑定的受管声明 | 安装顺序、display name、enablement 均不是 claim |
| weftext.term.delegation-lease | 委托租约 / Delegation Lease | D10 | 对当前 D6 principal 权限作进一步有限收窄的任务/运行授权边界 | 不增加 D6 capability，不可多级转授 |
| weftext.term.standing-approval | 持续批准 / Standing Approval | D10 | 对有限、可机械判定的未来操作集合给予有期限/次数/预算上限的批准 | 不是永久同意、自然语言目标或 D6 Policy |
| weftext.term.approval-use | 批准使用记录 / Approval Use | D10 + proposed D6 amendment | 把一份已准备请求、完整预览、实际 footprint 与一个批准的单次消费/预留绑定 | 不是 author plan、receipt 或第三 ledger |
| weftext.term.automation-definition | 自动化定义 / Automation Definition | D10 | 一个受管、版本化的单 invocation 调度定义 | 不是通用 workflow DAG 或作者 Document |
| weftext.term.automation-occurrence | 自动化发生项 / Automation Occurrence | D10 | 由 definition revision 与源 occurrence 机械产生的一次调度机会 | 不等于作者 recurrence occurrence identity |
| weftext.term.run | 运行 / Run | D10 | Agent 或 Automation 的受管执行记录及 step outcomes | Run completed 不等于 author committed |
| weftext.term.context-bundle | 上下文包 / Context Bundle | D10 | 当前受权、最小化、版本绑定、面向特定 recipient 的模型/工具上下文 | 不是 author snapshot 或可转授权 token |
| weftext.term.tool-value-profile | 工具值配置 / Tool Value Profile | D10 | 复用 D7 TypeSpec/V 有限子集的工具参数/结果代数 | 不是任意 JSON Schema、D4 Field value 全集 |
| weftext.term.input-slot | 输入槽 / Input Slot | D10 | 单次 invocation 使用、绑定 exact bytes/用途/recipient 的受限文件输入句柄 | 不是路径、ResourceRef 或跨调用 file handle |
| weftext.term.tool-adapter | 工具适配器 / Tool Adapter | D10 | 把一个已接纳外部 tool protocol 映射到 Tool Value 与明确 effect class 的 adapter | 不是作者 Action adapter |
| weftext.term.mcp-adapter | MCP 适配器 / MCP Adapter | D10 | Tool Adapter 使用 MCP 作为运输/发现协议的具体类型 | MCP annotations/prompts 不构成 Weftext 权限 |
| weftext.term.model-adapter | 模型适配器 / Model Adapter | D10 | 将受管 ContextBundle 与模型 API/本地模型 transport 连接的 adapter | 模型输出不是 author authority |
| weftext.term.connector | 连接器 / Connector | D10 | 具名外部系统的协议、账户、cursor/version 与 sync control adapter | provider ID/cursor 不是 EntityRef |
| weftext.term.secret-reference | 凭据引用 / Secret Reference | D10 | 对 OS/Server secret store 中实际 credential 的受管引用及 generation | 不是 secret bytes、author source 或 model input |
| weftext.term.external-effect-intent | 外部效果意图 / External Effect Intent | D10 | 已冻结 target/account/request/idempotency/approval/budget 的一次外部 mutation 请求 | 不是 D6 transaction 或 rollback |
| weftext.term.external-outcome-unknown | 外部结果未知 / External Outcome Unknown | D10 | 无法证明外部 mutation 是否发生或是否完整发生的持久状态 | 不是 failed 或 cancelled |
| weftext.term.cost-reservation | 费用预留 / Cost Reservation | D10 | 在可能收费的 attempt 前对一个或多个费用账户原子占用的有限上限 | 不是账单事实或 effect idempotency |
| weftext.term.audit-started | 审计开始记录 / Audit Started Record | D10 | protected step 真正执行前耐久写入的最小安全审计事实 | 不代表 step 已成功 |
| weftext.term.money | 费用值 / Money | D10 | currency + Counter microUnits 的 exact cost 表示 | 不做隐式 FX，不使用 binary float |

## 3. 精确受控名称

下列类型/记录名由 D10 candidate 冻结为受控 spelling；最终 public wire 是否暴露由实施接口阶段决定，但同一概念不得同时创造近义类型：

- `ActivationBinding/1`
- `DelegationLease/1`
- `StandingApprovalEnvelope/1`
- `ApprovalUse/1`
- `ToolValueProfile/1`
- `InputSlot`
- `AutomationOccurrenceKey/1`
- `ExternalEffectIntent/1`
- `Money/1`

现有上游名称按原 owner 原样消费：

- `RegistrySnapshot/1`
- `RegistryBinding/1`
- `PrincipalContext`
- `ObservationScope`
- `PreparedIntent`
- `ActionSpec`
- `PreparedActionBinding/2`
- `EffectManifest/1`
- `PreparedEditBinding/1`
- `SourceBinding`
- `OriginBinding`
- `Provenance`
- `SourceVersion`
- `OperationId`

D10 不定义 alias 去替代这些名称。

## 4. Package、module、pack 与 plugin

**Extension Package**是安装/升级/签名单位，可以包含一个或多个 Contribution。权限和运行可用性逐 Contribution 计算，拒绝某个网络 Connector 不得自动禁用同 package 的纯数据 contribution。

**Bundled Module**表示第一方产品组织/UI 形态，例如 Calendar/People/Library 等 D1 已有模块方向；module 不成为新的 namespace owner class 或权限域。

**Pack**只用于没有任意 code/process/network/secret 能力的声明式语义/数据包。一个需要执行代码或网络的“pack”必须在受控面重新分类为 executable Contribution/Connector，而不是靠名称规避 runtime gate。

**plugin**仅为用户可理解的泛称或历史文本。controlled schema、CLI/API、测试 fixture 和候选设计不得用 plugin 替代具体类别。

## 5. Registry、Catalog 与 capability

**D4 Registry**持有 semantic namespace、Field/Facet/Relation/Calendar/unit 等作者语义，并按 D4 累计 evolution 规则演进。

**Capability Catalog**持有 contribution/runtime 的当前部署描述；它不能定义 Field、Facet 或关系语义。Catalog change 与 Registry change 可由同一个 Activation Binding 协调，但两者不是同一对象。

**D1 capability**描述某个正式产品表面在当前 release/current subject 下是否可提供一个产品能力，并使用 D1 固定 unavailable reasons。D10 Contribution availability 只是 D1 判断的一项下游事实，不能改变 D1 reason precedence。

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

author source 是 D2/D3/D6 认可的作者 payload；external input、ContextBundle、tool result、model output、package asset、transcript、connector cache 与 provider response 都不是 author source。

D3 Provenance 只是来源证据，不授予 authority。D10 package/provider origin 同样不能转化为写权限。SourceBinding/OriginBinding 是 D3 绑定语义；Connector 自己的 cursor/etag 不得借名“binding”后绕过它们。

## 8. Approval、confirmation、authorization 与 delegation

**authorization**：当前 D6 Policy、ObservationScope、authority/cut 与适用上游权限门共同给出的当前工作区资格。

**delegation**：Delegation Lease 对当前 authorization 的进一步收窄；它不是授权来源。

**interactive confirmation**：当前完整 preview 后由用户明确确认原 request；D8/D7 保持默认路径。

**Standing Approval**：仅对候选规定的机械 envelope 预先允许未来相同类型操作，并仍需 fresh prepare/preview/current authorization。

**ApprovalUse**：一份已准备 request 实际消费 Standing Approval 的受管绑定；不能由客户端自报。

**approval-required**：D10 控制层判断不能机械消费 Standing Approval、必须进入交互确认的状态；不能替代原 D6/D7 semantic errors。

## 9. Secret、context、egress 与 external effect

Secret Reference 只指 secret store 中的 credential；secret bytes 不进入 ToolValue、ContextBundle 或普通日志。

Context Bundle 是一次模型/工具调用的实际受权上下文。持有 ContextBundle 不授新的 read，也不允许换 recipient。

egress 指数据离开当前受信 Core/host boundary 到具名 model/tool/connector/external service；network access 只是 transport 能力，不能自动授权任意 egress recipient。

external side effect 指改变外部系统状态的请求。读取外部数据和写外部数据使用不同 effect class；“read-only tool”只在 Weftext 本地接纳 contract 下成立，不能信任远端 self-annotation。

External Outcome Unknown 表示已无法证明 external request 的结果；不能称 timeout、failure 或 cancellation。

## 10. Audit、transcript、log 与 evidence

Audit 是恢复与安全所需的受保护事实；Transcript 是可选的人机/model 对话记录；ordinary log/telemetry 是运行观测。三者保留、权限与导出规则不同。

Audit Started Record 只证明 protected step 已进入执行边界，不证明其成功。真正 author 成功仍只看 D3/D6 receipt；external 成功看具名 adapter 规定的完整 success evidence。

测试 report、CI log、model witness、screenshot 和 protocol trace 都是 evidence，不是 authority。候选必须标出它们证明的有限范围。

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
