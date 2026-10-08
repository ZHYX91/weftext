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

本 A2 作者候选现已整合 D1–D10，包括 D10 原九组完整双语正文与真实 A2 D6/D7/D8/D9 继任 owner。fixed8d7 非作者 ACCEPT 0P0/0P1/0P2 仅在 D9 有界范围关闭新增 D9-6012-P1-01/P2-01；此前全部独立 CLOSED 继续按原范围。**这绝不是 D10 或 global A2 独立接受**；产品/runtime/实施仍 UNRUN。

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
| D8 | 已保全设计候选；fixed5eca/ff10 的有界关闭范围保持，fixed8b6 又独立 CLOSED 最后一个 D8-C98B-P1-03。Acceptance JSON 仍为 213/213 唯一 ID 并保留确定性 guard；来源审计维持 direct 167 / upstream 146 / 保留 owner 447，共 114 行 applicability 改动；不是 A2 全局接受 |
| D9 | fixed8d7 非作者 ACCEPT 0P0/0P1/0P2 只覆盖完整 D9 有界范围；保留 fresh Plan4/Catalog3/Selection2/Projection2/Loss2/Confirmation2/Receipt4/PrintReceipt1、Annotation Value4/R6、六图表 complete_data、历史 Plan/Receipt1–3 精确恢复、104 个稳定验收 ID 与 41 概念。较早有界 CLOSED 不重开，不等于 D10/global 接受。 |
| D10 | 九组 EN/ZH 来源已全文纳入 current A2 作者候选；D10-CONTROL 自足承接 D6 现任 mixed-version record/proof/schedule/approval/custody 继任类型与十九类可读视图，D10-UPSTREAM 对接实际 D8/D9；原 125 项 U/F/A/T/E/P/D 保留来源，并由唯一 D10-ACCEPTANCE.json 投影双语正反 oracle；D10/全局非作者审查 PENDING，产品 UNRUN。 |
| Mandatory A2 source | 固定 S Mandatory §§1–15 原 1141 行及跨模块未编号条件保留。原 D8/D9 范围不改，D10 当前作者整合消费 §§2–13 中 D10 相关义务、§14 实际 D9 Office 与 §15 完整 Query/View/D8/D9；125 项 D10 原场景及负面门禁保留逐项来源，不当作产品证据。 |

D1–D10 现均有 A2 作者候选。完整来源读取与一致性检查都不代表独立设计接受、运行激活或产品实施；后续必须分别通过 D10 非作者完整复核和全新 Pro 完整 A2 终审。

## 3. 当前文件

