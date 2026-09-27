---
source_language: zh-CN
translation_of: TASK.zh-CN.md
translation_status: synced
---

[简体中文](TASK.zh-CN.md)

# D10: Agent, Automation, and External Capabilities

## Goal and current status

Status: candidate revision R05. The author revision is formed and awaits a new complete independent joint final review of a fixed candidate. Produce a complete, implementable, independently reviewable design defining what an Agent may read, context selection, proposed actions, approval, audit, and revocation, and how automation, scheduling, connectors, MCP, model/tool adapters, and conversion coordination share security boundaries without sharing a second author authority.

The current author candidate uses a narrow Broker, typed Capability Catalog, and specialized executors; Core remains the sole author transaction authority. The candidate also carries the required D6/D7 Standing Approval companion-amendment proposal, but those amendments do not take effect before independent acceptance and coordinated activation.

## Fixed inputs

Use the fixed Git commit supplied by the controller. Completely read the design inputs listed by ../inputs.json and maintain actual reading coverage. The current author session completed the original 48/48 from U and additionally read the D4 reference catalog added by S, for current 49/49 coverage. START explicitly distinguishes the historical 48/48 from the newly added 49th input.

Upstream D1-D9 remain authoritative until any coordinated amendment is actually accepted. Old model, stage, authorization, and historical candidate labels inside snapshots are historical source text only and do not override this task packet.

## Constraints and questions

Core is the sole author-transaction authority. Reuse D3 identity/lifecycle, D6 authorization/transactions/recovery, D7 exact Actions/result/effects, D8 Draft/confirmation, and D9 worker/proposal/publication. External model output, transcripts, secrets, credentials, Provider provenance, Runs, and tool results never become author authority or gain a second write path.

Installation, D4 Registry, D10 Capability Catalog, runtime health, and author content stay separate. Disablement, uninstall, failed upgrade, credential rotation, and provider outage do not delete author facts. Registry/Catalog activation has explicit generations and complete history and cannot derive namespace ownership from install order or display name.

Desktop/CLI may host the same local Broker/control domain; Server hosts managed capabilities; WebUI can initiate/manage them only through Server. Initial Mobile has no Agent, automation, connector/conversion execution, credential management, or approval/delegation entrypoints for these capabilities.

Read, content egress, workspace mutation, external side effect, and secret use are independent authorization dimensions. Web pages, Documents, tool results, MCP descriptors/prompts/resources, and model text are untrusted input and cannot expand principal, delegation, tool allowlist, egress recipient, network/file/process, secret, budget, or approval.

Define principals, Delegation Lease, Standing Approval, grant lifetimes, revocation, exact-input binding, idempotency, unknown-outcome recovery, cancellation/restart, queues/scheduling/concurrency, audit/retention/export, resource budgets, and cost ceilings. A past interactive confirmation cannot become indefinite background authority.

External effects cannot be described as atomic with a Core transaction. Distinguish package installation, capability availability, permission denial, temporary unavailability, and non-disclosing diagnostics. MCP is only a Tool Adapter transport and is not the Weftext permission or identity system.

## Deliverables

The author stage maintains complete synchronized Chinese/English candidate material in docs/design/d10/:

- CANDIDATE.zh-CN.md / CANDIDATE.md
- TERMINOLOGY.zh-CN.md / TERMINOLOGY.md
- SCENARIO-DISPOSITIONS.zh-CN.md / SCENARIO-DISPOSITIONS.md
- IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md / IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md
- UPSTREAM-AMENDMENTS.zh-CN.md / UPSTREAM-AMENDMENTS.md
- CONTROL-CONTRACT.zh-CN.md / CONTROL-CONTRACT.md
- this TASK pair
- the START pair

CANDIDATE is self-contained and fixes the final architecture, data types, operation contracts, state machines, errors/races, permission/approval/egress/secret rules, activation/upgrade/rollback, Agent/Automation/Connector/MCP, budget/cost/audit, and five-surface boundaries. It compares credible complete A/B/C/D alternatives and states the selection rationale and non-goals.

