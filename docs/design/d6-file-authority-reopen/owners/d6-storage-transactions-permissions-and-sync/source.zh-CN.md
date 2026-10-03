---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：`c99e6e2b-dc0c-4a3b-ab35-75a85d12ef99`。

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 中 revision05/D7/D9 的生效或候选标签只作为历史来源保留，不支配本后像。稳定文档 ID 保持不变。只有本重开所需全部 owner 后像、D10 消费者、fresh 独立联合审查和协调接受全部完成后，才可讨论激活；禁止半包启用新版受管成功。

# D6 Storage Transactions Permissions and Sync

## 1. 范围、权威分层与非目标

D6-FA-r01 的首要变化是把当前作者字节、可移植控制事实、不可重建执行责任、可重建索引和设备草稿分为不同逻辑 owner；任一物理介质都不得因方便而成为第二可写真相。

当前逻辑 owner 闭合如下：

| 事实 | 唯一逻辑 owner | 物理承载 | 可复制性 |
|---|---|---|---|
| Document 当前 exact source | 对应普通 .adoc 文件 | 用户可见普通文件 | 可由普通同步器复制字节 |
| Resource 当前 bytes | 对应普通资源文件 | 用户可见普通文件 | 可复制 |
| Node/Resource/Annotation identity、分配、不复用事实 | Portable Workspace Metadata | 库内可移植元数据 | 可复制 |
| Node parent/order、lifecycle、Trash 恢复 membership、文件绑定 | Portable Workspace Metadata | 库内可移植元数据 | 可复制 |
| Annotation 当前 closed value | Portable Workspace Metadata 中的普通数据记录 | 库内可移植数据文件 | 可复制 |
| 共享 Registry/config 与 WorkspaceAuthorizationBundle/1（Policy/3 + 已锚定 public trust history） | Portable Workspace Metadata | 库内可移植控制文件；凭据/私钥/host key handle 除外 | 可复制，但必须通过 anchor/history 验证 |
| 已认证 managed revision-seal artifact（`RevisionTokenSealArtifact/1`） | Portable Workspace Metadata | 受管 `.weftext-meta` 根下 immutable/versioned record | 只随原始 bytes 与 trust history 一并可复制 |
| 原 request/decision/receipt、事务恢复、unknown、批准/claim/Money、revision-seal outbox item、必要 pins | Durable Control Store | 库外耐久 SQLite + 私有 pin 区 | 不由普通文件同步复制消费资格 |
| metadata/search/query candidate/OCR 等派生缓存 | Derived Index Store | 库外设备本地 SQLite | 可删除重建，不同步 |
| Draft、input log、Selection、IME、未提交协作状态 | Draft Store / collaborative session runtime | 设备或 Server 私有区 | 默认不作为作者内容同步 |

Portable Workspace Metadata 以下简称 portable metadata，Durable Control Store 以下简称 durable control，Derived Index Store 以下简称 derived index；这些简称仅为本文叙述，不新增内容实体域。

Document/Resource 的 current bytes 与 portable metadata 共同形成文件型工作区，但两者职责互斥：metadata 可以指向准确当前文件、保存身份/结构/配置及版本链，不得复制完整 current body 成为另一作者源；durable control 可以保存某次原 decision 的必要 before/after pins，却不得保存全库 current body 作为读取回退。索引和 Draft 永远不是作者权威。

本稿不修改 D2 AsciiDoc Profile2 的 source 语法，不新增 D3 EntityRef，不引入 D5 持久 Record，不冻结 OT/CRDT 算法，不实现产品代码，不改变既有发布支持矩阵。实时共同文本编辑仍服从 D1 的 G2 后续排期。

## 2. Portable Workspace Metadata 与物理文件绑定

### 2.1 Portable metadata 的 current truth

一个文件型 Workspace 至少具有一个受管 portable metadata 根。首代建议物理目录名为 .weftext-meta；该名称属于 D6 文件后端，不是 D2 作者语法。portable metadata 中每个事实只有一个 current 逻辑 owner：

- Workspace identity、root NodeRef、从未复用的 identity birth/tombstone/burn-portable 摘要；
- NodeRef 到 Document 普通文件的当前 FileBinding；
- ResourceRef 到资源普通文件的当前 FileBinding；
- 每个 live Node 的 parent 与 sibling order；order 以每个 parent 的有序 child Ref 列表作为唯一 current 表示，child.parent 与 ordinal 由该列表机械导出，禁止两份可独立修改表示；
- Trash forest、restore membership 与 original-location hint；
- Annotation current closed value 及 owner；
- portable Registry binding、共享 series/scope config，以及包含 Policy/3 与已锚定 public revision-seal trust history 的 versioned WorkspaceAuthorizationBundle/1；
- Replica registration、ChangeRecord、Frontier、InstallationNotice、ContentCompletionProof、ConflictRecord；
- sealed managed production version 的 immutable `RevisionTokenSealArtifact/1` record；
- 必须可携带的 source semantic state 与 observation epoch。

文件路径、标题、内容摘要、mtime、inode/file-id 都不是 D3 identity。FileBinding 只说明在一个 portable version 中哪个普通文件承载指定 Ref 的当前 bytes；外部 rename/move 可经协调更新 FileBinding，不改变 Ref。Node parent/order 不从文件夹层级猜测；文件夹可作为 UI/存储映射，但不能替代结构 owner。

Portable metadata 的单个记录必须采用版本化 closed format、确定排序和完整 source-of-truth 关系。一个事实不得同时在两个 sidecar 中可独立写。分片只影响物理布局，读取器必须能通过 workspace root 和版本化目录恢复唯一 current record set。

当前 revision-seal profile 在受管 `.weftext-meta` 根下维护一个按 `RevisionTokenSealKey/1` 逻辑寻址的 revision-token-seal collection；exact canonical artifact bytes 与唯一性是规范要求，接纳后 immutable。既有 logical policy component 另存 exact `WorkspaceAuthorizationBundle/1` bytes：独立 authorizationRevision、完整 Policy/3、一条 WorkspaceTrustRootDeclaration/1、trustRevision 与累计 root-signed WorkspaceTrustDeclaration/1 chain；不新增 PortableComponentKey kind。接收端只能从合法 bootstrap 或显式 fingerprint import 建立的受保护 WorkspaceTrustAnchor/1 认证 root，再验证 declaration signature/predecessor、每条 declaration DecisionKey→CP3 activation ChangeId 与 causal cut。self-signed copied root、sender identity、D4 Registry seed、D10 publisher/package key 或 arrival order 都不是 authority。root/domain private key 与 opaque key handle 永不 portable；sync provider 只转发 public bundle/artifact bytes，绝不是 signing authority。

### 2.2 文件对象与外部修改

D6 的文件安装证据绑定 trusted FileObjectBinding，而不是只绑定 sha256。FileObjectBinding 是 host 可信内部类型，至少固定 backend identity、规范相对路径、object generation/identity（后端支持时）、当前 observerDomain 的 observationEpoch、byteLength、digest 及 absent/present 分支。digest 用于完整性和比较，不单独构成 CAS、identity 或生产版本。

SourceVersion/2 内的 observationEpoch 属于该生产版本的生产历史；SourceObservation/1 内的 observationEpoch 属于当前 observerDomain 的观察世代，数值相等也不合并。外部程序可以直接编辑 .adoc/Resource。Core 观察到任何下列事件时必须提升受影响 observerDomain 的 observationEpoch，并使旧 SourceObservation/1、SourceVersionRef/1.sourceToken、依赖该观察的 locator/map/PreparedIntent 与对应 dependency cut 失效；已经封存的历史 SourceVersion/2 不被改写：

- object identity/generation 变化；
- bytes、长度或 metadata 与受管 binding 不符；
- watcher/journal gap；
- 文件被删除、替换、重命名且不能由 portable metadata 的已知 ChangeRecord 解释；
- provider placeholder 变为 materialized 或反向变化；
- 后端声明其事件连续性丢失。

