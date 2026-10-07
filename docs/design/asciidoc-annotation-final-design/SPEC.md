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

CoreSemanticProjection/1 remains test-oracle comparison data and is never a product read/edit/query wire. Current managed AsciiDoc product reads use D2DocumentSnapshot/3 and the closed product family in SCHEMAS §4.4. Exact source remains the sole author authority; product projection, editor maps, indexes, rendering trees and Query values are discardable derived state.

The product family is complete by **observable fixed-Ruby semantics**, not merely by AST shape. Each block/inline arm carries the dedicated fixed-derived facts that its kind exposes, the closed D2NativeAttributeSet/1 for arbitrary legal named native attributes, and D2BlockCommonSemantics/1 for the public block slots that do not reduce to that named map (style/caption/numeral/subs/positional values). This prevents both failure modes: a free JSON escape is forbidden, but legal author attributes or positional semantics are not rejected merely because their names were not enumerated by Weftext.

The required product semantics include, at minimum and without being limited to the examples below:

- section sectname/special/numbered/numeral/caption and distinct authored/effective heading levels;
- source/listing language, linenums, start, indent, tabsize, highlight and line-comment facts;
- quote/verse attribution and citetitle;
- admonition name/textlabel/icon, list style/start/reversed/checklist/interactive/coids, table format/grid/frame/stripes/physical grid positions, and TOC levels;
- quoted inline id/roles, xref refid/path, ref versus bibref anchors, visible/concealed indexterm plus see/see-also, footnote ref/xref state, exact callout guard form, and inline image versus icon semantics;
- image/audio/video target-specific dimensions, timing, poster, preload, playlist/list, theme/lang, controls and other fixed-2.0.26 converter-observable options.

These are parsed once by the fixed native parser/model and projected through one product family. D7/D8/D9 do not reparse exact source to recover missing source language, quote credit, section kind, captions/numerals/substitutions, icon/bibliography-anchor kind, index semantics or media options.

Every semantic value additionally references D2SourceOriginGraph/2. That graph distinguishes authored ranges, authored reference sites, substitutions, generated values, synthetic values and multi-origin results and derives a single writable source only when all retained provenance paths converge on one exact authored range. A flat range list is not sufficient. Structured editing fails closed for generated/ambiguous/multi-origin values, while authorized exact Source read/save remains independently available.

The document-level product is likewise closed over the actual public/converter-observable fixed Ruby 2.0.26 state. D2DocumentMetadata/3 preserves backend/basebackend/filetype/outfilesuffix, safeMode, doctype, complete doctitle/main/subtitle, revision, and all Document#authors name/firstname/middlename/lastname/initials/email members; initials are never guessed from a full name. It also carries the complete final present Document#attributes map instead of pretending a physical header-source-range list is the effective state. Builtin/generated/environment-derived attributes use generated/substitution origins and never fabricate authored physical ranges; headerAttributes is only the real lexical header-entry list. This domain tracks fixed 0b99b39c9df884d4aec13bba45f03cdbab505769 Document/converter consumption. D9/D8 consume the product directly and do not reconstruct it from full names, raw Source, ambient backend, or host defaults.

The final attribute map also preserves the fixed Ruby value **type**, not only its spelling. D2EffectiveDocumentAttribute/1 reuses the closed D2NativeSemanticValue/1 union: absent Hash keys have no entry; present nil/Boolean/String/Integer map to null/Boolean/text/canonical-integer respectively; an Array is legal only as an ordered textArray of String elements. Thus safe-mode-level, max-include-depth and authorcount remain integers, and mannames remains the ordered Array consumed by HTML5 join and DocBook map. ProcessorEnvironment API overrides are already limited to text set-values or null unset actions. Accepted extensions do not gain an arbitrary-object escape: a final Symbol/Float/Hash/nested or mixed Array/opaque object makes the rich product unavailable unless a future explicitly versioned product domain admits it. Core never to_s/joins/JSON-flattens the value and never recovers its type from the attribute name or source; D7/D8/D9 receive the typed value and its same-evaluation origins.

Field-level provenance is mechanically carried by the closed SCHEMAS §4.4 values rather than promised by node-level sourceOrigins. Every D2NativeAttributeEntry/1 carries sourceOrigins; common/per-kind semantic members have field-mirrored origins; independently editable block/inline slots have slotOrigins; sequence slots pair one-for-one in value order. A structured edit may use only the edited slot's own writableSource and must revalidate write authorization/current Observation for that exact SourceOwner plus all evaluation dependencies at the final barrier. A unique authored positive case cannot be made uniformly readonly for implementation convenience. Generated, multi-origin, ambiguous, non-author, or unauthorized-source values remain readable but structured-readonly. Thus style, language, and body in [source,ruby], and id, each role, and child content in [#foo.red], have distinct mechanically associated origins. Consumers may not infer them from a coarse range, path, text, or a second parse.

Every root/include source unit is version-exact. A current managed D2DocumentSnapshot/3 is produced only from a managed_file processor input whose owner is the snapshot owner, and its root SourceUnitBinding/SourceObservation must match that exact input. Managed include units retain the actual SourceObservation/1 used by evaluation; artifact/network inputs retain exact immutable pins. D2DocumentSnapshot/3 freezes the exact AsciiDocProcessorEnvironment/3, its canonical digest, root/include SourceUnitBinding values, ObservationScope and complete DependencyProof/3 at one authorized read barrier. The proof includes every managed source unit and each actual Registry/foreign/authorization dependency used by the projection. A changed included Node SourceObservation, artifact/network pin, processor environment or used dependency makes the old semantic tree stale even when root source and document_format did not change.

A snapshot pin therefore answers “what tree was produced”; its evaluation binding answers “is that tree still current?” D7/D8/D9 validate the complete evaluation binding at their final read barrier. A later barrier is legal only through the existing scope_dependencies continuity proof showing every bound source/environment/control/negative dependency unchanged. Unrelated unavailable global facts do not block a consumer whose closed evaluation dependency set does not use them.

D2Heading/3 keeps authoredLevel distinct from effectiveLevel. WeftextManaged permits authored levels 6–9. The fixed Ruby leveloffset state machine has only the lower clamp, so effectiveLevel is an arbitrary-precision nonnegative canonical integer with no Counter/int64 language ceiling. D7 headings exposes it through the existing TypeSpec {kind:"integer"} canonical decimal value. Resource budget exhaustion is budget_exceeded, never syntax invalidity or numeric overflow. Common 1–9 headings continue through the same query/outline/render path.

Native link/xref/image/citation grammar is parsed first by the one AsciiDoc parser. D2IdentityAdapter/1 is applied only after a native occurrence exists and the Weftext target syntax independently validates. It may attach stable NodeRef, owner-local ResourceRef or citation identity but cannot infer identity from path/title/text/hash. A managed include retains its own versioned SourceOwner/Origin facts; inclusion never changes root Node identity or grants the including Document write authority over the included source.

Legacy Profile-v2 restrictions that rejected otherwise-native open/include/pass/extension constructs remain historical implementation input only. Current fixed-baseline legality comes from the complete parser/product contract above. A legal construct that lacks a rich-editor control remains readable and exact-Source-saveable; the control is unavailable instead of the syntax being rejected or flattened.

Invalid source still preserves the authorized exact source and ordered diagnostics while product projection is unavailable and D2 commit eligibility rejects. Physical decode/source-envelope failures remain D6 errors, not D2 invalid syntax. Repair and Source surfaces therefore remain available under their original authorization.


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
  kind:"d2_document_format_current_qualification",version:1,
  key:DocumentFormatDependencyKey/1,
  stamp:{epoch:Token,revision:Counter},
  componentImage:{state:"present",version:Counter,
                  byteLength:Counter,sha256:"64-lowercase-hex"},
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

