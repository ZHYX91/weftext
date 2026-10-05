---
source_language: zh-CN
translation_of: SPEC.zh-CN.md
translation_status: synced
---

[简体中文](SPEC.zh-CN.md)

# Weftext AsciiDoc / Annotation Final Design Candidate: coordinated owner replacements

Status: **candidate-design-not-implemented**. This specification is bound to parent input e8aa0b341630a57c786c0891d4bbd1620247441d. It is only a coordinated design candidate on top of that commit; it does not accept this PR, A2, the global design, implementation, or release.

## 0. Precedence, scope, and why this is one joint PR

This file and [SCHEMAS.md](SCHEMAS.md) together form the complete normative replacement for the actual-owner sections named by replacements.json: this file freezes behavior, algorithms, and owner boundaries; SCHEMAS freezes closed current shapes, ordering, cross-field rules, and historical dispatch. Summary wording here may neither broaden nor narrow SCHEMAS. Fixed-S input 7e18168dad3e6d120fce0dd607dc10fa7894e252, all 49 snapshots, and docs/design/inputs.json remain unchanged. Any fixed-e8aa owner section not named by the router retains its existing meaning.

The candidate consolidates the independently reviewed cumulative v3.10 core design and FC-3.4 through FC-3.4d actual-owner coordination into a public, reviewable contract. Old candidate labels, private conversation, and author self-review are not protocol dependencies.

A two-PR split was considered. It is not self-consistent: the current DependencyProof/3 -> PreparedActionBinding/4 -> D10 ApprovalUse/2 chain and the shared EffectManifest/3 / EffectBytes/3 family simultaneously carry managed-format currentness, D10 author recovery, and WorkspaceBootstrapPlan/4. Publishing a temporary closed /3 union and expanding that same version in a second PR would violate the accepted closed-decoder rules. Therefore this is one joint PR organized by D2-D10 owner.

There remains one author authority, one D6 planning CAS, and one P seal. No second source truth, portable ledger, decision system, or free-form adapter gate is introduced. Derived Index remains discardable and never reconstructs current authority.

## 1. D2: complete AsciiDoc language contract

### 1.1 Fixed language baseline and production boundary

The observable AsciiDoc core-language semantics for Weftext are fixed to:

```text
asciidoctor-ruby/2.0.26
commit 0b99b39c9df884d4aec13bba45f03cdbab505769
```

Ruby Asciidoctor is a conformance oracle only, never a production dependency. Production remains Rust-native. A mature implementation is preferred; Asciidork commit 79671b57923841e41fc91d2c1d2fce18cca3131c is an implementation candidate, not a language ceiling. Implementation difficulty, missing rich-editor controls, or an unavailable renderer provider cannot redefine valid AsciiDoc as a safe subset.

The core contract includes the fixed 2.0.26 semantics for block/inline/list/table/description/callout/checklist forms, attribute lists and document attributes, includes, substitutions and macros, passthroughs, STEM, media, footnotes/indexterms/catalog, doctype/backend behavior, document title/author/revision, block title/reftext, native TOC/section numbering, leveloffset, diagnostics, and all processor-environment inputs that alter observable semantics. A valid construct with no structural editing widget must remain parseable, readable, source-editable, savable, and losslessly round-trippable.

Native title-separator semantics remain unchanged: default : , with [separator=::] and :title-separator: :: supported as Asciidoctor 2.0.26 defines them. **Whether new Weftext templates default to :: remains an unresolved product choice and is not selected here.**

### 1.2 Processor environment

The Core Gate freezes AsciiDocProcessorEnvironment/3 instead of consulting ambient host state. Its canonical bytes explicitly carry doctype, backend semantic profile, safe mode, standalone mode, base/input/output identity, the ordered API attribute-event sequence, user-home, UTF-8 source encoding, locale profile, date/time/epoch inputs, include resolution and file/network authority, extension registry, provider profiles, and every other registered input that can change fixed-2.0.26 observable semantics. Authored attribute events remain provenance-distinct from host/builtin/include inputs. Weftext root controls such as wf-kind and wf-facets consume root-authored provenance only; host injection or included text cannot take over root Node classification.

attributeOverrides is the ordered API-event sequence received from options[:attributes], not a name-sorted set. Each input pair is normalized exactly as fixed Ruby 2.0.26 handles ! / @ syntax, then lowercases the name and records one of hard_set, soft_set, hard_unset, or soft_unset. Array order is the API Hash enumeration order and D3-CJ/3 preserves it. Repeated lowercase names are legal events: the first occurrence fixes that key's ordered-map position and a later same-name event replaces its state without moving the key. Sorting, deduplicating, or collapsing to final values is forbidden.

That order is observable. For embedded processing (standalone=false), when both notitle and showtitle are present, fixed 2.0.26 examines which of those keys is last in ordered attr_overrides and derives the opposite alias from that key. Therefore API order notitle="" then showtitle="" and the reverse order cannot share ProcessorEnvironment canonical bytes or an oracle/cache identity.

The managed/source Gate admits UTF-8 source bytes only, so sourceEncoding is the closed constant UTF-8; another encoding requires explicit import/transcoding rather than oracle inference. localeProfile is an explicit controlled oracle-harness descriptor with no ambient-host fallback. Fixed 2.0.26 does not currently auto-load locale attribute files from lang; lang remains a document attribute. providerProfiles lists only provider profiles actually allowed/selected for the evaluation and may be empty when no provider is used; provider availability remains separate from core-language validity.

### 1.3 Native headings, deep authored headings, and run-in rendering

The native Asciidoctor section state machine is retained, including discrete/floating titles, book part/chapter behavior, skip warnings, fragment relaxation, negative-leveloffset handling, and doctitle conditions. WeftextManaged adds only these semantic extensions:

- authored section levels 6-9;
- standard leveloffset remains standard and the effective level is not clamped merely because authored levels are 6-9; it may exceed 9;
- stable Node links/backlinks never derive identity from heading text;
- a run-in heading and its body remain distinct source and semantic nodes even when a renderer presents them on one line.

TOC, search, outline, backlink, export, and editor surfaces consume the same section projection. A renderer depth limit is a backend capability result, not a mutation of the core tree.

### 1.4 Stable links and source provenance

Weftext Node-link, citation, and Resource adapters consume the native link/image/attribute-list parse result; they are not a second link parser. Identity comes from D3 NodeRef/ResourceRef. Text, path, title, and hash never infer identity. A managed included span belongs to its real included source owner. Unmanaged or network includes can participate as authorized evaluation snapshots, but do not automatically become durable exact annotation/write owners.

Every semantic span retains a full source-origin graph. Synthetic multi-origin text without a unique writable source can still be read, but structured write must fail closed.

## 2. Test oracle: Witness/7 and implementation-independent semantic projection

### 2.1 Two-layer conformance gate

For the same source S and frozen environment E:

```text
W = observedRuby226(S,E)
C = completedWeftextCoreEvaluation(S,E)
R = projectRuby(W)
T = projectWeftext(C)

CoreSemanticGate = D3-CJ/3(R) == D3-CJ/3(T)
```

The input/environment binding, observer-neutrality gate, and completeness of both projections are additionally required. Final HTML equality, raw trace equality, or equality of a few selected strings are not substitutes.

The Ruby observer is test-side evidence only. It is not a product parser or authority. It cannot replace the selected backend, insert markers/sentinels, mutate evaluator strings or encodings, add extra calls to content/text/title/reftext/xreftext, or rerun parsing/regexes to manufacture evidence. Actual converter return values continue into later substitutions unchanged.

### 2.2 OracleSemanticWitness/7

Witness/7 retains all Witness/6 document, block, collection, catalog, diagnostic, observed-string, call, operation, inline, and content evidence and adds:

```text
modelPropertyObservations:[OracleModelPropertyObservation/1...]
```

Model observation only copies fields, attributes, relationships, and actual producer results that already exist. Section state, caption/numeral, final table widths, final Cell object/style/alignment/span, ListItem checklist/coids, authored named/positional/rekey provenance, and temporary converter writes must be captured at the real producer. projectRuby never reimplements Ruby parser/model algorithms to fill missing facts.

### 2.3 Snapshot, bind, and cut base invariants

modelPropertyObservations[].observationId is unique only within that array namespace and is strictly monotonic in actual append order. A snapshot's identity is its observationId; there is no second snapshot-ID namespace. For each subject, its snapshots ordered by observationId form one immediate-predecessor chain: the first has previousSnapshotObservationId=null and every later snapshot points exactly to the directly preceding snapshot of the same subject.

Define latestSnapshotBefore(subjectId,o) as the same-subject snapshot with the greatest observationId strictly less than o. Every bind snapshotObservationId exists, belongs to the same subject, is strictly earlier than the bind, and equals latestSnapshotBefore(subjectId,bind.observationId). A cut head likewise selects latestSnapshotBefore(subjectId,cut.observationId). heads are strictly sorted by numeric subjectId and subjectId is unique.

### 2.4 Exact semantic-head closure

For each cut C, select rootBind(C) as the latest valid bind before C whose carrier.stream="document" and carrier.entry=0. It binds a document subject and C.documentSubjectId equals rootBind(C).subjectId. If a treeprocessor actually returns a new Document, the new M37-14 document/0 publication and bind establish the new root; the former root remains historical evidence but is not live merely because it used to be root.

Structural closure starts with C.documentSubjectId and traverses the latest snapshot of each queued subject at C. ForwardSemanticRole/1 is exactly header, child, dlist_term, dlist_description, table_column, table_head_cell, table_body_cell, table_foot_cell, and cell_inner_document; only these roles add subjects. Every target exists and has a semantic-head kind. parent and cell_column are backedges only: after forward traversal their targets are already in structural closure. A backedge never expands the closure or revives a stale Cell or Document.

Supplementary subjects use only two closed rules. An inline subject is live only when a valid pre-cut bind targets inlineObservations and its latest snapshot has a parent relation to an already-live structural subject. A catalog_record is live only when a valid pre-cut bind targets catalogEvents and its latest snapshot has exactly one catalog-ownership parent to the Document subject that actually received Document#register, with that Document already in structural closure. There is no owner relation and an inner-Document catalog record cannot be reassigned to the top-level root. attribute_buffer and table_parser_context are always supporting-only and never become heads merely because snapshots exist.

Let R be structural closure plus those live inline/catalog supplementary subjects, and H=set(C.heads[].subjectId). A valid cut strictly requires:

```text
H == R
```

Each head also points to that subject's latest physical snapshot at C. Physical membership/latest selection is separate from final CSP survival: temporary/internal snapshots, supporting evidence, or a final physical absent value never authorize the verifier to pick an older head. PropertyProfile/CSP independently chooses author-semantic winners from A/P/F/I provenance.

### 2.5 Supporting-evidence closure

Build a typed worklist from every cut-head snapshot, rootBind(C), and every valid pre-cut bind whose subjectId is in R. A snapshot recursively follows previousSnapshotObservationId, every ModelPropertyProvenance.inputs in fields/attributes, and all nested string/array/map ModelObservedValue members. The five ModelPropertyInput/1 arms have fixed edges: model_slot -> the named model snapshot and slot; observed_value -> OracleObservedString; observed_slice -> the named OracleObservedString plus byte-half-open range validation; operation -> OracleStringOperation; inline_field -> OracleInlineObservation plus fieldPath validation.

OracleStringOperation follows its callId, all input/removed slices, outputValueId, and every run: exact_copy follows its slice; derived follows all slices plus its referenced operationId; generated follows inlineEventId when non-null. The operation graph obeys its retained DAG contract. OracleEvaluationCall recursively follows parentCallId only within the call namespace. OracleInlineObservation follows callId, parentCallId, producingOperationId, all nested OracleFieldValue members in nodeType/fields, and returnValueId. OracleFieldValue recursively follows observed_string, array, and entries values. OracleContentObservation follows callId, resultValueIds, and contributingInlineEventIds. OracleObservedSlice resolves its valueId to OracleObservedString and validates bounds; ObservedString is a leaf.

