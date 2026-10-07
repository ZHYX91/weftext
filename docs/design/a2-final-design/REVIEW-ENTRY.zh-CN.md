---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D6 作者候选复核入口——fixed5e21 后导航修订

最新独立导航复核：fixed9c3b（`9c3b84282a7d000c53342e14e09fffe3f6138e4e`），REVISE，0 个开放 P1 / 1 个开放 P2。`A2-D6-829-P2-01` 已在 fixed5e21 独立 CLOSED；此前各项有界关闭继续保留。仅 `A2-D6-01CC-P2-01` 仍 OPEN。本轮由协调者只修复评审元数据，等待固定提交独立复核；不构成 D6/全局 A2 接受。D7–D10 完整模块、Mandatory 925–1141、SEARCH-01–08 与全新独立 Pro 全局终审继续 pending；runtime 证据仍 UNRUN。

## 1. 固定对象与作者时间线

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。

D6 finding 来源是 fixed829 `829efce6aacbe944714e093c98065b01d50b2593`。第一次非作者修订复核是 fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`。四残余作者修订从 `32cfb9c387deddb12fb021a44147df0d7ffab322` 开始；后续窄修经过 `d1ab2a5c0762450877614ad14340a26cfa0c1dfd`。下一轮独立复核把完成候选固定在 `1fc4a3937bbc06b4785d39f1eaf498f3c9c39abc`。此前两项 P2 修订从该 head 开始。语义关闭复核固定 5e21e9f00e1fa4e893f2544211133adbeac35ae0 并关闭语义 P2；本次元数据补修从 fixed9c3b 开始，关闭其指出的四处残余措辞。

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

### Navigation P2

最新独立导航复核：fixed9c3b（`9c3b84282a7d000c53342e14e09fffe3f6138e4e`），REVISE，0 个开放 P1 / 1 个开放 P2。`A2-D6-829-P2-01` 已在 fixed5e21 独立 CLOSED；此前各项有界关闭继续保留。仅 `A2-D6-01CC-P2-01` 仍 OPEN。本轮由协调者只修复评审元数据，等待固定提交独立复核；不构成 D6/全局 A2 接受。D7–D10 完整模块、Mandatory 925–1141、SEARCH-01–08 与全新独立 Pro 全局终审继续 pending；runtime 证据仍 UNRUN。

历史时间线：fixed829 来源 → fixed01cc 较早复核 → fixed32cf/d1ab 修订 → fixed1fc4 复核 → fixed5e21 语义复核 → fixed9c3b 导航复核。较早裁决只描述各自固定对象。

## 4. 证据与非声明边界

本修订 FULL 仅限 D6 source-map 语义 disposition/navigation surface，以及判断它们所需、已经登记的 fixed/current owner evidence。fixed-S snapshots 与 `docs/design/inputs.json` 受保护且保持不变。

D7–D10 仍只在 D6 具名 load-bearing intersection 上 PARTIAL；完整 D7–D10 A2 module 与 Mandatory 925–1141 继续 pending，本批不进入这些范围。

产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 行为证据全部 UNRUN。文档、机器与 CI 检查只证明仓库一致性，不替代语义接受。最终 A2 仍需后续完整整合、fresh Pro/global 非作者终审、一个 explicit accepted-design SHA，以及同一 accepted SHA 上的 freeze/implementation-start 材料。
