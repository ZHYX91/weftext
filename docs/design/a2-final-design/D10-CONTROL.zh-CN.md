---
source_language: zh-CN
translation_status: source
---

[English](D10-CONTROL.md)

# D10 控制与管理合同

## 0. A2 当前唯一 owner 及前身解码边界

**作者候选，尚未独立接受或实施。**下面逐字纳入 A2 D6-SCHEMAS §10/§10.1 的现任混合 version 与直接类型。原 CONTROL §§1–16 的业务不变量继续继承，但 Image1/Dependencies2/Binding2/Schedule1/Link1/ApprovalUse1/Record2/Proof1 的旧「current」称谓是历史来源，首次产生必须按本节的真实现任类型，历史真实记录原样恢复且不重编码、不借 LWW、没有第二 store/CAS。

### 0.1 现任 D6 mixed-version 完整 schema

以下均为 current outer-holder schema。每个 versioned carrier 都必须按 tag 严格解码对应的精确 inner type，并保留其原 canonical bytes 与 pins；未知 tag 必须 fail closed。carrier 只负责选择 decoder，不增加 authority、不建立 decision/CAS，也不迁移 inner record。

```text
D10WorkspaceReadDependencies/2 = {
  kind:"d10_workspace_read_dependencies",version:2,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  dependencyProof:DependencyProof/3,
  registryReads:[{binding:RegistryBinding/1,snapshot:RegistrySnapshot/1,
                  snapshotPin:PinRef/2}...],
  evidencePins:[PinRef/2...]
}

D10VersionedControlRecordImage/1 =
    {schema:"d10_control_record_image/1",value:D10ControlRecordImage/1}
  | {schema:"d10_control_record_image/2",value:D10ControlRecordImage/2}

D10ControlRecordImage/2 = {
  kind:"d10_control_record_image",version:2,
  binding:Binding<K>/1,scope:Scope/1,
  usageRevision:Option<Counter>,view:ControlCurrentView<K>/1,
  supplement:
      {kind:"automation",subscription:ScheduleSubscription/2,
       stop:Binding<stop>/1,creator:Token}
    | {kind:"run",createdBinding:Binding<run>/1,
       principal:Token,fixedBudget:BudgetCaps/1,
       invocation:Option<AutomationInvocation/1>,
       admission:Option<LeaseRunUse/1>,
       authorSteps:[D10VersionedAuthorStepResponsibility/1...]}
}

D10ControlRecordPin/2 = {
  image:D10ControlRecordImage/2,pin:PinRef/2
}

D10VersionedControlRecordPin/1 =
    {schema:"d10_control_record_pin/1",value:D10ControlRecordPin/1}
  | {schema:"d10_control_record_pin/2",value:D10ControlRecordPin/2}

D10ControlRange/2 =
    {kind:"records",version:2,scope:Scope/1,
     kinds:[ControlRecordKind/1],epoch:Token,revision:Counter,
     members:[ControlRef<K>/1]}
  | {kind:"cost_lineage",version:2,key:CostLayerKey/1,
     epoch:Token,revision:Counter,
     reservations:[ControlRef<reservation>/1]}
  | {kind:"occurrences",version:2,
     automation:ControlRef<automation>/1,epoch:Token,revision:Counter,
     records:[D10VersionedAutomationOccurrenceRecord/1...]}

D10VersionedControlRange/1 =
    {schema:"d10_control_range/1",value:D10ControlRange/1}
  | {schema:"d10_control_range/2",value:D10ControlRange/2}

D10VersionedControlPrepareBinding/1 =
    {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
  | {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
  | {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}

ControlPrepareBinding/1 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  dependencyPins:ControlDependencies/1
}

ControlPrepareBinding/2 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/2
}

ControlPrepareBinding/3 = {
  kind:"d10_control_prepare_binding",version:3,
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/3
}

对 current unseen external-consent preparation，confirmationRequirement 仍是 fixed-parent attended-confirmation protocol 使用的同一个闭合 ExternalConfirmationRequirement/1，并且必须在原 commit request 交付前冻结于 ControlPrepareBinding/3。可变 confirmation fact 仍只属于 ExternalConfirmationRecord/1，不能塞进 Binding3 或 ControlDependencies/3。current confirmation eligibility 因而消费 Binding3 + Dependencies3，同时保留原 trusted event、principal、time window、immutable intent/preview、disclosure、approval_unavailable/preflight、saved-result replay 与 result-redaction 规则。已经证明的历史 ControlPrepareBinding/2 保留其精确 requirement、Dependencies2、canonical intent bytes、confirmation association 与 recovery decoder。

ControlDependencies/3 = {
  kind:"d10_control_dependencies",version:3,
  configBindings:[Binding<K>/1],
  usageBindings:[{ref:ControlRef<lease|approval|grant|cost_account>/1,
                  usageRevision:Counter}...],
  authorityProof:Token,authorizationGenerations:[Token...],
  stopRefs:[ControlRef<stop>/1...],
  workspaceReads:Option<D10WorkspaceReadDependencies/2>,
  recordPins:[D10VersionedControlRecordPin/1...],
  controlRanges:[D10VersionedControlRange/1...]
}

D10ControlEffectPlan/2 = {
  kind:"d10_control_effect_plan",version:2,
  changes:[{before:Option<D10VersionedControlRecordImage/1>,
            after:D10VersionedControlRecordImage/1}...],
  activationSelector:Option<{before:D10ActivationSelector/1,
                             after:D10ActivationSelector/1}>,
  recordPins:[D10VersionedControlRecordPin/1...],
  registryChange:Option<{
    before:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    after:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    evolution:RegistryEvolutionProof/1,
    beforePin:PinRef/2,afterPin:PinRef/2}>
}

D10VersionedAuthorStepResponsibility/1 =
    {schema:"d10_author_step_responsibility/1",
     value:D10AuthorStepResponsibility/1}
  | {schema:"d10_author_step_responsibility/2",
     value:D10AuthorStepResponsibility/2}

D10VersionedScheduleSubscription/1 =
    {schema:"d10_schedule_subscription/1",value:ScheduleSubscription/1}
  | {schema:"d10_schedule_subscription/2",value:ScheduleSubscription/2}

D10VersionedAutomationOccurrenceRecord/1 =
    {schema:"d10_automation_occurrence_record/1",
     value:AutomationOccurrenceRecord/1}
  | {schema:"d10_automation_occurrence_record/2",
     value:AutomationOccurrenceRecord/2}

D10VersionedApprovalUse/1 =
    {schema:"d10_approval_use/1",value:ApprovalUse/1}
  | {schema:"d10_approval_use/2",value:ApprovalUse/2}

D10ExecutionClaims/2 = {
  kind:"d10_execution_claims",version:2,
  recordPins:[D10VersionedControlRecordPin/1...],
  prepareBindings:[D10VersionedControlPrepareBinding/1...],
  leaseRuns:[LeaseRunUse/1...],
  authorSteps:[D10VersionedAuthorStepResponsibility/1...],
  subscriptions:[D10VersionedScheduleSubscription/1...],
  occurrenceRecords:[D10VersionedAutomationOccurrenceRecord/1...],
  ranges:[D10VersionedControlRange/1...],
  continuityPins:[PinRef/2...]
}

D10MoneyResponsibility/2 = {
  kind:"d10_money_responsibility",version:2,
  reservations:[{binding:Binding<reservation>/1,
                 value:CostReservation/1,
                 attribution:CostBudgetAttribution/1,
                 settlements:[CostSettlementDecision/1...]}...],
  layers:[CostLayerTotal/1...],
  recordPins:[D10VersionedControlRecordPin/1...],
  ranges:[D10VersionedControlRange/1...],
  evidencePins:[PinRef/2...]
}

StopCapacity/1 = {
  issued:Counter,reserved:Counter
}

D10ExecutionInventory/2 = {
  kind:"d10_execution_inventory",version:2,
  workspaceRef:WorkspaceRef,storeIncarnation:Uuid,
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  stopCapacity:StopCapacity/1
}

ExecutionResponsibilityRecord/3 = {
  kind:"d6_execution_responsibility",version:3,
  workspaceRef:WorkspaceRef,executionDomainId:Uuid,
  holder:
      {kind:"local_replica",replicaEpoch:Uuid}
    | {kind:"server",authorityInstanceId:Uuid,deploymentId:Uuid},
  revision:Counter,status:"active"|"paused"|"transferring",
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  lastContinuityProof:ExecutionContinuityProof/2
}

ExecutionContinuityProof/2 =
    {kind:"initial",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,birthProofToken:Token}
  | {kind:"checkpoint",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,barrierToken:Token}
  | {kind:"handoff",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,fromHolder:ExecutionHolder,
     toHolder:ExecutionHolder,oldRevision:Counter,
     barrierToken:Token,oldHolderFenceToken:Token}
```

Image2 只允许 automation/run；其它 record kind 继续使用精确 Image1。Pin1 保留历史 D10-Control-Record/1 前缀与 decoder。Pin2 的完整内容是 UTF8 D10-Control-Record/2、NUL 与 D3-CJ/3(Image2)；PinRef/2 的 payloadKind 是 artifact，retention 沿用 recovery 或 approval_money，byteLength 与 SHA-256 必须覆盖完整前缀内容。schema tag 与 payload domain 不一致必须失败，Pin2 不得重新 pin Image1。

对 D10VersionedControlRecordPin/1 数组，先令 I=carrier.value.image。排序依次使用 D3-CJ/3(I.binding.ref)、binding.revision 数值、usageRevision 的 none 先于 some、some 时的 usageRevision 数值、最后 D3-CJ/3(I)。同 cut identity 是 I.binding.ref、I.binding.revision、I.usageRevision。相同 identity 的第二个 byte-equal image 属于重复并拒绝；bytes 不同则是 integrity_conflict。version 不能拆分该 identity。

Range kind rank 固定为 records=0、cost_lineage=1、occurrences=2。跨版本 logical identity 分别是 scope 加完整排序后的 kinds、CostLayerKey、Automation Ref。mixed range 先按 rank 再按 identity canonical bytes 排序；每个 identity 只能出现一次。occurrences range 的内部记录按 AutomationOccurrenceKey 排序；一个 key 不能同时出现 V1 与 V2。

D10ControlEffectPlan/2.changes 按完整 after.value.binding.ref 的 canonical bytes 排序唯一。before 存在时，before.binding.ref 与 after.binding.ref 必须相等，recordPins 还必须含有与真实存储 before schema 完全匹配的 versioned pin。Image1 before 配 Pin2 非法。Image1 Automation+Subscription1 → Image2+Subscription2 是合法 current configure。若 scheduling owner 的 continue 规则成立，普通 configure 可以让旧 subscription 保持同一 generation；mixed 支持不能强迫 replace 或后台 migration。

ControlDependencies/3 的数组保持 owner 排序：configBindings 与 usageBindings 按完整 Ref；authorizationGenerations 按 token；stopRefs 按完整 Ref；recordPins/ranges 按上述 mixed 顺序。每个 config binding 都有精确匹配 image/pin；每个 usage binding 都有相同 usageRevision 的 image；每个 stopRef 都有精确 stop Image1/Pin1。current binding、usage revision、range fence、pin 与 Workspace evidence 必须来自同一个真实 Authority Store barrier。barrier A 的 V1 evidence 与 barrier B 的 V2 evidence 不能拼成 complete dependency snapshot。缺失必要历史 bytes、decoder 或 pin 时，在 disclosure 之后返回 state_unavailable；同 cut 矛盾是 integrity_conflict。

D10ExecutionClaims/2 的 canonical identity 固定如下：recordPins 使用上面的规则；prepareBindings 在 Binding1/2/3 之间统一按 inner StableControlKey；leaseRuns 按完整 Run ControlRef；authorSteps 在版本间统一按 run Ref 加 stepId；subscriptions 在版本间统一按 Automation Ref 加 generation；occurrenceRecords 按 AutomationOccurrenceKey；ranges 使用上面的规则；continuityPins 按 pinToken。每个 identity 跨版本唯一。Binding1(K) 与 Binding3(K) 即使 canonicalIntentBytes 和 originalCommitRequest byte-equal 也不能共存。Binding1 保留历史六成员精确 shape；不能增加 kind、version、confirmationRequirement 或 /2-/3 dependency field，也不能 LWW、repin。

D10MoneyResponsibility/2 的 reservation 按完整 Binding<reservation>/1 canonical bytes 排序唯一；layer 按 CostLayerKey canonical bytes 排序唯一；recordPins/ranges 使用 mixed 规则；evidencePins 按 pinToken 排序唯一。D10ExecutionInventory/2 的 approvalUses 按 DecisionKey canonical bytes 跨版本唯一，externalUnknowns 与 stopState 均按 binding.ref canonical bytes 排序唯一。所有 pending、unknown、dedup、stop responsibility，以及仍被引用的已完成 external attempt 都必须保留。只有在 admission、planning、send、schedule writer 都停在同一个真实 store barrier 后才能捕获 inventory。

Inventory2 artifact 的精确 payload 是 UTF8 D6-Execution-Inventory/2、NUL、D3-CJ/3(D10ExecutionInventory/2)。PinRef/2 为 artifact/recovery，byteLength 与 SHA-256 覆盖完整前缀 bytes。Inventory1 保留 D6-Execution-Inventory/1，绝不重新 pin 成 /2。

Record3.workspaceRef 必须等于 Inventory2.workspaceRef。Record3 的 approvalUses、claims、moneyLineage、externalUnknowns、stopState 五类 payload 分别与 Inventory2 对应成员 byte-equal。Proof2.inventoryPin 必须选择这一份精确 Inventory2，且 Inventory2.storeIncarnation 等于 Proof2.storeIncarnation。受保护的 birth/barrier/fence token mapping 必须证明同一实际 store 与 capture barrier；不新增独立 caller-supplied store-incarnation proof object。

Inventory2.stopCapacity 必须是同一 authoritative safety-store barrier 下的精确 fixed-parent StopCapacity/1。StopCapacity/1 仍只有 issued、reserved，不在 Record3 中重复。Workspace-only handoff 不能把 shared safety counter 复制到第二个 active store。完整 store handoff 必须 fence 全部受影响 writer，并保留所有 target/latch reservation 与 safety capacity；边界不可证明时 takeover unavailable。

Record2、Proof1、Inventory1 都保留历史 decoder/domain。Record3 只在真实 responsibility mutation、checkpoint 或 custody handoff 时产生，必须保持 executionDomainId；unchanged holder 不做后台 migration。

### 0.2 现任 schedule/author-step 完整 schema

以下类型是 §10 mixed wrappers 所引用的 current exact inner values；不是 wrapper 自行发明的自由对象。

```text
D10ControlInput/2 = {
  kind:"d10_control",
  version:2,
  key:StableControlKey/1,
  operationId:Uuid,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  preview:ControlPreview/1,
  dependencies:ControlDependencies/3,
  effectPlan:D10ControlEffectPlan/2,
  confirmationRequirement:ExternalConfirmationRequirement/1
}

ScheduleRecurrenceEvidence/2 = {
  kind:"d10_schedule_recurrence_evidence",
  version:2,
  observation:SourceObservation/1,
  sourcePin:PinRef/2,
  metadataPin:PinRef/2,
  dependencyProof:DependencyProof/3,
  registrySnapshot:RegistrySnapshot/1,
  registryEvolution:Option<RegistryEvolutionProof/1>,
  recurrenceContext:RecurrenceReadContext/1
}

ScheduleSourceBinding/2 =
    {kind:"once",atUtcSeconds:CanonicalDecimal}
  | {kind:"recurrence",
     ownerNodeRef:NodeRef,
     recurrenceOccurrenceKey:occurrenceKey,
     rangeOccurrenceKey:occurrenceKey,
     initial:ScheduleRecurrenceEvidence/2,
     checkpoint:ScheduleRecurrenceEvidence/2,
     continuityPins:[PinRef/2...]}

ScheduleSubscription/2 = {
  kind:"d10_schedule_subscription",
  version:2,
  automation:ControlRef<automation>/1,
  generation:Counter,
  lowerOriginalStartUtcSeconds:CanonicalDecimal,
  activeDefinition:Binding<automation>/1,
  definitionRevision:Counter,
  source:ScheduleSourceBinding/2
}

ScheduleOccurrenceProof/2 =
    {version:2,kind:"once",at:zoned_instant}
  | {version:2,kind:"recurrence",
     evidence:ScheduleRecurrenceEvidence/2,
     projection:<complete accepted D4 recurrence projection outcome>,
     originalStart:<the selected outcome row's D4 originalStart>}

AutomationOccurrenceRecord/2 = {
  kind:"d10_automation_occurrence_record",
  version:2,
  key:AutomationOccurrenceKey/1,
  definition:Binding<automation>/1,
  definitionRevision:Counter,
  dueUtcSeconds:CanonicalDecimal,
  proof:ScheduleOccurrenceProof/2,
  disposition:AutomationOccurrenceDisposition/1
}

ScheduleContinuityWitness/2 = {
  kind:"d6_schedule_continuity",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  revision:Counter,
  initial:ScheduleRecurrenceEvidence/2,
  checkpoint:ScheduleRecurrenceEvidence/2,
  status:"continuous"|"binding_changed"|"gap",
  producerEpoch:Token,
  consumedTransition:Counter
}

ScheduleContinuityStep/2 = {
  kind:"d6_schedule_continuity_step",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  before:ScheduleRecurrenceEvidence/2,
  after:ScheduleRecurrenceEvidence/2,
  portableChanges:[{
    changeRecordPin:PinRef/2,
    installationNoticePin:PinRef/2,
    completionProofPin:PinRef/2
  }...],
  dependencyBefore:DependencyProof/3,
  dependencyAfter:DependencyProof/3,
  retainedInputs:[PinRef/2...]
}

ScheduleContinuityInvalidation/2 = {
  kind:"d6_schedule_continuity_invalidation",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  status:"binding_changed"|"gap",
  evidencePins:[PinRef/2...]
}
```

当前 continuity artifact 的编码按版本精确分域：

```text
Witness2ArtifactBytes =
  UTF8("D6-Schedule-Continuity/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityWitness/2)

Step2ArtifactBytes =
  UTF8("D6-Schedule-Step/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityStep/2)

Invalidation2ArtifactBytes =
  UTF8("D6-Schedule-Invalidation/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityInvalidation/2)
```

三者对应的 PinRef/2 都使用 payloadKind=artifact、retentionClass=recovery。byteLength 与 SHA-256 必须覆盖完整 domain prefix、唯一的 NUL byte 和完整 canonical object bytes。digest 本身不认证 continuity；仍必须验证受保护 producer/subscription provenance、producerEpoch、精确 generation、retained transition chain，以及原 authorization/disclosure gate。

decoder 必须先按 authenticated artifact domain 闭合分派，再解 inner object。D6-Schedule-Continuity/1 只能解 historical ScheduleContinuityWitness/1；D6-Schedule-Step/1 只能解 historical ScheduleContinuityStep/1；D6-Schedule-Invalidation/1 只能解 historical ScheduleContinuityInvalidation/1。D6-Schedule-Continuity/2 只能解 ScheduleContinuityWitness/2；D6-Schedule-Step/2 只能解 ScheduleContinuityStep/2；D6-Schedule-Invalidation/2 只能解 ScheduleContinuityInvalidation/2。unknown domain、已知 domain 搭配错误 kind/version、其它 prefix、缺失 NUL 或非 canonical object bytes 都必须按原 disclosure/error boundary fail closed。禁止 fallback decoder、扩展 /1 decoder、repin 或重新编码历史 bytes。

```text
D10AuthorPreparationLink/2 = {
  kind:"d10_author_preparation_link",
  version:2,
  run:ControlRef<run>/1,
  stepId:Counter,
  automation:Binding<automation>/1,
  definitionRevision:Counter,
  taskDigest:Sha256,
  preparedBindingToken:Token,
  request:d6_commit_request/2
}

ApprovalUse/2 = {
  version:2,
  approval:Binding<approval>/1,
  run:ControlRef<run>/1,
  stepId:Counter,
  request:d6_commit_request/2,
  decisionKey:DecisionKey/2,
  preparedBindingToken:Token,
  previewSemanticDigest:Sha256,
  delegationBinding:Binding<lease>/1,
  activationBinding:ActivationBinding/1,
  count:ApprovalCountReservation/1,
  budgetReservations:[ControlRef<reservation>/1...]
}

D10AuthorStepResponsibility/2 =
    {version:2,kind:"core_field_member",
     link:D10AuthorPreparationLink/2,
     decisionKey:DecisionKey/2,protocolOwner:"D6",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
  | {version:2,kind:"interactive",
     run:ControlRef<run>/1,stepId:Counter,decisionKey:DecisionKey/2,
     authorRequest:
         {protocolOwner:"D3",request:D3IdentityOperationRequest/13}
       | {protocolOwner:"D6",request:d6_commit_request/2},
     preparedFormat:
       "d7_prepared_action_binding4"|"d8_prepared_edit_binding3",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
```

对 fresh current core_field_member step，D10AuthorPreparationLink/2.preparedBindingToken 必须选择精确 PAB4，且该 PAB4 的原 request 必须等于 link.request。Core 必须在返回 prepared step 或允许 submission 前，原子保存 Link2、完整 PAB4 与所需 preview/effect/recovery pins。ApprovalUse/2.preparedBindingToken 选择同一个 PAB4。其 preview digest 固定为
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3))；
EffectBytes/3 slot 只投影为 {encoding,byteLength,payloadDigest}，不按成员名递归猜测。fresh current automatic author qualification 使用 DependencyProof/3；只要语义解析 managed Document，就必须包含 document_format，并且只有在完整验证 EffectManifest/3、EffectBytes/3 与 MutationFootprint 后才能构造 ApprovalUse/2。历史 Link1/PAB3/ApprovalUse1 的 saved 或 planned association 保留原 decoder、bytes、pins、request 与 OperationId。

**责任闭合分支分派：**`core_field_member` 必须持真实 Automation binding、不可变 definition revision/task digest、Link2 所选 D7 PAB4、符合条件的 Standing Approval 与 ApprovalUse2；`interactive` 没有 Automation/Link2/ApprovalUse2，而是以其 `preparedFormat` 将真实 D3IdentityOperationRequest/13 或 d6_commit_request/2 绑定到已保存的 D7 PAB4 或 D8 PreparedEditBinding/3。每条交互 step 的 Core 都保留原 owner 完整 prepared/effects/preview 证据、受保护 preparedRecordPin/recoveryPins，核对准确 request/OperationId/DecisionKey 关联，按原 D7/D8 preview 合同完整展示每页和必要 EffectBytes/3，并在原 owner 提交前要求真实认证自然人针对准确语义作出新的明确确认；Agent 建议、Start 事件、tool 布尔值或 standing approval 不能替代。D3 仍拥有 identity submit，D6 仍拥有唯一 D6 作者 ledger 与 final P。两臂共用同一已准入 Run/LeaseRunUse、Stop latch、Policy/authority/fence/custody、可信时钟、activation、真实读写 grant 和 budgets。saved/planned/unknown/冷恢复按**原保存分支与真实 prepared 版本**分派，保留原 request/DecisionKey/OperationId/pins；同 Run 受保护后续不二次扣 maxRuns，也不建第二作者成功记录。错误分支、虚构 Automation/Link2、缺人工确认、preparedFormat/pins/preview digest 不一致、跨 store 关联、撤权/Stop/预算失败均沿原 owner 既有错误优先次序拒绝，不降级偷走自动窄 profile。

