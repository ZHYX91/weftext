---
source_language: zh-CN
translation_status: source
---

[English](D3-IMPACT.md)

源文档 ID：04f16e0f-a8ca-4d4a-8d75-007e46d44975。

# A2 D3 Implementation Impact and Test Outline

配套：[D3 主文](D3.zh-CN.md) · [current schema](D3-SCHEMAS.zh-CN.md) · [source map](D3-SOURCE-MAP.json)

状态：A2 D3 current implementation/test companion；作者候选，未独立接受、未实现、未激活。parent 全部 implementation obligations、cases 与最小反例逐项保留；fresh current carrier 使用 wire13/Proof3/Notice3/CP4/PAB4/Value4，historical record 继续 recorded decoder/bytes/pins/recovery。本文记录设计验收义务，不是已运行产品测试。

## 1. 实施切片

本文件只规定未来实现、回归、故障恢复、隐私、预算与验收义务；任何“必须测试”“必须验证”都不是已经执行或通过的声明。实现切片必须同时落到 D3 主文、D3 Lexicon 与真实 D6 P1 生产者已经冻结的边界，而不是另造近似接口。

| 切片 | 新版职责 | 保留边界 |
|---|---|---|
| wire13 current 与版本化账本分流 | 消费 `DecisionKey/2`、`CommitDomain/2`、`Frontier/2`、`InputDescriptor/3`，并实现共同门后的 saved/planned/unseen 唯一分流；真实 historical wire9-12 record 必须先按 recorded decoder 恢复 | `D3-CJ/3`、`D3Integer`、Ref/Locator 词法和 24 个 D3 错误家族的原 stage/priority 不变；同 P 只有一个主决议 |
| source currentness 与 revision | 区分 production `SourceVersion/2`、本地 current `SourceObservation/1`/`SourceVersionRef/1`，消费 `SourceStamp/1`、`SourceRevisionPlan/1`、`RevisionTokenSource/2`、`RevisionTokenBinding/2` 与 `d6_source_revision/2` | 旧 opaque revision token/Locator decoder 不升级；foreign production revision/epoch/`externalSequence` 不向新的受管修订号捐值 |
| dependency/range | 消费十五类闭合 `DependencyKey/3`（含 `document_format`） 与 D3 九类 `StructureRange`，实现授权先于隐藏读取、完整正负/空证明与 P/M 连续性 | partial I、无命中、相同 hash/count、provider 状态或 Frontier 前缀都不等于全集 |
| `replica_local` | `create_node`、`move_node`、`reorder_node`、`trash` 的真实 local_structure + `scope_dependencies` | 安装仍为 `WriteProtection=strict`；只能在完整连续已封存无关扩展证明成立时继续同一计划 |
| `managed_atomic` | restore/purge/copy/fork/continue/import 与全部强闭包 | 一律 `frontierPolicy=exact` + `WriteProtection=strict`，不得自动降级为 local |
| copy/fork/import/Definition Transfer | 单一私有 candidate map、owner/membership 矩阵、真实前像/结果 pins、全部 typed slots、Q 双射及两遍同源位置闭环 | 不扫描普通 CEL/text/unknown JSON，不发明 D7 schema、Definition identity 或第二 wire |
| portable publication | 同一决议的 receipt/companion、本地 `SourceVersionRef/1` projection 与 `ContentCompletionProof/4.sourceChanges` production transport 分离 | `InstallationNotice/3.baseFrontier` 不改写；receiver 自建本地 Observation/Ref |
| 冲突与恢复 | 当前 `ConflictRecord/2 + Frontier/2`、历史 `/1 + Frontier/1`、真实已封存的头，以及 saved/planned/unknown 三类状态的恢复 | `ConflictKey/1`、`ConflictId` 与 `D6-ConflictKey/1` 的哈希域不变；不采用 LWW，也不按修改时间自动选出胜者 |
| 普通保存 | D6 的人工普通单一现有文档来源保存使用 ordinary/complete（普通/完整）语义轴和 strict/observed_only（严格/仅观察）安装保护轴 | D3 身份/结构/生命周期、D5 结构化操作、服务器检查点以及强动作/自动化/批准/资金流程始终采用 strict（严格）保护 |
| downstream gates | D4/D5/D7/D8/D9/D10 只在真实依赖处消费新增生产者合同 | 未协调的强 consumer 只门控依赖它的路径，不永久取消合格 ordinary/local/offline 能力 |

固定 S 的 snapshots、design inputs 与 catalog 全部只读，P2 只同步真实 owner afterimage 与三份 routing 元数据；本 Impact 不允许通过“测试准备”修改历史输入。

## 2. decoder、共同门与版本路由

实现必须先按真实记录版本和 wire 版本分流，并保持历史 bytes/decoder 不被当前协议重解释：

- v9/v10/v11：仅对实际存在的 saved/planned/unknown、receipt/error/effects、Preparation/Observation/token/pin 记录按原 decoder、原 bytes/fingerprint、原 profile、授权、custody、TTL/clock、retention 与 no-duplicate-effect 责任恢复。存在 decoder、fixture、草稿或候选文字本身不证明该原型曾部署或 active。
- v12：只接受真实 D6 owner decoder 成功解出的 `DecisionKey/2`、`CommitDomain/2`、`Frontier/2`、`ObservationScope/2`、`DependencyProof/3`、`OwnerInputBinding/2`、`InputDescriptor/3` 与适用 `SourceObservation/1`，再由 D3 检查闭合 owner request。
- 其它版本：unsupported；不得猜兼容、改版本号后重放或批量迁移旧字节。

共同顺序必须可通过故障注入和重放测试证明：
1. outer/D3 closed static decode、canonicalization 与只依赖 request bytes 的 cross-field 检查；stage1/2 的 `invalid_identity_ref`、`domain_kind_mismatch`、`workspace_ref_misbound` 保持原优先级。
2. 当前主体最低 state/result disclosure、记录原 profile 适用的 `ObservationScope/2`/mode 授权与 prior `CommitDomain/2` 资格；不得先读取隐藏业务内容来选择较窄 scope。
3. 证明提交域栅栏、可移植信任与后端、P 账本以及保管/权威连续性；可用性或连续性未获证明时，必须先于对可达完整性的判断处理。
4. 以记录实际版本、完整原 canonical request/fingerprint 与 `protocolOwner` 定位同一 `DecisionKey/2`，随后立刻三分支：
   - saved：只检查原实际 effect/mode 或原 result-disclosure scope 的当前交付授权，返回原 bytes 或恢复原 publication/outbox；
   - planned：只恢复同一个冻结 plan；
   - unseen：此时才进入当前 owner/consumer、Observation、DependencyProof、revision basis、Frontier、预算和 planning CAS。

