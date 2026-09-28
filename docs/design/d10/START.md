---
source_language: zh-CN
translation_of: START.zh-CN.md
translation_status: synced
---

[简体中文](START.zh-CN.md)

# D10 Reading and Candidate Checkpoint

Status: candidate revision `D10-r08-joint-review-fixes-2026-09-28`. The complete independent joint review of fixed R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` against fixed S completed candidate18/18 and S49/49 and returned REVISE (P0=0, P1=1, P2=10; overall, terminology, and bilingual semantics require revision). R08 is the author remediation and has not been independently reviewed, accepted, or coordinatedly activated. This file records input/candidate handoff state, not a Gate verdict.

The integrated fixed-input baseline is S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`; S contains the original 48 inputs from U=`f205831c848729f7ddbc3ba0cf32b689459c0c98` plus the added D4 reference catalog. Author branch: docs/d10-start. Existing Draft PR: #2, base=docs/chat-collaboration. D1-D9 snapshots remain authoritative; D10 UPSTREAM-AMENDMENTS is only an unactivated companion proposal.

## 1. Reading coverage

Reading evidence is separated by actor/lineage. The **original author lineage** historically completed U48/48 and then the supplemental D4 reference catalog, for historical S49/49. The **current continuation author** has personally completed S16/49 full reads. D9 workers/export and templates were dependency-scoped partial reads only and are not counted as full. The **independent joint review of fixed R07** separately completed S49/49. None of these records inherits another actor's read count. The code block below remains the original author-lineage 48-file U inventory:

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

The original author lineage's supplemental 49th input comes from S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` and was read in full: 102856 characters and 3685 content lines (the trailing newline yields one extra empty element in line splitting). The catalog contains 61 Fields, 7 Facets, 22 value-type aliases, 4 qualifier sets, and 1 Calendar series policy. That historical lineage checked it against the D7 Narrow Field Qualification positive `people/phone` construction and relation/cross-Field negatives. This is not re-counted as a continuation-author full read.

The following repository/design rules and task entrypoints were completely read in the original author lineage; this list is historical provenance, not continuation-author S coverage:

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

For the **original author lineage**, the recorded 48/48 and later S49/49 had no known unread ranges after those rereads. The current continuation author remains at S16/49 full reads by design; the remaining 33 S files are not silently claimed read. Search summaries, tool/workflow summaries, directory listings, and partial ranges never count as full reads.

## 3. From first checkpoint to complete candidate

The original author lineage's first batch read only 4/48 design inputs and created START to verify real GitHub writing. That lineage then completely read the remaining 44 files and, based on the full input set, compared Options A/B/C before converging on Option D: Narrow Delegation Broker + Typed Contribution Catalog + Specialized Executors.

A second design-convergence pass further corrected the initial plan:

- it no longer assumes "no upstream amendments"; unattended author submission requires an explicit coordinated D6/D7 amendment;
- first-generation standing approval is strictly limited to the existing D7 single-owner, single-Field, exactly-one-current-Entry, one-existing-scalar-member set_field_member case;
- D4 Registry and D10 Catalog remain separate but are activated together by Activation Binding; the D4 cumulative semantic ledger cannot roll back its pointer or delete history;
- `ToolValueProfile/1`, `ToolType/1`, and `ToolValue/1` are a D10-owned restricted closed algebra rather than D7 aliases, and MCP is only a Tool Adapter transport. The named Core field-member adapter separately consumes the original D7 TypedLiteral/ResolvedCodeScope to build the exact D7 author intent; arbitrary ToolValue can never become NodeRef, FieldId, selector, or author request;
- package trust fixes SHA-256 content binding plus application-level Ed25519 publisher signatures and keeps PublisherIdentity separate from NamespaceClaim;
- external effect, idempotency, outcome_unknown, credential rotation, cost reservation, cancellation races, and audit failure now each have one normative conclusion.

These are author-candidate choices and not independent acceptance.


## 4. Current candidate material

The complete candidate in this directory is **nine synchronized Chinese/English Markdown pairs (18 exact paths)**:

- CANDIDATE.zh-CN.md / CANDIDATE.md
- CONTROL-CONTRACT.zh-CN.md / CONTROL-CONTRACT.md
- UPSTREAM-AMENDMENTS.zh-CN.md / UPSTREAM-AMENDMENTS.md
- TERMINOLOGY.zh-CN.md / TERMINOLOGY.md
- SCENARIO-DISPOSITIONS.zh-CN.md / SCENARIO-DISPOSITIONS.md
- IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md / IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md
- REVIEW-DISPOSITIONS.zh-CN.md / REVIEW-DISPOSITIONS.md
- TASK.zh-CN.md / TASK.md
- START.zh-CN.md / START.md

No supplementary tenth record exists; REVIEW-DISPOSITIONS is one of the nine pairs.

The four fixed-S supplemental paths referenced by the author packet are `docs/design/README.md`, `docs/design/README.zh-CN.md`, `docs/design/inputs.json`, and `docs/design/snapshots/d4-reference-catalog-registry/source.json`. The first three are README/index material; only the catalog is the 49th normative source. R08 author work does not modify any of them.