对 fresh ScheduleSubscription/2 registration，current D6 Storage producer 只有在 selected source/Field/Registry/current scheduling gates 通过、有限 retention 已预留、且真实持续维护的 Core source/control transition producer 已在同一 configuration transaction 注册后，才能创建 ScheduleContinuityWitness/2。initial 与 checkpoint 使用 subscription 的精确 ScheduleRecurrenceEvidence/2，revision=1、consumedTransition=0，并生成 fresh producerEpoch。retained witness pin 必须精确使用上面的 Witness2ArtifactBytes。current 正向 transition 使用 ScheduleContinuityStep/2 与 DependencyProof/3；新的 current portable transition 使用真实 ChangeRecord/1、InstallationNotice/3 与 ContentCompletionProof/4 pins，每个 retained current step pin 都必须精确使用上面的 Step2ArtifactBytes。retained history 中的历史 transition 保持原 /1 artifact domain 与精确 decoder。相关 P-only control/rule transition 仍从真实 protected before/after state 捕获。

fold、compaction、receiver admission、D10 continuityPins consumption 与 recovery 都必须先按每个 protected schedule-continuity artifact 的精确 domain 分派，再解 inner object。current Witness2/Step2 不能放在 /1 domain 下接受，historical Witness1/Step1 也不能放在 /2 domain 下接受。continuityPins 是按 pinToken 排序且唯一的精确 PinRef/2；真实历史若跨过显式 same-generation bridge，可以保留 version-mixed original typed chain，但每个元素都保留自己的精确 bytes/domain/decoder。另一条合法 full retained-chain 路径同样保留每个 original typed artifact 与 source/control evidence，不能把整条 chain 归一化成一个版本。typed evidence 缺失或 unknown 时，在原 authorization/disclosure check 后沿用原 gap/unavailable 行为；已证明 domain/object mismatch 时不得用相同 digest/current state 修复。

当不存在合法 current after evidence 时，current producer 必须产生 ScheduleContinuityInvalidation/2，不能伪造 ScheduleRecurrenceEvidence/2。binding_changed 需要完整可信的 selected-business discontinuity 证据；after unavailable/unknown、missing history、unknown decoder、observer/producer gap 或无法保留必要 transition 均为 gap。Invalidation 比较同一 current witness/registration，原子推进下一 checked transition/revision，保留最后合法 checkpoint，并对该 generation 永久不可 reset。其 artifact pin 必须精确使用上面的 Invalidation2ArtifactBytes。fixed-parent inbox/capacity/final-counter reservation、authorization、compaction 与无关 source 可用性规则保持不变。

Schedule current proof 中真实解析 managed Document 时必须含 source + document_format dependency。source/profile bytes 未变但 format proof continuity gap 得到 gap；binding 发生真实改变得到 binding_changed，即使最终 recurrence/range 值碰巧相同。已有 Subscription1 继续作为 historical retention owner，并配套 Witness1/Step1/Invalidation1 及其精确 /1 artifact domain。它只有通过 explicit continue + complete retained history 证明没有 intervening format/rule/business discontinuity，并建立 current Evidence2/Proof3 cut，才可变成 same-generation Subscription2；否则必须 replace。这个 bridge 保留每个旧 pin 与 producer association；只有 bridge 之后新产生的 Witness2/Step2/Invalidation2 才使用 /2 domain。绝不把 version-1 witness/step/invalidation repin 或重新编码成 version 2，也绝不 reset invalid generation。



---


revision: D10-FA-r01-2026-10-02；状态：协调作者候选，未接受、未激活、未实现。最近一次完整历史 R08 评审绑定 C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` 与 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，结论 REVISE（P0=0、P1=3、P2=8）。十一项历史最终处置仍为 OPEN。具名修订已有限定独立复核，实际跨 owner 整合及 fresh 全局接受仍未完成；D10-REVIEW 区分各层证据。

## 0.3 首次新建与真实记录的唯一分派

本合同只消费同一个 D6 闭合生产者，不建立 D10 第二类型权威。首次 ControlPrepareBinding/3 在真实 D6 planning/seal 屏障冻结 ControlDependencies/3、D10WorkspaceReadDependencies/2、D10ControlInput/2、D10ControlEffectPlan/2。Automation 或 Run 记录的现任后像/pin 是 Image2/Pin2，其余**十七类仍使用合法的 Image1/Pin1**，通过唯一 mixed tagged carrier 分派；真实历史 Image1 Automation/Run 则继续走原 decoder。已有决议关联的 ControlPrepareBinding1/2、Dependencies1/2 保留原 tag、字节和责任链。新建**自动 core_field_member** D10 作者 step 才使用真实 Automation 的 D7 PAB4、EffectManifest3/EffectBytes3、D10AuthorPreparationLink2、ApprovalUse2 和原 D6 request；另一个**人工 interactive** step 使用原 D3 Request13 或 D6 request2、经 owner 资格验证的 D7 PAB4 或 D8 EditBinding3 和真实完整 preview 后的人工确认，不使用 Link2/Standing Approval；真实历史 PAB3/Link1/ApprovalUse1 的 preview digest 和 decoder 不能按新代数重签。Link2/ApprovalUse2 **仅**要求真实 Automation 的 `core_field_member` 分支；独立 `interactive` 使用真实 D3/D6 作者请求与 D7 PAB4 或 D8 PreparedEditBinding3、逐次明确确认，绝不强制 Link2/Standing Approval。原 Control §7 的公共 result/error 与十九类完整 current view 继续现行；§8.1/§8.2 中已被 §0 继任的 Image1/Plan1/ApprovalUse1 示例只在真实旧版本恢复范围有效。已安装 stop/Money/unknown 与未用容量不能因同步、I 重建、回执读取、重试、相同 digest 或显示名归零。ScheduleSubscription2、Witness/Step/Invalidation2、Inventory2/Record3 与 mixed claim/pin 只按 §0 全部精确 owner schema；不存在自造 StoreIncarnationProof 请求或第二持久决策账本。

## 1. 权威、适用范围与错误边界

Core 仍是唯一作者事务权威。Workspace 作者提交、saved decision、receipt 和 planned recovery 继续由 D6 拥有；本合同不得保存第二份作者成功 ledger。部署 trust、package、secret、external account、pricing 和资源使用授权属于 D10 host control domain，不授予 Workspace 内容权限。

公开能力在进入本合同前仍经过 D1 的真实 contractMajor、surface、release、policy、principal、component/configuration、reachability/version-combination 和 health 门。设计接受、文档协调激活或存在一个 control record 都不是运行时 available 证据。


D10 管理/control 入口的错误对象为：

    D10ControlError/1 {
      kind:"d10_control_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "authority_unavailable" | "integrity_conflict" |
        "control_conflict" | "stale_revision" |
        "state_unavailable" | "budget_exceeded"
    }

独立的 D10 Run/step 错误对象为：

    D10RunStepError/1 {
      kind:"d10_run_step_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "control_conflict" | "binding_changed" |
        "approval_required" | "approval_expired" |
        "delegation_expired" | "delegation_exhausted" |
        "budget_exceeded" | "audit_unavailable" |
        "state_unavailable" | "invalid_output" |
        "cancelled" | "external_outcome_unknown"
    }

`D10ControlError/1` 只用于 `d10_control_prepare`、`d10_host_control_commit`、`d10_control_result`、`d10_control_read`、`d10_interactive_run_start`、`d10_interactive_run_result`、`d10_secret_stage`、`d10_emergency_stop` 和 `d10_emergency_stop_result`。`D10RunStepError/1` 只用于尚未进入其它 protocol owner 的 D10 Run/step：Run admission、ContextBundle/model/tool/connector 执行、D6 前 approval/delegation 检查以及 external-effect 执行/恢复。

一旦进入 D3、D6、D7、D8 或 D9 入口，就逐字返回该 owner 原 closed error/envelope。D10 不包装 D6 `not_visible`、`approval_unavailable`、`execution_stopped`、`transaction_aborted`，不包装 D7 action/effects 错误，也不改名 D3/D8/D9 错误。诊断 UI 可在 owner error 后另做当前受权的 `d10_control_read`，但该读取不得改变正式结果。

管理/control 固定顺序：closed decode 与 D1 capability 门 → 当前 principal/scope/audience/对象观察资格 → authority/fence/custody → stable-key conflict 或 exact ControlRef lookup → 受保护记录 integrity/continuity → 仅在适用时检查当前 revision/dependency/budget。对象缺失、wrong scope/audience、当前无观察权统一 `not_visible`；authority/fence/custody 不可证明为 `authority_unavailable`；受保护记录/证据确定矛盾为 `integrity_conflict`；同 stable key 不同完整输入为 `control_conflict`；当前可见对象的 expected configuration revision 不匹配为 `stale_revision`；可信时间、外部证据或必要连续性暂不可证明为 `state_unavailable`。

Run/step 继续使用 Candidate §21 顺序：closed decode/version → D1 static capability/surface/release → current principal/control visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution。阶段先于细分；`binding_changed` 只属于 Run/step，管理 CAS 用 `stale_revision`；D6 前的 `approval_required|approval_expired` 永不替代进入 D6 后由 D6 owner 返回的 `approval_unavailable/preflight`。

## 2. 共享闭合类型

下列类型均为 D10 本代正式合同。所有 object 拒绝 unknown member、duplicate member 和非法 null。Counter 逐字复用 D6 的 0..2^63-1 非 Boolean integer；Uuid 使用 canonical lowercase RFC 4122 text。所有数组有界、排序和唯一性规则由包含类型指定。

    Version/1 = {
      major:Counter, minor:Counter, patch:Counter
    }

    VersionRange/1 = {
      minimum:Version/1,
      maximumExclusive:Version/1
    }

VersionRange 必须满足 minimum < maximumExclusive。不存在 latest、wildcard 或 ambient package version。

    Scope/1 =
      {kind:"workspace", workspaceRef:D3.WorkspaceRef}
    | {kind:"deployment", storeIncarnation:Uuid}

ControlRecordKind/1 闭集为：
受控记录类别闭集为：`automation, lease, approval, planned_approval, external_approval, run, workspace_budget, activation, deployment_policy, trust, package, external_account, secret, grant, cost_account, pricing, reservation, external_effect, stop`。

    ControlRef<K>/1 = {
      storeIncarnation:Uuid,
      kind:K,
      id:Uuid
    }

    Binding<K>/1 = {
      ref:ControlRef<K>/1,
      revision:Counter
    }

    Target<K>/1 =
      {kind:"new"}
    | {kind:"existing", binding:Binding<K>/1}

control id 与 incarnation 永不重用。retire、archive、revoke、close 或删除显示入口都不允许以后用同 id 指向另一对象。相同显示名称重建必须得到新 id。


`Binding<K>/1.revision` 是该 control record 唯一的 configuration/lifecycle CAS revision。已有 owner 类型若已有具名 revision，则它与 Binding revision 是同一值而非第二权威：`leaseRevision == Binding<lease>.revision`、`approvalRevision == Binding<approval|planned_approval|external_approval>.revision`、`grantRevision == Binding<grant>.revision`、`CostReservation.revision == Binding<reservation>.revision`、`DeploymentControlPolicy.revision == Binding<deployment_policy>.revision`；`ActivationBinding.activationGeneration == Binding<activation>.revision`。Automation 特意保留独立 `definitionRevision`：enable/disable/archive 只增加 control revision，不改变不可变的 semantic definition revision。

独立累计 usage CAS 只有 `lease`、`approval`、`grant`、`cost_account` 使用 `usageRevision`。普通 Run admission 只推进 Lease usage，不改变 `leaseRevision`，因此另一 Run 消耗容量不会让已经 admitted 的同 Run 因 revision 漂移失效。Approval reserve/consume/released_terminal 同样只推进 approval usage，不改变 `approvalRevision`。Grant 沿用 §6 已有 usageRevision；cost-account held/spent 在下文 §7 current projection 中使用同类独立 revision。配置/lifecycle 变化不能清零累计 usage，usage 变化也不能把 revoked/retired/closed 配置复活。

    Money/1 = {
      currency:CurrencyCode,
      microUnits:Counter
    }

CurrencyCode 是 exact 三位 ASCII A-Z。D10 不执行隐式外汇换算。

本文其余基础标量沿用现有 closed JSON 语义：`Token` 是 D6 §1 的非空 opaque token；`Text` 是 Unicode scalar string；`Bytes` 是受相应入口字节预算约束的 byte sequence；`Boolean` 只接受 JSON true/false；`Sha256` 是 `sha256:` 加 64 位 lowercase hex；`HostPrincipal` 是受信 host authentication 映射得到的 `Token`。当前主体身份永远不能由请求自报；但已经通过 H 授权的管理操作可以在 policy、reconciler 或 ResourceUseGrant 的**目标主体字段**中填写另一个 `HostPrincipal`，这表示被管理的受权对象，不表示当前主体冒充该目标主体。`Ed25519PublicKey` 与 `Ed25519Signature` 只在受信 package/trust adapter 中出现，编码格式必须由该 adapter 的已接纳 profile 固定，普通 control caller 不能自报已验证。

    HostOrWorkspacePrincipal/1 =
        {kind:"workspace",
         workspaceRef:D3.WorkspaceRef,
         principal:Token}
      | {kind:"deployment",
         storeIncarnation:Uuid,
         principal:HostPrincipal}

`HostOrWorkspacePrincipal/1` 是 D10 对成功 safety transition 线性化 cut 上受信 authenticated actor 的 closed projection。workspace arm 把当前 D6 认证的 Workspace principal 与准确 WorkspaceRef 绑定；deployment arm 把 H 认证的 HostPrincipal 与实际 D10 storeIncarnation 绑定。两支都不是 caller 自报值、作者 EntityRef 或 capability token。

受控 ASCII token 语法：

```text
LowerCamelAscii = [a-z][A-Za-z0-9]{0,62}
LowerKebabAscii = [a-z][a-z0-9]*(?:-[a-z0-9]+)*
CanonicalInteger = "-"? ("0" | [1-9][0-9]*)
CanonicalDecimal = CanonicalInteger ("." [0-9]*[1-9])?
```

`LowerKebabAscii` 总长 1..63 bytes；`CanonicalInteger` 和 `CanonicalDecimal` 必须在相应 ToolType 的显式 bounds 内。decimal 不允许尾随零、小数点后空串、指数写法或 negative zero。

    ScheduleHorizon/1 = {
      start:D4.zoned_instant,
      endExclusive:D4.zoned_instant
    }

ScheduleHorizon 不是 D4 新类型；两个成员逐字使用 D4 zoned_instant 作者值的时间语义，并由同一 exact UTC comparator 证明 start < endExclusive。本文不使用不存在的 “D4 bounded-instant-range” 名称。

## 3. Tool Value Profile

ToolValueProfile/1 由 D10 Tool Adapter 拥有，不是 D7 TypeSpec 的别名，也不是任意 JSON Schema。它只提供外部工具参数和结果的有限值代数。

    ToolValueProfile/1 = {
      kind:"d10_tool_value_profile",
      wireVersion:1,
      input:ToolType/1,
      output:ToolType/1
    }

    ToolType/1 =
      {kind:"bool"}
    | {kind:"text", maximumUtf8Bytes:Counter}
    | {kind:"int64"}
    | {kind:"integer", minimum:CanonicalInteger, maximum:CanonicalInteger}
    | {kind:"decimal", minimum:CanonicalDecimal, maximum:CanonicalDecimal}
    | {kind:"optional", item:ToolType/1}
    | {kind:"object", members:[ToolMember/1]}
    | {kind:"list", minimum:Counter, maximum:Counter, item:ToolType/1}
    | {kind:"union", arms:[ToolArm/1]}

    ToolMember/1 = {
      name:LowerCamelAscii,
      required:Boolean,
      type:ToolType/1
    }

    ToolArm/1 = {
      tag:LowerKebabAscii,
      type:ToolType/1
    }

`maximumUtf8Bytes` 取 1..8388608；对象最多 64 个 `members`；联合类型有 2..8 个 `arms`；列表 `maximum` 为 1..4096 且 `minimum <= maximum`；完整类型深度最多 16，规范类型字节最多 65536。成员名和联合标签按 UTF-8 字节排序且唯一。整数和十进制定点值使用规范十进制字符串，禁止二进制浮点、NaN、Infinity 和负零。

ToolValue/1 exact wire 为：

```text
ToolValue/1 =
  {kind:"bool", value:Boolean}
