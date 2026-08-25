---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext documentation authority

This directory contains the current public product contracts and architecture decisions. Repository documentation uses Markdown independently of the managed Weftext AsciiDoc format.

Public documents describe Weftext behavior and external format classes generically. Product comparisons, screenshots, research, decision history, schedules, handoffs, and acceptance logs stay in the private control workspace outside this repository.

The bilingual source/translation layout and public-content rules are defined in the [documentation policy](DOCUMENTATION.md). Shared translations and writing conventions are defined in the [terminology guide](TERMINOLOGY.md).

## Reading paths

- To understand how Weftext stores and organizes content, start with the [user guide](guides/README.md).
- To implement or review a feature, use the authority map below to find its specification and architecture decision.
- To see what exists now and which release gaps remain, read the [public roadmap](../ROADMAP.md).

## Authority map

| Concern | Current authority |
| --- | --- |
| managed nodes, identity, metadata envelope, attributes, resources, and annotations | architecture 14; specifications 02, 04, and 15 |
| navigation, editor, ribbon, contextual tabs, Inspector, format painter, tables, images, and standalone AsciiDoc mode | architectures 05, 06, and 20; specifications 06, 09, and 12 |
| actions, target scope, draft gates, Trash, recovery, and structural transactions | architectures 17 and 22; specifications 02, 08, 12, and 13 |
| native checklists, task nodes, promotion, task views, and Task Board | architectures 18 and 21; specifications 17 and 21 |
| Query, expressions, node/task/heading collections, filters, sorting, grouping, and aggregation | architectures 07 and 19; specification 18 |
| Template Library, placeholders, Designer, role transitions, and subtree instantiation | architecture 19; specification 19 |
| multidimensional-table records, field types, views, relations, and native-table upgrade | architecture 20; specification 20 |
| generic boards and source-specific actions | architecture 21; specification 21 |
| device-local history, Git-free compare/merge, backup targets/repositories, and restore | architecture 22; specifications 08 and 22 |
| links, Chrono, synchronization, citations, import, AI, Server, testing, and release | the corresponding numbered specifications and architectures |

## Architecture decisions

- [`architecture/01-runtime-architecture.md`](architecture/01-runtime-architecture.md): Desktop, WebUI, Server, CLI, Core, and backend boundaries.
- [`architecture/05-shared-navigation-information-architecture.md`](architecture/05-shared-navigation-information-architecture.md): Explorer Hierarchy/Contents navigation and presentation state.
- [`architecture/06-content-io-and-rich-rendering.md`](architecture/06-content-io-and-rich-rendering.md): source-preserving import/export, rich rendering, OCR, and enhancement boundaries.
- [`architecture/07-collections-query-and-views.md`](architecture/07-collections-query-and-views.md): Query-derived collections and dynamic views.
- [`architecture/14-canonical-document-metadata-and-review.md`](architecture/14-canonical-document-metadata-and-review.md): canonical source, identity envelope, attributes, typed data, and review state.
- [`architecture/15-content-intake-foundation.md`](architecture/15-content-intake-foundation.md): common intake probe, worker, IR, preview, transaction, and receipt boundary.
- [`architecture/16-pdf-import-and-ocr.md`](architecture/16-pdf-import-and-ocr.md): selected PDF/OCR package and proof gates.
- [`architecture/17-workspace-trash-item-store.md`](architecture/17-workspace-trash-item-store.md): item-backed Workspace Trash, restore, synchronization, and backup.
- [`architecture/18-task-nodes-and-checklist-promotion.md`](architecture/18-task-nodes-and-checklist-promotion.md): native checklist/task-node model and promotion transaction.
- [`architecture/19-expression-query-and-template-library.md`](architecture/19-expression-query-and-template-library.md): shared expression language, canonical Query, and Template Library.
- [`architecture/20-multidimensional-tables-and-editor-surfaces.md`](architecture/20-multidimensional-tables-and-editor-surfaces.md): multidimensional tables and editor information architecture.
- [`architecture/21-board-views.md`](architecture/21-board-views.md): generic boards, Task Board preset, typed actions, and accessibility.
- [`architecture/22-document-history-comparison-and-backup-repositories.md`](architecture/22-document-history-comparison-and-backup-repositories.md): history, comparison, backup repositories, and restore.
- [`architecture/dependencies.md`](architecture/dependencies.md): dependency policy.

## Specifications

Specifications are numbered by contract area; numbers remain stable even when a document is removed or consolidated.

1. [`01-product-boundary.md`](specifications/01-product-boundary.md)
2. [`02-node-storage.md`](specifications/02-node-storage.md)
3. [`03-chrono.md`](specifications/03-chrono.md)
4. [`04-annotations.md`](specifications/04-annotations.md)
5. [`05-sync-and-index.md`](specifications/05-sync-and-index.md)
6. [`06-application-ui.md`](specifications/06-application-ui.md)
7. [`07-server-collaboration.md`](specifications/07-server-collaboration.md)
8. [`08-data-safety-backup.md`](specifications/08-data-safety-backup.md)
9. [`09-testing-release.md`](specifications/09-testing-release.md)
10. [`10-links-and-potential-mentions.md`](specifications/10-links-and-potential-mentions.md)
11. [`11-ai-agent-integration.md`](specifications/11-ai-agent-integration.md)
12. [`12-document-actions.md`](specifications/12-document-actions.md)
13. [`13-workspace-transactions.md`](specifications/13-workspace-transactions.md)
15. [`15-weftext-asciidoc-profile.md`](specifications/15-weftext-asciidoc-profile.md)
16. [`16-citations-and-bibliography.md`](specifications/16-citations-and-bibliography.md)
17. [`17-tasks-and-query.md`](specifications/17-tasks-and-query.md)
18. [`18-canonical-query-and-expression.md`](specifications/18-canonical-query-and-expression.md)
19. [`19-node-template-library.md`](specifications/19-node-template-library.md)
20. [`20-multidimensional-tables-and-editor-surfaces.md`](specifications/20-multidimensional-tables-and-editor-surfaces.md)
21. [`21-board-views.md`](specifications/21-board-views.md)
22. [`22-document-history-comparison-and-backup-repositories.md`](specifications/22-document-history-comparison-and-backup-repositories.md)

The root [`ROADMAP.md`](../ROADMAP.md) is the concise public implementation-status view. A newer normative specification or architecture decision is the authority for target behavior; code and fixtures are evidence of the implemented subset, not a second public contract.
