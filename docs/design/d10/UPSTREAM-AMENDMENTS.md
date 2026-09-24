---
source_language: zh-CN
translation_of: UPSTREAM-AMENDMENTS.zh-CN.md
translation_status: synced
---

[简体中文](UPSTREAM-AMENDMENTS.zh-CN.md)

# D10 Coordinated D6/D7 Upstream Amendment Proposal

revision: D10-r01-candidate-2026-09-25; status: candidate upstream amendment proposal, not jointly accepted or coordinatedly activated. Fixed upstream input commit is `f205831c848729f7ddbc3ba0cf32b689459c0c98`. Current D1-D9 snapshots remain authoritative. This document gives independent review complete future companion text that can be mechanically compared; it modifies no snapshot and does not authorize early product implementation of unattended author submission.

## 1. Purpose and unchanged boundaries

Existing D7 already freezes fresh Action prepare, complete preview/effects, original D3/D6 request, replay, and unknown recovery. Existing D6 freezes current authorization, planned/commit CAS, the sole author ledger, and the final author commit point. The missing contract is how a future Standing Approval is mechanically validated, counted, revoked, and recovered before final submission when D7 otherwise describes user confirmation after preview.

This proposal opens only one narrow profile defined by the D10 candidate: an existing Node + one Field + exactly one Entry in the current complete Field + one existing scalar member using the existing D7 `set_field_member`. All other D7/D8/D3 author mutations remain interactively confirmed per operation.

The following remain unchanged:

- D3 wireVersion11, Result/9, modes, identity/lifecycle stages, and receipts;
- shapes of D6 `d6_commit_request`, `d6_commit_receipt`, `d6_error`, and the Workspace+OperationId ledger key;
- exact shape of D7 `ActionSpec`, `d7_action_prepare`, `d7_action_prepared`, and `PreparedActionBinding/2`;
- D7 `EffectManifest/1`, EffectBytes, delivery epoch, and preview/committed transport;
- D8 `PreparedEditBinding/1`, Draft/IME/explicit confirmation;
- D4 Registry, D7 Narrow Field Qualification, and D6 Policy/ObservationScope;
- existing current authorization, deny precedence, non-disclosure, authority/fence, dependency CAS, replay, and planned recovery.

Therefore this amendment does not require a D3 mirror update, D7 binding version 3, D6 wire version 2, or a new protocolOwner.

## 2. D6 Storage Transactions Permissions and Sync — proposed additions

### 2.1 Add after the permission model: D10 delegation and approval are not D6 permission sources

Add the following complete normative text after the current `PrincipalContext` and generation/revocation rules in the D6 main document:

> **D10 delegation/approval consumption.** D10 may provide current DelegationLease and approval evidence from a host/Core managed control domain, but these can only narrow eligibility already granted by this section's D6 Policy, ObservationScope, state-disclosure, and actual-footprint gates. They can never add a capability, turn deny into allow, or replace current authorization. Ordinary requests, Agents, workers, Connectors, MCP servers, and client JSON cannot self-assert principal, delegation, standing approval, approval count, or proof.
>
> Every plan, commit, saved-receipt delivery, and result/effects delivery using D10 delegation still performs this document's existing current-principal/delegation/policy/generation gates first. DelegationLease expiry or revocation prevents new protected steps but does not rewrite a committed decision as failure and does not roll back an external effect that has already crossed its send linearization point. An old approval or capability probe is never a durable authorization ticket.
>
> Only an author-submit profile explicitly named by the D10/1 coordinated contract may use Standing Approval instead of this operation's interactive confirmation; the first profile is D7 `set_field_member` with `single_field_member`. Every other D3/D6/D7/D8 author intent follows its original confirmation contract. D10 approval cannot legalize a D6/D7 plan that is otherwise invalid, unobservable, semantically conflicting, over budget, or stale.

### 2.2 Add before the author commit point: ApprovalUse is additional authorization evidence

Add the following complete text near the D6 main-document unique author commit point and planning/commit dependency revalidation rules:

