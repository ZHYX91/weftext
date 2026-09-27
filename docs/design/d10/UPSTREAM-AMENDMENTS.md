---
source_language: zh-CN
translation_of: UPSTREAM-AMENDMENTS.zh-CN.md
translation_status: synced
---

[简体中文](UPSTREAM-AMENDMENTS.zh-CN.md)

# D10 Coordinated D6/D7 Upstream Amendment Proposal

revision: D10-r04-final-review-fixes-2026-09-28; status: candidate upstream amendment proposal, not jointly accepted or coordinatedly activated. Fixed upstream input commit is `f205831c848729f7ddbc3ba0cf32b689459c0c98`. Current D1-D9 snapshots remain authoritative. This document gives independent review complete future companion text; it changes no snapshot and does not authorize early product implementation of unattended author submission.

## 1. Purpose and unchanged boundaries

Existing D7 freezes fresh Action prepare, complete preview/effects, the original D3/D6 request, replay, and unknown recovery. Existing D6 freezes current authorization, planned/commit CAS, the sole author ledger, and the final author commit point. D10 needs three composition semantics that current upstream does not define:

1. When Standing Approval loses a race after D6 entry, D6 needs one expressible error that does not create a permanent rejection.
2. When a D6 decision is already planned and the original preview transport expired, a currently authorized user needs a read-only path to inspect the **saved original preview**, not a re-prepare.
3. ApprovalUse count reservation needs one terminal state when an authoritative `terminal_failed` decision is recorded, separate from cost-reservation semantics.

This proposal still opens only one narrow automatic author profile from the D10 candidate: existing Node + one Field + exactly one Entry in the current complete Field + one existing scalar member through the existing D7 `set_field_member`. Every other D7/D8/D3 author mutation remains interactively confirmed per operation.

The following remain unchanged:

- D3 wireVersion11, Result/9, modes, identity/lifecycle stages, and receipts;
- exact shapes of D6 `d6_commit_request` and `d6_commit_receipt`, the Workspace+OperationId ledger key, and the unique author commit point;
- exact shapes of D7 `ActionSpec`, `d7_action_prepare`, `d7_action_prepared`, and `PreparedActionBinding/2`;
- D7 `EffectManifest/1`, EffectBytes item/schema, and committed transport;
- D8 `PreparedEditBinding/1`, Draft/IME/explicit confirmation;
- D4 Registry, D7 Narrow Field Qualification, and D6 Policy/ObservationScope;
- existing current authorization, deny precedence, non-disclosure, authority/fence, dependency CAS, replay, and planned recovery.

The D6 `d6_error` object shape and disposition set remain, but the first jointly specified public unattended author-submit contract needs coordinated closed-set additions `approval_unavailable` and `execution_stopped`. Both are real closed-enum extensions rather than overloaded existing errors, and they create no anonymous old/new D10 error profile. D7 Preview/Effects needs a planned-only read-only recovery entrypoint while existing `d7_effects_resolve/open` remains committed-only. R05 also proposes Policy/2 `d10_control_self`, freezing the original non-Field set for bootstrap profile/2, and explicit profile/3. These remain design proposals: independent acceptance/coordinated activation is not product implementation or D1 capability availability.

## 2. D6 Storage Transactions Permissions and Sync — proposed additions

### 2.1 D10 delegation/approval is not a D6 permission source

Add after the D6 main-document current `PrincipalContext`, generation, and revocation rules:

> **D10 delegation/approval consumption.** D10 may provide current DelegationLease, StandingApproval, and PlannedDecisionApproval evidence from a host/Core managed control domain, but these can only narrow eligibility already granted by D6 Policy, ObservationScope, state disclosure, and actual-footprint gates. They cannot add a capability, turn deny into allow, or replace current authorization. Ordinary requests, Agents, workers, Connectors, MCP servers, and client JSON cannot self-assert principal, delegation, approval, approval count, or proof.
>
> Every plan, commit, saved-receipt delivery, and result/effects delivery using D10 delegation still performs existing D6 current-principal/delegation/policy/generation gates first. DelegationLease expiry, revocation, or unprovable time continuity prevents new protected steps. `maxRuns` exhaustion prevents only a **new Run that has no LeaseRunUse**; later steps of an already admitted Run and recovery of that Run's original planned request are not rejected again merely because remaining is zero, while current authorization, exact leaseRevision, trusted time, ActivationBinding, approval, and budget gates still apply. None of these control failures rewrites a committed decision as failure or rolls back an external effect that already crossed its send linearization point. An old approval or capability probe is never a durable authorization ticket.
>
> Only an author-submit profile explicitly listed by the D10/1 coordinated contract may use Standing Approval instead of per-operation interactive confirmation; the first profile is D7 `set_field_member` with `single_field_member`. Every other D3/D6/D7/D8 author intent follows its original confirmation contract. D10 approval cannot legalize an otherwise invalid, unobservable, stale, over-budget, or semantically conflicting plan.

