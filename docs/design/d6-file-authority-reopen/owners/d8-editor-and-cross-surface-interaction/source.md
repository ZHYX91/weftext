---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 3c031fa6-0f70-43ee-9388-cc007d4d2a98.

# D8 Editor and Cross-Surface Interaction Contract

Candidate status: D8-FA-r01; complete owner afterimage, not independently accepted, activated, or implemented. Fixed source S is 7e18168dad3e6d120fce0dd607dc10fa7894e252. Original acceptance, model, and platform evidence retains only its original scope; this candidate is not implementation evidence. The D3 conflict adapter, current D7 effects producers, and current qualification of portable Locators across replicas still require their respective coordination and independent acceptance. These integration gates do not retroactively cancel recovery of real historical decisions.

Revision: D8-FA-r01; candidate, not independently accepted or activated. This candidate comprises this main contract, Editor Interfaces, Direction and Accessibility, Acceptance Matrix, Terminology, Implementation Impact, and the coordinated D7 effects-producer changes. Architecture acceptance does not establish implementation, platform support, or measured performance.

## 1. Choice, problem, and non-goals

One shared editing controller manages Drafts, input transactions, selection, and request state for the same author target. Every UI produces the same Core intent through it. Source edits exact source, Write edits content through Core-proved mappings, and Read is a noneditable projection; these are not three independently saved contents. Properties, native tables, and Node collections retain their own domain targets regardless of tabular presentation.

Inputs are D1's five surfaces/capability boundaries, D2 Profile2/wire2, D3 wire12/Result9, current D4 types/Entry/Registry and D6/D7 consumers, D5 without durable Records, current D6 Policy/3 and durable submission, and D7 typed DAG/CEL1/pure View/explicit Action. D3 remains the sole identity owner, D6 the author-commit owner, and D7 the complete effects owner. D8 adds editing adapters and consumer state, not document/field/row identities, a collaboration database, or another parser.

This generation does not deliver realtime coediting/CRDT, arbitrary rich-text/HTML editing, arbitrary formula execution, Mobile conversion/Agent, translated Arabic UI, automatically synchronized cross-device Drafts, or portable block-direction syntax. D9 template/conversion entries remain explicitly unavailable until their actual adapters are coordinated and independently accepted; the UI cannot guess template construction. Arabic/Hebrew content, bidi presentation, and RTL shell remain separate mandatory D8 capabilities.

## 2. State ownership and shared sessions

| State | Owner, binding, and lifetime |
|---|---|
| Committed author source/Ref/version | Core authority under D2–D6; UI writes neither files nor databases |
| EditSession | Current client, principal/session, authority connection, WorkspaceRef, complete target Ref, and read scope; no content identity or reuse across logins |
| Base | Core-authorized exact source/complete Field projection/Annotation value and complete SourceObservation/1, immutable within this session |
| Draft | User proposal: exact text or typed input, monotonic draftSerial, Base binding; no author revision, D3 Locator, or Query-result authority |
| DraftProjection | Discardable mapping produced by the same Core for the exact Draft and serial; late responses cannot overwrite newer Drafts |
| Selection/viewport | Discardable current-session state, including direction/focus and complete Base observation plus draftSerial or committed state |
| Prepared/Submitted | Core-retained immutable plan/preview and complete original submit request; UI retains that request for recovery without changing tokens/effects |

Panes for the same principal, authority, and owner in one client share one writer controller. Read-only panes may bind the same Draft or an explicit committed Base. Property/table and Source/Write editors cannot maintain mutually unaware submittable branches: when a Draft is dirty, another same-owner entry focuses the existing editor, explicitly discards, or retains a separate uncommittable merge proposal; it never silently overwrites the first. No implicit cross-process/device session merge exists; Core versions/CAS decide.

Read scope never expands. A phone-only session cannot open full Source/Write or borrow another principal's cache through another pane. A full-source reader can switch same-owner surfaces; others use complete D7 Field selection and narrow qualification. Partial property projections label unauthorized/unavailable fields, not empty values.

Draft serial is a local nonnegative counter incremented once per completed input transaction. Exhaustion closes the session while retaining the proposal for reopening; it never wraps. It is not a D6 revision Counter. A session has one active input transaction. Dialogs retain fixed target/Base despite background focus, sort, or selection changes.