不得在查原账本前统一要求当前 `SourceObservation/1`、`DependencyProof/3`、当前 Frontier 或新 D4/D5/D7 consumer。same `OperationId` 在不同完整 `CommitDomain/2` 中可形成独立 key；同 key 的 wrong owner/different fingerprint 仍按原冲突处理，不读取原请求业务内容。

D6 前门只使用既有 v2 error；D3 完整家族继续包括 `invalid_identity_ref`、`domain_kind_mismatch`、`workspace_ref_misbound`、`identity_not_visible`、`identity_authority_unavailable`、`workspace_integrity_conflict`、`workspace_identity_conflict`、`operation_id_conflict`、`owner_mismatch`、`root_operation_forbidden`、`identity_not_resolvable`、`entity_not_live`、`entity_not_restorable`、`invalid_ordinal`、`orphan_creation`、`structural_cycle`、`invalid_locator`、`stale_locator`、`identity_collision`、`cross_workspace_identity_preservation`、`identity_map_incomplete`、`operation_precondition_failed`、`inbound_reference_conflict`、`identity_commit_aborted`。原 stage6 继续为空，不新增错误家族占位。

## 3. canonical request、pins、source revision 与计划冻结

wire13 canonical request 不永久嵌入完整 Document/Resource/Annotation bytes。`d3_identity_operation/13` owner descriptor、`InputDescriptor/3`、typed `PinRef`、完整 `SourceObservation/1`、`DependencyProof/3` 和 owner-specific protected input 必须逐项比较；仅 hash 或最终文本相同不足。

实施必须验证：
1. `InputDescriptor/3` canonical bytes 相同，但任一 exact pin、Observation、DependencyProof 或 `FileObjectBinding` 不同，都不是同一输入。
2. 相同 production `SourceVersion/2`，若 current `observationEpoch` 或 `FileObjectBinding` 变化，旧输入不能恢复 currentness。
3. planned recovery 只使用原 pins/Observation/dependency/candidate map/version basis，不重读 current source 后伪造等价 request。
4. pin cleanup 服从真实 last-reference/retention，不能删除 saved/planned/unknown/conflict/Approval/Money/outbox 恢复仍需要的证据。
5. strict request 不能通过改 owner descriptor、`frontierPolicy`、pins 或 plan 原地降级。

production/current 分域必须有可执行测试：
- `SourceVersion/2` 的 managed 与 external 分支都保留 production `CommitDomain/2` 与 production `observationEpoch`；current `SourceObservation/1` 另有 observer `CommitDomain/2`、current `observationEpoch`、`FileObjectBinding` 和 pins。
- `SourceVersionRef/1` 只选择完整受保护 Observation；receiver 或另一个副本不得复制 sender `sourceToken`。
- 两个副本从同一 production before/source cut 开始时，各自在自己的 operation `CommitDomain/2` 下建立并验证完整本地 current `SourceObservation/1`。

`H(D,E)`、`SourceRevisionPlan/1` 与修订令牌的验收必须覆盖：
- 同一 production domain 跨 production `observationEpoch` 不重置 `H(D,E)`；新 managed after 使用连续 sealed history 的 checked H+1，MAX 不 wrap。
- foreign production revision、foreign production epoch、`externalSequence` 都不捐给新 production domain 的 revision。
- 只有真实完整空历史才允许 H=0；missing/corrupt/gap/unproved history 不是空历史。
- `SourceRevisionPlan/1` 冻结前像观察或已证明不存在的分支、同一生产域的 `lastIssued`/完整空历史依据、`SourceStamp/1`、精确的 `afterPin`、候选映射与版本依据；规划 CAS 胜出后不得重新采样 H、修订号、令牌或身份。
- `RevisionTokenBinding/2` 是闭合稳定生产地址记录 `{kind,version,token,source}`；`source` 使用 `RevisionTokenSource/2`，tag 为 `d6_source_revision/2`。managed plan 在 C/Q 物化前固定一条拟议 token；winning CAS/seal 选出唯一 canonical binding 与 original sealed-outbox 关联，loser/aborted/seal 不可证明的 token 不能借另一 seal。新的读取资格另要求 current SourceObservation.sourceVersion 精确相等；bare stamp/version、caller 自选 token、相同 hash 或旧 runtime 证据均拒绝。
- 真实 source change 在 seal 时才把 frozen stamp 与同一决议 `ChangeId/1` 合成 managed `SourceVersion/2`。
- 真正的原始 no-op 不产生受管后像；来源未改变的结构/生命周期操作也不产生受管后像或推进 H；删除使用 absent 后像，且不产生后像修订号或推进 H；即使外部来源字节相同，接纳后仍形成外部前像与受管后像。

验收场景对应主文反例 1–7、26、53、54，并必须有成功、stale、gap、unknown 与 crash/replay 版本，而不是只测 happy path。

## 4. replica registration、丢 I、丢 P 与责任连续性

### 4.1 新设备

正式 fixture：Workspace W 的普通文件与 portable F/M 完整到设备 B，但 B 没有 A 的 P/I。B 必须先验证 Workspace/portable trust、replica registry、birth/tombstone/structure records 和实际 files，再注册新的 `ReplicaEpoch`，保留 W 与已有 refs，并在新的 `CommitDomain/2` 只取得 ordinary content qualification。

反例：
- 把 B 当 `continue_workspace`；
- 复制 A 的 control DB/WAL/SHM 让 A/B 同时消费同一 Approval/Money；
- 让 replica registration 接管旧 P、unknown、external effect 或 D10 execution responsibility。

### 4.2 丢 I

I 可全部删除。恢复必须证明：
- identity、parent/order、lifecycle、portable conflict/tombstone 从 F/M/P 真实 owner 恢复；
- Query/search/parser/OCR 渐进重建；
- 不新 mint birth/receipt/tombstone，不重置 `ChangeId/1`/`Frontier/2`；
- I 不拥有 proof epoch/revision、完整枚举边界、empty proof 或连续消费位置。

十五类 current `DependencyKey/3`（含 `document_format`）的 stamp 必须在 I 缺失、部分构建、parser/OCR 版本变化、watcher gap、百万小文件下测试。I 重建本身不重签 proof；只有真实 correctness facts/continuity 丢失才需要新 proof epoch。

### 4.3 丢 P

