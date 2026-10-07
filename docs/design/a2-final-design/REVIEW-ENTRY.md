---
source_language: zh-CN
translation_of: REVIEW-ENTRY.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-ENTRY.zh-CN.md)
# A2 D1-D9 author-candidate review entry — D9 complete integration

A subsequent non-author review bound to fixed26be (`26be071d4c2075343d9ebf272e00f23769ce0d64`) concluded PASS with P0=0 / P1=0 / P2=0 and CLOSED the final D7 residual; earlier bounded closures remain bound to their original SHAs. D7 is accepted in its D7 scope. At fixed5eca, the later D8 non-author review independently CLOSED D8-C98B-P1-01, D8-C98B-P1-02 and D8-C98B-P2-02, leaving D8-C98B-P1-03 and D8-C98B-P2-01 OPEN and adding D8-5ECA-P2-01 OPEN. This successor author candidate repairs only those three residual D8 objects. The D9 author batch starts exactly at `5eca16c40cdf2e1892f6930d51c632ea720a460c`, fully integrates the eight D9 fixed sources, current bilingual owners, Mandatory §14, current D7 D9-binding/PAB consumers and final-FC D9 successors, and leaves D9 plus ROOT-D9-ZH-P2-01 only `author-resolved-pending-independent-review`. No D8, D9 or global A2 acceptance is claimed. Complete D10 integration and fresh Pro/global review remain pending; product/runtime evidence is UNRUN.

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

D7 is independently PASS at fixed26be in its D7 scope. For D8, fixed5eca independently CLOSED P1-01/P1-02/P2-02; only residual P1-03/P2-01 plus D8-5ECA-P2-01 are repaired by this successor author and remain pending exact-final-SHA independent review. D9 is a complete author candidate from exact 5eca: eight fixed-S sources FULL, current D9 owner EN/ZH FULL, Mandatory §14 FULL, D7 D9-binding/PAB consumers FULL and D9-relevant final-FC successors FULL. Fresh author work is currentized to wire13/PAB4/Effect3 and export/publication to Plan3/Receipt3; visible native-table selector and current Value4/R6 Annotation consumer design boundaries are present. The natural-Chinese quality repair removes the later filler workaround without changing D9 English semantics. Every D9 result remains author-resolved-pending-independent-review. Complete D10 remains later.

Product/runtime/OS/GUI/crypto/real-replica/crash/provider/performance/migration/activation/deployment behavioral evidence is UNRUN. Documentation/machine/CI checks establish repository consistency only, not semantic acceptance. Final A2 still requires later complete integration, a fresh Pro/global non-author review, one explicit accepted-design SHA, and freeze/implementation-start material on that accepted SHA.


## D8 review target

Review the exact final stop SHA from PR #5 only for D8-C98B-P1-03, D8-C98B-P2-01 and D8-5ECA-P2-01. Do not reopen the fixed5eca closures or unchanged D1-D7/D8 accepted scope, and do not infer semantic acceptance from authored repair or green CI. Verify the static definition-save gate versus unchanged D7 §7 runtime View gate across the ten `VIEW-BLD-01..10` fixtures; the four SourceTransform ordinary-save/recovery siblings and the updated 760-row counts (167 direct / 146 upstream / 447 retained owner; 114 authored applicability changes); the 54/54 section navigation with zero broad whole-file target; and the deterministic D8-ACCEPTANCE JSON/projection guard with 213/213 unique IDs. Protected S49/inputs must remain immutable. D9/D10 full modules and global A2 are out of this D8 incremental review scope.

## D9 review target

Review the exact final stop SHA from PR #5 as a new non-author D9 review. D9 is a complete author candidate only; source coverage, generated machine maps, counts, hashes and green documentation CI are not acceptance. Independently verify ROOT-D9-ZH-P2-01 as part of that D9 review: the ten affected Chinese public files must be natural complete translations with preserved protocol literals and EN/ZH structural parity, not a filler-based checker workaround. This D9 review does not accept the separate D8 residual scope.

The reviewer must verify:

- all eight fixed-S D9 sources against current D9 owner EN/ZH afterimages, Mandatory §14 lines 884–924, both replacement routers, current D7 D9-binding/PAB owners and D9-relevant final-FC successors;
- D9-SOURCE-MAP.json, D9-REGISTRY.json, D9-TERMS.json and the 80 unique rows in D9-ACCEPTANCE.json;
- the single Core parser/identity/authorization boundary, ImportIR/Mapping/Loss/ConversionInput/ImportJob closure, D4 Registry/Field admission and no second identity/ledger/patch authority;
- Node Template Recipe/ConstructionInput, sourceSubjectBindings to original-receipt join, PAB4/MinimumMapping/current wire13 and saved/planned/unknown recovery;
- ordinary Office token/style/repeat rules plus the visible qualified native-table selector grammar: template bytes self-contained, internal nt_/nc_ Plan keys only, shortest-unique suffix/title/occurrence, same-rowset repeat and Unicode escaping;
- complete ExportPlan/3 and PublicationReceipt/3 fields, typed evidence/recovery pins, generationPolicy=none exact paths, D2Snapshot3/D8 presentation rendering, output-name canonicalization, immutable confirmation, create-only publication and separate Resource author receipt;
- current PortableAnnotationRecord/4 / Value4 / sole R6 AnnotationInlineProfile consumer boundary: Node-template omission is local to construction and annotation_index never substitutes for body reads or creates a global plain-only downgrade;
- workers/routes/sandbox, D9 no-network, image/region policy, Mobile negative conversion surface and direct-but-not-full D10 intersections;
- historical 57/63/59/82/90/130/12 evidence sets remain separate, while I01–I12 and all product/runtime/GUI/Office/OS/performance/deployment evidence remain UNRUN.

Independent review must report P0/P1/P2 dispositions and any necessary read gaps. D9 cannot move beyond author-resolved-pending-independent-review until that review is complete. Complete D10 integration and a fresh non-author Pro/global A2 review remain mandatory afterwards.
