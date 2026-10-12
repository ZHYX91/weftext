---
source_language: zh-CN
translation_of: CORE.zh-CN.md
translation_status: synced
---

[简体中文](CORE.zh-CN.md)

# Typst Core: storage, identity, transactions and authorized compilation

Status: coordinated author candidate; not independently accepted or implemented. Input: `af6737a057c7bb69763dfaad0a57da4652e085f3`. This defines a new target and does not claim that the old prototype has migrated. Product, conditional filesystem installation, synchronization, crash, cryptography, Server, performance, package isolation and UI acceptance are **UNRUN**. Protocol objects here are not objects authors must write manually.

## 1. One format and inventory

The new target has one managed Document, `X/X.typ`. The entire root `.weftext-format` is exactly ASCII `weftext.typst.v1` followed by one LF: `weftext.typst.v1\n`. BOM, CR, spaces and additional lines are forbidden. This selects the candidate profile; it proves neither Workspace nor Node birth, file binding nor permission. Missing, incorrect and unknown markers follow ordinary unmanaged-content or explicit incompatible/incomplete diagnosis; never guess or rewrite a marker.

Core is the sole inventory authority. Directory ancestry determines parentage; paths are locators and IDs are lowercase UUIDv4. Do not persist parent/path/name/child-path lists. Natural name ordering, parent-owned sorting policy and child sparse manual rank remain. Root `.weftext-rules` is independent portable authority, never inherited from `.gitignore`. Unmanaged directories are complete, non-reentered subtrees without UUID/envelope; ignored bytes remain backup content. A managed Node's same-named `.typ` cannot be classified separately. Ordinary noncanonical files are resources; derive an owner UUID only within a managed owner boundary. Independent annotations still use `weftext.annotations.json`, without a node metadata directory. SQLite/index/graph/backlinks/thumbnails are rebuildable derivatives.

File-manager and Server create, rename, move, copy, trash, restore, purge and resource replacement use the same Core plan/transaction. Core rejects structural targets inside ignored/unmanaged boundaries and managed subtrees containing such a boundary. Shells cannot move files and repair metadata afterward. A trusted backend proves canonical containment, symlink/reparse/hard-link aliases, case and cross-volume rename; uncertain object identity or boundaries make installation unavailable. Root AGENTS' `.adoc` and `weftext.asciidoc.v1\n` describe the existing prototype, not implemented `.typ` support. Old material enters through explicit import with retained input and a loss preview, never extension replacement, implicit dual authority or startup conversion.

## 2. One closed profile identity

```text
TypstProfileCarrier/1 = {
  kind:"weftext_typst_profile",version:1,
  profileId:"weftext.typst.v1",documentLanguage:"typst",
  languageBaseline:"0.15.1",rootMarker:"weftext.typst.v1\n",
  sourceContract:"typst_source/1"
}
TypstDocumentFormatBinding/1 = {
  kind:"weftext_typst_document_format",version:1,
  ownerNodeRef:NodeRef,bindingRevision:Counter,
  profileCarrier:TypstProfileCarrier/1
}
TypstDocumentQualification/1 = {
  kind:"weftext_typst_document_qualification",version:1,
  sourceObservation:SourceObservation/1,
  documentFormat:{key:DocumentFormatDependencyKey/1,
    stamp:{epoch:Token,revision:Counter},
    componentImage:ComponentImage/1,
    binding:TypstDocumentFormatBinding/1,bindingPin:PinRef/2}
}
```

All members are required; reject unknown, duplicate, missing and null members. Counter, Ref, Token, Pin and canonical JSON use their owners' exact decoders. D3-CJ/3 encodes the complete new object including its carrier; deleting new fields never produces a valid old digest. Binding revision begins at1 and checked-increments on a real format change. Restore retains the exact historical binding; fresh identity create/copy/fork begins at1. The component uses the existing document_format key. Present bytes are the complete canonical binding, image.version equals bindingRevision, and the pin binds the same NodeRef, purpose and all bytes. Observation.entityRef, binding.ownerNodeRef and key.ownerNodeRef agree; key.workspaceRef equals that NodeRef's Workspace. Verify complete image/stamp/pin and observation at the same cut.

Inventory, source reads, Field/Body projection, Query/editing, prepare, pins, installation, seal, receiver and rendering/export consume this exact carrier. A new receiver chooses the new decoder from the authenticated Workspace root profile and exact component binding.kind/version. Old ManagedDocumentFormatBinding1/2 decoders do not accept Typst constants. Marker, filename, package declaration, caller carrier or compiled metadata query cannot alone choose a decoder. A changed format stamp/binding invalidates an old proof even when source bytes/version are unchanged. Current root selection never rebinds old-profile recovery.

## 3. Control declarations do not replace birth or authorization

[SOURCE](SOURCE.md) owns TypstRootDeclaration/1 and native Field/Schema rules. Core consumes authenticated_structure, invalid_control or incomplete_source; authenticated_structure proves only exact root-authored structure. The unique root-authored `#metadata((id: ..., types: ...)) <weftext>`, permitted prefix, protected standard-library binding, complete root reserved-label disambiguation and direct literal ID/types must satisfy SOURCE. Titles may precede the declaration; Core does not impose a first-expression restriction. Include/import/template/generated results cannot supply identity. Then independently verify original Node birth, Workspace/NodeRef, lifecycle, file-object binding, complete portable history, format qualification and current Policy. Changed/duplicate IDs, externally copied files with IDs and partial-sync missing metadata confer no new birth and never trigger automatic UUID regeneration.

New IDs come only from the original D3 stage12 private candidate map. Mapped/new subjects consume that map; existing subjects consume the authenticated original identity. Prepare does not disclose fresh IDs. Copy/template/import use one map for all Nodes, Resources, Annotations and references. Same-identity restore/repair/explicit conversion preserve original identity and exact historical evidence. Path, hash and equal strings cannot transfer identity. Types are not Fields; business setters cannot alter id/types. Facet/type changes retain node-control/Source and complete schema gates.

```text
TypstControlConstruction/1 = {
  kind:"typst_control_construction",version:1,
  subject:PayloadSubjectKey,sourcePin:PinRef/2,
  declarationRange:{start:Counter,end:Counter},
  idRange:null|{start:Counter,end:Counter},
  typesRange:null|{start:Counter,end:Counter},
  declaredTypes:[text...]
}
TypstDocumentConstruction/1 = {
  kind:"typst_document_construction",version:1,
  profileCarrier:TypstProfileCarrier/1,subject:PayloadSubjectKey,
  sourcePin:PinRef/2,control:TypstControlConstruction/1,
  transferPlanPin:null|PinRef/2
}
```

Only Core constructs these from SOURCE's official AST, exact skeleton/source pin and authorized same-cut Recipe/Facet selection. Caller offsets, UUIDs and trust flags are forbidden. Subject is Node existing/mapped/new; sourcePin binds that subject's complete UTF-8 preimage. Ranges are official AST half-open byte intervals with BOM/EOL/comments preserved. Existing declaration has positive width, idRange selects its unique direct string token, and typesRange selects the complete direct tuple/array. Missing types uses null and a verified complete-dictionary insertion plan. A fresh skeleton alone may use a zero-width declaration insertion point after BOM with both value ranges null; nonfresh missing control never creates identity. Insertion cannot reclassify original body. SOURCE owns declaredTypes ordering and authored shorthand; the schema owner provides validated effective closure.

After stage12 use the same map. Combine control patches and FEATURES' complete TypstDefinitionTransferPlan/1 AST-reference edits into nonoverlapping exact byte edits; reject overlap. Reparse final official AST and positions after length changes. New Documents do not use AsciiDoc Result10 H or single-line JSON Q framing. Transfers needing final production addresses use the same SourceRevisionPlan1 and proposed revision token for two passes: construct, then verify every typed address/range and self-reference in the complete final source. Old line/span assumptions are invalid. Before obtaining afterPin prove control/map/birth consistency, schema types, final declaration uniqueness, complete typed coverage and unchanged undeclared bytes/AST classification. CAS freezes construction, transfer pin, final bytes, map, version basis and token. Recovery never replans from current files.