| {kind:"text", value:Text}
| {kind:"int64", value:CanonicalInteger}
| {kind:"integer", value:CanonicalInteger}
| {kind:"decimal", value:CanonicalDecimal}
| {kind:"optional", value:{kind:"none"} | {kind:"some", value:ToolValue/1}}
| {kind:"object", members:[{name:LowerCamelAscii, value:ToolValue/1}]}
| {kind:"list", items:[ToolValue/1]}
| {kind:"union", tag:LowerKebabAscii, value:ToolValue/1}
```

ToolValue/1 必须和调用点绑定的 ToolType/1 逐层一致。object 的 members 按 name 排序且唯一，只能出现声明成员；list 数量满足 bounds；union tag 必须命中声明 arm。int64 还必须落在 signed 64-bit 范围。

缺少必需成员、出现额外成员、选择错误联合分支、越界或超预算都返回 invalid_request。ToolValue 不能承载 EntityRef、Locator、SecretRef、文件路径能力、ActionEvidence、plan/result token、开放 map 或可执行值。

## 4. Package、Contribution 与依赖

PackageId/1 与 D4 SemanticNamespaceId 是不同类型，即使字符串可能相同。PackageId 使用 1..127 ASCII bytes 的点分 lower-kebab segments；LocalContributionId、ExtensionPointId 和 LocalOperationId 各为 1..63 ASCII bytes lower-kebab。比较全部 exact ASCII。

    PackageManifest/1 = {
      kind:"d10_package_manifest",
      wireVersion:1,
      packageId:PackageId,
      packageVersion:Version/1,
      publisherKeyId:Sha256,
      assets:[PackageAsset/1],
      contributions:[Contribution/1]
    }

    PackageAsset/1 = {
      assetId:LowerKebabAscii,
      digest:Sha256,
      byteLength:Counter
    }

    Contribution/1 = {
      contributionId:LocalContributionId,
      kind:ContributionKind,
      contractVersion:Version/1,
      descriptorAssetId:LowerKebabAscii,
      dependencies:[ContributionDependency/1]
    }


`view` Contribution descriptor asset 使用 closed root dispatch：要么是 D7 原 owner 的 `ViewSpec`，要么是下列只承载 D7-owned 纯数据 SearchContribution 的 D10 carrier：

    D10D7SearchDescriptor/1 = {
      kind:"d10_d7_search_descriptor",
      wireVersion:1,
      search:<S D7 Query Algebra §6 的 exact SearchContribution>
    }

这不会把 `SearchContribution` 变成 ViewSpec，也不新增 `search` ContributionKind。一份 D10 Contribution 恰承载一份 D7 SearchContribution；内部对象继续逐字是 D7 的 `{contributionId,version,fieldId,textPath,role}`，沿用原 D7 owner、D4-style semantic contribution ID 词法、正 Counter version、0..8 静态 text path 与 `name|alias|content` role。

R08 同时冻结一个具名、版本化的第一方 Core author adapter。它不是 generic Tool callback，也不会把 ToolValue 变成 D7 value alias：

    CoreFieldMemberAdapterDescriptor/1 = {
      kind:"d10_core_field_member_adapter",
      wireVersion:1,
      actionKind:"set_field_member"
    }

首代唯一 adapter identity 是 packageId `weftext.automation`、package-local contributionId `set-field-member`、Contribution kind `action`、contractVersion `{major:1,minor:0,patch:0}`。packageVersion 与 descriptorDigest 仍逐字使用普通第一方 PackageManifest/ContributionBinding 接纳规则。该 package 不是 Bundled Module，也不创建 D4 namespace 或作者事实。

    SingleFieldMemberPath/1 =
      [D4.ObjectMemberSpec.name]   // exact length 1..7

    FieldMemberTask/1 = {
      ownerNodeRef:D3.NodeRef,
      fieldId:D4.FieldId,
      selection:"require_exactly_one_entry",
      memberPath:SingleFieldMemberPath/1,
      value:D7.TypedLiteral
    }

`FieldMemberTask.value` 是原 D7 Action literal，不是 approval scalar。它只能是一个允许的 `SingleFieldMemberScalarType/1`，或恰一层 item 为该 scalar、value state 为 `some` 的 D7 Optional wrapper。`optional.none`、Ref/Locator/control token、object、list、set、union、nested Optional，以及把 ToolValue text 提升为 Ref/FieldId 都拒绝。D4 optional member 因而保留原 D7 Optional bridge，而 `SingleFieldMemberRule.memberType` 比较 present 的底层 scalar。

    D10AuthorPreparationLink/1 = {
      run:ControlRef<run>/1,
      stepId:Counter,
      automation:Binding<automation>/1,
      definitionRevision:Counter,
      taskDigest:Sha256,
      preparedBindingToken:Token,
      request:D6.d6_commit_request
    }

这是受保护的 Core recovery link，不是 public request，也不是第二 author decision。`preparedBindingToken` 是原 D7 PreparedActionBinding/3 的准确 token，`request` 是其原 D6 request。Core 在返回 prepared author step 或允许提交之前，必须把此 link、原 PreparedActionBinding/3 与必要 pins 原子保存。

ContributionKind 闭集为：
贡献类型闭集为：`module, schema, view, action, template, preset, pack, tool, model, connector, importer, exporter, conversion, renderer, localization`。

这组 kind 覆盖 Mandatory Intake 已选的 module、Profile/schema、View、Action、Template、Preset、Pack、Connector/Adapter，以及 D9 import/export/conversion 和 D10 tool/model/connector 贡献。`runtime` 不是独立 `ContributionKind`；它只表示上述可执行 contribution 所使用的 D10 运行基础设施。kind 列表不表示所有类别已产品实现。

    ContributionDependency/1 = {
      role:"primary_parent" | "additional",
      parent:{
        packageId:PackageId,
        contributionId:LocalContributionId,
        extensionPointId:ExtensionPointId
      },
      requiredContractRange:VersionRange/1
    }

dependency 是具体 dependent Contribution 的成员，不存在 package 级 ambient dependency。pack kind 必须恰有一个 primary_parent；其他 contribution 至多一个 primary_parent，可以有 0..32 个 additional。parent tuple 必须解析到 exact activated contribution 和 extension point。resolved parent contract version 与 dependency result 进入 Capability Catalog digest；不能跟随 latest。

一个 connector contribution 不可用只使该 contribution inactive；同 package 的 template、schema、pack 或 view 只按自己的依赖和能力计算。定义历史、Contribution activation、UI visibility 继续保持三个独立轴。

每个 kind 的 descriptor 由既有 owner 的闭合合同验证：schema 只引用 D4 Registry 中已有的 namespace/Facet bindings；view/action 消费 D7 closed descriptors；template/preset/importer/exporter/conversion 消费 D9 合同；tool/model/connector 的 descriptor 与运行基础设施由 D10；module 只组织产品贡献，不拥有作者事实。


对 `D10D7SearchDescriptor/1`，进入当前 Capability Catalog 前还必须完整证明：

1. `descriptorAssetId` 精确命中一个 `PackageAsset/1`；不可变 bytes 的 `byteLength` 与 SHA-256 均匹配，根 strict-decode 为 `d10_d7_search_descriptor`，active `ContributionBinding.descriptorDigest` 逐字等于同一 asset digest。
2. D10 PackageId/packageVersion、D10 package-local `Contribution.contributionId`、D10 `contractVersion`、D7 `SearchContribution.contributionId`、D7 正 Counter `version` 是五个不同 identity/version 域；名字或数字相同不产生映射。即便 D10 contractVersion 不变，D7 search 任一 member/version 改变也必须改变 Catalog digest。
3. D7 SearchContribution ID 的 namespace 由当前 D4 owner tuple + D10 trust/NamespaceClaim proof 证明；`fieldId` 另行在准确 `RegistryBinding/1` 下解析，证明 namespace owner、Field current-available，并把完整 alias-expanded `textPath` 验证到 D7 text 或 Optional<text>。不要求 SearchContribution ID namespace 与 Field namespace 相同。
4. 一个当前 Catalog 内 D7 SearchContribution ID 唯一；全部 active 且 descriptor root 为 `d10_d7_search_descriptor` 的 `view` Contributions 组成完整 SearchContribution 集，按 D7 contributionId canonical bytes 排序。遗漏、重复、wrong owner、wrong digest、坏 path、必要 Field unavailable 都使 successor activation fail closed，旧 ActivationBinding 继续 current。
5. `ActivationBinding.activationGeneration`、`capabilityCatalogDigest` 与准确 D4 `registryBinding` 共同绑定完整集合。D7 explicit selection 绑定所选 `(contributionId,version)`；D7 `all` 绑定该 generation 的完整集合。新增/删除/version/descriptor/owner/Registry 变化使旧 search dependency/result 失效，不能静默改变字段。
6. carrier 只是纯数据，不授 `field_read`、network、secret、script、私有全文 index payload、author write 或 alias-source 权威。执行仍是 D7 `scan → explicit read → CEL match/rank → sort/project` 且逐项通过当前 D6 授权；缺失/不可用的 selected contribution 走原 D7 unavailable，不能静默跳过后宣称空结果成功。

第一方正例：`weftext.people` package 的合法 `view` carrier 可承载 D7 SearchContribution `people/search-name`、version `1`、`fieldId:"people/name"`、`textPath:["text"]`、`role:"name"`。只有 D4 保留 tuple `people→(first_party,weftext.people)`、asset `digest`、当前 Registry proof 与完整 Catalog 都成立时才能激活；同名第三方 package、wrong digest 或仅 runtime discovery 都保持 inactive/pending。

## 5. 四个第一方 module 的唯一 package 映射

本节是 D10 新候选映射，不冒称 D1 已定义 package/module ID、代码路径或 locale 资源。D4 已冻结的 semantic namespaces、owner tuples、FacetId 和 semanticMajor 原样保持。

| 产品 | PackageId | module contribution | schema contribution | D4 semantic owner / Facet | extension points |
| --- | --- | --- | --- | --- | --- |
| 日历 / Calendar | `weftext.calendar` | `module` | `schema` | `calendar` → `first_party,weftext.calendar`；`calendar/period-note`、`calendar/range-note`、`calendar/event`，`semanticMajor=1` | `calendar-system`、`holiday-schedule` |
| 文献库 / Library | `weftext.library` | `module` | `schema` | `library` → `first_party,weftext.library`；`library/work`，`semanticMajor=1` | `none` |
| 人物 / People | `weftext.people` | `module` | `schema` | `people` → `first_party,weftext.people`；`people/person`，`semanticMajor=1` | `none` |
| 组织 / Organizations | `weftext.organizations` | `module` | `schema` | `organizations` → `first_party,weftext.organizations`；`organizations/organization`，`semanticMajor=1` | `schema-pack` |

每个 module contribution 的 descriptor 只包含正式中英文产品名、同 package schema contribution 引用以及上表 extension points。schema descriptor 只列 exact SemanticNamespaceId、D4 namespace ownerId 和 FacetId/semanticMajor，不复制 FieldDefinition 或 FacetSchema bytes。

PackageId、Contribution contractVersion、D4 semanticMajor 是三个独立版本域。package update 不得在同 FacetId/semanticMajor 下改变 D4 semantic digest。

候选 code convention 分别为 CalendarModuleContribution、LibraryModuleContribution、PeopleModuleContribution、OrganizationsModuleContribution；候选 variables 为 calendar_module、library_module、people_module、organizations_module。候选代码 namespace d10::bundled::<domain> 与候选 locale keys module.calendar.title、module.library.title、module.people.title、module.organizations.title 都是设计映射，当前没有实现证据。没有独立 CLI verb；通用管理入口只消费完整 package/contribution ref。

禁止把 D4 ownerId 自动解释为 package ownership proof；上表只建立显式一对一映射。第三方签名、安装顺序、显示名或相同字符串都不能取得 first-party package/namespace。

## 6. Principal、管理域与 ResourceUseGrant

请求不能携带 principal、owner 或 authorized 布尔值。当前主体只能来自受信 host/D10 authentication mapping。

Workspace 自助资格 S：当前 Workspace Policy/3 对当前主体、workspace scope 显式 allow 协调 capability d10_control_self。它只允许管理自己的有限 D10 控制记录，不授予 author read/write、policy_admin、registry_admin、部署账户管理或 secret 原值读取。实际 author 操作仍逐项要求原 D6/D7 权限。

Workspace 管理资格 W：当前 Workspace、workspace scope 的 policy_admin；涉及 Registry activation 还必须有 registry_admin。W 可以停止、撤销、归档工作区控制记录和管理工作区预算，但不能冒充另一主体创建或扩大其 Lease/Approval。

Deployment 管理资格 H 由 DeploymentControlPolicy/1 拥有：

    DeploymentControlPolicy/1 = {
      kind:"d10_deployment_control_policy",
      wireVersion:1,
      revision:Counter,
      administrators:[HostPrincipal],
      reconcilers:[{
        principal:HostPrincipal,
        costAccountIds:[ControlRef<cost_account>/1]
      }]
    }

初始 H 只能由受信本地主机显式建立或 Server 部署 operator 配置；首次 HTTP、Workspace ownership、allocate_workspace 或 administer_issuer 不自动得到 H。普通 reconciler 只能对列出的 cost account 做证据驱动 settlement。

    ResourceUseGrant/1 = {
      kind:"d10_resource_use_grant",
      wireVersion:1,
      grantId:Uuid,
      grantRevision:Counter,
      usageRevision:Counter,
      state:"active" | "revoked" | "retired",
      grantee:{
        hostPrincipal:HostPrincipal,
        workspacePrincipal:Token,
        workspaceRef:D3.WorkspaceRef
      },
      contributions:[ContributionBinding/1],
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      permission:ResourcePermission/1,
      usage:ResourceUsage/1
    }

    ContributionBinding/1 = {
      packageId:PackageId,
      packageVersion:Version/1,
      contributionId:LocalContributionId,
      contractVersion:Version/1,
      descriptorDigest:Sha256
    }

    ResourceUseGrantSpec/1 = {
      grantee:{
        hostPrincipal:HostPrincipal,
        workspacePrincipal:Token,
        workspaceRef:D3.WorkspaceRef
      },
      contributions:[ContributionBinding/1],
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      permission:ResourcePermission/1
    }

`ResourceUseGrantSpec/1` 是创建/更新输入；`grantId`、两个 revision、state 和 usage 只能由 Core 从现有记录与事务结果产生。

ResourcePermission/1 四个 variant：

    {kind:"cost",
     account:Binding<cost_account>/1,
     limits:{currency:CurrencyCode,
             totalMicroUnits:Counter,
             perAttemptMicroUnits:Counter,
             maxAttempts:Counter}}

    {kind:"secret",
     secret:Binding<secret>/1,
     account:Binding<external_account>/1,
     audience:ContributionBinding/1,
     usageKind:"authenticate",
     limits:{maxUses:Counter}}

    {kind:"egress",
     account:Binding<external_account>/1,
     operations:[LocalOperationId],
     limits:{maxCalls:Counter,
             totalBytes:Counter,
             perCallBytes:Counter}}

    {kind:"external_effect",
     account:Binding<external_account>/1,
     calls:[{operation:LocalOperationId,target:ToolValue/1}],
     limits:{maxAttempts:Counter}}

同 grant 中 permission kind 唯一；secret 不蕴含 egress，egress 不蕴含 mutation，cost 不蕴含 account manage。cost 要求 perAttemptMicroUnits <= totalMicroUnits；egress 要求 perCallBytes <= totalBytes；所有可执行上限为有限正数，cost 金额可为 0。

ResourceUsage/1 与 permission kind 一一对应：
cost={spentMicroUnits,heldMicroUnits,attemptsStarted}；
secret={usesStarted}；
egress={callsStarted,bytesSent,bytesHeld}；
external_effect={attemptsStarted}。
这些 Counter 与仍在途的 attempt reservation 一起构成准入依赖。

同 grantId 的续期、限额调整或 grantRevision 更新不得清零 usage、held、spent 或 attemptsStarted；usageRevision 独立单调增加。降低限额不得低于已消费加仍 held 的金额/次数。改变 grantee、resource kind、cost account、external account 或 currency 必须创建新 grantId。新 grant 也不能消除旧 attempt、old grant 或实际 account 的费用义务；所有 grant 继续竞争同一实际 account ceiling。

## 7. Stable prepare、closed body 与公共入口

普通控制入口：

    D10ControlPrepare/1 = {
      kind:"d10_control_prepare",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      body:ControlBody/1
    }

ControlBody/1 只有七个 variant：

1. automation_configure：
   automation:Target<automation>；
   lease:Target<lease>；
   approval:Option<Target<approval>>；
   definition:AutomationSpec/1；
   scheduleUpdate:AutomationScheduleUpdate/1；
   delegation:LeaseSpec/1；
   standing:Option<StandingApprovalSpec/1>。
   approval 与 standing 必须同为 none 或同为 some。该固定 bundle 是唯一允许一起创建/重绑 Automation、Lease、Standing Approval 的组合操作，不是通用 batch/DAG。

2. consent：
   target:Target<planned_approval|external_approval>；
   consent:ConsentSpec/1。

3. state：
   target:Binding<K>；
   action:StateAction。
   K/action 适用闭集：
   automation→enable|disable|archive；
   lease/approval→revoke|archive；
   grant→revoke|archive，其中 `archive` 映射到已有 `retired` 状态；
   run→cancel|archive；
   trust→revoke；
   package/pricing→retire；
   external_account→disconnect；
   cost_account→close；
   secret→revoke。
   不存在 generic delete 或 revoked→active。

4. workspace_limits：
   target:Binding<workspace_budget>；
   limits:BudgetCaps/1。
   仅 W。

5. activation：
   target:Target<activation>；
   packages:[ContributionBinding/1]；
   registrySnapshot:D4.RegistrySnapshot/1；
   registryEvolution:Option<D4.RegistryEvolutionProof/1>；
   trust:Binding<trust>。
   仅 W 且 registry_admin。非 bootstrap Registry 必须带 evolution proof。

6. deployment_put：
   target:Target<K>；
   value:DeploymentValue/1。
   K 只允许 `deployment_policy|trust|package|external_account|secret|grant|cost_account|pricing`；K 必须与 `value.kind` 对应，而且只有 H 可以执行。

7. cost_reconcile：
   reservation:Binding<reservation>；
   evidence:EvidenceTicket/1。
   仅 H 或该 account 的 reconciler；没有 targetState、actual 或 manual amount 字段。

Option<T> 只有 {kind:"none"} 或 {kind:"some",value:T}，禁止 null。

    BudgetCaps/1 = {
      maxSteps:Counter,
      maxInputBytes:Counter,
      maxOutputBytes:Counter,
      maxElapsedMillis:Counter,
      costs:[{
        grant:Binding<grant>/1,
        maximum:Money/1
      }]
    }

所有 maxSteps/maxInputBytes/maxOutputBytes/maxElapsedMillis 必须正且有限；costs 按 grant ref 排序唯一，currency 必须匹配 grant/account。 costs 为 0..32 项；Core 在配置时将每个 grant 准确解析为实际 account/currency。同一 BudgetCaps 中指向同一实际 account/currency 的多项 maximum 必须相等，否则 invalid_request；这些项共同表达一个 owner/account ceiling，各 grant 自身限制仍独立。某层无对应账户 cap 就不允许经过该层的新收费 attempt。换 grant 不改变 §10 的 owner/account 累计键。

    AutomationInvocation/1 =
        {kind:"tool",
         contribution:ContributionBinding/1,
         parameters:ToolValue/1}
      | {kind:"core_field_member",
         contribution:ContributionBinding/1,
         task:FieldMemberTask/1}

    AutomationSpec/1 = {
      label:Text,
      invocation:AutomationInvocation/1,
      schedule:AutomationSchedule/1,
      missedPolicy:"skip" | "run_once",
      missedWindowSeconds:CanonicalDecimal,
      queueLimit:Counter,
      budgets:BudgetCaps/1
    }

    AutomationSchedule/1 =
      {kind:"once", at:D4.zoned_instant}
    | {kind:"recurrence",
       ownerNodeRef:D3.NodeRef,
       recurrenceOccurrenceKey:D4.occurrenceKey,
       rangeOccurrenceKey:D4.occurrenceKey,
       horizon:ScheduleHorizon/1,
       outputLimit:Counter}

`tool` parameters 必须由对应 Contribution 的 active ToolValueProfile/1 输入类型验证。`core_field_member` 只接受上文准确接纳的第一方 adapter identity 和 closed `FieldMemberTask/1`；它不使用 ToolValueProfile，也不增加任何 read/write 权限。recurrence selector 只定位同一 revision 重新绑定的 D4 recurrence source，不成为 durable EntityRef。

    LeaseReadGrant/1 = {
      scope:<exact current D6 Policy/3 grant.scope>,
      capabilities:[LeaseReadCapability/1]
    }

    LeaseReadCapability/1 =
      {kind:"field_read", fieldIds:[D4.FieldId]}
    | {kind:
        "workspace_state" | "entity_state" | "locator_state" |
        "source_read" | "resource_read" | "annotation_read" |
        "source_envelope_state"}

`LeaseReadGrant/1` 是 D10 的减权投影，不是新的 D6 grant wire。它逐字消费 S D6 Control §4 的 scope/capability decoder 和 scope 适用矩阵：例如 `workspace_state` 只允许 workspace scope，`entity_state|locator_state|source_envelope_state` 可使用原 workspace/ref_set 规则，Field read 使用 D6 原 `field_read` 形状。它禁止 write、admin、repair、audit、export、`commit_sequence_state` 与 `d10_control_self`，因此只能收窄调用主体已经拥有的读取/状态观察资格。

    LeaseSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      maxRuns:Counter,
      activationBinding:ActivationBinding/1,
      capabilityAllowlist:[D1.CapabilityId],
      readGrants:[LeaseReadGrant/1],
      resourceGrants:[Binding<grant>/1],
      budgets:BudgetCaps/1
    }

`readGrants` 只接受上面的 `LeaseReadGrant/1`；实际授权仍由当前 D6 Policy 按原 deny precedence 计算，Lease 不能增加权限。`notBefore < notAfter`，`maxRuns` 正且有限。


一个 `core_field_member` Run step 只能按下面的确定映射执行，不存在 callback dispatch：

1. 绑定当前 Automation definitionRevision、准确 invocation、ActivationBinding、Lease、当前 D6 principal 与全部有限预算。
2. 在读取值之前，针对配置的 owner/Field/member 执行原 D7 Narrow Field Qualification 完整图，并证明所有必要的当前观察/写入 scope。Core 内部可以做完整 source 验证，但窄主体不能因此收到隐藏 source bytes。
3. 读取 `task.ownerNodeRef/task.fieldId` 的完整 current Field。自动执行要求恰一 Entry，且配置 member 已经 present。从该准确前像取得当前完整 SourceObservation/1、对应 SourceVersionRef/1、真实 managed SourceVersion/2.revision 作为 `expectedRevision`、真实 `occurrenceKey` 与原完整 `rawEntrySource`；Automation 配置不持久保存旧 selector 或 sourceToken。外部版本不能用 externalSequence 冒充 managed Counter。
4. 逐字构造原 D7 intent：`{format:"weftext.action",version:1,intent:{kind:"set_field_member",selector:{owner:task.ownerNodeRef,fieldId:task.fieldId,expectedRevision:<fresh owner source revision>,occurrenceKey:<the unique current Entry key>,rawEntrySource:<the exact original Entry JSON text>},memberPath:task.memberPath,value:task.value}}`。
5. 调用原 d7_action_prepare/2，以该 Field owner 的当前 SourceVersionRef/1 构成恰一项 selectedSources，并绑定相同 Workspace、当前 commitDomain、完整 expectedFrontier 和原 budget；保留其 PreparedActionBinding/3，读取并验证完整 preview/effects/MutationFootprint，再用**实际** member-change 或逐字 raw-no-op 与独立的当前 Standing Approval 比较。Approval 从不提供 target、Entry、memberPath 或本次 requested value。
6. Core 从原 prepared semantics 构造 ApprovalUse，并用原 `d6_commit_request` 进入 D6；planning/final/replay 继续由 D6 owner。

对 S `people/phone.label` 这样的 optional member，task.value 是 D7 Optional TypedLiteral：`{type:{kind:"optional",item:{kind:"semantic_code",scope:<the complete people contribution-set scope>}},value:{state:"some",value:"people/work"}}`。D4 scope 仍完整包含 `people/other|people/personal|people/work` 三个 code；approval enum 可以有意只允许其中子集。当前恰一 phone Entry 且 present `personal→work` 是真实 member-change；present `work→work` 只有完整 proposed source 与 before bytes 逐字相等时才进入既有 raw-no-op。当前 phone Entry 为零或多条时自动 profile 不适用；用户仍可交互选择第二条同值 phone 的真实 D7 selector，走普通 D7 确认路径。

所有原 D2/D4/D6/D7 限额继续生效，包括 D4 raw Entry 65,528 bytes 与既有完整 source/header/carrier/entry/check budgets。Core 内部完整 source 验证不会扩大 public disclosure。重启后若 `D10AuthorPreparationLink/1` 存在，Core 只能恢复该准确原 PreparedActionBinding/request。若不存在，只有在能证明原子保存从未成功且 request 从未交付时才可新 prepare。若存在性/连续性未知则返回 `state_unavailable`；planned 或 submitted-unknown 必须恢复原 request，绝不能新建 OperationId。

    StandingApprovalSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      rule:SingleFieldMemberRule/1,
      maxSuccessfulCommits:Counter,
      costGrants:[Binding<grant>/1]
    }

`D10-CONTROL` 是首版自动作者规则唯一 owner：

    SingleFieldMemberRule/1 = {
      kind:"single_field_member",
      ownerNodeRef:D3.NodeRef,
      fieldId:D4.FieldId,
      selection:"require_exactly_one_entry",
      memberPath:[D4.ObjectMemberSpec.name],
      memberType:SingleFieldMemberScalarType/1,
      valueConstraint:SingleFieldMemberValueConstraint/1
    }

    SingleFieldMemberScalarType/1 =
        {kind:"bool"}
      | {kind:"text"}
      | {kind:"int64"}
      | {kind:"integer"}
      | {kind:"decimal"}
      | {kind:"semantic_code",
         scope:<S D7 Value §5.2 已冻结 ResolvedCodeScope 的 exact structure>}

    SingleFieldMemberValueConstraint/1 =
        {kind:"enum", values:[D7.TypedLiteral]}
      | {kind:"numeric_range", minimum:D7.TypedLiteral, maximum:D7.TypedLiteral}
      | {kind:"text_utf8_no_crlf", maximumUtf8Bytes:Counter}

`memberPath` 恰为 1..7 个名字；每项逐字使用 D4 `ObjectMemberSpec/1.name` decoder。从完整 alias-expanded Field valueType 根开始，每个非终端只能选择 direct object member；禁止 list/set/sequence index、union arm selector、wildcard、JSON Pointer、运行时字符串和动态 FieldId。终端 member 必须在 current Entry 中实际 present。D4 根 depth=1、maximum=8，alias expansion 仍消耗原 depth，所以 1..7 只是绝对外层上限，不能绕过原 D4 depth/schema/source gate。

`memberType` 是底层 scalar D7 bridge type。D4 required member 的原 D7 `set_field_member` Action literal 类型必须直接等于该 TypeSpec；D4 optional member 的 Action literal 必须是 exact `{kind:"optional",item:<memberType>}` 且 value 为 `some`，Core 只把 present scalar 抽出用于 rule/constraint 比较。`optional.none` 永不自动批准。这样保留 D7 原 optional-member bridge，不把 scalar 域与 Optional 外层混同。

`enum.values` 为 0..64 个完整 scalar D7 TypedLiteral；空集合合法但不匹配任何自动操作。每项 type 必须与 `memberType` byte-equal；数组按完整 D3-CJ/3 canonical UTF-8 bytes 严格升序且唯一。membership 使用 D7 Value §2 原同型 equality：exact text 按 scalar、numeric 精确数学值、semantic_code 按完整 resolved scope + code；禁止数值 widening 或 text/code coercion。

`numeric_range` 只用于 int64/integer/decimal，minimum/maximum 的完整 type 必须等于 `memberType` 并按原 D7 同型 order 满足 `minimum <= maximum`。`text_utf8_no_crlf` 只用于 exact `{kind:"text"}`；`maximumUtf8Bytes` 为 0..65528，直接受 S D4 raw Entry 最大 UTF-8 bytes 约束。候选 Unicode scalar text 编成 UTF-8 后不得超过该值，且不得包含 U+000D/U+000A；完整 proposed D4 Entry/source 仍单独经过 D4/D2 framing、escaping、nonEmpty、schema、cardinality 和 source budget。

constraint 只收窄当前 D4/D7 值域；enum 命中不替代 Registry contribution availability、Narrow Field Qualification、current D6 authorization、`source_envelope_state` 或 commit 时另行必须具备的 `commit_sequence_state`。

实际效果仍只有两支。member-change 要求原 D7 adapter 只产生一个 existing scalar-member MutationFootprint 与完整 owner_fields field_change，其它 source/member/qualifier/note/provenance/Entry/body/Facet/Ref/relation/control 均不变。raw-no-op 还要求同 current owner/Field/唯一 Entry/member、按上述 required/present-optional 投影后的 D7 同型 equality，以及原 adapter 的完整 proposed source 与 before bytes 逐字相同；MutationFootprint、field_change、D6 sourceVersions 都为空。仅 typed equality 不能授权 byte rewrite。

Standing Approval 配置自身 malformed 时返回管理域 `D10ControlError.invalid_request`。已经合法的 D7 Action 若 scalar 值不在冻结 constraint 内，只表示 Standing Approval 不覆盖它，走 D6 前 Run/step `approval_required`；过期为 `approval_expired`。进入正式 D6 后由 D6 按 coordinated amendment 独立拥有错误。

semantic_code 规范正例直接使用 S 当前目录：`people/phone` 的 `people/labeled-text-value.label` 是 optional contribution-set semantic_code。只有当前 phone Entry 的 `label` 已 present 时，D7 Action value 才是 Optional<semantic_code>.some；`SingleFieldMemberRule.memberType` 是底层 semantic_code scope。enum 可只允许 `people/personal` 与 `people/work`：present `personal→work` 是真实 member-change，`work→work` 是 raw-no-op；即便 D4 允许 `people/other`，未列入 approval enum 时也不得自动批准。

R08 把 immutable external request semantics、具体 send attempt 与可变 external-effect lifecycle 明确分开：

    FrozenEffectBytes/1 = {
      bytes:Bytes,
      byteLength:Counter,
      digest:Sha256
    }

    ExternalTarget/1 = {
      operation:LocalOperationId,
      target:ToolValue/1
    }

    ExternalIdempotencyBinding/1 =
        {kind:"none"}
      | {kind:"bounded_key",
         key:Text,
         notBefore:D4.zoned_instant,
         notAfter:D4.zoned_instant,
         proof:FrozenEffectBytes/1}

    ExternalEffectIntent/1 = {
      effect:ControlRef<external_effect>/1,
      workspaceRef:D3.WorkspaceRef,
      contributionBinding:ContributionBinding/1,
      accountBinding:Binding<external_account>/1,
      targetBinding:ExternalTarget/1,
      requestPayload:FrozenEffectBytes/1,
      idempotencyBinding:ExternalIdempotencyBinding/1
    }

    ExternalRequestBinding/1 = {
      effect:ControlRef<external_effect>/1,
      requestDigest:Sha256
    }

    ExternalExecutionBinding/1 = {
      sendAttemptId:Uuid,
      intent:ExternalRequestBinding/1,
      delegationBinding:Binding<lease>/1,
      approvalBinding:Binding<external_approval>/1,
      externalEffectGrant:Binding<grant>/1,
      egressBinding:Binding<grant>/1,
      secretGeneration:Option<{
        secret:Binding<secret>/1,
        secretVersionId:Token,
        grant:Binding<grant>/1
      }>,
      budgetReservations:[{
        billableAttemptId:Uuid,
        reservation:ControlRef<reservation>/1
      }]
    }

    ExternalEffectCurrentView/1 = {
      kind:"external_effect_state",
      state:"prepared" | "submitting" | "succeeded" |
            "failed_no_effect" | "outcome_unknown" | "cancelled",
      recoveryMode:"automatic" | "manual_required",
      contribution:ContributionBinding/1,
      account:Binding<external_account>/1,
      operation:LocalOperationId,
      requestDigest:Sha256,
      targetDigest:Sha256
    }

`FrozenEffectBytes.byteLength` 必须等于真实 bytes 长度，digest 等于其 SHA-256。每份记录都必须落入当前 Run/Automation/Lease input 与 egress 限额的最窄有限预算。`bounded_key.key` 是非空、最多 1024 UTF-8 bytes 的文本，`notBefore < notAfter`；proof 是 accepted adapter 对同 contribution/account/operation/target/key/window 的完整 immutable 证据。该 proof 是内部 bytes，**不是** EvidenceTicket 的新 arm。

`requestDigest` 是完整 frozen ExternalEffectIntent 经 D10-External-Intent/1 域 canonical 编码的 SHA-256；`targetDigest` 是完整 ExternalTarget 经 D10-External-Target/1 域的摘要。Core 保留完整原 bytes。`budgetReservations` 为 0..32，按 reservation ref 排序唯一；每项解析到 `attemptId==billableAttemptId` 的 CostReservation，之后结算推进 reservation revision 不会改写 immutable send binding。`sendAttemptId` 与每个 billableAttemptId 分域。

普通 current read 只返回 `ExternalEffectCurrentView/1`；绝不返回 request payload、target ToolValue、idempotency key/proof、secret generation、approval record 或 reservation identities。即便只是 digest/account/contribution，也必须先通过原冻结 effect scope 的当前披露授权。

    ConsentSpec/1 =
      {kind:"planned",
       originalRequest:D6.d6_commit_request,
       previewSemanticDigest:Sha256,
       lease:Binding<lease>/1,
       activationBinding:ActivationBinding/1,
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}
    | {kind:"external",
       intent:ControlRef<external_effect>/1,
       requestDigest:Sha256,
       resourceGrants:[Binding<grant>/1],
       notBefore:D4.zoned_instant,
       notAfter:D4.zoned_instant}

planned 只授权原 planned request，不 reprepare、不换 OperationId、不换 target；external 只授权 exact ControlRef 与 requestDigest 都匹配的 immutable ExternalEffectIntent。合法 `prepared→submitting→...` lifecycle revision 本身不改变 frozen request digest，因此不会自行使 consent 失效；改变 contribution/account/target/payload/idempotency 必须新建 effect intent 并重新 consent。

prepare 成功返回：

    D10ControlPrepared/1 = {
      kind:"d10_control_prepared",
      wireVersion:1,
      requestId:Uuid,
      prepareToken:Token,
      preview:ControlPreview/1,
      commit:
        {kind:"workspace", request:D6.d6_commit_request}
      | {kind:"deployment",
         request:{kind:"d10_host_control_commit",
                  wireVersion:1,
                  scope:Scope/1,
                  requestId:Uuid,
                  prepareToken:Token}}
    }

    ControlPreview/1 = {
      kind:"d10_control_preview",
      wireVersion:1,
      canonicalIntentBytes:Bytes,
      affected:[{
        ref:ControlRef<K>/1,
        change:"create" | "update" | "enable" | "disable" |
               "revoke" | "cancel" | "archive" | "retire" |
               "disconnect" | "close" | "settle",
        beforeRevision:Option<Counter>,
        proposedAfterRevision:Option<Counter>
      }],
      resourceUses:[{
        grant:Binding<grant>/1,
        maximum:Option<Money/1>
      }],
      externalRequest:Option<ExternalConsentPreview/1>
    }

affected 按 ref canonical bytes 排序且唯一；resourceUses 按 grant ref 排序且唯一。canonicalIntentBytes 是本次已成功 closed-decode 的完整 D10-Control-Intent/1 bytes，因此包含 caller 已提供并获权查看的拟议控制语义；preview 不另带 secret bytes、隐藏作者值或其它用户账务。

external consent 通过既有 prepare 响应获得受权的完整请求查阅路径。closed body 为 consent 且 consent 为 external 时，`externalRequest` 恰为 some；其它 body 一律为 none。Core 先验证当前主体对完整请求范围、contribution、account、target 和 payload 的权限，再由受保护最小映射定位准确 frozen intent；仅可见摘要或持有宽泛资源 grant 不足以读取全文。缺失、隐藏、错误 audience 在泄露 intent 存在前统一为 `not_visible`。Core 将完整 frozen intent 与提交的 effect Ref、request digest 比较，验证 payload 的实际长度/摘要，并把真实 bytes pin 到原 preparation。不匹配为 `control_conflict`，受保护 bytes 已证明矛盾为 `integrity_conflict`，pins/连续性暂不可证为 `state_unavailable`。真实 lifecycle transition 不改变 immutable intent；不另造 effect current-read 入口。

    ExternalConsentPreview/1 = {
      intent:ExternalRequestBinding/1,
      workspaceRef:D3.WorkspaceRef,
      contributionBinding:ContributionBinding/1,
      accountBinding:Binding<external_account>/1,
      targetBinding:ExternalTarget/1,
      requestPayload:FrozenEffectBytes/1,
      idempotency:
          {kind:"none"}
        | {kind:"bounded_key", keyDigest:Sha256,
           notBefore:D4.zoned_instant, notAfter:D4.zoned_instant,
           proofDigest:Sha256}
    }

    ExternalConsentConfirmation/1 = {
      key:StableControlKey/1,
      intent:ExternalRequestBinding/1,
      previewDigest:Sha256,
      principal:Token,
      clockEpoch:Token,
      confirmedAt:D4.zoned_instant
    }

preview 从 frozen intent 复制准确 target ToolValue、operation、完整 payload bytes、实际 account 和 contribution，不从模型描述或替代 preview 参数接收这些值。bounded-key 摘要披露已接纳的有效区间，以及 canonical key 和完整 proof 分别在 D10-External-Key/1、D10-External-Idempotency-Proof/1 域下的 SHA-256 摘要；不披露可复用 key、proof bytes 或凭据。包括这些受保护 bytes 在内的完整 intent 仍由原 request digest 固定。Secret 认证字节只在批准后通过既有可信认证通道注入，不得改变获批业务 target 或 payload。需要在业务 bytes 内替换隐藏凭据的载荷不属于此 profile。

完整 canonical ControlPreview 不超过 16777216 bytes，并受更严格的入口/transport 预算约束；完整 external payload 不超过 8388608 bytes。任一超限为 `budget_exceeded`，不交付截断查阅或可确认的部分响应。这是一次完整交付，不新增分页协议。已接纳可信 UI 或有人操作的 CLI 验证全部 frozen byte length/digest 和完整 canonical preview，以不执行内容的准确表示展示所有 target 成员和 payload，并在允许确认前提供完整表示。文本控制字符转义，二进制提供无损字节视图。Adapter 摘要、模型文案、折叠前缀或单独摘要不能满足完整展示；界面无法完整展示该请求时不能确认。这只证明提供并明确确认了哪份完整请求，不声称人已逐字理解。

只有当前认证用户在该可信确认界面的本次明确操作，才能建立内部 `ExternalConsentConfirmation/1`；被委托 Agent/tool/worker/connector 不能创建它、调用可信事件通道或用 JSON 标记替代。可信界面提交绑定其当前完整展示的已认证用户事件，Core 验证当前 audience、D10-Control-Preview/1 域下的完整 preview digest、准确原 stable key/intent、可信时间、consent interval 及当前 dependencies，再在独立受保护 ExternalConfirmationRecord/1 中原子记下事实。该事件是已接纳界面的内部确认操作，不是公共 control body、新作者请求或第二成功账本。没有此人工确认事件的普通认证脚本或被委托执行 session 不能制造事实；平台接纳必须证明被批准的 executable contribution 无法调用这个事件通道。

不可变 ControlPrepareBinding/2 保存 ExternalConfirmationRequirement/1，§8 独立保护的 ExternalConfirmationRecord/1 保存当前已确认 fact；确认绝不修改绑定输入。fact 的 principal 必须等于 stable key 中真实 Workspace 主体，不能采用调用方自报 grantor。新 target/payload/intent、不同 preview/principal 或变化的依赖不能继承确认。调用方在中断后可按原受信事件规则重新展示仍有效的同一 preparation。历史 result/current 投影继续脱敏，不返回完整 review 或该保护 fact。

对尚未决议的 external-consent commit，D10 adapter 先执行当前可见性/authority 和 stable-key/saved-result 分流，再要求此准确且当前合格的确认，才转交原 Workspace 请求。准备可见且合格但没有确认事件时，界面继续等待可信用户操作；这是展示状态，不是新增公共响应或 Run 错误。Prepare 仍返回原完整 prepared 响应。不虚构 D10 Workspace-commit 入口：唯一公共 Workspace submit 是原 D6 request，正式结果归下述 D6 拒绝规则。可信确认/时间/连续性不可证明时不能确认，在适用 D10 读取/准备中仍为 `state_unavailable`。Core 把保存的确认及完整原 preview 关联到原 D6 control plan，作为受保护准入依赖；planning CAS 和 final commit 都重验该依赖、完整当前请求披露/操作权限、consent 时间、未变 intent 及原依赖。直接提交返回的 D6 request 不能绕过。进入 D6 后，原 permission/business 错误保持优先；其余资格成立但确认缺失或不可用时返回协调的 D6 `approval_unavailable/preflight`，保留既有 planned 的恢复责任，不创建替代批准或决议。已 applied consent 先按当前结果权限重放，再考虑任何新确认门；不重新要求确认或改判失败。实际 D6 producer 和已接纳可信界面支持此关联之后，该路径才可用。



查询：

    D10ControlResultRequest/1 = {
      kind:"d10_control_result",
      wireVersion:1,
      scope:Scope/1,
      requestId:Uuid
    }

历史 prepare/apply 与 current exact-record read 必须分成两个入口。

    D10ControlResult/1 =
        {kind:"d10_control_result_prepared", wireVersion:1,
         scope:Scope/1, requestId:Uuid, prepared:ControlPreparedHistory/1}
      | {kind:"d10_control_result_applied", wireVersion:1,
         scope:Scope/1, requestId:Uuid,
         prepared:ControlPreparedHistory/1, applied:ControlAppliedHistory/1}

    ControlOperationKind/1 =
        "automation_configure" | "consent" | "state" |
        "workspace_limits" | "activation" |
        "deployment_put" | "cost_reconcile"

    ControlAffectedChange/1 = {
      ref:ControlRef<K>/1,
      change:"create" | "update" | "enable" | "disable" |
             "revoke" | "cancel" | "archive" | "retire" |
             "disconnect" | "close" | "settle",
      beforeRevision:Option<Counter>,
      proposedAfterRevision:Option<Counter>
    }

    ControlResourceUse/1 = {
      grant:Binding<grant>/1,
      maximum:Option<Money/1>
    }

    ControlPreviewSummary/1 = {
      affected:[ControlAffectedChange/1],
      resourceUses:[ControlResourceUse/1]
    }

    ControlPreparedHistory/1 = {
      intentDigest:Sha256,
      operation:ControlOperationKind/1,
      allocatedControlRefs:[ControlRef<K>/1],
      previewSummary:ControlPreviewSummary/1,
      commitOwner:"D6" | "D10"
    }

受保护的 `ControlPrepareBinding/2.canonicalIntentBytes` 继续保存完整 canonical control intent B，包括其中嵌套的 planned 作者请求 A；stable-key conflict 仍比较完整 bytes。`intentDigest` 只是这些 saved bytes 的 SHA-256。历史 public result 永不返回 A、完整 B、生成的 control-submit request M、prepareToken 或 planned-preview bytes/token。`operation` 恰取现有七种 ControlBody kind；`ControlAffectedChange/1` 与 `ControlResourceUse/1` 就是现有 ControlPreview item shape，成员、enum、排序、唯一性、Option 与 Money 语义不变。

首次 `d10_control_prepare` 只有在当前披露资格覆盖完整 B 与任何嵌套 A scope 后，才可返回包含 M 的完整 `D10ControlPrepared/1`；之前持有 A 不等于当前读取资格。同 stable key 已有权威 applied success 时，prepare 返回同一个 `d10_control_result_applied` historical arm，绝不伪装新的 prepared object。若 applied-success existence 或 linkage 暂不可证明，则返回 `state_unavailable`，绝不降格 prepared。

    ControlAppliedHistory/1 =
        {kind:"workspace",
         ownerReceipt:D6.d6_commit_receipt,
         changes:[ControlRevisionDelta/1],
         usageChanges:[ControlUsageRevisionDelta/1]}
      | {kind:"deployment",
         changes:[ControlRevisionDelta/1],
         usageChanges:[ControlUsageRevisionDelta/1],
         evidence:[EvidenceTicket/1],
         auditRef:Token}

    ControlRevisionDelta/1 = {
      ref:ControlRef<K>/1,
      beforeRevision:Option<Counter>,
      afterRevision:Counter
    }

    ControlUsageRevisionDelta/1 = {
      ref:ControlRef<lease|approval|grant|cost_account>/1,
      beforeUsageRevision:Counter,
      afterUsageRevision:Counter
    }

Workspace applied history 只能从同一权威 D6 saved decision 投影：`ownerReceipt` 是原不可变 receipt bytes，`changes/usageChanges` 只来自与该 decision 同事务关联的 D10 control effects。Deployment arm 只从原 `DeploymentControlDecision/1` 加 §8 同事务关联的 record/account deltas 投影。两者都不是第二成功账本。

`d10_host_control_commit` 首次原子成功及准确成功重放的直接公开响应，都只能是已有 `D10ControlResult/1` 的 applied 分支：`{kind:"d10_control_result_applied",wireVersion:1,scope:<original deployment scope>,requestId:<original requestId>,prepared:<original ControlPreparedHistory/1>,applied:<original ControlAppliedHistory/1 deployment arm>}`。Core 从同一个保存的 decision 及其关联 preparation/effects 构造此响应；不得返回内部 `DeploymentControlDecision/1`、空确认、新 prepared object 或第二份 receipt。真实 no-op 仍返回此 applied 分支，其中配置 `changes` 为空、`usageChanges` 为实际保存的增量。之后的 r6 配置不能替换任何原 r5 字段。交付前 Core 重新验证当前主体对完整公开结果的披露资格：资格丧失返回 `D10ControlError.not_visible`；保存成功的关联暂不可证明返回 `state_unavailable`，确定矛盾返回 `integrity_conflict`。这些交付失败不改变已经提交的 decision 和效果；重试/结果查询按原有门恢复同一 applied history，不重新执行操作。

result 顺序固定为：当前 result-disclosure 授权 → authority/custody/continuity → stable key。missing/hidden 为 `not_visible`。能证明 prepare binding 且能证明没有 applied success 时才返回 prepared；权威 applied success 存在时返回 applied。D6 recorded rejection/terminal failure 仍归 D6，调用原 D6 request replay 取得；D10 result 只有在证明没有 applied success 后才可返回 prepared history。若 applied success 的存在/缺失或 linkage 暂不可证明，必须 `state_unavailable`，不能降格为 prepared；确定 decision/effect linkage 矛盾为 `integrity_conflict`。

若 r5 已 applied、响应丢失，另一个合法请求后来把同对象更新到 r6，原 request 重试在当前披露授权通过后仍返回保存的 r5 applied history，绝不能临时替换成 r6。

current state 使用另一个入口：

    D10ControlReadRequest/1 = {
      kind:"d10_control_read", wireVersion:1,
      scope:Scope/1,
      ref:ControlRef<K>/1
    }

    D10ControlCurrent/1 = {
      kind:"d10_control_current", wireVersion:1,
      scope:Scope/1,
      binding:Binding<K>/1,
      usageRevision:Option<Counter>,
      view:ControlCurrentView<K>/1
    }

非 Stop 的 K 永远严格要求 `scope == record.scope`；没有 wildcard、list、name 或 display lookup。**仅 Stop 的双资格 relation：**严格解码、D1 `automation.stop` 后先确认**当前真实认证** W 用户或 H operator，完整 target/StopOwner/latch 及嵌套 Run/Automation 披露权**先于** existence。W 请求 scope 必须为 target 真实 `{kind:"workspace",workspaceRef}`；H 必须为 `{kind:"deployment",storeIncarnation:<actual-store-incarnation>}`，且当前 DeploymentControlPolicy 授予针对准确 target **及其 Workspace** 的 stop/read 权，不是 H 连接即可。通过真实 authority/fence/custody 后，以**唯一一份**受保护 `StopOwner/1` 核实实际 target 完整 Ref、真实 Workspace、唯一 stop latch Ref、target/latch/owner/storeIncarnation 全等、`requestId==target.id`，以及持久真实 scope 为 **Workspace** 的唯一 Stop Image1。H scope 仅是读取资格，绝非第二份持久 Stop 身份/image。`D10ControlCurrent.scope` 投影已验证 W/H 请求，但 `binding`、`usageRevision=none`、`stop_state`、状态及 revision 全来自**同一读取 cut 的唯一受保护 Stop record**。`d10_emergency_stop_result` 复用完全一致 target/StopOwner/latch relation 与原存 receipt/not-applied cut；target r5→r6 不修改它。隐藏/不存在、跨 Workspace、错 store、嵌套无授权 → `not_visible`；authority/fence/custody 暂不可证 → `authority_unavailable`；已证 owner/store/target 矛盾 → `integrity_conflict`；latch/result 连续性暂失 → `state_unavailable`，保持原优先级。即使存在成功历史，结果交付前仍重查当前披露。绝不能为 H 复制 Stop fact、audit、安全容量、revision 或结果。非 Stop 的 visible record 连续性未知仍为 `state_unavailable`，兼容 state/member 未知是 integrity/version 失败，不能猜测状态。

首版 19 类 current projection 闭集如下：

    RunOrigin/1 =
        {kind:"interactive"}
      | {kind:"automation",
         automation:Binding<automation>/1,
         definitionRevision:Counter,
         occurrenceKey:AutomationOccurrenceKey/1}

`RunOrigin/1` 是 Core 创建且不可变的来源事实，由受保护 Run 记录、`LeaseRunUse/1.origin` 和公共 `run_state.origin` 投影共同使用。interactive 分支仅有 `kind`，不要求 Automation、definition revision、occurrence claim，也不填占位值或 null。automation 分支绑定实际创建此 Run 的 Automation、不可变 definition 和完整 occurrence claim；key 内的完整 Automation Ref 必须等于 origin 的 automation.ref，原 claim 的 definitionRevision 必须等于 origin 的 definitionRevision；key 自身不包含 definitionRevision。调用方或模型不能选择或更换既有 Run 的 origin。当前 control revision 可以改变，但不能改写历史 origin binding；新的执行仍须通过当前门禁。Run 不能切换分支以逃离已有 occurrence claim 或预算谱系。

交互 Run 的 Lease target 必须准确指向该 Run 的完整 ControlRef；Automation Run 则准确指向 origin 的 Automation ControlRef。两支同样受真实认证主体、Workspace、activation、准确已准入 Lease revision、有限上限、stop latch 及全部既有准入/恢复规则约束。只有 automation 分支读写 occurrence claim。受保护 Run、准入记录及实际 claim/Lease 之间存在已证明矛盾时，control read 返回 `integrity_conflict`，Run 执行前返回 `control_conflict`；连续性不可证明为 `state_unavailable`。Run 本身可见但嵌套 origin/Lease/stop binding 不可披露时，整体返回 `not_visible`；隐藏 Automation 不能投影成 interactive。此处修订尚未激活的候选类型，不授权实际历史记录的 decoder 猜测 origin。

### 7.1 可信 interactive-start 生产者（并非第八个 ControlBody）

首代**新 interactive Run 及其 Run-targeted Lease**只由以下有限 Core runtime 入口生产，另一路为既有 Automation occurrence 生产者。本路径**不是** `automation_configure`、第八个 `ControlBody`、公开作者请求、泛 callback 或 Agent 自报的 `approved` 布尔值。D1 `automation.manage` 必须真正通过 release/contract-major/surface/version/health 门；当前 D6 Policy/3 还须独立允许实际认证用户的 workspace `d10_control_self`。Field 权限、issuer、policy_admin、工具描述和 H 登录均不蕴含二者。

```text
D10InteractiveRunStartRequest/1 = {
  kind:"d10_interactive_run_start",wireVersion:1,
  requestId:Uuid,
  scope:{kind:"workspace",workspaceRef:D3.WorkspaceRef},
  activationBinding:ActivationBinding/1,
  delegation:LeaseSpec/1
}

