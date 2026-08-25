---
source_language: zh-CN
translation_of: 17-tasks-and-query.zh-CN.md
translation_status: synced
---

[简体中文](17-tasks-and-query.zh-CN.md)

# Weftext tasks and query views

This specification defines the canonical two-level checklist/task-node model and its Query views. Earlier task metadata is accepted only through explicit, reviewed migration; runtime source and UI expose only the canonical model.

## Scope and authority

This specification defines native AsciiDoc checklist occurrences, durable managed task nodes, checklist-to-node promotion, their unified query projection, and explicit task-import/conversion boundaries. The architecture decision is [`../architecture/18-task-nodes-and-checklist-promotion.md`](../architecture/18-task-nodes-and-checklist-promotion.md).

The exact AsciiDoc source and managed node tree remain portable authority. Core alone recognizes checklist ranges, validates task-node attributes, resolves identity and dependencies, produces query plans/results, and constructs every revision-checked edit or workspace transaction. Desktop, CLI, Server, WebUI, exporters, and agents do not rediscover tasks, parse task attributes, patch source, or write files independently.

There is no canonical trailing task macro, task sidecar, task manifest, task database, arbitrary checkbox-status vocabulary, `tasks` code fence, JavaScript evaluator, or client-private query extension.

## Native checklist layer

A lightweight task is a native unordered checklist item:

```adoc
* [ ] Open
* [x] Closed
* [*] Also closed
```

`[ ]` is open. `[x]` and `[*]` are closed. Weftext writes `[x]` when closing an open item and preserves either accepted closed spelling until an explicit toggle changes that exact marker. `%interactive` is only an Asciidoctor rendering option; Weftext UI actions remain Core-owned source edits.

A checklist occurrence has no durable task identity. Its exact action target is `{ owningNodeId, documentRevision, sourceRange, parserOccurrence }`. Viewing, indexing, querying, or toggling it never writes an ID. Moving or editing surrounding source may invalidate that occurrence evidence; callers reload or report a stale target instead of guessing.

List nesting is authored outline structure only. It does not imply task dependency, parent task, project, inherited state/date/priority, recurrence, or task-node identity. A checklist cannot carry task-level annotations, resources, dependencies, dates, priority, or durable cross-document references. Requiring one of those capabilities invokes promotion.

## Durable task-node profile

A durable task is an ordinary active managed node. Its task identity is the node's existing lowercase UUIDv4 in `weftext.id`; the identity is not repeated in an attribute. The document title is the task title, the document body contains details and acceptance criteria, ordinary node resources are task resources, and `weftext.annotations.json` carries portable review.

The AsciiDoc document header contains this closed profile:

```adoc
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
---
= Publish Weftext 1.0
:weftext-task: v1
:weftext-task-state: in-progress
:weftext-task-priority: high
:weftext-task-scheduled: 2026-09-01
:weftext-task-due: 2026-09-05
:weftext-task-depends-on: 9b74c989-7bac-472f-9a8f-01f0db9f7a10
```

Only literal attribute entries in the document header are authority. Body redefinitions remain processor state. Values do not expand AsciiDoc attributes, substitutions, environment variables, paths, URIs, macros, or executable expressions.

| Source attribute | Decoded field | Value and rule |
| --- | --- | --- |
| `weftext-task` | `profile` | required exact `v1` |
| `weftext-task-state` | `state` | required `todo`, `in-progress`, `on-hold`, `completed`, or `cancelled` |
| `weftext-task-priority` | `priority` | optional `highest`, `high`, `medium`, `normal`, `low`, or `lowest`; absence means `normal` |
| `weftext-task-created` | `created` | optional ISO date or explicit-offset RFC 3339 instant |
| `weftext-task-start` | `start` | optional ISO date or explicit-offset RFC 3339 instant |
| `weftext-task-scheduled` | `scheduled` | optional ISO date or explicit-offset RFC 3339 instant |
| `weftext-task-due` | `due` | optional ISO date or explicit-offset RFC 3339 instant |
| `weftext-task-closed` | `closed` | optional; valid only when state is `completed` or `cancelled` |
| `weftext-task-depends-on` | `depends-on` | optional, unique task-node UUIDs separated by exactly one ASCII space |

