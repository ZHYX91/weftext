---
source_language: zh-CN
translation_of: REVIEW-ENTRY.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-ENTRY.zh-CN.md)
# A2 D1-D9 author-candidate review entry — D9 complete integration

D7 fixed26be non-author PASS stays D7-bounded. Independently CLOSED findings at fixed5eca/ff10, fixed8b6, fixed4da7, fixed79026, fixed466b and the original D9-BAF-P1-02/D9-466B-P2-01 at fixed6012 remain bound to their original scope. The fixed6012 independent review was REVISE 0P0/1P1/1P2 only because of two NEW regressions: D9-6012-P1-01 graph query_json must not require ViewSpec.nodeDetails, and D9-6012-P2-01 invalid_output is not worker-only. This author batch repairs precisely those two as author-resolved-pending-independent-review, never self-CLOSED or D9/global PASS. Product UNRUN; full D10 and a new non-author Pro/global A2 review remain later.

## 1. Fixed objects and author chronology

Base branch: docs/asciidoc-annotation-final-design.
Fixed base SHA: 97f4734f82a760cb6716c8122b84494da2b61164.
Fixed historical input S: 7e18168dad3e6d120fce0dd607dc10fa7894e252.
Protected inputs blob: 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.
Candidate branch: docs/a2-final-design-integration.
D9 author integration start SHA: 5eca16c40cdf2e1892f6930d51c632ea720a460c.

