---
source_language: zh-CN
translation_of: REVIEW-DISPOSITIONS.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-DISPOSITIONS.zh-CN.md)

# D10 Staged Independent-Review Issue Dispositions

revision: D10-r03-supplement-2026-09-28; status: author revision record, pending independent re-review. This file records issues from the first two independent-review batches, author change locations, and still-pending review status. It is not an independent reviewer closure record and not a Gate verdict.

## 1. Review-coverage boundary

Reviewed candidate is fixed as C=`1c2bcd3e2926966c628292072c7a54a2bd5df40e`, with fixed upstream U=`f205831c848729f7ddbc3ba0cf32b689459c0c98`.

The first two independent-review batches completely read all 14 D10 files in C plus eight upstream files in U: D1 main/impact, D6 main/Control, and D7 Execution/Narrow Field/Prepared Binding/Preview Effects. Forty of the original 48 U inputs still await independent review. This author revision additionally read the D4 reference catalog added by S, but the independent reviewer has not yet completely read it; under the current 49-input scope, 41 upstream inputs therefore remain pending independent review.

The author's earlier U 48/48 reading and this revision's supplemental 49th input are author-source coverage only and must not be represented as independent-review coverage. Historical 48/48 records are not retroactively rewritten as if 49 inputs had been read then. Every status in the table is fixed as "author revised; pending independent re-review".

## 2. Issue dispositions

| ID | Original severity | Issue | Author revision decision | Main change locations | Current status |
| --- | --- | --- | --- | --- | --- |
| B1-01 | P1 | D10 pre-submit approval errors cannot express a final-use race or revocation after D6 entry, while the original D6 error set has no approval error | Explicitly extend the D6 closed enum with `approval_unavailable`, disposition only `preflight`. It is used only when original D6 permission/ObservationScope and business prerequisites still hold and the associated approval dependency alone is lost. unseen writes no ledger; planned stays planned. D10 passes it through verbatim rather than wrapping it as approval_required/expired | CANDIDATE §15.1/§21; UPSTREAM-AMENDMENTS §2.3, §3.2, §3.3; IMPLEMENTATION §11; SCENARIO A13 | Author revised; pending independent re-review |
| B1-02 | P1 | After planned, both old preview token and approval may expire; a new client has no old copy, while current effects resolve/open is committed-only and re-prepare changes request/OperationId | Add proposed D7 `d7_planned_preview_open/opened` read-only entrypoint. Under current audience, original ObservationScope, permission, and authority/custody/continuity, it creates a new finite recovery-delivery epoch from the original planned PreparedActionBinding/2, semantic preview, and pins. It never reruns Query, changes target, or revives old tokens. Complete inspection may create `PlannedDecisionApproval/1` bound to the exact original request; deterministic dependency conflict still cannot be revived | CANDIDATE §15.2; UPSTREAM-AMENDMENTS §5; TERMINOLOGY §2/§8; SCENARIO A09; IMPLEMENTATION §7 | Author revised; pending independent re-review |
| B1-03 | P2 | An ApprovalUse reserved by a planned decision has no terminal count state after authoritative terminal_failed | Fix ApprovalUse count as one-way `unreserved→reserved→consumed|released_terminal`. Only the same abort transaction that records original D6 authoritative terminal_failed may perform `reserved→released_terminal`; history remains and replay never releases twice. Cancellation, TTL, temporary revocation, and Lease expiry do not release. Cost reservations are separate | CANDIDATE §15.3/§18; UPSTREAM-AMENDMENTS §2.4–§2.5, §3.2; SCENARIO A14/F19; IMPLEMENTATION §2.1/§7 | Author revised; pending independent re-review |
| B1-04 | P2 | A raw no-op has empty actual footprint and owner_fields effects, but the prior candidate required exactly one member change | Standing approval now has two legal branches: member-change and byte-exact raw-no-op. No-op still validates the same unique Entry/member, type, valueConstraint, current permission, dependencies, and Narrow Field Qualification, while MutationFootprint, field_change, and sourceVersions remain empty and no effect/revision is invented. A committed D6 no-op still consumes once and replay does not | CANDIDATE §14; UPSTREAM-AMENDMENTS §2.2/§4.2; SCENARIO A10; IMPLEMENTATION §7 | Author revised; pending independent re-review |
| B1-05 | P2 | Chinese "release only if unsent" and English "no billable send occurred" differed, and an actually sent request with reliable zero bill had no unique state | Unify cost state: `released` only when billable execution or billable send is proven never to have started; an actually sent request with reliable final zero cost is `settled(0)`; a possibly started attempt with unprovable final cost is `uncertain`. Author terminal_failed does not automatically alter cost state | CANDIDATE §18; TERMINOLOGY §9; SCENARIO F21/F23; IMPLEMENTATION §9 | Author revised; pending independent re-review |
| B2-01 | P1 | DelegationLease.maxRuns had no unique consumption unit or admission transaction; failure/cancellation/recovery could reset the count | Select "consume at first permitted protected execution". queued claim and pre-admission cancellation consume nothing. A Core-managed Run-admission CAS validates the current revision, trusted time, ActivationBinding, budgets, and cumulative use across one `leaseId` lineage and atomically writes `LeaseRunUse/1`. Failure/cancellation/crash after admission never refunds; same-Run recovery never consumes twice. Clarification: later protected steps of a Run with a complete existing `LeaseRunUse/1`, including recovery of its original planned request, do not compare remaining count again. With `maxRuns=1` already consumed once, the same Run may continue, while every step still revalidates current authorization, exact revision, trusted time, ActivationBinding, approval, and budgets. Only a new Run returns `delegation_exhausted` when the count is exhausted; loss of authority, revocation, expiry, or binding change still blocks. Terminal occurrence claim/outcome remains durably deduplicated. Lease expiry does not depend on cleanup, and unprovable time continuity returns `state_unavailable` | CANDIDATE §8/§13/§20/§21; TERMINOLOGY §2/§8; SCENARIO U02/F03/F05/F24–F27; IMPLEMENTATION §2.1/§6/§11/§15; TASK staged-revision constraints | Author revised; pending independent re-review |

