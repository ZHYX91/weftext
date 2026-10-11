---
source_language: zh-CN
translation_of: AUTHORING-INTEGRATION.zh-CN.md
translation_status: synced
---

[简体中文](AUTHORING-INTEGRATION.zh-CN.md)

# Authoring integration: exact sources, versions and consumers

Status: this is the normative integration part of [AUTHORING](AUTHORING.zh-CN.md), still an unimplemented design candidate. Only the named current slots below are replaced; true recorded objects and unlisted semantics remain. It is not another Field/schema catalog and does not automatically close the original 21 global findings.

## 1. Explicit inner dispatch and historical domains

Identify the real outer kind/version first, then choose inner decoders only at the enumerated typed slots, never by recursive field search or latest-version guessing. New ActionSpec3, Snapshot2, Binding2, Recipe3 and Environment4 carry explicit versions. Holders retaining their outer shapes select the new or original profile using the listed inner discriminator; original closed arms, pins/bytes and recovery responsibilities remain unchanged. Saved/planned/unknown first resolve their original decision and arm, never re-encoding for current software. Unseen requests must meet the current profile; missing new version fields cannot downgrade. Empty unused slots do not invent a version requirement.

New syntax adds no permission, ledger, CAS, Notice or CP. Protected canonical descriptors, complete previews, original source/registry proofs and final read barriers bind the exact same inner objects. Unknown or illegally mixed arms reject; parseable JSON is not complete schema validation.

## 2. Real native Field selections and effects

```text
FieldSelector/2 =
  {kind:"entry",owner:NodeRef,fieldId:FieldId,expectedRevision:Counter,
   occurrenceKey:UUID,rawEntrySource:text}
 |{kind:"native_header",owner:NodeRef,fieldId:FieldId,expectedRevision:Counter,
   occurrenceKey:UUID,rawAttributeSources:[text...]}

FieldSource/3 =
  {kind:"absent"}
 |{kind:"entry",entries:[{occurrenceKey:UUID,rawEntrySource:text}...]}
 |{kind:"native_header",occurrenceKey:UUID,rawAttributeSources:[text...],
   resolution:{state:"value",value:TypedValue}|{state:"absent"}}

FieldSourceImage/3 = {
  format:"weftext.field-sources",version:3,ownerNodeRef:NodeRef,fieldId:FieldId,
  sourceVersion:SourceVersion/2|D7ProposedVersion/1,source:FieldSource/3
}
```

All objects are closed. Entry raw is the complete original Entry JSON. Native rawAttributeSources contains every set/unset event for the bound Field name in the same root header, in actual author order, preserving each event’s exact UTF-8 text and own line ending without intervening other fields/body. The same-cut authoringBindings determines the name; callers supply no second name. Absolute sourceRange stays in internal NativeFieldOrigin so Field-only output does not disclose hidden preceding lengths. Repeated assignments produce one effective fact; absence has no selector. FieldSource.absent means no author carrier for this Field, not absent Node source. Empty Entry collections canonicalize to absent. Nonempty native raw sets retain native_header even after final unset, with resolution.state=absent; that occurrenceKey locates only the carrier and grants no fact selector. Only resolution=value yields a native fact selector. Invalid native values never masquerade as absence or TypedValue; repair uses full Source exact_source_utf8 images without a usable narrow selector. Recipe field_set creating a value from effective absence uses native-absent with its retained raw set as the complete before image.

Selection/read first validates original source_envelope_state, complete Field read and disclosure of actual native dependencies before returning complete raw sets and typed values; submission additionally checks original commit_sequence_state/write rights. Undisclosable substitutions, non-root/generated origins or incomplete dependencies cannot yield a usable narrow selector or leak hidden reasons, offsets or raw. Core relocates and byte-matches all events in the complete source, bound to full SourceVersion/Observation, Registry and positive/negative dependencies. Matching derived keys or raw never proves continuity. Over-budget output fails wholly rather than truncating. An ordinary dependency-free task status must retain a Field-only read/write success path without body-read rights.

