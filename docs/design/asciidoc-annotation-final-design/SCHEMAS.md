---
source_language: zh-CN
translation_of: SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](SCHEMAS.zh-CN.md)

# AsciiDoc / Annotation Final Design: Closed Schemas and Current Cross-Owner Contracts

Status: **candidate-design-not-implemented**. This is the normative closed-schema companion to [SPEC.md](SPEC.md). SPEC owns behavioral algorithms; this file freezes current successor members, unions, ordering, and historical dispatch. A named closed shape here does not permit unlisted members. Unchanged nested named types such as WorkspaceRef, NodeRef, Frontier/2, and PinRef/2 import the exact decoder from the fixed-parent owner named by replacements.json; that is an explicit unchanged type import, not an omitted successor member list.

Fixed parent: e8aa0b341630a57c786c0891d4bbd1620247441d.

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
  sourceEncoding:"UTF-8",
  localeProfile:ProcessorLocaleProfile/1,
  attributeOverrides:[ProcessorAttributeOverride/1...],
  timeInputs:ProcessorTimeInputs/1,
  includeEnvironment:IncludeEnvironment/2,
  extensionProfiles:[AcceptedProcessorExtensionProfile/1...],
  providerProfiles:[AcceptedProcessorProviderProfile/1...]
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

ProcessorLocaleProfile/1 = {
  profileId:text,
  lang:text|null,
  lcAll:text|null,
  lcCtype:text|null,
  lcTime:text|null,
  descriptorSha256:"sha256:<64 lowercase hex>"
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

AcceptedProcessorProviderProfile/1 = {
  providerKind:"stem"|"mermaid"|"syntax_highlighter"|"html"|"pdf"|"docx",
  profileId:text,
  semanticVersion:text,
  descriptorSha256:"sha256:<64 lowercase hex>"
}
```

attributeOverrides is an ordered sequence, never sorted or deduplicated. Each event name is the non-empty lowercase name after fixed-Ruby key-syntax processing. hard_set/soft_set require non-null value; hard_unset/soft_unset require null. The canonical decoder reconstructs ordered attr_overrides in array order: the first occurrence appends the key and a later same-name event replaces state without moving that key. D3-CJ/3 therefore preserves API enumeration order directly. Environments that differ only by the first-key order of notitle/showtitle have different canonical bytes. Embedded-mode alias derivation uses that ordered key sequence exactly as fixed 2.0.26 does; sorting before derivation is invalid.

sourceEncoding is exactly UTF-8; other source encodings are outside this Gate. localeProfile is a complete registered controlled-oracle descriptor: profileId+descriptorSha256 and the four explicit locale-environment values freeze it with no ambient-host fallback. extensionProfiles sort uniquely by profileId, providerProfiles by (providerKind,profileId), and sourceUnits by logicalPath.value. An empty providerProfiles means no provider was invoked for this evaluation, not that providers do not exist. With safe mode below server, an unoverridden user-home comes from ambientUserHome; server/secure use the native "." default. SOURCE_DATE_EPOCH, when present, drives local*/doc* UTC values; otherwise local* uses clockNow and doc* uses inputMtime when available, then clockNow.

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

Common block slots are exactly id, style, roles, options, caption, numeral, subs, positional, named/<original-name>. Facts already carried by CoreBlock.kind/level/title/reftext/body/children are not duplicated.

```text
id
style
roles
options
caption
numeral
subs
positional
named/<original-name>
```

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

Authored foo-option=bar yields both option membership and named/foo-option="bar"; an authored empty value preserves the empty string. Physical empty foo-option created by %foo/options=foo/opts=foo expresses membership only. Temporary/internal writers such as DocBook root-option never become authored merely by key name.

# 4. Witness/7 retained evidence and model evidence

## 4.1 Oracle baseline, observer manifest, and retained closed evidence types

Every type in this subsection is **test-oracle evidence**, not product persistent authority, a second parser, or a renderer wire. It records facts already produced by the fixed Ruby 2.0.26 evaluation. The observer must not rerun parsers/getters, mutate evaluator strings, replace converter return values, insert markers/sentinels, or write semantic state merely to populate evidence.

```text
OracleBaseline/1 = {
  kind:"asciidoctor_ruby_baseline",
  version:1,
  repository:"asciidoctor/asciidoctor",
  release:"2.0.26",
  sourceCommit:"0b99b39c9df884d4aec13bba45f03cdbab505769",
  observerContract:"ruby_actual_evaluation_observer/1",
  observerManifestSha256:"sha256:<64 lowercase hex>"
}

OracleObserverManifest/1 = {
  kind:"ruby_actual_evaluation_observer_manifest",
  version:1,
  sourceCommit:"0b99b39c9df884d4aec13bba45f03cdbab505769",
  rubyRuntime:{implementation:text,version:text,buildId:text},
  sites:[OracleObserverSite/1...]
}

OracleObserverSite/1 = {
  siteId:UInt,
  file:text,
  fileBlobSha1:"<40 lowercase hex>",
  method:text,
  callSite:text,
  observedOperation:text,
  captureRule:text,
  rangeTransferRule:text
}

OracleDocumentState/2 = {
  backend:text,
  basebackend:text,
  doctype:"article"|"book"|"manpage"|"inline",
  safeMode:"unsafe"|"safe"|"server"|"secure",
  doctitle:{combined:text|null,main:text|null,subtitle:text|null},
  authors:[{name:text,firstname:text|null,middlename:text|null,
            lastname:text|null,initials:text|null,email:text|null}...],
  revision:null|{number:text|null,date:text|null,remark:text|null},
  attributes:[{name:text,value:text}...]
}

OracleBlockContext/1 =
    "admonition"|"audio"|"colist"|"dlist"|"document"|"example"|
    "floating_title"|"image"|"listing"|"literal"|"olist"|"open"|
    "page_break"|"paragraph"|"pass"|"preamble"|"quote"|"section"|
    "sidebar"|"stem"|"table"|"thematic_break"|"toc"|"ulist"|
    "verse"|"video"

OracleContentModel/1 =
    "compound"|"simple"|"verbatim"|"raw"|"empty"

OracleBlockEvent/2 = {
  ordinal:UInt,
  parentOrdinal:UInt|null,
  context:OracleBlockContext/1,
  contentModel:OracleContentModel/1|null,
  level:CanonicalSignedDecimal|null,
  style:text|null,
  id:text|null,
  title:text|null,
  roles:[text...],
  options:[text...],
  attributes:[{name:text,value:text}...]
}

OracleCollectionEvent/1 =
    {kind:"list_item",listOrdinal:UInt,itemOrdinal:UInt,
     listContext:"ulist"|"olist"|"colist"|"dlist_term"|"dlist_description",
     marker:text|null,style:text|null,text:text|null,
     checklist:"none"|"checked"|"unchecked"}
  | {kind:"table_cell",tableOrdinal:UInt,section:"head"|"body"|"foot",
     rowOrdinal:UInt,columnOrdinal:UInt,colspan:UInt|null,rowspan:UInt|null,
     style:text|null,text:text|null}

OracleCatalogEvent/1 =
    {kind:"ref",id:text,reftext:text|null,
     targetKind:"section"|"inline_ref"|"bibref"|"other"}
  | {kind:"footnote",index:UInt,id:text|null,text:text|null}
  | {kind:"link",target:text}
  | {kind:"image",target:text}
  | {kind:"include",target:text}
  | {kind:"callout",listOrdinal:UInt,itemOrdinal:UInt}

OracleDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error"|"fatal",
  semanticCode:text,
  exactMessage:text,
  logicalFile:text|null,
  line:UInt|null
}

OracleObservedString/1 = {
  valueId:UInt,
  rubyEncoding:text,
  bytesBase64:text,
  byteLength:UInt
}

OracleObservedSlice/1 = {
  valueId:UInt,
  startByte:UInt,
  endByte:UInt
}

OracleEvaluationCallKind/1 =
    "parse"|"attribute_playback"|"content"|"title"|"list_text"|
    "cell_text"|"cell_content"|"apply_subs"|"substitution_stage"|
    "inline_convert"|"block_convert"|"metadata_access"|"effect"

OracleEvaluationCall/1 = {
  callId:UInt,
  parentCallId:UInt|null,
  siteId:UInt,
  nodeOrdinal:UInt|null,
  invocationOrdinal:UInt,
  kind:OracleEvaluationCallKind/1
}

OracleStringOperationKind/1 =
    "copy"|"slice"|"concat"|"join"|"regex_replace"|"split_piece"|
    "character_map"|"trim"|"delete"|"normalize"|"encode"|
    "converter_return"|"scalar_assign"

OracleOutputRunRelation/1 =
    {kind:"exact_copy",input:OracleObservedSlice/1}
  | {kind:"derived",inputs:[OracleObservedSlice/1...],operationId:UInt}
  | {kind:"generated",siteId:UInt,inlineEventId:UInt|null}

OracleOutputRun/1 = {
  startByte:UInt,
  endByte:UInt,
  relation:OracleOutputRunRelation/1
}

OracleStringOperation/1 = {
  operationId:UInt,
  callId:UInt,
  siteId:UInt,
  kind:OracleStringOperationKind/1,
  inputs:[OracleObservedSlice/1...],
  outputValueId:UInt,
  runs:[OracleOutputRun/1...],
  removedInputs:[OracleObservedSlice/1...]
}

OracleFieldPathSegment/1 =
    {kind:"field",name:text}
  | {kind:"attribute",name:text}
  | {kind:"position",index:UInt}
  | {kind:"array",index:UInt}

OracleFieldEntryKey/1 =
    {kind:"name",value:text}
  | {kind:"position",index:UInt}
  | {kind:"symbol",value:text}

OracleFieldValue/1 =
    null
  | {kind:"boolean",value:Boolean}
  | {kind:"integer",decimal:CanonicalSignedDecimal}
  | {kind:"symbol",value:text}
  | {kind:"observed_string",valueId:UInt}
  | {kind:"array",items:[OracleFieldValue/1...]}
  | {kind:"entries",items:[{key:OracleFieldEntryKey/1,
                             value:OracleFieldValue/1}...]}

OracleFieldObservation/1 = {
  path:[OracleFieldPathSegment/1...],
  value:OracleFieldValue/1
}

OracleInlineContext/1 =
    "anchor"|"break"|"button"|"callout"|"footnote"|"image"|
    "indexterm"|"kbd"|"menu"|"quoted"

OracleInlineObservation/3 = {
  eventId:UInt,
  callId:UInt,
  parentCallId:UInt|null,
  producingOperationId:UInt|null,
  context:OracleInlineContext/1,
  nodeType:OracleFieldValue/1,
  fields:[OracleFieldObservation/1...],
  returnValueId:UInt|null
}

OracleContentRole/1 =
    "body"|"title"|"list_item"|"table_cell"|
    "footnote_body"|"reftext"|"raw_content"

OracleContentObservation/1 = {
  callId:UInt,
  role:OracleContentRole/1,
  resultValueIds:[UInt...],
  contributingInlineEventIds:[UInt...]
}

