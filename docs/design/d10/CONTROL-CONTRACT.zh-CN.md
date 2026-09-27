---
source_language: zh-CN
translation_status: source
---

[English](CONTROL-CONTRACT.md)

# D10 控制与管理合同

revision: D10-r05-unified-control-contract-2026-09-28；状态：candidate author revision，等待完整独立联合终审。本文件冻结 R05 所需的 D10 管理、授权、幂等、恢复、费用、停止和 package/contribution 控制语义。它不修改固定上游 S；所有 D6/D7 变更只在 UPSTREAM-AMENDMENTS 中作为未激活提案。

## 1. 权威、适用范围与错误边界

Core 仍是唯一作者事务权威。Workspace 作者提交、saved decision、receipt 和 planned recovery 继续由 D6 拥有；本合同不得保存第二份作者成功 ledger。部署 trust、package、secret、external account、pricing 和资源使用授权属于 D10 host control domain，不授予 Workspace 内容权限。

公开能力在进入本合同前仍经过 D1 的真实 contractMajor、surface、release、policy、principal、component/configuration、reachability/version-combination 和 health 门。设计接受、文档协调激活或存在一个 control record 都不是运行时 available 证据。

D10 控制入口自己的错误对象为：

    D10ControlError/1 {
      kind:"d10_control_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "authority_unavailable" | "integrity_conflict" |
        "control_conflict" | "stale_revision" |
        "state_unavailable" | "budget_exceeded"
    }

这些错误只用于 D10 prepare、host control、result、secret staging 和 stop。Workspace author commit 一旦进入 D6，继续返回 D6 自己的 closed error；D10 不包装或重新命名 D6 not_visible、approval_unavailable、execution_stopped 或 transaction_aborted。

对象不存在、scope/audience 不匹配、主体当前无权观察对象三者统一 not_visible。authority/fence/custody 不可证明为 authority_unavailable；受保护记录或证据完整性冲突为 integrity_conflict；同 stable key 不同完整输入为 control_conflict；当前可见对象的 expected revision 不匹配为 stale_revision；可信时间、外部证据或必要连续性暂不可证明为 state_unavailable。

## 2. 共享闭合类型

下列类型均为 D10 本代正式合同。所有 object 拒绝 unknown member、duplicate member 和非法 null。Counter 逐字复用 D6 的 0..2^63-1 非 Boolean integer；Uuid 使用 canonical lowercase RFC 4122 text。所有数组有界、排序和唯一性规则由包含类型指定。

    Version/1 = {
      major:Counter, minor:Counter, patch:Counter
    }

    VersionRange/1 = {
      minimum:Version/1,
      maximumExclusive:Version/1
    }

VersionRange 必须满足 minimum < maximumExclusive。不存在 latest、wildcard 或 ambient package version。

    Scope/1 =
      {kind:"workspace", workspaceRef:D3.WorkspaceRef}
    | {kind:"deployment", storeIncarnation:Uuid}

ControlRecordKind/1 闭集为：
受控记录类别闭集为：`automation, lease, approval, planned_approval, external_approval, run, workspace_budget, activation, deployment_policy, trust, package, external_account, secret, grant, cost_account, pricing, reservation, external_effect, stop`。

    ControlRef<K>/1 = {
      storeIncarnation:Uuid,
      kind:K,
      id:Uuid
    }

    Binding<K>/1 = {
      ref:ControlRef<K>/1,
      revision:Counter
    }

    Target<K>/1 =
      {kind:"new"}
    | {kind:"existing", binding:Binding<K>/1}

control id 与 incarnation 永不重用。retire、archive、revoke、close 或删除显示入口都不允许以后用同 id 指向另一对象。相同显示名称重建必须得到新 id。

    Money/1 = {
      currency:CurrencyCode,
      microUnits:Counter
    }

CurrencyCode 是 exact 三位 ASCII A-Z。D10 不执行隐式外汇换算。

本文其余基础标量沿用现有 closed JSON 语义：`Token` 是 D6 §1 的非空 opaque token；`Text` 是 Unicode scalar string；`Bytes` 是受相应入口字节预算约束的 byte sequence；`Boolean` 只接受 JSON true/false；`Sha256` 是 `sha256:` 加 64 位 lowercase hex；`HostPrincipal` 是受信 host authentication 映射得到的 `Token`，不是请求字段。`Ed25519PublicKey` 与 `Ed25519Signature` 只在受信 package/trust adapter 中出现，编码格式必须由该 adapter 的已接纳 profile 固定，普通 control caller 不能自报已验证。

受控 ASCII token 语法：

```text
LowerCamelAscii = [a-z][A-Za-z0-9]{0,62}
LowerKebabAscii = [a-z][a-z0-9]*(?:-[a-z0-9]+)*
CanonicalInteger = "-"? ("0" | [1-9][0-9]*)
CanonicalDecimal = CanonicalInteger ("." [0-9]*[1-9])?
```

`LowerKebabAscii` 总长 1..63 bytes；`CanonicalInteger` 和 `CanonicalDecimal` 必须在相应 ToolType 的显式 bounds 内。decimal 不允许尾随零、小数点后空串、指数写法或 negative zero。

    ScheduleHorizon/1 = {
      start:D4.zoned_instant,
      endExclusive:D4.zoned_instant
    }

