---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext DSH bridge

`weftext-agent-dsh` is the first-party Rust host bridge for the DeepSeek Harness SDK JSON-RPC runtime. It does not bundle Node.js, DSH, a model, credentials, or writable workspace integration.

The bridge launches a caller-selected runtime, performs one version-checked initialization, sends prompt content blocks, normalizes bounded events, shuts down gracefully, and terminates the process for cancellation when the wire protocol has no cancel operation. The CLI can inspect support and probe a caller-provided runtime without sending a model prompt:

```text
weftext agent dsh support
weftext agent dsh probe <runtime> <provider> <model> <cwd> [runtime arguments...]
```

`integrations/dsh/weftext-readonly.cordis.yml` starts the official MCP client with `weftext agent mcp serve <workspace>`. It exposes only bounded workspace inventory and exact-document reads selected by node UUID. It disables shell access, raw filesystem tools, workspace-context loading, skills, jobs, goals, and subagents.

This is a read-only integration foundation. Mutation proposal, user approval, Core commit, packaged-runtime acceptance, and product UI remain required before writable integration can be claimed.
