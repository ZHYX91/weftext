---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: `3762120f-fb70-4cf7-9699-68602e9bf9fc`.

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Historical revision05/D7/D9 acceptance prose in fixed S is provenance only. The stable document ID is preserved. This file defines future implementation/acceptance obligations only; it authorizes no code, dependency, release, deployment, or A2 work.

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

- Create off-workspace control.sqlite3, WAL/SHM and private pin area.
- Fully support original D6 wire1, Policy/1/2, legacy Prepared/saved decision decoders/replay; never upgrade saved records.
- Candidate v2 schema may be exercised in architecture fixtures, but product capability remains not_in_release/unsupported_version.
- P-loss tests must prove that old receipt, ApprovalUse, Money and unknown are not synthesized from portable/current files.

### S3 Portable metadata + replica model

- Implement ReplicaRecord, CommitDomain, ChangeId/Frontier, SourceVersion/2, InstallationNotice, ContentCompletionProof and ConflictRecord.
- child order has one ordered-list owner and deterministic move/reorder changes.
- A new device registers a new ReplicaEpoch; retired epochs never revive.
- Remote change admission creates a local ChangeId and never replays the remote OperationId.

### S4 Safe file installation

Implement and prove at least one qualified existing-file primitive: conditional_replace or a truly exclusive_write_window. Advisory locking is not an acceptable test substitute. New files use create_only.

Fault injection covers:
- before staged write;
- before/after staged data flush;
- before/after InstallationNotice flush;
- before/after every component install;
- file data flush;
- directory-entry flush;
- installed verification;
- before/during/after P seal transaction;
- before/after ContentCompletionProof write/flush;
- lost response transport.

Every point proves no lost competing bytes, no duplicate charge, no duplicate identity and no inference of success from unknown.

### S5 Ordinary save and semantic pending

- D6 source-save ordinary/complete profiles;
- separation of local typed gates from complete obligations;
- semantic_pending consumer-deny matrix;
- external observation epoch and ABA;
- r5 replay/current r6 separation;
- installed write set verified against planned poststate while unwritten dependencies remain compared with before/cut.

Until D4/D5 consumer afterimages exist, semantic_pending remains candidate-only and unavailable in product capability.

### S6 Sync/conflict

- two offline replicas: source/source, create/create, move/edit, move/move, Trash/edit, policy conflict, partial transport, placeholder, identity collision;
- stable conflictId and new-head superseding prepared resolution;
- source merge preserves original bytes; D3-owned placement/lifecycle resolution returns owner_update_required until D3 wire12 exists;
- purge waits for complete inbound proof plus registered-replica Frontier acknowledgement or retirement.

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
- T_first_reliable_save — active target completes safe install + P seal;
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

All rows are future evidence obligations, not passed tests.

