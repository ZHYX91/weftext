---
source_language: zh-CN
translation_of: D8-INTERFACES.zh-CN.md
translation_status: synced
---

[简体中文](D8-INTERFACES.zh-CN.md)

# A2 D8 Current Interfaces and State Machines

Status: author-resolved-pending-independent. This file is the current D8 interface owner. It supersedes former-current fixed-S / D6-FA versions only where a named successor exists; genuine historical decoders remain exact.

## 1. Common decode and owner boundary

Every D8 JSON value is one strict UTF-8 object. Duplicate, unknown, or missing members; illegal null; and unknown kind/version reject. D8 structural integers reuse the D6 Counter decoder and reject Boolean, fraction, exponent, -0, and overflow. Embedded D2/D3/D4/D6/D7 values retain their owner decoders; D8 does not rewrite their null, number, or version rules.

D8 does not define a new EntityRef, Locator, SourceVersion, SourceObservation, Query, Field, Action, Effect, receipt, or PublicationReceipt version. A D8 public error cannot wrap or rename a D3/D6/D7/D9 error after that owner has been entered.

D8SourceTarget/2 is exactly ref plus expectedObservation. ref must equal expectedObservation.entityRef. Workspace and observerDomain must match the request workspaceRef/commitDomain. Production domain/version and the current observer domain/epoch remain distinct. This object is not a permission ticket; Core reconstructs and validates the complete protected evidence.

## 2. Document read

The current read request is:

```text
{wireVersion:2,kind:"d8_document_read",
 workspaceRef,commitDomain,ownerNodeRef,budget}
```

Success is:

```text
{wireVersion:2,kind:"d8_document",
 workspaceRef,commitDomain,ownerNodeRef,
 sourceObservation:SourceObservation/1,
 documentRevisionToken,
 domainFenceToken,
 snapshot:D2DocumentSnapshot/3}
```

Order is closed decode -> Workspace/entity disclosure -> complete source_read plus source-envelope/current-format qualification -> authority/cut -> exact source plus fixed Asciidoctor 2.0.26 D2 evaluation -> final delivery barrier.

For managed source, the revision token is valid only after the real RevisionTokenSeal/Binding proof resolves to exactly sourceObservation.sourceVersion. External source retains its original external-event/version proof. An authorized D2-invalid source can return only through the complete invalid Snapshot/3 branch; physical decoding failure remains source_unavailable. A successful read does not update an existing Draft Base, re-sign a Locator, or grant write authority.

## 3. Draft projection

```text
d8_draft_project/2 = {
 wireVersion:2,kind:"d8_draft_project",
 workspaceRef,commitDomain,ownerNodeRef,
 expectedSourceObservation,draftSerial,source,budget
}
```

A valid success retains the exact source, baseObservation, serial, current D2 metadata/attribute carriers/body, editMap={flows,plainRegions,sites}, and navigation origins={elements,flows}. An invalid success returns only exact source plus diagnostics and never leaks partial metadata, body, map, or origins.

mapBinding={ownerNodeRef,baseObservation,draftSerial,source} is exact in all four members. Changing any member cannot relabel an old map.

BodyPath walks only structural members/indexes in the current D2 body schema; same-named keys inside user payload do not participate. There is one flow per Inline slot. A plainRegion is one maximal exact raw ordinary-text region under one parent. A site is a parser-proved sibling boundary. Navigation origin is independent from editability.

## 4. TextReplace and DraftWrite

d8_draft_text_replace/2 operates on one nonnull flow segment. The request explicitly carries flowPath, segmentIndex, start/end, and replacementText. start/end must be Unicode 18 extended-grapheme boundaries in the complete flow. replacementText contains no CR/LF. Success returns d8_draft_text_replaced with projection serial=inputSerial+1 and has no implicit caret member.

d8_draft_write/2 has only this command union:

```text
{kind:"splice_plain",regionIndex,start,end,text}
{kind:"break_plain",regionIndex,offset}
{kind:"insert_at_site",siteIndex,text}
```

