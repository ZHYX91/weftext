---
source_language: zh-CN
translation_of: 08-data-safety-backup.zh-CN.md
translation_status: synced
---

[简体中文](08-data-safety-backup.zh-CN.md)

# Data safety, recovery, and backup

Status: current architecture contract.

## Crash-safe writes

Every canonical document, sidecar, reserved table-record file, resource, and structural mutation goes through a Core transaction. A durable commit must:

1. validate the target type, path boundary, base revision, and permissions;
2. write request-owned staging in a location with compatible atomic-replace semantics;
3. verify encoding, expected length, and content digest;
4. flush file data and required metadata using the strongest supported platform semantics;
5. atomically replace or execute a recoverable multi-file commit protocol;
6. retain enough journal evidence to finish or roll back after interruption;
7. reopen and verify the committed revision before declaring success.

The application detects unfinished transactions at startup. It does not continue overwriting an ambiguous workspace. Disk full, file locks, permission changes, process termination, external edits, antivirus interference, and sync-provider interference produce structured recovery states.

## External edits and conflicts

An external edit is compared with the base revision. If both sides changed, Weftext offers Core-owned Git-independent compare, three-way merge when the base is proven, keep local, keep external, or keep both as applicable. Last-writer-wins overwrite is not an acceptable default. Inputs, structural comparison, identity protection, merge drafts, and revision-bound apply are governed by [`22-document-history-comparison-and-backup-repositories.md`](22-document-history-comparison-and-backup-repositories.md).

Cloud conflict copies, remote-delete/local-edit, node identity conflicts, and rename/move conflicts enter a Conflict Center. Resolution itself is a transaction and must not be overwritten by autosave.

## Device-local drafts

Desktop crash-recovery drafts are versioned device-local records, not workspace content, synchronization state, transaction journals, or backup. A v2 record binds the workspace instance, node ID, canonical document profile, exact base revision, source, and update time. Records are written through request-owned temporary staging, flushed, atomically replaced, and reopened for verification. An interrupted replacement leaves the last verified record recoverable; corrupt or unsupported records remain visible as recovery issues instead of being silently deleted or regenerated.

Draft autosave never writes the selected workspace. On reopen, a draft whose profile and base revision still match the document may be restored for editing. If the disk revision changed, the application preserves both versions and requires an explicit compare-and-resolve choice before a Core commit. A verified document commit or explicit user discard removes the corresponding draft. Draft retention and cleanup remain device-local policy and do not change node identity.

Before Desktop authorizes a bulk import that could touch existing nodes, its native boundary inventories the complete device-draft scope. Any intersecting draft or unreadable/corrupt draft record blocks preflight until the user commits, explicitly discards, or otherwise resolves it. A CLI process cannot infer another device's private draft store; operators must close or coordinate other clients before a bulk mutation.

## Recent document history

Recent History is a separate bounded device-local exact-source store under the OS Weftext application-data location, never the workspace, portable synchronization, draft store, transaction journal, Trash, or formal backup repository. It records verified pre/result canonical-document sources for successful commits through the versioned content-addressed store in specification 22. It is enabled by default and labeled `This device only`; users may clear/disable it under an explicit retention/privacy control.

A workspace commit succeeds independently of history capture. If history publication fails after commit, the result remains successful with a structured warning and visible version gap. Recent History v1 does not promise complete resources, annotations, table records, Template state, ignored/unmanaged bytes, Trash, subtree, or Server recovery. Another device receives none of it through ordinary workspace sync. Comparing/restoring one source version remains a revision-bound Core operation, not direct copying from the history directory.

## Trash and restore

Ordinary deletion writes only the closed Workspace Trash item-store authority defined in [`../architecture/17-workspace-trash-item-store.md`](../architecture/17-workspace-trash-item-store.md). One deleted node root becomes one item containing its complete unchanged subtree. Every independently restorable resource becomes its own item, with batch items sharing an operation ID. A plan binds the workspace revision, generated item IDs, exact source inventory, manifest bytes, payload digests, and no-clobber destinations before any mutation. A crash or external edit cannot produce a reported partial batch, reused item ID, overwritten item, or guessed source.

