---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext

Weftext 是以**原生 Typst（`.typ`）文件**为作者源的知识与写作工具。它围绕笔记、结构化属性、查询和导出组织工作，同时保留原始文件、笔记身份与保存恢复边界。项目**仍在开发，尚未正式发布**；源码中的功能不等于已通过整版或跨平台验收。

## 能做什么

- 管理本地笔记和目录，使用导航、普通写作、属性及 Query；原生 `.typ` 文件始终是作者内容的依据。
- 在受控 Server / WebUI 中使用独立的多用户授权和同所有者同步；未授权的资料不能因索引、缓存或导出而获得额外访问权。
- 按候选范围查看链接与结构关系、多级标签、标题结构和缩略图；这些较新的功能仍须分别核对实际候选的完整性。
- 通过明确的预览、确认及原请求恢复处理保存、结构变化和 Office 输出；初步资料识别需另外安装本机识别运行时，低可信结果不能当成完整成功。

## 当前范围与成熟度

| 阶段 | 已有依据和边界 |
| --- | --- |
| **0.6.0 Windows／NTFS** | 原既定 v0.6 范围在同一最终优化程序上通过 HTTP、相关浏览器和真实 NAS HTTPS 验收，**但没有正式公开发布**；该结论不自动延伸至其他平台。 |
| **0.6.2 本地交付候选** | 初始 Windows 本地／自托管识别链路覆盖 PDF 文本层、PNG／JPEG OCR、首批公文与数电普通发票；独立运行时、部分结果和复杂版式限制仍需核对。 |
| **此前 0.6.3-rc.4 候选** | 导航、属性／Query、标签、关系图、只读标题结构等切片已接入；**整版验收尚未完成**。 |
| **0.7.0-rc.3 开发候选** | 已整合 GUI／CLI、碎片采集、提醒、语言与宿主基础、保存恢复和原生作者关闭生命周期，并接入受控 Linux Render／Notes 服务路径。各候选的检查与实测证据分别记录；完整桌面、安装及跨平台验收仍未完成，**不代表 v0.7 产品或安装包已验收**。 |

各阶段真实记录、支持矩阵与未完成项分别见[本地候选说明](docs/MVP.zh-CN.md)、[路线图](docs/ROADMAP.zh-CN.md)和[验收工作流](docs/WORKFLOW.zh-CN.md)。不要把较早候选的通过记录直接套用到当前提交。

## 在本地了解与检查

1. 先读[本地使用范围](docs/MVP.zh-CN.md)和[Server / WebUI 指南](docs/SERVER.zh-CN.md)，确定适用环境与文件系统；不要拿真实资料库直接做实验。
2. 检查文档、链接和模型约束时，使用 Python 3.11 或更新版本：`python scripts/check.py`。涉及设计模型的修改才另加 `--models`；**这不是产品功能验收**。
3. 在满足开发依赖的 Windows 环境，`python scripts/mvp.py check` 会构建并运行当前本地产品检查。它不发布软件，完整命令、环境及验收含义见[工作流](docs/WORKFLOW.zh-CN.md)。
4. 需要独立命令行时参阅[CLI 使用与原请求恢复](docs/CLI.zh-CN.md)。CLI 与桌面入口使用同一 Core；效果不确定时应查原请求结果，而不是用新编号盲目重试。

## 尚未验证的范围

- Windows 原生窗口与安装交付、真实系统 IME、完整辅助技术、实际断电以及历史 Undo 异常仍有独立验收限制。
- 完整 Linux、Android 和 Apple 产品及相应文件系统、输入设备与平台性能不能靠 Windows／浏览器或模拟测试推定通过。
- Obsidian 等插件仅供独立交互设计参考；**Weftext 不依赖其运行，也没有与 Obsidian Vault 联动或同步的功能**。

## 技术资料

- [需求与范围](docs/REQUIREMENTS.zh-CN.md) · [技术设计](docs/design/README.zh-CN.md) · [实施顺序](docs/design/INTEGRATION.zh-CN.md)
- [固定设计评审](docs/design/REVIEW.zh-CN.md) · [本地候选](docs/MVP.zh-CN.md) · [Server / WebUI](docs/SERVER.zh-CN.md)
- [CLI](docs/CLI.zh-CN.md) · [Linux 受控工作进程](docs/LINUX-WORKERS.zh-CN.md) · [路线图](docs/ROADMAP.zh-CN.md) · [开发、验收与发布](docs/WORKFLOW.zh-CN.md)

设计检查、真实产品测试和正式发布是三个不同的结论；通过文档检查不表示候选已经可供正式使用。