### 2.2 ApprovalUse is additional authorization evidence and explicitly supports raw no-op

Add near the unique D6 author commit point and planning/commit dependency revalidation rules:

> For a D6-owned Action using D10 Standing Approval, after D7 prepare completes, Core independently constructs managed `ApprovalUse/1` from the saved PreparedActionBinding/2, complete owner_fields preview, actual MutationFootprint, current D6 authorization, and current D10 control records. The client still holds only the original `d6_commit_request`; it cannot add approved, approvalId, delegation, proof, budget override, or effect override to the request.
>
> `ApprovalUse/1` immutably binds approvalId/revision, Run/step, complete canonical D6 request, planToken, corresponding PreparedActionBinding, complete preview semantic binding, actual footprint proof, DelegationLease/ActivationBinding, approval-count reservation, and every applicable budget/cost reservation. It is additional authorization evidence, not a second author plan, second ledger, receipt, or capability token.
>
> Core may establish ApprovalUse only for `single_field_member`. Common prerequisites are: exactly one Entry in the current complete Field; original Action is `set_field_member`; owner, FieldId, memberPath, and memberType exactly match the envelope; value satisfies the closed constraint; D7 Narrow Field Qualification succeeds; and complete preview exists and is disclosable. There are only two allowed actual-effect branches:
>
> 1. **member-change:** MutationFootprint contains exactly the existing scalar-member change and the public owner_fields preview contains the exact `field_change`;
> 2. **raw-no-op:** the original Action still resolves to the same unique Entry/existing member, requested typed value equals the current value, and complete proposed source bytes equal before byte-for-byte. MutationFootprint is empty, preview contains no `field_change`, and final D6 receipt has empty `sourceVersions`; no effect, footprint, or revision may be invented.
>
> Any change to occurrenceKey, other member, qualifier, note, provenance, other Entry, body/title/coreKind/Facet, Ref/relation, identity/lifecycle/placement, or D6 control state makes standing approval inapplicable. Core never chooses first/preferred/same-value Entry and never degrades failure to whole-entry/source write.
>
> If a raw no-op forms a committed decision under original D6 rules, it still consumes one successful-commit approval count; replay of the same saved decision never consumes again.

### 2.3 The sole error for an in-D6 approval race

Add the following error contract to both the D6 main document and Control Interfaces:

> Before formal `d6_commit_request` entry, the D10 adapter may return D10 approval_required, approval_expired, delegation_expired, or delegation_exhausted from already observable control state. That probe does not eliminate later races.
>
> Once the unattended path has associated ApprovalUse as an internal dependency of the original planToken and entered D6, exactly one case uses new `d6_error.code="approval_unavailable"`, always with disposition `preflight`: the approval dependency becomes unusable in a race while original D6 current author authorization/ObservationScope and applicable business visibility still hold.
>
> Original D6 permission failure remains `not_visible/preflight`; business dependency, semantic, budget, authority, and integrity failures keep their original code/disposition and cannot be masked by `approval_unavailable`.
>
> For an unseen request, `approval_unavailable/preflight` writes no author ledger, recorded rejection, or planned decision. The original prepare may be retried under new valid approval while still valid, or the caller may return to an interactive path. For an already planned request, the same error leaves ledger=planned and preserves original plan/pins/reservations; it cannot become `semantic_rejected` or `terminal_failed`.
>
> The D10 adapter passes this D6 error through verbatim. UI that wants to distinguish revoked, expired, or race-lost approval performs a separately authorized D10 control read and does not smuggle control detail through the author error.
>
> This is part of the first jointly specified public D6-Control/1 unattended author-submit error set. Fixed-S D6 is an unpublished design contract rather than an already deployed old D10 profile. Any component combination that can enter the formal branch supports the same closed set; incompatible combinations are rejected at the D1 capability/version gate. Policy/1/2, bootstrap profile/1/2, and historical saved-decision decoders remain unchanged.

