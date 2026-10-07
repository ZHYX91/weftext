---
source_language: zh-CN
translation_status: source
---

[English](D6-SCHEMAS.md)

# A2 D6 当前闭合 Schema

状态：A2 D6 schema companion；current closed shapes 从 fixed97 owner replacement 精确承接。它不是第二语义 authority、实现、迁移或独立接受。historical decoder 继续由 D6-CONTROL 按 recorded bytes 分派，绝不原地扩展。

# 5. Managed format 与 D6 current successor

```text
ManagedDocumentFormatProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  managedProfile:"weftext_managed/1"
}

ManagedDocumentFormatBinding/1 = {
  kind:"weftext_managed_document_format",
  version:1,
  ownerNodeRef:NodeRef,
  bindingRevision:Counter,
  profile:ManagedDocumentFormatProfile/1
}

DocumentFormatDependencyKey/1 = {
  kind:"document_format",
  workspaceRef:WorkspaceRef,
  ownerNodeRef:NodeRef
}

DocumentFormatCurrentQualification/1 = {
  kind:"d2_document_format_current_qualification",
  version:1,
  key:DocumentFormatDependencyKey/1,
  stamp:{epoch:Token,revision:Counter},
  componentImage:{state:"present",version:Counter,
                  byteLength:Counter,sha256:"64-lowercase-hex"},
  binding:ManagedDocumentFormatBinding/1,
  bindingPin:PinRef/2
}

ManagedDocumentSemanticQualification/1 = {
  sourceObservation:SourceObservation/1,
  documentFormat:DocumentFormatCurrentQualification/1
}
```

BaselineOnly不是binding arm。fresh/copy/fork fresh identity用revision1；formal same-Workspace restore恢复exact historical binding；普通backup重新admission；真实managed profile migration checked +1。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

## 5.1 Dependency family

```text
DependencyKey/3 =
  source(rank0) |
  document_format(rank1) |
  lifecycle(2) | placement_range(3) | ref_inbound(4) |
  relation_incidence(5) | calendar_scope(6) | registry(7) |
  temporal_rules(8) | authorization(9) | foreign_binding(10) |
  query_scan(11) | replica_registry(12) | conflict_record(13) |
  execution_resource(14)
```

除新增document_format外，其余arm字段与fixed-e8aa DependencyKey/2 exact对应arm逐成员相同。

```text
DependencyProof/3 = {
  kind:"d6_dependency_proof",
  version:3,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  baseFrontier:Frontier/2,
  entries:[{
    key:DependencyKey/3,
    stamp:{epoch:Token,revision:Counter},
    evidencePins:[PinRef/2...]
  }...]
}

InputDescriptor/3 = {
  kind:"d6_input_descriptor",
  version:3,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  intentKind:text,
  saveProfile:"ordinary"|"complete"|"control_only",
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  frontierPolicy:"exact"|"scope_dependencies",
  observationScope:ObservationScope/2,
  sourceInputs:[{
    entityRef:EntityRef,
    observation:SourceObservation/1,
    role:"before"|"dependency"
  }...],
  controlInputs:[{
    key:DependencyKey/3,
    stamp:{epoch:Token,revision:Counter}
  }...],
  ownerInput:OwnerInputBinding/2
}

PreparedIntent/3 = {
  kind:"d6_prepared_intent",
  version:3,
  planToken:Token,
  operationId:Uuid,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  beforeCut:Frontier/2,
  proposedState:SemanticState/1,
  mutationFootprint:MutationFootprint,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  budgetBinding:BudgetBinding/1,
  pinDirectory:PinDirectory,
  installationPlan:InstallationPlan,
  inputRetentionState:InputRetentionState,
  expiresAt:PreparedDeadline,
  previewBinding:PreviewBinding
}
```

最后六个嵌套名使用fixed-e8aa PreparedIntent/2的exact closed decoder和语义；本successor没有改变它们的成员，只改变Descriptor/Proof family。planToken current tag=d6_plan/3。

## 5.2 Portable component / Notice / CP

```text
PortableComponentKey/2 =
  document(rank0) |
  document_format(rank1) |
  resource(2) | annotation(3) | node_binding(4) |
  child_list(5) | lifecycle(6) | trash_membership(7) |
  policy(8) | registry(9) | period_scope(10) |
  replica_registry(11) | conflict(12)

document_format =
  {kind:"document_format",ownerNodeRef:NodeRef}

InstallationNotice/3 = {
  format:"weftext.installation-notice",
  version:3,
  decisionKey:DecisionKey/2,
  guarantee:"replica_local"|"managed_atomic",
  writeProtection:"strict"|"observed_only",
  baseFrontier:Frontier/2,
  components:[{
    key:PortableComponentKey/2,
    before:ComponentImage/1,
    after:ComponentImage/1
  }...]
}

ContentCompletionProof/4 =
    {format:"weftext.content-completion",version:4,
     outcome:"committed",
     decisionKey:DecisionKey/2,changeId:ChangeId/1,
     guarantee:"replica_local"|"managed_atomic",
     writeProtection:"strict"|"observed_only",
     semanticState:SemanticState/1,
     frontierBefore:Frontier/2,frontierAfter:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...],
     sourceChanges:[{
       entityRef:EntityRef,
       before:SourceVersion/2|"absent",
       after:SourceVersion/2|"absent"
     }...],
     receiptDigest:"sha256:64-lowercase-hex"}
  | {format:"weftext.content-completion",version:4,
     outcome:"restored",
     decisionKey:DecisionKey/2,
     baseFrontier:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...]}
```

ComponentImage/1 与 PinRef/2 保持 fixed-e8aa 的精确结构。document_format 的 present component bytes 必须是 D3-CJ/3(binding)，且 ComponentImage.version=bindingRevision。

Notice3 除 component-key decoder 升为 PortableComponentKey/2 外，逐项继承 Notice2 invariant：components 非空、按 fixed-rank/canonical-key 排序且唯一，notice 在 install 前冻结并保留原 baseFrontier。CP4 除 component-key decoder 与 current ChangeRecord/1 linkage 外，逐项继承 CP3 committed/restored invariant。committed CP4 的 components 与 Notice3 key 集合及顺序严格相同，每个 after 都是实际 installed/sealed image；sourceChanges 是完整、按 EntityRef 排序且唯一的真实 source-state delta；receiptDigest 绑定原 receipt。fresh managed Document 必须在同一 plan/P/CP4 中同时带 document 与 document_format。format-only 且 source 不变时 sourceChanges=[]，不产生 SourceRevisionPlan、managed SourceVersion 或 H advance。

## 5.3 ChangeRecord

```text
ChangeRecord/1 = {
  format:"weftext.change-record",
  version:1,
  decisionKey:DecisionKey/2,
  changeId:ChangeId/1,
  installationNotice:{
    format:"weftext.installation-notice",version:3,
    byteLength:Counter,sha256:"64-lowercase-hex"
  },
  completionProof:{
    format:"weftext.content-completion",version:4,
    byteLength:Counter,sha256:"64-lowercase-hex"
  },
  frontierBefore:Frontier/2,
  frontierAfter:Frontier/2
}
```

CP4 必须 committed 且 DecisionKey/ChangeId/frontiers 与 ChangeRecord 逐项相等；Notice3/CP4 byteLength/digest 必须匹配 exact D3-CJ/3 canonical bytes。同一 P seal 固定 ChangeId、CP4 与 ChangeRecord；publication 只重发原 pin。

frontierBefore 是实际 verified pre-seal Frontier；frontierAfter 必须恰好由 frontierBefore 增加本 ChangeId，其他 domain 不回退。frontierPolicy=exact 时，frontierBefore 与原 expected/base/notice Frontier byte-equal。scope_dependencies 时，还必须从 Notice3.baseFrontier 到 frontierBefore 再到 frontierAfter 证明完整连续、已验证的 ChangeRecord/completion chain，并保留原 unrelatedness proof；vector number、provider sync 状态或当前文件均不能替代。

restored CP4 只允许 decisionKey、baseFrontier，以及每项都等于对应 Notice3 before image 的 component after images；禁止 changeId、guarantee、writeProtection、semanticState、frontierBefore、frontierAfter、sourceChanges、receiptDigest 和全部成功语义。

receiver admission 必须 strict-decode exact Notice3/CP4 version 与 canonical bytes，验证 component key 集合/顺序严格一致，取得并验证每个实际 component byte 与 owner version，验证完整 production SourceVersion changes，并证明连续 chain。present document_format component 还必须 strict-decode exact ManagedDocumentFormatBinding/1 bytes。缺失/未知 bytes 或 decoder、component-version mismatch、Notice/CP/ChangeRecord mismatch 都只能是 incomplete/proof_unavailable，不能成功 admission。

