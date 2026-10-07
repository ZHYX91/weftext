---
source_language: zh-CN
translation_status: source
---

[English](D7-SCHEMAS.md)
# A2 D7 当前 Schema 补充

状态：这是 D7 作者候选的当前 schema companion；不是实现，也不是独立接受。

## 1. 优先级

逐字节保留的 D7 作者 schema 位于 d7/owners。本 companion 只列出 fixed97 改变的当前 successor 分派，以及因此发生变化的跨字段规则。下文没有具名改变的 QuerySpec、ParameterSpec、TypedLiteral、TypeSpec、TerminalSchema、ViewSpec、QueryCall、CEL 与操作符成员，继续使用本地保留 owner 正文中的精确闭合结构。

## 2. 当前版本清单

```text
Query outer: wireVersion 2
QuerySpec current: version 2
QuerySpec retained decoder: version 1
ViewSpec: version 1
Action prepare: D7ActionPrepareRequest/3
Action author: D7ActionSpec/2
Action input: D7ActionInput/3
Prepared action: PreparedActionBinding/4
Proposed input: D7ProposedInput/3
Effects: EffectManifest/3 + EffectBytes/3
MinimumMapping: /3 retained
D3 fresh request: wire13
D6 fresh proof: InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3
```

外层 successor 的版本变化绝不意味着内层类型自动升版。历史字节必须先按记录时的 tag 分派，之后才能判断是否进入针对未见请求的当前验证。

## 3. 当前 Action 准备与输入

```text
D7ActionPrepareRequest/3 = {
  wireVersion:3, kind:"d7_action_prepare",
  workspaceRef:WorkspaceRef, commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  action:D7ActionSpec/2,
  selectedSources:[SourceVersionRef/1...],
  budget:BudgetBinding/1,
  evidenceToken?:Token
}

D7ActionInput/3 = {
  kind:"d7_action_input", version:3,
  action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext/1...],
  proposedInputs:[D7ProposedInput/3...]
}
```

D7ActionSpec/2 是 fixed-parent ActionSpec/1 的顶层 version 2：它保留 fixed-parent 中仍然合法的全部 intent，移除 fixed97 已经替代的历史 apply-suggestion 分支，并加入 fixed97 owner 定义的当前闭合 Annotation intent。version 2 不会顺带放宽任何其它 Action intent。

selectedSources 使用既有 SourceVersionRef profile，但它本身不证明当前性；当前性来自 prepare 阶段绑定的完整 Observation 与 proof。

## 4. PreparedActionBinding/4

```text
PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding", version:4,
  bindingToken:Token, protocolOwner:"D3"|"D6",
  operationId:UUIDv4, workspaceRef:WorkspaceRef,
  principalAudienceToken:Token, action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext/1...],
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  constructionInput:null|TemplateConstructionInput/2,
  proposedInputs:[D7ProposedInput/3...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,
  expiresAt:<D6 protected deadline>,
  request:D3IdentityOperationRequest/13|d6_commit_request/2,
  preview:<complete EffectManifest/3>,
  resolutionInput:null|D3ResolutionInput/1
}
```

D7ActionInput 中的六个成员分别与 PAB 中对应成员逐字节相等。对于 D6 所有的当前 Action，D6 ownerInput 的 canonical descriptor 必须是精确的 D7ActionInput/3；其 pinRefs 只覆盖实际受保护的 proposed 以及 source/definition/rule 证据，并遵守 D6 的规范排序。D3 所有的 identity Action 保留 D3 自己的 owner descriptor；PAB4 只是跨 owner 的准备记录，不会建立第二份 D3 请求权威。

bindingToken 是随机生成并受保护的关联材料，不从 request 字节推导，因此不存在 request-hash 自引用环。原始 request、binding、preview、proof、pins、plan 和最终 P 仍属于同一个 decision。

## 5. D7ProposedInput/3

```text
D7ProposedInput/3 = {
  subject:PayloadSubjectKey,
  payloadKind:
    "exact_source_document"|"resource_bytes"|"annotation_value",
  encoding:
    "exact_source_utf8"|"resource_bytes"|
    "d3_annotation_value4"|"d3_symbolic_result9",
  pin:PinRef/2
}

legal pairs:
  exact_source_document <-> exact_source_utf8
  resource_bytes        <-> resource_bytes
  annotation_value      <-> d3_annotation_value4
  annotation_value      <-> d3_symbolic_result9
```

D6 当前的 concrete Annotation 后像使用完整 Value/4 字节；symbolic Result/9 分支只在既有 D3 分支中合法。d3_annotation_value3 在 Input3 中非法。真实历史 Input2/PAB3 继续保留 annotation_value3 与原 pins，绝不能被升级重编码。

## 6. EffectManifest/3 与 EffectBytes/3

