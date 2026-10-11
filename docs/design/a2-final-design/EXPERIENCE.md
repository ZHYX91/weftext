---
source_language: zh-CN
translation_of: EXPERIENCE.zh-CN.md
translation_status: synced
---

[简体中文](EXPERIENCE.zh-CN.md)

# Complete writing, research, query and output workflows

Status: design revision candidate, not implemented; consumes [authoring](AUTHORING.zh-CN.md) and existing D1–D10. This document specifies observable interactions and necessary boundaries; implementation validates component styling and dimensions. Research feature names do not introduce another domain model. Included means included in the candidate, not already supported by the product.

## 1. Shared rules

Desktop, WebUI, CLI and approved Agents use the same Core; WebUI writes and downloads pass current Server authorization. UI exposes available actions and reasons for unavailability, without shrinking legal Source to available widgets, replacing authorization with hidden buttons, or calling a preview saved. One Node + Field + Query + View + Action model serves tasks, database-like tables, boards, calendars and relations without another database or duplicated task details.

Every workflow preserves selection, unsaved Draft, Undo and source context; changing views does not commit. Async results bind the original target/operation and cannot apply automatically after revocation, expiry or Base changes. Personal layouts, recents and panel ordering are device preferences, not changes to author order, Query sort, shared authorization or frozen results.

## 2. Writing and reading context

Query rows, backlinks, search and annotations can open source in a side preview or split while preserving list position, filters, selection and Draft. Returning restores UI context and freshly authorized content, not old result handles. Missing, unreadable and stale-range sources get distinct states without guessed reanchoring.

The outline supports heading filtering, current position, folding, section focus and returning to the whole document. It consumes D2 heading projections including native/Weftext deep headings, RTL and titleless notes; focus does not delete/reorder source. Focus and typewriter scrolling are independent switches. IME composition, assistive technology, keyboard navigation and deliberate user scrolling take priority over automatic scrolling. Source and rendered modes share one Draft; missing exact mappings lead explicitly to Source, not equal-text coordinate guesses.

Personal named reading/writing/review layouts store panel geometry, view choice and re-resolvable navigation targets, never permissions, results, runtime selectors or substitute Drafts. Opening reauthorizes and rereads; unavailable panels retain explanations and unsaved drafts survive layout cleanup. Cross-device layout sync and live multi-user cursors require separate designs.

## 3. Quick capture and property editing

One capture panel provides parent, type/template, optional title, business properties and body, showing real destination and required conflicts/missing values before creation. Selecting a template only prepares D9 results; confirmation uses original creation/installation. Save-and-add-another resets only after the original save result. Network unknown preserves the original request and Draft rather than creating a duplicate. Shortcuts do not bypass authorization, preview or confirmation.

Property panels group/search/pin common fields and distinguish authored, derived read-only and unavailable values. Simple fields use native [authoring](AUTHORING.zh-CN.md); complex values expose individual occurrence notes/provenance. Exact source changes are inspectable. Labels and frequency suggestions are not FieldIds; device recents update only after successful commit, not cancellation/hover. Identity is read-only; type edits follow full constraints.

Relation pickers expose authorized candidates and direction; selection previews actual Field/inverse changes rather than drawing client-only edges. Rollup wizards construct existing read-only Query projections. Intranet forms based on existing Facets/templates reuse this panel/Core under the current submitting user. Anonymous public forms, submitting as others and cross-object multi-step macros are outside this candidate.

## 4. Query, tasks and multidimensional views

List search, visual filters and optional textual shortcuts produce the same Query. Query editing separates what data from View presentation, exposing definition, parameters, scope and completeness. Saved views remain existing Document occurrences rather than persisted copies of results. Saved-search and Trash Query global P1 findings remain unresolved; new UI does not close them.

A task is an ordinary Node with task classification. Details are its body and status/due are original Fields. Task tables, smart lists, boards and calendars consume the same Query/Fields. Completion, card dragging and due edits show precise field changes through original Actions. Without a uniquely proven field mapping, drag submission is unavailable with explicit editing offered. Aggregate/read-only/hidden/stale rows are not writable objects.