Restore preserves permanent node identity and original resource bytes. It resolves the former parent/owner by UUID. If that UUID is active and the target is free, original restore is available. If the parent/owner has its own complete Trash item, an explicit preview may restore the required parent chain and selected item atomically. If the origin is missing or a migration item is marked `originStatus: unknown`, the item remains in Trash until the user chooses an existing target. Names and legacy paths are display evidence only: Weftext never fabricates a same-named parent, and exact or case-only conflicts require an explicit rename or alternate target rather than overwrite.

The accepted Template contract treats a Trash payload as closed inactive exact bytes, not as active ordinary source. Generic Trash refuses the active configured Template Library root; a combined reviewed transaction must clear or rebind `template_library_root` while trashing it. A Template Root/Part restored to its proven role under the active Library may retain exact bytes. Restore elsewhere requires explicit role conversion that materializes/deletes active slots and removes profile/sidecar or blocks. Restore never guesses or rebinds the Library configuration except through the combined operation's exact rollback receipt or a new configuration plan.

Malformed, incomplete, duplicate, tampered, or sync-conflicted items enter reconciliation and retain all evidence. Permanent deletion is a separate revision-bound, higher-permission plan with explicit item/digest/byte-total confirmation. Retention and cleanup policies are visible and testable and invoke that same action boundary. Trash is synchronization-carried deletion state, not backup.

Multidimensional-table records use a separate table-local Deleted Records lifecycle because they are not nodes or ordinary resources and cannot extend the closed Workspace Trash v1 manifest by implication. Ordinary row deletion revision-patches the exact record from `active` to `trashed`, retaining its values and binding a generated operation ID and explicit UTC deletion time. Restore returns that exact record to active state. Permanently deleting selected trashed record files is a distinct digest/byte-bound high-permission transaction and never removes referenced resources by guess. Trashing the owning table node still uses normal Workspace Trash and preserves the complete canonical document, sidecar, active/trashed record store, resources, annotations, descendants, and identities.

## Backup

Synchronization is not backup. A new workspace has no default formal backup destination. Recent History, drafts, Trash, transaction evidence, a second directory on the same disk, or a sync provider does not change `Backup: Not configured`. Setup is non-blocking but the UI remains truthful until the user selects a local/external/network target, a supported remote backend, or an authorized Server-managed target.

The backup contract requires:

- versioned snapshots to a user-selected local, external, mounted/network, Server-managed, or specifically supported remote destination;
- retention policy and protected restore points;
- content and metadata integrity verification;
- single-node, subtree, and full-workspace restore;
- Server control-plane backup when applicable;
- dry-run inventory before restore;
- restoration into an alternate location;
- routine automated restore drills with recorded results.

A sync provider's version history cannot be the sole backup. A successful copy is not acceptance evidence until a clean restore is opened, inventoried, and compared with the expected identities and content.

Backup configuration records both destination and executor outside the workspace. Executor mode is This device, Weftext Server, or Read only. A raw filesystem/network/cloud-synchronized repository has one writer; other devices attach read-only or use independent repositories. A managed backend may accept several requesters only behind one serialized service-side executor. The file/object repository uses immutable SHA-256 objects plus complete immutable snapshot manifests and protected markers, never Git or a database as authority. Its v1 layout, 24-hourly/30-daily/12-monthly minimum retention, writer takeover, manifest-last publication, garbage collection, health states, and compare/restore binding are normative in specification 22.

### Content-boundary backup invariant

Ignore means “exclude from Weftext product discovery,” not “exclude from backup.” A full-workspace snapshot or export advertised as backup must walk the physical workspace through a dedicated fail-closed backup inventory and include `.weftext-rules`, the root canonical document and its exact `weftext.template_library_root` configuration, managed content, every valid `weftext.template.json`, every valid `weftext.table.json` plus complete `weftext.records` directory including active and table-local trashed rows, visible unmanaged content, ignored content, and every `.weftext-trash/_weftext.items` directory, manifest, empty directory, and payload byte. Template/table documents, role sidecars/stores, resources, annotations, and Template/table-bearing Trash payloads are physical authority even when ordinary semantic projections exclude some of them. Trash item and table-record authority cannot be excluded by content rules or a product-discovery filter. The backup inventory must still reject or explicitly diagnose links, junctions/reparse points, path escapes, unreadable entries, live transaction ambiguity, invalid Template role/sidecar/configuration relations, invalid table sidecar/record/resource relations, and invalid manifest/payload relations. It may identify an entry as ignored, a table record, or a complete Trash item in the dry run, but the default backup set cannot silently omit its bytes. A user-requested exclusion policy is a separate explicit backup option shown in the preview and receipt; it never changes workspace classification and cannot claim a restorable Template Library, multidimensional table, or Trash state after excluding any required configuration, sidecar, record, or item byte.

