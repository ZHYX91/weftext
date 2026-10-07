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

当前作者候选已整合 D1–D6；仍只是作者候选，不是独立接受。D7–D10 完整模块留待后续批次。

## 2. 本批进度

| 模块 | 本批 A2 状态 |
| --- | --- |
| D1 | 已整合为 current candidate；fixed-446 在其有界范围内独立关闭 D1/D2 findings |
| D2 | 已整合为 current candidate；fixed-446 在其有界范围内独立关闭 D1/D2 findings |
| D3 | 已整合为 current 作者候选；另一个 fixed-a62 非作者窄复核已 CLOSED 两个有界 D3 P1 finding；这不是 global A2 acceptance |
| D4 | 已整合为 current 作者候选；P2-02 继续仅在 fixed4282 有界 CLOSED；fixed32cf 已独立 CLOSED P2-01 R2，并通过 71/72 条 D4 R1；唯一 R1 残余已在当前候选修订，仍为 author-resolved-pending-independent |
| D5 | 已整合为 current 作者候选；P2-02 继续仅在 fixed4282 有界 CLOSED；fixed32cf 已独立 CLOSED P2-01 R2，并通过 117/130 条 D5 R1；13 条 D5 R1 残余已在当前候选修订，仍为 author-resolved-pending-independent |
| D6 | fixed01cc 独立复核已 CLOSED A2-D6-829-P1-01 与 P2-03，并留下 P1-02/P2-01/P2-02 与新增导航 P2 OPEN/PARTIAL；四项残余从 `32cfb9c387deddb12fb021a44147df0d7ffab322` 修订，目前仅 author-resolved-pending-independent |
| D7 | 完整模块仍 TODO；已读 D2 交集与 D3-direct 中文 owner 输入 |
| D8 | 完整模块仍 TODO；已读 D1/D2 交集与 D3-direct 中文 owner 输入 |
| D9 | 完整模块仍 TODO；已读 D2 交集与 D3-direct 中文 owner 输入 |
| D10 | 完整模块仍 TODO；已读 TASK/导航与部分 D3-direct 中文 owner 输入 |
| Mandatory A2 source | D4/D5 适用 §1–§14（1–924 行）已读并映射；925–1141 行留待后续批次 |

仅知道路径、route、blob 或标题，绝不等于已经语义全文阅读；任何 TODO 模块都不得据此写成 accepted 或 complete。

## 3. 当前文件

- D1.zh-CN.md 是本批自包含 current D1 候选。
- D2.zh-CN.md 是自包含 current D2 候选。
- D3.zh-CN.md、D3-IMPACT.zh-CN.md、D3-LEXICON.zh-CN.md、D3-SCHEMAS.zh-CN.md 与 D3-TERMS.json 共同构成 current D3 作者候选。
- D3-SOURCE-MAP.json 记录 D3 source-qualified cases 与 direct current coordination row。
- D4.zh-CN.md、D4-IMPACT.zh-CN.md、D4-LEXICON.zh-CN.md 构成 D4 作者候选；D4-SOURCE-MAP.json 与 D4-CATALOG-MAP.json 保留 provenance。
- D5.zh-CN.md、D5-IMPACT.zh-CN.md、D5-LEXICON.zh-CN.md 构成 D5 作者候选；D5-SOURCE-MAP.json 保留 provenance。
- D6.zh-CN.md、D6-CONTROL.zh-CN.md、D6-SCHEMAS.zh-CN.md、D6-IMPACT.zh-CN.md、D6-LEXICON.zh-CN.md 与 D6-REGISTRY.json 构成 current D6 作者候选；D6-SOURCE-MAP.json 保存细粒度 provenance。
- REVIEW-ENTRY.zh-CN.md 是 D1-D6 作者候选复核入口。
- SOURCE-MAP.zh-CN.md 是人类可读的来源与 disposition 图。
- SOURCE-MAP.json 逐条记录 49 个固定 S 输入的 S blob、阅读状态、current 来源以及 source-qualified 义务组。

## 4. 候选内部优先级

对 D1–D6，本目录对应文件是 current candidate。只有当 D1.zh-CN.md、D2.zh-CN.md 或 SOURCE-MAP 明确标记 historical decoder 或 source-qualified evidence 时，早期正文才继续承担历史恢复或证据义务。

D7–D10 目前还没有完整 A2 current definition；为闭合 D3–D5 而消费的 direct producer/consumer 不等于完整 owner 模块已被静默整合。

## 5. 接受边界

D1–D6 整合保留 source-qualified 历史义务和 fixed-SHA 有界审查证据。fixed-446 已独立关闭有界 D1/D2 findings，fixed-a62 已独立 CLOSED 两个有界 D3 P1。fixed4282 仅独立 CLOSED D4/D5 P2-02 的“六份 D10 真实来源资格”有界范围。fixed32cf 对 P2-01 给出 REVISE P0=0/P1=0/P2=1：R2 已独立 CLOSED；R1 在 D4+D5 合计 202 条中通过 188 条并留下 14 条残余。当前候选已修这 14 条映射，但仍仅 author-resolved-pending-independent，等待新的 exact-stop 复核。D6 非作者在 fixed01cc 独立 CLOSED P1-01 与 P2-03，但 REVISE 保留 P1-02/P2-01/P2-02 和导航 P2 OPEN/PARTIAL。当前 D6 四残余修订从 fixed32cf 开始，仅为 author-resolved-pending-independent；D6 未 accepted。D7–D10 完整模块、Mandatory 925–1141 与 fresh global review 仍 pending。runtime/OS/GUI/真实 replica/migration/activation 全部 UNRUN。
