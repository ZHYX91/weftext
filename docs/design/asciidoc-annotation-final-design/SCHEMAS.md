---
source_language: zh-CN
translation_of: SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](SCHEMAS.zh-CN.md)

# AsciiDoc / Annotation Final Design: Closed Schemas and Current Cross-Owner Contracts

Status: **candidate-design-not-implemented**. This is the normative closed-schema companion to [SPEC.md](SPEC.md). SPEC owns behavioral algorithms; this file freezes current successor members, unions, ordering, and historical dispatch. A named closed shape here does not permit unlisted members. Unchanged nested named types such as `WorkspaceRef`, `NodeRef`, `Frontier/2`, and `PinRef/2` import the exact decoder from the fixed-parent owner named by `replacements.json`; that is an explicit unchanged type import, not an omitted successor member list.

Fixed parent: `e8aa0b341630a57c786c0891d4bbd1620247441d`.

## 0. Common rules

- Every object is closed. Unknown, missing, duplicate, illegal-null, and wrong-union-arm members reject.
- JSON integers do not accept Boolean, float, or exponential spellings. Arbitrary stable integer semantics use canonical decimal text where specified.
- No unstated NFC, case fold, or trim.
- Language sequences preserve order; sets are canonically sorted/unique; multisets preserve multiplicity.
- D3-CJ/3 is the canonical JSON byte encoding used by all named canonical byte contracts.
- Current successors apply only to unseen/current production. Real saved/planned/unknown history retains its exact original decoder, bytes, pins, and recovery obligations and is never background-migrated.
- Having a decoder or historical design text does not prove that a prototype was deployed.

# 1. D2 Processor Environment

```text
AsciiDocProcessorEnvironment/3 = {
  kind:"weftext_asciidoc_processor_environment",
  version:3,
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  documentProfile:"baseline_only/1"|"weftext_managed/1",
  backend:ProcessorBackendSemanticProfile/1,
  doctype:"unset"|"article"|"book"|"manpage"|"inline",
  safeMode:"unsafe"|"safe"|"server"|"secure",
  standalone:Boolean,
  input:ProcessorInputIdentity/2,
  output:ProcessorOutputIdentity/1,
  baseDir:ProcessorPath/1,
  ambientUserHome:ProcessorPath/1,
  attributeOverrides:[ProcessorAttributeOverride/1...],
  timeInputs:ProcessorTimeInputs/1,
  includeEnvironment:IncludeEnvironment/2,
  extensionProfiles:[AcceptedProcessorExtensionProfile/1...]
}

ProcessorPath/1 = {
  kind:"virtual_posix_absolute",
  value:text
}

ProcessorBackendSemanticProfile/1 = {
  profileId:text,
  backend:text,
  basebackend:text,
  filetype:text,
  outfilesuffix:text,
  semanticDescriptorSha256:"sha256:<64 lowercase hex>"
}

ProcessorInputIdentity/2 =
    {kind:"managed_file",ownerNodeRef:NodeRef,
     sourceObservation:SourceObservation/1,
     logicalDocfile:ProcessorPath/1,logicalDocdir:ProcessorPath/1}
  | {kind:"artifact_file",artifactPin:PinRef/2,
     logicalDocfile:ProcessorPath/1,logicalDocdir:ProcessorPath/1}
  | {kind:"string_input",logicalName:text}

ProcessorOutputIdentity/1 = {
  logicalOutfile:ProcessorPath/1|null,
  logicalOutdir:ProcessorPath/1|null
}

ProcessorAttributeOverride/1 = {
  name:text,
  action:"hard_set"|"soft_set"|"hard_unset"|"soft_unset",
  value:text|null
}

CanonicalSignedDecimal = "0" | "-"?[1-9][0-9]*

ProcessorInstant/1 = {
  utcSeconds:CanonicalSignedDecimal,
  offsetMinutes:integer(-1439..1439)
}

ProcessorTimeInputs/1 = {
  sourceDateEpochSeconds:CanonicalSignedDecimal|null,
  inputMtime:ProcessorInstant/1|null,
  clockNow:ProcessorInstant/1
}

SourceUnitBinding/1 = {
  logicalPath:ProcessorPath/1,
  source:
      {kind:"managed",ownerNodeRef:NodeRef,
       sourceObservation:SourceObservation/1}
    | {kind:"artifact",pin:PinRef/2}
    | {kind:"network_snapshot",
       requestDigest:"sha256:<64 lowercase hex>",pin:PinRef/2}
}

IncludeEnvironment/2 = {
  resolverProfile:"weftext_virtual_include/1",
  maxDepth:integer(0..65535),
  uriRead:"disabled"|"authorized_snapshot",
  sourceUnits:[SourceUnitBinding/1...]
}

AcceptedProcessorExtensionProfile/1 = {
  profileId:text,
  semanticVersion:text,
  descriptorSha256:"sha256:<64 lowercase hex>"
}
```

`attributeOverrides` sort uniquely by lowercase name, extension profiles by profileId, and sourceUnits by logicalPath.value. Set actions require non-null values and unset actions require null. With safe mode below server, an unoverridden user-home comes from ambientUserHome; server/secure use the native "." default. SOURCE_DATE_EPOCH, when present, drives local*/doc* UTC values; otherwise local* uses clockNow and doc* uses inputMtime when available, then clockNow.

# 2. Core Semantic Projection

