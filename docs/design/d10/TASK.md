---
source_language: zh-CN
translation_of: TASK.zh-CN.md
translation_status: synced
---

[简体中文](TASK.zh-CN.md)

# D10: Agent, Automation, and External Capabilities

## Goal and current status

revision: D10-FA-r01-2026-10-02; status: coordinated author candidate, not accepted, activated, or implemented. The last complete historical R08 review of C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` against S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` returned REVISE (P0=0, P1=3, P2=8). All eleven historical final dispositions remain OPEN. Named repairs have limited independent reviews; actual cross-owner integration and fresh global acceptance remain incomplete. REVIEW-DISPOSITIONS separates those evidence scopes.

The current author candidate uses a narrow Broker, typed Capability Catalog, and specialized executors; Core remains the sole author transaction authority. The candidate also carries the required D6/D7 Standing Approval companion-amendment proposal, but those amendments do not take effect before independent acceptance and coordinated activation.

## Fixed inputs

Use the fixed Git commit supplied by the controller and report reading evidence by actor/lineage rather than merging it. The original author lineage historically completed S49/49. The historical R08 continuation author personally completed S16/49 full reads; D9 workers/export and templates were dependency-scoped partial reads only and are not counted as full. The independent joint review of fixed R07 separately completed S49/49. None of these coverage records substitutes for another, and workflow/tool summaries do not count as full reads. START records the exact provenance.

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
- REVIEW-DISPOSITIONS.zh-CN.md / REVIEW-DISPOSITIONS.md
- TASK.zh-CN.md / TASK.md
- START.zh-CN.md / START.md

CANDIDATE is self-contained and fixes the final architecture, data types, operation contracts, state machines, errors/races, permission/approval/egress/secret rules, activation/upgrade/rollback, Agent/Automation/Connector/MCP, budget/cost/audit, and five-surface boundaries. It compares credible complete A/B/C/D alternatives and states the selection rationale and non-goals.

SCENARIO-DISPOSITIONS covers D10 routing from mandatory intake and the failure boundaries in this TASK item by item, including source location, accept/revise/reject/defer-with-owner, execution mode, candidate landing, and validation obligation.

UPSTREAM-AMENDMENTS gives exact proposed addition/replacement text for required D6 main text, D6 Control Interfaces, and D7 Execution/Action, plus unchanged wire/owner/version boundaries. It never describes an author proposal as already effective upstream.

## Author execution stage

Current design work coordinates this directory with the explicit owner afterimages and unique routing in `../d6-file-authority-reopen/`. Fixed S snapshots remain byte-unchanged. Each document has one assigned writer; author and independent reviewer roles remain separate. No product implementation, merge, release, or activation follows from this candidate. A2 integration and a fresh ordinary Chat Pro global review are required before design freeze.

Completion requires all nine bilingual D10 pairs / 18 exact paths plus every actual required D1–D9 owner afterimage to agree at one immutable candidate; fixed S49 and its inventory remain unchanged; all 125 existing scenario IDs and their full obligations remain traceable, and any new cases are explicit. Applicable documentation/input checks, actual reading coverage, terminology, bilingual semantics, and every historical/current finding are assessed on that exact candidate. Old S49/49 coverage, named differential PASS results, and CI cannot be inherited as full acceptance. Zero open P0/P1, explicit disposition of remaining P2, and fresh independent global Pro acceptance are required for design freeze, which still is not implementation or release.

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

R08 additionally freezes the following author-stage boundaries already owned by CONTROL/CANDIDATE/UPSTREAM/SCENARIO/IMPLEMENTATION:

- unattended authoring uses only the named first-party Core field-member adapter with closed `FieldMemberTask/1`; task intent is distinct from Standing Approval, and arbitrary ToolValue never becomes NodeRef/FieldId/selector/author request;
- control history keeps full canonical B internally for dedup while public history returns only the seven-kind summary and never A/B/M;
- external consent binds stable effect Ref + requestDigest; lifecycle revision is separate from immutable request semantics; public current view does not disclose payload/key/secret/reservation identity;
- emergency stop is a specialized same-store D10 safety transaction with exact-target dedup, pre-reserved capacity, receipt/result replay, and no ordinary prepare or second author ledger;
- D8 13 kinds and D9 17 public kinds each have one technical-interface owner; consumes/returns/operates-on data retain their original domain owners, including D6 ImportJob.

## Independent review

After the complete candidate is fixed in a commit, hand it to a fresh independent Chat GPT-6 Pro for review from scratch. Author self-review is not an independent Gate.

Historical C=`35fab950dabedfb92c9f12858701be8afe6faa74` and fixed R06 C=`32a0868ae9443a3f839cfb4f5e9bbcace308314d` remain historical REVISE records. The complete independent joint review of fixed R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` completed candidate18/18 and S49/49 and returned REVISE with P0=0, P1=1 (`B03-P1-01`) and the ten P2 findings recorded in REVIEW-DISPOSITIONS; overall, terminology, and bilingual semantics all require revision. That complete R07 review is evidence for author remediation only, not R08 acceptance.

Completion requires all nine bilingual D10 pairs / 18 exact paths plus every actual required D1–D9 owner afterimage to agree at one immutable candidate; fixed S49 and its inventory remain unchanged; all 125 existing scenario IDs and their full obligations remain traceable, and any new cases are explicit. Applicable documentation/input checks, actual reading coverage, terminology, bilingual semantics, and every historical/current finding are assessed on that exact candidate. Old S49/49 coverage, named differential PASS results, and CI cannot be inherited as full acceptance. Zero open P0/P1, explicit disposition of remaining P2, and fresh independent global Pro acceptance are required for design freeze, which still is not implementation or release.

Finite simulations, models, or CI do not establish product support. Real Core, durable fault, OS sandbox, real protocol-provider, UI/device, and release evidence remain separated by the Implementation Impact document, with unfinished items explicitly pending.
