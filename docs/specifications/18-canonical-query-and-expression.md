---
source_language: zh-CN
translation_of: 18-canonical-query-and-expression.zh-CN.md
translation_status: synced
---

[简体中文](18-canonical-query-and-expression.zh-CN.md)

# Canonical Query and `weftext.expr.v1`

This specification defines canonical Query and `weftext.expr.v1`. Delivered callers use the canonical Query outer grammar, lexical context, typed domains/scopes, bounded execution, and stable result identity through the explicitly scoped `weftext.query-expression-subset.v0` capability. The complete evaluator and reusable Template bindings remain separate capabilities; unsupported expressions must fail with a precise diagnostic. Superseded query syntax is accepted only by an explicit one-time migration and is never a runtime alias.

## Authority and separation

Canonical Query is a derived, permission-filtered read over portable managed authority. Core alone parses, type-checks, resolves scope, evaluates, orders, and returns action evidence. Delivered Query callers transport that Core request/result rather than implementing a second parser or evaluator. Exporters, templates, saved-query storage, and agent actions acquire no implicit execution authority merely because the Core subset exists.

`weftext.expr.v1` is the common expression substrate used by Query expressions and Template bindings. Query owns `from`/`scope`/projection/order clauses. Template owns slot declarations and sidecar bindings. An expression is not a Query, and a Template slot cannot contain or execute Query clauses.

`weftext.query-expression-subset.v0` is an honest compiler-capability identifier serialized in derived Query plans so Core can reject forged or stale plans. It implements only the expression forms exercised by the current canonical Query runtime, including explicit source/context references, fixed predicates, null behavior, and day offsets. It must never appear in authored AsciiDoc, Template bindings, saved definitions, or compatibility negotiation, and no caller may rewrite an unsupported `weftext.expr.v1` expression into that subset. Adding the remaining expression features extends or replaces the derived capability in one Core compiler; it does not add another Query grammar.

## `weftext.expr.v1` values and literals

The closed value types are:

| Type | Literal or construction |
| --- | --- |
| `string` | JSON-compatible double-quoted UTF-8 string |
| `bool` | `true` or `false` |
| `number` | exact portable base-10 decimal defined below |
| `null` | `null` |
| `date` | `date("YYYY-MM-DD")` |
| `instant` | `instant("RFC3339-with-explicit-offset")` |
| `duration` | unquoted day-only `P1D` through `P36500D` |
| `UUID` | `uuid("lowercase-uuid-v4")` |
| `list<T>` | `[expr, ...]`; elements have one compatible type |
| `record` | `{"name": expr, ...}` with unique literal keys |

A number literal has an optional `-`, decimal digits, and an optional fractional part. It has no leading `+`, exponent, radix prefix, separator, NaN, or infinity. After removing the decimal point and insignificant leading zeroes, its signed coefficient has at most 34 significant decimal digits; its scale is `0..18`. Zero has one coefficient digit. Values are exact mathematical decimals, so comparison may normalize trailing fractional zeroes but never rounds. A literal, constructor result, transport value, or operation outside coefficient/scale bounds returns `numeric_overflow`; there is no implicit rounding or binary floating-point authority.

Duration v1 is exactly an unquoted `P<n>D` with integer `n` in `1..36500`. Hours, weeks, months, years, fractional/sign-prefixed values, zero, and other ISO-8601 duration forms are invalid. This keeps `context.today + P14D` one canonical day-arithmetic surface.

There is no implicit string/number/date/UUID conversion. Record member access uses `value.member` for closed identifier fields. Literal property maps use bracket access such as `node.document.properties["名称"]`; dot access does not reinterpret arbitrary property names. Indexing a missing property returns null. Unknown closed-record members are type errors.

## Operators and pure functions

The fixed v1 operators are:

- comparison: `=`, `!=`, `<`, `<=`, `>`, and `>=` over compatible scalar types;
- boolean: `not`, `and`, and `or`, with precedence parentheses, `not`, `and`, then `or`;
- null: `is null` and `is not null`;
- membership: `value in list` over one compatible element type; and
- temporal arithmetic: `date|instant + duration`, `date|instant - duration`, and same-kind temporal subtraction to `duration`.

Except for `is null` and `is not null`, either operand being null makes an ordinary `=`, `!=`, `<`, `<=`, `>`, or `>=` comparison a `null_comparison` type error. Boolean operands must be bool. For membership, the right operand must be a non-null homogeneous list of the left operand's non-null base type; a null left value returns false, null list elements never match, and a null right operand is a type error. Division, numeric arithmetic, regex, user-defined operators, overloaded string `+`, assignment, mutation, and implicit truthiness are absent from v1.

The only callable functions are:

| Function | Result |
| --- | --- |
| `contains(string, string)` | literal case-sensitive containment |
| `starts_with(string, string)` / `ends_with(string, string)` | literal case-sensitive prefix/suffix |
| `format_date(date, format)` | locale-independent ASCII Gregorian formatting defined below |
| `length(string|list)` | scalar/list length as number |
| `concat(string, ...)` | concatenated string, at least one argument |
| `coalesce(value, ...)` | first non-null compatible value |
| `date(string)` / `instant(string)` / `uuid(string)` | validated typed construction |

