---
source_language: zh-CN
translation_of: 01-runtime-architecture.zh-CN.md
translation_status: synced
---

[简体中文](01-runtime-architecture.zh-CN.md)

# Runtime architecture

Weftext is an experimental codebase written primarily in Rust, with one Core and several clients. Its only managed-document format is the Weftext AsciiDoc Profile, using the `weftext` metadata envelope and `weftext.annotations.json`. Markdown is limited to import, export, and visible unmanaged content. See [canonical metadata](14-canonical-document-metadata-and-review.md).

## One product, multiple clients

```text
Desktop local mode -> Weftext Core <- Server
CLI                -> Weftext Core <- WebUI / remote Desktop / CLI administration
```

Core is the only authority for node identity, profile selection, actions, plans, transactions, validation, and conflict semantics. The AsciiDoc profile crate provides exact-source models, diagnostics, protected regions, and syntax edits; it does not own workspace mutation. Desktop, WebUI, Server, CLI, and agents are callers and must not create alternative parsing, filesystem, or authorization rules.

## Component boundaries

- `weftext-core` owns domain types, plans, transactions, and backend-neutral rules.
- `weftext-asciidoc` owns the exact-source profile model and syntax operations only.
- `weftext-cli`, `weftext-desktop`, `weftext-ui`, and `weftext-server` are Core callers.
- The server owns authenticated API access, ACL, audit, collaboration coordination, and its non-portable control plane.
- The agent boundary exposes scoped reads and reviewed typed actions; it never exposes raw workspace writes or a shell.

## Local and hosted modes

Local Desktop and CLI call Core against a filesystem workspace. Portable source, resources, and sidecars are authority; indexes and UI state are rebuildable.

Hosted clients call the authenticated Server API and never write the hosted workspace directly. The Server authorizes the actor, checks the base revision, invokes Core, commits through the hosted backend, and publishes the result. Accounts, ACL, sessions, audit, and presence are control-plane state, not document frontmatter. Folder synchronization is useful for one user's devices but is not multi-user collaboration; collaborative editing goes through Weftext Server.

## Shared UI contract

Desktop and browser clients share document/node view models, action identifiers and previews, editor and annotation behavior, localization/accessibility behavior, and protocol conformance tests. Desktop-only lifecycle, folder-picker, operating-system, credential, and update functions stay in the desktop shell. No UI framework is persistent-format authority.

## Agent and transaction boundary

An agent session receives only selected node context, document reads, search, action descriptions, previews, approvals, outcomes, diagnostics, cancellation, and events. In hosted mode an agent is an authorized Server client; it never receives direct hosted-directory access.

Every mutation binds actor, origin, typed target, base revision, deterministic plan, validation, and commit result. Structural edits use recoverable workspace transactions; document edits use revision-checked document transactions. Durable collaboration state materializes into canonical source/resources/annotations, while presence remains ephemeral.
