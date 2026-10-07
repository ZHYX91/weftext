---
source_language: zh-CN
translation_status: source
---

[English](D8-SOURCE-MAP.md)

# A2 D8 来源映射

状态：保留既有 D8 作者候选；本继任作者窄批只修复双语同步，等待独立复核。

本文件只记录 provenance，不建立第二规范。现有机器库存继续由 D8-SOURCE-MAP.json 承担。本窄修复不重新生成 fixed 八件输入、160 个 fixed case、Mandatory 库存、current FC 库存或 SEARCH fixture 库存。

## 1. 已保全库存与本次范围

现有机器映射仍包含 54 个 fixed-S D8 prose section、全部 160 个 fixed case、RTL 九问与七类场景、Mandatory §15 的 19 个 heading、SEARCH-01–08、27 个 current D7 Search fixture、两层 replacement router，以及 current AsciiDoc/Annotation acceptance 库存。

这些数量只表示库存存在，不证明每项 source obligation 已完成语义映射，也不证明 D8 已被独立接受。

## 2. 仍开放的来源映射与语义核销缺口

本次双语同步不会改写 fixedProseSections 数组。7e3f 已保全候选中，这个数组共有 54 项，其中 50 项的 currentTargets 仍是宽泛整文件 target，没有精确到 current section。

这个 50/54 的导航缺口继续保持 OPEN。原作者留下的 FC applicability 分类、historical→current disposition 核销，以及逐项义务语义验证也继续 OPEN。文档检查变绿、数量、hash 或来源清单都不能关闭这些缺口。

后续必须由绑定固定 SHA 的非作者完整 D8 复核决定如何关闭或修订。

## 3. 本窄修复实际改变的内容

本修复只同步 D8 main、interfaces、schemas、direction/accessibility、lexicon、impact 与 acceptance 的中英文，并更新机器映射中这些已改变候选文件的 target blob 引用和 OPEN 状态说明。

fixedProseSections 的 source path、blob、section、disposition、owner 与现有 currentTargets 其余内容保持原样。不建立第二份 source catalog 或 replacement inventory。

## 4. 受保护输入与证据边界

受保护 fixed-S 库存继续是 49 个 snapshots 加 docs/design/inputs.json；expected inputs blob 仍为 787d03c31a55496f81ed03fd54a6fdfff50a2ad4。

此前登记为 FULL 的阅读范围继续只表示阅读证据：fixed 八件 D8 输入、D6-FA D8 afterimage、current D8 直接相关的 AsciiDoc/Annotation owner section、D7 Search 与 27 个 fixture、current D7 View contract。它们不等于 D8 语义接受。

D9 与 D10 只在具名 D8 direct producer/consumer 边界记为 PARTIAL；其完整 A2 模块在本轮接受范围仍为 UNREAD。global A2 终审同样 UNREAD。产品 GUI、IME、AT、OS、多副本与性能证据继续 UNRUN。
