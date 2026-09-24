---
source_language: zh-CN
translation_of: START.zh-CN.md
translation_status: synced
---

[简体中文](START.zh-CN.md)

# D10 Start Reading Checkpoint

Status: candidate-preparation. This file records only the first D10 reading batch, boundaries, and structures to compare. It is not the complete D10 candidate and does not establish design acceptance, implementation completion, or release permission.

Fixed input commit: `f205831c848729f7ddbc3ba0cf32b689459c0c98`. Author branch: `docs/d10-start`. The later Draft PR targets `docs/chat-collaboration`. Old model, stage, and authorization text inside snapshots remains historical input only; the current D10 task packet governs this execution.

## Completed reading

The following 11 files were read directly from the fixed input commit through end of file; search summaries were not used as substitutes for the source text:

- `AGENTS.zh-CN.md`
- `docs/DOCUMENTATION.zh-CN.md`
- `docs/design/AGENTS.zh-CN.md`
- `docs/design/README.zh-CN.md`
- `docs/design/inputs.json`
- `docs/design/d10/TASK.zh-CN.md`
- `scripts/check_docs.py`
- `docs/design/snapshots/d1-product-surface-and-capability-boundary/source.md`
- `docs/design/snapshots/d1-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d4-d10-a2-mandatory-scenario-inputs-2026-08-28/source.md`
- `docs/design/snapshots/d9-conversion-templates-and-workers/source.md`

The last four items above belong to the 48-file design input inventory. This batch therefore fully read 4/48 design inputs. The mandatory scenario intake was read in segments through its end. One larger segment response was truncated by the tool; that range was then completely reread in smaller segments, and the truncated response was not counted as completed evidence.

## Design inputs not yet read

The following 44 inputs from `docs/design/inputs.json` have not yet been completely read. They must not be treated as covered before the complete D10 candidate is formed:

- `docs/design/snapshots/d2-document-content-and-domain-objects/source.md`
- `docs/design/snapshots/d3-identity-references-ownership-and-lifecycle/source.md`
- `docs/design/snapshots/d3-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d4-attribute-types-schema-and-relations/source.md`
- `docs/design/snapshots/d4-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d5-tables-and-node-collections/source.md`
- `docs/design/snapshots/d5-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d6-control-interfaces/source.md`
- `docs/design/snapshots/d6-storage-transactions-permissions-and-sync/source.md`
- `docs/design/snapshots/d6-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d6-terminology-registry/source.json`
- `docs/design/snapshots/d7-definition-transfer/source.md`
- `docs/design/snapshots/d7-execution-and-action-interfaces/source.md`
- `docs/design/snapshots/d7-narrow-field-qualification/source.md`
- `docs/design/snapshots/d7-prepared-action-binding/source.md`
- `docs/design/snapshots/d7-preview-and-effects-transport/source.md`
- `docs/design/snapshots/d7-query-algebra/source.md`
- `docs/design/snapshots/d7-query-view-action/source.md`
- `docs/design/snapshots/d7-scenario-dispositions/source.md`
- `docs/design/snapshots/d7-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d7-terminology-registry/source.json`
- `docs/design/snapshots/d7-value-and-cel-profile/source.md`
- `docs/design/snapshots/d7-view-contract/source.md`
- `docs/design/snapshots/d8-acceptance-matrix/source.json`
- `docs/design/snapshots/d8-acceptance-matrix/source.md`
- `docs/design/snapshots/d8-direction-and-accessibility/source.md`
- `docs/design/snapshots/d8-editor-and-cross-surface-interaction/source.md`
- `docs/design/snapshots/d8-editor-interfaces/source.md`
- `docs/design/snapshots/d8-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d9-acceptance-matrix/source.md`
- `docs/design/snapshots/d9-coordinated-d3-d7-binding-amendment/source.md`
- `docs/design/snapshots/d9-import-ir-and-mapping/source.md`
- `docs/design/snapshots/d9-templates/source.md`
- `docs/design/snapshots/d9-terminology-and-naming-lexicon/source.md`
- `docs/design/snapshots/d9-workers-and-export/source.md`
- `docs/design/snapshots/d2-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d3-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d4-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d5-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d6-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d7-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d8-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d9-implementation-impact-and-test-outline/source.md`
- `docs/design/snapshots/d8-rtl-and-bidirectional-interaction-mandatory-review-intake-2026-09-01/source.md`