> For a D6-owned Action using D10 Standing Approval, after D7 prepare is complete, Core must independently construct a managed `ApprovalUse/1` from the saved PreparedActionBinding/2, complete owner_fields preview, actual MutationFootprint, current D6 authorization, and current D10 control records. The client still holds only the original `d6_commit_request`; it cannot add `approved`, approvalId, delegation, proof, budget override, or effect override to the request.
>
> `ApprovalUse/1` immutably binds at least approvalId/revision, Run/step, complete canonical D6 request, planToken, the corresponding PreparedActionBinding record, complete preview semantic binding, actual footprint proof, DelegationLease/ActivationBinding, approval-count reservation, and all applicable budget/cost reservations. It is additional authorization evidence for current author submission and is not a second author plan, second ledger, receipt, or capability token.
>
> Core may construct ApprovalUse only for the D10 `single_field_member` profile: the current complete Field has exactly one Entry; the Action is the original `set_field_member`; owner, FieldId, memberPath, and memberType exactly match the envelope; value satisfies the envelope's closed constraint; D7 Narrow Field Qualification succeeds; the complete preview has been formed and is deliverable; and the actual footprint changes only one existing scalar member of that existing Entry. Any change to occurrenceKey, other members, qualifiers, note, provenance, other Entries, body/title/coreKind/Facet, Ref/relation, identity/lifecycle/placement, or D6 control state makes standing approval inapplicable. Core does not select first/preferred/same-value Entry and does not degrade failure to whole-entry/source write.
>
> Standing Approval cannot hide an upstream error. If the Action/Field/preview/D6 permission itself fails, return the original D7/D6 error. If the business plan is valid but no approval can be consumed, the D10 caller receives its own approval_required/expired control result and may move to interactive confirmation; this is not recorded as D6 semantic_rejected.

### 2.3 Add to planned CAS: atomic reservation

Extend the D6 main-document planned CAS contract with:

> For an unseen D6 request with ApprovalUse, after all original step-6 business/authorization/dependency checks pass and before planned is written, planning CAS also compares current approval/delegation/activation revisions, exact binding between the original ApprovalUse and request/plan/preview/footprint, approval validity and non-revoked state, `committed + reserved < maxSuccessfulCommits`, and prior versions of every applicable budget/cost reservation. Only when all are true may the same database write transaction store planned, the complete fixed plan/pins, ApprovalUse reserved, approval-count reserved, and related resource reservations.
>
> When two concurrent requests race for the last approval count or budget, at most one CAS wins. The loser returns to the original D6 restart gate and cannot treat a previously read balance as current fact. reserved is not committed and does not modify author source early. Replay of the same canonical request under the same Workspace+OperationId is associated with the same reservation and never reserves again.
>
> Prepare/preview records that never become planned may release temporary resources after expiry. Once planned references ApprovalUse/reservations, those records follow the original ledger-recovery lifetime and are not reclaimed by preview TTL, Agent-session exit, or ordinary cache GC.

### 2.4 Add to final author commit: atomic consumption

Add to the D6 main-document final database transaction:

> For a planned decision with ApprovalUse, final author commit still revalidates current D6 authorization, authority/fence, original business dependencies, and the complete original plan. It also compares that ApprovalUse still binds the same plan/request, the reserved record is complete, and current delegation/approval has not made this new author submission ineligible. Standing Approval expiry/revocation affects author submission that has not linearized; if final commit wins the author-decision CAS before the managed expiry/revoke transaction it may complete, otherwise it remains planned/blocked and revocation is not persisted as a permanent business rejection.
>
> When all checks pass, author payload/control effects, the D6 committed decision, canonical receipt bytes, approval-count reserved→consumed, ApprovalUse terminal link, applicable budget settlement/control effects, and required audit link are published in the same final author transaction. A raw-source no-op that forms a committed decision under original D6 rules still counts as one successful commit and consumes one approval count; replay of the same saved decision does not consume again.
>
> If the final author transaction rolls back, author source, receipt, approval-count consumed, and settlement all roll back to the original planned/reserved state. The system cannot observe "approval consumed but author not committed" or the reverse half-state. Later derived audit aggregation/telemetry failure cannot invite a duplicate author request; recovery consults only the original ledger/control records.

### 2.5 Add to journal/recovery: cancellation, expiry, and supplemental approval

Add after the D6 main-document recovery table and planned-recovery rules:

> A planned decision does not automatically become terminal_failed because its Run is cancelled or its DelegationLease/Standing Approval expires or is revoked. Recovery first performs original current authorization/non-disclosure, then validates the original plan/dependencies and approval evidence needed for this submission. When current approval is insufficient, remain planned/blocked; do not create a new OperationId, rerun Query/resample the target, or modify the original request or PreparedActionBinding.
>
> A user may give new explicit one-shot interactive authorization for this exact original planned decision; Core binds it as new supplemental authorization evidence to the original plan rather than replacing the original StandingApprovalEnvelope. If original business dependencies have a deterministic conflict, new approval cannot revive the plan and the existing authoritative-abort conditions still apply.
>
> Delivery/replay of a committed decision does not require the historical Standing Approval to remain unexpired. It requires only continuous original saved-decision state and the current caller authorization already required by the original protocol. Lost-receipt replay returns original bytes and does not increment approval count, budget spend, or author revision.

