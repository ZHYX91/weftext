---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: fd325b06-f122-4a6e-bbb6-e484c556ae2b.

Candidate status: D9 file-authority PL-IR-01 repair terminology consumer; not independently accepted, activated, or implemented. Fixed-S names/identities remain. This lexicon now describes the authored stable-production-address/current-Observation split and keeps the new exact repair pending independent review; it does not close P1 or any other integration gate.

# D9 Terminology and Naming Gate

Fixed-S scope and evidence statement: existing D3/D4/D5/D6/D7/D8 terminology authority remains. D9 adds terms only within conversion and does not revive historical objects under borrowed names.

| Canonical term | Sole meaning/owner | Explicitly forbidden |
|---|---|---|
| SourceArtifact | External input bytes obtained and pinned by the host; connects to D3 source_artifact evidence without making the D9 descriptor a new identity class | Paths/hashes as Node IDs; equal digests treated as the same input occurrence |
| Import IR | Weftext conversion intermediate representation; its profile defines observations and loss without author authority | A third-party DoclingDocument becoming the domain AST; automatic compatibility for old ImportIr versions |
| SourceLocation / Observation | Coordinates and extraction evidence within one input version | D3 Locator, writable target, cross-version anchor, or D4 Provenance authorization ticket |
| Provider / Route | Installed and reviewed execution adapter / fixed finite pipeline | Free commands, template scripts, worker-asserted authority, generic fallback |
| ImportMapping / MappingProposal | Explicit Core conversion selection / complete proposal before preparation | Query operator, new D7 ActionSpec kind, generic patch to existing sources |
| ImportJob | Reuses the D6 finite multi-batch control record | A second author ledger or whole-job atomicity/automatic rollback promise |
| coupling group / batch | Indivisible author group / one original atomic D3/D6 request | UI page, worker process, Query page; silently splitting a Template at 1000 items |
| ConversionInput | Product evidence artifact part binding raw inputs, IR, mapping, loss, and route | Council packaging hashes, persistent external identity, or OriginBinding |
| Node Template / TemplateRecipe | D2 Template and explicit Resource recipe for one fresh Core construction | Office macro, continuous instance binding, free parameter interpolation |
| Office template / placeholder / style directive / repeat band | Ordinary Office text template / value insertion / visible style exemplar / one complete row or column | A content-control/named-range/Excel-Table enhancement layer; attr/record aliases |
| RenderSnapshot / D7ResultPin / ExportPlan | Finite rendering projection / original complete D7 schema-and-V result pin / current output preparation record | Author snapshot import, replacement for D6 PreparedIntent, saved rowHandle identity |
| StagedOutput / PublicationReceipt | Complete unpublished output / external publication fact only | D3/D6 commit receipt or evidence of Resource creation |
| LossReport / issue | Core loss selection for file import and Node Template / original observation issue | Export addresses, arbitrary cross-domain locations, safety approval, proof that undiscovered content is complete |
| ExportInputCatalog / ExportProjection / ExportLossReport | Complete authorized input / finite rendering projection / fixed export loss report for the current ExportPlan | New author source, arbitrary client values, import SourceLocation, Query rowHandle identity |
| image_physical_size/1 / imageSizes | Verifiable image physical facts and quantization / frozen user output-layout selection | Fabricating source physical dimensions from pixel counts, 72/96 DPI, or viewport |
| RegionBody / d9rg1 | Non-identity profile geometry facts | Complete Locator or opaque registry identity |
| ResourceRegionLocator / l1 | Original D3 complete ResourceRef/revision plus geometry binding | New D9 locator kind or repeated fresh Ref inside the inner body |

Public names use Field rather than ambiguous property/attr. FieldId is the complete namespace/path, never a UI label. header means only a D2 raw header, meta only title/subtitle, and node.id an authorized display. data.SET.COLUMN exists only in the explicitly selected dataset. Paragraph/table rows, Field occurrences, Node Collections, and Query rows are four distinct domains. Dataset is a general export-input term, not a new persistent Record type. Future ICS, OriginBinding, and mapping_table have meaning only under their explicit D3/D6 profiles; this generation provides no implicit support.

Error vocabularies have exactly four domains: the main document's d9_error.code, original D1 reasons, original D7/D3/D6 errors, and internal Worker failed.code. Template ambiguous_binding/missing descriptions map externally to d9_error template_invalid or mapping_required; they do not create undefined codes. Diagnostic.code uses only the main d9_error closed set, and feature only the IR/loss set. A plain-text message is not a stable wire discriminator.

