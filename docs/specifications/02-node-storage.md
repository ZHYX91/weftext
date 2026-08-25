---
source_language: zh-CN
translation_of: 02-node-storage.zh-CN.md
translation_status: synced
---
[简体中文](02-node-storage.zh-CN.md)

# Weftext node storage

Status: accepted canonical storage format.

A root `.weftext-format` containing exactly `weftext.asciidoc.v1\n` is required and selects `X/X.adoc` for the complete workspace. A missing or unknown marker fails closed or enters explicit import/adoption; it never selects Markdown. See [`15-weftext-asciidoc-profile.md`](15-weftext-asciidoc-profile.md) and [`../architecture/14-canonical-document-metadata-and-review.md`](../architecture/14-canonical-document-metadata-and-review.md).

## Content classes

A managed node is one directory containing exactly one same-named AsciiDoc node document:

```text
Project/
├─ Project.adoc
├─ image.png
└─ Child/
   └─ Child.adoc
```

The workspace root is always a managed node. Managed nodes use lowercase UUIDv4 identity and the revision, parent, ordering, and narrow-patch rules below.

Workspace storage has four Core-owned classes:

- managed node: the `X/X.adoc` structure above, with `weftext.id` and node behavior;
- visible unmanaged content: an ordinary directory or file explicitly classified by `.weftext-rules`, including unmanaged Markdown, shown by file/resource surfaces but not treated as a node; and
- ignored content: physical workspace bytes excluded from product inventory, resource browsing, search, derived indexes, and watcher results; and
- Trash item authority: the reserved `.weftext-trash/_weftext.items` store, manifests, and payloads, which are hidden from ordinary discovery but fully participate in revision, synchronization, backup, conflict detection, and migration.

An unmanaged item receives no UUID and no Weftext envelope. Unmanaged Markdown remains exact bytes and is never parsed or rewritten as a node document except through an explicit importer. An unmanaged directory forms a complete subtree boundary: Core may enumerate visible files beneath it for a file view, but it does not rediscover a same-named AsciiDoc/UUID pair as a managed node. A managed node's canonical same-named AsciiDoc document is represented by the managed-node item and is never repeated as an ordinary file. Every other ordinary file inside a managed directory is a resource owned by that node regardless of extension, including `.md`, `.txt`, images, PDFs, and office files. A node-owned `.md` resource is not parsed as node source and contributes no headings, tasks, links, properties, annotations, or Query rows; it may be opened/downloaded as an attachment or explicitly imported into a new canonical node. Only an explicit unmanaged rule removes owner identity and classifies that file as unmanaged Markdown. Reserved sidecars, reserved node-local stores, and transaction evidence never fall back to resources. The role-constrained `weftext.template.json` and table-constrained `weftext.table.json`/`weftext.records` names are reserved even where their presence is invalid.

When `.weftext-rules` is absent, every reachable content directory must be a managed node and every ordinary file belongs to its containing managed node as a resource. Adding the rules file opts into explicit unmanaged/ignored boundary semantics. Removing it from a workspace that contains such directories produces inventory diagnostics; no bytes are changed.

## Portable content rules

The optional UTF-8 root file `.weftext-rules` is portable workspace authority. It is included in the workspace revision, synchronized and backed up with the workspace, and never copied into node metadata. Weftext does not read, translate, or inherit `.gitignore`; VCS configuration is device/tool policy rather than Weftext product authority.

Blank lines and full-line comments whose trimmed form begins with `#` are ignored. Inline comments are not supported: a literal hash inside a pattern must be escaped. The first nonblank, non-comment line must be exactly, with no leading/trailing whitespace:

```text
weftext-content-rules-v1
```

Every later nonblank, non-comment line is one of the following, also with no leading whitespace and exactly one unescaped separator space after the action:

```text
unmanaged path/pattern
ignore path/pattern
```

Rules are evaluated in authored order and the last matching rule determines the action for that entry. The default is the strict managed/resource interpretation. Matching is case-sensitive on every platform. Paths are relative to the workspace root, contain UTF-8 components separated only by `/`, and never start with `/`, `\`, a Windows drive prefix, `.` or `..` components. `*` matches zero or more scalars inside one component, `?` matches exactly one scalar, and `**` is valid only as a complete component and matches zero or more complete components. A trailing `/` restricts a pattern to directories. `\ `, `\#`, `\\`, `\*`, and `\?` encode literal space, hash, backslash, star, and question mark; other escapes, unescaped pattern spaces/hashes, empty components, NUL, malformed UTF-8, unknown actions, missing/unknown headers, lines above 4096 bytes, and files above 1 MiB are invalid.