No other function is available. In particular there is no filesystem, network, environment, process, secret, ambient clock, random, locale lookup, `eval`, dynamic loading, reflection, extension callback, Query execution, or template execution.

`format_date` accepts only the tokens `YYYY`, `MM`, and `DD`, each at most once, plus literal ASCII `-`, `/`, `.`, `_`, and space separators in a format string of at most 64 bytes. Tokens emit zero-padded proleptic-Gregorian year, month, and day; letters, escapes, names, variable-width fields, and every other token are invalid. Output is capped by the ordinary 4,096-byte string limit. The function is locale- and timezone-independent and reads no ambient state: `format_date(context.today, "YYYY-MM-DD")` formats only the explicitly supplied date. Locale- or timezone-sensitive formatting would require a separately versioned function with explicit arguments.

## Evaluation roots and context

Query expressions may access only their declared row alias, `this`, and `context`. Template bindings may access only `input` and `context`. Each root is a closed record type supplied before evaluation.

`context` has exactly:

| Member | Type |
| --- | --- |
| `context.today` | date |
| `context.now` | instant |
| `context.timezone` | string |
| `context.locale` | string |

The caller supplies all four as one immutable context. `today` and `now` are never bare identifiers and never read from an ambient clock. The timezone is an explicit IANA identifier and the instant retains its explicit offset; locale cannot alter comparison, ordering, parsing, or case behavior.

## Embedded Query surface

The sole canonical embedded surface is:

```adoc
.Due soon
[.weftext-query,version=1,view=task-list]
....
from tasks as task
scope subtree(this.node)
where task.closed = false
  and task.due is not null
  and task.due <= context.today + P14D
select task.id, task.title, task.state, task.due
order by task.due asc nulls last
limit 100
....
```

The optional AsciiDoc block title supplies `this.query.title`. The block style/role is exactly `.weftext-query`; `version=1` is required. `view` is optional and accepts `table`, `list`, `task-list`, `board`, `calendar`, `timeline`, or `gallery`. It is an initial presentation hint only. `source` is forbidden as a block attribute because source semantics belong exclusively to `from`. `task-list` requires the `tasks` domain.

A generic AsciiDoc processor retains an ordinary role-bearing literal block and exact body. It does not execute the query. Core recognizes the block only outside protected ranges and separates parsing from evaluation.

## Query grammar

The body is UTF-8 with ASCII lowercase keywords, JSON-compatible strings, and `#` line comments. Clauses occur at most once and only in this order:

```text
from        required
scope       required
where       required
group by    optional
select      required
order by    required
limit       required
```

V1 domains are `nodes`, `tasks`, `headings`, and `templates`. Every source declaration is `from <domain> as <alias>`. The alias is required, uses ASCII snake-case, and may be chosen by the author, but cannot shadow `this`, `context`, a domain name, or a keyword. `row.*` and bare field names are invalid.

`scope` is one of:

```text
scope workspace
scope descendants(<node-reference>)
scope subtree(<node-reference>)
scope section(<heading-reference>)
```

A node reference is a typed Node record such as `this.node`; a heading reference is a typed Heading record such as `this.heading`. There is no path/name/UUID scope and no implicit current node. `descendants` excludes the named node; `subtree` includes it. `section` selects rows lexically owned by the parser-owned heading section. Passing null `this.heading` returns exact `missing_heading_context`. The `templates` domain accepts only `workspace` in v1 because template rows already belong to the configured library.

`where` requires one bool expression; an unfiltered query writes `where true` explicitly. `group by` accepts one scalar expression and optional output alias. `select` accepts one to 32 comma-separated expressions. A direct member path infers its final member name; `as <output-name>` is optional. A computed expression requires `as`, and all resulting ASCII snake-case output names must be unique. `order by` requires one to eight comma-separated expressions followed by `asc|desc` and optional `nulls first|last`. `limit` is an integer `1..1000`. Stable final tie-breakers are domain path, row kind where applicable, parser source start where applicable, and UUID/action evidence.

Authorization filters candidate rows and resolves scope before any expression, hidden-dependent diagnostic, grouping, count, ordering, projection, limit, suggestion, export, or cache entry. Hidden and missing explicit targets produce one non-disclosing unavailable result.

## Closed row records

`from nodes as node` exposes:

| Member | Type and meaning |
| --- | --- |
| `node.id` | UUID |
| `node.name` | string current basename |
| `node.path` | string current workspace-relative locator |
| `node.depth` | number |
| `node.parent_id` | UUID or null |
| `node.display_title` | derived string: authored Document Title, otherwise current name |
| `node.document` | closed Document record |

The Document record exposes `title: string|null`, `subtitle: string|null`, `display_title: string`, and `properties`. Authored `title` and `subtitle` remain nullable and are never replaced by the derived fallback. `display_title` is explicitly derived. `properties["名称"]` returns the bounded literal document-header attribute string or null; body redefinitions and processor expansion are excluded.

