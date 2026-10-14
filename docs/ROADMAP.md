---
source_language: zh-CN
translation_of: ROADMAP.zh-CN.md
translation_status: synced
---

[简体中文](ROADMAP.zh-CN.md)

# Implementation roadmap

2026-10-13: plan the current scope as **one initial MVP plus seven further milestones**. Version numbers divide the work and may become multiple smaller releases after testing. They are neither delivery dates nor claims of implementation. [REQUIREMENTS](REQUIREMENTS.md) remains the scope authority; research candidates are not automatically included.

The **0.6.0-alpha.1** candidate retains bounded Windows local features from v0.2–v0.5 below and adds the [first bounded Server / WebUI slice](SERVER.md). v0.6 remains incomplete: same-owner/partial synchronization and multiuser authorization for all consumers are still being implemented. See the [preview guide](MVP.md) for actual availability, checks and outstanding acceptance. The version number does not establish complete IME, rich-editing, arbitrary Office conversion or cross-platform acceptance.

| Milestone | What users can accomplish | Main requirements and completion conditions |
| --- | --- | --- |
| v0.1 Local writing MVP | Create/open a Windows local library and `A/A.typ` notes; browse unmanaged entries; open ordinary `.typ` files outside libraries; source editing, native preview, basic five views, search, basic links, save and recovery. | Bounded slices of RQ01–03,12,22,24–26. Deliver a runnable artifact and test draft retention, refusal to overwrite external changes, Save As, empty documents and safe text editing. Full rich editing, sync and Office are outside this version. |
| v0.2 Complete writing experience | Continuous editing across five views; common list/table/math controls; titles/subtitles, deep and run-in headings; library styles with native overrides, layouts and Chinese support. | RQ01–06,25–26; §§5.3–5.5. Preserve a single draft, selection and Undo history; retain native constructs that cannot be safely rewritten. Test IME, keyboard, accessibility and complex source rather than counting source fallback as complete rich editing. |
| v0.3 Structured information and Query | Rich people, organization, paper and task types; properties and quick capture; search/filter/Query; lists, tables, boards, calendars, aggregation and charts. | RQ08–11,15–17,NAV01–05. Consistent field semantics across editing, querying and views; distinguish multivalues, exact numbers, missing values, errors and incomplete results. Task details remain document content. |
| v0.4 Links and knowledge organization | Link-form conversion, outgoing/incoming/backlinks and overlapping potential links; document/media annotations; templates, complete copying, moving, trash/restore and backup. | RQ12–14,18–19,22. Correct identities/references, preserve discussions when anchors fail, no guessed reanchoring; actual recovery evidence for multi-file operations and unknown outcomes. |
| v0.5 Office output and interchange | Editable Word documents, real workbooks, bibliography integration, dependency-aware import/export, explicit loss reports and local downloads. | RQ07,10,19–20. Validate in real Word/spreadsheet applications, including run-in numbering and body character styles; preserve originals. Server download revocation is verified in v0.6. |
| v0.6 Multiple users and synchronization | Server/WebUI, intranet operation, multiple users without real-time editing, offline replicas and partial sync; authorization and download delivery. | RQ21–24 and all consumers. Conflicts preserve both drafts without blocking unrelated notes; revocation protects search, previews, annotations, caches and downloads. Real-time collaboration stays deferred. |
| v0.7 Multiple clients and overall acceptance | Complete the promised desktop environments, Mobile and CLI; file access, touch, RTL, performance, installation/update and offline fonts across clients. | RQ03,21,25–26 and platform conditions. Use the [WORKFLOW](WORKFLOW.md) impact matrix to select actual target environments and complete platform product cases and outstanding checks; Windows, browser viewports or execution-environment qualification do not establish other platforms' acceptance. |
| v0.8 Agents and automation | Authorized reading, proposed changes and human application, followed by scoped rules and specific connectors. | RQ27. Separate authorization for writing, egress, credentials, cost and external effects; visible stopping/failure/unknown outcomes, no blind retry of unknown external effects. First-generation external effects require per-request confirmation. |

Saving reliability, authorization boundaries, accessibility and performance receive continuous checks from the first version. Reuse existing evidence, checking actual changes and affected consumers rather than restarting a global design review every version. Dependencies can change implementation order. Complete acceptance must cover all 27 requirement groups, navigation additions, styles, five views and Chinese support, not merely a version count.

