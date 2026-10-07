---
source_language: zh-CN
translation_of: D9-SCHEMAS.zh-CN.md
translation_status: synced
---

[简体中文](D9-SCHEMAS.zh-CN.md)

# A2 D9 Schemas

Status: **author-resolved-pending-independent-review**. This file currentizes the D9 owner contracts through final FC. It does not duplicate D3/D4/D6/D7 schemas.

## 1. Decoder rules

All D9 JSON objects are closed: duplicate/unknown/missing members, illegal null, wrong union arm, invalid Unicode scalar, wrong canonical ordering or out-of-range Counter reject. Numbers used as protocol Counters are nonnegative canonical integers; source numeric lexemes remain text until their owning conversion rule validates them.

Version dispatch is by actual tag/version before members are interpreted. Historical records never fall through to a current decoder.

## 2. SourceArtifact and artifact wire

`SourceArtifact` exact:
```text
{artifactToken,byteLength,sha256,displayName,originClass}
```
`originClass` is one of local_file/upload/clipboard_plain/generated and is host evidence only.

Pure D9/1 probe/convert/state/cancel objects retain their current owner-afterimage exact members and closed enums. They never carry Workspace/CommitDomain or author capabilities.

## 3. ImportIR/1

`ImportIR/1` exact top level:
```text
{format:"weftext.conversion-ir",version:1,sources,profileId,documents,resources,issues}
```

Keys are continuous local Counter ordinals. `SourceLocation` is exactly bytes/page/cell/part. `Observation` is `{origins,confidence,method}` with nonempty origins, nullable 0..10000 confidence and method extracted/ocr/inferred/user_supplied.

Page, flow and workbook unions remain the exact current D9 owner shapes. Workbook `CellValue` is blank/text/boolean/number-lexeme/error. Formula presence/cache and hidden/merge facts are separate. Unknown fields or unrepresented semantics enter the closed issue/loss process or block coverage; they never hide in free JSON.

## 4. ImportMapping/1 and LossReport/1

`ImportMapping/1` exact `{version:1,documents}`; every IR document has one sorted unique mapping. Current mapping arms are omit/document_node/rows_to_nodes/table_document with the complete owner-defined ranges, title/body/retain-original, Field conversion, formula, hidden policy and destination fields.

`LossReport/1` exact `{version:1,items}`. Each item is `{lossKey,feature,locations,effect,severity,allowedChoices}`; key order is continuous. notice has no choice, requires_choice has accept_loss/reject, blocking has reject only. File locations use Import `SourceLocation`; Node-template losses use `TemplateLossLocation`. The domains never mix.

## 5. ConversionInput/2

Current author preparation binds an immutable `ConversionInput/2` ZIP. Only manifest.json, ordered source/N.bin and ordered resource/N.bin entries are allowed; stored compression, no encryption, no links/duplicates/external references. The manifest has no self-hash.

Part 0 contains the complete mapping/construction projection, route/profile, initial loss/choices, groups/batches and complete proposed object manifest. Raw parts retain exact original bytes. partOrdinal is not a PinRef or authority. Object ordinals are deterministic source-location/role tuples and are not identity.

## 6. TemplateRecipe/2 and TemplateConstructionInput/2

`TemplateRecipe/2` exact:
```text
{format:"weftext.node-template",version:2,parameters,nodes}
```
Each node freezes an exact `TemplateSourceAddress/2`, parentIndex, targetFacets and closed bindings. Slot arms are title/body_text/field_append. No expression or opaque extension member exists.

`TemplateConstruct/2` exact:
```text
{version:2,template,recipe,parameters,resources,destination,externalNodePolicy}
```

`TemplateConstructionInput/2` exact:
```text
{kind:"node_template_construction",version:2,construction,inputPins,
 omittedAnnotations,lossReport,lossChoices,sourceSubjectBindings}
```
Input pins carry actual `SourceVersion/2`, current `SourceObservation/1`, payload kind and `PinRef/2`. Omitted Annotation locations carry the exact current version address but not body. Source-subject bindings are sorted unique and must close exactly over selected sources and original request fresh subjects.

## 7. Current PAB4 ownership

D9 does not define PAB. Current D7 owns `PreparedActionBinding/4`, which binds current action input, concrete proposed inputs, optional built-in D9 construction evidence, `DependencyProof/3`, original request, full preview/effects, pins, audience/budget/fingerprint/currentness.

