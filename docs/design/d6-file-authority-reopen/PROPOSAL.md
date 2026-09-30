---
source_language: zh-CN
translation_of: PROPOSAL.zh-CN.md
translation_status: synced
---

[简体中文](PROPOSAL.zh-CN.md)

# D6-FA-r01 File Authority Reopen — Coordinated Candidate Note

Status: **candidate / partial coordinated candidate / not accepted / not activated / not implemented**.

Fixed upstream remains S=7e18168dad3e6d120fce0dd607dc10fa7894e252 and the old D10 author-candidate baseline remains C8=d99f053b9386c9c9e1664251fdec9f00e33fac2c. This directory contains D6-FA-r01 candidate afterimages only and does not modify snapshots, inputs, or the original 18 D10 files. Activation requires every real replacement owner, D10 consumer update, fresh independent joint review, and coordinated acceptance.

This file is routing/version/difference documentation only. Normative operation semantics belong only to their owner files.

## 1. Decided product choices

1. Ordinary `.adoc` files uniquely carry current Document exact source and ordinary resource files current Resource bytes.
2. Discardable SQLite I and durable SQLite P are outside the synchronized directory. I is rebuildable; P keeps non-reconstructible decision/recovery/unknown/approval/claim/Money/pins. Neither is document authority.
3. Portable metadata physically carries identity, parent/order, lifecycle, shared configuration/ACL/trust while logical ownership remains D3/D4 and the actual domain owners.
4. Registered replicas may perform ordinary offline edit/create/move/reorder/Trash and later expose source/placement/lifecycle/policy conflicts explicitly.
5. Copying portable files never copies global execution responsibility.
6. Server remains multi-user for Draft/read/prepare/edit while one hosted backend has one durable commit holder.
7. Real-time collaboration remains post-G2, with no OT/CRDT or per-keystroke author commit frozen here.
8. Ordinary file save in a large workspace does not wait for unrelated complete index/OCR, while complete Query/Action still proves completeness.
9. The user approved WriteProtection=observed_only for a trusted human ordinary save of one existing live Document when complete authorization, local gates, durable input/actually-read-before retention, and the single P decision chain still hold. It relaxes only exclusion of an unobserved external race; observed conflict, revocation, idempotency, durability, and strong-Action qualification are unchanged.

## 2. Actual replacements now: 16

`replacements.json` lists only the 16 fixed-S owner replacements that actually exist:

- five D6: Storage, Control, Terminology Lexicon, Terminology Registry, Implementation Impact/Test;
- two D1: Product Surface/Capability Boundary, Implementation Impact/Test; G0-B generated the corresponding owner afterimages.
- three D3: Identity/References/Ownership/Lifecycle, Terminology Lexicon, Implementation Impact/Test; G0-B generated the corresponding owner afterimages.
- three D4: Attribute Types/Schema/Relations, Terminology Lexicon, Implementation Impact/Test;
- three D5: Tables/Node Collections, Terminology Lexicon, Implementation Impact/Test.

D4 Reference Catalog and mandatory scenarios remain fixed read-only inputs and are not replacement entries. This batch changes no catalog definitions or limits.

## 3. Current owner routing

| fact / capability | logical owner | D6-FA-r01 physical/control |
|---|---|---|
| Document/Resource current bytes | D2/Resource domain | ordinary files |
| identity/parent/order/lifecycle | D3 | portable metadata + D3 wire12 |
| Field/Facet/relation/Calendar Registry semantics | D4 | portable Registry metadata + D6 SourceVersion/control binding |
| native table structure / Node Collection semantics | D5 | exact source / D7 result |
| durable decision/recovery | protocol owner + D6 | local P |
| search/parser/index candidates | source owners | rebuildable I |
| global execution responsibility | D6/D10 | continuous fenced control |

P/I never becomes a second author truth for Field, relation, collection, or parent/order.

## 4. Three qualification layers and closed D4/D5 consumption

- **ordinary replica content**: ordinary source save, D3 create_node/move/reorder/Trash, and D5 native-table local edits prove the local source/identity/structure/policy and installation ranges actually touched.
- **complete semantic/action proof**: D4 relation/strong Facet mutation/unique Calendar, D5 collection membership/bulk/requireMembership, restore/purge/copy/fork/import, D7 all_result/automation retain complete positive/negative ranges.
- **global execution responsibility**: Automation, ApprovalUse, claim, Money, external unknown, stop use a separate continuous-responsibility domain.

Existing D4/D5 afterimages remain. G0-A completes the D6 producer, and G0-B completes D1 product consumption plus D3 owner companion consumption. D1/D3 now register candidate consumption boundaries for Frontier/2, SourceObservation, WriteProtection, and the minimal receipt/companion, but D4/D5, D7/D8/D9/D10 have not completed their corresponding consumer updates, so dependent new success remains gated.

Existing D4/D5 afterimages fix:
- D4 namespace `available|retained_unavailable|invalid|not_present`, Node typed `complete|partial|unavailable`, D6 `complete_semantics|semantic_pending`, and future D7 complete-cut as four different dimensions.
- `semantic_pending` is allowed only after local typed facts pass while cross-object/complete obligations remain unproved; local type/cardinality/requiredness failure still rejects.
- body or truly disjoint namespace edits may save when unavailable/invalid raw bytes remain byte-equal; typed edit touching those namespaces rejects.
- D5 native cell/row/column structured edits do not require a whole-workspace Query cut; strong Node Collection mutation requires complete membership/cut.
- current page, I coverage, placeholder, and equal hash never prove relation/unique/Calendar/collection negative range or all_result.