The new D7 field-selection open, selection, selection_evidence and action_evidence family uses wireVersion3, retaining exact original members except the version and FieldSelector2 targets; selection_evidence.actionKind additionally accepts set_field_source. Original wire2 retains its original Selector. Tokens retain original runtime audience, TTL, budget, complete source cut and authorization generation; gates are revalidated before occurrenceIndex access, never re-signing old tokens. General Query rows gain no Field lineage.

The current D7 EffectBytes3 encoding union adds only field_sources3, exactly decoding FieldSourceImage3; original field_entries2 retains image2. Current EffectManifest3 owner_fields.field_change before/after both use image3 without mixing generations; each sourceVersion equals its effect beforeVersion/afterVersion. Images expose complete Fields, not diffs or synthetic JSON. Preview after retains proposed basis; committed after is truly sealed. D8 current preview/bytes explicitly accepts this encoding; full Source remains exact_source_utf8 without another effect authority.

## 3. One Field writer and template parameters

```text
D7ActionSpec/3 = {format:"weftext.action",version:3,intent}

new intent:
{kind:"set_field_source",selector:FieldSelector/2,
 replacement:{kind:"native_header",value:TypedLiteral}
            |{kind:"entry",entry:D4Entry}}

TemplateRecipe/3 = {format:"weftext.node-template",version:3,parameters,nodes}
new binding slot:
{kind:"field_set",fieldId:FieldId,carrier:"native_header"|"entry"}
```

Spec3 retains every unchanged Spec2 intent. replace_field/set_field_member still accept only entry arms; native calls return action_not_applicable. remove_field and cleanup_facet_fields accept Selector2; native removal covers every same-name set/unset so old values cannot reappear, and cleanup still proves no effective-Facet dependency. append_field/field_append remain Entry-only and conflict with an existing header carrier. Bulk replace_single uses the same writer and requires exactly one fact per owner, never implicit append. New set_field_source handles native value edits and explicit representation conversion without arbitrary source patching.

Native-to-native uses a verified invertible codec and changes only the exact final effective-value span, preserving other assignments, placement and trivia; full reparse proves the new typed value and all effects. Header-to-Entry removes the complete same-name set and fixes the selected derived UUIDv5 as the Entry occurrenceKey (D4 already permits UUID1–5), no longer treating it as header continuity. Entry-to-native requires one occurrence, invertibility and no unpreservable qualifier/note/provenance/union information; otherwise reject and retain Draft. Entry-to-Entry preserves its key. Conversion/removal proves all native-reference effects in other attributes/body; if authorized Fields cannot bound those effects, require independent full Source authorization or reject, never a Boolean promise from hidden scans. Full-source users may preview and explicitly expand dependent native references to equivalent text before conversion, without shadow attributes.

An absent Field is created by an authorized Source constructor or Template field_set, never a fabricated selector. AssignFacet initialEntries remains the exact Entry constructor and may retain an already-present required native Field; adding an Entry for that Field conflicts. RemoveFacet edits only native weftext-types declarations and validates closure while retaining business values. Original FacetOperationRequest2 Entry-only cleanup is not widened: new FacetOperationRequest3 retains other members and uses wireVersion=3 and replaces only cleanupSelectors with complete Selector2 arrays, retaining every other null/empty-array and operationKind applicability rule, constructed by Core from real selections for the same D4 gate.

New current action arms in D7ActionPrepareRequest3, D7ActionInput3 and PAB4 are exact Spec3, dispatched by action.version; real Spec2 records retain their old domain. Canonical-descriptor/PAB byte equality remains. D8 property/task controls use that same Action. D10 core_field_member remains the existing Entry-only object-member action and cannot treat scalar status as an object member. Ordinary task status uses explicit Spec3 set_field_source; other Automation capabilities do not expand.

Recipe3 retains parameters and original title/body_text/field_append slots. Field_set uses the original parameter binding to set one singleton default fact, creating it if absent; multiple occurrences reject and carrier changes use the same lossless conversion. Native parameters match expanded ValueTypeSpec and full codec/wrappers without guessed calendars. TemplateConstruct2.recipe already references a protected Resource version, so its exact pin decoder dispatches on Recipe.version 2 or 3. ConstructionInput2/sourceSubjectBindings/PAB4 gain no members or premature final NodeId. Import typed-Field mappings similarly select explicit carriers in Core and produce real raw; unknown/non-invertible values enter original loss/rejection paths.