P 丢失/损坏后：
- old `ReplicaEpoch` 不得继续发新决议；
- portable Notice/Proof 与实际 components 只用于圈定恢复/冲突，不得从 current files 重建 original decision/unknown/Money；
- unknown installation、Approval/Money/claim/outbox/stop 责任继续保留，不能凭 equal hash 或空 control DB 猜 terminal；
- 安全协调后可 retire old epoch 并注册新 epoch 继续 ordinary content；
- execution responsibility takeover 需要独立 custody/fence 证明。

对应验收场景 40、41、44、50；必须证明“普通内容可以继续”与“全局执行责任仍不可用”同时成立。

## 5. ordinary/complete 与 strict/observed_only 两轴

`ordinary|complete` 是语义证明范围；`strict|observed_only` 是安装保护。两轴不得互相推导。

所有 D3 身份/结构/生命周期模式，包括 `create_node`、`move_node`、`reorder_node`、`trash` 以及恢复、清除、复制、分叉、接续和导入，均保持 `WriteProtection=strict`。D5 的结构化单元格/行/列/重排、批量/集合/提升，以及 D7 强动作、自动化、服务器检查点、批准和资金流程也都必须采用严格保护。

`observed_only` 只允许 D6 trusted-human `interactive_source_save`，且人工必须在 planning 开始前显式选择并冻结。完整正例必须同时满足：
- 恰一个 existing live Document；
- `saveProfile=ordinary` + `guarantee=replica_local`；
- whole-source 完整 read/replace；
- author-source write set 为空或只有该 Document；
- 无适用 body/Field/node-control deny；
- 不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar 或任何其它 entity；
- Draft Base 精确绑定当前完整 `SourceObservation/1`；
- 实际读取/固定的 before `B` 与用户输入 `N` 进入耐久计划/retention。

真实弱保护风险必须进入测试 oracle：final trusted object/event-continuity check 之后才出现且从未被观察到的 `C` 可能被 `N` 覆盖，`C` 也可能没有可恢复副本；later C 可重新成为 current，但不能抹掉 durable B/N 或改写旧 receipt/proof。

负例必须覆盖：
- strict 计划开始后 backend/capability 失败不能 fallback `observed_only`；
- revocation、known competition、stale Base、watcher/event gap、deny、P durability 不足不能走弱保存；
- observed competition 进入 conflict/reprepare/paused；
- install owner/outcome unknown 进入 `recovery_unknown`；
- prepared 或 `inputRetentionState=retained` 不等于 Saved；
- sealed `durable_observed_only` 不冒充 strict `reliable`。

对应验收场景 33–36、55。不得新增逐次批准流程来“补强”弱路径。

## 6. `replica_local` 与 `managed_atomic` operation matrix

`replica_local` 只缩小语义依赖范围，不改变安装保护。只有四个 D3 local_structure mode 使用 `scope_dependencies`：`create_node`、`move_node`、`reorder_node`、`trash`。它们必须保留原 subject/parent/ordinal/closure/proposed after/pins/version basis/`InstallationNotice/3.baseFrontier`，不能因为 current Frontier 变大而重选计划。

继续同一计划必须同时证明：
- original base -> actual current cut 是完整、连续、verified-sealed、无回退链；
- 每个新增/前进 head 有真实 portable ChangeRecord/completion history；
- 原十五类依赖中的所有实际 source/control/auth/Registry/rules/membership/positive-negative range/owner-version 仍成立；
- 新 sealed effects 与这些 complete dependencies 确实无关；
- unrelated evidence 与 actual cut 耐久保存在 P，而不是丢 P 后从两个 vector 猜。

`managed_atomic` 全部 D3 mode 固定 `frontierPolicy=exact` + `WriteProtection=strict`。任何完整范围、payload、authority/custody、owner version、pin continuity 或 strong consumer 缺失都只阻断真实依赖该项的强路径，不自动降级 local，也不借 Server 并行性改成 `scope_dependencies`。

### 6.1 create/move/reorder/Trash local positive/negative

必须保留旧 Impact 的 create/move/reorder/Trash 正负场景，并增加九类 `StructureRange` 的精确检查：
`live_children`、`trash_children`、`trash_roots`、`ancestor_chain`、`subtree`、`owner_resources`、`owner_annotations`、`reply_closure`、`restore_membership`。

每种范围都要测试：
- non-empty complete；
- empty complete；
- hidden member；
- missing shard/I/O failure；
- partial/building I；
- watcher/journal gap；
- auth/revocation 在 final revalidation 变化；
- ordered boundary/cycle/owner/reply invariant；
- same hash/count/no-hit 不能替代 complete proof。

授权/披露必须先于隐藏 sibling、owner member、reply edge、inbound slot 或 restore membership 读取。

### 6.2 copy/fork/import/continue 与 Definition Transfer

必须覆盖真实 mode/owner/现有 container 矩阵：
- `copy_node_subtree` 默认只复制合法 live closure；trashed owner-local member 不因 owner 相等复活；
- standalone `copy_resource|copy_annotation` primary 必须 fresh-map 到显式 destination owner，owner 不原地交换；
- 只有原 mode 允许的 destination-owner existing container 才可按 paired preimage/result/slot/S 矩阵修改；
- `copy_annotation` fresh mapped primary 初始 reply 只走 reference plan，绝不作为 S container；
- exact fork 覆盖完整 live+trashed closure，identityMap 完整，两态与 Trash placement 精确分区；
- `continue_workspace` 需要 old authority stopped/fenced，并且证明 cut 后无 committed facts，或取得足以完整合并 post-cut facts 的 authoritative ledger；
- partial identity-bearing bundle 只用 artifact/source pins 作 preimage，不在线查询 source authority；
- ordinary import 的 worker/IR/provider ID 不得成为 content identity。

所有新建/映射身份都使用同一份私有候选映射。定义转移必须遍历全部真实类型化根：`DefinitionAddress.owner`/完整定位器、类型规范声明的引用、`CollectionCreationPolicy.parent`、已保存的视图/查询调用、查询选择器字面量、`QueryRef` 参数、参数默认值、固定视图域的 `TypedLiteral`，以及动态块的字面量/上下文绑定；普通 CEL 表达式文本、普通文本和未知 JSON 不扫描。

