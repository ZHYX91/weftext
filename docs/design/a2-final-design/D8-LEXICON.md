---
source_language: zh-CN
translation_of: D8-LEXICON.zh-CN.md
translation_status: synced
---

[简体中文](D8-LEXICON.zh-CN.md)

# A2 D8 Terminology and Naming

Status: author-resolved-pending-independent. The nine fixed-S D8 concept identities remain. D10 R08 technical-interface-owner metadata is consumed as current coordination input, while the current schema version comes from the real D8 owner. Prepared Edit Binding is currently /3; the older /2 proposal is not made current again.

## 1. Nine D8 domain concepts

| Concept ID | Chinese / English | Current owner/type | Explicit exclusion |
| --- | --- | --- | --- |
| weftext.term.edit_session | 编辑会话 / Edit Session | D8 UI state; EditSession | Not Workspace/Document identity or commit authority |
| weftext.term.edit_draft | 编辑草稿 / Edit Draft | D8 proposal; Draft + draftSerial | Not author Source or source revision; this row defines the current naming boundary only and adds no identity, permission, or execution semantics |
| weftext.term.draft_projection | 草稿投影 / Draft Projection | Core D8 projection; d8_draft_projection | Not a D2 Snapshot/payload or Locator; this row defines the current naming boundary only and adds no identity, permission, or execution semantics |
| weftext.term.draft_edit_map | 草稿编辑映射 / Draft Edit Map | flows/plainRegions/sites | Not durable paragraph identity, Locator, or client parser |
| weftext.term.prepared_edit_binding | 编辑准备绑定 / Prepared Edit Binding | protected PreparedEditBinding/3 | Not D7 PreparedActionBinding or a third ledger |
| weftext.term.composition_transaction | 组合输入事务 / Composition Transaction | D8 host IME state | Not a D6 author transaction or receipt |
| weftext.term.caret_affinity | 光标亲和位置 / Caret Affinity | upstream/downstream | Not a source offset or Locator |
| weftext.term.layout_epoch | 布局代 / Layout Epoch | discardable layout generation | Not a result/auth/source revision; this row defines the current naming boundary only and adds no identity, permission, or execution semantics |
| weftext.term.direction_preference | 方向偏好 / Direction Preference | device/session ltr|rtl|auto | Not locale, portable author direction, or Query order; this row defines the current naming boundary only and adds no identity, permission, or execution semantics |

No tenth D8 domain concept such as Record, Row, DocumentRef, or SearchUIState is introduced. D2 Snapshot, D3 Value, D6 SourceObservation, and D7 Query/View/Action remain data consumed from their original owners; D8 UI use does not transfer ownership.

## 2. Thirteen kinds and one technical interface owner per kind

| kind | Technical interface owner | Consumes / operates on | Returns / owner boundary |
| --- | --- | --- | --- |
| d8_document_read | D8 Document Read Interface | NodeRef + current D6 read qualification | d8_document |
| d8_document | D8 Document Read Interface | exact read result | Snapshot/3 envelope |
| d8_draft_project | D8 Draft Projection Interface | Draft proposal + current cut | d8_draft_projection |
| d8_draft_projection | D8 Draft Projection Interface | one proposal | projection + Draft Edit Map |
| d8_draft_text_replace | D8 Draft Text Replace Interface | Draft + one proved scalar segment | d8_draft_text_replaced; this row defines the current naming boundary only and adds no identity, permission, or execution semantics |
| d8_draft_text_replaced | D8 Draft Text Replace Interface | exact replace result | complete projection; no implicit caret |
| d8_draft_write | D8 Draft Write Interface | Draft + region/site mapping | d8_draft_written |
| d8_draft_written | D8 Draft Write Interface | exact command result | projection + caret |
| d8_edit_prepare | D8 Edit Prepare Interface | D8EditPrepareRequest/3 | d8_edit_prepared |
| d8_edit_prepared | D8 Edit Prepare Interface | PreparedEditBinding/3 result | original request + preview; no commit claim |
| d8_undo_prepare | D8 Undo Prepare Interface | original decision/effects + exact current after | d8_edit_prepared for a fresh inverse; this row defines the current naming boundary only and adds no identity, permission, or execution semantics |
| d8_editor_error | D8 Error Interface | D8 public-entry failure | closed D8 owner error |
| d8_prepared_edit_binding | D8 Prepared Edit Binding Internal Interface | immutable /3 preparation state | protected only; never a public response kind |