## 3. D6 Control Interfaces — proposed additions/replacements

### 3.1 Add a producer-classification paragraph after §2 PreparedIntent

After the paragraph describing valid closed adapters that can generate managed PreparedIntent, add:

> D10 Broker itself is not a PreparedIntent producer. When a D10 Agent/Automation proposes a workspace mutation, it can only call an existing D7/D8/Core closed adapter. For the D7 `single_field_member` standing-approval profile, D7 prepare first creates the complete PreparedActionBinding/2, PreparedIntent, preview, and original `d6_commit_request` under its normal contract; only then may a Core-managed D10 approval adapter read the protected prepare record and establish ApprovalUse. D10 runtime cannot directly construct PreparedIntent, MutationFootprint, source bytes, or proof.
>
> ApprovalUse does not change the exact members of `d6_commit_request`, canonical request key, planToken tag, protocolOwner, or ledger key. It is uniquely associated with the original request through planToken/protected internal records; no external caller can append an approval field.

### 3.2 Complete augmentation of §2 unique order

For the D10 standing-approval profile, original D6 steps 1-8 remain, with only these additional checks in steps 5-8:

> **Step 5 addition:** after resolving this principal's planToken, when its PreparedIntent requires D10 Standing Approval, use only the already-authorized minimal control mapping to identify approval profile and ApprovalUse locator. missing/wrong-audience/wrong-workspace remains not_visible. The complete ApprovalUse may be read only after current D6 authorization/ObservationScope has passed.
>
> **Step 6 addition:** besides original business semantic/dependency/budget validation, Core validates D10 `single_field_member` eligibility, current approval/delegation/activation, complete preview/footprint, and count/cost reservation candidate. Business errors remain original D6/D7 owner errors. If the business plan is valid but D10 approval alone is unavailable, the calling D10 adapter returns approval_required/approval_expired and does not persist a D6 recorded rejection.
>
> **Step 7 addition:** planned CAS atomically reserves ApprovalUse, approval count, and budgets together with the complete plan/pins. A CAS loser returns to original step 3. No reservation can create an author effect early.
>
> **Step 8 addition:** final author commit revalidates current authorization and final eligibility of ApprovalUse and consumes approval count/settles applicable managed budget in the same transaction. Failure/crash follows original D6 planned recovery and never creates a separate D10 author decision.

### 3.3 §3 receipt and error wire remains unchanged

Add:

> Standing Approval adds no member/code to `d6_commit_receipt` or `d6_error`. The author-protocol caller sees the original D6 commit/replay bytes. D10 UI/Broker may additionally show, from an authorized D10 Run/ApprovalUse record, that standing approval was consumed, but this is control evidence and is not written into the receipt.
>
> approval_required, approval_expired, delegation_expired, control_conflict, and other D10 control errors are returned only by D10 control adapters. Once formal submission enters `d6_commit_request`, original D6 error/disposition has priority and is not wrapped. Loss of current author permission continues to be original not_visible rather than an approval error.

### 3.4 Add the general boundary for managed D10 control intents

Add to the D6 Control Interfaces managed-configuration area without freezing every D10 public wire in this amendment:

> Durable D10 control state such as ActivationBinding, DelegationLease, StandingApprovalEnvelope, and AutomationDefinition may be changed only by Core-managed closed control adapters with independent control revision/CAS, current-principal authorization, complete preview/audit, and replay contracts. It cannot be mutated through free payload in `d6_commit_request`, ordinary author source, provider callback, or Agent JSON.
>
> This D10 candidate freezes those objects' semantics and their composition with author submission. Exact public IPC/HTTP envelopes are an implementation-interface choice, but may not weaken closed decode, version/CAS, non-disclosure, audit, or current-authorization obligations. Any future decision to route a D10 control mutation through a generic D6 management request must reuse the existing unique ledger/authoritative control transaction and cannot create an unledgered management bypass.

## 4. D7 Execution and Action Interfaces — proposed replacement/additions

### 4.1 Replace the ActionSpec overview confirmation sentence

Fixed-upstream D7 Execution/Action §5 currently describes the user confirming through the original D3/D6 submission after viewing preview. Replace that statement completely with:

> The default confirmation for ActionSpec remains: after obtaining and validating a complete current preview, the current user explicitly sends the original D3/D6 request returned by prepare. Only the D10/1 coordinated `single_field_member` profile may continue without a per-operation human click: Core must mechanically prove a valid ApprovalUse from this fresh D7 prepare's complete PreparedActionBinding/2, EffectManifest/EffectBytes, actual MutationFootprint, and current authorization. This exception creates no D7 confirmation token, changes no request, and does not claim that a user read the preview item by item. Every other ActionSpec kind and D3-owned intent continues to require original interactive confirmation unless a future coordinated contract explicitly opens it item by item.

