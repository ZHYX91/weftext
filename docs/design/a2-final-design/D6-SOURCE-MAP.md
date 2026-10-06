---
source_language: zh-CN
translation_of: D6-SOURCE-MAP.zh-CN.md
translation_status: synced
---

[简体中文](D6-SOURCE-MAP.zh-CN.md)

# A2 D6 fixed01cc residual-repair source map

Status: independent review at `01cc40b819df78fbe724f1c64c27284ad60fc6c8` returned **REVISE**. `A2-D6-829-P1-01` and `A2-D6-829-P2-03` are independently CLOSED at that fixed object. `A2-D6-829-P1-02`, `A2-D6-829-P2-01`, `A2-D6-829-P2-02`, and navigation finding `A2-D6-01CC-P2-01` are the four residuals repaired from author start `32cfb9c387deddb12fb021a44147df0d7ffab322` and remain **author-resolved-pending-independent** until a new exact-stop review. This is not D6/global acceptance, implementation, activation, or release.

## 1. Review objects and status separation

The original five findings came from fixed829 `829efce6aacbe944714e093c98065b01d50b2593`; that SHA remains finding-origin provenance. The subsequent non-author review object is fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`.

Fixed01cc independently closed:
- `A2-D6-829-P1-01`: the one fresh current Key3→Proof3→Descriptor3→Prepared3/Notice3/final-P/CP4+ChangeRecord1 chain and its current trust/bootstrap/replica/execution successors.
- `A2-D6-829-P2-03`: per-producer-update schedule capacity of at most 4096 retained pins plus 16 MiB canonical evidence metadata, with source/component bytes under separate reserved PinBudget and gap-before-progress behavior.

Fixed01cc left OPEN/PARTIAL:
- `A2-D6-829-P1-02`: Chinese closed-union cardinality.
- `A2-D6-829-P2-01`: complete source→current-owner/anchor and bilingual source evidence.
- `A2-D6-829-P2-02`: four generic Registry fields and their exact pointer dispositions.
- `A2-D6-01CC-P2-01`: current fixed-object navigation.

The next reviewer must bind the exact final stop SHA recorded in PR metadata/handoff; this file intentionally does not self-reference an unknown future commit hash.

## 2. P1-02 — fifteen-arm current dependency family

Current fresh D6 is exactly `DependencyKey/3` with **15 arms**, ranks `source=0`, `document_format=1`, …, `execution_resource=14`; `DependencyProof/3`, `InputDescriptor/3`, `PreparedIntent/3` and `d6_plan/3` are the matching current family. English and Chinese Control must state the same count, and `D6-DK-CARD-01` in D6-IMPACT is the explicit design oracle.

A managed-Document semantic consumer binds `document_format`; M1→M2 makes the old Proof3 stale even if Source bytes/SourceVersion compare equal. A path that does not consume managed-Document semantics does not invent that dependency. Format-only transition has `sourceChanges=[]` and creates no SourceRevisionPlan, managed SourceVersion, or H advance. Genuine Key2/Proof2 remains the exact fourteen-arm historical family.

## 3. P2-01 — complete source/current placement

The machine map preserves all **299** section mappings but no longer treats whole files as semantic targets:
- all 23 fixed-S Control sections retain their already-precise anchors;
- the other 276 fixed-S Storage/Impact/Lexicon, current-parent Storage/Control/Impact/Lexicon, and fixed97 SPEC/SCHEMAS sections now identify actual current file+anchor(s), disposition, owner/basis, and source-qualified oracle;
- parent and fixed97 bilingual evidence records real EN/ZH path+blob pairs. Fixed-S snapshots remain their actual single preserved source; no nonexistent bilingual snapshot or JSON twin is invented.

For fixed97 ACCEPTANCE, all **760** original rows and original source text stay present. The **115** D6 intersections keep their full EN/ZH source rows. The five already precise targets remain unchanged; the other 110 now identify the actual current owner/consumer and concrete anchor(s) rather than the generic “D6-IMPACT intersection” placeholder. Cross-owner obligations name that owner and D6’s actual producer/consumer role instead of creating a second D6 authority.

The current fixed97 direct-owner contracts remain load-bearing: ResultPage §17.1, twelve-member BudgetBinding §17.2, ImportJob §17.3, managed configuration/control read §17.4, immutable ByteHandle/ByteRead §17.5, authorization-before-read/ObservationScope §17.6, and SourceBinding/OriginBinding §17.7. Historical numeric SourceVersion, Scope1, wire11/12 keep their business invariants only under their recorded versions.

## 4. P2-02 — four Registry successors only

D6-REGISTRY still has **52 concepts / 17 cross-stage bindings** and preserves conceptId, ownedNames, aliases, locale, firstFreeze and unrelated exact values. Only the four reviewed generic fields are repaired:
- commit-protocol definition: current D3 is wire13 + InputDescriptor/3 + `d3_identity_operation/13`; genuine wire9–12 is exact history; `d6_commit_request/2` remains the actual D6 submit.
- conflict-record definition: current outer is InputDescriptor3/Proof3/Prepared3; source_merge/choose_source_head retain inner Input2/Plan1/Preview1 + ownerKind/2; policy_bundle_choice uses inner Input3/Plan2/Preview2 + ownerKind/3.
- prepared-intent exclusions: current D3 wire13/InputDescriptor3/`d3_identity_operation/13` is not a D6-plan member; genuine wire9–12 stays historical.
- plan-token exclusions: same current/historical split; no D3 request gains an undeclared D6 plan token.

The corresponding four current-parent Registry pointers are named-current-successor mappings with their exact predecessor value/current family oracle. All other genuinely exact pointer mappings remain exact.

## 5. Evidence boundary

FULL for this residual repair: the D6 fixed/current owner inputs actually mapped here; all 299 D6 section records and their source-qualified targets; all 760 fixed97 ACCEPTANCE rows with the 115 D6 intersections fully targeted; both Registry pointer inventories; parent Impact 89 records (85 unique IDs + four range/metadata records); and the bilingual pairs explicitly recorded in the machine map.

PARTIAL: only the explicit D7-D10 direct-holder/load-bearing intersections needed by D6. Full D7-D10 A2 modules are not accepted or claimed fully read.

UNREAD/pending: unlisted full D7-D10 owner sources, Mandatory 925–1141, and product/runtime/OS/GUI/crypto/real-replica/crash/provider/performance/migration/activation/deployment evidence.

P1-01 and P2-03 stay independently CLOSED at fixed01cc and are regression-protected, not reopened. The four residuals above are only author-resolved-pending-independent until the new fixed-stop non-author review.
