---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: a96b68da-72b7-434b-826e-e14d07c96077.

# D1 Product Surfaces and Capability Boundary

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. The fixed-S decision frozen on 2026-08-27 remains historical source material. This afterimage may activate only with all required replacement owners, D10 consumers, fresh independent joint review, and coordinated acceptance. It authorizes no code, dependency, release, deployment, or A2 work.

## 1. Scope, inputs, and version boundary

D1 still owns only the formal product surfaces, execution modes, capability ownership, process/trust/network boundaries, state ownership, and release sequencing. D2-D10 own their respective content, identity, type, structure, storage, Query/Action, editor, conversion, and external-capability protocols.

D6-FA-r01 makes five material D1 changes:

- Ordinary files carry current local-workspace content bytes, and external programs or file-sync services may modify or transport those bytes.
- One logical Workspace may have multiple registered physical replicas, each able to perform ordinary edit, create, move, reorder, and Trash work offline.
- The single-commit-holder invariant is scoped to each physical replica or hosted backend, not to one globally editable device per Workspace.
- Server remains multi-client and multi-principal, with concurrent editing sessions; durable commit ordering does not mean one human editor.
- Continuous execution responsibility for Automation, approval counts, Money, external unknowns, and stop is separate from ordinary content replicas, and file copies do not copy consumption authority.

This afterimage does not change the five formal product surfaces, advance real-time collaboration delivery, or expand the current release-platform claims.

## 2. Self-contained problem statement

Weftext must simultaneously provide:

1. A local product based on ordinary files without an account, Server, or network.
2. Multiple physical copies of one logical Workspace that can perform ordinary content work offline and later surface content, structure, lifecycle, and policy conflicts explicitly.
3. Hosted deployment through Server with the same Core semantics for Browser, Desktop, CLI, and Mobile while preserving simultaneous multi-user work.
4. No second author-write semantics in workers, Agents, automation, connectors, sync services, or indexes.
5. A globally consumable execution resource only when a continuous and fenced execution-responsibility domain proves authority; ordinary content availability is not conditioned on that domain being online.
6. Large workspaces may open, edit, and complete active-file save before metadata, full search, and OCR finish; strict reliable and observed_only durable_observed_only are presented and measured separately.

Critical failures include two processes believing they can commit the same physical replica, marketing file sync as collaboration, copying global spending authority with a replica, treating a partial index as a complete set, interpreting Server's single durable writer as a single-user product, presenting Draft/input retention or sync upload as file-save success, or presenting observed_only as strict reliable or strong-Action qualification.

## 3. Global invariants

| ID | Rule |
|---|---|
| D1-I01 | Each physical local replica has at most one durable commit holder at a time. A hosted backend likewise has one durable commit holder across Server processes, instances, and overlapping restart windows. Different registered physical replicas may each make ordinary offline commits; that does not make them co-holders of one global execution authority. |
| D1-I02 | Desktop, WebUI, Server, CLI, and Mobile share Core domain results, diagnostics, plans, and commit semantics. A surface chooses interaction but cannot invent identity, conflict, or commit outcomes. |
| D1-I03 | WebUI is always a Server client. It never opens a local directory or holds a hosted path/database. |
| D1-I04 | Desktop, CLI, and Mobile call local Core in local mode and Server in remote mode. Local caches never reinterpret remote results. |
| D1-I05 | Ordinary sync services copy files and portable metadata only. An observed external change invalidates qualifications that depend on the old SourceObservation, observation epoch, or affected installation range. Core retains Draft/branches and reacquires local qualification. observed_only never overwrites an observed competitor, while one local change still does not make the entire replica permanently read-only. |
| D1-I06 | Workers, Agents, automation, and connectors may only produce inputs, results, or proposals. Final author commits still pass through local Core or Server Core. |
| D1-I07 | Capability state comes from versioned capability description, not UI visibility, OS detection, cache state, or a past success. |
| D1-I08 | Unsupported, undelivered, unconfigured, offline, incompatible, unauthorized, or policy-denied cases are explicit and never fall back to approximate writes. |
| D1-I09 | Platform support claims require actual build, install, launch, and scenario evidence. |
| D1-I10 | Current public artifacts have no production platform signing/notarization/store identity; development signing is not a release capability. |
| D1-I11 | Product state distinguishes input retained, not_saved, reliable, durable_observed_only, recovery_unknown, and portable publication pending/published. observed_only is possible only for trusted interactive ordinary source-save of one existing live Document; automatic Draft retention never authorizes unattended weak installation. |

