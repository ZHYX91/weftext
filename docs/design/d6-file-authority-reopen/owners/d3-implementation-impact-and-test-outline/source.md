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
| wire12 | closed request/receipt using CommitDomain, Frontier, InputDescriptor, guarantee | D3-CJ/3, D3Integer, Ref/Locator lexemes unchanged |
| scoped ledger | WorkspaceId + CommitDomain + OperationId | saved v9/v10/v11 replay unchanged |
| replica_local | local reliable create_node, move, reorder, Trash | never promoted to complete-set proof |
| managed_atomic | restore/purge/copy/fork/continue/import and complete closure | never silently downgraded |
| resolver | conflict/incomplete/placeholder plus SourceVersion/2 currentness | no hidden-branch leakage |
| portable identity | birth/parent-order/lifecycle/tombstone/no-reuse | path/I/P never logical owner |
| preparation | D7 backing schema remains D7-owned | no duplicate PreparedActionBinding/3 |
| sync conflicts | D6 ConflictRecord plus D3 typed resolution | no LWW or mtime winner |

## 2. Decoder and version routing

Implementation first routes by wireVersion:

- v9/v10/v11: saved decisions, planned recovery, and receipt/error/outcome replay only; no new decisions.
- v12: only the D6-FA-r01 request shape.
- any other version: unsupported wire, with no compatibility guessing.

The v12 decoder invokes D6 owner decoders for CommitDomain/2, Frontier/1, and InputDescriptor/2 before checking Workspace/domain/guarantee/inputDescriptor cross-field equality.

The same OperationId is independent in two CommitDomain values; the same domain/key with different fingerprint is operation_id_conflict.

## 3. Canonical request and pins

wire12 canonical request never permanently embeds full Document/Resource/Annotation bytes. The D3 owner descriptor stores a closed semantic descriptor plus typed PinRef slots.

Tests prove:

1. same canonical InputDescriptor but one different exact pin is not the same input;
2. same sha256 with different source binding/provenance does not restore original input;
3. planned recovery reuses original pins instead of rereading current source and inventing an equivalent request;
4. old v11 saved request bytes are not migrated to InputDescriptor/2;
5. pin cleanup follows D6 last-reference/retention and never drops planned/unknown/conflict/approval-money evidence.

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

### 5.1 Existing source edit

Owned by D6 source-save; D3 supplies current live owner/identity prerequisites only. D2-invalid proposals fail ordinary save while external invalid bytes remain in repair-read flow.

### 5.2 create_node

Positive proof covers active replica, current parent/ancestor, complete destination sibling list, fresh allocation, D2-valid source, local typed facts, safe installation, and portable birth+placement.

Negative cases:

- hidden/missing sibling range;
- parent in conflict/placeholder;
- fresh ID colliding with remote birth;
- invalid local typed fact;
- backend offering only hash-then-replace.

### 5.3 move/reorder

Positive coverage includes same-parent, cross-parent, final-index no-op, and compound movement. Complete old/new sibling lists and ancestor cycle proof are mandatory.

Negative coverage includes duplicate final index, cycle, hidden sibling, concurrent move head, and parent lifecycle conflict.

### 5.4 Trash

Positive coverage includes complete local subtree/owner/reply membership, Trash sibling order, policy, and safe metadata installation.

If the complete inbound range is unproved, SemanticState is semantic_pending(inbound). A receipt never fabricates a global referenceLifecycleTransitions enumeration.

## 6. managed_atomic strong gates

managed_atomic fixtures cover at least:

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

Any missing complete range, authority/custody proof, D4/D5/D7 owner version, or pin continuity fails rather than downgrading to replica_local.

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

## 9. purge frontier and tombstones

purge fixture locks:

- current replica-registry revision;
- active replica set;
- required Frontier;
- target Trash change;
- all known source/lifecycle/placement heads;
- complete inbound/reference proof;
- D4/D5 complete obligations;
- allocation/tombstone history.

One missing active-replica frontier pauses/conflicts purge with zero payload deletion.

After retiring a replica, purge may be reevaluated; the retired epoch never becomes active again. Old files later admitted use a fresh ReplicaEpoch and tombstones prevent identity resurrection.

