---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# CEL 组合语义实测

2026-10-13，真实 `cel.dev/cel-go v0.32.0` 执行 [main.go](main.go) 的 15 个案例，全部通过，见 [results.json](results.json)。版本固定于 [go.mod](go.mod) 和 [go.sum](go.sum)。本实验只核 CEL 执行器，不是 Weftext 的 Rust Query、Cell adapter 或授权实现。

结果覆盖 `&&/||` 两侧换序的错误吸收、`exists/all` 换序、`filter/map` 错误、条件分支与缺字段 guard，以及预算耗尽不能被外层 `true` 吸收。按照 [CEL 语言规范](https://github.com/cel-expr/cel-spec/blob/master/doc/langdef.md#logical-operators)，不能将 `&&` 当作保证左先右后的程序 guard；必要时使用条件表达式。FEATURES §5.4 区分可吸收的业务 EvaluationIssue 与不可吸收的宿主授权/覆盖/取消/资源中止。

在本目录运行：

```text
go run .
```

首次运行可能下载锁定依赖。结果输出至 stdout；比较结果前不要覆盖已记录证据。普通文档检查不下载或执行 Go。完整 Weftext adapter、静态 profile 和平台预算仍须实现后独立测试。
