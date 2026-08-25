---
source_language: zh-CN
translation_of: AGENTS.zh-CN.md
translation_status: synced
---

[简体中文](AGENTS.zh-CN.md)

# Weftext repository rules

## Product boundary

- Weftext is implemented natively in Rust.
- Production document conversion uses Weftext-owned import infrastructure and does not depend on another product's metadata, adapter, manifest, wire contract, or release.
- Public repository documentation describes current Weftext contracts. Product comparisons, screenshots, research notes, schedules, handoffs, decision history, and acceptance logs stay outside this repository.
- Human-facing public Markdown is maintained as a Chinese source and synchronized English translation. Test fixtures, generated evidence, lockfiles, and machine-readable samples are not localized.

## Storage authority

- The product has one managed-node shape: `X/X.adoc`, selected by exact root marker bytes `weftext.asciidoc.v1\n` in `.weftext-format`. Markdown is explicit import/export, visible unmanaged content, or a node-owned attachment, never a peer managed generation.
- Root-level `.weftext-rules` may classify ordinary paths as visible unmanaged or ignored content. Core is the sole inventory authority; shells must not rediscover nodes independently.
- An unmanaged directory is a complete subtree boundary. It has no UUID or `weftext` envelope, and scanning never re-enters it. A managed node's same-named AsciiDoc document cannot be classified separately.
- Ordinary non-canonical files are resources. An owner UUID is derived only when the containing boundary is a managed node.
- Node identity is a lowercase UUIDv4 in `weftext.id`. Paths are locators. Parentage comes from the directory tree; do not persist parent, path, name, or child-path lists.
- Default child order is natural name order. A parent owns sorting policy; a child stores only its sparse manual rank.
- Portable node annotations use `weftext.annotations.json`; do not add a metadata directory inside a node.
- Native AsciiDoc checklist items are identity-free and revision-scoped. A durable task is an ordinary managed node with the closed `weftext-task` header profile and the node's existing UUID. Do not introduce another task identity namespace or task database.
- Search, backlinks, graph, thumbnails, collections, boards, and statistics are derived and rebuildable. No synchronized SQLite database is content authority.
- `.weftext-rules` is portable workspace authority and is never inherited from `.gitignore`. Ignored bytes remain backup content while being excluded from product inventory and projections.
- Reserved names are role-based: root controls use `.weftext-*`; closed store internals may use `_weftext.*`; portable node-local sidecars use `weftext.*`.

## Editing and safety

- Structural changes use Core plans and transactions, not UI-specific file operations.
- Core rejects targets inside ignored or unmanaged boundaries and rejects structural changes to a managed subtree that contains such a boundary.
- During partial synchronization, missing metadata is an incomplete state and never permission to regenerate identity.
- Preserve exact AsciiDoc/YAML formatting. System-envelope patches are narrow and fail closed on ambiguous YAML.
- Read the current specifications before changing a persistent format or domain rule. Update specifications and tests in the same change.

## Product surfaces

- Desktop, WebUI, Server, CLI, and approved agent callers consume the same Core actions and transactions.
- WebUI supports ordinary intranet use as well as administration.
- Server clients never write hosted workspaces directly; authentication, ACL, revision, non-disclosure, and audit are enforced at the Server boundary.
- A synchronized folder is not a collaboration server. Real-time multi-user editing requires Weftext Server.
- Status reports state the exact implemented surface and remaining gaps; a Core primitive alone is not a completed product feature.

## Engineering

- Core behavior is headlessly testable and shared by present and future shells.
- Unsafe Rust is forbidden.
- Add dependencies only when the standard library and existing dependencies are insufficient; document license and portability impact.
- For active code, run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
