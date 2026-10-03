---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：`592603c4-c7ef-4572-aee6-256aa3aa7955`。

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 中 D7/D9 的生效标签与 revision05 文本只作为历史来源，不支配本后像；稳定文档 ID 保持。当前本文件作为作者候选已经补齐的 D6 生产者边界包括：完整 managed/external SourceVersion/2 生产历史，当前 observerDomain 的 SourceObservation/1 与 SourceVersionRef/1，SourceStamp/1、受保护内部 SourceRevisionPlan/1、RevisionTokenBinding/2 与 d6_source_revision/2、ContentCompletionProof/3，以及既有 Frontier/2、InstallationNotice/2、ConflictRecord/2 的当前版本边界；DecisionKey/2、InputDescriptor/2、PreparedIntent/2、D3DecisionCompanion/2 等既有 wire 形状仍按正文，不因本状态行升版。生产 CommitDomain 与当前 observerDomain 继续分离，D3 Locator 词法、D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence 与 D5 revision-bound locator 仍归各自 owner。Document 的普通 .adoc 与 Resource 原字节权威、P 中不可重建的执行/恢复事实、可删除重建的 I，以及 ordinary 内容资格与 complete Action 资格的独立边界均不改变。D3 wire11、D6 wire1、Policy/1/2 和实际旧 D7/D8 binding 的保存字节继续按各自原 decoder/gate/pin/continuity 履约。P1 Lexicon/Registry/Impact/routing 后像与完整 P2 D3 main/Lexicon/Impact 候选现已存在，但候选存在不等于独立接受或激活；其余 P2/P3 与 D7–D10 消费者配套及 fresh 联合接受仍未完成。本文件禁止半包激活依赖新版生产者的 managed success，也不得因此永久禁用不依赖这些缺失强路径配套的既定 ordinary/local/offline 操作。

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
sequence=1..MAX。一个真实 portable effect 的 ChangeId 只在 P seal transaction 中 checked 分配，且同一 decision 只分配一次；prepare、planning、staging、InstallationNotice 都不预留成功序号。失败、paused、recovery_unknown、control_only 和真正 raw no_op 都不产生 content ChangeId。source 删除、source 未变但真实发生 placement/lifecycle/structure 等 portable effect 时仍按 portable decision 在 seal 取得 ChangeId，只是不产生新的 managed SourceVersion、也不推进该实体 H(D,E)。已经 seal 的 ChangeId 不因 receipt delivery、outbox 或 portable publication 后续失败而重新分配。ChangeId 不是 OperationId、EntityRef、sourceOccurrenceKey 或全局时间。

Frontier/2 exact：
~~~json
{"kind":"d6_frontier","version":2,"heads":[<ChangeId/1>...]}
~~~
heads 可空；非空时按 CommitDomain canonical key 排序且每个 domain 至多一个 head，表示该 domain 已验证连续 sealed 因果记录的最大前缀。Frontier/2 不证明 payload 物化、placeholder 下载、Query 全集、Registry 完整性、D7 complete cut 或 execution responsibility。Frontier/1 仅按历史 decoder/replay。

frontierPolicy 闭集仍为 exact|scope_dependencies。`exact` 要求当前用于 unseen/planning 的完整 Frontier 与 InputDescriptor.expectedFrontier 逐字相等；D3 managed_atomic，包括未来 D7 媒介生成的 D3 managed_atomic request，继续使用这一 exact 分支，除非 D3 自身以后另行版本化，D6 不单边放宽。

`scope_dependencies` 只允许原 expectedFrontier 到当前 Frontier 的**真实连续、已验证 sealed 因果非回退扩展**。向量上的 sequence 逐项不减只是必要条件，不是充分证明：原有 domain 不能回退或出现 hole；新增/前进的每个 head 必须由完整连续 sealed record 链验证，不能以 provider“已同步”、index ready、mtime、最终 hash/bytes 相同或只比较两个向量数字代替。未知是否连续或未知是否无关时，不得乐观接纳。

选择 scope_dependencies 后，原 canonical request、InputDescriptor.expectedFrontier、DependencyProof.baseFrontier、targets、Query/selector、pins、proposed bytes、MutationFootprint、WriteProtection、owner request 和版本依据都保持冻结，commit 不重新采样或改写。Core 必须逐项重验原完整 source/control/authorization、全部正负范围 DependencyProof、FileObjectBinding/evidence pins、Registry/rules、安装资格、owner version及其它真实依赖，并证明新增 sealed effects 与这些绑定依赖无关。真实 SourceObservation、source token 所选观察、FileObjectBinding、pin、DependencyKey stamp、授权、Registry、relation/calendar/collection membership 或 negative range 等发生变化时，仍按其 owner 规则 stale/conflict/reprepare；scope_dependencies 不能把这些变化洗成“无关”。

证明无关扩展不会重新签发 SourceObservation/1、SourceVersionRef/1.sourceToken、revision token、PinRef 或 DependencyProof stamp，也不会把旧 token 搬到新的 observationEpoch；这些对象只有其各自真实 owner 规则允许时才保持。它同样不要求一个只依赖完整真实局部范围的 ordinary 操作额外等待无关的全 Workspace Query/索引证明。D3 replica_local 的 create/move/reorder/trash 仍按其原 local_structure + scope_dependencies 消费真实局部结构依赖；完整 P2 D3 候选现已定义新版 DependencyKey/恢复细化的消费，独立接受与激活仍受 §17 约束。仍缺实际必要 coordinated consumer 的新分支只在 §5 的 unseen 门保持 owner_update_required/proof_unavailable；本段不替代 D3 owner。

portable decision 的实际 seal 前 Frontier 可以在 scope_dependencies 下包含上述已证明无关扩展；原 expectedFrontier 和 notice.baseFrontier 不随之改写。新 FA ContentCompletionProof/3 记录真实 frontierBefore/frontierAfter，并按 §6.3 与原 notice、连续 change records、P 中保存的无关扩展证据交叉验证。

### 1.5 SourceVersion/2、SourceObservation/1、SourceVersionRef/1 与 revision token profile/2

SourceVersion/2 的既有 closed union 保持不变。