Every bind also statically resolves ModelCarrierReference. document permits entry=0 only; every other entry is a zero-based index into blockEvents, collectionEvents, catalogEvents, inlineObservations, contentObservations, or calls and must have the stream's exact type. Inline, Content, and Call carriers continue through the recursive edges above. Block/Collection/Catalog/document carriers retain their own closed shape/ordinal validation but create no cross-array event chronology.

The support closure may contain old intermediate snapshots, attribute_buffer, table_parser_context, old Cells, and temporary-writer evidence without adding those subjects to H. Any dangling or wrong-kind reference, invalid slice/index, illegal operation/call DAG edge, or missing provenance input makes Witness evidence invalid/incomplete rather than shrinking valid AsciiDoc. No parser or getter may be called again to manufacture missing evidence.

### 2.6 Namespace separation, producer-time binding, and cut cardinality

Only references in the modelPropertyObservations.observationId namespace use numeric backward/latest checks: previous/bind/cut snapshot references and model_slot observations. operationId, inlineEventId, valueId/observedStringId, and callId are resolved only in their own namespaces under their retained existence/type/DAG rules; subjectId is object identity and carrier.entry is an array index. Cross-namespace comparisons such as operationId < modelObservationId or inlineEventId < modelObservationId are forbidden, and no global ordinal is introduced.

Bind-time chronology cannot be proven by the final wire. Complete Witness validity requires both static-decoder validity and producer-conformance validity. At the real bind callback the controlled observer has already published the target carrier, entry < targetStream.lengthAtBind, and still holds the exact same Ruby object represented by that carrier; only then may it append the bind. document/0 likewise publishes the actual returned Document carrier first. Later filling of the final array can never cure an earlier invalid bind, and text/title/source/path/hash lookup cannot reconstruct object identity afterward.

A complete top-level Witness/7 has exactly one model_ready cut. If selected-backend evaluation never occurs or terminates abnormally, there is no valid evaluation_complete. If actual evaluation returns normally, there is exactly one evaluation_complete, later than model_ready and using the same top-level documentSubjectId. Nested inner Documents do not create another top-level cut namespace.

### 2.7 Catalog ownership and temporary state

Catalog ownership is only parent -> the actual Document#register receiver Document subject. That parent is used for supplementary liveness and reverse consistency; it never pulls a stale Document outside structural closure into R.

A temporary overlay closes before a valid evaluation_complete only through a real restore write or a real cleanup delete/closure on the fixed source path; the observer never fabricates a restore. The load-bearing DocBook root-option path is, for example, authored value -> internal set_option temporary value -> remove_attr absent/deleted, and evaluation_complete selects the latest physical cleanup snapshot. PropertyProfile may still preserve the earlier valid authored winner through provenance. If a later genuine authored overwrite/delete occurred, temporary cleanup cannot resurrect an older authored value.

## 3. CoreSemanticProjection/1 and canonical property profile

### 3.1 Projection

The equality surface contains final document/block structure, flows, catalog facts, index terms, final attribute/counter state, and diagnostics. CoreFlow/1 uses final runs and marks with Unicode-scalar coordinates. Visible text is owned once; hard breaks, footnote references, and other atoms do not duplicate text already owned by the flow.

Raw Ruby IDs, call counts, temporary converter-return nodes, and intermediate dependency edges are excluded from equality. Their real effect on final target strings, text, catalog, counters, or attributes must remain. Authored passthrough and backend-feedback raw data are not guessed into ordinary formatting marks.

### 3.2 Canonical PropertyProfile

Property provenance is A/P/F/I: authored named, authored positional, fixed-derived semantic, and internal. Positional rekeying leaves one canonical slot. A genuine authored named attribute remains even when the current renderer ignores it. Internal cloaked-context, caches, reader objects, and temporary root-option state never leak via generic to_s.

Common fields include id, style, ordered roles, set-like options, caption, numeral, explicit substitutions, unconsumed positional fields, and named/<name>. The profile also fixes actual 2.0.26 semantic fields for quote/verse attribution+citetitle, source language/linenums, section sectname/special/numbered, tables/columns/cells, lists/checklist/coids, media, and marks/atoms.

Explicit foo-option=bar preserves both option membership and named/foo-option="bar"; an explicit empty value preserves the empty string. %foo, options=foo, and opts=foo create membership only. Actual writer order decides the winner.

```text
options += "foo"
named/foo-option = "bar"
```

### 3.3 Canonical arrays/diagnostics

Language sequences retain order. Sets are sorted/unique. Multisets sort by D3-CJ/3(item) UTF-8 bytes while preserving multiplicity. Unique-key conflicts reject instead of LWW. Diagnostics compare a common canonical code and null | {logicalFile,line}; a Rust-only column is not invented on the Ruby side.

```text
null | {logicalFile:text|null,line:UInt|null}
```

### 3.4 Current D2 product projection

CoreSemanticProjection/1 remains test-oracle comparison data and is never a product read/edit/query wire. Current managed AsciiDoc product reads use D2DocumentSnapshot/3 and the closed D2DocumentPayload/3, D2DocumentBody/3, D2ProductBlock/3, D2ProductInline/3 family in SCHEMAS §4.4. Exact source remains the sole author authority. The product projection, editor maps, indexes, rendering trees, and search/query values are disposable derived state; none may become a second parser, source store, durable block identity, or write authority.

The current projection covers every fixed-Asciidoctor-2.0.26 native block and inline category that can affect observable document semantics, plus the narrowly specified Weftext adapters. A valid construct is never made syntactically unsupported merely because no rich-editor control or provider exists. A surface that cannot structurally edit a legal construct exposes the exact Source path and preserves all untouched bytes on save. Unsupported structural editing returns an editing/unavailable result instead of dropping, flattening, or rewriting the construct.

| semantic category | current product shape | actual current consumers |
|---|---|---|
| document metadata/title/author/revision and header attributes | D2DocumentSnapshot/3 → D2DocumentMetadata/3 | D8 document/Draft read, D7 title read, D9 document render/export |
| sections and floating titles | D2SectionBlock/3 + D2Heading/3 with authoredLevel and effectiveLevel | D7 headings scan/Query, D8 outline/folding/visual editor, D9 HTML/PDF/DOCX/ODT |
| paragraphs and inline semantics | D2ParagraphBlock/3 + D2ProductInline/3 | D7 body_text, D8 visual/read/Source, D9 rendered/semantic export |
| unordered/ordered/description/callout/checklist lists | D2ListBlock/3 + D2ListItem/3 | D8 structure/Source, D7 body_text, D9 render/export |
| tables, columns, rows, cells, including AsciiDoc cells | D2TableBlock/3 + D2TableCell/3 | D5 native-table boundary, D8 table/read/Source, D9 table/document export |
| example/sidebar/open/admonition/listing/literal/source/pass/stem/quote/verse | D2DelimitedBlock/3 or D2ContainerBlock/3 | D8 read/Source with optional structural UI, D7 body_text where defined, D9 supported render route |
| image/audio/video and other media | D2MediaBlock/3 plus D2IdentityAdapter/1 when Weftext-owned | D8 media surface, D7 readable projection, D9 resource-aware export |
| native links/xrefs/anchors/footnotes/indexterms/STEM/quoted text/kbd/menu/buttons/callouts/breaks | D2ProductInline/3 closed arms | D7 readable/query projection, D8 navigation/visual/Source, D9 render/export |
| include/substitution-generated or multi-origin semantics | D2SourceOrigin/2 arrays on the produced semantic node | D8 navigation and write gating, D7 read/query dependency, D9 evidence/loss mapping |

D2Heading/3 keeps authoredLevel distinct from effectiveLevel. WeftextManaged permits authored levels 6–9; standard leveloffset can produce effective levels above 9 and is not clamped by a Weftext cap. Native section state, document-title/book-part rules, warning behavior, explicit anchors and source ranges are retained. TOC, search, outline, stable links, Query, editor, and export therefore consume one product heading projection rather than independent heading reconstructions.

Native link/xref/image/citation grammar is parsed first by the one AsciiDoc parser. D2IdentityAdapter/1 is applied only after a native occurrence exists and only when the Weftext target syntax has been independently validated. The adapter may attach a stable NodeRef, owner-local ResourceRef, or citation target, but it does not change native label/source grammar and cannot infer identity from path/title/text/hash. Backlinks remain derived. A managed include keeps the included source's own owner in D2SourceOrigin/2; inclusion never changes root Node identity or grant the including Document write authority over that included source.

Legacy Profile-v2 restrictions that rejected otherwise-native constructs are superseded for current managed AsciiDoc. In particular, a foreign delimiter-looking line inside a native delimited block is interpreted by the fixed native parser unless that exact native grammar makes it structural; D2 no longer invents a cross-family delimiter error. The Weftext attribute-carrier extension remains reserved only at its own explicit root-carrier grammar and does not generalize into a second block parser. The previous open-AsciiDoc/include/pass/extension prohibition is retained only as historical implementation input and is replaced for current implementation by this complete fixed-baseline product contract.

Invalid source still preserves the exact authorized source and ordered diagnostics while product projection is unavailable and D2 commit eligibility rejects. Physical decode/source-envelope failures remain D6 errors, not D2 invalid syntax. Repair and Source surfaces therefore remain available under their original authorization even when D7 Query, rich visual projection, or rendered export cannot obtain a valid semantic projection.

## 4. Provider profiles: language validity is separate from renderer availability

Mermaid, STEM/TeX/AsciiMath, HTML, and PDF are versioned ecosystem profiles, not AsciiDoc syntax switches. Valid core source stays valid when a provider is unavailable; BackendStatus reports unavailable/denied/incomplete and exact .adoc export can still succeed.

### 4.1 MermaidProviderProfile/1

Pinned upstream inputs are:

```text
@mermaid-js/mermaid-cli 12.0.0
tag commit db1ceebbe529d7975474eb0d0e9c23e9dc57cd37
package.json Git blob 883e27cc3e536e4efa9a48351626d8126ae9d36d
package-lock.json Git blob b814e336ee6af8ba265a9c29e86cb8c3ca4a923d
packageManager npm/10.8.1
engines.node >=22.13.0
peer puppeteer ^25.0.0
```

The lockfile Git blob pins the recursive npm input, but execution may not rely on ranges alone. An executable provider must register:

```text
MermaidProviderProfile/1 = {
  kind:"mermaid_cli", version:1,
  cli:{version:"12.0.0",sourceCommit:"db1ceebbe529d7975474eb0d0e9c23e9dc57cd37",
       packageJsonGitBlob:"883e27cc3e536e4efa9a48351626d8126ae9d36d",
       packageLockGitBlob:"b814e336ee6af8ba265a9c29e86cb8c3ca4a923d"},
  node:{version:text,executableSha256:"sha256:<64 lowercase hex>"},
  browser:{product:text,buildId:text,executableSha256:"sha256:<64 lowercase hex>"},
  fonts:[{family:text,assetSha256:"sha256:<64 lowercase hex>",licenseId:text}...],
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  network:"denied"|{kind:"allowlist",origins:[text...]},
  localResources:"denied"|{kind:"explicit_pins",pins:[PinRef/2...]},
  output:"svg"|"png"|"pdf",
  outputValidator:"weftext-render-output/1"
}
```

node.version must satisfy >=22.13.0 and the executable digest must also be fixed. Missing an exact browser/font/config field makes the provider unavailable. CLI 12 uses size rather than width/height, pdf-paper-format rather than pdfFit, and themeCSS rather than cssFile. Default embedded fonts do not make arbitrary host fonts equivalent. Online npx dependency acquisition and floating-semver replacement dependency trees are forbidden.

SVG validation rejects script, event handlers, javascript URIs, DTD/entities, and unauthorized network URIs/CSS/fonts. Unsafe output is rejected rather than sanitized and then claimed equivalent.

### 4.2 STEM / HTML / PDF profiles