D3 current native requests use wire13 / D3IdentityInput/13 with InputDescriptor/3. expectedAuthority remains a genuinely optional inherited member: replica_local create_node/move_node/reorder_node/trash omits expectedAuthority/workspaceProposal/preparationBinding entirely; managed_atomic uses create/continue/existing exactly under the fixed-parent mode matrix. JSON null is never a substitute for absence, and a forbidden member is invalid. D3DecisionCompanion/2 remains unchanged. D3ResolutionInputUse/2 stores exact InputDescriptor/3; historical /1 stores Descriptor2 only.

D4 current outer qualification consumes Descriptor3/Proof3/15-arm keys; a managed Document semantic operation includes both source(owner) and document_format(owner). Inner RelationReadContext/2, RelationReadBinding/2, occurrence keys, numeric sourceRevision, and Calendar/Registry contracts do not change version merely for this outer dependency.

D5 uses the same current outer proof for native tables, Fields, and strong collection operations. A format-stamp change makes an old preparation stale even when SourceVersion is unchanged. Table/row/cell locators, occurrence keys, RevisionTokenBinding, SourceObservation, and D5 intent grammar remain their existing types.

Trash retains the exact format binding; restore retains the historical binding; purge removes the document_format component in the same strict plan/Notice/P/CP as the owner closure. Retained history is not current qualification.

## 8. D7/D8/D9 current holders

PreparedActionBinding/4 carries current Descriptor3/Proof3/PreparedIntent3. MinimumMapping/3 is not mechanically version-bumped. Query/Action/CEL author syntax is unchanged; qualification/evidence and the product projection consumed by source adapters are what version. EffectManifest/3 and EffectBytes/3 remain the one shared current transport family; historical Plan1/Plan3 stay on their original decoders.

### 8.1 D7 product-projection consumers

D7 outer runtime remains wireVersion2 and QuerySpec/ViewSpec author schemas remain version 1. Current headings scan strict-decodes D2DocumentSnapshot/3, validates its complete evaluation binding at the final D7 read barrier, requires available projection, and emits owner/title plus level using the existing D7 arbitrary-precision TypeSpec {kind:"integer"} and canonical decimal value. The internal position is the exact current DocumentElementLocator. An old saved result whose schema says level:int64 remains historical result data; current product scanning never truncates, saturates or rejects a legal large Ruby effective level merely to fit int64.

The existing body_text source recursively consumes D2DocumentBody/3 after the same evaluation-currentness check. It uses semantic text from the complete D2ProductInline/3 family, preserves source/list/table order and uses the product semantic profiles rather than reparsing exact source. A legal arm with no body_text mapping makes that adapter unavailable; it never shrinks D2 syntax or silently to_s-flattens a product arm.

D7 definition-transfer retains the complete definitionTransfers/Result9 semantics. A fresh current D3 submission uses D3IdentityOperationRequest/13 and its exact mode matrix; genuine saved/planned wire12 requests, effects, Locators and recovery continue under their recorded decoder.

### 8.2 D8 document, Draft, visual presentation, and run-in policy

D8 outer document entry remains wireVersion2. Current d8_document.snapshot is D2DocumentSnapshot/3 at the same qualified root observation and includes its exact product evaluation binding/origin graph. D8 validates the complete root/include/environment dependency set before treating a projection/map as current. A changed managed include therefore invalidates the old tree even when root bytes did not change. Invalid D2 source keeps the existing exact Draft/repair behavior and never leaks a partial new projection.

Workspace run-in default is D8-owned shared configuration in Portable Workspace Metadata. It is not Document source, Policy/3 authorization, Registry data, a PortableComponentKey or device-local preference. The current immutable record/head-set contract is SCHEMAS §6.4. D8WorkspacePresentationPolicy/1 remains the single logical current value when one head exists; the /2 record is its immutable portable history carrier, not a second value authority. The head-set carries the owner-specific current observation stamp. Normal mutation and conflict resolution use policy_admin, the exact observed head-set/stamp, one D6 planning CAS and one P decision. That same portable decision allocates the ordinary ChangeId/ChangeRecord and commits exactly one presentation_policy_change owner effect; no second ledger/CAS or Policy/Registry revision is introduced. Two offline successors with the same numeric revision but different canonical record bytes/ChangeIds remain two heads; no revision-number winner, arrival order or LWW rule exists. Multiple heads make only default-dependent presentation unavailable until an authorized explicit multi-parent successor resolves them. Sync/admission validates the retained ChangeRecord/receipt/effect association and ancestry before a head can be current. Saved/planned/unknown work recovers its frozen head-set stamp and never resamples the branch.

SCHEMAS §6.4 fixes the concrete mutation producer to the original D6 stages. SetRequest validates closed decode → presentation-state disclosure → policy_admin → frontier/domain → head-set → exact expectedHeads → retained parent pins/ancestry → semantics/budget. A changing path returns D8PresentationPolicyPrepareResult/2 containing the same operation's D8PresentationPolicyInput/1, Proposal/1, parent headEvidence, PreparedIntent/3, d6_commit_request/2, and preview EffectManifest/3. That PreparedIntent uses the existing control_only Workspace observation scope, saveProfile=control_only, sourceInputs=[], and managed_atomic/strict; it binds only real authorization/control dependencies and parent-record pins. It neither reads/installs author source nor creates a SourceRevisionPlan or author-source mutation; the sole ChangeId is the ordinary portable decision ChangeId allocated only at final P. The planning CAS freezes only Proposal/Input/head stamp/pins/dependencies. Because the committed record contains an activationChangeId that does not yet exist, prepare/planning/staging/Notice/install/verify cannot precreate a committed record, record hash, portable pin, outbox, or ChangeId. Final P revalidates authorization and frozen heads, then checked-allocates C and, in the same P transaction, constructs/encodes D8WorkspacePresentationPolicy/2 from frozen Proposal+C, creates its hash/pin/address, committed owner effect, ChangeRecord/receipt association, and protected D8PresentationPolicyOutboxItem/1. Publication/crash retry after P only replays the exact pinned bytes; receiver admission validates record/pin, activation ChangeId, effect/receipt/ChangeRecord, and complete ancestry. Equal hash, a trusted sender, a Boolean verified claim, or provider latest is not proof. Same single-head/value is a zero-plan/zero-P no_change; a multi-head set requires explicit resolution even when values agree; saved/planned/unknown restores original frozen bytes/pins/request without resampling.

Fresh unseen create_workspace/fork_workspace does not wait for a later SetRequest. WorkspaceBootstrapPlan/4 carries D8PresentationPolicyBootstrapInit/1 with a custody-proved protected empty before and the fixed proposal parents=[], revision=1, defaultPresentation=separate. It uses the original create/fork authority, OperationId, planning CAS, DecisionKey and final P rather than target policy_admin or a second control transaction. Before P it freezes only the init/proposal and committed=null owner effect. The same bootstrap P ChangeId materializes the canonical /2 record/hash/pin/address, committed presentation_policy_change, original receipt/ChangeRecord association, outbox and one-head transition atomically with Workspace activation; any loser/abort leaves neither an active Workspace nor partial policy state. A successfully activated Plan4 target therefore has a usable one-head default immediately. A proved [] state remains a real explicit initialization state, while missing/corrupt/unproved policy evidence is unavailable and never a silent separate fallback.

Presentation consumes that Workspace record **conditionally**. Explicit .run-in, explicit .separate, role_conflict→Separate and no-eligible-body results do not read or bind Workspace policy. With neither role, only an eligible body satisfying the implicit-default physical-adjacency rule consumes one current policy head. Therefore a missing/corrupt/conflicted Workspace policy cannot block an explicitly separate/run-in document, but it does block Use Default when the default is actually needed.

Enable removes separate and ensures run-in; Disable removes run-in and ensures separate; Use Default removes both. These are ordinary source-role edits and never mutate the Workspace policy. Heading and first paragraph remain independent D2 nodes/source ranges; RunIn is presentation only and never reparsed into author source.