```text
UInt = nonnegative JSON integer
DecimalInteger = "0" | "-"?[1-9][0-9]*

SemanticValue =
    null | Boolean | text
  | {integer:DecimalInteger}
  | {array:[SemanticValue...]}

Property = [name:text,value:SemanticValue]

CoreSemanticProjection/1 = {
  format:"weftext.core-semantic-projection",
  version:1,
  document:CoreDocument/1,
  blocks:[CoreBlock/1...],
  catalog:CoreCatalog/1,
  indexTerms:[CoreIndexTerm/1...],
  finalState:CoreFinalState/1,
  diagnostics:[CoreDiagnostic/1...]
}

CoreDocument/1 = {
  doctype:"article"|"book"|"manpage"|"inline",
  title:null|{
    combined:CoreFlow/1,
    main:CoreFlow/1,
    subtitle:CoreFlow/1|null
  },
  authors:[{
    name:text|null,firstname:text|null,middlename:text|null,
    lastname:text|null,initials:text|null,email:text|null
  }...],
  revision:null|{number:text|null,date:text|null,remark:text|null}
}

CoreBlock/1 = {
  kind:CoreBlockKind/1,
  level:DecimalInteger|null,
  properties:[Property...],
  title:CoreFlow/1|null,
  reftext:CoreFlow/1|null,
  body:CoreFlow/1|null,
  children:[CoreBlock/1...]
}

CoreBlockKind/1 =
  preamble|section|floating_title|paragraph|
  open|example|sidebar|listing|literal|stem|pass|
  quote|verse|admonition|ulist|olist|dlist|colist|
  list_item|dlist_entry|dlist_term|dlist_description|
  table|table_head|table_body|table_foot|table_row|table_cell|
  image|audio|video|thematic_break|page_break|toc

CoreFlow/1 = {runs:[CoreRun/1...],marks:[CoreMark/1...]}

CoreRun/1 =
    {kind:"text",value:text}
  | {kind:"soft_break"}
  | {kind:"hard_break"}
  | {kind:"atom",
     name:"image"|"icon"|"stem"|"footnote_ref"|"callout"|"kbd"|"button"|"menu",
     properties:[Property...]}
  | {kind:"raw",medium:"passthrough"|"backend_feedback",value:text}

CoreMark/1 = {
  kind:"strong"|"emphasis"|"monospace"|"mark"|
       "superscript"|"subscript"|"double_quote"|"single_quote"|
       "role"|"link"|"xref",
  start:UInt,end:UInt,properties:[Property...]
}

CoreSite/1 =
    {kind:"flow",path:SemanticPath,start:UInt,end:UInt}
  | {kind:"scalar",path:SemanticPath,field:text,start:UInt,end:UInt}
  | {kind:"node",path:SemanticPath}
  | {kind:"detached",owner:SemanticPath}

CoreIndexTerm/1 = {
  site:CoreSite/1,
  visibility:"visible"|"concealed",
  terms:[text...],see:text|null,seeAlso:[text...]
}

CoreCatalog/1 = {
  anchors:[{id:text,reftext:CoreFlow/1|null,site:CoreSite/1}...],
  footnotes:[{index:DecimalInteger,id:text|null,body:CoreFlow/1}...],
  links:[{target:text}...],
  images:[{target:text,properties:[Property...]}...],
  includes:[{target:text,properties:[Property...]}...],
  callouts:[{list:UInt,item:UInt,properties:[Property...]}...]
}

CoreFinalState/1 = {
  attributeChanges:[{
    name:text,
    result:{kind:"absent"}|{kind:"present",value:SemanticValue}
  }...],
  counters:[{name:text,value:SemanticValue}...]
}

CoreDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error"|"fatal",
  semanticCode:text,
  location:null|{logicalFile:text|null,line:UInt|null}
}
```

Property names sort uniquely by UTF-8 bytes. Marks sort by start ascending, end descending, fixed kind rank, then D3-CJ/3(properties). One Unicode scalar occupies one flow unit; a break or atom occupies one. Visible text has one owner only.

Catalog anchors sort by id, footnotes by numeric index, links/images/includes by D3-CJ/3(item) while preserving multiplicity, and callouts by list/item. Diagnostics sort by null-first logical file/line, severity rank, then semanticCode, preserving duplicates.

# 3. CoreSemanticPropertyProfile/1

Only four provenance classes exist: A=authored_named, P=authored_positional, F=fixed_derived, I=internal. The projector consumes observed final model/consumer facts and never reruns parser/model logic.

Common block slots are exactly `id, style, roles, options, caption, numeral, subs, positional, named/<original-name>`. Facts already carried by `CoreBlock.kind/level/title/reftext/body/children` are not duplicated.

Required per-kind semantic fields are:

| kind | required additional fields |
|---|---|
| section | sectname, special, numbered, actual numeral/caption |
| listing | language, linenums, start, indent, tabsize, highlight, line-comment |
| literal | indent, tabsize, line-comment |
| stem | normalized notation/style |
| quote / verse | attribution, citetitle; verse also real indent/tabsize |
| admonition | name, textlabel, actual icon |
| ulist | final list style, actual checklist/interactive options |
| olist | numbering style, actual start, reversed |
| dlist | style, labelwidth, itemwidth, actual options |
| list_item | marker, checklist state, actual coids |
| table | cols, format, separator, width, frame, grid, stripes, float, orientation, colcount, rowcount, tablepcwidth, tableabswidth when produced, columns |
| table_cell | colspan,rowspan,halign,valign,cellStyle |
| image | target,alt,width,height,format,scaledwidth,scale,link,window,float,align,fallback and actual title/imagesdir |
| audio | target,start,end and common options |
| video | target,poster,width,height,start,end,preload,float,align,hash,theme,lang,list,playlist |
| toc | actual levels; title stays CoreBlock.title |

Synthetic dlist/table grouping nodes carry empty properties when they have no real authored object.

Marks/atoms: link requires target; xref target/refid/path; image/icon their media fields; stem notation plus evaluated source; footnote_ref index/referenceKind/target; callout number/id/guard; kbd ordered keys; button text; menu menu/submenus/menuitem.

Authored `foo-option=bar` yields both option membership and `named/foo-option="bar"`; an authored empty value preserves the empty string. Physical empty `foo-option` created by `%foo/options=foo/opts=foo` expresses membership only. Temporary/internal writers such as DocBook root-option never become authored merely by key name.

# 4. Witness/7 model evidence

