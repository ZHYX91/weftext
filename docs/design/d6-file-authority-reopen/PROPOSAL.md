---
source_language: zh-CN
translation_of: PROPOSAL.zh-CN.md
translation_status: synced
---

[简体中文](PROPOSAL.zh-CN.md)

# D6-FA-r01 File Authority Reopen — Coordinated Candidate Note

Status: **candidate / partial coordinated candidate / not accepted / not activated / not implemented**.

Fixed upstream remains S=7e18168dad3e6d120fce0dd607dc10fa7894e252 and the old D10 author-candidate baseline remains C8=d99f053b9386c9c9e1664251fdec9f00e33fac2c. This directory contains D6-FA-r01 candidate afterimages only and does not modify snapshots, inputs, or the original 18 D10 files. Activation is forbidden until every real replacement owner, D10 consumer update, fresh independent joint review, and coordinated acceptance are complete.

This file is routing/version/difference documentation only. Normative operation semantics belong only to the corresponding files under owners/; this file is not a second transaction, conflict, identity, or authorization protocol.

## 1. Decided product choices

1. Ordinary .adoc files uniquely carry current Document exact source; ordinary resource files uniquely carry current Resource bytes.
2. A separate SQLite outside the synchronized directory provides device-local rebuildable indexing and may be deleted/rebuilt; it never stores a whole-workspace current-body or full-AST mirror.
3. A different durable SQLite outside the synchronized directory stores non-reconstructible execution control: original decisions/receipts, transaction recovery, unknowns, approval/claim, Money/budget responsibility, and necessary pins.
4. Portable identity, parent/order, lifecycle, shared configuration/ACL/trust are carried by portable workspace metadata. D3 remains their logical identity/placement/lifecycle owner while D6 owns physical storage/transactions.
5. Multiple registered devices may perform ordinary offline edit, create, move, reorder, and Trash work, then resolve source/placement/lifecycle/policy conflicts explicitly.
6. File copying never copies Automation, approval-count, Money, external-unknown, or stop consumption authority.
7. Intranet Server remains multi-user. Multiple sessions may hold Draft/read/prepare state concurrently while one hosted backend still has one durable commit holder.
8. Real-time collaborative text remains post-G2; this candidate does not claim OT/CRDT implementation and does not adopt per-keystroke author commits.

## 2. Replacements now produced

replacements.json lists only ten fixed-S owner replacements that actually exist.

First D6 batch:
- d6-storage-transactions-permissions-and-sync
- d6-control-interfaces
- d6-terminology-and-naming-lexicon
- d6-terminology-registry
- d6-implementation-impact-and-test-outline

Second D1/D3 batch:
- d1-product-surface-and-capability-boundary
- d1-implementation-impact-and-test-outline
- d3-identity-references-ownership-and-lifecycle
- d3-terminology-and-naming-lexicon
- d3-implementation-impact-and-test-outline

The machine list records only fixed-S sourcePath/sourceBlob, actual output paths, consumers, and real version differences. Future afterimages are not pre-registered.

## 3. Core difference routing

| Topic | fixed-S assumption | Current D6-FA-r01 owner |
|---|---|---|
| current Document/Resource bytes | complete bytes in authority SQLite | D6 Storage: ordinary files are current author bytes; P/I hold no whole-workspace current body |
| identity / parent / order / lifecycle | current state in the payload/control database | D3 remains logical owner; D6 portable metadata is physical carrier only |
| operation ledger | WorkspaceId+OperationId | D3 wire12 / D6 v2: WorkspaceId + CommitDomain + OperationId |
| ordinary multi-device writes | insufficient continuity leads to global read-only/fork/reconciliation | D1/D3/D6: each active replicaEpoch has bounded ordinary operations with explicit conflict records |
| author commit | one SQLite transaction publishes payload/control/receipt | D6: file installation -> P seal -> portable publication; reliable save differs from portable publication |
| SourceVersion | Ref+Counter/store incarnation | D6 SourceVersion/2 binds CommitDomain, observationEpoch, revision, ChangeId |
| partial index | complete scan or unavailable | unrelated ordinary save does not wait for complete index; complete Query/Action never consumes a gap |
| external file edit | checkout proposal only | the file itself is current bytes; observation gaps advance observationEpoch and ABA never reuses hash continuity |
| pins/effects | decisions may keep complete source pins indefinitely | purpose-bound pins plus last-reference/unknown protection; legacy retention is not weakened retroactively |
| local lifecycle | strong workspace-wide closure | D3 wire12 separates replica_local Trash from managed_atomic restore/purge |
| collaboration | Server has one commit holder; real-time later | D1 separates multi-user sessions from one durable commit holder; real-time remains post-G2 |

