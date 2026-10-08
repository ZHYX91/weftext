---
source_language: zh-CN
translation_of: D7-REGISTRY-QUALIFICATION.zh-CN.md
translation_status: synced
---

[简体中文](D7-REGISTRY-QUALIFICATION.zh-CN.md)
# A2 D7 terminology Registry qualification

Status: author repair for A2-D7-2F89-P1-01. D7-REGISTRY.json remains the exact parent-current machine Registry; this document is a qualification overlay, not a second Registry authority.

## 1. Three distinct layers

The immutable fixed-S Registry is blob 4615d8c03df22ba9630728f946715de8e4308438 with 34 concepts and 8 cross-stage bindings.

The coordinated parent-current Registry is blob 3cab5124f46822729093d5d955908b05eb65bb87 with 34 concepts and 13 cross-stage bindings. D7-REGISTRY.json is byte-identical to that parent-current blob.

A2 does not create another Registry object. A2 keeps the parent-current 34/13 object and applies only the named successor qualifications below. Historical records continue to dispatch through their original concrete versions.

## 2. Binding-by-binding qualification

| ID | Parent-current owner / meaning | A2 disposition | Fresh A2 concrete rule | Historical retention |
| --- | --- | --- | --- | --- |
| B01 | D3 identity algebra / D7 runtime payload | retained-current | EntityRef, Locator, LogicalOccurrenceKey, ResultRowHandle, Provenance and ActionEvidence keep the same owners and separation | all recorded forms remain under their original owner |
| B02 | D4 schema and authorship | retained-current | FacetId, FieldId, FieldSelector and authoredProvenance remain D4/D5-owned as stated | no version rewrite |
| B03 | D6 control and transport | named-current-successor-overlay | SourceObservation/1, SourceVersionRef/1, CommitDomain/2, Frontier/2, OwnerInputBinding/2 and ObservationScope/2 remain; fresh complete proof is DependencyProof/3 rather than the parent text's /2 | genuine Proof2/older protected records retain exact decoder and pins |
| B04 | D2 occurrence / D7 payload | named-current-successor-overlay | saved Query/View remains a D2 occurrence, but fresh A2 carrier is the current D2 product/SavedDefinition contract and may contain QuerySpec/2; no ViewRef is created | Profile2 wording is source history; actual recorded old payloads remain exact |
| B05 | D3 identity submit / Definition Transfer | named-current-successor-overlay | fresh identity submit is D3 wire13 and cross-owner preparation is PAB4; DefinitionTransfer and Result/9 Q remain their real inner types | wire12/PAB3 associations recover only when actually recorded |
| B06 | D6 capability split | retained-current | Policy/3 keeps source_envelope_state and commit_sequence_state independent; no implication is added | Policy/1-/2 records keep exact historical semantics |
| B07 | D4 DerivedDuration | retained-current | D7 continues to consume the already-frozen D4 result through the generic adapter | unchanged |
| B08 | D5 CollectionCreationPolicy | retained-current | D5 owns saved collection creation semantics; D7 carries it without another creation policy type | unchanged |
| B09 | D7 protected record and delivery | named-current-successor-overlay | fresh A2 uses PAB4, D7ActionInput/3, D7ProposedInput/3, EffectManifest/3, EffectBytes/3 and ownerKind d7_action/3; MinimumMapping/3, D7DefinitionInput/2, D7ResolutionAccess/1, FieldEntryImage/2 and D7DefinitionTransferEffects/2 remain at their real versions | PAB3/Input2/Effect2/d7_action2 and older records recover exact bytes/pins |
| B10 | D3 resolution / D6 installation / D7 read-only canonical effects | retained-current with current outer carriers | D3ResolutionInput/1, D3CanonicalEffectPlan/1, D3CanonicalPlanProjection/1, D3CanonicalEffects/1, ConflictInstallInput/1 and D7 canonical/restore images retain their inner versions; current outer request/proof/effect carriers are wire13/Proof3/Effect3 where applicable | original inner and outer version pairs dispatch exactly |
| B11 | D8 edit consumer / D7 transport | named-current-successor-overlay | fresh D8 edit consumer is PreparedEditBinding/3 with PreparedIntent3 and EffectManifest/3/EffectBytes/3 | PreparedEditBinding/2 and Manifest2/EffectBytes2 are historical only when actually recorded |
| B12 | D9 template consumer / D7 preparation | named-current-successor-overlay | TemplateRecipe/2, TemplateConstruct/2, TemplateConstructionInput/2 and D9EntityVersionAddress/2 remain D9-owned; current construction adapter enters PAB4 rather than PAB3 | historical PAB3 construction associations remain exact |
| B13 | D6 bootstrap / D7 display / D10 consumer | named-current-successor-overlay plus exact historical dispatch | fresh current bootstrap uses WorkspaceBootstrapPlan/4, WorkspaceBootstrapProfile/4 and WorkspaceTrustGenesis/2; Policy/3 remains; current D7 effect transport uses EffectManifest/3 and EffectBytes/3; the existing d7_planned_preview_open/opened surface keeps its real outer contract and reopens the actual protected current or historical record selected by that record | Plan1/Plan3/Profile3/symbolic2 and Manifest1-2 records retain exact family/custody; a historical Manifest2 record is not relabeled Effect3, while fresh current preview payload uses Effect3 |