相同 digest 不能证明 A→B→A 未发生；观察缺口后必须提升当前 observerDomain 的 observationEpoch，即使最终 bytes 与旧值相同。无法归因到已知 ChangeRecord 的新外部状态形成 external SourceVersion/2，并在其 production CommitDomain+observationEpoch 内 checked 增加 externalSequence；external 版本没有 managed revision 或 ChangeId，externalSequence 不得填入 D3/D4/D5 的 managed sourceRevision/Locator/selector。

外部 current bytes 若 strict UTF-8 或 D2 parse 无效，仍作为真实外部原始字节保留；Node identity 不消失，但 D2 projection unavailable，状态为 external_invalid。Core 不用旧 pin 或 index 覆盖它，也不声称该状态是 Core author commit。合法 external bytes 仍可供受权 raw/source read、Draft 与人工普通整源保存使用；只有需要 managed inner revision 的 structured/typed selector 操作必须先由明确 managed 接纳/保存形成真实 managed SourceVersion，不能把缺 managed revision 永久解释为禁止既定普通文件编辑。

## 3. Durable Control Store、Derived Index 与 Draft

### 3.1 Durable control

每个可执行 CommitDomain 具有库外 durable control SQLite。它只保存不可从 current files/portable metadata 安全重建的执行控制事实：

- canonical request、fingerprint/input descriptor、decision/receipt/error；
- planned/terminal recovery、attempt/budget/lease、reservation 与 exact execution owner；
- installation recovery state、write set 与 portable-publication state；
- 每个 sealed managed after 对应的 `RevisionTokenSealOutboxItem/1`，以及保存 exact signed artifact bytes 的 `portable_metadata` pin；
- ApprovalUse、claim、Money/费用谱系、external request/send/result unknown、stop responsibility；
- PreparedIntent/PreparedAction/PreparedEdit 所需的准确 input descriptors 与必要 pins；
- ImportJob/Export publication 等受管作业控制；
- 当前 execution authority/custody/fence。

运行中的 control.sqlite3、WAL、SHM、私有 pin staging 均不得置于普通同步目录，也不得由网盘 merge。SQLite 单写者只限定一个物理数据库写事务的并发，不限制产品同时有多个用户、Draft、read、prepare 或不同文档编辑会话。

durable control 不是 portable metadata 的 current owner。它可以引用某次 portable metadata/source 的版本并保存恢复所需副本；恢复只能完成原 plan 绑定的 current version，不得从 control DB 推导一套独立最新 parent/order/policy 覆盖 portable metadata。

### 3.2 Derived index

Derived index 是设备本地 SQLite，状态闭集 building|ready|unavailable。每个层级记录完整构建 inventory、parser/profile/Registry/search/OCR version、输入 SourceVersion/Frontier 与连续失效消费位置。ready 只代表该层及范围覆盖；部分/旧 index 不能证明空范围或全集。

允许层级至少为：inventory/metadata、D2 parse projections、typed Field/relation candidates、search candidates、attachment extraction、OCR。不得用方便理由将全库 current source 或完整 AST 永久复制到 index。contentless FTS/gram 可以保存候选 token，但仍受权限、覆盖和回读语义约束。

删除整个 derived index 后，Workspace identity/structure/policy、original decisions、ApprovalUse/Money 以及仍完整存在于真实 P/M owner 的 DependencyProof 范围连续性事实均不变；重建不得 mint identity、重新消费 approval、重放 external effect或重签旧proof。正确性所需的范围 epoch/revision、完整枚举边界、empty证明和连续消费位置不由I拥有：I只能缓存候选/枚举结果，`ready`、hash、row count或“没有命中”不能自行证明完整或empty。若这些正确性范围事实本身丢失、watcher gap或owner-version连续性不可证明，旧stamp失效；只能在当前授权下重新完整枚举真实范围并建立新continuity epoch。placeholder、I/O失败、隐藏未授权对象或未覆盖分片都不能当empty。D6保存其闭合key/stamp/pins；D3/D4/D7各自拥有其范围枚举算法，Storage不接受free JSON，也不让缺strong range永久阻断不依赖它的ordinary局部操作。

### 3.3 Draft

Draft 与 D8 原 state machine 保持设备/session 私有。Draft 保存用户 proposal、input log、selection 与 base binding；它没有 author revision、ChangeId、D3 locator authority 或 committed status。普通 cache 清理不能静默删除 dirty Draft。durable save 与 portable publication 是独立可读状态，UI 不得以 Draft persistence、worker success、HTTP 200 或 sync provider upload 代替它们。

## 4. CommitDomain、ReplicaEpoch、DecisionKey、ChangeId、Frontier 与来源观察

closed wire 形状由 Control Interfaces 拥有；本节冻结其存储语义。CommitDomain 区分 replica/server：replica=WorkspaceRef+ReplicaEpoch，server=WorkspaceRef+当前托管 AuthorityInstanceId。新 v2 DecisionKey/2 是 WorkspaceRef+完整 CommitDomain+OperationId；P 以 workspaceId+D3-CJ/3(commitDomain)+operationId 索引，同键恰一 D3 或 D6 protocol owner/canonical decision。不同 domain 可复用相同 OperationId。旧 v1 key/D3 v9-v11 saved decisions按原decoder/continuity，不迁移。

ReplicaEpoch 只由 Core 显式注册时 mint、UUIDv4、同 Workspace 永不重用；它只授当前共享 policy/trust 下普通内容域资格，不是 D3 continue、AuthorityInstanceId 或 execution custody。复制文件不产生 epoch，retired 不复活。

ChangeId/1 由完整 CommitDomain 和单调 sequence 组成，首个 portable content change 的 sequence=1；递增必须 checked，超过 MAX 不得 wrap。ChangeId **只在 portable effect 的 P seal** 分配；prepare、planning、staging、InstallationNotice 都不预留序号。失败、paused、recovery_unknown、control_only 和 true raw no-op 均无 ChangeId 或序号空洞。

Frontier/2 是每 CommitDomain 至多一个最大已验证连续 sealed ChangeId 的 canonical vector，按 domain bytes排序唯一。它可接纳remote verified ChangeId，但不重放remote OperationId，也不为transport另mint本域ChangeId；不证明payload物化、placeholder下载、index complete、D7 complete cut或provider完成。frontierPolicy=exact|scope_dependencies：exact要求完整Frontier equality；scope_dependencies只允许从原expectedFrontier到当前Frontier的可证明非回退无关扩展，并逐项重验原source/control/auth/positive-negative dependency。纯无关sealed head增长本身不是依赖变化；真实SourceObservation、FileObjectBinding/pin、授权、Registry/rule、membership/negative-range或其它绑定依赖变化仍必须stale/conflict/reprepare。scope_dependencies不能换target/Query/source/request、重选当前页、消除ABA或把未知gap当无关。Frontier/1只legacy。

SourceVersion/2 标识实际source及生产 CommitDomain。对生产域 D 和实体 E，`H(D,E)` 是该domain连续已封存历史中 E 的最大 managed revision；只有从domain birth/registration、P continuity及已验证portable sealed history证明从未为E封存managed版本时，完整空历史才允许H=0，缺失/损坏/未知历史不得当空。每个真实managed source change checked使用H+1，同一生产域跨observationEpoch不重置H，MAX不wrap。

fresh managed source在完整空历史上revision=1。已有source首次由另一生产域写入时使用新生产域自己的H+1，不取旧域revision+1；之后跨域往返分别继续各自H。跨域相同revision数字不等价。真实raw no-op保留原SourceVersion/2，即使生产域异于当前operation；纯placement/lifecycle/control且source未变也不增source revision；source删除after=absent，不造删除版本。equal-byte external admission建立managed接纳事实，仍不是raw no-op。external SourceVersion/2保持既有完整变体，仍包含kind、version、entityRef、commitDomain、observationEpoch、externalSequence；这里讨论的生产版本字段是commitDomain、observationEpoch与externalSequence，它没有managed revision或changeId。接纳到managed时after revision仍由当前managed生产域H+1决定，externalSequence永不充当inner sourceRevision。

