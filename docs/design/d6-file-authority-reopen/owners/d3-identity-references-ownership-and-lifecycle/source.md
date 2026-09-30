---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 1bde8ca5-50fd-41ec-a146-b31d951e46c2.

# D3 Identity, References, Ownership, and Lifecycle

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Fixed-S D3 wire11/Result9 and saved v9/v10 decisions remain historical compatibility sources. This afterimage defines D3 wire12 for new decisions and may activate only with D6-FA-r01 plus the later D4/D5/D7/D8/D9/D10 owner afterimages under fresh joint review. It authorizes no product implementation, release, or A2.

## 1. Scope, frozen inputs, and non-goals

D3 remains the sole owner of:

- identity algebra for WorkspaceRef, NodeRef, ResourceRef, and AnnotationRef;
- logical Node parent/order semantics;
- live, trashed, and tombstoned lifecycle for Node/Resource/Annotation;
- Locator, anchor, reference lifecycle, and copy/fork/continue/import identity rules;
- D3 operation request, receipt, resolver outcome, and fail-closed families.

D6 solely owns CommitDomain/2, ReplicaEpoch, ChangeId/1, Frontier/1, SourceVersion/2, InputDescriptor/2, PreparedIntent/2, file installation, P seal, ContentCompletionProof, ConflictRecord, and execution responsibility. D3 consumes those exact types and never redefines them or creates a second writable truth.

D2 exact source, D4 typed schema/relations, D5 structure, D7 Query/Action, D8 editor, D9 conversion, and D10 external execution remain with their owners.

## 2. Design goals

1. Physical path, index row, mtime, digest, and sync-provider identifier are not content identity.
2. Multiple registered replicas of one logical Workspace preserve the same durable refs while allowing ordinary offline content operations independently.
3. Ordinary replica content qualification is separate from global execution responsibility.
4. Parent/order, lifecycle, tombstones, and no-reuse facts are portable and never exist only in discardable I.
5. replica_local success proves a reliable decision in one commit domain, not a single global current state or a complete D7 set.
6. managed_atomic retains complete-closure, complete-range, and strong recovery semantics for operations that need them.
7. External changes, placeholders, partial transport, and ABA never regain old continuity merely because hashes match.
8. Saved-decision replay and current-source state are always separate.

## 3. Identity algebra and terminology domains

WorkspaceId, AuthorityInstanceId, NodeId, ResourceId, and AnnotationId remain canonical lowercase RFC 4122 UUIDv4. D3Integer remains the mathematical domain 0..2^63-1; bool, floating point, exponent form, negative values, and wrap are forbidden.

NodeRef is WorkspaceId + NodeId. ResourceRef/AnnotationRef encode owner NodeRef plus an owner-local ID. Equality always compares the complete typed ref; bare UUID, path, title, digest, or provider key never participates.

Document has no separate durable ID and is addressed by its owning NodeRef. Document elements, headings, paragraphs, table rows/cells, Field occurrences, and Saved Query/View occurrences are not a fourth entity class.

ForeignIdentityKey, SourceBinding, OriginBinding, Provenance, LogicalOccurrenceKey, ResultRowHandle, ActionEvidence, OperationId, and AuditEventId never coerce to content refs.

## 4. wire12, commit domains, and idempotency ledger

### 4.1 D6 type consumption

wire12 embeds CommitDomain/2, Frontier/1, and InputDescriptor/2 exactly and invokes the D6 decoders. Their canonical bytes follow D6 ownership; D3 accepts no private look-alike object.

A replica CommitDomain requires the replicaEpoch in a current active ReplicaRecord. A server CommitDomain requires the authorityInstanceId of the current hosted authority. D6 proves domain/backend/fence continuity before D3 reads its identity ledger.

### 4.2 OperationId ledger key

The only new-wire ledger key is:

    (boundWorkspaceRef.workspaceId, D3-CJ/3(commitDomain), operationId)

protocolOwner=D3 is stored in the shared D6 v2 control ledger. The same Workspace and OperationId in different CommitDomain values are independent keys. One key has exactly one request fingerprint/decision.

The state machine remains unseen -> rejected or unseen -> planned -> committed | terminal_failed. rejected/committed/terminal_failed bytes are immutable and planned can only recover its original plan.

A different fingerprint on the same key returns operation_id_conflict without revealing the original request.

