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

当前作者候选保留 D1–D8，并从 exact `5eca16c40cdf2e1892f6930d51c632ea720a460c` 完成 D9 全量作者整合。D7 已在 fixed26be 获非作者 PASS。fixed5eca 的 D8 closure 保持，后续 ff10 又独立 CLOSED D8-C98B-P2-01 与 D8-5ECA-P2-01；现在只剩 D8-C98B-P1-03 OPEN，范围已收窄到 VIEW-BLD-04/05/06 与 Main §11.2 的延期布局保存措辞。ROOT-D9-ZH-P2-01 已在 ff10 独立 CLOSED。当前 D9 三项 finding 的修复仍只是 `author-resolved-pending-independent-review`；D10 完整模块留后续。

## 2. 本批进度

| 模块 | 本批 A2 状态 |
| --- | --- |
| D1 | 已整合为 current candidate；fixed-446 在其有界范围内独立关闭 D1/D2 findings |
| D2 | 已整合为 current candidate；fixed-446 在其有界范围内独立关闭 D1/D2 findings |
| D3 | 已整合为 current 作者候选；另一个 fixed-a62 非作者窄复核已 CLOSED 两个有界 D3 P1 finding；这不是 global A2 acceptance |
| D4 | 已整合为 current 作者候选；A2-D4D5:P2-02 继续在 fixed4282 有界 CLOSED，A2-D4D5:P2-01 已在 fixed1fc4 独立复核后 bounded CLOSED；不构成全局接受 |
| D5 | 已整合为 current 作者候选；A2-D4D5:P2-02 继续在 fixed4282 有界 CLOSED，A2-D4D5:P2-01 已在 fixed1fc4 独立复核后 bounded CLOSED；不构成全局接受 |
| D6 | fixed2f89 已独立 CLOSED fixed454e 之后剩余的两项 navigation finding；既有 D6 bounded semantic closure 仍只保留各自记录范围。 |
| D7 | fixed26be 非作者复核 = PASS（P0=0/P1=0/P2=0）；最终 A2-D7-2F89-P2-01 在该 SHA CLOSED；此前有界 closure 继续绑定各自原 SHA |
| D8 | 已保全作者候选；fixed5eca 已独立 CLOSED P1-01/P1-02/P2-02，ff10 又独立 CLOSED P2-01 与 D8-5ECA-P2-01；现在只剩 P1-03 在收窄后的延期布局保存冲突上 OPEN；acceptance JSON 继续是唯一六字段结构源，213/213 唯一 ID 与确定性 guard 保持；source-map 审计仍为 direct 167 / upstream 146 / 保留 owner 447，共 114 行作者 applicability 改动 |
| D9 | 完整作者候选：八份 fixed-S 来源均已全文读取，现任 D9 owner 中英文均已全文读取，Mandatory §14 与 D9 适用 Mandatory §15 均已覆盖；D7 的 binding/PAB/View consumer 已覆盖，final-FC D9 successor 已覆盖；现任使用 wire13/PAB4/Effect3 与 Plan4/Catalog3/Selection2/Projection2/Loss2/Confirmation2/Receipt4/PrintReceipt1，并精确恢复 Plan/Receipt1–3；Acceptance 共 97 行，Registry/Terms 共 41 个 concept；三个 D9-BAF finding 等待绑定最终 SHA 的独立复核 |
| D10 | 完整模块仍 TODO；D7 只消费 SearchContribution/Catalog 与 D7 approval/effects/custody 交叉 |
| Mandatory A2 source | D8 已完整消费 D8 适用的 §15.1–15.8.9 chart/View interaction 与 RTL intake；D9 消费 Mandatory §14 的 884–924 行，以及 D9 适用 §15.5–15.8 的 1042–1141 行和全部十个 View/export fixture；D10 owner module 完整整合留后续 |

仅知道路径、route、blob 或标题，绝不等于已经语义全文阅读；任何 TODO 模块都不得据此写成 accepted 或 complete。D8、D9 都已不再是 TODO，但仍只是未接受、等待独立复核的作者候选。

## 3. 当前文件