当前副本/Server观察用 SourceObservation/1 另绑定 observerDomain、EntityRef、完整生产 SourceVersion/2、当前observationEpoch、FileObjectBinding与evidence pins。observerDomain=当前operation domain，sourceVersion.commitDomain可不同；生产SourceVersion的observationEpoch保留生产历史，SourceObservation.observationEpoch表示当前observer世代。placeholder/缺metadata/conflict/continuity不明无成功Observation。SourceVersionRef/1 token选择完整Observation；gap、external replacement或discontinuous rematerialization即使生产version/hash/文本相同也使旧当前资格失效。

除下文具名独立 conflict-only /2 外，Storage在P中为每个将产生managed source version的plan耐久保存内部 SourceRevisionPlan/1：before当前Observation或明确absent、同生产域该实体最后已封存managed SourceVersion或经证明的none、拟议SourceStamp/1以及exact after pin。SourceStamp仅含同DecisionKey下的拟议entity/revision/生产observationEpoch地址；当前operation作为after生产域时，其epoch取原plan冻结的当前operation-domain target观察世代，不复制foreign before的生产epoch。它没有本次ChangeId，不是成功SourceVersion。closed shape由后续同一P1 Control正式冻结；Storage不建第二wire owner。winning plan固定这一依据，seal前重验last-issued history。

D3 仅供冲突解决的 canonical 物化路径另消费 Control §3.1.1 ConflictInstallInput/1 与 §6.2.1 SourceRevisionPlan/2。此不同类型的 wrapper 在原 DecisionKey/audience/preparation/input-use guard 下证明真实已安装 sealed-head source/metadata、当前 FileObjectBinding 与安装世代。它绝非 SourceObservation，没有 sourceToken，不进 InputDescriptor.sourceInputs，不提供 canonical read、Query、D8 或普通写资格；普通/current Observation 对 conflict 仍拒绝。原 plan 独立保存完整 wrapper、source/conflict/range proof、before/selected/final pins 和 exact recovery state，source/metadata 经 strict protection 与唯一 P seal 安装。canonical birth claim/所选生产版本或最终 bytes 不同，均为真实 source-state admission，即使 bytes 相同也通过 /2 取当前域一次 H+1；claim/version/bytes 保持则不造 source version/H 增量。metadata-only open resolution 仍 portable，只有全空 already-resolved 选择为 true no_op。existing canonical identity 不得称 fresh/absent；同 decision 的 fresh copy 保留独立 /1 plan，单实体不产生两版最终 source。CP3 运输真实已安装生产 before 与唯一 sealed managed after，不把历史所选版本当新版本。saved/planned/unknown 保留原 exact-version record、pins、guard 和安装责任，不重签 Observation、不重选 head、不新 decision 或改历史。Control 唯一拥有 shape，D3 拥有显式业务选择和完整净效果投影；Storage 不新增通用 Resource write API 或 raw-mode 权限。

D3 stage12私有candidate map仍是fresh identity唯一候选来源。新的revision-token profile/2由Control冻结、Storage保存，绑定observerDomain/current observationEpoch以及managed SourceStamp或external SourceVersion；D3 Locator/revision-token外层词法和D4/D5 inner selector wire不改。拟议managed token只供同一原plan符号/验证，decision seal并形成完整current SourceObservation后才取得current locator资格。旧profile/decoder原样保留，相同hash/文字/revision/I不能跨观察gap续认。

已有CommitDomain若丢失P/生产历史连续性，不得猜H=0继续该domain；但这不永久冻结普通文件Workspace。portable current完整可验证且无未决安装风险的范围可按既有replica registration登记新ReplicaEpoch/CommitDomain继续ordinary content，新域从自己的完整空历史开始H；旧域decision/unknown/Money仍不得从文件重建。

## 5. 授权、普通保存与完整语义资格

### 5.1 三层资格

D6 冻结三类互不蕴含的资格：

1. ordinary replica content qualification：证明本次普通 source/create/move/reorder/Trash 所实际触及的 source/identity/structure/policy 和安装后端；
2. complete semantic/action qualification：证明 D4/D5/D7 指定动作真实需要的完整正/负范围、完整 cut、授权与语义；
3. global execution responsibility：证明 Automation、ApprovalUse、claim、Money、external unknown、stop 等执行域只有连续责任持有者。

无 global execution responsibility 只暂停对应执行能力，不永久禁止 ordinary content。building index 只影响需要其完整证明的动作，不永久禁止 ordinary content。反之，ordinary content success 不升级为全集 Query/Action proof。

### 5.2 ordinary 证明范围

普通完整 Document edit 至少绑定：完整 SourceObservation/1（含实际生产 SourceVersion/2）、完整当前 source、entity lifecycle、当前 policy、actual MutationFootprint、D2 完整 parse、所有实际修改 local typed facts及当前 Registry。未修改的原 bytes 必须逐字保持。若变换触及需要完整 cross-object proof 的 D4/D5 条件而当前不能证明，Core 可以在显式 ordinary-save profile 下得到 semantic_pending；不能把它记录为 complete semantics。

create Node 绑定：destination parent、完整目标 sibling list、必要祖先 chain/cycle proof、新 identity reservation、完整新 Document source、实际 local typed admission、old/new policy scope。move/reorder 绑定 subject、old/new parent、两边完整 sibling list、必要 ancestor/cycle proof、旧新授权范围；未涉及的全库 source 不作为前置。ordinary Trash 绑定明确 subtree/owner-local closure、相关 live/Trash sibling lists、restore membership 与权限；它只证明本地 lifecycle effect，不伪造完整全库 inbound-reference enumeration。

restore、purge、relation mutation、unique Calendar/config、collection create/remove、all_result/bulk 等若上游语义要求完整范围，则继续走 complete qualification；无法取得完整范围时不得降级成同一强 Action 的 ordinary success。用户若只想保留 source bytes，可另发明确 ordinary-save request，其结果 semantic_pending，强 Action 仍未成功。

有限 Field 权限仍须使用 D7/D6 的静态独立性证明；不得先读取隐藏 sibling/Field/constraint 再根据实际值决定是否允许。parent/order 的局部证明也须在读取 sibling list 前具备对应结构观察资格。

### 5.3 SemanticState

受管 source 状态分为：

- complete_semantics：D2 valid，所有本次适用 D4/D5/结构/控制语义已在声明 scope 上完整证明；
- semantic_pending：D2 valid，本次实际修改的 local typed facts 已通过，但一个或多个跨对象/全集 obligation 尚未证明；
- external_invalid：current external bytes 无法通过 strict UTF-8/D2，因此没有成功 Core author decision；只可 Source/repair/read-raw 路径消费。

semantic_pending 可以作为普通保存的 current source；strict reliable或durable_observed_only由WriteProtection独立表示。semantic_pending不能作为以下能力的合格输入：需要完整 D4 relation/unique/Calendar invariant 的 mutation；需要 complete query cut 的 D7 ActionEvidence/all_result/post-query；自动化/Agent 自动写；purge；任何声称“整个 Workspace 约束已验证”的导出或审计。只做 exact source read、Source editor、原文件搜索或明确局部 projection 的消费者可以在自身授权下使用，并必须向用户显示 pending 状态。

固定C中的D4/D5 current afterimage已经消费A/B/C的Frontier/2、SourceObservation与WriteProtection边界；本P1新增的生产revision/range-proof/portable-record版本仍待P2/P3配套，不据此宣布新strong success可用。D7 complete consumer仍待后续owner。

### 5.4 人工普通保存的并发保护边界

WriteProtection与ContentGuarantee/SemanticState分域。observed_only只允许受信 `interactive_source_save` 对恰一个既有live Document执行ordinary+replica_local整源保存，并要求完整source read/replace、author source write set为空或仅该Document、无适用body/Field/node-control deny、不修改identity、parent/order、lifecycle、shared policy、Registry、Calendar scope或其它entity，且Draft Base等于当前选定SourceObservation。人工必须在planning开始前显式选择observed_only并冻结profile；planning开始后strict失败、known Base conflict、失权、耐久失败、strong obligation失败或其它资格缺失都不得fallback为observed_only。

