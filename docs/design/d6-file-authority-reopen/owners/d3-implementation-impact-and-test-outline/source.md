---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 04f16e0f-a8ca-4d4a-8d75-007e46d44975.

# D3 Implementation Impact and Test Outline

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Fixed-S D3 Impact is historical source material. This file defines future implementation/acceptance obligations only and authorizes no source, dependency, release, deployment, or A2 work.

## 1. Implementation slices

| Slice | New responsibility | Preserved boundary |
|---|---|---|
| wire12 | DecisionKey/2, Frontier/2, InputDescriptor/2, and D3-native owner input | D3-CJ/3, D3Integer, Ref/Locator lexemes unchanged; receipt/companion retain real owners |
| scoped ledger | D6 DecisionKey/2 + protocolOwner=D3 with one primary decision in P | saved v9/v10/v11 replay unchanged |
| replica_local | local semantic proof for create_node, move, reorder, Trash; installation remains strict | never promoted to observed_only or complete-set proof |
| managed_atomic | restore/purge/copy/fork/continue/import and complete closure | never silently downgraded |
| resolver | conflict/incomplete/placeholder plus complete SourceObservation/1 currentness | never infer currentness from bare revision, production domain, or equal hash |
| portable identity | birth/parent-order/lifecycle/tombstone/no-reuse | path/I/P never logical owner |
| preparation | D7 backing schema remains D7-owned | no duplicate PreparedActionBinding/3 |
| sync conflicts | D6 ConflictRecord plus D3 typed resolution | no LWW or mtime winner |

## 2. Decoder and version routing

Implementation first routes by wireVersion:

- v9/v10/v11: saved decisions, planned recovery, and receipt/error/outcome replay only; no new decisions. Original bytes, fingerprint, pin retention, and authority/custody continuity remain unchanged.
- v12: only the request/owner-input shape jointly defined by G0-B and the A D6 owner afterimage.
- any other version: unsupported wire, with no compatibility guessing.

The v12 decoder first invokes D6 owner decoders for DecisionKey/2, CommitDomain/2, Frontier/2, ObservationScope/2, DependencyProof/2, OwnerInputBinding/2, InputDescriptor/2, and SourceObservation/1.
ownerInput has protocolOwner=D3 and ownerKind=d3_identity_operation/12, with writeProtection fixed strict in the descriptor.
It then checks Workspace, domain, guarantee, expectedFrontier, frontierPolicy, observationScope, owner descriptor, and request cross-field equality.

The same OperationId is independent in two CommitDomains. Different protocol owner or different fingerprint at one DecisionKey is operation_id_conflict.

## 3. Canonical request and pins

wire12 canonical request never permanently embeds full Document/Resource/Annotation bytes. The D3-native owner descriptor is d3_identity_operation/12 and stores only mode, strict WriteProtection, applicable expectedAuthority/workspaceProposal, and intent; large immutable input uses typed PinRef slots only. Source currentness is separately bound by complete SourceObservation/1 in InputDescriptor.sourceInputs.

Tests prove:

1. same canonical InputDescriptor but any different exact pin, SourceObservation, or DependencyProof is not the same input;
2. same sha256 or production SourceVersion with different observationEpoch/FileObjectBinding does not restore original input;
3. planned recovery reuses original pins/observation/dependency instead of rereading current source and inventing an equivalent request;
4. old v11 saved request bytes are not migrated to InputDescriptor/2;
5. pin cleanup follows D6 last-reference/retention and never drops planned/unknown/conflict/approval-money evidence;
6. a strict request never downgrades by mutating owner descriptor, frontierPolicy, or plan.

## 4. replica registration and control loss

### 4.1 New device

Fixture: portable F/M for Workspace W arrives on device B with none of A's P/I. B validates portable metadata, registers a fresh ReplicaEpoch, preserves W and all refs, and can perform ordinary work in a new CommitDomain while execution responsibility remains unavailable.

Counterexample: treating B as continue_workspace, or copying A's control.sqlite3/WAL/SHM so A and B can both spend one approval/Money resource.

### 4.2 Lost I

After deleting I:

- refs/resolver/parent/order/lifecycle recover from F/M;
- Query/search rebuild progressively;
- no fresh birth/receipt/tombstone is invented;
- ChangeId/Frontier is not reset.

### 4.3 Lost P

After deleting/corrupting P:

