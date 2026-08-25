---
source_language: zh-CN
translation_of: 05-shared-navigation-information-architecture.zh-CN.md
translation_status: synced
---

[简体中文](05-shared-navigation-information-architecture.zh-CN.md)

# Shared navigation information architecture

Desktop and Server WebUI use one navigation grammar. The activity bar selects Explorer, Search, or Chrono; recovery, account, and settings are utilities. Explorer provides **Hierarchy** and **Contents**. The center holds the active managed document in Write, Source, or Read, and the inspector holds Outline, Properties, Annotations, Backlinks, and authorized permission detail.

## Projections and opening

Hierarchy is the complete managed-node tree. Contents is a single-location list of immediate managed children plus Core-visible unmanaged directories, unmanaged Markdown, and resources. The canonical `X/X.adoc` is represented only by its node row. Ignored content is never projected.

Opening a managed row opens its UUID. Opening an unmanaged directory changes only the Contents location; it does not create a node tab. Unmanaged Markdown and resources are read-only inventory rows until a separately specified shared action permits more. They never acquire node properties, Chrono, Trash, ranks, annotations, backlinks, or revision-checked node editing merely because they are visible.

## Authority and action targets

Hierarchy and Contents consume one Core inventory and Core-derived managed-child order. Clients may group visible classes but cannot independently reconstruct rank or classify filesystem paths. Unmanaged rows never participate in node ordering.

One shared resolver captures an immutable typed target at activation: the focused-pane node for a current-node command, or the explicit row UUID/item ID for a row command. Planning, confirmation, and commit use that captured value. Later focus, selection, refresh, or virtualization cannot retarget an operation. Unavailable, unauthorized, or stale targets fail visibly.

## State, authorization, and accessibility

Explorer mode, expansion, scrolling, filtering, width, and unmanaged browsing locator are device-local presentation state. Node selection and document history are UUID based. With split panes, the focused pane defines the current node; navigation must preserve drafts and respect revision checks before replacing a pane.

Server filtering happens before rows, counts, breadcrumbs, search results, recents, or events reach the browser. Events are invalidation hints, not authority. Navigation is keyboard complete, has named landmarks and correct tree/list semantics, and supports screen readers, high contrast, zoom, reduced motion, RTL/mixed text, long names, narrow sidebars, and large-workspace virtualization.
