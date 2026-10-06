---
source_language: zh-CN
translation_of: D3-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D3-IMPACT.zh-CN.md)

Source document ID: 04f16e0f-a8ca-4d4a-8d75-007e46d44975.

# A2 D3 Implementation Impact and Test Outline

Companions: [D3 main](D3.md) · [current schemas](D3-SCHEMAS.md) · [source map](D3-SOURCE-MAP.json)

Status: A2 D3 current implementation/test companion; author candidate, not independently accepted, implemented, or activated. All parent implementation obligations, cases and minimal counterexamples are retained. Fresh current carriers use wire13/Proof3/Notice3/CP4/PAB4/Value4; historical records recover by their recorded decoders/bytes/pins/recovery. These are design acceptance obligations, not executed product tests.

## 1. Implementation slices

This file defines only future implementation, regression, crash-recovery, privacy, budget, and acceptance obligations. Every “must test” or “must verify” statement is a specification requirement, never a claim that the work has already run or passed. Implementation slices must land against the boundaries already frozen by the D3 main afterimage, the D3 Lexicon, and the real D6 P1 producers rather than creating lookalike interfaces.

| Slice | New responsibility | Preserved boundary |
|---|---|---|
| wire12 and ledger branching | consume `DecisionKey/2`, `CommitDomain/2`, `Frontier/2`, `InputDescriptor/2`, and implement the unique saved/planned/unseen split after the common gate | `D3-CJ/3`, `D3Integer`, Ref/Locator lexemes, and the original stage/priority of all 24 D3 error families remain unchanged; one P has one primary decision |
| source currentness and revision | separate production `SourceVersion/2` from local current `SourceObservation/1`/`SourceVersionRef/1`, consuming `SourceStamp/1`, `SourceRevisionPlan/1`, `RevisionTokenSource/2`, `RevisionTokenBinding/2`, and `d6_source_revision/2` | historical opaque revision-token/Locator decoders are not upgraded; foreign production revision/epoch/`externalSequence` never donate to a new managed revision |
| dependency/range | consume the fifteen closed `DependencyKey/3` kinds and the nine D3 `StructureRange` variants, enforcing authorization-before-hidden-read, complete positive/negative/empty proof, and P/M continuity | partial I, no-hit, equal hash/count, provider state, and Frontier prefix are never complete-set proof |
| `replica_local` | real local_structure + `scope_dependencies` for `create_node`, `move_node`, `reorder_node`, and `trash` | installation remains `WriteProtection=strict`; the same plan continues only with a complete continuous proved-unrelated sealed extension |
| `managed_atomic` | restore/purge/copy/fork/continue/import and every strong closure | always `frontierPolicy=exact` + `WriteProtection=strict`, never silently downgraded to local |
| copy/fork/import/Definition Transfer | one private candidate map, owner/membership matrix, real preimage/result pins, all typed slots, Q bijection, and two-pass same-source position closure | ordinary CEL/text/unknown JSON is not scanned, and no D7 schema, Definition identity, or second wire is invented |
| portable publication | separate same-decision receipt/companion and local `SourceVersionRef/1` projection from `ContentCompletionProof/4.sourceChanges` production transport | `InstallationNotice/3.baseFrontier` is immutable; receiver constructs its own local Observation/Ref |
| conflict/recovery | current `ConflictRecord/2 + Frontier/2`, historical `/1 + Frontier/1`, real sealed heads, and saved/planned/unknown recovery | `ConflictKey/1`, `ConflictId`, and `D6-ConflictKey/1` hash domain remain unchanged; no LWW/mtime winner |
| ordinary save | D6 trusted-human ordinary single-Document source-save with independent ordinary/complete and strict/observed_only axes | D3 identity/structure/lifecycle, D5 structured, Server checkpoint, strong Action/Automation/Approval/Money remain strict |
| downstream gates | D4/D5/D7/D8/D9/D10 consume the new producer contract only where they genuinely depend on it | an uncoordinated strong consumer gates only its dependent path and never permanently disables qualified ordinary/local/offline capability |

All fixed-S snapshots, design inputs, and catalog bytes are read-only. P2 synchronizes only real owner afterimages plus the three routing metadata files; this Impact never authorizes test preparation to modify historical inputs.

## 2. Decoder, common gate, and version routing

Implementation first routes by the actual stored-record version and wire version, preserving historical bytes/decoders from current-protocol reinterpretation:

- v9/v10/v11: only actually existing saved/planned/unknown, receipt/error/effects, Preparation/Observation/token/pin records recover under the original decoder, bytes/fingerprint, original profile, authorization, custody, TTL/clock, retention, and no-duplicate-effect obligations. A decoder, fixture, draft, or candidate prose does not prove that the prototype was deployed or active.
- v12: accept only real D6 owner-decoder outputs for `DecisionKey/2`, `CommitDomain/2`, `Frontier/2`, `ObservationScope/2`, `DependencyProof/3`, `OwnerInputBinding/2`, `InputDescriptor/3`, and applicable `SourceObservation/1`, then let D3 validate the closed owner request.
- any other version: unsupported; never guess compatibility, renumber old records, or bulk-migrate legacy bytes.

Fault-injection and replay tests must prove the common ordering:
1. outer/D3 closed static decode, canonicalization, and cross-field checks depending only on request bytes; stage1/2 `invalid_identity_ref`, `domain_kind_mismatch`, and `workspace_ref_misbound` keep their original priority.
2. current-principal minimum state/result disclosure, the record's original-profile `ObservationScope/2`/mode authorization, and prior `CommitDomain/2` qualification; hidden business content is never read first to select a narrower scope.
3. domain fence, portable trust/backend, P ledger, and custody/authority continuity; availability/unproved continuity precedes reachable integrity.
4. locate the same `DecisionKey/2` using the record's actual version, complete original canonical request/fingerprint, and `protocolOwner`, then immediately branch:
   - saved: check only current delivery authorization for the original actual effect/mode or original result-disclosure scope, then return original bytes or resume original publication/outbox;
   - planned: resume only the same frozen plan;
   - unseen: only now enter current owner/consumer, Observation, DependencyProof, revision basis, Frontier, budget, and planning CAS.

