---
source_language: zh-CN
translation_of: D3-SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](D3-SCHEMAS.zh-CN.md)

# A2 D3 Current Closed Schemas

Status: A2 D3 author candidate; not independent acceptance, implementation, activation, or migration.

This companion is self-contained for the **changed current carriers** that D3 consumes. It copies the exact fixed97 closed schema text instead of restating it by version number. Stable D3 Ref/Locator/D3-CJ/Result9/receipt element schemas remain written in the A2 D3 main body and are not mechanically version-bumped.

Current dispatch is exact:
- native identity request: D3IdentityOperationRequest/13 and D3IdentityInput/13;
- D6 input: DependencyKey/3, DependencyProof/3, InputDescriptor/3, PreparedIntent/3;
- portable current: InstallationNotice/3, ContentCompletionProof/4, ChangeRecord/1;
- resolution guard: D3ResolutionInputUse/2;
- D7 current: PreparedActionBinding/4, EffectManifest/3, EffectBytes/3;
- Annotation: D3-Annotation-Value/4 and PortableAnnotationRecord/4;
- bootstrap/trust: WorkspaceBootstrapPlan/4 and WorkspaceTrustGenesis/2;
- D10 current mixed author holder: the versioned /2-/3 current carriers below.

D3ResolverInput/12, D3DecisionCompanion/2, RevisionTokenBinding/2 and canonical-conflict-only SourceRevisionPlan/2 are **not** mechanically bumped. Genuine historical wire9-12, Descriptor/Proof/Prepared families, Notice/CP1-3, PAB1-3, Value3, Plan1/3 and their pins/OperationId/TTL/recovery remain byte-exact under their recorded decoder.

## Part A — current D6/D3/D7/D8 direct holders

# 5. Managed format and D6 successors

```text
ManagedDocumentFormatProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  managedProfile:"weftext_managed/1"
}

ManagedDocumentFormatBinding/1 = {
  kind:"weftext_managed_document_format",version:1,
  ownerNodeRef:NodeRef,bindingRevision:Counter,
  profile:ManagedDocumentFormatProfile/1
}

DocumentFormatDependencyKey/1 = {
  kind:"document_format",workspaceRef:WorkspaceRef,ownerNodeRef:NodeRef
}

DocumentFormatCurrentQualification/1 = {
  kind:"d2_document_format_current_qualification",version:1,
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

BaselineOnly is not a portable binding arm. Fresh/copy/fork fresh identity starts at binding revision 1; formal same-Workspace restore restores the exact historical binding; ordinary backup bytes require fresh admission; a real managed-profile migration checked-increments the revision.

## 5.1 Dependency family

```text
DependencyKey/3 =
  source(0)|document_format(1)|lifecycle(2)|placement_range(3)|
  ref_inbound(4)|relation_incidence(5)|calendar_scope(6)|registry(7)|
  temporal_rules(8)|authorization(9)|foreign_binding(10)|query_scan(11)|
  replica_registry(12)|conflict_record(13)|execution_resource(14)
```

```text
DependencyProof/3 = {
  kind:"d6_dependency_proof",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  baseFrontier:Frontier/2,
  entries:[{key:DependencyKey/3,
            stamp:{epoch:Token,revision:Counter},
            evidencePins:[PinRef/2...]}...]
}

InputDescriptor/3 = {
  kind:"d6_input_descriptor",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  intentKind:text,
  saveProfile:"ordinary"|"complete"|"control_only",
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  frontierPolicy:"exact"|"scope_dependencies",
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  controlInputs:[{key:DependencyKey/3,
                  stamp:{epoch:Token,revision:Counter}}...],
  ownerInput:OwnerInputBinding/2
}

PreparedIntent/3 = {
  kind:"d6_prepared_intent",version:3,
  planToken:Token,operationId:Uuid,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,beforeCut:Frontier/2,
  proposedState:SemanticState/1,mutationFootprint:MutationFootprint,
  dependencyProof:DependencyProof/3,observationProof:ObservationProof,
  budgetBinding:BudgetBinding/1,pinDirectory:PinDirectory,
  installationPlan:InstallationPlan,inputRetentionState:InputRetentionState,
  expiresAt:PreparedDeadline,previewBinding:PreviewBinding
}
```

The final unchanged nested names above use the exact fixed-parent PreparedIntent/2 decoder. Current planToken tag is d6_plan/3.

## 5.2 Portable component / Notice / CP

```text
PortableComponentKey/2 =
  document(0)|document_format(1)|resource(2)|annotation(3)|
  node_binding(4)|child_list(5)|lifecycle(6)|trash_membership(7)|
  policy(8)|registry(9)|period_scope(10)|replica_registry(11)|conflict(12)

InstallationNotice/3 = {
  format:"weftext.installation-notice",version:3,
  decisionKey:DecisionKey/2,
  guarantee:"replica_local"|"managed_atomic",
  writeProtection:"strict"|"observed_only",
  baseFrontier:Frontier/2,
  components:[{key:PortableComponentKey/2,
               before:ComponentImage/1,after:ComponentImage/1}...]
}

ContentCompletionProof/4 =
    {format:"weftext.content-completion",version:4,outcome:"committed",
     decisionKey:DecisionKey/2,changeId:ChangeId/1,
     guarantee:"replica_local"|"managed_atomic",
     writeProtection:"strict"|"observed_only",
     semanticState:SemanticState/1,
     frontierBefore:Frontier/2,frontierAfter:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...],
     sourceChanges:[{entityRef:EntityRef,
                     before:SourceVersion/2|"absent",
                     after:SourceVersion/2|"absent"}...],
     receiptDigest:"sha256:64-lowercase-hex"}
  | {format:"weftext.content-completion",version:4,outcome:"restored",
     decisionKey:DecisionKey/2,baseFrontier:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...]}
```

PinRef/2 and ComponentImage/1 are unchanged fixed-parent types.

Notice3 inherits every Notice2 invariant except the versioned component-key decoder: components is non-empty, PortableComponentKey/2 fixed-rank/canonical-key sorted and unique, and the notice is frozen before installation with the original baseFrontier. CP4 inherits every CP3 committed/restored invariant except the component-key decoder and current ChangeRecord/1 linkage. For committed CP4, components is exactly the Notice3 key set in identical order, every after is the actual installed/sealed image, sourceChanges is the complete EntityRef-sorted unique real source-state delta, and receiptDigest binds the original receipt. A fresh managed Document carries both document and document_format in the same plan/P/CP4. A format-only change with unchanged source has sourceChanges=[] and creates no SourceRevisionPlan, managed SourceVersion, or H advance.

## 5.3 ChangeRecord

```text
ChangeRecord/1 = {
  format:"weftext.change-record",version:1,
  decisionKey:DecisionKey/2,changeId:ChangeId/1,
  installationNotice:{format:"weftext.installation-notice",version:3,
                      byteLength:Counter,sha256:"64-lowercase-hex"},
  completionProof:{format:"weftext.content-completion",version:4,
                   byteLength:Counter,sha256:"64-lowercase-hex"},
  frontierBefore:Frontier/2,frontierAfter:Frontier/2
}
```

CP4 must be committed and match DecisionKey, ChangeId, and frontiers. Notice/CP lengths and digests bind their exact D3-CJ/3 canonical bytes. One P seal fixes ChangeId, CP4, and ChangeRecord; publication only retransmits original pins.

frontierBefore is the actual verified pre-seal Frontier and frontierAfter is exactly frontierBefore plus this ChangeId with no other-domain regression. With frontierPolicy=exact, frontierBefore is byte-equal to the original expected/base/notice Frontier. With scope_dependencies, admission requires the complete continuous verified ChangeRecord/completion chain from Notice3.baseFrontier through frontierBefore and frontierAfter plus the original retained unrelatedness proof; vector numbers, provider sync state, or current files never substitute.

Restored CP4 contains only decisionKey, baseFrontier, and component after images that each equal the corresponding Notice3 before image. It forbids changeId, guarantee, writeProtection, semanticState, frontierBefore, frontierAfter, sourceChanges, receiptDigest, and all success semantics.

Receiver admission strict-decodes the exact Notice3/CP4 versions and canonical bytes, validates equal component key sets/order, obtains and validates every actual component byte and owner version, validates complete production SourceVersion changes, and verifies the continuous chain. A present document_format component additionally strict-decodes exact ManagedDocumentFormatBinding/1 bytes. Missing/unknown bytes or decoder, a component-version mismatch, or a Notice/CP/ChangeRecord mismatch is incomplete/proof_unavailable, never successful admission.

# 6. D3/D7/D8/D9 direct holders

```text
D3ResolutionInputUse/2 = {
  kind:"d3_resolution_input_use",version:2,
  decisionKey:DecisionKey/2,principalAudienceToken:Token,
  bindingToken:Token,inputDescriptor:InputDescriptor/3
}