M37Site/1 = {
  group:
    "M37-01"|"M37-02"|"M37-03"|"M37-04"|"M37-05"|"M37-06"|
    "M37-07"|"M37-08"|"M37-09"|"M37-10"|"M37-11"|"M37-12"|
    "M37-13"|"M37-14"|"M37-15"|"M37-16"|"M37-17"|"M37-18",
  file:text,
  method:text,
  point:text
}

OracleSemanticWitness/7 = {
  format:"weftext.asciidoc-oracle-witness",version:7,
  baseline:OracleBaseline/1,
  processorEnvironmentSha256:"sha256:<64 lowercase hex>",
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

SemanticHeadSubjectKind/1 =
  "document"|"section"|"block"|"list"|"list_item"|"table"|
  "column"|"cell"|"inline"|"catalog_record"

SupportingOnlySubjectKind/1 =
  "attribute_buffer"|"table_parser_context"

ForwardSemanticRole/1 =
  "header"|"child"|"dlist_term"|"dlist_description"|
  "table_column"|"table_head_cell"|"table_body_cell"|"table_foot_cell"|
  "cell_inner_document"

BackedgeSemanticRole/1 =
  "parent"|"cell_column"

ModelRelation/1 = {
  role:ForwardSemanticRole/1|BackedgeSemanticRole/1,
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

## 4.2 Retained evidence canonical rules and producer manifests

`OracleBaseline/1.observerManifestSha256` must equal `SHA-256(D3-CJ/3(OracleObserverManifest/1))`. `processorEnvironmentSha256` must equal SHA-256 of the exact canonical `AsciiDocProcessorEnvironment/3` bytes. The manifest is not caller-selected coverage: siteId is unique and numerically sorted; fileBlobSha1 must be the real Git blob at the fixed sourceCommit; method/callSite/observedOperation/captureRule/rangeTransferRule must identify the independently reviewed instrumentation point. A missing site, uncovered required branch, or manifest/source mismatch is `oracle_observation_incomplete`, never permission to narrow valid AsciiDoc.

The complete actual-evaluation observer producer families are:

| producer family | required actual facts |
|---|---|
| `AbstractBlock#convert` | Original conversion call after attribute playback and node/parent association; converter is not replaced. |
| `Block#content` | Real input lines, content model, original apply_subs invocation and final return, including raw/verbatim trimming/join. |
| `Substitutors#apply_subs` | Actual substitution order, each stage input/output, and passthrough extract/restore linkage. |
| `sub_quotes / convert_quoted_text` | Original MatchData/captures, Inline constructor arguments, and actual converter return. |
| `sub_attributes / sub_replacements` | Original match ranges, replacement results, drop/drop-line behavior, and counter/set effects. |
| `sub_macros` | Actual macro match/captures and Inline text/target/id/refid/path/attributes. |
| `sub_post_replacements` | Actual split/slice/HardLineBreakRx input ranges and complete break text. |
| `Inline#convert / selected converter` | Actual context/type/scalar arguments and return value with no observer-added getter. |
| `AttributeList` | Original StringScanner positions/scan/get-byte results and produced scalars; no second attribute-list parse. |
| `ListItem#text / Cell#text/#content` | Original getter calls, actual outputs, and a-cell inner-document call relationship. |
| `title / reftext` | Original first computation, cache hit, and actual scalar return without observer-triggered reads. |
| footnote/counter/catalog mutation sites | Original operation inputs and actual results without executing them again. |

`OracleObservedString/1.valueId`, `OracleEvaluationCall/1.callId`, `OracleStringOperation/1.operationId`, and `OracleInlineObservation/3.eventId` are unique only within their own array namespaces. Equal bytes from distinct origins receive distinct valueIds; an in-place-mutated Ruby String produces a new immutable snapshot. bytesBase64 is canonical padded RFC4648 Base64 whose decoded length equals byteLength. A slice is a byte half-open interval on that exact string version and must satisfy `0 <= startByte <= endByte <= byteLength`; evaluation-string positions are never .adoc source ranges.

`OracleStringOperation/1.runs` sort by startByte, do not overlap, and completely cover the actual output. exact_copy copies its input slice byte-for-byte; derived preserves all real inputs and never pretends to have one authored offset; generated names the actual site and optional inline observation. removedInputs retain real deleted inputs, so a zero-output concealed construct is not lost. `OracleOutputRunRelation/1.generated.inlineEventId` and `OracleContentObservation/1.contributingInlineEventIds` refer only to `OracleInlineObservation/3.eventId`; the retained field names neither recreate nor decode as the retired `OracleInlineEvent`/marker observer.

`OracleInlineObservation/3.fields` sort uniquely by `D3-CJ/3(path)`; every string leaf references a real ObservedString. OracleFieldValue has no arbitrary Ruby-object `to_s` escape. ContentObservation resultValueIds preserve the original String/Array return structure; ordinary text comes from real content/list/cell/title/reftext returns rather than HTML flattening. BlockEvent roles/options preserve Ruby order while attributes sort uniquely by name; DocumentState attributes also sort uniquely by name. CatalogEvent has no indexterm fallback because fixed 2.0.26 does not retain `:indexterms` in the catalog. Diagnostic exactMessage remains evidence; the cross-implementation CSP compares only the defined common diagnostic projection.

The M37 writer-site set is closed. `M37Site/1`'s `(group,file,method,point)` must match the fixed producer family for its row; point is a stable branch identifier from the reviewed manifest, not a free string:

| group | fixed producer family | capture point |
|---|---|---|
| M37-01 | `AbstractNode#initialize`, `AbstractBlock#initialize`, `Inline#initialize` | After original constructor assignment. |
| M37-02 | `AttributeList#parse_attribute/#parse_into/.rekey` | After real named/positional/options expansion/copy/rekey writes. |
| M37-03 | `Parser.parse_block_metadata_line/process_attribute_entry/store_attribute/parse_style_attribute/yield_buffered_attribute` | After actual metadata/shorthand/set/unset writes. |
| M37-04 | `Parser.next_block/build_block` | After style/media/source/quote/admonition/STEM assignment and before return. |
| M37-05 | `AbstractNode#set_attr/remove_attr/set_option/update_attributes/role=/add_role/remove_role` plus real direct Hash writes | After the original mutation while preserving real caller provenance. |
| M37-06 | `Parser.initialize_section` | After sectname/special/numbered/update_attributes. |
| M37-07 | `AbstractBlock#assign_numeral/#assign_caption` and section attach | After numeral/caption/counter writes. |
| M37-08 | `Table#initialize/#create_columns` | After table width/orientation/columns/colcount exist. |
| M37-09 | `Table::Column#initialize/#assign_width`, `Table#assign_column_widths` | After each real width write; final group snapshot only after last-column balancing. |
| M37-10 | `Table::Cell#initialize/#reinitialize`, `Table#partition_header_footer` | After the final Cell object and head/body/foot membership are known. |
| M37-11 | `Parser.parse_colspecs/parse_cellspec/parse_table`, `Table::ParserContext#initialize/#close_cell/#close_row/#close_table` | After original specs/results and real column/row/Cell relations exist. |
| M37-12 | `ListItem#initialize/#fold_first`, `Parser.parse_list_item/parse_list/parse_description_list` | After marker/checklist/style/fold and term-description assembly. |
| M37-13 | `Parser.parse_callout_list`, original `Callouts#register/#callout_ids` return sites | After late coids are really written. |
| M37-14 | `Document#parse`, original `AbstractBlock#convert`, `Inline#convert`, and collection-consumer entries | model_ready cutoff without extra parse/convert. |
| M37-15 | Actual inline/converter/alt/title/reftext/media scalar consumer sites | Reuse existing evaluation evidence and bind the contemporaneous model/attribute state. |
| M37-16 | Actual `Document#register` catalog insertion | Catalog primitive fields and parent→actual receiver. |
| M37-17 | Fixed-converter temporary model/map writes | After temporary writes and after the real restore or cleanup delete. |
| M37-18 | Actual root-evaluation return | evaluation_complete cutoff, observing only already-produced state. |

M37-02/M37-03/M37-05 must also cover direct Hash assignment/update/delete/clear in the fixed source, not merely wrapper calls. The concrete manifest plus the fixed source is producer-conformance evidence; the projector cannot infer missing facts from field names, source text, or final HTML.

### 4.3 Exact Witness/7 closure verifier

**A. Observation/snapshot/bind static layer.** modelPropertyObservations is in actual append order and observationId is strictly increasing. For each subject, sort all snapshots by observationId: the first previousSnapshotObservationId is null and every later one points exactly to the directly preceding snapshot of the same subject. The same subjectId has byte-equal kind in every snapshot; real-Ruby-object to subjectId one-to-one identity is producer conformance. Define latestSnapshotBefore(s,o) as the greatest same-subject snapshot observationId below o. Every bind resolves an existing same-subject latestSnapshotBefore(subjectId,bind.observationId).

Static ModelCarrierReference dispatch is exactly:
- document: entry is 0 and resolves Witness.document;
- blockEvents / collectionEvents / catalogEvents / inlineObservations / contentObservations / calls: entry is a zero-based index in that exact array and the target has the stream's exact type.

carrier.entry is never compared numerically with a model observationId.

**B. Root and physical semantic closure.** For cut C, rootBind(C) is the latest valid pre-cut bind with carrier=(document,0); it exists, its subject kind is document, and C.documentSubjectId=rootBind.subjectId. For every candidate subject s, latest[s]=latestSnapshotBefore(s,C.observationId).

The structural worklist starts at C.documentSubjectId and expands only ForwardSemanticRole/1 from latest[s]. Every target exists and has SemanticHeadSubjectKind/1. After forward traversal, every parent/cell_column backedge from a latest snapshot in Rstruct already targets Rstruct; a backedge never adds a subject. Former roots, old Cells, and other stale objects cannot be revived through reverse edges.

Then exactly two supplementary passes run:
1. inline: kind=inline; at least one valid pre-cut bind targets inlineObservations; latest snapshot has exactly one parent whose target is already in Rstruct. Add it to R.
2. catalog_record: at least one valid pre-cut bind targets catalogEvents; latest snapshot has exactly one catalog-ownership parent; target kind=document and target is already in Rstruct. Add it to R. There is no owner role and a Document outside Rstruct is never pulled in.

SupportingOnlySubjectKind/1 never enters R. After supplementary addition, every backedge in R again targets a subject already in R. Let H=set(C.heads[].subjectId). heads are strictly sorted by subjectId, subjectId is unique, H==R, and every head.snapshotObservationId equals latest[head.subjectId].observationId. Missing live subjects, extra stale subjects, duplicates, stale heads, or relation conflicts invalidate/incomplete the Witness.

**C. Supporting evidence closure.** Seeds are exactly all cut-head snapshots, rootBind(C), and every valid pre-cut bind whose subjectId is in R. Recurse typed edges until fixed point:
- snapshot -> previousSnapshotObservationId; every field/attribute provenance.inputs; all nested string/array/map ModelObservedValue members;
- ModelPropertyInput.model_slot -> named model snapshot plus required slot; observed_value -> observedStrings[valueId]; observed_slice -> observedStrings[valueId] plus byte-range validation; operation -> operations[operationId]; inline_field -> inlineObservations[inlineEventId] plus fieldPath;
- OracleStringOperation -> calls[callId], every input/removed slice, observedStrings[outputValueId]; run.exact_copy -> slice; run.derived -> all slices plus referenced operationId; run.generated -> non-null inlineEventId;
- OracleEvaluationCall -> non-null parentCallId;
- OracleInlineObservation -> callId, non-null parentCallId, non-null producingOperationId, recursively nested OracleFieldValue in nodeType/fields, non-null returnValueId;
- OracleFieldValue.observed_string -> observedStrings[valueId]; array/entries -> every nested value;
- OracleContentObservation -> calls[callId], all resultValueIds, all contributingInlineEventIds;
- OracleObservedSlice -> observedStrings[valueId] and 0<=startByte<=endByte<=byteLength; OracleObservedString is a leaf;
- bind -> its carrier; Inline/Content/Call carriers continue through the edges above; other carriers apply their retained closed validation with no invented cross-array chronology.

Same-namespace model references use backward/latest checks; operation/value/inline/call stay in their own existence/type/DAG namespaces; subject references use identity/relation only. Cross-namespace numeric comparison is forbidden. A dangling/wrong-kind reference, invalid slice/index, operation/call cycle, or missing provenance input fails. The support closure may contain SupportingOnlySubjectKind/1, old snapshots/Cells, and temporary evidence without adding them to H.

**D. Producer time and cardinality.** The static decoder proves final stream/index/type and the reference/closure rules above; complete Witness validity additionally requires producerConformanceValid. At every real bind callback the carrier has already been appended, entry < targetStream.lengthAtBind, the callback still holds the exact same Ruby object, and the bind is appended afterward. document/0 binds only after the actual returned top-level Document carrier is published. Later filling of the carrier array never cures an earlier invalid bind; text/title/source/path/hash matching is forbidden.

A complete top-level Witness/7 has exactly one model_ready cut. If selected-backend evaluation is absent or exits abnormally there are zero valid evaluation_complete cuts; a normal return produces exactly one, later than model_ready with byte-equal documentSubjectId. Inner Documents do not create another top-level cut namespace. model_ready/evaluation_complete heads are the latest physical snapshots at their cuts; physical membership is separate from CoreSemanticPropertyProfile/1 survival/winner semantics.

Catalog ownership remains only parent -> actual Document#register receiver. A temporary overlay closes before a valid evaluation_complete through the fixed source's real restore write or cleanup delete/closure; the observer never fabricates a restore.

## 4.4 D2 current product Document projection

This family is product data, not OracleSemanticWitness/7 or CoreSemanticProjection/1.

```text
D2SourceOwner/1 =
    {kind:"root_document",ownerNodeRef:NodeRef}
  | {kind:"managed_include",ownerNodeRef:NodeRef}
  | {kind:"artifact_include",pin:PinRef/2}
  | {kind:"network_snapshot",pin:PinRef/2}

D2SourceRange/2 = {
  source:D2SourceOwner/1,
  startByte:Counter,endByte:Counter,
  startLine:Counter,startColumn:Counter,
  endLine:Counter,endColumn:Counter
}

D2ProductDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error",
  semanticCode:text,
  message:text,
  sourceRange:D2SourceRange/2|null
}

D2TextProjection/2 = {
  text:text,
  sourceOrigins:[D2SourceRange/2...]
}

D2DocumentMetadata/3 = {
  doctype:"article"|"book"|"manpage"|"inline",
  title:D2TextProjection/2|null,
  subtitle:D2TextProjection/2|null,
  authors:[{name:text,email:text|null}...],
  revision:null|{number:text|null,date:text|null,remark:text|null},
  headerAttributes:[{name:text,value:text|null,
                    sourceRange:D2SourceRange/2}...]
}

D2AttributeCarrierBlock/2 = {
  kind:"attribute_carrier_block",
  namespaceToken:text,
  sourceRange:D2SourceRange/2,
  namespaceRange:D2SourceRange/2,
  entries:[{kind:"lexical_attribute_entry",rawEntrySource:text,
            sourceRange:D2SourceRange/2}...]
}

D2IdentityAdapter/1 =
    {kind:"node_link",target:NodeRef}
  | {kind:"resource",target:ResourceRef}
  | {kind:"citation",target:NodeRef,citationKey:text|null}

D2ProductInline/3 =
    {kind:"text",text:text,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"quoted",style:"strong"|"emphasis"|"monospaced"|"mark"|
                         "superscript"|"subscript"|"double"|"single",
     children:[D2ProductInline/3...],sourceOrigins:[D2SourceRange/2...]}
  | {kind:"link",nativeTarget:text,label:[D2ProductInline/3...],
     adapter:D2IdentityAdapter/1|null,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"xref",nativeTarget:text,label:[D2ProductInline/3...],
     adapter:D2IdentityAdapter/1|null,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"anchor",id:text,reftext:text|null,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"image",nativeTarget:text,alt:text|null,
     adapter:D2IdentityAdapter/1|null,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"footnote",id:text|null,children:[D2ProductInline/3...],
     sourceOrigins:[D2SourceRange/2...]}
  | {kind:"indexterm",terms:[text...],sourceOrigins:[D2SourceRange/2...]}
  | {kind:"stem",notation:"tex"|"asciimath",source:text,
     sourceOrigins:[D2SourceRange/2...]}
  | {kind:"kbd",keys:[text...],sourceOrigins:[D2SourceRange/2...]}
  | {kind:"menu",path:[text...],sourceOrigins:[D2SourceRange/2...]}
  | {kind:"button",label:text,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"callout",label:text,sourceOrigins:[D2SourceRange/2...]}
  | {kind:"line_break",sourceOrigins:[D2SourceRange/2...]}
  | {kind:"passthrough",text:text,sourceOrigins:[D2SourceRange/2...]}

D2Heading/3 = {
  kind:"heading",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:[D2SourceRange/2...],
  authoredLevel:integer(1..9),
  effectiveLevel:Counter,
  inlines:[D2ProductInline/3...],
  anchor:text|null,
  roles:[text...],
  options:[text...]
}

D2ParagraphBlock/3 = {
  kind:"paragraph",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  inlines:[D2ProductInline/3...]
}

D2SectionBlock/3 = {
  kind:"section",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  heading:D2Heading/3,
  children:[D2ProductBlock/3...]
}

D2ListItem/3 = {
  kind:"list_item",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  marker:text|null,checked:Boolean|null,
  terms:[[D2ProductInline/3...]...],
  inlines:[D2ProductInline/3...],
  children:[D2ProductBlock/3...]
}

D2ListBlock/3 = {
  kind:"list",listKind:"unordered"|"ordered"|"description"|"callout",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  items:[D2ListItem/3...]
}

D2TableColumn/3 = {
  ordinal:Counter,width:text|null,halign:text|null,valign:text|null,
  style:text|null,sourceOrigins:[D2SourceRange/2...]
}

D2TableCell/3 = {
  kind:"table_cell",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  colspan:Counter,rowspan:Counter,style:text|null,
  halign:text|null,valign:text|null,
  content:
      {kind:"inline",inlines:[D2ProductInline/3...]}
    | {kind:"blocks",children:[D2ProductBlock/3...]}
}

D2TableRow/3 = {
  kind:"table_row",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  section:"head"|"body"|"foot",
  cells:[D2TableCell/3...]
}

D2TableBlock/3 = {
  kind:"table",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  columns:[D2TableColumn/3...],
  rows:[D2TableRow/3...]
}

D2DelimitedBlock/3 = {
  kind:"delimited",
  blockKind:"listing"|"literal"|"source"|"pass"|"stem"|"quote"|"verse",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  style:text|null,
  content:
      {kind:"text",text:text}
    | {kind:"inline",inlines:[D2ProductInline/3...]}
    | {kind:"blocks",children:[D2ProductBlock/3...]}
}

D2ContainerBlock/3 = {
  kind:"container",
  blockKind:"example"|"sidebar"|"open"|"admonition"|"preamble",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  children:[D2ProductBlock/3...]
}

D2MediaBlock/3 = {
  kind:"media",mediaKind:"image"|"audio"|"video",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  nativeTarget:text,alt:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  adapter:D2IdentityAdapter/1|null
}

D2AtomicBlock/3 = {
  kind:"atomic",
  blockKind:"floating_title"|"page_break"|"thematic_break"|"toc",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:[D2SourceRange/2...],
  level:Counter|null,
  title:[D2ProductInline/3...],
  roles:[text...],options:[text...]
}

D2SavedDefinitionBlock/3 = {
  kind:"saved_query_view_definition",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  definitionKind:"query"|"view"|"dynamic_block",
  payload:text
}

D2BibliographyPlacementBlock/3 = {
  kind:"bibliography_placement",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2
}

D2ProductBlock/3 =
    D2SectionBlock/3 | D2ParagraphBlock/3 | D2ListBlock/3 |
    D2TableBlock/3 | D2DelimitedBlock/3 | D2ContainerBlock/3 |
    D2MediaBlock/3 | D2AtomicBlock/3 |
    D2SavedDefinitionBlock/3 | D2BibliographyPlacementBlock/3

D2DocumentBody/3 = {
  kind:"document_body",
  children:[D2ProductBlock/3...]
}

D2DocumentPayload/3 =
    {source:text,
     parse:{status:"valid",diagnostics:[D2ProductDiagnostic/1...]},
     projection:{state:"available",
                 metadata:D2DocumentMetadata/3,
                 attributeCarrierBlocks:[D2AttributeCarrierBlock/2...],
                 body:D2DocumentBody/3},
     d2CommitEligibility:"eligible",
     repairVisibility:"not_required"}
  | {source:text,
     parse:{status:"invalid",diagnostics:[D2ProductDiagnostic/1...]},
     projection:{state:"unavailable"},
     d2CommitEligibility:"reject",
     repairVisibility:"exact_source_only"}

D2DocumentSnapshot/3 = {
  wireVersion:3,
  kind:"document_snapshot",
  ownerNodeRef:NodeRef,
  document:D2DocumentPayload/3
}
```

All product unions are strict: unknown/missing members, unknown arms, duplicate JSON keys, illegal nulls, and cross-arm members reject. D2ProductBlock/3 is the current complete product block family for the fixed baseline plus the named Weftext extensions; there is no generic/opaque block or inline escape. Semantic sequences preserve parser order. roles/options preserve their language-defined order; unique sets are explicitly named as such by their owner.

For an available projection, every addressable product occurrence has the exact current D3 DocumentElementLocator required by its type. D2SourceRange/2 byte intervals are half-open and line/column endpoints refer to the same source owner; start is not after end. sourceOrigins is nonempty whenever a semantic value is produced from source. A managed include uses its included owner NodeRef, and an artifact/network include uses the exact authorized pin; no path/title/hash infers an owner. A product locator or origin grants no write permission.

BaselineOnly accepts only headings the fixed Ruby baseline actually parses. WeftextManaged additionally admits authoredLevel 6–9. effectiveLevel is the uncapped result of the native section/leveloffset state machine and may exceed 9. D2SectionBlock/3.heading and the corresponding section locator/range refer to the same actual section occurrence. A re-renderer cannot recompute a different level.

D2IdentityAdapter/1 may appear only after the enclosing native occurrence has parsed successfully. node_link and citation targets must be D3-qualified stable identity; resource targets are owner-local and their owner equals the Document/source-owner rule that created the occurrence. The adapter is additional typed identity, never a replacement native parser or a writable source target.

The canonical protected pin for a complete current product snapshot is artifact/recovery over:

```text
UTF8("D2-Document-Snapshot/3") || NUL ||
D3-CJ/3(complete D2DocumentSnapshot/3)
```

Historical document_snapshot wireVersion2 and its limited BodyBlock/Inline decoder remain historical-only and are never widened to accept /3.

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

## 5.1 Dependency family

```text
DependencyKey/3 =
  source(0)|document_format(1)|lifecycle(2)|placement_range(3)|
  ref_inbound(4)|relation_incidence(5)|calendar_scope(6)|registry(7)|
  temporal_rules(8)|authorization(9)|foreign_binding(10)|query_scan(11)|
  replica_registry(12)|conflict_record(13)|execution_resource(14)
```

```text
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

The final unchanged nested names above use the exact fixed-parent PreparedIntent/2 decoder. Current planToken tag is d6_plan/3.

## 5.2 Portable component / Notice / CP

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

PinRef/2 and ComponentImage/1 are unchanged fixed-parent types.

Notice3 inherits every Notice2 invariant except the versioned component-key decoder: components is non-empty, PortableComponentKey/2 fixed-rank/canonical-key sorted and unique, and the notice is frozen before installation with the original baseFrontier. CP4 inherits every CP3 committed/restored invariant except the component-key decoder and current ChangeRecord/1 linkage. For committed CP4, components is exactly the Notice3 key set in identical order, every after is the actual installed/sealed image, sourceChanges is the complete EntityRef-sorted unique real source-state delta, and receiptDigest binds the original receipt. A fresh managed Document carries both document and document_format in the same plan/P/CP4. A format-only change with unchanged source has sourceChanges=[] and creates no SourceRevisionPlan, managed SourceVersion, or H advance.

## 5.3 ChangeRecord

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

CP4 must be committed and match DecisionKey, ChangeId, and frontiers. Notice/CP lengths and digests bind their exact D3-CJ/3 canonical bytes. One P seal fixes ChangeId, CP4, and ChangeRecord; publication only retransmits original pins.

frontierBefore is the actual verified pre-seal Frontier and frontierAfter is exactly frontierBefore plus this ChangeId with no other-domain regression. With frontierPolicy=exact, frontierBefore is byte-equal to the original expected/base/notice Frontier. With scope_dependencies, admission requires the complete continuous verified ChangeRecord/completion chain from Notice3.baseFrontier through frontierBefore and frontierAfter plus the original retained unrelatedness proof; vector numbers, provider sync state, or current files never substitute.

Restored CP4 contains only decisionKey, baseFrontier, and component after images that each equal the corresponding Notice3 before image. It forbids changeId, guarantee, writeProtection, semanticState, frontierBefore, frontierAfter, sourceChanges, receiptDigest, and all success semantics.

Receiver admission strict-decodes the exact Notice3/CP4 versions and canonical bytes, validates equal component key sets/order, obtains and validates every actual component byte and owner version, validates complete production SourceVersion changes, and verifies the continuous chain. A present document_format component additionally strict-decodes exact ManagedDocumentFormatBinding/1 bytes. Missing/unknown bytes or decoder, a component-version mismatch, or a Notice/CP/ChangeRecord mismatch is incomplete/proof_unavailable, never successful admission.

# 6. D3/D7/D8/D9 direct holders

```text
D3ResolutionInputUse/2 = {
  kind:"d3_resolution_input_use",version:2,
  decisionKey:DecisionKey/2,principalAudienceToken:Token,
  bindingToken:Token,inputDescriptor:InputDescriptor/3
}

D3IdentityInput/13 = {
  kind:"d3_identity_input",version:13,
  mode:D3Mode,
  writeProtection:"strict",
  expectedAuthority:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  intent:D3Intent
}

D3IdentityOperationRequest/13 = {
  wireVersion:13,
  kind:"identity_operation_request",
  operationId:UUIDv4,
  boundWorkspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  inputDescriptor:InputDescriptor/3,
  mode:D3Mode,
  expectedAuthority:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  preparationBinding?:PreparationBinding,
  intent:D3Intent
}
```

D3IdentityInput/13 has the exact D3IdentityInput/12 six-member semantic set: only the nested D6 descriptor family changes to current InputDescriptor/3 at the outer request boundary; mode/authority/proposal/intent and writeProtection=strict retain their fixed-parent decoders and mode matrix. Optional members marked ? are absent when the fixed-parent mode matrix forbids them; JSON null is not a substitute. D3IdentityOperationRequest/13 is the exact wire12 top-level member set with wireVersion=13 and InputDescriptor/3. Its cross-field equalities, guarantee/frontier policy, ownerInput protocolOwner=D3/ownerKind=d3_identity_operation/13, requestFingerprint rule, DecisionKey/OperationId ledger ordering, saved/planned/unseen branching, and error/disclosure order are inherited unchanged. Historical wire9–12 are not widened or re-encoded.

## 6.1 PreparedActionBinding/4

The current D7 action successor is exact inheritance, not a free extension. D7ActionSpec/2 is the fixed-parent ActionSpec/1 top-level object with version=2; its intent decoder is exactly the fixed-parent intent union with only the historical apply_suggestion arm removed, then the two closed arms below added. Every non-suggestion arm keeps its fixed-parent members and semantics byte-for-byte.

```text
D7ApplySuggestionIntent/2 = {
  kind:"apply_suggestion",
  annotation:AnnotationRef,
  expectedAnnotationRevisionToken:AnnotationRevisionToken/1
}

D7RejectSuggestionIntent/2 = {
  kind:"reject_suggestion",
  annotation:AnnotationRef,
  expectedAnnotationRevisionToken:AnnotationRevisionToken/1
}

D7ActionSpec/2 :=
  ActionSpec/1 with top-level version=2,
  with the fixed-parent apply_suggestion arm removed,
  plus D7ApplySuggestionIntent/2 and D7RejectSuggestionIntent/2

D7ActionPrepareRequest/3 = {
  wireVersion:3,kind:"d7_action_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  action:D7ActionSpec/2,
  selectedSources:[SourceVersionRef/1...],
  budget:BudgetBinding/1,
  evidenceToken?:Token
}

PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding",version:4,
  bindingToken:Token,protocolOwner:"D3"|"D6",
  operationId:UUIDv4,workspaceRef:WorkspaceRef,
  principalAudienceToken:Token,action:D7ActionSpec/2,
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
  request:D3IdentityOperationRequest/13|d6_commit_request/2,
  preview:<complete EffectManifest/3>,
  resolutionInput:null|D3ResolutionInput/1
}
```

MinimumMapping/3, D7DefinitionInput/2, D7ProposedInput/2, and D7ResolutionAccess/1 remain unchanged exact fixed-parent types.

## 6.2 EffectManifest/3 / EffectBytes/3

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

## 6.3 PreparedEditBinding/3

```text
D8EditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     source:text}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     value:AnnotationEditableValue/1,
     targetPolicy:"preserve"|"replace_current"}