```text
OracleSemanticWitness/7 = {
  format:"weftext.asciidoc-oracle-witness",version:7,
  baseline,processorEnvironmentSha256,
  document:OracleDocumentState/2,
  blockEvents:[OracleBlockEvent/2...],
  collectionEvents:[OracleCollectionEvent/1...],
  catalogEvents:[OracleCatalogEvent/1...],
  diagnostics:[OracleDiagnostic/1...],
  observedStrings:[OracleObservedString/1...],
  calls:[OracleEvaluationCall/1...],
  operations:[OracleStringOperation/1...],
  inlineObservations:[OracleInlineObservation/3...],
  contentObservations:[OracleContentObservation/1...],
  modelPropertyObservations:[OracleModelPropertyObservation/1...]
}

ModelSubject/1 = {
  id:UInt,
  kind:"document"|"section"|"block"|"list"|"list_item"|"table"|
       "column"|"cell"|"inline"|"attribute_buffer"|
       "table_parser_context"|"catalog_record"
}

ModelAttributeKey/1 =
    {kind:"name",value:text}
  | {kind:"position",decimal:DecimalInteger}
  | {kind:"symbol",value:text}

ModelObservedValue/1 =
    {kind:"nil"}
  | {kind:"boolean",value:Boolean}
  | {kind:"integer",decimal:DecimalInteger}
  | {kind:"symbol",name:text}
  | {kind:"float64",ieee754beHex:"<16 lowercase hex>"}
  | {kind:"string",observedStringId:UInt}
  | {kind:"array",items:[ModelObservedValue/1...]}
  | {kind:"map",entries:[{key:ModelAttributeKey/1,value:ModelObservedValue/1}...]}
  | {kind:"opaque_internal",classTag:text}

ModelSlotState/1 =
    {state:"present",value:ModelObservedValue/1,
     provenance:ModelPropertyProvenance/1}
  | {state:"absent",cause:"never_initialized"|"deleted",
     provenance:ModelPropertyProvenance/1|null}

ModelPropertyProvenance/1 = {
  class:"authored_named"|"authored_positional"|"fixed_derived"|"internal",
  form:"named_assignment"|"positional_assignment"|"style_shorthand"|
       "option_syntax"|"rekey"|"copy"|"inherit"|"computed"|"default"|
       "delete"|"temporary"|"restore",
  writerSite:M37Site/1,
  inputs:[ModelPropertyInput/1...]
}

ModelPropertyInput/1 =
    {kind:"model_slot",observationId:UInt,slot:ModelSlotAddress/1}
  | {kind:"observed_value",observedStringId:UInt}
  | {kind:"observed_slice",valueId:UInt,startByte:UInt,endByte:UInt}
  | {kind:"operation",operationId:UInt}
  | {kind:"inline_field",inlineEventId:UInt,fieldPath:[text|UInt...]}

ModelSlotAddress/1 =
    {kind:"field",name:ModelFieldName/1}
  | {kind:"attribute",key:ModelAttributeKey/1}

ModelFieldName/1 =
  context|node_name|id|content_model|level|style|caption|numeral|
  sectname|special|numbered|index|marker|colspan|rowspan|
  has_header_option|format|delimiter|type|target|imagesdir

ModelRelation/1 = {
  role:"parent"|"header"|"child"|"dlist_term"|"dlist_description"|
       "table_column"|"table_head_cell"|"table_body_cell"|"table_foot_cell"|
       "cell_column"|"cell_inner_document",
  position:[UInt...],
  targetSubjectId:UInt
}

ModelCarrierReference/1 = {
  stream:"document"|"blockEvents"|"collectionEvents"|"catalogEvents"|
         "inlineObservations"|"contentObservations"|"calls",
  entry:UInt
}

OracleModelPropertyObservation/1 =
    {kind:"snapshot",observationId:UInt,subject:ModelSubject/1,
     previousSnapshotObservationId:UInt|null,captureSite:M37Site/1,
     moment:"after_construct"|"after_write"|"after_finalize"|"before_use"|"after_use",
     fields:[{name:ModelFieldName/1,slot:ModelSlotState/1}...],
     attributes:[{key:ModelAttributeKey/1,slot:ModelSlotState/1}...],
     relations:[ModelRelation/1...],
     coverage:"complete_profile_fields_and_attribute_slots"}
  | {kind:"bind",observationId:UInt,subjectId:UInt,
     snapshotObservationId:UInt,carrier:ModelCarrierReference/1}
  | {kind:"cut",observationId:UInt,captureSite:M37Site/1,
     phase:"model_ready"|"evaluation_complete",documentSubjectId:UInt,
     heads:[{subjectId:UInt,snapshotObservationId:UInt}...]}
```

Per-subject snapshots form an immediate-predecessor chain. Cut heads sort numerically by subjectId and exactly equal the reachable semantic closure. Only references within the model-observation namespace use numeric temporal comparison. operation/inline/value/call references use their own namespace contracts. carrier.entry is a target-array index and is never numerically compared with model observation IDs.

Producer conformance additionally requires, at the real bind callback, that the target carrier has already been appended, `entry < targetStream.lengthAtBind`, and the producer still holds the exact same Ruby object. The wire decoder can only validate final existence/type/index, not infer cross-stream time. document/0 has the same rule.

Catalog ownership is only `parent -> actual Document#register receiver`. A temporary overlay must close through a real restore or cleanup delete; the observer never fabricates a restore write.

# 5. Managed format and D6 successors

```text
ManagedDocumentFormatProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  managedProfile:"weftext_managed/1"
}

ManagedDocumentFormatBinding/1 = {
  kind:"weftext_managed_document_format",version:1,
  ownerNodeRef:NodeRef,bindingRevision:Counter,
  profile:ManagedDocumentFormatProfile/1
}

DocumentFormatDependencyKey/1 = {
  kind:"document_format",workspaceRef:WorkspaceRef,ownerNodeRef:NodeRef
}

DocumentFormatCurrentQualification/1 = {
  kind:"d2_document_format_current_qualification",version:1,
  key:DocumentFormatDependencyKey/1,
  stamp:{epoch:Token,revision:Counter},
  componentImage:{state:"present",version:Counter,
                  byteLength:Counter,sha256:"64-lowercase-hex"},
  binding:ManagedDocumentFormatBinding/1,
  bindingPin:PinRef/2
}

ManagedDocumentSemanticQualification/1 = {
  sourceObservation:SourceObservation/1,
  documentFormat:DocumentFormatCurrentQualification/1
}
```

BaselineOnly is not a portable binding arm. Fresh/copy/fork fresh identity starts at binding revision 1; formal same-Workspace restore restores the exact historical binding; ordinary backup bytes require fresh admission; a real managed-profile migration checked-increments the revision.

