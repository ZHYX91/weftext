---
source_language: zh-CN
translation_of: REVIEW-ENTRY.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-ENTRY.zh-CN.md)
# A2 D1-D5 + fixed446 D3 repair author-candidate review entry

Status: stopped author candidate after the D4/D5 integration batch; not independently accepted, implemented, merged, released, deployed, or globally frozen.

## 1. Fixed base and review binding

Base branch: docs/asciidoc-annotation-final-design.
Fixed base SHA: 97f4734f82a760cb6716c8122b84494da2b61164.
Fixed historical input S: 7e18168dad3e6d120fce0dd607dc10fa7894e252.
Protected inputs blob: 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.
Candidate branch: docs/a2-final-design-integration.

A reviewer must bind to the exact PR5 head produced by this batch. Do not follow the moving branch and do not extend any older fixed-SHA conclusion beyond its stated scope.

## 2. Prior independent review state

The non-author fixed-44617a4 report independently CLOSED the bounded D1/D2 P1-01 and P2-01 findings.

That same report left D3:P1-01 OPEN for current-successor/predecessor dispatch and fourteen-key mixing, and D3:P1-02 OPEN for current canonical Annotation retaining the Value3 enum. The report reviews fixed 44617a4ddf3bbe538c7a9e2bab9fe8a4013fe706 only. Starting from the stopped D4/D5 head 9952f3ec19b88251fa43e7de8dec44d7e8954f24, this narrow author repair reconciles those two actual gaps. Their status is resolved-pending-independent-review only; this author does not mark either CLOSED/PASS.

## 3. D4/D5 completed author artifacts

D4.md, D4-IMPACT.md and D4-LEXICON.md form the D4 current author candidate. D4-SOURCE-MAP.json preserves fixed inputs, mandatory intake, acceptance rows and direct producer/consumer provenance. D4-CATALOG-MAP.json maps all catalog JSON Pointers while the immutable fixed catalog remains the sole static catalog authority.

D5.md, D5-IMPACT.md and D5-LEXICON.md form the D5 current author candidate. D5-SOURCE-MAP.json preserves the fixed D5 inputs, mandatory intake, acceptance rows, direct producer/consumer provenance and the explicit full-native-table/titleless/six-row-domain supersessions.

No separate D4/D5 schema authority is introduced. D4 closed schemas remain self-contained in the D4 main exact-contract sections; D5 table/collection/intents remain self-contained in the D5 main exact-contract restoration. Historical inner versions and SourceRevisionPlan branches remain source-qualified instead of being mechanically renumbered.

## 4. Read and mapping scope

Full fixed-S intake for this batch:
- D4 main, Impact, Lexicon, and the complete reference catalog;
- D5 main, Impact, and Lexicon.

Current parent D4/D5 main, Impact and Lexicon were consumed as complete bilingual files and carried forward with current overlays.

Mandatory scenario input §§1-14, lines 1-924, was read and mapped for D4/D5, including unnumbered prose and the Office-table binding section. Lines 925-1141 remain for later D6-D10 work.

The fixed97 acceptance table was parsed as a whole. D4-SOURCE-MAP preserves 72 selected D4-relevant rows; D5-SOURCE-MAP preserves 130 selected D5-relevant rows. D4 catalog mapping contains 2,854 JSON Pointers, 95 named records and 7 global limits.

D6-D9 owner files were read only at D4/D5 direct producer/consumer intersections; these are partial reads, not full module integration. There is no standalone D10 owner file in the parent owner tree, so D10 is fixed97 SPEC/SCHEMAS direct-partial only.

## 5. fixed446 D3 repair review targets

Review the exact repair head for three oracles: (1) fresh-current conflict preparation has exactly one Input13/Request13 path and historical Input12 remains record-recovery only; (2) unchanged managed source bytes with changed consumed document_format invalidates old Proof3 and requires the fifteenth Key3 arm; (3) current canonical concrete Annotation uses Value4 through plan/preview/PAB4/Effect3 and rejects current Value3, while genuine historical Value3 bytes remain untouched.

The current dispatch is named once by D3-SCHEMAS: Request/Input13; Key3/Proof3/Descriptor3/Prepared3; Notice3/CP4/ChangeRecord1; PAB4/EffectManifest3/EffectBytes3; Value4. D3ResolverInput12, D3DecisionCompanion2, RevisionTokenBinding2 and the SourceRevisionPlan1/2/3 role split are retained without mechanical bumps.

## 6. Pending and unrun scope

D6-D10 full A2 owner integration remains pending. Mandatory input lines 925-1141 remain pending. D3:P1-01 and D3:P1-02 are resolved-pending-independent-review, not author-closed; the next non-author review must bind this exact repair head.

Runtime, OS, GUI, real replica, product implementation, deployment, migration, and activation scenarios are UNRUN. Documentation/source/schema mapping checks are not product evidence.

Final A2 still requires completion of D6-D10, a fixed final SHA, fresh non-author global review, and same-accepted-design-SHA freeze/startup work.