### 4.3 wire12 identity operation request

New decisions accept only this closed top-level object:

~~~json
{
  "wireVersion":12,
  "kind":"identity_operation_request",
  "operationId":"uuid-v4",
  "boundWorkspaceRef":<WorkspaceRef>,
  "commitDomain":<D6 CommitDomain/2>,
  "guarantee":"replica_local|managed_atomic",
  "expectedFrontier":<D6 Frontier/1>,
  "inputDescriptor":<D6 InputDescriptor/2>,
  "mode":<D3Mode>,
  "expectedAuthority":<ExpectedAuthority?>,
  "workspaceProposal":<WorkspaceAllocationProposal?>,
  "preparationBinding":<PreparationBinding?>,
  "intent":<D3Intent>
}
~~~

Question-mark members appear only where the mode matrix allows; otherwise they are absent, never null. Unknown members fail closed.

inputDescriptor.workspaceRef, commitDomain, expectedFrontier, and guarantee must be canonical-equal to the top-level values. intentKind identifies D3 identity operation v12. ownerInput canonical descriptors are produced by D3 v12, and large Document/Resource/Annotation bytes appear only through typed D6 PinRef slots. The canonical request never permanently embeds full source bodies or attachments.

Exact input equality compares canonical InputDescriptor, owner descriptor, and every referenced exact pin. Equal sha256 values alone are not equal input and do not establish ABA continuity.

### 4.4 requestFingerprint

D3-CJ/3 and SHA-256 rules are unchanged. The fingerprint body includes every canonical request field except top-level operationId; workspaceProposal still enters through its complete canonical digest.

Any change to commitDomain, guarantee, expectedFrontier, InputDescriptor, authority expectation, proposal, preparationBinding, intent, or plan changes the fingerprint.

### 4.5 guarantee and mode matrix

| mode | replica_local | managed_atomic |
|---|---|---|
| create_node | allowed | allowed |
| move_node | allowed | allowed |
| reorder_node | allowed | allowed |
| trash | allowed | allowed |
| create_workspace | forbidden | allowed |
| create_resource/create_annotation | forbidden | allowed |
| copy_node_subtree/copy_resource/copy_annotation | forbidden | allowed |
| fork_workspace/continue_workspace | forbidden | allowed |
| restore/purge | forbidden | allowed |
| import_new | forbidden | allowed |

replica_local forbids workspaceProposal, expectedAuthority, and preparationBinding. Its commit-domain fence comes from D6 active replica, portable registry/policy, and file-backend qualification.

managed_atomic retains wire11 expectedAuthority modes: create/fork use create; continue uses continue; all other modes use existing. create/fork keep proposal P1/P2 and TargetLedgerCustody, and a server CommitDomain must match the current expected authority instance.

D7-mediated wire12 preparation awaits the D7 owner afterimage. The outer preparationBinding token wrapper remains, but this partial candidate does not allow it to create a wire12 managed success. The D6 preparation capability returns owner_update_required with zero D3 ledger decision. D3-native typed operations may omit preparationBinding.

### 4.6 allocation and UUID no-reuse

Within one CommitDomain, Core reserves/burns fresh IDs in P allocation history. Portable metadata records accepted births, tombstones, and no-reuse facts.

If two offline replicas independently mint the same complete typed ref, the receiver never chooses by sync order or hash. It creates identity_collision and no combined canonical entity exists until explicit resolution. Resolution may retain one birth claim and fresh-copy/import the losing branch content under a new ID. It never silently rekeys that branch.

An accepted tombstone ID is never reused. A burned reservation is likewise unavailable for future allocation in its commit domain.

### 4.7 SourceVersion/2 and ABA

D3 Locator, selector, and prepared-evidence currentness binds D6 SourceVersion/2. A bare revision Counter has no cross-CommitDomain meaning.

An observationEpoch change invalidates old locator/preparation/action evidence. A->B->A cannot regain old continuity even if final bytes/digest match. Higher-level sourceOccurrenceKey continuity remains a D10 concern and is never inferred from path/hash by D3.

### 4.8 legacy v9/v10/v11

Saved v9/v10/v11 requests, proposals, plans, receipts, errors, and PreparedActionBinding/1,/2 continue under their original decoder, fingerprint, authorization, custody, pin-retention, and byte-equivalent replay rules.