ScheduleHorizon 不是 D4 新类型；两个成员逐字使用 D4 zoned_instant 作者值的时间语义，并由同一 exact UTC comparator 证明 start < endExclusive。本文不使用不存在的 “D4 bounded-instant-range” 名称。

## 3. Tool Value Profile

ToolValueProfile/1 由 D10 Tool Adapter 拥有，不是 D7 TypeSpec 的别名，也不是任意 JSON Schema。它只提供外部工具参数和结果的有限值代数。

    ToolValueProfile/1 = {
      kind:"d10_tool_value_profile",
      wireVersion:1,
      input:ToolType/1,
      output:ToolType/1
    }

    ToolType/1 =
      {kind:"bool"}
    | {kind:"text", maximumUtf8Bytes:Counter}
    | {kind:"int64"}
    | {kind:"integer", minimum:CanonicalInteger, maximum:CanonicalInteger}
    | {kind:"decimal", minimum:CanonicalDecimal, maximum:CanonicalDecimal}
    | {kind:"optional", item:ToolType/1}
    | {kind:"object", members:[ToolMember/1]}
    | {kind:"list", minimum:Counter, maximum:Counter, item:ToolType/1}
    | {kind:"union", arms:[ToolArm/1]}

    ToolMember/1 = {
      name:LowerCamelAscii,
      required:Boolean,
      type:ToolType/1
    }

    ToolArm/1 = {
      tag:LowerKebabAscii,
      type:ToolType/1
    }

`maximumUtf8Bytes` 取 1..8388608；对象最多 64 个 `members`；联合类型有 2..8 个 `arms`；列表 `maximum` 为 1..4096 且 `minimum <= maximum`；完整类型深度最多 16，规范类型字节最多 65536。成员名和联合标签按 UTF-8 字节排序且唯一。整数和十进制定点值使用规范十进制字符串，禁止二进制浮点、NaN、Infinity 和负零。

ToolValue/1 exact wire 为：

```text
ToolValue/1 =
  {kind:"bool", value:Boolean}
| {kind:"text", value:Text}
| {kind:"int64", value:CanonicalInteger}
| {kind:"integer", value:CanonicalInteger}
| {kind:"decimal", value:CanonicalDecimal}
| {kind:"optional", value:{kind:"none"} | {kind:"some", value:ToolValue/1}}
| {kind:"object", members:[{name:LowerCamelAscii, value:ToolValue/1}]}
| {kind:"list", items:[ToolValue/1]}
| {kind:"union", tag:LowerKebabAscii, value:ToolValue/1}
```

ToolValue/1 必须和调用点绑定的 ToolType/1 逐层一致。object 的 members 按 name 排序且唯一，只能出现声明成员；list 数量满足 bounds；union tag 必须命中声明 arm。int64 还必须落在 signed 64-bit 范围。

缺少必需成员、出现额外成员、选择错误联合分支、越界或超预算都返回 invalid_request。ToolValue 不能承载 EntityRef、Locator、SecretRef、文件路径能力、ActionEvidence、plan/result token、开放 map 或可执行值。

## 4. Package、Contribution 与依赖

PackageId/1 与 D4 SemanticNamespaceId 是不同类型，即使字符串可能相同。PackageId 使用 1..127 ASCII bytes 的点分 lower-kebab segments；LocalContributionId、ExtensionPointId 和 LocalOperationId 各为 1..63 ASCII bytes lower-kebab。比较全部 exact ASCII。

    PackageManifest/1 = {
      kind:"d10_package_manifest",
      wireVersion:1,
      packageId:PackageId,
      packageVersion:Version/1,
      publisherKeyId:Sha256,
      assets:[PackageAsset/1],
      contributions:[Contribution/1]
    }

    PackageAsset/1 = {
      assetId:LowerKebabAscii,
      digest:Sha256,
      byteLength:Counter
    }

    Contribution/1 = {
      contributionId:LocalContributionId,
      kind:ContributionKind,
      contractVersion:Version/1,
      descriptorAssetId:LowerKebabAscii,
      dependencies:[ContributionDependency/1]
    }

ContributionKind 闭集为：
贡献类型闭集为：`module, schema, view, action, template, preset, pack, tool, model, connector, importer, exporter, conversion, renderer, localization`。

这组 kind 覆盖 Mandatory Intake 已选的 module、Profile/schema、View、Action、Template、Preset、Pack、Connector/Adapter 以及 D9 import/export/conversion 和 D10 tool/model/runtime 贡献；它不表示所有 kind 已产品实现。

    ContributionDependency/1 = {
      role:"primary_parent" | "additional",
      parent:{
        packageId:PackageId,
        contributionId:LocalContributionId,
        extensionPointId:ExtensionPointId
      },
      requiredContractRange:VersionRange/1
    }

dependency 是具体 dependent Contribution 的成员，不存在 package 级 ambient dependency。pack kind 必须恰有一个 primary_parent；其他 contribution 至多一个 primary_parent，可以有 0..32 个 additional。parent tuple 必须解析到 exact activated contribution 和 extension point。resolved parent contract version 与 dependency result 进入 Capability Catalog digest；不能跟随 latest。

一个 connector contribution 不可用只使该 contribution inactive；同 package 的 template、schema、pack 或 view 只按自己的依赖和能力计算。定义历史、Contribution activation、UI visibility 继续保持三个独立轴。