```text
DependencyKey/3 =
  source(0)|document_format(1)|lifecycle(2)|placement_range(3)|
  ref_inbound(4)|relation_incidence(5)|calendar_scope(6)|registry(7)|
  temporal_rules(8)|authorization(9)|foreign_binding(10)|query_scan(11)|
  replica_registry(12)|conflict_record(13)|execution_resource(14)

DependencyProof/3 = {
  kind:"d6_dependency_proof",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  baseFrontier:Frontier/2,
  entries:[{key:DependencyKey/3,
            stamp:{epoch:Token,revision:Counter},
            evidencePins:[PinRef/2...]}...]
}

InputDescriptor/3 = {
  kind:"d6_input_descriptor",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  intentKind:text,
  saveProfile:"ordinary"|"complete"|"control_only",
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  frontierPolicy:"exact"|"scope_dependencies",
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  controlInputs:[{key:DependencyKey/3,
                  stamp:{epoch:Token,revision:Counter}}...],
  ownerInput:OwnerInputBinding/2
}

PreparedIntent/3 = {
  kind:"d6_prepared_intent",version:3,
  planToken:Token,operationId:Uuid,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,beforeCut:Frontier/2,
  proposedState:SemanticState/1,mutationFootprint:MutationFootprint,
  dependencyProof:DependencyProof/3,observationProof:ObservationProof,
  budgetBinding:BudgetBinding/1,pinDirectory:PinDirectory,
  installationPlan:InstallationPlan,inputRetentionState:InputRetentionState,
  expiresAt:PreparedDeadline,previewBinding:PreviewBinding
}
```

The final unchanged nested names above use the exact fixed-parent PreparedIntent/2 decoder. Current planToken tag is `d6_plan/3`.

```text
PortableComponentKey/2 =
  document(0)|document_format(1)|resource(2)|annotation(3)|
  node_binding(4)|child_list(5)|lifecycle(6)|trash_membership(7)|
  policy(8)|registry(9)|period_scope(10)|replica_registry(11)|conflict(12)

InstallationNotice/3 = {
  format:"weftext.installation-notice",version:3,
  decisionKey:DecisionKey/2,
  guarantee:"replica_local"|"managed_atomic",
  writeProtection:"strict"|"observed_only",
  baseFrontier:Frontier/2,
  components:[{key:PortableComponentKey/2,
               before:ComponentImage/1,after:ComponentImage/1}...]
}

ContentCompletionProof/4 =
    {format:"weftext.content-completion",version:4,outcome:"committed",
     decisionKey:DecisionKey/2,changeId:ChangeId/1,
     guarantee:"replica_local"|"managed_atomic",
     writeProtection:"strict"|"observed_only",
     semanticState:SemanticState/1,
     frontierBefore:Frontier/2,frontierAfter:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...],
     sourceChanges:[{entityRef:EntityRef,
                     before:SourceVersion/2|"absent",
                     after:SourceVersion/2|"absent"}...],
     receiptDigest:"sha256:64-lowercase-hex"}
  | {format:"weftext.content-completion",version:4,outcome:"restored",
     decisionKey:DecisionKey/2,baseFrontier:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...]}
```

`PinRef/2` and `ComponentImage/1` are unchanged fixed-parent types.

```text
ChangeRecord/1 = {
  format:"weftext.change-record",version:1,
  decisionKey:DecisionKey/2,changeId:ChangeId/1,
  installationNotice:{format:"weftext.installation-notice",version:3,
                      byteLength:Counter,sha256:"64-lowercase-hex"},
  completionProof:{format:"weftext.content-completion",version:4,
                   byteLength:Counter,sha256:"64-lowercase-hex"},
  frontierBefore:Frontier/2,frontierAfter:Frontier/2
}
```

CP4 must be committed and match DecisionKey, ChangeId, and frontiers. Notice/CP lengths and digests bind their exact canonical bytes. One P seal fixes ChangeId, CP4, and ChangeRecord; publication only retransmits original pins.

# 6. D3/D7/D8/D9 direct holders

```text
D3ResolutionInputUse/2 = {
  kind:"d3_resolution_input_use",version:2,
  decisionKey:DecisionKey/2,principalAudienceToken:Token,
  bindingToken:Token,inputDescriptor:InputDescriptor/3
}

PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding",version:4,
  bindingToken:Token,protocolOwner:"D3"|"D6",
  operationId:UUIDv4,workspaceRef:WorkspaceRef,
  principalAudienceToken:Token,action:ActionSpec,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext...],
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  constructionInput:null|TemplateConstructionInput/2,
  proposedInputs:[D7ProposedInput/2...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,expiresAt:<D6 protected deadline>,
  request:<D3 wire13|d6_commit_request/2>,
  preview:<complete EffectManifest/3>,
  resolutionInput:null|D3ResolutionInput/1
}
```

`MinimumMapping/3`, D7DefinitionInput/2, D7ProposedInput/2, and D7ResolutionAccess/1 remain unchanged exact fixed-parent types.

```text
EffectManifest/3 = {
  format:"weftext.effects",version:3,
  phase:"preview"|"committed",
  protocolOwner:"D3"|"D6",operationId:UUIDv4,
  workspaceRef:WorkspaceRef,profile:"full"|"owner_fields",
  items:[EffectItem/3...],decisionKey:DecisionKey/2
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

EffectItem/3 has the same fourteen semantic arms as the fixed-parent EffectItem/2: source_change, conditional_source_change, entity_state_change, d3_plan, d3_receipt, semantic_extension, period_scope_change, series_configuration_change, workspace_bootstrap, authority_change, field_change, conflict_branch_source, conflict_resolution_change, and canonical_plan; every byte slot uses EffectBytes/3, Annotation source images use Value/4, and current workspace bootstrap admits Plan4. No generic/free payload arm exists.

```text
PreparedEditBinding/3 = {
  kind:"d8_prepared_edit_binding",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  operationId:UUIDv4,principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  intent:<D8 current closed edit intent>,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:<OriginalD6Request>},
  sourceInputs:<InputDescriptor/3.sourceInputs>,
  proposedInputs:[{entityRef:EntityRef,pin:PinRef/2}],
  registryInputs:[ValidatedCatalogContext...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,expiresAt:<D6 protected deadline>,
  request:d6_commit_request/2,preview:<complete EffectManifest/3>
}
```

proposedInputs contains exactly one item and its entityRef equals intent.target.ref. OwnerInputBinding/2 remains unchanged; current ownerKind is `d8_edit/3`.

```text
ExportPlan/3 = {
  kind:"d9_export_plan",version:3,
  planToken:Token,workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,authorizationGeneration:Token,
  inputCatalog:ExportInputCatalog/2,
  contentSelection:ExportContentSelection/1,
  projection:ExportProjection/1,
  templateBinding:null|<exact template bytes/version binding>,
  routeBinding:<exact route/profile/version binding>,
  targetFormat:<D9 closed export target>,
  initialLossReport:ExportLossReport/1,
  outputBudget:BudgetBinding/1,
  destinationIntent:<protected destination intent>,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  sourceRuleRoutePins:[PinRef/2...],
  stagedOutputs:[{name:text,byteLength:Counter,
                  sha256:"sha256:<64 lowercase hex>",pin:PinRef/2}...]
}

