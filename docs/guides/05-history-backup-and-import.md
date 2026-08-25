---
source_language: zh-CN
translation_of: 05-history-backup-and-import.zh-CN.md
translation_status: synced
---

[简体中文](05-history-backup-and-import.zh-CN.md)

# History, backup, and import

## Recent History is not formal backup

**Accepted design:** Recent History is stored by default in the current device's Weftext application-data location and supports viewing and comparing recent document versions. It is not inside the note workspace and does not automatically follow workspace synchronization to another device. The interface labels it “This device only.”

Formal backup stores verifiable complete workspace snapshots. It covers managed nodes, attachments, unmanaged and ignored content, Trash items, templates, and required multidimensional-table files. Drafts, Trash, transaction logs, a synchronization provider's version history, or an ordinary same-disk copy cannot automatically be called formal backup.

## Defaults and backup locations

A new workspace has no formal backup target by default. The interface must show “Backup: Not configured” and “Recent History: This device only” instead of implying a false safe state.

The user can configure a local directory, external device, network location, supported remote backend, or a target managed by an approved server. The target and the device that runs backup are independent choices: any approved device can execute backup under the same repository contract, but only one write lease is allowed at a time.

A backup on the same disk can help recover accidental deletion but cannot protect against device failure. A copy inside a synchronization tree should not be the only formal backup.

## Comparison and restore without Git

Weftext comparison does not depend on Git. A user can compare the current document with its previous version, a history entry, another document, a backup snapshot, or an external-conflict draft.

Comparison can use inline or side-by-side views and show structural context. Choosing “Use left,” “Use right,” or “Keep both” first creates a reviewable result draft; final application remains one Core transaction bound to the current revision.

Restoring an old document version restores only that document's source text by default. Restoring attachments, child nodes, templates, multidimensional tables, or a complete workspace requires the complete formal-backup restore flow and begins with a no-write dry run.

## Importing external formats

Markdown can be an explicit import input. The importer handles the base syntax, and a bounded, explicitly versioned compatibility profile can recognize selected extensions. This does not make Markdown a second managed node language.

Real-format import follows one pipeline: detection, a constrained worker process, intermediate representation, preview, transaction, and receipt. PDF is the first planned complete conversion flow and can use local OCR with user authorization; import failure does not directly modify the workspace.

## Current status

**Current foundation:** Weftext has implementation foundations for exact revisions, transaction recovery, draft recovery, Trash, and backup safety boundaries.

**Pre-release limitation:** the unified Version History interface, cross-document comparison, formal backup repositories, complete restore drills, and production PDF import are still being implemented and accepted. Without a configured backup, users should not treat Recent History or a synchronization service as a complete recovery guarantee.

See the detailed contracts for [data safety and backup](../specifications/08-data-safety-backup.md), [history, comparison, and backup repositories](../specifications/22-document-history-comparison-and-backup-repositories.md), and the [content intake architecture](../architecture/15-content-intake-foundation.md).