唯一放宽是最后验证后到安装前从未观察的external race可能被N覆盖。已读B与输入N按原plan耐久保留；未观察C可能没有可恢复副本，later C也可能再次替换current file，但不能丢B/N。已观察change、watcher gap、stale Base、细项deny、Core竞争、unknown install或P continuity缺失仍停；known竞争/gap走conflict-reprepare，unknown install保持recovery_unknown，prepare/retained不是Saved。D3 identity/parent/order/lifecycle、D5 structured cell/row/column/reorder、bulk/collection/promotion、D7 strong Action、Automation、server checkpoint、approval/Money全部strict；ordinary语义与strict|observed_only保护两轴不扩大权限，也没有逐次审批。

## 6. 文件安装能力、WriteProtection 与可靠保存

### 6.1 BackendQualification
strict已有文件只允许真实conditional_replace或exclusive_write_window；expected-absent新文件可create_only。conditional必须trusted generation且read-hash-then-rename不是CAS；exclusive须排除威胁模型内全部writer，advisory锁不算；create_only原子create-if-absent。

observed_replace只服务§5.4 observed_only，记录最后验证object generation但不是CAS；破坏安装前再次检查trusted object/event continuity，已观察competition就保留B/N/current并停止。只有未观察race属于获准弱保证。

所有路径仍证明staged/target/directory-entry或rename耐久、canonical containment和installation provenance。strict无合格原语→install_unavailable/paused；满足§5.4的意图可新prepare observed_only，已有strict plan不能改弱。

### 6.2 Guarantee
ContentGuarantee与WriteProtection分开。replica_local证明本domain固定write set按请求保护级别durable install+P seal，不声明其它offline replica无并发。managed_atomic要求所有author/portable-control write进入受管barrier且WriteProtection=strict，不承诺外部工具多文件瞬时原子。多个rename不是全局事务。

## 7. Prepared、安装、seal 与 portable publication

### 7.1 InputDescriptor、SourceRevisionPlan、DependencyProof 与 pins

D6 v2 的规范提交请求保持为小型控制请求，不内嵌完整正文或资源字节。PreparedIntent/2 保存工作区与提交域（Workspace/CommitDomain）、意图 intent、内容前沿及其策略 Frontier/2+frontierPolicy、观察范围 ObservationScope/2、当前来源观察 SourceObservation/1、依赖证明 DependencyProof/2、保存范围与配置 scope/profile/write-set、Registry/policy/rule，以及所有者输入 OwnerInputBinding/2；完整字节继续存放在按用途绑定的 pins 中。

ordinary/fresh source plan冻结§4 SourceRevisionPlan/1；仅具名D3 canonical-resolution分量使用Control §6.2.1的/2。/1 record包含：before观察或absent、该生产域该实体last sealed managed版本/完整空历史证明、拟议SourceStamp和exact after pin。它只是版本分配依据，不是成功版本，不含本次尚不存在的ChangeId；external→managed接纳也走该分支，true raw no-op和纯structure/control source-unchanged分支不造SourceRevisionPlan。

DependencyProof/2引用的完整枚举、empty证明、range epoch/revision、当前授权和必要pins必须存在于真实P/M owner并可在planning、verify与recovery按原plan复验，不能只存在于I。D6维护source、authorization、replica_registry、conflict_record、execution_resource连续性；D3/D4/D7提供其具体closed key/enumeration。未完成owner后的strong consumer保持owner_update_required/proof_unavailable；不接受free JSON，也不阻断不依赖该strong range的ordinary局部操作。

input equality 要求descriptor、exact pins/owners、SourceObservation、DependencyProof、SourceRevisionPlan与closed owner input完整一致；相同sha256、H数字或I重建结果不足。修改source/Observation、mapping、policy、frontierPolicy、scope、WriteProtection、owner request或版本依据需新prepare；同OperationId只exact replay，不能改plan降级strict。

### 7.2 Pin 分类

每个 pin 必须记录 purpose、owner request/job、byteLength、payload type、source version、
  created control revision、last-reference policy、retention class、capacity account。允许长期保护的类别仅为：

- planned/installation recovery 的 before/after；
- unresolved conflict branch 与 merge base；
- external unknown/approval/Money 责任所需冻结证据；
- ImportJob/ExportPlan/Query result 等其 owner 协议明确要求的 pin；
- 明确用户 history/backup policy。

打开/解析/索引 Workspace 不能自动为每份 current source 建永久 pin。decision 不再引用、无 unknown/冲突/用户保留且达到 retention policy 后，大 effects bytes 可以失效；canonical request、decision state、receipt/error、费用/批准/claim 责任和最小 input descriptor 继续耐久。旧 D6/D7 协议已经承诺永久/decision-lifetime pins 的 saved decision 不追溯删 pin；新 retention 只适用于新版本记录。历史 effects pin 已合法回收后读取返回 effects_unavailable，不能回读 current file 冒充旧 after。

容量不足在开始新 installation 前失败或暂停，不删除 protected pin。planned/unknown pin 的最后引用不能由 TTL、preview expiry 或用户关闭窗口解除。

§7.2.1 的准确已注册调度连续性 owner 亦属于本节允许的长期恢复保留用途；只保留所选订阅实际需要的证据，不扩大为全工作区永久固定。

### 7.2.1 受保护调度连续性生产者

活动 D10 ScheduleSubscription/1，以及仍被武装、认领或未知恢复引用的退休订阅，是明确允许的恢复保留 owner。打开、解析或索引工作区不创建订阅，也不为全部源永久固定。owner 是准确且不复用的自动化控制引用与订阅代际，不是 Field key、路径或当前定义修订。创建先通过 D10 Control §16 所选源、字段、Registry 和完整调度门，预留有限保留容量，再在同一配置事务保存初始证据并注册到真实 Core 源与控制变化生产者。注册失败不能产生看似连续的订阅。

    ScheduleContinuityWitness/1 = {
      kind:"d6_schedule_continuity", version:1,
      automation:D10.ControlRef<automation>/1,
      subscriptionGeneration:Counter,
      revision:Counter,
      initial:D10.ScheduleRecurrenceEvidence/1,
      checkpoint:D10.ScheduleRecurrenceEvidence/1,
      status:"continuous"|"binding_changed"|"gap",
      producerEpoch:Token,
      consumedTransition:Counter
    }

    ScheduleContinuityStep/1 = {
      kind:"d6_schedule_continuity_step", version:1,
      automation:D10.ControlRef<automation>/1,
      subscriptionGeneration:Counter,
      expectedWitnessRevision:Counter,
      producerEpoch:Token,
      transition:Counter,
      before:D10.ScheduleRecurrenceEvidence/1,
      after:D10.ScheduleRecurrenceEvidence/1,
      portableChanges:[{
        changeRecordPin:PinRef/2,
        installationNoticePin:PinRef/2,
        completionProofPin:PinRef/2
      }],
      dependencyBefore:DependencyProof/2,
      dependencyAfter:DependencyProof/2,
      retainedInputs:[PinRef/2]
    }

    ScheduleContinuityInvalidation/1 = {
      kind:"d6_schedule_continuity_invalidation", version:1,
      automation:D10.ControlRef<automation>/1,
      subscriptionGeneration:Counter,
      expectedWitnessRevision:Counter,
      producerEpoch:Token,
      transition:Counter,
      status:"binding_changed"|"gap",
      evidencePins:[PinRef/2]
    }

源已消失、冲突或无效时，可能不存在有效后态 Observation 或 D4 上下文。此时生产者使用上述闭合失效记录，保留最后有效检查点，不能虚构 ScheduleRecurrenceEvidence。binding_changed 要求完整受信证据证明所选业务真实不连续；后态不可用、未知或历史缺失都是 gap。evidencePins 保留实际可得的原分型证据，只有保护生产者自身证明采集或可用性缺口时才允许为空，不能据此声称业务变化。失效比较同一当前见证和注册，原子保存永久状态及下一个 checked 转换和修订。最后一个 Counter 增量专门预留给失效：正向折叠在 MAX 前停止，容量或计数耗尽使用该保留增量标记缺口，不回绕，也不阻止无关普通源工作。失效引用使用 artifact/recovery，字节为 UTF-8 的 D6-Schedule-Invalidation/1、一个 NUL，再接完整 D3-CJ/3 字节，不能重置已经失效的代际。

