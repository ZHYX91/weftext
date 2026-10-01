---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: `c99e6e2b-dc0c-4a3b-ab35-75a85d12ef99`.

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. The revision05/D7/D9 status labels in fixed S are historical provenance only and do not govern this afterimage. The stable document ID is preserved. Activation is forbidden until every required owner afterimage, D10 consumer update, fresh independent joint review, and coordinated acceptance is complete; partial activation of managed v2 success is prohibited.

# D6 Storage Transactions Permissions and Sync

## 1. Scope, authority layers, and non-goals

The primary D6-FA-r01 change is to separate current author bytes, portable control facts, non-reconstructible execution responsibility, rebuildable indexes, and device/session drafts into distinct logical owners. No physical medium may become a second writable truth merely for convenience.

The current logical ownership matrix is closed:

| Fact | Unique logical owner | Physical carrier | Copy semantics |
|---|---|---|---|
| current Document exact source | the corresponding ordinary .adoc file | ordinary user-visible file | byte-copyable by ordinary sync |
| current Resource bytes | the corresponding ordinary resource file | ordinary user-visible file | byte-copyable |
| Node/Resource/Annotation identity, allocation and no-reuse | Portable Workspace Metadata | portable files inside the workspace | copyable |
| Node parent/order, lifecycle, Trash restore membership, file binding | Portable Workspace Metadata | portable metadata files | copyable |
| current Annotation closed value | ordinary data record inside Portable Workspace Metadata | portable data file | copyable |
| shared Registry/configuration, ACL/policy and trust declarations | Portable Workspace Metadata | portable control files; credentials/private keys excluded | copyable subject to trust validation |
| original request/decision/receipt, transaction recovery, unknown effects, approval/claim/Money and necessary pins | Durable Control Store | durable SQLite outside the workspace plus private pin area | file copying does not copy consumption authority |
| metadata/search/query-candidate/OCR caches | Derived Index Store | device-local SQLite outside the workspace | discardable/rebuildable; not synchronized |
| Draft, input log, Selection, IME, uncommitted collaboration state | Draft Store / collaboration runtime | device- or Server-private | not author content |

“Portable metadata”, “durable control”, and “derived index” are prose abbreviations in this document, not new public entity domains.

Current Document/Resource bytes and portable metadata jointly form a file-backed workspace, but their responsibilities are disjoint. Metadata may point to the exact current file and own identity/structure/configuration/version chains; it must not duplicate the whole current body as another author source. Durable control may retain the before/after pins necessary for a particular decision but must not keep a whole-workspace current-body copy as a read fallback. Indexes and Draft are never author authority.

This specification does not change D2 AsciiDoc Profile2 source syntax, add a D3 EntityRef, introduce a persistent D5 Record domain, freeze OT/CRDT, implement product code, or expand any release-support matrix. Real-time collaborative text editing remains scheduled after G2 by D1.

## 2. Portable Workspace Metadata and physical file bindings

### 2.1 Portable current truth

A file-backed Workspace has one managed portable-metadata root. The first-generation physical directory name is .weftext-meta; the name belongs to the D6 file backend, not D2 author syntax.

Portable metadata uniquely owns:

- Workspace identity, root NodeRef, non-reused identity birth/tombstone/burn-portable facts;
- current FileBinding from NodeRef to the ordinary Document file;
- current FileBinding from ResourceRef to the ordinary resource file;
- parent and sibling order for every live Node. The ordered child-Ref list of each parent is the only current representation; child.parent and ordinal are mechanically derived and may not be separately writable;
- Trash forest, restore membership, and original-location hints;
- current Annotation closed value and owner;
- portable Registry bindings, shared series/scope configuration, shared ACL/policy/trust declarations;
- replica registration, ChangeRecord, Frontier, InstallationNotice, ContentCompletionProof, ConflictRecord;
- portable source semantic state and observation epoch.

Path, title, digest, mtime, inode/file-id are never D3 identity. FileBinding only says which ordinary file carries a Ref’s current bytes in a particular portable version. A coordinated external rename may change FileBinding without changing the Ref. Parent/order never comes from directory hierarchy.

Each portable record uses a versioned closed format, deterministic ordering, and one source-of-truth relationship. Sharding may change physical layout but may not create independently writable copies of the same fact.

### 2.2 File objects and external modification

File installation evidence binds a trusted FileObjectBinding rather than a digest alone. The trusted internal binding includes backend identity, canonical relative path, backend object generation/identity when available, the current observerDomain observationEpoch, byteLength, digest, and an absent/present branch. Digest provides integrity/comparison evidence; it is neither CAS, identity, nor a production version.

The observationEpoch inside SourceVersion/2 belongs to that production version’s production history; SourceObservation/1.observationEpoch belongs to the current observerDomain observation generation, and equal numeric values never collapse those roles. External programs may directly edit .adoc and Resource files. Any of the following advances the affected observerDomain observationEpoch and invalidates the old SourceObservation/1, SourceVersionRef/1.sourceToken, locators/maps/PreparedIntent bound to that observation, and the applicable dependency cut; already sealed historical SourceVersion/2 is never rewritten:

- object identity/generation change;
- byte/length/metadata mismatch against the managed binding;
- watcher/journal gap;
- delete/replace/rename not explained by a known portable ChangeRecord;
- placeholder materialization/dematerialization;
- backend loss of event continuity.