**B05/B09/B13 named current producer/consumer overlay:** Direct interactive Run start is the trusted attended D10 Core §7.1 runtime producer, not a D7 Action or eighth control action. Only after the existing Run-targeted LeaseRunUse CAS admission can the same Run prepare original D7/D8 author work. Its saved D10 Step2 arm decides: **automatic** `core_field_member` alone requires a genuine Automation, current D7 `PreparedActionBinding/4`/`EffectManifest/3`/`EffectBytes/3`, Link2/ApprovalUse2 and qualified Standing Approval; **interactive** without Automation uses real D3 Request13 or D6 request2, current D7 PAB4 or D8 PreparedEditBinding3, exact preparedFormat/preparedRecordPin/recoveryPins and all original D7/D8 preview pages/EffectBytes followed by explicit authenticated confirmation for the same request. D3/D6 retain their original owner, DecisionKey/OperationId and applicable one D6 final P; interactive never needs or creates Link2/ApprovalUse2. Real recorded historical PAB3/Manifest2/Link1/ApprovalUse1 takes **its own** saved decoder, exact pins and preview semantic digest, even across cold restart. Current PAB4 cannot be fetched through a PAB3-only pin, a historical Manifest2 cannot be rehashed as Manifest3, and a Link1 cannot be relabeled Link2 for cold start. `d7_planned_preview_open` dispatches by the actual stored version, retains the exact original request/OperationId/complete preview and closed error/disclosure order. There is still one D7 owner/Registry, no D10 second registry or author ledger. B13 remains exact D6 Profile4/Plan4/Genesis2 plus mandatory D8 initial presentation policy at one final P, never the old D10 Plan3 consumer.

## 3. Ownership and precedence

The parent-current Registry remains the sole machine list of the thirteen bindings. This overlay neither edits the thirteen parent rows nor republishes them under new version strings. It states which concrete versions are fresh-current in A2 and which parent strings are historical or unchanged.

Where this table says named-current-successor-overlay, the exact current schema in D7-SCHEMAS and the fixed97 owner wins for fresh unseen work only. Where it says retained-current, the parent binding remains current without a mechanical version bump. Historical-record-only never means deleted: saved/planned/unknown/committed records retain their original decoder, pins, request fingerprints, custody and recovery.

## 4. Review oracle

A reviewer must be able to start from any of B01-B13, identify its sole semantic owner, distinguish fixed-S 34/8 from parent-current 34/13, identify any A2 fresh successor without rewriting unchanged inner versions, and identify the historical decoder branch. A count alone is not acceptance evidence.