Success is d8_draft_written plus a complete projection and proved caret. A region splice preserves every byte outside the selected range. insert_at_site evaluates the nine 0/1/2 by 0/1/2 EOL candidates with a full reparse, then selects the minimum number of generated EOLs and the minimum prefix. The EOL policy applies only to generated characters; existing and pasted EOLs remain exact.

A Source no-op may return a new serial/caret but creates no content Undo group. The serial never wraps. Undo never revives an old map or serial.

## 5. Async input protocol

Each editor controller has at most one transform in flight. The host records:

```text
InputTransaction {
 owner, generation, inputSerial, draftSerial,
 beforeSource, beforeSelection,
 nativeAfter, nativeSelection,
 compositionState, successorLog
}
```

This is UI state, not portable wire. A result is accepted only while owner, Base, source, inputSerial, and generation all still match that transaction. A late result cannot overwrite successor native input.

IME begin/update is preedit. commit/end/final input forms the Draft transaction. Active composition forbids prepare, submit, and structural commands. cancel/blur/background never creates a source write. The final composition is one Undo group.

## 6. Current Edit prepare /3

The current request uses the closed intent and prepare request below:

```text
D8EditIntent/3 =
  {kind:"document",target:D8SourceTarget/2,source:text}
| {kind:"annotation",target:D8SourceTarget/2,
   expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
   value:AnnotationEditableProposal/1,
   targetPolicy:"preserve"|"replace_current"}
| {kind:"annotation_reconfirm_suggestion",target:D8SourceTarget/2,
   expectedAnnotationRevisionToken:AnnotationRevisionToken/1}

D8EditPrepareRequest/3 = {
 wireVersion:3,kind:"d8_edit_prepare",
 workspaceRef,commitDomain,
 saveProfile:"ordinary"|"complete",
 guarantee:"replica_local"|"managed_atomic",
 writeProtection:"strict"|"observed_only",
 intent:D8EditIntent/3,budget:BudgetBinding/1
}
```

For document intent, the proposed pin selects exact UTF-8 source. An Annotation caller supplies only AnnotationEditableProposal/1. After the operation-class gate, Core constructs the complete Value/4. The caller cannot supply actor/time/trusted/state/confirmation/basis/expectedText/pointAffinity.

PreparedEditBinding/3 has these exact current semantic members:

```text
{kind:"d8_prepared_edit_binding",version:3,
 workspaceRef,commitDomain,operationId,principalAudienceToken,
 inputDescriptor:InputDescriptor/3,
 intent:D8EditIntent/3,
 origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request},
 sourceInputs:InputDescriptor/3.sourceInputs,
 proposedInputs:[{entityRef,pin:PinRef/2}],
 registryInputs:[ValidatedCatalogContext...],
 dependencyProof:DependencyProof/3,
 observationProof:PreparedIntent/3.observationProof,
 budgetBinding,expiresAt,
 request:d6_commit_request/2,
 preview:EffectManifest/3}
```

For the current OwnerInputBinding, ownerKind=intentKind=d8_edit/3 and canonicalDescriptorBytes is D3-CJ/3(D8EditInput/3). Current effect bytes use EffectBytes/3. The Annotation pin encoding is d3_annotation_value4.

ordinary/complete and strict/observed_only are independent axes. observed_only is limited to the qualified trusted interactive whole-source save of one existing live Document. Annotation, Undo, structured, bulk, automation, and managed_atomic paths cannot use it.

Prepare success proves only that an immutable plan and preview exist. It is not Saved. Submission remains the original D6 request; planning and final submission revalidate descriptor, proof, dependencies, authorization, and installation.

## 7. Annotation read / Draft

```text
D8AnnotationReadRequest/1 = {
 wireVersion:1,kind:"d8_annotation_read",
 workspaceRef,commitDomain,annotationRef
}

D8AnnotationReadResponse/1 = {
 kind:"d8_annotation_read",version:1,
 workspaceRef,commitDomain,annotationRef,
 sourceObservation,
 annotationRevisionToken,
 value:D3-Annotation-Value/4,
 targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
 body:D8AnnotationBodyRead/1
}

D8AnnotationDraftOpenRequest/1 = {
 wireVersion:1,kind:"d8_annotation_draft_open",
 workspaceRef,commitDomain,annotationRef
}
```

