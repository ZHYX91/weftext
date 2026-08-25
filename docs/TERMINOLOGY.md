---
source_language: zh-CN
translation_of: TERMINOLOGY.zh-CN.md
translation_status: synced
---

[简体中文](TERMINOLOGY.zh-CN.md)

# Public documentation terminology

This glossary keeps terminology consistent across Weftext's public Chinese and English documentation. It supports readability but does not replace types, fields, or diagnostic codes defined by the specifications. Code identifiers, filenames, product component names, and commands that have not been localized remain unchanged.

## Product and interface

| English concept | Preferred Chinese | Usage |
| --- | --- | --- |
| workspace | 工作区 | Do not use 工作空间 |
| managed node | 受管节点 | A directory node governed by the Weftext format and Core |
| unmanaged content | 非受管内容 | Visible files or directories that do not participate in node semantics |
| source text | 源文本 | The exact document text saved by the author |
| product surface | 产品端、产品界面 | Choose by context; do not translate literally as 产品表面 |
| UI surface | 界面、操作界面 | Avoid a literal 表面 translation |
| syntax surface | 语法形式 | Syntax that an author can write |
| Inspector | 检查器 | When retaining the English product name, write Inspector（检查器） |
| ribbon | 功能区 | The editor's task-oriented top tabs |

## Storage and safety

| English concept | Preferred Chinese | Usage |
| --- | --- | --- |
| authority | 权威、权威源 | The final source of truth; do not translate as 权限 |
| authorization | 授权 | Keep distinct from authority |
| envelope | 元数据封装 | A protected system-metadata range in a document |
| sidecar | 伴随文件 | A role-constrained file adjacent to the main document |
| inventory | 清单、盘点 | Use 清单 for a set of files and 盘点 for the action |
| payload | 载荷 | Complete content carried by a transaction or Trash item |
| fail closed | 安全拒绝 | Reject an operation when safety or a unique meaning cannot be established |
| Trash | 回收站 | The product feature name; code identifiers remain unchanged |
| draft | 草稿 | Do not use 草案 for uncommitted editor content |
| receipt | 回执 | Durable result evidence for a commit, import, backup, or restore |

## Editing, data, and execution

| English concept | Preferred Chinese | Usage |
| --- | --- | --- |
| native checklist | 原生待办项 | A lightweight checklist occurrence in AsciiDoc source |
| task node | 任务节点 | A task with persistent node identity |
| promotion | 提升 | Converting a native checklist into a task node |
| native table | 原生表格 | An ordinary AsciiDoc table |
| multidimensional table | 多维表格 | A table node with typed records and multiple views |
| record | 记录 | A row in a multidimensional table; it is not a node |
| worker | 工作进程 | A constrained import, OCR, or conversion process |
| caller | 调用方 | Desktop, WebUI, Server, CLI, or an approved agent |
| actor | 操作者 | A human or service identity that is authorized, initiates an operation, and receives audit attribution |
| participant | 参与者 | Use only for collaboration contexts such as annotation conversations, members, or presence |
| agent | 智能体 | An AI agent; proxy in a networking context remains 代理 |
| profile | 规范配置、配置档 | Choose by context and avoid confusion with an ordinary configuration file |
| projection | 投影 | A rebuildable result derived from authoritative content |

## Writing rules

- Explain the user impact of an uncommon technical concept in plain language when it first appears.
- Distinguish implemented foundations, accepted product contracts still being built, and unmet release gates.
- Organize user guides around tasks and outcomes; use verifiable constraint language in specifications.
- Prefer short sentences and paragraphs. Give each paragraph one main point.
- Chinese and English must express the same meaning but need not be literal, sentence-by-sentence translations.
- Public documentation describes only the current contract and status; research comparisons, internal schedules, and decision history stay outside it.
