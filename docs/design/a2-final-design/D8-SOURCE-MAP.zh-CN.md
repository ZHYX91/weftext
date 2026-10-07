---
source_language: zh-CN
translation_status: source
---

[English](D8-SOURCE-MAP.md)

# A2 D8 来源映射

状态：c98b 的五项 D8 finding 已由作者修订；仅为 `author-resolved-pending-independent-review`。作者不自行关闭 P1/P2，也不接受 D8/global A2。

本文件只记录 provenance，不建立第二规范。机器库存继续由 `D8-SOURCE-MAP.json` 承担。本 c98b 修订不重新生成 fixed 八源、160 个 fixed case、FA17 继承、Mandatory/RTL/SEARCH 库存、replacement router 或受保护 inputs；只修复独立复核指出的不完整 D8 current routing/applicability。

## 1. 已保全库存与本次范围

现有机器映射仍包含 54 个 fixed-S D8 prose section、全部 160 个 fixed case、RTL 九问与七类场景、Mandatory §15 的 19 个 heading、SEARCH-01–08、27 个 current D7 Search fixture、两层 replacement router，以及 current AsciiDoc/Annotation acceptance 库存。

这些数量只表示库存存在，不证明每项 source obligation 已完成语义映射，也不证明 D8 已被独立接受。

## 2. c98b 来源映射/applicability 作者修订

54 个 `fixedProseSections` 全部保留原 source path/blob/section/line range，并改为 section-level current target；宽泛整文件 target 现为 0/54。current AsciiDoc/Annotation FC 库存仍严格是 760 条原 source row；每条现在都记录原 source path/blob/section/line、current owner、D8 applicability、适用时的 section-level D8 target、disposition 与逐行 audit basis。

作者按完整 obligation 文本重新核了 760/760 行；本次残余修订后，共有 114 行 applicability 与原分类相比被调整。当前作者计数为 direct/immediate D8 consumer 167、upstream current prerequisite 146、保留在真实 non-D8 current owner 447。本轮把 T3-XF-17、T3-XF-35、X34-06、FC34-ST-19 调整为与 FC34-ST-02 相同的 upstream-current-prerequisite save/prepare/recovery 边界；其 SourceTransform producer 语义仍归 final AsciiDoc/Annotation SPEC 所有。这些全部只是**作者修订结果**：每个变更行仍是 `author-resolved-pending-independent-review`；section 存在、数量、hash 与 docs CI 只提供机械证据，不等于独立语义接受。

后续 fixed-SHA 非作者复核必须独立验证或修订这些 classification/disposition。历史 decoder/bytes 与原 source-qualified 条件保持不变，不从历史记录虚构 current migration。

## 3. 本窄修复实际改变的内容

本修订同步 D8 main、interfaces、schemas、direction/accessibility、lexicon、impact 与 acceptance 中英文；同时修复 View builder/旧入口合同、验收 ID/极性、机器 map target blob，并记录五项 finding 的作者修订状态，但不自行关闭复核。

fixedProseSections 的 source path、blob、section、disposition、owner 与现有 currentTargets 其余内容保持原样。不建立第二份 source catalog 或 replacement inventory。

## 4. 受保护输入与证据边界

受保护 fixed-S 库存继续是 49 个 snapshots 加 docs/design/inputs.json；expected inputs blob 仍为 787d03c31a55496f81ed03fd54a6fdfff50a2ad4。

此前登记为 FULL 的阅读范围继续只表示阅读证据：fixed 八件 D8 输入、D6-FA D8 afterimage、current D8 直接相关的 AsciiDoc/Annotation owner section、D7 Search 与 27 个 fixture、current D7 View contract。它们不等于 D8 语义接受。

D9 与 D10 只在具名 D8 direct producer/consumer 边界记为 PARTIAL；其完整 A2 模块在本轮接受范围仍为 UNREAD。global A2 终审同样 UNREAD。产品 GUI、IME、AT、OS、多副本与性能证据继续 UNRUN。

在 fixed5eca（`5eca16c40cdf2e1892f6930d51c632ea720a460c`）的非作者复审中，D8-C98B-P1-01、D8-C98B-P1-02、D8-C98B-P2-02 已独立 CLOSED；D8-C98B-P1-03 与 D8-C98B-P2-01 仍 OPEN，并新增 D8-5ECA-P2-01 为 OPEN。本轮作者修订只处理这三个残余对象，不自行把任何一个改成 CLOSED。