## 4. Generic transactions and finite language slot replacement

D6-CONTROL §§1–9,15,18–20 and D3 §§4,10–15 retain ownership of DecisionKey2, CommitDomain2, ChangeId1, Frontier2, SourceVersion2, SourceObservation1, PinRef2, SourceRevisionPlan1/2/3, RevisionTokenBinding2, signed artifacts, P ledger, installation, recovery and error order. Source-save prepare wire2, commit request wire2, Notice3/CP4/ChangeRecord1, PortableComponentKey2 and OwnerInputBinding2 keep their exact generic shapes without parser/carrier/encoding fields or another submit entry. The sole generic dependency successor is generation4 for compile_world below. Source-save uses SOURCE validation; body compilation errors do not automatically invalidate source syntax/control. Ordinary save may be semantic_pending; complete save proves all actual obligations. Strict/observed-only authorization, one existing live Document, explicit trusted human selection and prohibited downgrade remain.

```text
CompileWorldDependencyKey/1 = {
  kind:"compile_world",workspaceRef:WorkspaceRef,ownerNodeRef:NodeRef,
  target:"html"|"paged",principalAudienceToken:Token
}
DependencyKey/4 = exact original fifteen DependencyKey/3 arms | CompileWorldDependencyKey/1(rank15)
DependencyProof/4 = exact DependencyProof/3 members; version=4; entries[].key=DependencyKey/4
InputDescriptor/4 = exact InputDescriptor/3 members; version=4; controlInputs[].key=DependencyKey/4
PreparedIntent/4 = exact PreparedIntent/3 members; version=4;
  inputDescriptor=InputDescriptor/4; dependencyProof=DependencyProof/4
```

These closed successors retain original kind literals; planToken's protected tag is d6_plan/4. Generation3 does not gain a sixteenth arm. All new candidate prepares use4, without mixed3/4 within a holder. Static Field/ordinary source operations need no compile_world entry and need not compile/read body. Wire2 prepare has no embedded descriptor/proof; wire2 commit selects a protected token, so neither wire mechanically changes. Actual current slots are identity envelope.inputDescriptor, PAB adapter.dependencyProof/observationProof, EDITOR prepared binding.inputDescriptor/dependencyProof, D9 export input, D10 workspace_reads.dependencyProof, source-save holders and their PreparedIntent backing. Owner input, pin directory and preview retain generic decoders.

Language-coupled holders use one new nominal adapter. This is a complete finite schema definition, not unknown members added to old types:

```text
FormatAdapterEnvelope/1 = {
  kind:"weftext_format_adapter",version:1,
  role:Role,profileCarrier:TypstProfileCarrier/1,payload:Payload(Role)
}
Role = identity_input | identity_request | proposed_input |
       action_input | prepared_action | effect_manifest | effect_bytes |
       edit_input | prepared_edit | recipe | construction_input |
       render_binding | export_plan | bootstrap_plan |
       workspace_reads | control_effect_plan | recurrence_evidence |
       activation_image | author_step
```

There are two explicit definitions of Payload(Role). Slot-derived roles copy the fixed-input schema member set/matrices with only listed substitutions; original inner kind/version literals identify that new nominal payload and are never passed to an old decoder. Complete-new-payload roles use the named owner's entire closed type, without simultaneously imposing old members. Envelope kind/version/role selects the new nominal type; every source format qualification has the same carrier. Unlisted members and optional/null/order/permission constraints of slot-derived roles remain exact. No reflective scans, arbitrary JSON, recursive version guessing, any or extra roles exist. Named language-coupled nested types use corresponding envelopes; generic atoms remain original. This avoids another transaction authority and hidden format slots.

| Role / exact old schema owner | Exact payload substitutions |
| --- | --- |
| identity_input / D3-SCHEMAS §6 D3IdentityInput14, AUTHORING-INTEGRATION §6.1 | Actual Document construction pins in intent select TypstDocumentConstruction1; original intent/mode/authority matrices remain. Owner kind/intent is typst_identity/1. |
| identity_request / D3IdentityOperationRequest14 | inputDescriptor becomes4; intent as above. preparationBinding remains exactly `{kind:"d7_preparation_binding",bindingToken}`; only token backing selects prepared_action, without embedded object/cycle. Fingerprint covers full envelope bytes; receipt/error selects its domain and does not masquerade as wire14. |
| proposed_input / D7ProposedInput3 | Exact `{subject,payloadKind,encoding,pin}`. Document only exact_source_utf8 or typst_document_construction1; Resource only resource_bytes; Annotation only typst_annotation_value1 or typst_annotation_construction1. All old Result9/10 are forbidden in new carriers; §4.1 defines same-map Annotation construction. |
| action_input / D7ActionInput3; prepared_action / PAB4 | action is FEATURES.TypstActionSpec1; every canonicalCallInputs item uses complete FEATURES.TypstQueryCall1 without old literal/parameter domains. QueryRef, inline/saved definitions, parameter type/default, arguments and context.now typed slots select FEATURES' closed TypedLiteral/ParameterSpec/native Query family, not merely two addresses. Definition/schema inputs use TypstDefinitionInput1/SOURCE qualification; construction/proposals/request/effects select matching roles. Proof4 and original observation/budget/deadline remain. |
| effect_manifest / EffectManifest3; effect_bytes / EffectBytes3 | Exactly §4.1 closed effect/encoding matrix. Preserve complete-set/phase/disclosure gates; digests and construction preimages never replace committed after bytes. |
| edit_input / D8EditInput3; prepared_edit / PreparedEditBinding3 | Complete new payloads are EDITOR TypstEditInput1 and TypstPreparedEditBinding1; do not copy old D8 members. Their descriptor/proof/preparedIntent become4, preview uses effect_manifest envelope and sourceInputs remain original. Do not invent request inside PreparedIntent: derive the original commit request2 from protected planToken/operationId and current fence; the protected prepare record retains its exact bytes. Old D8 map/locators confer no write qualification. |
| recipe / Recipe3; construction_input / TemplateConstructionInput3, AUTHORING-INTEGRATION §2 | Complete new payloads are FEATURES TypstRecipe1 and TypstTemplateConstructionInput1. Identity construction uses §3; same-cut schema/environment and original execution/freezing order remain. |
| render_binding / D9DocumentRenderBinding1; export_plan / ExportPlan4 | Rendering uses FEATURES TypstExportBinding1 with exact format/source snapshot/environment/world/route map. Apply this table to corresponding plan rendering/source/Recipe/definition/schema slots. Routes, budgets, loss confirmation, pins, worker/proposal/download and receipt recovery remain. |
| bootstrap_plan / WorkspaceBootstrapPlan4 | targetRegistry snapshot/binding select SOURCE snapshot/binding; profile.registrySeedBinding becomes TypstSchemaBinding1. Original custody freezes complete authorized seed/digest/marker/layout/source-format effects. Remove initialPresentationPolicy; retain dual trust/initial policy/creator/series/period (§7). |
| workspace_reads / D10WorkspaceReadDependencies2 | registryReads[i] snapshot/binding use SOURCE new types and snapshotPin covers the full snapshot; sourceInputs remain original and proof becomes4. |
| control_effect_plan / D10ControlEffectPlan2 | registryChange.before.snapshot is actual recorded schema; after is TypstSchemaSnapshot1; evolution is SOURCE's complete explicit evolution. Apply this table only to activation/author language slots in changes; other images remain original. |
| recurrence_evidence / ScheduleRecurrenceEvidence2 | payload.dependencyProof is exactly DependencyProof4; registrySnapshot/evolution use SOURCE types; observation/sourcePin/metadataPin/recurrenceContext retain original definitions. §4.3 specifies every producer/consumer, before/after proof, ancestor, pin and historical dispatch. |
| activation_image / D10ControlRecordImage1 activation arm; author_step / D10AuthorStepResponsibility2 | Apply listed adapter choices to supplement registry/evolution, request/prepared binding/descriptor/render qualification. Other nineteen control projections, claims/Money/stop/external consent are not replaced. |