D10InteractiveRunResultRequest/1 = {
  kind:"d10_interactive_run_result",wireVersion:1,
  requestId:Uuid,
  scope:{kind:"workspace",workspaceRef:D3.WorkspaceRef}
}

D10InteractiveRunStarted/1 = {
  kind:"d10_interactive_run_started",wireVersion:1,
  requestId:Uuid,
  scope:{kind:"workspace",workspaceRef:D3.WorkspaceRef},
  run:Binding<run>/1,
  lease:Binding<lease>/1,
  activationBinding:ActivationBinding/1,
  origin:{kind:"interactive"},
  admission:{kind:"not_admitted"}
}
```

调用方仅能选择首次发出前持久保存的 `requestId`；请求没有 `principal`、Run/Lease Ref、Automation、occurrence、target、authorRequest、`approved` 或创建授权 token。`delegation` 是**完整有限** LeaseSpec/1，不是任意描述：其 activationBinding 必须与外层完全一致；`notBefore < notAfter`、正且有限 maxRuns、D1 allowlist、准确 D6 LeaseReadGrant scope/Field set、H 已发的 ResourceUseGrant binding 和每个 BudgetCaps 均闭合解码。Core 逐一验证与真实当前 Policy/3 allow/deny、主体/Workspace/audience、active selector、实际 Contribution/H grant 授权、owner/account/currency、spent/held ceiling 和可信时钟的**交集/收窄**。无适用费用 grant 不意味着免费账务；Lease 或用户不创建部署 grant、读写、secret、egress、账户或 Field 权限。固定 Run budget 准确取已接受 LeaseSpec.budgets，不能靠后续修改其他预算层提高。

Core 只在有**内部可信到场用户启动事件**时接受首次请求。该事件由受信 Desktop、已认证 Server attended UI（WebUI 经 Server Broker）或真正到场 CLI adapter 产生，绑定 D6 实际认证自然人/会话、准确 Workspace、**完整 canonical start 请求摘要**、可信 clock epoch、完整 Lease/activation 展示和新鲜显式 Start 动作。Core 必须校验事件和真实请求字节/主体/audience/时间。事件不是公开 JSON、caller 布尔值、Agent bearer token、模型/tool callback、无人值守 CLI flag 或脚本可访问的通道。Mobile/不受信 worker 不得伪造。Adapter 不提供或修改 principal；Core 从 D1/D6 既有认证边界取得真实主体。不能完整展示请求和证明真实本人动作的 surface 不提供此路径。

**首次启动与结果唯一顺序：**(1) 严格闭合 decode/界限和实际 D1 可用性；(2) 认证用户、到场事件、准确 W scope、Policy/3 self capability、全部建议权限与嵌套资源披露，均须**先于**受保护 existence lookup；hidden/wrong Workspace/principal/scope → `not_visible`，授权解码后畸形或缺少有效到场事件 → `invalid_request`；(3) D6 真实 custody/fence/authority、**同一**实际打开的 control-store incarnation 与可信 clock 连续性（分别 `authority_unavailable`、`state_unavailable`）；(4) stable key 使用实际 scope incarnation、认证主体和 requestId，并比较**完整 D10-Interactive-Start/1 canonical 请求字节**，同 key 异 intent 为 `control_conflict`，不得只比摘要；(5) 已证明 applied 则经当前披露门重放原 Started 结果，不因当前版本变化拒绝；应用/未应用关联未知为 `state_unavailable`，已证矛盾为 `integrity_conflict`；(6) 仅证明 unseen 时验证 active selector、完整 Policy/Lease/grants/clock `notBefore <= now < notAfter`、预算、stop 及完整负向 record/range/capacity proof（已变更 binding 为 `control_conflict`，容量/溢出 `budget_exceeded`，连续性未知 `state_unavailable`）；(7) 在**唯一真实 Authority Store**控制 CAS/写序列化中重检 before/range/授权/stop/容量并原子保存；(8) 发送前重过完整结果披露门。start/result 使用原 D10ControlError/1；开始受保护执行后按 D10RunStepError/1 或下游 D3/D6/D7/D8/D9 owner 原错误返回。

第一次获胜 CAS 由 Core 在真实 storeIncarnation 分配恰一组新鲜不复用完整 `ControlRef<run>`、`ControlRef<lease>`、`ControlRef<stop>`，均属于真实 Workspace scope。Run/Lease 配置 Binding 初始 revision1。Lease Image1 为 active，principal 来自**受信 D6**，target 是新 Run 完整 Ref，spec 为已验证 LeaseSpec，`runsConsumed=0`、`usageRevision=some(0)`。Run Image2 包含 `run_state {lifecycle:"active",executionState:"queued",origin:{kind:"interactive"},lease:<Lease Binding>,admission:{kind:"not_admitted"},stop:<Stop Binding>}`，`usageRevision=none`；其受保护 supplement 为 `createdBinding=<originalRunRevision1>,principal=<authenticatedPrincipal>,fixedBudget=<LeaseSpec.budgets>,invocation:none,admission:none,authorSteps:[]`。另建**唯一** W-scope Image1 `stop_state`、revision1/open latch 与准确关联 Run、Workspace、storeIncarnation 的 `StopOwner/1`。在 Run 可管理前必须预留 §11 唯一 latch、不可变结果/audit slot 与 safety sequence 单位；StopCapacity 不依赖普通管理配额。**同一**既有 control-store 事务原子写入原 key/完整字节、不可变 start 结果、三份 record/关联、原 owner/range fence/pins、预留安全容量、保留及 audit。并发 start/撤权/激活/stop/容量由**同一** store 序列化；失败/崩溃/输家不得留下部分 Run/Lease/Stop、已应用假象、丢失的 safety slot 或可使用 Run。这不是 D6 OperationId、作者 DecisionKey/P、第八个 ControlBody、Automation/occurrence claim、Standing Approval 或第二决议账本。

创建**不消耗 maxRuns**。首个受保护 step 前，原 §8.2 `LeaseRunUse/1` CAS 检查 Lease target 等于**完整 Run Ref**、不变的 interactive origin、真实主体/Workspace/activation、未 stop、可信时钟、当前 grant/Policy 与全部有限预算；仅此 CAS 将累计 lease use 加**一次**并保存准确 admitted lease revision/时间。`maxRuns=1` 的同一已准入 Run 可执行第二受保护 step，或崩溃/失答后恢复**原** planned D6 请求，不再消耗，也不捏造 Automation claim。后续每步仍复核当前授权、准确已准入 Lease revision、时间、activation、stop、approval、预算；过期/撤权/stop/连续性未知禁止新执行，不撤销已提交工作。第二个 Run 不得使用这份 Run-targeted Lease。执行失败/取消/重启不退第一次用量。saved/planned/unknown 作者工作必须按**原保存**责任臂恢复：自动臂用真实 Link2/PAB4/ApprovalUse2；交互臂用原 D3/D6 request、真实 D7 PAB4 或 D8 EditBinding3、准确 preparedFormat/preparedRecordPin/recoveryPins 及必要的重新全量 preview 人工明确确认。不重新创建 OperationId、DecisionKey、prepared binding、approval 或作者 P。

`d10_interactive_run_result` 只有在原 principal/scope、authority/fence/custody 与完整嵌套 Run/Lease/Stop 结果披露通过时才返回原存的准确 `D10InteractiveRunStarted/1` 或原 control error。证明 unseen/hidden → `not_visible`；原结果或三份关联任何一处暂不可证 → `state_unavailable`，不新建 Run。初次响应丢失、同请求并发或之后 Run/Lease r5→r6 均恢复不可变原 revision1 结果；同 key 异 intent → `control_conflict`。错误 store/scope 不能别名为本地 Ref；交付前必须重新过披露门。

受保护的 stable `StableControlKey/1` 比对 `D10-Interactive-Start/1` 的完整请求字节；Lease `target` 必须等于 Run 的完整 Ref。生产者创建不计入 `maxRuns`，首次原准入 CAS 才计入 `maxRuns`。

**产品尚未执行的验收义务：**认证到场用户无 Automation 获 Run 与 Run-targeted `maxRuns=1` Lease，首次准入仅一次；真实 D3 identity 与 D6 author（含 D8 EditBinding3）必须经过 D7 全量 preview/明确确认，同 Run 第二步及剩余量零时原 planned 冷恢复不重扣。不构造 interactive Link2/ApprovalUse2；真实 Automation core_field_member 仍需二者。反例：伪造 principal/事件、未授权 D6 Field/read/resource、扩权账户/预算、同 key 异字节、错 store、缺 owner pins、StopCapacity 耗尽/stop 竞争、撤销/过期 lease/activation、clock epoch 丢失和费用溢出。必须按原错误域拒绝并证明零半态、无重复 lease use、无 D6 作者决议副作用。

### 7. 十九种可读的封闭 current projection

以下每类完整保留原四列：kind、精确 scope、完整 view、config/domain/usage 语义；仅修 Markdown 显示，不修改 decoder。

#### 1. automation

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"automation_state",state:"enabled"|"disabled"|"archived",definitionRevision:Counter,subscriptionGeneration:Counter,lowerOriginalStartUtcSeconds:CanonicalDecimal,definition:AutomationSpec/1,lease:Binding<lease>/1,approval:Option<Binding<approval>/1>}
```

