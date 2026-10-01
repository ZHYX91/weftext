---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: `592603c4-c7ef-4572-aee6-256aa3aa7955`.

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. D7/D9 activation labels and revision05 prose in fixed S are historical provenance only and do not govern this afterimage; the stable document ID is preserved. The D6 producer boundaries now authored in this candidate include the complete managed/external SourceVersion/2 production history, current-observer SourceObservation/1 and SourceVersionRef/1, SourceStamp/1, protected internal SourceRevisionPlan/1, RevisionTokenBinding/2 with d6_source_revision/2, ContentCompletionProof/3, and the current version boundaries of existing Frontier/2, InstallationNotice/2, and ConflictRecord/2. Existing DecisionKey/2, InputDescriptor/2, PreparedIntent/2, D3DecisionCompanion/2, and other wire shapes remain as defined in the body and are not versioned by this status line. Production CommitDomain remains distinct from current observerDomain; D3 Locator lexical ownership, D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence, and the D5 revision-bound locator remain with their actual owners. Ordinary .adoc and Resource bytes remain author authority, P retains non-reconstructible execution/recovery facts, I remains rebuildable, and ordinary-content qualification stays distinct from complete-Action qualification. D3 wire11, D6 wire1, Policy/1/2, and actual saved legacy D7/D8 binding bytes continue under their original decoder/gate/pin/continuity obligations. Remaining P1 Lexicon/Registry/Impact/routing, P2/P3, and actual D7–D10 consumers have not completed fresh joint acceptance. Partial activation of managed success that depends on the new producers is forbidden, while approved ordinary/local/offline operations that do not depend on the missing strong-path coordination are not permanently disabled.

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
sequence is 1..MAX. One real portable effect receives exactly one ChangeId by checked allocation inside P seal for that decision. prepare, planning, staging, and InstallationNotice reserve no successful sequence. Failure, paused, recovery_unknown, control_only, and true raw no_op create no content ChangeId. Source deletion and source-unchanged portable placement/lifecycle/structure effects still receive the portable decision's ChangeId at seal, but create no managed SourceVersion and do not advance H(D,E). A sealed ChangeId is never reallocated because receipt delivery, outbox, or portable publication later fails. ChangeId is not OperationId, EntityRef, sourceOccurrenceKey, or global time.

Frontier/2 is:
~~~json
{"kind":"d6_frontier","version":2,"heads":[<ChangeId/1>...]}
~~~
heads may be empty; otherwise they are sorted by canonical CommitDomain key and unique by domain, representing the greatest verified continuous sealed causal prefix in each listed domain. Frontier/2 proves neither payload materialization, placeholder download, complete Query scope, Registry completeness, D7 complete cut, nor execution responsibility. Frontier/1 remains legacy decoder/replay only.

frontierPolicy remains exactly exact|scope_dependencies. `exact` requires the complete Frontier used for an unseen/planning decision to be byte-equal to InputDescriptor.expectedFrontier. Every D3 managed_atomic path, including a future D7-mediated D3 managed_atomic request, remains on this exact branch unless D3 itself explicitly versions that rule; D6 never weakens it unilaterally.

`scope_dependencies` admits only a **real continuous verified-sealed causal non-regressing extension** from the original expectedFrontier to the current Frontier. Per-domain nondecreasing sequence numbers are necessary but never sufficient: an original domain never regresses or contains a hole, and every added/advanced head is backed by a complete continuous sealed-record chain. Provider “synced” state, index readiness, mtime, equal final hash/bytes, or comparing the two vector numbers is not such evidence. If continuity or unrelatedness cannot be proved, the extension is not admitted.

Selecting scope_dependencies never rewrites the original canonical request, InputDescriptor.expectedFrontier, DependencyProof.baseFrontier, targets, Query/selector, pins, proposed bytes, MutationFootprint, WriteProtection, owner request, or version basis. Core revalidates the complete original source/control/authorization dependencies, every positive/negative range DependencyProof, FileObjectBinding/evidence pins, Registry/rules, installation qualification, owner version, and every other bound dependency and proves that the newly sealed effects are unrelated to them. A changed SourceObservation, the observation selected by a source token, FileObjectBinding, pin, DependencyKey stamp, authorization, Registry, relation/calendar/collection membership or negative range remains stale/conflict/reprepare under its owner rules; scope_dependencies never relabels such a change as unrelated.

A proved unrelated Frontier extension does not re-sign SourceObservation/1, SourceVersionRef/1.sourceToken, revision token, PinRef, or DependencyProof stamp, and never carries an old token into a new observationEpoch. Those objects survive only under their own continuity rules. Conversely, a qualified ordinary operation whose contract uses complete real local evidence is not forced to wait for an unrelated whole-Workspace Query/index proof merely because scope_dependencies is selected. D3 replica_local create/move/reorder/trash retains its owner-defined local_structure + scope_dependencies path. Until P2 D3 actually consumes the new DependencyKey/recovery refinements, branches depending on them remain owner_update_required/proof_unavailable rather than being implemented by this paragraph.

The actual pre-seal Frontier of a portable decision may therefore include a proved unrelated extension under scope_dependencies, while original expectedFrontier and notice.baseFrontier remain unchanged. New-FA ContentCompletionProof/3 records the actual frontierBefore/frontierAfter and cross-validates them under §6.3 against the original notice, continuous change records, and the unrelated-extension evidence retained in P.

### 1.5 SourceVersion/2, SourceObservation/1, SourceVersionRef/1, and revision-token profile/2

The existing SourceVersion/2 closed union is unchanged.