OwnerInputBinding2.canonicalDescriptorBytes is the complete canonical envelope. Owner kind equals intentKind: typst_identity/1, typst_action/1, typst_edit/1, typst_bootstrap/1, or the original control intent whose descriptor contains named adapter slots. D6's action/edit input protocolOwner remains D7/D8; identity uses D3. Final decision remains mutually exclusive D3 or D6 ownership. pinRefs is the exact sorted unique union of all proposed/schema/world/transfer/preview and original required evidence pins. PreparedIntent4.pinDirectory additionally covers proof4, observations and installation-plan pins. Token backing selects the envelope type; tags/audience/expiry confer no authority. Original PreviewBinding now binds the complete owner preview envelope. After bytes, format, manifest, route and schema cannot switch profiles.

Original execution-action §7 defines CollectionCall with exactly call/nodeColumn. This profile's action.collection.call in collection_create and every actual CollectionCall position select FEATURES.TypstQueryCall1; nodeColumn remains the exact terminal column name and ParentPolicy its original closed domain. Canonical calls, action calls and corresponding prepared-action members share the same complete native call, without another old arguments/context copy. FEATURES §§6–7 solely define complete Recipe/Construction/TransferPlan/TransferEffects parameter/value/resource/loss/source-subject/effect-slot shapes. Core references these whole new payloads instead of inferring parameters from old Recipe or SOURCE Fields or duplicating an effect decoder.

Under these finite mappings and named owners' complete decoders, valid new instances have a positive prepare→original planning CAS→single P→receipt/publication path, not permanent owner_update_required placeholders. Unknown role/version or incomplete evidence makes the affected operation unavailable. Language-independent Resource/structure/trust/Money/stop operations retain generic current types without unnecessary wrappers.

Native Query requests/pages/Views select FEATURES' complete TypstQueryResult1/TypstQueryType1/TypstQueryV1/TypstTerminalSchema1/TypstViewValue1, without widening old D7 types/values. Generic result handle/cursor/epoch/cut/authorization/budget lifetime retains one Core producer; only authorized complete native Query creates the result. New render/export resultPins use the record below and View slots use TypstViewValue1. Old results retain real D7ResultPin/ViewSpec1; do not mix pages or guess profiles by member names. Exact query_json uses FEATURES' complete weftext.typst-query-result-export1 schema/data. Static Field export needs no whole-body compilation/read.

```text
TypstResultPin/1 = {
 kind:"typst_result_pin",version:1,profileCarrier:TypstProfileCarrier/1,
 resultToken:Token,producerEpoch:Token,principalAudienceToken:Token,
 authorizationGeneration:Token,frontier:Frontier/2,
 schemaPin:PinRef/2,dataPin:PinRef/2,dependencyProof:DependencyProof/4
}
```

Schema pin binds complete TypstTerminalSchema1; data pin decodes complete rows/scalar/graph under that schema, preserving bags/order/exact native values. Protected resultToken selects the same producer epoch and complete input/cut/pins, matching current authorization and every positive/negative dependency. It grants no caller authority. Render/export consumes this record and matching View without rerunning Query, extra reads, omitted null/array items or approximate float/date values. Invalid profile/stale/missing pins rejects wholly; original pins/domains recover unchanged.

### 4.1 Complete effects and Annotation construction

effect_manifest retains every EffectManifest3 member with the exhaustive items below. effect_bytes retains exactly EffectBytes3's handleToken/encoding/byteLength; only this matrix's encodings are legal. Complete exact bytes bind length/epoch/phase/pin. Source images retain the exact three SourceImage2 variants and D7ProposedVersion1, replacing only bytes with the new effect_bytes. Production/proposed versions agree with subject/map/source plan.

| Rank / kind | Exact new payload, encoding and phase |
| --- | --- |
| 0 source_change | Original kind/subject/before/after. Present Document preimage/committed after only exact_source_utf8; Resource only resource_bytes; Annotation only typst_annotation_value1. Preview proposed Node/Annotation may use their construction encodings, never parsed as present. |
| 1 conditional_source_change | Original full before/result/versionBasis/forceAdmission. Preview only, existing Node, complete typst_document_construction1 with an actual transfer-conditioned source-state branch. After both passes true no-op omits committed item; otherwise exactly one real source_change. Old C/H is invalid. |
| 2 entity_state_change | Original complete EntityImage/subject structure/lifecycle matrices. |
| 3 d3_plan / d3_receipt | Original phase singleton; request is identity_request envelope. Receipt is the byte-equivalent result of its full request domain, never switched to old wire. |
| 4 semantic_extension | Committed only, using the complete encoding matrix below with secondary ranks1/2/3. Retire complex-fact relation-copy without fake padding; actual source materialization/definition transfer still provides complete plans/results. |
| 5 period_scope_change; 6 series_configuration_change | Original complete bindings/configuration and sorting. Canonically sort complete actual SeriesScope after identity materialization. |
| 7 workspace_bootstrap | Preview only typst_symbolic_bootstrap1; committed only typst_bootstrap_plan1, decoding the complete bootstrap_plan envelope below. |
| 8 authority_change | Original complete AuthorityImage and preview-pending/committed-real rules. |
| 9 field_change | Original owner/Field/beforeVersion/afterVersion/before/after members. Both bytes use typst_field_image1 and complete TypstFieldImage1 below, without Entry UUID. owner_fields only. |
| 10 conflict_branch_source; 11 conflict_resolution_change; 12 canonical_plan | Original full heads/canonical-plan/control rules. Actual source/Node/Annotation results use rank0 encodings; language payload pins select proposed_input envelope; no omitted branch. |

Reuse exact secondary comparators from original effects owner §4. Retire rank13 presentation_policy_change without renumbering. full and owner_fields are exclusive: owner_fields contains all actual authorized Fields at rank9 only, after proving complete author-effect coverage. Do not disclose unselected source/Fields. True bytes/value/state no-op yields empty items and original sourceVersions; equal-byte external admission is not no-op. Before is actual physical preimage, not selected historical head. Preview refinement has only the original conditional exception and cannot add unpreviewed effects.

```text
format -> encoding (committed only; secondary rank)
typst_source_materialization_effects/1 -> typst_source_materialization_effects1 (1)
typst_definition_transfer_effects/1 -> typst_definition_transfer_effects1 (2)
typst_canonical_effects/1 -> typst_canonical_effects1 (3)
```

```text
TypstFieldImage/1 = {
  kind:"typst_field_image",version:1,profileCarrier:TypstProfileCarrier/1,
  ownerNodeRef:NodeRef,fieldId:FieldId,
  sourceVersion:SourceVersion/2|D7ProposedVersion/1,
  field:SOURCE.TypstSelectedField/1
}
TypstSourceMaterializationEffects/1 = {
  kind:"typst_source_materialization_effects",version:1,decisionKey:DecisionKey/2,
  owners:[{ownerNodeRef:NodeRef,before:SourceVersion/2,after:SourceVersion/2,
    beforePin:PinRef/2,afterPin:PinRef/2,
    fields:[{fieldId:FieldId,beforeImagePin:PinRef/2,afterImagePin:PinRef/2}...],
    transformEvidencePin:null|PinRef/2}...]
}
```

SelectedField is the complete exact selected Field state/native value/rawFieldSource without absolute ranges/AST paths/other-field positions. Full internal Projection stays protected. Image FieldId/version matches field_change and actual source; arrays are never truncated, equal items reordered or raw reserialized. Materialization effects sort uniquely by complete owner/FieldId and cover every actual materializer owner, including internal no-op owners with equal versions/empty fields without fake public source effects. Plans/results match same-decision receipt/CP4/afterPin one-to-one; full source evidence requires full authorization. FEATURES §6 solely owns TypstDefinitionTransferEffects1; rank4 uses its complete closed transfers array, not another same-named plan/results object. typst_canonical_effects1 is a new nominal domain with exact D3CanonicalEffects1 members, applying finite mappings only to plan/source payload/effect bytes. Original fingerprint/receipt/source-version/full-origin rules remain; old decoders do not expand.

