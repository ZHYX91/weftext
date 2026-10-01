---
source_language: zh-CN
translation_of: PROPOSAL.zh-CN.md
translation_status: synced
---

[简体中文](PROPOSAL.zh-CN.md)

# D6-FA-r01 File Authority Reopen — Coordinated Candidate Note

Status: **candidate; P1 author-side artifacts are complete; the coordinated candidate remains incomplete; not accepted, not activated, not implemented, and not merged**.

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
| Field/Facet/relation/Calendar Registry semantics | D4 | portable Registry metadata + D6 production SourceVersion/2 and current SourceObservation/SourceVersionRef outer qualification |
| native table structure / Node Collection semantics | D5 | exact source / D7 result + D6 current SourceObservation outer qualification |
| durable decision/recovery | protocol owner + D6 | local P |
| search/parser/index candidates | source owners | rebuildable I |
| global execution responsibility | D6/D10 | continuous fenced control |

P/I never becomes a second author truth for Field, relation, collection, or parent/order.

## 4. Three qualification layers, current producers, and downstream consumption boundary

- **ordinary replica content**: ordinary source save and D3 create_node/move/reorder/Trash prove only the source, identity, structure, policy, and installation ranges actually touched. D5 native cell/row/column/reorder may proceed from complete real local evidence without waiting for unrelated I or a whole-Workspace Query, while structured edits remain strict. Missing unrelated strong proof never permanently makes an otherwise qualified ordinary/local/offline path read-only.
- **complete semantic/action proof**: D4 relation, strong Facet mutation, Calendar uniqueness, D5 collection membership/bulk/requireMembership/restore/purge/copy/fork/import, and D7 all_result, strong Action, and Automation retain their owner-defined complete positive/negative ranges, current authorization, and complete cut. semantic_pending, local success, or a partial index never substitutes.
- **global execution responsibility**: Automation, ApprovalUse, claim, Money, external unknown, and stop retain a separate continuously fenced responsibility chain. Replica registration, ChangeId/Frontier, or file copying never grants those consumption rights.

The private P1 Storage, Control, Terminology Lexicon, machine Registry, and Implementation Impact/Test owner afterimages now actually exist, but remain unaccepted, unactivated, and unimplemented candidates. The fixed-C D1/D3 G0-B afterimages and D4/D5 A/B/C candidates also genuinely exist. “Candidate exists” means only that the corresponding earlier boundary has an afterimage; it does not mean that those consumers already consume every producer rule completed later in this P1. Affected D3/D4 paths still need P2 to provide the real native-descriptor, companion, range-key, version, and recovery consumers; D5 still needs P3 for locator/cut/save-protection consumption; those paths then still need fresh joint acceptance.

Source production version and current observation remain separate. SourceVersion/2 is production history: the managed arm records production CommitDomain, production observationEpoch, revision, and ChangeId; the external arm records production CommitDomain, production observationEpoch, and externalSequence and has no managed revision/ChangeId. Production domain may differ from the current operation/observer domain. For production domain D and entity E, H(D,E) is the greatest revision in that domain's continuous sealed managed history. H=0 is legal only for a proved complete empty history; a real managed after uses checked H+1, MAX never wraps, production observationEpoch does not reset H within one production domain, and cross-domain returns continue each domain's own H. Equal-byte external admission is still managed admission. Source deletion and source-unchanged portable placement/lifecycle/structure effects do not advance H, although a real portable effect still receives this decision's ChangeId at seal.

SourceObservation/1 separately binds the current operation CommitDomain as observerDomain together with the complete production SourceVersion/2, current observationEpoch, FileObjectBinding, and evidencePins. SourceVersionRef/1.sourceToken selects only that complete current Observation. A watcher gap, replacement, or discontinuous rematerialization invalidates old Observation, Ref, and protected revision-token currentness even when production SourceVersion, revision, hash, or final text is equal. Only an original plan that will produce a managed after freezes internal SourceRevisionPlan/1, SourceStamp/1, and after pin in P; there is no new ChangeId for this decision before seal. RevisionTokenBinding/2 with d6_source_revision/2 binds observerDomain/current observationEpoch to a managed SourceStamp or complete external SourceVersion. It does not change the D3 Locator opaque-token lexical form and does not replace the D4 sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence or D5 revision-bound locator inner wire.

