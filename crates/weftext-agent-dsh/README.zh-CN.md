---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext DSH 桥接

`weftext-agent-dsh` 是 DeepSeek Harness SDK JSON-RPC 运行时的第一方 Rust 主机桥接。它不捆绑 Node.js、DSH、模型、凭据或可写工作区集成。

桥接会启动调用方选择的运行时，执行一次经过版本校验的初始化，发送提示内容块，规范化有边界事件，平稳关闭进程，并在协议没有取消操作时通过终止进程取消任务。CLI 可以检查支持状态，也可以在不发送模型提示的情况下探测调用方提供的运行时：

```text
weftext agent dsh support
weftext agent dsh probe <runtime> <provider> <model> <cwd> [runtime arguments...]
```

`integrations/dsh/weftext-readonly.cordis.yml` 启动官方 MCP 客户端和 `weftext agent mcp serve <workspace>`。它只公开有边界的工作区清单和按节点 UUID 选择的精确文档读取，并禁用 Shell、原始文件系统工具、工作区上下文加载、技能、作业、目标和子智能体。

这只是只读集成基础。必须完成变更提案、用户批准、Core 提交、打包运行时验收和产品 UI，才能声称支持可写集成。