Current `SourceObservation/1`, `DependencyProof/3`, current Frontier, or a new D4/D5/D7 consumer is never a uniform precondition before original-ledger lookup. The same `OperationId` may identify independent keys under different complete `CommitDomain/2` values; wrong owner/different fingerprint at the same key keeps the original conflict behavior without exposing the original request's business content.

The D6 front door uses only existing v2 errors. The complete D3 family remains `invalid_identity_ref`, `domain_kind_mismatch`, `workspace_ref_misbound`, `identity_not_visible`, `identity_authority_unavailable`, `workspace_integrity_conflict`, `workspace_identity_conflict`, `operation_id_conflict`, `owner_mismatch`, `root_operation_forbidden`, `identity_not_resolvable`, `entity_not_live`, `entity_not_restorable`, `invalid_ordinal`, `orphan_creation`, `structural_cycle`, `invalid_locator`, `stale_locator`, `identity_collision`, `cross_workspace_identity_preservation`, `identity_map_incomplete`, `operation_precondition_failed`, `inbound_reference_conflict`, and `identity_commit_aborted`. Original stage6 stays empty and no new error family occupies it.

## 3. Canonical request, pins, source revision, and plan freeze

wire13 canonical request never permanently embeds complete Document/Resource/Annotation bytes. `d3_identity_operation/13` owner descriptor, `InputDescriptor/3`, typed `PinRef`, complete `SourceObservation/1`, `DependencyProof/3`, and owner-specific protected input compare item-for-item; equal hash or equal final text is insufficient.

Implementation proves:
1. equal canonical `InputDescriptor/3` bytes with any different exact pin, Observation, DependencyProof, or `FileObjectBinding` is not the same input.
2. equal production `SourceVersion/2` with changed current `observationEpoch` or `FileObjectBinding` never restores old currentness.
3. planned recovery uses original pins/Observation/dependency/candidate map/version basis rather than rereading current source and fabricating an equivalent request.
4. pin cleanup follows real last-reference/retention and never removes evidence still needed by saved/planned/unknown/conflict/Approval/Money/outbox recovery.
5. a strict request never downgrades in place by mutating owner descriptor, `frontierPolicy`, pins, or plan.

Production/current separation has executable coverage:
- both managed and external `SourceVersion/2` retain production `CommitDomain/2` plus production `observationEpoch`; current `SourceObservation/1` separately has observer `CommitDomain/2`, current `observationEpoch`, `FileObjectBinding`, and pins.
- `SourceVersionRef/1` selects only the complete protected Observation; a receiver or peer replica never copies the sender `sourceToken`.
- when two replicas start from one production before/source cut, each creates and validates its own complete local current `SourceObservation/1` under its operation `CommitDomain/2`.

Acceptance for `H(D,E)`, `SourceRevisionPlan/1`, and revision tokens covers:
- `H(D,E)` does not reset across production `observationEpoch` in one production domain; a managed after uses checked H+1 from continuous sealed history and MAX never wraps.
- foreign production revision, foreign production epoch, and `externalSequence` never donate the new production-domain revision.
- H=0 is legal only for a genuinely completely proved empty history; missing/corrupt/gapped/unproved history is not empty.
- `SourceRevisionPlan/1` freezes before Observation or proved-absent, same-domain `lastIssued`/empty-history basis, `SourceStamp/1`, exact `afterPin`, candidate map, and version basis; after winning planning CAS, H/revision/token/identity is never resampled.
- `RevisionTokenBinding/2` is the closed stable production-address record `{kind,version,token,source}`; `source` is `RevisionTokenSource/2` and the tag is `d6_source_revision/2`. A managed plan fixes one proposed token before C/Q materialization; winning CAS/seal selects the sole canonical binding and original sealed-outbox association, while loser/aborted/unproved tokens never borrow another seal. New read qualification separately requires exact current SourceObservation.sourceVersion equality; bare stamp/version, caller-selected token, equal hash or old runtime evidence is rejected.
- only seal of a real source change combines the frozen stamp with the same decision's `ChangeId/1` into managed `SourceVersion/2`.
- true raw no-op creates no managed after; source-unchanged structure/lifecycle creates no managed after/H increment; delete uses absent after with no after revision/H increment; equal-byte external admission remains external-before + managed-after.

Acceptance scenarios map to main counterexamples 1–7, 26, 53, and 54 and require success, stale, gap, unknown, and crash/replay variants rather than happy-path-only coverage.

## 4. Replica registration, lost I, lost P, and responsibility continuity

### 4.1 New device

Fixture: ordinary files plus portable F/M for Workspace W arrive at device B, with none of device A's P/I. B first validates Workspace/portable trust, replica registry, birth/tombstone/structure records, and actual files, then registers a new `ReplicaEpoch`, preserves W and existing refs, and receives ordinary content qualification only in its new `CommitDomain/2`.

Counterexamples:
- treating B as `continue_workspace`;
- copying A's control DB/WAL/SHM so A and B can both consume the same Approval/Money;
- using replica registration to take over old P, unknown state, external effect, or D10 execution responsibility.

### 4.2 Lost I

I is fully discardable. Recovery proves:
- identity, parent/order, lifecycle, portable conflict/tombstone recover from real F/M/P owners;
- Query/search/parser/OCR rebuild progressively;
- no new birth/receipt/tombstone is minted and `ChangeId/1`/`Frontier/2` never resets;
- I does not own proof epoch/revision, complete-enumeration boundary, empty proof, or continuous-consumption position.

