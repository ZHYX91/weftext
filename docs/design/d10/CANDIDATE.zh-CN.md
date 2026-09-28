---
source_language: zh-CN
translation_status: source
---

[English](CANDIDATE.md)

# D10 Agent、自动化与外部能力候选

revision: D10-r08-joint-review-fixes-2026-09-28；状态：固定 R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` 对 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` 的完整独立联合终审已完成 C18/18、S49/49，结论 REVISE：P0=0、P1=1（B03-P1-01）、P2=10，整体、术语及中英语义均需修订。R08 是作者统一修订，等待新的完整独立复核；不表示已独立接受、实现、发布、激活、合并或可以开始 A2。

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
3. 网页、Document、工具结果、MCP 描述、模型输出、模板文字和外部提供方响应都属于不可信数据；它们不能扩大 principal/delegation、工具 allowlist、出站 recipient、网络/文件/进程、secret、预算或 approval。
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

所有影响工作区操作资格的 durable record 必须通过 Core 管理的 closed adapter 修改。普通 management write 使用 CONTROL-CONTRACT §7 prepare/commit；不可逆 emergency stop 是明确的 safety 例外：CONTROL-CONTRACT §11 在同一 managed Authority Store 中定义预留 latch/result/sequence capacity 的专用 closed write，它不是普通 prepare，也不是 D6 author transaction。普通运行 telemetry 可以独立运输，但不得被恢复器当作批准、授权、费用、stop 或效果事实。

R08 的管理授权、closed body、stable request key、prepare/result/current/secret/stop 入口、具名 Core field-member author adapter、ResourceUseGrant、immutable external-effect request/send binding、部署控制决议、费用结算、stop safety 线性化和 package manifest 的**精确规范**统一由 [CONTROL-CONTRACT](CONTROL-CONTRACT.zh-CN.md) 拥有。该文档是本候选的一部分，不是实现笔记。Workspace author-affecting control write 仍进入原 D6 authority/decision transaction；deployment host control 只能改变部署控制记录，不能经 host adapter 写作者 payload 或伪造 D6 receipt。

## 6. Registry、Catalog 与激活

D4 Registry 仍是唯一 semantic namespace/schema 权威。D10 证明 package/publisher/namespace claim、安装资产与贡献来源，随后把完整候选 `RegistrySnapshot/1`、`RegistryBinding/1` 交给 D4 既有验证、演进和 catalog load；D10 不增加 Registry member，也不改变 D4 Field/Facet/Relation/Calendar/Unit 语义。

D10 另维护 Capability Catalog，记录每个可执行或纯数据 Contribution 的精确包摘要/版本、贡献类型/版本、运行 profile、平台/架构、依赖、宿主权限、网络/出站类别、secret 要求、费用 profile 与 D1 capability ID。Catalog 不能保存第二份 FieldDefinition，也不能用显示名称或安装顺序证明命名空间 owner。

R08 保留 R07 对 D7 SearchContribution 补齐真正的正向 Catalog 路径，同时不增加 D4 Registry member 或第二搜索引擎。CONTROL-CONTRACT §4 唯一冻结 D10 `view` kind 的纯数据 carrier、不可变 descriptor asset digest、D4 owner/Field/textPath 验证、D7/D10 两层 ID 与版本分域、同 cut 完整 SearchContribution 集合及 `activationGeneration + capabilityCatalogDigest + registryBinding` 失效语义。合法第一方 SearchContribution 因此可安装/激活并进入原 D7 search Query builder；未接纳/runtime-only descriptor 只能 pending，selected contribution unavailable 仍按原 D7 失败而不能静默跳过。

R05 将 package/contribution wire 收敛到 CONTROL-CONTRACT §4–§5：dependency 是**具体 dependent Contribution 自身的成员**，不是 package 级 ambient 数组；一个 connector 不可用不能停用同包无关 schema/template/data pack。Contribution kind 闭集显式保留 module/schema/view/action/template/preset/pack/tool/model/connector/importer/exporter/conversion/renderer/localization。Calendar、Library、People、Organizations 各有唯一第一方 PackageId→module contribution→schema contribution 映射，且明确引用 D4 已冻结 namespace owner 与 FacetId；这些 D10 PackageId 不成为第二 D4 namespace，也不冒称 D1 已存在 module ID、代码路径或 locale 资源。

补充 S 的 D4 reference catalog 是当前候选的第 49 份输入：61 个 Field、7 个 Facet、22 个 value-type alias、4 个 qualifier set 和 1 个 Calendar series policy。它不改变既有 D4/D7 语义，只把先前遗漏的规范机器图纳入实际输入。对 `single_field_member` 尤其重要的是：实际目录有 27 个 relation Field，`calendar/event` 还有 `calendar/range` 与 `calendar/recurrence` 的跨 Field `union_variant_equal`，另有 required-field 与 Field 内部约束；因此自动路径必须继续执行 D7 Narrow Field Qualification 的完整 Registry 图证明，不能从“只修改一个 Field”直接推出安全。正向 `people/phone` 在目录中是普通非 relation Field，位于无额外 Facet constraint 的 `people/person`，自身 constraints 为空，但其 alias、qualifier、source-envelope 和其它 D7 依赖仍必须完整验证。

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

active-selector record 才是本次切换的 CAS 对象。current-read projection 中的 `activation.current` 在同一受权 cut 由 selector 派生；旧 binding 变成 non-current 不会改写历史 ActivationBinding 的 generation/revision。

暂时 offline/health failure 不产生新 semantic generation。真正 package、definition、runtime contract 或 trust 变化才产生后继 binding。未激活升级失败可丢弃 staging；已激活后的“回滚”必须是后继激活，不能把 D4 semantic ledger 指针倒退或删除中间历史。

disable/uninstall 可使 executable contribution unavailable，也可使需要该 provider 才能证明的定义进入原 D4 unavailable 分支，但必须保留作者 raw source、累计 tombstone/migration 与必要的已接纳语义证据；不能把 unavailable 解释为空集合或删除事实。

