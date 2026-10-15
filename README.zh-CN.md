---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext

基于原生 Typst 的文件型知识与写作工具。当前状态：**0.6.0-alpha.1 Windows 本地与受控服务端开发版，完整 v0.6 仍未完成**。托管消费者、同所有者／部分同步和移动树外新 Node 父级的有界 Compound 切片已分批接通与验收；最新完整打包仍是首个 Server / WebUI 切片，后续独立测试程序不冒作新的整包验收。旧原型已归档，不是当前构建依赖。

- [本地候选使用与边界](docs/MVP.zh-CN.md)：写作、属性与 Query、链接与批注、Office 输出与资料交换，以及保存恢复；含构建命令和实际限制。
- [Server / WebUI 指南](docs/SERVER.zh-CN.md)：多用户五档、字段与 Query、批注、资源、结构和输出的分别授权，以及同所有者托管同步；实际已验范围和剩余条件见 MVP。
- [版本路线图](docs/ROADMAP.zh-CN.md)：当前各版本的实施范围与完成条件，包含 v0.6.1／v0.6.2 增强及后续多端验收。

- [需求](docs/REQUIREMENTS.zh-CN.md)：唯一当前需求集；27 组需求及导航、丰富类型、库样式、五档视图、中文默认支持。
- [技术设计](docs/design/README.zh-CN.md)：作者源、Core、编辑器、业务功能；必要 schema、例子和有限实验随文档保留。
- [实施顺序](docs/design/INTEGRATION.zh-CN.md)：纵向闭环、当前 v0.6 进度与后续版本的衔接。
- [最终设计接受](docs/design/REVIEW.zh-CN.md)：固定技术基线、结论与证据限制。
- [验收与发布](docs/WORKFLOW.zh-CN.md)：简短开发入口，区分设计检查与产品实测。

## 本地检查

Python 3.11 或更新版本，无额外 Python 包：

```text
python scripts/check.py --models
```

默认检查当前文档的文件链接、双语配对、JSON 与需求库存；`--models` 在临时副本运行有限设计模型，不改已记录证据。官方 Typst 编译与平台原型按各自 README 手动复现，不在每次纯文字修改时重复构建。

历史原型、旧设计和完整评审可从归档恢复，但不是当前构建依赖。当前合同变化需同步相关消费者与双语文档。尚未交付的软件不能因文档检查通过而标为可用。