每个 kind 的 descriptor 由既有 owner 的闭合合同验证：schema 只引用 D4 Registry 中已有的 namespace/Facet bindings；view/action 消费 D7 closed descriptors；template/preset/importer/exporter/conversion 消费 D9 合同；tool/model/connector/runtime 由 D10；module 只组织产品贡献，不拥有作者事实。

## 5. 四个第一方 module 的唯一 package 映射

本节是 D10 新候选映射，不冒称 D1 已定义 package/module ID、代码路径或 locale 资源。D4 已冻结的 semantic namespaces、owner tuples、FacetId 和 semanticMajor 原样保持。

| 产品 | PackageId | module contribution | schema contribution | D4 semantic owner / Facet | extension points |
| --- | --- | --- | --- | --- | --- |
| 日历 / Calendar | `weftext.calendar` | `module` | `schema` | `calendar` → `first_party,weftext.calendar`；`calendar/period-note`、`calendar/range-note`、`calendar/event`，`semanticMajor=1` | `calendar-system`、`holiday-schedule` |
| 文献库 / Library | `weftext.library` | `module` | `schema` | `library` → `first_party,weftext.library`；`library/work`，`semanticMajor=1` | `none` |
| 人物 / People | `weftext.people` | `module` | `schema` | `people` → `first_party,weftext.people`；`people/person`，`semanticMajor=1` | `none` |
| 组织 / Organizations | `weftext.organizations` | `module` | `schema` | `organizations` → `first_party,weftext.organizations`；`organizations/organization`，`semanticMajor=1` | `schema-pack` |

每个 module contribution 的 descriptor 只包含正式中英文产品名、同 package schema contribution 引用以及上表 extension points。schema descriptor 只列 exact SemanticNamespaceId、D4 namespace ownerId 和 FacetId/semanticMajor，不复制 FieldDefinition 或 FacetSchema bytes。

PackageId、Contribution contractVersion、D4 semanticMajor 是三个独立版本域。package update 不得在同 FacetId/semanticMajor 下改变 D4 semantic digest。

候选 code convention 分别为 CalendarModuleContribution、LibraryModuleContribution、PeopleModuleContribution、OrganizationsModuleContribution；候选 variables 为 calendar_module、library_module、people_module、organizations_module。候选代码 namespace d10::bundled::<domain> 与候选 locale keys module.calendar.title、module.library.title、module.people.title、module.organizations.title 都是设计映射，当前没有实现证据。没有独立 CLI verb；通用管理入口只消费完整 package/contribution ref。

禁止把 D4 ownerId 自动解释为 package ownership proof；上表只建立显式一对一映射。第三方签名、安装顺序、显示名或相同字符串都不能取得 first-party package/namespace。

## 6. Principal、管理域与 ResourceUseGrant

请求不能携带 principal、owner 或 authorized 布尔值。当前主体只能来自受信 host/D10 authentication mapping。

Workspace 自助资格 S：当前 Workspace Policy/2 对当前主体、workspace scope 显式 allow D6 proposed capability d10_control_self。它只允许管理自己的有限 D10 控制记录，不授予 author read/write、policy_admin、registry_admin、部署账户管理或 secret 原值读取。实际 author 操作仍逐项要求原 D6/D7 权限。

Workspace 管理资格 W：当前 Workspace、workspace scope 的 policy_admin；涉及 Registry activation 还必须有 registry_admin。W 可以停止、撤销、归档工作区控制记录和管理工作区预算，但不能冒充另一主体创建或扩大其 Lease/Approval。

Deployment 管理资格 H 由 DeploymentControlPolicy/1 拥有：

    DeploymentControlPolicy/1 = {
      kind:"d10_deployment_control_policy",
      wireVersion:1,
      revision:Counter,
      administrators:[HostPrincipal],
      reconcilers:[{
        principal:HostPrincipal,
        costAccountIds:[ControlRef<cost_account>/1]
      }]
    }

初始 H 只能由受信本地主机显式建立或 Server 部署 operator 配置；首次 HTTP、Workspace ownership、allocate_workspace 或 administer_issuer 不自动得到 H。普通 reconciler 只能对列出的 cost account 做证据驱动 settlement。

    ResourceUseGrant/1 = {
      kind:"d10_resource_use_grant",
      wireVersion:1,
      grantId:Uuid,
      grantRevision:Counter,
      usageRevision:Counter,
      state:"active" | "revoked" | "retired",
      grantee:{
        hostPrincipal:HostPrincipal,
        workspacePrincipal:Token,
        workspaceRef:D3.WorkspaceRef
      },
      contributions:[ContributionBinding/1],
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      permission:ResourcePermission/1,
      usage:ResourceUsage/1
    }

    ContributionBinding/1 = {
      packageId:PackageId,
      packageVersion:Version/1,
      contributionId:LocalContributionId,
      contractVersion:Version/1,
      descriptorDigest:Sha256
    }

    ResourceUseGrantSpec/1 = {
      grantee:{
        hostPrincipal:HostPrincipal,
        workspacePrincipal:Token,
        workspaceRef:D3.WorkspaceRef
      },
      contributions:[ContributionBinding/1],
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      permission:ResourcePermission/1
    }

`ResourceUseGrantSpec/1` 是创建/更新输入；`grantId`、两个 revision、state 和 usage 只能由 Core 从现有记录与事务结果产生。