## Local v0.5 completion: timeline and embedded Query

Connected in local candidate `0.5.0-alpha.2` on 2026-10-14: five-view embedded Query completes **v0.2/v0.3**; the note timeline belongs to **v0.4/Core**. Both reuse original version storage, recovery and Query execution. Previous `0.5.0-alpha.1` lacks these complete interactions. Versions and pre-existing storage do not replace acceptance.

Requirements are in [REQUIREMENTS](REQUIREMENTS.md) §§5.4.1,5.6; behavior is owned by [CORE](design/CORE.md) §3.1, [EDITOR](design/EDITOR.md) and [FEATURES](design/FEATURES.md). Implement save timestamps in existing commit records and per-note history enumeration, then read-only viewing/comparison and Draft-protected restoration. Next connect embedded Query's five views, builder and result states to the same source. Validate affected Core, editor and Query paths before packaging the next candidate; reuse unchanged Office evidence.

The following conditions govern this implementation and acceptance. Per-case outcomes use the existing [WORKFLOW](WORKFLOW.md) candidate record. Core/HTTP and isolated headless-browser checks separately describe their scope; simulated keyboard/touch events do not replace real OS IME, physical touch or complete assistive-technology acceptance.

| Case | Required observable result |
| --- | --- |
| LHT01 Saved versions and time | Browse three distinct saves by actual save time. Older records without timestamps show unknown; ties/clock rollback do not invent commit order. Drafts and uncommitted/unresolved operations do not impersonate saved revisions. |
| LHT02 Read-only viewing/comparison | Open the exact older revision and compare it with the current saved revision; with a Draft, explicitly select saved content or a Draft snapshot. Browsing, comparing and closing the view do not edit source or lose Draft/Undo. |
| LHT03 Protected restoration | After saving v1, v2 and v3, restore the different content of v1 while a Draft exists: first explicitly protect the Draft, then create a new save v4. v2, v3 and the protected Draft remain recoverable; cancellation leaves the current Draft unchanged. |
| LHT04 Identity continuity | Application rename/move and restart preserve the original note's history. Newly copied notes and namesakes stay separate. Unproved identity after an external rename/replacement is reported for resolution rather than guessed. |
| LHT05 Retention and absence | Disclose actual retained coverage; unretained/missing/unreadable revisions are explicitly unavailable, never replaced with latest. Missing original configuration/dependencies does not block still-readable exact source; no promise of every version or keystroke forever. |
| LHT06 Conflict and retry | External changes after restore preparation cause refusal to overwrite, preserving both drafts. A lost response queries the same original operation without another save. Unresolved recovery or failed Draft persistence cannot be silently skipped. |
| IQV01 First and second views | For the same embedded query, V01 edits source directly with a separate result preview; V02 always shows full query source alongside results, which never obscure or replace it. |
| IQV02 Third view | Results are the default. Keyboard/touch can explicitly expand the corresponding source; collapsing after editing retains input. Ordinary result focus does not edit source. |
| IQV03 Fourth view | Results are the default, with a builder for edits. Focusing results/controls never reveals syntax. Only explicit Edit source enters a separate editor or source-visible view. Cancel leaves Draft unchanged; confirmation is an undoable edit to the same Draft. |
| IQV04 Fifth view | Show results and states without definition-edit buttons, menus or shortcuts. Result interaction cannot directly write body content; authorized navigation, selection and copying remain available. |
| IQV05 View/Draft continuity | Switch an unsaved query through all five views while retaining exact source, Draft and Undo. Switching adds no content-Undo item and does not autosave. Builder/source changes share the same Undo/Redo history. |
| IQV06 Shared execution and states | Every view uses the same definition/Draft basis and Query mechanism, distinguishing authorization, errors, partial, stale and complete-empty results. Late old results after edits cannot impersonate new results or confer write eligibility. View changes create no second executor. |
| IQV07 Multiple queries and complex source | In a note with multiple queries, edit only the selected definition. Preserve original source and explain limits when the builder cannot express it, its source is ambiguous or syntax is temporarily invalid; never simplify, guess a source or overwrite another query. |

This round covers local notes and existing local authorization boundaries. **v0.6** verifies separate multiuser authorization for history listing/reading/comparing/restoring, cached/displayed content after revocation, and received/missing/concurrent versions after synchronization. Timestamps alone cannot turn synchronized history into a false single causal chain. File-version timelines always remain distinct from Query business timelines for tasks/events.