### 6.1 Pack 父领域、extension point 与生命周期

每个 domain Pack Contribution 必须声明**恰一个 primary parent domain/extension point**与 required compatible version range；跨域附加依赖继续逐项显式声明，不能用“安装了同一 package”或 UI 位置推断。候选 Catalog dependency 语义固定为 `parentDomainId,extensionPointId,requiredVersionRange,resolvedParentVersion,resolutionState`，但本代不因此冻结新的公共 IPC。构造 ActivationBinding 时必须把 dependency 解析到当前准确 parent extension-point version，解析结果进入 Capability Catalog digest；parent version/binding 改变会使旧 dependent binding 失效，必须经过 successor activation，不能在运行中跟随“latest”。

package **安装/验证**与 contribution **激活**分开。父 domain 缺失时 package 仍可作为已验证资产存在，用户配置、来源、签名、历史 binding 和可恢复选择保留；dependent domain contribution 不激活。父 extension point 被明确停用时同样不激活。版本范围不兼容时不能降级到旧规则。仅仅隐藏父模块 UI、关闭某个导航入口或不显示设置页，若 parent semantic/capability 仍 active compatible，则**不改变 dependency state**，不能用 UI visibility 偷偷停用或启用 contribution。

内部 dependency resolution 只允许 `ready|missing|disabled|incompatible|unsupported_surface` 五类结果。它不是第二套产品 availability reason；对外 capability 仍按 D1 固定优先级裁决。在更高优先级 reason 不成立且该 dependency 是实际阻塞原因时：surface 不支持→`unsupported_surface`，parent component 缺失→`missing_component`，parent 已存在但显式停用→`not_configured`，parent version 不兼容→`incompatible_version`。policy denial 必须更早遮蔽这些部署细节；offline/temporary failure 仍沿 D1 原定义。UI hidden-only 没有 unavailable reason，因为它不是 capability 状态。

D4 语义保留与 D10 激活是不同轴。已经接纳到 D4 semantic ledger 的 Field/Facet/alias 定义、tombstone、migration、digest 和必要 immutable asset 不能因为 pack 或父模块 disable/uninstall 被删除。若当前 RegistryBinding 仍能证明该 namespace/definitions 为 `complete`，Core 可以继续解释、查询和保留已有作者事实；这只是 portable schema interpretation，不代表依赖 contribution 仍 active，也不自动开放该 pack 的 View、Action、derived rule 或新建作者事实能力。若 definitions 无法完整证明，则按 D4 `unavailable` / `provider_or_schema_unavailable` 路径保留 raw source；不得把未知或 unavailable 解释为空值。

Calendar System/Holiday Schedule 这类 rule/data Pack 默认没有作者 schema 注入权：parent Calendar extension point 非 `ready` 时不注册历法/节假日领域规则、不生成派生字段/View、不刷新相关 cache，也不暗中执行 dependent connector；已存在的 period/range/event 作者 facts 不变，pack 配置与来源仍保留。Organizations country schema pack 若其已接受 schema bytes 仍由当前 D4 binding 完整证明，可以继续解释已有 namespaced author facts；但 parent contribution 不 active 时不得借 retained schema 偷开 Organizations 专属 View/Action、自动 Assign 或 connector。

父 dependency 状态变化的 activation 规则固定：

| 条件 | D10 contribution activation | D4/作者事实 | UI/config |
| --- | --- | --- | --- |
| parent ready + version compatible + surface supported | 可在其它授权/预算门通过后 active | 按 current RegistryBinding 解释 | UI 可显示，也可被用户隐藏 |
| parent missing | inactive | 已接受 history/raw source 保留；definitions 能否 typed 解释按 D4 complete/unavailable | config/source 可恢复查看 |
| parent disabled | inactive | 同上；不删除 schema/作者 facts | 全局扩展管理仍可见 |
| parent incompatible | inactive | 旧 history 保留；不得用 incompatible runtime 产生新派生语义 | 显示版本不兼容诊断 |
| surface unsupported | 该 surface inactive | portable author facts 仍按该 surface 的 Core/D4 能力处理 | 不出现可执行入口；全局/其它 surface 配置不删除 |
| UI hidden only | activation 不变 | 完全不变 | 只隐藏入口，不改变 capability |

当前选择是 **C：定义/历史保留、contribution 激活、UI 可见性三轴分离**。替代 A“父停用即删除 schema/作者事实”违反 D4 raw/history 不变量，reject；替代 B“pack 已安装就无视父状态继续运行”违反 Intake §6.5/§8.3 的 parent extension-point 边界，reject；替代 D“把 Calendar/Organizations pack 规则烘进 Core”把领域语义迁入 Core，reject。该选择不新增 Calendar 算法、产品模块或安装 UI 实现。


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

首版只允许用户或管理员向一个具名 Automation 或 Run 作一层 delegation；Agent、tool、connector 不得再次把权限转授另一主体。子步骤仍在原 Run 内，并受原 Lease 与预算限制。

有效资格为：D1 capability ∩ current D6 Policy/ObservationScope ∩ DelegationLease ∩ contribution deployment policy ∩ egress grant ∩ secret-use grant ∩ external-effect approval ∩ current ActivationBinding ∩ budgets。任何一项失败都不能由另一项补足。

R05 进一步把“使用资源”和“管理资源”分开。Workspace 自助管理需要 proposed D6 Policy/2 capability `d10_control_self`；Workspace 管理仍使用 `policy_admin`，Registry 激活另需 `registry_admin`；deployment trust/account/secret/pricing/grant 只由 host DeploymentControlPolicy 管理。用户可合法创建和管理自己的有限 Automation，但选择部署费用账户、secret、egress 或 external-effect resource 必须持有由部署管理员签发的 exact ResourceUseGrant。grant revision/续期不清零 spent/held/attempt 次数；新 grant 也不消除旧 attempt 对实际账户的义务。完整字段、授权顺序与 stable replay 见 CONTROL-CONTRACT §6–§9。

