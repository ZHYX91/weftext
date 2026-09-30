---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: `592603c4-c7ef-4572-aee6-256aa3aa7955`.

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. D7/D9 activation labels and revision05 prose in fixed S are historical provenance only and do not govern this afterimage. The stable document ID is preserved. D3 wire11, D6 wire1, Policy/1/2, and saved D7/D8 legacy bindings continue under their original decoders/gates. Partial activation of managed v2 success is forbidden.

# D6 Control Interfaces

This file owns D6-FA-r01 closed control types, D6 wireVersion2, PreparedIntent/2, Policy/3, SourceVersion/2, ordinary source save, decision/install state, portable installation/completion records, ConflictRecord, and error/authorization ordering. D3 continues to own identity/lifecycle. No generic D6 interface may create, move, Trash, restore, or purge an Entity.

All JSON objects are closed. Unknown/missing/duplicate keys, illegal null, wrong union variants, and invalid Unicode scalars are rejected. A member may be absent only where this document explicitly marks it optional; null never stands for absent.

## 1. Common scalars and canonical encoding

### 1.1 Counter, Uuid, Token

Counter remains a non-Boolean JSON integer in 0..9223372036854775807. Floating point, exponent notation, -0, strings, and validation only after a double roundtrip are forbidden. Checked increment beyond MAX fails and never wraps.

Uuid reuses D3 canonical lowercase RFC4122 UUIDv4. WorkspaceRef/NodeRef/ResourceRef/AnnotationRef/FieldId use their owner decoders; D6 does not define lookalike structures.

Token retains the original D6 lexical profile: canonical unpadded base64url encoding of 32 random bytes, exactly 43 ASCII characters. Tag, audience, and record ownership are separately protected. Token is not identity, permission, or source version. Unknown/wrong-tag/wrong-audience follows each interface’s non-disclosure order.

Canonical bytes for successfully decoded D6 v2 objects use D3-CJ/3. This does not modify the D3 request fingerprint and does not turn a D6 object into D3 wire.

### 1.2 CommitDomain/2

CommitDomain is a closed union.

Replica:

~~~json
{"kind":"replica","workspaceRef":<WorkspaceRef>,"replicaEpoch":"uuid-v4"}
~~~

Server:

~~~json
{"kind":"server","workspaceRef":<WorkspaceRef>,"authorityInstanceId":"uuid-v4"}
~~~

workspaceRef must equal the outer Workspace. replicaEpoch is not AuthorityInstanceId. server authorityInstanceId uses the D3 authority domain. Equality is complete canonical-byte equality. Ordering is kind rank replica=0/server=1, then canonical WorkspaceRef, then UUID ASCII bytes.

### 1.3 ReplicaEpoch

ReplicaEpoch is a canonical UUIDv4 minted by Core during authorized replica registration and never reused within the Workspace. It is not portable content identity, D3 continue token, global execution lease, or caller-selected device ID.

Portable ReplicaRecord is exactly:

~~~json
{"format":"weftext.replica","version":1,"workspaceRef":<WorkspaceRef>,"replicaEpoch":"uuid-v4","state":"active|retired","registrationSequence":<Counter>}
~~~

registrationSequence starts at 1 and increments continuously per Workspace. It is distinct from content change sequence and commit sequence. retired never becomes active again; a returning/new device gets a new epoch.

### 1.4 DecisionKey/2, ChangeId/1, and Frontier/2

