---
source_language: zh-CN
translation_of: 20-multidimensional-tables-and-editor-surfaces.zh-CN.md
translation_status: synced
---

[简体中文](20-multidimensional-tables-and-editor-surfaces.zh-CN.md)

# Multidimensional tables and editor surfaces

This specification defines the multidimensional-table contract. A product surface may create `weftext.table.json` and `weftext.records` only through Core inventory, closed decoding, transactions, recovery, backup, and the acceptance requirements in this document.

## Terms and non-goals

A **multidimensional table** is one ordinary managed node with a table profile and homogeneous table-local records. A **record** is one JSON value object owned by that table. A **dynamic view** is a canonical Query projection embedded in a document. A **native table** is an ordinary AsciiDoc table block.

These terms are not interchangeable. In particular:

- a record is not a node, document, heading, task, resource, annotation, or Query row domain;
- a record has no `weftext.id`, canonical `.adoc`, children, independent ACL, node properties, or node promotion path;
- a table profile is not a workspace-wide database, node registry, or general node-metadata directory;
- a dynamic view does not copy queried entities into table records; and
- canonical Query v1 does not acquire a hidden `records` domain through this specification.

The product may offer “Create linked node” as an ordinary node-creation plus node-relation transaction in a later version. It must not call that operation record promotion, preserve the record UUID as node identity, or make the two objects one authority.

## Portable layout and classification

A table node retains the canonical `X/X.adoc` document and may contain ordinary child nodes and resources. Its table authority consists of:

```text
X/
  X.adoc
  weftext.table.json
  weftext.records/
    <first-two-hex>/
      <lowercase-record-uuid>.json
```

The filenames `weftext.table.json` and `weftext.records` are reserved node-local names everywhere inside a managed node. The sidecar is valid only beside that node's canonical document. The record store is valid only when the adjacent sidecar is valid. Either name in an unmanaged location remains unmanaged content; either name in a managed node with the wrong file kind, link/reparse type, invalid profile, or invalid layout is a typed inventory diagnostic and never falls back to an ordinary node-owned resource.

Content rules cannot classify inside `weftext.records`, and node discovery never re-enters it. It contains exactly 256 optional lowercase shard directories `00` through `ff`; each contains only regular `.json` record files. Empty shard directories are valid. No nested directories, conflict-copy names, temporary files, symlinks, junctions/reparse points, or unrelated resources are valid portable authority there.

The record filename without `.json` is a lowercase UUIDv4. Its shard directory must equal the UUID's first two hexadecimal characters, and the decoded record `id` must equal the filename. A record identity is the typed pair `(owning table node UUID, record UUID)`; a record UUID need be unique only within its table, though Core-generated UUIDs remain random v4. There is no workspace-wide record registry.

Normal node-owned images and files stay ordinary resources outside the reserved record store. A conventional `assets/` directory is allowed but has no special authority. The table sidecar, every shard directory, every record byte, and every referenced resource participate in physical inventory, workspace revision, synchronization, backup, conflict detection, transaction planning, and node Trash/restore.

## Table sidecar v1

`weftext.table.json` is UTF-8 JSON with profile `weftext.table/v1` and integer version `1`. Its top-level members are exactly:

```json
{
  "profile": "weftext.table/v1",
  "version": 1,
  "primaryFieldId": "7a4cd2c3-e4bf-4ff6-b1e4-786f20b51218",
  "fields": [],
  "views": []
}
```

Duplicate keys, unknown members, non-UTF-8 text, non-integer versions, unsupported profiles, noncanonical UUIDs, duplicate IDs or names under Unicode NFC plus default case-folding, and configured limits fail closed. JSON member order and whitespace are not semantic. Core emits deterministic two-space-indented UTF-8 with LF when it authors or rewrites this dedicated machine-structured file; it never reserializes the canonical AsciiDoc document as a side effect.

There are 1 through 256 ordered field definitions. Each has exactly `id`, `name`, `type`, `required`, and `config`. `id` is a stable lowercase UUIDv4. `name` is mutable, trimmed display text of 1 through 128 Unicode scalar values and is never expression or record identity. `required` is boolean. `type` is one of:

`text`, `rich_text`, `boolean`, `decimal`, `date`, `instant`, `single_select`, `multi_select`, `url`, `email`, `node_relation`, `record_relation`, `image`, `file`, `formula`, or `rollup`.

`config` is closed by type:

- `text`, `rich_text`, `boolean`, `date`, `instant`, `url`, and `email` use `{}`;
- `decimal` uses `{ "scale": N }`, where `N` is 0 through 18;
- `single_select` and `multi_select` use `{ "options": [...] }`; each option has exactly stable UUID `id`, mutable `name`, and a token from the Weftext-owned accessible `color` catalog;
- `node_relation` uses `{ "multiple": boolean }`;
- `record_relation` uses `{ "targetTableNodeId": UUID, "multiple": boolean }` and the target must resolve to one authorized valid table node when evaluated or edited;
- `image` and `file` use `{ "multiple": boolean }`;
- `formula` uses `{ "resultType": scalar-type, "expression": string }`; and
- `rollup` uses `{ "relationFieldId": UUID, "targetFieldId": UUID, "function": aggregate }`.

Formula result types are `text`, `boolean`, `decimal`, `date`, or `instant`. Rollup aggregates are `count`, `count_non_null`, `sum`, `average`, `minimum`, and `maximum`; Core type-checks the referenced relation, target, and aggregate. Formula and rollup fields must have `required: false`. Cycles, unresolved field IDs, incompatible types, invalid target tables, and formulas outside the shared expression bounds invalidate the table profile rather than producing guessed values.

The primary field must resolve uniquely to a `text` field and cannot be removed or converted in v1. Its `required` flag remains an explicit schema choice. A rename changes only its display name. The UI uses its value as record display text and falls back to a localized “Untitled record” presentation without writing a default when the field is optional and absent.

## Record v1

Every record file is UTF-8 JSON with profile `weftext.table-record/v1` and integer version `1`. Its top-level members are exactly:

```json
{
  "profile": "weftext.table-record/v1",
  "version": 1,
  "id": "0a51d9d8-459d-4c32-9f68-0bf2b4e96f43",
  "state": "active",
  "trashedAt": null,
  "trashOperationId": null,
  "values": {}
}
```

`state` is `active` or `trashed`. An active record requires null `trashedAt` and `trashOperationId`. A trashed record requires a canonical UTC RFC 3339 `trashedAt` and a lowercase UUIDv4 `trashOperationId`; these values are generated once during the delete plan. Restore returns all three members to the active form. Product table views exclude trashed records unless the user opens the dedicated Deleted Records surface.

`values` is an object whose keys are existing non-computed field UUIDs. Missing means null; explicit JSON null is not a canonical stored value. A required field must be present with a valid nonempty value. Formula and rollup field IDs are forbidden. Unknown, duplicate, or removed field IDs invalidate the record until an explicit schema-repair transaction resolves them.

Value encoding is determined only by the field definition:

- `text`, `url`, and `email` are JSON strings; URL and email receive closed syntax validation but no network lookup;
- `rich_text` is one JSON string validated as bounded `weftext.asciidoc-inline.v1`, with no blocks, raw HTML, includes, macros with side effects, or editor-private JSON;
- `boolean` is a JSON boolean;
- `decimal` is a canonical base-10 string with no exponent, at most 34 significant digits, and no more fractional digits than the field scale;
- `date` is `YYYY-MM-DD`;
- `instant` is a canonical UTC RFC 3339 string ending in `Z`;
- `single_select` is one configured option UUID and `multi_select` is an ordered unique array of configured option UUIDs;
- `node_relation` is an ordered unique array of lowercase managed-node UUIDs, with at most one item when `multiple` is false;
- `record_relation` is an ordered unique array of objects containing exactly `tableNodeId` and `recordId`, with at most one item when `multiple` is false; and
- `image` and `file` are ordered unique arrays, with at most one item when `multiple` is false.

An image item contains exactly `path`, `alt`, and `caption`; a file item contains exactly `path` and `label`. Presentation strings are nullable bounded text. `path` is a normalized `/`-separated node-relative resource locator. It cannot be absolute, contain empty/`.`/`..` components, escape the owning table node, name its canonical document, table sidecar, record store, annotation sidecar, transaction evidence, or a link/reparse point. Resource bytes are never embedded in JSON. A missing or unauthorized target is a typed broken-relation diagnostic, not permission to rewrite or erase the authored value.

The table sidecar is limited to 1 MiB, a record to 1 MiB, a stored string to 64 KiB, record values to 256, select options to 4,096 per field, shared views to 128, relation items to 1,000 per cell, and referenced resource locators to 1,000 per cell. The shared expression token/depth/step limits also apply. Crossing a limit fails atomically with no partial record or partial result.