这是持续维护的保护控制见证，不是调用方证明、可移植签名、通用账本或可重建索引缓存。initial 不可变，逐字等于原订阅证据。checkpoint 初始等于 initial；只有在准确切面成功注册得到证明后，才从 revision=1、consumedTransition=0 开始。producerEpoch 标识真实连续注册的 Core 消费者及其耐久转换收件箱。数值计数不能创造连续性；源与控制生产者必须保留每次匹配前后转换，证明从同一注册点完整交付，覆盖所有因果分支及中间相关 Registry、规则和配置状态。重置或重建收件箱不能复用世代，第二写者不能独立更新见证。

每一步都从实际已验证的同切面状态内部派生。before 等于当前见证 checkpoint，预期修订、世代及下一个 checked 转换序号匹配。完整依赖键集合覆盖所选 owner、身份及生命周期、必需 Facet 和两个 Entry key、相关源、Registry、schema、历法、时区、tzdb 和规则输入，以及适用完整业务范围。新发现依赖必须先取得资格，并证明其全部中间历史，否则为缺口，不能先读隐藏状态再扩范围。前后依赖证明对应真实切面；完整 Registry 快照、演进证明及 D4 RecurrenceReadContext 来自前后证据。Core 按既有分型解码器保留真实中间源、元数据、已接纳规则资产和控制图像，不能只用摘要替代。

每次可移植转换的三个 artifact 引用严格解码为真实原 D6 ChangeRecord、其版本化 InstallationNotice 及 ContentCompletionProof，必须证明同一已封存 ChangeId、分量图像及完整因果前驱链，不能从当前文件重建。源及元数据载荷使用真实 exact_source_document 或 portable_metadata 类和生产 SourceVersion。历史转换保持真实原解码器；新转换使用 InstallationNotice/2 和 CP3。纯 P 控制或规则转换从真实序列化的保护前后状态采集，retainedInputs 保存完整快照及已接纳规则资产；不能因为没有源变化就跳过匹配依赖键的修订。未注册的提供者变化、外部写入或替换、观察缺口、转换缺失、解码器未知或分支不可证，均须在新扫描使用见证前标记 gap。

Core 对每个中间转换准确应用 D10 原业务连续性谓词：owner 持续 live，必需 Facet 和所选 key 持续存在，完整解码的 recurrence/range 值及真实相关语义规则身份不变。合法无关正文、Entry 或格式变化可推进 checkpoint；删除重建、暂时消失或规则改变后复原则将本代永久标为 binding_changed。单纯预期与最终摘要相等不能折叠掉这些转换。规则身份不变且新增范围完整覆盖得到证明时，扩大 horizon 合法。observed_only 的 B/N 不证明未观察的外部 C 从未出现；受影响弱安装没有原严格完整历史证据时，不能认证调度连续性。

消费与源控制转换发布、订阅配置和检查点更新、认领、停止及责任检查共用同一 P 序列化域。生产者要么先耐久保留完整下一转换再释放历史引用，稍后由获权消费者验证；要么在该事务中验证并折入见证。推进时修订、转换序号及完整检查点一起更新，不暴露部分检查点。保存的保护见证代表已验证前缀，因此成功折叠后已无引用的中间载荷可释放，而不丢掉已建立的不变量。压缩不能替代不可变初始证据、当前检查点、未消费收件箱、原已处理发生键与处置，或其它未决责任。不折叠时可以验证完整保留链，其完整性及谓词义务相同。

容量不足时源控制提交不能静默丢证据。若已注册有限收件箱无法保留匹配转换，同一生产者事务必须先将订阅标记为 gap，才能释放不再使用的历史容量；普通源工作随后可以继续，但未来扫描不能再称连续。后端或提供者不能原子执行该失效时，本调度配置不可用。每次更新最多 4096 个引用、16 MiB 规范证据；源和分量字节另按 PinBudget 预留。超大或不完整历史不能部分推进。已证明业务变化在调度域为 binding_changed、管理域为 control_conflict；缺口为 state_unavailable。两者都需要当前资格下显式 replace 建立新代，不改变原认领或未知请求。重建、重启、相同最新字节或更大 Frontier 都不能重置为 continuous。

见证及转换字节使用 artifact PinRef/2、recovery 保留类，分别采用 UTF-8 的 D6-Schedule-Continuity/1 或 D6-Schedule-Step/1、一个 NUL，再接 D3-CJ/3 规范字节。Core 通过保护记录来源及准确订阅、生产者关联认证，不能只凭摘要认证。D10 continuityPins 可以保存这些分型见证与转换引用，或完整原链，不接受自由 proof-map。读取或消费先检查当前源、字段及 owner 授权，不披露隐藏历史。replace 与新配置在同一事务退休旧生产者注册；只有全部订阅、原决议、发生项、未知、费用及其它保护引用都消失，并符合原保留承诺后，GC 才能释放引用。真实旧合同的较长固定义务继续保留，不引入全工作区永久历史或自动删除旧恢复证据。

### 7.3 状态机

每个 v2 content decision 保存独立 DecisionState、InstallationState、ReliableSaveState、PortablePublicationState：

DecisionState：unseen → rejected | planned → committed | terminal_failed。
InstallationState：prepared → planned → installing(k) → installed；
  另有 conflict | recovery_unknown | paused_authorization | paused_capacity。
ReliableSaveState：not_saved | reliable | durable_observed_only | not_applicable。
InputRetentionState：not_retained | retained | unavailable。
PortablePublicationState：not_published | pending | published | conflict。

顺序固定：

1. prepare：冻结 InputDescriptor、write set、pins、proof、preview、budget 与 WriteProtection；可能产生managed版本时同时冻结SourceRevisionPlan。只有 proposal、实际 read-before 和必要 binding 已形成 durable pin 时才能进入 retained；没有 author effect，retained 也不等于 saved。
2. planning：先证明 required pins durable/capacity reserved及SourceRevisionPlan的last-issued/empty-history依据连续；P事务保存 canonical request、fixed plan、reservation、installation recovery description、版本依据与planned。只冻结拟议SourceStamp，不分配ChangeId，不把stamp写成已封存SourceVersion。
3. portable notice：修改 portable current 前，先耐久写入 InstallationNotice/2。记录包含 DecisionKey、guarantee、WriteProtection、base Frontier/2 与 before/after components；baseFrontier可含历史ChangeId，但notice没有**本decision尚未seal的ChangeId**，也不含approval、Money或external payload。
4. install：staged after 必须先耐久落盘；strict 使用 create_only/conditional/exclusive，observed_only 仅§5.4并在安装前最后检查 object/event continuity。已观察competition则保留B/N/current并停止；D3结构/lifecycle、多对象、D5 structured和strong操作仍strict。
5. verify：written=planned after；unwritten dependency继续按原before/cut expectation。exact仍要求完整Frontier equality；scope_dependencies只接纳与原完整source/control/auth/正负范围均无关的可证明非回退sealed扩展。真实SourceObservation、FileObjectBinding/pin、授权、Registry/rules、membership/negative-range或其它dependency变化必须stale/conflict/reprepare。unknown provenance、third_state、late competition或失权保持paused/conflict/recovery_unknown；仍无本次ChangeId。
6. seal：written=planned after且原plan/deps/auth仍成立时，P单一transaction checked分配ChangeId。只有实际source change且after为managed source的分支，才把对应SourceRevisionPlan的SourceStamp与该ChangeId合成为唯一managed SourceVersion/2，并原子推进该生产域实体H(D,E)；SourceStamp不成为第二current truth。对每个这样的 managed after，同一 transaction 取得 winning plan 冻结的 exact RevisionTokenBinding/2，构造完整 RevisionTokenSealAssociation/1，使用实际 seal cut 上对该生产 CommitDomain 有效的既有 portable-trust key 签名 domain-separated canonical body，把 exact RevisionTokenSealArtifact/1 bytes 保存到 portable_metadata pin，并写入恰一条 RevisionTokenSealOutboxItem/1。签名/trust 失败必须在 commit 前中止；绝不允许 seal 后 lookup/regeneration。source 删除 after=absent 仍保留本次portable effect的ChangeId及原notice/proof恢复责任，但不创建managed SourceVersion/association、不使用SourceRevisionPlan、也不推进H。source 未变的 structure/lifecycle portable effect 同样不创建 SourceVersion/association，也不推进H，但仍使用本次 seal 的 ChangeId；P-only control_only和true raw no-op仍按§7.4不产生content ChangeId。随后同一 transaction 写committed decision、receipt、effects、ReliableSaveState、适用charge、association pins/items 与既有outbox。strict→reliable；observed_only→durable_observed_only。这里是唯一commit point。
7. publication：新FA portable decision从sealed事实生成ContentCompletionProof/3并推进Frontier/2；proof运输实际生产SourceVersion before/after，不运输发送副本的SourceObservation token。另由同一个既有 outbox 对每个 managed after 发布已经 pin 的 exact RevisionTokenSealArtifact/1 bytes；不重新生成、不重签，也不给 CP3/Notice 加成员。接收端分别验证 CP3/history 与 signed artifact 并交叉核对，再为自己的 observerDomain 建立 SourceObservation。ContentCompletionProof/2及更早版本只按原decoder/replay；control_only/no_op为not_applicable。

