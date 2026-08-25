---
source_language: zh-CN
translation_of: 06-application-ui.zh-CN.md
translation_status: synced
---

[简体中文](06-application-ui.zh-CN.md)

# Application UI

Status: current architecture contract.

## Required clients

Weftext provides a Desktop application for local workspaces and a WebUI for Weftext Server. The WebUI is not an administration-only console: it supports normal reading, editing, navigation, annotations, and collaboration. The Desktop can also act as a remote Server client.

The first usable Desktop release includes:

- node tree with create, rename, move, copy, item-backed Trash, restore/ancestor-chain previews, and a separately protected permanent-delete flow;
- tabs, split views, back/forward, breadcrumbs, recent nodes, bookmarks, and session restore;
- Write, Source, and Read views that retain the same semantic position when switching;
- current-document find/replace and workspace search;
- document-header Properties editing plus purpose-built `weftext` controls, without destructive YAML reserialization;
- native checklist toggle, checklist-to-task-node promotion, typed task-node controls, and one canonical Query-derived task view over both row kinds;
- optional portable node icons with a visual picker and consistent navigation rendering;
- Chrono root selection and fixed period-node creation;
- highlights, underline, strike, comments, unresolved anchors, and review threads;
- sync/conflict center, transaction recovery, index rebuild, Safe Mode, and diagnostics;
- keyboard operation, command palette, screen-reader semantics, high contrast, reduced motion, and IME support.

The first usable WebUI includes the same core node, editor, search, Chrono, annotation, conflict, and navigation flows, subject to Server permissions. It additionally includes sign-in, session/device management, member and role administration for authorized users, deployment health, audit access, and backup/restore controls.

Core supplies one shared inventory model for managed nodes, visible unmanaged directories/Markdown, node-owned resources, and derived icons. The accepted navigation uses a narrow activity bar for Explorer, Search, and Chrono. Explorer—not the activity bar—switches between Hierarchy and Contents modes, because the managed-node tree and Finder-like current-location list are two projections of the same workspace. No shell may implement either projection with an independent filesystem scan. The complete decision is recorded in [`../architecture/05-shared-navigation-information-architecture.md`](../architecture/05-shared-navigation-information-architecture.md).

### Shared navigation shell

Hierarchy shows the complete managed-node tree. Its disclosure control expands a branch and its name opens the node; one whole-row button must not perform both actions. Contents is a single-location list, initially following the node in the focused editor pane. It shows immediate managed children plus Core-visible unmanaged directories, unmanaged Markdown, and resources. It may browse into a visible unmanaged directory without creating a node tab or changing the current managed document. The Contents header and breadcrumb always distinguish the browsed locator from the focused managed node and provide a return action.

The canonical same-named AsciiDoc document is represented by its managed-node item and is never duplicated. Ignored content is absent from rows, counts, empty states, search, events, errors, thumbnails, recent items, and accessible descriptions. Unmanaged rows have no node UUID and receive no node-only Properties, icon, Chrono, Trash, ordering, annotation, backlink, or editing action. Raw-file editing and general filesystem operations require a separate specification.

Hierarchy and Contents use the same Core-derived parent/child projection and managed-child order. A client may group content classes visually, but it preserves Core order for managed children and stable natural-name order for unmanaged groups. If a caller payload lacks sufficient ordered relationship data, the shared Core/API projection is extended; Desktop and Server WebUI must not implement independent sorting authority. Manual reorder and cross-parent drag/drop are outside this Explorer contract and require separate Core transactions.

The center remains the managed document in Write, Source, or Read. The right inspector owns Outline, Properties, Annotations, Backlinks, and authorized Server permission detail when available; the editor does not add a second permanent child-node surface. Search, Chrono, recents, bookmarks, tabs, and both Explorer modes open nodes through the same UUID action and target the focused editor pane. With split panes, Explorer selection follows focus without discarding either pane's controlled source. Dirty-draft persistence and stale-revision behavior remain mandatory before replacement navigation.

Activity selection, Explorer mode, expansion, scroll, filters, and width are device-local presentation state. Managed navigation identity remains UUID based. An unmanaged browse locator may be restored only after the current Core inventory validates it. Server responses are permission-filtered before projection; the browser never receives hidden items and then conceals them. The identity and session boundary remains Owner-only while multi-role ACL is unavailable.

The shared Explorer navigation contract is implemented in the shared React Desktop/WebUI source and in the Server's same-origin thin WebUI. Core returns a rebuildable version-1 ordered navigation projection derived from the current inventory; Desktop, the CLI bridge, and Server API expose that same projection and derive their compatibility lists from it. Both UI surfaces window large results in 240-row increments and report interaction timing separately from Core scanning. This implementation does not make the Server thin client the complete shared WebUI, add raw unmanaged-file actions, or provide a permissions inspector.

The current WebUI prototype has an explicit local-workspace acceptance mode: a native CLI bridge binds to loopback, grants one selected Weftext workspace, and gives the browser a random fragment-only token. It loads the real node tree, Core block/link model, and external derived search index; supports document read/save, exact structural preview/commit, Trash/restore, properties, portable icons, annotations, resources, Chrono, and stale-revision conflicts. Operational `weftext` YAML remains concealed outside Source. It is not the Weftext Server, does not grant browser directory access, and does not establish Server or collaboration completion.