D4SourceMaterializationEffects2 retains original /1 kind, operationId, registryBindings, owners and complete version bases, with wireVersion=2. Each entries item retains sourceSubject/resultOwnerNodeRef/fieldId/referenceChanges, replaces the single occurrenceKey with beforeOccurrenceKey/afterOccurrenceKey (each UUID or null), and replaces rawEntrySource slots with beforeSource/afterSource: selected-carrier FieldSource3, absent on a fresh or fully removed side. A non-absent source has exactly one Entry item or one native arm, matching its own side’s occurrenceKey; truly absent sides use null keys. Keys need not match across sides: Entry UUIDv4→native UUIDv5 retains both, while header→Entry can preserve its key under the stated rule. Sort uniquely by original canonical sourceSubject key, resultOwnerNodeRef, FieldId, beforeOccurrenceKey and afterOccurrenceKey; null precedes UUID and UUIDs compare exact ASCII bytes. Never split conversion into unassociated removal/addition that loses its preimage. Items and owners are recomputed from the same full before/after sources, retaining every unchanged C owner. New semantic_extension format is d4_source_materialization_effects/2 and encoding is d4_source_materialization_effects2; D7/D8 explicitly decode it. In retained effects §4 format rank, /2 shares materialization rank=1 with /1 and the two are mutually exclusive in one manifest; original /1 stays exact. This placement does not close the old global effect-ordering finding.

### 3.1 Named native narrow adapter and dependency-state authority

This section precisely replaces the §2 opening and conditions2–5 of d7/owners/narrow-field/source for the new native literal-value branch. Original Entry narrow adapters and other exclusions remain; Selector2 does not bypass static proof.

Original source_envelope_state discloses only its enumerated aggregate validity/capacity, not native dependency topology. New Policy4 exactly inherits D6-CONTROL §10/§10.1 Policy3 {version,revision,grants}, grant/deny and original capabilities, fixes version=4 and adds only capability {kind:"native_dependency_state",fieldId:FieldId}. It supports workspace scope only and exact current-schema FieldId, defaults ungranted and is implied by no read/write/admin capability. Installation/change still requires the original policy_admin management path. Policy3 gains no grant. D6 policy decoders, policy key/pin and policy slots in authorization DependencyKey3/proof dispatch exactly on policy.version3 or4 with the same authorization generation/original seal freezing the full object; recorded1/2/3 recover unchanged. Existing bootstrap may retain Policy3. Until explicitly granted, native narrow edits are unavailable while independently authorized full Source editing remains.

Exact current Policy4 consumer slots are WorkspaceAuthorizationBundle2.policy, ConflictResolutionInput3.resolution.policy and ConflictResolutionPreview2.resolution.policy in D3-SCHEMAS/D6-SCHEMAS, plus the complete proposed policy and result Bundle2.policy in the D6-CONTROL §10.1 management path. They strictly accept Policy3|4 by policy.version. A3→4 change is an explicit policy successor authorized by original policy_admin, checked-incrementing Policy.revision, auth generation and bundle.authorizationRevision under original rules while preserving trust histories; changing only version or silently adding grants is illegal. Conflict choices and resultBundle.policy retain the actual original version or a lawful explicit successor and match byte-for-byte. Real old Bundle1/recorded domains are not widened. Current bootstrap.initialPolicy may retain original Policy3 without new grants; Bundle2, CP and pin prefixes do not gain versions.

This capability discloses only the specified Field’s aggregate isolation qualification and consequent selection/prepare result, never names, counts, paths, positions, raw, other Field values or body. Reuse original selectionToken/proof and authorization invalidation, adding no status endpoint, token type or proof. Read-only typed Query does not require it. Wire3 Field selection for a Field with a Registry native binding checks this grant before reading author source/topology; missing grant returns not_visible uniformly. Explicit native selectors/intents check it in advance too, rather than scanning and deciding permission from accidental isolation. Entry-only wire2 is not widened and cannot produce native selectors.

