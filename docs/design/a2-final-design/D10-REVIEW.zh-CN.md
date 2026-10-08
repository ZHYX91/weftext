---
source_language: zh-CN
translation_status: source
---

[English](D10-REVIEW.md)

# D10 独立审查问题处置

## A2 D10 评审证据当前读法

本文件**逐字保留**原 R06/R07/R08 与历史 B/R05 作者处置表，作为来源资格，不把旧条目“OPEN/待复审”直接当成当前 fixed8d7 的新 finding，也不因作者落地自行 CLOSED。fixed8d7 非作者已仅对 D9 完整差分给出 ACCEPT 0P0/0P1/0P2，必要读 gap0；所有旧 D9 个别问题按对应 SHA CLOSED。**D10 完整 18 文件 current+A2 direct owner 的非作者复核尚未执行**；所有本节旧作者处理只代表历史提交归属，不能汇总为 D10/global PASS。新的 D10 作者候选仍为 author-resolved-pending-independent-review，需将 R08 十一项逐原证据和后续受限独立结论核销，不得机械删除历史 ID。

---


revision: D10-FA-r01-2026-10-02；状态：协调作者候选，未接受、未激活、未实现。最近一次完整历史 R08 评审绑定 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 与 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，结论 REVISE（P0=0、P1=3、P2=8）。十一项历史最终处置仍为 OPEN。具名修订已有限定独立复核，实际跨 owner 整合及 fresh 全局接受仍未完成；D10-REVIEW 区分各层证据。


## 1. 审查覆盖边界

历史固定 C=`35fab950dabedfb92c9f12858701be8afe6faa74` 已完成对应上游审查并以 REVISE 结束。固定 R06 C=`32a0868ae9443a3f839cfb4f5e9bbcace308314d` 的完整独立审查完成候选18/18、S49/49，并以 JR001 P1、JR002–JR009 P2 的 REVISE 结束；这只是历史记录，不能视为 R07 或 R08 已接受。

随后完整独立联合终审固定 R07 C=`cf46461848d5dfe4dcd0f482ede934243cd098a4` 与 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，完成**候选18/18、S49/49，规范正文无阅读缺口**，最终 **REVISE**：P0=0、P1=1（`B03-P1-01`）、P2=10（`B01-P2-01`、`B02-P2-01`、`B02-P2-02`、`B02-P2-03`、`B03-P2-01`、`B03-P2-02`、`B03-P2-03`、`B03-P2-04`、`B10-P2-01`、`B11-P2-01`）。整体、术语与中英语义均需修订；D8、D9 各新增一项当前 finding。

R08 是对这组固定 R07 问题的作者修订。旧 R06/R07 覆盖只作为历史证据，不能转记为最终 R08 commit 已被独立阅读、通过或关闭。

### 1.1 后续 R08 评审与当前修订范围

后续完整评审绑定 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 与同一 S，有其独立的候选18/18、S49/49阅读记录，结论 REVISE，术语/双语 FAIL：P0=0/P1=3/P2=8。下列十一项取代了当时的 R07 当前问题表；旧九项在历史范围内关闭，两项由新具体发现承接，不能相加为二十二项当前问题。§2–4 保留早期作者处置及其当时状态，不是今日评审结果。

下表具名修正均有绑定其实际修订字节的限定独立复核；后续变更及实际 owner 生产端另须审查。十一项历史最终处置在 fresh 全局接受前仍为 OPEN；限定 PASS 不能转给其它全文 hash。