ResourcePermission/1 四个 variant：

    {kind:"cost",
     account:Binding<cost_account>/1,
     limits:{currency:CurrencyCode,
             totalMicroUnits:Counter,
             perAttemptMicroUnits:Counter,
             maxAttempts:Counter}}

    {kind:"secret",
     secret:Binding<secret>/1,
     account:Binding<external_account>/1,
     audience:ContributionBinding/1,
     usageKind:"authenticate",
     limits:{maxUses:Counter}}

    {kind:"egress",
     account:Binding<external_account>/1,
     operations:[LocalOperationId],
     limits:{maxCalls:Counter,
             totalBytes:Counter,
             perCallBytes:Counter}}

    {kind:"external_effect",
     account:Binding<external_account>/1,
     calls:[{operation:LocalOperationId,target:ToolValue/1}],
     limits:{maxAttempts:Counter}}

同 grant 中 permission kind 唯一；secret 不蕴含 egress，egress 不蕴含 mutation，cost 不蕴含 account manage。cost 要求 perAttemptMicroUnits <= totalMicroUnits；egress 要求 perCallBytes <= totalBytes；所有可执行上限为有限正数，cost 金额可为 0。

ResourceUsage/1 与 permission kind 一一对应：
cost={spentMicroUnits,heldMicroUnits,attemptsStarted}；
secret={usesStarted}；
egress={callsStarted,bytesSent,bytesHeld}；
external_effect={attemptsStarted}。
这些 Counter 与仍在途的 attempt reservation 一起构成准入依赖。

同 grantId 的续期、限额调整或 grantRevision 更新不得清零 usage、held、spent 或 attemptsStarted；usageRevision 独立单调增加。降低限额不得低于已消费加仍 held 的金额/次数。改变 grantee、resource kind、cost account、external account 或 currency 必须创建新 grantId。新 grant 也不能消除旧 attempt、old grant 或实际 account 的费用义务；所有 grant 继续竞争同一实际 account ceiling。

## 7. Stable prepare、closed body 与公共入口

普通控制入口：

    D10ControlPrepare/1 = {
      kind:"d10_control_prepare",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      body:ControlBody/1
    }

ControlBody/1 只有七个 variant：

1. automation_configure：
   automation:Target<automation>；
   lease:Target<lease>；
   approval:Option<Target<approval>>；
   definition:AutomationSpec/1；
   delegation:LeaseSpec/1；
   standing:Option<StandingApprovalSpec/1>。
   approval 与 standing 必须同为 none 或同为 some。该固定 bundle 是唯一允许一起创建/重绑 Automation、Lease、Standing Approval 的组合操作，不是通用 batch/DAG。

2. consent：
   target:Target<planned_approval|external_approval>；
   consent:ConsentSpec/1。

3. state：
   target:Binding<K>；
   action:StateAction。
   K/action 适用闭集：
   automation→enable|disable|archive；
   lease/approval/grant→revoke|archive；
   run→cancel|archive；
   package/pricing→retire；
   external_account→disconnect；
   cost_account→close；
   secret→revoke。
   不存在 generic delete 或 revoked→active。

4. workspace_limits：
   target:Binding<workspace_budget>；
   limits:BudgetCaps/1。
   仅 W。

5. activation：
   target:Target<activation>；
   packages:[ContributionBinding/1]；
   registrySnapshot:D4.RegistrySnapshot/1；
   registryEvolution:Option<D4.RegistryEvolutionProof/1>；
   trust:Binding<trust>。
   仅 W 且 registry_admin。非 bootstrap Registry 必须带 evolution proof。

6. deployment_put：
   target:Target<K>；
   value:DeploymentValue/1。
   K 只允许 `deployment_policy|trust|package|external_account|secret|grant|cost_account|pricing`；K 必须与 `value.kind` 对应，而且只有 H 可以执行。

7. cost_reconcile：
   reservation:Binding<reservation>；
   evidence:EvidenceTicket/1。
   仅 H 或该 account 的 reconciler；没有 targetState、actual 或 manual amount 字段。

Option<T> 只有 {kind:"none"} 或 {kind:"some",value:T}，禁止 null。

    BudgetCaps/1 = {
      maxSteps:Counter,
      maxInputBytes:Counter,
      maxOutputBytes:Counter,
      maxElapsedMillis:Counter,
      costs:[{
        grant:Binding<grant>/1,
        maximum:Money/1
      }]
    }

所有 maxSteps/maxInputBytes/maxOutputBytes/maxElapsedMillis 必须正且有限；costs 按 grant ref 排序唯一，currency 必须匹配 grant/account。

    AutomationSpec/1 = {
      label:Text,
      invocation:{
        contribution:ContributionBinding/1,
        parameters:ToolValue/1
      },
      schedule:AutomationSchedule/1,
      missedPolicy:"skip" | "run_once",
      queueLimit:Counter,
      budgets:BudgetCaps/1
    }

    AutomationSchedule/1 =
      {kind:"once", at:D4.zoned_instant}
    | {kind:"recurrence",
       ownerNodeRef:D3.NodeRef,
       recurrenceOccurrenceKey:D4.occurrenceKey,
       rangeOccurrenceKey:D4.occurrenceKey,
       horizon:ScheduleHorizon/1,
       outputLimit:Counter}