### 4.2 Add standing-approval eligibility before §6 prepare/commit

Add this complete section:

> **D10 standing-approval eligibility.** D7 prepare never knows or trusts caller-asserted approval. It still completes its original sequence of closed intent decode, potential observation, authority/cut, source/selector/definition/rule/dependency, proposed source/MutationFootprint, D2/D4/D5/D7 gates, and immutable prepare/preview. Only after all succeed may a Core-managed D10 adapter attempt a match.
>
> The only current profile requires ActionSpec.intent.kind exactly `set_field_member`; selector points to exactly one Entry in the current complete Field; memberPath is a static object-member-name path terminating at an existing scalar member; owner/Field/member type exactly matches StandingApprovalEnvelope; value passes the original TypedLiteral/D4 decoder and the envelope valueConstraint; Narrow Field Qualification succeeds for the same RegistryBinding/current source; and MutationFootprint contains exactly that member-value change. Any Entry insertion/removal, occurrenceKey/qualifier/note/provenance change, other member, Facet/body/title/coreKind change, relation/ref, identity/lifecycle/placement/control effect, or effect requiring full-source preview makes eligibility=false.
>
> eligibility=false does not change legality of the D7 Action. If the original Action can be submitted interactively, the D10 caller transitions to awaiting_confirmation; if the Action itself is invalid, return the original D7 error. D7 defines no looser parser, Field selector, permission, or semantic fallback for automation.

### 4.3 Add to replay/recovery paragraph

Add:

> D10 standing approval does not change D7 OperationId/replay rules. A fresh automation occurrence needs a fresh prepare. A lost receipt for the same committed request can only resend the original request. ApprovalUse/count/budget replay and recovery are owned by the D6 coordinated amendment; D7 cannot issue a semantically equivalent Action under a new OperationId merely because an Agent/automation restarted.
>
> Existing D7 preview delivery epoch, EffectBytes, ActionEvidence, FieldSelection, result epoch, and source-revision invalidation rules remain. Standing Approval cannot revive stale evidence/selector/preview. When a new prepare is required, it produces a new request/OperationId and requires a new approval use; neither an old click nor an old mechanical approval carries forward.

## 5. Explicit non-amendment for D8 and D9

D8 gains no unattended edit branch. Document/Annotation edit, dirty Draft, IME, current serial, complete preview, and explicit confirmation remain unchanged. Agent/Automation may only propose into D8 and cannot fabricate EditSession or human origin.

D9 does not change Provider/Route, worker sandbox, ExportPlan, LossReport, PublicationReceipt, or external publisher. D10 external transport/Connector grants no network to a D9 worker, and D9 publication confirmation grants no Standing Approval author write.

## 6. Activation and version compatibility

These amendments take effect only after independent review acceptance, controller coordinated decision, and activation together with the D10 candidate. Before activation:

- product may implement/read-only D10 Broker, Agent proposal, Automation scheduling, Tool/MCP, Connector read, and external-effect control;
- all D7/D8 author proposals still use current per-operation confirmation;
- the `single_field_member` unattended author-submit capability must report not_in_release or another truthful D1 unavailable state and cannot be hidden behind a private flag.

After activation, historical saved D3/D6 decisions keep their original decoders/bytes. Previously committed history is not backfilled with ApprovalUse. An old D7 prepare without a decision does not automatically gain standing approval; the user must fresh-prepare or use only an exact recovery route explicitly supported by the then-coordinated version.

## 7. Joint counterexamples required for independent review

1. Two same-value Field Entries must not select first under automatic approval.
2. Approval count=1 with two Runs planning concurrently allows at most one planned/reserved winner.
3. Process crash after approval reserve does not reserve again on restart.
4. Approval revoke/expiry after D6 planned does not auto-terminal-fail and does not substitute a new envelope.
5. Final commit racing approval revoke has one linearized result.
6. Author commit succeeds, receipt is lost, approval expires: replay returns original receipt without another count.
7. Committed raw no-op consumes once and replay does not recount.
8. Stale FieldSelection, A→B→A, and Registry generation change cannot be revived by standing approval.
9. A footprint mutant changing note/provenance/Facet/second member must reject automatic approval.
10. D8 dirty Draft combined with valid background author commit leaves the Draft intact.
11. D3 create/lifecycle Action cannot use the single_field_member envelope for unattended submission.
12. A D10 control error cannot wrap/leak original D6 not_visible or hidden facts.

These author-candidate counterexamples are not test results; actual evidence remains layered by the Implementation Impact document.