The Windows Desktop Alpha packages the same interaction source as local static assets in a native WebView2 window. The user grants one workspace through the operating-system folder picker; the UI receives no general filesystem capability. Native commands validate and recover the workspace, restore the most recent workspace and selected node, perform exact Core reads and transactions, and query a rebuildable incremental search index stored outside the workspace. Desktop persists device-local revision-bound drafts outside the workspace and sends stale drafts to Recovery Center before any commit. Safe Mode is persisted locally and enforced by the native transaction boundary, so the UI can continue saving recovery drafts while document and structural commits are refused. Diagnostics expose recovery counts and issue codes without document bodies or complete absolute paths. The shared managed-node editor exposes only canonical AsciiDoc plans; Markdown remains import/export, unmanaged content, or a node-owned attachment. Automated evidence establishes the engineering boundary but does not complete the packaged manual matrix.

Source and structured Write share one exact-source CodeMirror adapter boundary. Source shows an exact AsciiDoc buffer; syntax highlighting, when accepted, is presentation rather than a second parser. Write hides only the exact Core-reported `weftext` frontmatter envelope and receives semantic decorations/ranges from Core; it runs no frontend document parser. CodeMirror may normalize presentation line endings, but every edit and selection maps back to exact controlled UTF-8 source so unrelated bytes remain unchanged. The sole top-level `weftext` mapping is folded by default with an explicit accessible toggle; an invalid or structurally ambiguous envelope fails open for inspection. Neither adapter can parse workspace identity, access files, select a format from extensions, or commit a draft.

## Views

The editor exposes three peer views:

1. Write: direct, structured editing from the selected Core profile model.
2. Source: the actual canonical source with profile-aware labels, diagnostics, folding, and accepted highlighting.
3. Read: non-editable final rendering.

Review is an overlay or mode, not a fourth document representation. All views operate on the same source revision and Core document model.

### Adjacent heading and body presentation

Weftext provides an optional workspace-portable setting that presents an eligible AsciiDoc section heading and its adjacent ordinary paragraph as one run-in visual paragraph. The root node stores the default as `weftext.adjacent_heading_body`; absence means separate presentation. Enabling, disabling, or rendering the setting does not rewrite document body source.

The accepted resolution order is the per-heading `[.run-in]` role, the per-heading `[.separate]` role, and then the root default. Under the root `run_in` default, the body must be an ordinary paragraph beginning on the physical line immediately after the heading; a blank line preserves separate presentation. The complete eligibility and protected-block rules live in [`15-weftext-asciidoc-profile.md`](15-weftext-asciidoc-profile.md). The UI consumes the Core run-in group and never reproduces those rules.

The heading and paragraph remain distinct logical blocks inside a derived run-in group. A terminal block ID on the heading belongs only to the heading; a terminal block ID on the paragraph belongs only to the paragraph. Navigation may scroll the shared visual container, but it highlights and selects the addressed logical block. The outline, table of contents, heading path, heading rename, and heading-link display use only the authored heading text and exclude the body and block ID. Duplicate IDs remain an identity diagnostic and are never reconciled by grouping.

Desktop, WebUI, Server rendering, CLI-derived document inspection, export adapters, search, links, annotations, and accessibility trees must consume the same resolved setting and Core block model. The root-only `weftext.adjacent_heading_body` field is its sole portable workspace-default authority; no client may substitute a device-local interpretation.

### System metadata presentation

- Write hides the reserved `weftext` envelope and does not include it in rendered word counts, selection, copy, or potential-link discovery.
- Read hides frontmatter by default.
- Properties exposes title/header information and safe literal descriptive document attributes. `weftext` fields such as aliases, icon, and order use separate purpose-built node controls rather than ordinary document-property editing.
- Source remains the exact AsciiDoc source. It shows `weftext` in a default-collapsed, clearly labeled system-metadata fold rather than removing or fabricating source text.
- Node information and diagnostics may reveal and copy the node ID and sorting state. Direct system-metadata edits are advanced operations, remain revision checked, and fail closed on invalid identity, duplicates, or unsupported YAML.
- A plain-Markdown export may explicitly offer to remove Weftext metadata. Normal save, synchronization, backup, and Weftext workspace export preserve it.

Hiding metadata is a presentation decision, not access control. Node IDs are not credentials, and Server authorization never depends on a client concealing `weftext`.

### Editor interaction strategy

Weftext adopts proven document-editor interactions without adopting another product's storage model. The first editor sequence is: one controlled node draft shared by Write, Source, and Read; cursor and scroll continuity; cursor-local Core profile formatting; current-document find/replace; outline with current-heading highlight and filtering; quick node open; and workspace search. Focus and typewriter modes follow after Chinese IME and accessibility evidence.