parameters 必须由该 contribution 的 active ToolValueProfile/1 输入类型验证。recurrence selector 只定位同一 revision 重新绑定的 D4 recurrence source，不成为 durable EntityRef。

    LeaseSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      maxRuns:Counter,
      activationBinding:ActivationBinding/1,
      capabilityAllowlist:[D1.CapabilityId],
      readGrants:[D6 closed read capability request],
      resourceGrants:[Binding<grant>/1],
      budgets:BudgetCaps/1
    }

readGrants 只能使用原 D6 读取/状态观察 capability，不得包含 policy_admin、registry_admin、binding_admin、repair 或 d10_control_self。notBefore < notAfter，maxRuns 正且有限。

    StandingApprovalSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      rule:SingleFieldMemberRule/1,
      maxSuccessfulCommits:Counter,
      costGrants:[Binding<grant>/1]
    }

`SingleFieldMemberRule/1` 完全承接 C §14：动作固定为 `set_field_member`；目标限制为一个 owner/Field 下唯一 Entry 的单个 member；值类型闭集为 `text|bool|int64|integer|decimal`，约束闭集为 `any|enum|numeric_range`，并保留 raw no-op 分支。它不扩展到 create、delete、Facet、native table 或 bulk 操作。

    ConsentSpec/1 =
      {kind:"planned",
       originalRequest:D6.d6_commit_request,
       previewSemanticDigest:Sha256,
       lease:Binding<lease>/1,
       activationBinding:ActivationBinding/1,
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}
    | {kind:"external",
       intent:Binding<external_effect>/1,
       requestDigest:Sha256,
       resourceGrants:[Binding<grant>/1],
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}

planned 只授权原 planned request，不 reprepare、不换 OperationId、不换 target；external 只授权原 ExternalEffectIntent。

prepare 成功返回：

    D10ControlPrepared/1 = {
      kind:"d10_control_prepared",
      wireVersion:1,
      requestId:Uuid,
      prepareToken:Token,
      preview:ControlPreview/1,
      commit:
        {kind:"workspace", request:D6.d6_commit_request}
      | {kind:"deployment",
         request:{kind:"d10_host_control_commit",
                  wireVersion:1,
                  scope:Scope/1,
                  requestId:Uuid,
                  prepareToken:Token}}
    }

    ControlPreview/1 = {
      kind:"d10_control_preview",
      wireVersion:1,
      canonicalIntentBytes:Bytes,
      affected:[{
        ref:ControlRef<K>/1,
        change:"create" | "update" | "enable" | "disable" |
               "revoke" | "cancel" | "archive" | "retire" |
               "disconnect" | "close" | "settle",
        beforeRevision:Option<Counter>,
        proposedAfterRevision:Option<Counter>
      }],
      resourceUses:[{
        grant:Binding<grant>/1,
        maximum:Option<Money/1>
      }]
    }

affected 按 ref canonical bytes 排序且唯一；resourceUses 按 grant ref 排序且唯一。canonicalIntentBytes 是本次已成功 closed-decode 的完整 D10-Control-Intent/1 bytes，因此包含 caller 已提供并获权查看的拟议控制语义；preview 不另带 secret bytes、隐藏作者值或其它用户账务。

ControlPreview/1 只包含当前主体已获权的控制元数据；超预算拒绝，不截断。

查询：

    D10ControlResultRequest/1 = {
      kind:"d10_control_result",
      wireVersion:1,
      scope:Scope/1,
      requestId:Uuid
    }

它返回当前有权披露的 prepare/apply 历史结果，不等价于重新执行，也不声称历史 after revision 仍是当前状态。

## 8. Idempotency、CAS 与结果重放顺序

稳定 key 为 (scope incarnation, initiating principal, requestId)。requestId 必须在首次调用前由客户端保存并在 transport retry 中复用。

Core 内部保存：

    StableControlKey/1 = {
      scope:Scope/1,
      initiatingPrincipal:Token,
      requestId:Uuid
    }

    PreparedCommitRequest/1 =
      {kind:"workspace", request:D6.d6_commit_request}
    | {kind:"deployment", request:{
        kind:"d10_host_control_commit",
        wireVersion:1,
        scope:Scope/1,
        requestId:Uuid,
        prepareToken:Token
      }}

    ControlDependencies/1 = {
      configBindings:[Binding<K>/1],
      usageBindings:[{
        ref:ControlRef<K>/1,
        usageRevision:Counter
      }],
      authorityProof:Token,
      authorizationGenerations:[Token],
      stopRefs:[ControlRef<stop>/1],
      sourceOrRegistryBindings:[Sha256]
    }

`ControlDependencies/1` 是 Core 从实际受权读取形成的内部完整依赖集，不是 caller 输入。数组按完整 canonical bytes 排序且唯一；sourceOrRegistryBindings 只保存已有 owner 的 binding digest，不创造新作者或 Registry token。

    ControlPrepareBinding/1 = {
      key:StableControlKey/1,
      canonicalIntentBytes:Bytes,
      allocatedControlRefs:[ControlRef<K>/1],
      originalCommitRequest:PreparedCommitRequest/1,
      immutablePreview:ControlPreview/1,
      dependencyPins:ControlDependencies/1
    }

canonicalIntentBytes 是完整成功 closed-decode 后的 D3-CJ/3 canonical bytes，domain tag 固定 D10-Control-Intent/1。digest 可以作为索引，冲突判定必须比较完整 bytes。prepare binding 是稳定准备/定位记录，不是第二 author decision。

