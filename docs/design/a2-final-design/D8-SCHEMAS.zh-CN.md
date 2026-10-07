---
source_language: zh-CN
translation_status: source
---

[English](D8-SCHEMAS.md)
# A2 D8 Closed Schemas and Version Dispatch

状态：author-resolved-pending-independent-review。本文关闭 D8 current schema/version 路由，不扩大历史 decoder。完整 owner 语义见 D8 与 D8-INTERFACES；嵌入类型继续由 D2/D3/D4/D6/D7 owner 定义。

## 1. 当前 family 对照

| family | 当前作者/读取 family | 历史处理 |
| --- | --- | --- |
| Document 读取 | D8 outer wireVersion2 + D2DocumentSnapshot/3 | 旧 D2 snapshot 只按记录 decoder 分派 |
| Draft projection / text replace / write | D8 wireVersion2 | wire1 仅用于历史恢复 |
| Edit prepare | D8EditPrepareRequest/3 + D8EditInput/3 + PreparedEditBinding/3 | /1 与 /2 保留逐字节历史恢复 |
| D6 prepare/submit | InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 + d6_commit_request/2 | 先按记录的 owner version 分派 |
| Preview | EffectManifest/3 + EffectBytes/3 | Effect1/2 保持真实历史 family |
| Annotation author value | PortableAnnotationRecord/4 + D3-Annotation-Value/4 | Value3 与旧 Annotation record 保持历史语义 |
| Annotation read/draft | D8AnnotationRead*/1 + D8AnnotationDraft*/1 | 不做通用版本强制转换 |
| Presentation policy | logical policy/1 + immutable record/2 + SetRequest/2 | 旧 prototype 不建立 dual-read |
| View | D7 ViewSpec/1 | D8 不机械升版 |
| Search | 新 authoring 使用 D7 QuerySpec/2；QuerySpec/1 继续精确分派 | D8 不改写已保存 Query 的版本 |

Outer number similarity never implies inner schema upgrade.

## 2. Source target 与 Document response

~~~text
D8SourceTarget/2 = {
 ref:EntityRef,
 expectedObservation:SourceObservation/1
}

D8DocumentReadRequest/2 = {
 wireVersion:2,kind:"d8_document_read",
 workspaceRef:WorkspaceRef,
 commitDomain:CommitDomain/2,
 ownerNodeRef:NodeRef,
 budget:BudgetBinding/1
}

D8DocumentResponse/2 = {
 wireVersion:2,kind:"d8_document",
 workspaceRef:WorkspaceRef,
 commitDomain:CommitDomain/2,
 ownerNodeRef:NodeRef,
 sourceObservation:SourceObservation/1,
 documentRevisionToken:RevisionToken,
 domainFenceToken:Token,
 snapshot:D2DocumentSnapshot/3
}
~~~

Cross-fields：
ownerNodeRef/workspace/domain/sourceObservation/snapshot owner 必须一致；
managed token 必须认证到 sourceObservation.sourceVersion。
Snapshot/3 的 valid/invalid 分支保持 D2 owner exact。

## 3. Draft projection 形状

~~~text
DraftMapBinding/2 = {
 ownerNodeRef:NodeRef,
 baseObservation:SourceObservation/1,
 draftSerial:Counter,
 source:text
}

DraftEditMap/2 = {
 flows:[DraftFlow/2...],
 plainRegions:[PlainRegion/2...],
 sites:[DraftSite/2...]
}
~~~

DraftFlow/2 = {path:BodyPath,text,segments:[...]}; editable segment 的 sourceRange 非 null，
read-only escape/join/atom segment 不通过编辑 map 暴露 raw-scalar 写权限。

PlainRegion/2 = {sourceRange,text,parentPath,childStart,childEnd}；
text 是 raw exact slice，
允许 empty/blank/EOL-only。
DraftSite/2 = {sourceRange,parentPath,childIndex}，
sourceRange 为 zero width。

NavigationOrigins/2 与 DraftEditMap/2 分开；navigation segment relation=scalar|escape|join|atom 且 sourceRange 总是实际 origin。

## 4. Draft write 命令

~~~text
DraftTextReplaceRequest/2 = {
 wireVersion:2,kind:"d8_draft_text_replace",
 workspaceRef,commitDomain,ownerNodeRef,
 expectedSourceObservation,draftSerial,source,mapBinding,
 flowPath,segmentIndex,start,end,replacementText,budget
}

DraftWriteCommand/2 =
    {kind:"splice_plain",regionIndex,start,end,text}
  | {kind:"break_plain",regionIndex,offset}
  | {kind:"insert_at_site",siteIndex,text}
~~~