PublicationReceipt/3 = {
  kind:"d9_publication_receipt",version:3,
  publicationToken:Token,planToken:Token,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  outputs:[{name:text,byteLength:Counter,
            sha256:"sha256:<64 lowercase hex>"}...],
  lossReport:ExportLossReport/1,
  lossChoices:[<exact original confirmation choice>...],
  routeProfileStyleVersions:[<exact bound version>...],
  destinationDisplay:text
}
```

PublicationReceipt proves external publication only; it is not a D6 author receipt. Historical export/publication versions retain original decoders.

# 7. Annotation closed values

```text
D3-Annotation-Value/4 = {
  kind:"d3_annotation_value",version:4,
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:Suggestion/3|null,
  creator:AnnotationActorSnapshot/1,
  authoredAt:AnnotationTimeSnapshot/2,
  lastEditor:AnnotationActorSnapshot/1,
  editedAt:AnnotationTimeSnapshot/2
}

D3-Annotation-Target-Projection/1 =
    {kind:"document",owner:NodeRef}
  | {kind:"document_element",locator:DocumentElementLocator}
  | {kind:"document_range",locator:DocumentRangeLocator}
  | {kind:"resource",resourceRef:ResourceRef}
  | {kind:"resource_region",locator:ResourceRegionLocator}

AnnotationInlineProfile/1 = {
  languageBaseline:"asciidoctor-ruby/2.0.26",
  doctype:"inline",processorBackend:"html5_semantic_environment/1",
  safeMode:"secure",maxSourceBytes:65536,maxRenderedBytes:262144,
  maxInlineSemanticNodes:4096,managedAdapters:"disabled",
  networkEffects:"denied",fileEffects:"denied",processEffects:"denied"
}

AnnotationAppearance/1 = {
  mark:"highlight"|"underline"|"squiggle"|"strike",
  theme:"yellow"|"red"|"green"|"blue"|"purple"|"pink"|"gray"
}

AnnotationActorSnapshot/1 = {
  kind:"annotation_actor_snapshot",version:1,
  originWorkspaceRef:WorkspaceRef,
  authentication:"workspace_authenticated_origin"|
                 "device_local_unverified"|"imported_unverified",
  displayName:text
}

AnnotationTimeSnapshot/2 = {
  instant:RFC3339-with-offset,
  producer:"prepare_server_clock"|"prepare_device_clock"|"imported_unverified"
}

Suggestion/3 = {
  version:3,kind:"replace"|"delete"|"insert",
  state:"pending"|"accepted"|"rejected",
  confirmation:"confirmed"|"needs_reconfirmation"|"not_applicable",
  targetBasisSha256:"sha256:<64 lowercase hex>",
  expectedText:null|{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  pointAffinity:null|"left"|"right",
  replacementSource:null|text
}
```

Replies are same-owner, acyclic comments with suggestion=null and reviewState=not_applicable. Root reviewState is open|resolved. Pending suggestions use confirmed|needs_reconfirmation; accepted/rejected are terminal with confirmation=not_applicable.

# 8. SourceTransform

```text
PortableTransformCompilation/1 =
    {kind:"representable",events:[SourceTransformPortableEvent/3...],
     afterSourceSha256:"sha256:<64 lowercase hex>"}
  | {kind:"unavailable",
     reason:"provenance_gap"|"unsupported_transaction"|
            "invalid_utf8_boundary"|"generated_cross_anchor_edit"|
            "boundary_slot_unrepresentable"|"provenance_cycle"|
            "after_replay_mismatch"|"payload_digest_mismatch"}

SourceTransformPortableEvent/3 =
    {kind:"replace",startByte:Counter,endByte:Counter,
     removedByteLength:Counter,removedSha256:"sha256:<64 lowercase hex>",
     replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}
  | {kind:"insert",atByte:Counter,replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}

TransformEmissionPlan/1 =
    {kind:"disabled",
     reason:"no_exact_core_edit_plan"|"transform_profile_unavailable"}
  | {kind:"required",profile:"d6_source_transform_seal/1",
     expectedTrustRevision:Counter,
     expectedTrustKeyId:"sha256:<64 lowercase hex>"}

CoreSourceEditPlan/2 = {
  kind:"d6_core_source_edit_plan",version:2,
  decisionKey:DecisionKey/2,ownerNodeRef:NodeRef,
  beforeObservation:SourceObservation/1,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,transformEmission:TransformEmissionPlan/1
}

SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",version:2,
  decisionKey:DecisionKey/2,changeId:ChangeId/1,ownerNodeRef:NodeRef,
  before:SourceVersion/2,after:SourceVersion/2,
  beforeSourceSha256:"sha256:<64 lowercase hex>",
  afterSourceSha256:"sha256:<64 lowercase hex>",
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}

SourceTransformSealArtifact/1 = {
  format:"weftext.source-transform-seal",version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2,
  signature:"<86 ASCII unpadded base64url>"
}

SourceTransformSealKey/1 = {changeId:ChangeId/1,ownerNodeRef:NodeRef}

SourceTransformSealOutboxItem/1 = {
  kind:"d6_source_transform_seal_outbox_item",version:1,
  key:SourceTransformSealKey/1,artifactPin:PinRef/2
}
```

Plan/evidence edit canonical bytes must match exactly. Seal never recompiles/reorders/merges. A required sealed decision has exactly one outbox item; disabled has none. Mapping uses only original-before coordinates and replacement payload is sliced from the mechanical generatedOutputSpan in afterPin, never content-searched.

# 9. D6 dual-profile trust

```text
SealProfileId/1 =
  "d6_revision_token_seal/1"|"d6_source_transform_seal/1"