[`../../schemas/task-node-v1.schema.json`](../../schemas/task-node-v1.schema.json) freezes the decoded field shape. Core additionally validates calendar dates, explicit offsets, duplicate attributes, state/closed combinations, UUID uniqueness, target task profiles, self-dependencies, duplicate edges, authorization, and cycles.

Date-only values remain calendar dates. Instants require uppercase `T`, optional fractional seconds, and `Z` or a numeric `+/-HH:MM` offset. Natural-language dates are UI input only and must be previewed as canonical literal values. `blocked` is derived from authorized open dependencies; `overdue` is derived from `due` plus an explicit calendar/offset context. Neither is stored as state. Recurrence fields and automatic successors are not part of task-node v1.

Unknown/duplicate `weftext-task-*` attributes, unsupported profile versions, missing/invalid state, illegal close data, malformed dates, missing/invalid dependency targets, or cycles make the task profile invalid. The underlying managed node remains visible under ordinary node rules, while task actions/projection report exact diagnostics and fail closed. Arbitrary non-reserved header attributes remain ordinary literal string properties and do not become task fields.

The workspace root, reserved Trash node, unmanaged content, and ignored content cannot carry this profile. A task node's directory parent and child nodes remain structural organization only. A child node is not automatically a subtask or dependency.

## Task-node identity and lifecycle

Rename, move, cloud replica, Trash, and restore preserve task identity because they preserve the node UUID. Copy follows ordinary node-copy rules and rekeys the copied subtree. Any accepted internal copied node links or dependency edges are rewritten only through the reviewed Core copy mapping; external edges keep their original UUID targets. Duplicate node UUIDs remain workspace identity conflicts.

Task-node state is one authoritative field. `todo`, `in-progress`, and `on-hold` are open; `completed` and `cancelled` are closed. A transition uses a narrow revision-checked header-attribute plan. Closing may add an explicitly supplied valid close date/instant; reopening removes `weftext-task-closed`. Core never fabricates a close time merely because an already checked checklist is promoted.

Task-node dependency replacement is a complete-set workspace plan over an authorized current graph. Every target must resolve uniquely to an active valid task node. Missing and unauthorized targets remain non-disclosing; self-edges, duplicates, and cycles fail without writing. A task node may be structurally moved without changing dependencies.

## Checklist-to-node promotion

`Promote checklist to task node` is a distinct semantic workspace action. Context menu, accessible row action, command palette, CLI, Server, and approved agent proposals all call the same Core planner.

The action target is captured once from one exact checklist occurrence. Planning is read-only and records:

- source node UUID, document/workspace revisions, exact checklist range, list depth, marker state, principal text, and the complete attached continuation/descendant branch;
- generated node UUID, selected active parent, portable directory name, document title, initial task attributes, exact new document source, and every created path;
- exact replacement source containing a normal unordered-list item at the same depth with `node:<uuid>[<label>]`;
- affected annotations/links/indexes, authorization, counts/bytes, exact draft-sensitive node IDs, and ordered recoverable journal steps.

The default target parent is the checklist's owning node. Preview may choose another existing active parent. It shows title, path/name, initial state, lifted content, link label, affected annotations, and every conflict. Core never silently suffixes an occupied name or creates a guessed parent.

`[ ]` maps to `todo`. `[x]` and `[*]` map to `completed` without synthesizing `weftext-task-closed`. The replacement is not a checklist and cannot be toggled independently:

```adoc
* node:550e8400-e29b-41d4-a716-446655440000[Publish Weftext 1.0]
```

The UUID is link authority; the label is authored display text. Weftext may enrich the resolved link with current task presentation, but it does not cache state or dates into the referring document. Rename does not silently rewrite an authored label.

Continuation blocks and descendant list content are part of promotion scope. Core may lift them only through a deterministic AsciiDoc-aware transformation whose exact result is previewed. If indentation, continuation attachment, protected content, or overlapping annotations cannot be converted/reanchored without ambiguity, planning fails rather than losing or duplicating content.

Commit rechecks the exact draft registry for the source and all rewritten nodes, both revisions, generated identity, destination occupancy, and source evidence. One recoverable workspace transaction creates the complete task node and replaces the checklist branch. Success contains both; rollback contains neither. Focus or selection changes after invocation cannot retarget the plan.

