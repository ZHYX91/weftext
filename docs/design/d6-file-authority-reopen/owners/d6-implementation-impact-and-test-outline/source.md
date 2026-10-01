---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: `3762120f-fb70-4cf7-9699-68602e9bf9fc`.

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented, and not merged. Historical revision05/D7/D9 acceptance or candidate prose in fixed S is provenance only and does not govern this afterimage; the stable document ID is preserved. The private sole-author P1 Storage, Control, Lexicon, and machine Registry afterimages now actually exist, and this file synchronizes future implementation and acceptance obligations to their final supplied text. Those producers, Registry, and this Impact/Test Outline are still not independent acceptance or activation evidence. PROPOSAL/replacements routing, P2 D3/D4, P3 D5, actual D7–D10 consumers, the eleven existing OPEN findings, U6/U7, fresh full joint acceptance, A2 self-contained reconstruction, and the separate fresh Pro global review after A2 all remain gated. This file defines future implementation/conformance evidence only; it authorizes no code, dependency, source, CI, release, deployment, or A2 work. Fixtures, performance entries, platform fault injection, and documentation checks are requirements until actually run, never passed evidence by being named here.

# D6 Implementation Impact and Test Outline

## 1. Implementation slices and unique ownership

D6-FA-r01 implementation must keep logical owners separate and must never re-collapse them into “one database stores everything”.

| Slice | Responsibility | Explicitly forbidden |
|---|---|---|
| F1 portable file backend | current .adoc/Resource bytes, FileBinding, safe install primitive, external-change observation | replacing user files from P/I body copies; calling hash+rename CAS |
| F2 portable metadata | identity, child order, lifecycle/Trash, Annotation, shared policy/trust, Change/Frontier/conflict records | two independently writable parent/order representations; path/title as Ref |
| P durable control | SQLite decision/recovery/unknown/approval/claim/Money, PreparedIntent, pins, execution responsibility | whole-workspace current-body read fallback; sync-provider merge |
| I derived index | discardable inventory/parser/search/OCR SQLite with layered coverage/checkpoints | identity/policy/receipt ownership; partial index as completeness proof |
| D Draft/session | Draft/input/selection/IME and local/Server transient collaboration state | author revision or commit evidence |
| C coordinated consumers | new D3/D4/D5/D7/D8/D9/D10 versions | partial activation or generic D6 bypass of original owners |

Implementation requires a static ownership audit: each current-truth field has exactly one logical write owner. Compatibility dual-write, silent startup migration, legacy authority.sqlite body mirrors, and bidirectional current-state sync between portable metadata and P are non-conforming.

## 2. Staged implementation order

### S1 Portable reader + no-write open

- Open a file-backed Workspace, read the portable-metadata entry point, and validate containment/closed records.
- Read current Document/Resource bytes from ordinary files. A missing index must not prevent opening a named file.
- External invalid bytes remain visible in Source/repair inventory and are never silently replacement-decoded then saved.
- Do not allow managed v2 commits until S2-S5 are satisfied.

### S2 Durable control + legacy replay

- Create off-workspace control.sqlite3, WAL/SHM, and a private pin area, persisting canonical request/fingerprint, DecisionKey, planned/committed/unknown recovery, budget/attempt state, installation state, necessary pins, ApprovalUse/claim/Money/external unknown, and execution responsibility. P is never a second owner of current body or portable structure.
- Fully support the actual historical D3 v9/v10/v11 records, D6 wire1 commit/receipt/error, Policy/1/2, SourceVersion/1, Frontier/1, InstallationNotice/1, ContentCompletionProof/1 and the already-existing ContentCompletionProof/2, ConflictRecord/1, legacy revision-token profiles, D3 primary receipt/companion, D7 PreparedActionBinding/1,/2, D8 PreparedEditBinding/1, and actual historical Result/ByteHandle records. Dispatch by the version under which the record was really saved, preserving original bytes/fingerprint, authorization, pins, clock/continuity, and recovery; never auto-reencode or upgrade.
- saved, planned, and unknown are distinct recovery branches. After common disclosure, CommitDomain/P continuity, and original request/fingerprint lookup, saved performs current delivery authorization only for the original actual effect/mode or result-disclosure scope and then replays original receipt/error/effects bytes or resumes the original-version publication/outbox; it never requires the old source/Frontier/business dependency to become current, writes over a later current source, charges again, or reallocates versions. planned resumes only the original frozen plan, InputDescriptor, applicable SourceRevisionPlan, pins, OperationId, budget/attempt, installation state, and original version basis; before seal it has no ChangeId for this decision and never new-prepares a second success. unknown retains original pins, external-effect evidence, charge/approval/claim, and execution-continuity responsibility and is never guessed from current files, equal hash, I, or reauthorization.
- New v2 fixtures may cover SourceRevisionPlan/1, RevisionTokenBinding/2, d6_source_revision/2, ContentCompletionProof/3, ConflictRecord/2, and DependencyKey/2, but remain behind not_in_release/unsupported_version or the applicable owner gate. A fixture does not prove that every historical prototype was active or that a new producer has been accepted by its consumers.
- P-loss tests prove that old receipts, ApprovalUse, Money, unknown state, clock/pin continuity, or an old production-domain H(D,E) cannot be synthesized from portable/current files, Derived Index, equal digest, or an empty control DB. When portable current state is fully verifiable and the affected range has no unresolved installation risk, replica registration may create a new ReplicaEpoch/CommitDomain for ordinary content; historical responsibility in the lost domain is still not reconstructed.

### S3 Portable metadata + replica model