D3IdentityInput/13 = {
  kind:"d3_identity_input",version:13,
  mode:D3Mode,
  writeProtection:"strict",
  expectedAuthority?:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  intent:D3Intent
}

D3IdentityOperationRequest/13 = {
  wireVersion:13,
  kind:"identity_operation_request",
  operationId:UUIDv4,
  boundWorkspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  inputDescriptor:InputDescriptor/3,
  mode:D3Mode,
  expectedAuthority?:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  preparationBinding?:PreparationBinding,
  intent:D3Intent
}
```

D3IdentityInput/13 has the exact D3IdentityInput/12 semantic member set: only the nested D6 descriptor family changes to current InputDescriptor/3 at the outer request boundary. expectedAuthority, workspaceProposal and preparationBinding are genuinely optional members, never nullable placeholders. replica_local permits only create_node/move_node/reorder_node/trash and requires all three members absent. managed_atomic follows the fixed-parent matrix exactly: create/fork require expectedAuthority=create and the required proposal; continue requires expectedAuthority=continue; other managed_atomic modes require expectedAuthority=existing; preparationBinding appears only where the inherited D7-mediated mode admits it. A forbidden member is invalid_request even when its value would otherwise decode, and JSON null is always invalid. D3IdentityOperationRequest/13 is the exact wire12 top-level member set with wireVersion=13 and InputDescriptor/3. Descriptor/request mode, authority and proposal are byte-equal wherever present; guarantee/frontier policy, ownerInput protocolOwner=D3/ownerKind=d3_identity_operation/13, requestFingerprint rule, DecisionKey/OperationId ledger ordering, saved/planned/unseen branching, and error/disclosure order are inherited unchanged. Historical wire9–12 are not widened or re-encoded.

## 6.1 PreparedActionBinding/4

The current D7 action successor is exact inheritance, not a free extension. D7ActionSpec/2 is the fixed-parent ActionSpec/1 top-level object with version=2; its intent decoder is exactly the fixed-parent intent union with only the historical apply_suggestion arm removed, then the three closed Annotation arms below added. Every other fixed-parent arm keeps its members and semantics byte-for-byte.

```text
D7CreateAnnotationIntent/2 = {
  kind:"create_annotation",
  destinationOwnerRef:NodeRef,
  value:AnnotationEditableProposal/1
}

D7ApplySuggestionIntent/2 = {
  kind:"apply_suggestion",
  annotation:AnnotationRef,
  expectedAnnotationRevisionToken:AnnotationRevisionToken/1
}

D7RejectSuggestionIntent/2 = {
  kind:"reject_suggestion",
  annotation:AnnotationRef,
  expectedAnnotationRevisionToken:AnnotationRevisionToken/1
}

D7ActionSpec/2 :=
  ActionSpec/1 with top-level version=2,
  with the fixed-parent apply_suggestion arm removed,
  plus D7CreateAnnotationIntent/2,
       D7ApplySuggestionIntent/2 and D7RejectSuggestionIntent/2

D7ActionPrepareRequest/3 = {
  wireVersion:3,kind:"d7_action_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  action:D7ActionSpec/2,
  selectedSources:[SourceVersionRef/1...],
  budget:BudgetBinding/1,
  evidenceToken?:Token
}

D7ActionInput/3 = {
  kind:"d7_action_input",version:3,
  action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext...],
  proposedInputs:[D7ProposedInput/3...]
}

PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding",version:4,
  bindingToken:Token,protocolOwner:"D3"|"D6",
  operationId:UUIDv4,workspaceRef:WorkspaceRef,
  principalAudienceToken:Token,action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext...],
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  constructionInput:null|TemplateConstructionInput/2,
  proposedInputs:[D7ProposedInput/3...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,expiresAt:<D6 protected deadline>,
  request:D3IdentityOperationRequest/13|d6_commit_request/2,
  preview:<complete EffectManifest/3>,
  resolutionInput:null|D3ResolutionInput/1
}
```

MinimumMapping/3, D7DefinitionInput/2, and D7ResolutionAccess/1 retain their exact fixed-e8aa shapes. D7ProposedInput/2 is retained only for genuine historical d7_action/2 / PAB3 decoding and is never widened in place.

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
```

The only legal current `/3` cross-fields are exact_source_document↔exact_source_utf8, resource_bytes↔resource_bytes, and annotation_value↔d3_annotation_value4|d3_symbolic_result9. A protocolOwner=D6 current concrete Annotation after uses d3_annotation_value4 and pins exactly D3-CJ/3(the complete D3-Annotation-Value/4). d3_symbolic_result9 remains legal only for the inherited current D3 symbolic-result branch under its real Result/9 subject/pin rules. `d3_annotation_value3` is invalid in `/3`. OwnerInputBinding/2.pinRefs exactly covers every `/3` proposed pin plus actually protected evidence. Historical PAB3/Input2 continues byte-for-byte with D7ProposedInput/2 and d3_annotation_value3 and is never repinned or re-encoded.

For protocolOwner=D6 current D7 actions, InputDescriptor/3.ownerInput has protocolOwner=D7 and ownerKind=intentKind=d7_action/3; canonicalDescriptorBytes is exactly D3-CJ/3(D7ActionInput/3). The six D7ActionInput/3 members equal their PreparedActionBinding/4 counterparts individually, and ownerInput.pinRefs continues to cover exactly proposed pins plus actually protected source/definition/rule evidence under D6 sorted/unique pin rules. protocolOwner=D3 identity actions such as create_annotation keep D3's own d3_identity_operation/13 owner descriptor; PAB4 binds cross-owner preparation and creates no second D3 request authority. Historical d7_action/2/PAB3 keeps its original decoder, bytes and recovery.

## 6.2 EffectManifest/3 / EffectBytes/3

```text
EffectManifest/3 = {
  format:"weftext.effects",version:3,
  phase:"preview"|"committed",
  protocolOwner:"D3"|"D6",operationId:UUIDv4,
  workspaceRef:WorkspaceRef,profile:"full"|"owner_fields",
  items:[EffectItem/3...],decisionKey:DecisionKey/2
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

EffectItem/3 retains the fixed-parent fourteen arms and adds exactly one current owner-specific arm, presentation_policy_change:D8PresentationPolicyEffect/1. This is the same pattern as series_configuration_change: it is a typed shared-configuration owner effect, not a new PortableComponentKey, Policy/3 mutation, Registry value, author source, or generic JSON. Every byte slot uses EffectBytes/3, Annotation source images use Value/4, and current workspace bootstrap admits Plan4. Historical EffectItem/1-/2 decoders are not widened.

## 6.3 PreparedEditBinding/3

```text
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
    {kind:"document",
     target:D8SourceTarget/2,
     proposedSource:PinRef/2}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2,
     targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",
     target:D8SourceTarget/2,
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

