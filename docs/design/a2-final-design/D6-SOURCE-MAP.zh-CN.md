---
source_language: zh-CN
translation_status: source
---

[English](D6-SOURCE-MAP.md)

# A2 D6 fixed5e21 导航修订来源映射

最新独立导航复核：fixed9c3b（`9c3b84282a7d000c53342e14e09fffe3f6138e4e`），REVISE，0 个开放 P1 / 1 个开放 P2。`A2-D6-829-P2-01` 已在 fixed5e21 独立 CLOSED；此前各项有界关闭继续保留。仅 `A2-D6-01CC-P2-01` 仍 OPEN。本轮由协调者只修复评审元数据，等待固定提交独立复核；不构成 D6/全局 A2 接受。D7–D10 完整模块、Mandatory 925–1141、SEARCH-01–08 与全新独立 Pro 全局终审继续 pending；runtime 证据仍 UNRUN。

## 1. 固定复核时间线

finding origin：fixed829 `829efce6aacbe944714e093c98065b01d50b2593`。
较早非作者复核：fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`。
四残余作者修订起点：`32cfb9c387deddb12fb021a44147df0d7ffab322`。
后续 follow-up：`d1ab2a5c0762450877614ad14340a26cfa0c1dfd`。
较早非作者固定对象：`1fc4a3937bbc06b4785d39f1eaf498f3c9c39abc`。
语义关闭复核：`5e21e9f00e1fa4e893f2544211133adbeac35ae0`。
最新导航复核：`9c3b84282a7d000c53342e14e09fffe3f6138e4e`。

fixed1fc4 已独立 CLOSED P1-02 与 P2-02，并通过 D4/D5 十四条残余；结合原 188/202 条 R1 通过项和 R2 关闭，A2-D4D5:P2-01 现为 bounded CLOSED。fixed5e21 随后独立 CLOSED 语义 P2；仅导航问题仍待关闭。

## 2. P2-01——按义务 disposition

全部 **47** 个旧 `retain-or-named-current-successor` section group 都重新审计。结果：**27** 个确实混合，现用 `split-by-sub-obligation` 拆为 **84** 个显式 sub-obligation；**18** 个是单一 disposition 的 retained/current 义务（包括 retained-unexecuted 的实施/测试义务）；**2** 个只是 historical-only 状态/provenance。另有 3 个紧邻 fixed-S cross-owner group 存在同一旧版本歧义，也拆成 **6** 个显式 sub-obligation。299 个 source-section identity 与其精确来源证据完全不变。

最小反例被明确写出。fixed-S Storage 第 20、28–30、36–46、52 行选择 SQLite authority store 保存 current Document/Resource/Annotation bytes，把普通 `.adoc` 仅视为 checkout/proposal，并把 current source pointer 与 control 共置在 `authority.sqlite3`。current D6 §1–§3 则把 current Document/Resource bytes 放在普通文件 F，把 identity/structure/current Annotation 与 portable control 放在 M，把 decision/recovery/private pin 放在工作区外 P，把可重建事实放在 I，并把 Draft 独立为 D。因此旧物理正文存储选择属于 named-supersede，不能保留为 current。

同一批 group 中真正的业务义务仍按实际情况保留：one writable author truth、无第二套 Field/relation/Record 作者权威、完整 source 与 owner/structure 可取回、使用 canonical identity 而不是 path/rowid、受限 staging/pin、authority continuity/fencing、不能把“无 index row”当 empty proof、replay/idempotency，以及禁止静默 LWW/multi-master author commit。旧 candidate status、旧 wire/Policy 数字和 predecessor carrier/version 选择只按 historical 或 named-successor 分派，不能冒充 current 业务义务。

fixed-S snapshot 字节完全不改；current F/M/P/I/D 架构不重新设计；不建立第二权威。

## 3. Navigation P2——单一详细状态入口

[A2 REVIEW-ENTRY](REVIEW-ENTRY.zh-CN.md)

最新独立导航复核：fixed9c3b（`9c3b84282a7d000c53342e14e09fffe3f6138e4e`），REVISE，0 个开放 P1 / 1 个开放 P2。`A2-D6-829-P2-01` 已在 fixed5e21 独立 CLOSED；此前各项有界关闭继续保留。仅 `A2-D6-01CC-P2-01` 仍 OPEN。本轮由协调者只修复评审元数据，等待固定提交独立复核；不构成 D6/全局 A2 接受。D7–D10 完整模块、Mandatory 925–1141、SEARCH-01–08 与全新独立 Pro 全局终审继续 pending；runtime 证据仍 UNRUN。

D6-SOURCE-MAP.json 保存当前 D6 评审状态；其他机器 map 引用它。较早固定裁决是历史证据，不是当前待审问题。不自填未知未来 commit；实际交回 SHA 记录在 PR metadata。

## 4. 保留证据与边界

保留库存不变：**299** 个 section identity；**760** 条 fixed97 case、其中 **115** 个 D6 intersection；**34** 个来源；**11** 对真实双语 pair；**89** 个 parent Impact record（85 个 unique，加 4 个预期 FA01/FA30/PL01/PL55 repeat）；以及 **767/1246** 个 Registry pointer。受保护的 fixed-S 49 snapshots 与 `docs/design/inputs.json` 保持逐字节不变。

D4/D5 的 202-row mapping、906/170/716 来源覆盖、33 Mandatory group、2854-catalog 唯一权威、六个 D10 blob/54 history，以及已关闭的十四条残余目标，本批都不改。

D7–D10 完整整合与 fresh Pro/global 复核继续 pending。新设计的产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 场景仍全部 UNRUN。仓库检查全绿也不等于语义接受。