All fourteen `DependencyKey/2` stamps are tested with I absent, partially built, parser/OCR version changes, watcher gaps, and one million small files. Rebuilding I alone never re-signs proof; only real loss of correctness facts/continuity requires a new proof epoch.

### 4.3 Lost P

After P is lost/corrupt:
- old `ReplicaEpoch` issues no new decision;
- portable Notice/Proof plus actual components only bound recovery/conflict and never reconstruct original decision/unknown/Money from current files;
- unknown installation, Approval/Money/claim/outbox/stop responsibilities remain and equal hash or an empty control DB never fabricates a terminal outcome;
- after safe reconciliation, old epoch may retire and a new epoch may register for ordinary content;
- execution-responsibility takeover requires independent custody/fence proof.

These correspond to acceptance scenarios 40, 41, 44, and 50 and must prove both that ordinary content can continue and that global execution responsibility remains unavailable.

## 5. ordinary/complete and strict/observed_only axes

`ordinary|complete` is the semantic-proof axis; `strict|observed_only` is the installation-protection axis. Neither axis implies the other.

Every D3 identity/structure/lifecycle mode, including `create_node`, `move_node`, `reorder_node`, `trash`, restore/purge/copy/fork/continue/import, remains `WriteProtection=strict`. D5 structured cell/row/column/reorder, bulk/collection/promotion, D7 strong Action, Automation, Server checkpoint, Approval, and Money are also strict.

`observed_only` belongs only to D6 trusted-human `interactive_source_save`, explicitly selected and frozen by the human before planning starts. A complete positive case has every condition:
- exactly one existing live Document;
- `saveProfile=ordinary` + `guarantee=replica_local`;
- whole-source complete read/replace;
- author-source write set empty or containing only that Document;
- no applicable body/Field/node-control deny;
- no identity, parent/order, lifecycle, shared-policy, Registry, Calendar, or other-entity mutation;
- Draft Base exactly binds the complete current `SourceObservation/1`;
- the actually read/frozen before `B` and user input `N` enter durable plan/retention.

The real weak-protection risk is part of the test oracle: an external `C` that appears only after the final trusted object/event-continuity check and was never observed may be overwritten by `N`, and `C` may have no recoverable copy. A later C may again become current but never erases durable B/N or rewrites the historical receipt/proof.

Negative coverage includes:
- a strict plan never falls back to `observed_only` after backend/capability failure;
- revocation, known competition, stale Base, watcher/event gap, deny, or insufficient P durability never qualifies for weak save;
- observed competition goes conflict/reprepare/paused;
- unknown install owner/outcome goes `recovery_unknown`;
- prepared or `inputRetentionState=retained` is not Saved;
- sealed `durable_observed_only` never masquerades as strict `reliable`.

These correspond to acceptance scenarios 33–36 and 55. No per-attempt approval workflow is invented to “strengthen” this weak path.

## 6. `replica_local` and `managed_atomic` operation matrix

`replica_local` narrows semantic dependency scope only and never changes installation protection. Exactly four D3 local_structure modes use `scope_dependencies`: `create_node`, `move_node`, `reorder_node`, and `trash`. They preserve original subject/parent/ordinal/closure/proposed after/pins/version basis/`InstallationNotice/2.baseFrontier` and never reselect a plan merely because current Frontier advanced.

Continuing the same plan proves all of:
- original base -> actual current cut is one complete continuous verified-sealed non-regressing chain;
- every added/advanced head has real portable ChangeRecord/completion history;
- every actually bound source/control/auth/Registry/rules/membership/positive-negative range/owner-version dependency among the fourteen kinds still holds;
- new sealed effects are genuinely unrelated to those complete dependencies;
- unrelatedness evidence and actual cut are retained durably in P rather than inferred from two vectors after P loss.

Every D3 `managed_atomic` mode remains `frontierPolicy=exact` + `WriteProtection=strict`. Missing complete range, payload, authority/custody, owner version, pin continuity, or strong consumer gates only the strong path that truly requires it; there is no automatic local downgrade and Server parallelism never turns it into `scope_dependencies`.

### 6.1 create/move/reorder/Trash local positive/negative

The old Impact create/move/reorder/Trash positive and negative cases remain, with exact checks for all nine `StructureRange` variants:
`live_children`, `trash_children`, `trash_roots`, `ancestor_chain`, `subtree`, `owner_resources`, `owner_annotations`, `reply_closure`, and `restore_membership`.

Every variant tests:
- complete non-empty;
- complete empty;
- hidden member;
- missing shard/I/O failure;
- partial/building I;
- watcher/journal gap;
- auth/revocation change during final revalidation;
- ordered-boundary/cycle/owner/reply invariant;
- equal hash/count/no-hit never substituting for complete proof.

Authorization/disclosure precedes reads of hidden siblings, owner members, reply edges, inbound slots, and restore membership.

### 6.2 copy/fork/import/continue and Definition Transfer

Coverage preserves the real mode/owner/existing-container matrix:
- `copy_node_subtree` defaults to the legal live closure; a trashed owner-local member never revives because owner matches;
- standalone `copy_resource|copy_annotation` primary fresh-maps to the explicit destination owner and never reowners in place;
- only the destination-owner existing container allowed by the original mode may change under the paired preimage/result/slot/S matrix;
- a `copy_annotation` fresh mapped primary expresses initial reply only through reference plan and is never an S container;
- exact fork covers the complete live+trashed closure with complete identityMap and exact lifecycle/Trash-placement partition;
- `continue_workspace` requires old authority stopped/fenced and either no committed facts after the cut or an authoritative ledger sufficient to fully merge those post-cut facts;
- partial identity-bearing bundle uses only artifact/source pins as preimage and never queries source authority online;
- ordinary import worker/IR/provider IDs never become content identity.

All fresh/mapped identity uses one private candidate map. Definition Transfer traverses all actual typed roots: `DefinitionAddress.owner`/complete Locator, TypeSpec-declared Ref, `CollectionCreationPolicy.parent`, saved View/Query calls, Query selector literals, `QueryRef` arguments, parameter defaults, fixed View Domain `TypedLiteral`, and DynamicBlock literal/context. Ordinary CEL/text/unknown JSON is not scanned.