D1-I01 constrains who may durably modify a physical backend. It does not constrain how many users may concurrently hold Drafts, read, prepare, comment, or enter edits.

## 4. Product-surface responsibilities

### 4.1 Desktop

Desktop is the complete local file-workspace surface and an optional Server client. It owns user file authorization, windows, device Drafts, selections, and UI state, and calls either local Core or remote Server.

Local mode may open a registered replica and edit while offline. Arriving sync changes are handed to Core/D6 coordination. Desktop WebView never directly modifies workspace files, portable metadata, P, or I.

### 4.2 WebUI

WebUI works only through Server. Browser cache, Draft, selection, cursor, and presence are not workspace authority. When disconnected it may retain an explicitly uncommitted Draft, but it cannot claim a hosted save.

### 4.3 Server

Server is the unique online durable-write boundary for hosted workspaces. It owns authenticated sessions, authorization enforcement, audit, hosted backup/recovery, concurrency coordination, WebUI, and versioned client APIs, and invokes the same Core.

Commit qualification for one hosted backend is unique across all Server processes and instances. An instance without qualification rejects writes or remains read-only. This restriction applies to durable commit, not to user sessions. Multiple users may concurrently open the same or different documents and hold Draft, read, prepare, and interaction state.

Reads, parsing, and preparation for unrelated documents may proceed concurrently. Durable commits serialize only across their actual target/dependency requirements. Two ordinary non-real-time sessions may edit the same document: after one saves, the other keeps its Draft and enters stale/conflict handling against the new Base.

Real-time collaborative text editing remains a post-G2 capability. Server will coordinate session input, broadcasts, and checkpoints while durable results still pass Core. D6-FA-r01 freezes no OT/CRDT algorithm, and a collaboration log never becomes a second author source.

### 4.4 CLI

CLI supports both local and remote modes. It can run the same Core ordinary operations on a local file replica or call Server. Destructive, external, or conflict-resolution operations requiring confirmation need explicit inputs and cannot hide interactive approval.

### 4.5 Mobile

Mobile retains the same domain semantics. Its local backend exposes a write only when the required file-installation and durability properties are proven; otherwise it returns explicit unavailability. Remote Mobile remains a Server client. Existing conversion, Agent, automation, and connector exclusions are not expanded by this reopen.

## 5. Platform and capability matrix

| Capability | Desktop | WebUI | Server | CLI | Mobile |
|---|---|---|---|---|---|
| Local file-workspace Core read/write | L | — | — | L | L |
| Hosted Core read/write | R | R | H | R | R |
| Ordinary edit, Query, View, Action | L/R | R | H | L/R | L/R |
| Conflict presentation after file sync | L | — | — | L | L |
| Server multi-user sessions | R | R | H | R | R |
| Real-time collaboration, presence, cursors | R (post-G2) | R (post-G2) | H (post-G2) | — | R (post-G2) |
| Local rebuildable index | L | non-authoritative browser cache | H derived cache | L | L |
| Local backup/recovery | L | — | — | L | L |
| Hosted backup/recovery | — | I | H | I | — |
| Automation/Agent/connector | original D1 boundary | original D1 boundary | original D1 boundary | original D1 boundary | original D1 boundary |
| Direct write bypassing Core | — | — | — | — | — |

L/R/H/I retain their fixed-S meanings: locally hosted, remote client, Server hosted, and initiate/manage only.