## 3. Shared editor state machine

Four orthogonal axes distinguish access, disconnection, and dirty content:

- buffer=`clean|dirty`: clean exactly when Draft equals Base byte-for-byte, not when rendering looks equal.
- input=`idle|composing`: composition is one unfinished local input transaction.
- submission=`none|preparing|preview|submitted_unknown|planned|committed|rejected|terminal_failed`.
- access=`current|offline_remote|needs_revalidation|denied|authority_unavailable`: a local observation; only Core grants final authority or determines a decision.

| Event/condition | Required transition |
|---|---|
| Text/value/structure input | When idle, edit the current Draft outside the already-submitted branch; new serial, invalidated old projection/preview, local proposal only |
| Composition start/update | Freeze original draftSerial and replacement range; preserve precomposition buffer/selection; update preedit only, without prepare, commit, or shortcuts |
| Composition completion | One complete final replacement, at most one Draft transaction, zero if text unchanged; confirm final native state without applying compositionend and later input twice |
| Composition cancellation | Restore precomposition buffer/selection, zero author changes; focus loss cannot be guessed to mean cancel or confirm |
| Request preview | Require idle/current, complete target binding, and no unresolved submission; freeze serial/intent and Core-prepare; further input is allowed, but late prepared results remain historical and cannot confirm a newer Draft |
| Prepare succeeds | Enter preview only while serial/target/access match; obtain complete manifest and display actual domain, scope, losses, and unavailability |
| Confirm submit | Separate explicit action; verify current serial/preview, complete effects availability, and no composition; retain original request before sending; immediately submitted_unknown, not Saved |
| Receipt | Match original request/OperationId/protocolOwner and display actual committed outcome. After current authority permits establishing the original actual after-image and updating Base under successor-Draft rules, derive buffer solely from exact equality of current Draft and new Base. Serial/request/generation constrain receipt ownership and updates, not additional cleanliness conditions. Never overwrite current Draft/selection or roll back serial. A new complete Base observation invalidates old maps |
| Planned/disconnection/timeout/indeterminate response | Retain original request/proposal and recover/replay under the original protocol; never repeat an unknown submission under a new OperationId |
| Prepare/submit rejected | Retain Draft and show the actual authorized error; reprepare only after proving the original decision cannot commit |
| Cancel prepare/close preview | May stop UI waiting/discard an unsubmitted preview; cannot cancel planned or roll back committed |
| Permission/authority/Registry/revision changes | Invalidate confirmation qualification and affected derived results; retain input as a local proposal and obtain a new preview after Core revalidation; a submitted branch first checks its original decision |
| Mode/direction/zoom/viewport changes | No source mutation, automatic composition completion, or submit; defer necessary layout updates until composition ends |

Input may continue in a labelled successor Draft while submitted_unknown/planned, but it cannot be submitted until the predecessor has a proved outcome. The successor is based on submitted source plus the local input log. If the predecessor committed with exactly the expected actual after bytes, that becomes the new Base. Rejection or a different actual after requires a three-way proposal, never silent rebasing. D3 symbolic creations reopen from real receipts/after-images, not guessed Refs.

Revocation cannot recall bytes already delivered. Stop further disclosure, clear removable undelivered results/previews, lock the session, and require reauthentication; cached bytes do not authorize requests or prove permission. User-authored proposals follow device Draft policy and grant no original-source/history access. Local cache clearing is not promised as reliable secure erasure.

## 4. Source, Write, Read, and Draft parsing

Source may retain temporarily invalid text. Preserve exact UTF-8 scalars, BOM, mixed CRLF/CR/LF, trivia, ordering, and untouched fragments. Only explicitly replaced bytes change; mode/save/IME/copy/direction changes do not normalize NFC, EOLs, digits, quotes, or whitespace. Malformed UTF-8 belongs to D6's physical-repair envelope with no D2 payload; silently decoding replacement characters and saving them is forbidden.

Write renders only a complete successful current-Draft D2 projection and map from Core. On invalid input the whole Draft projection is unavailable: expose Source repair and the real diagnostic, never old body with new header. An explicitly revision-labelled last committed view may be shown separately, not as the current Draft. D4 typed unavailability is distinct from D2 invalidity. Unknown namespaces whose raw bytes are preserved cannot disappear merely because body text is edited.