Fresh D9 author submission uses only the mode-legal shapes of `D3IdentityOperationRequest/13` or the existing D6 request. Historical PAB1/2/3 and wire11/12 are decoder-selected historical records only.

## 8. Office token grammar

Ordinary tokens retain the owner grammar and one-pass/four-brace escape. Simple native-table unique ASCII leaf retains `data.native_table.COLUMN`.

Qualified native-table authoring grammar is:
```text
native-selector := "native.table" table-qual "::column" column-qual
table-qual      := "[]" | "[" json-string "]" [ "#" counter ]
                 | "[null]#" counter
column-qual     := "[" json-string *( "," json-string ) "]" [ "#" counter ]
```

The selector occurs inside the same outer `{{ ... }}`, `{{ ↓ ... }}`, or `{{ → ... }}` token forms. JSON strings are decoded once to Unicode scalars. Counter spelling is canonical decimal. Qualification must be the shortest unique chain allowed by Main §7.

This named revision supersedes fresh authoring of qualified FC hash tokens only. Internal compiled Plan keys may still be `nt_...` / `nc_...`; the exact visible token plus `D9NativeTableSelector/1` is frozen in current Plan binding evidence. Historical hash-authored templates/plans retain their exact decoder.

## 9. Export content and projection

`ExportContentSelection/1` remains exact `{version:1,bodyInput,bibliographyInput}`; each input is null or one exact Document catalog index.

`ExportProjection/1` remains exact `{version:1,bindings,datasets}`. Binding paths are unique; values use the bounded RenderSnapshot union. Dataset names/columns are closed and rows preserve the complete frozen order and bag. Each cell has nonempty origins.

`D7ResultPin` is internal D9 control evidence over a complete D7 TerminalSchema/V result and its producing epoch/auth/cut/dependencies. D9 does not redefine D7 V.

## 10. ExportPlan/3

The exact current Final-FC shape is:
```text
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
```

No namesake abbreviated object is a valid Plan. Every member above is required except the explicitly nullable render/template/route bindings. `planToken` uses only the current `d9_export_plan/3` tag. `inputCatalog` is exactly the retained `ExportInputCatalog/2`, not a renamed D9 wrapper.

`D9ExportInputDomain/1` is exactly `document|native_table|node_collection|query_rows|query_json|resource`. `D9ExportTarget/1` is the closed Final-FC target union: asciidoc_source, resource_exact, html/pdf/docx/odt with profileId, csv_utf8, tsv_utf8, xlsx/ods with profileId, and query_json. Exact-source/resource/query-json plans require `generationPolicy={kind:"none"}` and null document/template/route bindings.

`generationPolicy.render` freezes the closed bindingChoices, missingPolicy, imageSizes, layoutChoices and nativeTableBindings arrays. Native-table binding records remain sorted/unique by exact compiled (setName,columnName), and each freezes the exact `D9NativeTableSelector/1`. This A2 D9 candidate changes only the fresh qualified Office authoring spelling; the Plan3 selector/binding record remains the Final-FC type.

For document rendering, `D9DocumentRenderBinding/1` binds ownerNodeRef, current SourceObservation, exact D2-Document-Snapshot/3 pin, ManagedDocumentSemanticQualification/1 and the exact D8PresentationDecision/1 actually consumed. Route steps are continuous 0..N-1 and each step freezes provider/version/input/output profile/options hash, BudgetBinding/1 and sorted/unique evidencePins. Style bundles are sorted/unique by styleBundleId.

`recoveryPins` contains only the retained original recovery pins not otherwise reachable by the typed Plan. `evidencePins` is exactly the pinToken-sorted/unique recursive typed PinRef union of inputCatalog, documentRenderBinding, templateBinding, routeBinding, styleBundles, dependencyProof, observationProof, stagedOutputs and recoveryPins; it is not a free extension array.

Template paths canonical-sort by exact Unicode scalar sequence, normalization none and case-sensitive. Controlled output names first pass the complete validity/alias/prefix/reserved checks, then stagedOutputs sort by exact stored raw unsigned UTF-8 bytes. All set-like arrays reject duplicates/conflicts. Legal input permutations canonicalize once before freeze; frozen/received/recovery records must already be canonical and are never repaired on read.

