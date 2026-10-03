---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 3e8f9777-29c5-4ee7-9098-b924becd255c.

# D8 Direction, Bidirectional Text, and Accessibility

Candidate status: D8-FA-r01; complete owner afterimage, not independently accepted, activated, or implemented. Fixed source S is 7e18168dad3e6d120fce0dd607dc10fa7894e252. Original acceptance, model, and platform evidence retains only its original scope; this candidate is not implementation evidence. The D3 conflict adapter, current D7 effects producers, and current qualification of portable Locators across replicas still require their respective coordination and independent acceptance. These integration gates do not retroactively cancel recovery of real historical decisions.

Revision: D8-FA-r01; candidate and part of the D8 specification. This document answers the mandatory RTL intake dated 2026-09-01. Content preservation, bidi interaction, RTL shell, and UI translation are independently accepted; none proves another.

## 1. Standards, versions, and product choices

This generation pins grapheme/default-word segmentation and bidi tests to Unicode18.0.0, using [UAX #9 revision52](https://www.unicode.org/reports/tr9/tr9-52.html) and [UAX #29 revision49](https://www.unicode.org/reports/tr29/tr29-49.html). Fixed S records an official-release check on 2026-09-23; this batch preserves that selected version and does not claim a new verification of release status. Implementation pins the same tables and corpus instead of changing semantics with each device's undeclared ICU version. UBA displays logically ordered characters; segmentation determines movement/deletion boundaries. Shaping, fonts, and wrapping still need real platform testing.

[Input Events Level 2](https://www.w3.org/TR/input-events-2/) informs identification of browser input/composition differences; its text does not prove browser behavior. D8 normalizes actual events instead of assuming every beforeinput is cancellable. [WCAG 2.2](https://www.w3.org/TR/WCAG22/) AA is the WebUI target; native surfaces also require actual platform assistive-technology tests. The keys, defaults, mirroring, and persistence below are Weftext product choices, not decisions delegated to a standard.

## 2. Direction authority, inheritance, and persistence

| Layer | Choice, default, and override | Persistence and boundary |
|---|---|---|
| Shell | User setting=`ltr|rtl|auto`; auto uses only the selected UI locale descriptor, falling back to ltr when no installed descriptor exists | Current user's device preference; the first content character never selects shell direction; changing it writes no author source |
| Document Read/Write | Document display override=`ltr|rtl|auto`, default auto; explicit value supplies paragraph base direction; auto uses each paragraph's first strong character, otherwise the current document fallback | Discardable personal reading preference keyed by principal+authority+NodeRef; fallback is current shell direction; never copied as another owner's author fact |
| Block/paragraph/cell | Explicit session override, then explicit document setting, then auto; auto examines each text flow independently and neutral text inherits document fallback | Override binds a source range in the current complete SourceObservation/1 or a draftSerial bound to that Base; discard/reselect on revision change, never infer cross-revision continuity from text |
| Inline/name/label | Ordinary user content is an independent bidi isolate, default auto; D7 View data labels use existing textDirection below; technical tokens use dedicated rules | No new author inline-direction syntax; a name cannot control adjacent menus/labels |
| Native table, Node grid, board | Container direction controls visual column placement; each cell independently uses auto or its session override | columnId and logical column order stay unchanged; preferences do not add unknown ViewSpec members |
| Query/View | Container layout can use current personal preference; data-label paragraph bases consume validated ViewSpec.options.textDirection exactly; CEL, ordering, time zone/context, row handles, and data remain unchanged | Discardable settings never enter QuerySpec, DynamicBlock, result schema, or author facts |
| Source/code/technical diagnostics | Default ltr skeleton, isolating each logical line/technical fragment; original text retains its bidi runs, with explicit logical-codepoint inspection | CSS override does not rearrange author characters or change source Bidi_Control; inspection markers never save back |

auto uses UBA first-strong rules, including isolate handling; European/Arabic-Indic digits do not detect locale. Empty, numeric-only, and punctuation-only text uses the table's fallback and exposes the changeable setting. Explicit ltr/rtl sets paragraph base level; it neither reverses strings nor overrides valid author Unicode direction controls.

D7 View data labels and container direction are computed separately. Every ordinary text data value, authored column/legend label, title, and description is its own text flow. Existing `options.textDirection=ltr|rtl` explicitly determines its paragraph base; `auto` uses that flow's first strong character and the View container for neutral fallback. Document display and block/session overrides do not enter the View or override this author option. Personal direction changes only the container and auto-neutral fallback. Typed Ref/UUID/FieldId, source coordinates, and canonical numbers remain dedicated ltr technical fields under §3, not ordinary inferred labels. Unknown textDirection fails D7 closed decoding. Editing saved textDirection is an explicit author-definition edit through D8/D6 prepare/commit; temporary shell/container changes do not modify ViewSpec, Query, or Action.

For example, shell=rtl and document override=rtl with View.textDirection=ltr still gives View labels an ltr base while its container can be RTL. With View.textDirection=auto, label=`שלום` has an rtl first-strong base, while label=`123` uses container fallback. Display changes never reverse scalars or change columnId, row handle, Query order, or submit request.

Executable direction table for one ordinary View label flow; dedicated typed values use §3:

| Shell/container | Document/session override | Saved textDirection | Label | Result base |
|---|---|---|---|---|
| rtl/rtl | ltr or rtl | ltr | שלום or 123 | ltr |
| ltr/ltr | ltr or rtl | rtl | ABC or 123 | rtl |
| Any/ltr | Any | auto | שלום | rtl |
| Any/rtl | Any | auto | ABC | ltr |
| Any/ltr | Any | auto | 123, punctuation, or empty | ltr |
| Any/rtl | Any | auto | 123, punctuation, or empty | rtl |

Reload preserves saved View textDirection. A retained personal container preference preserves fallback; if cleared, current shell determines auto-neutral fallback again. Neither case changes the saved View. Temporary Document/block/session overrides never cross into it.

Caret boundary acceptance is separate: an empty flow has one stop and clamps both ways. For one grapheme, a clamped first step at either document end followed by the reverse step may move and need not return to the origin. Bidi dual-affinity and soft-wrap visual stops cannot be deduplicated by logical point. Round-trip identity applies only to qualifying successful adjacent steps; changed layout epochs require new hit-testing.

App `lang` denotes the actual UI language. Content language is marked only from known author facts or the user's explicit reading-language choice; Arabic glyphs do not prove language. Direction and language are separate. The interface may use a manually selected rtl shell while retaining Chinese/English UI; this does not pass Arabic localization.

This generation adds no portable D2 block/inline direction attribute and does not interpret an ordinary header named `dir` as hidden control. Explicit author direction syntax preserved across devices/exports requires later explicit D2/D9 changes. Existing Unicode author characters remain exact. Deleting device preferences does not change document meaning, identity, or Core output.

## 3. Mixed text and isolation

The corpus jointly includes Arabic, Hebrew, CJK, Latin, European and Arabic-Indic digits, emoji, combining marks, punctuation, brackets, URLs, paths, email, Refs, citations, and formula/code. Isolate ordinary user text so names/clipboard content cannot affect adjacent buttons, paths, or error codes. DOM/native isolation is presentation only and never enters source or clipboard.

Complete Node/Resource/Annotation references, UUID, FieldId, operationId, source coordinates, versions, URLs, paths, and email technical fields display in independent ltr regions. Link labels may use auto and must be distinguishable from the target. Displaying a URL does not fetch it; clicking creates a navigation intent under product capability/security rules. A path is not a Ref.

Code, CEL, Query JSON, and inert formula text do not mirror operators or syntax order. Existing Core token/syntax mapping may segment technical display. Without exact mapping, show ltr source plus visible direction-control inspection, not a UI lexer that changes interpretation. Control/confusable warnings explain display risks without deleting or normalizing source; users can explicitly inspect codepoints/raw text.

Baseline samples, retained in logical order: `مرحبا 123 / ١٢٣ ABC 中文 שלום`, `A (שלום) 12:34`, `مرحبا https://example.test/a?x=1`, `x + שלום == 1`, `e\u0301` (actual combining marks are also fixtures), and family-emoji ZWJ sequences. Capture platform screenshots with the same logical text and caret positions; a screenshot alone cannot prove copy/delete behavior.

## 4. Keyboard, selection, and visual order

Each text flow and ordinary input region produces grapheme-boundary caret stops in one layout epoch, including logical source point and `upstream|downstream` affinity. Visual Left/Right selects adjacent stops on the current visual line. Bidi boundaries may give one logical point two visual positions; retain affinity instead of deduplicating to an arbitrary position. At line ends continue from the corresponding edge of the adjacent visual line; clamp at document ends. Only for collapsed selection, unchanged layout epoch, a first step that actually moved to an adjacent stop, and no intervening input/target/layout change must Left then Right (or the reverse) return to the same stop and affinity. A clamped first step has no round-trip requirement. A noncollapsed selection first collapses to the explicitly chosen visual end; collapse is not the inverse of adjacent movement.

Logical Previous/Next Grapheme follows source order. Word Previous/Next uses pinned Unicode18 default word boundaries and skips nonword gaps without device locale. No dictionary-based CJK enhancement is promised. Source codepoint mode is explicit, not default. Both selection endpoints are logical; Shift changes only focus. Rendering may produce several rectangles, but operations target one explicit logical range.

Home/End selects the leftmost/rightmost stop on the visual line. Document Start/End selects logical source start/end, using normal platform modifiers and explicit directory names. Paragraph Start/End is a separate logical command, never renamed with direction. Up/Down preserves a desired visual x, chooses the nearest valid stop on the adjacent visual line, and breaks equal-distance ties by current affinity then smaller logical position. Pointer hit-testing uses the current layout epoch; if it changed, repeat hit-testing instead of applying old pixels to new objects.

Backspace/Delete follows the main contract's logically preceding/following grapheme; direction never swaps them. Selection/deletion cannot split emoji, combining clusters, or CRLF. Character-level Source inspection is a separate explicit command. Copy follows logical selection order. Toolbars, context menus, and IME candidates anchor to the focus caret's visual rectangle while actions retain the same logical target.

Table arrows in navigation mode move to the visually adjacent cell, mapping through current columnId to logical columns. In edit mode arrows belong to the text editor; Escape leaves editing at the same cell. Tab/Shift+Tab follows logical column order to the next/previous actionable cell; mirroring changes placement, with traversal still inline-start to inline-end reading order. Enter opens the current cell editor or explicit preview, never submits during IME. Tree inline-end expands/enters children, inline-start collapses/goes to parent (physical Left/Right reverse under RTL); Up/Down follows logical tree order. Every operation has an accessible command without relying on remembering arrow direction.

## 5. Mirroring boundary

| Mirrors with shell/container | Retains semantic direction |
|---|---|
| Navigation at inline-start, inspector at inline-end; pane resize handles, submenu expansion side, preferred popover alignment, borders/spacing/start-end corners | Source character order, Node/Field identity, child ordinal, Query sort, source table column order |
| Breadcrumb separators/next-level chevrons, expandable tree chevrons, purely navigational back/forward arrows | Media play, brand/logo, download/upload, mathematical operators, code, clock graphics |
| Visual table/board columns and scroll affordances; keyboard hit-testing still maps to the same columnId | Calendar/Timeline earlier-to-later axes, number lines, chart axes/legend mapping: this generation places earlier/smaller on the left; D7 has no reverse-axis option and RTL does not introduce one |
| Pure shell navigation/layout decoration | D7 relation arrows denote real from→to; RTL never reverses edge/owner; geographic/process arrows follow data |

Viewport avoidance may move a menu to the other side without changing target or command. Logical CSS implements decided layout, not product semantics. Adapters normalize browser scroll-origin/scrollLeft differences. Focus, accessible order, and true column order are recorded independently; row-reverse must not yield incorrect tab order.

## 6. Accessibility contract

Creation, editing, target selection, preview, confirmation, conflict handling, recovery, reanchoring, and board dragging all have keyboard/AT-completable paths. Dragging offers a target-column/position alternative. Controls expose stable role/name/state and read-only/disabled reasons; errors associate with fields. Focus is visible and not fully covered by fixed toolbars. Announce state changes proportionately, without interrupting speech for each compositionupdate.

Screen readers follow logical content and hierarchy. Visual bidi order does not reorder accessible strings or cause presentation controls/decorative glyphs to be read. Expose selection direction/range, source line/column, cell headers/coordinates, stale Annotation, unsaved, and unknown-outcome states. Source/Write panes do not announce hidden duplicate copies. Mode switches restore focus to the corresponding target: ordinary regions use exact raw scalar coordinates; read-only elements/atoms/escapes/joins use Core origins to select exact source ranges and announce that these are not per-character writable maps. Promised origins in a valid projection cannot fall back to Source start or string search.

Required later configurations include 200% text zoom, 400% page zoom, 320 CSS px reflow, system high contrast, and prefers-reduced-motion. Single-column body reflow hides no content/action. Two-dimensional tables may retain labelled-region scrolling and row-detail access. Touch targets meet the platform baseline (at least WCAG2.2 AA target-size rules for Web); color is never the only encoding of permission, error, or selection. Apply the same checks to LTR/RTL.

Virtual lists expose real row/column counts or explicitly unknown counts, not twenty rendered rows as the total. Keep focused objects mounted or use an active-descendant scheme verified with actual AT. When a screen reader requests an unrendered item, resolve/load the real logical target before focus/announcement. Sorting/result reset invalidates the old focused row and returns focus to a stable container, never transfers its DOM position to a different row. The composition host cannot unmount.

## 7. Cross-surface matrix

All rows are target contracts. None becomes Supported merely from this candidate.

| Capability/scenario | Desktop local/remote | Server WebUI | Mobile local/remote |
|---|---|---|---|
| Canonical read/project/prepare/commit | Local Core/Server | Server only | Bundled Core/Server, with qualified backend |
| Source/Write/Read, Field/table/collection | Shared state; keyboard/pointer/accessibility commands | Same state, browser-event adapter | Same state, touch sheets/keyboards/screen reader; no reduced semantics |
| Native IME | Real OS/WebView IME | Real browser IME | Soft keyboard, handwriting/speech, hardware keyboard |
| LTR/RTL and mixed scripts | Evidence per OS/WebView/font | Evidence per browser/font/AT | iOS/Android layout, input, and AT evidence |
| Offline local commit | Local allowed; remote Draft only | Draft only | Local allowed; remote Draft only |
| Restricted clipboard | Explicit unavailability; cut never deletes first | Browser user-gesture/permission restrictions | OS/background restrictions; no background clipboard reading |
| Undo/reanchor/conflict | Same Core plans and complete preview | Same | Same; touch convenience never allows fuzzy reanchoring |
| Agent/conversion/template capability | Explicit D1/D9/D10 availability | Server capability only | D1 currently excludes conversion execution/delegation/approval and Agent entry; committed results may be read |
| Arabic UI translation | Not delivered | Not delivered | Not delivered |

Platform modifiers may differ; command semantics, targets, and errors do not. A specific host lacking safe input, full effects display, AT navigation, or capacity accurately reports the affected capability unavailable. It neither removes Mobile entirely from the formal direction nor labels untested combinations supported.

## 8. Disposition of the mandatory RTL intake

The nine questions map respectively to capability separation (§1/7), direction authority (§2), mixed content (§3), editing interaction (§4 and main §§5–9), mirroring (§5), accessibility (§6), cross-surface behavior (§7), localization (§1/2/7), and scale (§6/Impact). The acceptance matrix includes positive/negative Arabic-only, Hebrew-only, and mixed cases for save/reload/offline/conflict/recovery and identical Core requests. dir=auto, citation locale, and string preservation alone do not pass these scenarios.

Source EOLs and interparagraph blank lines in ordinary Write regions are separately visible and describable to AT. Their edit coordinates do not alter Read's D2 paragraph meaning. Empty-region/space/tab carets, Enter followed by per-EOL Backspace/Delete, reopening, and Undo/Redo follow Interfaces §3 without old DOM/revision dependence. An origin relation neither changes grapheme boundaries nor grants write permission.
