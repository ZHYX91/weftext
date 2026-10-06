---
source_language: zh-CN
translation_status: source
---

[English](D6-SOURCE-MAP.md)

# A2 D6 fixed829 来源、案例、Registry 与修复映射

状态：fixed829 五项 finding 仅为 `author-resolved-pending-independent`；这不是独立接受、实现、激活或全局 A2 接受。

## 1. 固定复核与修复对象

D6 独立裁决绑定 `829efce6aacbe944714e093c98065b01d50b2593`，结论为 REVISE，P0=0、P1=2、P2=3。本次作者修复从 `4282d416e6a1ea4a344a2647d9a2feb82e3ba15a` 起步；下一位复核者必须绑定本修复实际产生的 final stop SHA，不能跟随 moving branch。

D1/D2 与两个有界 D3 finding 保留此前 independent CLOSED 状态。D4/D5 audit finding 属另一 fixed-object 工作流，不计入这五项 D6 finding。

## 2. FULL / PARTIAL / UNREAD 证据边界

本 D6 修复中的 FULL 仅表示：机器 map 所代表的五份 fixed-S D6 输入；本批实际使用的 current-parent D6 Storage/Control/Impact/Lexicon 中英 owner pair 与 52-concept/17-binding registry；D6 交叉实际需要的 current A2 D3-D5 文件；全部 89 个 parent D6 Impact source record，其中是 85 个唯一 case ID 加 4 个范围/元数据 record；fixed-S/current D6 两套 Registry pointer inventory；以及机器 map 实际登记为 D6 intersection 的 fixed97 row。

PARTIAL 指 D7-D10：这里只消费机器 map 明确列出的 direct-holder 文件与 load-bearing intersection，不声称它们的完整 A2 module 已全文读取或整合。

UNREAD/pending 包括未列出的 D7-D10 完整模块来源、Mandatory 925-1141，以及产品/runtime/OS/GUI/crypto/真实 replica/provider/performance/migration/activation 证据。

## 3. fixed829 五项 finding

### A2-D6-829-P1-01

来源：fixed-S 的 D6 Control/Storage、当前 parent 的 D6 owner 后像、fixed97 当前后继材料，以及机器映射具名的 D3/D10 直接 holder。

current anchors：D6-CONTROL §0.1、§4.3-§5、§6、§9.4、§10.2、§20-§22；D6 Main §7-§9、§14、§17；D6-SCHEMAS §5、§9-§10；D6-IMPACT §5、§7-§11。

设计判据：当前新鲜路径只有一条外层链：`DependencyProof/3`/Input3/Prepared3 使用 `d6_plan/3`，随后进入 Notice3、原唯一 final P，再到 CP4/ChangeRecord1。当前 trust/bootstrap/replica/execution family 各只有一个具名 current producer。真实前身 Notice/CP/trust/bootstrap/responsibility 以及已保存、已计划或结果未知的记录，保留其已记录 decoder、bytes、pins、授权、错误顺序、installation/seal/receipt 与 recovery。不得新增 migration、第二 ledger、第二 CAS、第二 submit 或按版本名 fallback。

### A2-D6-829-P1-02

来源：D6-CONTROL §3.4-§3.6、D6-SCHEMAS §5.1、D6 Main §14、D6-IMPACT §5/§10，以及 fixed97 双语 row T3-PROFILE-22、FC34-FMT-19、FC34-FMT-05、FC34B-D10-05、FC34B-D10-06。

设计判据：current Proof3/Input3/Prepared3 的 literal version 都是 3，plan token tag 是 `d6_plan/3`；Key3 恰好 15 个 rank 0..14，且 `document_format` 为 rank 1。实际消费 managed-Document 语义的路径必须冻结 format binding；即使 source bytes/version 不变，M1→M2 也使旧 Proof3 失效。不消费 managed-Document 语义的路径不伪造该 dependency。format-only portable change 的 sourceChanges=[]，不创建 SourceRevisionPlan、managed SourceVersion 或 H advance。真实 Key2/Proof2 继续是原十四臂 historical family。

