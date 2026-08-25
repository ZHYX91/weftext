---
source_language: zh-CN
translation_of: 21-board-views.zh-CN.md
translation_status: synced
---

[简体中文](21-board-views.zh-CN.md)

# Board views

This specification binds the generic board projection, Query/table adapters, Task Board preset, mutation rules, and accessibility requirements. Board actions are available only when the referenced Core action and transaction can revalidate and commit the defined typed intent.

## Board contract

A board is a derived arrangement of typed source rows into lanes and cards. It has no canonical Card file, Card ID namespace, card database, or portable frontend state. Every delivered board projection binds:

- `sourceKind`: canonical Query or multidimensional table;
- exact source identity and revision: Query owner/range/source revision or table-node UUID/shared-view ID/sidecar revision;
- permission-filtered lane keys and labels;
- typed row/card identity and current authority fingerprint;
- projected display fields;
- deterministic card ordering and result/window bounds;
- per-card and per-lane capabilities with structured unavailable reasons; and
- opaque action evidence sufficient for Core to revalidate, never raw path/DOM/visual-index authority.

A task-node card key is its node UUID. A node card key is its node UUID. A table-record card key is `(table node UUID, record UUID)`. A native-checklist card key is a revision-local occurrence binding of owner-node UUID, canonical document revision, parser-confirmed source range, and row kind; it is not persisted, linked, or presented as durable checklist identity. A refreshed source revision may replace that key and invalidates older actions.

The common board shell may consume this projection to render and route actions. It may not manufacture lanes, card fields, totals, identities, permissions, or mutations from rendered text. Source adapters are Core/backend modules shared by delivered callers, not separate browser evaluators.

## Canonical Query board

A portable Query board is an ordinary canonical block with `view=board` and exactly one scalar `group by`. The current Query grammar remains authority:

```adoc
.Tasks
[.weftext-query,version=1,view=board]
....
from tasks as task
scope subtree(this.node)
where true
group by task.state as lane
select task.kind, task.id, task.title, task.state, task.priority, task.due, task.blocked, task.owner_node
order by task.due asc nulls last, task.title asc
limit 1000
....
```

`from`, `scope`, and `where` define candidate population; `group by` defines the lane key; `select` defines card fields; `order by` defines order inside every lane; and `limit` bounds the complete delivered row result. The board never interprets `view=board` as permission to replace those clauses or save a second body. A missing `group by`, non-scalar group, unsupported source version, or unavailable domain produces a builder diagnostic rather than an ungrouped portable board.

Core evaluates authorization before scope, filter, group, projection, order, limit, counts, diagnostics, or suggestions. Clients receive only authorized cards and lane values. V1 Query has no aggregate/total result contract, so a Query board lane may display “N loaded” for delivered cards but never labels that number as the total matching population. Pagination, totals, shared lane labels/order, swimlanes, or other semantics require a reviewed Query/view version rather than an API-only browser enhancement.

The Task Board preset overrides generic scalar lane presentation with the fixed task-state catalog below. Other Query boards order lane keys through Core's deterministic scalar ascending order with null last; localized display formatting cannot change that order. `order by` controls cards, not lane identity. Lane collapse, focused card, scroll, density, and temporary client-only hiding are device-local.

## Query-board write capability

A Query board is read-only by default. Cross-lane movement is enabled only when Core proves that the group expression is one direct writable source field, the target lane represents one exact valid field value, the row kind has one narrow accepted action, and the caller is authorized against the current revision.

For a Node Board, v1 may enable movement only for a group expression that is exactly one direct literal document property such as `node.document.properties["status"]`. Existing non-null string values form lanes and null forms Unassigned. Moving to an existing lane patches that exact property string; moving to Unassigned removes it through the normal property action. Computed expressions, display titles, aliases, paths, parents, multi-valued values, processor attributes, formulas, and protected/reserved fields remain read-only. Node Board v1 cannot create or reorder lanes because arbitrary string properties have no portable option catalog.

Heading and Template Root rows are read-only board cards in v1. A future direct action may enable a specific field, but visual grouping alone never authorizes heading restructuring or Template role mutation. Generic Query Board v1 has no inline Add card because Query scope/grouping does not identify a safe node parent, document insertion position, or source constructor.

Before commit, Core evaluates the prospective changed row against the same Query. If it would leave the result because of `where`, scope, authorization, domain validity, or limit/order effects, the action preview states that disposition. Success reloads the authoritative result; the client does not pin a moved card into a lane where it no longer belongs.

## Multidimensional-table board