## 6. Execution modes and state ownership

| Mode | Durable commit holder | Non-authoritative state | Forbidden interpretation |
|---|---|---|---|
| Desktop/CLI/Mobile local replica | one Core plus eligible file backend for that physical replica | Draft, I, UI, sync progress | two processes reliably saving the same replica |
| Fully offline local | same | network capabilities unavailable | blocking unrelated ordinary edits because execution custody is absent |
| Multi-device file copy | one local commit holder per registered physical replica | sync provider transports F/M only | file copy is a cross-device atomic transaction or copies Money authority |
| Server hosted | Server Core qualified for the hosted backend | client Draft/cache, Server session/presence | one backend commit sequence means one user session |
| Server real-time collaboration | Server Core remains durable authority | collaborative input stream and transient checkpoints | OT/CRDT/operation log automatically becomes author source |
| Remote offline | Server remains hosted authority | client non-authoritative Draft | calling the Draft saved or synchronized |
| worker/Agent/automation | initiating Core/Server | task state, secret references, transcript | external result directly writes files or P |

A local file replica cannot simultaneously be treated as writable by two local Core processes, or be directly opened for writing by a local client while it is a Server-hosted backend.

## 7. Deployment and process topology

### 7.1 Local replica

UI or CLI -> local host -> Core -> file backend F/M. The same host keeps P outside the synchronized workspace and I as discardable local state. A sync program only sees approved ordinary files and portable metadata, never active P/I/WAL/SHM.

### 7.2 Multi-replica sync

Device A and B have independent replicaEpoch values and local commit domains. The sync service transports files and complete portable records. A receiver validates the completion proof, actual components, and conflicts before adoption. It neither replays the sender's OperationId nor imports the sender's execution responsibility.

### 7.3 Hosted Server

Browser/Desktop/CLI/Mobile -> authenticated Server API -> authorization/audit -> Core -> hosted file backend plus Server P/I. Many frontend connections and Drafts may coexist; only Core-sealed managed results enter the unique durable commit sequence, and neither Draft nor broadcast acknowledgement is save success.

### 7.4 Failover

Server failover fences both control storage and author-file writes. Fencing only SQLite while the old instance can still rename or replace hosted files does not satisfy unique commit ownership.

## 8. Trust, authorization, and network boundary

Authorization is revalidated at the boundary that holds the workspace backend. Client claims of authorization, sync-provider upload state, and index completeness are not authorization evidence.

An ordinary local operation observes only the Source, identity/lifecycle, structure, and policy ranges it actually needs. Strong Actions still prove their required complete positive and negative ranges under D4-D7. An incomplete index may be supplemented by source scan; inability to prove a required complete range blocks that strong operation, not unrelated ordinary saves.

Revocation and commit are linearized by the authority-holding boundary. In Server collaboration, every participant's uncommitted contribution is rechecked before a durable checkpoint. One participant's permission cannot authorize another participant's input.

## 9. Capability ownership

Core continues to own domain interpretation, planning, validation, and commit. D6 owns F/M/P/I physical/control contracts, WriteProtection, ReliableSaveState, input retention, portable publication, conflict records, and execution-responsibility continuity. D3 owns identity, parent/order, and lifecycle. Server control plane owns authentication, accounts, sessions, audit, and collaboration coordination. No database or sync provider becomes a domain owner.

## 10. Capability negotiation and unavailable semantics

The original D1 capability negotiation and unavailable-reason ordering remain. D6 workspace-specific outcomes such as workspace busy, domain unavailable, install unavailable, conflict, and owner update required are operation results, not new D1 capability reasons.

Capability probing is not an authorization ticket. Actual commit revalidates current policy, backend qualification, SourceObservation, Frontier/2, and required dependencies. A strict request never downgrades in place to observed_only.

## 11. Coordinated versions and release claims

D6-FA-r01 is a design generation and does not itself change the current release support matrix. New D1/D3/D6/D7/D8/D9/D10 afterimages enter a product contract major only after complete coordinated acceptance. This batch does not use a generic "contractMajor 2" to bypass concrete wire/version consumption.