Never-registered devices are outside the active set, and a sync-provider "complete" flag is not acknowledgement.

## 10. Resolver and privacy

New resolver tests cover every typed entity as:

resolved | trashed | tombstoned | conflicted | incomplete | placeholder | not_found | not_visible | workspace_unavailable | invalid.

Without state-disclosure, existence distinctions are all hidden. Locator reaches resolved/stale/anchor ambiguity only after canonical live entity plus locator-disclosure qualification.

Equal bytes/span/token after observationEpoch change never restore old currentness.

## 11. D6 installation plus D3 decision

Fault injection covers:

1. crash before P planned;
2. unknown planning transaction after pins durable;
3. before InstallationNotice;
4. after notice before first component;
5. every staging/flush/install/directory-flush point;
6. installed verification;
7. before P seal;
8. after seal before ContentCompletionProof;
9. during proof write/flush;
10. lost response delivery.

Each cell permits only D6 exact_before/exact_after/third_state/unavailable combined with the D3 decision state.

Written targets compare with planned poststate while unwritten dependencies compare with original cut. Installing after and then requiring target=before is forbidden.

third_state preserves current file/pins and is never overwritten or blindly rolled back. Publication failure after P seal never terminalizes the committed decision.

## 12. r5/r6 and saved replay

Golden sequence:

- O5 reliably commits r5 and loses response;
- O6 later commits r6;
- exact retry of O5.

Only result: O5 returns original r5 receipt bytes, current-source read returns r6, with no second source write, repeated ApprovalUse/Money charge, or revision rollback.

Revocation may hide O5 replay as not_visible but never alters decision; regained authorization only redelivers original decision.

## 13. Server multi-user

At minimum test:

1. A/B prepare different documents concurrently and both can finish.
2. A/B edit same document from one Base; A seals first and B retains Draft with stale/conflict.
3. B loses permission before B checkpoint and cannot commit.
4. A loses permission after A seal; history remains committed and later delivery is authorization-gated.
5. restart/failover fences both P and file writer.
6. old instance can only read/reject and cannot rename author files.
7. serialized SQLite writer never blocks multiple frontend Draft/session states.

Real-time collaboration remains unimplemented; these are future interface requirements, not passed product tests.

## 14. semantic_pending consumer gate

Before D4/D5 afterimages are complete, pending is fixed-reject as complete proof for:

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
- saved v9/v10/v11 replay router;
- replica/server CommitDomain;
- same OperationId across domains allowed and same-domain fingerprint conflict;
- exact InputDescriptor equality and pin mismatch;
- replica_local/managed_atomic mode matrix;
- receipt domain/changeId/guarantee/semanticState members;
- resolver conflict/incomplete/placeholder;
- unchanged D3-CJ/3 permutation;
- unchanged Result/9, Annotation Value/3, and Locator l1 version numbers.

Historical v9/v10/v11 corpus only regresses old decoders and cannot be renamed to 12 to claim new semantics passed.

## 17. Terminology gate

D3 Lexicon afterimage proves:

- exact fixed-S set of 42 conceptId values;
- unchanged exact-set ownedNames for every entry;
- unchanged firstFreeze;
- D6 imported names are not re-owned by D3;
- Preparation Binding/Definition Transfer/Definition Result Segment retain historical firstFreeze;
- any new technical field without a real owner mapping is rejected;
- retired identifiers remain disjoint from owned sets.

## 18. Downstream owner and activation gate

Coordinated follow-up must complete:

- D4 semantic_pending consumption;
- D5 local/complete structure ranges;
- D7 CommitDomain/frontier/new Prepared/effects;
- D8 Source/Live/Read and collaboration checkpoints;
- D9 ImportJob/export scoped pins;
- D10 execution-responsibility sub-schemas and sourceOccurrenceKey.

Until these owners are jointly accepted, the product cannot claim "wire12 coordinated managed success" as activated.

## 19. Acceptance conclusion boundary

This file is an author candidate, not independent review. Future tests bind a fixed commit, real platforms/backends, and actual outputs; prose examples and documentation CI are not implementation acceptance.

The old D10 B13 result remains REVISE, terminology/bilingual FAIL, with 3 P1 and 8 P2 findings, 11 OPEN.