D2 Inline has only text/link/resource_occurrence/node_link/citation, not arbitrary bold/italic/math formatting. Write text/link/heading/list/native-table commands must produce legal D2 source; unfrozen format buttons cannot pretend availability. Formula/code is inert source/literal or explicitly displayed D7 CEL; opening/hovering never executes it. Protected payloads may be edited as whole source, without rich-editor reinterpretation of their contents.

Core supplies Write maps; rendered nodes/DOM offsets are not author positions. Non-bijective mappings—multiline paragraph joins, escaped cells, citation labels, resource placeholders—are atomic/read-only occurrences with explicit Source navigation. Never save rendered innerText as source. Interfaces freezes the Draft Edit Map, separate navigation origins, inert replacement, and ordinary-region commands. Core plainRegions include every original space/tab/EOL; first typing, blanks, paragraph breaks/joins, emptying, and repeated deletion all produce exact proposals. Ordinary Write hosts display source EOLs and interparagraph blank lines; Read joins a single EOL as D2 space. Make the distinction visible rather than hiding EOLs and guessing source, or treating metadata/structure syntax as ordinary text. Atomic/escape/join restrictions apply only where editing cannot be proved, not to all Write as a mere proofreader. Further structural commands reuse existing closed D7 Actions; unclosed buttons remain unavailable.

Source→Write first finishes/cancels composition by user/native confirmation, requests the current serial's projection, then switches on success. Write→Source preserves Draft and maps to logical source. Read may show a labelled Draft preview or committed revision; default opening is committed. Read does not directly edit content; independent actions such as Annotation require explicit target selection and a Core plan.

Local Desktop/Mobile use the same bundled Core. Remote Desktop/WebUI/Mobile request only Server projections/transforms/prepare, without a second hosted-source parser. Remote offline mode permits plain Source Draft input, already-delivered read-only snapshots, and Draft Undo; current rich projection/Query/validation/commit is unavailable. Reconnect reauthorizes and binds the current version; offline rendering is not complete semantics.

## 5. Caret, selection, and command routing

D3 retains durable addresses. Editing selection is nondurable `owner + baseObservation + draftSerial-or-committed + anchor + focus + affinity`. Anchor/focus are logical source points, plainRegion scalar offsets, or exactly mapped text-run points. Preserve selection direction and take min/max only when executing a range. Affinity chooses visual caret position at bidi/wrap boundaries, not edited text.

Public source coordinates use D2 zero-based logical lines, Unicode scalar columns, and exclusive ends. Convert internal UTF-8 bytes, DOM UTF-16, and native positions through the same exact string; JS length is not a scalar count. Never split CRLF or surrogate pairs. Ordinary movement/deletion uses extended grapheme clusters. Explicit Source codepoint inspection/editing permits review of controls/combining characters without changing the default; it still cannot create unpaired surrogates or split CRLF.

Visual Left/Right within one layout epoch follows Direction. Every device exposes logical previous/next through keyboard/accessibility commands. Backspace/Delete removes the logically preceding/following grapheme, not the screen-left/right character. Existing selection deletes its logical range. Crossing an atomic occurrence requires explicitly including the complete source target, otherwise it is read-only or routes to Source. A rectangular grid selection is a target set, not a continuous body range.

Routing priority is native composition/candidate UI, then focused open dialog/menu, then local editor commands, then shell commands. Escape first reaches composition, then closes a local popup and restores focus, then offers abandonment of an unsubmitted Draft. Ctrl/Meta+Enter, Enter, Tab, slash, and shortcut Delete never steal composition. Delete outside a focused editor has no content effect. Grid Delete opens a domain-labelled action choice, not default Node Trash.

Slash palettes, command palettes, context menus, and inline toolbars use one command directory binding capability, domain, target, shortcut, accessible name, and read-only reason. Opening a menu does not prepare/execute. Confirmation retains its opening target; stale targets reject and require reselection. Pointer/keyboard invoke the same intent. Asynchronous refresh cannot move focus to a new same-named row.

## 6. IME, composition, and platform events

Host adapters normalize actual sequences to begin/update/commit/cancel; controllers do not assume one browser order. Begin retains original buffer/selection and input generation. Preedit may be transient, but cannot enter prepare, Query, author history, or automatic durable submission. A final input may follow compositionend; matching original event transaction and final native buffer yields one commit, never appending both event.data and text already in the DOM.