ControlDependencies/1 内部恰含本次实际读取的 config bindings、usage bindings、authority/fence/custody proof、authorization generations、stop refs 和必要 source/Registry bindings；客户端不能声明 complete。

共同顺序固定：

1. closed decode、D1 capability/release/surface/version/health 门；
2. 用最小受保护映射验证 current principal、scope、audience、对象观察和操作资格；未授权、wrong audience、不存在均 not_visible；
3. 验证 authority/fence/custody/continuity；
4. 查 stable key。visible same-key different canonical bytes → control_conflict；
5. same-key same-input 已有 authoritative decision 时，先重新验证当前主体对原结果和效果范围的披露资格，然后重放保存结果；此时不再用当前 target revision 否定历史成功；
6. 尚无 decision 才验证 expected config/usage revisions、当前授权、依赖、时间和预算，构造或恢复原 prepare；
7. Workspace 提交进入原 D6 transaction/ledger；Deployment 提交进入同一 store incarnation 的 closed host control transaction；
8. 原子保存效果、决议/receipt 关联、记录与账户增量、依赖失效信息、证据固定引用和必需的审计关联。

另一个合法请求已经把对象 r5→r6 后，原 r5 成功请求重试仍返回 r5 saved outcome；读取 current state 是另一个受权 read。当前主体若已经失去结果披露资格则 not_visible，但旧 decision 不能删除或改写。

失败 prepare/commit 不创建 applied decision。已存在的 prepare binding 可以在 transient state_unavailable 后用同一输入恢复；修改 expected revision/body/target 必须新 requestId。防重 binding、terminal proof、planned/unknown/uncertain pins 不能因普通 TTL 清理后让旧 requestId 再执行。

configuration revision 与 usageRevision 分域；checked increment 到 Counter 上限时 budget_exceeded，禁止 wrap/reset。retired id/incarnation 永不复用，防止 ABA。

## 9. Deployment value 与证据

    DeploymentValue/1 =
      {kind:"deployment_policy", value:DeploymentControlPolicy/1}
    | {kind:"trust", publisherId:Text, publicKey:Ed25519PublicKey,
       previous:Option<Binding<trust>/1>,
       proof:EvidenceTicket/1,
       claims:[NamespaceClaim/1]}
    | {kind:"package", manifest:PackageManifest/1,
       signature:Ed25519Signature}
    | {kind:"external_account",
       provider:ContributionBinding/1,
       externalAccountId:Text,
       endpointId:LocalOperationId,
       proof:EvidenceTicket/1}
    | {kind:"secret",
       account:Binding<external_account>/1,
       audience:ContributionBinding/1,
       usageKind:"authenticate",
       staged:SecretStageTicket/1}
    | {kind:"grant", spec:ResourceUseGrantSpec/1}
    | {kind:"cost_account",
       currency:CurrencyCode,
       ceiling:Counter,
       pricing:Option<Binding<pricing>/1>}
    | {kind:"pricing",
       account:Binding<external_account>/1,
       currency:CurrencyCode,
       fixedMicroUnits:Counter,
       meters:[PricingMeter/1],
       evidence:EvidenceTicket/1}

    NamespaceClaim/1 = {
      namespaceId:D4.SemanticNamespaceId,
      ownerClass:"first_party" | "publisher",
      ownerId:Token
    }

`NamespaceClaim/1` 只把已验证 PublisherIdentity 绑定到 D4 已有 semantic namespace owner claim；它不创建第二 namespace registry，也不能覆盖 `core`、`wf` 或其它保留 owner。

    PricingMeter/1 = {
      meterId:LocalOperationId,
      numerator:Counter,
      denominator:Counter,
      maxUnits:Counter
    }

`denominator` 和 `maxUnits` 必须为正；同一 pricing 内 meterId 按 ASCII 排序且唯一。最大费用使用 checked integer/rational arithmetic，不能使用 binary float。

EvidenceTicket/1：

    {ticketId:Token,
     evidenceClass:
       "publisher_rotation" | "namespace_claim" |
       "account_control" | "pricing_contract" |
       "final_bill" | "never_started",
     evidenceDigest:Sha256}

ticket 由受信 adapter 产生并在内部绑定完整原始证据、主体、scope、provider/account/attempt。普通 caller 不能自报 verified 或把一种 evidenceClass 当另一种使用。

Secret 原值只通过 trusted secret channel：

    D10SecretStageRequest/1 = {
      kind:"d10_secret_stage",
      wireVersion:1,
      requestId:Uuid,
      account:Binding<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretBytes:Bytes
    }

只有 H 可 stage。secretBytes 不进入普通 control canonical intent、preview、log、transcript 或 Workspace。成功返回：

    SecretStageTicket/1 = {
      ticketId:Token,
      account:ControlRef<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretVersionId:Token
    }

stage 自身使用同主体/store/requestId stable key；受信 secret store 只有在能够证明相同 exact secret input 时才重放原 ticket。普通数据库事务随后只能发布该 immutable ticket 指向的 secret version。

DeploymentControlDecision/1 是 host domain 成功结果：

    DeploymentControlDecision/1 = {
      key:StableControlKey/1,
      canonicalIntentBytes:Bytes,
      changes:[{
        ref:ControlRef<K>/1,
        beforeRevision:Option<Counter>,
        afterRevision:Counter
      }],
      evidence:[EvidenceTicket/1],
      auditRef:Token
    }