Only set_field_source native-to-native direct literal edits (and identical bulk replace_single decomposition) enter this adapter. Original condition1 and the complete Registry graph first establish potential Field/constraint closure; then check original Field read/write, entity/state, source_envelope_state, commit_sequence_state, source_write deny and the new capability. Complete attribute-use/origin evidence from the same D2 evaluation must then prove a uniquely writable root direct codec literal without substitution/import/generated input and no reads of its name by other header fields, body, conditions, macro/include targets, URLs or extensions/providers. Track complete actual native reads and negative ranges, never just literal reference searches; untrackable execution is unavailable. Any external use or unavailable proof yields only aggregate ineligibility without hidden reasons/topology. Body dependencies require independent full Source authorization in this increment; other-Field dependencies also require their complete enlarged authorized path, not this adapter.

For condition2, read complete FieldSource3 and codec/union/code/qualifier/provenance contributions and prove exactly one valid effective fact before/after. For condition3, unchanged present count preserves required_field/cardinality; every possible Facet constraint still needs static proof, root union variant stays unchanged, and unknown constructors or unauthorized dependencies reject without inspecting hidden membership to loosen rules. For condition4, only the final effective value’s exact byte span may change; all other set/unset events, weftext-id/weftext-types, namespaces, trivia, other header fields and body remain byte-equal. The codec must emit a safe legal native literal; complete reparse cannot add attribute/control lines or change parse structure. For condition5, the original plan freezes complete FieldSource3 images, actual byte patch, per-constraint proofs, source/Registry/environment and complete positive/negative native-read evidence. Internal whole source serves preservation and authorized aggregate validation only; public output stays within Field/metadata scope.

Bind this proof to before and proposed-after evaluation under the same frozen processor environment/source pins and revalidate at the final read barrier. Unset/absence/invalid values, remove, representation conversion, multiple origins or new fields gain no automatic narrow qualification; use separately authorized full Source or reject with Draft retained. Isolation-related success/failure is explicitly authorized aggregate metadata, not a claim that source activity is wholly hidden. INT-01 explicitly grants native_dependency_state(tasks/status): prior graph proof/authorization→complete no-external-read proof→one valid status safe open→done patch→same-cut FieldSource3 image→original P seal. Missing grant rejects before topology reads; body references and unknown extensions yield aggregate ineligibility, never automatic grants.

## 4. Registry and actual control slots

Snapshot2 has the exact eighteen members in AUTHORING §4. RegistryEvolutionProof2 is closed {version:2,previousSnapshot,currentSnapshot}, with an authenticated true Snapshot1 or2 previousSnapshot and mandatory Snapshot2 currentSnapshot; original predecessor, full digest, ownership and semantic-ledger checks remain. Its new version is a discriminator, not a widening of unversioned Proof1. Catalog and RegistryBinding1 stay unchanged.

The table normatively patches actual current slots. New-current preserves complete Snapshot2 bytes/digest/pins rather than dropping authoringBindings to fit old slots. New dispatch through existing holders uses snapshot.version=2 or evolution.version=2 at exactly the fixed paths listed. Recorded old holders first use their original recovery arm without substituting pinned objects. RegistryReads within one holder cannot mix1/2; the actual evolution previousSnapshot is the named exception.

| Actual owner / slot | New-current type and rule |
| --- | --- |
| D4 §2.2 handshake, ValidatedCatalogContext, registry/temporal proof | Snapshot2 + EvolutionProof2 for every non-bootstrap generation; authenticated original bootstrap retains the predecessor registry_bootstrap marker with no previousSnapshot/evolution proof; fixed catalog retained. |
| D3-SCHEMAS WorkspaceBootstrapTargetRegistry1.snapshot and D6 bootstrap producer | Snapshot2; original target trust/authorization/install rules remain without duplicated Plan4/policy. |
| D3-SCHEMAS and D10-CONTROL §0 D10WorkspaceReadDependencies2.registryReads[].snapshot | Snapshot2; exact binding/pin in the same cut; empty arrays invent no reads. |
| D3-SCHEMAS and D10-CONTROL §0 D10ControlEffectPlan2.registryChange | Actual Snapshot1 or2 before, Snapshot2 after and Proof2 evolution; sourceChange/authorization/order preserved. |
| D10-CONTROL §7 activation body, §8.1 activation supplement and D3-SCHEMAS counterparts | Snapshot2 and optional Proof2; actual before image retains its recorded arm, after has explicit inner version2. |
| D10-CONTROL §0.2 ScheduleRecurrenceEvidence2.registrySnapshot/registryEvolution | Snapshot2/optional Proof2 at the complete same scheduling cut; historical evidence/Witness/Step are not re-signed. |
| D6-CONTROL format, registry/period/bootstrap proof; D10-CONTROL §12 bootstrap consumers | Real Snapshot2/Proof2 and complete qualification, never an inferred empty table from missing files/plugins. |