D8PinnedEditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     proposedSource:PinRef/2}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2,
     targetPolicy:"preserve"|"replace_current"}

D8EditInput/3 = {
  kind:"d8_edit_input",version:3,
  invocationClass:"interactive_source_save"|"noninteractive",
  writeProtection:"strict"|"observed_only",
  intent:D8PinnedEditIntent/3,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request}
}

D8EditPrepareRequest/3 = {
  wireVersion:3,kind:"d8_edit_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  saveProfile:"ordinary"|"complete",
  guarantee:"replica_local"|"managed_atomic",
  writeProtection:"strict"|"observed_only",
  intent:D8EditIntent/3,
  budget:BudgetBinding/1
}

D8AnnotationDraftProjection/1 =
    {kind:"d8_annotation_draft",version:1,
     annotationRef:AnnotationRef,
     baseObservation:SourceObservation/1,
     baseRevisionToken:AnnotationRevisionToken/1,
     draftSerial:Counter,
     access:"editable"|"readonly",
     value:AnnotationEditableValue/1,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     body:{state:"valid",source:text}}
  | {kind:"d8_annotation_draft",version:1,
     annotationRef:AnnotationRef,
     baseObservation:SourceObservation/1,
     baseRevisionToken:AnnotationRevisionToken/1,
     draftSerial:Counter,
     access:"editable"|"readonly",
     value:AnnotationEditableValue/1,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     body:{state:"invalid",source:text,diagnostics:[CoreDiagnostic/1...]}}

