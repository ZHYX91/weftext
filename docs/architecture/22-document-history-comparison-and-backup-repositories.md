---
source_language: zh-CN
translation_of: 22-document-history-comparison-and-backup-repositories.zh-CN.md
translation_status: synced
---

[简体中文](22-document-history-comparison-and-backup-repositories.zh-CN.md)

# Document history, comparison, and backup repositories

Chrono, timeline views, Version History, transaction journals, drafts, and Trash are distinct. The workspace contains only current portable authority. Version history and backup remain outside it so saving does not create synchronized hidden history or unbounded workspace growth.

## History tiers

**Recent History** is a bounded device-local content-addressed store of verified canonical-document commits. It records exact pre/result source objects and closed entry evidence, is explicitly labeled device-only, and may be disabled/cleared. A successful workspace commit with failed history capture remains successful and reports a visible gap; it never retries the document write.

**Backup Repository** is a user-configured local, network, Server-managed, or explicitly supported remote destination containing immutable full-workspace snapshot manifests and deduplicated objects. Without a configured destination, formal backup is not configured. Trash and recent history are not backup.

## Repository and execution boundary

One repository has one explicit workspace lineage and a single write executor unless a supported backend serializes conditional writes/leases. Credentials remain in OS/Server operational storage, never workspace or repository manifests. A repository uses closed versioned manifests, immutable no-clobber objects, complete physical inventory, verified publication, protected restore points, recoverable retention, and reachability-based garbage collection; derived catalog databases are rebuildable.

Full snapshots include managed documents/resources/annotations, table and template authority, visible unmanaged and ignored physical bytes, complete Trash, root controls, empty directories, and recovery-relevant authority. Hosted backup coordinates the disjoint Server control plane at one consistency point.

## Compare, merge, and restore

Version History is a permission-filtered derived union of device, Backup, Server, and protected-point entries, each with visible provenance. Node UUID preserves history through rename/move. Core comparison is Git-independent and exact-source/AsciiDoc-aware. It compares authorized immutable revisions/digests, distinguishes structural/resource/annotation differences, and never normalizes source or guesses a three-way base.

Compare is read-only. Applying changes or restoring document content uses a revision-bound Core merge/restore plan that binds target revision, input digests, selected dispositions, proposed source, diagnostics, authorization, and draft gate, then commits once as a new event. Complete node/subtree/workspace restore uses backup dry-run, no-overwrite, and recovery plans. History content is excluded from ordinary search, Query, links, graph, Chrono, tasks, and agent context until an explicit authorized history action.