Write is the default ordinary-node editing surface. The Home/Insert groups of its compact top ribbon expose undo/redo, a semantic paragraph/H1-H9 selector, format painter and clear-format actions, inline emphasis, lists and checklists, quote/admonition, code, mathematics and diagrams, links and node links, node-owned images, tables, and footnotes/endnotes as the profile gains those capabilities. A selection bubble, slash menu, context menu, keyboard shortcuts, and command palette may offer the same actions; none owns separate mutation logic. Document title and optional subtitle remain dedicated header controls because they are document-header fields, not choices in the body-heading selector.

Every surface resolves through the semantic action registry and a Core-reported selection capability. A toolbar button cannot infer validity from rendered HTML or manufacture AsciiDoc punctuation. Unsupported, protected, malformed, mixed-kind, or stale selections disable the action and expose a concise reason. The toolbar remains keyboard reachable, communicates pressed/indeterminate/disabled state to assistive technology, survives narrow layouts through an accessible overflow menu, and does not steal an active IME composition.

The format painter copies a typed Core `FormatDescriptor`, not CSS, HTML, literal delimiters, or arbitrary source. One activation applies to the next compatible target; double activation locks it for repeated use; `Escape` cancels it. The shared shortcut defaults are `Ctrl+Shift+C`/`Cmd+Shift+C` to capture and `Ctrl+Shift+V`/`Cmd+Shift+V` to apply, while a Repeat Last Format action reuses the last compatible descriptor. It may copy semantic inline marks and compatible paragraph/list/quote/heading presentation. It never copies content, link destinations or node UUIDs, anchors/IDs, citation or note bodies, comments/annotations, task state/dates/dependencies, Query meaning, Template slots, or resource paths. Incompatible target kinds fail closed with an explanation; a successful application is a narrow `DocumentFormatPlan` against the controlled draft.

Current-document find uses literal Unicode text. Write and Read search only the visible document body; Source searches the complete exact source because the user explicitly chose that view. Replacement is an exact splice of the controlled draft, never an implicit workspace commit, and therefore inherits device-draft recovery plus the normal Core preview and base-revision check. The outline excludes frontmatter and fenced content, uses Core heading blocks and UTF-8 source ranges for live workspaces, and treats any frontend-only extraction as simulated-prototype fallback rather than format authority. Switching nodes and Write/Source views restores source selection and scroll from device-local state keyed by workspace and node UUID. Source fold expansion is also persisted by workspace/node UUID, and a restored selection inside the system range expands it before positioning.

Structured affordances edit canonical AsciiDoc tables and insert node-owned image resources through typed Core Profile plans plus resource transactions. Images become resources owned by the selected node only after a visible target preview; the canonical reference is planned before the resource commit so a syntax error cannot leave an invitation to retry an already-successful import. Table operations are restricted to the Core-reported table block and must not reformat unrelated source. Math, diagrams, import/export, canonical Query collections, Template Designer, spelling, and stylistic themes are outside the current editor implementation. Their planning boundaries are recorded in [`../architecture/06-content-io-and-rich-rendering.md`](../architecture/06-content-io-and-rich-rendering.md), [`../architecture/07-collections-query-and-views.md`](../architecture/07-collections-query-and-views.md), and [`../architecture/19-expression-query-and-template-library.md`](../architecture/19-expression-query-and-template-library.md); those contracts do not claim current rendering, conversion, OCR, Query, Template, or multi-view implementation.

Table actions operate on a Core-resolved logical selection, not on DOM coordinates. Selecting the first contiguous `N` rows may mark the column-header region; selecting the first contiguous `N` columns may mark the row-header region; the two regions may coexist and their intersection remains one set of cells. A non-leading or discontinuous selection cannot be declared a header region. The context menu names both the action and its effect, for example “Use first 2 rows as column headers”, and the table inspector exposes the same controls without requiring right-click.

A rectangular cell selection may merge only when every selected cell belongs to one table and the rectangle does not cut across an existing span, protected range, or unsupported cell structure. With zero or one non-empty cell, merge is direct. With multiple non-empty cells, Weftext presents the exact row-major content-composition preview and requires confirmation; it never discards content. Split restores the span's rows and columns, keeps the merged content in the leading cell, and creates empty remaining cells. Split does not pretend to recover the earlier distribution; immediate Undo is the exact way to restore the pre-merge grid and contents. Invalid merge/split actions remain visible but disabled with the Core reason.

Heading editing uses one clear H1–H9 level selector in each editable pane instead of nine adjacent buttons. A heading context menu offers Change level, Convert to paragraph, Copy section link, and Set/remove explicit anchor where the profile permits them; toolbar, keyboard, and command-palette routes remain equivalent. Changing a heading with descendants requires an explicit `heading_only` or `preserve_subtree` policy and previews the resulting outline before application. The subtree policy shifts descendant levels by the same delta and is unavailable if any result would fall outside H1-H9; the heading-only policy is unavailable when it would produce a hierarchy rejected by the profile. Paragraph conversion that would reparent descendant sections also requires a structural preview. Neither action changes heading text, anchors, links, or annotations unless that field is itself the explicit action target. Quote controls increase or decrease the Core-reported nesting depth by one and present the resulting level; they never infer depth from frontend styling. Read uses native `h1`–`h6`, accessible role headings for H7–H9, and genuinely nested block quotes. The outline and heading path show the actual level without truncation.