| B10-01 | P2 | D3 lexicon already gives `weftext.term.origin-binding` ownership of `OriginBinding` / `origin_binding`, while Adopt also places `adoption_binding` in its wire/API variable note and `owned-names.codeConventions`, creating a second binding-name convention | Propose a minimal D3 lexicon erratum: Adopt keeps only the `adopt_*` code convention. Any association binding created by Adopt continues to use existing `OriginBinding(ForeignIdentityKey, NodeRef)` and the `origin_binding` name, owned by `weftext.term.origin-binding`. Delete the `adoption_binding` convention with no compatibility alias, dual read, migration alias, second identity, wire, or capability. Add positive `adopt_*`→`OriginBinding/origin_binding` and negative "`adoption_binding` absent from controlled positive surfaces" validation | UPSTREAM-AMENDMENTS §6; SCENARIO U27; IMPLEMENTATION terminology/corpus; this table | Author revised; pending independent re-review |

## 3. Unchanged boundaries

This revision does not broaden automatic author writes. The first generation still permits only the D7 `set_field_member` single_field_member profile; D3 create/lifecycle, D8 document/annotation edit, bulk/Facet/native-table, and other operations remain interactively confirmed or unsupported.

Core remains the sole author-transaction authority. ApprovalUse, LeaseRunUse, PlannedDecisionApproval, Run, and ExternalEffectIntent are managed control evidence, not a second ledger or second author receipt.

External `outcome_unknown` and cost `uncertain` remain. Approval/Run fixes do not turn them into automatic retry or automatic refund.

All proposed D6/D7 additions exist only in UPSTREAM-AMENDMENTS. Fixed upstream U is unchanged, and those amendments do not take effect before independent acceptance and coordinated activation.

## 4. Documentation-quality revision

The previous Chinese candidate repeatedly appended a generic sentence saying technical names did not expand authority in order to satisfy mixed-prose checks, and some lines gained doubled punctuation. Those sentences were not normative semantics. This revision removes that padding and explains the actual boundary in natural Chinese while retaining necessary controlled identifiers. Repository check_docs rules are not relaxed; any remaining mixed-prose failure must be fixed by real wording rather than repeated filler.

## 5. Targets for subsequent independent re-review

A later reviewer should reconstruct at least the original counterexample for each item and check:

- whether `approval_unavailable/preflight` is the sole non-permanent, non-leaking D6-owner result for the approval race;
- whether planned-preview recovery is truly read-only over saved semantics and consistent with committed effects transport, old preview TTL, and current authorization;
- whether released_terminal occurs only on authoritative abort and is completely separate from cost state;
- whether raw no-op invents no footprint/effect/version;
- whether settled(0), released, and uncertain are mutually exclusive and recover consistently;
- whether LeaseRunUse, occurrence claim, terminal outcome, and trusted time prevent restart/rescan/enable-disable from resetting counts or duplicating Runs.

This file cannot turn "author revised" into "independently closed". Independent reading of the remaining 40 U inputs plus the new S catalog must also continue before a final D10 Gate conclusion.
