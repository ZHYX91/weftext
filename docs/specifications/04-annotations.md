---
source_language: zh-CN
translation_of: 04-annotations.zh-CN.md
translation_status: synced
---
[简体中文](04-annotations.zh-CN.md)

# Node annotations and review

Status: accepted sidecar contract.

Annotations are portable review content, not authored document source, frontmatter properties, or Server-only records. A managed node creates one reserved sidecar only when needed:

```text
Document/
├── Document.adoc
└── weftext.annotations.json
```

The sidecar stores the owning `document_id` as a foreign-key check against `weftext.id`; the AsciiDoc document remains node identity authority. Only managed nodes can own this sidecar. Unmanaged files, ordinary resources, and unmanaged directories cannot acquire node annotations through this boundary.

## Annotation model

An annotation combines independently:

- a target: text range, insertion point, block, complete document, or reviewed resource region;
- a kind: comment, mark, suggestion-insert, suggestion-delete, or another versioned review action;
- an optional visual mark: highlight, underline, squiggle, or strikeout;
- an optional stable theme token: yellow, red, green, blue, purple, pink, or gray;
- labels, an optional comment thread, and open/resolved/orphaned status.

Color is presentation, not meaning. Meanings such as important, question, error, and verify are labels. An authorial underline or strikeout is ordinary AsciiDoc source; a review underline or strikeout is an overlay in this sidecar. Accepting an insertion/deletion suggestion invokes a normal Core document plan and transaction. Rejecting or resolving it changes only the sidecar.

Every annotation and message has its own lowercase UUIDv4. Rename, move, and cloud replication preserve them. Copying a node as a distinct node rekeys its annotations and messages together with `document_id`. Duplicate IDs and sync conflict copies require explicit reconciliation. An absent sidecar during partial synchronization is incomplete state and never authorizes creation of an empty replacement.

## Message bodies and actors

Every message stores exact constrained AsciiDoc inline source:

```json
{
  "id": "a27fe847-6cf7-48bb-9e85-f672d10461f7",
  "author_id": "70d407dd-8538-45da-bb3d-d2eb4baa8539",
  "author_name": "Zhang San",
  "created_at": "2026-08-24T10:20:00+08:00",
  "body": {
    "format": "weftext.asciidoc.inline.v1",
    "source": "Please verify this _claim_."
  }
}
```

`author_id` is a stable collaboration actor identifier; `author_name` is a display snapshot and never an account lookup key. Account credentials, email unless explicitly authored in the message, session IDs, role grants, presence, cursor state, and unsent drafts do not enter portable annotations. Edited/deleted message behavior and audit retention are schema-versioned; a destructive client may not silently erase history required by an accepted review policy.

The sidecar has one canonical inline-body language. Unknown sidecar or body versions fail closed.

## Anchors

Text anchors retain:

- the exact base document revision;
- UTF-8 source start/end offsets for that revision;
- the exact selected visible quote;
- bounded prefix and suffix context;
- containing block/section identity or path evidence when available.

Offsets are evidence, not timeless identity. After a document edit, Core reanchors exact block evidence and context only when one deterministic target exists. Ambiguous or missing targets become `orphaned`; they are never silently attached elsewhere or discarded. The UI maps browser string positions to Core UTF-8 positions but never authors its own persistent anchor rules.

Resource-region annotations additionally store the node-owned resource locator, resource digest, media kind, and a versioned coordinate/page/time target appropriate to that resource. A resource target never grants the annotation renderer filesystem or network access.

## Mutation and multi-user authority

Create, mark, reply, edit, resolve, reopen, reanchor, suggest, accept, and reject are typed Core actions. First creation uses a recoverable create-file journal step; later changes use sidecar revision checks, verified atomic replacement, and startup recovery. A successful document mutation and its associated annotation transition commit in one recoverable transaction when their consistency requires it.

Desktop and CLI use the same Core boundary. In hosted mode Weftext Server authenticates and authorizes the actor, serializes concurrent mutations of a node sidecar, rejects stale plans, publishes bounded events, and records required audit evidence. Browsers never write hosted files directly.

A hosted read first resolves the requested UUID through the actor's authorized node projection, then validates only that node document, its fixed sidecar, and same-directory annotation conflict copies. It must not open or require validity from a hidden ancestor or unrelated hidden node. This scoped read rule does not weaken mutation authority: every sidecar creation or change still uses the complete hosted-replica snapshot, workspace revision, Core plan, and recoverable transaction.

A synchronized folder copies the sidecar but is not a multi-user collaboration protocol. Server SQLite may store accounts, roles, sessions, audit events, presence, delivery queues, and derived unresolved-count/activity indexes. It must not become the only copy of portable comment, mark, suggestion, or thread bodies.

## Validation

The sidecar schema is closed and versioned. Unknown fields are preserved only where the schema explicitly provides forward-compatible extension storage; unknown required semantics otherwise fail closed. Limits cover file size, annotations, messages, body bytes, context bytes, labels, and resource targets.

Acceptance covers CJK/RTL/emoji, mixed line endings, overlapping annotations, concurrent replies, stale resolve/edit, actor removal, node copy/move/Trash/restore, partial sync, conflict copies, document edits that preserve/orphan anchors, suggestion acceptance rollback, Server authorization/non-disclosure, crash recovery, and exact backup/restore.
