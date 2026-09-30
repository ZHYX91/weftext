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

File installation evidence binds a trusted FileObjectBinding rather than a digest alone. The trusted internal binding includes backend identity, canonical relative path, backend object generation/identity when available, observationEpoch, byteLength, digest, and an absent/present branch. Digest provides integrity evidence; it is neither identity nor a CAS primitive.

External programs may directly edit .adoc and Resource files. Any of the following advances the affected observation epoch and invalidates stale SourceVersion, locator/map, PreparedIntent and cut:

- object identity/generation change;
- byte/length/metadata mismatch against the managed binding;
- watcher/journal gap;
- delete/replace/rename not explained by a known portable ChangeRecord;
- placeholder materialization/dematerialization;
- backend loss of event continuity.

Equal digest does not prove that A→B→A did not occur. After an observation gap the epoch advances even when final bytes equal the prior bytes. External bytes that fail strict UTF-8 or D2 parse remain the actual external raw bytes; Node identity remains but D2 projection is unavailable and the state is external_invalid. Core never replaces them with an old pin or index and never calls that state a Core author commit.

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

Deleting the entire index leaves identity/structure/policy, original decisions, ApprovalUse/Money unchanged. Rebuild never mints identities, consumes approvals or replays external effects.

### 3.3 Draft

Draft preserves the existing D8 session model: user proposal, input log, selection and base binding. It has no author revision, ChangeId, D3 locator authority, or committed status. Ordinary cache cleaning never silently removes dirty Draft. Durable save and portable publication are independently readable states; Draft persistence, worker success, HTTP 200, or sync upload never substitutes for them.

## 4. CommitDomain, ReplicaEpoch, ChangeId, Frontier and SourceVersion

Control Interfaces owns the exact closed wire shapes; this section freezes storage semantics.

CommitDomain distinguishes replica and server. A replica domain is uniquely WorkspaceRef+ReplicaEpoch; a server domain is WorkspaceRef+the hosted AuthorityInstanceId. The v2 operation-ledger key is WorkspaceId + canonical CommitDomain bytes + OperationId. Different domains may reuse the same OperationId UUID. D3/D6 protocol-owner collision remains enforced within one domain. Legacy v1 keys and D3 v9/v10/v11 saved decisions retain their original decoder and continuity rules and are not migrated into v2 keys.

ReplicaEpoch is minted only by Core during explicit replica registration, is a canonical UUIDv4, and is never reused in the Workspace. Registration grants only ordinary content-domain qualification under current shared policy/trust. It is not D3 continue, AuthorityInstanceId, or global execution custody. Copying portable files does not create an epoch, and a retired epoch is never revived from copied metadata.

ChangeId is full CommitDomain + monotonic changeSequence. The first content change of a domain uses sequence 1; increment is checked and never wraps. ChangeRecord stores direct causal Frontier, actual write set, semantic state, and completion-proof binding. ChangeId is not EntityRef, OperationId, or source-occurrence identity.

Frontier is a canonical vector containing at most one maximum accepted ChangeId per CommitDomain, sorted by canonical domain bytes with no duplicates. Empty Frontier is valid only for genesis/no changes. Accepting a remote change creates a new local sync-admission decision/ChangeId with that remote ChangeId as a causal predecessor; the receiver never replays the remote OperationId into its own ledger.

SourceVersion/2 binds full EntityRef, CommitDomain, observationEpoch, source revision and the ChangeId that established the current source. revision increments only for managed source changes in that domain and does not increment for raw no-op. An unknown external change advances observationEpoch and produces an external observation branch; equal digest never revives the old SourceVersion. Numeric revisions in different CommitDomains are incomparable.

## 5. Authorization, ordinary save, and complete semantic qualification

### 5.1 Three independent qualification layers

D6 freezes:

1. ordinary replica content qualification — the actual source/identity/structure/policy scope touched by ordinary source/create/move/reorder/Trash plus an eligible installation backend;
2. complete semantic/action qualification — the complete positive/negative ranges, complete cut, authorization and semantics required by the particular D4/D5/D7 action;
3. global execution responsibility — continuous single responsibility for Automation, ApprovalUse, claim, Money, external unknown and stop.

Loss of global execution responsibility suspends only that execution capability, not ordinary content. A building index affects operations needing complete proof, not unrelated ordinary content. Conversely, ordinary content success never upgrades to complete Query/Action proof.

### 5.2 Ordinary proof scopes

Ordinary full-Document edit binds target SourceVersion/2, complete current source, entity lifecycle, current policy, actual MutationFootprint, complete D2 parse, and every actually modified local typed fact with its current Registry definition. Unmodified bytes are preserved exactly. If a touched transformation needs cross-object proof that is currently unavailable, an explicitly requested ordinary-save profile may produce semantic_pending rather than complete semantics; it may not be recorded as the original strong Action success.

create Node binds destination parent, full target sibling list, required ancestor/cycle proof, new identity reservation, complete new source, actual local typed admission, and old/new policy scope. move/reorder binds subject, old/new parent, both complete sibling lists, necessary ancestor/cycle proof, and old/new authorization scope. Ordinary Trash binds the selected subtree/owner-local closure, affected live/Trash sibling lists, restore membership and permission; it does not fabricate a complete workspace-wide inbound-reference enumeration.

restore, purge, relation mutation, unique Calendar/configuration, collection operations, all_result/bulk and other upstream-complete operations remain complete-qualified. A failed complete Action may not silently fall back into the same successful Action. The user may separately issue an ordinary source-save request, whose result is semantic_pending and does not make the strong Action successful.

Narrow Field qualification retains static independence proof and never reads hidden data first to decide permission. Structure qualification similarly requires structure-observation permission before reading hidden sibling lists.

### 5.3 SemanticState

Managed source state is:

- complete_semantics: D2 valid and all applicable D4/D5/structure/control semantics for the declared scope are completely proved;
- semantic_pending: D2 valid and actually modified local typed facts passed their local gates, while one or more cross-object/complete-range obligations remain unproved;
- external_invalid: current external bytes fail strict UTF-8 or D2 and do not correspond to a successful Core author decision; only Source/repair/raw-read paths may consume them.

semantic_pending may be the current source after an ordinary reliable save. It is not eligible for mutation that requires complete D4 relation/unique/Calendar invariants, D7 ActionEvidence/all_result/post-query over a complete cut, automatic Agent/automation writes, purge, or any export/audit claim that all workspace semantics are validated. Exact source read, Source editing, exact-file search and an explicitly local projection may consume it under their own permissions and must expose pending state.

This branch requires real later D4/D5/D7 owner afterimages. Until those consumers exist, managed semantic_pending success cannot be activated.

## 6. File installation qualification and reliable save

### 6.1 BackendQualification

Every FileBinding backend exposes trusted capabilities. Reliable replacement of an existing file is allowed only with one of:

- conditional_replace: a real compare-and-replace based on a trusted object generation/etag/file identity;
- exclusive_write_window: the host proves that every writer in the supported threat model is unable to modify/replace/delete the target from final before verification through completed installation; advisory-only locks do not qualify;
- create_only: atomic create-if-absent for an expected-absent target only.

The backend also proves durability of staged bytes, installed file data, and directory-entry/rename persistence. “Hash immediately before replace” is not conditional_replace. If safe installation cannot be proved, Core retains the current file and the Draft/after branch and returns conflict or install_unavailable; it never reports reliable save success.

Uncooperative external software may ignore Weftext coordination. The guarantee covers only the writer set actually proved by BackendQualification. If the backend cannot exclude a race, replacement of an existing file cannot succeed under the reliable-save profile. Any discovered competing bytes are preserved and never deleted or classified as ours merely because they equal an after digest.

