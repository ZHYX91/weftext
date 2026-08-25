---
source_language: zh-CN
translation_status: source
---

[English](11-ai-agent-integration.md)

# AI 智能体集成

本规范定义与运行器无关的智能体契约、主机桥接以及范围受限的只读 MCP 工具。变更工具、UI、打包和 Server 委托只有在保持下述授权、审批和事务边界时才可公开。

## 支持级别

Weftext 可集成多种 AI 智能体运行器，但支持声明必须明确：

- 第一层：Weftext 维护适配器、兼容性矩阵、UI 路径、文档、安全审查、符合性套件、打包验收和受支持版本诊断。
- 兼容：适配器实现公开契约并通过核心符合性测试，但 Weftext 不承诺打包或跨版本验收。
- 实验性：不保证兼容性或发布。

DeepSeek Harness（DSH）是第一个指定为第一层的运行器。这一优先级不使 Weftext 成为 DSH 前端，不要求使用单一模型提供方，也不妨碍后来加入第一层的运行器。

## 架构边界

稳定的 Weftext 侧与运行器无关，由以下部分构成：

- 范围受限的上下文和文档/搜索读取；
- 结构化工具/动作描述及能力要求；
- 确定性预览、基础修订、审批、提交结果和结构化错误；
- 可用时的流式会话事件、状态、取消和重连/恢复语义；
- 参与者、委托客户端、适配器和审计归因。

Core 不依赖 DSH、Node.js、模型 SDK、提示格式或智能体转录模式。`weftext` 元数据封装或批注伴随文件中不添加任何运行器专有字段。重建或解释工作区不需要智能体会话。

## 导入增强补丁

导入器只有在完成确定性的本地提取后，才可请求可选智能体增强。智能体接收用户批准的页面/区域证据和有界 Weftext Import IR 片段，并针对明确目标 ID 及一个基础 IR 修订返回类型化补丁。受支持操作可修正 OCR、阅读顺序、标题分类、表格结构、公式或图形描述。整篇文档源文本替换、任意 AsciiDoc 重写、直接工作区文件、过期目标及越界补丁均被拒绝。本地验证器将接受的补丁应用至 IR、重新生成精确提案，并使用通常预览和 Core 事务。提供方、外部出口、成本、保留、脱敏、置信度和智能体来源仍在导入回执中可见。

## DSH 适配器

第一方 DSH 集成由两个协调部分组成：

1. 主机桥接使 Desktop 或 Server 能够启动或连接经过测试的 DSH 运行时，将生命周期/会话事件映射到共享智能体 UI，请求取消，并呈现失败或恢复状态。
2. Weftext 工具/plugin 包为 DSH 提供范围受限的读取和结构化 Weftext 动作，绝不实现直接文件系统变更。

预览主机传输使用以换行分隔的 JSON-RPC 服务器，因为它公开持久的 `session.event` 数据和整个智能体的 `session.status`。Wire `0.0.1` 没有逐提示结果、提示取消、会话关闭、活动审批请求或真正的协议协商。桥接在提示前强制一次成功初始化，验证服务名称和受支持版本，并将取消如实表示为整个运行时终止。打包可修改启动机制，但不得改变工作区持久化或 Core 动作语义。

DSH 目前自称开发者预览，并警告 API 可能发生不兼容变更。因此每个声称支持 DSH 的 Weftext 版本都公布经过测试的 DSH 版本和适配器版本。启动执行版本/能力握手；不受支持的组合安全拒绝并说明受支持路径。

