---
source_language: zh-CN
translation_of: D8-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D8-IMPACT.zh-CN.md)

# A2 D8 Implementation Impact and Acceptance Overlay

Status: author-resolved-pending-independent-review. This file states implementation and test obligations only. Except for repository checks explicitly listed in the final handoff, it does not claim that product behavior has been executed.

## 1. Core implementation surface

Implementation must keep one Core parser and the current D2 Asciidoctor 2.0.26 product. Source read, Draft projection, edit map/origins, prepare, preview, commit, and recovery all consume the same real Source/Observation rather than a UI parser or cache.

At minimum, the implementation needs:

- a D2 Snapshot/3 and sourceOrigins adapter;
- D8 wire2 document/Draft entries;
- Draft Edit Map plus UTF-8/UTF-16/scalar/EOL/grapheme conversion;
- an async input/IME controller;
- D8EditPrepareRequest/3 plus PreparedEditBinding/3;
- D7/D6 EffectManifest/3 and EffectBytes/3 preview paging;
- a committed single-source Undo/Redo adapter;
- Annotation Value4/R6 read/draft/edit/reconfirm/suggestion support;
- presentation-policy complex-owner graph/set/resolve support;
- a D7 Search interaction adapter;
- a D7 View renderer/interaction/accessibility adapter.

If one adapter is unavailable, only the corresponding rich/structured surface becomes unavailable. Legal Source remains readable and editable through the independent exact-source path.

## 2. Exact-source and parser conformance

Tests cover the complete current D2 legal AsciiDoc surface: titleless and subtitle behavior; H1-H9; run-in/discrete/float headings; strong/emphasis/strike; links/citations; STEM/Mermaid; attributes/includes/substitutions; unknown/inert content; and native-table spans, block/multiline cells, headers/footers, ragged trivia, and advanced styles.

If a rich adapter is unsupported, the result is unavailable/unrepresentable for that adapter while exact Source round-trip remains valid.

Every positive Draft edit compares the complete parse tree, diagnostics, sourceOrigins, and every unselected byte. Rendered-text equality alone is insufficient. An incremental parser passes only when its complete result is equivalent to full parsing.

## 3. Draft Edit Map and coordinate tests

The corpus covers:

- UTF-8 <-> UTF-16 <-> scalar <-> grapheme conversion;
- CRLF/CR/LF, BOM, and mixed EOL;
- emoji ZWJ, combining sequences, CJK, and RTL isolates;
- repeated equal cells/text;
- escape/join/atom read-only origins;
- plainRegion maximality;
- zero-width and nonzero-column EOF sites;
- all nine prefix/suffix EOL candidates;
- empty, blank, spaces, and tabs;
- Enter -> paragraph split, delete-last-text, merge, and retype;
- exact preservation of every unselected byte.

Negative tests cover fuzzy string search, DOM-index addressing, and latest-source relocation.

## 4. Async/IME/Undo tests

At minimum, tests construct response reordering, a slow transform followed by successor typing, owner/session switch, mid-flight revocation, IME begin/update/end/cancel, composition blur/background, Enter/slash during composition, surrogate/emoji mutation, reverse selection, system Undo integration, and one shared Source/Write/Read Draft stack.

Assertions require that a late response never overwrites later input or drops the successor log; preedit cannot prepare; final composition forms one Undo group; Undo/Redo serials remain monotonic; and an old map never revives.

## 5. Clipboard and OS boundary

Real OS clipboard tests separately prove text/plain paste, exact Source copy, logical copy, cut write-before-delete, clipboard-failure-no-delete, RTL logical order, TSV representability rejection, and Resource/download permission.

HTML/RTF/code/script is never executed merely from clipboard content. If a platform API cannot prove clipboard success/atomicity, cut must not delete Source.

Until D9 rich clipboard is implemented, the capability remains truthfully unavailable. A D8 test cannot fabricate that capability.

## 6. Prepare / preview / recovery

Real Core decoder/conformance covers PB3, PIntent3, Proof3, Effect3, TTL, budget, pins, OperationId, planToken, preview cursor/epoch, and every preview page. Randomly missing a required page or byte slot before confirmation must prevent submission.

The historical corpus includes real wire1/PB1, wire2/PB2, and current PB3. saved/planned/unknown each restore their original bytes, request, pins, and OperationId. Current reprepare cannot "repair" an unknown historical decision.

The observed_only positive exists only for the qualified trusted interactive one-Document whole-source case. Annotation, Undo, structured, bulk, and automation cases are negative. A legal ordinary source path remains usable when an unrelated whole-Workspace proof is absent.

## 7. Committed Undo / Redo

A real sequence covers commit A -> exact current after -> Undo prepare/preview/confirm -> inverse commit -> fresh Redo prepare.

Negative cases include:

- an intervening other commit;
- byte-equal but different production-version ABA;
- missing before pin;
- an additional control/Field/identity effect;
- lost permission;
- stale format/Observation;
- cross-Workspace use;
- reuse of an old Draft selector.

Every inverse and Redo receives a fresh OperationId. The old receipt retains its historical responsibility.

## 8. Annotation

Current Value4/R6 tests cover:

- valid, invalid, and absent bodies;
- Core-only attribution;
- whole document/resource versus range/region stale behavior;
- Trash suspend/restore followed by fresh qualification;
- exact stable production address plus receiver Observation;
- d9rg1 PDF/image regions;
- manual reattach;
- reconfirm suggestion;
- apply/reject race;
- hidden-target rejection;
- real target-source mutation plus accepted Annotation in the same seal;
- copy/import/fork identity mapping;
- read-only historical Value3/plain_text recovery.

