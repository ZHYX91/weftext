---
source_language: zh-CN
translation_of: D8-DIRECTION.zh-CN.md
translation_status: synced
---

[简体中文](D8-DIRECTION.zh-CN.md)

# A2 D8 Direction, Bidi, Layout and Accessibility

Status: author-resolved-pending-independent-review. This file carries the fixed-S Direction/Accessibility source, the RTL mandatory intake, the D6-FA afterimage, and current D2/D7/D8 owner contracts. It does not claim that any platform renderer, IME, font stack, assistive technology, or localization has been implemented.

## 1. Four capabilities remain separate

1. Unicode content preservation: source bytes, logical text, and Ref/Field/CEL identifiers are not rewritten by locale or direction.
2. Bidi text editing: caret, selection, hit testing, copy, and delete follow logical text plus UAX #9.
3. RTL shell: pane/tree/menu/grid/icon/toolbar visual geometry and focus path.
4. RTL localization: Arabic/Hebrew UI strings, plural rules, punctuation, control labels, and assistive-technology names.

Passing one capability never establishes the other three. Correct Arabic/Hebrew byte preservation is not evidence of an RTL shell or translated UI.

## 2. Unicode and normative data

Editor selection and grapheme behavior use Unicode 18.0.0, UAX #9 revision 52, and UAX #29 revision 49. D7 CEL Unicode 15.1 belongs only to the weftext.cel/1 NFC matching profile. D8 cannot use it to silently change editor grapheme or bidi behavior. A Unicode/UAX/data-version change requires a named successor, named data version, fixtures, and independent review.

CRLF is one logical EOL unit. NEL/LS/PS do not automatically become D2 newlines. UTF-8 byte, UTF-16 code unit, Unicode scalar, extended grapheme, and visual glyph offset are distinct coordinate domains.

## 3. Direction precedence

Direction sources do not substitute for one another:

- D2 authored/native direction follows only the real source/owner contract.
- D7 ViewSpec.options.textDirection controls only View data labels.
- D8 block/flow/session/device preference controls only interaction presentation.
- App locale/language controls only shell/localization.

Inside one applicable scope, explicit authored/native direction has priority. The corresponding View/block/session explicit preference follows. auto uses the first-strong rule over that scope's logical text. all-neutral content uses the defined inherited/fallback direction. Digits alone never choose direction.

A device preference never becomes portable content, a hidden sidecar, Query state, or shared presentation-policy authority. run-in/separate is Workspace presentation policy and is independent from bidi direction; run_in is not an RTL flag.

## 4. Caret model

```text
LogicalCaret = {
 logicalPosition,
 affinity:"upstream"|"downstream",
 layoutEpoch
}
```

One logical point may have two genuine visual stops. affinity chooses the visual side of the same logical point and is not a source offset or identity. Font loading, soft wrap, zoom, or container-width changes create a new LayoutEpoch; an old visual coordinate cannot be reused across epochs.

If the old logical point remains valid in the new layout, reconstruct from the same logical point plus affinity. If that affinity stop disappears, clamp only to the defined nearest stop for the same logical point. If source or Draft generation changed, rebuild from the new Draft map and current selection transaction. DOM node/index is never fallback identity.

## 5. Movement and selection

- Left/Right moves to the adjacent visual caret stop.
- Ctrl/Meta+Left/Right moves on logical word boundaries.
- Up/Down navigates visual lines; the preferred inline coordinate is valid only in the same layoutEpoch.
- Home/End moves to the current visual line start/end.
- Ctrl/Meta+Home/End moves to the logical document/flow edge.
- Selection retains logical anchor/focus/affinity; reverse selection never reverses replacement bytes.

Backspace/Delete removes the logically previous/following Unicode 18 extended grapheme. CRLF, combining sequences, emoji ZWJ sequences, and isolate controls are indivisible. Deletion operates on logical Source/Draft and never guesses from the visually left/right glyph.

## 6. Hit testing and async layout

Pointer/touch hit testing returns logical point, affinity, and layoutEpoch. The same visual x/y cannot be replayed across epochs. After asynchronous font loading, bidi paragraph resolution, line wrapping, or high-contrast font substitution, perform a fresh hit test. If the old point cannot be proved in the new layout, do not dispatch an edit.

Virtualized focus is owned by a logical entity/row/condition key rather than a DOM index. A focused or composing item is pinned into the virtualization window. If it cannot be pinned, explicitly cancel/reset the interaction before recycling; input cannot silently land in another row.

## 7. Logical copy and technical tokens