第6步成功而第7步失败时decision与reliable/durable_observed_only已成立，publication=pending；恢复只补同一sealed版本的proof，不重写source、不换OperationId、不再次增加H/ChangeId、不重复收费、不重装N。远端在proof/components完整前只看到incomplete transport。

撤权发生在seal前：停止seal。能在BackendQualification下安全恢复exact before才恢复并flush；否则保留before/after/current、SourceRevisionPlan与recovery evidence，进入paused_authorization或recovery_unknown，不覆盖第三种bytes。撤权不把未知安装写成永久业务rejection。

### 7.4 raw no-op

committed effectClass 的闭合取值为 portable|control_only|no_op。
只有 portable 会产生 ChangeId、source changes 并推进 Frontier/2；P-only control 使用 control_only。
true no-op 要求 sourceVersions=[]、InstallationState=not_required、ReliableSaveState=not_applicable、PortablePublicationState=not_applicable。
domainCommitSequence 可增加 1，但 source revision、ChangeId 与 Frontier 都不变。
equal-byte external admission 不属于 no-op。

## 8. 崩溃恢复与历史结果

恢复总则：先读 P 决议，再读取 installation notice、write-set bindings 与实际文件。客户端超时、文件存在、mtime、digest 或 provider 状态都不能替代 P。

每个 component 分类只允许：

- exact_before：FileObjectBinding/bytes 与原 plan before 完整匹配；
- exact_after：与该 plan 固定 poststate 完整匹配，并可证明安装归属；
- third_state：其它 bytes/object/placeholder/缺失；
- unavailable：后端无法安全读取或证明。

相同 bytes 但不能证明安装归属时不能从 third/unavailable 提升到 exact_after。

observed_only read-before仅实际read/pin前像，不枚举未读C；未观察C若被覆盖可能无可恢复副本，这是批准边界，record不得虚构。已观察competition仍third_state/conflict；unknown install不猜。

故障规则：

- 无 planned：只清理已证明无引用 staging；未赢得plan的拟议SourceStamp/H候选不产生历史。
- planned、未 install：恢复同一 InputDescriptor、SourceRevisionPlan、拟议 RevisionTokenBinding、pins、reservation、OperationId和budget counters；不重新sample identity、H、revision token或当前页，也没有本次ChangeId；loser/aborted 记录不能借另一 decision 的 seal 变成 canonical。
- installing：only before/after且installation lineage连续可证时恢复同一plan；third_state保留current bytes/pins/版本依据并进入conflict/recovery_unknown。
- 全部 after 但 seal 未知：先读P。P committed使用已保存ChangeId/SourceVersions/receipt；P planned只恢复原plan，不能凭files、hash或SourceStamp猜committed。
- committed 但 response 丢失：通过当前**原saved效果范围的交付授权**后返回原receipt bytes；不要求旧before SourceObservation、旧Frontier或r5业务dependency仍等于current r6，也不再写files/Frontier/H/费用。当前撤权可以遮蔽交付，但不改saved decision。
- committed、portable publication pending：只发布原 ContentCompletionProof/3，以及 saved RevisionTokenSealOutboxItem/1/pin 选定的 exact RevisionTokenSealArtifact/1 bytes。不得从 SourceStamp/SourceVersion 重构 artifact、改用 current trust key、重新签名、mint 另一 token、重装N或再次分配ChangeId。历史decision继续原/1或/2 decoder，绝不倒追获得本未部署 profile。
- derived index/outbox 更新失败：按owner重建/补消费，不回滚author decision或从I恢复P责任。

saved、planned、unseen必须分流。saved只做原request/fingerprint/continuity定位、当前适用披露/交付授权和原bytes重放；planned只恢复原plan并按实际安装/依赖状态继续或暂停；只有unseen才以当前owner版本、SourceObservation、DependencyProof和frontierPolicy建立新业务decision。当前r6新证明不能回溯拒绝r5，也不给r5新增强资格。

历史 r5 receipt 与 current r6 source 分开：replay r5只返回r5 saved bytes；current read使用current SourceObservation/SourceVersion/Frontier。later r6 complete proof只证明r6，不修改r5。Undo/restore仍需新plan/preview，receipt replay不能倒退current。

## 9. 同步、接纳与冲突

### 9.1 Replica registration 不等于 execution takeover

新设备获得完整 portable Workspace 时，先从受保护 WorkspaceTrustAnchor/1 认证 Workspace root，再验证 workspace identity、portable metadata chain 与 WorkspaceAuthorizationBundle/Registry，然后显式 register 新 ReplicaEpoch。joining host 在受保护存储生成 domain key；一个原 P seal 以同一 DecisionKey/ChangeId 同时推进 replica_registry 与 policy/WorkspaceAuthorizationBundle，接收端把 active ReplicaRecord 与 exact root-signed authorize declaration/PoP 交叉验证。注册只建立该 ordinary content CommitDomain 与 exact revision-seal key authorization。它不接管旧 P 的 ApprovalUse、claim、Money、external unknown 或 Automation lease，也不使用 D3 continue_workspace。

执行域接管必须另有完整 continuity proof，证明原 decision/receipt、charges、unknown、claims、standing approvals、stop state 全部连续且旧执行者已失效；无法证明则该执行能力暂停，但普通 replica content 继续。

P 丢失时，原 execution decisions/unknown 不能从 files 重建。若 portable installation notice 指示某范围可能有未决安装，则该范围先进入 reconciliation；无安装疑点的普通 source 仍可在新 ReplicaEpoch 下工作。建立空 control DB 不退款、不补 approval、不重发 external request。

### 9.2 Transport completeness

同步 provider 只运输 ordinary files 和 portable metadata immutable/versioned records；
  不运输活动 control.sqlite3/WAL/SHM、derived index 或 Draft。

新FA portable change使用ContentCompletionProof/3。接收端只有在某ChangeId的InstallationNotice、ContentCompletionProof/3、全部listed component bytes/metadata以及proof内生产SourceVersion before/after都到齐并互相验证后，才接纳该change到本域Frontier。对每个 non-absent managed sourceChanges.after，还必须到齐 RevisionTokenSealKey/1={proof.changeId,entityRef} 对应的 exact RevisionTokenSealArtifact/1。通过既有授权与 portable-history/trust 门后，接收端验证生产 CommitDomain 的历史 trust declaration，取得 exact RevisionTokenSealVerificationKey/1，验证 artifact 的 domain-separated Ed25519 signature 与 canonical bytes，strict decode association，并把 association.decisionKey/changeId/sourceVersion 及 SourceStamp 字段与 CP3 逐字核对。sender/forwarder 身份、相同 SourceStamp/SourceVersion/digest 或重算 token 都不够。只有两条证据链都验证成功，才能持久化 canonical token→production-version mapping。proof中的SourceVersion是生产历史，不是发送方current SourceObservation；随后接收端才根据自己的CommitDomain、当前FileObjectBinding、observationEpoch、evidence pins和continuity建立新的SourceObservation/SourceVersionRef，不能复制发送方sourceToken。