TypstSourceMaterializationEffects1 applies only to real existing-source writers. Fresh/copy/template target preimage is absent; complete initial Fields/constraint gates are covered by TypstDocumentConstruction1 and complete source_change/fresh version basis. Do not fabricate an existing-only extension, prior version or Observation. An existing absent Field uses SelectedField.absent with an exact before-image pin; a fresh Node needs no beforeFieldImage. Annotation construction encoding is only for D3 same-map identity construction; D6 existing edits use complete concrete Value1.

### 4.2 Actual D10 closed ancestors

These complete named ancestor replacements do not insert envelopes into old closed ControlDependencies3:

```text
ControlDependencies/4 = all ControlDependencies/3 members; version=4;
 workspaceReads:Option<FormatAdapterEnvelope/1(role=workspace_reads)>
ControlPrepareBinding/4 = all ControlPrepareBinding/3 members; version=4;
 dependencyPins:ControlDependencies/4
D10ControlInput/3 = all D10ControlInput/2 members; version=3;
 dependencies:ControlDependencies/4,
 effectPlan:FormatAdapterEnvelope/1(role=control_effect_plan)
```

Old outers retain original decoders. New prepare backing is Binding4; canonicalIntentBytes binds complete Input3/dependencies/effectPlan. OwnerInputBinding2 retains D10 ownership with owner kind/intent d10_control/3 and exact canonical Input3. originalCommitRequest remains PreparedCommitRequest1; single commit/confirmation/stop/claims/Money/recovery algorithms stay. The new nominal versioned prepare binding admits schema=d10_control_prepare_binding/4,value=Binding4 or actual original1/2/3 arms; old Versioned1 does not expand.

Language-related record ancestors select the new carrier below without fictitious flat members. Substitution is by statically declared type, never runtime member-name scanning. Original config/binding/usage/scope/view/attribution/time/stop/cost slots remain. Unaffected records may retain their actual original arms. Complete pinned bytes correspond to the record; reject unknown roles or wrong wrappers.

```text
D10FormatControlCarrier/1={kind:"d10_format_control_carrier",version:1,
 profileCarrier:TypstProfileCarrier/1,role,payload:S(Base(role))}
role -> Base(role)
image -> D10ControlRecordImage/1|/2
pin -> D10ControlRecordPin/1|/2
range -> D10ControlRange/2
claims -> D10ExecutionClaims/2
author_step -> D10AuthorStepResponsibility/2
subscription -> ScheduleSubscription/2
occurrence -> AutomationOccurrenceRecord/2
schedule_witness -> ScheduleContinuityWitness/2
schedule_step -> ScheduleContinuityStep/2
schedule_invalidation -> ScheduleContinuityInvalidation/2
S(RecordImage activation arm).supplement.registrySnapshot/evolution -> SOURCE new types
S(RecordImage run arm).supplement.authorSteps[] -> carrier(author_step)
S(RecordImage automation arm).supplement.subscription -> carrier(subscription)
S(RecordPin).image -> carrier(image)
S(ControlRange occurrences arm).records[] -> carrier(occurrence)
S(ControlRange records arm).members[] -> unchanged ControlRef (not record bytes)
S(ScheduleSourceBinding recurrence arm).initial/checkpoint -> recurrence_evidence envelope
S(ScheduleOccurrenceProof recurrence arm).evidence -> recurrence_evidence envelope
S(Witness/Step/Invalidation): every statically typed ScheduleRecurrenceEvidence/2 slot -> recurrence_evidence envelope
S(Claims).recordPins/prepareBindings/authorSteps/subscriptions/occurrenceRecords/ranges ->
 carrier(pin)/new VersionedBinding4/carrier(author_step)/carrier(subscription)/carrier(occurrence)/carrier(range)
S(AuthorStep interactive arm).authorRequest.request -> identity_request envelope|original commit2
S(AuthorStep interactive arm).preparedFormat -> "typst_prepared_action1"|"typst_prepared_edit1"
S(AuthorStep core_field_member arm).link -> nominal original Link2 with new prepared_action backing
```

The finite substitution also applies to ScheduleSourceBinding/ScheduleOccurrenceProof inside these base types and this section's declared record-wrapper ancestors; all unlisted members/legal values remain exact. Dependencies4 recordPins/controlRanges, effect-plan changes/recordPins, Binding4 preview/claim pins and matching Inventory typed evidence explicitly select new carriers or actual old arms. New versioned schema tags are exactly d10_format_control_<role>/1 with corresponding complete carrier values, never additions to old Versioned1. Continuity artifacts use D6-Typst-Schedule-<Witness|Step|Invalidation>/1 plus NUL and complete carrier bytes, within original capacity/producer/permanent-gap/same-P semantics. Old artifacts are not reencoded.

This finite typed-schema replacement defines all affected ancestors once. Old Record/Pin prefixes/signature bytes remain unchanged. New carrier canonical bytes include profile/role and use D10-Format-Control/1 plus NUL, never old Record/1 prefix. Unaffected callers need no control-storage change; type closure grants no package/plugin control authority.

typst_symbolic_bootstrap1 is exactly `{kind:"typst_symbolic_bootstrap",version:1,template,slots}`. Template is the complete bootstrap_plan envelope, permitting symbolic_subject only at its actual typed fresh NodeRef positions. Slots are the complete path/subject array. Literals/Workspace/authority/handles/Policy never substitute. Use the original member-UTF8/Counter-index comparator; reject duplicates, ancestor overlap and missing/extra slots. After the same map, sort only actual initialSeriesConfigurations/periodScopeBindings with original comparators, then validate the new plan without initialPresentationPolicy. Committed typst_bootstrap_plan1 binds the complete materialized envelope. Preview is not mutation/evidence and discloses no preallocated IDs.

typst_annotation_construction1 decodes exactly TypstAnnotationConstruction1 with kind=typst_annotation_construction, version1, profileCarrier, subject, template, refSlots, revisionSlots and bodyReferencePlanPin. Subject is Annotation existing/mapped/new. Template is a nominal symbolic EDITOR TypstAnnotationValue1: only real typed Ref positions in target/replyTo/locators allow symbolic_subject; all other Value members and actor/time freezing rules remain. refSlots exactly enumerate path/subject substitutions. revisionSlots enumerate locator production revisionToken path/sourceSubject substitutions with Core proposed-token placeholders, never arbitrary literals. Optional bodyReferencePlanPin binds complete before-body pin and official AST inline-link string edits: exact token range, original decoded address and targetSubject. No arbitrary source fragments; Core encodes the whole safe native string using the same map, preserves other body bytes and fully reparses inline content. Typed paths and entity kinds match; each sourceSubject has a same-plan source revision/token and SOURCE validates final coordinates. Only Core builds this from authorized copy preimages or validated proposals and trusted actor/time, not caller attribution.

One stage12 map rewrites fresh AnnotationRef, target/reply and typed inline links. Same proposed token/two-pass final coordinate validation produces complete Value1, then one P saves aggregate/version/token/receipt. Existing concrete edits need no symbolic input. Fresh-copy target/reply has a positive path without expanding Result9's Value4 decoder; old Result9 is recorded recovery only.

### 4.3 Complete current scheduling proof chain (TR-08)

The complete new nominal payload below replaces the original ScheduleRecurrenceEvidence2 Proof3 slot. Its outer is §4's complete FormatAdapterEnvelope1, role=recurrence_evidence. Inner kind/version remain structural identifiers decoded only in this new outer domain. The actual member is dependencyProof, with no proof alias.

