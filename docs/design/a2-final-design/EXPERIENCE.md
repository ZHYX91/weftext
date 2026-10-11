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

One capture panel provides parent, type/template, optional title, business properties and body, showing real destination and required conflicts/missing values before creation. Selecting a template only prepares D9 results; confirmation uses original creation/installation. Save-and-add-another resets only after the original save result. Network unknown preserves the original request and Draft rather than creating a duplicate. Shortcuts do not bypass authorization, preview or confirmation. On success, original resultAllocations identify the actual new Node for a freshly authorized open and destination display; title/filename never guesses the result. Capture within a Query does not guarantee membership: requireMembership follows original D5 §19.3 validation before commit. Without that requirement, creation may succeed outside the filter; show that state and an open action without changing Query or creating another note.

Property panels group/search/pin common fields and distinguish authored, derived read-only and unavailable values. Simple fields use native [authoring](AUTHORING.zh-CN.md); complex values expose individual occurrence notes/provenance. Exact source changes are inspectable. Labels and frequency suggestions are not FieldIds; device recents update only after successful commit, not cancellation/hover. Identity is read-only; type edits follow full constraints.

Relation pickers expose authorized candidates and direction; selection previews actual Field/inverse changes rather than drawing client-only edges. Rollup wizards construct existing read-only Query projections. Intranet forms based on existing Facets/templates reuse this panel/Core under the current submitting user. Anonymous public forms, submitting as others and cross-object multi-step macros are outside this candidate.

## 4. Query, tasks and multidimensional views

List search, visual filters and optional textual shortcuts produce the same Query. Query editing separates what data from View presentation, exposing definition, parameters, scope and completeness. Saved views remain existing Document occurrences rather than persisted copies of results. Saved-search and Trash Query global P1 findings remain unresolved; new UI does not close them.

A task is an ordinary Node with task classification. Details are its body and status/due are original Fields. Task tables, smart lists, boards and calendars consume the same Query/Fields. Completion, card dragging and due edits show precise field changes through original Actions. Without a uniquely proven field mapping, drag submission is unavailable with explicit editing offered. Aggregate/read-only/hidden/stale rows are not writable objects.

Query examples cover incomplete tasks, due ordering, related projects, read-only summaries and period ranges with explicit empty, missing, error, partial-index and complete states. Displaying 200 rows is not complete membership; bulk actions retain original full-target/budget rules without silent splitting. No arbitrary JavaScript/SQL/Datalog entry point is added.

Period navigation offers previous/next/parent; calendar clicks select existing objects first and only an explicit create action makes missing notes. Existing calendar/period/series-scope semantics are reused without inferring new facts from today's date or host time zone. Rule previews expose calendar, time zone, range and inability reasons. Delivered reminders, snooze, completion-relative recurrence, a personal My Day set and natural-language dates remain separate topics, not assumed from Calendar/Automation's existence.

## 5. Annotations, citations and links

Audio/video uses D9-SCHEMAS §16 exact rational timebase/tick half-open interval issuance before original JSON Annotation save/navigation, never whole-media substitution. Annotation reads include stored summaries without granting hidden media/prose reads; chapter/Review Bundle context remains independently authorized.

Reading connects original text, personal annotations and citation sources. Annotation content stays in independent `weftext.annotations.json`; discussion state and target resolvability display separately. PDF/image region selection→annotation→explicit citation insertion→return to original region uses existing D3/D8/D9 coordinates/source/permissions. Missing providers or incompatible coordinates retain the annotation/status rather than screenshot-based reanchoring. Ordinary prose equally supports exact document_range selection→thought/mark→original JSON Annotation save→return to source. Chapter/source summaries are derived groups of currently authorized Annotations, not copied bodies. Stale, unresolved or unreadable targets remain explicitly ungrouped while separately authorized thoughts survive without exposing hidden source. Explicit citation insertion uses the full Source Draft structural-edit qualification in §5.1, not a read adapter disguised as a writer. Unsaved annotation drafts retain their existing lifecycle.

Outgoing links, incoming links and potential mentions are separate sets. Real links come from parsed source occurrences; suggestions do not count as graph edges, actual links or reference completeness. Link health diagnoses missing targets, stale ranges, no inbound links and isolation. A Workspace-wide no-inbound conclusion requires authorized complete relevant scope; otherwise report none found in the visible scope, without exposing hidden-node counts or automatically removing apparently isolated objects.