| ID | Scenario | Required result |
|---|---|---|
| FA01 | I deleted; 100k unrelated docs unparsed; edit one D2-valid note | T_first_reliable_save can complete without unrelated index/OCR; complete Action still waits for proof |
| FA02 | existing target offers only “hash then replace” | no reliable commit; preserve current+Draft/after and return install_unavailable |
| FA03 | third party writes B before conditional replace; planned before=A | never overwrite B; conflict/paused with A/B/after evidence |
| FA04 | external A→B→A plus watcher gap | observationEpoch increments; stale map/locator/prepared/evidence invalid |
| FA05 | after install succeeds but implementation compares written target to before | test fails that implementation; conforming code compares planned after for written targets |
| FA06 | P seal succeeds; ContentCompletionProof write fails | reliable receipt succeeds; publication pending; retry only publishes proof |
| FA07 | lost response; r5 committed; later r6 edit | replay r5 returns original receipt; current remains r6; no rewrite/recharge |
| FA08 | two offline devices edit same note differently | source_concurrent with both heads; no LWW; both original bytes recoverable |
| FA09 | A moves Node; B edits its source | combine only if dimensions independently prove compatible and policy/structure revalidate; else conflict |
| FA10 | A Trash; B edit | no implicit delete-wins; preserve edit branch and Trash intent |
| FA11 | purge vs old-replica restore | tombstone never revives; old bytes are fresh-copy/reconciliation input only |
| FA12 | child_list/Document/CompletionProof arrive in parts | incomplete; no new identity; Frontier does not advance |
| FA13 | placeholder has metadata but bytes not downloaded | not_materialized/source_unavailable, never empty/not_found |
| FA14 | duplicate Ref with different birth | identity_collision; resolver never chooses first; D3 owner resolves |
| FA15 | building index misses one relation target | complete Query/Action rejects or scans fully; empty-range success forbidden |
| FA16 | semantic_pending source enters automatic Action | complete consumer rejects; Source/explicit local read still allowed |
| FA17 | P lost, portable files intact | new ReplicaEpoch may handle unrelated ordinary content; old approval/Money/unknown not reconstructed |
| FA18 | I lost, P intact | rebuild I without replaying decisions, minting Refs or changing charge |
| FA19 | copied P database opened on two devices | never creates two execution holders; takeover needs continuity and fencing |
| FA20 | single-use approval, network result unknown, crash | restart recovers original request/provider evidence; no new OperationId/second consume |
| FA21 | work batch charged then crash | charge remains; retry does not refund; exhausted work stays paused |
| FA22 | Server Alice/Bob edit different Documents | both Drafts/prepares run concurrently; each commits once; SQLite writer serialization never becomes UI exclusivity |
| FA23 | Server Alice/Bob same Document, same old base | first seal succeeds; second keeps Draft and becomes stale/conflict, never overwrites |
| FA24 | Bob revocation races collaboration checkpoint seal | unique linearization; if revocation wins, Bob’s uncommitted contribution cannot commit under Alice’s identity |
| FA25 | Server failover fences DB but not file writes | conformance failure; old instance must be unable to rename/write author files |
| FA26 | collaboration receive ack without checkpoint | UI/API must not report reliable save/committed |
| FA27 | IME preedit events then final input | preedit never becomes author op; final confirmation is at most one Draft/checkpoint input transaction |
| FA28 | protected conflict pins hit capacity | restrict new work; never delete last-reference conflict evidence |
| FA29 | new historical effect pin legitimately expired | effects_unavailable; current file never impersonates old after |
| FA30 | legacy wire1 saved decision | byte-equivalent original replay and original pin promise survive v2 retention |

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

Tests retain:

- D3 v9/v10/v11 historical saved requests/receipts/errors;
- D6 wire1 commit/receipt/error;
- Policy/1/2;
- SourceVersion/1;
- D7 PreparedActionBinding/1,/2;
- D8 PreparedEditBinding/1;
- existing Result/ByteHandle historical tokens.

A v2 fixture is never obtained by changing only wireVersion. It is independently constructed from this closed schema. old→new implicit upgrade and new→old fallback both fail. Unknown owner version returns unsupported_version/owner_update_required and never approximates an old path.

Legacy canonical requests/pins remain legacy evidence even if they carry large source material. PreparedIntent/2 uses InputDescriptor plus purpose-bound PinRef. Tests prove that equal hash with different SourceVersion/owner descriptor is not the same input.

## 8. Downstream consumption gate

The corresponding new path remains unavailable until these real afterimages exist and are jointly accepted:

- D3 wire12 — CommitDomain ledger, replica-local create/move/Trash, purge Frontier, legacy replay;
- D4 — semantic_pending local-vs-complete typed obligation matrix;
- D5 — native structure/collection/bulk local-vs-complete gates;
- D7 — domain/frontier cut, PreparedActionBinding/3, new EffectManifest retention, Action evidence;
- D8 — Source/Live/Read, reliable/portable state, conflict and collaboration session;
- D9 — ImportJob/ExportPlan new pins/version;
- D10 — sourceOccurrenceKey continuity, recipient/target/payload approval, Money/claim/stop execution responsibility.

The mere existence of D6 documents never means these consumers already support the new semantics.

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

## 10. Fresh review gate

The complete coordinated candidate requires a new independent review from scratch over the new proposal, every replacement owner, all 18 D10 files, and the fixed S49 inputs. The author’s current 16/49 full-read plus partial-read provenance does not inherit an older independent 49/49 pass.

The old B13 result remains REVISE, terminology/bilingual FAIL, P0=0/P1=3/P2=8, eleven OPEN findings. This batch only provides foundational related surfaces and closes/reclassifies none.