```text
Payload(recurrence_evidence) = {
 kind:"d10_schedule_recurrence_evidence",version:2,
 observation:SourceObservation/1,sourcePin:PinRef/2,metadataPin:PinRef/2,
 dependencyProof:DependencyProof/4,
 registrySnapshot:SOURCE.TypstSchemaSnapshot/1,
 registryEvolution:null|SOURCE.TypstSchemaEvolution/1,
 recurrenceContext:RecurrenceReadContext/1
}
CurrentRecurrenceEvidence = FormatAdapterEnvelope/1(role=recurrence_evidence)
S(ScheduleContinuityWitness/2).initial/checkpoint = CurrentRecurrenceEvidence
S(ScheduleContinuityStep/2).before/after = CurrentRecurrenceEvidence
S(ScheduleContinuityStep/2).dependencyBefore/dependencyAfter = DependencyProof/4
S(ScheduleContinuityInvalidation/2).evidencePins = [PinRef/2...]
```

Invalidation retains its complete original member set without invented dependencyProof/before/after. Evidence pins retain actual binding_changed/gap evidence; pins naming new evidence/steps decode in the new domain. Other Witness/Step fields, Step.portableChanges ChangeRecord1/Notice3/CP4 triples, retainedInputs and original state machine remain. Old generation2 Proof3 decoders do not expand.

| Actual current producer/consumer slot | Complete type and cross-check |
| --- | --- |
| Registration→ControlDependencies4.workspaceReads | workspace_reads proof4 matches same-configuration D10ControlInput3.dependencies, ControlPrepareBinding4.dependencyPins and InputDescriptor4/PreparedIntent4. Actual observations enter sourceInputs; keys/stamps correspond to controlInputs. Registration remains the original configuration transaction, not missing-file inference. |
| ScheduleSubscription.source recurrence arm | initial/checkpoint are CurrentRecurrenceEvidence in carrier(subscription). Initial binds registration cut; checkpoint binds the last continuously validated cut. Do not rewrite historical cuts equal. |
| carrier(schedule_witness).payload | initial/checkpoint select complete new evidence; original revision1/transition0/fresh producerEpoch remain. Only the registered source/control-transition producer updates checkpoints. |
| carrier(schedule_step).payload | before/after are complete new evidence. dependencyBefore is byte-equal to before.payload.dependencyProof; dependencyAfter equals after.payload.dependencyProof. Each binds actual cut, Workspace/domain, observations, metadata, schema/evolution, rules/authorization and positive/negative ranges, without resampling. |
| AutomationOccurrenceRecord.proof recurrence arm | evidence selects CurrentRecurrenceEvidence; outer ScheduleOccurrenceProof is not DependencyProof. Complete projection/originalStart/due/key retain contracts; carrier(occurrence) reaches ControlRange occurrences.records[] through §4.2 wrappers. |
| carrier(claims).payload | subscriptions/occurrenceRecords/ranges select matching current carriers, retaining every continuity artifact/actual record pin. Automation supplement subscription, run supplement authorSteps and Inventory evidence follow §4.2; claims gains no fictional flat dependencyProof. |
| D10ControlInput3→author/control prepare→single P | dependencies is ControlDependencies4, effectPlan the profile envelope; descriptor/preview/backing/pin directory use generic4 throughout. Commit2 remains; registration/subscription/producer association/retention/records share one original transaction without another ledger. |

Each source pin binds observation's complete bytes; metadata pin selects actual protected metadata of that version. Schema/evolution match real registry dependency/pins. Proof4 covers actual source/document_format/registry/rule/authorization reads; compile_world is present only when consumed. Omitting it cannot authorize compilation; including it cannot justify lowering proof to3. Unknown schema, partial metadata, gaps and denied access cannot become continuous steps.

RetainedInputs/invalidation evidencePins/consumer pinDirectory collect the sorted unique union of before/after source/metadata pins, all Proof4 evidence pins, schema/evolution pins, actual rule/auth/format/World evidence, portableChanges triples and original continuity/preview/proposal/recovery references. Deduplication never drops artifacts or substitutes digest/current files for historical before evidence. Canonical payloads/protected pins retain both complete proofs, without free duplicate objects. Validate artifact §4.2 prefix/NUL/full-carrier kind/role/profile/length/digest; reject faux old domains. Original4096-pin/16MiB evidence limits, separate source PinBudget, checked advance/reference-safe retention remain. Failure to retain the next step first records permanent gap/invalidation in the same transaction, then unrelated work may continue.

Genuine old Subscription/Witness/Step/Invalidation in original1/2 domains with Proof2/3 recover recorded decoders/pins, not new payloads or reencoded4. Original D6 §7.2.1/D10-CONTROL §0.2 Subscription1→2 continuation and mixed retained history preserve their original producer, initial/checkpoint, continuityPins and artifact domains. Those clauses require complete retained history without format/rule/business discontinuity; this does not invent a3→4 migration bridge. A Typst format/binding change is actual discontinuity: original control transitions reject same-generation continuation or explicitly replace/register a new generation with current4 evidence. Upgrades never modify old chains; missing history is not empty. Real saved/planned/unknown and actual old continuation associations fulfill original operations. No new bridge, regenerated producer or revived invalid generation exists.

EDITOR §8.1 solely owns TR-07 Annotation complete eligibility. Core consumes complete/strict along pinned intent, edit_input envelope, InputDescriptor4.saveProfile, PreparedIntent4 installation protection and original commit2. Ordinary Annotation creates no new plan and retains Draft; ordinary Document save keeps its matrix.

## 5. Exact source, Frontier, map and pins

SourceVersion2 keeps managed/external variants: managed producing domain/observationEpoch/revision/changeId never mix with external domain/observationEpoch/externalSequence. Current SourceObservation1 separately stores observerDomain, entityRef, complete production sourceVersion, current observationEpoch, FileObjectBinding1 and evidencePins. Observer domain equals the operation domain; producer may differ. Watcher gaps, ABA, external/object replacement and unproved rematerialization invalidate observations despite equal bytes/digest. SourceVersionRef1 selects a complete protected observation, not a version number.

Frontier2 has only kind/version/heads. Validate Workspace at heads[i].commitDomain.workspaceRef, never nonexistent frontier.workspaceRef or heads[i].workspaceRef. Empty heads still requires outer descriptor/proof Workspace/CommitDomain, current cut and complete proved-empty continuity. Exact managed_atomic and every mandated full equality remain. scope_dependencies admits only complete continuous, nonregressing sealed extensions proved unrelated to every bound positive/negative range; vector numbers and index readiness are insufficient.

Original SourceRevisionPlan1 ordinary/fresh, D3 canonical-conflict-only2 and D6 source-conflict3 matrices remain. Prove H(D,E) from that producer's sealed history; missing history is not zero. Real source changes checked-increment H; fresh proved-empty begins at1; observer epoch changes do not reset H. True raw no-op, source-unchanged control/structure and deletion create no managed after revision. Equal-byte external admission is a real change. One winning planning CAS freezes exact afterPin, before observation/original conflict wrapper, last-issued/empty history, map, stamp, token, format/schema/world/transfer and installation plan. Only seal allocates one ChangeId, combines stamp into SourceVersion and signs original revision-token/source-transform artifacts with the original dual trust profiles and real whole-source before digest. Prepared tokens/spans are not current persistent addresses.

## 6. Official compilation environment and authorized World

