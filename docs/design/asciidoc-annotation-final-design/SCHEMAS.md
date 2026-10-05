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

attributeOverrides sort uniquely by lowercase name, extension profiles by profileId, and sourceUnits by logicalPath.value. Set actions require non-null values and unset actions require null. With safe mode below server, an unoverridden user-home comes from ambientUserHome; server/secure use the native "." default. SOURCE_DATE_EPOCH, when present, drives local*/doc* UTC values; otherwise local* uses clockNow and doc* uses inputMtime when available, then clockNow.

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

Per-subject snapshots form an immediate-predecessor chain. Cut heads sort numerically by subjectId and exactly equal the reachable semantic closure. Only references within the model-observation namespace use numeric temporal comparison. operation/inline/value/call references use their own namespace contracts. carrier.entry is a target-array index and is never numerically compared with model observation IDs.

Producer conformance additionally requires, at the real bind callback, that the target carrier has already been appended, entry < targetStream.lengthAtBind, and the producer still holds the exact same Ruby object. The wire decoder can only validate final existence/type/index, not infer cross-stream time. document/0 has the same rule.

Catalog ownership is only parent -> actual Document#register receiver. A temporary overlay must close through a real restore or cleanup delete; the observer never fabricates a restore write.

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

```

## 6.1 PreparedActionBinding/4

```text
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

proposedInputs contains exactly one item and its entityRef equals intent.target.ref. OwnerInputBinding/2 remains unchanged; current ownerKind is d8_edit/3.

## 6.4 ExportPlan/3 / PublicationReceipt/3

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
```

```text
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