## 5. Version and compatibility

Produced:
- D6 Control wire 1->2, PreparedIntent 1->2, Policy 2->3, SourceVersion 1->2; G0-A freezes Frontier/2, ObservationScope/2, DependencyProof/2, InstallationNotice/2, ContentCompletionProof/2, SourceObservation/1, and WriteProtection.
- D3 identity operation/receipt/resolver 11->12 with CommitDomain-scoped ledger key.
- D4 public Entry/Type/RelationReadContext/Binding/Recurrence/effect shapes remain; the new profile adds outer D6 InputDescriptor/2 binding for source-bearing evidence to SourceVersion/2, CommitDomain, Frontier.
- D5 adds no durable identity wire; D5 v1 domains/limits remain while D6-FA freezes local-versus-complete consumption.

G0-B also registers D1 product consumption: ReliableSaveState/InputRetention boundaries separate strict reliable save from durable_observed_only retention metrics; observed_only does not populate the old strict T_first_reliable_save.

G0-B also registers D3-native descriptor, SourceObservation/1, minimal receipt/no_op, and same-P primary/companion consumption boundaries while retaining later consumer, joint-review, and activation gates.

Historical D3 v9/v10/v11, D6 wire1, D7 PreparedActionBinding/1,/2 saved bytes use original decoders/gates/retention. They are never backfilled or re-encoded.

## 6. D4 catalog unchanged

Fixed catalog blob:
`ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb`

It still defines 4 QualifierSetSpec, 22 aliases, 61 Fields, 7 Facets, 1 CalendarSeriesScopePolicy, and the original limits.

In particular:
- `people/labeled-text-value.label` is optional `semantic_code` contribution-set;
- codes are `people/other|people/personal|people/work`;
- `people/phone` and related fields reuse the alias;
- absent label is not empty/unknown/default other;
- display label is not SemanticCodeId;
- relation canonical owner/inverse and Calendar comparator/series-scope are unchanged.

## 7. D5 domain and hard limits

There is still no durable Record/RecordCollection domain. Document table row/cell is revision-bound occurrence and Node Collection membership is D7 Query-derived, not parent/owner/persistent membership.

Hard limits remain:
- explicit Node targets per collection mutation <=1000;
- native structured table row targets <=1000;
- preview page <=200;
- fetched page <=200;
- new Nodes per import batch <=1000.

Narrower budgets may reduce but never expand limits; pagination prefix is not completeness.

## 8. Consumers/owners still pending

| Owner | required coordinated work |
|---|---|
| D1 | G0-B registered ordinary-save product state/metrics and strict versus ordinary-file wording; not activated |
| D3 | G0-B registered D3-native owner descriptor, Frontier/2, SourceObservation, companion/receipt consumption; not activated |
| D4 | production SourceVersion versus observerDomain and weak B->N proof scope remain pending for C-batch consumption |
| D5 | native ordinary-save protection consumption while bulk/collection stays strict |
| D7 | CommitDomain/Frontier/SourceVersion complete cut, new Prepared, effects pin retention, partial-index gate |
| D8 | Source/Live/Read, three Live marker modes, Draft/IME/Undo/selection, multi-session and collaboration checkpoint |
| D9 | scoped pins, ImportJob, exact export inputs, publication, new request/effects consumer |
| D10 | recipient/target/payload approval, sourceOccurrenceKey continuity, Money lineage, Run/Lease/Automation/deployment execution responsibility |

Before new D7 Prepared exists, D4/D5 strong Actions requiring complete D7 preparation remain unavailable. Partial activation is forbidden.

## 9. Large-workspace, conflicts, and historical result

T_first_open, T_first_edit, T_first_reliable_save, T_full_search_ready, and T_OCR_ready stay separate, with no unmeasured seconds claim; G0-B registers the ReliableSaveState/InputRetention boundary between strict reliable save and durable_observed_only retention. durable_observed_only still does not populate the old strict T_first_reliable_save metric and does not remove later consumer, joint-review, or activation gates.

D4/D5 local operations do not become globally read-only because unrelated I coverage is missing. A strong consumer may scan source for complete proof; otherwise it is unavailable.

r5 receipt, current r6, I rebuild, P loss, and external A->B->A remain separate. Later validation never rewrites old receipts. Multi-replica source/table/Field/collection conflicts are explicit rather than LWW.

## 10. Existing D10 review state

Fixed B13 remains **REVISE**, terminology/bilingual FAIL, P0=0, P1=3, P2=8, eleven OPEN findings:

- P1: `R08-B13-P1-01`, `R08-B13-P1-02`, `R08-B13-P1-03`
- P2: `R08-B01-P2-01`, `R08-B02-P2-01`, `R08-B02-P2-02`, `R08-B02-P2-03`, `R08-B05-P2-01`, `R08-B11-P2-01`, `R08-B12-P2-01`, `R08-B13-P2-01`

This candidate closes, reclassifies, or independently accepts none. A future immutable candidate still requires fresh complete joint review of the new proposal, all 18 D10 files, fixed S49, and every real replacement-owner afterimage. Author checks/CI are not independent acceptance.
