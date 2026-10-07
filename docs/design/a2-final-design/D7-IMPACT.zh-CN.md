---
source_language: zh-CN
translation_status: source
---

[English](D7-IMPACT.md)
# A2 D7 实现影响与验收补充

状态：这是 fixed85bdadf 独立增量复核之后的 D7 作者候选实现影响说明。该复核只留下 A2-D7-2F89-P2-01 OPEN；本作者修订只处理这一 machine-oracle 残余，不自行关闭它，也不声称任何实现或产品行为已经完成。

## 1. 保留的实现范围

完整既有 implementation/test outline 继续逐字节保存在 d7/owners/implementation-impact。代码实现仍只有一套 Core Query DAG、一套 CEL evaluator、结果分页/订阅、纯 View projection、显式 Action prepare/preview/submit、EffectBytes delivery、Definition Transfer、Narrow Field qualification，以及按真实记录版本分派的历史恢复。已经独立关闭的 immutable-source mapping 修复不在本批重开。

## 2. 当前 successor 增量

fresh authoring 使用 QuerySpec/2，外层执行 carrier 继续是 wireVersion2。QuerySpec/1 保持准确的历史/版本 decoder，绝不原地扩宽。QuerySpec/2 只增加 D7-QUERY-V2 规定的版本化 source 与通用 union_all。

实现必须加入 D6 §19 / D6-SCHEMAS §11 定义的文件绑定元数据生产者，由 D6 负责。读取节点文件名、节点路径或资源文件名时，先完成当前引用的披露检查并取得 locator_state 权限，再从同一受保护观察截面的当前 SourceObservation/FileObjectBinding 取值。仅披露位置元数据不要求 source_read/resource_read/structure_state；另行读取正文或资源字节仍须通过原内容读取权限检查。重命名、移动、授权代次或观察状态发生变化时，依赖这些数据的结果必须重置。

fresh QuerySpec/2 的 title 是 Optional<text>。Temporal Calendar/Timeline 与 link-display consumer 必须使用 D7-QUERY-V2 中明确的 Optional 处理；filename/path 永远不能补造 title，本地化“无标题”只能是 presentation-only placeholder。union_all 必须实现带 inputOrdinal 的准确 internal K constructor，保留重复 input branch，继承既有 checked result/work/byte budget，并在 take/paging 顺序具有语义之前要求显式 sort。

fixed97 的当前准备与效果主链继续保留 PAB4/Descriptor3/Proof3/PreparedIntent3、Action 准备请求与输入的第 3 版、ActionSpec2、D7ProposedInput3、EffectManifest3/EffectBytes3、D3 wire13、Notice3/CP4/ChangeRecord1、当前 D2DocumentSnapshot/3 以及 Value4 批注。这些名称分别指向原有的准备绑定、描述符、依赖证明、准备意图、动作输入、效果运输、身份请求、安装记录、文档快照和批注类型；各环节继续使用其真实版本。不得在旧版本标签下扩宽任何前代解码器。

## 3. Search compiler 与 machine-oracle 边界

普通文本、visual condition 与显式 shortcut mode 都生成同一 SearchConditionAst/1，并走唯一 QuerySpec/2 compiler。D7-SEARCH-FIXTURES.json 是设计 machine oracle：每个正例实际保存 shortcut AST、与具体 UI widget 无关的 visual input/AST、canonical AST、非 wire 的 compiler-review metadata、可按 QuerySpec/2 严格解码的真实对象，以及带 canonical CEL AST/reference ordinal/field order 的规范 §5 CanonicalGraph serialization；negative、incomplete 与 browse case 明确不生成 Query。Node/Resource 混合 OR oracle 具有真实的逐 domain CEL 专门化、同一 public schema、真实 union_all input order 与最终 sort/project。

实现与 conformance 必须覆盖递归 precedence、adjacency、repeated NOT、准确 keyword boundary、FieldId/member-path grammar、修正后的单反斜线 Windows 与 escaped-@ 例、bare trailing backslash 的字面规则、quoted trailing escape 的 incomplete 分类、三个 preset 的空输入、显式 source override、Optional title 处理、Node/Resource OR 经 union_all、domain 不相容 AND 的拒绝，以及 QuerySpec/1 与 /2 的严格分派。

这些 machine oracle 不是“产品 parser/compiler 已运行”的证据。仓库/设计检查可以严格检查 fixture shape 并机械重算 §5 relation graph，但后续产品测试仍必须真正运行 parser、QuerySpec decoder 与 CEL compiler，并把其 canonical AST/Query graph 与这些设计 oracle 比较。

## 4. 保存/复制/导出 owner 边界

saved definition 的 copy/fork/import 继续由 D7 Definition Transfer 加普通 D3 author path 完成，按版本解码 QuerySpec 并保留 embedded bytes。D9 query_json 只表示完整 terminal D7 result 的 weftext.query-result-export/1，保留 TerminalSchema/data/bag/order；它不是 QuerySpec author source、saved-definition migration 格式或第二 Query 权威。

## 5. 机械与语义检查

仓库检查必须覆盖中英配对文档、受保护设计输入、JSON 有效性、D7 machine-map 引用、fixed-S 34/8 与 parent-current 34/13 Registry qualification，以及 S 输入逐字不变。direct design regression 还必须分别检查 D6 metadata 授权顺序和“无 content read 的正例”、QuerySpec/2 Optional-title consumer、union_all K/order/budget、Definition Transfer 版本分派、D9 query_json 边界、shortcut visual/compiler oracle，以及本批没有改动主体语义的 PAB4/Effect3/Narrow Field/View 合同。

## 6. 产品证据边界

runtime、OS、GUI、IME、辅助技术、renderer/export、真实 QuerySpec/2 parser/compiler、真实 metadata producer、index/provider、database/replica/race、performance、migration 与 activation 场景继续全部 UNRUN。documentation、machine 与 source CI 只证明仓库一致性，不能关闭 D7 语义 finding。

## 7. 后续 gate

下一 gate 只对 A2-D7-2F89-P2-01 与本次直接变化的 Search fixture/oracle surface 做 fixed-SHA 非作者增量复核。A2-D7-2F89-P1-01、A2-D7-2F89-P1-02 与 A2-D7-1068244-P2-01 继续保持 fixed85bdadf 的 independently CLOSED；A2-D7-2F89-P2-02 继续保持 fixed1068244 的 independently CLOSED。D8-D10 完整模块、Search+D8 平台执行证据与后续 fresh Pro/global review 继续分开。
