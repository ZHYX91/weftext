---
source_language: zh-CN
translation_of: 18-task-nodes-and-checklist-promotion.zh-CN.md
translation_status: synced
---

[简体中文](18-task-nodes-and-checklist-promotion.zh-CN.md)

# Task nodes and checklist promotion

Weftext has two canonical task forms: a native AsciiDoc checklist occurrence for a lightweight identity-free action, and an ordinary managed node carrying the closed `weftext-task` v1 document-header profile for a durable task. There is no inline task manifest, task sidecar, task database, or second task UUID namespace.

## Native checklist boundary

```adoc
* [ ] Open
* [x] Closed
* [*] Also closed
```

`[ ]` is open; `[x]` and `[*]` are closed. A checklist occurrence is identified only by owner node UUID, document revision, exact source range, and parser-confirmed list occurrence. It does not gain an ID when rendered, searched, queried, or toggled. Nested structure does not create dependencies, recurrence, ownership, or typed fields.

## Durable task profile

A durable task is an ordinary managed node and uses its existing `weftext.id` UUID. The header profile has closed literal fields:

| Attribute | Rule |
| --- | --- |
| `weftext-task` | required exact `v1` |
| `weftext-task-state` | required `todo`, `in-progress`, `on-hold`, `completed`, or `cancelled` |
| `weftext-task-priority` | optional `highest`, `high`, `medium`, `normal`, `low`, or `lowest` |
| `weftext-task-created`, `-start`, `-scheduled`, `-due`, `-closed` | optional ISO date or explicit-offset RFC 3339 instant; `closed` only for closed states |
| `weftext-task-depends-on` | optional unique space-separated task-node UUIDs; no self-edge or cycle |

Date-only values remain dates and instants never infer a timezone. `blocked` and `overdue` are derived. Unknown, duplicate, invalid, ambiguous, or cyclic fields diagnose the profile without hiding the underlying node. The root, Trash, unmanaged, and ignored content cannot carry this profile.

## Promotion and projection

Promotion is a Core plan that freezes source occurrence/revision, destination UUID/parent/name/title, task attributes, exact source replacement, affected annotations, draft evidence, and journal steps. It creates the task node and replaces the original checklist branch with a normal list item containing `node:<uuid>[<label>]` in one recoverable transaction. It never leaves a synchronized checkbox mirror, silently adds a suffix, drops ambiguous continuation content, or retargets after focus changes.

The `tasks` Query domain is a tagged union. Checklist rows expose only real occurrence evidence and derived open/closed state. Node rows expose the task-node UUID and typed profile fields. Toggles, promotion, scheduling, dependencies, bulk edits, and board moves select source-specific revision-bound Core actions. Authorization precedes dependency resolution, rows, counts, groups, and diagnostics.