slot/Q 验收必须证明：
- 每个 typed Ref/Locator root 恰一 slot，完整 Locator root 不再拆 owner 重叠 slot；
- 无 Ref 的已识别 payload 仍有 `slots=[]`；
- source/result occurrences 与真实 pins 独立验证；
- Q 与 saved-definition payload span 一一对应，不与 B/M/N/E/S/C 重叠；
- 两遍位置物化使用同一 candidate map、`SourceRevisionPlan/1`/revision token 和同一最终 source；
- 第二遍得到的跨度不相等、目标存在歧义或缺失、类型化槽遗漏，或所有者/类型错误时，整个操作都必须失败。

对应验收场景 13–21，不得以“见主文”代替可执行 fixture。

## 7. sync/conflict、identity collision 与 no-reuse

并发矩阵继续覆盖：
- source/source -> `source_concurrent`，两 heads 保留；
- create/create distinct IDs -> 两 birth 保留，parent order 可冲突；
- create/create same typed ref -> `identity_collision`；
- move/edit -> 只有因果、metadata/structure 与实际 dependency 都证明时才可组合；
- move/move -> `placement_concurrent`；
- Trash/edit -> `lifecycle_concurrent`；
- Trash/restore -> lifecycle conflict 或 exact strong revalidation；
- body/metadata 分批 -> `incomplete_transport`；
- on-demand bytes -> placeholder/not_materialized；
- policy concurrency -> 不做 grant union；
- ABA watcher gap -> old evidence/token/Locator stale。

current conflict 必须用 `ConflictRecord/2 + Frontier/2`，但 `ConflictKey/1`、`ConflictId` 和 `D6-ConflictKey/1` hash domain 不变。每个 head 都是真实、连续验证、已封存且与 key 相关的 `ChangeId/1`；external competition、unknown install、third-state bytes 没有 sealed head 时不得伪造。

历史 `ConflictRecord/1 + Frontier/1` 只走原 decoder/bytes/recovery；同一个未改变 `ConflictKey/1` 不存在 /1 与 /2 两个 current record。resolved history 不 reopen，不按 UUID/path/mtime/final hash 选 winner。

identity collision/no-reuse 仍必须验证：两个 local reliable birth claim、无 canonical winner、显式 branch resolution、losing content fresh-copy/import、新 ID、旧 collision ref 历史不改、tombstone 不复活、retired replica 旧 bytes 不能恢复 live。

对应验收场景 46–51。

## 8. purge、restore 与 replica coverage

restore 与 purge 的 membership 必须分别测试。

restore：
- 只允许 `managed_atomic` + `frontierPolicy=exact` + `WriteProtection=strict`；
- 只恢复原 Node Trash/lifecycle decision 的 `restore_membership` 中当前仍可恢复的成员；
- 更早独立 Trash 的 Resource/Annotation 不因 owner 相同被恢复；
- 需要真实 Trash graph/location、owner dirs、reply closure、refs 与其它 owner complete dependencies。

purge：
- 同样保持 `managed_atomic` + `frontierPolicy=exact` + `WriteProtection=strict`；
- 必须覆盖 target owner closure 中全部当前仍 trashed 的 owned Resource/Annotation，包括更早独立 Trash 的成员；
- 完整 `replica_registry` 必须证明 active/retired `ReplicaRecord` 目录、`registrationSequence`、gap-free continuity，以及所有 active replica 对 required sealed cut 的真实 admission 或合法 retirement；
- 需要完整 inbound/foreign/control、D4 relation incidence、Registry、Calendar/temporal、D5 与其它 strong owner proof；
- 提供方显示“已同步”、当前在线副本集合、稳定前沿值、计数/哈希、部分 I，或本地 Trash 的 `semantic_pending`，都不能作为清除操作的充分证明；
- auth/disclosure 在隐藏 range 读取前，未授权时不泄露负范围、成员数或冲突数。

source-unchanged purge lifecycle effect 可以没有 source change，但仍有真实 portable `ChangeId/1`；delete after absent 不建立 after `SourceRevisionPlan/1` 或 H increment。

对应验收场景 10–12、26。少任一 active replica causality/ack 或 complete range 时保持 paused/unavailable/conflict，payload deletion 必须为零。

## 9. dependency completeness、large workspace 与 partial index

`DependencyProof/3` 恰有十五类闭合 `DependencyKey/3`：
`source`、`document_format`、`lifecycle`、`placement_range`、`ref_inbound`、`relation_incidence`、`calendar_scope`、`registry`、`temporal_rules`、`authorization`、`foreign_binding`、`query_scan`、`replica_registry`、`conflict_record`、`execution_resource`。

测试必须用真实 owner 证明每一类的正/负/empty/currentness，而不是一个 generic owner JSON。重点：
- `source`：完整 current Observation、bytes/value pin、`FileObjectBinding`；
- `document_format`：真实语义解析 managed Document 时的精确 current format binding/qualification；source bytes 不变绝不能让旧 format proof 继续 current；
- D3 `lifecycle`、`placement_range`、`ref_inbound`：真实 portable identity/structure/reference dirs；
- D4 relation/calendar/registry/temporal：真实 D4/Registry owner；
- `authorization`：当前 principal/audience/policy generation；
- `foreign_binding`：SourceBinding/OriginBinding continuity 与 comparator；
- `query_scan`：D7 complete visible enumeration，不接受 page/cursor/partial I；
- `replica_registry`、`conflict_record`、`execution_resource`：真实 D6 control records。

large-workspace matrix保留：
- I completely missing；
- metadata-only I；
- 50%/99%/100% candidate search；
- parser/OCR version change；
- watcher gap；
- 1,000,000 small files。

普通 local operation 只等待它真实需要的 local ranges；strong Query/Action 只有完整 source scan 或 qualified complete coverage 才成功。building index 永远不把未扫描对象当 empty。

对应验收场景 8、9、22、45。

## 10. resolver、privacy 与 non-disclosure

resolver 继续覆盖：
状态全集保持为 resolved（已解析）、trashed（已进入 Trash）、tombstoned（已形成墓碑）、conflicted（存在冲突）、incomplete（信息不完整）、placeholder（占位未物化）、not_found（不存在）、not_visible（不可见）、workspace_unavailable（工作区不可用）、invalid（无效）。

没有 state disclosure 时，存在性差异全部遮蔽。Locator 只有 canonical live owner + locator disclosure 后才进入 resolved/stale/anchor ambiguity；stale 不 fuzzy-reanchor。

