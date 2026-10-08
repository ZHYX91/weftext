---
source_language: zh-CN
translation_of: D10-CONTROL.zh-CN.md
translation_status: synced
---

[简体中文](D10-CONTROL.zh-CN.md)

# D10 Control and Management Contract

## 0. A2 current single-owner routing and predecessor decoders

**Author candidate, not independently accepted/implemented.** The complete current D6-SCHEMAS §10/§10.1 exact mixed-version and direct types appear below, so no external patch reconstruction is needed. All original CONTROL §§1–16 business invariants survive; Image1/Dependencies2/Binding2/Schedule1/Link1/ApprovalUse1/Record2/Proof1 claiming to be current are historical source. Fresh production uses the exact successor below; genuine historical records retain original bytes, decoder and recovery, without LWW, second store or CAS.

### 0.1 Exact D6 mixed-version current schemas

The following are current outer-holder schemas. Every versioned carrier strict-decodes the exact tagged inner type and retains its original canonical bytes/pins. Unknown tags fail closed. A carrier is only a decoder discriminator: it grants no authority, creates no decision/CAS, and never migrates the inner record.

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

For a current unseen external-consent preparation, confirmationRequirement is the same closed ExternalConfirmationRequirement/1 used by the fixed-parent attended-confirmation protocol and is frozen inside ControlPrepareBinding/3 before the original commit request is delivered. The mutable confirmation fact remains exclusively in ExternalConfirmationRecord/1 and is not inserted into Binding3 or ControlDependencies/3. Current confirmation eligibility therefore consumes Binding3 + Dependencies3 while preserving the original trusted-event, principal, time-window, immutable-intent/preview, disclosure, approval_unavailable/preflight, saved-result replay, and result-redaction rules. A proven historical ControlPrepareBinding/2 retains its exact requirement, Dependencies2, canonical intent bytes, confirmation association, and recovery decoder.

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

Image2 is closed to automation/run. Every other record kind keeps exact Image1. Pin1 keeps its historical D10-Control-Record/1 prefix and decoder. Pin2 contains exactly UTF8 D10-Control-Record/2, NUL, and D3-CJ/3(Image2); its PinRef/2 is artifact with the original recovery or approval_money retention and exact prefixed byteLength/SHA-256. A schema-tag/payload-domain mismatch fails. Pin2 never repins Image1.

For D10VersionedControlRecordPin/1 arrays let I=carrier.value.image. Sort by D3-CJ/3(I.binding.ref), binding.revision numeric, usageRevision none before some, numeric usageRevision when present, then D3-CJ/3(I). The logical record-cut identity is I.binding.ref + I.binding.revision + I.usageRevision. A second byte-equal image at that identity is a duplicate and is rejected; different bytes are integrity_conflict. Version does not split the identity.

Range kind rank is records=0, cost_lineage=1, occurrences=2. The cross-version logical identity is respectively scope plus the complete sorted kinds, CostLayerKey, or Automation Ref. Sort mixed ranges by rank then canonical identity bytes and allow one value per identity. An occurrences range sorts its records by AutomationOccurrenceKey; one key cannot occur in both V1 and V2.

D10ControlEffectPlan/2.changes sorts uniquely by the complete canonical after.value.binding.ref. When before is present, before.binding.ref equals after.binding.ref and recordPins contains exactly the versioned pin for the actual stored before schema. Image1 before plus Pin2 is invalid. Image1 Automation+Subscription1 to Image2+Subscription2 is a valid current configure transition. A normal configure may continue the old subscription at the same generation when the scheduling owner continue rule succeeds; mixed support never forces replacement or background migration.

ControlDependencies/3 arrays keep their fixed owner order: configBindings and usageBindings by full Ref, authorizationGenerations by token, stopRefs by full Ref, recordPins/ranges by the mixed orders above. Each config binding has its exact matching image/pin, each usage binding has the same usageRevision image, and each stopRef has exact stop Image1/Pin1. All current bindings, usage revisions, range fences, pins, and Workspace evidence are captured from one actual Authority Store barrier. Combining barrier-A V1 evidence with barrier-B V2 evidence is not a complete dependency snapshot. Missing required historical bytes/decoder/pin is state_unavailable after disclosure; contradictory same-cut evidence is integrity_conflict.

D10ExecutionClaims/2 canonical identities are: recordPins as above; prepareBindings by inner StableControlKey across Binding1/2/3; leaseRuns by full Run ControlRef; authorSteps by run Ref plus stepId across versions; subscriptions by Automation Ref plus generation across versions; occurrenceRecords by AutomationOccurrenceKey; ranges as above; continuityPins by pinToken. Each identity is unique across versions. In particular Binding1(K) and Binding3(K) cannot coexist even if canonicalIntentBytes and originalCommitRequest are byte-equal. Binding1 retains its exact six-member historical shape; no kind, version, confirmationRequirement, /2-/3 dependency field, LWW, or repinning is introduced.

D10MoneyResponsibility/2 orders reservations uniquely by complete Binding<reservation>/1 canonical bytes, layers uniquely by CostLayerKey canonical bytes, recordPins/ranges by the mixed rules, and evidencePins uniquely by pinToken. D10ExecutionInventory/2 orders approvalUses uniquely by DecisionKey canonical bytes across versions, externalUnknowns uniquely by binding.ref canonical bytes, and stopState uniquely by binding.ref canonical bytes. Every pending/unknown/dedup/stop responsibility and every still-referenced completed external attempt is retained. The inventory is captured only after admission, planning, send, and schedule writers stop at one real store barrier.

The exact Inventory2 artifact payload is UTF8 D6-Execution-Inventory/2, NUL, D3-CJ/3(D10ExecutionInventory/2). The PinRef/2 is artifact/recovery and its byteLength/SHA-256 covers the complete prefixed bytes. Inventory1 keeps D6-Execution-Inventory/1 and is never repinned as /2.

Record3.workspaceRef equals Inventory2.workspaceRef. Record3.approvalUses, claims, moneyLineage, externalUnknowns, and stopState are each byte-equal to the corresponding Inventory2 member. Proof2.inventoryPin selects that exact Inventory2 and Inventory2.storeIncarnation equals Proof2.storeIncarnation. The protected birth/barrier/fence token mapping proves the same actual store and capture barrier; there is no separate caller-supplied store-incarnation proof object.

Inventory2.stopCapacity is the exact fixed-parent StopCapacity/1 at the same authoritative safety-store barrier. StopCapacity/1 remains only issued/reserved and is not duplicated in Record3. A Workspace-only handoff cannot copy the shared safety counters to a second active store. A complete store handoff must fence every affected writer and preserve all target/latch reservations and safety capacity; inability to prove that boundary makes takeover unavailable.

Record2/Proof1/Inventory1 remain historical and keep their exact decoder/domain. Record3 is emitted only for an actual responsibility mutation, checkpoint, or custody handoff; it preserves executionDomainId and never background-migrates an unchanged holder.

### 0.2 Exact current schedule/author-step schemas

These are the exact current inner values referenced by the §10 mixed wrappers; they are not free objects invented by the wrappers.

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

The current continuity artifact encodings are exact and version-separated:

```text
Witness2ArtifactBytes =
  UTF8("D6-Schedule-Continuity/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityWitness/2)

Step2ArtifactBytes =
  UTF8("D6-Schedule-Step/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityStep/2)

Invalidation2ArtifactBytes =
  UTF8("D6-Schedule-Invalidation/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityInvalidation/2)
```

Each corresponding PinRef/2 has payloadKind=artifact and retentionClass=recovery. Its byteLength and SHA-256 cover the complete domain prefix, the one NUL byte, and the complete canonical object bytes above. A digest never authenticates continuity by itself: the protected producer/subscription provenance, producerEpoch, exact generation, retained transition chain, and original authorization/disclosure gates remain required.

Decoder dispatch is closed by the authenticated artifact domain before inner decoding. D6-Schedule-Continuity/1 accepts only historical ScheduleContinuityWitness/1; D6-Schedule-Step/1 accepts only historical ScheduleContinuityStep/1; D6-Schedule-Invalidation/1 accepts only historical ScheduleContinuityInvalidation/1. D6-Schedule-Continuity/2 accepts only ScheduleContinuityWitness/2; D6-Schedule-Step/2 accepts only ScheduleContinuityStep/2; D6-Schedule-Invalidation/2 accepts only ScheduleContinuityInvalidation/2. Unknown domains, a known domain paired with the wrong kind/version, alternate prefixes, missing NUL, or non-canonical object bytes fail closed through the original disclosure/error boundary. There is no fallback decoder, widening of a /1 decoder, repinning, or re-encoding of historical bytes.

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

For a fresh current core_field_member step, D10AuthorPreparationLink/2.preparedBindingToken must select the exact PAB4 whose original request equals link.request. Core saves Link2, that complete PAB4, and the required preview/effect/recovery pins atomically before returning the prepared step or permitting submission. ApprovalUse/2.preparedBindingToken selects that same PAB4. Its preview digest is
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3));
each EffectBytes/3 slot is projected only as {encoding,byteLength,payloadDigest}, never discovered by recursively guessing member names. Fresh current automatic author qualification uses DependencyProof/3, including document_format whenever the managed Document is semantically parsed, and constructs ApprovalUse/2 only after complete EffectManifest/3 / EffectBytes/3 / MutationFootprint validation. Historical Link1/PAB3/ApprovalUse1 saved or planned associations keep their original decoder, bytes, pins, request and OperationId.

**Closed responsibility-arm dispatch:** `core_field_member` requires a *real* Automation binding, immutable definition revision/task digest, Link2-selected D7 PAB4, qualified Standing Approval and ApprovalUse2; `interactive` has no Automation/Link2/ApprovalUse2 and instead binds the genuine D3IdentityOperationRequest/13 or d6_commit_request/2 to the saved D7 PAB4 or D8 PreparedEditBinding/3 selected by its `preparedFormat`. For each interactive step Core retains the complete original owner prepared/effects/preview evidence, protected preparedRecordPin and recoveryPins, verifies exact request/OperationId/DecisionKey association, presents all pages and required EffectBytes/3 under the original D7/D8 preview contract, and requires a new explicit authenticated natural-user confirmation of those exact semantics before owner submit; an Agent suggestion, Start event, tool flag or standing approval cannot substitute. D3 remains the identity submit owner; D6 remains the only D6 author ledger/final P. Both arms share the same admitted Run/LeaseRunUse, Stop latch, Policy/authority/fence/custody, trusted clock, activation, actual write/read grants and budgets. Saved/planned/unknown/cold recovery dispatches the *original stored arm and actual prepared version*, original request/DecisionKey/OperationId and pins; same-Run protected continuation does not consume maxRuns twice and no second author success record is created. Wrong arm, invented Automation/Link2, missing confirmation, mismatched preparedFormat/pins/preview digest, cross-store association or revocation/Stop/budget failure rejects using the original ordered owner errors, without falling back to the automatic narrow profile.

For a fresh ScheduleSubscription/2 registration, the current D6 Storage producer creates ScheduleContinuityWitness/2 only in the same configuration transaction that passes the selected source/Field/Registry/current scheduling gates, reserves finite retention, and attaches the actual continuously maintained Core source/control transition producer. initial and checkpoint are the subscription's exact ScheduleRecurrenceEvidence/2, revision=1, consumedTransition=0, and producerEpoch is fresh. The retained witness pin is exactly Witness2ArtifactBytes above. Positive current transitions use ScheduleContinuityStep/2 with DependencyProof/3 and the actual ChangeRecord/1, InstallationNotice/3, and ContentCompletionProof/4 pins for new current portable transitions; each retained current step pin is exactly Step2ArtifactBytes above. Historical transitions inside retained history keep their original exact /1 artifact domains and decoders. P-only relevant control/rule transitions remain captured from their actual protected before/after state.

Fold, compaction, receiver admission, D10 continuityPins consumption, and recovery all dispatch each protected schedule-continuity artifact by its exact domain before decoding the inner object. A current Witness2/Step2 is never accepted under a /1 domain, and a historical Witness1/Step1 is never accepted under a /2 domain. continuityPins are token-sorted/unique exact PinRef/2 values and may retain a version-mixed original typed chain when real history crosses the explicit same-generation bridge; each element keeps its own exact bytes/domain/decoder. The alternative full retained-chain path likewise keeps each original typed artifact and source/control evidence rather than normalizing the chain to one version. Missing or unknown typed evidence produces the original gap/unavailable behavior after the ordinary authorization/disclosure checks; a proved domain/object mismatch is never repaired from equal digest/current state.

When no valid current after evidence can exist, the current producer emits ScheduleContinuityInvalidation/2 instead of fabricating ScheduleRecurrenceEvidence/2. binding_changed requires complete trusted evidence of a selected-business discontinuity; unavailable/unknown after, missing history, unknown decoder, observer/producer gap, or inability to retain a required transition is gap. Invalidation compares the same current witness/registration, atomically advances the next checked transition/revision, keeps the last valid checkpoint, and is permanently non-resetting for that generation. Its artifact pin is exactly Invalidation2ArtifactBytes above. The fixed-parent inbox/capacity/final-counter reservation, authorization, compaction and unrelated-source-availability rules remain unchanged.

A current schedule proof that semantically parses a managed Document must include both source and document_format dependencies. If source/profile bytes are unchanged but format-proof continuity has a gap, the result is gap; a real binding transition is binding_changed even when the final recurrence/range value happens to compare equal. An existing Subscription1 remains a historical retention owner with Witness1/Step1/Invalidation1 and their exact /1 artifact domains. It may become a same-generation Subscription2 only through explicit continue plus complete retained history proving no intervening format/rule/business discontinuity and establishing the current Evidence2/Proof3 cut; otherwise replace is required. The bridge preserves every old pin/producer association and starts using /2 domains only for newly produced Witness2/Step2/Invalidation2 artifacts. It never repins or re-encodes a version-1 witness/step/invalidation as version 2 and never resets an invalid generation.



---