Equal digest does not prove that A→B→A did not occur. After an observation gap the current observerDomain observationEpoch advances even when final bytes equal the prior bytes. A new external state not attributable to a known ChangeRecord forms an external SourceVersion/2 and checked-increments externalSequence within its production CommitDomain+observationEpoch. An external version has no managed revision or ChangeId, and externalSequence is never substituted into D3/D4/D5 managed sourceRevision/Locator/selector.

External bytes that fail strict UTF-8 or D2 parse remain the actual external raw bytes; Node identity remains but D2 projection is unavailable and the state is external_invalid. Core never replaces them with an old pin/index and never calls that state a Core author commit. Valid external bytes remain usable for authorized raw/source read, Draft, and human ordinary whole-source save. Only structured/typed-selector operations that require a managed inner revision must first obtain an explicit managed admission/save producing a real managed SourceVersion; absence of a managed revision is not a permanent ban on the approved ordinary-file editing model.

## 3. Durable control, derived index, and Draft

### 3.1 Durable control

Every executable CommitDomain has a durable control SQLite database outside the workspace. It stores only control facts that cannot be safely reconstructed from current files/portable metadata:

- canonical requests, fingerprints/input descriptors, decisions, receipts and errors;
- planned/terminal recovery, attempts/budgets/leases, reservations and exact execution owner;
- installation recovery state, write set and portable-publication state;
- ApprovalUse, claim, Money/cost lineage, frozen external request/send/result unknown, stop responsibility;
- exact descriptors and necessary pins for PreparedIntent/PreparedAction/PreparedEdit;
- managed job control such as ImportJob/Export publication;
- current execution authority/custody/fence.

Active control.sqlite3, WAL, SHM and private pin staging are outside ordinary sync and are never merged by a sync provider. SQLite’s single writer constrains one physical database write transaction; it does not limit the product to one user, one Draft, one reader, one prepare, or one editing session.

Durable control is not the current owner of portable metadata. It may reference a portable/source version and keep recovery copies for a decision, but recovery may only complete the version bound by the original plan and may not derive a separate latest parent/order/policy that overwrites portable metadata.

### 3.2 Derived index

The derived index is a device-local SQLite database with state building|ready|unavailable. Each layer records complete build inventory, parser/profile/Registry/search/OCR versions, input SourceVersion/Frontier and the continuously consumed invalidation position. ready is scoped to that layer and range. A partial or old index cannot prove an empty range or completeness.

Layers may include inventory/metadata, D2 parse projection, typed Field/relation candidates, search candidates, attachment extraction, and OCR. A whole-workspace current-source or full-AST replica may not be retained merely for convenience. Contentless FTS/gram structures may store candidate tokens but remain subject to permission, coverage and final source re-read semantics.

Deleting the entire index leaves identity/structure/policy, original decisions, ApprovalUse/Money, and any DependencyProof range-continuity facts still complete in their real P/M owners unchanged. Rebuild never mints identity, consumes approval, replays an external effect, or re-signs an old proof. Correctness-critical range epoch/revision, complete-enumeration boundary, empty proof, and continuous-consumption position are not owned by I: I only caches candidates/enumerations, and `ready`, hash, row count, or “no hit” never proves completeness or empty. If those correctness-range facts themselves are lost, or watcher/owner-version continuity cannot be proved, the old stamp is invalid; a new proof requires a complete real-range enumeration under current authorization and a new continuity epoch. Placeholder, I/O failure, hidden unauthorized objects, or uncovered shards never mean empty. D6 stores its closed key/stamp/pins while D3/D4/D7 own their enumeration algorithms. Storage accepts no free JSON and missing strong-range proof never permanently blocks an ordinary local operation that does not depend on it.

### 3.3 Draft

Draft preserves the existing D8 session model: user proposal, input log, selection and base binding. It has no author revision, ChangeId, D3 locator authority, or committed status. Ordinary cache cleaning never silently removes dirty Draft. Durable save and portable publication are independently readable states; Draft persistence, worker success, HTTP 200, or sync upload never substitutes for them.

## 4. CommitDomain, ReplicaEpoch, DecisionKey, ChangeId, Frontier, and source observation

Control Interfaces owns exact wire shapes. CommitDomain distinguishes replica/server: replica=WorkspaceRef+ReplicaEpoch and server=WorkspaceRef+current hosted AuthorityInstanceId. New v2 DecisionKey/2 is WorkspaceRef+complete CommitDomain+OperationId; P indexes workspaceId+D3-CJ/3(commitDomain)+operationId and one key has exactly one D3 or D6 protocol-owner/canonical decision. Different domains may reuse OperationId. Legacy v1/D3 v9-v11 saved decisions retain original decoder/continuity.

ReplicaEpoch is Core-minted only on explicit registration, UUIDv4, never reused. It grants ordinary content-domain qualification under current shared policy/trust only, not D3 continue, AuthorityInstanceId, or execution custody. File copy never creates an epoch and retired never revives.

ChangeId/1 is complete CommitDomain+monotonic sequence, first portable content change 1, checked increment/no wrap. ChangeId is allocated **only at P seal for a portable effect**; prepare/planning/staging/InstallationNotice reserve none. Failure, paused, recovery_unknown, control_only, true raw no-op create no ChangeId/hole.