D8DocumentRenderBinding/1 freezes the exact D2 snapshot pin and D8PresentationDecision/1 actually used. Cache invalidation follows real dependency consumption: a Workspace policy change invalidates an old workspace_default decision, while explicit/no-body/conflict-fallback caches do not acquire a policy dependency they never read.

### 8.3 D9 semantic/rendered export and exact preparation

D9 current export uses SCHEMAS §6.5 ExportPlan/3. Exact AsciiDoc source, exact Resource and query_json use generationPolicy=none and no template/route/document-render binding. They do not require a generation-policy registry, renderer, provider or run-in policy, so unrelated provider/configuration failure cannot block these exact paths.

Rendered document targets freeze an exact D2DocumentSnapshot/3 plus its evaluation binding, ManagedDocumentSemanticQualification/1, the exact D8PresentationDecision/1 actually consumed, and the accepted route/profile chain. HTML uses the same product/run-in decision as D8. DOCX/ODT may map effective levels 1–9 to explicit heading styles when the selected target profile supports them; larger arbitrary-precision levels and target limits become explicit loss/unavailability, never D2 syntax rejection. Include/environment dependencies are revalidated before preparation; after a Plan is frozen, a later include/source/presentation-policy change does not rerender or mutate that Plan.

Generation policy is a finite **per-plan** closed value: binding choices, missingPolicy=empty choices, explicit Resource image sizes, layout choices and native-table token bindings. There is no external generation-policy registry/descriptor. Missing policy cannot bypass permission/type/unknown-path errors. Image size and layout choices remain keyed by the exact authorized ResourceRef and fixed Template rules.

Canonical comparators for every set-like collection are also part of the Plan contract: route-step array position equals step=0..N-1 and each step.evidencePins sorts by pinToken; styleBundles sorts by styleBundleId. For bindingChoices/missingPolicy, templatePath is compared by exact Unicode scalar sequence lexicographically, normalization=none and case-sensitive, with shorter exact scalar prefix first; no locale/case-folding rule participates, and both arrays use that same comparator for cross-set exclusion. For stagedOutputs/receipt.outputs, every fresh-current name first satisfies SCHEMAS §6.5 D9ControlledRelativeOutputName/1 and the complete bundle passes its deterministic exact-name, portable-alias, file/directory-prefix and reserved-system-name conflict checks. Only then are the exact stored protocol-name UTF-8 octets compared unsigned lexicographically, with the shorter exact byte prefix first. The comparator itself still performs no normalization, case folding, locale collation, host/path-library ordering or separator rewrite. nativeTableBindings remains keyed by (setName,columnName); existing imageSizes/layoutChoices/lossChoices/top-level evidencePins/recoveryPins keep their established keys. D9 prepare detects duplicate/conflicting keys and canonical-sorts before freezing Plan bytes/token/pins/staged manifest/confirmation basis, so legal input permutations yield byte-for-byte identical Plans. A frozen, received, or recovery current record that is already disordered, contains duplicate/conflicting names, or contains a name outside the closed output-name domain fails; readers never sort, normalize, rename or LWW-repair protected bytes.

D9ControlledRelativeOutputName/1 closes the fixed-parent “controlled relative name” requirement for fresh current ExportPlan/3. Core owns final output names; workers never choose filesystem paths. External-bundle dataFiles names, every ExportPlan/3 stagedOutputs.name, every PublicationReceipt/3 outputs.name, the bundle manifest reportFile name, and the names consumed by server-download delivery all use the same protocol domain. The two root system members are exactly loss-report.json and manifest.json. They are generated by Core, are included in stagedOutputs and receipt.outputs, and are reserved against every dataFile by the same alias/prefix conflict rules before any Plan freezes. A dataFile may use nested components such as assets/图/附件.png, but it cannot claim or alias either root system member or place a descendant below one. Resource handoff consumes the selected original verified dataFile bytes/name; its separate resourceName remains an independent D3 author intent and is never inferred by renaming the export output.

The protocol name is byte-preserving. Unicode normalization and case folding are used only to derive the rejection-only portable alias key defined in SCHEMAS; they never rewrite the stored name, manifest, pin, digest, confirmation, receipt or published bytes. Destination qualification may additionally reject a protocol-valid name when the selected filesystem/storage API cannot create that exact name without extra aliasing or representation loss. Such host capability is an additional availability check, not protocol authority: it cannot make a protocol-invalid name valid, change the alias relation, normalize or rename a frozen name, overwrite an existing target, or convert a failed bundle into partial copy. Name-safety/alias/prefix/reserved failures are structural safety failures and cannot be accepted through ExportLossChoice. They do not reclassify legal AsciiDoc/Resource/Query source or mutate stable source identity.

Current prepare validates the complete proposed dataFile-name set together with the fixed report/manifest names before output sorting, token allocation, pins, staging or confirmation. Inspect/confirmation/publication/receipt preserve those exact names. Saved/planned/unknown current work recovers the original names, bytes, pins, OperationId/token and installation/publication evidence; unknown publication never retries under a renamed output. Genuine historical ExportPlan/1-/2 and PublicationReceipt/1-/2 retain their recorded decoder, names, ordering, bytes, pins and recovery rules even when an old name would fail D9ControlledRelativeOutputName/1; no read-time migration, resort, repin or re-encoding is allowed.

Template binding inputIndex must select the exact template catalog item and byte-equal pin/profile decoder. Route steps freeze actual provider/version/profile transitions and the terminal profile must match the target. Plan3 uses only protected d9_export_plan/3 tokens; PublicationReceipt3 uses d9_publication/3. Historical Plan/Receipt1-/2 retain original tags, bytes, pins and recovery. Confirmation/inspect/unknown-publication lookup dispatches token tag before record version.

Plan evidencePins has one derivation, not an arbitrary set: the recursively reached typed PinRef members of input catalog, document render binding, template, route, styles, dependency/observation proof and staged outputs, union the explicit retained recoveryPins, sorted/unique by pinToken. Omitting a reachable pin or adding unrelated evidence changes/invalidates the Plan. PublicationReceipt3 repeats the exact target/template/route/styles/generation-policy/presentation selection and actual output digests.

Native table→Office dataset names follow SCHEMAS §6.5.1 and require no ordinary-Node export metadata. Core derives physical columns from D2TableBlock/3/D2TableCell/3, including multi-row heads and colspan/rowspan. Text matching is exact Unicode scalar sequence, normalization=none, case-sensitive and whitespace-preserving. Resolution tries leaf text, then the shortest longer header suffix, then exact table title, then deterministic table/column occurrence ordinals. For example, a single lowercase-ASCII leaf `amount` uses `data.native_table.amount`; two leaves Amount under Plan/Actual make bare Amount ambiguous and require the Plan/Amount or Actual/Amount qualified selector/token; two equal-titled tables with the same full path require the frozen occurrence qualifier. Zero candidates is mapping_required; unresolved multiplicity is ambiguous_binding; first/last wins, guessed suffixes and author-source rewrites are forbidden. Simple unique lowercase-ASCII leaf names remain the short COLUMN token; qualified/CJK/RTL/combining selectors use the fixed ASCII digest token encoding in SCHEMAS. Adding a later colliding table/column can invalidate a previously short fresh binding; an already prepared Plan remains frozen to its original table projection/selector/staged bytes.

The retained D9 permission/state machine remains load-bearing: potential scope/authorization precedes sensitive reads; inspect/confirmation/publish/delivery recheck current authorization; unreadable never becomes none; Query authorization generations do not mix; create-only external publication keeps durability/unknown-outcome rules; Save-as-Resource is a separate current D3 create_resource preparation/receipt. Fresh current D3 requests obey the corrected optional expectedAuthority mode matrix.


### 8.4 Fixed-parent direct-holder dispatch and historical boundary