```text
StemProviderProfile/1 = {
  kind:"stem",version:1,
  notation:"tex"|"asciimath",
  engine:{name:text,version:text,artifactSha256:"sha256:<64 lowercase hex>"},
  fonts:[{family:text,assetSha256:"sha256:<64 lowercase hex>",licenseId:text}...],
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  network:"denied"|{kind:"allowlist",origins:[text...]},
  localResources:"denied"|{kind:"explicit_pins",pins:[PinRef/2...]}
}

HtmlRendererProfile/1 = {
  kind:"html",version:1,
  renderer:{name:text,version:text,artifactSha256:"sha256:<64 lowercase hex>"},
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  resourcePolicy:"embedded_or_explicit_pins/1",
  outputValidator:"weftext-html-output/1"
}

PdfRendererProfile/1 = {
  kind:"pdf",version:1,
  renderer:{name:text,version:text,artifactSha256:"sha256:<64 lowercase hex>"},
  browser:null|{product:text,buildId:text,executableSha256:"sha256:<64 lowercase hex>"},
  fonts:[{family:text,assetSha256:"sha256:<64 lowercase hex>",licenseId:text}...],
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  network:"denied"|{kind:"allowlist",origins:[text...]},
  outputValidator:"weftext-pdf-output/1"
}
```

Native HTML STEM paths may otherwise rely on a CDN; offline read/export must never make that network dependency implicit. Missing engine/resource pins means renderer unavailable, not invalid language. HTML/PDF providers likewise pin the actual artifact/config/font/browser when used and never complete the profile from host defaults.

**This PR does not install or execute these providers.** No exact Node/browser/font execution profile has been registered, so it does not claim Mermaid/STEM/PDF renderer tests have passed. That is an explicit implementation-evidence gap, not a reduced language target.

## 5. Managed document format and current semantic qualification

### 5.1 Unique managed profile

```text
ManagedDocumentFormatProfile/1 = {
  languageBaseline:"asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  managedProfile:"weftext_managed/1"
}

ManagedDocumentFormatBinding/1 = {
  kind:"weftext_managed_document_format",version:1,
  ownerNodeRef:NodeRef,bindingRevision:Counter,
  profile:ManagedDocumentFormatProfile/1
}
```

BaselineOnly is an unbound analysis path, not a portable binding variant. A managed current Node with missing format binding is incomplete/proof-unavailable and does not fall back to BaselineOnly or a latest profile.

A fresh managed Node begins at bindingRevision 1. Same-Workspace copy/fork with fresh identity creates a new revision-1 binding while preserving the exact profile generation. Formal same-Workspace restore restores the exact historical binding. Ordinary backup-byte import has no identity continuity: analyze BaselineOnly, produce the managed delta, obtain explicit admission, and create a new revision-1 binding. Only a real managed-profile migration increments bindingRevision. A source-unchanged profile-only migration creates one portable ChangeId but sourceChanges=[]; it does not increment SourceVersion or H(D,E).

### 5.2 Current qualification

DocumentFormatCurrentQualification/1 binds a document-format DependencyKey/stamp, present ComponentImage, exact binding, and portable_metadata binding pin; the image version equals bindingRevision and the pin bytes equal D3-CJ/3(binding). ManagedDocumentSemanticQualification/1 binds the current SourceObservation and this format qualification. Either changing makes old semantic projections and preparations stale.

```text
DocumentFormatCurrentQualification/1 = {
  key:DocumentFormatDependencyKey/1,
  stamp:{epoch:Token,revision:Counter},
  componentImage:ComponentImage/1,
  binding:ManagedDocumentFormatBinding/1,
  bindingPin:PinRef/2
}
```

## 6. D6 dependency, portable component, and completion successors

### 6.1 DependencyKey/3

DependencyKey/3 has the fixed rank order: source=0, document_format=1, lifecycle=2, placement_range=3, ref_inbound=4, relation_incidence=5, calendar_scope=6, registry=7, temporal_rules=8, authorization=9, foreign_binding=10, query_scan=11, replica_registry=12, conflict_record=13, execution_resource=14. Historical Key/2 stays fourteen-arm under its original ranks.

```text
0 source
1 document_format
2 lifecycle
3 placement_range
4 ref_inbound
5 relation_incidence
6 calendar_scope
7 registry
8 temporal_rules
9 authorization
10 foreign_binding
11 query_scan
12 replica_registry
13 conflict_record
14 execution_resource
```

DependencyProof/3, InputDescriptor/3, and PreparedIntent/3 retain their prior responsibilities while consuming Key/3.

### 6.2 PortableComponentKey/2

PortableComponentKey/2 ranks document=0, document_format=1, resource=2, annotation=3, node_binding=4, child_list=5, lifecycle=6, trash_membership=7, policy=8, registry=9, period_scope=10, replica_registry=11, conflict=12. The document_format component contains exact ManagedDocumentFormatBinding/1 bytes and its ComponentImage version is bindingRevision.

```text
0 document
1 document_format
2 resource
3 annotation
4 node_binding
5 child_list
6 lifecycle
7 trash_membership
8 policy
9 registry
10 period_scope
11 replica_registry
12 conflict
``` PinRef/2 and ComponentImage/1 do not change.

InstallationNotice/3 and ContentCompletionProof/4 are the new-current component family. Historical Notice2/CP3 are not expanded in place.

Notice3 changes only the portable component-key decoder from PortableComponentKey/1 to /2. It otherwise inherits the fixed-parent Notice2 contract exactly: components is non-empty, fixed-rank/canonical-key sorted and unique; the notice is frozen by the original plan and durable before the first portable-current installation; baseFrontier remains the original plan baseline; and the notice contains no not-yet-sealed ChangeId, receipt, credential, approval, Money, or execution authority.

CP4 changes only the component-key decoder to PortableComponentKey/2 and adds the current ChangeRecord/1 linkage below. Every other CP3 committed/restored validation remains normative: exact Notice key-set equality and ordering, actual installed/sealed after images, complete EntityRef-sorted sourceChanges, the original receipt digest, one-step Frontier advancement, scope_dependencies continuous-chain proof, restored-arm exclusions, and receiver validation of real component bytes and owner versions.

For a fresh managed Document, the source-document component and its document_format component are produced by the same original plan, installed by the same portable decision, and become current only through the same P seal/CP4; there is no half-activated managed source. A format-only transition with unchanged source changes only document_format, has sourceChanges=[], creates no SourceRevisionPlan or managed SourceVersion, and does not advance H(D,E).

### 6.3 ChangeRecord/1

The first normatively closed ChangeRecord family is:

```text
ChangeRecord/1 = {
  format:"weftext.change-record",version:1,
  decisionKey:DecisionKey/2,changeId:ChangeId/1,
  installationNotice:{format:"weftext.installation-notice",version:3,byteLength:Counter,sha256:"64-lowercase-hex"},
  completionProof:{format:"weftext.content-completion",version:4,byteLength:Counter,sha256:"64-lowercase-hex"},
  frontierBefore:Frontier/2,frontierAfter:Frontier/2
}
```

It is fixed in the same P as the ChangeId and CP4; Notice3 is the pre-install notice frozen by the original plan. ChangeRecord indexes exact Notice/CP/frontiers and never duplicates component or source-change authority. Unknown pre-FC bytes are never relabeled as ChangeRecord/1.

For committed CP4, decisionKey.workspaceRef matches every component/sourceChanges Workspace and changeId.commitDomain equals decisionKey.commitDomain. CP4.components is exactly the corresponding Notice3 key set in identical canonical order, and every CP4 after image is the actual installed and sealed after for that key. CP4.sourceChanges is empty or the complete EntityRef-canonical sorted/unique set of real source-state changes; source-unchanged portable effects do not fabricate entries. Non-absent after values are the managed SourceVersion/2 values produced from the winning source plan and this same ChangeId. receiptDigest authenticates the original receipt bytes for this decision and grants no extra disclosure or execution authority.

frontierBefore is the actual validated Frontier immediately before seal. frontierAfter is exactly frontierBefore plus this ChangeId in decisionKey.commitDomain, with no regression or unrelated rewrite. Under exact policy, frontierBefore equals the original expected/base/notice Frontier. Under scope_dependencies, every added head from Notice3.baseFrontier through frontierBefore and this ChangeId through frontierAfter requires the complete continuous verified ChangeRecord/completion chain and the original frozen dependency proof must still prove every added effect unrelated; vector-number comparison, provider sync state, or current files are insufficient.

Restored CP4 is legal only when every Notice3 component has been safely restored to its original before image without seal. Its closed restored arm contains only decisionKey, baseFrontier, and those component images; changeId, success guarantee/writeProtection/semanticState, frontierBefore/frontierAfter, sourceChanges, receiptDigest, and success semantics are forbidden.

A receiver admits ChangeRecord1 only after strict-decoding the exact canonical Notice3 and CP4 bytes selected by the record digests, validating their versions and cross-fields, obtaining every listed component byte/metadata item, validating each owner decoder/version and ComponentImage against the actual bytes, validating complete production SourceVersion before/after evidence, and proving the continuous causal chain. A document_format component additionally strict-decodes its exact ManagedDocumentFormatBinding/1. Missing bytes, unknown decoder/version, incomplete chain, or Notice/CP mismatch is incomplete/proof_unavailable rather than success. These checks preserve the original authorization, recovery, and error ordering and add no second ledger, CAS, or commit point.

## 7. D3, D4, and D5 current consumers

D3 current native requests use wire13 / D3IdentityInput/13 with InputDescriptor/3. D3DecisionCompanion/2 remains unchanged. D3ResolutionInputUse/2 stores exact InputDescriptor/3; historical /1 stores Descriptor2 only.

D4 current outer qualification consumes Descriptor3/Proof3/15-arm keys; a managed Document semantic operation includes both source(owner) and document_format(owner). Inner RelationReadContext/2, RelationReadBinding/2, occurrence keys, numeric sourceRevision, and Calendar/Registry contracts do not change version merely for this outer dependency.

D5 uses the same current outer proof for native tables, Fields, and strong collection operations. A format-stamp change makes an old preparation stale even when SourceVersion is unchanged. Table/row/cell locators, occurrence keys, RevisionTokenBinding, SourceObservation, and D5 intent grammar remain their existing types.

Trash retains the exact format binding; restore retains the historical binding; purge removes the document_format component in the same strict plan/Notice/P/CP as the owner closure. Retained history is not current qualification.

## 8. D7/D8/D9 current holders

PreparedActionBinding/4 carries current Descriptor3/Proof3/PreparedIntent3. MinimumMapping/3 is not mechanically version-bumped. Query/Action/CEL author syntax is unchanged; qualification/evidence and the product projection consumed by source adapters are what version. EffectManifest/3 and EffectBytes/3 remain the one shared current transport family; historical Plan1/Plan3 stay on their original decoders.

### 8.1 D7 product-projection consumers

D7 outer runtime remains its existing wireVersion2 and QuerySpec/ViewSpec author schemas remain version 1. Current headings scan strict-decodes D2DocumentSnapshot/3, requires its available projection, and emits the existing heading object {owner:NodeRef,title:text,level:int64}; level is D2Heading/3.effectiveLevel and the internal position is that heading's exact current DocumentElementLocator. The authored level remains available to owner-aware editor/export consumers but does not create HeadingRef. Invalid/unavailable product projection fails the whole applicable scan under the retained authorization/error order and cannot be skipped.

The existing body_text source now recursively consumes D2DocumentBody/3. It uses semantic text from D2ProductInline/3; explicit link/citation labels only, resource captions only, list source order, section heading plus children, table TAB/LF rules, and raw payload for literal/source/pass where the D7 rule calls for readable raw. It remains a read projection, never exact source and never writable. New native block/inline arms are handled by their explicit D2 product type; no unknown arm may be flattened through to_s or omitted. A legal arm for which body_text has no semantic-text mapping makes that adapter unavailable rather than shrinking the D2 language.

D7 definition-transfer keeps the complete definitionTransfers/Result9 inner transformation semantics already owned by D7, but a new current D3 submission is D3IdentityOperationRequest/13. Genuine saved/planned wire12 requests, Result/effect history, Locators, and recovery continue under their recorded decoder and are never relabeled as wire13.

