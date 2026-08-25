---
source_language: zh-CN
translation_of: 22-document-history-comparison-and-backup-repositories.zh-CN.md
translation_status: synced
---

[简体中文](22-document-history-comparison-and-backup-repositories.zh-CN.md)

# Document history, comparison, and backup repositories

This specification defines document history, comparison, and formal backup repositories. No client may label device history, Trash, a sync folder, or an unverified copy as formal backup.

## Product states and terminology

The backup status is one of `not_configured`, `healthy`, `overdue`, `unavailable`, `verification_required`, or `read_only`. A new local workspace starts `not_configured`. Weftext creates no implicit backup target beside the workspace, inside application data, or elsewhere on the same disk. Recent History may be enabled, but the product displays:

```text
Backup: Not configured
Recent History: This device only
Last formal backup: None
```

Connecting to a Server changes this only when the Server reports an authorized configured target and current verified status. A cloud-synchronized workspace, sync-provider version history, Workspace Trash, device draft store, transaction journal, local Recent History, or derived index never changes `not_configured` to `healthy`.

The UI term **Version History** covers document revisions. **Chrono** remains date-node creation, and **Timeline** remains a data/view layout. Neither is a backup or revision store.

## Recent History store v1

Desktop stores Recent History outside the selected workspace under its OS application-data root, logically:

```text
workspaces/<workspace-instance-scope>/history-v1/
  _weftext.history-store.json
  entries/<entry-id>.json
  objects/sha256/<first-two-hex>/<digest>
```

The trusted native process derives the bounded workspace-instance scope; it is never a browser path, credential, portable workspace ID, or absolute path disclosed in public DTOs. A copied workspace at another physical instance receives a separate local scope unless an explicit recovery/import action attaches selected history as read-only evidence.

`_weftext.history-store.json` has profile `weftext.document-history-store/v1`, integer version `1`, lowercase UUIDv4 `storeId`, opaque workspace-instance binding digest, current root-node UUID, created time, and retention configuration. It contains no source bodies, target address, credentials, search data, or mutable “current document” pointer. Unknown/duplicate members, invalid identifiers/times/bounds, or a workspace-binding mismatch fail closed and preserve the store for diagnostics.

Every immutable entry is named by lowercase UUIDv4 and has closed profile `weftext.document-history-entry/v1`, version `1`, matching `id`, node UUID, base/result document revision strings, base/result object SHA-256 digests and lengths, action kind, explicit-offset commit time, nullable bounded actor/device display evidence, and optional preceding entry ID for presentation. Action kind is `document_save`, `format`, `property_edit`, `merge`, `history_restore`, `external_resolution`, or another separately versioned Core-authored kind. The predecessor is not required for object recovery and cannot override revision/digest evidence.

Object files contain exact raw canonical source bytes. Their lowercase 64-hex filename and shard bind the SHA-256 digest; existing objects are reopened and verified before reuse. Entries never embed source, paths, drafts, rendered HTML, patches, or annotations. Full-entry JSON publication is no-clobber and occurs only after both objects verify. An orphan object or staging byte is not a version and is cleaned only through bounded store maintenance.

For a successful canonical-document commit, Core/native authority supplies exact verified pre/result sources and revisions. History staging may begin before workspace mutation, but the entry is published only after the workspace commit verifies. If entry publication fails, the commit result remains successful and adds a structured `history_capture_failed` warning; no empty/fabricated entry is written. Retrying history capture is a separate idempotent operation over exact retained commit evidence, never a repeated document commit.

Recent History is enabled by default with a visible, configurable time/space retention policy. The release must publish its bounded defaults; disabling or clearing requires confirmation that states `This device only` and reports entries/bytes. Retention removes entries oldest-first within policy, then removes only objects unreachable from remaining entries. Protected formal backup snapshots and other devices/stores are outside that operation. Corrupt/unreadable entries or objects remain Recovery/Diagnostics issues and are not silently regenerated from current source.

V1 Recent History captures canonical document source only. It does not claim exact node/subtree recovery when resources, annotations, table records, Template sidecars, ignored/unmanaged content, Trash, child nodes, or Server state changed. The history UI labels a source-only restore accordingly. Complete restore uses a formal backup snapshot.

## Backup target configuration

Backup configuration is device or Server operational state outside the workspace. It includes target kind/address, repository ID after initialization, executor mode/identity, schedule, retention, authentication reference, bandwidth/offline policy, and last operation/verification summaries. Credentials remain in OS or Server secret storage. Diagnostics redact secrets and complete remote/local addresses by default.

Supported target kinds are explicitly capability-versioned. V1 may deliver local directory, mounted/network directory, Server-managed filesystem, and reviewed remote object storage independently; an unavailable kind is not represented as a generic URL fallback. Each backend proves path/root confinement, no-clobber object creation, atomic or conditionally created manifests, bounded list/read/delete, read-after-write verification, retry/idempotency, interruption recovery, and error mapping before product support.