Actual D10 activation remains the Image1/Pin1 activation arm, not automation/run-only Image2. It selects the new nested domain only at supplement.registrySnapshot.version=2 and corresponding Proof2. The D10-Control-Record/1 pin prefix remains while complete canonical pinned bytes/digest cover version and bindings. Dispatch recursively per named slot; one Snapshot2 never widens an entire outer object. Old before/initial/checkpoint/image/pin each keep exact original versions. A proven explicit evolution chain may cross1→2; the prohibition on mixed registryReads in one read does not prohibit mixed history across that transition.

## 5. Format production, reception and rendering

Profile2/Binding2 are exact AUTHORING §7 types. New managed2 link addressing also follows [LINKS](LINKS.zh-CN.md); its new Workspace-root codec never activates implicitly on reread/recovery of actual old profiles. AsciiDocProcessorEnvironment4 inherits all D2 Environment3 members, with version=4 and documentProfile additionally accepting weftext_managed/2; every other environment member remains. Managed2 evaluation equals verified Binding2.profile including original languageBaseline. A real render example requires snapshot.evaluation.processorEnvironment.documentProfile to equal semanticQualification.documentFormat.binding.profile.managedProfile, with identical languageBaseline values. DocumentFormatCurrentQualification2 inherits all /1 members, uses version=2 and strict Binding1|Binding2; componentImage.version still equals bindingRevision, never decoder1/2. ManagedDocumentSemanticQualification2 retains the original sourceObservation/documentFormat members with Qualification2 in documentFormat. Real old Qualification1/Semantic1/Environment3 always retain original decoders.

D2 evaluation/snapshot processor-environment slots explicitly accept Environment4 through its inner version; old environment pins are not re-encoded. D3-SCHEMAS/D6-SCHEMAS format qualification and D6-CONTROL §0.1.2 component decoding, §0.1.3 receiver admission and document_format dependencies use these exact types. Receivers dispatch on actual binding.version1 or2; unknown versions/wrong profiles reject. Portable_metadata pins are exact D3-CJ/3(binding); original CP4 rank, Notice3, ChangeRecord1, P seal and full associations remain. Missing managed2 binding/registry stays incomplete; readable control attributes never grants admission.

The new current semanticQualification slot of D9DocumentRenderBinding1 uses SemanticQualification2, with its exact D2 snapshot pin consuming Environment4. Select the new rendering profile by semanticQualification.documentFormat.version=2 and snapshot.evaluation.processorEnvironment.version=4, without altering frozen Semantic1/Environment3 in old ExportPlan4. New Plan/confirmation input unions and digests cover the complete new inner objects; prepared plans never change profile. Null render bindings for exact-source/resource/query-json remain unaffected, as do real old Plan/receipt/recovery records.

## 6. Native ID/type construction and original C/Q

Original Result9/C/Q accepts no new control-declaration tag. Core-internal ManagedControlConstruction1 is a named complete-source intermediate, not bytes submitable as original Result9 and not self-authenticated by workers or file UUIDs. The original global symbolic-Document-input P1 remains open: formal D3 framing still needs joint independent acceptance. Until then, managed2 creation/template/import remains end-to-end dependency-open; existing-identity property edits do not require fresh-ID construction.

```text
ManagedControlConstruction/1 = {
  kind:"managed_control_construction",version:1,
  subject:PayloadSubjectKey,
  preimagePin:PinRef/2,
  idSite:{kind:"existing_value",range:D2SourceRange/2,rawValue:text}
        |{kind:"insert_header",byteOffset:Counter,eol:"LF"|"CRLF"},
  declaredTypes:[text...]
}
```