Existing G1/G1.1/G2 platform evidence gates remain. Without real collaboration, performance, and fault-injection evidence, release notes state that those capabilities are not delivered.

## 12. Release dependency order and real-time collaboration schedule

The existing order remains:

shared Core -> G1 local release -> G1.1 CLI and Server security base -> WebUI -> G2 hosted release -> real-time collaboration -> Mobile remote capabilities.

Real-time collaboration does not block a safe non-real-time Server/WebUI release, and file synchronization never substitutes for it. D6-FA-r01 freezes only the commit, authorization, Draft, and conflict boundaries into which future collaboration must plug; it does not implement OT/CRDT.

## 13. Explicit non-goals

- Do not market a synchronized folder as real-time collaboration.
- Do not require all replicas online before ordinary editing.
- Do not let an unregistered or retired replica silently obtain execution responsibility.
- Do not freeze automatic LWW, mtime winners, or any CRDT/OT algorithm here.
- Do not turn each keystroke into an author commit.
- Do not turn WebUI into a local-file PWA.
- Do not expand current platform, signing, or store claims.
- Do not make complete indexing or OCR a universal prerequisite for opening or ordinary file save, and never count weak-protection latency as the strict reliable-save metric.

## 14. Rejected alternatives

| Alternative | Decision | Reason |
|---|---|---|
| one globally writable device per Workspace | reject | violates confirmed multi-device offline ordinary content |
| multiple writer processes for one physical replica | reject | no deterministic installation/version/recovery boundary |
| file sync is collaboration | reject | lacks principal, authorization, commit order, presence, and deterministic conflict protocol |
| Server single writer means single user | reject | durable ordering and frontend session concurrency are distinct layers |
| browser offline Draft later becomes a Server commit automatically | reject | bypasses current Server authorization/version/conflict gates |
| portable metadata copy also copies global quota | reject | content replicas and execution responsibility are separate |
| entire workspace read-only until indexing completes | reject | unrelated ranges must not block ordinary local operations |
| silent last-writer-wins | reject | loses concurrent bytes, identity, or user intent |

## 15. End-to-end scenarios and minimal counterexamples

### S1 Local offline edit

Desktop opens an active document offline and edits it. With strict installation qualification it may show “reliably saved”. With observed_only, and only when trusted-interactive, one-existing-live-Document, complete read/whole-source replace, zero-or-one source write set, and no narrow deny all hold, it may show “saved · ordinary file mode” with a stable warning that another program may still write concurrently. Observed conflict, unknown installation, or failed eligibility retains input and shows conflict/recovery-pending/unavailable. Automatic Draft retention never triggers unattended observed_only installation.

### S2 Two local processes

Desktop and CLI contend for one physical replica. Only one obtains commit-holder qualification; the other fails or remains read-only. Both reporting reliable save is a counterexample.

### S3 Two-device file sync

A and B edit the same note offline and later sync. Both sealed ordinary-save results retain their actual WriteProtection, and the receiver creates an explicit conflict instead of choosing by mtime or upgrading observed_only to reliable. If source arrives before metadata, the state is incomplete; missing metadata is neither fresh identity nor deletion.

### S4 WebUI offline Draft

Browser disconnection retains an explicitly uncommitted Draft. Reconnection re-enters coordination with current Server authorization and version. localStorage/IndexedDB is not a hosted commit.

### S5 Remote authorization change

Two users edit one hosted document. A commits while B is still uncommitted, then B loses write authorization. B keeps a local Draft but cannot submit through A's session or an old capability. Revocation never rewrites A's committed fact.

### S6 Different-document concurrency

A edits N1 while B edits N2. Read and preparation may run in parallel, and durable commits serialize only on actual write/dependency ranges. An N1 commit cannot erase an unrelated N2 Draft merely because a global counter changed.

### S7 Same-document non-real-time concurrency

