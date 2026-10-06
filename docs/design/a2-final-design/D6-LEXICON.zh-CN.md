---
source_language: zh-CN
translation_status: source
---

[English](D6-LEXICON.md)

# A2 D6 术语与命名词表

状态：A2 D6 术语作者候选；除具名 current definition successor 外，public concept 的 exact ID/ownedNames/firstFreeze 全部保留。

本文件是机器注册表 source.json 的可读后像。受控名称只表达所属概念，不自动授能力、身份、事务 owner 或实现状态。D3 Typed Reference、OperationId、Authority、lifecycle；D4 Field/Registry/Calendar；D7 Query/Action/Effect；D8 Editor；D9 conversion；D10 Agent/approval/provider 各保持原 owner。

## 1. 完整公共概念表

| conceptId | 中文 | English | ownedNames | D6-FA-r01 定义 | 明确排除 | firstFreeze |
|---|---|---|---|---|---|---|
| weftext.term.storage-domain | 事务存储域 | Storage Domain | \`StorageDomain\`<br>\`storageDomain\` | 一个CommitDomain内的受管提交/恢复范围；文件作者字节、portable metadata与库外durable control可跨不同物理介质组合，但不能产生第二可写真相 | 不是Workspace identity、不是单一SQLite正文库、不是跨CommitDomain全局事务 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.authority-store | 权威存储 | Authority Store | \`AuthorityStore\`<br>\`authorityStore\` | Core对当前author/portable-control truth的逻辑权威视图；D6-FA-r01中当前Document/Resource bytes来自普通文件，portable identity/structure/policy来自portable metadata，执行decision来自独立durable control | 不再等于保存全库正文的一份SQLite；不是index、history或export artifact | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.storage-fence | 物理写栅栏 | Storage Fence | \`StorageFence\`<br>\`storageFence\` | CommitDomain接管/写入资格的耐久单调栅栏；Server须同时fence durable control与作者文件写入，replica还绑定当前backend/observation资格 | 不是D3 authority generation、SourceVersion revision、文件hash或advisory lock本身 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.authorized-cut | 一致授权读取切点 | Authorized Cut | \`AuthorizedCut\`<br>\`authorizedCut\`<br>\`openAuthorizedCut\`<br>\`subscribeCut\` | 绑定CommitDomain、Frontier、observation epoch、源/控制版本与认证上下文的受权读取切点；scope可为局部或complete | 不授永不过期读取权；局部cut不自动证明全Workspace负范围 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.mutation-footprint | 实际修改范围 | Mutation Footprint | \`MutationFootprint\`<br>\`mutationFootprint\` | 真实before/proposed中全部实际语义修改的受信投影 | 不是客户端自报patch范围 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.prepared-intent | 准备意图 | Prepared Intent | \`PreparedIntent\`<br>\`preparedIntent\`<br>\`d6_prepare_request\`<br>\`d6_prepared_intent\` | Core保存的immutable完整意图、源计划、依赖与预算 | 不是D3 planned、content reservation或自由回调 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.plan-token | 计划控制令牌 | Plan Token | \`PlanToken\`<br>\`planToken\` | 选择一份受管PreparedIntent的opaque运输token | 不是identity、授权凭证或可编辑计划 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-handle | 完整结果句柄 | Result Handle | \`ResultHandle\`<br>\`resultToken\`<br>\`publishResult\` | 绑定完整Query/cut/schema/audience/期限的受管结果及其token投影 | 不是Node/Record/View identity或D7 row selector | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-cursor | 结果游标 | Result Cursor | \`ResultCursor\`<br>\`cursorToken\`<br>\`nextCursorToken\` | 绑定同一result/epoch/运输位置的opaque游标 | 不是semantic limit、row identity或独立结果 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.budget-binding | 预算绑定 | Budget Binding | \`BudgetBinding\`<br>\`budgetBinding\` | 固定资源规模上限及其可信消耗域 | 不混淆原语义额度、attempt时长和等待墙钟 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.series-scope-configuration | 系列范围配置 | Series Scope Configuration | \`SeriesScopeConfiguration\`<br>\`seriesScopeConfiguration\`<br>\`d6_series_configuration_intent\`<br>\`d6_series_configuration_remove_intent\` | 跨全部periodKey的exact series+scope受管multiplicity配置，绑定D4 Registry及policy；unique按各完整period key分别检查 | 不是每次create自行选择unique/many | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.import-job | 导入作业 | Import Job | \`ImportJob\`<br>\`importJob\`<br>\`stageInput\`<br>\`planAtomicGroups\`<br>\`commitImportBatch\` | 有限输入、不可拆groups、显式batch及真实committed前缀 | 不是全输入全局原子事务、普通import的身份合并 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.source-checkout | 编辑检出 | Source Checkout | \`SourceCheckout\`<br>\`sourceCheckout\` | 历史/显式导出编辑场景中的source proposal呈现；D6-FA-r01普通文件型Workspace的当前.adoc本身是Document author bytes，不再把日常文件编辑称为checkout | 不是普通文件型Workspace的第二活动author source，也不授write/identity | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.source-version | 源版本 | Source Version | \`SourceVersion\`<br>\`sourceVersions\`<br>\`sourceRevision\`<br>\`readSource\`<br>\`DocumentRevision\`<br>\`ResourceRevision\`<br>\`AnnotationRevision\` | SourceVersion/2 绑定完整 EntityRef 与生产 CommitDomain、生产 observationEpoch；managed 分支还绑定该生产域连续已封存 managed 历史中的 revision 与 ChangeId，external 分支绑定 externalSequence 且没有 managed revision/ChangeId。managed revision 由该生产域实体的 H(D,E)+1 checked 分配，各生产域分别续接；当前 observerDomain 由 SourceObservation/1 另行绑定 | 不是只靠摘要防 ABA；externalSequence 不是 managed revision；跨生产域裸 revision 不可比较；旧 SourceVersion/1 只服务历史 decoder/replay | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.principal-context | 认证主体上下文 | Principal Context | \`PrincipalContext\`<br>\`principalContext\` | host/D10认证的主体、session、delegation与policy generation | 不是客户端JSON自行声明的principal | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.commit-protocol | 控制提交协议 | Control Commit Protocol | \`D6ControlCommit\`<br>\`d6_commit_request\`<br>\`d6_commit_receipt\`<br>\`d6_error\`<br>\`commitBoundPlan\`<br>\`replayOperation\` | D6 own v2 ledger的闭合prepare/plan/file-install/durable-seal/receipt/replay协议；operation key含CommitDomain，D3 identity/lifecycle仍走其owner wire | 不包装D3绕过其阶段；不是文件hash CAS；不是第二Action领域 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.effects-token | 效果清单令牌 | Effects Token | \`EffectsToken\`<br>\`effectsToken\` | 引用同一decision保存的完整效果清单的运输token | 不是第二作者源、独立成功receipt或写能力 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-page-protocol | 结果分页协议 | Result Page Protocol | \`D6ResultPage\`<br>\`d6_result_page_request\`<br>\`d6_result_page\`<br>\`d6_result_error\`<br>\`readResultPage\` | 完整结果的当前受权分页与闭合错误 | 不是D7 row schema或streaming部分成功 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.attempt-allowance | 执行次数额度 | Attempt Allowance | \`AttemptAllowance\`<br>\`attemptAllowance\`<br>\`d6_attempt_allowance_intent\` | 绑定planned decision的有限执行attempt资格及管理意图 | 不扩大原语义预算或退还已耗work | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.execution-resource-summary | 执行资源摘要 | Execution Resource Summary | \`ExecutionResourceSummary\`<br>\`executionResourceSummary\` | 向当前Workspace policy_admin交付的受管计划占用与暂停类别 | 不是普通用户可枚举的作者内容或自动放弃权限 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.dependency-proof | 依赖完整性证明 | Dependency Completeness Proof | \`DependencyProof\`<br>\`dependencyProof\`<br>\`readCompleteScope\`<br>\`validateDependencies\` | DependencyProof/3 以十五类 closed DependencyKey/3、每范围 stamp 的 epoch/revision、必要 evidence pins 及正负范围证据绑定 Workspace/CommitDomain/base Frontier；完整性来自当前授权下的一致 snapshot/range barrier，或连续无漏变更链加最终复验。Derived Index 只缓存候选，受保护连续性事实仍完整时删除 I 不会凭空使证明失效 | 不是客户端 readSet 声明、free JSON、building/partial index 或“没有命中”；unknown/partial 不得证明 empty，Frontier 数字增长也不自动证明扩展与真实依赖无关 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.foreign-version-comparison | 外部版本比较 | Foreign Version Comparison | \`ForeignVersionComparison\`<br>\`compareForeignVersion\` | 由D9/D10固定比较器证明外部版本关系的纯读取 | 不重定义D3 SourceBinding或OriginBinding；比较不执行绑定退役或任何持久修改 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.binding-retirement-intent | 绑定退役意图 | Binding Retirement Intent | \`BindingRetirementIntent\`<br>\`retireBinding\` | 显式受权的既有SourceBinding/OriginBinding控制退役意图，由所属D9/D10输入适配并同事务提交 | 不重新定义D3绑定、不由只读版本比较自动触发 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.control-inspection-protocol | 控制状态读取协议 | Control Inspection Protocol | \`ControlInspection\`<br>\`d6_control_read_request\`<br>\`d6_control_state\`<br>\`d6_control_error\` | 向当前Workspace policy_admin读取同cut配置/CAS版本与执行资源摘要的closed协议 | 不是通用作者源API或OperationId枚举 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.byte-handle | 资源字节句柄 | Resource Byte Handle | \`ByteHandle\`<br>\`ByteHandleRecord\`<br>\`d6_byte_handle\`<br>\`handleToken\` | D2 Resource snapshot中绑定已提交完整ResourceRef/版本/cut/descriptor/audience/pin/期限/预算的只读字节句柄 | 不是上传staging、identity、Query result/cursor、plan、effects、权限凭证或latest指针 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.byte-read-protocol | 资源字节读取协议 | Resource Byte Read Protocol | \`ByteReadProtocol\`<br>\`readResourceBytes\`<br>\`d6_byte_read_request\`<br>\`d6_byte_chunk\`<br>\`d6_byte_error\` | 固定ByteHandle上的closed offset/maxBytes读取、完整chunk与准确EOF、当前授权和共享有界计费 | 不是D7 row协议、repair Document envelope、自由URL/路径读取或新增提交入口 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.observation-scope | 潜在约束观察范围 | Constraint Observation Scope | \`ObservationScope\`<br>\`observationScope\`<br>\`d6_observation_scope\` | 在作者/范围读取前固定的结果观察上界，覆盖空范围并按当前权限证明 | 不是实际写集或内部完整读取证明 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.issuer-control-policy | 签发域控制权限 | Issuer Control Policy | \`IssuerControlPolicy\`<br>\`issuerControlPolicy\`<br>\`d6_issuer_control_policy\` | issuer自身allocate/administer授权及当前revision | 不是身份认证或Workspace内容授权 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.workspace-bootstrap-plan | 工作区初始控制计划 | Workspace Bootstrap Plan | \`WorkspaceBootstrapPlan\`<br>\`workspaceBootstrapPlan\`<br>\`d6_workspace_bootstrap_plan\` | 当前 Plan/3 在原 D3 计划内同时固定 WorkspaceTrustGenesis/1、首份 policy/Registry/Calendar 与主体映射，并与 activation 共用唯一原 P seal；Plan/2 是未激活候选前身 | 不是第二decision、可编辑请求、package trust 或永久管理员 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.bootstrap-profile | 工作区初始控制配置 | Workspace Bootstrap Profile | \`WorkspaceBootstrapProfile\`<br>\`bootstrapProfile\`<br>\`d6_bootstrap_profile\` | family不可变的Registry种子和显式初始范围/multiplicity选择 | 不是D2 Profile或Facet，不是每次重试最新配置 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.calendar-period-scope-binding | 历法周期范围绑定 | Calendar Period Scope Binding | \`CalendarPeriodScopeBinding\`<br>\`periodScopeBindings\`<br>\`d6_period_scope_intent\` | period Node到明确D4范围的必要控制选择及独立ABA版本 | 不是作者series/periodKey副本、Field或content identity | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.source-envelope-state-capability | 源包络状态能力 | Source Envelope State Capability | \`SourceEnvelopeStateCapability\`<br>\`source_envelope_state\` | Policy/3（及历史Policy/2原路径）显式授予指定源版本/容量/聚合有效性及编辑outcome观察 | 不授body/其它Field/decision内容读取；不由source或Field权限蕴含 | D7 coordinated revision02 candidate; not activated |
| weftext.term.commit-sequence-state-capability | 提交序号状态能力 | Commit Sequence State Capability | \`CommitSequenceStateCapability\`<br>\`commit_sequence_state\` | Policy/3显式授予指定CommitDomain的D3/D6提交活动序号观察；历史Policy/2 consumer保持其原Workspace-wide语义 | 不提供跨离线replica虚构全局序；不授decision内容读取 | D7 coordinated revision02 candidate; not activated |
| weftext.term.commit-domain | 提交域 | Commit Domain | \`CommitDomain,commitDomain\` | 一次D3/D6幂等decision、版本与恢复的命名空间；replica由WorkspaceRef+ReplicaEpoch，server由WorkspaceRef+AuthorityInstanceId闭合 | 不是Workspace identity、global execution responsibility或跨设备余额 | D6-FA-r01 candidate; not activated |
| weftext.term.replica-epoch | 副本世代 | Replica Epoch | \`ReplicaEpoch,replicaEpoch\` | Core显式登记一个文件副本普通写入资格时mint且同Workspace不复用的UUIDv4 | 不是AuthorityInstanceId、设备硬件ID或Automation lease | D6-FA-r01 candidate; not activated |
| weftext.term.change-id | 内容变更标识 | Change ID | \`ChangeId,changeId\` | CommitDomain+从1连续的content change sequence，标识portable内容版本因果点 | 不是EntityRef、OperationId、sourceOccurrenceKey或全局时间 | D6-FA-r01 candidate; not activated |
| weftext.term.frontier | 内容前沿 | Frontier | \`Frontier,frontier\` | 每CommitDomain至多一个最大已接纳ChangeId的规范向量，表达已知连续内容前缀 | 不是全局提交序、Query row set或同步provider状态 | D6-FA-r01 candidate; not activated |
| weftext.term.portable-workspace-metadata | 可移植工作区元数据 | Portable Workspace Metadata | \`PortableWorkspaceMetadata,portableMetadata\` | 库内唯一承载identity/structure/lifecycle/shared policy/trust/portable change与conflict记录的版本化文件集合 | 不保存全库current正文副本；不是durable execution ledger或index | D6-FA-r01 candidate; not activated |
| weftext.term.durable-control-store | 耐久控制库 | Durable Control Store | \`DurableControlStore,durableControlStore\` | 库外独立SQLite及私有pins，保存不可从笔记重建的decision/recovery/unknown/approval/claim/Money执行责任 | 不是Document/Resource current bytes、portable parent/order或可删index | D6-FA-r01 candidate; not activated |
| weftext.term.derived-index-store | 派生索引库 | Derived Index Store | \`DerivedIndexStore,derivedIndexStore\` | 库外设备本地可删SQLite，按版本/覆盖记录metadata、parser/search/OCR等可重建事实 | 不是author source、identity/policy owner或全集证明本身 | D6-FA-r01 candidate; not activated |
| weftext.term.content-guarantee | 内容提交保证 | Content Guarantee | \`ContentGuarantee,contentGuarantee\` | 提交结果的闭集 replica_local 或 managed_atomic，描述本 CommitDomain 对固定 portable write set 的安装/受管屏障保证；它与 WriteProtection 的 strict、observed_only 保护轴正交，managed_atomic 及所有强路径保持 strict | 不扩大授权；不声称外部工具跨文件瞬时原子或离线副本无并发；observed_only 不能把结构、强 Action、Automation、Approval 或 Money 路径降级 | D6-FA-r01 candidate; not activated |
| weftext.term.semantic-state | 语义状态 | Semantic State | \`SemanticState,semanticState\` | D2-valid受管source的complete_semantics或semantic_pending及其明确未证明obligations | external_invalid不属于成功SemanticState；pending不等于D4/D5全集通过 | D6-FA-r01 candidate; not activated |
| weftext.term.installation-notice | 安装告知记录 | Installation Notice | \`InstallationNotice,installationNotice\` | 触碰 portable current 前耐久写入的 InstallationNotice/2，绑定 DecisionKey、guarantee、WriteProtection、原 baseFrontier 以及完整 component before/after；baseFrontier 可以包含此前已经封存的历史 ChangeId，但记录本身不含这个尚未 seal decision 的新 ChangeId | 不是 commit proof、receipt、approval、Money/external payload 或 execution authority；历史 base head 中的旧 ChangeId 不等于本 decision 已成功 | D6-FA-r01 candidate; not activated |
| weftext.term.content-completion-proof | 内容完成证明 | Content Completion Proof | \`ContentCompletionProof,contentCompletionProof\` | 新生产路径的 ContentCompletionProof/3 是 P seal 后发布到 portable metadata 的不可变证明，绑定同一 DecisionKey/ChangeId、actual frontierBefore/frontierAfter、actual components，并以生产 SourceVersion/2 或 absent 表达 sourceChanges；接收端验证连续 sealed 链后建立自己的 current SourceObservation | 不授 receipt 读取、Money/approval 消费或 execution takeover；历史 ContentCompletionProof/1,/2 保留原 bytes/decoder/pins，/2 的 SourceVersionRef sourceChanges 不按 /3 重解 | D6-FA-r01 candidate; not activated |
| weftext.term.conflict-record | 冲突记录 | Conflict Record | \`ConflictRecord,conflictRecord,ConflictKey,conflictId\` | 当前 new-FA ConflictRecord/2 保留既有 ConflictKey/1/ConflictId。D6 wire3 ConflictResolution/2 精确选择 source/head 或 WorkspaceAuthorizationBundleAddress/1；递归保留直接 revoke/rotate compromise 与旧 inherited carry 的原始 activation cut；fresh recovery 只限受影响 domain/profile，并在唯一 d6_commit_request/2 前把 branch evidence 与 preview 冻结进 PreparedIntent/2。placement/lifecycle/identity 仍导航到 D3 | 不是自动 merge、LWW、arrival-order、allow-union、通用 trust add、one-stage resolver、第二 ledger/CAS/submit 或 D3 bypass；不得伪造未 seal head，历史 Record/1 保留原 decoder | D6-FA-r01 candidate; not activated |
| weftext.term.reliable-save-state | 可靠保存状态 | Reliable Save State | \`ReliableSaveState,reliableSaveState\` | 闭集区分 not_saved、strict 安装并 P seal 后的 reliable、完全满足弱保护资格并 seal 后的 durable_observed_only，以及不适用文件保存的 not_applicable；prepared/input retained 仍不是 Saved，portable publication 由独立状态读取 | 不等于 Draft 已保存、sync 上传、InstallationNotice 或 ContentCompletionProof 已发布；durable_observed_only 不声称最后检查后未观察的外部 C 不会被 N 覆盖 | D6-FA-r01 candidate; not activated |
| weftext.term.execution-responsibility | 执行责任域 | Execution Responsibility | \`ExecutionResponsibility,ExecutionResponsibilityRecord,executionDomainId\` | 连续持有Automation/ApprovalUse/claim/Money/external-unknown/stop事实的受保护执行域，独立于普通CommitDomain | 不是ReplicaEpoch、Workspace content identity或文件同步资格 | D6-FA-r01 candidate; not activated |

## 2. D6-FA-r01 的关键消歧

### 2.1 Authority Store 不再等于“正文 SQLite”

历史 controlled name AuthorityStore/authorityStore 保留，因为已有 D6/D3/D7 文本与历史 saved evidence引用它。D6-FA-r01 将其定义收窄为 Core 对当前作者/portable-control truth 的逻辑权威视图：

- Document/Resource current bytes：普通文件；
- identity/parent/order/lifecycle/shared policy/trust：Portable Workspace Metadata；
- operation decision/unknown/approval/Money：Durable Control Store；
- index/cache：Derived Index Store。

因此不得从名称 AuthorityStore 推导一个 SQLite 表持有 current body，也不得从 DurableControlStore 推导它能覆盖 portable parent/order。

### 2.2 Storage Domain 与 Commit Domain

StorageDomain 是物理/恢复协调范围的历史公共概念；CommitDomain 是 FA-r01 新幂等/版本命名空间。一个文件型 Workspace 可以把普通文件、portable metadata 与库外 durable control 组合为一个受管提交范围，但 operation key 必须显式包含 CommitDomain。二者都不是 Workspace identity。

ReplicaEpoch 只标识 ordinary replica writer 世代；ExecutionResponsibility 是 Automation/approval/Money 等全局执行责任。ReplicaEpoch 不蕴含 execution responsibility，后者也不阻止无关 ordinary content。

### 2.3 Source Version

SourceVersion/1 的 ownedNames 保留给历史 decoder。SourceVersion/2 是完整生产版本 union：managed 分支保存完整 EntityRef、生产 CommitDomain、生产 observationEpoch、managed revision 与 ChangeId；external 分支保存完整 EntityRef、生产 CommitDomain、生产 observationEpoch 与 externalSequence，并且没有 managed revision 或 ChangeId。SourceVersion/2 的生产域可以不同于当前 operation/observer 域，externalSequence 也永不填入 D3/D4/D5 的 managed sourceRevision。

对生产域 D 和实体 E，H(D,E) 是 D 的连续已封存 managed 生产历史中 E 的最大 revision。只有从该域 birth/registration、P continuity 与已验证 portable sealed history 证明从未为 E 封存 managed 版本时，完整空历史才允许 H=0；缺失、损坏、有 gap 或未知历史都不能当空。新的 managed after 使用 checked H+1，MAX 不 wrap，同一生产域跨生产 observationEpoch 不重置 H。一个 source 首次由另一生产域写入时使用新生产域自己的 H+1，之后跨域往返分别继续各自 H；不同生产域相同 revision 数字不等价。true raw no-op 保留原 SourceVersion/2；纯 placement/lifecycle/control 且 source 未变不增加 H；删除的 after=absent，不创建删除版 SourceVersion 也不推进 H。equal-byte external admission 仍是显式 managed 接纳，按当前生产域 H+1 形成新 managed after。

当前副本或 Server 的读取资格由完整 SourceObservation/1 单独绑定 observerDomain、EntityRef、生产 SourceVersion/2、当前 observer observationEpoch、FileObjectBinding 与 evidence pins。生产 SourceVersion.observationEpoch 与当前 SourceObservation.observationEpoch 属于不同角色，即使数值相同也不合并。watcher/journal gap、external/object replacement、placeholder 状态变化或 discontinuous rematerialization 都会使旧当前观察资格失效；相同生产版本、revision、digest 或最终文本不能恢复旧观察。SourceVersionRef/1 的 sourceToken 只选择这份完整受保护 Observation，不是裸 revision、digest、I cache 或生产 SourceVersion。

只在计划确实会产生新的 managed after 时，P 内部 `SourceRevisionPlan/1` 才建立，并在 C/Q 位置物化前冻结一条拟议 `RevisionTokenBinding/2`。binding 只含 token+`RevisionTokenSource/2`，专职稳定生产地址，不表示当前 observer 资格。winning planning CAS 将该 exact binding 与版本依据、pins 一起冻结。唯一 P seal 对每个真实 managed after 建立一条 closed `RevisionTokenSealAssociation/1`，并封装到一份 signed `RevisionTokenSealArtifact/1`；exact artifact bytes 由 `RevisionTokenSealVerificationKey/1` 选出的历史 portable-trust key 认证，作为 portable metadata pin 保存，并由 `RevisionTokenSealOutboxItem/1` 引用。签名消息使用 domain-separated D3-CJ/3 canonical bytes，因此相同 SourceStamp/SourceVersion/digest 或 sender trust 都不能认证另一 token。loser、aborted plan 或相同 stamp/H+1 复用绝不能借别人的 seal。publication/retry/I rebuild 只复用原 artifact bytes，绝不重签。新的 observer 分别验证该 artifact 与 CP3、交叉核对 exact production version，再独立证明 fresh current `SourceObservation/1`；这可以取得一次新的读取资格，但绝不复活旧 selector、ActionEvidence、Draft/map、PAB 或 PreparedIntent。旧 d6d/d6r/d6a profile、D3 Locator 的 opaque revision-token 词法，以及 D4/D5 的 inner sourceRevision/selector wire 都保持原 owner 与原形状。
### 2.4 Reliable Save 与 Portable Publication

ReliableSaveState 与 ContentGuarantee、SemanticState、WriteProtection 分轴。not_saved 表示尚无文件保存成功；reliable 只来自 strict 安装加 P seal；durable_observed_only 只来自完整满足资格的人工普通文件弱保护保存并 seal；not_applicable 用于没有文件保存效果的路径。prepare 成功、PreparedIntent/input retained、Draft persistence、worker/HTTP success、InstallationNotice 或 sync upload 都不是 Saved，portable publication 也由独立状态表示。

observed_only 只允许受信 interactive_source_save 的人工在 planning 开始前显式选择并冻结，目标恰为一个既有 live Document，保存类型为 ordinary+replica_local，具备完整 source read/replace，author source write set 为空或仅该 Document，没有适用 body/Field/node-control deny，不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity，并且 Draft Base 等于当前选定 SourceObservation。除此之外的 noninteractive、D3 identity/structure/lifecycle、D5 structured、bulk/collection、D7 strong Action、Automation、server checkpoint、Approval 与 Money 路径都保持 strict；strict 失败、known conflict、失权、耐久失败或 strong obligation 失败都不得在 planning 后 fallback 成 observed_only。

在合格 observed_only 中，原 plan 耐久保留实际读取的 before B 与用户输入 N。唯一放宽是最后可信检查之后、安装 N 之前从未被观察的外部 C 可能被 N 覆盖，而且可能没有可恢复副本；后来另一个 C 也可以再次替换 current file，但 B/N 的耐久保留责任不因此消失。已经观察到的 competition、stale Base、watcher gap、third_state 或失权都不属于该放宽；unknown install 保持 recovery_unknown，不能用相同 hash 猜成功。

planned 只恢复同一原冻结 plan：原 InputDescriptor、适用 SourceRevisionPlan/SourceStamp、before/after pins、OperationId、预算/attempt、WriteProtection、InstallationNotice 与安装状态继续沿用，不重新准备第二个 decision，也不预分配本次 ChangeId。saved/committed 则在原 request/fingerprint/continuity 定位后，按原实际 effect/mode 或结果披露范围做当前交付授权，再重放原 receipt/error/effects bytes 或补原版本 publication/outbox；不要求旧 source/Frontier/业务证明仍等于 current，不回写后来的 current source，不重分配版本或重复收费。撤权可以遮蔽交付但不改写历史 decision；unknown 与旧版本 pins/恢复责任也不能被新 wire、I 重建或重新授权抹掉。

新 FA portable decision 在 seal 后使用 ContentCompletionProof/4 发布；publication pending 只补同一已封存 decision 的 proof/outbox。reliable 或 durable_observed_only 都不自动表示 portable published。
### 2.5 Semantic State

complete_semantics 与 semantic_pending 都要求 D2 valid。pending 必须显式列出尚未证明的 closed obligation；不能用“稍后检查”自由字符串。

external_invalid 不是 SemanticState，因为它没有成功 Core author decision。它属于 CurrentSourceState 的外部 raw branch。

semantic_pending 不得被 D7 complete Query/Action、purge、relation/unique/Calendar mutation、自动化写入当 complete。允许的局部消费必须由后续 D4/D5/D7/D8 owner 明确冻结。

## 3. 技术成员归属

- D6 拥有提交域、观察与生产版本的控制类型：CommitDomain、DecisionKey/2、ReplicaEpoch、ChangeId、Frontier/2、SourceVersion/2、SourceObservation/1、SourceVersionRef/1；SourceStamp/1、内部 SourceRevisionPlan/1、RevisionTokenBinding/2 与 d6_source_revision/2 也由 D6 Control 冻结技术形状，但不新增 public conceptId。
- D6 拥有语义与依赖证明载体：SemanticState、ContentGuarantee、ObservationScope/2、DependencyProof/3、DependencyKey/3 与 WriteProtection。DependencyKey/3 的十五类 closed key、排序、stamp 与 D6 持久/校验规则归 D6；StructureRange、D4 relation/calendar 范围、D7 query_scan 等具体枚举算法仍归各自 owner，不接受 free JSON 近似替代。
- D6 拥有安装、完成、冲突与执行连续性类型：InstallationNotice/2、ContentCompletionProof/3、ConflictRecord/2、ReliableSaveState、ExecutionResponsibilityRecord。历史 ContentCompletionProof/1,/2、ConflictRecord/1、Frontier/1、InstallationNotice/1 继续由其原 decoder/bytes/pins 解释，不按新版本静默重解。
- NodeRef/ResourceRef/AnnotationRef、OperationId、AuthorityInstanceId、D3 lifecycle receipt 与 D3 identity/placement/lifecycle producer：D3；D6 不得用 generic commit、SourceRevisionPlan 或 companion 扩大 D3 WriteScope。固定 C 已有 D3 wire12 候选，但其 P2 native descriptor/companion consumer 仍需实际配套与 fresh 接受。
- FieldId、RegistryBinding、relation/Calendar typed semantics：D4；relation_incidence 与 calendar_scope 的具体枚举仍由 D4 owner 提供，D6 只承载 closed dependency key/stamp/pins 与提交边界。
- QuerySpec、ActionSpec、PreparedActionBinding、EffectManifest/EffectBytes 均归 D7 所有；query_scan 也由 D7 负责其扫描语义。完整 Result/Action 所需的 selector、授权世代、正负范围和 reset 规则同样归 D7。
- D7 的 Query 模块负责查询消费者配套，Value-CEL 模块负责值与表达式求值消费者配套，View 模块负责视图消费者配套，Narrow Field 模块负责窄字段资格消费者配套，Definition Transfer 模块负责定义转移消费者配套，Preview-Effects 模块负责预览与效果消费者配套，Execution-Action 模块负责执行与动作消费者配套，Prepared 模块负责准备态消费者配套，Scenarios 模块负责场景处置消费者配套，Lexicon-Registry 模块负责术语与注册表消费者配套，Impact 模块负责实现影响与测试边界配套；这些模块都仍需在后续形成完整的消费者后像。
- Draft/Edit Map/IME/Undo/Source-Live-Read：D8；ImportJob 转换输入与 ExportPlan/Publication：D9，D6 只拥有耐久控制、pins 与预算边界。
- ApprovalUse、sourceOccurrenceKey、ToolValue、Money 各级谱系、Agent/Automation/connector external effect：D10/相应 Money owner；D6 只保存连续耐久 container 与 no-double-consume/no-replay 边界，不能用 execution_resource 或 CommitDomain 替代 D10 Run/Lease/Automation/Money/unknown 合同。
## 4. Policy/3 能力受控映射

Policy/3 沿用 Policy/2全部能力，并新增：

| capability | 语义 | 不蕴含 |
|---|---|---|
| replica_register | 显式登记新ReplicaEpoch | source read/write、execution takeover |
| replica_retire | 退役ordinary replica writer epoch | 删除其历史、Money退款 |
| conflict_read | 读取已获state scope的ConflictRecord | conflict source bytes |
| conflict_resolve | 进入对应owner resolution prepare | source/policy/D3 write |
| structure_state | 观察 portable parent/order 与结构范围 | source/Field/write |
| portable_frontier_state | 观察完整 Frontier/2 | decision/source/write |
| execution_custody_admin | 管理执行责任连续性/接管 | 新approval、扩Money、作者写 |
| d10_control_self | 有限管理自己的 D10 工作区控制记录 | 作者内容、资源或部署权限 |

commit_sequence_state 在 Policy/3 中只观察指定 CommitDomain 的 domainCommitSequence。旧 Policy/2 consumer 的 Workspace-wide定义只保留历史路径，禁止把它补到新离线replica模型。

## 5. 碰撞、别名与迁移

1. 不新增 local workspace id、device workspace id、sync id 作为内容 identity。
2. ReplicaEpoch、AuthorityInstanceId、executionDomainId 都是不同 UUID domain，不可互换。
3. ChangeId 不得简称 revision、commit id、operation id 或 sync token。
4. Frontier 不得称 global latest/version；它是多domain因果前沿。
5. ConflictId 是 ConflictKey 的稳定地址，不是独立 durable content identity。
6. DurableControlStore 不得简称 index；DerivedIndexStore 不得简称 database authority。
7. save 文案必须区分 Draft/input retained、strict reliable author save、observed_only ordinary-file save、portable published；UI/CLI 不能把这些状态折叠成一个成功。
8. 新协议没有旧别名兼容。历史 saved evidence中的旧controlled name保留，不做文本迁移或删除。

## G0-A Write Protection 与技术版本边界

Write Protection / 写入保护级别由 D6 storage/control 拥有。closed enum 仍只有 strict 与 observed_only；受控 owned names 为 WriteProtection、writeProtection，locale 为 storage.write_protection。它与 ContentGuarantee、SemanticState、ReliableSaveState 分轴，不是权限、用户确认、Query 完整性、ApprovalUse 或 CAS。本轮不因其它内部技术类型再增加 public conceptId；既有 Write Protection concept identity、owned names 与 firstFreeze 保持不变。

observed_only 只允许受信人工对恰一个既有 live Document 做 ordinary+replica_local 整源保存：必须完整 source read/replace，没有适用 body/Field/node-control deny，author source write set 为空或仅该 Document，不修改 identity、parent/order、lifecycle、shared policy、Registry、Calendar scope 或其它 entity，且 Draft Base 等于当前 SourceObservation；人工必须在 planning 开始前显式选择并冻结。已观察竞争、stale Base、watcher gap、失权、unknown install 或其它资格失败不受豁免。strict/managed_atomic、结构或多对象修改、D5 structured、bulk/collection、D7 strong Action、Automation、Approval 与 Money 都不得降为 observed_only。

当前新生产路径使用 Frontier/2、SourceVersion/2、SourceObservation/1、SourceVersionRef/1、ObservationScope/2、DependencyProof/3 与十五类 DependencyKey/3、SourceStamp/1、内部 SourceRevisionPlan/1、RevisionTokenBinding/2+d6_source_revision/2、RevisionTokenSealAssociation/1、RevisionTokenSealArtifact/1、RevisionTokenSealVerificationKey/1、RevisionTokenSealKey/1、RevisionTokenSealOutboxItem/1、InstallationNotice/3、ContentCompletionProof/4 和 ConflictRecord/2。InstallationNotice/3 只绑定原 DecisionKey/baseFrontier/WriteProtection/components，不保存尚未 seal 的本 decision 新 ChangeId；ContentCompletionProof/4 在 seal 后绑定真实 ChangeId、生产 SourceVersion sourceChanges 和 actual pre/post Frontier；ConflictRecord/2 使用 Frontier/2，而 ConflictKey/1/ConflictId/hash domain 不升版。

Frontier/2 只证明连续 sealed 因果前缀，不证明 Query 全集、payload 已下载或 Registry/index 完整。scope_dependencies 只允许具有完整连续 sealed 证据且已证明与原 source/control/auth/正负范围无关的非回退扩展；真实依赖变化、unknown gap 或 partial proof 都不能被洗成无关。Derived Index 删除或重建本身不会破坏仍完整保存在真实 P/M owner 中的范围连续性事实，但真正丢失 proof/continuity 时必须新建 epoch，不能由相同 hash 复活旧证明。

历史 Frontier/1、InstallationNotice/1、ContentCompletionProof/1,/2、ConflictRecord/1、旧 revision-token profile 与旧 saved/planned/unknown records 继续原 decoder/bytes/pins/授权/连续性。ContentCompletionProof/2 的 SourceVersionRef sourceChanges 不迁成 /3 的生产 SourceVersion，ConflictRecord/1 的 createdAtFrontier 不按 Frontier/2 重解。
## 6. Legacy 与激活边界

旧 SourceVersion/1、Policy/1/2、D6 wire1、PreparedIntent、Frontier/1、InstallationNotice/1、ContentCompletionProof/1，以及已经存在的 ContentCompletionProof/2、ConflictRecord/1、旧 revision-token profile、D3 primary receipt/companion、D7 PreparedActionBinding/1,/2、D8 PreparedEditBinding/1 和历史 receipt/plan/unknown 继续按其原 decoder、bytes/fingerprint、授权、pins、期限与 continuity 履约。存在 decoder 或候选文字不证明所有历史 prototype 都曾部署或 active；但实际存在的 saved、planned、unknown 记录不能因新合同或缺部署证据而被取消恢复、保留和防重复执行责任。机器 registry 的 firstFreeze 仍是历史来源属性，不因 FA-r01 重开而改写。

当前固定 C 已经包含 D3 wire12 以及 D4/D5 对 A/B/C 的 SourceObservation/Frontier/WriteProtection 消费候选；这些候选并非不存在，但仍未独立接受、未激活、未实现本 P1 新生产者的全部规则。实际私人 Storage/Control 作者后像现已冻结生产域 revision/H、SourceRevisionPlan/RevisionTokenBinding、十五类 DependencyKey、ContentCompletionProof/4、ConflictRecord/2、U5 Frontier/恢复等生产者合同；本 Lexicon 只是对这些真实作者字节的术语配套，不构成语义接受。

依赖新生产者的 managed/strong success 仍须完成同一 P1 的机器 Registry、Impact/Test Outline、PROPOSAL/replacements routing，P2 D3/D4、P3 D5，以及 D7 Query/Value-CEL/View/Narrow Field/Definition Transfer/Preview-Effects/Execution-Action/Prepared/Scenarios/Lexicon-Registry/Impact、D8、D9、D10 的实际 consumer afterimages，并经过 fresh 独立全量联合审查与协调接受。缺少这些 strong consumer 不得半包激活；同时，不依赖缺失 strong proof 的既定 ordinary .adoc/Resource read、Draft、完整合格人工整源保存与局部离线操作也不得被永久禁用。

saved 决议只在当前原实际效果/结果披露范围的交付授权下重放原 bytes 或补原版本 publication/outbox，不重新执行旧业务、覆盖后来的 current source 或重复收费；planned 只恢复原冻结 plan、版本依据、pins、预算与安装状态，不新 prepare 第二 decision；unknown 与旧 pins/外部 effect/Approval/Money/执行连续责任保持原合同，不能从 current file、I、相同 hash 或重新授权猜掉。

旧十一项 OPEN、U6/U7、后续 A2 自包含重建以及 A2 后另一轮 fresh Pro 全局终审继续保留，作者不能在术语稿中自行核销或接受。当前作者阅读只覆盖本任务实际读取的两份 Lexicon 与四份私人 producer，不等于固定 S 49 原输入、双语全部 owner、D10 all18 或任何独立审查已经完成。新 public names 只有在所需 replacement owner 与 consumer 共同接受后才可进入产品/API；候选目录、fixture、文档检查或 CI 本身都不表示 capability available、schema released 或 semantics activated。

## 7. 仅供冲突解决的安装输入

ConflictInstallInput/1 继续是 D6 Control §3.1.1 仅供 D3 §10.1 使用的受保护 installation-before 类型，并与 D3ResolutionInputUse/1 不可分离；SourceRevisionPlan/2 继续只解码 D3 canonical-resolution managed after。独立的 D6 §9.4 source-conflict 路径只为 source_merge/choose_source_head 使用 SourceConflictBefore/1 + SourceConflictVersionBasis/1 与 SourceRevisionPlan/3。before 绑定真实已安装 managed production version/FileObjectBinding/source+metadata pins；basis 绑定完整 sealed base/all-head production versions 或 exact chosen-head production version。两条路径都不产生普通 SourceObservation/SourceVersionRef，冲突 subject 都不进入 sourceInputs，也不能用于 Query、D8、普通 save 或另一个 owner 的 resolver；其它实际 source 读取仍使用真实 current Observation。

/1 继续是 ordinary/fresh-source decoder。/2 与 /3 都保留当前 production-domain H、不可变 SourceStamp、单 planning CAS、唯一 seal ChangeId 与原 recovery。/3 中 choose_source_head 即使 bytes 相同但 selected production SourceVersion 不同，仍是真实 admission/H+1；真正 source-unchanged merge 或已经安装 exact selected version 不产生 /3/H/sourceChanges，尽管 conflict control 仍可 portable。CP3 before 永远是真实物理 before。这些只是既有 Source Version/Conflict/Dependency Proof owner 下的技术投影，不新增内容身份、公共 concept ID、alias 或第二业务 owner；现有记录保持原 decoder/pin/guard/recovery，D3 /2 授权没有扩张。

## 8. D10 控制、引导与连续性的联合名称

实际生产者位于 D6 Control §4.3–4.4、§10.1–10.2、§16 及 Storage §7.2.1。它们是协调作者候选，不是已接受的运行功能。既有 firstFreeze 历史来源及真实保存、计划、未知记录的解码器不变。

| 概念及英文名 | D6 所属技术名称 | 输入、输出与边界 |
| --- | --- | --- |
| D10 控制输入 / D10 Control Input | D10ControlInput/1; d10_control/1 | 消费准确 D10 稳定意图、完整分型控制图像、效果计划、依赖及固定确认要求，产生原 PreparedIntent/3 与 request/2。protocolOwner 仍为 D6，生成的 M 和后来的确认不进入自身不可变描述符。 |
| D10 自助控制 / D10 Self Control | d10_control_self | Policy/3 中显式且仅限工作区的能力，允许认证主体管理自己的有限 D10 记录，不授作者、资源或部署权。 |
| 调度连续性见证 / Schedule Continuity Witness | ScheduleContinuityWitness/1; ScheduleContinuityStep/1; ScheduleContinuityInvalidation/1; d6_schedule_continuity; d6_schedule_continuity_step | 消费实际完整的已注册源与控制转换及 D10 订阅语义，产生保护连续检查点，或该代永久的业务变化或缺口状态；不使用自由证明图或索引重建。 |
| 执行连续性证明 / Execution Continuity Proof | ExecutionContinuityProof/1 | 消费完整 D10ExecutionInventory/1、真实保护屏障与备份及旧持有者隔离，绑定唯一执行责任记录，不建立第二账户或作者账本。 |
| 工作区引导配置 / Workspace Bootstrap Profile | WorkspaceBootstrapProfile/4; d6_bootstrap_profile | 原六成员采用 wireVersion=3，固定显式 Policy/3 能力集合、完整目标 Registry 字段及不可变 family 绑定；IssuerControlPolicy 顶层仍为 /1。 |
| 工作区引导计划 / Workspace Bootstrap Plan | WorkspaceBootstrapPlan/3; d6_workspace_bootstrap_plan | 当前完整成员集合采用 wireVersion=3、profile/3、Policy/3 与 WorkspaceTrustGenesis/1，仍属于原 D3 全 fresh 唯一决议；D7 当前符号编码明确 plan3；真实 Plan/1 历史保持原解码器，Plan/2 仅为未激活候选前身。 |

| 工作区修订封存信任 / Workspace Revision-Seal Trust | WorkspaceAuthorizationBundle/1; WorkspaceTrustRootDeclaration/1; WorkspaceTrustRootFingerprint/1; WorkspaceTrustAnchor/1; WorkspaceTrustDeclaration/1; WorkspaceAuthorizationBundleAddress/1; TrustConflictCarry/1; TrustConflictOutcome/1; WorkspaceTrustRootKeyHandle/1; DomainSealKeyHandle/1; RevisionTokenSealVerificationKey/1 | D6 所属 producer chain。root fingerprint 继续与 rootKeyId 分离。TrustConflictCarry/1 现在精确标识原始 direct compromise declaration/cut：revoke 指 trustKeyId，rotate 指 replacesTrustKeyId，后续 resolve_conflict 只递归携同一 factId，不能移动原 cut。fresh outcome PoP 以父 resolve 字段加该 outcome 自身 key tuple 重构既有 D6-Domain-Seal-Key-PoP/1 body，交换签名必须失败。`validate_historical` 与 `authorize_new_sign` 继续分离。 |

新增概念的代码约定在既有 Weftext.Core.Storage 命名空间使用上表准确 PascalCase 类型名和 camelCase 成员名，不设替代别名、manifest 贡献或新 CLI 命令。候选区域键依次为 storage.d10_control_input、storage.d10_control_self、storage.schedule_continuity_witness 和 storage.execution_continuity_proof；界面使用上表准确中英文概念名，不缩写。新实施禁止未列别名，但不为删别名改写历史记录或用户正文。D10 继续拥有完整记录图像、控制效果、批准使用和计次、租约准入、费用、订阅及发送记录的技术定义；D6 只消费这些准确闭合类型，不复制第二份 schema。

## 9. 当前技术后继登记表

既有公开概念标识与受控名称集合继续由 D6-REGISTRY 持有；本表只登记技术版本分派，不新增公开概念。

| 名称 | 所有者 | 状态 | 替代 | 当前含义 |
| --- | --- | --- | --- | --- |
| `DocumentFormatCurrentQualification/1` | D2/D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DependencyKey/3` | D6 | current | `DependencyKey/2` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DependencyProof/3` | D6 | current | `DependencyProof/2` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `InputDescriptor/3` | D6 | current | `InputDescriptor/2` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `PreparedIntent/3` | D6 | current | `PreparedIntent/2` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `PortableComponentKey/2` | D6 | current | `PortableComponentKey/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `InstallationNotice/3` | D6 | current | `InstallationNotice/2` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ContentCompletionProof/4` | D6 | current | `ContentCompletionProof/3` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ChangeRecord/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `PortableTransformCompilation/1` | D6/Core | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `SourceTransformPortableEvent/3` | D6/Core | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `CoreSourceEditPlan/2` | D6/Core | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `TransformEmissionPlan/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `SourceTransformEvidence/2` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `SourceTransformSealArtifact/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `SourceTransformSealKey/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `SourceTransformSealOutboxItem/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `SourceTransformSealVerificationKey/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceTrustDeclaration/2` | D6 | current | `WorkspaceTrustDeclaration/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceAuthorizationBundle/2` | D6 | current | `WorkspaceAuthorizationBundle/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DomainSealKeyHandle/2` | D6 | current | `DomainSealKeyHandle/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceTrustGenesis/2` | D6 | current | `WorkspaceTrustGenesis/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceBootstrapPlan/4` | D6 | current | `WorkspaceBootstrapPlan/3` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `D10ControlInput/2` | D10/D6 | current | `D10ControlInput/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ExecutionResponsibilityRecord/3` | D6/D10 | current | `ExecutionResponsibilityRecord/2` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ExecutionContinuityProof/2` | D6/D10 | current | `ExecutionContinuityProof/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `PinRef/2` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `ComponentImage/1` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `OwnerInputBinding/2` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `ObservationScope/2` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `WorkspaceTrustRootDeclaration/1` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `WorkspaceTrustAnchor/1` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `RevisionTokenSealVerificationKey/1` | D6 | retained-current | — | 保持当前版本，不机械升版；精确字段、权限与恢复语义继续由原所有者合同约束。 |
| `SourceTransformSealSignedBody/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceTrustPredecessor/2` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DomainSealKeyPoPBody/2` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DomainSealKeyRotateContinuityBody/2` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceTrustDeclarationSignedBody/2` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DomainSealKeyAddPrepare/3` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `DomainSealKeyRotatePrepare/3` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |loss_recovery\|compromise. |
| `DomainSealKeyRevokePrepare/3` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |loss\|compromise. |
| `WorkspaceBootstrapProfile/4` | D6 | current | `WorkspaceBootstrapProfile/3` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceBootstrapCreatorBinding/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceBootstrapTargetRegistry/1` | D6/D4 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceBootstrapSeriesConfiguration/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `WorkspaceBootstrapPeriodScopeBinding/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `TrustConflictCarry/2` | D6 | current | `TrustConflictCarry/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `TrustConflictOutcome/2` | D6 | current | `TrustConflictOutcome/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `FreshDomainAuthorizationSpec/2` | D6 | current | `FreshDomainAuthorizationSpec/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `PolicyBundleHeadEvidence/2` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ConflictResolutionPolicyDerivedPlan/2` | D6 | current | `ConflictResolutionDerivedPlan/1 policy_bundle arm` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ConflictResolutionInput/3` | D6 | current | `ConflictResolutionInput/2 policy_bundle_choice owner descriptor` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ScheduleContinuityWitness/2` | D6/D10 | current | `ScheduleContinuityWitness/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ScheduleContinuityStep/2` | D6/D10 | current | `ScheduleContinuityStep/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ScheduleContinuityInvalidation/2` | D6/D10 | current | `ScheduleContinuityInvalidation/1` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `TrustConflictCarryValidationHop/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `TrustConflictCarryValidationEvidence/1` | D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `ConflictResolutionPreview/2` | D6 | current | `ConflictResolutionPreview/1 policy_bundle arm` | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `PortableAnnotationRecord/4` | D3/D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `D2ProductEvaluationBinding/1` | D2/D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `D8PresentationPolicyHeadSet/1` | D8/D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `AnnotationAggregate/1` | D6 Storage | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `AnnotationAggregateObservation/1` | D6 Storage | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `AnnotationAggregateInstall/1` | D6 Storage | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |
| `D8PresentationPolicyBootstrapInit/1` | D8/D6 | current | — | 当前具名技术类型；精确字段、交叉校验、权限与恢复语义见配套闭合规范与控制正文，前身版本只按真实历史记录分派。 |

## 10. 接受边界

本 Lexicon 不是独立接受。historical evidence 与普通作者 prose 绝不能被当成 denylist 扫描。
