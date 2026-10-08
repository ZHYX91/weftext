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

This family is product data, not OracleSemanticWitness/7 or CoreSemanticProjection/1. The current /3 family is still an unimplemented candidate, so this section completes that same /3 contract rather than inventing a deployed /4 migration.

```text
D2ProductInteger := CanonicalSignedDecimal
// effective section levels require a nonnegative value; there is no upper language clamp.

D2NativeSemanticValue/1 =
    null | Boolean | text
  | {integer:CanonicalSignedDecimal}
  | {textArray:[text...]}

D2NativeAttributeEntry/1 = {
  name:text,
  value:D2NativeSemanticValue/1,
  provenance:"authored_named"|"authored_positional"|"fixed_derived",
  sourceOrigins:D2SourceOriginSet/2
}

D2NativeAttributeSet/1 = {
  entries:[D2NativeAttributeEntry/1...]
}

D2BlockCommonSemantics/1 = {
  style:text|null,
  caption:text|null,
  numeral:text|null,
  subs:[text...],
  positional:[D2NativeSemanticValue/1...],
  origins:{
    style:D2SourceOriginSet/2,
    caption:D2SourceOriginSet/2,
    numeral:D2SourceOriginSet/2,
    subs:[D2SourceOriginSet/2...],
    positional:[D2SourceOriginSet/2...]
  }
}

D2SourceOwner/1 =
    {kind:"root_document",logicalPath:ProcessorPath/1,
     ownerNodeRef:NodeRef,sourceObservation:SourceObservation/1}
  | {kind:"managed_include",logicalPath:ProcessorPath/1,
     ownerNodeRef:NodeRef,sourceObservation:SourceObservation/1}
  | {kind:"artifact_include",logicalPath:ProcessorPath/1,pin:PinRef/2}
  | {kind:"network_snapshot",logicalPath:ProcessorPath/1,
     requestDigest:"sha256:<64 lowercase hex>",pin:PinRef/2}

D2SourceRange/2 = {
  source:D2SourceOwner/1,
  startByte:Counter,endByte:Counter,
  startLine:Counter,startColumn:Counter,
  endLine:Counter,endColumn:Counter
}

D2WritableSource/1 =
    {kind:"unique",range:D2SourceRange/2}
  | {kind:"none",
     reason:"generated"|"multi_origin"|"ambiguous"|"non_author_source"}

D2SourceTransformKind/1 =
  "attribute"|"specialchars"|"quotes"|"macros"|"replacements"|
  "post_replacements"|"include"|"parser_derived"|"model_derived"|
  "weftext_adapter"

D2SourceOrigin/2 =
    {id:Counter,kind:"authored",range:D2SourceRange/2,
     inputs:[],writableSource:D2WritableSource/1}
  | {id:Counter,kind:"reference",range:D2SourceRange/2,
     inputs:[Counter...],writableSource:D2WritableSource/1}
  | {id:Counter,kind:"substitution",operation:D2SourceTransformKind/1,
     range:D2SourceRange/2|null,inputs:[Counter...],
     writableSource:D2WritableSource/1}
  | {id:Counter,kind:"generated",operation:D2SourceTransformKind/1,
     range:D2SourceRange/2|null,inputs:[Counter...],
     writableSource:D2WritableSource/1}
  | {id:Counter,kind:"synthetic",inputs:[Counter...],
     writableSource:{kind:"none",reason:"generated"|"multi_origin"|"ambiguous"}}
  | {id:Counter,kind:"multi_origin",inputs:[Counter...],
     writableSource:{kind:"none",reason:"multi_origin"|"ambiguous"}}

D2SourceOriginGraph/2 = {
  origins:[D2SourceOrigin/2...]
}

D2SourceOriginSet/2 = {
  originIds:[Counter...],
  writableSource:D2WritableSource/1
}

D2ProductReadBarrier/1 = {
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  frontier:Frontier/2
}

D2ProductEvaluationBinding/1 = {
  processorEnvironment:AsciiDocProcessorEnvironment/3,
  processorEnvironmentSha256:"sha256:<64 lowercase hex>",
  rootSource:SourceUnitBinding/1,
  includeSources:[SourceUnitBinding/1...],
  readBarrier:D2ProductReadBarrier/1,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3
}

D2ProductDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error",
  semanticCode:text,
  message:text,
  sourceRange:D2SourceRange/2|null
}

D2TextProjection/2 = {
  text:text,
  sourceOrigins:D2SourceOriginSet/2
}

D2DocumentBackendState/1 = {
  backend:text,
  basebackend:text,
  filetype:text,
  outfilesuffix:text,
  origins:{
    backend:D2SourceOriginSet/2,
    basebackend:D2SourceOriginSet/2,
    filetype:D2SourceOriginSet/2,
    outfilesuffix:D2SourceOriginSet/2
  }
}

D2DocumentAuthor/1 = {
  name:text,
  firstname:text|null,
  middlename:text|null,
  lastname:text|null,
  initials:text|null,
  email:text|null,
  origins:{
    name:D2SourceOriginSet/2,
    firstname:D2SourceOriginSet/2,
    middlename:D2SourceOriginSet/2,
    lastname:D2SourceOriginSet/2,
    initials:D2SourceOriginSet/2,
    email:D2SourceOriginSet/2
  }
}

D2DocumentRevision/1 = {
  number:text|null,
  date:text|null,
  remark:text|null,
  origins:{
    number:D2SourceOriginSet/2,
    date:D2SourceOriginSet/2,
    remark:D2SourceOriginSet/2
  }
}

D2EffectiveDocumentAttribute/1 = {
  name:text,
  value:D2NativeSemanticValue/1,
  sourceOrigins:D2SourceOriginSet/2
}

D2DocumentMetadata/3 = {
  backend:D2DocumentBackendState/1,
  doctype:"article"|"book"|"manpage"|"inline",
  safeMode:"unsafe"|"safe"|"server"|"secure",
  doctypeOrigins:D2SourceOriginSet/2,
  safeModeOrigins:D2SourceOriginSet/2,
  title:D2TextProjection/2|null,
  mainTitle:D2TextProjection/2|null,
  subtitle:D2TextProjection/2|null,
  authors:[D2DocumentAuthor/1...],
  revision:D2DocumentRevision/1|null,
  effectiveAttributes:[D2EffectiveDocumentAttribute/1...],
  headerAttributes:[{name:text,value:text|null,
                    sourceRange:D2SourceRange/2,
                    sourceOrigins:D2SourceOriginSet/2}...]
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

D2InlineMediaSemantics/1 =
    {kind:"image",
     width:text|null,height:text|null,format:text|null,
     scaledwidth:text|null,scale:text|null,link:text|null,window:text|null,
     float:text|null,align:text|null,fallback:text|null,imagesdir:text|null,
     origins:{width:D2SourceOriginSet/2,height:D2SourceOriginSet/2,
              format:D2SourceOriginSet/2,scaledwidth:D2SourceOriginSet/2,
              scale:D2SourceOriginSet/2,link:D2SourceOriginSet/2,
              window:D2SourceOriginSet/2,float:D2SourceOriginSet/2,
              align:D2SourceOriginSet/2,fallback:D2SourceOriginSet/2,
              imagesdir:D2SourceOriginSet/2}}
  | {kind:"icon",
     size:text|null,flip:text|null,rotate:text|null,title:text|null,
     width:text|null,height:text|null,
     origins:{size:D2SourceOriginSet/2,flip:D2SourceOriginSet/2,
              rotate:D2SourceOriginSet/2,title:D2SourceOriginSet/2,
              width:D2SourceOriginSet/2,height:D2SourceOriginSet/2}}

D2ProductInline/3 =
    {kind:"text",text:text,sourceOrigins:D2SourceOriginSet/2}
  | {kind:"quoted",style:"strong"|"emphasis"|"monospaced"|"mark"|
                         "superscript"|"subscript"|"double"|"single"|"unquoted",
     id:text|null,roles:[text...],children:[D2ProductInline/3...],
     slotOrigins:{style:D2SourceOriginSet/2,id:D2SourceOriginSet/2,
                  roles:[D2SourceOriginSet/2...]},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"link",nativeTarget:text,label:[D2ProductInline/3...],
     slotOrigins:{nativeTarget:D2SourceOriginSet/2},
     adapter:D2IdentityAdapter/1|null,nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"xref",nativeTarget:text,refid:text|null,path:text|null,
     label:[D2ProductInline/3...],adapter:D2IdentityAdapter/1|null,
     slotOrigins:{nativeTarget:D2SourceOriginSet/2,
                  refid:D2SourceOriginSet/2,path:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"anchor",referenceKind:"ref"|"bibref",
     id:text,reftext:text|null,
     slotOrigins:{referenceKind:D2SourceOriginSet/2,id:D2SourceOriginSet/2,
                  reftext:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"image",nativeTarget:text,alt:text|null,
     semantics:D2InlineMediaSemantics/1,
     slotOrigins:{nativeTarget:D2SourceOriginSet/2,alt:D2SourceOriginSet/2},
     adapter:D2IdentityAdapter/1|null,
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"footnote",id:text|null,index:CanonicalSignedDecimal|null,
     referenceKind:null|"ref"|"xref",target:text|null,
     children:[D2ProductInline/3...],
     slotOrigins:{id:D2SourceOriginSet/2,index:D2SourceOriginSet/2,
                  referenceKind:D2SourceOriginSet/2,target:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"indexterm",visibility:"visible"|"concealed",
     text:text|null,terms:[text...],see:text|null,seeAlso:[text...],
     slotOrigins:{visibility:D2SourceOriginSet/2,text:D2SourceOriginSet/2,
                  terms:[D2SourceOriginSet/2...],see:D2SourceOriginSet/2,
                  seeAlso:[D2SourceOriginSet/2...]},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"stem",notation:"tex"|"asciimath",source:text,
     slotOrigins:{notation:D2SourceOriginSet/2,source:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"kbd",keys:[text...],
     slotOrigins:{keys:[D2SourceOriginSet/2...]},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"menu",path:[text...],
     slotOrigins:{path:[D2SourceOriginSet/2...]},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"button",label:text,
     slotOrigins:{label:D2SourceOriginSet/2},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"callout",label:text,number:CanonicalSignedDecimal|null,
     id:text|null,
     guard:null|text|{before:text,after:text},
     slotOrigins:{label:D2SourceOriginSet/2,number:D2SourceOriginSet/2,
                  id:D2SourceOriginSet/2,guard:D2SourceOriginSet/2},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"line_break",sourceOrigins:D2SourceOriginSet/2}
  | {kind:"passthrough",text:text,sourceOrigins:D2SourceOriginSet/2}

D2Heading/3 = {
  kind:"heading",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:D2SourceOriginSet/2,
  authoredLevel:integer(1..9),
  effectiveLevel:D2ProductInteger,
  inlines:[D2ProductInline/3...],
  anchor:text|null,
  roles:[text...],
  options:[text...],
  slotOrigins:{authoredLevel:D2SourceOriginSet/2,
               effectiveLevel:D2SourceOriginSet/2,
               anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  nativeAttributes:D2NativeAttributeSet/1
}

D2ParagraphBlock/3 = {
  kind:"paragraph",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  inlines:[D2ProductInline/3...],
  nativeAttributes:D2NativeAttributeSet/1
}

D2SectionSemantics/1 = {
  sectname:text,
  special:Boolean,
  numbered:false|true|"chapter",
  numeral:text|null,
  caption:text|null,
  origins:{sectname:D2SourceOriginSet/2,special:D2SourceOriginSet/2,
           numbered:D2SourceOriginSet/2,numeral:D2SourceOriginSet/2,
           caption:D2SourceOriginSet/2}
}

D2SectionBlock/3 = {
  kind:"section",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  heading:D2Heading/3,
  common:D2BlockCommonSemantics/1,
  semantics:D2SectionSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  children:[D2ProductBlock/3...]
}

D2ListItemSemantics/1 = {
  coids:[text...],
  coidOrigins:[D2SourceOriginSet/2...],
  nativeAttributes:D2NativeAttributeSet/1
}

D2ListItem/3 = {
  kind:"list_item",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  marker:text|null,checked:Boolean|null,
  slotOrigins:{marker:D2SourceOriginSet/2,checked:D2SourceOriginSet/2},
  terms:[[D2ProductInline/3...]...],
  inlines:[D2ProductInline/3...],
  semantics:D2ListItemSemantics/1,
  children:[D2ProductBlock/3...]
}

D2ListSemantics/1 =
    {kind:"unordered",style:text|null,checklist:Boolean,interactive:Boolean,
     origins:{style:D2SourceOriginSet/2,checklist:D2SourceOriginSet/2,
              interactive:D2SourceOriginSet/2}}
  | {kind:"ordered",style:text|null,start:CanonicalSignedDecimal|null,
     reversed:Boolean,
     origins:{style:D2SourceOriginSet/2,start:D2SourceOriginSet/2,
              reversed:D2SourceOriginSet/2}}
  | {kind:"description",style:text|null,labelwidth:text|null,
     itemwidth:text|null,
     origins:{style:D2SourceOriginSet/2,labelwidth:D2SourceOriginSet/2,
              itemwidth:D2SourceOriginSet/2}}
  | {kind:"callout",style:text|null,
     origins:{style:D2SourceOriginSet/2}}

D2ListBlock/3 = {
  kind:"list",listKind:"unordered"|"ordered"|"description"|"callout",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2ListSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  items:[D2ListItem/3...]
}

D2TableColumn/3 = {
  ordinal:Counter,width:text|null,halign:text|null,valign:text|null,
  style:text|null,sourceOrigins:D2SourceOriginSet/2,
  slotOrigins:{width:D2SourceOriginSet/2,halign:D2SourceOriginSet/2,
               valign:D2SourceOriginSet/2,style:D2SourceOriginSet/2},
  nativeAttributes:D2NativeAttributeSet/1
}

D2TableCell/3 = {
  kind:"table_cell",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  columnStart:Counter,colspan:Counter,rowspan:Counter,style:text|null,
  halign:text|null,valign:text|null,
  slotOrigins:{columnStart:D2SourceOriginSet/2,colspan:D2SourceOriginSet/2,
               rowspan:D2SourceOriginSet/2,style:D2SourceOriginSet/2,
               halign:D2SourceOriginSet/2,valign:D2SourceOriginSet/2},
  nativeAttributes:D2NativeAttributeSet/1,
  content:
      {kind:"inline",inlines:[D2ProductInline/3...]}
    | {kind:"blocks",children:[D2ProductBlock/3...]}
}

D2TableRow/3 = {
  kind:"table_row",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  section:"head"|"body"|"foot",
  sectionOrigins:D2SourceOriginSet/2,
  cells:[D2TableCell/3...]
}

D2TableSemantics/1 = {
  cols:text|null,format:text|null,separator:text|null,width:text|null,
  frame:text|null,grid:text|null,stripes:text|null,float:text|null,
  orientation:text|null,colcount:Counter,rowcount:Counter,
  tablepcwidth:text|null,tableabswidth:text|null,
  origins:{cols:D2SourceOriginSet/2,format:D2SourceOriginSet/2,
           separator:D2SourceOriginSet/2,width:D2SourceOriginSet/2,
           frame:D2SourceOriginSet/2,grid:D2SourceOriginSet/2,
           stripes:D2SourceOriginSet/2,float:D2SourceOriginSet/2,
           orientation:D2SourceOriginSet/2,colcount:D2SourceOriginSet/2,
           rowcount:D2SourceOriginSet/2,tablepcwidth:D2SourceOriginSet/2,
           tableabswidth:D2SourceOriginSet/2}
}

D2TableBlock/3 = {
  kind:"table",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2TableSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  columns:[D2TableColumn/3...],
  rows:[D2TableRow/3...]
}

D2DelimitedSemantics/1 =
    {kind:"listing"|"source",language:text|null,linenums:Boolean,
     start:CanonicalSignedDecimal|null,indent:CanonicalSignedDecimal|null,
     tabsize:CanonicalSignedDecimal|null,highlight:text|null,
     lineComment:text|null,
     origins:{language:D2SourceOriginSet/2,linenums:D2SourceOriginSet/2,
              start:D2SourceOriginSet/2,indent:D2SourceOriginSet/2,
              tabsize:D2SourceOriginSet/2,highlight:D2SourceOriginSet/2,
              lineComment:D2SourceOriginSet/2}}
  | {kind:"literal",indent:CanonicalSignedDecimal|null,
     tabsize:CanonicalSignedDecimal|null,lineComment:text|null,
     origins:{indent:D2SourceOriginSet/2,tabsize:D2SourceOriginSet/2,
              lineComment:D2SourceOriginSet/2}}
  | {kind:"quote",attribution:text|null,citetitle:text|null,
     origins:{attribution:D2SourceOriginSet/2,citetitle:D2SourceOriginSet/2}}
  | {kind:"verse",attribution:text|null,citetitle:text|null,
     indent:CanonicalSignedDecimal|null,tabsize:CanonicalSignedDecimal|null,
     origins:{attribution:D2SourceOriginSet/2,citetitle:D2SourceOriginSet/2,
              indent:D2SourceOriginSet/2,tabsize:D2SourceOriginSet/2}}
  | {kind:"stem",notation:text|null,
     origins:{notation:D2SourceOriginSet/2}}
  | {kind:"pass"}

D2DelimitedBlock/3 = {
  kind:"delimited",
  blockKind:"listing"|"literal"|"source"|"pass"|"stem"|"quote"|"verse",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  style:text|null,
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...],
               style:D2SourceOriginSet/2,
               contentText:D2SourceOriginSet/2|null},
  common:D2BlockCommonSemantics/1,
  semantics:D2DelimitedSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  content:
      {kind:"text",text:text,sourceOrigins:D2SourceOriginSet/2}
    | {kind:"inline",inlines:[D2ProductInline/3...]}
    | {kind:"blocks",children:[D2ProductBlock/3...]}
}

D2ContainerSemantics/1 =
    {kind:"admonition",name:text,textlabel:text|null,icon:text|null,
     origins:{name:D2SourceOriginSet/2,textlabel:D2SourceOriginSet/2,
              icon:D2SourceOriginSet/2}}
  | {kind:"example"|"sidebar"|"open"|"preamble"|"abstract"|"partintro"}

D2ContainerBlock/3 = {
  kind:"container",
  blockKind:"example"|"sidebar"|"open"|"admonition"|"preamble"|
            "abstract"|"partintro",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2ContainerSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  children:[D2ProductBlock/3...]
}

D2MediaSemantics/1 =
    {kind:"image",width:text|null,height:text|null,format:text|null,
     scaledwidth:text|null,scale:text|null,link:text|null,window:text|null,
     float:text|null,align:text|null,fallback:text|null,imagesdir:text|null,
     origins:{width:D2SourceOriginSet/2,height:D2SourceOriginSet/2,
              format:D2SourceOriginSet/2,scaledwidth:D2SourceOriginSet/2,
              scale:D2SourceOriginSet/2,link:D2SourceOriginSet/2,
              window:D2SourceOriginSet/2,float:D2SourceOriginSet/2,
              align:D2SourceOriginSet/2,fallback:D2SourceOriginSet/2,
              imagesdir:D2SourceOriginSet/2}}
  | {kind:"audio",start:text|null,end:text|null,
     autoplay:Boolean,controls:Boolean,loop:Boolean,
     origins:{start:D2SourceOriginSet/2,end:D2SourceOriginSet/2,
              autoplay:D2SourceOriginSet/2,controls:D2SourceOriginSet/2,
              loop:D2SourceOriginSet/2}}
  | {kind:"video",poster:text|null,width:text|null,height:text|null,
     start:text|null,end:text|null,preload:text|null,float:text|null,
     align:text|null,hash:text|null,theme:text|null,lang:text|null,
     list:text|null,playlist:text|null,autoplay:Boolean,loop:Boolean,
     muted:Boolean,controls:Boolean,fullscreen:Boolean,modest:Boolean,
     related:Boolean,
     origins:{poster:D2SourceOriginSet/2,width:D2SourceOriginSet/2,
              height:D2SourceOriginSet/2,start:D2SourceOriginSet/2,
              end:D2SourceOriginSet/2,preload:D2SourceOriginSet/2,
              float:D2SourceOriginSet/2,align:D2SourceOriginSet/2,
              hash:D2SourceOriginSet/2,theme:D2SourceOriginSet/2,
              lang:D2SourceOriginSet/2,list:D2SourceOriginSet/2,
              playlist:D2SourceOriginSet/2,autoplay:D2SourceOriginSet/2,
              loop:D2SourceOriginSet/2,muted:D2SourceOriginSet/2,
              controls:D2SourceOriginSet/2,fullscreen:D2SourceOriginSet/2,
              modest:D2SourceOriginSet/2,related:D2SourceOriginSet/2}}

D2MediaBlock/3 = {
  kind:"media",mediaKind:"image"|"audio"|"video",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  nativeTarget:text,alt:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{nativeTarget:D2SourceOriginSet/2,alt:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2MediaSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  adapter:D2IdentityAdapter/1|null
}

D2AtomicSemantics/1 =
    {kind:"floating_title"}
  | {kind:"page_break"}
  | {kind:"thematic_break"}
  | {kind:"toc",levels:CanonicalSignedDecimal|null,
     origins:{levels:D2SourceOriginSet/2}}

D2AtomicBlock/3 = {
  kind:"atomic",
  blockKind:"floating_title"|"page_break"|"thematic_break"|"toc",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,
  level:D2ProductInteger|null,
  title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,level:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2AtomicSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1
}

D2SavedDefinitionBlock/3 = {
  kind:"saved_query_view_definition",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:D2SourceOriginSet/2,
  definitionKind:"query"|"view"|"dynamic_block",
  payload:text,
  slotOrigins:{definitionKind:D2SourceOriginSet/2,payload:D2SourceOriginSet/2}
}

D2BibliographyPlacementBlock/3 = {
  kind:"bibliography_placement",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:D2SourceOriginSet/2
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
  evaluation:D2ProductEvaluationBinding/1,
  originGraph:D2SourceOriginGraph/2,
  document:D2DocumentPayload/3
}

D7HeadingProjection/2 = {
  owner:NodeRef,
  title:text,
  level:CanonicalSignedDecimal
}
```