PreparedEditBinding/3 = {
  kind:"d8_prepared_edit_binding",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  operationId:UUIDv4,principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  intent:D8EditIntent/3,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request},
  sourceInputs:InputDescriptor/3.sourceInputs,
  proposedInputs:[{entityRef:EntityRef,pin:PinRef/2}],
  registryInputs:[ValidatedCatalogContext...],
  dependencyProof:DependencyProof/3,
  observationProof:PreparedIntent/3.observationProof,
  budgetBinding:BudgetBinding/1,expiresAt:PreparedDeadline,
  request:d6_commit_request/2,preview:EffectManifest/3
}
```

For document intent, proposedInputs.pin selects exact UTF-8 source. For annotation intent, proposedInputs has exactly one item whose entityRef equals target.ref and whose PinRef/2 payloadKind is annotation_value; the pinned bytes are exactly D3-CJ/3(the complete Core-constructed D3-Annotation-Value/4), never AnnotationEditableValue/1 alone. target.ref is AnnotationRef and expectedAnnotationRevisionToken equals the current PortableAnnotationRecord/4 token at prepare. Annotation requires saveProfile=complete and writeProtection=strict. observed_only remains restricted to the retained qualified interactive Document path.

OwnerInputBinding/2 current ownerKind/intentKind is d8_edit/3 and canonicalDescriptorBytes is D3-CJ/3(D8EditInput/3). Its pinRefs are exactly the sorted/unique pins named by D8PinnedEditIntent/3 plus the retained origin evidence. D8EditInput/3.intent is mechanically derived from PreparedEditBinding/3.intent by replacing only source/value bytes with the corresponding proposed pin; no actor/time or trust flag exists in either request shape. Historical d8_edit_prepare wire1/2 and PreparedEditBinding/1/2 keep their original intent/value decoders and recovery.

## 6.4 D8 workspace presentation policy and render binding

```text
D8WorkspacePresentationPolicy/1 = {
  kind:"d8_workspace_presentation_policy",version:1,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  defaultPresentation:"separate"|"run_in"
}