官方上游参考：[产品概览](https://deepseek.com/harness/en/)、[仓库](https://github.com/deepseek-ai/deepseek-harness)、[架构](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md)、[SDK JSON-RPC server](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/sdk/server/README.md) 和 [ACP 范围](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/acp/acp/README.md)。

## 上下文与数据处理

会话以明确的工作区、子树或节点范围以及能力授予开始。Weftext 仅提供所请求工具需要的最少上下文，并在披露前过滤不可访问数据。搜索、反向链接、别名、诊断、计数、最近项目和错误均遵守相同范围。

第一方集成不为 DSH 提供可写 Weftext 工作区挂载。本地读取应使用 Weftext 上下文工具或有意限定的只读表示。托管模式中，适配器使用已认证 Server API，绝不看到托管文件系统路径。

模型凭据、DSH 配置、插件配置、会话转录、检查点、缓存和审批日志属于设备或 Server 控制平面状态。它们默认不写入可移植工作区内容，且除非明确备份策略另有规定，不纳入可移植工作区备份。导出选定智能体输出通过 Core 事务创建或编辑普通节点，并记录可见来源，而不嵌入密钥。

## 动作与审批

只读工具只能在获授上下文策略内自动运行。变更请求包含人类参与者、委托客户端和智能体来源、目标、能力、基础修订、提议计划、受影响节点/资源、外部出口影响以及确认策略。

面向用户的状态区分：

1. 生成文本或提议；
2. 等待审批的预览；
3. 已批准、等待提交的动作；
4. 已提交的 Core 事务或结构化失败。

批量、破坏性、跨工作区、权限、密钥访问和外部出口操作需要明确策略与预览。取消停止未来工作，但不假装撤销已经提交的事务。撤销或恢复使用通常的 Core 语义。

Server 模式的有效权限为 `human actor ∩ delegated session capability ∩ workspace policy`。撤销在下次工具调用或提交前生效。智能体错误不得披露不可访问节点是否存在。

## 本地与 Server 路径

本地 Desktop 集成负责适配器生命周期、安全凭据引用、选定上下文、审批、事件展示、取消和诊断。本地模式即使不要求 Server 账户，也会记录来源和会话能力。

Server 集成可以使用 Server 管理的适配器，或接纳已授权的远程适配器；二者都是委托客户端。Server 管理的执行还要求明确沙箱、资源配额、网络出口、密钥、保留、升级和运营者可观测性策略。实时协作不允许智能体绕过修订检查或结构事务串行化。

## 第一层验收

DSH 第一层支持只有在以下全部获得证据时才算完成：

- 已发布 Weftext/DSH 兼容性矩阵和安全拒绝的版本诊断；
- 维护第一方桥接和 Weftext DSH 工具/plugin 包；
- 真实会话事件/状态流、取消和记录在案的重连/恢复行为；
- 基于真实 Core 动作的范围受限读取/搜索和提议/预览/审批/提交流程；
- 第一方集成没有原始可写工作区访问；
- 本地打包 Desktop 测试，以及声称支持 Server 时的角色/ACL/不披露/审计测试；
- 过期修订、被拒能力、适配器崩溃、Server 重启、不兼容版本和部分会话恢复测试；
- 每个打包运行时组件的依赖许可证、供应链、SBOM、更新和回滚证据。

模拟对话、直接源文件写入、未锁定开发者安装或单独一次成功模型响应，均不是第一层证据。

## 当前能力边界

当前 Rust 工作区包含 `weftext-agent`、`weftext-agent-dsh` 和 `weftext-agent-mcp`。第一个 crate 定义能力交集、动作请求、预览、审批决定、运行时能力、握手和规范化事件。第二个负责 DSH 进程启动、协议分帧、严格初始化/版本检查、提示回执、通知映射、stderr 诊断、优雅关闭及基于终止的取消。第三个仅通过 MCP stdio 提供 `workspace_inventory` 和 `read_document`。其启动参数固定一个工作区，文档选择使用重建后有效清单中的节点 UUID，结果使用相对路径，工具目录不含变更、shell、任意路径或外部出口操作。CLI 提供 `weftext agent mcp serve`、`weftext agent dsh support` 和 `weftext agent dsh probe`。

协议测试使用表现为确定性假 DSH 运行时的真实子进程，不需要模型凭据。MCP 测试执行初始化、确定性工具发现、清单、精确源文本读取、初始化前拒绝和越界 UUID 拒绝。以 `@modelcontextprotocol/sdk` 1.12.0——已发布 DSH MCP 客户端的依赖行——进行的集成检查启动原生服务器并成功调用两项工具。

`integrations/dsh/weftext-readonly.cordis.yml` 是 Weftext 自有 DSH 组合配置。它挂载官方 `@deepseek-ai/dsh-mcp-client`，并明确禁用 Bash、原始文件系统、工作区上下文加载、skills、jobs、goals 和 subagents。官方发布的 npm JSON-RPC 示例仍需要部署方拥有的 Cordis 配置，独立发布的候选版本包目前不能构成与此新组合兼容的已打包依赖集。因此 Weftext 报告 `read_only_tools`，其 `ready: false`：源级配置和 MCP 互操作性已有证据，但打包 DSH 验收、模型驱动调用证据、UI 和全部变更流程仍未完成。