- D1.zh-CN.md 是本批自包含 current D1 候选。
- D2.zh-CN.md 是自包含 current D2 候选。
- D3.zh-CN.md、D3-IMPACT.zh-CN.md、D3-LEXICON.zh-CN.md、D3-SCHEMAS.zh-CN.md 与 D3-TERMS.json 共同构成 current D3 作者候选。
- D3-SOURCE-MAP.json 记录 D3 的来源限定 case，以及直接 current coordination row。
- D4.zh-CN.md、D4-IMPACT.zh-CN.md、D4-LEXICON.zh-CN.md 构成 D4 作者候选；D4-SOURCE-MAP.json 与 D4-CATALOG-MAP.json 保留 provenance。
- D5.zh-CN.md、D5-IMPACT.zh-CN.md、D5-LEXICON.zh-CN.md 构成 D5 作者候选；D5-SOURCE-MAP.json 保留 provenance。
- D6.zh-CN.md、D6-CONTROL.zh-CN.md、D6-SCHEMAS.zh-CN.md、D6-IMPACT.zh-CN.md、D6-LEXICON.zh-CN.md 与 D6-REGISTRY.json 构成 current D6 作者候选；D6-SOURCE-MAP.json 保存细粒度 provenance。
- D7.zh-CN.md、D7-SCHEMAS.zh-CN.md、D7-QUERY-V2.zh-CN.md、D7-SEARCH.zh-CN.md、D7-IMPACT.zh-CN.md、D7-REGISTRY.json 与 D7-REGISTRY-QUALIFICATION.zh-CN.md，再加上逐字节保留的 d7/owners 子树，共同构成当前 D7 作者候选；D7-SEARCH-FIXTURES.json 保存快捷解析的机器验收例，D7-SOURCE-MAP.json 保存机器来源追踪。
- D8.zh-CN.md、D8-INTERFACES.zh-CN.md、D8-SCHEMAS.zh-CN.md、D8-DIRECTION.zh-CN.md、D8-ACCEPTANCE.zh-CN.md、D8-LEXICON.zh-CN.md、D8-IMPACT.zh-CN.md、D8-TERMS.json、D8-REGISTRY.json 与 D8-SOURCE-MAP.json 构成已保全的 D8 作者候选。fixed5eca 已 CLOSED D8-C98B-P1-01/P1-02/P2-02；ff10 又独立 CLOSED D8-C98B-P2-01 与 D8-5ECA-P2-01。只剩收窄后的 D8-C98B-P1-03 延期布局保存冲突 OPEN，当前 D9 批不修它。
- D9.zh-CN.md、D9-INTERFACES.zh-CN.md、D9-SCHEMAS.zh-CN.md、D9-ACCEPTANCE.zh-CN.md/JSON、D9-IMPACT.zh-CN.md、D9-LEXICON.zh-CN.md、D9-TERMS.json、D9-REGISTRY.json 与 D9-SOURCE-MAP.zh-CN.md/JSON 共同构成完整 D9 作者候选。qualified native-table Office authoring 对 simple `data.native_table.COLUMN` 保持原拼法；fresh qualified/non-ASCII 情况使用可见的 `native.table[...]::column[...]`；internal `nt_/nc_` 仅作为 Plan metadata，不产生另一套作者权威。
- REVIEW-ENTRY.zh-CN.md 是 D1–D9 作者候选复核入口。
- SOURCE-MAP.zh-CN.md 是人类可读的来源与 disposition 图。
- SOURCE-MAP.json 逐条记录 49 个固定 S 输入的 S blob、阅读状态、current 来源以及 source-qualified 义务组。

## 4. 候选内部优先级

对 D1–D9，本目录对应文件是 current candidate。只有当 D1.zh-CN.md、D2.zh-CN.md 或 SOURCE-MAP 明确标记 historical decoder 或 source-qualified evidence 时，早期正文才继续承担历史恢复或证据义务。

D10 目前还没有完整 A2 current definition。D9 只读取 D10 的具名直接交叉（specialized executor、worker no-network、Mobile negative surface、PublicationReceipt/Resource 分权与 technical-interface naming）；该 PARTIAL 阅读不等于完整 D10 已整合。

## 5. 接受边界

后续非作者复核绑定 fixed26be（`26be071d4c2075343d9ebf272e00f23769ce0d64`），结论 PASS：P0=0 / P1=0 / P2=0，并 CLOSED 最终 `A2-D7-2F89-P2-01`。此前有界 closure 继续绑定各自原 SHA。D7 因而在 D7 范围 accepted；这不接受 D8 或 global A2。

D8 下一步是单独对 D8-C98B-P1-03 做最窄作者修复与非作者复审；fixed5eca/ff10 已关闭项都不重开。D9 下一步只对 D9-BAF-P1-01、D9-BAF-P1-02、D9-BAF-P2-02 做绑定最终 SHA 的修复增量复审，同时核本三修批新增中文、97 行 acceptance authority、41 concept 的 registry/terms、Plan4 Annotation/View carrier、Mandatory §15 consumer 与修正后的 source range。ROOT-D9-ZH-P2-01 已在 ff10 CLOSED，不重开。两种 scope 互不冒充接受。D10 完整整合与 fresh independent Pro/global review 继续 pending；产品/runtime 证据仍 UNRUN。