| R08 稳定 ID | 原级别 | 实际反例与当前修订落点 | 修订范围 |
| --- | --- | --- | --- |
| R08-B13-P1-01 | P1 | 模型声称收件人 A，实际 target=B/payload=P；CONTROL §7 要求完整受信请求查阅与精确确认 | 限定 PASS；D6 生产端整合待完成 |
| R08-B13-P1-02 | P1 | horizon/cache/源变化令同一发生重复，或不同 originalStart 合并；CONTROL §16 定义稳定键、连续性、armed/claim 及整个遗漏窗口 | 限定 PASS；实际 D6 历史留存/消费待完成 |
| R08-B13-P1-03 | P1 | 90单位 uncertain attempt 在版本变化/重启后失去原预算层归属；CONTROL §10 保留不可变五/六层归属及一次结算 | 限定 PASS；全局费用/恢复审查待完成 |
| R08-B13-P2-01 | P2 | 首次 host-control 成功没有唯一公开 envelope；CONTROL §7 固定首次与重放使用同一 applied-history 成功结果 | 限定 PASS |
| R08-B01-P2-01 | P2 | 用户直接发起交互 Run，没有 Automation；CONTROL RunOrigin/1 与准入/当前投影现有真实交互分支 | 限定 PASS |
| R08-B02-P2-01 | P2 | 公开 PackageManifest/Contribution 被误分为未定义 IPC；D10-LEXICON §13 明确实际公共载体 | 限定 PASS |
| R08-B02-P2-02 | P2 | 只允许 publisher 的术语排除合法第一方 people namespace；D10-LEXICON 保留 D4 精确第一方 owner claim | 限定 PASS |
| R08-B02-P2-03 | P2 | 多个 kind 可以属于同一接口 owner；SCENARIO P18 和 UPSTREAM §8 要求单值映射而非双射 | 限定 PASS |
| R08-B05-P2-01 | P2 | never_bound 显式 Adopt 被误拒；SCENARIO U12 保留真实 D3 正向分支，不开放 ICS | 限定 PASS |
| R08-B11-P2-01 | P2 | 文本替换响应被错误要求包含 caret；UPSTREAM §8 消费 D8 精确响应 | 限定 PASS |
| R08-B12-P2-01 | P2 | import_next 被错误要求返回终态；UPSTREAM §8 保留 import_state 查询终态 | 限定 PASS |

新文件权威 owner 后像、公开载体映射修正及 D6/D7/D10 版本/提交/恢复协调是本轮新增工作。落盘本身不关闭 PL 或当前 owner 问题，也不能代替当前候选全文阅读记录。各 actor 分别记录真实全文和依赖局部阅读；本轮不继承旧 actor 计数冒称 S49/49。

## 2. 历史 R06→R07 JR001–JR009 作者处置

| ID | 级别 | 历史作者修订 | 历史 R07 主要落点 | 验证目标 | R07 形成时记录状态 |
| --- | --- | --- | --- | --- | --- |
| JR001 | P1 | CONTROL 唯一拥有完整 SingleFieldMemberRule；保留 semantic_code、0..64 同型 TypedLiteral enum、numeric range、bounded exact text、1..7 D4 member path、Optional-present bridge、member-change/raw-no-op | CONTROL §7；D10 §14；UPSTREAM approval 消费面；Implementation；场景 | 真实 `people/phone.label` present Optional semantic-code change + raw-no-op + 第二同值 phone 负例；禁止 whole-source widening | 作者已修；待独立复核 |
| JR002 | P2 | 历史 prepare/applied result 与 current exact ControlRef read 分开；闭合19类 current projection、scope、lifecycle/config/domain/usage revision 与 non-disclosure | CONTROL §7–§8；Candidate recovery/error；Implementation/F32 | r5丢响应/r6 current：result=r5、current read=r6；secret 无明文；planned preview 只走D7；applied连续性不明=`state_unavailable` | 作者已修；待独立复核 |
| JR003 | P2 | `D10ControlError/1` 管理域与 `D10RunStepError/1` 分离；逐入口映射并保留 D3/D6/D7/D8/D9 原 envelope | CONTROL §1；D10 §21；Implementation | management stale_revision != runtime binding_changed；D6前 approval error 不包装 D6 approval_unavailable | 作者已修；待独立复核 |
| JR004 | P2 | 分别登记 ToolValueProfile/ToolType/ToolValue，明确 D10 自有 closed algebra、非 D7 alias | CONTROL §3；D10-LEXICON §2/§3/§13；D10-READING/Implementation | controlled surface 不再写复用 D7 subset；exact numeric/optional/object/list/union decoder | 作者已修；待独立复核 |
| JR005 | P2 | 一份 CostReservation 仍是一 attempt/account/grant/pricing/currency；多层 limit 是 ceiling；真实多账户费用用独立可归属 reservation | CONTROL §10；D10 §18；D10-LEXICON；F20；Implementation | 同一费用不重复；多层容量竞争；实际多账户各自归属 | 作者已修；待独立复核 |
| JR006 | P2 | 统一 R07、九对18路径、完整 review 历史与 S 四个补充路径 | 全部状态头；D10-TASK；D10-READING；本文件 | 不残留阶段性阅读/seven-pair/r04/当前R06 状态；只有 catalog 是第49规范源 | 作者已修；待独立复核 |
| JR007 | P2 | ImportJob 继续归 D6，保留原 concept ID、ownedNames、`storage.import_job`、firstFreeze；D9 只消费 | UPSTREAM §8.2；D10-LEXICON §13.2/§15 | 不再有 D9 new/split ownership 或 D9 firstFreeze | 作者已修；待独立复核 |
| JR008 | P2 | NamespaceClaim 直接消费 exact D4 owner tuple；ownerId 绝不是 D6 Token，proof 独立 | CONTROL §9；Candidate trust；Implementation | 正例 people 保留 tuple；随机43字符 Token/同名 package/install-order 负例 | 作者已修；待独立复核 |
| JR009 | P2 | D7 SearchContribution 在 `view` 下有唯一 D10 纯数据 carrier，ID/版本分域、descriptor digest/owner/Registry proof、完整 Catalog generation binding | CONTROL §4；D10 §6；D10-LEXICON 继承表；U18/U22；Implementation | 合法第一方 install→activate→D7 search；wrong owner/digest/duplicate/omission/unavailable/script 负例 | 作者已修；待独立复核 |

