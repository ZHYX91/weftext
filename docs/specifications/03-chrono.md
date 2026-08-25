---
source_language: zh-CN
translation_of: 03-chrono.zh-CN.md
translation_status: synced
---
[简体中文](03-chrono.zh-CN.md)

# Chrono nodes

Status: accepted current Chrono boundary.

The user selects one Chrono root node. Internal period names and paths are fixed; arbitrary path templates are not part of the format.

Canonical names are:

- year: `YYYY`
- quarter: `YYYY-Qn`
- month: `YYYY-MM`
- ISO week: `YYYY-Www`
- day: `YYYY-MM-DD`

All generated periods are strict nodes. Period nodes live under their year node so that ISO weeks are not forced into a possibly incorrect month or quarter hierarchy:

The selected Chrono root and every reused/generated period must be managed content. An unmanaged or ignored directory with a canonical-looking period name is a boundary conflict, not a reusable period node, and Core will not enter or overwrite it.

```text
Chrono/
├─ Chrono.adoc
└─ 2026/
   ├─ 2026.adoc
   ├─ 2026-Q3/2026-Q3.adoc
   ├─ 2026-08/2026-08.adoc
   ├─ 2026-W34/2026-W34.adoc
   └─ 2026-08-21/2026-08-21.adoc
```

Configuration may select enabled periods, timezone, creation policy, display preferences, and an optional authorized Template Root UUID from the configured Template Library. It cannot redefine canonical period spelling or relative placement. The Template Root receives frozen typed period/date input through the contract in [`19-node-template-library.md`](19-node-template-library.md). A template declaring any `node_name` is incompatible and blocks Chrono use rather than being ignored; Chrono supplies each fixed period basename as the sole explicit target name, and no path template is accepted. Chrono remains analogous to selecting its root UUID: paths and hierarchy stay Core-owned.

Template Library root, Template Roots, and Template Parts are excluded as Chrono roots and period-node candidates. Chrono instantiation creates ordinary period nodes with fresh UUIDs, rewrites the selected template's internal links, copies owned resources, omits design annotations by default, and commits the complete missing period set through one reviewed recoverable transaction. A hidden, stale, invalid, unavailable, or name-binding template is non-disclosing and blocks before write.

The Chrono implementation accepts a selected node ID as the root, validates one Gregorian date, always includes the year, and plans all missing requested period nodes in one recoverable workspace transaction. Existing canonical nodes are reused; conflicts and a stale workspace revision are refused. Desktop and the loopback WebUI bridge expose the same preview, and commit uses the cached plan rather than regenerating identities.