Managed:
~~~json
{"kind":"managed_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"revision":<Counter>,"changeId":<ChangeId/1>}
~~~

External:
~~~json
{"kind":"external_source_version","version":2,"entityRef":<EntityRef>,"commitDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"externalSequence":<Counter>}
~~~

observationEpoch/revision/externalSequence are 1..MAX. A managed changeId.commitDomain equals its production commitDomain. A Document entityRef is its owner NodeRef; Resource/Annotation use complete Refs. The external variant likewise retains its complete existing kind, version, entityRef, commitDomain, observationEpoch, and externalSequence fields. It has no managed revision or changeId, and externalSequence is never interpreted as a managed sourceRevision.

For production domain D and entity E, `H(D,E)` is the greatest managed revision of E in D's continuous sealed production history. H=0 is a complete empty history only when CommitDomain birth/registration, P continuity, and verified portable sealed history prove that D has never sealed a managed source version for E. Missing, corrupt, gapped, or otherwise unproved history is never treated as empty. Every source change that actually produces a managed after uses checked H+1. production observationEpoch changes do not reset H within a production domain, and MAX never wraps. A fresh managed source is revision=1 in a proved empty history.

When an existing source is first written by another production domain, its new managed after uses that new production domain's own H+1, never the old production domain's revision+1. Later cross-domain returns continue each domain's own H. Equal revision numbers in different production CommitDomains are not equal versions. A true raw no-op preserves the original SourceVersion/2 even when its production domain differs from the current operation. Pure placement/lifecycle/control with unchanged source does not increment source revision. Source deletion has after=absent, creates no "deleted version", and does not advance H. Equal-byte external admission is an explicit external-to-managed admission and is not a raw no-op; it still creates the managed after from the current production domain's H+1. externalSequence never participates in managed H or an inner sourceRevision.

SourceVersion/2.observationEpoch is the production-observation generation retained by that production version. The current operation's observation generation is independently recorded by SourceObservation/1; equal numeric values never collapse these roles. Current replica/Server observation remains:
~~~json
{"kind":"d6_source_observation","version":1,"observerDomain":<CommitDomain/2>,"entityRef":<EntityRef>,"sourceVersion":<SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<FileObjectBinding/1>,"evidencePins":[<PinRef/2>...]}
~~~
observerDomain equals the current operation CommitDomain and entityRef equals sourceVersion.entityRef; evidencePins are token-sorted/unique. sourceVersion.commitDomain may differ from observerDomain. The current observationEpoch, FileObjectBinding, evidencePins, and applicable control/Registry/incidence dependencies belong to the same current cut. Placeholder, missing metadata, a conflict branch, or unproved observation continuity yields no successful SourceObservation. A watcher gap, external replace, object replacement, or discontinuous rematerialization moves to a new current observation generation; an equal production SourceVersion, digest, or final text never restores the old current-observation qualification.

The narrow public projection remains SourceVersionRef/1:
~~~json
{"entityRef":<EntityRef>,"sourceToken":<Token>}
~~~
sourceToken tag=d6_source_observation/1 selects a complete protected SourceObservation, not a bare revision, digest, I cache, or production SourceVersion. It does not replace any D3/D4/D5 inner revision/selector wire.

New-decision source revision tokens are managed by protected RevisionTokenBinding/2. Its closed shape is:
~~~json
{"kind":"d6_revision_token_binding","version":2,"token":<Token>,"observerDomain":<CommitDomain/2>,"observationEpoch":<Counter>,"source":<RevisionTokenSource/2>}
~~~
RevisionTokenSource/2 has exactly two variants:
~~~json
{"kind":"managed","sourceStamp":<SourceStamp/1>}
~~~
or
~~~json
{"kind":"external","sourceVersion":<external SourceVersion/2>}
~~~
The external sourceVersion must decode as the complete external SourceVersion/2 above. The managed sourceStamp is produced only by the §6.2 SourceRevisionPlan/1; it may identify a proposed managed revision inside that original plan and, after that decision seals and its managed SourceVersion is proved, remains the stable binding basis for that revision token. RevisionTokenBinding stores no current source bytes and owns no D3 Locator, D4 occurrence, or D5 table locator.

`token` uses the §1.1 43-ASCII canonical Token lexical form with protected tag d6_source_revision/2; callers cannot construct either tag or binding. observerDomain and the bound entity belong to the same Workspace, and observationEpoch is that observerDomain's current observation generation for the source. For managed source, sourceStamp.entityRef belongs to observerDomain.workspaceRef while sourceStamp.decisionKey.commitDomain is the production domain that produces the managed after. For external source, sourceVersion.entityRef belongs to observerDomain.workspaceRef. A legitimate cross-production-domain observation never requires the production domain to equal observerDomain.

The binding mapping is durable. A proposed managed binding lives only with the original plan in P before seal. Only after successful seal, when that token is required as version evidence by committed source/Locators, is the same immutable binding/token carried in the corresponding protected portable source-version metadata for other replicas to resolve after complete transport validation. The record stores version/observation binding only, never source bytes, read permission, or write permission. An external binding exists only under its current observation qualification; without managed seal it is never presented as a portable managed binding.

Stability is keyed by the binding key excluding the token member: `(observerDomain,observationEpoch,source)`. The same key must reissue the same token while that observation generation remains continuous. Retry, restart, or reread never mints an equivalent second token for that key. The token is not derived from source bytes, digest, revision number, or path. A changed observerDomain or observationEpoch, a gap/replacement, or different binding content requires a different binding/token. Equal production version or text never revives an old token.

Before seal, a managed sourceStamp may be consumed only by Core inside the same original plan for the D3 stage12 private candidate map, D7 DefinitionTransfer two-pass Q/Locator materialization, and other explicitly managed proposed-position validation. It grants no current locator capability and is never presented as a committed SourceVersion. After the original plan seals, the token is eligible for currentness validation only when the same DecisionKey's sealed managed SourceVersion exactly corresponds to the stamp's entityRef/revision/observationEpoch, its ChangeId belongs to that sealed decision, and the current complete SourceObservation remains valid. An aborted/terminal-failed plan or unproved seal never turns a proposed token into a current token.

Historical d6d/d6r/d6a revision-token profiles, the original opaque lexical decoders for DocumentRevision/ResourceRevision/AnnotationRevision, their original store-incarnation/numeric-revision meaning, and saved bytes all remain intact. They are never re-encoded as d6_source_revision/2 and no new D6 lexical pre-gate is inserted into an old D3 Locator decoder. The new profile resolves only through its protected binding. D3 Locator members continue to carry revision tokens as the existing opaque string, while D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence and D5 revision-bound locator wire remain shape-identical.
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

### 3.4 DependencyProof/2 and DependencyKey/2

The existing DependencyProof/2 closed envelope remains:
~~~json
{"kind":"d6_dependency_proof","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"baseFrontier":<Frontier/2>,"entries":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>},"evidencePins":[<PinRef/2>...]}...]}
~~~
The actual `entries` array may be empty or contain multiple items, and each `evidencePins` array may be empty or contain multiple items. Array cardinality, uniqueness, and ordering are defined here; omission of a JSON member never means “unknown”.

DependencyKey/2 is a closed union of exactly fourteen kinds. Every key contains exactly `kind`, `workspaceRef`, and the additional members listed for its variant. Unknown/missing/duplicate members, illegal null, wrong union arm, wrong Ref domain, or an extra alias is `invalid_request`. `workspaceRef` equals DependencyProof.workspaceRef and commitDomain.workspaceRef. Ref sets use the existing D3 RefKey canonical ordering with no duplicates; FieldId sets use D4 canonical ordering. Author sibling order, Entry order, relation-fact order, and other semantically ordered lists are not sets and are never reordered merely to canonicalize a DependencyKey.

The fixed kind ranks are source=0, lifecycle=1, placement_range=2, ref_inbound=3, relation_incidence=4, calendar_scope=5, registry=6, temporal_rules=7, authorization=8, foreign_binding=9, query_scan=10, replica_registry=11, conflict_record=12, execution_resource=13. DependencyProof.entries sort first by kind rank and then by unsigned byte-lexicographic order of D3-CJ/3 canonical UTF-8 bytes for the complete key; complete keys are unique, so the same key never appears twice under different stamps. `evidencePins` are PinRef.pinToken-canonical sorted/unique. Refs and nested scopes belong to the outer Workspace unless an original D3/D4 author-value type explicitly permits preservation of a foreign Ref; preservation never grants enumeration or disclosure in that foreign Workspace under this key.

The fourteen exact key shapes are:

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

`decisionKey.workspaceRef` equals execution_resource.workspaceRef. `principalAudienceToken` comes from the trusted host/authentication audience binding rather than caller-declared principal data. It retains the §1.1 Token lexical profile and protected audience mapping and never becomes a capability merely because it appears in a key.

#### StructureRange

StructureRange is the placement_range nested closed union with exactly these nine variants:

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

Every NodeRef/AnnotationRef belongs to the outer Workspace and passes the original D3 Ref decoder; Resource/Annotation owner-local ranges retain D3 owner rules. `live_children` and `trash_children` prove the complete ordered child list of that parent and preserve D3 sibling order rather than Ref-sorting it. `trash_roots` proves the complete Trash-root list. `ancestor_chain` proves the complete chain to the root in the named forest and its no-cycle condition. `subtree` includes root itself and covers the complete subtree in the named forest. `owner_resources` and `owner_annotations` are complete owner-local directories and are never filtered by the current UI. `reply_closure` and `restore_membership` use the original D3 Annotation reply-closure and Trash restore-membership semantics. An operation requiring several ranges lists several placement_range keys; no free `closure` string merges them.

D3 remains owner of lifecycle/placement/ref-inbound enumeration algorithms. This file freezes the key carrier, completeness proof, and D6 commit/recovery consumption boundary without becoming a second identity owner. Until P2 D3 afterimages consume these keys, a strong path requiring their new semantics remains `owner_update_required`/`proof_unavailable`; an otherwise qualified ordinary local operation that does not depend on such a range is not permanently disabled.

#### CalendarRange, SeriesScope, and RegistryBinding

CalendarRange has exactly four variants:

~~~text
{kind:"binding",nodeRef:NodeRef}
{kind:"series",seriesScope:SeriesScope}
{kind:"period",seriesScope:SeriesScope,periodKey:<D4 canonical PeriodKey>}
{kind:"scope_inbound",scope:CalendarScope}
~~~

NodeRef belongs to the outer Workspace. CalendarScope reuses the D4 closed union exactly:

~~~text
{kind:"workspace",workspaceId:<current WorkspaceId>}
OR
{kind:"node",scopeNodeRef:NodeRef}
~~~

A node scope's scopeNodeRef belongs to the same Workspace. SeriesScope has exactly `series,scope,policyBinding`:

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

The seven `series` members are the same canonical semantic values from the D4-validated author Entry; title, path, locale, and device timezone never supply defaults. periodKind is closed to `day|week|month|quarter|year`. seriesKey retains D4 required exact TypedText semantics, including legal empty-string author value. RegistryBinding/1 byte-matches both `registryGeneration` and `snapshotDigest` of the authenticated RegistrySnapshot/1; generation or digest alone never substitutes. policyId, policyVersion, and policySchemaDigest match the verified CalendarSeriesScopePolicy/1 contribution under that same RegistryBinding.

`periodKey` is decoded through the verified calendar period-rule contribution's keyProfile into one canonical semantic key. ISO-v1 lexical profiles remain day=`YYYY-MM-DD`, week=`YYYY-Www`, month=`YYYY-MM`, quarter=`YYYY-Qq`, year=`YYYY`, with the original D4 Gregorian/ISO-week/bounds validation rather than regex-only acceptance.

`binding` proves one period Node's CalendarPeriodScopeBinding, its independent binding revision, the actual source period/series interpretation, and current configuration; no active binding is fabricated when no valid period Entry exists. `series` proves all periodKey membership for that series+scope, its SeriesScopeConfiguration, the complete negative range, and applicable control inbound. `period` proves every member in one complete `{series,periodKey,scope}` range; unique/many comes only from managed configuration and is never chosen from current hits or one create. `scope_inbound` proves all period bindings/configuration/control inbound referring to the scope. Empty range still has its own range stamp. D4 owns Calendar semantics, while D6 owns persistence/stamps for managed configuration/binding. Strong Calendar success remains gated until the related D4/P2 consumer actually consumes this contract.

#### QueryScanSelector

QueryScanSelector has only:

~~~text
{kind:"workspace"}
{kind:"entities",refs:[EntityRef]}
{kind:"subtree",root:NodeRef,includeRoot:Boolean}
~~~

`entities.refs` is non-empty, D3-RefKey sorted and unique, and matches domain: nodes admits NodeRef only, resources ResourceRef only, annotations AnnotationRef only, and headings only NodeRef as heading source owner. `subtree` is valid only for domain=nodes or headings; root is a same-Workspace NodeRef and includeRoot is a JSON Boolean. For headings, the subtree selects headings whose source Nodes are selected; it creates no HeadingRef identity.

refs are the validated result of the owning D7 Query selector rather than a second client query language. QueryScanSelector contains no CEL/arbitrary predicate, ResultHandle, row handle, cursor, page position, sort/take, or free payload. Full Query expression, SavedQueryDefinition, pre/postQuery, parameters, ordering, and original-plan binding remain D7-owned. A complete headings scan additionally reads the accurate D2 source for every applicable Node. physical/D2 invalid source, placeholder, source unavailable, or unproved coverage is never silently skipped to claim complete. Until the D7 afterimage consumes the new query_scan contract, a strong complete path depending on it remains owner-gated; the union name alone is not D7 acceptance.

#### Owners, completeness, and disclosure for the fourteen kinds

- `source`: D6 owns observation/version/file-binding proof while content semantics remain with the Node/Resource/Annotation owner. A positive proof binds the complete current SourceObservation/1, exact bytes/value pin, FileObjectBinding, current source validity, and actual use by this operation; SourceObservation.entityRef equals key.entityRef. An absent source is proved from real identity/lifecycle/FileBinding plus a trusted absent object and never from a placeholder, I miss, I/O failure, or “not downloaded”. A narrow Field path may let trusted Core read/preserve a complete source internally, but without source_read it never returns body bytes to the principal.
- `lifecycle`: D3-owned. It proves complete birth/canonical claim, live/Trash/tombstoned/never-known state and applicable owner relation independently of source revision. Original D3 state disclosure precedes existence/lifecycle access; absence comes from complete identity inventory, never derived-index miss.
- `placement_range`: D3-owned. It completely enumerates the selected StructureRange, including boundaries, order, cycle conditions, and absence. structure_state/original D3 disclosure precedes hidden sibling reads; Core never scans hidden siblings and then chooses a “safe” profile.
- `ref_inbound`: D3 identity/lifecycle reference-closure owner; concrete foreign/control slots remain with their original owners. It completely enumerates the original D3 reference slots, lifecycle/control inbound, and live/Trash applicability for key.target. Zero inbound requires complete directory/stamp. Future D7 SavedQueryDefinition is not smuggled into D3 node_link/citation union.
- `relation_incidence`: D4-owned. It reuses the exact RelationReadContext/2 incidence scope: fieldId, endpointNodeRef, revisionToken, and complete factSelectors where a factSelector remains `{ownerNodeRef,fieldId,occurrenceKey}`. It proves canonical owner, source/entity state, incidence coverage, and complete positive/negative set. A literal arm has no Node incidence and a symmetric fact is not duplicated. masked/unprovable cannot produce complete success. Retaining a legal foreign NodeRef author value does not authorize cross-Workspace enumeration under this key; strong semantics needing foreign current state remain owner-gated.
- `calendar_scope`: D4 semantics plus D6 managed-configuration persistence. CalendarRange proof covers period scope binding, SeriesScopeConfiguration, RegistryBinding/policy, complete period membership/negative range, and applicable control inbound. Configuration absent, binding absent, and empty period range have independent versions and are never inferred from index empty/current page. unique continues to group by complete `{series,periodKey,scope}`.
- `registry`: D4 is the semantic owner, while provider/trust authenticity remains owned by the real provider path. The first profile conservatively binds the complete same-Workspace RegistrySnapshot/1, RegistryBinding/1, required RegistryEvolutionProof, and all Field/Facet/alias/namespace/contribution directories. A complete directory does not mean every definition is available; an unavailable or unknown definition is distinct from a proven complete empty definition set. An operation that truly does not depend on Registry need not add this key.
- `temporal_rules`: D4 semantics with D6 persistence of binding evidence. The first profile conservatively binds the Workspace's admitted temporal-rule directory and records every actually read calendar comparator, period rule, timezone/tzdb rule set, RecurrenceReadBinding/1, and finite horizon coverage. It does not materialize infinite time. Missing segments, unproved rule provenance, and an unsupported business value are distinct; device timezone never fills a gap.
- `authorization`: D6-owned. principalAudienceToken comes from trusted principal/session/delegation mapping. Proof binds current Policy/3 version/auth generation, delegation, original ObservationScope/2, and applicable metadata capabilities. Any change that can affect this principal's input/result disclosure or write qualification changes the stamp. Public DependencyProof never reveals hidden grants, deny members, or author values. deny-before-allow, default deny, write-not-read, and separate source_envelope_state/commit_sequence_state/structure_state/portable_frontier_state disclosure boundaries remain.
- `foreign_binding`: D3 owns binding semantics/identity constraints, D6 persists managed directory/continuity, and D9/D10 or the concrete source owner owns external-version format/comparison. The first profile conservatively covers the complete relevant SourceBinding/OriginBinding directory, active/retired history, mapping/version, and original comparator. Equal UID, etag, path, digest, or row text never substitutes for binding. Records decode only under their known original version. An unknown profile or missing real source decoder makes the dependent strong path `proof_unavailable`/owner-gated; no generic JSON wrapper is invented and ordinary file work not needing that proof remains available.
- `query_scan`: D7-owned, with D6 carrying key/stamp. Proof establishes complete visible enumeration and hidden-policy generation for principalAudienceToken under the selected domain/selector plus every real source/control dependency that had to be read. headings has the extra D2-source rule above. Saved definitions keep their original SavedQueryDefinition source/address-resolution dependencies; pre/postQuery linkage stays bound to the original immutable Prepared plan. A proposed fresh object with no sealed source/version never gets a fabricated “committed query_scan stamp”.
- `replica_registry`: D6-owned. It proves the complete active/retired ReplicaRecord directory, registrationSequence, and gap-free continuity. purge additionally validates each participating replica's real admission of the named purge Frontier or explicit retirement. Registry count, provider “synced”, one replica Frontier, or an index row never substitutes for acknowledgement. A protocol stage that permits an empty directory still proves the complete directory.
- `conflict_record`: D6 owns records/directories while resolution semantics remain with the original D3/D4/D6 domain owner. selector=id proves the exact ConflictId record under its version decoder; selector=subject proves the complete conflict directory covering that subject, including open/resolution_prepared/resolved/superseded relationships and real heads. conflict_read plus minimum subject disclosure precedes access; unauthorized/unknown follows not_visible. External competition, third_state, or unknown install with no sealed head never fabricates ChangeId for ConflictKey.
- `execution_resource`: D6-owned. The key binds one complete DecisionKey/2 and exact protocolOwner and reads that original operation's resource-policy revision, attempt allowance/consumption, cumulative work, protected pins/capacity, and pause category under the existing execution-resource control-read authorization. decisionKey only adds explicit CommitDomain location and never permits OperationId enumeration. Absence, P loss, or missing historical summary never resets quota, refunds work, or reissues eligibility. This key does not absorb D10 Run/Lease/Automation/Workspace/deployment Money lineage, provider unknown, or sourceOccurrenceKey; U6/D10 continues to own those continuity contracts.

#### Stamp generation, continuity, and error mapping

The `stamp` shape remains exactly `{"epoch":<Token>,"revision":<Counter>}`; no epochToken/revisionCounter aliases are created. epoch is the continuity generation of proof for this exact DependencyKey, not SourceVersion production observationEpoch, SourceObservation.observationEpoch, CommitDomain generation, or a Workspace-global generation. Its protected record binds back to the complete DependencyKey/2 and is never caller-issued.

After a real complete enumeration, a new epoch may start at revision=0. Zero means only “first complete proof revision in this new range generation”; it never means source revision0, unknown, empty, or not-scanned. Within a provably continuous event stream for the same epoch, any source, identity, membership, order, control, authorization, or visibility change that can affect the key first invalidates the old current proof; revision then checked-increments and the required current validation completes before a new current stamp is published. MAX never wraps.

A watcher/journal gap, owner-rule/decoder version change, unrecoverable event gap, actual loss of correctness-evidence directory, or unproved range continuity creates a new epoch. Equal final content/hash/member count never restores the old epoch. Deleting or rebuilding Derived Index alone does **not** delete correctness-control facts still complete in their actual Durable Control Store/Portable Workspace Metadata owner. If those facts and the continuous event stream remain intact, I rebuild only repopulates candidate cache and neither re-signs proof nor changes epoch. If the real proof/continuity facts were actually lost, the old proof is permanently invalid and only a new complete current-authorized enumeration establishes a new epoch; I or repeated hash cannot resurrect it.

Empty and non-empty ranges have the same completeness standard: establish the key's required state/content/metadata authorization first, then traverse every applicable directory, shard, owner range, and source and prove no omission. Placeholder, unknown decoder, I/O failure, missing shard, hidden unauthorized range, partial index, or “no candidate hit” cannot produce empty success. Complete enumeration is backed by a trusted consistent snapshot/range read barrier or a complete gap-free change stream plus final revalidation. Repeated scans with equal hash are not a consistent cut. If the backend can prove only local evidence, complete proof is unavailable, while an ordinary operation whose contract requires only that local evidence may continue under its own qualification.

The last evidence pin/range proof still referenced by a planned decision remains durably protected under its real P/portable owner and PinRef retention. Capacity shortage rejects creation of a new proof/plan or puts the planned operation in paused_capacity; Core never drops last-reference evidence and guesses it back from I. A proof stores closed key, stamp, required pins, and protected enumeration/continuity evidence, not another current-body mirror.

Only existing public error/state surfaces are used:
- current disclosure/authorization failure is `not_visible` and stops before hidden-range access;
- authorized but currently unreadable source bytes use the applicable `source_unavailable`;
- authorized but complete range/decoder/continuity proof cannot be established uses `proof_unavailable`; unproved CommitDomain/P continuity itself remains `domain_unavailable`;
- for an unseen canonical intent, a complete current proof that deterministically establishes an impermissible change of a bound dependency uses the existing step-6 `dependency_conflict`;
- after a decision is planned, revocation, unknown continuity, or installation competition is never converted into a new recorded business rejection. A proved competing dependency follows the original plan's conflict/paused recovery path; unproved continuity, installation provenance, or outcome remains paused/recovery_unknown. The later Control group still updates the exact §5/§8 submit/recovery ordering, but this section forbids hiding uncertainty behind a business terminal outcome.
No error gains free details, hidden counts, partial-member lists, or internal cause.

### 3.5 InputDescriptor/2

The closed shape remains:
~~~json
{"kind":"d6_input_descriptor","version":2,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"intentKind":<controlled-text>,"saveProfile":"ordinary|complete|control_only","guarantee":"replica_local|managed_atomic","expectedFrontier":<Frontier/2>,"frontierPolicy":"exact|scope_dependencies","observationScope":<ObservationScope/2>,"sourceInputs":[{"entityRef":<EntityRef>,"observation":<SourceObservation/1>,"role":"before|dependency"}...],"controlInputs":[{"key":<DependencyKey/2>,"stamp":{"epoch":<Token>,"revision":<Counter>}}...],"ownerInput":<OwnerInputBinding/2>}
~~~
The actual sourceInputs/controlInputs arrays may be empty or contain multiple items; the member set, version, and unions do not change.

workspaceRef/commitDomain are byte-equal to the corresponding outer plan members. DependencyProof/2.workspaceRef/commitDomain equal them and DependencyProof.baseFrontier is byte-equal to expectedFrontier. sourceInputs are sorted/unique by D3-CJ/3 canonical bytes of the complete item; each entityRef equals observation.entityRef and observation.observerDomain equals commitDomain. sourceInputs carry the real current Observation, never SourceVersionRef, hash, or I state.

controlInputs are sorted/unique by the fixed DependencyKey kind rank plus D3-CJ/3 key bytes. Every controlInputs item has exactly one byte-equal key in the same DependencyProof.entries and its stamp is byte-equal; a dependency used by this intent never bypasses InputDescriptor through another stamp. A source DependencyKey used to establish currentness of a sourceInputs item points at the same entityRef and is consistent with that Observation's file object, pins, and current cut; an equal bare revision number from another production domain is never a match.

ObservationScope/2 remains a prior authorized observation upper bound, not dependency completeness or write permission. Every actual DependencyKey read lies within that scope and current Policy authorization. Where the D3/D4/D7 owner algorithm has not yet supplied the complete range required here, the path remains owner_update_required/proof_unavailable; the existence of this union is never itself a completeness proof.

expectedFrontier remains the original prepared input and is never resampled at commit. frontierPolicy=`exact` retains full Frontier equality. `scope_dependencies` admits only the proved unrelated non-regressing extension defined by §1.4/§6.3 while revalidating every bound source/control/auth/positive-negative dependency. An unrelated coarse Frontier advance alone need not fail a local ordinary intent, and a real bound dependency change is never ignored because scope_dependencies was selected.

Complete input equality compares full InputDescriptor canonical bytes, complete OwnerInputBinding descriptor, every protected exact PinRef record, SourceObservation, DependencyProof keys/stamps/pins, and every other fixed owner input. Equal hash, equal final text, equal revision number, or rebuilt I is insufficient. Changing source/current Observation, any key/stamp, mapping, Policy/Registry/rule, scope, frontierPolicy, WriteProtection, owner request, or version basis requires a new prepare. The same OperationId remains exact replay only.

### 3.6 PreparedIntent/2

The PreparedIntent/2 immutable member set remains unchanged:
identity/context are kind,version,planToken,operationId,workspaceRef,commitDomain,principalAudienceToken,inputDescriptor,beforeCut;
proposed state/proof are proposedState,mutationFootprint,dependencyProof,observationProof,budgetBinding,pinDirectory;
installation/delivery are installationPlan,inputRetentionState,expiresAt,previewBinding.

kind remains d6_prepared_intent, version=2, and planToken remains tagged d6_plan/2. principalAudienceToken equals the trusted authenticated audience for this plan. Any authorization or query_scan key in the same plan has byte-equal principalAudienceToken. dependencyProof.workspaceRef/commitDomain/baseFrontier respectively equal PreparedIntent workspaceRef/commitDomain and inputDescriptor.expectedFrontier; the key/stamp relationships between inputDescriptor and dependencyProof satisfy §3.5 item-for-item. observationProof covers sourceInputs current-observation qualification and is consistent with the same current cut's file object/evidence pins and applicable control/Registry/incidence dependencies; it never substitutes for DependencyProof.

pinDirectory covers OwnerInputBinding.pinRefs, DependencyProof.evidencePins, SourceObservation.evidencePins, and proposal/read-before/after/recovery pins actually referenced by installationPlan. The same pin is never rebound to a different protected record. Last-reference pins retain the original retention/capacity rules.

inputRetentionState remains not_retained|retained|unavailable. retained requires durable proposal, actually-read before, and every required binding/pin. prepared/retained is neither Saved nor portable published.

installationPlan still freezes write set, after bytes, owner versions, recovery, required WriteProtection, and portable records, preallocates no ChangeId, and never resamples at commit. SourceRevisionPlan/1 and d6_source_revision/2 from the preceding Control core group are private installation-plan/protected-P version basis only: they never pre-seal SourceVersion, expose a fresh D3 Ref through preview, resample identity outside the D3 stage12 candidate map, or edit revision after the plan wins. Deletion/source-unchanged/no-op branches retain their explicit no-SourceRevisionPlan rules. previewBinding continues to reference owner preview only.

D3/D4 concrete range algorithms, D7 query_scan/SavedQuery/pre-post linkage, and D8/D9/D10 later consumers are not implemented or accepted merely because these carrier types are now defined. Strong success depending on an uncoordinated consumer remains gated. Qualified ordinary `.adoc` reads, human ordinary saves, and offline operations requiring only real local evidence are not permanently disabled by an unrelated missing complete proof.
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

The following order explicitly separates saved, planned, and unseen. prepared/retained means only that proposal, read-before, and required bindings/pins are durable; it is not Saved. No branch may replace an existing record for the same DecisionKey by preparing a second decision.

1. closed decode and static cross-field equality; failure is the existing invalid_request/preflight with zero author/business-state reads.
2. under the current authenticated principal, establish the interface's minimum state disclosure, original ObservationScope/2, and prior CommitDomain qualification. Failure is uniformly not_visible. Hidden author facts, result members, or old-ledger business contents are never read first to choose scope.
3. prove CommitDomain, expectedDomainFenceToken, portable trust/backend, and P-ledger custody/continuity. Unproved is domain_unavailable; proved corruption uses integrity_conflict. Business DecisionKey state is read only after this common continuity gate succeeds.
4. locate DecisionKey by `(workspaceId,D3-CJ/3(commitDomain),operationId)` and compare protocolOwner plus original canonical request/fingerprint. Different owner or different request at the same key is operation_id_conflict; the complete original request prevents ledger probing. Then branch on existing state:
   - saved decision, including committed and any recorded/terminal outcome durably saved by its original protocol: under current disclosure/delivery authorization for the **original actual effect/mode or original result-disclosure scope**, return the original sealed receipt/error bytes or resume its original publication/outbox. Do not require the old before SourceObservation to remain current, old Frontier to equal current Frontier, preview/plan TTL to remain valid, or current r6 business dependencies to become true again, and do not re-run business selection. Current revocation may hide delivery but never edits decision, receipt bytes, original ChangeId, charge, or historical recovery responsibility.
   - planned: restore the same original InputDescriptor, applicable SourceRevisionPlan, before/after pins, write set, OperationId, budget/attempt counters, WriteProtection, InstallationNotice, and installation state. Never new-prepare a second success, reselect target/Query/current page, resample identity/H/revision, or edit the original request. Actual current authorization, original dependency continuity, and installation provenance decide only whether that original plan may continue or remains paused/conflict/recovery_unknown. An unsealed planned decision has no ChangeId for this decision.
   - unseen: only this branch continues to step 5 to create a new decision under current producer/consumer contracts.
5. unseen validates fence, planToken tag/audience, PreparedIntent/2, inputRetentionState, owner version, and its fixed InputDescriptor/DependencyProof/ObservationProof/pins. A strong consumer missing its coordinated owner afterimage returns owner_update_required/proof_unavailable; an otherwise qualified ordinary file path that does not depend on that strong range is not permanently disabled.
6. before entering planning, unseen revalidates under frontierPolicy the complete current Frontier/2, SourceObservation/1, SourceRevisionPlan version basis, DependencyProof/2, MutationFootprint authorization, applicable local/complete semantic gates, budget, and every unwritten dependency. exact uses §1.4 complete equality. scope_dependencies admits only the proved unrelated continuous non-regressing extension of §1.4/§6.3. semantic_pending means only that local typed facts passed while an obligation explicitly permitted to remain pending lacks cross-object/complete proof; it never authorizes all_result, bulk, strong Action, Automation, or another success that requires complete proof.
7. For a human ordinary whole-source save, observed_only is explicitly selected and frozen by a trusted `interactive_source_save` human **before planning starts**. All eligibility still holds: exactly one existing live Document; ordinary+replica_local; complete source read/replace; author-source write set empty or limited to that Document; no applicable body/Field/node-control deny; no identity, parent/order, lifecycle, shared-policy, Registry, Calendar-scope, or other-entity mutation; Draft Base equals the selected current SourceObservation. noninteractive, D3 identity/parent/order/lifecycle, D5 structured cell/row/column/reorder, bulk/collection/promotion, D7 strong Action, Automation, server checkpoint, Approval, and Money are always strict. Once planning starts, strict-capability failure, known conflict, revocation, durability failure, strong-obligation failure, or any missing eligibility never falls back to observed_only.
8. the planning CAS atomically stores canonical request, fixed plan, InputDescriptor, DependencyProof, applicable SourceRevisionPlan, write set, exact before/after pins, budget/reservation, recovery description, required WriteProtection, version basis, and planned. Nothing is resampled after the plan wins. Only proposed SourceStamp is frozen here: **no ChangeId is allocated, no SourceStamp/RevisionToken is recorded as sealed SourceVersion, and no fresh D3 Ref is exposed**.
9. before modifying any portable-current component, durably write InstallationNotice/2 with original DecisionKey, guarantee, WriteProtection, notice.baseFrontier, and complete component before/after. baseFrontier may contain historical ChangeIds that already existed; the notice contains no new ChangeId for this not-yet-sealed decision and no receipt, Approval/Money, or external payload.
10. after staging/pins are durable, install under the planned capability. strict uses a real strict FileInstallCapability. observed_only is only the step-7 path and performs the final trusted object/event-continuity check immediately before destructive installation. Its original plan durably retains actual read-before B and user input N. An unseen external C after the final check may be overwritten by N and may have no recoverable copy, and a later C may replace current file again, but durable B/N is not discarded. Any observed competition, stale Base, watcher gap, revocation, or third state is outside the weak relaxation and enters the original conflict/paused/reprepare path; unknown installation provenance is recovery_unknown.
11. verify each written component against the **original planned after** with complete FileObjectBinding/bytes and continuous installation provenance. Core never requires its own written component to remain equal to before after installation. Every unwritten dependency is revalidated against its original before/cut expectation, including source/control/auth, Registry/rules, DependencyProof positive/negative ranges, and applicable Frontier policy. scope_dependencies admits only an unrelated extension proved and retained in P. Unknown provenance, third_state, late competition, revocation, or unproved continuity remains paused/conflict/recovery_unknown. There is still no ChangeId for this decision.
12. seal is the only decision commit point. After written=planned after and original plan/deps/auth, domain fence, SourceRevisionPlan lastIssued/empty-history basis, and applicable installation proof all pass, one durable P transaction checked-allocates the single ChangeId for **this portable decision**. Only an actual source change whose after is managed combines its original SourceRevisionPlan.after SourceStamp with that ChangeId into the unique managed SourceVersion/2 and atomically advances H(D,E) for that production-domain entity. Source deletion with after=absent remains a portable effect using this ChangeId but creates no managed SourceVersion, uses no deleted-after SourceRevisionPlan, and does not advance H. A source-unchanged portable structure/lifecycle effect likewise uses the portable ChangeId but creates no source version/H increment. P-only control_only and true raw no_op create no content ChangeId. The same P transaction performs the existing domainCommitSequence update and writes committed decision, original receipt, effects, ReliableSaveState, applicable charge, and outbox. Charges settle once for the original decision and replay/recovery never charges again. strict->reliable and observed_only->durable_observed_only.
13. new-FA portable publication derives ContentCompletionProof/3 only from the facts sealed in step 12 and advances Frontier/2. proof.frontierBefore/frontierAfter are the actual pre/post-seal Frontiers; notice.baseFrontier and InputDescriptor.expectedFrontier retain the original baseline. Under scope_dependencies, publication also validates the complete continuous ChangeRecord/portable-proof chain from the original base to actual frontierBefore, the unrelated-extension judgment retained in P, and continuing validity of original DependencyProof. Publication never reconstructs “unrelated” merely from larger vector numbers, provider state, or equal final hash. The proof transports actual production SourceVersion before/after rather than sender SourceObservation token; the receiver creates its own current Observation under §6.3. ContentCompletionProof/2, /1, and other historical records remain under their original decoder/bytes/recovery and are never decoded as /3.
14. if seal succeeded but publication/outbox/delivery failed, decision, ChangeId, managed SourceVersion/H results, ReliableSaveState, receipt, and applicable charge are already fixed. Recovery only publishes the original proof/outbox for that sealed decision or delivers original bytes under current authorization. It never reinstalls N, changes OperationId, allocates another ChangeId, advances H again, reruns business selection, writes over a later current source, or charges again. delivery finally rechecks current authorization; revocation may hide delivery but never alters the historical decision.

For observed_only, B is only the actually read/pinned before and never enumerates C unseen after the final check. Later current=C never rewrites the old receipt/Proof and never weakens durable B/N retention. Crash/unknown follows §8 and the client never guesses the outcome.

A true raw no-op remains effectClass=no_op with sourceVersions=[], InstallationState=not_required, ReliableSaveState=not_applicable, and PortablePublicationState=not_applicable. Under the existing contract its domainCommitSequence may advance, but no content ChangeId, source revision, H, or Frontier head does. P-only control uses control_only; a real portable structure/lifecycle/delete/source effect uses portable.

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

### 6.2 SourceStamp/1, SourceRevisionPlan/1, and InstallationNotice/2

The existing SourceStamp/1 closed shape is unchanged:
~~~json
{"kind":"decision_source","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"revision":<Counter>,"observationEpoch":<Counter>}
~~~
It is determinate before installation only as the same original plan's proposed managed-source address. It is not SourceVersion/2, ChangeId, receipt, or second current truth. decisionKey.commitDomain is the proposed managed after's production domain; entityRef belongs to the same Workspace; revision/observationEpoch are 1..MAX. A SourceStamp becomes usable as the version basis of a sealed managed version only after that original plan seals and §6.3 can uniquely match it to the actual managed SourceVersion/2. Without seal there is no managed SourceVersion or ChangeId for this decision.

Whenever a plan **will produce a new managed after**, installationPlan stores one SourceRevisionPlan/1 for that entity. Its closed shape is:
~~~json
{"kind":"d6_source_revision_plan","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<SourceObservation/1|"absent">,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

All cross-field rules are mandatory:

- decisionKey equals the outer original plan DecisionKey; after.decisionKey is byte-equal to it; entityRef=after.entityRef.
- after's production domain is decisionKey.commitDomain. A non-`"none"` lastIssued is the latest continuously sealed managed SourceVersion/2 for the same entityRef and production CommitDomain, with revision exactly H(D,E). A larger revision from another production domain, externalSequence, file hash, I cache, or equal text never substitutes.
- lastIssued=`"none"` is legal only when DependencyProof/2 plus protected production history prove a complete empty managed history for that entityRef in the production CommitDomain. Missing history, unproved P continuity, a portable-history hole, MAX/overflow uncertainty, or other unknown state is not `"none"`.
- after.revision is 1 when lastIssued=`"none"` and otherwise checked(lastIssued.revision+1). observationEpoch changes never reset this sequence within the production domain.
- after.observationEpoch is the production-observation generation frozen by the original plan for the target source in the after production domain. A before from another production domain never donates its sourceVersion.observationEpoch to after. Fresh/expected-absent targets use the same plan's trusted absent FileObjectBinding/target-backend observation generation.
- A SourceObservation/1 before is the complete current Observation actually read by the original plan and has before.entityRef=entityRef. before=`"absent"` is legal only when the original D3 identity/owner plan has proved a truly fresh/expected-absent branch; callers never infer it from path absence.
- afterPin is the plan's exact after bytes/value pin and matches entity kind: Document owner NodeRef→exact_source_document, ResourceRef→resource_bytes, AnnotationRef→annotation_value. The protected pin record binds the same entityRef; digest equality is insufficient.
- For D3 fresh content, entityRef/after comes only from the original stage12 private candidate map and winning reservation and is never exposed as a fresh Ref in prepare/preview. D7 DefinitionTransfer two-pass Q/Locator materialization uses the same candidate map, after SourceStamp, and revision-token binding in both passes; the second pass or commit never resamples revision.
- The planning CAS persists complete SourceRevisionPlan, pins, and H/empty-history basis; these members are immutable after the winning plan. Before seal Core revalidates lastIssued/empty-history, domain fence, after pin, and original dependencies. A newly sealed managed version for the same production-domain entity makes the old plan stale/conflict; after.revision is never edited in place to a newer H+1.
- SourceRevisionPlan allocates no ChangeId. At seal, only an actual source change producing a managed after combines after SourceStamp with the seal-allocated ChangeId into managed SourceVersion/2 and atomically advances H(D,E). True raw no-op, pure structure/lifecycle/control with unchanged source, and source deletion after=absent create no SourceRevisionPlan, managed after, or H increment. Deletion and other real portable structure/lifecycle effects may still receive their normal portable-effect ChangeId at seal; absence of SourceRevisionPlan never reclassifies them as no_op.
- Equal-byte external admission still produces a managed after and therefore has SourceRevisionPlan. Its before.sourceVersion is the complete external SourceVersion/2, while after revision comes only from current production-domain lastIssued/H. externalSequence never enters after.revision.

InstallationNotice/2 keeps its existing closed shape:
~~~json
{"format":"weftext.installation-notice","version":2,"decisionKey":<DecisionKey/2>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"before":<ComponentImage/1>,"after":<ComponentImage/1>}...]}
~~~
components is non-empty, fixed-rank/canonical-key sorted unique. observed_only is §4.1 only; every other portable author plan is strict. The notice is durable before the first portable-current install. baseFrontier is the Frontier frozen for that original plan/notice; it may contain historical ChangeIds already in existence, but the notice contains **no ChangeId for this not-yet-sealed decision**, and no receipt, approval, Money, external payload, or credential. SourceRevisionPlan remains protected P/plan state and never turns InstallationNotice into a second public version table.
### 6.3 ContentCompletionProof/3 and historical /2

New FA portable decisions use ContentCompletionProof/3. The committed closed shape is:
~~~json
{"format":"weftext.content-completion","version":3,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersion/2|"absent">,"after":<managed SourceVersion/2|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~

/3 retains every other /2 member and responsibility boundary; only portable sourceChanges move from local current-observation handles to production SourceVersion. All members additionally satisfy:

- decisionKey.workspaceRef matches every component/sourceChanges Workspace and changeId.commitDomain=decisionKey.commitDomain. receiptDigest verifies original receipt bytes for the same decision only and grants no receipt read, approval/Money consumption, or execution takeover.
- components is exactly the corresponding InstallationNotice/2 key set in the original fixed-rank/canonical-key order and every after is the actually installed/sealed after. The proof never adds an undeclared component and digest equality never replaces validation of component bytes/owner version.
- sourceChanges may be empty; otherwise it is complete EntityRef-canonical sorted/unique and exactly covers source-state changes made by this decision. A source-unchanged structure/lifecycle portable effect never emits a fake sourceChanges item.
- A non-`"absent"` before is the complete production SourceVersion/2 inside the original plan's before SourceObservation/1. It may be managed or external and its production commitDomain may differ from this decision domain. before=`"absent"` is only the original plan's proved fresh branch.
- A non-`"absent"` after is managed SourceVersion/2 with entityRef equal to sourceChanges.entityRef, commitDomain=decisionKey.commitDomain, changeId byte-equal to proof.changeId, revision/observationEpoch byte-equal to that entity's SourceRevisionPlan/1.after SourceStamp, and predecessor production history equal to the validated lastIssued/empty-history basis. One SourceRevisionPlan produces exactly one such sealed after.
- External→managed admission, including equal-byte admission, uses external before+managed after and never carries externalSequence into after. Source deletion uses production SourceVersion before+after=`"absent"` and has no SourceRevisionPlan after, managed SourceVersion, or H increment, while still being the actual source deletion of this portable decision identified by proof.changeId. Raw no-op creates no /3 portable source change.
- observed_only proves only durable installation/seal of original read-before B and input N and never proves absence of an unseen C after the final observation. The proof never fabricates C or rewrites original after from later current bytes.

frontierBefore is the actually validated Frontier immediately before seal. frontierAfter is exactly frontierBefore plus proof.changeId with no regression of any other domain head. InstallationNotice/2.baseFrontier and InputDescriptor.expectedFrontier retain the original plan baseline and are never rewritten by the proof.

With frontierPolicy=`exact`, frontierBefore is byte-equal to original expectedFrontier, DependencyProof.baseFrontier, and notice.baseFrontier.

With frontierPolicy=`scope_dependencies`, frontierBefore may be a non-regressing extension of notice.baseFrontier, but producer seal and /3 generation require all of:
1. every head added from notice.baseFrontier to frontierBefore has a complete, continuous, verified portable ChangeRecord/corresponding completion-proof chain with no hole, fabricated head, or sequence-number jump;
2. every source/control/authorization/positive-negative range entry of the original frozen DependencyProof/2 remains valid under the same original plan and proves the added sealed effects unrelated to those bound keys; a real SourceObservation, FileObjectBinding/pin, Registry/rule, authorization, membership/negative-range, or other dependency change still invalidates the plan;
3. validation of the unrelated extension is durably retained with the original plan, original notice, and actual seal Frontier in P recovery state. Publication never reconstructs “unrelated” after P loss by comparing two Frontier numbers.

A receiver admitting /3 validates portable trust, decision/changeId relation, notice/proof/components, production SourceVersions, and the complete continuous sealed-record chain from notice.baseFrontier through frontierBefore to frontierAfter. Larger vector sequence numbers alone never substitute for missing intermediate causal records. /3 is not a portable copy of private DependencyProof and grants the receiver no new complete Query/Action proof. A strong consumer builds a new current SourceObservation and complete local DependencyProof in its own observerDomain. The receiver never copies sender SourceVersionRef/sourceToken; after verifying production history it signs a new SourceObservation/SourceVersionRef from its own CommitDomain, current FileObjectBinding, observationEpoch, evidencePins, and current control/Registry/incidence cut.

When every component has been safely restored to before without seal, the /3 restored closed shape is:
~~~json
{"format":"weftext.content-completion","version":3,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...]}
~~~
restored forbids changeId, guarantee/writeProtection/semanticState, frontierBefore/frontierAfter, sourceChanges, receiptDigest, or any success semantics. components must prove complete safe restoration to original before. Never generate it when restoration is unproved.

ContentCompletionProof/2 retains its historical closed definition, original decoder, and original bytes. Historical committed remains:
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersionRef/1|"absent">,"after":<SourceVersionRef/1|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~
Historical restored remains:
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/1>,"after":<ComponentImage/1>}...]}
~~~
/2 replays under original saved bytes, token bindings, admission and recovery gates. /3 proof never upgrades, re-encodes, or backfills an old r5, and /2 SourceVersionRef never implies a portable production-version field that was not present.
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
The D3DecisionCompanion/2 closed shape, version=2, and existing member set remain unchanged. It gains no ChangeId, SourceVersion, WriteScope, SourceRevisionPlan, or other member and is not a second receipt. For a new protocolOwner=D3 decision using the current producer contract, the D3 primary receipt and this companion are written atomically in the same P seal transaction; they refer to the same DecisionKey, the same post-seal domainCommitSequence, and the same effectsToken. The companion stores only the established D6-side association and creates neither another ledger/decision truth nor additional write authority.