For the body, absent means exactSource=null and semanticText=""; valid means exact source plus semantic text; invalid means exact source plus diagnostics and semanticText=null. Read returns the complete Value/4 including attribution. Draft retains only editable fields. With no annotation_write, access is readonly; readonly cannot enter prepare/3.

ordinary_edit, manual_reattach, and reconfirm_suggestion are derived mechanically from the intent arm and targetPolicy. They are not caller trust flags.

## 8. Preview / confirmation / recovery

The D8 confirmation UI consumes the current D7/D6 full preview transport: EffectManifest/3, EffectBytes/3, preview token/cursor/epoch, and every effect page. Before confirmation, the UI must prove that all required pages/bytes are available and belong to the same plan/epoch. A manifest summary, current page, or old cached bytes is not enough.

After TTL, authorization, epoch, or reset invalidates the preview, the old preview cannot submit. planned/saved/unknown recovery first restores the original request, decoder, pins, and OperationId, then continues the original responsibility. A fresh prepare never pretends to resolve an unknown old decision.

## 9. Committed Undo prepare

d8_undo_prepare prepares a new operation; it is not a ledger rollback. It restores the original request/receipt/effect before/after and proves:

- the original decision has exactly one source change;
- there is no other control/identity/entity effect;
- the current production version and bytes exactly equal the original after;
- current authorization, format, Observation, and installation qualification are fresh;
- complete history pins remain available.

Success returns the ordinary current d8_edit_prepared family with a new OperationId, planToken, and preview. Redo is likewise a fresh operation.

## 10. Presentation policy

The current shared presentation policy is a D8-owned complex P-only configuration. It is not D6 Policy/3 and not a PortableComponentKey.

The principal current types are:

- D8WorkspacePresentationPolicy/1: logical current view of the unique head;
- immutable D8WorkspacePresentationPolicy/2: parents, revision, defaultPresentation, activationChangeId;
- protected D8PresentationPolicyHeadSet/1;
- D8PresentationPolicySetRequest/2;
- D8PresentationPolicyPrepareResult/2;
- D8PresentationPolicyEffect/1;
- D8DocumentRenderBinding/1.

SetRequest order is decode -> presentation-state disclosure -> policy_admin -> expectedFrontier/domain -> protected head set -> exact expectedHeads -> parent record/pin/ancestry -> semantics/budget. One current head with the requested value is no-change. [] is legal only when uninitialized owner state is proved. A multi-head conflict is never resolved by equal values or LWW.

Prepare does not create a ChangeId or current record. Only final P, after all checks, allocates one ChangeId, constructs/hashes/pins the record, and commits effect/receipt/ChangeRecord/outbox/head transition. Offline concurrent writers can create two revision=2 heads. revision, arrival order, and hash do not select a winner. Explicit resolution names the complete current head set as parents.

## 11. Errors and disclosure order

The inherited D8 editor error family remains closed. Its first group is `invalid_request|not_visible|authority_unavailable|domain_unavailable|integrity_conflict|source_unavailable`. Its second group is `proof_unavailable|owner_update_required|stale_target|ambiguous_selection|unrepresentable_text|semantic_rejected|budget_exceeded`.

When a current coordinated path has a real owner-specific unavailable/conflict result, it must return that result; owner_update_required is not a generic implementation placeholder. Static decode comes first. Sensitive existence, version, Locator, and detail become observable only after the applicable disclosure gate. After D3/D6/D7/D9 is entered, D8 returns that owner's original error without renaming it.

## 12. Surface parity

Desktop/CLI/Mobile local Core and Server Core call the same interfaces; WebUI calls Server. If a surface lacks a renderer, IME adapter, or table editor, it reports the real capability as unavailable rather than changing request meaning. Remote/offline hosted author source does not gain local commit authority merely because a local parser exists.