此前两项已澄清问题保持关闭且不改设计。owner/组合核验已完整完成；R07 不再保留“其它 owner 组合仍待核”的笼统声明。 本表为历史记录；上文固定 R07 的后续完整评审已经取代其当时的 pending 状态。

## 3. 历史 R07→R08 B01–B11 作者处置

| ID | 级别 | 固定 R07 C 原 finding 定位 | 已落盘 R08 作者修订 | R08 主要 owner / consumer | fresh 复核验证目标 | 当前状态 |
| --- | --- | --- | --- | --- | --- | --- |
| B03-P1-01 | P1 | CONTROL EN452–473；D10 §15 | 新增具名第一方 `CoreFieldMemberAdapterDescriptor/1`、closed `FieldMemberTask/1`，闭合 task→fresh Field→原 D7 FieldSelector→`set_field_member`→prepare/preview/MutationFootprint→ApprovalUse→原 D6 request 映射，并用受保护 `D10AuthorPreparationLink/1` 恢复原请求。task intent 与 Standing Approval 继续分域。 | CONTROL §4/§7；D10 §§13–15；SCENARIO 自动作者场景；IMPLEMENTATION §§4.1/7/15 | 真实 `people/phone.label` present Optional semantic-code `personal→work`；逐字 `work→work`；第二条同值 phone 自动不适用但交互路径可用；冷启动恢复原 request，continuity unknown 不产生新 OperationId | 作者修订已落盘；待 fresh 独立复核 |
| B01-P2-01 | P2 | CONTROL EN926–962、EN622–636 | 闭合 exact-target stop 成功/查询：`requestId==target.id`、预留 latch/result/safety-sequence capacity、不可逆 revision-2 receipt、只读 lost-response result query、W/H 当前 authority/continuity 顺序，并明确不走普通 prepare、不造 D6 author receipt 或第二成功账本。 | CONTROL §1/§7/§11；D10 §20；UPSTREAM D6/final/send consumers；SCENARIO F34/F35；IMPLEMENTATION §11.1/§15 | 首次 stop、丢响应后 target r5→r6 重试、W/H 有权读取同一 latch、hidden target、continuity unknown、MAX/预留容量边界以及 admission/final-commit/send 竞争 | 作者修订已落盘；待 fresh 独立复核 |
| B02-P2-01 | P2 | D10-LEXICON EN204/214/234/238 | 修正 public-wire 分类：Money 与 ActivationBinding 是有具体 carrier 的 closed public value；完整 ExternalEffectIntent/ExternalExecutionBinding 是受保护内部记录，`ExternalEffectCurrentView/1` 才是 public current projection。新 adapter/history/stop public carrier 另行登记。 | D10-LEXICON §13；CONTROL §§2/7/8/11；D10；IMPLEMENTATION | 每个具名 value/projection 都能定位唯一 carrier/owner；“无独立 RPC”不等于“无 public wire”；public value 本身不授 write/secret 权限 | 作者修订已落盘；待 fresh 独立复核 |
| B02-P2-02 | P2 | SCENARIO EN13/32/115/133 | 按真实首代分支重分 U12/T02/E05。U12 分开 unsupported ICS、已支持普通显式 fresh import 与 D3 binding/Adopt 状态；T02 pending admission 是自动状态处理但不使工具 callable；E05 对同一 immutable external request 保持 `outcome_unknown`，回读 miss 既不是成功也不是安全重发依据。 | SCENARIO U12/T02/E05/F17；D10 §17；IMPLEMENTATION §15 | 不凭空开放 ICS；独立 ordinary import 保持 fresh；同 request 只恢复原结果；active_live 不自动 upsert；retired 需显式 Adopt；pending tool 不可运行；unknown effect 不换 request/key | 作者修订已落盘；待 fresh 独立复核 |
| B02-P2-03 | P2 | CONTROL EN553–568、EN629–636 | 内部继续保存完整 canonical B 用于 stable-key equality；public prepared history 改为七种 operation kind 的 `ControlPreparedHistory/1` 摘要。历史 result 不返回嵌套作者请求 A、完整 B 或生成 submit request M；首次 prepare 仅在当前披露覆盖 B 与嵌套 A 后交付 M。已 applied 的 prepare replay 返回 historical applied arm，不伪装 fresh prepared。 | CONTROL §7–§8；SCENARIO F32；IMPLEMENTATION §§2.1/4.1/15 | 同完整 B 防重、same key/different B 冲突；r5 saved history 与 current r6 分开；当前授权先行；applied linkage 不可证明为 `state_unavailable`，不降格 prepared | 作者修订已落盘；待 fresh 独立复核 |
| B03-P2-01 | P2 | CONTROL EN706–707、EN943–949；D10 EN388–400 | 闭合 external-effect 返回树：immutable `FrozenEffectBytes/1` / `ExternalEffectIntent/1`、准确 `ExternalExecutionBinding/1`、stable effect Ref + requestDigest consent、closed `ExternalEffectCurrentView/1`，并定义 `HostOrWorkspacePrincipal/1`。lifecycle revision 不再兼任 frozen request identity。 | CONTROL §§2/7/11；D10 §17；SCENARIO E05/F17/F35；IMPLEMENTATION §§8/11.1/15 | current projection 不泄露 payload/target/key/proof/secret/reservation identity；prepared→submitting 不使 consent 自失效；frozen bytes/target 不可替换；sendAttemptId 与 billable attempt 分域；D9 worker 不继承 D10 egress | 作者修订已落盘；待 fresh 独立复核 |
| B03-P2-02 | P2 | D10-READING EN106 / CN105 | R08 中英统一说明 `ToolValueProfile/1`、`ToolType/1`、`ToolValue/1` 是 D10 自有受限代数；具名 Core field-member adapter 明确消费原 D7 TypedLiteral/ResolvedCodeScope；普通 ToolValue 不能成为 NodeRef、FieldId、selector 或 author request。 | D10-READING §3/§5；CONTROL §3/§4/§7；D10-LEXICON；D10 §§13–15 | 中英 owner 声明与 CONTROL 一致；旧“复用 D7 有限子集”只能作为明确已取代历史或被删除 | 本 R08 批次作者修订已落盘；待 fresh 独立复核 |
| B03-P2-03 | P2 | REVIEW EN91 / CN90；CN81 | 删除过时“仍在读旧 C”“六项”和未来 R05 当前态，改为固定 R07 C18/18+S49/49 已完成 REVISE 的记录，并新增当前 11 finding R08 作者处置表。旧 R06/R07 记录继续明确为历史。 | REVIEW §§1–7；D10-TASK/D10-READING 交接 consumers | 当前 ID 恰为11项、1个P1+10个P2；当前文本不再声称旧 review 仍在运行，也不把 R08 写成独立接受 | 本 R08 批次作者修订已落盘；待 fresh 独立复核 |
| B03-P2-04 | P2 | D10-TASK EN19/CN18；D10-READING EN17/CN16；IMPLEMENTATION §16 | 阅读证据分账：原作者 lineage 历史 S49/49；历史 R08 接续作者全文 S16/49；D9 workers/export、templates 只算依赖局部；固定 R07 独立评审另行 S49/49。三套证据互不继承。 | D10-TASK Fixed Inputs/Completion/Review；D10-READING §§1–2/6；IMPLEMENTATION §16 | 三套覆盖始终分开；本 R08 批次不增加 S 全文计数；workflow/tool summary 不算全文 | 本 R08 批次作者修订已落盘；待 fresh 独立复核 |
| B10-P2-01 | P2 | UPSTREAM EN309/311/320–325；CN308/310/319–324；SCENARIO P18 | 把含糊 kind→concept 共 owner 改成每个 D8 kind 一个 technical-interface owner。13 kind 全部映射；Draft/Draft Projection/Draft Edit Map/Prepared Edit Binding 与上游 value 只是 consumes/returns 数据。D8 domain concept 仍恰为九个；wire/IME/confirm/Undo 不改。 | UPSTREAM §8.1；SCENARIO P18；IMPLEMENTATION §§13/15 | 精确 13-kind 唯一 owner 扫描；`d8_draft_text_replace/write` 消费 Draft Edit Map 但不共享 kind owner；dirty Draft 允许合法后台 D7 commit 并按 stale/rebase 处理；Undo 不回滚后续后台 commit | 作者修订已落盘；待 fresh 独立复核 |
| B11-P2-01 | P2 | UPSTREAM EN335/386–392；CN334/385–391；SCENARIO P19 | 为17个 D9 public kind 各给一个 technical-interface owner，并把 consumes/returns/operates-on 分开。36个 D9-owned naming row 继续归 D9；继承 `weftext.term.import-job` 保留 D6 concept ID、ownedNames、`storage.import_job` 与历史 firstFreeze。D2/D3/D6/D7 名称保留原 owner。 | UPSTREAM §8.2；SCENARIO P19；IMPLEMENTATION §§13/15 | 精确 17-kind 唯一 owner + 36-D9/1-inherited-D6 naming 检查；`d9_import_state` operates on D6 ImportJob 但不拥有它；PublicationReceipt 只证明 publication；D7ResultPin nested schema/V 继续归 D7 | 作者修订已落盘；待 fresh 独立复核 |