During composition, do not reconstruct/unmount the host, replace value, reset selection, reorder its virtual row, or change direction/font/wrapping width. Queue asynchronous author changes; afterward preserve the proposal and handle staleness. Revocation stops submit and further disclosure immediately while native staging can safely finish as a local proposal. Before background/navigation, request native finish/cancel. If the outcome is unknowable, retain precomposition Draft plus explicitly unfinished preedit for recovery, never pretend preedit is final.

One IME confirmation is one Draft Undo group. Latin dead keys, combining marks, emoji ZWJ, speech, and handwriting replacements obey original ranges/final text. Spellcheck/autocorrect is an explicit native replacement transaction and cannot bypass target/serial. During composition, Undo/Redo belongs to native input, not the global Draft stack.

## 7. Copy, cut, paste, and drag

Offer two explicit text copies: Source copies selected exact source; Read/Write “Copy text” copies logical readable text with source markup removal disclosed, never visual glyph order. Exclude presentation bidi controls, hidden ARIA labels, and inaccessible fields. Loss of formatting is an explicit display conversion, not canonical export. A separate D3-reference command copies a safe textual complete typed address; title/path is not a Ref.

Default paste reads only OS-provided text/plain. HTML/RTF/custom MIME is not automatically author syntax. Rich content uses the platform's explicit plain flavor; if absent, explain unavailability and leave the clipboard unchanged. Source paste is an explicit possibly invalid proposal. Write paste uses Core inert fragments/plainRegion splice, rejecting completely if unrepresentable and offering explicit Source/Document/Resource choices; no truncation or macro execution. Native multirow/TSV batch paste is unavailable, with no generated Source or splitting into multiple single-row commits. Users may independently open generic Source and submit an explicit source proposal, but that is not implemented table batch paste.

Cut writes the clipboard successfully before deleting the fixed selection in one Draft transaction. Clipboard rejection means no deletion; Read cannot cut. Cross-Node object copy/drag uses D3 fresh identity/owner rewriting and complete D7 preview. Resource tokens in text do not automatically become valid across owners; URLs/images are not automatically fetched/imported. Never delete a source before an incomplete/unknown cross-Workspace copy; source Trash is a separate explicit submission.

Local body dragging proposes a fixed range move within one Draft; overlapping ranges are no-op or rejection, not guessed relocation. Grid/board dragging is a real Field/structure action. Visual destination columns map through columnId to the same Field value; RTL does not reverse meaning. Equivalent menu/keyboard target selection is required.

## 8. Undo, Redo, and history

Draft Undo/Redo records before/after bytes or typed input, selection, input serial, and group boundaries in the session log. Restoring content allocates a new monotonic serial and projection; never decrease serial or reuse old maps, and never write Core. Composition/paste each forms one group. Consecutive typing may group only for the same target without selection/mode/command changes, and never across prepare/submit or owners. New input clears corresponding Draft Redo, not committed history. Restored text is clean only when it equals Base.

Committed Undo is a new plan, preview, and OperationId against the current cut, not database rollback, old receipt reuse, or resurrection of an old revision. Interfaces §7 limits direct Undo to a D8 single-source edit whose complete current version/value still equal original after. Any later same-owner edit rejects direct Undo and preserves subsequent unrelated content. An explicit three-way Source proposal may be opened, without automatic overwrite or a claim of selective cross-revision Undo. Fully validate history access, current permission, and dependencies. Button/AT labels distinguish unsubmitted-input Undo from committed-operation Undo.

Redo is a new inverse intent against committed Undo, not resending the original committed request. Deletion/Trash inverse is available only through explicit D3 restore. Purge, external effects, unproved history, and two cross-Workspace operations have no fictitious atomic Undo. Recover an unknown decision before offering Undo. Current read permission governs history; caches cannot bypass it.

## 9. Staleness, conflict, and reanchoring

A new version invalidates old Document/Field/native selectors. Same key/title/text/digest or source A→B→A does not preserve authority/address. Exact local input logs may maintain Draft carets but do not establish D3 cross-revision Locator continuity.