只有原子成功提交才保存 decision；preflight/authorization/CAS/evidence/overflow 失败不保存 applied decision。changes 恰覆盖配置变化，真实 no-op 为空。usage/account/reservation 变化和 decision 同事务保存。

## 10. 费用 reservation 与可恢复 settlement

    CostReservation/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      account:Binding<cost_account>/1,
      grant:Binding<grant>/1,
      pricing:Binding<pricing>/1,
      currency:CurrencyCode,
      upperBound:Money/1,
      revision:Counter,
      state:"reserved" | "uncertain" | "settled" | "released",
      actual:Option<Money/1>
    }

`actual` 仅在 state=settled 时为 some；其它状态必须为 none。

`attemptId` 与 `reservationId` 永不复用。

状态机：

    reserved -> settled(actual) | released | uncertain
    uncertain -> settled(actual) | released

settled 和 released 是终态；uncertain 是可恢复非终态并继续占用完整 upperBound。

released 仅在可靠 never_started 证据证明本 attempt 的 billable execution/send 从未开始时合法。已经发送、后来可靠最终账单为 0 必须 settled(0)，不是 released。可靠 final_bill 只可归属同 reservation/attempt/account/currency/pricing，得到 settled(actual)。例如 reserve100→uncertain→final bill20 必须 settled(20)，只返回 80。

    CostSettlementDecision/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      priorRevision:Counter,
      kind:"settled" | "released",
      actual:Option<Money/1>,
      evidence:EvidenceTicket/1
    }

`settled` 要求 `actual=some`；`released` 要求 `actual=none` 且 `evidenceClass=never_started`；`final_bill` 只能得到 settled。attempt、account 或 currency 不匹配、账单尚未 final、汇总账单无法唯一拆分，或证据连续性不足时，都保持 uncertain 并占用完整上限，返回 `state_unavailable`。已接纳适配器给出格式错误结果时沿其原 invalid-output 合同处理，不能猜测结论。

reconcile 用 expected reservation revision CAS。成功事务原子追加 CostSettlementDecision、更新 reservation state/revision、grant/account held/spent/available、evidence/audit。相同 evidence/prior revision/derived decision replay 不再次返额；并发不同 decision 至多一个 CAS winner。actual 超 upperBound 进入原 overcharge anomaly/freeze 路径，普通 reconcile 不提高 ceiling。

管理员无证据输入 0、effect idempotency、业务 rollback、author abort、TTL 或 Run terminal 都不是 released/settled 证据。ApprovalUse count、LeaseRunUse 和 cost reservation 继续三域独立。

## 11. Emergency stop 与线性化

公开 stop：

    D10EmergencyStopRequest/1 = {
      kind:"d10_emergency_stop",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:
        ControlRef<automation>/1
      | ControlRef<run>/1
    }

普通 owner 可停止自己精确拥有的 automation/run；W 限 Workspace；H 限 Deployment。stop 不要求目标 configuration revision，从而不会因为普通配置 Counter 饱和而失效；它只需要准确 id/incarnation 和当前 stop authority。

每个可执行对象首次 enable/admit 前必须预留一个持久 ExecutionStopLatch/1：

    ExecutionStopLatch/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal>,
      stoppedAtControlSequence:Option<Counter>
    }

open→stopped 单向，不支持 clear。重复 stop 是同效果幂等。stop 不消费 maxRuns、ApprovalUse、cost、普通管理 quota 或 executor budget。

线性化规则：

1. 新 Run admission 与 stop 在同一 store serialization domain。admission transaction 内比较 applicable latches 并原子写 occurrence claim/LeaseRunUse。stop 先提交则不准入；admission 先提交则次数保持已消费，但后续步骤继续检查 stop。
2. D6 最终 author commit 在实际持有写锁的最终 transaction 内重新验证受保护 RunBinding 对应全部 stop latch。不能只在 prepare 或 transaction 外 check。commit 先线性化则结果保留；stop 先线性化则该新 commit 不发生。
3. External transport 与 stop 使用同域 send fence。fence 内重验 latch、保存 ExternalEffectIntent/started evidence 与费用 hold，并保持 fence 到第一次不可撤回的真实发送交接完成；不能只入队后释放。之后等待远端响应不持有 fence。stop 先赢则不发送；send handoff 先赢则保留已发送事实，后续可能 outcome_unknown。
4. crash 后先恢复 latch、started/send evidence、reservation 和 decision，再恢复 executor；证据不足不得把可能已发送改成 cancelled。
5. stop 不阻断当前获权的 authoritative abort、cost settlement、evidence retention、audit retention 和 reference-safe cleanup；这些操作本身不恢复 executor 权限。
6. 临时 disable、Lease expiry、暂时撤权或普通 cancel 不是不可逆 abort 证明。

D6 coordinated amendment 增加 execution_stopped/preflight：尚未 planned 的 D10-bound author request 在 current authorization/ObservationScope 通过后被不可逆 stop 阻止时返回，不写 author decision。已 planned request 只有在 current authorization、continuity、完整 RunBinding 和不可逆 stop 全部证明后，原 D6 recovery 才能以 transaction_aborted/terminal 保存 authoritative abort；同事务按既有规则释放 ApprovalUse count。费用不自动释放。