Query examples cover incomplete tasks, due ordering, related projects, read-only summaries and period ranges with explicit empty, missing, error, partial-index and complete states. Displaying 200 rows is not complete membership; bulk actions retain original full-target/budget rules without silent splitting. No arbitrary JavaScript/SQL/Datalog entry point is added.

Period navigation offers previous/next/parent; calendar clicks select existing objects first and only an explicit create action makes missing notes. Existing calendar/period/series-scope semantics are reused without inferring new facts from today's date or host time zone. Rule previews expose calendar, time zone, range and inability reasons. Delivered reminders, snooze, completion-relative recurrence, a personal My Day set and natural-language dates remain separate topics, not assumed from Calendar/Automation's existence.

## 5. Annotations, citations and links

Reading connects original text, personal annotations and citation sources. Annotation content stays in independent `weftext.annotations.json`; discussion state and target resolvability display separately. PDF/image region selection→annotation→explicit citation insertion→return to original region uses existing D3/D8/D9 coordinates/source/permissions. Missing providers or incompatible coordinates retain the annotation/status rather than screenshot-based reanchoring. Unsaved annotation drafts retain their existing lifecycle.

Outgoing links, incoming links and potential mentions are separate sets. Real links come from parsed source occurrences; suggestions do not count as graph edges, actual links or reference completeness. Link health diagnoses missing targets, stale ranges, no inbound links and isolation. A Workspace-wide no-inbound conclusion requires authorized complete relevant scope; otherwise report none found in the visible scope, without exposing hidden-node counts or automatically removing apparently isolated objects.

### 5.1 Potential links in body text

A device-level show-potential-links setting defaults off and never edits source when toggled. First-version terms come from currently authorized live Nodes' authored titles and explicit schema-recognized aliases, not arbitrary similarly named attributes, history/Trash or hidden nodes. Unavailable alias sources explicitly narrow the scan rather than claiming completeness. Deduplicate candidates by full NodeRef, not title; exclude the current Node by default.

Match ordinary visible text fragments with precise root-authored writable D2 origins using exact Unicode-scalar literals, without fuzzy matching, case folding or normalization. Do not decorate code, math, URLs, existing links, YAML/attribute values, annotations, generated/include/multi-origin/passthrough content or unproven writable ranges with conversion suggestions. Do not join words across fragments. Matches carry complete `[start,end)` source ranges; partial expanded-character matches or missing inverse maps cannot produce conversion actions.

Group multiple targets at the same range; retain distinct nested or crossing ranges. In “白云山”, `[0,3)` may offer the notes “白云山” and “白云山2026” whose alias matches, while `山` at `[2,3)` remains separate. In “白云山南路”, `[0,3)` and `[2,5)` both remain. Hover/keyboard focus exposes matched text, full candidate titles, authorized location/context and match reason. Multiple targets require a choice, never first-wins.

Initial visuals use at most two separate underline lanes; existing real links keep their styling. More overlaps use a density indicator and more-matches entry rather than unlimited lanes or color alone. Allocate lanes by ascending start, descending end for equal starts, choosing the lowest free lane. Groups without a lane remain accessible in the popup. Wrap lines by actual rendered fragments; RTL maps logical source ranges to layout rather than sorting meaning by screen x. Touch long-press/action menus, keyboard commands and accessible lists provide equivalent access.

A normal click opens candidate selection/details; explicit convert-to-link changes Draft only after fresh target authorization/identity resolution and exact source-range/text validation. Use the existing D2 native link/xref plus identity adapter writer, preserving visible text without another wiki-link syntax. One conversion is one Undo unit; saving uses original D8/D6. ABA, input changes, target deletion/rename, revocation or schema changes invalidate suggestions rather than replacing words automatically or reviving old evidence.