Path-based node identity, treating Markdown as a node, a frontend-only Markdown parser, and silent canonical-file autosave are explicitly not adopted. Rule-classified unmanaged Markdown remains byte-preserved and has no node action/transaction affordance. A `.md` inside a managed node is an ordinary node-owned attachment with the same resource actions as `.txt`, images, and PDFs; its contents do not enter node semantics. Device-local drafts may autosave for managed-node crash recovery; committing them to the workspace remains revision checked.

Tabs are UUID sessions rather than path bookmarks. Opening a node already present in another tab selects that tab instead of creating two state owners for one node; each tab therefore retains its node's view, selection, and scroll state under workspace UUID plus node UUID. Closing an active tab selects an adjacent surviving tab. Back/forward, actionable breadcrumbs, recents, and bookmarks store node UUIDs only. The optional second pane has its own node UUID, view, selection, scroll, find/replace, and controlled draft state. Workspace paths remain current locators supplied by Core and are never persisted as navigation identity.

On Desktop or live WebUI restart, persisted navigation is restored before choosing the opened document. If the active tab UUID is still valid but differs from the backend's last remembered document, the client opens the active tab UUID through the normal Core document-read boundary and only then presents it. The active tab, selected node, visible document, and editor state must never disagree.

Each editable pane has an explicit revision-checked save/preview entry for its own node and controlled source. Closing or switching a dirty second pane retains its device draft and communicates that state; it never silently discards it. A current recovery draft may populate that pane and remain dirty. A stale recovery draft never replaces the disk source automatically: the pane shows the current disk document, surfaces the stale draft in the same Recovery/Conflict boundary as the primary pane, and requires an explicit recovery choice before preview/commit.

### Ribbon, Data, Tasks, and References

The ribbon contract defines one compact, collapsible top control with six persistent tabs: Home (`开始`), Insert (`插入`), Data (`数据`), References (`引用`), Review (`审阅`), and View (`查看`). Data covers node collections, task views, heading/template views, multidimensional tables, filters, grouping, sorting, and summaries. References covers citations, notes, captions, cross-references, and indexes. A paper is represented by a template or workflow, not a node type.

Table, Image, Dynamic View, Task, and Template Design tabs are contextual and appear only for a compatible Core-resolved selection. Selecting a native table exposes header-row/header-column, merge/split, caption, insert/delete, and upgrade actions. Selecting a multidimensional table exposes its fields and record-view commands. Selecting a dynamic view exposes the canonical Query builder and current layout without changing its source domain. Selecting a checklist or task node exposes frequent state, priority, due, and dependency commands; this is not a permanent seventh tab. Every command delegates to the same semantic action registry and typed Core plan used by context menus, slash commands, shortcuts, selection bubbles, and the command palette.

The right Inspector owns durable contextual detail: Outline, Properties, Annotations, Backlinks, Task detail, Citation/Reference detail, and permitted Server access when available. A task-node header may show compact state/priority/due chips, while the full form remains in Task detail; a native checklist shows only supported simple controls plus promotion. Selecting a citation opens occurrence and resolved-reference detail, opening a reference node exposes bibliographic fields, and document scope offers citation/bibliography diagnostics. Review remains comments, suggestions, spelling, and change resolution rather than absorbing citation workflows.

There are two visibly separate Data creation paths. New Multidimensional Table creates one managed table node whose rows are non-node JSON records. Insert Dynamic View writes one canonical `.weftext-query` block for an explicit Nodes, Tasks, Headings, or Templates source. The builder edits the exact Query at the cursor and never saves a hidden parallel frontend configuration. Temporary filter, density, width, selection, and scroll may be device-local; a portable field/filter/group/summary/layout change is offered only when the current versioned Query source can represent it. The complete file, view, native-upgrade, ribbon, accessibility, and transaction contract is [`20-multidimensional-tables-and-editor-surfaces.md`](20-multidimensional-tables-and-editor-surfaces.md).

Board is one reusable view layout inside this Data surface. `Insert -> Dynamic View -> Board` offers Task Board, Node Board, and Custom Query Board; a multidimensional table instead uses `New view -> Board` inside that table. Task Board is a preset that authors `from tasks`, direct state grouping, selected fields, ordering, and `view=board` into one exact Query. It renders native checklists and task nodes together while showing their different capabilities. Moving a checklist only between To do/Completed patches its exact marker; moving it into In progress/On hold/Cancelled requires a visible atomic promotion-and-transition preview. Task-node movement patches its real state. Table Board movement patches one `single_select` record field. A computed/read-only Query group never gains drag merely because it appears as a lane.

Horizontal movement changes only a typed lane field. V1 offers no vertical manual card order, swimlanes, WIP limits, silent checklist promotion, generic card database, or drag-only action. Query `order by` or table-view sort remains within-lane authority. Every drag has Move to, keyboard, touch, and screen-reader equivalents. Completed/Cancelled task lanes are collapsed presentation by default, card source kind remains textually distinguishable, Add card creates a task node or table record only where the target constructor is explicit, and the selected card's real source detail remains in the right Inspector. See [`21-board-views.md`](21-board-views.md).

### Standalone AsciiDoc editor

