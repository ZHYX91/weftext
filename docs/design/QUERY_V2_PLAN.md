# Query v2 — greenfield redesign and implementation plan

Status: approved direction / implementation work plan. This document does not claim that planned runtime capabilities are implemented.

## 1. Project status and design premise

WefText is pre-release. There are no deployed Query definitions, real user notes, or production datasets that require migration. Query v2 is therefore a greenfield design, not a compatibility-preserving upgrade.

**Do not retain or build:** legacy QueryPlan shapes, legacy CEL profile behavior, old parameter records, compatibility aliases, dual parsers, adapters, migration code, deprecated fields, or compatibility tests. Delete obsolete implementations when their replacements land. Do not preserve old behavior merely because it exists in the current prototype.

This does not authorize careless implementation: the new system still needs strict correctness, security, permission isolation, snapshot consistency, deterministic results, cancellation, resource bounds, and tests.

## 2. Keep the valuable principles, not the old implementation

Retain only principles that survive fresh review:

- One query semantic model shared by global search, saved queries, embedded queries, navigation filters, API and result views.
- Schema-driven native types and exact comparison semantics.
- Explicit absent, null, invalid and unavailable states.
- Same-element semantics for nested collections.
- Frozen, deterministic result sets and stable pagination.
- Explicit scope and authorization; query results never imply write permission.
- Saved-query dependencies and parameter binding are explicit and statically validated.

Re-evaluate every current module and interface against these principles. If an abstraction does not materially support a required user outcome or a verified quality property, remove it.

## 3. User-facing query language and builder

The primary interface is a visual builder. Users should not need to learn CEL to perform ordinary queries.

### 3.1 Unified condition model

Provide nested condition groups with AND / OR / NOT, arbitrary nesting within a documented budget, and individually editable/removable/reorderable conditions. Support all applicable comparison operators: equals, not-equals, less/less-or-equal, greater/greater-or-equal, contains, starts-with, ends-with, membership, emptiness/presence and explicit state predicates.

Do not give the builder a reduced operator set relative to the execution engine. A feature is either supported end-to-end or clearly marked unavailable; no inert controls.

### 3.2 First-class search

Provide explicit text-search nodes, separate from scalar string containment. Define tokenization, phrase matching, all/any/excluded terms, field scope (title/body/headings/selected properties), case and Unicode behavior, ranking, match snippets and locations. Search nodes must compose with typed property filters in the same plan.

Do not promise fuzzy search or relevance ranking until the chosen index and ranking semantics are implemented and tested. Exact substring search remains a distinct operation.

### 3.3 Nested and relational data

The builder must handle schema-defined records, unions, arrays and nested fields without forcing ordinary users into raw CEL. Provide same-item any/all conditions, nested condition groups, array projection/UNNEST, relation traversal with explicit direction/type/depth and deduplication semantics.

Begin with direct and bounded two-hop relation traversal. Deeper graph traversal is a later capability requiring explicit cost and cycle semantics, not an accidental consequence of recursive query composition.

### 3.4 Time and parameters

Provide intuitive relative periods: today, yesterday, this/last week, this/last month, this/last quarter, this/last year, last N days, next N days and custom ranges. Freeze the reference instant, timezone and timezone-rule version once per run. Civil dates must never be silently converted to instants.

Define one parameter model independent of a particular query definition. Support text, number, boolean, date/time, enum, multi-select, node reference, tag reference and optional values; defaults, requiredness, validation, labels/help, and caller-supplied values must have explicit precedence. The same saved query should be reusable with different parameter values without cloning its definition.

### 3.5 Projection, sorting, grouping and output

Support multiple sort keys, explicit null/missing ordering, stable tie-breaking, multiple grouping keys and multiple aggregate metrics. Define count, distinct-count, sum, average, min, max and other metrics only where type semantics are sound. Currency and unit aggregation must reject incompatible values unless an explicit, versioned conversion is selected.

Keep query semantics separate from personal presentation. One frozen result should feed table, list, board, calendar, chart, aggregate summary and export. Changing the view must not silently rerun or alter membership.

## 4. Expression language decision

Keep CEL only if a focused evaluation demonstrates that its parser, static type checker, extension model, diagnostics and execution costs meet the product contract. Do not treat the current CEL profile as irrevocable.

If CEL remains, publish one deliberately small and coherent profile with:
- consistent, documented typed constructors;
- explicit collection operations and same-item binding;
- no dynamic evaluation, ambient I/O, arbitrary callbacks or network access;
- statically validated functions and parameter types;
- stable diagnostics and deterministic semantics;
- no duplicate constructors for indistinguishable types.

The visual builder and expression editor must compile to the same canonical plan. Unsupported source constructs must remain intact when editing is not lossless; builder operations must not silently delete or rewrite unknown expressions. Full raw-plan editing is a separate advanced surface, not a substitute for a complete builder.

## 5. Performance and scale are design-time requirements

Do not defer performance architecture until after feature completion. Do defer speculative micro-optimization until profiling demonstrates value.

### 5.1 Required work now