- Implement ReplicaRecord, CommitDomain/2, ChangeId/1, Frontier/2, complete managed/external SourceVersion/2, SourceObservation/1, SourceVersionRef/1, internal SourceRevisionPlan/1, SourceStamp/1, RevisionTokenBinding/2 with d6_source_revision/2, InstallationNotice/2, ContentCompletionProof/3, and current ConflictRecord/2. Historical Frontier/1, ContentCompletionProof/1,/2, ConflictRecord/1, and legacy token profiles keep their original decoders/bytes.
- SourceVersion/2 production CommitDomain and production observationEpoch are distinct from SourceObservation/1 observerDomain and current observationEpoch. For production domain D and entity E, H(D,E) is the greatest revision in the continuous sealed managed history. H=0 is legal only for a proved complete empty history from domain birth/registration, P continuity, and verified portable sealed history. A real managed after uses checked H+1, MAX never wraps, and a production observationEpoch change does not reset H. Cross-domain writes and returns continue each domain's own H; equal bare revisions across domains are incomparable.
- A true raw no-op preserves the original SourceVersion/2. Pure placement/lifecycle/control with unchanged source does not advance H. Source deletion has after=absent and creates no deleted SourceVersion or H increment, while the real portable effect still receives its ChangeId at seal. Equal-byte external admission is still explicit external→managed admission and creates the managed after from the current production domain's H+1; externalSequence never substitutes for managed sourceRevision.
- Only an original plan that will actually produce a managed after freezes SourceRevisionPlan/1: actual before Observation or explicit absent, same-production-domain lastIssued/complete-empty-history proof, proposed SourceStamp, and exact after pin. Those version bases are immutable after the winning plan and same-domain history is revalidated before seal. plan/staging/InstallationNotice allocate no ChangeId for this decision; the single P seal combines SourceStamp with the real ChangeId into managed SourceVersion/2 and atomically advances H.
- RevisionTokenBinding/2 uses the protected d6_source_revision/2 tag to bind current observerDomain/current observationEpoch to managed SourceStamp or complete external SourceVersion. A proposed managed token is only original-plan candidate/locator validation evidence until seal and a complete current SourceObservation establish currentness. A watcher gap, external/object replacement, or discontinuous rematerialization invalidates old observation/token qualification even when production SourceVersion, revision, hash, or text is equal. D3 Locator outer opaque revision-token lexical form, D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence, and D5 revision-bound locator wire retain their original shapes.
- Frontier/2 represents only verified continuous sealed causal prefixes per CommitDomain. It proves neither payload materialization, placeholder download, Registry/index completeness, nor D7 complete Query. Remote admission does not replay remote OperationId and does not mint a local ChangeId merely for transport.
- New-FA transport uses ContentCompletionProof/3 carrying the real production SourceVersion before/after or absent, actual frontierBefore/frontierAfter, and actual components. A receiver advances Frontier only after validating InstallationNotice, proof, every component, production versions, and the complete continuous sealed chain, then establishes a new SourceObservation/SourceVersionRef from its own CommitDomain, FileObjectBinding, observationEpoch, evidence pins, and current control/Registry/incidence cut. It never reuses the sender sourceToken.
- New conflicts use ConflictRecord/2+Frontier/2. ConflictKey/1, ConflictId, and the D6-ConflictKey/1 hash domain do not change; historical Record/1 remains Frontier/1. child order still has one ordered-list owner. New devices register a new ReplicaEpoch and retired epochs never revive.

### S4 Safe file installation

Strong paths implement and prove conditional_replace or a truly exclusive_write_window. Advisory locking, read-hash-then-rename, or equal final digest is never a substitute for strict CAS/exclusion. Expected-absent new files still use create_only.

observed_replace/WriteProtection=observed_only is legal only when a trusted human interactive_source_save explicitly selects and freezes it before planning starts and every qualification holds: exactly one existing live Document; ordinary+replica_local whole-source save; complete source read/replace; author-source write set empty or limited to that Document; no applicable body/Field/node-control deny; no identity, parent/order, lifecycle, shared-policy, Registry, Calendar-scope, or other-entity mutation; Draft Base equals the selected current SourceObservation. noninteractive, managed_atomic, D3 identity/structure/lifecycle, D5 structured cell/row/column/reorder, bulk/collection, D7 strong Action, Automation, server checkpoint, Approval, and Money remain strict. After planning starts, strict-capability failure, known conflict, revocation, durability failure, strong-obligation failure, or another missing qualification never falls back to observed_only.

The observed_only original plan durably retains the actually read before B and user input N. The sole relaxation is that an external C never observed after the final trusted check and before N installation may be overwritten by N and may have no recoverable copy. A later C may replace current file again, but the durable B/N obligation remains. Observed competition, stale Base, watcher gap, third_state, or revocation is outside the relaxation. Unknown installation/provenance remains recovery_unknown and equal hash/text never guesses success.

Fault injection covers:
- before staged write;
- before/after staged-data flush;
- before/after durable planning of SourceRevisionPlan/version basis;
- before/after InstallationNotice/2 flush, verifying that it carries no new ChangeId for this unsealed decision;
- before/after every component install;
- file-data flush;
- directory-entry flush;
- installed verification;
- before/during/after the P seal transaction, verifying that seal is the sole decision commit point;
- before/after ContentCompletionProof/3 and outbox write/flush;
- lost response/receipt delivery.

Every strict-path point proves that observed competing bytes are never silently lost. observed_only proves it never overwrites an observed competitor, B/N is durable, unseen C is never fabricated, and unknown never becomes guessed success. prepare/retained is not Saved. Only strict install+P seal yields reliable; only fully qualified observed_only install+P seal yields durable_observed_only. Once seal succeeds, publication/delivery failure resumes only the same proof/outbox or original receipt delivery and never reinstalls N, changes OperationId, reallocates ChangeId/managed revision, advances H again, or charges again.

### S5 Ordinary save and semantic pending

