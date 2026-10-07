---
source_language: zh-CN
translation_of: D8-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D8-IMPACT.zh-CN.md)
# A2 D8 Implementation Impact and Acceptance Overlay

Status: author-resolved-pending-independent. This file states implementation/test obligations only. Product behavior is not claimed unless the final handoff names concrete executed evidence.

## 1. Core surface

Implementation keeps one Core parser/current D2 Asciidoctor 2.0.26 product and one real Source/Observation across read, Draft projection, maps/origins, prepare, preview and commit/recovery. Required adapters include Snapshot3/sourceOrigins, D8 wire2 Draft interfaces, coordinate/Edit Map, async IME, D8 prepare/PB3, Effect3 preview paging, committed single-source Undo, Annotation Value4/R6, presentation-policy graph, D7 Search interaction, and D7 View renderer/accessibility.

Missing rich adapters close only those rich/structured surfaces and never invalidate legal Source.

## 2. Source/parser conformance

Full current D2 legal source tests cover titleless/subtitle/deep headings/run-in, emphasis/strike, links/citations/STEM/Mermaid, attributes/includes/substitutions, unknown/inert source, and complete native-table spans/block/multiline styles. Unsupported rich editing must be unavailable/unrepresentable while exact Source round-trip still works.

Every Draft-edit positive compares complete parse tree, diagnostics, sourceOrigins and all unselected bytes. Incremental parsing passes only when equivalent to full parsing.

## 3. Edit Map and coordinates

Tests cover UTF-8/UTF-16/scalar/grapheme, CRLF/CR/LF, BOM/mixed EOL, emoji ZWJ/combining/CJK/RTL isolates, repeated equal cells, read-only escape/join/atom origins, maximal plainRegions, zero/nonzero EOF sites, nine EOL candidates, empty/blank/spaces/tabs, split/delete/merge/retype and exact unselected bytes. Fuzzy text, DOM index and latest-source relocation are negative cases.

## 4. Async/IME/Undo

Exercise reordered/late response, successor typing, owner/session switch, revocation, full IME lifecycle, blur/background, commands during composition, emoji/surrogate mutation, reverse selection, system Undo and cross Source/Write/Read stack. Late results do not overwrite later input; preedit never prepares; final composition is one Undo group; serials remain monotonic.

## 5. Clipboard

Real OS tests separately prove text/plain, exact-source copy, logical copy, cut write-before-delete, failure-no-delete, RTL logical order, TSV all-or-fail and Resource/download permission. HTML/RTF/code/script does not execute. If clipboard success cannot be established, cut does not delete. D9 rich clipboard remains truthfully unavailable until implemented.

## 6. Prepare/preview/recovery

Real Core covers PB3/PIntent3/Proof3/Effect3, TTL/budget/pins/OperationId/planToken and every preview page/byte slot. Missing one required preview piece blocks confirmation.

History corpus covers real wire1/PB1, wire2/PB2 and current PB3; saved/planned/unknown restores original bytes/request/pins/OperationId before current gates. Unknown is not repaired with a new request.

observed_only has only the qualified interactive one-Document positive. Annotation/Undo/structured/bulk/automation are negatives. Legal ordinary source paths remain usable without unrelated whole-Workspace proof.

## 7. Committed Undo

Exercise commit→fresh inverse→commit→fresh redo. Negatives include intervening commit, byte-equal different production version, missing before pin, additional effect, lost permission, stale format/Observation, cross-Workspace and old Draft selector. Every inverse/redo gets a fresh OperationId.

## 8. Annotation

Value4/R6 tests cover valid/invalid/absent body, Core attribution, whole vs range/region currentness, Trash suspend/restore, stable address + current observation, d9rg1, manual reattach, reconfirm, accept/reject race, hidden-target reject, atomic target mutation+accepted Value4, copy/import/fork identity mapping, and historical Value3/plain_text recovery. Invalid R6 never falls back to plain text.

## 9. Structured domains

Test all six row domains. Native table fixtures include span/block/multiline/header/footer/ragged/trivia and preserve Source validity when rich editing is unavailable. Existing set/insert/remove/reorder/checklist/promote APIs remain; native-column/TSV batch/auto-ragged generation stays closed. Field tests retain occurrence note/provenance/order and narrow phone-only proof. Query/collection covers duplicate Nodes, page/all, aggregate readonly, explicit parent, full postquery and dirty-Draft background commit behavior.

## 10. SEARCH-01–08

Product Search tests consume the current D7 fixture/QuerySpec/CanonicalGraph oracle directly. They cover three presets, source distinctions, exact/NFC/CJK/RTL, shortcut grammar/errors, visual/shortcut/plain roundtrip, opaque advanced state, permission/index states, save/reopen/copy/import dispatch and fresh hit resolution. IME preedit executes no Query and errors never fallback. Mixed-union duplicate occurrences are preserved.

## 11. View/render/AT

Real renderer tests consume ViewSpec/1 and require complete data, Query-side wide-to-long, explicit line order, Panel Partition, basis/domain checks, deterministic network and accessible table. Deferred layouts return unsupported_layout rather than being enabled by a chart library. Desktop/WebUI/Mobile separately prove rendering, keyboard/focus, AT/RTL, high contrast, zoom and reduced motion; Mobile fallback uses identical complete data.

## 12. Unicode/RTL/AT

Run Unicode18/UAX9r52/UAX29r49 plus all 98 fixed RTL cases. Claimed platforms use real NVDA/VoiceOver/TalkBack as applicable. Async font loading, soft wrap, dual affinity, hit-test epoch and virtual focus pinning need host evidence.

## 13. Performance

Retain p95 input≤50ms, layout≤100ms, at least 1000 events and three cold/warm runs, Desktop 10MiB/100000 lines, Mobile 1MiB/10000, one 1MiB line, deterministic 10× overflow, 100k navigation Nodes, 10k rows/page≤200, and editor memory Desktop≤256MiB/Mobile≤128MiB. Record hardware/OS/runtime/build/dataset/warmup/raw distributions. Budget failure keeps Draft/dirty and never truncates or splits.

## 14. D9/D10 boundary

D9 tests only named D8 intersections: rich clipboard unavailable, Resource-region validation, template no dirty-Draft overwrite, query_json result-only, export complete inputs. D10 tests explicit D8 proposals, no arbitrary patch/system clipboard/workspace mount, SearchContribution still D7 Query, and Mobile negative author-approval capability. These do not accept complete D9/D10.

## 15. Evidence classes

Report semantic model, repository/static checks, real Core, OS, GUI/IME/font/bidi, AT, replica/race/crash, performance, and migration/activation/deployment separately. At authoring time every product category above remains UNRUN; repository CI is consistency evidence only.
