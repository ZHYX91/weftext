---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 7db65eaf-0892-4e64-b62f-42b7988fdc99.

# D8 RTL and Bidirectional Interaction Mandatory Intake — Disposition of the 2026-09-01 Source

Candidate status: D8-FA-r01; complete owner afterimage, not independently accepted, activated, or implemented. Fixed source S is 7e18168dad3e6d120fce0dd607dc10fa7894e252. Original acceptance, model, and platform evidence retains only its original scope; this candidate is not implementation evidence. The D3 conflict adapter, current D7 effects producers, and current qualification of portable Locators across replicas still require their respective coordination and independent acceptance. These integration gates do not retroactively cancel recovery of real historical decisions.

## Purpose and authority boundary

This file preserves the complete controller-owned mandatory RTL obligations and evidence boundaries from fixed S. The original note handed chat-local requirements to a fresh D8 task. Its status=mandatory-review-input-not-frozen-decision, owner=D8-editor-and-cross-platform-interaction, d8_started=no, d4_changed=no, repos_weftext_modified=no, and brand_modified=no are source status from 2026-09-01, not current task state or instructions. This afterimage modifies no implementation, brand, or upstream snapshot, and does not independently activate an RTL model or Arabic localization.

The original task was Architecture Convergence and Clean-Slate Refactor - D8; its external control records were not published with the input. Current choices are in this batch's main contract, Direction, Interfaces, and Acceptance Matrix, subject to real independent review and controller disposition. Preserving intake/dispositions does not declare acceptance.

## Implementation observations and evidence limits

The 2026-09-01 targeted audit found only isolated RTL content handling, insufficient for full RTL UI claims. On 2026-10-02 the author refreshed targeted source observations read-only in the current main working tree, without running the UI:

- Desktop, Server WebUI, and prototype root language remains `zh-CN` at `apps/desktop/index.html:2`, `crates/weftext-server/webui/index.html:2`, and `prototypes/webui/app/layout.tsx:29`.
- User-controlled names use local `dir="auto"` at `crates/weftext-server/webui/app.js:269,334,1226` and `prototypes/webui/app/page.tsx:3864-3898`, with additional menu, restore, and transaction-preview targets. This demonstrates local content direction, not a global shell.
- The original scan found no mechanism establishing and propagating shell/editor RTL through global `document.dir`, root `dir="rtl"|"auto"`, `direction`, `writing-mode`, or `unicode-bidi`. The current targeted source search likewise establishes no complete mechanism. A sorting variable named direction is not text direction; a limited search does not prove repository-wide absence.
- Physical styles `border-right`, `border-left`, `text-align:left`, and asymmetric corners remain, including `prototypes/webui/app/globals.css:39,49,343,347,465,498,547,644,761`. Isolated `text-align:start` does not prove complete mirroring.
- Fixed S records product evidence for Arabic citation locales. Citation presentation proves neither Arabic UI translation nor complete RTL editing. Data fixtures cannot replace end-to-end caret/IME/focus/mirroring tests.
- The original audit found no explicit end-to-end RTL layout/focus/caret/selection/mixed-text/interaction case in the UI tests it inspected. This batch's bounded test-file search also obtained no such evidence. No real platforms were run; this is not a repository-wide absence claim.

The source also records existing public obligations: `docs/architecture/05-shared-navigation-information-architecture.md:29` requires RTL/mixed-text shared navigation, and `docs/specifications/09-testing-release.md:60,64,91` retains RTL in cross-surface editing/navigation/selection/IME/command acceptance. These are source locations, not claims that their current line numbers were reverified here.

A gap therefore remains between partial support and complete interaction acceptance. D8 explicitly addresses it instead of inheriting assumed complete UI support. Actual product support requires evidence for each platform.

## Nine mandatory questions and current dispositions

1. **Capability separation:** independently accept Unicode/Arabic content preservation, bidi interaction, RTL layout, and Arabic UI translation; none implies another. Direction §1/7.
2. **Direction authority:** for shell, Document/editor, blocks, inline content, names, tables, and Query/View, define ltr/rtl/auto selection, scope, inheritance, overrides, persistence, and accessibility APIs without a second content authority. Direction §2.
3. **Mixed content:** cover Arabic/Hebrew with CJK/Latin, European/Arabic-Indic digits, punctuation, URL/path/email, Node/Resource Refs, citations, formula/code, inline markup, tables, and diagnostics. Direction §3.
4. **Editing interaction:** specify visual/logical carets, selection, Home/End/arrows, Backspace/Delete, word/grapheme navigation, drag selection, clipboard, Undo/Redo, composition/IME, toolbars/menus/slash/palettes, Annotation, reanchor, and stale behavior. Direction §4, main §§5–9, Interfaces §3.
5. **Mirroring:** define boundaries for navigation, panes, menus/popovers, tables/boards, icons/chevrons, borders/spacing, scrolling/focus, and nonmirrored semantic icons/data direction. Physical CSS cannot silently decide semantics. Direction §5.
6. **Accessibility:** keyboard-complete operation, stable role/name/state, focus/reading order, screen readers, zoom/reflow, high contrast, reduced motion, and preserved meaning despite visual/logical differences. Direction §6.
7. **Cross-surface parity:** explicit Desktop/Server WebUI/Mobile matrix; devices change affordance or actual availability, not shared Core plans/errors/Refs/commits/content authority. Direction §7.
8. **Localization:** separately decide when application chrome is translated into Arabic/other RTL languages. Citation locale, dir=auto names, and successful storage do not pass localization. This generation claims no Arabic UI delivery. Direction §1/2/7.
9. **Performance/scale:** deterministic large-document/collection/long-name/virtual-navigation/rapid-direction behavior without locale-dependent ordering or another parser. Direction §6, Impact §2.

## Mandatory acceptance scenarios

The complete candidate and independent final review package include all these positive/negative cases:

- Arabic-only/Hebrew-only documents, names, properties, cells, Queries, Annotations, and search results through editing, save, reload, offline Draft, conflict, and recovery. Query/search remains read-only; edit/save means genuine author input or explicit Action.
- Mixed RTL/LTR paragraphs/UI labels with digits, punctuation, URLs, Refs, citations, code, formulas, invalid and protected source.
- Caret, selection, deletion, composition, copy/paste, Undo/Redo, Annotation targeting, and reanchoring at bidi boundaries.
- Mirrored shell/navigation/menu/table/board alongside explicitly nonmirrored semantic content.
- Keyboard/screen-reader focus and reading order across Desktop, Server WebUI, and Mobile.
- Honest reporting of unsupported localization/platform behavior, never inferred from partial content support.
- Regression evidence that direction changes preserve underlying Core action/plan/commit semantics.

All original 160 cases remain. New observation/save-protection/recovery and no-caret queue obligations are additional, not replacements. These are acceptance obligations, not product test results.

## Handoff and independent acceptance gate

Subsequent review treats this file as mandatory input, refreshes drift-prone implementation observations, distinguishes evidence from product authority, and carries every scenario into complete independent review/controller disposition. DeepSeek and GPT-5.6 Sol + Pro 5/5 named in fixed S identify historical process provenance, not current instructions to start chats, models, or agents. Current execution follows human authorization. Omitting full RTL/bidi disposition or substituting citation locale/dir=auto spot checks prevents D8 acceptance.