If an `unmanaged` or `ignore` rule matches a directory, that directory is an immediate recursive barrier. Descendant rules are not a mechanism to re-enter managed scanning. A later rule can override an earlier rule only for an entry Core actually reaches before a directory barrier. Classifying `.weftext-format` or `.weftext-rules`, classifying the workspace root, classifying `.weftext-trash/_weftext.items` or anything below it, classifying a valid `weftext.records` store or its descendants, or classifying a managed node's canonical AsciiDoc document without classifying the complete node directory is a boundary conflict.

Any invalid or conflicting rule makes the inventory invalid. Core reports `InvalidContentRules` or `CanonicalDocumentBoundary` and stops before broadening discovery. It never silently ignores an invalid line. The authority filename is reserved to the selected root; another reachable `.weftext-rules` is an explicit multiple-authority diagnostic rather than a nested override. Symlinks and Windows junction/reparse points are inspected before trusting a classification and are never followed, including when their lexical path would be ignored. Canonical/resolved containment checks, relative-component validation, and transaction-time rechecks prevent absolute, `..`, separator, link, reparse, or nested-authority aliases from crossing the selected root.

The format has no general infrastructure directory, workspace-wide manifest, or central identity registry. The only workspace-wide portable infrastructure subtree is the closed Trash item store defined below. A versioned role may own a narrowly named node-local sidecar/store such as the table profile below; it does not authorize a general metadata directory or cross-node registry. `.weftext-format` selects the profile and `.weftext-rules` contains classifications only. Reserved VCS directories, `weftext.annotations.json`, table sidecars/record stores, Trash manifests/payloads, and transaction evidence are not ordinary resources. Application caches and derived indexes live outside the workspace.

Reserved naming is role-based. Root controls use `.weftext-*`; closed children inside a reserved store may use `_weftext.*`; portable node-local sidecars use `weftext.*` without a leading dot or underscore; and reserved typed document-header attributes use `weftext-*`. There is no rule that every Weftext-owned JSON file or directory begins with `_`. See [`../architecture/14-canonical-document-metadata-and-review.md`](../architecture/14-canonical-document-metadata-and-review.md).

## Multidimensional-table node authority

A multidimensional table remains one managed node with its normal node UUID, canonical AsciiDoc document, resources, children, and node-level permissions. An adjacent `weftext.table.json` sidecar with profile `weftext.table/v1` declares its field schema and portable shared views. The sibling `weftext.records` reserved store contains one `weftext.table-record/v1` JSON file per homogeneous record, sharded by the first two hexadecimal characters of its lowercase UUID filename.

A record UUID is table-local and addressed only together with the owning table-node UUID. It is not `weftext.id`; the record receives no node document, envelope, children, independent ACL, annotation sidecar, or promotion path. Node and record relations use typed values, while referenced images/files remain ordinary node-owned resources outside the reserved store. Formula/rollup values and indexes are derived and never stored as competing record authority.

The table sidecar and record store are an indivisible role relation. Invalid/missing profiles, wrong file kinds, filename/shard/decoded-ID mismatch, duplicate IDs or JSON keys, unknown schema/value fields, partial synchronization, conflict-copy names, or links/reparse points produce a table diagnostic and never broaden resource or node discovery. Content rules cannot enter the store. Normal row deletion is a record-state transaction into the table's Deleted Records surface; deleting the table node uses ordinary Workspace Trash and preserves the complete table branch. Copy rekeys the destination node and every record; move, Trash, and restore preserve them. The exact closed schemas, types, limits, transactions, dynamic-view separation, backup, and acceptance contract are in [`20-multidimensional-tables-and-editor-surfaces.md`](20-multidimensional-tables-and-editor-surfaces.md).

## Workspace Trash item authority

Workspace Trash is a hidden Core-owned special node at `.weftext-trash`. Deleted objects are not renamed into its managed child hierarchy. Every independently restorable object occupies one no-clobber item directory:

```text
.weftext-trash/_weftext.items/<trashItemId>/
├─ _weftext.trash-item.json
└─ payload/<original node directory or original resource filename>
```

`trashItemId` is a temporary lowercase UUIDv4 generated and occupancy-checked during a revision-bound plan; it is distinct from every permanent node UUID. The manifest filename is exactly `_weftext.trash-item.json`, its closed schema is `weftext.trash-item/v1`, and its ID must equal the directory basename. It also records one `operationId`, `kind`, timestamp, origin status, exact identity/origin fields, and payload length/digest evidence. Unknown fields, duplicate JSON keys, ID mismatches, exact or case-fold item collisions, missing/extra payload entries, conflict-copy names, altered digests, links/reparse points, or duplicate active/trashed node UUIDs are reconciliation diagnostics. They never authorize overwrite, silent suffixing, rekeying, or cleanup.

