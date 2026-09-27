---
source_language: zh-CN
translation_of: START.zh-CN.md
translation_status: synced
---

[简体中文](START.zh-CN.md)

# D10 Reading and Candidate Checkpoint

Status: candidate revision r02, under staged independent re-review and not independently accepted or coordinatedly activated. This file records actual author-session input coverage, reading integrity, and candidate handoff state; it is not a Gate verdict.

Fixed upstream input commit: f205831c848729f7ddbc3ba0cf32b689459c0c98. Author branch: docs/d10-start. Existing Draft PR: #2, base=docs/chat-collaboration. D1-D9 snapshots remain authoritative; D10 UPSTREAM-AMENDMENTS is only an unactivated companion proposal.

## 1. Reading coverage

Design-input reading is complete at 48/48 with 0 unread. The complete set matches docs/design/inputs.json at the fixed input:

```text
docs/design/snapshots/d1-product-surface-and-capability-boundary/source.md
docs/design/snapshots/d1-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d4-d10-a2-mandatory-scenario-inputs-2026-08-28/source.md
docs/design/snapshots/d9-conversion-templates-and-workers/source.md
docs/design/snapshots/d2-document-content-and-domain-objects/source.md
docs/design/snapshots/d3-identity-references-ownership-and-lifecycle/source.md
docs/design/snapshots/d3-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d4-attribute-types-schema-and-relations/source.md
docs/design/snapshots/d4-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d5-tables-and-node-collections/source.md
docs/design/snapshots/d5-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d6-control-interfaces/source.md
docs/design/snapshots/d6-storage-transactions-permissions-and-sync/source.md
docs/design/snapshots/d6-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d6-terminology-registry/source.json
docs/design/snapshots/d7-definition-transfer/source.md
docs/design/snapshots/d7-execution-and-action-interfaces/source.md
docs/design/snapshots/d7-narrow-field-qualification/source.md
docs/design/snapshots/d7-prepared-action-binding/source.md
docs/design/snapshots/d7-preview-and-effects-transport/source.md
docs/design/snapshots/d7-query-algebra/source.md
docs/design/snapshots/d7-query-view-action/source.md
docs/design/snapshots/d7-scenario-dispositions/source.md
docs/design/snapshots/d7-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d7-terminology-registry/source.json
docs/design/snapshots/d7-value-and-cel-profile/source.md
docs/design/snapshots/d7-view-contract/source.md
docs/design/snapshots/d8-acceptance-matrix/source.json
docs/design/snapshots/d8-acceptance-matrix/source.md
docs/design/snapshots/d8-direction-and-accessibility/source.md
docs/design/snapshots/d8-editor-and-cross-surface-interaction/source.md
docs/design/snapshots/d8-editor-interfaces/source.md
docs/design/snapshots/d8-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d9-acceptance-matrix/source.md
docs/design/snapshots/d9-coordinated-d3-d7-binding-amendment/source.md
docs/design/snapshots/d9-import-ir-and-mapping/source.md
docs/design/snapshots/d9-templates/source.md
docs/design/snapshots/d9-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d9-workers-and-export/source.md
docs/design/snapshots/d2-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d3-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d4-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d5-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d6-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d7-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d8-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d9-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d8-rtl-and-bidirectional-interaction-mandatory-review-intake-2026-09-01/source.md
```

The repository/design rules and task entrypoints at the fixed input were also read completely:

- AGENTS.zh-CN.md
- docs/DOCUMENTATION.zh-CN.md
- docs/design/AGENTS.zh-CN.md
- docs/design/README.zh-CN.md
- docs/design/inputs.json
- docs/design/d10/TASK.zh-CN.md
- scripts/check_docs.py

Before forming the candidate, the branch START was reread to restore exact progress. Search summaries, directory listings, and partial matches were never counted as complete file reads.

## 2. Truncation and reread record

Some retrieval calls returned truncated output. A truncated call was never counted as a completed read; missing ranges were reread in smaller standalone segments. Explicit rereads included:

- middle and tail ranges of the D3 identity main document;
- D7 Query Algebra;
- D7 files after an aggregated multi-file read, then reread individually;
- D8 Editor Interfaces;
- D8 Acceptance Matrix Markdown and JSON;
- Mandatory Scenario Intake, where a large aggregate range was replaced by smaller continuous segments through end of file.

There are no known remaining unread ranges. "48/48" means the author actually completed input reading; it does not mean the author or independent reviewer has proven all upstream implementation.

## 3. From first checkpoint to complete candidate