Desktop may open an explicitly selected `.adoc` or `.asciidoc` file outside every Weftext workspace as a **Standalone AsciiDoc** document. File association, Open File, drag-and-drop, and recent-file reopening all enter the same single-file grant. Opening the file never scans its parent/siblings, looks for `.weftext-format`, adopts the directory, creates a node UUID or `weftext` envelope, creates an annotation sidecar, or turns relative paths into workspace authority. Markdown files are not accepted by this editor mode; they remain attachment, unmanaged, import, or export inputs.

Standalone parsing uses the shared safe AsciiDoc syntax/model/rendering/formatting engine without managed-node envelope validation. Generic AsciiDoc remains exact source. Weftext-only constructs such as `node:` links, canonical Query, Template roles/slots, task-node profiles, portable annotations, and workspace citation resolution are preserved as authored bytes but shown as unavailable rather than executed, resolved, normalized, or removed. “Add to workspace” is an explicit import action that creates a new canonical node with a fresh UUID and never rewrites the external source in place.

The six persistent ribbon tabs retain their positions so the UI does not jump between modes. Each disabled group remains visible with the reason “Requires a Weftext workspace”:

| Tab | Enabled for a standalone AsciiDoc file | Disabled or reduced |
| --- | --- | --- |
| Home (`开始`) | save, undo/redo, document title/subtitle, paragraph and H1–H9 formatting, format painter, clear format, emphasis, lists/native checklists, quote/admonition, code, find/replace | node properties/icon, durable-task fields, node actions, workspace save/commit |
| Insert (`插入`) | ordinary URL/file links, current-document anchors/xrefs, images by authored relative locator, native tables, code/literal blocks, mathematics/diagram source, footnotes | node link/embed, Chrono note, Dynamic View, Template instantiation, resource ownership/import into a node; copying an asset beside the file waits for a separate reviewed external-file action |
| Data (`数据`) | no v1 commands; the tab remains in place as an explanatory disabled surface | node/task/heading/template Query, multidimensional tables, boards, workspace sorting/filtering/grouping/statistics |
| References (`引用`) | current-document cross-references, anchors, captions, footnotes/endnotes, index-term source where the safe profile supports it | workspace citation picker, reference-node search/editing, bibliography resolution, cross-node references |
| Review (`审阅`) | spelling/style checks, compare draft with saved file, standalone recent history when available | portable comments/annotations/suggestions, workspace review queues, ACL/audit actions, formal workspace backup restore |
| View (`查看`) | Write/Source/Read, outline, split view, zoom, focus/typewriter, wrapping, theme, full screen | Explorer/Search/Chrono, node Properties, Backlinks, Graph, Tasks, workspace Version History and Backup inspectors |

Contextual Table and Image tabs appear for generic native selections. Table header/merge/split and other structurally safe actions remain available through exact standalone formatting plans; heading-level actions remain available from Home and the heading context menu. Image source attributes may be edited, but node-resource ownership actions are unavailable. A native checklist exposes only identity-free toggle/edit controls; promotion and durable task fields are unavailable. Dynamic View, Task-node, and Template Design contextual tabs do not appear. No disabled feature may be simulated by a frontend-only parser or hidden sidecar.

Standalone Save binds the originally granted normalized file path, a file-object identity where available, and the exact opened digest. It stages in the same directory, preserves applicable permissions, flushes, atomically replaces, reopens, and verifies the exact bytes. An external modification, replacement, link/reparse transition, permission loss, or target disappearance opens Compare/Save As and never overwrites silently. Device-local drafts and recent history key by a privacy-preserving file identity and do not create sibling files. Closing the last standalone tab releases the file grant.

### Version History, comparison, and backup status

The product calls revision history **Version History**, reserving Chrono for date nodes and Timeline for data/view layout. Review owns Compare; the right Inspector owns Version History. Compare offers Previous version, Version History, Another document, Backup snapshot, and Resolve draft/external conflict. It supports inline/split modes, exact source/structural context, system-metadata fold, change navigation, and keyboard/screen-reader Use left/Use right/Keep both actions. Comparison is read-only until the user creates one controlled result draft; final merge/restore is one revision-bound Core plan and never requires Git.

Version History merges available `This device`, `Backup`, `Server`, and `Protected restore point` entries while keeping provenance and restore completeness visible. Recent History is enabled by default outside the workspace and labeled `This device only`; it does not follow workspace sync. Restoring one document source explicitly excludes resources/sidecars unless a complete formal backup action is selected. Comparing two nodes folds identity-only system differences by default and never copies `weftext.id` into the target.

Backup settings ask for Backup destination and Run backups by (`This device`, `Weftext Server`, or `Read only`). Destination choices are capability-versioned local/external/network/Server/supported-remote backends rather than an arbitrary URL. A raw directory repository has one writer. Target, executor, schedule/retention, last success, last verification/clean restore, pending changes, and redacted availability remain visible.

With no configured target the UI shows `Backup: Not configured`, `Recent History: This device only`, and `Last formal backup: None`. Setup is non-blocking and Not now remains available, but drafts, Trash, local history, sync-provider history, or a same-disk folder never produce a green/safe state. The complete store, repository, compare, merge, restore, accessibility, and non-disclosure contract is [`22-document-history-comparison-and-backup-repositories.md`](22-document-history-comparison-and-backup-repositories.md).