Rebase retains Base/current/proposed and real differences. Core recomputes footprint, complete D2/D4 gates, and dependencies at the new cut, producing a new explicitly confirmed preview. Even nonoverlap does not authorize background commit. Unproved targets retain conflict proposals; fuzzy nearest-match writes are forbidden. Read-only conflict views disclose only currently authorized portions; losing full-source read cannot expose old full history as a comparison.

Reference clicks follow D3's independent entity/locator authorization, lifecycle, and stale order. Displaying historical authored Ref values does not prove resolvability. The five Annotation target arms below determine exactness; not every source change makes every target stale. Reanchoring may propose a current same-owner selection/element/Resource, explicitly showing old target, candidate, and lost context. User selection precedes Core validation of the new Value/current version. Equal-text alternatives have no default winner; old tokens remain unchanged.

D2 `targetStatus=resolved|stale` describes exactness only. D3 `suspended` is lifecycle, not a third targetStatus. An exactly matching target in Trash may remain resolved while navigation/actions are suspended. Restore removes suspension if the original bound version/Value remains unchanged; it creates no new Locator. “Unchanged” below requires a legal same-owner target, current qualification, and no independent invalidation.

| D3 Value/3 target | Unrelated Document source change | Resource-byte replacement | Trash/restore |
|---|---|---|---|
| document (owner) | Whole-document remains resolved | Does not change exactness | Owner Trash suspends; same-Ref restore lifts suspension; lifecycle alone does not make it stale |
| document_element (locator) | Stale when Document version no longer matches, even equal text/ABA | Unrelated Resource does not alter Document Locator | Does not repair staleness; unchanged bound version stays exact |
| document_range (locator) | Same; unchanged character position proves no continuity | Same | Same |
| resource (resourceRef) | Unrelated Document editing does not alter whole-resource exactness | Still denotes the same ResourceRef and remains resolved | Resource/owner Trash suspends; same-Ref restore can lift it |
| resource_region (locator) | Unrelated Document editing does not alter region exactness | Changed Resource version makes the old region stale | Restore does not resurrect old byte versions; resolved only if original binding is still exact |

Resource-region profiles must actually be supported. Purge/absence/denial follows original D3 order; invisible targets cannot be reported as stale. D2 projections and D3 Value/3 targets retain their own field shapes.

Field/carrier occurrences have no AnnotationTarget; use D4 Entry note. Wrapping raw ranges as document ranges cannot bypass this exclusion. An unsubmitted Draft selection may retain only a local Annotation proposal, never issue a D3 Locator/new Annotation. Commit explicitly, reselect at the real version, then create. D7 apply_suggestion retains exact target/version, atomic Document+Annotation planning, and original replacement text regardless of direction.

Unfrozen Resource-region formats explicitly report unsupported while allowing existing whole-resource targets. DOM pixels cannot invent regionTokens. Later D9 profiles must bind exact Resource version, geometry/page/direction, and accessible alternatives; current D8 does not claim arbitrary PDF/image reanchoring.

## 10. Properties, tables, collections, and action mapping

| User intent | Sole target/entry | Required preservation |
|---|---|---|
| Complete Source, Query/View definition edit | D8 document prepare → D6 | Complete current source version, actual footprint, D2/D4 and applicable D7 definition gates, full preview |
| Ordinary Write text | D8 fragment/minimal ordinary-body command, then document prepare | Complete edit map/origins, Core region/tree proof, input queue, one Undo group |
| Property append/replace/remove/member | D7 Field Actions/selection | Complete FieldId, raw Entry, version, Registry, duplicates, notes/provenance |
| Native cell/row/checklist/existing reorder | Each existing D7 Native Action with its exact EntityTarget/NativeSelector/tableLocator shape | Original Locator/range, trivia, complete D2 source, no row identity |
| Native columns/multirow generation/ragged repair lacking a closed adapter | Unavailable; disable generated Source routes; existing reorder is outside this exclusion | Future adapter needs D5 exact per-row effects/1,000-row cap; generic Source neither proves generating intent nor implements that command |
| Collection create/remove | D7 collection_create/remove | Explicit parent/title/source, requireMembership, complete post-query including top/take |
| Board column drag | One explicit D7 Field/collection intent | Unavailable without a unique writable Field; caption is not FieldId; inverse/aggregate is not writable |
| Bulk Fields/forms | Closed homogeneous D7 bulk_field | 1,000 explicit targets, actual Field/null/empty/type semantics; no Source bypass or arbitrary mixed batch |
| Trash/restore/copy/promotion | D7 d3_operation or specific promotion → D3 | Complete closure, actual owner/Ref, original wire12/Result9, distinct intents |
| Annotation creation/lifecycle | Original D3 create_annotation/lifecycle | Plain body, same-owner target, acyclic replies |
| Annotation edit/explicit reanchor | D8 annotation prepare → D6 | Same AnnotationRef, exact old version, complete legal new Value/3; no creation |
| Accept suggestion | Original D7 apply_suggestion | Exact current range, original replacement, same atomic D6 effects |
| Template instantiation | Valid D9 output → D7 create/collection_create | Unavailable without actual D9 output capability; no invented D8 template parser |