Fixed C already contains a D3 wire12 candidate, but that does not mean the new P1 producer rules are consumed or independently accepted by the D3 native-descriptor/companion consumer. An affected new protocolOwner=D3 path remains owner_update_required until the actual P2 D3 afterimage is complete. D3 must genuinely consume the d3_identity_operation/12 OwnerInputBinding, the current complete SourceObservation/1 and DependencyProof/2, and, when a managed after will be produced, the same original plan's SourceRevisionPlan/1 and d6_source_revision/2 binding. The sealed production SourceVersion/2, SourceVersionRef/1, ContentCompletionProof/3, and recovery split must also agree with this Control file's sole producer definitions. D6 never copies those objects into the companion to bypass D3 ownership and never changes D3 wire12, Locator revision-token lexical ownership, the D4 inner-selector wire, or D3's actual write scope.

Historical D3 primary receipts, companions, and saved/planned/unknown decisions continue under their original versions, bytes, decoders, pin/authorization/continuity, and recovery obligations. New P1 types never upgrade, re-encode, or grant them a new strong qualification. Managed success depending on these new producers is not partially activated before the P2 D3 consumer, later D7/D10 consumers, and complete fresh joint acceptance are finished. Conversely, an approved ordinary/local operation that does not depend on the missing strong consumer continues under its original owner qualification and is not permanently disabled merely because this companion coordination is incomplete.
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