Copy Text always emits logical text order. RTL visual display never reverses characters. Ref, CEL, paths, email, URL, citation keys, and ASCII/numeric technical tokens use direction isolation for readability without changing source/value bytes. Copy Source Fragment emits exact raw Source order.

When a visual selection crosses bidi runs, execution still normalizes the logical anchor/focus range. Replacement bytes are never assembled from DOM visual-fragment order.

## 8. Shell mirroring

An RTL shell may mirror the navigation tree, pane affordances, menu/submenu geometry, breadcrumbs, drawer/sidebar geometry, and explicitly directional previous/next controls. It must not indiscriminately mirror:

- brand/logo;
- image/media content;
- math/STEM;
- chart numeric/time axes;
- graph relationship direction or arrows whose direction is data;
- code/CEL/path/Ref;
- calendar chronology;
- native-table logical column/Field identity.

Native-table Tab/Shift+Tab, arrow navigation, and structured editing always bind logical row/column identity. RTL changes visual placement only and never swaps FieldId, columnId, or relationship direction.

## 9. View interaction

D7 Query/View order remains semantic authority. RTL never reverses category/series/legend/panel semantic order, Query sort, network edge direction, or calendar time. A renderer may mirror screen geometry, while accessible table, export, and keyboard order still use the original typed data/order.

Chart hover/selection/focus uses the D7 projected key/column identity. A rendered bar index cannot be used to rediscover source. Hiding a series is reversible device-local presentation state, is marked as partial display, and changes neither Query/ViewSpec nor a full-export claim.

The View builder follows the same logical order. Binding pickers expose the current terminal-schema `columnId` and type, not a visual index. Reordering controls changes only a D7 member whose order is author-significant; CSS/RTL mirroring never rewrites that order. A basic builder that cannot expose a legal member provides an accessible advanced/source route rather than dropping it. Validation errors identify the exact field/binding in logical order, remain focusable, and do not reveal unauthorized schema/data.

## 10. Assistive technology

Every interactive control exposes real role, name, state, value, checked, expanded, selected, busy, and invalid state as applicable. Focus order follows logical task/Query order rather than CSS visual reorder.

Screen-reader behavior must satisfy all of the following:

- Document/Source is read in logical content order.
- Heading level uses the current effective heading level; H6-H9 is never flattened to H5.
- A titleless document does not announce filename as title.
- A table exposes its real row/column/header/span relationships.
- Annotation rich body exposes current semantic text; invalid R6 reports diagnostics instead of inventing a plain fallback.
- Every chart has an accessible table with the same data and order.
- partial, stale, loading, complete zero, and permission failure remain distinct.
- Hidden unauthorized count, value, or target never enters an accessible name or description.

IME preedit is not announced character by character. Composition start/update/end uses bounded state announcements. Error spans are reported by logical offset and a focusable condition/control; RTL never changes the source position.

## 11. Keyboard, zoom, and responsive behavior

Acceptance covers keyboard-only operation, 200% and 400% zoom, 320 CSS px, high contrast, and reduced motion. The focus indicator remains visible in bidi text and high contrast. Hover-only information has a keyboard/AT equivalent.

When a two-dimensional graphic cannot fit a narrow surface, D8 may use the same complete-data table/list alternative permitted by D7 and must explicitly report the graphical renderer unavailable. Sampling or truncation is not support.

Mobile touch targets, sheets, virtual keyboards, and hardware keyboards invoke the same logical intent. Screen resize or orientation change never changes Draft source, selection identity, Query semantics, or author permission.

## 12. RTL acceptance matrix

Arabic-only, Hebrew-only, and mixed CJK/RTL are covered separately across:

- editor typing/save/reload/offline/conflict/recovery/direction;
- Annotation edit/save/reload/offline/conflict/recovery/direction;
- list/table/board/calendar/timeline/search/chart;
- exact Source copy, logical copy, paste/cut;
- IME/composition;
- bidi dual affinity, wrapping, and font relayout;
- screen reader, keyboard, zoom, high contrast, and reduced motion.

This retains the fixed 98 RTL cases. The two languages or the different surfaces cannot be collapsed into one "RTL smoke test".

## 13. Nonclaims

Without real Desktop/WebUI/Mobile/browser/AT/IME/font-shaping execution, the corresponding capability remains UNRUN. Design fixtures, Unicode tables, and browser compatibility lists do not prove OS clipboard behavior, VoiceOver/NVDA/TalkBack behavior, Mobile keyboards, or a real renderer.
