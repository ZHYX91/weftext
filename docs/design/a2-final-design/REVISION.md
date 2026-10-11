---
source_language: zh-CN
translation_of: REVISION.zh-CN.md
translation_status: synced
---

[简体中文](REVISION.zh-CN.md)

# Authoring and workflow revision entry

Status: design candidate pending independent review, not global acceptance or implementation start. Revision base: `6b13b03b8da3e9e3d14b8663e91442568c8917e0`; author branch: `design/yaml-properties-ux-20261011`. [AUTHORING](AUTHORING.zh-CN.md) and [EXPERIENCE](EXPERIENCE.zh-CN.md) define the complete new rules. Other modules reference these sole owners rather than duplicating configuration/grammar.

## 1. Changes

- Restore note-carried minimal YAML: `weftext.id` and `weftext.types`; `task` is the short source name, still resolving to the original facet.
- Simple business values use native AsciiDoc attributes on either side of the title with schema-defined types. Complex facts retain full capability; explicit conversion preserves provenance, notes and repeated values.
- Coordinate ID allocation/copy/recovery, whole-file coordinates, Registry name mapping, Field read/write and format binding rather than changing examples alone.
- Include five complete writing/research, capture, task/Query views, annotation/link and template-to-intranet-download workflows; give individual dispositions to other research candidates.

## 2. Named replacements and retained obligations

Only within this revision candidate, these exact concerns are replaced; unlisted obligations remain. Fixed sources/reports do not change, and prior reading/bounded acceptance does not accept the new design.

| Prior owner / concern | New owner | Disposition |
| --- | --- | --- |
| D2 / root `wf-kind`/`wf-facets` controls and predecessor YAML prohibition | AUTHORING §§1–2 | YAML solely owns control; old attributes retain native meaning only, without restoring an AsciiDoc subset. |
| D3 / source-carried Node identity and file binding | AUTHORING §3 | Inline ID declaration; original WorkspaceId/NodeId, birth, non-reuse, collision, lifecycle and authorization remain. |
| D4 §2.2 / closed RegistrySnapshot1 | AUTHORING §4 | Snapshot2 adds complete authoringBindings; Snapshot1 decoder and immutable catalog bytes remain unchanged. |
| D4 §16.1 / Entry-only Fields and MapAttribute-only import | AUTHORING §§4–6 | Bound native singleton attributes directly produce Fields; complex Entry1 remains; dual carriers conflict rather than duplicating facts. |
| D6 §§1–2 / physical document identity declaration | AUTHORING §3 | Envelope and original metadata birth/file binding agree under one decision; no YAML structure/permission storage. |
| fixed97 SPEC §5 / managed1-only format | AUTHORING §7 | Exact new Profile2/Binding2 scope; recorded versions remain; source changes are not profile-only operations. |
| D7/D8/D9 / Entry-only materialization and copy/template source | AUTHORING §§5–7 | Distinguish actual header/Entry origins with full previews/materialization and original authority; no fabricated JSON source. |
| D10 / fresh RegistrySnapshot1 handshake with D4 | AUTHORING §4 | New authoring uses complete Snapshot2 with complete digest/owner validation; actual old plans retain original snapshots. |
| D8 / basic presentation and action entry points | EXPERIENCE §§1–6 | Five workflows, optional potential-link hints and accessibility without another writing authority. |

Machine entry [AUTHORING-REVISION.json](AUTHORING-REVISION.json) records these dispositions. Prior A2 source maps still locate their fixed versions and unchanged obligations. Explicitly changed semantics route to these new owners; an old retain entry cannot restore a superseded restriction. This revision does not duplicate a complete source catalog.

## 3. Verification and outstanding scope

Review SRC-01–10 in [AUTHORING §8](AUTHORING.zh-CN.md#8-验收要求) and UX-01–16 in [EXPERIENCE §8](EXPERIENCE.zh-CN.md#8-设计验收), including positive, error, revocation/concurrency/recovery and real source/producer/consumer paths. These are design cases, product UNRUN; document checks and reviewer counts do not establish implementation.

The fixed-6b13 global review's six P1 and fifteen P2 findings remain open: symbolic Document input, standalone presentation policy production, native heading/block product domains, Trash/saved search, versions/sources/decoders, annotation time/hidden digests and effect ordering. New interaction descriptions do not automatically fix them. Actual affected repairs need complete counterexamples and closure evidence. Outstanding D7–D10/global/Mandatory/source reading also receives no automatic completion credit.

Preserve all 49 fixed-S snapshots and `docs/design/inputs.json`. Keep Chinese/English synchronized. No product implementation, runtime migration, merge or deployment is claimed. Independent reviewers first inspect changes/affected dependencies, then discuss disagreements and corrections. Local passes do not replace the final full global review.
