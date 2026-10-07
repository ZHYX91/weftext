---
source_language: zh-CN
translation_of: D7-SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](D7-SCHEMAS.zh-CN.md)
# A2 D7 Current Schema Overlay

Status: current-schema companion to the D7 author candidate; not implementation or independent acceptance.

## 1. Precedence

The byte-complete retained D7 author schemas live under d7/owners. This companion names only current successor dispatch and the cross-field rules changed by fixed97. Any author-level QuerySpec/ParameterSpec/TypedLiteral/TypeSpec/TerminalSchema/ViewSpec/QueryCall/CEL/operator member not changed below retains the exact closed shape in the local retained owner text.

## 2. Current version inventory

```text
Query outer: wireVersion 2
QuerySpec: version 1
ViewSpec: version 1
Action prepare: D7ActionPrepareRequest/3
Action author: D7ActionSpec/2
Action input: D7ActionInput/3
Prepared action: PreparedActionBinding/4
Proposed input: D7ProposedInput/3
Effects: EffectManifest/3 + EffectBytes/3
MinimumMapping: /3 retained
D3 fresh request: wire13
D6 fresh proof: InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3
```

An outer successor never implies an inner successor. Historical bytes always dispatch by their recorded tag before unseen-current validation.

## 3. Current Action prepare and input

```text
D7ActionPrepareRequest/3 = {
  wireVersion:3, kind:"d7_action_prepare",
  workspaceRef:WorkspaceRef, commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  action:D7ActionSpec/2,
  selectedSources:[SourceVersionRef/1...],
  budget:BudgetBinding/1,
  evidenceToken?:Token
}

D7ActionInput/3 = {
  kind:"d7_action_input", version:3,
  action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext/1...],
  proposedInputs:[D7ProposedInput/3...]
}
```

D7ActionSpec/2 is the fixed-parent ActionSpec/1 top-level shape at version 2, retaining all legal fixed-parent intents except the historical apply-suggestion arm that fixed97 replaced, and adding the current closed Annotation intents defined by the fixed97 owner. No other Action intent is widened by version 2.

selectedSources uses the retained SourceVersionRef profile and does not itself prove currentness. Currentness comes from the complete observations/proof bound into preparation.

## 4. PreparedActionBinding/4

```text
PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding", version:4,
  bindingToken:Token, protocolOwner:"D3"|"D6",
  operationId:UUIDv4, workspaceRef:WorkspaceRef,
  principalAudienceToken:Token, action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext/1...],
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  constructionInput:null|TemplateConstructionInput/2,
  proposedInputs:[D7ProposedInput/3...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,
  expiresAt:<D6 protected deadline>,
  request:D3IdentityOperationRequest/13|d6_commit_request/2,
  preview:<complete EffectManifest/3>,
  resolutionInput:null|D3ResolutionInput/1
}
```

The six D7ActionInput members equal their PAB counterparts individually. For a D6-owned current action the D6 ownerInput canonical descriptor is the exact D7ActionInput/3 and its pinRefs cover precisely protected proposed and actual source/definition/rule evidence under D6 ordering. D3-owned identity actions retain their D3 owner descriptor; PAB4 is cross-owner preparation and never creates a second D3 request authority.

bindingToken is random protected association material. It is not derived from request bytes and creates no hash cycle. The original request, binding, preview, proof, pins, plan and final P remain one decision.

## 5. D7ProposedInput/3

```text
D7ProposedInput/3 = {
  subject:PayloadSubjectKey,
  payloadKind:
    "exact_source_document"|"resource_bytes"|"annotation_value",
  encoding:
    "exact_source_utf8"|"resource_bytes"|
    "d3_annotation_value4"|"d3_symbolic_result9",
  pin:PinRef/2
}

legal pairs:
  exact_source_document <-> exact_source_utf8
  resource_bytes        <-> resource_bytes
  annotation_value      <-> d3_annotation_value4
  annotation_value      <-> d3_symbolic_result9
```

A D6 current concrete Annotation after uses complete Value/4 bytes. The symbolic Result/9 arm is legal only in its retained D3 branch. d3_annotation_value3 is invalid in Input3. Genuine historical Input2/PAB3 retains annotation_value3 and its original pins and is never upgraded.