Existing Native capability is explicitly retained: `set_native_cell`, `insert_native_row`, `remove_native_row`, `reorder_native_rows` with proved trivia preservation, `toggle_checklist`, and `promote_native_row`/`promote_checklist`. Each uses complete original D7 parameters, authorization, D2/D4 revalidation, and its D3/D6 owner—not one generic NativeSelector. insert_native_row uses owner/expectedRevision/tableLocator/at/cells; reorder_native_rows uses owner/expectedRevision/tableLocator/rows as a complete unique permutation, rejecting when trivia preservation cannot be proved. Two or more rows do not turn existing reorder into unclosed multirow generation. The exclusion covers unsupported generated batches/TSV/columns/ragged repair and Source substitutes, not independent explicit existing Actions; splitting cannot masquerade as an undelivered batch capability.

Multivalue Fields distinguish spaces, empty text, missing, none, and invalid. Clearing input does not default to remove/replace-all. Preserve unknown namespace source and label unavailable. Display locale for dates/numbers/units does not change typed input. Explicit previewable conversion may exist, but submitted values must be D4 canonical; Arabic digits do not silently become canonical ASCII. This generation promises canonical input only; unfrozen localized conversion is unavailable.

Query/Read cells default to read-only. Explicit Action and target-column selection precedes evidence issuance. Sorting/paging/refresh writes no author content. Result reset invalidates selection/evidence, never selects a new row by screen index. Cross-page Select All names the complete current result epoch; prepare fixes deduplicated targets and commit cannot recompute an expanded set. Hidden columns/filters do not delete data.

## 11. Offline behavior, Drafts, backgrounding, and recovery

A qualified local Core can commit offline. Disconnected remote sessions retain only non-authoritative Drafts, clearly separating “Draft saved on this device”, “pending submission”, “outcome unknown”, and “committed”. Idle input, successful preview/HTTP, or a successful Draft disk write does not prove synchronized author source.

Drafts live in a device-private area partitioned by principal/authority/Workspace/complete target/Base version, not portable author source/DynamicBlock. They do not synchronize across devices by default. Explicit device policy exposes capacity/TTL; ordinary cache cleanup cannot silently discard dirty proposals. Quota/disk failure immediately reports unsaved Draft state and allows copying/explicit export of authorized personal data. Closing dirty/unknown windows offers retain/back/explicit proposal discard. Discard does not cancel a submitted request.

Crash recovery loads original request/state before reconnection/authentication and queries/replays that request; unknown OperationIds never generate automatic new requests. Backgrounding does not make deadlines/TTL infinite; Core's current state governs resume. Device Draft cleanup cannot remove planned Core pins/ledger. Without local persistence, label the Draft session-only rather than promising restart recovery.

## 12. Platforms, accessibility, and scale

Direction and Accessibility normatively defines visual/logical commands, direction authority, mirroring, screen-reader order, and platform matrix. Desktop, Server WebUI, and Mobile share Core outcomes/errors/permissions/commit; input devices alter affordance only. CLI/headless Server consume the same business requests without fictional caret/IME requirements.

Implementation Impact defines performance targets and capacity tiers as later measurement gates, not current results. Large documents preserve exact source/version; parsing/full-scope/Query/effects run asynchronously in budgeted Core. Virtualization affects viewport only: selection/IME/focus do not derive from recycled DOM positions. Query completeness and D5 page/action limits remain. If a complete valid projection cannot be safely rendered, disclose the large-document Source/Read capability limit instead of a fake partial canonical projection.

