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
8. 大库 ordinary file save 不等待无关 full index/OCR；complete Query/Action仍必须完整证明。
9. 用户已批准：受信人工单一既有 live Document 的 ordinary save 可在完整授权、局部门、耐久输入/已读取前像和唯一P决议链保持时使用 WriteProtection=observed_only；它只放宽未观察外部竞争排除，不放宽已观察冲突、失权、幂等、耐久或强Action资格。

## 2. 当前实际 replacements：16

`replacements.json` 只登记实际存在的 16 份 fixed-S owner replacement：

- D6 五份：Storage、Control、Terminology Lexicon、Terminology Registry、Implementation Impact/Test。
- D1 两份：Product Surface/Capability Boundary、Implementation Impact/Test；G0-B 已生成对应 owner 后像。
- D3 三份：Identity/References/Ownership/Lifecycle、Terminology Lexicon、Implementation Impact/Test；G0-B 已生成对应 owner 后像。
- D4 三份：Attribute Types/Schema/Relations、Terminology Lexicon、Implementation Impact/Test。
- D5 三份：Tables/Node Collections、Terminology Lexicon、Implementation Impact/Test。

D4 Reference Catalog 与 mandatory scenarios 只是固定只读输入，不登记 replacement；本批不改 catalog typed definitions 或 limits。

## 3. 当前 owner 路由

| fact / capability | logical owner | D6-FA-r01 physical/control |
|---|---|---|
| Document/Resource current bytes | D2/Resource domain | ordinary files |
| identity/parent/order/lifecycle | D3 | portable metadata + D3 wire12 |
| Field/Facet/关系/Calendar Registry 语义 | D4 | 可移植 Registry 元数据 + D6 生产 SourceVersion/2 与当前 SourceObservation/SourceVersionRef 外层资格 |
| native table structure / Node Collection semantics | D5 | exact source / D7 result + D6 当前 SourceObservation 外层资格 |
| durable decision/recovery | protocol owner + D6 | local P |
| search/parser/index candidates | source owners | rebuildable I |
| global execution responsibility | D6/D10 | continuous fenced control |

P/I不能成为第二份 Field、relation、collection、parent/order作者真相。

## 4. 三层资格与当前 D4/D5 候选消费

- **ordinary replica content**：普通 source save、D3 create_node/move/reorder/Trash 只证明真实触及的 local source/identity/structure/policy 和安装范围；D5 native table cell/row/column/reorder 可依完整真实局部证据工作，不等待无关 I 或全 Workspace Query，但结构化编辑仍使用 strict 保护。
- **完整语义/动作证明**：D4 relation、Facet 强修改、Calendar 唯一性，D5 collection membership、bulk、requireMembership，restore、purge、copy、fork、import，以及 D7 all_result、automation 等继续要求完整正负范围。
- **global execution responsibility**：Automation、ApprovalUse、claim、Money、external unknown、stop 继续要求独立连续责任。

G0-A 已提供 D6 producer，G0-B 已登记 D1 产品消费与 D3 owner 配套消费；本 generation 现已形成 D4/D5 六份 owner 候选后像并登记其当前 consumer 边界。它们仍只是 candidate，未接受、未激活、未实现；D7/D8/D9/D10 与后续联合接受门继续保留。

D4/D5 的 source-bearing 消费保留原内层 wire：D4 的 sourceRevision、OccurrenceKey、Entry/Type/RelationReadContext/Binding/Recurrence 以及 D5 的 revision-bound table/row/cell locator 不被替换。外层资格使用 D6 当前定义：SourceVersion/2 仍是生产版本历史；managed_source_version/2 保留 entityRef、commitDomain、observationEpoch、revision、changeId 且 changeId.commitDomain 等于该生产 commitDomain，external_source_version/2 保留 entityRef、commitDomain、observationEpoch、externalSequence。生产 commitDomain 可以不同于当前 operation 域；完整 SourceObservation/1 要求 observerDomain 等于 operation CommitDomain、entityRef 等于 sourceVersion.entityRef，并以当前 observationEpoch、fileObjectBinding、evidencePins 以及同一 cut 内的 control、Registry、incidence 依赖证明当前观察。SourceVersionRef/1.sourceToken 以 d6_source_observation/1 选择完整当前 Observation，InputDescriptor/2.sourceInputs[].observation 实际承载它。watcher gap、replacement 或 discontinuous rematerialization 会使旧 token 与其保护的 locator/selector 失效，即使生产版本、hash、I 或相同行文字相同也不能续认。Frontier/2 只表示 sealed causal/dependency prefix，不替代全集 Query、Registry 完整性或 payload materialization proof。

