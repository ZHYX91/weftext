---
source_language: zh-CN
translation_of: D8-SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](D8-SCHEMAS.zh-CN.md)
# A2 D8 Closed Schemas and Version Dispatch

Status: author-resolved-pending-independent. This file closes current D8 schema/version routing without widening historical decoders. Full semantics are in D8 and D8-INTERFACES; embedded values retain their original D2/D3/D4/D6/D7 owners.

## 1. Current family table

Current document read and Draft project/text-replace/write use D8 outer wireVersion2 with D2DocumentSnapshot/3. Current edit preparation uses D8EditPrepareRequest/3, D8EditInput/3, PreparedEditBinding/3, InputDescriptor/3, DependencyProof/3, PreparedIntent/3, d6_commit_request/2, EffectManifest/3 and EffectBytes/3. Current Annotation uses PortableAnnotationRecord/4, D3-Annotation-Value/4 and R6 AnnotationInlineBody/1; Annotation read/draft has its own /1 family. Presentation policy uses a logical /1 view over immutable /2 records and SetRequest/2. D7 View remains ViewSpec/1; new Query authoring uses QuerySpec/2 while real QuerySpec/1 remains exact.

Outer number similarity never implies inner upgrade. Former D8 wire1, PreparedEditBinding/1-/2, historical effects/snapshots/Value3 recover only under their recorded decoders.

## 2. Source target and document response

D8SourceTarget/2 is {ref:EntityRef,expectedObservation:SourceObservation/1}. Current d8_document_read/2 contains workspaceRef, CommitDomain/2, ownerNodeRef and BudgetBinding/1. Current d8_document/2 returns exact SourceObservation/1, documentRevisionToken, domainFenceToken and D2DocumentSnapshot/3. Owner/Workspace/domain/observation/snapshot cross-fields agree, and a managed revision token authenticates to the observation's production SourceVersion.

## 3. Draft projection

DraftMapBinding/2 is {ownerNodeRef,baseObservation,draftSerial,source}. DraftEditMap/2 carries flows, plainRegions and sites. Flow paths follow current D2 schema only. Editable segments have a nonnull scalar source range; escape/join/atom editing is not granted by navigation. PlainRegion is an exact raw slice and may be empty/blank/EOL-only. DraftSite is a zero-width Core-proved sibling boundary. Navigation origins are a separate mapping family.

## 4. Draft writes

d8_draft_text_replace/2 explicitly binds flowPath, segmentIndex, start/end and replacementText. Replacement text contains no CR/LF and endpoints are Unicode18 grapheme boundaries.

DraftWriteCommand/2 is exactly splice_plain(regionIndex,start,end,text), break_plain(regionIndex,offset), or insert_at_site(siteIndex,text). These coordinates are proposal-local Counters only. CRLF is indivisible. Splice/insert text may contain complete CR/LF/CRLF.

## 5. D8EditIntent/3

D8EditIntent/3 is exactly document(target,source), annotation(target,expectedAnnotationRevisionToken,value:AnnotationEditableProposal/1,targetPolicy preserve|replace_current), or annotation_reconfirm_suggestion(target,expectedAnnotationRevisionToken). D8PinnedEditIntent/3 replaces source/value bytes with the appropriate PinRef/2.

D8EditInput/3 contains kind/version, invocationClass interactive_source_save|noninteractive, writeProtection strict|observed_only, pinned intent, and direct|undo origin.

D8EditPrepareRequest/3 is wireVersion3 and carries workspaceRef, CommitDomain/2, saveProfile ordinary|complete, guarantee replica_local|managed_atomic, writeProtection, intent, and BudgetBinding/1. Unknown arms/members/null reject. Annotation caller values contain no actor/time/trust/suggestion evidence; reconfirm carries no caller value.

## 6. PreparedEditBinding/3

