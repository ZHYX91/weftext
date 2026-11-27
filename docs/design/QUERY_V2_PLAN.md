---
source_language: zh-CN
translation_of: QUERY_V2_PLAN.zh-CN.md
translation_status: synced
---

[简体中文](QUERY_V2_PLAN.zh-CN.md)

# Query refactoring scope and acceptance criteria

WefText is in development. It has no user notes, deployed queries or production data. Update the repository's examples and fixtures alongside the implementation. Each replacement covers readers, writers, examples, tests and both documentation languages, and removes the superseded format, parser branches and adapters.

## Repository baseline

This inventory describes main at `bd0aad3b`. See [RQ15](../REQUIREMENTS.md) and the [Query technical design](FEATURES.md) for product requirements. [MVP](../MVP.md) records the scope of runtime acceptance.

| Surface | Existing behavior | Acceptance focus |
| --- | --- | --- |
| `weftext-data/src/query.rs` and `query_advanced.rs` | Type-checked CEL, UNNEST, multiple sort keys, grouping keys and aggregates, DISTINCT, relations and saved calls | Share parameter and condition semantics. Specify ordering, empty states, cost and cancellation. Measure record lookup/equality and per-row context and AST cloning |
| Local, hosted and embedded queries | Scopes, plans, parameters and frozen inputs reach the shared evaluator | Use one definition contract for the API, saves, calls, native Typst, copy and sync checks. Execute one definition with different inputs |
| `app/src/query-panel.ts` and `query-plan-controls.ts` | Local table, list, board, calendar and numeric bar chart. Builder surfaces expose different operator sets | Align applicable operators and explanations. Qualify multiple metrics, embedded views and hosted views per actual surface |
| Text and search | The panel can compile user text into title or body CEL substring conditions | Keep exact substring semantics distinct. Specify scopes, tokenization, phrase, all, any and excluded terms. Compose search with property filters |
| Product checks | Linux frontend checks and real Windows Rust, HTTP and package gates exist | Run regressions through each product entry. Record design-model and source evidence separately from browser and platform results |

## User operations

1. Support nested all/any/negated groups within a documented depth, with editing, removal and reordering. Offer equality, inequality, all four ordered comparisons, contains, prefixes, suffixes, membership, emptiness and state predicates as applicable to each field. Bind any/all conditions to the same record in a collection. Support array expansion, unions and nested members.
2. Scope search to titles, bodies, headings and authorized properties. Specify Chinese and English tokenization, Unicode, case handling, term order and phrases. Test match locations, snippets and ordering. Fuzzy matching and relevance ranking require an implementation and quality evidence.
3. Specify relation types, directions, depth and deduplication. Start with direct and bounded two-hop traversals. Cover repeated edges, cycles, missing and unauthorized targets, excess fan-out and stable results. Greater depth needs its own cost model.
4. Cover today, yesterday, this and last week/month/quarter/year, the past or next N days and custom ranges. Freeze one instant, timezone and rule version for the run. Handle civil dates and instants separately. Cover month boundaries, leap days and daylight-saving transitions.
5. Declarations describe names, types, labels, help, required inputs and defaults. The caller supplies values separately. Cover text, exact numbers, booleans, date/time, enums, multi-select, node/tag references and explicit nulls. Validate names, types and defaults statically. Demonstrate reuse of the same definition with different arguments.
6. Align the builder and runtime for multi-key sorting, null/missing placement, stable ties, multiple grouping keys and metrics. Verify count, distinct count, sum, average, min and max for each applicable type. Mixed currency or units require an explicitly versioned conversion. Table, list, board, calendar, chart, summary and export consume a frozen result. Switching presentation preserves membership and revision.

## Expressions and execution

The visual and expression editors share a canonical plan. Evaluate CEL syntax, static checks, extensions, diagnostics and runtime costs against real workloads. A retained CEL profile needs consistent typed constructors, collection operations and stable diagnostics, and excludes ambient I/O, network, dynamic evaluation and arbitrary callbacks. Preserve the source of expressions the builder cannot edit losslessly. The full-plan editor is separate from ordinary condition editing.

Build each snapshot from one frozen input. Preserve absent, null, invalid and unavailable states. Report coverage and freshness separately. Keep order, occurrence identity and paging stable. Recheck current permission on every page and action, and stop delivery on revocation. Keep grouped, aggregated, distinct and ambiguous-origin rows read-only. Describe the precise consistency boundary across authorities.

## Performance evidence

Generate deterministic fixtures of 1k, 10k, 50k and 100k entities with representative field lengths, nested values and high-cardinality groups. Record hardware, build mode, data shape and seed. Cover indexed and unindexed filters, full text, collections, sorting, grouping, traversal, paging and repeated refresh. Record cold and warm p50/p95, peak RSS, allocations, index build/update cost, cancellation latency and concurrency. Compare index results with a full scan.

Enforce budgets for steps, time, input/decoded bytes, rows, result size, memory, nesting and fan-out where work or allocation occurs. Observe cancellation during scan, evaluation, sort, aggregate, index and serialization. Keep editing and navigation responsive and distinguish progress, interruption and partial results. Measure and remove quadratic paths, per-row registration, repeated parsing/cloning and unbounded materialization.

Indexes are rebuildable projections. Define version, coverage, freshness, incremental updates, crash recovery and permission filtering. Measure size, write amplification, update latency and query benefit separately for full-text, common scalar, type, tag and relation indexes.

## Delivery order

Use independently executable slices for the entry-point inventory, core conditions and parameters, execution and benchmarks, text search, collections and relations, relative dates and parameter forms, frozen result views, and removal of obsolete code and documentation. Each slice records success and refusal cases and the commands and outcomes. Before release, combine index updates/recovery/revocation, concurrency and cancellation, paging and changed-input scenarios. Record implemented behavior, tested behavior and outstanding acceptance per surface.