D8AnnotationDraftProjection/1 =
    {kind:"d8_annotation_draft",version:1,
     annotationRef:AnnotationRef,
     baseObservation:SourceObservation/1,
     baseRevisionToken:AnnotationRevisionToken/1,
     draftSerial:Counter,
     access:"editable"|"readonly",
     value:AnnotationEditableValue/1,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     body:{state:"valid",source:text}}
  | {kind:"d8_annotation_draft",version:1,
     annotationRef:AnnotationRef,
     baseObservation:SourceObservation/1,
     baseRevisionToken:AnnotationRevisionToken/1,
     draftSerial:Counter,
     access:"editable"|"readonly",
     value:AnnotationEditableValue/1,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     body:{state:"invalid",source:text,diagnostics:[CoreDiagnostic/1...]}}

PreparedEditBinding/3 = {
  kind:"d8_prepared_edit_binding",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  operationId:UUIDv4,principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  intent:D8EditIntent/3,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request},
  sourceInputs:InputDescriptor/3.sourceInputs,
  proposedInputs:[{entityRef:EntityRef,pin:PinRef/2}],
  registryInputs:[ValidatedCatalogContext...],
  dependencyProof:DependencyProof/3,
  observationProof:PreparedIntent/3.observationProof,
  budgetBinding:BudgetBinding/1,expiresAt:PreparedDeadline,
  request:d6_commit_request/2,preview:EffectManifest/3
}
```

For document intent, proposedInputs.pin selects exact UTF-8 source. For annotation intent, proposedInputs has exactly one item whose entityRef equals target.ref and whose PinRef/2 payloadKind is annotation_value; the pinned bytes are exactly D3-CJ/3(the complete Core-constructed D3-Annotation-Value/4), never AnnotationEditableValue/1 alone. target.ref is AnnotationRef and expectedAnnotationRevisionToken equals the current PortableAnnotationRecord/4 token at prepare. Annotation requires saveProfile=complete and writeProtection=strict. observed_only remains restricted to the retained qualified interactive Document path.

OwnerInputBinding/2 current ownerKind/intentKind is d8_edit/3 and canonicalDescriptorBytes is D3-CJ/3(D8EditInput/3). Its pinRefs are exactly the sorted/unique pins named by D8PinnedEditIntent/3 plus the retained origin evidence. D8EditInput/3.intent is mechanically derived from PreparedEditBinding/3.intent by replacing only source/value bytes with the corresponding proposed pin; no actor/time or trust flag exists in either request shape. Historical d8_edit_prepare wire1/2 and PreparedEditBinding/1/2 keep their original intent/value decoders and recovery.

### 6.3.1 Current Annotation read / Draft producer and operation-class input

```text
D8AnnotationReadRequest/1 = {
  wireVersion:1,kind:"d8_annotation_read",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef
}

D8AnnotationBodyRead/1 =
    {state:"absent",exactSource:null,semanticText:""}
  | {state:"valid",exactSource:text,semanticText:text}
  | {state:"invalid",exactSource:text,semanticText:null,
     diagnostics:[CoreDiagnostic/1...]}

D8AnnotationReadResponse/1 = {
  kind:"d8_annotation_read",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef,
  sourceObservation:SourceObservation/1,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4,
  targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
  body:D8AnnotationBodyRead/1
}

D8AnnotationDraftOpenRequest/1 = {
  wireVersion:1,kind:"d8_annotation_draft_open",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef
}

D8AnnotationDraftOpenResponse/1 = {
  kind:"d8_annotation_draft_open",version:1,
  read:D8AnnotationReadResponse/1,
  draft:D8AnnotationDraftProjection/1
}
```

These are real Core/D8 entries, not response-only shapes. `d8_annotation_read` returns the complete Value/4 at the final authorized read barrier, including creator/authoredAt/lastEditor/editedAt for display. Draft still stores/edits only AnnotationEditableValue/1 and is never a second author authority. `d8_annotation_draft_open` reuses the same current read: draft.access=editable with annotation_write and readonly otherwise; readonly cannot enter D8EditPrepareRequest/3. Draft baseObservation/baseRevisionToken equal read.sourceObservation/read.annotationRevisionToken, and at serial=0 its editable projection equals the eight editable fields of read.value. Local Draft may subsequently diverge, but prepare and the final read barrier revalidate the original base token/Observation.

The D8EditIntent/3 operation class is mechanically derived from its closed arm and targetPolicy, never from a caller trusted flag: annotation+preserve=ordinary_edit, annotation+replace_current=manual_reattach, annotation_reconfirm_suggestion=reconfirm_suggestion. Caller AnnotationEditableProposal/1 carries no Suggestion lifecycle/evidence. Core first applies the closed operation-class gate to current before + proposal, expands one complete candidate Value/4 with before attribution preserved, and only then performs canonical candidate-vs-before no-op comparison. Only a differing candidate is upgraded with fresh Core lastEditor/editedAt, a fresh revision token and the unique planned/pinned final Value/4. The reconfirm arm carries no caller value; its proposedValue is entirely produced by a fresh Core target read/recomputation and a successful reconfirm is a real state transition.


## Part B — Annotation, SourceTransform, trust/bootstrap, and D10 mixed holders

# 7. Annotation closed values

```text
AnnotationRevisionToken/1 := nonempty opaque JSON string

PortableAnnotationRecord/4 = {
  kind:"portable_annotation",version:4,
  annotationRef:AnnotationRef,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4
}

AnnotationEditableValue/1 = {
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:Suggestion/3|null
}

D3-Annotation-Value/4 = {
  kind:"d3_annotation_value",version:4,
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:Suggestion/3|null,
  creator:AnnotationActorSnapshot/1,
  authoredAt:AnnotationTimeSnapshot/2,
  lastEditor:AnnotationActorSnapshot/1,
  editedAt:AnnotationTimeSnapshot/2
}

D3-Annotation-Target-Projection/1 =
    {kind:"document",owner:NodeRef}
  | {kind:"document_element",locator:DocumentElementLocator}
  | {kind:"document_range",locator:DocumentRangeLocator}
  | {kind:"resource",resourceRef:ResourceRef}
  | {kind:"resource_region",locator:ResourceRegionLocator}

AnnotationInlineBody/1 = {
  format:"asciidoc-inline",
  version:1,
  languageBaseline:"asciidoctor-ruby/2.0.26",
  source:text
}

AsciiDocInlineBody/1 := AnnotationInlineBody/1

AnnotationInlineProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  doctype:"inline",
  processorBackend:"html5-semantic/1",
  safeMode:"secure",
  maxSourceBytes:65536,
  maxRenderedBytes:262144,
  maxInlineSemanticNodes:4096,
  managedAdapters:"disabled",
  networkEffects:"denied",
  fileEffects:"denied",
  processEffects:"denied"
}

AnnotationAppearance/1 = {
  mark:"highlight"|"underline"|"squiggle"|"strike",
  theme:"yellow"|"red"|"green"|"blue"|"purple"|"pink"|"gray"
}

AnnotationActorSnapshot/1 = {
  kind:"annotation_actor_snapshot",version:1,
  originWorkspaceRef:WorkspaceRef,
  authentication:"workspace_authenticated_origin"|
                 "device_local_unverified"|"imported_unverified",
  displayName:text
}

AnnotationTimeSnapshot/2 = {
  instant:RFC3339-with-offset,
  producer:"prepare_server_clock"|"prepare_device_clock"|"imported_unverified"
}