ContentCompletionProof/2、InstallationNotice/1及更早saved transport继续原decoder/bytes/接纳规则，不机械改成/3。Document-before-sidecar、sidecar-before-Resource、生产版本metadata不完整、placeholder未materialized或proof/components不一致都是incomplete，不是empty/deleted/committed，也不能从I补齐。

未下载 placeholder 的状态是 not_materialized。需要读取其 bytes 的 operation 返回 source_unavailable/对应 owner unavailable；不能用 size=0、not_found、相同hash或旧locator代替。

### 9.3 ConflictRecord 与版本边界

同步检测到并发 source、placement、lifecycle、identity 或 policy heads 时建立稳定 ConflictRecord。ConflictKey 至少绑定 WorkspaceRef、closed conflict kind、受影响 Ref 集、所有并发 head ChangeId；Ref 和 heads 规范排序、唯一。ConflictId 是对完整 canonical ConflictKey 做域分离 SHA-256 得到的地址，完整 key 仍必须保存并比较，hash不能代替证据。

新FA记录使用ConflictRecord/2并把created cut绑定为Frontier/2；ConflictKey/1、ConflictSubject、ConflictId的`D6-ConflictKey/1` hash domain及排序均不升版。ConflictRecord/1继续按原Frontier/1 decoder/bytes作为历史输入，不允许version=1却按Frontier/2解释。closed shape、字段decoder和read/prepare接口由同一P1 Control正式冻结；Storage只拥有portable存储、版本选择和恢复语义。

state仍含open、resolution_prepared、resolved、superseded。新head使旧open/resolution_prepared记录superseded并建立关联的新key/record；resolution绑定exact current key/heads与owner-specific plan，最终写仍编译为原D3/D6 typed request。resolved/superseded历史不重写成新version，也不按当前Frontier重新hash。

至少覆盖：

- source/source：保留两个 head 和共同 base；非重叠也只可生成明确 merge proposal，不后台自动 commit；
- create/create：两个不同 identity 均保留；同 parent order 冲突显式解决，不按 UUID/mtime 伪造用户顺序；
- move/edit：identity 相同且维度真正独立时可组合，但须重新验证 destination structure/policy；否则冲突；
- move/move 或 cycle：不自动选一个 parent，不挂 root；
- Trash/edit：保留 edit bytes 与 Trash intent，不默认 delete-wins；
- purge/restore：已证明 tombstone 不复活；遗留 bytes 只能作为 fresh-copy 输入；
- duplicate Ref/birth mismatch：identity_collision，普通 resolver 不选“第一个”；解决可保留一边 identity、另一边通过 D3 fresh copy/adopt 流程；
- policy conflict：不做 allow union；在显式解决前按不扩大权限的安全交集/deny 优先提供受限读取，管理写暂停；
- sidecar/body partial：incomplete，不 mint identity；
- placeholder：not_materialized。

存在某个 ConflictRecord 不冻结整个 Workspace。用户可明确打开某分支并继续形成后继 branch；没有 branch 选择的 ordinary read 不得随机取一个 current。

### 9.4 Trash 与 purge

多设备 ordinary delete 只执行 Trash。local Trash receipt 证明本操作已知 closure、owner-local membership、placement 与实际 source effects；它不能伪造全 Workspace inbound reference enumeration。

permanent purge 保持强操作：要求完整相关 inbound/owner closure、complete semantic proof，并要求登记 replica 对 purge frontier 已确认，或显式 retire 无法参与的 replica。retired replica 以后带回旧 bytes 只能进入 conflict/reconciliation，不能恢复旧 ReplicaEpoch 或 tombstoned identity。Purge 不承诺从已经复制到其它设备/backup 的历史字节安全擦除。

## 10. Index、Query 完整性与大库启动

首开路径分阶段，不使用全库完整 hash 作为启动前提：

1. T_first_open：读取 workspace root/portable metadata 入口，列出已发现目录和活动目标；
2. T_first_edit：打开活动 source，建立 Draft；
3. T_first_reliable_save：只有使用 strict 保护完成实际安装并经 P seal 耐久封存，才计入 reliable；observed_only 路径产生的 durable_observed_only 另行计量，绝不计入旧 strict 指标 T_first_reliable_save；
4. T_full_search_ready：指定 search profile/范围的 complete coverage 成立；
5. T_OCR_ready：指定附件/模型/版本的 OCR 层完整或明确失败。

inventory/metadata 枚举、D2 parse、typed index、search candidates、attachment extraction、OCR 使用有界队列、有限在途 bytes、批量 index transaction 和可续建 checkpoint。10k/100k/1M 小文件与几十 GB 附件/正文必须分别实测；本架构不给秒数保证。

Exact source scan 是正确性 baseline。候选 index 命中后必须回读准确 current SourceObservation/source version 进行最终判断；召回不完整的 tokenizer/gram 不能用于 exact/NFC/regex 的 complete proof。D7 complete Query 需要完整 authorized execution、query_scan及所有适用正/负DependencyProof；building/partial index只能作为明确已扫描范围的探索视图，不能签发完整 ResultHandle/ActionEvidence。

完整范围proof必须来自真实owner的一致枚举或可证明无缺口的连续变更消费，并在发布前重验当前授权与依赖。empty范围不是“index没有row”：同样须证明枚举入口、全部适用目录/分片、隐藏策略与continuity stamp。无法证明完整、event gap、placeholder未materialized或owner decoder不可用时，complete consumer返回对应unavailable/reset，不返回empty。

I重建不复活old proof。原P/M correctness range epoch/revision和连续消费事实仍完整时，重建I只是恢复cache；这些事实实际丢失时，旧proof失效，须在当前授权下完整重新枚举并建立新range epoch。只依赖真实局部证据的ordinary save或D3 replica_local操作不因无关complete-query proof不可用而永久停止。

## 11. Server 多用户与实时协作接入

Server 托管 Workspace 仍有唯一持久 CommitDomain/提交持有者，跨全部 Server 进程/实例/failover 必须 fence 旧 writer 对**durable control 和作者文件**的写能力。只让旧 SQLite transaction 失败、却允许旧进程继续 rename author files，不算 fencing。

多个认证用户可以并行：

- 读取不同/相同文档；
- 持有各自 Draft；
- 执行 D2 parse、D7 Query/prepare；
- 编辑不同文档；
- 编辑同一文档并在不同 base 上保留提案。

不同文档的 author commit 只锁其 write set 和真实依赖范围，按固定 key 顺序取得范围锁；P 的最终 seal transaction 可以短暂串行化 commitSequence。无关文档不得仅因一个全局 coarse sequence 改变就自动 abort。

同文档非实时阶段：A seal 新 version 后，B 的 Draft 不丢失；B 的旧 base prepare/commit 返回 stale/conflict，并进入 base/current/proposed 三方流程。

G2 后实时会话的 D6 接入合同现在冻结，但不冻结 OT/CRDT 算法：

- session 绑定 Workspace/Node、committed Base SourceVersion、
  sessionEpoch、participant principal/session 和有序 client input sequence；
