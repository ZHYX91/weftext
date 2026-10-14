---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Actual CEL composition probe

On 2026-10-13, real `cel.dev/cel-go v0.32.0` passed all 15 cases in [main.go](main.go); see [results.json](results.json). [go.mod](go.mod) and [go.sum](go.sum) pin dependencies. This tests the CEL runtime, not Weftext's Rust Query, Cell adapters or authorization implementation.

Cases cover error absorption with both operand orders for `&&/||`, reordered `exists/all`, `filter/map` errors, conditional branches, missing-field guards and a cost abort that an outer `true` cannot absorb. The [CEL language specification](https://github.com/cel-expr/cel-spec/blob/master/doc/langdef.md#logical-operators) does not make `&&` a guaranteed left-before-right program guard; use conditional expressions when needed. FEATURES §5.4 separates absorbable business EvaluationIssue from non-absorbable host authorization, coverage, cancellation and resource aborts.

Run from this directory:

```text
go run .
```

The first run may download pinned dependencies. Results go to stdout; compare before replacing recorded evidence. Normal documentation checks neither download nor run Go. Actual Weftext adapters, static profile and platform budgets still require implementation and independent tests.