revision: D10-FA-r01-2026-10-02; status: coordinated author candidate, not accepted, activated, or implemented. The last complete historical R08 review of C8=`d99f053b9386c9c9e1664251fdec9f00e33fac2c` against S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` returned REVISE (P0=0, P1=3, P2=8). All eleven historical final dispositions remain OPEN. Named repairs have limited independent reviews; actual cross-owner integration and fresh global acceptance remain incomplete. D10-REVIEW separates those evidence scopes.

## 0.3 Canonical new-versus-recorded dispatch

This is one closed D6 producer, not a second D10 type authority. Fresh ControlPrepareBinding/3 binds ControlDependencies/3, D10WorkspaceReadDependencies/2, D10ControlInput/2 and D10ControlEffectPlan/2 at the same real D6 planning/seal barrier. For an automation or Run control record the current image/pin is Image2/Pin2; the other seventeen control K kinds *still use the valid Image1/Pin1* through the explicit mixed tagged carrier, and a genuine original Image1 automation/run stays historical. A ControlPrepareBinding1/2 or Dependencies1/2 from a real recorded decision remains byte-exact under its original tag and record/custody chain. A newly prepared **automatic core_field_member** D10 author step uses the real Automation's D7 PAB4, EffectManifest3/EffectBytes3, D10AuthorPreparationLink2 and ApprovalUse2 with its original D6 request; a separately prepared **interactive** step instead uses the original D3 Request13 or D6 request2 and owner-qualified D7 PAB4 or D8 EditBinding3 with genuine full-preview human confirmation, never Link2/Standing Approval; a genuine historical PAB3/Link1/ApprovalUse1 uses its own original preview digest and decoder, not a new preview hash. Link2/ApprovalUse2 is required **only** for the actual Automation `core_field_member` arm; an independent `interactive` step uses real D3/D6 author request and D7 PAB4 or D8 PreparedEditBinding3 with explicit confirmation, never Link2/Standing Approval. The original control contract §7 public result/error shapes and nineteen complete public current view categories remain current; the original §8.1/§8.2 illustrative Image1/Plan1/ApprovalUse1 concrete record examples remain valid only in their actual versioned history where superseded above. Original protected stop/money/unknown and unspent capacity cannot be reset through a sync, I rebuild, receipt read, retry, equal digest or display label. ScheduleSubscription2, Witness/Step/Invalidation2, Inventory2/Record3 and mixed claims/record pins obey only the complete exact owner schema in §0; there is no invented StoreIncarnationProof request or second durable ledger.

## 1. Authority, scope, and error boundary

Core remains the sole author-transaction authority. Workspace author commits, saved decisions, receipts, and planned recovery remain owned by D6; this contract never stores a second author-success ledger. Deployment trust, packages, secrets, external accounts, pricing, and resource-use grants belong to the D10 host control domain and grant no Workspace content authority.

Public capabilities still pass the real D1 contractMajor, surface, release, policy, principal, component/configuration, reachability/version-combination, and health gates before entering this contract. Design acceptance, coordinated design activation, or the existence of a control record is not runtime available evidence.


The D10 management/control error object is:

    D10ControlError/1 {
      kind:"d10_control_error", wireVersion:1,
      code:
        "invalid_request" | "not_visible" |
        "authority_unavailable" | "integrity_conflict" |
        "control_conflict" | "stale_revision" |
        "state_unavailable" | "budget_exceeded"
    }

The separate D10 Run/step error object is:

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

`D10ControlError/1` is used only by `d10_control_prepare`, `d10_host_control_commit`, `d10_control_result`, `d10_control_read`, `d10_interactive_run_start`, `d10_interactive_run_result`, `d10_secret_stage`, `d10_emergency_stop`, and `d10_emergency_stop_result`. `D10RunStepError/1` is used only while a D10-owned Run/step has not entered another protocol owner: Automation scanning/arming/claim before Run creation, Run admission, ContextBundle/model/tool/connector execution, pre-D6 approval/delegation checks, and external-effect execution/recovery.

Once a request enters D3, D6, D7, D8, or D9, that owner returns its original closed error/envelope unchanged. D10 never wraps D6 `not_visible`, `approval_unavailable`, `execution_stopped`, or `transaction_aborted`; never wraps D7 action/effects errors; and never renames D3/D8/D9 errors. A D10 diagnostic UI may perform a separately authorized `d10_control_read` after the owner error, but that read cannot alter the formal result.

Management/control ordering is fixed: closed decode and D1 capability gates → current principal/scope/audience/object-observation authority → authority/fence/custody → stable-key conflict or exact ControlRef lookup → protected-record integrity/continuity → current-revision/dependency/budget checks only when applicable. Absent object, wrong scope/audience, and current lack of object-observation authority are all `not_visible`. Unprovable authority/fence/custody is `authority_unavailable`; a proven protected-record/evidence contradiction is `integrity_conflict`; same stable key with different complete input is `control_conflict`; an expected configuration revision mismatch on a currently visible object is `stale_revision`; temporarily unprovable trusted time, external evidence, or required continuity is `state_unavailable`.

Run/step ordering remains Candidate §21: closed decode/version → D1 static capability/surface/release → current principal/control visibility → delegation/data observation → deployment binding → exact input/approval → budget/audit → execution. Stage ordering precedes specialization. In particular `binding_changed` belongs only to the Run/step domain; management CAS uses `stale_revision`, and pre-D6 `approval_required|approval_expired` never replace the D6-owned in-race `approval_unavailable/preflight`.

## 2. Shared closed types

The following are normative D10 generation-one contracts. Every object rejects unknown members, duplicate members, and illegal null. Counter reuses D6 exact 0..2^63-1 non-Boolean integer semantics; Uuid uses canonical lowercase RFC 4122 text. Every array is bounded, ordered, and unique as specified by its containing type.

    Version/1 = {
      major:Counter, minor:Counter, patch:Counter
    }

    VersionRange/1 = {
      minimum:Version/1,
      maximumExclusive:Version/1
    }

VersionRange requires minimum < maximumExclusive. There is no latest, wildcard, or ambient package version.

    Scope/1 =
      {kind:"workspace", workspaceRef:D3.WorkspaceRef}
    | {kind:"deployment", storeIncarnation:Uuid}

ControlRecordKind/1 is closed to:
The controlled record-kind set is `automation, lease, approval, planned_approval, external_approval, run, workspace_budget, activation, deployment_policy, trust, package, external_account, secret, grant, cost_account, pricing, reservation, external_effect, stop`.

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

Control IDs and incarnations are never reused. Retire, archive, revoke, close, or removal from a display surface never permits a later object to use the same ID. Re-creating the same display name receives a fresh ID.


`Binding<K>/1.revision` is the sole configuration/lifecycle CAS revision for the addressed control record. When an existing owned record already has a named revision, the names are identical rather than parallel authorities: `leaseRevision == Binding<lease>.revision`, `approvalRevision == Binding<approval|planned_approval|external_approval>.revision`, `grantRevision == Binding<grant>.revision`, `CostReservation.revision == Binding<reservation>.revision`, and `DeploymentControlPolicy.revision == Binding<deployment_policy>.revision`. `ActivationBinding.activationGeneration == Binding<activation>.revision`. Automation intentionally has a distinct `definitionRevision`: enable/disable/archive advances the automation control revision without changing the immutable semantic definition revision.

Independent cumulative-use CAS uses `usageRevision` only for `lease`, `approval`, `grant`, and `cost_account`. A normal Run admission increments Lease usage without changing `leaseRevision`, so the already-admitted Run's exact Lease binding does not become stale merely because another Run consumed capacity. Approval reserve/consume/released_terminal similarly advances approval usage without changing `approvalRevision`. Grant already owns `usageRevision` in §6; cost-account held/spent usage receives the same independent revision in §7 below. Configuration/lifecycle changes never reset cumulative usage, and usage changes never revive a revoked/retired/closed configuration.

    Money/1 = {
      currency:CurrencyCode,
      microUnits:Counter
    }

CurrencyCode is exact three-character ASCII A-Z. D10 performs no implicit foreign-exchange conversion.

Other primitive scalars reuse existing closed JSON semantics. `Token` is the non-empty opaque token from D6 §1; `Text` is a Unicode-scalar string; `Bytes` is a byte sequence bounded by the applicable entrypoint budget; `Boolean` accepts JSON true/false only; `Sha256` is `sha256:` plus 64 lowercase hex digits; `HostPrincipal` is a `Token` produced by trusted host authentication mapping. The current actor is never caller-supplied, but an H-authorized management operation may name another `HostPrincipal` in a policy, reconciler, or ResourceUseGrant **target-principal field**; that configures the authorized target and does not impersonate that principal as the caller. `Ed25519PublicKey` and `Ed25519Signature` appear only inside accepted package/trust adapters whose admitted profile fixes their encoding; an ordinary control caller cannot self-assert verification.

    HostOrWorkspacePrincipal/1 =
        {kind:"workspace",
         workspaceRef:D3.WorkspaceRef,
         principal:Token}
      | {kind:"deployment",
         storeIncarnation:Uuid,
         principal:HostPrincipal}

`HostOrWorkspacePrincipal/1` is a D10 closed projection of the trusted authenticated actor at the successful safety-transition cut. The workspace arm pairs the current D6-authenticated Workspace principal with its exact WorkspaceRef; the deployment arm pairs the H-authenticated HostPrincipal with the actual D10 storeIncarnation. Neither arm is caller-supplied, an author EntityRef, or a capability token.

Controlled ASCII-token grammar:

```text
LowerCamelAscii = [a-z][A-Za-z0-9]{0,62}
LowerKebabAscii = [a-z][a-z0-9]*(?:-[a-z0-9]+)*
CanonicalInteger = "-"? ("0" | [1-9][0-9]*)
CanonicalDecimal = CanonicalInteger ("." [0-9]*[1-9])?
```

`LowerKebabAscii` is 1..63 bytes. `CanonicalInteger` and `CanonicalDecimal` must also satisfy the explicit bounds of the applicable ToolType. decimal forbids trailing zero, empty fractional part, exponent notation, and negative zero.

    ScheduleHorizon/1 = {
      start:D4.zoned_instant,
      endExclusive:D4.zoned_instant
    }

ScheduleHorizon is not a new D4 type. Both members use the exact temporal semantics of D4 zoned_instant and the same exact UTC comparator proves start < endExclusive. This document does not use the nonexistent name “D4 bounded-instant-range”.

## 3. Tool Value Profile

ToolValueProfile/1 is owned by the D10 Tool Adapter. It is not an alias for D7 TypeSpec and is not arbitrary JSON Schema. It provides only a bounded value algebra for external-tool arguments and results.

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

`maximumUtf8Bytes` is 1..8388608; an object has at most 64 `members`; a union has 2..8 `arms`; list `maximum` is 1..4096 with `minimum <= maximum`; complete type depth is at most 16 and canonical type bytes at most 65536. Member names and arm tags are sorted by UTF-8 bytes and unique. integer/decimal use canonical base-10 strings; binary float, NaN, Infinity, and negative zero are forbidden.

ToolValue/1 exact wire is:

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

ToolValue/1 matches the ToolType/1 bound at the call site recursively. object members are name-sorted and unique and contain only declared members; list length satisfies the bounds; union tag matches one declared arm. int64 also lies within signed 64-bit range.

Missing required members, extras, wrong arms, bounds violations, or budget overflow are invalid_request. ToolValue contains no EntityRef, Locator, SecretRef, file-path capability, ActionEvidence, plan/result token, open map, or executable value.

## 4. Package, Contribution, and dependencies

PackageId/1 and D4 SemanticNamespaceId are distinct types even when their strings match. PackageId uses 1..127 ASCII bytes of dot-separated lower-kebab segments. LocalContributionId, ExtensionPointId, and LocalOperationId each use one 1..63 ASCII-byte lower-kebab segment. All comparisons are exact ASCII.

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


A `view` Contribution descriptor asset has a closed root dispatch. It is either the original D7 `ViewSpec` owned by D7, or the following D10 carrier for the D7-owned pure-data SearchContribution:

    D10D7SearchDescriptor/1 = {
      kind:"d10_d7_search_descriptor",
      wireVersion:1,
      search:<exact S D7 Query Algebra §6 SearchContribution>
    }

This does not make `SearchContribution` a ViewSpec and does not create a `search` ContributionKind. One D10 Contribution carries exactly one D7 SearchContribution. The D7 object remains exact `{contributionId,version,fieldId,textPath,role}` with its original D7 owner, D4-style semantic contribution ID grammar, positive Counter version, 0..8 static text path, and `name|alias|content` role.

R08 also freezes one named, versioned first-party Core author adapter. It is not a generic Tool callback and does not make ToolValue a D7 value alias:

    CoreFieldMemberAdapterDescriptor/1 = {
      kind:"d10_core_field_member_adapter",
      wireVersion:1,
      actionKind:"set_field_member"
    }

The only generation-one adapter identity is packageId `weftext.automation`, package-local contributionId `set-field-member`, Contribution kind `action`, and contractVersion `{major:1,minor:0,patch:0}`. packageVersion and descriptorDigest still use ordinary accepted first-party PackageManifest/ContributionBinding rules. This package is not a Bundled Module and creates no D4 namespace or author fact.

    SingleFieldMemberPath/1 =
      [D4.ObjectMemberSpec.name]   // exact length 1..7

    FieldMemberTask/1 = {
      ownerNodeRef:D3.NodeRef,
      fieldId:D4.FieldId,
      selection:"require_exactly_one_entry",
      memberPath:SingleFieldMemberPath/1,
      value:D7.TypedLiteral
    }

`FieldMemberTask.value` is the original D7 Action literal, not the approval scalar. It is either one allowed scalar `SingleFieldMemberScalarType/1`, or exactly one D7 Optional wrapper whose item is that scalar and whose value state is `some`. `optional.none`, Ref/Locator/control tokens, objects, lists, sets, unions, nested Optional, and ToolValue text promoted into Ref/FieldId are rejected. An optional D4 member therefore keeps the original D7 Optional bridge while `SingleFieldMemberRule.memberType` compares the present underlying scalar.

    D10AuthorPreparationLink/1 = {
      run:ControlRef<run>/1,
      stepId:Counter,
      automation:Binding<automation>/1,
      definitionRevision:Counter,
      taskDigest:Sha256,
      preparedBindingToken:Token,
      request:D6.d6_commit_request
    }

This is a protected Core recovery link, not a public request and not a second author decision. `preparedBindingToken` is the exact token of the original D7 PreparedActionBinding/3 and `request` is its original D6 request. Core saves this link, the original PreparedActionBinding/3, and required pins atomically before returning the prepared author step or allowing submission.

ContributionKind is closed to:
The Contribution-kind closed set is `module, schema, view, action, template, preset, pack, tool, model, connector, importer, exporter, conversion, renderer, localization`.

This set covers the Mandatory Intake-selected module, Profile/schema, View, Action, Template, Preset, Pack, Connector/Adapter, D9 import/export/conversion, and D10 tool/model/connector contribution families. `runtime` is not an independent `ContributionKind`; it denotes D10 runtime infrastructure used by those executable Contributions. The kind list does not claim that every category is product-implemented.

    ContributionDependency/1 = {
      role:"primary_parent" | "additional",
      parent:{
        packageId:PackageId,
        contributionId:LocalContributionId,
        extensionPointId:ExtensionPointId
      },
      requiredContractRange:VersionRange/1
    }

A dependency belongs to one concrete dependent Contribution; there is no ambient package-level dependency. A pack Contribution has exactly one primary_parent. Other Contributions have at most one primary_parent and 0..32 additional dependencies. The parent tuple resolves to one exact activated Contribution and extension point. The resolved parent contract version and dependency result enter the Capability Catalog digest and never follow latest.

An unavailable connector Contribution makes only that Contribution inactive. A template, schema, pack, or view in the same package is evaluated from its own dependencies and capabilities. Definition history, Contribution activation, and UI visibility remain three separate axes.

Each kind’s descriptor is validated by its existing semantic owner: schema references existing D4 Registry namespace/Facet bindings; view/action consume D7 closed descriptors; template/preset/importer/exporter/conversion consume D9 contracts; D10 owns tool/model/connector descriptors and their runtime infrastructure; module only organizes product contributions and owns no author facts.


For `D10D7SearchDescriptor/1`, activation additionally proves all of the following before it may enter the current Capability Catalog:

1. `descriptorAssetId` resolves to exactly one `PackageAsset/1`; immutable bytes have the declared `byteLength`, SHA-256 equals that asset's `digest`, the strict root decoder is `d10_d7_search_descriptor`, and the active `ContributionBinding.descriptorDigest` is that same digest.
2. D10 PackageId/packageVersion, D10 package-local `Contribution.contributionId`, D10 `contractVersion`, D7 `SearchContribution.contributionId`, and D7 positive Counter `version` are five distinct identity/version domains. Equal spellings or numbers create no mapping. Changing any D7 search member/version changes the Catalog digest even when D10 contractVersion is unchanged.
3. The namespace portion of the D7 SearchContribution ID is authorized by the current D4 owner tuple plus D10 trust/NamespaceClaim proof. Independently, `fieldId` resolves under the exact current `RegistryBinding/1`, its namespace owner is verified, the Field is current-available, and the complete alias-expanded `textPath` terminates in D7 text or Optional<text>. No rule requires the SearchContribution ID namespace to equal the Field namespace.
4. In one current Catalog, D7 SearchContribution IDs are unique. All active `view` Contributions whose descriptor root is `d10_d7_search_descriptor` form the complete SearchContribution set, sorted by D7 contributionId canonical bytes. Omission, duplicate ID, wrong owner, wrong digest, malformed path, or unavailable required Field makes the successor activation fail closed; the previous ActivationBinding remains current.
5. `ActivationBinding.activationGeneration`, `capabilityCatalogDigest`, and the exact D4 `registryBinding` jointly bind that complete set. Explicit D7 search selection binds the selected `(contributionId,version)` rows; D7 `all` binds the complete set at that generation. A successor addition/removal/version/descriptor/owner/Registry change invalidates old search dependencies/results rather than silently changing their fields.
6. The carrier is pure data. It grants no `field_read`, network, secret, script, index-private payload, author write, or alias-source authority. Query execution remains the D7 `scan → explicit read → CEL match/rank → sort/project` pipeline under current D6 authorization. Missing/unavailable selected contributions follow original D7 unavailability; they are never silently skipped and never become an empty successful search.

The positive first-party construction is a valid `weftext.people` package `view` carrier whose accepted descriptor contains D7 SearchContribution `people/search-name`, version `1`, `fieldId:"people/name"`, `textPath:["text"]`, `role:"name"`. Activation succeeds only with the reserved D4 tuple `people→(first_party,weftext.people)`, matching descriptor asset digest, current Registry proof, and complete Catalog construction. A same-named third-party package, wrong digest, or runtime-only discovery remains inactive/pending.

## 5. Unique package mapping for four first-party modules

This section is a new D10 candidate mapping. It does not claim that D1 already defined package/module IDs, code paths, or locale resources. D4-frozen semantic namespaces, owner tuples, FacetIds, and semanticMajor remain unchanged.

| Product | PackageId | module contribution | schema contribution | D4 semantic owner / Facet | extension points |
| --- | --- | --- | --- | --- | --- |
| Calendar / 日历 | `weftext.calendar` | `module` | `schema` | `calendar` → `first_party,weftext.calendar`; `calendar/period-note`, `calendar/range-note`, `calendar/event`, `semanticMajor=1` | `calendar-system`, `holiday-schedule` |
| Library / 文献库 | `weftext.library` | `module` | `schema` | `library` → `first_party,weftext.library`; `library/work`, `semanticMajor=1` | `none` |
| People / 人物 | `weftext.people` | `module` | `schema` | `people` → `first_party,weftext.people`; `people/person`, `semanticMajor=1` | `none` |
| Organizations / 组织 | `weftext.organizations` | `module` | `schema` | `organizations` → `first_party,weftext.organizations`; `organizations/organization`, `semanticMajor=1` | `schema-pack` |

Each module descriptor contains only the formal Chinese/English product names, the same-package schema-contribution reference, and the extension points above. A schema descriptor lists only exact SemanticNamespaceId, D4 namespace ownerId, and FacetId/semanticMajor; it does not copy FieldDefinition or FacetSchema bytes.

PackageId, Contribution contractVersion, and D4 semanticMajor are three independent version domains. A package update cannot change the D4 semantic digest under the same FacetId/semanticMajor.

Candidate code conventions are CalendarModuleContribution, LibraryModuleContribution, PeopleModuleContribution, and OrganizationsModuleContribution; candidate variables are calendar_module, library_module, people_module, and organizations_module. Candidate code namespace d10::bundled::<domain> and candidate locale keys module.calendar.title, module.library.title, module.people.title, and module.organizations.title are design mappings with no current implementation evidence. There is no independent CLI verb; generic management consumes the complete package/contribution reference.

A D4 ownerId is never automatically package-ownership proof. The table creates an explicit mapping only. Third-party signature, install order, display name, or an identical string cannot acquire first-party package/namespace ownership.

## 6. Principal, management domains, and ResourceUseGrant

Requests never carry principal, owner, or an authorized Boolean. Current identity only comes from trusted host/D10 authentication mapping.

Workspace self-service qualification S is an explicit allow of current coordinated D6 Policy/3 capability d10_control_self for the current principal at workspace scope. It only allows management of that principal’s own finite D10 control records and grants no author read/write, policy_admin, registry_admin, deployment-account management, or secret plaintext. Actual author operations still require their original D6/D7 permissions.

Workspace-management qualification W is policy_admin at workspace scope in the current Workspace. Registry activation additionally requires registry_admin. W may stop, revoke, archive, and manage Workspace budgets, but cannot impersonate another principal to create or enlarge that principal’s Lease/Approval.

Deployment-management qualification H is owned by DeploymentControlPolicy/1:

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

Initial H comes only from explicit trusted-local-host initialization or Server deployment-operator configuration. First HTTP request, Workspace ownership, allocate_workspace, or administer_issuer does not imply H. A reconciler may only perform evidence-driven settlement on listed cost accounts.

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

`ResourceUseGrantSpec/1` is the create/update input. `grantId`, both revisions, state, and usage are produced only by Core from the existing record and transaction result.

ResourcePermission/1 has exactly four variants:

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

One permission kind is present per grant. secret does not imply egress, egress does not imply mutation, and cost does not imply account administration. cost requires perAttemptMicroUnits <= totalMicroUnits; egress requires perCallBytes <= totalBytes. Executable limits are finite positive values; cost amounts may be zero.

ResourceUsage/1 matches the permission kind:
cost={spentMicroUnits,heldMicroUnits,attemptsStarted};
secret={usesStarted};
egress={callsStarted,bytesSent,bytesHeld};
external_effect={attemptsStarted}.
These counters plus outstanding attempt reservations are admission dependencies.

Renewing, narrowing, or changing grantRevision under the same grantId never clears usage, held, spent, or attemptsStarted; usageRevision advances independently. A reduced limit cannot be below already consumed plus held usage. Changing grantee, resource kind, cost account, external account, or currency requires a new grantId. A new grant never erases old attempt, old grant, or actual-account liabilities, and every grant still competes for the same actual account ceiling.

## 7. Stable prepare, closed body, and public entrypoints

The ordinary control entrypoint is:

    D10ControlPrepare/1 = {
      kind:"d10_control_prepare",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      body:ControlBody/1
    }

ControlBody/1 has exactly seven variants:

1. automation_configure:
   automation:Target<automation>;
   lease:Target<lease>;
   approval:Option<Target<approval>>;
   definition:AutomationSpec/1;
   scheduleUpdate:AutomationScheduleUpdate/1;
   delegation:LeaseSpec/1;
   standing:Option<StandingApprovalSpec/1>.
   approval and standing are both none or both some. This fixed bundle is the only combined operation for creating/rebinding Automation, Lease, and Standing Approval and is not a generic batch/DAG.

2. consent:
   target:Target<planned_approval|external_approval>;
   consent:ConsentSpec/1.

3. state:
   target:Binding<K>;
   action:StateAction.
   Closed applicability:
   automation→enable|disable|archive;
   lease/approval→revoke|archive;
   grant→revoke|archive, where archive is the existing `retired` state;
   run→cancel|archive;
   trust→revoke;
   package/pricing→retire;
   external_account→disconnect;
   cost_account→close;
   secret→revoke.
   There is no generic delete or revoked→active transition.

4. workspace_limits:
   target:Binding<workspace_budget>;
   limits:BudgetCaps/1.
   W only.

5. activation:
   target:Target<activation>;
   packages:[ContributionBinding/1];
   registrySnapshot:D4.RegistrySnapshot/1;
   registryEvolution:Option<D4.RegistryEvolutionProof/1>;
   trust:Binding<trust>.
   W plus registry_admin only. A non-bootstrap Registry requires evolution proof.

6. deployment_put:
   target:Target<K>;
   value:DeploymentValue/1.
   K is only `deployment_policy|trust|package|external_account|secret|grant|cost_account|pricing`, must match `value.kind`, and is available only to H.

7. cost_reconcile:
   reservation:Binding<reservation>;
   evidence:EvidenceTicket/1.
   H or the account’s reconciler only; no targetState, actual, or manual-amount field exists.

Option<T> is only {kind:"none"} or {kind:"some",value:T}; null is forbidden.

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

All step/input/output/elapsed maxima are finite positive values. costs is sorted and unique by grant ref and currency matches the grant/account. costs has 0..32 entries. Core resolves each grant to the exact actual account/currency at configuration. Multiple entries in one BudgetCaps for the same actual account/currency must have equal maximum values, otherwise invalid_request; they jointly express one owner/account ceiling, while each grant keeps its own independent limits. A layer with no matching account cap permits no new billable attempt through that layer. Grant replacement never changes the cumulative owner/account key defined in §10.

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

`tool` parameters must validate against the active ToolValueProfile/1 input type of that Contribution. `core_field_member` accepts only the exact accepted first-party adapter identity above and its closed `FieldMemberTask/1`; it does not use ToolValueProfile and grants no extra read/write authority. A recurrence selector only locates D4 recurrence source rebound to the same current revision and never becomes a durable EntityRef.

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

`LeaseReadGrant/1` is a D10 attenuation projection rather than a new D6 grant wire. It consumes the exact scope/capability decoder and scope-applicability matrix from S D6 Control §4: for example `workspace_state` is workspace-scope only, `entity_state|locator_state|source_envelope_state` retain the original workspace/ref_set rules, and Field read uses the original D6 `field_read` shape. It forbids write/admin/repair/audit/export, `commit_sequence_state`, and `d10_control_self`, so it can only narrow read/state-observation authority the actor already possesses.

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

`readGrants` accepts only `LeaseReadGrant/1` above; actual authorization is still computed from current D6 Policy with its original deny precedence, and a Lease cannot add authority. `notBefore < notAfter` and `maxRuns` is finite positive.


For one `core_field_member` Run step, Core performs exactly this mapping and no callback dispatch:

1. Bind the current Automation definitionRevision, exact invocation, ActivationBinding, Lease, current D6 principal, and all finite budgets.
2. Before reading values, run the original D7 Narrow Field Qualification graph for the configured owner/Field/member and prove every required current observation/write scope. Full-source validation may occur internally, but a narrow caller never receives hidden source bytes.
3. Read the complete current Field for `task.ownerNodeRef/task.fieldId`. Automatic execution requires exactly one Entry and the configured member must already be present. From that exact preimage obtain the complete current SourceObservation/1, corresponding SourceVersionRef/1, real managed SourceVersion/2.revision as `expectedRevision`, its real `occurrenceKey`, and the original complete `rawEntrySource`; no stale selector or sourceToken is persisted in Automation configuration. An externalSequence never substitutes for a managed Counter.
4. Construct the original D7 intent exactly as `{format:"weftext.action",version:1,intent:{kind:"set_field_member",selector:{owner:task.ownerNodeRef,fieldId:task.fieldId,expectedRevision:<fresh owner source revision>,occurrenceKey:<the unique current Entry key>,rawEntrySource:<the exact original Entry JSON text>},memberPath:task.memberPath,value:task.value}}`.
5. Call original d7_action_prepare/2 with exactly one selectedSources entry, the current SourceVersionRef/1 for that Field owner, and the same Workspace, current commitDomain, complete expectedFrontier and original budget; retain its PreparedActionBinding/3, fetch and validate the complete preview/effects/MutationFootprint, then compare the **actual** member-change or byte-exact raw-no-op against the separate current Standing Approval. Approval never supplies the target, Entry, member path, or requested value.
6. Core constructs ApprovalUse from the original prepared semantics and enters D6 with the original `d6_commit_request`. Planning/final/replay remain owned by D6.

For optional S `people/phone.label`, task.value is the D7 Optional TypedLiteral `{type:{kind:"optional",item:{kind:"semantic_code",scope:<the complete people contribution-set scope>}},value:{state:"some",value:"people/work"}}`. The D4 scope remains all three codes `people/other|people/personal|people/work`; an approval enum may intentionally allow only a subset. One current phone Entry with present `personal→work` is a true member-change. Present `work→work` is eligible only when the complete proposed source is byte-equal and therefore exercises the existing raw-no-op branch. Zero or multiple current phone Entries make the automatic profile inapplicable; an interactive user may still select the second same-value phone by its real D7 selector and use the ordinary D7 confirmation path.

All original D2/D4/D6/D7 limits remain active, including D4 raw Entry 65,528 bytes and the existing complete source/header/carrier/entry/check budgets. Internal complete-source validation never expands public disclosure. If `D10AuthorPreparationLink/1` exists after restart, Core resumes only that exact original PreparedActionBinding/request. If it is absent and Core can prove the atomic save never succeeded and no request was delivered, a new prepare may be created. If existence/continuity is unknown, return `state_unavailable`; planned or submitted-unknown work recovers the original request and never creates a new OperationId.

    StandingApprovalSpec/1 = {
      notBefore:D4.zoned_instant,
      notAfter:D4.zoned_instant,
      rule:SingleFieldMemberRule/1,
      maxSuccessfulCommits:Counter,
      costGrants:[Binding<grant>/1]
    }

`D10-CONTROL` is the unique owner of the generation-one automatic-author rule:

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
         scope:<exact ResolvedCodeScope structure frozen by S D7 Value §5.2>}

    SingleFieldMemberValueConstraint/1 =
        {kind:"enum", values:[D7.TypedLiteral]}
      | {kind:"numeric_range", minimum:D7.TypedLiteral, maximum:D7.TypedLiteral}
      | {kind:"text_utf8_no_crlf", maximumUtf8Bytes:Counter}

`memberPath` has 1..7 names. Each name uses the original D4 `ObjectMemberSpec/1.name` decoder. Starting at the complete alias-expanded Field valueType root, every nonterminal element must select a direct object member; list/set/sequence indexes, union-arm selectors, wildcard, JSON Pointer syntax, runtime strings, and dynamic FieldIds are forbidden. The terminal member must already be present in the current Entry. D4 type depth remains maximum 8 and alias expansion consumes that original depth, so 1..7 is only an absolute outer bound and never bypasses the D4 depth/schema/source checks.

`memberType` is the underlying scalar D7 bridge type. For a required D4 member the original D7 `set_field_member` Action literal has exactly that TypeSpec. For an optional D4 member the Action literal must have exact D7 type `{kind:"optional",item:<memberType>}` and value `some`; Core extracts only that present scalar for the rule/constraint comparison. `optional.none` is never automatically approved. This preserves the original D7 optional-member bridge rather than confusing the scalar domain with its Optional wrapper.

`enum.values` has 0..64 complete scalar D7 TypedLiterals. Empty is legal and matches no automatic operation. Every literal type must be byte-equal to `memberType`; values are strictly sorted and unique by complete D3-CJ/3 canonical UTF-8 bytes. Membership uses the original D7 Value §2 same-type equality: exact text by scalar value, numeric values exactly, and semantic_code by complete resolved scope plus code. No numeric widening or text/code coercion is permitted.

`numeric_range` is only for int64/integer/decimal. minimum and maximum must have exactly the same complete type as `memberType` and satisfy the original D7 same-type order `minimum <= maximum`. `text_utf8_no_crlf` is only for exact `{kind:"text"}`; `maximumUtf8Bytes` is 0..65528, directly bounded by the S D4 maximum raw Entry UTF-8 size. The candidate Unicode-scalar text is encoded as UTF-8, must be at most that limit, and must contain neither U+000D nor U+000A. The complete proposed D4 Entry/source is still separately encoded and checked against D4/D2 framing, escaping, nonEmpty, schema, cardinality, and source budgets.

All constraints only narrow the current D4/D7 domain. A valid enum hit never substitutes for current Registry contribution availability, Narrow Field Qualification, current D6 authorization, `source_envelope_state`, or the separately required `commit_sequence_state` at author commit.

The two actual-effect branches remain closed. Member-change requires the original D7 adapter to produce exactly one existing scalar-member MutationFootprint and the complete owner_fields field_change while every other source/member/qualifier/note/provenance/Entry/body/Facet/Ref/relation/control fact stays unchanged. Raw-no-op additionally requires the same current owner/Field/unique Entry/member, D7 same-type equality after the required/present-optional projection above, and byte-for-byte equality of the original adapter's complete proposed source to the before source; MutationFootprint, field_change, and D6 sourceVersions remain empty. Typed equality alone cannot authorize a byte rewrite.

A malformed Standing Approval configuration returns management `D10ControlError.invalid_request`. A currently valid D7 Action whose scalar value is outside the frozen constraint is simply not covered by Standing Approval and returns the pre-D6 Run/step `approval_required` path; expiry returns `approval_expired`. After formal D6 entry, D6 owns the error exactly as specified in the coordinated amendment.

Normative positive semantic-code fixture: current S `people/phone` expands `people/labeled-text-value.label` as an optional contribution-set semantic_code. With one current phone Entry whose `label` is present, the D7 Action value is Optional<semantic_code>.some; `SingleFieldMemberRule.memberType` is the underlying semantic_code scope. An enum containing `people/personal` and `people/work` may authorize present `personal→work`, while `work→work` exercises raw-no-op. An allowed D4 code omitted from the approval enum, for example `people/other`, does not become automatically approved.

R08 separates immutable external-request semantics, a concrete send attempt, and the mutable external-effect lifecycle:

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

`FrozenEffectBytes.byteLength` equals the exact bytes and digest equals their SHA-256. Every instance must fit the narrowest applicable finite Run/Automation/Lease input and egress limits. `bounded_key.key` is non-empty UTF-8 text of at most 1024 bytes, `notBefore < notAfter`, and proof is the accepted adapter's complete immutable evidence for the same contribution/account/operation/target/key/window. This proof is internal bytes and is **not** an EvidenceTicket arm.

`requestDigest` is SHA-256 of the complete canonical frozen ExternalEffectIntent under domain D10-External-Intent/1; `targetDigest` is the complete ExternalTarget digest under D10-External-Target/1. Core retains the full bytes. `budgetReservations` is 0..32, sorted/unique by reservation ref, and each item resolves to a CostReservation whose `attemptId==billableAttemptId`; later settlement may advance that reservation revision without mutating the immutable send binding. `sendAttemptId` is distinct from every billableAttemptId.

The public current view is only `ExternalEffectCurrentView/1`. It never returns request payload bytes, target ToolValue, idempotency key/proof, secret generation, approval record, or reservation identities. Even digest/account/contribution disclosure requires current authority for the original frozen effect scope.

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

planned authorizes only the original planned request and never reprepares, changes OperationId, or changes target. external authorizes only the immutable ExternalEffectIntent whose exact ControlRef and requestDigest match. A legal `prepared→submitting→...` lifecycle revision never changes that frozen request digest and therefore never invalidates consent by itself; changing contribution/account/target/payload/idempotency creates a new effect intent and needs new consent.

A successful prepare returns:

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

affected is sorted/unique by ref canonical bytes; resourceUses is sorted/unique by grant ref. canonicalIntentBytes is the complete successfully closed-decoded D10-Control-Intent/1 byte sequence, so it carries the proposed control semantics already supplied by and visible to the caller. preview adds no secret bytes, hidden author value, or another user's billing.

The external arm of consent has a positive, authorized full-request review path through the existing prepare response. `externalRequest` is some exactly when the closed body is consent with external consent, and none for every other body. Core resolves the exact frozen intent from its protected mapping only after current authority to the complete request scope, contribution, account, target and payload; permission to observe a digest or use a broad resource grant is insufficient. Missing/hidden/wrong audience returns `not_visible` before revealing intent existence. Core compares the full frozen intent with the supplied effect Ref and request digest, validates the frozen payload length/digest, and pins the actual bytes in the original preparation. A mismatch returns `control_conflict`; a proven protected-byte contradiction returns `integrity_conflict`; temporarily missing pins/continuity returns `state_unavailable`. An actual lifecycle transition does not change this immutable intent. No effect-specific current-read endpoint is needed.

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

The preview copies the exact target ToolValue, operation, complete payload bytes, actual account and contribution from the frozen intent; it never accepts these values from a model description or a replacement preview argument. The bounded-key summary discloses its accepted validity window and SHA-256 digests of the canonical key and complete proof under D10-External-Key/1 and D10-External-Idempotency-Proof/1; it conveys no reusable key, proof bytes or credential. The full intent, including those protected bytes, stays pinned under the original request digest. Secret authentication bytes are injected only after approval through the existing trusted authentication channel and cannot alter the approved business target or payload. Payloads requiring hidden credential substitution inside business bytes are not admitted by this profile.

The complete canonical ControlPreview is bounded by 16777216 bytes and any stricter entrypoint/transport budget; its complete external payload is at most 8388608 bytes. Exceeding either bound returns `budget_exceeded`, with no truncated review or confirmable partial response. This is one complete delivery, not a new paging protocol. The accepted trusted UI or attended CLI verifies every frozen byte length/digest and the entire canonical preview, renders all target components and payload through an inert exact representation, and makes the full representation available before enabling confirmation. Text control characters are escaped; binary content has a lossless byte view. Adapter summaries, model prose, a collapsed prefix, or a digest alone cannot satisfy complete presentation. If the surface cannot present this request completely, it cannot confirm it. This proves which complete request was made available and explicitly confirmed, not that a person mentally read each byte.

Only a fresh explicit action by the current authenticated user in that trusted confirmation surface can establish the internal `ExternalConsentConfirmation/1`; the delegated Agent/tool/worker/connector cannot create it, call the trusted event channel, or substitute a JSON flag. The trusted surface supplies an authenticated user event tied to its current complete presentation; Core verifies current audience, complete-preview digest under D10-Control-Preview/1, exact original stable key/intent, trusted time, consent interval and current dependencies, then atomically records the fact in its distinct protected ExternalConfirmationRecord/1. This event is an internal confirmation operation of the accepted surface, not a public control body, new author request, or second success ledger. An ordinary authenticated script or a delegated execution session without that attended confirmation event cannot manufacture the fact. Platform acceptance must demonstrate that this event channel cannot be invoked by the executable contribution being approved.

The immutable ControlPrepareBinding/2 stores ExternalConfirmationRequirement/1, while the distinct protected ExternalConfirmationRecord/1 in §8 stores the current confirmed fact. Confirmation never mutates the bound input. The fact principal equals the actual Workspace principal in the stable key, never a caller-provided grantor. New target/payload/intent, a different preview or principal, or changed dependencies cannot inherit confirmation. A caller may re-present the same still-valid preparation after interruption under the original trusted event rules. Historical result/current projections remain redacted and never return the full review or this protected fact.

For an undecided external-consent commit, after current visibility/authority and stable-key/saved-result handling, the D10 adapter requires this exact currently eligible confirmation before forwarding the original Workspace request. A visible eligible preparation with no confirmed event remains awaiting the trusted user action in that surface; this is presentation state, not a new public response or Run error. Prepare still returns the original complete prepared response. There is no invented D10 Workspace-commit endpoint: the only public Workspace submit is the original D6 request, whose rejection below owns the formal result. Unprovable trusted confirmation/time/continuity prevents confirmation and remains `state_unavailable` on applicable D10 reads/preparation. Core associates the saved confirmation and full original preview with the original D6 control plan as a protected eligibility dependency; both planning CAS and final commit revalidate it, complete current request-disclosure/operation authority, consent time, unchanged intent and original dependencies. Directly submitting the returned D6 request cannot bypass the check. After D6 entry, original permission/business errors keep precedence; an otherwise eligible but absent or unusable confirmation returns the coordinated D6 `approval_unavailable/preflight`, leaves an existing planned decision recoverable, and creates no alternative approval or decision. Saved applied consent is replayed under current result authority before any fresh-confirmation gate; it is never asked to confirm again or converted into failure. The actual D6 producer and accepted trusted surface must support this association before this path is available.



Result query:

    D10ControlResultRequest/1 = {
      kind:"d10_control_result",
      wireVersion:1,
      scope:Scope/1,
      requestId:Uuid
    }

Historical prepare/apply and current exact-record reads are deliberately separate.

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

The protected `ControlPrepareBinding/2.canonicalIntentBytes` continues to store complete canonical control intent B, including any nested planned author request A; stable-key conflict still compares the complete bytes. `intentDigest` is only SHA-256 of those saved bytes. Historical public result never returns A, complete B, generated control-submit request M, prepareToken, or planned-preview bytes/token. `operation` is exactly one of the seven ControlBody kinds; `ControlAffectedChange/1` and `ControlResourceUse/1` are the existing ControlPreview item shapes with unchanged fields, enums, sorting, uniqueness, Option, and Money semantics.

Initial `d10_control_prepare` may return full `D10ControlPrepared/1`, including M, only after current disclosure authority covers complete B and any nested A scope. Earlier possession of A is not current read authority. Same-key prepare after an authoritative applied success returns the same `d10_control_result_applied` historical arm, never a fresh-looking prepared object. If applied-success existence or linkage is unprovable, return `state_unavailable`; never downgrade to prepared.

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

Workspace applied history is projected from the same authoritative D6 saved decision: `ownerReceipt` is the original immutable receipt bytes and `changes/usageChanges` are only its decision-linked D10 control effects. Deployment applied history is projected from the original `DeploymentControlDecision/1` plus the record/account deltas atomically linked to that decision by §8. Neither arm is a second success ledger.

The direct successful response to `d10_host_control_commit`, both on its first atomic success and on exact successful replay, is exclusively the existing `D10ControlResult/1` applied arm: `{kind:"d10_control_result_applied",wireVersion:1,scope:<original deployment scope>,requestId:<original requestId>,prepared:<original ControlPreparedHistory/1>,applied:<original ControlAppliedHistory/1 deployment arm>}`. Core constructs it from that same saved decision and its linked preparation/effects; it never returns the internal `DeploymentControlDecision/1`, an empty acknowledgement, a fresh prepared object, or a second receipt. A true no-op still returns this applied arm with empty configuration `changes` and the actual saved `usageChanges`. A later r6 configuration does not replace any original r5 field. Immediately before delivery Core rechecks current disclosure authority over the complete public result: loss of that authority returns `D10ControlError.not_visible`; temporarily unprovable saved-success linkage returns `state_unavailable`, and a proven contradiction returns `integrity_conflict`. These delivery failures preserve the already committed decision and effects; retry/result lookup recovers the same applied history after the original gates, without rerunning the operation.

Result resolution order is: current result-disclosure authorization → authority/custody/continuity → stable key. Missing/hidden is `not_visible`. If a prepare binding is proven and no applied success exists, return prepared. If an authoritative applied success exists, return applied. A D6 recorded rejection/terminal failure remains owned by D6 and is obtained by replaying the original D6 request; D10 result may return prepared history only after it proves that no applied success exists. If the presence/absence or linkage of an applied success is temporarily unprovable, return `state_unavailable` rather than downgrading to prepared. Proven decision/effect linkage contradiction is `integrity_conflict`.

If r5 applied, the response was lost, and another valid request later changes the same record to r6, retry of the original request returns the saved r5 applied history after current disclosure authorization. It never substitutes r6.

Current state uses a different entrypoint:

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

Non-Stop K always requires `scope == record.scope` exactly; there is no wildcard, list, name or display lookup. **Stop-only dual access relation:** after closed decode and D1 `automation.stop`, establish the **currently authenticated** W user or H operator and verify full target, StopOwner, latch and nested Run/Automation disclosure **before** existence. W request scope must be the target's real `{kind:"workspace",workspaceRef}`; H request scope must be `{kind:"deployment",storeIncarnation:<actual-store-incarnation>}` with current target-and-Workspace-qualified DeploymentControlPolicy stop/read authority, not mere H access. After real authority/fence/custody, use the **one** protected `StopOwner/1` mapping to verify the actual target full ControlRef, target's real Workspace, unique stop latch Ref, equality of all target/latch/owner/storeIncarnations, `requestId==target.id`, and the single persisted Stop Image1 with its real **Workspace** scope. H's scope is a read qualifier, never a second persisted Stop identity or image. Project `D10ControlCurrent.scope` from the validated W/H request, but `binding`, `usageRevision=none`, `stop_state`, state and revision from **that same single protected Stop record at one cut**. `d10_emergency_stop_result` uses the identical target/StopOwner/latch relation and saved original receipt/not-applied cut; r5→r6 target configuration never mutates it. Hidden/absent, cross-Workspace, wrong store or unqualified nested identity → `not_visible`; authority/fence/custody temporarily unprovable → `authority_unavailable`; proved owner/store/target contradiction → `integrity_conflict`; temporary latch/result continuity loss → `state_unavailable` in the original precedence. Final result disclosure must be rechecked even after a saved success. No Stop fact, audit, safety reservation, revision or result may be duplicated for H. A visible non-Stop record with unknown protected continuity remains `state_unavailable`; unknown compatible state/member remains integrity/version failure, not a guessed state.

The generation-one current projections are closed as follows:

    RunOrigin/1 =
        {kind:"interactive"}
      | {kind:"automation",
         automation:Binding<automation>/1,
         definitionRevision:Counter,
         occurrenceKey:AutomationOccurrenceKey/1}

`RunOrigin/1` is an immutable Core-created source fact shared by the protected Run record, `LeaseRunUse/1.origin`, and the public `run_state.origin` projection. The interactive arm has exactly `kind`; it requires no Automation, definition revision, occurrence claim, or sentinel/null substitute. The automation arm binds the actual automation, immutable definition, and complete occurrence claim that created this Run; its key's complete Automation Ref equals origin.automation.ref and the original claim's definitionRevision equals origin.definitionRevision; the key itself contains no definitionRevision. No caller/model chooses or changes an existing Run's origin. A current control revision may change without rewriting the historical origin binding; new execution still passes the current gates. A Run cannot switch arms to escape an existing occurrence claim or budget lineage.

For an interactive Run, the Lease target is exactly that Run's full ControlRef. For an automation Run it is exactly the origin's Automation ControlRef. In both cases the authenticated principal, Workspace, activation, exact admitted Lease revision, finite limits, stop latch, and all existing admission/recovery rules apply. Only the automation arm reads or writes an occurrence claim. A contradiction between the protected Run, its admission, and its actual claim/Lease is `integrity_conflict` on control read and `control_conflict` before Run execution; unprovable continuity is `state_unavailable`. An otherwise visible Run with an unauthorized nested origin/Lease/stop binding returns `not_visible` as a whole; a hidden Automation never becomes an interactive projection. These revisions amend the unactivated candidate types; they do not authorize guessing an origin when decoding any actual historical record.

### 7.1 Trusted interactive-start production (not an eighth ControlBody)

The **only** fresh interactive Run plus Run-targeted Lease producer is the narrow Core runtime entrypoint below, parallel to (not delegated from) the original Automation occurrence producer. This is neither `automation_configure`, an eighth `ControlBody`, a public author request, a generic callback nor an Agent-controlled `approved` flag. D1 must actually expose `automation.manage` through its real release/contract-major/surface/version/health precedence; current D6 Policy/3 must independently allow the real authenticated user's workspace `d10_control_self`. Neither Field authority, issuer control, policy_admin, a tool description, nor a Deployment connection implies these.

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

`requestId` is persisted by the client **before** first delivery and is the only caller-selected stable coordinate. The closed request has no `principal`, Run/Lease Ref, Automation, occurrence, target, authorRequest, `approved`, or creation authorization token. `delegation` is the entire finite LeaseSpec/1, not an unconstrained description: its internal activationBinding must exactly equal the outer binding; its `notBefore < notAfter`, finite positive `maxRuns`, D1 capability allowlist, exact D6 LeaseReadGrant scope/Field set, H-issued ResourceUseGrant bindings and every BudgetCaps entry are fully decoded and verified. Core intersects those bounds with the real current Policy/3 allow/deny, actor/workspace/audience, active ActivationBinding/selector, actual contribution and deployment grant permissions, owner/account/currency, current held/spent ceilings and clock. An empty billable-grant set grants no billing; neither a Lease nor a user can mint a new deployment grant, read/write permission, secret, egress, account or Field permission. Frozen Run budget is exactly the accepted LeaseSpec.budgets, not later increased by management of another layer.

An accepted start additionally requires an **internal trusted attended-user event** created by the registered Desktop, authenticated Server-hosted attended UI (including WebUI through Server Broker), or genuinely attended CLI adapter. The event attests the actual D6-authenticated natural user and session, exact Workspace, trusted clock epoch, digest of the **complete canonical start request**, full visible Lease/activation review and a fresh explicit Start act. Core verifies the event against the requested bytes, principal, current audience and time. This event is **not** caller JSON, a client-controlled boolean, a reusable bearer token, an Agent/model/tool callback, an unattended CLI switch, or a script-accessible channel. Mobile and untrusted workers cannot generate it. Adapters do not supply or modify the real principal; Core obtains it from the existing D1/D6 authentication boundary. A surface incapable of presenting the complete request or proving the actual human act cannot offer this path.

**Unique first-start/result order:** (1) strict closed decode/bounds and real D1 capability availability; (2) authenticate user and attended event, exact W scope, Policy/3 self-capability, every requested authorization/disclosure and nested resource audience **before protected existence**; hidden/wrong workspace/principal/scope gives `not_visible`, malformed input or a missing/invalid attended event after authorized decoding gives `invalid_request`; (3) verify actual D6 custody/fence/authority, **the same** open control-store incarnation and trusted clock continuity (`authority_unavailable` for authority/fence loss, `state_unavailable` for temporary clock/record continuity); (4) look up existing `StableControlKey/1` using actual scope incarnation, authenticated principal and requestId and compare the complete canonical `D10-Interactive-Start/1` request bytes (same key/different bytes → `control_conflict`, not digest equality); (5) authoritative applied success replays the immutable original Started result after current disclosure, with no new allocation or stale-current-binding veto; an unknown applied/not-applied link returns `state_unavailable`, proven contrary bytes `integrity_conflict`; (6) only on proven unseen, validate active selector and full actual Policy/Lease/grants/clock `notBefore <= now < notAfter`, finite budget, stop and complete negative record/range/capacity proofs (`control_conflict` on changed known binding, `budget_exceeded` for capacity/checked arithmetic, `state_unavailable` for unknown continuity); (7) recheck those predicates within **one actual Authority Store control serialization/CAS** and atomically save all results; (8) repeat the final complete-result disclosure barrier before transport. Existing D10ControlError/1 owns start/result; once execution begins D10RunStepError/1 or the D3/D6/D7/D8/D9 owner applies unchanged.

On the unseen winning CAS Core allocates exactly one fresh never-reused full `ControlRef<run>`, `ControlRef<lease>` and `ControlRef<stop>` in the actual storeIncarnation, all with real Workspace scope. The Run and Lease configuration Bindings start at revision 1. The Lease Image1 has state active, **principal from trusted D6**, `target` equal to the entire new Run Ref, accepted LeaseSpec, `runsConsumed=0`, `usageRevision=some(0)`. Run Image2 has `run_state {lifecycle:"active",executionState:"queued",origin:{kind:"interactive"},lease:<Lease Binding>,admission:{kind:"not_admitted"},stop:<Stop Binding>}`, `usageRevision=none`; its protected supplement has `createdBinding=<originalRunRevision1>,principal=<authenticatedPrincipal>,fixedBudget=<LeaseSpec.budgets>,invocation:none,admission:none,authorSteps:[]`. Core creates the **one** W-scoped Image1 `stop_state` with open latch revision 1 and exact `StopOwner/1` linking this Run, Workspace and storeIncarnation. Before the Run is manageable, it reserves exactly one §11 latch, immutable result/audit slot and safety sequence unit; StopCapacity never relies on ordinary management quota. The same transaction persists original key/complete bytes, immutable start result, all three records and associations, original owner/range fences/pins, safety capacity, retention and audit. Competing start/revoke/activation/stop/capacity attempts serialize in this **same** store; fail/crash/losing CAS leaves no partial Run/Lease/Stop, no applied-key illusion, no lost safety slot and no writable/usable Run. This is **not** a D6 OperationId, author DecisionKey/P, new ControlBody, Automation/occurrence claim, Standing Approval or second decision ledger.

Creation does **not** consume `maxRuns`. The original §8.2 first-protected-step `LeaseRunUse/1` CAS later verifies the exact Run-targeted Lease, immutable interactive origin, actual principal/Workspace/activation, live stop, trusted clock, current grants/Policy and all finite budgets. Only that CAS increments cumulative lease uses **once** and stores the complete admitted lease revision and original time. An already-admitted `maxRuns=1` Run may execute its second protected step or recover the same originally planned D6 request after crash/lost response without another consumption or invented Automation claim. Every later step still rechecks current authority, exact admitted Lease revision, time, activation, stop, approval and budgets; expiry/revocation/stop/unknown continuity forbids new execution without undoing committed work. A second Run cannot use this Run-targeted Lease. Failed step/cancel/restart never refunds the first Run use. All original saved/planned/unknown author work resumes under its **recorded** responsibility arm: automatic uses its real Link2/PAB4/ApprovalUse2; interactive uses original D3/D6 request, actual D7 PAB4 or D8 EditBinding3, exact preparedFormat, preparedRecordPin, recoveryPins and renewed full-preview human confirmation where required. No replacement OperationId, DecisionKey, prepared binding, approval or author P is created.

`d10_interactive_run_result` accepts only its closed request under the original principal/scope, authority/fence/custody and complete nested Run/Lease/Stop result-disclosure rules; it returns the saved exact `D10InteractiveRunStarted/1` or the original control error. Proven unseen/hidden → `not_visible`; uncertain original result or any of three linked records → `state_unavailable`, not a fresh Run. A lost initial response, identical concurrent request and later Run/Lease revision r5→r6 restore the immutable originally saved revision-1 result. Changed intent with original key → `control_conflict`. Wrong control store or scope never aliases a local Ref. All read/write results require the final disclosure gate.

**Unexecuted test oracle:** authenticated interactive user with no Automation gets new Run + Run-targeted `maxRuns=1` Lease, first admission once, D7 full-preview/explicit-confirmation for D3 identity and D6 author steps (including the D8 EditBinding3 path), second step and cold recovery of original planned work with remaining 0. No Link2/ApprovalUse2 is constructed for interactive; a real Automation core_field_member still requires both. Mutants: fake principal/event, unauthorized D6 Field/read/resource, expanded account or budget, duplicate key/different request bytes, wrong store, missing owner pins, StopCapacity exhaustion/stop race, revoked or expired lease/activation, clock epoch loss and budget overflow. They must fail in the original closed error domain and leave zero half-state, no duplicated lease use and no D6 author decision side effect.

### 7. Nineteen readable closed current projections

Each category retains the exact four original cells: kind, scope, complete view and configuration/domain/usage semantics. Only Markdown presentation changes; no decoder changes.

#### 1. automation

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"automation_state",state:"enabled"|"disabled"|"archived",definitionRevision:Counter,subscriptionGeneration:Counter,lowerOriginalStartUtcSeconds:CanonicalDecimal,definition:AutomationSpec/1,lease:Binding<lease>/1,approval:Option<Binding<approval>/1>}
```