### Workspace item icons

Node icons are optional navigation presentation stored as one scalar in `weftext.icon`. The scalar is either one literal emoji or one stable Weftext-owned built-in token. Desktop and WebUI use the same Core resolver and supported-value catalog, so a shell cannot invent a separate icon store or write an icon outside a revision-checked node action.

The icon-picker contract provides a searchable visual picker for emoji and Weftext-owned built-in symbols. Choosing from the picker replaces the scalar `weftext.icon` value through a narrow patch, and clearing removes that field. A commit response returns the Core-resolved icon so Source edits refresh navigation without a frontend YAML parser. The UI offers three compact-list placements—before the node name, after the node name, or hidden—and a separate toggle for showing the icon beside the opened document title. Placement and title visibility are application or account preferences because different users and devices may choose different density; the selected node icon itself remains workspace-portable operational metadata.

The same resolved icon is used in the node tree, quick open, workspace-search results, recent nodes, and bookmarks when those surfaces display icons. Unknown tokens remain exact source but resolve to no explicit icon. Icons are decorative and never replace the accessible node name. Size, baseline, spacing, contrast, and title scaling come from semantic UI tokens rather than arbitrary per-user pixel values.

The display priority is shared Core semantics: a supported explicit node scalar; otherwise the Weftext default managed-node icon; otherwise the normal folder, Markdown-file, or ordinary-file icon for unmanaged content; otherwise existing special root/Trash state presentation. A default is derived UI state and is never written into `weftext.icon`. The current icon contract does not inherit an ancestor icon or interpret a resource image as an icon. Unsupported scalar values remain visible in Source and node diagnostics but do not produce a broken-image or missing-symbol placeholder.

Managed-node content items carry an optional UUID populated for that class; unmanaged items carry none. Every item has a root-relative locator and lexical parent locator, and resources may carry the owning managed-node UUID. The canonical `X.adoc` of a managed node is represented by the node item and must not be shown again as an ordinary file. Unmanaged rows are non-node presentation: create/move/copy/Trash/Chrono/icon/property actions are unavailable for them.

### Themes

All components use semantic design tokens for canvas, surface, text, border, focus, selection, annotation, warning, conflict, success, and disabled states. The interaction prototype validates those tokens with one reference appearance; additional stylistic preset families are outside the current contract.

Packaged Desktop acceptance covers light, dark, and high-contrast system themes against editor and transaction states. A theme cannot convey identity, authorization, conflict, or annotation state through color alone, and it cannot change Core behavior.

## Actions

One semantic action registry supplies context menus, buttons, keyboard shortcuts, command palette, CLI, and Server API. Each action defines its typed target, authorization, availability, input fields, preview, confirmation policy, transaction construction, structured errors, and undo/recovery behavior. UI code cannot implement direct file moves, ad hoc link rewriting, or a second target-selection rule.

The registry uses the single action-target resolver from the shared navigation architecture. An editor/current-node command captures the focused pane's `focusedNodeId`; a concrete tree/content/resource/Trash row action captures that row's explicit UUID or `trashItemId`. Create-under-current-node and current-node Chrono commands use the same focused UUID. The captured identity is immutable through asynchronous preview and commit: a later focus or selection change cannot retarget it, and a stale or unavailable target fails instead of falling back to the primary pane or current Explorer selection.

The user-facing registry exposes separate entries and confirmations for **Rename current node**, **Move entire node branch**, **Copy entire node branch**, **Move entire node branch to Trash**, **Restore Trash Item**, **Move node-owned resource to Trash**, **Permanently delete Trash Item**, **Promote checklist to task node**, **Create from template**, and every Template role-boundary conversion. Rename is not a hidden mode of Move, and Trash never displays a destination-parent or target-name field. Restore shows a target or alternate-name field only for a Core-provided explicit-target choice. Resource removal accepts a Core-owned resource identity, not an unmanaged-file locator. Permanent deletion has its own destructive styling, higher authorization, byte-bound second confirmation, and cannot share the ordinary move-to-Trash control. Internal action IDs and raw names such as `move`, `copy`, `trash`, or `restore` are not accessible labels.

A node action operates on a **node branch**: the same-named canonical document, every descendant node, and the node-owned resources and annotation sidecars in that complete managed subtree. It does not mean only the document currently visible in the editor. Destination pickers may say “This branch and its descendants cannot be selected as the destination”; they must also say “The operation still includes the complete branch.” Accessible wording must distinguish destination filtering from operation scope.