**配置/域/用量：** `binding.revision` 是 control/lifecycle CAS；`definitionRevision` 只随 semantic definition 改变；usageRevision=none。

#### 2. lease

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"lease_state",state:"active"|"revoked"|"archived",principal:Token,target:ControlRef<automation|run>/1,spec:LeaseSpec/1,runsConsumed:Counter}
```

**配置/域/用量：** `binding.revision==leaseRevision`；用量修订只在新的 `LeaseRunUse/1` 消耗谱系时推进；普通用量变化不会使已经准入的运行因租约修订变化而失效。

#### 3. approval

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"approval_state",state:"active"|"revoked"|"archived",grantingPrincipal:Token,automation:Binding<automation>/1,definitionRevision:Counter,lease:Binding<lease>/1,activationBinding:ActivationBinding/1,spec:StandingApprovalSpec/1,reserved:Counter,consumed:Counter,releasedTerminal:Counter}
```

**配置/域/用量：** `binding.revision==approvalRevision`；用量修订只随批准使用的预留、消费或终态释放变化；撤销或归档推进批准配置修订，不清除累计用量。这里的配置修订只描述批准规则和生命周期，累计使用量始终由独立用量修订追踪，二者不会相互重置、替代或隐式恢复权限。

#### 4. planned_approval

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"planned_approval_state",grantingPrincipal:Token,originalRequestDigest:Sha256,previewSemanticDigest:Sha256,lease:Binding<lease>/1,activationBinding:ActivationBinding/1,notBefore:D4.zoned_instant,notAfter:D4.zoned_instant}
```

**配置/域/用量：** binding revision 就是 approvalRevision；usageRevision=none；不在此泄露 author request/preview bytes。

#### 5. external_approval

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"external_approval_state",grantingPrincipal:Token,intent:ControlRef<external_effect>/1,requestDigest:Sha256,resourceGrants:[Binding<grant>/1],notBefore:D4.zoned_instant,notAfter:D4.zoned_instant}
```

**配置/域/用量：** binding revision 就是 approvalRevision；usageRevision=none。

#### 6. run

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"run_state",lifecycle:"active"|"archived",executionState:"queued"|"running"|"awaiting_confirmation"|"blocked"|"cancelling"|"reconciling"|"completed"|"failed"|"cancelled",origin:RunOrigin/1,lease:Binding<lease>/1,admission:{kind:"not_admitted"}|{kind:"admitted",leaseId:Uuid,leaseRevision:Counter,admittedAt:D4.zoned_instant},stop:Binding<stop>/1}
```

**配置/域/用量：** durable Run/lifecycle transition 推进 binding revision；usageRevision=none，maxRuns 消耗归 Lease usage。

#### 7. workspace_budget

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"workspace_budget_state",limits:BudgetCaps/1}
```

**配置/域/用量：** 绑定修订只随限额配置变化；没有独立累计用量修订，实际费用和预留仍由授权、账户及费用预留记录持有，不建立第二预算账本。

#### 8. activation

**精确 scope：** `workspace`

**完整 view：**

```text
{kind:"activation_state",current:Boolean,activation:ActivationBinding/1,packages:[ContributionBinding/1],trust:Binding<trust>/1}
```

**配置/域/用量：** `binding.revision==activation.activationGeneration`；successor activation 使旧 record `current:false`，不删除；usageRevision=none。

#### 9. deployment_policy

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"deployment_policy_state",policy:DeploymentControlPolicy/1}
```

**配置/域/用量：** `binding.revision==policy.revision`；无独立累计用量修订。该记录只描述部署管理策略本身的配置变化，管理员和协调者列表变化都属于同一配置修订，不另建立累计使用计数。

#### 10. trust

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"trust_state",state:"active"|"revoked",publisherId:Text,publicKey:Ed25519PublicKey,previous:Option<Binding<trust>/1>,proof:EvidenceTicket/1,claims:[NamespaceClaim/1]}
```

**配置/域/用量：** key/claim/rotation/revoke 推进 binding revision；usageRevision=none。

#### 11. package

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"package_state",state:"installed"|"retired",manifest:PackageManifest/1,signature:Ed25519Signature}
```

**配置/域/用量：** 安装、更新或退役推进绑定修订；包版本域独立；无独立累计用量修订。包版本只标识不可变包内容的版本，不充当控制记录修订，也不会因为激活次数或运行次数而产生另一套使用计数。

#### 12. external_account

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"external_account_state",state:"connected"|"disconnected",provider:ContributionBinding/1,externalAccountId:Text,endpointId:LocalOperationId,proof:EvidenceTicket/1}
```

**配置/域/用量：** config/disconnect 推进 binding revision；usageRevision=none。

#### 13. secret

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"secret_state",state:"active"|"revoked",account:Binding<external_account>/1,audience:ContributionBinding/1,usageKind:"authenticate",secretVersionId:Token}
```

**配置/域/用量：** 发布、轮换、重新绑定或撤销都会推进绑定修订；绝不返回明文或暂存凭据字节；没有独立累计用量修订，使用次数仍归资源使用授权。

#### 14. grant

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"grant_state",grant:ResourceUseGrant/1}
```

**配置/域/用量：** `binding.ref.id==grant.grantId`、`binding.revision==grant.grantRevision`；存在独立用量修订且其值等于 `grant.usageRevision`；归档动作映射到已有 `retired` 状态。配置修订负责授权内容和生命周期，用量修订负责已发生的消费；退役不会清空已发生用量、未结预留或实际账户责任。

#### 15. cost_account

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"cost_account_state",state:"active"|"frozen"|"closed",currency:CurrencyCode,ceiling:Counter,pricing:Option<Binding<pricing>/1>,spentMicroUnits:Counter,heldMicroUnits:Counter}
```

**配置/域/用量：** 配置、关闭或冻结推进绑定修订；已支出和占用的费用变化推进独立用量修订；冻结不会清除既有责任。

#### 16. pricing

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"pricing_state",state:"active"|"retired",account:Binding<external_account>/1,currency:CurrencyCode,fixedMicroUnits:Counter,meters:[PricingMeter/1],evidence:EvidenceTicket/1}
```

**配置/域/用量：** pricing update/retire 推进 binding revision；usageRevision=none。

#### 17. reservation

**精确 scope：** `deployment`

**完整 view：**

```text
{kind:"reservation_state",reservation:CostReservation/1}
```

**配置/域/用量：** `binding.ref.id==reservation.reservationId` 且 `binding.revision==reservation.revision`；usageRevision=none；`uncertain` 是已有可恢复状态，不是 unknown。

#### 18. external_effect

**精确 scope：** `workspace`

**完整 view：**

```text
ExternalEffectCurrentView/1
```

**配置/域/用量：** 外部效果或恢复状态的耐久转换推进绑定修订；没有独立累计用量修订；费用和尝试次数分别仍归费用预留与资源授权记录。

**原 Stop helper 类型：**

```text
    ExecutionStopLatchView/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal/1>
    }
```

#### 19. stop

**精确 scope：** 经 W 读取时为 target 的准确 Workspace scope；经 H 读取时为准确 deployment storeIncarnation

**完整 view：**

```text
{kind:"stop_state",latch:ExecutionStopLatchView/1}
```

**配置/域/用量：** fresh open revision=1；只有 `open→stopped` 增一次；重复 stop 是 no-op。内部 safety sequence 不公开；无独立 usageRevision。


对 `lease|approval|grant|cost_account`，`D10ControlCurrent.usageRevision` 必须为 `some` 且等于当前独立 usage revision；其它 kind 必须为 `none`。可信时间跨 `notBefore/notAfter` 只改变当前 eligibility，不静默修改 configuration revision 或持久化 `expired` state。revoked/retired/archived/closed 对当前有权 reader 仍可观察，永不伪装成 absence。

`activation_state.current` 在同一受权 cut 中由准确 active-selector record 派生；切换时的 CAS 对象是 selector，而不是历史 ActivationBinding record。successor activation 比较准确 current predecessor selector，并把新 ActivationBinding 与 selector 原子发布。旧 generation 变为 non-current 不会改变该历史 generation 或其 Binding revision。

首代不提供 `planned_approval` 或 `external_approval` 的独立提前 revoke state action。有限时间边界、准确 Lease/Activation/request binding、当前 ResourceUseGrant 状态、当前 D6 authorization 与不可逆 stop gate 都在相关 planned/send 边界重新检查。这个支持范围限制不会创造 generic approval lifecycle state，也不会绕过原 authoritative-abort 规则释放 planned work。

## 8. Idempotency、CAS 与结果重放顺序

稳定 key 为 (scope incarnation, initiating principal, requestId)。requestId 必须在首次调用前由客户端保存并在 transport retry 中复用。

Core 内部保存：

    StableControlKey/1 = {
      scope:Scope/1,
      initiatingPrincipal:Token,
      requestId:Uuid
    }

    PreparedCommitRequest/1 =
      {kind:"workspace", request:D6.d6_commit_request}
    | {kind:"deployment", request:{
        kind:"d10_host_control_commit",
        wireVersion:1,
        scope:Scope/1,
        requestId:Uuid,
        prepareToken:Token
      }}

    D10WorkspaceReadDependencies/1 = {
      workspaceRef:D3.WorkspaceRef,
      commitDomain:D6.CommitDomain/2,
      observationScope:D6.ObservationScope/2,
      sourceInputs:[{
        entityRef:D3.EntityRef,
        observation:D6.SourceObservation/1,
        role:"before" | "dependency"
      }],
      dependencyProof:D6.DependencyProof/2,
      registryReads:[{
        binding:D4.RegistryBinding/1,
        snapshot:D4.RegistrySnapshot/1,
        snapshotPin:D6.PinRef/2
      }],
      evidencePins:[D6.PinRef/2]
    }

    ControlDependencies/2 = {
      configBindings:[Binding<K>/1],
      usageBindings:[{
        ref:ControlRef<lease|approval|grant|cost_account>/1,
        usageRevision:Counter
      }],
      authorityProof:Token,
      authorizationGenerations:[Token],
      stopRefs:[ControlRef<stop>/1],
      workspaceReads:Option<D10WorkspaceReadDependencies/1>,
      recordPins:[D10ControlRecordPin/1],
      controlRanges:[D10ControlRange/1]
    }

ControlDependencies/2 保存 Core 实际读取的完整依赖，不接受调用方断言或只有摘要的绑定。Workspace 控制的 workspaceReads 必为 some，纯 deployment 控制必为 none，deployment body 不能夹带作者读取。工作区字段等于本计划真实的 sourceInputs、observationScope 与 DependencyProof。最终 observationProof 只在外层 D6 PreparedIntent/2 中由这些准确输入和固定引用单向构造，不能嵌回 ownerInput；既不新增 ObservationProof/2 类型，也不产生自身循环。sourceInputs 沿用 D6 的排序、唯一性、role 及当前观察/pin 规则。每个 Registry 项保留实际完整 RegistrySnapshot/1 及精确 binding，使用 artifact PinRef/2 保留其规范字节，Core 验证原 D4 所有权、完整性和 schema 规则。没有 Registry 用途时数组为空，不能构造假 binding。绑定数组按完整 Ref、Registry 读取按完整 binding、pins 按 pinToken 规范排序且唯一；授权世代 token 保留原 owner 语义并规范排序去重。每个数组最多4096项，完整依赖元数据规范字节最多16777216；被固定的 source/payload bytes 另受原预算约束。超限返回 budget_exceeded，不能截断。

D6 producer 将真实 source/Registry/authorization/range 依赖放入各自实际 DependencyKey/2 kind/stamp，保留必要 controlInputs 对应与 pins。D10 configuration/usage/stop 依赖仍以完整闭合 owner 值置于同一 InputDescriptor 的规范 ownerInput 中，在同一 planning/seal 事务对实际保护记录逐项 CAS；摘要或无关 execution_resource key 不能代替。这里不增加第十五种 DependencyKey。完整配置/历史、独立使用状态、权威证据与 pins 随 preparation 保留，planned 后按原恢复寿命保留。缺历史绝不当空依赖。只有 Lease maxRuns、批准次数、grant 累计用量及实际账户 held/spent 使用上述独立 usage binding，不能取代配置 binding。

    ExternalConfirmationRequirement/1 =
        {kind:"none"}
      | {kind:"external_consent",
         key:StableControlKey/1,
         intent:ExternalRequestBinding/1,
         previewDigest:Sha256,
         principal:Token,
         notBefore:D4.zoned_instant,
         notAfter:D4.zoned_instant}

    ExternalConfirmationRecord/1 = {
      requirement:ExternalConfirmationRequirement/1,
      revision:Counter,
      current:Option<ExternalConsentConfirmation/1>
    }

    ControlPrepareBinding/2 = {
      key:StableControlKey/1,
      canonicalIntentBytes:Bytes,
      allocatedControlRefs:[ControlRef<K>/1],
      originalCommitRequest:PreparedCommitRequest/1,
      immutablePreview:ControlPreview/1,
      confirmationRequirement:ExternalConfirmationRequirement/1,
      dependencyPins:ControlDependencies/2
    }

external requirement 当且仅当 body 是外部 consent 时存在。key、真实发起 principal、完整冻结 intent 与 preview digest 等于原 preparation；notBefore/notAfter 精确等于原 ConsentSpec 区间且 notBefore < notAfter。它在生成原请求前即不可变。其它 body 必为 none 且不生成确认记录。每个 external requirement 恰有一份按完整 stable key 定位的保护记录，初始 revision=1、current=none。只有 §7 受信交互事件可 CAS 为已核验且匹配的确认。重复相同 fact 为 no-op；新合法受信事件的重新展示可用 checked revision+1 替换当前已不合资格的 fact，但保留 planned/recovery 引用的一切历史 fact。两者都不改变 requirement、原请求、preview、InputDescriptor 或依赖字节。MAX 时 budget_exceeded，不能回绕。planning/seal 在该保护记录 CAS 下读取当前匹配且有效的 fact；fact 是额外动态资格，不是冻结输入的可变成员。只有原 owner 授权/业务门通过后，缺失或不可用 fact 才按 approval_unavailable 处理；saved decision 重放先于新的确认。

D6 d10_control/1 producer 绑定完整不可变原意图、allocated refs、真实前态/拟议效果、preview、ControlDependencies/2 和固定确认要求。它的 descriptor 不嵌入自己即将生成的 originalCommitRequest：先固定 descriptor，再由 D6 生成请求，最后在交付前原子保存 ControlPrepareBinding/2 的精确关联。planned-consent body 中嵌套的原作者请求 A 仍是合法输入；本次生成的控制提交 M 只在外层关联。任何消费者都不能把 M 哈希回生成 M 的 descriptor，也不能将后来的确认插入 InputDescriptor。真实已保存 /1 记录保留原 decoder 与精确恢复，不能把旧字节改标为 /2。

canonicalIntentBytes 是完整成功 closed-decode 后的 D3-CJ/3 canonical bytes，domain tag 固定 D10-Control-Intent/1。digest 可以作为索引，冲突判定必须比较完整 bytes。prepare binding 是稳定准备/定位记录，不是第二 author decision。


共同顺序固定：

