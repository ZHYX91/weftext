---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)
# A2 Final Design Integration Candidate

Status: candidate design, not independently accepted, not implemented, not released.

## 1. Fixed object and authorship

This A2 integration is based on parent commit 97f4734f82a760cb6716c8122b84494da2b61164. Fixed historical input S is 7e18168dad3e6d120fce0dd607dc10fa7894e252. The protected input inventory is docs/design/inputs.json at blob 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.

This directory is the only A2 candidate authority for modules integrated here. Earlier snapshots, D6 file-authority owner afterimages, and the AsciiDoc/Annotation final-design candidate remain source provenance and historical decoder evidence; they do not form a second current A2 definition.

This A2 candidate preserves D1–D9 without the complete D10 module. D7 fixed26be PASS remains D7-bounded; fixed5eca/ff10 D8, fixed8b6 D8/D9, fixed4da7 D9 canonical/renderer/root mapping, fixed79026 Annotation, fixed466b FC map and fixed6012 D9-BAF-P1-02/D9-466B-P2-01 remain independently CLOSED at their fixed scopes. ONLY new D9-6012-P1-01 (no required graph nodeDetails) and D9-6012-P2-01 (Host/Core/Region invalid_output retained, not worker-only) are author-resolved-pending-independent-review. Product UNRUN; no D9/global PASS.

## 2. Batch progress

| Module | A2 status in this batch |
| --- | --- |
| D1 | integrated current candidate; fixed-446 D1/D2 findings independently closed in that bounded scope |
| D2 | integrated current candidate; fixed-446 D1/D2 findings independently closed in that bounded scope |
| D3 | integrated current author candidate; a separate fixed-a62 non-author narrow review CLOSED the two bounded D3 P1 findings; this is not global A2 acceptance |
| D4 | integrated current author candidate; A2-D4D5:P2-02 remains bounded CLOSED at fixed4282 and A2-D4D5:P2-01 is bounded CLOSED after the fixed1fc4 independent review; no global acceptance |
| D5 | integrated current author candidate; A2-D4D5:P2-02 remains bounded CLOSED at fixed4282 and A2-D4D5:P2-01 is bounded CLOSED after the fixed1fc4 independent review; no global acceptance |
| D6 | fixed2f89 independently CLOSED the two navigation findings that remained after fixed454e; prior bounded D6 semantic closures remain limited to their recorded scope. |
| D7 | fixed26be non-author review = PASS (P0=0/P1=0/P2=0); the final A2-D7-2F89-P2-01 is CLOSED there; earlier bounded closures keep their original SHAs |
| D8 | preserved design candidate; fixed5eca/ff10 bounded closures remain, and fixed8b6 independently CLOSED final D8-C98B-P1-03. Acceptance JSON remains 213/213 unique IDs with the deterministic guard; the source audit remains 167 direct / 146 upstream / 447 retained owner and 114 authored applicability changes. This is not global A2 acceptance |
| D9 | complete author candidate; fresh Plan4/Catalog3/Selection2/Projection2/Loss2/Confirmation2/Receipt4/PrintReceipt1 with exact historical Plan/Receipt1–3 recovery, one complete Annotation carrier, finite six-chart complete_data View route and canonical cross-domain admission; 104 unique acceptance rows / 41 registry concepts. fixed8b6 independently CLOSED D9-BAF-P2-02. Two D9 P1 and two D9 P2 findings remain author-resolved-pending-independent-review; ROOT-8B6-MAP-P2-01 separately awaits mapping review |
| D10 | TODO as a full module; D7 consumed SearchContribution/Catalog and D7 approval/effects/custody intersections |
| Mandatory A2 source | D8 consumed the complete D8-applicable §15.1–15.8.9 chart/View interaction set and RTL intake; D9 consumes Mandatory §14 lines 884–924 plus D9-applicable §15.5–15.8 lines 1042–1141 and all ten View/export fixtures; D10 owner-module completion remains later |

No TODO module is treated as accepted, complete, or semantically read merely because its path, route, blob, or title is known. D8 and D9 are no longer TODO but both remain unaccepted author candidates pending independent review.

## 3. Current files