slot/Q acceptance proves:
- exactly one slot per typed Ref/Locator root and no overlapping owner slot for a complete Locator root;
- a recognized no-Ref payload still has `slots=[]`;
- source/result occurrences and real pins validate independently;
- Q is one-to-one with the saved-definition payload span and never overlaps B/M/N/E/S/C;
- two-pass position materialization uses the same candidate map, `SourceRevisionPlan/1`/revision token, and same final source;
- unequal second-pass span, ambiguous/missing target, missing typed slot, and wrong owner/type reject the whole operation.

These correspond to acceptance scenarios 13–21 and require executable fixtures rather than a reference back to main prose.

## 7. sync/conflict, identity collision, and no-reuse

The concurrency matrix continues to cover:
- source/source -> `source_concurrent` with both heads retained;
- create/create distinct IDs -> both births retained, parent order may conflict;
- create/create same typed ref -> `identity_collision`;
- move/edit -> compose only when causality, metadata/structure, and actual dependencies are all proved;
- move/move -> `placement_concurrent`;
- Trash/edit -> `lifecycle_concurrent`;
- Trash/restore -> lifecycle conflict or exact strong revalidation;
- body/metadata split arrival -> `incomplete_transport`;
- on-demand bytes -> placeholder/not_materialized;
- policy concurrency -> no grant union;
- ABA watcher gap -> old evidence/token/Locator becomes stale.

Current conflict uses `ConflictRecord/2 + Frontier/2`, while `ConflictKey/1`, `ConflictId`, and `D6-ConflictKey/1` hash domain remain unchanged. Every head is a real continuously verified sealed `ChangeId/1` relevant to the key; external competition, unknown install, and third-state bytes without a sealed head never fabricate one.

Historical `ConflictRecord/1 + Frontier/1` uses only its original decoder/bytes/recovery. /1 and /2 are never two current records for the same unchanged `ConflictKey/1`. Resolved history never reopens and UUID/path/mtime/final hash never selects a winner.

Identity-collision/no-reuse coverage still proves two locally reliable birth claims, no canonical winner, explicit branch resolution, losing-content fresh-copy/import under a new ID, unchanged historical collision ref, no tombstone revival, and no reactivation from retired-replica old bytes.

These correspond to acceptance scenarios 46–51.

## 8. purge, restore, and replica coverage

Restore and purge membership are tested separately.

Restore:
- only `managed_atomic` + `frontierPolicy=exact` + `WriteProtection=strict`;
- restores only members from the original Node Trash/lifecycle decision's `restore_membership` that remain restorable;
- Resource/Annotation independently trashed earlier never restores merely because owner matches;
- requires real Trash graph/location, owner directories, reply closure, refs, and other-owner complete dependencies.

Purge:
- also `managed_atomic` + `frontierPolicy=exact` + `WriteProtection=strict`;
- covers every currently trashed owned Resource/Annotation in the target owner closure, including members trashed independently earlier;
- complete `replica_registry` proves the active/retired `ReplicaRecord` directory, `registrationSequence`, gap-free continuity, and real admission of the required sealed cut by every active replica or valid retirement;
- requires complete inbound/foreign/control, D4 relation incidence, Registry, Calendar/temporal, D5, and other strong-owner proof;
- provider “synced”, current-online set, stable Frontier, count/hash, partial I, and local-Trash `semantic_pending` never qualify purge;
- auth/disclosure precedes hidden-range reads and unauthorized failure leaks no negative range, member count, or conflict count.

A source-unchanged purge lifecycle effect may have no source change but still has a real portable `ChangeId/1`; delete after absent creates no after `SourceRevisionPlan/1` or H increment.

These correspond to acceptance scenarios 10–12 and 26. Missing any active-replica causality/ack or complete range leaves paused/unavailable/conflict with zero payload deletion.

## 9. Dependency completeness, large workspace, and partial index

`DependencyProof/3` has exactly fifteen closed `DependencyKey/3` kinds:
`source`, `lifecycle`, `placement_range`, `ref_inbound`, `relation_incidence`, `calendar_scope`, `registry`, `temporal_rules`, `authorization`, `foreign_binding`, `query_scan`, `replica_registry`, `conflict_record`, and `execution_resource`.

Tests use the actual owner for positive/negative/empty/currentness proof rather than generic owner JSON. Key focus:
- `source`: complete current Observation, bytes/value pin, and `FileObjectBinding`;
- D3 `lifecycle`, `placement_range`, and `ref_inbound`: real portable identity/structure/reference directories;
- D4 relation/calendar/registry/temporal: real D4/Registry owner;
- `authorization`: current principal/audience/policy generation;
- `foreign_binding`: SourceBinding/OriginBinding continuity and comparator;
- `query_scan`: D7 complete visible enumeration, never page/cursor/partial I;
- `replica_registry`, `conflict_record`, and `execution_resource`: real D6 control records.

Large-workspace matrix remains:
- I completely missing;
- metadata-only I;
- 50%/99%/100% candidate search;
- parser/OCR version change;
- watcher gap;
- 1,000,000 small files.

An ordinary local operation waits only on its actual local ranges. Strong Query/Action succeeds only with complete source scan or qualified complete coverage. A building index never treats unscanned objects as empty.

These correspond to acceptance scenarios 8, 9, 22, and 45.

## 10. Resolver, privacy, and non-disclosure

Resolver still covers:
resolved | trashed | tombstoned | conflicted | incomplete | placeholder | not_found | not_visible | workspace_unavailable | invalid.

Without state disclosure, existence distinctions remain hidden. Locator reaches resolved/stale/anchor ambiguity only after canonical live owner plus locator disclosure; stale never fuzzy-reanchors.