### 2.4 planned CAS atomically reserves

Extend D6 planned CAS:

> For an unseen D6 request carrying ApprovalUse, after original step-6 business, authorization, dependency, semantic, and budget validation and before planned is written, planning CAS also compares current approval/delegation/activation revisions, exact ApprovalUse binding to request/plan/preview/footprint, approval time/revoked state, `consumed + reserved < maxSuccessfulCommits`, and old versions of every applicable budget/cost reservation.
>
> Only when all are true may the same database write transaction store planned, the complete fixed plan/pins, ApprovalUse count `unreserved→reserved`, and related resource reservations. Two requests racing for the last approval count or budget produce at most one winner; the loser returns to the original restart gate and cannot reuse a previously observed balance.
>
> Replay of the same canonical Workspace+OperationId request remains bound to the same reservation and never reserves twice. Temporary ApprovalUse that never reaches planned may expire with preparation lifetime. Once referenced by planned, the approval reservation and recovery pins follow original ledger-recovery lifetime and are not reclaimed by preview TTL, Agent-session exit, or ordinary cache GC.

### 2.5 final author commit, terminal release, and replay

Extend D6 final transaction and recovery:

> For a planned decision with ApprovalUse, final author commit still revalidates current D6 authorization, authority/fence, original business dependencies, and complete plan. It additionally proves that ApprovalUse still binds the same request/plan and that current standing/supplemental approval permits this **new** author submission.
>
> On success, author payload/control effects, D6 committed decision, canonical receipt bytes, approval count `reserved→consumed`, ApprovalUse terminal link, applicable D6 budget control effects, and required audit link publish in the same final author transaction. A committed raw no-op also becomes consumed. Transaction rollback returns every item to the original planned/reserved state.
>
> If the original D6 ledger under complete continuity and current authorization proves the planned decision **can never commit** and records authoritative `terminal_failed` under existing rules, the same abort transaction changes ApprovalUse count reservation `reserved→released_terminal`. released_terminal does not count against maxSuccessfulCommits reserved/consumed total, but immutable history remains and terminal replay never releases twice.
>
> Only authoritative terminal_failed releases approval count. Run cancellation, preview/approval TTL, temporary revocation, Lease expiry, attempt/work pause, and temporarily unprovable authority/continuity are not aborts and retain the reservation.
>
> Approval count and cost reservations are separate. Author terminal_failed does not refund model/tool/external cost that occurred, may have occurred, or is unknown; D10 settled/released/uncertain rules apply independently.
>
> Delivery/replay of a committed decision does not require historical Standing Approval to remain unexpired. It requires continuous original saved-decision state and current caller authorization already required by the original protocol. Lost-receipt replay returns original bytes without another approval count, budget spend, or source revision.

## 3. D6 Control Interfaces — proposed additions/replacements

### 3.1 PreparedIntent producer classification

After §2's paragraph describing valid closed adapters that produce managed PreparedIntent, add:

> D10 Broker itself is not a PreparedIntent producer. A D10 Agent/Automation workspace mutation can only call an existing D7/D8/Core closed adapter. For `single_field_member`, D7 prepare first produces PreparedActionBinding/2, PreparedIntent, preview, and the original `d6_commit_request` under its normal contract; only then may a Core-managed D10 approval adapter build ApprovalUse from protected records. D10 runtime cannot directly construct PreparedIntent, MutationFootprint, source bytes, or proof.
>
> ApprovalUse changes no exact `d6_commit_request` member, canonical request key, planToken tag, protocolOwner, or ledger key. It is associated with the original planToken only through protected internal records; no external caller can append an approval field.

### 3.2 Complete additions to §2 steps 5–8

For the D10 standing-approval profile, original steps 1–8 remain with these additions only:

> **Step 5 addition:** after resolving this principal's planToken, when the unattended path has associated ApprovalUse, use only the already authorized minimal control mapping to locate its profile/record. missing, wrong audience, or wrong Workspace stays `not_visible`. Complete ApprovalUse is read only after current D6 authorization/ObservationScope passes.
>
> **Step 6 addition:** besides original business dependency, semantic, and budget validation, verify both `single_field_member` effect branches, current approval/delegation/activation, and count/cost reservation candidate. If original D6 permission or business semantics fail, return the original owner error. Only when those still hold and ApprovalUse alone cannot be consumed return `d6_error approval_unavailable/preflight`. An unseen request writes no decision.
>
> **Step 7 addition:** planned CAS atomically stores plan/pins and changes ApprovalUse count `unreserved→reserved`; a CAS loser returns to original step 3. No reservation creates author effects early.
>
> **Step 8 addition:** before planned recovery or final author commit, recheck current D6 authorization and current approval eligibility. If approval alone is unavailable, return `approval_unavailable/preflight` and keep planned. Deterministic business conflict follows the original terminal_failed rules. Successful commit changes count `reserved→consumed` in the same transaction; authoritative terminal_failed changes it `reserved→released_terminal` in the same abort transaction.