- D6 source-save keeps ordinary and complete semantic qualification separate and orthogonal to strict|observed_only WriteProtection. Ordinary success never grants complete Query/Action qualification, and failure of a strong consumer never turns the same Action into an ordinary success.
- An ordinary existing-Document whole-source save binds the complete current SourceObservation/1, including the real production SourceVersion/2, FileObjectBinding/evidence pins, current lifecycle/policy, actual MutationFootprint, D2 parse, and every actually modified local typed fact. A valid source with an unproved cross-object/complete-range obligation may only become semantic_pending. semantic_pending can serve an explicitly local Source/edit/read consumer under its own contract, but not relation/unique/Calendar strong mutation, all_result/post-query, Automation, purge, or another complete consumer.
- DependencyProof/2 consumes exactly fourteen closed DependencyKey/2 kinds: source, lifecycle, placement_range, ref_inbound, relation_incidence, calendar_scope, registry, temporal_rules, authorization, foreign_binding, query_scan, replica_registry, conflict_record, execution_resource. Each key's owner, range, epoch/revision stamp, evidence pins, positive/negative set, and disclosure follow final Control. Free JSON, I rows, provider “synced”, final hash, or Frontier numbers never substitute for a real complete proof.
- Complete range proof comes from a currently authorized consistent snapshot/range barrier or a gap-free continuous change chain plus final revalidation. Empty and non-empty ranges use the same standard. Partial/building index, index miss, placeholder, unknown decoder, I/O failure, missing shard, or hidden unauthorized data never proves empty. Deleting/rebuilding I alone does not destroy range epoch/revision and continuity facts still complete in their real P/M owners; actual correctness-fact loss or a gap invalidates the old proof and requires a new epoch plus complete enumeration.
- frontierPolicy=exact retains complete Frontier equality and D3 managed_atomic stays exact. scope_dependencies admits only a real continuous verified-sealed non-regressing extension from the original expectedFrontier that is proved unrelated to every original source/control/authorization and positive/negative DependencyKey. The original canonical request, InputDescriptor.expectedFrontier, DependencyProof.baseFrontier, targets, Query/selector, pins, proposed bytes, WriteProtection, owner input, and version basis are never re-signed or resampled. A real bound-dependency change or unknown gap is never relabeled unrelated. Conversely, a qualified ordinary/local/replica_local operation depending only on complete real local evidence is not permanently blocked by an unrelated whole-Workspace Query/index gate.
- Current Observation and revision-token continuity are checked independently from production SourceVersion. Equal production version/revision/hash/text never restores SourceObservation/SourceVersionRef/d6_source_revision/2 currentness across watcher gaps, replacement, or rematerialization.
- Submit/recovery first passes common closed decode, state disclosure, CommitDomain/fence/P continuity, and DecisionKey/request-fingerprint anti-probe checks, then splits saved/planned/unseen. saved performs current delivery authorization for the original actual-effect/result-disclosure scope and replays original bytes. planned resumes original plan, SourceRevisionPlan, pins, budget, installation state, and version basis and has no pre-seal ChangeId. Only unseen creates a new business decision from current owner/Observation/DependencyProof. Current r6 proof never retrospectively rejects or upgrades an r5 saved decision.
- Written components are verified against original planned after and unwritten dependencies against original before/cut. Core's own installation does not self-conflict. Late competition, revocation, unknown provenance, and third_state remain conflict/paused/recovery_unknown rather than a new business rejection.
- Fixed C already contains D3 wire12 and D4/D5 candidate consumption of the earlier A/B/C SourceObservation/Frontier/WriteProtection boundaries. The new P1 production-domain H/SourceRevisionPlan/revision-token profile/DependencyKey refinement/ContentCompletionProof/3/ConflictRecord/2/U5 recovery rules still require real P2 D3/D4 and P3 D5 afterimages plus fresh joint acceptance. Existing candidates never imply automatic support for these producers, while ordinary file read, Draft, qualified human whole-source save, and local offline work that does not depend on a missing strong consumer remains available.

### S6 Sync/conflict

- Continue explicit two-offline-replica cases for source/source, create/create, move/edit, move/move, Trash/edit, policy conflict, partial transport, placeholder, and identity collision. Ordinary multi-device/offline content capability is never globally frozen merely because complete-Action consumers are not coordinated.
- A new-FA portable change is complete only when InstallationNotice/2, ContentCompletionProof/3, every listed component byte/metadata item, the real production SourceVersion before/after, and the complete continuous sealed-record chain from notice.baseFrontier through actual frontierBefore to frontierAfter all validate together. Missing pieces, an unmaterialized placeholder, incomplete production-version metadata, or a chain hole is incomplete: no identity minting, Frontier advance, completion from I, or provider/index-status shortcut.
- After validating production history, the receiver creates its own current SourceObservation/SourceVersionRef. It never copies the sender sourceToken and never skips local FileObjectBinding, observer observationEpoch, pins, or continuity because production revision/hash/text matches.
- Concurrent portable heads use ConflictRecord/2+Frontier/2 while ConflictKey/1, ConflictId, and the D6-ConflictKey/1 hash domain remain unchanged. Historical ConflictRecord/1+Frontier/1 keeps original bytes/decoder. Unsealed external competition, third_state, or unknown install has no real ChangeId and never fabricates a head.
- conflictId stays stable. When a new sealed head changes the complete key, the old open/resolution_prepared record is superseded under its version rules and a successor is created. No mtime/LWW winner is chosen. Source merge preserves original bytes and policy conflict never falls back to allow-union.
- Fixed C already has a D3 wire12 candidate, but D3-owned placement/lifecycle/identity resolution still requires the P2 native-descriptor/companion consumer and real DependencyKey/SourceRevisionPlan recovery coordination. Until that gate and fresh joint acceptance, the affected new resolution prepare returns owner_update_required. D6 never bypasses D3 with a generic payload and the specification never claims that wire12 is absent.
- purge remains strong: complete inbound/owner closure, complete semantic proof, and real admission of the purge Frontier by each participating registered replica or explicit retirement are required. Provider “synced”, replica count, or one Frontier row never substitutes for acknowledgement.

### S7 Server multi-user

- concurrent multi-user reads/prepares/Drafts;
- different-document computation in parallel with only necessary range locks and brief P-seal serialization;
- two Drafts on the same document never overwrite each other;
- revocation/seal race;
- Server failover fences both P and author-file paths;
- a second Server instance cannot bypass P coordination and rename hosted files directly.