- the old replicaEpoch issues no new decision;
- portable InstallationNotice bounds possible unresolved ranges;
- current files never reconstruct original decision/unknown/Money;
- after authorized repair, retire the old epoch and register a fresh epoch;
- execution responsibility needs independent takeover proof.

## 5. replica_local qualification matrix

replica_local changes only the D3 semantic proof scope, never the installation-protection level. create_node, move_node, reorder_node, and Trash owner descriptors always use WriteProtection=strict, frontierPolicy=scope_dependencies, and ObservationScope/2=local_structure. observed_only belongs only to D6 human ordinary source-save of one existing live Document; any D3 mode selecting it fails before ledger access.

### 5.1 Existing source edit

Existing-Document ordinary source save remains owned by D6 source-save. D3 supplies current live owner/identity prerequisites only. D2-invalid proposals fail ordinary save while external invalid bytes remain in repair-read flow. Narrow Field/body authority never obtains whole-source replacement through a D3 action or observed_only.

### 5.2 create_node

Positive proof covers active replica, current parent/ancestor, complete destination sibling list, fresh allocation, D2-valid source, local typed facts, strict installation, portable birth+placement, and current SourceObservation/DependencyProof.

Negative cases:

- hidden/missing sibling range;
- parent in conflict/placeholder;
- fresh ID colliding with remote birth;
- invalid local typed fact;
- strict backend qualification unavailable;
- unrelated Frontier/2 extension followed by failure to revalidate the original dependency scope.

### 5.3 move/reorder

Positive coverage includes same-parent, cross-parent, final-index no-op, and compound movement. Complete old/new sibling lists, ancestor-cycle proof, current authorization, and original scope_dependencies range are mandatory.

Negative coverage includes duplicate final index, cycle, hidden sibling, concurrent move head, parent lifecycle conflict, observationEpoch change, or observed competition.

### 5.4 Trash

Positive coverage includes complete local subtree/owner/reply membership, Trash sibling order, policy, strict metadata installation, and relevant SourceObservation/DependencyProof.

If the complete inbound range is unproved, SemanticState is semantic_pending(inbound). A receipt never fabricates a global referenceLifecycleTransitions enumeration. An observed Trash/edit race is an explicit conflict and observed_only is never used as delete-wins/edit-wins.

## 6. managed_atomic strong gates

Every managed_atomic D3 mode fixes WriteProtection=strict, frontierPolicy=exact, and complete observation/dependency scope. Fixtures cover at least:

- full Trash with complete inbound range;
- restore to original/explicit location;
- purge with replica frontier;
- same-Workspace copy;
- owner-local copy;
- cross-Workspace transfer;
- fork;
- continue/failover;
- identity-bearing import;
- ordinary import.

Any missing complete range, payload materialization, authority/custody proof, D4/D5/D7 owner version, SourceObservation/DependencyProof, or pin continuity fails rather than downgrading to replica_local or switching to observed_only.

## 7. sync/conflict matrix

Required concurrent fixtures:

| Case | Expected |
|---|---|
| source/source | source_concurrent with both heads retained |
| create/create distinct IDs | both births retained; parent order may conflict |
| create/create same typed ref | identity_collision with no automatic winner |
| move/edit | compose only with proven causality and no conflict |
| move/move | placement_concurrent |
| Trash/edit | lifecycle_concurrent |
| Trash/restore | lifecycle conflict or strong-gate revalidation |
| body before metadata | incomplete_transport |
| metadata before body | incomplete_transport/placeholder |
| on-demand bytes absent | placeholder, not not_found |
| concurrent policy | policy_concurrent, no grant union |
| ABA watcher gap | observationEpoch advances and old evidence invalidates |

ConflictId stability also verifies identical complete ConflictKey -> same ID, new head -> new record superseding old open/prepared, and resolved history remains immutable.

## 8. identity collision and no-reuse

Offline UUIDv4 collision has deterministic acceptance:

1. both birth claims are locally reliable;
2. sync creates no canonical winner;
3. resolver reports conflicted;
4. resolution explicitly selects one claim;
5. losing payload gets fresh ID via copy/import;
6. original collision ref history is not rewritten;
7. tombstoned ID never revives;
8. retired-replica old bytes never reactivate the old birth.

"Probability is tiny" is not a semantic test.

## 9. purge Frontier/2, materialization, and tombstones

purge fixtures lock together:

