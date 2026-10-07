---
source_language: zh-CN
translation_status: source
---

[English](D9-SCHEMAS.md)

# A2 D9 Schema

状态：**author-resolved-pending-independent-review**。本文把 D9 owner contract currentize 到 final FC，不复制 D3/D4/D6/D7 schema。

## 1. Decoder rules

全部 D9 JSON object 都是封闭对象：出现重复/未知/缺失成员、非法 null、错误的 union arm、非法 Unicode scalar、错误 canonical order 或越界 Counter 时都必须拒绝。protocol Counter 使用非负 canonical integer；source numeric lexeme 在其 owner conversion rule 完成验证前必须保持文本形式。

先按真实 tag/version dispatch，再解释 member。historical record 不 fallback 到 current decoder。

## 2. SourceArtifact 与 artifact wire

`SourceArtifact` exact：
```text
{artifactToken,byteLength,sha256,displayName,originClass}
```
`originClass` 只允许 local_file/upload/clipboard_plain/generated，且只代表 host evidence。

纯 D9/1 的 probe/convert/state/cancel object 保留现任 owner afterimage 中精确的成员与封闭 enum，不携带 Workspace/CommitDomain 或任何 author capability。

## 3. ImportIR/1

`ImportIR/1` exact top level：
```text
{format:"weftext.conversion-ir",version:1,sources,profileId,documents,resources,issues}
```

key 是连续的本地 Counter ordinal。`SourceLocation` 只包含 bytes/page/cell/part。`Observation` 为 `{origins,confidence,method}`：origins 非空，confidence 为 null 或 0..10000，method 只能是 extracted/ocr/inferred/user_supplied。

Page/flow/workbook union 保持现任 D9 owner 的精确 shape。Workbook `CellValue` 分为 blank/text/boolean/number-lexeme/error；公式是否存在及其 cache 与 hidden/merge facts 分开记录。未知字段或不可表示语义必须进入封闭 issue/loss 或阻断 coverage，不能藏入自由 JSON。

## 4. ImportMapping/1 与 LossReport/1

`ImportMapping/1` 精确为 `{version:1,documents}`；每个 IR document 恰好有一个已排序且唯一的 mapping。现任 mapping arm 为 omit/document_node/rows_to_nodes/table_document，并保留 owner 定义的完整 range、title/body/retain-original、Field conversion、formula、hidden policy 与 destination field。

`LossReport/1` exact `{version:1,items}`。每项 `{lossKey,feature,locations,effect,severity,allowedChoices}`；key 连续。notice 无 choice，requires_choice 为 accept_loss/reject，blocking 只有 reject。文件 location 使用 Import `SourceLocation`；Node Template 使用 `TemplateLossLocation`，两域不混用。

## 5. ConversionInput/2

现任 author preparation 绑定 immutable `ConversionInput/2` ZIP。包内只允许 manifest.json、按序排列的 source/N.bin 与 resource/N.bin；必须使用 stored compression，不允许 encryption、link、duplicate 或 external reference。manifest 不包含自身 hash。

part0 保存完整的 mapping/construction 投影、route/profile、初始 loss/choice、groups/batches 与完整 proposed-object manifest。raw part 保留精确原始字节。partOrdinal 不是 PinRef/authority；object ordinal 由 source-location/role tuple 决定，而不是 identity。

## 6. TemplateRecipe/2 与 TemplateConstructionInput/2

`TemplateRecipe/2` exact：
```text
{format:"weftext.node-template",version:2,parameters,nodes}
```
每个 node 冻结精确的 `TemplateSourceAddress/2`、parentIndex、targetFacets 与封闭 binding。slot 只能是 title/body_text/field_append，不允许 expression 或 opaque extension。

`TemplateConstruct/2` exact：
```text
{version:2,template,recipe,parameters,resources,destination,externalNodePolicy}
```

`TemplateConstructionInput/2` exact：
```text
{kind:"node_template_construction",version:2,construction,inputPins,
 omittedAnnotations,lossReport,lossChoices,sourceSubjectBindings}
```
input pin 携带真实 `SourceVersion/2`、现任 `SourceObservation/1`、payload kind 与 `PinRef/2`。被省略的 Annotation location 只携带精确的现任 version address，不携带 body。source-subject binding 必须排序且唯一，并与所选 source 及原 request 的 fresh subject 精确闭合。

## 7. Current PAB4 ownership

D9 不定义 PAB。现任 D7 拥有 `PreparedActionBinding/4`，它绑定现任 action input、concrete proposed input、可选的内建 D9 construction evidence、`DependencyProof/3`、原 request、完整 preview/effect、pins、audience/budget/fingerprint/currentness。