Suggestion/3 = {
  version:3,kind:"replace"|"delete"|"insert",
  state:"pending"|"accepted"|"rejected",
  confirmation:"confirmed"|"needs_reconfirmation"|"not_applicable",
  targetBasisSha256:"sha256:<64 lowercase hex>",
  expectedText:null|{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  pointAffinity:null|"left"|"right",
  replacementSource:null|text
}
```

`AnnotationInlineBody/1` is the sole current name and preserves the four-member data shape originally named `AsciiDocInlineBody/1`. `AsciiDocInlineBody/1` is a **schema alias only**: its canonical bytes are exactly those of `AnnotationInlineBody/1`; it creates no second wire/version and asserts no historical deployment. The body is portable source data, not a processor profile. Evaluation always uses the single `AnnotationInlineProfile/1` above. The body's `languageBaseline="asciidoctor-ruby/2.0.26"` must correspond to the 2.0.26 version fixed by the profile's commit-qualified baseline. The complete source must form exactly one paragraph, allowing soft wraps and trailing whitespace; a second paragraph, heading, list, delimited block, table, or block macro is `invalid_annotation_body` rather than silently ignored.

Replies are same-owner, acyclic comments with suggestion=null and reviewState=not_applicable. Root reviewState is open|resolved. Pending suggestions use confirmed|needs_reconfirmation; accepted/rejected are terminal with confirmation=not_applicable.

PortableAnnotationRecord/4 is the current logical Portable Metadata record in the existing node-local annotations JSON authority. Its annotationRef owner matches the containing Node. D3-Annotation-Value/4 canonical bytes are exactly D3-CJ/3(value); current annotation_value payload bindings and PinRef/2 values hash/pin those bytes, not the outer PortableAnnotationRecord/4. annotationRevisionToken is opaque, never derived from the value digest, and is unique per committed Value/4 state for one AnnotationRef.

For current wire13 D3 mutation, the Value/4 base decoder keeps the inherited logical slots annotation_target ordinal0 and annotation_reply ordinal1. target is always a reference slot. replyTo is a reference slot when nonnull; an identity-preserving existing-Annotation reply change uses the inherited structural S form and cannot also produce a reply reference result. All other Value/4 members are nonreference bytes. Every changed current Value/4 gets one new final AnnotationRevisionToken/1, and all target/reply toSource addresses, annotation_reply_change evidence, SourceRevisionPlan/result pin, source change and receipt use that same token. Caller AnnotationEditableProposal/1 bytes are never compared directly with AnnotationEditableValue/1. Core first expands the proposal through the operation-class gate into one complete candidate Value/4 carrying before attribution; canonical candidate Value/4 equal to canonical before Value/4 is the no-op and keeps the complete Suggestion evidence, token/SourceVersion/H/attribution with no SourceRevisionPlan. Only a differing candidate receives fresh lastEditor/editedAt and a fresh final token.

Current payloadBindings with payloadKind=annotation_value dispatch by actual request family: wire13 current mutation strict-decodes Value/4, while genuine historical wire9-12 plans keep their Value/3 decoder. D3-Symbolic-Result/9 framing is unchanged; its annotation base bytes and slot spans are computed from the decoder selected by that request family. This does not widen the historical decoder.

The current Portable Metadata path does not materialize D2 Annotation-v2 outer wire as author state. Historical D2 v2 annotation snapshots, Value/3, plain_text body, replace_plain_text suggestion and targetStatus resolved/stale retain their real historical decoder/recovery only. Current body bytes are AnnotationInlineBody/1 and use exactly the single AnnotationInlineProfile/1; no plain-text compatibility fallback or second parser exists.

## 7.1 Caller proposal and Node-local physical aggregate

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

SuggestionAuthorProposal/1 contains only author-proposed kind/replacementSource: replace requires replacementSource (which may be empty), delete requires null, and insert requires nonempty replacementSource. state, confirmation, targetBasisSha256, expectedText, and pointAffinity are never caller proposal fields. AnnotationEditableProposal/1 keeps the same purpose/reply/body/appearance/labels/review invariants as AnnotationEditableValue/1; the Core operation-class gate produces the controlled Suggestion/3 before→after. The gate has five derived current-state classes rather than treating null as absence: fresh create=`absent_annotation`; existing suggestion=null=`no_suggestion`; pending=`pending_confirmed|pending_needs_reconfirmation`; terminal=`terminal_accepted|terminal_rejected`. interactive_create admits legal no-suggestion comment/mark/reply and legal root suggestion. ordinary_edit admits no_suggestion→no_suggestion, no_suggestion→pending+needs_reconfirmation after real target qualification, pending→pending, pending→no_suggestion, and terminal→the same terminal only. manual_reattach admits no_suggestion→no_suggestion or pending→pending+needs_reconfirmation and never combines reattach with category conversion. A no-suggestion edit with unchanged target and a pending→no_suggestion clear do not acquire target-source-read authority; conversions/operations that consume target content use only their named qualification/read step. After this gate Core expands a complete candidate Value/4 with before attribution and compares that canonical Value/4 to before; Proposal bytes are never a no-op comparator.

The current physical bytes of `weftext.annotations.json` are exactly D3-CJ/3(the complete AnnotationAggregate/1), with no BOM, trailing newline, or second envelope. records is nonempty, sorted by complete canonical AnnotationRef bytes and unique; every annotationRef.owner equals ownerNodeRef, every nonnull replyTo has that owner, and the complete records reply graph is acyclic. The unique physical representation of an empty set is an absent sidecar. Unknown/missing members, duplicate JSON keys, unknown version, owner mismatch, duplicate Ref, or reply cycle fail the whole aggregate strict decode; a normal current read never partially trusts records that happen to look valid.

AnnotationAggregateObservation/1 is a physical-file observation, not an Annotation SourceVersion/SourceObservation, revision token, or identity. `FileObjectBinding/1` and `FileInstallCapability/2` are reused **byte-for-byte from fixed D6 Control Interfaces §2**, not versioned here: absent binding=`{kind:"absent",backendToken,relativePath,observationEpoch,parentGenerationToken}`; present binding=`{kind:"present",backendToken,relativePath,observationEpoch,objectGenerationToken,byteLength,sha256}`; capabilities remain create_only(parentGenerationToken), conditional_replace(expectedObjectGenerationToken), exclusive_write_window(windowToken,expectedObjectGenerationToken), and observed_replace(observedObjectGenerationToken). PortableRelativePath, Token/Counter, 64-lowercase-hex, containment, and continuity retain that owner's exact rules. fileObjectBinding.kind=absent requires aggregateBytesPin=null; kind=present requires a PinRef/2 with payloadKind=portable_metadata whose bytes are the complete physical JSON above. A digest alone is never CAS. AnnotationAggregateInstall/1 is only a named file-write coordination binding inside the existing D6 installation plan, not a new ledger/CAS/author store; before is always the fresh Observation, never a bare `"absent"`. A PinRef after selects the complete after aggregate and `"absent"` deletes the last record. installCapability cross-fields exactly with the before FileObjectBinding: absent→create_only with equal parentGenerationToken; present→a D6-permitted replace capability with exact generation/window token. The managed_atomic strong path still accepts only that owner's strict capabilities; observed_replace remains valid only where D6 §4.1 already grants observed_only eligibility and never becomes CAS. changedAnnotationRefs is canonical sorted/unique and exactly the logical record difference between before and after. A DecisionKey has at most one AnnotationAggregateInstall/1 for one Node.

# 8. SourceTransform

```text
PortableTransformCompilation/1 =
    {kind:"representable",events:[SourceTransformPortableEvent/3...],
     afterSourceSha256:"sha256:<64 lowercase hex>"}
  | {kind:"unavailable",
     reason:"provenance_gap"|"unsupported_transaction"|
            "invalid_utf8_boundary"|"generated_cross_anchor_edit"|
            "boundary_slot_unrepresentable"|"provenance_cycle"|
            "after_replay_mismatch"|"payload_digest_mismatch"}

SourceTransformPortableEvent/3 =
    {kind:"replace",startByte:Counter,endByte:Counter,
     removedByteLength:Counter,removedSha256:"sha256:<64 lowercase hex>",
     replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}
  | {kind:"insert",atByte:Counter,replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}

TransformEmissionPlan/1 =
    {kind:"disabled",
     reason:"no_exact_core_edit_plan"|"transform_profile_unavailable"}
  | {kind:"required",profile:"d6_source_transform_seal/1",
     expectedTrustRevision:Counter,
     expectedTrustKeyId:"sha256:<64 lowercase hex>"}

CoreSourceEditPlan/2 = {
  kind:"d6_core_source_edit_plan",version:2,
  decisionKey:DecisionKey/2,ownerNodeRef:NodeRef,
  beforeObservation:SourceObservation/1,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,transformEmission:TransformEmissionPlan/1
}

SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",version:2,
  decisionKey:DecisionKey/2,changeId:ChangeId/1,ownerNodeRef:NodeRef,
  before:SourceVersion/2,after:SourceVersion/2,
  beforeSourceSha256:"sha256:<64 lowercase hex>",
  afterSourceSha256:"sha256:<64 lowercase hex>",
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}