### 8.2 D8 document, Draft, visual presentation, and run-in policy

D8 public outer entries remain wireVersion2. Current d8_document.snapshot is exactly D2DocumentSnapshot/3 at the same qualified SourceObservation; an old saved response containing D2 document_snapshot wire2 remains historical data only and is not accepted as a new current read. Current valid Draft projection derives metadata/body/origins from D2DocumentPayload/3 and D2DocumentBody/3. It may remove locator members only where the existing Draft contract explicitly says so; it may not omit an unfamiliar legal block/inline arm. A visual control may be unavailable, but exact Source read/edit/save remains the lossless fallback. Invalid D2 source retains the existing authorized invalid-Draft/repair behavior and does not leak partial semantic projection.

Workspace run-in default is owned by D8 as the protected D8WorkspacePresentationPolicy/1 record in SCHEMAS §6.4. Every active Workspace has exactly one record, initially revision=1 and defaultPresentation=separate. Changing it requires the existing Workspace policy_admin authorization, expected revision equality, checked increment, and one protected configuration update; it is not a Document source edit, D3 identity decision, or second author source. Ordinary readers do not need policy_admin merely to consume the already-authorized current presentation setting together with an authorized Document. An unavailable/corrupt policy record makes implicit-default presentation unavailable rather than silently choosing a host default.

Per-source roles override the Workspace default without mutating it. Exactly run-in selects RunIn when the first eligible paragraph is semantically adjacent under the existing explicit-role trivia rule; exactly separate selects Separate; both produce role_conflict and Separate fallback; neither uses the current Workspace default. With neither role, defaultPresentation=run_in applies only to the existing implicit-default physical-adjacency rule, so an intervening blank/comment keeps Separate. No eligible paragraph is Separate. Enable removes separate and ensures run-in; Disable removes run-in and ensures separate; Use Default removes both. These are ordinary source-role edits through D8 and never write D8WorkspacePresentationPolicy/1.

Heading and first paragraph remain distinct source ranges and distinct D2 semantic nodes for all four cases. RunIn is a D8PresentationResult/1 only; synthetic visual joining is not reparsed and has no source scalar. The D8DocumentRenderBinding/1 cache input freezes SourceObservation, DocumentFormatCurrentQualification, D2 snapshot pin, the exact presentation-policy binding, and renderer profile digest. A presentation-policy revision therefore invalidates a current D8 render cache. Changing only that policy does not change source bytes, SourceVersion, heading identity, outline, or authored roles.

### 8.3 D9 semantic/rendered export and exact preparation

D9 current export uses the closed ExportPlan/3 family in SCHEMAS §6.5. Exact AsciiDoc source export selects target asciidoc_source, consumes authorized exact source bytes from ExportInputCatalog/2, needs no product semantic projection, no run-in policy, and no renderer/provider route. Provider unavailability therefore cannot make legal AsciiDoc syntax invalid or block an authorized exact-source export.

A document target html, pdf, docx, or odt requires D9DocumentRenderBinding/1: exact current D2DocumentSnapshot/3, ManagedDocumentSemanticQualification/1, D8WorkspacePresentationPolicyBinding/1, and the route/profile evidence used to build the immutable output. HTML consumes the same deep-heading/run-in projection as D8. DOCX/ODT map effective heading levels 1–9 to their explicit heading structure when the selected profile supports it; effective levels beyond that, and any target limitation on run-in/layout/styles/fonts/accessibility, appear in the complete ExportLossReport rather than causing a D2 syntax rejection. PDF is a controlled route over the frozen semantic/render inputs and likewise reports target-specific losses. Missing/unsupported provider or route is export unavailable; it never rewrites the source or projection.

ExportPlan/3 freezes input domain/catalog/order, content selection, complete render projection, source-format qualification, run-in presentation-policy binding where applicable, exact template binding, route and every provider/profile/version step, style-bundle versions, registered generation-policy binding, target, initial loss report, output budget, protected destination intent, DependencyProof/3/ObservationScope, all evidence pins, and exact staged output pins/digests. D9ExportConfirmation/1 separately binds the planToken and complete D9ExportLossChoice/1 set; choices never mutate the plan or convert a blocking loss to success. PublicationReceipt/3 records the exact plan-selected target/route/template/styles/presentation policy, original report/choices, destination display, and actual published output digests.

The same retained D9 permission and state machine remains load-bearing: establish potential scope and authorization before sensitive reads; recheck current authorization at inspect, confirmation, publish, and delivery; never turn unreadable into none; never mix Query authorization generations; create-only external publication retains its durability/unknown-outcome rules; Save-as-Resource is a separate D3 create_resource preparation and receipt. Source/format/route/template/Query/authorization changes invalidate an unpublished plan as their original owner requires. A later Workspace presentation-policy change does not mutate an already prepared plan: that plan continues with the exact frozen policy binding, while a newly prepared export consumes the new current revision.

### 8.4 Fixed-parent direct-holder dispatch and historical boundary

The replacement router explicitly covers the current statements in D2 implementation impact, D7 Definition Transfer, D9 Import IR, and D9 Acceptance Matrix. D2's historical Profile-v2 implementation prohibitions remain immutable snapshot evidence, but current implementation obligations use D2DocumentSnapshot/3 and the complete native-baseline product family above. D9 Import IR retains ImportIR/1, Mapping/1, ConversionInput/2 and all file-safety/mapping/loss rules; only its current outer author submission is D3IdentityOperationRequest/13, while genuine saved/planned wire12 import decisions recover exactly as before. D9 Acceptance S12 is superseded only in its obsolete five-heading-level premise: authored WeftextManaged levels 6–9 are valid and must flow through D2/D7/D8/D9; target-specific unsupported depths require explicit loss/degradation, not source rejection.

No global text replacement upgrades historical wire12, document_snapshot wire2, or ExportPlan/1-/2 records. Saved/planned/unknown recovery runs before current producer gates and retains original bytes, permissions, errors, confirmation and publication responsibilities.

This batch deliberately does not close the Annotation Value/4 mutation/revision/slot path, the D8 Annotation edit-intent successor, D7 replace/delete/insert suggestion-action mapping, or the AsciiDocInlineBody alias-registry issue. Those remain the explicitly separate Annotation-mutation batch.

## 9. Independent portable JSON Annotation model

Annotation is a portable current value under D3 AnnotationRef identity and is not embedded in Document source. Every root/reply is a separate AnnotationRef; a thread is mechanically the same-owner reply_closure and cycles reject. Concurrent fresh replies may coexist; concurrent edits to one Annotation use revision/CAS/ConflictRecord rather than timestamp LWW.

### 9.1 D3-Annotation-Value/4

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
```

Unknown/missing/duplicate members, illegal nulls, and non-UTF-8 reject. Labels are at most 64 exact-unique entries, each 1..128 UTF-8 bytes; body source is at most 65536 bytes.

Purpose invariants: a root comment requires a nonempty body and no suggestion; a root mark requires appearance and no suggestion; a root suggestion requires Suggestion/3 and may use body as review rationale; all root review states are open|resolved. A reply is same-owner, acyclic, purpose=comment, suggestion=null, and reviewState=not_applicable. Resolve/reopen changes only the thread root. Target resolution and review state are independent.

### 9.2 D3-Annotation-Target-Projection/1

The only identity/locator projection is the closed union:

```text
document        {kind:"document",owner:NodeRef}
document_element{kind:"document_element",locator:DocumentElementLocator}
document_range  {kind:"document_range",locator:DocumentRangeLocator}
resource        {kind:"resource",resourceRef:ResourceRef}
resource_region {kind:"resource_region",locator:ResourceRegionLocator}
```

Each variant allows only those fields. The owner/locator/resource must match AnnotationRef.owner. AnnotationRef, a bare target NodeRef, AuthorAnchorAddress, and cross-owner locators/refs are invalid. Outer wire and Projection/1 materialize bijectively; path/title/hash/ambient owner cannot fill missing identity.

### 9.3 Annotation inline body and the single evaluation profile

The exact closed shapes of `AnnotationInlineBody/1`, its compatibility schema alias `AsciiDocInlineBody/1`, and the single `AnnotationInlineProfile/1` are normative in [SCHEMAS.md §7](SCHEMAS.md#7-annotation-closed-values). SPEC no longer maintains a second literal definition of that profile.

`AnnotationInlineBody/1` retains its four-member portable data shape: format/version/languageBaseline/source; the body is not a processor profile. The evaluation profile fixes the exact Ruby 2.0.26 commit, doctype=inline, processorBackend=`html5-semantic/1`, secure safe mode, and the closed resource/effect limits. The short baseline literal stored in the body identifies the portable source-data version and must correspond to the profile's commit-qualified 2.0.26 baseline; it cannot select another implementation version.

The complete source must form exactly one paragraph, with soft wraps and trailing whitespace allowed. A second paragraph, heading, list, delimited block, table, or block macro is `invalid_annotation_body` rather than silently ignored. Fixed-2.0.26 inline strong/emphasis/mono/mark/roles/URL links/xref/STEM/footnote and similar inline constructs remain available. n1/r1/weftext-cite/carrier/query/view/deep-heading/run-in managed adapters are disabled in this body profile.

### 9.4 Appearance, actor, and time

```text
AnnotationAppearance/1 = {
  mark:"highlight"|"underline"|"squiggle"|"strike",
  theme:"yellow"|"red"|"green"|"blue"|"purple"|"pink"|"gray"
}

AnnotationActorSnapshot/1 = {
  kind:"annotation_actor_snapshot",version:1,originWorkspaceRef:WorkspaceRef,
  authentication:"workspace_authenticated_origin"|"device_local_unverified"|"imported_unverified",
  displayName:text
}

