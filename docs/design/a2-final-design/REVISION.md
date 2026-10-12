---
source_language: zh-CN
translation_of: REVISION.zh-CN.md
translation_status: synced
---

[简体中文](REVISION.zh-CN.md)

# Authoring and workflow revision entry

The subsequent primary-format candidate is now [native-first Typst](../typst-native/README.md), whose entry manages the coordinated specifications and this round's independent review status. This page and its referenced owners/machine entries continue to record the fixed AsciiDoc candidate below; they have not been fully rewritten for Typst and transfer no bounded pass or global acceptance to it. New-note authoring follows the new candidate's named rules, with product implementation and global final review retaining separate boundaries. This Typst revision completed independent Sol review; the new entry records its exact scope and unfinished global boundaries.

Status: design candidate. The native-attribute/link increment completed bounded independent review at `e703f9e7bb9908d8b6a322620b59b184d0f7bb5f`; the original global review remains incomplete, without global acceptance or implementation start. Revision base: `6b13b03b8da3e9e3d14b8663e91442568c8917e0`; author branch: `design/yaml-properties-ux-20261011`. [AUTHORING](AUTHORING.zh-CN.md), [authoring integration](AUTHORING-INTEGRATION.zh-CN.md), [EXPERIENCE](EXPERIENCE.zh-CN.md) and [LINKS](LINKS.zh-CN.md) define the new rules. Other modules reference these sole owners rather than duplicating configuration/grammar.

## 1. Changes

- Remove YAML front matter from new notes; ID/types use native `weftext-id` and `weftext-types`. `task` remains the shorthand for the original facet; the comma list defines only a value protocol.
- Simple business values use native AsciiDoc attributes on either side of the title with schema-defined types. Complex facts retain full capability; explicit conversion preserves provenance, notes and repeated values.
- Coordinate ID allocation/copy/recovery, whole-file coordinates, Registry name mapping, Field read/write and format binding rather than changing examples alone.
- Include five complete writing/research, capture, task/Query views, annotation/link and template-to-intranet-download workflows; give individual dispositions to other research candidates.
- Add stable-ID, source-relative, Workspace-root and local-absolute link forms with context-menu conversion/copy. Shortest paths never guess names Workspace-wide; ID-plus-anchor and general offline routing remain explicitly deferred.

This increment uses completed draft `35cf55f95da45ccab910adb85c2e7276197b47f6` as its preimage and directly replaces undeployed managed2 source mapping without inventing runtime history generations.

## 2. Named replacements and retained obligations

Only within this revision candidate, these exact concerns are replaced; unlisted obligations remain. Fixed sources/reports do not change, and prior reading/bounded acceptance does not accept the new design.

| Prior owner / concern | New owner | Disposition |
| --- | --- | --- |
| D2 / root `wf-kind`/`wf-facets` controls and predecessor YAML prohibition | AUTHORING §§1–2 | Reserved native controls replace draft YAML and old controls with complete origin qualification, without restoring an AsciiDoc subset. |
| D3 / source-carried Node identity and file binding | AUTHORING §3 | Inline ID declaration; original WorkspaceId/NodeId, birth, non-reuse, collision, lifecycle and authorization remain. |
| D4 §2.2 / closed RegistrySnapshot1 | AUTHORING §4 | Snapshot2 adds complete authoringBindings; Snapshot1 decoder and immutable catalog bytes remain unchanged. |
| D4 §16.1 / Entry-only Fields and MapAttribute-only import | AUTHORING §§4–6 | Bound native singleton attributes directly produce Fields; complex Entry1 remains; dual carriers conflict rather than duplicating facts. |
| D6 §§1–2 / physical document identity declaration | AUTHORING §3 | Native ID and original metadata birth/file binding agree under one decision; attributes do not store structure/permissions. |
| fixed97 SPEC §5 / managed1-only format | AUTHORING §7 | Exact new Profile2/Binding2 scope; recorded versions remain; source changes are not profile-only operations. |
| D7/D8/D9 / Entry-only materialization and copy/template source | AUTHORING §§5–7 | Distinguish actual header/Entry origins with full previews/materialization and original authority; no fabricated JSON source. |
| D10 / fresh RegistrySnapshot1 handshake with D4 | AUTHORING §4 | New authoring uses complete Snapshot2 with complete digest/owner validation; actual old plans retain original snapshots. |
| D8 / basic presentation and action entry points | EXPERIENCE §§1–6 | Five workflows, optional potential-link hints and accessibility without another writing authority. |

[LINKS](LINKS.zh-CN.md) §§1–7 coordinates D2 nativeTarget/adapter, D3 path/reference identity boundaries, D8 full Source conversion/clipboard and D9 copy/template relocation/exact export capability without secretly adding closed-shape members.

Machine entry [AUTHORING-REVISION.json](AUTHORING-REVISION.json) records these dispositions. Prior A2 source maps still locate their fixed versions and unchanged obligations. Explicitly changed semantics route to these new owners; an old retain entry cannot restore a superseded restriction. This revision does not duplicate a complete source catalog.

