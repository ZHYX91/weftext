---
source_language: zh-CN
translation_of: D8-LEXICON.zh-CN.md
translation_status: synced
---

[简体中文](D8-LEXICON.zh-CN.md)
# A2 D8 Terminology and Naming

Status: author-resolved-pending-independent. The nine fixed-S D8 concept identities remain. D10 R08 technical-interface-owner metadata is absorbed as current coordination input, while current schema versions come from the actual D8 owner: Prepared Edit Binding is now /3 rather than reviving the older /2 proposal.

## 1. Nine D8 domain concepts

The only D8 domain concept IDs are edit_session, edit_draft, draft_projection, draft_edit_map, prepared_edit_binding, composition_transaction, caret_affinity, layout_epoch, and direction_preference under the weftext.term namespace. They respectively own Edit Session UI state, nonauthoritative Draft proposal state, discardable Draft Projection, proposal-local Edit Map, protected PreparedEditBinding/3, host IME composition state, upstream/downstream visual affinity, discardable layout generation, and device/session ltr|rtl|auto preference.

None is Workspace/Document identity, author source revision, D3 Locator, D7 PreparedActionBinding, D6 transaction/receipt, Query/result epoch, locale, portable author direction, or Query order. D2 snapshots, D3 values, D6 observations and D7 Query/View/Action data remain consumed owner data rather than new D8 concepts. No tenth Record/Row/DocumentRef/SearchUIState concept is introduced.

## 2. Thirteen kinds and unique technical owners

d8_document_read/document belong to the Document Read Interface. d8_draft_project/projection belong to Draft Projection. d8_draft_text_replace/replaced belong to Draft Text Replace. d8_draft_write/written belong to Draft Write. d8_edit_prepare/prepared belong to Edit Prepare. d8_undo_prepare belongs to Undo Prepare. d8_editor_error belongs to the Error Interface. Internal d8_prepared_edit_binding belongs to the Prepared Edit Binding Internal Interface and represents protected PreparedEditBinding/3, never a public response.

Several kinds may share one interface owner; consumed data concepts such as Draft Edit Map are not co-owners. document|annotation is only a local intent discriminator, never an EntityKind.

Current successor interfaces additionally include D8-owned Annotation read/draft-open and presentation-policy kinds. They add no new domain concept identity.

## 3. Mode and status vocabulary

Source / Write / Read are modes, not three Documents. readonly is interaction permission, not a fourth content copy. Locally saved Draft, pending submission, planned, outcome unknown, and committed are distinct. Preview is not a receipt.

Clean/dirty is independent from authority/version. A Draft exactly equal to committed after may be clean while retaining a new serial and current production/Observation identity.

## 4. Direction vocabulary

“RTL support” cannot be a single passing claim. Unicode content preservation, bidi interaction, RTL shell, and RTL localization are separate capabilities. dir=auto is only a presentation mechanism. Caret Affinity is not a source side and Layout Epoch is not a result epoch.

Use “Panel Partition” / 分面板 rather than Facet for D7 View partitioning. View textDirection, App locale, and authored source direction stay separate.

## 5. Search vocabulary

File List Filter, Quick Open, and Global Search are surfaces/presets, not Query kinds. plain/visual/shortcut are compiler-input UI, not executors. SearchConditionAst/1, QuerySpec/2, CanonicalGraph, and SearchContribution remain D7-owned.

title, subtitle, node_file_name, node_relative_path, resource_file_name, and displayTitle are distinct. Filename/path is never called title. Snippet/highlight offsets are never Locators.

## 6. Annotation vocabulary

AnnotationRef, Value/4, AnnotationInlineBody/1, Suggestion, target and reply remain D3-owned. D8 Annotation Draft is only proposal state. manual reattach, reconfirm suggestion, apply suggestion and reject suggestion are distinct operation classes; requalification is not reattachment. d9rg1 is inner Resource geometry, not a complete Locator.

## 7. Structured row vocabulary

Native Document Table Row, Field Value Occurrence, NodeRefCollection row, Query row, Import IR row and Resource preview row are distinct. They do not create RecordRef/TableRowId/CollectionRef. OccurrenceKey is not a durable row identity and rowHandle is not an author ID.

## 8. View vocabulary

A renderer consumes View data and does not own Query/aggregate semantics. Wide-to-long is a Query transform, never renderer melt. D7-deferred layouts stay deferred. A narrow-screen table/list fallback is a same-complete-data alternative with the graphical renderer unavailable, not a sampled fallback.

## 9. Controlled aliases

There is no published compatibility alias. Implementations remove unlisted controlled aliases rather than dual-read them. Forbidden aliases include EditorSession wire, author/source Draft, document_snapshot for Draft Projection, Locator/parser for Edit Map, PreparedActionBinding for Prepared Edit Binding, sourceAffinity/sourceSide, Query epoch for LayoutEpoch, localeDirection for DirectionPreference, persistent Record/row identity aliases, and any D8SearchEngine/D8QuerySpec.

## 10. Evidence boundary

This lexicon freezes owner naming and collision rules only. It does not prove locale resources or UI implementation. No new CLI verb or locale key is inferred from a design concept except the already-defined Direction Preference selector and Draft status phrases.