ConflictResolutionPolicyDerivedPlan/2 = {
  kind:"policy_bundle",
  selected:WorkspaceAuthorizationBundleAddress/1,
  headEvidence:[PolicyBundleHeadEvidence/2...],
  selectedBundleVersion:1|2,
  selectedBundlePin:PinRef/2,
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
```

FreshDomainAuthorizationSpec/2 has exactly the same two JSON members as historical /1 but profile is the full SealProfileId/1 union. The array is canonical D3-CJ/3 sorted/unique and contains no key material. Current ConflictResolutionInput/3 is a protected owner-descriptor successor only for policy_bundle_choice; it does not version the public request, add a submit path, or change conflict_resolve/policy_admin/disclosure/error order. A genuinely proven saved/planned old owner descriptor retains its recorded decoder and pins.

PolicyBundleHeadEvidence/2 is complete, ChangeId-sorted/unique and byte-equal in head set to expectedKey.heads. version=3 means branchEvidence.completionProofPin strict-decodes ContentCompletionProof/3 and the exact policy after-image strict-decodes WorkspaceAuthorizationBundle/1. version=4 means completionProofPin strict-decodes ContentCompletionProof/4, branchEvidence.changeRecordPin strict-decodes the matching ChangeRecord/1 and exact Notice3/CP4 chain, and the policy after-image strict-decodes the declared Bundle1 or Bundle2 version. In both arms policyBundlePin is present, pins the exact canonical bundle bytes, and WorkspaceAuthorizationBundleAddress/1 authorizationRevision/trustRevision/byteLength/sha256 matches them. CP3+Bundle2, unknown versions, a missing CP4 ChangeRecord, tag/byte mismatch, or decoder fallback is rejected.

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

ConflictResolutionPolicyDerivedPlan/2 freezes the exact per-head proof/bundle dispatch, selected bundle pin, complete carry union, inherited subset, Outcome2 values, and exact result bundle pin/version. resultBundlePin must strict-decode the derived result and reproduce its canonical bytes. The original one planning CAS and final P publish the current policy component through Notice3/CP4/ChangeRecord1; staged authorize_fresh handles become usable only in that same commit. Receiver validation recomputes all head dispatch, Carry1/Carry2 facts, mixed recursive union, PoP/2 and root signatures, exact result bundle, and same-decision conflict-record transition.

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

For a fresh current core_field_member step, D10AuthorPreparationLink/2.preparedBindingToken must select the exact PAB4 whose original request equals link.request. Core saves Link2, that complete PAB4, and the required preview/effect/recovery pins atomically before returning the prepared step or permitting submission. ApprovalUse/2.preparedBindingToken selects that same PAB4. Its preview digest is
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3));
each EffectBytes/3 slot is projected only as {encoding,byteLength,payloadDigest}, never discovered by recursively guessing member names. Fresh current automatic author qualification uses DependencyProof/3, including document_format whenever the managed Document is semantically parsed, and constructs ApprovalUse/2 only after complete EffectManifest/3 / EffectBytes/3 / MutationFootprint validation. Historical Link1/PAB3/ApprovalUse1 saved or planned associations keep their original decoder, bytes, pins, request and OperationId.

For a fresh ScheduleSubscription/2 registration, the current D6 Storage producer creates ScheduleContinuityWitness/2 only in the same configuration transaction that passes the selected source/Field/Registry/current scheduling gates, reserves finite retention, and attaches the actual continuously maintained Core source/control transition producer. initial and checkpoint are the subscription's exact ScheduleRecurrenceEvidence/2, revision=1, consumedTransition=0, and producerEpoch is fresh. Positive current transitions use ScheduleContinuityStep/2 with DependencyProof/3 and the actual ChangeRecord/1, InstallationNotice/3, and ContentCompletionProof/4 pins for new current portable transitions; historical transitions inside retained history keep their original exact decoders. P-only relevant control/rule transitions remain captured from their actual protected before/after state.

When no valid current after evidence can exist, the current producer emits ScheduleContinuityInvalidation/2 instead of fabricating ScheduleRecurrenceEvidence/2. binding_changed requires complete trusted evidence of a selected-business discontinuity; unavailable/unknown after, missing history, unknown decoder, observer/producer gap, or inability to retain a required transition is gap. Invalidation compares the same current witness/registration, atomically advances the next checked transition/revision, keeps the last valid checkpoint, and is permanently non-resetting for that generation. Its artifact pin uses recovery retention and UTF8("D6-Schedule-Invalidation/2") || NUL || D3-CJ/3(complete ScheduleContinuityInvalidation/2). The fixed-parent inbox/capacity/final-counter reservation, authorization, compaction and unrelated-source-availability rules remain unchanged.

A current schedule proof that semantically parses a managed Document must include both source and document_format dependencies. If source/profile bytes are unchanged but format-proof continuity has a gap, the result is gap; a real binding transition is binding_changed even when the final recurrence/range value happens to compare equal. An existing Subscription1 remains a historical retention owner with Witness1/Step1/Invalidation1. It may become a same-generation Subscription2 only through explicit continue plus complete retained history proving no intervening format/rule/business discontinuity and establishing the current Evidence2/Proof3 cut; otherwise replace is required. The bridge preserves old pins and producer evidence and never re-encodes version-1 witness/step/invalidation bytes as version 2.

# 11. Historical dispatch and one authority set

Saved work replays under its original owner/version. Planned work restores the original descriptor/proof/prepared/pins/Notice/install/version basis. Unknown work retains original Approval/Money/claim/external/stop liabilities. Only unseen work uses current successors. Old bytes/pins are never re-encoded merely because they are carried by a mixed wrapper.

All successors continue through the original single DecisionKey, planning CAS, install, P seal, receipt, and outbox. document_format is not a second Document authority; SourceTransform is not source authority; ChangeRecord is not a second portable truth; a D10 version wrapper grants no authority; Derived Index never reconstructs current proof.

# 12. Provider profiles

Provider closed shapes are in SPEC §4. They qualify ecosystem renderers and never alter core-language validity. Mermaid's fixed source is CLI 12.0.0 commit db1ceebbe529d7975474eb0d0e9c23e9dc57cd37 with fixed package.json and package-lock Git blobs. Node/browser/fonts/config still require exact runtime version/digest registration. Without a registered runtime the provider is unavailable; floating semver, host browser/font defaults, or network installation cannot fill the gap.

# 13. Acceptance reference

Every schema and cross-field rule above has explicit positive/negative design obligations in [ACCEPTANCE.md](ACCEPTANCE.md): 437 core + 117 coordination = 554, all unexecuted. Those row bodies are normative obligations; the count is not a substitute for them.