## 7.1 Caller proposal 与 Node-local physical aggregate

```text
SuggestionAuthorProposal/1 = {
  kind:"replace"|"delete"|"insert",
  replacementSource:null|text
}

AnnotationEditableProposal/1 = {
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:SuggestionAuthorProposal/1|null
}

AnnotationAggregate/1 = {
  format:"weftext.annotations",version:1,
  ownerNodeRef:NodeRef,
  records:[PortableAnnotationRecord/4...]
}

AnnotationAggregateObservation/1 = {
  kind:"d6_annotation_aggregate_observation",version:1,
  workspaceRef:WorkspaceRef,
  ownerNodeRef:NodeRef,
  observerDomain:CommitDomain/2,
  fileObjectBinding:FileObjectBinding/1,
  aggregateBytesPin:PinRef/2|null
}

AnnotationAggregateInstall/1 = {
  kind:"d6_annotation_aggregate_install",version:1,
  ownerNodeRef:NodeRef,
  before:AnnotationAggregateObservation/1,
  after:"absent"|PinRef/2,
  installCapability:FileInstallCapability/2,
  changedAnnotationRefs:[AnnotationRef...]
}
```

SuggestionAuthorProposal/1 只承载作者可提议的 kind/replacementSource。replace 必须带 replacementSource（允许空串），delete 必须为 null，insert 必须带非空内容。state、confirmation、targetBasisSha256、expectedText、pointAffinity 永远不属于 caller proposal。AnnotationEditableProposal/1 的 purpose、reply、body、appearance、labels 与 review 约束继续与 AnnotationEditableValue/1 相同；Core 通过操作类别 gate 决定 Suggestion/3 受控的 before→after。gate 使用派生 current-state class，而不把 null 当成 fresh absence：fresh create=`absent_annotation`；已有 suggestion=null=`no_suggestion`；pending=`pending_confirmed|pending_needs_reconfirmation`；terminal=`terminal_accepted|terminal_rejected`。interactive_create 接纳合法 no-suggestion comment/mark/reply 与合法 root suggestion；ordinary_edit 接纳 no_suggestion→no_suggestion、真实目标资格后的 no_suggestion→pending+needs_reconfirmation、pending→pending、pending→no_suggestion，以及 terminal→同一 terminal only；manual_reattach 只接纳 no_suggestion→no_suggestion 或 pending→pending+needs_reconfirmation，不能和类别转换合并。target 不变的 no-suggestion 编辑以及 pending→no_suggestion 清除不会凭空获得 target-source-read 权限；真正消费目标内容的转换/操作只在各自具名 qualification/read 阶段读取。完成 gate 后 Core 才展开保留 before attribution 的完整 candidate Value/4 并与 before 比较 canonical bytes；Proposal bytes 永远不是 no-op comparator。

`weftext.annotations.json` 的 current physical bytes 恰为 D3-CJ/3(完整 AnnotationAggregate/1)，无 BOM、无额外换行或第二 envelope。records 必须非空，按完整 AnnotationRef canonical bytes 升序且唯一；每个 annotationRef.owner 必须等于 ownerNodeRef，nonnull replyTo 必须同 owner 且整张 records reply graph 无环。空集合唯一物理表示是 sidecar absent，禁止同时保留空 aggregate。unknown/missing member、duplicate JSON key、unknown version、owner mismatch、duplicate Ref 或 reply cycle 都使整个 aggregate strict-decode 失败；normal current read 不能部分信任其中“看起来合法”的记录。

AnnotationAggregateObservation/1 是物理文件观察，不是 Annotation 的 SourceVersion/SourceObservation、revision token 或 identity。这里的 `FileObjectBinding/1` 与 `FileInstallCapability/2` **逐字复用 fixed D6 Control Interfaces §2**，不是本候选新版本：absent binding=`{kind:"absent",backendToken,relativePath,observationEpoch,parentGenerationToken}`；present binding=`{kind:"present",backendToken,relativePath,observationEpoch,objectGenerationToken,byteLength,sha256}`；capability 仍只有 create_only(parentGenerationToken)、conditional_replace(expectedObjectGenerationToken)、exclusive_write_window(windowToken,expectedObjectGenerationToken)、observed_replace(observedObjectGenerationToken)。PortableRelativePath、Token/Counter、64-lowercase-hex 及全部 containment/continuity 规则保持该 owner 原义。fileObjectBinding.kind=absent 时 aggregateBytesPin 必须 null；kind=present 时 aggregateBytesPin 必须是 payloadKind=portable_metadata 且 bytes 等于上述完整 physical JSON 的 PinRef/2。digest 单独永远不是 CAS。AnnotationAggregateInstall/1 只是现有 D6 安装计划的一个具名 file-write coordination binding，不创建新 ledger/CAS/author store；before 必须是 fresh Observation，而不是裸 `"absent"`。after 为 PinRef 时指向完整 after aggregate，为 `"absent"` 时表示删除最后一个 record。installCapability 必须与 before FileObjectBinding 精确交叉：absent→create_only 且 parentGenerationToken 相等；present→原 D6 允许的 replace capability 且 generation/window token 精确匹配。managed_atomic strong path 仍只接受该 owner 的 strict capability；observed_replace 仅在 D6 §4.1 原本 observed_only eligibility 已成立时有效，绝不升级成 CAS。changedAnnotationRefs 按 Ref canonical bytes 排序唯一，并恰等于 before/after logical record 差集。一个 Node 在一个 DecisionKey 下最多一个 AnnotationAggregateInstall/1。

# 8. SourceTransform

```text
PortableTransformCompilation/1 =
    {kind:"representable",
     events:[SourceTransformPortableEvent/3...],
     afterSourceSha256:"sha256:<64 lowercase hex>"}
  | {kind:"unavailable",
     reason:"provenance_gap"|"unsupported_transaction"|
            "invalid_utf8_boundary"|"generated_cross_anchor_edit"|
            "boundary_slot_unrepresentable"|"provenance_cycle"|
            "after_replay_mismatch"|"payload_digest_mismatch"}

SourceTransformPortableEvent/3 =
    {kind:"replace",
     startByte:Counter,endByte:Counter,
     removedByteLength:Counter,
     removedSha256:"sha256:<64 lowercase hex>",
     replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}
  | {kind:"insert",
     atByte:Counter,
     replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}

TransformEmissionPlan/1 =
    {kind:"disabled",
     reason:"no_exact_core_edit_plan"|"transform_profile_unavailable"}
  | {kind:"required",
     profile:"d6_source_transform_seal/1",
     expectedTrustRevision:Counter,
     expectedTrustKeyId:"sha256:<64 lowercase hex>"}

CoreSourceEditPlan/2 = {
  kind:"d6_core_source_edit_plan",
  version:2,
  decisionKey:DecisionKey/2,
  ownerNodeRef:NodeRef,
  beforeObservation:SourceObservation/1,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,
  transformEmission:TransformEmissionPlan/1
}

SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",
  version:2,
  decisionKey:DecisionKey/2,
  changeId:ChangeId/1,
  ownerNodeRef:NodeRef,
  before:SourceVersion/2,
  after:SourceVersion/2,
  beforeSourceSha256:"sha256:<64 lowercase hex>",
  afterSourceSha256:"sha256:<64 lowercase hex>",
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}

SourceTransformSealSignedBody/1 = {
  format:"weftext.source-transform-seal",
  version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2
}

SourceTransformSealArtifact/1 = {
  format:"weftext.source-transform-seal",
  version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2,
  signature:"<86 ASCII unpadded base64url>"
}

SourceTransformSealKey/1 = {
  changeId:ChangeId/1,
  ownerNodeRef:NodeRef
}

SourceTransformSealOutboxItem/1 = {
  kind:"d6_source_transform_seal_outbox_item",
  version:1,
  key:SourceTransformSealKey/1,
  artifactPin:PinRef/2
}
```

plan/evidence edits 的 D3-CJ/3 必须逐字节相等；seal 不得重编译、重排或合并。required 决策密封后恰有一个 outbox item，disabled 则为零；artifactPin 保存 portable_metadata 的精确规范 artifact bytes。

Event3 必须针对精确的 beforeBytes/afterBytes 做闭合校验。replace 要求 0<=startByte<endByte<=before length，两端都位于 UTF-8 scalar boundary；removedByteLength=endByte-startByte，且 removedSha256=SHA-256(beforeBytes[startByte:endByte])。
insert 要求 0<=atByte<=before length、atByte 位于 UTF-8 scalar boundary，且 replacementByteLength 非零。
replacement interval 必须组成两两不重叠的 maximal island；同一点的 insert 按 transaction order 合并。严格位于 replacement island 内的 insert 必须折入该 island，否则 compilation unavailable。
规范边界顺序固定为：在 p 结束的左 replacement、insert@p、从 p 开始的右 replacement。