## 4. 历史分批问题处置

| ID | 原级别 | 问题 | 作者修订决定 | 主要修改位置 | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| B1-01 | P1 | D10 前置 approval 错误不能表达进入 D6 后最后次数被抢占/批准撤销的竞争，原 D6 error 闭集又没有批准错误 | 明确扩展 D6 closed enum：新增 `approval_unavailable`，disposition 只能 `preflight`。只有原 D6 permission/ObservationScope 与业务前提仍成立、关联 approval dependency 单独失效时使用。unseen 不写 ledger；planned 保持 planned。D10 adapter 逐字传递，不包装成 approval_required/expired | D10 §15.1/§21；D10-UPSTREAM §2.3、§3.2、§3.3；IMPLEMENTATION §11；SCENARIO A13 | 作者已修订；等待独立复核 |
| B1-02 | P1 | planned 后旧 preview token 和批准均过期，新客户端没有旧副本；现有 effects resolve/open 又只支持 committed，重新 prepare 会换 request/OperationId | 新增 proposed D7 `d7_planned_preview_open/opened` 只读入口。它在 current audience、原 ObservationScope、权限、authority/custody/continuity 通过后，从原 planned 保存的 PreparedActionBinding/2、semantic preview 和 pins 建立新的有限 recovery delivery epoch；不重跑 Query、不换 target、不复活旧 token。完整查阅后可建立 `PlannedDecisionApproval/1` 绑定 exact 原 request；确定 dependency conflict 仍不能复活 | D10 §15.2；D10-UPSTREAM §5；D10-LEXICON §2/§8；SCENARIO A09；IMPLEMENTATION §7 | 作者已修订；等待独立复核 |
| B1-03 | P2 | ApprovalUse 已 reserved 后原 plan authoritative terminal_failed，没有批准次数 reservation 的终态 | ApprovalUse count 固定单向状态 `unreserved→reserved→consumed|released_terminal`。只有原 D6 authoritative terminal_failed 的同一 abort transaction 可以 `reserved→released_terminal`；历史保留，replay 不二次释放。cancel、TTL、临时撤权、Lease 过期不释放。费用 reservation 独立 | D10 §15.3/§18；D10-UPSTREAM §2.4–§2.5、§3.2；SCENARIO A14/F19；IMPLEMENTATION §2.1/§7 | 作者已修订；等待独立复核 |
| B1-04 | P2 | raw no-op 的实际 footprint 与 D7 owner_fields effects 为空，却被旧候选要求“恰一个 member 改变” | standing approval 明确分成 member-change 与 byte-exact raw-no-op 两个合法分支。no-op 仍验证同一唯一 Entry/member、类型、valueConstraint、当前权限、依赖与 Narrow Field Qualification，但 MutationFootprint、field_change、sourceVersions 均为空；不伪造 effect/revision。若 D6 committed 仍消费一次批准，replay 不重复 | D10 §14；D10-UPSTREAM §2.2/§4.2；SCENARIO A10；IMPLEMENTATION §7 | 作者已修订；等待独立复核 |
| B1-05 | P2 | 中文“能证明未发送才 release”与英文“no billable send occurred”不一致，且实际发送后可靠账单0的终态不清 | 统一费用状态：`released` 只在能证明收费执行或 billable send 从未开始；实际发送后若最终费用可靠为0，必须 `settled(0)`；可能开始但费用不可证为 `uncertain`。author terminal_failed 不自动改变费用状态 | D10 §18；D10-LEXICON §9；SCENARIO F21/F23；IMPLEMENTATION §9 | 作者已修订；等待独立复核 |
| B2-01 | P1 | DelegationLease.maxRuns 没有唯一消费单位/准入事务；失败/取消/恢复可能重置次数 | 选择“第一次获准受保护执行即消费”语义。queued claim 和准入前取消不消费；Core-managed Run-admission CAS 按同一 `leaseId` 谱系验证 current revision、trusted time、ActivationBinding、budgets 与累计次数，并原子写 `LeaseRunUse/1`。准入后失败/取消/崩溃不退款，同 Run 恢复不重复消费。进一步明确：已有完整 `LeaseRunUse/1` 的同 Run 后续受保护步骤和原 planned 恢复不再比较 remaining count；`maxRuns=1` 已消费 1 次也能继续同 Run，但仍逐步重验当前授权、准确 revision、可信时间、ActivationBinding、批准和预算。只有新 Run 在次数耗尽时返回 `delegation_exhausted`；失权/撤销/过期/绑定变化仍阻止执行。terminal occurrence 的 claim/outcome 必须耐久防重；Lease expiry 不依赖 cleanup，时间连续性不可证返回 `state_unavailable` | D10 §8/§13/§20/§21；D10-LEXICON §2/§8；SCENARIO U02/F03/F05/F24–F27；IMPLEMENTATION §2.1/§6/§11/§15；D10-TASK 当前修订约束 | 作者已修订；等待独立复核 |