Each planned v2 decision continues to store the per-component internal InstallComponentState:

pending|staged_durable|installed_after|restored_before|third_state|unavailable.

Recovery always **reads the original DecisionKey state and original-version record in P first**, then the InstallationNotice, write-set bindings/pins, and actual components referenced by that record. Client timeout, file presence, mtime, provider state, I cache, or digest never substitutes for P and never decides whether the decision committed.

An actual file/component is classified only as:
- exact_before: the complete trusted FileObjectBinding/control binding plus bytes/value is item-for-item equal to original planned before and its read continuity is proved;
- exact_after: the complete trusted binding plus bytes/value is item-for-item equal to original planned after and installation provenance is proved to belong to this original plan;
- third_state: any other materialized bytes/object, observed competing state, unexpected absence/presence, or state differing from the original plan;
- unavailable: backend/object/clock/domain/installation continuity cannot currently be read or proved, or required evidence is unavailable.

Equal digest, equal text, same path, same bare revision, or rebuilt I never substitutes for binding/provenance and never upgrades third_state/unavailable to exact_after. Proven P/portable-record corruption continues through the original integrity_conflict gate. Mere inability to prove something remains an availability/recovery condition and is never relabeled proved corruption.

A written exact_after component is still compared with original planned after while unwritten dependencies remain compared with original before/cut expectations. Core's own installation does not self-conflict merely because the before SourceObservation changed. A scope_dependencies Frontier extension is consumed only with the evidence required by §1.4/§5 and retained in the original P record; recovery never re-signs SourceObservation, source token, PinRef, or DependencyProof stamp merely to make an old plan pass.