**Config/domain/usage:** `binding.revision` is the control/lifecycle CAS; `definitionRevision` changes only semantic definition. usageRevision none.

#### 2. lease

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"lease_state",state:"active"|"revoked"|"archived",principal:Token,target:ControlRef<automation|run>/1,spec:LeaseSpec/1,runsConsumed:Counter}
```

**Config/domain/usage:** `binding.revision==leaseRevision`; usageRevision some and advances only when a new `LeaseRunUse/1` consumes the lineage. Normal usage never stales an already-admitted Run's leaseRevision.

#### 3. approval

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"approval_state",state:"active"|"revoked"|"archived",grantingPrincipal:Token,automation:Binding<automation>/1,definitionRevision:Counter,lease:Binding<lease>/1,activationBinding:ActivationBinding/1,spec:StandingApprovalSpec/1,reserved:Counter,consumed:Counter,releasedTerminal:Counter}
```

**Config/domain/usage:** `binding.revision==approvalRevision`; usageRevision some for ApprovalUse reserve/consume/released_terminal only. Revocation/archive advances approvalRevision without resetting usage.

#### 4. planned_approval

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"planned_approval_state",grantingPrincipal:Token,originalRequestDigest:Sha256,previewSemanticDigest:Sha256,lease:Binding<lease>/1,activationBinding:ActivationBinding/1,notBefore:D4.zoned_instant,notAfter:D4.zoned_instant}
```

**Config/domain/usage:** binding revision is its approvalRevision; usageRevision none. No author request/preview bytes are exposed here.

#### 5. external_approval

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"external_approval_state",grantingPrincipal:Token,intent:ControlRef<external_effect>/1,requestDigest:Sha256,resourceGrants:[Binding<grant>/1],notBefore:D4.zoned_instant,notAfter:D4.zoned_instant}
```