Real-time collaboration remains not_in_release until a later OT/CRDT adapter and D1 release evidence. Candidate conformance still models receive/broadcast/checkpoint as distinct states.

### S8 Index/large workspace

- delete/rebuild index; layered coverage; independent parser/search/OCR invalidation;
- 10k / 100k / 1M small files;
- attachment-heavy and body-heavy tens-of-GB workspaces;
- cold and warm filesystem cache;
- named CPU/RAM/OS/filesystem and SSD/HDD or named slow storage;
- sync placeholders/on-demand download;
- restart/resume, bulk external change, watcher gap;
- exact/NFC/regex candidate reread and scan fallback for incomplete recall;
- no-body-replica audit.

## 3. Five mandatory time metrics

Every large-workspace report separately records:

- T_first_open — workspace entry/active navigation becomes responsive;
- T_first_edit — active Document Draft accepts input;
- T_first_reliable_save — counts only strict safe install + P seal; observed_only durable_observed_only is recorded separately and does not populate the old strict D1 metric before D1 update;
- T_full_search_ready — complete coverage exists for the named search profile/range;
- T_OCR_ready — named attachment/OCR profile completes or explicitly fails.

A single “startup time” is insufficient. T_first_edit is not author save; T_first_reliable_save is not portable published; T_full_search_ready is not OCR ready.

Also record peak RSS, index DB size, P DB size, protected-pin bytes, bytes read, file count, parse throughput, P-seal latency, portable-publication latency, resume work after restart, and single-file/bulk incremental cost.

No actual data means no claim of named-competitor speed claims, “seconds to rebuild”, or any named performance pass.

## 4. no-body-replica acceptance

After exercising a real candidate package, inspect every:

- durable-control SQLite table/index/FTS shadow;
- derived-index table/FTS shadow;
- temp/staging/pin area;
- portable metadata;
- log/audit store;
- crash-recovery file.

Assert:

1. I has no complete-current-Document body column or serialized whole-workspace full AST;
2. P has no whole-workspace current-source mirror usable as a read fallback;
3. every complete-source pin is traceable to one explicit owner/purpose/retention/capacity account;
4. releasable source pins can be physically reclaimed without damaging canonical decision/receipt;
5. legacy fixtures whose old contract promised decision-lifetime pins keep them;
6. an externally changed F file is never overwritten automatically from stale P/I bytes.

Do not infer absence of sensitive content from a table name such as “contentless”; tokens/grams/positions are also audited for disclosure and capacity.

## 5. Positive/negative acceptance matrix

FA01–FA30 identities remain unchanged. Every row below is a future implementation evidence obligation, not a test run or pass in this author candidate. Every case also obeys current disclosure, CommitDomain/P continuity, real owner boundaries, and historical recovery obligations.