- D1.md is the self-contained current D1 candidate for this batch.
- D2.md is the self-contained current D2 candidate.
- D3.md, D3-IMPACT.md, D3-LEXICON.md, D3-SCHEMAS.md, and D3-TERMS.json form the current D3 author candidate.
- D3-SOURCE-MAP.json records D3 source-qualified cases and direct current coordination rows.
- D4.md, D4-IMPACT.md, and D4-LEXICON.md form the D4 author candidate; D4-SOURCE-MAP.json and D4-CATALOG-MAP.json retain provenance.
- D5.md, D5-IMPACT.md, and D5-LEXICON.md form the D5 author candidate; D5-SOURCE-MAP.json retains provenance.
- D6.md, D6-CONTROL.md, D6-SCHEMAS.md, D6-IMPACT.md, D6-LEXICON.md and D6-REGISTRY.json form the current D6 author candidate; D6-SOURCE-MAP.json preserves detailed provenance.
- D7.md, D7-SCHEMAS.md, D7-QUERY-V2.md, D7-SEARCH.md, D7-IMPACT.md, D7-REGISTRY.json and D7-REGISTRY-QUALIFICATION.md plus the byte-retained d7/owners subtree form the current D7 author candidate; D7-SEARCH-FIXTURES.json is the shortcut/parser oracle and D7-SOURCE-MAP.json is the machine trace.
- D8.md, D8-INTERFACES.md, D8-SCHEMAS.md, D8-DIRECTION.md, D8-ACCEPTANCE.md, D8-LEXICON.md, D8-IMPACT.md, D8-TERMS.json, D8-REGISTRY.json and D8-SOURCE-MAP.json form the retained D8 candidate. fixed5eca/ff10 closures remain, and fixed8b6 independently CLOSED the last D8-C98B-P1-03; the D9 repair does not reopen it.
- D9.md, D9-INTERFACES.md, D9-SCHEMAS.md, D9-ACCEPTANCE.md/JSON, D9-IMPACT.md, D9-LEXICON.md, D9-TERMS.json, D9-REGISTRY.json and D9-SOURCE-MAP.md/JSON form the complete D9 author candidate. The qualified native-table Office authoring revision keeps simple `data.native_table.COLUMN` and uses visible `native.table[...]::column[...]` for qualified/non-ASCII fresh authoring; internal `nt_/nc_` keys remain Plan metadata only.
- REVIEW-ENTRY.md is the review entry for the D1-D9 author candidate.
- SOURCE-MAP.md gives the human-readable source/disposition map.
- SOURCE-MAP.json records all 49 fixed-S inputs, exact S blobs, read status, current sources, and source-qualified obligation groups.

## 4. Precedence inside this candidate

For D1 through D9, the A2 files in this directory are the current candidate text. A named historical decoder or historical scenario remains normative only for historical recovery or source-qualified evidence when D1.md, D2.md, or SOURCE-MAP explicitly says so.

For D10 there is no complete A2 current definition yet. D9 reads only the named direct D10 intersections (specialized-executor boundary, worker no-network rule, Mobile negative surface, PublicationReceipt/Resource split and technical-interface naming); that PARTIAL read does not integrate the complete D10 module.

## 5. Acceptance boundary

A subsequent non-author review bound to fixed26be (`26be071d4c2075343d9ebf272e00f23769ce0d64`) concluded PASS with P0=0 / P1=0 / P2=0 and CLOSED the final `A2-D7-2F89-P2-01`. Earlier bounded closures remain bound to their original SHAs. D7 is therefore accepted in its D7 scope; this does not accept D8 or global A2.

Previously independently CLOSED: D8-C98B-P1-03/D9-BAF-P2-02 fixed8b6, D9-8B6-P2-01/02 and ROOT-8B6-MAP-P2-01 fixed4da7, D9-BAF-P1-01 fixed79026, D9-4DA7-MAP-P2-01 fixed466b, D9-BAF-P1-02/D9-466B-P2-01 fixed6012 and ROOT-D9-ZH-P2-01 ff10. Next non-author pins PR #5 actual final SHA and inspects only fixed6012→final: D9-6012-P1-01 graph original TerminalSchema/static query_json nodes/edges, optional ViewSpec.nodeDetails and missing actual column negative; D9-6012-P2-01 original Host/Core/S33 coverage invalid_output, D9 Region issuance, caller selector invalid_request and unchanged owner priority. Source anchors FC SCHEMAS §6.6.1, FC SPEC §§8.3/16a, D9 Interfaces §7, original D7 execution §1, D9 workers/export §3 and Import IR §2/§8. Full D10 and fresh Pro/global A2 remain later, product UNRUN.