Implementation verifies:
- closed decode -> workspace/domain binding -> current disclosure -> continuity -> conflict/incomplete -> identity lifecycle -> locator disclosure -> revision/coordinate;
- hidden sibling/member/reply/inbound/range reads happen only after the corresponding authorization succeeds;
- auth/visibility/membership/owner-rule changes during final revalidation invalidate proof;
- equal bytes/span/hash/token payload never restores currentness across observer epoch/gap/replacement;
- D3 range or Frontier prefix never replaces D4/D6/D7 owner's complete-set/absence/Query proof;
- preserving a foreign Ref grants no foreign-Workspace enumeration/disclosure.

These correspond to acceptance scenarios 6, 48, 49, and 52.

## 11. Strict installation, P seal, Notice3/CP4/ChangeRecord1, and crash matrix

Every D3 identity/structure/lifecycle mode remains `WriteProtection=strict`. Fault injection preserves every old Impact install point and updates publication to the current P1 contract:
1. before P planned;
2. planning transaction unknown after pins/Observation/DependencyProof durable;
3. before `InstallationNotice/3`;
4. after notice before first component;
5. every staging/flush/install/directory-flush point;
6. observed competition;
7. installed verification;
8. before P seal;
9. after seal before `ContentCompletionProof/4` generation/persistence;
10. during CP3 write/flush/transport;
11. lost response delivery.

Each cell lands only in real exact-before, exact-after-with-provenance, third-state, unavailable/recovery_unknown plus D3 decision state. Equal hash with different `FileObjectBinding`/provenance never proves original-plan installation.

P seal is the sole author-decision commit point. In one transaction, a portable effect:
- allocates the decision's real `ChangeId/1`;
- revalidates frozen `SourceRevisionPlan/1` for actual source changes and forms managed production `SourceVersion/2` where applicable;
- stores D3 primary receipt, `D3DecisionCompanion/2`, D6 state/effects, and applicable charge/outbox;
- creates no source version for source-unchanged structure/lifecycle;
- creates no content ChangeId for raw no-op;
- represents deletion as production before + absent after;
- represents equal-byte external admission as external-before/managed-after.

`ContentCompletionProof/4` verifies:
- `components` exactly matches original `InstallationNotice/3.components` key set/order;
- `sourceChanges` covers only actual source-state changes using complete production `SourceVersion/2|absent`;
- local `SourceVersionRef/1` projection in current receipt/effect metadata never substitutes for portable production transport;
- `frontierBefore` is the actually validated `Frontier/2` immediately before seal and `frontierAfter` adds exactly this proof's `changeId`;
- exact branch base is byte-equal to original expected/dependency/Notice base;
- `scope_dependencies` branch carries complete continuous sealed history, original-dependency unrelatedness revalidation, and durable P evidence without changing Notice base;
- receiver validates production history and constructs its own `SourceObservation/1`/`SourceVersionRef/1` under its observer `CommitDomain/2`, `FileObjectBinding`, current epoch/pins, never copying sender token;
- CP3 grants no complete Query/Action/negative-range/execution-responsibility qualification;
- pre-seal restored outcome exists only when every component is completely proved restored to before and carries no success/ChangeId;
- post-seal publication failure remains committed + pending and recovery publishes only the same proof/outbox without reinstall, revision resampling, another H/ChangeId/domainCommitSequence advance, or charge.

These correspond to acceptance scenarios 23–27, 44, and 50.

## 12. Replay, saved/planned/unseen, no-op, and unknown

The golden r5/r6 sequence remains and expands:
- O5 portable-commits under `DecisionKey/2` and loses response;
- O6 later changes current source/state;
- retry the exact original O5 request;
- after common continuity/custody plus original request/fingerprint success, current delivery authorization is checked only for O5's original actual effect/mode or original result-disclosure scope;
- the only outcome is original O5 receipt/error/effects bytes or original publication/outbox while current O6 is read separately.

Saved replay does not require old before to remain current, old Frontier to equal current, old preview/preparation TTL to be current, or a new D4/D5/D7 consumer to approve the old result. Revocation hides delivery only and never changes historical decision; restored authorization delivers the same old bytes. There is no reinstall, H/revision/ChangeId reallocation, second domainCommitSequence increment, repeated ApprovalUse/Money charge, or duplicate external effect.

Planned recovery restores only original canonical request/fingerprint, `InputDescriptor/2`, candidate map, `SourceRevisionPlan/1`/H basis, pins, reservations/write set, `InstallationNotice/2`, `WriteProtection`, owner version, attempt/budget, preparation TTL/clock, and installation state. Current authorization/dependency continuity/install provenance determines only continuation of that same plan versus paused/conflict/recovery_unknown; there is no new prepare, requery/reselection, or resampling.

P/install outcome unknown is not a fourth re-execution branch. Original record, pins, Approval/Money/claim/outbox, stop/recovery/no-duplicate-effect obligations remain. Current files/equal hash/I/empty new control DB never guess success/failure, retry with a new `OperationId`, refund, or reset quota.

effectClass still tests:
- portable: real F/M portable change with `ChangeId/1` allocated at seal;
- control_only: P-only control, no content ChangeId/Frontier advance;
- no_op: all twelve D3 arrays empty and installation/save/publication not applicable;
- equal-byte external managed admission is not no_op.

Only after proving the original plan can never commit and every remnant/reservation/pin/external responsibility is safely resolved may it become terminal_failed + `identity_commit_aborted`. Capacity shortage, temporary revocation, missing consumer, and unknown install are not business rejection.

These correspond to acceptance scenarios 28–32, 43, and 55.

## 13. Server multi-user, Draft, and ordinary offline

Server supports multiple principals, sessions, and Drafts simultaneously; principal/audience comes from each request's protected context and is never borrowed across sessions. Every Draft retains its own Base/selection/input.