Automatic demotion is not defined. A task node may own body, descendants, resources, annotations, dependencies, permissions, or history that a checklist cannot represent.

## Canonical task Query projection

There is one canonical Query facility shared with node, heading, and Template Root domains. Its complete surface, `weftext.expr.v1` type system, lexical `this`, explicit time context, scope rules, bounds, and diagnostics are authoritative in [`18-canonical-query-and-expression.md`](18-canonical-query-and-expression.md). This specification freezes only the task-row semantics.

```adoc
.Due soon
[.weftext-query,version=1,view=task-list]
....
from tasks as task
scope subtree(this.node)
where task.closed = false
  and task.due is not null
  and task.due <= context.today + P14D
select task.id, task.title, task.state, task.due
order by task.due asc nulls last
limit 100
....
```

The body `from tasks as task` selects source semantics; the author may use another legal explicit alias, and every field remains alias-qualified. `view=task-list` is presentation only. The block does not inherit source, scope, time, node, document, or heading from UI focus.

The task alias is a tagged union. `kind` is `checklist` or `node`. `id` is null for a checklist and the node UUID for a task node. `owner_node` is the closed owning Node record. `title` is the checklist principal text or task-node authored/derived display title. `closed` and `state` are non-null. `checklist_depth` is checklist-only. Priority, `created`, `start`, `scheduled`, `due`, `closed_at`, and permission-filtered `blocked` are nullable task-node fields and remain null for checklists. The query surface never fabricates durable identity or task-node values for a checklist.

Rows carry non-projected action evidence. A checklist row retains owning node UUID/revision/exact parser range; a task-node row retains node UUID/revision and validated profile revision. Editing a row invokes the matching Core source/node action. Query results, counts, groups, boards, calendars, and task lists remain rebuildable derived views.

Authorization filters task candidates and resolves `subtree(this.node)`, `descendants(this.node)`, or `section(this.heading)` before expression evaluation, counts, grouping, ordering, projection, limits, errors, or suggestions. A query in the preamble or title-only document has null `this.heading`; `section(this.heading)` returns exact `missing_heading_context` rather than widening scope. `= Title` remains Document Title and `==` through `==========` remain H1 through H9.

Superseded Query grammar is not a runtime alias. It is accepted only by the private one-time migration converter described below.

## Import and migration boundary

Markdown is explicit importer input, not an alias in the canonical parser. Baseline syntax and selected extensions are recognized only through bounded, explicitly versioned compatibility profiles. [`../../schemas/task-import-v1.schema.json`](../../schemas/task-import-v1.schema.json) freezes the selected profile and its settings evidence; conversion produces:

| Source concept | Target |
| --- | --- |
| plain unchecked/checked item | native `* [ ]` / `* [x]` |
| typed lifecycle, date, priority, dependency, durable ID, or task detail | new managed task node plus original-position `node:` link |
| open/in-progress/on-hold/completed/cancelled source status | task-node `weftext-task-state` when more than simple open/closed meaning is required |
| prior task ID | generated task-node UUID mapping recorded in plan/receipt |
| dependency | resolved task-node UUID edge |
| safely convertible prior task query | canonical `.weftext-query` block |
| recurrence, reminders, scripts, unsupported/custom semantics | blocking decision until an accepted target exists |

Import planning inventories selected documents together, freezes source bytes/digests and settings, generates every node UUID/path once, previews every new node and source replacement, validates names/dependencies/content boundaries, and writes nothing. Commit requires an external exact snapshot, the same reviewed plan, recoverable workspace journal, receipt, and exact rollback.

Superseded trailing `task:[...]` metadata is handled by the same explicit migration. [`../../schemas/task-metadata-v1.schema.json`](../../schemas/task-metadata-v1.schema.json) defines accepted input. Its decoded `depends-on` value is bounded by the 4,096-byte target-header limit and at most 110 tokens/unique UUIDs; an overflow is one `InvalidDependency`, and repeated invalid, self, or duplicate tokens produce at most one diagnostic of each kind. Each valid structured occurrence becomes one task node through a reviewed old-to-new mapping that rewrites accepted dependencies and links, replaces the exact source position, and blocks recurrence or other information without a lossless target decision. Runtime never treats the macro and task node as two canonical forms.

