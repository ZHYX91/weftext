---
source_language: zh-CN
translation_of: D8-INTERFACES.zh-CN.md
translation_status: synced
---

[简体中文](D8-INTERFACES.zh-CN.md)
# A2 D8 Current Interfaces and State Machines

Status: author-resolved-pending-independent. This is the current D8 interface owner. It supersedes fixed-S/D6-FA former-current versions only where a named successor exists; genuine historical decoders remain exact.

## 1. Decode and ownership

All D8 JSON is strict UTF-8 single-object data. Duplicate/unknown/missing members, illegal null, unknown kind/version, and invalid structural numbers reject. D8 structural integers reuse D6 Counter. Embedded D2/D3/D4/D6/D7 values retain their own exact decoders.

D8 does not redefine EntityRef, Locator, SourceVersion, SourceObservation, Query, Field, Action, Effect, receipt, or PublicationReceipt. Once execution enters D3/D6/D7/D9, that owner returns its own formal error.

D8SourceTarget/2 is exactly {ref:EntityRef,expectedObservation:SourceObservation/1}. Ref, Workspace, observerDomain and request domain cross-fields must agree. Production domain/version and current observer domain/epoch remain separate. A supplied object is not authority; Core reconstructs its qualification.

## 2. Document read

Current request is wireVersion2 d8_document_read with workspaceRef, commitDomain, ownerNodeRef and budget. Success is wireVersion2 d8_document carrying the exact current SourceObservation/1, documentRevisionToken, domainFenceToken and D2DocumentSnapshot/3.

Order is closed decode → Workspace/entity disclosure → complete source_read plus source-envelope/current-format qualification → authority/cut → exact source plus D2 fixed-2.0.26 evaluation → final delivery barrier. Managed revision tokens must authenticate to exactly the production SourceVersion in the current observation. An authorized D2-invalid source may return only through the complete invalid Snapshot/3 branch; physical decode failure remains source_unavailable. A read never updates an existing Draft Base or grants write.

## 3. Draft projection

wireVersion2 d8_draft_project binds ownerNodeRef/baseObservation/draftSerial/source exactly. Valid projection returns complete current D2 projections plus editMap={flows,plainRegions,sites} and independent navigation origins; invalid projection returns exact source+diagnostics only.

BodyPath follows only the current D2 body schema. Equal displayed strings never identify occurrences. A flow is one Inline slot, a plainRegion is one maximal raw exact ordinary-text region under one parent, and a site is a Core-proved sibling boundary. UTF-8/UTF-16/scalar/grapheme/logical-EOL/visual coordinates are separate layers. Navigation mappings do not imply editability. Changing any map-binding member requires a fresh projection.

## 4. TextReplace and DraftWrite

wireVersion2 d8_draft_text_replace edits only one proven nonnull flow segment; endpoints are Unicode18 grapheme boundaries and replacementText has no CR/LF. Success d8_draft_text_replaced increments the serial and has no implicit caret.

wireVersion2 d8_draft_write has only splice_plain, break_plain, and insert_at_site. Success d8_draft_written returns a complete projection and a proven caret. Outside the selected raw range every byte is preserved; complete reparse must keep every non-target semantic/raw value; inserted text must remain ordinary inert text. Site insertion evaluates the nine 0/1/2 prefix/suffix EOL candidates and selects the minimum proven form. Existing and pasted EOLs are never normalized.

No-op source edits may advance Draft serial/caret but create no content Undo group. Old map/serial values never revive.

## 5. Async input and IME

Each controller has at most one Core transform in flight. UI state freezes owner, generation, inputSerial, draftSerial, before source/selection, native after/selection, composition state and successor input log. Results apply only to their own matching generation and cannot overwrite later native input.

IME begin/update is preedit, never preparable. Final composition is one input transaction and Draft Undo group. Structural commands and submit do not race composition. Cancel/blur/background cannot fabricate an author write. A single host owns composition.

## 6. Current edit prepare /3

Current D8EditIntent/3 has document, annotation, and annotation_reconfirm_suggestion arms. D8EditPrepareRequest/3 is wireVersion3 with workspaceRef, commitDomain, saveProfile ordinary|complete, guarantee replica_local|managed_atomic, writeProtection strict|observed_only, intent, and BudgetBinding/1.