Deleting one node produces one `node` item containing the complete original same-named directory and all descendants, canonical `X/X.adoc` files, permanent node UUIDs, annotations, and owned resources without renaming. Descendants do not receive manifests. The node manifest records `nodeId`, `originalParentNodeId`, `originalName`, optional non-authoritative `ancestorNodeIds`, and a canonical aggregate inventory/digest. Deleting several independent node-owned resource files produces one `resource` item per regular file, each preserving its filename and recording `originalOwnerNodeId`, `originalName`, byte length, and SHA-256; all items from one batch share an `operationId`. A resource does not gain a permanent UUID.

An unmanaged or ignored entry cannot enter Trash through either API. A managed subtree containing such a boundary is rejected as a whole. Canonical documents, annotation sidecars, transaction evidence, and any reserved Trash path are not independently deletable resources. Node UUIDs retained below a valid item payload are inactive authority: ordinary node scanning, navigation, links, search, Chrono, tasks, and queries do not enter the store, while revision, synchronization, backup, verification, and migration include every item directory and byte.

Restore resolves permanent UUIDs, never historical paths or names as identity. Original restore is available only when the recorded parent/owner UUID is active and the exact/case-fold target is free. If the origin is itself a complete node item, Core may preview one atomic parent-chain restore before the selected item. If the origin is missing or legacy `originStatus` is `unknown`, the item remains in Trash until the user explicitly selects an existing target; Core never fabricates a same-named parent. A conflict requires an explicit alternate target or portable rename and is always no-overwrite. Permanent deletion is a separate high-permission, digest-bound, explicitly confirmed transaction. Trash is not backup.

The complete manifest shapes, payload digest, transaction, synchronization, migration, and caller rules are authoritative in [`../architecture/17-workspace-trash-item-store.md`](../architecture/17-workspace-trash-item-store.md) and [`13-workspace-transactions.md`](13-workspace-transactions.md).

## Identity

Every node document contains a lowercase UUIDv4:

```yaml
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
---
```

The frontmatter must contain exactly one top-level `weftext` mapping. `id` is persistent identity. Path is the current locator; parent and name are derived from the directory tree. Path, parent, name, node type, child paths, filesystem timestamps, and derived state are never persisted in the envelope.

Move and rename preserve IDs. A Weftext copy rekeys the copied subtree. A cloud replica preserves IDs. Delete followed by new creation at the same path creates a new ID. A duplicate ID inside one workspace is an identity conflict.

The root node ID identifies the initial workspace lineage. Forking versus replicating a complete root is an explicit operation; a raw filesystem copy is reported as an unresolved replica/fork state until the user chooses.

## Sorting

Absent configuration means natural name ordering, ascending. A parent controls how its direct children are sorted:

```yaml
weftext:
  child_sort: manual
```

or:

```yaml
weftext:
  child_sort: name
  child_sort_direction: ascending
```

A child stores only its sparse rank:

```yaml
weftext:
  sibling_rank: 2048
```

Ranks are positive integers normally spaced by 1024. They are meaningful only when the actual parent uses manual mode. Missing ranks sort after ranked children; ties use normalized basename and then path. Switching to manual materializes sparse ranks. Name mode ignores dormant ranks.

## Source authority

The filesystem is authoritative for structure. The `weftext` envelope is authoritative for identity, navigation icon, aliases, minimal sorting policy, and root-only Template Library selection. A rebuildable local index maps ID to current path. There is no general node JSON manifest and no central identity registry. The one fixed `weftext.template.json` contract is valid only on a derived Template Root and cannot be generalized to ordinary nodes.

Metadata edits must patch only the required YAML ranges and fail closed on ambiguous or unsupported frontmatter. Whole-frontmatter reserialization is not an acceptable normal edit path.

## Reference-capable nodes

A bibliographic reference remains an ordinary managed node; it does not gain a node type, manifest, or parallel database. The top-level YAML `reference` mapping is not part of the target envelope. Typed Citation Data carries structured authored bibliographic facts for reference creation/editing. `weftext.id` remains stable node/reference identity; a mutable citation key never substitutes for it. See [`16-citations-and-bibliography.md`](16-citations-and-bibliography.md).

## Task source authority

Simple tasks are native checklist occurrences in the canonical document body. They are identified only by owning node UUID, exact document revision, exact source range, and parser-confirmed occurrence; viewing, indexing, or toggling one never writes identity or typed metadata.

