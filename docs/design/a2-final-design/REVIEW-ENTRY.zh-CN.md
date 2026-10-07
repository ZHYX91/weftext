---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)
# A2 D1–D6 作者候选复核入口——当前残余交回

状态：仅作者候选；未独立接受、未实现、未合并、未发布、未部署、未全局冻结。

## 1. 固定对象与作者范围

Base branch：docs/asciidoc-annotation-final-design。
固定 base SHA：97f4734f82a760cb6716c8122b84494da2b61164。
固定历史输入 S：7e18168dad3e6d120fce0dd607dc10fa7894e252。
受保护 inputs blob：787d03c31a55496f81ed03fd54a6fdfff50a2ad4。
候选 branch：docs/a2-final-design-integration。
D6 四残余修订从精确作者 head `32cfb9c387deddb12fb021a44147df0d7ffab322` 开始；后续这批 D4/D5 R1 映射 + D6 source-map hash 窄修从 `d1ab2a5c0762450877614ad14340a26cfa0c1dfd` 开始。

下一位非作者复核者必须绑定 PR metadata/交接记录的实际 final stop SHA，不得追 moving branch。

## 2. 本修订之外保留的独立状态

- 有界 D1/D2 findings 继续 independent CLOSED。
- 有界 D3 P1-01/P1-02 继续在 fixed-a62 independent CLOSED。
- D4/D5 P2-02 只在 fixed4282 的六份真实 D10 来源资格有界范围 independent CLOSED。
- fixed32cf 对 D4/D5 P2-01 给出 REVISE P0=0/P1=0/P2=1。R2 已独立 CLOSED；R1 在 202 条中通过 188 条，留下 14 条残余（D4 1 条、D5 13 条）。这 14 条映射已在本 follow-up 修订，仍只为 author-resolved-pending-independent；它们不是 D6 finding。

全局 A2 未 accepted。

## 3. D6 固定复核对象

D6 原五项 finding 的复核对象是 fixed829 `829efce6aacbe944714e093c98065b01d50b2593`；现在只保留为 finding-origin provenance。

后续独立 D6 复核对象是 fixed01cc `01cc40b819df78fbe724f1c64c27284ad60fc6c8`，结论 REVISE。

fixed01cc 已独立 CLOSED：
- **A2-D6-829-P1-01**——唯一的当前新路径为 Key3→Proof3→Descriptor3→Prepared3 / Notice3 / 原 final P / CP4+ChangeRecord1，并覆盖已复核的信任、引导、副本与执行回归。
- **A2-D6-829-P2-03**——schedule capacity 单位与 gap 顺序。

fixed01cc 仍 OPEN/PARTIAL，本次修订：
- **A2-D6-829-P1-02**——中文十五-arm closed-union 基数。
- **A2-D6-829-P2-01**——完整 source→current-owner/anchor 与真实双语来源证据。
- **A2-D6-829-P2-02**——四个通用 Registry 字段和精确 pointer disposition。
- **A2-D6-01CC-P2-01**——当前固定对象导航。

这四项只标 **author-resolved-pending-independent**，不得自行 CLOSED。

## 4. 四项残余的复核目标

### P1-02

核 D6-CONTROL 中英文都明确 current `DependencyKey/3` 恰为 15 arm；rank 0..14，`document_format=1`，十五个列举 shape 一致。`DependencyProof/3`、`InputDescriptor/3`、`PreparedIntent/3`、`d6_plan/3` 必须属于同一个 fresh-current family。双语 D6-IMPACT 的 `D6-DK-CARD-01` 是编号基数 oracle。真实 Key2/Proof2 继续十四-arm historical。

直接回归还必须保持：实际消费 managed Document 语义时，M1→M2 即使 source bytes/SourceVersion 相同也使 Proof3 stale；不消费者不伪造 dependency；format-only transition 的 `sourceChanges=[]`，且不推进 SourceRevisionPlan/managed SourceVersion/H。

### P2-01