Executor mode is `this_device`, `server`, or `read_only`:

- `this_device` permits only the bound Desktop executor to snapshot, protect/unprotect, retain, verify, and garbage-collect;
- `server` permits only the configured Server backup service to do so, while clients request authorized operations; and
- `read_only` permits inventory, compare, and restore planning but no repository mutation.

A raw filesystem/network/cloud-synchronized repository has exactly one configured writer binding. Another device attaches read-only or initializes a separate repository. Writer transfer is an explicit reviewed takeover that verifies the repository, proves no active lease/request, replaces the executor binding atomically, and records a receipt; opening the same directory never silently steals it. A managed backend may serialize several requesters behind one server-side executor, but clients still do not publish manifests or run retention directly.

The target must resolve outside and disjoint from the selected workspace and any request-owned workspace staging. Lexical and resolved containment, symlink, Windows junction/reparse, hard-link alias, and nested-target checks fail closed. A same-disk target is labeled `local recovery only` unless the user explicitly acknowledges that it does not protect against disk/device loss. A target inside the workspace's sync tree cannot be the sole formal backup.

## Backup Repository v1

One repository directory is bound to one workspace backup lineage:

```text
<repository>/
  _weftext.backup-repository.json
  objects/sha256/<first-two-hex>/<digest>
  snapshots/<snapshot-id>.json
  protected/<snapshot-id>.json
  staging/<operation-id>/...
```

The closed repository manifest uses profile `weftext.backup-repository/v1`, version `1`, repository UUIDv4, lineage UUIDv4, root-node UUID, created time, writer mode/opaque writer binding, and object/snapshot schema versions. It contains no target address, credential, machine path, workspace path, user secret, or mutable latest-snapshot authority. A repository may be moved as a complete target; reconnect verifies its identity and explicit workspace-lineage binding.

Attaching a workspace requires an exact active root-node UUID plus explicit lineage selection. Two physical copies with the same root UUID never silently share a write lineage: the user either reconnects the known continuing instance or forks a fresh repository lineage. Forking does not copy or claim the old history unless an explicit import/read-only attachment is reviewed.

An object path is lowercase SHA-256 and contains the exact regular-file bytes. Object creation is immutable no-clobber; existing bytes/digest/length are verified before reuse. Compression or encryption, if later accepted, must retain a separately versioned plaintext content-digest binding and recovery contract; v1 does not claim either.

Every `snapshots/<snapshot-id>.json` is an immutable closed `weftext.backup-snapshot/v1` manifest named by matching UUIDv4. It contains repository/lineage IDs, root-node UUID, explicit-offset created time, reason (`scheduled`, `manual`, `pre_migration`, `pre_restore`, or separately versioned kind), nullable preceding snapshot ID, workspace format marker/profile, complete physical inventory digest, and a canonically path-sorted entry list. Each entry is one closed variant:

- directory: normalized workspace-relative `/` path and `kind: directory`;
- regular file: path, `kind: file`, byte length, and SHA-256 object digest.

The root directory uses an empty relative path; absolute, empty non-root components, `.`/`..`, separator aliases, duplicate/exact-casefold collisions, non-UTF-8 paths, unknown kinds/members, links/reparse points, hard-link alias ambiguity, missing/extra object, or digest/length mismatch invalidates the snapshot. Empty directories therefore round-trip. The snapshot includes every physical authority required by specification 08, including ignored bytes and Trash stores; classification may be recorded as non-authoritative display evidence but cannot omit bytes.

Snapshot publication takes one exact fail-closed physical inventory and workspace revision/lease, stages/verifies all absent objects, writes and reopens the complete manifest last through atomic/no-clobber semantics, and returns a receipt containing snapshot ID, inventory digest, file/directory/byte totals, reused/new object totals, target/repository identity, and verification result. Workspace mutation after inventory rejects or restarts planning; it never produces a manifest mixing times. Staging without a verified manifest is incomplete operation evidence, not a snapshot.

`protected/<snapshot-id>.json` is a closed `weftext.backup-protection/v1` marker with matching repository/lineage/snapshot IDs, explicit-offset protected time, and bounded label. Protection does not modify the immutable snapshot. Unprotect/delete is explicit and permission-checked; retention cannot remove a manifest with a valid marker. An invalid marker blocks retention/garbage collection and preserves all possibly referenced objects.

The initial configured retention policy keeps at least 24 hourly, 30 daily, and 12 monthly snapshots plus every protected snapshot. Scheduling may skip creation when the complete physical digest is unchanged while recording a successful check, but retention windows are not silently weakened. Retention first previews exact unprotected snapshot manifests; commit removes only those manifests/markers allowed by policy through a recoverable repository transaction. Object garbage collection is separate: it revalidates every retained/protected manifest, computes reachability, previews object/byte deletion, and never deletes on incomplete/corrupt inventory, active staging, another lease, or read-only mode.