1. closed decode、D1 capability/release/surface/version/health 门；
2. 用最小受保护映射验证 current principal、scope、audience、对象观察和操作资格；未授权、wrong audience、不存在均 not_visible；
3. 验证 authority/fence/custody/continuity；
4. 查 stable key。visible same-key different canonical bytes → control_conflict；
5. same-key same-input 已有 authoritative decision 时，先重新验证当前主体对原结果和效果范围的披露资格，然后重放保存结果；此时不再用当前 target revision 否定历史成功；
6. 尚无 decision 才验证 expected config/usage revisions、当前授权、依赖、时间和预算，构造或恢复原 prepare；
7. Workspace 提交进入原 D6 transaction/ledger；Deployment 提交进入同一 store incarnation 的 closed host control transaction；
8. 原子保存效果、决议/receipt 关联、记录与账户增量、依赖失效信息、证据固定引用和必需的审计关联。

另一个合法请求已经把对象 r5→r6 后，原 r5 成功请求重试仍返回 r5 saved outcome；读取 current state 是另一个受权 read。当前主体若已经失去结果披露资格则 not_visible，但旧 decision 不能删除或改写。

失败 prepare/commit 不创建 applied decision。已存在的 prepare binding 可以在 transient state_unavailable 后用同一输入恢复；修改 expected revision/body/target 必须新 requestId。防重 binding、terminal proof、planned/unknown/uncertain pins 不能因普通 TTL 清理后让旧 requestId 再执行。

configuration revision 与 `usageRevision` 分域；checked increment 到 Counter 上限时 budget_exceeded，禁止 wrap/reset。retired id/incarnation 永不复用，防止 ABA。

### 8.1 原受保护 Record/1 前身语义（A2 current schema 在 §0）

以下都是 Core 内部类型，不是公开读取响应、新的控制记录种类或第二账本。公开 ControlCurrentView 单独不能充当完整前像。本轮尚未部署的候选按下列形状闭合保护表示；真实已保存旧记录仍按原解码器和恢复义务处理。

    D10ControlRecordImage/1 = {
      binding:Binding<K>/1,
      scope:Scope/1,
      usageRevision:Option<Counter>,
      view:ControlCurrentView<K>/1,
      supplement:D10ControlSupplement/1
    }

    D10ControlSupplement/1 =
        {kind:"none"}
      | {kind:"automation", subscription:ScheduleSubscription/1,
         stop:Binding<stop>/1, creator:Token}
      | {kind:"run", createdBinding:Binding<run>/1,
         principal:Token, fixedBudget:BudgetCaps/1,
         invocation:Option<AutomationInvocation/1>,
         admission:Option<LeaseRunUse/1>,
         authorSteps:[D10AuthorStepResponsibility/1]}
      | {kind:"planned_approval", originalRequest:D6.d6_commit_request/2,
         grantedAt:D4.zoned_instant, clockEpoch:Token}
      | {kind:"external_approval", intent:ExternalEffectIntent/1,
         requirement:ExternalConfirmationRequirement/1}
      | {kind:"activation", registrySnapshot:D4.RegistrySnapshot/1,
         registryEvolution:Option<D4.RegistryEvolutionProof/1>}
      | {kind:"reservation", attribution:CostBudgetAttribution/1,
         settlements:[CostSettlementDecision/1]}
      | {kind:"external_effect", intent:ExternalEffectIntent/1,
         attempts:[D10ExternalSendRecord/1]}
      | {kind:"stop", owner:StopOwner/1,
         latch:ExecutionStopLatch/1, receipt:Option<D10EmergencyStopReceipt/1>}

上述八类的补充标签必须等于 K；其它种类只能用 none。view 仍逐字采用 §7 的闭合形状，提供其余全部配置、生命周期和用量成员。范围、身份、修订及重叠字段必须一致。run.createdBinding 是不可变的创建绑定，fixedBudget 是在该处固定的预算；只有自动化来源的 invocation 为 some 并保存原调用，交互 Agent 运行则为 none；来源来自真实创建记录，不能用当前自动化替换。尚未准入的运行其 admission 为 none，已准入则保存完整原 LeaseRunUse。历史作者关联按 stepId 唯一且完整。计划批准保留原 A，其完整规范字节产生公开摘要。外部批准保留完整准确意图及固定确认要求。封存消费的真实确认事实与修订保存在独立保护确认记录和保存决议关联中，不能进入这个不可变拟议图像。内部保留包含安全序号的完整停止锁。凭据图像只含原凭据版本引用，受信凭据存储另行保留该不可变版本；图像和控制意图都不含凭据原值。

activation.current 是特定切面的投影。真正选择器为 D10ActivationSelector/1 = {workspaceRef:D3.WorkspaceRef, revision:Counter, current:Option<Binding<activation>/1>}。每个工作区控制域只有一个永久选择器身份，在同一事务比较，选择后继时推进一次；历史激活图像及世代不改写。初始 none 只有在保护证据证明从未选择过激活时合法。automation.subscription.activeDefinition 指向图像的拟议绑定；这个有限值不是嵌入图像，不产生引用循环。

    D10ControlRecordPin/1 = {
      image:D10ControlRecordImage/1,
      pin:D6.PinRef/2
    }

固定引用的类别为 artifact，保留类为 recovery 或 approval_money；其字节恰为 UTF-8 的 D10-Control-Record/1、一个 NUL，再接图像的 D3-CJ/3 规范字节。byteLength 与裸 SHA-256 必须匹配；D10 带前缀的摘要显示不改变 D6 摘要解码器。Core 验证原分型记录的保护存储来源后才签发引用，不能认证调用方自写字节。缺历史在披露门后为 state_unavailable，已证实图像矛盾为 integrity_conflict。ControlDependencies 中每项真实配置、用量和停止绑定都有准确图像及固定引用；多个历史修订或用量切面可并存，按完整 Ref、配置修订、可选用量修订、再按完整规范图像字节排序。同一保护切面的矛盾须拒绝，不能因修订相等而隐藏。引用既不授予公开披露权，也不让历史状态成为当前。

    D10ControlRange/1 =
        {kind:"records", scope:Scope/1, kinds:[ControlRecordKind/1],
         epoch:Token, revision:Counter, members:[ControlRef<K>/1]}
      | {kind:"cost_lineage", key:CostLayerKey/1,
         epoch:Token, revision:Counter,
         reservations:[ControlRef<reservation>/1]}
      | {kind:"occurrences", automation:ControlRef<automation>/1,
         epoch:Token, revision:Counter,
         records:[AutomationOccurrenceRecord/1]}

这些是同一 InputDescriptor.ownerInput 中受保护的领域范围值，不是新增 D6 依赖键或调用方自报证明。records 完整枚举准确范围及显式非空、排序唯一种类集合内的全部记录，包括保留的非当前状态。费用范围覆盖原归属中包含该准确累计键的每份预留；发生项范围覆盖该自动化全部代际，包括已武装和已处理状态。epoch/revision 来自实际控制存储持续维护的范围栅栏。每次匹配的插入、删除或相关修改都在同事务推进栅栏，包括移入或移出范围；达到上限后拒绝新的普通工作，不能回绕。完整同切面扫描证明正向和负向成员关系，包括空集；索引单独不能证明。事务比较栅栏及完整值，竞争插入不能逃离 CAS。前缀、最新一行或所选页面都不是完整集合。连续性缺失则暂停，已知冻结依赖变化则冲突。每个数组最多 4096 项、规范元数据最多 16 MiB；超限使完整准备失败，不保存部分效果。

    D10ControlEffectPlan/1 = {
      kind:"d10_control_effect_plan", version:1,
      changes:[{
        before:Option<D10ControlRecordImage/1>,
        after:D10ControlRecordImage/1
      }],
      activationSelector:Option<{
        before:D10ActivationSelector/1,
        after:D10ActivationSelector/1
      }>,
      recordPins:[D10ControlRecordPin/1],
      registryChange:Option<{
        before:{snapshot:D4.RegistrySnapshot/1,binding:D4.RegistryBinding/1},
        after:{snapshot:D4.RegistrySnapshot/1,binding:D4.RegistryBinding/1},
        evolution:D4.RegistryEvolutionProof/1,
        beforePin:D6.PinRef/2, afterPin:D6.PinRef/2
      }>
    }

变化按 after.binding.ref 规范排序且唯一。既有记录要求完整原前像；创建要求 none、受保护的从未使用过的已分配 Ref，以及完整负向范围证明。真正无操作不列入。不存在删除分支。每个变化的配置或生命周期修订检查旧值加一，创建为一；纯用量变化保留配置修订，只推进实际用量修订。所有未变用量、责任、历史身份及非目标配置保留。recordPins 保存完整读取图像、必需原外部意图、计划 A、Registry 图像及原预算配置，不只保存被改行。计划从闭合 body、受信主体、已分配引用和完整同切面输入确定派生。预览的 affected/resourceUses 准确投影这些真实效果，本身不是写计划。只有激活操作可以携带选择器前后像，其它 body 必为 none。工作区适配器只创建或更新各自列明的工作区记录和真实激活、Registry 控制效果，不能借此执行部署配置、对账、凭据、任意回调或作者源编辑。既有 state 操作若指向部署记录，继续按 §7 的真实 H 和领域分派，不进入本工作区适配器。

planning 将固定计划及依赖引用与原 D6 计划一起保存。seal 再比较每个尚未写入的控制前像及范围栅栏，在一个 P 事务内发布准确后像、选择器、增量和 D6 决议关联。可移植文件安装期间不提前发布或安装控制后像。记录拒绝或终态作者结果不能伪装成 D10 已应用历史。一旦封存，后续配置变化不替换保存计划或原结果。

registryChange 当且仅当激活真实改变工作区可移植 Registry 时为 some。两个固定引用都是按真实 D4 owner 认证的准确 portable_metadata Registry 分量图像；完整前后快照、绑定和演进必须匹配对应激活图像与真实 Registry 依赖，使用既有 {kind:"registry",workspaceRef} 分量键。该变化按 D6 §4.3 使用完整、严格、可移植分支，产生一个真实 ChangeId 和 CP3，不推进源或 H。Registry 不变时为 none，纯目录或选择器更新保持 control_only；其它 body 必须为 none。不可变完整激活意图向获权控制预览提供拟议 Registry，内部效果计划不据此公开隐藏历史记录。

### 8.2 准确运行准入与作者批准责任

    LeaseRunUse/1 = {
      lease:Binding<lease>/1, run:ControlRef<run>/1,
      origin:RunOrigin/1, admissionClockEpoch:Token,
      admittedAt:D4.zoned_instant
    }

    ApprovalCountReservation/1 = {
      decisionKey:D6.DecisionKey/2,
      approval:Binding<approval>/1,
      state:"unreserved"|"reserved"|"consumed"|"released_terminal"
    }

    ApprovalUse/1 = {
      approval:Binding<approval>/1, run:ControlRef<run>/1,
      stepId:Counter, request:D6.d6_commit_request/2,
      decisionKey:D6.DecisionKey/2,
      preparedBindingToken:Token,
      previewSemanticDigest:Sha256,
      delegationBinding:Binding<lease>/1,
      activationBinding:ActivationBinding/1,
      count:ApprovalCountReservation/1,
      budgetReservations:[ControlRef<reservation>/1]
    }

这些定义替代 D10 中未分型的示意列表，属于 Core 内部值，不增加提交成员。完整控制引用包含 storeIncarnation。LeaseRunUse 按完整 Run Ref 唯一，只能由第一次受保护步骤的准入 CAS 产生，等于运行来源、准确已准入租约及可信准入时间。其创建与租约用量加一原子完成。后续步骤或原计划恢复使用同一记录，不再检查剩余次数大于零或再次消费；当前准确租约配置、时间、授权、激活及停止门仍适用。准入丢失或未知只能不可用，不能重新准入。发生项认领、武装和排队创建不消费运行次数。

LeaseRunUse 正文别称是派生值，不是额外成员：leaseId 等于 lease.ref.id，leaseRevision 等于 lease.revision，runId 等于 run.id。公开 run_state.admission 准确投影这些原值及 admittedAt，不替代完整保护记录；比较身份时不能丢弃存储世代或 Ref 种类。

ApprovalUse 与计划 ConsentSpec 使用的作者预览摘要为 SHA-256，其输入是 UTF-8 的 D10-Author-Preview/1、一个 NUL，再接完整原预览 EffectManifest/2 的 D3-CJ/3 字节，包括每项效果和 DecisionKey。按 D7 闭合分型解码器定位每个 EffectBytes 槽，将运输对象准确替换为 {encoding,byteLength,payloadDigest:Sha256}，摘要取该槽完整准确固定载荷。完整分型遍历只包含以下槽：source_change 中每个 present/proposed SourceImage 的 before/after.bytes；conditional_source_change 的 before.bytes 和 result.bytes；semantic_extension.bytes；workspace_bootstrap.bytes；field_change 的 before 和 after；conflict_branch_source.bytes；以及 canonical_plan.payloads 每个元素的 before、selected 和 result。absent SourceImage 没有字节槽，其余效果变体也没有 EffectBytes 槽。未知变体必须拒绝；不得按成员名递归猜测，也不得改写已解码载荷内部的 token。payloadDigest 对完整准确载荷字节直接计算 SHA-256，不纳入运输句柄，也不添加摘要前缀。原计划和请求中的不可变 preparationBinding 与用途保护 token 保留，只替换本交付世代的运输句柄。其它成员不变。这是内部确定性摘要投影，不是新的公开 EffectBytes 格式；不纳入 handleToken、游标或交付世代。Core 先核验全部字节和长度，并保留完整原语义与引用用于准确比较，不能只留摘要。因此在新的合格交付世代重新打开原计划预览，仍得到同一摘要，不复活旧 token 或改变计划。

ApprovalUse 按完整原 DecisionKey 唯一，请求必须准确派生该键。受保护 token 必须选择该请求实际原 D7 PreparedActionBinding/3，语义摘要绑定不可变完整预览，与运输 token 分开。Core 保留实际准备记录、完整修改范围、源及依赖固定引用、原规则图像，验证 §7 的两个单字段分支后才能生成使用记录。字段读取或调用方 JSON 不能合成它。不增加第二 planToken、自由修改范围回调或重复作者请求。count 的键和批准等于外层使用记录；它是放在不可变准备输入旁的可变保护资格，不能反向参与该输入的哈希。

未见请求的 planning 在真实权威存储序列化边界内验证当前资格及 reserved + consumed < maxSuccessfulCommits，再原子保存原计划并令 unreserved→reserved。竞争最后一个名额至多一个成功。同一 P seal 执行 reserved→consumed，包括真正逐字无操作；保存结果重放两者都不执行。只有在全部安装残余解决后的原权威终态中止，才在该中止事务执行 reserved→released_terminal。其它原因保留预留。每次真实转换都检查用量计数并推进独立用量修订；完整使用集合证明总量，配置修订不清零。费用占用保持独立。

受保护的补充计划批准就是完整 planned_approval 记录图像，绑定原 A、不可变预览摘要、原租约和激活、授予主体及有限区间。恢复只有在当前授权、完整原预览、源与业务、租约、时钟和停止门通过后，才可用它替代已经失效的长期规则。它不改变计划、不释放原长期批准预留，最终成功仍消费该原预留。不把任意新批准塞进不可变描述符。所有资格记录通过原请求的保护关联定位，在 planning 与 seal 检查准确当前修订；客户端直接提交已经返回的 D6 请求也不能绕过。

## 9. Deployment value 与证据

    DeploymentValue/1 =
      {kind:"deployment_policy", value:DeploymentControlPolicy/1}
    | {kind:"trust", publisherId:Text, publicKey:Ed25519PublicKey,
       previous:Option<Binding<trust>/1>,
       proof:EvidenceTicket/1,
       claims:[NamespaceClaim/1]}
    | {kind:"package", manifest:PackageManifest/1,
       signature:Ed25519Signature}
    | {kind:"external_account",
       provider:ContributionBinding/1,
       externalAccountId:Text,
       endpointId:LocalOperationId,
       proof:EvidenceTicket/1}
    | {kind:"secret",
       account:Binding<external_account>/1,
       audience:ContributionBinding/1,
       usageKind:"authenticate",
       staged:SecretStageTicket/1}
    | {kind:"grant", spec:ResourceUseGrantSpec/1}
    | {kind:"cost_account",
       currency:CurrencyCode,
       ceiling:Counter,
       pricing:Option<Binding<pricing>/1>}
    | {kind:"pricing",
       account:Binding<external_account>/1,
       currency:CurrencyCode,
       fixedMicroUnits:Counter,
       meters:[PricingMeter/1],
       evidence:EvidenceTicket/1}

本节的发布方与扩展包信任只用于证明部署扩展的真实性。任何 D10 `trust` 记录、`PublisherIdentity`、`NamespaceClaim`、`PackageManifest` 签名、包密钥轮换或撤销、部署管理员资格或安装顺序，都不能创建或替换 D6 `WorkspaceTrustAnchor/1`、`WorkspaceTrustRootDeclaration/1`、`WorkspaceTrustDeclaration/1`、`DomainSealKeyHandle/1`，也不能授予 `CommitDomain` 签名资格、工作区策略管理权或作者写权限。D6 的 revision-seal 信任由 `WorkspaceAuthorizationBundle/1` 独立锚定并版本化；D10 的扩展包签名不能满足其引导、注册、密钥轮换或撤销、历史验证以及 `authorize_new_sign` 门。

    NamespaceClaim/1 = {
      namespaceId:D4.SemanticNamespaceId,
      ownerClass:"first_party" | "publisher",
      ownerId:NamespaceOwnerId/1,
      proof:EvidenceTicket/1
    }

`NamespaceOwnerId/1` 只是 D10 对 D4 Registry row 既有 scalar `ownerId` 的验证名称，不是新 namespace identity。`first_party` 时必须逐字是该保留 namespace 的 D4 tuple 值（`weftext.people`、`weftext.organizations`、`weftext.calendar`、`weftext.library`）；`publisher` 时必须逐字等于 accepted trust record 的 `publisherId`，并由 `namespace_claim` EvidenceTicket 证明同一 `(namespaceId,ownerClass,ownerId)` tuple。D6 随机 Token/ticket 绝不能塞进 `ownerId`；proof 使用独立 `proof` member。

`NamespaceClaim/1` 只把已验证 PublisherIdentity/first-party root 绑定到 D4 已有 semantic namespace owner tuple；不创建第二 Registry，也不能覆盖 `core`、`wf`、其它保留 owner 或另一个已验证 publisher。install order、package/display name、enablement 和字符串巧合都不是 owner proof。

    PricingMeter/1 = {
      meterId:LocalOperationId,
      numerator:Counter,
      denominator:Counter,
      maxUnits:Counter
    }

`denominator` 和 `maxUnits` 必须为正；同一 pricing 内 meterId 按 ASCII 排序且唯一。最大费用使用 checked integer/rational arithmetic，不能使用 binary float。

EvidenceTicket/1：

    {ticketId:Token,
     evidenceClass:
       "publisher_rotation" | "namespace_claim" |
       "account_control" | "pricing_contract" |
       "final_bill" | "never_started",
     evidenceDigest:Sha256}

ticket 由受信 adapter 产生并在内部绑定完整原始证据、主体、scope、provider/account/attempt。普通 caller 不能自报 verified 或把一种 evidenceClass 当另一种使用。

Secret 原值只通过 trusted secret channel：

    D10SecretStageRequest/1 = {
      kind:"d10_secret_stage",
      wireVersion:1,
      requestId:Uuid,
      account:Binding<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretBytes:Bytes
    }

只有 H 可 stage。secretBytes 不进入普通 control canonical intent、preview、log、transcript 或 Workspace。成功返回：

    SecretStageTicket/1 = {
      ticketId:Token,
      account:ControlRef<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretVersionId:Token
    }

stage 自身使用同主体/store/requestId stable key；受信 secret store 只有在能够证明相同 exact secret input 时才重放原 ticket。普通数据库事务随后只能发布该 immutable ticket 指向的 secret version。

DeploymentControlDecision/1 是受保护的内部 host domain 成功记录，其公开投影是 §7 的 applied 响应：

    DeploymentControlDecision/1 = {
      key:StableControlKey/1,
      canonicalIntentBytes:Bytes,
      changes:[{
        ref:ControlRef<K>/1,
        beforeRevision:Option<Counter>,
        afterRevision:Counter
      }],
      evidence:[EvidenceTicket/1],
      auditRef:Token
    }

只有原子成功提交才保存 decision；preflight/authorization/CAS/evidence/overflow 失败不保存 applied decision。changes 恰覆盖配置变化，真实 no-op 为空。usage/account/reservation 变化和 decision 同事务保存。

## 10. 费用 reservation 与可恢复 settlement

    CostReservation/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      account:Binding<cost_account>/1,
      grant:Binding<grant>/1,
      pricing:Binding<pricing>/1,
      currency:CurrencyCode,
      upperBound:Money/1,
      revision:Counter,
      state:"reserved" | "uncertain" | "settled" | "released",
      actual:Option<Money/1>
    }


每份 `CostReservation/1` 恰属于一个 billable `attemptId`、一个 actual `cost_account`、一个 grant、一个 pricing binding 和一个 currency。Run/Lease/Automation/Workspace/deployment 在同一 admission 中检查的是多层 ceiling/projection，不是本 reservation 的多个 actual account；同一费用只能记一次。若一个操作真实产生可分别归属到多个 actual account 的费用，则建立可分别归属的 attempts/reservations/evidence；若需要 group admission，可对这些 reservations 做原子准入，但不能把一份 reservation 改成 multi-account object。

所有可收费 attempt，包括 model、普通或只读 tool、network request 及 reconciliation，都为每份真实 reservation 建立唯一受保护 `CostBudgetAttribution/1`。Core 从真实获权 Run 和原 reservation 派生，不从模型、显示名称、普通 audit 摘要或调用方自报 billing owner 构造。在任何可收费执行开始前，同一准入事务将其与 reservation 一起保存。它是一笔收费的不可变归属，不是第二账户、成功账本或新的公共 reservation payload。

    CostBudgetAttribution/1 = {
      reservation:ControlRef<reservation>/1,
      attemptId:Uuid,
      workspaceRef:D3.WorkspaceRef,
      origin:RunOrigin/1,
      layers:[CostBudgetLayer/1]
    }

    CostBudgetLayer/1 =
        {kind:"run", binding:Binding<run>/1, ceiling:Money/1}
      | {kind:"lease", binding:Binding<lease>/1, ceiling:Money/1}
      | {kind:"automation", binding:Binding<automation>/1,
         definitionRevision:Counter, ceiling:Money/1}
      | {kind:"workspace", binding:Binding<workspace_budget>/1, ceiling:Money/1}
      | {kind:"grant", binding:Binding<grant>/1, ceiling:Money/1}
      | {kind:"account", binding:Binding<cost_account>/1, ceiling:Money/1}

数组严格按 run、lease、仅当真实 Run origin 为 automation 时的 automation、workspace、grant、account 排序，无重复。交互 Run 恰五层，Automation Run 恰六层。`reservation`、`attemptId` 等于原 reservation；grant/account binding 与其原 grant/account 完全相等。全部 ceiling 使用同一 currency。Workspace、origin 等于受保护 Run 和原 LeaseRunUse；Lease 是 Run 实际已准入 binding。automation 层指向 origin 的同一 Automation Ref，但其 binding、definitionRevision 是此 attempt 准入时实际检查的预算配置，可以晚于不可变的来源 definition；这不改写 Run 原 invocation/claim/approval。workspace 层指向该 Workspace/control domain 唯一永久 workspace_budget 身份。必要历史配置值及准确 cap 选取证据与归属共同保留，不能用当前对象替代缺失原 binding。