## Expressions, formulas, and table views

Table filters and formulas reuse the grammar, types, decimal rules, pure functions, and resource limits of `weftext.expr.v1`. The table profile adds one closed host binding:

- `record.id` is the table-local record UUID;
- `record.fields["<field UUID>"]` is the nullable typed value for a stored or computed field; and
- `context` is the same explicit evaluation-context record used by canonical Query.

There are no bare field names, mutable-name lookup, implicit current row, ambient clock, filesystem, network, environment, random values, dynamic loading, `eval`, or client-private functions. The visual builder displays field names but serializes stable IDs. Core detects formula and rollup dependency cycles before evaluation. A computed value is returned with input/schema revisions and cannot be edited as a stored cell.

Each portable shared view has exactly `id`, `name`, `layout`, `filter`, `fields`, `sort`, `group`, `summaries`, and `layoutConfig`. IDs and names follow the field identity/name rules. `layout` is `grid`, `board`, `calendar`, `timeline`, `gallery`, or `form`. `filter` is null or one bounded expression string. `fields` is an ordered array of `{ "fieldId": UUID, "visible": boolean }` containing every current field exactly once. `sort` is an ordered array of `{ "fieldId": UUID, "direction": "asc"|"desc", "nulls": "first"|"last" }`. `group` is null or one equivalent field/direction/nulls object. `summaries` is an ordered unique array of `{ "fieldId": UUID, "function": aggregate }` with type-checked aggregates.

`layoutConfig` is closed by layout: grid and form use `{}`; board uses exactly `groupFieldId`; calendar and timeline use exactly `startFieldId` plus nullable `endFieldId`; gallery uses nullable `coverFieldId`. Board v1 requires `groupFieldId` to resolve to one `single_select` field: option UUID/order supplies one unambiguous lane/value mapping, and missing value supplies Unassigned. Multi-select, relation, formula, rollup, and other fields cannot become a writable board group by visual inference. Field widths, row heights, density, open panels, temporary filters, selection, scroll, lane collapse, and virtualization windows are device-local and never written into the shared view. Core applies filter, authorization, stable sort/group, and a final record-ID tie-breaker before pagination. Summaries are computed over the complete authorized filtered population and are labeled separately from the delivered window; a client must not present a page subtotal as the total. The complete Board projection/action contract is [`21-board-views.md`](21-board-views.md).

Private views are stored outside the workspace by workspace UUID plus table-node UUID. “Save as shared view” previews the exact sidecar replacement and conflicts on a stale table revision. A shared view never contains copied record values or canonical Query source because its population is exactly the owning table.

## Dynamic views and canonical Query

An ordinary document inserts a dynamic view as one canonical `.weftext-query` block at the cursor. The builder requires a named source—Nodes, Tasks, Headings, or Templates—then edits scope, fields, filter, grouping, order, limit, and initial layout through the same Core parser/type checker used by Source. The generated body remains the sole portable semantic authority and degrades as an ordinary role-bearing literal block in generic AsciiDoc.

The source domain determines row kind. A layout never does: choosing task-list does not change `from nodes` into `from tasks`, and choosing table does not create multidimensional-table records. “Node collection”, “Task view”, “Heading view”, and “Template view” are UI names for explicit Query domains, not storage types.

The existing Query block `view` attribute is only the accepted initial presentation hint. Temporary filtering, widths, density, expansion, and scroll are device-local. A Query Board uses its canonical scalar `group by` as lane authority and follows specification 21; it does not reuse the table's `groupFieldId`. If portable column binding, custom lane order/labels, calendar fields, summaries, or other shared presentation cannot be represented by canonical Query v1, the UI may preview it privately but must disable “Save to document” with a precise version-gate explanation. It may not write frontend JSON, comments, attributes, or a second sidecar to bypass that gate. Query aggregates and any future records domain require a separately reviewed Query version.

Editable dynamic-view cells are available only when Core maps the selected projected field and typed row identity to one existing narrow action. Derived display titles, formula-like projections, group headers, summaries, hidden/unauthorized values, and ambiguous expressions are read-only. Bulk changes, board drag, calendar reschedule, and fill operations use ordinary deterministic workspace transactions and never mutate a browser result cache.

## Ribbon, context tabs, and Inspector

The Desktop and WebUI editor use one compact, collapsible ribbon backed by the shared semantic action registry. Its persistent tabs and minimum ownership are:

| Product label | Owns |
| --- | --- |
| Home (`开始`) | undo/redo, paragraph/H1-H9, format painter, clear format, inline marks, lists/checklists, quote/admonition, code basics |
| Insert (`插入`) | link/node link/embed, image, native table, multidimensional table, dynamic view, code/math/diagram, notes, citation, cross-reference |
| Data (`数据`) | fields, filter, sort, group, summary, layout, refresh, shared/private view state, canonical Query source entry |
| References (`引用`) | citation/reference node, footnote/endnote, bibliography, caption, cross-reference, index, diagnostics |
| Review (`审阅`) | comment, suggestion, spelling, change navigation, resolution |
| View (`查看`) | Write/Source/Read, panes, Outline/Inspector visibility, zoom/density, focus and accessible presentation |

Table, Image, Dynamic View, Task, and Template Design are contextual tabs. Only applicable tabs appear, and each consumes the same Core capability result and action as context menus, slash commands, shortcuts, selection bubbles, and the command palette. Narrow windows collapse groups into accessible labeled overflow; the ribbon cannot steal an IME composition or depend on hover/right-click.

Selecting a native table exposes Table actions such as leading header rows/columns, merge/split, insert/delete, captions, and upgrade. Selecting a multidimensional table or dynamic view exposes Data plus the corresponding context tab, with the object name and whether a change is private or shared always visible. Selecting a checklist or task node exposes Task for frequent state, priority, due, and dependency commands; it does not create a permanent seventh tab.

The right Inspector retains durable context panels: Outline, Properties, Annotations, Backlinks, Task detail, Citation/Reference detail, and permitted Server access detail. Task-node title chips may show state, priority, and due date, while the Inspector owns the complete form; a native checklist receives only its supported simple controls plus promotion. References is a permanent ribbon tab, not a special “Paper” tab or node type. Selecting a citation shows its occurrence and resolved reference; opening a reference node shows bibliographic fields; document scope shows citation and bibliography diagnostics. Academic document setups are templates, not storage modes.

Ribbon/Inspector/device state never enters canonical source unless the user invokes an explicit portable action. Focused tab, selected row, visual index, Inspector state, and current URL are not transaction identity.

## Core transactions

Every table mutation is a typed, revision-bound Core plan with exact target identity, current digests, proposed bytes, scope/count/byte evidence, authorization, draft sensitivity, no-clobber paths, journal steps, and recovery receipt. Required actions include:

- create a table node, canonical document, valid sidecar, and empty record store in one commit;
- add/edit one record by replacing exactly one record JSON file;
- bulk add/edit by staging every exact record replacement and committing once;
- soft-delete/restore records by exact state transitions, with one generated operation ID for a batch;
- permanently delete only selected already-trashed records under higher confirmation;
- add or rename a field through a sidecar-only change where values need no rewrite;
- remove or convert a field, delete an option, or change requiredness only after previewing every affected record conversion/blocker;
- add/edit/remove a shared view through an exact sidecar replacement;
- import resources and patch referencing records atomically;
- create or modify relations only after authorized typed target resolution; and
- upgrade one exact native table range into a complete fresh table node plus one block-embed replacement.

Schema and record writes cannot report partial success. A schema conversion stages the new sidecar and every affected record, revalidates the complete prospective table, then commits through one recoverable journal. A stale sidecar, record, document, relation target, resource, workspace revision, ACL, or device draft rejects commit instead of replanning.

Moving a complete table node preserves its node and record IDs. Copying gives the destination a fresh node UUID and every record a fresh record UUID, then rewrites record relations whose table target is the copied source through the complete mapping. Node relations and relations to external tables remain unchanged. Reverse relations outside the copied branch are not rewritten. Trashing/restoring the node preserves the complete sidecar, records including table-local trashed records, resources, children, and identities as one ordinary node-branch item.

Soft-deleting a record does not move bytes to Workspace Trash. Permanent deletion removes only selected trashed record JSON after exact digest confirmation and never deletes a resource implicitly. Deleting resources referenced by active or trashed records blocks or previews the exact reference repairs. Recovery at every injected journal step must produce either the verified old table or verified new table, never a mixed schema/record state.

## Native-table upgrade

Upgrade is offered only for a Core-parsed native AsciiDoc table and only when the canonical `node::UUID[]` block-embed feature is deliverable on that client. The preview includes the source node/range/revision, header interpretation, proposed field names/types/IDs, record IDs and values, destination node/parent/name, resources, exact replacement source, unsupported constructs, and complete transaction scope.