fresh D9 author submission 只使用 mode 合法的 `D3IdentityOperationRequest/13` 或既有 D6 request。historical PAB1/2/3 与 wire11/12 只按 decoder 选中的历史记录处理。

## 8. Office token grammar

普通 token 保持 owner grammar 以及 one-pass/four-brace escape。simple native-table 的唯一 ASCII leaf 继续使用 `data.native_table.COLUMN`。

qualified native-table authoring grammar：
```text
native-selector := "native.table" table-qual "::column" column-qual
table-qual      := "[]" | "[" json-string "]" [ "#" counter ]
                 | "[null]#" counter
column-qual     := "[" json-string *( "," json-string ) "]" [ "#" counter ]
```

selector 放在同一 outer `{{ ... }}`、`{{ ↓ ... }}`、`{{ → ... }}` token form 内。JSON string 只解码一次到 Unicode scalar。Counter 使用 canonical decimal。qualification 必须是 Main §7 允许的 shortest unique chain。

这个具名修订只 supersede fresh qualified FC hash-token authoring。internal compiled Plan key 仍可使用 `nt_...` / `nc_...`；现任 Plan binding evidence 冻结精确 visible token 与 `D9NativeTableSelector/1`。historical hash-authored template/plan 保留其精确 decoder。

## 9. Export content 与 projection

`ExportContentSelection/1` 保持 exact `{version:1,bodyInput,bibliographyInput}`；每个 input 为 null 或一个 exact Document catalog index。

`ExportProjection/1` 精确为 `{version:1,bindings,datasets}`。binding path 必须唯一；value 使用有界 RenderSnapshot union。dataset name/column 都是封闭值，row 保留完整冻结的 order/bag，每个 cell 都必须具有非空 origin。

`D7ResultPin` 是 D9 对完整 D7 TerminalSchema/V result 及其 producer epoch/auth/cut/dependency 的内部 control evidence。D9 不重新定义 D7 V。

## 10. ExportPlan/3

final FC 的 exact current shape 为：
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

任何同名简化 object 都不是合法 Plan。除明示可 null 的 document/template/route binding 外，上述 member 全部必填。`planToken` 只使用 current `d9_export_plan/3` tag。`inputCatalog` 精确复用 `ExportInputCatalog/2`，不另改名为 D9 wrapper。

`D9ExportInputDomain/1` 精确为 `document|native_table|node_collection|query_rows|query_json|resource`。`D9ExportTarget/1` 使用 final FC 的封闭 target union：asciidoc_source、resource_exact、带 profileId 的 html/pdf/docx/odt、csv_utf8、tsv_utf8、带 profileId 的 xlsx/ods，以及 query_json。对 exact-source/resource/query-json 的 plan 强制使用 `generationPolicy={kind:"none"}`，且文档、模板和路由 binding 全部为 null。

`generationPolicy.render` 冻结封闭的 bindingChoices、missingPolicy、imageSizes、layoutChoices 与 nativeTableBindings。native-table binding record 按精确编译出的 (setName,columnName) 排序并去重，同时逐项冻结精确的 `D9NativeTableSelector/1`。本 A2 D9 候选只改变 fresh qualified Office authoring spelling；Plan3 selector/binding record 继续使用 final FC type。

document rendering 的 `D9DocumentRenderBinding/1` 绑定 ownerNodeRef、现任 SourceObservation、精确 D2-Document-Snapshot/3 pin、ManagedDocumentSemanticQualification/1 与实际消费的精确 D8PresentationDecision/1。route step 必须连续为 0..N-1，并逐步冻结 provider/version/input/output profile/options hash、BudgetBinding/1 与已排序去重的 evidencePins。style bundle 按 styleBundleId 排序并去重。

`recoveryPins` 只保留原 recovery contract 真正需要、且不能从其它 typed Plan member 到达的 pin。`evidencePins` 必须精确等于 inputCatalog、documentRenderBinding、templateBinding、routeBinding、styleBundles、dependencyProof、observationProof、stagedOutputs 的递归 typed PinRef 与 recoveryPins 的 pinToken-sorted/unique 并集；它不是自由 extension array。

template path 按精确 Unicode scalar sequence、normalization none、case-sensitive canonical sort 排序。controlled output name 先通过完整 validity/alias/prefix/reserved 检查，再让 stagedOutputs 按实际存储的原始 unsigned UTF-8 bytes 排序。全部 set-like array 都拒绝 duplicate/conflict。合法 input permutation 只允许在 freeze 前 canonicalize 一次；frozen/received/recovery record 必须已经 canonical，读取时绝不修复。