Lease 的 readScope 不能虚构 D6 不存在的 Field ref-set 权限。Core 先按 D6 原授权取得合法读写范围，D10 再用准确 owner、Field 和 context 约束收窄。过期、撤销或 generation 变化阻止新受保护步骤；已经提交的作者决议和已经线性化发送的外部请求不被倒推回滚。

`maxRuns` 是同一个 `leaseId` 谱系在整个寿命内允许进入受保护执行的 Run 总上限，取值为有限正 Counter。修改 `leaseRevision` 不会清零既有消费；受权的后继 revision 可以提高上限，但不能把上限降到已消费次数以下，撤销使用独立 revoked 状态。创建 queued Run、建立 occurrence claim、执行纯调度记账或在第一项受保护步骤前取消，都不消费 `maxRuns`。

第一次获准执行受保护步骤之前，Core-managed Run admission CAS 必须同时验证当前准确 `leaseId/leaseRevision`、可信当前时间位于 `notBefore..notAfter`、当前 ActivationBinding、Run/occurrence 绑定和所有准入预算，并证明该 `leaseId` 谱系累计消费小于 `maxRuns`。CAS 成功时耐久建立：

```text
LeaseRunUse/1 {
  leaseId,
  leaseRevision,
  runId,
  automationId,
  definitionRevision,
  occurrenceKey,
  admissionClockEpoch,
  admittedAt
}
```

该记录与累计消费在同一受管 control transaction 保存，是 Run 准入防重事实，不是作者 ledger。CAS 成功就是 `maxRuns` 的消费线性化点：之后模型或工具失败、用户取消、Run 失败或取消、进程崩溃都不退款。同一 Run 重启只恢复原 `LeaseRunUse/1`，不再次消费。上限耗尽时，在进入 D6/D7 或外部 transport 前返回 D10 `delegation_exhausted`。


`maxRuns` 只在“这个 Run 尚无 `LeaseRunUse/1`”的首次准入分支比较剩余次数并消费。已经存在、完整且与同一 `runId`、`leaseId`、准确 `leaseRevision` 绑定的 `LeaseRunUse/1` 时，同 Run 的第二个或后续受保护步骤、以及该 Run 内原 D6 planned request 的恢复都**不再次检查 remaining>0，也不再次消费次数**；即使 `maxRuns=1` 且累计消费已经等于 1，也不能把同 Run 误判成“新 Run 已耗尽”。这些后续步骤仍逐次验证当前 D6/D10 授权、记录中的准确 Lease revision、可信时间、当前 ActivationBinding、适用批准和预算。Lease 被撤销、过期、revision/binding 改变或其它当前资格失败仍会阻止执行；只有“新 Run 没有既有 LeaseRunUse 且累计已达 maxRuns”才返回 `delegation_exhausted`。既有 LeaseRunUse 丢失或连续性不可证明时返回 `state_unavailable`，不能重新消费一次来猜测恢复。

Lease 的可信时间资格在每个新的受保护步骤以及最终作者或外部提交前重新检查。清理任务是否运行不决定 Lease 是否过期；只要可信当前时间已经超过 `notAfter`，新步骤固定返回 `delegation_expired`。若 clock epoch 或时间连续性不可证明，固定返回 `state_unavailable` 并暂停，不能假定“时间没有经过”；可信时间恢复后按真实当前时间重新裁决。已经发生的作者提交、外部效果、费用和 `LeaseRunUse/1` 历史均不回退。
## 9. Context、出站与提示注入

Agent 没有 ambient workspace。每个 context request 明确选择 workspace、typed source、最大范围、目的、目标模型/工具 recipient 和预算。Core/Server 在当前授权下构造 immutable ContextBundle，绑定精确 source versions/result epoch/authorization generation 与实际选出的 bytes；上下文选择遵守最小必要和数据最小化。

Context readable 不等于允许 egress。发送到 Model A、Connector B 或 remote MCP C 分别是不同 recipient authorization；切换 provider、endpoint、account 或 remote origin 必须重新匹配。

优先级固定：host/product policy 与受管 delegation/approval 是控制输入；用户明确任务是业务输入；Document、web page、email、tool result、MCP description/prompt/resource、model output 都是不可信数据。任何来自不可信数据的“忽略系统规则”“调用更多工具”“发送 secret”“批准下一步”均只能作为普通文本，不改变控制状态。

Secret、credential、token、plan/effects handles、private environment 和未选择 workspace data 不得进入普通 ContextBundle。日志和 transcript 使用显式 redaction；redaction 失败的敏感值不允许发布到普通日志。

## 10. Tool Value Profile 与 MCP

R05 不再把 ToolValueProfile/1 留作“复用某个子集”的类型占位。CONTROL-CONTRACT §3 由 D10 Tool Adapter owner 正式冻结 ToolValueProfile/1、ToolType/1 与 ToolValue/1 的 closed wire：bool、text、int64、integer、decimal、optional、closed object、bounded list 和 closed union，并固定 depth/member/arm/list/byte 限额、canonical numeric 与 unknown-member 拒绝。它不是 D7 TypeSpec alias，也不是任意 JSON Schema。

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

首版 Automation 只调度一个已经接纳的 invocation，不提供通用 DAG、循环或自由脚本。CONTROL-CONTRACT §7 把 invocation 冻结为 closed union：普通已接纳外部工具使用 `kind:"tool"` + ToolValue；无人值守作者路径只能使用具名第一方 `weftext.automation/set-field-member` Core adapter 与 `FieldMemberTask/1`。任何 ToolValue 字符串、title、path 或 model output 都不能提升为 NodeRef/FieldId。definition 保存该准确 invocation、schedule、DelegationLease、approval binding、budgets、queue limit 与 missed policy。

schedule 仅支持一次性 D4 ZonedInstant，或从明确的 Calendar recurrence/range source、有限 horizon 与 limit 产生 occurrence。Core 必须从实际 source 与冻结的 D4 rule context 计算，不接受 executor 自报可信时间规则；无法得到确定 instant 的 date-only 输入不暗补午夜。

