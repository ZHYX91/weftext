---
source_language: zh-CN
translation_of: D7-SOURCE-MAP.zh-CN.md
translation_status: synced
---

[简体中文](D7-SOURCE-MAP.zh-CN.md)
# A2 D7 Source and Case Map

Status: sole-residual author repair after fixed85bdadf. That independent review is REVISE at P0=0/P1=0/P2=1: P1-01, P1-02 and the Impact-sync P2 are independently CLOSED at fixed85bdadf, immutable-source trace P2-02 remains CLOSED at fixed1068244, and only P2-01 is author-repaired here and remains OPEN pending independent review.

## 1. Fixed objects and prior closures

The repair start is 2f89a55cb1f924a47281f59e6419fff7c0c206ed. Base remains 97f4734f82a760cb6716c8122b84494da2b61164, fixed S remains 7e18168dad3e6d120fce0dd607dc10fa7894e252, and the protected inputs blob remains 787d03c31a55496f81ed03fd54a6fdfff50a2ad4.

fixed2f89 independently closed A2-D6-01CC-P2-01, A2-NAV-454E-P2-01 and COORD-D7-DOC-QUALITY-01 in bounded scope. fixed1068244 independently CLOSED A2-D7-2F89-P2-02. fixed85bdadf independently CLOSED A2-D7-2F89-P1-01, A2-D7-2F89-P1-02 and A2-D7-1068244-P2-01. The only remaining finding is A2-D7-2F89-P2-01; this author batch repairs only it and leaves it OPEN pending independent review.

## 2. Read coverage

The fixed-S D7 thirteen sources and the extra D9 coordinated D3-D7 binding source remain FULL from the completed D7 review. The current bilingual D7 owner set remains FULL. D1/D2/D4/D5 and D7-required D6 intersections retain the previously recorded coverage. D3/D8/D9/D10 remain PARTIAL only at the named direct D7 producer/consumer intersections; their whole modules were not reopened.

This repair reread the direct evidence required for the sole P2: applicable root/design AGENTS, protected inputs and both replacement layers, current QuerySpec/2, Query Algebra, Value/CEL, Query/View/Action §5 CanonicalGraph, D7-SEARCH, the complete Search fixture oracle and Impact/status surfaces. Previously accepted D6 metadata, Registry and D9 query_json semantics were not reopened.

## 3. Registry qualification

The immutable fixed-S Registry is 34 concepts / 8 cross-stage bindings. The parent-current Registry and D7-REGISTRY.json are the same blob 3cab5124f46822729093d5d955908b05eb65bb87 and contain 34 concepts / 13 bindings.

D7-REGISTRY-QUALIFICATION maps B01-B13 individually. D7-REGISTRY.json remains the only parent-current machine Registry and is not rewritten. A2 only names fresh successor versions where the actual fixed97/current owner changed; unchanged inner versions remain unchanged and real historical records keep their original decoders.

## 4. Query and Search residual repair

D7-QUERY-V2 keeps QuerySpec/2 as the current author successor and now consumes a D6-owned FileBinding metadata producer gated by entity_state+locator_state rather than whole content read or structure_state. It also closes Optional-title direct consumers, the exact union_all LogicalOccurrenceKey constructor and the D9 query_json result-only boundary.

D7-SEARCH keeps the already-closed lexer/escape behavior and D7-SEARCH-FIXTURES.json now contains explicit shortcut/visual/canonical ASTs plus actual strict-decodable QuerySpec/2 objects and direct §5 CanonicalGraph serializations. The former compiler description is retained only as explicitly non-wire review metadata. Mixed Node/Resource OR is really domain-specialized into legal CEL branches with one schema, union_all and final sort/project; Optional, Field/member NFC and the existing negative/browse cases are retained. D7-IMPACT EN/ZH are synchronized to this boundary.

## 5. Immutable source trace

D7-SOURCE-MAP.json schema version 2 no longer labels current-mirror summaries as original conditions. The 20 D7 integration obligations each carry immutable fixed-S sourceId/path/blob/section/range, precise current target, disposition, owner and positive/negative oracle.

Of the previous 151 case rows, 148 remain source-qualified case rows with immutable fixed-S or Mandatory source slices. The three old Chart mirror rows CHART-L194/L196/L198 are retained only as navigation summaries. Mandatory Chart §15 is separately mapped as seventeen contiguous obligation groups covering §15.1 through §15.8.9, including all ten §15.7 fixtures and both §15.8.9 hard questions.

The retained current scenario mirror remains useful as a current disposition target, but it never replaces immutable source text.

## 6. Current finding state

- A2-D7-2F89-P1-01: independently CLOSED at fixed85bdadf.
- A2-D7-2F89-P1-02: independently CLOSED at fixed85bdadf.
- A2-D7-2F89-P2-01: sole OPEN finding; real QuerySpec/2/CanonicalGraph oracle author repair is present in D7-SEARCH and D7-SEARCH-FIXTURES.json, pending independent review of this fixed delta.
- A2-D7-2F89-P2-02: independently CLOSED at fixed1068244; immutable mapping rows/groups are not reopened.
- A2-D7-1068244-P2-01: independently CLOSED at fixed85bdadf.

No author statement closes A2-D7-2F89-P2-01 or D7/global acceptance.

## 7. Evidence boundary and next gate

Repository documentation, input-integrity, JSON and source-map checks prove mechanical consistency only. Product Search, runtime, Desktop/Mobile/WebUI GUI, IME/AT, renderer/export, real index/provider, CAS/races, migration, deployment and historical execution remain UNRUN.

The next gate is a fixed-SHA non-author incremental review only of A2-D7-2F89-P2-01 and the directly changed Search fixture/oracle delta. All four prior closures retain their original review SHAs. D8-D10 full integration and the later fresh Pro/global A2 review remain separate.
