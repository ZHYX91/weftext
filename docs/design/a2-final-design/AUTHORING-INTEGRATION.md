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
   value:TypedValue}

FieldSourceImage/3 = {
  format:"weftext.field-sources",version:3,ownerNodeRef:NodeRef,fieldId:FieldId,
  sourceVersion:SourceVersion/2|D7ProposedVersion/1,source:FieldSource/3
}
```

All objects are closed. Entry raw is the complete original Entry JSON. Native rawAttributeSources contains every set/unset event for the bound Field name in the same root header, in actual author order, preserving each event’s exact UTF-8 text and own line ending without intervening other fields/body. The same-cut authoringBindings determines the name; callers supply no second name. Absolute sourceRange stays in internal NativeFieldOrigin so Field-only output does not disclose hidden preceding lengths. Repeated assignments produce one effective fact; absence has no selector. FieldSource.absent means no author carrier for this Field, not absent Node source. Empty Entry collections canonicalize to absent.

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

An absent Field is created by an authorized Source constructor or Template field_set, never a fabricated selector. AssignFacet initialEntries remains the exact Entry constructor and may retain an already-present required native Field; adding an Entry for that Field conflicts. RemoveFacet edits only YAML declared types and validates closure while retaining business values. Original FacetOperationRequest2 Entry-only cleanup is not widened: new FacetOperationRequest3 retains other members and uses wireVersion=3 and replaces only cleanupSelectors with complete Selector2 arrays, retaining every other null/empty-array and operationKind applicability rule, constructed by Core from real selections for the same D4 gate.

New current action arms in D7ActionPrepareRequest3, D7ActionInput3 and PAB4 are exact Spec3, dispatched by action.version; real Spec2 records retain their old domain. Canonical-descriptor/PAB byte equality remains. D8 property/task controls use that same Action. D10 core_field_member remains the existing Entry-only object-member action and cannot treat scalar status as an object member. Ordinary task status uses explicit Spec3 set_field_source; other Automation capabilities do not expand.

Recipe3 retains parameters and original title/body_text/field_append slots. Field_set uses the original parameter binding to set one singleton default fact, creating it if absent; multiple occurrences reject and carrier changes use the same lossless conversion. Native parameters match expanded ValueTypeSpec and full codec/wrappers without guessed calendars. TemplateConstruct2.recipe already references a protected Resource version, so its exact pin decoder dispatches on Recipe.version 2 or 3. ConstructionInput2/sourceSubjectBindings/PAB4 gain no members or premature final NodeId. Import typed-Field mappings similarly select explicit carriers in Core and produce real raw; unknown/non-invertible values enter original loss/rejection paths.

D4SourceMaterializationEffects2 retains original /1 kind, operationId, registryBindings, owners and complete version bases, with wireVersion=2. Each entries item retains sourceSubject/resultOwnerNodeRef/fieldId/occurrenceKey/referenceChanges and replaces the two rawEntrySource slots with beforeSource/afterSource: single-fact FieldSource3, absent on a fresh/deleted side. Entry arms contain at most one item whose key equals the outer item key; native keys likewise agree. Items and owners are recomputed from the same full before/after sources, retaining every unchanged C owner. New semantic_extension format is d4_source_materialization_effects/2 and encoding is d4_source_materialization_effects2; D7/D8 explicitly decode it and original /1 stays exact.

## 4. Registry and actual control slots

Snapshot2 has the exact eighteen members in AUTHORING §4. RegistryEvolutionProof2 is closed {version:2,previousSnapshot,currentSnapshot}, with an authenticated true Snapshot1 or2 previousSnapshot and mandatory Snapshot2 currentSnapshot; original predecessor, full digest, ownership and semantic-ledger checks remain. Its new version is a discriminator, not a widening of unversioned Proof1. Catalog and RegistryBinding1 stay unchanged.

The table normatively patches actual current slots. New-current preserves complete Snapshot2 bytes/digest/pins rather than dropping authoringBindings to fit old slots. New dispatch through existing holders uses snapshot.version=2 or evolution.version=2 at exactly the fixed paths listed. Recorded old holders first use their original recovery arm without substituting pinned objects. RegistryReads within one holder cannot mix1/2; the actual evolution previousSnapshot is the named exception.

| Actual owner / slot | New-current type and rule |
| --- | --- |
| D4 §2.2 handshake, ValidatedCatalogContext, registry/temporal proof | Snapshot2 + required EvolutionProof2; fixed catalog retained. |
| D3-SCHEMAS WorkspaceBootstrapTargetRegistry1.snapshot and D6 bootstrap producer | Snapshot2; original target trust/authorization/install rules remain without duplicated Plan4/policy. |
| D3-SCHEMAS and D10-CONTROL §0 D10WorkspaceReadDependencies2.registryReads[].snapshot | Snapshot2; exact binding/pin in the same cut; empty arrays invent no reads. |
| D3-SCHEMAS and D10-CONTROL §0 D10ControlEffectPlan2.registryChange | Actual Snapshot1 or2 before, Snapshot2 after and Proof2 evolution; sourceChange/authorization/order preserved. |
| D10-CONTROL §7 activation body, §8.1 activation supplement and D3-SCHEMAS counterparts | Snapshot2 and optional Proof2; actual before image retains its recorded arm, after has explicit inner version2. |
| D10-CONTROL §0.2 ScheduleRecurrenceEvidence2.registrySnapshot/registryEvolution | Snapshot2/optional Proof2 at the complete same scheduling cut; historical evidence/Witness/Step are not re-signed. |
| D6-CONTROL format, registry/period/bootstrap proof; D10-CONTROL §12 bootstrap consumers | Real Snapshot2/Proof2 and complete qualification, never an inferred empty table from missing files/plugins. |

Actual D10 activation remains the Image1/Pin1 activation arm, not automation/run-only Image2. It selects the new nested domain only at supplement.registrySnapshot.version=2 and corresponding Proof2. The D10-Control-Record/1 pin prefix remains while complete canonical pinned bytes/digest cover version and bindings. Dispatch recursively per named slot; one Snapshot2 never widens an entire outer object. Old before/initial/checkpoint/image/pin each keep exact original versions. A proven explicit evolution chain may cross1→2; the prohibition on mixed registryReads in one read does not prohibit mixed history across that transition.

## 5. Format production, reception and rendering

Profile2/Binding2 are exact AUTHORING §7 types. AsciiDocProcessorEnvironment4 inherits all D2 Environment3 members, with version=4 and documentProfile additionally accepting weftext_managed/2; every other environment member remains. Managed2 evaluation equals verified Binding2.profile including original languageBaseline. DocumentFormatCurrentQualification2 inherits all /1 members, uses version=2 and strict Binding1|Binding2; componentImage.version still equals bindingRevision, never decoder1/2. ManagedDocumentSemanticQualification2 retains the original sourceObservation/documentFormat members with Qualification2 in documentFormat. Real old Qualification1/Semantic1/Environment3 always retain original decoders.

D2 evaluation/snapshot processor-environment slots explicitly accept Environment4 through its inner version; old environment pins are not re-encoded. D3-SCHEMAS/D6-SCHEMAS format qualification and D6-CONTROL §0.1.2 component decoding, §0.1.3 receiver admission and document_format dependencies use these exact types. Receivers dispatch on actual binding.version1 or2; unknown versions/wrong profiles reject. Portable_metadata pins are exact D3-CJ/3(binding); original CP4 rank, Notice3, ChangeRecord1, P seal and full associations remain. Missing managed2 binding/registry stays incomplete; readable YAML never grants admission.

The new current semanticQualification slot of D9DocumentRenderBinding1 uses SemanticQualification2, with its exact D2 snapshot pin consuming Environment4. Select the new rendering profile by documentFormat.version=2 and snapshot.environment.version=4, without altering frozen Semantic1/Environment3 in old ExportPlan4. New Plan/confirmation input unions and digests cover the complete new inner objects; prepared plans never change profile. Null render bindings for exact-source/resource/query-json remain unaffected, as do real old Plan/receipt/recovery records.

## 6. YAML ID construction and original C/Q

Original Result9/C/Q does not accept extra YAML tags. This candidate adds Core-internal ManagedEnvelopeConstruction1 as a named intermediate for complete-source construction, not bytes submitable as original Result9 and not self-authenticated by workers or file UUIDs. This does not resolve the original global symbolic-Document-input P1: formal D3 submission framing still needs joint independent acceptance with that finding. Until then, managed2 creation/template/import end-to-end status is explicitly dependency-open, not implementation-ready because of this algorithm or an empty-C/Q example. Existing-identity property edits do not depend on fresh-ID construction.

```text
ManagedEnvelopeConstruction/1 = {
  kind:"managed_envelope_construction",version:1,
  subject:PayloadSubjectKey,
  preimagePin:PinRef/2,
  envelopeSite:{kind:"existing_id",range:D2SourceRange,rawScalar:text}
              |{kind:"new_envelope",byteOffset:Counter,eol:"LF"|"CRLF"},
  declaredTypes:[text...]
}
```

Only Core constructs this from real input: existing_id covers a verified copy/template preimage, while new_envelope inserts uniquely after optional BOM into an exact ID-free AsciiDoc skeleton, not yet a legal managed Document. DeclaredTypes comes from actual authorized type choices/Recipe and same-cut schema validation; malformed input or another envelope cannot bypass validation through new_envelope. This intermediate and original pin retain original operation budgets/authorization/retention without a publicly preallocated UUID.

Formal production uses the original D3 stage12 private candidate map to resolve subject to the same unique NodeId. Existing_id replaces only the exact UUID scalar, preserving quotes/comments/BOM/EOL; new_envelope uses a deterministic minimal envelope, safely quoted type tokens and frozen EOL. Full reparse proves id==candidate subject==birth/file binding. Compute the exact envelope edit/displacement, then use full SourceTransform to map original C/Q pinned preimage ranges into intermediate/final source; retain both passes, the same revision token and Q final-position verification. The afterPin covers bytes0 through EOF. Format binding, source and metadata freeze/install consistently under the single winning planning CAS/P seal. Any mismatch aborts wholly; planned recovery uses frozen bytes/map/UUID without resampling or recalculating positions from current source.

Ordinary copy/fork of actual managed1 retains Profile1/Binding1 and original source without YAML. Explicit copy-plus-format-conversion preview uses the fresh subject and this envelope, with Binding2.bindingRevision=1. Same-identity conversion checked-increments bindingRevision and real source edits produce real sourceChanges. Restore always uses original historical versions. Profile-only/empty-sourceChanges never hides envelope insertion.

## 7. Integration acceptance and remaining boundaries

INT-01–06 are design cases for later implementation, product UNRUN; each requires positive, rejection and old-record recovery review.

| ID | Required coverage |
| --- | --- |
| INT-01 | Existing qualified managed2 task, Field-only status open→done without body read: wire3 selection, Spec3, true image3, one commit and D8 read; reject hidden substitution/ABA/revocation. |
| INT-02 | Removing repeated same-name set/unset never reveals an earlier value; lossless two-way conversion; equal complex facts remain distinct; reject unpreservable notes/body references. |
| INT-03 | Recipe3 field_set native status replaces defaults; field_append conflicts with header; new-instance ID construction retains §6’s open global framing dependency. |
| INT-04 | Explicit actual Snapshot1→2 evolution with complete bootstrap/activation/proof/recurrence consumers; reject missing version/bindings, mixed read generations and wrong digest. |
| INT-05 | Binding2 pin→seal→second-replica admission→Qualification2/Environment4→D2/D9; reject unknown versions; actual Binding1 recovery/plain copy never silently converts. |
| INT-06 | Envelope displacement, C/Q and full afterPin agree; candidate/ID mismatch or partial metadata never installs; actual saved/planned/unknown are not re-encoded. |