## 6. EffectManifest/3 and EffectBytes/3

```text
EffectManifest/3 = {
  format:"weftext.effects", version:3,
  phase:"preview"|"committed",
  protocolOwner:"D3"|"D6", operationId:UUIDv4,
  workspaceRef:WorkspaceRef, profile:"full"|"owner_fields",
  items:[EffectItem/3...], decisionKey:DecisionKey/2
}

EffectBytes/3 = {
  handleToken:Token,
  encoding:
    "exact_source_utf8"|"resource_bytes"|"d3_annotation_value4"|
    "d3_symbolic_result9"|"d4_relation_copy_effects1"|
    "d4_source_materialization_effects1"|
    "d7_definition_transfer_effects2"|"d3_canonical_effects1"|
    "d6_workspace_bootstrap_plan1"|"d6_workspace_bootstrap_plan3"|
    "d6_workspace_bootstrap_plan4"|"d7_symbolic_json3"|"field_entries2",
  byteLength:Counter
}
```

EffectItem/3 retains the predecessor effect arms and the current owner-specific successor arms selected by fixed97. Every bytes slot uses EffectBytes/3. A decoder never discovers byte slots by recursively guessing member names. Preview and committed are distinct phases; a delivery epoch authorizes transport only and does not mutate semantic effect bytes.

## 7. Source plans and D6 current types

```text
source-plan dispatch:
  ordinary/fresh managed source         -> SourceRevisionPlan/1
  guarded D3 canonical conflict         -> SourceRevisionPlan/2
  D6 source_merge / choose_source_head  -> SourceRevisionPlan/3

current production address:
  SourceVersion/2

current receiver qualification:
  SourceObservation/1

portable current D6:
  DependencyKey/3
  DependencyProof/3
  InputDescriptor/3
  PreparedIntent/3
  InstallationNotice/3
  ContentCompletionProof/4
  ChangeRecord/1
```

SourceVersion and SourceObservation are different domains even when their contained production version is equal. A fresh read can establish a new current Observation for the same authenticated production version; it cannot mutate a saved Locator, DefinitionAddress, prepared selector, Query result, or ActionEvidence.

## 8. Strict JSON and numeric decoding

All closed objects reject duplicate keys, unknown members, missing required members, illegal null and cross-arm members. Optional means member absence unless that exact schema says otherwise. JSON Boolean is not an integer.

D7 integer and decimal values keep the exact canonical textual decoders of the retained Value/CEL profile. D3/D6 Counter and other bounded meta-wire integers retain their own range and overflow rules. No outer current version silently accepts exponent notation, host floating values, negative zero, or a nested numeric representation that the declared inner decoder rejects. Conversely, arbitrary-precision D7 integer values, such as current heading effectiveLevel, cannot be rejected merely because a host or historical schema used int64.

## 9. Query execution closure

Query outer remains wireVersion2 and the complete QuerySpec/1 author grammar remains in d7/owners/query-algebra. DAG validation, canonical ordinal, CEL typing, feature gates, source qualification, terminal schema, result encoding, paging/subscription reset, and complete error ordering are one closed path. Unknown feature or unavailable dependency cannot be converted into an empty bag.

## 10. Search schema boundary

Search introduces no new persistent Query or View schema. Plain search, visual conditions, and the optional shortcut grammar compile to an ordinary QuerySpec/1 plus current SearchContribution dependencies. Saving stores only the canonical Query definition. Shortcut parser version, text cursor, open filter popover, recent query, expansion state, and device direction are interaction state and do not enter DynamicBlock or saved Query.

The detailed grammar and SEARCH-01 through SEARCH-08 acceptance are in D7-SEARCH.

## 11. Historical recovery

Saved, planned, unknown, committed, and transport-recovery paths locate the exact original owner/version before applying current unseen gates. Old request fingerprints, MinimumMapping, pins, tokens, clocks, custody, installation proof, outbox, effect bytes, and unknown responsibilities remain exactly as produced. No current schema declares a permanent migration for an unproved prototype.