### 8.1 Planned, committed, and historical recovery branches

An unsealed planned decision has **no ChangeId for this decision**. Planned recovery reuses original canonical request/fingerprint, InputDescriptor, applicable SourceRevisionPlan/SourceStamp, before/after pins, write set/reservation, OperationId, budget/attempt counters, WriteProtection, InstallationNotice, installation state, and original owner-version basis. It never resamples current before, identity, H, revision token, target/Query/current page, and never new-prepares a second decision in place of the original. Where SourceRevisionPlan exists, after revision remains the winning plan's frozen value and continuation toward seal must reprove that original lastIssued/empty-history basis has not been invalidated by a new managed version in the same production domain.

- planned/not installed: continue the same original plan only when original domain/fence/P continuity and current authorization permit it. Temporarily unproved qualification leaves the decision planned in its applicable paused state rather than converting availability loss into business rejection.
- installing: continue the original plan only while every actual component is classified as exact_before/exact_after under continuous installation lineage. A third_state, observed competition, or late external replacement preserves current bytes, B/N, pins, and version basis and enters conflict/paused. Unknown installation outcome or ownership is fixed recovery_unknown.
- all written components appear to be after but seal is unknown: **read P first**. If P remains planned, there is only the original plan and proposed SourceStamp; files/hash/SourceStamp never invent ChangeId, managed SourceVersion, or success. Only P=committed uses its stored real ChangeId, SourceVersions, receipt, and effects.
- if original pins, clock epoch, CommitDomain/fence, or P continuity cannot be proved, neither Derived Index, current files, equal hash, nor an empty control DB on a new device reconstructs them. Keep the original planned decision paused/recovery_unknown or use the original integrity gate when corruption is actually proved. Protected last-reference evidence retains its original retention responsibility.

