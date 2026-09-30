---
source_language: zh-CN
translation_of: PROPOSAL.zh-CN.md
translation_status: synced
---

[简体中文](PROPOSAL.zh-CN.md)

# D6-FA-r01 File Authority Reopen — Coordinated Candidate Note

Status: **candidate / partial coordinated candidate / not accepted / not activated / not implemented**.

This directory is the D6 file-authority reopen candidate workspace over fixed upstream S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`. The D10 author candidate was C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` immediately before this batch. These afterimages do not modify `docs/design/snapshots/`, `docs/design/inputs.json`, or fixed S. Activation is forbidden until all required owner afterimages, D10 consumers, fresh independent joint review, and coordinated acceptance are complete.

This file only routes differences and versions. Normative operational semantics belong exclusively to the relevant files under `owners/`; this note is not a second transaction, conflict, or permission protocol.

## 1. Decided product choices

1. Current Document exact source is uniquely carried by ordinary `.adoc` files; current Resource bytes are uniquely carried by ordinary resource files. External editors may modify them and ordinary sync providers may transport their bytes.
2. Device-local rebuildable indexes use a separate SQLite database outside the synchronized directory. It may be deleted and rebuilt and must not retain a full current-body or full-AST replica of the whole workspace.
3. Non-reconstructible execution control facts use a distinct durable SQLite control database outside the synchronized directory. It stores original decisions/receipts, transaction recovery, unknown external effects, approval consumption, claims, Money/budget responsibility, and necessary pins. It is not the document database.
4. Portable identity, parent/order, lifecycle, shared configuration/ACL/trust are uniquely owned by portable workspace metadata. They may not exist only in a discardable index, and the local control SQLite may not become a second writable current truth.
5. Multiple devices may edit, create, move, and ordinarily Trash notes while offline. Synchronization exposes concurrent content/structural conflicts explicitly. Copying workspace files never copies Automation, approval-count, Money, or external-effect consumption authority.
6. Intranet Server deployments retain multi-user concurrent editing. Multiple users/sessions may concurrently hold Drafts, read, prepare, and edit the same or different documents; durable results are serialized through the unique Server/Core commit boundary. SQLite single-writer transactions do not imply a single-user product.
7. Real-time collaborative text editing remains on the existing D1 roadmap after G2. This candidate does not claim an implemented OT/CRDT protocol or released collaboration capability.

## 2. Replacements produced by this batch

This batch produces complete afterimages only for:

- `d6-storage-transactions-permissions-and-sync`
- `d6-control-interfaces`
- `d6-terminology-and-naming-lexicon`
- `d6-terminology-registry`
- `d6-implementation-impact-and-test-outline`

Their fixed-S sources, source blobs, output paths, and version changes are recorded only in `replacements.json`. Planned future afterimages are not listed there until they exist.

## 3. Difference routing

| Topic | Fixed-S assumption | D6-FA-r01 owner |
|---|---|---|
| current Document/Resource bytes | complete bytes in authority SQLite | Storage: ordinary files are current author bytes; control DB never stores a whole-workspace current-body replica |
| identity / parent / order / lifecycle | current with payload/control in one DB transaction | Storage: portable metadata uniquely owns current facts; P stores decisions/recovery evidence only |
| operation ledger | WorkspaceId+OperationId under one continuous authority store | Control/Storage: CommitDomain enters the v2 key; legacy saved v1/v9-v11 bytes remain unchanged |
| ordinary multi-device writes | dual writers without continuity forced read-only/fork/reconciliation | Storage: each replicaEpoch may make ordinary replica-local commits and produce ChangeId/frontier; global execution responsibility remains separate |
| author commit | one SQLite transaction publishes payload/control/receipt | Storage: file installation, durable-control seal, and portable publication are distinct phases; reliable save and portable publication are distinct facts |
| source version | Ref+Counter/store incarnation | Control: SourceVersion/2 explicitly binds CommitDomain, observationEpoch, revision, ChangeId |
| partial index | full scan or unavailable | retained; it does not block unrelated ordinary saves and never satisfies complete Action proof |
| external file edit | checkout proposal only | file itself is current source; external change advances observation epoch and may yield conflict/unavailable |
| pins/effects | planned/terminal decisions may retain complete source pins indefinitely | purpose-bound pins, last-reference/unknown protection, capacity and expirable historical effects; legacy promises are not retroactively weakened |
| collaboration | Server has one commit holder; real-time later | multi-user Draft/prepare/broadcast ingress with one durable commit sequence; OT/CRDT remains unfrozen |