Scanning prioritizes the viewport, is cancellable and respects original document/work budgets without blocking typing. Popups first show at most 20 targets and page by stable NodeRef order. Dense documents may stop decoration with an explicit incomplete-scan state. Truncation/budget exhaustion stays visible, never representing shown matches as the complete set. An incomplete scan may still explicitly convert a concrete fully authorized/validated candidate without global negative assertions. Rebuildable candidate indexes invalidate on authorization/name/schema changes and never own facts.

## 6. Templates, tables and output

Templates show purpose, parameters, examples and actual output preview; errors lead to original input positions and actionable next steps. Generation, validation, publication preparation, published state and cleanup warnings differ. Worker success is not an author-save receipt. Completed intranet Server exports expose an authorized download returning original staged bytes under the original receipt's controlled name, without requiring direct server-disk access. Expiry, revocation, interrupted download and unknown publication retain separate handling; never silently rerun publication or generate a different file.

Native tables expose existing row commands, exact Source editing, unsaved-draft recovery, copy-out and explicit discard. Missing controls for a legal complex table retain source read/write without flattening to a subset. Range paste, column reorder/generation and rich HTML paste need new conversion/loss rules and remain separate increments rather than claimed existing controls.

## 7. Research dispositions

| Candidate | Disposition | Model and boundary |
| --- | --- | --- |
| Split writing/research, outline, focus | Included §§1–2 | One Source/Draft, no new block database. |
| Property layout, capture, template forms | Included §3 | Same Field/schema/Template/Core, no arbitrary macros or anonymous forms. |
| Database-like views, smart lists, task details | Included §4 | Same Query/View/Action; original search P1 remains open. |
| Period navigation and rule previews | Included §4 | Frozen calendars; delivery/snooze/completion recurrence separate. |
| Reading annotations, PDF citation, links/health | Included §5 | JSON Annotation/Region, full authorization/completeness boundaries. |
| Optional potential-link overlap UI | Included §5.1 | Cancellable suggestions and explicit real-link conversion, no fake edges. |
| Local named layouts | Included §2 | No synced Draft/results/permissions; shared layouts separate. |
| Export previews, diagnostics, intranet downloads | Included §6 | Original D9 publication/receipt, no extra conversion-host dependency. |
| Basic table operations/recovery | Included §6 | Advanced range/column paste requires its own conversion/loss contract. |
| Persistent multi-Query dashboards and complex forms | Later topic | Needs layout author source, cross-query currentness and update boundaries. |
| Persistent whiteboards, flashcards, reading/audio progress | Later topic | Needs authored geometry, learning events and media/personal sync models respectively. |
| Arbitrary scripts, whole-library rollback, live cursors | Excluded from this increment | Cannot bypass Core or replace execution/recovery/collaboration design with UI. |

The seven personal-plugin design inputs map to content/structure boundaries, common fields and post-commit preferences, period navigation, link diagnostics/suggestions, native numbering/headings, table operations/recovery, and conversion task-state/output experience. Reuse design, not host-private storage or runtimes. Separate research records retain source facts and selection rationale, not claims of existing product support.


## 8. Design acceptance

All UX cases are design requirements, product UNRUN: UX-01 side-open/return retains Draft and never returns revoked content; UX-02 IME/RTL/assistive technology coexist with focus/typewriter; UX-03 unknown save followed by capture does not duplicate creation; UX-04 property conversion preserves data/unrelated source; UX-05 table/board/calendar edit the same task without writing aggregate/read-only rows; UX-06 partial Query never implies full membership; UX-07 calendar selection does not create; UX-08 stale annotation targets do not erase discussion; UX-09 hidden backlinks do not leak; UX-10 same-range multi-target, nested and crossing matches remain selectable; UX-11 more than two lanes or 20 targets remain accessible via keyboard/touch/RTL; UX-12 stale/revoked/existing-link/generated text is not misconverted; UX-13 explicit conversion/Undo/save retains one original operation; UX-14 failed/unknown export and expired downloads do not claim success; UX-15 legal complex table source survives; UX-16 local layouts neither overwrite Draft/shared author order nor restore old permissions.
