---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-DISPOSITIONS.md)

# D10 分批独立审查问题处置

revision: D10-r04-final-review-fixes-2026-09-28；状态：author revision record，等待独立复核。本文件记录前两批独立审查提出的问题、作者修改位置和仍待复核状态；它不是独立 reviewer 的关闭记录，也不是 Gate verdict。

## 1. 审查覆盖边界

被评旧候选固定为 C=`1c2bcd3e2926966c628292072c7a54a2bd5df40e`，固定上游为 U=`f205831c848729f7ddbc3ba0cf32b689459c0c98`，补充目录来自 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`。

历史上，前两批独立审查先完整读取 C 的 14/14 D10 文件，以及 U 的 D1 主文/实施、D6 主文/Control、D7 Execution/Narrow Field/Prepared Binding/Preview Effects 共 8/48；这个“8/48”只保留为过程记录。

独立评审随后已经继续完成：C 全部 14/14 候选文件、U 原 48/48 输入，以及 S 新增的 D4 reference catalog 1/1。按当前输入口径，独立 reviewer 的指定上游阅读覆盖是 **49/49，未读 0**。这仍只评价旧 C，不自动覆盖其后的作者 r02/r03/r04 修订。

旧 C 的独立结论是 **revise**。截至本轮收到的完整问题集合为 **3 个 P1 + 7 个 P2，共 10 项**：原 B1-01、B1-02、B2-01 为 P1；B1-03、B1-04、B1-05、B10-01、B11-01、B11-02、B11-03 为 P2。当前作者 r04 对这些问题进行修订，但尚未获得独立接受。

作者自己的 U 48/48 与 S 第49份阅读只是作者来源覆盖，不能替代上述独立 verdict；同样，独立 reviewer 对旧 C 的全读也不能被写成当前 r04 已通过。下表每个作者处置状态都固定为“作者已修订，等待独立复核”。
## 2. 问题处置

| ID | 原级别 | 问题 | 作者修订决定 | 主要修改位置 | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| B1-01 | P1 | D10 前置 approval 错误不能表达进入 D6 后最后次数被抢占/批准撤销的竞争，原 D6 error 闭集又没有批准错误 | 明确扩展 D6 closed enum：新增 `approval_unavailable`，disposition 只能 `preflight`。只有原 D6 permission/ObservationScope 与业务前提仍成立、关联 approval dependency 单独失效时使用。unseen 不写 ledger；planned 保持 planned。D10 adapter 逐字传递，不包装成 approval_required/expired | CANDIDATE §15.1/§21；UPSTREAM-AMENDMENTS §2.3、§3.2、§3.3；IMPLEMENTATION §11；SCENARIO A13 | 作者已修订；等待独立复核 |
| B1-02 | P1 | planned 后旧 preview token 和批准均过期，新客户端没有旧副本；现有 effects resolve/open 又只支持 committed，重新 prepare 会换 request/OperationId | 新增 proposed D7 `d7_planned_preview_open/opened` 只读入口。它在 current audience、原 ObservationScope、权限、authority/custody/continuity 通过后，从原 planned 保存的 PreparedActionBinding/2、semantic preview 和 pins 建立新的有限 recovery delivery epoch；不重跑 Query、不换 target、不复活旧 token。完整查阅后可建立 `PlannedDecisionApproval/1` 绑定 exact 原 request；确定 dependency conflict 仍不能复活 | CANDIDATE §15.2；UPSTREAM-AMENDMENTS §5；TERMINOLOGY §2/§8；SCENARIO A09；IMPLEMENTATION §7 | 作者已修订；等待独立复核 |
| B1-03 | P2 | ApprovalUse 已 reserved 后原 plan authoritative terminal_failed，没有批准次数 reservation 的终态 | ApprovalUse count 固定单向状态 `unreserved→reserved→consumed|released_terminal`。只有原 D6 authoritative terminal_failed 的同一 abort transaction 可以 `reserved→released_terminal`；历史保留，replay 不二次释放。cancel、TTL、临时撤权、Lease 过期不释放。费用 reservation 独立 | CANDIDATE §15.3/§18；UPSTREAM-AMENDMENTS §2.4–§2.5、§3.2；SCENARIO A14/F19；IMPLEMENTATION §2.1/§7 | 作者已修订；等待独立复核 |
| B1-04 | P2 | raw no-op 的实际 footprint 与 D7 owner_fields effects 为空，却被旧候选要求“恰一个 member 改变” | standing approval 明确分成 member-change 与 byte-exact raw-no-op 两个合法分支。no-op 仍验证同一唯一 Entry/member、类型、valueConstraint、当前权限、依赖与 Narrow Field Qualification，但 MutationFootprint、field_change、sourceVersions 均为空；不伪造 effect/revision。若 D6 committed 仍消费一次批准，replay 不重复 | CANDIDATE §14；UPSTREAM-AMENDMENTS §2.2/§4.2；SCENARIO A10；IMPLEMENTATION §7 | 作者已修订；等待独立复核 |
| B1-05 | P2 | 中文“能证明未发送才 release”与英文“no billable send occurred”不一致，且实际发送后可靠账单0的终态不清 | 统一费用状态：`released` 只在能证明收费执行或 billable send 从未开始；实际发送后若最终费用可靠为0，必须 `settled(0)`；可能开始但费用不可证为 `uncertain`。author terminal_failed 不自动改变费用状态 | CANDIDATE §18；TERMINOLOGY §9；SCENARIO F21/F23；IMPLEMENTATION §9 | 作者已修订；等待独立复核 |
| B2-01 | P1 | DelegationLease.maxRuns 没有唯一消费单位/准入事务；失败/取消/恢复可能重置次数 | 选择“第一次获准受保护执行即消费”语义。queued claim 和准入前取消不消费；Core-managed Run-admission CAS 按同一 `leaseId` 谱系验证 current revision、trusted time、ActivationBinding、budgets 与累计次数，并原子写 `LeaseRunUse/1`。准入后失败/取消/崩溃不退款，同 Run 恢复不重复消费。进一步明确：已有完整 `LeaseRunUse/1` 的同 Run 后续受保护步骤和原 planned 恢复不再比较 remaining count；`maxRuns=1` 已消费 1 次也能继续同 Run，但仍逐步重验当前授权、准确 revision、可信时间、ActivationBinding、批准和预算。只有新 Run 在次数耗尽时返回 `delegation_exhausted`；失权/撤销/过期/绑定变化仍阻止执行。terminal occurrence 的 claim/outcome 必须耐久防重；Lease expiry 不依赖 cleanup，时间连续性不可证返回 `state_unavailable` | CANDIDATE §8/§13/§20/§21；TERMINOLOGY §2/§8；SCENARIO U02/F03/F05/F24–F27；IMPLEMENTATION §2.1/§6/§11/§15；TASK 当前修订约束 | 作者已修订；等待独立复核 |

| B10-01 | P2 | D3 词表中 `weftext.term.origin-binding` 已拥有 `OriginBinding` / `origin_binding`，但 Adopt 又把 `adoption_binding` 写入 wire/API 变量说明和 `owned-names.codeConventions`，形成第二个绑定命名约定 | 提出最小 D3 词表勘误：Adopt 只保留 `adopt_*` code convention；任何 Adopt 关联绑定值继续使用既有 `OriginBinding(ForeignIdentityKey, NodeRef)` 类型及 `origin_binding` 名称，并归 `weftext.term.origin-binding` 所有。删除 `adoption_binding` 约定，不提供 compatibility alias、双读、迁移别名、第二 identity/wire/capability。增加正向 `adopt_*`→`OriginBinding/origin_binding` 与反向“`adoption_binding` 不得出现在受控正向面”的验证 | UPSTREAM-AMENDMENTS §6；SCENARIO U27；IMPLEMENTATION terminology/corpus；本表 | 作者已修订；等待独立复核 |

| B11-01 | P2 | SCENARIO-DISPOSITIONS 多个 Intake 来源定位错引或把候选推导攻击写成直接 A2 条目 | 对 §1 全部 U01–U27 重新逐行核对 Mandatory Intake 与上游原合同。U05/U06/U07/U18 改到实际章节；U08–U21 的 §9 编号逐项复核；U17 明确标为“候选推导攻击”而非 Intake 原文。新增来源类别说明，不删除任何原场景；Pack lifecycle 另增 P10–P15 | SCENARIO-DISPOSITIONS §1/§6/§7；Candidate §6.1 | 作者已修订；等待独立复核 |
| B11-02 | P2 | D10 Lexicon 只有概念表/部分 type，未满足 Intake §8.5.1 每概念 13 项可追踪映射 | TERMINOLOGY 新增逐概念结构化映射，覆盖现有全部 D10 受控概念并补 Pack、Bundled Module、Agent Session、Parent Extension Dependency；每项固定 stable ID、中英正式名、定义、owner、排除、wire/API/manifest/schema 状态、代码约定、CLI/UI+locale、简称、禁止/历史 alias、正反例、首次冻结/状态/迁移。另列 D1–D9 继承名称/collision，未公开 IPC/无 CLI 明确写无，不新增实现资源 | TERMINOLOGY §2、§4–§5、§13；IMPLEMENTATION terminology gates | 作者已修订；等待独立复核 |
| B11-03 | P2 | Pack 父领域/extension-point 依赖、版本、disable/UI-hidden/surface lifecycle 未闭合；holiday 算法延期被错误当成通用生命周期答案 | 选择“D4 定义/历史保留、D10 contribution 激活、UI 可见性三轴分离”。domain Pack 必须声明 primary parent domain/extension point + version range；Activation 时 pin 准确 parent binding。missing/disabled/incompatible/unsupported_surface 分别产生唯一 inactive 结果并投影既有 D1 reason；UI hidden-only 不改 activation。已接受 D4 schema/history/raw source 保留，typed interpretation 只依 current Registry complete/unavailable；parent inactive 不得暗中运行 View/Action/rule/connector | CANDIDATE §6.1；TERMINOLOGY §4–§5/§13；SCENARIO U07/U19/P10–P15/D01；IMPLEMENTATION §3；UPSTREAM-AMENDMENTS Pack non-amendment note | 作者已修订；等待独立复核 |

## 3. 保持不变的边界

本轮没有放宽自动作者写入范围。首版仍只允许 D7 `set_field_member` 的 single_field_member profile；D3 create/lifecycle、D8 document/annotation edit、bulk/Facet/native-table 等仍需要交互确认或保持不支持。

Core 仍是唯一 author transaction authority。ApprovalUse、LeaseRunUse、PlannedDecisionApproval、Run 与 ExternalEffectIntent 都是受管控制证据，不是第二 ledger 或第二 author receipt。

外部结果 `outcome_unknown` 与费用 `uncertain` 保持；没有为了修复批准/Run 语义而改成自动重试或自动退款。

D6/D7 的新增条款全部只存在 UPSTREAM-AMENDMENTS 提案中。固定上游 U 没有被修改；修订在独立接受与协调激活前不生效。

## 4. 文档质量修订

上一版中文为了满足混排检查，重复追加了“技术名称不扩大权限”等泛句，并产生双句号。它们不是规范语义。本轮删除这些填充句，改为在对应段落直接用中文解释真实边界，同时保留必要的受控英文标识。仓库 check_docs 规则不放松；若仍有混排失败，只修改实际文案，不通过重复填充绕过。

## 5. 后续独立复核目标

后续 reviewer 至少应对本表六项重新构造原反例，并检查：

- `approval_unavailable/preflight` 是否是 D6 owner 下唯一、非永久且不泄漏的 race 结果；
- planned-preview recovery 是否真正只读原保存语义，并与 committed effects transport、旧 preview TTL 和 current authorization 一致；
- released_terminal 是否只在 authoritative abort 发生，并与费用状态完全分开；
- raw no-op 是否没有伪造 footprint/effect/version；
- settled(0)、released、uncertain 是否互斥且恢复一致；
- LeaseRunUse、occurrence claim、terminal outcome、trusted time 是否能阻止重启/重扫/启停造成次数重置或重复 Run。

本文件不能把“作者已修改”写成“独立已关闭”。独立上游指定阅读现在为 49/49、未读 0；剩余工作是对当前 r04 作者修订做独立复核，而不是补旧 C 的输入阅读。只有独立 reviewer 重新裁决当前修订后，才能形成最终 D10 Gate 结论。
