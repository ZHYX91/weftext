---
source_language: zh-CN
translation_status: source
---
[English](01-product-boundary.md)

# Weftext 产品边界

状态：已接受的产品边界。

Weftext 只有一种规范的受管文档格式：由必需的工作区标记选择、位于 `X/X.adoc` 的 Weftext AsciiDoc Profile。Desktop 也可以通过独立单文件边界编辑一个明确打开的外部 `.adoc` 或 `.asciidoc` 文件；该文件不是工作区或节点，不获得 Weftext 身份或工作区语义。Markdown 只能作为明确的导入/导出格式、明确分类的可见非受管内容或普通节点拥有的附件；它不是第二种受管格式，也不是独立编辑器模式。仓库工程文档可以独立使用 Markdown。参见 [`15-weftext-asciidoc-profile.md`](15-weftext-asciidoc-profile.zh-CN.md)、[`06-application-ui.md`](06-application-ui.zh-CN.md) 和 [`../architecture/14-canonical-document-metadata-and-review.md`](../architecture/14-canonical-document-metadata-and-review.zh-CN.md)。

Weftext / 文缕是一款知识工作区产品。它负责工作区存储、结构化源文编辑与阅读、节点操作、链接、排序、Chrono 笔记、批注、派生索引、可安全同步的恢复、本地 Desktop 应用、内网 Server、浏览器 WebUI、身份与授权、协作、备份/恢复，以及在这些能力之上受监督的 AI agent 操作。

工作区存储包含三类由 Core 管理的内容：带 UUID 身份的受管 `X/X.adoc` 节点；没有节点身份的可见非受管目录/文件/资源；以及从产品发现中排除的忽略内容。可选的根 `.weftext-rules` 文件是后两类内容的可移植工作区权威。Shell 和 Server 传输层必须使用 Core 的分类结果，不得依据文件名形状、文档字节、`.gitignore` 或前端扫描推断节点身份。

生产级文档转换、OCR、数学/图表渲染和完整结构化集合视图都在当前产品边界之外。导入使用 Weftext 自有的格式中立边界；外部转换器 schema 不得进入 Core 或工作区。外部转换器调用、线协议、兼容层和联合发布回执也在该边界之外。参见 [`../architecture/15-content-intake-foundation.md`](../architecture/15-content-intake-foundation.zh-CN.md)、[`../architecture/16-pdf-import-and-ocr.md`](../architecture/16-pdf-import-and-ocr.zh-CN.md) 和 [`../architecture/07-collections-query-and-views.md`](../architecture/07-collections-query-and-views.zh-CN.md)。

## AI agent 边界

AI agent harness 通过版本化、与 harness 无关的 Weftext 操作和上下文边界接入。DeepSeek Harness (DSH) 是首个一级支持的 harness。一级支持包括 Weftext 维护的适配器、明确的兼容性矩阵、能力协商、本地和 Server 集成路径、流式会话状态、取消、审批与审计行为以及符合性测试；这不表示 DSH 是唯一支持的 harness。

Weftext Core 始终与模型和 harness 无关。DSH runtime 文件、plugin、会话日志、模型配置和密钥属于控制平面或设备状态，不是工作区权威。DSH 不是 Core 的 Cargo 依赖，其预览协议不得泄漏到持久化 AsciiDoc 或伴随文件格式中。

Agent 绝不会获得第二条变更路径。第一方集成不会向 harness 授予对 Weftext 工作区的原始写权限；创建、编辑、重命名、移动、复制、Trash、恢复、批注和链接变更，都必须像 UI 与 CLI 一样使用带修订版检查的 Core 计划或事务。在 Server 模式下，agent 是委托客户端，其有效能力不得超过已认证人类操作者的能力。

## 架构

```text
Desktop local mode ───────> Weftext Core ───────> local workspace backend
                                  ^
                                  |  (including approved agent actions)
WebUI / Desktop / CLI ─────> Weftext Server ────> hosted workspace backend
```

所有变更都必须表达为 Core 计划或事务。Shell 不得实现独立的文件系统语义。托管模式下，客户端绝不直接写入 Server 工作区，而是使用经过认证和授权的 Server API。同步文件夹支持文件复制，但不是多用户协作机制。

非受管和忽略内容不参与节点链接、反向链接、图、Chrono、节点排序或节点事务。忽略内容仍是用于备份的物理工作区数据；忽略操作绝不会删除或重写它。

公共产品状态与方向汇总于 [`../../ROADMAP.md`](../../ROADMAP.zh-CN.md)。运行时边界定义于 [`../architecture/01-runtime-architecture.md`](../architecture/01-runtime-architecture.zh-CN.md)。