### 3.3 Exact extension of §3 receipt and error contract

Extend the `d6_error.code` closed set to:

```text
invalid_request
not_visible
authority_unavailable
integrity_conflict
operation_id_conflict
plan_expired
approval_unavailable
execution_stopped
dependency_conflict
semantic_rejected
budget_exceeded
transaction_aborted
```

`approval_unavailable` permits only disposition=`preflight`. It means the coordinated D10 author-submit approval dependency cannot be consumed after D6 entry while original D6 author permission/ObservationScope and applicable business prerequisites did not fail earlier. It creates no recorded rejection and reveals no internal approval cause.

`execution_stopped` likewise permits only disposition=`preflight`, after current authorization/ObservationScope succeeds. It means only that an irreversible ExecutionStopLatch bound to this request has won the linearization ordering and blocks a new author submit. It cannot replace `not_visible`, approval errors, temporary disable, Lease expiry, or business dependency errors. An unseen request writes no ledger; a planned request follows the strict authoritative-abort conditions in §3.5.

Original phase/key/permission/availability rules remain; `dependency_conflict|semantic_rejected|budget_exceeded` remain recorded only under the original step-6 contract, and permanent abort after planned remains only `transaction_aborted|terminal`. A consumer that did not negotiate D10 standing-approval capability cannot receive the new enum.

Standing Approval adds no receipt member. D10 UI may display use origin from an authorized Run/ApprovalUse control state, but that control evidence is not an author receipt.

### 3.4 D10 control state and Lease Run admission

Durable D10 control state such as ActivationBinding, DelegationLease, StandingApprovalEnvelope, AutomationDefinition, LeaseRunUse, and PlannedDecisionApproval may be changed only through Core-managed closed control adapters with independent control revision/CAS, current-principal authorization, audit, and replay contracts. They cannot be changed through author source, provider callback, Agent JSON, or free payload in `d6_commit_request`.

`maxRuns` is consumed by a D10 Run-admission CAS immediately before the first protected execution and never enters the D6 author ledger. CAS accumulates uses over one `leaseId` lineage and writes `LeaseRunUse/1`; restart of the same Run does not consume again, and failure/cancellation/crash after admission never refunds it. Only a **new Run with no existing LeaseRunUse** returns D10 `delegation_exhausted` when cumulative use has reached `maxRuns`. Later protected steps of an already admitted Run and recovery of that Run's original planned request do not compare remaining count again and do not consume again; even when `maxRuns=1` and cumulative use is already 1, that same Run is not rejected as a new Run. Those later steps still revalidate current authorization, exact `leaseRevision`, trusted time, ActivationBinding, approval, and budgets; revocation, expiry, revision/binding change, or unprovable continuity still blocks execution.

### 3.5 D10 Workspace self-management, bootstrap profile/3, and irreversible stop

This is an explicit coordinated amendment to fixed-S D6 Control and does not modify the S snapshot.

Add one no-argument Policy/2 capability d10_control_self, valid only at workspace scope. It lets the current principal use the R05 CONTROL-CONTRACT closed Workspace control adapter to manage that principal's own finite Automation/Lease/Approval/Run control records. It implies no source/Field read-write, policy_admin, registry_admin, binding_admin, repair, commit_sequence_state, or deployment resource. No other capability implies it and deny precedence remains.

The Workspace D10 control adapter is a managed PreparedIntent producer only for CONTROL-CONTRACT §7 Workspace bodies automation_configure, consent, state, workspace_limits, and activation. It cannot accept deployment_put, cost_reconcile, secret bytes, or a free callback. The producer derives the original d6_commit_request from the real current principal, complete closed body, authorized reads, and stable prepare binding; an external caller still cannot assert principal, authorized, effect, or writer. Any final author-payload mutation can still come only from the original D7/D8/D3 adapter; an ordinary D10 control mutation cannot construct author source bytes.

