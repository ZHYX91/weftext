---
source_language: zh-CN
translation_status: source
---

[English](D6-SOURCE-MAP.md)

# A2 D6 fixed01cc 残余修订来源映射

状态：绑定 `01cc40b819df78fbe724f1c64c27284ad60fc6c8` 的独立复核结论为 **REVISE**。`A2-D6-829-P1-01` 与 `A2-D6-829-P2-03` 已在该固定对象独立 CLOSED；`A2-D6-829-P1-02`、`A2-D6-829-P2-01`、`A2-D6-829-P2-02` 与导航 finding `A2-D6-01CC-P2-01` 是从作者起点 `32cfb9c387deddb12fb021a44147df0d7ffab322` 修订的四项残余，在新的 exact-stop 非作者复核前只标 **author-resolved-pending-independent**。这不是 D6/global 接受、实现、激活或发布。

## 1. 复核对象与状态分离

原五项 finding 来源于 fixed829 `829efce6aacbe944714e093c98065b01d50b2593`；该 SHA 继续只作为 finding-origin provenance。后续非作者复核对象是 fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`。

fixed01cc 已独立关闭：
- `A2-D6-829-P1-01`：唯一 fresh current Key3→Proof3→Descriptor3→Prepared3/Notice3/final-P/CP4+ChangeRecord1 链及其 current trust/bootstrap/replica/execution 后继。
- `A2-D6-829-P2-03`：每 producer update 最多 4096 retained pins 加 16 MiB canonical evidence metadata；source/component bytes 使用独立 reserved PinBudget，容量失败先写 gap 再允许普通进度。

fixed01cc 继续 OPEN/PARTIAL：
- `A2-D6-829-P1-02`：中文 closed-union 基数。
- `A2-D6-829-P2-01`：完整 source→current-owner/anchor 与双语来源证据。
- `A2-D6-829-P2-02`：四个通用 Registry 字段及精确 pointer disposition。
- `A2-D6-01CC-P2-01`：当前固定对象导航。

下一位复核者必须绑定 PR metadata/交接中记录的实际 final stop SHA；本文不会把未知未来 commit hash 自指进文件。

## 2. P1-02——十五-arm current dependency family

fresh current D6 恰为 **15 个** `DependencyKey/3` arm，rank 为 `source=0`、`document_format=1`、…、`execution_resource=14`；匹配的 current family 是 `DependencyProof/3`、`InputDescriptor/3`、`PreparedIntent/3` 与 `d6_plan/3`。Control 中英文必须陈述同一基数，D6-IMPACT 中的 `D6-DK-CARD-01` 是显式设计 oracle。

实际消费 managed-Document 语义的路径必须绑定 `document_format`；即使 Source bytes/SourceVersion 相同，M1→M2 也使旧 Proof3 stale。不消费 managed-Document 语义的路径不伪造 dependency。format-only transition 的 `sourceChanges=[]`，不创建 SourceRevisionPlan、managed SourceVersion 或 H advance。真实 Key2/Proof2 继续是精确的十四-arm historical family。

## 3. P2-01——完整来源/current 落点

机器 map 保留全部 **299** 个 section mapping，但不再把 whole file 当作语义 target：
- fixed-S Control 的 23 个 section 保留已经精确的 anchors；
- 其余 276 个 fixed-S Storage/Impact/Lexicon、current-parent Storage/Control/Impact/Lexicon 与 fixed97 SPEC/SCHEMAS section 都写明真实 current file+anchor、disposition、owner/basis 与来源限定 oracle；
- parent 与 fixed97 双语证据登记真实 EN/ZH path+blob pair。fixed-S snapshot 仍只是实际存在的单份受保护 source；不编造不存在的双语 snapshot 或 JSON twin。

fixed97 ACCEPTANCE 的 **760** 条原 row 与原 source text 全部保留；其中 **115** 条 D6 intersection 保留完整 EN/ZH source row。原来已经精确的 5 条 target 保持不动；其余 110 条改成真实 current owner/consumer 和具体 anchor，不再使用“D6-IMPACT intersection”泛 placeholder。跨 owner 义务明确写真实 owner 与 D6 实际 producer/consumer 角色，不建立第二套 D6 权威。

当前 fixed97 direct-owner 合同继续有效：ResultPage §17.1、12-member BudgetBinding §17.2、ImportJob §17.3、受管配置/control read §17.4、immutable ByteHandle/ByteRead §17.5、authorization-before-read/ObservationScope §17.6、SourceBinding/OriginBinding §17.7。历史 numeric SourceVersion、Scope1、wire11/12 只在各自 recorded version 下保留业务不变量。

## 4. P2-02——只修四个 Registry successor

D6-REGISTRY 继续保持 **52 concepts / 17 cross-stage bindings**，并保留 conceptId、ownedNames、aliases、locale、firstFreeze 与其它无关 exact value。只修复复核点名的四个通用字段：
- commit-protocol definition：current D3 是 wire13 + InputDescriptor/3 + `d3_identity_operation/13`；真实 wire9–12 是精确历史；`d6_commit_request/2` 仍是实际 D6 submit。
- conflict-record definition：current outer 使用 InputDescriptor3/Proof3/Prepared3；source_merge/choose_source_head 保留 inner Input2/Plan1/Preview1 + ownerKind/2；policy_bundle_choice 使用 inner Input3/Plan2/Preview2 + ownerKind/3。
- prepared-intent exclusions：current D3 wire13/InputDescriptor3/`d3_identity_operation/13` 不是 D6 plan 成员；真实 wire9–12 继续历史恢复。
- plan-token exclusions：同一 current/historical 分派；D3 request 不会因此获得未声明 D6 plan token。

对应四个 current-parent Registry pointer 改为 named-current-successor，并记录精确 predecessor value/current family oracle；其它真正 exact pointer 映射保持 exact。

## 5. 证据边界

本残余修订的 FULL：本 map 实际使用的 D6 fixed/current owner 输入；全部 299 个 D6 section record 及来源限定 target；全部 760 条 fixed97 ACCEPTANCE row，其中 115 条 D6 intersection 完整精确落点；两套 Registry pointer inventory；parent Impact 89 records（85 unique ID + 4 个 range/metadata record）；以及机器 map 明确登记的真实双语 pair。

PARTIAL：只包括 D6 所需的具名 D7-D10 direct-holder/load-bearing intersection；不声称 D7-D10 完整 A2 module 已接受或全文读取。

UNREAD/pending：未列出的完整 D7-D10 owner source、Mandatory 925–1141，以及产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 证据。

P1-01 与 P2-03 继续保持 fixed01cc 独立 CLOSED，只做回归保护，不重开。上面四项残余在新的 fixed-stop 非作者复核前仅为 author-resolved-pending-independent。