A durable task is an ordinary managed node carrying the closed `weftext-task` v1 profile in literal AsciiDoc document-header attributes. Its existing `weftext.id` is task identity. It receives no node type in the YAML envelope, task manifest, task sidecar, separate task UUID, or database row. A checklist that needs dates, priority, dependencies, body/resources, task-level annotations, or other durable behavior is explicitly promoted through a recoverable workspace transaction; its original checklist position becomes a stable `node:` link and no checkbox mirror remains.

Task indexes, query results, boards, calendars, dependency graphs, and task counts are derived and rebuildable. Checklist toggles patch the exact owning source; task-node changes patch the narrow literal header attribute; promotion creates the node and replaces the source occurrence atomically. Recurrence is not defined for task-node v1. The complete authority is [`../architecture/18-task-nodes-and-checklist-promotion.md`](../architecture/18-task-nodes-and-checklist-promotion.md) and [`17-tasks-and-query.md`](17-tasks-and-query.md). The trailing `task:[...]` macro is accepted only as a migration input and is not node authority.

## Template Library roles

The workspace root may persist one `weftext.template_library_root` lowercase UUIDv4. Its uniquely resolved active managed node is the Template Library root and is a container only. Every direct managed child is one Template Root; every descendant below that child is a Template Part owned by that root. A Part is not independently instantiable, and v1 has no nested category layer.

These remain `X/X.adoc` managed nodes, but their role is derived from the configured root and current tree. Template Library root, Root, and Part are excluded from every ordinary semantic projection: nodes, tasks, headings/outlines, citations/bibliographies, search, graph, Chrono, default links/backlinks, recents, and canonical `nodes|tasks|headings` Query rows. They appear only through a dedicated Template Library projection; only Template Roots appear in the explicit `templates` Query domain, which returns `domain_unavailable` until deferred role-aware inventory exists.

Only a Template Root carries adjacent `weftext.template.json` with the fixed pair profile `weftext.node-template.v1` and version `1`. The sidecar defines the closed parameter/slot contract for the entire Root/Part subtree, and every slot scope is the permanent Node UUID of its Root or Part. The same filename on the Library root, a Part, an ordinary node, or another location is a diagnostic, not a resource or alternate role declaration. Independently written `slot:name[]` and `slot::name[]` are inert in ordinary AsciiDoc and gain semantics only through a validated role/profile; a formerly valid prototype cannot move out and silently become inert.

The deferred instantiation target creates a fresh-UUID ordinary subtree in one reviewed recoverable transaction, rewrites internal links through the complete mapping, copies owned resources, omits design annotations by default, and never copies the sidecar or role. Moving across a Template role boundary is a future conversion transaction with exact role/source/sidecar validation and draft gates, not a path-only move; leaving Template space must materialize/delete all active slots and leave zero profile/sidecar residual or block. Generic Trash refuses the currently configured Library root unless the same explicit transaction clears/rebinds root configuration. Trash payloads remain closed exact bytes, and restore never guesses or rebinds configuration. See [`19-node-template-library.md`](19-node-template-library.md).

## Envelope and document properties

`weftext` is the only permitted top-level YAML key. Its v1 fields are `id`, `icon`, `aliases`, `child_sort`, `child_sort_direction`, `sibling_rank`, and root-only `adjacent_heading_body` plus `template_library_root`. The icon is one supported Weftext token or literal emoji scalar. Aliases are an ordered string list used for node lookup and links; they are not title, tags, or identity. `adjacent_heading_body` is `run_in` or `separate`, with absence equivalent to `separate`. `template_library_root` is an optional lowercase UUIDv4 and is never inferred from a folder name or sidecar. Duplicate fields, unknown top-level keys, structurally invalid values, YAML aliases/tags, or an ambiguous envelope fail closed. Unknown inner fields are preserved and diagnosed for forward compatibility.

Title, subtitle, author, revision, language, description, keywords/tags, status, and custom note properties use the AsciiDoc document header and header attributes. Only header attribute entries enter the Properties projection; later body redefinitions remain processing state. Arbitrary custom attributes remain literal strings. A closed reserved profile such as `weftext-task` may independently type its own `weftext-task-*` fields under a versioned contract; that does not infer types for unrelated attributes. Complex structured data uses a versioned typed Profile construct rather than YAML or an improvised attribute encoding.

Icon, alias, and order UI actions submit typed intent and perform narrow revision-checked patches. Merely viewing a node never writes defaults. Core-derived fallback icons are presentation only. Hiding the envelope in Write or Read does not remove it from exact Source or make it a security boundary.