DependencyProof/2 uses exactly fourteen closed DependencyKey/2 kinds: source, lifecycle, placement_range, ref_inbound, relation_incidence, calendar_scope, registry, temporal_rules, authorization, foreign_binding, query_scan, replica_registry, conflict_record, and execution_resource. Each key uses its real owner-defined range, protected evidence pins, and `{epoch,revision}` continuity stamp. Completeness comes from a currently authorized consistent snapshot/range barrier or a gap-free continuous change chain plus final revalidation. I is candidate cache only. Deleting/rebuilding I does not by itself destroy complete continuity facts still retained by their real P/M owners; actual proof loss, a gap, owner-rule/decoder change, or unproved continuity invalidates the old proof and requires a new epoch. Partial/building index state, index miss, placeholder, unknown state, provider “synced”, equal final hash, or larger Frontier vector numbers never prove empty or complete.

Frontier/2 is only the verified continuous sealed causal prefix per CommitDomain; it proves neither payload materialization, placeholder download, Registry/index completeness, nor D7 complete Query. frontierPolicy=exact requires full original Frontier equality and D3 managed_atomic remains exact. scope_dependencies admits only a real continuous verified-sealed non-regressing extension from the original expectedFrontier that is proved unrelated to every original source/control/authorization and positive/negative DependencyKey. The original canonical request, expectedFrontier, DependencyProof.baseFrontier, targets, Query/selector, pins, proposed bytes, WriteProtection, owner input, and version basis are never re-signed or resampled. A real bound-dependency change or unknown gap is never relabeled “unrelated”; conversely, an ordinary/local operation depending only on complete real local evidence is not incorrectly blocked by an unrelated whole-Workspace gate.

Ordinary semantic qualification and strict|observed_only write protection are independent axes, and ordinary may use strict. observed_only is legal only when a trusted human interactive_source_save explicitly selects and freezes it before planning starts and every condition holds: exactly one existing live Document, ordinary+replica_local, complete source read/replace, author-source write set empty or limited to that Document, no applicable body/Field/node-control deny, no identity/parent/order/lifecycle/shared-policy/Registry/Calendar-scope/other-entity mutation, and Draft Base equal to the selected current SourceObservation. noninteractive, managed_atomic, D3 structure/lifecycle, D5 structured cell/row/column/reorder, bulk/collection, D7 strong Action, Automation, server checkpoint, Approval, and Money remain strict. The original plan durably retains B and N. An external C still unseen after the final trusted check may be overwritten by N and may have no recoverable copy; a later C may replace current file again, but B/N retention remains. Observed competition, stale Base, watcher gap, revocation, or another failed qualification receives no weak exemption. Unknown install remains recovery_unknown and prepare/retained is not Saved.

Portable installation and publication follow the real owner versions. InstallationNotice/2 is durable before the first portable-current install and binds DecisionKey, original baseFrontier, WriteProtection, and component before/after. baseFrontier may contain historical sealed ChangeIds, but the notice contains no new ChangeId for this not-yet-sealed decision. The single P seal allocates the portable decision's ChangeId and, only for a real managed source change, combines SourceRevisionPlan/1 SourceStamp with that ChangeId into managed SourceVersion/2. The new path then publishes ContentCompletionProof/3 with real production SourceVersion before/after and actual frontierBefore/frontierAfter; a receiver validates the complete continuous sealed chain and creates its own SourceObservation rather than copying sender tokens. Current conflicts use ConflictRecord/2+Frontier/2 while ConflictKey/1, ConflictId, and the `D6-ConflictKey/1` hash domain remain unchanged.

Historical ContentCompletionProof/1,/2, ConflictRecord/1, Frontier/1, InstallationNotice/1, legacy revision-token profiles, Policy/1/2, D6 wire1, existing D3/D7/D8 bindings/receipts, and actual saved/planned/unknown records continue under their original bytes, decoders, pins, authorization, lifetime, and continuity. ContentCompletionProof/2 SourceVersionRef sourceChanges never migrate to /3 production SourceVersion and ConflictRecord/1 createdAtFrontier is never decoded as Frontier/2. The presence of a decoder does not prove that every historical prototype was deployed or active, but actual historical recovery obligations are never erased by new wire, I rebuild, or reauthorization.

## 5. Version and compatibility

The P1 author candidate now records these actual version boundaries. This section only routes versions frozen by their owners and creates no second wire:

- D6 Control wire 1->2, PreparedIntent 1->2, Policy 2->3, SourceVersion 1->2, Frontier 1->2, ObservationScope 1->2, DependencyProof 1->2, and InstallationNotice 1->2. WriteProtection adds the current strict|observed_only axis. SourceObservation/1, SourceVersionRef/1, and SourceStamp/1 retain their existing version numbers.
- The new producer path advances current ContentCompletionProof from /2 to /3. /3 sourceChanges uses production SourceVersion/2|absent and records actual frontierBefore/frontierAfter. Historical /1 and /2 retain their original bytes, SourceVersionRef sourceChanges, decoders, pins, and recovery gates; they are never backfilled or re-encoded.
- Current new conflict records use ConflictRecord/2+Frontier/2. Historical ConflictRecord/1+Frontier/1 keeps its original decoder. ConflictKey/1, ConflictId format, and the `D6-ConflictKey/1` hash domain are not versioned.
- New-decision protected revision-token resolution uses RevisionTokenBinding/2 with d6_source_revision/2. Legacy d6d/d6r/d6a profiles and opaque DocumentRevision/ResourceRevision/AnnotationRevision decoders remain intact. SourceRevisionPlan/1 is an internal P plan type and SourceStamp/1 is a proposed version address; neither is another public current-source wire and neither changes D3/D4/D5 inner selector shapes.
- DependencyProof remains /2. This P1 completes DependencyKey/2 as the fourteen-kind closed union with ordering, stamps, owner/range/continuity, and error rules; it does not bump DependencyProof again. The machine Registry root schema remains D6-Terminology/2. Its 48 concept identities, 13 crossStageBindings, ownedNames, and firstFreeze values remain; P1 synchronizes selected definitions and binding contracts without inventing another Registry schema version.

The D3 identity operation/receipt/resolver 11->12 candidate already exists with CommitDomain in its ledger key. That does not mean P2 has consumed SourceRevisionPlan/RevisionTokenBinding/DependencyKey/ContentCompletionProof/ConflictRecord/recovery semantics. D4 public Entry/Type/RelationReadContext/Binding/Recurrence/effect shapes and sourceRevision/OccurrenceKey/Entry-selector wire remain unchanged, as do D5 v1 domains/limits and revision-bound locators. New D6 outer qualification composes complete SourceObservation/1, SourceVersionRef/1, DependencyProof/2, and owner input without rewriting those inner wires.

Existing D1, D3, D4, and D5 candidates continue to mean only the candidate consumption surface they actually contain. P2 D3/D4 and P3 D5 must genuinely consume the final P1 producer before affected owner_update_required/proof_unavailable gates can be removed. D7 cannot be updated only by adding a new Prepared form: Query Algebra, Value/CEL, View, Narrow Field Qualification, Definition Transfer, Preview/Effects, Execution/Action, Prepared Action Binding, Scenario Dispositions, Terminology Lexicon/Registry, and Implementation Impact/Test Outline all require coordinated consumers; D8, D9, and all eighteen D10 owners likewise remain real downstream work.

Historical D3 v9/v10/v11, D6 wire1, Policy/1/2, SourceVersion/1, Frontier/1, InstallationNotice/1, ContentCompletionProof/1,/2, ConflictRecord/1, legacy revision-token profiles, D7 PreparedActionBinding/1,/2, D8 PreparedEditBinding/1, and actual Result/ByteHandle/saved/planned/unknown records continue under their original versions, bytes, authorization/pins/continuity, and recovery. A prototype without deployment evidence is never promoted to active compatibility, while an actual historical obligation is never cancelled merely because a new version exists.

None of these author-side version records means accepted, activated, or implemented. PROPOSAL/replacements is routing only. Coordinated effect still depends on P2/P3, actual D7–D10 consumers, later disposition of the eleven existing OPEN findings and U6/U7, fresh independent full joint review and coordinated acceptance, then A2 self-contained reconstruction and the separate fresh Pro global review after A2.

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

## 8. Current coordinated consumers and owners still pending

