---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: `680b2061-ccec-4bef-b9a8-9d7080f837fb`.

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Legacy revision05/D7 coordinated terminology status in fixed S is historical provenance only. The stable document ID, every existing conceptId, ownedNames and firstFreeze are preserved. New concepts carry their own D6-FA-r01 candidate firstFreeze.

# D6 Terminology and Naming Lexicon

This is the readable afterimage corresponding to the machine registry source.json. Controlled names identify concepts only; they do not grant capability, identity, transaction ownership or implementation status. D3 keeps Typed Reference/OperationId/Authority/lifecycle; D4 keeps Field/Registry/Calendar; D7 keeps Query/Action/Effect; D8 keeps Editor; D9 keeps conversion; D10 keeps Agent/approval/provider concepts.

## 1. Complete public concept table

| conceptId | English | ownedNames | D6-FA-r01 meaning | firstFreeze |
|---|---|---|---|---|
| weftext.term.storage-domain | Storage Domain | \`StorageDomain\`<br>\`storageDomain\` | Managed commit/recovery scope for one CommitDomain. File author bytes, portable metadata and off-workspace durable control may span media without creating a second writable truth. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.authority-store | Authority Store | \`AuthorityStore\`<br>\`authorityStore\` | Logical Core authority view over current author and portable-control truth; it is no longer a SQLite database containing all current bodies. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.storage-fence | Storage Fence | \`StorageFence\`<br>\`storageFence\` | Durable monotonic eligibility fence for a CommitDomain holder; Server fencing covers both durable control and author-file writes. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.authorized-cut | Authorized Cut | \`AuthorizedCut\`<br>\`authorizedCut\`<br>\`openAuthorizedCut\`<br>\`subscribeCut\` | Authorized read cut binding CommitDomain, Frontier, observation epoch, source/control versions and principal context, with an explicit local or complete scope. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.mutation-footprint | Mutation Footprint | \`MutationFootprint\`<br>\`mutationFootprint\` | Trusted projection of every actual semantic modification between true before and proposed state; never a caller-declared patch scope. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.prepared-intent | Prepared Intent | \`PreparedIntent\`<br>\`preparedIntent\`<br>\`d6_prepare_request\`<br>\`d6_prepared_intent\` | Core-managed immutable intent containing owner input descriptor, plan, dependencies, scope, budget, installation plan and pins before commit. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.plan-token | Plan Token | \`PlanToken\`<br>\`planToken\` | Opaque transport token selecting one managed PreparedIntent; not identity, authorization or an editable plan. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-handle | Result Handle | \`ResultHandle\`<br>\`resultToken\`<br>\`publishResult\` | Managed complete Query/result record bound to cut, schema, audience and lifetime; not durable row/content identity. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-cursor | Result Cursor | \`ResultCursor\`<br>\`cursorToken\`<br>\`nextCursorToken\` | Opaque transport position within one result epoch; not a semantic limit or row identity. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.budget-binding | Budget Binding | \`BudgetBinding\`<br>\`budgetBinding\` | Fixed resource limits and trusted cumulative accounting for one plan; retries do not reset semantic scale or charged work. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.series-scope-configuration | Series Scope Configuration | \`SeriesScopeConfiguration\`<br>\`seriesScopeConfiguration\`<br>\`d6_series_configuration_intent\`<br>\`d6_series_configuration_remove_intent\` | Managed multiplicity configuration for one exact calendar series+scope across all period keys. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.import-job | Import Job | \`ImportJob\`<br>\`importJob\`<br>\`stageInput\`<br>\`planAtomicGroups\`<br>\`commitImportBatch\` | Durable managed import-job control over finite input, indivisible groups, batches and actual committed prefix; not another transaction ledger. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.source-checkout | Source Checkout | \`SourceCheckout\`<br>\`sourceCheckout\` | Legacy/explicit exported source-proposal presentation. In a file-backed workspace the ordinary current .adoc file is itself the Document author bytes. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.source-version | Source Version | \`SourceVersion\`<br>\`sourceVersions\`<br>\`sourceRevision\`<br>\`readSource\`<br>\`DocumentRevision\`<br>\`ResourceRevision\`<br>\`AnnotationRevision\` | SourceVersion/2 binding full EntityRef to CommitDomain, observationEpoch and either managed revision+ChangeId or external observation sequence. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.principal-context | Principal Context | \`PrincipalContext\`<br>\`principalContext\` | Host/D10-authenticated principal, session, delegation and policy-generation context; never caller-declared identity. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.commit-protocol | Control Commit Protocol | \`D6ControlCommit\`<br>\`d6_commit_request\`<br>\`d6_commit_receipt\`<br>\`d6_error\`<br>\`commitBoundPlan\`<br>\`replayOperation\` | Closed D6 v2 plan/file-install/durable-seal/receipt/replay protocol for the D6-owned ledger. D3 identity/lifecycle stays with D3. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.effects-token | Effects Token | \`EffectsToken\`<br>\`effectsToken\` | Transport token referring to the complete effect set of the same decision; never a second author source or write capability. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.result-page-protocol | Result Page Protocol | \`D6ResultPage\`<br>\`d6_result_page_request\`<br>\`d6_result_page\`<br>\`d6_result_error\`<br>\`readResultPage\` | Closed authorized pagination of a complete result with explicit terminal state and closed errors. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.attempt-allowance | Attempt Allowance | \`AttemptAllowance\`<br>\`attemptAllowance\`<br>\`d6_attempt_allowance_intent\` | Finite durable execution-attempt eligibility bound to a planned decision; it does not refund charged work or enlarge the original semantic budget. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.execution-resource-summary | Execution Resource Summary | \`ExecutionResourceSummary\`<br>\`executionResourceSummary\` | Managed policy-admin view of planned resource occupancy, charged work, pins and pause category without author-value disclosure. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.dependency-proof | Dependency Completeness Proof | \`DependencyProof\`<br>\`dependencyProof\`<br>\`readCompleteScope\`<br>\`validateDependencies\` | Trusted complete positive/negative dependency proof bound to CommitDomain, Frontier and observation epoch; a partial index is only an optimization when coverage is proved. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.foreign-version-comparison | Foreign Version Comparison | \`ForeignVersionComparison\`<br>\`compareForeignVersion\` | Pure D9/D10-owned comparison of an external version relation; never a mutation or redefinition of D3 bindings. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.binding-retirement-intent | Binding Retirement Intent | \`BindingRetirementIntent\`<br>\`retireBinding\` | Explicit authorized retirement of an existing SourceBinding/OriginBinding through its owning adapter and transaction. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.control-inspection-protocol | Control Inspection Protocol | \`ControlInspection\`<br>\`d6_control_read_request\`<br>\`d6_control_state\`<br>\`d6_control_error\` | Closed current-authorized read protocol for exact managed control targets; not a generic author-source API or OperationId enumerator. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.byte-handle | Resource Byte Handle | \`ByteHandle\`<br>\`ByteHandleRecord\`<br>\`d6_byte_handle\`<br>\`handleToken\` | Read-only handle bound to one committed Resource version/cut/descriptor/audience/pin/lifetime/budget; not latest, upload staging or identity. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.byte-read-protocol | Resource Byte Read Protocol | \`ByteReadProtocol\`<br>\`readResourceBytes\`<br>\`d6_byte_read_request\`<br>\`d6_byte_chunk\`<br>\`d6_byte_error\` | Closed offset/maxBytes byte read over one fixed Resource ByteHandle with exact EOF, current authorization and shared bounded accounting. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.observation-scope | Constraint Observation Scope | \`ObservationScope\`<br>\`observationScope\`<br>\`d6_observation_scope\` | Potential result/constraint observation upper bound selected and authorized before author-value evaluation; not the actual write set. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.issuer-control-policy | Issuer Control Policy | \`IssuerControlPolicy\`<br>\`issuerControlPolicy\`<br>\`d6_issuer_control_policy\` | Issuer-local allocate/administer authorization and its current revision; not Workspace content authorization. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.workspace-bootstrap-plan | Workspace Bootstrap Plan | \`WorkspaceBootstrapPlan\`<br>\`workspaceBootstrapPlan\`<br>\`d6_workspace_bootstrap_plan\` | Protected D6 control portion of a D3 create/fork plan, publishing initial policy/Registry/Calendar state atomically with activation. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.bootstrap-profile | Workspace Bootstrap Profile | \`WorkspaceBootstrapProfile\`<br>\`bootstrapProfile\`<br>\`d6_bootstrap_profile\` | Family-immutable Registry seed and explicit initial scope/multiplicity choices; not a D2 Profile or retry-time ambient configuration. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.calendar-period-scope-binding | Calendar Period Scope Binding | \`CalendarPeriodScopeBinding\`<br>\`periodScopeBindings\`<br>\`d6_period_scope_intent\` | Managed choice binding a period Node to an exact D4 scope with an independent ABA-resistant control version. | D6 revision05-observation-bootstrap candidate; not activated |
| weftext.term.source-envelope-state-capability | Source Envelope State Capability | \`SourceEnvelopeStateCapability\`<br>\`source_envelope_state\` | Explicit Policy/3 (and legacy Policy/2 path) right to observe source revision/capacity/aggregate validity and source-edit outcome metadata. | D7 coordinated revision02 candidate; not activated |
| weftext.term.commit-sequence-state-capability | Commit Sequence State Capability | \`CommitSequenceStateCapability\`<br>\`commit_sequence_state\` | Policy/3 right to observe one CommitDomain activity sequence; legacy Policy/2 retains its historical workspace-wide meaning only on legacy paths. | D7 coordinated revision02 candidate; not activated |
| weftext.term.commit-domain | Commit Domain | \`CommitDomain,commitDomain\` | Namespace for one D3/D6 idempotency, version and recovery ledger. Replica uses WorkspaceRef+ReplicaEpoch; Server uses WorkspaceRef+AuthorityInstanceId. | D6-FA-r01 candidate; not activated |
| weftext.term.replica-epoch | Replica Epoch | \`ReplicaEpoch,replicaEpoch\` | Core-minted non-reused UUIDv4 identifying one ordinary file-replica writer generation; not authority or execution lease. | D6-FA-r01 candidate; not activated |
| weftext.term.change-id | Change ID | \`ChangeId,changeId\` | CommitDomain plus continuous content-change sequence identifying one portable causal content point. | D6-FA-r01 candidate; not activated |
| weftext.term.frontier | Frontier | \`Frontier,frontier\` | Canonical vector with at most one maximum accepted ChangeId per CommitDomain, representing known continuous content prefixes. | D6-FA-r01 candidate; not activated |
| weftext.term.portable-workspace-metadata | Portable Workspace Metadata | \`PortableWorkspaceMetadata,portableMetadata\` | Portable files uniquely owning identity, structure, lifecycle, shared policy/trust and portable change/conflict records without duplicating all current bodies. | D6-FA-r01 candidate; not activated |
| weftext.term.durable-control-store | Durable Control Store | \`DurableControlStore,durableControlStore\` | Off-workspace durable SQLite and private pins for decisions, recovery, unknown effects, approval/claim/Money responsibility; not current body/structure truth. | D6-FA-r01 candidate; not activated |
| weftext.term.derived-index-store | Derived Index Store | \`DerivedIndexStore,derivedIndexStore\` | Device-local discardable SQLite for rebuildable metadata/parser/search/OCR facts; never author authority or completeness by itself. | D6-FA-r01 candidate; not activated |
| weftext.term.content-guarantee | Content Guarantee | \`ContentGuarantee,contentGuarantee\` | Closed replica_local or managed_atomic guarantee describing what the current CommitDomain proved about installation/publication. | D6-FA-r01 candidate; not activated |
| weftext.term.semantic-state | Semantic State | \`SemanticState,semanticState\` | complete_semantics or semantic_pending for a D2-valid managed source, with closed unproved obligations for pending. | D6-FA-r01 candidate; not activated |
| weftext.term.installation-notice | Installation Notice | \`InstallationNotice,installationNotice\` | Portable immutable pre-install record binding ChangeId, write set and before/after summaries; not commit proof or execution authority. | D6-FA-r01 candidate; not activated |
| weftext.term.content-completion-proof | Content Completion Proof | \`ContentCompletionProof,contentCompletionProof\` | Portable immutable post-seal proof binding ChangeId, semantic state, Frontier advance and actual after components; grants no execution authority. | D6-FA-r01 candidate; not activated |
| weftext.term.conflict-record | Conflict Record | \`ConflictRecord,conflictRecord,ConflictKey,conflictId\` | Portable concurrent-version record addressed by canonical ConflictKey/ConflictId and resolved only through owner-specific preparation. | D6-FA-r01 candidate; not activated |
| weftext.term.reliable-save-state | Reliable Save State | \`ReliableSaveState,reliableSaveState\` | State distinguishing not_saved from a file installation followed by durable control seal; portable publication is separate. | D6-FA-r01 candidate; not activated |
| weftext.term.execution-responsibility | Execution Responsibility | \`ExecutionResponsibility,ExecutionResponsibilityRecord,executionDomainId\` | Protected continuity domain for Automation/ApprovalUse/claim/Money/external-unknown/stop responsibility, separate from ordinary CommitDomain. | D6-FA-r01 candidate; not activated |

## 2. D6-FA-r01 disambiguation

### 2.1 Authority Store no longer means “body SQLite”

The controlled AuthorityStore/authorityStore names remain because historical D6/D3/D7 text and saved evidence use them. D6-FA-r01 narrows the meaning to Core’s logical view over current author/portable-control truth:

- current Document/Resource bytes: ordinary files;
- identity/parent/order/lifecycle/shared policy/trust: Portable Workspace Metadata;
- operation decision/unknown/approval/Money: Durable Control Store;
- index/cache: Derived Index Store.

AuthorityStore must therefore never be read as “a SQLite table containing the current body”, and DurableControlStore never owns current portable parent/order.

### 2.2 Storage Domain and Commit Domain

StorageDomain remains the historical physical/recovery coordination concept. CommitDomain is the new FA-r01 idempotency/version namespace. A file-backed Workspace may coordinate ordinary files, portable metadata and off-workspace durable control as one managed operation while the operation key still explicitly contains CommitDomain. Neither concept is Workspace identity.

ReplicaEpoch denotes an ordinary replica writer generation. ExecutionResponsibility denotes continuous Automation/approval/Money/external-effect responsibility. Neither implies the other, and loss of execution responsibility does not freeze unrelated ordinary content.

### 2.3 Source Version

The SourceVersion/1 controlled names remain for legacy decoders. New managed producers use SourceVersion/2 and bind complete Ref, CommitDomain, observationEpoch, revision and ChangeId; external observation uses its closed external variant. A consumer that compares only revision numbers is not FA-r01 conforming.

A→B→A, watcher gaps and placeholder materialization may change observation epoch/variant. Equal digest never revives old locator, prepared state, ActionEvidence or sourceOccurrenceKey continuity.

### 2.4 Reliable Save and Portable Publication

ReliableSaveState answers only whether this domain’s fixed write set was installed through a qualified file primitive and passed the durable control seal. ContentCompletionProof/portable publication separately answers whether the sealed change is a complete portable version another replica may admit.

Draft persistence, HTTP success, worker success, sync upload and InstallationNotice are never reliable save. reliable is not automatically portable published.

### 2.5 Semantic State

Both complete_semantics and semantic_pending require D2 validity. pending lists closed, explicitly unproved obligations; it is not a free “check later” string.

external_invalid is not SemanticState because it has no successful Core author decision. It is a branch of CurrentSourceState.

semantic_pending is never consumed as complete by complete D7 Query/Action, purge, relation/unique/Calendar mutation, or automatic writes. Any local consumer must be explicitly frozen by its D4/D5/D7/D8 owner.

## 3. Technical ownership

- CommitDomain, ReplicaEpoch, ChangeId, Frontier, SourceVersion/2, SemanticState, ContentGuarantee, InstallationNotice, ContentCompletionProof, ConflictRecord, ReliableSaveState, ExecutionResponsibilityRecord: D6.
- NodeRef/ResourceRef/AnnotationRef, OperationId, AuthorityInstanceId, D3 lifecycle receipt: D3. A generic D6 commit never redefines them.
- FieldId, RegistryBinding, relation/Calendar typed semantics: D4.
- QuerySpec, ActionSpec, PreparedActionBinding, EffectManifest/EffectBytes: D7. FA-r01 consumer versions require later coordinated afterimages.
- Draft/Edit Map/IME/Undo/Source-Live-Read: D8.
- Import conversion inputs and ExportPlan/Publication: D9; D6 owns only durability/budget boundaries.
- ApprovalUse, sourceOccurrenceKey, ToolValue, Money lineage, Agent/Automation/connector external effects: D10/respective Money owner; D6 only owns the durable continuity container and no-double-consume/no-replay boundary.

## 4. Policy/3 controlled capabilities

Policy/3 retains every Policy/2 capability and adds:

| capability | meaning | does not imply |
|---|---|---|
| replica_register | explicitly register a new ReplicaEpoch | source read/write or execution takeover |
| replica_retire | retire an ordinary replica writer epoch | deletion of history or Money refund |
| conflict_read | read an authorized ConflictRecord | conflict source bytes |
| conflict_resolve | enter owner-specific resolution prepare | source/policy/D3 write |
| execution_custody_admin | manage execution-responsibility continuity/takeover | new approvals, larger Money, author write |

In Policy/3, commit_sequence_state observes one named CommitDomain’s domainCommitSequence. The historical Policy/2 workspace-wide meaning remains only on its legacy path and is never projected onto the offline-replica model.

## 5. Collisions, aliases and migration

1. No local-workspace-id, device-workspace-id or sync-id becomes content identity.
2. ReplicaEpoch, AuthorityInstanceId and executionDomainId are distinct UUID domains.
3. ChangeId is never shortened to revision, commit id, operation id or sync token.
4. Frontier is never called global latest/version; it is a multi-domain causal frontier.
5. ConflictId is a stable address of a complete ConflictKey, not independent durable content identity.
6. DurableControlStore is not “the index”; DerivedIndexStore is not “database authority”.
7. Product text distinguishes Draft saved, reliable author save, and portable published.
8. No compatibility alias is introduced for the new protocol. Historical saved evidence retains historical controlled names without text migration/deletion.

## 6. Legacy and activation boundary

Legacy SourceVersion, Policy/1/2, D6 wire1, PreparedIntent, D7 binding/effects and historical receipts retain their original decoder and firstFreeze. firstFreeze in the machine registry is historical provenance and is never rewritten merely because FA-r01 reopens the owner.

New public names may enter product/API only after every required replacement owner and consumer is jointly accepted. Presence in this candidate directory does not make a capability available and does not authorize injecting candidate names into a D10 manifest/contribution or product log as a released schema.