```text
TypstProcessorEnvironment/1 = {
  kind:"weftext_typst_environment",version:1,
  profileCarrier:TypstProfileCarrier/1,
  compilerBuildSha256:Digest,standardLibrarySha256:Digest,
  syntaxBuildSha256:Digest,projectionBridgeBuildSha256:Digest,
  target:"html"|"paged",
  date:null|text,timezone:"UTC",
  packageLockPin:PinRef/2,fontCatalogPin:PinRef/2,
  resourcePolicyPin:PinRef/2,budget:TypstCompileBudget/1
}
TypstCompileBudget/1 = {
  sourceBytes:Counter,totalResourceBytes:Counter,resourceCount:Counter,
  importDepth:Counter,decodedPixels:Counter,fontBytes:Counter,
  pluginFuel:Counter,pluginMemoryBytes:Counter,
  workerMemoryBytes:Counter,wallTimeMillis:Counter,outputBytes:Counter
}
TypstWorldProof/1 = {
  kind:"weftext_typst_world_proof",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,frontier:Frontier/2,
  environmentPin:PinRef/2,authorizationProofPin:PinRef/2,
  reads:[TypstWorldRead/1...],complete:true
}
TypstWorldRead/1 =
 {kind:"workspace_source",request:text,outcome:"present"|"absent",
  observation:null|SourceObservation/1,bytesPin:null|PinRef/2,
  resolutionEvidencePin:PinRef/2}
 |{kind:"environment_asset",request:text,outcome:"present"|"absent",
   assetKind:"package"|"font"|"plugin",artifactPin:null|PinRef/2,
   manifestPin:PinRef/2,resolutionEvidencePin:PinRef/2}
```

Digest is sha256 plus64 lowercase hexadecimal digits. Authorized exact Rust compiler/syntax/stdlib builds and deployment resource providers produce actual, nonempty, nonguessed digests. Baseline fixes language version; build/target/date/fonts/packages/resource policy are actual dependencies. Null date prohibits system-date reads; otherwise freeze an ISO Gregorian UTC date from the authorized host clock cut, never local timezone. Every budget member is positive; freeze the tighter deployment/caller limit, use checked cumulative accounting and reject overflow. Noninterruptible compiler/plugin work runs in workers with hard CPU/wall/memory termination and no authorized filesystem mount. Exhaustion produces no successful projection/compile; retain Draft/source and diagnostic. Typed pins bind complete package/font lock manifests and actual bytes/digests; latest, machine fonts and downloaded results never join automatically.

Core supplies a snapshot World, not a Workspace mount. Include/import/read/json/yaml/csv/xml/image/bibliography and every data loader share the resolver. Native JSON/YAML legality does not grant paths. First authenticate and minimally disclose entity state, then applicable source/resource read, locator/envelope permissions and deny rules, then canonical resolution and authorized bytes. Reading another Node's primary source requires its source_read; cross-ACL resources require their own owner grant. Public primary-source access does not include private inputs. Unproved permission fails without disclosure, never empty-file/cached-fragment fallback or distinguishable existence errors.

Request is canonical logical text without host absolute paths. Workspace present requires exact observation and bytesPin; absent requires both null and evidence of the complete authorized directory/namespace negative range and trusted absent object. Environment assets never invent Node/Resource identity or observations: present artifactPin selects approved lock bytes/digest; manifestPin proves namespace/version/font face/plugin entry. Absent has null artifactPin and complete authorized lock negative-range evidence. Each manifest/evidence remains subject to resource admission and current authorization. Date exists only in environment. Missing file, placeholder, offline, missing metadata or index miss is not absence. Record every actual attempt, including error branches, package candidates, font fallback/missing glyph selection and negative lookup; sort uniquely by kind rank/request/assetKind canonical bytes. Different outcomes for one request conflict. Untracked attempts prohibit complete=true or narrow/strong proofs. Denied traces remain protected diagnostic/audit material and disclose no hidden paths/counts.

World offers no absolute host paths, escaping parent traversal, symlink/reparse/hard-link bypass, unauthorized unmanaged/ignored reads, environment variables, arbitrary host filesystem or network. Ordinary compilation downloads no packages; only administrator-admitted digest-fixed local locks are usable. Indirect package/font/plugin reads use the same World. WASM plugins get explicit arguments and bounded fuel/memory, without host filesystem/network, secrets or Core actions. WASM is no Core capability. Connectors retain D10 authentication/grant/cost/confirmation/stop/unknown responsibility, produce ordinary resource proposals, then World reads those resources explicitly. Packages may compute but cannot invoke the whole-workspace Query executor or write commands.

WorldProof is exact evidence for the new compile_world key, with original source/authorization/registry/format keys for workspace resources; execution_resource's decoder is untouched. Core's real World observation manager produces complete environment, authorized source/asset positive and negative traces at the same observer domain, owner, target, audience and cut, then records the proof pin and protected stamp. Epoch is the manager's real incarnation; revision checked-advances on any bound positive/negative resource, lock, font fallback, date/environment, source or authorization change. Gaps invalidate old stamps; rebuilding cannot recreate them. Evidence pins cover proof/environment/all reads/manifests/authorization. Prepare, preview delivery, render/export, final barrier/P and receiver-local reads revalidate full proof/current resources; change means stale/reprepare. Static operations need no World key. Saved/planned recovery retains original proof/pins; revocation pauses and missing resources/trace never trigger resampling. Literal Field saving needs static SOURCE evidence, not hidden whole-body reads.

The new narrow-disclosure capability is exactly `{kind:"typst_dependency_state",fieldId:FieldId}`. Policy5 inherits every Policy3 member/grant/deny rule, sets version5 and adds only this capability arm. Old Policy4 AsciiDoc native_dependency_state does not authorize Typst. Only Workspace scope and current exact schema FieldId are valid. Default is no grant; Field/source/envelope/admin/package capabilities never imply it. Original policy_admin transactions explicitly install it. It discloses only aggregate Field isolation eligibility, not body/topology/count/path/other values. Without grant return not_visible before reading native dependency topology. With grant still require original Field read/write, source-write deny/type gates and complete static constraints plus official before/after use and negative World isolation evidence. Unknown/multiorigin rejects and retains Draft; complete Source-authorized paths remain.

Because old Bundle2.policy is closed to Policy3/4, WorkspaceAuthorizationBundle3 inherits Bundle2 members, sets version3 and accepts only Policy3 or Policy5. Trust root/revision/declarations and the Declaration1 prefix followed by Declaration2 suffix remain exact. New policy component bytes use complete canonical Bundle3, with version/Policy5 included in digest/pin; old decoder is unchanged. Consumer slots are bootstrap initialPolicy/target policy component, authorization evidence, D10 issuer/control/bootstrap grants, trust/replica/conflict selected/result bundles and their head/carry pins. Bootstrap may use Policy3 without a narrow grant. New evidence explicitly records bundleVersion3 with CP4/ChangeRecord1 in its new nominal domain, never an old1/2 arm. Original declaration/PoP/root signature domains remain; derive activation from same-decision actual CP4 policy after-image. New bundle version never resigns old declarations or shifts historical cuts. Real saved/planned old policy/bundle/evidence recovers as recorded, not reexecuted because a new grant is absent.

### 6.1 Exact Bundle3 trust consumers

These finite complete successors replace the preceding nominal-domain summary; no adapter roles or old head/hop decoders expand.

```text
PolicyBundleHeadEvidence/4 = PolicyBundleHeadEvidence/3
 | {head:ChangeId/1,completionProofVersion:4,bundleVersion:3}
TrustConflictCarryValidationHop/3 = TrustConflictCarryValidationHop/2
 | {declarationVersion:2,declarationRevision:Counter,
    declarationDigest:Digest,activationChangeId:ChangeId/1,
    completionProofVersion:4,bundleVersion:3,
    changeRecordPin:PinRef/2,completionProofPin:PinRef/2,policyBundlePin:PinRef/2}
TrustConflictCarryValidationEvidence/3 = {
 head:ChangeId/1,factId:Digest,origin:TrustConflictCarryValidationHop/3,
 carriers:[TrustConflictCarryValidationHop/3...]
}
ConflictResolutionPolicyDerivedPlan/3 = exact Plan/2 members/constraints;
 headEvidence:[PolicyBundleHeadEvidence/4...],
 selectedBundleVersion:1|2|3,carryEvidence:[TrustConflictCarryValidationEvidence/3...],
 resultBundleVersion:3
ConflictResolutionInput/4 = exact Input/3 members/constraints; version=4;
 resolution.policy:Policy/3|Policy/5,derivedPlan:ConflictResolutionPolicyDerivedPlan/3
ConflictResolutionPreview/3 = exact Preview/2 members/constraints; version=3;
 resolution.policy:Policy/3|Policy/5,derivedPlan:ConflictResolutionPolicyDerivedPlan/3
```