All product unions are strict: unknown/missing members, unknown arms, duplicate JSON keys, illegal nulls, and cross-arm members reject. D2ProductBlock/3 and D2ProductInline/3 are the current complete fixed-baseline product families plus the named Weftext extension blocks; there is no generic/opaque block, inline, property JSON, or second AST escape.

D2NativeAttributeSet/1 is the only variable-name native attribute carrier. Its values are the closed D2NativeSemanticValue/1 union; entries preserve the fixed parser's actual final named-attribute-map enumeration order and have unique names. Arbitrary legal author attribute names therefore remain representable without introducing free JSON. D2BlockCommonSemantics/1 separately carries the public AbstractBlock slots that are not reliably recoverable from that named map: actual style, caption, numeral, substitution list and positional values in original positional order. Dedicated per-kind semantic members are not optional shortcuts: when the same fixed-Ruby fact is present in nativeAttributes/common and a dedicated member, they must agree byte-for-byte after the fixed semantic conversion. Internal temporary attributes are excluded unless the fixed public model/converter can observe them. Inline image semantics additionally retain image versus icon type; reference anchors retain ref versus bibref, footnotes retain only the closed null|ref|xref type, and callout guard preserves either its scalar guard or exact two-part comment guard.

Field-level provenance is part of the closed product value, not a second parse. Every D2NativeAttributeEntry/1 directly carries sourceOrigins. D2BlockCommonSemantics/1, every named per-kind semantics carrier, and block/inline slotOrigins objects mirror their semantic members field by field. For a sequence-valued slot, the origin array has exactly the value-array length and pairs in the same order; a null or derived scalar still has its own D2SourceOriginSet/2. Recursive D2ProductInline/3 sequences such as title/label/children/terms carry field origins in each child, so a parent's coarse sourceOrigins cannot stand in for an independently addressable slot. Unknown/missing origin members, length mismatch, reusing one independent slot's origin for another, or reconstructing a mapping from sourceRange/path/text all fail strict validation.

A structured write gate uses only the edited slot's D2SourceOriginSet/2.writableSource, then applies current write authorization for that exact range.source owner and final evaluation/read-barrier currentness. When writableSource is unique and that source owner is writable/current, the owner's existing structured-edit positive path must remain available. generated, multi_origin, ambiguous, non_author_source, or missing write authorization leaves the slot readable but structured-readonly. A unique authored range in a managed include requires write permission to that include's own source owner and never inherits root-Document write permission. For `[source,ruby]`, block style, language, and body text bind their distinct spans; for `[#foo.red]`, id, each role, and child text bind their distinct spans. Reference/substitution/environment-derived values retain their real graph inputs but never infer a writable target from a node-level range. Any stale bound source/environment/dependency at the final barrier invalidates the old slot write and requires reprepare.

D2DocumentMetadata/3 is the product carrier for the fixed Ruby 2.0.26 final Document public/converter-observable state, not an approximate header summary. The backend quartet and safeMode/doctype come from the same evaluation; title is the complete doctitle and mainTitle/subtitle are the same fixed title partition. authors preserve Document#authors order and all six name/firstname/middlename/lastname/initials/email members; D9 or UI code must not split them again from a full name or source. effectiveAttributes preserves the complete present fixed Document#attributes map in actual enumeration order with unique names, including converter-visible builtin/generated/environment-derived attributes. Each entry uses sourceOrigins for authored/substitution/generated provenance and never fabricates a physical sourceRange for a value with no author span. headerAttributes is only the real physical lexical header-entry list with ranges and is not the effective-attribute authority. Revision number/date/remark and every other independent document-level member likewise retain field-level origins. D9/D8 consumers use these exact product members and never guess missing state from raw source, fullname, a default backend, or ambient environment.

