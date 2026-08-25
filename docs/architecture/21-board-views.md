---
source_language: zh-CN
translation_of: 21-board-views.zh-CN.md
translation_status: synced
---

[简体中文](21-board-views.zh-CN.md)

# Board views

Board is a common derived presentation shell, not a card database. Query boards and multidimensional-table boards share lanes, cards, virtualization, selection, and accessibility only where their typed row identities and Core action adapters allow it. Board persistence remains in the source domain.

## Task Board

Task Board groups the canonical checklist/task-node union by task state. Checklist cards expose only checklist capabilities; task-node cards expose durable task capabilities. A lane move is a source-specific Core action: it patches the exact checklist marker, task-node state header, or accepted table record field. It never creates a generic card/lane authority or silently promotes a checklist.

Adding a card creates a task node with an explicit active parent and explicit lane state. Adding a simple checklist is available only at an exact editable document occurrence. Card-title edits are source-specific; derived titles and computed fields are read-only.

## Ordering and accessibility

Horizontal drag changes only the lane field. Within-lane persistent manual order is not introduced: Query uses canonical `order by`; table boards use typed shared-view sorting. A placeholder may animate but must return to derived order after commit.

Every drag has an equivalent menu, keyboard command, and accessible state selector. Desktop/wide WebUI use virtualized lanes; narrow layouts use an accessible lane selector and vertical card list. Server filtering occurs before cards, lanes, counts, picker values, or diagnostics reach the browser.

## Surface boundary

Board is reached through dynamic-view or table-view actions and uses the existing Data contextual surface. It does not add a permanent board authority, workflow engine, board-specific comments/permissions/automation, swimlanes, hidden rank, or drag-only behavior.
