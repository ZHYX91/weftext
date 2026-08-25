---
source_language: zh-CN
translation_of: 07-collections-query-and-views.zh-CN.md
translation_status: synced
---

[简体中文](07-collections-query-and-views.zh-CN.md)

# Collections, Query, and derived views

Collection views are permission-filtered derived views over one canonical Query result; they do not create an authoritative database, duplicate entities, or change identity. Query contracts are defined by [architecture 19](19-expression-query-and-template-library.md) and the corresponding specifications. Boards and multidimensional tables have their own persistent authority and adapters.

## Query and multidimensional-table boundary

A Query view may be a table, list, task list, board, calendar, timeline, gallery, chart, or dashboard. `from` selects an explicit source domain; typed `scope` defines the authorized population; `view` is only an initial presentation hint. Core owns parsing, typing, scope, authorization, execution, ordering, diagnostics, and action evidence. Clients cannot add aliases, fields, functions, implicit focus, or another evaluator.

A multidimensional table is one managed node with homogeneous table-local JSON records. A record is not a node. Query results use the term dynamic view; multidimensional table is reserved for authored record data. Boards may share presentation infrastructure but retain source-specific typed row identities and Core actions.

## Expressions, properties, and editing

`weftext.expr.v1` is bounded, deterministic, side-effect-free, and statically type-checkable. It has no filesystem, network, environment, ambient clock, random, dynamic-load, or `eval` access. Query lexical `this` is parser-owned embedding context, never UI focus; time enters only through explicit context.

Simple properties are bounded literal document-header attributes. Custom values are nullable strings unless a versioned schema says otherwise. Edits are narrow revision-bound Core actions. Bulk changes, drag, scheduling, relation changes, forms, and multi-row edits require deterministic plans, authorization, preview, draft gates, recoverable commit, and a receipt. Server authorization filters rows, fields, groups, counts, suggestions, diagnostics, and exports before delivery.

## Template Library and data surfaces

One configured Template Library root contains Template Roots and Template Parts. These remain canonical nodes but are excluded from ordinary projections and are visible through the library or explicit template Query domain. A Template Root owns `weftext.template.json`; instantiation creates a fresh-UUID ordinary subtree, rewrites internal links through a reviewed map, copies owned resources, and commits once.

Tasks use the checklist/task-node union in [architecture 18](18-task-nodes-and-checklist-promotion.md). Homogeneous authored data uses the multidimensional-table profile in [architecture 20](20-multidimensional-tables-and-editor-surfaces.md). Neither is smuggled into Query as a generic records domain.