D6 finding origin is fixed829 `829efce6aacbe944714e093c98065b01d50b2593`. The first non-author repair review was fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`. The four-residual author repair started at `32cfb9c387deddb12fb021a44147df0d7ffab322`; the later narrow follow-up passed through `d1ab2a5c0762450877614ad14340a26cfa0c1dfd`. The next independent review fixed the completed candidate at `1fc4a3937bbc06b4785d39f1eaf498f3c9c39abc`. The previous two-P2 repair started there. The semantic closure review fixed 5e21e9f00e1fa4e893f2544211133adbeac35ae0 and closed the semantic P2. fixed454e then opened two navigation-only P2 items. The later full D7 review fixed at 2f89a55cb1f924a47281f59e6419fff7c0c206ed independently closed both navigation items and the document-quality coordination item, while opening four D7 findings. This repair batch starts exactly from fixed2f89 and addresses only those four findings without self-closing them.

The next non-author reviewer must bind the exact final stop SHA recorded in PR metadata/handoff. Do not follow a moving branch.

## 2. Retained independent state

- bounded D1/D2 findings remain independently CLOSED.
- bounded D3 P1-01/P1-02 remain independently CLOSED at fixed-a62.
- A2-D4D5:P2-02 remains independently CLOSED only in the fixed4282 bounded six-real-D10-source qualification scope.
- A2-D4D5:P2-01 is now bounded CLOSED: fixed32cf had already CLOSED R2 and passed 188/202 R1 mappings; the fixed1fc4 independent review passed all 14 residual mappings.
- D6 `A2-D6-829-P1-01` and `A2-D6-829-P2-03` remain independently CLOSED from fixed01cc.
- The fixed1fc4 independent review additionally CLOSED D6 `A2-D6-829-P1-02` and `A2-D6-829-P2-02`.

None of these bounded conclusions accepts D6 or global A2.

## 3. Retained semantic repair and current navigation repair

### P2-01

`A2-D6-829-P2-01` concerns semantic disposition, not source inventory. The 299 section identities, 760 fixed97 cases, source bytes, bilingual conditions, Registry pointer inventories, parent Impact records, and protected inputs remain intact.

All 47 legacy `retain-or-named-current-successor` groups are re-audited by obligation. The machine map now distinguishes pure retained obligations, pure historical status/provenance, retained-but-unexecuted test duties, and `split-by-sub-obligation` groups. Each split group records line range, disposition, current owner/anchor, and an independently judgeable oracle. In particular, fixed-S Storage lines 20, 28–30, 36–46 and 52 no longer make SQLite-held current bodies, checkout-only authoring, or one-database source/control co-location current requirements: current D6 §1–§3 is the F/M/P/I/D file-backed authority. The retained rules are the actual business invariants such as one writable truth, no second Field/relation author authority, complete source retrieval, exact identity boundaries, continuity/fencing, and no LWW shortcut.

Three adjacent fixed-S cross-owner groups with the same old-version ambiguity are also split so old wire/Policy numbers are historical while their cross-owner business obligations remain explicit. This does not reopen the already-closed Key3/Registry or D4/D5 findings.

### fixed2f89 independent review state

At fixed2f89 the independent reviewer CLOSED `A2-D6-01CC-P2-01`, `A2-NAV-454E-P2-01`, and `COORD-D7-DOC-QUALITY-01` in their bounded scopes. They are not current pending findings.

fixed1068244 later independently CLOSED A2-D7-2F89-P2-02. It retained P1-01, P1-02 and P2-01 as OPEN and added A2-D7-1068244-P2-01 for D7-IMPACT EN/ZH drift. The next fixed85bdadf independent review then CLOSED P1-01, P1-02 and the Impact-sync P2, leaving only P2-01 OPEN. This author batch starts exactly from fixed85bdadf and repairs only that QuerySpec/CanonicalGraph oracle residual without self-closing it.

Historical chronology is fixed829 origin → fixed01cc → fixed32cf/d1ab → fixed1fc4 → fixed5e21 → fixed9c3b/fixed454e → fixed2f89 full D7 review → fixed1068244 incremental review → fixed85bdadf sole-residual review. Earlier verdicts describe only their fixed objects.

## 4. Evidence and nonclaims

FULL for this repair is limited to the D6 source-map semantic disposition/navigation surfaces and the already-recorded fixed/current owner evidence needed to judge them. Fixed-S snapshots and `docs/design/inputs.json` are protected and unchanged.

The two original D9 P1 and older fixed8b6/fixed4da7 P2 residuals were later independently CLOSED within their bounded reviews. The new fixed6012 P1/P2 are the sole current author-pending D9 findings; ROOT-D9-ZH-P2-01 remains CLOSED at ff10. This historical section does not reopen prior findings.

Product/runtime/OS/GUI/crypto/real-replica/crash/provider/performance/migration/activation/deployment behavioral evidence is UNRUN. Documentation/machine/CI checks establish repository consistency only, not semantic acceptance. Final A2 still requires later complete integration, a fresh Pro/global non-author review, one explicit accepted-design SHA, and freeze/implementation-start material on that accepted SHA.


## D8 review target

D8-C98B-P1-03 was independently CLOSED at fixed8b6. Its bounded static-decode/advanced-route/zero-total review is not reopened by the new D9 repair. Earlier fixed5eca/ff10 D8 closures remain in force; D8 product/runtime evidence is still UNRUN.

## D9 review target

New independent non-author reviewer: pin the actual final PR #5 SHA and review ONLY fixed6012→final for D9-6012-P1-01 and D9-6012-P2-01. Graph: original D7 execution/action §1 graph TerminalSchema, original D9 workers/export §3 query_json {nodes,edges} plus real columns/V/order, optional D7 network ViewSpec.nodeDetails?; authorize isolated node with empty edges and no ViewSpec, reject actual nodes.displayName omission, truncation or reorder. Coverage: original D9 Import IR §2 Host/Core, S33 omitted XLSX sheet despite worker success/schema-valid IR → invalid_output, and original D9 Region §8 issuance; caller View index=99/wrong payload still invalid_request, proved old protected contradiction integrity_conflict, proof/source availability and current auth/entered D7 ordering unchanged. Retain all fixed6012/earlier independent CLOSED outcomes, §3a original token sequence and historical Plan1–3; full D10/global later. Green docs are not semantic PASS.

The reviewer must verify:

- D9-BAF-P1-01 (independently CLOSED at fixed79026, retained context only): verify the single Annotation Value4/PortableRecord4/R6 carrier and independent context disclosure, then the exact catalog-index/mode/projection bijection plus exact selected annotation_content payload.recordPin comparison, cross-domain exclusion, no Document body selection inside Annotation plans, review-versus-backup target incompatibility, duplicate/unselected projection rejection, pin/currentness checks and freeze→loss→confirm→create-only/Resource/print outcomes;
- D9-BAF-P1-02 / D9-466B-P1-01 and D9-466B-P2-01 are independently CLOSED at fixed6012; they are context, not current review targets. Only new D9-6012-P1-01 (complete graph schema/data without required nodeDetails) and D9-6012-P2-01 (Host/Core/worker/Region invalid_output retained outside caller selector) await incremental non-author review;
- D9-8B6-P2-01 / D9-8B6-P2-02 (independently CLOSED at fixed4da7; retained context, not a new review target): preserve Annotation selection/projection sets, ordered disclosure fragments, canonical origins and renderer asset/evidence-pin arrays, plus unconditional renderer_unavailable for legal current D7 network in the closed six-layout D9 route. Do not convert graph query_json to rows or drop any actual D7 nodes/edges columns/V/order; optional network ViewSpec.nodeDetails is a presentation binding, not a required graph TerminalSchema/data member. Independent D9-BAF-P2-02 source-range closure remains fixed at fixed8b6;
- ROOT-8B6-MAP-P2-01 (independently CLOSED at fixed4da7; no new adjudication): preserve D8-SOURCE-MAP.json current-consumer ACCEPTANCE.md path/blob against the separately preserved 760 original row-source blobs. Verify D9-SOURCE-MAP.json source qualification, D9-REGISTRY.json, D9-TERMS.json and all 104 uniquely identified D9-ACCEPTANCE rows and their bilingual projections;
- the single Core parser/identity/authorization boundary, ImportIR/Mapping/Loss/ConversionInput/ImportJob closure, D4 Registry/Field admission and no second identity/ledger/patch authority;
- Node Template Recipe/ConstructionInput, sourceSubjectBindings to original-receipt join, PAB4/MinimumMapping/current wire13 and saved/planned/unknown recovery;
- ordinary Office token/style/repeat rules plus the visible qualified native-table selector grammar: template bytes self-contained, internal nt_/nc_ Plan keys only, shortest-unique suffix/title/occurrence, same-rowset repeat and Unicode escaping;
- fresh Plan4/Catalog3/Selection2/Projection2/Loss2/Confirmation2/Receipt4/PrintReceipt1 closed families plus exact Plan/Receipt1–3 recovery; FC SPEC §8.3 exact Source/Resource/query_json generationPolicy=none, controlled-name/canonical ordering, template/route/style, recursive evidencePins and unknown-publication rules are inherited by /4; SPEC §17 unseen-current dispatch, §18 inventory, SCHEMAS §§6.5–6.6, terminology, acceptance and replacement router must all agree that recorded /3 stays recovery-only; D2Snapshot3/D8 presentation rendering, immutable confirmation, create-only publication and separate Resource author receipt remain intact;
- current PortableAnnotationRecord/4 / Value4 / sole R6 AnnotationInlineProfile consumer boundary remains intact; annotation_index never substitutes for the new content carrier;
- workers/routes/sandbox, D9 no-network, image/region policy, Mobile negative conversion surface and direct-but-not-full D10 intersections;
- historical 57/63/59/82/90/130/12 evidence sets remain separate, while I01–I12 and all product/runtime/GUI/Office/OS/performance/deployment evidence remain UNRUN.

Independent review must report P0/P1/P2 dispositions and any necessary read gaps. D9 cannot move beyond author-resolved-pending-independent-review until that review is complete. Complete D10 integration and a fresh non-author Pro/global A2 review remain mandatory afterwards.
