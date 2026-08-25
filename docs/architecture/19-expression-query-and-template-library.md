---
source_language: zh-CN
translation_of: 19-expression-query-and-template-library.zh-CN.md
translation_status: synced
---

[简体中文](19-expression-query-and-template-library.zh-CN.md)

# Expression, Query, and Template Library

`weftext.expr.v1` is a bounded, deterministic, pure, statically typed value language. It has no filesystem, network, environment, ambient clock, randomness, dynamic loading, or evaluation escape. Query and Template bindings share this expression substrate but not each other's grammar or authority.

## Canonical Query

A canonical Query is a version-1 `.weftext-query` role-bearing literal block. Its body starts with explicit `from` source and alias, has a typed `scope`, uses alias-qualified fields, and has ordered filter/group/projection/order/limit clauses. `view` is presentation only. `this` is lexical parser-owned embedding context, never active UI focus or a request default; time is supplied through explicit context.

Core alone parses, type-checks, authorizes, executes, orders, and diagnoses Query. Source domains are explicit: nodes, headings, checklist/task union, and Template Roots. Results are permission filtered before rows, fields, counts, suggestions, diagnostics, cache, or export. Saved source with lexical `this` must carry immutable embedding binding or fails with `missing_context`.

## Template Library

The workspace root may configure one Template Library root UUID. Its direct children are Template Roots and descendants are Template Parts. Those roles are derived from topology, retain canonical node storage, and are excluded from ordinary projections. Only a Template Root owns `weftext.template.json` with profile `weftext.node-template.v1` and version `1`.

Template parameters and bindings use frozen `input` and explicit `context`; slots are constrained `slot:name[]` and `slot::name[]` constructs and never execute Query. Instantiation previews a fresh-UUID ordinary subtree, complete internal-link mapping, owned-resource copies, annotation disposition, and destination conflicts, then commits once. Role conversion is explicit and cannot leave profile/slot/sidecar authority on an ordinary node.
