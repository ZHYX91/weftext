---
source_language: zh-CN
translation_status: source
---

[English](18-task-nodes-and-checklist-promotion.md)

# 任务节点和清单提升

Weftext 有两种规范任务形式：用于轻量、无身份操作的原生 AsciiDoc 待办项出现位置，以及携带封闭 `weftext-task` v1 文档头 Profile 的普通受管节点，用于持久任务。不存在内联任务 manifest、任务伴随文件、任务数据库或第二个任务 UUID 命名空间。

## 原生待办项边界

```adoc
* [ ] 打开
* [x] 已关闭
* [*] 同样已关闭
```

`[ ]` 是未完成；`[x]` 和 `[*]` 是已完成。清单出现项只由拥有者节点 UUID、文档修订、精确源范围和解析器确认的列表出现项标识。它在渲染、搜索、Query 或切换时不会获得 ID。嵌套结构不会创建依赖、重复、拥有关系或类型化字段。

## 持久任务 Profile

持久任务是普通受管节点，并使用其既有 `weftext.id` UUID。文档头 Profile 具有封闭的字面字段：

| 属性 | 规则 |
| --- | --- |
| `weftext-task` | 必需，精确为 `v1` |
| `weftext-task-state` | 必需：`todo`、`in-progress`、`on-hold`、`completed` 或 `cancelled` |
| `weftext-task-priority` | 可选：`highest`、`high`、`medium`、`normal`、`low` 或 `lowest` |
| `weftext-task-created`、`-start`、`-scheduled`、`-due`、`-closed` | 可选 ISO 日期或带显式偏移的 RFC 3339 瞬间；`closed` 仅适用于关闭状态 |
| `weftext-task-depends-on` | 可选、唯一的以空格分隔的任务节点 UUID；不得自环或成环 |

仅日期的值仍是日期，瞬间绝不推断时区。`blocked` 和 `overdue` 是派生的。未知、重复、无效、歧义或循环字段会诊断该 Profile，但不会隐藏底层节点。根、回收站、非受管和被忽略内容不能携带此 Profile。

## 提升和投影

提升是一个 Core 计划，冻结源出现项/修订、目标 UUID/父项/名称/标题、任务属性、精确源替换、受影响批注、草稿证据和日志步骤。它在一次可恢复事务中创建任务节点，并将原清单分支替换为包含 `node:<uuid>[<label>]` 的普通列表项。它绝不留下同步复选框镜像、悄然附加后缀、丢弃歧义续接内容，或在焦点变化后重定向。

`tasks` Query 域是一个带标签的并集。清单行只公开真实出现项证据和派生的未完成/已完成状态。节点行公开任务节点 UUID 和类型化 Profile 字段。切换、提升、排期、依赖、批量编辑和看板移动都选择源特定、修订绑定的 Core 操作。授权发生在依赖解析、行、计数、分组和诊断之前。