SourceTransformSealSignedBody/1 = {
  format:"weftext.source-transform-seal",version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2
}

SourceTransformSealArtifact/1 = {
  format:"weftext.source-transform-seal",version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2,
  signature:"<86 ASCII unpadded base64url>"
}

SourceTransformSealKey/1 = {changeId:ChangeId/1,ownerNodeRef:NodeRef}

SourceTransformSealOutboxItem/1 = {
  kind:"d6_source_transform_seal_outbox_item",version:1,
  key:SourceTransformSealKey/1,artifactPin:PinRef/2
}
```

Plan/evidence edit canonical bytes must match exactly. Seal never recompiles/reorders/merges. A required sealed decision has exactly one outbox item; disabled has none.

Event3 validation is closed against exact beforeBytes/afterBytes. replace requires 0<=startByte<endByte<=before length, UTF-8 scalar boundaries, removedByteLength=endByte-startByte, and removedSha256=SHA-256(beforeBytes[startByte:endByte]); insert requires 0<=atByte<=before length, a UTF-8 scalar boundary, and non-zero replacementByteLength. Replacement intervals are non-overlapping maximal islands; same-point inserts are merged in transaction order; an insert strictly inside a replacement island is folded into that island or compilation is unavailable. Canonical boundary order is left replacement ending at p, insert@p, right replacement starting at p.

SourceTransformEvidence/2.beforeSourceSha256 is exactly "sha256:" + lowercase_hex(SHA-256(exact complete before-source bytes corresponding to evidence.before and ownerNodeRef)). Receiver validation obtains those historical bytes from the producing CP/history and retained version evidence, recomputes the whole-source digest, and requires equality before accepting the artifact. Event removedSha256/length, successful slice replay, equal SourceVersion fields, or current-file bytes cannot substitute. Missing exact-before bytes follows the existing proof/state-unavailable boundary; a proved digest contradiction is integrity failure. This cross-field adds no member and does not version SourceTransformEvidence/2.

generatedOutputSpan uses the SPEC §11.1 replay-cursor algorithm over exact before/after pins. It byte-compares every unchanged gap, assigns the event span [afterCursor,afterCursor+replacementByteLength), validates that exact after slice against replacement length/hash, advances replace beforeCursor to endByte while insert leaves it at q, and requires the final tails and afterSourceSha256 to match. Content search is forbidden. Mapping defines delta(replace)=replacementByteLength-(endByte-startByte) with checked signed arithmetic.

SourceTransformSealSignedBody/1 is exactly SourceTransformSealArtifact/1 with only signature removed. signature is exactly 86 ASCII unpadded-base64url characters decoding to 64 Ed25519 bytes. The authenticated message is exactly ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1). Complete transported/stored artifact bytes are exactly D3-CJ/3(SourceTransformSealArtifact/1 including signature); alternate JSON serialization is rejected.

# 9. D6 dual-profile trust

```text
SealProfileId/1 =
  "d6_revision_token_seal/1"|"d6_source_transform_seal/1"

WorkspaceTrustPredecessor/2 =
    {kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}
  | {kind:"declaration",revision:Counter,
     sha256:"sha256:<64 lowercase hex>"}

