---
source_language: zh-CN
translation_of: 03-tasks-query-and-templates.zh-CN.md
translation_status: synced
---

[简体中文](03-tasks-query-and-templates.zh-CN.md)

# Tasks, Query, and templates

## Two task forms

A native AsciiDoc checklist is useful for quick capture. It belongs to its document and source position, has no independent UUID, and has no durable priority, dependency relationship, or task-level annotation.

A task node is an ordinary managed node that uses its existing node UUID as task identity. It can have a body, attachments, annotations, state, priority, dates, and dependencies without requiring a separate task database.

**Accepted design:** when a lightweight checklist needs durable capabilities, the user can explicitly promote it to a task node. Weftext previews the new node location, identity, body, and source replacement, then commits them in one recoverable transaction. The original position becomes a stable `node:` link instead of retaining a checkbox copy that can drift out of sync.

## What Query does

Query derives collection views from current workspace content. It can query nodes, tasks, body headings, or template roles and can filter, sort, group, and select fields. A result is not a second database and does not change source-record identity.

The Data tab and Dynamic View builder edit the canonical Query stored in the document. When saving a filter, sort, or layout, the user can inspect the Query that will be written; the interface does not keep a separate hidden configuration.

Query and template expressions share `weftext.expr.v1`. They use the same values, operators, null rules, and safety limits, while Query clauses and template placeholders remain different because they solve different problems.

## The `this` context

`this.node` is the current node, `this.document` is the current document, `this.heading` is the nearest body heading that owns the current source position, and `this.query` is the current Query block.

When a document has only a title or the current position is in the preamble, `this.heading` is null. Use `this.document.title` for the document title, `this.document.subtitle` for the subtitle, and `this.document.display_title` for the derived display title. There is no `this.title` or `this.subtitle` shortcut.

## Template Library

**Accepted design:** the Template Library is a special managed subtree explicitly configured by the user. Template Root and Template Part entries still use ordinary node storage but have validated template roles and a companion file.

The Template Designer uses forms to declare parameters, defaults, choices, node names, and content slots, so users do not need to memorize placeholder spelling. Instantiation first shows a complete subtree preview, then creates fresh node UUIDs, rewrites internal links, and copies attachments in one transaction.

Template entries do not mix into ordinary search, task, graph, or timeline results. Moving an ordinary node into or out of the Template Library requires an explicit role conversion; moving a folder alone cannot change its role.

## Current status

**Current foundation:** Core has task parsing, parts of the typed task and Query planning capabilities, and shared caller boundaries.

**Pre-release limitation:** complete task promotion, the full expression evaluator, the visual Query builder, Template Designer, and template instantiation are still being implemented and accepted.

See the detailed contracts for [tasks and Query](../specifications/17-tasks-and-query.md), [Query and expressions](../specifications/18-canonical-query-and-expression.md), and the [Template Library](../specifications/19-node-template-library.md).