A committed decision is different from planned. Once seal succeeded, original ChangeId, actual managed SourceVersion/H results, domainCommitSequence, receipt, effects, charge, and decision bytes are fixed:
- after a lost response, return original receipt bytes only after current disclosure/delivery authorization for the **original saved actual-effect scope**. Old before SourceObservation need not remain current, old Frontier need not equal r6 current, old business dependencies need not pass again, and preview need not remain valid. Current revocation may make delivery not_visible but never revokes or rewrites the saved decision.
- when new-FA portablePublicationState is pending, publish only the same ContentCompletionProof/3/outbox from the original sealed decision, original notice/components, and original retained scope_dependencies-extension evidence. Never reinstall N, change OperationId, allocate ChangeId again, advance H, recalculate source version, or charge again. Historical /1 or /2 decisions generate/recover their **original-version** proof with original decoder/bytes and never auto-upgrade to /3.
- publication/outbox/Derived Index failure resumes only that owner's derived/transport work; it never rolls back the author decision and never reconstructs P approval/Money/unknown responsibility from I.
- a current r6 SourceObservation, DependencyProof, or complete proof proves r6 only. It never retrospectively upgrades, rejects, or re-encodes an historical r5 receipt/saved bytes. Undo/restore remains a new explicit plan rather than receipt replay rolling current backward.

The saved/planned/unseen business split in §5 is the only submit order. §8 only recovers an existing record and provides no second commit entry point. Legacy D6/D3/D7/D8 planned/saved/unknown records continue under their original versions, pins, authorization/continuity, and decoders. New SourceRevisionPlan, ContentCompletionProof/3, or other P1 types apply only to explicit new-version paths and never mechanically migrate historical records.

### 8.2 observed_only recovery boundary

observed_only read-before B is only the before actually read and durably pinned by the original plan, and N is that same plan's durably retained user input. An external C never observed after the final check may be overwritten when N is installed and may have no recoverable copy; no record invents it. A later C may also replace current file again, but durable B/N is not discarded.

Any observed competition, stale Base, watcher gap, or third_state is outside the approved weak race and follows conflict/reprepare/paused. Unknown install is recovery_unknown and equal hash/text never guesses success. A failed strict plan never becomes observed_only during recovery. Once an observed_only decision is committed, recovery only completes its original receipt/proof/outbox and retains B/N; it never installs N again or rewrites a later current file back to the original after.

## 9. ConflictKey/1, ConflictRecord/1-/2, and resolution prepare

### 9.1 ConflictSubject and kind

ConflictSubject remains closed:

- {"kind":"workspace","workspaceRef":WorkspaceRef}
- {"kind":"entity","ref":EntityRef}

Conflict kind remains source_concurrent | placement_concurrent | lifecycle_concurrent | identity_collision | policy_concurrent | incomplete_transport | placeholder.

ConflictSubject, conflict kind, and ConflictKey/1 do not change merely because ConflictRecord advances to /2. Subjects and heads come from real managed portable history. An external race, unknown install, or third-state bytes with no sealed ChangeId never gets a fabricated head merely to fit ConflictKey; it remains installation/recovery/conflict evidence until represented by a real sealed branch.

### 9.2 ConflictKey/1, ConflictId, and ConflictRecord/1-/2

ConflictKey/1 remains exactly:
~~~json
{"workspaceRef":<WorkspaceRef>,"kind":<conflict-kind>,"subjects":[<ConflictSubject>...],"heads":[<ChangeId>...]}
~~~

subjects and heads are non-empty and unique. Subjects sort workspace rank0 then entity rank1+canonical Ref; heads sort by ChangeId. Every member belongs to the same Workspace. source/placement/lifecycle/identity includes at least one entity subject; policy includes a workspace subject. incomplete/placeholder lists the actually affected workspace/entity subjects. Every head is a real verified-continuous sealed ChangeId in that Workspace and its portable change history proves relevance to the ConflictKey subjects/kind. Sequence numbers, hash, mtime, path, or an unsealed external state alone never create a head.

