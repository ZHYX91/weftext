---
source_language: zh-CN
translation_of: D9-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D9-IMPACT.zh-CN.md)

# A2 D9 Implementation Impact and Test Outline

Status: **author-resolved-pending-independent-review**. This is an implementation obligation list, not implementation evidence.

## 1. Replacement boundary

Core keeps D2/D4 domain interpretation and D3/D6 transaction authority. Conversion/Office/XML/model/runtime dependencies remain in optional coordinator/worker packages. The implementation must replace unpublished legacy ImportIr/YAML proposal decoders, free provider command/fallback aliases, old attr/record/H1–H9/formula-reorder/broad-view template parsing, free export dictionaries, rowHandle identity and D9-private Locator aliases as one migration set; historical research files remain historical evidence only.

Current implementation must target D3 wire13, PAB4, Effect3, D2Snapshot3 and fresh ExportPlan/Receipt4. It must retain exact decoders for genuinely stored Plan/Receipt1-/2-/3 and other historical records instead of dual-writing or migrating them.

## 2. I01–I12 gates — all UNRUN

| Gate | Required evidence |
| --- | --- |
| I01 IR/codec | Full strict decoders for closed members/unions, duplicate/unknown/null/Unicode/Counter/order/budgets, complete format coverage, independent parser/render comparison. |
| I02 hostile files | ZIP aliases/traversal/link/device/bomb, ODF repeats, XML DTD/XXE/entity, active/encrypted/unknown variants, malicious stdout/slot/exit-zero truncation, all limits. |
| I03 OS sandbox | Named Windows/Linux/macOS builds proving file/network/process-tree/resource/cleanup isolation, not timeout/container labels. |
| I04 mapping/admission | Complete D2 parse/product, D4 Registry/types/qualifiers/cardinality/references/control, fresh-root authority, ConversionInput/Result9/PAB4/Effect3 byte closure. |
| I05 real ImportJob | 10k Nodes/10 batches/beyond-memory input, SCC/coupling groups and real D6 fault injection before/after commit, lost receipt, restart, revoke, cancel, TTL/pins. |
| I06 Office template | Real Word/WPS/LibreOffice DOCX/ODT/XLSX/ODS, split/mixed styles, escaping, invalid XML scalars, typed none/complex values, visible native selector, 0/1/N repeats, merges and limits. |
| I07 document output | H1–H9 current source semantics, target losses, body/bibliography single placement, style bundles, CJK/RTL/AT/fonts/pagination on named Office versions. |
| I08 typed export | All D7 result domains, reset/order/bag/ties/none/graph, arbitrary-precision values, date/instant, formula-injection negatives and typed spreadsheet output. |
| I09 publication | Proved create-only filesystem boundary, ENOSPC/name conflict/user move/flush/rename crash/revoke race; unknown never republishes under another name. |
| I10 region | Crop/MediaBox/UserUnit/Rotate/EXIF/density/rounding, d9rg1 + l1 currentness, copy/fork, stale/not-visible and keyboard/AT use. |
| I11 surfaces | Desktop/CLI/Server/WebUI parity, Mobile negative capability, D1 overlap reason priority, nondisclosing errors and D8 generated-proposal/Draft conflicts. |
| I12 release/naming | Full dependency/SBOM/license/model/font/platform install/uninstall, no provider/config backdoor, legacy-name scan, capability entries bound to real gates. |

Passing one profile opens only the exact format × operation × variant × route × platform × version tuple. Library/profile semantics changes rerun affected gates.

## 3. Mandatory §14 Office binding implementation

The visible qualified selector grammar in D9 Main/Schemas is an implementation requirement. The compiler must parse exact JSON-string components and canonical occurrence suffixes from template bytes, compile them to D9NativeTableSelector/1, enforce shortest-unique qualification and verify every repeat-band token resolves to the same table/rowset/order.

Fixtures must cover:
1. duplicate tables with same leaf;
2. multi-row header shortest suffix;
3. same title + full path requiring table/column occurrence;
4. CJK, RTL, combining, emoji, space, slash, double-colon, quote and bracket text;
5. repeat-band same-source unification;
6. later collision causing fresh ambiguity while frozen Plan stays fixed;
7. ordinary Document table versus schema-bearing collection versus ordinary no-template XLSX;
8. copy/move/style and stale template;
9. no-template XLSX positive path;
10. unavailable Mobile, corrupt/malicious input and stale authority.

Hash-derived nt_/nc_ names are tested only as internal Plan keys or genuine historical authoring; they are not the new qualified user spelling.

## 4. Workers and provider evidence

Docling Lite current install evidence states completeForExecution=false and therefore remains unavailable. XPS/OXPS, OFD and CAJ/HN must not inherit availability from candidate libraries, extension recognition or a different variant. Every provider record must bind exact dependency versions, license/distribution decision, model/font assets, sandbox evidence and corpus tests.