## 12. D6 Policy/2 与 bootstrap profile/3 提案边界

固定 S 的 Policy/1 decoder、Policy/2 既有能力和所有已保存 policy/decision 保持。R05 proposed Policy/2 新增 closed no-argument capability d10_control_self，仅允许 workspace scope；它不被 Field/source/policy_admin 隐含，也不隐含其它权限。

固定 S profile/2 的“全部非 Field capability”在 amendment 中明确冻结为 S 当时集合：

固定集合为 `workspace_state, entity_state, locator_state, source_read, source_write, body_write, node_control, node_create, resource_read, resource_write, annotation_read, annotation_write, lifecycle, registry_admin, binding_admin, policy_admin, export, repair, audit, source_envelope_state, commit_sequence_state`。

profile/2 永远不自动包含以后新增的 d10_control_self。

新增 d6_bootstrap_profile wireVersion=3；其它 member shape 与 S profile/2 相同。profile/3 的 initial Policy 仍是 Policy/2，初始 creator grant 等于上面冻结集合 + d10_control_self，再加 S 原规则从 target Registry 生成的全部 Field read/write。deny 仍为空。

只有当前 administer_issuer 的显式 issuer profile update 才影响以后新 family。现存 family 保存的 profile/1 或 profile/2 副本、replacement、WorkspaceBootstrapPlan、saved decisions、replay/continue/failover 均不重算或补 grant。现存 Workspace 获取 d10_control_self 只能由当前 policy_admin 走原 Policy 修改路径显式安装；Field 权限主体不能自授。

## 13. Public capability 与首版 D6 错误兼容

D10 定义、D1 发现/发布的候选 capability IDs：

- automation.manage
- workspace.extensions.manage
- deployment.external.manage
- automation.stop
- automation.author_submit

这些 ID 必须先进入所选 D1 contractMajor 的正式 capability catalog，并继续经过所有真实 release/surface/policy/version/component/configuration/reachability/health 门。文档接受或 coordinated design activation 不使其自动 available。

第一份共同公开的 unattended author-submit 合同直接包含 D6 approval_unavailable/preflight 与 execution_stopped/preflight，不建立匿名 old/new D10 error profile。未知 capability ID 仍按 D1 unsupported_feature；不允许的组件版本组合按 D1 incompatible_version。固定 D6 Policy/1/2、bootstrap profile/1/2 和历史 saved decision decoder 不因删除未发布 D10 profile 文字而删除。

## 14. 原子性、audit 与保留

Workspace 控制改变若影响 author decision，必须进入原 D6 authority store transaction；Deployment host 控制只在同一个 store incarnation 的 closed host transaction 中改变 deployment records。不能依赖多个独立数据库、HTTP 服务或 ATTACH/WAL 猜测原子性。

每个成功受保护控制操作原子保存：record/config change、usage/account delta、decision/receipt linkage、dependency invalidation、evidence pins 和 required audit link。普通受保护执行在必要 audit start 无法耐久保存时 fail closed；emergency stop 使用 enable/admission 时预留的独立安全槽，不被普通 audit/budget quota 阻断。

retire/archive 不删除仍被 saved decision、planned recovery、unknown external effect、uncertain cost、evidence、tombstone/migration 或 active binding 引用的记录。容量不足可以拒绝新普通操作，不能通过删除防重历史让旧 requestId 再执行。

## 15. R05 验收反例

至少必须覆盖：

1. 窄 Field 用户拥有 d10_control_self、真实 Field 权限及管理员发放的 cost grant，可以建立自己的有限 automation；没有 grant 或自助 capability 时明确拒绝。
2. 同 stable key 的 r5 写已提交但响应丢失，另一个请求把对象改到 r6；原请求重试在当前披露授权通过后返回 r5 saved result，不重复建对象，也不因 r6 误判 stale。
3. same key 修改 target/body/expected revision 返回 control_conflict；旧 id 删除后同名重建不能命中旧请求。
4. grant revision/renewal 不清零 spent/held/attempts；换新 grant 也不抹旧 reservation/account liability。
5. 预留 100 后实际发送，崩溃恢复为 `uncertain`；同 attempt 的最终账单 20 得到 `settled(20)` 且只返 80；最终账单 0 得到 `settled(0)`；只有 never-started proof 才得到 `released`。
6. 两个 reconciler 并发同 reservation 只有一个 CAS winner；提交后丢响应 replay 不双返余额。
7. stop 与 Run admission、D6 final commit、external send fence 的每个线性化顺序；stop 后 settlement/authoritative abort/evidence cleanup 仍可执行。
8. profile/2 family 在软件升级后不获得 d10_control_self；显式 profile/3 只影响之后新 family；既有 Workspace 只能 policy_admin 显式授予。
9. connector contribution unavailable 不停用同 package 的 schema/template/data pack contribution。
10. Calendar/Library/People/Organizations 的 PackageId、module contribution、schema contribution 与 D4 namespace/Facet 一一映射但类型不混同；第三方同名 package 不取得 first-party D4 owner。
11. 设计文档 accepted 但 release 尚未交付、Mobile unsupported、policy deny、版本组合不允许或 provider unhealthy 时，D1 仍返回真实 unavailable reason。
12. 当前 caller 失去对象披露权后，同 stable key result 查询返回 not_visible，但保存 decision 不删除。