- Establish benchmark fixtures for 1k, 10k, 50k and 100k entities, with realistic field sizes and nested values.
- Measure cold/warm latency, p50/p95, peak resident memory, allocations, index build/update cost, cancellation latency and concurrent-query behavior.
- Benchmark simple indexed filters, unindexed filters, full-text queries, nested collection predicates, sorting, high-cardinality grouping, relation traversal, paging and repeated refresh.
- Inspect and remove avoidable O(n²) paths, per-row compilation/initialization, redundant cloning, repeated serialization and unbounded materialization.
- Define query budgets for CPU/steps, input bytes, decoded bytes, rows, result bytes, memory, nesting, fan-out and wall-clock time. Budgets must be enforced at actual allocation/work boundaries, not inferred from row counts.
- Make cancellation cooperative through scan, evaluation, sort, aggregate, index and serialization phases.
- Ensure background work never blocks editing or navigation; progress and partial/failure states are explicit.

### 5.2 Index design

Indexes are rebuildable projections, never a second source of truth. Design versioning, incremental maintenance, crash recovery, invalidation, permission filtering, coverage/freshness reporting and rebuild behavior before relying on them.

Start with evidence-driven indexes: full-text, common typed scalar fields, type, tag and relation indexes. Do not index every field by default. Record index size, write amplification, update latency and query benefit; choose indexes from realistic workloads.

### 5.3 Runtime implementation review

Audit the current CEL evaluator and data conversion paths for:
- repeated Context/function registration and AST cloning per row;
- repeated schema/shape resolution;
- ordered record lookup and equality complexity;
- collection conversion allocations;
- sort/group materialization and high-cardinality behavior;
- repeated parsing/serialization and duplicate snapshots.

For example, the current OrderedRecord implementation uses linear member lookup and performs member-by-member lookup during equality. Replace with an order-preserving representation that has an efficient lookup index, or remove the custom representation if a simpler typed value model is better. Benchmark before and after; do not optimize by intuition alone.

## 6. Correctness and security invariants

Every implementation and test must preserve:
- a coherent frozen input cut; never assemble a purported snapshot from unrelated current reads;
- exact type/value comparison; no implicit decimal/float, date/instant, currency or unit conversion;
- explicit missing/null/invalid/unavailable semantics;
- complete vs partial coverage and current vs stale freshness as separate dimensions;
- frozen-result pagination; no re-running against latest data for later pages;
- current authorization on every page and every action; revoke means invalidate and stop delivery;
- aggregate, distinct, computed and joined rows never inherit a representative row's write capability;
- explicit cross-authority consistency claims; no invented global atomicity;
- deterministic ordering and stable occurrence identity;
- bounded work and memory, cancellation, and honest unknown/failure states.

## 7. Implementation sequence and PR boundaries

Use independently reviewable vertical slices. Do not create one enormous PR or land speculative infrastructure without a consumer.

1. **Contract and deletion inventory:** map all current query entry points, duplicated models, dead code, tests and documentation. Decide the canonical plan/value/execution contracts. Remove obsolete duplicate definitions as the replacement is introduced.
2. **Core semantics:** canonical typed plan, static validation, parameter binding, condition groups, complete operators and deterministic errors. Add property-based and adversarial tests.
3. **Execution and performance foundation:** evaluator reuse/fast paths, complexity fixes, cooperative cancellation, budgets and benchmark harness.
4. **Text search:** explicit search AST node, index contract, incremental index and UI; test Chinese/English tokenization, phrase behavior, Unicode and ranking.
5. **Collections and relations:** nested builder, UNNEST, multiple grouping/aggregates, DISTINCT, bounded relation traversal and cost limits.
6. **Time and parameter UX:** relative time semantics, frozen context, reusable parameter definitions and input controls.
7. **Result presentation:** chart and aggregate views over the same frozen result; export reuses that result.
8. **Whole-system cleanup:** remove dead compatibility scaffolding, redundant adapters and obsolete tests/docs; update architecture and user docs to the single new contract.

A PR may combine adjacent steps only when it remains independently testable and reviewable. Each PR must include tests and evidence for its own acceptance criteria. If the available environment cannot run the full build or benchmarks, state exactly what was and was not verified; never claim local execution that did not occur.

## 8. Acceptance gates

- No legacy query parser, schema, parameter shape, adapter or compatibility branch remains.
- One canonical plan and typed value contract is consumed by all query surfaces.
- Every advertised operator and builder control has end-to-end tests.
- Common queries are expressible without CEL.
- Advanced expressions are statically checked and never silently rewritten.
- Performance benchmarks exist and report reproducible environment, dataset shape, cold/warm state and p50/p95.
- No known unbounded or avoidable quadratic path remains without a documented, measured reason.
- Index correctness is tested against a full-scan oracle, including incremental updates, crashes, permission changes and partial coverage.
- Query cancellation, concurrency, stale results, paging and authorization revocation have adversarial tests.
- Documentation describes only implemented behavior and clearly separates implemented, planned and unavailable capabilities.