Frontier/2 is a canonical vector with at most one maximum verified continuous sealed ChangeId per domain, domain-byte sorted unique. It may admit remote verified ChangeId without replaying remote OperationId or minting local ChangeId merely for transport. It proves neither payload materialization, placeholder download, index completeness, D7 complete cut, nor provider completion. frontierPolicy=exact|scope_dependencies: exact requires complete Frontier equality. scope_dependencies admits only a proved non-regressing unrelated extension from original expectedFrontier to current Frontier and revalidates every original source/control/auth/positive-negative dependency. Growth by a truly unrelated sealed head is not itself a dependency change; a changed SourceObservation, FileObjectBinding/pin, authorization, Registry/rule, membership/negative-range proof, or other bound dependency is still stale/conflict/reprepare. scope_dependencies never changes target/Query/source/request, reselects a current page, erases ABA, or treats an unknown gap as unrelated. Frontier/1 is legacy only.

SourceVersion/2 identifies actual source plus production CommitDomain. For production domain D and entity E, `H(D,E)` is the greatest managed revision of E in D’s continuous sealed history. H=0 is a complete empty history only when domain birth/registration, P continuity, and verified portable sealed history prove that D has never sealed a managed version for E; missing/corrupt/unknown history is never empty. Every real managed source change uses checked H+1, observationEpoch changes do not reset H within the same production domain, and MAX never wraps.

A fresh managed source is revision=1 in a proved empty history. An existing source first written by another production domain uses that new domain’s own H+1, never old-domain revision+1; later cross-domain returns continue each domain’s own H. Equal revision numbers across domains are not equal versions. A true raw no-op preserves original SourceVersion/2 even when its production domain differs from current operation. Pure placement/lifecycle/control with unchanged source does not increment source revision. Source deletion has absent after and no “deleted SourceVersion”. Equal-byte external admission establishes managed admission and is not raw no-op. The external SourceVersion/2 keeps its existing complete variant with kind, version, entityRef, commitDomain, observationEpoch, and externalSequence. The production-version fields relevant here are commitDomain, observationEpoch, and externalSequence; it has no managed revision or changeId. Managed admission still uses the current managed production domain’s H+1 and externalSequence never becomes inner sourceRevision.

Current replica/Server observation separately binds observerDomain, EntityRef, complete production SourceVersion/2, current observationEpoch, FileObjectBinding, and evidence pins in SourceObservation/1. observerDomain=current operation domain while sourceVersion.commitDomain may differ. SourceVersion.observationEpoch remains production history; SourceObservation.observationEpoch is the current observer generation. Placeholder/missing metadata/conflict/unproved continuity yields no successful Observation. SourceVersionRef/1 selects the complete Observation; gap, external replacement, or discontinuous rematerialization invalidates old current qualification even when production version/hash/text is equal.

For every plan that will produce a managed source version, Storage durably stores internal SourceRevisionPlan/1 in P: before current Observation or explicit absent, the last sealed managed SourceVersion for this production-domain entity or proved none, proposed SourceStamp/1, and exact after pin. SourceStamp is only the same DecisionKey’s proposed entity/revision/production-observationEpoch address. When current operation is the after production domain, that epoch is the target observation generation frozen by the original plan in the current operation domain, never copied from a foreign before production epoch. It contains no new ChangeId and is not successful SourceVersion. Closed shape is owned by the later same-P1 Control afterimage; Storage creates no second wire owner. The winning plan freezes this basis and seal revalidates last-issued history.

The D3 stage12 private candidate map remains the sole fresh-identity candidate source. Control freezes a revision-token profile/2 and Storage persists it, binding observerDomain/current observationEpoch plus managed SourceStamp or external SourceVersion; D3 Locator/revision-token outer lexical shape and D4/D5 inner selector wire do not change. A proposed managed token is plan-internal symbolic/validation evidence until the decision seals and a complete current SourceObservation exists. Legacy profile/decoder remains unchanged; equal hash/text/revision/I never restores qualification across an observation gap.

If an existing CommitDomain loses P/production-history continuity, it cannot guess H=0 and continue issuing managed revisions in that domain. This does not permanently freeze the file-backed Workspace: when portable current state is fully verifiable and the affected range has no unresolved installation risk, existing replica-registration rules may create a new ReplicaEpoch/CommitDomain for ordinary content. The new domain starts from its own complete empty H; old-domain decisions/unknown/Money are never reconstructed from files.

## 5. Authorization, ordinary save, and complete semantic qualification

### 5.1 Three independent qualification layers

D6 freezes:

1. ordinary replica content qualification — the actual source/identity/structure/policy scope touched by ordinary source/create/move/reorder/Trash plus an eligible installation backend;
2. complete semantic/action qualification — the complete positive/negative ranges, complete cut, authorization and semantics required by the particular D4/D5/D7 action;
3. global execution responsibility — continuous single responsibility for Automation, ApprovalUse, claim, Money, external unknown and stop.

Loss of global execution responsibility suspends only that execution capability, not ordinary content. A building index affects operations needing complete proof, not unrelated ordinary content. Conversely, ordinary content success never upgrades to complete Query/Action proof.

### 5.2 Ordinary proof scopes

Ordinary full-Document edit binds complete SourceObservation/1 (including actual production SourceVersion/2), complete current source, entity lifecycle, current policy, actual MutationFootprint, complete D2 parse, and every actually modified local typed fact with its current Registry definition. Unmodified bytes are preserved exactly. If a touched transformation needs cross-object proof that is currently unavailable, an explicitly requested ordinary-save profile may produce semantic_pending rather than complete semantics; it may not be recorded as the original strong Action success.

