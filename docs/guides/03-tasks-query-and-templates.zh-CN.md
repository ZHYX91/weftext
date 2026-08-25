---
source_language: zh-CN
translation_status: source
---

[English](03-tasks-query-and-templates.md)

# 待办、Query 与模板

## 两种任务形态

原生 AsciiDoc 待办项适合快速记录。它属于当前文档和源位置，没有独立 UUID，也没有持久优先级、依赖关系或任务级批注。

任务节点是普通受管节点，使用节点现有 UUID 作为任务身份。它可以拥有正文、附件、批注、状态、优先级、日期和依赖关系，不需要另一套任务数据库。

**已确定设计：**当轻量待办项需要持久能力时，用户可以将它显式提升为任务节点。Weftext 会预览新节点位置、身份、正文和原位置替换，并通过一次可恢复事务提交。原位置变成稳定的 `node:` 链接，不会同时保留一个容易失去同步的复选框副本。

## Query 是什么

Query 从当前工作区内容派生集合视图。它可以查询节点、任务、正文标题或模板角色，并进行筛选、排序、分组和字段选择。结果不是第二份数据库，也不会改变源记录的身份。

界面中的数据选项卡和动态视图构建器编辑文档里的规范 Query。保存筛选、排序或布局时，用户可以查看将要写入的 Query；界面不会另外保存一份隐藏配置。

Query 和模板表达式共享 `weftext.expr.v1`。两者使用相同的值、运算符、null 规则和安全限制，但 Query 子句与模板占位符保持不同，因为它们解决的是不同问题。

## `this` 上下文

`this.node` 表示当前节点，`this.document` 表示当前文档，`this.heading` 表示当前源位置所属的最近正文标题，`this.query` 表示当前 Query 块。

文档只有标题或当前位置处于前言时，`this.heading` 为 null。文档标题使用 `this.document.title`，副标题使用 `this.document.subtitle`，派生显示标题使用 `this.document.display_title`。没有 `this.title` 或 `this.subtitle` 捷径。

## 模板库

**已确定设计：**模板库是用户明确配置的特殊受管子树。库中的 Template Root 和 Template Part 仍使用普通节点存储，但拥有受验证的模板角色和伴随文件。

模板设计器通过表单帮助用户声明参数、默认值、选择项、节点名称和内容 slot，不要求记忆占位符拼写。实例化会先显示完整子树预览，再通过一次事务创建全新节点 UUID、重写内部链接并复制附件。

模板记录不会混入普通搜索、任务、关系图或时间线结果。普通节点移入或移出模板库时，需要显式角色转换；只移动文件夹不足以改变角色。

## 当前状态

**当前基础：**Core 已有任务解析、部分类型化任务与 Query 计划能力，以及共享调用边界。

**发布前限制：**完整任务提升、完整表达式求值器、可视化 Query 构建器、模板设计器和模板实例化仍在实现与验收中。

详细契约见[任务与 Query](../specifications/17-tasks-and-query.zh-CN.md)、[Query 与表达式](../specifications/18-canonical-query-and-expression.zh-CN.md)和[模板库](../specifications/19-node-template-library.zh-CN.md)。
