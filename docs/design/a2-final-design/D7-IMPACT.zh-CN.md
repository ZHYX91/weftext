---
source_language: zh-CN
translation_status: source
---

[English](D7-IMPACT.md)
# A2 D7 实现影响与验收补充

状态：这是 fixed1068244 增量独审之后的 D7 作者候选实现影响说明；不声称任何实现或产品行为已经完成。

## 1. 保留的实现范围

完整既有 implementation/test outline 继续逐字节保存在 d7/owners/implementation-impact。代码实现仍只有一套 Core Query DAG、一套 CEL evaluator、结果分页/订阅、纯 View projection、显式 Action prepare/preview/submit、EffectBytes delivery、Definition Transfer、Narrow Field qualification，以及按真实记录版本分派的历史恢复。已经独立关闭的 immutable-source mapping 修复不在本批重开。

## 2. 当前 successor 增量

fresh authoring 使用 QuerySpec/2，外层执行 carrier 继续是 wireVersion2。QuerySpec/1 保持准确的历史/版本 decoder，绝不原地扩宽。QuerySpec/2 只增加 D7-QUERY-V2 规定的版本化 source 与通用 union_all。

实现必须加入 D6 §19 / D6-SCHEMAS §11 的 D6-owned FileBinding metadata producer。Node basename/path 与 Resource basename 先通过当前 Ref disclosure 和 locator_state，在一个受保护 current SourceObservation/FileObjectBinding cut 上取值；仅为 locator metadata 时不要求 source_read/resource_read/structure_state。另行读取 body 或 Resource bytes 时仍走原 content gate。rename/move、auth generation 或 observation 变化都会 reset 依赖结果。

fresh QuerySpec/2 的 title 是 Optional<text>。Temporal Calendar/Timeline 与 link-display consumer 必须使用 D7-QUERY-V2 中明确的 Optional 处理；filename/path 永远不能补造 title，本地化“无标题”只能是 presentation-only placeholder。union_all 必须实现带 inputOrdinal 的准确 internal K constructor，保留重复 input branch，继承既有 checked result/work/byte budget，并在 take/paging 顺序具有语义之前要求显式 sort。

fixed97 的当前 prepare/effect 主链继续是 PAB4/Descriptor3/Proof3/PreparedIntent3、Action prepare/input version 3、ActionSpec2、D7ProposedInput3、EffectManifest3/EffectBytes3、D3 wire13、Notice3/CP4/ChangeRecord1、current D2DocumentSnapshot/3 与 Value4 Annotation。任何 predecessor decoder 都不原地扩宽。

## 3. Search compiler 与 machine-oracle 边界

普通文本、visual condition 与显式 shortcut mode 都生成同一 SearchConditionAst/1，并走唯一 QuerySpec/2 compiler。D7-SEARCH-FIXTURES.json 是设计 machine oracle：每个正例实际保存 shortcut AST、与具体 UI widget 无关的 visual input/AST、canonical AST、完整可审查的 QuerySpec/2 canonical description 与完整结构化 CanonicalGraph description；negative、incomplete 与 browse case 明确不生成 Query。

实现与 conformance 必须覆盖递归 precedence、adjacency、repeated NOT、准确 keyword boundary、FieldId/member-path grammar、修正后的单反斜线 Windows 与 escaped-@ 例、bare trailing backslash 的字面规则、quoted trailing escape 的 incomplete 分类、三个 preset 的空输入、显式 source override、Optional title 处理、Node/Resource OR 经 union_all、domain 不相容 AND 的拒绝，以及 QuerySpec/1 与 /2 的严格分派。

这些 machine oracle 不是“产品 parser/compiler 已运行”的证据。后续产品测试必须真正运行 parser/compiler，并把其 canonical AST/Query graph 与这些设计 oracle 比较。

## 4. 保存/复制/导出 owner 边界

saved definition 的 copy/fork/import 继续由 D7 Definition Transfer 加普通 D3 author path 完成，按版本解码 QuerySpec 并保留 embedded bytes。D9 query_json 只表示完整 terminal D7 result 的 weftext.query-result-export/1，保留 TerminalSchema/data/bag/order；它不是 QuerySpec author source、saved-definition migration 格式或第二 Query 权威。

## 5. 机械与语义检查

仓库检查必须覆盖中英配对文档、受保护设计输入、JSON 有效性、D7 machine-map 引用、fixed-S 34/8 与 parent-current 34/13 Registry qualification，以及 S 输入逐字不变。direct design regression 还必须分别检查 D6 metadata 授权顺序和“无 content read 的正例”、QuerySpec/2 Optional-title consumer、union_all K/order/budget、Definition Transfer 版本分派、D9 query_json 边界、shortcut visual/compiler oracle，以及本批没有改动主体语义的 PAB4/Effect3/Narrow Field/View 合同。

## 6. 产品证据边界

runtime、OS、GUI、IME、辅助技术、renderer/export、真实 QuerySpec/2 parser/compiler、真实 metadata producer、index/provider、database/replica/race、performance、migration 与 activation 场景继续全部 UNRUN。documentation、machine 与 source CI 只证明仓库一致性，不能关闭 D7 语义 finding。

## 7. 后续 gate

下一 gate 是对 fixed1068244 独审仍剩的 2 个 P1 与 2 个 P2，以及它们直接影响的 regression surface 做 fixed-SHA 非作者增量复核。A2-D7-2F89-P2-02 已在 fixed1068244 独立 CLOSED，本批不把它重新列成 OPEN。D8-D10 完整模块、Search+D8 平台执行证据与后续 fresh Pro/global review 继续分开。
