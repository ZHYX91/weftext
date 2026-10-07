---
source_language: zh-CN
translation_status: source
---

[English](D7-SOURCE-MAP.md)
# A2 D7 来源与案例映射

状态：这是完整 D7 作者候选的机器追踪 companion；不是独立接受。

## 1. 固定来源

fixed-S 的十三份 D7 来源均已完整读取；额外的 fixed-S D9 coordinated D3-D7 binding 来源也已完整读取。准确路径和 Git blob 记录在 D7-SOURCE-MAP.json。fixed snapshots 与 docs/design/inputs.json 受保护且保持不变。

当前协调后的 D7 双语 owner 集也已完整读取，并按原 blob 复制到 d7/owners。当前 D9 binding 的中英文来源复制到 d7/coordination。D7-REGISTRY.json 直接复用当前机器 Registry 的准确 blob，继续包含 34 个 concept 和 8 个 cross-stage binding。

## 2. 当前来源与跨 owner 阅读范围

A2 D1 与 D2 为 FULL，其中 D2 包含完整的当前原生 AsciiDoc 产品族。A2 D4 与 D5 为 FULL，并包含 D4 不可变 Catalog 的 7 个全局限制和 95 个具名记录。A2 D6 及其当前 successor schema 已覆盖 D7 所需的全部权威、当前性、索引和恢复交叉。

A2 D3 没有在早先已经完成 A2 整合后再次整篇重复读取；本轮完整读取了 D7 直接依赖的 identity、wire13、稳定地址、Definition Transfer 与恢复章节。D8、D9、D10 刻意只在真实的 D7 producer/consumer 交叉上记为 PARTIAL；它们的完整 A2 模块仍属于后续批次。机器映射明确记录这一区别。

## 3. 案例覆盖

机器映射分别记录 20 项整合义务、SEARCH-01 至 SEARCH-08、36 个保留的 D7 具名场景序号、17 个非回退主题序号、Mandatory A2-01 至 A2-57、16 个 Facet 案例、16 个 People 案例、6 个 ICS 案例，以及完整的 Chart §15 组。

这些序号键不会替代来源中的人类可读名称或条件。每一项都绑定不可变的来源路径和 blob，复核者仍须回到源字节判断；本地复制的当前 scenarios 继续保留全部具名案例。任何组都不能只因为计数完整就自动获得接受结论。

## 4. 具名 current successor

本批只应用窄范围的 current successor：当前 D2DocumentSnapshot/3 的 heading/body 语义、当前 Annotation Value4/R6 正文读取、fixed97 的 PAB4/Action3/Effect3 outer family、D3 wire13/D6 Proof3 当前性，以及新增 SEARCH 设计。历史 decoder bytes 与 saved/planned/unknown 的恢复仍按原记录版本执行。

SourceRevisionPlan/1、/2、/3 继续按真实角色分域，不能因为外层 successor 就统一重编号。MinimumMapping/3 保持原版本。

## 5. SEARCH 来源

SEARCH-01 至 SEARCH-08 来自本轮已授权的内联任务。按要求，私人任务原文不会逐字复制到公开仓库。D7-SOURCE-MAP.json 只记录规范化后的义务 ID、当前落点与复核 oracle；D7-SEARCH 保存可公开的规范设计。这样既遵守不公开私人 Chat 原文的边界，也让八项义务可以逐项审查。

## 6. 导航 finding

A2-D6-01CC-P2-01 与 A2-NAV-454E-P2-01 都只是作者已经修订的导航元数据问题，目前仍等待固定 SHA 的独立复核；本作者不自行关闭它们。

第一项现在只保留一个角色中性的“等待独立复核”当前状态，并把详细 provenance 统一放在这份机器映射中；历史 sourceEvidence 与语义 finding 都不改。第二项把 fixed-5c 的旧句子改成历史时间线：当时作者没有自行关闭；后续 fixed446 独立复核已经在有界范围关闭 D1/D2 映射修订。该有界结论不构成 D6 或全局 A2 接受。

## 7. 证据边界

仓库文档、输入完整性、JSON 和映射检查只能证明机械一致性。产品 runtime、OS、GUI、renderer/export、database、真实 replica、provider、performance、migration 与 activation 行为在本批全部为 UNRUN。

下一位复核者必须绑定实际 final stop SHA，不得追随 moving branch。D8–D10 完整整合以及之后全新的全局非作者复核继续 pending。