SCENARIO-DISPOSITIONS covers D10 routing from mandatory intake and the failure boundaries in this TASK item by item, including source location, accept/revise/reject/defer-with-owner, execution mode, candidate landing, and validation obligation.

UPSTREAM-AMENDMENTS gives exact proposed addition/replacement text for required D6 main text, D6 Control Interfaces, and D7 Execution/Action, plus unchanged wire/owner/version boundaries. It never describes an author proposal as already effective upstream.

## Author execution stage

The same author conversation first completes planning, then writes the complete candidate, performs repository validation, and verifies the actual committed artifacts on the same candidate branch and existing PR. Major design issues may return to author planning, but stage transitions do not create another candidate branch or duplicate PR.

The author modifies only necessary design material under docs/design/d10/ and the existing PR title/body. Do not modify input snapshots, product implementation, brand, repository permissions, or branch protection; do not merge, release, or start A2.

Author completion means: current input coverage is accurately 49/49 while the old U 48/48 remains historical reading context; all nine bilingual files under docs/design/d10/ are synchronized; required upstream amendments remain explicitly inactive; the candidate includes the four fixed-S supplemental input files with every other snapshot byte-identical to S; actual document checks/CI are read and accurately reported; no pending/failure is mislabeled as pass.

## Current staged-revision constraints

The same author continues revising this candidate after staged independent review; author revision does not change the fact that the independent Gate is incomplete. The following contracts are now explicit:

- Delegation Lease `maxRuns` is consumed once at the atomic Run-admission point immediately before the first protected execution; cancellation before that point consumes nothing, while failure, cancellation, or crash after admission never refunds it, and recovery of the same Run never consumes twice.
- Claim, Run identity, and terminal proof for one Automation occurrence remain durably deduplicated; restart, rescan, disable→enable, and cache rebuild cannot start another Run.
- Lease expiry is determined by trusted current time rather than a cleanup task; unprovable time continuity fails closed and never assumes the Lease remains unexpired.
- When Standing Approval loses a race after formal D6 entry, UPSTREAM-AMENDMENTS proposes D6 `approval_unavailable/preflight`; it is not disguised as a D10 pre-submit approval error and is never persisted as a permanent semantic rejection.
- After a planned decision's original preview transport expires, UPSTREAM-AMENDMENTS provides a read-only planned-preview recovery entrypoint that reissues a finite delivery epoch from saved semantics; it does not reprepare, change OperationId, rerun Query, or change target.
- Authoritative `terminal_failed` may release an approval-count reservation in the same abort transaction, while cancellation, TTL, temporary revocation, and Lease expiry do not; cost reservation follows its separate settled/released/uncertain contract.
- Both raw no-op and real member change belong to the first single_field_member automatic profile, but raw no-op keeps MutationFootprint, field_change, and sourceVersions empty rather than inventing effects.
- Cost `released` means billable execution is proven never to have started; an actually sent request with a final zero bill is `settled(0)`.

These remain candidate amendments and do not change current upstream authority before coordinated D6/D7 acceptance.

## Independent review

After the complete candidate is fixed in a commit, hand it to a fresh independent Chat GPT-6 Pro for review from scratch. Author self-review is not an independent Gate.

Independent final review still targets old fixed C=`35fab950dabedfb92c9f12858701be8afe6faa74`; formal coverage is currently 16/16 candidate files and 28/49 upstream inputs, with 1 P1 + 5 P2 formally open and the D3 batch awaiting its formal report. None of that coverage or verdict transfers to R05 as read, accepted, or closed. Once R05 is fixed it requires a fresh complete joint final review, still requiring a complete verdict, P0/P1=0, and closure of terminology, scenarios, dependencies, evidence, and required amendments.

Finite simulations, models, or CI do not establish product support. Real Core, durable fault, OS sandbox, real protocol-provider, UI/device, and release evidence remain separated by the Implementation Impact document, with unfinished items explicitly pending.
