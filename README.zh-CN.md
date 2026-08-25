---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext / 文缕

> [!WARNING]
> **这是一个个人玩具项目，仅用于学习和实验。** 它尚未达到适合实际使用的软件标准，不建议用于实际工作、重要数据或生产环境。请假定它可能损坏或丢失数据；本项目不承诺稳定性、安全性、兼容性、持续维护或技术支持。

Weftext 是一个主要使用 Rust 编写、采用本地文件存储的实验性知识工作区。其设计包括目录节点、精确源文本、共享 Core、Windows Desktop、带浏览器 WebUI 的 Server、CLI，以及用于显式打开工作区外 AsciiDoc 文件的独立编辑器。

## 格式与权威

每个受管节点都是一个目录，其中包含与目录同名的 AsciiDoc 文档：

```text
Project/
├─ Project.adoc
├─ image.png
└─ Child/
   └─ Child.adoc
```

工作区根目录的 `.weftext-format` 必须精确包含 `weftext.asciidoc.v1\n`。标记缺失、未知或格式错误时，系统须安全拒绝或进入显式采用/导入流程。文件扩展名不能选择工作区权威。

每个节点都在 `weftext.id` 中保存持久 UUID。路径只是定位符，不是身份。可移植的非正文评论、高亮、审阅标记和建议保存在可选的节点本地 `weftext.annotations.json` 伴随文件中。搜索索引、反向链接、关系图、缩略图、看板、集合和统计均为可重建的派生结果；同步数据库不作为内容权威。

原生 AsciiDoc 待办项是轻量、无身份的出现项。需要持久身份或类型化字段时，用户显式将其提升为普通受管任务节点；节点现有 UUID 即任务身份，原位置改为稳定的 `node:` 链接。

Markdown 可作为显式导入/导出输入、可见的非受管内容或节点拥有的普通附件，但不是受管节点语言，也不是独立编辑器模式。Markdown 导入支持基础语法，并可通过有边界、显式版本化的兼容配置识别部分扩展方言。

## 产品端与主要组成

- `crates/weftext-core`：节点、身份、内容、事务、恢复、搜索、批注、Chrono、Query 和派生投影的权威。
- `apps/desktop`：Windows 桌面应用、共享 React UI、原生工作区选择、设备本地草稿恢复、安全模式和直接 Core 命令。
- `crates/weftext-server`：托管工作区与认证 API 基础，以及同源浏览器客户端；尚不属于可部署的多用户软件。
- `crates/weftext-cli`：通过同一组 Core 操作提供无界面访问。
- `crates/weftext-agent*`：受监督、能力受限的智能体集成；智能体不获得直接写入工作区的权威。
- `docs/specifications`：规范性的公开格式与产品契约。
- `docs/architecture`：当前公开架构决策。
- `docs/guides`：按用户任务组织的入门指南，并明确区分当前基础与仍在实现的设计。
- [公开文档术语](docs/TERMINOLOGY.zh-CN.md)：统一术语和写作规则。
- `ROADMAP.md`：简洁的公开状态和发布方向。

受管节点编辑器会随底层 Core 能力就绪而提供保留源文本的可视化命令、按任务组织的功能区、上下文“表格”和“图片”标签、检查器、格式刷、表格结构操作、模板、Query 驱动的集合、任务、看板、多维表格、历史、比较及备份入口。独立 AsciiDoc 模式复用安全的单文件编辑与渲染，但不提供依赖工作区的身份、导航、Query、模板库、持久任务、引文解析、历史或备份权威。

## 当前状态

已经实现的基础包括精确修订、可恢复工作区事务、无损文档编辑、存储分类、搜索、结构化文档模型、节点资源、批注、Chrono 操作、共享导航、Windows 桌面 Alpha 版本和仅限回环地址的服务器基础。完整编辑器、托管授权模型、导入/导出流水线、协作系统、打包可访问性矩阵和公开发布门槛尚未完成。详见[公开路线图](ROADMAP.zh-CN.md)。

## 构建与本地开发

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

在 Windows 上，安装 Microsoft C++ Build Tools 后，`apps/desktop/build-windows.cmd` 可构建本地 UI 和 NSIS 安装程序。

开发服务器只能在回环地址运行：

```text
cargo run -p weftext-server -- <workspace> --bind 127.0.0.1:8787
```

它会报告 `deploymentReady=false`；不要通过反向代理或内网地址对外暴露。