```text
EffectManifest/3 = {
  format:"weftext.effects", version:3,
  phase:"preview"|"committed",
  protocolOwner:"D3"|"D6", operationId:UUIDv4,
  workspaceRef:WorkspaceRef, profile:"full"|"owner_fields",
  items:[EffectItem/3...], decisionKey:DecisionKey/2
}

EffectBytes/3 = {
  handleToken:Token,
  encoding:
    "exact_source_utf8"|"resource_bytes"|"d3_annotation_value4"|
    "d3_symbolic_result9"|"d4_relation_copy_effects1"|
    "d4_source_materialization_effects1"|
    "d7_definition_transfer_effects2"|"d3_canonical_effects1"|
    "d6_workspace_bootstrap_plan1"|"d6_workspace_bootstrap_plan3"|
    "d6_workspace_bootstrap_plan4"|"d7_symbolic_json3"|"field_entries2",
  byteLength:Counter
}
```

EffectItem/3 保留 predecessor 中仍然有效的 effect 分支，以及 fixed97 选定的当前 owner-specific successor 分支。所有字节槽都使用 EffectBytes/3；decoder 不能通过递归猜字段名来发现字节槽。preview 与 committed 是两个不同 phase；delivery epoch 只授权本次传输，不会改变语义 effect bytes。

## 7. Source plan 与 D6 当前类型

```text
source-plan dispatch:
  ordinary/fresh managed source         -> SourceRevisionPlan/1
  guarded D3 canonical conflict         -> SourceRevisionPlan/2
  D6 source_merge / choose_source_head  -> SourceRevisionPlan/3

current production address:
  SourceVersion/2

current receiver qualification:
  SourceObservation/1

portable current D6:
  DependencyKey/3
  DependencyProof/3
  InputDescriptor/3
  PreparedIntent/3
  InstallationNotice/3
  ContentCompletionProof/4
  ChangeRecord/1
```

即使其中包含的 production version 相同，SourceVersion 与 SourceObservation 仍属于不同数据域。一次新的读取可以为同一个已认证 production version 建立新的当前 Observation，但不能因此改写 saved Locator、DefinitionAddress、已经准备好的 selector、Query result 或 ActionEvidence。

## 8. 严格 JSON 与数值解码

所有闭合 object 都拒绝重复 key、未知成员、缺少必需成员、非法 null 和跨 union 分支成员。除非某个精确 schema 明确另有规定，Optional 表示成员缺席。JSON Boolean 不能当作整数。

D7 integer/decimal 继续使用既有 Value/CEL profile 的精确规范文本 decoder；D3/D6 Counter 及其它有界 meta-wire integer 继续使用各自的范围和 overflow 规则。外层 current version 不能让内层 decoder 静默接受原本非法的指数形式、host 浮点、negative zero 或不同的嵌套数值表示。反过来，current heading 的 effectiveLevel 等任意精度 D7 integer 也不能仅因为 host 或历史 schema 使用 int64 就被拒绝。

## 9. Query 执行闭合

Query outer 继续使用 wireVersion2。QuerySpec/2 是当前作者 schema：它以保留的 QuerySpec/1 grammar 为基础，只增加 D7-QUERY-V2 明确列出的版本化能力，包括可空的当前 title/subtitle、D6-owned Node 文件名与路径、Resource 文件名，以及 union_all。FileBinding source 只消费 D6FileBindingMetadataObservation/1，并使用既有 entity_state+locator_state gate；绝不隐含获得 source_read/resource_read/structure_state。QuerySpec/1 仍由独立 decoder 按原规则解释，绝不原地扩宽。union_all 只用 D7-QUERY-V2 的准确 inputOrdinal constructor 扩展 retained internal K tree，并在显式 sort 之前保持 unordered。DAG 验证、规范 ordinal、CEL 类型检查、feature gate、来源资格、terminal schema、结果编码、分页/订阅重置和完整错误顺序继续组成一条闭合执行链；未知 feature 或不可用 dependency 都不能伪装成空 bag。

## 10. 搜索 schema 边界

Search 不新增第二持久 Search 或 View schema。普通搜索、可视化条件与可选快捷语法都确定性编译到 QuerySpec/2，并绑定 current SearchContribution dependency。D7-SEARCH 的 ephemeral SearchConditionAst/1 只作为 compiler IR，绝不持久化。保存时只保存 canonical Query definition。shortcut parser version、text cursor、open filter popover、recent query、expansion state 与 device direction 都是 interaction state，不进入 DynamicBlock 或 saved Query。

详细语法与 SEARCH-01 至 SEARCH-08 验收义务位于 D7-SEARCH。

## 11. 历史恢复

saved、planned、unknown、committed 以及传输恢复路径都必须先定位精确的原 owner/version，再应用针对未见请求的当前门禁。旧 request fingerprint、MinimumMapping、pins、tokens、clock、custody、installation proof、outbox、effect bytes 和 unknown 责任全部按实际产生它们的版本保留。当前 schema 不会为无法证明实际部署过的 prototype 虚构永久 migration。