SourceTransformEvidence/2.beforeSourceSha256 必须恰为 "sha256:" + lowercase_hex(SHA-256(evidence.before 与 ownerNodeRef 对应的完整精确 before-source bytes))。receiver 必须从 producing CP/history 与 retained version evidence 取得这份历史 bytes，重新计算 whole-source digest，并在接受 artifact 前要求相等。Event removedSha256/length、slice replay 成功、SourceVersion 字段相等或 current-file bytes 都不能替代。缺 exact-before bytes 时沿用既有 proof/state-unavailable 边界；已证明 digest 矛盾则属于 integrity failure。该交叉字段不新增成员，也不提升 SourceTransformEvidence/2 版本。

generatedOutputSpan 使用 SPEC §11.1 的 replay-cursor 算法处理精确的 before/after pins。每段未变化间隙都要逐字节比较，当前 event span 恰为 [afterCursor,afterCursor+replacementByteLength)，并用该 after slice 验证 replacement 的长度与 hash。
replace 将 beforeCursor 推到 endByte；insert 保持在 q。最后剩余字节与 afterSourceSha256 都必须匹配，禁止内容搜索。
mapping 定义 delta(replace)=replacementByteLength-(endByte-startByte)，并使用有溢出检查的有符号运算。

SourceTransformSealSignedBody/1 恰为 SourceTransformSealArtifact/1 只删除 signature。signature 必须恰为 86 个 ASCII unpadded-base64url 字符，解码后是 64-byte Ed25519 signature。
待签消息恰为 ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1)。完整传输/存储 artifact bytes 恰为含 signature 的 SourceTransformSealArtifact/1 的 D3-CJ/3；其它 JSON serialization 必须拒绝。

# 9. D6 dual-profile trust

```text
SealProfileId/1 =
  "d6_revision_token_seal/1" |
  "d6_source_transform_seal/1"

WorkspaceTrustPredecessor/2 =
    {kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}
  | {kind:"declaration",revision:Counter,
     sha256:"sha256:<64 lowercase hex>"}

WorkspaceTrustDeclaration/2 = {
  kind:"d6_workspace_trust_declaration",
  version:2,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,
  decisionKey:DecisionKey/2,
  action:
      {kind:"authorize",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>"}
    | {kind:"rotate",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,
       oldTrustKeyId:"sha256:<64 lowercase hex>",
       newTrustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>",
       continuitySignature:"<86 ASCII unpadded base64url>"|"not_required",
       mode:"ordinary"|"loss_recovery"|"compromise"}
    | {kind:"revoke",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,
       trustKeyId:"sha256:<64 lowercase hex>",
       mode:"administrative"|"loss"|"compromise"}
    | {kind:"resolve_conflict",
       conflictId:ConflictId,
       resolvedHeads:[ChangeId/1...],
       selected:WorkspaceAuthorizationBundleAddress/1,
       outcomes:[TrustConflictOutcome/2...],
       inheritedCompromises:[TrustConflictCarry/1|TrustConflictCarry/2...]},
  rootSignature:"<86 ASCII unpadded base64url>"
}

DomainSealKeyPoPBody/2 = {
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,decisionKey:DecisionKey/2,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  trustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>"
}

DomainSealKeyRotateContinuityBody/2 = {
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,decisionKey:DecisionKey/2,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  oldTrustKeyId:"sha256:<64 lowercase hex>",
  newTrustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>",
  possessionSignature:"<86 ASCII unpadded base64url>",mode:"ordinary"
}

WorkspaceTrustDeclarationSignedBody/2 :=
  WorkspaceTrustDeclaration/2 只删除 rootSignature

WorkspaceAuthorizationBundle/2 = {
  kind:"d6_workspace_authorization_bundle",
  version:2,
  workspaceRef:WorkspaceRef,
  authorizationRevision:Counter,
  policy:Policy/3,
  trustRoot:WorkspaceTrustRootDeclaration/1,
  trustRevision:Counter,
  trustDeclarations:[WorkspaceTrustDeclaration/1|WorkspaceTrustDeclaration/2...]
}

DomainSealKeyHandle/2 = {
  kind:"d6_domain_seal_key_handle",
  version:2,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1,
  trustKeyId:"sha256:<64 lowercase hex>",
  secureHandle:Token,
  state:"staged"|"usable"|"retired"|"lost"
}

SourceTransformSealVerificationKey/1 = {
  kind:"d6_source_transform_seal_verification_key",
  version:1,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  profile:"d6_source_transform_seal/1",
  trustKeyId:"sha256:<64 lowercase hex>",
  algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>"
}

TrustConflictCarry/2 = {
  kind:"d6_trust_conflict_carry",
  version:2,
  factId:"sha256:<64 lowercase hex>",
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1,
  compromisedTrustKeyId:"sha256:<64 lowercase hex>",
  originAction:"revoke"|"rotate",
  originDecisionKey:DecisionKey/2,
  originDeclarationRevision:Counter,
  originDeclarationDigest:"sha256:<64 lowercase hex>",
  originActivationChangeId:ChangeId/1
}

TrustConflictOutcome/2 =
    {commitDomain:CommitDomain/2,profile:SealProfileId/1,
     state:"keep_current",trustKeyId:"sha256:<64 lowercase hex>"}
  | {commitDomain:CommitDomain/2,profile:SealProfileId/1,state:"none"}
  | {commitDomain:CommitDomain/2,profile:SealProfileId/1,
     state:"authorize_fresh",trustKeyId:"sha256:<64 lowercase hex>",
     algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
     possessionSignature:"<86 ASCII unpadded base64url>"}

DomainSealKeyAddPrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_add_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  budget:BudgetBinding/1
}

DomainSealKeyRotatePrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_rotate_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  expectedTrustKeyId:"sha256:<64 lowercase hex>",
  mode:"ordinary"|"loss_recovery"|"compromise",
  budget:BudgetBinding/1
}

DomainSealKeyRevokePrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_revoke_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  expectedTrustKeyId:"sha256:<64 lowercase hex>",
  mode:"administrative"|"loss"|"compromise",
  budget:BudgetBinding/1
}
```

公开 conflict request 继续使用 fixed-parent d6_conflict_prepare wireVersion3 与 ConflictResolution/2。
对当前尚未建立决议的 policy_bundle_choice，同一 selected/policy/freshAuthorizations JSON arm 使用下面的 current inner type 与受保护 descriptor。
source_merge 与 choose_source_head 继续使用 fixed-parent ConflictResolutionInput/2 与 ConflictResolutionDerivedPlan/1。

```text
FreshDomainAuthorizationSpec/2 = {
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1
}

PolicyBundleHeadEvidence/2 =
    {head:ChangeId/1,completionProofVersion:3,bundleVersion:1}
  | {head:ChangeId/1,completionProofVersion:4,bundleVersion:1|2}

TrustConflictCarryValidationHop/1 =
    {declarationVersion:1,declarationRevision:Counter,
     declarationDigest:"sha256:<64 lowercase hex>",
     activationChangeId:ChangeId/1,
     completionProofVersion:3,bundleVersion:1,
     changeRecordPin:PinRef/2,
     completionProofPin:PinRef/2,
     policyBundlePin:PinRef/2}
  | {declarationVersion:2,declarationRevision:Counter,
     declarationDigest:"sha256:<64 lowercase hex>",
     activationChangeId:ChangeId/1,
     completionProofVersion:4,bundleVersion:2,
     changeRecordPin:PinRef/2,
     completionProofPin:PinRef/2,
     policyBundlePin:PinRef/2}

TrustConflictCarryValidationEvidence/1 = {
  head:ChangeId/1,
  factId:"sha256:<64 lowercase hex>",
  origin:TrustConflictCarryValidationHop/1,
  carriers:[TrustConflictCarryValidationHop/1...]
}

ConflictResolutionPolicyDerivedPlan/2 = {
  kind:"policy_bundle",
  selected:WorkspaceAuthorizationBundleAddress/1,
  headEvidence:[PolicyBundleHeadEvidence/2...],
  selectedBundleVersion:1|2,
  selectedBundlePin:PinRef/2,
  carryEvidence:[TrustConflictCarryValidationEvidence/1...],
  effectiveCompromises:[
    TrustConflictCarry/1|TrustConflictCarry/2...
  ],
  inheritedCompromises:[
    TrustConflictCarry/1|TrustConflictCarry/2...
  ],
  outcomes:[TrustConflictOutcome/2...],
  resultBundleVersion:1|2,
  resultBundlePin:PinRef/2
}

ConflictResolutionInput/3 = {
  kind:"d6_conflict_resolution_input",version:3,
  conflictId:ConflictId,expectedKey:ConflictKey/1,
  resolution:{
    kind:"policy_bundle_choice",
    selected:WorkspaceAuthorizationBundleAddress/1,
    policy:Policy/3,
    freshAuthorizations:[FreshDomainAuthorizationSpec/2...]
  },
  branchEvidence:[ConflictResolutionBranchEvidence/1...],
  derivedPlan:ConflictResolutionPolicyDerivedPlan/2
}

ConflictResolutionPreview/2 = {
  kind:"d6_conflict_resolution_preview",version:2,
  conflictId:ConflictId,expectedKey:ConflictKey/1,
  resolution:{
    kind:"policy_bundle_choice",
    selected:WorkspaceAuthorizationBundleAddress/1,
    policy:Policy/3,
    freshAuthorizations:[FreshDomainAuthorizationSpec/2...]
  },
  branchEvidenceDigest:"sha256:<64 lowercase hex>",
  derivedPlan:ConflictResolutionPolicyDerivedPlan/2
}
```

