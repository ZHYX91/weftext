---
source_language: zh-CN
translation_status: source
---

[English](README.md)
# A2 最终设计整合候选

状态：设计候选；未独立接受、未实现、未发布。

## 1. 固定对象与作者身份

本 A2 整合以父提交 97f4734f82a760cb6716c8122b84494da2b61164 为固定起点，历史输入 S 固定为 7e18168dad3e6d120fce0dd607dc10fa7894e252。受保护输入清单是 docs/design/inputs.json，其 blob 为 787d03c31a55496f81ed03fd54a6fdfff50a2ad4。

对已经在本目录整合的模块，本目录是唯一 A2 候选正文。早期 snapshots、D6 file-authority owner afterimage 和 AsciiDoc/Annotation final-design candidate 继续作为来源与历史 decoder 证据，但不与 A2 形成第二套 current definition。

本批只整合 D1 与 D2。这是作者候选，不是独立接受。

## 2. 本批进度

| 模块 | 本批 A2 状态 |
| --- | --- |
| D1 | 已整合为 current candidate |
| D2 | 已整合为 current candidate |
| D3 | TODO；已知来源与导航，尚未整合 |
| D4 | TODO；已知来源与导航，尚未整合 |
| D5 | TODO；已知来源与导航，尚未整合 |
| D6 | TODO；只读取 D1/D2 直接相交的 current 段落 |
| D7 | TODO；只读取 D2 相交的 current overlay/consumer 段落 |
| D8 | TODO；只读取 D1/D2 直接相交的 current 段落 |
| D9 | TODO；只读取 D2 相交的 current overlay/consumer 段落 |
| D10 | TODO；已读取 TASK/导航，尚未整合最终模块 |
| Mandatory A2 source | TODO；只有 D1/D2 直接相交部分已具名处理 |

仅知道路径、route、blob 或标题，绝不等于已经语义全文阅读；任何 TODO 模块都不得据此写成 accepted 或 complete。

## 3. 当前文件

- D1.zh-CN.md 是本批自包含 current D1 候选。
- D2.zh-CN.md 是本批自包含 current D2 候选。
- SOURCE-MAP.zh-CN.md 是人类可读的来源与 disposition 图。
- SOURCE-MAP.json 逐条记录 49 个固定 S 输入的 S blob、阅读状态、current 来源以及 source-qualified 义务组。

## 4. 候选内部优先级

对 D1 与 D2，本目录对应文件是 current candidate。只有当 D1.zh-CN.md、D2.zh-CN.md 或 SOURCE-MAP 明确标记 historical decoder 或 source-qualified evidence 时，早期正文才继续承担历史恢复或证据义务。

D3–D10 目前还没有 A2 current definition。它们的 existing owner 继续作为后续批次输入；本目录不能被解读成已经静默替换这些模块。

## 5. 接受边界

D1/D2 整合保留 source-qualified 历史义务和已有 fixed-SHA 限定审查证据，但作者不能独立关闭 A2 finding。除非明确记录某次精确外部执行，否则所有设计/runtime 验收场景都仍是未运行义务。最终 A2 仍须由全新的非作者对一个固定完整 A2 commit 做全局终审。