effectiveAttributes[].value uses exactly the already-closed D2NativeSemanticValue/1 domain. The final fixed-Ruby Hash key's presence and Ruby value class are preserved: an absent key has no effectiveAttributes entry; a present nil is null; TrueClass/FalseClass is Boolean; String is text; Integer is {integer:CanonicalSignedDecimal} using the exact unbounded canonical base-10 form; Array is admitted only as {textArray:[text...]} when every element is a String, preserving element order, duplicates, and empty strings. Fixed-core safe-mode-level, max-include-depth, and authorcount therefore remain integers, while mannames remains an ordered text array as consumed by the fixed HTML5/DocBook converters. No producer may to_s, JSON-stringify, join, normalize, sort, or reparse these values. ProcessorAttributeOverride/1 itself admits only text set-values and null unset actions and therefore cannot inject another Ruby class. An accepted extension profile is product-compatible only when every present final Document#attributes value remains in this closed domain; Symbol, Float, Hash, nested/mixed/non-String Array, or another opaque object makes the rich product projection unavailable rather than creating free JSON. The typed value and sourceOrigins come from the same fixed evaluation; generated/non-author values never fabricate a physical range, and D7/D8/D9 consume the typed value directly.

D2SourceOriginGraph/2 is local to one snapshot. origin ids are dense 0..N-1; every input id is smaller than the node that cites it, so the graph is acyclic without a second identity namespace. authored has one physical source range. reference identifies the actual authored reference site plus its source input. substitution/generated identify the fixed transformation family and all actual inputs. multi_origin has at least two inputs. sourceOrigins.originIds are sorted/unique and refer to existing graph nodes. Its writableSource is mechanically derived from those nodes: unique only when every retained path converges on one byte-equal authored writable range; generated, non-author, ambiguous, or multi-source values are read-only for structured writes. Exact Source read/save remains independently available under its original authorization.

D2SourceOwner/1 is version-exact. root_document and managed_include carry the actual SourceObservation/1 used by evaluation; artifact/network units carry their exact immutable pin, and every variant carries the logicalPath from the processor environment. No path/title/hash infers a source unit. A current managed D2DocumentSnapshot/3 requires processorEnvironment.input.kind=managed_file, its ownerNodeRef byte-equal to snapshot.ownerNodeRef, and rootSource.kind=managed with the same owner/SourceObservation/logicalPath. includeSources is byte-equal to processorEnvironment.includeEnvironment.sourceUnits. processorEnvironmentSha256 is exactly:

```text
"sha256:" + lowercase_hex(
  SHA-256(
    UTF8("D2-Processor-Environment/3") || NUL ||
    D3-CJ/3(processorEnvironment)
  )
)
```

The readBarrier Workspace/CommitDomain equals dependencyProof, readBarrier.frontier equals dependencyProof.baseFrontier, and observationScope is the scope used to obtain that proof. The proof contains the root source key, every managed include source key, document_format for a managed root, and every other Registry/foreign/authorization dependency actually consumed by the native/Weftext projection. Each SourceUnitBinding managed observation or immutable pin is byte-equal to the corresponding source owner in originGraph. A source unit or environment not used by the evaluation must not be inserted merely to make a larger proof.

A current D7/D8/D9 consumer validates the complete evaluation binding at its final authorized read barrier before consuming projection bytes. The exact same cut is valid directly; a later cut may be used only through the retained continuous scope_dependencies rule proving all bound source units, environment-relevant control, authorization and negative dependencies unchanged. Changing a managed include SourceObservation, artifact/network pin, processor-environment bytes, or any actual dependency makes the old tree stale even when root source and document_format are byte-equal. Snapshot pin proves what tree was produced; evaluation binding proves whether that tree is current.

For an available projection, every addressable product occurrence has the exact current D3 DocumentElementLocator required by its type. D2SourceRange/2 byte intervals are half-open and line/column endpoints refer to the exact versioned source owner; start is not after end. Semantic sequences preserve parser order. roles/options preserve their language-defined order; unique sets are explicitly named as such by their owner. columnStart is the zero-based physical table-grid column after applying preceding row/column spans and is unique for each occupied cell origin in one row.

BaselineOnly accepts only headings the fixed Ruby baseline actually parses. WeftextManaged additionally admits authoredLevel 6–9. effectiveLevel is a nonnegative CanonicalSignedDecimal carrying the fixed Ruby Integer result after its lower clamp only; it has no Counter/int64 upper language limit. Resource/work budgets may return budget_exceeded while processing a huge legal level, but they never reclassify the source as syntax-invalid or numeric_overflow. D7 headings exposes this value with existing TypeSpec {kind:"integer"} and its canonical decimal V encoding; the old int64 heading projection is historical only.

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
  expectedAuthority?:ExpectedAuthority,
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
  expectedAuthority?:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  preparationBinding?:PreparationBinding,
  intent:D3Intent
}
```

D3IdentityInput/13 has the exact D3IdentityInput/12 semantic member set: only the nested D6 descriptor family changes to current InputDescriptor/3 at the outer request boundary. expectedAuthority, workspaceProposal and preparationBinding are genuinely optional members, never nullable placeholders. replica_local permits only create_node/move_node/reorder_node/trash and requires all three members absent. managed_atomic follows the fixed-parent matrix exactly: create/fork require expectedAuthority=create and the required proposal; continue requires expectedAuthority=continue; other managed_atomic modes require expectedAuthority=existing; preparationBinding appears only where the inherited D7-mediated mode admits it. A forbidden member is invalid_request even when its value would otherwise decode, and JSON null is always invalid. D3IdentityOperationRequest/13 is the exact wire12 top-level member set with wireVersion=13 and InputDescriptor/3. Descriptor/request mode, authority and proposal are byte-equal wherever present; guarantee/frontier policy, ownerInput protocolOwner=D3/ownerKind=d3_identity_operation/13, requestFingerprint rule, DecisionKey/OperationId ledger ordering, saved/planned/unseen branching, and error/disclosure order are inherited unchanged. Historical wire9–12 are not widened or re-encoded.

## 6.1 PreparedActionBinding/4

The current D7 action successor is exact inheritance, not a free extension. D7ActionSpec/2 is the fixed-parent ActionSpec/1 top-level object with version=2; its intent decoder is exactly the fixed-parent intent union with only the historical apply_suggestion arm removed, then the three closed Annotation arms below added. Every other fixed-parent arm keeps its members and semantics byte-for-byte.

```text
D7CreateAnnotationIntent/2 = {
  kind:"create_annotation",
  destinationOwnerRef:NodeRef,
  value:AnnotationEditableProposal/1
}

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
  plus D7CreateAnnotationIntent/2,
       D7ApplySuggestionIntent/2 and D7RejectSuggestionIntent/2

D7ActionPrepareRequest/3 = {
  wireVersion:3,kind:"d7_action_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  action:D7ActionSpec/2,
  selectedSources:[SourceVersionRef/1...],
  budget:BudgetBinding/1,
  evidenceToken?:Token
}

D7ActionInput/3 = {
  kind:"d7_action_input",version:3,
  action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext...],
  proposedInputs:[D7ProposedInput/3...]
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
  proposedInputs:[D7ProposedInput/3...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,expiresAt:<D6 protected deadline>,
  request:D3IdentityOperationRequest/13|d6_commit_request/2,
  preview:<complete EffectManifest/3>,
  resolutionInput:null|D3ResolutionInput/1
}
```

MinimumMapping/3, D7DefinitionInput/2, and D7ResolutionAccess/1 retain their exact fixed-e8aa shapes. D7ProposedInput/2 is retained only for genuine historical d7_action/2 / PAB3 decoding and is never widened in place.

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
```

The only legal current `/3` cross-fields are exact_source_document↔exact_source_utf8, resource_bytes↔resource_bytes, and annotation_value↔d3_annotation_value4|d3_symbolic_result9. A protocolOwner=D6 current concrete Annotation after uses d3_annotation_value4 and pins exactly D3-CJ/3(the complete D3-Annotation-Value/4). d3_symbolic_result9 remains legal only for the inherited current D3 symbolic-result branch under its real Result/9 subject/pin rules. `d3_annotation_value3` is invalid in `/3`. OwnerInputBinding/2.pinRefs exactly covers every `/3` proposed pin plus actually protected evidence. Historical PAB3/Input2 continues byte-for-byte with D7ProposedInput/2 and d3_annotation_value3 and is never repinned or re-encoded.

For protocolOwner=D6 current D7 actions, InputDescriptor/3.ownerInput has protocolOwner=D7 and ownerKind=intentKind=d7_action/3; canonicalDescriptorBytes is exactly D3-CJ/3(D7ActionInput/3). The six D7ActionInput/3 members equal their PreparedActionBinding/4 counterparts individually, and ownerInput.pinRefs continues to cover exactly proposed pins plus actually protected source/definition/rule evidence under D6 sorted/unique pin rules. protocolOwner=D3 identity actions such as create_annotation keep D3's own d3_identity_operation/13 owner descriptor; PAB4 binds cross-owner preparation and creates no second D3 request authority. Historical d7_action/2/PAB3 keeps its original decoder, bytes and recovery.

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

EffectItem/3 retains the fixed-parent fourteen arms and adds exactly one current owner-specific arm, presentation_policy_change:D8PresentationPolicyEffect/1. This is the same pattern as series_configuration_change: it is a typed shared-configuration owner effect, not a new PortableComponentKey, Policy/3 mutation, Registry value, author source, or generic JSON. Every byte slot uses EffectBytes/3, Annotation source images use Value/4, and current workspace bootstrap admits Plan4. Historical EffectItem/1-/2 decoders are not widened.

## 6.3 PreparedEditBinding/3

```text
D8EditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     source:text}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     value:AnnotationEditableProposal/1,
     targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1}

D8PinnedEditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     proposedSource:PinRef/2}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2,
     targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2}

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

### 6.3.1 Current Annotation read / Draft producer and operation-class input

```text
D8AnnotationReadRequest/1 = {
  wireVersion:1,kind:"d8_annotation_read",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef
}

D8AnnotationBodyRead/1 =
    {state:"absent",exactSource:null,semanticText:""}
  | {state:"valid",exactSource:text,semanticText:text}
  | {state:"invalid",exactSource:text,semanticText:null,
     diagnostics:[CoreDiagnostic/1...]}

D8AnnotationReadResponse/1 = {
  kind:"d8_annotation_read",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef,
  sourceObservation:SourceObservation/1,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4,
  targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
  body:D8AnnotationBodyRead/1
}

D8AnnotationDraftOpenRequest/1 = {
  wireVersion:1,kind:"d8_annotation_draft_open",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef
}

D8AnnotationDraftOpenResponse/1 = {
  kind:"d8_annotation_draft_open",version:1,
  read:D8AnnotationReadResponse/1,
  draft:D8AnnotationDraftProjection/1
}
```

These are real Core/D8 entries, not response-only shapes. `d8_annotation_read` returns the complete Value/4 at the final authorized read barrier, including creator/authoredAt/lastEditor/editedAt for display. Draft still stores/edits only AnnotationEditableValue/1 and is never a second author authority. `d8_annotation_draft_open` reuses the same current read: draft.access=editable with annotation_write and readonly otherwise; readonly cannot enter D8EditPrepareRequest/3. Draft baseObservation/baseRevisionToken equal read.sourceObservation/read.annotationRevisionToken, and at serial=0 its editable projection equals the eight editable fields of read.value. Local Draft may subsequently diverge, but prepare and the final read barrier revalidate the original base token/Observation.

The D8EditIntent/3 operation class is mechanically derived from its closed arm and targetPolicy, never from a caller trusted flag: annotation+preserve=ordinary_edit, annotation+replace_current=manual_reattach, annotation_reconfirm_suggestion=reconfirm_suggestion. Caller AnnotationEditableProposal/1 carries no Suggestion lifecycle/evidence. Core first applies the closed operation-class gate to current before + proposal, expands one complete candidate Value/4 with before attribution preserved, and only then performs canonical candidate-vs-before no-op comparison. Only a differing candidate is upgraded with fresh Core lastEditor/editedAt, a fresh revision token and the unique planned/pinned final Value/4. The reconfirm arm carries no caller value; its proposedValue is entirely produced by a fresh Core target read/recomputation and a successful reconfirm is a real state transition.

## 6.4 D8 workspace presentation policy and render binding

Presentation preference is shared Workspace configuration in existing Portable Workspace Metadata, but it is a D8-owned complex configuration like SeriesScopeConfiguration. It deliberately does not add a PortableComponentKey arm or reuse Policy/3/Registry. Currentness is proved by the complete D8 head set below and the same D6 planning CAS/P decision that writes its owner effect.

```text
D8PresentationPolicyAddress/1 = {
  workspaceRef:WorkspaceRef,
  revision:Counter,
  recordSha256:"sha256:<64 lowercase hex>"
}