实现必须验证：
- 处理顺序保持为：闭合解码 -> 工作区/提交域绑定 -> 当前状态披露 -> 连续性证明 -> 冲突/不完整状态 -> 精确身份与生命周期 -> 定位器披露 -> 修订号/坐标验证；
- hidden sibling/member/reply/inbound/range 只在对应授权通过后读取；
- auth/visibility/membership/owner-rule 在 final revalidation 变化会使 proof 失效；
- same bytes/span/hash/token payload 不跨 observer epoch/gap/replacement 恢复 currentness；
- D3 range 或 Frontier prefix 不替代 D4/D6/D7 owner 的 complete set/absence/Query proof；
- preserved foreign Ref 不授 foreign Workspace enumeration/disclosure。

对应验收场景 6、48、49、52。

## 11. strict installation、P seal、Notice3/CP4/ChangeRecord1 与 crash matrix

所有 D3 identity/structure/lifecycle mode 固定 `WriteProtection=strict`。故障注入必须至少覆盖旧 Impact 的每个安装点，并同步为当前 P1 publication：
1. P planned 前；
2. pins/Observation/DependencyProof durable 后 planning transaction unknown；
3. `InstallationNotice/3` 前；
4. notice 后第一 component 前；
5. 每个 staging/flush/install/directory-flush 点；
6. observed competition；
7. installed verification；
8. P seal 前；
9. seal 后 `ContentCompletionProof/4` 生成/持久化前；
10. current CP4 + ChangeRecord/1 generation/write/flush/transport 中；
11. response delivery 丢失。

每格只能落入真实 exact-before、exact-after-with-provenance、third-state、unavailable/recovery_unknown 与 D3 decision 组合。相同 hash 但不同 `FileObjectBinding`/provenance 不能证明原 plan 写入。

P seal 是唯一 author-decision commit point。portable effect 在同一 transaction：
- 分配同一决议真实 `ChangeId/1`；
- 对真实 source change 验证 frozen `SourceRevisionPlan/1`，必要时形成 managed production `SourceVersion/2`；
- 保存 D3 primary receipt、`D3DecisionCompanion/2`、D6 state/effects、适用 charge/outbox；
- source-unchanged structure/lifecycle 不创建 source version；
- raw no-op 不创建 content ChangeId；
- deletion 使用 production before + absent after；
- equal-byte external admission 形成 external-before/managed-after。

`ContentCompletionProof/4` 必须验证：
- `components` 与原 `InstallationNotice/3.components` key set/顺序一致；
- `sourceChanges` 只覆盖真实 source-state changes，使用完整 production `SourceVersion/2|absent`；
- current receipt/effect metadata 的 `SourceVersionRef/1` projection 绝不代替 portable production transport；
- `frontierBefore` 是 seal 前真实验证的 `Frontier/2`，`frontierAfter` 恰加入本 proof `changeId`；
- exact 分支的 base 与原 expected/dependency/Notice base byte-equal；
- `scope_dependencies` 分支必须带完整 continuous sealed chain、原 dependencies 无关重验和耐久 P evidence，不改 Notice base；
- receiver 验证 production history 后，以自己的 observer `CommitDomain/2`、`FileObjectBinding`、current epoch/pins 创建自己的 `SourceObservation/1`/`SourceVersionRef/1`，不复制 sender token；
- CP4 + ChangeRecord/1 不授 complete Query/Action/negative-range/execution-responsibility；
- pre-seal restored outcome 只有在每一 component 已完整证明恢复 before 时可产生，且无 success/ChangeId；
- post-seal publication failure 保持 committed + pending，恢复只发布同一个 proof/outbox，不重装、不重采 revision、不再增 H/ChangeId/domainCommitSequence 或收费。

对应验收场景 23–27、44、50。

## 12. replay、saved/planned/unseen、no-op 与 unknown

golden r5/r6 sequence继续保留并扩展：
- O5 以 `DecisionKey/2` portable committed，response 丢失；
- O6 随后更新 current source/state；
- retry O5 exact original request；
- 共同 continuity/custody + original request/fingerprint 成功后，只按 O5 原 actual effect/mode 或原 result-disclosure scope 检查当前交付授权；
- 唯一结果是原 O5 receipt/error/effects bytes 或原 publication/outbox，current O6 另读。

saved replay 不要求 old before 仍 current、不要求 old Frontier=current、不重新检查 preview/preparation TTL，也不重新通过新 D4/D5/D7 consumer。revocation 只遮蔽交付，不改变历史 decision；重获权仍交原 bytes。不得重装 source/metadata、重分配 H/revision/ChangeId、重增 domainCommitSequence、重复 ApprovalUse/Money charge 或 external effect。

planned 恢复只恢复原规范请求/指纹、该 record 原始 InputDescriptor（fresh current 为 `/3`，真实 historical record 使用其 recorded version）、候选映射、`SourceRevisionPlan/1`/H 依据、固定证据、预留/写集、该 record 原始 InstallationNotice（fresh current 为 `/3`，historical 使用其 recorded version）、`WriteProtection`、所有者版本、尝试次数/预算、准备期限/时钟和安装状态。当前授权、依赖连续性与安装来源证明只决定继续原计划还是保持 paused/conflict/recovery_unknown，不允许重新 prepare、重新查询、重选目标或重新采样。

P/install outcome unknown 不是第四条重执行分支。必须保留原 record、pins、Approval/Money/claim/outbox、stop/recovery/no-duplicate-effect 责任；current files/equal hash/I/empty new control DB 都不能猜 success/failure、换 `OperationId` 重试、退款或重置 quota。

effectClass 仍测试：
- portable：真实 F/M portable change，在 seal 分配 `ChangeId/1`；
- control_only：只 P control，无 content ChangeId/Frontier advance；
- no_op：十二类 D3 arrays 为空，installation/save/publication not applicable；
- equal-byte external managed admission 不是 no_op。

只有原 plan 永不可能提交并且所有 remnant/reservation/pin/external responsibility 安全清结，才允许 terminal_failed + `identity_commit_aborted`；capacity shortage、temporary revocation、missing consumer、unknown install 不是 business rejection。

对应验收场景 28–32、43、55。

## 13. Server 多用户、Draft 与 ordinary offline

Server 必须允许多个主体、多个 session、多个 Draft 同时存在；principal/audience 来自各自 protected context，不能跨会话借用。每个 Draft 保留自己的 Base/selection/input。

验收：
1. 不同文档可以并行 prepare 并在真实无冲突时完成。
2. 同文档同 Base，A 先 seal 后，B 保留 Draft 并进入 stale/conflict/reprepare，不能覆盖。
3. B 在 checkpoint 前撤权不能提交。
4. A 已 seal 后撤权只遮蔽交付，不回滚历史 decision。
5. restart/failover 同时 fence P 与 file writer。
6. old instance 只读/拒写，不能 rename author files。
7. serial backend writer 不抹掉多个前端 Draft/session。
8. 两个离线副本对同一结构范围 move/Trash 保留并发 heads，不 LWW。
9. P/I schema 不得出现 whole-workspace current body/full AST mirror 或第二 parent/order authority。

