---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 042833a6-ecb2-4909-8712-ce4ec464ff34.

# D1 Implementation Impact and Test Outline

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. The fixed-S D1 Implementation Impact is historical source material. This file defines future implementation/acceptance obligations and authorizes no implementation, dependency change, release, or A2.

## 1. Impact scope

D1-FA changes execution modes and state ownership without creating new domain objects. Major future slices are:

| Slice | Future responsibility | Forbidden inference |
|---|---|---|
| local file replica | occupancy, backend qualification, external-change invalidation, replicaEpoch admission | one logical Workspace has only one device |
| multi-replica sync | transport F/M, complete-record adoption, conflict presentation | sync provider is transaction or execution authority |
| Server | multiple sessions, scoped concurrency, one durable commit holder, fencing both files and P | SQLite single writer means single user |
| large-workspace startup | active document first, progressive I, layered parser/search/OCR | editing waits for full indexing |
| real-time collaboration | post-G2 session input, broadcast, checkpoint interfaces | this batch already delivers OT/CRDT |

## 2. Product state machine and UI projection

Every surface distinguishes these milestones and states:

- T_first_open: enter the workspace, browse discovered structure, and open a materialized target.
- T_first_edit: active Draft accepts input; this is not file save.
- input retained: proposal, actually read before image, and required bindings are durably retained while file installation may not have happened.
- T_first_reliable_save: reached only after a strict path completes qualified file installation and D6 P seal; observed_only never populates it.
- durable_observed_only: only for the approved trusted-interactive ordinary save of one existing live Document; it proves durable input/read-before retention plus seal but does not promise exclusion of an external race never observed.
- recovery_unknown: installation occurrence or provenance cannot be proved; the UI never shows save success.
- portable publication pending/published: describes publication of an already sealed result and never rewrites the receipt.
- T_full_search_ready: declared complete search scope is proved by complete scan or qualified candidate index plus source reread.
- T_OCR_ready: OCR work for selected attachments/model/configuration is complete with explicit failures.

Sync upload, Preview, Prepared, worker success, HTTP success, Draft persistence, and P planned never project as file-save success. Product wording does not expose P-ledger internals and observed_only adds no per-save approval prompt; stable help text explains only its concurrency-protection difference.

## 3. Local implementation obligations

Desktop/CLI/Mobile local hosts must:

1. provide process-level exclusive commit qualification for one physical replica;
2. keep active P/I outside the synchronized directory;
3. turn external file changes into D6 observations instead of UI merges;
4. retain Draft/after and return install unavailable when a strict request lacks conditional/exclusive installation capability; only trusted-interactive ordinary source-save of one existing live Document with complete source read/replace, zero-or-one source write set, and no narrow deny may use a new observed_only prepare, never an in-place strict downgrade;
5. invalidate only affected SourceVersion, cut, prepare, and installation ranges, allowing unrelated ordinary work after requalification;
6. rebuild I progressively from F/M without reminting identity or inventing decisions;
7. preserve provable ordinary content when P is lost, while pausing unrecoverable execution responsibility instead of guessing receipts from files.

## 4. Server multi-user implementation obligations

Server must support multiple sessions and one durable commit boundary together:

- every authenticated principal has an independent authorization context and Draft/selection/session;
- one document may have multiple Drafts and unrelated-document reads/prepares may run concurrently;
- durable checkpoints/commits serialize only on actual write/dependency scope;
- an old Base never overwrites a newly published source; conflict retains all uncommitted input;
- revocation is rechecked before checkpoint and another participant cannot donate authority;
- broadcast originates only from managed results/outbox and recipients still pass read gates;
- failover fences P and author-file writes together;
- serialized SQL transactions are not a frontend user lock.

## 5. Multi-replica synchronization obligations

A sync provider transports only ordinary files and portable metadata records. A receiver distinguishes:

- complete ChangeId plus ContentCompletionProof;
- InstallationNotice or partial components without completion proof;
- placeholder/not_materialized;
- concurrent source/placement/lifecycle heads for one subject;
- identity collision;
- retired-replica content returning later.

No case is resolved automatically by mtime, path, filename, equal digest, or last uploader.

## 6. Future real-time collaboration interface

Real-time collaboration remains post-G2. Before implementation it must provide:

- session epoch and participant identity;
- per-participant input sequence and exact Base;
- separation of IME preedit from final input;
- separation of transient broadcast from durable checkpoint;
- revocation, disconnect, reconnect, and gap reset;
- retained input with stopped checkpoint when rebase cannot be proved;
- one managed source proposal from the collaboration algorithm into Core.

D1-FA does not freeze the OT/CRDT algorithm, encoding, or optimization.

## 7. Large-workspace and index implementation outline

Startup does not hash the whole workspace or copy every current body/AST into I. Implementation uses:

- streaming directory/portable-metadata enumeration;
- active-document priority queues;
- bounded in-flight bytes, parsed objects, workers, and I batch transactions;
- resumable coverage/checkpoints;
- independently versioned metadata, parser, candidate-search, attachment-extraction, and OCR layers;
- source scan or explicit refusal when a complete Action lacks complete coverage.

Default token/FTS matches are candidates only; exact/NFC/regex final semantics belong to their owner and source reread.

## 8. Performance validation matrix

Future benchmarks include:

| Axis | Minimum set |
|---|---|
| file count | 10k, 100k, 1m small files |
| composition | tens-of-GB attachment-heavy and source-heavy workspaces |
| download state | fully local and on-demand sync |
| cache | cold and warm |
| hardware | recorded CPU, RAM, OS, filesystem, disk |
| lifecycle | first open, deleted I, existing workspace on new device, restart during build, single/batch modification |
| outputs | five time milestones, separate durable_observed_only latency, peak RAM, I/P/pin size, bytes read, incremental time |

These are acceptance targets, not existing performance results; no seconds-level or named-competitor speed claim is permitted without measurement.

## 9. Security and negative tests

Required cases include:

1. Desktop/CLI contention on one local replica with at most one reliable save.
2. Two registered replicas both saving ordinary content offline and later creating explicit conflicts rather than a global lock.
3. move/edit, Trash/edit, and source/metadata partial arrival.
4. portable metadata copying without duplicate global approval/Money consumption.
5. deleting I without losing identity, parent/order, policy, or receipt.
6. losing P without reconstructing external unknown, receipts, or Money from current source, while ordinary unambiguous content still opens.
7. two Server users on same/different documents with one durable order.
8. revocation racing commit with one canonical outcome.
9. failover that fences DB but not file writes failing conformance.
10. partial index never proving a complete Action.
11. no-body-replica checks over P/I/schema/FTS shadow state, rejecting whole-workspace current-body and full-AST mirrors.
12. any pre-G2 statement that real-time collaboration is available failing release evidence.
13. automatic Draft retention, background autosave timers, workers, or reconnect never trigger an observed_only author installation; a trusted interactive save intent is still required.
14. narrow Field/body permission never gains whole-source replacement through observed_only; complete whole-source replacement authority and every applicable deny are checked first.
15. observed external competition is stale/conflict; only an unobserved race is the approved observed_only limitation. Unknown install is recovery_unknown and never inferred successful from hash/final bytes.
16. strong Action and bulk execution never use observed_only.
17. Automation, approval, or Money consumption never uses observed_only.
18. D3 create/move/reorder/Trash and restore/purge/copy/fork/import never use observed_only.

## 10. Cross-owner follow-up obligations

- D3: the G0-B wire12, DecisionKey/2, D3-native owner input, strict installation, companion/replay contract must stay synchronized with this file; its companion afterimage is produced in this candidate but remains inactive.
- D4: semantic_pending consumption for relation/unique/calendar/cross-object types.
- D5: local versus complete ranges for native/bulk/collection operations.
- D7: scoped Query/Action cuts, new Prepared version, effects/pin retention.
- D8: Source/Live/Read, multi-user Drafts, IME/Undo/selection, collaboration checkpoints.
- D9: scoped pins and new request/effects consumption for ImportJob/export/publish.
- D10: execution responsibility for approval/claim/Money/sourceOccurrenceKey/stop.

No new managed-success path may activate while these coordinated afterimages are incomplete.

## 11. Acceptance boundary

This file defines future implementation and validation only. Documentation CI, author self-review, compilability, or historical tests are not independent acceptance.

The old D10 B13 result remains REVISE, terminology/bilingual FAIL, with 3 P1 and 8 P2 findings, 11 OPEN in total.