create Node binds destination parent, full target sibling list, required ancestor/cycle proof, new identity reservation, complete new source, actual local typed admission, and old/new policy scope. move/reorder binds subject, old/new parent, both complete sibling lists, necessary ancestor/cycle proof, and old/new authorization scope. Ordinary Trash binds the selected subtree/owner-local closure, affected live/Trash sibling lists, restore membership and permission; it does not fabricate a complete workspace-wide inbound-reference enumeration.

restore, purge, relation mutation, unique Calendar/configuration, collection operations, all_result/bulk and other upstream-complete operations remain complete-qualified. A failed complete Action may not silently fall back into the same successful Action. The user may separately issue an ordinary source-save request, whose result is semantic_pending and does not make the strong Action successful.

Narrow Field qualification retains static independence proof and never reads hidden data first to decide permission. Structure qualification similarly requires structure-observation permission before reading hidden sibling lists.

### 5.3 SemanticState

Managed source state is:

- complete_semantics: D2 valid and all applicable D4/D5/structure/control semantics for the declared scope are completely proved;
- semantic_pending: D2 valid and actually modified local typed facts passed their local gates, while one or more cross-object/complete-range obligations remain unproved;
- external_invalid: current external bytes fail strict UTF-8 or D2 and do not correspond to a successful Core author decision; only Source/repair/raw-read paths may consume them.

semantic_pending may be the current source after an ordinary save; strict reliable versus durable_observed_only is represented independently by WriteProtection. It is not eligible for mutation that requires complete D4 relation/unique/Calendar invariants, D7 ActionEvidence/all_result/post-query over a complete cut, automatic Agent/automation writes, purge, or any export/audit claim that all workspace semantics are validated. Exact source read, Source editing, exact-file search and an explicitly local projection may consume it under their own permissions and must expose pending state.

The current D4/D5 afterimages in fixed C already consume the A/B/C Frontier/2, SourceObservation, and WriteProtection boundaries. This P1 adds production-revision, range-proof, and portable-record versions that still require P2/P3 consumer updates, so no new strong success is declared available. The D7 complete consumer remains future.

### 5.4 Concurrent-write boundary for human ordinary save

WriteProtection is orthogonal to ContentGuarantee/SemanticState. observed_only is limited to trusted `interactive_source_save` of exactly one existing live Document as ordinary+replica_local whole-source save, with complete source read/replace, author source write set empty or limited to that Document, no applicable body/Field/node-control deny, no identity, parent/order, lifecycle, shared policy, Registry, Calendar scope, or other-entity mutation, and Draft Base equal to the selected current SourceObservation. The human explicitly selects observed_only before planning starts and the profile freezes. After planning starts, strict failure, known Base conflict, revocation, durability failure, strong-obligation failure, or any missing eligibility never falls back to observed_only.

The sole relaxation is an external race never observed after final verification and before installing N. Read before-image B and input N remain durably retained by the original plan. An unseen C may have no recoverable copy and a later C may replace current file again, but durable B/N is not discarded. Observed change, watcher gap, stale Base, narrow deny, competing Core writer, unknown install, or lost P continuity still stops; known competition/gap is conflict-reprepare, unknown install remains recovery_unknown, and prepare/retained is not Saved. D3 identity/parent/order/lifecycle, D5 structured cell/row/column/reorder, bulk/collection/promotion, D7 strong Action, Automation, server checkpoint, approval, and Money all remain strict. Ordinary semantics and strict|observed_only protection are independent axes and never expand authorization; there is no per-save approval.

## 6. File installation capability, WriteProtection, and reliable save

### 6.1 BackendQualification
strict existing-file paths use real conditional_replace or exclusive_write_window; expected-absent new files may use create_only. conditional requires trusted generation and read-hash-then-rename is not CAS; exclusive excludes all threat-model writers and advisory locking is insufficient; create_only is atomic create-if-absent.

observed_replace is only §5.4 observed_only, records final verified object generation but is not CAS; immediately before destructive install recheck trusted object/event continuity and stop with B/N/current retained if competition is observed. Only an unobserved race belongs to the approved weak guarantee.

All paths still prove staged/target/directory-entry or rename durability, canonical containment, and installation provenance. strict without a qualified primitive -> install_unavailable/paused; an intent satisfying §5.4 may new-prepare observed_only, but existing strict plan never weakens.

### 6.2 Guarantee
ContentGuarantee is separate from WriteProtection. replica_local proves fixed write set durably installed+P sealed under requested protection and says nothing about other offline replicas. managed_atomic requires all author/portable-control writes inside the managed barrier with WriteProtection=strict and promises no instantaneous multi-file atomicity to external tools. Multiple renames are not a global transaction.

## 7. Prepared input, installation, seal and portable publication

### 7.1 InputDescriptor, SourceRevisionPlan, DependencyProof, and pins

The v2 canonical commit request remains small and never embeds complete bytes. PreparedIntent/2 stores Workspace/CommitDomain, intent, Frontier/2+frontierPolicy, ObservationScope/2, SourceObservation/1, DependencyProof/2, scope/profile/write set, Registry/policy/rule, and OwnerInputBinding/2; complete bytes remain purpose-bound pins.

Whenever a plan may produce a new managed source version, its installation plan also freezes the §4 SourceRevisionPlan/1: before observation or absent, the last sealed managed version for this production-domain entity or complete-empty-history proof, proposed SourceStamp, and exact after pin. It is version-allocation evidence, not a successful version, and contains no not-yet-existing ChangeId. external→managed admission uses this branch; true raw no-op and pure structure/control source-unchanged branches create no SourceRevisionPlan.