Server parallel prepare 不放宽 strong policy：所有 `managed_atomic` 仍 `frontierPolicy=exact`；只有原本四个 `replica_local` local_structure mode 可按完整 unrelated sealed-chain 规则继续。Server checkpoint 始终 strict strong consumer。

对应验收场景 38、39、46、47。实时协作算法仍是未来接口契约，不得声称已实现/通过。

## 14. `semantic_pending` consumer gate 与世代边界

`semantic_pending` 只表示 D2-valid local facts 已通过，而合同明确允许继续 pending 的 cross-object/complete obligation 尚未证明。它不能满足：
- D2/typed invalid 或 deny；
- relation “无任何目标”负范围；
- unique “全 Workspace 唯一”；
- Calendar complete expansion；
- collection/full membership postcondition；
- complete inbound/cross-object type closure；
- D7 complete Query/all_result、derived write set 或 bulk；
- Automation/strong Action 依赖全集 absence/uniqueness；
- managed restore/purge/copy/fork/import；
- Server checkpoint；
- Approval/Money。

普通 source presentation、Draft、局部编辑和明确 branch read 可显示 pending，但必须暴露未证明 obligations，不得显示“全部有效”。

世代说明必须准确：固定 C 已完成原 G0-A/G0-B baseline 对应的较早 D4/D5 候选消费者工作；它们是真实候选历史，但未接受/未激活。本 P1 新增的 production revision token、十五 key/range（含 document_format）、CP4+ChangeRecord1/ConflictRecord2、M5 replay/recovery 等规则仍需适用的 D3/D4 P2、D5 P3 与 D7 consumer afterimage。缺 strong consumer 只门控依赖它的强路径，不永久取消合格 ordinary `.adoc`/Resource read、Draft、human whole-source save 或 local offline operation。

对应验收场景 37、45。

## 15. budgets、pins、retention 与 resource/unknown

原 Impact 的 pin-retention、预算与资源义务继续保留，并与 P1 `execution_resource` 依赖同步：
- `DependencyKey/3.kind=execution_resource` 只冻结该原操作的资源策略版本、允许/已消耗的尝试次数、累计工作量、受保护固定证据/容量以及暂停类别；
- 它不吸收 D10 Money lineage、Run/Lease/Automation/deployment continuity、provider unknown 或 `sourceOccurrenceKey`；
- planned/unknown/conflict 状态、Approval/Money 责任以及原发布/外发队列仍作为最后引用持有的固定证据，不得因预览有效期届满、重建 I、当前文件可读或出现新的所有者版本而删除；
- 合同明确允许过期的 historical effect pin 真正失效后，只返回其原 `effects_unavailable`，不得用 current file 冒充 historical after；
- P/install unknown 时不得退款、重复收费、重置 quota/approval、重新发送 external effect，或从 empty control DB 重建责任；
- budget overflow 在 Definition Transfer 两遍 materialization、完整 range enumeration、current CP4+ChangeRecord1 publication 和 recovery 中都必须可判定地暂停/失败，不允许部分成功后偷偷扩大预算。

## 16. wire13 current、historical corpus 与 terminology gate

新 corpus 必须独立覆盖：
- wire13 new decision accept；0..11/unknown 对新 decision 拒绝；
- actual saved/planned/unknown v9/v10/v11 原 decoder/bytes/fingerprint/pins/custody/TTL/recovery；
- `DecisionKey/2`、`protocolOwner=D3`、same `OperationId` cross-domain independence、same-key fingerprint conflict；
- replica/server `CommitDomain/2`；
- `Frontier/2` exact 与 `scope_dependencies`；
- `ObservationScope/2` local_structure、workspace_constraints、prepared_workspace；
- closed `d3_identity_operation/13` owner binding + `WriteProtection=strict`；
- exact `InputDescriptor/3`、pin/Observation/DependencyProof mismatch；
- `replica_local|managed_atomic` mode matrix；
- production `SourceVersion/2` vs local Observation domain；
- `SourceStamp/1`、`SourceRevisionPlan/1`、`RevisionTokenSource/2`、`RevisionTokenBinding/2` 与 `d6_source_revision/2`；
- 十五类 `DependencyKey/3`（含 `document_format`）和九类 `StructureRange`；
- current `ContentCompletionProof/4` + `ChangeRecord/1` 与真实 historical CP1-3 recorded decoder 分流；
- `ConflictRecord/2 + Frontier/2` 与 historical `/1 + Frontier/1`；
- unchanged `ConflictKey/1`/`ConflictId`/`D6-ConflictKey/1` hash domain；
- effectClass portable/control_only/no_op + same-P `D3DecisionCompanion/2`；
- unchanged `D3-CJ/3`、Result/9 与 Locator l1；fresh current Annotation 使用 Value/4，historical Value/3 只按 recorded decoder 保留。

历史 corpus 只对实际存在 records 承担兼容；decoder/fixture/draft/prose 不自动证明原型 active。不得把 legacy 版本号改成12后声称新语义通过，也不得因“未发现部署记录”删除已证明存在记录的恢复合同。

Terminology gate继续验证：
- fixed-S 42 `conceptId` exact set；
- 每条 `ownedNames` exact set；
- `firstFreeze` 不变；
- D6 imported/internal technical names 不被 D3 re-own；
- Preparation Binding/Definition Transfer/Definition Result Segment 保留历史 firstFreeze；
- 新 technical field 无真实 producer/owner mapping 时拒绝或停用依赖路径；
- retired identifiers 与 owned set 互斥；
- 中英同一协议标识使用相同 inline-code 名称和版本。

对应验收场景 42、53、54。

## 17. 实施影响图与静态/动态测试入口

未来实现必须具备至少这些可独立故障注入和断言的入口：
- closed wire decoder/encoder/fingerprint；
- common gate + saved/planned/unseen ledger router；
- InputDescriptor/pin/Observation/DependencyProof exact comparator；
- production/source-observer currentness verifier；
- H/SourceRevisionPlan/revision-token allocator-verifier；
- 15-key/9-range enumeration and completeness verifier（含 document-format currentness）；
- local/exact Frontier policy verifier；
- Trash restore/purge membership evaluator；
- copy/fork/import candidate-map and owner matrix materializer；
- Definition Transfer typed-slot/Q parser/materializer/two-pass span verifier；
- ConflictRecord version router and typed D3 owner-resolution adapter；
- strict installation/crash recovery state machine；
- current CP4 + ChangeRecord/1 producer/receiver verifier，并保留 historical completion-version router；
- historical replay/no-duplicate-effect router；
- ordinary D6 observed-only qualification checker；
- Server Draft/base/currentness integration；
- consumer-version gates for D4/D5/D7/D8/D9/D10。

