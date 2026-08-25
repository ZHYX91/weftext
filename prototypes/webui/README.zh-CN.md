---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext WebUI 交互原型

本原型使用模拟数据或一个显式授权的本地 Weftext 工作区，验证 Desktop/WebUI 共享交互模型。它不是 Weftext Server，也不能作为产品完成证据。

## 本地 Core 工作区切片

```text
weftext prototype serve D:\path\to\Workspace
```

命令只绑定 `127.0.0.1`，生成随机 Bearer 令牌并打印 `openUrl`。请打开完整 URL，不要把其中的令牌复制到查询字符串。

在本地模式下，原型通过 Core 读取精确 UTF-8 AsciiDoc 和修订，浏览真实节点树，为访问过的每个节点保留受控草稿，解析当前草稿以提供结构化呈现，在“源文本”视图之外隐藏受保护的身份信封，生成确定性的保存预览，并通过与 CLI 相同的 Core 操作提交。结构操作、链接、潜在提及、搜索、过期修订拒绝和内容边界规则也继续由 Core 负责。浏览器不获得目录句柄，也不能选择任意文件系统路径。

该原型不定义第二种文档格式或额外的独立编辑器模式。尚未完成的生产界面以公开规范为准。

## 验证

```text
npm run lint
npm test
```
