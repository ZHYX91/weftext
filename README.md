---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext / 文缕

> [!WARNING]
> **This is a personal toy project for learning and experimentation.** It has not reached a standard suitable for practical use. Do not use it for real work, important data, or production environments. Assume that it may corrupt or lose data. The project makes no commitment to stability, security, compatibility, continued maintenance, or support.

Weftext is an experimental knowledge-workspace codebase written primarily in Rust and designed around local file storage. Its design includes directory nodes, exact source text, a shared Core, Windows Desktop, a Server with browser WebUI, a CLI, and a standalone editor for explicitly opened external AsciiDoc files.

## Format and authority

Every managed node is a directory containing a same-named AsciiDoc document:

```text
Project/
├─ Project.adoc
├─ image.png
└─ Child/
   └─ Child.adoc
```

The root `.weftext-format` contains exactly `weftext.asciidoc.v1\n`. A missing, unknown, or malformed marker fails closed or enters an explicit adoption/import flow. File extensions never select workspace authority.

Each node has a persistent UUID in `weftext.id`. Paths are locators, not identity. Portable non-authorial comments, highlights, review marks, and suggestions live in the optional node-local `weftext.annotations.json` sidecar. Search indexes, backlinks, graphs, thumbnails, boards, collections, and statistics are derived and rebuildable; no synchronized database is content authority.

Native AsciiDoc checklist items are lightweight, identity-free occurrences. An item that needs durable identity or typed fields is explicitly promoted into an ordinary managed task node; the node's existing UUID is the task identity and the original position becomes a stable `node:` link.

Markdown is supported as explicit import/export input, visible unmanaged content, or an ordinary node-owned attachment. It is not a managed-node language or a standalone editor mode. Markdown import supports baseline syntax and may recognize selected extensions through bounded, explicitly versioned compatibility profiles.

## Product surfaces

- `crates/weftext-core`: node, identity, content, transaction, recovery, search, annotation, Chrono, Query, and derived-projection authority.
- `apps/desktop`: Windows Desktop application, shared React UI, native workspace selection, device-local draft recovery, Safe Mode, and direct Core commands.
- `crates/weftext-server`: hosted-workspace and authenticated API foundation with a same-origin browser client; it is not yet deployment-ready multi-user software.
- `crates/weftext-cli`: headless access to the same Core actions.
- `crates/weftext-agent*`: supervised, capability-bounded agent integration; agents never receive direct workspace write authority.
- `docs/specifications`: normative public format and product contracts.
- `docs/architecture`: current public architecture decisions.
- `docs/guides`: task-oriented reader guides that distinguish current foundations from designs still being implemented.
- [Public documentation terminology](docs/TERMINOLOGY.md): shared terminology and writing rules.
- `ROADMAP.md`: concise public status and release direction.

Managed-node editing provides source-preserving visual commands, a ribbon organized by task, contextual Table and Image tabs, a contextual Inspector, format painter, structural table operations, templates, Query-backed collections, tasks, boards, multidimensional tables, history, comparison, and backup entry points as their underlying Core capabilities become available. Standalone AsciiDoc mode reuses safe single-file editing and rendering, but disables workspace-only identity, navigation, Query, Template Library, durable-task, citation-resolution, history, and backup authority.

## Current status

The implemented foundation provides exact revisions, recoverable workspace transactions, lossless document edits, storage classification, search, structured document models, node resources, annotations, Chrono actions, shared navigation, a Windows Desktop alpha, and a loopback Server foundation. The complete editor, hosted authorization model, import/export pipeline, collaboration system, packaged accessibility matrix, and public release gates are not complete. See the [public roadmap](ROADMAP.md).

## Build and local development

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

On Windows, `apps/desktop/build-windows.cmd` builds the local UI and NSIS installer after the Microsoft C++ Build Tools are available.

The development Server may be run only on loopback:

```text
cargo run -p weftext-server -- <workspace> --bind 127.0.0.1:8787
```

It reports `deploymentReady=false`; do not reverse proxy or expose it as an intranet service.