The current hosted content boundary has no backup command. No copy helper may be relabeled as boundary-aware backup. Product backup completion requires a clean restore proving ignored files, unmanaged bytes, active and trashed node UUIDs, exact Template Library root configuration, every Template Root/Part document/sidecar/resource/annotation, valid Trash manifests/payloads, the rule file, and the disjoint Server control plane were all handled according to the recorded policy. Restore must re-inventory and verify the same Library/Root/Part roles without path guessing before enabling Template interfaces. Restore writes use their own reviewed boundary and cannot interpret “ignored,” “Trash,” `originStatus: unknown`, or a sidecar as permission to overwrite an existing target or silently bind Template configuration.

### Identity and session control plane

The identity and session boundary Server control plane is an explicitly configured directory disjoint from the hosted workspace. It contains the local Owner credential digest, session/revocation state, minimal security events, and the one-time bootstrap artifact before initialization. It is operational security state, not portable workspace content and not a document, node, resource, search, annotation, or revision authority. Copying or synchronizing the workspace alone therefore does not copy Server identity or active sessions.

The current identity and session boundary has no backup command or restore drill. Operators must therefore protect the workspace and control plane as two separate backup sets and stop the Server before any manual database copy; copying only the SQLite main file while WAL state is live is not a supported snapshot. Product acceptance requires a restore procedure that restores and integrity-checks portable content and control-plane state into disjoint locations before opening network service, invalidates or deliberately preserves sessions according to documented policy, protects control-plane file permissions, and proves clean-instance recovery. The raw bootstrap secret is excluded after bootstrap consumes it and is never exported in diagnostic bundles.

## Format versioning

Every persistent Weftext format has an explicit version. Opening an older supported version follows inventory, compatibility check, backup/restore point, dry run, versioned transaction, and verification. An interrupted version upgrade can continue or roll back. An application opens an unsupported format read-only or fails closed; it never guesses a workspace language or silently drops fields.

The legacy direct Trash layout `.weftext-trash/<name>` is accepted only through an explicit migration plan. The plan inventories and digest-binds every legacy entry, generates fresh temporary Trash item IDs, installs `_weftext.trash-item.json` plus unchanged payload through a recoverable journal, and removes direct entries only after verification. A former path or basename does not prove a parent/owner UUID. Without older transaction-bound origin evidence the manifest is `originStatus: unknown`, and restore requires an explicit existing target. Migration never keeps old direct entries and `_weftext.items` as two accepted authorities; collision, malformed content, partial sync state, or unreadable bytes block it and remain recoverable from the external snapshot.

## Diagnostics and safe mode

Desktop, WebUI, Server, and CLI expose appropriate variants of Safe Mode, transaction recovery, index rebuild, workspace verification, logs, and a redacted diagnostic bundle. Diagnostics exclude document bodies by default and never include secrets, session tokens, complete absolute paths, or inaccessible-node metadata. The user can inspect the bundle manifest before export.

Desktop Safe Mode is enforced at the native/Core-facing transaction boundary rather than only by disabled controls. It refuses document and structural workspace commits while permitting reads, verification, diagnostics, recovery comparison, and device-local draft persistence. Leaving Safe Mode is an explicit local preference change; it does not itself commit a draft.

## Security baseline

- Node IDs are identity, not credentials.
- Server write operations require authenticated sessions and authorization.
- Secrets stay in OS or Server secret storage, not workspace files, logs, URLs, or diagnostics.
- Paths are confined to the selected workspace or request-owned staging. A selected root is resolved once to an existing canonical non-link directory, so a platform-provided alias in an ancestor does not become workspace authority; a link or reparse point at the selected root or anywhere below it fails closed unless a future policy explicitly supports it.
- Bulk, destructive, cross-workspace, permission, and plaintext-egress operations require preview and confirmation.
- Dependencies, installers, updates, container images, and release artifacts require provenance, notices, SBOM, vulnerability review, and rollback instructions.

Weftext does not claim protection against an administrator or kernel-level malware on an unlocked device, user-authorized exfiltration, screen capture, or loss of all valid encryption credentials. Any encrypted workspace feature requires a separate reviewed design and independent recovery path.
