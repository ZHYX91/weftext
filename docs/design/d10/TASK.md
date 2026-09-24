---
source_language: zh-CN
translation_of: TASK.zh-CN.md
translation_status: synced
---

[简体中文](TASK.zh-CN.md)

# D10: Agent, Automation, and External Capabilities

## Goal and current status

Status: candidate. Produce a complete, implementable, independently reviewable design defining what an Agent may read, context selection, proposed actions, approval, audit, and revocation, and how automation, scheduling, connectors, MCP, model/tool adapters, and conversion coordination share security boundaries without sharing a second author authority.

The current author candidate uses a narrow Broker, typed Capability Catalog, and specialized executors; Core remains the sole author transaction authority. The candidate also carries the required D6/D7 Standing Approval companion-amendment proposal, but those amendments do not take effect before independent acceptance and coordinated activation.

## Fixed inputs

Use the fixed Git commit supplied by the controller. Completely read the design inputs listed by ../inputs.json and maintain actual reading coverage. The current author session has completed 48/48 input reads; START records that fact and the real truncated-range rereads.

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
- this TASK pair
- the START pair

CANDIDATE is self-contained and fixes the final architecture, data types, operation contracts, state machines, errors/races, permission/approval/egress/secret rules, activation/upgrade/rollback, Agent/Automation/Connector/MCP, budget/cost/audit, and five-surface boundaries. It compares credible complete A/B/C/D alternatives and states the selection rationale and non-goals.

SCENARIO-DISPOSITIONS covers D10 routing from mandatory intake and the failure boundaries in this TASK item by item, including source location, accept/revise/reject/defer-with-owner, execution mode, candidate landing, and validation obligation.

UPSTREAM-AMENDMENTS gives exact proposed addition/replacement text for required D6 main text, D6 Control Interfaces, and D7 Execution/Action, plus unchanged wire/owner/version boundaries. It never describes an author proposal as already effective upstream.

## Author execution stage

The same author conversation first completes planning, then writes the complete candidate, performs repository validation, and verifies the actual committed artifacts on the same candidate branch and existing PR. Major design issues may return to author planning, but stage transitions do not create another candidate branch or duplicate PR.

The author modifies only necessary design material under docs/design/d10/ and the existing PR title/body. Do not modify input snapshots, product implementation, brand, repository permissions, or branch protection; do not merge, release, or start A2.

Author completion means: actual 48/48 input coverage; seven complete bilingual document pairs; required upstream amendment clearly unactivated; diff restricted to the authorized directory; actual document checks/CI read and accurately reported; no pending/failure mislabeled as pass.

## Independent review

After the complete candidate is fixed in a commit, hand it to a fresh independent Chat GPT-6 Pro for review from scratch. Author self-review is not an independent Gate.

Independent review at minimum requires a complete accept/pass judgment; P0/P1=0 before coordinated activation can be recommended; terminology pass; evaluation of a simpler complete alternative; examination of whether the D6/D7 amendment is necessary and sufficient; and closure of mandatory scenarios, dependencies, and evidence boundaries.

Finite simulations, models, or CI do not establish product support. Real Core, durable fault, OS sandbox, real protocol-provider, UI/device, and release evidence remain separated by the Implementation Impact document, with unfinished items explicitly pending.