D8WorkspacePresentationPolicy/1 = {
  kind:"d8_workspace_presentation_policy",version:1,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  defaultPresentation:"separate"|"run_in"
}

D8WorkspacePresentationPolicy/2 = {
  kind:"d8_workspace_presentation_policy_record",version:2,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  parents:[D8PresentationPolicyAddress/1...],
  defaultPresentation:"separate"|"run_in",
  activationChangeId:ChangeId/1
}

D8PresentationPolicyHeadSet/1 = {
  kind:"d8_presentation_policy_heads",version:1,
  workspaceRef:WorkspaceRef,
  stamp:{epoch:Token,revision:Counter},
  heads:[D8PresentationPolicyAddress/1...]
}

D8WorkspacePresentationPolicyBinding/2 = {
  policy:D8WorkspacePresentationPolicy/1,
  record:D8WorkspacePresentationPolicy/2,
  address:D8PresentationPolicyAddress/1,
  headSet:D8PresentationPolicyHeadSet/1,
  pin:PinRef/2
}

D8PresentationPolicyProposal/1 = {
  workspaceRef:WorkspaceRef,
  parents:[D8PresentationPolicyAddress/1...],
  revision:Counter,
  defaultPresentation:"separate"|"run_in"
}

D8PresentationPolicyBootstrapInit/1 = {
  kind:"d8_presentation_policy_bootstrap_init",version:1,
  before:D8PresentationPolicyHeadSet/1,
  proposal:D8PresentationPolicyProposal/1
}

D8PresentationPolicySetRequest/2 = {
  wireVersion:2,
  kind:"d8_presentation_policy_set",
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  expectedHeads:[D8PresentationPolicyAddress/1...],
  defaultPresentation:"separate"|"run_in",
  budget:BudgetBinding/1
}

D8PresentationPolicyInput/1 = {
  kind:"d8_presentation_policy_input",version:1,
  workspaceRef:WorkspaceRef,
  headSet:D8PresentationPolicyHeadSet/1,
  defaultPresentation:"separate"|"run_in"
}

D8PresentationPolicyHeadEvidence/1 = {
  address:D8PresentationPolicyAddress/1,
  recordPin:PinRef/2
}

D8PresentationPolicyPrepareResult/2 =
    {kind:"d8_presentation_policy_no_change",version:2,
     workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
     headSet:D8PresentationPolicyHeadSet/1}
  | {kind:"d8_presentation_policy_prepared",version:2,
     operationId:UUIDv4,
     input:D8PresentationPolicyInput/1,
     proposal:D8PresentationPolicyProposal/1,
     headEvidence:[D8PresentationPolicyHeadEvidence/1...],
     prepared:PreparedIntent/3,
     request:d6_commit_request/2,
     preview:EffectManifest/3}

D8PresentationPolicyOutboxItem/1 = {
  kind:"d8_presentation_policy_outbox",version:1,
  decisionKey:DecisionKey/2,
  address:D8PresentationPolicyAddress/1,
  recordPin:PinRef/2
}

D8PresentationPolicyEffect/1 = {
  kind:"presentation_policy_change",version:1,
  before:D8PresentationPolicyHeadSet/1,
  proposal:D8PresentationPolicyProposal/1,
  committed:null|D8WorkspacePresentationPolicy/2
}

D8PresentationDecision/1 =
    {kind:"explicit",
     sourceOverride:"run_in"|"separate"|"conflict",
     effective:"run_in"|"separate",
     diagnostic:null|"role_conflict"}
  | {kind:"no_eligible_body",effective:"separate"}
  | {kind:"workspace_default",
     policy:D8WorkspacePresentationPolicyBinding/2,
     effective:"run_in"|"separate"}

D8PresentationResult/1 = {
  heading:DocumentElementLocator,
  body:DocumentElementLocator|null,
  decision:D8PresentationDecision/1
}

D8DocumentRenderBinding/1 = {
  sourceObservation:SourceObservation/1,
  documentFormat:DocumentFormatCurrentQualification/1,
  documentSnapshotPin:PinRef/2,
  presentation:D8PresentationDecision/1,
  rendererProfileSha256:"sha256:<64 lowercase hex>"
}
```

D8WorkspacePresentationPolicy/2 record bytes are exactly:

```text
UTF8("D8-Workspace-Presentation-Policy/2") || NUL ||
D3-CJ/3(complete policy)
```

recordSha256 hashes those complete prefixed bytes. PinRef/2 selects those exact bytes with payloadKind=portable_metadata and recovery retention; address.workspaceRef/revision equal the record. policy in D8WorkspacePresentationPolicyBinding/2 is the unique logical current view mechanically projected as {kind:"d8_workspace_presentation_policy",version:1,workspaceRef:record.workspaceRef,revision:record.revision,defaultPresentation:record.defaultPresentation}; it is not separately stored. This preserves the Stage4A /1 current-value contract without introducing another authority. parents are sorted/unique by (revision,recordSha256), all belong to the same Workspace, and every parent record must be retained and valid. The initial record has revision=1, parents=[] and defaultPresentation=separate. Any successor has parents equal the complete protected current head set at preparation/seal, revision=checked(max(parent.revision)+1), and activationChangeId equal to the one ChangeId allocated by that same portable D6 P decision. Thus two offline writers may both create revision 2, but their different hashes/ChangeIds remain two heads; revision alone, arrival order and LWW never choose a winner.

D8PresentationPolicyHeadSet/1 is the D8 owner-specific current observation of all maximal valid policy records in Portable Workspace Metadata. Its stamp is protected Core state for this exact Workspace configuration: a freshly established continuous proof epoch starts at revision=1 after complete head enumeration; epoch changes on continuity loss/rebuild of correctness evidence, and revision checked-increments for every later proved head-set transition. Neither number selects a branch. It is not caller evidence or an index scan. heads are sorted/unique; one head is current and more than one is a conflict. heads=[] is a **proved uninitialized owner state**, never evidence inferred from absence: it is legal only when an actual producer binds a complete protected empty-set observation, namely D8PresentationPolicyBootstrapInit/1.before for a still-unactivated custody-proved fresh target or an explicitly proved current uninitialized state admitted to SetRequest. Missing/corrupt metadata, an unknown parent decoder, a missing retained record, an unproved continuity gap, or merely no backing file is unavailable and never [] or an ambient default. A successfully activated current Plan4 fresh Workspace is never observable with [] because its initial record/head commits atomically with activation. A current binding requires headSet.heads == [address], record/pin/address byte equality, policy equal to the mechanical /1 view of record, and exact retained ancestry.

D8PresentationPolicyBootstrapInit/1 is not a public mutation request. before.workspaceRef and proposal.workspaceRef are byte-equal to the fresh target Workspace; before.heads=[] and before.stamp is a protected fresh D8 owner observation derived from the original D3 target-custody proof of genuinely empty target history, with before.stamp.revision=1. proposal is exactly parents=[], revision=1, defaultPresentation=separate. The helper carries no activationChangeId, committed record, address, hash, pin, outbox item, or second authorization decision and is legal only inside current unseen WorkspaceBootstrapPlan/4.

D8PresentationPolicySetRequest/2 is managed_atomic/strict and has a concrete current producer. Error order is closed decode → presentation-state disclosure → policy_admin → expectedFrontier/domain qualification → protected head-set read → exact expectedHeads equality → retained head-record/pin/ancestry availability → semantics/budget. An earlier failure does not disclose hidden heads or values. With exactly one current head whose defaultPresentation already equals the request, preparation returns d8_presentation_policy_no_change with the authorized exact headSet and stops with no PreparedIntent, P, ChangeId, record, or outbox. Initialization from [] and a multi-head conflict are never no-ops even when all conflicting values happen to agree.

A changing path builds D8PresentationPolicyInput/1 and Proposal/1. proposal.parents is the complete frozen headSet.heads, revision is 1 for [] or checked(max(parent.revision)+1), and defaultPresentation is the request; Proposal never contains activationChangeId. headEvidence is one-for-one in head order, address byte-equals the corresponding head, and recordPin selects the retained parent's exact canonical record bytes. Core then builds one existing PreparedIntent/3: InputDescriptor/3 uses protocolOwner=D8, ownerKind=intentKind=d8_presentation_policy/1, saveProfile=control_only, guarantee=managed_atomic, frontierPolicy=exact, observationScope=the existing control_only Workspace scope, and sourceInputs=[]. canonicalDescriptorBytes=D3-CJ/3(input); controlInputs/DependencyProof contain exactly the authorization and other real control dependencies actually read; pinRefs exactly cover headEvidence plus real authorization/dependency evidence. This P-only shared-configuration producer acquires no author source or SourceRevisionPlan and never turns P into an author-content store. The sole ChangeId remains the ordinary portable decision ChangeId allocated only at final P below. preview contains exactly one presentation_policy_change with before=frozen headSet, proposal=frozen proposal, committed=null. The prepared/request members of D8PresentationPolicyPrepareResult/2 are that one current PreparedIntent/3 and the original d6_commit_request/2 generated from the same operationId/planToken; Core atomically saves this outside association before return. Saved/planned/unknown recovery restores those original bytes/pins/head stamp/proposal/request and never resamples heads.

The winning planning CAS freezes the exact Input, head-set stamp, headEvidence, Proposal, authorization/dependency/budget, preview, and D6 plan responsibility. Because the current record necessarily contains an activationChangeId that does not yet exist, prepare, planning, staging, InstallationNotice, install, and verify never construct/encode D8WorkspacePresentationPolicy/2, compute recordSha256, create its portable_metadata pin or D8PresentationPolicyOutboxItem/1, or reserve a ChangeId. Presentation policy continues to use the fixed-parent complex-owner-effect class whose real EffectManifest rank-6 precedent is series_configuration_change: EffectItem/3 uses typed presentation_policy_change without adding PortableComponentKey. Any existing component install/verify work in that D6 decision describes only its real components and never invents a policy component.

Final P is the sole materialization point. P first follows original D6 ordering to revalidate policy_admin/authorization, domain fence, expectedFrontier, the complete frozen head-set stamp/heads, parent records/ancestry, dependencies, and budget. If another writer changes the head set after the old-head precheck, this plan conflicts/reprepares before any new ChangeId exists. Only after all checks pass does the same P transaction checked-allocate the one ChangeId C, construct the unique D8WorkspacePresentationPolicy/2 from frozen Proposal+C, strict D3-CJ/3 encode it with the fixed prefix, compute recordSha256, create its exact recovery portable_metadata PinRef/2/address, and atomically persist the committed presentation_policy_change (before/proposal/committed), original decision/receipt/effects, ChangeRecord association, D8PresentationPolicyOutboxItem/1, and the owner head-graph transition to that successor. record.activationChangeId, ChangeRecord.changeId, receipt/effect decision, and outbox decisionKey all agree. Any failure before P commits leaves no partial record/pin/head.

Two offline writers may separately seal from one parent and later synchronize into two maximal heads; arrival/hash/revision-number/LWW never selects a winner. Explicit resolution names the complete current multi-head set as parents and runs this same producer to one multi-parent successor. If P commits but publication/transport fails, retry only dereferences D8PresentationPolicyOutboxItem/1 and republishes the exact retained record bytes; it never re-encodes, reallocates ChangeId, recomputes the proposal, or changes heads. Receiver admission requires strict canonical record/address/pin validation plus activationChangeId, original EffectManifest/receipt/ChangeRecord decision association, parent ancestry, and head-set continuity. Equal hash, a trusted sender, a Boolean 'verified' claim, or provider latest is not proof. Revocation before final P stops under the original authorization error with zero ChangeId; revocation after P affects current disclosure/future management only and does not rewrite the historical decision/outbox. No second Policy store, ledger, CAS, or migration layer is introduced.


Sync/admission validates each immutable record domain/canonical bytes, activationChangeId, the exact retained ChangeRecord/receipt/EffectManifest presentation_policy_change association, ancestry and current head-set stamp continuity before adding it to the portable head graph. A record with a missing/unknown historical decoder, unproved activation decision or discontinuous head-set observation is unavailable and never current. Conflicting heads remain visible as a conflict state until an authorized explicit multi-parent successor resolves them. Compaction may release a non-head record only after every descendant/recovery reference that needs it is retained elsewhere under the existing last-reference rules. No device-local preference or hidden host default can replace this Workspace-wide record.

Run-in resolution is conditional. role_conflict is explicit+Separate and consumes no policy. Explicit separate consumes no policy. Explicit run-in consumes no policy and becomes RunIn only when the existing explicit-role semantic-adjacency body is eligible; otherwise no_eligible_body. With neither role, no eligible body is no_eligible_body and consumes no policy. Only a body eligible for the implicit-default physical-adjacency rule uses workspace_default and therefore requires a current one-head policy binding. Missing/conflicted policy makes only that default-dependent presentation unavailable.

D8 cache identity contains the complete D8DocumentRenderBinding/1. A policy change invalidates only cache entries whose presentation.kind=workspace_default and whose policy binding is no longer current. Explicit role/no-body/conflict results remain valid when unrelated Workspace policy changes, subject to their normal source/product dependencies.


## 6.5 ExportPlan/3 / PublicationReceipt/3

This section freezes the recorded /3 predecessor shape and the shared export-member rules that remain applicable to corresponding members of the §6.6 /4 successor. Fresh unseen current export uses /4. The /3 decoder, token tags, bytes, pins, confirmation and saved/planned/unknown recovery remain exact and are never widened in place. Unless §6.6 explicitly extends a closed member set, /4 inherits the §6.5 generation-policy, controlled-name, canonical-ordering, template/route/style/document-render, proof/pin and publication-durability rules.

The current /3 family keeps the fixed-parent ExportInputCatalog/2, ExportContentSelection/1, ExportProjection/1 and ExportLossReport/1. Per-plan generation choices are finite and belong in the immutable Plan; there is no separate generation-policy registry or hash-only descriptor authority.

```text
D9ExportPlanToken/3 := D6 Token tagged "d9_export_plan/3"
D9PublicationToken/3 := D6 Token tagged "d9_publication/3"

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

