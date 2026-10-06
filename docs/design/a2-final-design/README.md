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

The current author candidate integrates D1 through D6. It is authored work, not independent acceptance. D7-D10 remain future full-module batches.

## 2. Batch progress

| Module | A2 status in this batch |
| --- | --- |
| D1 | integrated current candidate; fixed-446 D1/D2 findings independently closed in that bounded scope |
| D2 | integrated current candidate; fixed-446 D1/D2 findings independently closed in that bounded scope |
| D3 | integrated current author candidate; a separate fixed-a62 non-author narrow review CLOSED the two bounded D3 P1 findings; this is not global A2 acceptance |
| D4 | integrated current author candidate; P2-02 independently CLOSED only at fixed4282 bounded D10-direct scope; P2-01 R1/R2 author-resolved-pending-independent and separately under non-author fixed32cf review; not self-CLOSED |
| D5 | integrated current author candidate; P2-02 independently CLOSED only at fixed4282 bounded D10-direct scope; P2-01 R1/R2 author-resolved-pending-independent and separately under non-author fixed32cf review; not self-CLOSED |
| D6 | fixed01cc independent review CLOSED A2-D6-829-P1-01 and P2-03, and left P1-02/P2-01/P2-02 plus new navigation P2 open/partial; the four residual repairs start from `32cfb9c387deddb12fb021a44147df0d7ffab322` and are author-resolved-pending-independent only |
| D7 | TODO as a full module; D2 intersections plus D3-direct Chinese owner inputs read |
| D8 | TODO as a full module; D1/D2 intersections plus D3-direct Chinese owner inputs read |
| D9 | TODO as a full module; D2 intersections plus D3-direct Chinese owner inputs read |
| D10 | TODO as a full module; TASK/navigation and selected D3-direct Chinese owner inputs read |
| Mandatory A2 source | D4/D5-applicable §§1-14 (lines 1-924) read and mapped; lines 925-1141 remain for later batches |

No TODO module is treated as accepted, complete, or semantically read merely because its path, route, blob, or title is known.

## 3. Current files

- D1.md is the self-contained current D1 candidate for this batch.
- D2.md is the self-contained current D2 candidate.
- D3.md, D3-IMPACT.md, D3-LEXICON.md, D3-SCHEMAS.md, and D3-TERMS.json form the current D3 author candidate.
- D3-SOURCE-MAP.json records D3 source-qualified cases and direct current coordination rows.
- D4.md, D4-IMPACT.md, and D4-LEXICON.md form the D4 author candidate; D4-SOURCE-MAP.json and D4-CATALOG-MAP.json retain provenance.
- D5.md, D5-IMPACT.md, and D5-LEXICON.md form the D5 author candidate; D5-SOURCE-MAP.json retains provenance.
- D6.md, D6-CONTROL.md, D6-SCHEMAS.md, D6-IMPACT.md, D6-LEXICON.md and D6-REGISTRY.json form the current D6 author candidate; D6-SOURCE-MAP.json preserves detailed provenance.
- REVIEW-ENTRY.md is the review entry for the D1-D6 author candidate.
- SOURCE-MAP.md gives the human-readable source/disposition map.
- SOURCE-MAP.json records all 49 fixed-S inputs, exact S blobs, read status, current sources, and source-qualified obligation groups.

## 4. Precedence inside this candidate

For D1 through D6, the A2 files in this directory are the current candidate text. A named historical decoder or historical scenario remains normative only for historical recovery or source-qualified evidence when D1.md, D2.md, or SOURCE-MAP explicitly says so.

For D7 through D10 there is no complete A2 current definition yet. Direct producer/consumer material consumed for D3-D5 does not silently integrate those full owner modules.

## 5. Acceptance boundary

The D1-D6 integration preserves source-qualified historical obligations and bounded review evidence. Fixed-446 independently closed the bounded D1/D2 findings, and fixed-a62 independently CLOSED the two bounded D3 P1 findings. Fixed4282 independently CLOSED only D4/D5 P2-02's bounded six-D10-source qualification. D4/D5 P2-01 R1/R2 is author-resolved-pending-independent and separately reviewed on immutable fixed32cf; this D6 repair does not decide it. The D6 non-author review on fixed01cc independently CLOSED P1-01 and P2-03 but returned REVISE with P1-02/P2-01/P2-02 and navigation P2 still open/partial. The current four-residual D6 repair starts from fixed32cf and remains author-resolved-pending-independent; D6 is not accepted. D7-D10 full modules, Mandatory 925-1141, and fresh global review remain pending. Runtime/OS/GUI/real-replica/migration/activation evidence is UNRUN.
