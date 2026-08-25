---
source_language: zh-CN
translation_status: source
---

[English](19-expression-query-and-template-library.md)

# 表达式、Query 和模板库

`weftext.expr.v1` 是一种有范围、确定、纯粹、静态类型化的值语言。它没有文件系统、网络、环境、环境时钟、随机、动态加载或求值逃逸。Query 和模板绑定共享这一表达式基底，但不共享彼此的语法或权威。

## 规范 Query

规范 Query 是一个版本 1、带 `.weftext-query` 角色的字面块。其正文以显式 `from` 源和别名开始，具有类型化 `scope`，使用带别名前缀的字段，并具有有序的筛选/分组/投影/排序/限制子句。`view` 仅用于呈现。`this` 是词法、由解析器拥有的嵌入上下文，绝不是活动 UI 焦点或请求默认值；时间通过显式上下文提供。

仅 Core 解析、类型检查、授权、执行、排序和诊断 Query。源域是显式的：节点、标题、清单/任务并集和模板根。在行、字段、计数、建议、诊断、缓存或导出之前，结果会按权限过滤。使用词法 `this` 的已保存源必须携带不可变嵌入绑定，否则以 `missing_context` 失败。

## 模板库

工作区根可以配置一个模板库根 UUID。它的直接子项是模板根，后代是模板部件。这些角色由拓扑派生，保留规范节点存储，并从常规投影中排除。只有模板根拥有 `weftext.template.json`，其 Profile 为 `weftext.node-template.v1`，版本为 `1`。

模板参数和绑定使用冻结的 `input` 和显式 `context`；槽位是受约束的 `slot:name[]` 和 `slot::name[]` 构造，绝不执行 Query。实例化预览带新 UUID 的普通子树、完整内部链接映射、所属资源副本、批注处置和目标冲突，随后一次提交。角色转换必须显式，且不能在普通节点上留下 Profile/槽位/伴随文件权威。