AnnotationTimeSnapshot/2 = {
  instant:RFC3339-with-offset,
  producer:"prepare_server_clock"|"prepare_device_clock"|"imported_unverified"
}
```

Actor/time fields are attribution and never LWW authority. The caller cannot supply the four attribution fields at creation. Trusted preparation injects creator/authoredAt and sets lastEditor=creator and editedAt=authoredAt, freezes them in the original PreparedIntent, and does not resample on replay. Later mutation preserves creator/authoredAt byte-for-byte and Core rewrites lastEditor/editedAt. Same-Workspace copy preserves review attribution; trusted cross-Workspace transfer preserves originWorkspaceRef; ordinary untrusted import is imported_unverified.

### 9.5 Suggestion/3

```text
Suggestion/3 = {
  version:3,
  kind:"replace"|"delete"|"insert",
  state:"pending"|"accepted"|"rejected",
  confirmation:"confirmed"|"needs_reconfirmation"|"not_applicable",
  targetBasisSha256:"sha256:<64 lowercase hex>",
  expectedText:null|{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  pointAffinity:null|"left"|"right",
  replacementSource:null|text
}
```

Pending uses confirmed|needs_reconfirmation; accepted/rejected use not_applicable and are terminal. The basis is exactly SHA256(Weftext-Suggestion-Target-Basis/1 + NUL + D3-CJ/3(complete stored target)); quote/prefix/context/display text are excluded. Replace uses a nonempty range, required expectedText, required replacementSource (which may be empty), and null affinity. Delete uses a nonempty range, required expectedText, null replacementSource and affinity. Insert uses a zero-width range, null expectedText, nonempty replacementSource, and left|right affinity.

## 10. Annotation targets and authorization

### 10.1 Source target

A source target binds its real owner, basis production SourceVersion, byte range or point affinity, expected bytes/context, and source provenance. Root-authored text belongs to the root Node; a managed included span belongs to the included Node. Unmanaged/network/synthetic multi-origin text cannot directly mint a durable exact target.

Resolution states distinguish exact, mapped, candidate, ambiguous, orphaned, and unavailable.

```text
exact
mapped
candidate
ambiguous
orphaned
unavailable
``` Candidate/fuzzy/context search never authorizes a source write; repeated text with multiple candidates is ambiguous rather than “first” or “nearest.”

### 10.2 Media target

PDF/image regions bind exact ResourceVersion plus normalized rectangle. Audio/video uses rational timebase/ticks with [start,end). Provider unavailability preserves existing raw annotation data but blocks creation of a new exact region when exact interpretation cannot be proven.

### 10.3 History and authorization

Historical quote/context reads require both Annotation disclosure and the corresponding historical-source disclosure. Bytes already lawfully delivered in a raw backup are not retroactively erased, while current online APIs continue to apply current authorization.

## 11. SourceTransform compiler and exact mapping

### 11.1 Total compilation

PortableTransformCompilation/1 is total: representable events + afterSourceSha256 or an unavailable reason from the closed set provenance_gap, unsupported_transaction, invalid_utf8_boundary, generated_cross_anchor_edit, boundary_slot_unrepresentable, provenance_cycle, after_replay_mismatch, payload_digest_mismatch.

```text
PortableTransformCompilation/1 =
  representable{events:[SourceTransformPortableEvent/3...],afterSourceSha256}
| unavailable{reason:
    provenance_gap|unsupported_transaction|invalid_utf8_boundary|
    generated_cross_anchor_edit|boundary_slot_unrepresentable|
    provenance_cycle|after_replay_mismatch|payload_digest_mismatch}
``` An unavailable transform does not by itself block ordinary source save.

SourceTransformPortableEvent/3 contains only replace and insert, all in original-before UTF-8 byte coordinates. Deletes are replacements whose replacementByteLength is zero. Every replace satisfies 0 <= startByte < endByte <= beforeByteLength, and startByte/endByte are UTF-8 scalar boundaries. removedByteLength equals endByte-startByte and removedSha256 equals SHA-256 of exactly beforeBytes[startByte:endByte]. Every insert satisfies 0 <= atByte <= beforeByteLength and atByte is a UTF-8 scalar boundary. A zero-length insert is non-canonical and is omitted. Generated-edit provenance folds into the originating event where representable; overlapping source replacements form one maximal replacement island; an insert strictly inside such an island is folded into that island or compilation is unavailable. Same-point inserts are merged in transaction order, so at most one insert remains at a point. Boundary insertions remain separate.

The canonical event array is ordered by original coordinate. A replacement whose interval ends at p appears before the event at p; at the same starting coordinate p, the unique insert@p appears before the replacement starting at p. Thus the boundary order is left replacement ending at p, unique insert at p, right replacement starting at p. Replacement source intervals are pairwise non-overlapping after maximal-island formation.

`generatedOutputSpan(E)` is computed without content search by one replay cursor over the exact before and after pins. Initialize beforeCursor=0 and afterCursor=0. For each canonical event E let q be E.startByte for replace or E.atByte for insert; require q>=beforeCursor, require the unchanged bytes beforeBytes[beforeCursor:q] to equal afterBytes[afterCursor:afterCursor+(q-beforeCursor)], then advance both cursors by that unchanged length. E's generatedOutputSpan is [afterCursor, afterCursor+E.replacementByteLength); it must be within afterBytes, its exact slice must match replacementByteLength and replacementSha256, and afterCursor advances to the span end. For replace, beforeCursor then becomes endByte; for insert it remains q. After the last event, the remaining beforeBytes[beforeCursor:] must be byte-equal to the remaining afterBytes[afterCursor:]. The resulting complete after bytes must match afterSourceSha256. Any failed boundary, length, digest, ordering, unchanged-segment, or final replay check makes the compilation/evidence invalid; a receiver never trusts a redundant field instead of the bytes.

### 11.2 Mapping

For replacement R=[a,b) with replacement length Lr, define delta(R)=Lr-(b-a) as a signed integer. For insertion I at q, len(I)=replacementByteLength. All sums below use checked signed arithmetic and the final mapped coordinate must remain within the replayed after-source byte range.

For a non-zero range [s,e), real replacement overlap or an insertion strictly inside the range stops exact mapping. Otherwise:

```text
s' = s + sum(delta(R), b<=s) + sum(len(I), q<=s)
e' = e + sum(delta(R), b<=e) + sum(len(I), q<e)
```

For point p, a replacement satisfying a<=p<b stops mapping; otherwise add deltas ending before/at p, inserts before p, and inserts at p only for right affinity.

```text
p' = p + sum(delta(R), b<=p) + sum(len(I), q<p)
     + (sum(len(I), q=p) when affinity=right else 0)
``` Each link in a transform chain independently revalidates source version, signature, trust cut, and result.

### 11.3 Frozen plan

CoreSourceEditPlan/2 freezes the /3 events, exact before observation, exact after pin, and TransformEmissionPlan/1.

```text
CoreSourceEditPlan/2 = {
  decisionKey,ownerNodeRef,beforeObservation,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin,transformEmission:TransformEmissionPlan/1
}
``` Emission is either disabled with `no_exact_core_edit_plan|transform_profile_unavailable`, or required with exact profile, expectedTrustRevision, and expectedTrustKeyId. The caller cannot choose. A winning required plan cannot downgrade during recovery; a disabled plan cannot upgrade. Seal cannot recompile or reorder events.

## 12. Signed transform evidence and outbox

SourceTransformEvidence/2 binds DecisionKey, ChangeId, owner, managed before/after SourceVersions, before/after hashes, the fixed coordinate/affinity profiles, and the exact /3 event array.

```text
SourceTransformEvidence/2 = {
  decisionKey,changeId,ownerNodeRef,before,after,
  beforeSourceSha256,afterSourceSha256,
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}
```

beforeSourceSha256 is a whole-source cross-field, not a free digest or an Event-slice summary. It is computed from the exact complete source bytes identified by evidence.before:

```text
beforeSourceSha256 =
  "sha256:" + lowercase_hex(SHA-256(exact_before_source_bytes))
```

The exact_before_source_bytes are the complete bytes of ownerNodeRef at the managed SourceVersion/2 in evidence.before, recovered from the producing CP/history and retained version evidence rather than from the receiver's current file. Receiver validation reacquires those exact bytes, recomputes the digest, and requires byte-equality before accepting the signed evidence. removedSha256 values, removedByteLength, equality of SourceVersion fields, or successful replay of individual slices never substitutes for this whole-source check. Missing exact-before bytes follows the existing proof/state-unavailable boundary; a proved digest contradiction is an integrity failure. No SourceTransformEvidence schema/version changes.

A required plan must satisfy byte-equality of D3-CJ/3(plan.edits) and D3-CJ/3(evidence.edits).

```text
D3-CJ/3(plan.edits) == D3-CJ/3(evidence.edits)
```

SourceTransformSealSignedBody/1 is exactly the complete SourceTransformSealArtifact/1 with the single signature member removed; it therefore contains format, version, trustKeyId, and the complete SourceTransformEvidence/2 and no other member. The signature lexical form is exactly 86 ASCII unpadded-base64url characters decoding to 64 Ed25519 signature bytes. The selected verification public key is exactly 43 ASCII unpadded-base64url characters decoding to 32 bytes, and trustKeyId is exactly "sha256:" + lowercase_hex(SHA-256(raw public-key bytes)).

The authenticated message is exactly `ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1)`. The transported/stored artifact bytes are exactly D3-CJ/3(the complete SourceTransformSealArtifact/1 including signature); a non-canonical transport representation is rejected even when it parses to equal JSON values. The signature never covers itself, an outbox address, or a later trust cut.

The outbox key is exactly {changeId,ownerNodeRef} and the outbox item pins exact canonical artifact bytes as portable_metadata. A required sealed decision has exactly one item; disabled has zero; two different items for one key are an integrity conflict. Required seal revalidates the frozen exact profile/trustRevision/trustKeyId and usable handle in the original P seal. Recovery republishes the exact pinned artifact by exact key and never recompiles events, searches by current hash/trust/source, or re-signs.

The transform artifact is not a PortableComponent and is not inserted into Notice/CP components. It shares the source decision's one P seal and ChangeId.

## 13. Trust profile, activation, and historical verification

A Workspace has exactly one retained WorkspaceTrustRootDeclaration/1 and one protected WorkspaceTrustAnchor/1. The closed seal profiles are d6_revision_token_seal/1 and d6_source_transform_seal/1. Historical WorkspaceTrustDeclaration/1 remains byte-exact revision-token-only. WorkspaceTrustDeclaration/2 is the current profile-discriminated successor; WorkspaceAuthorizationBundle/2 may contain a byte-exact /1 prefix followed by a /2 suffix, and the chain never returns to /1 after the first /2.

Declaration2 revision 1 uses predecessor `{kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}` with the complete retained fingerprint object, not a bare digest. That fingerprint is byte-equal to the one recomputed from the retained root declaration and stored in the unique protected WorkspaceTrustAnchor/1. Later predecessors use the exact previous declaration revision and SHA-256 of its D3-CJ/3 bytes. rootKeyId is never accepted in the predecessor fingerprint slot.

Declaration2 publicKey is exactly 43 ASCII unpadded-base64url characters decoding to 32 Ed25519 bytes. Every Ed25519 signature member is exactly 86 ASCII unpadded-base64url characters decoding to 64 bytes. Each trustKeyId/newTrustKeyId equals `"sha256:" + lowercase_hex(SHA-256(raw_32_byte_public_key))`; uppercase hex, padded base64, alternate serialization, or caller-provided key bytes outside the closed prepare protocol are rejected.

For authorize and rotate, possessionSignature authenticates exactly `ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2)`. The PoP body is the closed projection of the containing declaration's workspaceRef, revision, predecessor, decisionKey plus the exact action commitDomain/profile and the new key tuple; rotate normalizes newTrustKeyId into the body's trustKeyId member. An authorize_fresh TrustConflictOutcome/2 uses the same PoP domain/body reconstructed from the enclosing Declaration2 common fields plus that exact outcome's commitDomain/profile/key tuple. A PoP cannot be transplanted to another declaration revision, DecisionKey, domain, profile, or key.

An ordinary rotate uses mode="ordinary" and continuitySignature is the replaced key's Ed25519 signature over exactly `ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2)`. That closed body contains the declaration common fields and the complete rotate action except continuitySignature and rootSignature, including old/new key IDs, publicKey, possessionSignature, and mode="ordinary". For mode loss_recovery or compromise, continuitySignature is the literal `"not_required"`; no old-key signature is accepted or required. Revoke uses the separate closed mode set administrative|loss|compromise.

rootSignature authenticates exactly `ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2)`, where WorkspaceTrustDeclarationSignedBody/2 is the complete Declaration2 with only rootSignature removed. Thus the root binds the full action, PoP/continuity value, conflict outcomes/carries, predecessor, and DecisionKey.

Current unseen ordinary trust administration uses the closed wireVersion3 add/rotate/revoke prepare successors in SCHEMAS §9. Their profile is SealProfileId/1, so both revision-token and source-transform keys have a real public producer. The request never carries public/private key material: Core generates add/rotate keys and PoP inside the admitted secure store. The gate remains current workspace policy_admin, exact expected trust revision, current disclosure, anchored root, and usable root handle. Rotate uses ordinary|loss_recovery|compromise; revoke independently uses administrative|loss|compromise. The operation updates only the existing policy/WorkspaceAuthorizationBundle component through the original planning/install/single-P/CP4 path; it creates no second trust ledger, Boolean validator, CAS, or commit point. Genuine saved/planned wireVersion2 trust requests retain their exact decoder/recovery and are never rewritten to /3.

Declaration1 activation remains under its original DecisionKey -> CP3/public-history rule. Declaration2 activation is rederived through its DecisionKey -> committed CP4 + ChangeRecord/1 -> exact policy after-image that first appends the declaration sequence. Same-DecisionKey declarations share one activation ChangeId and are all-in/all-out in history_at(C). TrustConflictCarry/2 rederives its origin from the original Declaration2 and original CP4/ChangeRecord; a later resolver cut never replaces the origin cut.

A normal SourceTransform receiver validates the producing ChangeRecord/CP4 and uses C=CP4.frontierBefore for historical key verification. Later ordinary rotation does not invalidate an artifact valid at C. Compromise compares causal order between the artifact seal ChangeId and the original compromise activation ChangeId; causal-concurrent or later old-key seals fail regardless of arrival order. DomainSealKeyHandle/1 may continue new revision-token signing only while its exact Declaration1-authorized key remains current, safe, and usable; it never gains transform authority or is reencoded as Handle2.

### 13.1 Dual-profile policy-conflict resolution

The public d6_conflict_prepare request remains the fixed-parent wireVersion3 request and the policy_bundle_choice JSON member names remain selected, policy, and freshAuthorizations. The current unseen policy arm uses FreshDomainAuthorizationSpec/2 from SCHEMAS §9, whose profile is SealProfileId/1. Source-merge and choose-source-head arms remain on their fixed-parent contract. This is not a new submit surface: conflict_resolve, workspace policy_admin, subject disclosure, exact expectedKey, the original error order, the one winning planning CAS, the original d6_commit_request/2, and the one final P remain unchanged. A genuinely saved/planned policy resolution that can be proved to have been stored under an earlier owner descriptor restores that exact request, descriptor, pins, handles and recovery state and is never reparsed as the current successor. This candidate asserts no deployment that would justify a synthetic migration.

Every expectedKey head is validated before branch contents are trusted. A head whose retained completion proof strict-decodes as ContentCompletionProof/3 is a historical CP3 head and its policy after-image must strict-decode WorkspaceAuthorizationBundle/1. A head whose retained completion proof strict-decodes as ContentCompletionProof/4 is a current CP4 head: the exact ChangeRecord/1, Notice3/CP4 linkage, policy ComponentImage bytes/version and continuous chain must validate, after which the policy after-image is dispatched by its own exact version as WorkspaceAuthorizationBundle/1 or /2. The unchanged WorkspaceAuthorizationBundleAddress/1 still selects exactly one head and its authorizationRevision, trustRevision, byteLength and SHA-256 must match the exact selected bundle bytes. An unknown proof/bundle version, CP3 carrying Bundle2, missing ChangeRecord for CP4, mismatched address, different Workspace/root fingerprint, or incomplete common history fails through the existing unavailable/integrity boundaries; no decoder fallback, current-host guess, arrival order, or LWW is permitted.

For each validated head the resolver normalizes both SealProfileId/1 profiles. Bundle1 contributes only the revision-token history and has source-transform state none. Bundle2 folds its exact mixed Declaration1 prefix and Declaration2 suffix. The effective compromise set starts from the longest common trust prefix and recursively unions every legal losing and selected branch fact. Carry1 is verified exactly under its fixed-parent Declaration1/CP3 contract. Carry2 is verified from the original Declaration2 bytes and original CP4 activation cut. A direct Declaration2 revoke with mode compromise maps compromisedTrustKeyId to action.trustKeyId; a direct Declaration2 rotate with mode compromise maps it to action.oldTrustKeyId. originDeclarationDigest hashes the complete canonical Declaration2 including rootSignature. Carry2 factId is:

```text
body = {
  workspaceRef,commitDomain,profile,compromisedTrustKeyId,
  originAction,originDecisionKey,originDeclarationRevision,
  originDeclarationDigest,originActivationChangeId
}

factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL || D3-CJ/3(body)
    )
  )
```

originActivationChangeId is always rederived from the original Declaration2 DecisionKey through its committed CP4 + ChangeRecord/1 and the exact Bundle2 after-image that first appended that declaration. A later resolver ChangeId never substitutes. An inherited Carry1 or Carry2 is accepted only after recomputing its version-specific factId, validating the named original declaration/root signature and direct compromise action/key mapping, rederiving its original activation cut, and validating every carrying resolver link. The fold is recursive. The effective union is canonical ASCII factId order; byte-equal duplicates collapse, while the same factId with non-byte-equal canonical bytes is integrity_conflict. Facts from an unselected branch remain in the union. A resolver cannot make a compromised key safe by choosing another branch or by carrying the fact at resolver time.

affectedDomainProfiles is the union of all domain/profile pairs whose normalized head state differs and all pairs named by the effective compromise union. freshAuthorizations is sorted/unique by canonical bytes and may name either seal profile, but only an affected pair whose selected state is none or whose selected current key is unsafe under the effective union. A safe unrelated current key cannot be rotated through conflict resolution. If no trust resolution is needed and freshAuthorizations is empty, a policy-only result preserves the selected bundle family and trustRevision. Otherwise the current resolver emits exactly one WorkspaceTrustDeclaration/2 resolve_conflict declaration. A Bundle1 selection becomes a Bundle2 result by retaining its exact Declaration1 prefix and appending that Declaration2; a Bundle2 selection appends Declaration2. The declaration revision is selected.trustRevision+1; conflictId is byte-equal to the request conflictId; selected is byte-equal to the request WorkspaceAuthorizationBundleAddress/1; predecessor hashes the exact selected last declaration; resolvedHeads is the complete sorted expectedKey.heads; inheritedCompromises is the canonical effective union minus facts already effective on the selected chain; and outcomes is complete for every affected pair. TrustConflictOutcome/2 uses keep_current only for a selected current key not made unsafe by the effective facts; otherwise it is none or, when the exact affected pair was explicitly requested, authorize_fresh.

Every authorize_fresh outcome uses a newly generated protected DomainSealKeyHandle/2 and the exact PoP/2 body/message already frozen above. The containing Declaration2 root signature therefore binds the profile, new key, PoP, mixed inherited carries, complete outcomes and predecessor. The proposed Policy/3 remains complete: when byte-equal to selected.policy its policy revision is preserved; otherwise it must be the legal checked successor under the original policy_admin rules. The resulting bundle increments authorizationRevision once; trustRevision increments once only when the Declaration2 is appended. The current portable resolution uses the original current Notice3/CP4/ChangeRecord path and one final P; staged fresh handles become usable only with that same committed transition. No losing branch bytes are rewritten.

The current unseen policy arm is a complete owner-input/preview/recovery chain rather than only a Plan2 value. OwnerInputBinding/2 has protocolOwner=D6 and ownerKind=d6_conflict_resolution/3; current InputDescriptor/3.intentKind is the same controlled value, guarantee=managed_atomic, frontierPolicy=exact, saveProfile=control_only, and sourceInputs is empty. canonicalDescriptorBytes is exactly D3-CJ/3(ConflictResolutionInput/3). Source_merge and choose_source_head do not use that owner version: their ownerKind/intentKind remain d6_conflict_resolution/2 and their exact owner descriptor/derived plan/preview remain ConflictResolutionInput/2, ConflictResolutionDerivedPlan/1, and ConflictResolutionPreview/1, while their outer unseen D6 carrier is the current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family.

ConflictResolutionInput/3 contains the complete branchEvidence plus ConflictResolutionPolicyDerivedPlan/2. Plan2 contains the per-head CP3/CP4 and Bundle1/2 dispatch, selected bundle version/pin, complete per-head TrustConflictCarryValidationEvidence/1, effective/inherited Carry1|Carry2 sets, complete Outcome2 array, and result bundle version/pin. For each expected head and every fact used in that head's effective fold, carryEvidence retains the direct original compromise activation plus every resolver carrier actually traversed. Each hop pins the exact ChangeRecord, completion proof, and policy bundle used to validate that declaration; version-1 hops retain their historical CP3/Bundle1 activation evidence and version-2 hops use ChangeRecord1/CP4/Bundle2. Thus an original Carry activation cut or intermediate carrier cannot be reconstructed from a later current bundle, a digest alone, Derived Index, or resolver time.

OwnerInputBinding/2.pinRefs for the current policy arm is exactly the canonical sorted/unique union of every branchEvidence changeRecordPin, completionProofPin and present policyBundlePin; selectedBundlePin; resultBundlePin; and every changeRecordPin, completionProofPin and policyBundlePin in every carryEvidence origin/carrier hop. Duplicate equal PinRefs occur once. The policy arm has no branch sourcePins. DependencyProof/3 evidence remains in its own owner and is not omitted from PreparedIntent retention merely because it is not duplicated into OwnerInputBinding.pinRefs.

The immutable owner preview is closed ConflictResolutionPreview/2. Its conflictId, expectedKey and resolution are byte-equal to Input3; branchEvidenceDigest is SHA-256 of D3-CJ/3(the complete branchEvidence array); its derivedPlan is byte-equal to the complete Plan2, including headEvidence, carryEvidence, mixed carries, Outcome2 and result bundle version/pin. PreparedIntent/3.previewBinding binds exactly that Preview2 and its protected canonical preview pin. For this policy arm, pinDirectory is the canonical duplicate-free union of OwnerInputBinding.pinRefs, every DependencyProof/3 evidence pin, the exact preview pin selected by previewBinding, and every proposal/before/after/recovery PinRef named by installationPlan. There are no SourceObservation pins because sourceInputs is empty. The one planning CAS freezes InputDescriptor3, OwnerInputBinding2, Preview2, pinDirectory, installationPlan, staged fresh-key associations and resulting bytes together; final submit remains only d6_commit_request/2 and the one final P remains the only commit point.

Receiver admission replays the frozen Input3/Plan2/Preview2/pin set, validates every head proof/bundle dispatch, every retained Carry origin/carrier hop, every root/declaration signature and predecessor, recomputes Carry1/Carry2 unions and fact IDs, verifies every authorize_fresh PoP/2, requires byte-equal result-bundle bytes and the same-decision CP4/ChangeRecord/conflict-record transition, and rejects any omitted losing-branch compromise. Planned recovery restores those exact descriptor/preview/pins/handles and never derives a substitute from current state. Saved/planned/unknown records proven to belong to an older owner/outer carrier restore their original request, Input2/Plan1/Preview1, pins, handles and OperationId before unseen-current dispatch. No deployment migration is inferred from the existence of these current successors. A legal current dual-profile conflict with complete evidence and authority therefore has the positive path above without creating a second ledger, CAS or submit.

A required branch example is transform KT1 split concurrently into ordinary KT1→KT2 and compromise KT1→KT3. Selecting the ordinary branch does not erase the losing branch's original KT1 compromise fact or move its cut to the resolver. KT2 may be retained only if it is independently safe under the complete fold. Where the selected transform state is none or its selected current key is unsafe, a requested source-transform FreshDomainAuthorizationSpec/2 produces a fresh transform key and a valid Outcome2 instead of forcing the Workspace to remain permanently unresolved.

### 13.2 Replica registration current producer

d6_replica_register_prepare retains its fixed-parent wireVersion2 request and specialized replica_register authority; it gains no profile member and ordinary dual-profile trust add/rotate/revoke is not an authority substitute. For a current unseen registration, the winning prepare mints one fresh ReplicaEpoch and the joining secure store generates exactly two fresh DomainSealKeyHandle/2 values for that replica CommitDomain, both staged, in fixed profile order revision-token then source-transform. Caller JSON never supplies either key.

The same frozen plan appends exactly two WorkspaceTrustDeclaration/2 authorize declarations in that profile order under one DecisionKey. The first declaration is selected trustRevision+1 and the second is +2; the second predecessor hashes the complete first Declaration2 bytes. Each declaration has its own PoP/2 and rootSignature. The plan also freezes expectedReplicaRegistryRevision, the selected current bundle/trustRevision, the new ReplicaRecord, both staged handle associations, both declarations, and the exact Bundle2 after-image.

One and only one final P seal publishes the replica_registry after-image and policy/Bundle2 after-image in the same current CP4/ChangeRecord transition and same ChangeId. The two handles move staged→usable together only after that commit is admitted; neither profile can become usable from a partial prefix or from copied public bytes. Receiver admission requires that same CP4 to prove both component transitions, the exact ReplicaEpoch/CommitDomain, the fixed two-profile order, Declaration2 predecessor chain, same DecisionKey, root/PoP signatures, exact Bundle2 result and absence of an unresolved policy/trust conflict.

Planned/recovery restores the exact two staged handles and declarations from the winning plan and never regenerates keys. Any actually proved historical replica-registration decision remains under its recorded decoder/bytes and is not retrofitted with a second key. Retirement prevents future signing by the replica under both profiles once the original retire transition is admitted.

WorkspaceBootstrapProfile/4 has the same closed member set and issuer semantics as Profile3 and changes only the fresh-target trust genesis family to WorkspaceTrustGenesis/2. WorkspaceBootstrapPlan/4 retains the D3 allocation chain's canonical lowercase UUID proposalId and all other UUID members. Its creator binding, target Registry binding, series configurations, and period-scope bindings are the closed helper types in SCHEMAS §9; there are no descriptive placeholders.

Plan4 is only the current unseen fresh create_workspace/fork_workspace bootstrap successor. It preserves the original D3 proposal authenticity, issuer/target-custody ordering, one planning CAS, one DecisionKey, and one P seal. Create derives the target Registry from the frozen eligible seed; fork carries and maps the complete source Registry/configuration history at the fixed source cut. Fresh bootstrap uses one root and exactly two Declaration2 authorizations in order: revision 1 revision-token, revision 2 source-transform, same DecisionKey and activation ChangeId, with rev2 predecessor hashing exact rev1 canonical bytes. Both staged handles become usable only after that one seal commits. There is no observable one-profile prefix.

Ordinary managed copy is not bootstrap and keeps its existing scope/configuration mapping rules. Restore/recovery dispatches the exact saved decoder/plan and never injects Genesis2 or upgrades old bytes. continue_workspace/failover preserves current policy, principal mappings, Registry/configuration, and profile trust state instead of re-running bootstrap. This design candidate is not deployed and therefore asserts no migration or dual-write from Plan3; any actually proved historical record continues only under its recorded contract.

Replica registration remains its specialized same-record producer and is dual-profile for current FC operation; generic trust prepare is not a substitute for replica_register authority. Continuation evaluates revision and transform profiles independently as current(K)|none|conflicted_or_unproved. Any unproved profile prevents partial activation of a new signing domain; otherwise the complete sequence is optional old revision revoke, optional old transform revoke, new revision authorize, new transform authorize in fixed profile order under one DecisionKey/CP4 with no observable intermediate prefix.

### 13.3 D6 Storage §9.1 / §9.3 current consumers

The fixed-parent D6 Storage §9.1 single-profile registration/admission prose is superseded only for a current unseen replica registration. The Storage consumer now admits the exact §13.2 transition: one fresh ReplicaEpoch; exactly two fresh staged DomainSealKeyHandle/2 values in revision-token then source-transform order; exactly two Declaration2 authorizations in that order under one DecisionKey; replica_registry and policy/Bundle2 after-images in the same Notice3/CP4/ChangeRecord1 and one final P/ChangeId; and simultaneous staged->usable for both handles only after that complete transition is admitted. Receiver admission cross-validates the active ReplicaRecord, both Declaration2 predecessor/signature/PoP chains, exact fixed profile order, one DecisionKey and exact Bundle2 bytes. Retention/planned recovery preserves the exact staged pair, declarations, component pins and original request and never regenerates a key. A proven historical registration retains its recorded single-profile decoder and bytes. replica_register remains the specialized authority and cannot be replaced by generic trust administration. replica_retire blocks future signing for both current profiles once its original transition is admitted but does not erase historical authorization or take over ApprovalUse, claims, Money, external unknowns, Automation leases, or execution custody. Ordinary replica content and execution-responsibility takeover remain separate contracts.

The fixed-parent Storage §9.3 conflict direct consumer is likewise arm-dispatched. Current unseen source_merge and choose_source_head use current outer InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 but retain exact ConflictResolutionInput/2, Plan1, Preview1, source pins and source semantics. Current unseen policy_bundle_choice uses ConflictResolutionInput/3, Plan2, Preview2, mixed Carry1/2, the exact OwnerInputBinding pin union and PreparedIntent3 preview/pinDirectory closure above. Storage retains every losing branch byte, exact bundle/activation evidence, result bundle pin and protected preview pin until the original last-reference rules permit release. Final write remains the original D6 typed request and one P seal. Saved/planned/unknown work always restores its recorded outer carrier, request, owner descriptor, preview, pins, fresh-handle association and OperationId first; current outer types never reinterpret or migrate those bytes. These replacements cover Storage's current direct-consumer statements only and leave unrelated historical conflict, transport and recovery semantics intact.

## 14. D10 mixed-version control and execution responsibility

### 14.1 Current-owner routing and historical dispatch

For a current unseen/fresh D10 Workspace-control operation, the coordinated chain is D10WorkspaceReadDependencies/2 with DependencyProof/3 -> ControlDependencies/3 -> D10ControlInput/2 -> ControlPrepareBinding/3. A fresh unattended D10 author step uses PreparedActionBinding/4 and EffectManifest/3 / EffectBytes/3 to construct ApprovalUse/2. A fresh current interactive author step uses the exact current PreparedActionBinding/4 or PreparedEditBinding/3 selected by its owner contract. New scheduling uses the current ScheduleSubscription/2 and AutomationOccurrenceRecord/2 families.

This current routing explicitly supersedes the fresh producer clauses in fixed-parent D10 CONTROL-CONTRACT §4 and §7 and D6 Storage §7.2.1, in addition to the already-routed D10 sections. It is not a global search-and-replace of version names. A genuinely saved/planned/unknown record continues under the exact decoder and recovery contract that actually stored it: historical D10AuthorPreparationLink/1, PreparedActionBinding/3, EffectManifest/2 / EffectBytes/2, PreparedEditBinding/2, ApprovalUse/1, ScheduleSubscription/1, ScheduleContinuityWitness/1, ScheduleContinuityStep/1, ScheduleContinuityInvalidation/1, AutomationOccurrenceRecord/1, ControlPrepareBinding/1 or /2, their original pins, canonical request, OperationId, confirmation facts, and recovery evidence remain byte-exact when those are the recorded formats. A current mixed holder may reference those old responsibilities through explicit versioned carriers without migrating, repinning, or regenerating them.

For a fresh core_field_member author preparation, the D10 CONTROL §4 recovery link is D10AuthorPreparationLink/2. Its preparedBindingToken selects the exact current PreparedActionBinding/4 and its request is the original D6 d6_commit_request/2 produced by that preparation. Core atomically saves Link2, the complete PAB4, EffectManifest/3 / required EffectBytes/3 semantic evidence, and required recovery pins before returning the prepared author step or allowing submission. The current §7 mapping performs the existing fresh Field/Entry selection under DependencyProof/3, including document_format whenever a managed Document is semantically parsed, calls the current D7 prepare owner, retains PAB4, validates the complete EffectManifest3/EffectBytes3 and actual MutationFootprint, then constructs ApprovalUse/2. The Standing Approval still cannot select the target, Entry, member path, or requested value, and D6 still owns planning, final seal, saved replay, and the original request. A historical Link1/PAB3/ApprovalUse1 association, once proved, is recovered exactly and is never re-prepared as current.

For a fresh or otherwise undecided current external-consent control preparation, ControlPrepareBinding/3 stores the same ExternalConfirmationRequirement/1 that the fixed-parent semantics require, while the distinct protected ExternalConfirmationRecord/1 continues to own the current confirmed fact. The trusted attended confirmation event, complete immutable preview/intent binding, current principal, trusted time, consent interval, dependency revalidation, non-disclosure, and approval_unavailable/preflight ordering remain unchanged. Confirmation never mutates Binding3. A saved/prepared ControlPrepareBinding/2 retains its original requirement, canonical intent bytes, Dependencies2, confirmation association, result lookup, and recovery; it is not rewritten merely because current unseen prepares use Binding3.

The fixed-parent D6 direct consumer follows the same split. Current unseen D10 control uses D10ControlInput/2, ControlDependencies/3 and D10ControlEffectPlan/2 instead of the fixed-parent /1-/2 producer prose; current unseen automatic author use consumes ApprovalUse/2 plus Link2/PAB4/Effect3. Saved/planned historical associations are recovered before current qualification and are never rewritten merely because current successors exist.

The D6 Storage §7.2.1 current producer likewise starts a fresh scheduling registration only for ScheduleSubscription/2. After the current D10 scheduling gates and finite retention reservation succeed, the same configuration transaction registers the actual Core source/control producer and creates ScheduleContinuityWitness/2 with ScheduleRecurrenceEvidence/2 initial/checkpoint, revision=1, consumedTransition=0, and a fresh producerEpoch. The current witness artifact bytes are exactly UTF8("D6-Schedule-Continuity/2") || NUL || D3-CJ/3(complete ScheduleContinuityWitness/2). Current positive transitions use ScheduleContinuityStep/2 with DependencyProof/3 and the actual current ChangeRecord/1 + Notice3 + CP4 pins; each current step artifact is exactly UTF8("D6-Schedule-Step/2") || NUL || D3-CJ/3(complete ScheduleContinuityStep/2). A vanished, conflicted, invalid, unavailable, or history-gapped current source uses ScheduleContinuityInvalidation/2 with its already-closed D6-Schedule-Invalidation/2 domain. All three current pins use PinRef/2 payloadKind=artifact and retentionClass=recovery, and byteLength/SHA-256 covers the complete domain+NUL+canonical-object bytes. The digest is comparison evidence only; protected producer/subscription provenance and exact generation/epoch/transition continuity still authenticate the witness. binding_changed still requires proved selected-business discontinuity and gap still covers unavailable/unknown history. The producer preserves the fixed-parent atomic inbox, retention, counter, capacity, authorization, compaction, error, and no-reset rules; this successor changes only the current typed evidence/dependency/component family and exact current artifact domains.

D6 producer/fold/compaction/receiver/recovery and the D10 §16 continuityPins consumer dispatch by the authenticated domain before decoding the inner record. D6-Schedule-Continuity/1 and D6-Schedule-Step/1 accept only historical Witness1 and Step1; D6-Schedule-Continuity/2 and D6-Schedule-Step/2 accept only Witness2 and Step2. The same exact /1-versus-/2 rule applies to Invalidation1/Invalidation2. An unknown domain, domain/object version mismatch, alternate prefix, missing NUL, or non-canonical bytes fails closed through the original authorization/disclosure and unavailable/integrity boundaries; no fallback decoder widens /1. continuityPins may contain exact current typed /2 pins, exact historical typed /1 pins, or the complete original typed chain when real retained history spans both, but every element keeps its own original bytes/domain/decoder. Compaction may discard intermediate payload only after the retained exact typed witness/step chain still proves the fixed-parent continuity invariants.

An existing or retired ScheduleSubscription/1 remains a valid historical recovery-retention owner with Witness1/Step1/Invalidation1 and its original /1 pins/producer obligations. It is never background-migrated. The already-defined explicit same-generation Subscription1 -> Subscription2 continue is legal only after complete retained history proves no intervening format/rule/business discontinuity and establishes the current Evidence2/Proof3 cut; otherwise replace creates the new generation. Such continue preserves the semantic generation and every old typed artifact pin; only newly produced Witness2/Step2/Invalidation2 artifacts use /2 domains. It never repins or re-encodes old /1 bytes or resets an invalid generation.

### 14.2 Images, pins, ranges, and effect plans

D10ControlRecordImage/2 exists only for automation and run records whose nested current schema changed. Other record kinds, including planned_approval, external_approval, activation, reservation, external_effect, stop, and supplement=none records, continue to use exact Image1. Image2 cannot be downgraded to Image1, and an old Image1 is never re-encoded merely to enter a mixed holder.

Pin1 continues to authenticate the fixed-parent D10-Control-Record/1 payload. Pin2 authenticates the exact D10-Control-Record/2 prefix plus D3-CJ/3(Image2), with artifact payload kind, the original recovery or approval_money retention class, and exact byteLength/SHA-256. Pin2 never repins Image1.

A mixed record-pin array is ordered by the inner image: canonical binding.ref bytes, numeric binding.revision, usageRevision with none before some, numeric usageRevision when present, then complete canonical image bytes. The logical same-cut identity is binding.ref + binding.revision + usageRevision. Two entries with that identity and byte-equal image are duplicate responsibility and are rejected; two with different image bytes are integrity_conflict. Schema version is never a reason to retain two values for one cut.

Range kind rank is records=0, cost_lineage=1, occurrences=2. The logical range identity is respectively scope plus the complete sorted kind set, CostLayerKey, or Automation Ref. A mixed range array orders by kind rank and canonical logical-identity bytes and contains each logical identity at most once across Range1/Range2. An occurrences range orders its inner records by AutomationOccurrenceKey; V1/V2 values with one occurrence key cannot coexist.

D10ControlEffectPlan/2 changes are sorted uniquely by the complete canonical after.value.binding.ref. A non-none before must have the same binding.ref as after and exactly one matching versioned record pin for the actual stored before schema. Image1 before with Pin2 is invalid. Current Automation configure may legitimately move Image1+Subscription1 to Image2+Subscription2. A true continuation may retain the same subscription generation when the D10 scheduling owner says continue; generation replacement occurs only when that owner rule requires replace. Mixed-holder uniqueness is a snapshot rule, not a requirement to replace every old subscription.

### 14.3 ControlDependencies/3 and the one-cut rule

ControlDependencies/3 retains the complete actual configBindings, usageBindings, authority proof, authorization generations, stopRefs, optional Workspace reads, versioned record pins, and versioned ranges. Config bindings remain canonical by full Ref; usage bindings by full Ref; authorization-generation tokens by canonical token bytes; stop refs by full Ref; record pins and ranges use the mixed rules above.

Every configBinding has the exact matching record image/pin. Every usageBinding has a matching image at the same usageRevision. Every stopRef has its exact stop Image1/Pin1 because stop is not mechanically upgraded. Current range fences, bindings, usage revisions, record images and the protected Workspace evidence used for one preparation come from one real Authority Store barrier. A V1 value captured at barrier A cannot be combined with a V2 value captured at barrier B and called complete. A historical image referenced by current responsibility remains immutable evidence and does not become current configuration merely by appearing in the holder. Missing required old bytes/pins/decoder is state_unavailable after the ordinary disclosure gates; contradictory same-cut evidence is integrity_conflict. No current Image2 may substitute for a required old Pin1.

### 14.4 Claims, Money, and Inventory canonicality

D10ExecutionClaims/2 keeps all mixed responsibilities complete. recordPins use the mixed pin order. prepareBindings are ordered by inner StableControlKey canonical bytes and that key is globally unique across Binding1, Binding2 and Binding3. leaseRuns are ordered uniquely by full Run ControlRef. authorSteps are unique by run Ref plus stepId across Step1/Step2. subscriptions are unique by Automation Ref plus generation across Subscription1/Subscription2. occurrenceRecords are unique by AutomationOccurrenceKey. ranges use the mixed range order. continuityPins are ordered uniquely by pinToken. Each continuity pin is an exact typed D6 schedule artifact and dispatches by its retained D6-Schedule-Continuity/{1|2}, D6-Schedule-Step/{1|2}, or D6-Schedule-Invalidation/{1|2} domain before inner decode; no free proof-map, schema guessing, /1 decoder widening, or cross-version repin is allowed. Wrappers select an exact decoder only; they do not add kind/version/confirmation fields to Binding1, do not LWW, and do not alter original pins or dependencies.

D10MoneyResponsibility/2 orders reservations uniquely by the complete Binding<reservation>/1 canonical key, layers uniquely by CostLayerKey canonical bytes, recordPins and ranges by the mixed rules, and evidencePins uniquely by pinToken. Contradictory values for one reservation identity cannot be represented as two liabilities, and CostLayerTotal remains a verified projection of the complete reservation/attribution set rather than a balance reconstructed from current configuration.

D10ExecutionInventory/2 orders ApprovalUse carriers uniquely by DecisionKey canonical bytes across versions, externalUnknowns uniquely by binding.ref canonical bytes, and stopState uniquely by binding.ref canonical bytes. It includes every responsibility still needed for pending work, recovery, deduplication or irreversible stop, including completed external attempts that remain referenced. Inventory2 is captured only after admission, planning, send, and schedule writers for the old holder are stopped at one real store barrier; mixing old and new values from different barriers is forbidden.

### 14.5 Inventory pin, Record3 equality, store incarnation, and StopCapacity

The Inventory2 artifact is exactly UTF8 D6-Execution-Inventory/2, one NUL byte, then D3-CJ/3(D10ExecutionInventory/2). Its PinRef/2 has payloadKind=artifact, retentionClass=recovery, and byteLength/SHA-256 for those complete prefixed bytes. Inventory1 keeps its historical D6-Execution-Inventory/1 domain and is never repinned or re-encoded as Inventory2.

ExecutionResponsibilityRecord/3 and its pinned Inventory2 must agree exactly: workspaceRef is equal, and the five responsibility payloads approvalUses, claims, moneyLineage, externalUnknowns, and stopState are byte-equal canonical values. ExecutionContinuityProof/2.inventoryPin must select that exact Inventory2. Inventory2.storeIncarnation is byte-equal to ExecutionContinuityProof/2.storeIncarnation, and the protected birth/barrier/fence token mapping must prove that actual store and the same capture barrier; no separate caller-provided store-incarnation proof object is introduced.

Inventory2.stopCapacity is the exact StopCapacity/1 value from the authoritative safety store at that same barrier. StopCapacity/1 remains the fixed-parent {issued,reserved} counter pair and is not duplicated as a Record3 member. A Workspace-only custody move cannot copy those shared counters into a second active store. Either the original authoritative safety store remains the reachable serialized owner, or a complete store handoff fences every affected writer and preserves all target/latch reservations and capacity. Unproved store continuity or safety capacity makes takeover unavailable; zero, a visible subset, or a rebuilt value is never accepted.

Record2/Proof1/Inventory1 retain their historical decoder and payload domain. Record3 is formed only for a real responsibility mutation, checkpoint, or custody handoff. Such a transition strict-decodes every retained historical responsibility, captures one complete mixed Inventory2, pins it once, creates Proof2, checked-increments the record revision when applicable, and preserves executionDomainId. It never mints replacement ControlRefs, requests, approvals, reservations, claims, or a second execution ledger.

## 15. Suggestion lifecycle

Suggestion state is separate from review state. The current model distinguishes pending, needs_reconfirmation, accepted, and rejected, with replace/delete/insert intent and exact target basis/expected bytes or point.

Reject first requires Annotation state disclosure plus annotation_read/write; it does not require target-source read and does not introduce a blind-write profile. Accept requires Annotation and target disclosure, fresh current target qualification, current expected bytes/point, and a fresh D7 prepare. Mapped geometry is only input to that new qualification and cannot revive old PreparedIntent/ActionEvidence.

Explicit reanchor moves pending to needs_reconfirmation. Reconfirmation recomputes the target basis and expected bytes/point at the fresh current target. Accept writes the Document after-image and Annotation accepted state in one D6 seal. Accept/reject racing on one current Annotation revision has one winner. Geometry that maps successfully is insufficient if expected bytes changed.

## 16. Copy/import/export/backup of annotations

Node copy creates fresh AnnotationRefs, rewrites the full reply graph, and rebuilds targets through the real source/resource identity map. Attribution fields remain attribution, but old locators are not copied and positions are never guessed from text. Failure to rebuild a mandatory target/reply closes the typed operation; a byte-preserving raw copy is not typed success.

Import preserves only targets whose owner/provenance can be proven under the new identity map. Otherwise the record remains candidate/unavailable for explicit user repair. Export/Review Bundle includes source/history/media context only when current permissions allow it; renderer convenience never upgrades resolution. Portable backup carries annotation JSON and portable identity/value, not consumable execution authority.

## 17. Error order, recovery, and version boundary

The common order remains: closed decode -> minimum disclosure/capability -> authority/domain/backend/trust -> stable key and saved/planned/unseen -> current target/source/control -> dependency/semantic/budget -> one planning CAS -> install -> one P seal -> output authorization.

Saved results replay under the original owner/version. Planned work restores the original request, descriptor/proof, preparation, pins, Notice, install state, and version basis and is never upgraded in place. Unknown work retains original Approval/Money/claim/external/stop responsibility. Only unseen current work uses Descriptor3/PAB4/Edit3/ExportPlan3/Notice3/CP4/Declaration2, etc. The existence of a decoder or historical design text does not prove a prototype was deployed.

Missing a transform artifact, index row, provider, or strong proof is not a reason for a permanent generic owner_update_required once this coordination is present. Each affected path uses its actual unavailable/conflict error while raw source, Draft, repair, and other independent paths remain available under their existing qualifications.

## 18. Schema-version inventory

Current additions/successors include ManagedDocumentFormatProfile/1, ManagedDocumentFormatBinding/1, DocumentFormatCurrentQualification/1, ManagedDocumentSemanticQualification/1, OracleSemanticWitness/7, CoreSemanticProjection/1, DependencyKey/3, DependencyProof/3, InputDescriptor/3, PreparedIntent/3, PortableComponentKey/2, InstallationNotice/3, ContentCompletionProof/4, ChangeRecord/1, D3IdentityInput/13, D3ResolutionInputUse/2, PreparedActionBinding/4, EffectManifest/3, EffectBytes/3, PreparedEditBinding/3, ExportPlan/3, PublicationReceipt/3, PortableTransformCompilation/1, SourceTransformPortableEvent/3, CoreSourceEditPlan/2, TransformEmissionPlan/1, SourceTransformEvidence/2, SourceTransformSealArtifact/1, SourceTransformSealOutboxItem/1, WorkspaceTrustDeclaration/2, WorkspaceAuthorizationBundle/2, DomainSealKeyHandle/2, TrustConflictCarry/2, TrustConflictOutcome/2, FreshDomainAuthorizationSpec/2, PolicyBundleHeadEvidence/2, TrustConflictCarryValidationHop/1, TrustConflictCarryValidationEvidence/1, ConflictResolutionPolicyDerivedPlan/2, ConflictResolutionInput/3, ConflictResolutionPreview/2, WorkspaceTrustGenesis/2, WorkspaceBootstrapPlan/4, D10WorkspaceReadDependencies/2, ControlDependencies/3, D10ControlInput/2, ControlPrepareBinding/3, D10ControlRecordImage/2, D10ControlRecordPin/2, D10ControlRange/2, D10ControlEffectPlan/2, ApprovalUse/2, D10AuthorPreparationLink/2, D10AuthorStepResponsibility/2, ScheduleRecurrenceEvidence/2, ScheduleSubscription/2, ScheduleContinuityWitness/2, ScheduleContinuityStep/2, ScheduleContinuityInvalidation/2, ScheduleOccurrenceProof/2, AutomationOccurrenceRecord/2, D10MoneyResponsibility/2, D10ExecutionClaims/2, D10ExecutionInventory/2, ExecutionResponsibilityRecord/3, and ExecutionContinuityProof/2.

Key retained types include PinRef/2, ComponentImage/1, OwnerInputBinding/2, ObservationScope/2, D3DecisionCompanion/2, D4/D5 inner selector/locator types, WorkspaceTrustRootDeclaration/1, WorkspaceTrustAnchor/1, RevisionTokenSealVerificationKey/1, LeaseRunUse/1, D10ExternalResponsibility/1, D10StopResponsibility/1, and StopCapacity/1. Historical decoders are never expanded in place.

The current managed-format type family used by this candidate is:

```text
ManagedDocumentFormatProfile/1
ManagedDocumentFormatBinding/1
DocumentFormatCurrentQualification/1
ManagedDocumentSemanticQualification/1
```

## 19. Acceptance and evidence boundary

The itemized design obligations are in ACCEPTANCE.zh-CN.md / ACCEPTANCE.md: 438 core design oracles and 138 actual-owner coordination fixtures, 576 obligations total. They are **unexecuted design obligations**, not implementation test results.

This PR did not install or execute the Ruby oracle, Asciidork, Mermaid CLI, browser/Puppeteer, STEM/PDF providers, product SQLite/storage code, replica/crash/crypto paths, real Automation scheduling, or execution-custody handoff. Author checks are limited to document/JSON/router consistency. Independent design review must bind to the exact stopped PR head SHA.