Under this amendment the fixed-S profile/2 phrase “all non-Field capabilities” is frozen to the controlled set `workspace_state, entity_state, locator_state, source_read, source_write, body_write, node_control, node_create, resource_read, resource_write, annotation_read, annotation_write, lifecycle, registry_admin, binding_admin, policy_admin, export, repair, audit, source_envelope_state, commit_sequence_state` present in S. Later capabilities never enter profile/2 automatically.

Add d6_bootstrap_profile wireVersion=3 with the same members kind,wireVersion,profileRevision,registrySeedBinding,newSeriesMultiplicity,initialPeriodScope. profile/3 still creates initialPolicy.version=2. Its creator Workspace grant equals the frozen profile/2 non-Field set above plus d10_control_self plus the original S rule generating all target-Registry Field read/write grants; deny is empty.

Only an explicit issuer-profile management operation by current administer_issuer can select profile/3 for subsequently issued families. Stored profile/1/2 copies of existing families, replacement, WorkspaceBootstrapPlan, saved decisions, replay/continue/failover are never recomputed or augmented. An existing Workspace acquires d10_control_self only through the original Policy-management transaction by current policy_admin; a Field-authorized principal cannot self-grant it.

The first jointly specified public d6_error.code set also adds execution_stopped, valid only with disposition=preflight. For an unseen D10-bound author request, current authorization/ObservationScope succeeds first; then D6 checks the protected RunBinding's irreversible stop latch. If stop already linearized, return execution_stopped/preflight with no author decision.

D6 final author commit revalidates the same RunBinding stop latches inside the real final authority-store write serialization. A prepare-time or out-of-transaction check is insufficient. Stop linearized first means the new author commit does not occur; commit linearized first preserves the committed decision and later stop cannot roll it back.

For an already planned request, stop cannot directly write a terminal result. Only when current authorization, authority/continuity, the original complete plan/RunBinding, and irreversible stop are all proven, and original D6 recovery proves the plan can never submit, may the original author ledger write transaction_aborted/terminal. The same abort transaction follows the existing ApprovalUse rule for releasing approval-count reservation. Temporary disable, Lease expiry, ordinary Run cancellation, temporary authorization loss, or temporarily unprovable clock/state is not that proof and leaves the request planned. Cost reservation is never automatically released by author abort.

Stop does not block currently authorized authoritative abort, cost settlement, evidence/audit retention, or reference-safe cleanup. Those operations do not restore executor or author-write authority.

## 4. D7 Execution and Action Interfaces — proposed replacement/additions

### 4.1 ActionSpec default confirmation statement

Replace the current §5 semantics that the user confirms through the original D3/D6 submission after seeing preview with:

> Default ActionSpec confirmation remains: after obtaining and validating a complete current preview, the current user explicitly sends the original D3/D6 request returned by prepare. Only the D10/1 coordinated `single_field_member` profile may continue without this operation's human click: Core must mechanically prove valid ApprovalUse from the complete PreparedActionBinding/2, EffectManifest/EffectBytes, actual MutationFootprint, and current authorization of this fresh D7 prepare. This exception creates no D7 confirmation token, changes no request, and does not claim a user read preview item by item. Every other ActionSpec kind and D3-owned intent keeps original interactive confirmation.

### 4.2 standing-approval eligibility including raw no-op

Add before §6 prepare/commit:

> D7 prepare never knows or trusts caller-asserted approval. It still completes original closed intent decode, potential observation, authority/cut, source/selector/definition/rule/dependency, proposed source/MutationFootprint, D2/D4/D5/D7 gates, and immutable prepare/preview. Only after all succeed may a Core-managed D10 adapter attempt a match.
>
> The only current profile requires ActionSpec.intent.kind exactly `set_field_member`, exactly one Entry in the current complete Field, memberPath naming an existing scalar member, exact owner/Field/member type/value constraint, and successful Narrow Field Qualification. Actual effect is only:
>
> - member-change: MutationFootprint is exactly the selected member and owner_fields preview contains the exact `field_change`;
> - raw-no-op: requested value already equals the current typed member and complete proposed source equals before bytes; MutationFootprint is empty and preview contains no `field_change`.
>
> eligibility=false does not change legality of the original Action. An Action that can be submitted interactively transitions to awaiting_confirmation; an invalid Action returns the original D7 error. D7 gains no looser parser, selector, permission, or semantic fallback for automation.

