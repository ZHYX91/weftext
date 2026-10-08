---
source_language: zh-CN
translation_of: D6-CONTROL.zh-CN.md
translation_status: synced
---

[简体中文](D6-CONTROL.zh-CN.md)

# A2 D6 Control Interfaces

Status: A2 D6 current Control author candidate; not independently accepted, implemented, activated, merged, released, or deployed. Historical records always dispatch by the version actually recorded; SourceRevisionPlan /1,/2,/3 roles are not mechanically renumbered.

## 0. Single current dispatch

## 0.1 Current dependency, portable component, and completion successor summary

### 0.1.1 DependencyKey/3

DependencyKey/3 has the fixed rank order: source=0, document_format=1, lifecycle=2, placement_range=3, ref_inbound=4, relation_incidence=5, calendar_scope=6, registry=7, temporal_rules=8, authorization=9, foreign_binding=10, query_scan=11, replica_registry=12, conflict_record=13, execution_resource=14. Historical Key/2 stays fourteen-arm under its original ranks.

```text
0 source
1 document_format
2 lifecycle
3 placement_range
4 ref_inbound
5 relation_incidence
6 calendar_scope
7 registry
8 temporal_rules
9 authorization
10 foreign_binding
11 query_scan
12 replica_registry
13 conflict_record
14 execution_resource
```

DependencyProof/3, InputDescriptor/3, and PreparedIntent/3 retain their prior responsibilities while consuming Key/3.

### 0.1.2 PortableComponentKey/2

PortableComponentKey/2 ranks document=0, document_format=1, resource=2, annotation=3, node_binding=4, child_list=5, lifecycle=6, trash_membership=7, policy=8, registry=9, period_scope=10, replica_registry=11, conflict=12. The document_format component contains exact ManagedDocumentFormatBinding/1 bytes and its ComponentImage version is bindingRevision.

```text
0 document
1 document_format
2 resource
3 annotation
4 node_binding
5 child_list
6 lifecycle
7 trash_membership
8 policy
9 registry
10 period_scope
11 replica_registry
12 conflict
``` PinRef/2 and ComponentImage/1 do not change.

InstallationNotice/3 and ContentCompletionProof/4 are the new-current component family. Historical Notice2/CP3 are not expanded in place.

Notice3 changes only the portable component-key decoder from PortableComponentKey/1 to /2. It otherwise inherits the fixed-parent Notice2 contract exactly: components is non-empty, fixed-rank/canonical-key sorted and unique; the notice is frozen by the original plan and durable before the first portable-current installation; baseFrontier remains the original plan baseline; and the notice contains no not-yet-sealed ChangeId, receipt, credential, approval, Money, or execution authority.

CP4 changes only the component-key decoder to PortableComponentKey/2 and adds the current ChangeRecord/1 linkage below. Every other CP3 committed/restored validation remains normative: exact Notice key-set equality and ordering, actual installed/sealed after images, complete EntityRef-sorted sourceChanges, the original receipt digest, one-step Frontier advancement, scope_dependencies continuous-chain proof, restored-arm exclusions, and receiver validation of real component bytes and owner versions.

For a fresh managed Document, the source-document component and its document_format component are produced by the same original plan, installed by the same portable decision, and become current only through the same P seal/CP4; there is no half-activated managed source. A format-only transition with unchanged source changes only document_format, has sourceChanges=[], creates no SourceRevisionPlan or managed SourceVersion, and does not advance H(D,E).

### 0.1.3 ChangeRecord/1

The first normatively closed ChangeRecord family is:

```text
ChangeRecord/1 = {
  format:"weftext.change-record",version:1,
  decisionKey:DecisionKey/2,changeId:ChangeId/1,
  installationNotice:{format:"weftext.installation-notice",version:3,byteLength:Counter,sha256:"64-lowercase-hex"},
  completionProof:{format:"weftext.content-completion",version:4,byteLength:Counter,sha256:"64-lowercase-hex"},
  frontierBefore:Frontier/2,frontierAfter:Frontier/2
}
```

It is fixed in the same P as the ChangeId and CP4; Notice3 is the pre-install notice frozen by the original plan. ChangeRecord indexes exact Notice/CP/frontiers and never duplicates component or source-change authority. Unknown pre-FC bytes are never relabeled as ChangeRecord/1.

For committed CP4, decisionKey.workspaceRef matches every component/sourceChanges Workspace and changeId.commitDomain equals decisionKey.commitDomain. CP4.components is exactly the corresponding Notice3 key set in identical canonical order, and every CP4 after image is the actual installed and sealed after for that key. CP4.sourceChanges is empty or the complete EntityRef-canonical sorted/unique set of real source-state changes; source-unchanged portable effects do not fabricate entries. Non-absent after values are the managed SourceVersion/2 values produced from the winning source plan and this same ChangeId. receiptDigest authenticates the original receipt bytes for this decision and grants no extra disclosure or execution authority.

frontierBefore is the actual validated Frontier immediately before seal. frontierAfter is exactly frontierBefore plus this ChangeId in decisionKey.commitDomain, with no regression or unrelated rewrite. Under exact policy, frontierBefore equals the original expected/base/notice Frontier. Under scope_dependencies, every added head from Notice3.baseFrontier through frontierBefore and this ChangeId through frontierAfter requires the complete continuous verified ChangeRecord/completion chain and the original frozen dependency proof must still prove every added effect unrelated; vector-number comparison, provider sync state, or current files are insufficient.

Restored CP4 is legal only when every Notice3 component has been safely restored to its original before image without seal. Its closed restored arm contains only decisionKey, baseFrontier, and those component images; changeId, success guarantee/writeProtection/semanticState, frontierBefore/frontierAfter, sourceChanges, receiptDigest, and success semantics are forbidden.

A receiver admits ChangeRecord1 only after strict-decoding the exact canonical Notice3 and CP4 bytes selected by the record digests, validating their versions and cross-fields, obtaining every listed component byte/metadata item, validating each owner decoder/version and ComponentImage against the actual bytes, validating complete production SourceVersion before/after evidence, and proving the continuous causal chain. A document_format component additionally strict-decodes its exact ManagedDocumentFormatBinding/1. Missing bytes, unknown decoder/version, incomplete chain, or Notice/CP mismatch is incomplete/proof_unavailable rather than success. These checks preserve the original authorization, recovery, and error ordering and add no second ledger, CAS, or commit point.

The exact current closed shapes are copied in D6-SCHEMAS. This Control file owns D6 behavior, authorization, ordering, planning/recovery and technical type meaning; D6-SCHEMAS is a schema companion, not a second ledger or semantic authority.

This file owns D6-FA-r01 closed control types, D6 wireVersion2, PreparedIntent/3, Policy/3, SourceVersion/2, ordinary source save, decision/install state, portable installation/completion records, ConflictRecord, and error/authorization ordering. D3 continues to own identity/lifecycle. No generic D6 interface may create, move, Trash, restore, or purge an Entity.

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

A proved unrelated Frontier extension does not re-sign SourceObservation/1, SourceVersionRef/1.sourceToken, revision token, PinRef, or DependencyProof stamp, and never carries an old token into a new observationEpoch. Those objects survive only under their own continuity rules. Conversely, a qualified ordinary operation whose contract uses complete real local evidence is not forced to wait for an unrelated whole-Workspace Query/index proof merely because scope_dependencies is selected. D3 replica_local create/move/reorder/trash retains its owner-defined local_structure + scope_dependencies path. The current A2 D3 wire13 candidate now defines consumption of the current Key3/recovery refinements and its two bounded D3 findings are independently CLOSED; this does not independently accept or activate this D6 repair, whose acceptance boundary is D6 Main §18. Any new branch still missing its required coordinated consumer remains owner_update_required/proof_unavailable at the unseen gate in §5; this paragraph does not replace D3 ownership.

The actual pre-seal Frontier of a portable decision may therefore include a proved unrelated extension under scope_dependencies, while original expectedFrontier and notice.baseFrontier remain unchanged. New-FA ContentCompletionProof/4 records the actual frontierBefore/frontierAfter and cross-validates them under §6.3 against the original notice, continuous change records, and the unrelated-extension evidence retained in P.

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

New-decision source revision tokens are managed by protected `RevisionTokenBinding/2`. Its closed shape is:
~~~json
{"kind":"d6_revision_token_binding","version":2,"token":<Token>,"source":<RevisionTokenSource/2>}
~~~
`RevisionTokenSource/2` has exactly two variants:
~~~json
{"kind":"managed","sourceStamp":<SourceStamp/1>}
~~~
or
~~~json
{"kind":"external","sourceVersion":<external SourceVersion/2>}
~~~
The external sourceVersion must decode as the complete external SourceVersion/2 above. The managed sourceStamp is produced only by §6.2 SourceRevisionPlan/1 or the explicit conflict-only /2 in §6.2.1. `RevisionTokenBinding/2` is a stable production-version address: it carries neither current `observerDomain` nor current `SourceObservation/1.observationEpoch`, stores no source bytes, and grants no read, write, selector, ActionEvidence, Draft, or preparation capability. Production `SourceStamp/1.observationEpoch` remains part of the production address.

`token` uses the §1.1 canonical Token lexical form with protected tag `d6_source_revision/2`. For managed source, the complete SourceStamp identifies the proposed production address; for external source, the complete external SourceVersion/2 identifies the producer event. Neither arm proves current observer qualification.


#### 1.5.1 Authenticated managed revision-seal association