FreshDomainAuthorizationSpec/2 的 JSON 成员仍只有原来的两个，但 profile 扩为完整 SealProfileId/1 union。数组按 D3-CJ/3 规范排序且唯一，不含任何 key material。current ConflictResolutionInput/3 只作为 policy_bundle_choice 的 protected owner-descriptor successor；它不提升公开 request 版本、不新增 submit path，也不改变 conflict_resolve、policy_admin、disclosure 或 error order。任何可证明真实 saved/planned 的旧 owner descriptor 都保留 recorded decoder、preview 与 pins。

PolicyBundleHeadEvidence/2 必须完整、按 ChangeId 排序唯一，并与 expectedKey.heads 的集合逐项相等。version=3 表示 branchEvidence.completionProofPin 必须严格解码 ContentCompletionProof/3，且精确 policy after-image 必须严格解码 WorkspaceAuthorizationBundle/1。version=4 表示 completionProofPin 严格解码 ContentCompletionProof/4，branchEvidence.changeRecordPin 严格解码匹配的 ChangeRecord/1 与精确 Notice3/CP4 chain，policy after-image 再严格解码为声明的 Bundle1 或 Bundle2。两种 arm 都要求 policyBundlePin 存在并 pin 精确 canonical bundle bytes；WorkspaceAuthorizationBundleAddress/1 的 authorizationRevision、trustRevision、byteLength、sha256 必须匹配。policy_bundle_choice 的每个 branchEvidence.sourcePins 必须为空。CP3+Bundle2、未知版本、CP4 缺 ChangeRecord、tag/bytes 不一致或 decoder fallback 一律拒绝。

TrustConflictCarryValidationEvidence/1 是某个 effective fact 在某个 expectedKey head 上实际消费的完整 retained validation path。数组先按 head ChangeId，再按 ASCII factId 排序唯一；resolver 的逐 head effective compromise fold 使用的每个 (head,factId) 恰有一项。origin 是 Carry 所命名的直接 compromise declaration；carriers 按 declaration revision 递增，逐项列出该 head 上从 origin 之后真实经过的所有 resolve_conflict declaration。version-1 hop 严格解码其真实历史 ChangeRecord/CP3/Bundle1 activation evidence；version-2 hop 严格解码 ChangeRecord/1 + CP4 + Bundle2。declarationDigest 与 activationChangeId 必须逐字节等于这些精确 pins 所证明的 declaration 和 activation cut。origin hop 必须匹配 Carry 的 originDeclarationRevision、originDeclarationDigest 与 originActivationChangeId，并重新验证 action/key 映射。每个 carrier hop 都必须是实际携带该 fact 的合法 resolver。缺少任何实际 traversed carrier、把 origin cut 换成 resolver 时间，或用相同 current bytes 替换历史 evidence 都失败。

对 current policy arm，OwnerInputBinding/2.protocolOwner 固定为 D6，ownerKind 精确为 d6_conflict_resolution/3；InputDescriptor/3.intentKind 必须与该 ownerKind byte-equal。canonicalDescriptorBytes 精确等于 D3-CJ/3(ConflictResolutionInput/3)。OwnerInputBinding/2.pinRefs 是按 canonical PinRef 排序且去重后的精确并集，只包含：每个 branchEvidence.changeRecordPin；每个 branchEvidence.completionProofPin；每个存在的 branchEvidence.policyBundlePin；selectedBundlePin；resultBundlePin；以及 carryEvidence 中每个 origin/carrier 的 changeRecordPin、completionProofPin、policyBundlePin。hash-only declaration reference、current bundle、Derived Index row 或未列出的 pin 都不能替代这些条目。若 selected bundle pin 与对应 branch policyBundlePin 相同，set 去重后只出现一次。

ConflictResolutionPreview/2 是 current policy 的 immutable owner preview。branchEvidenceDigest 精确为 "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(完整 ConflictResolutionInput/3.branchEvidence array)))。preview 的 conflictId、expectedKey、resolution 必须与 Input3 byte-equal；derivedPlan 必须与完整 Plan2 byte-equal，包括 headEvidence、carryEvidence、mixed Carry1/2、Outcome2 以及 resultBundleVersion/pin。PreparedIntent/3.previewBinding 必须绑定这份精确 Preview2 与其 canonical preview pin。对这个 control_only policy arm，PreparedIntent/3.pinDirectory 必须是以下集合的 canonical duplicate-free union：OwnerInputBinding.pinRefs、DependencyProof/3 的全部 evidencePin、previewBinding 所选精确 preview pin、以及 fixed installationPlan 实际命名的全部 proposal/before/after/recovery PinRef。sourceInputs 为空，因此这里没有 SourceObservation evidence pin。相同 pin 不能重新绑定到另一份 protected record。

两个 current source arm 不使用这些 policy successor。source_merge 与 choose_source_head 继续保留 ConflictResolutionInput/2、ConflictResolutionDerivedPlan/1、ConflictResolutionPreview/1。OwnerInputBinding.ownerKind 与 InputDescriptor/3.intentKind 都固定为 d6_conflict_resolution/2；canonicalDescriptorBytes 必须精确等于 D3-CJ/3(Input2)。
原有完整 pin union、source semantic evidence、saveProfile=complete、scope selection 与 Preview1 规则继续有效。只有外层尚未建立决议的 D6 carrier 改用 current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family；planToken 为 d6_plan/3。

Carry2 的 originDeclarationDigest 恰为 "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(包含 rootSignature 的完整原 WorkspaceTrustDeclaration/2)))。直接 Declaration2 compromise 映射固定为 revoke -> action.trustKeyId，rotate -> action.oldTrustKeyId。Carry2 fact body 是按 D3-CJ/3 闭合编码的九个字段：workspaceRef、commitDomain、profile、compromisedTrustKeyId、originAction、originDecisionKey、originDeclarationRevision、originDeclarationDigest、originActivationChangeId。factId 固定为：

```text
factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL ||
      D3-CJ/3(Carry2 fact body)
    )
  )
```

originActivationChangeId 只能从原 Declaration2 DecisionKey，经 committed CP4 + ChangeRecord/1 与首次追加它的精确 Bundle2 after-image 重派生。Carry1 保持 fixed-parent /1 fact domain 与 Declaration1/CP3 origin 规则。Declaration2 resolve_conflict 可以继承 Carry1/Carry2；每项都必须递归回溯并重验其 direct original compromise declaration。effectiveCompromises 与 inheritedCompromises 都按 ASCII factId 排序唯一；重复 factId 必须 canonical bytes 相等，否则 integrity_conflict。effective union 同时包含 selected 与 losing branches，因此 branch choice 不能丢 compromise fact。

Bundle1 的 source-transform normalized state 是 none。affectedDomainProfiles 是所有 head 间 normalized domain/profile state 不同的 pair，与 effective compromise union 指向的所有 pair 的并集。需要 trust-resolution declaration 时，TrustConflictOutcome/2 按 D3-CJ/3(commitDomain,profile) 排序唯一，并完整覆盖每个 affected pair。keep_current 只允许 selected current key 在完整 effective union 下仍安全。显式请求的 affected pair 可以 authorize_fresh，其新 key 使用 DomainSealKeyHandle/2 与 PoP/2。若无需 trust resolution 且没有 fresh authorization，policy-only 结果保留 selectedBundleVersion/trustRevision；否则恰追加一条 WorkspaceTrustDeclaration/2 resolve_conflict，resultBundleVersion=2。selected Bundle1 的精确 Declaration1 prefix 必须原样保留，再追加新的 Declaration2。