### 6.2 Guarantee

ContentGuarantee is:

- replica_local: the fixed write set in this CommitDomain was safely installed and durably sealed; it says nothing about concurrent offline versions on another replica;
- managed_atomic: every managed author/portable-control write of the operation was protected by one managed installation/publication barrier and managed readers observe the old or sealed new cut.

managed_atomic never claims that multiple renames form a filesystem-wide transaction. External file readers may observe physical intermediate states; the product must not advertise instantaneous cross-file snapshots to arbitrary external tools.

## 7. Prepared input, installation, seal and portable publication

### 7.1 InputDescriptor and pins

The v2 canonical commit request remains a small control request and never embeds complete Document/Resource bytes. PreparedIntent/2 stores InputDescriptor/2 containing exact Workspace/CommitDomain, intent kind, before SourceVersion/metadata versions, expected Frontier, scope/profile, write-set descriptors, Registry/policy/rule versions and owner-specific canonical input. Complete source/bytes are stored only in the purpose-bound PinDirectory.

Input equality requires complete canonical descriptor equality plus equality of the exact referenced pins/owner inputs. Equal SHA-256 is not sufficient. A source, mapping, policy, frontier, scope or owner-request change requires a new prepare/OperationId unless the owner protocol explicitly defines canonical replay.

### 7.2 Pin classes and retention

Every pin records purpose, owning request/job, byteLength, payload type, source version, creation control revision, last-reference policy, retention class and capacity account.

Long-lived protection is permitted only for planned/installation recovery before/after, unresolved conflict heads and merge base, frozen external-unknown/approval/Money evidence, owner-protocol ImportJob/ExportPlan/Query pins, and explicit user history/backup policy.

Opening, parsing or indexing a workspace never creates permanent pins for every current source. Once no decision/unknown/conflict/user retention references large effect bytes and retention permits release, those bytes may expire while canonical request, decision, receipt/error, approval/Money/claim responsibility and the minimal input descriptor remain durable. Saved decisions created under legacy contracts that promised decision-lifetime/permanent pins retain the legacy promise; D6-FA-r01 does not retroactively delete them. Reading legitimately expired new historical effects returns effects_unavailable and never substitutes current files for historical after bytes.

Capacity exhaustion rejects or pauses new prepare/install instead of deleting a protected last-reference pin.

### 7.3 State machine and ordering

Each v2 content decision stores independent DecisionState, InstallationState, ReliableSaveState and PortablePublicationState:

DecisionState: unseen → rejected | planned → committed | terminal_failed.
InstallationState: prepared → planned → installing(k) → installed, plus conflict | recovery_unknown | paused_authorization | paused_capacity.
ReliableSaveState: not_saved | reliable.
PortablePublicationState: not_published | pending | published | conflict.

The only order is:

1. prepare freezes InputDescriptor, exact write set, before/after pins, semantic proof, preview and budget; no author effect;
2. planning proves required pins durable/capacity-reserved, then one durable-control transaction stores canonical request, fixed plan, reservations, installation recovery description and planned;
3. before any portable-current modification, write and durably flush an InstallationNotice containing ChangeId, operation binding, guarantee, before Frontier and component/write-set descriptors; it contains no approval/Money/external payload and is not proof of success;
4. install each fully written/flushed staged after only through create_only/conditional_replace/exclusive window, recording the actual FileObjectBinding/outcome; move/Trash first makes the recoverable destination durable and never permanently deletes current first;
5. installed verification compares each written component against **the operation’s own fixed planned poststate**, not against the original before. Revalidate current authorization plus all unwritten positive/negative dependencies, policy, Registry/rules, relevant Frontier and concurrent control facts. Produced ChangeId/SourceVersion are compared to the fixed planned poststate;
6. seal, only when every written component equals planned poststate and unwritten dependencies still hold. One durable-control transaction writes committed, canonical receipt, ReliableSaveState=reliable, effects metadata, execution charges/approval consumption where applicable, and outbox. This durable P seal is the decision commit point;
7. portable publication derives the immutable ContentCompletionProof from the saved decision, durably writes it into portable metadata, advances Frontier, releases the portable publication barrier, and sets published.