### A2-D6-829-P2-01

来源：fixed-S D6 Control §5-§13，特别是 98-247 行，以及其中具名的真实 D3/D9/D10 owner boundary。

当前落点：D6-CONTROL §17.1 的 ResultPage 分页合同、§17.2 的 BudgetBinding、§17.3 的 ImportJob、§17.4 的受管配置/control read、§17.5 的 ByteRead、§17.6 的继承接口与 ObservationScope、§17.7 的 SourceBinding/OriginBinding。

设计判据：仍然有效的原合同继续完整保留正向路径、数值域、授权与恢复。Result paging 保留 pageSize 1..200、完整结果先于分页、合法的空 nonterminal page、terminal cursor、TTL/reset 与错误顺序。Budget 保留原 12 个 member 与 numeric domain。ImportJob 保留 pins、mapping DAG/SCC、atomic groups、每 batch 的 OperationId、canonical requests/plans/receipts、binding/version/watermark/budget 与 committed-prefix recovery。ByteRead 保留 resource_bytes/1、offset/maxBytes、解码后的 base64 长度、short-read-not-EOF、snapshot/current authorization、TTL/reset/shared charging。SourceBinding/OriginBinding 保留真实 comparator 与 owner boundary。旧 numeric SourceVersion、wire11/12 与 Scope1 只作为版本限定的前身证据，不是当前新鲜 carrier。

### A2-D6-829-P2-02

来源：fixed-S D6 Lexicon/Registry、current-parent 52-concept/17-binding Registry，以及 current D6 Lexicon/Registry。

current anchors：D6-LEXICON §3 与 §7-§9；D6-REGISTRY generic definitions、currentTechnicalSuccessors、historicalDispatch。

设计判据：原 concept ID、全部 fields、owned names、aliases、locale mapping 与 firstFreeze provenance 全部保留。通用当前定义只指向真实 Notice3/CP4、Proof3/15-key、Input2、Witness2、Inventory2/Record3/Proof2、Profile4/Plan4、Bundle2/Declaration2/Handle2 family。前身只保留为显式的版本限定历史事实；不建立 compatibility layer，也不随机机械升 inner type。

### A2-D6-829-P2-03

来源：current-parent D6 Storage §7.2.1 capacity 段及其完整 inherited scheduling obligations。

current anchors：D6 Main §7.2.1；D6-IMPACT §10.1 的 `D6-SCHED-CAP-02` 等案例；D6-REGISTRY schedule-continuity-witness。

设计判据：单位是每 producer update 最多 4096 retained pins、最多 16 MiB canonical evidence metadata；source/component bytes 使用独立 reserved PinBudget。capacity failure 必须先在同一 producer transaction 原子记录 gap/invalidation，compaction 保持 reference-safe。1000 transitions × 5 pins = 5000 pins，因此即使 1000 小于 4096 也必须失败；4096 不是 transition-count ceiling。

## 4. 机器审计包

D6-SOURCE-MAP.json 将 fixed-S Control 23 个 section 全部来源限定到具体 current anchor，不再只写 whole-file target。它保留 89 个 parent Impact source record，并明确区分 85 个唯一 case ID 与 4 个范围/元数据 record。760 条 fixed97 row 全部保留，其中 115 条 D6 intersection 附带真实双语 source row。Registry pointer map 保留 767 个 fixed-S 与 1246 个 current-parent pointer，并区分 exact-retain value 与 named current successor。

机器库存与导航不能替代语义接受。

## 5. 剩余边界

D7-D10 完整 A2 module、Mandatory 925-1141、A2 完成与真正 fresh Pro/global review 仍 pending。runtime 与产品行为全部 UNRUN。后续仍须有 accepted-design SHA 才能进入 freeze/implementation startup。