### 4.3 replay and restart

Add:

> Standing Approval does not change D7 OperationId/replay. A fresh automation occurrence requires fresh prepare; loss of a receipt for the same committed request only resends the original request. D10/Automation restart cannot create a semantically equivalent new OperationId merely because the original preview transport expired.
>
> Existing ActionEvidence, FieldSelection, result epoch, source revision, and preview delivery-epoch invalidation rules remain. Standing Approval cannot revive stale evidence/selector. If a decision has not reached planned and must be re-prepared, it gets a new request/OperationId and a new approval use; old mechanical approval does not carry forward.

### 4.4 Irreversible stop binding for D7 prepare

D7 ActionSpec, PreparedActionBinding/2, and EffectManifest/EffectBytes add no caller-supplied stop field. For D10 unattended author-submit, after successful prepare Core creates a protected internal association from planToken to the current Run/Automation/Lease and its ExecutionStopLatch refs; the caller cannot remove, replace, or self-assert those refs. Final submission is rechecked by D6 §3.5 inside the real transaction. The D7 interactive path gains no new automatic-confirmation authority from stop; stop only prevents a background submit that has not yet linearized.

## 5. D7 Preview and Effects Transport — planned recovery extension

### 5.1 New read-only planned-preview entrypoint

Existing `d7_effects_resolve` remains committed-only, with planned/rejected/terminal/unseen still returning `effects_unavailable`. Existing `d7_effects_open` continues to accept committed `effectsToken` only.

Add:

```text
d7_planned_preview_open {
  wireVersion: 1,
  kind: "d7_planned_preview_open",
  protocolOwner: "D6",
  request: <original d6_commit_request>
}

d7_planned_preview_opened {
  wireVersion: 1,
  kind: "d7_planned_preview_opened",
  request,
  previewToken,
  previewCursorToken,
  previewManifest
}
```

It is not prepare, commit, resolve, or author-state mutation. Input is the complete original D6 request; OperationId-only, planToken-only, or approvalId-only ledger probing is forbidden.

The unique order is:

1. closed decode;
2. current authenticated audience and protected minimal locator mapping;
3. original D6 current authorization, original ObservationScope, and complete-preview disclosure eligibility;
4. current authority/custody/ledger continuity;
5. byte-equal canonical request at the same key with ledger state=planned;
6. complete original PreparedActionBinding/2, semantic preview, and all required pins;
7. create a new finite recovery delivery epoch and return a fresh action_preview token/cursor/header.

The new epoch's semantic items, order, EffectBytes payloads, and profile come byte-equivalently from the immutable preview saved with the planned decision. It cannot rerun Query, parse a current drifted definition, reselect a target, regenerate proposed source, or switch Registry. It only reissues transport handles.

The recovery epoch has an independent finite TTL and neither extends nor revives old preview token/cursor. Page and EffectBytes continue to use original D7 transport. Expired recovery epoch uses `preview_expired`; proven epoch invalidation uses `reset_required`; unprovable pin/continuity or request/state mismatch uses `effects_unavailable`; earlier authorization failure remains `not_visible`. No new effects-error enum is added.

### 5.2 Interactive approval consumption boundary

Only after manifest, every page, and every required EffectBytes has been completely obtained under one recovery epoch and decoded without gaps may UI create D10 `PlannedDecisionApproval/1`.

The record binds the exact original request, original planned decision, original immutable preview semantic record, current granting principal/DelegationLease/ActivationBinding, and finite grant lifetime. It does not bind the short-lived preview token itself and changes no request, OperationId, PreparedActionBinding, plan, or target.

Final submission still uses the original D6 request and §2/§3 current authorization, approval, dependency, and authority/fence checks. New interactive approval only proves that the user has freshly inspected and authorized the original plan; it cannot legalize a deterministic dependency conflict, stale target, or damaged pins.

If the original plan already holds a standing ApprovalUse count reservation, that reservation remains occupied during interactive rescue. Successful author commit still changes `reserved→consumed`; only authoritative terminal_failed changes `reserved→released_terminal`. The interactive click cannot release the slot early for a second Run.

## 6. D3 Terminology and Naming Lexicon — minimal erratum proposal