D9TemplateBindingChoice/1 = {
  templatePath:text,
  inputIndex:Counter
}

D9MissingPolicyChoice/1 = {
  templatePath:text,
  action:"empty"
}

D9ImageSizeChoice/1 = {
  resource:ResourceRef,
  widthMicrometres:Counter,
  heightMicrometres:Counter
}

D9LayoutChoice/1 = {
  resource:ResourceRef,
  choice:"preserve_aspect_within_box"|"use_exact_dimensions"
}

D9NativeTableSelector/1 = {
  inputIndex:Counter,
  tableLocator:DocumentElementLocator,
  tableQualifier:
      {kind:"unqualified"}
    | {kind:"title",text:text}
    | {kind:"occurrence",title:text|null,ordinal:Counter},
  columnQualifier:
      {kind:"leaf",text:text}
    | {kind:"header_suffix",segments:[text...]}
    | {kind:"occurrence",segments:[text...],ordinal:Counter}
}

D9NativeTableTokenBinding/1 = {
  setName:text,
  columnName:text,
  selector:D9NativeTableSelector/1
}

D9ExportGenerationPolicy/1 =
    {kind:"none"}
  | {kind:"render",
     bindingChoices:[D9TemplateBindingChoice/1...],
     missingPolicy:[D9MissingPolicyChoice/1...],
     imageSizes:[D9ImageSizeChoice/1...],
     layoutChoices:[D9LayoutChoice/1...],
     nativeTableBindings:[D9NativeTableTokenBinding/1...]}

D9DocumentRenderBinding/1 = {
  ownerNodeRef:NodeRef,
  sourceObservation:SourceObservation/1,
  documentSnapshotPin:PinRef/2,
  semanticQualification:ManagedDocumentSemanticQualification/1,
  presentation:D8PresentationDecision/1
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
  planToken:D9ExportPlanToken/3,
  lossChoices:[D9ExportLossChoice/1...]
}

D9ControlledRelativeOutputName/1 := text satisfying the §6.5 closed output-name domain

D9ExportStagedOutput/1 = {
  name:D9ControlledRelativeOutputName/1,
  byteLength:Counter,
  sha256:"sha256:<64 lowercase hex>",
  pin:PinRef/2
}

D9PublishedOutput/1 = {
  name:D9ControlledRelativeOutputName/1,
  byteLength:Counter,
  sha256:"sha256:<64 lowercase hex>"
}

ExportPlan/3 = {
  kind:"d9_export_plan",version:3,
  planToken:D9ExportPlanToken/3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,authorizationGeneration:Token,
  inputDomain:D9ExportInputDomain/1,
  inputCatalog:ExportInputCatalog/2,
  contentSelection:ExportContentSelection/1,
  projection:ExportProjection/1,
  documentRenderBinding:D9DocumentRenderBinding/1|null,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  target:D9ExportTarget/1,
  initialLossReport:ExportLossReport/1,
  outputBudget:BudgetBinding/1,
  destination:D9ExportDestinationIntent/1,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  recoveryPins:[PinRef/2...],
  evidencePins:[PinRef/2...],
  stagedOutputs:[D9ExportStagedOutput/1...]
}

PublicationReceipt/3 = {
  kind:"d9_publication_receipt",version:3,
  publicationToken:D9PublicationToken/3,
  planToken:D9ExportPlanToken/3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  outputs:[D9PublishedOutput/1...],
  lossReport:ExportLossReport/1,
  lossChoices:[D9ExportLossChoice/1...],
  target:D9ExportTarget/1,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  presentation:D8PresentationDecision/1|null,
  destinationDisplay:text
}
```

D9ControlledRelativeOutputName/1 is a nonempty Unicode-scalar string whose stored UTF-8 encoding is the exact protocol name. U+002F SOLIDUS ("/") is the only path separator. The string has one or more nonempty components, does not begin or end with "/", and contains no empty component. A component is neither "." nor ".."; contains none of U+0000..U+001F, U+007F, U+005C ("\\") or the ASCII characters : * ? " < > |; and does not end in U+0020 SPACE or U+002E FULL STOP. These rules make rooted Unix names, repeated separators, Windows drive/root syntax and UNC/backslash syntax invalid rather than host-dependent aliases. Unicode scalar values outside those exclusions remain allowed; the protocol is not ASCII-only.

For the device-name check only, take each component's substring before its first U+002E, ASCII-fold A-Z to a-z, and reject con, prn, aux, nul, clock$, conin$, conout$, com1..com9 and lpt1..lpt9. The same rejection also applies to stems COM¹/COM²/COM³ and LPT¹/LPT²/LPT³ after ASCII letter folding, where the final character is respectively U+00B9, U+00B2 or U+00B3. No host device table or locale extends or weakens this protocol check; a destination capability may only reject additional names.

PortableAlias(name) is derived solely for conflict rejection. Split the already-valid name on "/". For each component, apply Unicode 15.1.0 NFC, then the Unicode 15.1.0 full default case-fold mapping from CaseFolding.txt using status C and F mappings but not locale-specific T mappings, then NFC again; join the resulting components with "/". The normalization algorithm is Unicode Standard Annex #15 for Unicode 15.1.0. Implementations must produce the same scalar result as those versioned Unicode data; ambient locale or an unpinned host Unicode library is not authority. PortableAlias never replaces the stored name and never participates in the raw UTF-8 sort.

Within one fresh-current bundle, exact duplicate names are invalid. Two distinct names with equal PortableAlias are aliases and are invalid. Treat each PortableAlias as a component vector: if one member's vector is a proper prefix of another, the set has a file/directory conflict and is invalid. The root names loss-report.json and manifest.json are reserved system members; Core supplies them exactly, and every dataFile is checked against their exact names, aliases and prefix relation. Therefore Report.txt/report.txt and canonically equivalent NFC/NFD spellings conflict; dir/file is a valid nested shape while a name containing dir\\file is invalid. A host-specific filesystem may detect additional collisions and reject the destination, but it may not alter this portable relation.

For recorded ExportPlan/3, stagedOutputs is the canonical complete staged member set: all validated dataFiles plus exact loss-report.json and manifest.json, each with its actual bytes/pin. PublicationReceipt/3.outputs remains the same complete name/digest set for bytes actually published under a recorded /3 plan. Fresh ExportPlan/4 inherits this same complete staged-member rule, and PublicationReceipt/4 plus D9PrintReceipt/1 use the corresponding controlled output members from the /4 plan. The external manifest's reportFile names exact loss-report.json and its recorded length/digest must equal that staged/published member; manifest.json remains non-self-hashing as inherited. Server-download and Resource-handoff paths select original staged dataFile bytes by this exact controlled name and cannot synthesize a replacement name. Safety/alias/prefix/reserved failure is invalid preparation, never an ExportLossChoice.

Exact-source/resource-exact/query-json plans require generationPolicy={kind:"none"}, templateBinding=null, routeBinding=null and documentRenderBinding=null; their projection contains no renderer-derived values. They remain preparable when template/provider/generation registries are unavailable because this contract requires none of them. Rendered HTML/PDF/DOCX/ODT and finite table outputs use generationPolicy.render; each array may be empty when that policy class is not used.

D9TemplateBindingChoice/1 is required only for an actually ambiguous binding that the authorized template/profile permits the user to resolve; inputIndex selects the exact frozen catalog item and cannot grant an unselected read. D9MissingPolicyChoice/1 may name only an existing exact template path whose authorized projection value is none; action=empty cannot hide an unknown path, unreadable input, type error or unavailable schema. imageSizes are ResourceRef-key sorted/unique, dimensions are positive, and every resource equals an authorized resource catalog input. layoutChoices are ResourceRef-key sorted/unique, require the same resource in imageSizes, and are present exactly when the fixed Templates rule requires an explicit layout choice. preserve_aspect_within_box deterministically scales to the largest same-ratio size not exceeding both chosen dimensions; use_exact_dimensions uses both chosen dimensions and records the required layout loss.

The /3 predecessor defines one closed comparator contract for every set-like array, and fresh ExportPlan/4 inherits those comparators for every unchanged corresponding member. routeBinding.steps uses array position i with step=i, therefore exactly 0..N-1; each step.evidencePins is sorted/unique by pinToken. styleBundles is sorted/unique by UTF-8 bytes of styleBundleId; the same ID with a different version/pin is a conflict. stagedOutputs and PublicationReceipt/3-/4 outputs first satisfy D9ControlledRelativeOutputName/1 and the complete exact-name/PortableAlias/file-directory-prefix/reserved-system-name conflict rules above, then sort/unique by the exact stored protocol name string's unsigned UTF-8 octets lexicographically: no Unicode normalization, case folding, locale collation, host/path-library collation, or separator rewriting participates in this ordering, and the shorter exact byte prefix sorts first. Any duplicate exact name is invalid, any portable alias/prefix/reserved conflict is invalid, and the same exact name with different byteLength/sha256/pin is an explicit integrity conflict, never LWW. generationPolicy.render.bindingChoices and missingPolicy use one shared templatePath Unicode-scalar lexicographic comparator: compare the exact decoded Unicode scalar sequence by scalar value, normalization=none and case-sensitive, with no locale/case folding; the shorter exact scalar prefix sorts first. Canonically equivalent but differently encoded scalar sequences remain distinct template keys unless another existing Templates rule rejects them; this template comparator is deliberately separate from the output-name alias relation. Each array is sorted/unique by its comparator, one exact templatePath cannot select two inputIndex/action bodies, and the same exact path cannot simultaneously appear as both a bindingChoice and missingPolicy. nativeTableBindings is sorted/unique by the controlled ASCII (setName,columnName) tuple; the same key with a different token/selector fails. Existing ResourceRef sorting/uniqueness for imageSizes/layoutChoices, lossKey for lossChoices, and pinToken for plan evidencePins/recoveryPins remain unchanged; implementations do not choose an alternate order.

D9 fresh prepare strict-decodes all choices, validates the complete current output-name domain and bundle conflict set, detects duplicate/conflicting keys, and performs the one canonical sort above before freezing any ExportPlan/4 bytes, /4 plan token, protected pin, staged manifest, or confirmation basis. Thus permutations of the same legal set produce byte-for-byte identical Plans. A duplicate key is rejected even when its body is byte-equal because the collection is unique; an unequal body is additionally an integrity conflict. A frozen fresh ExportPlan/4 or PublicationReceipt/4 must already be canonical and all current output names must satisfy D9ControlledRelativeOutputName/1. A recorded ExportPlan/3, PublicationReceipt/3, recovery record, or received /3 record remains on its exact /3 decoder and protected bytes; admission never sorts, normalizes, renames, repins or re-encodes it into /4. Genuine ExportPlan/1-/2 and PublicationReceipt/1-/2 likewise remain on their original decoder/bytes/name rules. No recorded /1-/3 value is retroactively migrated by the fresh-current predicate. Receipt route/style/generation-policy selections remain byte-equal to the protected Plan, while outputs use the independent raw-name comparator above.

D9ExportTemplateBinding/1.inputIndex selects exactly one inputCatalog.items[index] whose payload.kind=template; pin is byte-equal to that item's pin. profileId/profileVersion select the accepted decoder that successfully parsed those exact bytes; profile mismatch or an unavailable decoder is template unavailable, never a fallback. No second template registry or filename lookup participates.

For a nonnull route, steps are nonempty and numbered continuously from zero. Every step's input/output profile is accepted by that exact provider/version. The terminal output profile equals the target: html/pdf/docx/odt/xlsx/ods use target.profileId; csv_utf8 uses "text/csv-utf8/1"; tsv_utf8 uses "text/tsv-utf8/1". routeBinding.profileId/profileVersion identifies the accepted route profile that contains this exact terminal chain. A target/terminal mismatch rejects preparation rather than choosing another route.

For document rendering, documentSnapshotPin selects exact D2-Document-Snapshot/3 bytes for ownerNodeRef and sourceObservation. The snapshot evaluation root SourceObservation and ManagedDocumentSemanticQualification.sourceObservation are byte-equal to sourceObservation, document_format is current, and every snapshot include/environment/dependency is validated under §4.4 at the final export read barrier. presentation is the exact D8PresentationDecision/1 used to stage output. An explicit/no-body/conflict-fallback decision contains no Workspace policy pin; workspace_default contains the exact current D8 policy binding consumed. Later source/include/policy changes never mutate or rerender an already prepared plan; a fresh prepare uses fresh current bindings.

Define Pins(X) as the recursively reached PinRef/2 members of the closed typed value X, following only members whose schema is PinRef/2 (or a closed type containing such members), never text/digests/handles. For a recorded ExportPlan/3, do not traverse the plan's evidencePins member itself while deriving its exact /3 evidencePins. The recorded /3 evidencePins remains exactly the pinToken-sorted/unique union of the following predecessor members. Fresh ExportPlan/4 inherits the same recursive PinRef rule but extends the union only through the explicitly typed projection and viewRenderBinding members listed in §6.6:

```text
Pins(inputCatalog) +
Pins(documentRenderBinding) +
Pins(templateBinding) +
Pins(routeBinding) +
Pins(styleBundles) +
Pins(dependencyProof) +
Pins(observationProof) +
Pins(stagedOutputs) +
recoveryPins
```

recoveryPins is itself pinToken-sorted/unique and contains only original route/rule/destination/unknown-publication pins that the retained fixed-parent recovery contract actually requires and that are not otherwise reachable above. It is not an extension point. Omitting a reachable typed pin, adding an unrelated pin, or duplicating semantic evidence under a different pin makes the Plan invalid. This uniquely closes FC4A-EXP-05 without making evidencePins another authority.

### 6.5.1 Native table → Office SET/COLUMN naming

Native table dataset naming is derived only from the authorized D2TableBlock/3 product projection and the Office template; ordinary Nodes gain no export configuration. Text comparison uses exact Unicode scalar sequences with normalization=none, case-sensitive and whitespace-preserving. CJK, RTL and combining-character text therefore has one host-independent comparison rule.

For each physical table column, construct headerPath in head-row source order. A head cell contributes its semantic inline text once when its [columnStart,columnStart+colspan) covers that column; rowspan does not duplicate the same cell on later logical rows. Empty header cells contribute the empty string. The column leaf is the last segment, or empty when there is no head segment. Table title is its complete semantic title text or null.

The selector is the shortest unique qualifier in this exact sequence:

1. leaf text alone across candidate tables;
2. shortest suffix of headerPath ending at that leaf;
3. the same header suffix plus exact table title;
4. the same facts plus zero-based table occurrence among byte-equal titled/path candidates and, if needed, zero-based column occurrence among byte-equal full header paths in that table.

At each level: zero matches is mapping_required; one is selected; more than one proceeds to the next level. After the final occurrence level, anything other than one is ambiguous_binding. No first/last winner, source suffix invention, current UI order, filename, rowHandle, path guess or author-source edit is permitted. All members of one repeat SET must resolve to the same table/rowset.

SET/COLUMN external names remain the existing ASCII [a-z][a-z0-9_]{0,63}. The unqualified native-table SET is "native_table". A leaf-only COLUMN that already matches that ASCII grammar uses the exact leaf when unique. Every qualified or non-ASCII selector uses a deterministic ASCII token:

```text
SET    = "nt_" + base32hex_lower(SHA-256(
           UTF8("D9-Native-Table-Set/1") || NUL ||
           D3-CJ/3(tableQualifier)))