- receive ack、broadcast、durable checkpoint 三者分离；receive/broadcast 不叫 saved；
- 参与完整 source session 需要自身完整 source_read；一个用户不能借另一参与者资格看到隐藏 bytes；
- IME preedit 不成为 shared author op；composition 最终确认至多形成一个 Draft input transaction；
- checkpoint 由显式 save/collaboration-checkpoint 触发，不能每键 author commit；
- checkpoint 对合成后的 exact source 重做当前每个尚未提交贡献者的 authorization、actual footprint、D2/D4/D5适用门和依赖；
- 撤权在 seal 前线性化获胜时，该主体未提交贡献不能借别人的身份提交；相关 input 保留成待重确认 proposal；
- committed update 只从 saved decision/outbox 广播；每个接收者再次经过当前读取 gate；
- event gap、reconnect、mapping proof缺失时 reset/rebase，不按同文字猜 selection；
- collaboration transient op log 只为活跃 session/恢复使用，受 TTL/capacity 限制，不成为第二 durable Document source。

未来 OT/CRDT adapter 必须证明输入归属、合成结果、source mapping、selection mapping、重复/迟到处理和收敛；证明失败保留所有用户输入并停止该 checkpoint，不以单用户锁或静默丢 op 伪装成功。D1 原 G2 后续发布边界不因此提前。

## 12. History、backup、pins 与修复

Portable backup 由一个明确 Frontier 的 ordinary files + portable metadata closed snapshot 组成；它不自动包含可消费的 execution control authority。需要灾难恢复 global execution responsibility 时必须另有受保护 control backup，并在接管时证明旧执行者失效和 ledger/charges/unknown continuity。

History 可以保存旧 source/resource bytes，但它是明确 retention product，不是 current source。普通 purge resolver 不可从 history 复活同 identity；用户可明确从 history bytes 建 fresh content。tombstone/no-reuse facts按 D3 保留。

repair 只在显式 repair/audit 权限下运行。它可以：

- 检查 portable record graph、component digests、FileBinding、Frontier holes；
- 恢复 P 已证明的原 planned install；
- 将第三种 bytes 保存为 conflict branch；
- 重建 derived index；
- 重新 materialize缺失但有合法 backup 的 ordinary file。

repair 不可：

- 从 current file 猜原 receipt/ApprovalUse/Money；
- 以相同 digest 认定原 install 成功；
- 自动删除 conflicting bytes；
- 重新分配旧 Ref；
- 用 backup 绕过 purge/tombstone；
- 无 continuity 时重发 external effect。

## 13. 预算、资源与执行责任

BudgetBinding、attempt allowance 和 persistent charge 继续是执行控制事实。work batch 先 charge 后执行；crash 不退款；attempt/clock epoch 无法证明时保持 planned/paused，不伪造 terminal business failure。

Pin/capacity 预算至少分开：recovery pins、conflict pins、preview/query pins、import/export pins、user history。protected planned/unknown pins 不因 preview TTL 清理。容量不足时可以拒绝新 prepare/install，不能通过删除 last-reference pin 取得空间。

global execution responsibility 的最小 continuity 包括：

- canonical original request/decision/receipt/error；
- current lease/claim/standing approval consumption；
- Money budget lineage：Run、Lease、Automation、Workspace、
  deployment 及 reservation/charged/refund evidence；
- external request/send/result frozen binding 与 unknown state；
- stop/emergency state；
- sourceOccurrenceKey 等自动化连续性所需 evidence。

普通 replica registration、FileBinding、ChangeId/Frontier 不提供上述消费资格。sourceOccurrenceKey 不能从 path、line、same Field key、same digest 或“唯一候选”重建；只有受管连续 change chain 能证明延续。观察 gap 后依赖 continuity 的 automation 暂停并要求显式 rebind。stop 先 durable 阻止新派发/消费，再处理在途取消；无法证明 provider effect 已撤销时仍为 unknown。

## 14. 与 D3/D4/D5、P1 producer 与后续 consumer 的版本化协调边界

固定 C 已包含 D3 wire12 及 D4/D5 当前 SourceObservation/Frontier/WriteProtection consumer后像；它们仍是未接受、未激活候选。本P1新增的生产域revision规则、SourceRevisionPlan、revision-token profile/2、DependencyKey具体闭合、ContentCompletionProof/3、ConflictRecord/2和U5 range/frontier细化尚未被全部owner消费，不能把现有A/B/C解释为自动支持这些producer规则。

同一P1后续须由D6 Control正式冻结closed类型/decoder，Lexicon/Registry/Impact同步术语与验收，PROPOSAL/replacements登记真实changedSections/versionChanges；P2由D3/D4消费版本/range key/recovery分流，P3由D5消费locator/cut规则。对应owner后像未实际形成前，依赖这些新定义的strong路径保持owner_update_required/proof_unavailable；既定ordinary文件读取、Draft、完整合格人工整源保存以及不依赖缺失strong range的局部离线操作不得因此永久禁用。

D7 的完整查询/准备/效果（Query/Prepared/Effects）、D8 的普通编辑/草稿/输入法组合/撤销（Draft/IME/Undo）、D9 的构造/导入/导出（construction/import/export），以及 D10 的批准/接收方目标载荷/来源发生键/费用/未知责任（approval/recipient-target-payload/sourceOccurrenceKey/Money/unknown），仍须通过后续消费者门禁。Storage 摘要不得擅自发明这些协议的闭合字段，也不得把 partial/semantic_pending 提升为强操作成功。

旧 D6 wire1、Policy/1/2、SourceVersion/1、Frontier/1、InstallationNotice/1、ContentCompletionProof/1及已承诺的ContentCompletionProof/2、ConflictRecord/1、旧revision-token profile、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1及其saved/planned decisions/pins均按原decoder、bytes、authorization/retention/continuity恢复重放。新/3、ConflictRecord/2、profile/2和SourceRevisionPlan只用于明确新版本路径；不得重编码旧receipt、用新proof升级旧r5、追溯删旧pins或从相同hash推断新资格。

这些producer修订仍须完整双语后像、正/负/unknown/recovery证据、fresh联合全量审查和协调接受；本Storage artifact、作者自查或文档CI均不构成激活。

## 15. 接受边界

D6-FA-r01 目前只是作者部分联合候选：

- 没有产品代码实现；
- 没有文件系统 conditional-replace/exclusive primitive 实机证明；
- 没有 Server multi-user/real-time conformance；
- 没有 10k/100k/1M 文件或几十 GB 性能数据；
- 没有 D3/D4/D5/D7/D8/D9/D10 全部 consumer afterimage；
- 没有 fresh 独立联合审查。

任何 documentation check/CI 只证明其明确检查项，不构成上述证据。完整后继候选须以固定 S 49 原输入、全部实际 replacement owner、新 D10 十八份共同接受。


### PL-IR-01 稳定地址的保留边界

对当前新 profile，每个 sealed managed SourceVersion/2 即使提交时尚无 Locator，也必须由 winning plan/seal 选出唯一 canonical d6_source_revision/2 binding。exact canonical RevisionTokenSealArtifact/1 bytes 是 immutable Portable Workspace Metadata；生产端 P record 同时保留对应 RevisionTokenSealOutboxItem/1 与 portable_metadata pin，直到原 publication/retry/recovery 及全部 portable-address/last-reference 义务允许释放。已锚定 WorkspaceTrustRootDeclaration/1、为任何仍保留 artifact 重建 history_at 所需的每条 WorkspaceTrustDeclaration/1，以及把 declaration 绑定到 activation ChangeId 的 policy-bundle version/ChangeRecord evidence，都是 public-history last reference，不得提前 GC。rotation/revocation 可在不再有合法 planned signer 后退役或销毁旧 private DomainSealKeyHandle；历史验签从不需要该 private key。Derived Index 可缓存 token→version/current-trust projection，但不是真实性 owner。I 重建只重新验证保留 anchor/public history/artifact signature/cross-fields 并恢复 cache。P recovery 即使 rotate/revoke/key loss 后也只重发 exact pinned artifact bytes，绝不重签。未提交/失败 P attempt 算出的 signature 只是 staging，不能冒充 seal；post-install planned/recovery_unknown 保留原 plan/pins/trust-cut 责任，后续 commit 前仍须重新通过 authorize_new_sign。required admission 前真实 artifact/public trust history 丢失，不能从相同 bytes、digest、SourceStamp、SourceVersion、revision 或 rescan 修复。不新增第二作者库、ledger、CAS、Notice component 或 CP3 member。
