---
source_language: zh-CN
translation_of: 20-multidimensional-tables-and-editor-surfaces.zh-CN.md
translation_status: synced
---

[简体中文](20-multidimensional-tables-and-editor-surfaces.zh-CN.md)

# Multidimensional tables and editor surfaces

A multidimensional table is one managed node that owns a typed, homogeneous record store. It is distinct from Query: a record has a table-local UUID and is not a managed node, cannot acquire a node document/directory, and participates only through explicit typed relation fields.

## Persistent boundary

The table node retains canonical `X/X.adoc` source and a closed Profile sidecar/store for schema, views, records, and tombstones. Large records are sharded under the table-owned closed store rather than a giant JSON array or database file. Table Core owns validation, inventory, derived indexes, record identity, transactions, backup, Trash, and recovery. CSV and export files are interoperability output, never record authority.

Schema fields and view definitions are versioned, typed, and bounded. A shared view may store only accepted table-view configuration; personal presentation state stays local. Formulas, relations, rollups, sorts, filters, groups, and display fields are derived through Core and cannot leak unauthorized node references, values, counts, or diagnostics.

## Editor and action surface

The editor uses **Data** as the umbrella contextual surface for Query and multidimensional-table features. It exposes source-specific actions and never turns a grid buffer, rendered cell, or browser state into storage authority. Record create/edit/delete/restore, bulk paste/fill, import, relation change, and view changes use typed revision-bound Core plans with preview, draft gates, authorization, recoverable commit, and receipt.

## Native-table upgrade

An upgrade from a native AsciiDoc table is explicit, previewed, and transactional. It freezes the source table/range/revision, schema inference evidence, generated record IDs, destination files, source disposition, annotation consequences, and rollback steps. Unsupported or ambiguous source remains source and diagnostic evidence; upgrade never silently replaces it. The resulting multidimensional table remains one node and does not promote its records into nodes.