Document pins exact UTF-8 source. Annotation caller input is AnnotationEditableProposal/1; Core alone constructs complete Value/4 after the operation-class gate. Caller actor/time/trust/lifecycle-evidence fields do not exist.

PreparedEditBinding/3 is the immutable current protected record with workspace/domain/operation/principal, InputDescriptor/3, complete intent and direct|undo origin, exact sourceInputs, one proposed pin, consumed Registry contexts, DependencyProof/3, PreparedIntent/3 observationProof, budget/deadline, original d6_commit_request/2, and EffectManifest/3 preview. OwnerInputBinding current ownerKind/intentKind is d8_edit/3 and encodes D8EditInput/3. Annotation EffectBytes/3 uses d3_annotation_value4.

ordinary/complete and strict/observed_only are independent axes. observed_only is limited to the trusted interactive single-existing-Document full-source path. Annotation, Undo, structured/bulk/automation/managed_atomic paths remain strict.

Prepare is not Saved. Submission remains the original D6 request and revalidates the frozen descriptor/proofs/dependencies/authorization/install state.

## 7. Annotation read and Draft

D8AnnotationReadRequest/1 and D8AnnotationDraftOpenRequest/1 are real Core entries. Read returns complete Value/4, SourceObservation, AnnotationRevisionToken, targetResolution and D8AnnotationBodyRead/1. absent body has semanticText empty; valid body has exact source+semantic text; invalid body retains exact source+diagnostics with semanticText null.

Draft contains only AnnotationEditableValue/1. With annotation_read but not annotation_write it is readonly and cannot enter prepare/3. ordinary_edit/manual_reattach/reconfirm_suggestion are mechanically selected operation classes, not caller trust flags.

## 8. Preview, confirmation, recovery

D8 confirmation consumes the current complete D7/D6 preview transport: EffectManifest/3, EffectBytes/3, preview tokens/cursors/epoch and every effect page. A manifest summary or one page is insufficient. TTL/auth/epoch/reset invalidates submission.

Saved/planned/unknown recovery dispatches the original record version first and restores the original request, bytes, pins, OperationId and responsibilities. A new ID never retries an unknown old outcome.

## 9. Committed Undo

d8_undo_prepare prepares a fresh inverse edit, never ledger rollback. It is allowed only when the exact original request/receipt/effects prove one source change and no other control/identity/entity effect, complete pins remain, current production version+bytes equal the original after, and current qualification is fresh. Success uses the ordinary current prepared family with a new OperationId/plan. Redo is another fresh operation.

## 10. Presentation policy

D8 presentation policy is a D8-owned complex P-only configuration, not D6 Policy/3 or a PortableComponentKey. The current logical /1 view is derived from one immutable /2 head record. Current types include the protected head set, SetRequest/2, PrepareResult/2, presentation_policy_change effect, outbox item and D8DocumentRenderBinding/1.

Set order is decode → disclosure → policy_admin → frontier/domain → protected heads → exact expectedHeads → retained parent evidence/ancestry → semantics/budget. One equal current head is no-change. [] is only a proved uninitialized owner state; multi-head conflict is never LWW.

Prepare creates no ChangeId/current record. Final P alone allocates ChangeId and atomically creates the canonical record/hash/pin/effect/receipt/ChangeRecord/outbox/head transition. Concurrent offline revision-2 records can coexist as heads; arrival/revision/hash cannot select a winner. Resolution explicitly parents the complete head set.

## 11. Errors and disclosure

The inherited D8 editor error family remains closed:

`invalid_request|not_visible|authority_unavailable|domain_unavailable|integrity_conflict|source_unavailable|proof_unavailable|owner_update_required|stale_target|ambiguous_selection|unrepresentable_text|semantic_rejected|budget_exceeded`.

Current coordinated paths must return their actual unavailable/conflict result when the owner contract provides one; `owner_update_required` is not a generic implementation placeholder. Sensitive existence/version/Locator details are observable only after their applicable disclosure gates. Once execution enters D3/D6/D7/D9, that owner returns its original error without a D8 wrapper.

## 12. Surface parity

Desktop/CLI/Mobile local Core and Server Core invoke these same semantic interfaces; WebUI calls Server. Missing renderer/IME/table-editor capability is reported as a real capability difference rather than changing request meaning. A remote/offline hosted source does not gain local author-commit authority merely because a local parser exists.
