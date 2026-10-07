---
source_language: zh-CN
translation_status: source
---

[English](D7-SCHEMAS.md)
# A2 D7 Current Schema Overlay

状态：D7 作者候选的 current-schema companion；不是实现或独立接受。

## 1. 优先级

byte-complete 的保留 D7 author schema 位于 d7/owners。本 companion 只具名 fixed97 改变的 current successor dispatch 与 cross-field rule。下文没有改变的 QuerySpec/ParameterSpec/TypedLiteral/TypeSpec/TerminalSchema/ViewSpec/QueryCall/CEL/operator member，继续使用本地保留 owner 正文中的 exact closed shape。

## 2. Current version inventory

```text
Query outer: wireVersion 2
QuerySpec: version 1
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

outer successor 永远不意味着 inner successor。historical bytes 必须先按 recorded tag 分派，之后才可能进入 unseen-current validation。

## 3. Current Action prepare 与 input

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

D7ActionSpec/2 是 fixed-parent ActionSpec/1 顶层 version 2：保留所有合法 fixed-parent intent，但移除 fixed97 已替换的 historical apply-suggestion arm，并加入 fixed97 owner 定义的 current closed Annotation intents。version 2 不放宽其它 Action intent。

selectedSources 使用 retained SourceVersionRef profile，本身不证明 currentness；currentness 来自 prepare 绑定的完整 observation/proof。

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

D7ActionInput 的六个 member 分别与 PAB 对应 member byte-equal。D6-owned current action 的 D6 ownerInput canonical descriptor 必须是 exact D7ActionInput/3，pinRefs 只覆盖受保护 proposed 与实际 source/definition/rule evidence，并遵循 D6 排序。D3-owned identity action 保留 D3 owner descriptor；PAB4 只是 cross-owner preparation，不创建第二 D3 request authority。

bindingToken 是随机受保护 association material，不从 request bytes 推导，因此不存在 hash cycle。original request、binding、preview、proof、pins、plan 与 final P 仍是一个 decision。

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

D6 current concrete Annotation after 使用完整 Value/4 bytes；symbolic Result/9 arm 只在 retained D3 branch 合法。d3_annotation_value3 在 Input3 中非法。真实 historical Input2/PAB3 继续保留 annotation_value3 与原 pins，绝不升级。

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

EffectItem/3 保留 predecessor effect arms 与 fixed97 选定的 current owner-specific successor arms。每个 byte slot 使用 EffectBytes/3，decoder 不通过递归猜 member name 来发现 slot。preview/committed 是不同 phase；delivery epoch 只授权 transport，不改变 semantic effect bytes。

## 7. Source plan 与 D6 current type

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

即使 contained production version 相同，SourceVersion 与 SourceObservation 仍是不同 domain。fresh read 可以为同一 authenticated production version 建立新的 current Observation，但不能改写 saved Locator、DefinitionAddress、prepared selector、Query result 或 ActionEvidence。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 8. Strict JSON 与 numeric decode；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

全部 closed object 拒绝 duplicate key、unknown member、missing required member、非法 null 与 cross-arm member。除非 exact schema 明确另有定义，Optional 表示 member absence。JSON Boolean 不能当 integer。

D7 integer/decimal 保持 retained Value/CEL profile 的 exact canonical textual decoder；D3/D6 Counter 与其它 bounded meta-wire integer 保持自己的 range/overflow rule。outer current version 不能 silent 接受 inner decoder 原本拒绝的 exponent、host floating value、negative zero 或 nested numeric representation。反过来，current heading effectiveLevel 等 arbitrary-precision D7 integer 也不能因 host 或 historical schema 使用 int64 就被拒绝。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 9. Query execution closure

Query outer 仍为 wireVersion2，完整 QuerySpec/1 author grammar 位于 d7/owners/query-algebra。DAG validation、canonical ordinal、CEL typing、feature gate、source qualification、terminal schema、result encoding、paging/subscription reset 与完整 error order 共同形成一条 closed path。unknown feature 或 unavailable dependency 不能被转换为空 bag。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 10. Search schema boundary

Search 不新增 persistent Query/View schema。plain search、visual condition 与 optional shortcut grammar 都编译为 ordinary QuerySpec/1，并绑定 current SearchContribution dependency。保存时只保存 canonical Query definition。shortcut parser version、text cursor、open filter popover、recent query、expansion state 与 device direction 都是 interaction state，不进入 DynamicBlock 或 saved Query。

详细 grammar 与 SEARCH-01 到 SEARCH-08 acceptance 位于 D7-SEARCH。

## 11. Historical recovery

saved、planned、unknown、committed 与 transport-recovery 都先定位 exact original owner/version，再应用 current unseen gate。旧 request fingerprint、MinimumMapping、pins、tokens、clock、custody、installation proof、outbox、effect bytes 与 unknown responsibility 全部按原产生版本保留。current schema 不为无法证明实际部署的 prototype 声明 permanent migration。