The first leading column-header region supplies display names; absent/duplicate names receive explicit reviewed proposals rather than silent final labels. Type inference is advisory and never commits without confirmation. Row-header columns are ordinary fields. Spans or merged data cells, nested/unsupported blocks, ambiguous headers, formula semantics, inaccessible resources, size limits, collisions, or an invalid destination block conversion until resolved.

Commit creates the fresh node and all authority, verifies it, and replaces the exact native-table range with `node::<new node UUID>[]` in the same journal. Failure or rollback restores the original source byte-for-byte and removes every generated target. A frontend may not leave the native table beside a separately created table node as two accepted copies.

## Sync, index, permissions, and backup

The physical scanner validates the sidecar and record-store structure before returning an active table projection. Partial sidecars, missing record shards, filename/ID/shard mismatch, duplicates, unknown fields, invalid values, conflict-copy names, links/reparse points, or incompatible schema/record arrival produce typed reconciliation issues. A previously verified device-local projection may remain available read-only and visibly stale; it cannot authorize a mutation or be presented as current authority.

Record filtering, sorting, grouping, summaries, formula results, reverse relations, and thumbnails are disposable derived indexes outside the workspace. They key entries by table-node UUID, record UUID, and authoritative fingerprints. Rebuilding from the same authorized portable bytes and explicit evaluation context must produce the same rows, order, groups, values, and diagnostics. A successful authority commit followed by index failure remains a successful commit with an index warning and rebuild requirement.

Table v1 has table/node-level permissions only. Server authorization filters table existence, schema fields, records, relations, summaries, options, diagnostics, exports, and picker suggestions before browser delivery. Hidden and missing relation targets are indistinguishable. Row-level ACL, public forms, per-record comments, automation, and external API tokens require later explicit specifications.

A full-workspace backup includes the canonical table document, sidecar, every shard directory and active/trashed record byte, ordinary resources, annotations, children, transaction evidence required for recovery, and any complete Workspace Trash item containing the table branch. Clean restore re-inventories the exact table and rebuilds indexes without a database backup. A backup that omits records or table-local trashed rows cannot claim the table is restorable.

## Acceptance matrix

Release evidence must include:

- exact valid/invalid sidecar and record fixtures; duplicate JSON keys; unknown members; UUID/file/shard binding; NFC/case-fold collisions; every field/value type; decimal/date/instant boundaries; constrained AsciiDoc inline values; and configured size/depth/count ceilings;
- field/option/view rename without identity churn; add/remove/type conversion; required-field blockers; formula/rollup typing and cycles; stable sort/null/tie behavior; context-bound time; full-population summaries; and deterministic index rebuild equivalence;
- node and record relations across rename/move/copy/Trash/restore, inaccessible targets, partial sync, target deletion, record soft delete/restore, batch operation IDs, permanent-delete confirmation, shared resources, and resource-reference repair;
- one-record, bulk, schema, resource, upgrade, copy, move, node Trash, restore, and permanent-delete transactions with stale conflicts, disk full, locks, process termination, every journal interruption, startup recovery, exact rollback, and successful-commit/index-failure behavior;
- native-table upgrade with headers, Unicode/CJK/RTL, CRLF/LF, empty/null values, advisory typing, resources, collisions, merged/protected blockers, exact `node::` replacement, and byte-identical rollback;
- 10,000-, 100,000-, and 1,000,000-record sizing evidence for incremental scan, index, filter, sort, group, summary, virtualization, bounded memory, and single-record edit without unrelated rewrites; normative budgets are recorded only after representative hardware is published;
- keyboard-only grid navigation and range selection, screen-reader row/column/header semantics, focus restoration under virtualization, high contrast, zoom, reduced motion, IME-safe cell editing, accessible ribbon overflow, contextual tabs, Inspector relationships, and parity of semantic actions across all entry surfaces;
- dynamic-view builder round-trip to exact canonical Query, source/layout separation, no hidden portable JSON, shared-save version gating, editable-cell action evidence, ACL non-disclosure, and identical Core results across every delivered caller; and
- full physical backup plus clean alternate-location restore containing active/trashed records, resources, relations, node Trash payloads, and no required database or index artifact.

Until this matrix passes, JSON examples and UI prototypes are format/design fixtures only and must not be written into user workspaces by a release build.