CANDIDATE is the self-contained main specification; CONTROL-CONTRACT uniquely owns D10 closed management/current/error/rule, author-adapter, external-effect, history, and stop wires; TERMINOLOGY closes naming collisions; SCENARIO-DISPOSITIONS preserves and adjudicates mandatory/task/race scenarios; Implementation Impact layers future evidence; UPSTREAM-AMENDMENTS remains inactive companion text; REVIEW-DISPOSITIONS records independent findings and author landing; TASK defines process; START records handoff state.

## 5. Current design selection and still-unactivated parts

The author recommends Option D. Core remains the sole author-transaction authority and D10 Broker has no second write path; D4 Registry and D10 Capability Catalog remain separate; Agent/Automation/Connector/MCP all pass current permission, delegation, egress, secret, budget, audit, and their owning protocols.

CONTROL-CONTRACT remains the unique wire owner. In particular, D10 ToolValue types stay a restricted D10 algebra; the Core field-member adapter bridges explicitly to the original D7 TypedLiteral/ResolvedCodeScope and does not promote generic ToolValue into author identity or author request.

The D6/D7 Standing Approval amendment is not jointly accepted. Therefore, even with complete candidate documents, the product cannot mark unattended author submit available. Before coordinated activation, Automation may only prepare author proposals and follows existing D7/D8 per-operation confirmation.

The candidate also does not claim implementation of OS sandbox, real MCP/model/connector services, credential storage, cost system, real Core amendment, or five-surface UI. Those evidence items remain pending in Implementation Impact.

## 6. Independent-final-review status

Historical fixed C=`35fab950dabedfb92c9f12858701be8afe6faa74` and fixed R06 C=`32a0868ae9443a3f839cfb4f5e9bbcace308314d` remain historical REVISE records.

The complete independent joint review of fixed R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` against S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` completed candidate **18/18** and S **49/49** with no normative reading gap. Final result: **REVISE**, P0=0, P1=1 (`B03-P1-01`), P2=10 (`B01-P2-01`, `B02-P2-01`, `B02-P2-02`, `B02-P2-03`, `B03-P2-01`, `B03-P2-02`, `B03-P2-03`, `B03-P2-04`, `B10-P2-01`, `B11-P2-01`). Overall, terminology, and bilingual semantics all require revision. R08 is the author remediation and closes none of these findings by author assertion.

All companion D3/D6/D7/D8/D9 amendments remain inactive proposals. Author checks, documentation CI, and bounded models prove only their artifacts; no merge, release, activation, or A2/product implementation is authorized by this handoff.

## 7. Next completion and review gate

Before fixing the final R08 candidate, all **nine bilingual pairs / 18 exact paths** must agree on `D10-r08-joint-review-fixes-2026-09-28`; fixed S and every path outside docs/design/d10/ must remain unchanged; the English/Chinese scenario corpus must retain the same **125 IDs** with matching branch classifications; and the applicable repository documentation/input checks must pass. Reading and CI evidence must be reported at the actual actor/layer and no pending/failure may be called pass.

The fixed final R08 commit then requires a **fresh complete independent joint review from scratch: candidate18/18 + fixed S49/49**. R07's completed 18/18+49/49 review cannot be inherited as a differential pass because R08 changes public wire, authorization, transaction/safety, recovery, external-send, and D8/D9 interface-owner contracts. Any required upstream amendment still needs later coordinated acceptance, versioning, activation, and acceptance evidence.

Observed pre-R08-B checkpoint only: head `29ecca0cab43652a2e60f9997a68ec6ba8f6ca2b`; Design documents run `36383316785` failed solely on the 19 known mixed-prose findings in TERMINOLOGY.zh-CN.md and UPSTREAM-AMENDMENTS.zh-CN.md, so its input checks were skipped. At controller observation, source-gate run `36383316792` remained in progress and included the known rust-source documentation failure and existing ui-source dependency-audit failure. This is a dated execution record, not normative state or a pass claim.



Historical fixed C=`35fab950dabedfb92c9f12858701be8afe6faa74` completed 49/49 upstream reading and ended REVISE; it is historical evidence only.

The complete independent final review of fixed R06 C=`32a0868ae9443a3f839cfb4f5e9bbcace308314d` against S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` completed candidate **18/18** and upstream **49/49**, with no reading gap. Final verdict: **REVISE**, P0=0, P1=1 (`JR001`), P2=8 (`JR002`–`JR009`); terminology failed and bilingual translation failed. D8/D9 produced no additional findings; The two previously clarified questions remain closed, and owner/composition review is complete.

R07 is the author remediation of that complete issue set. Neither the old C35 review nor the R06 18/18+49/49 review counts as an independent read/pass of the new R07 commit. REVIEW-DISPOSITIONS marks every JR row only as author revised / pending independent re-review.

Author checks, documentation CI, and bounded models prove only their artifacts. All D6/D7/D3/D8/D9 companion amendments remain inactive proposals; do not merge, release, activate, or start A2.