D8WorkspacePresentationPolicyBinding/1 = {
  policy:D8WorkspacePresentationPolicy/1,
  pin:PinRef/2
}

D8PresentationPolicySetRequest/1 = {
  wireVersion:1,
  kind:"d8_presentation_policy_set",
  workspaceRef:WorkspaceRef,
  expectedRevision:Counter,
  defaultPresentation:"separate"|"run_in",
  budget:BudgetBinding/1
}

D8PresentationResult/1 = {
  heading:DocumentElementLocator,
  body:DocumentElementLocator|null,
  sourceOverride:"run_in"|"separate"|"default"|"conflict",
  effective:"run_in"|"separate",
  policyRevision:Counter,
  diagnostic:null|"role_conflict"
}

D8DocumentRenderBinding/1 = {
  sourceObservation:SourceObservation/1,
  documentFormat:DocumentFormatCurrentQualification/1,
  documentSnapshotPin:PinRef/2,
  presentationPolicy:D8WorkspacePresentationPolicyBinding/1,
  rendererProfileSha256:"sha256:<64 lowercase hex>"
}
```

Every active Workspace has one presentation-policy record, initialized at revision=1/defaultPresentation=separate. Mutation requires policy_admin, expectedRevision equality, checked increment, and one protected configuration write. The policy pin is artifact/recovery over UTF8("D8-Workspace-Presentation-Policy/1") || NUL || D3-CJ/3(complete policy). It is not author source and grants no source write. Current visual rendering uses D8DocumentRenderBinding/1; documentSnapshotPin selects exact D2-Document-Snapshot/3 bytes for sourceObservation.entityRef, and documentFormat belongs to that same owner/current read. Cache identity includes the complete binding.

Run-in resolution is closed: conflict => Separate+role_conflict; explicit separate => Separate; explicit run-in => RunIn only for the existing eligible first-paragraph rule; default uses policy only when neither role exists and retains the stricter implicit physical-adjacency rule. No eligible body => Separate. Source role commands mutate source roles only, never this policy.

## 6.5 ExportPlan/3 / PublicationReceipt/3

The following current export types close fields that were only prose in the fixed-parent ExportPlan/2 contract.

```text
D9ExportInputDomain/1 =
  "document"|"native_table"|"node_collection"|"query_rows"|"query_json"|"resource"

D9ExportTarget/1 =
    {kind:"asciidoc_source"}
  | {kind:"resource_exact"}
  | {kind:"html",profileId:text}
  | {kind:"pdf",profileId:text}
  | {kind:"docx",profileId:text}
  | {kind:"odt",profileId:text}
  | {kind:"csv_utf8"}
  | {kind:"tsv_utf8"}
  | {kind:"xlsx",profileId:text}
  | {kind:"ods",profileId:text}
  | {kind:"query_json"}

D9ExportTemplateBinding/1 = {
  inputIndex:Counter,
  profileId:text,
  profileVersion:text,
  pin:PinRef/2
}

D9ExportRouteStepBinding/1 = {
  step:Counter,
  providerId:text,
  providerVersion:text,
  inputProfileId:text,
  outputProfileId:text,
  optionsSha256:"sha256:<64 lowercase hex>",
  stepBudget:BudgetBinding/1,
  evidencePins:[PinRef/2...]
}

D9ExportRouteBinding/1 = {
  routeId:text,
  routeRevision:Token,
  profileId:text,
  profileVersion:text,
  steps:[D9ExportRouteStepBinding/1...]
}

D9ExportStyleBundleBinding/1 = {
  styleBundleId:text,
  version:Counter,
  pin:PinRef/2
}

D9ExportGenerationPolicyBinding/1 = {
  profileId:text,
  version:Counter,
  descriptorSha256:"sha256:<64 lowercase hex>",
  pin:PinRef/2
}

D9DocumentRenderBinding/1 = {
  ownerNodeRef:NodeRef,
  sourceObservation:SourceObservation/1,
  documentSnapshotPin:PinRef/2,
  semanticQualification:ManagedDocumentSemanticQualification/1,
  presentationPolicy:D8WorkspacePresentationPolicyBinding/1
}

D9ExportDestinationIntent/1 =
    {kind:"external_bundle",destinationHandle:Token,basename:text,createOnly:true}
  | {kind:"server_download",downloadToken:Token,basename:text}
  | {kind:"resource_handoff",ownerNodeRef:NodeRef,resourceName:text}

D9ExportLossChoice/1 = {
  lossKey:Counter,
  choice:"accept_loss"|"reject"
}

D9ExportConfirmation/1 = {
  planToken:Token,
  lossChoices:[D9ExportLossChoice/1...]
}

D9ExportStagedOutput/1 = {
  name:text,
  byteLength:Counter,
  sha256:"sha256:<64 lowercase hex>",
  pin:PinRef/2
}

D9PublishedOutput/1 = {
  name:text,
  byteLength:Counter,
  sha256:"sha256:<64 lowercase hex>"
}

ExportPlan/3 = {
  kind:"d9_export_plan",version:3,
  planToken:Token,workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,authorizationGeneration:Token,
  inputDomain:D9ExportInputDomain/1,
  inputCatalog:ExportInputCatalog/2,
  contentSelection:ExportContentSelection/1,
  projection:ExportProjection/1,
  documentRenderBinding:D9DocumentRenderBinding/1|null,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicyBinding/1,
  target:D9ExportTarget/1,
  initialLossReport:ExportLossReport/1,
  outputBudget:BudgetBinding/1,
  destination:D9ExportDestinationIntent/1,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  evidencePins:[PinRef/2...],
  stagedOutputs:[D9ExportStagedOutput/1...]
}

PublicationReceipt/3 = {
  kind:"d9_publication_receipt",version:3,
  publicationToken:Token,planToken:Token,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  outputs:[D9PublishedOutput/1...],
  lossReport:ExportLossReport/1,
  lossChoices:[D9ExportLossChoice/1...],
  target:D9ExportTarget/1,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  presentationPolicy:D8WorkspacePresentationPolicyBinding/1|null,
  destinationDisplay:text
}
```

ExportInputCatalog/2, ExportContentSelection/1, ExportProjection/1, and ExportLossReport/1 retain the complete fixed-parent D9 closed shapes and semantics. D9ExportRouteBinding/1 steps are continuous from zero and the route is the exact accepted finite acyclic route; evidencePins in each step are pinToken-sorted/unique. styleBundles sort uniquely by styleBundleId; plan evidencePins sort uniquely by pinToken; stagedOutputs and receipt outputs sort uniquely by controlled relative name. lossChoices sort by lossKey, are unique, and exactly cover requires_choice/blocking items under the retained severity matrix. A reject choice cancels; a blocking item can never be accepted.

asciidoc_source and resource_exact require routeBinding=null, templateBinding=null, documentRenderBinding=null, and projection contains no renderer-derived values. query_json likewise serializes the exact D7 result under its retained static profile and has no template binding. html/pdf/docx/odt require inputDomain=document, a nonnull documentRenderBinding, and a nonnull accepted route. docx/odt may require templateBinding according to the selected route profile. csv/tsv/xlsx/ods require the retained finite RenderSnapshot projection and a matching route/profile. No target arm is inferred from filename.

For document rendering, documentSnapshotPin selects exact D2-Document-Snapshot/3 canonical bytes whose owner equals ownerNodeRef and sourceObservation.entityRef. semanticQualification.sourceObservation is byte-equal to sourceObservation. presentationPolicy is the exact policy revision used to compute run-in presentation. A later presentation-policy change does not invalidate or mutate an already prepared immutable plan, but every fresh prepare uses the then-current binding. A source observation, document-format qualification, route/template/profile/style/Query/authorization change follows the retained D9 invalidation/reset rules.

D9ExportDestinationIntent/1 is protected intent, not permission. external_bundle is create-only and follows the retained same-volume staging/atomic-rename/durability/unknown state machine. server_download follows per-chunk current authorization. resource_handoff does not create a Resource; after export confirmation, the exact staged bytes enter a separate current D3 create_resource prepare/confirmation/receipt.

D9ExportConfirmation/1 never changes catalog, projection, route, target, destination, policies, report, budget, or staged bytes. PublicationReceipt/3 proves external publication only and must byte-match the protected original plan/confirmation for every repeated member. Historical ExportPlan/1-/2 and PublicationReceipt/1-/2 retain exact original decoders, permissions, confirmations, staged bytes, unknown duties, and recovery ordering.

# 7. Annotation closed values

```text
AnnotationRevisionToken/1 := nonempty opaque JSON string

PortableAnnotationRecord/4 = {
  kind:"portable_annotation",version:4,
  annotationRef:AnnotationRef,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4
}

AnnotationEditableValue/1 = {
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:Suggestion/3|null
}

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

AnnotationInlineBody/1 = {
  format:"asciidoc-inline",
  version:1,
  languageBaseline:"asciidoctor-ruby/2.0.26",
  source:text
}

AsciiDocInlineBody/1 := AnnotationInlineBody/1

AnnotationInlineProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  doctype:"inline",
  processorBackend:"html5-semantic/1",
  safeMode:"secure",
  maxSourceBytes:65536,
  maxRenderedBytes:262144,
  maxInlineSemanticNodes:4096,
  managedAdapters:"disabled",
  networkEffects:"denied",
  fileEffects:"denied",
  processEffects:"denied"
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

`AnnotationInlineBody/1` is the sole current name and preserves the four-member data shape originally named `AsciiDocInlineBody/1`. `AsciiDocInlineBody/1` is a **schema alias only**: its canonical bytes are exactly those of `AnnotationInlineBody/1`; it creates no second wire/version and asserts no historical deployment. The body is portable source data, not a processor profile. Evaluation always uses the single `AnnotationInlineProfile/1` above. The body's `languageBaseline="asciidoctor-ruby/2.0.26"` must correspond to the 2.0.26 version fixed by the profile's commit-qualified baseline. The complete source must form exactly one paragraph, allowing soft wraps and trailing whitespace; a second paragraph, heading, list, delimited block, table, or block macro is `invalid_annotation_body` rather than silently ignored.

Replies are same-owner, acyclic comments with suggestion=null and reviewState=not_applicable. Root reviewState is open|resolved. Pending suggestions use confirmed|needs_reconfirmation; accepted/rejected are terminal with confirmation=not_applicable.

PortableAnnotationRecord/4 is the current logical Portable Metadata record in the existing node-local annotations JSON authority. Its annotationRef owner matches the containing Node. D3-Annotation-Value/4 canonical bytes are exactly D3-CJ/3(value); current annotation_value payload bindings and PinRef/2 values hash/pin those bytes, not the outer PortableAnnotationRecord/4. annotationRevisionToken is opaque, never derived from the value digest, and is unique per committed Value/4 state for one AnnotationRef.

For current wire13 D3 mutation, the Value/4 base decoder keeps the inherited logical slots annotation_target ordinal0 and annotation_reply ordinal1. target is always a reference slot. replyTo is a reference slot when nonnull; an identity-preserving existing-Annotation reply change uses the inherited structural S form and cannot also produce a reply reference result. All other Value/4 members are nonreference bytes. Every changed current Value/4 gets one new final AnnotationRevisionToken/1, and all target/reply toSource addresses, annotation_reply_change evidence, SourceRevisionPlan/result pin, source change and receipt use that same token. A byte-equivalent current editable proposal is a no-op and keeps the current token/SourceVersion/attribution.