### 5.1 Potential links in body text

A device-level show-potential-links setting defaults off and never edits source when toggled. First-version terms come from currently authorized live Nodes' authored titles and explicit schema-recognized aliases, not arbitrary similarly named attributes, history/Trash or hidden nodes. Unavailable alias sources explicitly narrow the scan rather than claiming completeness. Deduplicate candidates by full NodeRef, not title; exclude the current Node by default.

Match ordinary visible text fragments with precise root-authored writable D2 origins using exact Unicode-scalar literals, without fuzzy matching, case folding or normalization. Do not decorate code, math, URLs, existing links, foreign front matter/attribute values, annotations, generated/include/multi-origin/passthrough content or unproven writable ranges with conversion suggestions. Do not join words across fragments. Matches carry complete `[start,end)` Unicode-scalar intervals tied to the same Draft serial/flow; example numbers are not UTF-8 byte offsets. Conversion uses a fresh Draft map to exact D2 full-file byte/EOL ranges with identical owner, Base, draftSerial and source. Both endpoints must be original D8 Unicode18 grapheme boundaries without splitting CRLF. Partial expanded-character/grapheme matches or missing inverse maps retain discovery and an unavailable-conversion reason, never an expanded selection. Soft wraps can decorate one logical fragment; source joins/EOL or separate fragments never form a word. This does not constrain schema-codec legal Unicode scalar values.

Group multiple targets at the same range; retain distinct nested or crossing ranges. In “白云山”, `[0,3)` may offer the notes “白云山” and “白云山2026” whose alias matches, while `山` at `[2,3)` remains separate. In “白云山南路”, `[0,3)` and `[2,5)` both remain. Hover/keyboard focus exposes matched text, full candidate titles, authorized location/context and match reason. Multiple targets require a choice, never first-wins.

Initial visuals use at most two separate underline lanes; existing real links keep their styling. More overlaps use a density indicator and more-matches entry rather than unlimited lanes or color alone. Allocate lanes by ascending start, descending end for equal starts, choosing the lowest free lane. Groups without a lane remain accessible in the popup. Wrap lines by actual rendered fragments; RTL maps logical source ranges to layout rather than sorting meaning by screen x. Touch long-press/action menus, keyboard commands and accessible lists provide equivalent access.

A normal click opens candidate selection/details; explicit convert-to-link changes Draft only after fresh target authorization/identity resolution and exact source-range/text validation. Conversion requires full Source read/write qualification: readable target does not imply writable source, and missing qualification makes conversion explicitly unavailable. UI automatically constructs exact complete proposed Source in the same Draft using D2 native link/xref and original NodeRef encoding. The same parser fully reparses it to prove the selected full NodeRef, unchanged visible text and untouched bytes before original Source-edit prepare/save. The D2 identity adapter validates projections; it is not a writer. Do not use the successful d8_draft_text_replace/2 or splice_plain branches that forbid new macros. Labels with brackets, commas, quotes or role-like text need native encoding and reparse equivalence; failure retains Draft and Source preview/repair without claiming conversion. Users need not handwrite syntax and no second wiki-link syntax is introduced. The popup has an explicit suggestion accessible name; Escape restores selection without stealing typing focus. One conversion is one Undo unit; saving uses original D8/D6. ABA, input changes, target deletion/rename, revocation or schema changes invalidate suggestions rather than replacing words automatically or reviving old evidence.

Scanning prioritizes the viewport, is cancellable and respects original document/work budgets without blocking typing. Popups first show at most 20 targets and page by stable NodeRef order. Dense documents may stop decoration with an explicit incomplete-scan state. Truncation/budget exhaustion stays visible, never representing shown matches as the complete set. An incomplete scan may still explicitly convert a concrete fully authorized/validated candidate without global negative assertions. Rebuildable candidate indexes invalidate on authorization/name/schema changes and never own facts.

### 5.2 Link forms and conversion