Every rename, move, copy, node-to-Trash, and node-restore preview and confirmation renders Core's closed, versioned `scopeSummary`: `rootNode: { nodeId, displayName }`, `descendantNodeCount` excluding the root, `resourceCount`, `annotationSidecarCount`, exact `byteTotal`, canonically ordered `affectedDocumentNodeIds` and `rewrittenDocumentNodeIds`, `identityPolicy`, `trashItemCount`, and nullable `operationId`. For an ancestor-chain restore, `rootNode` is the outermost node restored by the atomic plan, while the captured action target still identifies the item the user selected. The exact plan still carries byte ranges/replacements, identity mappings, item IDs, and paths where needed; the summary cannot be recomputed from visible rows. Rename, move, Trash, and restore show `identityPolicy: preserve`; copy shows `identityPolicy: rekey` and the complete old-to-new UUID mapping. Trash counts are item counts: one multi-descendant branch creates and displays one item, while a resource batch creates one item per independently restorable file under one `operationId`. Create/import and resource-only or heterogeneous multi-item permanent-delete plans keep their dedicated typed evidence; they must not fabricate a preserve/rekey node summary when no single operated node branch exists.

Before preview becomes executable, the shared mutation boundary intersects Core's exact draft-sensitive UUID set with the authoritative current device/session draft registry. That set comprises every node in the source branch, every node whose canonical document the plan replaces or rewrites, and any preserved node/owner identity whose Trash resource or item is being removed, restored, or permanently deleted. A dirty source root, dirty descendant, or dirty link-rewrite document blocks with the exact node list and requires an ordinary revision-checked save or an explicit draft discard followed by a fresh preview. Copy cannot opt into copying the older disk bytes, and Trash cannot remove an active node while its draft survives as an unhandled edit. Unrelated dirty nodes do not block.

Commit repeats the same scoped draft lookup immediately before applying the revision-bound plan. A draft that becomes dirty after preview produces a typed conflict and no write; the client cannot satisfy the gate with a cached boolean or a confirmation override. Saving normally changes workspace/document evidence and therefore requires replanning. Discard means deliberately removing the identified draft through the draft boundary, not ignoring it for this action. Desktop, local WebUI, hosted WebUI, CLI DTOs, and Server DTOs preserve the Core scope and draft-gate evidence without renaming, dropping, or independently calculating fields.

Destructive, bulk, cross-workspace, permission, and external-egress actions require an explicit preview. Copying text or links does not mutate the document unless the selected action explicitly creates a stable anchor, and the clipboard is updated only after that transaction succeeds.

### Checklist and task-node surface

A native checklist row exposes exact toggle and **Promote checklist to task node**. Promotion is available from the context menu, an always-keyboard-reachable row action, the command palette, and matching caller surfaces; right-click is never the only path. The command captures the exact owning node/revision/source occurrence once and does not follow later editor focus or selection.

Preview shows the generated task-node UUID, title, portable name, default owning-node parent or explicitly selected active parent, initial state, exact content lifted from continuations/descendants, annotations that will reanchor or block, the exact `node:` link replacement, scope/bytes, and draft conflicts. `[ ]` maps to `todo`; `[x]`/`[*]` map to `completed` without inventing a close timestamp. A name conflict requires an explicit alternate name or parent; no client silently suffixes one.

After commit the original location is a normal list item containing the stable node link, not a checkbox. UI may enrich that resolved link with current task-node state, priority, or due date, but clicking its status edits the target node attribute and clicking its label opens the target. The referring source never caches a second task state, and a renamed target does not silently rewrite an authored link label.

Task-node detail uses the ordinary node editor, Properties projection, resources, annotations, links, history, permissions, structural actions, and Trash/restore. The task inspector exposes the closed `weftext-task` profile through typed controls and exact preview/commit rather than asking users to edit raw attributes. Native checklists and task nodes appear in one task view with visibly distinguishable kinds; unavailable checklist fields remain empty rather than receiving fabricated defaults. Recurrence controls are absent until a task-node series/occurrence contract is accepted.

### Canonical Query and Template surface

The Query editor and result views consume only Core's canonical `.weftext-query` model. Source selection comes from the body `from` clause; the block `view` attribute changes presentation only. Row fields use the authored explicit alias. `this.node`, `this.document`, `this.heading`, and `this.query` are lexical parser context and never follow the active pane, selected row, or focused editor. A Saved Query that uses `this` exposes its explicit binding; a missing binding shows `missing_context`. The UI does not rewrite a null heading to the document title or silently widen `section(this.heading)`.

The Template Library UI contract uses a dedicated projection of the configured root UUID. The Library container, each direct-child Template Root, and every owned Template Part remain visibly distinguished from ordinary nodes; a Part displays its owning Root and cannot be instantiated independently. Nodes, tasks, headings/outlines, citations, search, graph, Chrono, default backlinks, recents, and other ordinary views do not mix these roles into their result sets. When role-aware inventory is unavailable, the picker/result surface shows `domain_unavailable` rather than an empty Library or path-derived rows.

The Template Designer contract has three peer views over one controlled exact subtree:

1. **Design** renders typed inline/block slot chips, converts a selection to a parameter, inserts inline or block slots from slash actions, shows parameter/schema/binding/default/example controls, and performs parameter/slot rename through one exact Core transaction.
2. **Preview** accepts sample typed input/context and displays the complete proposed generated subtree without writing.
3. **Source** shows exact Template Root/Part `.adoc` and `weftext.template.json` bytes, profile ranges, and diagnostics without regenerating either representation.