ConflictResolutionPolicyDerivedPlan/2 冻结每个 head 的 proof/bundle 分派与 selected bundle pin，还要完整冻结逐 head carry validation evidence、完整 carry union、inherited subset、Outcome2 以及精确 result bundle pin/version。resultBundlePin 必须严格解码 derived result 并复现 canonical bytes。
原唯一 planning CAS 会同时冻结 InputDescriptor/3、OwnerInputBinding/2、Preview2、pinDirectory、staged fresh-handle association 与 installationPlan。final submit 仍只有 d6_commit_request/2。唯一 final P 通过 Notice3/CP4/ChangeRecord1 发布当前 policy component 与同一 conflict-record transition；staged authorize_fresh handle 只能在该 commit 中变 usable。
planned recovery 恢复精确的 Input3/Plan2/Preview2 bytes 与 pins，绝不从当前 history 重建。receiver 必须重算每个 head 的分派、Carry1/Carry2 facts、全部 retained carry-validation hop、mixed recursive union、PoP/2 与 root signatures；还必须验证精确 result bundle、preview/descriptor 交叉字段，以及同一 DecisionKey 的 conflict-record transition。

每个 publicKey 必须解码为精确的 32-byte Ed25519 key，并哈希到对应 trustKeyId；每个 signature 值必须解码为精确的 64-byte Ed25519 signature。
authorize/rotate 的 possessionSignature 必须按精确消息签署。
签名消息为 ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2)。
rotate 把 newTrustKeyId 映射到 PoP body 的 trustKeyId。
authorize_fresh 使用外层 Declaration2 的 common fields；再加入该 outcome 的 commitDomain/profile/key tuple，构造同一 PoP body。
ordinary rotate 的 continuitySignature 必须按精确消息签署。
签名消息为 ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2)。
loss_recovery/compromise 必须使用 literal "not_required"。
rootSignature 签署 exact ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2)。

revision-1 root predecessor 的 fingerprint 是完整 WorkspaceTrustRootFingerprint/1，并与 retained root/anchor fingerprint byte-equal；rootKeyId 不能替代。Declaration/1 永远只授权 revision-token profile。Bundle2 允许 Declaration1 历史 prefix，出现首个 Declaration2 后后继都只能 /2。Declaration1 activation 继续用 CP3；Declaration2 activation 由同 DecisionKey ChangeRecord1 -> exact CP4 -> policy after-image -> Bundle2 重派生。

当前未见过的普通 trust management 只使用上述 wireVersion3 prepare successor，不携带 caller key material。
它保留 fixed-parent 的 policy_admin、root-handle、current-state gate 与原 planning/install/single-P 路径，只把当前 completion family 接到 CP4。
已保存或已规划的 wireVersion2 record 继续使用原 decoder/recovery。

```text
WorkspaceTrustGenesis/2 = {
  kind:"d6_workspace_trust_genesis",version:2,
  rootDeclaration:WorkspaceTrustRootDeclaration/1,
  initialDomainDeclarations:[
    WorkspaceTrustDeclaration/2,
    WorkspaceTrustDeclaration/2
  ]
}

WorkspaceBootstrapProfile/4 = {
  kind:"d6_bootstrap_profile",wireVersion:4,
  profileRevision:Counter,
  registrySeedBinding:RegistryBinding/1,
  newSeriesMultiplicity:"unique"|"many",
  initialPeriodScope:"workspace"
}

WorkspaceBootstrapCreatorBinding/1 = {
  issuerPrincipal:Token,
  targetPrincipal:Token,
  principalAudienceToken:Token
}

WorkspaceBootstrapTargetRegistry/1 = {
  snapshot:RegistrySnapshot/1,
  binding:RegistryBinding/1
}

WorkspaceBootstrapSeriesConfiguration/1 = {
  seriesScope:SeriesScope,
  multiplicity:"unique"|"many",
  revision:1
}

WorkspaceBootstrapPeriodScopeBinding/1 = {
  nodeRef:NodeRef,
  scope:CalendarScope,
  revision:1
}

WorkspaceBootstrapPlan/4 = {
  kind:"d6_workspace_bootstrap_plan",
  wireVersion:4,
  operationId:Uuid,
  proposalId:Uuid,
  issuerAuthorityInstanceId:Uuid,
  targetWorkspaceRef:WorkspaceRef,
  targetAuthorityInstanceId:Uuid,
  profile:WorkspaceBootstrapProfile/4,
  creatorBinding:WorkspaceBootstrapCreatorBinding/1,
  targetRegistry:WorkspaceBootstrapTargetRegistry/1,
  initialPolicy:Policy/3,
  trustGenesis:WorkspaceTrustGenesis/2,
  initialPresentationPolicy:D8PresentationPolicyBootstrapInit/1,
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
```

全部 UUID 成员使用 D3 的 canonical lowercase UUID decoder。Genesis 恰两项：revision 1 是 revision-token authorize，revision 2 是 source-transform authorize；两者使用同一 DecisionKey/activation ChangeId，且 rev2 predecessor 必须哈希精确的 rev1 canonical bytes。
series configuration 按 canonical SeriesScope 排序且唯一；period binding 按完整 NodeRef 排序且唯一，每个有效 prepared period 恰有一项。
这些 helper member 保留 fixed-parent Plan3 的字段语义；Profile4 只改变 dual-profile genesis family。

Plan4 只用于 unseen fresh create/fork，并保留原 D3 proposal/custody/CAS/P boundary。普通 copy 不使用 Plan4。saved/planned/unknown recovery 保留实际 recorded decoder/bytes；restore/continue/failover 不合成 Genesis2。本候选不声称 Plan3 已部署，也不虚构 migration。

initialPresentationPolicy 是必填字段，并在同一个 Plan4 内闭合 D8 的 fresh-target state。before/proposal 的 WorkspaceRef 必须等于 targetWorkspaceRef；before 是 §6.4 定义的受保护 fresh empty-head observation，且 stamp.revision=1；proposal 精确为 parents=[]、revision=1、defaultPresentation=separate。
准备、规划和暂存阶段必须使用原 create/fork 的 OperationId、planning CAS 和 DecisionKey 冻结该 helper，并冻结一个 committed=null 的 typed presentation_policy_change preview。这不是 D8 SetRequest，不要求尚未激活的 target 具备 policy_admin，不新增第二 CAS/ledger，也不在最终 P 之前创建已提交的 policy record/hash/address/pin/outbox/ChangeId。

只有原 bootstrap 的全部最终检查成功后，同一个 P 才按 checked allocation 分配唯一的 bootstrap ChangeId C。Core 在这个原子 P 事务中根据 frozen proposal+C 构造 canonical D8WorkspacePresentationPolicy/2，计算 fixed-prefix hash、用于 recovery 的精确 portable_metadata pin/address 与 D8PresentationPolicyOutboxItem/1，提交 presentation_policy_change 及原 receipt/effects/ChangeRecord 关联，把 D8 head graph 从 frozen empty before 推进到 heads=[address]（epoch 不变，stamp revision 经 checked increment 到 2），并同时提交 target activation/custody 与其余全部 bootstrap effect。
record.activationChangeId 和 ChangeRecord.changeId 都必须是 C，outbox decisionKey 仍是原 create/fork DecisionKey。planning loser、abort、final check 失败或 P commit 失败都必须保持 target 未激活，并且不留下 partial D8 record/pin/head/outbox。Plan4 target 成功激活后，必须立即具备 revision=1/defaultPresentation=separate 的 /1 logical current view。P 后 publication retry 只能使用 retained outbox 的 exact bytes，并执行正常的 receiver same-decision/ancestry admission。

普通 managed copy 不使用 Plan4，也不能合成这项初始化。真实 saved/planned/unknown Plan4 必须恢复原 initialPresentationPolicy、OperationId/proposal/custody mapping、pins 与 decoder，绝不能重新采样 [] 或 current branch。restore/continue/failover 保留其 recorded/current policy state，不重跑 bootstrap。真正历史 Plan1/Plan3 bytes 保持 actual decoder，不注入该字段。本候选不声称已经 active 的 Workspace 需要 migration 或 dual-write。

legacy Handle1 只在 exact Declaration1-authorized key 仍 current、safe、usable 时继续 revision-token 新签名；永远不能签 transform。continuation 对两个 profile 分别求 current(K)|none|conflicted_or_unproved，任一 unproved 都禁止部分激活新 domain。declaration order 固定为 optional old revision revoke、optional old transform revoke、new revision authorize、new transform authorize，全部在同一 DecisionKey/CP4 且无可观察中间 prefix。

# 10. D10 mixed-version current outer schemas

以下均为 current outer-holder schema。每个 versioned carrier 都必须按 tag 严格解码对应的精确 inner type，并保留其原 canonical bytes 与 pins；未知 tag 必须 fail closed。carrier 只负责选择 decoder，不增加 authority、不建立 decision/CAS，也不迁移 inner record。

