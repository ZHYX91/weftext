---
_weftext:
  id: "7db65eaf-0892-4e64-b62f-42b7988fdc99"
---

# D8 RTL and Bidirectional Interaction Mandatory Review Intake 2026-09-01

status=mandatory-review-input-not-frozen-decision
owner=D8-editor-and-cross-platform-interaction
d8_started=no
d4_changed=no
repos_weftext_modified=no
brand_modified=no

## Purpose and authority boundary

This note is a persistent, controller-owned input for the future D8 review. It closes the handoff gap created by chat-local discussion: a fresh D8 task does not inherit this controller conversation and must receive the requirement through Weftext-Control.

This note does not start D8, freeze an RTL model, claim Arabic localization, reopen D1-D7, or modify any current implementation. Its observations are a 2026-09-01 snapshot that D8 preflight must refresh. Product conclusions belong in the future D8 candidate and decision after the required independent review and final gate.

The active D8 scope is Architecture Convergence and Clean-Slate Refactor - D8（外部控制记录未随本输入发布）.

## Current implementation snapshot

The current repository supports isolated pieces of RTL content handling, but the audited evidence is insufficient for a claim of full RTL UI support:

- Desktop, Server WebUI, and the WebUI prototype currently fix the root document language to `zh-CN`: `apps/desktop/index.html:2`, `crates/weftext-server/webui/index.html:2`, and `prototypes/webui/app/layout.tsx:29`.
- Some user-controlled names use `dir="auto"`, including Server WebUI rows at `crates/weftext-server/webui/app.js:269,334,1226` and prototype hierarchy/content/trash surfaces around `prototypes/webui/app/page.tsx:3864-3898`. This is local content-direction handling, not an application-wide direction model.
- The active UI scan found no global `document.dir`, root `dir="rtl"|"auto"`, `direction`, `writing-mode`, or `unicode-bidi` mechanism that establishes and propagates an RTL shell/editor state.
- The prototype stylesheet still contains physical-direction rules such as `border-right`, `border-left`, `text-align:left`, and asymmetric left/right radii; representative locations include `prototypes/webui/app/globals.css:39,49,343,347,465,498,547,644,761`. Some `text-align:start` usage exists, but it does not by itself prove layout mirroring.
- Arabic citation-locale behavior exists in the product evidence, but citation presentation is not evidence of Arabic UI localization or full RTL editor behavior.
- The audited UI test surfaces contain no explicit end-to-end RTL layout, focus-order, cursor/selection, mixed-text, or interaction acceptance case.

The current public repository contract already requires RTL/mixed-text behavior:

- `docs/architecture/05-shared-navigation-information-architecture.md:29` requires shared navigation to support RTL/mixed text.
- `docs/specifications/09-testing-release.md:60,64,91` keeps RTL in cross-surface editor, navigation, selection, IME, and command-surface acceptance.

Therefore the current state is a contract-to-implementation and contract-to-explicit-D8-review gap. D8 must adjudicate it; it must not inherit an assumption that current UI support is complete.

## Mandatory D8 review questions

D8 must explicitly answer all of the following:

1. **Capability separation:** distinguish Unicode/Arabic content preservation, bidirectional text behavior, RTL application layout, and Arabic UI localization. Passing one capability must not imply the others.
2. **Direction authority:** define how `ltr`, `rtl`, and `auto` are selected, scoped, inherited, overridden, persisted, and exposed to accessibility APIs across application shell, document/editor, block, inline content, names, tables, and query views. The source of direction must not become a second content authority.
3. **Mixed-content semantics:** cover Arabic/Hebrew text mixed with CJK/Latin text, European and Arabic-Indic digits, punctuation, URLs, paths, email, Node/Resource references, citations, formulas, code, inline markup, tables, and diagnostics.
4. **Editor interaction:** specify visual/logical cursor movement, selection, Home/End and arrow behavior, Backspace/Delete, word and grapheme navigation, drag selection, copy/paste, undo/redo, composition/IME, inline toolbars, context menus, slash menus, command palette actions, annotations, reanchoring, and stale-revision behavior.
5. **Layout and mirroring:** define which navigation, panes, menus, popovers, tables, boards, icons, chevrons, borders, spacing, scroll affordances, and focus order mirror; identify semantic icons or data directions that must not mirror. Physical CSS direction must not silently decide product semantics.
6. **Accessibility:** require keyboard-complete operation, stable accessible names/roles/state, sensible focus and reading order, screen-reader behavior, zoom/reflow, high contrast, reduced motion, and no loss of meaning when visual and logical order differ.
7. **Cross-surface parity:** provide a Desktop, Server WebUI, and Mobile interaction/capability matrix. Device differences may change affordances or availability, but not shared Core plans, errors, references, commits, or content authority.
8. **Localization boundary:** decide separately whether and when the application chrome is translated into Arabic or another RTL locale. An Arabic citation locale, `dir="auto"` on a name, or successful text storage cannot satisfy UI-localization acceptance.
9. **Performance and scale:** retain deterministic behavior for large documents, large collections, long names, virtualized navigation, and rapid direction changes without using locale-dependent ordering or a second parser.

## Mandatory acceptance scenarios

The D8 candidate, DeepSeek review package, and final GPT-5.6 Sol + Pro 5/5 gate package must include positive and negative cases for:

- Arabic-only and Hebrew-only documents, names, properties, table cells, queries, annotations, and search results through edit, save, reload, offline draft, conflict, and recovery paths;
- mixed RTL/LTR paragraphs and UI labels containing digits, punctuation, URLs, references, citations, code, formulas, and malformed/protected source;
- cursor, selection, deletion, composition, copy/paste, undo/redo, annotation targeting, and reanchoring at bidi boundaries;
- mirrored shell/navigation/menu/table/board interaction together with explicitly non-mirrored semantic content;
- keyboard and screen-reader focus/reading order across Desktop, Server WebUI, and Mobile;
- proof that unsupported localization or platform behavior is reported honestly and is not inferred from partial content support;
- regression checks demonstrating identical underlying Core action/plan/commit semantics regardless of UI direction.

## Required future handoff

The controller that creates the fresh D8 task must name this file as mandatory input in the task prompt. D8 preflight must reread the then-current implementation, refresh every drift-prone observation above, preserve the distinction between evidence and product authority, and carry the accepted scenarios into its external DeepSeek review and independent final gate. D8 cannot be accepted if RTL/bidirectional behavior is omitted or reduced to citation locale or `dir="auto"` spot checks.