并发策略固定 serial。missed policy 只有 skip 和 run_once；run_once 只执行当前恢复窗口中最新一个遗漏 occurrence，其余记录 skipped。恢复窗口必须有有限 policy 上限，不能无限回放历史。

```text
AutomationOccurrenceKey/1 =
  (automationId, definitionRevision, sourceOccurrenceKey)
```

同一 key 最多对应一个 Run identity 和一份 durable claim；进入 terminal 后也不能因为 restart、schedule rescan、disable→enable 或 scheduler cache rebuild 创建第二个 Run。enable/disable 只改变 control revision，不改变 definitionRevision，也不删除 claim 或 terminal proof。definition 的 invocation、schedule、Lease、approval 或预算语义变化产生新 definitionRevision，并只接管明确 activation point 之后的 occurrence。

terminal occurrence 历史可以压缩，但只允许压缩成仍能证明该 key 已经产生过原 Run 和原 terminal outcome 的耐久摘要；当某 definition revision 仍可能被 scheduler 重扫或恢复时，不得用普通 GC 把“已执行”变回“从未见过”。同一个原 claim 的恢复始终返回原 Run identity 和原 outcome。

Occurrence claim 可以先于 Run admission 建立，因此 queued 或 blocked 且从未进入受保护执行的 Run 不消耗 Lease `maxRuns`。已经有同一 Run 的 `LeaseRunUse/1` 后，后续 step 与原 planned 恢复复用该准入事实，不再因为全局 remaining=0 重新执行 Run-admission count gate。第一次受保护步骤通过 §8 的 Run-admission CAS 后才建立 `LeaseRunUse/1`。若 `maxRuns` 已耗尽，当前 occurrence 保持其既有 claim/Run 并进入 blocked，返回 `delegation_exhausted`；不得把同一 occurrence 换新 Run 绕过上限。

每次 occurrence 开始和每个新的受保护步骤前重新验证 current capability、准确 Lease revision、trusted time、D6 authorization、ActivationBinding、secret generation 与预算。已生成 D3/D6 request 后重启只恢复原 Run 和原 request；不能重新采样 target 或生成新 OperationId。暂停期间 Lease 可以自然到期，即使清理 scheduler 从未运行；可信时间推进后必须阻止新的 context、model、tool、external step 和最终作者提交。时钟连续性丢失时 fail closed 为 `state_unavailable`，恢复可信时间后再判断 expired 或 active。

## 14. Standing Approval

Standing Approval 不是“允许 Agent 以后任意修改”。首版完整规则只由 CONTROL-CONTRACT §7 的 `SingleFieldMemberRule/1` 拥有；本节只消费同一定义，不再另写竞争的 type/constraint enum。