If step 6 succeeds and step 7 fails, the decision and reliable save remain successful while portable publication is pending. Recovery publishes the same completion proof only; it never rewrites source, creates a new OperationId, or charges again. Other replicas treat files that arrive before the complete proof/components as incomplete transport.

Revocation before seal prevents seal. If exact before can be safely restored under BackendQualification, restore and flush it. If safe restoration cannot be proved, retain before/after/current/recovery evidence and enter paused_authorization or recovery_unknown. Revocation does not turn unknown installation into permanent business rejection.

## 8. Crash recovery and historical results

Recovery first reads durable-control decision state, then installation notice/write-set bindings/actual files. Client timeout, file presence, mtime, digest or provider status never substitutes for P.

Each component is classified only as exact_before, exact_after with proved installation provenance, third_state, or unavailable. Equal bytes without installation provenance cannot be upgraded from third/unavailable to exact_after.

Rules:

- no planned: clean only proven-unreferenced staging;
- planned/not installed: recover the same plan/reservation, never resample identity;
- installing: when every component is provably before/after and installation lineage is continuous, recover the same plan; any third_state preserves current bytes/pins and enters conflict/recovery_unknown;
- all after but seal unknown: read P. committed replays receipt; planned resumes the original plan and never guesses committed from files;
- committed response lost: after current replay authorization, return original receipt without rewriting files/Frontier or charging again;
- committed with portable publication pending: recover only ContentCompletionProof;
- index/outbox failure: rebuild/catch up without rolling back the author decision.

Historical receipt r5 and current source r6 are distinct: replay r5 returns saved r5 bytes while current read returns r6 SourceVersion/Frontier. Undo/restore is a new plan/preview and receipt replay never rolls current backward.

## 9. Synchronization, admission, and conflicts

### 9.1 Replica registration is not execution takeover

A new device verifies complete portable Workspace identity/metadata chain/policy/trust and available current components, then explicitly registers a new ReplicaEpoch. Registration establishes only an ordinary-content CommitDomain. It does not take over ApprovalUse, claim, Money, external unknown or Automation lease and does not invoke D3 continue_workspace.

Execution takeover separately proves complete continuity of decisions/receipts/charges/unknown/claims/standing approvals/stop and proves the former execution holder is fenced. Failure suspends that execution capability only; ordinary replica content continues.

Loss of P never reconstructs execution decisions/unknown from notes. A portable InstallationNotice may force reconciliation of its affected range, but unrelated ordinary source remains usable in a new ReplicaEpoch. Creating an empty control DB never refunds Money, restores an approval, or resends an external request.

### 9.2 Transport completeness

Sync providers transport ordinary files and immutable/versioned portable metadata only; they never transport active control DB/WAL/SHM, derived index or Draft.

A receiver admits a ChangeId into its local Frontier only after InstallationNotice, ContentCompletionProof, and every listed component have arrived and mutually validate. Document-before-sidecar, sidecar-before-Resource, or unmaterialized placeholders are incomplete, not empty/deleted/committed.

A placeholder state is not_materialized. Operations needing bytes return source_unavailable/owner-specific unavailable; size zero or not_found is never substituted.

### 9.3 ConflictRecord

Concurrent source, placement, lifecycle, identity or policy heads create a stable ConflictRecord. ConflictKey binds WorkspaceRef, a closed conflict kind, the affected Ref set and all concurrent head ChangeIds. Refs and heads are unique and canonically sorted. ConflictId is a domain-separated SHA-256 address over canonical ConflictKey, while the complete key is retained and compared; the digest is not the evidence.

