---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：`592603c4-c7ef-4572-aee6-256aa3aa7955`。

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 中 D7/D9 的生效标签与 revision05 文本只作为历史来源，不支配本后像。稳定文档 ID 保持。D3 wire11、D6 wire1、Policy/1/2、D7/D8 旧 binding 的 saved bytes 继续按原 decoder/gate 履约；本文件不允许半包激活新版 managed success。

# D6 Control Interfaces

本文件拥有 D6-FA-r01 的 closed control types、D6 wireVersion2、PreparedIntent/2、Policy/3、SourceVersion/2、普通 source save、decision/install state、portable installation/completion records、ConflictRecord 和错误/授权顺序。D3 identity/lifecycle 仍归 D3；本文件不得用通用 D6 入口创建/移动/Trash/purge Entity。

全部 JSON object closed：unknown/missing/duplicate key、非法 null、wrong union、非法 Unicode scalar 拒绝。可选 member 只有本文明确标 optional 时可省略；不以 null 代替 absent。

## 1. 共同标量与规范编码

### 1.1 Counter、Uuid、Token

Counter 保持 non-Boolean JSON integer 0..9223372036854775807，禁止浮点、指数、-0、string 与 double-roundtrip 后验证。所有 checked increment 超过 MAX 失败，不 wrap。

Uuid 逐字复用 D3 canonical lowercase RFC4122 UUIDv4。WorkspaceRef/NodeRef/ResourceRef/AnnotationRef/FieldId 使用其 owner decoder，不在 D6 建同名近似结构。

Token 保持原 D6 43 ASCII canonical base64url / 32 random bytes 词法；tag、audience、record owner 在受保护映射中独立保存。Token 不是 identity、permission 或 source version。wrong-tag/wrong-audience 与 unknown 按各入口遮蔽规则处理。

D6 v2 object canonical bytes 使用 D3-CJ/3 对成功 decode 的 closed object 编码；这不修改 D3 requestFingerprint，也不使 D6 object 成为 D3 wire。

### 1.2 CommitDomain/2

CommitDomain 是 closed union：

replica：
~~~json
{"kind":"replica","workspaceRef":<WorkspaceRef>,"replicaEpoch":"uuid-v4"}
~~~

server：
~~~json
{"kind":"server","workspaceRef":<WorkspaceRef>,"authorityInstanceId":"uuid-v4"}
~~~

workspaceRef 必须与调用外层 Workspace 相等。replicaEpoch 不是 AuthorityInstanceId；server authorityInstanceId 逐字使用 D3 authority 域。比较为完整 canonical bytes equality。排序先 kind rank replica=0/server=1，再 WorkspaceRef canonical key，再 UUID ASCII bytes。

### 1.3 ReplicaEpoch

ReplicaEpoch 是 canonical UUIDv4，由 Core 的受权 replica registration mint；同一 Workspace 永不复用。它不是 portable content identity、D3 continue token、global execution lease 或用户可选择的 device ID。

注册产生的 portable ReplicaRecord exact：

~~~json
{"format":"weftext.replica","version":1,"workspaceRef":<WorkspaceRef>,"replicaEpoch":"uuid-v4","state":"active|retired","registrationSequence":<Counter>}
~~~

registrationSequence 每个 Workspace 从1连续 checked increment，只用于 portable replica registry；不与 content change sequence 或 commit sequence混用。retired 不可回到 active；新设备须新 epoch。

### 1.4 ChangeId/1 与 Frontier/1

ChangeId exact：

~~~json
{"commitDomain":<CommitDomain/2>,"sequence":<Counter>}
~~~

sequence 必须 1..MAX；0 禁止。每个 CommitDomain 自身的 content change sequence 从1连续增长。ChangeId.domain.workspaceRef 与当前 Workspace 相等。排序为 CommitDomain key 后 sequence 数值。

Frontier exact：

~~~json
{"kind":"d6_frontier","version":1,"heads":[<ChangeId>...]}
~~~

heads 可空，仅表示 genesis/尚无 content change；非空时按 CommitDomain key排序、domain唯一，每个 entry 表示已接纳该 domain 的连续前缀 1..sequence。禁止两个同 domain entry、sequence=0、foreign Workspace。Frontier equality 为完整 canonical bytes，不把集合摘要代替本体。

### 1.5 SourceVersion/2

SourceVersion/2 是 closed union。

managed source：

