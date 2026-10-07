---
source_language: zh-CN
translation_of: D8-SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](D8-SCHEMAS.zh-CN.md)

# A2 D8 Closed Schemas and Version Dispatch

Status: author-resolved-pending-independent-review. This file closes current D8 schema/version routing without widening historical decoders. Full owner semantics are in D8 and D8-INTERFACES. Embedded types remain owned by D2/D3/D4/D6/D7.

## 1. Current family table

| Family | Current author/read family | Historical handling |
| --- | --- | --- |
| Document read | D8 outer wireVersion2 + D2DocumentSnapshot/3 | Old D2 snapshots dispatch only by recorded decoder |
| Draft project / text replace / write | D8 wireVersion2 | wire1 remains historical only |
| Edit prepare | D8EditPrepareRequest/3 + D8EditInput/3 + PreparedEditBinding/3 | /1 and /2 retain exact historical recovery |
| D6 prepare/submit | InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 + d6_commit_request/2 | Recorded owner version dispatches first |
| Preview | EffectManifest/3 + EffectBytes/3 | Effect1/2 remain exact historical families |
| Annotation author value | PortableAnnotationRecord/4 + D3-Annotation-Value/4 | Value3/older Annotation records remain historical |
| Annotation read/draft | D8AnnotationRead*/1 + D8AnnotationDraft*/1 | No generic version coercion |
| Presentation policy | logical policy/1 + immutable record/2 + SetRequest/2 | Older prototype is not dual-read |
| View | D7 ViewSpec/1 | D8 does not bump it |
| Search | D7 QuerySpec/2 for new authoring; QuerySpec/1 remains exact | D8 never rewrites the saved Query version |

Similarity between outer version numbers never implies an inner schema upgrade.

## 2. Source target and document response

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

Cross-fields are exact: ownerNodeRef, Workspace/domain, sourceObservation, and the snapshot owner agree. A managed token authenticates to sourceObservation.sourceVersion. Snapshot/3 valid/invalid branches remain exactly D2-owned.

## 3. Draft projection shape

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

DraftFlow/2 is path, text, and segments. An editable segment has a nonnull sourceRange. Read-only escape/join/atom segments do not expose raw-scalar write permission through the edit map.

PlainRegion/2 is sourceRange, text, parentPath, childStart, and childEnd. text is the exact raw slice and may be empty, blank, or EOL-only. DraftSite/2 is sourceRange, parentPath, and childIndex; its sourceRange is zero width.

NavigationOrigins/2 remains separate from DraftEditMap/2. A navigation segment relation is scalar, escape, join, or atom, and its sourceRange always points to the real origin.

## 4. Draft write commands

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

All ordinals/offsets are proposal-local Counters and never survive a serial/revision change. replacementText for text_replace contains no CR/LF. splice/insert text may contain complete CR, LF, or CRLF. Every endpoint is an extended-grapheme boundary and CRLF is indivisible.

## 5. D8EditIntent/3 and prepare

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

Unknown arms, members, or illegal null reject. An Annotation caller cannot provide actor/time/trusted/suggestion evidence. The reconfirm arm contains no caller-provided value. SourceTransform profile/compilation availability is not a member of D8EditInput/3 or PreparedEditBinding/3 and does not become a second D8 save certificate; an upstream disabled/unavailable transform plan may coexist with an otherwise legal ordinary Source save.

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

proposedInputs contains exactly one item. For Document intent the pin selects exact UTF-8 source. For Annotation intent the pin payloadKind is annotation_value and the bytes are D3-CJ/3 of the complete Core-constructed Value/4. sourceInputs is byte-equal to inputDescriptor.sourceInputs. OwnerInputBinding uses ownerKind/intentKind=d8_edit/3, and canonicalDescriptorBytes is the complete D8EditInput/3.

## 7. Current Annotation read/draft

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

The editable Draft projection never contains complete attribution authority. readonly cannot prepare.

