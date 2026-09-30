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

### 1.4 DecisionKey/2、ChangeId/1 与 Frontier/2

DecisionKey/2 exact：
~~~json
{"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"operationId":"uuid-v4"}
~~~
P ledger 仍以 workspaceId、D3-CJ/3(commitDomain)、operationId 索引。protocolOwner 不进入键，而在同一键内恰一 D3|D6 owner；同键换 owner 或 canonical request 固定 operation_id_conflict。

ChangeId/1 exact：
~~~json
{"commitDomain":<CommitDomain/2>,"sequence":<Counter>}
~~~
sequence=1..MAX。portable effect 的 ChangeId 只在 P seal transaction 中 checked 分配；prepare/planning/staging/InstallationNotice 不预留成功序号。失败、paused、recovery_unknown、control_only、真正 no_op 都不产生 content ChangeId。ChangeId不是OperationId、EntityRef、sourceOccurrenceKey或全局时间。

Frontier/2 exact：
~~~json
{"kind":"d6_frontier","version":2,"heads":[<ChangeId/1>...]}
~~~
heads可空；非空按CommitDomain canonical key排序且domain唯一，表示已验证连续sealed因果记录前缀。它不证明payload物化、placeholder下载、Query完整scope或execution responsibility。frontierPolicy闭集 exact|scope_dependencies：exact要求完整Frontier equality；scope_dependencies只允许可证明非回退的无关扩展并重验原source/control/authorization/positive-negative dependency scope，禁止换target/Query/source/canonical request。Frontier/1仅历史decoder/replay。

### 1.5 SourceVersion/2、SourceObservation/1 与 SourceVersionRef/1

SourceVersion/2 closed union保持完整：

managed：
~~~json
{"kind":"managed_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"revision":<Counter>,"changeId":<ChangeId/1>}
~~~

external：
~~~json
{"kind":"external_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"externalSequence":<Counter>}
~~~

observationEpoch/revision/externalSequence均1..MAX。managed changeId.commitDomain等于生产commitDomain；Document entityRef为owner NodeRef，Resource/Annotation使用完整Ref。managed revision只在生产domain+entity真实managed source change上+1，fresh=1，raw no-op不增。observationEpoch在外部观察连续性不可证明时增加，即使最终bytes相同；externalSequence在同epoch内每个无法归因到已知ChangeRecord的新外部状态增加。managed/external永不相等，不同生产CommitDomain裸revision不可比较。

当前副本/Server如何观察一个来源由SourceObservation/1独立绑定：
~~~json
{"kind":"d6_source_observation","version":1,"observerDomain":<CommitDomain/2>,"entityRef":<EntityRef>,"sourceVersion":<SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<FileObjectBinding/1>,"evidencePins":[<PinRef/2>...]}
~~~
observerDomain必须等于当前operation CommitDomain，entityRef=sourceVersion.entityRef；evidencePins按token排序唯一。placeholder、缺metadata、冲突branch、观察连续性不可证明时没有成功SourceObservation。sourceVersion.commitDomain可以与observerDomain不同。

窄公开版本投影SourceVersionRef/1：
~~~json
{"entityRef":<EntityRef>,"sourceToken":<Token>}
~~~
sourceToken tag=d6_source_observation/1，选择完整受保护SourceObservation，不是裸revision/digest/生产SourceVersion。watcher gap、external replace、不连续rematerialize使旧token失效。legacy D2/D3 revision-token词法owner不变。

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

## 2. FileObjectBinding、WriteProtection 与安装资格（受信内部类型）

FileObjectBinding/1 永不接受客户端构造。

absent：
~~~json
{"kind":"absent","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"parentGenerationToken":<Token>}
~~~
present：
~~~json
{"kind":"present","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"objectGenerationToken":<Token>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~
PortableRelativePath 保持非空 UTF-8 路径，使用“/”分隔，并禁止绝对路径、空段、“.”、“..”、NUL、平台别名和 reparse 跳转；host 仍须证明规范路径位于允许范围内，path 本身不是 identity。

WriteProtection closed enum strict|observed_only，与ContentGuarantee/SemanticState/ReliableSaveState分域。

FileInstallCapability/2：
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
或
~~~json
{"kind":"observed_replace","observedObjectGenerationToken":<Token>}
~~~
前三者strict：conditional_replace必须真实trusted generation，read-hash-then-rename不是CAS；exclusive必须排除威胁模型内全部writer，advisory lock不合格；create_only只expected-absent。observed_replace不是CAS，只记录最后验证观察且仅§4.1 observed_only资格可用。

全部路径仍证明staged bytes、installed data、必要directory-entry/rename durability、containment、object type与installation provenance。已观察竞争、失权、细项deny、Core竞争、unknown install、P continuity缺失都不因observed_only豁免；strict不得原地降级。

## 3. InputDescriptor/2、PinRef/2 与 PreparedIntent/2

### 3.1 PinRef/2

PinRef 是受管内部 closed object，不接受客户端自报：

~~~json
{"kind":"d6_pin_ref","version":2,"pinToken":<Token>,"payloadKind":"exact_source_document|resource_bytes|annotation_value|portable_metadata|effect_bytes|artifact","byteLength":<Counter>,"sha256":"64-lowercase-hex","retentionClass":"recovery|conflict|external_unknown|approval_money|query_preview|import_export|user_history"}
~~~

source pin 另保存完整 SourceVersion/2 与 EntityRef 于受保护 record，不在公开 PinRef 增添可选形状。pinToken 的 tag 固定 d6_pin/2。

### 3.2 OwnerInputBinding/2

~~~json
{"kind":"d6_owner_input_binding","version":2,"protocolOwner":"D3|D6|D7|D8|D9","ownerKind":<controlled-text>,"canonicalDescriptorBytes":<immutable-bytes>,"pinRefs":[<PinRef/2>...]}
~~~
protocolOwner是input descriptor owner，不等于最终decision owner；DecisionKey仍只D3或D6 decision。ownerKind由对应owner version冻结，禁止free callback/JSON；descriptor完整保存，小心大bytes仅typed PinRef slot。D3预留d3_identity_operation/12，配套consumer完成前owner_update_required；D7/D8/D9同理。

### 3.3 ObservationScope/2

ObservationScope/2 的闭合 profile 分为六类，所有字段都受对应 owner 的闭合类型约束：
- local_source{workspaceRef,commitDomain,ownerNodeRef}：单一源观察；
- owner_fields{workspaceRef,commitDomain,ownerNodeRef,fieldIds}：指定字段观察；
- local_structure{workspaceRef,commitDomain,operation,subjects,parentCandidates}：局部结构观察；
- workspace_constraints{workspaceRef,commitDomain}：工作区约束观察；
- control_only{workspaceRef,commitDomain}：仅控制事实观察；
- prepared_workspace{workspaceRef,commitDomain}：已准备工作区观察。
local_structure.operation 的闭合取值为 create_node|move_node|reorder_node|trash。该 scope 只是观察上界，不是写入权限、实际写集或完整性证明。Policy/3 明确提供 structure_state 与 portable_frontier_state，分别只披露可移植结构状态和完整 Frontier/2。

### 3.4 DependencyProof/2

~~~json
{"kind":"d6_dependency_proof","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"baseFrontier":<Frontier/2>,"entries":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>},"evidencePins":[<PinRef/2>...]}...]}
~~~
DependencyProof/2 的 entries 按 canonical 顺序唯一排列，stamp.epoch 表示范围连续性世代；删除 I、发生 watcher gap、重建或 owner version 变化时都不得复用旧世代。
DependencyKey/2 的闭合集合前半包括以下项目：
source、lifecycle、placement_range、ref_inbound；
relation_incidence、calendar_scope、registry。
闭合集合后半包括 temporal_rules、authorization、foreign_binding、query_scan；
以及 replica_registry、conflict_record、execution_resource。
每类都使用对应 owner 的闭合 key，不接受 free JSON。
即使范围为空，也必须有完整枚举和 stamp；index-ready 或 hash 本身都不能证明 complete。

### 3.5 InputDescriptor/2

~~~json
{"kind":"d6_input_descriptor","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"intentKind":<controlled-text>,"saveProfile":"ordinary|complete|control_only","guarantee":"replica_local|managed_atomic","expectedFrontier":<Frontier/2>,"frontierPolicy":"exact|scope_dependencies","observationScope":<ObservationScope/2>,"sourceInputs":[{"entityRef":<EntityRef>,"observation":<SourceObservation/1>,"role":"before|dependency"}...],"controlInputs":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>}}...],"ownerInput":<OwnerInputBinding/2>}
~~~
source/control inputs canonical排序唯一；完整equality比较descriptor、owner descriptor、exact pins、SourceObservation，hash相同不足。

### 3.6 PreparedIntent/2

PreparedIntent/2 的不可变成员分为三组：
身份与上下文为 kind、version、planToken、operationId、workspaceRef、commitDomain、principalAudienceToken、inputDescriptor、beforeCut；
拟议状态与证明为 proposedState、mutationFootprint、dependencyProof、observationProof、budgetBinding、pinDirectory；
安装与交付为 installationPlan、inputRetentionState、expiresAt、previewBinding。

kind 固定为 d6_prepared_intent，version=2；planToken 使用 d6_plan/2 标签。inputRetentionState 的闭合取值为 not_retained|retained|unavailable；只有 proposal、实际读取的 read-before 和必要 bindings 全部形成 durable pin 后才能进入 retained，prepared/retained 都不表示 saved。
installationPlan 固定 write set、after bytes、owner versions、recovery、required WriteProtection 和 portable records；不预分配 ChangeId，也不在 commit 时重新采样。previewBinding 只引用 owner preview；last-reference pin 继续遵循原有 protection/retention。

## 4. D6 v2 准备与提交请求

### 4.1 existing Document source save prepare

~~~json
{"wireVersion":2,"kind":"d6_source_save_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"ownerNodeRef":<NodeRef>,"expectedSourceToken":<Token>,"saveProfile":"ordinary|complete","guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","source":<text>,"budget":<BudgetBinding>}
~~~
ownerNodeRef属于Workspace；expectedSourceToken tag=d6_source_observation/1且当前授权下选择完整SourceObservation/1。source立即exact pin，commit不携带source。physical-invalid external仍走repair。

ordinary要求D2 valid和实际修改local typed facts；合法未证明complete obligations→semantic_pending。complete要求全部适用D4/D5/D7义务且不降级。

observed_only 的闭合资格同时要求：受信调用类别为 interactive_source_save；目标是一个 existing live Document；保存类型为 ordinary+replica_local；调用者具有完整 source read/replace 权限且没有适用的 body/Field/node-control deny；author source write set 为空或仅包含该 Document；不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity；Draft Base 必须等于 selected SourceObservation。
stale Base、已观察的 external change 或 continuity gap 都必须先进入 conflict/reprepare；noninteractive 只能使用 strict。

owner descriptor：
~~~json
{"kind":"d6_source_save_input","version":2,"invocationClass":"interactive_source_save|noninteractive","expectedSourceObservation":<SourceObservation/1>,"proposedSource":<PinRef/2>,"writeProtection":"strict|observed_only"}
~~~
成功：
~~~json
{"wireVersion":2,"kind":"d6_prepared_intent","planToken":<Token>,"semanticState":<SemanticState/1>,"writeProtection":"strict|observed_only","inputRetentionState":"retained"}
~~~
这里只表示 proposal、read-before 与必要 bindings 已形成 durable pin，并不表示 saved。D3 lifecycle、D7 Action、Automation、approval/Money 都禁止使用 observed_only。

### 4.2 commit request/2

d6_commit_request/2 exact：

~~~json
{"wireVersion":2,"kind":"d6_commit_request","operationId":"uuid-v4","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"expectedDomainFenceToken":<Token>,"planToken":<Token>}
~~~

commit request不携带 source、patch、budget、preview或费用 override。expectedDomainFenceToken tag=d6_domain_fence/2，
  绑定当前 CommitDomain qualification：replica 时绑定 active ReplicaEpoch、portable registry/policy/backend epoch；server 时还绑定当前 D3 authority/custody/fence generation。它不是permission。

v2 ledger key恰为 (workspaceId, D3-CJ/3(commitDomain), operationId)；record另保存 protocolOwner=D6。相同 key 不同 canonical request固定 operation_id_conflict。D3 wire12后像已存在且共享同DecisionKey owner互斥，但G0-A native descriptor/companion consumer尚未更新；完成前新protocolOwner=D3请求owner_update_required，零D3 decision。

## 5. 提交、安装、seal、发布的唯一顺序

1. closed decode/static equality；失败invalid_request/preflight、零业务读。
2. current principal最小disclosure、ObservationScope/2、CommitDomain资格；失败not_visible。
3. domain/fence/P continuity；不可证明domain_unavailable，证实损坏integrity_conflict。
4. 读DecisionKey；different owner/request→operation_id_conflict；saved重放原bytes，planned仅恢复原plan。
5. unseen验证fence、planToken、PreparedIntent/2、inputRetentionState、当前授权/owner version。
6. frontierPolicy下重验Frontier/2、SourceObservation、DependencyProof/2、MutationFootprint auth、语义、budget与未写deps；scope_dependencies只接受无关非回退扩展。
7. planning CAS保存fixed plan/pins/after/recovery/WriteProtection，**不分配ChangeId**。
8. 修改portable current前durable写InstallationNotice/2：DecisionKey、baseFrontier、WriteProtection、component before/after；无ChangeId。
9. install：strict 只能使用 strict capability；observed_only 只适用于 §4.1，并在破坏性安装前执行最后一次 trusted object/event check。若已观察 competition，则进入 conflict/paused 并保留 B/N/current；unknown install 进入 recovery_unknown。
10. verify：written 目标必须等于 planned after，不再要求 before。
unwritten deps、policy/auth、Registry/rules、Frontier/2 和 control facts 都按原 cut 重验。
出现 unknown provenance、late competition 或失权时，保持 paused/conflict/recovery_unknown；此时尚无 ChangeId。
11. seal：P 的单一 durable transaction 重新验证 plan/auth。
portable effect 此时才分配 ChangeId/SourceVersion，并写入 committed decision、receipt、ReliableSaveState、effects、outbox 及适用 charge。
strict 对应 reliable；observed_only 对应 durable_observed_only。
control_only/no_op 对应 not_applicable，且没有 content ChangeId。
这是唯一的 decision commit point。
12. portable publication：sealed portable decision→ContentCompletionProof/2并推进Frontier/2；失败仅pending，恢复不重装N、不换OperationId、不重复收费。
13. delivery重验当前授权；撤权可遮蔽交付不改decision。

observed_only的B只代表实际read/pin before，不枚举未读C。later current=C不改旧receipt，publication不重装N。crash由§8恢复，客户端不猜。D6 v1/legacy按原decoder/order。

真正的 raw no-op 使用 effectClass=no_op。
sourceVersions=[]，InstallationState=not_required，ReliableSaveState=not_applicable，PortablePublicationState=not_applicable。
domainCommitSequence 可增加 1，但不推进 ChangeId、source revision 或 Frontier。
P-only control 使用 control_only；portable F/M 使用 portable。

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

### 6.2 SourceStamp/1 与 InstallationNotice/2

SourceStamp/1：
~~~json
{"kind":"decision_source","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"revision":<Counter>,"observationEpoch":<Counter>}
~~~
安装前可确定，不是SourceVersion/2或第二current truth；只有同DecisionKey committed ContentCompletionProof/2解析到sealed SourceVersion。

InstallationNotice/2：
~~~json
{"format":"weftext.installation-notice","version":2,"decisionKey":<DecisionKey/2>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"before":<ComponentImage/1>,"after":<ComponentImage/1>}...]}
~~~
components非空且固定rank+canonical key唯一排序。observed_only仅§4.1；其它portable author plan必须strict。notice在首个install前durable，不含ChangeId/receipt/approval/Money/external payload/credential。

### 6.3 ContentCompletionProof/2

committed：
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersionRef/1|"absent">,"after":<SourceVersionRef/1|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~
frontierAfter=frontierBefore加入sealed changeId且不回退其它head；components与notice key集合相等并actual after；sourceChanges按EntityRef唯一排序。receiptDigest不授receipt/execution authority。observed_only只证明本decision安装与实际read-before，不证明无未观察竞争。

未seal且全部component安全恢复before，可写restored：
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...]}
~~~
restored禁止ChangeId/receipt/semantic success；无法证明安全恢复时不得生成。

## 7. commit receipt、D3 companion、decision state 与 current source

### 7.1 d6_commit_receipt/2

~~~json
{"wireVersion":2,"kind":"d6_commit_receipt","operationId":"uuid-v4","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"domainCommitSequence":<Counter>,"effectClass":"portable|control_only|no_op","guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","sourceVersions":[<SourceVersionRef/1>...],"effectsToken":<Token>}
~~~
writeProtection 只在 portable receipt 中出现；control_only/no_op 必须省略它且 sourceVersions 为空。portable sourceVersions 按 EntityRef 唯一排序。receipt 不公开完整 Frontier、生产方 SourceVersion、obligations 或决议内部信息；读取完整 Frontier 需要 portable_frontier_state，读取 domainCommitSequence 需要 domain-scoped commit_sequence_state。receipt 的封存时字节不因后续 publication 或 current source 改写。

### 7.2 D3DecisionCompanion/2

~~~json
{"kind":"d6_decision_companion","version":2,"decisionKey":<DecisionKey/2>,"domainCommitSequence":<Counter>,"effectsToken":<Token>}
~~~
不是第二receipt；未来D3 primary receipt与companion同P transaction。B批前new protocolOwner=D3→owner_update_required、零D3 decision。

### 7.3 d6_decision_state_read

request：
~~~json
{"wireVersion":2,"kind":"d6_decision_state_read","protocolOwner":"D6|D3","request":<original-complete-request>}
~~~
success：
~~~json
{"wireVersion":2,"kind":"d6_decision_state","protocolOwner":"D6|D3","decisionState":"planned|committed|rejected|terminal_failed","installationState":"not_required|planned|installing|installed|conflict|recovery_unknown|paused_authorization|paused_capacity","reliableSaveState":"not_saved|reliable|durable_observed_only|not_applicable","inputRetentionState":"not_retained|retained|unavailable","portablePublicationState":"not_published|pending|published|conflict|not_applicable"}
~~~
strict portable 对应 reliable；observed_only 对应 durable_observed_only；control/no_op 对应 not_applicable。inputRetention 独立于保存状态。完整原 request 用于防止 ledger probe；授权顺序仍为闭合解码→disclosure/scope→domain continuity→DecisionKey/fingerprint→state。撤权可返回 not_visible，但不改写历史。

### 7.4 d6_current_source_read

request仍workspaceRef、commitDomain、entityRef，source read permission先于domain/backend/FileBinding。
managed：
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"managed","sourceVersionRef":<SourceVersionRef/1>,"semanticState":<SemanticState/1>,"byteLength":<Counter>}
~~~
external：
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external","sourceVersionRef":<SourceVersionRef/1>,"validation":"d2_valid|unverified","byteLength":<Counter>}
~~~
external_invalid：
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external_invalid","sourceVersionRef":<SourceVersionRef/1>,"validation":"physical_invalid|d2_invalid","byteLength":<Counter>}
~~~
bytes仍走授权source/ByteHandle/repair。需要full version的owner走protected SourceObservation/DependencyProof。

effects metadata 保存 effectClass、WriteProtection 和 owner preview binding；observed_only 的 before 只能称为 observed_before/read_before，不能声称覆盖了所有实际被替换的 external bytes；D7 effects consumer 更新前返回 owner_update_required。

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

### 8.1 observed_only 恢复限定

observed_only read-before只表示实际read/pin前像，不枚举最后检查后未读C。未观察C若被覆盖可能无可恢复副本，这是批准边界，record不得虚构。任何已观察competition仍third_state/conflict；unknown install固定recovery_unknown，相同hash不猜success。committed publication pending只补同一ContentCompletionProof/2，绝不重装N。

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
第二组错误前半：integrity_conflict | operation_id_conflict | plan_expired；
第二组错误后半：source_unavailable | proof_unavailable | dependency_conflict；
第三组：semantic_rejected | budget_exceeded | install_unavailable | conflict；
第四组：conflict_changed | state_unavailable | owner_update_required | effects_unavailable | transaction_aborted。

规则：

- planning 前已知的 invalid_request/unsupported_version/not_visible/domain_unavailable 仅 preflight；
  integrity_conflict、operation_id_conflict 与 plan_expired 也只能属于 preflight；
  source_unavailable、proof_unavailable 与 install_unavailable 也只能属于 preflight；
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
