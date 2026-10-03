---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 38d4f1f1-958a-4b10-a3e0-7d1627f87f3e.

# D8 Terminology and Naming

Candidate status: D8-FA-r01 PL-IR-01 repair terminology consumer; complete owner afterimage, not independently accepted, activated, or implemented. Fixed-S terminology identities remain. This lexicon consumes the authored stable-production-address/current-Observation split; the exact repair remains pending independent review and does not close P1. D3 conflict/current D7 effects gates remain separate.

Revision: D8-FA-r01; candidate. Existing D1–D7 concepts, Refs, Locators, Fields, Actions, PreparedIntent, and EffectManifest retain their domains. New concepts apply only at the explicit levels below. The complete public wire-name inventory belongs to Editor Interfaces/Direction; review tools, filenames, and test counts are not product terms.

| Concept | Canonical English / Chinese | Owner and naming | Must not be confused with |
|---|---|---|---|
| weftext.term.edit_session | Edit Session / 编辑会话 | UI state; EditSession | Workspace, Document identity, commit authority |
| weftext.term.edit_draft | Edit Draft / 编辑草稿 | Non-authoritative user proposal; Draft / draftSerial | Saved author source, sourceRevision |
| weftext.term.draft_projection | Draft Projection / 草稿投影 | Core's discardable projection of this proposal; d8_draft_projection | D2 document_payload, valid D3 locator |
| weftext.term.draft_edit_map | Draft Edit Map / 草稿编辑映射 | Core-produced flow/path/segment/plainRegion/site, bound to one complete proposal | D3 Locator, durable paragraph identity, client parser |
| weftext.term.prepared_edit_binding | Prepared Edit Binding / 编辑准备绑定 | Core immutable D8 record; d8_prepared_edit_binding | D7 PreparedActionBinding, ActionSpec, third ledger |
| weftext.term.composition_transaction | Composition Transaction / 组合输入事务 | UI begin/update/commit/cancel group | D6 author transaction or operation receipt |
| weftext.term.caret_affinity | Caret Affinity / 光标亲和位置 | Visual side of the same logical point, upstream/downstream | Source offset, permanent locator |
| weftext.term.layout_epoch | Layout Epoch / 布局代 | One shaping/wrapping/hit-test generation, discardable by the client | Result epoch, auth generation, author revision |
| weftext.term.direction_preference | Direction Preference / 方向偏好 | Device/session presentation, ltr/rtl/auto | Locale, portable author direction, Query ordering |

The closed D8 request/response kind inventory is `d8_document_read`, `d8_document`, `d8_draft_project`, `d8_draft_projection`, `d8_draft_text_replace`, `d8_draft_text_replaced`, `d8_draft_write`, `d8_draft_written`, `d8_edit_prepare`, `d8_edit_prepared`, `d8_undo_prepare`, and `d8_editor_error`. The separate internal binding kind is `d8_prepared_edit_binding`. `document|annotation` is only the local D8 intent discriminator, not an additional D3 entity kind. Editor Interfaces solely defines members and closed errors.

Source / Write / Read are mode names. Read-only describes current interaction permission, not a fourth document. “Draft saved on this device”, “pending submission”, “outcome unknown”, and “committed” are distinct; Draft autosave is not a saved Workspace. `planned` means Core retained the original plan; `preview` is not a receipt. A selection is distinct from its contiguous source range; a visual rectangle does not automatically denote that range.

“RTL support” must be separated into Unicode content preservation, bidi interaction, RTL shell, and translation of the UI into an RTL language. `dir=auto` is one presentation mechanism, not the complete capability. An Arabic citation locale is not Arabic UI localization.

Controlled-name verification covers these nine concepts and thirteen kind declarations and their uses, both Prepared Binding names in D7 transport, and D3/D6 consumer versions. It does not scan arbitrary user prose or code words and claim mechanically proved one-word/one-meaning natural language. Independent review explicitly decides terminology acceptance.

BodyPath, flow, segment, regionIndex, and siteIndex are local Draft Edit Map coordinates. splice_plain/break_plain/insert_at_site are Draft transformations, not D7 ActionSpec or D6 author transactions. inputSerial echoes the original request; projection.draftSerial is the next proposal sequence, not an author revision.

origins is a separate read-only Draft Projection member. Its elements/flows/relation confer no editing permission or new domain identity. plainRegion is local raw text, not a D2 paragraph; blank regions do not create empty D2 entities. The D2 Inline union remains exactly text/link/resource_occurrence/node_link/citation; kind:"resource" is not Inline. The resource arm of D3 Annotation target remains valid under its owner and must not be globally renamed.

Current consumer naming: new D8 envelopes use wire2; D8SourceTarget/2 binds complete SourceObservation/1. Draft baseObservation denotes current observation, not an alias for production revision. PreparedEditBinding/2 consumes D6 PreparedIntent/2, d6_plan/2, InputDescriptor/2, DependencyProof/2, Policy/3, and the actual D7 EffectManifest/2 producer. Real old /1 records recover under their original decoder without added members. Native D3 wire12 and D6 wire2 retain their respective submit fields. These versions introduce no tenth identity concept and do not change any of the nine concept IDs.

ReliableSaveState, InputRetention, WriteProtection, SemanticState, SourceStamp, SourceRevisionPlan, and CommitDomain remain D6-owned. The UI distinguishes retained, reliable, durable_observed_only, and portable publication. semantic_pending is not an alias for invalid; externalSequence is not sourceRevision. The D8 TextReplace response has no caret. Only d8_draft_written has a closed caret member; queue terminology cannot introduce an implicit field.