## 8. Annotation Value/4 and R6 references

D8 does not duplicate the D3-owned schema. Current D8 consumes only PortableAnnotationRecord/4; complete D3-Annotation-Value/4; AnnotationEditableValue/1 and AnnotationEditableProposal/1; AnnotationInlineBody/1 with format asciidoc-inline and profile R6; current Suggestion/3 state/evidence; the five-arm target union; and current AnnotationAggregate rules.

Value3/plain_text appears only in a genuine historical decoder. Current prepare/effects never downgrade their encoding.

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

EffectItem/3 retains its existing arms and adds the typed presentation_policy_change arm. Current EffectBytes/3 uses the owner-closed encoding set, including exact_source_utf8, resource_bytes, d3_annotation_value4, d3_symbolic_result9, d7_definition_transfer_effects2, d3_canonical_effects1, workspace bootstrap 1/3/4, d7_symbolic_json3, and field_entries2. D8 adds no free encoding.

## 10. Current presentation-policy schemas

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

The address recordSha256 binds the prefixed canonical /2 record bytes. parents and head arrays are sorted/unique. The logical /1 value is mechanically projected only from the unique current /2 record and is not persisted separately. Before final P there is no committed record/hash/pin/ChangeId.

## 11. Direction/layout state

The following values are D8 UI state rather than author wire: DirectionPreference=ltr|rtl|auto; CaretAffinity=upstream|downstream; LayoutEpoch; logical caret/selection anchor/focus; composition generation/transaction; and device-local View interaction such as hover, selection, legend hide, zoom, pan, and fold.

They are never encoded into QuerySpec, ViewSpec, Document source, Annotation Value, or a hidden portable sidecar.

## 12. Consumed Search/View schemas

D8 Search controls produce only the D7 compiler input and consume the real QuerySpec/2/CanonicalGraph. There is no new D8 Search wire. Saving a Query remains a D7 definition operation.

A D8 renderer consumes only D7 ViewSpec/1={format:"weftext.view",version:1,inputSchema,layout,bindings,options}. D8 adds no filter, sort, CEL, aggregate, or script member. The formal View validation error family remains D7-owned.

The View builder has **no portable schema**. Its working value is the complete strict-decoded `ViewSpec/1` plus ephemeral UI selection/focus/validation state. Saved Query/View/DynamicBlock occurrences remain current D2/D7 SavedDefinition author data; `DynamicBlock/1` retains only its real Query/View call and declared bindings. Builder control availability, advanced-route choice, validation messages, open tabs, selection, dirty state, or device layout are never serialized into ViewSpec/DynamicBlock or a hidden sidecar.

Definition-save validation is not a new wire or schema. It consumes only the existing D7 strict decoder, current definition owner/revision/authorization, current terminal schema, and static layout/binding/type/option rules. It consumes no ResultHandle. The existing D7 View §7 runtime validator remains the sole complete-result validator and is invoked only for render/export; its value/key/order/domain/hierarchy/budget/delivery errors do not mutate or invalidate the saved definition.

A basic builder may construct a new complete `ViewSpec/1` only from the exact closed members already owned by D7. It may not synthesize a partial ViewSpec, drop unknown-to-the-control but legal current members, or coerce a future/unknown version into version 1.

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

The family is inherited and closed. A coordinated current producer/path uses its real owner unavailable/conflict result rather than owner_update_required as a generic implementation placeholder.

## 14. Error/version matrix

The current D8 decoder cannot accept a /3 intent under old wireVersion2 prepare; Value3 under current /3 prepare; PreparedEditBinding/2 as /3; historical EffectManifest/2 as current /3; a historical D2 snapshot as Snapshot/3; query_json as QuerySpec; device direction/layout state as an author member; or historical Annotation plain_text as current R6.

Every genuine old record first dispatches by its recorded version/tag and then follows its original recovery contract. An unknown future version fails closed.