COLUMN = "nc_" + base32hex_lower(SHA-256(
           UTF8("D9-Native-Table-Column/1") || NUL ||
           D3-CJ/3({tableQualifier,columnQualifier})))
```

base32hex_lower is the 52-character lowercase RFC4648 base32hex encoding of the full 256-bit digest with no padding, so both names fit the 63-byte grammar. Plan stores D9NativeTableTokenBinding/1, so the token is never used as identity or inverted by guess; Core recomputes candidate tokens from the current product projection and requires the stored selector to be the unique match. A digest collision between non-byte-equal selectors is ambiguous_binding/integrity failure, not equality.

Adding a later same-name column/table can make an earlier short selector non-unique; a new prepare must then fail ambiguous_binding until the template contains the newly required qualified token. The old prepared ExportPlan remains immutable because it already froze its table projection, selector and staged bytes.

ExportInputCatalog/2, ExportContentSelection/1, ExportProjection/1, and ExportLossReport/1 retain their fixed-parent closed shapes and semantics. D9NativeTableTokenBinding/1 records how the existing dataset SET/COLUMN names were derived; it does not add a second dataset schema. Projection origins for every native-table dataset column/cell must point to the same inputIndex/tableLocator selected by its binding, with table grid facts taken from D2TableCell/3.

D9ExportConfirmation/1 remains the exact confirmation for a recorded ExportPlan/3: it never changes catalog, projection, route, target, destination, generation policy, report, budget, presentation or staged bytes. Fresh ExportPlan/4 uses D9ExportConfirmation/2 from §6.6, which inherits that immutability and additionally freezes the new Annotation/View selection and renderer members. lossChoices keep the same lossKey sort/uniqueness and complete requires_choice/blocking coverage. PublicationReceipt/3 byte-matches its protected /3 Plan and confirmation; PublicationReceipt/4 byte-matches the protected /4 Plan and Confirmation/2 and records actual published output digests.

Recorded ExportPlan/3 uses only d9_export_plan/3 tokens and recorded PublicationReceipt/3 uses d9_publication/3. Fresh unseen ExportPlan/4 uses d9_export_plan/4, fresh external publication uses d9_publication/4, and print-only delivery uses d9_print/1. Lookup/inspect/confirmation/unknown recovery dispatches token tag before strict record.version decoding. ExportPlan/1-/2-/3 and PublicationReceipt/1-/2-/3 keep their recorded tags, exact bytes, pins, permissions, confirmation, unknown-publication state and recovery; no recorded token is repinned or re-encoded into /4.


## 6.6 Fresh-current D9 export successor /4

The /3 export family above is frozen historical/current-recovery input after this successor is introduced; it is never widened in place. Fresh unseen export uses the following /4 family. All unchanged nested types keep their existing decoder; the new versions exist only where the closed member/union set changes.

~~~text
D9ExportPlanToken/4 := D6 Token tagged "d9_export_plan/4"
D9PublicationToken/4 := D6 Token tagged "d9_publication/4"
D9PrintToken/1 := D6 Token tagged "d9_print/1"

D9ExportInputDomain/2 =
  "document"|"native_table"|"node_collection"|"query_rows"|"query_json"|"resource"
  | "annotation"|"view"

D9AnnotationContentInput/1 = {
  kind:"annotation_content",
  annotationRef:AnnotationRef,
  sourceObservation:SourceObservation/1,
  annotationRevisionToken:AnnotationRevisionToken/1,
  record:PortableAnnotationRecord/4,
  recordPin:PinRef/2,
  body:D8AnnotationBodyRead/1,
  targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable"
}

ExportInputCatalog/3 = {
  version:3,
  items:[{index:Counter,label:text,payload:D9ExportCatalogPayload/3}...]
}

D9ExportCatalogPayload/3 =
    {kind:"document",sourceVersion:SourceVersion/2,observation:SourceObservation/1,pin:PinRef/2}
  | {kind:"resource",sourceVersion:SourceVersion/2,observation:SourceObservation/1,pin:PinRef/2}
  | {kind:"field",ownerVersion:SourceVersion/2,ownerObservation:SourceObservation/1,
     fieldId:FieldId,entries:[Entry/1...]}
  | {kind:"annotation_index",ownerVersion:SourceVersion/2,
     targets:[D9EntityVersionAddress/2...]}
  | D9AnnotationContentInput/1
  | {kind:"template",origin:D9ExportTemplateOrigin/1,pin:PinRef/2}
  | {kind:"query_result",result:D7ResultPin}


D9EntityVersionAddress/2 = {
  ref:EntityRef,
  sourceVersion:SourceVersion/2
}

D9ExportTemplateOrigin/1 =
    {kind:"artifact",source:SourceArtifact}
  | {kind:"resource",sourceVersion:SourceVersion/2,observation:SourceObservation/1}
  | {kind:"route_asset",routeRevision:Token,assetId:text,assetVersion:text}

D9ExportBindingProjection/1 = {
  path:text,
  value:RenderSnapshot,
  origins:[ExportInputLocation/2...]
}

D9ExportCellProjection/1 = {
  value:RenderSnapshot|null,
  origins:[ExportInputLocation/2...]
}

D9ExportColumnProjection/1 = {
  name:text,
  valueKind:text,
  nullable:Boolean
}

D9ExportRowProjection/1 = {
  cells:[D9ExportCellProjection/1...]
}

D9ExportDatasetProjection/1 = {
  name:text,
  columns:[D9ExportColumnProjection/1...],
  rows:[D9ExportRowProjection/1...]
}

D9AnnotationBackupFile/1 = {
  format:"weftext.annotation-backup",
  version:1,
  records:[PortableAnnotationRecord/4...]
}

D9AnnotationSelection/1 = {
  inputIndex:Counter,
  mode:"portable_backup"|"review_bundle_r6",
  includeSourceHistory:Boolean,
  includeTargetContext:Boolean
}

ExportContentSelection/2 = {
  version:2,
  bodyInput:Counter|null,
  bibliographyInput:Counter|null,
  annotationInputs:[D9AnnotationSelection/1...],
  viewInput:Counter|null
}

D9AnnotationDisclosureProjection/1 =
    {state:"not_requested"}
  | {state:"unavailable"}
  | {state:"disclosed",
     fragments:[{value:RenderSnapshot,origins:[ExportInputLocation/2...]}...]}

D9AnnotationExportProjection/1 =
    {kind:"portable_backup",inputIndex:Counter,recordPin:PinRef/2}
  | {kind:"review_bundle_r6",inputIndex:Counter,
     purpose:"comment"|"mark"|"suggestion",
     semanticBody:text|null,
     appearance:AnnotationAppearance/1|null,
     labels:[text...],
     reviewState:"open"|"resolved"|"not_applicable",
     suggestion:Suggestion/3|null,
     replyTo:AnnotationRef|null,
     creator:AnnotationActorSnapshot/1,
     authoredAt:AnnotationTimeSnapshot/2,
     lastEditor:AnnotationActorSnapshot/1,
     editedAt:AnnotationTimeSnapshot/2,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     sourceHistory:D9AnnotationDisclosureProjection/1,
     targetContext:D9AnnotationDisclosureProjection/1}

D9ViewAssetBinding/1 = {
  role:"font"|"color_profile"|"page_profile"|"accessibility_profile",
  assetId:text,
  assetVersion:text,
  pin:PinRef/2
}

D9ViewRendererBinding/1 = {
  rendererId:text,
  rendererVersion:text,
  profileId:text,
  profileVersion:text,
  layout:"metric"|"bar"|"line"|"scatter"|"pie"|"heatmap",
  targetKind:"docx"|"xlsx"|"pdf"|"svg"|"png"|"print",
  accessibilityProfile:"same-data-table-alt-text/1",
  assets:[D9ViewAssetBinding/1...],
  evidencePins:[PinRef/2...]
}

D9ViewOutputScope/1 := "complete_data"

D9ViewRenderBinding/1 = {
  resultInput:Counter,
  querySemanticSha256:"sha256:<64 lowercase hex>",
  snapshotResultSha256:"sha256:<64 lowercase hex>",
  resultEpoch:Token,
  authorizationGeneration:Token,
  viewSpec:ViewSpec/1,
  viewSpecSha256:"sha256:<64 lowercase hex>",
  renderer:D9ViewRendererBinding/1,
  outputScope:D9ViewOutputScope/1
}

D9ViewExportProjection/1 = {
  resultInput:Counter,
  viewSpecSha256:"sha256:<64 lowercase hex>",
  outputScope:D9ViewOutputScope/1,
  accessibility:"same_data_table_required"
}

ExportProjection/2 = {
  version:2,
  bindings:[D9ExportBindingProjection/1...],
  datasets:[D9ExportDatasetProjection/1...],
  annotations:[D9AnnotationExportProjection/1...],
  view:D9ViewExportProjection/1|null
}

ExportInputLocation/2 =
    {kind:"input",inputIndex:Counter}
  | {kind:"source_range",inputIndex:Counter,start:Counter,end:Counter}
  | {kind:"annotation",inputIndex:Counter,targetIndex:Counter}
  | {kind:"annotation_record",inputIndex:Counter}
  | {kind:"annotation_body",inputIndex:Counter}
  | {kind:"annotation_disclosure",inputIndex:Counter,
     scope:"source_history"|"target_context",fragment:Counter}
  | {kind:"query_cell",inputIndex:Counter,table:"rows"|"nodes"|"edges",
     row:Counter,column:Counter}
  | {kind:"query_scalar",inputIndex:Counter}
  | {kind:"template_range",inputIndex:Counter,part:text,
     elementPath:[Counter...],start:Counter,end:Counter}
  | {kind:"view",inputIndex:Counter,
     aspect:"chart"|"accessible_table"|"font"|"color"|"page"|"alt_text"}

ExportLossLocation/2 =
    ExportInputLocation/2
  | {kind:"binding",path:text}
  | {kind:"dataset_cell",set:text,row:Counter,column:Counter}
  | {kind:"block",path:text,blockIndex:Counter}

ExportLossReport/2 = {
  format:"weftext.export-loss",version:2,
  planToken:D9ExportPlanToken/4,
  inputs:[{index:Counter,label:text,kind:text}...],
  items:[{lossKey:Counter,feature:text,locations:[ExportLossLocation/2...],
          effect:text,severity:"notice"|"requires_choice"|"blocking",
          allowedChoices:["accept_loss"|"reject"...]}...]
}

D9ExportConfirmation/2 = {
  version:2,
  planToken:D9ExportPlanToken/4,
  lossChoices:[D9ExportLossChoice/1...]
}

D9ExportTarget/2 =
    {kind:"asciidoc_source"}
  | {kind:"resource_exact"}
  | {kind:"annotation_backup"}
  | {kind:"html",profileId:text}
  | {kind:"pdf",profileId:text}
  | {kind:"docx",profileId:text}
  | {kind:"odt",profileId:text}
  | {kind:"csv_utf8"}
  | {kind:"tsv_utf8"}
  | {kind:"xlsx",profileId:text}
  | {kind:"ods",profileId:text}
  | {kind:"query_json"}
  | {kind:"svg",profileId:text}
  | {kind:"png",profileId:text}
  | {kind:"print",profileId:text}

D9ExportDestinationIntent/2 =
    {kind:"external_bundle",destinationHandle:Token,basename:text,createOnly:true}
  | {kind:"server_download",downloadToken:Token,basename:text}
  | {kind:"resource_handoff",ownerNodeRef:NodeRef,resourceName:text}
  | {kind:"print",printIntentToken:Token}

D9ExportBundleManifest/2 = {
  format:"weftext.export-bundle",version:2,
  planToken:D9ExportPlanToken/4,
  dataFiles:[D9PublishedOutput/1...],
  reportFile:D9PublishedOutput/1
}

ExportPlan/4 = {
  kind:"d9_export_plan",version:4,
  planToken:D9ExportPlanToken/4,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,authorizationGeneration:Token,
  inputDomain:D9ExportInputDomain/2,
  inputCatalog:ExportInputCatalog/3,
  contentSelection:ExportContentSelection/2,
  projection:ExportProjection/2,
  documentRenderBinding:D9DocumentRenderBinding/1|null,
  viewRenderBinding:D9ViewRenderBinding/1|null,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  target:D9ExportTarget/2,
  initialLossReport:ExportLossReport/2,
  outputBudget:BudgetBinding/1,
  destination:D9ExportDestinationIntent/2,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  recoveryPins:[PinRef/2...],
  evidencePins:[PinRef/2...],
  stagedOutputs:[D9ExportStagedOutput/1...]
}

PublicationReceipt/4 = {
  kind:"d9_publication_receipt",version:4,
  publicationToken:D9PublicationToken/4,
  planToken:D9ExportPlanToken/4,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  outputs:[D9PublishedOutput/1...],
  lossReport:ExportLossReport/2,
  lossChoices:[D9ExportLossChoice/1...],
  target:D9ExportTarget/2,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  viewRenderBinding:D9ViewRenderBinding/1|null,
  presentation:D8PresentationDecision/1|null,
  destinationDisplay:text
}

D9PrintReceipt/1 = {
  kind:"d9_print_receipt",version:1,
  printToken:D9PrintToken/1,
  planToken:D9ExportPlanToken/4,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  output:D9PublishedOutput/1,
  lossReport:ExportLossReport/2,
  lossChoices:[D9ExportLossChoice/1...],
  viewRenderBinding:D9ViewRenderBinding/1,
  destinationDisplay:text
}
~~~

### 6.6.1 Plan/4 relational admission and canonical collections

The /4 JSON shapes above are closed, and the following cross-field relations are also part of strict admission. Complete evidencePins coverage cannot substitute for these relations.

Admission order is fixed: (1) strict closed/version/union/member/null decode; (2) verify every frozen set-like array is already canonical and duplicate/conflict free; (3) validate catalog indices and payload kinds; (4) validate selection-to-projection cardinality and mode relations; (5) validate View result/ViewSpec/renderer/target/destination/receipt relations; (6) validate current authorization, Observation/revision/result epoch and typed pins/proofs; (7) run the existing Annotation or D7 View semantic/currentness gate in its owner-defined order; (8) validate route/profile/assets, budget and loss; then and only then freeze staged bytes. A relation failure rejects the candidate. It is never repaired by a supplemental read, Query rerun, hidden arm deletion, reordering of protected bytes, renderer fallback or projection rewrite.

Cross-domain exclusion is mandatory: when inputDomain is not annotation, contentSelection.annotationInputs and projection.annotations must both be empty; when inputDomain is not view, contentSelection.viewInput, viewRenderBinding and projection.view must all be null. A catalog item does not itself grant permission to select or export it, and no unused arm may be consumed via an unrelated projection. These exclusions are checked at relational admission before any output preparation.

For inputDomain=annotation:
- annotationInputs is nonempty, bodyInput=null, bibliographyInput=null, viewInput=null, viewRenderBinding=null, projection.view=null, and documentRenderBinding=null.
- annotationInputs is a set-like array keyed by inputIndex, sorted by ascending Counter and unique. One catalog input may appear at most once in the Plan and therefore has exactly one mode. Mixed portable_backup and review_bundle_r6 modes in one Plan are rejected; use separate Plans.
- Each selected inputIndex names exactly one inputCatalog.items[index] whose payload is annotation_content.
- projection.annotations has exactly the same cardinality and ascending inputIndex sequence as annotationInputs. At each position, portable_backup maps only to a portable_backup projection with byte-equal inputIndex and recordPin; review_bundle_r6 maps only to a review_bundle_r6 projection with the same inputIndex. A selected input without a projection, an unselected projection, duplicate index, cross-mode projection, or sequence [0,1,0] is invalid even when every referenced catalog item is individually authorized.
- A portable_backup selection requires includeSourceHistory=false and includeTargetContext=false; both flags are inapplicable and true is rejected rather than ignored. target.kind must be annotation_backup and generationPolicy.kind must be none. Conversely, a review_bundle_r6 selection cannot use annotation_backup as its target; it requires a named, accepted rendered Review Bundle target/profile and cannot write a backup file under review semantics.
- For review_bundle_r6, includeSourceHistory=false requires sourceHistory.state=not_requested and true requires disclosed or unavailable; the same rule applies independently to includeTargetContext and targetContext. Requested-but-denied context is unavailable, never not_requested or empty facts.
- D9AnnotationDisclosureProjection/1.fragments is an ordered sequence preserving the authorized producer/context order. state=disclosed has one or more fragments. Fragment order is never sorted. Each fragment origins array is a nonempty set-like array sorted/unique by canonical D3-CJ/3(ExportInputLocation/2) bytes; duplicate origins reject.

For inputDomain=view:
- bodyInput=null, bibliographyInput=null, annotationInputs=[], viewInput is nonnull, documentRenderBinding=null, viewRenderBinding is nonnull, and projection.view is nonnull. Document export remains a separate input-domain path and may continue to use D9DocumentRenderBinding/1.presentation; D8 document presentation is not a View data-scope control.
- contentSelection.viewInput, viewRenderBinding.resultInput, and projection.view.resultInput are byte-equal and select exactly one query_result catalog item. All hash/epoch/auth fields in the binding derive from that exact selected D7ResultPin.
- viewRenderBinding.renderer.layout is byte-equal to viewRenderBinding.viewSpec.layout.
- viewRenderBinding.outputScope and projection.view.outputScope are both exactly complete_data. This first profile exports every series/panel/item represented by the complete authorized D7 result and ViewSpec. Device-local legend hide/show state is not author data and cannot change export scope. The same-data accessible table covers the identical complete-data scope. Supporting current_display later requires a versioned successor with stable hidden-series keys and a qualified local-state source; it cannot be added as a Boolean or inferred from absence.
- The outer target.kind is exactly one of docx|xlsx|pdf|svg|png|print, equals renderer.targetKind, and its profileId is byte-equal to renderer.profileId. target.kind=print iff destination.kind=print; every non-print View target rejects a print destination.
- renderer.assets is a set-like array sorted by (role rank font<color_profile<page_profile<accessibility_profile, UTF8(assetId), UTF8(assetVersion), pin.pinToken) and unique. The same (role,assetId) with non-byte-equal version/pin is a conflict. Multiple font/color assets with different assetId are allowed. The array may be empty only when the accepted renderer/profile proves it consumes no external asset of these roles.
- renderer.evidencePins is a set-like array sorted/unique by pinToken; it contains exactly the installation/profile evidence required by that renderer route and no unrelated pin.
- Current D9ViewRendererBinding/1.layout remains the closed six-member union. A valid D7 network View therefore always returns renderer_unavailable on this current D9 route. Installing a profile cannot expand the union; network graphics require an actual future versioned renderer/schema successor.

Delivery/receipt compatibility is also closed:
- destination.kind=external_bundle may produce PublicationReceipt/4; repeated Plan members, including viewRenderBinding, are byte-equal. PublicationReceipt/4.presentation is byte-equal to documentRenderBinding.presentation when that document binding exists, otherwise it is null; it never carries View output scope.
- destination.kind=print produces only D9PrintReceipt/1; its viewRenderBinding is byte-equal to the Plan and its target is the Plan print target.
- destination.kind=resource_handoff produces only the existing separate D7/D3 author result over exact staged bytes; no PublicationReceipt/4 or D9PrintReceipt/1 is fabricated.
- destination.kind=server_download uses the existing delivery/state result and produces neither portable publication nor print receipt.

Canonicalization happens exactly once before Plan/4 freeze. A frozen, received, inspect, confirmation, saved/planned/unknown or recovery record must already satisfy these orders and relations; noncanonical arrays or relational mismatch reject without read-time sorting/repair. Existing D7 row order, Annotation disclosure-fragment order and other owner-defined ordered sequences are preserved and are not globally sorted.

D9ViewRenderBinding/1 and Plan/4 remain design-candidate types with product execution UNRUN; this correction changes no deployed/recorded View-binding bytes. Once a /4 family is actually accepted/deployed, adding another View output scope or renderer-layout member requires a versioned successor rather than widening /1 in place.

ExportInputCatalog/3 keeps every /2 arm byte-for-byte and adds only annotation_content. That arm is formed from one actual current D8AnnotationReadResponse/1: annotationRef, sourceObservation, annotationRevisionToken, value, body, and targetResolution are byte-equal to that read. record is exactly the PortableAnnotationRecord/4 formed from those fields and recordPin selects exactly D3-CJ/3(record) bytes under the existing PinRef/2 integrity rules. The Plan dependencyProof and observationProof cover the same-cut Annotation read and any separately authorized context reads. The final export barrier rechecks both sourceObservation and annotationRevisionToken; equal body text cannot substitute for a changed revision.

annotation_index remains omission-directory evidence only and can never populate annotation_content, annotationInputs, or an Annotation body/context projection. Portable backup requires inputDomain=annotation, target.kind=annotation_backup, one or more mode=portable_backup selections, generationPolicy=none, and generates exactly D9AnnotationBackupFile/1: records are sorted/unique by complete canonical AnnotationRef bytes, every record is byte-equal to the D3-CJ/3 value selected by its chosen recordPin, and the backup file is the canonical UTF-8 D3-CJ/3(D9AnnotationBackupFile/1) bytes. It serializes no current permission, SourceObservation capability, revision-signing capability, PAB, or ActionEvidence. Review Bundle requires mode=review_bundle_r6 and uses only the already-produced D8AnnotationBodyRead/1: valid uses its R6 semantic text, absent uses null, and invalid is renderer unavailable rather than a second parse. Purpose, appearance, labels, reviewState, suggestion, reply and attribution come from the same complete Value/4.

Annotation body/attribution and target/source context are independently qualified. includeSourceHistory and includeTargetContext select an attempt, not permission. A hidden/unavailable target yields targetContext.state=unavailable without suppressing an otherwise legal Review Bundle body, attribution or reply. A disclosed fragment must derive from separately authorized catalog inputs in the same cut and carry nonempty ExportInputLocation/2 origins. Missing context is represented by the corresponding projection state and complete loss item; it is never replaced by display labels, annotation_index, or guessed target bytes.

A View export has inputDomain=view, exactly one viewInput selecting a query_result catalog item, no Annotation selections, and one nonnull viewRenderBinding. Before Plan/4 freeze Core runs the existing D7 View §7 order over that exact complete D7ResultPin, beginning with closed/static ViewSpec decoding. Layouts outside the current D7 ViewSpec/1 closed set, including deferred tree/treemap/sunburst, Gantt and boxplot/quantile, fail with unsupported_layout before D9 renderer selection. For a valid current D7 layout, this first closed D9 chart profile renders only metric|bar|line|scatter|pie|heatmap; another valid D7 layout such as network, or a six-chart layout whose named backend/profile is unavailable, returns renderer_unavailable for this route. Neither case converts rows into a chart substitute. The renderer never re-queries, enriches, sorts, aggregates, bins, samples or changes ViewSpec semantics.

querySemanticSha256 is SHA-256 of D3-CJ/3(the exact D7 SemanticStateKey) and snapshotResultSha256 is SHA-256 of D3-CJ/3(the exact D7 SnapshotResultKey) retained by the selected D7ResultPin. resultEpoch and authorizationGeneration are byte-equal to that same result evidence; viewSpecSha256 is SHA-256 of D3-CJ/3(the exact ViewSpec/1). These hashes are frozen cross-checks, not identities or new cache authorities. The selected D7ResultPin remains the complete result/cut/dependency authority.

A View renderer binding is valid only for one named installed renderer/profile/version and one exact target kind. Its canonical asset/evidence collections and target/profile relation are defined by §6.6.1. The fixed complete_data scope requires the same complete data table plus title/description alt-text semantics, query/panel order, CJK/RTL preservation and non-color-only meaning. PDF, SVG, PNG and print profiles may render the six layouts directly. DOCX/XLSX profiles may do so only when their named profile proves the same View semantics; if an Office template is used, §6.5.1 visible-template authority and all §14 rules still apply. Unsupported backend/profile/layout combinations are stable unavailable results, never silent data-table substitution.

For Plan/4, `evidencePins` is exactly the pinToken-sorted/unique recursive union of `Pins(inputCatalog)`, `Pins(projection)`, `Pins(documentRenderBinding)`, `Pins(viewRenderBinding)`, `Pins(templateBinding)`, `Pins(routeBinding)`, `Pins(styleBundles)`, `Pins(dependencyProof)`, `Pins(observationProof)`, `Pins(stagedOutputs)`, and `recoveryPins`, excluding the Plan's own `evidencePins` member. Thus Annotation record/context pins and every View renderer/asset/result dependency actually consumed are covered by the existing one-union rule.

D9ExportConfirmation/2 cannot alter the catalog, Annotation/View selection, projection, ViewSpec, renderer/profile/assets, route/template, target, destination, loss report or staged bytes. External publication remains create-only and produces PublicationReceipt/4; Resource handoff remains the separate original D7/D3 author path over exact staged bytes. A print destination uses the same frozen/staged/confirmed Plan and produces only D9PrintReceipt/1; it grants no author capability and cannot claim external publication. Current Plan/4 bundles use D9ExportBundleManifest/2.

Real ExportPlan/1-/2-/3, PublicationReceipt/1-/2-/3, their plan/publication token tags, ExportInputCatalog/2, ExportContentSelection/1, ExportProjection/1, ExportLossReport/1, D9ExportConfirmation/1, bundle manifest/1, pins, confirmation bytes and saved/planned/unknown recovery remain exact historical decoders. No /3 record is repinned, re-encoded, sorted, renamed or upgraded into /4.


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

For current wire13 D3 mutation, the Value/4 base decoder keeps the inherited logical slots annotation_target ordinal0 and annotation_reply ordinal1. target is always a reference slot. replyTo is a reference slot when nonnull; an identity-preserving existing-Annotation reply change uses the inherited structural S form and cannot also produce a reply reference result. All other Value/4 members are nonreference bytes. Every changed current Value/4 gets one new final AnnotationRevisionToken/1, and all target/reply toSource addresses, annotation_reply_change evidence, SourceRevisionPlan/result pin, source change and receipt use that same token. Caller AnnotationEditableProposal/1 bytes are never compared directly with AnnotationEditableValue/1. Core first expands the proposal through the operation-class gate into one complete candidate Value/4 carrying before attribution; canonical candidate Value/4 equal to canonical before Value/4 is the no-op and keeps the complete Suggestion evidence, token/SourceVersion/H/attribution with no SourceRevisionPlan. Only a differing candidate receives fresh lastEditor/editedAt and a fresh final token.

Current payloadBindings with payloadKind=annotation_value dispatch by actual request family: wire13 current mutation strict-decodes Value/4, while genuine historical wire9-12 plans keep their Value/3 decoder. D3-Symbolic-Result/9 framing is unchanged; its annotation base bytes and slot spans are computed from the decoder selected by that request family. This does not widen the historical decoder.

The current Portable Metadata path does not materialize D2 Annotation-v2 outer wire as author state. Historical D2 v2 annotation snapshots, Value/3, plain_text body, replace_plain_text suggestion and targetStatus resolved/stale retain their real historical decoder/recovery only. Current body bytes are AnnotationInlineBody/1 and use exactly the single AnnotationInlineProfile/1; no plain-text compatibility fallback or second parser exists.

## 7.1 Caller proposal and Node-local physical aggregate

```text
SuggestionAuthorProposal/1 = {
  kind:"replace"|"delete"|"insert",
  replacementSource:null|text
}