Invalid R6 never falls back to plain_text. Old suggestion evidence never revives.

## 9. Structured row / table / Field / Query

Each of the six row domains is tested independently. Native-table fixtures include spans, block/multiline cells, header/footer, ragged rows, and trivia. Unsupported rich editing leaves Source valid.

Existing set/insert/remove/reorder/checklist/promote APIs remain available under their real gates. Native-column generation, TSV batch generation, and automatic ragged repair remain closed.

Field tests cover equal-value duplicate occurrences, note/provenance/order, inverse-derived read-only data, and the phone-only narrow current proof.

Query/collection tests cover duplicate Nodes, page versus all, aggregate read-only state, explicit parent with no root fallback, requireMembership post-query top/take, and a background commit that makes a dirty Draft stale without overwriting it.

## 10. SEARCH-01-08 product tests

Product Search tests consume the current D7 Search fixtures, QuerySpec, and CanonicalGraph oracle directly instead of creating another expected-result engine.

The corpus covers:

- the three preset scope/source/empty behaviors;
- title/subtitle/filename/path/Resource distinctions;
- exact/NFC/CJK/RTL matching;
- shortcut quote/escape/keyword/precedence/incomplete/error spans;
- visual/shortcut/plain round trip;
- opaque advanced conditions;
- permission/count/rank/snippet/index states;
- save/reopen/copy/import version dispatch;
- fresh hit resolution.

IME preedit executes zero Queries. A UI error never falls back to a different Query. Mixed-union duplicate occurrences are preserved.

## 11. View / renderer / AT

Real renderer tests consume D7 ViewSpec/1 fixtures. Semantic drawing waits for a complete result. Wide data is converted to long form only by Query. line uses explicit sorting and verifies increasing x. The corpus also covers Panel Partition, quantity/date basis, deterministic network placement, and the accessible table.

A deferred layout returns unsupported_layout; a chart library cannot silently enable it.

Desktop, WebUI, and Mobile separately test pixels, keyboard, focus, AT, RTL, high contrast, zoom, and reduced motion. A Mobile fallback uses the same complete data.

Builder tests use the real current D7 definition decoder/validator and the existing definition-owner save path. They verify open -> edit -> cancel/save -> reopen; strict column/type validation; lossless retention of legal members not represented by basic controls; stale owner/revision and permission revocation; current Query/schema change; advanced/source routing; DynamicBlock retention; and the absence of a second View/Query store or hidden UI sidecar. A historical `.weftext-query view=...` sample is never silently rewritten: without a proved current Definition Transfer mapping it remains exact-source/advanced-only.

Mandatory §15.7 has ten separately identified product obligations, `VIEW-BLD-01` through `VIEW-BLD-10`: grouped aggregate duplicate, line order/gap/incomplete, pie zero/negative/duplicate/count, hierarchy/cycle/deferred status, Gantt/dependency/deferred status, boxplot/quantile/deferred status, incomplete ResultHandle, ACL/reset/offline/provider/renderer failure, Desktop/WebUI/Server/Mobile/CLI plus CJK/RTL/print equivalence, and Dashboard one-block failure isolation. Deferred layouts are tested for preservation plus explicit unsupported rendering, not implemented pixels.

## 12. Unicode / RTL / AT corpus

Unicode 18, UAX #9 revision 52, and UAX #29 revision 49 conformance is combined with all fixed 98 RTL cases, without collapsing them into one smoke test.

Claimed surfaces use real NVDA, VoiceOver, or TalkBack as applicable to test role, name, state, focus, logical reading, and composition noise. Async font loading, wrap, dual affinity, hit-test epoch, and virtual-focus pinning require real host evidence.

## 13. Performance gates

Retain all original gates:

- input p95 <= 50 ms;
- layout p95 <= 100 ms;
- at least 1000 events, with three cold and three warm runs;
- Desktop 10 MiB / 100000 lines;
- Mobile 1 MiB / 10000 lines;
- one 1 MiB line;
- deterministic reject/degrade at 10x over-limit;
- 100000 navigation Nodes, 10000 rows, page <= 200;
- steady editor memory Desktop <= 256 MiB and Mobile <= 128 MiB.

Measurement records hardware, OS, runtime, build, dataset, warmup, samples, and raw distribution. CPU-heavy parse/layout work does not run on the UI thread. Budget overflow preserves Draft/dirty state and never truncates source or splits the transaction.

## 14. Direct D9/D10 integration

D9 integration tests only the named D8 boundaries: rich clipboard unavailable, Resource-region validator, template output does not overwrite a dirty Draft, query_json is result-only, and export consumes complete input.

D10 integration tests only explicit D8 proposal use, no arbitrary source patch/system clipboard/workspace mount, SearchContribution still feeding the D7 Query path, and Mobile negative author-approval capability.

These checks never substitute for complete D9 or D10 module acceptance.

## 15. Evidence classes and current status

Evidence is reported separately for:

- bounded semantic/unit models;
- repository/static checks;
- real Core decoder/state machine;
- real OS/clipboard/filesystem;
- GUI/IME/font/bidi;
- assistive technology;
- multi-replica/race/crash;
- performance;
- migration/activation/deployment.

At author-candidate time, every product category above remains UNRUN. GitHub documentation/source checks establish repository consistency only; they do not establish D8 semantic acceptance. The author has repaired the D8 source-map navigation and per-obligation applicability audit; both remain pending fixed-SHA independent semantic review rather than self-accepted. Product/runtime evidence remains UNRUN.