Private prior Query blocks are also one-time migration input when they use superseded task fields or syntax. A read-only converter may map `phase`/`resolution` predicates to unified `state`, `structured` to `kind`, and prior owner fields to the canonical `owner_node` record only when type, null, authorized population, scope, ordering, and projection remain exact. It rewrites superseded source attributes, bare fields, bare `today`, body `sort`, and old scope spellings into the sole canonical surface. `recurring` and any field/predicate whose meaning is lost by the task-node target block conversion block migration. Commit changes task occurrences, dependencies, links, and affected Query blocks in the same reviewed workspace plan; runtime never retains two parsers or silently evaluates a different population.

## Migration execution boundary

The closed [`../../schemas/task-rebaseline-v1.schema.json`](../../schemas/task-rebaseline-v1.schema.json) contract defines complete-local-workspace migration preview. The opaque local capture is revision-bound and path-redacted, but it neither proves identity nor grants Owner permission; ACL/Owner authorization precedes planning. The planner inventories active macro input, freezes document/workspace revisions and exact source previews, assigns fresh task-node UUID/name/path mappings once, maps accepted fields only after the complete mapping exists, and revalidates reviewed IDs without regeneration. `conversionReady` means that the preview found no typed semantic blocker; it is committed only by its dedicated reviewed authority, never generic commit.

Migration transaction authority binds the reviewed digest and complete value, one workspace lease, exact root identity, old-to-new mapping, source/new-node bytes, annotation evidence, active drafts, an external snapshot, and staged recovery evidence. Commit rechecks current Owner authorization and drafts. Recovery validates exact endpoints, either finalizes the reviewed result or restores the reviewed pre-state, and preserves unknown states without unrelated writes.

Rollback authority derives only from the reviewed forward plan and exact committed result. It revalidates the complete result and original snapshot, requires current Owner review and a second confirmation, reverses source changes, moves generated task-node trees to journal holding, and validates every recovery artifact before finalizing either exact endpoint. It is recovery evidence, not a durable product receipt.

Untrusted serialized previews use Core's bounded JSON decoder before authority revalidation. Direct serde deserialization is only a data-model convenience and does not establish byte ceilings, canonical reviewed text, digest integrity, or current workspace evidence.

Core provides exact native/structured inline parsing, task UUID/dependency indexes, typed inline edits, recurring completion, dependency replacement, recoverable task transactions, import previews, CLI/Owner Server/local bridge contracts, and a shared local task inspector. Query callers use the canonical outer grammar, explicit task alias, lexical `this`, explicit `context`, stable output names, and the bounded derived-plan capability `weftext.query-expression-subset.v0`. The complete shared `weftext.expr.v1` evaluator, migration commit authority, hosted authorization, durable receipts, annotation migration, caller integration, accessibility/IME, and large-workspace operation must preserve their defined contracts before exposure. No surface may write superseded metadata after it writes task nodes.

## Acceptance boundary

Portable fixtures cover checklist recognition/toggle, protected contexts, mixed line endings, CJK/RTL/emoji, every task-node field, invalid combinations, duplicate/unknown attributes, dates/offsets, dependency failures/cycles, content boundaries, copy/rekey, Trash/restore, sync conflicts, and permission filtering.

Promotion fixtures cover leaf and nested checklist branches, continuations, generated names/UUIDs, alternate parents, conflicts, exact source replacement, node-link label escaping, annotation reanchoring/blocking, source/destination staleness, dirty drafts, every crash boundary, verified rollback, and absence of a surviving checkbox mirror.

Query fixtures cover the tagged row union, `task.title`, nullability, explicit aliases, lexical node/document/heading/query context, title-only/preamble missing heading, typed scopes, `context.today + P14D`, grouping, stable ordering, projection, limits, non-disclosure, malformed input, and resource ceilings. Cross-surface acceptance proves identical Core plans/results in Desktop, local WebUI, hosted WebUI, CLI, Server, import/export, backup/restore, synchronization, and approved agent actions. Migration acceptance proves complete externally backed up macro-to-node and prior-Query preview, commit, restart recovery, receipt, exact rollback, and no dual runtime.
