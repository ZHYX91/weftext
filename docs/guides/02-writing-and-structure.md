---
source_language: zh-CN
translation_of: 02-writing-and-structure.zh-CN.md
translation_status: synced
---

[简体中文](02-writing-and-structure.zh-CN.md)

# Writing and document structure

## Three document views

Weftext provides three views over the same exact AsciiDoc source text:

- **Write** edits content and structure through visual commands.
- **Source** shows and edits the exact source text.
- **Read** provides safe, non-editable rendering.

Switching views does not create a second document. Every change must use a revision-bound exact source operation produced by Core; the interface does not save by concatenating markup or modifying rendered HTML.

## Ribbon and alternate entry points

**Accepted design:** the top ribbon contains six persistent tabs: Home, Insert, Data, References, Review, and View. Contextual tabs for Table, Image, Dynamic View, Task, and Template Design appear only when the matching content is selected.

The same action can also appear in a context menu, selection bubble, slash menu, command palette, or keyboard shortcut. They share one action definition, so right-click is never the only route.

The format painter copies semantic formatting, not CSS, HTML, or arbitrary source. Click to apply once, double-click for repeated use, and press `Escape` to cancel. It never copies text, link targets, node identity, task state, annotations, or resource paths.

## Headings and the document title

The document title and optional subtitle belong to the document header; they are not body heading levels. The body uses an H1–H9 selector. When changing a heading level, the user can change only that heading or preserve the relative hierarchy of its subtree where valid. An operation that may reorganize the outline shows a preview first.

In Query, `this.heading` means only the nearest body heading that owns the current source position. It is null when the document has only a title, the cursor is in the preamble, or the position belongs to no body heading; it never falls back to the document title.

## Tables and images

A native table supports assigning the first N rows or columns as headers, rectangular merge, split, row and column insertion or deletion, and captions. Before merging multiple non-empty cells, Weftext must preview how their content will be combined so data is not silently discarded.

An image is first imported safely as a node attachment and then referenced by canonical source. A failed import cannot leave half-written source or an orphaned resource. Standalone file mode can use an author-selected relative path but does not gain node-resource ownership.

## Current status

**Current foundation:** exact revisions, source-preserving edit plans, the Write/Source/Read model, shared navigation, and a Windows Desktop Alpha have implementation foundations.

**Pre-release limitation:** the complete ribbon, every formatting command, table structure editing, image workflows, format painter, accessibility, and IME acceptance still need to be completed in packaged applications. A visible design entry point does not mean the capability has passed release acceptance.

See the detailed contracts for the [application UI](../specifications/06-application-ui.md), [document actions](../specifications/12-document-actions.md), and [AsciiDoc profile](../specifications/15-weftext-asciidoc-profile.md).
