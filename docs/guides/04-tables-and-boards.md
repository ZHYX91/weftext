---
source_language: zh-CN
translation_of: 04-tables-and-boards.zh-CN.md
translation_status: synced
---

[简体中文](04-tables-and-boards.zh-CN.md)

# Multidimensional tables and boards

## Native and multidimensional tables

A native table is ordinary AsciiDoc document content. It is useful for a relatively stable small table inside a document, follows the document revision, and is maintained through document editing and formatting commands.

A multidimensional table is a special managed node for many homogeneous records with defined fields. One multidimensional table is one node; it is not one node per row. A record is not a node and cannot be promoted to one.

## Storing records without a database

**Accepted design:** beside the table node's canonical document, a `weftext.table.json` companion file stores fields and shared views. Each record is one bounded JSON file in `weftext.records`, sharded by record UUID.

This layout supports synchronization, backup, conflict diagnostics, and record-level changes while keeping ordinary files authoritative. Indexes and computed results are rebuildable, so a database does not become a second source of truth.

Links, rich text, and images can be stored as typed JSON values. A node relationship stores stable node identity; images and files remain ordinary attachments owned by the table node, and records refer to them. JSON can express these values, but the interface does not require users to edit JSON directly.

## Views and actions

The same records can have grid, board, calendar, timeline, gallery, and form views. Filters, sorts, groups, displayed fields, and summaries can be saved as shared views; column widths, scroll position, current selection, and temporary filters normally remain device-local.

A native table can be upgraded through a preview into a new multidimensional-table node. The upgrade validates headers and cells, creates the table node and records, and replaces the source table with a stable `node:` reference. The complete operation is one recoverable transaction and cannot leave a half-created table after failure.

## Boards

A board is a data layout, not a new card database. Task Board is only a Query-board preset; node, task, and multidimensional-table boards use the real identities and actions of their respective data sources.

Moving a card horizontally changes only an explicit grouping field. A task node changes state, a simple checklist can make only the state changes supported by its structure, and a multidimensional-table record changes the matching single-select field. Every drag action has menu, keyboard, touch, and screen-reader equivalents.

The first version has no manual vertical ordering inside a lane. Card order comes from the Query sort clause or table-view sort, avoiding a hidden ordering authority.

## Current status

**Current foundation:** existing Core code provides general file-classification, workspace-transaction, and derived-index mechanisms that this feature can reuse. There is currently no delivered multidimensional-table record store, editor, or board implementation.

**Pre-release limitation:** the multidimensional-table editor, native-table upgrade, all views, and generic boards have not completed product implementation and large-data acceptance.

See the detailed contracts for [multidimensional tables](../specifications/20-multidimensional-tables-and-editor-surfaces.md) and [board views](../specifications/21-board-views.md).