Current payloadBindings with payloadKind=annotation_value dispatch by actual request family: wire13 current mutation strict-decodes Value/4, while genuine historical wire9-12 plans keep their Value/3 decoder. D3-Symbolic-Result/9 framing is unchanged; its annotation base bytes and slot spans are computed from the decoder selected by that request family. This does not widen the historical decoder.

The current Portable Metadata path does not materialize D2 Annotation-v2 outer wire as author state. Historical D2 v2 annotation snapshots, Value/3, plain_text body, replace_plain_text suggestion and targetStatus resolved/stale retain their real historical decoder/recovery only. Current body bytes are AnnotationInlineBody/1 and use exactly the single AnnotationInlineProfile/1; no plain-text compatibility fallback or second parser exists.

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

SourceTransformSealSignedBody/1 = {
  format:"weftext.source-transform-seal",version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2
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

Plan/evidence edit canonical bytes must match exactly. Seal never recompiles/reorders/merges. A required sealed decision has exactly one outbox item; disabled has none.

Event3 validation is closed against exact beforeBytes/afterBytes. replace requires 0<=startByte<endByte<=before length, UTF-8 scalar boundaries, removedByteLength=endByte-startByte, and removedSha256=SHA-256(beforeBytes[startByte:endByte]); insert requires 0<=atByte<=before length, a UTF-8 scalar boundary, and non-zero replacementByteLength. Replacement intervals are non-overlapping maximal islands; same-point inserts are merged in transaction order; an insert strictly inside a replacement island is folded into that island or compilation is unavailable. Canonical boundary order is left replacement ending at p, insert@p, right replacement starting at p.

SourceTransformEvidence/2.beforeSourceSha256 is exactly "sha256:" + lowercase_hex(SHA-256(exact complete before-source bytes corresponding to evidence.before and ownerNodeRef)). Receiver validation obtains those historical bytes from the producing CP/history and retained version evidence, recomputes the whole-source digest, and requires equality before accepting the artifact. Event removedSha256/length, successful slice replay, equal SourceVersion fields, or current-file bytes cannot substitute. Missing exact-before bytes follows the existing proof/state-unavailable boundary; a proved digest contradiction is integrity failure. This cross-field adds no member and does not version SourceTransformEvidence/2.

generatedOutputSpan uses the SPEC §11.1 replay-cursor algorithm over exact before/after pins. It byte-compares every unchanged gap, assigns the event span [afterCursor,afterCursor+replacementByteLength), validates that exact after slice against replacement length/hash, advances replace beforeCursor to endByte while insert leaves it at q, and requires the final tails and afterSourceSha256 to match. Content search is forbidden. Mapping defines delta(replace)=replacementByteLength-(endByte-startByte) with checked signed arithmetic.

SourceTransformSealSignedBody/1 is exactly SourceTransformSealArtifact/1 with only signature removed. signature is exactly 86 ASCII unpadded-base64url characters decoding to 64 Ed25519 bytes. The authenticated message is exactly ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1). Complete transported/stored artifact bytes are exactly D3-CJ/3(SourceTransformSealArtifact/1 including signature); alternate JSON serialization is rejected.

# 9. D6 dual-profile trust

```text
SealProfileId/1 =
  "d6_revision_token_seal/1"|"d6_source_transform_seal/1"

WorkspaceTrustPredecessor/2 =
    {kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}
  | {kind:"declaration",revision:Counter,
     sha256:"sha256:<64 lowercase hex>"}

WorkspaceTrustDeclaration/2 = {
  kind:"d6_workspace_trust_declaration",version:2,
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,
  decisionKey:DecisionKey/2,
  action:
      {kind:"authorize",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>"}
    | {kind:"rotate",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,oldTrustKeyId:"sha256:<64 lowercase hex>",
       newTrustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
       publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>",
       continuitySignature:"<86 ASCII unpadded base64url>"|"not_required",
       mode:"ordinary"|"loss_recovery"|"compromise"}
    | {kind:"revoke",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       mode:"administrative"|"loss"|"compromise"}
    | {kind:"resolve_conflict",
       conflictId:ConflictId,
       resolvedHeads:[ChangeId/1...],
       selected:WorkspaceAuthorizationBundleAddress/1,
       outcomes:[TrustConflictOutcome/2...],
       inheritedCompromises:[TrustConflictCarry/1|TrustConflictCarry/2...]},
  rootSignature:"<86 ASCII unpadded base64url>"
}

DomainSealKeyPoPBody/2 = {
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,decisionKey:DecisionKey/2,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  trustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>"
}

DomainSealKeyRotateContinuityBody/2 = {
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,decisionKey:DecisionKey/2,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  oldTrustKeyId:"sha256:<64 lowercase hex>",
  newTrustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>",
  possessionSignature:"<86 ASCII unpadded base64url>",mode:"ordinary"
}

WorkspaceTrustDeclarationSignedBody/2 :=
  WorkspaceTrustDeclaration/2 with only rootSignature removed

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
  algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>"
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
     algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
     possessionSignature:"<86 ASCII unpadded base64url>"}

DomainSealKeyAddPrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_add_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  budget:BudgetBinding/1
}

DomainSealKeyRotatePrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_rotate_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  expectedTrustKeyId:"sha256:<64 lowercase hex>",
  mode:"ordinary"|"loss_recovery"|"compromise",
  budget:BudgetBinding/1
}

DomainSealKeyRevokePrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_revoke_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  expectedTrustKeyId:"sha256:<64 lowercase hex>",
  mode:"administrative"|"loss"|"compromise",
  budget:BudgetBinding/1
}
```

The public conflict request remains fixed-parent d6_conflict_prepare wireVersion3 with ConflictResolution/2. For a current unseen policy_bundle_choice, the same selected/policy/freshAuthorizations JSON arm is strict-decoded with the following current inner and protected descriptor types. Source-merge and choose-source-head keep the fixed-parent ConflictResolutionInput/2 / ConflictResolutionDerivedPlan/1 path.

```text
FreshDomainAuthorizationSpec/2 = {
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1
}

PolicyBundleHeadEvidence/2 =
    {head:ChangeId/1,completionProofVersion:3,bundleVersion:1}
  | {head:ChangeId/1,completionProofVersion:4,bundleVersion:1|2}

TrustConflictCarryValidationHop/1 =
    {declarationVersion:1,declarationRevision:Counter,
     declarationDigest:"sha256:<64 lowercase hex>",
     activationChangeId:ChangeId/1,
     completionProofVersion:3,bundleVersion:1,
     changeRecordPin:PinRef/2,
     completionProofPin:PinRef/2,
     policyBundlePin:PinRef/2}
  | {declarationVersion:2,declarationRevision:Counter,
     declarationDigest:"sha256:<64 lowercase hex>",
     activationChangeId:ChangeId/1,
     completionProofVersion:4,bundleVersion:2,
     changeRecordPin:PinRef/2,
     completionProofPin:PinRef/2,
     policyBundlePin:PinRef/2}

TrustConflictCarryValidationEvidence/1 = {
  head:ChangeId/1,
  factId:"sha256:<64 lowercase hex>",
  origin:TrustConflictCarryValidationHop/1,
  carriers:[TrustConflictCarryValidationHop/1...]
}

ConflictResolutionPolicyDerivedPlan/2 = {
  kind:"policy_bundle",
  selected:WorkspaceAuthorizationBundleAddress/1,
  headEvidence:[PolicyBundleHeadEvidence/2...],
  selectedBundleVersion:1|2,
  selectedBundlePin:PinRef/2,
  carryEvidence:[TrustConflictCarryValidationEvidence/1...],
  effectiveCompromises:[
    TrustConflictCarry/1|TrustConflictCarry/2...
  ],
  inheritedCompromises:[
    TrustConflictCarry/1|TrustConflictCarry/2...
  ],
  outcomes:[TrustConflictOutcome/2...],
  resultBundleVersion:1|2,
  resultBundlePin:PinRef/2
}

ConflictResolutionInput/3 = {
  kind:"d6_conflict_resolution_input",version:3,
  conflictId:ConflictId,expectedKey:ConflictKey/1,
  resolution:{
    kind:"policy_bundle_choice",
    selected:WorkspaceAuthorizationBundleAddress/1,
    policy:Policy/3,
    freshAuthorizations:[FreshDomainAuthorizationSpec/2...]
  },
  branchEvidence:[ConflictResolutionBranchEvidence/1...],
  derivedPlan:ConflictResolutionPolicyDerivedPlan/2
}

ConflictResolutionPreview/2 = {
  kind:"d6_conflict_resolution_preview",version:2,
  conflictId:ConflictId,expectedKey:ConflictKey/1,
  resolution:{
    kind:"policy_bundle_choice",
    selected:WorkspaceAuthorizationBundleAddress/1,
    policy:Policy/3,
    freshAuthorizations:[FreshDomainAuthorizationSpec/2...]
  },
  branchEvidenceDigest:"sha256:<64 lowercase hex>",
  derivedPlan:ConflictResolutionPolicyDerivedPlan/2
}
```

FreshDomainAuthorizationSpec/2 has exactly the same two JSON members as historical /1 but profile is the full SealProfileId/1 union. The array is canonical D3-CJ/3 sorted/unique and contains no key material. Current ConflictResolutionInput/3 is a protected owner-descriptor successor only for policy_bundle_choice; it does not version the public request, add a submit path, or change conflict_resolve/policy_admin/disclosure/error order. A genuinely proven saved/planned old owner descriptor retains its recorded decoder, preview and pins.

PolicyBundleHeadEvidence/2 is complete, ChangeId-sorted/unique and byte-equal in head set to expectedKey.heads. version=3 means branchEvidence.completionProofPin strict-decodes ContentCompletionProof/3 and the exact policy after-image strict-decodes WorkspaceAuthorizationBundle/1. version=4 means completionProofPin strict-decodes ContentCompletionProof/4, branchEvidence.changeRecordPin strict-decodes the matching ChangeRecord/1 and exact Notice3/CP4 chain, and the policy after-image strict-decodes the declared Bundle1 or Bundle2 version. In both arms policyBundlePin is present, pins the exact canonical bundle bytes, and WorkspaceAuthorizationBundleAddress/1 authorizationRevision/trustRevision/byteLength/sha256 matches them. For policy_bundle_choice every branchEvidence.sourcePins array is empty. CP3+Bundle2, unknown versions, a missing CP4 ChangeRecord, tag/byte mismatch, or decoder fallback is rejected.