## 11. Output names and bundle

`D9ControlledRelativeOutputName/1` preserves exact UTF-8 and consists of nonempty components separated only by '/'. Empty/dot/dotdot components, backslash, controls, forbidden punctuation, trailing space/dot, rooted/drive/UNC and reserved device-name stems reject.

PortableAlias is Unicode 15.1 NFC → full default CaseFolding C/F → NFC per component and is rejection-only. Exact duplicate, equal alias and alias-prefix conflicts reject across the complete bundle. loss-report.json and manifest.json are reserved root members.

Staged output metadata and `PublicationReceipt/3.outputs` are sorted unique by exact stored output name.

## 12. ExportLossReport/1 and confirmation

`ExportLossReport/1` remains:
```text
{format:"weftext.export-loss",version:1,planToken,inputs,items}
```
Locations are the closed ExportInputLocation plus binding/dataset_cell/block projection locations. source_range uses UTF-8 bytes; template_range uses Unicode scalar offsets.

`D9ExportConfirmation/1` exact `{planToken,lossChoices}`. Choices are sorted unique and cover every requires_choice/blocking item as defined by the report matrix. Confirmation never mutates Plan/staged bytes.

## 13. PublicationReceipt/3

The exact current Final-FC shape is:
```text
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

The current token tag is only `d9_publication/3`. Every repeated Plan member is byte-equal to the protected Plan/confirmation, and outputs are the actual published controlled-name/digest set. This receipt proves external publication only; it is never D3 ChangeId/CP or Resource-author success. Historical Plan/Receipt1/2 retain original tags, bytes, names, permissions and unknown-publication recovery even when a historical name would fail the current predicate.

## 14. Query JSON and typed tabular export

query_json is exact `{format:"weftext.query-result-export",version:1,schema,data}`; data shape is determined by TerminalSchema kind and embeds original D7 V values with exact numeric/Unicode semantics. Runtime result/cursor/row handles are absent.

CSV is RFC4180/CRLF with exact selected scalar projection. TSV has no quote escape and rejects tab/CR/LF fields. XLSX/ODS keep text as text; integer/decimal/date/instant numeric/serial coercion requires a named safe profile and otherwise uses exact text or rejects. Dangerous spreadsheet-formula text rejects under the fixed Unicode-15.1 rule.

## 15. WorkerInvocation/1

`WorkerInvocation/1` exact `{version:1,jobToken,step,routeId,routeRevision,profileId,inputs,options,budget}`. Input slots are host assigned. Terminal worker result is the closed ok/failed union; failed code is only unsupported/encrypted/unsafe/malformed/budget/cancelled/internal. Worker output cannot contain author permission, receipt, identity or destination.

## 16. Region

`RegionBody` exact `{version:1,profile,page,rect}`; `d9rg1` is the canonical inner token over that body. page and rect use the current D9 geometry profile. The token is not a Locator. D3 `ResourceRegionLocator/l1` remains the outer authoritative identity/revision binding.

## 16a. Inherited current Annotation Value/4 / R6 types

D9 does not own these schemas, but any D9 content export/copy/import consumer that uses current Annotation content consumes them exactly:

```text
PortableAnnotationRecord/4 = {
  kind:"portable_annotation",version:4,
  annotationRef:AnnotationRef,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4
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

AnnotationInlineBody/1 = {
  format:"asciidoc-inline",version:1,
  languageBaseline:"asciidoctor-ruby/2.0.26",source:text
}
```

The target projection is the closed document/document_element/document_range/resource/resource_region union owned by current D3. Body evaluation uses only the current `AnnotationInlineProfile/1` R6 profile with its commit-qualified Asciidoctor Ruby 2.0.26 baseline and secure no-managed-adapter/no-file/no-network/no-process effects. The existing D9 `annotation_index` export-catalog arm remains omission-directory evidence and cannot substitute for reading a complete Value/4. Historical Value/3/plain_text records retain historical decoding only.

## 17. D9 error

D9/1 and D9/2 use only their owner-defined closed code sets. Current workspace-side availability includes domain/proof/integrity/recovery conditions from the current owner afterimage. D3/D6/D7 errors are never encoded as synthetic D9 success/failure records after their boundary is entered.
