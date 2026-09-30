---
source_language: zh-CN
translation_status: source
---

[English](PROPOSAL.md)

# D6-FA-r01 文件权威重开联合候选说明

状态：**candidate / partial coordinated candidate / 未接受 / 未激活 / 未实现**。

固定上游仍为 S=7e18168dad3e6d120fce0dd607dc10fa7894e252；旧 D10 作者候选基线仍为 C8=d99f053b9386c9c9e1664251fdec9f00e33fac2c。本目录只保存 D6-FA-r01 候选后像，不修改 snapshots、inputs 或原 D10 十八份。只有全部真实 replacement owner、D10 消费修订、fresh 独立联合审查和协调接受完成后，才能讨论激活。

本文件只做路由、版本与差异说明；规范操作语义唯一归 owners/ 下对应 owner 后像。

## 1. 已确定产品取舍

1. Document 当前 exact source 由普通 `.adoc` 文件唯一承载；Resource 当前 bytes 由普通资源文件唯一承载。
2. 可删 SQLite I 与耐久 SQLite P 均在同步目录外；I 可重建，P 保存不可从文件重建的 decision/recovery/unknown/approval/claim/Money/pins；两者都不是正文 authority。
3. 可移植 identity、parent/order、lifecycle、shared configuration/ACL/trust 由 portable metadata物理承载，逻辑 owner仍归 D3/D4等真实领域 owner。
4. 多台登记 replica 可离线普通 edit/create/move/reorder/Trash，随后显式处理 source/placement/lifecycle/policy conflicts。
5. portable file复制不复制 global execution responsibility。
6. Server 保留多人 Draft/read/prepare/edit；一个托管后端的持久 commit holder唯一不等于单用户。
7. 实时协作仍 post-G2；未冻结 OT/CRDT，不采用每键 author commit。
8. 大库 ordinary reliable save 不等待无关 full index/OCR；complete Query/Action仍必须完整证明。

## 2. 当前实际 replacements：16

`replacements.json` 只登记实际存在的 16 份 fixed-S owner replacement：

- D6 五份：Storage、Control、Terminology Lexicon、Terminology Registry、Implementation Impact/Test。
- D1 两份：Product Surface/Capability Boundary、Implementation Impact/Test。
- D3 三份：Identity/References/Ownership/Lifecycle、Terminology Lexicon、Implementation Impact/Test。
- D4 三份：Attribute Types/Schema/Relations、Terminology Lexicon、Implementation Impact/Test。
- D5 三份：Tables/Node Collections、Terminology Lexicon、Implementation Impact/Test。

D4 Reference Catalog 与 mandatory scenarios 只是固定只读输入，不登记 replacement；本批不改 catalog typed definitions 或 limits。

## 3. 当前 owner 路由

| fact / capability | logical owner | D6-FA-r01 physical/control |
|---|---|---|
| Document/Resource current bytes | D2/Resource domain | ordinary files |
| identity/parent/order/lifecycle | D3 | portable metadata + D3 wire12 |
| Field/Facet/relation/Calendar Registry semantics | D4 | portable Registry metadata + D6 SourceVersion/control binding |
| native table structure / Node Collection semantics | D5 | exact source / D7 result |
| durable decision/recovery | protocol owner + D6 | local P |
| search/parser/index candidates | source owners | rebuildable I |
| global execution responsibility | D6/D10 | continuous fenced control |

P/I不能成为第二份 Field、relation、collection、parent/order作者真相。

## 4. 三层资格与新增 D4/D5 闭合

- **ordinary replica content**：普通 source save、D3 create_node/move/reorder/Trash、
  D5 native table local edit只证明真实触及的 local source/identity/structure/policy和安装范围。
- **complete semantic/action proof**：D4 relation/Facet strong mutation/unique Calendar、
  D5 collection membership/bulk/requireMembership、
  restore/purge/copy/fork/import、D7 all_result/automation等保留完整正负范围。
- **global execution responsibility**：Automation、ApprovalUse、claim、Money、external unknown、stop要求独立连续责任。

D4/D5 本批明确：
- D4 namespace `available|retained_unavailable|invalid|not_present`、Node typed `complete|partial|unavailable`、D6 `complete_semantics|semantic_pending` 与未来 D7 complete-cut 是四个不同维度。
- `semantic_pending` 只允许“本地 typed facts已经通过、跨对象/全集义务尚未证明”；本地 type/cardinality/requiredness失败仍拒绝。
- body/真正不相交 namespace 可以在 unavailable/invalid raw bytes byte-equal时普通保存；触及 unavailable/invalid namespace的 typed edit仍拒绝。
- D5 native table cell/row/column局部 structured edit不要求全库 Query cut；
  Node Collection strong mutation必须 complete membership/cut。