TrustConflictCarryValidationEvidence/1 is the complete retained validation path for one effective fact on one expectedKey head. The array is sorted uniquely by head ChangeId then ASCII factId and contains exactly one item for every (head,factId) used by the resolver's per-head effective compromise fold. origin is the direct compromise declaration named by the Carry; carriers are every resolve_conflict declaration actually traversed after that origin on that head, in ascending declaration revision. A version-1 hop strict-decodes its exact historical ChangeRecord/CP3/Bundle1 activation evidence; a version-2 hop strict-decodes ChangeRecord/1 + CP4 + Bundle2. declarationDigest and activationChangeId are byte-equal to the declaration and activation cut proved by those exact pins. The origin hop matches originDeclarationRevision/originDeclarationDigest/originActivationChangeId of the Carry, and its action/key mapping is rechecked. Every carrier hop must be a valid resolver that actually carries that fact. Missing a traversed carrier, replacing an origin cut with resolver time, or substituting equal current bytes fails.

For the current policy arm, OwnerInputBinding/2.protocolOwner is D6 and ownerKind is exactly d6_conflict_resolution/3; InputDescriptor/3.intentKind is byte-equal to that ownerKind. canonicalDescriptorBytes is exactly D3-CJ/3(ConflictResolutionInput/3). OwnerInputBinding/2.pinRefs is the canonical PinRef sort, duplicate-free union of exactly: every branchEvidence.changeRecordPin; every branchEvidence.completionProofPin; every present branchEvidence.policyBundlePin; selectedBundlePin; resultBundlePin; and every origin/carrier changeRecordPin, completionProofPin and policyBundlePin in carryEvidence. No hash-only declaration reference, current bundle, derived-index row, or unstated pin may replace one of these entries. The selected bundle pin may equal its branch policyBundlePin and is then represented once after set deduplication.

ConflictResolutionPreview/2 is the immutable current policy preview. branchEvidenceDigest is exactly "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(the complete ConflictResolutionInput/3.branchEvidence array))). The preview's conflictId, expectedKey and resolution are byte-equal to Input3 and its derivedPlan is byte-equal to the complete Plan2, including headEvidence, carryEvidence, mixed Carry1/2 values, Outcome2 values and resultBundleVersion/pin. PreparedIntent/3.previewBinding binds exactly this Preview2 and its exact canonical preview pin. For this control_only policy arm, PreparedIntent/3.pinDirectory is the canonical duplicate-free union of OwnerInputBinding.pinRefs, every DependencyProof/3 evidencePin, that exact preview pin, and every proposal/before/after/recovery PinRef actually named by the fixed installationPlan; sourceInputs is empty, so it contributes no SourceObservation evidence pins. The same pin cannot be rebound to a different protected record.

The two current source arms do not use these policy successors. source_merge and choose_source_head retain ConflictResolutionInput/2, ConflictResolutionDerivedPlan/1 and ConflictResolutionPreview/1, with OwnerInputBinding.ownerKind and InputDescriptor/3.intentKind both d6_conflict_resolution/2 and canonicalDescriptorBytes exactly D3-CJ/3(Input2). Their original complete pin union, source semantic evidence, saveProfile=complete, scope selection and Preview1 rules remain normative, while the outer unseen carrier is the current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family and planToken d6_plan/3.

For Carry2, originDeclarationDigest is exactly "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(complete original WorkspaceTrustDeclaration/2 including rootSignature))). Direct Declaration2 compromise mapping is revoke -> action.trustKeyId and rotate -> action.oldTrustKeyId. Define the exact Carry2 fact body as the nine fields workspaceRef, commitDomain, profile, compromisedTrustKeyId, originAction, originDecisionKey, originDeclarationRevision, originDeclarationDigest, originActivationChangeId in that closed object order under D3-CJ/3. Then:

```text
factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL ||
      D3-CJ/3(Carry2 fact body)
    )
  )
```

originActivationChangeId is rederived only from the original Declaration2 DecisionKey through committed CP4 + ChangeRecord/1 and the exact Bundle2 after-image that first appended it. Carry1 keeps its fixed-parent /1 fact domain and Declaration1/CP3 origin rules. A Declaration2 resolve_conflict may inherit Carry1 and Carry2; each member is recursively revalidated back to its direct original compromise declaration. effectiveCompromises and inheritedCompromises are ASCII factId sorted, unique, and contain byte-equal values for duplicate factIds; same factId with non-byte-equal canonical bytes is integrity_conflict. The effective union includes selected and losing branches, so branch choice never discards a compromise fact.

Bundle1 normalizes source-transform state to none. affectedDomainProfiles is the union of all differing normalized domain/profile states plus all pairs named by the effective compromise union. TrustConflictOutcome/2 is canonical sorted/unique by D3-CJ/3(commitDomain,profile) and complete for every affected pair when a trust-resolution declaration is needed. keep_current is legal only for a selected current key that remains safe under the complete effective union. A requested affected pair may authorize_fresh; its new key uses DomainSealKeyHandle/2 and PoP/2. A policy-only result with no trust resolution and no fresh authorization preserves selectedBundleVersion/trustRevision. Otherwise exactly one WorkspaceTrustDeclaration/2 resolve_conflict is appended and resultBundleVersion=2; a selected Bundle1 is preserved byte-exact as its Declaration1 prefix before the new Declaration2.

ConflictResolutionPolicyDerivedPlan/2 freezes the exact per-head proof/bundle dispatch, selected bundle pin, complete per-head carry validation evidence, complete carry union, inherited subset, Outcome2 values, and exact result bundle pin/version. resultBundlePin must strict-decode the derived result and reproduce its canonical bytes. The original one planning CAS freezes InputDescriptor/3, OwnerInputBinding/2, Preview2, pinDirectory, staged fresh-handle associations and installationPlan together. Final submit remains only d6_commit_request/2. The one final P publishes the current policy component through Notice3/CP4/ChangeRecord1 and the same conflict-record transition; staged authorize_fresh handles become usable only in that same commit. Planned recovery restores those exact Input3/Plan2/Preview2 bytes and pins and never rebuilds them from current history. Receiver validation recomputes all head dispatch, Carry1/Carry2 facts, every retained carry-validation hop, mixed recursive union, PoP/2 and root signatures, exact result bundle, preview/descriptor cross-fields, and same-decision conflict-record transition.

Every publicKey decodes to exactly 32 Ed25519 bytes and hashes to the corresponding trustKeyId. Every signature lexical value decodes to exactly 64 Ed25519 bytes. For authorize/rotate, possessionSignature signs exactly ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2). Rotate maps newTrustKeyId to the PoP body's trustKeyId. authorize_fresh constructs the same body from the enclosing Declaration2 workspaceRef/revision/predecessor/decisionKey plus that exact outcome's commitDomain/profile/key tuple. Ordinary rotate continuitySignature signs exactly ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2); loss_recovery/compromise require the literal "not_required". rootSignature signs exactly ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2).

Revision-1 root predecessor fingerprint is the complete WorkspaceTrustRootFingerprint/1 and is byte-equal to the retained root/anchor fingerprint; rootKeyId never substitutes. Declaration/1 remains revision-token-only. Bundle2 permits a Declaration1 historical prefix; after the first Declaration2 every successor is Declaration2. Declaration1 activation remains CP3. Declaration2 activation is rederived through same-DecisionKey ChangeRecord1 -> exact CP4 -> policy after-image -> Bundle2.

Current unseen ordinary trust management uses only the wireVersion3 prepare successors above. They carry no caller key material and preserve the fixed-parent policy_admin/root-handle/current-state gates plus the original planning/install/single-P path, now with CP4. Saved/planned wireVersion2 records keep their original decoder and recovery.

```text
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
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
```

All UUID members use the canonical lowercase D3 UUID decoder. Genesis has exactly two declarations: revision 1 revision-token authorize and revision 2 source-transform authorize, same DecisionKey/activation ChangeId, with rev2 predecessor hashing exact rev1 canonical bytes. Series configurations are unique and sorted by canonical SeriesScope; period bindings are unique and sorted by full NodeRef with exactly one per valid prepared period. The helper members retain the fixed-parent Plan3 field semantics; Profile4 changes only the dual-profile genesis family.

Plan4 is only for unseen fresh create/fork and preserves the original D3 proposal/custody/CAS/P boundary. Ordinary copy is not Plan4. Saved/planned/unknown recovery keeps the actual recorded decoder and bytes; restore/continue/failover do not synthesize Genesis2. No deployment or migration from Plan3 is asserted.

Legacy Handle1 may continue new revision-token signing only while its exact Declaration1-authorized key remains current, safe, and usable; it never signs transforms. Continuation evaluates revision and transform profiles independently as current(K)|none|conflicted_or_unproved; any unproved state prevents partial new-domain activation. The declaration order is optional old revision revoke, optional old transform revoke, new revision authorize, new transform authorize, all under one DecisionKey/CP4 with no observable intermediate prefix.

# 10. D10 mixed-version outer schemas

The following are current outer-holder schemas. Every versioned carrier strict-decodes the exact tagged inner type and retains its original canonical bytes/pins. Unknown tags fail closed. A carrier is only a decoder discriminator: it grants no authority, creates no decision/CAS, and never migrates the inner record.

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

For a current unseen external-consent preparation, confirmationRequirement is the same closed ExternalConfirmationRequirement/1 used by the fixed-parent attended-confirmation protocol and is frozen inside ControlPrepareBinding/3 before the original commit request is delivered. The mutable confirmation fact remains exclusively in ExternalConfirmationRecord/1 and is not inserted into Binding3 or ControlDependencies/3. Current confirmation eligibility therefore consumes Binding3 + Dependencies3 while preserving the original trusted-event, principal, time-window, immutable-intent/preview, disclosure, approval_unavailable/preflight, saved-result replay, and result-redaction rules. A proven historical ControlPrepareBinding/2 retains its exact requirement, Dependencies2, canonical intent bytes, confirmation association, and recovery decoder.

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

