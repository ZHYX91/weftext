---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D9 作者候选复核入口——D9 完整整合

后续非作者复核绑定 fixed26be（`26be071d4c2075343d9ebf272e00f23769ce0d64`），结论 PASS：P0=0 / P1=0 / P2=0，并 CLOSED 最终 D7 残余；此前有界 closure 继续绑定各自原 SHA。D7 在 D7 范围 accepted。D8 继续是已保全、未接受的作者候选，其独立复核保持单独 pending。本 D9 作者批精确从 `5eca16c40cdf2e1892f6930d51c632ea720a460c` 开始，完整整合八份 D9 fixed source、current bilingual owner、Mandatory §14、current D7 D9-binding/PAB consumer 与 final-FC D9 successor，并把 D9 统一保持在 `author-resolved-pending-independent-review`。不自行接受 D8、D9 或 global A2。完整 D10 与 fresh Pro/global 复核继续 pending；产品/runtime 证据仍 UNRUN。

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

D7 已在 fixed26be 的 D7 范围独立 PASS。D8 继续是已保全、未接受候选，等待自己的 exact-final-SHA 独立复核。D9 现在从 exact 5eca 形成完整作者候选：八份 fixed-S FULL、current D9 owner 中英 FULL、Mandatory §14 FULL、D7 D9-binding/PAB consumer FULL、D9 相关 final-FC successor FULL；fresh author currentize 到 wire13/PAB4/Effect3，export/publication 到 Plan3/Receipt3，并闭合可见 native-table selector 与 current Value4/R6 Annotation consumer 设计边界。所有 D9 结果只为 author-resolved-pending-independent-review。完整 D10 仍留后续。

产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 行为证据全部 UNRUN。文档、机器与 CI 检查只证明仓库一致性，不替代语义接受。最终 A2 仍需后续完整整合、fresh Pro/global 非作者终审、一个 explicit accepted-design SHA，以及同一 accepted SHA 上的 freeze/implementation-start 材料。


## D8 复核目标

下一位非作者只绑定 PR #5 的最终 stop SHA。D8 继续等待独立复核；不得因为作者修订或 CI 变绿就推导语义接受。重点核验 `D8-SOURCE-MAP.json`、`D8-ACCEPTANCE.json`、current PB3/Value4/Snapshot3 边界、SEARCH-01–08 interaction、current D7 View hard answer/builder/旧入口路由、RTL/AT/性能义务，以及 S49/inputs 零变。必须重新核作者给出的 54/54 section navigation、760/760 FC 分类（direct 167 / upstream 142 / 保留 owner 451；110 行 applicability 作者改动）、acceptance 213/213 唯一性/极性与十个 `VIEW-BLD-01..10` fixture；它们都只是作者 resolved，复核者可以修订。D9/D10 完整模块与 global A2 不属于本轮 D8 接受范围。

## D9 复核目标

下一位非作者 reviewer 必须绑定 PR #5 的 exact final stop SHA，对 D9 做新的完整独立复核。D9 只是完整作者候选；source coverage、machine map、count、hash 和绿色文档 CI 都不是 acceptance。

必须核验：

- 八份 fixed-S D9 source、现任 D9 owner 中英文 afterimage、Mandatory §14 的 884–924 行、两层 replacement router、现任 D7 D9-binding/PAB owner，以及适用于 D9 的 final-FC successor；
- D9-SOURCE-MAP.json、D9-REGISTRY.json、D9-TERMS.json 与 D9-ACCEPTANCE.json 中 80 个唯一 obligation；
- 单一 Core 解析器/身份/授权边界、ImportIR/Mapping/Loss/ConversionInput/ImportJob 闭合、D4 Registry/Field admission，以及禁止建立第二套 identity/ledger/patch authority；
- Node Template Recipe/ConstructionInput、sourceSubjectBindings 与原 receipt 的关联、PAB4/MinimumMapping/现任 wire13，以及 saved/planned/unknown 的恢复规则；
- 普通 Office token/style/repeat 规则，以及可见的 qualified native-table selector grammar：template bytes 必须自包含，nt_/nc_ 只作内部 Plan key，并采用最短唯一 suffix/title/occurrence、同一 rowset 的 repeat 与 Unicode escape；
- 完整 ExportPlan/3 与 PublicationReceipt/3 字段、类型化 evidence/recovery pins、generationPolicy=none 精确路径、D2Snapshot3/D8 呈现渲染、output-name 规范化、confirmation 不可变、create-only 发布，以及与之分离的 Resource author receipt；
- current PortableAnnotationRecord/4 / Value4 / 唯一 R6 AnnotationInlineProfile consumer 边界：Node Template omission 只局限 construction，annotation_index 绝不能替代 body read 或形成全局 plain-only downgrade；
- worker/route/sandbox、D9 默认不联网、image/region、Mobile negative conversion surface，以及与 D10 直接相交但不等于完整 D10 的边界。
- historical 57/63/59/82/90/130/12 evidence set 分开记账；I01–I12 及产品/runtime/GUI/Office/OS/performance/deployment 全部继续 UNRUN。

独立复核必须报告 P0/P1/P2 disposition 与任何必要 read gap。完成前 D9 不能超出 author-resolved-pending-independent-review。之后仍必须完整整合 D10，并做全新非作者 Pro/global A2 终审。