```text
D10WorkspaceReadDependencies/2 = {
  kind:"d10_workspace_read_dependencies",version:2,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  dependencyProof:DependencyProof/3,
  registryReads:[{binding:RegistryBinding/1,snapshot:RegistrySnapshot/1,
                  snapshotPin:PinRef/2}...],
  evidencePins:[PinRef/2...]
}

D10VersionedControlRecordImage/1 =
    {schema:"d10_control_record_image/1",value:D10ControlRecordImage/1}
  | {schema:"d10_control_record_image/2",value:D10ControlRecordImage/2}

D10ControlRecordImage/2 = {
  kind:"d10_control_record_image",version:2,
  binding:Binding<K>/1,scope:Scope/1,
  usageRevision:Option<Counter>,view:ControlCurrentView<K>/1,
  supplement:
      {kind:"automation",subscription:ScheduleSubscription/2,
       stop:Binding<stop>/1,creator:Token}
    | {kind:"run",createdBinding:Binding<run>/1,
       principal:Token,fixedBudget:BudgetCaps/1,
       invocation:Option<AutomationInvocation/1>,
       admission:Option<LeaseRunUse/1>,
       authorSteps:[D10VersionedAuthorStepResponsibility/1...]}
}

D10ControlRecordPin/2 = {
  image:D10ControlRecordImage/2,pin:PinRef/2
}

D10VersionedControlRecordPin/1 =
    {schema:"d10_control_record_pin/1",value:D10ControlRecordPin/1}
  | {schema:"d10_control_record_pin/2",value:D10ControlRecordPin/2}

D10ControlRange/2 =
    {kind:"records",version:2,scope:Scope/1,
     kinds:[ControlRecordKind/1],epoch:Token,revision:Counter,
     members:[ControlRef<K>/1]}
  | {kind:"cost_lineage",version:2,key:CostLayerKey/1,
     epoch:Token,revision:Counter,
     reservations:[ControlRef<reservation>/1]}
  | {kind:"occurrences",version:2,
     automation:ControlRef<automation>/1,epoch:Token,revision:Counter,
     records:[D10VersionedAutomationOccurrenceRecord/1...]}

D10VersionedControlRange/1 =
    {schema:"d10_control_range/1",value:D10ControlRange/1}
  | {schema:"d10_control_range/2",value:D10ControlRange/2}

D10VersionedControlPrepareBinding/1 =
    {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
  | {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
  | {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}

ControlPrepareBinding/1 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  dependencyPins:ControlDependencies/1
}

ControlPrepareBinding/2 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/2
}

ControlPrepareBinding/3 = {
  kind:"d10_control_prepare_binding",version:3,
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/3
}

对 current unseen external-consent preparation，confirmationRequirement 仍是 fixed-parent attended-confirmation protocol 使用的同一个闭合 ExternalConfirmationRequirement/1，并且必须在原 commit request 交付前冻结于 ControlPrepareBinding/3。可变 confirmation fact 仍只属于 ExternalConfirmationRecord/1，不能塞进 Binding3 或 ControlDependencies/3。current confirmation eligibility 因而消费 Binding3 + Dependencies3，同时保留原 trusted event、principal、time window、immutable intent/preview、disclosure、approval_unavailable/preflight、saved-result replay 与 result-redaction 规则。已经证明的历史 ControlPrepareBinding/2 保留其精确 requirement、Dependencies2、canonical intent bytes、confirmation association 与 recovery decoder。

ControlDependencies/3 = {
  kind:"d10_control_dependencies",version:3,
  configBindings:[Binding<K>/1],
  usageBindings:[{ref:ControlRef<lease|approval|grant|cost_account>/1,
                  usageRevision:Counter}...],
  authorityProof:Token,authorizationGenerations:[Token...],
  stopRefs:[ControlRef<stop>/1...],
  workspaceReads:Option<D10WorkspaceReadDependencies/2>,
  recordPins:[D10VersionedControlRecordPin/1...],
  controlRanges:[D10VersionedControlRange/1...]
}

D10ControlEffectPlan/2 = {
  kind:"d10_control_effect_plan",version:2,
  changes:[{before:Option<D10VersionedControlRecordImage/1>,
            after:D10VersionedControlRecordImage/1}...],
  activationSelector:Option<{before:D10ActivationSelector/1,
                             after:D10ActivationSelector/1}>,
  recordPins:[D10VersionedControlRecordPin/1...],
  registryChange:Option<{
    before:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    after:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    evolution:RegistryEvolutionProof/1,
    beforePin:PinRef/2,afterPin:PinRef/2}>
}

D10VersionedAuthorStepResponsibility/1 =
    {schema:"d10_author_step_responsibility/1",
     value:D10AuthorStepResponsibility/1}
  | {schema:"d10_author_step_responsibility/2",
     value:D10AuthorStepResponsibility/2}

D10VersionedScheduleSubscription/1 =
    {schema:"d10_schedule_subscription/1",value:ScheduleSubscription/1}
  | {schema:"d10_schedule_subscription/2",value:ScheduleSubscription/2}

D10VersionedAutomationOccurrenceRecord/1 =
    {schema:"d10_automation_occurrence_record/1",
     value:AutomationOccurrenceRecord/1}
  | {schema:"d10_automation_occurrence_record/2",
     value:AutomationOccurrenceRecord/2}

D10VersionedApprovalUse/1 =
    {schema:"d10_approval_use/1",value:ApprovalUse/1}
  | {schema:"d10_approval_use/2",value:ApprovalUse/2}

D10ExecutionClaims/2 = {
  kind:"d10_execution_claims",version:2,
  recordPins:[D10VersionedControlRecordPin/1...],
  prepareBindings:[D10VersionedControlPrepareBinding/1...],
  leaseRuns:[LeaseRunUse/1...],
  authorSteps:[D10VersionedAuthorStepResponsibility/1...],
  subscriptions:[D10VersionedScheduleSubscription/1...],
  occurrenceRecords:[D10VersionedAutomationOccurrenceRecord/1...],
  ranges:[D10VersionedControlRange/1...],
  continuityPins:[PinRef/2...]
}

D10MoneyResponsibility/2 = {
  kind:"d10_money_responsibility",version:2,
  reservations:[{binding:Binding<reservation>/1,
                 value:CostReservation/1,
                 attribution:CostBudgetAttribution/1,
                 settlements:[CostSettlementDecision/1...]}...],
  layers:[CostLayerTotal/1...],
  recordPins:[D10VersionedControlRecordPin/1...],
  ranges:[D10VersionedControlRange/1...],
  evidencePins:[PinRef/2...]
}

StopCapacity/1 = {
  issued:Counter,reserved:Counter
}

D10ExecutionInventory/2 = {
  kind:"d10_execution_inventory",version:2,
  workspaceRef:WorkspaceRef,storeIncarnation:Uuid,
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  stopCapacity:StopCapacity/1
}

ExecutionResponsibilityRecord/3 = {
  kind:"d6_execution_responsibility",version:3,
  workspaceRef:WorkspaceRef,executionDomainId:Uuid,
  holder:
      {kind:"local_replica",replicaEpoch:Uuid}
    | {kind:"server",authorityInstanceId:Uuid,deploymentId:Uuid},
  revision:Counter,status:"active"|"paused"|"transferring",
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  lastContinuityProof:ExecutionContinuityProof/2
}

ExecutionContinuityProof/2 =
    {kind:"initial",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,birthProofToken:Token}
  | {kind:"checkpoint",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,barrierToken:Token}
  | {kind:"handoff",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,fromHolder:ExecutionHolder,
     toHolder:ExecutionHolder,oldRevision:Counter,
     barrierToken:Token,oldHolderFenceToken:Token}
```

Image2 只允许 automation/run；其它 record kind 继续使用精确 Image1。Pin1 保留历史 D10-Control-Record/1 前缀与 decoder。Pin2 的完整内容是 UTF8 D10-Control-Record/2、NUL 与 D3-CJ/3(Image2)；PinRef/2 的 payloadKind 是 artifact，retention 沿用 recovery 或 approval_money，byteLength 与 SHA-256 必须覆盖完整前缀内容。schema tag 与 payload domain 不一致必须失败，Pin2 不得重新 pin Image1。

对 D10VersionedControlRecordPin/1 数组，先令 I=carrier.value.image。排序依次使用 D3-CJ/3(I.binding.ref)、binding.revision 数值、usageRevision 的 none 先于 some、some 时的 usageRevision 数值、最后 D3-CJ/3(I)。同 cut identity 是 I.binding.ref、I.binding.revision、I.usageRevision。相同 identity 的第二个 byte-equal image 属于重复并拒绝；bytes 不同则是 integrity_conflict。version 不能拆分该 identity。