For a managed source only, the portable proof that one exact revision token was selected by the winning seal is the closed `RevisionTokenSealAssociation/1`:
~~~json
{"kind":"d6_revision_token_seal_association","version":1,"decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>,"binding":<RevisionTokenBinding/2>}
~~~
Every member is required. Unknown, duplicate, missing or null members, wrong nested versions/tags, non-managed `sourceVersion`, or a non-managed `binding.source` reject. Cross-field equality is exact: `changeId.commitDomain=decisionKey.commitDomain`; `sourceVersion.changeId=changeId`; `sourceVersion.commitDomain=decisionKey.commitDomain`; `binding.source.sourceStamp.decisionKey=decisionKey`; and sourceVersion entityRef/revision/production observationEpoch are byte-equal to the binding SourceStamp entityRef/revision/observationEpoch. All Workspace bindings must agree. The token is not derived from these fields.

The portable artifact is closed `RevisionTokenSealArtifact/1`:
~~~json
{"format":"weftext.revision-token-seal","version":1,"trustKeyId":"sha256:64-lowercase-hex","association":<RevisionTokenSealAssociation/1>,"signature":"<86-ASCII-unpadded-base64url>"}
~~~
The signature decodes to exactly 64 Ed25519 signature bytes. `trustKeyId` is exactly `"sha256:" + lowercase_hex(SHA-256(raw_32_byte_public_key))`; it selects a key and is not itself authentication. `RevisionTokenSealSignedBody/1` is the same closed object with the `signature` member removed. The authenticated message is exactly the UTF-8 bytes
~~~text
ASCII "D6-Revision-Token-Seal/1" || NUL || D3-CJ/3(RevisionTokenSealSignedBody/1)
~~~
and the complete transported/stored artifact bytes are exactly `D3-CJ/3(RevisionTokenSealArtifact/1)`. A receiver rejects a transport representation whose bytes are not byte-equal to that canonical encoding. The signature never covers itself, a pin digest, CP3 bytes, or a future outbox address, so this profile introduces no self-reference cycle and no third C/Q materialization pass.

The revision-seal trust root is a real D6-owned producer, not a pre-existing validator name.

`WorkspaceTrustRootDeclaration/1` is the closed portable root declaration:
~~~json
{"kind":"d6_workspace_trust_root","version":1,"workspaceRef":<WorkspaceRef>,"establishmentDecisionKey":<DecisionKey/2>,"rootKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","selfSignature":"<86-ASCII-unpadded-base64url>"}
~~~
The public key decodes to exactly 32 Ed25519 bytes and hashes to `rootKeyId`. `establishmentDecisionKey.workspaceRef` and its nested CommitDomain Workspace are byte-equal to `workspaceRef`. The self-signature authenticates exactly `ASCII "D6-Workspace-Trust-Root/1" || NUL || D3-CJ/3(WorkspaceTrustRootDeclaration/1 with selfSignature removed)`; it proves possession only and never self-authorizes copied bytes.

The closed declaration fingerprint is:
~~~json
{"kind":"d6_workspace_trust_root_fingerprint","version":1,"profile":"d6_workspace_trust_root_cj3/1","digest":"sha256:64-lowercase-hex"}
~~~
After strict-decoding the root declaration and verifying the key hash and self-signature above, Core defines `canonicalRootDeclarationBytes=D3-CJ/3(complete WorkspaceTrustRootDeclaration/1 including selfSignature)`. The fingerprint digest is exactly `"sha256:" + lowercase_hex(SHA-256(ASCII "D6-Workspace-Trust-Root-Declaration/1" || NUL || canonicalRootDeclarationBytes))`. No transport spelling, declaration-without-signature bytes, raw public key, or `rootKeyId` is accepted as this digest.

Every authenticating host has one protected, non-portable `WorkspaceTrustAnchor/1`:
~~~json
{"kind":"d6_workspace_trust_anchor","version":1,"workspaceRef":<WorkspaceRef>,"rootFingerprint":<WorkspaceTrustRootFingerprint/1>,"rootKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","establishedBy":<{"kind":"workspace_bootstrap","issuerAuthorityInstanceId":"uuid-v4","proposalId":"uuid-v4"}|{"kind":"explicit_import","anchorImportId":"uuid-v4"}>}
~~~
The anchor repeats the declaration's exact Workspace/key tuple and stores the exact recomputed fingerprint object. A fresh create/fork stages it only through the already-authenticated issuer/target-custody path and makes it usable only when that bootstrap P seal commits.

An existing Workspace is anchored only through the host-local closed import:
~~~json
{"wireVersion":1,"kind":"d6_workspace_trust_anchor_import","workspaceRef":<WorkspaceRef>,"rootDeclaration":<WorkspaceTrustRootDeclaration/1>,"expectedRootFingerprint":<WorkspaceTrustRootFingerprint/1>}
~~~
Trusted local/deployment-operator authentication and explicit out-of-band confirmation of that complete fingerprint object precede the write. Core strict-decodes `rootDeclaration`, requires its Workspace to equal the request Workspace, rechecks `rootKeyId` from the raw public key, verifies `selfSignature`, recomputes `WorkspaceTrustRootFingerprint/1` from the complete canonical declaration bytes above, and requires byte-equality with `expectedRootFingerprint`. An already present byte-equal anchor is exact replay; a different anchor/fingerprint is rejected. Import writes no Workspace author state, P decision, Frontier or policy. Reading a self-signed root from copied/synchronized files, or substituting the key fingerprint for the declaration fingerprint, is insufficient.
The common root declaration, fingerprint and protected anchor above remain retained-current. The following `WorkspaceAuthorizationBundle/1` + `WorkspaceTrustDeclaration/1` + `DomainSealKeyHandle/1` activation subprofile is a **genuine predecessor/historical dispatch** only when those bytes were actually recorded; fresh current dual-profile trust uses the §20 successors (`Bundle2`/`Declaration2`/`Handle2`) and the current Notice3/CP4/ChangeRecord1 path. All predecessor authorization, signature, positive-path, error-order, pin-retention and saved/planned/unknown recovery obligations below remain exact.

For that predecessor profile, the exact bytes of the already-existing `PortableComponentKey/2={"kind":"policy","workspaceRef":...}` are the closed:
~~~json
{"kind":"d6_workspace_authorization_bundle","version":1,"workspaceRef":<WorkspaceRef>,"authorizationRevision":<Counter>,"policy":<Policy/3>,"trustRoot":<WorkspaceTrustRootDeclaration/1>,"trustRevision":<Counter>,"trustDeclarations":[<WorkspaceTrustDeclaration/1>...]}
~~~
`authorizationRevision` starts at 1 and checked-increments for every policy or trust change; `Policy/3.revision` changes only for a policy change. `trustRevision` starts at 1 with the initial domain authorization and checked-increments once per appended declaration. The array is cumulative and exactly revisions 1..trustRevision. Deletion, reordering, duplicate/gapped revision, or mutation of any earlier declaration is integrity failure. One portable decision may append a fixed non-empty consecutive sequence under the same DecisionKey; that entire sequence is one atomic policy-component transition and no prefix ending inside the same activation ChangeId is a valid history cut. That predecessor adds no member to its recorded Notice2/CP3 or PortableComponentKey kind.

Revision 1 has predecessor `{"kind":"root","fingerprint":<WorkspaceTrustRootFingerprint/1>}` and that fingerprint is byte-equal to the anchor/import value computed above; revision n>1 has `{"kind":"declaration","revision":n-1,"sha256":<SHA-256(D3-CJ/3(previous declaration))>}`. `rootKeyId` is never accepted in the predecessor fingerprint slot. `WorkspaceTrustDeclaration/1` is a closed action union. Authorize is:
~~~json
{"kind":"d6_workspace_trust_declaration","version":1,"workspaceRef":<WorkspaceRef>,"revision":<Counter>,"predecessor":<predecessor>,"decisionKey":<DecisionKey/2>,"action":"authorize","commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","trustKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","possessionSignature":"<86-ASCII-unpadded-base64url>","rootSignature":"<86-ASCII-unpadded-base64url>"}
~~~
Rotate has the same prefix, `action:"rotate"`, `replacesTrustKeyId`, `mode:"ordinary|loss_recovery|compromise"`, the new key tuple/possessionSignature, `priorContinuitySignature`, and rootSignature. For ordinary rotation priorContinuitySignature is the replaced key's Ed25519 signature; for loss_recovery/compromise it is the literal `"not_required"`. Revoke has the same prefix plus `action:"revoke"`, commitDomain, profile, trustKeyId, `mode:"administrative|loss|compromise"`, rootSignature and no new key. Authorize while another key is current, rotate from the wrong current key, or revoke a non-current key rejects; authorize after a prior revoke opens a new interval. The fourth current-candidate arm `action:"resolve_conflict"` is owned exclusively by §9.4 and cannot be emitted by ordinary add/rotate/revoke.

Authorize/rotate possession signs `ASCII "D6-Domain-Seal-Key-PoP/1" || NUL || D3-CJ/3({workspaceRef,revision,predecessor,decisionKey,commitDomain,profile,trustKeyId,algorithm,publicKey})`. Ordinary rotate additionally uses the replaced key under `D6-Domain-Seal-Key-Rotate/1`. The Workspace root signs the complete declaration without rootSignature under `D6-Workspace-Trust-Declaration/1`, so loss/compromise recovery does not require a bad old key to approve its own removal.

A declaration has no caller-selected effective time. For a recorded Declaration1 predecessor, its activation ChangeId is exactly the `ContentCompletionProof/3.changeId` of `declaration.decisionKey` whose existing policy component after-image first appends that exact declaration sequence. The complete CP3/ChangeRecord chain must validate. The same signed declaration cannot be moved to another DecisionKey, backdated, used to fill a gap, or chosen by arrival order. Concurrent different policy/trust successors of one predecessor are an existing policy conflict; no LWW applies and new signing that depends on the ambiguous bundle is unavailable until the existing conflict-resolution path selects a canonical branch.

The normalized verifier remains:
~~~json
{"kind":"d6_revision_token_seal_verification_key","version":1,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","trustKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>"}
~~~
It is derived only from the protected anchor plus exact portable history; there is no validator Boolean or caller field. `history_at(C)` is the greatest complete trust-revision prefix whose activation ChangeIds are causally included in verified Frontier cut C; same-DecisionKey declarations sharing one activation ChangeId are all included or all excluded. Ordinary replay for one exact (CommitDomain,profile) is none -> authorize K -> rotate K→K2 -> ... -> revoke K -> none. A §9.4 `resolve_conflict` declaration is accepted only after its selected bundle address, resolvedHeads, root signature, inheritedCompromises and complete outcomes validate; it applies the listed `keep_current|none|authorize_fresh` result to each exact domain/profile in outcomes and leaves every unlisted selected-chain state unchanged. `authorize_fresh` additionally validates its public-key hash and possessionSignature. Thus the linear successor chain remains executable without consulting arrival order or current host state. `validate_historical(K,C)` requires the anchored root, every signature/predecessor link including any resolve_conflict transition, K as the unique authorized key at that exact historical cut, and that predecessor artifact/CP3 cross-check. Later ordinary rotation/revocation never backdates C. After a `mode=compromise` declaration is admitted, or after §9.4 carries that compromise through an inheritedCompromise, an old-key artifact remains historical-valid only when its seal ChangeId is proved causally before the original compromise activation ChangeId; concurrent or later old-key seals are rejected.

`authorize_new_sign(K,currentCut)` is separate. Inside the single original P seal it revalidates the current non-conflicted WorkspaceAuthorizationBundle, exact active CommitDomain/fence, and K as current for the exact domain/profile at currentCut, and requires the matching usable host-protected private-key handle. The plan may retain its prepared trustRevision/key tuple, but final authority comes only from this same-P check. If rotate/revoke enters the applicable current cut first, K cannot sign; if the author seal commits first, its exact artifact remains historical after a later ordinary rotate/revoke. Causally concurrent offline branches are not converted to arrival order: policy conflict blocks later new signing, while historical validation follows the proved cut; compromise uses the stricter concurrent rejection above.

Private material exists only in protected `WorkspaceTrustRootKeyHandle/1` and `DomainSealKeyHandle/1`, which bind Workspace, exact domain/profile/key tuple and an opaque admitted secure-store handle. Core key generation occurs inside that store. Legal key import uses a trusted host key-import channel and never puts private bytes in an ordinary D6 request, log, Workspace, Draft, sync payload or transcript. Handle state staged|usable|retired|lost is operational, not authority. File sync/copy transports public bundle/declarations/artifacts only and never copies signing rights. Root-key loss blocks new trust mutations but not existing domain signing or historical verification; domain-key loss blocks new managed seals until a root-authorized rotate/add creates a usable replacement. Old private keys may be destroyed after no legal planned signer can use them; public history remains.

Actual trust-management entries are closed Control/2 requests and accept no caller key material:
~~~json
{"wireVersion":2,"kind":"d6_domain_seal_key_add_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","expectedTrustRevision":<Counter>,"budget":<BudgetBinding>}
{"wireVersion":2,"kind":"d6_domain_seal_key_rotate_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","expectedTrustRevision":<Counter>,"expectedTrustKeyId":"sha256:64-lowercase-hex","mode":"ordinary|loss_recovery|compromise","budget":<BudgetBinding>}
{"wireVersion":2,"kind":"d6_domain_seal_key_revoke_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","expectedTrustRevision":<Counter>,"expectedTrustKeyId":"sha256:64-lowercase-hex","mode":"administrative|loss|compromise","budget":<BudgetBinding>}
~~~
They require current workspace-scope policy_admin, exact current trustRevision, current state disclosure, an anchored root and usable root private-key handle. Add/rotate generates the new domain key and PoP inside Core; ordinary rotate also proves the old key, while loss_recovery/compromise does not. Each is a strict managed_atomic portable control change that updates only the existing policy component/bundle in the original planning/install/P/Notice3/CP4/ChangeRecord1 chain, receives one ordinary ChangeId, has no sourceChanges/revision-token artifact, and creates no second ledger/CAS/commit point. Replica registration, fresh bootstrap and continuation/failover use the specialized same-record rules below.

The internal protected key is closed `RevisionTokenSealKey/1 = {"changeId":<ChangeId/1>,"entityRef":<EntityRef>}`. The existing decision outbox stores one closed `RevisionTokenSealOutboxItem/1` per actual managed after:
~~~json
{"kind":"d6_revision_token_seal_outbox","version":1,"key":<RevisionTokenSealKey/1>,"artifactPin":<PinRef/2>}
~~~
The pin's protected record has payloadKind=portable_metadata and retains the exact canonical `RevisionTokenSealArtifact/1` bytes. key.changeId equals association.changeId and key.entityRef equals association.sourceVersion.entityRef. For one decision these items are complete and unique for every non-absent managed after and are ordered by complete EntityRef canonical key. This is an item in the existing P/outbox, not a second ledger, CAS, receipt, Notice component or CP3 member.

Every plan that can produce a managed after allocates exactly one proposed revision token before C/Q or other revision-bound materialization and saves the complete binding with that plan. Both Q passes, restart and recovery of that plan reuse it. The winning planning CAS freezes candidate map, SourceRevisionPlan, afterPin, H/empty-history basis and this binding together. A CAS loser, aborted/terminal-failed plan, or unproved seal never acquires a canonical production binding even if another decision later seals the same numeric H+1 or a byte-equal SourceStamp.

At the single P seal, every actual newly sealed managed SourceVersion/2 selects exactly one canonical RevisionTokenBinding/2 from the winning plan, even when no Locator yet refers to that version. After allocating the one ChangeId and forming the actual managed SourceVersion, the same P transaction constructs the exact RevisionTokenSealAssociation/1, calls authorize_new_sign for the winning plan's frozen trust key against the same final WorkspaceAuthorizationBundle/Frontier cut, records that protected trust cut with the decision, signs RevisionTokenSealSignedBody/1, persists the canonical RevisionTokenSealArtifact/1 bytes and portable_metadata pin, and writes the matching RevisionTokenSealOutboxItem/1. Signing failure or ambiguous/unavailable historical trust aborts the seal before commit. Later observers, replica registration, I rebuild, retry or first Locator use never mint, reconstruct or re-sign another artifact/token for that production version. Canonicality comes from the winning-plan binding plus this original same-P signed artifact, not SourceStamp equality alone.

Historical predecessor publication carries the exact retained RevisionTokenSealArtifact/1 bytes separately from CP3. Its InstallationNotice/2 and ContentCompletionProof/3 keep their recorded closed shapes: neither receives a binding, association, signature, artifact pin or new component. Fresh current publication instead uses the existing Notice3/CP4/ChangeRecord1 chain and likewise keeps the signed artifact in the separate existing outbox; neither path introduces a second ledger/CAS. The version-qualified portable proof chain proves the decision/change/version history; the Ed25519 artifact signature under the historical portable-trust key authenticates the exact binding bytes. Neither proof substitutes for the other. Stamp equality, sender/forwarder trust, equal bytes/digest, or a caller assertion cannot authenticate a random token. Missing, malformed, non-canonical, untrusted or signature-invalid artifact evidence follows the existing incomplete/unavailable boundary and creates no mapping. Two non-byte-equal RevisionTokenSealArtifact/1 records that both validate from the original portable-trust history for the same exact managed SourceVersion are a reachable integrity contradiction; Core never chooses by arrival order.

Stable managed address resolution and current observation qualification are separate. A new authorized read resolves the revision token to its exact sealed production SourceVersion/2, then independently obtains a real current SourceObservation/1 in the caller's observerDomain. Qualification requires byte-equal Observation.sourceVersion plus current FileObjectBinding, observation generation, pins, authorization and owner dependencies at the requested cut. Changing observerDomain/current observationEpoch does not rewrite the portable token. A watcher gap/replacement/rematerialization invalidates the old current Observation and every runtime selector/preparation that depended on it, but does not by itself rewrite the stable production address. Equal final bytes never establish the old production version when actual history differs or is unproved. A successful new Observation grants only a new read: it never mutates or revives an old SourceVersionRef/sourceToken, selector, ActionEvidence, PreparedActionBinding, Draft/map, PreparedIntent, or saved plan.

The external arm has no managed seal or managed canonical-binding publication. Its token resolves only with original protected external-event evidence for the complete external SourceVersion/2. Another observer seeing equal bytes does not inherit that event/token and establishes its own real external Observation/version. If original external-event evidence and receiver current Observation independently prove the exact same external SourceVersion, a new read may use that address under the ordinary authorization and coordinate/profile gates; otherwise the persistent external position is unavailable/stale for that use. Ordinary authorized external read/repair/Draft/whole-source paths remain available, and a structured path needing a managed inner revision still performs explicit managed admission first.

Before seal, a managed SourceStamp/binding is Core-only evidence inside the same original plan for the stage12 candidate map, D7 DefinitionTransfer two-pass Q/Locator materialization and other explicitly managed proposed-position validation. It grants no current Locator capability. After seal, the canonical token becomes a stable production address only through the exact winning-plan binding plus verified RevisionTokenSealArtifact/1 above; current use still requires the independent current Observation test. Historical d6d/d6r/d6a revision-token profiles, the original opaque lexical decoders for DocumentRevision/ResourceRevision/AnnotationRevision, their original store-incarnation/numeric-revision meaning, and saved bytes all remain intact. They are never re-encoded as d6_source_revision/2 and no new D6 lexical pre-gate is inserted into an old D3 Locator decoder. D3 Locator members continue to carry revision tokens as the existing opaque string, while D4 inner sourceRevision/OccurrenceKey/Entry/Type/RelationReadContext/Binding/Recurrence and D5 revision-bound locator wire remain shape-identical.
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

## 3. InputDescriptor/3, PinRef/2, and PreparedIntent/3

### 3.1 PinRef/2

PinRef is a managed internal closed object and is never accepted as caller-authored evidence:

~~~json
{"kind":"d6_pin_ref","version":2,"pinToken":<Token>,"payloadKind":"exact_source_document|resource_bytes|annotation_value|portable_metadata|effect_bytes|artifact","byteLength":<Counter>,"sha256":"64-lowercase-hex","retentionClass":"recovery|conflict|external_unknown|approval_money|query_preview|import_export|user_history"}
~~~

A source pin also stores the complete SourceVersion/2 and EntityRef in its protected record. pinToken tag is d6_pin/2.

### 3.1.1 ConflictInstallInput/1: resolution-only physical before

ConflictInstallInput/1 is a protected D6 installation input used only by the D3 resolution path in D3 §10.1. It is a different type from SourceObservation/1; the ordinary rule that an unresolved conflict branch yields no successful current Observation remains unchanged. There is no public read/prepare RPC for this input, no sourceToken or SourceVersionRef, and no qualification for Query, D8 editing, ordinary source save or raw D3 execution.

~~~json
{"kind":"d6_conflict_install_input","version":1,"decisionKey":<DecisionKey/2>,"principalAudienceToken":<Token>,"bindingToken":<Token>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"entityRef":<EntityRef>,"installedHead":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<present FileObjectBinding/1>,"sourcePin":<PinRef/2>,"metadataPin":<PinRef/2>}
~~~

All members are required and closed. Only the trusted D3 resolution preparer may request this D6 producer after current conflict_read/conflict_resolve, full potential subject/structure/source/body/Field/control disclosure and actual write permissions, domain/P/backend continuity, and complete expectedKey equality have passed. An unknown or undisclosable conflict/input address is not_visible; missing or unproved current installation/provenance is source_unavailable/proof_unavailable under the original owner mapping, and reachable contradictory evidence is integrity_conflict. No partial wrapper or branch bytes are returned. A known changed conflict remains conflict_changed under the existing prepare order.

The wrapper proves actual installed-before state, never merely a historical source with equal bytes. decisionKey supplies the current observer/installation domain; its Workspace equals expectedKey, entityRef and the original request. installedHead is a verified head of expectedKey. The complete sealed history at that head proves sourceVersion for this entity and its actual installed claim/metadata; sourceVersion.changeId may be an ancestor of installedHead but must be proved at that cut. sourceVersion is managed, sealed and byte-equal to that proved production version. It may have a different production domain or production observationEpoch. The wrapper observationEpoch is the current installation generation and equals fileObjectBinding.observationEpoch. The present FileObjectBinding, full exact sourcePin and actual installed metadataPin are established together under the same trusted snapshot/write barrier or gap-free installation history and final revalidation. metadataPin has payloadKind=portable_metadata and includes the complete D3-owned installed claim/lifecycle/structure facts needed to match that head, validated by that owner; sourcePin has the entity's real exact_source_document/resource_bytes/annotation_value decoder. A digest comparison without installation provenance, a placeholder, missing component, unsealed external/third-state bytes, another head's borrowed metadata or unknown installation cannot produce this wrapper. There is no absent or external fallback.

The existing d7_preparation bindingToken and trusted principalAudienceToken match the complete D3ResolutionInput/1, D7 /3 record and D3ResolutionInputUse/1 guard. Core atomically saves wrapper, guard, minimal mapping, complete record and all referenced pins before any prepared success. Every wrapper and pin retains an immutable resolution-use association to this DecisionKey, bindingToken, audience and complete original InputDescriptor. Copying a wrapper, its pins, descriptor text or digest into another key, ordinary descriptor, another preparation or another owner never grants use. Removing the association fails closed; missing P continuity never reconstructs it from files or historical hashes. Same-key different request still reaches original stage5 operation_id_conflict before new business validation; saved/planned recover their original records before current expectedKey/TTL checks.

InputDescriptor/3.sourceInputs remains the closed array of ordinary SourceObservation/1 inputs and never accepts this wrapper. The complete resolution record separately carries every conflict installation input; its sourcePin/metadataPin are included in the native OwnerInputBinding pin set and original guard. DependencyProof/3 still includes the real source key for each such entity, conflict_record key, every actual lifecycle/placement/inbound/Registry/rule/authorization range and negative dependency. Only this guarded D3 resolution context may establish that source entry from the complete wrapper and current source/control continuity; it proves installation/selected-branch planning, not canonical-current source or a reusable complete Query cut. Omitting source entries, substituting ordinary Observation, or exporting this proof as ordinary qualification is forbidden. Written components later compare against this original plan's after, and unwritten components against its wrapper before; Core's own install does not demand re-signing a new before.

An unreferenced expired preparation cannot establish a new decision. Once referenced by a planned/saved/unknown decision, wrapper/guard/mapping/pins retain that decision's original last-reference, budget, installation and recovery obligations. No public current source token exists until the resolution actually seals and ordinary current observation is independently re-established. This is an installation-input producer, not a new identity/source-resolution business owner or a second ledger.

### 3.1.2 SourceConflictBefore/1 and SourceConflictVersionBasis/1: D6 source-conflict installation input

These protected types are produced only for the D6-owned `source_merge` and `choose_source_head` arms in §9.4. They are deliberately distinct from D3 §10.1 `ConflictInstallInput/1`: D3 keeps its existing guard and SourceRevisionPlan/2 decoder, while the D6 source arms receive no D3 resolution authority. An unresolved `source_concurrent` subject still has no successful ordinary current `SourceObservation/1`.

`SourceConflictBefore/1` is the complete physical installed-before:
~~~json
{"kind":"d6_source_conflict_before","version":1,"decisionKey":<DecisionKey/2>,"principalAudienceToken":<Token>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"ownerNodeRef":<NodeRef>,"installedHead":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>,"observationEpoch":<Counter>,"fileObjectBinding":<present FileObjectBinding/1>,"sourcePin":<PinRef/2>,"metadataPin":<PinRef/2>}
~~~
All members are required and closed. The §9.4 producer runs only after closed decode, minimum subject disclosure, current `conflict_resolve` plus the actual source/body/Field/control read and write permissions, exact `expectedKey`, CommitDomain/P/backend continuity, and the pre-read ObservationScope selection below have succeeded. `expectedKey.kind=source_concurrent`; ownerNodeRef is the exact entity subject; installedHead is one verified member of expectedKey.heads. The sealed history at installedHead proves sourceVersion for that entity and the actually installed claim/metadata at that cut. sourceVersion is the complete managed production version actually installed, not a selected historical alternative. observationEpoch is the current installation generation and equals fileObjectBinding.observationEpoch. The present FileObjectBinding, exact sourcePin and metadataPin are captured under one trusted snapshot/write barrier or gap-free installation history plus final revalidation. Placeholder, absent, external/third-state bytes, missing physical metadata, a borrowed head pin, digest equality without provenance, or unknown installation cannot produce this type.

`SourceConflictVersionBasis/1` is the complete typed branch-production basis and is arm-matched:
~~~text
{"kind":"source_merge","baseSourceVersion":<managed SourceVersion/2>,"headSourceVersions":[{"head":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>}...]}
{"kind":"choose_source_head","head":<ChangeId/1>,"sourceVersion":<managed SourceVersion/2>}
~~~
For source_merge, baseSourceVersion is the exact common/base production version bound by baseSourcePin and headSourceVersions is complete for expectedKey.heads, ChangeId-sorted/unique, with each full sourceVersion byte-equal to the protected record behind the matching head sourcePin. For choose_source_head, head equals resolution.head and sourceVersion is the complete production version proved by that head exact source pin/history. Every version is validated from real sealed branch evidence; equal bytes, digest, bare revision, I cache or a caller-constructed version cannot substitute. The full protected ConflictResolutionInput/2, branch/source/metadata pins and both typed objects are retained together under one immutable resolution-use association to the same DecisionKey, principalAudienceToken, expectedKey and exact resolution arm. Copying either object or pin to another plan, key, audience or arm fails closed.

Neither type enters `InputDescriptor.sourceInputs`, produces a sourceToken/SourceVersionRef, or qualifies ordinary read/save, Query, D8 or another owner. The exact conflicted subject `source` DependencyKey may be established only inside this named D6 resolution guard from the complete before+basis plus current conflict/source/control continuity; it is not a reusable canonical-current source proof. Any unrelated non-conflicted source that this preparation actually reads still requires its real current SourceObservation/1 and therefore remains in sourceInputs. Once a planned/saved/unknown decision references these records, their full bytes, pins, association and recovery state retain the original last-reference obligations; no retry reconstructs them from current files or I.
### 3.2 OwnerInputBinding/2

~~~json
{"kind":"d6_owner_input_binding","version":2,"protocolOwner":"D3|D6|D7|D8|D9","ownerKind":<controlled-text>,"canonicalDescriptorBytes":<immutable-bytes>,"pinRefs":[<PinRef/2>...]}
~~~
protocolOwner is input-descriptor owner, not final decision owner; DecisionKey still permits only D3 or D6 decision. ownerKind is owner-version frozen, never free callback/JSON; descriptor is complete and large bytes use typed PinRef slots only. The current A2 D3 candidate defines fresh `d3_identity_operation/13` on wire13 and its native Descriptor3/companion consumption; genuine wire9–12 records remain exact historical dispatch. That bounded D3 closure does not accept this D6 repair under D6 Main §18; still-missing D7/D8/D9 consumer coordination gates affected unseen requests under §5, without re-gating saved/planned recovery.

### 3.3 ObservationScope/2

Closed profiles: local_source{workspaceRef,commitDomain,ownerNodeRef}, owner_fields{workspaceRef,commitDomain,ownerNodeRef,fieldIds}, local_structure{workspaceRef,commitDomain,operation,subjects,parentCandidates}, workspace_constraints{workspaceRef,commitDomain}, control_only{workspaceRef,commitDomain}, prepared_workspace{workspaceRef,commitDomain}. local_structure.operation=create_node|move_node|reorder_node|trash. Scope is observation upper bound, not write permission/write set/completeness proof. Policy/3 explicit structure_state and portable_frontier_state disclose only portable structure and complete Frontier/2 respectively.

### 3.4 DependencyProof/3 and DependencyKey/3

The existing DependencyProof/3 closed envelope remains:
~~~json
{"kind":"d6_dependency_proof","version":3,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"baseFrontier":<Frontier/2>,"entries":[{"key":<DependencyKey/3>,"stamp":{"epoch":<Token>,"revision":<Counter>},"evidencePins":[<PinRef/2>...]}...]}
~~~
The actual `entries` array may be empty or contain multiple items, and each `evidencePins` array may be empty or contain multiple items. Array cardinality, uniqueness, and ordering are defined here; omission of a JSON member never means “unknown”.

DependencyKey/3 is a closed union of exactly fifteen current kinds. Every key contains exactly `kind`, `workspaceRef`, and the additional members listed for its variant. Unknown/missing/duplicate members, illegal null, wrong union arm, wrong Ref domain, or an extra alias is `invalid_request`. `workspaceRef` equals DependencyProof.workspaceRef and commitDomain.workspaceRef. Ref sets use the existing D3 RefKey canonical ordering with no duplicates; FieldId sets use D4 canonical ordering. Author sibling order, Entry order, relation-fact order, and other semantically ordered lists are not sets and are never reordered merely to canonicalize a DependencyKey.

The fixed kind ranks are source=0, document_format=1, lifecycle=2, placement_range=3, ref_inbound=4, relation_incidence=5, calendar_scope=6, registry=7, temporal_rules=8, authorization=9, foreign_binding=10, query_scan=11, replica_registry=12, conflict_record=13, execution_resource=14. DependencyProof.entries sort first by kind rank and then by unsigned byte-lexicographic order of D3-CJ/3 canonical UTF-8 bytes for the complete key; complete keys are unique, so the same key never appears twice under different stamps. `evidencePins` are PinRef.pinToken-canonical sorted/unique. Refs and nested scopes belong to the outer Workspace unless an original D3/D4 author-value type explicitly permits preservation of a foreign Ref; preservation never grants enumeration or disclosure in that foreign Workspace under this key.

The fifteen exact key shapes are:

~~~text
source:
  {kind:"source",workspaceRef,entityRef:EntityRef}

document_format:
  {kind:"document_format",workspaceRef,ownerNodeRef:NodeRef}

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

#### document_format current dependency

For fresh-current Key3, `document_format(workspaceRef,ownerNodeRef)` is rank 1. Any operation that semantically parses a managed Document freezes the exact ManagedDocumentFormatBinding/1 and DocumentFormatCurrentQualification/1 stamp/component image/bindingPin at the same authorized cut. Equal source bytes/version never substitute for format equality. Operations that do not consume managed-Document semantics do not fabricate this dependency. A format-binding change invalidates the old Proof3 and is revalidated before the original planning CAS and final P.

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

D3 remains owner of lifecycle/placement/ref-inbound enumeration algorithms. This file freezes the key carrier, completeness proof, and D6 commit/recovery consumption boundary without becoming a second identity owner. The current A2 D3 wire13 candidate supplies these range enumerations and their consumption; its two bounded findings are independently CLOSED, while this D6 repair still requires independent review under D6 Main §18, and a new strong path still missing a required coordinated consumer remains `owner_update_required`/`proof_unavailable` at the unseen gate in §5; an otherwise qualified ordinary local operation that does not depend on such a range is not permanently disabled.

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

#### Owners, completeness, and disclosure for the fifteen current kinds

- `source`: D6 owns observation/version/file-binding proof while content semantics remain with the Node/Resource/Annotation owner. Outside the two explicitly guarded conflict-resolution contexts, a positive proof binds the complete current SourceObservation/1, exact bytes/value pin, FileObjectBinding, current source validity, and actual use by this operation; SourceObservation.entityRef equals key.entityRef. D3 §10.1 may establish its exact source entry only from ConflictInstallInput/1 under the D3 resolution guard; D6 §9.4 source_merge/choose_source_head may establish the conflicted subject source entry only from SourceConflictBefore/1 + SourceConflictVersionBasis/1 under d6_conflict_resolution/2. Neither exception creates canonical current Observation or a reusable source proof. An absent source is proved from real identity/lifecycle/FileBinding plus a trusted absent object and never from a placeholder, I miss, I/O failure, or “not downloaded”. A narrow Field path may let trusted Core read/preserve a complete source internally, but without source_read it never returns body bytes to the principal.
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

The `stamp` shape remains exactly `{"epoch":<Token>,"revision":<Counter>}`; no epochToken/revisionCounter aliases are created. epoch is the continuity generation of proof for this exact DependencyKey, not SourceVersion production observationEpoch, SourceObservation.observationEpoch, CommitDomain generation, or a Workspace-global generation. Its protected record binds back to the complete DependencyKey/3 and is never caller-issued.

After a real complete enumeration, a new epoch may start at revision=0. Zero means only “first complete proof revision in this new range generation”; it never means source revision0, unknown, empty, or not-scanned. Within a provably continuous event stream for the same epoch, any source, identity, membership, order, control, authorization, or visibility change that can affect the key first invalidates the old current proof; revision then checked-increments and the required current validation completes before a new current stamp is published. MAX never wraps.

A watcher/journal gap, owner-rule/decoder version change, unrecoverable event gap, actual loss of correctness-evidence directory, or unproved range continuity creates a new epoch. Equal final content/hash/member count never restores the old epoch. Deleting or rebuilding Derived Index alone does **not** delete correctness-control facts still complete in their actual Durable Control Store/Portable Workspace Metadata owner. If those facts and the continuous event stream remain intact, I rebuild only repopulates candidate cache and neither re-signs proof nor changes epoch. If the real proof/continuity facts were actually lost, the old proof is permanently invalid and only a new complete current-authorized enumeration establishes a new epoch; I or repeated hash cannot resurrect it.

Empty and non-empty ranges have the same completeness standard: establish the key's required state/content/metadata authorization first, then traverse every applicable directory, shard, owner range, and source and prove no omission. Placeholder, unknown decoder, I/O failure, missing shard, hidden unauthorized range, partial index, or “no candidate hit” cannot produce empty success. Complete enumeration is backed by a trusted consistent snapshot/range read barrier or a complete gap-free change stream plus final revalidation. Repeated scans with equal hash are not a consistent cut. If the backend can prove only local evidence, complete proof is unavailable, while an ordinary operation whose contract requires only that local evidence may continue under its own qualification.

The last evidence pin/range proof still referenced by a planned decision remains durably protected under its real P/portable owner and PinRef retention. Capacity shortage rejects creation of a new proof/plan or puts the planned operation in paused_capacity; Core never drops last-reference evidence and guesses it back from I. A proof stores closed key, stamp, required pins, and protected enumeration/continuity evidence, not another current-body mirror.

Only existing public error/state surfaces are used:
- current disclosure/authorization failure is `not_visible` and stops before hidden-range access;
- authorized but currently unreadable source bytes use the applicable `source_unavailable`;
- authorized but complete range/decoder/continuity proof cannot be established uses `proof_unavailable`; unproved CommitDomain/P continuity itself remains `domain_unavailable`;
- for an unseen canonical intent, a complete current proof that deterministically establishes an impermissible change of a bound dependency uses the existing step-6 `dependency_conflict`;
- after a decision is planned, revocation, unknown continuity, or installation competition is never converted into a new recorded business rejection. A proved competing dependency follows the original plan's conflict/paused recovery path; unproved continuity, installation provenance, or outcome remains paused/recovery_unknown. The exact submit/recovery ordering is defined in §5/§8; uncertainty must not be hidden behind a business terminal outcome.
No error gains free details, hidden counts, partial-member lists, or internal cause.

### 3.5 InputDescriptor/3

The closed shape remains:
~~~json
{"kind":"d6_input_descriptor","version":3,"workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"intentKind":<controlled-text>,"saveProfile":"ordinary|complete|control_only","guarantee":"replica_local|managed_atomic","expectedFrontier":<Frontier/2>,"frontierPolicy":"exact|scope_dependencies","observationScope":<ObservationScope/2>,"sourceInputs":[{"entityRef":<EntityRef>,"observation":<SourceObservation/1>,"role":"before|dependency"}...],"controlInputs":[{"key":<DependencyKey/3>,"stamp":{"epoch":<Token>,"revision":<Counter>}}...],"ownerInput":<OwnerInputBinding/2>}
~~~
The actual sourceInputs/controlInputs arrays may be empty or contain multiple items; the member set, version, and unions do not change.

workspaceRef/commitDomain are byte-equal to the corresponding outer plan members. DependencyProof/3.workspaceRef/commitDomain equal them and DependencyProof.baseFrontier is byte-equal to expectedFrontier. sourceInputs are sorted/unique by D3-CJ/3 canonical bytes of the complete item; each entityRef equals observation.entityRef and observation.observerDomain equals commitDomain. sourceInputs carry the real current Observation, never SourceVersionRef, hash, or I state. The conflicted subject of D3 §10.1 or D6 §9.4 is deliberately absent from sourceInputs and is carried only by its named protected owner input/plan; this exception never permits a fake Observation. Any unrelated source actually read by the same preparation remains an ordinary sourceInputs item with its own real current Observation.

controlInputs are sorted/unique by the fixed DependencyKey kind rank plus D3-CJ/3 key bytes. Every controlInputs item has exactly one byte-equal key in the same DependencyProof.entries and its stamp is byte-equal; a dependency used by this intent never bypasses InputDescriptor through another stamp. A source DependencyKey used to establish currentness of a sourceInputs item points at the same entityRef and is consistent with that Observation's file object, pins, and current cut; an equal bare revision number from another production domain is never a match.

ObservationScope/2 remains a prior authorized observation upper bound, not dependency completeness or write permission. Every actual DependencyKey read lies within that scope and current Policy authorization. Where the D3/D4/D7 owner algorithm has not yet supplied the complete range required here, the path remains owner_update_required/proof_unavailable; the existence of this union is never itself a completeness proof.

expectedFrontier remains the original prepared input and is never resampled at commit. frontierPolicy=`exact` retains full Frontier equality. `scope_dependencies` admits only the proved unrelated non-regressing extension defined by §1.4/§6.3 while revalidating every bound source/control/auth/positive-negative dependency. An unrelated coarse Frontier advance alone need not fail a local ordinary intent, and a real bound dependency change is never ignored because scope_dependencies was selected.

Complete input equality compares full InputDescriptor canonical bytes, complete OwnerInputBinding descriptor, every protected exact PinRef record, SourceObservation, DependencyProof keys/stamps/pins, and every other fixed owner input. Equal hash, equal final text, equal revision number, or rebuilt I is insufficient. Changing source/current Observation, any key/stamp, mapping, Policy/Registry/rule, scope, frontierPolicy, WriteProtection, owner request, or version basis requires a new prepare. The same OperationId remains exact replay only.

### 3.6 PreparedIntent/3

The PreparedIntent/3 immutable member set remains unchanged:
identity/context are kind,version,planToken,operationId,workspaceRef,commitDomain,principalAudienceToken,inputDescriptor,beforeCut;
proposed state/proof are proposedState,mutationFootprint,dependencyProof,observationProof,budgetBinding,pinDirectory;
installation/delivery are installationPlan,inputRetentionState,expiresAt,previewBinding.

kind remains d6_prepared_intent, version=3, and planToken is tagged d6_plan/3. principalAudienceToken equals the trusted authenticated audience for this plan. Any authorization or query_scan key in the same plan has byte-equal principalAudienceToken. dependencyProof.workspaceRef/commitDomain/baseFrontier respectively equal PreparedIntent workspaceRef/commitDomain and inputDescriptor.expectedFrontier; the key/stamp relationships between inputDescriptor and dependencyProof satisfy §3.5 item-for-item. observationProof covers sourceInputs current-observation qualification and is consistent with the same current cut's file object/evidence pins and applicable control/Registry/incidence dependencies; it never substitutes for DependencyProof.

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

The v2 ledger key is exactly (workspaceId, D3-CJ/3(commitDomain), operationId), with protocolOwner=D6 in the record. Same key/different canonical request is operation_id_conflict. The independently closed bounded D3 current candidate uses wire13 with `d3_identity_operation/13` and defines native Descriptor3/companion consumption while sharing the same DecisionKey owner collision; genuine wire9–12 records remain historical exact dispatch. Candidate presence does not grant D6 Main §18 acceptance/activation; an affected unseen protocolOwner=D3 request still missing required coordinated consumers returns owner_update_required with zero new D3 decision. Existing saved/planned/unknown decisions retain the original-record split below.

### 4.3 D10 Workspace control predecessor and current successor dispatch

D10ControlPrepare/1 remains the public management prepare owned by D10 and no second public D6 submit is introduced. The detailed `D10ControlInput/1` + `ControlDependencies/2` block below is retained as the exact predecessor contract for genuine recorded records and as the source-qualified business invariant set. **Fresh current unseen** Workspace control uses the §21 `D10ControlInput/2` + `ControlDependencies/3` + `ControlPrepareBinding/3` successor, current Proof3/Key3, and the same one planning/P ledger; where §21 replaces a versioned carrier, that successor wins without rewriting predecessor bytes.

For the predecessor carrier, OwnerInputBinding/2 uses protocolOwner=D6, ownerKind and InputDescriptor.intentKind both d10_control/1, and the following complete canonical descriptor:

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

These are internal Core-generated bytes, strict-decoded using the actual D10 Control Contract §7–8 types, not caller attestations. The stable key's scope must be this Workspace and its initiatingPrincipal must be the trusted current principal. The first authorized same-key prepare atomically reserves the never-used operationId and ControlRefs, fixes the descriptor and original preparation association, and returns one original d6_commit_request/2. Exact replay restores it; changed canonical intent conflicts; uncertain prior existence creates nothing. All identifiers and allocation history remain retained under the original stable-key rules. No D10 ControlRef becomes an EntityRef.

The descriptor fixes the entire successfully decoded canonical body, actual full before/proposed control images, complete typed artifact pins, original immutable preview, genuine control ranges/dependencies and fixed external-confirmation requirement. Nested original author request A in planned consent is a legitimate immutable part of that body. The generated control request M, its planToken, request hash and future human confirmation fact are absent from the descriptor. After D6 constructs M, Core atomically saves the exact outside ControlPrepareBinding/2→M association before returning it. A crash before delivery restores that same association; no request is generated twice. DependencyProof/sourceInputs and observationScope equal dependencies.workspaceReads.some; corresponding real DependencyKey/3 entries and controlInputs match exactly. The final observationProof is constructed once in outer PreparedIntent/3 from those original inputs/pins, never fed back into ownerInput. Required artifact pins enter the same pinDirectory and retention reservation.

D10 config/usage/stop full values and complete range fences are owner-specific fixed inputs within this InputDescriptor, compared under the real P transaction at planning and seal. This is consistent with §3.5's rule that no dependency bypasses the descriptor; it adds no sixteenth DependencyKey and does not overload execution_resource. Minimum disclosure and current D10 S/W qualifications precede protected reads. P-only Workspace control uses saveProfile=control_only and control_only effects, no source versions or content ChangeId; every D10 branch keeps guarantee=managed_atomic and strict protection. ObservationScope is separately selected from the declared body and protected original-request mapping before author values are read. A body with no author observation uses control_only. A core_field_member/standing-rule configuration whose entire potential author scope is one statically qualified owner/Field uses owner_fields and the original complete D7 Narrow qualification, preserving the narrow self-management path. A recurrence configuration and every activation use workspace_constraints; consent uses the complete original request/effect disclosure scope from its protected mapping. Other genuinely wider or multiple-owner author scopes require workspace_constraints before any read. Actual source/Registry/range reads still use their real keys and current authorization. The profile cannot expand after hidden reads or fall back after failure; control_only never grants a hidden author read.

Activation's actual Registry effect is explicit: when a successor Registry changes portable current metadata, its selected entry uses complete + workspace_constraints and the existing strict portable install/seal/publication path. D10's typed registryChange contains complete original/proposed Registry values, their exact pins and D4 evolution proof; Core validates and installs that one Registry component once. It creates the one portable ChangeId and the same Notice3/CP4/ChangeRecord1 transition, but no source revision/H. If Registry bytes/state are unchanged and only P Catalog/selector changes, it is P-only control_only. This branch is determined by the fixed actual effect before planning, never a failure fallback. Catalog/selector and all D10 control after-images remain unpublished until the same P seal, so portable installation cannot expose a half-active catalog. It does not mutate the author source or create a second Registry.

The Workspace adapter accepts only automation_configure, workspace consent, applicable workspace state, workspace_limits and activation under D10's exact kind/scope authorization. Deployment control, cost_reconcile, secret staging, issuer administration and external send keep their own existing closed domains and never enter this adapter. A selected Registry branch cannot evade registry_admin/policy_admin or its real complete business gates. Fresh bootstrap remains the original D3 producer in §10.2.

### 4.4 D10 live eligibility at the two original CAS points

Unattended D10 author execution requires the actual current D10 ApprovalUse/2, exact LeaseRunUse/1, original D7 PreparedActionBinding/4 and current strict complete qualification. The protected association is created before the original request can be submitted or delivered to the executor; omitting a caller token cannot remove it. The association is not a new public request field or an editable ownerInput. The D10 Workspace external-consent branch similarly has a fixed requirement and a separate protected ExternalConfirmationRecord/1; only the admitted trusted human event may establish a matching fact. Direct submission of the already returned D6 request still finds and validates these original associations.

At §5 step4, saved decisions always replay their original result before fresh approval/time/stop/count checks. Planned restores its exact request/plan/install state and reservations. Only after current D6 disclosure/authorization, domain continuity and the original applicable business/dependency/budget gates, the unseen planning branch checks current D10 qualification. Under the same real store serialization as planning, Core checks exact Run/Lease/activation, trusted time, original approval/effect match, stop latches and external confirmation where applicable; it CASes the actual current eligibility revisions and count totals. Approval count unreserved→reserved is saved atomically with planned. Lease maxRuns is consumed only by the earlier original Run-admission CAS, never at author planning or later recovery. The descriptor does not change when a valid later confirmation or supplemental planned approval is associated with the same original request.

Step11 verifies written components against original after and unwritten dependencies against original before. Step12 repeats current D6 authorization and original business proof, then these same actual D10 gates under the single final P write-serialization boundary. It publishes fixed control effects, decision/receipt/audit and consumes the original reserved approval slot in that one transaction, even for raw no-op. It reads the exact current matching confirmation record/revision rather than demanding its none-at-prepare state stay unchanged. An unrelated usage transition is never overwritten by an old projected after-image; a frozen control write whose actual dependency changed must reprepare, whereas runtime count reservation compares current qualified totals under its own CAS. Saved replay cannot consume again.

If ordinary permission/business gates pass but an associated approval, Lease/time eligibility, supplemental consent or required trusted confirmation is unusable, approval_unavailable/preflight is returned. Unseen creates no P decision/reservation; planned retains planned, all original pins, actual installation state and reservations. This is neither recorded rejection nor terminal failure. An unprovable authority/backend continuity retains its earlier D6 error priority. Current control details may be explained only through a separately authorized D10 read. Irreversible stop uses execution_stopped/preflight before a new plan; an existing plan follows the recovery rule below. Both new codes belong only to the explicit current wire2 closed set and never reinterpret saved older error bytes.

Stop and final P seal serialize in the same actual Authority Store. Seal first fixes the original committed result. Stop first prevents seal, including when physical files have already been installed in step10. Such a plan retains its managed barrier, actual B/N pins, original notice, installation provenance and control/count/cost responsibility; it does not claim that no bytes changed. Authorized recovery may undo only a proved installation owned by that original plan without overwriting a later or third state. Only when the irreversible stop makes original commit impossible and every installation remnant is safely resolved under §8 may the original transaction_aborted/terminal be recorded and its count reserved→released_terminal atomically. Unknown provenance, third state, temporary revocation/expiry/cancel or unproved rollback preserves planned/paused recovery and all liabilities. Costs require their own original settlement evidence. Stop never blocks authorized abort, settlement, audit or reference-safe cleanup, and none of those resumes executor authority.

## 5. Unique submit/install/seal/publish order

The following order explicitly separates saved, planned, and unseen. prepared/retained means only that proposal, read-before, and required bindings/pins are durable; it is not Saved. No branch may replace an existing record for the same DecisionKey by preparing a second decision.

1. closed decode and static cross-field equality; failure is the existing invalid_request/preflight with zero author/business-state reads.
2. under the current authenticated principal, establish the interface's minimum state disclosure, original ObservationScope/2, and prior CommitDomain qualification. Failure is uniformly not_visible. Hidden author facts, result members, or old-ledger business contents are never read first to choose scope.
3. prove CommitDomain, the applicable protected domain fence, portable trust/backend, and P-ledger custody/continuity under the actual entry and original profile. The D6 d6_commit_request/2 entry validates its declared expectedDomainFenceToken; native D3 uses its own domain/authority/custody qualification and carries no such member. D3 create/fork follows §4.5.1 of the D3 owner: issuer A temporarily holds custody for fresh Workspace W/target authority B, while DecisionKey and commitDomain bind W/B from the first request through seal/replay/custody transfer. Its stage4 -> P1 -> P2 -> TL1 -> TL2 -> stage5 order proves issuer/source qualification, then proposal, then target custody; P1/P2 failure reads no target ledger. The fresh target is not required to be active or already have target source_write/policy_admin/current policy, and A never substitutes its own domain or Frontier for W/B. The issuer proves the fresh target empty history under the original bootstrap profile. A continuity-proved older qualification permits original-decision lookup only, not an unseen new decision. Unproved is domain_unavailable or its original D3 availability projection; only proved reachable corruption uses integrity_conflict or its original D3 integrity projection. Business DecisionKey state is read only after this common continuity gate succeeds.
4. locate DecisionKey by `(workspaceId,D3-CJ/3(commitDomain),operationId)` and compare protocolOwner plus original canonical request/fingerprint. Different owner or different request at the same key is operation_id_conflict; the complete original request prevents ledger probing. Then branch on existing state:
   - saved decision, including committed and any recorded/terminal outcome durably saved by its original protocol: under current disclosure/delivery authorization for the **original actual effect/mode or original result-disclosure scope**, return the original sealed receipt/error bytes or resume its original publication/outbox. Do not require the old before SourceObservation to remain current, old Frontier to equal current Frontier, preview/plan TTL to remain valid, or current r6 business dependencies to become true again, and do not re-run business selection. Current revocation may hide delivery but never edits decision, receipt bytes, original ChangeId, charge, or historical recovery responsibility.
   - planned: restore the same original InputDescriptor, applicable SourceRevisionPlan, before/after pins, write set, OperationId, budget/attempt counters, WriteProtection, InstallationNotice, and installation state. Never new-prepare a second success, reselect target/Query/current page, resample identity/H/revision, or edit the original request. Actual current authorization, original dependency continuity, and installation provenance decide only whether that original plan may continue or remains paused/conflict/recovery_unknown. An unsealed planned decision has no ChangeId for this decision.
   - unseen: only this branch continues to step 5 to create a new decision under current producer/consumer contracts.
5. unseen validates the current fence and preparation inputs required by the actual closed entry:
   - D6 d6_commit_request/2 validates its carried planToken tag/audience, selected PreparedIntent/3, inputRetentionState, owner version, and fixed InputDescriptor/DependencyProof/ObservationProof/pins.
   - Native fresh-current D3 `identity_operation_request` wire13 validates its exact InputDescriptor/3, `d3_identity_operation/13` owner descriptor, protected inputs/pins, and native private plan under the D3 stages. A proved genuine wire12 record retains its exact recorded request/owner descriptor and recovery before current unseen dispatch. It neither carries nor requires planToken, a D6 prepare call, PreparedIntent/3, or expectedDomainFenceToken; adding any undeclared member is a closed-decode failure. A D7 preparationBinding is checked under the actual versioned D3/D7 contract on permitted managed_atomic requests. D3 §10.1 resolution-produced input requires its immutable preparationBinding and D3ResolutionInputUse/1 guard; removing the token never turns that input into a qualified raw request. Minimum mapping/usage disclosure precedes branch reads, the existing-key fingerprint comparison precedes new-business validation, and only unseen checks new choice/head/expiry/producer gates. Unrelated native input may omit the binding only where its original mode permits it; omission grants no D7 preparation qualification.
   Each entry retains its actual owner/error ordering and shares the single DecisionKey/P planning CAS; this split creates no second D6 identity submit. A strong consumer missing its coordinated owner afterimage returns owner_update_required/proof_unavailable here; an otherwise qualified ordinary file path that does not depend on that strong range is not permanently disabled.
6. before entering planning, unseen revalidates under frontierPolicy the complete current Frontier/2, every ordinary SourceObservation/1, the D3-only guarded ConflictInstallInput/1 when applicable, or the D6-only SourceConflictBefore/1 + SourceConflictVersionBasis/1 pair when a §9.4 source arm is planned, together with the explicitly decoded SourceRevisionPlan version basis, DependencyProof/3, MutationFootprint authorization, applicable local/complete semantic gates, budget, and every unwritten dependency. exact uses §1.4 complete equality. scope_dependencies admits only the proved unrelated continuous non-regressing extension of §1.4/§6.3. semantic_pending means only that local typed facts passed while an obligation explicitly permitted to remain pending lacks cross-object/complete proof; it never authorizes all_result, bulk, strong Action, Automation, or another success that requires complete proof.
7. For a human ordinary whole-source save, observed_only is explicitly selected and frozen by a trusted `interactive_source_save` human **before planning starts**. All eligibility still holds: exactly one existing live Document; ordinary+replica_local; complete source read/replace; author-source write set empty or limited to that Document; no applicable body/Field/node-control deny; no identity, parent/order, lifecycle, shared-policy, Registry, Calendar-scope, or other-entity mutation; Draft Base equals the selected current SourceObservation. noninteractive, D3 identity/parent/order/lifecycle, D5 structured cell/row/column/reorder, bulk/collection/promotion, D7 strong Action, Automation, server checkpoint, Approval, and Money are always strict. Once planning starts, strict-capability failure, known conflict, revocation, durability failure, strong-obligation failure, or any missing eligibility never falls back to observed_only.
8. the planning CAS atomically stores canonical request, fixed plan, InputDescriptor, DependencyProof, applicable SourceRevisionPlan, write set, exact before/after pins, budget/reservation, recovery description, required WriteProtection, version basis, and planned. Nothing is resampled after the plan wins. Only proposed SourceStamp is frozen here: **no ChangeId is allocated, no SourceStamp/RevisionToken is recorded as sealed SourceVersion, and no fresh D3 Ref is exposed**.
9. before modifying any portable-current component, durably write InstallationNotice/3 with original DecisionKey, guarantee, WriteProtection, notice.baseFrontier, and complete component before/after. baseFrontier may contain historical ChangeIds that already existed; the notice contains no new ChangeId for this not-yet-sealed decision and no receipt, Approval/Money, or external payload.
10. after staging/pins are durable, install under the planned capability. strict uses a real strict FileInstallCapability. observed_only is only the step-7 path and performs the final trusted object/event-continuity check immediately before destructive installation. Its original plan durably retains actual read-before B and user input N. An unseen external C after the final check may be overwritten by N and may have no recoverable copy, and a later C may replace current file again, but durable B/N is not discarded. Any observed competition, stale Base, watcher gap, revocation, or third state is outside the weak relaxation and enters the original conflict/paused/reprepare path; unknown installation provenance is recovery_unknown.
11. verify each written component against the **original planned after** with complete FileObjectBinding/bytes and continuous installation provenance. Core never requires its own written component to remain equal to before after installation. Every unwritten dependency is revalidated against its original before/cut expectation, including source/control/auth, Registry/rules, DependencyProof positive/negative ranges, and applicable Frontier policy. scope_dependencies admits only an unrelated extension proved and retained in P. Unknown provenance, third_state, late competition, revocation, or unproved continuity remains paused/conflict/recovery_unknown. There is still no ChangeId for this decision.
12. seal is the only decision commit point. After written=planned after and original plan/deps/auth, domain fence, SourceRevisionPlan lastIssued/empty-history basis, and applicable installation proof all pass, one durable P transaction checked-allocates the single ChangeId for **this portable decision**. Only an actual source change whose after is managed combines its original SourceRevisionPlan.after SourceStamp with that ChangeId into the unique managed SourceVersion/2 and atomically advances H(D,E) for that production-domain entity. For each such managed after the same transaction selects the exact proposed RevisionTokenBinding/2 frozen by the winning plan as that SourceVersion's sole canonical binding, builds/signs the exact RevisionTokenSealArtifact/1 under §1.5.1, pins its canonical bytes, and writes its RevisionTokenSealOutboxItem/1; no Locator need already exist. Source deletion with after=absent remains a portable effect using this ChangeId but creates no managed SourceVersion, uses no deleted-after SourceRevisionPlan, and does not advance H. A source-unchanged portable structure/lifecycle effect likewise uses the portable ChangeId but creates no source version/H increment. P-only control_only and true raw no_op create no content ChangeId. The same P transaction performs the existing domainCommitSequence update and writes committed decision, original receipt, effects, ReliableSaveState, applicable charge, and outbox. Charges settle once for the original decision and replay/recovery never charges again. strict->reliable and observed_only->durable_observed_only.
13. current portable publication derives ContentCompletionProof/4 plus ChangeRecord/1 only from the facts sealed in step 12 and advances Frontier/2. proof.frontierBefore/frontierAfter are the actual pre/post-seal Frontiers; notice.baseFrontier and InputDescriptor.expectedFrontier retain the original baseline. Under scope_dependencies, publication also validates the complete continuous ChangeRecord/portable-proof chain from the original base to actual frontierBefore, the unrelated-extension judgment retained in P, and continuing validity of original DependencyProof. Publication never reconstructs “unrelated” merely from larger vector numbers, provider state, or equal final hash. The proof transports actual production SourceVersion before/after rather than sender SourceObservation token. Separately, the original outbox publishes each sealed managed after's exact retained RevisionTokenSealArtifact/1 bytes selected by its RevisionTokenSealOutboxItem/1; current Notice3/CP4 receive no extra binding/artifact member beyond their closed current schemas. The receiver verifies the artifact's historical trust key, signature and complete association under §1.5.1 and §6.3 before storing any token→production-version mapping, then creates its own current Observation. ContentCompletionProof/2, /1, and other historical records remain under their original decoder/bytes/recovery and are never decoded as /3.
14. if seal succeeded but publication/outbox/delivery failed, decision, ChangeId, managed SourceVersion/H results, ReliableSaveState, receipt, and applicable charge are already fixed. Recovery only publishes the original proof and the exact already-pinned RevisionTokenSealArtifact/1/outbox bytes for that sealed decision, or delivers original bytes under current authorization. It never reinstalls N, changes OperationId, allocates another ChangeId, advances H again, reruns business selection, writes over a later current source, or charges again. delivery finally rechecks current authorization; revocation may hide delivery but never alters the historical decision.

For observed_only, B is only the actually read/pinned before and never enumerates C unseen after the final check. Later current=C never rewrites the old receipt/Proof and never weakens durable B/N retention. Crash/unknown follows §8 and the client never guesses the outcome.

A true raw no-op remains effectClass=no_op with sourceVersions=[], InstallationState=not_required, ReliableSaveState=not_applicable, and PortablePublicationState=not_applicable. Under the existing contract its domainCommitSequence may advance, but no content ChangeId, source revision, H, or Frontier head does. P-only control uses control_only; a real portable structure/lifecycle/delete/source effect uses portable.

## 6. PortableComponentKey, InstallationNotice, ContentCompletionProof

### 6.1 PortableComponentKey/2

Closed union:

- {"kind":"document","ownerNodeRef":NodeRef}
- {"kind":"document_format","ownerNodeRef":NodeRef}
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

version belongs to the component owner. For the current new-profile `policy` component carrying WorkspaceAuthorizationBundle/1, ComponentImage.version is exactly `bundle.authorizationRevision`, never `Policy/3.revision`; a policy-only or trust-only change therefore advances the component version exactly once. Genuine historical profiles retain the component-version mapping of the contract that produced their bytes and are never reinterpreted by this rule. Digest verifies listed bytes; it is not identity.

### 6.2 SourceStamp/1, SourceRevisionPlan/1, and InstallationNotice/3

The existing SourceStamp/1 closed shape is unchanged:
~~~json
{"kind":"decision_source","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"revision":<Counter>,"observationEpoch":<Counter>}
~~~
It is determinate before installation only as the same original plan's proposed managed-source address. It is not SourceVersion/2, ChangeId, receipt, or second current truth. decisionKey.commitDomain is the proposed managed after's production domain; entityRef belongs to the same Workspace; revision/observationEpoch are 1..MAX. A SourceStamp becomes usable as the version basis of a sealed managed version only after that original plan seals and §6.3 can uniquely match it to the actual managed SourceVersion/2. Without seal there is no managed SourceVersion or ChangeId for this decision.

Except for the explicitly separate conflict-only /2 decoder in §6.2.1, whenever a plan **will produce a new managed after**, installationPlan stores one SourceRevisionPlan/1 for that entity. Its closed shape is:
~~~json
{"kind":"d6_source_revision_plan","version":1,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<SourceObservation/1|"absent">,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

All cross-field rules are mandatory:

- decisionKey equals the outer original plan DecisionKey; after.decisionKey is byte-equal to it; entityRef=after.entityRef.
- after's production domain is decisionKey.commitDomain. A non-`"none"` lastIssued is the latest continuously sealed managed SourceVersion/2 for the same entityRef and production CommitDomain, with revision exactly H(D,E). A larger revision from another production domain, externalSequence, file hash, I cache, or equal text never substitutes.
- lastIssued=`"none"` is legal only when DependencyProof/3 plus protected production history prove a complete empty managed history for that entityRef in the production CommitDomain. Missing history, unproved P continuity, a portable-history hole, MAX/overflow uncertainty, or other unknown state is not `"none"`.
- after.revision is 1 when lastIssued=`"none"` and otherwise checked(lastIssued.revision+1). observationEpoch changes never reset this sequence within the production domain.
- after.observationEpoch is the production-observation generation frozen by the original plan for the target source in the after production domain. A before from another production domain never donates its sourceVersion.observationEpoch to after. Fresh/expected-absent targets use the same plan's trusted absent FileObjectBinding/target-backend observation generation.
- A SourceObservation/1 before is the complete current Observation actually read by the original plan and has before.entityRef=entityRef. before=`"absent"` is legal only when the original D3 identity/owner plan has proved a truly fresh/expected-absent branch; callers never infer it from path absence.
- afterPin is the plan's exact after bytes/value pin and matches entity kind: Document owner NodeRef→exact_source_document, ResourceRef→resource_bytes, AnnotationRef→annotation_value. The protected pin record binds the same entityRef; digest equality is insufficient.
- For D3 fresh content, entityRef/after comes only from the original stage12 private candidate map and winning reservation and is never exposed as a fresh Ref in prepare/preview. The original plan fixes one proposed RevisionTokenBinding/2 before revision-bound source materialization. D7 DefinitionTransfer two-pass Q/Locator materialization uses the same candidate map, after SourceStamp, and that same proposed binding in both passes; the second pass or commit never resamples revision or token. The winning planning CAS freezes this binding with SourceRevisionPlan and pins; loser/aborted/unproved-seal records cannot become canonical through another decision.
- The planning CAS persists complete SourceRevisionPlan, pins, and H/empty-history basis; these members are immutable after the winning plan. Before seal Core revalidates lastIssued/empty-history, domain fence, after pin, and original dependencies. A newly sealed managed version for the same production-domain entity makes the old plan stale/conflict; after.revision is never edited in place to a newer H+1.
- SourceRevisionPlan allocates no ChangeId. At seal, only an actual source change producing a managed after combines after SourceStamp with the seal-allocated ChangeId into managed SourceVersion/2 and atomically advances H(D,E). True raw no-op, pure structure/lifecycle/control with unchanged source, and source deletion after=absent create no SourceRevisionPlan, managed after, or H increment. Deletion and other real portable structure/lifecycle effects may still receive their normal portable-effect ChangeId at seal; absence of SourceRevisionPlan never reclassifies them as no_op.
- Equal-byte external admission still produces a managed after and therefore has SourceRevisionPlan. Its before.sourceVersion is the complete external SourceVersion/2, while after revision comes only from current production-domain lastIssued/H. externalSequence never enters after.revision.

InstallationNotice/3 keeps its existing closed shape:
~~~json
{"format":"weftext.installation-notice","version":3,"decisionKey":<DecisionKey/2>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/2>,"before":<ComponentImage/1>,"after":<ComponentImage/1>}...]}
~~~
components is non-empty, fixed-rank/canonical-key sorted unique. observed_only is §4.1 only; every other portable author plan is strict. The notice is durable before the first portable-current install. baseFrontier is the Frontier frozen for that original plan/notice; it may contain historical ChangeIds already in existence, but the notice contains **no ChangeId for this not-yet-sealed decision**, and no receipt, approval, Money, external payload, or credential. SourceRevisionPlan remains protected P/plan state and never turns InstallationNotice into a second public version table.

### 6.2.1 Conflict-only SourceRevisionPlan/2

Ordinary source production and fresh output from the native copy still use SourceRevisionPlan/1 exactly as above. Only the canonical-source materialization component of D3 §10.1 may use the following separate internal decoder; no public commit wire, InputDescriptor, SourceObservation, SourceStamp or CP4 shape changes:

~~~json
{"kind":"d6_source_revision_plan","version":2,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<ConflictInstallInput/1>,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

All /1 cross-field rules for decisionKey/entityRef, current production-domain H and complete-empty-history proof, immutable proposed after, exact after pin, strict install, CAS, same-P seal and recovery apply unchanged. The sole before arm is the complete guarded ConflictInstallInput/1 from §3.1.1, with equal DecisionKey/entityRef; ordinary Observation, absent, free metadata and caller-crafted wrapper are forbidden. Current production observationEpoch comes from the trusted target installation generation, not selectedHead's historical production epoch. A native fresh copy alongside this component has its own /1 plan and proved fresh absence; the existing canonical entity is never relabeled absent or allocated again. One entity has at most one resulting source-version plan in the single decision; overlapping canonical/native edits require the exact before/final bytes, metadata, version-basis and typed-effect equality specified by D3 §10.1.1.3, or reject at its stage14 before planning, never installed as two writes/revisions.

D3 chooses the canonical claim/lifecycle and full selected source under its explicit resolution binding; D6 only installs its exact validated final after. Source-state change has one rule: different final bytes/value, or an explicit switch of the canonical birth claim or selected production SourceVersion, is a real canonical admission even if bytes are equal. It receives one managed after with checked H(current production domain,entity)+1 and this decision's single seal ChangeId. If final bytes/value, selected production version and claim are all retained, no /2 plan, sourceChanges entry or H increment is produced; a placement/lifecycle/conflict-state-only portable change may still use the decision ChangeId. A proved already-resolved selection with every author/control effect empty is true no_op, with no ChangeId or H advance. Fresh-copy always allocates its admitted fresh identity and is never no_op.

CP4 sourceChanges.before is the wrapper's actual installed production SourceVersion; after is the managed SourceVersion combining this exact after SourceStamp with the common seal ChangeId. It never substitutes the selected historical head's version as physical before or copies that old version into a new managed after. The original notice, complete physical metadata/source components, conflict effect, primary D3 receipt, companion and effects are committed under one P transaction. Planned/unknown recovery retains the original /2 wrapper/version basis/pins/guard and resumes only that installation; it never reacquires a current Observation, reselects the head, changes H in place, or guesses seal from files. Actual legacy /1 plans remain /1 and are not upgraded. Decoder selection is explicit version, never presence/absence of fields.

D3 §10.1.1.3 additionally owns the protected D3CanonicalEffectPlan/1 and public D3CanonicalEffects/1 semantic extension. The former, all exact before/selected/Result9 pins and the original native plan remain atomically bound by the existing D7 /3 record, native descriptor and input-use guard; no new ownerKind or submit is introduced. The original twelve primary receipt arrays cover only the native component. The extension independently proves the actual canonical component, with its own original-schema reference/S/lifecycle evidence and explicit physical-effect aliases. D6 saves both complete components and their D7 full public projection with the same primary receipt/companion at P seal. D6 sourceVersions and CP4 cover every actual physical source change once, including canonical changes absent from the primary arrays; aliases neither duplicate writes nor H. Missing mandatory extension prevents new preparation/seal success, while missing delivery evidence after a real seal reports effects_unavailable without changing that historical commit. Saved/planned/unknown keep the original complete plans/extension/pins; ordinary Observation and real historical decoder rules do not change.

### 6.2.2 D6 source-conflict SourceRevisionPlan/3

Only the D6-owned §9.4 `source_merge` and `choose_source_head` arms may use this separate internal decoder. D3 canonical resolution remains exclusively on /2, and ordinary/fresh source production remains on /1. No public commit wire, InputDescriptor member set, SourceObservation, SourceStamp, CP4 or DependencyKey union changes.

~~~json
{"kind":"d6_source_revision_plan","version":3,"decisionKey":<DecisionKey/2>,"entityRef":<EntityRef>,"before":<SourceConflictBefore/1>,"versionBasis":<SourceConflictVersionBasis/1>,"lastIssued":<managed SourceVersion/2|"none">,"after":<SourceStamp/1>,"afterPin":<PinRef/2>}
~~~

All /1 rules for the outer DecisionKey/entityRef, exact current production-domain H or proved complete empty history, checked H+1, immutable SourceStamp, exact after pin, strict installation, one planning CAS, one P seal and recovery apply. before.decisionKey/entity and the complete resolution-use association match the outer plan; versionBasis arm equals the frozen ConflictResolution/2 arm. after.observationEpoch is the trusted current installation generation from before.observationEpoch in this operation production domain, never a historical head production epoch. There is no absent branch and no ordinary Observation substitute.

For `choose_source_head`, a source-state admission is required when the final exact bytes/value differ from before.sourcePin **or** versionBasis.sourceVersion is not byte-equal to before.sourceVersion. Thus two sealed heads with identical bytes but different production SourceVersion/2 values still produce one /3 managed after and checked H+1 when the other production version is selected. If the selected head exact production version and final bytes/value are both already the installed before, source is unchanged: no /3 plan, CP4 sourceChanges item or H increment is produced, while the conflict-record resolution remains a real portable control effect. For `source_merge`, versionBasis proves the exact base/all-head lineage; a source-state admission occurs when the proposed exact merged bytes/value differ from before. A genuinely byte-identical merge that selects no different historical production version is source-unchanged and follows the same no-/3 rule.

The winning planning CAS stores the complete before, versionBasis, branch/base/head/proposed pins, H basis, after pin, semantic preview and original ConflictResolutionInput/2. A losing/aborted prepare, changed expectedKey, changed installed physical before, wrong selected production version, mismatched arm, missing pin/proof or unproved H basis cannot be edited into a winner and allocates no ChangeId/H. At step 6 all these exact members and the current dependencies are revalidated; after planning, crash/retry resumes only this frozen plan under §5/§8.

At the sole P seal, an actual /3 source admission combines its frozen after SourceStamp with the one decision ChangeId, advances H exactly once, emits exactly one managed SourceVersion/2 and its ordinary revision-token seal artifact, and writes CP4.sourceChanges.before from before.sourceVersion and after from that managed version. The historical chosen head/versionBasis is never substituted for the actual physical before. If installation or seal provenance is unknown, recovery remains paused/recovery_unknown; equal bytes or a later current file never guesses success. Saved recovery republishes/replays only the original sealed bytes; planned recovery never reacquires an ordinary Observation, reselects a head, changes versionBasis/H, or creates a second seal.
### 6.3 ContentCompletionProof/4 and historical /1-/3

Current portable decisions use ContentCompletionProof/4. The committed closed shape is:
~~~json
{"format":"weftext.content-completion","version":4,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/2>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersion/2|"absent">,"after":<managed SourceVersion/2|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~

CP4 retains the complete committed/restored obligations of genuine CP3 while using PortableComponentKey/2 and ChangeRecord/1. Genuine ContentCompletionProof/1-/3 records retain their original decoders, bytes, pins, authorization and recovery. Current CP4 additionally satisfies:

- decisionKey.workspaceRef matches every component/sourceChanges Workspace and changeId.commitDomain=decisionKey.commitDomain. receiptDigest verifies original receipt bytes for the same decision only and grants no receipt read, approval/Money consumption, or execution takeover.
- components is exactly the corresponding InstallationNotice/3 key set in the original fixed-rank/canonical-key order and every after is the actually installed/sealed after. The proof never adds an undeclared component and digest equality never replaces validation of component bytes/owner version.
- sourceChanges may be empty; otherwise it is complete EntityRef-canonical sorted/unique and exactly covers source-state changes made by this decision. A source-unchanged structure/lifecycle portable effect never emits a fake sourceChanges item.
- A non-`"absent"` before is the complete production SourceVersion/2 inside the original plan before SourceObservation/1, the D3-only /2 plan ConflictInstallInput/1, or the D6 source-conflict /3 plan SourceConflictBefore/1. In every case CP4 records the actual installed/observed physical before, never merely the selected historical branch version. Ordinary observation may be managed or external; the wrapper is proved managed and its production commitDomain may differ from this decision domain. before=`"absent"` is only the original plan's proved fresh branch.
- A non-`"absent"` after is managed SourceVersion/2 with entityRef equal to sourceChanges.entityRef, commitDomain=decisionKey.commitDomain, changeId byte-equal to proof.changeId, revision/observationEpoch byte-equal to that entity's actually decoded SourceRevisionPlan/1, D3 conflict-only /2 or D6 source-conflict /3 after SourceStamp, and predecessor production history equal to the validated lastIssued/empty-history basis. One SourceRevisionPlan produces exactly one such sealed after.
- External→managed admission, including equal-byte admission, uses external before+managed after and never carries externalSequence into after. Source deletion uses production SourceVersion before+after=`"absent"` and has no SourceRevisionPlan after, managed SourceVersion, or H increment, while still being the actual source deletion of this portable decision identified by proof.changeId. Raw no-op creates no /3 portable source change.
- observed_only proves only durable installation/seal of original read-before B and input N and never proves absence of an unseen C after the final observation. The proof never fabricates C or rewrites original after from later current bytes.

frontierBefore is the actually validated Frontier immediately before seal. frontierAfter is exactly frontierBefore plus proof.changeId with no regression of any other domain head. InstallationNotice/3.baseFrontier and InputDescriptor.expectedFrontier retain the original plan baseline and are never rewritten by the proof.

With frontierPolicy=`exact`, frontierBefore is byte-equal to original expectedFrontier, DependencyProof.baseFrontier, and notice.baseFrontier.

With frontierPolicy=`scope_dependencies`, frontierBefore may be a non-regressing extension of notice.baseFrontier, but producer seal and /3 generation require all of:
1. every head added from notice.baseFrontier to frontierBefore has a complete, continuous, verified portable ChangeRecord/corresponding completion-proof chain with no hole, fabricated head, or sequence-number jump;
2. every source/control/authorization/positive-negative range entry of the original frozen DependencyProof/3 remains valid under the same original plan and proves the added sealed effects unrelated to those bound keys; a real SourceObservation, FileObjectBinding/pin, Registry/rule, authorization, membership/negative-range, or other dependency change still invalidates the plan;
3. validation of the unrelated extension is durably retained with the original plan, original notice, and actual seal Frontier in P recovery state. Publication never reconstructs “unrelated” after P loss by comparing two Frontier numbers.

A receiver admitting /3 validates portable trust, decision/changeId relation, notice/proof/components, production SourceVersions, and the complete continuous sealed-record chain from notice.baseFrontier through frontierBefore to frontierAfter. Larger vector sequence numbers alone never substitute for missing intermediate causal records. For every non-absent managed sourceChanges.after, it requires exactly one separately carried RevisionTokenSealArtifact/1 for key {proof.changeId,entityRef}. After the normal disclosure/portable-continuity gates, Core sets C=proof.frontierBefore and calls validate_historical for artifact.trustKeyId and association.decisionKey.commitDomain at C, except for the exact same-P bootstrap-genesis rule in §10.2. It recomputes trustKeyId from the returned raw public key, verifies the Ed25519 signature over the exact domain-separated signed body, strict-decodes the association, and requires association.decisionKey=proof.decisionKey, association.changeId=proof.changeId, association.sourceVersion byte-equal to sourceChanges.after, and every §1.5.1 SourceStamp/binding equality. Only then is binding.token stored as the canonical address of that production version. A legal forwarder merely relays the original artifact bytes and need not be trusted or re-sign them. Stamp/sourceVersion/digest equality without this signature cannot authenticate t or any same-stamp t2. Missing/malformed/noncanonical/untrusted/signature-invalid evidence is incomplete/unavailable; two non-byte-equal artifacts that both validate from the anchored historical root for the same exact SourceVersion are an integrity contradiction. This changes neither CP4 nor Notice member/component sets. /3 is not a portable copy of private DependencyProof and grants no new complete Query/Action proof. A strong consumer creates a new current SourceObservation and complete local DependencyProof in its own observerDomain; sender SourceVersionRef/sourceToken is never copied. A persistent managed Locator may be newly read-qualified only when its stable token resolves through that verified canonical binding to the same production SourceVersion and the new current Observation.sourceVersion is byte-equal; this does not update old selectors, preparations, Draft/maps or Action evidence.

When every component has been safely restored to before without seal, the /3 restored closed shape is:
~~~json
{"format":"weftext.content-completion","version":4,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/2>,"after":<ComponentImage/1>}...]}
~~~
restored forbids changeId, guarantee/writeProtection/semanticState, frontierBefore/frontierAfter, sourceChanges, receiptDigest, or any success semantics. components must prove complete safe restoration to original before. Never generate it when restoration is unproved.

ContentCompletionProof/2 retains its historical closed definition, original decoder, and original bytes. Historical committed remains:
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"committed","decisionKey":<DecisionKey/2>,"changeId":<ChangeId/1>,"guarantee":"replica_local|managed_atomic","writeProtection":"strict|observed_only","semanticState":<SemanticState/1>,"frontierBefore":<Frontier/2>,"frontierAfter":<Frontier/2>,"components":[{"key":<PortableComponentKey/2>,"after":<ComponentImage/1>}...],"sourceChanges":[{"entityRef":<EntityRef>,"before":<SourceVersionRef/1|"absent">,"after":<SourceVersionRef/1|"absent">}...],"receiptDigest":"sha256:64-lowercase-hex"}
~~~
Historical restored remains:
~~~json
{"format":"weftext.content-completion","version":2,"outcome":"restored","decisionKey":<DecisionKey/2>,"baseFrontier":<Frontier/2>,"components":[{"key":<PortableComponentKey/2>,"after":<ComponentImage/1>}...]}
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

The current A2 D3 wire13 main/Lexicon/Impact candidate defines the native Descriptor3/companion consumption required here and its bounded D3 findings are independently CLOSED. That conclusion does not independently accept this D6 repair, complete the remaining cross-owner coordination, or authorize activation under D6 Main §18. New-execution consumer checks apply only to unseen decisions under §5; saved/planned/unknown decisions retain their original recovery split. Fresh-current D3 consumes the `d3_identity_operation/13` OwnerInputBinding; a genuine historical wire12 decision keeps its recorded owner descriptor. D3 must genuinely consume the complete ordinary SourceObservation/1 or §3.1.1 guarded ConflictInstallInput/1 under its sole resolution purpose, and DependencyProof/3. A managed after uses the same original plan's explicitly admitted SourceRevisionPlan/1 or conflict-only /2 and d6_source_revision/2 binding. The sealed production SourceVersion/2, SourceVersionRef/1, ContentCompletionProof/4, and recovery split must also agree with this Control file's sole producer definitions. D6 never copies those objects into the companion to bypass D3 ownership and never rewrites genuine historical D3 wire9–12 bytes, Locator revision-token lexical ownership, the D4 inner-selector wire, or D3's actual write scope.

Historical D3 primary receipts, companions, and saved/planned/unknown decisions continue under their original versions, bytes, decoders, pin/authorization/continuity, and recovery obligations. New P1 types never upgrade, re-encode, or grant them a new strong qualification. Managed success depending on these new producers is not partially activated before the remaining P2 coordination, later D7/D10 consumers, and complete fresh joint acceptance are finished. Conversely, an approved ordinary/local operation that does not depend on the missing strong consumer continues under its original owner qualification and is not permanently disabled merely because this companion coordination is incomplete.
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
- when a fresh-current portablePublicationState is pending, publish only the same original sealed decision's retained Notice3/ContentCompletionProof/4/ChangeRecord1 and matching outbox/component bytes, using the original retained scope_dependencies-extension evidence. Never reinstall N, change OperationId, allocate ChangeId again, advance H, recalculate source version, or charge again. Genuine predecessor decisions recover only their **recorded** Notice/CP/outbox family with original decoder/bytes/pins and never auto-upgrade to Notice3/CP4.
- publication/outbox/Derived Index failure resumes only that owner's derived/transport work; it never rolls back the author decision and never reconstructs P approval/Money/unknown responsibility from I.
- a current r6 SourceObservation, DependencyProof, or complete proof proves r6 only. It never retrospectively upgrades, rejects, or re-encodes an historical r5 receipt/saved bytes. Undo/restore remains a new explicit plan rather than receipt replay rolling current backward.

The saved/planned/unseen business split in §5 is the only submit order. §8 only recovers an existing record and provides no second commit entry point. Legacy D6/D3/D7/D8 planned/saved/unknown records continue under their original versions, pins, authorization/continuity, and decoders. Fresh-current SourceRevisionPlan and Notice3/ContentCompletionProof/4/ChangeRecord1 apply only to the explicit current family and never mechanically migrate historical records; genuine CP1/2/3 and Notice1/2 remain version-qualified recovery.

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

The current `source_merge` and `choose_source_head` arms retain the closed wireVersion=3 request below and their ConflictResolutionInput/2 / Plan1 / Preview1 source contracts. Fresh current `policy_bundle_choice` is normatively closed at the later current trust-conflict anchor with Input3/Plan2/Preview2, FreshDomainAuthorizationSpec/2, mixed Bundle1/2 and Carry1/2, Declaration2/Outcome2, dual seal profiles, and Notice3/CP4/ChangeRecord1. A genuine saved/planned/unknown policy predecessor continues only under its recorded Input2/Plan1/Preview1, FreshDomainAuthorizationSpec/1, Bundle1/Carry1/Declaration1 bytes, pins, authorization and recovery.

The two current source arms use the closed wireVersion=3 request:
~~~json
{"wireVersion":3,"kind":"d6_conflict_prepare","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/2>,"budget":<BudgetBinding>}
~~~

`ConflictResolution/2` remains the closed union:
~~~text
{"kind":"source_merge","ownerNodeRef":NodeRef,"source":text}
{"kind":"choose_source_head","ownerNodeRef":NodeRef,"head":ChangeId}
{"kind":"policy_bundle_choice","selected":WorkspaceAuthorizationBundleAddress/1,"policy":Policy/3,"freshAuthorizations":[FreshDomainAuthorizationSpec/1...]}
~~~
The first two arms retain their source semantics. `WorkspaceAuthorizationBundleAddress/1` remains:
~~~json
{"kind":"d6_workspace_authorization_bundle_address","version":1,"head":<ChangeId>,"authorizationRevision":<Counter>,"trustRevision":<Counter>,"byteLength":<Counter>,"sha256":"64-lowercase-hex"}
~~~
The address selects exactly one member of `expectedKey.heads`. Core loads that head's verified CP3 policy-component after-image, requires `ComponentImage.version=authorizationRevision`, byteLength/sha256 equality, strict-decodes the complete `WorkspaceAuthorizationBundle/1`, and requires its trustRevision to match. The digest is SHA-256 of the exact canonical stored bundle bytes; it is comparison evidence only and never replaces the selected ChangeId or bytes.

`FreshDomainAuthorizationSpec/1` remains exactly `{"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1"}`. The array may be empty and is sorted/unique by D3-CJ/3 bytes. It carries no caller public/private key.

#### Historical policy_bundle_choice predecessor evidence

The following Bundle1/Carry1/Declaration1 policy text applies only to a genuine predecessor record that actually used that schema. Its complete authorization, non-disclosure, root/PoP/signature, positive path, error order, pin retention and saved/planned/unknown recovery remain binding for that record; fresh current policy conflict uses the later Input3/Plan2/Preview2 successor.

For a historical `policy_concurrent` predecessor, `conflict_resolve` plus workspace `policy_admin`, complete subject disclosure, exact `expectedKey`, every head's CP3/component bytes, and continuous portable-history proof are required before branch contents are read. Every head bundle must have the same Workspace and byte-equal anchored `WorkspaceTrustRootFingerprint/1`; a missing/corrupt head, different root, unproved common history, or changed key is unavailable/integrity conflict. `selected` must identify exactly one head/bundle; current host state, arrival order and LWW never choose it.

#### Compromise fact extraction and carry

A compromise fact is about the originally compromised key and its original causal cut, not about the resolver that later carries it. Closed `TrustConflictCarry/1` is:
~~~json
{"kind":"d6_trust_conflict_carry","version":1,"factId":"sha256:64-lowercase-hex","workspaceRef":<WorkspaceRef>,"commitDomain":<CommitDomain/2>,"profile":"d6_revision_token_seal/1","compromisedTrustKeyId":"sha256:64-lowercase-hex","originAction":"revoke|rotate","originDecisionKey":<DecisionKey/2>,"originDeclarationRevision":<Counter>,"originDeclarationDigest":"sha256:64-lowercase-hex","originActivationChangeId":<ChangeId/1>}
~~~
`originDeclarationDigest` is `"sha256:" + lowercase_hex(SHA-256(D3-CJ/3(complete original WorkspaceTrustDeclaration/1 including rootSignature)))`. The origin declaration is always a direct root-signed `mode:"compromise"` declaration: for `action:"revoke"`, `compromisedTrustKeyId=trustKeyId`; for `action:"rotate"`, `compromisedTrustKeyId=replacesTrustKeyId`. `originDecisionKey`, revision and digest name that exact declaration, and `originActivationChangeId` is rederived from its DecisionKey→CP3 activation binding. A later resolver ChangeId is never substituted for this original cut.

`factId` is exactly `"sha256:" + lowercase_hex(SHA-256(ASCII "D6-Trust-Compromise-Fact/1" || NUL || D3-CJ/3({workspaceRef,commitDomain,profile,compromisedTrustKeyId,originAction,originDecisionKey,originDeclarationRevision,originDeclarationDigest,originActivationChangeId})))`. Those fields are the complete fact identity. Two records with the same factId must be byte-equal or the history is integrity-conflicted.

For each validated head, Core starts with every compromise fact already effective at the end of the longest common trust prefix, then folds that head's divergent suffix to compute its effective compromise set. A direct `revoke mode=compromise` contributes its trustKeyId fact. A direct `rotate mode=compromise` contributes its replacesTrustKeyId fact. An already accepted `resolve_conflict` contributes every member of its `inheritedCompromises`; each inherited member is accepted only after Core recomputes factId, loads and validates the named original declaration/root signature, checks its mode/action/key mapping, rederives the exact original activation ChangeId, and validates the carrying resolve_conflict declaration itself. This fold is recursive: a fact carried by one or many earlier resolvers is still the same original fact.

The effective set for the new resolution is the union of every validated head's effective compromise set. Deduplication is by factId only; duplicate byte-equal facts collapse to one, while same factId/non-byte-equal bytes reject. Canonical order is ascending ASCII factId. The selected chain's already-effective factIds are subtracted only from the new declaration's `inheritedCompromises` array to avoid duplicate storage; outcome safety is always evaluated against the full effective union, so selecting a policy-only branch that still says K1 current cannot revive K1 when any legal losing branch proved K1 compromised.

Every original compromise declaration, its DecisionKey→CP3/ChangeRecord activation evidence, every retained resolver declaration that carries the fact, and every descendant declaration that still references that carried fact remain public-history last references under Storage §PL-IR-01. Recursive carrying never rewrites the original activation cut.

#### Fresh authorization eligibility, outcomes and PoP

Let `affectedDomainProfiles` be the union of: (a) exact domain/profile pairs whose current normalized trust state differs among the validated heads; and (b) exact domain/profile pairs named by at least one fact in the effective compromise union. Every `freshAuthorizations` member must be in affectedDomainProfiles and, on the selected chain, must either be current state `none` or have its selected current key named by an effective compromise fact. Otherwise prepare rejects the unrelated fresh request. A safe selected current key may not be rotated through this resolver. Thus conflict resolution is not a generic add/rotate surface.

If every head has byte-identical trust history and `freshAuthorizations` is empty, the resolution may be policy-only and needs no root private key. Otherwise the resolver requires the anchored root plus a usable `WorkspaceTrustRootKeyHandle/1` and appends exactly one root-signed `WorkspaceTrustDeclaration/1` with `action:"resolve_conflict"` to the selected trust chain:
~~~json
{"kind":"d6_workspace_trust_declaration","version":1,"workspaceRef":<WorkspaceRef>,"revision":<Counter>,"predecessor":<predecessor>,"decisionKey":<DecisionKey/2>,"action":"resolve_conflict","conflictId":<ConflictId>,"resolvedHeads":[<ChangeId>...],"selected":<WorkspaceAuthorizationBundleAddress/1>,"inheritedCompromises":[<TrustConflictCarry/1>...],"outcomes":[<TrustConflictOutcome/1>...],"rootSignature":"<86-ASCII-unpadded-base64url>"}
~~~
`revision=selected.trustRevision+1`; predecessor is the selected chain's exact last-declaration digest; resolvedHeads byte-equals sorted complete `expectedKey.heads`; inheritedCompromises is the canonical factId-sorted effective-union-minus-selected-existing set above. The root signs the ordinary `D6-Workspace-Trust-Declaration/1` body with only rootSignature removed.

`TrustConflictOutcome/1` is sorted/unique by D3-CJ/3 of `(commitDomain,profile)` and is complete for every exact domain/profile whose head states differ, whose effective compromise facts target that domain/profile, or whose eligible fresh authorization is requested:
~~~text
{"commitDomain":CommitDomain/2,"profile":"d6_revision_token_seal/1","state":"keep_current","trustKeyId":"sha256:64-lowercase-hex"}
{"commitDomain":CommitDomain/2,"profile":"d6_revision_token_seal/1","state":"none"}
{"commitDomain":CommitDomain/2,"profile":"d6_revision_token_seal/1","state":"authorize_fresh","trustKeyId":"sha256:64-lowercase-hex","algorithm":"ed25519","publicKey":"<43-ASCII-unpadded-base64url>","possessionSignature":"<86-ASCII-unpadded-base64url>"}
~~~
A selected current key may be `keep_current` only when no fact in the full effective compromise union names that key. If the selected current key is compromised, the result is `none` unless its exact domain/profile is eligible and present in freshAuthorizations, in which case Core generates a fresh protected key and emits `authorize_fresh`. Selected `none` likewise remains none unless that exact pair is eligible and explicitly requested. A fresh outcome never reuses a caller key.

For each `authorize_fresh` outcome, possession uses the existing domain `D6-Domain-Seal-Key-PoP/1` and this exact closed body:
~~~json
{"workspaceRef":<parent resolve_conflict workspaceRef>,"revision":<parent revision>,"predecessor":<parent predecessor>,"decisionKey":<parent decisionKey>,"commitDomain":<outcome commitDomain>,"profile":"d6_revision_token_seal/1","trustKeyId":<outcome trustKeyId>,"algorithm":"ed25519","publicKey":<outcome publicKey>}
~~~
The signature is exactly `ASCII "D6-Domain-Seal-Key-PoP/1" || NUL || D3-CJ/3(body above)`. The four parent fields are fixed before fresh-key generation; the remaining five fields come from that one outcome. `possessionSignature` itself is absent from the PoP body, and parent `rootSignature` is also absent, so there is no signature recursion. Core recomputes trustKeyId from the raw 32-byte public key before signing. Multiple fresh outcomes each sign their own reconstructed body; swapping possessionSignature values between outcomes fails verification. After all outcome PoPs are fixed, the Workspace root signs the complete resolve_conflict declaration with only rootSignature removed. A receiver reconstructs the same PoP body from the admitted parent declaration plus that exact outcome and verifies the same bytes.

`validate_historical` treats every effective compromise fact at its `originActivationChangeId`: artifacts for the compromised key validate only when their seal ChangeId is causally before that original cut; concurrent/later artifacts reject. Historical validation of a pre-resolution branch still follows that branch's anchored bytes. No resolver rewrites losing declaration bytes or moves an old compromise to a new cut.

The proposed policy is complete. If byte-equal to selected.policy, its Policy revision is preserved; otherwise it must be the legal checked successor of selected.policy and pass current policy_admin rules. The resulting bundle is based on the exact selected bundle, checked-increments `authorizationRevision` once, installs that policy, and either preserves trustRevision for a true policy-only resolution or checked-increments it once for the single resolve_conflict declaration. Its policy ComponentImage.version equals the resulting authorizationRevision.

#### Current source-arm preparation and historical policy predecessor

The two current source arms use InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 and the existing final submit; their owner descriptor remains ConflictResolutionInput/2. The policy material retained in this subsection is historical-only. Fresh current policy_bundle_choice uses the later Input3/Plan2/Preview2 owner contract and the same one-ledger/one-submit boundary. OwnerInputBinding/2 has `protocolOwner="D6"`; `ownerKind` and `InputDescriptor.intentKind` are both `d6_conflict_resolution/2`. The complete canonical owner descriptor is closed `ConflictResolutionInput/2`:
~~~json
{"kind":"d6_conflict_resolution_input","version":2,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/2>,"branchEvidence":[<ConflictResolutionBranchEvidence/1>...],"derivedPlan":<ConflictResolutionDerivedPlan/1>}
~~~
`ConflictResolutionBranchEvidence/1` is exactly:
~~~json
{"head":<ChangeId>,"changeRecordPin":<PinRef/2>,"completionProofPin":<PinRef/2>,"policyBundlePin":<PinRef/2|"absent">,"sourcePins":[{"entityRef":<EntityRef>,"sourcePin":<PinRef/2>,"metadataPin":<PinRef/2>}...]}
~~~
branchEvidence is complete for expectedKey.heads, sorted by ChangeId, and every listed pin is the exact protected/canonical evidence actually consumed by the arm; sourcePins are sorted by EntityRef and may be empty. Missing evidence is not represented by omission.

`ConflictResolutionDerivedPlan/1` is the closed arm-matched union:
~~~text
{"kind":"source","ownerNodeRef":NodeRef,"baseSourcePin":PinRef/2,"headSourcePins":[{"head":ChangeId,"sourcePin":PinRef/2}...],"conflictedBefore":<SourceConflictBefore/1>,"versionBasis":<SourceConflictVersionBasis/1>,"proposedSourcePin":PinRef/2,"semanticPreviewPin":PinRef/2}
{"kind":"policy_bundle","selected":WorkspaceAuthorizationBundleAddress/1,"selectedBundlePin":PinRef/2,"effectiveCompromises":[TrustConflictCarry/1...],"inheritedCompromises":[TrustConflictCarry/1...],"outcomes":[TrustConflictOutcome/1...],"resultBundlePin":PinRef/2}
~~~
For source_merge, proposedSourcePin pins the exact request source; for choose_source_head it pins the exact selected head source bytes. Both source arms retain the original base/head source pins, D2/local/complete semantic evidence and preview pin. conflictedBefore is the §3.1.2 actual installed physical before, never a branch-current Observation, and versionBasis is the complete arm-matched production-version basis. A missing physical before, missing branch version, wrong selected version, mismatched arm/DecisionKey/audience/key, or copied guard is unavailable/invalid and never repaired from equal bytes/hash or I. For policy_bundle_choice, the descriptor freezes every head bundle/evidence pin, selected address/bundle, complete effective compromise union, exact inheritedCompromises, outcomes, any fresh public key/PoP bytes, and the exact proposed resulting WorkspaceAuthorizationBundle/1 pin. Protected fresh private-key handles are bound by the same installationPlan but are never serialized into this descriptor.

`OwnerInputBinding/2.canonicalDescriptorBytes` is exactly D3-CJ/3(ConflictResolutionInput/2), and its pinRefs are the sorted unique union of every pin named by branchEvidence/derivedPlan, including conflictedBefore/versionBasis source and metadata pins. `InputDescriptor/3` uses guarantee=`managed_atomic`, frontierPolicy=`exact`, and strict write protection.

For source_merge/choose_source_head, saveProfile=`complete`. **Before any author/source-semantic/D4/D5/D7 range read**, after only closed decode, minimum subject disclosure and the already-authorized conflict/control identity needed to classify the arm, Core selects the smallest existing ObservationScope that is guaranteed by the owner profile to contain the complete required closure. `local_source` is legal only when the predeclared complete owner profile proves that every applicable author dependency is the single ownerNodeRef source and no relation_incidence, query_scan, foreign Field/structure/member range or other cross-object dependency can be read. If any applicable complete obligation may require those wider reads, the least sufficient existing wider profile, normally `workspace_constraints`, is selected up front. The scope is never widened or narrowed after inspecting author values. If the actual owner-required DependencyKey would fall outside the chosen scope, prepare stops with the existing owner_update_required/proof_unavailable path before that out-of-scope read and before planning. No new ObservationScope kind is added. The conflicted ownerNodeRef itself is not put in sourceInputs: its `source` DependencyKey/controlInputs entry is bound only through §3.1.2 before+basis under this exact owner guard. Unrelated non-conflicted sources actually read remain ordinary sourceInputs with real current Observations. Every actual source-arm dependency inside the chosen scope—including conflict_record, authorization, the guarded source key, and any applicable relation_incidence, query_scan, lifecycle, placement, Registry/rule or other member of the fixed fifteen-kind union—has its exact DependencyProof entry and matching controlInputs stamp; saveProfile=complete never permits omission.

For the genuine historical policy_bundle_choice predecessor, dependency/control inputs use exactly the recorded predecessor Proof2/Key2 fourteen-kind family and are never re-encoded. Fresh current policy_bundle_choice instead uses the §20.1 Input3/Proof3/Key3 fifteen-kind family with control_only, empty sourceInputs, actual conflict_record/authorization entries and any other genuinely read Key3. Frontier remains only expectedFrontier + proof.baseFrontier + frontierPolicy, and policy/trust history remains in the owner branch evidence and exact pins; no policy-history or sixteenth key exists.

The immutable owner preview is closed `ConflictResolutionPreview/1`:
~~~json
{"kind":"d6_conflict_resolution_preview","version":1,"conflictId":<ConflictId>,"expectedKey":<ConflictKey/1>,"resolution":<ConflictResolution/2>,"branchEvidenceDigest":"sha256:64-lowercase-hex","derivedPlan":<ConflictResolutionDerivedPlan/1>}
~~~
`branchEvidenceDigest` is exactly `"sha256:" + lowercase_hex(SHA-256(D3-CJ/3(the complete branchEvidence array)))`. PreparedIntent/3.previewBinding binds exactly this preview record; it is not a second request and cannot be edited at commit. The pinDirectory contains all owner/dependency/preview pins required by §3.6.

Successful prepare for either current source arm returns the current PreparedIntent/3 response with strict write protection:
~~~json
{"wireVersion":3,"kind":"d6_prepared_intent","planToken":<Token>,"semanticState":<SemanticState/1>,"writeProtection":"strict","inputRetentionState":"retained"}
~~~
Its planToken is the `d6_plan/3` token for that exact PreparedIntent/3. A genuine historical policy predecessor returns and recovers only its recorded response/version/tag; fresh current policy prepare is defined at §20.1. Final submit is exclusively the existing `d6_commit_request/2`; it carries no resolution/source/policy override. The one planning CAS freezes the complete InputDescriptor/OwnerInputBinding, branch pins, previewBinding, source or policy derived plan, fresh handle associations and resulting bytes. A source arm that changes source state additionally freezes exactly one SourceRevisionPlan/3; a proved source-unchanged source arm freezes none. The one final P seal revalidates expectedKey/current authorization/fence, step-6 conflicted-before/version-basis/H and those exact frozen dependencies, then atomically installs the source or resulting policy component plus the ConflictRecord resolution/supersession effect. There is no one-stage resolver, second submit, ledger, CAS, second CP4, or observable intermediate trust prefix.

Closed-decode and error ordering stay on the existing D6 surfaces: malformed wire3/Resolution2 is invalid_request before state reads; disclosure/authorization remains not_visible; unavailable branch/CP3/history evidence uses the existing state/proof/domain-unavailable boundaries; expectedKey change is conflict_changed. No new error union is introduced. §5 remains authoritative: saved returns/resumes the original saved result before new business checks; planned restores the same PreparedIntent/descriptor/pins/preview/installation state and never reparses a different Resolution2; unseen alone runs this wire3 preparation. Exact replay restores the same retained plan association. A changed resolution/choice/source/expectedKey cannot reuse that planToken and requires a new unseen prepare; if an original decision already exists under its allocated OperationId, §5 saved/planned/unknown recovery runs first and §8 preserves that original responsibility.

Fresh current policy receiver admission uses the mixed §20.1 dispatch: historical heads validate their recorded CP3/Bundle1 evidence, current heads validate Notice3/CP4/ChangeRecord1 plus Bundle2, and the complete selected/losing compromise fold, PoPs, root signatures, result bytes and same-decision conflict transition must all validate. Current source-resolution receiver/publication uses Notice3/CP4/ChangeRecord1 with the same frozen source pins and semantic proof; genuine historical source records keep their recorded CP decoder. Any mismatch is unavailable/integrity conflict, never arrival-order repair.

The former wireVersion=2/ConflictResolution/1 candidate with `policy_choice:{policy}` is not a current new-FA surface and was not activated; no migration shim is invented. Any real historical prepared/saved/planned/unknown record that can actually be proved still recovers only under its original decoder, request fingerprint, pins and obligations. Placement/lifecycle/identity conflicts continue to D3 §10.1's typed resolver and its original single D3 submit/P decision.

source_merge/choose_source_head still reload every head/base/current permission and rerun D2/local/complete gates. Any new head makes expectedKey stale -> conflict_changed; an old click is never reused.
## 10. Policy/3

Policy/3 retains the exact top-level members version,revision,grants with version=3. Policy/1/2 retain their original decoders, bytes, capability/scope semantics, and never auto-upgrade. grants remain subject,effect,scope,capabilities with deny-before-allow and default deny. Every Policy/2 capability remains byte-for-byte available under its original rules; DependencyProof expansion changes none of them.

The Policy/3 no-argument capability extension is closed to:

replica_register | replica_retire | conflict_read | conflict_resolve | execution_custody_admin | structure_state | portable_frontier_state | d10_control_self.

Semantics remain:
- d10_control_self permits only currently authenticated self-management of the finite D10 Workspace control records at workspace scope. It is explicit, default-deny, grants no author/content/resource/deployment administration, and is neither implied by nor implies any other capability.
- structure_state permits observation of portable parent/order, live/Trash structural scope, and structural state required by D3 StructureRange proof. It grants no source, Field, decision, lifecycle write, or author-body read.
- portable_frontier_state permits observation of complete Frontier/2 and its verified-continuous heads only. It proves no payload materialization, Query/Registry completeness, source bytes, decision detail, or execution responsibility.
- replica_register permits exactly the specialized fresh-replica side effect of §13: mint one never-used ReplicaEpoch and, in the same decision, append exactly one `authorize` declaration for that fresh replica CommitDomain and fixed `d6_revision_token_seal/1` profile. It grants no policy_admin, root-key management, general add/rotate/revoke, other-domain trust mutation, source read/write, or execution takeover. replica_retire only marks the exact ReplicaRecord inactive under its existing gate; `authorize_new_sign` then fails the active-domain/fence check for that epoch. Retire does not append a revoke declaration or erase historical authorization. A compromise/loss requiring trust revocation remains a separate root-authorized trust-management decision.
- conflict_read reveals only an authorized ConflictRecord after subject disclosure and never conflict source bytes, resolution, or author write.
- conflict_resolve enters owner-specific resolution prepare only; actual source/policy/D3 writes still need their original permissions.
- execution_custody_admin manages execution-responsibility continuity/takeover only and never expands Money, approvals, claims, or author source write.

These capabilities create no new implication and do not alter the original capability matrix. source_write deny still blocks full-source changes; Field/body deny still constrains corresponding footprints; write never implies read. An ordinary source-save profile still passes the original source/body/Field/node-control matrix, its actual ObservationScope, and every DependencyProof required by that intent. structure_state, portable_frontier_state, conflict, and replica capabilities are never bypasses for content authorization or strong completeness.

Applicable structure_state disclosure succeeds before hidden parent/sibling/Trash members are read; Core never scans a StructureRange and then decides whether the capability was needed. portable_frontier_state may expose complete Frontier/2 but knowledge of Frontier never implies source access, DependencyProof completeness, or D7 complete Query qualification.

In Policy/3, commit_sequence_state remains CommitDomain-scoped metadata read. The request names the complete CommitDomain and exposes only that domainCommitSequence; no global order is invented across offline replicas. Historical Policy/2 consumers retain their original workspace-wide definition on legacy saved/contract paths only.

The authorization DependencyKey/3 principalAudienceToken still comes from trusted principal/session/delegation mapping and binds current Policy/3 version/auth generation plus the original ObservationScope in proof. It never publishes the full grant table or hidden deny members. A Policy change invalidates/advances the related authorization stamp under §3.4, but a Policy revision is not itself a permission token.

observed_only is not a Policy capability. It is selected explicitly by a trusted human only under every §4.1 qualification before planning starts and then freezes. Nothing in this section expands weak-mode eligibility or allows strict, structured, bulk/collection, Action, Automation, server checkpoint, Approval, or Money paths to downgrade to weak protection.
### 10.1 Complete base Policy and narrow metadata producer

The complete Policy/3 shape is {version:3,revision:Counter,grants:[{subject:Token,effect:"allow"|"deny",scope,capabilities}]}. The trusted host/D10 authentication mapping supplies subject; a caller does not declare the current principal. Scope is exactly {kind:"workspace"}, {kind:"subtree",root:NodeRef} or {kind:"ref_set",refs:[EntityRef...]}. refs is nonempty, canonical sorted/unique by complete Ref, and belongs to this Workspace; existence is not required. Capabilities is a nonempty unique array. Field capability is exactly {kind:"field_read"|"field_write",fieldIds:[FieldId...]} with nonempty canonical sorted/unique FieldIds; every other capability is exactly {kind:K}.

The fixed S base K set is workspace_state|entity_state|locator_state|source_read|source_write|body_write|node_control|node_create|resource_read|resource_write|annotation_read|annotation_write|lifecycle|registry_admin|binding_admin|policy_admin|export|repair|audit|source_envelope_state|commit_sequence_state. The eight explicitly listed current extensions above are added only in Policy/3. workspace_state and commit_sequence_state require workspace scope. entity_state, locator_state and source_envelope_state allow workspace or ref_set, never subtree. ref_set accepts only those three state capabilities. Administrative Workspace operations, including d10_control_self, require workspace scope. Invalid scope/capability combinations reject the whole grant; roles expand only to these same grants and have no additional priority.

Ref-state qualification uses current principal, policy and the complete requested Ref before reading existence, lifecycle, parent, locator or index. It applies equally to live, Trash, tombstoned and never-known Ref domains. An exact-ref deny may override workspace allow, without storing a parent/old ACL on the minimal tombstone. Applicable subtree matching is only for content/Field/effect access after state disclosure, never an existence oracle. All matching denies override direct or derived allow; absence of allow denies. A revision or old preview is not authorization.

| Actual footprint | Required capability | Deny and semantic gate |
| --- | --- | --- |
| Proved ordinary body only | body_write or source_write | Applicable body_write/source_write deny blocks it. |
| One Field's value/Entry note/key | That field_write or source_write | Exact Field deny wins; original D4 transformation gates still apply. |
| Ordinary Document title/subtitle | source_write | body_write does not grant it. |
| coreKind/declared Facets affecting Node classification | source_write and node_control | Either deny blocks; whole-source access cannot evade Node control. |
| Parent/order/lifecycle | Original D3 request and its actual node_control/lifecycle permissions | D6 source edit cannot produce these effects. |
| Existing Resource/Annotation payload | resource_write or annotation_write | Not implied by source_write; fresh identity remains D3. |
| Raw difference not completely classifiable | source_write and proof that no applicable fine-grained deny is hidden | Unprovable classification rejects; typed gates remain. |

source_write deny blocks the whole source range even if a Field/body allow exists. Field/body deny applies to the corresponding actual change. Complete source_read still requires authority for every contained fact; an explicit Field read deny blocks disclosure of a source containing it. Write does not imply read. Core may internally preserve unchanged hidden Fields only on an owner_fields path that independently proves its result qualification; this never replaces observation authorization. Resource/Annotation access also checks complete owner scope. Export requires export and every input's read permission. Policy replacement requires current old-policy workspace policy_admin and writes the complete closed Policy/3 inside the current WorkspaceAuthorizationBundle/2 policy component. The same portable transition checked-increments Policy.revision, the authorization generation and bundle.authorizationRevision, preserves the complete trust histories byte-for-byte except for the explicitly planned change, and is sealed/published by the one original P/Notice3/CP4/ChangeRecord1 chain. Explicit self-revocation is valid and takes effect at commit; no permanent hidden administrator exists. D6 sourceVersions receipt disclosure and effect before/after reads retain their original respective state/read gates, while D3 primary receipts keep D3's own disclosure contract.

The same real §1.5 SourceObservation producer may serve a qualified D7 narrow metadata read after the original entity_state/source_envelope_state and required Field/Registry gates. Internal complete source retention and aggregate validation do not themselves require disclosure of all source bytes. The returned SourceVersionRef selects only that actual same-cut observation and confers no source/body/read/write authority. Invalid or unavailable envelope qualification produces no current-source token. Every later use rechecks its real current authorization and observation; no second D7 observation signer or new DependencyKey exists.

### 10.2 Issuer policy and complete Workspace bootstrap

Storage initialization and Workspace creation have separate authority. Host authentication alone grants no issuer control or existing Workspace access. IssuerControlPolicy remains exactly {kind:"d6_issuer_control_policy",wireVersion:1,issuerAuthorityInstanceId,revision,grants,bootstrapProfile}. revision is Counter; each grant is exactly {subject,effect,capabilities}, with issuer-local trusted principal Token, allow/deny, and a nonempty sorted unique array of allocate_workspace|administer_issuer. There is no Workspace/subtree/ref_set scope here. Default deny and deny precedence apply. administer_issuer does not imply allocate_workspace; neither grants source read or post-activation target access.

The first local issuer is created only in an explicitly established genuinely empty store, mapping the current OS identity through the trusted host. Server initialization uses an existing deployment operator's trusted configuration, never the first HTTP caller. One initialization transaction mints issuer AuthorityInstanceId, continuity/fence, revision=1 policy and principal mapping, explicitly granting both revocable issuer capabilities. It requires the complete profile and trusted Registry seed. Any marker, authority, ledger, configuration, damaged remnant or backup excludes empty initialization. Failure stays uninitialized; restore, continue, failover and repair never recreate initial grants.

Issuer management requires current administer_issuer, the complete old revision and proposed policy/profile, trusted authentication/delegation and current fence in one control CAS. A real change checked-increments revision; MAX rejects. The issuer's original management operation key binds complete immutable input/result: same-key replay is exact, changed input conflicts. This is neither a Workspace OperationId ledger nor a global identity registry. The existing issuer-management transport still requires its own closed host/CLI/RPC contract before implementation; D6 Workspace commit and D10 Workspace control do not impersonate it or authorize a free callback. Explicit revocation of the last administrator or allocation grant is valid; reopening or authenticating again never restores grants.

IssuerControlPolicy's nested bootstrapProfile dispatches by explicit wireVersion. Profile4 requires the corresponding D1 version qualification; the issuer top-level member set is unchanged. Historical profile/1-/3 and any genuinely recorded plan family retain exact bytes, capabilities, pins and recovery; decoder presence alone is not deployment evidence.

~~~text
WorkspaceTrustGenesis/2 = {
  kind:"d6_workspace_trust_genesis",version:2,
  rootDeclaration:WorkspaceTrustRootDeclaration/1,
  initialDomainDeclarations:[
    WorkspaceTrustDeclaration/2,
    WorkspaceTrustDeclaration/2
  ]
}

WorkspaceBootstrapProfile/4 = {
  kind:"d6_bootstrap_profile",wireVersion:4,
  profileRevision:Counter,registrySeedBinding:RegistryBinding/1,
  newSeriesMultiplicity:"unique"|"many",
  initialPeriodScope:"workspace"
}

WorkspaceBootstrapCreatorBinding/1 = {
  issuerPrincipal:Token,targetPrincipal:Token,
  principalAudienceToken:Token
}

WorkspaceBootstrapTargetRegistry/1 = {
  snapshot:RegistrySnapshot/1,binding:RegistryBinding/1
}

WorkspaceBootstrapSeriesConfiguration/1 = {
  seriesScope:SeriesScope,multiplicity:"unique"|"many",revision:1
}

WorkspaceBootstrapPeriodScopeBinding/1 = {
  nodeRef:NodeRef,scope:CalendarScope,revision:1
}

WorkspaceBootstrapPlan/4 = {
  kind:"d6_workspace_bootstrap_plan",wireVersion:4,
  operationId:Uuid,proposalId:Uuid,
  issuerAuthorityInstanceId:Uuid,targetWorkspaceRef:WorkspaceRef,
  targetAuthorityInstanceId:Uuid,profile:WorkspaceBootstrapProfile/4,
  creatorBinding:WorkspaceBootstrapCreatorBinding/1,
  targetRegistry:WorkspaceBootstrapTargetRegistry/1,
  initialPolicy:Policy/3,trustGenesis:WorkspaceTrustGenesis/2,
  initialPresentationPolicy:D8PresentationPolicyBootstrapInit/1,
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
~~~

All UUID members keep the original D3 canonical lowercase UUID decoder. This remains the D6-owned protected control portion of the one original D3 plan, not another request or decision. The admitted host secure store generates a fresh Workspace root keypair and a fresh target-server domain keypair; only public declarations plus protected handle associations enter the plan. `rootDeclaration.establishmentDecisionKey` and `initialDomainDeclaration.decisionKey` both equal the original create/fork DecisionKey. The initial declaration is revision=1/action=authorize for exact target server CommitDomain W/B and profile d6_revision_token_seal/1. Root self-signature, root signature and domain-key PoP must pass before the plan wins.

The current policy after-image is WorkspaceAuthorizationBundle/2 with authorizationRevision=1 and trustRevision=2. Genesis2 contains exactly two Declaration2 authorizations in fixed profile order, revision-token then source-transform, under the same DecisionKey and eventual activation ChangeId; declaration 2 hashes exact declaration 1 as predecessor. The one bootstrap P atomically commits Bundle2, mandatory initialPresentationPolicy, target activation/custody and all bootstrap components through Notice3/CP4/ChangeRecord1. Both trust profiles and the D8 revision1/separate parents=[] state become current only in that same P; no half-bootstrap, second admin request or second CAS exists.

Current Profile4 produces Plan4 only for unseen fresh create/fork. Genuine historical Plan1/Plan3 records, if proven, retain exact decoder, bytes, pins, trust family and saved/planned/unknown recovery; no migration, relabel, dual write or injected Genesis2 is asserted. Plan4 always carries mandatory initialPresentationPolicy frozen by the original proposal/custody cut.

Create builds target Registry by D4's one-time registry_bootstrap from the frozen seed, with real target Workspace/user-owner authentication. Seed creation/update requires a complete bootstrap-eligible seed without retirement/migration history. Fork instead retains the complete source Registry cumulative history, retirements/migrations and required RegistryEvolutionProof, reauthenticates target owners and binds a separate target Registry; it is not a history-free bootstrap seed. Both source and target bindings/inputs remain retained. Unportable/unverifiable definitions, ownership, rules or history reject before planning; no network install, FieldId rewrite or history deletion repairs them.

SeriesScope is the complete §3.4 series/scope/policyBinding. Configurations are unique and sorted by canonical SeriesScope; periodScopeBindings are unique/sorted by full NodeRef with exactly one per valid prepared period. Series/periodKey are read from complete author Entries, never copied into an editable control fact. The binding stores only the explicit scope choice. Create uses the family's explicit workspace scope for each prepared period and its frozen multiplicity for each distinct series. This is bootstrap initialization, not a default for ordinary missing configuration. Fork carries all source configurations/bindings at the fixed cut, preserves unique/many, maps Node subjects and node scopes through the complete Node map, maps workspace scope to target WorkspaceId, and verifies target Registry/policy. Missing or foreign/unmappable scope fails the whole fork before planning; no omitted configuration or second management commit repairs it.

Ordinary new periods explicitly choose workspace scope in their original plan and require an already configured series/scope; source edits retain the old binding and check both old/new ranges when series/key changes. Removing a period removes its binding in the same plan; Trash retains it. Explicit scope changes require current workspace policy_admin and the original complete-range management gates. Node scopes require proved live/Trash Node state, or the exact D3 prepared-origin for a fresh mapped scope. Control inbound references prevent purge until explicitly detached; purge may remove its own purged periods' bindings, but never implicitly deletes configuration or guesses a new scope. Managed copy preserves source scope, maps fresh node scopes, may retain legitimate same-Workspace unmapped scopes and rejects cross-Workspace unmappable scopes. Only a mapped fresh scope may initialize its exact source-cut configuration in that same copy plan with preserved multiplicity and fresh control generation/revision=1. It proves negative inventory/config range, target policy and the entire prepared period closure; no implicit workspace/existing-scope configuration is added. Ordinary artifacts follow the ordinary new-period rule. Complete fork follows the full rule above.

An empty target database is no proof that the final range is empty. D4 uniqueness checks the complete proposal result and all scopes at the same target negative-inventory/custody/allocation cut; unique groups by the complete period key and many permits multiple Nodes. The single original author seal publishes policy, Registry, all configurations/bindings, sources, authority activation, receipt/custody and invalidation together. Fresh source revision is 1 in the proved empty production history; control initialization does not increment it again. No half-active target or later administrator/configuration repair window exists.

Ordinary policy/config management applies only after activation. Bootstrap is exclusively the deterministic original create/fork branch. Claim/replacement race, rejection, burn, planned/terminal recovery and original D3 receipt remain unchanged. Lost profile/Registry derivation evidence preserves original availability/recovery, never latest-seed selection. Committed create/fork replay uses D3's current issuer/source/proposal gates and returns original receipt bytes without reinitializing user-edited policy or requiring new target permission for that original limited receipt. Extra target control/source/effect disclosure still requires current target rights. Continue/failover preserves current policy, principal mappings and configurations and never revives creator rights. Fork grants no source authority.

## 11. BudgetBinding/1 and pin capacity

BudgetBinding/1 members/numeric domain remain. PreparedIntent/3 also binds PinBudget/1:

~~~json
{"version":1,"maxRecoveryBytes":<Counter>,"maxConflictBytes":<Counter>,"maxPreviewBytes":<Counter>,"maxImportExportBytes":<Counter>,"maxHistoryBytes":<Counter>}
~~~

0 means no new allocation of that class. Effective limits are request/policy/host minima. Pin allocation reserves with checked add before allocation. All attempts share plan counters. Work units remain durably charged before execution and crash does not refund. Temporary staging and protected pins are separately accounted.

A protected last-reference pin is never deleted by TTL/preview expiry. Capacity shortage produces budget_exceeded or planned paused_capacity; it never frees planned/unknown/conflict last-reference evidence.

## 12. Result/ByteHandle and index consumption

The existing closed wire1 ResultHandle/ResultCursor and Resource ByteHandle/ByteRead shapes, token/tag semantics, saved-handle bytes, authorization order, budgets, TTL, pins, and error priority remain under their original contracts. This section does not change those wire1 shapes and does not treat the mere existence of a historical decoder as evidence that every old prototype is an active current compatibility surface. A saved handle that still satisfies its original authorization, pin, clock, and continuity obligations continues under its original bytes. A legitimately expired, invalidated, or collected record is never revived from current files, equal digest, or a new producer proof.

In the new file-backed producer path defined in this contract, any future consumer version that treats a result or snapshot as current, complete, or Action-eligible binds its protected immutable pin/record to the complete SourceObservation/1 actually read, including the complete production SourceVersion/2, production CommitDomain, and current observerDomain/observationEpoch/FileObjectBinding/evidence pins. Equal bare revision numbers in different production domains never identify the same current source. A result that depends on range/control completeness also binds the actual closed DependencyKey/3 entries, their DependencyProof/3 epoch/revision and evidence pins, and the positive/negative ranges, authorization, Registry/control, and other real dependencies proved by their owners. SourceVersionRef/1 only selects the complete current Observation. SourceRevisionPlan/1, a proposed SourceStamp/1, or an unsealed revision token never substitutes for committed current-source evidence.

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

displayLabel is authorized display text only, non-empty and at most 256 UTF-8 bytes. Requires workspace-scope replica_register and a verifiable current WorkspaceAuthorizationBundle/Registry; it never takes over authority. PreparedIntent/3 mints one never-used ReplicaEpoch. Through the trusted pairing channel Core generates exactly two fresh host-protected DomainSealKeyHandle/2 values for that replica CommitDomain, in fixed profile order revision-token then source-transform. The winning plan freezes both staged handles, the predecessor trust state and exactly two Declaration2 authorizations under one DecisionKey. The one final P writes active ReplicaRecord and Bundle2/policy after-images through one Notice3/CP4/ChangeRecord1 transition and ChangeId. Both handles become usable together only after complete receiver admission; no observable one-profile prefix or generic trust-admin substitute is legal. A proven historical single-profile registration retains its Handle1/Declaration1/CP3 decoder, bytes, pins and recovery.

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

code is closed to invalid_request | unsupported_version | not_visible | domain_unavailable | integrity_conflict | operation_id_conflict | plan_expired | source_unavailable | proof_unavailable | dependency_conflict | semantic_rejected | budget_exceeded | install_unavailable | conflict | conflict_changed | state_unavailable | owner_update_required | effects_unavailable | transaction_aborted | approval_unavailable | execution_stopped.

Rules:

- invalid_request/unsupported_version/not_visible/domain_unavailable/integrity_conflict/operation_id_conflict/plan_expired/source_unavailable/proof_unavailable and install_unavailable known before planning are preflight only;
- dependency_conflict/semantic_rejected/budget_exceeded at semantic step 6 may be recorded only when they are deterministic business outcomes of the canonical intent;
- install-time competition, revocation, capacity or recovery uncertainty is paused and never writes rejected/terminal;
- conflict_changed and owner_update_required are preflight and create no decision;
- after planned, transaction_aborted+terminal is allowed only when it is proved that the original plan can never commit and every installation remnant has been safely resolved;
- approval_unavailable and execution_stopped are preflight under §4.4; saved replay precedes them. A planned record keeps its original plan, installed state and reservations. Only the original proved terminal-abort conditions may release its count; an error disposition never itself proves rollback.
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

## 16. Execution responsibility historical recovery and current control inspection

Fresh/current execution custody uses ExecutionResponsibilityRecord/3 + D10ExecutionInventory/2 + ExecutionContinuityProof/2 as closed in SCHEMAS §10 and the later current mixed-holder clauses. Record3 and its pinned Inventory2 are captured at one real Authority Store barrier, agree byte-for-byte on the five responsibility payloads, and bind the same StoreIncarnation and StopCapacity. Money, ApprovalUse, external-unknown, claims and stop responsibilities remain with their real owners; backup/sync never creates execution authority. A handoff fences the old holder before enabling the new holder and creates no second ledger.

### 16.1 Genuine historical Record2 / Inventory1 / Proof1 recovery

For a genuine historical record only, ExecutionResponsibilityRecord/2 remains protected P control, never portable metadata. Its exact semantic members were:

kind, version, workspaceRef, executionDomainId, holder, revision, status, approvalUses, claims, moneyLineage, externalUnknowns, stopState, lastContinuityProof.

kind=d6_execution_responsibility, version=2. executionDomainId is a Core-minted UUIDv4 independent of CommitDomain/ReplicaEpoch. holder is local_replica{replicaEpoch} or server{authorityInstanceId,deploymentId}; deploymentId is a protected host UUID, not author identity. revision increments checked.

The payloads are now closed by the actual D10 Control Contract §8.2/§16.1: approvalUses is ApprovalUse/1[], claims is D10ExecutionClaims/1, moneyLineage is D10MoneyResponsibility/1, externalUnknowns is D10ExternalResponsibility/1[], and stopState is D10StopResponsibility/1[]. status is active|paused|transferring. No free JSON or “later” schema remains. The payload is a protected complete view of the original owner records at one execution-control cut, not an independently writable second account/claim/approval ledger. Every change to those original responsibilities invalidates the old view and checked-advances this record's revision in the same actual transaction; a snapshot cannot remain current after an omitted new claim, send or liability. An unavailable complete view pauses execution only, not ordinary source access.

    ExecutionContinuityProof/1 =
        {kind:"initial", storeIncarnation:Uuid,
         inventoryPin:PinRef/2, birthProofToken:Token}
      | {kind:"checkpoint", storeIncarnation:Uuid,
         inventoryPin:PinRef/2, barrierToken:Token}
      | {kind:"handoff", storeIncarnation:Uuid,
         inventoryPin:PinRef/2, fromHolder:ExecutionHolder,
         toHolder:ExecutionHolder, oldRevision:Counter,
         barrierToken:Token, oldHolderFenceToken:Token}

ExecutionHolder is exactly the holder union above. For this genuine Record2 family, lastContinuityProof is the historical `ExecutionContinuityProof/1` carrier. inventoryPin is artifact/recovery and strict-decodes D10ExecutionInventory/1, with D3-CJ/3 canonical bytes prefixed by UTF-8 D6-Execution-Inventory/1 and NUL. Its complete Workspace and five payload members equal this record; its stopCapacity retains the actual safety allocation. It includes exact original requests, prepared/history record images, meaningful empty-range proofs, Run admission, all subscription/occurrence states, count uses, all Money layer projections and original external send/unknown responsibility. Shared deployment accounts remain at their actual owner and are referenced continuously, never transferred as an invented zero balance.

The three tokens are protected Core handles, not caller assertions, interchangeable capability tokens or unexplained evidence blobs. birthProofToken binds this never-used executionDomainId, Workspace, storeIncarnation, first holder and a complete proven empty inventory at the actual store's original creation barrier. An existing or damaged store/backup cannot mint a new birth. barrierToken binds the exact executionDomainId, holder, record revision and full inventory pin to a real durable store barrier after freezing new admission/planning/send/claim work and retaining in-flight installation responsibility. oldHolderFenceToken additionally binds that same barrier/inventory and the exact from/to holder pair to the backend's proved irreversible exclusion of old execution. Core validates the actual trusted backup/handoff and fencing mechanism under its admitted backend contract before minting either token; unsupported or unproved exclusion is unavailable, not a signed guess. The handles' protected mappings and required original evidence survive recovery and are not reconstructed from I or portable files. A digest, provider “synced” flag or file copy never supplies one.

Checkpoint verifies complete positive/negative record/range membership under that barrier. Handoff requires current execution_custody_admin, full original inventory/pins, preserved storeIncarnation and ControlRefs, exact old revision/holder, durable transfer, and actual old-holder fencing before enabling the new holder. New record revision is checked old+1; transfer never resets quotas, identities, occurrences, clock evidence or unknowns. During transfer status remains transferring/paused and neither holder may start new work before its exact qualification is proven. A failed handoff retains the old responsibilities and cannot create an empty replacement domain. Initial/checkpoint/handoff pins obey last-reference retention. Historical responsibility records continue through their actual original decoder and cannot gain missing continuity from this new schema.

Execution takeover requires execution_custody_admin, the full old responsibility record, protected backup/remote handoff proof, and proof that the old holder is fenced. Failure is domain_unavailable/semantic_rejected and never creates a blank responsibility record. Takeover never copies or resets quota. sourceOccurrenceKey continuity requires complete D10 occurrence evidence; same path/hash/Field key is insufficient.

### 16.2 Current control inspection

Control inspection v2 adds targets decision_state (complete original request required), current_source (complete EntityRef), conflict (ConflictId), replica (ReplicaEpoch), existing execution_resource, and execution_responsibility (executionDomainId, restricted to custody admin/audit). There is no generic “list every OperationId/unknown in Workspace” API.

## 17. Current fixed97 direct-owner contract

The following retained contracts remain current unless a named successor below replaces only their versioned carrier. Each import names source/blob, exact section and current constructor/validator; it is not a second authority.

### 17.1 ResultPage / cursor transport

Import fixed-S D6 Control `docs/design/snapshots/d6-control-interfaces/source.md`, blob `32a8a863d1a86bb3f3dbaedc4e4eebf005c3f8a4`, §5 lines 98–107. Retain exact page request `wireVersion,kind,resultToken,cursorToken,pageSize` with version1/pageSize 1..200; exact reply `wireVersion,kind,resultToken,rows,terminal,nextCursorToken?`; complete result before paging; terminal=true forbids a next cursor and terminal=false requires one even for rows=[]; current authorization precedes expiry/reset/cursor/state/read. Empty nonterminal is legal and transport failure is never EOF. Constructor/validator: D6 result-spool publisher and strict result page/error decoders; D7 owns row/result semantics.

### 17.2 BudgetBinding/1

Import the same source/blob §6 lines 108–117. BudgetBinding/1 is exactly `version,maxInputBytes,maxSourceBytes,maxWriteBytes,maxDecodedBytes,maxWorkUnits,maxDependencies,maxNewEntities,maxTemporaryBytes,maxElapsedMillis,maxResultBytes,maxOutputBytes`; version1, all limits Counter, zero denies rather than means unlimited. Pointwise request/Workspace/host minimum, checked cumulative charging, burned work, finite attempt allowance and no truncating success remain complete. Constructor/validator: D6 budget binder and execution-resource CAS; PreparedIntent/3 references this unchanged inner type.

### 17.3 ImportJob / batch recovery

Import the same source/blob §7 lines 118–127. Retain authenticated owner, complete input descriptor/pins, mapping version, finite DAG/SCC and atomic groups, explicit batch order, stable per-batch OperationId, exact protocolOwner/canonical request/full plan, committed receipts, binding/version/watermark prefix, state and budget. Only committed complete-predecessor batches advance the prefix; retry cannot double-advance; changed mapping/input/groups create a new job. D9 owns format/mapping/comparator input, D3 or D6 owns the admitted author request/plan, and D6 atomically persists receipt/binding/watermark.

### 17.4 Managed configuration / control read

Import the same source/blob §§8–10 lines 128–173. Retain all four closed management intents, policy_admin/ObservationScope gates, one D6 prepare/commit ledger, CAS/replay/error order and closed d6_control_read target/state/error shapes. Current outer carriers are InputDescriptor/3, DependencyProof/3 and PreparedIntent/3; inner management contracts are not mechanically bumped.

### 17.5 Resource ByteHandle / ByteRead

Import the same source/blob §11 lines 174–189. d6_byte_read_request stays version1 with handleToken, offset and maxBytes 1..1048576. Success binds the immutable Resource cut and returns exact resourceRef, resourceRevisionToken, offset, totalBytes, unpadded base64url data and Boolean terminal; decoded length is min(maxBytes,totalBytes-offset). offset==totalBytes permits empty terminal success; offset>totalBytes is range_invalid; short read/corruption/I/O is never fake EOF. Gate order remains decode -> audience/current resource authorization -> expiry -> reset -> range -> pin/backend/budget -> complete chunk verification -> final delivery gate. All reads share the handle's persistent budget. D2 Resource snapshot issues the handle; D6 only transports that committed snapshot.

### 17.6 Cross-stage inherited interfaces / ObservationScope

Import the same source/blob §§12–13 lines 190–247 for their still-active **business invariants**, not for predecessor version numbers. The source line-192 numeric SourceVersion/revision projection is predecessor evidence: fresh current production/currentness uses §1.5 SourceVersion/2 + SourceObservation/1 and the §1.5.1 RevisionTokenBinding/2 signed stable-address association; the retained rule is that a bare numeric revision never compares across Ref/production domain and an old D3 opaque revision-token decoder is never silently widened. Genuine historical records keep their recorded token/version semantics.

Source lines 194–200 retain owner boundaries exactly: D3 owns cut/custody tokens; D4 owns RelationReadContext/2, RelationReadBinding/2 and RecurrenceReadBinding/1 inner shapes; D7 owns row/result semantics; D6 supplies only the version-qualified protected evidence and transport. Source lines 202–247 retain authorization-before-secret-read, no hidden-range probing, saved/planned delivery qualification, and complete-proof-versus-disclosure separation. Their predecessor ObservationScope/1 four-profile carrier is **not** current. Fresh current uses §3.3 ObservationScope/2 with exactly `local_source|owner_fields|local_structure|workspace_constraints|control_only|prepared_workspace`, Core-derived before author/range reads and stored in InputDescriptor/3. A genuine historical Scope1/wire11 or wire12 decision retains its exact recorded bytes; fresh D3 uses wire13/Descriptor3. No caller scope, free JSON, index miss, current empty range, or policy_admin alone creates broader observation authority.

### 17.7 SourceBinding / OriginBinding physical boundary

Import fixed-S D3 identity source `docs/design/snapshots/d3-identity-references-ownership-and-lifecycle/source.md`, blob `bc23922ae2de4c80dadc7b91e945c87ce154b55a`, SourceBinding/ForeignIdentityKey/OriginBinding clauses and lifecycle matrix. SourceBinding identifies one concrete source instance/collection scope; account token, cursor, etag, watermark and credentials are control-plane facts. OriginBinding permits at most one active ForeignIdentityKey association, preserves retired audit, never auto-restores active_non_live, and requires explicit adopt after retirement. source_deleted, Node Trash, detached binding and provider cancellation are separate axes. D6 owns physical persistence/retention/same-key linearization and durable base-current-remote evidence; D9/D10 or the provider owns external version/comparator and connector/credential execution.

### 17.8 Current successor dispatch

Fresh unseen D6 uses Key3/Proof3/Input3/Prepared3 with d6_plan/3. Portable current publication is PortableComponentKey/2 + Notice3 + one final P + CP4/ChangeRecord1. A managed-Document semantic consumer includes document_format; a format-only transition has sourceChanges=[] and no SourceRevisionPlan, managed SourceVersion or H advance. Genuine saved/planned/unknown Key2/Proof2/Descriptor2/Prepared2, Notice1/2 and CP1/2/3 retain original decoder, bytes, pins, OperationId, authorization, install/seal/receipt/retention/error order and recovery.


## 18. SourceTransform compiler and exact mapping

### 18.1 Total compilation

PortableTransformCompilation/1 is total: representable events + afterSourceSha256 or an unavailable reason from the closed set provenance_gap, unsupported_transaction, invalid_utf8_boundary, generated_cross_anchor_edit, boundary_slot_unrepresentable, provenance_cycle, after_replay_mismatch, payload_digest_mismatch.

```text
PortableTransformCompilation/1 =
  representable{events:[SourceTransformPortableEvent/3...],afterSourceSha256}
| unavailable{reason:
    provenance_gap|unsupported_transaction|invalid_utf8_boundary|
    generated_cross_anchor_edit|boundary_slot_unrepresentable|
    provenance_cycle|after_replay_mismatch|payload_digest_mismatch}
``` An unavailable transform does not by itself block ordinary source save.

SourceTransformPortableEvent/3 contains only replace and insert, all in original-before UTF-8 byte coordinates. Deletes are replacements whose replacementByteLength is zero. Every replace satisfies 0 <= startByte < endByte <= beforeByteLength, and startByte/endByte are UTF-8 scalar boundaries. removedByteLength equals endByte-startByte and removedSha256 equals SHA-256 of exactly beforeBytes[startByte:endByte]. Every insert satisfies 0 <= atByte <= beforeByteLength and atByte is a UTF-8 scalar boundary. A zero-length insert is non-canonical and is omitted. Generated-edit provenance folds into the originating event where representable; overlapping source replacements form one maximal replacement island; an insert strictly inside such an island is folded into that island or compilation is unavailable. Same-point inserts are merged in transaction order, so at most one insert remains at a point. Boundary insertions remain separate.

The canonical event array is ordered by original coordinate. A replacement whose interval ends at p appears before the event at p; at the same starting coordinate p, the unique insert@p appears before the replacement starting at p. Thus the boundary order is left replacement ending at p, unique insert at p, right replacement starting at p. Replacement source intervals are pairwise non-overlapping after maximal-island formation.

`generatedOutputSpan(E)` is computed without content search by one replay cursor over the exact before and after pins. Initialize beforeCursor=0 and afterCursor=0. For each canonical event E let q be E.startByte for replace or E.atByte for insert; require q>=beforeCursor, require the unchanged bytes beforeBytes[beforeCursor:q] to equal afterBytes[afterCursor:afterCursor+(q-beforeCursor)], then advance both cursors by that unchanged length. E's generatedOutputSpan is [afterCursor, afterCursor+E.replacementByteLength); it must be within afterBytes, its exact slice must match replacementByteLength and replacementSha256, and afterCursor advances to the span end. For replace, beforeCursor then becomes endByte; for insert it remains q. After the last event, the remaining beforeBytes[beforeCursor:] must be byte-equal to the remaining afterBytes[afterCursor:]. The resulting complete after bytes must match afterSourceSha256. Any failed boundary, length, digest, ordering, unchanged-segment, or final replay check makes the compilation/evidence invalid; a receiver never trusts a redundant field instead of the bytes.

### 18.2 Mapping

For replacement R=[a,b) with replacement length Lr, define delta(R)=Lr-(b-a) as a signed integer. For insertion I at q, len(I)=replacementByteLength. All sums below use checked signed arithmetic and the final mapped coordinate must remain within the replayed after-source byte range.

For a non-zero range [s,e), real replacement overlap or an insertion strictly inside the range stops exact mapping. Otherwise:

```text
s' = s + sum(delta(R), b<=s) + sum(len(I), q<=s)
e' = e + sum(delta(R), b<=e) + sum(len(I), q<e)
```

For point p, a replacement satisfying a<=p<b stops mapping; otherwise add deltas ending before/at p, inserts before p, and inserts at p only for right affinity.

```text
p' = p + sum(delta(R), b<=p) + sum(len(I), q<p)
     + (sum(len(I), q=p) when affinity=right else 0)
``` Each link in a transform chain independently revalidates source version, signature, trust cut, and result.

### 18.3 Frozen plan

CoreSourceEditPlan/2 freezes the /3 events, exact before observation, exact after pin, and TransformEmissionPlan/1.

```text
CoreSourceEditPlan/2 = {
  kind:"d6_core_source_edit_plan",version:2,
  decisionKey,ownerNodeRef,beforeObservation,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,
  transformEmission:TransformEmissionPlan/1
}

TransformEmissionPlan/1 =
  disabled{reason:"no_exact_core_edit_plan"|"transform_profile_unavailable"}
| required{profile:"d6_source_transform_seal/1",expectedTrustRevision,expectedTrustKeyId}
```

Emission is either disabled with `no_exact_core_edit_plan|transform_profile_unavailable`, or required with exact profile, expectedTrustRevision, and expectedTrustKeyId. The caller cannot choose. A winning required plan cannot downgrade during recovery; a disabled plan cannot upgrade. Seal cannot recompile or reorder events.

## 19. Signed transform evidence and outbox

SourceTransformEvidence/2 binds DecisionKey, ChangeId, owner, managed before/after SourceVersions, before/after hashes, the fixed coordinate/affinity profiles, and the exact /3 event array.

```text
SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",version:2,
  decisionKey,changeId,ownerNodeRef,
  before:managed SourceVersion/2,
  after:managed SourceVersion/2,
  beforeSourceSha256,afterSourceSha256,
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}
```

beforeSourceSha256 is a whole-source cross-field, not a free digest or an Event-slice summary. It is computed from the exact complete source bytes identified by evidence.before:

```text
beforeSourceSha256 =
  "sha256:" + lowercase_hex(SHA-256(exact_before_source_bytes))
```

The exact_before_source_bytes are the complete bytes of ownerNodeRef at the managed SourceVersion/2 in evidence.before, recovered from the producing CP/history and retained version evidence rather than from the receiver's current file. Receiver validation reacquires those exact bytes, recomputes the digest, and requires byte-equality before accepting the signed evidence. removedSha256 values, removedByteLength, equality of SourceVersion fields, or successful replay of individual slices never substitutes for this whole-source check. Missing exact-before bytes follows the existing proof/state-unavailable boundary; a proved digest contradiction is an integrity failure. No SourceTransformEvidence schema/version changes.

A required plan must satisfy byte-equality of D3-CJ/3(plan.edits) and D3-CJ/3(evidence.edits).

```text
D3-CJ/3(plan.edits) == D3-CJ/3(evidence.edits)
```

SourceTransformSealSignedBody/1 is exactly the complete SourceTransformSealArtifact/1 with the single signature member removed; it therefore contains format, version, trustKeyId, and the complete SourceTransformEvidence/2 and no other member. The signature lexical form is exactly 86 ASCII unpadded-base64url characters decoding to 64 Ed25519 signature bytes. The selected verification public key is exactly 43 ASCII unpadded-base64url characters decoding to 32 bytes, and trustKeyId is exactly "sha256:" + lowercase_hex(SHA-256(raw public-key bytes)).

The authenticated message is exactly `ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1)`. The transported/stored artifact bytes are exactly D3-CJ/3(the complete SourceTransformSealArtifact/1 including signature); a non-canonical transport representation is rejected even when it parses to equal JSON values. The signature never covers itself, an outbox address, or a later trust cut.

The outbox key is exactly {changeId,ownerNodeRef} and the outbox item pins exact canonical artifact bytes as portable_metadata. A required sealed decision has exactly one item; disabled has zero; two different items for one key are an integrity conflict. Required seal revalidates the frozen exact profile/trustRevision/trustKeyId and usable handle in the original P seal. Recovery republishes the exact pinned artifact by exact key and never recompiles events, searches by current hash/trust/source, or re-signs.

The transform artifact is not a PortableComponent and is not inserted into Notice/CP components. It shares the source decision's one P seal and ChangeId.

## 20. Trust profile, activation, and historical verification

A Workspace has exactly one retained WorkspaceTrustRootDeclaration/1 and one protected WorkspaceTrustAnchor/1. The closed seal profiles are d6_revision_token_seal/1 and d6_source_transform_seal/1. Historical WorkspaceTrustDeclaration/1 remains byte-exact revision-token-only. WorkspaceTrustDeclaration/2 is the current profile-discriminated successor; WorkspaceAuthorizationBundle/2 may contain a byte-exact /1 prefix followed by a /2 suffix, and the chain never returns to /1 after the first /2.

Declaration2 revision 1 uses predecessor `{kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}` with the complete retained fingerprint object, not a bare digest. That fingerprint is byte-equal to the one recomputed from the retained root declaration and stored in the unique protected WorkspaceTrustAnchor/1. Later predecessors use the exact previous declaration revision and SHA-256 of its D3-CJ/3 bytes. rootKeyId is never accepted in the predecessor fingerprint slot.

Declaration2 publicKey is exactly 43 ASCII unpadded-base64url characters decoding to 32 Ed25519 bytes. Every Ed25519 signature member is exactly 86 ASCII unpadded-base64url characters decoding to 64 bytes. Each trustKeyId/newTrustKeyId equals `"sha256:" + lowercase_hex(SHA-256(raw_32_byte_public_key))`; uppercase hex, padded base64, alternate serialization, or caller-provided key bytes outside the closed prepare protocol are rejected.

For authorize and rotate, possessionSignature authenticates exactly `ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2)`. The PoP body is the closed projection of the containing declaration's workspaceRef, revision, predecessor, decisionKey plus the exact action commitDomain/profile and the new key tuple; rotate normalizes newTrustKeyId into the body's trustKeyId member. An authorize_fresh TrustConflictOutcome/2 uses the same PoP domain/body reconstructed from the enclosing Declaration2 common fields plus that exact outcome's commitDomain/profile/key tuple. A PoP cannot be transplanted to another declaration revision, DecisionKey, domain, profile, or key.

An ordinary rotate uses mode="ordinary" and continuitySignature is the replaced key's Ed25519 signature over exactly `ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2)`. That closed body contains the declaration common fields and the complete rotate action except continuitySignature and rootSignature, including old/new key IDs, publicKey, possessionSignature, and mode="ordinary". For mode loss_recovery or compromise, continuitySignature is the literal `"not_required"`; no old-key signature is accepted or required. Revoke uses the separate closed mode set administrative|loss|compromise.

rootSignature authenticates exactly `ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2)`, where WorkspaceTrustDeclarationSignedBody/2 is the complete Declaration2 with only rootSignature removed. Thus the root binds the full action, PoP/continuity value, conflict outcomes/carries, predecessor, and DecisionKey.

Current unseen ordinary trust administration uses the closed wireVersion3 add/rotate/revoke prepare successors in SCHEMAS §9. Their profile is SealProfileId/1, so both revision-token and source-transform keys have a real public producer. The request never carries public/private key material: Core generates add/rotate keys and PoP inside the admitted secure store. The gate remains current workspace policy_admin, exact expected trust revision, current disclosure, anchored root, and usable root handle. Rotate uses ordinary|loss_recovery|compromise; revoke independently uses administrative|loss|compromise. The operation updates only the existing policy/WorkspaceAuthorizationBundle component through the original planning/install/single-P/CP4 path; it creates no second trust ledger, Boolean validator, CAS, or commit point. Genuine saved/planned wireVersion2 trust requests retain their exact decoder/recovery and are never rewritten to /3.

Declaration1 activation remains under its original DecisionKey -> CP3/public-history rule. Declaration2 activation is rederived through its DecisionKey -> committed CP4 + ChangeRecord/1 -> exact policy after-image that first appends the declaration sequence. Same-DecisionKey declarations share one activation ChangeId and are all-in/all-out in history_at(C). TrustConflictCarry/2 rederives its origin from the original Declaration2 and original CP4/ChangeRecord; a later resolver cut never replaces the origin cut.

A normal SourceTransform receiver validates the producing ChangeRecord/CP4 and uses C=CP4.frontierBefore for historical key verification. Later ordinary rotation does not invalidate an artifact valid at C. Compromise compares causal order between the artifact seal ChangeId and the original compromise activation ChangeId; causal-concurrent or later old-key seals fail regardless of arrival order. DomainSealKeyHandle/1 may continue new revision-token signing only while its exact Declaration1-authorized key remains current, safe, and usable; it never gains transform authority or is reencoded as Handle2.

### 20.1 Dual-profile policy-conflict resolution

The public d6_conflict_prepare request remains the fixed-parent wireVersion3 request and the policy_bundle_choice JSON member names remain selected, policy, and freshAuthorizations. The current unseen policy arm uses FreshDomainAuthorizationSpec/2 from SCHEMAS §9, whose profile is SealProfileId/1. Source-merge and choose-source-head arms remain on their fixed-parent contract. This is not a new submit surface: conflict_resolve, workspace policy_admin, subject disclosure, exact expectedKey, the original error order, the one winning planning CAS, the original d6_commit_request/2, and the one final P remain unchanged. A genuinely saved/planned policy resolution that can be proved to have been stored under an earlier owner descriptor restores that exact request, descriptor, pins, handles and recovery state and is never reparsed as the current successor. This candidate asserts no deployment that would justify a synthetic migration.

Every expectedKey head is validated before branch contents are trusted. A head whose retained completion proof strict-decodes as ContentCompletionProof/3 is a historical CP3 head and its policy after-image must strict-decode WorkspaceAuthorizationBundle/1. A head whose retained completion proof strict-decodes as ContentCompletionProof/4 is a current CP4 head: the exact ChangeRecord/1, Notice3/CP4 linkage, policy ComponentImage bytes/version and continuous chain must validate, after which the policy after-image is dispatched by its own exact version as WorkspaceAuthorizationBundle/1 or /2. The unchanged WorkspaceAuthorizationBundleAddress/1 still selects exactly one head and its authorizationRevision, trustRevision, byteLength and SHA-256 must match the exact selected bundle bytes. An unknown proof/bundle version, CP3 carrying Bundle2, missing ChangeRecord for CP4, mismatched address, different Workspace/root fingerprint, or incomplete common history fails through the existing unavailable/integrity boundaries; no decoder fallback, current-host guess, arrival order, or LWW is permitted.

For each validated head the resolver normalizes both SealProfileId/1 profiles. Bundle1 contributes only the revision-token history and has source-transform state none. Bundle2 folds its exact mixed Declaration1 prefix and Declaration2 suffix. The effective compromise set starts from the longest common trust prefix and recursively unions every legal losing and selected branch fact. Carry1 is verified exactly under its fixed-parent Declaration1/CP3 contract. Carry2 is verified from the original Declaration2 bytes and original CP4 activation cut. A direct Declaration2 revoke with mode compromise maps compromisedTrustKeyId to action.trustKeyId; a direct Declaration2 rotate with mode compromise maps it to action.oldTrustKeyId. originDeclarationDigest hashes the complete canonical Declaration2 including rootSignature. Carry2 factId is:

```text
body = {
  workspaceRef,commitDomain,profile,compromisedTrustKeyId,
  originAction,originDecisionKey,originDeclarationRevision,
  originDeclarationDigest,originActivationChangeId
}

factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL || D3-CJ/3(body)
    )
  )
```

originActivationChangeId is always rederived from the original Declaration2 DecisionKey through its committed CP4 + ChangeRecord/1 and the exact Bundle2 after-image that first appended that declaration. A later resolver ChangeId never substitutes. An inherited Carry1 or Carry2 is accepted only after recomputing its version-specific factId, validating the named original declaration/root signature and direct compromise action/key mapping, rederiving its original activation cut, and validating every carrying resolver link. The fold is recursive. The effective union is canonical ASCII factId order; byte-equal duplicates collapse, while the same factId with non-byte-equal canonical bytes is integrity_conflict. Facts from an unselected branch remain in the union. A resolver cannot make a compromised key safe by choosing another branch or by carrying the fact at resolver time.

affectedDomainProfiles is the union of all domain/profile pairs whose normalized head state differs and all pairs named by the effective compromise union. freshAuthorizations is sorted/unique by canonical bytes and may name either seal profile, but only an affected pair whose selected state is none or whose selected current key is unsafe under the effective union. A safe unrelated current key cannot be rotated through conflict resolution. If no trust resolution is needed and freshAuthorizations is empty, a policy-only result preserves the selected bundle family and trustRevision. Otherwise the current resolver emits exactly one WorkspaceTrustDeclaration/2 resolve_conflict declaration. A Bundle1 selection becomes a Bundle2 result by retaining its exact Declaration1 prefix and appending that Declaration2; a Bundle2 selection appends Declaration2. The declaration revision is selected.trustRevision+1; conflictId is byte-equal to the request conflictId; selected is byte-equal to the request WorkspaceAuthorizationBundleAddress/1; predecessor hashes the exact selected last declaration; resolvedHeads is the complete sorted expectedKey.heads; inheritedCompromises is the canonical effective union minus facts already effective on the selected chain; and outcomes is complete for every affected pair. TrustConflictOutcome/2 uses keep_current only for a selected current key not made unsafe by the effective facts; otherwise it is none or, when the exact affected pair was explicitly requested, authorize_fresh.

Every authorize_fresh outcome uses a newly generated protected DomainSealKeyHandle/2 and the exact PoP/2 body/message already frozen above. The containing Declaration2 root signature therefore binds the profile, new key, PoP, mixed inherited carries, complete outcomes and predecessor. The proposed Policy/3 remains complete: when byte-equal to selected.policy its policy revision is preserved; otherwise it must be the legal checked successor under the original policy_admin rules. The resulting bundle increments authorizationRevision once; trustRevision increments once only when the Declaration2 is appended. The current portable resolution uses the original current Notice3/CP4/ChangeRecord path and one final P; staged fresh handles become usable only with that same committed transition. No losing branch bytes are rewritten.

The current unseen policy arm is a complete owner-input/preview/recovery chain rather than only a Plan2 value. OwnerInputBinding/2 has protocolOwner=D6 and ownerKind=d6_conflict_resolution/3; current InputDescriptor/3.intentKind is the same controlled value, guarantee=managed_atomic, frontierPolicy=exact, saveProfile=control_only, and sourceInputs is empty. canonicalDescriptorBytes is exactly D3-CJ/3(ConflictResolutionInput/3). Source_merge and choose_source_head do not use that owner version: their ownerKind/intentKind remain d6_conflict_resolution/2 and their exact owner descriptor/derived plan/preview remain ConflictResolutionInput/2, ConflictResolutionDerivedPlan/1, and ConflictResolutionPreview/1, while their outer unseen D6 carrier is the current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family.

ConflictResolutionInput/3 contains the complete branchEvidence plus ConflictResolutionPolicyDerivedPlan/2. Plan2 contains the per-head CP3/CP4 and Bundle1/2 dispatch, selected bundle version/pin, complete per-head TrustConflictCarryValidationEvidence/1, effective/inherited Carry1|Carry2 sets, complete Outcome2 array, and result bundle version/pin. For each expected head and every fact used in that head's effective fold, carryEvidence retains the direct original compromise activation plus every resolver carrier actually traversed. Each hop pins the exact ChangeRecord, completion proof, and policy bundle used to validate that declaration; version-1 hops retain their historical CP3/Bundle1 activation evidence and version-2 hops use ChangeRecord1/CP4/Bundle2. Thus an original Carry activation cut or intermediate carrier cannot be reconstructed from a later current bundle, a digest alone, Derived Index, or resolver time.

OwnerInputBinding/2.pinRefs for the current policy arm is exactly the canonical sorted/unique union of every branchEvidence changeRecordPin, completionProofPin and present policyBundlePin; selectedBundlePin; resultBundlePin; and every changeRecordPin, completionProofPin and policyBundlePin in every carryEvidence origin/carrier hop. Duplicate equal PinRefs occur once. The policy arm has no branch sourcePins. DependencyProof/3 evidence remains in its own owner and is not omitted from PreparedIntent retention merely because it is not duplicated into OwnerInputBinding.pinRefs.

The immutable owner preview is closed ConflictResolutionPreview/2. Its conflictId, expectedKey and resolution are byte-equal to Input3; branchEvidenceDigest is SHA-256 of D3-CJ/3(the complete branchEvidence array); its derivedPlan is byte-equal to the complete Plan2, including headEvidence, carryEvidence, mixed carries, Outcome2 and result bundle version/pin. PreparedIntent/3.previewBinding binds exactly that Preview2 and its protected canonical preview pin. For this policy arm, pinDirectory is the canonical duplicate-free union of OwnerInputBinding.pinRefs, every DependencyProof/3 evidence pin, the exact preview pin selected by previewBinding, and every proposal/before/after/recovery PinRef named by installationPlan. There are no SourceObservation pins because sourceInputs is empty. The one planning CAS freezes InputDescriptor3, OwnerInputBinding2, Preview2, pinDirectory, installationPlan, staged fresh-key associations and resulting bytes together; final submit remains only d6_commit_request/2 and the one final P remains the only commit point.

Receiver admission replays the frozen Input3/Plan2/Preview2/pin set, validates every head proof/bundle dispatch, every retained Carry origin/carrier hop, every root/declaration signature and predecessor, recomputes Carry1/Carry2 unions and fact IDs, verifies every authorize_fresh PoP/2, requires byte-equal result-bundle bytes and the same-decision CP4/ChangeRecord/conflict-record transition, and rejects any omitted losing-branch compromise. Planned recovery restores those exact descriptor/preview/pins/handles and never derives a substitute from current state. Saved/planned/unknown records proven to belong to an older owner/outer carrier restore their original request, Input2/Plan1/Preview1, pins, handles and OperationId before unseen-current dispatch. No deployment migration is inferred from the existence of these current successors. A legal current dual-profile conflict with complete evidence and authority therefore has the positive path above without creating a second ledger, CAS or submit.

A required branch example is transform KT1 split concurrently into ordinary KT1→KT2 and compromise KT1→KT3. Selecting the ordinary branch does not erase the losing branch's original KT1 compromise fact or move its cut to the resolver. KT2 may be retained only if it is independently safe under the complete fold. Where the selected transform state is none or its selected current key is unsafe, a requested source-transform FreshDomainAuthorizationSpec/2 produces a fresh transform key and a valid Outcome2 instead of forcing the Workspace to remain permanently unresolved.

### 20.2 Replica registration current producer

d6_replica_register_prepare retains its fixed-parent wireVersion2 request and specialized replica_register authority; it gains no profile member and ordinary dual-profile trust add/rotate/revoke is not an authority substitute. For a current unseen registration, the winning prepare mints one fresh ReplicaEpoch and the joining secure store generates exactly two fresh DomainSealKeyHandle/2 values for that replica CommitDomain, both staged, in fixed profile order revision-token then source-transform. Caller JSON never supplies either key.

The same frozen plan appends exactly two WorkspaceTrustDeclaration/2 authorize declarations in that profile order under one DecisionKey. The first declaration is selected trustRevision+1 and the second is +2; the second predecessor hashes the complete first Declaration2 bytes. Each declaration has its own PoP/2 and rootSignature. The plan also freezes expectedReplicaRegistryRevision, the selected current bundle/trustRevision, the new ReplicaRecord, both staged handle associations, both declarations, and the exact Bundle2 after-image.

One and only one final P seal publishes the replica_registry after-image and policy/Bundle2 after-image in the same current CP4/ChangeRecord transition and same ChangeId. The two handles move staged→usable together only after that commit is admitted; neither profile can become usable from a partial prefix or from copied public bytes. Receiver admission requires that same CP4 to prove both component transitions, the exact ReplicaEpoch/CommitDomain, the fixed two-profile order, Declaration2 predecessor chain, same DecisionKey, root/PoP signatures, exact Bundle2 result and absence of an unresolved policy/trust conflict.

Planned/recovery restores the exact two staged handles and declarations from the winning plan and never regenerates keys. Any actually proved historical replica-registration decision remains under its recorded decoder/bytes and is not retrofitted with a second key. Retirement prevents future signing by the replica under both profiles once the original retire transition is admitted.

WorkspaceBootstrapProfile/4 has the same closed member set and issuer semantics as Profile3 and changes only the fresh-target trust genesis family to WorkspaceTrustGenesis/2. WorkspaceBootstrapPlan/4 retains the D3 allocation chain's canonical lowercase UUID proposalId and all other UUID members. Its creator binding, target Registry binding, series configurations, and period-scope bindings are the closed helper types in SCHEMAS §9; there are no descriptive placeholders.

Plan4 is only the current unseen fresh create_workspace/fork_workspace bootstrap successor. It preserves the original D3 proposal authenticity, issuer/target-custody ordering, one planning CAS, one DecisionKey, and one P seal. Create derives the target Registry from the frozen eligible seed; fork carries and maps the complete source Registry/configuration history at the fixed source cut. Fresh bootstrap uses one root and exactly two Declaration2 authorizations in order: revision 1 revision-token, revision 2 source-transform, same DecisionKey and activation ChangeId, with rev2 predecessor hashing exact rev1 canonical bytes. Both staged handles become usable only after that one seal commits. There is no observable one-profile prefix.

The same Plan4 also freezes mandatory initialPresentationPolicy. Prepare derives its D8 before stamp from the original issuer-proved empty target history, not from a missing file, and freezes parents=[], revision=1, separate plus a typed presentation_policy_change preview with committed=null. Planning/staging carry those exact bytes under the original operation and never create the ChangeId-dependent record/hash/pin/outbox. At the one final P, after all original proposal/custody/Registry/Policy/trust/source checks, Core allocates the original bootstrap ChangeId C and in that same atomic transition materializes the D8 /2 record, exact prefixed hash/pin/address, committed effect, original receipt/ChangeRecord association, outbox and head transition while activating W/B and all other bootstrap state. A losing CAS, abort or failed P cannot leave either a half-active Workspace or a policy record without activation. After P, default-dependent presentation immediately resolves against revision1/separate. Publication-unknown retry uses the exact outbox bytes and receiver admission checks the original bootstrap decision association. Saved/planned/unknown Plan4 resumes from its recorded init bytes without resampling state.

Ordinary managed copy is not bootstrap and keeps its existing scope/configuration mapping rules. Restore/recovery dispatches the exact saved decoder/plan and never injects Genesis2 or upgrades old bytes. continue_workspace/failover preserves current policy, principal mappings, Registry/configuration, and profile trust state instead of re-running bootstrap. This design candidate is not deployed and therefore asserts no migration or dual-write from Plan3; any actually proved historical record continues only under its recorded contract.

Replica registration remains its specialized same-record producer and is dual-profile for current FC operation; generic trust prepare is not a substitute for replica_register authority. Continuation evaluates revision and transform profiles independently as current(K)|none|conflicted_or_unproved. Any unproved profile prevents partial activation of a new signing domain; otherwise the complete sequence is optional old revision revoke, optional old transform revoke, new revision authorize, new transform authorize in fixed profile order under one DecisionKey/CP4 with no observable intermediate prefix.

### 20.3 D6 Storage §9.1 / §9.3 current consumers

The fixed-parent D6 Storage §9.1 single-profile registration/admission prose is superseded only for a current unseen replica registration. The Storage consumer now admits the exact §20.2 transition: one fresh ReplicaEpoch; exactly two fresh staged DomainSealKeyHandle/2 values in revision-token then source-transform order; exactly two Declaration2 authorizations in that order under one DecisionKey; replica_registry and policy/Bundle2 after-images in the same Notice3/CP4/ChangeRecord1 and one final P/ChangeId; and simultaneous staged->usable for both handles only after that complete transition is admitted. Receiver admission cross-validates the active ReplicaRecord, both Declaration2 predecessor/signature/PoP chains, exact fixed profile order, one DecisionKey and exact Bundle2 bytes. Retention/planned recovery preserves the exact staged pair, declarations, component pins and original request and never regenerates a key. A proven historical registration retains its recorded single-profile decoder and bytes. replica_register remains the specialized authority and cannot be replaced by generic trust administration. replica_retire blocks future signing for both current profiles once its original transition is admitted but does not erase historical authorization or take over ApprovalUse, claims, Money, external unknowns, Automation leases, or execution custody. Ordinary replica content and execution-responsibility takeover remain separate contracts.

The fixed-parent Storage §9.3 conflict direct consumer is likewise arm-dispatched. Current unseen source_merge and choose_source_head use current outer InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 but retain exact ConflictResolutionInput/2, Plan1, Preview1, source pins and source semantics. Current unseen policy_bundle_choice uses ConflictResolutionInput/3, Plan2, Preview2, mixed Carry1/2, the exact OwnerInputBinding pin union and PreparedIntent3 preview/pinDirectory closure above. Storage retains every losing branch byte, exact bundle/activation evidence, result bundle pin and protected preview pin until the original last-reference rules permit release. Final write remains the original D6 typed request and one P seal. Saved/planned/unknown work always restores its recorded outer carrier, request, owner descriptor, preview, pins, fresh-handle association and OperationId first; current outer types never reinterpret or migrate those bytes. These replacements cover Storage's current direct-consumer statements only and leave unrelated historical conflict, transport and recovery semantics intact.

## 21. D10 mixed-version control and execution responsibility

### 21.1 Current-owner routing and historical dispatch

For a current unseen/fresh D10 Workspace-control operation, the coordinated chain is D10WorkspaceReadDependencies/2 with DependencyProof/3 -> ControlDependencies/3 -> D10ControlInput/2 -> ControlPrepareBinding/3. A fresh unattended D10 author step uses PreparedActionBinding/4 and EffectManifest/3 / EffectBytes/3 to construct ApprovalUse/2. A fresh current interactive author step uses the exact current PreparedActionBinding/4 or PreparedEditBinding/3 selected by its owner contract. New scheduling uses the current ScheduleSubscription/2 and AutomationOccurrenceRecord/2 families.

This current routing explicitly supersedes the fresh producer clauses in fixed-parent D10 CONTROL-CONTRACT §4 and §7 and D6 Storage §7.2.1, in addition to the already-routed D10 sections. It is not a global search-and-replace of version names. A genuinely saved/planned/unknown record continues under the exact decoder and recovery contract that actually stored it: historical D10AuthorPreparationLink/1, PreparedActionBinding/3, EffectManifest/2 / EffectBytes/2, PreparedEditBinding/2, ApprovalUse/1, ScheduleSubscription/1, ScheduleContinuityWitness/1, ScheduleContinuityStep/1, ScheduleContinuityInvalidation/1, AutomationOccurrenceRecord/1, ControlPrepareBinding/1 or /2, their original pins, canonical request, OperationId, confirmation facts, and recovery evidence remain byte-exact when those are the recorded formats. A current mixed holder may reference those old responsibilities through explicit versioned carriers without migrating, repinning, or regenerating them.

For a fresh core_field_member author preparation, the D10 CONTROL §4 recovery link is D10AuthorPreparationLink/2. Its preparedBindingToken selects the exact current PreparedActionBinding/4 and its request is the original D6 d6_commit_request/2 produced by that preparation. Core atomically saves Link2, the complete PAB4, EffectManifest/3 / required EffectBytes/3 semantic evidence, and required recovery pins before returning the prepared author step or allowing submission. The current §7 mapping performs the existing fresh Field/Entry selection under DependencyProof/3, including document_format whenever a managed Document is semantically parsed, calls the current D7 prepare owner, retains PAB4, validates the complete EffectManifest3/EffectBytes3 and actual MutationFootprint, then constructs ApprovalUse/2. The Standing Approval still cannot select the target, Entry, member path, or requested value, and D6 still owns planning, final seal, saved replay, and the original request. A historical Link1/PAB3/ApprovalUse1 association, once proved, is recovered exactly and is never re-prepared as current.

For a fresh or otherwise undecided current external-consent control preparation, ControlPrepareBinding/3 stores the same ExternalConfirmationRequirement/1 that the fixed-parent semantics require, while the distinct protected ExternalConfirmationRecord/1 continues to own the current confirmed fact. The trusted attended confirmation event, complete immutable preview/intent binding, current principal, trusted time, consent interval, dependency revalidation, non-disclosure, and approval_unavailable/preflight ordering remain unchanged. Confirmation never mutates Binding3. A saved/prepared ControlPrepareBinding/2 retains its original requirement, canonical intent bytes, Dependencies2, confirmation association, result lookup, and recovery; it is not rewritten merely because current unseen prepares use Binding3.

The fixed-parent D6 direct consumer follows the same split. Current unseen D10 control uses D10ControlInput/2, ControlDependencies/3 and D10ControlEffectPlan/2 instead of the fixed-parent /1-/2 producer prose; current unseen automatic author use consumes ApprovalUse/2 plus Link2/PAB4/Effect3. Saved/planned historical associations are recovered before current qualification and are never rewritten merely because current successors exist.

The D6 Storage §7.2.1 current producer likewise starts a fresh scheduling registration only for ScheduleSubscription/2. After the current D10 scheduling gates and finite retention reservation succeed, the same configuration transaction registers the actual Core source/control producer and creates ScheduleContinuityWitness/2 with ScheduleRecurrenceEvidence/2 initial/checkpoint, revision=1, consumedTransition=0, and a fresh producerEpoch. The current witness artifact bytes are exactly UTF8("D6-Schedule-Continuity/2") || NUL || D3-CJ/3(complete ScheduleContinuityWitness/2). Current positive transitions use ScheduleContinuityStep/2 with DependencyProof/3 and the actual current ChangeRecord/1 + Notice3 + CP4 pins; each current step artifact is exactly UTF8("D6-Schedule-Step/2") || NUL || D3-CJ/3(complete ScheduleContinuityStep/2). A vanished, conflicted, invalid, unavailable, or history-gapped current source uses ScheduleContinuityInvalidation/2 with its already-closed D6-Schedule-Invalidation/2 domain. All three current pins use PinRef/2 payloadKind=artifact and retentionClass=recovery, and byteLength/SHA-256 covers the complete domain+NUL+canonical-object bytes. The digest is comparison evidence only; protected producer/subscription provenance and exact generation/epoch/transition continuity still authenticate the witness. binding_changed still requires proved selected-business discontinuity and gap still covers unavailable/unknown history. The producer preserves the fixed-parent atomic inbox, retention, counter, capacity, authorization, compaction, error, and no-reset rules; this successor changes only the current typed evidence/dependency/component family and exact current artifact domains.

D6 producer/fold/compaction/receiver/recovery and the D10 §16 continuityPins consumer dispatch by the authenticated domain before decoding the inner record. D6-Schedule-Continuity/1 and D6-Schedule-Step/1 accept only historical Witness1 and Step1; D6-Schedule-Continuity/2 and D6-Schedule-Step/2 accept only Witness2 and Step2. The same exact /1-versus-/2 rule applies to Invalidation1/Invalidation2. An unknown domain, domain/object version mismatch, alternate prefix, missing NUL, or non-canonical bytes fails closed through the original authorization/disclosure and unavailable/integrity boundaries; no fallback decoder widens /1. continuityPins may contain exact current typed /2 pins, exact historical typed /1 pins, or the complete original typed chain when real retained history spans both, but every element keeps its own original bytes/domain/decoder. Compaction may discard intermediate payload only after the retained exact typed witness/step chain still proves the fixed-parent continuity invariants.

An existing or retired ScheduleSubscription/1 remains a valid historical recovery-retention owner with Witness1/Step1/Invalidation1 and its original /1 pins/producer obligations. It is never background-migrated. The already-defined explicit same-generation Subscription1 -> Subscription2 continue is legal only after complete retained history proves no intervening format/rule/business discontinuity and establishes the current Evidence2/Proof3 cut; otherwise replace creates the new generation. Such continue preserves the semantic generation and every old typed artifact pin; only newly produced Witness2/Step2/Invalidation2 artifacts use /2 domains. It never repins or re-encodes old /1 bytes or resets an invalid generation.

### 21.2 Images, pins, ranges, and effect plans

D10ControlRecordImage/2 exists only for automation and run records whose nested current schema changed. Other record kinds, including planned_approval, external_approval, activation, reservation, external_effect, stop, and supplement=none records, continue to use exact Image1. Image2 cannot be downgraded to Image1, and an old Image1 is never re-encoded merely to enter a mixed holder.

Pin1 continues to authenticate the fixed-parent D10-Control-Record/1 payload. Pin2 authenticates the exact D10-Control-Record/2 prefix plus D3-CJ/3(Image2), with artifact payload kind, the original recovery or approval_money retention class, and exact byteLength/SHA-256. Pin2 never repins Image1.

A mixed record-pin array is ordered by the inner image: canonical binding.ref bytes, numeric binding.revision, usageRevision with none before some, numeric usageRevision when present, then complete canonical image bytes. The logical same-cut identity is binding.ref + binding.revision + usageRevision. Two entries with that identity and byte-equal image are duplicate responsibility and are rejected; two with different image bytes are integrity_conflict. Schema version is never a reason to retain two values for one cut.

Range kind rank is records=0, cost_lineage=1, occurrences=2. The logical range identity is respectively scope plus the complete sorted kind set, CostLayerKey, or Automation Ref. A mixed range array orders by kind rank and canonical logical-identity bytes and contains each logical identity at most once across Range1/Range2. An occurrences range orders its inner records by AutomationOccurrenceKey; V1/V2 values with one occurrence key cannot coexist.

D10ControlEffectPlan/2 changes are sorted uniquely by the complete canonical after.value.binding.ref. A non-none before must have the same binding.ref as after and exactly one matching versioned record pin for the actual stored before schema. Image1 before with Pin2 is invalid. Current Automation configure may legitimately move Image1+Subscription1 to Image2+Subscription2. A true continuation may retain the same subscription generation when the D10 scheduling owner says continue; generation replacement occurs only when that owner rule requires replace. Mixed-holder uniqueness is a snapshot rule, not a requirement to replace every old subscription.

### 21.3 ControlDependencies/3 and the one-cut rule

ControlDependencies/3 retains the complete actual configBindings, usageBindings, authority proof, authorization generations, stopRefs, optional Workspace reads, versioned record pins, and versioned ranges. Config bindings remain canonical by full Ref; usage bindings by full Ref; authorization-generation tokens by canonical token bytes; stop refs by full Ref; record pins and ranges use the mixed rules above.

Every configBinding has the exact matching record image/pin. Every usageBinding has a matching image at the same usageRevision. Every stopRef has its exact stop Image1/Pin1 because stop is not mechanically upgraded. Current range fences, bindings, usage revisions, record images and the protected Workspace evidence used for one preparation come from one real Authority Store barrier. A V1 value captured at barrier A cannot be combined with a V2 value captured at barrier B and called complete. A historical image referenced by current responsibility remains immutable evidence and does not become current configuration merely by appearing in the holder. Missing required old bytes/pins/decoder is state_unavailable after the ordinary disclosure gates; contradictory same-cut evidence is integrity_conflict. No current Image2 may substitute for a required old Pin1.

### 21.4 Claims, Money, and Inventory canonicality

D10ExecutionClaims/2 keeps all mixed responsibilities complete. recordPins use the mixed pin order. prepareBindings are ordered by inner StableControlKey canonical bytes and that key is globally unique across Binding1, Binding2 and Binding3. leaseRuns are ordered uniquely by full Run ControlRef. authorSteps are unique by run Ref plus stepId across Step1/Step2. subscriptions are unique by Automation Ref plus generation across Subscription1/Subscription2. occurrenceRecords are unique by AutomationOccurrenceKey. ranges use the mixed range order. continuityPins are ordered uniquely by pinToken. Each continuity pin is an exact typed D6 schedule artifact and dispatches by its retained D6-Schedule-Continuity/{1|2}, D6-Schedule-Step/{1|2}, or D6-Schedule-Invalidation/{1|2} domain before inner decode; no free proof-map, schema guessing, /1 decoder widening, or cross-version repin is allowed. Wrappers select an exact decoder only; they do not add kind/version/confirmation fields to Binding1, do not LWW, and do not alter original pins or dependencies.

D10MoneyResponsibility/2 orders reservations uniquely by the complete Binding<reservation>/1 canonical key, layers uniquely by CostLayerKey canonical bytes, recordPins and ranges by the mixed rules, and evidencePins uniquely by pinToken. Contradictory values for one reservation identity cannot be represented as two liabilities, and CostLayerTotal remains a verified projection of the complete reservation/attribution set rather than a balance reconstructed from current configuration.

D10ExecutionInventory/2 orders ApprovalUse carriers uniquely by DecisionKey canonical bytes across versions, externalUnknowns uniquely by binding.ref canonical bytes, and stopState uniquely by binding.ref canonical bytes. It includes every responsibility still needed for pending work, recovery, deduplication or irreversible stop, including completed external attempts that remain referenced. Inventory2 is captured only after admission, planning, send, and schedule writers for the old holder are stopped at one real store barrier; mixing old and new values from different barriers is forbidden.

### 21.5 Inventory pin, Record3 equality, store incarnation, and StopCapacity

The Inventory2 artifact is exactly UTF8 D6-Execution-Inventory/2, one NUL byte, then D3-CJ/3(D10ExecutionInventory/2). Its PinRef/2 has payloadKind=artifact, retentionClass=recovery, and byteLength/SHA-256 for those complete prefixed bytes. Inventory1 keeps its historical D6-Execution-Inventory/1 domain and is never repinned or re-encoded as Inventory2.

ExecutionResponsibilityRecord/3 and its pinned Inventory2 must agree exactly: workspaceRef is equal, and the five responsibility payloads approvalUses, claims, moneyLineage, externalUnknowns, and stopState are byte-equal canonical values. ExecutionContinuityProof/2.inventoryPin must select that exact Inventory2. Inventory2.storeIncarnation is byte-equal to ExecutionContinuityProof/2.storeIncarnation, and the protected birth/barrier/fence token mapping must prove that actual store and the same capture barrier; no separate caller-provided store-incarnation proof object is introduced.

Inventory2.stopCapacity is the exact StopCapacity/1 value from the authoritative safety store at that same barrier. StopCapacity/1 remains the fixed-parent {issued,reserved} counter pair and is not duplicated as a Record3 member. A Workspace-only custody move cannot copy those shared counters into a second active store. Either the original authoritative safety store remains the reachable serialized owner, or a complete store handoff fences every affected writer and preserves all target/latch reservations and capacity. Unproved store continuity or safety capacity makes takeover unavailable; zero, a visible subset, or a rebuilt value is never accepted.

Record2/Proof1/Inventory1 retain their historical decoder and payload domain. Record3 is formed only for a real responsibility mutation, checkpoint, or custody handoff. Such a transition strict-decodes every retained historical responsibility, captures one complete mixed Inventory2, pins it once, creates Proof2, checked-increments the record revision when applicable, and preserves executionDomainId. It never mints replacement ControlRefs, requests, approvals, reservations, claims, or a second execution ledger.

### 21.4 D10 interactive-start and Stop consumers at the actual D6 boundary

The only fresh interactive Run/Run-targeted Lease producer is the closed, trusted attended Core runtime operation `d10_interactive_run_start` in current D10-CONTROL §7.1: actual D1 `automation.manage`, D6-authenticated human workspace principal, current Policy/3 `d10_control_self`, complete finite LeaseSpec, current activation/clock/deployment grants/budget and same Authority Store atomic Run Image2+Lease Image1+one StopOwner/latch Image1 with pre-reserved safety capacity, audit and dedup result. It is **not** D6 `PreparedIntent`, a new author operation, D10's eighth ControlBody, Automation/occurrence or second D6 ledger. D6 consumes subsequent author steps by the saved `D10AuthorStepResponsibility/2` arm. **Only** the genuine Automation `core_field_member` arm binds D7 PAB4/Manifest3/EffectBytes3 to real Link2/ApprovalUse2/Standing Approval and its original `d6_commit_request/2`. The no-Automation `interactive` arm instead binds its original D3IdentityOperationRequest/13 or `d6_commit_request/2` to current D7 PAB4 or D8 PreparedEditBinding/3 selected by preparedFormat, complete protected preparedRecordPin/recoveryPins and authenticated explicit per-operation confirmation after full D7/D8 preview. D3 retains its identity submit owner; D6 retains the original DecisionKey/final P for its own request, without inventing Link2, Standing Approval or a second author result. Genuine historical Link1/PAB3/ApprovalUse1 and recorded interactive prepared forms retain their own closed decoder, bytes and pins. First protected step uses existing `LeaseRunUse/1` CAS; same Run later protected steps and saved/planned recovery do not consume `maxRuns` twice, but must recheck current authority, lease revision, clock, stop, activation and budgets. The start, admission and stop association must address the **same actually opened Authority Store incarnation** as this D6 final P and its safety latch. Uncertainty or a cross-store claim fails; no half Run/Lease/Stop, replacement author request or author ledger success is allowed.

For a D10 Control `stop` read, current authorized W workspace scope and target-qualified H deployment scope may resolve the same **one W-scoped** Image1/StopOwner/latch/first result through the strict D10-CONTROL §7 target/Workspace/store-incarnation relation; H access does not create another persisted Stop scope. Non-Stop record reads retain strict `scope == record.scope`. Original D6 final P and original first external handoff serialize against that one irreversible latch. Before revealing the same result to W/H, use current target/nested disclosure; wrong Workspace/store, hidden target or no H target authority is original `not_visible`, missing authority/continuity/custody keeps its named error ordering. This does not change D6's actual fresh Profile4/Plan4/Genesis2/initialPresentationPolicy bootstrap owner or its one original P; no Plan3 reappears.

## 22. Historical replay and candidate boundary

Every genuine saved/planned/unknown record first dispatches by its recorded outer carrier, owner descriptor, preview, pins, token/handle association, OperationId, authorization/custody/TTL and original error order. Current successors never re-encode historical bytes. An undeployed predecessor candidate creates no migration duty merely because its name appeared in design prose.
