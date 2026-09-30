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

### 1.4 ChangeId/1 and Frontier/1

ChangeId is exactly:

~~~json
{"commitDomain":<CommitDomain/2>,"sequence":<Counter>}
~~~

sequence is 1..MAX. Content change sequence begins at 1 per CommitDomain and increments continuously. The domain Workspace equals the current Workspace. Ordering is CommitDomain key followed by numeric sequence.

Frontier is exactly:

~~~json
{"kind":"d6_frontier","version":1,"heads":[<ChangeId>...]}
~~~

heads may be empty only for genesis/no content changes. Otherwise they are sorted by CommitDomain key, with at most one entry per domain. An entry means the continuous accepted prefix 1..sequence. Duplicate domains, sequence 0, and foreign Workspace entries are rejected. Frontier equality is complete canonical-byte equality; a set digest is never a substitute.

### 1.5 SourceVersion/2

SourceVersion/2 is a closed union.

Managed source:

~~~json
{"kind":"managed_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"revision":<Counter>,"changeId":<ChangeId>}
~~~

External observation:

~~~json
{"kind":"external_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"externalSequence":<Counter>}
~~~

observationEpoch, revision, and externalSequence are 1..MAX. Managed changeId.commitDomain equals commitDomain. Document entityRef is the owning NodeRef; Resource/Annotation use their complete Refs. Managed revision increments once for a real managed source change in the same domain+entity; fresh=1 and raw no-op does not increment. observationEpoch increments whenever continuity of external observation cannot be proved even if final bytes are identical. externalSequence increments for each newly observed external state within an epoch that cannot be attributed to a known ChangeRecord.

Managed and external variants are never equal. Numeric revisions from different CommitDomains are incomparable. Legacy D2/D3 revision-token lexical ownership is unchanged; a new consumer also binds SourceVersion/2 and never uses a bare Counter across domains.

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

## 2. FileObjectBinding and installation qualification (trusted internal types)

FileObjectBinding/1 is never client-constructed.

Absent:

~~~json
{"kind":"absent","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"parentGenerationToken":<Token>}
~~~

Present:

~~~json
{"kind":"present","backendToken":<Token>,"relativePath":<PortableRelativePath>,"observationEpoch":<Counter>,"objectGenerationToken":<Token>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~

PortableRelativePath is a non-empty UTF-8 scalar path using “/”, with no absolute form, empty component, ".", "..", NUL, platform alias, or reserved reparse escape. The host still proves canonical containment. Path is not identity.

FileInstallCapability/1 is one of:

~~~json
{"kind":"create_only","parentGenerationToken":<Token>}
~~~

~~~json
{"kind":"conditional_replace","expectedObjectGenerationToken":<Token>}
~~~

~~~json
{"kind":"exclusive_write_window","windowToken":<Token>,"expectedObjectGenerationToken":<Token>}
~~~

Only a trusted backend issues these. conditional_replace must compare a real trusted object generation; “read hash then rename” is not CAS. exclusive_write_window must exclude every writer in the supported threat model from modifying/replacing/deleting the target; advisory-only locks do not qualify.

If an existing target lacks conditional_replace/exclusive_write_window, prepare may retain Draft/after pins but commit returns install_unavailable before touching current state or remains planned+paused. It never reports reliable success. create_only is valid only for expected absent.

## 3. InputDescriptor/2, PinRef/2, and PreparedIntent/2

### 3.1 PinRef/2

PinRef is a managed internal closed object and is never accepted as caller-authored evidence:

~~~json
{"kind":"d6_pin_ref","version":2,"pinToken":<Token>,"payloadKind":"exact_source_document|resource_bytes|annotation_value|portable_metadata|effect_bytes|artifact","byteLength":<Counter>,"sha256":"64-lowercase-hex","retentionClass":"recovery|conflict|external_unknown|approval_money|query_preview|import_export|user_history"}
~~~

A source pin also stores the complete SourceVersion/2 and EntityRef in its protected record. pinToken tag is d6_pin/2.

### 3.2 OwnerInputBinding/2

OwnerInputBinding/2 has exact semantic members:

~~~json
{"kind":"d6_owner_input_binding","version":2,"protocolOwner":"D6|D7|D8|D9","ownerKind":<controlled-text>,"canonicalDescriptorBytes":<immutable-bytes>,"pinRefs":[<PinRef/2>...]}
~~~

ownerKind is frozen by the actual owner version; there is no free callback. canonicalDescriptorBytes are the complete owner-defined canonical descriptor with large source/resource byte fields replaced by typed PinRef slots; a digest alone is insufficient. pinRefs are unique and sorted by token bytes and correspond one-to-one with descriptor slots. D7/D8/D9 ownerKind values cannot produce managed v2 success until those afterimages exist.

### 3.3 InputDescriptor/2

InputDescriptor/2 is exactly:

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

sourceInputs sort by EntityRef canonical key then role before=0/dependency=1; each pair is unique. control kind is closed to policy|registry|placement_range|lifecycle_range|relation_range|calendar_scope|replica_registry|conflict_record|execution_resource and sorts by fixed kind rank plus owner key. key is an existing owner-defined closed value, never a free JSON path.

Complete equality requires InputDescriptor canonical bytes, owner descriptor bytes, and each referenced exact pin to match. Equal SHA-256 does not mean equal input.

### 3.4 PreparedIntent/2

PreparedIntent/2 exact semantic members are:

kind, version, planToken, operationId, workspaceRef, commitDomain, principalAudienceToken, inputDescriptor, beforeCut, proposedState, mutationFootprint, dependencyProof, observationProof, budgetBinding, pinDirectory, installationPlan, expiresAt, previewBinding.

kind=d6_prepared_intent and version=2. planToken tag=d6_plan/2. pinDirectory contains all PinRef/2 plus protected exact bytes/source bindings. installationPlan freezes component write set, planned ChangeId/SourceVersion/metadata versions, required BackendQualification, and portable records; commit never resamples another after state. previewBinding only references the owner’s complete preview; D6 does not invent a second effects format.

PreparedIntent/2 is not an author decision. An unreferenced prepared record may release large pins after expiry according to retention. Once planned/saved or protected by external unknown/conflict/approval-money, a last-reference pin is not removed by preview TTL.

## 4. D6 v2 prepare and commit requests

### 4.1 Existing Document source-save prepare

D6-owned ordinary/complete existing-Document source prepare is exactly:

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

ownerNodeRef belongs to the Workspace; expectedSourceVersion.entityRef and domain match. source is proposal input and Core immediately pins its exact UTF-8 bytes. The canonical commit request never includes it. Physical-invalid external bytes use repair/external-observation paths, not this API.

ordinary requires D2 valid plus all actually modified local typed facts. Allowed unproved complete obligations become semantic_pending. complete requires every applicable D4/D5/D7 complete obligation; failure remains semantic/dependency/availability and never auto-downgrades.

Success:

~~~json
{"wireVersion":2,"kind":"d6_prepared_intent","planToken":<Token>,"semanticState":<SemanticState/1>}
~~~

This is prepared, never saved.

D3 identity/lifecycle/create/move/reorder/Trash/restore/purge are forbidden here. They wait for a D3 afterimage that owns CommitDomain/profile; D6 does not create an equivalent D3 patch.

### 4.2 d6_commit_request/2

Exactly:

~~~json
{"wireVersion":2,"kind":"d6_commit_request","operationId":"uuid-v4","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"expectedDomainFenceToken":<Token>,"planToken":<Token>}
~~~

No source, patch, budget, preview, or cost override is carried. expectedDomainFenceToken tag=d6_domain_fence/2 binds current domain qualification: active ReplicaEpoch/portable registry/policy/backend epoch for replica and additionally current D3 authority/custody/fence generation for server. It is not permission.

The v2 ledger key is exactly (workspaceId, D3-CJ/3(commitDomain), operationId), with protocolOwner=D6 in the record. Same key/different canonical request is operation_id_conflict. Future D3 wire12 sharing the v2 key also collides by protocol owner; it is not claimed implemented until the D3 afterimage exists.

## 5. Unique submit/install/seal/publish order

The only v2 order is:

1. closed decode and static Workspace/domain equality; invalid_request/preflight, zero business reads;
2. current authenticated principal target state disclosure, potential observation profile, and actual CommitDomain-use qualification; only minimum token/audience/domain location may be read; not_visible/preflight on failure;
3. domain qualification: replica validates active epoch, portable registry/policy, backend identity/observation continuity; server additionally authority/custody/fence. Unprovable -> domain_unavailable; proven corruption -> integrity_conflict;
4. read v2 ledger key. Different owner/request -> operation_id_conflict. A saved decision replays original bytes after current replay authorization; planned recovers original plan. Preview TTL is irrelevant once a decision exists;
5. for unseen, expectedDomainFenceToken must be current; validate plan token tag/audience/workspace/domain/expiry and read complete PreparedIntent/2. Missing/wrong audience/tag -> not_visible; known-self expired -> plan_expired;
6. revalidate Frontier, before source/control versions, actual MutationFootprint authorization, local/complete semantic proof, budget and every unwritten dependency. Deterministic business conflicts may be recorded rejection; availability uncertainty is never recorded as permanent rejection;
7. planning CAS compares ledger unseen, domain fence, expected Frontier, required dependencies/authorization and persists fixed plan, pins, planned ChangeId/poststate and installation recovery. CAS loser restarts at step 3;
8. before modifying any current file, durably write InstallationNotice/1; failure leaves planned;
9. install each component only through BackendQualification. Binding mismatch -> conflict/paused. Unprovable safe install -> install_unavailable/paused. Unknown competing bytes are never overwritten;
10. installed verification compares written targets against **planned poststate**, not original before. Revalidate unwritten positive/negative dependencies, policy/auth, Registry/rules, frontier control. Produced versions compare to their planned poststate;
11. seal: one durable-control transaction writes committed, receipt, ReliableSaveState, effects metadata, applicable ApprovalUse/Money charge, and outbox. This is the author-decision commit point;
12. portable publication writes/flushes ContentCompletionProof/1 and advances portable Frontier. Failure does not roll back step 11; current state is reliable + publication_pending;
13. response delivery revalidates current authorization. Revocation may hide receipt delivery but never changes a committed decision.

Crash in steps 9-12 is resolved from the state/read interfaces, never by client inference. D6 v1 keeps its original order/decoder and is never reinterpreted through this sequence.

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

### 6.2 InstallationNotice/1

Exactly:

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

changeId.domain equals commitDomain. components is non-empty, uniquely keyed, and sorted by fixed PortableComponentKey rank+canonical key. Notice is durable before the first current-component install. It is not a commit proof and contains no receipt, approval, Money, external payload, or credential.

### 6.3 ContentCompletionProof/1

Exactly:

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

frontierAfter equals frontierBefore advanced to changeId.sequence in changeId.domain, never regressing another head. components exactly match the InstallationNotice key set and actual sealed after images. receiptDigest binds control receipt bytes but never grants receipt read or execution authority to another replica. Authenticity comes from the portable trust/record chain plus actual components, not the digest alone.

## 7. Commit receipt, decision state, and current source state

### 7.1 d6_commit_receipt/2

Exactly:

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

domainCommitSequence is the D3/D6 committed author/control sequence inside one CommitDomain. Fresh domain starts at 0 and first commit is 1. Different domain sequences are not a workspace-global activity order. A later D7 commit_sequence_state consumer must be domain-scoped. Policy/2 legacy behavior remains historical only.

sourceVersions contains only actually changed sources, sorted by EntityRef canonical key. Receipt bytes are immutable at seal, so portablePublicationAtReceipt is always pending for a file-backed content decision; later publication never rewrites receipt and is read from decision state. A raw-no-op/control decision uses the appropriate owner contract rather than inventing a content ChangeId.

### 7.2 d6_decision_state_read

Request:

~~~json
{"wireVersion":2,"kind":"d6_decision_state_read","protocolOwner":"D6|D3","request":<original-complete-request>}
~~~

D3 wire12 does not yet exist, so the new D3 branch returns unsupported_version until coordinated activation. The complete original request is required rather than bare OperationId.

Success:

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

decisionSourceVersions belong to the original decision, not current workspace source. If r5 is committed and current source is r6, this response remains r5; current source uses the separate API.

Authorization order is closed decode -> current replay disclosure/target scope -> CommitDomain continuity -> ledger key/fingerprint -> state. Revoked -> not_visible without changing decision. Unprovable continuity -> domain_unavailable. Original-request mismatch -> state_unavailable and no other-key disclosure.

### 7.3 d6_current_source_read

Request:

~~~json
{"wireVersion":2,"kind":"d6_current_source_read","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"entityRef":<NodeRef|ResourceRef|AnnotationRef>}
~~~

Current entity/source read permission precedes domain/backend and current FileBinding/metadata access.

Managed success:

~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"managed","sourceVersion":<managed SourceVersion/2>,"semanticState":<SemanticState/1>,"byteLength":<Counter>}
~~~

External valid/pending adoption:

~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external","sourceVersion":<external SourceVersion/2>,"validation":"d2_valid|unverified","byteLength":<Counter>}
~~~

External invalid:

~~~json
{"wireVersion":2,"kind":"d6_current_source","state":"external_invalid","sourceVersion":<external SourceVersion/2>,"validation":"physical_invalid|d2_invalid","byteLength":<Counter>}
~~~

These objects do not contain full bytes. Bytes use the corresponding authorized source/ByteHandle/repair path. external/unverified never means managed commit success.

## 8. Installation state and crash recovery

Each planned v2 decision stores per-component internal state pending|staged_durable|installed_after|restored_before|third_state|unavailable.

Recovery reads P, then classifies actual components only as exact_before, exact_after with proved installation provenance, third_state, or unavailable. Equal bytes without provenance do not become exact_after.

A written exact_after component is validated against planned after in commit step 10; unwritten dependencies remain compared to before/cut expectations. This separation is mandatory and prevents the incorrect rule “after installation every expectedSourceVersion must still be before”.

planned recovery reuses the original InputDescriptor, pins, ChangeId, versions, OperationId, and budget counters. If plan pins/clock/domain continuity cannot be proved, remain planned+paused/recovery_unknown; never reprepare another decision in its place.

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

New no-argument capabilities are replica_register | replica_retire | conflict_read | conflict_resolve | execution_custody_admin.

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

code is closed to invalid_request | unsupported_version | not_visible | domain_unavailable | integrity_conflict | operation_id_conflict | plan_expired | dependency_conflict | semantic_rejected | budget_exceeded | install_unavailable | conflict | conflict_changed | state_unavailable | owner_update_required | effects_unavailable | transaction_aborted.

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

D6 wire1 commit/receipt/error, Policy/1/2, SourceVersion/1, legacy Token tags, PreparedIntent, D7 PreparedActionBinding/1,/2, D8 PreparedEditBinding/1, and their planned/saved decisions continue with original bytes, fingerprint, authorization/continuity and pin-retention rules.

Forbidden: re-encoding an old request as wire2; adding CommitDomain/ChangeId to an old receipt; interpreting an old D4 gate as semantic_pending; using new retention to delete evidence promised by the old contract; treating equal source hash as proof that SourceVersion/1 equals a new-domain source.

New v2 consumers are incomplete. Required D3/D4/D5/D7/D8/D9/D10 owner afterimages must be authored and accepted together. This candidate may not produce “v2 managed commit success” in product or conformance fixtures as activated semantics. Author documentation checks are not a substitute for the coordinated gate.