- current page、I coverage、
  placeholder或 hash相同都不能证明 relation/unique/Calendar/collection negative range或 all_result。

## 5. 版本与兼容

已生成：
- D6 Control wire 1→2、PreparedIntent 1→2、Policy 2→3、SourceVersion 1→2。
- D3 identity operation/receipt/resolver 11→12，ledger key加入 CommitDomain。
- D4 public Entry/Type/RelationReadContext/Binding/Recurrence/effect shapes保持；
  新 profile只在 outer D6 InputDescriptor/2把 source-bearing evidence绑定 SourceVersion/2、CommitDomain、Frontier。
- D5 不新增 durable wire identity；保留 D5 v1 domains/limits，只新增 D6-FA local-vs-complete consumer合同。

历史 D3 v9/v10/v11、D6 wire1、D7 PreparedActionBinding/1,/2 等 saved bytes按原decoder/gate/retention重放；不补新字段、不重编码。

## 6. D4 catalog 不变

固定 catalog blob：
`ca6ed9232864ba1bbc49e9aaa81f9290a78fe6cb`

继续定义 4 QualifierSetSpec、22 aliases、61 Fields、7 Facets、1 CalendarSeriesScopePolicy及原 limits。本候选不修改 catalog。

特别保留：
- `people/labeled-text-value.label` 是 optional `semantic_code` contribution-set；
- codes `people/other|people/personal|people/work`；
- `people/phone` 等复用 alias；
- label absent不是empty/unknown/default other；
- display label不等 SemanticCodeId；
- relation canonical owner/inverse与 Calendar comparator/series-scope保持。

## 7. D5 domain 与 hard limits

仍无 Record/RecordCollection durable domain。Document table row/cell是 revision-bound occurrence；
  Node Collection membership是 D7 Query派生，不是 parent/owner/persistent membership。

hard limits继续：
- collection mutation explicit Node targets ≤1000；
- native table structured row targets ≤1000；
- preview page ≤200；
- fetched page ≤200；
- import batch new Nodes ≤1000。

更窄budget可降低，不得放宽；pagination前N不能冒充全集。

## 8. 当前仍待 owner

| Owner | 尚待协调 |
|---|---|
| D7 | CommitDomain/Frontier/SourceVersion complete cut、new Prepared、effects pin retention、partial-index gate |
| D8 | Source/Live/Read、Live三种标记策略、Draft/IME/Undo/selection、多会话与 collaboration checkpoint |
| D9 | scoped pins、ImportJob、exact export inputs、publication、新 request/effects consumer |
| D10 | recipient/target/payload approval、sourceOccurrenceKey continuity、Money谱系、Run/Lease/Automation/deployment execution responsibility |

新 D7 Prepared未完成前，D4/D5需要 complete D7 preparation 的 strong Action仍 unavailable；不得半包激活。

## 9. 大库、冲突与历史结果

T_first_open、T_first_edit、T_first_reliable_save、T_full_search_ready、T_OCR_ready继续分开，不承诺未经实测秒数。

D4/D5 local operations不因无关 I coverage永久只读；strong complete consumer可扫描source，无法完成则 unavailable。

r5 receipt、current r6、I rebuild、P loss、external A→B→A分别处理；任何后续补验都不改写旧 receipt。多replica source/table/Field/collection冲突显式记录，不LWW。

## 10. D10 旧终审状态

固定 B13 继续是 **REVISE**，术语/双语 FAIL，P0=0、P1=3、P2=8，共 11 OPEN：

- P1：`R08-B13-P1-01`、`R08-B13-P1-02`、`R08-B13-P1-03`
- P2：`R08-B01-P2-01`、`R08-B02-P2-01`、`R08-B02-P2-02`、`R08-B02-P2-03`、`R08-B05-P2-01`、`R08-B11-P2-01`、`R08-B12-P2-01`、`R08-B13-P2-01`

本候选不关闭、重分类或独立接受任何 finding。后继不可变候选仍须 fresh 完整联合审查：新提案、D10 十八份、固定 S49 与全部真实 replacement owner 后像。作者检查/CI不是独立接受。