A table board is a table view, not a canonical Query. A shared `weftext.table.json` view uses `layout: "board"`, and `layoutConfig.groupFieldId` must resolve to one current `single_select` field. Each configured option UUID is one lane in authored option order; option labels/colors are presentation and their UUIDs remain value authority. A final Unassigned lane represents missing/null. Multi-select, relation, formula, rollup, rich text, date, boolean, or computed grouping is read-only/unsupported for board v1 rather than duplicating cards or inventing lane mutation semantics.

One record appears in exactly one lane. Moving it to an option lane replaces that exact stored field value with the option UUID; moving it to Unassigned removes the field key. The record identity, other fields, resources, relations, and table sidecar remain unchanged. The action uses the table record transaction and revalidates the record, sidecar/options, view, authorization, current table/workspace revision, and prospective filter before commit.

Add card is available because the owning table and schema are explicit. It opens an inline or Inspector form whose grouping field is preset to the target option and whose other required fields must validate; commit creates one fresh record JSON through the table transaction. It never creates a node. Table boards may show Core-computed complete authorized filtered lane totals separately from loaded windows because specification 20 defines table summaries independently of Query v1.

Private table board view state remains outside the workspace. Saving or changing a shared view previews the exact sidecar replacement. Card edits remain record transactions and never rewrite the view sidecar just because the selected/focused lane changed.

## Task Board preset

Task Board is the specialized Query board produced by choosing `Insert -> Dynamic View -> Board -> Task Board`. It requires `from tasks` and direct `group by task.state`; changing either through Source turns it into a custom Query board on refresh instead of retaining hidden task-board semantics.

Core defines the complete lane catalog and order:

| Order | State | Product label | Closed |
| --- | --- | --- | --- |
| 1 | `todo` | To do | no |
| 2 | `in-progress` | In progress | no |
| 3 | `on-hold` | On hold | no |
| 4 | `completed` | Completed | yes |
| 5 | `cancelled` | Cancelled | yes |

All five lane headers remain available as workflow targets for the preset even when a Query filter currently yields no cards for a state. Completed and Cancelled start collapsed as device-local presentation; their headers remain focusable and expand temporarily during keyboard, touch, or drag movement. If a prospective transition no longer satisfies the authored Query, preview says that the card will leave the current view after commit.

Card presentation is kind-aware:

- checklist: checklist glyph, non-null `task.title`, source-node display, marker state, and only real checklist actions;
- task node: node/task glyph, title, selected authorized priority/due/blocked fields, and typed task-node actions.

Null task-node-only fields on a checklist are omitted, not shown as editable blanks or default values. Source kind is never conveyed only by color. Opening a checklist invokes exact occurrence navigation; opening a task node first makes Task detail available in the right Inspector and may open the ordinary node editor. Card title, state, priority, due, or dependency changes never mutate a cached board row directly.

## Task movement matrix

| Source card | Target lane | Required action |
| --- | --- | --- |
| unchecked checklist | Completed | exact checklist-marker close action |
| checked checklist | To do | exact checklist-marker reopen action |
| checklist | its current open/closed lane | no-op |
| checklist | In progress, On hold, or Cancelled | explicit atomic promotion-and-transition plan |
| task node | any different state | narrow task-node state plan |

A checklist has no representable `in-progress`, `on-hold`, or `cancelled` value. Those destinations remain visible but display “Promote to task node” on hover/focus/drop. Drop opens a compact revision-bound preview; it never changes state on pointer release alone. Cancel returns the card to its source lane.

The promotion-and-transition plan is distinct from two sequential mutations. It binds all ordinary promotion evidence plus the user-selected final state and creates the new task node directly with that state, replaces the exact checklist occurrence by the reviewed stable node link, handles the complete attached branch/annotations, and publishes one commit. Ordinary checklist promotion without a board target retains its existing default state mapping. Until the caller provides this composed action, the three complex checklist destinations are visible but disabled with the promotion-required reason.

A task-node move may use lightweight confirmation policy because it is one narrow reversible field action; permission, stale revision, dirty drafts, dependency/profile invalidity, or transaction recovery still fail closed. Multi-card movement requires one reviewed atomic plan with exact row/card identities and per-kind dispositions; v1 may omit it rather than loop single writes and report partial success.

## Ordering and vertical drag

V1 board position inside a lane is always derived:

- Query Board: canonical `order by` plus the Query domain's stable final tie-breakers;
- Table Board: the view's typed sort plus record-ID tie-breaker.

Vertical drag handles are absent. Pointer movement between two cards, keyboard reorder, or touch reorder cannot create portable or private manual order. If a client uses a transient placeholder during horizontal drag, the committed card returns to its derived sorted position. A manual-rank design must define portable authority, concurrent rank allocation, compaction, mixed checklist/task-node behavior, multiple boards with different ordering, copy/Trash/backup, and exact transactions before UI exposure.

## Creation and editing