No SQLite, derived catalog, object cache, or “latest” pointer is repository authority. Implementations may build disposable indexes outside or inside a clearly derived cache directory and rebuild them from repository/snapshot/protection manifests. Restore and verification work with those caches absent.

## No-default behavior and health

On workspace creation/open, Weftext offers non-blocking setup: choose a local/external/network target, use a Server-managed target, or Not now. Choosing Not now leaves `not_configured`, does not create a repository, and does not block edits. Settings, Version History, and Recovery Center continue to show the missing formal backup without repetitive modal interruption.

After target selection, the first full snapshot is offered immediately. `healthy` requires a current successful snapshot under policy, complete repository verification, and a clean restore drill within the configured verification interval. A completed copy without reopen/digest verification is not success. `overdue`, `unavailable`, or `verification_required` never display as safe. Workspace commits succeed independently of backup availability but report pending/unbacked state; they do not retry writes.

“Back up this version now” requests the configured executor to snapshot the complete workspace, not merely copy one Recent History source. A read-only client forwards an authorized Server request where supported or explains that its executor cannot write.

## Server and cross-device history

Server mode keeps current workspace, control plane, and history/backup repository as disjoint roots. The Server is the executor and serializes snapshot/retention/restore actions with workspace mutations. A consistent hosted backup binds one workspace revision/physical inventory, compatible control-plane snapshot, repository/history state, and receipt. Live control-plane SQLite is backed up through its accepted online/quiesced procedure; copying the main database file alone is invalid.

Clients see the same authorized Server Version History without direct repository credentials or paths. Server control-plane rows may index/audit snapshot and history IDs but old document bodies remain repository objects, not SQLite-only content authority. A portable workspace sync/copy omits Server history; a complete Server migration includes the separately inventoried workspace, control plane, and repository.

For non-Server cloud-folder workspaces, each device's Recent History remains local. Connecting several devices to one raw repository does not grant several writers. Devices may share read-only access to a mounted repository, or each writes a distinct repository. The UI may union read-only histories with provenance but never invents one total order from unsynchronized device clocks; it orders by time then stable source/entry ID and labels source.

## Comparison inputs and output

Core accepts closed immutable `DocumentCompareInput` references:

- `current`: node UUID plus exact current document revision;
- `recent_history`: authorized store/entry ID plus bound node/revisions/digests;
- `backup`: repository/snapshot ID plus node UUID and exact snapshot file digest;
- `draft`: node UUID, profile, base revision, exact source digest/source from the invoking device/session draft boundary;
- `external`: exact captured file source/revision from Conflict Center; or
- `other_node`: authorized node UUID plus exact current revision.

Two-way comparison takes `left` and `right`. Three-way takes explicit `base`, `left`, and `right`; all three profiles must be compatible or conversion is a separately reviewed operation. Authorization and non-disclosure precede history lookup, snapshot membership, source load, parse diagnostics, counts, or diff output. Hidden and missing versions/targets are indistinguishable.

Core parses each exact source with the canonical profile and produces a bounded deterministic `DocumentComparison`: input identities/digests, same-node flag, system-metadata disposition, ordered typed change groups, exact left/right source ranges, stable display snippets, structural context, conflict classification, and resource/annotation/record/structural summary where separately inventoried. Comparison is read-only and side-effect-free; it returns no filesystem path or write capability.

The engine matches parser-owned document header, sections/headings, paragraphs, lists, native tables, protected blocks, and Weftext extension blocks before stable line/word fallback. It does not execute includes/network/macros, render HTML, normalize line endings, expand attributes, or rewrite either source. Moved unique structures may be labeled moved only with deterministic proof; ambiguity remains delete/add or conflict rather than a guessed move.

When comparing different nodes, system-envelope ranges are collapsed and identity-only changes excluded from the default content group. `weftext.id` is always protected from apply. The user may inspect complete Source differences, but node identity, backup lineage, paths, derived backlinks/indexes, and filesystem timestamps are never copyable content hunks.

Limits cover each source/object size, blocks, headings, table cells, diff tokens/hunks, recursion, work steps, output bytes, and total compare time. Crossing a limit returns one typed no-partial-result diagnostic. CJK/RTL/grapheme boundaries and CRLF/LF/CR source ranges remain exact.

## Merge and history restore plans

Viewing/comparing history requires no workspace transaction. Any accepted hunk disposition first builds a controlled result draft. A `DocumentMergePlan` binds target node/current base revision/source, every compare input digest, comparison ID, selected hunk dispositions, exact proposed source, target profile parse/model diagnostics, system/protected-range policy, authorization, device draft observation, and expiry. It never applies visual line numbers, stale snippets, raw paths, or a client-generated patch.