WorkspaceTrustDeclaration/2 = {
  kind:"d6_workspace_trust_declaration",version:2,
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,
  decisionKey:DecisionKey/2,
  action:
      {kind:"authorize",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>"}
    | {kind:"rotate",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,oldTrustKeyId:"sha256:<64 lowercase hex>",
       newTrustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
       publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>",
       continuitySignature:"<86 ASCII unpadded base64url>"|"not_required",
       mode:"ordinary"|"loss_recovery"|"compromise"}
    | {kind:"revoke",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
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
  WorkspaceTrustDeclaration/2 with only rootSignature removed

WorkspaceAuthorizationBundle/2 = {
  kind:"d6_workspace_authorization_bundle",version:2,
  workspaceRef:WorkspaceRef,authorizationRevision:Counter,
  policy:Policy/3,trustRoot:WorkspaceTrustRootDeclaration/1,
  trustRevision:Counter,
  trustDeclarations:[WorkspaceTrustDeclaration/1|WorkspaceTrustDeclaration/2...]
}

DomainSealKeyHandle/2 = {
  kind:"d6_domain_seal_key_handle",version:2,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
  secureHandle:Token,state:"staged"|"usable"|"retired"|"lost"
}

SourceTransformSealVerificationKey/1 = {
  kind:"d6_source_transform_seal_verification_key",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:"d6_source_transform_seal/1",
  trustKeyId:"sha256:<64 lowercase hex>",
  algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>"
}

TrustConflictCarry/2 = {
  kind:"d6_trust_conflict_carry",version:2,
  factId:"sha256:<64 lowercase hex>",workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  compromisedTrustKeyId:"sha256:<64 lowercase hex>",
  originAction:"revoke"|"rotate",originDecisionKey:DecisionKey/2,
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

The public conflict request remains fixed-parent d6_conflict_prepare wireVersion3 with ConflictResolution/2. For a current unseen policy_bundle_choice, the same selected/policy/freshAuthorizations JSON arm is strict-decoded with the following current inner and protected descriptor types. Source-merge and choose-source-head keep the fixed-parent ConflictResolutionInput/2 / ConflictResolutionDerivedPlan/1 path.

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

FreshDomainAuthorizationSpec/2 has exactly the same two JSON members as historical /1 but profile is the full SealProfileId/1 union. The array is canonical D3-CJ/3 sorted/unique and contains no key material. Current ConflictResolutionInput/3 is a protected owner-descriptor successor only for policy_bundle_choice; it does not version the public request, add a submit path, or change conflict_resolve/policy_admin/disclosure/error order. A genuinely proven saved/planned old owner descriptor retains its recorded decoder, preview and pins.

PolicyBundleHeadEvidence/2 is complete, ChangeId-sorted/unique and byte-equal in head set to expectedKey.heads. version=3 means branchEvidence.completionProofPin strict-decodes ContentCompletionProof/3 and the exact policy after-image strict-decodes WorkspaceAuthorizationBundle/1. version=4 means completionProofPin strict-decodes ContentCompletionProof/4, branchEvidence.changeRecordPin strict-decodes the matching ChangeRecord/1 and exact Notice3/CP4 chain, and the policy after-image strict-decodes the declared Bundle1 or Bundle2 version. In both arms policyBundlePin is present, pins the exact canonical bundle bytes, and WorkspaceAuthorizationBundleAddress/1 authorizationRevision/trustRevision/byteLength/sha256 matches them. For policy_bundle_choice every branchEvidence.sourcePins array is empty. CP3+Bundle2, unknown versions, a missing CP4 ChangeRecord, tag/byte mismatch, or decoder fallback is rejected.

TrustConflictCarryValidationEvidence/1 is the complete retained validation path for one effective fact on one expectedKey head. The array is sorted uniquely by head ChangeId then ASCII factId and contains exactly one item for every (head,factId) used by the resolver's per-head effective compromise fold. origin is the direct compromise declaration named by the Carry; carriers are every resolve_conflict declaration actually traversed after that origin on that head, in ascending declaration revision. A version-1 hop strict-decodes its exact historical ChangeRecord/CP3/Bundle1 activation evidence; a version-2 hop strict-decodes ChangeRecord/1 + CP4 + Bundle2. declarationDigest and activationChangeId are byte-equal to the declaration and activation cut proved by those exact pins. The origin hop matches originDeclarationRevision/originDeclarationDigest/originActivationChangeId of the Carry, and its action/key mapping is rechecked. Every carrier hop must be a valid resolver that actually carries that fact. Missing a traversed carrier, replacing an origin cut with resolver time, or substituting equal current bytes fails.

For the current policy arm, OwnerInputBinding/2.protocolOwner is D6 and ownerKind is exactly d6_conflict_resolution/3; InputDescriptor/3.intentKind is byte-equal to that ownerKind. canonicalDescriptorBytes is exactly D3-CJ/3(ConflictResolutionInput/3). OwnerInputBinding/2.pinRefs is the canonical PinRef sort, duplicate-free union of exactly: every branchEvidence.changeRecordPin; every branchEvidence.completionProofPin; every present branchEvidence.policyBundlePin; selectedBundlePin; resultBundlePin; and every origin/carrier changeRecordPin, completionProofPin and policyBundlePin in carryEvidence. No hash-only declaration reference, current bundle, derived-index row, or unstated pin may replace one of these entries. The selected bundle pin may equal its branch policyBundlePin and is then represented once after set deduplication.

ConflictResolutionPreview/2 is the immutable current policy preview. branchEvidenceDigest is exactly "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(the complete ConflictResolutionInput/3.branchEvidence array))). The preview's conflictId, expectedKey and resolution are byte-equal to Input3 and its derivedPlan is byte-equal to the complete Plan2, including headEvidence, carryEvidence, mixed Carry1/2 values, Outcome2 values and resultBundleVersion/pin. PreparedIntent/3.previewBinding binds exactly this Preview2 and its exact canonical preview pin. For this control_only policy arm, PreparedIntent/3.pinDirectory is the canonical duplicate-free union of OwnerInputBinding.pinRefs, every DependencyProof/3 evidencePin, that exact preview pin, and every proposal/before/after/recovery PinRef actually named by the fixed installationPlan; sourceInputs is empty, so it contributes no SourceObservation evidence pins. The same pin cannot be rebound to a different protected record.

The two current source arms do not use these policy successors. source_merge and choose_source_head retain ConflictResolutionInput/2, ConflictResolutionDerivedPlan/1 and ConflictResolutionPreview/1, with OwnerInputBinding.ownerKind and InputDescriptor/3.intentKind both d6_conflict_resolution/2 and canonicalDescriptorBytes exactly D3-CJ/3(Input2). Their original complete pin union, source semantic evidence, saveProfile=complete, scope selection and Preview1 rules remain normative, while the outer unseen carrier is the current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family and planToken d6_plan/3.

For Carry2, originDeclarationDigest is exactly "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(complete original WorkspaceTrustDeclaration/2 including rootSignature))). Direct Declaration2 compromise mapping is revoke -> action.trustKeyId and rotate -> action.oldTrustKeyId. Define the exact Carry2 fact body as the nine fields workspaceRef, commitDomain, profile, compromisedTrustKeyId, originAction, originDecisionKey, originDeclarationRevision, originDeclarationDigest, originActivationChangeId in that closed object order under D3-CJ/3. Then:

```text
factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL ||
      D3-CJ/3(Carry2 fact body)
    )
  )
```

originActivationChangeId is rederived only from the original Declaration2 DecisionKey through committed CP4 + ChangeRecord/1 and the exact Bundle2 after-image that first appended it. Carry1 keeps its fixed-parent /1 fact domain and Declaration1/CP3 origin rules. A Declaration2 resolve_conflict may inherit Carry1 and Carry2; each member is recursively revalidated back to its direct original compromise declaration. effectiveCompromises and inheritedCompromises are ASCII factId sorted, unique, and contain byte-equal values for duplicate factIds; same factId with non-byte-equal canonical bytes is integrity_conflict. The effective union includes selected and losing branches, so branch choice never discards a compromise fact.

Bundle1 normalizes source-transform state to none. affectedDomainProfiles is the union of all differing normalized domain/profile states plus all pairs named by the effective compromise union. TrustConflictOutcome/2 is canonical sorted/unique by D3-CJ/3(commitDomain,profile) and complete for every affected pair when a trust-resolution declaration is needed. keep_current is legal only for a selected current key that remains safe under the complete effective union. A requested affected pair may authorize_fresh; its new key uses DomainSealKeyHandle/2 and PoP/2. A policy-only result with no trust resolution and no fresh authorization preserves selectedBundleVersion/trustRevision. Otherwise exactly one WorkspaceTrustDeclaration/2 resolve_conflict is appended and resultBundleVersion=2; a selected Bundle1 is preserved byte-exact as its Declaration1 prefix before the new Declaration2.

ConflictResolutionPolicyDerivedPlan/2 freezes the exact per-head proof/bundle dispatch, selected bundle pin, complete per-head carry validation evidence, complete carry union, inherited subset, Outcome2 values, and exact result bundle pin/version. resultBundlePin must strict-decode the derived result and reproduce its canonical bytes. The original one planning CAS freezes InputDescriptor/3, OwnerInputBinding/2, Preview2, pinDirectory, staged fresh-handle associations and installationPlan together. Final submit remains only d6_commit_request/2. The one final P publishes the current policy component through Notice3/CP4/ChangeRecord1 and the same conflict-record transition; staged authorize_fresh handles become usable only in that same commit. Planned recovery restores those exact Input3/Plan2/Preview2 bytes and pins and never rebuilds them from current history. Receiver validation recomputes all head dispatch, Carry1/Carry2 facts, every retained carry-validation hop, mixed recursive union, PoP/2 and root signatures, exact result bundle, preview/descriptor cross-fields, and same-decision conflict-record transition.

Every publicKey decodes to exactly 32 Ed25519 bytes and hashes to the corresponding trustKeyId. Every signature lexical value decodes to exactly 64 Ed25519 bytes. For authorize/rotate, possessionSignature signs exactly ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2). Rotate maps newTrustKeyId to the PoP body's trustKeyId. authorize_fresh constructs the same body from the enclosing Declaration2 workspaceRef/revision/predecessor/decisionKey plus that exact outcome's commitDomain/profile/key tuple. Ordinary rotate continuitySignature signs exactly ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2); loss_recovery/compromise require the literal "not_required". rootSignature signs exactly ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2).

Revision-1 root predecessor fingerprint is the complete WorkspaceTrustRootFingerprint/1 and is byte-equal to the retained root/anchor fingerprint; rootKeyId never substitutes. Declaration/1 remains revision-token-only. Bundle2 permits a Declaration1 historical prefix; after the first Declaration2 every successor is Declaration2. Declaration1 activation remains CP3. Declaration2 activation is rederived through same-DecisionKey ChangeRecord1 -> exact CP4 -> policy after-image -> Bundle2.

Current unseen ordinary trust management uses only the wireVersion3 prepare successors above. They carry no caller key material and preserve the fixed-parent policy_admin/root-handle/current-state gates plus the original planning/install/single-P path, now with CP4. Saved/planned wireVersion2 records keep their original decoder and recovery.

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
  profileRevision:Counter,registrySeedBinding:RegistryBinding/1,
  newSeriesMultiplicity:"unique"|"many",
  initialPeriodScope:"workspace"
}

WorkspaceBootstrapCreatorBinding/1 = {
  issuerPrincipal:Token,targetPrincipal:Token,
  principalAudienceToken:Token
}

WorkspaceBootstrapTargetRegistry/1 = {
  snapshot:RegistrySnapshot/1,binding:RegistryBinding/1
}

WorkspaceBootstrapSeriesConfiguration/1 = {
  seriesScope:SeriesScope,multiplicity:"unique"|"many",revision:1
}

WorkspaceBootstrapPeriodScopeBinding/1 = {
  nodeRef:NodeRef,scope:CalendarScope,revision:1
}