The replacement router explicitly covers the current statements in D2 implementation impact, D7 Definition Transfer, D9 Import IR, and D9 Acceptance Matrix. D2's historical Profile-v2 implementation prohibitions remain immutable snapshot evidence, but current implementation obligations use D2DocumentSnapshot/3 and the complete native-baseline product family above. D9 Import IR retains ImportIR/1, Mapping/1, ConversionInput/2 and all file-safety/mapping/loss rules; only its current outer author submission is D3IdentityOperationRequest/13, while genuine saved/planned wire12 import decisions recover exactly as before. D9 Acceptance S12 is superseded only in its obsolete five-heading-level premise: authored WeftextManaged levels 6–9 are valid and must flow through D2/D7/D8/D9; target-specific unsupported depths require explicit loss/degradation, not source rejection.

No global text replacement upgrades historical wire12, document_snapshot wire2, or ExportPlan/1-/2 records. Saved/planned/unknown recovery runs before current producer gates and retains original bytes, permissions, errors, confirmation and publication responsibilities.

Stage4A originally deferred Annotation mutation/alias work; the current candidate now contains the separately authored Stage4B Value/4/D8/D7/alias chain in §§9–16 and SCHEMAS §§6–7. This residual-A repair does not rewrite that chain. Its direct shared-contract touches are limited to the corrected generic wire13 expectedAuthority optionality and the current EffectItem/3 addition of the typed presentation_policy_change owner effect; both remain subject to direct incremental non-author review together with this fixed head. The three separate fixed19f Annotation residual findings and physical-JSON aggregate source gap remain outside this batch.

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

### 9.6 Portable authority, canonical bytes, revision and CAS

The sole portable author authority for a current Annotation is the existing node-local Portable Metadata entry in `weftext.annotations.json`. Its current logical record is `PortableAnnotationRecord/4` from SCHEMAS §7: durable `annotationRef`, opaque `annotationRevisionToken`, and complete `D3-Annotation-Value/4`. D2's historical Annotation-v2 outer snapshot is no longer a current author value. It remains a historical decoder/projection only; its plain-text body, `replace_plain_text` suggestion and resolved/stale target-status pair do not constrain current Value/4.

The exact current annotation payload bytes are only:

```text
annotationValueBytes = D3-CJ/3(complete D3-Annotation-Value/4)
annotationValueSha256 =
  "sha256:" + lowercase_hex(SHA-256(annotationValueBytes))
```

Every current `payloadBindings` entry with payloadKind=`annotation_value`, every D6 `SourceRevisionPlan/1.afterPin` for an Annotation, D8 proposed-value pin, current EffectBytes encoding `d3_annotation_value4`, and current materialized postimage refer to those same canonical bytes. The Portable Metadata envelope, AnnotationRef and revision token are not hashed into `annotationValueSha256`. Equal digest never proves identity, currentness or CAS.

`annotationRevisionToken` remains a nonempty opaque JSON string and is not content identity. For an existing Annotation, Core compares the exact current token and exact current SourceObservation before planning. `AnnotationEditableProposal/1` and the current/Draft `AnnotationEditableValue/1` projection are different closed shapes and are never compared directly. After closed request decode and the inherited authorization/currentness checks, Core applies the §9.10 operation-class gate and expands the caller proposal plus the complete current before into one complete candidate `D3-Annotation-Value/4` while byte-preserving the four before attribution fields. Core then compares D3-CJ/3(candidate Value/4) with D3-CJ/3(current before Value/4). Equality is the only true value no-op: Core preserves the complete current Suggestion evidence/state, attribution, annotationRevisionToken, SourceVersion and H, creates no SourceRevisionPlan/source_change, and does not implicitly reconfirm or otherwise refresh lifecycle evidence. Only when those canonical Value/4 bytes differ does Core derive fresh lastEditor/editedAt, construct the final complete Value/4, allocate one never-before-used token for that Annotation state, and freeze the unique SourceRevisionPlan/plan; all target/reply slot addresses, structural evidence, source change, receipt and materialization use that same final token. Explicit reconfirm is itself a lifecycle state transition and therefore is not converted into a no-op when it succeeds. A later return to old Value/4 bytes still uses a fresh token; ABA cannot reuse an old token.

The mutable author-value fields are purpose, target, replyTo, body, appearance, labels, reviewState and suggestion. Any actual committed change to any of them is a Value/4 mutation and therefore advances the Annotation revision. The four attribution fields are not caller-controlled mutation inputs. On interactive/current creation, trusted Core derives creator/authoredAt from the actual executing principal and trusted clock, then sets lastEditor=creator and editedAt=authoredAt. On later actual mutation, creator/authoredAt are byte-preserved and Core derives lastEditor/editedAt from the real executing principal/time; a caller-supplied "trusted", "verified", actor, time, or human-origin flag has no authority and is rejected by the closed request shape.

Attribution remains display/history data, never authentication, permission, ordering or LWW. A trusted same-Workspace copy may preserve the four existing attribution snapshots. A trusted cross-Workspace transfer preserves their originWorkspaceRef as historical origin. An ordinary untrusted import may retain display/time text only after the importer materializes the imported snapshots as `imported_unverified`; it cannot claim `workspace_authenticated_origin` or `prepare_server_clock`. A later local mutation of such an imported value preserves creator/authoredAt as imported history and writes lastEditor/editedAt from the actual local execution.

### 9.7 Value/4 D3 mutation binding, slots, receipts and lifecycle

Current wire13 annotation mutation uses the fixed D3 mutation algebra with the current Value/4 base decoder; historical wire9–12 plans continue to decode their real Value/3 bytes. The logical slots are unchanged:

```text
annotation_target  slotOrdinal=0  reference
annotation_reply   slotOrdinal=1  reference when nonnull;
                                  structural S only for identity-preserving reply change
```

Their spans are computed over the actual D3-CJ/3(Value/4) member values, so added Value/4 members may move byte offsets but never change logical ordinals. purpose/body/appearance/labels/reviewState/suggestion/creator/authoredAt/lastEditor/editedAt are nonreference bytes for D3 identity mutation. Inline AsciiDoc inside AnnotationInlineBody/1 remains the R6 inline profile and does not create hidden D3 slots; target identity and reply structure are represented only by the two slots above.

A current existing-Annotation result therefore has one complete Value/4 preimage/result payload pair. target@0 always retains its full reference plan/evidence. A nonnull unchanged reply uses its normal reference evidence. P→Q, P→null and null→Q on the same existing Annotation use exactly one annotation_reply structural plan/S segment plus one `annotation_reply_change`; the reply slot must not also appear as a reference result. A fresh or mapped fresh Annotation's initial nonnull reply remains a reference-plan/result slot and is never an S container. Every current Value/4 result, including a status-only or label-only change, hashes the full Value/4 base or the complete Symbolic Result bytes under the retained payload-binding rule; nonreference change is never invisible merely because target/reply are unchanged.

The fixed D3 mode-admission matrix remains load-bearing. `copy_resource` cannot allocate a fresh Annotation. Ordinary import's new-owner-only rule does not let a fresh imported Annotation become an existing Annotation's same-owner fresh reply. copy_node_subtree, fork and partial identity-bearing import do not gain an additional existing-Annotation S side effect. Generic subject unions do not widen these cases.

For restore of trashed Annotation A while changing reply P→Q, target@0 and nonnull reply@1 both use prestate=non_live_source and poststate=resolved, and every toSource/reply structural change/receipt reference to A uses one final annotationRevisionToken. If the old reply was null, reply alone uses prestate=absent while target remains non_live_source. Lifecycle-only cannot substitute for the typed target preimage or reply S when Value/4 changes. Independent Trash/restore with byte-identical Value/4 does not mint a new Annotation revision; lifecycle is separate portable metadata. Permanent Annotation purge follows the inherited D3 tombstone/no-reuse closure: it removes the live/trashed object from current Portable Metadata authority without manufacturing a replacement Value/4 or revision token, preserves only the history/backup bytes whose existing retention rules require them, and never permits the same AnnotationRef to be reused.