`Use left`, `Use right`, and `Keep both` are offered only where Core has one deterministic exact replacement. Conflicts requiring editing create an explicit editable result range. The final result is reparsed and previewed as exact source plus semantic changes before commit. Commit writes one target document through the ordinary recoverable transaction and records a new Recent History/Server history event; a post-commit history failure is a warning as above.

A `DocumentRestorePlan` is a specialized full-source replacement from one exact Recent History or backup version. It shows current-versus-selected comparison, target node, selected source provenance/time/digest, excluded resources/sidecars, system-envelope handling, resulting document, and draft/stale/permission consequences. Same-node restore preserves current `weftext.id`; an older envelope cannot replace identity. “Restore as new node” is instead a structural create plan with fresh UUID/name/parent and separately reviewed link/resource behavior.

Complete node/subtree/workspace restore comes only from a formal backup snapshot. Its dry run validates repository/snapshot/object integrity, target location, current physical inventory, identities, collisions, permissions, current drafts, no-overwrite policy, and rollback/alternate-location strategy. Restoring to an alternate location is the default low-risk drill. A clean restore reopens the target and compares complete expected inventory/identities/digests before the repository health can advance.

## UI contract

Review contains Compare and Version History. Compare offers Previous version, Version History, Another document, Backup snapshot, and Resolve draft/external conflict. Version History in the right Inspector merges available entries while displaying one of `This device`, `Backup`, `Server`, or `Protected restore point`, plus exact availability and whether a complete restore is possible.

The comparison surface supports inline and side-by-side modes, change navigation, structural context, source/render toggle where safe, system-metadata fold, resource/annotation summary, and per-change dispositions. Compare is non-editing until the user chooses Create result draft. Restore is never the primary action without a current-versus-selected preview; copying old source to clipboard is distinct from applying it.

Backup settings show target type/address in redacted form, executor, policy, last attempt, last successful snapshot, last verification, last clean restore drill, pending/unbacked changes, repository health, and read-only status. First-run Not now remains available. No configured target means no green shield/check mark or “protected” wording.

All controls have keyboard and screen-reader semantics; changed/inserted/deleted/conflicting content is not color-only. IME composition is isolated from change shortcuts. Large comparisons/history lists are virtualized without losing focus, source positions, provenance, or selected hunk state. Desktop, hosted/local WebUI, CLI, Server API, and approved agent surfaces consume the same Core comparison/plan evidence for every capability they expose.

## Acceptance matrix

Release evidence includes:

- clean workspace with `not_configured`; Recent History default/provenance; non-blocking setup; no implicit folder; truthful same-disk/read-only/overdue/unavailable/verification state; and no reclassification from sync/Trash/drafts/journals;
- exact Recent History store/entry/object decoding, duplicate-key/ID/path/digest/binding failures, pre/result capture, commit-success/history-failure separation, orphan cleanup, bounded retention/clear/disable, corrupt preservation, and no workspace or cross-device sync;
- local, external, network, Server, read-only, and each claimed remote backend with path disjointness, writer binding/takeover, lease, no-clobber, retry/idempotency, partial staging, credential redaction, offline recovery, and multi-request serialization;
- closed repository/snapshot/protection schemas, full physical inventory including empty/ignored/unmanaged/Trash/table/template bytes, immutable deduplicated objects, manifest-last publication, unchanged-snapshot checks, 24/30/12 retention, protected points, recoverable manifest deletion/garbage collection, derived-index removal/rebuild, and corruption refusal;
- scheduled/manual/pre-migration/pre-restore snapshots, complete receipts, workspace mutation race, same-root copied-workspace lineage fork/reconnect, local-only warning, paired Server workspace/control-plane/history consistency, and clean alternate-location restore drills;
- every comparison input pair and three-way base, same/different nodes, system identity protection, title/header/body/heading/list/table/protected/extension structure, moved/ambiguous blocks, line/word fallback, resources/annotations summaries, CRLF/LF/CR, CJK/RTL/graphemes, malformed source, limits/cancellation, deterministic output, and zero Git/runtime repository dependency;
- merge/restore hunk dispositions, conflict editing, exact proposed source, parse/profile failure, stale target/history/repository, dirty drafts, ACL/non-disclosure, replay/expiry, crash recovery, successful commit plus history-capture failure, new history event, identity preservation, and restore-as-new-node fresh identity; and
- Desktop/WebUI/Server/CLI parity where delivered, keyboard-only and screen-reader compare/history/restore, IME, zoom, high contrast, reduced motion, virtualization, redacted diagnostics, and permission-filtered history/count/time/actor/snapshot non-disclosure.

A Recent History list, Git wrapper, diff library screenshot, synchronized copy, or backup manifest without clean restore does not satisfy this contract.