## D10 scope

D10 defines how Agents, automation, and external capabilities operate without creating a second write authority. The scope includes what an Agent may read and how context is selected, proposed actions, approval, revocation, audit, and recovery; automation scheduling, queues, duplicates, and cancellation; connector protocols, credentials, and synchronization control state; Extension Package, manifest, contribution, runtime, namespace registration, and provider lifecycle; and the authorization, isolation, budgets, auditing, and external-side-effect boundaries shared with but not conflated with the D9 conversion coordinator.

Desktop and CLI may host a local broker/control plane, Server may host managed capabilities, and WebUI may only initiate or manage them through Server. Mobile initially runs no Agent, automation, connector, or conversion execution and does not manage credentials for these capabilities; it only consumes ordinary results after valid commits.

This batch does not define product implementation, modify D1–D9 snapshots, or freeze any candidate package, provider, manifest, or runtime syntax into a compatibility contract.

## Hard constraints already identified

- Core remains the sole author-transaction authority. Agents, automation, connectors, models, web content, tool results, transcripts, provider objects, and runtime caches cannot become author facts or a second workspace write path.
- Local and hosted modes must preserve D1 single-commit-holder and capability-discovery boundaries; UI or callers cannot redefine the same capability differently across product endpoints.
- D3 identity, owner, locator, provenance, origin, and lifecycle boundaries must be reused; external provider IDs, cursors, etags, UIDs, or names cannot replace Weftext identity.
- D6 authorization, budgets, transactions, recovery, and non-disclosure gate final workspace modification. Old capability discovery, old interactive consent, or a background timer is not a durable authorization ticket.
- D7 Action, preparation, exact input binding, preview/effects, result consumption, and unknown recovery must be reused; D10 does not establish another generic commit protocol.
- D8 Draft, confirmation, and cross-endpoint interaction boundaries must be reused; automated or model-generated proposals cannot be represented as native user input.
- D9 worker, isolated input, proposal, publication, and external-output boundaries remain authoritative. D10 may reuse their isolation and side-effect lessons but cannot turn conversion workers into general authors.
- User reads, content egress, workspace modification, and external side effects are separate authorization dimensions; permission in one dimension does not implicitly grant another.
- Web pages, documents, tool results, and model text are all untrusted input and cannot expand principal, scope, network, file, process, secret, or write permissions; prompt injection must be rejected at capability and data boundaries.
- Context selection follows least-necessary access and data minimization; secrets use handles or controlled references and do not enter workspace content, model context, ordinary logs, or exportable transcripts.
- Scheduled and background execution needs an explicit principal, capability scope, authorization lifetime, revocation, and revalidation. One interactive approval cannot silently become indefinite background authorization.
- External side effects and Core transactions cannot be presented as atomic. The design must cover idempotency keys, unknown outcomes, retry, compensation or human recovery, cancellation races, and crash restart.
- Installation, registry definitions, provider availability, and author content are orthogonal. Disablement, uninstall, version incompatibility, or failed upgrade cannot delete, default, or rewrite author facts.
- Extension, pack, connector, adapter, provider, module, contribution, and runtime terminology must pass the cross-cutting terminology gate; official, publisher, and user namespaces need anti-spoofing and collision rules.
- Runtime boundaries must cover network, file, process and resource isolation, source and dependency verification, update and rollback, resource budgets and spending caps, audit failure, and cached reads after permission changes.
- Capability unavailability, insufficient permission, missing components, lack of configuration, offline state, version incompatibility, and temporary failure must remain diagnosable without disclosing masked deployment details.

## Suggested candidate document structure

The complete candidate can be organized by the following logic; actual file splitting should wait until the complete reading pass:

1. Scope, non-goals, upstream contracts, and global invariants.
2. Terminology, package/contribution taxonomy, namespace, and registry.
3. Principals, delegation, capability, authorization lifetime, revocation, and exact input binding.
4. Agent context selection, tool invocation, proposals, approvals, and result consumption.
5. Automation scheduling, queueing, concurrency, duplication, cancellation, restart, and authorization revalidation.
6. Connectors, providers, secrets, synchronization control state, and external side effects.
7. Runtime isolation, network/file/process boundaries, dependency provenance, update, and rollback.
8. Desktop/CLI local control plane, Server hosted control plane, WebUI management entrypoints, and the Mobile unavailability matrix.
9. State machines, idempotency, unknown recovery, audit, retention, export, budgets, and spending.
10. Composition with D9 worker/conversion publication, D7 Action, and D8 confirmation.
11. Mandatory scenario dispositions, hostile-input and fault-injection cases, and cross-endpoint consistency.
12. Upstream amendment list, implementation impact, test outline, evidence still required, and acceptance conditions.

## Key open decisions and competing top-level designs

No option is frozen at this checkpoint. At least the following three complete top-level designs remain in competition:

- Option A: a shared broker/control plane plus specialized Agent runtime, automation scheduler, and connector adapters. The common layer owns only principals, authorization, capability, secret references, audit, budgets, job lifecycle, and external-side-effect recovery; domain runtimes keep closed protocols. This must prove that the common layer is sufficiently unified without becoming an open universal tool system.
- Option B: a unified capability host driven by typed contributions for Agents, automation, connectors, and selected external execution. The manifest declares inputs, outputs, permissions, sandbox, network, and side-effect classes, while the host centralizes scheduling and audit. This must prove that closed schemas, least privilege, upgrade compatibility, and provider isolation cannot be undermined by an open contribution blob.
- Option C: separate Agent, automation, and connector control subsystems that share only D6 authorization, D7 Action/result boundaries, unified audit identity, and a small capability vocabulary. This must prove that duplicated state machines and diagnostics do not drift, while avoiding an oversized common runtime created only for unification.

Open questions that require explicit comparison include the principal/delegation relation between interactive Agent sessions and background automation; boundaries among one-shot approval, session approval, policy approval, and scheduled authorization; whether context reading and content egress need distinct grants; recovery after an external side effect has an unknown result; how much connector synchronization should share with one-shot Actions; whether package-install trust and runtime authorization are separate layers; how registry definitions bind to installed provider versions; which operations must stop when audit fails; whether local and Server deployments share one wire contract or only semantic objects; and which component owns cross-capability budget, spending, and concurrency arbitration.

## Next complete-reading order

The next batch remains read-only and continues the explicit read/unread ledger:

1. D2 main text and D2 implementation impact, to fix Document, author source, domain-object, and Task/Template boundaries first.
2. D3 main text, D3 terminology, and D3 implementation impact, to fix identity, ownership, provenance, origin, and lifecycle vocabulary.
3. D4 main text, D4 terminology, and D4 implementation impact, to fix typed values, schema/field/relation, namespace, and registry-related upstream semantics.
4. D5 main text, D5 terminology, and D5 implementation impact, to fix Record, collection, occurrence, and scale boundaries.
5. Both D6 main interfaces, terminology, registry, and implementation impact, focusing on principals, permissions, transactions, audit, budgets, recovery, control state, and cache non-disclosure.
6. All D7 Action/Query/View/result-consumption interfaces, terminology, registry, and implementation impact, focusing on preparation, confirmation, unknown outcomes, effects, and the reusable boundary for capability contributions.
7. D8 main text, editor interfaces, direction/accessibility, acceptance matrix, terminology, implementation impact, and RTL intake, to fix how Agent/automation proposals enter Draft, confirmation, accessible interaction, and five-endpoint consistency.
8. Finally complete the remaining D9 acceptance, binding amendment, IR/mapping, templates, workers/export, terminology, and implementation impact before writing the full D10 candidate.

Until the complete input set is covered, work may extend candidate structure, open questions, and counterexamples only. It does not claim that the design gate has passed, freeze a top-level choice, or authorize changes to any upstream snapshot.