The `tasks` domain is the tagged checklist/task-node union. Its alias exposes `kind`, nullable `id`, `owner_node`, non-null `title`, `closed`, `state`, nullable `checklist_depth`, nullable `priority`, nullable `created`, `start`, `scheduled`, `due`, and `closed_at`, plus nullable permission-filtered `blocked`. `title` is the checklist principal text or task-node authored/derived display title. `owner_node` is the same closed Node record shape. Checklist-only fields remain null for task nodes and task-node-only fields remain null for checklists; no value is fabricated.

The `headings` domain returns one row per parser-owned body Heading, never the Document Title. Its alias exposes `title: string`, `level: number`, `anchor: string|null`, `parent: HeadingRef|null`, `path: list<string>`, `owning_node: Node`, and `document: Document`. H1 through H9 correspond to source `==` through `==========`. A row's `parent` is null for H1 and otherwise the nearest containing lower-level heading. Authorization is inherited from `owning_node` before the row exists.

The `templates` domain returns one row per Template Root, never one per Part. Its alias exposes `id`, `name`, `path`, `display_title`, `part_count`, and `parameter_count`. Template Library root and Template Parts are absent from every ordinary semantic domain, including `nodes`, `tasks`, `headings`, citations, and default backlink projections.

`templates` is a recognized stable v1 domain name. When Template role inventory is unavailable, evaluation returns exact `domain_unavailable`; it does not return an empty successful result, fall back to `nodes`, or inspect paths/sidecars opportunistically. When the inventory is available, ordinary authorization and non-disclosure run before any Template Root row or count exists.

## Lexical `this`

`this` is resolved only from the authored block location:

- `this.node`: owning Node record;
- `this.document`: owning Document record;
- `this.heading`: nearest parser-owned containing body Heading record or null, distinct from any `headings` row alias; and
- `this.query`: record with nullable `title` from the block title.

There is no `this.title` or `this.subtitle`. Use `this.document.title`, `this.document.subtitle`, or the explicit derived `this.document.display_title`.

The Heading record has `title: string`, `level: number`, `anchor: string|null`, `parent: HeadingRef|null`, and `path: list<string>`. `HeadingRef` contains the same non-recursive title/level/anchor/path fields. The native `= Title` is Document Title. `==` through `==========` are body H1 through H9. A title-only document and preamble resolve `this.heading` to null. H1 `parent` is null; a contained H2 reports its H1 parent. Accessing a heading member when `this.heading` is null produces exact `missing_heading_context`, never a document-title fallback.

Focus, selection, active tab, Explorer row, URL, current request node, and display path never affect `this`. A Saved Query that references `this` must persist or receive an explicit immutable embedding binding. If it is executed without one, evaluation returns `missing_context`. Saved-query storage is deferred; this execution rule is not.

## Bounds and diagnostics

Query body source above 16,384 UTF-8 bytes, more than 2,048 tokens, 256 expression nodes, nesting deeper than 32, 64 items in any list, 64 fields in any record, eight order keys, 32 selected fields, or limit above 1,000 is rejected before evaluation. The limit does not cap the containing managed AsciiDoc document. Each decoded string is at most 4,096 UTF-8 bytes. One evaluation has at most 65,536 expression steps. A result has at most 1,000 rows and at most 4 MiB of canonical serialized typed data, including groups and action evidence. Encoded and decoded sizes are checked before allocation amplification; exceeding any bound returns `resource_limit` and no partial plan or partial result. Evaluation cannot recurse.

Diagnostics carry exact source ranges and stable codes for syntax, duplicate/ordered clauses, unknown domain/alias/member/function, `domain_unavailable`, type mismatch, `null_comparison`, `numeric_overflow`, invalid literal, invalid scope, missing context, missing heading context, unavailable target, `resource_limit`, and prohibited effect. Parser failure never executes a partial plan.

## Migration and runtime boundary

The superseded `[query,source=...]` attribute surface, old body `scope` forms, body `sort`, bare `today`, bare fields, and any `row.*` spelling are accepted only by a private read-only one-time converter. The converter must emit the exact canonical block, prove equivalent authorized population, null behavior, ordering, projection, and time context, or block the whole migration. Product runtime, ordinary import output, documentation examples, and new writers never recognize both grammars.

## Acceptance

Acceptance covers explicit source aliasing; version/source/view separation; exact clause order; no bare fields or `row.*`; all types/operators/functions and prohibited effects; exact decimal overflow/no-rounding; day-only duration; ordinary null-comparison errors and fixed nullable membership; deterministic context; document-title-only and preamble heading null; H1/H2 ownership and H1 null parent; authored document title/subtitle versus derived display title; absence of `this.title`; nullable query block title; saved-query `missing_context`; typed scope references and hidden-target non-disclosure; pre-inventory `templates` domain unavailability; properties with Unicode keys; generic AsciiDoc literal-block degradation; malformed/protected/CJK/RTL/CRLF input; stable ordering; exact resource ceilings; and identical Core results across every delivered caller.
