---
source_language: zh-CN
translation_status: source
---

[English](D6-SOURCE-MAP.md)

# A2 D6 fixed1fc4 两项 P2 作者修订来源映射

状态：fixed1fc4 独立复核后，D6 仍为 **REVISE，0 个开放 P1、2 个开放 P2**。P1-01/P2-03 继续保留 fixed01cc 的 independently CLOSED；P1-02/P2-02 已在 fixed1fc4 independently CLOSED。本作者批次只修 `A2-D6-829-P2-01` 与 `A2-D6-01CC-P2-01`；在新的 exact-stop 复核前，两项都只标 author-resolved-pending-independent。这不是 D6/global 接受、实现、激活或发布。

## 1. 固定复核时间线

finding origin：fixed829 `829efce6aacbe944714e093c98065b01d50b2593`。
较早非作者复核：fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`。
四残余作者修订起点：`32cfb9c387deddb12fb021a44147df0d7ffab322`。
后续 follow-up：`d1ab2a5c0762450877614ad14340a26cfa0c1dfd`。
最新非作者固定对象：`1fc4a3937bbc06b4785d39f1eaf498f3c9c39abc`。

fixed1fc4 已独立 CLOSED P1-02 与 P2-02，并通过 D4/D5 十四条残余；结合原 188/202 条 R1 通过项和 R2 关闭，A2-D4D5:P2-01 现为 bounded CLOSED。本作者批次只接入上面两项 D6 P2。

## 2. P2-01——按义务 disposition

全部 **47** 个旧 `retain-or-named-current-successor` section group 都重新审计。结果：**27** 个确实混合，现用 `split-by-sub-obligation` 拆为 **84** 个显式 sub-obligation；**18** 个是单一 disposition 的 retained/current 义务（包括 retained-unexecuted 的实施/测试义务）；**2** 个只是 historical-only 状态/provenance。另有 3 个紧邻 fixed-S cross-owner group 存在同一旧版本歧义，也拆成 **6** 个显式 sub-obligation。299 个 source-section identity 与其精确来源证据完全不变。

最小反例被明确写出。fixed-S Storage 第 20、28–30、36–46、52 行选择 SQLite authority store 保存 current Document/Resource/Annotation bytes，把普通 `.adoc` 仅视为 checkout/proposal，并把 current source pointer 与 control 共置在 `authority.sqlite3`。current D6 §1–§3 则把 current Document/Resource bytes 放在普通文件 F，把 identity/structure/current Annotation 与 portable control 放在 M，把 decision/recovery/private pin 放在工作区外 P，把可重建事实放在 I，并把 Draft 独立为 D。因此旧物理正文存储选择属于 named-supersede，不能保留为 current。

同一批 group 中真正的业务义务仍按实际情况保留：one writable author truth、无第二套 Field/relation/Record 作者权威、完整 source 与 owner/structure 可取回、使用 canonical identity 而不是 path/rowid、受限 staging/pin、authority continuity/fencing、不能把“无 index row”当 empty proof、replay/idempotency，以及禁止静默 LWW/multi-master author commit。旧 candidate status、旧 wire/Policy 数字和 predecessor carrier/version 选择只按 historical 或 named-successor 分派，不能冒充 current 业务义务。

fixed-S snapshot 字节完全不改；current F/M/P/I/D 架构不重新设计；不建立第二权威。

## 3. Navigation P2——单一详细状态入口

[A2 REVIEW-ENTRY](REVIEW-ENTRY.zh-CN.md) 与本文是 D6 的详细状态/provenance 入口。D4/D5 human map 改为链接这两个入口，不再复制 D6 stop/status；A2 README 与 A2 SOURCE-MAP 同步 fixed1fc4：P1-01/P1-02/P2-02/P2-03 在各自记录的复核对象上 bounded independently CLOSED；只有 P2-01 与 navigation 在本批修订，并继续等待独立复核。

机器 map 记录 fixed829 为 finding origin、fixed01cc 为较早独立 D6 复核、fixed32cf/d1ab 为作者修订时间线、fixed1fc4 为最新独立复核，并把本批起点记录为 fixed1fc4。文件不会自指未知未来 final commit；真实 stop 只放 PR metadata/交接。

## 4. 保留证据与边界

保留库存不变：**299** 个 section identity；**760** 条 fixed97 case、其中 **115** 个 D6 intersection；**34** 个来源；**11** 对真实双语 pair；**89** 个 parent Impact record（85 个 unique，加 4 个预期 FA01/FA30/PL01/PL55 repeat）；以及 **767/1246** 个 Registry pointer。受保护的 fixed-S 49 snapshots 与 `docs/design/inputs.json` 保持逐字节不变。

D4/D5 的 202-row mapping、906/170/716 来源覆盖、33 Mandatory group、2854-catalog 唯一权威、六个 D10 blob/54 history，以及已关闭的十四条残余目标，本批都不改。

D7–D10 完整整合与 fresh Pro/global 复核继续 pending。新设计的产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 场景仍全部 UNRUN。仓库检查全绿也不等于语义接受。