Core 创建 Run 时在原受保护 Run 记录中固定有限 Run BudgetCaps：Automation Run 取其不可变来源 Automation definition 的 BudgetCaps，交互 Run 取其 Run-targeted Lease 的 BudgetCaps。首版没有独立的运行中 Run-budget 扩大或重置操作。run 层 binding 是拥有此不可变预算事实的实际 Run 创建 binding；后续 execution/lifecycle revision 推进不使该事实过期，也不清用量。每次 attempt 同时检查当前合格 Lease、适用时的当前 Automation budget、当前 Workspace budget、grant 和实际 account。其它层调高不能逃离固定 Run cap，当前更窄层仍约束新 attempt。这些检查不增加 Lease/approval 权限，也不解禁其它当前门失败的原 planned request。

每层累计键准确由 kind、完整 owner ControlRef、完整实际 cost-account ControlRef 和 currency 构成，按包含 storeIncarnation 的完整 canonical 值比较。配置 revision、Automation definitionRevision、grant 续期、requestId、列表位置和进程/cache epoch 都不是累计身份。account 层跨 Workspace/grant 计入该实际账户全部 reservation，grant 层计入原 grant；其它层只计保存归属中含该准确 owner key 的 reservation。因此共用 grant 的两个 Automation 不共用 Automation 上限，但仍竞争同一 grant/account 和适用 Workspace 上限。

本代没有周期重置。改预算、新 definition、停启、归档、重启和换 grant 都保持原 held/spent。提高 cap 是同谱系调额；降低到该谱系已证明 spent+held 以下，管理操作以 budget_exceeded 拒绝。从 BudgetCaps 列表移除账户只禁止经过此层的新 attempt，旧责任全部保留；重新添加时仍比较原累计。真实新建 Run、Lease 或 Automation 让新的非复用 owner 有自己的层，但不搬移旧 reservation，公共 Workspace/grant/account 层继续计入旧责任。workspace_budget 身份在每 Workspace/control domain 只创建一次，不能替换清零。cost_account currency 创建后不可变；换币必须真实新建独立账户并保留原责任，不隐式换汇或改名转账。

所有货币投影来自一份完整权威 reservation/attribution cut：reserved、uncertain 都把完整 upperBound 计入 held；settled 将 actual 计入 spent；released 计零。其它状态、部分账单或暂缺记录均不表示零。新 reservation 准入前，checked arithmetic 必须证明 spent+held+新上限不超过全部实际适用当前 cap，并满足原每次/次数/非货币限制。一个实际操作若需多个可分别归属账户，所需整组在同一准入事务内全部通过或全部不准入，每笔实际费用仍只独立归属一次。

准入、预算配置变更和结算共用同一实际 Authority Store 序列化边界。准入比较全部相关配置 binding、Run 不可变预算来源、原 grant/account usage revision、stop/当前权限以及完整 reservation/attribution membership cut，再原子保存 reservation、归属、实际 held/次数增量、保留证据及 audit。必须由完整范围/phantom 证明或实际共享 account usage fence 覆盖影响这些键的每个并发插入；只检查先前返回的旧行不够。争最后额度的并发 attempt 至多一个赢。Run/Automation/Workspace 货币投影不新增独立 usageRevision 或余额账本；索引可重建且必须对照同一受保护事实验证。无法提供此单一原子边界的部署不能声称此准入路径可用，也不能把两个数据库各自成功拼成一次成功。

已有 attempt 在任何新准入分支前恢复原 reservation 和完整归属；存在性/连续性未知时不建立替代归属或 attempt。Run 终止、Lease 过期、grant 退役、authority 恢复、stop 或普通 GC 均保留所有必要原配置/pins 及未决债务。保留证明或压缩只有仍能证明准确各键 held/spent、原 reservation/结算防重以及全部未决责任，才可替代原存储；不能把已用变成零，也不能依赖当前 Run 名称/配置。

结算先恢复原完整归属，再执行原证据/revision CAS。同一事务从每个原层累计键的 held 减去 upperBound；settled 把唯一 actual 加入这些同一键的 spent，released 不加 spent。同时原子更新原 grant/account 投影、全部派生层索引或其失效、证据和 audit。当前配置或 definition 改变不重归账；同 owner/account 后继配置自动看见更新后的同谱系总量。对账不要求旧配置/grant 仍能发起新执行，但 reconcile 主体仍须有原实际账户的当前权限。归属/连续性缺失为 state_unavailable，保留全部责任；受保护事实已证明矛盾时，control read/settlement 为 integrity_conflict，Run 执行前为适用 control_conflict，均在普通可见性门之后。reservation 投影或错误不披露隐藏 Run/Lease/Automation 细节。

准确 settlement 重放不二次减少 held 或返额。非 final 证据继续 uncertain 并占全额；uncertain90 后可靠 final20，在全部原适用层恰作 held-90、spent+20，返70一次。actual 超 upperBound 走原 overcharge/freeze，不借对账抬高 ceiling。author abort、TTL、cancel 或 Run terminal 不替代 never-started/final-bill 证据。公共 reservation_state 仍只返回原 CostReservation shape；能够查账户不意味着自动披露此受保护归属。

`actual` 仅在 state=settled 时为 some；其它状态必须为 none。

`attemptId` 与 `reservationId` 永不复用。

状态机：

    reserved -> settled(actual) | released | uncertain
    uncertain -> settled(actual) | released

settled 和 released 是终态；uncertain 是可恢复非终态并继续占用完整 upperBound。

released 仅在可靠 never_started 证据证明本 attempt 的 billable execution/send 从未开始时合法。已经发送、后来可靠最终账单为 0 必须 settled(0)，不是 released。可靠 final_bill 只可归属同 reservation/attempt/account/currency/pricing，得到 settled(actual)。例如 reserve100→uncertain→final bill20 必须 settled(20)，只返回 80。

    CostSettlementDecision/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      priorRevision:Counter,
      kind:"settled" | "released",
      actual:Option<Money/1>,
      evidence:EvidenceTicket/1
    }

`settled` 要求 `actual=some`；`released` 要求 `actual=none` 且 `evidenceClass=never_started`；`final_bill` 只能得到 settled。attempt、account 或 currency 不匹配、账单尚未 final、汇总账单无法唯一拆分，或证据连续性不足时，都保持 uncertain 并占用完整上限，返回 `state_unavailable`。已接纳适配器给出格式错误结果时沿其原 invalid-output 合同处理，不能猜测结论。

reconcile 用 expected reservation revision CAS。成功事务原子追加 CostSettlementDecision、更新 reservation state/revision、grant/account held/spent/available、evidence/audit。相同 evidence/prior revision/derived decision replay 不再次返额；并发不同 decision 至多一个 CAS winner。actual 超 upperBound 进入原 overcharge anomaly/freeze 路径，普通 reconcile 不提高 ceiling。

管理员无证据输入 0、effect idempotency、业务 rollback、author abort、TTL 或 Run terminal 都不是 released/settled 证据。ApprovalUse count、LeaseRunUse 和 cost reservation 继续三域独立。

## 11. Emergency stop 与线性化

Public stop 是独立于普通 control prepare 的专用 safety transaction：

    D10EmergencyStopRequest/1 = {
      kind:"d10_emergency_stop",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:
        ControlRef<automation>/1
      | ControlRef<run>/1
    }

首代要求 `requestId == target.id`。因此 requestId 是 exact target 的确定防重坐标，不是 caller 自选 capability。请求仍携带完整 target Ref；任何 text、display name、裸 UUID 或 ToolValue 都不能提升为 target。

    StopOwner/1 = {
      storeIncarnation:Uuid,
      workspaceRef:D3.WorkspaceRef,
      target:ControlRef<automation|run>/1,
      latch:ControlRef<stop>/1,
      requestId:Uuid
    }

    StopCapacity/1 = {
      issued:Counter,
      reserved:Counter
    }

受保护 stop stable key 是 `("D10-Emergency-Stop/1",target.storeIncarnation,target.kind,requestId)`。`requestId==target.id`，target/latch 的 storeIncarnation 都等于 StopOwner.storeIncarnation，而且一个 exact target 恰有一个 latch。workspaceRef 来自受保护控制状态中的真实 target Workspace。D10 storeIncarnation 是绑定实际打开 D6 authority-store backend 的内部持久控制域 incarnation；它不是已冻结 D6 public API 名称、WorkspaceId、filesystem path 或 authority token。

W 以 target 的准确 Workspace scope、H 以真实 deployment storeIncarnation 和 target-specific 当前 host 权限，遵 §7 严格 Stop-only 映射，从**唯一**持久 W-scoped latch/StopOwner image 读出一份结果，不创建第二份 Stop record/result。首次 stop、失答查询及 r5→r6 保持相同不可逆状态、回执与预留容量。

在首次 enable/admission 前，对象创建必须原子预留一个 durable latch、一份 StopOwner 关联、一个 audit/result slot 和一个 safety-sequence capacity 单位。若无法预留，该 executable object 就不能进入可管理/可运行状态。普通 control prepare、配置 Counter 空间、budget、approval/lease count、cost 和 executor quota 都不能消费这些容量。stop 不要求 target configuration revision，也不要求先执行普通 management write。

    ExecutionStopLatch/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal/1>,
      stoppedAtControlSequence:Option<Counter>
    }

open 时两个 Option 都必须 none；stopped 时两个都必须 some。唯一转换是 Binding revision 1/open → revision 2/stopped；没有 clear。Safety sequence 预留满足 `reserved <= MAX-issued`，MAX=2^63-1。首次 stop 原子执行 `issued:=issued+1`、`reserved:=reserved-1`，写入 sequence/actor/latch revision、immutable stop result/audit link 与必要 invalidation。因此容量耗尽只能拒绝新 executable target 建立，不能阻止既有已预留 target 的首次 stop。

    D10EmergencyStopReceipt/1 = {
      kind:"d10_emergency_stop_receipt",
      wireVersion:1,
      requestId:Uuid,
      target:ControlRef<automation|run>/1,
      latch:Binding<stop>/1,
      state:"stopped"
    }

receipt 要求 latch revision 2，是单次 latch transition 的确定投影；它既不是 D6 author receipt，也不是第二成功账本。

    D10EmergencyStopResultRequest/1 = {
      kind:"d10_emergency_stop_result",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:ControlRef<automation|run>/1
    }

    D10EmergencyStopResult/1 =
        D10EmergencyStopReceipt/1
      | {kind:"d10_emergency_stop_not_applied",
         wireVersion:1,
         requestId:Uuid,
         target:ControlRef<automation|run>/1,
         latch:Binding<stop>/1,
         state:"open"}

`not_applied` 只在该读取线性化点已经证明 latch revision 1/open 时成立，不承诺稍后无人 stop。

stop/result 固定顺序：closed decode 与 D1 automation.stop gate → 当前 authenticated principal、准确 W/H scope 与 target/stop 披露资格 → authority/fence/custody → protected stable-key/target equality → latch/result continuity → transition 或读取 → 输出前再次 current disclosure gate。hidden、missing、wrong-scope、wrong-store 统一 `not_visible`；authority/fence/custody 不可证明为 `authority_unavailable`，确定破坏为 `integrity_conflict`。已获可见性后，同 stable key 绑定不同 target 为 `control_conflict`；latch/result continuity 暂不可证明为 `state_unavailable`，绝不能返回 not_applied 或新 stop。

例如 target 配置在 r5 时 stop 成功但响应丢失，另一个合法配置随后把 target 改到 r6；同 exact target/requestId 重试仍返回原 receipt。当前 r6 不会使 stop receipt 失效或改写。若之后丧失 result disclosure 权，则查询返回 `not_visible`，而 latch 继续 stopped。

线性化规则：

1. 新 Run admission 与 safety transaction 共用同一个 Authority Store serialization domain。admission 检查 latch 并原子写 occurrence claim/LeaseRunUse。stop 先完成则不能 admission；admission 先完成则已消费计数保留，后续每个 protected step 仍检查 stop。
2. D6 final author commit 必须在真实 final author transaction 内、持有同一 write-serialization boundary 时重新检查全部关联 stop latch。commit 先完成保留 committed result；stop 先完成阻止新的 commit。
3. External transport 与 stop 共用一个 send fence。在首次不可逆 handoff 前，Core 已冻结 immutable ExternalEffectIntent 与 exact ExternalExecutionBinding，重新检查 stop 和全部 approval/grant/secret/cost binding，durably 记录 send-attempt/started evidence 与 holds，并把 fence 持有到第一次真实 send handoff。stop 先完成则不发送，handoff 先完成则保留原 send attempt，并可后续进入 outcome_unknown。
4. crash 后先恢复 latch、send-attempt evidence、reservation 和原 decision，再恢复 executor。unknown evidence 绝不能变成 cancelled，也不能支持新 effectId、idempotency key 或 OperationId。
5. stop 不阻断当前受权 authoritative abort、cost settlement、evidence/audit retention 或 reference-safe cleanup；这些操作不恢复 executor authority。
6. 临时 disable、Lease expiry、临时撤权或 ordinary cancel 不是 irreversible abort proof。

stop safety transaction 是同一 managed Authority Store 内的专用 closed write，不是 D10ControlPrepare，也不是 D6 author transaction。配套 D6 amendment 只定义 D6 final/planned author work 怎样消费 latch，并返回 `execution_stopped/preflight` 或原 authoritative-abort 结果。stop 不伪造普通 control history，不回滚 committed 作者事实或已经发送的 external effect，也不会仅因停止执行就释放费用。

## 12. 当前 D6 Policy3/Profile4/Plan4/Genesis2 的唯一消费

**当前唯一 owner** 是 D6 Control §§10.2/20.2 与 FC SCHEMAS §9：fresh `WorkspaceBootstrapProfile/4`（`d6_bootstrap_profile`,wireVersion=4）、`WorkspaceBootstrapPlan/4`（`d6_workspace_bootstrap_plan`,wireVersion=4）、`WorkspaceTrustGenesis/2`（version=2）、Policy/3 与 D8 必需的 `D8PresentationPolicyBootstrapInit/1`。D7-REGISTRY-QUALIFICATION B13 消费同一真实版本。D10 只向**经 issuer 正式准入的新 family** 的初始 Policy/3 增加 workspace-scope 无参数 `d10_control_self`，不是第二 bootstrap 生产者，也不改 Profile4 的六成员 wire。

固定 profile/2 非 Field 集合保留 `workspace_state, entity_state, locator_state, source_read, source_write, body_write, node_control, node_create, resource_read, resource_write, annotation_read, annotation_write, lifecycle, registry_admin, binding_admin, policy_admin, export, repair, audit, source_envelope_state, commit_sequence_state`。新签发 Profile4 初始 creator 的 Policy/3 grant 为上述集合，加 D6 的 `replica_register, replica_retire, conflict_read, conflict_resolve, execution_custody_admin, structure_state, portable_frontier_state`、D10 `d10_control_self` 与原完整目标 Registry Field read/write grant 派生，deny 为空。各能力按 D6 真实 scope，包括 CommitDomain 限制的 commit_sequence_state；新 Field/未来 capability 不自动授予。只有当前 `administer_issuer` 才能显式更新 issuer profile 供**此后新 family**使用，分配仍独立要求 `allocate_workspace`。既有 Workspace 只能由原当前 `policy_admin` 走完整 Policy/3 事务取得 d10_control_self；Field-only 不能自授，resume/login 不恢复创建者权限。

**未见过的 fresh** create_workspace/fork_workspace 仍由 D3 拥有原真实认证 proposal、issuer 与目标 custody/allocation；实际 Registry/Policy3 和 fork 完整源历史先于唯一 D3 planning CAS 全量准入。create 用 issuer 证明的真实空目标与固定合格 Registry seed；fork 携带并映射完整源 Registry evolution/retirement、配置、来源与源权威，不可冒充 create。只用一份原 OperationId、DecisionKey、final P。受信宿主暂存 root keypair 和两个 server DomainSealKeyHandle/2，caller 不提供。`WorkspaceTrustGenesis/2` 有**且只有两条**同一 DecisionKey 与最终 activation ChangeId 的签名 `WorkspaceTrustDeclaration/2` authorize：revision-token profile revision1，然后 source-transform profile revision2；第二条 predecessor 哈希完整第一条字节，各有 PoP/root signature。WorkspaceAuthorizationBundle/2 的 authorizationRevision1/trustRevision2；不能从单 profile prefix 激活。

Plan4 **必须**携带 D8 owner `D8PresentationPolicyBootstrapInit/1` 形式的 `initialPresentationPolicy`，其 before 来自原 issuer/custody 证明的**真实空历史**，不是缺少文件；受保护 head stamp 初始 revision1，proposal `parents=[]`、`revision=1`、`defaultPresentation="separate"`，typed `presentation_policy_change` preview 的 `committed=null`。planning/staging 固定原 plan/init 字节、两个 staged handles、root/PoP、Registry/series/period/source/Policy pins；**P 前不**产生 ChangeId 相关 D8 /2 record/hash/pin/address/effect/receipt/outbox/head 或可用密钥。只有**唯一原 final P** 重查 issuer/target custody、原 source/Registry/Policy/trust/CP4 依赖，分配原 ChangeId，原子发布 D8 /2 record 及准确 hash/pin/address、committed effect、原 receipt/Notice3/CP4/ChangeRecord1 关联、outbox、one-head transition、Workspace 激活及**两** staged→usable handles。输家/abort 不留半态；不存在第二 CAS、第二 P/ledger、事后 SetRequest 或修复窗口。

真实 saved/planned/unknown Plan4 只恢复**同一** D3 plan/OperationId/DecisionKey、冻结 init、staged handles、source pins 与原 P，不重选最新 seed 或重建 root keys。真实历史 Profile1–3、已记录 Plan1/Plan3/Genesis1、真正旧 decoder 保留准确字节、pins、issuer grants 和恢复；未部署候选前身命名不授权迁移层。ordinary managed copy **不是** bootstrap。continue/failover/restore/committed replay 保留真实已保存/当前 policy、presentation heads、trust 与原回执，不恢复 creator 权限。产品/runtime 全 UNRUN。

## 13. Public capability 与首版 D6 错误兼容

D10 定义、D1 发现/发布的候选 capability IDs：

- automation.manage
- workspace.extensions.manage
- deployment.external.manage
- automation.stop
- automation.author_submit

这些 ID 必须先进入所选 D1 contractMajor 的正式 capability catalog，并继续经过所有真实 release/surface/policy/version/component/configuration/reachability/health 门。文档接受或 coordinated design activation 不使其自动 available。

第一份共同公开的 unattended author-submit 合同直接包含 D6 approval_unavailable/preflight 与 execution_stopped/preflight，不建立匿名 old/new D10 error profile。未知 capability ID 仍按 D1 unsupported_feature；不允许的组件版本组合按 D1 incompatible_version。固定 D6 Policy/1/2、bootstrap profile/1/2 和历史 saved decision decoder 不因删除未发布 D10 profile 文字而删除。

## 14. 原子性、audit 与保留

Workspace 控制改变若影响 author decision，必须进入原 D6 authority store transaction；Deployment host 控制只在同一个 store incarnation 的 closed host transaction 中改变 deployment records。不能依赖多个独立数据库、HTTP 服务或 ATTACH/WAL 猜测原子性。

每个成功受保护控制操作原子保存：record/config change、usage/account delta、decision/receipt linkage、dependency invalidation、evidence pins 和 required audit link。普通受保护执行在必要 audit start 无法耐久保存时 fail closed；emergency stop 使用 enable/admission 时预留的独立安全槽，不被普通 audit/budget quota 阻断。

retire/archive 不删除仍被 saved decision、planned recovery、unknown external effect、uncertain cost、evidence、tombstone/migration 或 active binding 引用的记录。容量不足可以拒绝新普通操作，不能通过删除防重历史让旧 requestId 再执行。

## 15. R05 验收反例

至少必须覆盖：

1. 窄 Field 用户拥有 d10_control_self、真实 Field 权限及管理员发放的 cost grant，可以建立自己的有限 automation；没有 grant 或自助 capability 时明确拒绝。
2. 同 stable key 的 r5 写已提交但响应丢失，另一个请求把对象改到 r6；原请求重试在当前披露授权通过后返回 r5 saved result，不重复建对象，也不因 r6 误判 stale。
3. same key 修改 target/body/expected revision 返回 control_conflict；旧 id 删除后同名重建不能命中旧请求。
4. grant revision/renewal 不清零 spent/held/attempts；换新 grant 也不抹旧 reservation/account liability。
5. 预留 100 后实际发送，崩溃恢复为 `uncertain`；同 attempt 的最终账单 20 得到 `settled(20)` 且只返 80；最终账单 0 得到 `settled(0)`；只有 never-started proof 才得到 `released`。
6. 两个 reconciler 并发同 reservation 只有一个 CAS winner；提交后丢响应 replay 不双返余额。
7. stop 与 Run admission、D6 final commit、external send fence 的每个线性化顺序；stop 后 settlement/authoritative abort/evidence cleanup 仍可执行。
8. profile/2 family 在软件升级后不获得 d10_control_self；显式 profile/3 只影响之后新 family；既有 Workspace 只能 policy_admin 显式授予。
9. connector contribution unavailable 不停用同 package 的 schema/template/data pack contribution。
10. Calendar/Library/People/Organizations 的 PackageId、module contribution、schema contribution 与 D4 namespace/Facet 一一映射但类型不混同；第三方同名 package 不取得 first-party D4 owner。
11. 设计文档 accepted 但 release 尚未交付、Mobile unsupported、policy deny、版本组合不允许或 provider unhealthy 时，D1 仍返回真实 unavailable reason。
12. 当前 caller 失去对象披露权后，同 stable key result 查询返回 not_visible，但保存 decision 不删除。

调度坐标及 missedWindowSeconds 复用 CanonicalDecimal 的准确字符串词法，但分别使用 §16 的坐标来源/完整键预算及正有限时长边界；它们不是 ToolValue，不要求虚构 ToolType。

## 16. 稳定订阅、发生项与调度决议

本节唯一拥有 AutomationSpec 和 RunOrigin 引用的闭合调度值。Core 内部在 Run 创建前进行的 Automation 扫描/arming/claim 属于 §1 的 D10RunStepError/1 调度域；管理配置仍使用 D10ControlError/1，且不增加公共调度 RPC。它修订尚未部署的候选定义；真实历史 claim 或 Run 继续使用原 decoder、完整键、配置、结果与恢复责任。不能从新定义补造缺失的历史证据。

    SourceOccurrenceKey/1 =
        {kind:"once", originalStartUtcSeconds:CanonicalDecimal}
      | {kind:"recurrence", originalStartUtcSeconds:CanonicalDecimal}

    AutomationOccurrenceKey/1 = {
      automation:ControlRef<automation>/1,
      subscriptionGeneration:Counter,
      sourceOccurrence:SourceOccurrenceKey/1
    }

    ScheduleSelection/1 =
        {kind:"once"}
      | {kind:"recurrence", source:D6.SourceVersionRef/1}

    AutomationScheduleUpdate/1 =
        {kind:"initial", selection:ScheduleSelection/1,
         fromOriginalStart:D4.zoned_instant}
      | {kind:"continue", selection:ScheduleSelection/1}
      | {kind:"replace", selection:ScheduleSelection/1,
         fromOriginalStart:D4.zoned_instant}

