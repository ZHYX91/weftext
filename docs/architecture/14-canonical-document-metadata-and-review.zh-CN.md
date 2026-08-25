---
source_language: zh-CN
translation_status: source
---

[English](14-canonical-document-metadata-and-review.md)

# 规范文档、元数据和审阅权威

文缕只有一种受管节点形状，不存在受管 Markdown 生成：

```text
X/
├── X.adoc
├── weftext.annotations.json   # 仅在需要时创建
└── resources...
```

根 `.weftext-format` 必须严格为 `weftext.asciidoc.v1\n`。缺失、未知或格式不正确的标记会安全拒绝，或进入显式导入/接纳流程；扩展名不能选择生成。Markdown 仅是导入/导出、可见非受管内容或附件边界。

## 保留名称和元数据封装

根控制文件使用 `.weftext-*`；封闭存储子项可使用 `_weftext.*`；可移植节点本地伴随文件使用 `weftext.*`；系统元数据封装是唯一的顶层 YAML `weftext` 映射。新名称必须有已接受的存储/Profile 契约。封闭回收站存储由[架构 17](17-workspace-trash-item-store.zh-CN.md)定义。

每个受管文档都有唯一的顶层 `weftext` 映射。必需的 `id` 是小写 UUIDv4。可选 `icon`、有序 `aliases`、子项排序字段、稀疏 `sibling_rank`、根 `adjacent_heading_body` 和根 `template_library_root` 具有封闭语义。路径、父项、名称、角色、时间戳、反向链接、搜索、任务计数、缩略图和视图状态均为派生数据或设备/控制平面状态。元数据封装编辑是狭窄的、修订检查的 YAML 补丁；禁止整体重序列化前置元数据。

## AsciiDoc 文档头和类型化构造

文档头拥有标题、副标题、作者、修订、语言、描述性元数据、处理器配置和简单属性。只有有范围的字面文档头属性进入属性投影；正文重定义是处理器状态。属性不展开环境、路径、URI 或处理器。

复杂数据使用版本化 Profile 构造：原生待办项和 `weftext-task` 任务节点文档头；带 `weftext.expr.v1` 的规范 Query 块；角色受限的模板根 `weftext.template.json`；以及引用构造。它们不会创建第二套身份系统、manifest、任务数据库或通用元数据存储。

## 可移植审阅伴随文件

`weftext.annotations.json` 是非作者性高亮、评论、线程、建议和审阅标记的权威。作者性删除线仍是源文本；接受建议会调用 Core 文档事务。批注目标绑定基础修订、UTF-8 范围、引用、上下文和结构证据。Core 仅在唯一确定性匹配时重新锚定；缺失或歧义目标保持未解决。Server 串行化并授权托管伴随文件变更；凭据、草稿、在线状态和提供方状态绝不进入其中。