Complete enumeration, empty proof, range epoch/revision, current authorization, and required pins referenced by DependencyProof/2 must live in their real P/M owners and remain revalidatable under the original plan at planning, verify, and recovery; they never live only in I. D6 maintains continuity for source, authorization, replica_registry, conflict_record, and execution_resource. D3/D4/D7 provide their concrete closed keys/enumeration. An incomplete owner afterimage keeps a strong consumer owner_update_required/proof_unavailable; free JSON is rejected and an ordinary local operation that does not depend on that strong range remains available.

Input equality compares the descriptor, exact pins/owners, SourceObservation, DependencyProof, SourceRevisionPlan, and closed owner input completely. Equal sha256, H number, or rebuilt I is insufficient. Changing source/Observation, mapping, policy, frontierPolicy, scope, WriteProtection, owner request, or version basis requires new prepare; same OperationId is exact replay only and never strict downgrade.

### 7.2 Pin classes and retention

Every pin records purpose, owning request/job, byteLength, payload type, source version, creation control revision, last-reference policy, retention class and capacity account.

Long-lived protection is permitted only for planned/installation recovery before/after, unresolved conflict heads and merge base, frozen external-unknown/approval/Money evidence, owner-protocol ImportJob/ExportPlan/Query pins, and explicit user history/backup policy.

Opening, parsing or indexing a workspace never creates permanent pins for every current source. Once no decision/unknown/conflict/user retention references large effect bytes and retention permits release, those bytes may expire while canonical request, decision, receipt/error, approval/Money/claim responsibility and the minimal input descriptor remain durable. Saved decisions created under legacy contracts that promised decision-lifetime/permanent pins retain the legacy promise; D6-FA-r01 does not retroactively delete them. Reading legitimately expired new historical effects returns effects_unavailable and never substitutes current files for historical after bytes.

Capacity exhaustion rejects or pauses new prepare/install instead of deleting a protected last-reference pin.

### 7.3 State machine and ordering

Each v2 content decision stores independent DecisionState, InstallationState, ReliableSaveState and PortablePublicationState:

DecisionState: unseen → rejected | planned → committed | terminal_failed.
InstallationState: prepared → planned → installing(k) → installed, plus conflict | recovery_unknown | paused_authorization | paused_capacity.
ReliableSaveState: not_saved | reliable | durable_observed_only | not_applicable.
InputRetentionState: not_retained | retained | unavailable.
PortablePublicationState: not_published | pending | published | conflict.

The only order is:

1. prepare freezes InputDescriptor/write set/pins/proof/preview/budget/WriteProtection and SourceRevisionPlan when a managed version may be produced. retained requires durable proposal/read-before/bindings; no author effect and retained is not saved;
2. planning proves required pins durable/capacity-reserved and SourceRevisionPlan last-issued/empty-history continuity. One P transaction stores canonical request, fixed plan, reservations, installation recovery description, version basis, and planned. It freezes proposed SourceStamp only, allocates no ChangeId, and never records the stamp as a sealed SourceVersion;
3. before portable-current modification, durably write InstallationNotice/2 with DecisionKey/guarantee/WriteProtection/base Frontier/2/before-after components. baseFrontier may contain historical ChangeIds, but the notice has **no ChangeId for this not-yet-sealed decision** and no approval/Money/external payload;
4. install durably staged after. strict uses create_only/conditional/exclusive; observed_only is only §5.4 with final object/event check. Observed competition stops retaining B/N/current. D3 structure/lifecycle, multi-object, D5 structured, and strong operations remain strict;
5. verify written=planned after while unwritten dependencies continue against original before/cut expectations. exact still requires complete Frontier equality; scope_dependencies admits only a proved non-regressing sealed extension unrelated to every original source/control/auth/positive-negative range. Real SourceObservation, FileObjectBinding/pin, authorization, Registry/rules, membership/negative-range, or other dependency changes are stale/conflict/reprepare. Unknown provenance, third_state, late competition, or revocation remains paused/conflict/recovery_unknown; there is still no ChangeId for this decision;
6. seal when written=planned after and original plan/deps/auth still hold. One P transaction checked-allocates ChangeId. Only an actual source change whose after state is managed combines the corresponding SourceRevisionPlan SourceStamp with that ChangeId into the unique managed SourceVersion/2 and atomically advances H(D,E) for that production-domain entity; SourceStamp is never second current truth. Source deletion with after=absent retains this portable effect’s ChangeId and original notice/proof recovery responsibility but creates no managed SourceVersion, uses no SourceRevisionPlan, and does not advance H. A source-unchanged portable structure/lifecycle effect likewise creates no SourceVersion and does not advance H, while still using the ChangeId allocated at this seal as a portable effect. P-only control_only and true raw no-op still create no content ChangeId under §7.4. The same transaction then writes committed decision, receipt, effects, ReliableSaveState, applicable charge, and outbox. strict->reliable and observed_only->durable_observed_only. This is the only commit point;
7. publication for a new FA portable decision derives ContentCompletionProof/3 from sealed facts and advances Frontier/2. The proof transports actual production SourceVersion before/after, not the sender’s SourceObservation token. A receiver validates it and establishes its own SourceObservation for its observerDomain. ContentCompletionProof/2 and older versions remain original-decoder/replay only; control_only/no_op is not_applicable.

If step 6 succeeds and step 7 fails, decision and reliable/durable_observed_only remain successful while publication=pending. Recovery only publishes the same sealed-version proof and never rewrites source, changes OperationId, increments H/ChangeId again, recharges, or reinstalls N. A remote replica sees incomplete transport until proof/components are complete.