WorkspaceTrustDeclaration/2 = {
  kind:"d6_workspace_trust_declaration",version:2,
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:
      {kind:"root",fingerprint:"sha256:<64 lowercase hex>"}
    | {kind:"declaration",revision:Counter,
       sha256:"sha256:<64 lowercase hex>"},
  decisionKey:DecisionKey/2,
  action:
      {kind:"authorize",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:text,possessionSignature:text}
    | {kind:"rotate",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,oldTrustKeyId:"sha256:<64 lowercase hex>",
       newTrustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
       publicKey:text,possessionSignature:text,continuitySignature:text,
       mode:"administrative"|"loss"|"compromise"}
    | {kind:"revoke",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       mode:"administrative"|"loss"|"compromise"}
    | {kind:"resolve_conflict",
       outcomes:[TrustConflictOutcome/2...],
       inheritedCompromises:[TrustConflictCarry/1|TrustConflictCarry/2...]},
  rootSignature:text
}

WorkspaceAuthorizationBundle/2 = {
  kind:"d6_workspace_authorization_bundle",version:2,
  workspaceRef:WorkspaceRef,authorizationRevision:Counter,
  policy:Policy/3,trustRoot:WorkspaceTrustRootDeclaration/1,
  trustRevision:Counter,
  trustDeclarations:[WorkspaceTrustDeclaration/1|WorkspaceTrustDeclaration/2...]
}

DomainSealKeyHandle/2 = {
  kind:"d6_domain_seal_key_handle",version:2,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
  secureHandle:Token,state:"staged"|"usable"|"retired"|"lost"
}

SourceTransformSealVerificationKey/1 = {
  kind:"d6_source_transform_seal_verification_key",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:"d6_source_transform_seal/1",
  trustKeyId:"sha256:<64 lowercase hex>",
  algorithm:"ed25519",publicKey:text
}

TrustConflictCarry/2 = {
  kind:"d6_trust_conflict_carry",version:2,
  factId:"sha256:<64 lowercase hex>",workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  compromisedTrustKeyId:"sha256:<64 lowercase hex>",
  originAction:"revoke"|"rotate",originDecisionKey:DecisionKey/2,
  originDeclarationRevision:Counter,
  originDeclarationDigest:"sha256:<64 lowercase hex>",
  originActivationChangeId:ChangeId/1
}

TrustConflictOutcome/2 =
    {commitDomain:CommitDomain/2,profile:SealProfileId/1,
     state:"keep_current",trustKeyId:"sha256:<64 lowercase hex>"}
  | {commitDomain:CommitDomain/2,profile:SealProfileId/1,state:"none"}
  | {commitDomain:CommitDomain/2,profile:SealProfileId/1,
     state:"authorize_fresh",trustKeyId:"sha256:<64 lowercase hex>",
     algorithm:"ed25519",publicKey:text,possessionSignature:text}

WorkspaceTrustGenesis/2 = {
  kind:"d6_workspace_trust_genesis",version:2,
  rootDeclaration:WorkspaceTrustRootDeclaration/1,
  initialDomainDeclarations:[
    WorkspaceTrustDeclaration/2,
    WorkspaceTrustDeclaration/2
  ]
}
```

Declaration/1 remains revision-token-only. Bundle2 permits a Declaration1 historical prefix; after the first Declaration2, every successor is Declaration2. Declaration1 activation remains CP3. Declaration2 activation is rederived through same-DecisionKey ChangeRecord1 -> exact CP4 -> policy after-image -> Bundle2.

Genesis has exactly two declarations: revision 1 revision-token authorize and revision 2 source-transform authorize, same DecisionKey/activation ChangeId, with rev2 predecessor hashing exact rev1 canonical bytes.

`WorkspaceBootstrapPlan/4` exact members are:

```text
{
  kind:"d6_workspace_bootstrap_plan",wireVersion:4,
  operationId:Uuid,proposalId:Token,
  issuerAuthorityInstanceId:Uuid,targetWorkspaceRef:WorkspaceRef,
  targetAuthorityInstanceId:Uuid,profile:<d6_bootstrap_profile wire4>,
  creatorBinding:<fixed creator binding>,targetRegistry:<complete target Registry>,
  initialPolicy:Policy/3,trustGenesis:WorkspaceTrustGenesis/2,
  initialSeriesConfigurations:[<D6 current series config>...],
  periodScopeBindings:[<D6 current period binding>...]
}
```

Profile4 preserves profile3 principal mapping, Registry, Calendar, base capabilities, and explicit additional capabilities. It only fixes fresh-target genesis to two seal profiles.

Legacy Handle1 may continue new revision-token signing only while its exact Declaration1-authorized key remains current, safe, and usable; it never signs transforms. Continuation evaluates revision and transform profiles independently as current(K)|none|conflicted_or_unproved; any unproved state prevents partial new-domain activation. The declaration order is optional old revision revoke, optional old transform revoke, new revision authorize, new transform authorize, all under one DecisionKey/CP4 with no observable intermediate prefix.

# 10. D10 mixed-version outer schemas

```text
D10WorkspaceReadDependencies/2 = {
  kind:"d10_workspace_read_dependencies",version:2,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  dependencyProof:DependencyProof/3,
  registryReads:[{binding:RegistryBinding/1,snapshot:RegistrySnapshot/1,
                  snapshotPin:PinRef/2}...],
  evidencePins:[PinRef/2...]
}

D10VersionedControlRecordImage/1 =
    {schema:"d10_control_record_image/1",value:D10ControlRecordImage/1}
  | {schema:"d10_control_record_image/2",value:D10ControlRecordImage/2}

D10ControlRecordImage/2 = {
  kind:"d10_control_record_image",version:2,
  binding:Binding<K>/1,scope:Scope/1,
  usageRevision:Option<Counter>,view:ControlCurrentView<K>/1,
  supplement:
      {kind:"automation",subscription:ScheduleSubscription/2,
       stop:Binding<stop>/1,creator:Token}
    | {kind:"run",createdBinding:Binding<run>/1,
       principal:Token,fixedBudget:BudgetCaps/1,
       invocation:Option<AutomationInvocation/1>,
       admission:Option<LeaseRunUse/1>,
       authorSteps:[D10VersionedAuthorStepResponsibility/1...]}
}
```

Image2 only admits automation/run. All other record kinds remain exact Image1.

```text
D10ControlRecordPin/2 = {
  image:D10ControlRecordImage/2,pin:PinRef/2
}

D10VersionedControlRecordPin/1 =
    {schema:"d10_control_record_pin/1",value:D10ControlRecordPin/1}
  | {schema:"d10_control_record_pin/2",value:D10ControlRecordPin/2}

