---
source_language: zh-CN
translation_of: ROADMAP.zh-CN.md
translation_status: synced
---

[简体中文](ROADMAP.zh-CN.md)

# Weftext public roadmap

Weftext is pre-release software. This document states the current public foundation, accepted product direction, and release gaps. Internal schedules, research, decision history, and acceptance logs remain outside the public source repository.

## Current foundation

- Rust Core owns managed-node identity, content classification, exact revisions, source-preserving edits, recoverable workspace transactions, search/index projections, links, annotations, Chrono, Query plans, and shared navigation.
- Managed workspaces use one format: `.weftext-format` selects `weftext.asciidoc.v1`; nodes use `X/X.adoc`; system YAML has one top-level `weftext` mapping; portable review state uses `weftext.annotations.json`; `.weftext-rules` classifies visible unmanaged or ignored content.
- The item-backed Workspace Trash uses `.weftext-trash/_weftext.items/<trashItemId>`, closed manifests, no-clobber transactions, UUID-based restore, explicit permanent deletion, recovery journals, synchronization diagnostics, and physical backup coverage.
- Rename, Move, Copy, Trash, Restore, and Chrono mutations use frozen typed targets, exact scope, revision/draft gates, recoverable journals, and receipts through the shared Core boundary.
- Canonical Query uses one `[.weftext-query,version=1,view=...]` literal-block surface, explicit `from` aliases, typed scopes, lexical `this`, explicit time/locale context, stable output identity, permission filtering, and resource bounds. The implemented expression subset is narrower than the complete accepted `weftext.expr.v1` language.
- Windows Desktop, CLI, the loopback Server foundation, shared React UI, and same-origin browser UI consume the same Core authority. The Server is not deployment-ready.

## Accepted product direction

- Keep the Weftext AsciiDoc Profile as the sole canonical managed-source language. Keep Markdown at explicit import/export, visible unmanaged content, or node-owned attachment boundaries.
- Provide a source-preserving managed-node editor with ribbon tabs, contextual Table and Image tabs, Inspector, format painter, heading/table structural commands, accessible keyboard equivalents, and exact-source fallback.
- Provide Standalone AsciiDoc mode for one explicitly opened external `.adoc` or `.asciidoc` file. It never scans or adopts the containing directory and withholds every workspace-only capability.
- Keep native checklists identity-free. Promote an item that needs durable identity or typed fields into an ordinary task node in one recoverable transaction; use the node UUID as task identity and leave a stable node link at the source position.
- Implement Template Library as one configured special managed subtree. Templates remain outside ordinary search, Query, task, graph, and timeline projections and instantiate complete validated subtrees through one previewed Core transaction.
- Standardize Query predicates and template bindings on pure, deterministic, bounded `weftext.expr.v1`, while keeping Query clauses and template placeholder syntax distinct.
- Support Query-backed node/task/heading collections with visual filter, sort, group, aggregation, table, calendar, timeline, and board configuration. The UI edits canonical Query source rather than creating a hidden parallel model.
- Store each multidimensional table as one managed table node with one bounded JSON record file per row. Records are not nodes and cannot be promoted into nodes; relations and rich-text/image values use typed JSON forms and table-level transactions.
- Provide a common board shell for typed sources. A Task Board is one preset, not a separate authority; every drag, creation, and edit maps to a source-specific validated action.
- Separate device-local recent history from durable backup repositories. Let users configure local or remote backup targets and executors; provide Git-free document comparison, merge, and restore through Core plans.
- Build one common true-format intake pipeline. PDF is the first planned conversion slice; all imports use probe, bounded worker, normalized IR, preview, transaction, and receipt boundaries.
- Continue toward supervised agent actions and authenticated collaboration without granting agents, browsers, or UI shells direct filesystem write authority.

## Release gaps

- complete the production AsciiDoc editor and all ribbon/contextual commands across signed packaged GUI surfaces;
- finish task-node promotion, the complete `weftext.expr.v1` evaluator, Query product surfaces, Template Library engine/designer/instantiation, multidimensional-table editing, boards, history, comparison, and backup repositories;
- finish production PDF and other document import/export paths;
- complete accounts, roles, subtree ACL, non-disclosure, durable audit, deployment, backup drills, and real-time collaboration for Server;
- complete accessibility, IME, Windows 10/11, crash-recovery, long-document, large-workspace, signing, update, SBOM, and clean-install acceptance;
- publish no release until the applicable gates in the testing and release specification pass.

Stable contracts live in [`docs/specifications`](docs/specifications), and current architecture decisions live in [`docs/architecture`](docs/architecture).
