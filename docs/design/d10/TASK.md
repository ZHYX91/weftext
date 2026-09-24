---
source_language: zh-CN
translation_of: TASK.zh-CN.md
translation_status: synced
---

[简体中文](TASK.zh-CN.md)

# D10: Agent, automation and external capabilities

## Goal and current status

Status: candidate-preparation. Produce a complete implementable design for what an Agent may read, context selection, proposed actions, approval, auditing and revocation; explain how automation, scheduling, connectors and conversion coordination share permission boundaries. Compare credible complete alternatives without assuming one generic tool system solves every boundary.

## Required inputs

Use the Git commit supplied by the controller and read the [complete input index](../README.md) and `../inputs.json`. First read the full D1 main document and Impact, all D10 routing and cross-cutting terminology requirements in mandatory scenario intake, and the D9 main document. Then read D2–D8 main documents, relevant complete interfaces and all terminology/implementation obligations. Reading may be batched, but do not claim a complete candidate before coverage is complete. Maintain files/ranges read and an unread inventory.

## Constraints and questions

Core is the sole author-transaction authority. Reuse D3 identity/lifecycle, D6 authorization/transactions/recovery, D7 exact Actions and result consumption, D8 Draft/confirmation, and D9 worker/proposal/publication boundaries. External model output, transcripts, secrets, credentials and Provider provenance do not become author authority or gain a second write path. Installation, Registry definitions and author content remain orthogonal; disabling or failed upgrades cannot delete author facts.

Specify the local Desktop/CLI broker, reuse by hosted Server and WebUI calls; initial Mobile has no Agent, automation, connector or conversion execution. Separate read access, outbound disclosure, workspace mutation and external side-effect authorization. Tool output, webpages, documents and model text cannot expand authority. Cover prompt injection, minimal context, data minimization, secret storage/redaction, network/file/process isolation, package provenance/dependencies/upgrades and rollback.

Define principals, capabilities, grant lifetimes, revocation, exact-input binding, idempotency and unknown-outcome recovery, cancellation/restart, queues/scheduling/concurrency, audit/retention/export, resource budgets and cost ceilings. External effects cannot be described as atomic with a Core transaction. Scheduled execution must not turn past interactive consent into unlimited authority. Distinguish installation, capability availability, permission denial, temporary unavailability and non-disclosing diagnostics.

## Deliverables and acceptance

Produce one coherent candidate, concepts/lexicon, interfaces/state machines, explicit non-goals, complete alternative comparison, scenario dispositions, implementation impact and test outline. Enumerate necessary upstream amendments precisely; upstream remains unchanged until independent acceptance and coordinated activation. Separate actual tests, model checking, CI, device evidence and pending obligations; finite simulation does not establish product support.

At minimum cover malicious-document exfiltration, grant expiry/revocation, cached reads after permission changes, duplicate scheduling, unknown external effects after crashes, cancellation races, credential rotation, failed upgrades, audit failure and cross-surface consistency for identical requests. Independent review requires complete accept/pass, zero open P0/P1, terminology pass, assessment of better complete alternatives and dependency closure.

## Current execution boundary

The author uses ordinary Chat Latest at 极高; independent review uses Chat GPT-6 Pro. The first batch produces only a reading checkpoint, candidate structure and key open decisions, commits paired START documents on one branch and creates a draft PR to verify actual GitHub writes. Do not modify input snapshots, product code, permissions, branch protection or branding; do not merge or release. Continue the candidate under this same task packet afterward.