D10ControlRange/2 =
    {kind:"records",version:2,scope:Scope/1,
     kinds:[ControlRecordKind/1],epoch:Token,revision:Counter,
     members:[ControlRef<K>/1]}
  | {kind:"cost_lineage",version:2,key:CostLayerKey/1,
     epoch:Token,revision:Counter,
     reservations:[ControlRef<reservation>/1]}
  | {kind:"occurrences",version:2,
     automation:ControlRef<automation>/1,epoch:Token,revision:Counter,
     records:[D10VersionedAutomationOccurrenceRecord/1...]}

D10VersionedControlRange/1 =
    {schema:"d10_control_range/1",value:D10ControlRange/1}
  | {schema:"d10_control_range/2",value:D10ControlRange/2}

D10VersionedControlPrepareBinding/1 =
    {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
  | {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
  | {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}

ControlPrepareBinding/1 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  dependencyPins:ControlDependencies/1
}

ControlPrepareBinding/2 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/2
}

ControlPrepareBinding/3 = {
  kind:"d10_control_prepare_binding",version:3,
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/3
}

ControlDependencies/3 = {
  kind:"d10_control_dependencies",version:3,
  configBindings:[Binding<K>/1],
  usageBindings:[{ref:ControlRef<lease|approval|grant|cost_account>/1,
                  usageRevision:Counter}...],
  authorityProof:Token,authorizationGenerations:[Token...],
  stopRefs:[ControlRef<stop>/1...],
  workspaceReads:Option<D10WorkspaceReadDependencies/2>,
  recordPins:[D10VersionedControlRecordPin/1...],
  controlRanges:[D10VersionedControlRange/1...]
}

D10ControlEffectPlan/2 = {
  kind:"d10_control_effect_plan",version:2,
  changes:[{before:Option<D10VersionedControlRecordImage/1>,
            after:D10VersionedControlRecordImage/1}...],
  activationSelector:Option<{before:D10ActivationSelector/1,
                             after:D10ActivationSelector/1}>,
  recordPins:[D10VersionedControlRecordPin/1...],
  registryChange:Option<{
    before:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    after:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    evolution:RegistryEvolutionProof/1,
    beforePin:PinRef/2,afterPin:PinRef/2}>
}
```

Old Image1 automation/run -> new Image2 is a valid current configure path; the old before-image is never re-encoded as Image2.

```text
D10VersionedAuthorStepResponsibility/1 =
    {schema:"d10_author_step_responsibility/1",
     value:D10AuthorStepResponsibility/1}
  | {schema:"d10_author_step_responsibility/2",
     value:D10AuthorStepResponsibility/2}

D10VersionedScheduleSubscription/1 =
    {schema:"d10_schedule_subscription/1",value:ScheduleSubscription/1}
  | {schema:"d10_schedule_subscription/2",value:ScheduleSubscription/2}

D10VersionedAutomationOccurrenceRecord/1 =
    {schema:"d10_automation_occurrence_record/1",
     value:AutomationOccurrenceRecord/1}
  | {schema:"d10_automation_occurrence_record/2",
     value:AutomationOccurrenceRecord/2}

D10VersionedApprovalUse/1 =
    {schema:"d10_approval_use/1",value:ApprovalUse/1}
  | {schema:"d10_approval_use/2",value:ApprovalUse/2}

D10ExecutionClaims/2 = {
  kind:"d10_execution_claims",version:2,
  recordPins:[D10VersionedControlRecordPin/1...],
  prepareBindings:[D10VersionedControlPrepareBinding/1...],
  leaseRuns:[LeaseRunUse/1...],
  authorSteps:[D10VersionedAuthorStepResponsibility/1...],
  subscriptions:[D10VersionedScheduleSubscription/1...],
  occurrenceRecords:[D10VersionedAutomationOccurrenceRecord/1...],
  ranges:[D10VersionedControlRange/1...],
  continuityPins:[PinRef/2...]
}

D10MoneyResponsibility/2 = {
  kind:"d10_money_responsibility",version:2,
  reservations:[{binding:Binding<reservation>/1,
                 value:CostReservation/1,
                 attribution:CostBudgetAttribution/1,
                 settlements:[CostSettlementDecision/1...]}...],
  layers:[CostLayerTotal/1...],
  recordPins:[D10VersionedControlRecordPin/1...],
  ranges:[D10VersionedControlRange/1...],
  evidencePins:[PinRef/2...]
}

D10ExecutionInventory/2 = {
  kind:"d10_execution_inventory",version:2,
  workspaceRef:WorkspaceRef,storeIncarnation:Uuid,
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  stopCapacity:StopCapacity/1
}

ExecutionResponsibilityRecord/3 = {
  kind:"d6_execution_responsibility",version:3,
  workspaceRef:WorkspaceRef,executionDomainId:Uuid,
  holder:
      {kind:"local_replica",replicaEpoch:Uuid}
    | {kind:"server",authorityInstanceId:Uuid,deploymentId:Uuid},
  revision:Counter,status:"active"|"paused"|"transferring",
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  lastContinuityProof:ExecutionContinuityProof/2
}

ExecutionContinuityProof/2 =
    {kind:"initial",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,birthProofToken:Token}
  | {kind:"checkpoint",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,barrierToken:Token}
  | {kind:"handoff",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,fromHolder:ExecutionHolder,
     toHolder:ExecutionHolder,oldRevision:Counter,
     barrierToken:Token,oldHolderFenceToken:Token}
```

The Inventory2 pin domain is `UTF8("D6-Execution-Inventory/2") || NUL || D3-CJ/3(D10ExecutionInventory/2)`. Record2/Proof1/Inventory1 keep their historical domain. prepareBindings sort uniquely by inner StableControlKey across all 1/2/3 versions; other mixed arrays use their semantic identity across versions and never LWW.

## 10.1 D10 current schedule / author-step direct types

These are the exact current inner values referenced by the §10 mixed wrappers; they are not free objects invented by the wrappers.

```text
D10ControlInput/2 = {
  kind:"d10_control",
  version:2,
  key:StableControlKey/1,
  operationId:Uuid,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  preview:ControlPreview/1,
  dependencies:ControlDependencies/3,
  effectPlan:D10ControlEffectPlan/2,
  confirmationRequirement:ExternalConfirmationRequirement/1
}