[LINKS](LINKS.zh-CN.md) solely owns the full behavior. Context menus, touch and keyboard provide stable-ID, source-relative, Workspace-root and local-absolute conversion, address/complete-link copy and shortest valid relative paths. Device default ID never rewrites existing source; Workspace-root addresses explicitly name the Workspace, distinct from local disk roots and ordinary relative paths. Conversion is one Undo in the same full Source Draft; copying leaves Draft unchanged. Target/fragment/native attributes must be provably preserved, with path disclosure, future move behavior and stable-edge changes visible. Forms unable to preserve fragments are disabled without deleting anchors; general offline routing still needs a named D9 follow-up.

## 6. Templates, tables and output

Templates show purpose, parameters, examples and actual output preview; errors lead to original input positions and actionable next steps. Generation, validation, publication preparation, published state and cleanup warnings differ. Worker success is not an author-save receipt. For intranet server_download, authorization and confirmation bind the same ExportPlan/4 inputs, loss choices and staged bytes; deliver the exact dataFile bytes under its controlled name. The result page exposes Server delivery state and download, requiring neither an external destination nor PublicationReceipt/4 or direct disk access. Download tokens bind the original Plan/confirmation/dataFile and current audience; each request and chunk rechecks authorization and original retention expiry. Retries deliver only the same retained output; expiry is explicit and regeneration needs a new confirmation. External bundle publication consumes original PublicationReceipt/4; pure download never claims atomic file publication. Expiry, revocation, interrupted download and unknown publication retain separate handling; never silently rerun publication or generate a different file.

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
| Export previews, diagnostics, intranet downloads | Included §6 | Original D9 Plan/delivery distinct from external publication/receipt; no extra conversion-host dependency. |
| Basic table operations/recovery | Included §6 | Advanced range/column paste requires its own conversion/loss contract. |
| Persistent multi-Query dashboards and complex forms | Later topic | Needs layout author source, cross-query currentness and update boundaries. |
| Persistent whiteboards, flashcards, reading/audio progress | Later topic | Needs authored geometry, learning events and media/personal sync models respectively. |
| Arbitrary scripts, whole-library rollback, live cursors | Excluded from this increment | Cannot bypass Core or replace execution/recovery/collaboration design with UI. |
| Four link forms, conversion and copy | Included §5.2 / LINKS | Native syntax and same-target proof; new ID-plus-anchor and general offline routing explicitly deferred. |

The seven personal-plugin design inputs map to content/structure boundaries, common fields and post-commit preferences, period navigation, link diagnostics/suggestions, native numbering/headings, table operations/recovery, and conversion task-state/output experience. Reuse design, not host-private storage or runtimes. Separate research records retain source facts and selection rationale, not claims of existing product support.


## 8. Design acceptance

All UX cases are design requirements, product UNRUN: UX-01 side-open/return retains Draft and never returns revoked content; UX-02 IME/RTL/assistive technology coexist with focus/typewriter; UX-03 unknown save followed by capture does not duplicate creation; UX-04 property conversion preserves data/unrelated source; UX-05 table/board/calendar edit the same task without writing aggregate/read-only rows; UX-06 partial Query never implies full membership; UX-07 calendar selection does not create; UX-08 stale annotation targets do not erase discussion; UX-09 hidden backlinks do not leak; UX-10 same-range multi-target, nested and crossing matches remain selectable; UX-11 more than two lanes or 20 targets remain accessible via keyboard/touch/RTL; UX-12 stale/revoked/existing-link/generated text is not misconverted; UX-13 explicit conversion/Undo/save retains one original operation; UX-14 failed/unknown export and expired downloads do not claim success; UX-15 legal complex table source survives; UX-16 local layouts neither overwrite Draft/shared author order nor restore old permissions. UX-03 also covers exact post-create location outside the current Query; UX-08 covers prose annotations and failed chapter grouping without losing thoughts; UX-12 covers e plus a combining mark, emoji ZWJ and Chinese scalar versus UTF-8/UTF-16 coordinates; UX-13 covers full-source qualification, native-label encoding and complete reparse without macro injection through narrow text APIs; UX-14 covers pure download without an external destination/receipt and exact-dataFile retry.

UX-17 covers four link forms, whole-Workspace relocation, same-name/cross-library/local-versus-Server boundaries, conversion/copy/Undo and exact fragments through LINK-01–11. UX-18 covers task groups derived from FacetSchema rather than source nesting, no duplicated shared Fields and preferences that never rewrite existing source.