The D3 lexicon at fixed U contains one internal owned-name conflict between approximately lines 377/379 under `weftext.term.origin-binding` and lines 496/498 under `weftext.term.adopt`. `weftext.term.origin-binding` already owns code name `OriginBinding` and variable name `origin_binding`, while `weftext.term.adopt` says that variable `adoption_binding` must map to OriginBinding and also registers `adoption_binding` under Adopt `owned-names.codeConventions`. That gives one association-binding concept a second variable naming convention.

Propose only the following minimal erratum, with no change to identity, wire, capability, operation semantics, or the frozen OriginBinding type:

1. For `weftext.term.adopt`, change the Adopt wire/API note to: D3 freezes no mode; future API/code verbs use only `adopt_*`. When Adopt creates or re-establishes a foreign-object association, the binding value uses the existing D3 `OriginBinding(ForeignIdentityKey, NodeRef)` type and existing variable/field name `origin_binding`; Adopt defines no `adoption_binding`.
2. Change Adopt `owned-names.codeConventions` from `["adopt_*","adoption_binding"]` to `["adopt_*"]`.
3. Keep `weftext.term.origin-binding` owned names `OriginBinding` / `origin_binding` unchanged and uniquely owning this binding-name family.
4. Delete the `adoption_binding` convention with no compatibility alias, dual read, migration alias, or second variable name; it does not enter public wire, CLI, locale, identity, or capability.

Positive validation: an Adopt code path may be named `adopt_*`, but the associated binding value type and variable must resolve to `weftext.term.origin-binding` `OriginBinding` / `origin_binding`. Negative validation: controlled positive source, API/schema, fixtures, and terminology registry must not assign `adoption_binding` to Adopt or treat it as a compatibility alias for OriginBinding; scanners may not hide remnants through exceptions.

This is a minimal coordinated D3 terminology erratum proposal. Fixed U snapshots remain unchanged, and this document cannot claim the erratum is effective before independent re-review and coordinated acceptance.

## 7. Pack parent lifecycle — no new D1/D4 wire amendment

The Pack parent-domain/extension-point lifecycle selected by CANDIDATE §6.1 requires no change to fixed D1/D4 wire for these reasons:

1. D1 already freezes the closed `unsupported_surface|not_in_release|policy_denied|user_action_required|missing_component|not_configured|offline|incompatible_version|temporarily_unavailable` reasons and fixed precedence. D10 only maps the actual parent-dependency blocker onto those existing reasons and adds no product-level `parent_missing` reason.
2. D4 already freezes `RegistrySnapshot/1`, `RegistryBinding/1`, namespace definition `complete|unavailable`, raw-source preservation, generation+digest binding, and cumulative tombstone/migration/semantic-contribution ledger. D10 cannot rewrite those rules when a Pack or parent is disabled.
3. Parent Extension Dependency exists only in the D10 Capability Catalog/ActivationBinding deployment control plane. A domain Contribution declares parent domain, extension point, and version range; activation resolves an exact parent binding and covers it in the Catalog digest. It does not enter D4 Field/Facet schema and is not an author fact.
4. UI visibility remains a product projection. Hiding only the parent-module UI changes no D1 capability, D4 RegistryBinding, or D10 Contribution activation.

Therefore parent missing/disabled/incompatible/unsupported-surface makes dependent runtime/View/Action/rule Contributions inactive while accepted D4 definitions/history/raw author source remain under original D4 complete/unavailable semantics. If exact definitions remain complete, Core may interpret existing author facts without authorizing new domain behavior. If definitions cannot be proven, original `provider_or_schema_unavailable`/raw-retention behavior applies.

Upstream must be reopened only if a future design wants any of these: parent-dependency members inside `RegistrySnapshot/1`; a new D1 unavailable reason; UI disable changing schema availability; dependent domain behavior continuing under a disabled parent; or changed D4 semantic-ledger retention. The current candidate rejects all of those routes.

## 8. Explicit non-amendment for D8 and D9

D8 gains no unattended edit branch. Document/Annotation edit, dirty Draft, IME, current serial, complete preview, and explicit confirmation remain unchanged. Agent/Automation may only propose into D8 and cannot fabricate EditSession or human origin.

D9 does not change Provider/Route, worker sandbox, ExportPlan, LossReport, PublicationReceipt, or external publisher. D10 external transport/Connector grants no network to a D9 worker, and D9 publication confirmation grants no Standing Approval author write.

## 9. Activation and version compatibility

These amendments change the specification only after independent acceptance, controller coordination, and formation of a subsequent design version with the D10 candidate. **Design acceptance/coordinated activation is not product implementation, release, or runtime availability evidence.**

