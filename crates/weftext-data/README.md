---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext structured data and Query

This crate accepts frozen source, centralized schema and inventory supplied by Core for pure reads, validation and source transformations. It does not open workspace files, allocate identity or commit saves.

`Schema::from_author_json` installs central configuration, rejecting duplicate JSON keys, unknown configuration members, duplicate identities, cyclic definitions, wrong ABIs, selectors and parameters. Business types and fields come only from configuration. `Schema::bundled` uses the current complete authoring fixture; Core still decides whether it is installed.

`project` and `patch_field` use the official Typst AST and Engine's standard-binding certification. Unknown members retain their original values; forms cannot overwrite computed fields. Narrow writes preserve untouched bytes, comments, literal spelling and line endings. Core must revalidate current basis, schema and permissions before committing. Source offsets are UTF-8 bytes; browser adapters must convert them to UTF-16.

Ordered list edits reuse certified author expressions across moves, additions, removals and nested changes. Known record members can be added/removed and union branches changed without rewriting preserved unknown members. Arbitrary record-key reorder requires the source editor. Native source/value processing is bounded to 8 MiB, 100,000 value nodes and depth 64; array transformations allow at most one million candidate comparisons. Exceeding a limit yields an explicit unavailable/error result. `search_terms` reads installed permanent name/alias FieldIds and reports dictionary coverage separately from its terms.

Query uses the actual `cel` 0.15.0 runtime. Type checking rejects implicit numeric, date and instant conversions; i64, native decimal, extended exact decimal and RFC3339 fractional seconds retain precision. That runtime executes CEL logical operators, collection macros and conditionals directly. Host budgets are checked independently before execution and cannot be absorbed by `|| true`.

Core captures and publishes Query inputs, coverage and freshness. The crate does not promote per-file observations to complete snapshots, change statistics through display limits, or inherit write permission from aggregate rows. Fields, relations, resources and compiled products still require actual frozen Core dependencies. The application applies returned patches through the same save entry point.

Advanced Query supports same-cut saved calls with exact current/pinned definitions, typed parameters, ordered unions, distinct, unnest, related traversal and terminal aggregates. Temporal providers execute Gregorian and finite-table periods, exact zone conversion and bounded recurrence expansion from the installed calendar/zone versions. Missing history, provider coverage, target closure or host budget remains partial/unavailable rather than an empty complete result.

Product error explanations use `UiMessage` and named parameters generated from the sole catalog. Original JSON, CEL and compiler diagnostics carry separate provenance; neither rendered text nor message keys determine invalid, unavailable, coverage or budget states. `Cell::Unavailable` separates the machine fact `kind`, typed summary `reason` and complete parallel `diagnostics`. Parallel diagnostics do not consume the bounded cause chain. The producer independently computes `TypedRefClosure.completeWithoutUnboundRootFields`: it excludes only unknown root fields; invalid, computed or unbound members in known fields still fail that exact proof.

The product-owned `diagnostics` returned by `expandRecurrence` are canonical structured records in the native-value/CEL ABI, containing message keys, parameters, original diagnostics and causes. They are no longer a list of interface text. Locale is applied only for presentation and does not change CEL results, statuses, occurrence coordinates, budgets or authored event fields. Each CEL evaluation owns a separate typed host-error channel. Exceeding its 128-item or 8 MiB diagnostic budget explicitly rejects that derived result while retaining the input and already collected diagnostics; `|| true` cannot absorb it.

`parse_unique_json` is the shared strict JSON entry for Schema, Export and Knowledge: it reports duplicate keys as typed product facts and preserves genuine serde syntax diagnostics separately. Callers own their byte/member admission limits and original input bytes. Query `unionAll` distinguishes an absent producer from an explicit empty producer list; the latter is rejected by typed preflight.

Relevant specifications: [SOURCE](../../docs/design/SOURCE.md), [FEATURES](../../docs/design/FEATURES.md).

Verification:

```text
cargo test -p weftext-data
cargo clippy -p weftext-data --all-targets -- -D warnings
```

Tests invoke the official Typst compiler and CEL runtime, covering standard-binding counterexamples, exact numbers, same-item conditions, order and duplicates, error absorption, budgets, unknown-member preservation, narrow writes, period boundaries and frozen results. Passing tests does not establish acceptance of every platform or the complete product.