创建只接受 initial，并从 subscriptionGeneration=1 开始；已有 Automation 只接受 continue 或显式 replace。selection 分支必须等于 definition.schedule 分支。once 没有 source；recurrence 选择绑定定义中真实 owner 和两个准确 Field occurrence key 所在的完整当前合格 Observation。当前授权及 Registry 资格先于 source/Entry 查阅。需要 managed inner revision 的源必须先通过原有显式准入；配置操作不写入或隐式准入源码。输入 sourceToken 只属于原 prepare cut，不是永久的未来当前资格。

原发生坐标是从 D4 已接受 instant 以整数/十进制算术导出的、自 1970-01-01T00:00:00Z 起的精确 Gregorian UTC 秒数。once 取 at；recurrence 取 row.originalStart，绝不取 replacement 后的最终开始时间。CanonicalDecimal 去除多余零、指数写法和负零，保留全部小数位并允许负坐标。禁止转机器浮点、截到微秒或受限于宿主日期库的年份：合法本地年份 0001/9999 可能换算至 UTC 年 0/10000。等价 offset/小数拼写得到同一坐标；不同 originalStart 即使被移动至相同最终开始时间仍是不同发生项。date-only 输入不自动补午夜。完整 canonical key 上限为 131072 UTF-8 bytes；超预算拒绝，不截断或用哈希代替相等。

键相等比较完整闭合值。排序依次比较完整 Automation Ref canonical bytes、generation 数值、固定 kind 顺序 once 在 recurrence 之前、精确 UTC 数值坐标。键不含 definitionRevision、source revision/token、Field occurrence key、projection horizon、Query 调用/算子/parent key、cache epoch 或最终 due 时间。subscriptionGeneration 是同一非复用 Automation 内的正 checked Counter，不新增作者身份空间。initial/replace 将 fromOriginalStart 规范为固定、包含端点的原发生下界；修改投影窗口不能改变它。

实际受保护源证明保存在下列闭合内部记录中。这些记录不授予公共读取或执行权限，也不增加公共 control-record kind：

    ScheduleRecurrenceEvidence/1 = {
      observation:D6.SourceObservation/1,
      sourcePin:D6.PinRef/2,
      metadataPin:D6.PinRef/2,
      dependencyProof:D6.DependencyProof/2,
      registrySnapshot:D4.RegistrySnapshot/1,
      registryEvolution:Option<D4.RegistryEvolutionProof/1>,
      recurrenceContext:D4.RecurrenceReadContext/1
    }

    ScheduleSourceBinding/1 =
        {kind:"once", atUtcSeconds:CanonicalDecimal}
      | {kind:"recurrence", ownerNodeRef:D3.NodeRef,
         recurrenceOccurrenceKey:D4.occurrenceKey,
         rangeOccurrenceKey:D4.occurrenceKey,
         initial:ScheduleRecurrenceEvidence/1,
         checkpoint:ScheduleRecurrenceEvidence/1,
         continuityPins:[D6.PinRef/2]}

    ScheduleSubscription/1 = {
      automation:ControlRef<automation>/1,
      generation:Counter,
      lowerOriginalStartUtcSeconds:CanonicalDecimal,
      activeDefinition:Binding<automation>/1,
      definitionRevision:Counter,
      source:ScheduleSourceBinding/1
    }

recurrence 的 sourcePin 必须是该 Observation 完整 managed SourceVersion 和 owner 的 exact_source_document；metadataPin 是同 cut 对应的准确 portable_metadata 图像，证明 owner 的生命周期/身份，不能借另一 head 的 metadata。DependencyProof 属于当前 observerDomain/cut，完整覆盖实际 source/entity、授权、Registry、temporal rules 及 D4 producer 必需的其它业务范围。RegistrySnapshot 和除真实 bootstrap 外必需的 evolution proof 构造实际不可变 ValidatedCatalogContext。recurrenceContext 是实际完整、有限的 D4 context，包含规则来源、准确 revision 和 coverage。raw bytes 或单独解码 Entry 不代替 D4 对真实 Event/range-note Facet 与所选 calendar/recurrence、calendar/range Entry 的当前接受。initial 证据不可变；checkpoint 只有通过下述连续性证明才能推进。pins 必须是准确分型的 Core pin，按 token 排序唯一；每次 checkpoint 更新至多 4096 个 pin，完整更新的 canonical evidence 上限为 16 MiB，不计另行预留容量的不可变 source/component payload。证明不完整或超预算时不能部分推进 checkpoint。

连续性是真实受保护执行依赖，不是 I cache 事实。从前一 checkpoint 到新当前 cut 的每个中间 sealed source/control 状态都必须证明：所选 owner 持续 live、所需真实 Facet 始终存在、两个准确 Field key 始终存在、完整调度业务值没有改变。业务值比较使用 D4 解码后的 canonical recurrence/range 值和实际语义定义/规则 provider；只忽略源码格式以及不参与这些调度语义的 Entry note/qualifier/provenance 成员。不能忽略 recurrence、range、相关 schema、calendar rule、timezone 或 tzdb rule version 的变更。删除再创建后复用 key/最终值、Facet/lifecycle 暂时消失、规则改后复原都不构成延续。其它无关 Entry/body 修改在完整真实链证明上述不变量时，可以自动推进 checkpoint。

producer 必须保留完整、已验证的 D6 ChangeRecord/InstallationNotice/ContentCompletionProof 链及所有必要中间 source/metadata component，或者保留从该准确链与实际相关 control/rule 历史持续无间隙消费生成的受保护 witness。continuityPins 按原 decoder 固定那些原版本记录及完整 source/control 证据，不能装调用方自报结论、仅变大的 Frontier 数字或自由 proof-map 协议。witness 必须证明直到当前 checkpoint 的每条因果分支和相关控制转移，没有遗漏区间、重置或未观察前驱，并保持原 source selection 与 rule identity。只有保留的受保护 witness 仍证明相同不变量和全部未决责任时，压缩才可丢弃旧 payload。相等最终源码/hash、重建索引、变化的读取交付 token 或 provider 自称 synced 都不够。observed_only 只证明保留的 B/N，不证明不存在未观察 C；external/observer gap 或缺失中间历史不能获连续性认证。仅扩展 horizon 或重新建立本地观察本身不等于业务规则变化；必须证明实际 provider/rule 身份没变，并证明新增范围的完整 coverage。在内部验证必要历史前，先检查当前 source/Field/owner 授权；不向调用方返回隐藏历史内容，也不扩大 Lease。

D6 producer 将这些订阅依赖纳入受保护执行恢复，使用既有 recovery retention class 和实际 control-store 责任。它可以持续消费已验证变更并维护受保护 witness，也可以在下一次扫描前验证已保留的完整历史；两条路径都不能在正确性证据丢失后从 I 重建。历史无法证明时以 state_unavailable 暂停新调度；已证明 source/selection/rule 不连续时，调度域返回 binding_changed，要求显式 replace。在管理 prepare 中，相同已证实不一致使用 control_conflict，绝不返回仅属于 Run 域的 binding_changed。replace 固定新的合格源和下界，不伪称恢复了旧链；原 claim 和 unknown 请求仍可恢复。此 D6 留存/消费联合合同必须实际整合后，该路径才可用。

continue 保持 generation、下界和 schedule 分支：once 必须保持准确规范化 at 坐标；recurrence 必须保持真实 owner、两个所选 key 并证明上述完整连续性。改变分支、once at、source selection 或调度业务值必须显式 replace。修改 invocation、Lease、approval、budget、missed policy/window 或有限 projection horizon/limit 可以推进 definitionRevision 和配置 binding，但不能重新给已处理发生项编号。真实同定义 no-op 不虚构 revision。enable/disable/archive 也不改变历史键。replace 在同一事务中退休旧代创建新 claim 的资格，checked-increment generation，并固定新 source/lower bound；MAX 拒绝而不回绕。各代既有 claim 都保留原 Run、不可变 origin/definition、invocation、Lease/approval 关联、queued/blocked/unknown/terminal 状态及准确已保存请求。

唯一定义激活点是原 automation_configure control commit 在与 occurrence 决议相同实际 Authority Store 序列化域中的成功点。先于该 commit 赢得 claim 的项保留旧定义；其后首次赢得 claim 的项采用当前新定义，包括有限 missed window 内合格、此前未处理的过去坐标。armed 尚非 claim，也不预留旧定义。不能按墙钟猜测或 cache 扫描顺序分派。配置、source-checkpoint 更新、arming、claim、跨代排除与 stop/custody 检查共用真实事务 fence；并发 source/control 变化使未提交扫描失效或重试，不能执行部分选择。

    AutomationOccurrenceDisposition/1 =
        {kind:"armed", clockEpoch:Token,
         armedAtUtcSeconds:CanonicalDecimal}
      | {kind:"claimed", run:ControlRef<run>/1}
      | {kind:"skipped", reason:"missed_policy"|"outside_missed_window"}
      | {kind:"handover_skipped", prior:AutomationOccurrenceKey/1}

    AutomationOccurrenceRecord/1 = {
      key:AutomationOccurrenceKey/1,
      definition:Binding<automation>/1,
      definitionRevision:Counter,
      dueUtcSeconds:CanonicalDecimal,
      proof:ScheduleOccurrenceProof/1,
      disposition:AutomationOccurrenceDisposition/1
    }

    ScheduleOccurrenceProof/1 =
        {kind:"once", at:D4.zoned_instant}
      | {kind:"recurrence", evidence:ScheduleRecurrenceEvidence/1,
         projection:<complete accepted D4 recurrence projection outcome>,
         originalStart:<that outcome row's D4 originalStart>}

recurrence proof 保存实际完整 D4 outcome，包括 projectionIdentity、全部 rows 和 readSet；准确 originalStart 定位唯一一行。owner、managed source revision、两个 source key、Registry/read binding 和有限 horizon 都等于实际使用的证据。它不授予 D7 Query 完整性或执行 custody。dueUtcSeconds 是该行最终 range.start 的精确 UTC 坐标；once 取 at。过滤使用原 D4 replacement-aware 算法，必须包括从窗口外移动进来的 exception。合格项按精确 final due，再按 SourceOccurrenceKey 的固定 tag/数值坐标排序；不同原坐标不因最终 due 相同而合并。

armed 是同一 occurrence store 的耐久前态，不是 Run 或 LeaseRunUse。Core 只有在当前资格完整、可信读取时间严格早于 due、真实未来候选满足订阅原发生下界时，才能创建它并保留 arming 时刻及证明。clockEpoch 绑定真实可信时间来源；arming 的原子提交点重新证明 armedAtUtcSeconds < due，不能用计算开始时较早的时间倒签已经到期的项。时钟或事务点资格不可证明时不推进任何状态。不能为已过去的项补造 armed 来绕过 missed policy。完整 future projection 可以按 queue/storage 预算只武装最早的有限前缀；这个明确受限的 lookahead 不表示所有未来发生项都已武装。armed 项一旦到期，即使晚唤醒或重启也沿普通到期分支处理；due<now 本身不将它变成漏跑项。armed→claimed 前重新验证当前 subscription/source/rules/authority，并固定当前 active definition；转移在同一事务中创建唯一原 Run 及不可变 origin，替换 armed 记录，在原 Run-admission CAS 前不消耗 maxRuns。replace 后旧代 armed 记录不能 claim，也不算已经处理的坐标。

AutomationSpec.missedWindowSeconds 是有限正 CanonicalDecimal 时长，最多 31557600 秒。一次可信 scanTime 下，补跑区间准确为 [scanTime−missedWindowSeconds, scanTime)。只有该区间内已到期、无 armed、此前未处理的合格坐标使用 missedPolicy；已经 claimed/skipped 和已到期 armed 的记录先恢复或处理，不能重新分类。早于该区间的 once 项耐久记录为 outside_missed_window 跳过。recurrence 更早候选不必枚举成无限 skipped 历史；当前固定 schedule 不从已过期窗口执行它们，也不伪记为逐项 claimed。后续显式 replace 仍遵循新 source/lower-bound 和实际已处理坐标排除规则。

recurrence 的 configured horizon 是有限获准投影边界，不是发生项身份，也不替代补跑区间。用于一次决议的 D4 projection 必须完整覆盖整个适用补跑区间和正在处理的全部已到期 armed 项；若配置 horizon、temporal coverage、output/work/evidence 预算做不到，整个决议失败，不保存部分 disposition。future lookahead 仍在该 horizon 内。调用方不能缩小 page/window 来隐藏较新的漏跑项。有限当前投影和当前 source/rule 证明必须在当前 cut 重建；已保存 claim 在新扫描资格检查前始终先恢复原责任。

一次 missed-window 选择是既有 Authority Store 内的一次原子业务决议。固定 scanTime、准确窗口、active definition、完整合格集合及排序、先前已处理排除、policy 和完整 proof，然后在同一事务中保存每项结果 disposition、选中的 Run/origin 以及 queue/capacity reservation。skip 将每个合格漏跑项标为 skipped；run_once 只 claim 按 due/key 排序最大的合格漏跑项，其余全部标 skipped。已有原 claim 或 handover 排除的项不得产生第二 Run。完整决议全部提交或没有一条 skipped/claimed 新行；写入中途崩溃不能丢掉选中的 Run，也不能让后续重扫另选第二个更早项。queueLimit 计入 queued/active claimed Run；受保护容量还必须限制 armed/lookahead 记录。容量不足使整个适用决议失败；仅创建 claim 不消耗 Lease maxRuns。已提交决议恢复准确原 rows/Run；未提交计算丢弃后，按新的当前完整 cut 重算。

新代 claim 某坐标前，同一事务必须证明此 Automation 的完整先代已处理集合。准确 UTC 原坐标相同的先代 claimed、skipped 或 handover-skipped 都排除新 Run，once/recurrence 跨分支亦保守排除；新代写 handover_skipped，指向最初的非 handover 已处理 key。沿链解析到原记录，拒绝循环/矛盾，不能跟随显示名或当前 source key。unknown/terminal/blocked 原 Run 都算已处理，仅有 armed 的先代记录不算。旧代可以更新既有 claim 的 outcome，但退休后不能新建此前不存在的 claim。完整范围/phantom 保护防止竞争；缺记录或索引不证明历史为空。GC/压缩必须保留准确原键、坐标、disposition 和原 Run/unknown 责任，以便继续排除。确实要重跑已处理坐标时，必须真实新建 Automation 或另行确认普通交互 Run，不能借修改本 Automation 的 definition/generation 绕过。

每个新受保护步骤仍应用当前授权、准确 Lease/activation/approval/time/budget/stop 及原请求恢复规则。subscription 已变或当前源暂无法重新证明，都不能替换 saved/planned/unknown 工作。公共 automation_state 显示 generation 和 lower bound 调度值；公共 run_state.origin 携带准确真实 claim key。完整 source proof、隐藏先代 claim、rule/history pins 和执行 custody 保持受保护。嵌套值无权时返回原不披露结果，不虚构空 schedule。

D6 Storage §7.2.1 拥有实际闭合 ScheduleContinuityWitness/1 与 ScheduleContinuityStep/1 生产者、已注册有限转换收件箱、原子检查点与失效及最后引用清理。continuityPins 使用这些准确分型 artifact 解码器或完整原分型链，本节 D10 业务谓词不变。此具体候选仍须独立联合接受和实施证据。

### 16.1 完整执行责任载荷

这些闭合保护值是 D6 ExecutionResponsibilityRecord/2 实际消费的 D10 载荷，保留原控制身份与事实，不建立另一账户、发生项存储或作者账本。数组使用完整规范键排序且唯一，对声明的执行域完整，包括空范围。一次转交每个数组最多 4096 项、规范元数据最多 16 MiB，载荷固定引用另行预留容量；超预算则暂停转交，不能丢记录，也不禁用无关普通源工作。

    CostLayerKey/1 = {
      kind:"run"|"lease"|"automation"|"workspace"|"grant"|"account",
      owner:ControlRef<K>/1, account:ControlRef<cost_account>/1,
      currency:CurrencyCode
    }

    CostLayerTotal/1 = {
      key:CostLayerKey/1, heldMicroUnits:Counter, spentMicroUnits:Counter,
      reservations:[ControlRef<reservation>/1]
    }

K 依次为 run、lease、automation、workspace_budget、grant 或 cost_account；账户层的 owner 等于 account。总量是 §10 完整预留与归属集合的已验证投影，不是可写余额。每份匹配预留，包括已结算历史，恰参与一次；成员未知不能当零。准确原层累计键在任何配置变化后都保留。

    D10ExternalSendRecord/1 = {
      binding:ExternalExecutionBinding/1,
      fenceToken:Token,
      outcome:
          {kind:"prepared"}
        | {kind:"started"}
        | {kind:"not_started", evidence:EvidenceTicket/1}
        | {kind:"response", bytes:FrozenEffectBytes/1}
        | {kind:"outcome_unknown"}
    }

栅栏 token 选择真实受保护发送栅栏记录，绑定准确 sendAttemptId、存储、持有者、意图及停止集合，不是调用方能力。prepared 已有耐久费用占用但尚未交付；started 在不可逆交付前耐久保存，崩溃后无法证明结果时保守成为 outcome_unknown，不能成为 not_started。not_started 要求原受信从未开始证据。response 是完整响应或效果证据，按不可变意图绑定的准确已接纳贡献合同解码；字节本身不证明成功、失败或最终账单。证据未知或不支持时保留未知及全部占用。协调在同一尝试下记录已验证响应；只有 D10 §17 的准确幂等或无效果资格成立后，才允许同一意图下另建重试。费用结算另有自身证据，不能从此结果推断。

    D10AuthorStepResponsibility/1 =
      {kind:"core_field_member", link:D10AuthorPreparationLink/1,
       decisionKey:D6.DecisionKey/2, protocolOwner:"D6",
       preparedRecordPin:D6.PinRef/2, recoveryPins:[D6.PinRef/2]}
    | {kind:"interactive", run:ControlRef<run>/1, stepId:Counter,
       decisionKey:D6.DecisionKey/2,
       authorRequest:
           {protocolOwner:"D3",request:<complete identity_operation_request wire12>}
         | {protocolOwner:"D6",request:D6.d6_commit_request/2},
       preparedFormat:"d7_prepared_action_binding3"|"d8_prepared_edit_binding2",
       preparedRecordPin:D6.PinRef/2, recoveryPins:[D6.PinRef/2]}

自动分支的 artifact 引用严格解码为 link.preparedBindingToken 选择的准确原 D7 PreparedActionBinding/3。交互分支的 preparedFormat 准确选择真实原 D7 PreparedActionBinding/3 或 D8 PreparedEditBinding/2；D8 要求 protocolOwner=D6，D7 则按原合同携带 D3 或 D6。完整内嵌请求、DecisionKey、主体、预览和引用必须一致。交互工作仍需真实受信用户确认及原 owner 门，不因此获得长期自动批准、新 ActionSpec 或提交入口。记录在交付或提交前原子关联原运行和步骤，不能通过另选请求重建。恢复引用包含真实存在的原 D3/D6 计划、主决议及同决议关联、安装 B/N 与来源和审计，各按原解码器处理。源引用不能冒充准备记录引用。P 仍独占当前决议及完整原回执或错误，不复制竞争的成功标记；保存、计划和未知先遵循原 owner。D9 生成的 D7 准备在同一原记录内保留真实 D9 构造输入，不另造作者请求。

    D10ExecutionClaims/1 = {
      recordPins:[D10ControlRecordPin/1],
      prepareBindings:[ControlPrepareBinding/2],
      leaseRuns:[LeaseRunUse/1],
      authorSteps:[D10AuthorStepResponsibility/1],
      subscriptions:[ScheduleSubscription/1],
      occurrenceRecords:[AutomationOccurrenceRecord/1],
      ranges:[D10ControlRange/1],
      continuityPins:[D6.PinRef/2]
    }

    D10MoneyResponsibility/1 = {
      reservations:[{
        binding:Binding<reservation>/1,
        value:CostReservation/1,
        attribution:CostBudgetAttribution/1,
        settlements:[CostSettlementDecision/1]
      }],
      layers:[CostLayerTotal/1],
      recordPins:[D10ControlRecordPin/1],
      ranges:[D10ControlRange/1],
      evidencePins:[D6.PinRef/2]
    }

    D10ExternalResponsibility/1 = {
      binding:Binding<external_effect>/1,
      intent:ExternalEffectIntent/1,
      state:ExternalEffectCurrentView/1,
      attempts:[D10ExternalSendRecord/1],
      evidencePins:[D6.PinRef/2]
    }

    D10StopResponsibility/1 = {
      binding:Binding<stop>/1, owner:StopOwner/1,
      latch:ExecutionStopLatch/1,
      receipt:Option<D10EmergencyStopReceipt/1>
    }

    D10ExecutionInventory/1 = {
      workspaceRef:D3.WorkspaceRef,
      storeIncarnation:Uuid,
      approvalUses:[ApprovalUse/1],
      claims:D10ExecutionClaims/1,
      moneyLineage:D10MoneyResponsibility/1,
      externalUnknowns:[D10ExternalResponsibility/1],
      stopState:[D10StopResponsibility/1],
      stopCapacity:StopCapacity/1
    }

清单包含全部活动或仍被引用的运行与配置、原准备（包括完整内嵌作者 A）、计次使用、已准入租约、订阅代际与连续性见证、已武装及已处理发生项、预留归属、外部已开始或未知尝试，以及停止结果和容量。防重复收费、认领或发送所需的终态证据仍须在清单中，或由准确原分型引用保留。externalUnknowns 也保留未决工作或防重所依赖的已知结果；字段名称不允许删除仍为依赖的已完成尝试。每个引用解析到准确原记录及版本，范围在同一保护存储屏障下证明完整性。配置图像包括重算归属必需的全部历史预算输入，不能用当前配置替代。共享部署账户总量在真实账户栅栏下包括其它执行域的预留；转交一个工作区不取得共享账户所有权，也不清零。此类部署责任仍在真实 owner，并保留连续引用，新执行恢复前仍须可达且合格。

只有旧执行持有者的准入、planning、发送和调度写者在一个真实存储屏障处停止后，才能生成清单。已进行的作者安装保留原屏障与恢复责任，不能为制造空清单而删除。新持有者执行要求 D6 验证完整原清单、真实隔离旧持有者、耐久保护转交，并保留 storeIncarnation 与控制引用。若新物理存储无法保留这些身份和保护连续性，接管只能不可用，不能创建空执行域。源文件复制或重建索引不能提供此证明。

停止容量计数与共享账户责任一样，仍位于其实际共享存储 owner。只转交一个工作区不能把全局计数复制到第二个活动存储。要么原权威安全存储仍是可达的唯一序列化 owner，要么完整存储转交隔离全部受影响写者并保留所有目标和锁预留。无法证明该边界就暂停接管，不能重置 issued/reserved 或只转交可见子集。

### 16.2 联合生产者验收场景

必须覆盖：r6 之后同键重放 r5；拒绝循环塞入生成的 M 而保留内嵌 A；缺少分型历史图像；隐藏控制源；竞争最后批准名额；逐字无操作只消费一次；原计划补充批准；文件安装后但封存前停止；第三状态恢复不退款；先缺外部确认、后同准备取得合法人工事件；共享账户多自动化限额；交互运行不虚构自动化；剩余次数为零仍恢复同运行；武装项晚唤醒；完整原子漏跑窗口；无关源推进；删除重建 ABA；规则改变后复原；观察缺口；完整及空转交范围；旧持有者发送与认领栅栏。文档检查不执行这些并发、存储或界面场景，仍须新的独立设计接受、后端测试及 D1 发布资格。
