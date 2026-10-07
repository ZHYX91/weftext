---
source_language: zh-CN
translation_of: REVIEW-ENTRY.zh-CN.md
translation_status: synced
---

[简体中文](REVIEW-ENTRY.zh-CN.md)
# A2 D1-D7 author-candidate review entry — D7 integration and navigation repair

Independent incremental review fixed85bdadf (`85bdadf448e0475715b62debe215fa4d04e30ab2`) concluded REVISE with P0=0 / P1=0 / P2=1. It independently CLOSED `A2-D7-2F89-P1-01`, `A2-D7-2F89-P1-02`, and `A2-D7-1068244-P2-01`; `A2-D7-2F89-P2-02` remains independently CLOSED at fixed1068244. The sole residual is `A2-D7-2F89-P2-01`, narrowed to the real QuerySpec/2/CanonicalGraph Search compiler oracle. This author batch starts exactly from fixed85bdadf, repairs only that residual and does not self-close it. Earlier bounded closures remain at their recorded SHAs. No D7/global acceptance is claimed. D8-D10 full modules and fresh independent Pro/global review remain pending; product/runtime evidence is UNRUN.

## 1. Fixed objects and author chronology

Base branch: docs/asciidoc-annotation-final-design.
Fixed base SHA: 97f4734f82a760cb6716c8122b84494da2b61164.
Fixed historical input S: 7e18168dad3e6d120fce0dd607dc10fa7894e252.
Protected inputs blob: 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.
Candidate branch: docs/a2-final-design-integration.

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

D7 is now FULL as an author candidate: fixed-S D7 thirteen sources, the extra D9 binding source, current bilingual D7 owner set, current Registry, and named A2 direct intersections are recorded in D7-SOURCE-MAP.json. D8-D10 remain PARTIAL only at named D7 producer/consumer intersections; their complete A2 modules remain pending.

Product/runtime/OS/GUI/crypto/real-replica/crash/provider/performance/migration/activation/deployment behavioral evidence is UNRUN. Documentation/machine/CI checks establish repository consistency only, not semantic acceptance. Final A2 still requires later complete integration, a fresh Pro/global non-author review, one explicit accepted-design SHA, and freeze/implementation-start material on that accepted SHA.