Acceptance:
1. different documents may prepare concurrently and finish when genuinely conflict-free.
2. same document/same Base: after A seals, B keeps Draft and enters stale/conflict/reprepare instead of overwriting.
3. B revoked before checkpoint cannot commit.
4. A revoked after seal has historical decision preserved; only later delivery is hidden.
5. restart/failover fences both P and file writer.
6. old instance is read/reject-only and never renames author files.
7. serialized backend writer does not erase multiple frontend Draft/session states.
8. two offline replicas moving/Trashing the same structure range retain concurrent heads and never LWW.
9. P/I schema has no whole-workspace current-body/full-AST mirror or second parent/order authority.

Server parallel prepare never weakens strong policy: every `managed_atomic` remains `frontierPolicy=exact`; only the original four `replica_local` local_structure modes may continue under the complete unrelated sealed-chain rule. Server checkpoint remains a strict strong consumer.

These correspond to acceptance scenarios 38, 39, 46, and 47. Realtime collaboration remains a future interface contract, never a claim of implementation/pass.

## 14. `semantic_pending` consumer gate and generation boundary

`semantic_pending` means only that D2-valid local facts passed while a cross-object/complete obligation explicitly allowed to remain pending is still unproved. It never satisfies:
- D2/typed invalid or deny;
- relation “no target” negative range;
- unique “Workspace-global unique”;
- complete Calendar expansion;
- collection/full membership postcondition;
- complete inbound/cross-object type closure;
- D7 complete Query/all_result, derived write set, or bulk;
- Automation/strong Action depending on complete absence/uniqueness;
- managed restore/purge/copy/fork/import;
- Server checkpoint;
- Approval/Money.

Ordinary source presentation, Draft, local edit, and explicit branch read may display pending only while exposing unproved obligations rather than saying “all valid.”

Generation wording is exact: fixed C already contains the prior D4/D5 candidate consumer work for the original G0-A/G0-B baseline. That is real candidate history, although unaccepted/unactivated. The new P1 production revision token, fourteen-key/range, CP3/ConflictRecord2, and M5 replay/recovery rules still require applicable D3/D4 P2, D5 P3, and D7 consumer afterimages. A missing strong consumer gates only the strong path that depends on it and never permanently disables qualified ordinary `.adoc`/Resource read, Draft, human whole-source save, or local offline operation.

These correspond to acceptance scenarios 37 and 45.

## 15. Budgets, pins, retention, and resource/unknown

The old Impact pin-retention, budget, and resource obligations remain and synchronize with the P1 `execution_resource` dependency:
- `DependencyKey/3.kind=execution_resource` freezes only the original operation's resource-policy revision, attempt allowance/consumption, cumulative work, protected pins/capacity, and pause category;
- it does not absorb D10 Money lineage, Run/Lease/Automation/deployment continuity, provider unknown, or `sourceOccurrenceKey`;
- last-reference pins for planned/unknown/conflict/Approval/Money/original publication/outbox never disappear because of preview TTL, I rebuild, readable current file, or new owner version;
- when a historical effect pin legitimately expires under an explicit contract, delivery returns that original `effects_unavailable` and never substitutes current file for historical after;
- while P/install is unknown there is no refund, duplicate charge, quota/approval reset, duplicate external effect, or reconstruction from an empty control DB;
- budget overflow during Definition Transfer two-pass materialization, complete range enumeration, CP3 publication, or recovery deterministically pauses/fails rather than silently expanding budget after partial success.

## 16. wire13 current, historical corpus, and terminology gate

The new corpus independently covers:
- wire13 new-decision accept; 0..11/unknown rejected for new decision;
- actual saved/planned/unknown v9/v10/v11 under original decoder/bytes/fingerprint/pins/custody/TTL/recovery;
- `DecisionKey/2`, `protocolOwner=D3`, same `OperationId` cross-domain independence, same-key fingerprint conflict;
- replica/server `CommitDomain/2`;
- `Frontier/2` exact and `scope_dependencies`;
- `ObservationScope/2` local_structure, workspace_constraints, prepared_workspace;
- closed `d3_identity_operation/13` owner binding + `WriteProtection=strict`;
- exact `InputDescriptor/3` plus pin/Observation/DependencyProof mismatch;
- `replica_local|managed_atomic` mode matrix;
- production `SourceVersion/2` versus local Observation domain;
- `SourceStamp/1`, `SourceRevisionPlan/1`, `RevisionTokenSource/2`, `RevisionTokenBinding/2`, and `d6_source_revision/2`;
- fourteen `DependencyKey/3` kinds and nine `StructureRange` variants;
- `ContentCompletionProof/3` versus historical `/2`;
- `ConflictRecord/2 + Frontier/2` versus historical `/1 + Frontier/1`;
- unchanged `ConflictKey/1`/`ConflictId`/`D6-ConflictKey/1` hash domain;
- effectClass portable/control_only/no_op + same-P `D3DecisionCompanion/2`;
- unchanged `D3-CJ/3`, Result/9 and Locator l1; fresh current Annotation uses Value/4 while historical Value/3 is retained only by its recorded decoder.

Historical corpus carries compatibility only for records proven to exist; decoder/fixture/draft/prose does not automatically make a prototype active. Legacy version numbers are never changed to 12 to claim new semantics passed, and a proven historical recovery contract is never deleted because no deployment record was found for some other prototype.

Terminology gate continues to verify:
- exact fixed-S 42 `conceptId` set;
- each exact `ownedNames` set;
- unchanged `firstFreeze`;
- D6 imported/internal technical names are not re-owned by D3;
- Preparation Binding/Definition Transfer/Definition Result Segment retain historical firstFreeze;
- a new technical field without real producer/owner mapping rejects or disables the dependent path;
- retired identifiers remain disjoint from owned sets;
- bilingual prose uses the same inline-code protocol name/version for the same concept.

These correspond to acceptance scenarios 42, 53, and 54.

## 17. Implementation impact graph and static/dynamic test entry points