- current replica-registry revision;
- active replica set;
- required Frontier/2;
- target Trash ChangeId;
- known source/lifecycle/placement heads;
- relevant SourceObservation/1;
- complete inbound/reference DependencyProof/2;
- D4/D5 complete obligations;
- allocation/tombstone history.

Frontier/2 proves only a sealed causal prefix. One missing active-replica causality head pauses/conflicts purge. Even with every head present, an unmaterialized placeholder, unreadable required bytes, or incomplete negative DependencyProof still pauses/unavailable purge with zero payload deletion.

production SourceVersion.commitDomain may differ from the current purge observerDomain and a complete current SourceObservation remains valid. The same production version with changed observationEpoch invalidates the old input.

After retiring a replica, purge may be reevaluated; the retired epoch never becomes active again. Old files later admitted use a fresh ReplicaEpoch, and current SourceObservation, tombstone, and ConflictRecord prevent identity resurrection. Never-registered devices are outside the active set, while a sync-provider “complete” flag is neither causal nor materialization acknowledgement.

## 10. Resolver and privacy

New resolver tests cover every typed entity as:

resolved | trashed | tombstoned | conflicted | incomplete | placeholder | not_found | not_visible | workspace_unavailable | invalid.

Without state-disclosure, existence distinctions are all hidden. Locator reaches resolved/stale/anchor ambiguity only after canonical live entity plus locator-disclosure qualification.

Equal bytes/span/token after observationEpoch change never restore old currentness.

## 11. D6 strict installation, same-P seal, and D3 decision

Every D3 identity/structure/lifecycle mode fixes WriteProtection=strict. Fault injection covers:

1. crash before P planned;
2. unknown planning transaction after pins/SourceObservation/DependencyProof are durable;
3. before InstallationNotice/2;
4. after notice before first component;
5. every staging/flush/install/directory-flush point;
6. observed competing state;
7. installed verification;
8. before P seal;
9. after seal before ContentCompletionProof/2;
10. during proof write/flush;
11. lost response delivery.

Each cell permits only D6 exact_before, exact_after_with_provenance, third_state, or unavailable combined with D3 decision state. An observed race is conflict/paused. Unknown installation occurrence or provenance is recovery_unknown and equal hash never guesses success.

Written targets compare with planned poststate while unwritten dependencies compare with original cut. A strict request never downgrades to observed_only in place, and D3 never invokes the human source-save weak path to complete a structure or lifecycle effect.

P seal is the only author-decision commit point. A portable effect allocates ChangeId/SourceVersion only there and saves the D3 primary receipt, D3DecisionCompanion/2, D6 state/effects, and applicable charge/outbox in the same transaction. third_state preserves current file/pins and is never overwritten or blindly rolled back. Publication failure after seal never terminalizes the committed decision and recovery only publishes the same proof.

## 12. r5/r6, DecisionKey replay, and no-op

Golden sequence:

- O5 commits a portable decision at DecisionKey K and loses response;
- O6 later commits an update;
- retry the exact original O5 request.

Only result: O5 returns the original D3 primary receipt bound to the same D3DecisionCompanion/2 while current state reads O6 separately. There is no source/metadata reinstall, second ChangeId allocation, second domainCommitSequence increment, repeated ApprovalUse/Money charge, or version rollback.

Revocation may hide O5 replay as not_visible but never alters the decision; regained authorization only redelivers the original result.

Also test all effectClass branches:
- portable: a real F/M author change, ChangeId allocated at seal and Frontier/2 advanced; sourceVersions lists only actually changed source.
- control_only: P-only control mutation, no content ChangeId, no Frontier advance, empty sourceVersions.
- no_op: all twelve D3 effect arrays empty and installation/save/publication not_applicable; one decision sequence may exist but content ChangeId/source revision/Frontier do not advance.

Explicit managed admission of an external equal-byte state changes source/control state and is never disguised as no_op.

## 13. Server multi-user

At minimum test:

1. A/B prepare different documents concurrently and both can finish.
2. A/B edit same document from one Base; A seals first and B retains Draft with stale/conflict.
3. B loses permission before B checkpoint and cannot commit.
4. A loses permission after A seal; history remains committed and later delivery is authorization-gated.
5. restart/failover fences both P and file writer.
6. old instance can only read/reject and cannot rename author files.
7. serialized SQLite writer never blocks multiple frontend Draft/session states.
8. two offline replicas that independently move/Trash the same structural range retain concurrent heads and create explicit conflict rather than LWW.
9. P/I schema audit proves there is no whole-workspace current-body/full-AST mirror or second parent/order authority.