managed：
~~~json
{"kind":"managed_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"revision":<Counter>,"changeId":<ChangeId/1>}
~~~

external：
~~~json
{"kind":"external_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"externalSequence":<Counter>}
~~~

observationEpoch/revision/externalSequence 均为 1..MAX。managed 的 changeId.commitDomain 必须等于其生产 commitDomain；Document 的 entityRef 为 owner NodeRef，Resource/Annotation 使用完整 Ref。external 变体同样保留 kind、version、entityRef、commitDomain、observationEpoch、externalSequence 的完整既有字段；它没有 managed revision 或 changeId，externalSequence 也不得被解释为 managed sourceRevision。

对生产域 D 与实体 E，`H(D,E)` 定义为 D 的连续已封存生产历史中 E 的最大 managed revision。只有从该 CommitDomain 的 birth/registration、P continuity 以及已验证的 portable sealed history 证明“D 从未为 E 封存 managed source version”时，完整空历史才允许 H=0；历史缺失、损坏、发生 gap 或无法证明连续性时不得把未知当空。每次真实产生 managed after 的 source change 使用 checked H+1；同一生产域跨 production observationEpoch 不重置 H，MAX 不 wrap。fresh managed source 在完整空历史上 revision=1。

已有 source 首次由另一生产域写入时，新的 managed after 使用新生产域自己的 H+1，而不是旧生产域 revision+1；此后跨域往返分别继续各域自己的 H。不同生产 CommitDomain 中相同 revision 数字不表示同一版本。true raw no-op 保留原 SourceVersion/2，即使其生产域不同于当前 operation；纯 placement/lifecycle/control 且 source 未变不增加 source revision。删除 source 的 after 是 absent，不创建“删除版本”且不推进 H。equal-byte external admission 是从 external 状态到 managed 状态的显式接纳，即使字节相同也不是 raw no-op，仍按当前生产域 H+1 形成 managed after。externalSequence 永远不参与 managed H 或 inner sourceRevision 的计算。

SourceVersion/2.observationEpoch 是该生产版本保存的生产观察世代；当前 operation 的观察世代由 SourceObservation/1 独立保存，两者数值相同也不合并。当前副本/Server如何观察一个来源仍由 SourceObservation/1 绑定：
~~~json
{"kind":"d6_source_observation","version":1,"observerDomain":<CommitDomain/2>,"entityRef":<EntityRef>,"sourceVersion":<SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<FileObjectBinding/1>,"evidencePins":[<PinRef/2>...]}
~~~
observerDomain 必须等于当前 operation CommitDomain，entityRef 必须等于 sourceVersion.entityRef；evidencePins 按 token 排序唯一。sourceVersion.commitDomain 可以不同于 observerDomain。当前 observationEpoch、FileObjectBinding、evidencePins 及适用 control/Registry/incidence dependency 必须属于同一当前 cut。placeholder、缺 metadata、冲突 branch 或观察连续性不可证明时没有成功 SourceObservation。watcher gap、external replace、object replacement 或 discontinuous rematerialization 必须进入新的当前观察世代；即使生产 SourceVersion、digest 或最终文本相同，也不能恢复旧当前观察资格。

窄公开版本投影 SourceVersionRef/1 保持：
~~~json
{"entityRef":<EntityRef>,"sourceToken":<Token>}
~~~
sourceToken 的 tag 固定 d6_source_observation/1，选择完整受保护 SourceObservation，而不是裸 revision、digest、I cache 或生产 SourceVersion。它不替代 D3/D4/D5 的 inner revision/selector wire。

新决议的 source revision token 由受保护 `RevisionTokenBinding/2` 管理。其 closed shape 为：
~~~json
{"kind":"d6_revision_token_binding","version":2,"token":<Token>,"source":<RevisionTokenSource/2>}
~~~
`RevisionTokenSource/2` 只有两个 variant：
~~~json
{"kind":"managed","sourceStamp":<SourceStamp/1>}
~~~
或
~~~json
{"kind":"external","sourceVersion":<external SourceVersion/2>}
~~~
external variant 必须完整通过 external SourceVersion/2 decoder；managed sourceStamp 只由 §6.2 SourceRevisionPlan/1 或 §6.2.1 具名 conflict-only /2 产生。`RevisionTokenBinding/2` 专职表示稳定生产版本地址：不再携当前 `observerDomain` 或当前 `SourceObservation/1.observationEpoch`，不保存 source bytes，也不授读取、写入、selector、ActionEvidence、Draft 或 preparation 能力。生产 `SourceStamp/1.observationEpoch` 仍属于生产地址。

`token` 使用 §1.1 canonical Token 词法，受保护 tag 固定 `d6_source_revision/2`。managed arm 的完整 SourceStamp 标识拟议生产地址；external arm 的完整 external SourceVersion/2 标识 producer event；两者都不证明当前 observer 资格。


#### 1.5.1 受认证的 managed revision-seal association

仅对 managed source，用来证明 winning seal 确实选中了某一条 exact revision token 的 portable 证据是 closed `RevisionTokenSealAssociation/1`：
~~~json
{"kind":"d6_revision_token_seal_association","version":1,"decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>,"binding":<RevisionTokenBinding/2>}
~~~
所有成员都必需；unknown、duplicate、missing、null、错误嵌套版本/tag、非 managed 的 `sourceVersion` 或非 managed 的 `binding.source` 均拒绝。跨字段必须精确相等：`changeId.commitDomain=decisionKey.commitDomain`；`sourceVersion.changeId=changeId`；`sourceVersion.commitDomain=decisionKey.commitDomain`；`binding.source.sourceStamp.decisionKey=decisionKey`；且 sourceVersion 的 entityRef/revision/生产 observationEpoch 与 binding SourceStamp 的 entityRef/revision/observationEpoch 逐字相等。全部 Workspace 绑定必须一致。token 不从这些字段派生。

portable artifact 为 closed `RevisionTokenSealArtifact/1`：
~~~json
{"format":"weftext.revision-token-seal","version":1,"trustKeyId":"sha256:64-lowercase-hex","association":<RevisionTokenSealAssociation/1>,"signature":"<86-ASCII-unpadded-base64url>"}
~~~
signature 必须解码为恰好 64 字节 Ed25519 signature。`trustKeyId` 精确等于 `"sha256:" + lowercase_hex(SHA-256(raw_32_byte_public_key))`；它只选择 key，本身不提供认证。`RevisionTokenSealSignedBody/1` 是移除 `signature` 成员后的同一 closed object。被认证消息精确为以下 UTF-8 bytes：
~~~text
ASCII "D6-Revision-Token-Seal/1" || NUL || D3-CJ/3(RevisionTokenSealSignedBody/1)
~~~
完整运输/保存 artifact bytes 则精确为 `D3-CJ/3(RevisionTokenSealArtifact/1)`；若接收字节与该 canonical encoding 不逐字相等，接收端拒绝。signature 不覆盖自身、pin digest、CP3 bytes 或未来 outbox address，因此本 profile 不形成自引用环，也不增加第三遍 C/Q materialization。

revision-seal 的 trust root 是 D6 实际拥有的生产端，不再是假定存在的 validator 名称。

`WorkspaceTrustRootDeclaration/1` 是闭合 portable root declaration：
~~~json
{"kind":"d6_workspace_trust_root","version":1,"workspaceRef":<WorkspaceRef>,"establishmentDecisionKey":<DecisionKey/2>,"rootKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","selfSignature":"<86-ASCII-unpadded-base64url>"}
~~~
publicKey 解码为恰 32 字节 Ed25519 key 并哈希得到 `rootKeyId`；`establishmentDecisionKey.workspaceRef` 及其嵌套 CommitDomain Workspace 都必须逐字等于 `workspaceRef`。selfSignature 精确认证 `ASCII "D6-Workspace-Trust-Root/1" || NUL || D3-CJ/3(WorkspaceTrustRootDeclaration/1 with selfSignature removed)`；它只证明持钥，绝不让复制来的 bytes 自授权。

闭合 declaration fingerprint 为：
~~~json
{"kind":"d6_workspace_trust_root_fingerprint","version":1,"profile":"d6_workspace_trust_root_cj3/1","digest":"sha256:64-lowercase-hex"}
~~~
Core 先严格解码 root declaration，并按上文验证 key hash 与 selfSignature；随后定义 `canonicalRootDeclarationBytes=D3-CJ/3(complete WorkspaceTrustRootDeclaration/1 including selfSignature)`。fingerprint digest 精确为 `"sha256:" + lowercase_hex(SHA-256(ASCII "D6-Workspace-Trust-Root-Declaration/1" || NUL || canonicalRootDeclarationBytes))`。任何 transport 拼写、去掉 signature 的 declaration bytes、raw public key 或 `rootKeyId` 都不能冒充这个 digest。

每个需要认证本 Workspace 的 host 有一条受保护、不可 portable 的 `WorkspaceTrustAnchor/1`：
~~~json
{"kind":"d6_workspace_trust_anchor","version":1,"workspaceRef":<WorkspaceRef>,"rootFingerprint":<WorkspaceTrustRootFingerprint/1>,"rootKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","establishedBy":<{"kind":"workspace_bootstrap","issuerAuthorityInstanceId":"uuid-v4","proposalId":"uuid-v4"}|{"kind":"explicit_import","anchorImportId":"uuid-v4"}>}
~~~
anchor 重复 declaration 的 exact Workspace/key tuple，并保存按上述算法重算出的同一 fingerprint object。fresh create/fork 只能通过已经认证的 issuer/target-custody 路径 staged anchor，并且只有同一个 bootstrap P seal 成功后才 usable。

既有 Workspace 只能通过 host-local 闭合 import：
~~~json
{"wireVersion":1,"kind":"d6_workspace_trust_anchor_import","workspaceRef":<WorkspaceRef>,"rootDeclaration":<WorkspaceTrustRootDeclaration/1>,"expectedRootFingerprint":<WorkspaceTrustRootFingerprint/1>}
~~~
写入前必须有受信 local/deployment-operator 认证，并通过带外方式显式确认该完整 fingerprint object。Core strict-decode `rootDeclaration`，要求其 Workspace 与请求 Workspace 逐字相等，从 raw public key 重验 `rootKeyId`，验证 `selfSignature`，再按上文完整 canonical declaration bytes 重算 `WorkspaceTrustRootFingerprint/1`，并要求与 `expectedRootFingerprint` 逐字相等。已有 byte-equal anchor 是 exact replay；不同 anchor/fingerprint 拒绝。import 不写 Workspace author state、P decision、Frontier 或 policy。从 copied/synchronized files 读取 self-signed root，或拿 key fingerprint 替代 declaration fingerprint，都不足以建立信任。
当前 profile 下，既有 `PortableComponentKey/1={"kind":"policy","workspaceRef":...}` 的 exact bytes 是：
~~~json
{"kind":"d6_workspace_authorization_bundle","version":1,"workspaceRef":<WorkspaceRef>,"authorizationRevision":<Counter>,"policy":<Policy/3>,"trustRoot":<WorkspaceTrustRootDeclaration/1>,"trustRevision":<Counter>,"trustDeclarations":[<WorkspaceTrustDeclaration/1>...]}
~~~
`authorizationRevision` 从1开始，每次 policy 或 trust 改变 checked +1；`Policy/3.revision` 只在 policy 改变时增加。`trustRevision` 以初始 domain authorization 的1开始，每追加一条 declaration checked +1。数组累计且必须恰为 revision 1..trustRevision；删除、重排、重复/缺口 revision、改写旧 declaration 都是 integrity failure。一个 portable decision 可以在同一 DecisionKey 下追加固定非空连续 declaration 序列；该整段是一个原子 policy-component transition，同一 activation ChangeId 中间的前缀不是合法 history cut。不新增 Notice/CP3 member 或 PortableComponentKey kind。

revision 1 predecessor 恰为 `{"kind":"root","fingerprint":<WorkspaceTrustRootFingerprint/1>}`，且该 fingerprint 必须与上文 anchor/import 按同一算法得到的值逐字相等；n>1 为 `{"kind":"declaration","revision":n-1,"sha256":<SHA-256(D3-CJ/3(previous declaration))>}`。`rootKeyId` 永远不能放进 predecessor fingerprint 槽。 `WorkspaceTrustDeclaration/1` 是闭合 action union。authorize：
~~~json
{"kind":"d6_workspace_trust_declaration","version":1,"workspaceRef":<WorkspaceRef>,"revision":<Counter>,"predecessor":<predecessor>,"decisionKey":<DecisionKey/2>,"action":"authorize","commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","trustKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","possessionSignature":"<86-ASCII-unpadded-base64url>","rootSignature":"<86-ASCII-unpadded-base64url>"}
~~~
rotate 使用相同前缀，并有 `action:"rotate"`、`replacesTrustKeyId`、`mode:"ordinary|loss_recovery|compromise"`、新 key tuple/possessionSignature、`priorContinuitySignature` 与 rootSignature。ordinary rotation 的 priorContinuitySignature 必须由被替换旧 key 签出；loss_recovery/compromise 时必须精确为字符串 `"not_required"`。revoke 使用相同前缀加 `action:"revoke"`、commitDomain、profile、trustKeyId、`mode:"administrative|loss|compromise"` 与 rootSignature，没有新 key。已有 current key 时再次 authorize、rotate 指错旧 key、revoke 非 current key 均拒绝；revoke 后 authorize 可打开新区间。当前候选第四个 arm `action:"resolve_conflict"` 只归 §9.4 所有，普通 add/rotate/revoke 绝不能产生。

authorize/rotate 的 possession signature 覆盖 `ASCII "D6-Domain-Seal-Key-PoP/1" || NUL || D3-CJ/3({workspaceRef,revision,predecessor,decisionKey,commitDomain,profile,trustKeyId,algorithm,publicKey})`。ordinary rotate 还由旧 key 在 `D6-Domain-Seal-Key-Rotate/1` 域下认证同一 transition。Workspace root 在 `D6-Workspace-Trust-Declaration/1` 域下签完整 declaration（去掉 rootSignature），因此 domain key 丢失/被攻陷时无需坏 key 自己批准被移除。

declaration 不携 caller 可选 effective time。activation ChangeId 唯一来自 `declaration.decisionKey` 的 `ContentCompletionProof/3.changeId`，并且该 proof 的既有 policy component after-image 必须是首次追加该 exact declaration 序列的 bundle；完整 CP3/ChangeRecord chain 必须验证。不得把同一签名 declaration 搬到别的 DecisionKey、事后 backdate、补历史洞或按 arrival order 选择。相同 predecessor 上并发不同 policy/trust successor 形成既有 policy conflict；不允许 LWW，冲突解决前依赖该歧义 bundle 的新签名不可用。

normalized verifier 仍是：
~~~json
{"kind":"d6_revision_token_seal_verification_key","version":1,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","trustKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>"}
~~~
只能从受保护 anchor + exact portable history 派生，不存在额外 validator Boolean 或 caller 字段。`history_at(C)` 是 activation ChangeId 因果包含于已验证 Frontier cut C 的最大完整 trust-revision 前缀；同 DecisionKey、同 activation ChangeId 的连续 declarations 必须全包含或全排除。对 exact (CommitDomain,profile)，普通回放仍是 none -> authorize K -> rotate K→K2 -> ... -> revoke K -> none。§9.4 的 `resolve_conflict` declaration 只有在 selected bundle address、resolvedHeads、root signature、inheritedCompromises 与完整 outcomes 全部验证后才可应用；它对 outcomes 中每个 exact domain/profile 设置 `keep_current|none|authorize_fresh` 结果，未列出的 selected-chain 状态保持不变。`authorize_fresh` 还必须验证 public-key hash 与 possessionSignature。因此线性 successor chain 可执行且不依赖 arrival order 或 current host state。`validate_historical(K,C)` 必须验证 anchored root、全部 signature/predecessor link（含任何 resolve_conflict transition）、K 在该历史 cut 恰为唯一授权 key，以及 artifact/CP3 cross-check。后来的 ordinary rotate/revoke 不追溯改写 C。`mode=compromise` 一旦接纳，或 §9.4 通过 inheritedCompromise 继承该事实，旧 key artifact 只有在其 seal ChangeId 可证明因果早于原 compromise activation ChangeId 时才历史有效；并发或更晚旧 key seal 直接拒绝。

`authorize_new_sign(K,currentCut)` 与历史验签分开。在唯一原 P seal 内，它重验当前无 conflict WorkspaceAuthorizationBundle、exact active CommitDomain/fence、K 在 currentCut 是该 domain/profile current key，并要求匹配 usable host-protected private-key handle。plan 可以保存 prepare 时 exact trustRevision/key tuple，但最终 authority 只来自 same-P 检查。rotate/revoke 若先进入 applicable current cut，旧 K 不能新签；author seal 先 commit，则 exact artifact 在后续 ordinary rotate/revoke 后仍是历史事实。因果并发 offline branch 不按 arrival order 伪造成先后：policy conflict 阻止后续新签名，历史验签使用 proved cut；compromise 使用更严格并发拒绝。

private material 只存在受保护 `WorkspaceTrustRootKeyHandle/1` 与 `DomainSealKeyHandle/1`，绑定 Workspace、exact domain/profile/key tuple 与 admitted secure-store opaque handle。Core 在 secure store 内生成 key；合法 import 只走受信 host key-import channel，private bytes 不进入普通 D6 request、log、Workspace、Draft、sync payload 或 transcript。staged|usable|retired|lost 只是运行状态，不授 authority。文件同步/复制只带 public bundle/declarations/artifacts，绝不复制签名权。root key 丢失阻止新的 trust mutation，但不使现有 domain signing 或历史验签失效；domain key 丢失阻止新 managed seal，直到 root-authorized rotate/add 建立可用新 key。旧 private key 在不再有合法 planned signer 后可销毁，public history 继续保留。

实际 trust-management 入口是 closed Control/2 request，不接受 caller key material：
~~~json
{"wireVersion":2,"kind":"d6_domain_seal_key_add_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","expectedTrustRevision":<Counter>,"budget":<BudgetBinding>}
{"wireVersion":2,"kind":"d6_domain_seal_key_rotate_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","expectedTrustRevision":<Counter>,"expectedTrustKeyId":"sha256:64-lowercase-hex","mode":"ordinary|loss_recovery|compromise","budget":<BudgetBinding>}
{"wireVersion":2,"kind":"d6_domain_seal_key_revoke_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","expectedTrustRevision":<Counter>,"expectedTrustKeyId":"sha256:64-lowercase-hex","mode":"administrative|loss|compromise","budget":<BudgetBinding>}
~~~
要求 current workspace-scope policy_admin、exact current trustRevision、当前 state disclosure、已锚定 root 与 usable root private-key handle。add/rotate 由 Core 内部生成新 domain key/PoP；ordinary rotate 还证明旧 key，loss_recovery/compromise 不要求旧 key。每个操作都是 strict managed_atomic portable control change，只更新原 planning/install/P/CP3 链中的既有 policy component/bundle，取得一个普通 ChangeId，不产生 sourceChanges/revision-token artifact，也不建立第二 ledger/CAS/commit point。Replica registration、fresh bootstrap 与 continuation/failover 使用下文专用同记录规则。

内部受保护 key 为 closed `RevisionTokenSealKey/1 = {"changeId":<ChangeId/1>,"entityRef":<EntityRef>}`。既有 decision outbox 对每个真实 managed after 保存一条 closed `RevisionTokenSealOutboxItem/1`：
~~~json
{"kind":"d6_revision_token_seal_outbox","version":1,"key":<RevisionTokenSealKey/1>,"artifactPin":<PinRef/2>}
~~~
该 pin 的受保护记录 payloadKind=portable_metadata，并保留 exact canonical `RevisionTokenSealArtifact/1` bytes。key.changeId 等于 association.changeId，key.entityRef 等于 association.sourceVersion.entityRef。同一 decision 内，这些 item 必须完整且唯一覆盖每个 non-absent managed after，并按完整 EntityRef canonical key 排序。它只是既有 P/outbox 的一个 item，不是第二 ledger、CAS、receipt、Notice component 或 CP3 member。

凡原 plan 可能产生 managed after，Core 必须在 C/Q 或其它绑定 revision 的物化之前只分配一条拟议 revision token，并把完整 binding 随原 plan 保存；Q 两遍、restart 与 recovery 全部复用它。winning planning CAS 把 candidate map、SourceRevisionPlan、afterPin、H/empty-history 依据和该 binding 一起冻结。CAS loser、aborted/terminal-failed plan 或 seal 无法证明的 plan，即使另一 decision 后来 seal 相同数值 H+1 或逐字相同 SourceStamp，也绝不能取得 canonical production binding。

在唯一 P seal 中，每个真实新封存 managed SourceVersion/2 都从 winning plan 选出唯一 canonical RevisionTokenBinding/2，即使当时尚无 Locator 指向该版本。分配本 decision 唯一 ChangeId 并形成真实 managed SourceVersion 后，同一 P transaction 构造 exact RevisionTokenSealAssociation/1，对 winning plan 冻结的 trust key 以同一最终 WorkspaceAuthorizationBundle/Frontier cut 调用 authorize_new_sign，把该受保护 trust cut 随 decision 保存，随后签名 RevisionTokenSealSignedBody/1、耐久保存 canonical RevisionTokenSealArtifact/1 bytes 与 portable_metadata pin，并写入匹配的 RevisionTokenSealOutboxItem/1。签名失败或历史 trust 不可用/歧义时，seal 必须在 commit 前失败。之后换 observer、注册 replica、重建 I、retry 或第一次用 Locator，都不能为该生产版本另 mint、重构或重签 artifact/token。canonicality 来自 winning-plan binding 加本次原始同 P signed artifact，而不是 SourceStamp 字段相等。

publication 在 CP3 之外单独携带原样保留的 RevisionTokenSealArtifact/1 bytes。InstallationNotice/2 与 ContentCompletionProof/3 的 closed shape 保持不变：两者都不新增 binding、association、signature、artifact pin 或 component；不新增 CP4、第二 ledger 或第二 CAS。CP3/ChangeRecord chain 证明 decision/change/version history；历史 portable-trust key 下的 Ed25519 artifact signature 才认证 exact binding bytes，两层证据互不替代。只看 stamp 相等、sender/forwarder 可信、相同 bytes/digest 或 caller 声明都不能认证随机 token。缺失、malformed、非 canonical、不受信或 signature-invalid artifact 按既有 incomplete/unavailable 边界处理，不建立 mapping。同一 exact managed SourceVersion 若出现两份非逐字相等、但都能由原 portable-trust history 验证成功的 RevisionTokenSealArtifact/1，则属于可达 integrity 矛盾；Core 绝不按到达顺序任选。

稳定 managed 地址解析与当前观察资格严格分层。一次新的受权读取先把 revision token 解析到 exact sealed production SourceVersion/2，再在调用方 observerDomain 独立取得真实 current SourceObservation/1。只有 Observation.sourceVersion 逐字相等，且当前 FileObjectBinding、观察世代、pins、授权和 owner dependencies 在请求 cut 全部成立，才取得本次读取资格。observerDomain/current observationEpoch 改变不改 portable token。watcher gap、replacement 或 rematerialization 会使旧 current Observation 及依赖它的所有运行时 selector/preparation 失效，但不会因此改写稳定生产地址；实际历史不同或不可证明时，相同最终 bytes 也不能证明为旧生产版本。新的 Observation 成功只给一次新读取资格，绝不修改或复活旧 SourceVersionRef/sourceToken、selector、ActionEvidence、PreparedActionBinding、Draft/map、PreparedIntent 或 saved plan。

external arm 没有 managed seal，也不使用 managed canonical-binding 发布规则。其 token 只有在该完整 external SourceVersion/2 的原受保护 external-event 证据可证明时才能解析；另一个 observer 只看到相同 bytes 不继承该 event/token，而建立自己的真实 external Observation/version。若原 external-event 证据与接收端 current Observation 独立证明 exact 同一 external SourceVersion，则新读取可在普通授权及坐标/profile 门下使用该地址；否则该 persistent external position 对本次使用为 unavailable/stale。普通受权 external read、repair、Draft 与整源路径继续原合同；需要 managed inner revision 的 structured path 仍先显式 managed admission。

managed SourceStamp/binding 在 seal 前只作为同一原 plan 的 Core 内部证据，用于 stage12 candidate map、D7 DefinitionTransfer 两遍 Q/Locator 物化及其它具名拟议位置验证，不授 current Locator capability。seal 后，canonical token 只通过上述 exact winning-plan binding 加已验证 RevisionTokenSealArtifact/1 才成为稳定生产地址；当前使用仍要独立 current Observation。历史 d6d/d6r/d6a revision-token profiles、DocumentRevision/ResourceRevision/AnnotationRevision 的原 opaque lexical decoder、原 store-incarnation/numeric-revision 解释以及旧 saved bytes 全部保留；不重编码为 d6_source_revision/2，也不在旧 D3 Locator decoder 前增加新的 D6 词法门。D3 Locator member 继续把 revision token 当原有 opaque string，D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence 及 D5 revision-bound locator wire 均不因本节改形。
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

### 3.1.1 ConflictInstallInput/1：仅供冲突解决的物理前像

ConflictInstallInput/1 是仅由 D3 §10.1 冲突解决路径消费的受保护 D6 安装输入。它与 SourceObservation/1 是不同类型；未解决冲突分支不产生成功 current Observation 的普通规则完全保留。此输入没有公共 read/prepare RPC，没有 sourceToken 或 SourceVersionRef，也不提供 Query、D8 编辑、普通 source save 或 raw D3 执行资格。

~~~json
{"kind":"d6_conflict_install_input","version":1,"decisionKey":<DecisionKey/2>,"principalAudienceToken":<Token>,"bindingToken":<Token>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"entityRef":<EntityRef>,"installedHead":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<present FileObjectBinding/1>,"sourcePin":<PinRef/2>,"metadataPin":<PinRef/2>}
~~~

全部成员必需且 closed。只有可信 D3 resolution preparer 能在当前 conflict_read/conflict_resolve、完整潜在 subject/structure/source/body/Field/control 披露及实际写权、domain/P/backend 连续性和完整 expectedKey 相等通过后请求此 D6 producer。未知或不可披露的 conflict/input 地址为 not_visible；当前安装/来源缺失或不能证明按原 owner 映射为 source_unavailable/proof_unavailable，可达但证据矛盾为 integrity_conflict。不返回局部 wrapper 或分支值；已知 changed conflict 仍按既有 prepare 顺序为 conflict_changed。

wrapper 证明真实 installed-before，不能只取历史来源后比较字节相同。decisionKey 给出当前 observer/installation domain；其 Workspace 与 expectedKey、entityRef、原请求一致。installedHead 必须是 expectedKey 中已验证 head；该 head 的完整 sealed 历史证明此 entity 的 sourceVersion 及实际已安装 claim/metadata。sourceVersion.changeId 可为 installedHead 的祖先，但必须证明在该 cut 有效；sourceVersion 必须是完整 managed sealed 生产版本并与该证明逐字相同，生产域及生产 observationEpoch 可以不同。wrapper.observationEpoch 是当前安装世代，等于 fileObjectBinding.observationEpoch。present FileObjectBinding、完整 exact sourcePin 和真实已安装 metadataPin 必须在同一可信 snapshot/write barrier 或无缺口安装历史及最终复验下共同取得。metadataPin 的 payloadKind=portable_metadata，包含由 D3 owner 验证、与该 head 对应所必需的完整已安装 claim/lifecycle/structure 事实；sourcePin 使用实体真实 exact_source_document/resource_bytes/annotation_value decoder。只有 digest 相同而没有安装来源、placeholder、缺 component、unsealed external/third-state bytes、借用另一 head 的 metadata 或 unknown installation 都不能生产 wrapper，没有 absent/external 回退。

既有 d7_preparation bindingToken 与可信 principalAudienceToken 必须等于完整 D3ResolutionInput/1、D7 /3 record 和 D3ResolutionInputUse/1 guard。Core 在任何 prepared 成功前原子保存 wrapper、guard、最小映射、完整 record 及全部引用 pins。每个 wrapper/pin 都保留不可变 resolution-use 关联，绑定本 DecisionKey、bindingToken、audience 及完整原 InputDescriptor。把 wrapper、pins、descriptor text 或 digest 复制到另一 key、普通 descriptor、另一 preparation 或其它 owner 不能取得使用资格；剥离关联必须拒绝，P 连续性丢失不得从文件或历史 hash 重建。相同 key 的不同请求仍先到原 stage5 operation_id_conflict；saved/planned 先恢复原 record，再谈当前 expectedKey/TTL。

InputDescriptor/2.sourceInputs 仍只接 closed 的普通 SourceObservation/1 数组，绝不接受此 wrapper。完整 resolution record 另存全部冲突安装输入；其 sourcePin/metadataPin 进入 native OwnerInputBinding 的 pin set 和原 guard。DependencyProof/2 仍必须包含每个该实体真实 source key、conflict_record key，以及全部实际 lifecycle/placement/inbound/Registry/rule/authorization 正负范围。只有本具名 guarded D3 resolution 上下文能以完整 wrapper 和当前 source/control 连续性建立此 source entry；它证明安装/selected-branch 规划，不证明 canonical-current source，也不是可复用完整 Query cut。不得省略 source entry、冒充普通 Observation 或把此 proof 导出为普通资格。已写 component 后续比原 plan 的 after，未写部分比原 wrapper before；Core 自己安装不要求重签另一 before。

未被 decision 引用且已过期的 preparation 不得建立新决定。一旦 planned/saved/unknown decision 引用 wrapper/guard/mapping/pins，就保留该决定原 last-reference、预算、安装和恢复义务。只有 resolution 真正 seal 后再独立建立普通 current observation，才可能产生公共 current source token。本节仅是安装输入 producer，不是新的 identity/source-resolution 业务 owner，也不是第二 ledger。

### 3.1.2 SourceConflictBefore/1 与 SourceConflictVersionBasis/1：D6 source conflict 安装输入

这两个受保护类型仅由 §9.4 中 D6 所属的 `source_merge` 与 `choose_source_head` arm 生产和消费，刻意与 D3 §10.1 的 `ConflictInstallInput/1` 分离：D3 保持原 guard 与 SourceRevisionPlan/2 decoder，D6 source arm 不取得 D3 resolution 授权。未解决的 `source_concurrent` subject 仍不能产生成功的普通 current `SourceObservation/1`。

`SourceConflictBefore/1` 是完整真实物理 installed-before：
~~~json
{"kind":"d6_source_conflict_before","version":1,"decisionKey":<DecisionKey/2>,"principalAudienceToken":<Token>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"ownerNodeRef":<NodeRef>,"installedHead":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<present FileObjectBinding/1>,"sourcePin":<PinRef/2>,"metadataPin":<PinRef/2>}
~~~
全部成员必需且 closed。§9.4 producer 只有在 closed decode、最低 subject disclosure、当前 `conflict_resolve` 加实际 source/body/Field/control 读写权限、exact `expectedKey`、CommitDomain/P/backend 连续性以及下文“读取前选择 ObservationScope”均通过后才能运行。`expectedKey.kind=source_concurrent`；ownerNodeRef 是准确 entity subject；installedHead 是 expectedKey.heads 中一个已验证成员。该 installedHead 的 sealed history 必须证明此 entity 的 sourceVersion 以及该 cut 上实际安装的 claim/metadata；sourceVersion 是当前物理安装的完整 managed production version，不是后来选择的历史 alternative。observationEpoch 是当前安装世代且等于 fileObjectBinding.observationEpoch。present FileObjectBinding、exact sourcePin 与 metadataPin 必须由同一可信 snapshot/write barrier 或无缺口安装历史加最终复验共同取得。placeholder、absent、external/third-state bytes、缺失物理 metadata、借用另一 head 的 pin、只有 digest 相等无来源，或 unknown installation 都不能生产本类型。

`SourceConflictVersionBasis/1` 是完整 typed branch production basis，必须与 arm 精确匹配：
~~~text
{"kind":"source_merge","baseSourceVersion":<managed SourceVersion/2>,"headSourceVersions":[{"head":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>}...]}
{"kind":"choose_source_head","head":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>}
~~~
source_merge 的 baseSourceVersion 是 baseSourcePin 绑定的 exact common/base production version；headSourceVersions 对 expectedKey.heads 完整覆盖、按 ChangeId 排序唯一，每个完整 sourceVersion 都与对应 head sourcePin 的受保护 record 逐字相同。choose_source_head 的 head 必须等于 resolution.head，sourceVersion 是该 head 的 exact source pin/history 证明的完整 production version。每个 version 都来自真实 sealed branch evidence；相同 bytes、digest、裸 revision、I cache 或 caller 构造的 version 均不能替代。完整受保护 ConflictResolutionInput/2、branch/source/metadata pins 及这两个 typed object 必须在同一个不可变 resolution-use 关联中绑定同一 DecisionKey、principalAudienceToken、expectedKey 与 exact resolution arm。把任一 object/pin 复制到另一 plan、key、audience 或 arm 必须 fail closed。

这两个类型都不进入 `InputDescriptor.sourceInputs`，不产生 sourceToken/SourceVersionRef，也不能用于普通 read/save、Query、D8 或其它 owner。冲突 subject 的 exact `source` DependencyKey 只能在这个具名 D6 resolution guard 内由完整 before+basis 加当前 conflict/source/control continuity 建立；它不是可复用 canonical-current source proof。同一 preparation 若实际读取其它无冲突 source，则这些 source 仍必须具有真实 current SourceObservation/1 并进入 sourceInputs。一旦 planned/saved/unknown decision 引用这些记录，其完整 bytes、pins、关联与恢复状态都保留原 last-reference 义务；retry 不得从 current file 或 I 重建。
### 3.2 OwnerInputBinding/2

~~~json
{"kind":"d6_owner_input_binding","version":2,"protocolOwner":"D3|D6|D7|D8|D9","ownerKind":<controlled-text>,"canonicalDescriptorBytes":<immutable-bytes>,"pinRefs":[<PinRef/2>...]}
~~~
protocolOwner是input descriptor owner，不等于最终decision owner；DecisionKey仍只D3或D6 decision。ownerKind由对应owner version冻结，禁止free callback/JSON；descriptor完整保存，小心大bytes仅typed PinRef slot。完整 P2 D3 候选现已定义 d3_identity_operation/12 及其 native descriptor/companion 消费，仍受 §17 接受/激活门约束；尚缺的 D7/D8/D9 consumer 配套只在 §5 对受影响的 unseen 请求设门，不对 saved/planned 恢复重新设门。

### 3.3 ObservationScope/2

ObservationScope/2 的闭合 profile 分为六类，所有字段都受对应 owner 的闭合类型约束：
- local_source{workspaceRef,commitDomain,ownerNodeRef}：单一源观察；
- owner_fields{workspaceRef,commitDomain,ownerNodeRef,fieldIds}：指定字段观察；
- local_structure{workspaceRef,commitDomain,operation,subjects,parentCandidates}：局部结构观察；
- workspace_constraints{workspaceRef,commitDomain}：工作区约束观察；
- control_only{workspaceRef,commitDomain}：仅控制事实观察；
- prepared_workspace{workspaceRef,commitDomain}：已准备工作区观察。
local_structure.operation 的闭合取值为 create_node|move_node|reorder_node|trash。该 scope 只是观察上界，不是写入权限、实际写集或完整性证明。Policy/3 明确提供 structure_state 与 portable_frontier_state，分别只披露可移植结构状态和完整 Frontier/2。

### 3.4 DependencyProof/2 与 DependencyKey/2

DependencyProof/2 的既有 closed 外壳保持：
~~~json
{"kind":"d6_dependency_proof","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"baseFrontier":<Frontier/2>,"entries":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>},"evidencePins":[<PinRef/2>...]}...]}
~~~
实际 `entries` 可以为空或包含多项；每项 `evidencePins` 可以为空或包含多项。数组长度、唯一性与排序由本节定义，不以 JSON 省略成员表达“未知”。

DependencyKey/2 是以下十四个 kind 的 closed union。每个 key 都必须恰含 `kind`、`workspaceRef` 和该 variant 明列的附加成员；unknown/missing/duplicate member、非法 null、错 union、错 Ref 域或额外 alias 一律 `invalid_request`。`workspaceRef` 必须等于 DependencyProof.workspaceRef 及 commitDomain.workspaceRef。Ref 集使用既有 D3 RefKey canonical 顺序、无重复；FieldId 集使用 D4 规范顺序。作者 sibling order、Entry order、relation fact order 等本来有语义的顺序不是集合，不得为了 DependencyKey 排序而改写。

十四个 kind 的固定 rank 为：source=0、lifecycle=1、placement_range=2、ref_inbound=3、relation_incidence=4、calendar_scope=5、registry=6、temporal_rules=7、authorization=8、foreign_binding=9、query_scan=10、replica_registry=11、conflict_record=12、execution_resource=13。DependencyProof.entries 先按 kind rank，再按整个 key 的 D3-CJ/3 canonical UTF-8 bytes 作无符号 byte-lexicographic 升序，且完整 key 唯一；同样的 key 不允许以两个 stamp 重复出现。`evidencePins` 按 PinRef.pinToken 规范 bytes 排序唯一。外层 Workspace 内的 Ref 和嵌套 scope 原则上必须属于该 Workspace；原 D3/D4 作者值允许携带 foreign Ref 时，只能逐字保留该作者值，不能据此在本 key 下取得另一个 Workspace 的枚举或披露资格。

十四个 exact key shape 为：

~~~text
source:
  {kind:"source",workspaceRef,entityRef:EntityRef}

lifecycle:
  {kind:"lifecycle",workspaceRef,ref:EntityRef}

placement_range:
  {kind:"placement_range",workspaceRef,range:StructureRange}

ref_inbound:
  {kind:"ref_inbound",workspaceRef,target:EntityRef}

relation_incidence:
  {kind:"relation_incidence",workspaceRef,
   fieldId:FieldId,endpointNodeRef:NodeRef}

calendar_scope:
  {kind:"calendar_scope",workspaceRef,range:CalendarRange}

registry:
  {kind:"registry",workspaceRef}

temporal_rules:
  {kind:"temporal_rules",workspaceRef}

authorization:
  {kind:"authorization",workspaceRef,
   principalAudienceToken:Token}

foreign_binding:
  {kind:"foreign_binding",workspaceRef}

query_scan:
  {kind:"query_scan",workspaceRef,
   principalAudienceToken:Token,
   domain:"nodes"|"resources"|"annotations"|"headings",
   selector:QueryScanSelector}

replica_registry:
  {kind:"replica_registry",workspaceRef}

conflict_record:
  {kind:"conflict_record",workspaceRef,
   selector:
     {kind:"id",conflictId:ConflictId}
     OR
     {kind:"subject",subject:ConflictSubject/1}}

execution_resource:
  {kind:"execution_resource",workspaceRef,
   decisionKey:DecisionKey/2,
   protocolOwner:"D3"|"D6"}
~~~

`decisionKey.workspaceRef` 必须等于 execution_resource.workspaceRef。`principalAudienceToken` 必须是受信 host/authentication path 已绑定的 audience token，不能由请求体自报主体；它仍服从 §1.1 Token 词法与受保护 audience 映射，不因为进入 key 而成为 capability。

#### StructureRange

StructureRange 是 placement_range 内部的 closed union，只有以下九个 variant：

~~~text
{kind:"live_children",parentNodeRef:NodeRef}
{kind:"trash_children",parentNodeRef:NodeRef}
{kind:"trash_roots"}
{kind:"ancestor_chain",nodeRef:NodeRef,forest:"live"|"trash"}
{kind:"subtree",root:NodeRef,forest:"live"|"trash"}
{kind:"owner_resources",ownerNodeRef:NodeRef}
{kind:"owner_annotations",ownerNodeRef:NodeRef}
{kind:"reply_closure",annotationRef:AnnotationRef}
{kind:"restore_membership",nodeRef:NodeRef}
~~~

所有 NodeRef/AnnotationRef 必须属于外层 Workspace 并通过 D3 原 Ref decoder；Resource/Annotation owner-local 范围沿用 D3 的 owner 规则。`live_children` 与 `trash_children` 证明对应父项的完整有序 child list；其证明结果保留 D3 sibling 顺序，不按 Ref 排序。`trash_roots` 证明完整 Trash root list。`ancestor_chain` 证明给定 forest 内从主体到根的完整链及无环条件。`subtree` 包含 root 本身并覆盖对应 forest 的完整子树。`owner_resources`、`owner_annotations` 是完整 owner-local 目录，不按当前 UI 筛选。`reply_closure` 与 `restore_membership` 分别按 D3 原 Annotation reply closure 与 Trash restore membership 语义计算。一个操作需要多个范围时必须列多个 placement_range key；不得用自由字符串 `closure` 合并不同语义。

D3 是 lifecycle/placement/ref-inbound 范围算法 owner。本文件冻结 key carrier、证明完整性与 D6 commit/recovery消费边界，不把这些范围算法改写成 D6 identity owner；完整 P2 D3 候选现已提供这些范围的枚举与消费。独立接受/激活仍受 §17 约束，仍缺实际必要 coordinated consumer 的新 strong path 只在 §5 的 unseen 门保持 `owner_update_required`/`proof_unavailable`，但不依赖该范围的合格 ordinary 本地操作不因此永久不可用。

#### CalendarRange、SeriesScope 与 RegistryBinding

CalendarRange 只有以下四个 variant：

~~~text
{kind:"binding",nodeRef:NodeRef}
{kind:"series",seriesScope:SeriesScope}
{kind:"period",seriesScope:SeriesScope,periodKey:<D4 canonical PeriodKey>}
{kind:"scope_inbound",scope:CalendarScope}
~~~

NodeRef 必须属于外层 Workspace。CalendarScope 逐字复用 D4 closed union：

~~~text
{kind:"workspace",workspaceId:<current WorkspaceId>}
OR
{kind:"node",scopeNodeRef:NodeRef}
~~~

node scope 的 scopeNodeRef 必须属于同一 Workspace。SeriesScope 恰含 `series,scope,policyBinding` 三个成员：

~~~text
seriesScope:
  {
    series:{
      calendarId,
      calendarVersion,
      timeZone,
      tzdbVersion,
      periodKind,
      periodRuleId,
      seriesKey
    },
    scope:CalendarScope,
    policyBinding:{
      registryBinding:RegistryBinding/1,
      policyId,
      policyVersion,
      policySchemaDigest
    }
  }

RegistryBinding/1:
  {
    expectedRegistryGeneration,
    expectedSnapshotDigest
  }
~~~

`series` 的七个成员逐字使用 D4 已验证作者 Entry 的同名 canonical semantic value；不得把 title、path、locale 或设备时区补入。periodKind 闭集为 `day|week|month|quarter|year`。seriesKey 使用 D4 原 required exact TypedText 语义，空字符串仍是合法作者值。RegistryBinding/1 必须同时逐字匹配已认证 RegistrySnapshot/1 的 `registryGeneration` 与 `snapshotDigest`；generation 或 digest 任一项都不能单独替代完整 binding。policyId、policyVersion、policySchemaDigest 必须命中同一 RegistryBinding 下经验证的 CalendarSeriesScopePolicy/1 contribution。

`periodKey` 只能由已验证 calendar period-rule contribution 的 keyProfile 解码成唯一 canonical semantic key；ISO v1 lexical profile仍为 day=`YYYY-MM-DD`、week=`YYYY-Www`、month=`YYYY-MM`、quarter=`YYYY-Qq`、year=`YYYY`，并继续执行 D4 的真实 Gregorian/ISO week/bounds 检查，不能以 regex-only 字符串代替。

`binding` 证明一个 period Node 的 CalendarPeriodScopeBinding、其独立 binding revision、对应真实 source period/series 解释和当前配置；没有有效 period Entry 时不能伪造 active binding。`series` 证明该 series+scope 下全部 periodKey membership、SeriesScopeConfiguration、完整负范围及适用 control inbound。`period` 证明一个完整 `{series,periodKey,scope}` 范围的全部成员；unique/many 只来自受管配置，不能由当前命中数或一次 create 自行选择。`scope_inbound` 证明全部引用该 scope 的 period bindings/configuration/control inbound。空范围同样需要独立范围 stamp。Calendar语义归 D4，配置/绑定持久控制与stamp承载归 D6；D4/P2相关实际枚举 consumer 未完成前不得据此宣称 strong Calendar success 已激活。

#### QueryScanSelector

QueryScanSelector 只有：

~~~text
{kind:"workspace"}
{kind:"entities",refs:[EntityRef]}
{kind:"subtree",root:NodeRef,includeRoot:Boolean}
~~~

`entities.refs` 非空、按 D3 RefKey 排序唯一，并且必须与 domain 匹配：nodes 只允许 NodeRef，resources 只允许 ResourceRef，annotations 只允许 AnnotationRef，headings 只允许作为 heading source owner 的 NodeRef。`subtree` 只允许 domain=nodes 或 headings；root 是同 Workspace NodeRef。对 headings，subtree选择的是这些 source Nodes 中的 headings，不创建 HeadingRef identity。includeRoot 为真正 JSON Boolean。

refs 来自 D7 原 Query selector 的已验证求值结果，而不是客户端用此 key 发明另一套选择器。QueryScanSelector 不允许 CEL/任意 predicate、ResultHandle、row handle、cursor、page position、sort/take 或自由 payload。完整 Query 表达式、SavedQueryDefinition、pre/postQuery、参数、排序及原计划绑定仍归 D7。headings 的完整扫描还必须读取每个适用 Node 的准确 D2 source；physical/D2 invalid、placeholder、source unavailable 或无法证明覆盖时不能把该 Node 静默跳过后宣称 complete。D7 afterimage 未完成前，依赖新版 query_scan 作为 strong complete proof 的路径保持 owner gate；本 union 的存在本身不是 D7 接受。

#### 十四类的 owner、完整性与披露规则

- `source`：D6 拥有观察/版本/文件绑定证明，内容语义仍归对应 Node/Resource/Annotation owner。除两个明确 guarded conflict-resolution 上下文外，正例必须绑定完整 current SourceObservation/1、准确 bytes/value pin、FileObjectBinding、当前 source validity 与本操作实际用途；SourceObservation.entityRef 必须等于 key.entityRef。D3 §10.1 只能在 D3 resolution guard 下由 ConflictInstallInput/1 建立其 exact source entry；D6 §9.4 source_merge/choose_source_head 只能在 d6_conflict_resolution/2 下由 SourceConflictBefore/1 + SourceConflictVersionBasis/1 建立冲突 subject 的 exact source entry。两种例外都不产生 canonical current Observation 或可复用 source proof。若证明 source absent，必须从真实 identity/lifecycle/FileBinding 与受管 absent object 得到，不能从 placeholder、I miss、I/O失败或“文件没下载”推断。窄 Field 路径可以让 Core 在受信边界内完整读取并原样保留 source，但没有 source_read 时不得向主体输出正文。
- `lifecycle`：D3 owner。证明完整 birth/canonical claim、live/Trash/tombstoned/never-known 状态及适用 owner 关系，且其变化与 source revision 分域。必须先通过 D3 原 state-disclosure gate，再读取存在/lifecycle；absence 依赖完整 identity directory，不从 derived index miss推断。
- `placement_range`：D3 owner。按 StructureRange 完整枚举 sibling list、祖先链、子树、owner-local目录、reply closure 或 restore membership，证明边界、顺序、无环和缺项。structure_state/原D3结构披露先于读取隐藏 sibling；不得扫描后再选择“刚好安全”的范围。
- `ref_inbound`：D3 identity/lifecycle 引用闭包 owner，具体 foreign/control slot仍归各原 owner。对 key.target 完整枚举 D3 已冻结的引用槽位、lifecycle/control inbound 与 live/Trash 适用矩阵；零入站必须来自完整目录/stamp。D7 SavedQueryDefinition 或其它未来定义不会因此被偷塞进 D3 node_link/citation union。
- `relation_incidence`：D4 owner。精确复用 RelationReadContext/2 incidence scope：fieldId、endpointNodeRef、revisionToken、完整 factSelectors，factSelector仍为 `{ownerNodeRef,fieldId,occurrenceKey}`。证明 canonical owner、source/entity state、incidence cover 与完整正/负集合；literal arm没有Node incidence，symmetric事实不重复。masked/unprovable不能形成完整成功proof。作者值中合法 foreign NodeRef 的逐字保留不授本Workspace key跨Workspace枚举资格；需要foreign current state的strong语义仍须相应owner合同。
- `calendar_scope`：D4语义+D6配置持久 owner。按 CalendarRange 证明 period scope binding、SeriesScopeConfiguration、RegistryBinding/policy、完整period membership/负范围以及适用control inbound。配置 absent、binding absent和empty period range均有独立版本，不能由index empty或当前页推断。unique继续对完整 `{series,periodKey,scope}` 分组验证。
- `registry`：D4 是语义 owner，provider/trust 真实性仍由实际提供方负责。首代 key 保守绑定同一 Workspace 的完整 RegistrySnapshot/1、RegistryBinding/1、必要 RegistryEvolutionProof，以及全部 Field/Facet/alias/namespace/contribution 目录。目录完整不表示其中每项定义都可用；不可用或未知的定义与已经证明完整且为空的定义集合必须区分。一个操作确实不依赖 Registry 时，不强制添加此 key。
- `temporal_rules`：D4语义、D6保存binding证据。首代key保守绑定Workspace已接纳的temporal-rule目录，并在实际proof中列出本操作读到的calendar comparator、period-rule、timezone/tzdb rule set、RecurrenceReadBinding/1以及所需有限 horizon coverage。无需复制无限时间域；segments缺口、规则来源不可证明和“业务值unsupported”分别处理，设备时区不能补洞。
- `authorization`：D6 owner。principalAudienceToken来自受信 principal/session/delegation映射。proof绑定当前 Policy/3完整版本/授权generation、delegation、原 ObservationScope/2 和适用metadata capability；任何可能改变该主体对本操作结果/输入披露或写资格的授权变化必须使stamp变化。公开 DependencyProof 不输出hidden grant、deny成员或作者值。deny优先、default deny、write不蕴含read以及source_envelope_state/commit_sequence_state/structure_state/portable_frontier_state各自的披露边界保持。
- `foreign_binding`：D3拥有binding语义和identity约束，D6保存受管目录/连续性，D9/D10或具体来源owner拥有外部版本格式/比较器。首代key保守覆盖本Workspace相关 SourceBinding/OriginBinding 完整目录、active/retired历史、mapping/version及原比较器。相同UID、etag、path、digest或行文本不能代替binding。实际记录只按已知原版本decoder读取；未知profile或缺实际来源decoder时，依赖它的strong path为 `proof_unavailable`/owner gate，不引入free JSON wrapper，也不取消不需要该strong proof的普通文件读写。
- `query_scan`：D7 owner，D6承载key/stamp。proof证明principalAudienceToken对应主体在指定domain/selector下的完整可见枚举、隐藏策略generation及所有需要读取的真实source/control依赖。headings另按上文要求验证D2 source。保存定义继续绑定原SavedQueryDefinition source/地址解析依赖；pre/postQuery连接绑定原不可变Prepared计划。拟议fresh对象没有sealed source/version时不得伪造“已提交query_scan stamp”。
- `replica_registry`：D6 owner。证明完整active/retired ReplicaRecord目录、registrationSequence及无缺口连续版本。purge还要逐个验证参与副本对指定purge Frontier的真实已接纳证明或显式retirement；注册数量、provider“已同步”、某一个副本Frontier或index row都不能代替ack。空目录若协议阶段允许，也必须经完整目录证明。
- `conflict_record`：D6拥有记录/目录，resolution语义仍归D3/D4/D6原领域owner。selector=id只证明该ConflictId的exact record及其版本decoder；selector=subject证明覆盖该subject的完整conflict目录，包括open/resolution_prepared/resolved/superseded关系和相关真实heads。先 conflict_read capability与subject最低state disclosure；无权/unknown遵循原not_visible。无sealed head的external competition、third_state或unknown install不得捏造ChangeId填入ConflictKey。
- `execution_resource`：D6 owner。key绑定一个完整DecisionKey/2和其exact protocolOwner，读取该原operation的resource policy revision、attempt allowance/已耗attempt、累计work、protected pins/capacity与pause category，并沿用existing execution-resource control read的当前受管授权。decisionKey只提供明确CommitDomain定位；不能列举全部OperationId。absence、P丢失或找不到旧摘要都不是额度重置、退款或重新签发资格。此key不包含D10 Run/Lease/Automation/Workspace/deployment Money谱系、provider unknown或sourceOccurrenceKey；这些仍由U6/D10连续责任合同拥有。

#### stamp 生成、连续性与错误映射

`stamp` 的 shape逐字保持 `{"epoch":<Token>,"revision":<Counter>}`；不新增 epochToken/revisionCounter 别名。epoch是**该DependencyKey范围证明的连续性世代**，不是SourceVersion生产observationEpoch、SourceObservation.observationEpoch、CommitDomain generation或Workspace全局代数。受保护stamp记录必须反向绑定完整DependencyKey/2；客户端不能自行签发。

一个新epoch在完成一次真实完整枚举后可以从 revision=0 开始。这里的0只是“该新范围世代的首个完整proof revision”，不是source revision0、未知、empty或“尚未扫描”。在连续事件链可证明的同一epoch内，任何可能影响该key的source、identity、范围成员、顺序、控制、授权或可见性变化，都必须先使旧stamp不可再作为current proof，再checked增加revision并完成所需当前验证后发布新stamp。MAX不wrap。

watcher/journal gap、owner规则/decoder版本变化、无法恢复的事件缺口、正确性证据目录真正丢失、或无法证明原range连续性时必须换新epoch；不能因最终内容/hash/成员数相同而认回旧epoch。删除或重建Derived Index本身**不**删除仍完整存在于Durable Control Store/Portable Workspace Metadata真实owner中的正确性控制事实：这些事实及连续事件链仍完整时，I重建只恢复候选cache，不重签proof、不换epoch。若真实proof/continuity事实已经丢失，则旧proof永久失效；只能在当前授权下重新完整枚举并建立新epoch，不能从I或重复hash复活。

空范围和非空范围使用同一完整性标准：先有本key所需的state/content/metadata授权，再遍历所有适用目录、分片、owner范围和source，并证明没有遗漏。placeholder、unknown decoder、I/O失败、缺失shard、隐藏但未授权范围、partial index或“没有候选命中”都不能产生empty success。完整枚举必须依赖一个受信一致snapshot/range read barrier，或完整无缺口连续变更记录加最终复验；重复扫描得到相同hash不等于一致cut。若底层只能提供局部证据，complete proof就是不可用，但只依赖该局部证据的ordinary操作可以继续按自己的资格工作。

一个planned decision仍引用的最后一份evidence pin/range proof必须按P/portable owner及PinRef retention耐久保留。容量不足在建立新proof/plan时返回既有budget/proof失败或令planned进入paused_capacity；不能释放last-reference evidence后从I猜回。proof不是当前正文镜像，只保存闭合key、stamp、必要pins及受保护枚举/连续性依据。

对外只保留现有错误/状态面：
- 当前披露/授权失败：`not_visible`，且在读取隐藏范围成员前停止；
- 已获权但source bytes当前不可读：适用`source_unavailable`；
- 已获权但完整range/decoder/continuity proof不能建立：`proof_unavailable`；CommitDomain/P continuity本身不可证明仍用`domain_unavailable`；
- 对unseen canonical intent已经有完整当前proof，并确定原绑定dependency发生不允许的变化：按§5第6步的现有`dependency_conflict`处理；
- decision已经planned后，不把撤权、未知continuity或安装竞争另写成新的recorded rejection。已证明依赖竞争进入原plan的conflict/paused恢复分支；连续性、安装归属或结果不确定保持paused/recovery_unknown。具体提交与恢复顺序见§5/§8；不得借业务终态掩盖不确定性。
错误不得增加free details、hidden count、partial member list或内部cause。

### 3.5 InputDescriptor/2

closed shape保持：
~~~json
{"kind":"d6_input_descriptor","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"intentKind":<controlled-text>,"saveProfile":"ordinary|complete|control_only","guarantee":"replica_local|managed_atomic","expectedFrontier":<Frontier/2>,"frontierPolicy":"exact|scope_dependencies","observationScope":<ObservationScope/2>,"sourceInputs":[{"entityRef":<EntityRef>,"observation":<SourceObservation/1>,"role":"before|dependency"}...],"controlInputs":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>}}...],"ownerInput":<OwnerInputBinding/2>}
~~~
实际 sourceInputs/controlInputs 可以为空或包含多项；member set、版本号与union不变。

workspaceRef/commitDomain必须逐字等于外层plan对应成员；DependencyProof/2.workspaceRef/commitDomain必须与之相等，DependencyProof.baseFrontier必须逐字等于 expectedFrontier。sourceInputs按完整item的D3-CJ/3 canonical bytes排序唯一；每项entityRef=observation.entityRef，observation.observerDomain=commitDomain。sourceInputs携真实current Observation，不以SourceVersionRef、hash或I状态替代。D3 §10.1 或 D6 §9.4 的冲突 subject 明确不进入 sourceInputs，只能由各自具名受保护 owner input/plan 携带；该例外绝不允许伪造 Observation。同一 preparation 实际读取的其它无冲突 source 仍是普通 sourceInputs item，必须带各自真实 current Observation。

controlInputs同样按其DependencyKey/2的固定kind rank+D3-CJ/3 key bytes排序唯一。每个controlInputs项必须在同一DependencyProof.entries中有且只有一个byte-equal key，并且stamp逐字相等；DependencyProof中用于本intent的control dependency不得通过另一个不同stamp旁路InputDescriptor。source类DependencyKey若代表sourceInputs中的currentness证明，必须指向同entityRef并与该Observation的fileObject/pins/currentcut一致；不能把另一生产域相同revision数字当作匹配。

ObservationScope/2仍只是先验授权观察上界，不是dependency completeness或write permission。所有实际DependencyKey读取必须落在该scope及当前Policy允许范围内；若D3/D4/D7 owner算法尚未提供本节要求的完整范围，就保持owner_update_required/proof_unavailable，不能让union名字本身充当完成证明。

expectedFrontier保持原prepare输入，不在commit时重采样。frontierPolicy=`exact`继续要求完整Frontier equality；`scope_dependencies`只允许§1.4/§6.3所述可证明无关非回退扩展，并重新验证本descriptor绑定的全部source/control/auth/正负依赖。无关coarse Frontier增长不会自动使局部ordinary intent失败，真实绑定依赖变化也不会因scope_dependencies被忽略。

完整input equality比较整个InputDescriptor canonical bytes、OwnerInputBinding完整descriptor、所有exact PinRef受保护记录、SourceObservation、DependencyProof key/stamp/pins以及owner-input要求的其它固定输入；相同hash、相同最终文本、相同revision数字或重建I均不足。修改source/current Observation、任何key/stamp、mapping、Policy/Registry/rule、scope、frontierPolicy、WriteProtection、owner request或版本依据必须形成新的prepare；同OperationId仍只允许原canonical replay。

### 3.6 PreparedIntent/2

PreparedIntent/2 的不可变成员集合保持不变：
身份/上下文为 kind、version、planToken、operationId、workspaceRef、commitDomain、principalAudienceToken、inputDescriptor、beforeCut；
拟议状态/证明为 proposedState、mutationFootprint、dependencyProof、observationProof、budgetBinding、pinDirectory；
安装/交付为 installationPlan、inputRetentionState、expiresAt、previewBinding。

kind仍为d6_prepared_intent、version=2，planToken tag仍为d6_plan/2。principalAudienceToken必须等于本plan受信认证主体的audience绑定；同plan中的authorization key及query_scan key若存在，其principalAudienceToken必须逐字等于该值。dependencyProof.workspaceRef/commitDomain/baseFrontier分别等于PreparedIntent workspaceRef/commitDomain与inputDescriptor.expectedFrontier；inputDescriptor与dependencyProof的key/stamp关系按§3.5逐项成立。observationProof必须覆盖sourceInputs的当前观察资格并与同一current cut的fileObject/evidence pins、适用control/Registry/incidence依赖相容；它不替代DependencyProof。

pinDirectory必须覆盖OwnerInputBinding.pinRefs、DependencyProof.evidencePins、SourceObservation.evidencePins，以及installationPlan实际引用的proposal/read-before/after/recovery pins；同一pin不得以不同受保护记录重绑定。last-reference pin继续服从原retention/capacity规则。

inputRetentionState仍为not_retained|retained|unavailable。只有proposal、实际read-before和全部必要bindings/pins已经耐久保存时才能retained；prepared/retained都不表示Saved或portable published。

installationPlan仍冻结write set、after bytes、owner versions、recovery、required WriteProtection与portable records，不预分配ChangeId且commit不重新采样。前一Control core组新增的SourceRevisionPlan/1与d6_source_revision/2 binding只作为该private installationPlan/受保护P状态的版本依据：不得提前seal SourceVersion、不得向preview公开fresh D3 Ref、不得在D3 stage12 candidate map之外重采样identity，也不得在plan赢得后改revision。删除/source-unchanged/no-op分支仍按其明确规则不造SourceRevisionPlan。previewBinding仍只引用owner preview。

D3/D4具体range算法、D7 query_scan/SavedQuery/pre-post binding及D8/D9/D10后续consumer没有因本节文本出现而自动实现或接受。所有需要这些尚未配套consumer的strong success继续gate；合格ordinary `.adoc`读取、人工普通保存与只需要真实局部证据的离线操作不因无关complete proof缺失而永久不可用。
## 4. D6 v2 准备与提交请求

### 4.1 existing Document source save prepare

~~~json
{"wireVersion":2,"kind":"d6_source_save_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"ownerNodeRef":<NodeRef>,"expectedSourceToken":<Token>,"saveProfile":"ordinary|complete","guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","source":<text>,"budget":<BudgetBinding>}
~~~
ownerNodeRef属于Workspace；expectedSourceToken tag=d6_source_observation/1且当前授权下选择完整SourceObservation/1。source立即exact pin，commit不携带source。physical-invalid external仍走repair。

ordinary要求D2 valid和实际修改local typed facts；合法未证明complete obligations→semantic_pending。complete要求全部适用D4/D5/D7义务且不降级。

observed_only 的闭合资格同时要求：受信调用类别为 interactive_source_save；目标是一个 existing live Document；保存类型为 ordinary+replica_local；请求主体具有完整 source read/replace 权限且没有适用的 body/Field/node-control deny；author source write set 为空或仅包含该 Document；不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity；Draft Base 必须等于 selected SourceObservation。
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

v2 ledger key恰为 (workspaceId, D3-CJ/3(commitDomain), operationId)；record另保存 protocolOwner=D6。相同 key 不同 canonical request固定 operation_id_conflict。完整 P2 D3 wire12 候选现已定义 native descriptor/companion 消费，并共享同 DecisionKey owner 互斥；候选存在不授予 §17 接受/激活资格。仍缺实际必要 coordinated consumer 的受影响 unseen protocolOwner=D3 请求返回 owner_update_required，零新增 D3 decision；已有 saved/planned/unknown decision 继续按下文原记录分流。

### 4.3 D10 工作区控制生产者

公开管理准备 D10ControlPrepare/1 归 D10，其工作区分支调用本闭合 Core 适配器，不另设公开 D6 提交。OwnerInputBinding/2 的 protocolOwner 为 D6，ownerKind 与 InputDescriptor.intentKind 都为 d10_control/1，完整规范描述符如下：

    D10ControlInput/1 = {
      kind:"d10_control", version:1,
      key:D10.StableControlKey/1,
      operationId:Uuid,
      canonicalIntentBytes:Bytes,
      allocatedControlRefs:[D10.ControlRef<K>/1],
      preview:D10.ControlPreview/1,
      dependencies:D10.ControlDependencies/2,
      effectPlan:D10.D10ControlEffectPlan/1,
      confirmationRequirement:D10.ExternalConfirmationRequirement/1
    }

这些字节由 Core 内部生成，按真实 D10 Control Contract §7–8 严格解码，不是调用方断言。稳定键的范围必须是本工作区，发起主体必须等于受信当前主体。首次获权的同键准备原子预留从未使用的 operationId 和控制引用，固定描述符与原准备关联，返回一个原 d6_commit_request/2。准确重试恢复它；规范意图改变则冲突；既往存在性不确定时不创建任何新内容。全部身份和分配历史按原稳定键规则保留。D10 控制引用不成为 EntityRef。

描述符固定完整成功解码 body、实际完整控制前像和拟议后像、分型 artifact 固定引用、原不可变预览、真实控制范围与依赖，以及固定外部确认要求。计划 consent 中嵌套的原作者请求 A 是合法不可变输入。本次生成的控制请求 M、其 planToken、请求摘要及未来人工确认事实都不进入描述符。D6 生成 M 后，Core 在返回前原子保存外部 ControlPrepareBinding/2 到 M 的准确关联。交付前崩溃恢复同一关联，不重新生成请求。DependencyProof、sourceInputs 和 observationScope 等于 dependencies.workspaceReads.some，对应真实依赖键及 controlInputs 准确匹配。最终 observationProof 只在外层 PreparedIntent/2 中由这些原输入和引用构造一次，不嵌回 ownerInput。必要 artifact 引用进入同一 pinDirectory 和保留预留。

D10 配置、用量、停止完整值及完整范围栅栏都属于此 InputDescriptor 内的领域固定输入，在真实 P 事务的 planning 与 seal 比较。这符合 §3.5 依赖不能绕过描述符的规则，不增加第十五类 DependencyKey，也不挪用 execution_resource。最低披露资格及当前 D10 的 S/W 管理资格先于保护读取。纯 P 工作区控制使用 saveProfile=control_only，只产生控制效果，没有源版本或内容 ChangeId；所有 D10 分支都保持 guarantee=managed_atomic 和严格保护。观察范围另由声明 body 及原请求保护映射在作者值读取前选定。不观察作者内容的 body 使用 control_only；单字段适配器或长期规则配置若全部潜在作者范围只有一个静态合格 owner/Field，则使用 owner_fields 和原完整 D7 窄资格，保留窄主体自助管理的正向路径。recurrence 配置和全部激活操作使用 workspace_constraints；consent 从保护映射取得原请求或效果的完整披露范围。其它确需更广或多个 owner 的作者范围，须在读取前取得全域资格。真实源、Registry 及范围读取仍用各自实际依赖键和当前授权；不能先读隐藏值再扩大配置，也不能失败后降级，控制配置不授予隐藏作者读取权。

激活的真实 Registry 效果明确分支：后继 Registry 改变可移植当前元数据时，所选入口使用 complete 与 workspace_constraints，沿既有严格安装、封存及发布路径执行。D10 分型 registryChange 保存完整原值与拟议值、准确引用和 D4 演进证明；Core 验证并只安装一次该 Registry 分量，产生唯一可移植 ChangeId 与 CP3，但不产生源修订或 H。Registry 字节和状态未变、只改变 P 中目录或选择器时，仍为纯控制效果。分支由 planning 前固定的真实效果决定，不能失败后降级。目录、选择器及 D10 控制后像在同一 P seal 前均不发布，可移植安装不能暴露半激活目录。本分支不修改作者源，也不建立第二 Registry。

工作区适配器只接受 D10 准确种类与范围授权下的自动化配置、工作区 consent、适用工作区 state、工作区限额及激活。部署控制、对账、凭据暂存、issuer 管理和外部发送保持各自已有闭合领域，不进入本适配器。Registry 分支不能绕过 registry_admin、policy_admin 或真实完整业务门。新工作区引导仍由 §10.2 的原 D3 路径生产。

### 4.4 原两个 CAS 点上的 D10 当前资格

无人值守作者执行要求实际 D10 Control Contract §8.2 的 ApprovalUse/1、准确 LeaseRunUse/1、原 D7 PreparedActionBinding/3，以及当前严格完整资格。保护关联在原请求可提交或交给执行者前建立，删除调用方 token 不能删除它；它不是新增公开请求字段或可编辑 ownerInput。D10 工作区外部 consent 同样使用固定要求和独立保护 ExternalConfirmationRecord/1，只有已接纳受信人工事件能建立匹配事实。直接提交已经返回的 D6 请求仍定位并验证这些原关联。

§5 第4步中，保存决议始终先重放原结果，再考虑新的批准、时间、停止或计数门。planned 恢复准确原请求、计划、安装状态和预留。只有当前 D6 披露授权、领域连续性及原适用业务、依赖、预算门通过后，未见请求的 planning 分支才检查当前 D10 资格。在与 planning 相同的真实存储序列化边界内，Core 核对准确运行、租约、激活、可信时间、原批准与效果匹配、停止锁及适用外部确认，CAS 实际当前资格修订和计数。批准的 unreserved→reserved 与 planned 原子保存。租约 maxRuns 只在更早的原运行准入 CAS 消费，作者 planning 及后续恢复不再消费。合法后续确认或补充计划批准关联到同一原请求时，描述符不变。

第11步按原后像核验已写分量，按原前像核验尚未写依赖。第12步在唯一最终 P 写序列化边界内重验当前 D6 授权与原业务证明，再重验这些同一实际 D10 门；固定控制效果、决议回执、审计和原批准预留消费在一个事务发布，逐字无操作亦然。它读取准确当前匹配的确认记录与修订，不要求准备时的 none 状态永远不变。不能用旧计算后像覆盖无关用量变化；冻结控制写的真实依赖变化需重新准备，而运行期计次预留在自己的 CAS 下比较当前合格总量。保存结果重放不重复消费。

若普通权限和业务门均通过，只有关联的批准、租约时间资格、补充 consent 或必需受信确认不可用，则返回 approval_unavailable/preflight。未见请求不创建 P 决议或预留；planned 保持原状态、全部引用、真实安装状态及预留，不成为记录拒绝或终态失败。权威或后端连续性不可证明时保持更早的 D6 错误优先级。控制细节只能另经获权 D10 读取说明。不可逆停止在新计划前使用 execution_stopped/preflight；既有计划按下述恢复规则处理。两个新错误只属于显式当前 wire2 闭集，不重解释旧错误字节。

停止与最终 P seal 在同一真实权威存储中序列化。封存先完成则固定原提交结果；停止先完成则阻止封存，包括第10步已经物理安装文件的情况。此时保留受管屏障、真实 B/N 引用、原通知、安装来源证明及控制、计次、费用责任，不能声称字节从未改变。获权恢复只能撤销已经证明属于该原计划的安装，不能覆盖后续或第三状态。只有不可逆停止使原提交不再可能，且 §8 下每个安装残余均安全解决，才可保存原 transaction_aborted/terminal，并原子令计次 reserved→released_terminal。来源未知、第三状态、临时撤权或到期或取消、回滚不可证，都保留 planned 或暂停恢复及全部责任。费用仍须自身原结算证据。停止不阻断获权中止、对账、审计或引用安全清理，这些动作也不恢复执行权限。

## 5. 提交、安装、seal、发布的唯一顺序

下面顺序同时区分 saved、planned、unseen。prepared/retained 只表示提案、read-before 与必要 bindings/pins 已耐久保存，不是 Saved；任何 branch 都不能通过重新准备第二个 decision 来替代同一 DecisionKey 的既有记录。

1. closed decode 与静态 cross-field equality。失败为现有 invalid_request/preflight，零 author/business-state 读取。
2. 依据当前 authenticated principal 做本入口最小 state disclosure、原 ObservationScope/2 与 CommitDomain 的先验资格；未通过统一 not_visible。不得先读取隐藏作者事实、结果成员或旧 ledger 业务内容再决定 scope。
3. 按实际入口及原版本合同验证 CommitDomain、适用的受保护域栅栏、可移植信任与后端资格，以及 P 账本的保管和连续性。D6 的 d6_commit_request/2 校验其已声明的 expectedDomainFenceToken；原生 D3 按自身域、authority 与 custody 合同取得资格，不携带这个成员。D3 的 create/fork 按其 owner §4.5.1 执行：issuer A 暂管新 Workspace W/目标 authority B，但 DecisionKey 和 commitDomain 从首个请求直到 seal、重放和 custody 移交始终绑定 W/B。按 stage4 -> P1 -> P2 -> TL1 -> TL2 -> stage5 顺序，先证明 issuer/source 资格，再验证 proposal，最后验证 target custody；P1/P2 失败不读取 target ledger。不要求新目标已经 active 或已有 target source_write、policy_admin、current policy；A 不能用自身 domain 或 Frontier 代替 W/B。目标空历史由 issuer 按原 bootstrap profile 证明。旧资格若仍可证明连续，只允许查找原决议，不授权 unseen 新决议。无法证明时使用 domain_unavailable 或原 D3 对应的可用性错误；只有在可达状态下证实损坏，才使用 integrity_conflict 或原 D3 对应的完整性错误。共同连续性门通过后才能读取 DecisionKey 的业务记录。
4. 以 `(workspaceId,D3-CJ/3(commitDomain),operationId)` 定位 DecisionKey，并比较 protocolOwner 与原 canonical request/fingerprint。不同 owner 或同 key 不同 request 固定 operation_id_conflict；完整原 request 用于防止 ledger probe。随后按已存在状态分流：
   - saved decision（包括 committed 以及按原协议已耐久保存的 recorded/terminal outcome）：只按**原实际 effect/mode 或原结果披露范围的当前披露与交付授权**返回原封存 receipt/error bytes，或进入其原 publication/outbox 恢复。不得要求原 before SourceObservation 仍 current、旧 Frontier 等于当前 Frontier、旧 preview/plan TTL 未过期、当前 r6 业务依赖重新成立，也不得重新执行语义选择。当前撤权可以遮蔽交付，但不能修改 decision、receipt bytes、原 ChangeId、费用或历史恢复责任。
   - planned：恢复同一原 InputDescriptor、适用 SourceRevisionPlan、before/after pins、write set、OperationId、budget/attempt counters、WriteProtection、InstallationNotice 与 installation state。不得 new-prepare 第二个成功、重选 target/Query/current page、重采样 identity/H/revision 或修改原请求。实际当前授权、原 dependency continuity 与安装归属只决定原 plan 是否可以继续，或保持 paused/conflict/recovery_unknown；尚未 seal 的 planned 没有本 decision 的 ChangeId。
   - unseen：仅这一分支继续步骤5并以当前 producer/consumer合同建立新的 decision。
5. unseen 按实际闭合入口验证当前域栅栏及该入口要求的准备材料：
   - D6 的 d6_commit_request/2 校验所携 planToken 的类型标记与受众、选定的 PreparedIntent/2、inputRetentionState、owner 版本，以及固定的 InputDescriptor、DependencyProof、ObservationProof 和 pins。
   - 原生 D3 的 identity_operation_request wire12 按 D3 阶段校验其完整 InputDescriptor/2、d3_identity_operation/12 所属者描述、受保护输入与 pins，以及原生私有计划。它不携带也不要求 planToken、D6 prepare 调用、PreparedIntent/2 或 expectedDomainFenceToken；添加未声明成员即为闭合解码失败。获准 managed_atomic 请求中的 D7 preparationBinding 按实际版本的 D3/D7 合同检查。D3 §10.1 产生的 resolution 输入必须有其不可变 preparationBinding 和 D3ResolutionInputUse/1 用途保护；删去 token 不能把这种输入变成合格 raw 请求。最小映射和用途披露检查先于 branch 读取，既有 key 的 fingerprint 比较先于新业务验证，只有 unseen 才检查新的选择、head、期限与 producer 门禁。无关原生输入仅在原 mode 允许时省略 binding；省略不授予 D7 准备资格。
   各入口保留实际 owner 与错误检查顺序，并共用唯一 DecisionKey/P 的 planning CAS；这项分流不创建第二个 D6 身份提交入口。缺少配套 owner 消费合同的强路径在此返回 owner_update_required/proof_unavailable；不依赖该强范围的合格普通文件路径不因此永久禁用。
6. unseen 在进入 planning 之前，按 frontierPolicy 重验 Frontier/2、全部普通完整 current SourceObservation/1、适用时仅 D3 使用的 §3.1.1 ConflictInstallInput/1，或 §9.4 D6 source arm 的 SourceConflictBefore/1 + SourceConflictVersionBasis/1，并同时重验按显式 decoder 读取的 SourceRevisionPlan 版本依据、DependencyProof/2、MutationFootprint authorization、本次适用 local/complete semantic gates、预算和全部未写依赖。exact 使用 §1.4 的完整 equality；scope_dependencies 只接受 §1.4/§6.3 可证明无关的连续非回退扩展。semantic_pending 只能表达已经通过本地 typed facts、但仍欠允许 pending 的跨对象/全集义务；它不能授权 all_result、bulk、strong Action、Automation 或其它要求完整证明的成功。
7. 对人工 ordinary whole-source save，observed_only 必须由受信 `interactive_source_save` 的人工选择在**planning 开始前**显式完成并冻结。资格仍全部要求：恰一个 existing live Document、ordinary+replica_local、完整 source read/replace、author source write set 为空或仅该 Document、无适用 body/Field/node-control deny、不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity，并且 Draft Base 等于选定 current SourceObservation。noninteractive、D3 identity/parent/order/lifecycle、D5 structured cell/row/column/reorder、bulk/collection/promotion、D7 strong Action、Automation、server checkpoint、Approval、Money 一律 strict。planning 开始后 strict capability失败、known conflict、失权、durability failure、strong obligation失败或其它资格缺失都不得 fallback 为 observed_only。
8. planning CAS 原子保存 canonical request、fixed plan、InputDescriptor、DependencyProof、适用 SourceRevisionPlan、write set、exact before/after pins、budget/reservation、recovery description、required WriteProtection、版本依据及 planned。winning plan 后不重新采样。这里只固定拟议 SourceStamp；**不分配 ChangeId，不把 SourceStamp/RevisionToken 当成 sealed SourceVersion，也不公开 fresh D3 Ref**。
9. 修改任何 portable current component 前，先耐久写 InstallationNotice/2。它保存原 DecisionKey、guarantee、WriteProtection、notice.baseFrontier 与完整 component before/after。baseFrontier 可以含此前已封存的历史 ChangeId；notice 本身没有这个尚未 seal decision 的新 ChangeId，也没有 receipt、Approval/Money 或 external payload。
10. install 前保证 after staging/pins 已耐久。strict 只能使用真实 strict FileInstallCapability；observed_only 仅限步骤7资格并在破坏性安装前完成最后一次 trusted object/event continuity 检查。observed_only 的原 plan 已耐久保留实际 read-before B 与用户输入 N；未观察外部 C 可能在最后检查后被 N 覆盖且其 bytes 可能没有可恢复副本，后来另一个 C 也可能再次替换 current file，但这不允许丢弃已耐久的 B/N。任何已经观察到的 competition、stale Base、watcher gap、失权或第三种状态都不是 weak 豁免，进入原 conflict/paused/reprepare；安装归属未知固定 recovery_unknown。
11. verify 时，每个 written component 以**原 planned after**作为 expected current，并要求完整 FileObjectBinding/bytes 与连续 installation provenance；不能在自己的安装完成后又要求该 component 仍等于 before。所有 unwritten dependency 继续按原 before/cut expectation 重验，包括原 source/control/auth、Registry/rules、DependencyProof 正负范围和适用 Frontier policy。scope_dependencies 只接受已在P中可证明无关的扩展；unknown provenance、third_state、late competition、撤权或无法证明连续性保持 paused/conflict/recovery_unknown。此时仍没有本 decision 的 ChangeId。
12. seal 是唯一 decision commit point。written=planned after、原 plan/deps/auth、domain fence、SourceRevisionPlan lastIssued/empty-history依据及适用安装证明全部通过后，一个 P durable transaction 为**这个 portable decision** checked 分配唯一 ChangeId。只有实际 source change 且 after 是 managed source 的项目，才把原 SourceRevisionPlan.after SourceStamp 与该 ChangeId 合成为唯一 managed SourceVersion/2，并原子推进对应生产域实体的 H(D,E)。对每一个这样的 managed after，同一事务把 winning plan 冻结的 exact RevisionTokenBinding/2 选作该 SourceVersion 的唯一 canonical binding，按 §1.5.1 构造并签名 exact RevisionTokenSealArtifact/1，pin 其 canonical bytes，并写入 RevisionTokenSealOutboxItem/1；无需预先存在 Locator。source deletion 的 after=absent 仍是 portable effect 并使用这个 ChangeId，但不创建 managed SourceVersion、不使用删除后的 SourceRevisionPlan、也不推进 H；source 未变的 structure/lifecycle 等 portable effect同样有该 portable ChangeId而不造 source version/H。P-only control_only 与 true raw no_op 不产生 content ChangeId。该同一 P transaction 按既有 contract 写入/推进 domainCommitSequence、committed decision、原 receipt、effects、ReliableSaveState、适用 charge 与 outbox；费用只按原 decision 结算一次，重放/恢复不得重复收费。strict→reliable，observed_only→durable_observed_only。
13. 新 FA portable publication 只从步骤12已经 sealed 的事实生成 ContentCompletionProof/3 并推进 Frontier/2。proof.frontierBefore/frontierAfter 是实际 seal 前后 Frontier；notice.baseFrontier 与 InputDescriptor.expectedFrontier 保留原基线。scope_dependencies 下必须同时验证从原 base 到实际 frontierBefore 的完整连续 ChangeRecord/portable proof 链、P 中保存的无关扩展判断和原 DependencyProof仍有效；不能在 publication 时仅因向量数字增长、provider状态或最终hash相同补造“无关”。proof运输实际生产 SourceVersion before/after，不复制发送方 SourceObservation token。另由原 outbox 按 RevisionTokenSealOutboxItem/1 发布每个 sealed managed after 的 exact retained RevisionTokenSealArtifact/1 bytes；CP3 与 Notice 不增加 member/component。接收端在保存任何 token→production-version mapping 前，先按 §1.5.1 与 §6.3 验证 artifact 的历史 trust key、signature 与完整 association，再建立自己的 current Observation。ContentCompletionProof/2、/1 及其它历史 records 继续原 decoder/bytes/recovery，绝不按/3字段混解。
14. 若 seal 已成功而 publication/outbox/delivery 失败，decision、ChangeId、managed SourceVersion/H、ReliableSaveState、receipt与适用charge已经确定。恢复只发布同一 sealed decision 的原 proof 以及已经 pin 的 exact RevisionTokenSealArtifact/1/outbox bytes，或在当前授权下交付原bytes；不得重新安装 N、换 OperationId、重新分配 ChangeId、再次推进 H、重跑业务选择、回写后来 current source或重复收费。delivery 最后重验当前授权；撤权可以遮蔽交付，但不改历史 decision。

observed_only 的 B 只表示实际 read/pin 前像，绝不枚举最后检查后未读 C。later current=C 不改旧 receipt/Proof，也不削弱 B/N 的 durable retention。crash/unknown 按 §8 恢复，客户端不猜结果。

真正的原始无操作（raw no-op）仍使用 effectClass=no_op；sourceVersions=[]，InstallationState=not_required，ReliableSaveState=not_applicable，PortablePublicationState=not_applicable。按既有合同，其 domainCommitSequence 可以推进，但不产生内容 ChangeId，不增加 source revision 或 H，也不推进 Frontier head。仅 P 控制类结果使用 control_only；实际发生的可移植结构、生命周期、删除或来源效果使用 portable。
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

version 为该 component owner 逻辑版本。当前 new-profile 的 `policy` component 若承载 WorkspaceAuthorizationBundle/1，则 ComponentImage.version 必须精确等于 `bundle.authorizationRevision`，绝不是 `Policy/3.revision`；policy-only 或 trust-only 变化都因此只推进一次 component version。真实历史 profile 保留产生其 bytes 的原 component-version 映射，绝不按本规则重解释。digest 只用于验证 listed bytes，不定义 identity。

### 6.2 SourceStamp/1、SourceRevisionPlan/1 与 InstallationNotice/2

SourceStamp/1 的既有 closed shape 不变：
~~~json
{"kind":"decision_source","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"revision":<Counter>,"observationEpoch":<Counter>}
~~~
它在安装前可以确定，但只是同一原 plan 的拟议 managed source 地址，不是 SourceVersion/2、ChangeId、receipt 或第二份 current truth。decisionKey.commitDomain 是拟议 managed after 的生产域；entityRef 属于同一 Workspace；revision/observationEpoch 均为 1..MAX。SourceStamp 只有在同 DecisionKey 的原 plan 已 seal，并且按本节及 §6.3 能唯一对应到实际 managed SourceVersion/2 后，才可以作为该已封存版本的版本依据。没有 seal 就没有本次 managed SourceVersion，也没有本次 ChangeId。

除 §6.2.1 具名独立的 conflict-only /2 decoder 外，凡本次 plan **会产生新的 managed after**，installationPlan 必须为该实体保存一个 SourceRevisionPlan/1。closed shape 为：
~~~json
{"kind":"d6_source_revision_plan","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<SourceObservation/1|"absent">,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

跨字段规则全部必须成立：

- decisionKey 必须等于外层原 plan 的 DecisionKey；after.decisionKey 与之逐字相等；entityRef=after.entityRef。
- after 的生产域固定为 decisionKey.commitDomain。lastIssued 若非 `"none"`，必须是同一 entityRef、同一 production CommitDomain 的最新连续已封存 managed SourceVersion/2，并且其 revision 恰为该域该实体的 H(D,E)。不能以另一生产域的更大 revision、externalSequence、文件 hash、I cache 或当前行文代替。
- lastIssued=`"none"` 只有在 DependencyProof/2 及受保护生产历史能证明该 production CommitDomain 对该 entityRef 是完整空 managed 历史时合法；缺历史、P continuity 不可证明、portable history 有 hole 或 MAX/overflow 状态都不是 `"none"`。
- after.revision 在 lastIssued=`"none"` 时必须为1；否则必须是 checked(lastIssued.revision+1)。同一生产域跨 observationEpoch 不重置该序列。
- after.observationEpoch 是该原 plan 在 after production domain 对目标 source 冻结的生产观察世代。before 若来自另一生产域，其 sourceVersion.observationEpoch 不复制到 after。fresh/expected-absent目标使用同 plan 的可信 absent FileObjectBinding/目标后端观察世代。
- before 为 SourceObservation/1 时，它必须是该原 plan 实际读取的完整 current Observation，且 before.entityRef=entityRef；before=`"absent"` 只允许原 D3 identity/owner 计划已经证明真正 fresh/expected-absent 的分支，不能由客户端用路径不存在自行声称。
- afterPin 必须是本计划实际后像字节或值的精确 pin，并与实体种类相容：Document 的 owner NodeRef 对应 exact_source_document，ResourceRef 对应 resource_bytes，AnnotationRef 对应 annotation_value。受保护的 pin 记录必须绑定同一 entityRef；仅 digest 相等不能替代这种绑定关系。
- 对 D3 fresh content，entityRef/after 只能来自原 stage12 私有 candidate map 和 winning reservation；不得在 prepare/preview 公布 fresh Ref。原 plan 必须在绑定 revision 的 source 物化前固定一条拟议 RevisionTokenBinding/2。D7 DefinitionTransfer 的两遍 Q/Locator 物化必须使用同一 candidate map、同一 after SourceStamp 与同一拟议 binding，不能在第二遍或 commit 重新取 revision 或 token。winning planning CAS 将这条 binding 与 SourceRevisionPlan/pins 一起冻结；loser/aborted/seal 不可证明的记录不能借另一 decision 变成 canonical。
- planning CAS 保存完整 SourceRevisionPlan、相关 pins 与 H/empty-history 依据；winning plan 后这些成员不可变。seal 前重新验证 lastIssued/empty-history、domain fence、after pin 与原依赖。若同域同实体出现新的 sealed managed version，原 plan stale/conflict；不能把 after.revision 原地改成新的 H+1。
- SourceRevisionPlan 不分配本次 ChangeId。seal 时只有实际产生 managed after 的 source change，才把 after SourceStamp 与 seal 分配的 ChangeId 合成为 managed SourceVersion/2，并原子推进 H(D,E)。true raw no-op、纯 structure/lifecycle/control 且 source unchanged、以及 source deletion after=absent 都不创建 SourceRevisionPlan、managed after 或 H increment。删除和其它实际 portable structure/lifecycle effect仍可按其原 portable-effect规则在 seal 获得 ChangeId；不能因“无 SourceRevisionPlan”误归为 no_op。
- equal-byte external admission 仍产生 managed after，因此必须有 SourceRevisionPlan；其 before.sourceVersion 是完整 external SourceVersion/2，而 after revision 只由当前生产域的 lastIssued/H 决定。externalSequence 永不进入 after.revision。

InstallationNotice/2 的 closed shape保持不变：
~~~json
{"format":"weftext.installation-notice","version":2,"decisionKey":<DecisionKey/2>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"before":<ComponentImage/1>,"after":<ComponentImage/1>}...]}
~~~
components 非空且按固定 rank+canonical key 排序唯一。observed_only 仅 §4.1；其它 portable author plan 必须 strict。notice 在首个 portable current install 前 durable。baseFrontier 是该原 plan/notice 冻结的 Frontier；它可以包含已经存在的历史 ChangeId，但 notice **不包含本 decision 尚未 seal 的 ChangeId**，也不包含 receipt、approval、Money、external payload 或 credential。SourceRevisionPlan 保存在受保护 P/plan 中，不通过 InstallationNotice 变成第二个公开版本表。

### 6.2.1 仅供冲突解决的 SourceRevisionPlan/2

普通 source 生产和 native copy 的 fresh 输出仍逐字使用上述 SourceRevisionPlan/1。只有 D3 §10.1 的 canonical-source 物化分量能使用以下独立内部 decoder；公共 commit wire、InputDescriptor、SourceObservation、SourceStamp 与 CP3 形状均不变：

~~~json
{"kind":"d6_source_revision_plan","version":2,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<ConflictInstallInput/1>,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

/1 关于 decisionKey/entityRef、当前生产域 H 与完整空历史证明、不可变拟议 after、exact after pin、strict install、CAS、同 P seal 及恢复的全部交叉规则保持。唯一 before arm 是 §3.1.1 的完整 guarded ConflictInstallInput/1，DecisionKey/entityRef 相等；禁止普通 Observation、absent、自由 metadata 或自制 wrapper。当前生产 observationEpoch 来自可信 target 安装世代，不取 selectedHead 历史生产 epoch。并列 native fresh copy 有独立 /1 plan 和真正 fresh absence 证明；existing canonical entity 不得改称 absent 或再次分配。单 decision 中每实体至多一个最终 source-version plan；canonical/native 修改若重叠，必须满足 D3 §10.1.1.3 的 exact before/final bytes、metadata、version basis 与 typed-effect 相等，否则在其 stage14、planning 前拒绝，不能分两次写/增版。

D3 在显式 resolution binding 下选择 canonical claim/lifecycle 和完整来源，D6 仅安装其 exact 已验证最终 after。source-state change 规则唯一：最终 bytes/value 不同，或显式切换 canonical birth claim、所选生产 SourceVersion，均是真实 canonical admission，即使 bytes 相等也一样；它产生 checked H(当前生产域,entity)+1 的一个 managed after，并使用本 decision 唯一 seal ChangeId。若最终 bytes/value、所选生产版本及 claim 全部保持，则不产生 /2 plan、sourceChanges 或 H 增量；纯 placement/lifecycle/conflict-state portable 修改仍可使用本 decision ChangeId。already-resolved 且全部作者/control 效果为空的已证明相同选择是真正 no_op，无 ChangeId/H 增量。fresh-copy 必有真实 fresh identity，从不是 no_op。

CP3 sourceChanges.before 取 wrapper 的真实已安装生产 SourceVersion；after 由本 exact after SourceStamp 与共同 seal ChangeId 组合成 managed SourceVersion。不得把 selected historical head 的版本冒作物理 before，也不把旧版本复制成新 managed after。原 notice、完整物理 metadata/source components、conflict effect、D3 主回执、companion 与 effects 同一 P 事务提交。planned/unknown 恢复保留原 /2 wrapper/version basis/pins/guard，只继续原安装，不重新取 current Observation、不重选 head、不原位改 H、不由文件猜 seal。真实历史 /1 plan 仍按 /1，不升级；decoder 只按显式 version 选择，不从字段缺失猜测。

D3 §10.1.1.3 另拥有受保护 D3CanonicalEffectPlan/1 和公共 D3CanonicalEffects/1 语义扩展。前者、全部 exact before/selected/Result9 pins 与原 native plan 由既有 D7 /3 record、native descriptor 和 input-use guard 原子绑定，不增 ownerKind 或 submit。原十二个 primary receipt 数组只覆盖 native 分量；扩展以独立原 schema reference/S/lifecycle 证据及显式 physical-effect aliases 证明真实 canonical 分量。D6 在同一 P seal 保存完整两个分量和 D7 full 公共投影，以及同一主 receipt/companion。D6 sourceVersions 与 CP3 恰覆盖全部实际物理 source change 一次，包括主数组之外的 canonical 改变；alias 不重复 write 或 H。缺 mandatory extension 不得新的 prepare/seal 成功；真实 seal 后缺交付证据则 effects_unavailable，不能改历史 commit。saved/planned/unknown 保留原完整 plans/extension/pins，普通 Observation 和真实历史 decoder 规则不变。

### 6.2.2 D6 source-conflict SourceRevisionPlan/3

只有 D6 所属 §9.4 `source_merge` 与 `choose_source_head` arm 可使用这个独立内部 decoder。D3 canonical resolution 仍只能使用 /2，ordinary/fresh source production 仍使用 /1；不修改 public commit wire、InputDescriptor 成员集、SourceObservation、SourceStamp、CP3 或 DependencyKey union。

~~~json
{"kind":"d6_source_revision_plan","version":3,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<SourceConflictBefore/1>,"versionBasis":<SourceConflictVersionBasis/1>,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

/1 对 outer DecisionKey/entityRef、当前 production domain exact H 或完整空历史证明、checked H+1、不可变 SourceStamp、exact after pin、strict install、单 planning CAS、单 P seal 与 recovery 的规则全部适用。before.decisionKey/entity 及完整 resolution-use 关联必须与 outer plan 一致；versionBasis arm 必须与已冻结 ConflictResolution/2 arm 一致。after.observationEpoch 取本操作 production domain 中 before.observationEpoch 所证明的可信当前安装世代，不得取任一历史 head 的 production epoch。没有 absent branch，也不能用普通 Observation 冒充。

对 `choose_source_head`，只要最终 exact bytes/value 与 before.sourcePin 不同，**或** versionBasis.sourceVersion 与 before.sourceVersion 不逐字相同，就必须形成真实 source-state admission。因此两个 sealed head 即使 bytes 相同但 production SourceVersion/2 不同，选择另一个 production version 仍产生一个 /3 managed after 并 checked H+1。若所选 head 的 exact production version 与最终 bytes/value 都已等于 installed before，则 source 未变：不产生 /3 plan、CP3 sourceChanges 或 H increment，但 conflict-record resolution 仍是实际 portable control effect。对 `source_merge`，versionBasis 证明 exact base/all-head lineage；当 proposed exact merged bytes/value 与 before 不同时形成 source-state admission。真正 bytes 相同且不选择另一个历史 production version 的 merge 属 source-unchanged，同样不产生 /3。

winning planning CAS 保存完整 before、versionBasis、branch/base/head/proposed pins、H basis、after pin、semantic preview 与原 ConflictResolutionInput/2。losing/aborted prepare、changed expectedKey、changed installed physical before、错误 selected production version、arm 不匹配、缺 pin/proof 或 H basis 不可证明，都不能原地改成 winner，也不分配 ChangeId/H。step 6 必须复验所有这些 exact 成员与当前 dependencies；进入 planning 后的 crash/retry 只按 §5/§8 恢复该冻结 plan。

唯一 P seal 中，真实 /3 source admission 才把冻结 after SourceStamp 与本 decision 唯一 ChangeId 组合，H 只推进一次，产生恰一个 managed SourceVersion/2 及其普通 revision-token seal artifact；CP3.sourceChanges.before 来自 before.sourceVersion，after 来自该 managed version。历史 chosen head/versionBasis 绝不能替代真实物理 before。安装或 seal provenance 不明时保持 paused/recovery_unknown；相同 bytes 或后来 current file 都不能猜 success。saved recovery 只补发/重放原 sealed bytes；planned recovery 不重新取得普通 Observation、不重选 head、不改 versionBasis/H，也不建立第二次 seal。
### 6.3 ContentCompletionProof/3 与历史 /2

新 FA portable decision 使用 ContentCompletionProof/3。committed 的 closed shape 为：
~~~json
{"format":"weftext.content-completion","version":3,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersion/2|"absent">,"after":<managed SourceVersion/2|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~

/3 保留 /2 的其余成员集合与责任边界，只把 portable sourceChanges 从本地当前观察句柄改为生产 SourceVersion。所有成员还必须满足：

- decisionKey.workspaceRef 与所有 components/sourceChanges 的 Workspace一致；changeId.commitDomain=decisionKey.commitDomain。receiptDigest 只校验同decision原receipt bytes，不授receipt读取、approval/Money消费或execution takeover。
- components 与该 decision 的 InstallationNotice/2 key集合逐项相等、按原固定rank/canonical key唯一排序，且 after 必须是实际安装并封存的 after。proof不能添加notice未声明的component，也不能用digest相等代替真实component bytes/owner version验证。
- sourceChanges 可以为空；非空时按完整 EntityRef canonical key唯一排序，恰覆盖本decision实际发生的source状态变化。source unchanged的structure/lifecycle portable effect不列假sourceChanges。
- before 若不是 `"absent"`，必须是原 plan 的 before SourceObservation/1、D3-only /2 plan 的 ConflictInstallInput/1，或 D6 source-conflict /3 plan 的 SourceConflictBefore/1 中完整 production SourceVersion/2；普通观察可以是managed或external，两个 guarded wrapper 都必须按其具名路径证明，生产commitDomain可不同于本decision域。CP3 永远记录真实安装/观察到的物理 before，不能只记录后来选择的历史 branch version。before=`"absent"`只用于原plan已经证明的fresh branch。
- after 若不是 `"absent"`，必须是managed SourceVersion/2；其entityRef等于sourceChanges.entityRef，commitDomain=decisionKey.commitDomain，changeId逐字等于proof.changeId，revision/observationEpoch逐字等于该实体实际解码的SourceRevisionPlan/1、D3 conflict-only /2 或 D6 source-conflict /3 的 after SourceStamp，且其前一生产历史与lastIssued/empty-history验证一致。一个SourceRevisionPlan只能产生这一项sealed after。
- 从 external 到 managed 的接纳（包括相同字节接纳）以 external 生产版本作为 before、managed 生产版本作为 after；externalSequence 不迁入 after。来源删除以生产 SourceVersion 作为 before，after=`"absent"`；此分支不产生 after SourceRevisionPlan、managed SourceVersion 或 H 增量，但仍是本次 portable decision 的实际来源删除，并由 proof.changeId 标识。真正的 raw no-op 不产生 /3 portable source change。
- observed_only只证明原decision实际read-before B、输入N的耐久安装和seal，不证明最后观察后不存在未观察C；proof不得补造C或把later current bytes改写成原after。

frontierBefore 表示 seal 前实际验证通过的 Frontier，frontierAfter 必须恰为在 frontierBefore 上加入 proof.changeId 后所得的非回退 Frontier；其它domain head不得回退。InstallationNotice/2.baseFrontier 与 InputDescriptor.expectedFrontier 仍保存原计划基线，不被proof改写。

frontierPolicy=`exact` 时，frontierBefore 必须逐字等于原 expectedFrontier、DependencyProof.baseFrontier 和 notice.baseFrontier。

frontierPolicy=`scope_dependencies` 时，frontierBefore 可以是 notice.baseFrontier 的非回退扩展，但生产者只有在以下全部成立时才能seal并生成/3：
1. 从notice.baseFrontier到frontierBefore每个新增head都有完整、连续、已验证的portable ChangeRecord/相应完成证明链，无hole、伪造head或仅靠sequence数字跳跃；
2. 原 frozen DependencyProof/2 的全部source/control/authorization/positive-negative range entries仍在同一原plan下有效，并证明新增sealed effects与这些bound keys无关；真实SourceObservation、FileObjectBinding/pin、Registry/rule、授权、membership/negative-range或其它依赖变化仍使原plan失败；
3. 该无关扩展的验证与原plan、原notice、实际seal frontier一起耐久保存在P恢复记录中，publication不能在P丢失后仅比较两个Frontier数值重建“无关”结论。

接收端接纳/3时必须验证portable trust、decision/changeId关系、notice/proof/components、生产SourceVersion以及从notice.baseFrontier到frontierBefore再到frontierAfter的完整连续sealed记录链；不能因为两个向量的sequence变大就跳过缺失的中间因果记录。对每个 non-absent managed sourceChanges.after，必须存在恰一份另行携带、key 为 {proof.changeId,entityRef} 的 RevisionTokenSealArtifact/1。通过正常 disclosure/portable-continuity 门后，Core 令 C=proof.frontierBefore，并针对 artifact.trustKeyId 与 association.decisionKey.commitDomain 在 C 上调用 validate_historical；唯一例外是 §10.2 的 exact same-P bootstrap-genesis 规则。Core 从返回的 raw public key 重算 trustKeyId，验证 exact domain-separated signed body 的 Ed25519 signature，strict decode association，并要求 association.decisionKey=proof.decisionKey、association.changeId=proof.changeId、association.sourceVersion 与 sourceChanges.after 逐字相等及 §1.5.1 全部 SourceStamp/binding equality。只有全部通过后才能把 binding.token 保存为该生产版本 canonical address。合法 forwarder 只原样转发 artifact bytes，不需额外被信任或重签。仅 stamp/sourceVersion/digest 相等不能认证 t 或同 stamp t2。证据缺失、malformed、非 canonical、不受信或 signature-invalid 为 incomplete/unavailable；同一 exact SourceVersion 出现两份非逐字相等但都能从已锚定历史 root 验证成功的 artifact 才是 integrity 矛盾。CP3/Notice member/component 集合不变。/3 不是私有 DependencyProof 的 portable 副本，不授新的 complete Query/Action proof；strong consumer 必须在自己的 observerDomain 建立新 current SourceObservation 和本地完整 DependencyProof，绝不复制发送方 SourceVersionRef/sourceToken。持久 managed Locator 只有当稳定 token 经该 verified canonical binding 解析到同一 production SourceVersion，且新的 current Observation.sourceVersion 逐字相等时，才可在新读取中取得资格；旧 selector、preparation、Draft/map 或 Action evidence 不更新。

未seal且全部component已安全恢复before时，/3 restored closed shape 为：
~~~json
{"format":"weftext.content-completion","version":3,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...]}
~~~
restored 禁止 changeId、guarantee/writeProtection/semanticState、frontierBefore/frontierAfter、sourceChanges、receiptDigest 或任何成功语义；components必须证明全部已安全恢复到原before。无法证明安全恢复时不得生成。

ContentCompletionProof/2 保持历史 closed definition和原decoder/bytes，不把SourceVersionRef静默改成生产SourceVersion。历史 committed 仍是：
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersionRef/1|"absent">,"after":<SourceVersionRef/1|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~
历史 restored 仍是：
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...]}
~~~
/2按原saved bytes、原token绑定、原接纳/恢复gate履约；不得用/3的新证明升级、重编码或补写旧r5，也不得从/2的SourceVersionRef反推一个不存在的portable生产版本字段。
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
D3DecisionCompanion/2 的 closed shape、version=2 与既有字段集合保持不变，不增加 ChangeId、SourceVersion、WriteScope、SourceRevisionPlan 或其它成员，也不是第二份 receipt。对于使用当前生产者合同的新 protocolOwner=D3 decision，D3 primary receipt 与这个 companion 必须在同一个 P seal transaction 中原子写入；两者引用同一 DecisionKey、同一 seal 后 domainCommitSequence 和同一 effectsToken。companion 只保存 D6 侧既定关联，不建立另一条 ledger、另一份决议真相或额外写权限。

完整 P2 D3 main/Lexicon/Impact 候选现已定义这些 P1 生产规则的 native descriptor/companion 消费，但候选存在不等于独立接受、其余跨 owner 配套完成或 §17 激活授权。新执行 consumer 检查只适用于 §5 的 unseen decision；saved/planned/unknown decision 仍按原恢复分流。D3 必须真实消费 d3_identity_operation/12 的 OwnerInputBinding、完整普通 SourceObservation/1 或仅限 §3.1.1 resolution 用途的 guarded ConflictInstallInput/1，以及 DependencyProof/2；会产生 managed after 时消费同一原 plan 显式准入的 SourceRevisionPlan/1 或 conflict-only /2 与 d6_source_revision/2 绑定；seal 后的生产 SourceVersion/2、SourceVersionRef/1、ContentCompletionProof/3 及恢复分流也必须与本 Control 的唯一生产者定义一致。D6 不得在 companion 中复制这些对象来绕过 D3 owner，也不得改变 D3 wire12、Locator revision-token 词法、D4 inner selector wire 或 D3 实际 write scope。

旧 D3 primary receipt、旧 companion、旧 saved/planned/unknown decision 继续按其原版本、原 bytes、原 decoder、原 pin/授权/continuity 与恢复责任履约，不因新 P1 类型出现而升级、重编码或取得新的 strong qualification。其余 P2 配套、后续 D7/D10 consumer 以及完整 fresh 联合接受完成前，不得半包激活依赖这些新 producer 的 managed success；同时，已经批准且不依赖缺失 strong consumer 的 ordinary/local 操作继续按其原 owner 资格工作，不能仅因本 companion 配套尚未完成而永久取消。
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

P 内每个 planned v2 decision继续保存每个 component 的 internal InstallComponentState：

pending|staged_durable|installed_after|restored_before|third_state|unavailable。

恢复总则固定为**先读 P 中原 DecisionKey 状态和原版本记录，再读取该记录引用的 InstallationNotice、write-set bindings/pins 和实际 component**。客户端超时、文件存在、mtime、provider状态、I cache或digest都不能替代P，也不能决定是否已经 committed。

对实际 file/component只允许以下分类：
- exact_before：完整 trusted FileObjectBinding/控制binding与 bytes/value 逐项等于原 planned before，并且其读取连续性可证明；
- exact_after：完整 trusted binding与 bytes/value 逐项等于原 planned after，而且能证明 installation provenance 属于这个原 plan；
- third_state：其它已materialized bytes/object、observed competing state、unexpected absence/presence或与原plan不同的第三种状态；
- unavailable：当前不能安全读取、不能证明backend/object/clock/domain/installation连续性，或必要证据不可用。

相同 digest、相同文本、同路径、同裸revision或I重建都不能替代binding/provenance，不能把third_state/unavailable升级为exact_after。证实P/portable record损坏继续走原integrity_conflict门；单纯“无法证明”保持availability/recovery状态，不伪造成证实损坏。

若 written component 是 exact_after，继续以原 planned after 作为 expected current；未写 dependency 仍与原 before/cut expectation 比较。自己的安装不会因为before SourceObservation已经变化而自动自冲突。scope_dependencies的无关Frontier扩展只能按§1.4/§5和原P中证据消费；恢复不得重新签SourceObservation、source token、PinRef或DependencyProof stamp来让旧plan通过。

### 8.1 planned、committed 与历史恢复分流

未seal planned decision **没有本decision的ChangeId**。planned恢复只复用原 canonical request/fingerprint、InputDescriptor、适用 SourceRevisionPlan/SourceStamp、before/after pins、write set/reservation、OperationId、budget/attempt counters、WriteProtection、InstallationNotice、installation state与原owner版本依据。不得重新采样current before、identity、H、revision token、target/Query/current page，也不得new-prepare第二个decision替代它。原SourceRevisionPlan如果存在，其after revision保持winning plan冻结值；继续seal前必须重新证明原lastIssued/empty-history依据没有被新的同域managed version破坏。

- planned、未install：在原domain/fence/P continuity和当前授权允许时继续同一原plan；资格暂不可证明则保持planned并进入相应paused状态，不把availability缺失变成business rejection。
- installing：只有所有实际component都能按原plan分类为exact_before/exact_after且installation lineage连续时，才继续原plan。任何third_state、已观察competition或晚到外部替换保留current bytes、B/N、pins和版本依据并进入conflict/paused；安装结果或归属不能证明时固定recovery_unknown。
- 全部written看似after但seal未知：**先读P**。P仍planned就只有原plan及拟议SourceStamp，不得从files/hash/SourceStamp猜出ChangeId、managed SourceVersion或成功；P已committed才使用该记录保存的真实ChangeId、SourceVersions、receipt与effects。
- 原pins、clock epoch、CommitDomain/fence或P continuity无法证明时，不从Derived Index、当前文件、相同hash或新设备空control DB重建。保持原planned+paused/recovery_unknown或按证实损坏进入原integrity门；protected last-reference evidence仍按原retention责任保留。

committed decision 与planned不同。seal已经成功后，原ChangeId、实际managed SourceVersions/H结果、domainCommitSequence、receipt、effects、charge和decision bytes均已确定：
- response丢失时，在当前**原saved实际效果范围**的披露/交付授权通过后返回原receipt bytes。无需旧before SourceObservation仍current、旧Frontier等于r6 current、旧业务dependency重验成功或preview仍有效。当前撤权可使交付not_visible，但不能撤销或改写saved decision。
- 新FA portablePublicationState=pending时，只从原sealed decision、原notice/components及原保存的scope_dependencies扩展证据补发同一ContentCompletionProof/3/outbox；不得重装N、换OperationId、再次分配ChangeId、推进H、重算source version或重复收费。历史/1、/2 decision继续生成/恢复其**原版本**proof并使用原decoder/bytes，不能自动升级/3。
- publication/outbox/Derived Index更新失败只补相应owner的派生/运输工作，不回滚author decision，也不能从I重建P的approval/Money/unknown责任。
- 当前r6的新SourceObservation、DependencyProof或complete proof只证明r6；不得回溯升级、否定或重编码历史r5 receipt/saved bytes。Undo/restore仍是新的明确plan，而不是receipt replay倒退current。

saved、planned、unseen的业务分流以§5为唯一顺序；§8只恢复已经存在的记录，不提供绕过§5的第二提交入口。旧 D6/D3/D7/D8 planned/saved/unknown records继续按各自原版本、原pins、原authorization/continuity与原decoder责任恢复；新 SourceRevisionPlan、ContentCompletionProof/3 或其它P1类型只适用于明确的新版本路径，不机械迁移历史记录。

### 8.2 observed_only 恢复限定

observed_only 的 read-before B只表示原plan实际读取并耐久pin住的前像，N是同一原plan耐久保存的用户输入。最后检查后从未被观察的外部C可能在N安装时被覆盖且没有可恢复副本，record不得发明它；以后另一个C也可能替换current file，但已经耐久的B/N不能因此丢弃。

任何已经观察到的competition、stale Base、watcher gap或third_state都不是批准的弱竞争，继续走conflict/reprepare/paused；unknown install固定recovery_unknown，相同hash/文本不得猜success。strict plan失败不能在恢复时改成observed_only。已经committed的observed_only只补原receipt/proof/outbox并保留B/N retention，不再次安装N或把后来current file写回成原after。
## 9. ConflictKey/1、ConflictRecord/1-/2 与 resolution prepare

### 9.1 ConflictSubject/1 与 kind

ConflictSubject closed：
- {"kind":"workspace","workspaceRef":WorkspaceRef}
- {"kind":"entity","ref":EntityRef}

conflict kind闭集：
冲突 kind 闭集前四项为 source_concurrent | placement_concurrent | lifecycle_concurrent | identity_collision；
其余为 policy_concurrent | incomplete_transport | placeholder。

ConflictSubject、conflict kind 与 ConflictKey/1 均不因 ConflictRecord/2 升版而改变。subject/head 必须来自真实受管portable历史；不能为了表示一个尚未封存、无 ChangeId 的 external race、unknown install 或第三种bytes而伪造head。此类状态继续保存在installation/recovery/conflict evidence中，直到它能够由真实已封存branch表示。

### 9.2 ConflictKey/1、ConflictId 与 ConflictRecord/1-/2

ConflictKey/1 exact保持：
~~~json
{"workspaceRef":<WorkspaceRef>,"kind":<conflict-kind>,"subjects":[<ConflictSubject>...],"heads":[<ChangeId>...]}
~~~

subjects、heads 均非空、无重复。subjects按 workspace rank0/entity rank1+canonical ref排序；heads按 ChangeId key排序。所有成员同 Workspace。placement/lifecycle/source/identity至少含一个 entity subject；policy至少含 workspace subject。incomplete/placeholder按实际受影响 workspace/entity列出。每个head必须是该Workspace内已经验证连续sealed的真实ChangeId，并能由对应portable change history证明与本ConflictKey的subject/kind相关；仅有sequence数字、hash、mtime、path或未封存外部状态都不能生成head。

ConflictId 算法保持不变：ASCII d6c: + 64 lowercase hex，其中 hex=SHA-256(ASCII "D6-ConflictKey/1" + NUL + D3-CJ/3(ConflictKey))。ConflictId只作完整ConflictKey的稳定地址；验证时必须加载并byte-compare完整key。ConflictRecord版本不进入ConflictKey或hash输入，因此本次**不**升级ConflictKey/1、ConflictId格式或`D6-ConflictKey/1` hash domain。

新FA当前记录使用 ConflictRecord/2：
~~~json
{"format":"weftext.conflict","version":2,"conflictId":<ConflictId>,"key":<ConflictKey/1>,"state":"open|resolution_prepared|resolved|superseded","createdAtFrontier":<Frontier/2>,"supersedes":[<ConflictId>...]}
~~~
createdAtFrontier 必须是创建该记录时已经验证连续的Frontier/2，并至少覆盖key.heads：对每个head的CommitDomain，frontier中对应head.sequence不得小于该head且中间sealed记录连续可验证。它不是Query完整性或source materialization证明。supersedes可空，按ASCII排序唯一，且不得包含自身ConflictId。

历史 ConflictRecord/1 的 closed bytes 保持：
~~~json
{"format":"weftext.conflict","version":1,"conflictId":<ConflictId>,"key":<ConflictKey/1>,"state":"open|resolution_prepared|resolved|superseded","createdAtFrontier":<Frontier/1>,"supersedes":[<ConflictId>...]}
~~~
/1 继续使用原 Frontier/1 decoder、原 bytes、原授权与恢复gate；禁止把version=1的createdAtFrontier按Frontier/2解释，也禁止为了迁移而重编码resolved/superseded历史。若一个/1 open或resolution_prepared记录在其历史路径中仍可处理，继续走原decoder/gate；不会只因当前实现支持/2而原地改成/2。新head改变完整ConflictKey时，新的当前记录使用/2并可在supersedes中引用旧ConflictId；旧record保持不变。因为ConflictId只由key决定，不允许对同一ConflictKey同时创建/1与/2两个不同“current record”来伪造一次版本升级。

state闭集仍为 open | resolution_prepared | resolved | superseded。新真实head到达时，旧open/resolution_prepared记录只能按其版本规则进入superseded并产生绑定新完整key的后继记录；resolved历史不可重开。supersedes只是历史链，不授merge、LWW、write或resolution资格。

### 9.3 conflict read

request shape保持：
~~~json
{"wireVersion":2,"kind":"d6_conflict_read","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>}
~~~

读取顺序保持非披露：先closed decode，再验证conflict_read capability及所有subject的最低state disclosure，再读取record；无权、unknown address或不能先证明subject披露统一not_visible。通过资格后，Core加载ConflictId对应的完整key并byte-compare，然后按record自身version选择decoder：version=2使用ConflictRecord/2+Frontier/2，version=1只使用历史ConflictRecord/1+Frontier/1。unknown record version在已通过披露后返回现有state_unavailable，不按字段猜版本，也不回退另一个decoder。读取任一版本record都不授source bytes、branch选择、conflict_resolve或作者write。

当前/2读路径还必须验证key中的heads与createdAtFrontier之间上述连续sealed关系；portable history缺段、伪造head或record bytes损坏属于完整性/状态不可用，不把缺失head删掉后返回一个较小conflict。/1按其原历史decoder/gate，不用/2的新Frontier条件回溯改判其已保存bytes。
### 9.4 conflict resolution prepare

当前 new-FA prepare 继续使用闭合 wireVersion=3 request：
~~~json
{"wireVersion":3,"kind":"d6_conflict_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/2>,"budget":<BudgetBinding>}
~~~

`ConflictResolution/2` 继续是闭合 union：
~~~text
{"kind":"source_merge","ownerNodeRef":NodeRef,"source":text}
{"kind":"choose_source_head","ownerNodeRef":NodeRef,"head":ChangeId}
{"kind":"policy_bundle_choice","selected":WorkspaceAuthorizationBundleAddress/1,"policy":Policy/3,"freshAuthorizations":[FreshDomainAuthorizationSpec/1...]}
~~~
前两个 arm 保持原 source 语义。`WorkspaceAuthorizationBundleAddress/1` 保持：
~~~json
{"kind":"d6_workspace_authorization_bundle_address","version":1,"head":<ChangeId>,"authorizationRevision":<Counter>,"trustRevision":<Counter>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~
address 必须选择 `expectedKey.heads` 中恰一个成员。Core 加载该 head 已验证 CP3 的 policy-component after-image，要求 `ComponentImage.version=authorizationRevision`、byteLength/sha256 相等，strict-decode 完整 `WorkspaceAuthorizationBundle/1`，并要求 trustRevision 匹配。digest 只对 exact canonical stored bundle bytes 做 SHA-256 比较，绝不替代 selected ChangeId 或真实 bytes。

`FreshDomainAuthorizationSpec/1` 仍精确为 `{"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1"}`。数组可空，按 D3-CJ/3 bytes 排序唯一，不含 caller public/private key。

处理 `policy_concurrent` 时，读取 branch contents 前必须具备 current `conflict_resolve`、workspace `policy_admin`、完整 subject disclosure、exact `expectedKey`、每个 head 的 CP3/component bytes 与连续 portable-history proof。每个 head bundle 必须同 Workspace，并具有逐字相同的已锚定 `WorkspaceTrustRootFingerprint/1`；head 缺失/损坏、root 不同、共同历史不可证明或 key 已变化都属于 unavailable/integrity conflict。`selected` 必须精确定位一个 head/bundle；current host state、arrival order 与 LWW 都不能替用户选择。

#### Compromise fact extraction and carry

compromise fact 表示原始被泄露 key 及其原始 causal cut，不表示后来携带该事实的 resolver。闭合 `TrustConflictCarry/1` 为：
~~~json
{"kind":"d6_trust_conflict_carry","version":1,"factId":"sha256:64-lowercase-hex","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","compromisedTrustKeyId":"sha256:64-lowercase-hex","originAction":"revoke|rotate","originDecisionKey":<DecisionKey/2>,"originDeclarationRevision":<Counter>,"originDeclarationDigest":"sha256:64-lowercase-hex","originActivationChangeId":<ChangeId/1>}
~~~
`originDeclarationDigest` 精确为 `"sha256:" + lowercase_hex(SHA-256(D3-CJ/3(complete original WorkspaceTrustDeclaration/1 including rootSignature)))`。origin declaration 必须是直接 root-signed 的 `mode:"compromise"` declaration：若 `action:"revoke"`，则 `compromisedTrustKeyId=trustKeyId`；若 `action:"rotate"`，则 `compromisedTrustKeyId=replacesTrustKeyId`。`originDecisionKey`、revision 与 digest 定位该 exact declaration；`originActivationChangeId` 从它的 DecisionKey→CP3 activation binding 重新派生。后续 resolver ChangeId 永远不能代替这个原始 cut。

`factId` 精确为 `"sha256:" + lowercase_hex(SHA-256(ASCII "D6-Trust-Compromise-Fact/1" || NUL || D3-CJ/3({workspaceRef,commitDomain,profile,compromisedTrustKeyId,originAction,originDecisionKey,originDeclarationRevision,originDeclarationDigest,originActivationChangeId})))`。这些字段构成完整 fact identity。同 factId 的两份记录必须逐字相等，否则 history integrity-conflicted。

对每个已验证 head，Core 先继承最长共同 trust prefix 末端已经 effective 的全部 compromise facts，再折叠该 head 的 divergent suffix，得到它的 effective compromise set。直接 `revoke mode=compromise` 产生其 trustKeyId fact；直接 `rotate mode=compromise` 产生其 replacesTrustKeyId fact；已经接纳的 `resolve_conflict` 则贡献其全部 `inheritedCompromises`。每个 inherited member 只有在 Core 重算 factId、加载并验证所指原始 declaration/root signature、核对 mode/action/key 映射、重新派生 exact 原 activation ChangeId，并验证 carrying resolve_conflict declaration 本身后才可接纳。该 fold 可递归：同一 fact 即使被一个或多个早期 resolver 携带，仍是同一原始 fact。

新 resolution 的 effective set 是所有已验证 heads 的 effective compromise set 并集。只按 factId 去重；同 factId 且 byte-equal 的重复项折叠为一条，同 factId 但 bytes 不同直接拒绝。canonical order 为 factId ASCII 升序。selected chain 已经 effective 的 factId 只从新 declaration 的 `inheritedCompromises` 数组中扣除，避免重复存储；outcome 安全判断始终使用完整 effective union。因此即使 selected 是仍保持 K1 current 的纯 policy branch，只要任一合法 losing branch 已证明 K1 compromised，K1 也绝不能被复活。

每条原始 compromise declaration、其 DecisionKey→CP3/ChangeRecord activation evidence、每条仍携带该 fact 的已保留 resolver declaration，以及继续引用该 carried fact 的 descendant declaration，均按 Storage §PL-IR-01 属于 public-history last reference。递归 carry 永远不改写原始 activation cut。

#### Fresh authorization eligibility, outcomes and PoP

定义 `affectedDomainProfiles` 为两部分并集：（a）已验证 heads 之间 current normalized trust state 确实不同的 exact domain/profile；（b）effective compromise union 中至少一个 fact 所指的 exact domain/profile。`freshAuthorizations` 每一项都必须属于 affectedDomainProfiles，并且在 selected chain 上要么 current state 为 `none`，要么 selected current key 已被 effective compromise fact 命中；否则 prepare 拒绝这个 unrelated fresh request。安全的 selected current key 不能通过本 resolver 被 rotate。因此 conflict resolution 不是 generic add/rotate surface。

若所有 head 的 trust history 逐字相同且 `freshAuthorizations` 为空，则可以做 policy-only resolution，无需 root private key。否则 resolver 必须有已锚定 root 与 usable `WorkspaceTrustRootKeyHandle/1`，并在 selected trust chain 后恰追加一条 root-signed `WorkspaceTrustDeclaration/1`、`action:"resolve_conflict"`：
~~~json
{"kind":"d6_workspace_trust_declaration","version":1,"workspaceRef":<WorkspaceRef>,"revision":<Counter>,"predecessor":<predecessor>,"decisionKey":<DecisionKey/2>,"action":"resolve_conflict","conflictId":<ConflictId>,"resolvedHeads":[<ChangeId>...],"selected":<WorkspaceAuthorizationBundleAddress/1>,"inheritedCompromises":[<TrustConflictCarry/1>...],"outcomes":[<TrustConflictOutcome/1>...],"rootSignature":"<86-ASCII-unpadded-base64url>"}
~~~
`revision=selected.trustRevision+1`；predecessor 是 selected chain 最后 declaration 的 exact digest；resolvedHeads 必须逐字等于排序后的完整 `expectedKey.heads`；inheritedCompromises 必须是上文 effective-union-minus-selected-existing、按 factId canonical 排序的集合。root 继续按普通 `D6-Workspace-Trust-Declaration/1` 域对仅移除 rootSignature 的 body 签名。

`TrustConflictOutcome/1` 按 `(commitDomain,profile)` 的 D3-CJ/3 排序唯一，并完整覆盖：head state 不同、effective compromise fact 命中或存在 eligible fresh request 的每个 exact domain/profile：
~~~text
{"commitDomain":CommitDomain/2,"profile":"d6_revision_token_seal/1","state":"keep_current","trustKeyId":"sha256:64-lowercase-hex"}
{"commitDomain":CommitDomain/2,"profile":"d6_revision_token_seal/1","state":"none"}
{"commitDomain":CommitDomain/2,"profile":"d6_revision_token_seal/1","state":"authorize_fresh","trustKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","possessionSignature":"<86-ASCII-unpadded-base64url>"}
~~~
selected current key 只有在完整 effective compromise union 没有任何 fact 指向该 key 时才能 `keep_current`。若 selected current key 已 compromised，则无 fresh 请求时结果必须为 `none`；只有 exact domain/profile 同时 eligible 且列入 freshAuthorizations 时，Core 才生成 fresh protected key 并产生 `authorize_fresh`。selected 为 `none` 时也只有 eligible 且显式请求才能 authorize_fresh。fresh outcome 永不复用 caller key。

每个 `authorize_fresh` outcome 的 possession 复用既有 domain `D6-Domain-Seal-Key-PoP/1`，并精确签下面这个闭合 body：
~~~json
{"workspaceRef":<parent resolve_conflict workspaceRef>,"revision":<parent revision>,"predecessor":<parent predecessor>,"decisionKey":<parent decisionKey>,"commitDomain":<outcome commitDomain>,"profile":"d6_revision_token_seal/1","trustKeyId":<outcome trustKeyId>,"algorithm":"ed25519","publicKey":<outcome publicKey>}
~~~
signature 精确为 `ASCII "D6-Domain-Seal-Key-PoP/1" || NUL || D3-CJ/3(body above)`。前四个字段在 fresh-key generation 前由父 resolve_conflict 固定；后五个字段来自该 exact outcome。`possessionSignature` 本身不进入 PoP body，父 `rootSignature` 也不进入，因此不存在 signature recursion。Core 先从 raw 32-byte public key 重算 trustKeyId，再签名。多个 fresh outcomes 分别签自己的 reconstructed body；把一个 outcome 的 possessionSignature 换到另一个 outcome 必须验签失败。全部 outcome PoP 固定后，Workspace root 再对只移除 rootSignature 的完整 resolve_conflict declaration 签名。receiver 从已接纳 parent declaration 与该 exact outcome 重构同一 PoP body，并验证同一 bytes。

`validate_historical` 对每个 effective compromise fact 使用其 `originActivationChangeId`：被 compromise key 的 artifact 只有在 seal ChangeId 可证明因果早于该原始 cut 时才有效，并发或更晚直接拒绝。resolution 前 branch 的历史验签继续沿该 branch 的 anchored bytes；resolver 不重写 losing declaration bytes，也不把旧 compromise 移到新 cut。

proposed policy 必须完整。若与 selected.policy 逐字相等，则 Policy revision 保持；否则必须是 selected.policy 的合法 checked successor 并通过 current policy_admin。结果 bundle 以 exact selected bundle 为基础，`authorizationRevision` checked +1；真正 policy-only resolution 保持 trustRevision，否则单条 resolve_conflict declaration 只让 trustRevision checked +1。policy ComponentImage.version 等于结果 authorizationRevision。

#### PreparedIntent/2, preview and unique submit path

wire3 的三个 D6-owned arm 全部复用既有 D6 v2 preparation carrier 与 final submit；wireVersion=3 只版本化本 prepare request/owner descriptor，不新建 ledger 或 commit protocol。OwnerInputBinding/2 使用 `protocolOwner="D6"`；`ownerKind` 与 `InputDescriptor.intentKind` 都固定为 `d6_conflict_resolution/2`。完整 canonical owner descriptor 为闭合 `ConflictResolutionInput/2`：
~~~json
{"kind":"d6_conflict_resolution_input","version":2,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/2>,"branchEvidence":[<ConflictResolutionBranchEvidence/1>...],"derivedPlan":<ConflictResolutionDerivedPlan/1>}
~~~
`ConflictResolutionBranchEvidence/1` exact：
~~~json
{"head":<ChangeId>,"changeRecordPin":<PinRef/2>,"completionProofPin":<PinRef/2>,"policyBundlePin":<PinRef/2|"absent">,"sourcePins":[{"entityRef":<EntityRef>,"sourcePin":<PinRef/2>,"metadataPin":<PinRef/2>}...]}
~~~
branchEvidence 必须完整覆盖 expectedKey.heads，按 ChangeId 排序；每个 pin 都是该 arm 实际消费的 exact protected/canonical evidence。sourcePins 按 EntityRef 排序，可空；缺证据不能靠省略表示。

`ConflictResolutionDerivedPlan/1` 是与 arm 匹配的闭合 union：
~~~text
{"kind":"source","ownerNodeRef":NodeRef,"baseSourcePin":PinRef/2,"headSourcePins":[{"head":ChangeId,"sourcePin":PinRef/2}...],"conflictedBefore":<SourceConflictBefore/1>,"versionBasis":<SourceConflictVersionBasis/1>,"proposedSourcePin":PinRef/2,"semanticPreviewPin":PinRef/2}
{"kind":"policy_bundle","selected":WorkspaceAuthorizationBundleAddress/1,"selectedBundlePin":PinRef/2,"effectiveCompromises":[TrustConflictCarry/1...],"inheritedCompromises":[TrustConflictCarry/1...],"outcomes":[TrustConflictOutcome/1...],"resultBundlePin":PinRef/2}
~~~
source_merge 的 proposedSourcePin 固定请求中的准确 source 内容；choose_source_head 固定所选 head 的准确 source bytes。两个 source 分支都保留原始 base/head 的 source pins、D2/local/complete 语义证据与 preview pin。conflictedBefore 是 §3.1.2 的真实已安装物理 before，绝不是 branch-current Observation；versionBasis 是完整且与 arm 匹配的 production-version basis。缺 physical before、缺 branch version、selected version 错误、arm/DecisionKey/audience/key 不匹配或复制 guard，都按既有 unavailable/invalid 边界拒绝，不能用相同 bytes/hash 或 I 修补。policy_bundle_choice 的 descriptor 固定所有 head 的 bundle/evidence pins、selected address/bundle、完整 effective compromise union、准确 inheritedCompromises、全部 outcomes、所有 fresh public key/PoP bytes，以及拟议 WorkspaceAuthorizationBundle/1 结果的准确 pin。受保护的 fresh private-key handle 由同一 installationPlan 绑定，绝不序列化进本 descriptor。

`OwnerInputBinding/2.canonicalDescriptorBytes` 精确等于 D3-CJ/3(ConflictResolutionInput/2)，pinRefs 是 branchEvidence/derivedPlan 中全部 pin 的排序唯一并集，包括 conflictedBefore/versionBasis 的 source 与 metadata pins。`InputDescriptor/2` 使用 guarantee=`managed_atomic`、frontierPolicy=`exact` 与 strict write protection。

source_merge/choose_source_head 使用 saveProfile=`complete`。在任何 author/source-semantic/D4/D5/D7 range read **之前**，Core 只能在 closed decode、最低 subject disclosure 以及已经授权、只用于判定 arm 的 conflict/control identity 基础上，按 owner profile 选择能够覆盖完整 required closure 的最小既有 ObservationScope。只有当预先声明的 complete owner profile 能证明所有适用 author dependency 只有单一 ownerNodeRef source，且绝不会读取 relation_incidence、query_scan、foreign Field/structure/member range 或其它跨对象依赖时，才可选择 `local_source`。若任一适用 complete obligation 可能要求更宽读取，必须预先选择最小足够的既有 wider profile，通常为 `workspace_constraints`。不能先看 author value 再扩/缩 scope。若实际 owner-required DependencyKey 超出已选 scope，必须在该越界读取和 planning 前走既有 owner_update_required/proof_unavailable；不新增 ObservationScope kind。冲突 ownerNodeRef 本身不得进入 sourceInputs：它的 `source` DependencyKey/controlInputs entry 只能通过 §3.1.2 before+basis 与本 exact owner guard 绑定。同一准备中实际读取的其它无冲突 source 仍进入普通 sourceInputs，并携真实 current Observation。

policy_bundle_choice 使用 saveProfile=`control_only`、与实际控制读取匹配的既有 control-only/workspace control scope，sourceInputs 为空。真实 `conflict_record`、`authorization` DependencyKey/2 entries，以及确实读取时同一固定十四类 union 内的其它 key，进入 controlInputs。Frontier 只能由 InputDescriptor.expectedFrontier + DependencyProof.baseFrontier + frontierPolicy 表示；`portable_frontier_state` 是 Policy capability，不是 DependencyKey。完整 policy/trust history 由 OwnerInputBinding/ConflictResolutionInput.branchEvidence、exact bundle/declaration pins 与 derivedPlan 携带；不存在 `policy-history` DependencyKey。不新增第十五/十六 key，也不允许真实依赖绕过 InputDescriptor。

immutable owner preview 为闭合 `ConflictResolutionPreview/1`：
~~~json
{"kind":"d6_conflict_resolution_preview","version":1,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/2>,"branchEvidenceDigest":"sha256:64-lowercase-hex","derivedPlan":<ConflictResolutionDerivedPlan/1>}
~~~
`branchEvidenceDigest` 精确为 `"sha256:" + lowercase_hex(SHA-256(D3-CJ/3(the complete branchEvidence array)))`。PreparedIntent/2.previewBinding 精确绑定该 preview record；它不是第二请求，commit 时不可编辑。pinDirectory 按 §3.6 包含全部 owner/dependency/preview pins。

三个 arm prepare 成功都返回既有 response，并固定 strict write protection：
~~~json
{"wireVersion":2,"kind":"d6_prepared_intent","planToken":<Token>,"semanticState":<SemanticState/1>,"writeProtection":"strict","inputRetentionState":"retained"}
~~~
planToken 是该准确 PreparedIntent/2 的既有 `d6_plan/2` token。最终提交唯一允许既有 `d6_commit_request/2`，不得携带 resolution/source/policy override。唯一 planning CAS 冻结完整 InputDescriptor/OwnerInputBinding、全部 branch pins、previewBinding、source 或 policy 的 derived plan、fresh handle association 与结果 bytes。source arm 若真实改变 source state，还必须冻结恰一个 SourceRevisionPlan/3；已证明 source-unchanged 的 source arm 不冻结 /3。唯一 final P seal 重验 expectedKey、current authorization、fence、step-6 conflicted-before/version-basis/H 及这些准确冻结的 dependencies，再原子安装 source 或结果 policy component 与 ConflictRecord resolution/supersession effect。禁止 one-stage resolver、第二次 submit、第二 ledger/CAS、CP4 或中间 trust prefix。

closed-decode 与 error 顺序继续使用既有 D6 surface：malformed wire3/Resolution2 在任何 state read 前为 invalid_request；披露/授权失败仍为 not_visible；branch/CP3/history evidence 不可用使用既有 state/proof/domain-unavailable 边界；expectedKey 改变为 conflict_changed。不新增 error union。§5 继续是唯一顺序：saved 在新业务检查前返回/恢复原 saved result；planned 恢复同一 PreparedIntent/descriptor/pins/preview/installation state，绝不重新解释不同 Resolution2；只有 unseen 才运行本 wire3 preparation。exact replay 只恢复同一个 retained plan association。改变 resolution/choice/source/expectedKey 不能复用原 planToken，必须重新走 unseen prepare；若已按分配的 OperationId 存在原 decision，则先执行 §5 saved/planned/unknown recovery，§8 继续保留该原始责任。

接收端接纳 policy resolution 前必须重验完整原 ConflictKey heads、每个 head CP3/policy image、selected bundle address、递归 effective compromise fold 与 canonical dedup/order、完整 inheritedCompromises、每份 fresh-key PoP、root signature、结果 bundle bytes/version，以及同 decision conflict-record transition。source resolution 的 receiver/publication 继续既有 source/CP3 路径并使用相同 frozen source pins 与 semantic proof。任一 mismatch 都是 unavailable/integrity conflict，不能 arrival-order repair。

旧 wireVersion=2/ConflictResolution/1 候选中的 `policy_choice:{policy}` 不再是当前 new-FA surface，且未激活；不虚构 migration shim。任何确能证明存在的真实历史 prepared/saved/planned/unknown record 继续只按原 decoder、request fingerprint、pins 与义务恢复。placement/lifecycle/identity conflict 继续导航到 D3 §10.1 typed resolver，并保持原单一 D3 submit/P decision。

source_merge/choose_source_head 仍重新读取全部 heads/base/current 权限，并重跑 D2/local/complete gates。新 head 使 expectedKey stale -> conflict_changed；旧点击绝不复用。
## 10. Policy/3

Policy/3 的 exact top-level仍为 version,revision,grants，version=3。Policy/1/2 保留原 decoder、原 bytes、原 capability/scope语义，不自动升级。grant仍为 subject,effect,scope,capabilities；deny优先、默认拒绝。原 Policy/2 的全部 capability逐字保留，不因本次DependencyProof扩展而改变。

Policy/3 新增无参数 capability 的闭集为：

这八项无参数能力分别是：用于注册副本的 replica_register、用于停用副本的 replica_retire、用于读取冲突记录的 conflict_read、用于进入冲突解决准备的 conflict_resolve、用于管理执行责任连续性与接管的 execution_custody_admin、用于观察可移植结构状态的 structure_state，用于观察完整 Frontier/2 的 portable_frontier_state，以及仅供当前认证主体在 workspace scope 管理自己有限 D10 工作区控制记录的 d10_control_self。

其中：
- d10_control_self 必须显式授予、默认拒绝；不授作者内容、资源或部署管理权，不由任何其它能力蕴含，也不蕴含其它能力。
- structure_state只允许观察portable parent/order、live/Trash结构范围以及为D3 StructureRange证明所需的结构状态；它不授source、Field、decision、lifecycle write或作者正文读取。
- portable_frontier_state只允许观察完整Frontier/2及其已验证连续head；它不证明payload materialization、Query/Registry完整性、source bytes、decision详情或execution responsibility。
- replica_register 只允许 §13 的专用 fresh-replica 副作用：mint 一条从未使用的 ReplicaEpoch，并在同一个 decision 中仅追加一条针对该 fresh replica CommitDomain、固定 `d6_revision_token_seal/1` profile 的 `authorize` declaration。它不授 policy_admin、root-key 管理、通用 add/rotate/revoke、其它 domain trust mutation、source 读写或 execution takeover。replica_retire 只按既有 gate 把 exact ReplicaRecord 标为 inactive；随后 `authorize_new_sign` 会在该 epoch 的 active-domain/fence 检查失败。retire 不自动追加 revoke declaration，也不删除历史授权；若存在 compromise/loss 需要 trust revoke，必须另走 root-authorized trust-management decision。
- conflict_read只允许读取已经通过subject disclosure的ConflictRecord；不授conflict source bytes、resolution或作者write。
- conflict_resolve只允许进入owner-specific resolution prepare；实际source/policy/D3写仍需要原write capability。
- execution_custody_admin 只管理执行责任的连续性与接管；不扩大 Money、approval、claim，也不授予作者来源写权限。

上述capability互不新增隐含关系，也不改变原capability矩阵。source_write deny继续阻止完整源修改；Field/body deny继续约束对应footprint；write不蕴含read。普通source-save ordinary profile仍须通过原source/body/Field/node-control矩阵、实际ObservationScope及本次需要的DependencyProof。structure_state、portable_frontier_state、conflict/replica capability都不是绕过内容授权或strong completeness的通行证。

structure_state 必须在读取隐藏parent/sibling/Trash成员前通过适用披露门；不得先扫描StructureRange再根据命中结果决定是否需要此权限。portable_frontier_state可以披露完整Frontier/2，但不能由“Frontier已知”推导source、DependencyProof或D7完整Query资格。

commit_sequence_state在Policy/3中保持CommitDomain-scoped metadata读取：request必须给完整CommitDomain，只公开该domainCommitSequence，不制造跨offline replica的全局顺序。Policy/2历史consumer继续其原Workspace-wide定义，只服务原saved/旧contract path，不迁移语义。

authorization 类型的 DependencyKey/2 所用 principalAudienceToken 仍来自受信主体、会话与委托映射，并把当前 Policy/3 版本、授权 generation 和原 ObservationScope 绑定到 proof；它不会向请求方披露完整 grant 表或被隐藏的 deny 项。Policy 变化会使相关 authorization stamp 按 §3.4 失效或推进，但 Policy revision 本身不会因此成为 permission token。

observed_only仍不是Policy capability。它只可按§4.1完整资格由受信人工在planning开始前显式选择并冻结；任何本节capability都不能扩大其适用范围，也不能让strict、structured、bulk/collection、Action、Automation、server checkpoint、Approval或Money路径降级为weak保护。
### 10.1 完整基础 Policy 与窄元数据生产者


Core受管policy恰为version,revision,grants；当前 Policy/3 的 version=3，revision为Counter。原 Policy/1/2 保留各自解码器和能力闭集，不自动升级；当前配置可显式安装 Policy/3。grant恰为subject,effect,scope,capabilities。subject是host/D10认证映射后的principal Token；外部请求不能声明当前subject。effect=allow|deny。scope三variant：{kind:"workspace"}、{kind:"subtree",root:NodeRef}、{kind:"ref_set",refs:[EntityRef...]}；refs非空、无重复、按完整Ref规范bytes排序，全部属于本policy Workspace，不要求实体已存在。capabilities为非空、无重复capability union数组。

Field capability恰为{kind:"field_read"|"field_write",fieldIds:[FieldId...]}，集合非空、无重复，按规范FieldId排序。其他capability恰为{kind:K}，固定 S 的基础 K 闭集为：workspace_state|entity_state|locator_state|source_read|source_write|body_write|node_control|node_create|resource_read|resource_write|annotation_read|annotation_write|lifecycle|registry_admin|binding_admin|policy_admin|export|repair|audit|source_envelope_state|commit_sequence_state。当前 §10 明确列出的八项新能力仅在 Policy/3 中增加，其中 d10_control_self 只允许 workspace scope。Field集合不隐含body/title/Facet或control写权。角色展开只能产生这些同形grants；不另引role优先级。workspace级操作只接受workspace scope，不能用subtree的policy_admin管理全Workspace。

workspace_state/commit_sequence_state只允许workspace scope；entity_state/locator_state/source_envelope_state允许workspace或ref_set，禁止subtree。ref_set只配这三个明确state capability，不配Field或其它capability；包含不适用scope/capability的grant由decoder整体拒绝。其授权只看当前principal、policy及请求的完整Ref，在查entity存在、lifecycle、parent、locator或index之前完成，同一ref-domain一致覆盖live/trashed/tombstoned/never-known。workspace allow可被exact-ref deny遮蔽，所以parent隐藏/spouse可见不依赖当前树或对象存在。最小tombstone不得为授权而增存parent/旧ACL。以下按当前parent匹配的subtree规则仅适用于已经通过适用state-disclosure闸门之后的内容/Field/效果权限，不用于定义existence visibility。按子树动态隐去存在仍不属于该首版state grant。

匹配按当前受信principal、Workspace、完整Ref或适用的当前parent关系；deny优先于全部allow，默认拒绝。source_read/source_write是完整source能力，但显式某Field read deny阻止包含该Field的全源披露，write deny阻止实际修改该Field的proposal；原始源输出必须证明全部内容可读，Field API只投影允许事实。title/coreKind/Facet声明改变须按下面footprint矩阵取得资格并通过适用typed gates；body_write不能修改这些结构。resource/annotation还需完整owner scope，状态披露与内容读取分别判断。export需要export及其所有输入读取权。policy修改要求旧policy下policy_admin，并在当前 WorkspaceAuthorizationBundle/1 policy component 中保存完整 closed Policy/3；同一 portable transition checked 增加 Policy.revision、auth generation 与 bundle.authorizationRevision，同时逐字保留 trustRoot/trustRevision/trustDeclarations，并只由原唯一 P/CP3 链 seal/publish；撤销自身管理员权限是显式效果，可提交后立即生效。D8可设计防误操作preview，但不引入隐藏的永不可撤销管理员。

修改source、control或policy后Core计算实际effect并更新受影响授权范围generation；move需old/new域资格和D3现有规则。未授权subject不能通过计数、错误详因、缓存、效果清单或subscription得知隐藏facts。

能力矩阵是唯一蕴含规则，未列蕴含均不存在；匹配的explicit deny压过直接或派生allow：

| 实际footprint | 所需能力 | deny裁决 |
|---|---|---|
| 已证明仅普通body | body_write或source_write | body_write deny或source_write deny中实际适用的源范围禁令均阻止该效果 |
| 某Field的值/Entry note/key | 该Field的field_write或source_write | 该Field field_write deny优先；仍执行D4各具体变换门禁 |
| 普通文档title/subtitle元数据 | source_write | source_write deny阻止，不由body_write授予 |
| coreKind/declared Facets等影响Node分类的声明 | source_write且node_control | 任一deny均阻止；不能凭整源写越过Node control禁令 |
| parent/ordinal/lifecycle | D3请求以及node_control/lifecycle等其适用授权 | D6 source edit禁止产生此类effects，source_write不蕴含它们 |
| Resource/Annotation现有payload | resource_write或annotation_write | 不被source_write蕴含；新identity仍D3 |
| 无法完整分类的原字节差异 | source_write且证明没有任何适用细项deny被隐藏 | 无法证明则拒绝；不以source_write跳过typed检查 |

source_write deny表示该范围完整源禁止修改，不能绕由field/body细项allow；field/body deny仅限制相应细项，但一份包含其修改的整源proposal也被阻止。未改动的隐藏Field可由Core原样保留；只有 §3.3 可证明结果与其独立的owner_fields路径才可继续局部Field edit。不能以内部保留和错误遮蔽替代潜在结果观察资格。

write不蕴含read。D6正常提交/重放回执还要求其sourceVersions的entity-state披露资格；效果清单中before/after值分别按对应read能力交付，缺权返回遮蔽结果而非原文。受信内部footprint不受此输出裁剪影响。D3回执披露仍按D3原合同，不以此表新增D3阶段。


同一 §1.5 真实 SourceObservation 生产者可在原 entity_state、source_envelope_state 及必要 Field、Registry 门通过后服务 D7 窄元数据读取。内部完整源保留和聚合验证不等于必须向主体披露全部源字节。返回的 SourceVersionRef 只选择该真实同切面观察，不授予源、正文、读取或写入权限。信封资格无效或不可用时不产生当前源 token。后续每次使用仍检查真实当前授权及观察，不建立第二个 D7 签发者或新依赖键。

### 10.2 Issuer 策略与完整工作区引导


首次建立存储与创建Workspace不是同一授权。host/D10认证只给主体身份，不自动给issuer控制权限，更不赋予既有Workspace读写权。IssuerControlPolicy由现存issuer的独立控制域拥有，exact为{kind:"d6_issuer_control_policy",wireVersion:1,issuerAuthorityInstanceId,revision,grants,bootstrapProfile}。revision为Counter；grant exact {subject,effect,capabilities}，subject为该issuer控制域的认证principal Token，effect为allow/deny，capabilities为非空、sorted unique的allocate_workspace|administer_issuer数组。只作用于这个issuer，不接受Workspace/subtree/ref_set scope。deny优先、默认拒绝；administer_issuer不自动蕴含allocate_workspace，两个能力都不蕴含任何既有source读取或target激活后的能力。

**信任起点与管理。** 本地首次issuer只能在用户明确建立的全新空存储域，由受信host将当前OS登录身份映射为初始issuer principal；Server由已有部署operator的受信配置指定已认证身份，不能由普通HTTP请求自报subject或因首次连接抢占管理员。Core在一次初始化事务内mint issuer AuthorityInstanceId、建立连续性/fence控制、issuer policy revision=1及其认证映射；初始issuer principal被明确授予上述两种可撤销能力。初始配置必须提供完整bootstrapProfile及可信Registry seed。marker/authority/ledger/配置存在、缺损、冲突或旧备份不能再走“空域”初始化；失败保持未初始化，不能覆盖或补造旧grant。恢复、continue、failover、repair都不得触发首次issuer规则。

issuer policy变更是这个issuer控制域自己的显式管理操作：仅当前administer_issuer可准备/执行/重放，绑定完整旧revision及新policy/profile，当前认证/delegation和fence在同一控制事务CAS；实际改变checked-add revision，MAX拒绝。以issuer内的管理operation key保存不可变请求/结果，same-key重放不重复改变，different input conflict；这个控制账本不是D3 Workspace-local OperationId或跨Workspace全局注册表。issuer管理的host/CLI/RPC运输须在该实施入口冻结前提供closed schema，当前D6 Workspace提交API不能冒充此入口；本节已经冻结主体/权限、幂等、CAS和效果的完整语义，不授权自由代码管理回调。撤销最后一个issuer管理员或其allocate能力可以是显式效果，不能靠重新open恢复初始超级管理员；host上重新认证只恢复同一身份，不新增grant。

bootstrapProfile当前新签发exact为{kind:"d6_bootstrap_profile",wireVersion:3,profileRevision,registrySeedBinding,newSeriesMultiplicity,initialPeriodScope}；profileRevision为Counter，registrySeedBinding为D4 RegistryBinding/1，指向同一受保护、完整且经D4 trust root验证的不可变seed，newSeriesMultiplicity为显式unique或many，initialPeriodScope在本版固定为workspace；都不能缺省或从请求/冲突结果猜测。当前profile的固定policy规则是：新target只为本次经issuer认证的创建主体建立target-authority-local principal映射；不复制source principal Tokens、credentials、delegation或source ACL。profile/3 的 initialPolicy.version=3，其初始 Workspace grant 显式包含 §10.1 的 21 项固定基础能力，以及 replica_register、replica_retire、conflict_read、conflict_resolve、execution_custody_admin、structure_state、portable_frontier_state、d10_control_self，再加 target Registry 所列全部 FieldId 的 field_read/field_write（集合为空时不生成空Field capability）。deny为空。历史 profile/1 固定使用 Policy/1 和原非 Field 闭集，禁止补 source_envelope_state/commit_sequence_state；历史 profile/2 固定使用 Policy/2 及上述 21 项基础能力，禁止补本轮八项新能力；既有family保持原副本，只有显式issuer profile更新才影响新family。这是明确的首份owner policy，不是永久超级管理员。后续按普通Workspace policy修改可完全撤销。新增Registry Field不自动获得新Field grant，仍按当前policy与普通管理修改；source_read的既有完整源规则不变。

**issue/replacement绑定。** D3 A2在任何family lookup前只按当前issuer principal的allocate_workspace判断；它包含issue、replace和该issuer创建/fork目标的准入，故replace不需要先读family才能知道mode。D3 A3/A4仍分别处理issuer不可用/可达完整性，A5–A11及其先后不改。issue成功在原family/proposal/custody事务内固定一份受保护bootstrap profile副本、真实issuer principal/audience映射、该targetAuthority的初始principal映射，以及Registry seed完整输入；profile不变更D3 proposal/request shape或认证framing。已创建family永不重新套用“最新profile”。replacement继承同一family的固定profile和同一已认证人的身份，但为新target建立独立target principal映射并随原规则retire旧target控制映射；不能让一个旧target Token跨authority取得权限。无成功issue不得凭replace创建profile/family。配置更新只影响以后新family，旧family能否操作仍由当前issuer授权和原D3 family/custody gate决定。

**正式请求与唯一bootstrap计划。** D3 stage3先检查当前issuer allocate_workspace；fork还检查source的当前完整观察及source/export授权，stage4/P1/P2/TL/stage5次序不变。此时prepared_workspace只是公开mode与外层角色导出的临时资格，不能要求尚未在stage12生成的WorkspaceBootstrapPlan，也不提前认证family。target尚未激活，不能先读它的policy/exists/ledger来验证当前source_write或policy_admin。原proposal门与完整计划验证通过后，受权bootstrap资格仅允许验证和发布该proposal的有限全fresh target闭包；不授权任何既有Workspace改写。普通body/Facet/关系所需的source与域约束仍须在原stage14/15完整验证，不能以初始化为由接受invalid source。

WorkspaceBootstrapPlan/3 是 D3 原 plan 内 D6 拥有的受保护控制部分，新增 exact 成员 trustGenesis；其语义成员为 kind,wireVersion,operationId,proposalId,issuerAuthorityInstanceId,targetWorkspaceRef,targetAuthorityInstanceId,profile,creatorBinding,targetRegistry,initialPolicy,trustGenesis,initialSeriesConfigurations,periodScopeBindings，kind=d6_workspace_bootstrap_plan、wireVersion=3。trustGenesis 的闭合形状如下：

    {kind:"d6_workspace_trust_genesis",version:1,rootDeclaration:WorkspaceTrustRootDeclaration/1,initialDomainDeclaration:WorkspaceTrustDeclaration/1}

其余原成员责任不变。

本对象仍不是额外可编辑请求或第二 decision。已接纳 host secure store 生成 fresh Workspace root keypair 与 fresh target-server domain keypair；plan 只保存 public declarations 和受保护 handle 关联。`rootDeclaration.establishmentDecisionKey` 与 `initialDomainDeclaration.decisionKey` 都逐字等于原 create/fork DecisionKey。initial declaration 固定 revision=1/action=authorize，exact domain 是目标 server CommitDomain W/B，profile=d6_revision_token_seal/1。root self-signature、root signature 与 domain-key PoP 必须在 plan 赢得前通过。

既有 policy component after-image 是 authorizationRevision=1、trustRevision=1 且恰含该 root/declaration 的 WorkspaceAuthorizationBundle/1。唯一 bootstrap P seal 原子提交原 decision/ChangeId、此 bundle、target activation/custody handoff 及全部原 bootstrap component。只有同一个 bootstrap decision 的 managed after 可以在其 activation ChangeId 尚未可能进入 frontierBefore 时由 authorize_new_sign 消费已验证 same-P trustGenesis；这是唯一 genesis 例外。retry、registration、rotation、后续 author seal 或其它 DecisionKey 都不能使用。接收端先要求 exact anchored root，再验证 root/declaration/PoP、bootstrap CP3 policy after-image 与每个 source artifact；因此 fresh W 不要求预先已有关于自身的 trust history。

当前 profile/3 产生 Plan/3。Plan/2 是未激活候选前身，不为其虚构 migration/dual-write。真实已经签发的 profile/1 或 /2 family 若可证明存在，仍保留实际 Plan/1 合同和 saved/planned/unknown 恢复。当前 D7 transport 只为真实 Plan/1 history 与当前 Plan/3 解码；绝不把 Plan/1 或 Plan/2 重标 /3，也不向旧 bytes 注入 trust genesis。

**Registry和Calendar初始状态。** create target Registry从固定seed经过D4一次性registry_bootstrap建立，Workspace/user owner与target authority正确绑定；fork使用完整source Registry输入和同一trust-root规则为新target重新认证/绑定，source和target Binding分别保存，不能把source context直接当target。无法证明定义可移植、owner binding、规则或完整解释时原操作在planning前按适用D3/D4门拒绝，不能偷偷安装网络贡献或改作者FieldId。

initialSeriesConfigurations每项exact为{seriesScope,multiplicity,revision}，seriesScope逐字复用§3.4 的完整闭合对象，multiplicity为unique/many，revision=1；按seriesScope canonical bytes排序、唯一。series和periodKey来自完整prepared作者Entry，scope则是独立控制输入，不能声称calendar/period Entry中本来含有scope。periodScopeBindings项exact为{nodeRef,scope,revision}，revision=1，scope复用D4闭集；按NodeRef排序、每个有效period Node恰一项。它只保存D4范围选择这一必要控制事实，不复制series/periodKey/Field值、不形成第二作者源或用户可写的新Field。普通已激活Workspace的period创建/修改按本节普通范围规则确定选择规则在同一源plan中绑定显式scope，持久保存该控制选择；删除period或purge其Node同时移除该Node的binding，Trash保持，scope改变按当前Workspace policy_admin授权且改变配置/范围控制版本。普通Field修改不隐式改变scope。scope自身若为Node，必须在同Workspace指向完整已证明live/trashed Node；D6控制inbound检查阻止把仍被scope binding/config使用的Node直接purge，必须先按本节普通范围规则的closed scope变更与空配置删除意图解除依赖，不能在purge中猜重绑定。

create_workspace为所有prepared period按该family的initialPeriodScope显式绑定target workspace scope，并为每个不同series建立配置，采用已固定newSeriesMultiplicity；不是普通create缺配置时的隐式默认，也不冒充Node scope初始化接口。fork把source cut中当前SeriesScopeConfigurations及periodScopeBindings完整带入，保留unique/many；Node主体与node scope都按完整D3 Node map重绑定，Workspace scope换成target WorkspaceId，再按target Registry/policy验证。不得把source unique改成profile many。任一配置或binding不能完整解释/重绑定时整fork在planning前失败，禁止省略、保留foreign scope或靠第二次管理提交补洞。所有prepared period候选按其准确binding恰受一个配置覆盖，不允许漏配或从是否冲突选择multiplicity。

target尚无作者事实，但不能以数据库空表证明最终范围为空：D4唯一性校验读取该D3 proposal的完整prepared结果集合和全部scope，观察/当前range版本来自原target负inventory/custody及allocation依赖，全部同cut。unique按每个完整period key分别比较，many允许多个Nodes；与普通已激活Workspace相同。初始配置与policy/Registry、全部源、authority activation、ledger/receipt、custody移交及失效版本在同一个author commit原子发布。初始源revision仍为1；policy/config初始化不另增源版本。没有“先激活再补管理员/配置”的可观察窗口。

**普通管理、重放与恢复。** 普通 policy_admin 修改以及原配置意图只作用于已经激活的Workspace；不能在未激活target预先运行D6 management decision。本节bootstrap是一次D3 create/fork的确定控制初始化分支，激活后不再适用。claim/replace race、recorded rejection、planning、terminal failure、burn、custody和D3原receipt保持原规则；失败不产生半policy或可登录的半target。source或target Registry/profile派生证据不能恢复时保持原availability/恢复状态，不重选“最新seed”。已committed的原create/fork重放只按D3当前issuer/source授权及原proposal返回保存bytes，不再次执行policy/config初始化，更不覆盖用户后改policy；控制详情/target内容交付另按当前target权限，失去target权限不能借旧bootstrap结果读内容。continue/failover保留当前policy、principal映射和全部配置及其版本；换holder不恢复creator权限。fork的新grant不会授予对source的任何能力。

profile创建/更新时必须证明Registry seed是D4允许的无退役/迁移历史的完整bootstrap-eligible种子，且target owner重绑定后仍须真实trust-root认证，不能只换WorkspaceId。fork不是把source Registry重新当无历史seed bootstrap：保留source累计语义历史、退役/迁移约束和必需RegistryEvolutionProof，重新认证其target Workspace/user owner与target Binding；定义/历史/证明不可移植或不可验证时在原planning前拒绝，绝不清空历史来通过bootstrap。


IssuerControlPolicy 顶层成员和 wireVersion=1 不变，只按嵌套 bootstrapProfile 的显式 wireVersion 分派；新 /3 组合先过 D1 版本资格。真实已保存 BootstrapPlan/1 继续原 profile、policy、字节与恢复，不能改标为 /2。存在旧解码器不等于证明原型曾部署。



CalendarPeriodScopeBinding是D6控制事实，绑定一个具有有效calendar/period Entry的NodeRef到准确D4 scope；series与periodKey仍每次从唯一完整作者源解释，不能另存可编辑副本。每Node绑定的控制revision属于独立范围版本，首次1，实际scope/有无绑定变化checked-add1，删除后保留仅用于防ABA的控制版本；MAX时整体拒绝。它不进入D3最小tombstone，也不保留已purge源/分类。若没有有效period Entry则没有活动binding；Trash期间保留绑定及原D4范围归属。

**确定选择。** 普通新period（D3 create、ordinary-format import，或既有Node首次增加period）固定选择其Workspace scope；这条规范规则必须作为显式scope写入原plan，不能从当前冲突或index结果决定。所选series+scope配置必须已通过显式管理建立，不默认many，不在普通源写中自动建配置。对已有period的源编辑保留准确scope，即使series或periodKey改变，也必须读取新旧完整范围和新配置；删除period的源变换同事务移除自身binding。显式改变scope使用原唯一受管 scope 控制意图，不偷偷改Field或Node identity。

managed Node copy按原source cut携带每个copied period的控制scope：workspace改为target Workspace；node scope按D3完整Node map重绑定，same-Workspace map外合法live/trashed Ref可保留；跨Workspace map外或不可解释scope使整copy失败，不能改成workspace逃过原约束。普通artifact不携带可凭空信任的控制binding，按上述新period规则；完整fork按本节同时重绑定配置与binding。copy/create不改变已有配置。任一适用D3 mode须在原stage14/15验证这些确定规则与完整范围，仍不新增wire字段；D6生成的控制effects由原request、真实source cut和当前受管配置唯一派生并在plan固定。


受管复制的新范围配置： 上述“copy不改变已有配置”不禁止一个必要且封闭的新配置初始化分支：若node scope的原Node恰在本次D3 Node map中，其target scope Node是已证明的fresh prepared对象，Core必须将source cut的对应完整配置重绑定到该fresh scope，保留multiplicity及完整D4 policy含义，作为原copy计划的控制effects。不能预先要求尚不存在的新Node live或要求调用第二次管理提交。只有原映射scope的负inventory/配置负范围、原source配置及所有prepared period候选都完整可证，且target Registry可验证其policy时才可准备；source配置缺失/不可迁移、target配置碰撞或prepared unique冲突均整copy拒绝。首份配置revision=1、binding=1；source配置历史版本作为原plan读依赖，target新control世代独立。该分支仅建立本次映射fresh scope所需的配置，不改既有target配置、不为workspace scope或map外既有scope隐式建配置；后者缺配置仍须显式管理。所有fresh scope Node的状态按D3受保护prepared-origin和原reservation验证，提交后必须live/trashed；全部源/配置/binding及原receipt一次commit。配置派生由原copy输入、map、真实source cut唯一决定，不是可编辑companion或新增D3 wire。fullfork仍按本节完整处理所有source配置。

## 11. BudgetBinding/1 与 pin capacity

BudgetBinding/1原成员与数值域保留。v2 PreparedIntent另绑定 PinBudget/1：

~~~json
{"version":1,"maxRecoveryBytes":<Counter>,"maxConflictBytes":<Counter>,"maxPreviewBytes":<Counter>,"maxImportExportBytes":<Counter>,"maxHistoryBytes":<Counter>}
~~~

0表示该类不允许新增，不表示无限。实际额度取 request/policy/host min。每次 pin allocation前 checked-add并持久 reservation；同 plan所有attempt共享counter。work units仍先 durable charge后执行，crash不退款。temporary staging与 protected pins分别计费，不把 worker局部视图当总占用。

protected last-reference pin不能因 TTL/preview expiry被删。容量不足：新 prepare/install返回 budget_exceeded或 planned paused_capacity；不能通过删除 planned/unknown/conflict last-reference evidence继续。

## 12. Result/ByteHandle 与 index 消费

D6 ResultHandle/ResultCursor 与 Resource ByteHandle/ByteRead 的原 closed wire1、token/tag、saved handle bytes、授权顺序、预算、TTL、pin 与错误优先级继续按原协议解释。本节不修改这些 wire1 shape，也不把“存在历史 decoder”机械等同为所有旧原型都已作为当前新兼容面激活。已经保存并仍满足其原授权、pin、clock 与 continuity 责任的 handle 继续按原 bytes 工作；合法失效、过期或已回收的记录不能用当前文件、相同 digest 或新 producer proof 复活。

在本合同定义的新文件型生产路径中，任何未来新 consumer version 若要把结果或快照作为当前、完整或可用于 Action 的证据，受保护 immutable pin/record 必须绑定实际读取的完整 SourceObservation/1，以及其中完整生产 SourceVersion/2、生产 CommitDomain 与当前 observerDomain/observationEpoch/FileObjectBinding/evidence pins；不同生产域中相同裸 revision 数字从不表示同一 current source。需要范围或控制完整性的结果还必须绑定实际 closed DependencyKey/2、对应 DependencyProof/2 的 epoch/revision 与 evidence pins，以及 owner 已证明的正负范围、授权、Registry/control 和其它真实依赖。SourceVersionRef/1 只选择当前完整 Observation；SourceRevisionPlan/1、拟议 SourceStamp/1 或未 seal 的 revision token 不能冒充结果的已提交 current source。

Frontier/2 只表示各 CommitDomain 已验证连续 sealed 的因果前缀，不证明 Query 全集、Registry 完整、payload 已下载或 placeholder 已物化。building/partial index、index miss、placeholder、unknown decoder、physical/D2 invalid、I/O failure、范围连续性不明都不能被编码为完整 empty-success。exact/NFC/regex 候选索引必须证明对声明范围无漏召回；不能证明时就按 owner 规则回读并补扫真实 source，完整扫描仍必须经过当前授权和完整范围证明。Derived Index 只缓存候选，不签发完整性事实。

`scope_dependencies` 也不产生通用的 result/handle 保活例外。只有当对应 owner 的 consumer contract 明确允许，并且从原 expected/base Frontier 到当前 cut 的扩展具有完整连续 sealed 证据、原 source/control/authorization/正负范围 DependencyProof 与所有 pins/selector/Registry 依赖均未变化时，才可按该 owner 继续使用原计划或结果。真实 SourceObservation/token、authorization generation、query selector、DependencyKey stamp、pin、Registry/rule、membership/negative range、control 或观察连续性发生变化时，仍按 owner 规定 reset/reprepare；未知是否无关时不得继续。D7 的完整结果仍保留其整体当前授权世代与完整依赖 reset 规则，D6 不用 coarse Frontier 的“无关”判断覆盖 D7 owner。

Resource ByteHandle 保留其原有更窄的 immutable-snapshot 例外边界，而不推广给 ResultHandle 或 current-source evidence：handle 在签发时固定已提交 Resource、完整 ResourceRef/owner、原 SourceVersion 与 resourceRevisionToken、immutable cut、descriptor、pin、有限期限/clock、lifecycle/continuity 交付世代及预算。Resource 从 R1 更新到 R2 本身不把已签发且仍合格的 H 重绑或 reset，H 继续读取 pin 住的 R1 并回显 R1 token；这也不表示 R1 重新成为 current。每次读取仍先按原顺序重新通过当前主体的 D3 披露与完整 resource_read/owner-scope 授权。权限或 holder 变化本身不自动使快照失效：若能证明合法 holder/authority 连续接管，并完整保留原 handle 记录、clock/continuity 状态与 pin，则可继续同一 H；无法证明原 record、clock 或 pin 连续性时必须停止读取，并按原 wire1 的 byte_unavailable 处理。这个连续接管例外不覆盖 ResourceRef/owner lifecycle 变化，也不覆盖已经证明不再延续原快照的 authority/continuity 世代；这些情况仍使交付世代失效或要求 reset。expiry、reset、range、未知 clock/pin/backend 连续性、预算以及最终交付门继续完全遵守原 wire1 的既定检查与错误优先级；相同 bytes/digest、Trash 后 restore、重新授权或同名新 Resource 都不能复活已失效或过期的 handle。这个例外只保护已经固定并 pin 住的历史 Resource 快照，不能用来忽略 D7 Result 的授权、selector、dependency、完整 cut 或整体授权世代 reset，也不能推广到 Action 或任意 source/proof 变化。

D7 完整 Query/Result 的语言和 consumer 合同仍由 D7 全套 owner 共同冻结：Query Algebra、Value/CEL、View、Narrow Field、Definition Transfer、Preview/Effects、Execution/Action、Prepared Action Binding、Scenario Dispositions、Terminology Lexicon/Registry 与 Implementation Impact/Test Outline 都必须实际生成协调后像。D6 只提供本文件已经闭合的 SourceObservation、DependencyProof、pins、授权和运输生产者；它不通过 free JSON、新隐式 wire 或只改 Prepared 一页代替这些 owner。缺少实际 D7 consumer 时，相应新 complete Result/Action path 继续 owner_update_required/proof_unavailable，而不是由本节臆造 reset 字段或 handle 版本。

需要 complete Query cut、all_result、post-query、bulk/collection、强 Action 或 Automation 的路径必须持有其 D7 owner 要求的完整当前正负证据，不能借 partial exploration、semantic_pending、局部 ordinary success 或 ByteHandle 快照降门。反向同样成立：只需要当前获权 source/Resource 的 ordinary read/edit 或明确局部 projection，不因为某个无关 complete Action/全 Workspace 证明尚未形成而被无条件阻断；它们继续按自己的 SourceObservation、局部 DependencyProof、授权与原 handle 合同工作。
## 13. Replica registration/read

replica registration prepare request：

~~~json
{"wireVersion":2,"kind":"d6_replica_register_prepare","workspaceRef":<WorkspaceRef>,"expectedReplicaRegistryRevision":<Counter>,"displayLabel":<text>,"budget":<BudgetBinding>}
~~~

displayLabel只作受权显示，不是identity/path；不得为空，UTF-8 ≤256 bytes。要求 workspace scope replica_register，且 current WorkspaceAuthorizationBundle/Registry 可验证；不接管 authority。PreparedIntent/2 mint 从未使用的新 ReplicaEpoch。通过受信 registration pairing channel，joining host 上的 Core 生成 fresh host-protected Ed25519 DomainSealKeyHandle，精确绑定 {kind:"replica",workspaceRef,replicaEpoch}+d6_revision_token_seal/1；只把受保护 enrollment/PoP 交给 registering authority，普通 caller JSON 绝不能提供 authority key。winning plan 同时冻结 expectedReplicaRegistryRevision 与 expected trustRevision。唯一原 P seal 以同一 DecisionKey/ChangeId 在既有 replica_registry component 写 active ReplicaRecord，并在既有 policy/WorkspaceAuthorizationBundle component 追加 root-signed authorize declaration。只有 committed transition 被接纳后 joining handle 才 staged→usable。接收端只有在同一 CP3 证明两个 component transition、declaration.decisionKey 匹配、anchored root/root signature 与新-key PoP 全通过且无 policy/trust conflict 时，才接纳新 CommitDomain。只复制文件或 public declaration 而没有受保护 handle，不取得签名权。host-private bootstrap domain 可以执行注册，但成功 seal 前不暴露为可复用 CommitDomain。

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
第五组用于批准或停止资格竞争：approval_unavailable | execution_stopped。

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

approval_unavailable 与 execution_stopped 按 §4.4 使用 preflight，保存重放先于这两个门。planned 保留原计划、真实安装状态和预留，只有原已证明终态中止条件才可释放计次；错误 disposition 自身不证明已经回滚。

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

载荷现由实际 D10 Control Contract §8.2、§16.1 闭合：approvalUses 为 ApprovalUse/1 数组，claims 为 D10ExecutionClaims/1，moneyLineage 为 D10MoneyResponsibility/1，externalUnknowns 为 D10ExternalResponsibility/1 数组，stopState 为 D10StopResponsibility/1 数组。status 只有 active、paused、transferring 三态，不保留自由 JSON 或未来待定 schema。载荷是同一执行控制切面上原 owner 记录的完整保护视图，不是独立可写的第二账户、认领或批准账本。每次原责任变化都在同一真实事务使旧视图失效，并检查推进本记录修订；新增认领、发送或债务被遗漏时，旧快照不能仍为当前。完整视图不可用只暂停执行，不阻止普通源访问。

    ExecutionContinuityProof/1 =
        {kind:"initial", storeIncarnation:Uuid,
         inventoryPin:PinRef/2, birthProofToken:Token}
      | {kind:"checkpoint", storeIncarnation:Uuid,
         inventoryPin:PinRef/2, barrierToken:Token}
      | {kind:"handoff", storeIncarnation:Uuid,
         inventoryPin:PinRef/2, fromHolder:ExecutionHolder,
         toHolder:ExecutionHolder, oldRevision:Counter,
         barrierToken:Token, oldHolderFenceToken:Token}

ExecutionHolder 恰为上文 holder 联合，lastContinuityProof 使用这个闭合类型。inventoryPin 使用 artifact/recovery，严格解码 D10ExecutionInventory/1，其字节为 UTF-8 的 D6-Execution-Inventory/1、一个 NUL，再接 D3-CJ/3 规范字节。完整工作区及五项载荷成员等于本记录，stopCapacity 保留真实安全容量分配。清单包括准确原请求、准备及历史图像、有意义的空范围证明、运行准入、全部订阅和发生状态、计次使用、所有费用层投影，以及原外部发送与未知责任。共享部署账户仍在真实 owner 并保留连续引用，不能作为虚构零余额转交。

三个 token 都是 Core 保护句柄，不是调用方断言、可互换能力票据或未解释证据块。birthProofToken 在实际存储最初创建屏障处绑定本从未使用过的执行域、工作区、存储身份、首持有者及完整已证空清单。既有或损坏存储、备份不能重新签发出生证明。barrierToken 绑定准确执行域、持有者、记录修订及完整清单引用；真实耐久存储屏障先冻结新准入、planning、发送和认领，并保留进行中安装责任。oldHolderFenceToken 另把同一屏障与清单、准确前后持有者绑定到后端已证明的旧执行不可逆排除。Core 必须按已接纳后端合同验证真实受信备份或转交及隔离机制，才能签发句柄；不支持或不能证明排除时只能不可用，不能签名猜测。保护映射及必要原证据跨恢复保留，不能从索引或可移植文件重建。摘要、提供者自称同步或文件复制都不能产生它们。

检查点在该屏障下证明完整正向及负向记录和范围成员。转交要求当前 execution_custody_admin、完整原清单及引用、保留的存储身份和控制引用、准确旧修订与持有者、耐久转交，并在启用新持有者前真实隔离旧持有者。新记录修订检查旧值加一；转交不重置额度、身份、发生项、时钟证据或未知。转交期间保持 transferring 或 paused，任一持有者均须证明自身准确资格后才能启动新工作。失败保留原责任，不能创建空替代域。初始、检查点及转交引用遵守最后引用保留。历史责任记录继续原解码器，不能借新 schema 补造缺失连续性。

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

历史合同按记录实际保存时的版本和 owner 分流，不按当前字段名猜测或批量迁移。D6 wire1 commit/receipt/error、Policy/1/2、SourceVersion/1、Frontier/1、InstallationNotice/1、ContentCompletionProof/1，以及本候选正文已经明确保留的 ContentCompletionProof/2、ConflictRecord/1，旧 Token tag、旧 PreparedIntent、d6d/d6r/d6a revision-token profile 与 DocumentRevision/ResourceRevision/AnnotationRevision 原 opaque decoder，原 D3 primary receipt 及其 D6 companion bytes、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1，均继续按各自原 bytes、fingerprint、授权、连续性、pin retention 与恢复规则处理。任何实际已保存并被旧 decision 或 pin 引用的 SourceObservation/1、SourceVersionRef/1 及其受保护 token binding 同样只按生成它们的原合同解析；新当前观察或新 token 不会把旧 binding 重新签名。存在 decoder 或候选文本本身不证明某个历史 prototype 曾部署或全部 active；但一旦某个 saved、planned 或 unknown 记录按其原合同真实存在，其既有恢复、保留和非重复执行义务不能因缺少部署证据而取消。

当前新生产路径只适用于明确采用本候选新合同的记录。SourceVersion/2 继续是完整 managed/external 生产版本 union：managed 保留生产 CommitDomain、observationEpoch、revision、ChangeId，external 保留生产 CommitDomain、observationEpoch、externalSequence 且没有 managed revision/ChangeId；当前 SourceObservation/1 另以 operation CommitDomain 作为 observerDomain，SourceVersionRef/1.sourceToken 只选择该完整当前观察。SourceRevisionPlan/1 与 SourceStamp/1 只冻结拟议 managed after 的版本依据，RevisionTokenBinding/2 的 d6_source_revision/2 只通过受保护 binding 解析；seal 后才由真实 ChangeId 形成 managed SourceVersion/2。ContentCompletionProof/3 运输真实生产 SourceVersion before/after，ConflictRecord/2 使用 Frontier/2。D3 Locator 的原 opaque token member、D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence 和 D5 revision-bound locator 不因这些生产者改形。

历史与当前版本之间明确禁止以下重解释：
- 不把旧 request 重编码成 wire2，不给旧 receipt 补 CommitDomain、ChangeId、SourceVersion/2 或新 companion 字段；
- ContentCompletionProof/2 的 sourceChanges 继续使用其原 SourceVersionRef/1|absent 形状，不迁移为 ContentCompletionProof/3 的生产 SourceVersion/2，也不从旧 sourceToken 反推不存在的生产版本字段；
- ConflictRecord/1 的 createdAtFrontier 继续只按 Frontier/1 解释，不重编码成 ConflictRecord/2；ConflictKey/1、ConflictId 与 `D6-ConflictKey/1` hash domain 保持原样；
- 旧 d6d/d6r/d6a revision-token profile、原 D3 Locator opaque token、旧保存的观察/ref binding 不重标成 d6_source_revision/2，也不能用新的 SourceObservation token、相同裸 revision、相同 hash、相同文字或重建 I 代替旧证据；
- 不用当前 semantic_pending、局部 ordinary success 或新 complete proof 回溯解释旧 D4/D7 gate，也不因当前证明更强而升级旧 receipt、old r5 或历史 Action 资格；
- 不用新 retention、I 删除/重建、重新授权或当前文件仍可读取作为理由，删除旧合同仍要求的 last-reference pin、原 request/decision/receipt、费用/批准/claim/unknown 责任或其它耐久事实。

saved、planned、unknown 必须继续分流。对已经 committed/saved 的原 decision，只在原 request/fingerprint/continuity 定位成立后，按其原实际 effect、mode 或结果披露范围执行当前交付授权，然后重放原封存 receipt/error/effects bytes 或完成其原版本 publication/outbox；不得要求旧 source 仍 current、旧 Frontier 等于当前 cut、旧 preview 仍有效或当前新业务证明重新成功。撤权可以遮蔽交付，但不能改 decision、重装原 after、再次收费、重分配 ChangeId/revision 或复活已合法失效的 handle/token。

planned 记录只恢复原冻结 request、InputDescriptor/owner binding、原版本的 plan、before/after pins、预算/attempt、WriteProtection、安装状态、版本依据以及原合同规定的有限 TTL/clock/continuity。未 seal 的原 plan 不因当前新类型而获得 ChangeId，也不能通过新 prepare、重选目标/Query、重采样 H/revision 或改用新 semantic gate 变成第二个 decision；当前授权、原依赖与安装归属只能决定继续原 plan、按原规则 pause/conflict，或保持 recovery_unknown。旧 plan 已过其原合同的合法期限时按原 plan-expiry/恢复规则处理，新合同不能把它复活。

unknown 或无法证明结果的历史责任继续保留原 decision、pins、安装/外部 effect 证据、费用/批准/claim/执行连续性以及相应 stop/恢复责任；不得从 current files、相同 hash、Derived Index、空 control DB 或重新授权猜 success/failure，不得因此重发 external effect、重扣费、退款、补 approval 或抹掉 unknown。重新获得授权只可能允许在原连续性仍成立时继续原合同的恢复或交付，不会重建已经丢失的 P 事实、pin、clock epoch 或 token continuity。

这些兼容责任不表示所有仅有 decoder 或历史草稿文字、但没有部署证据的 prototype 都已成为 active 产品兼容面；实现只能对实际存在并可按其原版本证明的记录履约。反过来，缺少某个新 strong producer/consumer 也不得永久禁用已经批准且不依赖它的 ordinary `.adoc`/Resource 读取、Draft、合格人工整源保存以及局部离线操作。ordinary 内容成功仍不会自动取得 complete Query/Action/Automation 资格。

依赖本 P1 新生产者的 managed success 仍保持联合 gate：P1 Lexicon、机器 Registry、Impact/Test Outline 与 PROPOSAL/replacements routing 现已有实际候选后像，完整 P2 D3 main/Lexicon/Impact 候选也已存在；候选存在不等于独立接受或激活。其余 P2 D3/D4 配套、P3 D5，以及 D7 Query/Value-CEL/View/Narrow Field/Definition Transfer/Preview-Effects/Execution-Action/Prepared/Scenarios/Lexicon-Registry/Impact、D8、D9、D10 的实际消费者必须完整配套。随后仍需 fresh 独立全量联合审查和协调接受。未来 conformance fixture 只是应当验证正例、负例、unknown、恢复与历史 decoder 的要求，不能因文档出现或 fixture 被列出就声称已经运行、通过或激活。

旧十一项 OPEN、U6/U7、后续 A2 自包含重建以及最终另一轮 fresh Pro 终审继续保留；作者不能在本文件中自行核销或接受。作者文档检查、diffcheck、CI 或本作者自查只证明各自明确执行的检查，不代替独立接受。本候选不授权产品实现、source/deps/CI 修改、merge、activation、release 或 deploy。