~~~json
{"kind":"managed_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"revision":<Counter>,"changeId":<ChangeId>}
~~~

external observation：

~~~json
{"kind":"external_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"externalSequence":<Counter>}
~~~

managed observationEpoch、revision、externalSequence 均为1..MAX。managed changeId.commitDomain 必须 byte-equal commitDomain。Document entityRef 为 owner NodeRef；Resource/Annotation 使用完整 Ref。managed revision 在同 domain+entity 的实际 managed source change checked+1；fresh=1，raw no-op不增。observationEpoch 每当无法证明外部观察连续性时 checked+1，即使最终 bytes相同。externalSequence 在当前 observationEpoch 内每次发现一个不能归属于已知 ChangeRecord 的新外部状态 checked+1。

managed 与 external 永不相等；不同 CommitDomain 的 revision 数字没有比较意义。D2/D3旧 revision token 的词法仍由原 owner；新版 consumer 必须同时绑定 SourceVersion/2，不能拿裸 Counter 跨 domain续用。

### 1.6 SemanticState/1 与 ContentGuarantee

SemanticState closed：

~~~json
{"kind":"complete_semantics"}
~~~

或：

~~~json
{"kind":"semantic_pending","obligations":["relation"|"unique"|"calendar"|"collection"|"inbound"|"cross_object_type"...]}
~~~

obligations 非空、按固定 rank relation=0,unique=1,calendar=2,collection=3,inbound=4,cross_object_type=5 排序、无重复。两 variant 均保证当前 source strict UTF-8 且 D2 valid；semantic_pending 还保证本次实际修改的 local typed facts 已按当前 Registry 做完其局部门。它不宣称列出的 obligation 成功。

外部 strict UTF-8/D2 invalid 不编码为 SemanticState，而由 CurrentSourceState.external_invalid variant 表示，避免把没有成功 Core decision 的 raw bytes包装成“已保存语义”。

ContentGuarantee 是 exact text enum replica_local|managed_atomic，不接受其它字符串。

## 2. FileObjectBinding 与安装资格（受信内部类型）

FileObjectBinding/1 不由客户端构造，exact：

absent：
~~~json
{"kind":"absent","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"parentGenerationToken":<Token>}
~~~

present：
~~~json
{"kind":"present","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"objectGenerationToken":<Token>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~

PortableRelativePath 是 UTF-8 scalar path，使用“/”分隔、非空、不得绝对、不得有空段、"."、".."、NUL、平台别名或保留 reparse 跳转；实际文件系统还须做 host canonical containment 验证。relativePath 不是 identity。

FileInstallCapability/1 closed：

~~~json
{"kind":"create_only","parentGenerationToken":<Token>}
~~~

或

~~~json
{"kind":"conditional_replace","expectedObjectGenerationToken":<Token>}
~~~

或

~~~json
{"kind":"exclusive_write_window","windowToken":<Token>,"expectedObjectGenerationToken":<Token>}
~~~

这些 token 只由可信 backend签发并绑定实际文件对象/窗口；client不能自报。conditional_replace 必须由 backend 真正 compare object generation；“read hash then rename”不满足。exclusive_write_window 必须阻止本威胁模型中的全部 writer 修改/replace/delete；仅 advisory lock 不满足。

如果既有 target 没有 conditional_replace/exclusive_write_window，prepare 可以形成 Draft/after pin，但 commit 在触碰 current 前返回 install_unavailable/preflight 或保持 planned+paused，绝不报告 reliable success。create_only 只用于 expected absent。

## 3. InputDescriptor/2、PinRef/2 与 PreparedIntent/2

### 3.1 PinRef/2

PinRef 是受管内部 closed object，不接受客户端自报：

~~~json
{"kind":"d6_pin_ref","version":2,"pinToken":<Token>,"payloadKind":"exact_source_document|resource_bytes|annotation_value|portable_metadata|effect_bytes|artifact","byteLength":<Counter>,"sha256":"64-lowercase-hex","retentionClass":"recovery|conflict|external_unknown|approval_money|query_preview|import_export|user_history"}
~~~

source pin 另保存完整 SourceVersion/2 与 EntityRef 于受保护 record，不在公开 PinRef 增添可选形状。pinToken 的 tag 固定 d6_pin/2。

### 3.2 OwnerInputBinding/2

OwnerInputBinding/2 exact semantic record：

~~~json
{"kind":"d6_owner_input_binding","version":2,"protocolOwner":"D6|D7|D8|D9","ownerKind":<controlled-text>,"canonicalDescriptorBytes":<immutable-bytes>,"pinRefs":[<PinRef/2>...]}
~~~

ownerKind 的闭集由实际 owner version冻结；D6不得接受自由 callback。canonicalDescriptorBytes 必须是对应 owner 新版本的完整 canonical descriptor，其中大型 source/resource字段被 PinRef typed slot替代；不能只存一个 digest。pinRefs 按 token bytes排序、唯一，并与 descriptor slots一一对应。D7/D8/D9 afterimage 未完成前，其新 ownerKind 均不可用于 managed v2 success。

### 3.3 InputDescriptor/2

InputDescriptor/2 exact：

~~~json
{
  "kind":"d6_input_descriptor",
  "version":2,
  "workspaceRef":<WorkspaceRef>,
  "commitDomain":<CommitDomain/2>,
  "intentKind":<controlled-text>,
  "saveProfile":"ordinary|complete|control_only",
  "guarantee":"replica_local|managed_atomic",
  "expectedFrontier":<Frontier/1>,
  "sourceInputs":[{"entityRef":<EntityRef>,"sourceVersion":<SourceVersion/2>,"role":"before|dependency"}...],
  "controlInputs":[{"kind":<closed-control-kind>,"version":<Counter>,"key":<owner-closed-key>}...],
  "ownerInput":<OwnerInputBinding/2>
}
~~~

sourceInputs 按 EntityRef canonical key、role rank before=0/dependency=1 排序，pair唯一；
  controlInputs 的 kind 闭集前四项为 policy|registry|placement_range|lifecycle_range；
  其余为 relation_range|calendar_scope|replica_registry|conflict_record|execution_resource，并按 kind rank+owner key排序。key 使用各 owner 已有 closed value；无 free JSON path。完整相等要求 InputDescriptor canonical bytes、owner descriptor bytes 和每个 referenced exact pin都相等；sha256 相等本身不等于输入相同。

### 3.4 PreparedIntent/2

PreparedIntent/2 受管 immutable record exact semantic members：

kind,version,planToken,operationId,workspaceRef,commitDomain,principalAudienceToken,inputDescriptor,beforeCut,proposedState,
  mutationFootprint,dependencyProof,observationProof,budgetBinding,pinDirectory,installationPlan,expiresAt,previewBinding。

kind=d6_prepared_intent，version=2。planToken tag=d6_plan/2。pinDirectory 是全部 PinRef/2 加其受保护 exact bytes/source bindings。installationPlan 固定 component write set、planned ChangeId/SourceVersion/metadata versions、BackendQualification要求和 portable records；prepare 后不得重采样为另一个 after。previewBinding 只引用所属 owner 的完整 preview，不由 D6 发明第二效果格式。

PreparedIntent/2 本身不产生 author decision。未被任何 decision引用的 record 到期后可按 retention 释放大 pins；一旦 planned/saved 或被 external unknown/conflict/approval-money 保护，last-reference pin 不受 preview TTL 清理。

## 4. D6 v2 准备与提交请求

### 4.1 existing Document source save prepare

D6 自己拥有的 ordinary/complete existing-Document source prepare入口 exact：

~~~json
{
  "wireVersion":2,
  "kind":"d6_source_save_prepare",
  "workspaceRef":<WorkspaceRef>,
  "commitDomain":<CommitDomain/2>,
  "ownerNodeRef":<NodeRef>,
  "expectedSourceVersion":<SourceVersion/2>,
  "expectedFrontier":<Frontier/1>,
  "saveProfile":"ordinary|complete",
  "guarantee":"replica_local|managed_atomic",
  "source":<text>,
  "budget":<BudgetBinding>
}
~~~

ownerNodeRef 必须同 Workspace，expectedSourceVersion.entityRef=ownerNodeRef 且 domain相等。source 是 proposal 输入；Core immediately pins exact UTF-8 bytes，后续 canonical commit request不含它。外部 physical invalid bytes不通过本入口；走 repair/external observation。

ordinary profile：D2必须 valid，实际 local typed facts必须通过；无法证明的允许 obligation 形成 semantic_pending。complete profile：所有适用 D4/D5/D7 complete obligations 必须通过，否则 semantic_rejected/dependency/availability；不能降级 ordinary。

成功：

~~~json
{"wireVersion":2,"kind":"d6_prepared_intent","planToken":<Token>,"semanticState":<SemanticState/1>}
~~~

这里只说明 prepared；不是 saved。

D3 identity/lifecycle/create/move/reorder/Trash/restore/purge 禁止使用本入口。它们必须等待 D3 对 CommitDomain/profile 的 owner afterimage；本 D6 不构造“等价 D3 patch”。

### 4.2 commit request/2

d6_commit_request/2 exact：

~~~json
{"wireVersion":2,"kind":"d6_commit_request","operationId":"uuid-v4","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"expectedDomainFenceToken":<Token>,"planToken":<Token>}
~~~

commit request不携带 source、patch、budget、preview或费用 override。expectedDomainFenceToken tag=d6_domain_fence/2，
  绑定当前 CommitDomain qualification：replica 时绑定 active ReplicaEpoch、portable registry/policy/backend epoch；server 时还绑定当前 D3 authority/custody/fence generation。它不是permission。

v2 ledger key恰为 (workspaceId, D3-CJ/3(commitDomain), operationId)；record另保存 protocolOwner=D6。相同 key 不同 canonical request固定 operation_id_conflict。D3 future wire12与D6共享同 v2 key时 protocol owner同样互斥；在 D3 afterimage完成前不声明已实现。

## 5. 提交、安装、seal、发布的唯一顺序

v2 commit唯一顺序：

1. closed decode/static Workspace/domain equality；失败 invalid_request/preflight，零业务 state read。
2. 当前 authenticated principal 对所请求目标的 state disclosure、潜在 observation profile 与实际 commitDomain 使用资格；只可读最小 token/audience/domain定位映射。失败 not_visible/preflight。
3. CommitDomain qualification：replica 检 active ReplicaEpoch、portable registry/policy、
  backend identity/observation continuity；server 另检 authority/custody/fence。
    不可证明 domain_unavailable，证实损坏 integrity_conflict。
4. 读 v2 ledger key。different owner/request→operation_id_conflict。saved decision在当前 replay授权通过后返回原 bytes；planned恢复原 plan。replay不要求旧 preview TTL有效。
5. unseen 时 expectedDomainFenceToken必须 current；验证 planToken tag/audience/Workspace/domain/期限，读取完整 PreparedIntent/2。missing/wrong audience/tag统一not_visible；已知本人过期 plan_expired。
6. 重验 expectedFrontier、before source/control versions、actual MutationFootprint write permission、D2/local或complete语义proof、预算与全部未写 dependency；确定业务冲突可 recorded rejection，不确定 availability不写永久 rejection。
7. planning CAS 同时比较 ledger unseen、domain fence、expected Frontier、必要依赖/授权，
  保存 fixed plan、pins、planned ChangeId/poststate 与 installation recovery；CAS loser回第3步。
8. 在任何 current 文件修改前写 durable InstallationNotice/1；失败保持 planned。
9. 按 installationPlan 逐 component 使用 BackendQualification。发现 expected binding不符→conflict/paused；backend不能证明安全 install→install_unavailable/paused；不得覆盖 unknown bytes。
10. installed verification：written targets比较**planned poststate**；不再拿原 before 当 current。
  unwritten positive/negative dependencies、policy/auth、Registry/rules、frontier control 重新验证；自产生版本与 plan内 planned version比较。
11. seal：全部成立后，P 单一 durable transaction写 committed、receipt、ReliableSaveState、
  decision effects metadata、适用 ApprovalUse/Money charge、outbox。此处是 author decision commit point。
12. portable publication：写/flush ContentCompletionProof/1，推进 portable Frontier。失败不回滚第11步；current state为 reliable+publication_pending。
13. response delivery：重新 current授权；撤权可以遮蔽 receipt delivery，但不改变 committed decision。

步骤9-12之间的 crash 不由客户端猜结果，使用 §8 状态读取/恢复。D6 v1原顺序按原 decoder保留；不能用本顺序重解释旧 saved decision。

## 6. PortableComponentKey、InstallationNotice 与 ContentCompletionProof

### 6.1 PortableComponentKey/1

closed union：

- {"kind":"document","ownerNodeRef":NodeRef}
- {"kind":"resource","resourceRef":ResourceRef}
- {"kind":"annotation","annotationRef":AnnotationRef}
- {"kind":"node_binding","nodeRef":NodeRef}
- {"kind":"child_list","parentNodeRef":NodeRef}
- {"kind":"lifecycle","ref":EntityRef}
- {"kind":"trash_membership","nodeRef":NodeRef}
- {"kind":"policy","workspaceRef":WorkspaceRef}
- {"kind":"registry","workspaceRef":WorkspaceRef}
- {"kind":"period_scope","nodeRef":NodeRef}
- {"kind":"replica_registry","workspaceRef":WorkspaceRef}
- {"kind":"conflict","conflictId":ConflictId}

SeriesScopeConfiguration 等复杂控制仍由其 owner change/effects证明；本批不以 generic string key替代。未来需要新的 portable component kind必须版本化本 union。

ComponentImage/1：

absent：{"state":"absent"}

present：
~~~json
{"state":"present","version":<Counter>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~

version为该 component owner逻辑版本；digest只用于验证 listed bytes，不定义 identity。

### 6.2 InstallationNotice/1

portable immutable record exact：

~~~json
{
  "format":"weftext.installation-notice",
  "version":1,
  "workspaceRef":<WorkspaceRef>,
  "commitDomain":<CommitDomain/2>,
  "changeId":<ChangeId>,
  "operationId":"uuid-v4",
  "guarantee":"replica_local|managed_atomic",
  "frontierBefore":<Frontier/1>,
  "components":[{"key":<PortableComponentKey/1>,"before":<ComponentImage/1>,"after":<ComponentImage/1>}...]
}
~~~

changeId.domain=commitDomain。components 非空，按 PortableComponentKey 固定 rank+canonical key排序、key唯一。notice 在第一个 current component安装前必须 durable；它不是 commit proof，不含 receipt、approval、Money、external payload 或 authority credential。

### 6.3 ContentCompletionProof/1

portable immutable record exact：

~~~json
{
  "format":"weftext.content-completion",
  "version":1,
  "workspaceRef":<WorkspaceRef>,
  "commitDomain":<CommitDomain/2>,
  "changeId":<ChangeId>,
  "operationId":"uuid-v4",
  "guarantee":"replica_local|managed_atomic",
  "semanticState":<SemanticState/1>,
  "frontierBefore":<Frontier/1>,
  "frontierAfter":<Frontier/1>,
  "components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],
  "receiptDigest":"sha256:64-lowercase-hex"
}
~~~

frontierAfter 必须等于 frontierBefore 在 changeId.domain 上推进到 changeId.sequence，且其它 head不回退；首次 domain可新增。components 必须与 notice exact key集合相等并使用 actual sealed after。receiptDigest只绑定 control receipt bytes；接收 replica不凭它取得 receipt读取或 execution authority。proof真实性由 portable trust/record chain和实际 components共同验证；hash不替代完整 proof。

## 7. commit receipt、decision state 与 current source state

### 7.1 d6_commit_receipt/2

exact：

~~~json
{
  "wireVersion":2,
  "kind":"d6_commit_receipt",
  "operationId":"uuid-v4",
  "workspaceRef":<WorkspaceRef>,
  "commitDomain":<CommitDomain/2>,
  "domainCommitSequence":<Counter>,
  "changeId":<ChangeId>,
  "plannedFrontierAfter":<Frontier/1>,
  "guarantee":"replica_local|managed_atomic",
  "reliableSave":"reliable",
  "portablePublicationAtReceipt":"pending",
  "semanticState":<SemanticState/1>,
  "sourceVersions":[<managed SourceVersion/2>...],
  "effectsToken":<Token>
}
~~~

domainCommitSequence 是该 CommitDomain 的 D3/D6 author/control committed decision sequence；fresh domain 0 后首次 commit=1，raw no-op control decision仍按 owner规则增加。不同 domain sequence不可比较为 Workspace全局活动序。以后 D7 commit_sequence_state consumer必须版本化为 domain scoped。

sourceVersions 只列本 decision 实际改变的 source，按 EntityRef canonical key排序、无重复。receipt 是 immutable seal-time bytes，因此 portablePublicationAtReceipt 固定 pending；publication完成后**不改 receipt**，由 current decision state读取 published。raw no-op若没有 content ChangeId 的 owner contract必须使用control-only receipt variant；本文 source save 每个 committed content change有 ChangeId。

### 7.2 d6_decision_state_read

request exact：

~~~json
{"wireVersion":2,"kind":"d6_decision_state_read","protocolOwner":"D6|D3","request":<original-complete-request>}
~~~

D3 v12 request尚未定义，因此当前候选可实际 decode的新版仅 protocolOwner=D6；D3 branch必须返回 unsupported_version直到 D3 owner后像共同生效。使用完整原 request而非裸 OperationId，避免 ledger probe。

成功：

~~~json
{
  "wireVersion":2,
  "kind":"d6_decision_state",
  "protocolOwner":"D6",
  "decisionState":"planned|committed|rejected|terminal_failed",
  "installationState":"planned|installing|installed|conflict|recovery_unknown|paused_authorization|paused_capacity",
  "reliableSaveState":"not_saved|reliable",
  "portablePublicationState":"not_published|pending|published|conflict",
  "decisionSourceVersions":[<SourceVersion/2>...]
}
~~~

decisionSourceVersions 属于原 decision，不是 current workspace source。r5已 committed、current为r6时，本响应仍只显示r5 decision版本；要读r6使用 current source接口。

授权顺序：closed decode→当前原请求重放披露/目标范围→CommitDomain 连续性→账本键/指纹→状态。权限撤销 not_visible，不修改 decision。continuity不可证明 domain_unavailable。original request mismatch统一 state_unavailable，不泄露其它 key。

### 7.3 d6_current_source_read

request：

~~~json
{"wireVersion":2,"kind":"d6_current_source_read","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"entityRef":<NodeRef|ResourceRef|AnnotationRef>}
~~~

先当前 entity/source read权限，再 domain/backend availability，再 current FileBinding/portable metadata。成功 union：

managed：
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"managed","sourceVersion":<managed SourceVersion/2>,"semanticState":<SemanticState/1>,"byteLength":<Counter>}
~~~

external valid pending adoption：
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external","sourceVersion":<external SourceVersion/2>,"validation":"d2_valid|unverified","byteLength":<Counter>}
~~~

external invalid：
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external_invalid","sourceVersion":<external SourceVersion/2>,"validation":"physical_invalid|d2_invalid","byteLength":<Counter>}
~~~

成功对象不返回 full bytes；实际 bytes走对应授权 source/ByteHandle/repair入口。external/unverified不是 managed commit success。

## 8. InstallationState 与 crash recovery

P 内每个 planned v2 decision保存每 component 的 internal InstallComponentState：

pending|staged_durable|installed_after|restored_before|third_state|unavailable。

recovery读取 P 后，对实际 file/component只允许 classify：

- exact_before：完整 trusted binding+bytes与 planned before相等；
- exact_after：完整 trusted binding+bytes与 planned after相等且 installation provenance连续；
- third_state：其它 materialized bytes/object；
- unavailable：不能安全读取/证明。

digest相同但 provenance不成立不能升级 exact_after。

若 written component exact_after，commit step10以 planned after作为 expected current；未写 dependency继续与 before/cut expectation比较。这一分离是必须的，禁止“安装后再次要求所有 expectedSourceVersion仍为 before”导致正常 save自冲突。

planned恢复只复用原 InputDescriptor、pins、ChangeId、versions、OperationId与 budget counters。无法证明 planned pin/clock/domain continuity保持 planned+paused/recovery_unknown；不能新 prepare代替原 decision。

## 9. ConflictKey/1、ConflictRecord/1 与 resolution prepare

### 9.1 ConflictSubject/1 与 kind

ConflictSubject closed：
- {"kind":"workspace","workspaceRef":WorkspaceRef}
- {"kind":"entity","ref":EntityRef}

conflict kind闭集：
冲突 kind 闭集前四项为 source_concurrent | placement_concurrent | lifecycle_concurrent | identity_collision；
其余为 policy_concurrent | incomplete_transport | placeholder。

### 9.2 ConflictKey/1 与 ConflictId

ConflictKey exact：

~~~json
{"workspaceRef":<WorkspaceRef>,"kind":<conflict-kind>,"subjects":[<ConflictSubject>...],"heads":[<ChangeId>...]}
~~~

subjects、heads 均非空、无重复。subjects按 workspace rank0/entity rank1+canonical ref排序；heads按 ChangeId key排序。所有成员同 Workspace。placement/lifecycle/source/identity至少含一个 entity subject；policy至少含 workspace subject。incomplete/placeholder按实际受影响 workspace/entity列出。

ConflictId 是 ASCII d6c: + 64 lowercase hex，其中 hex=SHA-256(ASCII "D6-ConflictKey/1" + NUL + D3-CJ/3(ConflictKey))。ConflictId只作稳定地址；验证时必须加载并 byte-compare完整 ConflictKey。

ConflictRecord portable exact：

~~~json
{"format":"weftext.conflict","version":1,"conflictId":<ConflictId>,"key":<ConflictKey/1>,"state":"open|resolution_prepared|resolved|superseded","createdAtFrontier":<Frontier/1>,"supersedes":[<ConflictId>...]}
~~~

supersedes 可空，按 ASCII排序唯一。新 head出现时旧 open/resolution_prepared 变 superseded并建立新record；resolved 历史不改写。

### 9.3 conflict read

~~~json
{"wireVersion":2,"kind":"d6_conflict_read","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>}
~~~

先 conflict_read capability与所有 subject最低state disclosure，再读取record；无权/unknown统一not_visible。读取 record 不授 source bytes。

### 9.4 conflict resolution prepare

request exact：

~~~json
{"wireVersion":2,"kind":"d6_conflict_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/1>,"budget":<BudgetBinding>}
~~~

ConflictResolution/1 closed：

- source_merge：{"kind":"source_merge","ownerNodeRef":NodeRef,"source":text}
- choose_source_head：{"kind":"choose_source_head","ownerNodeRef":NodeRef,"head":ChangeId}
- policy_choice：{"kind":"policy_choice","policy":<Policy/3>}
- owner_resolution：{"kind":"owner_resolution","owner":"D3",
  "action":"placement|lifecycle|identity_fresh_copy"}

owner_resolution 没有 free payload，只表示必须转交真实 D3 owner；在 D3 wire12后像未完成前 prepare 返回 owner_update_required，不产生planToken。未来 D3 adapter必须把具体 typed D3 request/preview另行冻结，D6不能在此字段塞任意 JSON。

source_merge/choose_source_head 重新读取全部 heads/base/current授权并做 D2/local/complete gate；policy_choice需要当前 policy_admin 且不能做 grant union fallback。成功产生普通 PreparedIntent/2，最终仍走 d6_commit_request/2。新 heads使 expectedKey失配→conflict_changed；不得沿用旧点击。

## 10. Policy/3

Policy/3 exact top-level仍为 version,revision,grants；version=3。Policy/1/2保留原 decoder，不自动升级。grant仍为 subject,effect,scope,capabilities；deny优先、默认拒绝。原 Policy/2 的全部 capability逐字保留。

新增无参数 capability：

replica_register | replica_retire | conflict_read | conflict_resolve | execution_custody_admin。

新增能力不蕴含任何 source/Field/body/lifecycle 权利：

- replica_register/retire 只允许 workspace scope policy_admin语义下管理 portable replica registry；还须当前 trust/bootstrap条件；
- conflict_read允许读取已获 state scope内的 ConflictRecord，不授 conflict source bytes；
- conflict_resolve只允许进入对应 resolution prepare，实际 source/policy/D3 write仍分别需要原 write capability；
- execution_custody_admin只允许执行域 continuity/takeover管理，不授 Money扩大、approval新增或 author source写。

source_write deny继续阻止完整源修改；Field/body deny继续约束对应 footprint。write不蕴含read。普通 source-save ordinary profile仍需实际 source/body/field/node-control矩阵及潜在观察授权，不能用 conflict_resolve/replica capability绕过。

commit_sequence_state 在 Policy/3 中改为 CommitDomain-scoped metadata读取：request必须给完整 CommitDomain；它只公开该 domainCommitSequence，不提供跨离线 replica 的虚构全局序号。Policy/2历史 consumer继续原 Workspace-wide定义，只用于旧 saved/旧 contract path，不迁移其语义。

## 11. BudgetBinding/2 与 pin capacity

BudgetBinding/1原成员与数值域保留。v2 PreparedIntent另绑定 PinBudget/1：

~~~json
{"version":1,"maxRecoveryBytes":<Counter>,"maxConflictBytes":<Counter>,"maxPreviewBytes":<Counter>,"maxImportExportBytes":<Counter>,"maxHistoryBytes":<Counter>}
~~~

0表示该类不允许新增，不表示无限。实际额度取 request/policy/host min。每次 pin allocation前 checked-add并持久 reservation；同 plan所有attempt共享counter。work units仍先 durable charge后执行，crash不退款。temporary staging与 protected pins分别计费，不把 worker局部视图当总占用。

protected last-reference pin不能因 TTL/preview expiry被删。容量不足：新 prepare/install返回 budget_exceeded或 planned paused_capacity；不能通过删除 planned/unknown/conflict last-reference evidence继续。

## 12. Result/ByteHandle 与 index消费

D6 ResultHandle/ResultCursor、Resource ByteHandle/ByteRead 的原 closed wire1保持历史形状。新 file-backed implementation必须把其 immutable pins绑定 SourceVersion/2/CommitDomain/Frontier，并在新 consumer version明确 reset条件；本批不修改旧 wire1 saved handle。

新 Query/Result producer在 D7后像前不可激活。原则固定：

- current authorization先于index/result内容；
- complete result只能由 complete scope proof产生；
- building/partial index不能将缺项当 empty；
- exact/NFC/regex 的 candidate index必须证明无漏召回，否则补扫 source；
- result/action evidence不得把旧 CommitDomain/revision number绑定为另一个 domain current source；
- auth generation、observation gap、frontier dependency loss按 owner version reset。

## 13. Replica registration/read

replica registration prepare request：

~~~json
{"wireVersion":2,"kind":"d6_replica_register_prepare","workspaceRef":<WorkspaceRef>,"expectedReplicaRegistryRevision":<Counter>,"displayLabel":<text>,"budget":<BudgetBinding>}
~~~

displayLabel只作受权显示，不是identity/path；不得为空，UTF-8 ≤256 bytes。要求 workspace scope replica_register，当前 portable trust chain与完整 current registry可验证；无 authority takeover。成功 PreparedIntent/2 中 Core mint从未使用的新 ReplicaEpoch，plan写 portable ReplicaRecord active及registry revision+1。最终 commit使用本地 bootstrap CommitDomain，它在成功 registration seal 前不能执行 ordinary content；具体 bootstrap domain为host私有，不对外作为可复用 CommitDomain。

retire prepare：

~~~json
{"wireVersion":2,"kind":"d6_replica_retire_prepare","workspaceRef":<WorkspaceRef>,"replicaEpoch":"uuid-v4","expectedReplicaRegistryRevision":<Counter>,"expectedState":"active","budget":<BudgetBinding>}
~~~

要求 replica_retire。retire 不删除该 replica已发布 ChangeRecord/bytes，不撤销其历史；只阻止该 epoch未来作为 active ordinary writer。若它仍拥有 global execution responsibility，必须先按 execution custody协议完成/暂停；不能通过 retire偷偷退款或转移 Money。

## 14. Error/Disposition v2

d6_error/2 exact：

~~~json
{"wireVersion":2,"kind":"d6_error","code":<code>,"disposition":"preflight|recorded|paused|terminal"}
~~~

code闭集：
错误 code 闭集第一组：invalid_request | unsupported_version | not_visible | domain_unavailable；
第二组：integrity_conflict | operation_id_conflict | plan_expired | dependency_conflict；
第三组：semantic_rejected | budget_exceeded | install_unavailable | conflict；
第四组：conflict_changed | state_unavailable | owner_update_required | effects_unavailable | transaction_aborted。

规则：

- planning 前已知的 invalid_request/unsupported_version/not_visible/domain_unavailable 仅 preflight；
  integrity_conflict/operation_id_conflict/plan_expired/install_unavailable 也仅 preflight；
- dependency_conflict/semantic_rejected/budget_exceeded 在第6步且确定为该 canonical intent业务失败时可 recorded；
- install中竞争、撤权、容量或恢复不确定使用 paused，不写 rejected/terminal；
- conflict_changed 是 prepare preflight；
- owner_update_required 是未完成owner后像的 preflight，不创建 decision；
- planned后只有证明原 plan永不提交且安装残留已安全处置时，transaction_aborted+terminal；
- P seal已经committed后 portable publication pending不是 error，不得返 transaction_aborted。

错误不得附自由 details/hidden counts/original request。受权 audit可另读内部cause。

## 15. Current authorization and non-disclosure order

所有 v2入口共同遵守：

1. closed decode/static cross-field relation；
2. 当前 authenticated principal 的最小 capability/scope与 state-disclosure gate；
3. CommitDomain/replica/server qualification、fence、portable trust/backend availability；
4. 适用 ledger/token/record audience；
5. 当前 target/source/control existence/version；
6. 业务 dependency/semantic/budget；
7. mutation/install/decision；
8. 输出前 current authorization。

不得为判断“这次恰好没有秘密”先读取隐藏作者事实。ref/path/digest由调用方知道不授 existence。conflictId、ChangeId、SourceVersion、planToken 都不是 capability。

Policy/ACL change后 old session/prepared不是 grandfather ticket。撤权对未seal plan阻止 seal；对已committed decision只遮蔽后续receipt/effects交付，不篡改历史。

## 16. Global execution responsibility and control inspection

D6 v2 ExecutionResponsibilityRecord 是 P 内受保护 control，不进入 portable metadata。exact semantic fields：

kind,version,workspaceRef,executionDomainId,holder,revision,status,
  approvalUses,claims,moneyLineage,externalUnknowns,stopState,lastContinuityProof。

kind=d6_execution_responsibility，version=2。executionDomainId是Core mint UUIDv4，独立于 CommitDomain/ReplicaEpoch。
  holder closed为 local_replica{replicaEpoch} 或 server{authorityInstanceId,
    deploymentId}；deploymentId为host受保护 UUID，不是作者identity。revision checked+1。

approvalUses/claims/moneyLineage/externalUnknowns的完整子schema继续由D10/Money owner后像冻结；本文件只规定它们必须同 execution domain连续持久、takeover不得省略。未知子schema时对应能力 unavailable，不能存自由 JSON。

execution takeover准备要求 execution_custody_admin、完整旧 responsibility record、受保护 backup/remote handoff proof、旧holder已fenced；失败 domain_unavailable/semantic_rejected，不建立空record。takeover不复制或重置额度。sourceOccurrenceKey continuity需要D10完整 source occurrence证明；仅 path/hash/Field key相同不可续认。

Control inspection v2 target新增：

- decision_state：必须携原完整request；
- current_source：完整 EntityRef；
- conflict：ConflictId；
- replica：ReplicaEpoch；
- execution_resource：保留旧 target owner/OperationId；
- execution_responsibility：executionDomainId，仅 execution_custody_admin/audit。

每个 target按本文各自授权，不提供“列出全Workspace所有OperationId/unknown”通用API。

## 17. Legacy replay and coordinated activation

D6 wire1 commit/receipt/error、Policy/1/2、SourceVersion/1、旧 Token tag、旧 PreparedIntent、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1 及其 planned/saved decisions继续原 bytes、原fingerprint、原授权与continuity、原 pin retention重放/恢复。禁止：

- 把旧 request重编码成wire2；
- 给旧 receipt补 CommitDomain/ChangeId；
- 用新 semantic_pending解释旧 D4 gate；
- 用新 retention删除旧协议要求仍存在的pins；
- 因相同 source hash把旧 SourceVersion/1当成新版某 domain version。

新 v2 consumer尚未全部生成：D3/D4/D5/D7/D8/D9/D10缺失的owner afterimage必须先完成并联合接受。当前候选不得在产品或测试 fixture中产生“v2 managed commit成功”作为已激活语义。作者文档检查只验证文档本身，不是该联合门的替代。