首版无人值守作者 profile 仍是原 D7 `set_field_member`：单 existing Node、单 Field、当前完整 Field 恰一 Entry、一个 existing scalar member。Automation task 与 Standing Approval 必须分域：`FieldMemberTask/1` 提供本次 Run 的具体 ownerNodeRef/FieldId/memberPath/原 D7 TypedLiteral，`SingleFieldMemberRule/1` 只进一步收窄实际 prepared effect 是否可批准。Envelope 直接引用唯一 owner：

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
  rule: SingleFieldMemberRule/1,
  maxSuccessfulCommits,
  budgetAccountBindings
}
```

CONTROL-CONTRACT §7 保留 bool、exact text、int64、integer、decimal、semantic_code；0..64 TypedLiteral enum、同型 exact numeric closed range、有限且禁止 CR/LF 的 exact text、1..7 D4 静态 object-member path 以及两个实际效果分支。它同时逐字保留 D7 Optional bridge：D4 optional scalar member 只有当前已 present 且 D7 action 提供 Optional<scalar>.some 时才可自动批准；`none` 删除不属于自动路径。真实 semantic-code 正例使用 S `people/phone.label` contribution-set member。

共同前提仍为 current D1/D6/D10 authorization、准确 approval/automation/definition/delegation/activation binding、fresh source revision、完整 Field 恰一 Entry、D7 Narrow Field Qualification 成功、owner/Field/member/type 精确、新值落入 CONTROL 冻结 constraint、完整 owner_fields preview 可取，并可建立 audit 与 approval-count/cost reservation。

实际结果仍只有两支。**member-change** 要求原 D7 adapter 的完整 proposed source 和实际 MutationFootprint 只包含所选 existing scalar member 变化，所有其它作者/control 事实不变，owner_fields preview 有完整 Field before/after `field_change`。**逐字 raw no-op** 要求同 current 唯一 target、按 required/present-optional 投影后的 D7 同型 equality，以及原 adapter 的完整 proposed source 与 before bytes 逐字相同；MutationFootprint、`field_change`、D6 `sourceVersions` 均为空。

完整映射由 CONTROL-CONTRACT §7 唯一拥有。真实 S `people/phone.label` 的 task 输入是完整三-code D4 scope 上的 Optional<semantic_code>.some；唯一 present Entry 的 `personal→work` 是真实变化，`work→work` 可以是逐字 raw no-op，approval enum 可比 D4 code scope 更窄。若存在第二条 phone Entry，自动 exactly-one 选择不适用，但普通交互 D7 路径仍可用真实 occurrenceKey/rawEntrySource selector 明确选择第二条。其它结果都不属于 Standing Approval：不得选 first/preferred/same-value target，不得扩大 whole Entry/source，不得降级 append/remove，也不得换同名 target；只能进入交互确认、blocked 或 failure。若 raw no-op 按原 D6 形成 committed decision，仍消费一次 successful-commit approval count；同一 saved D6 decision replay 不重复消费。

## 15. Fresh prepare、批准消费与 planned 恢复

每次自动作者修改都消费 CONTROL-CONTRACT §7 的准确映射：fresh 完整 Field selection → 用 owner/FieldId/fresh expectedRevision/真实 occurrenceKey/准确 rawEntrySource 构造原 D7 FieldSelector → 原 `set_field_member` TypedLiteral → 原 `d7_action_prepare` → 完整 preview/effects/实际 MutationFootprint → 比较独立 StandingApprovalEnvelope → ApprovalUse → 原 D6 request。受保护的 `D10AuthorPreparationLink/1` 与原 PreparedActionBinding 在提交前原子保存；restart/planned/submitted_unknown 只能恢复该准确 request，绝不生成替代 OperationId。

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

ApprovalUse 是受管授权证据，不是 author plan，不新增 ActionSpec、PreparedActionBinding/2 或 `d6_commit_request` member。客户端不能提交 approved=true。Core 依据已保存 prepared record、preview 和 footprint 独立建立它。其 approval-count 状态只允许 `unreserved → reserved → consumed | released_terminal`；历史状态不可删除或回退。

现有上游不足以冻结这种无人值守确认，因此 UPSTREAM-AMENDMENTS 提出 D6/D7 coordinated amendment。在修订尚未共同接受前，无人值守 author commit 必须保持 unavailable，Automation 只能准备 proposal 等待交互确认。

### 15.1 D10 前置拒绝与 D6 内竞争失败分域

在尚未发送正式 `d6_commit_request` 之前，D10 adapter 可以根据已经可见的控制状态返回 D10 `approval_required`、`approval_expired`、`delegation_expired` 或 `delegation_exhausted`。这些只是前置控制结果，不证明之后没有竞争。

一旦 D10 自动提交路径已经把 ApprovalUse 作为内部依赖关联到原 planToken 并进入 D6，批准资格的竞争失败归 D6 owner。配套修订为 `d6_error.code` 增加唯一值 `approval_unavailable`，其 disposition 固定为 `preflight`。它只在原 D6 current authorization/ObservationScope 和适用业务可见性已经通过、而本次关联的 standing/supplemental approval 在 planning CAS 或 planned 恢复提交前不再可消费时返回。原 D6 权限失败仍是 `not_visible`；业务 dependency/semantic/budget 错误仍使用原 D6 code，不能被 `approval_unavailable` 遮蔽。

对 unseen request，`approval_unavailable/preflight` 不写 author ledger、不写 recorded rejection，也不建立 planned；调用方可在新的合法批准下重试原仍有效的准备，或回到交互路径。对已经 planned 的 request，同一错误保持原 ledger 为 planned，保留原 plan、pins 和 reservation；不能写成 `semantic_rejected` 或 `terminal_failed`。D10 adapter 不把这个 D6 error 包装成自身 approval code；如 UI 需要显示“次数耗尽/已撤销/已过期”，必须另经当前受权的 D10 control read 得出。

这是第一份共同公开 unattended author-submit 合同的一部分，而不是匿名 old/new D10 profile 兼容层。固定上游 D6 尚未发布为旧产品 API；共同接受时第一份公开 D6-Control/1 error closed set 直接包含该 code。能进入该正式 branch 的组件组合必须支持同一闭集；不兼容组合在 D1 capability/version gate 前拒绝，不能进入 D6 后再靠未知 enum 探测版本。Policy/1/2、bootstrap profile/1/2 和历史 saved decision decoder 保持原样。

### 15.2 planned 后重新查阅原 preview

原 PreparedActionBinding/2、语义 preview 与恢复所需 pins 在 planned 后按原 ledger recovery 寿命保留，但原 preview token 可以独立过期；现有 `d7_effects_resolve/open` 又只对 committed decision 开放。因此配套 D7 transport 必须增加一个只读 planned-preview 恢复入口，而不是延长旧 token 或重新 prepare：

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

处理顺序固定为：closed decode → 当前 authenticated audience 与受保护最小定位映射 → 原 D6 current authorization、原 ObservationScope 和完整 preview 披露资格 → authority/custody/ledger continuity → 同 key 的 byte-equal request 且状态确为 planned → 原 PreparedActionBinding/2、语义 preview 和所有 pins 完整可证 → 创建新的有限 recovery delivery epoch。

新 `previewToken` 仍使用 action_preview 运输族，但只绑定原 planned 保存的不可变 preview 语义和原 pins；不得重跑 Query、重新解析漂移后的 definition、重新选择 target、重算 proposed source 或改变 request。它有独立有限交付寿命，不延长或复活旧 previewToken/cursor。后续 page/EffectBytes 读取继续使用原 D7 transport；该 recovery epoch 过期仍按 preview transport 的 `preview_expired` 处理。非 planned、request 不等、pins/continuity 不可证统一沿现有 `d7_effects_error` 的 `effects_unavailable` 或更早的原授权错误；`d7_effects_resolve/open` 仍保持 committed-only。

只有在一个 recovery epoch 下完整读取 manifest、所有 pages 和必要 EffectBytes 后，UI 才能建立新的交互授权。授权记录为：

```text
PlannedDecisionApproval/1 {
  approvalId,
  approvalRevision,
  workspaceRef,
  grantingPrincipal,
  request,
  previewSemanticBinding,
  delegationBinding,
  activationBinding,
  grantedAt,
  expiresAt
}
```

它只绑定这份 exact 原 planned request 和保存的 immutable preview semantic record，不绑定可过期的运输 token，也不修改 OperationId、plan、PreparedActionBinding 或 target。最终提交仍重新执行当前 D6 authorization、trusted time 和原 business dependency 检查；确定的 dependency conflict 继续按 D6 原 authoritative-abort 规则处理，新的交互批准不能复活。

### 15.3 approval reservation 的终态

D6 planning CAS 才把 ApprovalUse 的 count 状态从 unreserved 原子变为 reserved。作者 commit 成功时，同一 final transaction 变为 consumed；same saved decision replay 不再消费。

若原 D6 ledger 在完整连续性和当前受权下证明 planned decision **确定永不提交**，并按原规则写 authoritative `terminal_failed`，该 abort transaction 同时把 ApprovalUse 的 count reservation 从 reserved 变为 `released_terminal`。released_terminal 不计入 `maxSuccessfulCommits` 的 reserved/consumed 总数，但保留不可变历史；terminal replay 不再次释放。

只有 authoritative terminal_failed 可以这样释放。Run cancel、preview/approval TTL、临时撤权、Lease 到期、attempt/work 暂停、authority/continuity 暂不可证都不构成 abort，reservation 继续保留。费用 reservation 是另一状态机：已经收费、可能收费或费用 unknown 的记录绝不因为 author terminal_failed 自动退款；它们按 §18 的 settled/released/uncertain 规则独立处理。

如果 planned decision 后由 `PlannedDecisionApproval/1` 明确人工恢复并最终 committed，原 standing-approval count reservation 仍从 reserved→consumed；本次人工授权不会把已经占用的 standing slot 退回再让另一个 Run 使用。
## 16. Connector 与 secret

Connector 是具名外部系统的协议 adapter。它拥有 provider-specific cursor/etag/version/account state，但这些只存在 D10/D6 控制域，不成为作者 Ref 或 Field。外部 stable ID 只有经 D3 已冻结的 SourceBinding/OriginBinding 协议才可参与 lookup/upsert，不能因 connector 安装自动创建 identity。

需要修改 SourceBinding、OriginBinding、watermark 或持续 sync state 的流程必须有具名 closed adapter 生成原 D3/D6 request，并把实际进度与对应 author commit 原子关联。首版 single_field_member standing approval 不包含这些 control effects，所以不能宣传为通用双向同步。

SecretRef 绑定 contribution、external account、usage、audience 与 secretGeneration。credential 原值只在受信 transport 的认证通道注入；不进入 workspace source、ContextBundle、model prompt、普通 ToolValue、transcript、普通 log 或 export。

credential rotation 产生新 generation。未发送的新调用必须使用当前允许 generation；已开始 external request 的恢复仍绑定其实际旧 generation。新 credential 可以在明确获准时用来只读 reconcile 同一账户，但不能偷偷重发一个 outcome_unknown mutation。

## 17. External effect

外部 mutation 与 Core transaction 永不组合成一个“原子成功”。

CONTROL-CONTRACT §7 唯一拥有完整内部 `ExternalEffectIntent/1`、`ExternalExecutionBinding/1` 与 public `ExternalEffectCurrentView/1`。immutable intent 冻结准确 contribution/account/operation/target/request payload 与 idempotency proof；具体 send attempt 另冻结 Lease、external-effect grant、egress grant、supplemental approval、secret generation 与全部可归属费用 reservation。secret bytes 永不进入这些 public projection。

状态为 prepared → submitting → succeeded | failed_no_effect | outcome_unknown；只有能证明请求尚未开始发送时才可从 prepared 进入 cancelled。lifecycle Binding revision 与 immutable requestDigest 分域，因此合法 prepared→submitting 不会自行使 consent 失效。改变 contribution/account/target/payload/idempotency 必须新建 effect intent 并重新 consent。outcome_unknown 保持未知直到可靠 reconcile；manual_required 是恢复方式，不是假失败终态。

自动 retry 只在原 intent 已包含且仍有效、针对准确请求接纳的 bounded idempotency proof，或已有可靠 failed_no_effect 证据时允许。retry 使用同一 EffectIntent、semantic request、target/account 与原 idempotency key，并重新验证 current authorization/egress/cost。proof 失效、target/request 改变或 credential 变化破坏原合同就停止自动发送。新的 sendAttemptId 不能拿新的 effectId 或 idempotency key 重做 unknown 原请求。

受信 transport 使用 CONTROL §11 send fence；首次不可逆 handoff 前冻结 ExternalExecutionBinding，并把 send-attempt evidence 与全部 cost hold 耐久关联。revocation/stop 先赢则不发送；handoff 先赢则保留已发送事实，后续可进入 outcome_unknown。D9 conversion worker 保持原 no-network 默认，不能因为 D10 有网络就借用 egress。D9 PublicationReceipt 只证明 external publication；把结果另存 Resource 仍是独立的原 D3/D7 author decision。

补偿操作是新的 ExternalEffectIntent，需要自己的授权、批准和费用，不是 rollback。一个 workflow 同时要求 Core write 与 external mutation 时分别展示两个 outcome，不生成综合 author receipt。

## 18. Budget、费用与并发预留

D6 原 work/attempt budget 保持。D10 为 model、tool、network 和 external cost 使用 CONTROL-CONTRACT §6、§10 的 ResourceUseGrant、CostReservation 与 CostSettlementDecision。Run、Lease、Automation、Workspace、grant 与 deployment account 的最窄剩余额度必须在同一 authority store 的准入事务中原子满足，避免两个并发 Run 同时看到最后余额。这些层级是 ceiling/projection，不是同一 reservation 的多个 actual account：一份 CostReservation 仍只绑定一个 attempt、一个实际账户、一个 grant、一个 pricing version 和一个 currency，一笔费用只记一次；真正可分别归属的多账户费用必须拆成可归属的 attempts/reservations/evidence，需要时可作为一个原子组准入。

Money/1、currency、microUnits、grant/account ceiling、pricing binding 和 checked arithmetic 的 exact shape 由 CONTROL-CONTRACT §2、§9 定义；没有隐式换汇，也没有调用方手写“实际费用”或目标终态的账务接口。每个可能收费 attempt 在开始前绑定一个独立 reservation、attemptId、实际 account/grant、pricing version 与有限 upper bound。grant 续期或 revision 更新不清空 spent、held、attempts；换 grant 不迁移或消除旧 reservation/account liability。

费用状态机固定为：

```text
reserved → settled(actual) | released | uncertain
uncertain → settled(actual) | released
```

只有 settled 与 released 是终态；uncertain 是**可恢复非终态**并持续占用完整 upper bound。released 只允许可靠 never-started evidence 证明本 attempt 的收费执行/send 从未开始；已经实际发送、最终可靠账单为 0 必须 settled(0)。同 reservation/attempt/account/currency/pricing 的可靠 final bill 使 uncertain→settled(actual)；例如 reserve100→uncertain→final bill20 只返80。

reconcile 只能由当前部署管理员或该账户具名 reconciler 使用 EvidenceTicket 驱动，采用 expected reservation revision CAS。成功必须在同一事务追加不可变 CostSettlementDecision、更新 reservation、grant/account held/spent/available、evidence 和 audit。same decision replay 不二次返额，并发冲突至多一个 winner。错误 attempt/account/currency、non-final 或不可唯一拆分的证据保持 uncertain/full bound。管理员无证据归零、effect idempotency、业务 rollback、author terminal_failed、TTL 或 Run terminal 都不是退款证据。

actual 超 reservation upper bound 沿原 overcharge anomaly/freeze 路径处理，普通 reconcile 不静默扩大 ceiling。ApprovalUse count、LeaseRunUse 与 cost reservation 继续完全分域。

## 19. Audit 与 retention

下列 protected step 必须在执行前写入耐久本地/Server authority-bound audit started record：敏感 workspace/context read、egress、secret use、经 D10 发起的 author submit、external mutation、delegation/approval/package activation mutation。不能写入时 fail closed。

远端 log collector 不可达不必使所有本地能力不可用：只要本地受保护 audit spool、完整性和保留预算仍可证明，可继续并稍后汇聚。真正本地 durable audit failure 则停止新的 protected step。

Core author commit 的 audit link 与原 author decision 同事务保存。external effect 的 terminal evidence 若无法耐久记录则保留 started/outcome_unknown，不能再发送一次“补日志”。取消、撤权、紧急停止必须保留独立 control-write reserve，不能被普通 telemetry quota 用尽而阻塞。

Transcript 与 audit 分开。transcript 可按部署 policy 有限保存/删除，但删除不能移除 planned author recovery、unknown external effect、uncertain cost 或安全审计所需 pins。Secret 与 raw credential 永不进入 transcript/audit export。

## 20. Run、取消与恢复

Run state 闭集为 queued|running|awaiting_confirmation|blocked|cancelling|reconciling|completed|failed|cancelled。每个 step 另保存实际领域 outcome，Run terminal state 不能抹掉已经 committed 的作者事实或已经发生的外部效果。

Occurrence claim、Run identity、`LeaseRunUse/1` 和 terminal outcome 是四个不同控制事实。scheduler 为一个 occurrence 建立 claim/Run 后，restart、rescan、disable→enable 或 cache rebuild 都必须恢复原 Run；terminal K 不会再次获得新的 Run identity。实现可以压缩历史，但压缩后仍必须证明 K 已经处理，不能以删除行的方式恢复“未执行”。

queued/prepared 且未通过 §8 Run-admission CAS 的 Run 可以在第一项受保护执行前取消，此时不消费 `maxRuns`。一旦 `LeaseRunUse/1` 已建立，之后 failed/cancelled/crash 均不返还该 run use。同一 Run 恢复只读取原 use。

已进入 D6 planned 的 request 不因取消 Run 自动 abort；Run 进入 cancelling/blocked，并按原 D6 planned recovery 处理。当前 delegation、trusted time 或 Standing Approval 失效可阻止缺乏当前资格的新 author commit，但不能把暂时失权写成永久业务 rejection。若已经有 committed decision，则不回滚。

external effect 进入 submitting 后，取消只阻止后续步骤；该 effect 最终仍是 succeeded、failed_no_effect 或 outcome_unknown。迟到 model/tool output 在 step 已关闭后不能启动新 tool/action。

R08 保留不可逆 emergency stop，但 stop 不是 rollback。CONTROL-CONTRACT §11 进一步闭合 exact-target stable key、预留 latch/result/safety-sequence capacity、immutable receipt 与只读 lost-response result query。该 safety write 共用 Authority Store serialization domain，但既不是普通 D10ControlPrepare，也不是 D6 author transaction；不要求先执行普通 management write，普通 configuration/budget 耗尽也不能阻止已预留 target 的首次 stop。该节同时规定三个实际线性化点：新 Run admission 与 stop 在同一 store serialization domain；D6 final author commit 必须在真正持有写锁的 final transaction 内重验对应 stop latch；external transport 必须把 stop gate 保持到第一次不可撤回的真实 send handoff，而不是只检查 queue admission。先线性化的 committed/sent 结果保留，后来的 stop 只阻止尚未线性化的新效果。

stop 不阻断当前获权的 authoritative abort、费用 settlement、evidence/audit retention 或 reference-safe cleanup。临时 disable、Lease expiry、普通 cancel 或暂时撤权都不是不可逆 abort proof。D6 配套提案新增 `execution_stopped/preflight`，并只允许已 planned request 在 current authorization、continuity、完整 RunBinding 和不可逆 stop 全部证明后进入原 `transaction_aborted/terminal` authoritative-abort 路径。

重启先恢复 durable occurrence claim、Run、LeaseRunUse、ApprovalUse/cost reservation 与原 requests。同 Run 的恢复先验证既有 LeaseRunUse 连续性，再验证当前授权、准确 Lease revision、可信时间、ActivationBinding、批准和预算；不得因为 `maxRuns` 的剩余数已经为 0 再做一次新 Run 准入。Lease 在暂停时自然过期不需要 cleanup task；可信时间超过 `notAfter` 后，新步骤和最终提交都拒绝。无法证明 clock continuity 时返回 `state_unavailable` 并保持原控制记录，不延长 deadline、不假定 Lease 仍有效。无法证明 execution side effect 的结果时进入 blocked/reconciling，而不是生成新 OperationId 或 effect ID。
## 21. 错误与不可用语义

D1 capability availability 及其固定 reason 优先级完全不变，尤其 `policy_denied` 必须先于组件、配置、network、version 与 health 细节。D10 不用一个 runtime_unavailable 覆盖 `missing_component`、`not_configured`、`offline`、`incompatible_version` 或 `temporarily_unavailable`。

CONTROL-CONTRACT §1 唯一拥有两个 D10 envelope。管理/current/history/secret/stop 入口使用 `D10ControlError/1`；尚未进入其它 protocol owner 的 D10 Run/step 使用 `D10RunStepError/1`，闭集为 `invalid_request|not_visible|control_conflict|binding_changed|approval_required|approval_expired|delegation_expired|delegation_exhausted|budget_exceeded|audit_unavailable|state_unavailable|invalid_output|cancelled|external_outcome_unknown`。Run/step 阶段固定为 closed decode/version → D1 static capability/surface/release → current principal/control-object visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution。unknown token、wrong tag、wrong audience 或无权控制对象继续统一 `not_visible`；expired/exhausted/conflict 细节只能在另行受权的 control disclosure 后取得。一旦进入 D3/D6/D7/D8/D9，原 owner envelope 必须逐字返回。

错误 owner 必须按进入边界区分：

| 失败位置 | owner 与规范结果 |
| --- | --- |
| 尚未调用正式 D6 request，D10 发现没有可用批准 | D10 `approval_required` 或 `approval_expired` |
| Run admission 发现 Lease 次数耗尽 | D10 `delegation_exhausted`；不进入 D6/D7 |
| Lease 已过期 | D10 `delegation_expired`；若 trusted time 不可证明则 `state_unavailable` |
| 已进入 D6，原 D6 author permission/ObservationScope 不成立 | 原 D6 `not_visible/preflight` |
| 已进入 D6，业务 dependency/semantic/budget 失败 | 原 D6 对应 code/disposition |
| 已进入 D6，只有关联的 standing/supplemental approval 在竞争中失效 | 配套修订新增 D6 `approval_unavailable/preflight`；unseen 不写 ledger，planned 保持 planned |
| 读取 planned 原 preview 的新 recovery transport 失败 | 原 D7 `d7_effects_error`，使用 `not_visible|preview_expired|reset_required|effects_unavailable|budget_exceeded` 等既有 code |
| committed effects 读取 | 继续只用既有 `d7_effects_resolve/open` 与原错误 |

因此 D10 probe 不能“消除”进入 D6 后的批准竞争，也不能在 D6 之外隐藏一个私有提交结果。`approval_unavailable` 与 R05 的 `execution_stopped` 是第一份共同公开 unattended author-submit D6 closed error set 的正式成员；不存在另一个仍可进入相同 branch 的旧 D10 error profile。设计接受只确定合同，运行时 availability 仍必须逐项通过 D1 的 contractMajor、release、surface、policy、principal、component/configuration、version combination、reachability 和 health 门。

一次 D6 `approval_unavailable/preflight` 可以在控制条件改变后用**同一个原 request**重试：unseen 仅在原 prepare 仍有效且重新取得合法批准时重试；planned 则在 §15.2 的 `PlannedDecisionApproval/1` 或仍有效原 approval 下恢复。它永远不写 `semantic_rejected`，也不因 TTL、cancel 或暂时撤权产生 terminal decision。

一旦请求进入原 D3/D6/D7/D8/D9 入口，除上述明确 D6 enum 扩展外，都返回该 owner 原有 error/disposition；D10 不包装成通用 AgentError，也不根据内部部署细节改变原拒绝顺序。
## 22. 产品端

Desktop 本地承载同一个 local Broker、scheduler、secret transport 与 Core adapter；CLI 本地连接/启动同一能力族，不创建另一套 scheduler 或 authority。Desktop/CLI 远端模式只调用 Server。

Server 承载 hosted Broker、scheduler、connector/model/tool executors 和 secret transport，并继续满足 D1 单实际提交持有者；本代不定义多主 scheduler/database 集群。WebUI 只能通过 Server 管理和发起能力，不直接运行 worker/model/connector、不持 secret。

Mobile 对 agent.session、automation、connector execution、conversion execution 和 credential management 返回 D1 unsupported_surface；不会用“只读监控”入口变相提供 D1 已禁止的批准或委托。Mobile 可读取普通已 committed workspace facts。

LTR/RTL、区域设置、屏幕阅读器和 Web/CLI 运输差异只影响呈现与交互，不改变请求字节、target、approval rule、费用、错误或作者/外部结果。

## 23. D1–D9 组合与必要修订

D1 表面、capability reason 和唯一提交持有者保持；D2 raw source/unknown provider preservation 保持；D3 identity/SourceBinding/OriginBinding/Provenance 保持；D4 Registry exact shape 与演进保持；D5 不新增持久 Record；D8 Draft/IME/explicit edit confirmation 与所有 Editor wire 保持；D9 worker/Template/ExportPlan/publication 与所有转换 wire 保持。R08 要求 D8、D9 原 owner 在 UPSTREAM-AMENDMENTS §8 补齐 Mandatory Intake §8.5.1 的术语与 technical-interface 映射。每个 public kind 都有唯一 technical-interface owner，并分开列 consumes/returns/operates-on；D10 不取得这些名称或 domain record 的 owner。

R08 coordinated amendment 集合保留既有 的 D6/D7 standing-approval author-submit、planned-preview 恢复、Policy/2 `d10_control_self`、bootstrap profile/3 与不可逆 stop 条款，并新增**仅命名元数据**的 D8/D9 owner 词表配套。精确提案在 UPSTREAM-AMENDMENTS；CONTROL-CONTRACT 只拥有 D10 host/control wire。D8/D9 词表修订不增加无人值守编辑、转换 profile、身份、作者提交或发布能力。共同设计接受或协调激活都**不等于运行时已实现/已发布**，`automation.manage|workspace.extensions.manage|deployment.external.manage|automation.stop|automation.author_submit` 仍必须进入 D1 正式 capability catalog 并通过全部真实 availability 门后才能 advertised available。

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
