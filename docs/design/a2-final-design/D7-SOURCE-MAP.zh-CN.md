---
source_language: zh-CN
translation_status: source
---

[English](D7-SOURCE-MAP.md)
# A2 D7 来源与案例映射

状态：这是 fixed1068244 之后的作者残余修复。immutable-source trace P2-02 已独立 CLOSED；剩余 2 个 P1 与 2 个 P2 在此完成作者修复，但仍保持 OPEN 等待独立复核。

## 1. 固定对象与既有关闭

本修复起点是 2f89a55cb1f924a47281f59e6419fff7c0c206ed。base 继续是 97f4734f82a760cb6716c8122b84494da2b61164，fixed S 继续是 7e18168dad3e6d120fce0dd607dc10fa7894e252，受保护 inputs blob 继续是 787d03c31a55496f81ed03fd54a6fdfff50a2ad4。

fixed2f89 已在有界范围独立关闭 A2-D6-01CC-P2-01、A2-NAV-454E-P2-01 与 COORD-D7-DOC-QUALITY-01；fixed1068244 又独立 CLOSED A2-D7-2F89-P2-02。当前残余是两个 P1、A2-D7-2F89-P2-01 与 A2-D7-1068244-P2-01；本作者批修复它们，但状态仍是 OPEN pending independent review。

## 2. 阅读覆盖

fixed-S D7 十三份来源以及额外 D9 coordinated D3-D7 binding source，继续沿用已完成 D7 复核的完整阅读证据。current 双语 D7 owner set 也继续保持完整阅读。D1/D2/D4/D5 与 D7 所需 D6 交叉沿用此前记录的覆盖；D3/D8/D9/D10 仍只覆盖具名的 D7 直接生产者/消费者交叉，本批没有重新审查这些完整模块。

本修复只重读四项 finding 的直接证据：fixed-S 与 parent-current 术语 Registry、闭合的 Query read/source grammar、current D2 title/subtitle 产品、D6 SourceObservation/FileObjectBinding/Policy 门、immutable Mandatory/scenario source range，以及准确的 current source-map package。

## 3. Registry 资格

immutable fixed-S Registry 实际是 34 个 concept、8 条 cross-stage binding。parent-current Registry 与 D7-REGISTRY.json 共用同一个 blob 3cab5124f46822729093d5d955908b05eb65bb87，实际是 34 个 concept、13 条 binding。

D7-REGISTRY-QUALIFICATION 对 B01-B13 逐条映射。D7-REGISTRY.json 继续是唯一 parent-current machine Registry，不被改写。A2 只在 fixed97/current owner 真实改变时具名 fresh successor；未变化的 inner version 保持不变，真实 historical record 保留原 decoder。

## 4. Query 与 Search 残余修复

D7-QUERY-V2 继续以 QuerySpec/2 作为当前作者 successor，但 FileBinding metadata 改为消费 D6-owned producer，并由 entity_state+locator_state 授权，而不是要求 whole content read 或 structure_state。它还闭合 Optional-title 直接 consumer、准确 union_all LogicalOccurrenceKey constructor 与 D9 query_json 的 result-only 边界。

D7-SEARCH 冻结 trailing backslash 的唯一分类；D7-SEARCH-FIXTURES.json 现在实际包含 shortcut/visual/canonical AST 以及完整 QuerySpec/2 与 CanonicalGraph 审查描述。三个 preset 空输入、source override、Node/Resource 混合 OR、Optional handling 与 /1-/2 严格版本边界都进入 machine oracle。D7-IMPACT 中英文同步到同一组义务。

## 5. Immutable source trace

D7-SOURCE-MAP.json 现使用第二版机器结构，不再把 current mirror 摘要称为 original condition。20 项 D7 整合义务逐项保存不可变 fixed-S 的来源标识、路径、blob、章节和行范围，同时给出准确的 current target、处置状态、owner 以及正负验收 oracle。

原 151 个 case row 中，148 个继续作为 source-qualified case，并直接记录 immutable fixed-S 或 Mandatory source slice。旧三个 Chart mirror row CHART-L194/L196/L198 只保留为 navigation summary。Mandatory Chart §15 另拆为 17 个连续 obligation group，覆盖 §15.1 至 §15.8.9，包括 §15.7 全部 10 个 fixtures 与 §15.8.9 两个 hard questions。

retained current scenario mirror 仍可用于 current disposition 导航，但绝不能替代 immutable source text。

## 6. 当前 finding 状态

- A2-D7-2F89-P1-01：作者残余修复已存在；继续 OPEN，等待独立复核。
- A2-D7-2F89-P1-02：作者残余修复位于 D6 §19/D6-SCHEMAS §11 与 D7-QUERY-V2/D7-SCHEMAS/D7-SEARCH；继续 OPEN。
- A2-D7-2F89-P2-01：作者残余修复位于 D7-SEARCH 与 D7-SEARCH-FIXTURES.json；继续 OPEN。
- A2-D7-2F89-P2-02：已在 fixed1068244 独立 CLOSED；本批不重开 immutable mapping rows/groups。
- A2-D7-1068244-P2-01：作者修复位于同步后的 D7-IMPACT 中英文；继续 OPEN。

作者没有自行关闭当前四项残余 finding。

## 7. 证据边界与下一 gate

仓库文档检查、输入完整性检查、JSON 与 source-map 检查只能证明机械一致性。Product Search、runtime、Desktop/Mobile/WebUI GUI、IME/AT、renderer/export、真实 index/provider、CAS/race、migration、deployment 与历史执行全部继续记为 UNRUN。

下一 gate 只对 A2-D7-2F89-P1-01、P1-02、P2-01、A2-D7-1068244-P2-01 及其直接回归面做 fixed-SHA 非作者增量复核。已经关闭的 P2-02 只作为既有证据保留。D8-D10 完整整合与后续 fresh Pro/global A2 review 继续分开。
