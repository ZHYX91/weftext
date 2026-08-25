---
source_language: zh-CN
translation_of: 17-workspace-trash-item-store.zh-CN.md
translation_status: synced
---

[简体中文](17-workspace-trash-item-store.zh-CN.md)

# Workspace Trash item-store architecture

Trash is a hidden Core-owned special node with a closed item store:

```text
.weftext-trash/
├─ .weftext-trash.adoc
└─ _weftext.items/
   └─ <trashItemId>/
      ├─ _weftext.trash-item.json
      └─ payload/<original entry>
```

The store is neither a managed node, unmanaged content, ignored content, resource directory, nor derived state. Discovery, search, links, Chrono, tasks, and Query do not enter it; synchronization, backup, revision, verification, and recovery include every byte.

## Closed manifest and identity

Each item has a new lowercase UUIDv4 `trashItemId`; a deletion action has an `operationId`. The permanent node UUID remains inside a node payload and is inactive until restore. `_weftext.trash-item.json` has schema `weftext.trash-item/v1`, unique closed keys, exact kind-specific origin fields, and digest/length evidence. Unknown origin is explicit and never inferred from a basename or path.

Node payload digests bind entry names, directory/file kinds, lengths, empty directories, and file bytes. Resource payloads contain exactly one regular node-owned file. An active/trashed duplicate node UUID, incomplete payload, malformed manifest, collision, or tampering is reconciliation evidence and fails closed; Core never rekeys, merges, overwrites, or guesses.

## Planning, restore, and deletion

Plans bind workspace revision, exact source/destination inventory, item IDs, manifest bytes, payload digest, and journal steps. Commit is no-clobber, durable, journaled, and recoverable: crash recovery reaches complete pre-state or complete item state, never reported partial success.

Restore plans select an exact item ID. `original` requires the recorded active parent/owner and a free name; `with-ancestors` restores a unique complete item chain atomically; `existing-target` requires an explicitly selected active target and explicit new name on conflict. Unknown origin always uses `existing-target`. Permanent deletion is a separate high-permission, confirmation-bound plan and Trash is never backup.

## Synchronization and migration

Partial arrival, conflict copies, duplicate IDs, malformed bytes, and simultaneous restore/delete are typed reconciliation cases. Full backup includes the complete store. Older layouts enter a one-time migration inventory that creates closed items with explicit unknown origin where evidence cannot prove it; there is no dual runtime authority.