Revocation before seal prevents seal. exact before is restored/flushed only when BackendQualification proves safe restoration; otherwise retain before/after/current, SourceRevisionPlan, and recovery evidence and enter paused_authorization or recovery_unknown without overwriting third bytes. Revocation never converts unknown installation into permanent business rejection.

### 7.4 Raw no-op

Committed effectClass=portable|control_only|no_op. Only portable creates ChangeId/source changes/Frontier/2 advance; P-only control uses control_only. True no-op: empty sourceVersions, InstallationState=not_required, ReliableSaveState=not_applicable, PortablePublicationState=not_applicable; domainCommitSequence may +1 while source revision/ChangeId/Frontier stays unchanged. Equal-byte external admission is not no-op.

## 8. Crash recovery and historical results

Recovery first reads durable-control decision state, then installation notice/write-set bindings/actual files. Client timeout, file presence, mtime, digest or provider status never substitutes for P.

Each component is classified only as exact_before, exact_after with proved installation provenance, third_state, or unavailable. Equal bytes without installation provenance cannot be upgraded from third/unavailable to exact_after.

observed_only read-before is only the actually read/pinned before, not unseen C; overwritten unseen C may have no recoverable copy, which is the approved boundary and records never invent. Observed competition stays third_state/conflict; unknown install is never guessed.

Rules:

- no planned: clean only proven-unreferenced staging; a proposed SourceStamp/H candidate that never won a plan creates no history;
- planned/not installed: recover the same InputDescriptor, SourceRevisionPlan, pins, reservation, OperationId, and budget counters; never resample identity, H, revision token, or current page, and there is no ChangeId for this decision;
- installing: only before/after with continuous installation lineage resumes the same plan; third_state preserves current bytes/pins/version basis and enters conflict/recovery_unknown;
- all after but seal unknown: read P first. committed uses saved ChangeId/SourceVersions/receipt; planned resumes original plan and never guesses committed from files/hash/SourceStamp;
- committed response lost: after current delivery authorization for the **original saved effect scope**, return original receipt bytes. Old before SourceObservation, old Frontier, or r5 business dependencies need not equal current r6. No files/Frontier/H/charge is rewritten. Current revocation may hide delivery but never changes saved decision;
- committed with portable publication pending: publish only the protocol proof derived from original sealed versions; new FA uses ContentCompletionProof/3 while historical decisions keep original /1 or /2 decoder. Never reinstall N or allocate ChangeId again;
- derived-index/outbox failure: rebuild/catch up under its owner without rolling back author decision or reconstructing P responsibility from I.

saved, planned, and unseen are separate branches. saved performs original request/fingerprint/continuity lookup, current applicable disclosure/delivery authorization, and original-byte replay. planned only restores the original plan and continues/pauses according to actual installation/dependency state. Only unseen applies current owner version, SourceObservation, DependencyProof, and frontierPolicy to create a new business decision. Current r6 proof never retrospectively rejects r5 and never grants r5 a new strong qualification.

Historical receipt r5 and current source r6 are distinct: replay r5 returns saved r5 bytes while current read uses current SourceObservation/SourceVersion/Frontier. A later r6 complete proof proves r6 only and never rewrites r5. Undo/restore still requires new plan/preview; receipt replay never rolls current backward.

## 9. Synchronization, admission, and conflicts

### 9.1 Replica registration is not execution takeover

A new device verifies complete portable Workspace identity/metadata chain/policy/trust and available current components, then explicitly registers a new ReplicaEpoch. Registration establishes only an ordinary-content CommitDomain. It does not take over ApprovalUse, claim, Money, external unknown or Automation lease and does not invoke D3 continue_workspace.

Execution takeover separately proves complete continuity of decisions/receipts/charges/unknown/claims/standing approvals/stop and proves the former execution holder is fenced. Failure suspends that execution capability only; ordinary replica content continues.

Loss of P never reconstructs execution decisions/unknown from notes. A portable InstallationNotice may force reconciliation of its affected range, but unrelated ordinary source remains usable in a new ReplicaEpoch. Creating an empty control DB never refunds Money, restores an approval, or resends an external request.

### 9.2 Transport completeness

Sync providers transport ordinary files and immutable/versioned portable metadata only; they never transport active control DB/WAL/SHM, derived index or Draft.

A new FA portable change uses ContentCompletionProof/3. A receiver admits a ChangeId into local Frontier only after InstallationNotice, ContentCompletionProof/3, every listed component byte/metadata item, and the proof’s production SourceVersion before/after all arrive and mutually validate. The proof’s SourceVersion is production history, not the sender’s current SourceObservation. The receiver establishes a new SourceObservation/SourceVersionRef from its own CommitDomain, current FileObjectBinding, observationEpoch, evidence pins, and continuity; sender sourceToken is never copied as local current qualification.

ContentCompletionProof/2, InstallationNotice/1, and older saved transport retain original decoder/bytes/admission and are never mechanically rewritten as /3. Document-before-sidecar, sidecar-before-Resource, incomplete production-version metadata, unmaterialized placeholder, or proof/component mismatch is incomplete, not empty/deleted/committed and never “completed from I”.

A placeholder state is not_materialized. Operations needing bytes return source_unavailable/owner-specific unavailable; size zero, not_found, equal hash, or old locator is never substituted.

### 9.3 ConflictRecord and version boundary