| ID | Scenario | Required result |
|---|---|---|
| FA01 | I deleted; 100k unrelated docs unparsed; edit one D2-valid note | Rebuild only the current SourceObservation, local DependencyProof, and installation qualification actually needed by the ordinary save. With a qualified strict primitive, T_first_reliable_save can complete without unrelated index/OCR or whole-Workspace Query proof; complete Action still waits for its own complete proof |
| FA02 | existing target lacks a strict conditional/exclusive primitive | strict returns install_unavailable. observed_only is available only when a trusted human selected/froze it before planning and all existing-single-live-Document ordinary+replica_local, complete-read/replace, no-applicable-deny, no-structure/other-entity-mutation, Draft-Base=current-Observation qualifications hold; durable_observed_only appears only after durable install+P seal |
| FA03 | third party writes B after Base A | if competing B is observed before install, strict and observed_only both stop and retain the competing current state plus original read-before/input. observed_only accepts only a C still unseen after the final trusted check being overwritten by N; unknown installation remains recovery_unknown |
| FA04 | external A→B→A plus watcher gap | current observer observationEpoch changes; old SourceObservation/1, SourceVersionRef/1, d6_source_revision/2 binding, map/locator/prepared/evidence all invalidate even if production SourceVersion, revision, hash, or final text is equal |
| FA05 | after install succeeds but implementation compares written target to before | the test catches the bug. Conforming code validates original planned after plus installation provenance for written components and revalidates only unwritten dependencies against original before/cut under exact or qualified scope_dependencies |
| FA06 | P seal succeeds; ContentCompletionProof/3 write fails | decision, real ChangeId, applicable managed SourceVersion/H, receipt/charge, and ReliableSaveState are already fixed: strict is reliable and qualified observed_only is durable_observed_only. publication is pending; retry only emits the same proof/outbox with no reinstall, recharge, or second H/ChangeId advance |
| FA07 | lost response; r5 committed; later r6 edit | after common continuity and original request/fingerprint lookup, authorize delivery only for r5's original actual-effect scope and return original receipt bytes. r5 source/Frontier/business dependency need not equal r6; current r6 is not rewritten and no charge repeats. Revocation may hide delivery only |
| FA08 | two offline devices edit same note differently | two real sealed heads form source_concurrent and current new records use ConflictRecord/2+Frontier/2. No LWW; original branch bytes/pins remain available. Unsealed external competition never gets a fabricated head |
| FA09 | A moves Node; B edits its source | combine only when placement/source dimensions are independently compatible, all relevant placement_range/source/authorization proofs are complete, and destination policy/structure revalidates; otherwise conflict. Equal SourceVersion revision never proves structure unchanged |
| FA10 | A Trash; B edit | no implicit delete-wins. Preserve edit branch and Trash intent with lifecycle/placement/source dependencies proved separately; an ambiguous ordinary read never picks a random current branch |
| FA11 | purge vs old-replica restore | tombstone never revives and old bytes are fresh-copy/reconciliation input only. purge additionally requires complete inbound/owner closure and each participating replica's purge-Frontier acknowledgement or retirement |
| FA12 | child_list/Document/ContentCompletionProof arrive in parts | advance Frontier only after InstallationNotice/2, ContentCompletionProof/3, every listed component, production SourceVersion before/after, and the complete continuous sealed chain mutually validate. Anything missing is incomplete; no identity mint, no provider/Frontier/hash shortcut, and receiver builds its own Observation rather than copying sender token |
| FA13 | placeholder has metadata but bytes not downloaded | not_materialized/source_unavailable, never empty/not_found. An old locator/I/cache never substitutes current source and no complete source/query_scan proof is created |
| FA14 | duplicate Ref with different birth | identity_collision; resolver never chooses first. D3 remains the resolution owner; fixed-C wire12 candidate never substitutes for the unfinished P2 native-descriptor/companion consumer |
| FA15 | building index misses one relation target | relation_incidence/query_scan complete consumer returns proof_unavailable, rejects complete Query/Action, or performs the required full real-source scan. Index miss, partial rows, or repeated hash never proves an empty range |
| FA16 | semantic_pending source enters automatic Action | complete consumer rejects. Source/explicit local read/edit remains available under its own authorization and real local evidence; unavailable unrelated whole-Workspace proof does not permanently disable it |
| FA17 | P lost, portable files intact | old production-domain H, planned/saved/unknown state, approval/Money/external responsibility are never guessed from files/I. A range with fully verifiable portable current state and no unresolved install risk may register a new ReplicaEpoch/CommitDomain for ordinary content; that new domain starts only from its own proved-empty H |
| FA18 | I lost, P intact | when real P/M range epoch/revision, pins, and continuous-change facts remain intact, rebuild only I cache: no proof re-sign, epoch change, decision replay, Ref mint, or charge change. If correctness facts are actually missing, old proof invalidates and current-authorized full enumeration establishes a new epoch; unrelated ordinary local work remains available |
| FA19 | copied P database opened on two devices | never creates two execution holders. takeover requires complete continuity and fencing; ordinary replica registration grants no Approval/Money/external consumption authority |
| FA20 | single-use approval, network result unknown, crash | restart first restores original request, provider binding, pins, Money/approval/claim/unknown continuity. No new OperationId, second consume, or guessed provider outcome from current files/equal hash |
| FA21 | work batch charged then crash | charge remains, attempt/restart does not refund, and absence of P/summary never resets quota. Unproved continuity/clock remains planned/paused/unknown rather than fabricated terminal success/failure |
| FA22 | Server Alice/Bob edit different Documents | both Drafts/prepares run concurrently and each commits once. Only real dependency-range locks and brief P-seal serialization apply; a proved-unrelated coarse sequence/Frontier advance never becomes whole-Workspace UI exclusion |
| FA23 | Server Alice/Bob same Document, same old base | first seal succeeds; second retains Draft and becomes stale/conflict without overwriting. Equal text or revision number never bypasses current Observation/FileObjectBinding continuity |
| FA24 | Bob revocation races collaboration checkpoint seal | one linearization point. If revocation wins, Bob's uncommitted contribution cannot seal under Alice. If seal already won, historical commit is immutable and only current delivery authorization can hide output |
| FA25 | Server failover fences DB but not file writes | conformance failure. The old instance must lose both P and author-file rename/write capability; SQLite fencing alone is insufficient |
| FA26 | collaboration receive ack without checkpoint | UI/API cannot report reliable, durable_observed_only, or committed. receive/broadcast, Draft persistence, and input retained are not Saved |
| FA27 | IME preedit events then final input | preedit never becomes author op. Final confirmation creates at most one Draft/checkpoint input transaction; any later checkpoint still passes current authorization, Observation, DependencyProof, and seal |
| FA28 | protected conflict pins hit capacity | restrict new prepare/install or pause the planned operation. Never delete last-reference conflict/recovery evidence and guess it back from I |
| FA29 | new historical effect pin legitimately expired | return effects_unavailable under the new retention contract. Current file, new Observation, or equal digest never impersonates old after. Legacy decision-lifetime pin promises are not retroactively deleted |
| FA30 | legacy wire1 saved decision | byte-equivalent original decoder/replay and original pin/authorization/continuity promises survive. SourceVersion/2, d6_source_revision/2, ContentCompletionProof/3, ConflictRecord/2, or a current stronger proof never re-encodes/upgrades the old record |

## 6. Permission/non-disclosure tests

At minimum:

- caller knows ConflictId but lacks state disclosure -> not_visible without record existence disclosure;
- replica_register without source_write -> may register but cannot edit;
- conflict_resolve without actual source/policy/D3 write -> original write gate fails;
- execution_custody_admin grants no Money enlargement/new approval/source read;
- phone-only Field edit gains no body/name from ordinary source profile;
- hidden sibling count/order is not read before structure-observation qualification;
- detailed external_invalid bytes/parse are exposed only through source/repair permission;
- revocation hides replay delivery but does not mutate the saved decision.

## 7. Legacy/version matrix

Implementation tests dispatch by the version and owner under which the record was actually saved and retain all of:

- D3 v9/v10/v11 historical saved requests/receipts/errors and actual old D3 primary receipt/companion;
- D6 wire1 commit/receipt/error and legacy PreparedIntent/Token tags;
- Policy/1/2;
- SourceVersion/1 plus actual historical d6d/d6r/d6a revision-token profiles and opaque DocumentRevision/ResourceRevision/AnnotationRevision decoders;
- Frontier/1 and InstallationNotice/1;
- ContentCompletionProof/1 and the already-existing ContentCompletionProof/2, whose sourceChanges remains SourceVersionRef/1|absent and is never decoded as /3 production SourceVersion/2;
- ConflictRecord/1 with createdAtFrontier remaining Frontier/1 and unchanged ConflictKey/1, ConflictId, and D6-ConflictKey/1 hash domain;
- D7 PreparedActionBinding/1,/2;
- D8 PreparedEditBinding/1;
- existing Result/ByteHandle historical tokens/records with their original authorization, pins, clock, continuity, and expiry/reset rules;
- any SourceObservation/1, SourceVersionRef/1, or protected token binding actually referenced by an old decision/pin, decoded only under its producing contract.