**Config/domain/usage:** binding revision is its approvalRevision; usageRevision none.

#### 6. run

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"run_state",lifecycle:"active"|"archived",executionState:"queued"|"running"|"awaiting_confirmation"|"blocked"|"cancelling"|"reconciling"|"completed"|"failed"|"cancelled",origin:RunOrigin/1,lease:Binding<lease>/1,admission:{kind:"not_admitted"}|{kind:"admitted",leaseId:Uuid,leaseRevision:Counter,admittedAt:D4.zoned_instant},stop:Binding<stop>/1}
```

**Config/domain/usage:** binding revision advances on durable Run/lifecycle transition. usageRevision none; maxRuns consumption belongs to Lease usage.

#### 7. workspace_budget

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"workspace_budget_state",limits:BudgetCaps/1}
```

**Config/domain/usage:** binding revision is limits CAS. usageRevision none; actual cost usage remains in grants/accounts/reservations rather than a second budget ledger.

#### 8. activation

**Exact scope:** `workspace`

**Complete view:**

```text
{kind:"activation_state",current:Boolean,activation:ActivationBinding/1,packages:[ContributionBinding/1],trust:Binding<trust>/1}
```

**Config/domain/usage:** `binding.revision==activation.activationGeneration`; successor activation makes earlier record `current:false` without deleting it. usageRevision none.