Forbidden:

- re-encoding an old request as wire12;
- adding CommitDomain/ChangeId to an old receipt;
- deleting legacy evidence using new retention;
- interpreting old complete gates through semantic_pending;
- restoring an old version qualification because current files hash like an old pin.

New decisions do not accept legacy wire. Legacy saved decisions do not accept new semantic fields.

## 5. Workspace, Node, owner, and ordered structure

### 5.1 Workspace and replica

WorkspaceRef remains the identity namespace; replicaEpoch is not Workspace identity. Replica registration preserves WorkspaceId and existing Node/Resource/Annotation refs while adding a new D6 CommitDomain.

Registration does not call continue_workspace, allocate a new WorkspaceId/AuthorityInstanceId, take over global execution responsibility, or assert that an old device stopped.

### 5.2 Node parent/order

The live Node tree remains a connected ordered tree from root. Root parent/ordinal are null; every non-root live Node has exactly one live parent. siblingOrdinal is unique and contiguous from zero within a parent.

D3 remains logical owner of parent/order while D6 portable metadata carries it physically. Path, directory tree, index order, file enumeration, and mtime never become a second owner.

move preserves NodeRef. title/path/label changes do not change child identity. Deleting a parent while retaining children requires an explicit move before deletion and never creates a live orphan.

### 5.3 Resource and Annotation owner

Resource/Annotation owner remains immutable. Cross-owner semantics always use fresh-copy. Owner Trash/purge closure, reply acyclicity, and Annotation owner-local target rules remain.

Same-owner Resource byte replacement and Annotation body/target edits preserve refs but produce new SourceVersion/2.

### 5.4 structural conflicts

Concurrent placement heads are never repaired automatically by UUID, path, ordinal, or mtime. D6 ConflictRecord kind placement_concurrent identifies the complete heads, and ordinary resolver does not choose a hidden winner.

A conflict branch can still be opened explicitly and edited, but an unqualified structure read cannot claim one canonical tree.

## 6. Locator, anchor, exact source, and external bytes

DocumentElementLocator, DocumentRangeLocator, ResourceRegionLocator, and l1 retain their wire11 lexical, owner, revision, span/range, base64url, and D3-CJ/3 rules. AuthorAnchorAddress remains a current-revision symbolic address, not an entity.

wire12 consumers additionally bind SourceVersion/2 and CommitDomain. Same text/span/token payload never restores currentness across observationEpoch.

Only D2-valid managed source can participate in an ordinary D3 author mutation. External physical-invalid or D2-invalid bytes remain actual file bytes available through D6 current-source/repair interfaces; they never become SemanticState, never produce a successful D3 author receipt, and are never overwritten by an old pin.

## 7. lifecycle: ordinary Trash and strong restore/purge

### 7.1 shared state machine

Node, Resource, and Annotation keep unallocated -> live -> trashed -> live|tombstoned. tombstoned is terminal and Document follows Node.

Trash keeps typed refs, payload, and recovery information. purge removes payload and publishes minimal tombstones that cannot be restored through Weftext.

### 7.2 replica_local Trash

replica_local Trash completely proves:

- target and actual subtree/owner-local/reply membership;
- contiguous unique ordinals in affected live/Trash sibling lists;
- current lifecycle-write permission and policy for that closure;
- safe installation for modified portable metadata components.

It does not require an offline ordinary delete to scan every other replica and every inbound reference in the workspace. Unproved inbound/cross-object obligations appear in D6 SemanticState.semantic_pending, including inbound where applicable. The receipt never claims its local referenceLifecycleTransitions are a workspace-global enumeration.

When a remote source head later arrives, delete/edit or lifecycle concurrency produces an explicit conflict rather than implicit delete-wins or edit-wins.

### 7.3 managed_atomic Trash

managed_atomic Trash may produce complete_semantics only after the complete positive/negative reference ranges are proved, retaining the complete suspended/reference-lifecycle evidence of the strong wire11 path.

### 7.4 restore

restore is managed_atomic only. It preserves identity and consumes the exact Trash graph, restore location, owner/reply closure, current references, and all required dependencies.

If restore conflicts with another replica's edit/move/delete head, ConflictRecord is resolved first. Path guessing and silent identity replacement are forbidden.

### 7.5 purge and tombstones

purge is managed_atomic only and requires:

1. target is already trashed;
2. complete owner/reply/subtree closure;
3. all applicable inbound refs and D4/D5 strong constraints;
4. a fixed portable replica-registry revision;
5. every active registered replica's accepted Frontier satisfies the purge coverage requirement;
6. a retired replica no longer counts as active acknowledgement and its epoch can never reactivate;
7. tombstone/no-reuse facts and ContentCompletionProof publish together.

A device never registered and not provably part of the Workspace does not block purge forever. If old directory bytes appear later, they register a new ReplicaEpoch and reconcile against current tombstones/ConflictRecord; they cannot resurrect the tombstoned ref.

A sync provider's "complete" flag is not proof of no inbound reference, no offline head, or purge coverage.

## 8. Create, Move, Copy, Fork, Continue, and Import

### 8.1 create_node

replica_local create_node requires:

- active replica CommitDomain;
- current parent/ancestor chain and complete destination sibling list;
- fresh NodeId reservation;
- D2-valid source;
- local typed facts in the source validated against current Registry;
- safe installation of portable birth, placement, and source components.

Unproved relation/unique/calendar/collection/inbound/cross-object obligations become semantic_pending and cannot later masquerade as complete D4/D5 success.

managed_atomic create_node may additionally require complete_semantics and complete D7/D4/D5 preparation.

### 8.2 move/reorder

replica_local move/reorder preserves NodeRef and requires old/new parent, complete affected sibling lists, ancestor chain needed for cycle proof, and current placement/policy qualification. destinationOrdinal remains the final post-commit index and input array order has no semantic effect.

Concurrent move/move or move versus parent deletion creates placement/lifecycle conflict; Core does not attach to root or choose a last writer.

### 8.3 copy and owner-local fresh identity

copy_node_subtree, copy_resource, and copy_annotation are managed_atomic only. wire11 identityMap, owner rewrite, reference-slot, Annotation target/reply, omission-closure, and fresh-identity semantics remain.

Equal bytes, title, or digest never establish identity equality. Cross-owner Resource/Annotation always receives fresh identity.

### 8.4 Workspace fork and continue

fork/continue are managed_atomic only.

continue is restricted to exclusive continuation/failover/disaster recovery of one authority lineage with complete control ledger, allocation/burn/tombstone history, saved decisions, and fencing of the old authority. Replica registration never uses continue.

fork creates fresh WorkspaceId/AuthorityInstanceId and all mapped content IDs. Source and target never share commit authority.

### 8.5 import/export re-entry

Formal-backup continue/fork, identity-bearing bundle, ordinary-format import, and current-Trash restore remain explicitly distinguished by artifact class, never inferred from path, digest, root UUID, or filename.

Ordinary format/worker IR never preserves source content identity. Partial identity bundles never query online source Workspace authority.

## 9. replica registration, new devices, lost I, and lost P

### 9.1 new-device admission

After ordinary files plus complete portable metadata arrive on a new device:

1. validate WorkspaceRef, portable trust, replica registry, birth/tombstone/structure records, and actual files;
2. use D6 replica registration to mint a new ReplicaEpoch;
3. grant that new CommitDomain ordinary content qualification only while preserving existing refs;
4. do not import another device's active P/WAL/SHM;
5. do not inherit Automation/ApprovalUse/Money/external-unknown consumption authority.

### 9.2 lost I

I is fully discardable. Core progressively rebuilds inventory/parser/search/OCR from F/M without reminting identity, parent/order, lifecycle, receipts, or conflicts.

### 9.3 lost P

When a replica loses P, its OperationId decisions, planned installation, external unknowns, and approval/Money state cannot be reconstructed from current files.

Ledger continuity for that replicaEpoch is unproved, so no new decision is issued under the old CommitDomain. Authorized repair first checks portable InstallationNotice/ContentCompletionProof and actual components; unresolved ranges become recovery/conflict state.

After safe reconciliation, the old ReplicaEpoch can be retired and a new epoch registered for ordinary content. Global execution responsibility remains unavailable until a dedicated custody takeover proves the old holder fenced; new replica registration never resets quota.

## 10. Synchronization, concurrent versions, and conflicts

### 10.1 D6 ConflictRecord is the only portable conflict record

D3 creates no second conflict database. D6 ConflictKey/ConflictId/ConflictRecord identifies source_concurrent, placement_concurrent, lifecycle_concurrent, identity_collision, policy_concurrent, incomplete_transport, and placeholder.