Diagnostics identify unused or missing parameters/slots, type mismatch, duplicate `(scope UUID,name)` declarations, illegal inline/block kind, protected placement, residual slot/profile/sidecar, invalid generated `node_name`, unresolved links/resources, ACL, drafts, staleness, and bounds. Repeated occurrences of one valid declaration are intentional chips and receive the same once-evaluated value. The Designer never evaluates Query; query-derived data must arrive as an already authorized frozen typed input.

Create Node offers **Blank** and **From Template**. From Template lists only authorized Template Roots, builds a schema form, and provides live no-write full-tree preview. A Root `node_name` binding supplies the displayed resolved target name and removes the override control; without one, the form requires one explicit caller name. A Part binding supplies its name, otherwise the prototype basename does. Preview shows the single name authority, node/resource counts, fresh UUID/link mapping, resource copy, design-annotation omission, collisions, ACL, and draft blockers, then submits one recoverable transaction. Moving into, out of, or between Template roles uses a separate conversion preview that shows role/owner and sidecar/source changes and materializes/deletes active slots before ordinary output; it never uses inert ordinary syntax as a path-only downgrade. The Template UI and engine are not implemented.

### Trash item and restore surface

Trash is a dedicated projection of Core's `.weftext-trash/_weftext.items` inventory, not a view of hidden managed children and not a browser filesystem scan. Each row is keyed by temporary `trashItemId` and shows kind, original name, deletion time, byte total, operation group, permanent node ID or original owner ID as authorized, and one of: origin active, origin in Trash, origin missing, origin unknown, or reconciliation required. Product surfaces never expose `_weftext.items` as an ordinary directory, never allow editing `_weftext.trash-item.json`, and never infer an origin from a displayed path/name.

Deleting a node previews one item for the complete root subtree, not one row per descendant. Trash badges, lists, counts, projections, and confirmations are keyed and counted by `trashItemId`; descendant nodes inside a payload never become rows or inflate a deletion count. Deleting multiple resources previews separate independently restorable items grouped under the same `operationId`, with the exact owner, filename, size, and digest for each. A boundary-crossing subtree, unmanaged/ignored file, reserved sidecar/document, unsafe path, item-ID collision, or changed workspace revision blocks the final control rather than offering a partial deletion.

Restore presents only Core-provided choices:

- restore to the original parent/owner when its UUID is active and the exact/case-fold target is free;
- restore the reviewed parent-item chain and selected item atomically when every required origin item resolves uniquely; or
- select an existing authorized target and, when necessary, type an explicit alternate portable name.

An unknown/missing origin defaults to “leave in Trash”; the primary action cannot create a guessed B1 or overwrite a same-named node/file. The confirmation shows every item/ancestor consumed, destination, rename, identity, conflict, base revision, and resulting plan ID. After commit the client reloads authoritative inventory rather than removing rows optimistically. Malformed, partial, duplicate, tampered, or sync-conflicted items offer reconciliation/diagnostic actions only.

Permanent deletion is visually and semantically separate from ordinary Trash and restore. It requires the higher Server/local policy capability plus a second confirmation bound to exact item IDs, payload digests, and total bytes; retention cleanup uses the same reviewed Core action. The UI states that Trash is synchronized deletion state, not backup. Desktop, local WebUI, hosted WebUI, CLI, Server API, and approved agent surfaces consume the same plan types and availability reasons.

## Agent sessions and approvals

An agent-enabled Desktop or WebUI provides a dedicated session surface rather than mixing harness output into document content. It shows the harness and adapter version, model/profile label when available, connection and execution state, selected context scope, tool calls, pending approvals, committed outcomes, cancellation, and reconnect or resume status.

Every mutation approval shows the acting user, agent origin, target nodes, base revision, requested capability, deterministic change preview, conflicts, and external-egress implications. The UI must distinguish a proposal, an approved but uncommitted action, a committed Core transaction, and plain generated text. Closing or cancelling a session cannot be presented as rollback of an already committed transaction.

DSH is the first-tier harness for this surface. Behavioral parity is required for the supported local and Server paths, but DSH-specific controls must remain behind harness-neutral view models so another harness does not require a second action or permission UI. Agent transcripts and hidden context are not saved into portable workspace content unless the user explicitly exports selected output through a normal transaction.

## Node navigation and moves

Moving a complete node branch offers explicit “move into this node” and “move beside this node” choices. The preview shows the Core scope summary, old and new parent, path changes, affected links, conflicts, identity preservation, draft blockers, and required permissions. The target list excludes the source branch because it is not a valid destination; that exclusion never narrows the branch being moved. A move does not infer or alter user-authored semantic relationships.

## State scopes

Every setting declares its scope:

- application: local installation defaults;
- workspace portable: open, synchronized workspace configuration;
- device local: window layout, caches, and device capabilities;
- server/account: identity, roles, sessions, and hosted-workspace policy;
- secret: OS or Server secret storage, never workspace files.

Locked or unauthorized content must not leak through recent lists, search counts, thumbnails, window titles, diagnostics, or restored UI state.

## UI completion evidence

Component screenshots and source tests are insufficient. Acceptance requires packaged Desktop workflows and supported-browser workflows using real Core transactions, including Chinese IME, keyboard-only navigation, accessibility checks, failure recovery, and cross-client behavioral parity.