Exact original Plan/Input/Preview owners are D3/D6-SCHEMAS §9, D6-CONTROL §0.1.4/§9.4 and D6 §17.1. Imported old arms use their exact decoders. New head/hop accepts only CP4/ChangeRecord1/Bundle3, real Declaration2/root/PoP signatures and the original policy image first appending that declaration. Preserve complete direct-origin/every traversed-carrier cuts/pins. Never shift origins to resolver time or keep only a selected safe branch. Complete fold/outcome/compromise inheritance remains.

Actual slots are Plan3.headEvidence/carryEvidence and Input4.derivedPlan/Preview3.derivedPlan; neither outer has carryEvidence. Decode each origin/carriers item by its actual completion/bundle versions. branchEvidence.policyBundlePin remains PinRef2, with complete Bundle1/2/3 selected by its corresponding headEvidence, not recursive pin guessing. Selected/result pins match Plan3's explicit versions. Owner kind/intent is d6_conflict_resolution/4 and descriptor bytes exactly canonical Input4. PreviewBinding selects complete Preview3/pin; PreparedIntent4.pinDirectory covers every branch/selected/result/origin/carrier pin under original union rules. Source merge/choose-head retains original owner input with generic descriptor/proof/prepared4, not policy-only successors.

New bootstrap, trust mutations, replica registration/continuation and D10 custody inspect actual policy components via exact bundle.version3 decoder. New declaration activation uses4/3 evidence and original signatures/dual-profile order. Policy installation uses the complete new request below, not a nonexistent old public policy-set request. Require current policy_admin, complete prior authorization generation, expected revisions and Frontier. Owner kind/intent is typst_policy/1; descriptor bytes are the whole request; sourceInputs is empty. One prepare freezes original policy/authorization dependencies, exact before/after bundle pins and complete preview. New policy-conflict prepare is original §9.4 request wireVersion3 changing only policy to3|5, preserving all members/OperationId/expectedKey and producing Input4. Both finalize through original commit wire2/single P.

```text
TypstPolicyPrepare/1 = {
 wireVersion:1,kind:"typst_policy_prepare",workspaceRef:WorkspaceRef,
 commitDomain:CommitDomain/2,operationId:Uuid,expectedFrontier:Frontier/2,
 expectedAuthorizationRevision:Counter,expectedPolicyRevision:Counter,
 policy:Policy/3|Policy/5,budget:BudgetBinding/1
}
```

New targets begin with Bundle3. Adopting real old Bundle1/2 history requires an authorized explicit import/format-entry intent retaining its chain. Bundle3 installation is a checked authorizationRevision successor, never an implicit equal-value upgrade. Freeze complete result3 after bytes in the plan; only appended declarations advance trustRevision. Genuine saved/planned/unknown, old before.policy4 and CP3/4/5 retain their actual arms. Ordinary reads neither grant Typst from Policy4 nor create migration decisions.

Seed production is authorized issuer-configuration admission, not SOURCE prose, metadata names or missing-file inference. It supplies complete SOURCE snapshot/binding/qualification and all real pins. Derive creator Field grants from that exact admitted seed; install its exact bytes in one bootstrap plan with proven target empty history/custody. Built-in contacts/tasks use SOURCE's named complete seed constructor; schema-admin evolution installs other definitions. Missing seed/authority/complete evolution rejects bootstrap, never treats unknown as empty. Fork validates and preserves complete source-schema history instead of masquerading as default-seed create.

## 7. Projection, revocation, caches and recovery

```text
TypstSourceEvaluation/1 = {
  kind:"weftext_typst_evaluation",version:1,
  qualification:TypstDocumentQualification/1,
  environment:TypstProcessorEnvironment/1,
  control:TypstRootDeclaration/1,fields:TypstFieldProjection/1,
  body:TypstBodyProjection/1
}
TypstSourceSnapshot/1 = {
  kind:"weftext_typst_source_snapshot",version:1,
  evaluation:TypstSourceEvaluation/1,exactSourcePin:PinRef/2,
  evaluationPin:PinRef/2
}
TypstBodyProjection/1 =
 {state:"available",target:"html"|"paged",worldProof:TypstWorldProof/1,
  compilationEvidencePin:PinRef/2,semanticProjectionPin:PinRef/2,
  originEvidencePin:PinRef/2,outputPin:PinRef/2}
 |{state:"unavailable",reason:"source_error"|"resource_unavailable"|
   "unsupported_projection"|"budget_exceeded",diagnosticPin:PinRef/2}
TypstCompilationEvidence/1 = {
  kind:"typst_compilation_evidence",version:1,
  sourcePin:PinRef/2,environmentPin:PinRef/2,worldProofPin:PinRef/2,
  semanticProjectionPin:PinRef/2,originEvidencePin:PinRef/2,
  outputPin:PinRef/2
}
```

The official compiler produces Content, element semantics, reading order and origins. EDITOR's TypstSemanticProjection1 is the complete canonical artifact. Official Content exists only inside the controlled process; do not invent an official wire serialization or pretend an in-memory object is pinned bytes. CompilationEvidence records all actual pins from one Core official evaluation/realization and the final converged introspector iteration. projectionBridgeBuildSha256 binds the actual EDITOR-specified official-hook bridge; independent consistency scenarios cover show, loops, rebinding, tables and heading origins. flow-semantic is a presentation backend, not an invented Target: it uses official Paged; html-experimental uses official Html. Valid content unsupported by flow still permits source/paged preview with that presentation capability unavailable. Ordinary reading/editing requires EDITOR acceptance. Projection is neither modified AST, equal-string matching nor another interpreter. Unavailable body never presents old HTML/PDF as current and does not block valid static controls/Fields or retained Draft. Diagnostics disclose authorized scope only. Generated/imported/multiorigin/show-transformed output is not automatically writable.

Evaluation pin binds the complete evaluation; source pin binds qualified exact whole bytes; body pins/WorldProof share environment/qualification/cut. Compile success does not prove birth, Field write authority, save or publication. Cache keys include production/current source observation, profile binding/stamp, build/target/date, full package/font/resource policy, all positive/negative dependencies and principal authorization generation. Source hash alone is insufficient. Revocation/deny/ACL changes linearize at Core/Server and invalidate affected snapshots/Query/index/cache/editor maps/download qualifications. Every cached use, HTML/image/thumbnail/search snippet/download/prepare and P checks current principal and true dependencies. Cached authorized inputs confer no access to another principal. Revoked responses cannot continue serving old source/resource bytes. Revocation before seal pauses authorization; seal first preserves the committed fact, with current disclosure still applied.

Index rebuild/recompile acquires new current observations, complete WorldProof and permissions; it does not revive old selectors/Drafts/maps/preparations. Package/font/negative range/environment changes stale their actual consumers, leaving unrelated ordinary saves available. Backup preserves original bytes/portable authority; restore validates recorded decoders and admission. Synchronization transports original source/format/metadata/Notice/CP/ChangeRecord/outbox only. Partial arrival is incomplete; receivers obtain their own observations/resource qualifications instead of sender tokens/compile authority.

New Typst run-in and author presentation controls change target-scoped native show rules in the same `.typ`, using shared Draft, exact patch, ordinary source prepare/Notice3/CP4/ChangeRecord1, P, synchronization and recovery. Device reading preferences affect layout only. Explicitly retire new presentation_policy_change effects, independent presentation Set and bootstrap initialPresentationPolicy: no new sidecar/head/outbox or old owner synthesized for fresh Workspaces. Delete that arm from the new effect payload; preserve other ranks without renumbering. Bootstrap still atomically commits root/Registry/Policy/two trust keys/all source-format/series/period/activation, without CP5 owner records. Genuine old presentation saved/planned/unknown and Notice4/CP5/ChangeRecord2/head/receipt/outbox recover under their recorded contracts; that is historical fulfillment, not another new presentation writer.