禁止为了实现方便而新增以路径作为身份、全局可变的当前正文表、第二套父级/顺序所有者、LWW、隐藏式重新分配 ID、第二份回执/账本，或由所有者自由定义的通用 JSON。

本节只定义实现切片与测试入口，不表示这些模块已经存在。

## 18. downstream owner、只读历史输入与激活门

固定 S 的全部 snapshots、design inputs、catalog 与其历史 bytes 只读；P2 不以“同步”名义修改它们。P2 只同步真实 owner afterimage 和三份 routing 元数据。

固定 C 已完成原 G0-A/G0-B baseline 对应的上一代 D4/D5 候选；当前 P1 新生产者规则还要求：
- D3/D4 P2：revision/currentness、15-key/range（含 document_format）、CP4+ChangeRecord1/ConflictRecord2、M5 分流与真实 D4 relation/calendar/registry/semantic_pending 消费；
- D5 P3：structured operations、revision-bound locator、complete range 与 strict ordinary/structured 边界；
- D7：查询、值表达式、视图、窄字段、定义转移、预览与效果、执行动作、准备和场景等全部消费者；
- D8：complete current `SourceObservation/1`、Draft/IME/Undo/selection/editor、Server checkpoint；
- D9：scoped constructor/import/export、artifact pins、unknown recovery；
- D10：ApprovalUse/Money、Run/Lease/Automation/deployment、stop/unknown、`sourceOccurrenceKey` 连续性及既定十八份文档/consumer responsibility。

已协调的 D6 producer 候选现已承认 native D3 descriptor/companion 存在，并允许 InstallationNotice.baseFrontier 的历史 sealed ChangeIds、排除本尚未封存决议的新 ChangeId。全新联合审查必须核对这些精确 producer/consumer 字节；候选存在不等于激活。

本 Impact 不打开这些 consumer、不修改 producer，也不把缺项解释为全局 ordinary/local 禁用。

现有 3 P1 + 8 P2、合计 11 OPEN 与 U6/U7 继续作为门。完整 consumer coordination 后仍需 fresh independent full joint acceptance 与修订复核。A2 self-contained D1-D10 重建已经获得人类既有授权，无需再次请求批准，但只能在上述完整 consumer coordination、fresh 独立联合接受和 repair recheck 完成后执行；本 Impact artifact 不启动或提前执行 A2。A2 之后仍有另一轮全新的 ordinary Chat Pro global final review。

## 19. 验收结论边界与最小反例覆盖

本文件是唯一候选作者的 Impact 后像，不是独立评审、实现证明或产品验收。任何未来测试必须绑定固定 commit、真实 backend/platform、真实输出工件和可复现 crash/replay/unknown 证据。文档 CI、静态 prose、fixture 名称或“已设计测试”都不等于实现 PASS。

除前述各节已分配的验收场景外，以下原 Impact 与 D3 main 反例必须全部具有明确 executable oracle，且不能只测成功路径：
- source 与 portable metadata 分批到达时，缺一侧是 incomplete/placeholder，不是 delete/fresh identity；
- create/move + remote edit 只有因果与全部真实 dependency 可证明时才能组合；
- Trash/edit 不存在 delete-wins/edit-wins；
- equal hash + 不同 `FileObjectBinding`/provenance 不证明安装属于原 plan；
- duplicate local UUID claim 只通过显式 conflict + fresh-copy losing branch 解决，不静默 rekey；
- D2-invalid external bytes 只能走 repair/read，不能产生成功 D3 author receipt；
- current revocation before seal 不提交，after seal 只遮蔽交付；
- same `OperationId` across different complete `CommitDomain/2` 可独立，同 key different fingerprint 冲突；
- canonical request 不永久嵌入 full body/source/resource bytes，purpose-specific pins逐项比较。

完整 55 个主文最小反例的覆盖关系必须在测试清单中可追踪：1–7/26/53/54 对应 §3；8–9/22/45 对应 §6/§9；10–12 对应 §8；13–21 对应 §6.2；23–27/44/50 对应 §11；28–32/43/55 对应 §12；33–37 对应 §5/§14；38–39 对应 §13；40–42 对应 §4/§16；46–52 对应 §7/§10/本节。编号用于 traceability，不允许用“覆盖主文全部反例”一句话替代真实 fixture/oracle。

旧 D10 的 3 P1 + 8 P2 共 11 OPEN 不因本文件关闭。等待联合接受的 D6 协调修订、D4 P2、D5 P3、完整 D7、D8、D9、D10 十八份、fresh 联合独立接受、repair recheck、已授权但尚未执行的 A2，以及最终另一轮 fresh ordinary Chat Pro 全局终审全部保留。

本 artifact 没有运行产品测试、实现、CI 验收、merge、activation、release 或 deployment，也不授权文档/GitHub/ref mutation。
## 20. 闭合接口修订验证

主文 §19 的 56–62 在既有 55 项清单上增加以下具体判据；它们是未来验证义务，不是已执行 fixture：

| 编号 | 输入与结果判据 | 零效果/隐私检查 |
|---|---|---|
| 56 | 无任何准备 token 的精确 native wire13 create_node，再给同对象增加 planToken | 前者进入原唯一 planning/seal，后者静态形状失败；无隐式 D6 prepare、第二 ledger 或全局 local 禁用 |
| 57 | A 签发并暂管未激活 W/B 的真实 create/fork；篡改 proposal；planned crash；saved rejection/commit | key 始终 W/B；stage3 issuer/source 后 P1/P2 后 TL；P1/P2 无效则 custody/target-ledger 零读；不要求 target policy/active/source_write，不重复初始化 |
| 58–59 | 每个 D3ResolverInput/12 arm/闭合 outcome；精确 F、无关 F+1、错误 binding、隐藏分支、anchor ambiguity | 逐字段比较 required/forbidden；F+1 为 workspace_unavailable，隐藏状态 not_visible；无授权前 target/head/foreign-route 读取或 branch/reason/source 泄露；输出前持有/复验同 cut |
| 60–61 | 同 W/O 不同域的 transfer keys；各 source state、target failed、sequence 0/MAX、历史 wire11 bytes | 验 full owner-qualified key、条件成员、单调不可变 record/终态；unknown 保持 pending，不猜域/联合 commit/自动重试删源 |
| 62 | 中英全部 42 个概念 definition、owner 限定与 migration/deletion | 同决议 sole OriginBinding、retired 显式 Adopt、template/exact tasks/task 禁止，以及受控 JSON/firstFreeze 不变；名称相等不能替代语义对照 |