## 4. Three qualification layers

- ordinary replica content: ordinary source save, create_node, local move/reorder, and Trash require the actual source/identity/structure/policy range plus an eligible installation primitive.
- complete semantic/action proof: relation, unique, Calendar, collection, complete bulk, restore/purge/copy/fork/import retain their complete positive/negative ranges.
- global execution responsibility: Automation, ApprovalUse, claim, Money, external unknown, and stop require a separate continuous and fenced responsibility domain.

semantic_pending means a D2-valid source is reliably saved and local typed facts passed their local gate while the listed cross-object obligations remain unproved. It is not D4/D5/D7 complete success.

## 5. Version transitions already authored

Produced D6 transitions:
- Control wire 1 -> 2
- PreparedIntent 1 -> 2
- Policy 2 -> 3
- SourceVersion 1 -> 2

Produced D3 transitions:
- identity operation wire 11 -> 12
- identity_change_receipt 11 -> 12
- resolver context/outcome 11 -> 12
- operation ledger key from WorkspaceId+OperationId to WorkspaceId+D3-CJ/3(CommitDomain)+OperationId
- replica_local versus managed_atomic mode matrix
- replica registration separated from continue
- purge active-replica Frontier gate
- saved v9/v10/v11 original decoder/bytes/gates preserved

D3-CJ/3, D3Integer, Ref/Locator lexemes, Annotation Value/3, and Result/9 do not automatically change version because wire12 changes.

Only D3/D6 owner afterimages are complete so far. D4/D5/D7/D8/D9/D10 consumers remain incomplete, therefore **partial activation and coordinated managed-success product outcomes are forbidden**.

## 6. Remaining required owners

| Owner | Required coordinated change |
|---|---|
| D4 schema/relations | exact semantic_pending consumption for relation/unique/Calendar/cross-object types |
| D5 structures | local versus complete ranges for native/bulk/collection |
| D7 Query/Action | CommitDomain/frontier/SourceVersion consumer, new Prepared version, effects-pin retention, partial-index gate |
| D8 editor | Source/Live/Read, three Live marker modes, Draft/IME/Undo/selection, multi-session and collaboration checkpoint |
| D9 import/export | scoped pins, ImportJob, exact export inputs, publication, new request/effects consumer |
| D10 | recipient/target/payload approval, sourceOccurrenceKey continuity, Money lineage, Run/Lease/Automation/deployment execution responsibility |

These files do not enter replacements.json until they actually exist.

## 7. First-open, large-workspace, and multi-user boundary

The candidate measures T_first_open, T_first_edit, T_first_reliable_save, T_full_search_ready, and T_OCR_ready separately. Ordinary reliable save does not wait for unrelated full indexing/OCR; complete Query/Action does not consume an incomplete index. No seconds-level claim exists without performance measurements.

Multiple Server users may edit the same or different documents concurrently. Durable commit ordering is not a frontend single-user lock. Post-G2 real-time collaboration still requires separate implementation evidence.

## 8. Existing D10 review state

Fixed B13 remains **REVISE**, terminology/bilingual FAIL, P0=0, P1=3, P2=8, eleven OPEN findings:

- P1: R08-B13-P1-01, R08-B13-P1-02, R08-B13-P1-03
- P2: R08-B01-P2-01, R08-B02-P2-01, R08-B02-P2-02, R08-B02-P2-03, R08-B05-P2-01, R08-B11-P2-01, R08-B12-P2-01, R08-B13-P2-01

This candidate closes, reclassifies, or independently accepts none of them. A future immutable candidate still requires fresh complete joint review of the proposal, all 18 D10 files, fixed S49, and every real replacement-owner afterimage. Author documentation checks and CI are not independent acceptance.