state is open → resolution_prepared → resolved. A new head supersedes the prepared key and creates a linked successor conflict. Resolution binds the exact current key/heads and an owner-specific plan; final write is compiled into the original D3/D6 typed request. There is no conflict bypass transaction.

Required cases include source/source, create/create ordering, move/edit, move/move cycle, Trash/edit, purge/restore, duplicate Ref/birth mismatch, policy conflict, partial sidecar/body arrival, and placeholder. No conflict freezes the entire Workspace: explicitly selected branches may continue to receive descendants, while an ambiguous ordinary read never chooses a random head.

### 9.4 Trash and purge

Multi-device ordinary delete means Trash. A local Trash receipt proves the known closure, owner-local membership, placement and actual source effects of that operation and does not fabricate a complete workspace-wide inbound-reference inventory.

Permanent purge remains strong: complete related inbound/owner closure, complete semantic proof, and acknowledgment of the purge Frontier by registered replicas, or explicit retirement of replicas that cannot participate. A retired replica returning later enters conflict/reconciliation and never revives its epoch or tombstoned identity. Purge does not promise secure erasure of historical bytes already copied to another device/backup.

## 10. Index/query completeness and large-workspace startup

Startup milestones are separate:

1. T_first_open: read workspace-root/portable-metadata entry and list discovered/active targets;
2. T_first_edit: open active source and Draft;
3. T_first_reliable_save: active target completes backend-qualified install+seal and reports reliable;
4. T_full_search_ready: complete coverage exists for the specified search profile/range;
5. T_OCR_ready: selected attachment/model/version OCR is complete or explicitly failed.

Inventory, metadata, D2 parse, typed index, search candidate, extraction and OCR use bounded byte queues, bounded concurrency, batched index transactions and resumable checkpoints. 10k/100k/1M small files and tens-of-GB attachment/body cases require real benchmark evidence; this architecture claims no seconds-level target.

Exact source scan is the correctness baseline. Candidate-index hits are re-read from the exact file/source version before final evaluation. A tokenizer/gram with incomplete recall for exact/NFC/regex cannot supply completeness. D7 complete Query still needs complete authorized execution and negative-range dependency proof; a building/partial index is only an explicitly partial exploration surface and cannot issue complete ResultHandle/ActionEvidence.

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

## 14. Versioned coordination with D3/D4/D5

Fixed-S D3 wire11, ledger key, stage order, continue/fork and Trash/purge/receipt remain the current historical contract. This candidate requires a later D3 wire12 to carry CommitDomain, replica-local profile, new ledger key, local Trash receipt, purge Frontier and legacy replay. D6 does not route identity/lifecycle through a generic bypass. No D3-v12-dependent managed success may activate before the D3 afterimage exists.

Existing complete D4/D5 operation gates are not silently satisfied by semantic_pending. This candidate only freezes storage’s ability to persist a D2-valid pending source. Which D4/D5 facts admit local-only proof and which remain complete must be specified by later owner afterimages. Until then, dependent Action/automation paths remain unavailable.

Legacy D6 wire1, Policy/1/2, SourceVersion/1, D7 PreparedActionBinding/1,/2, D8 PreparedEditBinding/1 and their saved decisions/pins recover and replay under original decoder/retention/continuity rules. New retention does not modify saved legacy bytes or retroactively remove evidence promised by the older contract.

## 15. Acceptance boundary

D6-FA-r01 is an author-side partial coordinated candidate only. There is no product implementation, real filesystem conditional-replace/exclusive proof, Server multi-user/real-time conformance, 10k/100k/1M-file or tens-of-GB performance evidence, complete D3/D4/D5/D7/D8/D9/D10 consumer afterimage, or fresh independent joint review.

Documentation checks/CI prove only their named checks. A complete successor candidate requires fixed-S 49 inputs, every actual replacement owner, and the new D10 18 files to be reviewed together.