ScheduleRecurrenceEvidence/2 = {
  kind:"d10_schedule_recurrence_evidence",
  version:2,
  observation:SourceObservation/1,
  sourcePin:PinRef/2,
  metadataPin:PinRef/2,
  dependencyProof:DependencyProof/3,
  registrySnapshot:RegistrySnapshot/1,
  registryEvolution:Option<RegistryEvolutionProof/1>,
  recurrenceContext:RecurrenceReadContext/1
}

ScheduleSourceBinding/2 =
    {kind:"once",atUtcSeconds:CanonicalDecimal}
  | {kind:"recurrence",
     ownerNodeRef:NodeRef,
     recurrenceOccurrenceKey:occurrenceKey,
     rangeOccurrenceKey:occurrenceKey,
     initial:ScheduleRecurrenceEvidence/2,
     checkpoint:ScheduleRecurrenceEvidence/2,
     continuityPins:[PinRef/2...]}

ScheduleSubscription/2 = {
  kind:"d10_schedule_subscription",
  version:2,
  automation:ControlRef<automation>/1,
  generation:Counter,
  lowerOriginalStartUtcSeconds:CanonicalDecimal,
  activeDefinition:Binding<automation>/1,
  definitionRevision:Counter,
  source:ScheduleSourceBinding/2
}

ScheduleOccurrenceProof/2 =
    {version:2,kind:"once",at:zoned_instant}
  | {version:2,kind:"recurrence",
     evidence:ScheduleRecurrenceEvidence/2,
     projection:<complete accepted D4 recurrence projection outcome>,
     originalStart:<the selected outcome row's D4 originalStart>}

AutomationOccurrenceRecord/2 = {
  kind:"d10_automation_occurrence_record",
  version:2,
  key:AutomationOccurrenceKey/1,
  definition:Binding<automation>/1,
  definitionRevision:Counter,
  dueUtcSeconds:CanonicalDecimal,
  proof:ScheduleOccurrenceProof/2,
  disposition:AutomationOccurrenceDisposition/1
}

ScheduleContinuityWitness/2 = {
  kind:"d6_schedule_continuity",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  revision:Counter,
  initial:ScheduleRecurrenceEvidence/2,
  checkpoint:ScheduleRecurrenceEvidence/2,
  status:"continuous"|"binding_changed"|"gap",
  producerEpoch:Token,
  consumedTransition:Counter
}

ScheduleContinuityStep/2 = {
  kind:"d6_schedule_continuity_step",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  before:ScheduleRecurrenceEvidence/2,
  after:ScheduleRecurrenceEvidence/2,
  portableChanges:[{
    changeRecordPin:PinRef/2,
    installationNoticePin:PinRef/2,
    completionProofPin:PinRef/2
  }...],
  dependencyBefore:DependencyProof/3,
  dependencyAfter:DependencyProof/3,
  retainedInputs:[PinRef/2...]
}

D10AuthorPreparationLink/2 = {
  kind:"d10_author_preparation_link",
  version:2,
  run:ControlRef<run>/1,
  stepId:Counter,
  automation:Binding<automation>/1,
  definitionRevision:Counter,
  taskDigest:Sha256,
  preparedBindingToken:Token,
  request:d6_commit_request/2
}

ApprovalUse/2 = {
  version:2,
  approval:Binding<approval>/1,
  run:ControlRef<run>/1,
  stepId:Counter,
  request:d6_commit_request/2,
  decisionKey:DecisionKey/2,
  preparedBindingToken:Token,
  previewSemanticDigest:Sha256,
  delegationBinding:Binding<lease>/1,
  activationBinding:ActivationBinding/1,
  count:ApprovalCountReservation/1,
  budgetReservations:[ControlRef<reservation>/1...]
}

D10AuthorStepResponsibility/2 =
    {version:2,kind:"core_field_member",
     link:D10AuthorPreparationLink/2,
     decisionKey:DecisionKey/2,protocolOwner:"D6",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
  | {version:2,kind:"interactive",
     run:ControlRef<run>/1,stepId:Counter,decisionKey:DecisionKey/2,
     authorRequest:
         {protocolOwner:"D3",request:<complete identity_operation_request wire13>}
       | {protocolOwner:"D6",request:d6_commit_request/2},
     preparedFormat:
       "d7_prepared_action_binding4"|"d8_prepared_edit_binding3",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
```

`ApprovalUse/2.preparedBindingToken` must select the exact PAB4. Its preview digest is
`SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3))`;
each EffectBytes/3 slot is projected only as `{encoding,byteLength,payloadDigest}`, never discovered by recursively guessing member names.

A current schedule proof that semantically parses a managed Document must include both `source` and `document_format` dependencies. If source/profile bytes are unchanged but format-proof continuity has a gap, the result is `gap`; a real binding transition is `binding_changed` even when the final recurrence/range value happens to compare equal. A Subscription1 may become a same-generation Subscription2 only through explicit continue plus complete history proving no format/rule discontinuity; otherwise replace is required.

# 11. Historical dispatch and one authority set

Saved work replays under its original owner/version. Planned work restores the original descriptor/proof/prepared/pins/Notice/install/version basis. Unknown work retains original Approval/Money/claim/external/stop liabilities. Only unseen work uses current successors. Old bytes/pins are never re-encoded merely because they are carried by a mixed wrapper.

All successors continue through the original single DecisionKey, planning CAS, install, P seal, receipt, and outbox. document_format is not a second Document authority; SourceTransform is not source authority; ChangeRecord is not a second portable truth; a D10 version wrapper grants no authority; Derived Index never reconstructs current proof.

# 12. Provider profiles

Provider closed shapes are in SPEC §4. They qualify ecosystem renderers and never alter core-language validity. Mermaid's fixed source is CLI 12.0.0 commit `db1ceebbe529d7975474eb0d0e9c23e9dc57cd37` with fixed package.json and package-lock Git blobs. Node/browser/fonts/config still require exact runtime version/digest registration. Without a registered runtime the provider is unavailable; floating semver, host browser/font defaults, or network installation cannot fill the gap.

# 13. Acceptance reference

Every schema and cross-field rule above has explicit positive/negative design obligations in [ACCEPTANCE.md](ACCEPTANCE.md): 437 core + 117 coordination = 554, all unexecuted. Those row bodies are normative obligations; the count is not a substitute for them.