#### 9. deployment_policy

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"deployment_policy_state",policy:DeploymentControlPolicy/1}
```

**Config/domain/usage:** `binding.revision==policy.revision`; usageRevision none.

#### 10. trust

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"trust_state",state:"active"|"revoked",publisherId:Text,publicKey:Ed25519PublicKey,previous:Option<Binding<trust>/1>,proof:EvidenceTicket/1,claims:[NamespaceClaim/1]}
```

**Config/domain/usage:** binding revision covers key/claim/rotation/revocation. usageRevision none.

#### 11. package

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"package_state",state:"installed"|"retired",manifest:PackageManifest/1,signature:Ed25519Signature}
```

**Config/domain/usage:** binding revision covers install/update/retire; packageVersion is independent. usageRevision none.

#### 12. external_account

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"external_account_state",state:"connected"|"disconnected",provider:ContributionBinding/1,externalAccountId:Text,endpointId:LocalOperationId,proof:EvidenceTicket/1}
```

**Config/domain/usage:** binding revision covers configuration/disconnect. usageRevision none.

#### 13. secret

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"secret_state",state:"active"|"revoked",account:Binding<external_account>/1,audience:ContributionBinding/1,usageKind:"authenticate",secretVersionId:Token}
```

**Config/domain/usage:** binding revision covers publish/rotation/rebind/revoke. No plaintext/staged bytes are returned. usageRevision none; secret-use counts remain in the ResourceUseGrant.

#### 14. grant

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"grant_state",grant:ResourceUseGrant/1}
```

**Config/domain/usage:** `binding.ref.id==grant.grantId`, `binding.revision==grant.grantRevision`, and usageRevision is some and equals `grant.usageRevision`. `archive` maps to the existing `retired` state.

#### 15. cost_account

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"cost_account_state",state:"active"|"frozen"|"closed",currency:CurrencyCode,ceiling:Counter,pricing:Option<Binding<pricing>/1>,spentMicroUnits:Counter,heldMicroUnits:Counter}
```

**Config/domain/usage:** binding revision covers config/close/freeze; usageRevision some for held/spent reservation deltas. Freeze never clears liabilities.

#### 16. pricing

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"pricing_state",state:"active"|"retired",account:Binding<external_account>/1,currency:CurrencyCode,fixedMicroUnits:Counter,meters:[PricingMeter/1],evidence:EvidenceTicket/1}
```

**Config/domain/usage:** binding revision covers pricing update/retire. usageRevision none.

#### 17. reservation

**Exact scope:** `deployment`

**Complete view:**

```text
{kind:"reservation_state",reservation:CostReservation/1}
```

**Config/domain/usage:** `binding.ref.id==reservation.reservationId` and `binding.revision==reservation.revision`. usageRevision none; `uncertain` is the existing recoverable state, not unknown.

#### 18. external_effect

**Exact scope:** `workspace`

**Complete view:**

```text
ExternalEffectCurrentView/1
```

**Config/domain/usage:** binding revision advances only on durable effect/recovery lifecycle transition and is distinct from the immutable requestDigest. usageRevision none; costs/attempt quotas remain their reservation/grant owners.

**Original Stop helper type:**

```text
    ExecutionStopLatchView/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal/1>
    }
```

#### 19. stop

**Exact scope:** exact target Workspace scope when read through W, or exact deployment storeIncarnation when read through H

**Complete view:**

```text
{kind:"stop_state",latch:ExecutionStopLatchView/1}
```

**Config/domain/usage:** fresh open record has revision 1; only `open→stopped` advances once; idempotent repeated stop is a no-op. Internal safety sequence is not public; usageRevision none.


For `lease|approval|grant|cost_account`, `D10ControlCurrent.usageRevision` must be `some` and equal the record's current independent usage revision; every other kind requires `none`. Trusted time crossing `notBefore/notAfter` changes current eligibility but does not silently mutate configuration revision or create a persisted `expired` state. Revocation/retirement/archive/close remains observable to an authorized reader and never aliases absence.

`activation_state.current` is derived in the same authorized cut from the exact active-selector record; the selector, not the historical ActivationBinding record, is the CAS object for switching. Successor activation compares the exact current predecessor selector and atomically publishes the new ActivationBinding plus selector. An old generation becoming non-current never changes that historical generation or its Binding revision.

Generation one exposes no independent early-revoke state action for `planned_approval` or `external_approval`. Their finite time bounds, exact Lease/Activation/request binding, current ResourceUseGrant state, current D6 authorization, and irreversible stop gates are rechecked at the relevant planned/send boundary. This support limitation does not create a generic approval lifecycle state and never releases planned work except through the original authoritative-abort rule.

## 8. Idempotency, CAS, and replay order

The stable key is (scope incarnation, initiating principal, requestId). The client persists requestId before the first call and reuses it for transport retry.

Core internally stores:

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

ControlDependencies/2 contains the complete dependencies actually read by Core, never caller assertions or only binding digests. workspaceReads is some for Workspace control and none for deployment-only control; a deployment body cannot smuggle in an author read. The Workspace fields equal the actual sourceInputs, observationScope and DependencyProof of this plan. The final observationProof is constructed only on the outer D6 PreparedIntent/2 from these exact inputs and pins, and is never embedded back into ownerInput; no ObservationProof/2 type or self-cycle exists. sourceInputs uses the original D6 order, uniqueness, roles and current-observation/pin rules. Each Registry entry preserves the complete actual RegistrySnapshot/1 and its exact binding, with an artifact PinRef/2 retaining those canonical bytes; Core validates all original D4 ownership/completeness/schema rules. An operation with no Registry use has an empty array, not a fake Registry binding. Array bindings are canonical sorted/unique by full Ref, Registry reads by full binding, and pins by pinToken. Authorization-generation tokens retain their original owner semantics and are canonical sorted/unique. Each array has at most 4096 items and complete canonical dependency metadata has at most 16777216 bytes; pinned source/payload bytes additionally obey their original budgets. Overflow is budget_exceeded, never truncation.

The D6 producer includes each real D6 source/Registry/authorization/range dependency under its actual DependencyKey/2 kind and stamp, with the required controlInputs correspondence and pins. D10 config/usage/stop dependencies remain their complete closed owner values inside the canonical ownerInput of the same InputDescriptor. They are compared against the actual protected records in the same planning/seal transaction; neither a digest nor an unrelated execution_resource key stands in for that CAS. This defines no fifteenth DependencyKey kind. The complete referenced configuration/history, independent usage state, authority evidence and pins remain retained with the preparation, and after planning for its full original recovery lifetime. Missing history never becomes an empty dependency set. Only lease maxRuns, approval counts, grant cumulative use and actual account held/spent use have the listed independent usage bindings; they never replace configuration bindings.

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

The external requirement exists exactly for the external consent body. Its key, actual initiating principal, complete frozen intent and preview digest equal the original preparation; notBefore/notAfter exactly equal the original ConsentSpec interval and satisfy notBefore < notAfter. It is immutable before the original request is generated. Other bodies require none and create no confirmation record. Each external requirement has exactly one protected record addressed by its full stable key; initial revision is 1 and current is none. Only the trusted attended event in §7 may CAS current to a verified matching confirmation. A duplicate identical fact is a no-op; re-presentation under a new valid trusted event may replace an ineligible fact with checked revision+1 while retaining every historical fact referenced by planned/recovery. Neither operation changes the requirement, original request, preview, InputDescriptor or dependency bytes. MAX fails closed with budget_exceeded. The planning/seal gate reads the currently eligible matching fact under this protected record's CAS; the fact is additional live eligibility, not a mutable member of the frozen input. A missing/unusable fact follows approval_unavailable only after the original owner authorization/business gates, while saved-decision replay precedes fresh confirmation.

The D6 d10_control/1 producer binds the complete immutable original intent, allocated refs, actual before/proposed effects, preview, ControlDependencies/2 and fixed confirmation requirement. It never embeds the generated originalCommitRequest in its own descriptor: the descriptor is finalized first, then D6 produces that request, and ControlPrepareBinding/2 atomically saves the exact association before delivery. Nested original author request A in a planned-consent body remains legitimate input; the generated control-submit M is only the outer association. No consumer may hash M into the descriptor that produces M or insert a later confirmation into the InputDescriptor. True saved /1 records retain their original decoder and exact recovery; this /2 contract does not relabel their bytes.

canonicalIntentBytes is the complete D3-CJ/3 canonical byte sequence after successful closed decode with domain tag D10-Control-Intent/1. A digest may index it but conflicts compare complete bytes. The binding stabilizes preparation and lookup and is not a second author decision.


The shared order is fixed:

1. closed decode plus D1 capability/release/surface/version/health gates;
2. use minimal protected mappings to validate current principal, scope, audience, observation, and operation authority; unauthorized, wrong-audience, and absent all become not_visible;
3. prove authority/fence/custody/continuity;
4. look up the stable key; visible same-key different canonical bytes → control_conflict;
5. for same-key same-input with an authoritative saved decision, revalidate current disclosure authority for the original result/effect scope and then replay the saved result; current target revision does not invalidate historical success;
6. only an undecided operation validates expected config/usage revisions, current authority, dependencies, time, and budget and constructs or resumes the original preparation;
7. Workspace submission enters the original D6 transaction/ledger; Deployment submission enters the closed host-control transaction in the same store incarnation;
8. atomically save effect, decision/receipt linkage, record/account deltas, dependency invalidation, evidence pins, and required audit linkage.

If another valid operation changes an object r5→r6 after the original r5 operation committed but its response was lost, retry of the original request returns the saved r5 outcome after current disclosure authorization; it neither recreates the object nor incorrectly fails stale. Reading current state is a different authorized read. If the caller later loses result-disclosure authority, result query returns not_visible while the saved decision remains immutable.

A failed prepare/commit creates no applied decision. An existing prepare binding may resume the same exact intent after transient state_unavailable. Changing expected revision, body, or target requires a fresh requestId. Dedup bindings, terminal proof, and planned/unknown/uncertain pins cannot be TTL-deleted so that an old requestId executes again.

Configuration revision and usageRevision are distinct. Checked increment at Counter maximum returns budget_exceeded and never wraps or resets. Retired IDs/incarnations are never reused, preventing ABA.

### 8.1 Original protected record /1 predecessor semantics (A2 current schemas in §0)

The following are internal Core types, not public read responses, new ControlRecordKind values, or a second ledger. A public ControlCurrentView alone is never a complete before-image. The current undeployed candidate closes its protected representation as follows; an actually saved older record keeps its original decoder and recovery obligations.

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

The supplement tag must equal K for the eight listed kinds; all other K require none. The view still uses exactly §7's closed shape and supplies every remaining configuration/lifecycle/usage member. Scope, identities, revisions and all overlapping fields must agree. run.createdBinding is its immutable creation binding, fixedBudget is the budget frozen there, and invocation is some with the original Automation invocation only for automation origin and none for an interactive Agent Run; origin comes from actual creation, never a current Automation substitution. A not-admitted Run has admission=none; an admitted Run carries its complete original LeaseRunUse. Historical author links are complete and unique by stepId. Planned approval retains original A, whose complete canonical bytes produce the public digest. External approval retains the exact full intent and fixed confirmation requirement. The actual confirmation fact and revision consumed at seal remain in their separately protected confirmation record and saved decision association, never in this immutable proposed image. A full stop latch, including safety sequence, is retained internally. Secret images contain only the original secret-version reference: the trusted secret store retains that immutable version separately; no image or control intent contains secret bytes.

activation.current is a cut-specific projection. The actual selector is D10ActivationSelector/1 = {workspaceRef:D3.WorkspaceRef, revision:Counter, current:Option<Binding<activation>/1>}. The selector has one permanent identity per Workspace/control store, is compared under the same transaction, and advances once when selecting a successor; historical activation images and generations are not rewritten. The initial none is legal only with protected proof that no activation was ever selected. automation.subscription.activeDefinition points to the image's proposed binding; this finite value is not an embedded image and creates no reference cycle.

    D10ControlRecordPin/1 = {
      image:D10ControlRecordImage/1,
      pin:D6.PinRef/2
    }