Range kind rank 固定为 records=0、cost_lineage=1、occurrences=2。跨版本 logical identity 分别是 scope 加完整排序后的 kinds、CostLayerKey、Automation Ref。mixed range 先按 rank 再按 identity canonical bytes 排序；每个 identity 只能出现一次。occurrences range 的内部记录按 AutomationOccurrenceKey 排序；一个 key 不能同时出现 V1 与 V2。

D10ControlEffectPlan/2.changes 按完整 after.value.binding.ref 的 canonical bytes 排序唯一。before 存在时，before.binding.ref 与 after.binding.ref 必须相等，recordPins 还必须含有与真实存储 before schema 完全匹配的 versioned pin。Image1 before 配 Pin2 非法。Image1 Automation+Subscription1 → Image2+Subscription2 是合法 current configure。若 scheduling owner 的 continue 规则成立，普通 configure 可以让旧 subscription 保持同一 generation；mixed 支持不能强迫 replace 或后台 migration。

ControlDependencies/3 的数组保持 owner 排序：configBindings 与 usageBindings 按完整 Ref；authorizationGenerations 按 token；stopRefs 按完整 Ref；recordPins/ranges 按上述 mixed 顺序。每个 config binding 都有精确匹配 image/pin；每个 usage binding 都有相同 usageRevision 的 image；每个 stopRef 都有精确 stop Image1/Pin1。current binding、usage revision、range fence、pin 与 Workspace evidence 必须来自同一个真实 Authority Store barrier。barrier A 的 V1 evidence 与 barrier B 的 V2 evidence 不能拼成 complete dependency snapshot。缺失必要历史 bytes、decoder 或 pin 时，在 disclosure 之后返回 state_unavailable；同 cut 矛盾是 integrity_conflict。

D10ExecutionClaims/2 的 canonical identity 固定如下：recordPins 使用上面的规则；prepareBindings 在 Binding1/2/3 之间统一按 inner StableControlKey；leaseRuns 按完整 Run ControlRef；authorSteps 在版本间统一按 run Ref 加 stepId；subscriptions 在版本间统一按 Automation Ref 加 generation；occurrenceRecords 按 AutomationOccurrenceKey；ranges 使用上面的规则；continuityPins 按 pinToken。每个 identity 跨版本唯一。Binding1(K) 与 Binding3(K) 即使 canonicalIntentBytes 和 originalCommitRequest byte-equal 也不能共存。Binding1 保留历史六成员精确 shape；不能增加 kind、version、confirmationRequirement 或 /2-/3 dependency field，也不能 LWW、repin。

D10MoneyResponsibility/2 的 reservation 按完整 Binding<reservation>/1 canonical bytes 排序唯一；layer 按 CostLayerKey canonical bytes 排序唯一；recordPins/ranges 使用 mixed 规则；evidencePins 按 pinToken 排序唯一。D10ExecutionInventory/2 的 approvalUses 按 DecisionKey canonical bytes 跨版本唯一，externalUnknowns 与 stopState 均按 binding.ref canonical bytes 排序唯一。所有 pending、unknown、dedup、stop responsibility，以及仍被引用的已完成 external attempt 都必须保留。只有在 admission、planning、send、schedule writer 都停在同一个真实 store barrier 后才能捕获 inventory。

Inventory2 artifact 的精确 payload 是 UTF8 D6-Execution-Inventory/2、NUL、D3-CJ/3(D10ExecutionInventory/2)。PinRef/2 为 artifact/recovery，byteLength 与 SHA-256 覆盖完整前缀 bytes。Inventory1 保留 D6-Execution-Inventory/1，绝不重新 pin 成 /2。

Record3.workspaceRef 必须等于 Inventory2.workspaceRef。Record3 的 approvalUses、claims、moneyLineage、externalUnknowns、stopState 五类 payload 分别与 Inventory2 对应成员 byte-equal。Proof2.inventoryPin 必须选择这一份精确 Inventory2，且 Inventory2.storeIncarnation 等于 Proof2.storeIncarnation。受保护的 birth/barrier/fence token mapping 必须证明同一实际 store 与 capture barrier；不新增独立 caller-supplied store-incarnation proof object。

Inventory2.stopCapacity 必须是同一 authoritative safety-store barrier 下的精确 fixed-parent StopCapacity/1。StopCapacity/1 仍只有 issued、reserved，不在 Record3 中重复。Workspace-only handoff 不能把 shared safety counter 复制到第二个 active store。完整 store handoff 必须 fence 全部受影响 writer，并保留所有 target/latch reservation 与 safety capacity；边界不可证明时 takeover unavailable。

Record2、Proof1、Inventory1 都保留历史 decoder/domain。Record3 只在真实 responsibility mutation、checkpoint 或 custody handoff 时产生，必须保持 executionDomainId；unchanged holder 不做后台 migration。

## 10.1 D10 current schedule / author-step 直接类型

以下类型是 §10 mixed wrappers 所引用的 current exact inner values；不是 wrapper 自行发明的自由对象。

```text
D10ControlInput/2 = {
  kind:"d10_control",
  version:2,
  key:StableControlKey/1,
  operationId:Uuid,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  preview:ControlPreview/1,
  dependencies:ControlDependencies/3,
  effectPlan:D10ControlEffectPlan/2,
  confirmationRequirement:ExternalConfirmationRequirement/1
}

ScheduleRecurrenceEvidence/2 = {
  kind:"d10_schedule_recurrence_evidence",
  version:2,
  observation:SourceObservation/1,
  sourcePin:PinRef/2,
  metadataPin:PinRef/2,
  dependencyProof:DependencyProof/3,
  registrySnapshot:RegistrySnapshot/1,
  registryEvolution:Option<RegistryEvolutionProof/1>,
  recurrenceContext:RecurrenceReadContext/1
}

ScheduleSourceBinding/2 =
    {kind:"once",atUtcSeconds:CanonicalDecimal}
  | {kind:"recurrence",
     ownerNodeRef:NodeRef,
     recurrenceOccurrenceKey:occurrenceKey,
     rangeOccurrenceKey:occurrenceKey,
     initial:ScheduleRecurrenceEvidence/2,
     checkpoint:ScheduleRecurrenceEvidence/2,
     continuityPins:[PinRef/2...]}

ScheduleSubscription/2 = {
  kind:"d10_schedule_subscription",
  version:2,
  automation:ControlRef<automation>/1,
  generation:Counter,
  lowerOriginalStartUtcSeconds:CanonicalDecimal,
  activeDefinition:Binding<automation>/1,
  definitionRevision:Counter,
  source:ScheduleSourceBinding/2
}

ScheduleOccurrenceProof/2 =
    {version:2,kind:"once",at:zoned_instant}
  | {version:2,kind:"recurrence",
     evidence:ScheduleRecurrenceEvidence/2,
     projection:<complete accepted D4 recurrence projection outcome>,
     originalStart:<the selected outcome row's D4 originalStart>}

AutomationOccurrenceRecord/2 = {
  kind:"d10_automation_occurrence_record",
  version:2,
  key:AutomationOccurrenceKey/1,
  definition:Binding<automation>/1,
  definitionRevision:Counter,
  dueUtcSeconds:CanonicalDecimal,
  proof:ScheduleOccurrenceProof/2,
  disposition:AutomationOccurrenceDisposition/1
}

ScheduleContinuityWitness/2 = {
  kind:"d6_schedule_continuity",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  revision:Counter,
  initial:ScheduleRecurrenceEvidence/2,
  checkpoint:ScheduleRecurrenceEvidence/2,
  status:"continuous"|"binding_changed"|"gap",
  producerEpoch:Token,
  consumedTransition:Counter
}

ScheduleContinuityStep/2 = {
  kind:"d6_schedule_continuity_step",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  before:ScheduleRecurrenceEvidence/2,
  after:ScheduleRecurrenceEvidence/2,
  portableChanges:[{
    changeRecordPin:PinRef/2,
    installationNoticePin:PinRef/2,
    completionProofPin:PinRef/2
  }...],
  dependencyBefore:DependencyProof/3,
  dependencyAfter:DependencyProof/3,
  retainedInputs:[PinRef/2...]
}

ScheduleContinuityInvalidation/2 = {
  kind:"d6_schedule_continuity_invalidation",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  status:"binding_changed"|"gap",
  evidencePins:[PinRef/2...]
}
```

当前 continuity artifact 的编码按版本精确分域：

