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

The current author candidate preserves the D1-D8 integration. D7 has a subsequent fixed26be non-author PASS (P0=0/P1=0/P2=0). D8 is a preserved author candidate whose bilingual documents are repaired in this narrow batch, but its original FC classification, historical-to-current dispositions, per-obligation semantic audit, and source-map navigation gaps remain OPEN pending a fixed-SHA non-author review. D9-D10 remain future full-module batches.

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
| D8 | preserved author candidate; this narrow batch synchronizes the bilingual main/interfaces/schemas/direction/lexicon/impact/acceptance text. The existing source inventory is retained, but FC/history disposition, per-obligation semantic review and the broad source-map targets remain OPEN; status pending independent review |
| D9 | TODO as a full module; D7 consumed only coordinated binding and direct construction/import/export intersections |
| D10 | TODO as a full module; D7 consumed SearchContribution/Catalog and D7 approval/effects/custody intersections |
| Mandatory A2 source | D8 consumed the complete D8-applicable §15.1–15.8.9 chart/View interaction set and the RTL mandatory intake; D9-D10 owner-module completion remains later |

No TODO module is treated as accepted, complete, or semantically read merely because its path, route, blob, or title is known. D8 is no longer TODO but remains an unaccepted author candidate.

## 3. Current files

- D1.md is the self-contained current D1 candidate for this batch.
- D2.md is the self-contained current D2 candidate.
- D3.md, D3-IMPACT.md, D3-LEXICON.md, D3-SCHEMAS.md, and D3-TERMS.json form the current D3 author candidate.
- D3-SOURCE-MAP.json records D3 source-qualified cases and direct current coordination rows.
- D4.md, D4-IMPACT.md, and D4-LEXICON.md form the D4 author candidate; D4-SOURCE-MAP.json and D4-CATALOG-MAP.json retain provenance.
- D5.md, D5-IMPACT.md, and D5-LEXICON.md form the D5 author candidate; D5-SOURCE-MAP.json retains provenance.
- D6.md, D6-CONTROL.md, D6-SCHEMAS.md, D6-IMPACT.md, D6-LEXICON.md and D6-REGISTRY.json form the current D6 author candidate; D6-SOURCE-MAP.json preserves detailed provenance.
- D7.md, D7-SCHEMAS.md, D7-QUERY-V2.md, D7-SEARCH.md, D7-IMPACT.md, D7-REGISTRY.json and D7-REGISTRY-QUALIFICATION.md plus the byte-retained d7/owners subtree form the current D7 author candidate; D7-SEARCH-FIXTURES.json is the shortcut/parser oracle and D7-SOURCE-MAP.json is the machine trace.
- D8.md, D8-INTERFACES.md, D8-SCHEMAS.md, D8-DIRECTION.md, D8-ACCEPTANCE.md, D8-LEXICON.md, D8-IMPACT.md, D8-TERMS.json, D8-REGISTRY.json and D8-SOURCE-MAP.json form the preserved D8 author candidate. Bilingual synchronization is repaired here; source-map navigation and semantic-audit gaps remain open.
- REVIEW-ENTRY.md is the review entry for the D1-D8 author candidate.
- SOURCE-MAP.md gives the human-readable source/disposition map.
- SOURCE-MAP.json records all 49 fixed-S inputs, exact S blobs, read status, current sources, and source-qualified obligation groups.

## 4. Precedence inside this candidate

For D1 through D8, the A2 files in this directory are the current candidate text. A named historical decoder or historical scenario remains normative only for historical recovery or source-qualified evidence when D1.md, D2.md, or SOURCE-MAP explicitly says so.

For D9 through D10 there is no complete A2 current definition yet. Direct producer/consumer material consumed for D8 does not silently integrate those full owner modules.

## 5. Acceptance boundary

A subsequent non-author review bound to fixed26be (`26be071d4c2075343d9ebf272e00f23769ce0d64`) concluded PASS with P0=0 / P1=0 / P2=0 and CLOSED the final `A2-D7-2F89-P2-01`. Earlier bounded closures remain bound to their original SHAs. D7 is therefore accepted in its D7 scope; this does not accept D8 or global A2.

D8 fixed-SHA independent review must include the still-open source-map/FC/history/per-obligation semantic audit. D9-D10 full modules, D8/Search runtime-platform evidence and fresh independent Pro/global review remain pending; product/runtime evidence is UNRUN.
