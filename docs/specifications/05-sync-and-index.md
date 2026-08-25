---
source_language: zh-CN
translation_of: 05-sync-and-index.zh-CN.md
translation_status: synced
---

[简体中文](05-sync-and-index.zh-CN.md)

# Sync and derived index

Status: current architecture contract.

Cloud folders copy files; they do not provide transactional collaboration. Portable authority consists of `.weftext-format`, `.weftext-rules` when present, managed node directories and AsciiDoc, visible unmanaged content, ignored physical content, resources, annotation sidecars, valid node-local table sidecars/record stores, and every manifest/payload byte in `.weftext-trash/_weftext.items`. Search, outgoing links, backlinks, potential mentions, alias lookup, graph, table formula/results/reverse relations, thumbnails, watcher state, and caches are derived and stored locally outside the workspace.

Device Recent History and formal Backup Repositories are also outside the workspace, but they are not derived indexes. Recent History is device-local exact-source recovery evidence and never follows ordinary workspace synchronization. A Backup Repository is a separately selected disjoint local/remote target with its own immutable object/snapshot authority and executor; the workspace scanner and content rules never enter it. Another machine sees history only through its own local store, an explicitly attached read-only/authorized repository, or Weftext Server—not because current workspace files synchronized. See [`22-document-history-comparison-and-backup-repositories.md`](22-document-history-comparison-and-backup-repositories.md).

Multi-user editing goes through Weftext Server as specified in [`07-server-collaboration.md`](07-server-collaboration.md). Several independent clients writing one synchronized directory are an unsupported collaboration topology.

The Core scanner distinguishes:

- valid nodes;
- valid multidimensional-table profiles whose sidecar, record-store layout, record identities, values, and resource locators agree with their owning managed node;
- visible unmanaged directories, loose files including Markdown, and ordinary resources with root-relative `/` locators, parent locators, optional managed node UUID, and optional resource-owner UUID;
- ignored boundary matches, which are retained internally only as safety barriers and are not returned through ordinary product inventory;
- complete Trash items, which are closed manifest/payload authority retained for revision, sync, backup, restore, conflict, and migration but never returned as active managed nodes or ordinary files;
- incomplete nodes whose AsciiDoc or identity envelope may still be arriving;
- invalid identity;
- duplicate identity;
- incompatible loose files and unpaired content directories when no rule classifies them;
- conflict-copy candidates and external moves.

Missing or invalid identity never authorizes automatic regeneration during a scan. Adoption and repair are explicit preview/commit operations. A local index can be discarded and rebuilt from portable authority.

The scanner never re-enters node discovery below an unmanaged or ignored directory and never enters active-node discovery below `_weftext.items` or a valid `weftext.records` store. Content rules cannot classify either reserved store or its descendants. Links, backlinks, aliases, potential mentions, graph, Chrono and node ordering consume only active managed nodes. Table record relations consume the separately validated authorized table projection; canonical Query v1 does not discover a records domain. Ordinary file/resource views may consume visible unmanaged inventory and ordinary node-owned resources but not table sidecars/record files or Trash manifests/payloads. All public locators are root-relative `/` strings; absolute host paths remain inside the trusted local process.

The link and alias index is rebuilt from node basenames, `weftext.aliases`, and document content. It never becomes identity authority and never writes links merely because a candidate was discovered. Candidate ordering, overlap, ambiguity, and explicit link actions are defined in [`10-links-and-potential-mentions.md`](10-links-and-potential-mentions.md).

The content-search index is a versioned derived projection stored under the device configuration directory by workspace UUID, never inside the workspace. It indexes the active managed-node name, visible AsciiDoc body, and safe literal descriptive document-header attributes; it excludes the `weftext` envelope, processor-control attributes, unmanaged content, ignored content, table sidecars/record JSON, the Trash special node, every item manifest, and every payload node/resource. Table records use a separate disposable schema-aware projection keyed by owning table-node UUID, record UUID, and authoritative fingerprints. Resolved explicit or derived default icons may be copied into result presentation, but neither index becomes icon, field, record, formula, relation, or property authority. UUIDs and current relative paths remain scanner projections rather than an identity registry.

A table projection validates the complete current sidecar/record relation before accepting new authority. Partial sidecar/shard/file arrival, sidecar/record schema mismatch, filename/ID/shard mismatch, duplicate IDs, conflict copies, invalid values, a missing required sidecar or record-store root, or linked/reparse content produces a typed reconciliation issue. A previously verified projection may remain visible read-only and labeled stale, but it cannot authorize edits, summaries, exports, or relation suggestions. Formula results, filters, stable ordering/grouping, and summaries rebuild deterministically from authorized files plus explicit evaluation context. A successful record/schema transaction followed by projection failure remains a successful authority commit with a structured rebuild warning.

A synchronized Trash item becomes usable only when its directory name, closed manifest, complete payload inventory, permanent identities, lengths, and digests agree. Partial arrival, manifest/payload alteration, duplicate or case-fold-colliding `trashItemId`, conflict-copy filenames, duplicate active/trashed node UUIDs, or competing restore/delete state produces a typed Trash reconciliation issue. Core retains all physical evidence and blocks ordinary Trash mutation; it does not classify the conflict as ignored/unmanaged, select a winner by timestamp, rekey a permanent node, or repair a manifest from a path. A complete replica snapshot is the only basis for treating an absent item as deletion.

Opening or searching a workspace reconciles the index with a valid Core inventory. Unchanged document entries are reusable by UUID plus a file fingerprint, moves update the derived locator, and missing or trashed entries are removed. Known document commits and structural transactions additionally invalidate their affected UUIDs explicitly, so correctness does not depend on timestamp granularity and unrelated documents are not reparsed. A corrupt, absent, unsupported, or manually deleted index is rebuilt atomically from portable workspace authority. Resources and annotation sidecars remain portable authority for derived features that consume them, but the first content-search projection intentionally does not make their bytes searchable.

An authoritative document or workspace transaction succeeds or fails independently of derived-index maintenance. If the Core commit succeeds and the following index refresh fails, the caller receives the successful commit plus a structured index warning and marks the index for later rebuild. It must not report the whole request as failed or invite the user to retry a write that already happened. Search may report index-unavailable until a later refresh/rebuild succeeds.

The external-index boundary is checked against resolved filesystem locations, not lexical path strings alone. The implementation resolves the existing index target or its nearest existing ancestor, applies the unresolved suffix, and rejects any result inside the resolved workspace. `..`, symlinks, Windows junctions/reparse points, relative paths, and a not-yet-created filename beneath an aliased workspace directory cannot bypass this rule. An ambiguous or unresolvable safety check fails closed. Recent History and Backup Repository target validation applies the same resolved disjointness principle but retains separate schemas, retention, authorization, and mutation boundaries; neither may be placed inside the workspace and an index can never be promoted into either store.

Structural writes eventually require a recoverable multi-file journal with base revisions, prepare/commit/cleanup states, atomic replacement where supported, and startup recovery. Until that journal is available, operations without a safe rollback path remain plans rather than pretending to be durable transactions.