```text
Witness2ArtifactBytes =
  UTF8("D6-Schedule-Continuity/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityWitness/2)

Step2ArtifactBytes =
  UTF8("D6-Schedule-Step/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityStep/2)

Invalidation2ArtifactBytes =
  UTF8("D6-Schedule-Invalidation/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityInvalidation/2)
```

三者对应的 PinRef/2 都使用 payloadKind=artifact、retentionClass=recovery。byteLength 与 SHA-256 必须覆盖完整 domain prefix、唯一的 NUL byte 和完整 canonical object bytes。digest 本身不认证 continuity；仍必须验证受保护 producer/subscription provenance、producerEpoch、精确 generation、retained transition chain，以及原 authorization/disclosure gate。

decoder 必须先按 authenticated artifact domain 闭合分派，再解 inner object。D6-Schedule-Continuity/1 只能解 historical ScheduleContinuityWitness/1；D6-Schedule-Step/1 只能解 historical ScheduleContinuityStep/1；D6-Schedule-Invalidation/1 只能解 historical ScheduleContinuityInvalidation/1。D6-Schedule-Continuity/2 只能解 ScheduleContinuityWitness/2；D6-Schedule-Step/2 只能解 ScheduleContinuityStep/2；D6-Schedule-Invalidation/2 只能解 ScheduleContinuityInvalidation/2。unknown domain、已知 domain 搭配错误 kind/version、其它 prefix、缺失 NUL 或非 canonical object bytes 都必须按原 disclosure/error boundary fail closed。禁止 fallback decoder、扩展 /1 decoder、repin 或重新编码历史 bytes。

```text
D10AuthorPreparationLink/2 = {
  kind:"d10_author_preparation_link",
  version:2,
  run:ControlRef<run>/1,
  stepId:Counter,
  automation:Binding<automation>/1,
  definitionRevision:Counter,
  taskDigest:Sha256,
  preparedBindingToken:Token,
  request:d6_commit_request/2
}

ApprovalUse/2 = {
  version:2,
  approval:Binding<approval>/1,
  run:ControlRef<run>/1,
  stepId:Counter,
  request:d6_commit_request/2,
  decisionKey:DecisionKey/2,
  preparedBindingToken:Token,
  previewSemanticDigest:Sha256,
  delegationBinding:Binding<lease>/1,
  activationBinding:ActivationBinding/1,
  count:ApprovalCountReservation/1,
  budgetReservations:[ControlRef<reservation>/1...]
}

D10AuthorStepResponsibility/2 =
    {version:2,kind:"core_field_member",
     link:D10AuthorPreparationLink/2,
     decisionKey:DecisionKey/2,protocolOwner:"D6",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
  | {version:2,kind:"interactive",
     run:ControlRef<run>/1,stepId:Counter,decisionKey:DecisionKey/2,
     authorRequest:
         {protocolOwner:"D3",request:D3IdentityOperationRequest/13}
       | {protocolOwner:"D6",request:d6_commit_request/2},
     preparedFormat:
       "d7_prepared_action_binding4"|"d8_prepared_edit_binding3",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
```

对 fresh current core_field_member step，D10AuthorPreparationLink/2.preparedBindingToken 必须选择精确 PAB4，且该 PAB4 的原 request 必须等于 link.request。Core 必须在返回 prepared step 或允许 submission 前，原子保存 Link2、完整 PAB4 与所需 preview/effect/recovery pins。ApprovalUse/2.preparedBindingToken 选择同一个 PAB4。其 preview digest 固定为
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3))；
EffectBytes/3 slot 只投影为 {encoding,byteLength,payloadDigest}，不按成员名递归猜测。fresh current automatic author qualification 使用 DependencyProof/3；只要语义解析 managed Document，就必须包含 document_format，并且只有在完整验证 EffectManifest/3、EffectBytes/3 与 MutationFootprint 后才能构造 ApprovalUse/2。历史 Link1/PAB3/ApprovalUse1 的 saved 或 planned association 保留原 decoder、bytes、pins、request 与 OperationId。

对 fresh ScheduleSubscription/2 registration，current D6 Storage producer 只有在 selected source/Field/Registry/current scheduling gates 通过、有限 retention 已预留、且真实持续维护的 Core source/control transition producer 已在同一 configuration transaction 注册后，才能创建 ScheduleContinuityWitness/2。initial 与 checkpoint 使用 subscription 的精确 ScheduleRecurrenceEvidence/2，revision=1、consumedTransition=0，并生成 fresh producerEpoch。retained witness pin 必须精确使用上面的 Witness2ArtifactBytes。current 正向 transition 使用 ScheduleContinuityStep/2 与 DependencyProof/3；新的 current portable transition 使用真实 ChangeRecord/1、InstallationNotice/3 与 ContentCompletionProof/4 pins，每个 retained current step pin 都必须精确使用上面的 Step2ArtifactBytes。retained history 中的历史 transition 保持原 /1 artifact domain 与精确 decoder。相关 P-only control/rule transition 仍从真实 protected before/after state 捕获。

fold、compaction、receiver admission、D10 continuityPins consumption 与 recovery 都必须先按每个 protected schedule-continuity artifact 的精确 domain 分派，再解 inner object。current Witness2/Step2 不能放在 /1 domain 下接受，historical Witness1/Step1 也不能放在 /2 domain 下接受。continuityPins 是按 pinToken 排序且唯一的精确 PinRef/2；真实历史若跨过显式 same-generation bridge，可以保留 version-mixed original typed chain，但每个元素都保留自己的精确 bytes/domain/decoder。另一条合法 full retained-chain 路径同样保留每个 original typed artifact 与 source/control evidence，不能把整条 chain 归一化成一个版本。typed evidence 缺失或 unknown 时，在原 authorization/disclosure check 后沿用原 gap/unavailable 行为；已证明 domain/object mismatch 时不得用相同 digest/current state 修复。

当不存在合法 current after evidence 时，current producer 必须产生 ScheduleContinuityInvalidation/2，不能伪造 ScheduleRecurrenceEvidence/2。binding_changed 需要完整可信的 selected-business discontinuity 证据；after unavailable/unknown、missing history、unknown decoder、observer/producer gap 或无法保留必要 transition 均为 gap。Invalidation 比较同一 current witness/registration，原子推进下一 checked transition/revision，保留最后合法 checkpoint，并对该 generation 永久不可 reset。其 artifact pin 必须精确使用上面的 Invalidation2ArtifactBytes。fixed-parent inbox/capacity/final-counter reservation、authorization、compaction 与无关 source 可用性规则保持不变。

Schedule current proof 中真实解析 managed Document 时必须含 source + document_format dependency。source/profile bytes 未变但 format proof continuity gap 得到 gap；binding 发生真实改变得到 binding_changed，即使最终 recurrence/range 值碰巧相同。已有 Subscription1 继续作为 historical retention owner，并配套 Witness1/Step1/Invalidation1 及其精确 /1 artifact domain。它只有通过 explicit continue + complete retained history 证明没有 intervening format/rule/business discontinuity，并建立 current Evidence2/Proof3 cut，才可变成 same-generation Subscription2；否则必须 replace。这个 bridge 保留每个旧 pin 与 producer association；只有 bridge 之后新产生的 Witness2/Step2/Invalidation2 才使用 /2 domain。绝不把 version-1 witness/step/invalidation repin 或重新编码成 version 2，也绝不 reset invalid generation。


## 11. D7 文件绑定元数据直接类型

fresh QuerySpec/2 使用的 D6-owned 受保护 metadata producer 采用下列闭合形状：

~~~text
D6FileBindingMetadataReadRequest/1 = {
  kind:"d6_file_binding_metadata_read",version:1,
  workspaceRef:WorkspaceRef,ref:NodeRef|ResourceRef,
  projection:"basename"|"relative_path"
}

D6FileBindingMetadataObservation/1 = {
  kind:"d6_file_binding_metadata_observation",version:1,
  workspaceRef:WorkspaceRef,ref:NodeRef|ResourceRef,
  projection:"basename"|"relative_path",value:text,
  observation:SourceObservation/1
}
~~~

cross-field 要求精确：request/observation 的 WorkspaceRef 与 Ref 逐字节相等；observation.entityRef 等于 ref；observation.fileObjectBinding 必须是 present；`relative_path` 只接受 NodeRef，且 value 等于完整 PortableRelativePath；`basename` 接受 NodeRef 或 ResourceRef，且 value 等于最后一个非空 '/' segment。不存在 AnnotationRef arm、absent arm、nullable value 或 host-path 表示。

该类型只是受保护内部证据。它不能进入 InputDescriptor.sourceInputs，不能变成 SourceVersionRef，不授 source/resource bytes，也不替代普通 authorization DependencyKey。consumer 必须同时绑定既有 authorization proof 与准确 SourceObservation/FileObjectBinding，并在最终 barrier 重验。
