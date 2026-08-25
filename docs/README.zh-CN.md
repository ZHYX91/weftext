---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext 文档权威

本目录包含当前公开产品契约和架构决策。仓库文档使用 Markdown，与 Weftext 受管文档采用 AsciiDoc 无关。

公开文档以通用方式描述 Weftext 行为和外部格式类别。产品比较、截图、研究、决策历史、排期、任务交接和验收日志保存在本仓库之外的私有控制工作区。

双语源文档/译文布局和公开内容规则见[文档策略](DOCUMENTATION.zh-CN.md)。统一译法和写作约定见[术语表](TERMINOLOGY.zh-CN.md)。

## 阅读路线

- 如果你想了解 Weftext 怎样保存和组织内容，请从[用户指南](guides/README.zh-CN.md)开始。
- 如果你正在实现或审核功能，请使用下方权威映射进入规范和架构文档。
- 如果你只想了解目前已经具备什么、还有哪些发布缺口，请阅读[公开路线图](../ROADMAP.zh-CN.md)。

## 权威映射

| 关注点 | 当前权威 |
| --- | --- |
| 受管节点、身份、元数据封装、属性、资源和批注 | 架构 14；规范 02、04、15 |
| 导航、编辑器、功能区、上下文选项卡、检查器、格式刷、表格、图片和独立 AsciiDoc 模式 | 架构 05、06、20；规范 06、09、12 |
| 操作、目标作用域、草稿门控、回收站、恢复和结构事务 | 架构 17、22；规范 02、08、12、13 |
| 原生待办项、任务节点、提升、任务视图和任务看板 | 架构 18、21；规范 17、21 |
| Query、表达式、节点/任务/标题集合、筛选、排序、分组和聚合 | 架构 07、19；规范 18 |
| 模板库、占位符、设计器、角色转换和子树实例化 | 架构 19；规范 19 |
| 多维表格记录、字段类型、视图、关系和原生表格升级 | 架构 20；规范 20 |
| 通用看板和数据源特定操作 | 架构 21；规范 21 |
| 设备本地历史、无需 Git 的比较/合并、备份目标/仓库和恢复 | 架构 22；规范 08、22 |
| 链接、Chrono、同步、引文、导入、AI、服务器、测试和发布 | 相应编号的规范与架构文档 |

## 架构决策

- [`architecture/01-runtime-architecture.zh-CN.md`](architecture/01-runtime-architecture.zh-CN.md)：Desktop、WebUI、Server、CLI、Core 和后端边界。
- [`architecture/05-shared-navigation-information-architecture.zh-CN.md`](architecture/05-shared-navigation-information-architecture.zh-CN.md)：资源管理器“层级/内容”导航和呈现状态。
- [`architecture/06-content-io-and-rich-rendering.zh-CN.md`](architecture/06-content-io-and-rich-rendering.zh-CN.md)：保留源文本的导入/导出、富内容渲染、OCR 和增强边界。
- [`architecture/07-collections-query-and-views.zh-CN.md`](architecture/07-collections-query-and-views.zh-CN.md)：Query 派生集合和动态视图。
- [`architecture/14-canonical-document-metadata-and-review.zh-CN.md`](architecture/14-canonical-document-metadata-and-review.zh-CN.md)：规范源文本、身份元数据封装、属性、类型化数据和审阅状态。
- [`architecture/15-content-intake-foundation.zh-CN.md`](architecture/15-content-intake-foundation.zh-CN.md)：统一导入探测、工作进程、IR、预览、事务和回执边界。
- [`architecture/16-pdf-import-and-ocr.zh-CN.md`](architecture/16-pdf-import-and-ocr.zh-CN.md)：选定的 PDF/OCR 包和证明门槛。
- [`architecture/17-workspace-trash-item-store.zh-CN.md`](architecture/17-workspace-trash-item-store.zh-CN.md)：基于条目的工作区回收站、恢复、同步和备份。
- [`architecture/18-task-nodes-and-checklist-promotion.zh-CN.md`](architecture/18-task-nodes-and-checklist-promotion.zh-CN.md)：原生待办项/任务节点模型和提升事务。
- [`architecture/19-expression-query-and-template-library.zh-CN.md`](architecture/19-expression-query-and-template-library.zh-CN.md)：共享表达式语言、规范 Query 和模板库。
- [`architecture/20-multidimensional-tables-and-editor-surfaces.zh-CN.md`](architecture/20-multidimensional-tables-and-editor-surfaces.zh-CN.md)：多维表格和编辑器信息架构。
- [`architecture/21-board-views.zh-CN.md`](architecture/21-board-views.zh-CN.md)：通用看板、任务看板预设、类型化操作和可访问性。
- [`architecture/22-document-history-comparison-and-backup-repositories.zh-CN.md`](architecture/22-document-history-comparison-and-backup-repositories.zh-CN.md)：历史、比较、备份仓库和恢复。
- [`architecture/dependencies.zh-CN.md`](architecture/dependencies.zh-CN.md)：依赖策略。