ConflictId remains ASCII d6c: plus 64 lowercase hex where hex = SHA-256(ASCII "D6-ConflictKey/1" + NUL + D3-CJ/3(ConflictKey)). ConflictId is only the stable address of the complete key and validation loads and byte-compares that complete key. ConflictRecord version is not part of ConflictKey/hash input, so this change does **not** version ConflictKey/1, ConflictId format, or the `D6-ConflictKey/1` hash domain.

New FA current records use ConflictRecord/2:
~~~json
{"format":"weftext.conflict","version":2,"conflictId":<ConflictId>,"key":<ConflictKey/1>,"state":"open|resolution_prepared|resolved|superseded","createdAtFrontier":<Frontier/2>,"supersedes":[<ConflictId>...]}
~~~
createdAtFrontier is the verified-continuous Frontier/2 at record creation and covers every key.head: for each head CommitDomain, its corresponding frontier head sequence is at least that head and the intervening sealed records are continuously verifiable. It is not complete Query or source-materialization proof. supersedes may be empty, is ASCII-sorted/unique, and never contains the record's own ConflictId.

Historical ConflictRecord/1 closed bytes remain:
~~~json
{"format":"weftext.conflict","version":1,"conflictId":<ConflictId>,"key":<ConflictKey/1>,"state":"open|resolution_prepared|resolved|superseded","createdAtFrontier":<Frontier/1>,"supersedes":[<ConflictId>...]}
~~~
/1 continues under its original Frontier/1 decoder, bytes, authorization, and recovery gates. A version=1 createdAtFrontier is never interpreted as Frontier/2 and resolved/superseded history is never re-encoded merely for migration. An existing /1 open or resolution_prepared record that remains actionable on its historical path continues under that original decoder/gate and is not rewritten in place because /2 exists. When a new head changes the complete ConflictKey, the new current successor uses /2 and may reference the old ConflictId in supersedes while the old record remains unchanged. Because ConflictId is key-derived, Core never creates simultaneous /1 and /2 "current records" for the same unchanged ConflictKey merely to simulate a version upgrade.

State remains open | resolution_prepared | resolved | superseded. A newly sealed real head can move an old open/resolution_prepared record to superseded only under that record version's rules and creates a successor for the new complete key. Resolved history never reopens. supersedes is history only and grants no merge, LWW, write, or resolution qualification.

### 9.3 conflict read