D3 owns typed placement/lifecycle/identity resolution semantics. D6 conflict_prepare owner_resolution only routes to those semantics and never carries a free JSON patch.

### 10.2 two edits to one Document

Both concurrent SourceVersion/2 heads are retained. A user may explicitly choose one head or provide a complete merged source; the three-way Base must be proven by causal predecessors rather than content similarity.

A merge creates a new ChangeId and preserves the old heads as history instead of overwriting their bytes.

### 10.3 create/move plus remote edit

When the same birth record and causal chain are proven, a source edit and independent placement change may be composed after policy/structure revalidation. Missing birth or portable metadata is incomplete state and never causes fresh identity allocation.

### 10.4 delete/edit

Trash plus concurrent edit yields lifecycle_concurrent. Both payload and lifecycle heads remain. Explicit resolution chooses Trash, restored live state, or fresh-copy content; there is no default delete-wins or edit-wins policy.

### 10.5 partial metadata and placeholder

Source before placement/identity, or metadata before source, is incomplete_transport. On-demand bytes not materialized are placeholder/not_materialized, not empty source, not_found, or deletion.

### 10.6 duplicate identity

Two incompatible birth claims for one typed ref are identity_collision. Ordinary resolver selects no winner. Resolution retains one canonical claim and explicitly fresh-copies/imports the losing branch content; it never silently changes an ID and calls it the same history.

### 10.7 ABA and observation gaps

Watcher gaps, external replacement, placeholder materialization, and discontinuous sync advance observationEpoch. Equal digest never proves original-install provenance, SourceVersion continuity, or current old locators.

## 11. Resolver and visibility

### 11.1 wire12 resolver context

Every new resolver request explicitly binds WorkspaceRef, CommitDomain, and expected Frontier. D6 performs state-disclosure and domain qualification before D3 resolves a typed ref/locator.

### 11.2 Entity outcome

wire12 Entity outcome retains resolved, trashed, tombstoned, not_found, not_visible, workspace_unavailable, and invalid, and adds three non-lifecycle states:

- conflicted: the typed ref has unresolved identity/placement/lifecycle/source conflict and no canonical branch may be selected;
- incomplete: portable components/records are not complete;
- placeholder: identity/resource is known but current bytes are not materialized.

These outcomes expose no hidden bytes from another branch. Reading ConflictRecord or source remains a D6-authorized interface.

### 11.3 Locator outcome

Locator first propagates owner/entity conflicted/incomplete/placeholder/trashed/tombstoned/not_found/not_visible/workspace_unavailable. Only a canonical live owner with locator-disclosure qualification proceeds to resolved/stale/anchor ambiguity.

stale never fuzzy-reanchors. D8 may request a proposal, but Core signs a new locator against the new SourceVersion.

### 11.4 non-disclosure order

The order is closed decode -> workspace/domain binding -> current state disclosure -> domain/backend continuity -> conflict/incomplete state -> exact identity lifecycle -> locator disclosure -> revision/coordinate.

Without authorization, conflict counts, placeholder, tombstone, and stale state never become existence oracles.

## 12. wire12 receipt, D6 seal, and historical result

### 12.1 identity_change_receipt v12

D3 receipt retains the twelve wire11 effect arrays and per-mode exact semantics while adding scoped commit binding:

~~~json
{
  "wireVersion":12,
  "kind":"identity_change_receipt",
  "operationId":"uuid-v4",
  "targetWorkspaceRef":<WorkspaceRef>,
  "commitDomain":<D6 CommitDomain/2>,
  "changeId":<D6 ChangeId/1>,
  "guarantee":"replica_local|managed_atomic",
  "semanticState":<D6 SemanticState/1>,
  "mode":<D3Mode>,
  "identityMap":[],
  "allocated":[],
  "resultAllocations":[],
  "resultLifecycles":[],
  "initialPlacements":[],
  "trashPlacements":[],
  "preserved":[],
  "tombstoned":[],
  "rewrittenReferences":[],
  "referenceLifecycleTransitions":[],
  "structuralChanges":[],
  "omittedObjects":[],
  "result":"committed"
}
~~~

sourceWorkspaceRef, destinationOwnerRef, issuer/target authority, continuation, artifactClass, and other mode-specific members retain the exact wire11 matrix.