实现 resolver 精确遵守主文 §11：静态 binding 比较不成为授权前失配 oracle，domain continuity 与精确 Frontier 相等均先于 target-state 读取。Transfer summary 只是协调器对两个既有独立 decision 的观察，不是新 mutation/rollback 协议。真实历史记录保留原 decoder 和责任。

### 20.1 冲突准备与单次原生决议

63–69 必须联测真实 D3 prepare、current D7 /3 binding/mapping 和完整preview producer。三种fresh-copy mode均以精确历史branch pins和完整native闭环做成功例，再分别去掉必需pin、加入plan未表达的source改写、删换binding、改head/key/selection、换audience；断言精确error family/顺序、未授权零读、无额外reservation/第二submit。普通raw wire13仍独立可达。

preview须从冻结before/proposed/branch/native/control证据独立重建全item集合和exact byte slots，再读取全部页及byte范围直到terminal。choice/hash-only pin、隐藏item、缺head source、错domain/header、混交付epoch或提前terminal都必须失败；prepare不分配来源身份。current manifest/EffectBytes版本、两类闭合conflict item及record producer由D7真实owner完成；该新路径等待完整owner后像与联合审查。这是具名整合要求，不是永久未指定adapter。

在stage14/CAS前head或权限变化、planning响应丢失、各安装边界、seal后交付前、preview过期、input guard损坏/缺失、unknown安装处注入失败。unseen遵守原operation_precondition_failed/availability顺序；saved/planned先在原profile当前披露门后使用原request/record/pins/candidate。验证唯一native fingerprint/DecisionKey/CAS/P seal、一份原D3 receipt与同P companion；无新submit envelope或D6 identity arm。真实 /1、/2 历史记录按其原decoder测试，不合成升级记录。

open-record相同bytes解决仍为portable；有证明的已resolved placement/lifecycle为真正no-op；每个fresh-copy都产生新身份。no-op不制造反向lifecycle transition，也不改变raw mode准入。canonical选择不能隐藏完整合成resolution plan外的物理source改写。这些都是设计判据，不声称fixture/产品已执行。

### 20.2 生命周期准入与双向 canonical 物化

case 70–73 补齐 IR-06/07 的设计覆盖：四类 selected lifecycle→requested-result 与真实 installed live/Trash 交叉，连同原 location/owner/reply closure 和独立 restore membership；placement 使用当前完整 sibling range；Node/Resource/Annotation 均比较 canonical A/copy B 与 canonical B/copy A。Resource installed a 但保留 B=b 时，断言同一 native decision 得到旧 R=b、fresh R′=a，物理 before/所选 source 分开绑定，canonical /2 version plan 与 fresh /1 plan 并存，完整 preview/receipt/CP4+ChangeRecord1 精确相等，不假造 Resource source RPC。同 bytes 改 canonical claim/version 仅增 H 一次，metadata-only source 不变不增，exact already-resolved no-op 无 ChangeId。raw mode matrix 与普通 current Observation 对 conflict 的拒绝保持。private wrapper/guard/DependencyProof source 资格只属 resolution，不能移植到普通 Query/D8/write。验证 omission/净 structural receipt、单实体唯一 final source、head/file 读前授权、stale head/CAS、install unknown、restart 与 saved replay。新增内容只指定执行 oracle，不声称产品运行或独立 PASS。

### 20.3 Canonical evidence and public completeness

主文 74–79 必须独立验证 owner N 的 Annotation reply 与 M 下 fresh copy、Node X→Y 且同一 actual fromSource 分属两个分量、原 E/delete/result-only/S/lifecycle-only grammar、完整 D4/D7 typed gates、overlap 精确相等、mandatory 公共 canonical plan/bytes/extension 和单 seal 恢复的正反例。分别独立重建两个分量证据集、执行原 comparator 唯一性，再对 CP4 + ChangeRecord/1 独立重建唯一去重的物理 source/control 集。原十二 receipt 数组仅表达 native；D3CanonicalEffects/1 是 canonical 必需完整公共证据，包含 resolved no-op 的空扩展。seal 后缺扩展为交付不可用，不是 saved commit 不存在。验证实际 BudgetBinding/1 exact fields 与 zero 语义。这些是设计验证义务，不是已运行产品测试或独立接受。


### 20.4 fixed-446 D3 P1 修复 oracle

`A2-D3:P1-01` 与 `A2-D3:P1-02` 的作者状态仅为 **resolved-pending-independent-review**；作者不自称关闭任一 finding。

- `D3-P1-01-A`：fresh current conflict prepare 只接受 `D3IdentityInput/13`，唯一 native request 路径是 `D3IdentityOperationRequest/13`；已证明真实存在的 historical Input12/wire12/PAB3 record 只按 recorded decoder 恢复，绝不成为第二条 fresh-current path。
- `D3-P1-01-B`：保持 exact managed source bytes/version 不变，只改变实际消费的 `document_format` binding/stamp。旧 Proof3 必须 stale；replacement proof 必含第十五个 Key3 arm 与 current Notice3/CP4/ChangeRecord1 chain。source bytes 相等不是逃逸口。
- `D3-P1-02`：current canonical Annotation concrete output 使用 `d3_annotation_value4`，D7 PAB4/EffectManifest3/EffectBytes3 transport 必须 strict-decode 同一完整 Value4。current plan 携带 `d3_annotation_value3` 必须拒绝；`d3_symbolic_result9` 只限真实 symbolic branch。historical Value3 record 保留原 bytes/recovery。

这些只是设计/一致性 oracle；本作者批次未运行 runtime/product fixture。

## 21. A2 implementation/test 状态

本 companion 中所有 source-qualified scenario、crash/fault、runtime、backend、平台、large-workspace、UI 与 concurrency 条件均为**未运行设计义务**。本批只执行文档/输入结构检查；不能以 schema 可解析、计数完整、CI 文档 job 或作者候选存在替代产品实现证据。D4–D10 完整模块仍为后续 A2 TODO；这里只记录本批实际读取的 D3 direct consumer/producer 协调边界。