Future implementation provides at least these independently fault-injectable/assertable entry points:
- closed wire decoder/encoder/fingerprint;
- common gate + saved/planned/unseen ledger router;
- InputDescriptor/pin/Observation/DependencyProof exact comparator;
- production/source-observer currentness verifier;
- H/SourceRevisionPlan/revision-token allocator-verifier;
- fourteen-key/nine-range enumeration and completeness verifier;
- local/exact Frontier-policy verifier;
- Trash restore/purge membership evaluator;
- copy/fork/import candidate-map and owner-matrix materializer;
- Definition Transfer typed-slot/Q parser/materializer/two-pass span verifier;
- ConflictRecord version router plus typed D3 owner-resolution adapter;
- strict installation/crash-recovery state machine;
- CP3 producer/receiver verifier;
- historical replay/no-duplicate-effect router;
- ordinary D6 observed-only qualification checker;
- Server Draft/base/currentness integration;
- consumer-version gates for D4/D5/D7/D8/D9/D10.

Implementation convenience never adds path identity, a global mutable current-body table, second parent/order owner, LWW, hidden ID reminting, second receipt/ledger, or generic owner-defined JSON.

This section specifies implementation slices and test entry points only and does not claim any module exists.

## 18. Downstream owner, read-only historical inputs, and activation gate

Every fixed-S snapshot, design input, catalog byte is read-only; P2 never modifies it under “synchronization.” P2 synchronizes only real owner afterimages plus the three routing metadata files.

Fixed C already contains the prior-generation D4/D5 candidate work for the original G0-A/G0-B baseline. Current P1 producer additions still require:
- D3/D4 P2: revision/currentness, fourteen-key/range, CP3/ConflictRecord2, M5 branching, and real D4 relation/calendar/registry/semantic_pending consumption;
- D5 P3: structured operations, revision-bound locator, complete range, and strict ordinary/structured boundary;
- D7: complete Query/Value-CEL/View/Narrow Field/Definition Transfer/Preview-Effects/Execution-Action/Prepared/Scenarios consumers;
- D8: complete current `SourceObservation/1`, Draft/IME/Undo/selection/editor, Server checkpoint;
- D9: scoped constructor/import/export, artifact pins, unknown recovery;
- D10: ApprovalUse/Money, Run/Lease/Automation/deployment, stop/unknown, `sourceOccurrenceKey` continuity, and the established eighteen documents/consumer responsibilities.

The coordinated D6 producer candidates now recognize native D3 descriptor/companion presence and permit historical sealed ChangeIds in InstallationNotice.baseFrontier while excluding this unsealed decision's new ChangeId. Verify these exact producer/consumer bytes together in fresh joint review; their presence is not activation.

This Impact opens neither consumer nor producer and never turns a missing gate into a global ordinary/local ban.

The existing 3 P1 + 8 P2 findings, 11 OPEN total, plus U6/U7 remain gated. Complete consumer coordination is followed by fresh independent full joint acceptance and repair recheck. The A2 self-contained D1-D10 reconstruction has already been authorized by the human and needs no further approval request, but it may execute only after that complete consumer coordination, fresh independent joint acceptance, and repair recheck; this Impact artifact does not start or pre-execute A2. A separate fresh ordinary Chat Pro global final review still follows A2.

## 19. Acceptance conclusion boundary and minimal-counterexample coverage

This file is the sole-candidate author's Impact afterimage, not independent review, implementation evidence, or product acceptance. Every future test binds a fixed commit, real backend/platform, actual output artifacts, and reproducible crash/replay/unknown evidence. Documentation CI, static prose, fixture names, and “tests designed” are never implementation PASS.

In addition to the per-section acceptance scenarios already assigned, all original Impact and D3-main counterexamples have explicit executable oracles and never success-only coverage:
- split arrival of source and portable metadata makes the missing side incomplete/placeholder, never delete/fresh identity;
- create/move + remote edit composes only when causality and all real dependencies are proved;
- Trash/edit has no delete-wins/edit-wins default;
- equal hash + different `FileObjectBinding`/provenance never proves original-plan installation;
- duplicate local UUID claim resolves only through explicit conflict + fresh-copy losing branch, never silent rekey;
- D2-invalid external bytes use repair/read only and never produce successful D3 author receipt;
- current revocation before seal prevents commit; after seal it hides delivery only;
- same `OperationId` across different complete `CommitDomain/2` values may be independent, while different fingerprint at the same key conflicts;
- canonical request never permanently embeds complete body/source/resource bytes and purpose-specific pins compare item-for-item.

The full set of 55 main-text minimal counterexamples remains traceable in the executable test inventory: 1–7/26/53/54 map to §3; 8–9/22/45 to §6/§9; 10–12 to §8; 13–21 to §6.2; 23–27/44/50 to §11; 28–32/43/55 to §12; 33–37 to §5/§14; 38–39 to §13; 40–42 to §4/§16; and 46–52 to §7/§10/this section. Numbering provides traceability and never substitutes a vague “all main counterexamples covered” statement for real fixtures/oracles.

The old D10 3 P1 + 8 P2 findings, 11 OPEN total, remain open. The coordinated D6 repairs awaiting joint acceptance, D4 P2, D5 P3, full D7, D8, D9, D10 eighteen-document set, fresh joint independent acceptance, repair recheck, the already-authorized but not-yet-executed A2, and the later separate fresh ordinary Chat Pro global final review all remain gated.

This artifact runs no product tests, implementation, CI acceptance, merge, activation, release, or deployment and authorizes no document/GitHub/ref mutation.
## 20. Closed-interface repair verification

Main §19 cases 56–62 add these concrete oracles to the existing 55-case inventory; they are future verification obligations, not executed fixtures:

| Cases | Inputs and asserted result | Required zero-effect/privacy checks |
|---|---|---|
| 56 | Exact native wire13 create_node without any prepare token; then the same closed object with an extra planToken | First reaches the original single planning/seal path; second fails static shape. No implicit D6 prepare, second ledger or blanket local ban |
| 57 | Genuine inactive target W/B issued/custodied by A, create and fork; tampered proposal; planned crash; saved rejection/commit | Key always W/B; stage3 issuer/source then P1/P2 then TL; invalid P1/P2 means zero custody/target-ledger read. No target policy/active/source_write requirement or repeated initialization |
| 58–59 | Every D3ResolverInput/12 arm and closed outcome; exact F, unrelated F+1, wrong binding, inaccessible branch, anchor ambiguity | Compare exact required/forbidden fields; F+1 is workspace_unavailable, hidden states not_visible; no preauthorization target/head/foreign-route read or branch/reason/source leak. Hold/revalidate same cut through output |
| 60–61 | Transfer keys W/O with different domains; each source state, failed target, sequence 0/MAX, old wire11 bytes | Validate full owner-qualified keys, conditional member presence, monotonic immutable record and terminal rules; unknown stays pending, no domain guessing/joint commit/automatic source retry |
| 62 | All 42 concept definitions, owner qualifiers and migration/deletion prose in both languages | Enforce same-decision sole OriginBinding, explicit retired Adopt, exact template/tasks/task exclusion, and immutable controlled JSON/firstFreeze; do not substitute name equality for semantic comparison |

The implementation resolver follows main §11 exactly: static binding comparison is not a preauthorization mismatch oracle, and both domain continuity and exact Frontier equality precede target-state reads. The transfer summary is a coordinator observation of two existing independent decisions, never a new mutation or rollback protocol. Original historical records retain their original decoders and obligations.

### 20.1 Conflict preparation and single native decision

Cases 63–69 require the actual D3 prepare, current D7 /3 binding/mapping and full preview producer together. Validate all three fresh-copy modes positively with exact historical branch pins and complete native closure; then remove one required pin, add an unplanned source rewrite, strip/swap binding, alter a head/key/selection, and change audience. Assert exact error family/order, zero unauthorized reads, zero extra reservations and no second submit. Ordinary raw wire13 remains independently reachable.

For the preview, independently reconstruct the entire item set and exact byte slots from frozen before/proposed/branch/native/control evidence, then fetch every page and byte range to terminal. A choice/hash-only pin, hidden item, missing head source, wrong domain/header, mixed delivery epoch or premature terminal must fail; no source/identity is allocated while preparing. D7 owns the current manifest/EffectBytes versions, two closed conflict items and record producer; the new path remains gated until those complete owner afterimages and joint review exist. This is an integration requirement, not a permanent unspecified adapter.

Inject head/right changes before stage14 and CAS, lost planning response, each install boundary, seal-before-delivery, expired preview, damaged/missing input guard and unknown installation. Unseen uses original operation_precondition_failed/availability ordering; saved/planned first use original request/record/pins/candidate under current original-profile disclosure. Verify single native fingerprint/DecisionKey/CAS/P seal, one original D3 receipt and same-P companion; no new submit envelope or D6 identity arm. Test real /1,/2 historical records with their original decoders rather than synthesizing upgraded records.

Test open-record equal-bytes resolution as portable, proved already-resolved placement/lifecycle as true no-op, and every fresh-copy as identity creation. A no-op neither creates inverse lifecycle transitions nor changes raw mode admission. The selected canonical result may not conceal a physical source rewrite absent from the full composed resolution plan. These assertions are designed oracles; no fixture/product execution is claimed.

### 20.2 Lifecycle admission and two-direction canonical materialization

Cases 70–73 close the design coverage for IR-06/07: the four selected lifecycle→requested-result rows crossed with actual installed live/Trash, native location/owner/reply closure and independent restore membership; placement across current complete sibling ranges; and canonical A/copy B versus canonical B/copy A for Node/Resource/Annotation. For Resource installed a but retaining B=b, assert old R=b plus fresh R′=a under one native decision, with separately bound physical before and selected source, a /2 canonical version plan and /1 fresh plan, exact full preview/receipt/CP3, and no invented Resource source RPC. Equal bytes with changed canonical claim/version increments H once; metadata-only unchanged source does not; exact already-resolved no-op has no ChangeId. Check raw mode matrix and ordinary current Observation rejection unchanged. Private wrapper/guard/DependencyProof source qualification is resolution-only, never transferable to ordinary Query/D8/write. Verify omission/net structural receipt rules, one final source per entity, authorization before head/file reads, stale head/CAS, install unknown, restart and saved replay. These additions specify execution oracles and claim no product run or independent PASS.

### 20.3 Canonical evidence and public completeness

Main cases 74–79 require independent positive/negative oracles for the owner-N Annotation reply with fresh copy under M, Node X→Y with shared actual fromSource across distinct components, original E/delete/result-only/S/lifecycle-only grammar, full D4/D7 typed gates, exact overlap equality, mandatory public canonical plan/bytes/extension and single-seal recovery. Reconstruct both component evidence sets independently, enforce original comparator uniqueness separately, and reconstruct one deduplicated physical source/control set against CP3. The original twelve receipt arrays cover only native effects; D3CanonicalEffects/1 is mandatory complete public evidence for canonical effects, including the empty resolved-no-op extension. Missing extension after seal is delivery unavailability, never absence of the saved commit. Test current BudgetBinding/1 exact fields and zero semantics. These are design verification obligations, not executed product tests or independent acceptance.


## 21. A2 implementation/test status

Every source-qualified scenario, crash/fault, runtime, backend, platform, large-workspace, UI and concurrency condition in this companion is an **unexecuted design obligation**. This batch executes only documentation/input structural checks. Schema readability, complete counts, a documentation CI job, or author-candidate presence is never product implementation evidence. D4-D10 full modules remain later A2 TODO; this file records only the D3-direct producer/consumer coordination actually read in this batch.