The pin is artifact with recovery or approval_money retention, containing exactly the D3-CJ/3 canonical image bytes prefixed by UTF-8 D10-Control-Record/1 and NUL. Its byteLength and bare SHA-256 must match those bytes; D10's prefixed Sha256 display does not change D6's digest decoder. Core verifies the original typed record against protected store provenance before minting a pin. A pin does not authenticate caller-authored bytes. Missing history is state_unavailable; a proven conflicting image is integrity_conflict, after disclosure gates. Every actual config/usage/stop binding in ControlDependencies has an exact image/pin; multiple historical revisions/use cuts may coexist and are ordered by complete Ref, configuration revision, optional usage revision, then complete canonical image bytes. Same protected-cut contradictions are rejected rather than hidden by equal revisions. Pins neither grant public disclosure nor make historical state current.

    D10ControlRange/1 =
        {kind:"records", scope:Scope/1, kinds:[ControlRecordKind/1],
         epoch:Token, revision:Counter, members:[ControlRef<K>/1]}
      | {kind:"cost_lineage", key:CostLayerKey/1,
         epoch:Token, revision:Counter,
         reservations:[ControlRef<reservation>/1]}
      | {kind:"occurrences", automation:ControlRef<automation>/1,
         epoch:Token, revision:Counter,
         records:[AutomationOccurrenceRecord/1]}

These are protected owner-specific range values inside the same InputDescriptor.ownerInput, not new D6 DependencyKey kinds or caller-supplied proof. A records range enumerates all records, including retained non-current states, for the exact scope and explicit nonempty sorted unique kind set. Cost ranges cover every reservation whose original attribution contains that exact cumulative key; occurrence ranges cover all generations of the exact Automation, including armed and handled states. Epoch/revision comes from the real control store's continuously maintained range fence. Every matching insertion, removal or relevant update advances that fence in the same transaction, including transitions into or out of a range; MAX refuses new ordinary work, never wraps. A complete same-cut scan proves positive and negative membership, including empty. An index alone never proves it. Transactions compare the fence and complete values; a racing matching insertion cannot escape CAS. No prefix, latest row or selected page is complete. Missing continuity pauses; known changed frozen dependencies conflict. Each array is at most 4096 entries and canonical metadata at most 16 MiB; exceeding either fails the complete prepare without partial effects.

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

Changes are canonical sorted/unique by after.binding.ref. Existing records require the complete original before; create requires none, a protected never-used allocated Ref, and a complete negative range proof. A true no-op is omitted. No delete arm exists. Each changed config/lifecycle revision is checked old+1 (creation=1); usage-only changes retain config revision and advance only the actual usage revision. All unchanged usage, liabilities, historical identity and non-target configuration are preserved. recordPins retains the full read images, required original external intents, planned A, Registry images and original budget configuration, not only rows changed. The plan is deterministically derived from the closed body, trusted principal, allocated refs and complete same-cut inputs. Preview affected/resourceUses exactly projects these real effects; it is not the write plan. Activation alone may carry the selector pair; all other bodies require none. Workspace body adapters may create/update only their listed workspace records and associated actual activation/Registry control effects. They never use this type to apply deployment_put, cost_reconcile, secrets, arbitrary callbacks or author-source edits. Existing workspace-qualified state actions on deployment records still follow §7's real H/domain dispatch, never this Workspace adapter.

Planning saves this fixed plan and dependency pins with the original D6 plan. Seal compares each unwritten control before-image and range fence again, then publishes the exact after-images, selector, deltas and D6 decision link in one P transaction. No protected control after-image is exposed or installed early during portable file installation. A recorded/rejected/terminal author outcome cannot masquerade as a D10 applied history. Once sealed, later config changes never replace the saved plan or its original result.

registryChange is some exactly when activation actually changes the Workspace portable Registry. Both pins are exact portable_metadata Registry component images authenticated under the actual D4 owner; the complete before/after snapshots, bindings and evolution must match the corresponding activation image and real Registry dependency. It uses the existing {kind:"registry",workspaceRef} component key. Such a change is complete/strict/portable under D6 §4.3 and receives one actual ChangeId/CP3, no source/H increment. With unchanged Registry, registryChange is none and a P-only Catalog/selector update is control_only. Other bodies require none. The immutable full canonical activation intent supplies the proposed Registry to the authorized control preview; no hidden historical record is publicly disclosed by the internal effect plan.

### 8.2 Exact Run admission and author-approval responsibility

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

These replace the illustrative untyped listings in D10; they are Core-owned internal values, not additional submit members. Complete ControlRef identities include storeIncarnation. LeaseRunUse is unique by full Run Ref, and originates only in the first protected-step admission CAS. It equals Run origin, exact admitted Lease and trusted admission time. Its creation and lease usage+1 are atomic. A later step or original planned recovery restores this same use without testing remaining>0 or consuming again; current exact Lease config, time, authorization, activation and stop still apply. Lost/unknown admission is unavailable, never a new admission. Claim/arming/queue creation consumes no Run use.

LeaseRunUse prose aliases are derived, not additional members: leaseId=lease.ref.id, leaseRevision=lease.revision and runId=run.id. Public run_state.admission projects exactly these original values and admittedAt; it does not replace the complete protected record. The storeIncarnation and Ref kinds are never discarded when comparing identity.

The author preview digest used by ApprovalUse and planned ConsentSpec is SHA-256 with UTF-8 domain D10-Author-Preview/1, NUL, then D3-CJ/3 of the complete original preview EffectManifest/2, including every item and DecisionKey. At each EffectBytes slot identified by the D7 closed typed decoder, replace the transport object with exactly {encoding,byteLength,payloadDigest:Sha256}, using the digest of that slot's complete exact pinned payload. The exhaustive typed traversal is source_change.before/after.bytes for each present/proposed SourceImage; conditional_source_change.before.bytes and result.bytes; semantic_extension.bytes; workspace_bootstrap.bytes; field_change.before and after; conflict_branch_source.bytes; and each canonical_plan.payloads element's before, selected and result. An absent SourceImage has no byte slot. The other item variants have no EffectBytes slot. Unknown variants reject; traversal never recursively guesses from member names or rewrites tokens inside decoded payloads. payloadDigest is ordinary SHA-256 over the complete exact payload bytes, without a transport handle or extra digest prefix. Immutable preparationBinding and usage-guard tokens inside original plans/requests remain; only this delivery epoch's transport handles are replaced. Every other member is unchanged. This is an internal deterministic digest projection, not a new public EffectBytes format; handleToken, cursor and delivery epoch are excluded. Core validates all bytes/lengths first and retains the complete original semantics/pins for exact comparison, not merely the digest. Reopening a planned preview under a fresh qualified delivery epoch therefore reproduces the same digest without reviving old tokens or changing the plan.

ApprovalUse is unique by the complete original DecisionKey, and its request derives exactly that key. The protected token must select the actual original D7 PreparedActionBinding/3 for that request; the semantic digest binds its immutable complete preview, independent of delivery tokens. Core retains the actual prepared record, complete footprint, source/dependency pins and original rule image, verifies the two §7 single-field branches, and creates the use from those facts. A field read or caller JSON cannot synthesize it. No second planToken field, free footprint callback, or duplicate author request is introduced. The count's key/approval equal the outer use. It is mutable protected eligibility stored beside the immutable prepared input, never hashed back into that input.

For unseen, planning under the real Authority Store serialization checks current qualification and reserved + consumed < maxSuccessfulCommits, then atomically saves the original plan and changes unreserved→reserved. Concurrent uses of the last slot have at most one winner. The same P seal changes reserved→consumed, including a true raw no-op. Saved replay does neither. Only an authoritative original terminal abort after all installation remnants are resolved changes reserved→released_terminal in that abort transaction. Other causes retain the reservation. Checked usage counters and their usageRevision advance with each real transition; the complete use set proves their totals, and no configuration revision resets them. Cost holds remain independent.

A protected supplemental planned approval is exactly its complete planned_approval record image. It binds original request A and immutable preview digest, original Lease/activation, granting principal and finite interval. Recovery may use it instead of a now-ineligible standing rule only after current authorization, complete original preview, source/business, Lease, clock and stop gates pass. It does not change the plan or release its standing reservation; final success consumes that original reservation. No arbitrary new approval is added to the immutable descriptor. Every eligibility record is found through the original request's protected association, and its exact current revision is checked at planning and seal; no client can bypass it by submitting the returned D6 request directly.

## 9. Deployment values and evidence

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

Publisher/package trust in this section is deployment-extension authenticity only. A D10 `trust` record, `PublisherIdentity`, `NamespaceClaim`, `PackageManifest` signature, package key rotation/revocation, deployment administrator, or package install order never creates or replaces D6 `WorkspaceTrustAnchor/1`, `WorkspaceTrustRootDeclaration/1`, `WorkspaceTrustDeclaration/1`, `DomainSealKeyHandle/1`, `CommitDomain` signing authority, Workspace policy authority, or author-write permission. D6 revision-seal trust is independently anchored and versioned inside the `WorkspaceAuthorizationBundle/1`; D10 package signatures cannot satisfy its bootstrap, registration, rotate/revoke, historical-validation, or `authorize_new_sign` gates.

    NamespaceClaim/1 = {
      namespaceId:D4.SemanticNamespaceId,
      ownerClass:"first_party" | "publisher",
      ownerId:NamespaceOwnerId/1,
      proof:EvidenceTicket/1
    }

`NamespaceOwnerId/1` is only a D10 validation name for the exact scalar `ownerId` already carried by the D4 Registry row; it is not a new namespace identity. For `first_party`, the value must be the exact reserved D4 tuple value for that namespace (`weftext.people`, `weftext.organizations`, `weftext.calendar`, or `weftext.library`). For `publisher`, it must be byte-equal to the accepted trust record's `publisherId` and that exact `(namespaceId,ownerClass,ownerId)` tuple must be proven by the `namespace_claim` EvidenceTicket. A D6 random Token/ticket can never occupy `ownerId`; proof is the separate `proof` member.

`NamespaceClaim/1` binds a verified PublisherIdentity/first-party root to the already-owned D4 semantic-namespace tuple. It creates no second namespace registry and cannot override `core`, `wf`, another reserved owner, or a different verified publisher. Install order, package/display name, enablement, and string coincidence are never owner proof.

    PricingMeter/1 = {
      meterId:LocalOperationId,
      numerator:Counter,
      denominator:Counter,
      maxUnits:Counter
    }

`denominator` and `maxUnits` are positive and meterId is ASCII-sorted and unique within one pricing record. Maximum cost uses checked integer/rational arithmetic and never binary float.

EvidenceTicket/1 is:

    {ticketId:Token,
     evidenceClass:
       "publisher_rotation" | "namespace_claim" |
       "account_control" | "pricing_contract" |
       "final_bill" | "never_started",
     evidenceDigest:Sha256}

A trusted adapter creates the ticket and internally binds complete original evidence, principal, scope, provider/account/attempt. An ordinary caller cannot assert verified or reuse one evidenceClass as another.

Secret plaintext uses only the trusted secret channel:

    D10SecretStageRequest/1 = {
      kind:"d10_secret_stage",
      wireVersion:1,
      requestId:Uuid,
      account:Binding<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretBytes:Bytes
    }

Only H may stage. secretBytes never enter the ordinary control canonical intent, preview, log, transcript, or Workspace. Success returns:

    SecretStageTicket/1 = {
      ticketId:Token,
      account:ControlRef<external_account>/1,
      audience:ContributionBinding/1,
      usageKind:"authenticate",
      secretVersionId:Token
    }

Staging has its own same-principal/store/requestId stable key. The trusted secret store replays the original ticket only when it can prove the exact same secret input. The ordinary database transaction publishes only the immutable secret version referenced by the ticket.

DeploymentControlDecision/1 is the protected internal host-domain success record; its public projection is the §7 applied response:

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

Only an atomically successful commit saves a decision. Preflight/authorization/CAS/evidence/overflow failure saves no applied decision. changes exactly covers configuration changes and is empty for a true no-op. Usage/account/reservation deltas commit in the same transaction as the decision.

## 10. Cost reservation and recoverable settlement

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


Each `CostReservation/1` belongs to exactly one billable `attemptId`, one actual `cost_account`, one grant, one pricing binding, and one currency. Run/Lease/Automation/Workspace/deployment limits checked during the same admission are layered ceilings/projections, not additional actual accounts for this reservation, and the same cost is recorded once. If one operation truly creates separately attributable charges against multiple actual accounts, it creates separately attributable attempts/reservations/evidence for those accounts; any required group admission is atomic over those reservations without turning one reservation into a multi-account object.

Every billable attempt, including model calls, ordinary or read-only tools, network requests and reconciliation, has exactly one protected `CostBudgetAttribution/1` per real reservation. Core derives it from the actual authorized Run and original reservation, never from a model, a display name, an audit summary or a caller-supplied billing owner. The same admission transaction saves it with the reservation before any billable execution. It is immutable attribution attached to that one charge, not a second account, success ledger, or new public reservation payload.

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

The array is exactly run, lease, automation if and only if the real Run origin is automation, workspace, grant, account, in that order with no duplicates. Interactive Runs have five layers and Automation Runs six. `reservation` and `attemptId` equal the original reservation; grant/account bindings exactly equal its grant/account. All ceilings use its currency. Workspace and origin equal the protected Run and original LeaseRunUse; the Lease is the Run's actual admitted binding. The automation layer addresses the same Automation Ref as origin, but its binding and definitionRevision identify the budget configuration actually checked at this attempt's admission, which may be a later configuration than the immutable originating definition. This never rewrites the Run's original invocation/claim/approval. The workspace layer addresses the one permanent workspace_budget identity for this Workspace/control domain. Required historical configuration values and their exact cap selection are retained with the attribution; a current object cannot substitute for a missing original binding.

Core freezes finite Run BudgetCaps in the original protected Run record when creating it: an Automation Run takes its originating immutable Automation definition's BudgetCaps; an interactive Run takes its Run-targeted Lease's BudgetCaps. This first profile has no independent runtime Run-budget expansion/reset operation. The run layer binding is the actual Run creation binding that owns this immutable budget fact; later execution/lifecycle revision changes do not invalidate it or reset use. Every attempt also checks the current eligible Lease, current Automation budget if applicable, current Workspace budget, grant and actual account. An increase elsewhere cannot evade the fixed Run cap, while a current narrower layer still restricts new attempts. These checks confer no new Lease/approval authority and do not unblock an original planned request whose other current gates fail.

The cumulative key for each layer is exactly its kind, complete owner ControlRef, complete actual cost-account ControlRef and currency. Compare complete canonical values, including storeIncarnation. Configuration revision, Automation definitionRevision, grant renewal, requestId, list position and process/cache epoch are not cumulative identity. The account layer covers all reservations for that actual account, across Workspace and grant boundaries; the grant layer covers its original grant; other layers select only reservations whose saved attribution contains that exact owner key. Thus two Automations sharing a grant do not share an Automation ceiling, but still compete for the same grant/account and applicable Workspace ceilings.

There is no periodic reset in this generation. Budget edits, new definitions, enable/disable, archive, restart and grant replacement preserve original held/spent. Raising a cap adjusts the same lineage; lowering it below that lineage's proven spent plus held is rejected with management budget_exceeded. Removing an account from a BudgetCaps list disables new attempts through that layer and preserves all old liabilities; adding it again compares the same prior totals. Creating a real new Run, Lease or Automation gives that new non-reused owner its own layer but never moves old reservations, and common Workspace/grant/account layers continue to count them. workspace_budget identity is created once per Workspace/control domain and cannot be replaced to reset usage. A cost_account's currency is immutable after creation; a different currency requires a genuinely new account with separately retained original liabilities, not implicit FX or renaming.

All monetary projections are defined from a complete authoritative reservation/attribution cut: reserved and uncertain each contribute the full upperBound to held, settled contributes actual to spent, released contributes zero. No other state, partial bill or temporary missing record implies zero. Before admitting a new reservation, checked arithmetic must prove spent + held + the proposed bound does not exceed every actual applicable current cap, together with all existing per-attempt/count/non-monetary limits. If one actual operation needs multiple separately attributable accounts, the entire required group passes in one admission transaction or none is admitted; each charge remains independently attributable once.

Admission, budget configuration changes, and settlement use the same actual Authority Store serialization boundary. Admission compares all relevant configuration bindings, the Run's immutable budget provenance, original grant/account usage revisions, stop/current authority, and the complete reservation/attribution membership cut, then atomically writes reservation, attribution, actual held/count deltas, retained evidence and audit. A range/phantom proof or the actual shared account usage fence must cover every concurrent insertion affecting these keys; checking only previously returned rows is insufficient. Concurrent attempts for the last capacity have at most one winner. Run/Automation/Workspace monetary projections need no new independent usageRevision or balance ledger: indexes are rebuildable and must be verified against the same protected facts. A deployment with no such single atomic boundary cannot advertise this admission path as available or combine independent database successes into one success.

An existing attempt restores its original reservation and complete attribution before any new-admission branch. Unknown existence/continuity never creates a replacement attribution or attempt. Every original configuration/pin and unresolved liability remains retained through Run termination, Lease expiry, grant retirement, authority recovery, stop and ordinary GC. Retained proof/compaction may replace storage only if it still proves exact per-key held/spent, original reservation/settlement deduplication and every unresolved responsibility; it cannot turn prior spending into zero or rely on current Run names/configuration.