必须审机器 map 本身：
- **299/299** section mapping 均有来源限定的具体 current target；原 23 个 fixed-S Control 精确 mapping 不变；
- 其余 **276** 个 section mapping 指向真实 owner/consumer 的 file+anchor，不再整文件兜底；
- fixed97 ACCEPTANCE **760/760** 原 row/source text 保留，**115** 个 D6 intersection 保留完整 EN/ZH row；
- 5 条原精确 fixed97 target 保持不变，其余 **110** 不再使用 generic D6-IMPACT placeholder；
- parent/fixed97/current-A2 记录真实中英文 path+blob pair；fixed-S snapshot 保持实际存在的单份受保护 source，不编造双语 snapshot/JSON twin；
- current D6 main 真实 blob 是 `50632ba345aeab1e28970e219ec53a7fceea5a3e` / `3a4bceae305d30f72b42866f468d4a911dae7f0a`；
- 本 follow-up 起点的 `D6-SOURCE-MAP.zh-CN.md` 真实 blob 为 `d297f52153492e3b392f3b972d39001fbf1b405b`；D6-SOURCE-MAP.json 的 `current_candidate_blobs.source_map_human_zh` 与 current human-map 双语 pair 必须都记录该真实 blob。这只是机械证据修正，不新增 D6 finding。

fixed97 的 D6 直接所有者完整合同继续保留在 Control §17.1–§17.7：ResultPage、12 成员的 BudgetBinding、ImportJob、受管配置/控制读取、ByteHandle/ByteRead、读取前授权/ObservationScope、SourceBinding/OriginBinding。

### P2-02

只审四个通用 Registry 字段：
- `/concepts/5/exclusions`
- `/concepts/6/exclusions`
- `/concepts/15/definition`
- `/concepts/44/definition`

当前 D3 使用 wire13 + InputDescriptor/3 + `d3_identity_operation/13`；真实 wire9–12 继续按既有历史记录解释，`d6_commit_request/2` 仍是 D6 提交入口。冲突解析的当前外层使用 Descriptor3/Proof3/Prepared3；来源合并和选择来源头分支保留内层 Input2/Plan1/Preview1 + ownerKind/2；策略包选择分支使用内层 Input3/Plan2/Preview2 + ownerKind/3。

D6-REGISTRY 必须继续保持 52 个概念和 17 个跨阶段绑定，concept ID、owned names、aliases、locale、firstFreeze 不变。Registry 指针清单继续为 767 个 fixed-S 项与 1246 个 current-parent 项；只新增这四个 parent-current named-current-successor。

### 导航 P2

A2 README/REVIEW-ENTRY/SOURCE-MAP、D6-SOURCE-MAP 与 PR metadata 必须区分：
- fixed829 = finding origin；
- fixed01cc = 独立 D6 复核对象；
- fixed32cf = 当前作者修订起点；
- P1-01/P2-03 = fixed01cc independent CLOSED；
- 四项残余 = author-resolved-pending-independent；
- 不构成 D6/global acceptance。

## 5. 证据边界与未运行范围

本修订 FULL 仅限 D6 fixed/current-owner 输入、全部 299 section mapping、全部 760 fixed97 row 及其中 115 D6 intersection、两套 Registry pointer inventory、parent Impact 89 records（85 unique ID + 4 intentional repeat/range-metadata record），以及 D6-SOURCE-MAP 明确登记的双语 pair。

D7–D10 只在 D6 具名 load-bearing intersection 上 PARTIAL；完整 D7–D10 A2 module 与 Mandatory 925–1141 仍 pending。

产品/runtime/OS/GUI/crypto/真实 replica/crash/provider/performance/migration/activation/deployment 行为证据全部 UNRUN。文档/机器检查只证明仓库一致性，不替代语义接受。

D6 仍有独立 OPEN P1 时不得开始 D7。最终 A2 仍需完整 D7–D10/Mandatory 整合、fresh Pro/global 非作者终审、一个 explicit accepted-design SHA，以及同一 accepted SHA 上的 freeze/implementation-start 材料。