Real-time collaboration remains unimplemented; these are future interface requirements, not passed product tests.

## 14. semantic_pending consumer gate

Existing D4/D5 afterimages do not yet consume the G0-A/G0-B source/observation/frontier contract. Until C, pending is fixed-reject as complete proof for:

- relation "no target";
- unique "workspace-global unique";
- complete Calendar expansion;
- collection-membership full postcondition;
- complete inbound range;
- cross-object type closure;
- D7 all_result/bulk;
- Automation depending on a complete Query.

Ordinary source presentation, Draft, local edit, and explicit branch read may consume pending while exposing unproved obligations rather than saying everything passed.

## 15. large-workspace and partial index

Test combinations include:

- I missing entirely;
- metadata-only I;
- candidate-search 50%/99%/100% coverage;
- parser-version change;
- OCR-version change;
- watcher gap;
- one million small files.

Ordinary edit/create/move/Trash waits only on its real local ranges. Strong Query/Action succeeds only with complete source scan or qualified complete coverage. A building index never treats unscanned objects as empty.

## 16. wire12 canonical and legacy corpus

New corpus independently covers:

- wireVersion12 accept; 0..11 and unknown rejected for new decisions;
- saved v9/v10/v11 replay router with original bytes/fingerprint/pins/authority-custody semantics unchanged;
- DecisionKey/2, protocolOwner=D3, same OperationId across domains allowed, same-key fingerprint conflict;
- replica/server CommitDomain;
- both Frontier/2 policies exact and scope_dependencies;
- ObservationScope/2 local_structure, workspace_constraints, prepared_workspace;
- closed d3_identity_operation/12 OwnerInputBinding descriptor with WriteProtection=strict;
- exact InputDescriptor equality plus pin, SourceObservation, and DependencyProof mismatch;
- replica_local/managed_atomic mode matrix;
- production SourceVersion domain may differ from observerDomain;
- observationEpoch change invalidates old evidence for the same production version;
- receipt effectClass portable/control_only/no_op plus same-P D3DecisionCompanion/2;
- ChangeId allocated only at seal;
- resolver conflict/incomplete/placeholder;
- unchanged D3-CJ/3 permutation;
- unchanged Result/9, Annotation Value/3, and Locator l1 version numbers.

Historical v9/v10/v11 corpus only regresses old decoders. It is never renumbered to 12 to claim new semantics passed and historical recovery contracts are never deleted merely because no deployment record was found.

## 17. Terminology gate

D3 Lexicon afterimage proves:

- exact fixed-S set of 42 conceptId values;
- unchanged exact-set ownedNames for every entry;
- unchanged firstFreeze;
- D6 imported names, including DecisionKey/2, Frontier/2, SourceObservation/1, OwnerInputBinding/2, D3DecisionCompanion/2, and WriteProtection, are not re-owned by D3;
- Preparation Binding/Definition Transfer/Definition Result Segment retain historical firstFreeze;
- any new technical field without a real owner mapping is rejected;
- retired identifiers remain disjoint from owned sets.

## 18. Downstream owner and activation gate

G0-B produces the D1 product consumption plus D3 wire12/terminology/impact companion afterimages, but the result remains candidate/not activated/not implemented.

Coordinated follow-up still must complete and be jointly accepted:

- D4: SourceObservation/production SourceVersion split, Frontier/2, weak B->N, and semantic_pending consumption;
- D5: native ordinary-save protection and local/complete structure ranges;
- D7: CommitDomain/Frontier/SourceObservation complete cut, new Prepared, and effects;
- D8: Source/Live/Read and collaboration checkpoints;
- D9: ImportJob/export scoped pins;
- D10: execution-responsibility sub-schemas and sourceOccurrenceKey.

A+B is not partial activation. Until the D4/D5 C batch and later owners are complete, the product cannot claim "wire12 coordinated managed success" as activated and G0-B fixtures never simulate an unclosed consumer.

## 19. Acceptance conclusion boundary

This file is an author candidate, not independent review. Future tests bind a fixed commit, real platforms/backends, and actual outputs; prose examples and documentation CI are not implementation acceptance.

The old D10 B13 result remains REVISE, terminology/bilingual FAIL, with 3 P1 and 8 P2 findings, 11 OPEN.