Copy/fork/import materialization rewrites target/reply only through the existing identityMap/candidate-map rules, then materializes one final Value/4 and one final revision token. The receipt's source version, target/reply toSource addresses and `annotation_reply_change.toAnnotationRevisionToken` all name that same final revision. Backup/export transports portable identity/value only under its existing disclosure rules; it transports no current permission, ActionEvidence or executable preparation.

### 9.8 D8 current Annotation edit surface

Current D8 edit preparation is the version-3 successor in SCHEMAS §6.3. `D8EditIntent/3` keeps the document arm; its current Annotation mutation arm accepts `AnnotationEditableProposal/1` plus the exact expected current Annotation revision token and targetPolicy, while the explicit reconfirm arm carries only the Annotation target/token. The caller cannot supply full Value/4 attribution or Suggestion state/confirmation/basis/expectedText/pointAffinity. Core independently reads the complete current PortableAnnotationRecord/4, SourceObservation and permissions, constructs the unique complete proposed Value/4 through the §9.10 operation-class gate, then pins exactly D3-CJ/3(proposed Value/4). `PreparedEditBinding/3.intent` is exactly `D8EditIntent/3`; `D8EditInput/3` is its current OwnerInputBinding descriptor. There is no `<D8 current intent>` placeholder and no client-provided actor/time/lifecycle evidence.

Annotation edit requires complete profile and strict write protection. It first establishes Annotation disclosure + annotation_read/write and exact current Annotation CAS. targetPolicy=preserve requires current target bytes/identity to remain byte-equal but may freshly requalify exact/mapped state without mutating stored target. targetPolicy=replace_current requires an explicit same-owner legal target selected under current target disclosure/qualification. A candidate/ambiguous/fuzzy location is read-only until an explicit manual reattach selects one exact target and prepares a new Value/4 mutation. Pure synchronization or a newly successful stable-locator qualification that leaves Value/4 bytes unchanged is not an edit, cannot refresh lastEditor/editedAt, cannot advance the token, and cannot revive an old PAB/EditBinding/ActionEvidence.

The body editor has one source truth: `AnnotationInlineBody/1.source`. Visual mode is a discardable render of the same R6 `AnnotationInlineProfile/1`; it never stores HTML or a second rich body. During a local Draft, invalid inline source keeps the exact draft source and diagnostics while visual rendering and prepare are unavailable. A principal with annotation_read but not annotation_write receives the authorized current value as read-only and cannot obtain a writable Draft merely because rendering succeeded. Corrupt/undecodable Portable Metadata is not partially projected: normal read follows the existing unavailable/integrity boundary, while an authorized repair/backup surface may expose the exact raw portable bytes without inventing Value/4 members.

Historical D8 wire1/wire2, PreparedEditBinding/1-/2, Value/3 proposed pins and genuine saved/planned/unknown requests recover under their original decoders and pins. They are not converted into EditBinding3/Value4 merely because the current editor understands Value/4.

### 9.9 D7 current Annotation creation and suggestion actions

Current interactive Annotation creation uses the dedicated `D7CreateAnnotationIntent/2` arm. The caller supplies only destinationOwnerRef plus `AnnotationEditableProposal/1`; the proposal contains no Suggestion terminal/lifecycle/evidence. After normal destination-owner disclosure/create authorization, Core validates target/reply owner rules and the R6 body, applies the §9.10 interactive_create gate (an initial suggestion is pending only, and confirmed evidence can come only from fresh target recomputation), injects creator/authoredAt/lastEditor/editedAt under §9.6, then constructs the complete current D3IdentityOperationRequest/13 mode=create_annotation with the inherited fresh primary Annotation subject, Value/4 result payload binding, target@0 reference plan and optional initial reply@1 reference plan. The same PAB4 binds that exact generated request and Value/4 pin. A caller-supplied generic d3_operation or noninteractive constructor cannot claim trusted current interactive creation by embedding actor/time/lifecycle snapshots; copy/import/create-member paths use their named owner preparation and attribution rules instead.

Current new D7 author preparation otherwise uses `D7ActionSpec/2` and `D7ActionPrepareRequest/3` from SCHEMAS §6.1. Every fixed-parent non-suggestion ActionSpec/1 arm is inherited byte-for-byte, with the new create_annotation arm added for the need above. The old `apply_suggestion(annotation:EntityTarget,targetLocator)` arm remains historical only. The current closed suggestion arms are:

```text
apply_suggestion  -> accept the stored pending Suggestion/3
reject_suggestion -> reject the stored pending Suggestion/3
```

Both name the AnnotationRef and exact expectedAnnotationRevisionToken. They do not accept targetLocator, replacement bytes, expectedText, point affinity, actor/time or a precomputed source patch from the caller.

For apply_suggestion, preparation freshly reads the current Annotation and stored Suggestion/3, requires pending+confirmed, freshly qualifies its stored target and actual target source, then maps kind exactly: replace→one SourceTransform replace using replacementSource; delete→one replacement with zero replacement bytes; insert→one SourceTransform insert at the stored zero-width point using stored pointAffinity. replace/delete verify current expectedText against the exact fresh bytes; insert verifies the point/basis. Any mapped/candidate geometry is only an input to fresh exact qualification. The target SourceOrigin identifies the real writable owner; AnnotationRef ownership and reply structure grant no target write authority.

Accept creates one immutable D6 plan containing the target source after-image plus the same Annotation Value/4 with Suggestion.state=accepted, confirmation=not_applicable and Core-written lastEditor/editedAt. Both source changes commit under the same DecisionKey, one planning CAS and one P seal; partial target-write-without-accepted-state or accepted-state-without-target-write is invalid. The target Document branch uses the existing CoreSourceEditPlan/2/SourceTransform evidence when representable and never revives an old PreparedIntent/PAB.

reject_suggestion requires Annotation state disclosure plus annotation_read/write and the exact current Annotation token, but does not read or require permission to the target source. It changes only the Annotation Value/4 to rejected/not_applicable through the same CAS/revision/actor-time rules. Accept racing reject or another status/body edit has one token winner; losers are stale/conflict and must freshly read/prepare.

### 9.10 Suggestion operation-class transition gate

`AnnotationEditableValue/1` remains the current/Draft editable projection but is no longer a direct current caller mutation wire. D7 create and D8 edit use SCHEMAS §7.1 `AnnotationEditableProposal/1`, which has no state/confirmation/basis/expectedText/pointAffinity. Core first reads the complete current before when one exists, mechanically derives the operation class from the real entry, and applies the only legal before+operation+proposal→after rules below. The derived before class is closed: a fresh create is `absent_annotation`; an existing Value/4 with suggestion=null is `no_suggestion`; a pending Suggestion/3 is `pending_confirmed` or `pending_needs_reconfirmation`; accepted/rejected are `terminal_accepted` / `terminal_rejected`. Fresh absence is never conflated with an existing no-suggestion Value. There is no generic trusted flag, second decision system, or UI-only gate.