DecisionKey/2 is:
~~~json
{"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"operationId":"uuid-v4"}
~~~
P still indexes workspaceId, D3-CJ/3(commitDomain), and operationId. protocolOwner is not part of the key and one key has exactly one D3|D6 owner; changing owner or canonical request at the same key is operation_id_conflict.

ChangeId/1 is:
~~~json
{"commitDomain":<CommitDomain/2>,"sequence":<Counter>}
~~~
sequence is 1..MAX. A portable effect receives ChangeId only by checked allocation inside P seal. prepare/planning/staging/InstallationNotice reserve no successful sequence. Failure, paused, recovery_unknown, control_only, and true no_op create no content ChangeId. ChangeId is not OperationId, EntityRef, sourceOccurrenceKey, or global time.

Frontier/2 is:
~~~json
{"kind":"d6_frontier","version":2,"heads":[<ChangeId/1>...]}
~~~
heads may be empty; otherwise CommitDomain-key sorted and unique by domain, meaning verified continuous sealed causal prefixes. It proves neither payload materialization, placeholder download, complete Query scope, nor execution responsibility. frontierPolicy is exact|scope_dependencies: exact requires full Frontier equality; scope_dependencies admits only a proved non-regressing unrelated extension and revalidates original source/control/authorization/positive-negative dependency scope. It never changes target, Query, source, or canonical request. Frontier/1 is legacy decoder/replay only.

### 1.5 SourceVersion/2, SourceObservation/1, and SourceVersionRef/1

SourceVersion/2 retains its complete closed union.

Managed:
~~~json
{"kind":"managed_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"revision":<Counter>,"changeId":<ChangeId/1>}
~~~

External:
~~~json
{"kind":"external_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"externalSequence":<Counter>}
~~~

observationEpoch/revision/externalSequence are 1..MAX. Managed changeId.commitDomain equals production commitDomain. Document entityRef is owner NodeRef; Resource/Annotation use complete Refs. Managed revision increments once only for a real managed source change in production domain+entity, fresh=1, and raw no-op does not increment. observationEpoch advances whenever external observation continuity is unproved even if bytes later match; externalSequence advances for each new external state in an epoch not attributable to a known ChangeRecord. Managed/external never compare equal and bare revisions from different production CommitDomains are incomparable.

Current replica/Server observation is separately bound by SourceObservation/1:
~~~json
{"kind":"d6_source_observation","version":1,"observerDomain":<CommitDomain/2>,"entityRef":<EntityRef>,"sourceVersion":<SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<FileObjectBinding/1>,"evidencePins":[<PinRef/2>...]}
~~~
observerDomain equals current operation CommitDomain, entityRef equals sourceVersion.entityRef, and evidencePins are token-sorted/unique. Placeholder, missing metadata, conflict branch, or unproved observation continuity yields no successful SourceObservation. sourceVersion.commitDomain may differ from observerDomain.

Narrow public version projection is SourceVersionRef/1:
~~~json
{"entityRef":<EntityRef>,"sourceToken":<Token>}
~~~
sourceToken tag=d6_source_observation/1 selects complete protected SourceObservation rather than bare revision/digest/production SourceVersion. Watcher gap, external replace, or discontinuous rematerialization invalidates the old token. Legacy D2/D3 revision-token lexical ownership remains unchanged.

### 1.6 SemanticState/1 and ContentGuarantee

SemanticState is either:

~~~json
{"kind":"complete_semantics"}
~~~

or:

~~~json
{"kind":"semantic_pending","obligations":["relation"|"unique"|"calendar"|"collection"|"inbound"|"cross_object_type"...]}
~~~

obligations is non-empty, unique, and ordered by fixed rank relation=0, unique=1, calendar=2, collection=3, inbound=4, cross_object_type=5. Both variants guarantee strict UTF-8 and D2 validity. semantic_pending also guarantees that actually modified local typed facts passed their local gates, but it claims nothing about the listed obligations.

Externally invalid strict-UTF8/D2 bytes are not encoded as SemanticState. CurrentSourceState.external_invalid represents them so raw bytes with no successful Core author decision are never mislabeled as saved semantics.

ContentGuarantee is the exact text enum replica_local|managed_atomic.

## 2. FileObjectBinding, WriteProtection, and installation qualification (trusted internal types)

FileObjectBinding/1 is never client-constructed.

Absent:
~~~json
{"kind":"absent","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"parentGenerationToken":<Token>}
~~~
Present:
~~~json
{"kind":"present","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"objectGenerationToken":<Token>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~
PortableRelativePath remains non-empty UTF-8 with "/" separators and forbids absolute/empty/"."/".."/NUL/platform alias/reparse escape; host still proves canonical containment and path is not identity.

WriteProtection is strict|observed_only, orthogonal to ContentGuarantee/SemanticState/ReliableSaveState.

FileInstallCapability/2:
~~~json
{"kind":"create_only","parentGenerationToken":<Token>}
~~~
or
~~~json
{"kind":"conditional_replace","expectedObjectGenerationToken":<Token>}
~~~
or
~~~json
{"kind":"exclusive_write_window","windowToken":<Token>,"expectedObjectGenerationToken":<Token>}
~~~
or
~~~json
{"kind":"observed_replace","observedObjectGenerationToken":<Token>}
~~~
The first three are strict: conditional_replace uses a real trusted generation and read-hash-then-rename is not CAS; exclusive excludes all writers in the threat model and advisory locking is insufficient; create_only is expected-absent only. observed_replace is not CAS, records only the final verified observation, and is usable only under §4.1 observed_only eligibility.

Every path still proves staged bytes, installed data, required directory-entry/rename durability, containment, object type, and installation provenance. Observed competition, revocation, narrow deny, competing Core writer, unknown install, or lost P continuity gets no observed_only exemption; strict never downgrades in place.

## 3. InputDescriptor/2, PinRef/2, and PreparedIntent/2

### 3.1 PinRef/2

PinRef is a managed internal closed object and is never accepted as caller-authored evidence:

~~~json
{"kind":"d6_pin_ref","version":2,"pinToken":<Token>,"payloadKind":"exact_source_document|resource_bytes|annotation_value|portable_metadata|effect_bytes|artifact","byteLength":<Counter>,"sha256":"64-lowercase-hex","retentionClass":"recovery|conflict|external_unknown|approval_money|query_preview|import_export|user_history"}
~~~

A source pin also stores the complete SourceVersion/2 and EntityRef in its protected record. pinToken tag is d6_pin/2.

### 3.2 OwnerInputBinding/2

~~~json
{"kind":"d6_owner_input_binding","version":2,"protocolOwner":"D3|D6|D7|D8|D9","ownerKind":<controlled-text>,"canonicalDescriptorBytes":<immutable-bytes>,"pinRefs":[<PinRef/2>...]}
~~~
protocolOwner is input-descriptor owner, not final decision owner; DecisionKey still permits only D3 or D6 decision. ownerKind is owner-version frozen, never free callback/JSON; descriptor is complete and large bytes use typed PinRef slots only. D3 reserves d3_identity_operation/12 and remains owner_update_required until its consumer update; D7/D8/D9 likewise.

### 3.3 ObservationScope/2

Closed profiles: local_source{workspaceRef,commitDomain,ownerNodeRef}, owner_fields{workspaceRef,commitDomain,ownerNodeRef,fieldIds}, local_structure{workspaceRef,commitDomain,operation,subjects,parentCandidates}, workspace_constraints{workspaceRef,commitDomain}, control_only{workspaceRef,commitDomain}, prepared_workspace{workspaceRef,commitDomain}. local_structure.operation=create_node|move_node|reorder_node|trash. Scope is observation upper bound, not write permission/write set/completeness proof. Policy/3 explicit structure_state and portable_frontier_state disclose only portable structure and complete Frontier/2 respectively.

### 3.4 DependencyProof/2

~~~json
{"kind":"d6_dependency_proof","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"baseFrontier":<Frontier/2>,"entries":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>},"evidencePins":[<PinRef/2>...]}...]}
~~~
entries are canonical-sorted/unique. stamp.epoch is range continuity; deleting I, watcher gap, rebuild, or owner-version change never reuses it. DependencyKey/2 is closed to source, lifecycle, placement_range, ref_inbound, relation_incidence, calendar_scope, registry, temporal_rules, authorization, foreign_binding, query_scan, replica_registry, conflict_record, execution_resource, each with owner-defined closed key; there is no free JSON. Empty range still needs enumeration+stamp and index-ready/hash proves nothing alone.

### 3.5 InputDescriptor/2

~~~json
{"kind":"d6_input_descriptor","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"intentKind":<controlled-text>,"saveProfile":"ordinary|complete|control_only","guarantee":"replica_local|managed_atomic","expectedFrontier":<Frontier/2>,"frontierPolicy":"exact|scope_dependencies","observationScope":<ObservationScope/2>,"sourceInputs":[{"entityRef":<EntityRef>,"observation":<SourceObservation/1>,"role":"before|dependency"}...],"controlInputs":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>}}...],"ownerInput":<OwnerInputBinding/2>}
~~~
source/control inputs are canonical-sorted/unique; complete equality compares descriptor, owner descriptor, exact pins, and SourceObservation, never hash alone.

### 3.6 PreparedIntent/2

Immutable members: kind,version,planToken,operationId,workspaceRef,commitDomain,principalAudienceToken,inputDescriptor,beforeCut,proposedState,mutationFootprint,dependencyProof,observationProof,budgetBinding,pinDirectory,installationPlan,inputRetentionState,expiresAt,previewBinding.

kind=d6_prepared_intent/version=2; planToken tag=d6_plan/2. inputRetentionState=not_retained|retained|unavailable; retained requires durable proposal, actually-read before, and required bindings, and prepared/retained is not saved. installationPlan freezes write set, after bytes, owner versions, recovery, required WriteProtection/portable records, preallocates no ChangeId, and never resamples at commit. previewBinding references owner preview only; last-reference pins retain original protection/retention rules.

## 4. D6 v2 prepare and commit requests

### 4.1 Existing Document source-save prepare

~~~json
{"wireVersion":2,"kind":"d6_source_save_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"ownerNodeRef":<NodeRef>,"expectedSourceToken":<Token>,"saveProfile":"ordinary|complete","guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","source":<text>,"budget":<BudgetBinding>}
~~~
ownerNodeRef belongs to Workspace; expectedSourceToken tag=d6_source_observation/1 selects complete SourceObservation/1 under current authorization. source is immediately exact-pinned and commit carries no source. Physical-invalid external still uses repair.

ordinary requires D2 valid and actually modified local typed facts; legitimate unproved complete obligations -> semantic_pending. complete requires every applicable D4/D5/D7 duty and never downgrades.

observed_only closed eligibility: trusted interactive_source_save; one existing live Document; ordinary+replica_local; complete source read/replace; no applicable body/Field/node-control deny; author-source write set empty or that Document; no identity/parent/order/lifecycle/shared policy/Registry/Calendar-scope/other-entity mutation; Draft Base=selected SourceObservation. stale Base, observed external change, or continuity gap -> conflict/reprepare; noninteractive requires strict.

Owner descriptor:
~~~json
{"kind":"d6_source_save_input","version":2,"invocationClass":"interactive_source_save|noninteractive","expectedSourceObservation":<SourceObservation/1>,"proposedSource":<PinRef/2>,"writeProtection":"strict|observed_only"}
~~~
Success:
~~~json
{"wireVersion":2,"kind":"d6_prepared_intent","planToken":<Token>,"semanticState":<SemanticState/1>,"writeProtection":"strict|observed_only","inputRetentionState":"retained"}
~~~
This only means proposal/read-before/bindings are durably pinned, not saved. D3 lifecycle, D7 Action, Automation, approval, and Money never use observed_only.

### 4.2 d6_commit_request/2

Exactly:

~~~json
{"wireVersion":2,"kind":"d6_commit_request","operationId":"uuid-v4","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"expectedDomainFenceToken":<Token>,"planToken":<Token>}
~~~

No source, patch, budget, preview, or cost override is carried. expectedDomainFenceToken tag=d6_domain_fence/2 binds current domain qualification: active ReplicaEpoch/portable registry/policy/backend epoch for replica and additionally current D3 authority/custody/fence generation for server. It is not permission.

The v2 ledger key is exactly (workspaceId, D3-CJ/3(commitDomain), operationId), with protocolOwner=D6 in the record. Same key/different canonical request is operation_id_conflict. The D3 wire12 afterimage exists and shares DecisionKey owner collision, but its G0-A native-descriptor/companion consumer is not updated; until then new protocolOwner=D3 is owner_update_required with zero D3 decision.

## 5. Unique submit/install/seal/publish order

1. closed decode/static equality; invalid_request/preflight and zero business reads.
2. current-principal minimum disclosure, ObservationScope/2, CommitDomain qualification; not_visible on failure.
3. domain/fence/P continuity; unproved -> domain_unavailable, proven corruption -> integrity_conflict.
4. read DecisionKey; different owner/request -> operation_id_conflict; saved replays original bytes and planned only resumes original plan.
5. unseen validates fence, planToken, PreparedIntent/2, inputRetentionState, current auth/owner version.
6. under frontierPolicy revalidate Frontier/2, SourceObservation, DependencyProof/2, MutationFootprint auth, semantics, budget, unwritten deps; scope_dependencies admits only unrelated non-regressing extension.
7. planning CAS stores fixed plan/pins/after/recovery/WriteProtection, **allocating no ChangeId**.
8. before portable-current mutation durably write InstallationNotice/2 with DecisionKey, baseFrontier, WriteProtection, component before/after and no ChangeId.
9. install: strict uses strict capability only; observed_only is only §4.1 and does final trusted object/event check. Observed competition -> conflict/paused with B/N/current retained; unknown install -> recovery_unknown.
10. verify written=planned after; unwritten deps/policy/auth/Registry/rules/Frontier/2/control facts against original cut; unknown provenance/late competition/revocation remains paused/conflict/recovery_unknown; no ChangeId yet.
11. seal: one durable P transaction rechecks plan/auth. A portable effect now allocates ChangeId/SourceVersion and writes committed/receipt/ReliableSaveState/effects/outbox/applicable charge. strict->reliable; observed_only->durable_observed_only; control_only/no_op->not_applicable with no content ChangeId. This is the only decision commit point.
12. portable publication derives ContentCompletionProof/2 and advances Frontier/2; failure only pending, recovery never reinstalls N, changes OperationId, or recharges.
13. delivery rechecks current authorization; revocation may hide delivery but never alters decision.

For observed_only, B is only the actually read/pinned before and never enumerates unread C. Later current=C never rewrites old receipt and publication never reinstalls N. Crash recovery uses §8 and clients never guess. D6 v1/legacy retains original decoder/order.

True raw no-op effectClass=no_op has empty sourceVersions, InstallationState=not_required, ReliableSaveState=not_applicable, PortablePublicationState=not_applicable; domainCommitSequence may +1 but no ChangeId/source revision/Frontier advances. P-only control uses control_only; portable F/M uses portable.

## 6. PortableComponentKey, InstallationNotice, ContentCompletionProof

### 6.1 PortableComponentKey/1

Closed union:

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

Complex controls such as SeriesScopeConfiguration remain proved by their owner effects; no generic string key replaces them. A new portable component kind requires a new version.

ComponentImage/1 is either {"state":"absent"} or:

~~~json
{"state":"present","version":<Counter>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~

version belongs to the component owner. Digest verifies listed bytes; it is not identity.

### 6.2 SourceStamp/1 and InstallationNotice/2

SourceStamp/1:
~~~json
{"kind":"decision_source","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"revision":<Counter>,"observationEpoch":<Counter>}
~~~
It is pre-install determinate, not SourceVersion/2 or a second current truth; only committed ContentCompletionProof/2 for the same DecisionKey resolves to sealed SourceVersion.

InstallationNotice/2:
~~~json
{"format":"weftext.installation-notice","version":2,"decisionKey":<DecisionKey/2>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"before":<ComponentImage/1>,"after":<ComponentImage/1>}...]}
~~~
components is non-empty, fixed-rank/canonical-key sorted unique. observed_only is §4.1 only; every other portable author plan is strict. Notice is durable before first install and has no ChangeId/receipt/approval/Money/external payload/credential.

### 6.3 ContentCompletionProof/2

Committed:
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersionRef/1|"absent">,"after":<SourceVersionRef/1|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~
frontierAfter is frontierBefore plus sealed changeId with no other head regression; components equals notice key set using actual after; sourceChanges is EntityRef-sorted unique. receiptDigest grants no receipt/execution authority. observed_only proves this decision installation and read-before, not absence of an unobserved competitor.

Before seal after every component is safely restored to before:
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...]}
~~~
restored forbids ChangeId/receipt/semantic success; never generate without proved safe restoration.

## 7. Commit receipt, D3 companion, decision state, and current source

### 7.1 d6_commit_receipt/2

~~~json
{"wireVersion":2,"kind":"d6_commit_receipt","operationId":"uuid-v4","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"domainCommitSequence":<Counter>,"effectClass":"portable|control_only|no_op","guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","sourceVersions":[<SourceVersionRef/1>...],"effectsToken":<Token>}
~~~
writeProtection exists only for portable; control_only/no_op omit it and sourceVersions is empty. Portable sourceVersions is EntityRef-sorted unique. Receipt exposes no full Frontier, production SourceVersion, obligations, or decision internals; full Frontier needs portable_frontier_state and domainCommitSequence needs domain-scoped commit_sequence_state. Seal-time receipt bytes are immutable across publication/current changes.

### 7.2 D3DecisionCompanion/2

~~~json
{"kind":"d6_decision_companion","version":2,"decisionKey":<DecisionKey/2>,"domainCommitSequence":<Counter>,"effectsToken":<Token>}
~~~
Not a second receipt; future D3 primary receipt and companion share one P transaction. Until B, new protocolOwner=D3 -> owner_update_required with zero D3 decision.

### 7.3 d6_decision_state_read

Request:
~~~json
{"wireVersion":2,"kind":"d6_decision_state_read","protocolOwner":"D6|D3","request":<original-complete-request>}
~~~
Success:
~~~json
{"wireVersion":2,"kind":"d6_decision_state","protocolOwner":"D6|D3","decisionState":"planned|committed|rejected|terminal_failed","installationState":"not_required|planned|installing|installed|conflict|recovery_unknown|paused_authorization|paused_capacity","reliableSaveState":"not_saved|reliable|durable_observed_only|not_applicable","inputRetentionState":"not_retained|retained|unavailable","portablePublicationState":"not_published|pending|published|conflict|not_applicable"}
~~~
strict portable -> reliable; observed_only -> durable_observed_only; control/no_op -> not_applicable. inputRetention is independent. Complete original request prevents ledger probe; auth order remains closed decode -> disclosure/scope -> domain continuity -> DecisionKey/fingerprint -> state. Revocation may return not_visible without changing history.

### 7.4 d6_current_source_read

Request still has workspaceRef, commitDomain, entityRef and source-read permission precedes domain/backend/FileBinding.
Managed:
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"managed","sourceVersionRef":<SourceVersionRef/1>,"semanticState":<SemanticState/1>,"byteLength":<Counter>}
~~~
External:
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external","sourceVersionRef":<SourceVersionRef/1>,"validation":"d2_valid|unverified","byteLength":<Counter>}
~~~
External-invalid:
~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external_invalid","sourceVersionRef":<SourceVersionRef/1>,"validation":"physical_invalid|d2_invalid","byteLength":<Counter>}
~~~
Bytes still use authorized source/ByteHandle/repair. Owners needing full version use protected SourceObservation/DependencyProof.

Effects metadata stores effectClass, WriteProtection, owner preview binding; observed_only before is observed_before/read_before and never claims every displaced external byte. Until D7 effects consumer update, owner_update_required.

## 8. Installation state and crash recovery

Each planned v2 decision stores per-component internal state pending|staged_durable|installed_after|restored_before|third_state|unavailable.

Recovery reads P, then classifies actual components only as exact_before, exact_after with proved installation provenance, third_state, or unavailable. Equal bytes without provenance do not become exact_after.

A written exact_after component is validated against planned after in commit step 10; unwritten dependencies remain compared to before/cut expectations. This separation is mandatory and prevents the incorrect rule “after installation every expectedSourceVersion must still be before”.

planned recovery reuses the original InputDescriptor, pins, ChangeId, versions, OperationId, and budget counters. If plan pins/clock/domain continuity cannot be proved, remain planned+paused/recovery_unknown; never reprepare another decision in its place.

### 8.1 observed_only recovery boundary

observed_only read-before is only the actually read/pinned before and never enumerates C unseen after the final check. An overwritten unseen C may have no recoverable copy; that is the approved boundary and no record invents one. Any observed competition remains third_state/conflict. Unknown install is recovery_unknown and equal hash never guesses success. Committed publication pending only resumes the same ContentCompletionProof/2 and never reinstalls N.

## 9. ConflictKey/1, ConflictRecord/1, and resolution prepare

### 9.1 ConflictSubject and kind

ConflictSubject:

- {"kind":"workspace","workspaceRef":WorkspaceRef}
- {"kind":"entity","ref":EntityRef}

Conflict kind is one of source_concurrent | placement_concurrent | lifecycle_concurrent | identity_collision | policy_concurrent | incomplete_transport | placeholder.

### 9.2 ConflictKey/1 and ConflictId

Exactly:

~~~json
{"workspaceRef":<WorkspaceRef>,"kind":<conflict-kind>,"subjects":[<ConflictSubject>...],"heads":[<ChangeId>...]}
~~~

subjects and heads are non-empty and unique. Subjects sort workspace rank0 then entity rank1+canonical Ref; heads sort by ChangeId. Every member belongs to the same Workspace. source/placement/lifecycle/identity includes at least one entity subject; policy includes a workspace subject.

ConflictId is ASCII d6c: plus 64 lowercase hex where hex = SHA-256(ASCII "D6-ConflictKey/1" + NUL + D3-CJ/3(ConflictKey)). It is only an address; the complete ConflictKey is stored and byte-compared.

Portable ConflictRecord:

~~~json
{"format":"weftext.conflict","version":1,"conflictId":<ConflictId>,"key":<ConflictKey/1>,"state":"open|resolution_prepared|resolved|superseded","createdAtFrontier":<Frontier/1>,"supersedes":[<ConflictId>...]}
~~~

supersedes may be empty and is sorted unique ASCII. New heads supersede an open/prepared record and create a linked successor. Resolved history is immutable.

### 9.3 conflict read

~~~json
{"wireVersion":2,"kind":"d6_conflict_read","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>}
~~~

conflict_read and minimum state disclosure for every subject precede record access. Unauthorized/unknown is not_visible. Reading a conflict never grants source bytes.

### 9.4 conflict resolution prepare

~~~json
{"wireVersion":2,"kind":"d6_conflict_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/1>,"budget":<BudgetBinding>}
~~~

ConflictResolution/1 is:

- source_merge: {"kind":"source_merge","ownerNodeRef":NodeRef,"source":text}
- choose_source_head: {"kind":"choose_source_head","ownerNodeRef":NodeRef,"head":ChangeId}
- policy_choice: {"kind":"policy_choice","policy":<Policy/3>}
- owner_resolution: {"kind":"owner_resolution","owner":"D3","action":"placement|lifecycle|identity_fresh_copy"}

owner_resolution has no free payload. Before D3 wire12 exists it returns owner_update_required and creates no planToken. A future D3 adapter must freeze the concrete typed request/preview; D6 never smuggles arbitrary JSON through this member.

source_merge/choose_source_head reload every head/base/current permission and rerun D2/local/complete gates. policy_choice requires current policy_admin and never performs allow-union fallback. Success creates an ordinary PreparedIntent/2 and final commit uses d6_commit_request/2. Any new head makes expectedKey stale -> conflict_changed; an old click is never reused.

## 10. Policy/3

Policy/3 retains top-level version,revision,grants with version=3. Policy/1/2 keep their original decoders and never auto-upgrade. grants remain subject,effect,scope,capabilities with deny-before-allow and default deny. Every Policy/2 capability remains unchanged.

New no-argument capabilities are replica_register | replica_retire | conflict_read | conflict_resolve | execution_custody_admin | structure_state | portable_frontier_state. structure_state discloses portable parent/order/structural scope only; portable_frontier_state discloses complete Frontier/2 only; neither grants source/Field/decision/write.

They imply no source/Field/body/lifecycle permission:

- replica_register/retire manage portable replica registry only under workspace-scope administrative semantics plus current trust/bootstrap;
- conflict_read reveals an authorized ConflictRecord, never conflict source bytes;
- conflict_resolve only enters resolution prepare; actual source/policy/D3 write still needs the original write permission;
- execution_custody_admin manages execution continuity/takeover only and does not expand Money, approvals or author source write.

source_write deny continues to block full-source changes; Field/body deny still blocks corresponding footprints. write never implies read. ordinary source save still requires the original source/body/Field/node-control matrix plus potential observation scope. conflict/replica capability is never a bypass.

In Policy/3, commit_sequence_state is CommitDomain-scoped. Its read request names the complete CommitDomain and exposes only domainCommitSequence. There is no invented global order across offline replicas. Policy/2 historical consumers retain their original workspace-wide definition on legacy paths only.

## 11. BudgetBinding/2 and pin capacity

BudgetBinding/1 members/numeric domain remain. PreparedIntent/2 also binds PinBudget/1:

~~~json
{"version":1,"maxRecoveryBytes":<Counter>,"maxConflictBytes":<Counter>,"maxPreviewBytes":<Counter>,"maxImportExportBytes":<Counter>,"maxHistoryBytes":<Counter>}
~~~

0 means no new allocation of that class. Effective limits are request/policy/host minima. Pin allocation reserves with checked add before allocation. All attempts share plan counters. Work units remain durably charged before execution and crash does not refund. Temporary staging and protected pins are separately accounted.

A protected last-reference pin is never deleted by TTL/preview expiry. Capacity shortage produces budget_exceeded or planned paused_capacity; it never frees planned/unknown/conflict last-reference evidence.

## 12. Result/ByteHandle and index consumption

Existing D6 ResultHandle/ResultCursor and Resource ByteHandle/ByteRead wire1 remain historical. A new file-backed implementation binds immutable pins to SourceVersion/2/CommitDomain/Frontier and a new consumer version must specify reset conditions; this batch does not rewrite saved wire1 handles.

No new Query/Result producer activates before the D7 afterimage. The fixed rules are current authorization before index/result content; complete result only from complete scope proof; partial index never means empty; candidate indexes for exact/NFC/regex prove recall or supplement with source scan; evidence never rebinds an old domain/revision number to another domain; auth/observation/frontier dependency loss resets under the owning version.

## 13. Replica registration

Prepare:

~~~json
{"wireVersion":2,"kind":"d6_replica_register_prepare","workspaceRef":<WorkspaceRef>,"expectedReplicaRegistryRevision":<Counter>,"displayLabel":<text>,"budget":<BudgetBinding>}
~~~

displayLabel is authorized display text only, non-empty and at most 256 UTF-8 bytes. Requires workspace-scope replica_register and a verifiable current portable trust/registry. It never takes over authority. PreparedIntent/2 mints a never-used ReplicaEpoch and plans an active ReplicaRecord plus registry revision+1. A host-private bootstrap domain may perform registration but is not exposed as a reusable CommitDomain before successful seal.

Retire prepare:

~~~json
{"wireVersion":2,"kind":"d6_replica_retire_prepare","workspaceRef":<WorkspaceRef>,"replicaEpoch":"uuid-v4","expectedReplicaRegistryRevision":<Counter>,"expectedState":"active","budget":<BudgetBinding>}
~~~

Requires replica_retire. Retire does not delete published ChangeRecords/bytes; it only prevents future writing by that epoch. If the replica currently holds global execution responsibility, the execution-custody protocol must first hand off or pause it. Retire never refunds or transfers Money by itself.

## 14. Error/Disposition v2

d6_error/2 is exactly:

~~~json
{"wireVersion":2,"kind":"d6_error","code":<code>,"disposition":"preflight|recorded|paused|terminal"}
~~~

code is closed to invalid_request | unsupported_version | not_visible | domain_unavailable | integrity_conflict | operation_id_conflict | plan_expired | source_unavailable | proof_unavailable | dependency_conflict | semantic_rejected | budget_exceeded | install_unavailable | conflict | conflict_changed | state_unavailable | owner_update_required | effects_unavailable | transaction_aborted.

Rules:

- invalid_request/unsupported_version/not_visible/domain_unavailable/integrity_conflict/operation_id_conflict/plan_expired and install_unavailable known before planning are preflight only;
- dependency_conflict/semantic_rejected/budget_exceeded at semantic step 6 may be recorded only when they are deterministic business outcomes of the canonical intent;
- install-time competition, revocation, capacity or recovery uncertainty is paused and never writes rejected/terminal;
- conflict_changed and owner_update_required are preflight and create no decision;
- after planned, transaction_aborted+terminal is allowed only when it is proved that the original plan can never commit and every installation remnant has been safely resolved;
- portable publication pending after P seal is not an error.

No free details/hidden counts/original request are added. Authorized audit may expose a separate internal cause.

## 15. Current authorization and non-disclosure order

All v2 interfaces use:

1. closed decode/static cross-field relation;
2. current authenticated principal’s minimum capability/scope and state-disclosure gate;
3. CommitDomain/replica/server qualification, fence, portable trust and backend availability;
4. applicable ledger/token/record audience;
5. current target/source/control existence/version;
6. business dependency/semantic/budget;
7. mutation/install/decision;
8. current authorization again before output.

No hidden author fact is read first to decide that “this run happened to be safe”. Caller knowledge of ref/path/digest grants no existence right. conflictId, ChangeId, SourceVersion and planToken are not capabilities.

Policy/ACL changes do not grandfather old sessions/prepares. Revocation before seal prevents seal. Revocation after committed may hide receipt/effects delivery but never rewrites history.

## 16. Global execution responsibility and control inspection

D6 v2 ExecutionResponsibilityRecord is protected P control, never portable metadata. Exact semantic members are:

kind, version, workspaceRef, executionDomainId, holder, revision, status, approvalUses, claims, moneyLineage, externalUnknowns, stopState, lastContinuityProof.

kind=d6_execution_responsibility, version=2. executionDomainId is a Core-minted UUIDv4 independent of CommitDomain/ReplicaEpoch. holder is local_replica{replicaEpoch} or server{authorityInstanceId,deploymentId}; deploymentId is a protected host UUID, not author identity. revision increments checked.

approvalUses/claims/moneyLineage/externalUnknowns are frozen later by the D10/Money owner afterimage. D6 requires that they move continuously as one execution domain and are never omitted by takeover. Before their schemas exist, the corresponding capability remains unavailable; no free JSON is stored.

Execution takeover requires execution_custody_admin, the full old responsibility record, protected backup/remote handoff proof, and proof that the old holder is fenced. Failure is domain_unavailable/semantic_rejected and never creates a blank responsibility record. Takeover never copies or resets quota. sourceOccurrenceKey continuity requires complete D10 occurrence evidence; same path/hash/Field key is insufficient.

Control inspection v2 adds targets decision_state (complete original request required), current_source (complete EntityRef), conflict (ConflictId), replica (ReplicaEpoch), existing execution_resource, and execution_responsibility (executionDomainId, restricted to custody admin/audit). There is no generic “list every OperationId/unknown in Workspace” API.

## 17. Legacy replay and coordinated activation

D6 wire1 commit/receipt/error, Policy/1/2, SourceVersion/1, Frontier/1, InstallationNotice/1, ContentCompletionProof/1, legacy Token tags, PreparedIntent, D7 PreparedActionBinding/1,/2, D8 PreparedEditBinding/1, and their planned/saved decisions continue with original bytes, fingerprint, authorization/continuity and pin-retention rules.

Forbidden: re-encoding an old request as wire2; adding CommitDomain/ChangeId to an old receipt; interpreting an old D4 gate as semantic_pending; using new retention to delete evidence promised by the old contract; treating equal source hash as proof that SourceVersion/1 equals a new-domain source.

New v2 consumers are incomplete. Required D3/D4/D5/D7/D8/D9/D10 owner afterimages must be authored and accepted together. This candidate may not produce “v2 managed commit success” in product or conformance fixtures as activated semantics. Author documentation checks are not a substitute for the coordinated gate.