Settlement recovers the original complete attribution before applying the original evidence/revision CAS. In the same transaction it removes upperBound from held on every saved layer key and, for settled, adds the one actual amount to spent on those same keys; released adds no spent. It updates the original grant/account projections and all derived layer indexes or their invalidation, evidence and audit atomically. Current configuration or definition changes do not reassign this charge; a later configuration of the same owner/account automatically sees the updated same-lineage totals. Old configuration/grant eligibility to start work is not required for evidence-based reconciliation, while the reconciler still needs actual current authority for the original account. Missing attribution/continuity returns state_unavailable and retains the full liability; proven protected contradictions return integrity_conflict on control read/settlement and the applicable control_conflict before Run execution, after ordinary visibility gates. Hidden Run/Lease/Automation details are not disclosed through the reservation projection or error.

Exact settlement replay does not remove held or return capacity twice. Non-final evidence leaves uncertain at the full bound; reliable final 20 after uncertain 90 changes every original applicable layer by held -90 and spent +20, returning 70 once. Actual above upperBound follows the existing overcharge/freeze rule and does not increase a ceiling through reconciliation. Author abort, TTL, cancellation or terminal Run state never substitutes for never-started/final-bill evidence. The public reservation_state still returns only the original CostReservation shape; this protected attribution is not automatically exposed to someone who can merely inspect the account.

`actual` is some only when state=settled and is none in every other state.

`attemptId` and `reservationId` are never reused.

State machine:

    reserved -> settled(actual) | released | uncertain
    uncertain -> settled(actual) | released

settled and released are terminal. uncertain is recoverable non-terminal and continues to occupy the complete upperBound.

released is legal only when reliable never_started evidence proves that billable execution/send for this attempt never began. An actually sent attempt whose reliable final bill is zero is settled(0), not released. Reliable final_bill must be attributable to the same reservation/attempt/account/currency/pricing and produces settled(actual). For reserve100→uncertain→final bill20, the only result is settled(20), returning 80.

    CostSettlementDecision/1 = {
      reservationId:Uuid,
      attemptId:Uuid,
      priorRevision:Counter,
      kind:"settled" | "released",
      actual:Option<Money/1>,
      evidence:EvidenceTicket/1
    }

`settled` requires `actual=some`. `released` requires `actual=none` and `evidenceClass=never_started`. `final_bill` can produce only settled. Wrong attempt/account/currency, non-final evidence, an aggregate bill that cannot be uniquely split, or insufficient continuity leaves uncertain with the complete bound and returns `state_unavailable`. Malformed accepted-adapter output follows its existing invalid-output contract and never guesses a result.

Reconcile uses CAS on the expected reservation revision. Success atomically appends CostSettlementDecision, updates reservation state/revision, grant/account held/spent/available projections, and evidence/audit. Exact evidence/prior-revision/derived-decision replay does not return capacity twice; concurrent different decisions have at most one CAS winner. actual above upperBound follows the existing overcharge anomaly/freeze path and normal reconciliation never raises the ceiling.

Administrator-entered zero, effect idempotency, business rollback, author abort, TTL, or Run terminal state is not released/settled evidence. ApprovalUse count, LeaseRunUse, and cost reservation remain three separate domains.

## 11. Emergency stop and linearization

Public stop is a specialized safety transaction outside ordinary control prepare:

    D10EmergencyStopRequest/1 = {
      kind:"d10_emergency_stop",
      wireVersion:1,
      requestId:Uuid,
      scope:Scope/1,
      target:
        ControlRef<automation>/1
      | ControlRef<run>/1
    }

Generation one requires `requestId == target.id`. requestId is a deterministic dedup coordinate for the exact target, not a caller-selected capability. The request still carries the complete target Ref; no text, display name, bare UUID, or ToolValue is promoted into it.

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

The protected stop stable key is `("D10-Emergency-Stop/1",target.storeIncarnation,target.kind,requestId)`. `requestId==target.id`, target/latch storeIncarnations equal StopOwner.storeIncarnation, and one exact target has exactly one latch. workspaceRef is the target's real Workspace from protected control state. D10 storeIncarnation is an internal persistent control-domain incarnation bound to the actually opened D6 authority-store backend; it is not a frozen D6 public API name, WorkspaceId, filesystem path, or authority token.

W uses exact target Workspace scope and H the actual deployment storeIncarnation plus target-specific current host authority. The strict Stop-only §7 mapping reads **one** persisted W-scoped latch/StopOwner image under two independently authorized query scopes, never two Stop records or results. First stop, lost-response query and r5→r6 preserve the same irreversible state, result and reserved capacity.

Before first enable/admission, object creation atomically reserves one durable latch, one StopOwner association, one audit/result slot, and one unit of safety-sequence capacity. If reservation fails, the executable object cannot become manageable/runnable. Ordinary control prepare, configuration Counter space, budgets, approval/lease counts, cost, and executor quotas cannot consume this capacity. Stop requires no target configuration revision and no preceding ordinary management write.

    ExecutionStopLatch/1 = {
      target:ControlRef<automation|run>/1,
      state:"open" | "stopped",
      stoppedBy:Option<HostOrWorkspacePrincipal/1>,
      stoppedAtControlSequence:Option<Counter>
    }

Open requires both options none; stopped requires both some. The transition is exactly Binding revision 1/open → revision 2/stopped. There is no clear operation. Safety sequence reservation obeys `reserved <= MAX-issued`, MAX=2^63-1. First stop atomically executes `issued:=issued+1`, `reserved:=reserved-1`, writes the sequence/actor/latch revision, immutable stop result/audit link, and required invalidations. Thus capacity exhaustion may reject creation of a new executable target, never the first stop of an existing reserved target.

    D10EmergencyStopReceipt/1 = {
      kind:"d10_emergency_stop_receipt",
      wireVersion:1,
      requestId:Uuid,
      target:ControlRef<automation|run>/1,
      latch:Binding<stop>/1,
      state:"stopped"
    }

The receipt requires latch revision 2 and is a deterministic projection of the single latch transition; it is neither a D6 author receipt nor a second success ledger.

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

`not_applied` requires proven latch revision 1/open at that read linearization point and is not a promise about a later stop.

Stop/result order is: closed decode and D1 automation.stop gate → current authenticated principal plus exact W/H scope and target/stop disclosure authority → authority/fence/custody → protected stable-key/target equality → latch/result continuity → transition or read → final current disclosure gate. Hidden, missing, wrong-scope, and wrong-store are `not_visible`; unprovable authority/fence/custody is `authority_unavailable`; proven corruption is `integrity_conflict`. After visibility, the same stable key bound to a different target is `control_conflict`. Unprovable latch/result continuity is `state_unavailable`, never not_applied or a fresh stop.

If target configuration r5 is stopped and the response is lost, then a separately legal configuration changes the target to r6, retry with the same exact target/requestId returns the original receipt. Current r6 neither invalidates nor rewrites it. Loss of result-disclosure authority instead returns `not_visible` while the latch remains stopped.

Linearization rules:

1. New Run admission and the safety transaction share the same Authority Store serialization domain. Admission checks latches and atomically writes occurrence claim/LeaseRunUse. Stop first means no admission; admission first preserves its consumed count and all later protected steps still check stop.
2. D6 final author commit rechecks every referenced stop latch inside the actual final author transaction while holding the same write-serialization boundary. Commit first preserves the committed result; stop first prevents the new commit.
3. External transport and stop share one send fence. Before the first irreversible handoff, Core has frozen the immutable ExternalEffectIntent and exact ExternalExecutionBinding, rechecks stop plus approval/grant/secret/cost bindings, durably stores send-attempt/started evidence and holds, and retains the fence through the first real send handoff. Stop first means no send; handoff first preserves the original send attempt and may later be outcome_unknown.
4. After crash, restore latch, send-attempt evidence, reservations, and original decisions before restoring an executor. Unknown evidence can never become cancelled or justify a new effectId, idempotency key, or OperationId.
5. Stop does not block currently authorized authoritative abort, cost settlement, evidence/audit retention, or reference-safe cleanup. Those operations do not regain executor authority.
6. Temporary disable, Lease expiry, temporary authorization loss, or ordinary cancel is not irreversible abort proof.

The stop safety transaction is a specialized closed write in the same managed Authority Store, not D10ControlPrepare and not a D6 author transaction. The coordinated D6 amendment only defines how D6 final/planned author work consumes the latch and returns `execution_stopped/preflight` or the original authoritative-abort result. Stop never fabricates ordinary control history, never rolls back committed author facts or an already-sent external effect, and never releases cost merely because execution was stopped.

## 12. Policy/3 and current D6 Profile4/Plan4/Genesis2 consumption

The **single current owner** is D6 Control §§10.2/20.2 and FC SCHEMAS §9: fresh `WorkspaceBootstrapProfile/4` (`d6_bootstrap_profile`,wireVersion=4), `WorkspaceBootstrapPlan/4` (`d6_workspace_bootstrap_plan`,wireVersion=4), `WorkspaceTrustGenesis/2` (version=2), current Policy/3 and the D8 mandatory `D8PresentationPolicyBootstrapInit/1`. D7-REGISTRY-QUALIFICATION B13 consumes these exact versions. D10 contributes only the workspace-scope no-argument `d10_control_self` grant for a **properly issuer-authorized new family**, not a second bootstrap producer or a changed six-member Profile4 wire.

The frozen profile/2 non-Field set remains `workspace_state, entity_state, locator_state, source_read, source_write, body_write, node_control, node_create, resource_read, resource_write, annotation_read, annotation_write, lifecycle, registry_admin, binding_admin, policy_admin, export, repair, audit, source_envelope_state, commit_sequence_state`. A newly issued Profile4 initial Policy/3 creator grant is that set plus D6's `replica_register, replica_retire, conflict_read, conflict_resolve, execution_custody_admin, structure_state, portable_frontier_state`, D10 `d10_control_self`, and the original complete target Registry Field read/write grant derivation, with deny empty. Each has D6's actual scope, including CommitDomain-scoped commit_sequence_state. New Fields/future capabilities are not implicit. Only current `administer_issuer` can explicitly update issuer profile for subsequently issued families, and `allocate_workspace` is independently required. Existing Workspaces obtain d10_control_self only by original current `policy_admin` complete Policy/3 transaction; Field-only principals cannot self-grant, and neither resume nor login revives creator powers.

For a **fresh unseen** create_workspace/fork_workspace, D3 owns the original authenticated proposal, issuer and target custody/allocation; the actual Registry/Policy3 and complete fork source history are validated **before** the sole D3 planning CAS. Create uses the issuer-proved empty target and eligible frozen Registry seed; fork preserves and maps the complete source Registry evolution/retirement, configuration, origin and source authority rather than treating itself as create. One original OperationId, DecisionKey and final P are used. Trusted host stages the root keypair and two server DomainSealKeyHandle/2 values; caller never supplies them. `WorkspaceTrustGenesis/2` has **exactly two** signed `WorkspaceTrustDeclaration/2` authorizations under the same DecisionKey and eventual activation ChangeId: revision-token profile revision 1 then source-transform profile revision 2; declaration 2's predecessor hashes exact declaration 1, each has its own PoP/root signature. WorkspaceAuthorizationBundle/2 has authorizationRevision 1/trustRevision 2. Neither profile is activated from a one-profile prefix.

Plan4's **mandatory** `initialPresentationPolicy` freezes the D8 owner `D8PresentationPolicyBootstrapInit/1` from the original issuer/custody-proved **empty history**, not from a missing file. Its protected before head stamp starts at revision 1, proposal has `parents=[]`, `revision=1`, `defaultPresentation="separate"`, and one typed `presentation_policy_change` preview with `committed=null`. Planning and staged operations preserve original plan/init bytes, both key handles, root/PoP proofs, Registry/series/period/source/Policy pins; they **do not** materialize any ChangeId-dependent D8 /2 record/hash/pin/address/effect/receipt/outbox/head or usable trust key. Only the **one original final P** rechecks issuer/target custody, all original source/Registry/Policy/trust/CP4 dependencies, allocates its original ChangeId and atomically publishes the D8 /2 record and exact hash/pin/address, committed effect, original receipt/Notice3/CP4/ChangeRecord1 association, outbox, one-head transition, Workspace activation and **both** staged→usable handles. Loser/abort leaves none of these partial effects; no second CAS, second P/ledger, later SetRequest or repair window exists.

Actual saved/planned/unknown Plan4 resumes its **identical** D3 plan/OperationId/DecisionKey, frozen init, staged handles, source pins and original P; it does not sample latest seed or generate replacement root keys. Genuine historical Profile1–3, recorded Plan1/Plan3/Genesis1 and real old decoders retain exact bytes, pins, issuer grants and recovery; undeployed predecessor naming does not justify a migration layer. Ordinary managed copy is **not** bootstrap. Continue/failover/restore/committed replay preserve their actual recorded/current policy, presentation heads, trust and original receipt without restoring creator rights. Product/runtime remains UNRUN.

## 13. Public capability and first-public D6 error compatibility

Candidate capability IDs defined by D10 and discovered/published by D1 are:

- automation.manage
- workspace.extensions.manage
- deployment.external.manage
- automation.stop
- automation.author_submit

These IDs must enter the selected D1 contractMajor’s official capability catalog and still pass every real release/surface/policy/version/component/configuration/reachability/health gate. Document acceptance or coordinated design activation does not make them runtime available.

The first jointly specified public unattended author-submit contract directly includes D6 approval_unavailable/preflight and execution_stopped/preflight. There is no anonymous old/new D10 error profile. Unknown capability ID remains D1 unsupported_feature; a disallowed component-version combination remains D1 incompatible_version. Fixed D6 Policy/1/2, bootstrap profile/1/2, and historical saved-decision decoders are not removed by deleting unpublished D10-profile wording.

## 14. Atomicity, audit, and retention

A Workspace control change that affects an author decision enters the original D6 authority-store transaction. Deployment host control changes only Deployment records in the closed host transaction for the same store incarnation. No contract assumes atomicity across independent databases, HTTP services, or ATTACH/WAL.

Every successful protected control operation atomically saves record/config change, usage/account delta, decision/receipt linkage, dependency invalidation, evidence pins, and required audit link. Ordinary protected execution fails closed if its required audit-start fact cannot be durably stored. Emergency stop uses a separate safety slot reserved at enable/admission and is not blocked by ordinary audit/budget quota exhaustion.

Retire/archive does not delete a record still referenced by a saved decision, planned recovery, unknown external effect, uncertain cost, evidence, tombstone/migration, or active binding. Capacity exhaustion may reject a new ordinary operation; it cannot delete dedup history so an old requestId runs again.

## 15. R05 acceptance counterexamples

At minimum verify:

1. A narrow-Field user with d10_control_self, actual Field authority, and an administrator-issued cost grant can create a finite owned automation; missing self capability or grant rejects clearly.
2. An r5 stable-key write committed but response was lost; another request changed the object to r6; retry returns saved r5 result after current disclosure authorization, without duplicate creation or false stale rejection.
3. Same key with changed target/body/expected revision returns control_conflict; delete/recreate under the same display name never captures the old request.
4. Grant renewal/revision does not clear spent/held/attempts; a replacement grant does not erase old reservation/account liability.
5. reserve 100, actually send, crash→`uncertain`; a same-attempt final bill of 20 produces `settled(20)` and returns only 80; a final bill of 0 produces `settled(0)`; only never-started proof produces `released`.
6. Two reconcilers race one reservation: at most one CAS winner; response loss after commit replays without double return.
7. Every ordering of stop versus Run admission, D6 final commit, and external send fence; settlement/authoritative abort/evidence cleanup still works after stop.
8. A profile/2 family does not gain d10_control_self after software upgrade; explicit profile/3 affects only later families; existing Workspaces require explicit policy_admin grant.
9. Unavailable connector Contribution does not disable same-package schema/template/data-pack Contribution.
10. Calendar/Library/People/Organizations PackageId, module Contribution, schema Contribution, and D4 namespace/Facet map one-to-one without type conflation; a same-named third-party package gains no first-party D4 owner.
11. Design accepted but release not shipped, Mobile unsupported, policy denied, component combination disallowed, or provider unhealthy still yields the real D1 unavailable result.
12. After a caller loses result-disclosure authority, same-key result query is not_visible while the saved decision remains preserved.

Scheduling coordinates and missedWindowSeconds reuse the exact CanonicalDecimal string grammar with the coordinate origin/key budget and positive finite-duration bounds in §16 respectively; they are not ToolValue and require no fabricated ToolType.

## 16. Stable subscription, occurrence and scheduling decision

This section owns the closed scheduling values referenced by AutomationSpec and RunOrigin. It changes the unpublished candidate definition; an actual historical claim or Run retains its original decoder, complete key, configuration, outcome and recovery responsibility. Missing historical evidence is never backfilled from the new definition.

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

Creation accepts initial only and starts subscriptionGeneration=1. An existing Automation accepts continue or explicit replace only. The selection arm matches definition.schedule. Once has no source. Recurrence selection binds the complete currently qualified Observation for the definition's actual owner and both exact Field occurrence keys. Current authority and Registry qualification precede source/Entry inspection. A source needing managed inner revision must first pass its original explicit admission; configuration never writes or implicitly admits it. The input sourceToken belongs to the original prepare cut, not a permanent future-currentness credential.

