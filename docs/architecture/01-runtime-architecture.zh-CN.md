---
source_language: zh-CN
translation_status: source
---

[English](01-runtime-architecture.md)

# 运行时架构

文缕是一个主要以 Rust 实现的实验性代码库，由一个 Core 和多个客户端构成。唯一的受管文档格式是使用 `weftext` 元数据封装和 `weftext.annotations.json` 的文缕 AsciiDoc Profile。Markdown 仅限于导入、导出和可见的非受管内容。参见[规范元数据](14-canonical-document-metadata-and-review.zh-CN.md)。

## 一个产品，多个客户端

```text
桌面端本地模式 -> 文缕 Core <- Server
CLI              -> 文缕 Core <- WebUI / 远程桌面端 / CLI 管理
```

Core 是节点身份、Profile 选择、操作、计划、事务、验证和冲突语义的唯一权威。AsciiDoc Profile crate 提供精确源代码模型、诊断、受保护区域和语法编辑；它不拥有工作区变更权威。桌面端、WebUI、Server、CLI 和智能体都是调用方，不能另行创建解析、文件系统或授权规则。

## 组件边界

- `weftext-core` 拥有领域类型、计划、事务和后端无关规则。
- `weftext-asciidoc` 只拥有精确源代码 Profile 模型和语法操作。
- `weftext-cli`、`weftext-desktop`、`weftext-ui` 和 `weftext-server` 都是 Core 调用方。
- Server 拥有经身份验证的 API 访问、ACL、审计、协作协调及其非可移植控制平面。
- 智能体边界仅公开有范围的读取和经审阅的类型化操作；它绝不公开原始工作区写入或 shell。

## 本地和托管模式

本地 Desktop 和 CLI 对文件系统工作区调用 Core。可移植源文本、资源和伴随文件是权威；索引和 UI 状态可重建。

托管客户端调用经身份验证的 Server API，绝不直接写入托管工作区。Server 对操作者授权、检查基础修订、调用 Core、通过托管后端提交并发布结果。账户、ACL、会话、审计和在线状态是控制平面状态，不是文档前置元数据。文件夹同步适合单个用户的设备，但不是多用户协作；协作编辑通过文缕 Server 进行。

## 共享 UI 契约

桌面端和浏览器客户端共享文档/节点视图模型、操作标识和预览、编辑器与批注行为、本地化/无障碍行为以及协议一致性测试。仅桌面端具备的生命周期、文件夹选择器、操作系统、凭据和更新功能留在桌面端壳层。没有任何 UI 框架是持久格式权威。

## 智能体和事务边界

智能体会话仅接收所选节点上下文、文档读取、搜索、操作说明、预览、批准、结果、诊断、取消和事件。在托管模式下，智能体是经授权的 Server 客户端；它绝不获得对托管目录的直接访问。

每次变更都绑定操作者、来源、类型化目标、基础修订、确定性计划、验证和提交结果。结构编辑使用可恢复的工作区事务；文档编辑使用修订检查的文档事务。持久协作状态会物化为规范源代码/资源/批注，而在线状态保持短暂。