StopCapacity/1 = {
  issued:Counter,reserved:Counter
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

Image2 is closed to automation/run. Every other record kind keeps exact Image1. Pin1 keeps its historical D10-Control-Record/1 prefix and decoder. Pin2 contains exactly UTF8 D10-Control-Record/2, NUL, and D3-CJ/3(Image2); its PinRef/2 is artifact with the original recovery or approval_money retention and exact prefixed byteLength/SHA-256. A schema-tag/payload-domain mismatch fails. Pin2 never repins Image1.

For D10VersionedControlRecordPin/1 arrays let I=carrier.value.image. Sort by D3-CJ/3(I.binding.ref), binding.revision numeric, usageRevision none before some, numeric usageRevision when present, then D3-CJ/3(I). The logical record-cut identity is I.binding.ref + I.binding.revision + I.usageRevision. A second byte-equal image at that identity is a duplicate and is rejected; different bytes are integrity_conflict. Version does not split the identity.

Range kind rank is records=0, cost_lineage=1, occurrences=2. The cross-version logical identity is respectively scope plus the complete sorted kinds, CostLayerKey, or Automation Ref. Sort mixed ranges by rank then canonical identity bytes and allow one value per identity. An occurrences range sorts its records by AutomationOccurrenceKey; one key cannot occur in both V1 and V2.

D10ControlEffectPlan/2.changes sorts uniquely by the complete canonical after.value.binding.ref. When before is present, before.binding.ref equals after.binding.ref and recordPins contains exactly the versioned pin for the actual stored before schema. Image1 before plus Pin2 is invalid. Image1 Automation+Subscription1 to Image2+Subscription2 is a valid current configure transition. A normal configure may continue the old subscription at the same generation when the scheduling owner continue rule succeeds; mixed support never forces replacement or background migration.

ControlDependencies/3 arrays keep their fixed owner order: configBindings and usageBindings by full Ref, authorizationGenerations by token, stopRefs by full Ref, recordPins/ranges by the mixed orders above. Each config binding has its exact matching image/pin, each usage binding has the same usageRevision image, and each stopRef has exact stop Image1/Pin1. All current bindings, usage revisions, range fences, pins, and Workspace evidence are captured from one actual Authority Store barrier. Combining barrier-A V1 evidence with barrier-B V2 evidence is not a complete dependency snapshot. Missing required historical bytes/decoder/pin is state_unavailable after disclosure; contradictory same-cut evidence is integrity_conflict.

D10ExecutionClaims/2 canonical identities are: recordPins as above; prepareBindings by inner StableControlKey across Binding1/2/3; leaseRuns by full Run ControlRef; authorSteps by run Ref plus stepId across versions; subscriptions by Automation Ref plus generation across versions; occurrenceRecords by AutomationOccurrenceKey; ranges as above; continuityPins by pinToken. Each identity is unique across versions. In particular Binding1(K) and Binding3(K) cannot coexist even if canonicalIntentBytes and originalCommitRequest are byte-equal. Binding1 retains its exact six-member historical shape; no kind, version, confirmationRequirement, /2-/3 dependency field, LWW, or repinning is introduced.

D10MoneyResponsibility/2 orders reservations uniquely by complete Binding<reservation>/1 canonical bytes, layers uniquely by CostLayerKey canonical bytes, recordPins/ranges by the mixed rules, and evidencePins uniquely by pinToken. D10ExecutionInventory/2 orders approvalUses uniquely by DecisionKey canonical bytes across versions, externalUnknowns uniquely by binding.ref canonical bytes, and stopState uniquely by binding.ref canonical bytes. Every pending/unknown/dedup/stop responsibility and every still-referenced completed external attempt is retained. The inventory is captured only after admission, planning, send, and schedule writers stop at one real store barrier.

The exact Inventory2 artifact payload is UTF8 D6-Execution-Inventory/2, NUL, D3-CJ/3(D10ExecutionInventory/2). The PinRef/2 is artifact/recovery and its byteLength/SHA-256 covers the complete prefixed bytes. Inventory1 keeps D6-Execution-Inventory/1 and is never repinned as /2.

Record3.workspaceRef equals Inventory2.workspaceRef. Record3.approvalUses, claims, moneyLineage, externalUnknowns, and stopState are each byte-equal to the corresponding Inventory2 member. Proof2.inventoryPin selects that exact Inventory2 and Inventory2.storeIncarnation equals Proof2.storeIncarnation. The protected birth/barrier/fence token mapping proves the same actual store and capture barrier; there is no separate caller-supplied store-incarnation proof object.

Inventory2.stopCapacity is the exact fixed-parent StopCapacity/1 at the same authoritative safety-store barrier. StopCapacity/1 remains only issued/reserved and is not duplicated in Record3. A Workspace-only handoff cannot copy the shared safety counters to a second active store. A complete store handoff must fence every affected writer and preserve all target/latch reservations and safety capacity; inability to prove that boundary makes takeover unavailable.

Record2/Proof1/Inventory1 remain historical and keep their exact decoder/domain. Record3 is emitted only for an actual responsibility mutation, checkpoint, or custody handoff; it preserves executionDomainId and never background-migrates an unchanged holder.

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

ScheduleContinuityInvalidation/2 = {
  kind:"d6_schedule_continuity_invalidation",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  status:"binding_changed"|"gap",
  evidencePins:[PinRef/2...]
}
```

The current continuity artifact encodings are exact and version-separated:

```text
Witness2ArtifactBytes =
  UTF8("D6-Schedule-Continuity/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityWitness/2)

Step2ArtifactBytes =
  UTF8("D6-Schedule-Step/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityStep/2)

Invalidation2ArtifactBytes =
  UTF8("D6-Schedule-Invalidation/2") || NUL ||
  D3-CJ/3(complete ScheduleContinuityInvalidation/2)
```

Each corresponding PinRef/2 has payloadKind=artifact and retentionClass=recovery. Its byteLength and SHA-256 cover the complete domain prefix, the one NUL byte, and the complete canonical object bytes above. A digest never authenticates continuity by itself: the protected producer/subscription provenance, producerEpoch, exact generation, retained transition chain, and original authorization/disclosure gates remain required.

Decoder dispatch is closed by the authenticated artifact domain before inner decoding. D6-Schedule-Continuity/1 accepts only historical ScheduleContinuityWitness/1; D6-Schedule-Step/1 accepts only historical ScheduleContinuityStep/1; D6-Schedule-Invalidation/1 accepts only historical ScheduleContinuityInvalidation/1. D6-Schedule-Continuity/2 accepts only ScheduleContinuityWitness/2; D6-Schedule-Step/2 accepts only ScheduleContinuityStep/2; D6-Schedule-Invalidation/2 accepts only ScheduleContinuityInvalidation/2. Unknown domains, a known domain paired with the wrong kind/version, alternate prefixes, missing NUL, or non-canonical object bytes fail closed through the original disclosure/error boundary. There is no fallback decoder, widening of a /1 decoder, repinning, or re-encoding of historical bytes.

```text
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
         {protocolOwner:"D3",request:D3IdentityOperationRequest/13}
       | {protocolOwner:"D6",request:d6_commit_request/2},
     preparedFormat:
       "d7_prepared_action_binding4"|"d8_prepared_edit_binding3",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
```

For a fresh current core_field_member step, D10AuthorPreparationLink/2.preparedBindingToken must select the exact PAB4 whose original request equals link.request. Core saves Link2, that complete PAB4, and the required preview/effect/recovery pins atomically before returning the prepared step or permitting submission. ApprovalUse/2.preparedBindingToken selects that same PAB4. Its preview digest is
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3));
each EffectBytes/3 slot is projected only as {encoding,byteLength,payloadDigest}, never discovered by recursively guessing member names. Fresh current automatic author qualification uses DependencyProof/3, including document_format whenever the managed Document is semantically parsed, and constructs ApprovalUse/2 only after complete EffectManifest/3 / EffectBytes/3 / MutationFootprint validation. Historical Link1/PAB3/ApprovalUse1 saved or planned associations keep their original decoder, bytes, pins, request and OperationId.

For a fresh ScheduleSubscription/2 registration, the current D6 Storage producer creates ScheduleContinuityWitness/2 only in the same configuration transaction that passes the selected source/Field/Registry/current scheduling gates, reserves finite retention, and attaches the actual continuously maintained Core source/control transition producer. initial and checkpoint are the subscription's exact ScheduleRecurrenceEvidence/2, revision=1, consumedTransition=0, and producerEpoch is fresh. The retained witness pin is exactly Witness2ArtifactBytes above. Positive current transitions use ScheduleContinuityStep/2 with DependencyProof/3 and the actual ChangeRecord/1, InstallationNotice/3, and ContentCompletionProof/4 pins for new current portable transitions; each retained current step pin is exactly Step2ArtifactBytes above. Historical transitions inside retained history keep their original exact /1 artifact domains and decoders. P-only relevant control/rule transitions remain captured from their actual protected before/after state.

Fold, compaction, receiver admission, D10 continuityPins consumption, and recovery all dispatch each protected schedule-continuity artifact by its exact domain before decoding the inner object. A current Witness2/Step2 is never accepted under a /1 domain, and a historical Witness1/Step1 is never accepted under a /2 domain. continuityPins are token-sorted/unique exact PinRef/2 values and may retain a version-mixed original typed chain when real history crosses the explicit same-generation bridge; each element keeps its own exact bytes/domain/decoder. The alternative full retained-chain path likewise keeps each original typed artifact and source/control evidence rather than normalizing the chain to one version. Missing or unknown typed evidence produces the original gap/unavailable behavior after the ordinary authorization/disclosure checks; a proved domain/object mismatch is never repaired from equal digest/current state.

When no valid current after evidence can exist, the current producer emits ScheduleContinuityInvalidation/2 instead of fabricating ScheduleRecurrenceEvidence/2. binding_changed requires complete trusted evidence of a selected-business discontinuity; unavailable/unknown after, missing history, unknown decoder, observer/producer gap, or inability to retain a required transition is gap. Invalidation compares the same current witness/registration, atomically advances the next checked transition/revision, keeps the last valid checkpoint, and is permanently non-resetting for that generation. Its artifact pin is exactly Invalidation2ArtifactBytes above. The fixed-parent inbox/capacity/final-counter reservation, authorization, compaction and unrelated-source-availability rules remain unchanged.

A current schedule proof that semantically parses a managed Document must include both source and document_format dependencies. If source/profile bytes are unchanged but format-proof continuity has a gap, the result is gap; a real binding transition is binding_changed even when the final recurrence/range value happens to compare equal. An existing Subscription1 remains a historical retention owner with Witness1/Step1/Invalidation1 and their exact /1 artifact domains. It may become a same-generation Subscription2 only through explicit continue plus complete retained history proving no intervening format/rule/business discontinuity and establishing the current Evidence2/Proof3 cut; otherwise replace is required. The bridge preserves every old pin/producer association and starts using /2 domains only for newly produced Witness2/Step2/Invalidation2 artifacts. It never repins or re-encodes a version-1 witness/step/invalidation as version 2 and never resets an invalid generation.

# 11. Historical dispatch and one authority set

Saved work replays under its original owner/version. Planned work restores the original descriptor/proof/prepared/pins/Notice/install/version basis. Unknown work retains original Approval/Money/claim/external/stop liabilities. Only unseen work uses current successors. Old bytes/pins are never re-encoded merely because they are carried by a mixed wrapper.

All successors continue through the original single DecisionKey, planning CAS, install, P seal, receipt, and outbox. document_format is not a second Document authority; SourceTransform is not source authority; ChangeRecord is not a second portable truth; a D10 version wrapper grants no authority; Derived Index never reconstructs current proof.

# 12. Provider profiles

Provider closed shapes are in SPEC §4. They qualify ecosystem renderers and never alter core-language validity. Mermaid's fixed source is CLI 12.0.0 commit db1ceebbe529d7975474eb0d0e9c23e9dc57cd37 with fixed package.json and package-lock Git blobs. Node/browser/fonts/config still require exact runtime version/digest registration. Without a registered runtime the provider is unavailable; floating semver, host browser/font defaults, or network installation cannot fill the gap.

# 13. Acceptance reference

Every schema and cross-field rule above has explicit positive/negative design obligations in [ACCEPTANCE.md](ACCEPTANCE.md): 438 core + 138 coordination = 576, all unexecuted. Those row bodies are normative obligations; the count is not a substitute for them.