## 13. Alternatives and tradeoffs

| Complete alternative | Disposition |
|---|---|
| DOM/rich-text AST as editable author source, exported to AsciiDoc | Reject: second authority, unrepresentable syntax, original-byte drift |
| Separate save/Undo/selection for each surface behind shared API names | Reject: cannot guarantee common targets, IME, or unknown-submit state |
| Author-commit every keystroke/automatic submission | Not selected: temporary invalidity, IME, preview, and scope are not closed; autosave means Draft only |
| Source textarea only, removing Write/properties/collections | Simpler but insufficient for structured editing, repeated facts, and cross-surface needs; Source is the complete fallback, not the only experience |
| CRDT operation log as long-lived authority | Reject this generation: crosses D6 transaction/permission/relation boundaries; later collaboration produces Core checkpoints only |
| Portable direction sidecar/hidden attributes automatically copied | Reject unfrozen D2/D4 author facts; discardable presentation preferences plus original Unicode suffice at the cost of no portable block-direction setting |

Limits are explicit: a safe Write subset, full-source access cost, revision-bound Annotations becoming stale when their version changes, explicit submission, no Arabic UI translation, no resource-region profile, no realtime coediting, and no measured performance proof. Independent review must judge these against hard requirements rather than hiding them as implementation details.

## 14. Current observation, save protection, and coordination

Every Base source/version binding here means complete SourceObservation/1 from Interfaces. Selection, direction overrides, projection, mapBinding, late responses, and queued transforms bind the same complete Base and original draftSerial/source. Production CommitDomain, current observerDomain, and their epochs remain distinct. Equal revisions from different domains, externalSequence, equal text, and rebuilt I cannot stand in for an old observation. Updating Base first obtains authorized real complete after-data and a new current observation; the receipt remains original bytes. Base updates cannot overwrite later Draft/selection. PL-IR-01 current cross-replica Locator qualification awaits a foundational disposition; identical portable production versions do not prove old-token currentness here.

Reading/local proposals do not require complete-Action admission. Authorized external exact source and Draft remain available; physically invalid source uses dedicated repair. Ordinary full-source save may use ordinary+strict. Only trusted interactive entry, explicit user choice, and every Interfaces/D6 condition admit ordinary+replica_local+observed_only. Annotation, Undo, generated structure/bulk, Automation, managed_atomic, and complete Action use strict. The conservative restriction on undecodable saved D7 definitions remains. semantic_pending hides neither D2/D4 invalidity nor missing proof mandatory for an action.

Distinguish device Draft persistence, retained inputs, prepared, submitted_unknown/planned, sealed reliable/durable_observed_only, and portable publication. Explain three weak-save windows: (1) prepare retains the actually read B and user N; (2) external C observed before the last check conflicts with old Base while retaining the proposal; (3) C unobserved between last check and overwrite may be lost—retaining B/N does not claim retention of C. Unknown installation recovers the original request without success claims or a new ID. Offline icons do not infer protection; prepare is not Saved. True raw no-op says content unchanged and creates no source revision/H increment; equal-byte external admission is real management admission producing a current-domain version.

Conflict UI consumes currently authorized D6 ConflictRecord/2. D6 prepares only its closed source_merge, choose_source_head, and policy_choice. Complete identity/placement/Trash choices navigate directly to D3ConflictResolutionPrepare/1, never the deleted owner_resolution arm with only owner/action, nor UI-composed source/head replacements for named interfaces. D3 success returns original wire12 plus preparationBinding and submits through the single native D3 decision entry. D6 editing submits original d6_commit_request/2. Protected conflict-installation-purpose input is not an ordinary Observation and cannot enter D8 Base, Query, or SourceVersionRef. Until D3 IR revisions and complete D7 preview producers are independently accepted, affected new strong paths retain named integration gates without fabricated acceptance.

Real historical saved/planned/unknown owner/wire/pins/permissions/custody/original receipts recover first. Native D3 has no D6 planToken/PreparedIntent/expectedDomainFenceToken; D6 alone checks its own fields. New wire2/consumer/preview-TTL gates do not retroactively reject old decisions; adding fields does not upgrade history. Cross-Workspace target copy and source Trash remain two explicit decisions, each displayed separately, with no atomic Undo promise.