AnnotationEditableProposal/1 = {
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:SuggestionAuthorProposal/1|null
}

AnnotationAggregate/1 = {
  format:"weftext.annotations",version:1,
  ownerNodeRef:NodeRef,
  records:[PortableAnnotationRecord/4...]
}

AnnotationAggregateObservation/1 = {
  kind:"d6_annotation_aggregate_observation",version:1,
  workspaceRef:WorkspaceRef,
  ownerNodeRef:NodeRef,
  observerDomain:CommitDomain/2,
  fileObjectBinding:FileObjectBinding/1,
  aggregateBytesPin:PinRef/2|null
}

AnnotationAggregateInstall/1 = {
  kind:"d6_annotation_aggregate_install",version:1,
  ownerNodeRef:NodeRef,
  before:AnnotationAggregateObservation/1,
  after:"absent"|PinRef/2,
  installCapability:FileInstallCapability/2,
  changedAnnotationRefs:[AnnotationRef...]
}
```

SuggestionAuthorProposal/1 contains only author-proposed kind/replacementSource: replace requires replacementSource (which may be empty), delete requires null, and insert requires nonempty replacementSource. state, confirmation, targetBasisSha256, expectedText, and pointAffinity are never caller proposal fields. AnnotationEditableProposal/1 keeps the same purpose/reply/body/appearance/labels/review invariants as AnnotationEditableValue/1; the Core operation-class gate produces the controlled Suggestion/3 before→after. The gate has five derived current-state classes rather than treating null as absence: fresh create=`absent_annotation`; existing suggestion=null=`no_suggestion`; pending=`pending_confirmed|pending_needs_reconfirmation`; terminal=`terminal_accepted|terminal_rejected`. interactive_create admits legal no-suggestion comment/mark/reply and legal root suggestion. ordinary_edit admits no_suggestion→no_suggestion, no_suggestion→pending+needs_reconfirmation after real target qualification, pending→pending, pending→no_suggestion, and terminal→the same terminal only. manual_reattach admits no_suggestion→no_suggestion or pending→pending+needs_reconfirmation and never combines reattach with category conversion. A no-suggestion edit with unchanged target and a pending→no_suggestion clear do not acquire target-source-read authority; conversions/operations that consume target content use only their named qualification/read step. After this gate Core expands a complete candidate Value/4 with before attribution and compares that canonical Value/4 to before; Proposal bytes are never a no-op comparator.

The current physical bytes of `weftext.annotations.json` are exactly D3-CJ/3(the complete AnnotationAggregate/1), with no BOM, trailing newline, or second envelope. records is nonempty, sorted by complete canonical AnnotationRef bytes and unique; every annotationRef.owner equals ownerNodeRef, every nonnull replyTo has that owner, and the complete records reply graph is acyclic. The unique physical representation of an empty set is an absent sidecar. Unknown/missing members, duplicate JSON keys, unknown version, owner mismatch, duplicate Ref, or reply cycle fail the whole aggregate strict decode; a normal current read never partially trusts records that happen to look valid.

AnnotationAggregateObservation/1 is a physical-file observation, not an Annotation SourceVersion/SourceObservation, revision token, or identity. `FileObjectBinding/1` and `FileInstallCapability/2` are reused **byte-for-byte from fixed D6 Control Interfaces §2**, not versioned here: absent binding=`{kind:"absent",backendToken,relativePath,observationEpoch,parentGenerationToken}`; present binding=`{kind:"present",backendToken,relativePath,observationEpoch,objectGenerationToken,byteLength,sha256}`; capabilities remain create_only(parentGenerationToken), conditional_replace(expectedObjectGenerationToken), exclusive_write_window(windowToken,expectedObjectGenerationToken), and observed_replace(observedObjectGenerationToken). PortableRelativePath, Token/Counter, 64-lowercase-hex, containment, and continuity retain that owner's exact rules. fileObjectBinding.kind=absent requires aggregateBytesPin=null; kind=present requires a PinRef/2 with payloadKind=portable_metadata whose bytes are the complete physical JSON above. A digest alone is never CAS. AnnotationAggregateInstall/1 is only a named file-write coordination binding inside the existing D6 installation plan, not a new ledger/CAS/author store; before is always the fresh Observation, never a bare `"absent"`. A PinRef after selects the complete after aggregate and `"absent"` deletes the last record. installCapability cross-fields exactly with the before FileObjectBinding: absent→create_only with equal parentGenerationToken; present→a D6-permitted replace capability with exact generation/window token. The managed_atomic strong path still accepts only that owner's strict capabilities; observed_replace remains valid only where D6 §4.1 already grants observed_only eligibility and never becomes CAS. changedAnnotationRefs is canonical sorted/unique and exactly the logical record difference between before and after. A DecisionKey has at most one AnnotationAggregateInstall/1 for one Node.

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
  initialPresentationPolicy:D8PresentationPolicyBootstrapInit/1,
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
```

