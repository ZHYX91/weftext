---
source_language: zh-CN
translation_of: D8-DIRECTION.zh-CN.md
translation_status: synced
---

[简体中文](D8-DIRECTION.zh-CN.md)
# A2 D8 Direction, Bidi, Layout and Accessibility

Status: author-resolved-pending-independent. This file carries the fixed-S Direction/Accessibility and RTL intake through the D6-FA and current D2/D7/D8 owner contracts. It claims no platform renderer, IME, font, AT, or localization implementation.

## 1. Four independent capabilities

Unicode content fidelity, bidi text editing, RTL application shell, and RTL localization are four separate capabilities. Passing one cannot establish another. Correctly preserving Arabic/Hebrew source bytes is not evidence of an RTL shell or translation.

## 2. Unicode data

Editing/selection/graphemes use Unicode 18.0.0, UAX #9 revision 52, and UAX #29 revision 49. D7 CEL Unicode15.1 is scoped only to the frozen weftext.cel/1 NFC profile and cannot silently change editor segmentation or bidi behavior. A Unicode/UAX/data upgrade requires a named successor plus data/fixture evidence and independent review.

CRLF is one logical EOL unit. NEL/LS/PS do not become D2 newlines. UTF-8 byte, UTF-16 code unit, Unicode scalar, extended grapheme, and visual glyph coordinates are never interchangeable.

## 3. Direction precedence

D2 authored/native direction, D7 ViewSpec textDirection, D8 block/flow/session/device presentation preference, and App locale are independent sources. Within an applicable scope, explicit authored/native direction wins; then the relevant explicit View/block/session preference; auto uses that scope's logical first-strong rule; all-neutral content uses the defined inherited/fallback direction. Digits alone do not choose direction.

Device preference is not portable content, a hidden sidecar, Query state, or shared presentation-policy authority. run_in/separate presentation policy is independent of bidi direction.

## 4. Caret model

A caret is logicalPosition + upstream|downstream affinity + LayoutEpoch. One logical point may have two genuine visual stops. Affinity selects a visual side and is not a source offset or identity.

Font loading, wrap, zoom, and container-width changes create a new LayoutEpoch. Rebuild a still-valid logical point in the new layout; conditionally clamp affinity only among stops for that same logical point. If Draft/source generation changed, rebuild from the new Draft map and input transaction. DOM node/index is never a fallback identity.

## 5. Movement and deletion

Left/Right is visual-stop movement. Ctrl/Meta+Left/Right is logical word movement. Up/Down navigates visual lines while any preferred inline coordinate is scoped to the same layout epoch. Home/End means visual line ends; Ctrl/Meta+Home/End means logical document/flow edges. Selection retains logical anchor/focus/affinity.

Backspace/Delete removes the logically previous/next Unicode18 extended grapheme. CRLF, combining sequences, emoji ZWJ sequences and isolate controls are not split. Editing acts on logical Draft/Source, never on the visually left/right glyph guess.

## 6. Hit testing and virtualization

Pointer/touch hit testing returns logical point, affinity and LayoutEpoch. Coordinates from another epoch are not replayed. Font/bidi/wrap/high-contrast relayout requires a new hit test or a provable logical reconstruction.

Virtual focus is owned by logical entity/row/condition identity rather than DOM position. Focused or composing content is pinned through virtualization. If it cannot be pinned, interaction is explicitly cancelled/reset before recycling; input is never silently redirected to another row.

## 7. Logical copy and technical tokens

Copy Text preserves logical order. RTL display never reverses copied characters. Ref, CEL, paths, emails, URLs, citation keys and ASCII/numeric technical tokens use visual isolation without changing bytes. Copy Source Fragment keeps raw source order.

Bidi visual selections normalize to logical anchor/focus for execution; replacement bytes are never assembled in DOM visual-fragment order.

## 8. Shell mirroring

Navigation tree, panes, menu/submenu, breadcrumb geometry, drawers and explicitly directional previous/next affordances may mirror. Brand/media, STEM/math, numeric/time chart axes, data-defined graph arrows, code/CEL/path/Ref, chronology, and native-table logical column/Field identity do not mirror semantically.

Native table keyboard navigation always uses logical rows/columns. RTL changes geometry only and cannot swap FieldId/columnId or relationship direction.

## 9. View interaction

D7 Query/View order remains semantic authority. RTL cannot reverse category/series/legend/panel order, Query sort, network edge direction or calendar time. Geometry may mirror, while accessible table/export/keyboard semantics retain the typed data/order.

Chart interaction uses projected keys/columns, never a rendered bar index as source identity. Locally hidden series are reversible device presentation state marked partial and do not change Query/ViewSpec or a full-export claim.

## 10. Assistive technology

Controls expose real role/name/state/value/selection/busy/invalid semantics. Focus follows logical task/Query order rather than CSS visual reorder. Screen readers use logical Document order, real effective deep heading levels, do not synthesize filename as title for titleless documents, expose native table headers/spans, use current Annotation rich semantic text, and provide a same-data chart table. Loading/partial/stale/complete-zero/permission states remain distinct, and unauthorized hidden counts/values/targets never enter accessibility labels.

IME preedit is not announced character-by-character. Composition state and errors are bounded and focusable. Error spans remain logical source positions under RTL.

## 11. Keyboard, zoom, responsive behavior

Acceptance covers keyboard-only, 200%/400% zoom, 320 CSS px, high contrast and reduced motion. Focus indication remains visible. Hover-only information has a keyboard/AT equivalent. A narrow renderer may use the D7-permitted same-complete-data table/list alternative while explicitly reporting the graphical layout unavailable; sampling/truncation is not support.

Mobile touch, sheets, virtual keyboard and hardware keyboard invoke the same logical intents. Resize/orientation does not change Draft bytes, selection identity, Query semantics, or author authorization.

## 12. RTL acceptance matrix

Arabic-only, Hebrew-only and mixed CJK/RTL are separately exercised across editor and Annotation edit/save/reload/offline/conflict/recovery/direction; list/table/board/calendar/timeline/search/chart; exact-source and logical copy/paste/cut; IME; dual affinity/wrap/font relayout; and screen reader/keyboard/zoom/high-contrast/reduced-motion. The fixed 98 RTL cases remain distinct rather than being reduced to one RTL smoke test.

## 13. Nonclaims

Without real Desktop/WebUI/Mobile/browser/AT/IME/font-shaping execution, these surfaces remain UNRUN. Design fixtures, Unicode tables, and browser compatibility lists do not prove OS clipboard or VoiceOver/NVDA/TalkBack behavior.