The request shape remains:
~~~json
{"wireVersion":2,"kind":"d6_conflict_read","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>}
~~~

Non-disclosure ordering remains: closed decode, then conflict_read capability plus minimum state disclosure for every subject, then record access. Unauthorized, unknown address, or inability to establish subject disclosure is uniformly not_visible. After qualification, Core loads the full key addressed by ConflictId and byte-compares it, then chooses the decoder from record.version: version=2 uses ConflictRecord/2+Frontier/2 and version=1 uses only historical ConflictRecord/1+Frontier/1. An unknown record version returns existing state_unavailable after disclosure; no decoder is guessed from fields and there is no fallback decoder. Reading either record version grants no source bytes, branch choice, conflict_resolve, or author write.

The current /2 read path also validates the continuous sealed relationship between key heads and createdAtFrontier. A portable-history hole, fabricated head, or corrupt record is integrity/state unavailability and never causes Core to delete missing heads and return a smaller conflict. /1 stays under its original historical decoder/gate; new /2 Frontier conditions never retrospectively reinterpret its saved bytes.
### 9.4 conflict resolution prepare

~~~json
{"wireVersion":2,"kind":"d6_conflict_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/1>,"budget":<BudgetBinding>}
~~~

ConflictResolution/1 is:

- source_merge: {"kind":"source_merge","ownerNodeRef":NodeRef,"source":text}
- choose_source_head: {"kind":"choose_source_head","ownerNodeRef":NodeRef,"head":ChangeId}
- policy_choice: {"kind":"policy_choice","policy":<Policy/3>}
- owner_resolution: {"kind":"owner_resolution","owner":"D3","action":"placement|lifecycle|identity_fresh_copy"}

owner_resolution has no free payload and only transfers control to the real D3 owner. Fixed C already contains the D3 wire12 candidate, but the P2 D3 native-descriptor/companion production rules and consumer coordination, plus later fresh joint acceptance, are not complete; until those gates complete, the affected prepare returns owner_update_required and creates no planToken. The later D3 adapter must freeze the concrete typed D3 request/preview, and D6 never smuggles arbitrary JSON through this member.

source_merge/choose_source_head reload every head/base/current permission and rerun D2/local/complete gates. policy_choice requires current policy_admin and never performs allow-union fallback. Success creates an ordinary PreparedIntent/2 and final commit uses d6_commit_request/2. Any new head makes expectedKey stale -> conflict_changed; an old click is never reused.

## 10. Policy/3

Policy/3 retains the exact top-level members version,revision,grants with version=3. Policy/1/2 retain their original decoders, bytes, capability/scope semantics, and never auto-upgrade. grants remain subject,effect,scope,capabilities with deny-before-allow and default deny. Every Policy/2 capability remains byte-for-byte available under its original rules; DependencyProof expansion changes none of them.

The Policy/3 no-argument capability extension is closed to:

replica_register | replica_retire | conflict_read | conflict_resolve | execution_custody_admin | structure_state | portable_frontier_state.

Semantics remain:
- structure_state permits observation of portable parent/order, live/Trash structural scope, and structural state required by D3 StructureRange proof. It grants no source, Field, decision, lifecycle write, or author-body read.
- portable_frontier_state permits observation of complete Frontier/2 and its verified-continuous heads only. It proves no payload materialization, Query/Registry completeness, source bytes, decision detail, or execution responsibility.
- replica_register/retire manages portable replica registry only under the original workspace-scope administrative semantics plus current trust/bootstrap; it grants no source read/write or execution takeover.
- conflict_read reveals only an authorized ConflictRecord after subject disclosure and never conflict source bytes, resolution, or author write.
- conflict_resolve enters owner-specific resolution prepare only; actual source/policy/D3 writes still need their original permissions.
- execution_custody_admin manages execution-responsibility continuity/takeover only and never expands Money, approvals, claims, or author source write.

These capabilities create no new implication and do not alter the original capability matrix. source_write deny still blocks full-source changes; Field/body deny still constrains corresponding footprints; write never implies read. An ordinary source-save profile still passes the original source/body/Field/node-control matrix, its actual ObservationScope, and every DependencyProof required by that intent. structure_state, portable_frontier_state, conflict, and replica capabilities are never bypasses for content authorization or strong completeness.

Applicable structure_state disclosure succeeds before hidden parent/sibling/Trash members are read; Core never scans a StructureRange and then decides whether the capability was needed. portable_frontier_state may expose complete Frontier/2 but knowledge of Frontier never implies source access, DependencyProof completeness, or D7 complete Query qualification.

In Policy/3, commit_sequence_state remains CommitDomain-scoped metadata read. The request names the complete CommitDomain and exposes only that domainCommitSequence; no global order is invented across offline replicas. Historical Policy/2 consumers retain their original workspace-wide definition on legacy saved/contract paths only.

The authorization DependencyKey/2 principalAudienceToken still comes from trusted principal/session/delegation mapping and binds current Policy/3 version/auth generation plus the original ObservationScope in proof. It never publishes the full grant table or hidden deny members. A Policy change invalidates/advances the related authorization stamp under §3.4, but a Policy revision is not itself a permission token.

observed_only is not a Policy capability. It is selected explicitly by a trusted human only under every §4.1 qualification before planning starts and then freezes. Nothing in this section expands weak-mode eligibility or allows strict, structured, bulk/collection, Action, Automation, server checkpoint, Approval, or Money paths to downgrade to weak protection.
## 11. BudgetBinding/2 and pin capacity

BudgetBinding/1 members/numeric domain remain. PreparedIntent/2 also binds PinBudget/1:

~~~json
{"version":1,"maxRecoveryBytes":<Counter>,"maxConflictBytes":<Counter>,"maxPreviewBytes":<Counter>,"maxImportExportBytes":<Counter>,"maxHistoryBytes":<Counter>}
~~~

0 means no new allocation of that class. Effective limits are request/policy/host minima. Pin allocation reserves with checked add before allocation. All attempts share plan counters. Work units remain durably charged before execution and crash does not refund. Temporary staging and protected pins are separately accounted.

A protected last-reference pin is never deleted by TTL/preview expiry. Capacity shortage produces budget_exceeded or planned paused_capacity; it never frees planned/unknown/conflict last-reference evidence.

## 12. Result/ByteHandle and index consumption

The existing closed wire1 ResultHandle/ResultCursor and Resource ByteHandle/ByteRead shapes, token/tag semantics, saved-handle bytes, authorization order, budgets, TTL, pins, and error priority remain under their original contracts. This section does not change those wire1 shapes and does not treat the mere existence of a historical decoder as evidence that every old prototype is an active current compatibility surface. A saved handle that still satisfies its original authorization, pin, clock, and continuity obligations continues under its original bytes. A legitimately expired, invalidated, or collected record is never revived from current files, equal digest, or a new producer proof.

In the new file-backed producer path obtained after assembling the other P1 Control author patches, any future consumer version that treats a result or snapshot as current, complete, or Action-eligible binds its protected immutable pin/record to the complete SourceObservation/1 actually read, including the complete production SourceVersion/2, production CommitDomain, and current observerDomain/observationEpoch/FileObjectBinding/evidence pins. Equal bare revision numbers in different production domains never identify the same current source. A result that depends on range/control completeness also binds the actual closed DependencyKey/2 entries, their DependencyProof/2 epoch/revision and evidence pins, and the positive/negative ranges, authorization, Registry/control, and other real dependencies proved by their owners. SourceVersionRef/1 only selects the complete current Observation. SourceRevisionPlan/1, a proposed SourceStamp/1, or an unsealed revision token never substitutes for committed current-source evidence.

Frontier/2 is only the verified continuous sealed causal prefix for each CommitDomain. It proves neither complete Query scope, Registry completeness, payload download, nor placeholder materialization. A building/partial index, index miss, placeholder, unknown decoder, physical/D2 invalid state, I/O failure, or unproved range continuity never becomes complete empty-success. Candidate indexes for exact/NFC/regex prove no false negatives for the declared range; otherwise the owner rereads and supplements from real source, and complete scanning still passes current authorization and complete-range proof. Derived Index caches candidates and never signs completeness by itself.

`scope_dependencies` creates no generic result/handle survival exception. Continuation is allowed only where the owning consumer contract explicitly permits it, the extension from original expected/base Frontier to the current cut has complete continuous sealed evidence, and every original source/control/authorization/positive-negative DependencyProof plus pin/selector/Registry dependency remains unchanged. A changed SourceObservation/token, authorization generation, query selector, DependencyKey stamp, pin, Registry/rule, membership/negative range, control fact, or observation continuity still resets/reprepares under its owner contract. Unknown unrelatedness never continues. D7 complete results retain D7's whole-result current-authorization generation and complete-dependency reset rules; D6 never overrides the D7 owner with a coarse-Frontier “unrelated” judgment.

Resource ByteHandle retains its original, narrower immutable-snapshot exception and that exception is not generalized to ResultHandle or current-source evidence. At issuance, the handle freezes the committed Resource, complete ResourceRef/owner, original SourceVersion and resourceRevisionToken, immutable cut, descriptor, pin, finite expiry/clock, lifecycle/continuity delivery epoch, and budget. A Resource update from R1 to R2 does not itself rebind or reset an otherwise qualified previously issued H; H continues reading pinned R1 and returns the R1 token, without making R1 current again. Every read still re-runs the original current-principal D3 disclosure and complete resource_read/owner-scope authorization in its specified order. A permission or holder change does not by itself invalidate the snapshot: a lawful holder/authority continuity takeover may preserve the same H only when the complete original handle record, clock/continuity state, and pin remain continuously proved. If original record, clock, or pin continuity cannot be proved, reading stops and follows the original wire1 byte_unavailable result. That continuity-takeover exception does not survive a ResourceRef/owner lifecycle change or a proved authority/continuity-epoch break for the old snapshot; those conditions still invalidate the delivery epoch or require reset. Expiry, reset, range, unknown clock/pin/backend continuity, budget, and the final delivery gate retain the original wire1 check and error priority. Equal bytes/digest, Trash-then-restore, reauthorization, or a same-name new Resource never revives an invalidated or expired handle. This exception protects only an already-fixed, pinned historical Resource snapshot and never excuses a D7 Result authorization, selector, dependency, complete cut, or whole-result authorization-generation reset, and it is not generalized to Action or arbitrary source/proof changes.

The complete D7 Query/Result language and consumer contract remain jointly owned by the full D7 owner set: Query Algebra, Value/CEL, View, Narrow Field Qualification, Definition Transfer, Preview/Effects, Execution/Action, Prepared Action Binding, Scenario Dispositions, Terminology Lexicon/Registry, and Implementation Impact/Test Outline all require actual coordinated afterimages. D6 provides only the SourceObservation, DependencyProof, pins, authorization, and transport producers closed in this file. It never replaces those owners through free JSON, an implicit new wire, or a Prepared-only rewrite. Until the actual D7 consumer exists, the corresponding new complete Result/Action path remains owner_update_required/proof_unavailable rather than acquiring an invented reset member or handle version here.

A path requiring a complete Query cut, all_result, post-query, bulk/collection, strong Action, or Automation carries the complete current positive/negative evidence required by its D7 owner and never lowers that gate through partial exploration, semantic_pending, local ordinary success, or a ByteHandle snapshot. The converse is also fixed: an ordinary authorized read/edit of current source/Resource or an explicitly local projection is not unconditionally blocked merely because an unrelated complete Action/whole-Workspace proof is unavailable. It continues under its own SourceObservation, local DependencyProof, authorization, and original handle contract.
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

- invalid_request/unsupported_version/not_visible/domain_unavailable/integrity_conflict/operation_id_conflict/plan_expired/source_unavailable/proof_unavailable and install_unavailable known before planning are preflight only;
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

Historical contracts are dispatched by the version and owner under which the record was actually saved, never guessed from current field names or bulk-migrated. D6 wire1 commit/receipt/error, Policy/1/2, SourceVersion/1, Frontier/1, InstallationNotice/1, ContentCompletionProof/1, and the ContentCompletionProof/2 and ConflictRecord/1 definitions explicitly retained by this candidate continue under their original bytes, fingerprint, authorization, continuity, pin-retention, and recovery rules. The same applies to legacy Token tags, old PreparedIntent, the d6d/d6r/d6a revision-token profiles and original opaque DocumentRevision/ResourceRevision/AnnotationRevision decoders, the original D3 primary receipt and its D6 companion bytes, D7 PreparedActionBinding/1,/2, and D8 PreparedEditBinding/1. Any SourceObservation/1, SourceVersionRef/1, or protected token binding that was actually saved and referenced by an older decision or pin is likewise decoded only under the contract that produced it; a new current observation or token never re-signs the old binding. The presence of a decoder or candidate prose does not by itself prove that a historical prototype was deployed or that every such surface is active. Conversely, once a saved, planned, or unknown record actually exists under its original contract, its promised recovery, retention, and no-duplicate-effect obligations are not cancelled merely because deployment evidence for other prototypes is absent.

The current new producer path applies only to records explicitly using this candidate's new contract. SourceVersion/2 remains the complete managed/external production-version union: managed retains production CommitDomain, observationEpoch, revision, and ChangeId; external retains production CommitDomain, observationEpoch, and externalSequence and has no managed revision/ChangeId. Current SourceObservation/1 separately uses the operation CommitDomain as observerDomain, and SourceVersionRef/1.sourceToken selects only that complete current observation. SourceRevisionPlan/1 and SourceStamp/1 freeze only the proposed managed-after version basis, while RevisionTokenBinding/2 with d6_source_revision/2 resolves only through its protected binding; a real ChangeId forms managed SourceVersion/2 only at seal. ContentCompletionProof/3 transports real production SourceVersion before/after and ConflictRecord/2 uses Frontier/2. The original opaque D3 Locator token member, D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence, and the D5 revision-bound locator do not change shape because of these producers.

The following reinterpretations across historical and current versions are forbidden:
- an old request is never re-encoded as wire2 and an old receipt never gains CommitDomain, ChangeId, SourceVersion/2, or new companion members;
- ContentCompletionProof/2 sourceChanges retain their original SourceVersionRef/1|absent shape and are never migrated to ContentCompletionProof/3 production SourceVersion/2; an old sourceToken never implies a production-version field that was not present;
- ConflictRecord/1 createdAtFrontier remains Frontier/1-only and is never re-encoded as ConflictRecord/2; ConflictKey/1, ConflictId, and the `D6-ConflictKey/1` hash domain remain unchanged;
- legacy d6d/d6r/d6a revision-token profiles, original D3 Locator opaque tokens, and saved observation/ref bindings are never retagged as d6_source_revision/2, and a new SourceObservation token, equal bare revision, equal hash/text, or rebuilt I never substitutes for old evidence;
- current semantic_pending, local ordinary success, or a newer complete proof never retrospectively reinterprets an old D4/D7 gate or upgrades an old receipt, old r5, or historical Action qualification;
- new retention, deletion/rebuild of I, reauthorization, or continuing readability of current files never authorizes deletion of a legacy last-reference pin, original request/decision/receipt, charge/approval/claim/unknown responsibility, or another durable fact promised by the old contract.

saved, planned, and unknown remain separate. For an already committed/saved original decision, after original request/fingerprint/continuity lookup succeeds, current delivery authorization is checked for that decision's original actual effect, mode, or result-disclosure scope, and the original sealed receipt/error/effects bytes are replayed or its original-version publication/outbox is resumed. Replay never requires the old source to remain current, the old Frontier to equal the current cut, the old preview to remain valid, or current new business proof to succeed again. Revocation may hide delivery but never edits the decision, reinstalls the original after, charges again, reallocates ChangeId/revision, or revives a legitimately expired handle/token.

A planned record resumes only its original frozen request, InputDescriptor/owner binding, original-version plan, before/after pins, budget/attempt state, WriteProtection, installation state, version basis, and the finite TTL/clock/continuity required by its original contract. An unsealed original plan gains no ChangeId from a current new type and is never replaced by a new prepare, a reselected target/Query, a resampled H/revision, or a new semantic gate as a second decision. Current authorization, original dependency continuity, and installation provenance decide only whether that original plan continues, pauses/conflicts under its original rules, or remains recovery_unknown. A plan that has legitimately expired under its original contract follows its original plan-expiry/recovery rules; the new contract does not revive it.

Historical unknown or outcome-unproved responsibility retains the original decision, pins, installation/external-effect evidence, charge/approval/claim/execution continuity, and applicable stop/recovery obligations. Current files, equal hash, Derived Index, an empty control DB, or reauthorization never guesses success/failure, resends an external effect, charges again, refunds, replenishes approval, or erases unknown. Regaining authorization can only permit original-contract recovery or delivery while the original continuity still exists; it cannot reconstruct lost P facts, pins, clock epoch, or token continuity.

These compatibility obligations do not imply that every prototype with only a decoder or historical draft prose but no deployment evidence is an active product compatibility surface. Implementations honor only records that actually exist and can be proved under their original version. Conversely, a missing new strong producer/consumer never permanently disables approved ordinary `.adoc`/Resource reads, Draft, qualified human whole-source saves, or local offline operations that do not depend on it. Ordinary-content success still does not grant complete Query/Action/Automation qualification.

Managed success depending on the new P1 producers remains behind the coordinated gate. The remaining P1 Lexicon, machine Registry, Impact/Test Outline, and PROPOSAL/replacements routing need actual afterimages; P2 D3/D4, P3 D5, and the actual D7 Query/Value-CEL/View/Narrow Field/Definition Transfer/Preview-Effects/Execution-Action/Prepared/Scenarios/Lexicon-Registry/Impact, D8, D9, and D10 consumers must be fully coordinated. Fresh independent full joint review and coordinated acceptance still follow. Future conformance fixtures are requirements to cover positive, negative, unknown, recovery, and historical-decoder cases; merely naming a fixture in documentation never means it ran, passed, or activated semantics.

The existing eleven OPEN items, U6/U7, the later A2 self-contained reconstruction, and the final separate fresh Pro review remain open gates; the author does not close or accept them here. Documentation checks, diffcheck, CI, or author self-check prove only the checks actually run and never substitute for independent acceptance. This candidate grants no permission for product implementation, source/dependency/CI changes, merge, activation, release, or deployment.