PreparedEditBinding/3 contains exact workspace/domain/OperationId/principal, InputDescriptor/3, intent, direct|undo origin, InputDescriptor sourceInputs, one proposed entity pin, actually consumed Registry contexts, DependencyProof/3, PreparedIntent/3 observationProof, budget/deadline, original d6_commit_request/2, and EffectManifest/3 preview.

Document pin bytes are exact UTF-8 source. Annotation pin payloadKind is annotation_value and bytes are D3-CJ/3 of the complete Core-constructed Value/4. sourceInputs equal inputDescriptor.sourceInputs byte-for-byte. OwnerInputBinding ownerKind/intentKind is d8_edit/3 and canonicalDescriptorBytes is complete D8EditInput/3.

## 7. Annotation read/draft

D8AnnotationBodyRead/1 has absent(exactSource null, semanticText empty), valid(exactSource and semanticText), and invalid(exactSource, semanticText null, diagnostics) arms.

D8AnnotationReadResponse/1 returns Workspace/domain/AnnotationRef, SourceObservation, AnnotationRevisionToken, complete Value/4, targetResolution, and body read. D8AnnotationDraftProjection/1 contains base observation/token/serial, editable|readonly access, AnnotationEditableValue/1, targetResolution, and valid|invalid body source. Draft never carries full attribution authority and readonly never prepares.

## 8. Annotation owner references

Current D8 consumes only PortableAnnotationRecord/4, complete D3-Annotation-Value/4, AnnotationEditableValue/Proposal, R6 AnnotationInlineBody/1, current Suggestion/3, current five-arm target union, and current AnnotationAggregate rules. Value3/plain_text occurs only in genuine historical decoding.

## 9. Effects

EffectManifest/3 is the current closed preview/committed manifest. EffectItem/3 retains its existing arms and the typed presentation_policy_change arm. EffectBytes/3 uses only its existing closed encodings including exact_source_utf8 and d3_annotation_value4. D8 defines no free effect-byte encoding.

## 10. Presentation policy

Current types are D8WorkspacePresentationPolicy/1 logical view, immutable D8WorkspacePresentationPolicy/2 record, D8PresentationPolicyAddress/1, protected HeadSet/1, Proposal/1, BootstrapInit/1, SetRequest/2, Input/1, HeadEvidence/1, PrepareResult/2, OutboxItem/1, Effect/1, PresentationDecision/1, PresentationResult/1, and D8DocumentRenderBinding/1.

Record hash covers the fixed D8-Workspace-Presentation-Policy/2 prefix, NUL, and complete D3-CJ/3 record bytes. Parent/head arrays are sorted unique. The logical /1 value is derived from the unique current /2 record and is not separately persisted. Prepare does not allocate ChangeId or construct the committed record; final P does so atomically after all checks.

## 11. Device-only direction/layout state

DirectionPreference, CaretAffinity, LayoutEpoch, logical caret/selection, composition generation, hover/selection/legend hide/zoom/pan/fold are D8 UI/device state. They are not members of QuerySpec, ViewSpec, Document source, Annotation Value, or a hidden portable sidecar.

## 12. Consumed Search/View schemas

D8 Search UI produces D7 compiler input and consumes real QuerySpec/2/CanonicalGraph; there is no D8 Search wire. Saved Query remains D7 definition authority.

D8 renderers consume only closed D7 ViewSpec/1. D8 cannot append filter/sort/CEL/aggregate/script members. Formal View validation errors remain D7-owned.

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

The family is inherited and closed. A coordinated current producer/path uses its real owner unavailable/conflict result rather than `owner_update_required` as a generic implementation placeholder.

## 14. Fail-closed version examples

Current decoding rejects a /3 intent hidden inside old wire2 prepare, Value3 in current /3 prepare, PAB2 as PAB3, EffectManifest2 as current 3, a historical D2 snapshot as Snapshot3, query_json as QuerySpec, device direction/layout state as author data, and historical plain_text as current R6 Annotation. Genuine old records dispatch by recorded version before current qualification; unknown future versions fail closed.