Concurrent source, placement, lifecycle, identity, or policy heads create a stable ConflictRecord. ConflictKey binds WorkspaceRef, a closed conflict kind, the affected Ref set, and every concurrent head ChangeId. Refs and heads are unique/canonically sorted. ConflictId is a domain-separated SHA-256 address over canonical ConflictKey; complete key remains stored/compared and hash is not evidence.

New FA records use ConflictRecord/2 with created cut bound as Frontier/2. ConflictKey/1, ConflictSubject, the `D6-ConflictKey/1` hash domain, and ordering do not change. ConflictRecord/1 remains historical input under its original Frontier/1 decoder/bytes; version=1 is never interpreted with Frontier/2. Closed shape, field decoders, and read/prepare interfaces are frozen later by the same P1 Control owner; Storage owns portable storage, version selection, and recovery semantics only.

State still includes open, resolution_prepared, resolved, superseded. A new head supersedes an old open/resolution_prepared record and creates the linked new key/record. Resolution binds exact current key/heads and owner-specific plan; final write still compiles to the original D3/D6 typed request. resolved/superseded history is never rewritten to the new version or rehashed under current Frontier.

Required cases include source/source, create/create ordering, move/edit, move/move cycle, Trash/edit, purge/restore, duplicate Ref/birth mismatch, policy conflict, partial sidecar/body arrival, and placeholder. No conflict freezes the entire Workspace: explicitly selected branches may continue to receive descendants, while an ambiguous ordinary read never chooses a random head.

### 9.4 Trash and purge

Multi-device ordinary delete means Trash. A local Trash receipt proves the known closure, owner-local membership, placement and actual source effects of that operation and does not fabricate a complete workspace-wide inbound-reference inventory.

Permanent purge remains strong: complete related inbound/owner closure, complete semantic proof, and acknowledgment of the purge Frontier by registered replicas, or explicit retirement of replicas that cannot participate. A retired replica returning later enters conflict/reconciliation and never revives its epoch or tombstoned identity. Purge does not promise secure erasure of historical bytes already copied to another device/backup.

## 10. Index/query completeness and large-workspace startup

Startup milestones are separate:

1. T_first_open: read workspace-root/portable-metadata entry and list discovered/active targets;
2. T_first_edit: open active source and Draft;
3. T_first_reliable_save: strict install+P seal yielding reliable only; observed_only durable_observed_only is measured separately and never populates the old strict T_first_reliable_save;
4. T_full_search_ready: complete coverage exists for the specified search profile/range;
5. T_OCR_ready: selected attachment/model/version OCR is complete or explicitly failed.

Inventory, metadata, D2 parse, typed index, search candidate, extraction and OCR use bounded byte queues, bounded concurrency, batched index transactions and resumable checkpoints. 10k/100k/1M small files and tens-of-GB attachment/body cases require real benchmark evidence; this architecture claims no seconds-level target.

Exact source scan is the correctness baseline. Candidate-index hits are re-read from the exact current SourceObservation/source version before final evaluation. A tokenizer/gram with incomplete recall for exact/NFC/regex cannot provide completeness. D7 complete Query requires complete authorized execution, query_scan, and every applicable positive/negative DependencyProof. A building/partial index is only an explicitly scanned-range exploration surface and cannot issue complete ResultHandle/ActionEvidence.

Complete-range proof comes from the real owner’s consistent enumeration or provably gap-free continuously consumed change stream and revalidates current authorization/dependencies before publication. Empty is not “no index rows”: enumeration entry points, every applicable directory/shard, hidden-policy state, and continuity stamp still require proof. If completeness, event continuity, placeholder materialization, or owner decoder cannot be proved, a complete consumer returns the applicable unavailable/reset outcome rather than empty.

Rebuilding I never resurrects old proof. If original P/M correctness range epoch/revision and continuous-consumption facts remain intact, I rebuild only restores cache. If those facts were actually lost, old proof is invalid and current-authorized complete enumeration establishes a new range epoch. Ordinary save or D3 replica_local operations depending only on real local evidence do not permanently stop because unrelated complete-query proof is unavailable.

## 11. Server multi-user and real-time collaboration ingress

A hosted Workspace retains one durable CommitDomain/commit holder. Failover fencing must stop the old process from writing **both durable control and author files**. Preventing only old SQLite transactions while allowing old file renames is not fencing.

Multiple authenticated users may concurrently read the same/different documents, hold independent Drafts, run parse/query/prepare, and edit the same/different documents.

Different-document author commits lock only their write set and actual dependency ranges in deterministic key order. The final P seal transaction may briefly serialize commitSequence. An unrelated document does not automatically invalidate a plan solely because a coarse global sequence changed.

Same-document non-real-time mode retains both Drafts. After A seals a new version, B’s old-base commit is stale/conflict and enters base/current/proposed flow; B’s Draft is never overwritten.

For real-time sessions after G2, D6 now freezes ingress requirements without freezing OT/CRDT:

- session binds Workspace/Node, committed Base SourceVersion, sessionEpoch, participant principal/session, ordered client input sequence;
- receive acknowledgement, broadcast, and durable checkpoint are distinct; receive/broadcast is never labeled saved;
- complete-source participation requires the participant’s own source_read; another participant’s permission never exposes hidden bytes;
- IME preedit is not a shared author op; final composition confirmation produces at most one Draft input transaction;
- checkpoint is explicit save/collaboration-checkpoint and never per-keystroke author commit;
- checkpoint revalidates each uncommitted contributor’s authorization, actual footprint, D2/D4/D5 gates and dependencies over the composed exact source;
- revocation winning before seal prevents that principal’s uncommitted contribution from being committed under another principal and preserves it as proposal requiring reconfirmation;
- committed updates are broadcast only from saved decisions/outbox and each receiver passes current read gates;
- event gap/reconnect/mapping uncertainty forces reset/rebase rather than text-similarity guessing;
- transient collaboration op log is bounded session/recovery state, never a second durable Document source.

