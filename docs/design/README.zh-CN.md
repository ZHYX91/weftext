---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Typst 新架构设计

状态：**design-refinement-not-implemented**。当前修订保留主体架构，补齐需求边界、CEL/复制合同并精简内建配置；固定旧基线的独立接受与当前实际检查分见 [REVIEW](REVIEW.zh-CN.md) §7、§8。

2026-10-20 增补 REQUIREMENTS §5.1.1 的多级标签：CORE §2.3 负责森林及身份／升级，FEATURES §§5.6、7.4 负责可信查询与交互，SOURCE／SERVER／INTEGRATION 同步衔接。该目标要求必要的共用实现重构，尚不属于 alpha.3 已验范围；完成门统一在 ROADMAP TAG01–10。

本目录整体替代前轮 `typst-native` 的技术方案作为当前已接受的技术设计；前轮文件与其固定范围评审留作历史证据，不与本目录的算法拼接使用。历史 49 份输入、`inputs.json` 和完整评审已归档，不是本仓库的构建或运行依赖。这里保留用户要完成的事情，重新选择技术，不为未部署的协议族设计正常运行兼容层。

| 文档 | 负责的问题 |
| --- | --- |
| [SOURCE](SOURCE.zh-CN.md) | 原生作者源、身份/类型声明、一套类型与值、schema、各类结构化数据及安全回写 |
| [CORE](CORE.zh-CN.md) | 文件/结构权威、身份与版本、保存/恢复、外部编辑、同步、授权和受控求值 |
| [EDITOR](EDITOR.zh-CN.md) | 一份草稿、流式阅读、富编辑、来源映射、标题/副标题/混排与 Word 语义 |
| [FEATURES](FEATURES.zh-CN.md) | 链接、Query/导航/视图、任务、批注、模板、转换、下载与可恢复外部任务 |
| [INTEGRATION](INTEGRATION.zh-CN.md) | 各层合同、需求与原问题的落实、实施顺序和重构准入条件 |
| [REVIEW](REVIEW.zh-CN.md) | 实际实验、独立意见、未解决问题和完成边界 |

源码是标准 Typst，应用业务含义由 schema 与明确角色约定承载。常用界面保持简洁，必要的嵌套、多值、关系和历史信息不因旧 AsciiDoc 属性限制而删除。能编译一段数据不等于已经具备完整表单、Query、权限或导出；这些消费者须一起验证。

本轮补齐文档与有界原型，不改真实资料。旧基线独立接受与当前修订分开记录，后续以纵向功能切片重构；每个切片按自身风险完成实现验收。设计审查、有限原型、产品验收和发布是不同状态。
