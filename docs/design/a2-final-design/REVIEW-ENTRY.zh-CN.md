---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D10 作者候选复核入口——完整 D10 交回

fixed8d7 非作者完整 D9 有界审查已 ACCEPT 0P0/0P1/0P2、必要阅读 gap=0，并独立 CLOSED D9-6012-P1-01/P2-01；更早 fixed6012/fixed466b 及其它所有 bounded CLOSED 维持。此结论绝不接受 D10 或全球 A2。完整 D10 九组双语作者候选已整合，状态仅 author-resolved-pending-independent-review，全部产品测试 UNRUN。下一步先完整 D10 非作者，再全新 Pro 完整 A2 独立审查。

## 1. 固定对象与作者时间线

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。
D9 作者整合起点 SHA：5eca16c40cdf2e1892f6930d51c632ea720a460c。

D6 finding 来源是 fixed829 `829efce6aacbe944714e093c98065b01d50b2593`。第一次非作者修订复核是 fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`。四残余作者修订从 `32cfb9c387deddb12fb021a44147df0d7ffab322` 开始；后续窄修经过 `d1ab2a5c0762450877614ad14340a26cfa0c1dfd`。下一轮独立复核把完成候选固定在 `1fc4a3937bbc06b4785d39f1eaf498f3c9c39abc`。此前两项 P2 修订从该 head 开始。语义关闭复核固定 5e21e9f00e1fa4e893f2544211133adbeac35ae0 并关闭语义 P2。fixed454e 随后新开两项 navigation-only P2。之后 fixed2f89 完整 D7 独立复核在有界范围关闭这两项 navigation finding 与 document-quality coordination finding，同时新开四项 D7 finding。本修复批精确从 fixed2f89 开始，只处理这四项，作者不自行关闭。

下一位非作者必须固定 PR #5 最终实际 SHA，独立全文复核 D10 原始十八文件、现任九组双语、固定 S49、原 125 场景、全部当前直接生产者与消费者；其后另开展全新 Pro 完整 A2 独立审查。不追 moving branch，也不采用 synthetic merge SHA。

## 2. 保留的独立状态

- 有界 D1/D2 findings 继续 independently CLOSED。
- 有界 D3 P1-01/P1-02 继续在 fixed-a62 independently CLOSED。
- A2-D4D5:P2-02 继续只在 fixed4282 的六份真实 D10 来源资格有界范围 independently CLOSED。
- A2-D4D5:P2-01 现为 bounded CLOSED：fixed32cf 已 CLOSED R2 并通过 188/202 条 R1；fixed1fc4 独立复核通过全部 14 条残余映射。
- D6 `A2-D6-829-P1-01` 与 `A2-D6-829-P2-03` 继续保留 fixed01cc 的 independently CLOSED。
- fixed1fc4 独立复核又 CLOSED D6 `A2-D6-829-P1-02` 与 `A2-D6-829-P2-02`。

上述有界结论都不构成 D6 或全局 A2 接受。

## 3. 保留的语义修订与当前导航修订

### P2-01

`A2-D6-829-P2-01` 修的是语义 disposition，不是来源清单本身。299 个 section identity、760 条 fixed97 case、来源字节、双语条件、Registry pointer inventory、parent Impact record 与受保护 inputs 全部保持。

全部 47 个旧 `retain-or-named-current-successor` group 都按义务重新审计。机器 map 现在区分纯 retained、纯历史状态/provenance、保留但未执行的测试义务，以及 `split-by-sub-obligation` group。每个 split group 都记录行范围、disposition、current owner/anchor 与可独立判断的 oracle。尤其 fixed-S Storage 第 20、28–30、36–46、52 行不再把“当前正文存在 SQLite、只能 checkout 编辑、正文与控制共置一个数据库”冒充 current 要求；current D6 §1–§3 明确使用 F/M/P/I/D 的 file-backed authority。真正保留的是 one writable truth、无第二套 Field/relation 作者权威、完整源可取回、精确 identity 边界、continuity/fencing，以及禁止 LWW shortcut 等业务不变量。

另有 3 个紧邻 fixed-S cross-owner group 存在相同旧版本歧义，也按同一原则拆分：旧 wire/Policy 数字只作历史限定，跨 owner 的真实业务义务继续明确保留。本次不重开已关闭的 Key3/Registry 或 D4/D5 findings。

### fixed2f89 独立复核状态

fixed2f89 独立复核已在有界范围 CLOSED `A2-D6-01CC-P2-01`、`A2-NAV-454E-P2-01` 与 `COORD-D7-DOC-QUALITY-01`；它们不再是当前 pending finding。

fixed1068244 后续独立 CLOSED A2-D7-2F89-P2-02，同时保留 P1-01、P1-02、P2-01 为 OPEN，并新增 D7-IMPACT 中英漂移 finding A2-D7-1068244-P2-01。下一轮 fixed85bdadf 独立复核又 CLOSED P1-01、P1-02 与 Impact-sync P2，只留下 P2-01 OPEN。本作者批精确从 fixed85bdadf 开始，只修该 QuerySpec/CanonicalGraph oracle 残余，且不自行关闭。

历史时间线为 fixed829 来源 → fixed01cc → fixed32cf/d1ab → fixed1fc4 → fixed5e21 → fixed9c3b/fixed454e → fixed2f89 完整 D7 复核 → fixed1068244 增量复核 → fixed85bdadf 单残余复核。较早裁决只描述各自 fixed object。

## 4. 证据与非声明边界

本修订 FULL 仅限 D6 source-map 语义 disposition/navigation surface，以及判断它们所需、已经登记的 fixed/current owner evidence。fixed-S snapshots 与 `docs/design/inputs.json` 受保护且保持不变。

原两项 D9 P1 及 fixed8b6/fixed4da7 旧 P2 已在后续各自有界复核中独立 CLOSED。当前仅新增 fixed6012 的 P1/P2 两项由作者修补、待独审；ROOT-D9-ZH-P2-01 在 ff10 保持 CLOSED。本历史整合段落不重开旧 finding。

产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 行为证据全部 UNRUN。文档、机器与 CI 检查只证明仓库一致性，不替代语义接受。最终 A2 仍需后续完整整合、fresh Pro/global 非作者终审、一个 explicit accepted-design SHA，以及同一 accepted SHA 上的 freeze/implementation-start 材料。


## D8 复核目标

D8-C98B-P1-03 已在 fixed8b6 独立 CLOSED，其静态解码、高级路径与 zero-total 有界结论不因新 D9 修复而重开。fixed5eca/ff10 的其它 D8 关闭结果保持；D8 产品/runtime 证据仍 UNRUN。

## D9 复核目标

历史 fixed6012→fixed8d7 D9 复核已 ACCEPT 并 CLOSED 两项 D9-6012 新回归；下列 D9 checklist 只保留原有界证据与恢复条件，不再是当前 OPEN 项。本轮现任审查目标是完整 D10 与真实 A2 D1–D9 直接消费者，其后另做全新 Pro A2 总审。

必须核验：

- D9-BAF-P1-01（fixed79026 已独立 CLOSED，仅保留上下文）：核唯一 Annotation Value4/PortableRecord4/R6 carrier、独立 context 披露、catalog index/mode/projection 精确一一对应及被选中 annotation_content payload.recordPin 的逐字校验、跨输入域排他、Annotation Plan 禁选普通 Document body、review/backup 目标不兼容、重复或未选 projection 拒绝、pin/currentness、freeze→loss→confirm→create-only/Resource/print 路径；
- D9-BAF-P1-02 / D9-466B-P1-01 与 D9-466B-P2-01 已在 fixed6012 独立 CLOSED；fixed8d7 进一步 CLOSED D9-6012-P1-01/P2-01，所有旧 finding 仅作历史来源，不重新裁决。
- D9-8B6-P2-01 / D9-8B6-P2-02（fixed4da7 有界范围独立 CLOSED，只保留上下文，不是新增复核目标）：保持 Annotation selection/projection 集合、保留原序的 disclosure fragment、canonical origins、renderer asset/evidence-pin 数组，以及合法 D7 network 在当前六布局 D9 路线必定 renderer_unavailable；不得把 graph query_json 降成 rows，也不得丢实际 D7 nodes/edges 列/V/顺序；network ViewSpec.nodeDetails 是可选展示绑定，不是 graph TerminalSchema/data 的必需成员。D9-BAF-P2-02 的来源范围独立 CLOSED 结论仍固定在 fixed8b6；
- ROOT-8B6-MAP-P2-01（fixed4da7 独立 CLOSED，本批不重复裁决）：保留 D8-SOURCE-MAP.json 中现任 consumer ACCEPTANCE.md path/blob 与原 760 条来源行的 source blob 不同角色。另核 D9-SOURCE-MAP.json 的当前来源资格、D9-REGISTRY.json、D9-TERMS.json 与全部 104 个唯一 D9-ACCEPTANCE row 及中英文投影；
- 单一 Core 解析器/身份/授权边界、ImportIR/Mapping/Loss/ConversionInput/ImportJob 闭合、D4 Registry/Field admission，以及禁止建立第二套 identity/ledger/patch authority；
- Node Template Recipe/ConstructionInput、sourceSubjectBindings 与原 receipt 的关联、PAB4/MinimumMapping/现任 wire13，以及 saved/planned/unknown 的恢复规则；
- 普通 Office token/style/repeat 规则，以及可见的 qualified native-table selector grammar：template bytes 必须自包含，nt_/nc_ 只作内部 Plan key，并采用最短唯一 suffix/title/occurrence、同一 rowset 的 repeat 与 Unicode escape；
- fresh Plan4/Catalog3/Selection2/Projection2/Loss2/Confirmation2/Receipt4/PrintReceipt1 封闭 family，并精确恢复 Plan/Receipt1–3；FC SPEC §8.3 的 exact Source/Resource/query_json generationPolicy=none、受控名称与 canonical ordering、template/route/style、递归 evidencePins、unknown-publication 规则都必须由 /4 继承；SPEC §17 unseen-current 分派、§18 inventory、SCHEMAS §§6.5–6.6、terminology、acceptance 与 replacement router 必须一致说明真实 /3 只作 recovery；D2Snapshot3/D8 呈现、confirmation 不可变、create-only 发布与独立 Resource author receipt 保持不变；
- current PortableAnnotationRecord/4 / Value4 / 唯一 R6 AnnotationInlineProfile consumer 边界保持不变；annotation_index 绝不能替代新的内容 carrier；
- worker/route/sandbox、D9 默认不联网、image/region、Mobile negative conversion surface，以及与 D10 直接相交但不等于完整 D10 的边界。
- historical 57/63/59/82/90/130/12 evidence set 分开记账；I01–I12 及产品/runtime/GUI/Office/OS/performance/deployment 全部继续 UNRUN。

历史 D9 独立复核必须报告 P0/P1/P2 与必要读取 gap；此前 D9 作者待审条件已仅在 D9 范围由 fixed8d7 非作者 ACCEPT 0P0/0P1/0P2、必要 gap=0 满足。当前 D10 全文已整合为作者候选，但完整 D10 非作者复核及另一轮全新 Pro/global 完整 A2 总审仍必须另行执行；D9 verdict 和绿色 docs 均不构成全局接受。

## 6. 现任 D10 完整非作者审查交接

必须冻结 Draft PR #5 **真实最终 HEAD**，不能使用 synthetic merge ref。范围：原 D10 九组双语 18 份完整文件、固定 S49（包括 D4 61 Field/7 Facet）、Mandatory §§1–15、原 125 项 D10 场景及全部未编号约束、现任 A2 D1–D9 与 D6/FC 直接 owner。当前九组完整双语文件为 D10、D10-CONTROL、D10-UPSTREAM、D10-LEXICON、D10-SCENARIOS、D10-IMPACT、D10-REVIEW、D10-TASK、D10-READING。唯一 D10-ACCEPTANCE.json 及双语投影是结构验收权威；D10-SOURCE-MAP.json 是来源路由，不建立新 Registry。[现任 D10](D10.zh-CN.md) 与 [D10 来源映射](D10-SOURCE-MAP.json) 只是导航，不是接受证据。

新非作者必须核原 CONTROL §§1–16.2、CANDIDATE §§1–25 全部 inline/未编号/封闭成员，全部场景/race、ToolValue 有限代数、精确 Principal/auth/egress/secret/Money/stop 限制，D6 唯一 P 与受保护 I/派生 D；十九类可读 current K 投影；D6 mixed Binding3/Deps3/Input2/Image2+Image1/Inventory2/ResponsibilityRecord3/Schedule2/ApprovalUse2、D7 PAB4/Effect3 和真实历史 decoder；Package/Contribution/Pack、四官方 module、D4 Registry/SearchContribution 纯数据边界、D9 Plan4/Annotation/View/print 以及真实 Plan/Receipt1–3。P19 旧 17-kind/36+1 是历史来源清单，不是 current 上限，不新造 Resolver12 或第二 owner。

重点负面是 U12 的 ICS、普通 fresh 导入、真正 D3 SourceBinding/Adopt 分轴，同 Run maxRuns 恢复，standing approval 精确 raw-no-op、planned-preview 只读恢复，不可逆 stop/fence/unknown 与真实费用责任，schedule A→B→A/gap、D1 五端及不可信模型/外部正文；证明 D2 AsciiDoc、D7 Query/View、D8 Source/Write/Read/IME/Annotation、D9 Office/query_json/Source/Resource 正向路径不会因无关 D10 provider 退化。原 R08 十一项及有界独审须单独分账，不能作者自关。

当前 D10 仅 **author-resolved-pending-independent-review**，尚未独立接受或激活。完整 D10 非作者须报告 P0/P1/P2 和必要阅读 gap；其后独立开展**全新 Pro 完整 A2 固定 SHA 总审**，P0/P1=0、全部 P2 有明确处置、必要 gap=0 才能指定 acceptedDesignSha。同 SHA 冻结与真实实施启动包仍另行执行。产品/Core/OS/IME/AT/Office/MCP/model/真实外部发送/D6 CAS/race/performance/migration/activation/deployment 均 UNRUN。
