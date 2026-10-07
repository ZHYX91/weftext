---
source_language: zh-CN
translation_status: source
---

[English](D7-SOURCE-MAP.md)
# A2 D7 来源与案例映射

状态：这是 fixed2f89 完整 D7 复核之后的作者修复。四项 fixed2f89 finding 已有作者修复，但仍保持 OPEN，等待独立复核。

## 1. 固定对象与既有关闭

本修复起点是 2f89a55cb1f924a47281f59e6419fff7c0c206ed。base 继续是 97f4734f82a760cb6716c8122b84494da2b61164，fixed S 继续是 7e18168dad3e6d120fce0dd607dc10fa7894e252，受保护 inputs blob 继续是 787d03c31a55496f81ed03fd54a6fdfff50a2ad4。

fixed2f89 已在有界范围独立关闭 A2-D6-01CC-P2-01、A2-NAV-454E-P2-01 与 COORD-D7-DOC-QUALITY-01；它们不再是当前 pending finding。当前四项 D7 finding 在 D7-SOURCE-MAP.json 中继续标为 OPEN，只注明作者修复已存在。

## 2. 阅读覆盖

fixed-S D7 十三份来源以及额外 D9 coordinated D3-D7 binding source，继续沿用已完成 D7 复核的完整阅读证据。current 双语 D7 owner set 也继续保持完整阅读。D1/D2/D4/D5 与 D7 所需 D6 交叉沿用此前记录的覆盖；D3/D8/D9/D10 仍只覆盖具名的 D7 直接生产者/消费者交叉，本批没有重新审查这些完整模块。

本修复只重读四项 finding 的直接证据：fixed-S 与 parent-current 术语 Registry、闭合的 Query read/source grammar、current D2 title/subtitle 产品、D6 SourceObservation/FileObjectBinding/Policy 门、immutable Mandatory/scenario source range，以及准确的 current source-map package。

## 3. Registry 资格

immutable fixed-S Registry 实际是 34 个 concept、8 条 cross-stage binding。parent-current Registry 与 D7-REGISTRY.json 共用同一个 blob 3cab5124f46822729093d5d955908b05eb65bb87，实际是 34 个 concept、13 条 binding。

D7-REGISTRY-QUALIFICATION 对 B01-B13 逐条映射。D7-REGISTRY.json 继续是唯一 parent-current machine Registry，不被改写。A2 只在 fixed97/current owner 真实改变时具名 fresh successor；未变化的 inner version 保持不变，真实 historical record 保留原 decoder。

## 4. Query 与 Search 修复

D7-QUERY-V2 将 QuerySpec/2 定义为当前新建作者 schema，QuerySpec/1 则继续由单独的准确 decoder 解释。QuerySpec/2 增加可空的 current title/subtitle、获权的 Node 文件名与路径、获权的 Resource 文件名，以及要求输入 schema 完全相等的通用 union_all。所有 read 都绑定真实 D2/D6 producer、授权结果、同一 cut 的 current Observation/FileBinding 和最终 barrier。

D7-SEARCH 现在完整规定快捷模式的 lexer、递归优先级 grammar、关键字边界、转义规则、FieldId/member-path 验证、三个 preset 的空输入行为，以及显式 source override。可视化条件和 shortcut 生成同一份临时 SearchConditionAst/1，并确定性编译到 QuerySpec/2。D7-SEARCH-FIXTURES.json 保存机器可核的正反等价 fixture；任何 parser error 都不能退回另一条 Query。

## 5. Immutable source trace

D7-SOURCE-MAP.json schema version 2 不再把 current mirror 摘要称为 original condition。20 项 D7 integration obligation 均记录 immutable fixed-S sourceId/path/blob/section/range、precise current target、disposition、owner 与 positive/negative oracle。

原 151 个 case row 中，148 个继续作为 source-qualified case，并直接记录 immutable fixed-S 或 Mandatory source slice。旧三个 Chart mirror row CHART-L194/L196/L198 只保留为 navigation summary。Mandatory Chart §15 另拆为 17 个连续 obligation group，覆盖 §15.1 至 §15.8.9，包括 §15.7 全部 10 个 fixtures 与 §15.8.9 两个 hard questions。

retained current scenario mirror 仍可用于 current disposition 导航，但绝不能替代 immutable source text。

## 6. 当前 finding 状态

- A2-D7-2F89-P1-01：作者修复位于 D7 §20、D7-REGISTRY-QUALIFICATION 与 machine bindingQualification；继续 OPEN，等待独立复核。
- A2-D7-2F89-P1-02：作者修复位于 D7-QUERY-V2、D7-SCHEMAS 与 D7-SEARCH；继续 OPEN，等待独立复核。
- A2-D7-2F89-P2-01：作者修复位于 D7-SEARCH 与 D7-SEARCH-FIXTURES.json；继续 OPEN，等待独立复核。
- A2-D7-2F89-P2-02：作者修复位于 D7-SOURCE-MAP.json 的 immutable range/case/chart group；继续 OPEN，等待独立复核。

作者没有自行关闭这四项。

## 7. 证据边界与下一 gate

仓库文档检查、输入完整性检查、JSON 与 source-map 检查只能证明机械一致性。Product Search、runtime、Desktop/Mobile/WebUI GUI、IME/AT、renderer/export、真实 index/provider、CAS/race、migration、deployment 与历史执行全部继续记为 UNRUN。

下一 gate 是对四项 fixed2f89 finding 及其受影响 direct surface 做 fixed-SHA 非作者增量复核。D8-D10 完整整合与后续 fresh Pro/global A2 review 继续分开。