changeId.commitDomain equals receipt.commitDomain and binds the same decision as the D6 seal-time receipt. D3 does not duplicate reliableSaveState/portablePublicationState: D6 receipt is immutable reliable + publication pending, while current D6 decision state later reports publication completion without modifying either receipt.

A replica_local receipt has complete arrays for the local scope actually proved by this request. When SemanticState=semantic_pending, it explicitly does not claim that the named obligation has been enumerated across the whole Workspace. Only managed_atomic + complete_semantics can satisfy legacy complete-set consumers.

### 12.2 r5 replay versus current r6

Replaying an original committed r5 request returns the original r5 D3/D6 receipt bytes. It never rewrites current r6 SourceVersion, Frontier, or publication state into the old receipt.

Current r6 is read through D6 current-source/portable-state interfaces. Historical decision and current state remain separate.

### 12.3 installation and P seal

D3 declares the planned identity/structure/lifecycle poststate; D6 performs file installation through FileObjectBinding/InstallCapability. Hash-then-replace is not CAS and multiple renames do not make a cross-file instantaneous transaction.

Written components are compared with planned poststate during installed verification; unwritten dependencies are still checked against before/cut expectations. Revocation, third state, unsafe before restoration, or backend unavailability yields paused/conflict/recovery_unknown rather than fake committed state.

D3 committed decision exists only after P seal. Failure to publish ContentCompletionProof leaves the decision reliably saved with portable publication pending.

## 13. fail-closed ordering and errors

wire12 retains the wire11 D3 family set, including identity_not_visible, identity_authority_unavailable, workspace_integrity_conflict, operation_id_conflict, owner_mismatch, root_operation_forbidden, identity_not_resolvable, entity_not_live, entity_not_restorable, invalid_ordinal, orphan_creation, structural_cycle, invalid_locator, stale_locator, identity_collision, cross_workspace_identity_preservation, identity_map_incomplete, operation_precondition_failed, inbound_reference_conflict, and identity_commit_aborted.

A new request has D6-owned gates before D3 ledger access:

1. D6 closed decode and current-principal minimum disclosure;
2. CommitDomain/replica/server qualification and backend fence;
3. open ConflictRecord, placeholder/incomplete transport, and P continuity;
4. InputDescriptor/pin availability and owner version;
5. then D3 identity gates corresponding to the old post-authorization sequence.

A D6-gate failure returns D6 v2 error rather than inventing a D3 family, and it does not read the D3 ledger key.

Inside D3, authorization still precedes existence; authority/custody availability precedes reachable integrity; different fingerprint is observed only after ledger continuity. Once an earlier gate wins, later state is not probed.

replica_local does not execute proposal P1/P2/TargetLedgerCustody. managed_atomic create/fork/continue and server authority retain the strong gates.

After planned, terminal_failed + identity_commit_aborted is legal only when the original plan can never commit and every installation remnant is safely resolved. Capacity, revocation, or uncertain recovery is not such proof.

## 14. operation-applicable qualification and semantic_pending

### 14.1 ordinary Document edit

Existing-Document ordinary source edit is owned by D6 source-save, not D3 identity_operation_request. D3 supplies only owner/lifecycle/identity prerequisites. D2 must be valid and local typed facts must pass their current Registry gates.

### 14.2 replica_local create/move/reorder/Trash

The local proof ranges are those in sections 7-8. Success may return complete_semantics or semantic_pending.

relation/unique/calendar/collection/inbound/cross_object_type obligations in semantic_pending are not complete success for:

- D7 complete Query negative-range proof;
- result/all_result derived write sets;
- bulk/collection postconditions;
- automation that relies on complete absence/uniqueness;
- restore/purge/copy/fork/import strong paths;
- D4/D5 Actions requiring complete cross-object constraints.

Until the D4/D5 owner afterimages define exact consumers, those paths return owner_update_required or version-unavailable semantics. Pending is never interpreted as an empty relation or valid complete set.

### 14.3 managed_atomic

managed_atomic proves the complete operation-applicable D3 closure plus the corresponding D4/D5/D7 dependencies. Failure to prove complete scope fails the operation and never silently downgrades to replica_local.

## 15. purge, Frontier, and offline replicas