The presence of a decoder or historical draft prose does not prove that the corresponding prototype was deployed or is active, and tests never expand “decoder exists” into a product compatibility claim. Conversely, once a saved, planned, or unknown record actually exists, its original request/decision/receipt/error, pins, lifetime, authorization, unknown state, charge/approval/claim, external-effect, and no-duplicate-effect obligations are not cancelled by a new version, I rebuild, reauthorization, or absence of deployment evidence for another prototype.

A new v2/new-FA fixture is never created by changing only wireVersion or copying old bytes. Construct it independently under the current closed schema and separately cover SourceVersion/2+SourceObservation/1, SourceRevisionPlan/1, RevisionTokenBinding/2+d6_source_revision/2, DependencyProof/2+all fourteen DependencyKey/2 kinds, InstallationNotice/2, ContentCompletionProof/3, and ConflictRecord/2. old→new implicit upgrade and new→old fallback both fail. Unknown owner version uses unsupported_version/owner_update_required or the owning unavailable state rather than approximating a legacy path.

A planned fixture proves that an unsealed decision has no ChangeId and resumes only its original plan/pins/version basis. A saved fixture proves that current r6 business proof does not retrospectively reject r5 and that revocation only hides delivery. An unknown fixture proves that equal hash/current file/Derived Index cannot guess success/failure. A new current Observation or revision token never re-signs an old binding.

A legacy canonical request containing full source/pins remains legacy evidence and is not migrated. New PreparedIntent/2 uses InputDescriptor, closed owner binding, and purpose-bound PinRef. Tests prove that equal hash, equal bare revision, or equal final text with different production domain, SourceObservation, DependencyKey stamp, owner descriptor, or pin continuity is never the same input/currentness.

## 8. Downstream consumption gate

The private P1 Storage, Control, Lexicon, and machine Registry author afterimages actually exist. This Impact/Test Outline only turns them into future acceptance obligations and cannot unilaterally mark a downstream owner accepted. A managed/strong path depending on a new producer remains owner_update_required, proof_unavailable, or the owner's existing unavailable outcome until its real consumer afterimage and fresh joint acceptance are complete. Approved ordinary `.adoc`/Resource reads, Draft, fully qualified human whole-source saves, and local offline operations that do not depend on a missing strong producer/consumer are not permanently disabled.

- D3: fixed C already has the wire12 candidate; it is no longer described as nonexistent. P2 still must genuinely consume D3-native OwnerInputBinding/2, DecisionKey/2, Frontier/2 policy, complete SourceObservation/1, and the D3-owned lifecycle, placement_range, and ref_inbound ranges among the fourteen DependencyKey kinds. A managed after additionally consumes the same original plan's SourceRevisionPlan/1 and d6_source_revision/2, and primary receipt plus D3DecisionCompanion/2 share one P seal. D3 Locator's opaque revision-token lexical form and real WriteScope are not expanded by D6.
- D4: fixed C has the local-vs-complete and earlier A/B/C SourceObservation/Frontier/WriteProtection consumer candidates, but P2 still coordinates production SourceVersion versus observerDomain, real relation_incidence/calendar_scope/registry/temporal_rules enumeration and stamps, RevisionTokenBinding/2 boundaries, saved/planned/unseen recovery split, and a new local Observation after ContentCompletionProof/3 reception. RelationReadContext/2, RelationReadBinding/2, and Recurrence /1 closed wire remain unchanged.
- D5: the existing local-vs-complete candidate still requires P3 consumption of new current Observation/currentness, revision-bound locator, DependencyProof cut, strict|observed_only ordinary-save boundary, and recovery split. D5 structured cell/row/column/reorder remains strict and never borrows the ordinary weak-save path.
- D7: Query Algebra must consume complete query_scan/cut; Value/CEL must consume real version/authorization dependencies; View must preserve complete-result/reset rules; Narrow Field Qualification must retain static independence/non-disclosure; Definition Transfer must reuse the same candidate map, SourceStamp, and revision binding; Preview/Effects must bind original plan, pins, and historical retention; Execution/Action must retain complete evidence and current authorization generation; Prepared Action Binding needs an actual new consumer afterimage; Scenario Dispositions must cover positive/negative/unknown/recovery; Terminology Lexicon/Registry must synchronize the real owners; Implementation Impact/Test Outline must synchronize the acceptance surface. D6 free JSON, a Prepared-only rewrite, or a coarse Frontier judgment substitutes for none of these.
- D8: later Source/Live/Read, Draft/Edit Map, IME/Undo, ReliableSaveState/portable publication, conflict, and collaboration session must bind the complete current SourceObservation rather than production SourceVersion alone; gap/replacement resets or rebases and legacy SourceVersion/1/PreparedEditBinding/1 decoding remains.
- D9: ImportJob/ExportPlan new pins/version, construction/import/export cuts, and foreign-version/binding consumers still need actual afterimages. D6 never invents a D9 comparator or free payload.
- D10: all eighteen actual owner files still need complete coordination for sourceOccurrenceKey continuity, reviewable recipient/target/payload approval, ApprovalUse, Run/Lease/Automation/Workspace/deployment Money lineage, claims, provider external request/result unknown, stop, and execution responsibility. execution_resource DependencyKey does not absorb those D10 subcontracts; U6/U7 remain open.
- The eleven existing OPEN findings, A2 pre/post fresh gates, and the final separate fresh Pro global review are not discharged by producer documents, machine Registry, or fixtures.

The existence of D6 documents, passing author-side mechanical checks, or readable candidate files never means these consumers already support the semantics, never counts as product conformance, and never makes every historical decoder an active surface.

## 9. Documentation/machine checks

At least verify:

- Chinese/English heading and closed enum/key sets match;
- terminology conceptId uniqueness, ownedNames collision checks, and preservation of every old firstFreeze;
- replacements.json lists only actual afterimages and exact fixed-S sourceBlob values;
- no wording claims a nonexistent future owner afterimage is active;
- D6 v2 JSON examples receive strict JSON/schema inspection; Counter is never Boolean;
- ConflictId, Frontier sorting, ChangeId overflow, Policy/3 capabilities have positive/negative cases;
- docs/design/snapshots, inputs, and all 18 D10 files remain unchanged in this batch.

Documentation CI success is not product conformance or independent review.

## 10. G0-A save/interface acceptance additions

Everything below is future implementation/conformance evidence, not a test run or pass in this author document.

- WriteProtection is explicitly selected and frozen by a trusted human before planning starts; a strict request never mutates into observed_only. observed_only is limited to exactly one existing live Document, ordinary+replica_local whole-source save, complete source read/replace, author-source write set empty or that Document only, no applicable body/Field/node-control deny, no identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation, and Draft Base=current SourceObservation. noninteractive, managed_atomic, structural/multi-entity, D5 structured, bulk/collection, D7 strong Action, Automation, server checkpoint, Approval, and Money are strict.
- Fault injection distinguishes inputRetentionState=retained, InstallationState, ReliableSaveState, and PortablePublicationState. prepare/retained is not Saved. strict durable install+P seal yields reliable; only a fully qualified observed_only durable install+P seal yields durable_observed_only. B/N stay durable under the original plan. Only C still unseen after the final check may be overwritten by N and may have no recoverable copy; a later C may become current without losing B/N. Observed competition, stale Base, watcher gap, third_state, revocation, or missing qualification never falls back; unknown installation/provenance is recovery_unknown.
- SourceVersion/2 tests cover managed and external production versions, production CommitDomain/production observationEpoch versus current observerDomain/current observationEpoch, and all H(D,E) rules: H=0 only for a proved complete empty history; fresh managed=1; checked H+1 within one domain; no reset across production observationEpoch; MAX no wrap; first write in another domain uses that domain's own H; return to an old domain resumes its H; equal-byte external admission still creates managed H+1; true raw no-op preserves source version; source deletion and source-unchanged portable structure/lifecycle do not advance H even though a real portable effect still receives a ChangeId at seal.
- SourceRevisionPlan/1 tests prove that only a plan producing a managed after freezes before Observation/absent, lastIssued/complete-empty-history basis, SourceStamp, and exact after pin. The winning plan never resamples revision, target, or H basis. plan, stage, InstallationNotice/2, and recovery_unknown have no new ChangeId for this decision. Only the single P seal combines SourceStamp with the seal ChangeId into managed SourceVersion/2 and advances H. Publication/delivery failure after seal never reallocates ChangeId/revision or charges again.
- RevisionTokenBinding/2 with d6_source_revision/2 proves token stability for the same `(observerDomain,observationEpoch,source)` while continuity holds. A managed proposal token is original-plan-only before seal. Watcher gap, external/object replacement, discontinuous rematerialization, or observerDomain/observationEpoch change invalidates old currentness even when production SourceVersion, bare revision, hash, or text is equal. D3 Locator and D4/D5 inner selector wire retain owner-defined shapes and are never silently redecoded as the new profile.
- SourceObservation/1 and SourceVersionRef/1 bind the real FileObjectBinding and evidence pins. After receiving ContentCompletionProof/3, a receiver creates its own Observation/Ref from its own CommitDomain, current FileObjectBinding, observationEpoch, pins, and current control/Registry/incidence cut and never reuses sender sourceToken.
- Frontier/2 proves only verified continuous sealed causal prefixes. exact requires complete equality with original expectedFrontier and D3 managed_atomic remains exact. scope_dependencies continues only when every new head from original base to current cut has a complete continuous sealed chain with no regression/hole, every original source/control/authorization/positive-negative DependencyKey remains valid, and the added effect is proved unrelated. The original canonical request, expectedFrontier, DependencyProof.baseFrontier, targets, Query/selector, pins, proposed bytes, WriteProtection, owner input, and version basis are never re-signed or resampled. Provider “synced”, larger vector numbers, and equal final hash/bytes are insufficient.
- DependencyProof/2 stamp remains `{epoch,revision}`. epoch is the continuity generation for that exact complete DependencyKey range. A newly completely enumerated epoch may begin at revision=0, which does not mean unknown, empty, or source revision0. Within one proved-continuous epoch, an affecting change invalidates the old current proof before checked revision increment. A gap, owner-decoder/rule change, actual proof-directory loss, or unproved continuity creates a new epoch. I-only delete/rebuild merely rebuilds cache while protected P/M correctness facts and event chain remain complete; if those facts are truly lost, current-authorized complete enumeration is required.
- Every one of the fourteen closed DependencyKey/2 kinds has positive, negative, unknown, and authorization-before-read tests, and none accepts free JSON:
  - source: positive binds complete current SourceObservation, exact bytes/value pin, and FileObjectBinding; absent comes only from real identity/lifecycle/FileBinding plus trusted absent state. Placeholder, I miss, I/O failure, and outward body disclosure without source_read never become success.
  - lifecycle: D3 proves complete birth/claim/live/Trash/tombstone/never-known plus owner relation after state disclosure; index miss never proves absence.
  - placement_range: D3 completely enumerates the nine StructureRange cases covering sibling lists, Trash roots, ancestor chain, subtree, owner-local resources/annotations, reply closure, and restore membership; structure disclosure precedes hidden sibling reads.
  - ref_inbound: D3/real slot owners completely enumerate target inbound; zero inbound requires a complete directory/stamp and never comes from a current page or empty I result.
  - relation_incidence: D4 proves positive/negative incidence under RelationReadContext/2 fieldId, endpointNodeRef, revisionToken, and complete factSelectors. masked/unprovable cannot be complete and original D4 wire remains unchanged.
  - calendar_scope: D4 semantics plus D6 configuration persistence covers binding, series, period, and scope_inbound CalendarRange, complete SeriesScope/RegistryBinding, period membership, negative range, and control inbound. index empty/current hits never choose unique/many.
  - registry: bind complete same-Workspace RegistrySnapshot/1, RegistryBinding/1, required RegistryEvolutionProof, and Field/Facet/alias/namespace/contribution directories. unavailable/unknown definition remains distinct from a proved complete empty definition set.
  - temporal_rules: bind actually used comparator, period rule, timezone/tzdb, RecurrenceReadBinding/1, and finite horizon coverage. Missing segment, unproved rule provenance, and unsupported business value are distinct and device timezone never fills a gap.
  - authorization: principalAudienceToken comes from trusted principal/session/delegation. Proof binds current Policy/3/auth generation, delegation, ObservationScope, and applicable capability without exposing hidden grants/denies. Any change affecting disclosure/write eligibility changes the stamp.
  - foreign_binding: D3 owns binding semantics, D6 directory continuity, and D9/D10/real source owner the comparator/version format. Unknown profile/decoder makes the strong path proof_unavailable; UID/etag/path/hash/text or free wrapper never substitutes.
  - query_scan: D7 proves complete visible enumeration and hidden-policy generation for the named principalAudienceToken/domain/selector; headings also validate accurate D2 source per applicable Node. Partial page, ResultHandle/cursor, CEL, or no hit never counts as a complete scan.
  - replica_registry: D6 proves the full active/retired ReplicaRecord directory, registrationSequence, and gap-free continuity. purge proves each replica's admission of the named Frontier or retirement; provider sync/count/one Frontier never substitutes.
  - conflict_record: id selector proves exact record/version and subject selector the complete conflict directory with real sealed heads. conflict_read and subject disclosure precede access; unsealed external competition, third_state, or unknown install never fabricate ChangeId.
  - execution_resource: D6 reads only the named DecisionKey+protocolOwner operation's resource policy, attempt state, cumulative work, pins/capacity, and pause category. It neither enumerates all OperationIds nor treats absence/P loss as refund/quota reset and never absorbs D10 Money/provider-unknown/sourceOccurrenceKey.