All UUID members use the canonical lowercase D3 UUID decoder. Genesis has exactly two declarations: revision 1 revision-token authorize and revision 2 source-transform authorize, same DecisionKey/activation ChangeId, with rev2 predecessor hashing exact rev1 canonical bytes. Series configurations are unique and sorted by canonical SeriesScope; period bindings are unique and sorted by full NodeRef with exactly one per valid prepared period. The helper members retain the fixed-parent Plan3 field semantics; Profile4 changes only the dual-profile genesis family.

Plan4 is only for unseen fresh create/fork and preserves the original D3 proposal/custody/CAS/P boundary. Ordinary copy is not Plan4. Saved/planned/unknown recovery keeps the actual recorded decoder and bytes; restore/continue/failover do not synthesize Genesis2. No deployment or migration from Plan3 is asserted.

initialPresentationPolicy is mandatory and closes the D8 fresh-target state inside that same Plan4. Its before/proposal WorkspaceRef equals targetWorkspaceRef; before is the protected fresh empty-head observation from §6.4 with stamp.revision=1; proposal is exactly parents=[], revision=1, defaultPresentation=separate. Prepare/planning/staging freeze that helper and one typed presentation_policy_change preview with committed=null under the original create/fork OperationId, planning CAS and DecisionKey. This is not a D8 SetRequest, needs no already-active target policy_admin, creates no second CAS/ledger, and creates no committed policy record/hash/address/pin/outbox/ChangeId before final P.

Only after every original bootstrap final check passes does the same P checked-allocate the one bootstrap ChangeId C. In that atomic P transaction Core constructs the canonical D8WorkspacePresentationPolicy/2 from the frozen proposal+C, computes its fixed-prefix hash, exact recovery portable_metadata pin/address and D8PresentationPolicyOutboxItem/1, commits presentation_policy_change plus the original receipt/effects/ChangeRecord association, advances the D8 head graph from the frozen empty before to heads=[address] with the same epoch and checked stamp revision=2, and commits target activation/custody plus every other bootstrap effect. record.activationChangeId and ChangeRecord.changeId are C and the outbox decisionKey is the original create/fork DecisionKey. A planning loser, abort, failed final check or failed P commit leaves the target unactivated and leaves no partial D8 record/pin/head/outbox. A successful active Plan4 target therefore immediately has one /1 logical current view at revision=1/defaultPresentation=separate. Post-P publication retry uses only the exact retained outbox bytes and normal receiver same-decision/ancestry admission.

Ordinary managed copy is not Plan4 and does not synthesize this initialization. An actual saved/planned/unknown Plan4 restores its exact original initialPresentationPolicy, OperationId/proposal/custody mappings, pins and decoder; it never resamples [] or the current branch. Restore/continue/failover preserve their recorded/current policy state instead of re-running bootstrap. Genuine historical Plan1/Plan3 bytes keep their actual decoder and receive no injected field. This candidate asserts no migration or dual-write for an already active Workspace.

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

Every schema and cross-field rule above has explicit positive/negative design obligations in [ACCEPTANCE.md](ACCEPTANCE.md): 438 core + 322 coordination = 760, all unexecuted. Those row bodies are normative obligations; the count is not a substitute for them.