| producer / operation | permitted Suggestion class before→after | mandatory Core behavior |
|---|---|---|
| D7 create_annotation / interactive_create | absent_annotation→no_suggestion for a legal root comment/root mark/reply; or absent_annotation→pending for a legal root suggestion | The caller supplies only the Proposal. Comment/mark/reply creation must satisfy the existing purpose/reply/body/appearance/review cross-fields and creates suggestion=null. Suggestion creation takes only kind/replacementSource from the caller; Core qualifies the actual selected target and may produce confirmed only after it itself reads/verifies that target and recomputes targetBasis/expectedText/point, otherwise pending+needs_reconfirmation. Initial accepted/rejected remains impossible. |
| D8 annotation+preserve / ordinary_edit | no_suggestion→no_suggestion; no_suggestion→pending+needs_reconfirmation; pending→pending; pending→no_suggestion; terminal→same terminal only | Legal ordinary edits keep body/appearance/labels/review and legal purpose/root↔reply changes available under the existing Value/4 cross-fields. An unchanged no-suggestion target needs no target-source-content read. Converting no_suggestion→pending is allowed only for a legal root suggestion after current Annotation authorization/token checks plus fresh qualification of the actual stored target; Core computes the lifecycle evidence but forces needs_reconfirmation, so this category conversion cannot manufacture confirmed/accepted/rejected. A later explicit reconfirm is required for confirmed. A pending suggestion may be cleared to a legal no-suggestion root/reply, discarding its pending evidence without claiming a target mutation. For pending→pending, byte-equal author suggestion inputs preserve the existing confirmed/needs evidence exactly; a target-dependent author change that requires new qualification forces needs_reconfirmation. Terminal Suggestion/3 is byte-equal and cannot be cleared, repurposed, reopened or switched to the other terminal state. |
| D8 annotation+replace_current / manual_reattach | no_suggestion→no_suggestion; pending→pending+needs_reconfirmation; terminal forbidden | A no-suggestion comment/mark/reply may be reattached to one exact legal same-owner target and remains suggestion=null; this path does not invent suggestion evidence. A pending suggestion reattach recomputes targetBasis and kind-required expectedText/point and forces needs_reconfirmation. This operation does not also perform a suggestion-category conversion. Terminal Suggestion cannot reattach. |
| D8 annotation_reconfirm_suggestion / reconfirm_suggestion | pending_needs_reconfirmation→pending_confirmed | Caller sends no value/evidence. Core freshly reads the stored target and actual target source/point and recomputes all targetBasis/expectedText/pointAffinity. Any currentness/permission/byte mismatch yields zero mutation. Successful reconfirm is a real state transition, not a no-op. |
| D7 apply_suggestion | pending_confirmed→terminal_accepted only | Retains §9.9 fresh target read, three-kind transform, target-after + Annotation-after in one D6 plan/one planning CAS/one P. |
| D7 reject_suggestion | pending_confirmed\|pending_needs_reconfirmation→terminal_rejected only | Requires only Annotation disclosure/read/write and exact token, never target read; a hidden/unavailable target remains rejectable. |
| D3 copy/import owner, Trash/restore, recovery | carried terminal display state is not interpreted as a fresh apply/reject decision | Existing identityMap/attribution/historical-decoder functionality is retained. Copy/import may preserve terminal display state and attribution, but without a current-Workspace apply receipt/target source change it cannot claim a target mutation occurred. Fresh ordinary import uses imported_unverified attribution. Pure byte-equal Trash/restore preserves token/Value and creates no new decision. |

For an existing Annotation, the gate expands the legal Proposal into a complete candidate Value/4 **before** no-op comparison. ordinary_edit preserves current Suggestion lifecycle/evidence whenever the author-visible suggestion inputs and target-dependent inputs are unchanged; it never implicitly reconfirms a pending suggestion or rewrites terminal evidence. The candidate initially carries the before creator/authoredAt/lastEditor/editedAt, and only a candidate that differs canonically from before is upgraded with fresh lastEditor/editedAt and a fresh revision token/SourceRevisionPlan. No-op therefore grants no extra capability and cannot bypass stale/currentness/final-barrier checks.

The inherited error/disclosure order remains operation-sensitive. Closed decode → Annotation state disclosure → annotation_read → annotation_write (for a mutation) → exact token/current Annotation always precede any target-source disclosure. ordinary_edit that keeps an existing no_suggestion value and stored target content unchanged does not gain a new target-source-read requirement merely because the gate exists; the same is true when a pending suggestion is cleared to no_suggestion. Creating a pending suggestion from no_suggestion, manual reattach, explicit reconfirm, and apply consume target information only at their named target qualification/read stage. A hidden target returns not_visible before any target bytes/expectedText are exposed; provider/source unavailability is not rewritten as orphaned. Failed reconfirm preserves pending+needs_reconfirmation. Permission revocation, concurrent token/Observation movement, stale Draft/base binding, or a failed final read barrier yields the inherited failure/reprepare with zero mutation even if the expanded candidate would otherwise compare equal. A noninteractive/generic constructor and an old selector/PAB/Draft/recovery artifact cannot reopen the bypass.

### 9.11 Current proposed carrier and Annotation read chain

`D7ProposedInput/3` is the sole current proposed-input carrier of D7ActionInput/3/PAB4. D6-owned apply/reject/other concrete current Annotation after-images use `payloadKind=annotation_value, encoding=d3_annotation_value4` and pin complete Value/4 canonical bytes. The legal D3 symbolic-result branch retains `d3_symbolic_result9`. Genuine PAB3/D7ProposedInput/2 retains its original d3_annotation_value3 decoder/bytes/pins; current decoding never packs Value4 bytes under /2.

This paragraph replaces fixed D7 Query Algebra §2 for `{kind:"annotation_body"}`. from remains AnnotationRef and the output remains text, but the producer reads current PortableAnnotationRecord/4 at the final authorized query read barrier and evaluates the sole AnnotationInlineProfile/1. body=null projects to `""`. A valid body returns **R6 semantic text**, not exact source: source `*Alice*` therefore yields query text `Alice` while D8 read/Draft alone exposes exact `*Alice*`. After annotation_read/currentness succeeds, invalid R6 fails the whole read with the existing D7 `source_unavailable` outcome; there is no historical D2 plain_text fallback, strip-markup guess, or partial semantic text.

SCHEMAS §6.3.1 `d8_annotation_read` and `d8_annotation_draft_open` are actual Core/D8 producers. Read performs Annotation disclosure/annotation_read before obtaining the current aggregate-backed record, Annotation SourceObservation/token and target resolution. It returns complete Value/4 (including creator/authoredAt/lastEditor/editedAt) plus body exactSource+semanticText/diagnostics. If the target itself is hidden/unavailable while the Annotation is readable, the Annotation may still be returned with targetResolution=unavailable and without target-source bytes. Draft-open returns that complete read plus the discardable editable projection. Without annotation_write it is readonly and cannot prepare. Editable prepare binds the base token/Observation and revalidates current record, aggregate observation, authorization and dependency cut both before planning and at the final read barrier; any movement is stale/reprepare, regardless of equal Value hash or locator.

### 9.12 `weftext.annotations.json` physical aggregate and D6 installation

Portable Workspace Metadata remains the only logical authority. Node-local `weftext.annotations.json` is its Annotation physical carrier, not a second metadata root, DB, ledger or CAS. The sole current physical format is SCHEMAS §7.1 `AnnotationAggregate/1`. The sidecar location is mechanically derived from the current Node FileBinding/managed-node boundary plus the fixed basename, so path is not persisted in the aggregate. A coordinated rename updates only FileBinding/physical observation; ownerNodeRef, AnnotationRef, Value4 and revision token do not change. Other `.weftext-meta` portable facts retain their D6 owner. Future sharding requires a closed layout successor with one deterministic authoritative shard for each AnnotationRef and never duplicate writable copies.

Each changed Annotation still has its own logical `PortableAnnotationRecord/4`, final AnnotationRevisionToken, and Value4/source-revision result. Whole-file canonical bytes/FileObjectBinding/AnnotationAggregateObservation/install pin are a distinct **physical layer**. Changing one record stales the old aggregate observation and any current read observation whose evidence binds that FileObjectBinding, but it does not allocate a new AnnotationRevisionToken, Value4, managed SourceVersion, or H for unchanged byte-equal records. A legal fresh read rereads and strict-decodes the complete aggregate and rebinds the unchanged logical record to a new current observation. That is observation refresh, not source mutation.