- Complete-range empty and non-empty results use the same consistent snapshot/range barrier or gap-free continuous change chain plus final revalidation. Partial/building index, index miss, unknown decoder, placeholder, I/O failure, missing shard, or hidden unauthorized data never becomes empty-success. Ordinary/local operations depending only on complete real local evidence continue under their own qualification and are not permanently stopped by an unrelated unavailable complete-query proof.
- ContentCompletionProof/3 tests cover notice/proof/component key agreement, real seal ChangeId, actual frontierBefore/frontierAfter, production SourceVersion before/after, external→managed admission, deletion with absent after, source-unchanged portable effect with no fake sourceChanges, and receiver-owned Observation. /3 grants no Query/Action completeness. ContentCompletionProof/1,/2 retain original bytes/decoders/pins and /2 SourceVersionRef sourceChanges is never decoded as /3.
- ConflictRecord/2 tests cover createdAtFrontier=Frontier/2, real sealed heads, and new-head supersession while ConflictKey/1, ConflictId, and the D6-ConflictKey/1 hash domain remain unchanged. ConflictRecord/1 remains Frontier/1 and historical records are never re-encoded merely because /2 exists.
- D3-native OwnerInputBinding/2 and D3DecisionCompanion/2 share the same DecisionKey/P seal and create no second receipt/ledger or WriteScope expansion. Fixed C already has the wire12 candidate, but new success depending on these P1 producers stays gated until the P2 native consumer exists.
- saved/planned/unseen get dedicated fixtures: common disclosure/P/domain continuity precedes business state; saved replays original bytes and current r6 business proof never retrospectively rejects it; planned has no pre-seal ChangeId and resumes only original plan/pins/version basis; unknown is not guessed from files/hash/I; revocation after committed may hide delivery but never edits history.
- Windows, Linux, sync-folder/placeholder backends, and Server failover still require real-platform fault injection for conditional/exclusive/create/observed primitives, flush, rename, event gaps, and crash windows. Documentation, API names, fixture lists, or CI do not substitute for execution.

Fixed C already contains the D3 wire12 and earlier D4/D5 consumer candidates, but they have not consumed this P1's full production-revision, DependencyKey/Proof, CP3/Record2, and recovery rules. New strong success depending on those rules remains gated until P2/P3 and fresh joint acceptance. D7, D8, D9, and D10 remain gated across the complete scope named in §8.

## 11. Fresh review gate

The complete coordinated candidate still requires a new independent review from scratch over the final new proposal/routing, every actual replacement owner, machine Registry, all eighteen D10 files, and the fixed S49 inputs, followed by cross-owner positive, negative, unknown, recovery, and historical-decoder review against the final assembled bytes. Author-produced Storage/Control/Lexicon/Registry/Impact artifacts, mechanical checks, design_inputs, docs, tests, diffcheck, or CI—when actually run—prove only their named checks and are not independent semantic acceptance, product implementation, or activation.

Author reading provenance is reported only for the files/ranges actually read in each author task. It never upgrades an older independent 49/49 review, a prior bilingual pass, D10 all18, or another conversation's coverage into fresh independent full reading for this candidate. The reading record of this Impact artifact proves only the two actual-C Impact files and the final private Storage/Control/Lexicon/Registry producers actually read here.

The historical B13 REVISE result and its then-current terminology/bilingual FAIL remain historical status, not an author-side closure merely because later terminology/machine checks improved. The original P0=0, P1=3, P2=8 eleven OPEN findings and U6/U7 remain open until later fresh independent joint review disposes of them. This batch neither closes nor reclassifies nor accepts them.

After this Impact author afterimage, P1 still has PROPOSAL/replacements routing. P2 D3/D4, P3 D5, all actual D7–D10 consumers, and fresh joint acceptance are still incomplete. A2 self-contained reconstruction must follow, then a separate fresh Pro global review after A2. Until those gates complete, the candidate remains unaccepted, unactivated, unimplemented, unmerged, and unreleased.