| B10-01 | P2 | D3 词表中 `weftext.term.origin-binding` 已拥有 `OriginBinding` / `origin_binding`，但 Adopt 又把 `adoption_binding` 写入 wire/API 变量说明和 `owned-names.codeConventions`，形成第二个绑定命名约定 | 提出最小 D3 词表勘误：Adopt 只保留 `adopt_*` code convention；任何 Adopt 关联绑定值继续使用既有 `OriginBinding(ForeignIdentityKey, NodeRef)` 类型及 `origin_binding` 名称，并归 `weftext.term.origin-binding` 所有。删除 `adoption_binding` 约定，不提供 compatibility alias、双读、迁移别名、第二 identity/wire/capability。增加正向 `adopt_*`→`OriginBinding/origin_binding` 与反向“`adoption_binding` 不得出现在受控正向面”的验证 | D10-UPSTREAM §6；SCENARIO U27；IMPLEMENTATION terminology/corpus；本表 | 作者已修订；等待独立复核 |

| B11-01 | P2 | D10-SCENARIOS 多个 Intake 来源定位错引或把候选推导攻击写成直接 A2 条目 | 对 §1 全部 U01–U27 重新逐行核对 Mandatory Intake 与上游原合同。U05/U06/U07/U18 改到实际章节；U08–U21 的 §9 编号逐项复核；U17 明确标为“候选推导攻击”而非 Intake 原文。新增来源类别说明，不删除任何原场景；Pack lifecycle 另增 P10–P15 | D10-SCENARIOS §1/§6/§7；Candidate §6.1 | 作者已修订；等待独立复核 |
| B11-02 | P2 | D10 Lexicon 只有概念表/部分 type，未满足 Intake §8.5.1 每概念 13 项可追踪映射 | D10-LEXICON 新增逐概念结构化映射，覆盖现有全部 D10 受控概念并补 Pack、Bundled Module、Agent Session、Parent Extension Dependency；每项固定 stable ID、中英正式名、定义、owner、排除、wire/API/manifest/schema 状态、代码约定、CLI/UI+locale、简称、禁止/历史 alias、正反例、首次冻结/状态/迁移。另列 D1–D9 继承名称/collision，未公开 IPC/无 CLI 明确写无，不新增实现资源 | D10-LEXICON §2、§4–§5、§13；IMPLEMENTATION terminology gates | 作者已修订；等待独立复核 |
| B11-03 | P2 | Pack 父领域/extension-point 依赖、版本、disable/UI-hidden/surface lifecycle 未闭合；holiday 算法延期被错误当成通用生命周期答案 | 选择“D4 定义/历史保留、D10 contribution 激活、UI 可见性三轴分离”。domain Pack 必须声明 primary parent domain/extension point + version range；Activation 时 pin 准确 parent binding。missing/disabled/incompatible/unsupported_surface 分别产生唯一 inactive 结果并投影既有 D1 reason；UI hidden-only 不改 activation。已接受 D4 schema/history/raw source 保留，typed interpretation 只依 current Registry complete/unavailable；parent inactive 不得暗中运行 View/Action/rule/connector | D10 §6.1；D10-LEXICON §4–§5/§13；SCENARIO U07/U19/P10–P15/D01；IMPLEMENTATION §3；D10-UPSTREAM Pack non-amendment note | 作者已修订；等待独立复核 |