purge preparation places the current ReplicaRecord active set and every active replica's accepted Frontier into D6 controlInputs as complete positive/negative dependencies.

The required frontier covers at least the target Trash ChangeId, all known lifecycle/placement/source heads, relevant inbound/reference proof, and current tombstone/allocation history.

Every active replica provides an accepted head proving it has no still-valid unseen branch before the required frontier. Failure to prove this pauses/conflicts purge rather than deleting payload.

An administrator may explicitly retire a permanently unavailable replica under current policy/trust and execution-responsibility checks. A retired epoch never reactivates. Old directory data later re-enters under a new ReplicaEpoch and current tombstones.

This avoids waiting for unknown never-registered devices while refusing to treat a sync provider's upload-complete flag as a completeness proof.

## 16. Stable boundary to D4-D10

### 16.1 D4/D5

D4 ref-valued fields remain typed refs. Exact semantic_pending consumption for relation, unique, calendar, and cross-object type is frozen by the D4 afterimage.

D5 similarly versions complete ranges for native/bulk/collection operations; D3 never infers completeness from a partial index.

### 16.2 D6

D6 physical paths, portable metadata records, P/I, safe installation, ConflictRecord, SourceVersion/2, Frontier, and execution responsibility follow the D6 owner afterimage. D3 has no generic patch bypass and P never becomes logical owner of parent/order.

### 16.3 D7

D7 Query/Action needs a new CommitDomain/frontier/SourceVersion consumer. PreparedActionBinding/1,/2 serve historical saved decisions only.

A future D7 afterimage defines the wire12-compatible preparation record. Before acceptance, a new request carrying preparationBinding cannot produce a successful decision. D3 does not copy a hypothetical PreparedActionBinding/3 schema here.

DefinitionTransfer and Result/9 Q-segment identity-transfer algebra continue under D3; D7 still owns SavedQuery/View/DynamicBlock payload decoding.

### 16.4 D8

D8 Source/Live/Read, Draft/IME/Undo/selection, and Server collaboration checkpoints bind SourceVersion/2. Presentation-mode changes never alter source or identity.

### 16.5 D9

D9 import/template/export consumes scoped pins and CommitDomain. Worker/IR IDs remain non-identity. ImportJob unknown first recovers the original request and never switches OperationId to retry blindly.

### 16.6 D10

D10 approval/claim/Money/sourceOccurrenceKey/stop lives in the continuous domain represented by D6 ExecutionResponsibilityRecord. Replica registration never copies, refunds, resets, or takes over those resources.

## 17. Server multi-user behavior and D3 serialization

Server may serve multiple principals and Drafts simultaneously. The principal/audience for D3 request comes from protected host/D6 context and cannot be borrowed from another session.

Different write sets may prepare concurrently; P/file installation and D3 decision linearize according to actual conflicts. A second same-document request with an old Base receives stale/conflict and retains its Draft.

Future collaboration produces one verifiable source/identity proposal into Core. Operation streams, presence, and CRDT objects are not NodeRef/SourceVersion/receipt.

## 18. Implementation impact graph

Implementation requires:

- D3 wire12 decoder/encoder/fingerprint;
- CommitDomain-scoped D3 ledger lookup;
- exact InputDescriptor/PinRef comparison;
- replica-local fresh-ID allocation and portable birth/tombstone facts;
- placement/lifecycle conflict projection;
- SourceVersion/2-aware invalidation of locators/evidence;
- local Trash versus managed restore/purge;
- resolver conflict/incomplete/placeholder states;
- legacy v9/v10/v11 replay routing;
- D6 protocolOwner=D3 decision-state integration;
- D7/D8/D9 future-consumer version gates.

Implementation convenience never introduces path identity, a global mutable current table, LWW, hidden ID reminting, or a second receipt.

## 19. Tests and minimal counterexamples

At minimum:

1. two replicas edit the same Document offline from one SourceVersion; both local reliable decisions survive and sync to source_concurrent;
2. A create/move plus B source edit composes only with proven causality; missing metadata is incomplete;
3. A Trash plus B edit has no delete-wins/edit-wins default;
4. source and portable metadata arriving separately never imply deletion or fresh identity;
5. incompatible birth claims for one typed ref yield identity_collision with no resolver winner;
6. A->B->A plus observation gap makes old locator/prepared/evidence stale despite equal digest;
7. crash injection across installation leaves third state unoverwritten, and post-seal publication failure leaves reliable + pending;
8. I deletion rebuilds; P loss never recreates decisions/unknown/Money;
9. r5 committed replay returns r5 bytes while current r6 is read separately;
10. two Server users keep same-document Drafts and the later old-Base commit cannot overwrite;
11. unrelated-document preparation remains concurrent without global-sequence false abort;
12. saved wire9/wire10/wire11 decisions use original decoders while new requests accept wire12 only;
13. partial index cannot authorize strong bulk/restore/purge or complete Query;
14. replica registration preserves WorkspaceId/refs and never calls continue;
15. duplicate local UUID claims resolve by explicit fresh-copy of the losing branch, not silent rekey;
16. purge pauses when active-replica frontier proof is missing and retired old data re-enters only under a new ReplicaEpoch;
17. D2-invalid external bytes are repair-readable but never produce successful D3 author receipt;
18. equal hash with different FileObject provenance is not installation proof;
19. revocation prevents seal of planned work while revocation after seal only hides delivery;
20. ContentCompletionProof failure never rolls back a sealed receipt and publication can be recovered later;
21. the same OperationId is independent across CommitDomain values and conflicts only within one domain;
22. semantic_pending never satisfies D7 all_result, Automation, or unique-negative completeness;
23. managed_atomic missing complete range never downgrades to replica_local;
24. historical Prepared1/2 remains historical and new binding is unavailable until the D7 owner update;
25. wire12 canonical request contains no permanent full-body copy; InputDescriptor plus purpose pins is compared exactly and hash-only equality is insufficient.

## 20. legacy compatibility and coordinated activation

D3-CJ/3, D3Integer, Ref/Locator lexemes, Annotation Value/3, Result/9, and the existing typed-identity algebra remain. wire12 changes only new-decision domain/frontier/input/guarantee and local-versus-strong qualification.

Saved v9/v10/v11 artifacts keep original bytes, pin retention, authority/custody gates, and receipt/error/outcome. They are never converted to wire12 to obtain replica-local semantics.

D3 wire12 is currently closed only with the produced D6-FA-r01 owner afterimage. D4/D5/D7/D8/D9/D10 consumers are incomplete, so this candidate cannot partially activate or produce a product fixture claiming the coordinated v12 passed.

## 21. D7 preparation and definition transfer

### 21.1 PreparedActionBinding ownership

PreparedActionBinding remains owned by D7. The fixed-S D3 section 21B mirror of /2 is historical source material and is not reproduced as a second drifting schema in this afterimage.

Saved wire9/v10/v11 decisions keep their original mirror/decoder. For wire12, the outer preparationBinding remains kind + bindingToken only and does not declare the backing-record version.

Until the D7 wire12-consumer afterimage is accepted, any new request carrying preparationBinding receives owner_update_required from the D6/D7 capability gate before D3 ledger access. D3 does not guess /3 fields, auto-upgrade /2, or create a second preparation ledger.

### 21.2 DefinitionTransfer and Result/9

DefinitionTransfer, the definitionTransfers array, Result/9 Q segment, and D3-Symbolic-Result/9 B/M/N/E/S/C/Q partition remain part of D3 identity-mutation algebra. D7 owns SavedQuery/View/DynamicBlock payload schema.

copy/fork/identity-bearing import involving recognized payloads still performs complete typed traversal, source/result occurrence binding, and candidate-map materialization. UUID-looking text/CEL is not a Ref.

### 21.3 exact input and pin lifetime

A future D7/D3 preparation ultimately produces D6 InputDescriptor/2, where large source/resource members in the canonical owner descriptor are represented by typed PinRef while the canonical request stays small.

Pins protected by planned/unknown/conflict/approval-money last references are never deleted by preview TTL. New D6/D7 versions define expirable historical effects. Evidence whose legacy protocol promised decision durability is not retroactively deleted.

## 22. Candidate acceptance boundary

This file is the complete D3 owner afterimage for D6-FA-r01, not independent acceptance. D3 terminology and implementation afterimages must remain synchronized, and later D4/D5/D7/D8/D9/D10 owners must consume the new versions before an immutable coordinated candidate exists.

The old D10 B13 result remains REVISE, terminology/bilingual FAIL, with 3 P1 and 8 P2 findings, 11 OPEN in total. This document closes none of them.
