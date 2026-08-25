---
source_language: zh-CN
translation_of: 01-workspaces-nodes-and-files.zh-CN.md
translation_status: synced
---

[简体中文](01-workspaces-nodes-and-files.zh-CN.md)

# Workspaces, nodes, and files

## Two concepts to learn first

A **workspace** is a folder selected by the user and managed by Weftext. A **node** is a directory inside that workspace with a same-named AsciiDoc document. For example, `Project/Project.adoc` is the body of the Project node.

Every node has a persistent UUID stored in `weftext.id`. Renaming or moving a node changes its path but not its identity. Links, task relationships, and other durable references therefore do not need to treat the path as identity.

## Current foundation

The `.weftext-format` file at the workspace root explicitly selects the Weftext AsciiDoc format. If the marker is missing, unknown, or damaged, Weftext refuses writes instead of guessing the format. Workspace creation, adoption, or import must be explicitly initiated by the user; this guide does not treat an unfinished guidance interface as a current capability.

Other ordinary files in a node directory are attachments owned by that node, including images, PDFs, office documents, `.txt`, and `.md` files. A Markdown attachment is not parsed as node content; the user can open it, download it, or explicitly import it as a new AsciiDoc node.

Search indexes, backlinks, graphs, collections, and statistics are derived from workspace content. Deleting these caches does not delete notes; Weftext can rebuild them.

## Content boundaries

Weftext classifies content into three categories:

- **Managed nodes** participate in identity, links, search, Query, and transactions.
- **Visible unmanaged content** can appear in file views but receives no node identity and is not rewritten as a node.
- **Ignored content** stays out of product navigation and derived views, although a complete backup must still handle its physical bytes according to the backup contract.

An unmanaged directory is a complete boundary. Weftext does not descend into it to discover node-like directories and does not adopt a file merely because it has an `.adoc` extension.

## Opening a file outside a workspace

**Accepted design:** the desktop application can explicitly open one external `.adoc` or `.asciidoc` file as a standalone AsciiDoc editor. It does not scan the parent directory, create a node UUID, write workspace metadata, or automatically turn adjacent files into attachments.

Standalone mode provides safe single-file editing, rendering, and document-local features. Node links, workspace Query, the Template Library, task nodes, workspace history, and formal backup are shown as unavailable with an explanation.

## Pre-release limitations

Weftext remains pre-release. A Windows Desktop Alpha, CLI, shared interface, and loopback-only Server foundation exist, but the complete editor and a deployable multi-user server are not finished. A synchronized folder is also not a real-time collaboration server.

See the implementation contracts for the [product boundary](../specifications/01-product-boundary.md), [node storage](../specifications/02-node-storage.md), and [application UI](../specifications/06-application-ui.md).