| B04-P1-01 | P1 | 关键控制操作缺少逐项授权、稳定幂等、恢复和资源 use/manage 合同 | R05 新增 D10-CONTROL：区分 Workspace self / Workspace admin / deployment admin，冻结七类 closed body、四类 ResourceUseGrant、stable key 与 canonical compare、当前可见授权→saved replay→尚无 decision 才核 current revision 的顺序、同域原子提交、ID/incarnation 防 ABA、secret staging、host decision、stop 线性化和 profile/3；Workspace author-affecting mutation 仍进入 D6 唯一 ledger | D10-CONTROL §1–§14；D10；D10-UPSTREAM §3.5/§9；IMPLEMENTATION §1.1/§11.1 | 作者已修订；独立问题保持开放 |
| B03-P2-01 | P2 | CostReservation 把 uncertain 称为终态但又允许未来账单恢复，缺恢复边/CAS/证据归属 | R05 将 uncertain 定义为可恢复非终态；同 attempt 的 final bill→settled(actual)，可靠 never-started proof→released，sent-zero 固定 settled(0)；CostSettlementDecision 使用 expected revision CAS，原子更新 reservation/grant/account/evidence/audit，同 decision replay 防双返额，证据不足保持完整占用 | D10-CONTROL §10；D10 §18；D10-LEXICON；SCENARIO F30/F31；IMPLEMENTATION §9 | 作者已修订；独立问题保持开放 |
| B04-P2-01 | P2 | Bundled Module 冒称 D1 已有 module ID/code/locale，且 module/package/schema 映射未闭合 | R05 明确 Bundled Module 是 D10 候选产品组织抽象，不冒称 D1 已有 runtime ID；冻结 Calendar/Library/People/Organizations 的唯一 D10 PackageId→module→schema 映射并引用实际 D4 namespace owner/FacetId，PackageId 与 D4 namespace 分型；candidate code/locale 明确未实现 | D10-CONTROL §4–§5；D10-LEXICON §14.2；SCENARIO P17 | 作者已修订；独立问题保持开放 |
| B04-P2-02 | P2 | D01 anniversary/birthday 双语漂移；F06 英文漏 response 边界 | R05 两语 D01 都保留 birthday、anniversary、holiday 三类具体规则算法；F06 两语都固定 durable intent→send→response 三个故障边界 | SCENARIO D01/F06 | 作者已修订；独立问题保持开放 |
| B04-P2-03 | P2 | D10-TASK / Implementation 当前完成门仍写 48/48 | R05 将当前作者完整输入统一为 S 的 49/49；D10-TASK、D10-READING、Implementation 的当前完成门改为 49/49，旧 U 的 48/48 仅保留在历史阅读语境 | D10-TASK；D10-READING；IMPLEMENTATION §16 | 作者已修订；独立问题保持开放 |
| B05-P2-01 | P2 | U02 把 terminal occurrence/原 Run 恢复规则冒称 D1 §4.1/4.4 明文 | R05 U02 分开直接上游依据与 D10 派生依据：D1 §4.1/§4.4 只支持共享 Core/产品端边界；terminal occurrence 与原 Run 恢复明确归 Candidate §13/§20 + Implementation §6 | SCENARIO U02 | 作者已修订；独立问题保持开放 |