A and B both edit revision r5. A saves r6 first. B's old Base is now an observed conflict and returns stale/conflict, retains B's Draft, and preserves the r5/r6 comparison. observed_only cannot overwrite A either.

### S8 Future real-time collaboration

Post-G2 participants may exchange transient collaborative input. Broadcast acknowledgement is not reliable save; a checkpoint becomes canonical only after Core validation. Until implemented, every surface reports the capability as not in the release.

### S9 Global execution responsibility

A and B both have file replicas, but only the continuous execution-responsibility holder may consume a given ApprovalUse/Money resource. B may still edit ordinary content and does not gain a second balance from file copying.

### S10 Large-workspace first open

A tens-of-GB workspace may reach T_first_open and T_first_edit first; a strict path may then reach T_first_reliable_save before T_full_search_ready and T_OCR_ready. observed_only latency to durable_observed_only is recorded separately and never populates T_first_reliable_save. No seconds-level guarantee is made without implementation benchmarks.

## 16. Inputs to D2-D10

| Owner | D1-FA input |
|---|---|
| D2 | exact-source/object semantics are the same across replicas/Server; external invalid bytes do not become valid Document because of path |
| D3 | identity and parent/order are path-independent; replica registration differs from Workspace continue; replica_local narrows semantic proof scope only, while D3 identity/structure/lifecycle installation remains strict |
| D4 | semantic_pending consumption is explicit and never equivalent to all constraints passing |
| D5 | native/collection/bulk complete-range gates are not satisfied by partial indexes |
| D6 | F/M/P/I/Draft separation, WriteProtection, ReliableSaveState, input retention, portable publication, conflict, replicaEpoch, execution responsibility |
| D7 | Query/Action binds CommitDomain/frontier; complete results require complete range proof |
| D8 | Source/Live/Read and Draft/IME/Undo/selection continuity; multi-session/checkpoint state is not a second author source |
| D9 | pins/import/export/publication follow the new domain/version and worker success is not author commit |
| D10 | approval/claim/Money/sourceOccurrenceKey/stop remain separate from ordinary content replicas and retain continuous responsibility |

## 17. Implementation impact graph

Future implementation touches local file-backend qualification and occupancy, portable replica registry, Server file-write fencing, range-scoped external-change invalidation, conflict presentation, D8 multi-session Draft binding, Server broadcast outbox, and layered large-workspace index state.

These are future implementation impacts and do not authorize product-source changes in this batch.

## 18. Acceptance and performance outline

Required evidence includes:

1. one durable commit holder for one physical replica under two-process contention;
2. two registered replicas both performing offline ordinary edits without copying execution responsibility;
3. external changes invalidating only relevant SourceVersion/cut/install ranges while unrelated notes can reacquire local qualification;
4. two Server users holding same- or different-document Drafts concurrently with unique durable commit ordering;
5. revocation, reconnect, restart, and failover never producing a second commit holder;
6. partial file-sync arrival, placeholders, and conflicts never pretending to be complete state;
7. separate measurement of T_first_open, T_first_edit, T_first_reliable_save, T_full_search_ready, and T_OCR_ready, with durable_observed_only save latency reported separately and excluded from T_first_reliable_save;
8. benchmark recording of file count, source bytes, attachment bytes, cold/warm cache, CPU, RAM, disk, peak memory, I/P size, and restart resume;
9. a no-body-replica assertion for P/I, including full-current-body and full-AST negatives;
10. explicit not-in-release behavior for unimplemented real-time collaboration rather than a file-sync fallback.

## 19. Candidate status and follow-up gate

This afterimage is the D1 owner candidate for D6-FA-r01 and is not independent acceptance. The historical frozen D1 decision remains source provenance only. New semantics require coordinated acceptance with D3/D6 and the later D4/D5/D7/D8/D9/D10 afterimages.

The old D10 B13 result remains REVISE with 3 P1 and 8 P2 findings, 11 OPEN in total. This document closes none of them.
