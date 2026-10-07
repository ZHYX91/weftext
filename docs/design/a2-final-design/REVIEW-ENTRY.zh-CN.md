---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D8 作者候选复核入口——D8 双语同步修复

后续非作者复核绑定 fixed26be（`26be071d4c2075343d9ebf272e00f23769ce0d64`），结论 PASS：P0=0 / P1=0 / P2=0，并 CLOSED 最终 `A2-D7-2F89-P2-01`。此前有界 closure 继续绑定各自原 SHA。D7 在 D7 范围 accepted。已保全的 D8 候选首次真实推送在 `7e3f0272f112cf0b8e61ab14fb9eaf72917e754b`；本继任作者窄批精确从该 SHA 开始，只修双语文档同步，不重新执行原 D8 整合。原作者留下的 FC 分类、historical→current disposition、source-map 导航与逐项义务语义核销缺口继续 OPEN；本作者不自行接受 D8 或 global A2。D9–D10 完整模块与 fresh Pro/global 复核继续 pending；产品/runtime 证据仍 UNRUN。

## 1. 固定对象与作者时间线

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。

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

D7 已在 fixed26be 的 D7 范围独立 PASS。D8 保持 7e3f 已保全的候选状态。本窄修复只同步 main/interfaces/schemas/direction/lexicon/impact/acceptance 中英文，并更新直接受影响的状态和 blob 元数据；它不关闭原 source-map、FC、history disposition 或逐项语义核销缺口。D8 仍等待 fixed-SHA 非作者完整复核。D9–D10 只在 D8 具名 producer/consumer intersection 上 PARTIAL；其完整 A2 module 继续 pending。

产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 行为证据全部 UNRUN。文档、机器与 CI 检查只证明仓库一致性，不替代语义接受。最终 A2 仍需后续完整整合、fresh Pro/global 非作者终审、一个 explicit accepted-design SHA，以及同一 accepted SHA 上的 freeze/implementation-start 材料。


## D8 复核目标

下一位非作者只绑定 PR #5 的最终 stop SHA。D8 继续等待独立复核；不得因为双语同步或 CI 变绿就推导语义接受。重点核验 `D8-SOURCE-MAP.json`、`D8-ACCEPTANCE.json`、current PB3/Value4/Snapshot3 边界、SEARCH-01–08 interaction、current D7 View 两个 hard answer、RTL/AT/性能义务，以及 S49/inputs 零变。还必须处理仍开放的来源映射/语义缺口：54 个 `fixedProseSections` 中有 50 个仍使用宽泛整文件 target，FC applicability 与 historical→current/逐项义务 disposition 也尚未独立闭合。D9/D10 完整模块与 global A2 不属于本轮 D8 接受范围。