Candidate first-public D10 capability IDs are defined by D10 and discovered/published by D1: automation.manage, workspace.extensions.manage, deployment.external.manage, automation.stop, and automation.author_submit. They first enter the official capability catalog of the selected D1 contractMajor and remain subject to D1-frozen release, surface, policy, principal, component/configuration, reachability/version-combination, and health gates. Unknown IDs remain D1 unsupported_feature; disallowed component-version combinations remain incompatible_version. D10/D6 adds no second product-level negotiator.

When the capability is actually released, the first jointly specified public unattended author-submit D6-Control/1 error set already contains approval_unavailable and execution_stopped. There is no legacy D10 author-submit error profile, enum fallback, migration parser, or dual-read/dual-write. Only the unpublished D10-profile assumption is removed; S-frozen Policy/1/2, bootstrap profile/1/2, IssuerControlPolicy, D1 bootstrap/contractMajor, and historical saved-decision decoders all remain.

Before runtime release, every D7/D8 author proposal continues to use current per-operation confirmation and the planned-preview recovery entrypoint plus unattended author submit remain truthfully unavailable under D1. Broker, Agent proposals, Tool/MCP, Connector read, and external-effect control that do not perform unattended author submit may progress independently, but each still requires its own D1 capability and real implementation evidence.

Historical committed D3/D6 decisions replay under their original decoder/bytes and are not backfilled with ApprovalUse, stop, or d10_control_self. Existing families keep their fixed bootstrap profile; no upgrade path silently reinterprets profile/2 as profile/3.

## 10. Joint counterexamples required for independent re-review

1. R1 prepare sees valid approval, R2 consumes the final use; when R1 reaches D6 and only approval lost the race, the result must be `approval_unavailable/preflight` and an unseen request has no ledger decision.
2. R1 is already planned when approval is revoked; the same error leaves it planned and cannot become semantic_rejected/terminal_failed.
3. A new client has no old preview copy but is currently authorized and continuity holds; `d7_planned_preview_open` can deliver the complete original preview without re-prepare.
4. Recovery preview cannot rerun Query or change target; a drifted current definition does not change saved semantics.
5. New PlannedDecisionApproval cannot revive deterministic dependency conflict.
6. Approval count=1 with two Runs planning concurrently yields at most one `reserved`.
7. A reserved plan later reaches authoritative terminal_failed after deterministic dependency change; the same abort transaction records `released_terminal`, and replay does not release twice.
8. Cancellation, TTL, temporary revocation, and Lease expiry do not release approval reservation.
9. Model/tool cost that occurred or remains uncertain is unaffected by author terminal_failed.
10. A set_field_member request whose value already equals current produces empty MutationFootprint, empty field_change, and empty sourceVersions while still validating target/type/value/permission/dependencies; a committed decision consumes one approval count.
11. Two same-value Field Entries cannot select first.
12. A footprint mutant that changes note, provenance, Facet, or a second member rejects unattended approval.
13. Author commit succeeds, receipt is lost, and approval later expires: replay returns original receipt without another count.
14. Valid background author commit with a dirty D8 Draft does not overwrite the Draft.
15. D3 create/lifecycle Action cannot use a `single_field_member` envelope for unattended submission.
16. A D10 control error cannot wrap or leak original D6 `not_visible` and hidden facts.
17. With `maxRuns=1` and an existing `LeaseRunUse/1` for the same Run, its second protected step and original planned recovery cannot return `delegation_exhausted` merely because remaining is zero; only a new Run is rejected.
18. Positive Adopt code keeps only `adopt_*`, while association binding uses only `OriginBinding` / `origin_binding`; `adoption_binding` must be absent from positive terminology/code surfaces and must not exist as an alias.
19. A profile/2 family must not gain d10_control_self after software upgrade; profile/3 affects only new families after explicit issuer update, and an existing Workspace can receive it only through current policy_admin.
20. Both orderings of stop versus new Run admission and D6 final author commit must have exactly one linearized result; authorized authoritative abort, cost settlement, and evidence retention still work after stop.
21. A stable Workspace-control request succeeded but its response was lost and another request later changed the object revision; after current disclosure authorization, retry must replay the old result without duplicate mutation and without falsely rejecting historical success because current revision changed.

These are author-revision review targets, not a claim that the first two independent-review batches are closed. Only later independent re-review can change their review status.