## 规范

规范按契约领域编号；即使某一文档被删除或合并，编号仍保持稳定。

1. [`01-product-boundary.zh-CN.md`](specifications/01-product-boundary.zh-CN.md)
2. [`02-node-storage.zh-CN.md`](specifications/02-node-storage.zh-CN.md)
3. [`03-chrono.zh-CN.md`](specifications/03-chrono.zh-CN.md)
4. [`04-annotations.zh-CN.md`](specifications/04-annotations.zh-CN.md)
5. [`05-sync-and-index.zh-CN.md`](specifications/05-sync-and-index.zh-CN.md)
6. [`06-application-ui.zh-CN.md`](specifications/06-application-ui.zh-CN.md)
7. [`07-server-collaboration.zh-CN.md`](specifications/07-server-collaboration.zh-CN.md)
8. [`08-data-safety-backup.zh-CN.md`](specifications/08-data-safety-backup.zh-CN.md)
9. [`09-testing-release.zh-CN.md`](specifications/09-testing-release.zh-CN.md)
10. [`10-links-and-potential-mentions.zh-CN.md`](specifications/10-links-and-potential-mentions.zh-CN.md)
11. [`11-ai-agent-integration.zh-CN.md`](specifications/11-ai-agent-integration.zh-CN.md)
12. [`12-document-actions.zh-CN.md`](specifications/12-document-actions.zh-CN.md)
13. [`13-workspace-transactions.zh-CN.md`](specifications/13-workspace-transactions.zh-CN.md)
15. [`15-weftext-asciidoc-profile.zh-CN.md`](specifications/15-weftext-asciidoc-profile.zh-CN.md)
16. [`16-citations-and-bibliography.zh-CN.md`](specifications/16-citations-and-bibliography.zh-CN.md)
17. [`17-tasks-and-query.zh-CN.md`](specifications/17-tasks-and-query.zh-CN.md)
18. [`18-canonical-query-and-expression.zh-CN.md`](specifications/18-canonical-query-and-expression.zh-CN.md)
19. [`19-node-template-library.zh-CN.md`](specifications/19-node-template-library.zh-CN.md)
20. [`20-multidimensional-tables-and-editor-surfaces.zh-CN.md`](specifications/20-multidimensional-tables-and-editor-surfaces.zh-CN.md)
21. [`21-board-views.zh-CN.md`](specifications/21-board-views.zh-CN.md)
22. [`22-document-history-comparison-and-backup-repositories.zh-CN.md`](specifications/22-document-history-comparison-and-backup-repositories.zh-CN.md)

根目录 [`ROADMAP.zh-CN.md`](../ROADMAP.zh-CN.md) 是简洁的公开实现状态视图。较新的规范性规范或架构决策是目标行为权威；代码和夹具只证明已实现子集，不构成第二套公开契约。