## v0.5 addition: proofreading, annotations and Word review interchange

Implementation of [REQUIREMENTS](REQUIREMENTS.md) §5.7 was confirmed on 2026-10-14. This belongs to stage 4 cross-object capabilities, extends v0.4 annotation foundations and completes the local path in v0.5 before v0.6. The earlier bounded alpha.3 closeout does not establish these additions or rewrite previous acceptance results.

| Order | v0.5 implementation slice | Completion gate |
| --- | --- | --- |
| 1 | Annotation provenance, proofreading rules/states and exact ranges/insertion positions for basic revisions | Reuse annotation JSON and Core transactions; retain old annotations, replies and unknown data; never guess stale positions. |
| 2 | Four local checks, results panel and batch annotation creation | Distinguish valid/suspect/excluded ranges; locate text across formatting; repeated runs do not duplicate annotations and late results cannot appear current. |
| 3 | Body-only and annotated Word output | Map comment text, ranges, attribution and replies according to actual support; report unlocated/unsupported items; do not rerun proofreading during export. |
| 4 | Word body, resources, comments and replies import | Retain exact originals/losses; separate new object identity from Word IDs; never overwrite existing notes. |
| 5 | Basic text insertion/deletion/replacement import, handling and export | Pending remains pending; accepting/rejecting has correct effects; overlapping, structural and formatting revisions are never silently flattened. |
| 6 | Complete round-trip acceptance on one candidate | Annotated/tracked DOCX → Weftext proofreading/replies/suggestion handling → DOCX → real Word inspection; list WPS/LibreOffice results independently. |

Required counterexamples include a word split across bold/italic runs, Chinese/non-BMP characters, repeated identical text, paragraph/table boundaries, crossed symbols, empty dictionaries, overlapping matches, ignored items, edits after checking, edits around insertion points, comments on deleted text, duplicate clicks/lost responses, source/annotation commit conflicts, unknown DOCX parts and complex revisions. Preview, confirmation and download refer to identical content; XML checks are not Office interaction acceptance.

v0.6 validates separate permissions, batch conflicts, offline sync, revoked results/history/artifact caches and downloads across this entire path. v0.7 completes other promised platforms; local rules do not wait for v0.8 Agents. Full complex Word revision round-tripping and AI semantic proofreading are not automatically included. Use existing candidate records for actual progress/results rather than creating another acceptance authority.

## v0.6 addition: stable-link fragment correctness

CR14 is connected in the current v0.6 candidate: hosted navigation parses stable identity and a strictly decoded fragment separately, then authorizes and locates the unique natively compiled label at its exact main-source range. Actual personal Draft and saved content are checked separately. Same-note navigation retains the session and Undo; cross-note navigation protects the original Draft first. Failed location retains the original link and editor; opening only the target requires explicit choice. Generated/foreign labels without an exact main-source range are explicitly unavailable. Engine, real HTTP and background-browser checks cover Chinese/encoded labels, missing/duplicate/unexecuted labels, Draft changes, moves/restarts, read-only access and revocation; evidence records the actual separate runs. This pass completes neither all of v0.6 nor later citation interactions; see [SERVER](SERVER.md) for the interface.

## Later validation and scope

v0.7 engineering validation reuses product entry points: build each target platform's candidate once per attempt. Engine/HTTP/background-browser checks cover what they can prove first; installation, system file integration, real IME and AT use isolated system environments with verified capabilities. Record Windows Sandbox connected-viewer limits separately from a VM's background capability. Linux/macOS desktop cases use the corresponding real OS; Android/iOS use their respective simulated environments and necessary physical devices. Separate declared, available and observed environment states. Startup failure never falls back to the host foreground; give advance notice and schedule any necessary short foreground window. Report minimal results separately for issue reproduction, fix regression and release acceptance, reusing valid evidence for unchanged scope. This is future validation work; current 0.5.0-alpha.2 gains no multiple-client or isolated-execution pass from this plan.

Whiteboards, learning systems, notification delivery, real-time collaboration and shared layouts remain candidates. Adding them requires an explicit scope decision so “all requirements” does not grow indefinitely. Formal releases follow [WORKFLOW](WORKFLOW.md); this roadmap does not authorize publication.