A future OT/CRDT adapter proves input attribution, composed source, source/selection mapping, duplicate/late-event handling and convergence. Failure preserves all user input and blocks the checkpoint; it does not silently drop operations or pretend that a single-user lock is real-time collaboration. The D1 post-G2 release boundary is unchanged.

## 12. History, backup, pins, and repair

A portable backup is an ordinary-file + portable-metadata closed snapshot at an explicit Frontier. It does not automatically carry consumable execution-control authority. Disaster recovery of global execution responsibility additionally requires protected control backup and proof that the former execution holder is fenced with continuous ledger/charge/unknown state.

History may retain old source/resource bytes but is explicit retention, not current source. Normal purge resolver cannot revive the same identity from history; a user may create fresh content from historical bytes.

repair, under repair/audit permission, may verify portable graphs/digests/FileBindings/Frontier holes, recover a P-proved planned install, retain third bytes as a conflict branch, rebuild the index, or re-materialize an ordinary file from an authorized backup. It may not infer receipts/ApprovalUse/Money from current files, infer install success from equal digest, delete competing bytes automatically, reuse old Refs, bypass purge/tombstones, or resend external effects without continuity.

## 13. Budgets, resources, and execution responsibility

BudgetBinding, attempt allowance and persistent charge remain execution-control facts. Work batches charge before execution; crash does not refund; uncertain attempt/clock epoch leaves the plan paused rather than fabricating terminal business failure.

Pin/capacity accounting distinguishes recovery, conflict, preview/query, import/export and user-history pins. Protected planned/unknown pins are not deleted by preview TTL. Capacity pressure may reject new prepare/install but never releases a protected last reference.

Minimum continuity for global execution responsibility includes canonical original request/decision/receipt/error; current lease/claim/standing-approval consumption; Money lineage across Run, Lease, Automation, Workspace and deployment including reservation/charge/refund evidence; frozen external request/send/result and unknown state; stop/emergency state; and sourceOccurrenceKey continuity evidence.

Replica registration, FileBinding and ChangeId/Frontier confer none of these consumption rights. sourceOccurrenceKey is never reconstructed from path, line, same Field key, digest or “unique candidate”; only a managed continuous change chain can prove continuity. An observation gap suspends automation requiring that continuity and requires explicit rebind. stop first durably prevents new dispatch/consumption, then attempts in-flight cancellation; an unproved provider effect remains unknown.

## 14. Versioned coordination with D3/D4/D5, P1 producers, and later consumers

Fixed C already contains D3 wire12 plus current D4/D5 SourceObservation/Frontier/WriteProtection consumer afterimages; they remain unaccepted/unactivated candidates. This P1’s production-domain revision rules, SourceRevisionPlan, revision-token profile/2, concrete DependencyKey closure, ContentCompletionProof/3, ConflictRecord/2, and U5 range/frontier refinement are not yet consumed by every owner. Existing A/B/C therefore does not automatically support these producer rules.

Remaining P1 must have D6 Control freeze closed types/decoders, Lexicon/Registry/Impact synchronize terminology/acceptance coverage, and PROPOSAL/replacements record real changedSections/versionChanges. P2 updates D3/D4 version/range-key/recovery consumption; P3 updates D5 locator/cut rules. Until those owner afterimages exist, affected strong paths remain owner_update_required/proof_unavailable. Existing ordinary file read, Draft, fully qualified human whole-source save, and local offline operations not depending on a missing strong range are not permanently disabled.

D7 complete Query/Prepared/Effects, D8 ordinary editing/Draft/IME/Undo, D9 construction/import/export, and D10 approval/recipient-target-payload/sourceOccurrenceKey/Money/unknown remain later consumer gates. A Storage summary cannot invent their closed fields or promote partial/semantic_pending into strong success.

Legacy D6 wire1, Policy/1/2, SourceVersion/1, Frontier/1, InstallationNotice/1, ContentCompletionProof/1 and already-promised ContentCompletionProof/2, ConflictRecord/1, legacy revision-token profile, D7 PreparedActionBinding/1,/2, D8 PreparedEditBinding/1, and their saved/planned decisions/pins recover/replay under original decoder, bytes, authorization/retention/continuity. New /3, ConflictRecord/2, profile/2, and SourceRevisionPlan apply only to explicit new-version paths. Old receipts are never re-encoded, old r5 is never upgraded by new proof, legacy pins are never retroactively deleted, and equal hash never creates new qualification.

These producer revisions still require complete bilingual afterimages, positive/negative/unknown/recovery evidence, fresh joint full review, and coordinated acceptance. This Storage artifact, author self-check, or documentation CI does not activate them.

## 15. Acceptance boundary

D6-FA-r01 is an author-side partial coordinated candidate only. There is no product implementation, real filesystem conditional-replace/exclusive proof, Server multi-user/real-time conformance, 10k/100k/1M-file or tens-of-GB performance evidence, complete D3/D4/D5/D7/D8/D9/D10 consumer afterimage, or fresh independent joint review.

Documentation checks/CI prove only their named checks. A complete successor candidate requires fixed-S 49 inputs, every actual replacement owner, and the new D10 18 files to be reviewed together.