## 11. Output names 与 bundle

`D9ControlledRelativeOutputName/1` 保持精确 UTF-8，只由“/”分隔的非空 component 组成。empty/dot/dotdot component、backslash、control、forbidden punctuation、trailing space/dot、rooted/drive/UNC 以及 reserved device-name stem 都必须拒绝。

PortableAlias 固定执行 Unicode 15.1 NFC → full default CaseFolding C/F → NFC，而且只用于拒绝判定。完整 bundle 必须拒绝精确重复、alias 相等与 alias-prefix 冲突。loss-report.json 与 manifest.json 是保留的根成员。

staged output metadata 与 `PublicationReceipt/3.outputs` 按 exact stored output name sorted unique。

## 12. ExportLossReport/1 与 confirmation

`ExportLossReport/1` 保持：
```text
{format:"weftext.export-loss",version:1,planToken,inputs,items}
```
location 使用封闭的 ExportInputLocation，并扩展到 binding、dataset_cell 与 block projection location。source_range 使用 UTF-8 字节偏移；template_range 使用 Unicode 标量偏移。

`D9ExportConfirmation/1` 精确为 `{planToken,lossChoices}`。choice 必须排序且唯一，并覆盖 report matrix 定义的全部 requires_choice/blocking。confirmation 不得修改 Plan 或 staged bytes。

## 13. PublicationReceipt/3

final FC 的 exact current shape 为：
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

current token tag 只能是 `d9_publication/3`。所有与 Plan 重复的 member 都必须与 protected Plan/confirmation byte-equal；outputs 是实际 published 的 controlled-name/digest set。此 receipt 只证明 external publication，绝不能替代 D3 ChangeId/CP 或 Resource author success。historical Plan/Receipt1/2 即使旧 name 不符合 current predicate，也继续原 tag、bytes、name、permission 与 unknown-publication recovery。

## 14. Query JSON 与 typed tabular export

query_json 精确为 `{format:"weftext.query-result-export",version:1,schema,data}`；data shape 由 TerminalSchema kind 唯一决定，并嵌入原始 D7 V，按精确 numeric/Unicode semantics 编码。runtime result/cursor/row handle 不写入文件。

CSV 使用 RFC4180/CRLF 与 exact selected scalar projection。TSV 无 quote escape，field 含 tab/CR/LF 即拒绝。XLSX/ODS 的 text 始终写 text；integer/decimal/date/instant numeric/serial coercion 需要具名安全 profile，否则使用 exact text 或拒绝。危险 spreadsheet-formula text 按固定 Unicode-15.1 rule 拒绝。

## 15. WorkerInvocation/1

`WorkerInvocation/1` 精确为 `{version:1,jobToken,step,routeId,routeRevision,profileId,inputs,options,budget}`。input slot 由 host 分配。终态 worker result 是封闭的 ok/failed union；failed code 只能是 unsupported/encrypted/unsafe/malformed/budget/cancelled/internal。Worker 输出不能包含作者权限、receipt、identity 或 destination。

## 16. Region

`RegionBody` 精确为 `{version:1,profile,page,rect}`；`d9rg1` 是该 body 的 canonical inner token。page/rect 使用现任 D9 geometry profile。它不是 Locator。D3 `ResourceRegionLocator/l1` 继续拥有外层 authoritative identity/revision binding。

## 16a. 继承的 current Annotation Value/4 / R6 types

D9 不拥有这些 schema；但任何实际消费 current Annotation content 的 D9 export/copy/import consumer 都必须精确消费：

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

target projection 使用现任 D3 拥有的封闭 document/document_element/document_range/resource/resource_region union。body 只通过现任 `AnnotationInlineProfile/1` R6 profile 求值，使用固定到提交的 Asciidoctor Ruby 2.0.26 baseline，并保持 secure、managed adapter/file/network/process effects disabled。既有 D9 `annotation_index` export-catalog arm 继续只表示 omission-directory evidence，不能替代完整 Value/4 read。historical Value/3/plain_text record 只保留 historical decoding。

## 17. D9 error

D9/1、D9/2 只使用 owner 定义的封闭 code set。现任 workspace-side availability 包括现任 owner afterimage 中的 domain/proof/integrity/recovery condition。进入 D3/D6/D7 boundary 后，其 error 绝不能重新编码成 synthetic D9 success/failure。