| B08-P2-01 | P2 | D8 九概念/十三 kind 缺逐概念 CLI/UI/locale 或无新增结论、kind→concept owner、alias 与迁移删除目标 | R06 在 D10-UPSTREAM §8.1 由 D8 owner 补九概念完整 naming metadata 与十三 kind 反向归属；未新增 UI/CLI/locale 的概念明确写无；D10 D10-LEXICON §15 只引用 owner，不建立 alias；D8 wire、IME、Write/Read、confirm、Undo 不变 | D10-UPSTREAM §8.1；D10-LEXICON §15；SCENARIO P18；IMPLEMENTATION §13/§15 | 作者已修订；独立问题保持开放 |
| B09-P2-01 | P2 | D9 grouped lexicon 和八份 D9 来源缺新增概念稳定 ID、完整中英名、wire/type/profile owner、surface/alias 与迁移目标 | R06 由 D9 owner 拆分 SourceArtifact/ImportIR/Provider/Route/Template/RenderSnapshot/D7ResultPin/ExportPlan/PublicationReceipt 等稳定 concept ID，明确 per-concept surface 为已有入口或无新增；继承 D2/D3/D6/D7 名称保持原 owner；PublicationReceipt 只表示外部发布 | D10-UPSTREAM §8.2；D10-LEXICON §15；SCENARIO P19；IMPLEMENTATION §13/§15 | 作者已修订；独立问题保持开放 |
| B09-P2-02 | P2 | U12 用“相同 UID 两次 ordinary import fresh”遗漏当前 ICS profile 未开放及 D3 SourceBinding/ForeignIdentityKey 状态矩阵 | R06 将 U12 改为三分支：当前 ICS conversion unavailable 且零作者效果；无 D3 binding 的已支持普通文件 profile 两次独立明确导入各 fresh、same request retry 精确重放；进入 D3 binding 语义后严格按 never_bound/active_live/active_non_live/retired/conflict/miss 与 explicit adopt 规则，UID 不能脱离 SourceBinding 独断 | SCENARIO U12；IMPLEMENTATION §15 | 作者已修订；独立问题保持开放 |