For a write, Storage starts from a fresh AnnotationAggregateObservation/1 whose FileObjectBinding is the fixed D6 Control §2 absent or present branch; a bare missing/guessed state is not a substitute. It applies every logical record change in this DecisionKey to one in-memory aggregate while preserving untouched logical record bytes, and emits exactly one AnnotationAggregateInstall/1 after pin. The managed_atomic strong path reuses D6 §6 and Control §2 create_only/conditional_replace/exclusive_write_window strict qualification. A before FileObjectBinding/object-generation mismatch is stale/paused; read-hash-then-rename and digest equality are not CAS. An observed_only path already eligible under D6 §4.1 may still use observed_replace, retaining only that owner's weaker guarantee and never claiming managed atomic/CAS. For concurrent managed updates of different records in the same sidecar, at most one physical CAS succeeds first; the loser rereads and reconstructs an after that includes the winner rather than LWW-overwriting it.

If one decision changes multiple Annotations in a Node, InstallationNotice3/CP4 still lists each actual `PortableComponentKey.annotation` logical component after, aligned with each Annotation sourceChanges/final revision, while InstallationPlan has exactly one physical sidecar after. Before P seal Core verifies the complete aggregate after, every changed logical component, SourceRevisionPlan/value pin, and file install agree; any mismatch prevents the whole seal. Sealed production history and old PreparedIntent are never updated, revived, or repinned merely because a later aggregate container is rewritten.

Sync/admission first verifies the original D6 ChangeRecord/Notice/CP/current-conflict continuous chain, then strict-decodes the entire aggregate. Arrival order, mtime, revision number, equal hash, or provider “latest” never chooses a winner. Duplicate Ref, wrong owner, reply cycle, duplicate JSON key, truncation, unknown format/version, or component/aggregate mismatch is never partially trusted: the strong path is incomplete/integrity_conflict, while authorized raw repair/backup may retrieve the exact original bytes without synthesizing members. Direct external JSON editing is not a trusted Core decision even when syntactically valid. Only a verified portable transition or explicit import/admission establishes new current state, and caller actor/lifecycle fields do not thereby become trusted apply/reject evidence; ordinary import attribution remains imported_unverified.

A backup at an explicit Frontier carries ordinary files plus exact portable sidecar bytes but no current permission/PAB/ActionEvidence. Node copy/import uses the existing identityMap/candidate-map to create fresh AnnotationRefs, rewrite target/reply, and emit the unique aggregate for the new owner. Terminal Suggestion state may remain as historical display data, but no destination target-mutation receipt is fabricated. Trash/restore with byte-equal Value4 moves only the original lifecycle/portable ownership state and preserves token; after restore a fresh read rebinds physical observation. Purge removes the record, and purging the last record makes canonical after sidecar absent. Reply graph, history retention and no-reuse remain under §9.7/D3; container rewriting does not alter them.

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
  kind:"d6_core_source_edit_plan",version:2,
  decisionKey,ownerNodeRef,beforeObservation,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,
  transformEmission:TransformEmissionPlan/1
}
``` Emission is either disabled with `no_exact_core_edit_plan|transform_profile_unavailable`, or required with exact profile, expectedTrustRevision, and expectedTrustKeyId. The caller cannot choose. A winning required plan cannot downgrade during recovery; a disabled plan cannot upgrade. Seal cannot recompile or reorder events.

## 12. Signed transform evidence and outbox

SourceTransformEvidence/2 binds DecisionKey, ChangeId, owner, managed before/after SourceVersions, before/after hashes, the fixed coordinate/affinity profiles, and the exact /3 event array.

```text
SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",version:2,
  decisionKey,changeId,ownerNodeRef,
  before:managed SourceVersion/2,
  after:managed SourceVersion/2,
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

The same Plan4 also freezes mandatory initialPresentationPolicy. Prepare derives its D8 before stamp from the original issuer-proved empty target history, not from a missing file, and freezes parents=[], revision=1, separate plus a typed presentation_policy_change preview with committed=null. Planning/staging carry those exact bytes under the original operation and never create the ChangeId-dependent record/hash/pin/outbox. At the one final P, after all original proposal/custody/Registry/Policy/trust/source checks, Core allocates the original bootstrap ChangeId C and in that same atomic transition materializes the D8 /2 record, exact prefixed hash/pin/address, committed effect, original receipt/ChangeRecord association, outbox and head transition while activating W/B and all other bootstrap state. A losing CAS, abort or failed P cannot leave either a half-active Workspace or a policy record without activation. After P, default-dependent presentation immediately resolves against revision1/separate. Publication-unknown retry uses the exact outbox bytes and receiver admission checks the original bootstrap decision association. Saved/planned/unknown Plan4 resumes from its recorded init bytes without resampling state.

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

Suggestion state remains separate from reviewState and is owned only by the complete Value/4. pending uses confirmed|needs_reconfirmation; accepted/rejected are terminal and use not_applicable. The current executable actions are the §9.9 D7ActionSpec/2 apply_suggestion and reject_suggestion arms.

Explicit manual reattach/reanchor is an ordinary Value/4 target mutation: it writes one new exact same-owner target, changes a pending suggestion to needs_reconfirmation, advances the Annotation revision and records the actual Core editor/time. Reconfirmation is another explicit current mutation that freshly qualifies that exact target, recomputes targetBasisSha256 and expectedText/point, and returns pending+confirmed under a new Annotation revision. Neither mapped/candidate geometry nor successful pure sync requalification changes Value/4 automatically.

Accept/reject always use the exact current Annotation CAS. Accept follows the three-kind mapping and one-seal Document+Annotation atomicity in §9.9. Reject has the positive target-unreadable path: after Annotation disclosure and annotation_read/write succeed, target hidden/unavailable does not block rejection because no target bytes are read or written. A concurrent body/review/label/appearance/reply/suggestion edit changes the same revision token and therefore stales the older suggestion action.

## 16. Copy/import/export/backup of annotations

Node copy creates fresh AnnotationRefs, rewrites the complete reply graph, and rebuilds targets only through the real source/resource identityMap. Fresh/mapped Annotation target@0 and initial nonnull reply@1 use the inherited reference-plan/result rules; they are not existing-Annotation structural S changes. Every materialized copied Value/4 uses its one final Annotation revision, while attribution is handled by §9.6. Old locators are never copied and positions are never guessed from text. Failure to rebuild a mandatory target/reply rejects typed copy; a byte-preserving raw copy is not typed success.

Ordinary import creates only fresh imported Annotation identity under the inherited owner rules. Proven targets/replies are rewritten through the actual import mapping. An unprovable target is kept only in an explicitly imported-unverified/candidate repair representation permitted by the import profile and cannot authorize navigation/write; it is never silently rebound by text. A fresh imported Annotation cannot be used to create an extra structural reply mutation on an existing Annotation when the D3 mode matrix forbids it.

Portable backup carries the current PortableAnnotationRecord/4 JSON values and identity with their historical attribution display data, but no current permission, SourceObservation, revision-token signing capability, PAB, ActionEvidence or executable authorization. Restore/recovery first dispatches actual recorded version: genuine old D2 Annotation-v2/Value3 records retain their historical decoder and recovery; current Value/4 records strict-decode as Value/4. Corrupt/unknown bytes remain repair/backup data and never become a partially trusted current Annotation.

Export/Review Bundle includes Annotation body, source/history excerpts and media-region context only when each existing disclosure permission succeeds. Resource-region annotations retain the exact resource identity/version/profile/geometry required by their target contract. Renderer convenience never upgrades candidate/ambiguous/orphaned/unavailable resolution or converts a hidden source into exported context.

### 16a. Fresh Annotation-content and current View export closure

Fresh unseen D9 export now uses ExportPlan/4. This successor exists because Annotation content and semantic View rendering cannot be encoded by the closed Plan/3 catalog/domain/projection family without widening historical decoders in place.

For Annotation content, Core first performs the ordinary current Annotation read and freezes one annotation_content catalog item from that exact D8AnnotationReadResponse/1. The private Plan therefore retains the current Observation, Annotation revision token, complete Value/4, exact PortableAnnotationRecord/4 canonical bytes/pin, body read and targetResolution. The same-cut Plan proof covers that read. A changed Observation, revision token, record pin, or qualification invalidates the unpublished Plan even when the visible body is byte-equal.