Naming checks prohibit a new durable EntityRef/Record/FieldRef. D9 numeric Counters differ from D4 numeric lexemes; originClass is a host input channel rather than D3 external identity origin; outputSlot/resourceKey/sourceIndex are local ordinals. Unknown profiles/versions reject, and old public prototype aliases are not retained. Future implementation removes active obsolete names from parsers, help, examples, and tests together; historical research may remain explicitly non-authoritative.

The coordinating terminology check must run again against the final candidate and actual gate. This text does not claim an independent naming gate has passed.

Current PreparedActionBinding/3 is the D7 owner's managed preparation record. constructionInput contains only source and compilation evidence, not a new Action, Ref, D3 mode, or commit entry. resultAllocations and identityMap follow the original mode; source-to-instance display is only the mechanical join of sourceSubjectBindings and the original receipt. hiddenPolicy is fixed mapping input; accept_loss confirms a fixed loss rather than executing an include/omit program.

TemplateLossLocation addresses only a template input pin or explicitly omitted Annotation, separately from external IR SourceLocation. It is not a D3 Locator, write target, or new durable address.

Every ExportLossLocation ordinal is interpreted only within the same immutable Plan catalog, complete result, or projection. Equal-valued duplicate rows remain separate. A report summary is not identity or authorization. source_range uses UTF-8 byte boundaries while template_range uses Unicode scalar boundaries. Export confirmation, external PublicationReceipt, and original D3/D6 author receipts retain separate roles.

ExportContentSelection/1 is internal consumption-selection evidence for an export Plan. bodyInput/bibliographyInput bind only its exact Document catalog input; null means not requested. It is not author schema, a permission grant, or new public wire; an input pin does not mean all its content was selected.

## Source and responsibility terminology for the current versions

| Canonical term | Sole current meaning/owner | Explicitly forbidden |
|---|---|---|
| SourceVersion/2 | D6 sealed managed production version or explicit external production-observation version, retaining its production domain/generation | Treating it as a current read token; externalSequence as managed revision |
| SourceObservation/1 / SourceVersionRef/1 | Complete protected current D6 observation / narrow {entityRef,sourceToken} projection | Reusing sender observations, re-signing equal Counters/bytes, or treating conflict installation input as current source |
| DecisionKey/2 | Complete Workspace, CommitDomain, and OperationId address of the original single D3/D6 decision | Merging the same OperationId across domains; a worker retry creating a second decision |
| TemplateRecipe/2 / TemplateSourceAddress/2 | Persistent node-template/2 recipe / exactly fixed managed production address | Following latest, storing current sourceToken, guessing Recipe/1's production domain, automatically rewriting body Locators |
| TemplateConstruct/2 / TemplateConstructionInput/2 | Request selecting current sources / complete current observations, real source pins, and compilation evidence | Arbitrary client AST, another ActionSpec or submit path, expanding an omission directory into Annotation body reads |
| D9EntityVersionAddress/2 | Evidence position containing a complete Ref and production SourceVersion/2 in an authorized same-cut directory | Acting alone as current Observation, SourceVersionRef, or write capability |
| ConversionInput/2 | Immutable product ZIP evidence for complete current inputs and construction projection | Reconstructing lost purpose-bound pins from partOrdinal/digest; calling an external artifact a managed source |
| ExportPlan/2 / ExportInputCatalog/2 / PublicationReceipt/2 | Complete current-domain output preparation / actual source catalog / external publication fact only | D6 CP3, author receipt, re-rendering to replace original bytes, renaming and retrying unknown publication |

ImportIR/1, WorkerInvocation/1, Office template/1, original SourceArtifact/IR coordinates, ExportContentSelection/1, ExportProjection/1, ExportLossReport/1, query-result-export/1, and bundle/1 do not change versions merely because a status line changed. Each current/historical version is selected by its actual decoder and named carrier; no new fields are silently added to old shapes and missing fields never determine the version.

“Complete source” means the finite authorized range actually consumed or explicitly enumerated for omission by this operation, not all Workspace/body content by default. “Current” belongs to an observation/authorization cut, “production version” to source history, and “external publication” to the original publication intent. Equal digests do not give them one lifetime. semantic_pending cannot authorize import, construction, or Query export that requires complete proof, and never conceals typed/source invalidity.

The authored PL-IR-01 repair is now part of the current terminology surface: a revision token is a stable production address, while SourceObservation/SourceVersionRef carry current observer qualification. PL-IR-01 remains pending independent review of the exact candidate, not an undefined naming/algorithm gate. D3/D6 IR-06/07 and complete current D7 producer gates remain separate. An author's naming check does not accept the package; historical saved/planned/unknown records keep their original decoder, disclosure, pins and no-repeat duties.
