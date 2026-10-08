---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D9 作者候选复核入口——D9 完整整合

D7 在 fixed26be 的 D7 范围独立 PASS。fixed5eca/ff10 的 D8 关闭结果保持；fixed8b6 已独立 CLOSED 最后一个 D8-C98B-P1-03 与 D9-BAF-P2-02。ROOT-D9-ZH-P2-01 保持 ff10 独立 CLOSED。本 D9 作者批保全八份 fixed-S/current D9 整合，修复 D9-BAF-P1-01/P1-02、D9-8B6-P2-01/P2-02，以及另外记录的 ROOT-8B6-MAP-P2-01 路径/blob 不匹配。新修复均等待绑定最终 SHA 的非作者复核，不宣称 D9/global 接受或产品执行；完整 D10 与全新 Pro/global 终审留后续。

## 1. 固定对象与作者时间线

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。
D9 作者整合起点 SHA：5eca16c40cdf2e1892f6930d51c632ea720a460c。

D6 finding 来源是 fixed829 `829efce6aacbe944714e093c98065b01d50b2593`。第一次非作者修订复核是 fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`。四残余作者修订从 `32cfb9c387deddb12fb021a44147df0d7ffab322` 开始；后续窄修经过 `d1ab2a5c0762450877614ad14340a26cfa0c1dfd`。下一轮独立复核把完成候选固定在 `1fc4a3937bbc06b4785d39f1eaf498f3c9c39abc`。此前两项 P2 修订从该 head 开始。语义关闭复核固定 5e21e9f00e1fa4e893f2544211133adbeac35ae0 并关闭语义 P2。fixed454e 随后新开两项 navigation-only P2。之后 fixed2f89 完整 D7 独立复核在有界范围关闭这两项 navigation finding 与 document-quality coordination finding，同时新开四项 D7 finding。本修复批精确从 fixed2f89 开始，只处理这四项，作者不自行关闭。

下一位非作者复核者必须绑定 PR metadata/交接记录的实际 final stop SHA，不得追 moving branch。

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

D7 在 fixed26be 独立 PASS；最后一个 D8 P1 与 D9-BAF-P2-02 都已在 fixed8b6 独立 CLOSED。当前 D9 设计保留精确的 Annotation Value4/R6 carrier、六图表 complete_data 导出、原 D7/FC owner 和记录型 Plan/Receipt1–3 恢复。两个 D9 P1 残余与两个 fixed8b6 P2 仅作者已修、待独审，另有单独的 root mapping 事项；ROOT-D9-ZH-P2-01 在 ff10 已 CLOSED。D10 全文与 A2 全局终审均未完成。

产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 行为证据全部 UNRUN。文档、机器与 CI 检查只证明仓库一致性，不替代语义接受。最终 A2 仍需后续完整整合、fresh Pro/global 非作者终审、一个 explicit accepted-design SHA，以及同一 accepted SHA 上的 freeze/implementation-start 材料。


## D8 复核目标

D8-C98B-P1-03 已在 fixed8b6 独立 CLOSED，其静态解码、高级路径与 zero-total 有界结论不因新 D9 修复而重开。fixed5eca/ff10 的其它 D8 关闭结果保持；D8 产品/runtime 证据仍 UNRUN。

## D9 复核目标

下一位非作者 reviewer 必须固定 PR #5 的实际最终 SHA，增量复核 D9-BAF-P1-01、D9-BAF-P1-02、D9-8B6-P2-01、D9-8B6-P2-02，并单独核 ROOT-8B6-MAP-P2-01 的映射。D9-BAF-P2-02 在 fixed8b6 已 CLOSED，ROOT-D9-ZH-P2-01 在 ff10 已 CLOSED；本次只核新改的中英文、跨域选择、mode/target/receipt 关系，不重开已接受 D8 范围或历史 Plan/Receipt decoder。绿色 docs 与数量不等于语义接受。

必须核验：

- D9-BAF-P1-01：核唯一 Annotation Value4/PortableRecord4/R6 carrier、独立 context 披露、catalog index/mode/projection 精确一一对应、跨输入域排他、Annotation Plan 禁选普通 Document body、review/backup 目标不兼容、重复或未选 projection 拒绝、pin/currentness、freeze→loss→confirm→create-only/Resource/print 路径；
- D9-BAF-P1-02：核完整 D7 ResultPin/ViewSpec 运行期原序、固定 complete_data scope、三个 resultInput 相等、renderer.layout 与 ViewSpec.layout 相等、targetKind/profileId/destination/receipt 一致、同数据可访问表格、本地隐藏 legend 不干扰导出，及六图表到 DOCX/XLSX/PDF/SVG/PNG/print 的正向路径；
- D9-8B6-P2-01 / D9-8B6-P2-02：核 Annotation selection/projection 集合、保留原序的 disclosure fragment、canonical origins、renderer asset/evidence-pin 数组，以及合法 D7 network 在当前六布局 D9 路线必定 renderer_unavailable；不得把 graph/nested nodeDetails/query_json 降级成 rows。D9-BAF-P2-02 的来源范围独立 CLOSED 结论仍固定在 fixed8b6；
- ROOT-8B6-MAP-P2-01：单独核 D8-SOURCE-MAP.json 中现任 consumer ACCEPTANCE.md path/blob 与原 760 条来源行的 source blob 不同角色。另核 D9-SOURCE-MAP.json 的当前来源资格、D9-REGISTRY.json、D9-TERMS.json 与全部 104 个唯一 D9-ACCEPTANCE row 及中英文投影；
- 单一 Core 解析器/身份/授权边界、ImportIR/Mapping/Loss/ConversionInput/ImportJob 闭合、D4 Registry/Field admission，以及禁止建立第二套 identity/ledger/patch authority；
- Node Template Recipe/ConstructionInput、sourceSubjectBindings 与原 receipt 的关联、PAB4/MinimumMapping/现任 wire13，以及 saved/planned/unknown 的恢复规则；
- 普通 Office token/style/repeat 规则，以及可见的 qualified native-table selector grammar：template bytes 必须自包含，nt_/nc_ 只作内部 Plan key，并采用最短唯一 suffix/title/occurrence、同一 rowset 的 repeat 与 Unicode escape；
- fresh Plan4/Catalog3/Selection2/Projection2/Loss2/Confirmation2/Receipt4/PrintReceipt1 封闭 family，并精确恢复 Plan/Receipt1–3；FC SPEC §8.3 的 exact Source/Resource/query_json generationPolicy=none、受控名称与 canonical ordering、template/route/style、递归 evidencePins、unknown-publication 规则都必须由 /4 继承；SPEC §17 unseen-current 分派、§18 inventory、SCHEMAS §§6.5–6.6、terminology、acceptance 与 replacement router 必须一致说明真实 /3 只作 recovery；D2Snapshot3/D8 呈现、confirmation 不可变、create-only 发布与独立 Resource author receipt 保持不变；
- current PortableAnnotationRecord/4 / Value4 / 唯一 R6 AnnotationInlineProfile consumer 边界保持不变；annotation_index 绝不能替代新的内容 carrier；
- worker/route/sandbox、D9 默认不联网、image/region、Mobile negative conversion surface，以及与 D10 直接相交但不等于完整 D10 的边界。
- historical 57/63/59/82/90/130/12 evidence set 分开记账；I01–I12 及产品/runtime/GUI/Office/OS/performance/deployment 全部继续 UNRUN。

独立复核必须报告 P0/P1/P2 disposition 与任何必要 read gap。完成前 D9 不能超出 author-resolved-pending-independent-review。之后仍必须完整整合 D10，并做全新非作者 Pro/global A2 终审。