WorkspaceBootstrapPlan/4 = {
  kind:"d6_workspace_bootstrap_plan",wireVersion:4,
  operationId:Uuid,proposalId:Uuid,
  issuerAuthorityInstanceId:Uuid,targetWorkspaceRef:WorkspaceRef,
  targetAuthorityInstanceId:Uuid,profile:WorkspaceBootstrapProfile/4,
  creatorBinding:WorkspaceBootstrapCreatorBinding/1,
  targetRegistry:WorkspaceBootstrapTargetRegistry/1,
  initialPolicy:Policy/3,trustGenesis:WorkspaceTrustGenesis/2,
  initialPresentationPolicy:D8PresentationPolicyBootstrapInit/1,
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
```

All UUID members use the canonical lowercase D3 UUID decoder. Genesis has exactly two declarations: revision 1 revision-token authorize and revision 2 source-transform authorize, same DecisionKey/activation ChangeId, with rev2 predecessor hashing exact rev1 canonical bytes. Series configurations are unique and sorted by canonical SeriesScope; period bindings are unique and sorted by full NodeRef with exactly one per valid prepared period. The helper members retain the fixed-parent Plan3 field semantics; Profile4 changes only the dual-profile genesis family.

Plan4 is only for unseen fresh create/fork and preserves the original D3 proposal/custody/CAS/P boundary. Ordinary copy is not Plan4. Saved/planned/unknown recovery keeps the actual recorded decoder and bytes; restore/continue/failover do not synthesize Genesis2. No deployment or migration from Plan3 is asserted.

initialPresentationPolicy is mandatory and closes the D8 fresh-target state inside that same Plan4. Its before/proposal WorkspaceRef equals targetWorkspaceRef; before is the protected fresh empty-head observation from §6.4 with stamp.revision=1; proposal is exactly parents=[], revision=1, defaultPresentation=separate. Prepare/planning/staging freeze that helper and one typed presentation_policy_change preview with committed=null under the original create/fork OperationId, planning CAS and DecisionKey. This is not a D8 SetRequest, needs no already-active target policy_admin, creates no second CAS/ledger, and creates no committed policy record/hash/address/pin/outbox/ChangeId before final P.

Only after every original bootstrap final check passes does the same P checked-allocate the one bootstrap ChangeId C. In that atomic P transaction Core constructs the canonical D8WorkspacePresentationPolicy/2 from the frozen proposal+C, computes its fixed-prefix hash, exact recovery portable_metadata pin/address and D8PresentationPolicyOutboxItem/1, commits presentation_policy_change plus the original receipt/effects/ChangeRecord association, advances the D8 head graph from the frozen empty before to heads=[address] with the same epoch and checked stamp revision=2, and commits target activation/custody plus every other bootstrap effect. record.activationChangeId and ChangeRecord.changeId are C and the outbox decisionKey is the original create/fork DecisionKey. A planning loser, abort, failed final check or failed P commit leaves the target unactivated and leaves no partial D8 record/pin/head/outbox. A successful active Plan4 target therefore immediately has one /1 logical current view at revision=1/defaultPresentation=separate. Post-P publication retry uses only the exact retained outbox bytes and normal receiver same-decision/ancestry admission.

Ordinary managed copy is not Plan4 and does not synthesize this initialization. An actual saved/planned/unknown Plan4 restores its exact original initialPresentationPolicy, OperationId/proposal/custody mappings, pins and decoder; it never resamples [] or the current branch. Restore/continue/failover preserve their recorded/current policy state instead of re-running bootstrap. Genuine historical Plan1/Plan3 bytes keep their actual decoder and receive no injected field. This candidate asserts no migration or dual-write for an already active Workspace.

Legacy Handle1 may continue new revision-token signing only while its exact Declaration1-authorized key remains current, safe, and usable; it never signs transforms. Continuation evaluates revision and transform profiles independently as current(K)|none|conflicted_or_unproved; any unproved state prevents partial new-domain activation. The declaration order is optional old revision revoke, optional old transform revoke, new revision authorize, new transform authorize, all under one DecisionKey/CP4 with no observable intermediate prefix.

# 10. D10 mixed-version outer schemas

The following are current outer-holder schemas. Every versioned carrier strict-decodes the exact tagged inner type and retains its original canonical bytes/pins. Unknown tags fail closed. A carrier is only a decoder discriminator: it grants no authority, creates no decision/CAS, and never migrates the inner record.

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

For a current unseen external-consent preparation, confirmationRequirement is the same closed ExternalConfirmationRequirement/1 used by the fixed-parent attended-confirmation protocol and is frozen inside ControlPrepareBinding/3 before the original commit request is delivered. The mutable confirmation fact remains exclusively in ExternalConfirmationRecord/1 and is not inserted into Binding3 or ControlDependencies/3. Current confirmation eligibility therefore consumes Binding3 + Dependencies3 while preserving the original trusted-event, principal, time-window, immutable-intent/preview, disclosure, approval_unavailable/preflight, saved-result replay, and result-redaction rules. A proven historical ControlPrepareBinding/2 retains its exact requirement, Dependencies2, canonical intent bytes, confirmation association, and recovery decoder.

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

Image2 is closed to automation/run. Every other record kind keeps exact Image1. Pin1 keeps its historical D10-Control-Record/1 prefix and decoder. Pin2 contains exactly UTF8 D10-Control-Record/2, NUL, and D3-CJ/3(Image2); its PinRef/2 is artifact with the original recovery or approval_money retention and exact prefixed byteLength/SHA-256. A schema-tag/payload-domain mismatch fails. Pin2 never repins Image1.

For D10VersionedControlRecordPin/1 arrays let I=carrier.value.image. Sort by D3-CJ/3(I.binding.ref), binding.revision numeric, usageRevision none before some, numeric usageRevision when present, then D3-CJ/3(I). The logical record-cut identity is I.binding.ref + I.binding.revision + I.usageRevision. A second byte-equal image at that identity is a duplicate and is rejected; different bytes are integrity_conflict. Version does not split the identity.

Range kind rank is records=0, cost_lineage=1, occurrences=2. The cross-version logical identity is respectively scope plus the complete sorted kinds, CostLayerKey, or Automation Ref. Sort mixed ranges by rank then canonical identity bytes and allow one value per identity. An occurrences range sorts its records by AutomationOccurrenceKey; one key cannot occur in both V1 and V2.

D10ControlEffectPlan/2.changes sorts uniquely by the complete canonical after.value.binding.ref. When before is present, before.binding.ref equals after.binding.ref and recordPins contains exactly the versioned pin for the actual stored before schema. Image1 before plus Pin2 is invalid. Image1 Automation+Subscription1 to Image2+Subscription2 is a valid current configure transition. A normal configure may continue the old subscription at the same generation when the scheduling owner continue rule succeeds; mixed support never forces replacement or background migration.

ControlDependencies/3 arrays keep their fixed owner order: configBindings and usageBindings by full Ref, authorizationGenerations by token, stopRefs by full Ref, recordPins/ranges by the mixed orders above. Each config binding has its exact matching image/pin, each usage binding has the same usageRevision image, and each stopRef has exact stop Image1/Pin1. All current bindings, usage revisions, range fences, pins, and Workspace evidence are captured from one actual Authority Store barrier. Combining barrier-A V1 evidence with barrier-B V2 evidence is not a complete dependency snapshot. Missing required historical bytes/decoder/pin is state_unavailable after disclosure; contradictory same-cut evidence is integrity_conflict.

D10ExecutionClaims/2 canonical identities are: recordPins as above; prepareBindings by inner StableControlKey across Binding1/2/3; leaseRuns by full Run ControlRef; authorSteps by run Ref plus stepId across versions; subscriptions by Automation Ref plus generation across versions; occurrenceRecords by AutomationOccurrenceKey; ranges as above; continuityPins by pinToken. Each identity is unique across versions. In particular Binding1(K) and Binding3(K) cannot coexist even if canonicalIntentBytes and originalCommitRequest are byte-equal. Binding1 retains its exact six-member historical shape; no kind, version, confirmationRequirement, /2-/3 dependency field, LWW, or repinning is introduced.

D10MoneyResponsibility/2 orders reservations uniquely by complete Binding<reservation>/1 canonical bytes, layers uniquely by CostLayerKey canonical bytes, recordPins/ranges by the mixed rules, and evidencePins uniquely by pinToken. D10ExecutionInventory/2 orders approvalUses uniquely by DecisionKey canonical bytes across versions, externalUnknowns uniquely by binding.ref canonical bytes, and stopState uniquely by binding.ref canonical bytes. Every pending/unknown/dedup/stop responsibility and every still-referenced completed external attempt is retained. The inventory is captured only after admission, planning, send, and schedule writers stop at one real store barrier.

The exact Inventory2 artifact payload is UTF8 D6-Execution-Inventory/2, NUL, D3-CJ/3(D10ExecutionInventory/2). The PinRef/2 is artifact/recovery and its byteLength/SHA-256 covers the complete prefixed bytes. Inventory1 keeps D6-Execution-Inventory/1 and is never repinned as /2.

Record3.workspaceRef equals Inventory2.workspaceRef. Record3.approvalUses, claims, moneyLineage, externalUnknowns, and stopState are each byte-equal to the corresponding Inventory2 member. Proof2.inventoryPin selects that exact Inventory2 and Inventory2.storeIncarnation equals Proof2.storeIncarnation. The protected birth/barrier/fence token mapping proves the same actual store and capture barrier; there is no separate caller-supplied store-incarnation proof object.

Inventory2.stopCapacity is the exact fixed-parent StopCapacity/1 at the same authoritative safety-store barrier. StopCapacity/1 remains only issued/reserved and is not duplicated in Record3. A Workspace-only handoff cannot copy the shared safety counters to a second active store. A complete store handoff must fence every affected writer and preserve all target/latch reservations and safety capacity; inability to prove that boundary makes takeover unavailable.

Record2/Proof1/Inventory1 remain historical and keep their exact decoder/domain. Record3 is emitted only for an actual responsibility mutation, checkpoint, or custody handoff; it preserves executionDomainId and never background-migrates an unchanged holder.

## 10.1 D10 current schedule / author-step direct types

These are the exact current inner values referenced by the §10 mixed wrappers; they are not free objects invented by the wrappers.

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

The current continuity artifact encodings are exact and version-separated:

```text
Witness2ArtifactBytes =
  UTF8("D6-Schedule-Continuity/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityWitness/2)

Step2ArtifactBytes =
  UTF8("D6-Schedule-Step/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityStep/2)

Invalidation2ArtifactBytes =
  UTF8("D6-Schedule-Invalidation/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityInvalidation/2)
```

Each corresponding PinRef/2 has payloadKind=artifact and retentionClass=recovery. Its byteLength and SHA-256 cover the complete domain prefix, the one NUL byte, and the complete canonical object bytes above. A digest never authenticates continuity by itself: the protected producer/subscription provenance, producerEpoch, exact generation, retained transition chain, and original authorization/disclosure gates remain required.

Decoder dispatch is closed by the authenticated artifact domain before inner decoding. D6-Schedule-Continuity/1 accepts only historical ScheduleContinuityWitness/1; D6-Schedule-Step/1 accepts only historical ScheduleContinuityStep/1; D6-Schedule-Invalidation/1 accepts only historical ScheduleContinuityInvalidation/1. D6-Schedule-Continuity/2 accepts only ScheduleContinuityWitness/2; D6-Schedule-Step/2 accepts only ScheduleContinuityStep/2; D6-Schedule-Invalidation/2 accepts only ScheduleContinuityInvalidation/2. Unknown domains, a known domain paired with the wrong kind/version, alternate prefixes, missing NUL, or non-canonical object bytes fail closed through the original disclosure/error boundary. There is no fallback decoder, widening of a /1 decoder, repinning, or re-encoding of historical bytes.

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

For a fresh current core_field_member step, D10AuthorPreparationLink/2.preparedBindingToken must select the exact PAB4 whose original request equals link.request. Core saves Link2, that complete PAB4, and the required preview/effect/recovery pins atomically before returning the prepared step or permitting submission. ApprovalUse/2.preparedBindingToken selects that same PAB4. Its preview digest is
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3));
each EffectBytes/3 slot is projected only as {encoding,byteLength,payloadDigest}, never discovered by recursively guessing member names. Fresh current automatic author qualification uses DependencyProof/3, including document_format whenever the managed Document is semantically parsed, and constructs ApprovalUse/2 only after complete EffectManifest/3 / EffectBytes/3 / MutationFootprint validation. Historical Link1/PAB3/ApprovalUse1 saved or planned associations keep their original decoder, bytes, pins, request and OperationId.

For a fresh ScheduleSubscription/2 registration, the current D6 Storage producer creates ScheduleContinuityWitness/2 only in the same configuration transaction that passes the selected source/Field/Registry/current scheduling gates, reserves finite retention, and attaches the actual continuously maintained Core source/control transition producer. initial and checkpoint are the subscription's exact ScheduleRecurrenceEvidence/2, revision=1, consumedTransition=0, and producerEpoch is fresh. The retained witness pin is exactly Witness2ArtifactBytes above. Positive current transitions use ScheduleContinuityStep/2 with DependencyProof/3 and the actual ChangeRecord/1, InstallationNotice/3, and ContentCompletionProof/4 pins for new current portable transitions; each retained current step pin is exactly Step2ArtifactBytes above. Historical transitions inside retained history keep their original exact /1 artifact domains and decoders. P-only relevant control/rule transitions remain captured from their actual protected before/after state.

Fold, compaction, receiver admission, D10 continuityPins consumption, and recovery all dispatch each protected schedule-continuity artifact by its exact domain before decoding the inner object. A current Witness2/Step2 is never accepted under a /1 domain, and a historical Witness1/Step1 is never accepted under a /2 domain. continuityPins are token-sorted/unique exact PinRef/2 values and may retain a version-mixed original typed chain when real history crosses the explicit same-generation bridge; each element keeps its own exact bytes/domain/decoder. The alternative full retained-chain path likewise keeps each original typed artifact and source/control evidence rather than normalizing the chain to one version. Missing or unknown typed evidence produces the original gap/unavailable behavior after the ordinary authorization/disclosure checks; a proved domain/object mismatch is never repaired from equal digest/current state.

When no valid current after evidence can exist, the current producer emits ScheduleContinuityInvalidation/2 instead of fabricating ScheduleRecurrenceEvidence/2. binding_changed requires complete trusted evidence of a selected-business discontinuity; unavailable/unknown after, missing history, unknown decoder, observer/producer gap, or inability to retain a required transition is gap. Invalidation compares the same current witness/registration, atomically advances the next checked transition/revision, keeps the last valid checkpoint, and is permanently non-resetting for that generation. Its artifact pin is exactly Invalidation2ArtifactBytes above. The fixed-parent inbox/capacity/final-counter reservation, authorization, compaction and unrelated-source-availability rules remain unchanged.

A current schedule proof that semantically parses a managed Document must include both source and document_format dependencies. If source/profile bytes are unchanged but format-proof continuity has a gap, the result is gap; a real binding transition is binding_changed even when the final recurrence/range value happens to compare equal. An existing Subscription1 remains a historical retention owner with Witness1/Step1/Invalidation1 and their exact /1 artifact domains. It may become a same-generation Subscription2 only through explicit continue plus complete retained history proving no intervening format/rule/business discontinuity and establishing the current Evidence2/Proof3 cut; otherwise replace is required. The bridge preserves every old pin/producer association and starts using /2 domains only for newly produced Witness2/Step2/Invalidation2 artifacts. It never repins or re-encodes a version-1 witness/step/invalidation as version 2 and never resets an invalid generation.