- D1.zh-CN.md 是本批自包含 current D1 候选。
- D2.zh-CN.md 是自包含 current D2 候选。
- D3.zh-CN.md、D3-IMPACT.zh-CN.md、D3-LEXICON.zh-CN.md、D3-SCHEMAS.zh-CN.md 与 D3-TERMS.json 共同构成 current D3 作者候选。
- D3-SOURCE-MAP.json 记录 D3 的来源限定 case，以及直接 current coordination row。
- D4.zh-CN.md、D4-IMPACT.zh-CN.md、D4-LEXICON.zh-CN.md 构成 D4 作者候选；D4-SOURCE-MAP.json 与 D4-CATALOG-MAP.json 保留 provenance。
- D5.zh-CN.md、D5-IMPACT.zh-CN.md、D5-LEXICON.zh-CN.md 构成 D5 作者候选；D5-SOURCE-MAP.json 保留 provenance。
- D6.zh-CN.md、D6-CONTROL.zh-CN.md、D6-SCHEMAS.zh-CN.md、D6-IMPACT.zh-CN.md、D6-LEXICON.zh-CN.md 与 D6-REGISTRY.json 构成 current D6 作者候选；D6-SOURCE-MAP.json 保存细粒度 provenance。
- D7.zh-CN.md、D7-SCHEMAS.zh-CN.md、D7-QUERY-V2.zh-CN.md、D7-SEARCH.zh-CN.md、D7-IMPACT.zh-CN.md、D7-REGISTRY.json 与 D7-REGISTRY-QUALIFICATION.zh-CN.md，再加上逐字节保留的 d7/owners 子树，共同构成当前 D7 作者候选；D7-SEARCH-FIXTURES.json 保存快捷解析的机器验收例，D7-SOURCE-MAP.json 保存机器来源追踪。
- D8.zh-CN.md、D8-INTERFACES.zh-CN.md、D8-SCHEMAS.zh-CN.md、D8-DIRECTION.zh-CN.md、D8-ACCEPTANCE.zh-CN.md、D8-LEXICON.zh-CN.md、D8-IMPACT.zh-CN.md、D8-TERMS.json、D8-REGISTRY.json 与 D8-SOURCE-MAP.json 构成已保全 D8 候选。fixed5eca/ff10 关闭范围保持；fixed8b6 已独立 CLOSED 最后一个 D8-C98B-P1-03，本 D9 批不重开。
- D9.zh-CN.md、D9-INTERFACES.zh-CN.md、D9-SCHEMAS.zh-CN.md、D9-ACCEPTANCE.zh-CN.md/JSON、D9-IMPACT.zh-CN.md、D9-LEXICON.zh-CN.md、D9-TERMS.json、D9-REGISTRY.json 与 D9-SOURCE-MAP.zh-CN.md/JSON 共同构成完整 D9 作者候选。qualified native-table Office authoring 对 simple `data.native_table.COLUMN` 保持原拼法；fresh qualified/non-ASCII 情况使用可见的 `native.table[...]::column[...]`；internal `nt_/nc_` 仅作为 Plan metadata，不产生另一套作者权威。
- D10.zh-CN.md、D10-CONTROL.zh-CN.md、D10-UPSTREAM.zh-CN.md、D10-SCENARIOS.zh-CN.md、D10-IMPACT.zh-CN.md、D10-LEXICON.zh-CN.md、D10-REVIEW.zh-CN.md、D10-TASK.zh-CN.md、D10-READING.zh-CN.md 是九组完整双语 D10 作者候选；D10-ACCEPTANCE.json 是唯一 125 ID 验收权威，D10-SOURCE-MAP.json 只作导航/历史映射，不建立第二 Registry。
- REVIEW-ENTRY.zh-CN.md 是 D1–D10 非作者复核入口。
- SOURCE-MAP.zh-CN.md 是人类可读的来源与 disposition 图。
- SOURCE-MAP.json 逐条记录 49 个固定 S 输入的 S blob、阅读状态、current 来源以及 source-qualified 义务组。

## 4. 候选内部优先级

对 D1–D10，首次新准备一律遵守本目录 A2 current 候选及真实具名 owner 后像。原 S49 snapshots、docs/design/d10/ R08 原文和真实已保存版本记录只保留来源/历史 decoder 资格，不得覆盖 fresh-current 或冒充已实施。

完整 D10 A2 作者正文现见 [D10](D10.zh-CN.md)，现任 D6 继任类型在 D10-CONTROL 自足列出，125 场景由唯一 D10-ACCEPTANCE/来源映射承担。docs/design/d10/ 旧 R08 候选原文只留来源历史资格，不形成第二 current 合同。

## 5. 接受边界

后续非作者复核绑定 fixed26be（`26be071d4c2075343d9ebf272e00f23769ce0d64`），结论 PASS：P0=0 / P1=0 / P2=0，并 CLOSED 最终 `A2-D7-2F89-P2-01`。此前有界 closure 继续绑定各自原 SHA。D7 因而在 D7 范围 accepted；这不接受 D8 或 global A2。

fixed8d7 已对完整 D9 有界范围独立 ACCEPT 0P0/0P1/0P2 并关闭 D9-6012-P1-01/P2-01；更早 D1–D9 的有界 CLOSED 只按各自固定 SHA 保留。下一步是**基于 PR #5 最终实际 SHA 的全新 D10 非作者全文复核**，随后另做**全新 Pro 完整 A2 非作者总审**。只有固定全局审查 P0/P1=0、所有 P2 明确处置且必要阅读 gap=0，才可能指定 acceptedDesignSha；同 SHA 冻结与真实实施启动包另外执行。全部产品/Office/OS/MCP/外部发送/并发/性能/迁移/激活/部署仍 UNRUN。