Portable backup serializes the exact selected PortableAnnotationRecord/4 values and no current permission/capability state. Review Bundle consumes only the single R6 body result already produced by the owner read. It carries purpose/review metadata, reply and attribution from the same Value/4. Source/history excerpts and target context are separate optional disclosure projections: failure to disclose either does not suppress an otherwise-readable Annotation body/attribution, and an unavailable target never becomes a guessed label/source excerpt. annotation_index remains omission-only and never substitutes for this content path.

For View export, Core freezes one complete D7ResultPin and one exact current ViewSpec/1, runs the existing D7 View runtime validation order to completion, then freezes the ViewSpec hash, D7 SemanticStateKey/SnapshotResultKey hashes, result epoch/auth generation, one renderer/profile/version, exact D8 presentation decision, and every consumed renderer/font/color/page/accessibility asset pin. The first D9 chart-export profile is intentionally finite: metric, bar, line, scatter, pie and heatmap only. Deferred or otherwise unsupported layouts/backends are renderer_unavailable for chart export rather than silently exporting data rows as if they were the chart.

The current positive target set for this dedicated View route is DOCX, XLSX, PDF, SVG, PNG and print when a named accepted profile for that exact layout/target is installed. Every profile must preserve the D7 same-data accessible table, plain-text title/description or equivalent alt-text semantics, logical Query/panel order, CJK/RTL, high-contrast/non-color-only meaning, finite page/font/color assets and complete loss reporting. DOCX/XLSX profiles that consume an Office template remain subject to the visible-template authority and repeat/style rules of Mandatory §14. A renderer library name or generic ECharts/Vega-style configuration is never author input.

Plan/4 preparation freezes all Annotation/View content, loss, renderer assets and exact staged bytes before confirmation. Confirmation cannot rerun Query, re-read a different Annotation revision, switch renderer/profile or repair a stale result. Query authorization-generation/reset, expired/incomplete results, changed Annotation revision, contradictory pins, missing assets or unavailable profiles follow their original typed failure order and never revive an older Plan. External publish remains create-only; Resource handoff remains a separate original D7/D3 author operation. Print uses the same frozen bytes and explicit confirmation but produces only the dedicated D9 print delivery receipt, not an author receipt or an external-publication claim.

ExportPlan/3, PublicationReceipt/3 and every earlier plan/catalog/projection/loss/confirmation/bundle family remain exact historical recovery inputs. Saved/planned/unknown records always dispatch by their recorded token/version and original bytes before any /4 current qualification.


## 17. Error order, recovery, and version boundary

The common order remains: closed decode -> minimum disclosure/capability -> authority/domain/backend/trust -> stable key and saved/planned/unseen -> current target/source/control -> dependency/semantic/budget -> one planning CAS -> install -> one P seal -> output authorization.

Saved results replay under the original owner/version. Planned work restores the original request, descriptor/proof, preparation, pins, Notice, install state, and version basis and is never upgraded in place. Unknown work retains original Approval/Money/claim/external/stop responsibility. Only unseen current work uses Descriptor3/PAB4/Edit3/ExportPlan3/Notice3/CP4/Declaration2, etc. The existence of a decoder or historical design text does not prove a prototype was deployed.

Missing a transform artifact, index row, provider, or strong proof is not a reason for a permanent generic owner_update_required once this coordination is present. Each affected path uses its actual unavailable/conflict error while raw source, Draft, repair, and other independent paths remain available under their existing qualifications.

## 18. Schema-version inventory

Current additions/successors include ManagedDocumentFormatProfile/1, ManagedDocumentFormatBinding/1, DocumentFormatCurrentQualification/1, ManagedDocumentSemanticQualification/1, OracleSemanticWitness/7, CoreSemanticProjection/1, DependencyKey/3, DependencyProof/3, InputDescriptor/3, PreparedIntent/3, PortableComponentKey/2, InstallationNotice/3, ContentCompletionProof/4, ChangeRecord/1, D3IdentityInput/13, D3ResolutionInputUse/2, PreparedActionBinding/4, D7ProposedInput/3, EffectManifest/3, EffectBytes/3, PreparedEditBinding/3, PortableAnnotationRecord/4, AnnotationEditableValue/1, AnnotationEditableProposal/1, SuggestionAuthorProposal/1, AnnotationAggregate/1, AnnotationAggregateObservation/1, AnnotationAggregateInstall/1, D8AnnotationReadRequest/1, D8AnnotationReadResponse/1, D8AnnotationBodyRead/1, D8AnnotationDraftOpenRequest/1, D8AnnotationDraftOpenResponse/1, D8EditIntent/3, D8EditInput/3, D7CreateAnnotationIntent/2, D7ActionSpec/2, D7ActionPrepareRequest/3, D7ActionInput/3, ExportPlan/3, PublicationReceipt/3, PortableTransformCompilation/1, SourceTransformPortableEvent/3, CoreSourceEditPlan/2, TransformEmissionPlan/1, SourceTransformEvidence/2, SourceTransformSealArtifact/1, SourceTransformSealOutboxItem/1, WorkspaceTrustDeclaration/2, WorkspaceAuthorizationBundle/2, DomainSealKeyHandle/2, TrustConflictCarry/2, TrustConflictOutcome/2, FreshDomainAuthorizationSpec/2, PolicyBundleHeadEvidence/2, TrustConflictCarryValidationHop/1, TrustConflictCarryValidationEvidence/1, ConflictResolutionPolicyDerivedPlan/2, ConflictResolutionInput/3, ConflictResolutionPreview/2, WorkspaceTrustGenesis/2, WorkspaceBootstrapPlan/4, D10WorkspaceReadDependencies/2, ControlDependencies/3, D10ControlInput/2, ControlPrepareBinding/3, D10ControlRecordImage/2, D10ControlRecordPin/2, D10ControlRange/2, D10ControlEffectPlan/2, ApprovalUse/2, D10AuthorPreparationLink/2, D10AuthorStepResponsibility/2, ScheduleRecurrenceEvidence/2, ScheduleSubscription/2, ScheduleContinuityWitness/2, ScheduleContinuityStep/2, ScheduleContinuityInvalidation/2, ScheduleOccurrenceProof/2, AutomationOccurrenceRecord/2, D10MoneyResponsibility/2, D10ExecutionClaims/2, D10ExecutionInventory/2, ExecutionResponsibilityRecord/3, and ExecutionContinuityProof/2.

Key retained types include PinRef/2, ComponentImage/1, OwnerInputBinding/2, ObservationScope/2, D3DecisionCompanion/2, D4/D5 inner selector/locator types, WorkspaceTrustRootDeclaration/1, WorkspaceTrustAnchor/1, RevisionTokenSealVerificationKey/1, LeaseRunUse/1, D10ExternalResponsibility/1, D10StopResponsibility/1, and StopCapacity/1. Historical decoders are never expanded in place.

The current managed-format type family used by this candidate is:

```text
ManagedDocumentFormatProfile/1
ManagedDocumentFormatBinding/1
DocumentFormatCurrentQualification/1
ManagedDocumentSemanticQualification/1
```

## 19. Acceptance and evidence boundary

The itemized design obligations are in ACCEPTANCE.zh-CN.md / ACCEPTANCE.md: 438 core design oracles and 322 actual-owner coordination fixtures, 760 obligations total. They are **unexecuted design obligations**, not implementation test results.

This PR did not install or execute the Ruby oracle, Asciidork, Mermaid CLI, browser/Puppeteer, STEM/PDF providers, product SQLite/storage code, replica/crash/crypto paths, real Automation scheduling, or execution-custody handoff. Author checks are limited to document/JSON/router consistency. Independent design review must bind to the exact stopped PR head SHA.