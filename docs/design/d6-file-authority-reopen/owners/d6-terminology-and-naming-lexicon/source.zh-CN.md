---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：`680b2061-ccec-4bef-b9a8-9d7080f837fb`。

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 中旧 revision05/D7 联合术语状态仅作历史来源。稳定文档 ID、既有 conceptId、ownedNames、firstFreeze 全部保持；新概念 firstFreeze 单独标为 D6-FA-r01 candidate。

# D6 Terminology and Naming Lexicon

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
| weftext.term.source-version | 源版本 | Source Version | \`SourceVersion\`<br>\`sourceVersions\`<br>\`sourceRevision\`<br>\`readSource\`<br>\`DocumentRevision\`<br>\`ResourceRevision\`<br>\`AnnotationRevision\` | SourceVersion/2将完整EntityRef与CommitDomain、observationEpoch及managed revision+ChangeId或external observation sequence绑定；跨domain裸revision不可比较 | 不是只靠摘要防ABA；不是DocumentRef；旧SourceVersion/1只服务历史decoder/replay | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.principal-context | 认证主体上下文 | Principal Context | \`PrincipalContext\`<br>\`principalContext\` | host/D10认证的主体、session、delegation与policy generation | 不是客户端JSON自行声明的principal | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.commit-protocol | 控制提交协议 | Control Commit Protocol | \`D6ControlCommit\`<br>\`d6_commit_request\`<br>\`d6_commit_receipt\`<br>\`d6_error\`<br>\`commitBoundPlan\`<br>\`replayOperation\` | D6 own v2 ledger的闭合prepare/plan/file-install/durable-seal/receipt/replay协议；operation key含CommitDomain，D3 identity/lifecycle仍走其owner wire | 不包装D3绕过其阶段；不是文件hash CAS；不是第二Action领域 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.effects-token | 效果清单令牌 | Effects Token | \`EffectsToken\`<br>\`effectsToken\` | 引用同一decision保存的完整效果清单的运输token | 不是第二作者源、独立成功receipt或写能力 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-page-protocol | 结果分页协议 | Result Page Protocol | \`D6ResultPage\`<br>\`d6_result_page_request\`<br>\`d6_result_page\`<br>\`d6_result_error\`<br>\`readResultPage\` | 完整结果的当前受权分页与闭合错误 | 不是D7 row schema或streaming部分成功 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.attempt-allowance | 执行次数额度 | Attempt Allowance | \`AttemptAllowance\`<br>\`attemptAllowance\`<br>\`d6_attempt_allowance_intent\` | 绑定planned decision的有限执行attempt资格及管理意图 | 不扩大原语义预算或退还已耗work | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.execution-resource-summary | 执行资源摘要 | Execution Resource Summary | \`ExecutionResourceSummary\`<br>\`executionResourceSummary\` | 向当前Workspace policy_admin交付的受管计划占用与暂停类别 | 不是普通用户可枚举的作者内容或自动放弃权限 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.dependency-proof | 依赖完整性证明 | Dependency Completeness Proof | \`DependencyProof\`<br>\`dependencyProof\`<br>\`readCompleteScope\`<br>\`validateDependencies\` | 受信完整读取及其正负范围版本证明，绑定CommitDomain/Frontier/observation epoch；部分index只能在证明覆盖时作为扫描优化 | 不是客户端readSet声明；不是building index；不自动证明结果可观察 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.foreign-version-comparison | 外部版本比较 | Foreign Version Comparison | \`ForeignVersionComparison\`<br>\`compareForeignVersion\` | 由D9/D10固定比较器证明外部版本关系的纯读取 | 不重定义D3 SourceBinding或OriginBinding；比较不执行绑定退役或任何持久修改 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.binding-retirement-intent | 绑定退役意图 | Binding Retirement Intent | \`BindingRetirementIntent\`<br>\`retireBinding\` | 显式受权的既有SourceBinding/OriginBinding控制退役意图，由所属D9/D10输入适配并同事务提交 | 不重新定义D3绑定、不由只读版本比较自动触发 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.control-inspection-protocol | 控制状态读取协议 | Control Inspection Protocol | \`ControlInspection\`<br>\`d6_control_read_request\`<br>\`d6_control_state\`<br>\`d6_control_error\` | 向当前Workspace policy_admin读取同cut配置/CAS版本与执行资源摘要的closed协议 | 不是通用作者源API或OperationId枚举 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.byte-handle | 资源字节句柄 | Resource Byte Handle | \`ByteHandle\`<br>\`ByteHandleRecord\`<br>\`d6_byte_handle\`<br>\`handleToken\` | D2 Resource snapshot中绑定已提交完整ResourceRef/版本/cut/descriptor/audience/pin/期限/预算的只读字节句柄 | 不是上传staging、identity、Query result/cursor、plan、effects、权限凭证或latest指针 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.byte-read-protocol | 资源字节读取协议 | Resource Byte Read Protocol | \`ByteReadProtocol\`<br>\`readResourceBytes\`<br>\`d6_byte_read_request\`<br>\`d6_byte_chunk\`<br>\`d6_byte_error\` | 固定ByteHandle上的closed offset/maxBytes读取、完整chunk与准确EOF、当前授权和共享有界计费 | 不是D7 row协议、repair Document envelope、自由URL/路径读取或新增提交入口 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.observation-scope | 潜在约束观察范围 | Constraint Observation Scope | \`ObservationScope\`<br>\`observationScope\`<br>\`d6_observation_scope\` | 在作者/范围读取前固定的结果观察上界，覆盖空范围并按当前权限证明 | 不是实际写集或内部完整读取证明 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.issuer-control-policy | 签发域控制权限 | Issuer Control Policy | \`IssuerControlPolicy\`<br>\`issuerControlPolicy\`<br>\`d6_issuer_control_policy\` | issuer自身allocate/administer授权及当前revision | 不是身份认证或Workspace内容授权 | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.workspace-bootstrap-plan | 工作区初始控制计划 | Workspace Bootstrap Plan | \`WorkspaceBootstrapPlan\`<br>\`workspaceBootstrapPlan\`<br>\`d6_workspace_bootstrap_plan\` | 原D3计划内固定首份policy/Registry/Calendar控制和主体映射并同activation发布 | 不是第二decision、可编辑请求或永久管理员 | D6 revision05-observation-bootstrap candidate; not activated |
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
| weftext.term.content-guarantee | 内容提交保证 | Content Guarantee | \`ContentGuarantee,contentGuarantee\` | 提交结果的闭集replica_local或managed_atomic；前者仅证明本CommitDomain可靠安装，后者还证明受管范围发布屏障 | 不声称任意外部工具跨文件瞬时原子或离线副本无并发 | D6-FA-r01 candidate; not activated |
| weftext.term.semantic-state | 语义状态 | Semantic State | \`SemanticState,semanticState\` | D2-valid受管source的complete_semantics或semantic_pending及其明确未证明obligations | external_invalid不属于成功SemanticState；pending不等于D4/D5全集通过 | D6-FA-r01 candidate; not activated |
| weftext.term.installation-notice | 安装告知记录 | Installation Notice | \`InstallationNotice,installationNotice\` | 触碰portable current前耐久写入的不可变ChangeId/write-set/before-after摘要记录，用于部分同步和P丢失时圈定风险 | 不是commit proof、receipt、approval或execution authority | D6-FA-r01 candidate; not activated |
| weftext.term.content-completion-proof | 内容完成证明 | Content Completion Proof | \`ContentCompletionProof,contentCompletionProof\` | P seal后发布到portable metadata的不可变证明，绑定ChangeId、semantic state、frontier推进及actual after components | 不授receipt读取、Money/approval消费或execution takeover | D6-FA-r01 candidate; not activated |
| weftext.term.conflict-record | 冲突记录 | Conflict Record | \`ConflictRecord,conflictRecord,ConflictKey,conflictId\` | 由规范ConflictKey/ConflictId定位的portable并发版本记录，状态open→resolution_prepared→resolved或superseded | 不是自动merge、LWW或D3 bypass transaction | D6-FA-r01 candidate; not activated |
| weftext.term.reliable-save-state | 可靠保存状态 | Reliable Save State | \`ReliableSaveState,reliableSaveState\` | 区分not_saved与P seal后的reliable；portable publication由独立状态读取 | 不等于Draft已保存、sync上传或ContentCompletionProof已发布 | D6-FA-r01 candidate; not activated |
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

SourceVersion/1 的 ownedNames 保留给历史 decoder。新 managed producer使用 SourceVersion/2，必须绑定完整 Ref、CommitDomain、observationEpoch、revision、ChangeId；external observation使用其 closed external variant。任何 consumer 若只比较 revision 数字，都不是 D6-FA-r01 合格实现。

source A→B→A、watcher gap、placeholder materialization 都可使 observation epoch/variant变化；
  digest相同不能恢复旧 locator、prepared、ActionEvidence 或 sourceOccurrenceKey continuity。

### 2.4 Reliable Save 与 Portable Publication

ReliableSaveState 只回答本 domain 的 planned write set是否经过合格 file install并完成 P durable seal。ContentCompletionProof/portable publication 另回答该 sealed change是否已经形成可由其它 replica接纳的完整 portable version。

Draft persistence、HTTP success、worker success、sync upload、
  InstallationNotice 都不能称 reliable save。reliable 也不能自动称 portable published。

### 2.5 Semantic State

complete_semantics 与 semantic_pending 都要求 D2 valid。pending 必须显式列出尚未证明的 closed obligation；不能用“稍后检查”自由字符串。

external_invalid 不是 SemanticState，因为它没有成功 Core author decision。它属于 CurrentSourceState 的外部 raw branch。

semantic_pending 不得被 D7 complete Query/Action、purge、relation/unique/Calendar mutation、自动化写入当 complete。允许的局部消费必须由后续 D4/D5/D7/D8 owner 明确冻结。

## 3. 技术成员归属

- CommitDomain、ReplicaEpoch、ChangeId、Frontier、SourceVersion/2、SemanticState、ContentGuarantee、
  InstallationNotice、ContentCompletionProof、ConflictRecord、ReliableSaveState、ExecutionResponsibilityRecord：D6。
- NodeRef/ResourceRef/AnnotationRef、OperationId、AuthorityInstanceId、D3 lifecycle receipt：D3；D6不得通过 generic commit重新定义。
- FieldId、RegistryBinding、relation/Calendar typed semantics：D4。
- QuerySpec、ActionSpec、PreparedActionBinding、EffectManifest/EffectBytes：D7；FA-r01新版本必须后续协调。
- Draft/Edit Map/IME/Undo/Source-Live-Read：D8。
- ImportJob 的转换输入、ExportPlan/Publication：D9；D6只拥有耐久控制与预算边界。
- ApprovalUse、sourceOccurrenceKey、ToolValue、Money各级谱系、Agent/Automation/connector external effect：D10/相应Money owner；D6只拥有连续耐久 container 与无双花/不重放边界。

## 4. Policy/3 能力受控映射

Policy/3 沿用 Policy/2全部能力，并新增：

| capability | 语义 | 不蕴含 |
|---|---|---|
| replica_register | 显式登记新ReplicaEpoch | source read/write、execution takeover |
| replica_retire | 退役ordinary replica writer epoch | 删除其历史、Money退款 |
| conflict_read | 读取已获state scope的ConflictRecord | conflict source bytes |
| conflict_resolve | 进入对应owner resolution prepare | source/policy/D3 write |
| execution_custody_admin | 管理执行责任连续性/接管 | 新approval、扩Money、作者写 |

commit_sequence_state 在 Policy/3 中只观察指定 CommitDomain 的 domainCommitSequence。旧 Policy/2 consumer 的 Workspace-wide定义只保留历史路径，禁止把它补到新离线replica模型。

## 5. 碰撞、别名与迁移

1. 不新增 local workspace id、device workspace id、sync id 作为内容 identity。
2. ReplicaEpoch、AuthorityInstanceId、executionDomainId 都是不同 UUID domain，不可互换。
3. ChangeId 不得简称 revision、commit id、operation id 或 sync token。
4. Frontier 不得称 global latest/version；它是多domain因果前沿。
5. ConflictId 是 ConflictKey 的稳定地址，不是独立 durable content identity。
6. DurableControlStore 不得简称 index；DerivedIndexStore 不得简称 database authority。
7. save 文案必须区分 Draft saved / reliable author save / portable published；UI/CLI 不能把三者折叠成一个成功。
8. 新协议没有旧别名兼容。历史 saved evidence中的旧controlled name保留，不做文本迁移或删除。

## 6. Legacy 与激活边界

旧 sourceVersion、Policy/1/2、D6 wire1、PreparedIntent、D7 binding/effects 和历史 receipt继续其原 decoder/firstFreeze。机器 registry 的 firstFreeze 是历史来源属性，不因 FA-r01重新冻结而改写。

新 public names只有在全部 replacement owner与consumer共同接受后可进入产品/API。当前目录存在不代表 capability available，也不能把 candidate term 注入 D10 manifest/contribution或产品日志作为已发布 schema。