## 5. 保持不变的边界

本轮没有放宽自动作者写入范围。首版仍只允许 D7 `set_field_member` 的 single_field_member profile；D3 create/lifecycle、D8 document/annotation edit、bulk/Facet/native-table 等仍需要交互确认或保持不支持。

Core 仍是唯一 author transaction authority。ApprovalUse、LeaseRunUse、PlannedDecisionApproval、Run 与 ExternalEffectIntent 都是受管控制证据，不是第二 ledger 或第二 author receipt。

外部结果 `outcome_unknown` 与费用 `uncertain` 保持；没有为了修复批准/Run 语义而改成自动重试或自动退款。

当前设计工作将本目录与 `../d6-file-authority-reopen/` 中显式 owner 后像及唯一路由协调。固定 S 快照保持原字节；每份正文只有一位指定作者，创作与独立审查分工保持。候选不代表产品实现、合并、发布或激活；设计冻结之前仍须 A2 整合及全新普通 Chat Pro 全局终审。

## 6. 文档质量修订

上一版中文为了满足混排检查，重复追加了“技术名称不扩大权限”等泛句，并产生双句号。它们不是规范语义。本轮删除这些填充句，改为在对应段落直接用中文解释真实边界，同时保留必要的受控英文标识。仓库 check_docs 规则不放松；若仍有混排失败，只修改实际文案，不通过重复填充绕过。

## 7. Fresh 独立复核要求

完成条件是九对双语 D10／18 个准确路径与全部必要 D1–D9 实际 owner 后像在同一不可变候选上一致；固定 S49 及其清单保持不变；既有125个场景 ID 与完整义务可追溯，新增案例明确列出。适用文档/输入检查、真实阅读覆盖、术语、中英语义以及历史/当前发现都绑定该候选。旧 S49/49 覆盖、具名差分 PASS 和 CI 均不能继承为全文接受。设计冻结要求零开放 P0/P1、剩余 P2 明确处置以及 fresh 独立全局 Pro 接受，仍不等于实现或发布。