所有 ordinal/offset 是 proposal-local Counter；不能跨 serial/revision。text_replace 的 replacementText 禁 CR/LF；splice/insert 可携完整 CR/LF/CRLF。endpoint 必须 grapheme boundary 且 CRLF 原子。

## 5. D8EditIntent/3 与 prepare

~~~text
D8EditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     source:text}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     value:AnnotationEditableProposal/1,
     targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1}

D8PinnedEditIntent/3 =
    {kind:"document",target:D8SourceTarget/2,proposedSource:PinRef/2}
  | {kind:"annotation",target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2,targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2}

D8EditInput/3 = {
 kind:"d8_edit_input",version:3,
 invocationClass:"interactive_source_save"|"noninteractive",
 writeProtection:"strict"|"observed_only",
 intent:D8PinnedEditIntent/3,
 origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request}
}

D8EditPrepareRequest/3 = {
 wireVersion:3,kind:"d8_edit_prepare",
 workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
 saveProfile:"ordinary"|"complete",
 guarantee:"replica_local"|"managed_atomic",
 writeProtection:"strict"|"observed_only",
 intent:D8EditIntent/3,
 budget:BudgetBinding/1
}
~~~

Unknown arm/member/null rejects。
Annotation caller 不得携 actor/time/trusted/suggestion evidence。
reconfirm arm 不携 caller value。

## 6. PreparedEditBinding/3

~~~text
PreparedEditBinding/3 = {
 kind:"d8_prepared_edit_binding",version:3,
 workspaceRef,commitDomain,
 operationId:UUIDv4,
 principalAudienceToken:Token,
 inputDescriptor:InputDescriptor/3,
 intent:D8EditIntent/3,
 origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request},
 sourceInputs:InputDescriptor/3.sourceInputs,
 proposedInputs:[{entityRef:EntityRef,pin:PinRef/2}],
 registryInputs:[ValidatedCatalogContext...],
 dependencyProof:DependencyProof/3,
 observationProof:PreparedIntent/3.observationProof,
 budgetBinding:BudgetBinding/1,
 expiresAt:PreparedDeadline,
 request:d6_commit_request/2,
 preview:EffectManifest/3
}
~~~

proposedInputs 恰一项。
Document pin 是 exact UTF-8 source；
Annotation pin payloadKind=annotation_value 且 bytes=D3-CJ/3(complete Core Value/4)。
sourceInputs byte-equal inputDescriptor.sourceInputs。
OwnerInputBinding ownerKind/intentKind=d8_edit/3，
canonicalDescriptorBytes 是完整 D8EditInput/3。

## 7. Annotation 当前 read/draft

~~~text
D8AnnotationBodyRead/1 =
    {state:"absent",exactSource:null,semanticText:""}
  | {state:"valid",exactSource:text,semanticText:text}
  | {state:"invalid",exactSource:text,semanticText:null,
     diagnostics:[CoreDiagnostic/1...]}

D8AnnotationReadResponse/1 = {
 kind:"d8_annotation_read",version:1,
 workspaceRef,commitDomain,annotationRef,
 sourceObservation:SourceObservation/1,
 annotationRevisionToken:AnnotationRevisionToken/1,
 value:D3-Annotation-Value/4,
 targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
 body:D8AnnotationBodyRead/1
}

D8AnnotationDraftProjection/1 = {
 kind:"d8_annotation_draft",version:1,
 annotationRef,baseObservation,baseRevisionToken,draftSerial,
 access:"editable"|"readonly",
 value:AnnotationEditableValue/1,
 targetResolution,
 body:{state:"valid",source:text}|{state:"invalid",source:text,diagnostics:[...]}
}
~~~

Draft editable projection绝不包含完整 attribution authority。readonly 不可 prepare。

## 8. Annotation Value/4 与 R6 引用

D8 不复制 D3 owner schema，但 current D8 必须只消费：
- PortableAnnotationRecord/4；
- complete D3-Annotation-Value/4；
- AnnotationEditableValue/1 / AnnotationEditableProposal/1；
- AnnotationInlineBody/1：format asciidoc-inline、profile R6；
- current Suggestion/3 state/evidence；
- target five-arm union及 AnnotationAggregate current rules。

任何 Value3/plain_text 只在真实历史 decoder 中出现，current prepare/effects 不降级编码。

## 9. Effect preview / bytes

~~~text
EffectManifest/3 = {
 format:"weftext.effects",version:3,
 phase:"preview"|"committed",
 protocolOwner:"D3"|"D6",operationId:UUIDv4,
 workspaceRef,profile:"full"|"owner_fields",
 items:[EffectItem/3...],decisionKey:DecisionKey/2
}
~~~

