---
source_language: zh-CN
translation_of: dependencies.zh-CN.md
translation_status: synced
---

[简体中文](dependencies.zh-CN.md)

# Active dependency policy

Weftext pins its resolved Rust and JavaScript graphs. Shipped dependencies must meet the workspace Rust baseline and supported-platform requirements, appear in notices and SBOMs, and pass license, security, size, offline, and update/rollback review. No dependency becomes Core persistence or transaction authority.

## Core and source boundaries

Storage/transaction support libraries provide serialization, hashing, UUID, staging, and filesystem-object evidence only. Core owns format markers, rules, names, identity, revisions, narrow exact-source patches, journals, recovery, and inventory. Raw filesystem identities are never serialized.

System metadata uses a narrow fail-closed `weftext` envelope reader/writer; a YAML parser may support bounded read analysis only and never general top-level YAML or whole-frontmatter reserialization. The native `weftext-asciidoc` crate and pinned parser crates own exact-source modeling, safe derived rendering, diagnostics, occurrences, and syntax edits; they have no workspace traversal, Server, UI, import, or unrestricted processor authority. A secure-mode comparison renderer is not canonical authority.

## Surface and service boundaries

Markdown libraries remain import/export or repository-tooling dependencies. Import/OCR, office, and ebook components live behind workers. Query timezone validation is offline and deterministic; it never reads host timezone, environment, filesystem zoneinfo, network, or ambient clock. Citation presentation exposes only allowlisted styles/locales and no caller-supplied path, URL, plugin, processor, workspace handle, or network client.

Desktop UI dependencies provide bundled UI rendering and system-mediated folder selection, not raw filesystem access. Editor dependencies are exact-buffer surfaces only; semantic ranges and edits come from Core. Server HTTP/runtime/database/crypto dependencies support service operations; its database is authority only for accounts, sessions, ACL, audit, and coordination, never portable workspace or Query authority. Exact resolved versions live in lockfiles; release notices and SBOMs derive from packaged artifacts.