Only Core constructs this closed intermediate from the same complete D2 native parse, header boundary and origin evidence. Existing_value addresses the verified copy/template preimage's unique direct weftext-id value span. Insert_header applies only to an exact skeleton lacking that reserved declaration or explicit format conversion of true managed1. The insertion point is immediately after optional BOM, with complete control lines and frozen EOL but no added blank terminating the header. If original text starts with a blank/body, full reparse must prove unchanged original title/body/author/revision classification; otherwise require explicit complete-source repair preview instead of blind insertion. Existing types are separately validated/patched rather than duplicated beside a new ID. Invalid, duplicate, unset, non-direct controls or foreign envelopes cannot bypass validation. Type positions/raw come from that preimage parse, not caller-trusted offsets. Subject permits Node existing/mapped/new only; declaredTypes derives from authorized choices/Recipe under the same-cut schema. Original budgets/authorization/retention apply without publicly preallocated UUIDs.

Mapped/new subjects obtain fresh NodeId only from the original D3 stage12 private candidate map; existing same-identity conversion uses only protected original NodeRef and birth/file binding without allocation. Existing ID edits touch only the exact literal; types use a separate patch, encoded by AUTHORING §2's comma protocol when changed. Equal expanded sets preserve shorthands/order/whitespace. New generation prefers fixed task, otherwise full facet IDs; empty types omit a new line or empty an existing value without generating unset. Missing types insert immediately after the complete ID line. If ID ends at EOF without EOL, add one separator EOL before the type line while retaining the original ID value span. Newly added EOL uses the document’s first actual EOL or fixed LF if none exists, frozen in complete proposed source. Template instances use Recipe target types without template, while the template itself keeps its identity. Ordinary copies without type changes preserve declarations. Controls are not Fields: Field materializers, MapAttribute, Recipe field_set, remove and bulk cannot touch them. Type changes retain Facet/Source gates; external ID edits do not transfer identity.

ID, types and business patches share complete preview/SourceTransform, retaining unrelated bytes/comments/BOM/EOL and native-origin dependencies. Full reparse proves ID agrees with unique allocation/original identity and birth/file binding, and types equal verified declaredTypes before D4 effective closure. Actual control displacement maps original C/Q pinned preimages into intermediate/final source; both passes reuse the same revision token, Q checks final positions and afterPin covers bytes0..EOF. The original single planning CAS/P seal freezes and installs format/source/metadata together. Contradictions reject wholly; planned recovery reuses frozen bytes/map/UUID without resampling or recalculation from current state.

Actual managed1 ordinary copy/fork retains Profile1/Binding1 and original source. Explicit copy-plus-format-conversion generates controls for a fresh subject with Binding2.bindingRevision=1. Same-identity conversion checked-increments bindingRevision; real source changes produce sourceChanges rather than empty profile-only changes. Restore retains historical versions. The undeployed YAML draft creates no new runtime decoder or migration responsibility.

## 7. Integration acceptance and remaining boundaries

INT-01–06 are design cases for later implementation, product UNRUN; each requires positive, rejection and old-record recovery review.

| ID | Required coverage |
| --- | --- |
| INT-01 | Existing qualified managed2 task, Field-only status open→done without body read: wire3 selection, Spec3, true image3, one commit and D8 read; reject hidden substitution/ABA/revocation. |
| INT-02 | Removing repeated same-name set/unset never reveals an earlier value; lossless two-way conversion; equal complex facts remain distinct; reject unpreservable notes/body references. |
| INT-03 | Recipe3 field_set native status replaces defaults; field_append conflicts with header; new-instance ID construction retains §6’s open global framing dependency. |
| INT-04 | Explicit actual Snapshot1→2 evolution with complete bootstrap/activation/proof/recurrence consumers; reject missing version/bindings, mixed read generations and wrong digest. |
| INT-05 | Binding2 pin→seal→second-replica admission→Qualification2/Environment4→D2/D9; reject unknown versions; actual Binding1 recovery/plain copy never silently converts. |
| INT-06 | Control-declaration displacement, C/Q and full afterPin agree; candidate/ID mismatch or partial metadata never installs; actual saved/planned/unknown are not re-encoded. |
