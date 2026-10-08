---
source_language: zh-CN
translation_status: source
---

[English](D8-SOURCE-MAP.md)

# A2 D8 来源映射

状态：fixed5eca 已独立 CLOSED D8-C98B-P1-01/P1-02/P2-02；ff10 又独立 CLOSED D8-C98B-P2-01 与 D8-5ECA-P2-01。唯一剩余的 D8-C98B-P1-03 已由本候选完成作者修订，但仍只是 `author-resolved-pending-independent-review`；作者不自行接受 D8/global A2。

本文件只记录 provenance，不建立第二规范。机器库存继续由 `D8-SOURCE-MAP.json` 承担。本 c98b 修订不重新生成 fixed 八源、160 个 fixed case、FA17 继承、Mandatory/RTL/SEARCH 库存、replacement router 或受保护 inputs；只修复独立复核指出的不完整 D8 current routing/applicability。

## 1. 已保全库存与本次范围

现有机器映射仍包含 54 个 fixed-S D8 prose section、全部 160 个 fixed case、RTL 九问与七类场景、Mandatory §15 的 19 个 heading、SEARCH-01–08、27 个 current D7 Search fixture、两层 replacement router，以及 current AsciiDoc/Annotation acceptance 库存。

这些数量只表示库存存在，不证明每项 source obligation 已完成语义映射，也不证明 D8 已被独立接受。

## 2. c98b 来源映射/applicability 作者修订

54 个 `fixedProseSections` 全部保留原 source path/blob/section/line range，并改为 section-level current target；宽泛整文件 target 现为 0/54。current AsciiDoc/Annotation FC 库存仍严格是 760 条原 source row；每条继续记录原 source path/blob/section/line、current owner、D8 applicability、适用时的 section-level D8 target、disposition 与逐行 audit basis。该库存的顶层 currentAcceptanceInventory path/blob 现在正确指向实际 current final-FC ACCEPTANCE.md 及 blob c3022df8…；每个 row 的 sourceBlob 则有意继续保留 fixed original f5f5b9b2… 与原 text/line/section 作为 provenance。两种 blob 角色不得混为同一 authority。

作者按完整 obligation 文本重新核了 760/760 行；本次残余修订后，共有 114 行 applicability 与原分类相比被调整。当前作者计数为 direct/immediate D8 consumer 167、upstream current prerequisite 146、保留在真实 non-D8 current owner 447。本轮把 T3-XF-17、T3-XF-35、X34-06、FC34-ST-19 调整为与 FC34-ST-02 相同的 upstream-current-prerequisite save/prepare/recovery 边界；其 SourceTransform producer 语义仍归 final AsciiDoc/Annotation SPEC 所有。这些全部只是**作者修订结果**：每个变更行仍是 `author-resolved-pending-independent-review`；section 存在、数量、hash 与 docs CI 只提供机械证据，不等于独立语义接受。

后续 fixed-SHA 非作者复核必须独立验证或修订这些 classification/disposition。历史 decoder/bytes 与原 source-qualified 条件保持不变，不从历史记录虚构 current migration。

## 3. 本窄修复实际改变的内容

此前 c98b/fixed5eca/ff10 修复全部保留为固定历史。本次最终窄修只改最后一个 P1 同族：`VIEW-BLD-03` 澄清饼图 zero-total 是合法 empty-total；`VIEW-BLD-04..06`、Main §11.2、Interfaces §13、Schemas §12 与 Impact §11 把延期的 tree/treemap/sunburst、Gantt、boxplot/quantile 绑定为现任静态 `unsupported_layout`，不再描述成“合法 current 定义仅缺 renderer”。`advanced_required` 只适用于现任 D7 decoder 已接受的 member。raw Source 与真正 historical/future bytes 继续由真实 owner/decoder 保全。机器 map 只更新 current target blob，不改 fixed source bytes。

fixedProseSections 的 source path、blob、section、disposition、owner 与现有 currentTargets 其余内容保持原样。不建立第二份 source catalog 或 replacement inventory。

## 4. 受保护输入与证据边界

受保护 fixed-S 库存继续是 49 个 snapshots 加 docs/design/inputs.json；expected inputs blob 仍为 787d03c31a55496f81ed03fd54a6fdfff50a2ad4。

此前登记为 FULL 的阅读范围继续只表示阅读证据：fixed 八件 D8 输入、D6-FA D8 afterimage、current D8 直接相关的 AsciiDoc/Annotation owner section、D7 Search 与 27 个 fixture、current D7 View contract。它们不等于 D8 语义接受。

D9 与 D10 只在具名 D8 direct producer/consumer 边界记为 PARTIAL；其完整 A2 模块在本轮接受范围仍为 UNREAD。global A2 终审同样 UNREAD。产品 GUI、IME、AT、OS、多副本与性能证据继续 UNRUN。

在 fixed5eca（`5eca16c40cdf2e1892f6930d51c632ea720a460c`）的非作者复审中，D8-C98B-P1-01、D8-C98B-P1-02、D8-C98B-P2-02 已独立 CLOSED。后续 ff10 又独立 CLOSED D8-C98B-P2-01 与 D8-5ECA-P2-01，只留下 D8-C98B-P1-03 OPEN。本候选完成这一个最终窄 P1 的作者修订，但不自行 CLOSED；必须由新的 exact-final-SHA 非作者复核裁决。