## 8. Real history and precise replacement paths

Recovery first applies original minimal disclosure, recorded authorization/custody/domain/P continuity. Locate original DecisionKey, complete canonical fingerprint and protocolOwner, then deliver/publish saved results, recover the exact planned plan or preserve unknown responsibility. Only unseen selects the new profile/adapter. Current source/schema/World unavailability cannot turn saved into new execution; different fingerprint under the same key remains operation_id_conflict. Never reconstruct old r5, missing P, receipt/token/Money/external results from current bytes. Real CP2 committed/restored keeps its twelve-kind component union and SourceVersionRef1, forbids document_format and reencoding/upgrading. Missing Policy/trust heads follow original recovery, never current-head supplementation.

No undeployed design version is asserted to contain runtime records, and no permanent migration bridge/database is automatically created. Proven actual saved/planned/unknown versions alone retain their original decoders/bytes/TTL/last-reference/no-duplicate-effect duties. Other old candidates remain read-only design input; a new design need not deploy them. Importing real old data is a new explicit intent.

| Exact old owner/section | Disposition and new location |
| --- | --- |
| Root AGENTS Storage; D1 §§3,5–7,9,12 | Product .adoc remains prototype; target inventory §1. Single holder/five surfaces/Server fencing/ordinary sync/non-disclosure/milestones retain §§4–8, without broader release claims. |
| AUTHORING-INTEGRATION §§5,6,6.1; D3/D6-SCHEMAS §5 | Replace AsciiDoc profile/binding/environment/H production with §§2,3,6,7. Old types do not expand. |
| D3 §§4.7,11.1,14–15,21.2; D3-SCHEMAS §§6–8 | Production/current versions, map, exact Frontier, pins/transfer retained §§3–5; language input/action/Result slots use §4. EDITOR owns new Annotation values and proposed/effects/receiver decoder; old Value4 stays unchanged. |
| D6-CONTROL §§0.1.1–3,3–9,15,17.8; D6 §§4–12 | Generic schemas, one transaction and error order retained §§4,5,8; format-byte consumer explicitly becomes §2 qualification. |
| D6-CONTROL §0.1.4,10.2,20; D10-CONTROL §12; D3/D6-SCHEMAS §9 | Retire new presentation effect/Set/initialization §7; real old recovery remains. Bootstrap Registry/format slots change §4; root/dual trust/PoP/activation/compromise/no partial prefix remain. |
| D6-CONTROL §§18–19; D6 §§15–17; D3-SCHEMAS §8 | Exact before digest, total mapping, dual signatures/outbox retained §5; new AST/transfer source §3 does not reinterpret old coordinates. |
| D10-CONTROL §§0.1–0.3,4,6–8,10–12,14,16; D3-SCHEMAS §10 | Claims/Money/approval/external unknown/stop/custody remain with original owner. Actual workspace/Registry/activation/recurrence/author-step language slots follow §4. Compilation packages grant no actions (§6). |

## 9. Original21 impacts and new acceptance

Original21 closure belongs to the original input, not Typst acceptance. IDs below have prefix A2-GLOBAL-6B13-. These reopen Core-related design duties; authors do not close them independently.

| Original items | Typst disposition and scenarios |
| --- | --- |
| P1-01,P2-14 | Same-map complete Document control/content/typed transfer §3, TC-01/02; old H/Q not transplanted. |
| P1-02,P2-15 | Explicitly retire independent presentation current path and replace with native-show source commit §7, TC-03; retain other canonical effect ranks and real old owner recovery. |
| P2-01,P2-03,P2-12 | Exact adapter definition transfer and actual nested Frontier/source bindings §§4–5, TC-02/04. |
| P2-02,P2-07 | Real history, both CP2 arms and failure duties retained §8, TC-05. |
| P2-08,P2-09 | Dual genesis, mixed declaration signature domains and complete activation retained §§5,7, TC-06. |
| P2-10 | Owner/slot replacements and unchanged obligations above; TC-07 verifies complete mapping rather than one retained statement. |
| P1-03,P1-04,P2-05,P2-06 | SOURCE/EDITOR own native heading/full locator/float/table replacements; Core supplies shared profile/cut/environment, TC-08. |
| P1-05,P1-06,P2-11,P2-13 | FEATURES/EDITOR own Trash read-only search/saved bindings/media annotations/hidden quote boundaries; Core retains lifecycle/authorization/typed-pin reads, TC-09. |

| New ID (all product UNRUN) | Positive path and rejection |
| --- | --- |
| TC-01 | Create/copy/template body, duplicate contacts, Node/Resource/Annotation/reference through one map/CAS/P and complete after. Reject duplicate declarations, map/ID mismatch, missing birth/format, partial metadata and UUID resampling. |
| TC-02 | Control insertion changes offsets; native definition/reference passes agree with final sourcePin. Reject self-reference, overlap, mixed profile, old H/single-line Q guessing and missing typed transfer. |
| TC-03 | Run-in edits exact scoped show rule with one source commit/sync/recovery; fresh bootstrap has no presentation initializer/CP5 owner while source/series/dual trust/activation share P. Recover real old zero-component/mixed presentation history; pre-P/P failures produce no committed state; post-P only retransmits original bytes. |
| TC-04 | Exact nested Frontier Workspace, proved-empty, different producer/observer, equal-byte external admission and format-only stale. Reject fictional flat fields, guessed empty H, extension gaps/changed positive-negative ranges and ABA. |
| TC-05 | Real saved r5/current r6, planned/unknown, CP2 committed/restored and missing historical Policy head recover original operation/decoder/pins without resigning, supplementing heads or charging. Third-state/missing P cannot fabricate terminal. |
| TC-06 | Two staged target keys, two Declaration2/PoP/root signatures and same-change activation; full losing-compromise fold and dual-profile continuation. Reject single prefix, missing key, old signature domain and regenerated root. |
| TC-07 | Every producer→consumer slot preserves carrier/pin/schema; reject fictional flat RootSource, wrong version, missing binding/digest and mixed old/new holders. |
| TC-08 | Full World proofs for include/read/json/yaml/package/font/plugin, cross-ACL and negative dependencies. Reject hidden existence probes, path/symlink escapes, cross-principal caches, implicit downloads, missing fallback fonts and budget exhaustion; retain Draft. |
| TC-09 | Linearize revocation with prepare/render/download/P; cache/index rebuild never revives proofs. Static root Fields survive legal body errors without old previews masquerading as current; reject hidden quote, Trash writes and duplicate execution. |
| TC-10 (TR-08/PF-03) | Authorized registration descriptor/proof/prepared4→subscription initial/checkpoint→witness→exact step before/after proof4→occurrence.proof.evidence→claims→D10ControlInput3 succeeds completely. Reject missing source/schema/format pins, proof3 inside new envelopes, mismatched step proofs, missing continuity/portable-change pins, wrong domains and gaps. Real old proof3/artifacts/already-qualified mixed chains recover saved/planned/unknown unchanged. Typst format discontinuity follows original control rejection/explicit replacement with a new generation, never an invented3→4 bridge or same-generation laundering. |
| TC-11 (TR-07/PF-04) | EDITOR complete/strict Annotation body edits/reattach/reconfirm and hidden-target reject follow real envelopes→generic4→commit2/P; apply retains original two-entity atomic chain. Ordinary/observed-only Annotation rejects before prepare and retains Draft; a caller annotation flag cannot downgrade qualification. Ordinary Document saving remains available. |

Document checks establish documentation consistency only. These scenarios need real official-language, conditional-installation, crash-injection, two-replica, Server ACL and hard resource-isolation implementation evidence.