EffectItem/3 保留既有 arm 并增加 typed presentation_policy_change。
EffectBytes/3 current encoding 使用 owner 已闭合集合，
其中包括 exact_source_utf8、resource_bytes 与 d3_annotation_value4。
还包括 d3_symbolic_result9、d7_definition_transfer_effects2 与 d3_canonical_effects1。
Workspace bootstrap 继续保留 1/3/4，并包括 d7_symbolic_json3 与 field_entries2。
D8 不增加自由 encoding。

## 10. Presentation policy 当前 schemas

~~~text
D8WorkspacePresentationPolicy/1 = {
 kind:"d8_workspace_presentation_policy",version:1,
 workspaceRef,revision,defaultPresentation:"separate"|"run_in"
}

D8WorkspacePresentationPolicy/2 = {
 kind:"d8_workspace_presentation_policy_record",version:2,
 workspaceRef,revision,
 parents:[D8PresentationPolicyAddress/1...],
 defaultPresentation:"separate"|"run_in",
 activationChangeId:ChangeId/1
}

D8PresentationPolicyHeadSet/1 = {
 kind:"d8_presentation_policy_heads",version:1,
 workspaceRef,stamp:{epoch:Token,revision:Counter},
 heads:[D8PresentationPolicyAddress/1...]
}

D8PresentationPolicySetRequest/2 = {
 wireVersion:2,kind:"d8_presentation_policy_set",
 workspaceRef,commitDomain,
 expectedFrontier:Frontier/2,
 expectedHeads:[D8PresentationPolicyAddress/1...],
 defaultPresentation:"separate"|"run_in",
 budget:BudgetBinding/1
}
~~~

address 的 recordSha256 绑定 prefixed canonical /2 bytes。parents/head arrays sorted unique。logical /1 只从唯一 current /2 record 机械投影，不单独持久化。final P 前没有 committed record/hash/pin/ChangeId。

## 11. Direction/layout 状态

这些是 D8 UI state，不是 author wire：
- DirectionPreference = ltr|rtl|auto
- CaretAffinity = upstream|downstream
- LayoutEpoch
- logical caret/selection anchor/focus
- composition generation/transaction
- device-local View interaction（hover/selection/legend hide/zoom/pan/fold）

不得编码进 QuerySpec、ViewSpec、Document source、Annotation Value 或 hidden sidecar。

## 12. Search/View 消费 schemas

D8 Search controls只生产 D7 compiler input，并消费 real QuerySpec/2/CanonicalGraph；不新增 D8 Search wire。保存 Query 仍是 D7 definition。

D8 renderer只消费 D7 ViewSpec/1={format:"weftext.view",version:1,inputSchema,layout,bindings,options}。
D8 不加入 filter/sort/CEL/aggregate/script member。
View validation error family保持 D7 owner。

View builder **没有 portable schema**。它的 working value 只是完整 strict-decoded `ViewSpec/1` 加 ephemeral UI selection/focus/validation state。Saved Query/View/DynamicBlock occurrence 继续是 current D2/D7 SavedDefinition author data；`DynamicBlock/1` 仍只保存真实 Query/View call 与 declared bindings。builder control availability、advanced-route 选择、validation message、open tab、selection、dirty state 或 device layout 都不得序列化进 ViewSpec/DynamicBlock 或隐藏 sidecar。

basic builder 只能从 D7 已拥有的 closed member 构造完整新 `ViewSpec/1`；不得合成 partial ViewSpec、丢弃“控件未知但 current 合法”的 member，也不得把 future/unknown version coercion 成 version 1。

## 13. Closed editor error family

~~~text
D8EditorError/2 = {
  wireVersion:2,
  kind:"d8_editor_error",
  code:
    "invalid_request"|"not_visible"|"authority_unavailable"|
    "domain_unavailable"|"integrity_conflict"|"source_unavailable"|
    "proof_unavailable"|"owner_update_required"|"stale_target"|
    "ambiguous_selection"|"unrepresentable_text"|
    "semantic_rejected"|"budget_exceeded"
}
~~~

该 family 是继承且 closed。current coordinated producer/path 若已有真实 owner unavailable/conflict 结果，不得把 owner_update_required 当通用 implementation 占位。

## 14. Error/version 矩阵

current D8 decoder 不能：
- 在 wireVersion2 prepare 上接受 /3 intent；
- 在 /3 prepare 上接受 Value3；
- 将 PreparedEditBinding/2 当 /3；
- 将 old EffectManifest/2 当 current /3；
- 将 D2 historical snapshot 当 Snapshot/3；
- 将 query_json 当 QuerySpec；
- 将 device direction/layout state 当 author member；
- 将 non-current Annotation plain_text 当 current R6。

每个 genuine old record先按记录 version/tag dispatch，
再执行其原 recovery；
unknown future version fail closed。