Several kinds may share one interface owner. Data concepts such as Draft Edit Map do not co-own a kind. document|annotation is only a D8 intent discriminator and never an EntityKind.

Current successor interfaces also include D8-owned Annotation read/draft-open and presentation-policy kinds. They do not create another domain concept identity:

- d8_annotation_read;
- d8_annotation_draft_open;
- d8_presentation_policy_set;
- d8_presentation_policy_no_change / d8_presentation_policy_prepared;
- internal presentation-policy record/head/outbox kinds.

## 3. Mode/status vocabulary

Source / Write / Read are modes, not three Documents. readonly describes current interaction permission, not a fourth content copy. Draft saved locally, pending submission, planned, outcome unknown, and committed are distinct states. preview is not a receipt.

clean/dirty is independent from authority/version. A Draft exactly equal to the committed after-image can be clean while serial, production version, and Observation remain current/new values.

## 4. Direction vocabulary

"RTL support" cannot be a single passing item. It must be split into Unicode content preservation, bidi interaction, RTL shell, and RTL localization. dir=auto is a presentation mechanism, not the complete capability. Caret Affinity is never called a source side. Layout Epoch is never called a result epoch.

Use "Panel Partition" / 分面板 rather than Facet so it cannot be confused with D4 Facet. View textDirection, App locale, and author-source direction remain separate.

## 5. Search vocabulary

File List Filter, Quick Open, and Global Search are surfaces/presets, not new Query kinds. plain, visual, and shortcut are compiler-input UI modes, not three executors. SearchConditionAst/1, QuerySpec/2, CanonicalGraph, and SearchContribution remain D7-owned.

title, subtitle, node_file_name, node_relative_path, resource_file_name, and displayTitle are distinct. They never become interchangeable merely because one UI displays them together. filename/path is never called title. snippet offset/highlight offset is never called a Locator.

## 6. Annotation vocabulary

AnnotationRef, Annotation Value/4, AnnotationInlineBody/1, Suggestion, target, and reply remain D3-owned. A D8 Annotation Draft is only proposal state. manual reattach, reconfirm suggestion, apply suggestion, and reject suggestion are distinct operation classes. requalification is not reattach.

The d9rg1 member inside resource_region is an inner geometry token, not a complete Locator. Complete ResourceRegionLocator remains D3-owned.

## 7. Structured-row vocabulary

Native Document Table Row, Field Value Occurrence, NodeRefCollection row, Query row, Import IR row, and Resource preview row are six distinct domains and cannot be collapsed into Record. A single grid UI does not erase these domain boundaries. RecordRef, TableRowId, and CollectionRef are forbidden. OccurrenceKey is not durable row identity; rowHandle is not an author ID.

## 8. View/renderer vocabulary

A renderer consumes View data and is not the owner of Query or aggregation. wide-to-long is a Query transform and is not called renderer melt. A D7-deferred layout cannot be marked supported. A table/list fallback must be described as "same complete data alternative; graphical renderer unavailable" rather than a sampled fallback.

## 9. Controlled aliases

There is no published compatibility alias. When implementation adopts these names, it removes any unlisted controlled type/kind alias; historical research prose and user prose are not migrated.

Forbidden aliases include:

- EditorSession wire alias;
- author_draft/source_draft;
- document_snapshot as an alias for Draft Projection;
- locator/parser as an alias for Draft Edit Map;
- PreparedActionBinding as an alias for Prepared Edit Binding;
- sourceAffinity/sourceSide;
- Query epoch as an alias for LayoutEpoch;
- localeDirection as an alias for DirectionPreference;
- Record/row identity aliases;
- D8SearchEngine / D8QuerySpec.

## 10. Naming evidence boundary

This lexicon freezes owner naming and collision rules only. It does not prove UI locale implementation. Apart from the existing Direction Preference selector and Draft status phrases, design prose does not create a new CLI verb or locale key.
