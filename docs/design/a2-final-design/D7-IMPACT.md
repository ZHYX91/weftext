---
source_language: zh-CN
translation_of: D7-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D7-IMPACT.zh-CN.md)
# A2 D7 Implementation Impact and Acceptance Overlay

Status: design impact for the D7 author candidate after the fixed85bdadf independent incremental review. That review left only A2-D7-2F89-P2-01 open; this author repair addresses only that machine-oracle residual and does not self-close it. No implementation or product behavior is claimed.

## 1. Retained implementation surface

The complete retained implementation/test outline remains byte-for-byte under d7/owners/implementation-impact. Code work still has one Core Query DAG, one CEL evaluator, result paging/subscriptions, pure View projection, explicit Action prepare/preview/submit, EffectBytes delivery, Definition Transfer, Narrow Field qualification and exact historical version dispatch. The independently closed immutable-source mapping repair is not reopened.

## 2. Current successor deltas

Fresh authoring uses QuerySpec/2 while the outer execution carrier remains wireVersion2. QuerySpec/1 stays an exact historical/current-version decoder and is never widened. QuerySpec/2 adds only the versioned sources and generic union_all defined by D7-QUERY-V2.

Implementation must add the D6-owned FileBinding metadata producer from D6 §19 / D6-SCHEMAS §11. Node basename/path and Resource basename require current Ref disclosure plus locator_state, bind one protected current SourceObservation/FileObjectBinding cut, and do not require source_read/resource_read/structure_state merely for locator metadata. Separate body/Resource-byte reads keep their original content gates. Rename/move/auth-generation/observation changes reset dependent results.

Fresh QuerySpec/2 title is Optional<text>. Temporal Calendar/Timeline and link-display consumers must use the explicit Optional handling in D7-QUERY-V2; filename/path never synthesize title and any localized untitled placeholder is presentation-only. union_all must implement the exact internal K constructor with inputOrdinal, preserve duplicate input branches, inherit the existing checked result/work/byte budgets, and require explicit sort before take/paging order can be semantic.

The fixed97 current preparation/effect chain remains PAB4/Descriptor3/Proof3/PreparedIntent3, Action prepare/input version 3, ActionSpec2, D7ProposedInput3, EffectManifest3/EffectBytes3, D3 wire13, Notice3/CP4/ChangeRecord1, current D2DocumentSnapshot/3 and Value4 Annotation. No predecessor decoder is widened in place.

## 3. Search compiler and machine-oracle boundary

Ordinary text, visual conditions and explicit shortcut mode all produce the same SearchConditionAst/1 and compile through one QuerySpec/2 path. D7-SEARCH-FIXTURES.json is a design machine oracle: each positive case records shortcut AST, UI-neutral visual input/AST, canonical AST, non-wire compiler-review metadata, an actual strict-decodable QuerySpec/2 object, and the normative §5 CanonicalGraph serialization with canonical CEL ASTs/reference ordinals/field order. Negative/incomplete/browse cases record no Query. The mixed Node/Resource OR oracle contains real per-domain CEL specialization, one identical public schema, real union_all input order and the final sort/project.

Implementation/conformance must cover recursive precedence, adjacency, repeated NOT, exact keyword boundaries, FieldId/member-path grammar, the corrected single-backslash Windows and escaped-@ examples, bare trailing-backslash literal behavior, quoted trailing-escape incompleteness, all three preset empty states, explicit source override, Optional title handling, Node/Resource OR through union_all, incompatible-domain AND rejection and QuerySpec/1-vs-/2 strict dispatch.

The machine oracle is not evidence that a product parser or compiler ran. Repository/design checks may strict-check the fixture shapes and mechanically recompute the §5 relation graph, but product tests must later execute the real parser/QuerySpec decoder/CEL compiler and compare their produced canonical AST/Query graph against these design oracles.

## 4. Save/copy/export owner boundaries

Saved definition copy/fork/import remains D7 Definition Transfer plus the ordinary D3 author path, with version-specific QuerySpec decoding and exact embedded bytes. D9 query_json is only weftext.query-result-export/1 over a complete terminal D7 result; it preserves TerminalSchema/data/bag/order and is not QuerySpec author source, a saved-definition migration format or a second Query authority.

## 5. Mechanical and semantic checks

Repository checks must cover paired documentation, protected design inputs, JSON validity, D7 machine-map references, fixed-S 34/8 versus parent-current 34/13 Registry qualification, and exact unchanged S inputs. Direct design regression must separately check the D6 metadata authorization order and no-content-read positive cases, QuerySpec/2 Optional-title consumers, union_all K/order/budget behavior, Definition Transfer version dispatch, D9 query_json boundary, shortcut visual/compiler oracles, and the retained PAB4/Effect3/Narrow Field/View contracts that this repair does not otherwise change.

## 6. Product evidence boundary

Runtime, OS, GUI, IME, accessibility, renderer/export, real QuerySpec/2 parser/compiler, real metadata producer, index/provider, database/replica/race, performance, migration and activation scenarios remain UNRUN. Documentation, machine and source CI establish repository consistency only and cannot close D7 semantics.

## 7. Next gates

The next gate is a fixed-SHA non-author incremental review only of A2-D7-2F89-P2-01 and the directly changed Search fixture/oracle surfaces. A2-D7-2F89-P1-01, A2-D7-2F89-P1-02 and A2-D7-1068244-P2-01 remain independently CLOSED at fixed85bdadf; A2-D7-2F89-P2-02 remains independently CLOSED at fixed1068244. D8-D10 full modules, Search+D8 platform execution evidence, and the later fresh Pro/global review remain separate.