The first batch read only 4/48 design inputs and created START to verify real GitHub writing. The same author conversation then completely read the remaining 44 files and, based on the full input set, compared Options A/B/C before converging on Option D: Narrow Delegation Broker + Typed Contribution Catalog + Specialized Executors.

A second design-convergence pass further corrected the initial plan:

- it no longer assumes "no upstream amendments"; unattended author submission requires an explicit coordinated D6/D7 amendment;
- first-generation standing approval is strictly limited to the existing D7 single-owner, single-Field, exactly-one-current-Entry, one-existing-scalar-member set_field_member case;
- D4 Registry and D10 Catalog remain separate but are activated together by Activation Binding; the D4 cumulative semantic ledger cannot roll back its pointer or delete history;
- Tool Value reuses a finite subset of the D7 type/value algebra, and MCP is only a Tool Adapter transport;
- package trust fixes SHA-256 content binding plus application-level Ed25519 publisher signatures and keeps PublisherIdentity separate from NamespaceClaim;
- external effect, idempotency, outcome_unknown, credential rotation, cost reservation, cancellation races, and audit failure now each have one normative conclusion.

These are author-candidate choices and not independent acceptance.

## 4. Current candidate material

The complete candidate in this directory is seven synchronized Chinese/English Markdown pairs:

- CANDIDATE.zh-CN.md / CANDIDATE.md
- TERMINOLOGY.zh-CN.md / TERMINOLOGY.md
- SCENARIO-DISPOSITIONS.zh-CN.md / SCENARIO-DISPOSITIONS.md
- IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md / IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md
- UPSTREAM-AMENDMENTS.zh-CN.md / UPSTREAM-AMENDMENTS.md
- TASK.zh-CN.md / TASK.md
- START.zh-CN.md / START.md

A supplementary re-review record is also maintained:

- REVIEW-DISPOSITIONS.zh-CN.md / REVIEW-DISPOSITIONS.md

CANDIDATE is the self-contained main specification; TERMINOLOGY closes naming collisions; SCENARIO-DISPOSITIONS adjudicates mandatory/task/race scenarios item by item; Implementation Impact layers future evidence; UPSTREAM-AMENDMENTS provides exact D6/D7 companion text; TASK defines the current author-to-independent-review process; START records actual input coverage.

## 5. Current design selection and still-unactivated parts

The author recommends Option D. Core remains the sole author-transaction authority and D10 Broker has no second write path; D4 Registry and D10 Capability Catalog remain separate; Agent/Automation/Connector/MCP all pass current permission, delegation, egress, secret, budget, audit, and their owning protocols.

The D6/D7 Standing Approval amendment is not jointly accepted. Therefore, even with complete candidate documents, the product cannot mark unattended author submit available. Before coordinated activation, Automation may only prepare author proposals and follows existing D7/D8 per-operation confirmation.

The candidate also does not claim implementation of OS sandbox, real MCP/model/connector services, credential storage, cost system, real Core amendment, or five-surface UI. Those evidence items remain pending in Implementation Impact.

## 6. Staged independent review and this author revision

The first two independent-review batches completely read all 14 D10 files in reviewed candidate C=`1c2bcd3e2926966c628292072c7a54a2bd5df40e`, plus eight fixed-upstream U=`f205831c848729f7ddbc3ba0cf32b689459c0c98` documents: D1 main/impact, D6 main/Control, and D7 Execution/Narrow Field/Prepared Binding/Preview Effects. The other 40 upstream inputs still await independent review.

The author's prior 48/48 input reading remains author-source coverage only and cannot substitute for independent upstream coverage. This revision responds to B1-01 through B1-05 and B2-01 from the first two batches. REVIEW-DISPOSITIONS records each author response. Every item remains "author revised, pending independent re-review" and must not be described as independently closed.

This revision focuses on:

1. one proposed D6 `approval_unavailable/preflight` result for an approval race after D6 entry, separated from D10 pre-submit approval errors;
2. read-only recovery delivery of the original preview for a planned decision, without extending old tokens or re-preparing;
3. atomic release of approval-count reservation only with authoritative terminal_failed, never ordinary cancellation/TTL/temporary loss of authority;
4. a raw-no-op branch with empty MutationFootprint/field_change/sourceVersions;
5. cost released only when billable execution never began, while an actually sent zero bill is `settled(0)`;
6. DelegationLease `maxRuns` consumed at first protected execution, never refunded after later failure/cancellation, never double-consumed on same-Run recovery, with terminal-occurrence and trusted-time deduplication/expiry semantics.

The next step remains continued independent review and targeted re-review, not an author-declared Gate pass. Proposed upstream amendments remain inactive until accepted, and A2 must not start.