## 3. Verification and outstanding scope

Review SRC-01–12 in [AUTHORING §8](AUTHORING.zh-CN.md#8-验收要求), UX-01–18 in [EXPERIENCE §8](EXPERIENCE.zh-CN.md#8-设计验收) and INT-01–06 in [integration §7](AUTHORING-INTEGRATION.zh-CN.md#7-集成验收与剩余边界), plus LINK-01–11 in [LINKS §7](LINKS.zh-CN.md#7-设计验收), including positive, error, revocation/concurrency/recovery and real source/producer/consumer paths. These are design cases, product UNRUN; document checks and reviewer counts do not establish implementation.

The original fixed-6b13 global review's 6 P1 and 15 P2 findings, their interactions with the new design and the gaps found during repair have received independent bounded review: 21 CLOSED, none awaiting repair. The integrated candidate is `50946b94749a020a08622f295188ab5a46605a40`. Commit/recovery and symbolic groups closed at `0b70df71b56042adeb677034cf51181e574a1f1a`; the later delta changes native heading scans and consumer routing without changing those groups' closed contracts. Native and Query reviewers separately reviewed that delta. Every ID below retains the original A2-GLOBAL-6B13 prefix; original reports, counterexamples and supplements retain their historical identity.

| Original ID | Repaired concern | Current owner / clause |
| --- | --- | --- |
| P1-01 | Complete document copy and native identity materialization | [AUTHORING-INTEGRATION](AUTHORING-INTEGRATION.md) 6.1 |
| P1-02 | Commit, synchronization and recovery evidence for standalone presentation policy | [D6-CONTROL](D6-CONTROL.md) 0.1.4 |
| P1-03 | Native book body sections at level zero | [D2](D2.md) 12.1 |
| P1-04 | Complete native block locators and queries over unaddressable headings | [D2](D2.md) 12.1; D3 6.2; D7 23 |
| P1-05 | Read qualification for explicit Trash searches | [D7-SEARCH](D7-SEARCH.md) 11 |
| P1-06 | Contribution bindings and reopening saved searches | [D7-SEARCH](D7-SEARCH.md) 11 |
| P2-01 | Current version consistency for definition-transfer effects | [D3](D3.md) 21.2 |
| P2-02 | Genuine historical versions and atomic commit failure rules | [D3-IMPACT](D3-IMPACT.md) 2; D6-CONTROL 5,17.8; D6-IMPACT FA06 |
| P2-03 | Validation through actual nested Frontier fields | [D3](D3.md) 11.1 |
| P2-04 | Complete multiline terminology projections and regression checks | [D3-LEXICON](D3-LEXICON.md) D3-TERMS.json; scripts/check_d3_terms.py |
| P2-05 | Unique exact encoding for native Float values | [D2](D2.md) 12.1 |
| P2-06 | Full native table syntax and matching current acceptance | [D5](D5.md) 0,3.1,19.1; D5-LEXICON; D5-IMPACT |
| P2-07 | Both genuine historical CP2 recovery arms | [D6-CONTROL](D6-CONTROL.md) CP2; D6-IMPACT G21-07 |
| P2-08 | Initialization and continuation for both trust profiles | [D6-CONTROL](D6-CONTROL.md) 20.2; D6-IMPACT PL19,PL34,PL35 |
| P2-09 | Fields and signing domains of mixed trust declarations | [D6-SCHEMAS](D6-SCHEMAS.md) 9; D6-REGISTRY.json |
| P2-10 | Source mappings that preserve the exact original obligations | [D6-SOURCE-MAP](D6-SOURCE-MAP.md) D6-SOURCE-MAP.json; D6-IMPACT G21-10A,G21-10B |
| P2-11 | Issuing and consuming temporal media annotations | [D9-SCHEMAS](D9-SCHEMAS.md) 16 |
| P2-12 | Root source binding through actual nested fields | [D2](D2.md) 12.1 |
| P2-13 | Read boundaries for hidden targets and stored annotation digests | [D3](D3.md) Annotation Value4; D8-ACCEPTANCE READ-06 |
| P2-14 | Complete symbolic bootstrap payloads and materialization order | [d7/owners/effects/source](d7/owners/effects/source.md) 3 |
| P2-15 | Unique canonical ordering for presentation policy effects | [d7/owners/effects/source](d7/owners/effects/source.md) 4 |

This closes only these 21 findings and directly affected repair interactions. Unfinished D7–D10, global and Mandatory/source reading continues to retain its recorded boundaries. No acceptedDesignSha, implementation or release acceptance is created. Mechanical checks pass for 381 public documents, protected inputs, 42 complete terms and 27 relevant checker tests; these results do not substitute for semantic review. Product tests remain unrun.

Preserve all 49 fixed-S snapshots and `docs/design/inputs.json`. Keep Chinese/English synchronized. No product implementation, runtime migration, merge or deployment is claimed. Independent reviewers first inspect changes/affected dependencies, then discuss disagreements and corrections. Local passes do not replace the final full global review.
