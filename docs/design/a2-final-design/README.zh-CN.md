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

当前 stopped 作者候选已整合 D1、D2、D3；仍只是作者候选，不是独立接受。

## 2. 本批进度

| 模块 | 本批 A2 状态 |
| --- | --- |
| D1 | 已整合为 current candidate |
| D2 | 已整合为 current candidate |
| D3 | 已整合为 current 作者候选；待非作者复核 |
| D4 | TODO；已知来源与导航，尚未整合 |
| D5 | TODO；已知来源与导航，尚未整合 |
| D6 | 完整模块仍 TODO；已读 D1/D2 交集与 D3-direct 中文 owner 输入 |
| D7 | 完整模块仍 TODO；已读 D2 交集与 D3-direct 中文 owner 输入 |
| D8 | 完整模块仍 TODO；已读 D1/D2 交集与 D3-direct 中文 owner 输入 |
| D9 | 完整模块仍 TODO；已读 D2 交集与 D3-direct 中文 owner 输入 |
| D10 | 完整模块仍 TODO；已读 TASK/导航与部分 D3-direct 中文 owner 输入 |
| Mandatory A2 source | TODO；只有 D1/D2 直接相交部分已具名处理 |

仅知道路径、route、blob 或标题，绝不等于已经语义全文阅读；任何 TODO 模块都不得据此写成 accepted 或 complete。

## 3. 当前文件

- D1.zh-CN.md 是本批自包含 current D1 候选。
- D2.zh-CN.md 是自包含 current D2 候选。
- D3.zh-CN.md、D3-IMPACT.zh-CN.md、D3-LEXICON.zh-CN.md、D3-SCHEMAS.zh-CN.md 与 D3-TERMS.json 共同构成 current D3 作者候选。
- D3-SOURCE-MAP.json 记录 D3 source-qualified cases 与 direct current coordination row。
- REVIEW-ENTRY.zh-CN.md 是 stopped D1-D3 作者候选的复核入口。
- SOURCE-MAP.zh-CN.md 是人类可读的来源与 disposition 图。
- SOURCE-MAP.json 逐条记录 49 个固定 S 输入的 S blob、阅读状态、current 来源以及 source-qualified 义务组。

## 4. 候选内部优先级

对 D1、D2、D3，本目录对应文件是 current candidate。只有当 D1.zh-CN.md、D2.zh-CN.md 或 SOURCE-MAP 明确标记 historical decoder 或 source-qualified evidence 时，早期正文才继续承担历史恢复或证据义务。

D4–D10 目前还没有完整 A2 current definition；本批为闭合 D3 而消费的 direct producer/consumer 不等于这些完整 owner 模块已被静默整合。

## 5. 接受边界

D1/D2/D3 整合保留 source-qualified 历史义务和已有 fixed-SHA 限定审查证据，但作者不能独立关闭 A2 finding。除非明确记录某次精确外部执行，否则所有设计/runtime 验收场景都仍是未运行义务。最终 A2 仍须由全新的非作者对一个固定完整 A2 commit 做全局终审。