ordinary 语义与 strict|observed_only 保存保护是独立两轴，ordinary 可以使用 strict。只有满足 D6 §4.1 全部条件的人工 raw 普通整源保存，才能在 planning 开始前显式选择 observed_only 并冻结 profile：trusted interactive_source_save、恰一个 existing live Document、ordinary + replica_local、完整 source read/replace、author write set 为空或仅该 Document、无 applicable body/Field/Node-control deny、无 identity/parent/order/lifecycle/shared policy/Registry/Calendar scope/other-entity mutation，并且 DraftBase 等于 selected current Observation。structured cell/row/column/reorder、bulk、collection、promotion、strong Action、Automation、server checkpoint、Approval、Money 都不使用 weak 保护；strict 失败、已知竞争、授权、耐久或 strong obligation 失败不得 fallback。observed_only 耐久保留已读前像 B 与用户输入 N；安装 N 可能覆盖从未观察到的外部 C，后来 C 也可能替换 current file，但已耐久 B/N 不丢失。已观察竞争、stale Base 或观察连续性 gap 要求 conflict/reprepare；unknown install 保持 recovery_unknown；prepare 不是 Saved。durable_observed_only 不计入旧 strict T_first_reliable_save。

D4/D5 继续保持以下边界：
- D4 namespace `available|retained_unavailable|invalid|not_present`、Node typed `complete|partial|unavailable`、D6 `complete_semantics|semantic_pending` 与未来 D7 complete-cut 是不同维度。
- `semantic_pending` 只表示本地 typed facts 已通过而跨对象/全集义务尚未证明；它不表示 empty，不洗掉 typed/source invalid 或缺失 strong evidence，也不授权 Action/all_result/bulk/Automation。
- body/真正不相交 namespace 可以在 unavailable/invalid raw bytes byte-equal 时普通保存；触及 unavailable/invalid namespace 的 typed edit 仍拒绝。
- 当前页面、I 覆盖、占位状态、裸 SourceVersion、hash 或相同行文字，都不能证明 relation、unique、Calendar、collection 的否定范围或 all_result。

## 5. 版本与兼容

已生成：
- D6 控制线协议由 1 升至 2，PreparedIntent 由 1 升至 2，Policy 由 2 升至 3，SourceVersion 由 1 升至 2；G0-A 同时冻结 Frontier/2、ObservationScope/2、DependencyProof/2、InstallationNotice/2、ContentCompletionProof/2、SourceObservation/1 与 WriteProtection 的候选定义。
- D3 identity operation/receipt/resolver 11→12，ledger key加入 CommitDomain。
- D4 公开的 Entry/Type/RelationReadContext/Binding/Recurrence/effect shapes 保持不变；既有内层 sourceRevision/OccurrenceKey/Entry selector wire 不变，外层 D6 InputDescriptor/2.sourceInputs[].observation 承载完整的当前 SourceObservation/1，SourceVersionRef/1.sourceToken 选择该当前 Observation；SourceVersion/2 仍表示生产版本，Frontier/2 仍只表示已封存的因果/依赖前缀。
- D5 不新增 durable wire identity；保留 D5 v1 domains/limits、revision-bound locator 与既有 Field inner selector。当前 source-bearing 操作增加同一 SourceObservation/1/SourceVersionRef/1 外层资格；structured native edit 保持 strict，collection/bulk strong Action 继续等待新版 D7 complete Prepared。