## 4. Separate qualification layers

- **ordinary replica content**: ordinary source edits, create, local move/reorder, and Trash require only the actual source/identity/structure/policy scope they touch plus an eligible installation primitive.
- **complete semantic/action proof**: D4/D5 relations, uniqueness, Calendar, collection membership, complete bulk operations, and purge still require their actual complete positive/negative ranges. A building/partial index is not such proof.
- **global execution responsibility**: Automation, ApprovalUse, claim, Money, external unknown, and stop require continuous single execution responsibility and are never inferred from file sync or replica registration.

`semantic_pending` means a D2-valid reliable save whose applicable local typed facts have passed their local gates while one or more cross-object/global obligations remain unproved. It is not a D4/D5-complete success and may not be automatically consumed by Query/Action/automation that requires complete semantics. This affects D4/D5 operation-applicable consumers and therefore requires real later owner afterimages.

## 5. Version transition

- D6 Control wire `1 -> 2`: CommitDomain, SourceVersion/2, frontier, ordinary-save and conflict interfaces, Policy/3.
- D6 PreparedIntent `1 -> 2`: accurate InputDescriptor, PinDirectory, and exact write set; the canonical commit request remains free of complete body bytes.
- D6 Policy `2 -> 3`: replica registration / ordinary content / conflict capabilities; Policy/1 and /2 retain their original decoders.
- D6 SourceVersion `1 -> 2`: bare numeric revision is never comparable across CommitDomain.
- D6 Effect/Prepared consumers require new versions. This batch does not yet produce D7/D8/D9 afterimages, so **D6-FA-r01 cannot be partially activated or produce managed v2 success**.
- A later D3 wire12 is expected to carry CommitDomain and local-vs-complete profiles. This batch does not write D3, so fixed-S D3 wire11 remains the only currently defined D3 contract.
- Historical D3 v9/v10/v11, D6 wire1, D7 PreparedActionBinding/1,/2, and all saved decisions continue under their original decoder, fingerprint, saved bytes, authorization and continuity gates. No field is backfilled and no saved request is re-encoded.

## 6. Required future owners

| Owner | Required coordinated change |
|---|---|
| D1 | replica qualification after external changes; shared-folder behavior limited to affected stale cuts/install ranges; preserve Server single commit holder, multi-client use and the post-G2 real-time roadmap |
| D3 | operation-ledger key, replica registration vs continue, local Trash receipts, purge frontier, move/order conflict, wire12 and legacy replay |
| D4 | exact semantic_pending consumption matrix for operation-applicable/relation/uniqueness/Calendar |
| D5 | native structure/bulk/collection boundary between ordinary save and complete proof |
| D7 | cut domain/frontier, PreparedActionBinding/3, effects-pin retention and partial-index gate |
| D8 | Source/Live/Read, three Live-mark modes, Draft/IME/Undo/selection continuity and collaborative checkpoints |
| D9 | scoped pins, ImportJob, exact export inputs, publication and new request/effects version consumption |
| D10 | recipient/target/payload approval, sourceOccurrenceKey continuity, Money lineage, Run/Lease/Automation/deployment execution responsibility |

These are pending routes only and do not appear in `replacements.json` until actual files exist.

## 7. Large workspace milestones

The afterimages distinguish `T_first_open`, `T_first_edit`, `T_first_reliable_save`, `T_full_search_ready`, and `T_OCR_ready`. Ordinary reliable save does not wait for unrelated full-workspace indexing or OCR; complete Query/Action does not consume incomplete indexes. Startup does not hash the whole workspace and I does not retain a whole-workspace body/AST copy. No performance test or seconds-level guarantee is claimed by this candidate.

## 8. Existing D10 review state

The fixed B13 result remains **REVISE**, terminology/bilingual FAIL, P0=0, P1=3, P2=8, eleven OPEN findings. This candidate supplies related foundation surfaces only and does not close, reclassify, or independently accept any finding.

Open IDs remain:

- P1: `R08-B13-P1-01`, `R08-B13-P1-02`, `R08-B13-P1-03`
- P2: `R08-B01-P2-01`, `R08-B02-P2-01`, `R08-B02-P2-02`, `R08-B02-P2-03`, `R08-B05-P2-01`, `R08-B11-P2-01`, `R08-B12-P2-01`, `R08-B13-P2-01`

A future immutable candidate requires a fresh complete independent joint review over the new proposal, all 18 D10 files, fixed S49, and every actual replacement-owner afterimage. Author documentation checks and CI are not independent acceptance.