| Owner | current candidate state / required coordinated work |
|---|---|
| P1 D6 | private author afterimages for Storage, Control, Terminology Lexicon, machine Registry, and Implementation Impact/Test now actually exist; this PROPOSAL/replacements provides final routing only. The whole candidate is still not independently accepted, activated, or implemented |
| D1 | the G0-B candidate already records ordinary-save product state/metrics and strict versus durable_observed_only boundaries; P1 routing does not activate it |
| D3 | fixed C already has wire12, owner-descriptor, SourceObservation, and companion/receipt candidate boundaries. P2 still must consume SourceRevisionPlan/1, RevisionTokenBinding/2+d6_source_revision/2, the D3-owned DependencyKey/2 ranges, saved/planned/unseen recovery, and same-P-seal rules before acceptance |
| D4 | three existing candidates retain production SourceVersion versus observerDomain separation, current-Observation outer qualification, and unchanged inner selectors. P2 still must consume complete relation_incidence/calendar_scope/registry/temporal_rules ranges and stamps, revision binding, receiver-local Observation after CP3, and recovery split |
| D5 | three existing candidates retain source/locator outer qualification, strict native structured edits, and the collection/bulk strong gate. P3 still must consume new currentness, DependencyProof cut, strict|observed_only ordinary-save protection, and recovery |
| D7 | Query Algebra, Value/CEL, View, Narrow Field Qualification, Definition Transfer, Preview/Effects, Execution/Action, Prepared Action Binding, Scenario Dispositions, Terminology Lexicon/Registry, and Implementation Impact/Test Outline all require real consumer afterimages. Complete Query/Result/Action additionally consumes query_scan, current authorization generation, complete positive/negative ranges, and reset rules |
| D8 | Source/Live/Read, Draft/Edit Map, IME/Undo, ReliableSaveState/portable publication, conflict, multi-session, and collaboration checkpoint still need complete current SourceObservation binding while retaining legacy SourceVersion/1/PreparedEditBinding/1 decoding |
| D9 | scoped pins, ImportJob, construction/import/export cuts, exact export inputs, publication, and foreign binding/version consumers still need actual afterimages |
| D10 | all eighteen actual owners still need recipient/target/payload approval, ApprovalUse, sourceOccurrenceKey continuity, Run/Lease/Automation/Workspace/deployment Money lineage, provider external-result unknown, claim, stop, and execution responsibility coordination; U6/U7 remain open |
| review/acceptance | the existing P0=0, P1=3, P2=8 eleven OPEN findings remain. Final assembled bytes still require fresh independent full joint review and coordinated acceptance, then A2 self-contained reconstruction and a separate fresh Pro global review after A2 |

Managed/strong success depending on a new P1 producer continues to return owner_update_required, proof_unavailable, or the owner's existing unavailable outcome until the corresponding P2/P3/D7–D10 consumer and fresh joint acceptance are complete. D6 documents, fixtures, author checks, or CI never justify partial activation. Conversely, approved ordinary `.adoc`/Resource reads, Draft, fully qualified human whole-source saves, and local offline work that does not depend on a missing strong producer/consumer are not permanently removed. Ordinary success also never upgrades to complete Query/Action/Automation qualification.

## 9. Large-workspace, conflicts, and historical result

T_first_open, T_first_edit, T_first_reliable_save, T_full_search_ready, and T_OCR_ready stay separate, with no unmeasured seconds claim; G0-B registers the ReliableSaveState/InputRetention boundary between strict reliable save and durable_observed_only retention. durable_observed_only still does not populate the old strict T_first_reliable_save metric and does not remove later consumer, joint-review, or activation gates.

D4/D5 local operations do not become globally read-only because unrelated I coverage is missing. A strong consumer may scan source for complete proof; otherwise it is unavailable.

r5 receipt, current r6, I rebuild, P loss, and external A->B->A remain separate. Later validation never rewrites old receipts. Multi-replica source/table/Field/collection conflicts are explicit rather than LWW.

## 10. Existing D10 review state

Fixed B13 remains **REVISE**, terminology/bilingual FAIL, P0=0, P1=3, P2=8, eleven OPEN findings:

- P1: `R08-B13-P1-01`, `R08-B13-P1-02`, `R08-B13-P1-03`
- P2: `R08-B01-P2-01`, `R08-B02-P2-01`, `R08-B02-P2-02`, `R08-B02-P2-03`, `R08-B05-P2-01`, `R08-B11-P2-01`, `R08-B12-P2-01`, `R08-B13-P2-01`

This candidate closes, reclassifies, or independently accepts none. A future immutable candidate still requires fresh complete joint review of the new proposal, all 18 D10 files, fixed S49, and every real replacement-owner afterimage. Author checks/CI are not independent acceptance.