G0-B 同步登记 D1 产品消费：ReliableSaveState/InputRetention 与 strict 可靠保存、durable_observed_only 耐久保留指标边界分开；observed_only 不计入旧 strict T_first_reliable_save。

G0-B 同步登记 D3-native descriptor、SourceObservation/1、最小 receipt/no_op 与同 P primary/companion 的消费边界；仍保持后续 consumer、联合审查和激活 gate。

历史 D5 v1 语义/兼容表面继续按原版本解释；D3 v9/v10/v11、D6 wire1、D7 PreparedActionBinding/1,/2 等已承诺 saved decision/receipt/unknown recovery 继续按原 decoder/gate/retention 和原 bytes 处理；不补新字段、不重编码。新 active Record 执行域及其专用 API/capability/aliases 的退役不机械删除这些历史恢复入口；历史 decoder 也不能恢复当前 Query/strong Prepared、Approval 或 Money 资格。没有部署证据的 prototype 不自动提升为全 active compatibility。

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

## 8. 当前已配套与仍待 consumer/owner

| Owner | 当前候选状态 / 尚待协调 |
|---|---|
| D1 | G0-B 已登记普通保存产品状态/指标与 strict/ordinary-file 文案消费；仍未激活 |
| D3 | G0-B 已登记 D3-native owner descriptor、Frontier/2、SourceObservation、companion/receipt 消费；仍未激活 |
| D4 | 三份当前 owner 后像已登记 production SourceVersion/observerDomain 分离、current SourceObservation/SourceVersionRef 外层资格、inner selector 保持和 qualified weak B→N范围；仍未接受、未激活 |
| D5 | 三份当前 owner 后像已登记 current source/locator observation资格、native structured strict与 collection/bulk strong gate；仍未接受、未激活 |
| D7 | CommitDomain/Frontier/SourceVersion/SourceObservation 的完整 cut、新版 Prepared、effects pin 保留、partial-index 门禁 |
| D8 | Source/Live/Read、Live三种标记策略、Draft/IME/Undo/selection、多会话与 collaboration checkpoint |
| D9 | scoped pins、ImportJob、exact export inputs、publication、新 request/effects consumer |
| D10 | recipient/target/payload 审批、sourceOccurrenceKey 连续性、Money 谱系、Run/Lease/Automation/deployment 执行责任 |

新 D7 Prepared未完成前，D4/D5需要 complete D7 preparation 的 strong Action仍 unavailable；D4/D5 的当前作者后像只注册候选消费，不构成部分激活或语义接受。

## 9. 大库、冲突与历史结果

T_first_open、T_first_edit、T_first_reliable_save、T_full_search_ready、T_OCR_ready继续分开，不承诺未经实测秒数；G0-B 已登记 ReliableSaveState/InputRetention 与 strict 可靠保存、durable_observed_only 耐久保留的指标边界。durable_observed_only 仍不计入旧 strict T_first_reliable_save，不解除后续 consumer、联合审查和激活 gate。

D4/D5 local operations不因无关 I coverage永久只读；strong complete consumer可扫描source，无法完成则 unavailable。

r5 receipt、current r6、I rebuild、P loss、external A→B→A分别处理；任何后续补验都不改写旧 receipt。多replica source/table/Field/collection冲突显式记录，不LWW。

## 10. D10 旧终审状态

固定 B13 继续是 **REVISE**，术语/双语 FAIL，P0=0、P1=3、P2=8，共 11 OPEN：

- P1：`R08-B13-P1-01`、`R08-B13-P1-02`、`R08-B13-P1-03`
- P2：`R08-B01-P2-01`、`R08-B02-P2-01`、`R08-B02-P2-02`、`R08-B02-P2-03`、`R08-B05-P2-01`、`R08-B11-P2-01`、`R08-B12-P2-01`、`R08-B13-P2-01`

本候选不关闭、重分类或独立接受任何 finding。后继不可变候选仍须 fresh 完整联合审查：新提案、D10 十八份、固定 S49 与全部真实 replacement owner 后像。作者检查/CI不是独立接受。