Task Board Add card always creates a task node. The request carries explicit parent node UUID, portable name/title input, initial lane state, current workspace revision, authorization, and generated identity/path preview. A `subtree(this.node)` embedded preset may propose `this.node` as parent; the user-visible immutable target is still captured before commit. Workspace scope, Saved Query, Custom Query, current focus, selected card, or visual lane never supplies an implicit parent.

Native checklists are created only where an exact editable document/list insertion point exists. A board may deep-link to “Add simple checklist in document”, but it must open/capture that target before proposing insertion; the column alone is insufficient authority.

Table Board Add card creates one record as specified above. Node/Heading/Template/Custom Query boards have no Add card in v1. Source-specific title editing is available only where an existing Core action can patch the exact checklist principal text, task/node document Title, direct document property, or record primary field. Derived display titles and computed/card-only projections are read-only.

## UI and accessibility

Selecting a Query board exposes Data plus Dynamic View; selecting a table board exposes Data plus Multidimensional Table. No permanent Board tab is added. Minimum controls are source/table, scope where applicable, filter, grouping field/expression, card fields, sort, shared/private status, refresh, Focus view, and Query Source where applicable.

Wide layout uses horizontally scrollable lanes with independently virtualized card lists. Every lane has a real heading, card count label qualified as loaded or total, Add capability where supported, collapse state, and accessible actions. Narrow/mobile layout uses a lane selector and one vertical card list; drag is optional. Focus view uses the same projection/source revision and never copies the board definition.

Every horizontal drag operation has equivalent Card menu -> Move to, keyboard Move command, and screen-reader state selector. Drag start announces card/source/current lane; valid targets and action kind are exposed without hover; promotion and leave-view consequences are announced before confirmation; success restores focus to the card if it remains or to the source lane/status message if it leaves. Escape cancels. Pointer, keyboard, and touch routes submit the same typed action.

IME composition blocks card drag and command interception until composition ends. Virtualization preserves active-descendant/card position semantics, selection, lane headers, and focus across loaded windows. Color never carries lane, card kind, blocked, invalid, or permission state alone. Zoom, high contrast, reduced motion, RTL/mixed direction, long CJK titles, narrow lanes, offline/stale state, and Inspector relationships are required cases.

## Concurrency, non-disclosure, and failure

Board rendering and actions re-use source authorization; the browser never receives hidden cards and then conceals them. Hidden/missing node, task, relation, table, field, lane option, and action target are indistinguishable. Counts, empty lanes, suggestions, diagnostics, drag targets, promotion previews, and Add defaults cannot disclose unauthorized authority.

An action captures its typed target and revision at invocation. Later focus, selection, lane reorder animation, scroll, or refresh cannot retarget it. Commit revalidates authority and prospective result. Success followed by index/projection refresh failure is reported as a successful commit plus refresh warning; retrying the write is not suggested. Failure leaves authoritative bytes unchanged or invokes normal journal recovery and then reloads rather than trusting optimistic UI.

## Acceptance matrix

Release evidence covers:

- the same Core board projection, lane/card ordering, identities, capabilities, unavailable reasons, and source revisions across every delivered caller;
- canonical Query round-trip with required scalar `group by`, source/view separation, exact card fields/order/limit, null lanes, computed/read-only groups, node direct-property moves, prospective leave-view behavior, loaded-count labeling, and absence of hidden portable config;
- table `single_select` option/Unassigned lanes, option order/rename/reorder/delete, record move/add/filter, current sidecar/record revision, exact shared/private view behavior, complete authorized totals, and rejection of multi-select/relation/computed grouping;
- all five task lanes/order/labels, collapsed closed lanes, checklist/task-node visual and capability differences, exact complete/reopen, task-node state changes, promotion-required disabled state until composed promotion is available, then atomic promotion-and-transition with cancel/stale/draft/annotation/rollback/crash cases and no intermediate/duplicate card;
- no vertical manual ordering on pointer/keyboard/touch, stable derived order after refresh, and exact tie-breakers under duplicate/null/Unicode values;
- explicit-parent task-node creation, no ambiguous checklist/Query creation, table-record creation without nodes, direct title/property edits only, and no generic card authority;
- authorization before cards/lanes/counts/diagnostics/suggestions/actions, hidden/missing indistinguishability, Server actor/session binding, replay refusal, and refresh-after-commit failure behavior;
- mouse, touch, keyboard-only, screen reader, IME, zoom, high contrast, reduced motion, RTL, CJK/long text, collapsed target, mobile lane selector, Focus view, virtualization, selection, and Inspector navigation; and
- representative Query boards up to the canonical result limits plus 10,000-, 100,000-, and 1,000,000-record table-board sizing for bounded memory, incremental projection, independent lane virtualization, cancellation, and no unrelated authority rewrite.

Passing a generic drag-and-drop component test or screenshot does not satisfy this contract.