Worker budget accounting is cumulative across route steps/retries. Cleanup must prove full process-tree termination; uncertain cleanup quarantines outputs and disables the route. Worker output is independently decoded/validated before pinning.

## 5. Import and mapping evidence

CSV fixtures include quoted newline, doubled quote, duplicate header, ragged records, empty file and exact Unicode. Workbook fixtures distinguish numeric lexeme, blank/absent/empty, formula/cache, hidden sheet/row/column, merge anchor/covered cells and source coordinates. Page/flow fixtures retain complete geometry/reading order/unrepresented issues. A coverage miss must fail even if the IR decoder passes.

D4 mapping tests consume real Registry definitions and prove positive and negative type/qualifier/cardinality/relation/Calendar cases. No label-based Field mapping or fresh-root permission borrowing is accepted.

## 6. Node Template evidence

Tests must cover duplicate source, self/cross-template fresh subject rewriting, owner-local Resource remap, external-current reference policy, Annotation omission index without body disclosure, parameter type checking, title/body_text/field_append overlap and D2 reparse equality.

Parent-import and simple collection branches are tested against their original D3/D7 requests. sourceSubjectBindings plus receipt resultAllocations must join uniquely; no second identity map is persisted.

## 7. PAB4 and recovery evidence

Fresh current tests use PAB4 and wire13 mode-legal shapes. Old PAB1/2/3/wire11/12 fixtures are recovery-only and never silently upgrade. MinimumMapping is proven before large record reads. Full preview bytes/effects exist before page 1 and pagination performs zero semantic reruns.

saved/planned/unknown cases preserve original request/OperationId/pins/owner and do not depend on expired preview TTL or current business validation except current disclosure/custody fences.

## 8. Export evidence

Tests cover exact-source/resource/query_json generationPolicy=none with renderer registries unavailable; rendered document D2Snapshot3 + D8 presentation binding; body/bibliography selection; narrow Field/Query/native_table no-extra-body-read; graph/scalar/rows complete values; canonical set ordering; current output-name validity/PortableAlias/reserved-name rules; fresh Plan4/Receipt4 plus strict historical Plan/Receipt1-/2-/3 dispatch.

Initial loss report, data bytes, loss-report.json, manifest.json and stagedOutputs are frozen and verified before confirmation. Any mutation after confirmation must fail rather than re-render.

## 8a. Annotation content and Mandatory §15 View evidence

Annotation conformance must cover a normal readable Value/4 export; the minimal target-hidden case where R6 body/attribution/reply remain exportable while target context is unavailable; portable-backup canonical record bytes; independent source-history versus target-context disclosure; changed revision/Observation/record-pin staleness; contradictory pins; authorization loss; and exact saved/planned/unknown dispatch. annotation_index must fail as a body source, and no D7 annotation_body semantic string may substitute for the complete record carrier.

View conformance consumes every D9-applicable Mandatory §15 fixture, not just one bar example. It must prove complete-result and D7 runtime validation before Plan freeze; long-form/order/domain/unit/empty-none-zero/panel/color/a11y semantics; negative/duplicate/invalid-order failures; incomplete first page; authorization reset; renderer/profile unavailability; font/color/page/alt-text losses; CJK/RTL/print equivalence; and Dashboard block isolation. The supported first D9 chart profile covers metric/bar/line/scatter/pie/heatmap only. Deferred tree/treemap/sunburst, Gantt and boxplot/quantile fail current ViewSpec closed decoding with unsupported_layout; a D7-supported current layout such as network that lacks a named D9 chart profile, or a supported six-chart layout whose backend/profile is absent, returns renderer_unavailable. Neither path may substitute data rows. Office View routes also rerun the applicable Mandatory §14 template/style/repeat fixtures.

## 9. Typed formats and image evidence

CSV dangerous starters include ASCII/fullwidth and leading Unicode White_Space. TSV embedded separators/newlines reject. XLSX/ODS keep text as text, preserve arbitrary-precision numeric policy and distinct none/empty/absent semantics, and never calculate formulas.

Image physical size uses actual density facts, exact ratios and half-even rounding. Missing/invalid/conflicting density branches are distinct. Explicit imageSizes is output layout policy, not source evidence.

## 10. Publication and Resource handoff

Real filesystem tests cover create-only atomic bundle publication, flush ordering, conflict, crash before/after rename, user move, restart and revoke race. Recovery never changes the final name. Fresh external PublicationReceipt/4, D9PrintReceipt/1 and author Resource receipt are three distinct outcomes; historical PublicationReceipt/3 remains independently recoverable.

## 11. Evidence ledger

Historical bounded evidence remains separate: 57, 63, 59, 82, 90, 130, and the 12-scenario chain. None is rerun or upgraded by this author batch. The current Design documents workflow may prove repository consistency only.

All I01–I12, real workers, Office/WPS/LibreOffice, OS sandbox, real D6 fault injection, performance, release and deployment remain **UNRUN** until actual implementation evidence exists.