The source coordinate is the exact Gregorian UTC second count since 1970-01-01T00:00:00Z, derived with integer/decimal arithmetic from a D4-accepted instant. Once uses at; recurrence uses row.originalStart, never a replacement's final start. CanonicalDecimal removes redundant zeros/exponents and negative zero, preserves every fractional digit, and permits negative coordinates. Do not round to a machine float or microsecond, or restrict conversion to a host date library's year range: an accepted local year 0001/9999 can map into UTC year 0/10000. Equivalent offsets/fraction spellings produce the same coordinate; different originalStart values remain distinct even when moved to the same final start. Date-only input has no automatic midnight conversion. A complete canonical key is limited to 131072 UTF-8 bytes; exceeding the budget rejects without truncating or replacing equality with a hash.

Key equality compares the complete closed value. Key ordering is complete Automation Ref canonical bytes, numeric generation, fixed kind order once before recurrence, then exact numeric UTC coordinate. The key includes no definitionRevision, source revision/token, Field occurrence key, projection horizon, Query invocation/operator/parent key, cache epoch or final due time. subscriptionGeneration is a positive checked Counter within the one non-reused Automation, not a new author identity namespace. Initial/replace canonicalizes fromOriginalStart into a fixed inclusive original-coordinate lower bound; changing a projection window cannot change that bound.

The actual protected source proof is retained in the following closed internal records. They confer no public read or execution authority and add no public control-record kind:

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

For recurrence, sourcePin is exact_source_document for that Observation's complete managed SourceVersion and owner. metadataPin is the exact corresponding portable_metadata image establishing the owner’s lifecycle/identity at the same cut; another head's metadata is invalid. DependencyProof belongs to the current observerDomain/cut and completely covers actual source/entity, authorization, Registry, temporal rules and all other business ranges needed by the D4 producer. RegistrySnapshot and, except at genuine bootstrap, its required evolution proof construct the real immutable ValidatedCatalogContext. recurrenceContext is the actual complete finite D4 context, with its rule provenance, exact revisions and coverage. Raw bytes or individual decoded Entries do not replace current D4 acceptance of real Event/range-note facets and the selected calendar/recurrence and calendar/range Entries. Initial evidence is immutable; checkpoint advances only after the continuity proof below. Pins are exact typed Core pins, token-sorted/unique, at most 4096 per checkpoint update; the whole update has a finite 16 MiB canonical evidence budget excluding separately capacity-reserved immutable source/component payloads. Incomplete or over-budget proof cannot partially advance the checkpoint.

Continuity is a real protected execution dependency, not an I cache fact. Every intermediate sealed source/control state between the previous checkpoint and the new current cut must prove the selected owner continuously live, the required real facets present, both exact Field keys continuously present, and their complete scheduling business values unchanged. Business value comparison uses D4's decoded canonical recurrence/range values and the actual semantic definitions/rule providers; it ignores only source formatting and Entry note/qualifier/provenance members that do not participate in those scheduling semantics. It does not ignore a changed recurrence, range, relevant schema, calendar rule, timezone or tzdb rule version. Reused key/final value after deletion/recreation, temporary facet/lifecycle removal, or a changed-and-restored rule is not continuity. A different unrelated Entry/body edit may advance checkpoint automatically when the complete actual chain proves these invariants.

The producer must retain either the complete verified D6 ChangeRecord/InstallationNotice/ContentCompletionProof chain with every necessary intermediate source/metadata component, or a protected gap-free continuously consumed witness derived from exactly that chain and the relevant actual control/rule history. continuityPins pin those original versioned records and their complete source/control evidence under their existing decoders; they are not caller-supplied statements, bare larger Frontier numbers, or a free proof-map protocol. The witness must prove every causal branch and relevant control transition through the saved checkpoint, with no omitted interval, reset or unobserved predecessor, and preserve the original source selection and rule identity. Compaction may discard an old payload only after a retained protected witness still proves those same invariants and all unresolved responsibilities. Equal final source/hash, a rebuilt index, a changed read-delivery token, or a provider saying synced is insufficient. observed_only proves its retained B/N, not absence of an unseen C; an external/observer gap or unavailable intermediate history cannot be certified continuous. A mere horizon extension or fresh local observation is not by itself a business-rule change; the actual unchanged provider/rule identity and newly needed complete coverage must be proved. Current source/field/owner authority is checked before internally verifying necessary history, and no hidden historical content is returned or added to the Lease.

The D6 producer retains these subscription dependencies with protected execution recovery, using the existing recovery retention class and actual control-store responsibility. It may eagerly maintain a protected continuous witness while consuming verified changes, or verify retained complete history before the next scan. Neither path may reconstruct it from I after the correctness evidence is lost. Failure to prove history pauses new scheduling as state_unavailable; a proved source/selection/rule discontinuity is binding_changed in the scheduling domain and requires explicit replace. In management prepare, the same proved mismatch is control_conflict, never the Run-only binding_changed. A replacement fixes a newly qualified source and lower bound, without pretending to restore the old chain. Original claims and unknown requests remain recoverable. This joint D6 retention/consumer contract must actually be integrated before this path is available.

continue retains generation and lower bound and requires the same schedule arm: once must retain the exact normalized at coordinate; recurrence must retain the actual owner and both selected keys and prove the complete continuity above. A different arm, once at, source selection or scheduling business value requires explicit replace. Changes to invocation, Lease, approval, budget, missed policy/window or finite projection horizon/limit may advance definitionRevision and the configuration binding but cannot re-key a previously handled occurrence. A true same-definition no-op does not invent a revision. enable/disable/archive likewise never re-key history. replace atomically retires the old generation's authority to create new claims, checked-increments generation, and fixes the new source/lower bound. MAX rejects without wrapping. Existing claims of every generation retain their original Run, immutable origin/definition, invocation, Lease/approval associations, queued/blocked/unknown/terminal state and exact saved requests.

The sole definition activation point is the successful original automation_configure control commit in the same actual Authority Store serialization domain as occurrence decisions. A claim winning before that commit retains its old definition; a first claim winning after it uses the active new definition, including an eligible previously unhandled past coordinate in the finite missed window. An armed item is not yet a claim and does not reserve the old definition. There is no wall-clock guess or cache-scan ordering rule. Config, source-checkpoint updates, arming, claims, cross-generation exclusions and stop/custody checks share the real transaction fences; a competing source/control change invalidates or retries the uncommitted scan without executing a partial selection.

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

Recurrence proof stores the actual complete D4 outcome, including projectionIdentity, all rows and readSet; the exact originalStart identifies one unique row. Its owner, managed source revision, both source keys, Registry/read binding and finite horizon equal the evidence actually used. It grants no D7 Query completeness or execution-custody authority. dueUtcSeconds is the exact UTC point of that row's final range.start; once uses at. Filtering uses the existing replacement-aware D4 algorithm so an exception moved into the window from outside is included. Order eligible items by exact final due, then SourceOccurrenceKey's fixed tag/numeric coordinate; different original coordinates never collapse merely because final due is equal.

armed is a durable pre-state in the same occurrence store, not a Run or LeaseRunUse. Core may create it only after current qualification and a trusted time reading strictly earlier than due, for a real future eligible coordinate at or above the subscription lower bound. It retains that arming time and proof. clockEpoch binds the actual trusted time source; the atomic arming point revalidates armedAtUtcSeconds < due and cannot backdate an already-due item using the earlier computation time. Unprovable clock or transaction-point qualification advances no state. It cannot be synthesized for an already-past item to bypass missed policy. A complete future projection may arm the earliest finite prefix allowed by queue/storage budget; that explicitly bounded lookahead is not a claim that all future occurrences were armed. Once an armed item is due, it follows normal due handling even after late wake-up or restart; due<now does not turn it into a missed item. Before armed→claimed, current subscription/source/rules/authority are revalidated and the current active definition is frozen. The transition atomically creates exactly one original Run and its immutable origin, replacing the armed record; no maxRuns is consumed until the original Run-admission CAS. An old generation's armed records cannot claim after replace and do not count as previously handled coordinates.

AutomationSpec.missedWindowSeconds is a finite positive CanonicalDecimal duration, at most 31557600 seconds. At one trusted scanTime, the catch-up interval is exactly [scanTime−missedWindowSeconds, scanTime). Only due, unarmed, previously unhandled eligible coordinates in this interval use missedPolicy; already claimed/skipped and due armed records are restored or handled first and never reclassified. A once item older than this interval is durably skipped with outside_missed_window. Recurrence candidates older than the interval need not be enumerated as an infinite skipped history; the current fixed schedule never executes them from an expired window. They are not falsely recorded as individually claimed, and a later explicit replacement still follows the new source/lower-bound and actual handled-coordinate exclusion rules.

For recurrence, the configured horizon is a finite authorized projection boundary, not occurrence identity or a substitute for the catch-up interval. The D4 projection used for a decision must completely cover the entire applicable catch-up interval and all due armed items being processed; if its configured horizon, temporal coverage, output/work/evidence budget cannot do so, the decision fails with no partial dispositions. Future lookahead remains within that horizon. A caller cannot narrow a page/window to suppress a newer missed occurrence. The required finite current projection and current source/rule proof are rebuilt under the current cut, while previously saved claims always restore their original responsibility before any new-scan qualification.

One missed-window selection is one atomic business decision in the existing Authority Store. Freeze scanTime, exact window, active definition, complete eligible set and ordering, prior-handled exclusions, policy and complete proof; then, in one transaction, persist every resulting disposition together with the selected Run/origin and any queue/capacity reservations. skip marks every eligible missed item skipped. run_once claims only the greatest eligible missed item in the due/key order and marks all other eligible missed items skipped. If an item already belongs to an original claim or handover exclusion, it cannot become a second new Run. The complete decision commits or none of its skipped/claimed rows do; crash between writes cannot lose the selected Run or let a later rescan select a second older item. queueLimit counts queued/active claimed Runs, and protected capacity also bounds armed/lookahead records; capacity exhaustion fails the whole applicable decision. Claim creation alone consumes no Lease maxRuns. Recovery of a committed decision restores its exact rows/Run; an uncommitted computation is discarded and recomputed from the now-current complete cut.

Before a new generation claims a coordinate, the same transaction proves the complete earlier-generation handled set for this Automation. Any prior claimed, skipped or handover-skipped record with the same exact UTC original coordinate excludes a new Run, conservatively across once/recurrence arms; write handover_skipped pointing to the original non-handover handled key. Resolve a chain to that original record, reject cycles/contradictions, and never follow a display name or current source key. Unknown/terminal/blocked original Runs all count as handled. A prior armed-only record does not. The old generation can update an existing claim's outcome but cannot create a previously absent claim after retirement. Complete range/phantom protection prevents races; a missing record/index is not proof of empty history. GC/compaction must preserve exact original key, coordinate, disposition and original Run/unknown responsibility for this exclusion. An intentional rerun of an already handled coordinate requires a genuinely new Automation or separately confirmed interactive Run, not changing this Automation's definition or generation.

Every new protected step still applies current authorization, exact Lease/activation/approval/time/budget/stop and original request recovery rules. Saved/planned/unknown work is never replaced merely because the subscription changed or its current source cannot be requalified. Public automation_state exposes generation and the lower bound as scheduling values; public run_state.origin carries the exact real claim key. Full source proof, hidden prior claims, rule/history pins and execution custody remain protected. Unauthorized nested values return the original nondisclosing result, not a synthetic empty schedule.

D6 Storage §7.2.1 owns the actual closed ScheduleContinuityWitness/1 and ScheduleContinuityStep/1 producer, its registered finite transition inbox, atomic checkpoint/invalidations and last-reference GC. continuityPins uses those exact typed artifact decoders or the complete original typed chain; the accepted D10 predicate in this section is unchanged. This concrete candidate still requires independent joint acceptance and implementation evidence.

### 16.1 Complete execution responsibility payloads

These closed protected values are the actual D10 payloads consumed by D6 ExecutionResponsibilityRecord/2. They preserve original control identities and facts, rather than creating another account, occurrence store or author ledger. Arrays use full canonical keys, are sorted and unique, and are complete for the declared execution domain, including empty ranges. A transfer is bounded to 4096 entries per array and 16 MiB of canonical metadata plus separately reserved payload pins; an over-budget transfer pauses without dropping records or disabling unrelated ordinary source work.

    CostLayerKey/1 = {
      kind:"run"|"lease"|"automation"|"workspace"|"grant"|"account",
      owner:ControlRef<K>/1, account:ControlRef<cost_account>/1,
      currency:CurrencyCode
    }

    CostLayerTotal/1 = {
      key:CostLayerKey/1, heldMicroUnits:Counter, spentMicroUnits:Counter,
      reservations:[ControlRef<reservation>/1]
    }

K is respectively run, lease, automation, workspace_budget, grant or cost_account; account-layer owner equals account. These totals are verified projections of the complete reservation/attribution set in §10, not writable balances. Every matching reservation, including settled history, participates once; unknown membership cannot mean zero. Exact original layer keys survive all configuration changes.

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

The fence token selects the real protected send-fence record for this exact sendAttemptId, store, holder, intent and stop set. It is not a caller capability. prepared has durable holds but no handoff; started is durably recorded before irreversible handoff and conservatively becomes outcome_unknown after an unproved crash, never not_started. not_started requires the original trusted never_started evidence. response is the complete response/effect evidence decoded by the exact accepted contribution contract bound in the immutable intent; raw bytes alone do not prove success, failure or final billing. Unknown or unsupported evidence keeps outcome_unknown and all holds. Reconciliation records the verified response under that same attempt; a permitted retry creates a separate attempt under the same intent only after §17 of D10's exact idempotency/no-effect qualification. Cost settlement has its own evidence and cannot be inferred from this outcome.

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

The automatic branch's artifact pin strict-decodes the exact original D7 PreparedActionBinding/3 selected by link.preparedBindingToken. The interactive branch's declared preparedFormat selects exactly the actual original D7 PreparedActionBinding/3 or D8 PreparedEditBinding/2; D8 requires protocolOwner=D6, while D7 may carry D3 or D6 under its original contract. The complete embedded request, DecisionKey, audience, preview and pins must agree. Interactive work still requires its actual trusted user confirmation and original owner gates; it gains no standing-approval eligibility, new ActionSpec or submit. The record is atomically associated with the original Run/step before delivery/submission, never reconstructed by selecting a new request. Recovery pins include the original D3/D6 plan, primary/companion decision, installation B/N/provenance and audit where they actually exist under their original decoder. A source pin cannot masquerade as a prepared-record pin. P remains the sole owner of current decision and complete original receipt/error; no competing success flag is copied. Saved/planned/unknown follows the original owner first. D9-generated D7 preparations retain their actual D9 construction input inside that same original D7 record, not a second author request.

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

The inventory includes every active or still-referenced Run/configuration, original preparation (including full nested author A), count use, admitted Lease, subscription generation/continuity witness, armed/handled occurrence, reservation/attribution, external started/unknown attempt and stop result/capacity. Terminal evidence required to prevent repeat spending/claim/send remains included or retained by exact original typed pin. externalUnknowns also retains non-unknown attempts referenced by pending work or deduplication; its member name does not permit dropping a completed attempt that is still a dependency. Every reference resolves to the exact original record/version, and ranges establish completeness under the same protected store barrier. Config images include all historical budget inputs needed to reproduce attribution; current configuration is not a substitute. Shared deployment account totals include other execution domains' reservations under the actual account fence: transferring one Workspace never claims ownership of or resets the shared account. Such deployment liabilities stay at their true owner and are continuously referenced; they must remain reachable and eligible before new execution resumes.

The inventory is captured only after admission, planning, send and schedule writers for the old execution holder are stopped at one real store barrier. Author installation already in progress retains its original barriers and recovery responsibility; it cannot be discarded to make the inventory appear empty. New-holder execution requires D6's complete original-inventory verification, actual old-holder fencing, durable protected transfer and preserved storeIncarnation/ControlRefs. If a new physical store cannot preserve those identities and their protected continuity, takeover is unavailable, not a new empty execution domain. No source-file copy or rebuilt index supplies this proof.

The stop-capacity counters remain at their actual shared store owner just as shared account liability does. A Workspace-only transfer cannot copy that global counter into a second active store. Either the original authoritative safety store remains the reachable serialized owner, or a complete store handoff fences every affected writer and preserves all target/latch reservations. Inability to prove that boundary pauses takeover; it never resets issued/reserved or transfers only a visible subset.

### 16.2 Joint producer acceptance cases

Required cases include same-key r5 replay after r6; cyclic insertion of generated M rejected while nested A survives; missing typed historical image; hidden control source; concurrent last approval slot; raw-no-op consumption once; original planned supplemental approval; stop after file install but before seal; third-state recovery with no refund; missing external confirmation followed by a valid same-preparation human event; shared-account multi-Automation limits; interactive Run with no fake Automation; Run recovery at remaining=0; armed late wake-up; complete atomic missed-window disposition; unrelated source progression; delete/recreate ABA; changed-and-restored rule; observer gap; full and empty transfer ranges; and old-holder send/claim fencing. Document checks do not execute these concurrency, storage or UI cases. Fresh independent design acceptance, backend tests and D1 release qualification remain required.
