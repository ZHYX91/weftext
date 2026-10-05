---
source_language: zh-CN
translation_of: SPEC.zh-CN.md
translation_status: synced
---

[简体中文](SPEC.zh-CN.md)

# Weftext AsciiDoc / Annotation Final Design Candidate: coordinated owner replacements

Status: **candidate-design-not-implemented**. This specification is bound to parent input `e8aa0b341630a57c786c0891d4bbd1620247441d`. It is only a coordinated design candidate on top of that commit; it does not accept this PR, A2, the global design, implementation, or release.

## 0. Precedence, scope, and why this is one joint PR

This file and [SCHEMAS.md](SCHEMAS.md) together form the complete normative replacement for the actual-owner sections named by `replacements.json`: this file freezes behavior, algorithms, and owner boundaries; SCHEMAS freezes closed current shapes, ordering, cross-field rules, and historical dispatch. Summary wording here may neither broaden nor narrow SCHEMAS. Fixed-S input `7e18168dad3e6d120fce0dd607dc10fa7894e252`, all 49 snapshots, and `docs/design/inputs.json` remain unchanged. Any fixed-e8aa owner section not named by the router retains its existing meaning.

The candidate consolidates the independently reviewed cumulative v3.10 core design and FC-3.4 through FC-3.4d actual-owner coordination into a public, reviewable contract. Old candidate labels, private conversation, and author self-review are not protocol dependencies.

A two-PR split was considered. It is not self-consistent: the current `DependencyProof/3 -> PreparedActionBinding/4 -> D10 ApprovalUse/2` chain and the shared `EffectManifest/3 / EffectBytes/3` family simultaneously carry managed-format currentness, D10 author recovery, and `WorkspaceBootstrapPlan/4`. Publishing a temporary closed `/3` union and expanding that same version in a second PR would violate the accepted closed-decoder rules. Therefore this is one joint PR organized by D2-D10 owner.

There remains one author authority, one D6 planning CAS, and one P seal. No second source truth, portable ledger, decision system, or free-form adapter gate is introduced. Derived Index remains discardable and never reconstructs current authority.

## 1. D2: complete AsciiDoc language contract

### 1.1 Fixed language baseline and production boundary

The observable AsciiDoc core-language semantics for Weftext are fixed to:

```text
asciidoctor-ruby/2.0.26
commit 0b99b39c9df884d4aec13bba45f03cdbab505769
```

Ruby Asciidoctor is a conformance oracle only, never a production dependency. Production remains Rust-native. A mature implementation is preferred; Asciidork commit `79671b57923841e41fc91d2c1d2fce18cca3131c` is an implementation candidate, not a language ceiling. Implementation difficulty, missing rich-editor controls, or an unavailable renderer provider cannot redefine valid AsciiDoc as a safe subset.

The core contract includes the fixed 2.0.26 semantics for block/inline/list/table/description/callout/checklist forms, attribute lists and document attributes, includes, substitutions and macros, passthroughs, STEM, media, footnotes/indexterms/catalog, doctype/backend behavior, document title/author/revision, block title/reftext, native TOC/section numbering, `leveloffset`, diagnostics, and all processor-environment inputs that alter observable semantics. A valid construct with no structural editing widget must remain parseable, readable, source-editable, savable, and losslessly round-trippable.

Native title-separator semantics remain unchanged: default `: `, with `[separator=::]` and `:title-separator: ::` supported as Asciidoctor 2.0.26 defines them. **Whether new Weftext templates default to `::` remains an unresolved product choice and is not selected here.**

### 1.2 Processor environment

The Core Gate freezes `AsciiDocProcessorEnvironment/3` instead of consulting ambient host state. It closes doctype, semantic backend profile, safe mode, base/input/output directories, ordered hard/soft set/unset attribute operations, user-home, locale/encoding, date/time/epoch inputs, include resolver, file/network permissions, extension registry, provider profile, and every other environment input that can change fixed-2.0.26 observable semantics. Authored attribute events are provenance-distinct from host/builtin/include inputs. Weftext root controls such as `wf-kind` and `wf-facets` consume root-authored provenance only; host injection or included text cannot take over root Node classification.

### 1.3 Native headings, deep authored headings, and run-in rendering

The native Asciidoctor section state machine is retained, including discrete/floating titles, book part/chapter behavior, skip warnings, fragment relaxation, negative-leveloffset handling, and doctitle conditions. WeftextManaged adds only these semantic extensions:

- authored section levels 6-9;
- standard `leveloffset` remains standard and the effective level is not clamped merely because authored levels are 6-9; it may exceed 9;
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

The Ruby observer is test-side evidence only. It is not a product parser or authority. It cannot replace the selected backend, insert markers/sentinels, mutate evaluator strings or encodings, add extra calls to `content/text/title/reftext/xreftext`, or rerun parsing/regexes to manufacture evidence. Actual converter return values continue into later substitutions unchanged.

### 2.2 `OracleSemanticWitness/7`

Witness/7 retains all Witness/6 document, block, collection, catalog, diagnostic, observed-string, call, operation, inline, and content evidence and adds:

```text
modelPropertyObservations:[OracleModelPropertyObservation/1...]
```

Model observation only copies fields, attributes, relationships, and actual producer results that already exist. Section state, caption/numeral, final table widths, final Cell object/style/alignment/span, ListItem checklist/coids, authored named/positional/rekey provenance, and temporary converter writes must be captured at the real producer. `projectRuby` never reimplements Ruby parser/model algorithms to fill missing facts.

### 2.3 Snapshot/reference/cut contract

`modelPropertyObservations[].observationId` is unique and strictly append-monotonic in that array. A snapshot's identity is its observationId; there is no second snapshot-ID namespace. Each subject has one immediate-predecessor chain: the first snapshot has null previous; each later snapshot points exactly to that subject's directly preceding snapshot.

Bind and cut snapshot references must exist, belong to the subject, and select the latest snapshot at that point. Cut heads are numerically sorted by subjectId, exactly one per live semantic subject, and satisfy:

```text
set(cut.heads.subjectId) == reachableSemanticClosure(cut)
```

Forward discovery uses actual semantic relations such as header/child/dlist/table-column/final-cell/cell-inner-document. `parent` and `cell_column` are reverse consistency checks and cannot revive stale objects. A treeprocessor-returned Document becomes the root. A Cell replaced by `reinitialize` is not live beside its replacement.

`model_ready` occurs after the actual top-level `Document#parse` completes, including attribute restoration/treeprocessors and the actual returned Document. A successful selected-backend evaluation produces exactly one `evaluation_complete`; abnormal termination produces none. Real post-ready semantic writes require successor heads; temporary physical writes cannot be hidden by selecting an older head.

### 2.4 Namespace separation and producer-time carrier binding

Numeric ordering applies only to references inside the `modelPropertyObservations.observationId` namespace: previous/bind/cut snapshots and model-slot observation references. `operationId`, `inlineEventId`, `valueId/observedStringId`, and `callId` use their retained namespaces and DAG rules. `ModelCarrierReference.entry` is a zero-based index into its named carrier array. No global event ordinal exists.

Cross-stream chronology is a controlled producer invariant, not something the final decoder can infer from numbers. At the actual bind callback, the target carrier entry must already be appended (`entry < targetStream.lengthAtBind`) and the producer must still hold the exact same Ruby object. `document/0` uses the equivalent publication rule for the actual returned Document. The final decoder checks final existence/type/index and object-binding structure only. Appending a carrier later cannot cure an invalid earlier bind.

### 2.5 Catalog parent and temporary physical state

A catalog subject has exactly one ownership relation:

```text
parent -> subject of the actual Document#register receiver
```

An inner AsciiDoc-cell Document owns its own registered catalog facts; they are not reassigned to the top-level Document.

DocBook `root-option` evidence follows the real physical execution (for example authored value -> internal `set_option` temporary value -> `remove_attr` absent/deleted). The observer cannot synthesize a restore write. The latest physical snapshot and the PropertyProfile's authored semantic winner are distinct layers: internal cleanup does not erase earlier authored provenance, nor can it revive an authored value that was genuinely overwritten or deleted later.

## 3. `CoreSemanticProjection/1` and canonical property profile

The equality surface contains final document/block structure, flows, catalog facts, index terms, final attribute/counter state, and diagnostics. `CoreFlow/1` uses final runs and marks with Unicode-scalar coordinates. Visible text is owned once; hard breaks, footnote references, and other atoms do not duplicate text already owned by the flow.

Raw Ruby IDs, call counts, temporary converter-return nodes, and intermediate dependency edges are excluded from equality. Their real effect on final target strings, text, catalog, counters, or attributes must remain. Authored passthrough and backend-feedback raw data are not guessed into ordinary formatting marks.

Property provenance is A/P/F/I: authored named, authored positional, fixed-derived semantic, and internal. Positional rekeying leaves one canonical slot. A genuine authored named attribute remains even when the current renderer ignores it. Internal `cloaked-context`, caches, reader objects, and temporary `root-option` state never leak via generic `to_s`.

Common fields include id, style, ordered roles, set-like options, caption, numeral, explicit substitutions, unconsumed positional fields, and `named/<name>`. The profile also fixes actual 2.0.26 semantic fields for quote/verse attribution+citetitle, source language/linenums, section sectname/special/numbered, tables/columns/cells, lists/checklist/coids, media, and marks/atoms.

Explicit `foo-option=bar` preserves both option membership and `named/foo-option="bar"`; an explicit empty value preserves the empty string. `%foo`, `options=foo`, and `opts=foo` create membership only. Actual writer order decides the winner.

Language sequences retain order. Sets are sorted/unique. Multisets sort by D3-CJ/3(item) UTF-8 bytes while preserving multiplicity. Unique-key conflicts reject instead of LWW. Diagnostics compare a common canonical code and `null | {logicalFile,line}`; a Rust-only column is not invented on the Ruby side.

## 4. Provider profiles: language validity is separate from renderer availability

Mermaid, STEM/TeX/AsciiMath, HTML, and PDF are versioned ecosystem profiles, not AsciiDoc syntax switches. Valid core source stays valid when a provider is unavailable; BackendStatus reports unavailable/denied/incomplete and exact `.adoc` export can still succeed.

### 4.1 `MermaidProviderProfile/1`

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

`node.version` must satisfy >=22.13.0 and the executable digest must also be fixed. Missing an exact browser/font/config field makes the provider unavailable. CLI 12 uses `size` rather than width/height, `pdf-paper-format` rather than pdfFit, and `themeCSS` rather than cssFile. Default embedded fonts do not make arbitrary host fonts equivalent. Online `npx` dependency acquisition and floating-semver replacement dependency trees are forbidden.

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

A fresh managed Node begins at bindingRevision 1. Same-Workspace copy/fork with fresh identity creates a new revision-1 binding while preserving the exact profile generation. Formal same-Workspace restore restores the exact historical binding. Ordinary backup-byte import has no identity continuity: analyze BaselineOnly, produce the managed delta, obtain explicit admission, and create a new revision-1 binding. Only a real managed-profile migration increments bindingRevision. A source-unchanged profile-only migration creates one portable ChangeId but `sourceChanges=[]`; it does not increment SourceVersion or H(D,E).

`DocumentFormatCurrentQualification/1` binds a document-format DependencyKey/stamp, present ComponentImage, exact binding, and portable_metadata binding pin; the image version equals bindingRevision and the pin bytes equal D3-CJ/3(binding). `ManagedDocumentSemanticQualification/1` binds the current SourceObservation and this format qualification. Either changing makes old semantic projections and preparations stale.

## 6. D6 dependency, portable component, and completion successors

`DependencyKey/3` has the fixed rank order: source=0, document_format=1, lifecycle=2, placement_range=3, ref_inbound=4, relation_incidence=5, calendar_scope=6, registry=7, temporal_rules=8, authorization=9, foreign_binding=10, query_scan=11, replica_registry=12, conflict_record=13, execution_resource=14. Historical Key/2 stays fourteen-arm under its original ranks. `DependencyProof/3`, `InputDescriptor/3`, and `PreparedIntent/3` retain their prior responsibilities while consuming Key/3.

`PortableComponentKey/2` ranks document=0, document_format=1, resource=2, annotation=3, node_binding=4, child_list=5, lifecycle=6, trash_membership=7, policy=8, registry=9, period_scope=10, replica_registry=11, conflict=12. The document_format component contains exact `ManagedDocumentFormatBinding/1` bytes and its ComponentImage version is bindingRevision. PinRef/2 and ComponentImage/1 do not change.

`InstallationNotice/3` and `ContentCompletionProof/4` are the new-current component family. Historical Notice2/CP3 are not expanded in place.

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

## 7. D3, D4, and D5 current consumers

D3 current native requests use wire13 / `D3IdentityInput/13` with InputDescriptor/3. `D3DecisionCompanion/2` remains unchanged. `D3ResolutionInputUse/2` stores exact InputDescriptor/3; historical `/1` stores Descriptor2 only.

D4 current outer qualification consumes Descriptor3/Proof3/15-arm keys; a managed Document semantic operation includes both source(owner) and document_format(owner). Inner RelationReadContext/2, RelationReadBinding/2, occurrence keys, numeric sourceRevision, and Calendar/Registry contracts do not change version merely for this outer dependency.

D5 uses the same current outer proof for native tables, Fields, and strong collection operations. A format-stamp change makes an old preparation stale even when SourceVersion is unchanged. Table/row/cell locators, occurrence keys, RevisionTokenBinding, SourceObservation, and D5 intent grammar remain their existing types.

Trash retains the exact format binding; restore retains the historical binding; purge removes the document_format component in the same strict plan/Notice/P/CP as the owner closure. Retained history is not current qualification.

## 8. D7/D8/D9 current holders

PreparedActionBinding/4 carries current Descriptor3/Proof3/PreparedIntent3. MinimumMapping/3 is not mechanically version-bumped. Query/Action/CEL language syntax is unchanged; qualification/evidence is what versions.

EffectManifest/3 and EffectBytes/3 are one shared current closed transport family. They add the accepted Plan4/symbolic-json3 encodings while historical Plan1/Plan3 stay on their original decoder. This same family is consumed by D3 conflict preview and D10 author-preview hashing.

D8 PreparedEditBinding/3 uses current descriptor/proof. A Draft base binds SourceObservation plus DocumentFormatCurrentQualification. A format change preserves dirty Draft text/input log but invalidates the old semantic projection, map, preview, and prepared confirmation.

D9 ExportPlan/3 and PublicationReceipt/3 freeze current proof and semantic qualification. Exact-source-only export does not need an unrelated semantic parse; rendered/semantic export does. The current d9_probe format union does not silently acquire an adoc/asciidoc arm.

## 9. Independent portable JSON Annotation model

Annotation is a portable current value under D3 `AnnotationRef` identity and is not embedded in Document source. Every root/reply is a separate AnnotationRef; a thread is mechanically the same-owner `reply_closure` and cycles reject. Concurrent fresh replies may coexist; concurrent edits to one Annotation use revision/CAS/ConflictRecord rather than timestamp LWW.

### 9.1 `D3-Annotation-Value/4`

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

### 9.2 `D3-Annotation-Target-Projection/1`

The only identity/locator projection is the closed union:

```text
document        {kind:"document",owner:NodeRef}
document_element{kind:"document_element",locator:DocumentElementLocator}
document_range  {kind:"document_range",locator:DocumentRangeLocator}
resource        {kind:"resource",resourceRef:ResourceRef}
resource_region {kind:"resource_region",locator:ResourceRegionLocator}
```

Each variant allows only those fields. The owner/locator/resource must match AnnotationRef.owner. AnnotationRef, a bare target NodeRef, AuthorAnchorAddress, and cross-owner locators/refs are invalid. Outer wire and Projection/1 materialize bijectively; path/title/hash/ambient owner cannot fill missing identity.

### 9.3 `AnnotationInlineProfile/1`

```text
AnnotationInlineProfile/1 = {
  languageBaseline:"asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  doctype:"inline",processorBackend:"html5-semantic/1",safeMode:"secure",
  maxSourceBytes:65536,maxRenderedBytes:262144,maxInlineSemanticNodes:4096,
  managedAdapters:"disabled",networkEffects:"denied",fileEffects:"denied",processEffects:"denied"
}
```

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

### 9.5 `Suggestion/3`

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

Pending uses confirmed|needs_reconfirmation; accepted/rejected use not_applicable and are terminal. The basis is exactly SHA256(`Weftext-Suggestion-Target-Basis/1` + NUL + D3-CJ/3(complete stored target)); quote/prefix/context/display text are excluded. Replace uses a nonempty range, required expectedText, required replacementSource (which may be empty), and null affinity. Delete uses a nonempty range, required expectedText, null replacementSource and affinity. Insert uses a zero-width range, null expectedText, nonempty replacementSource, and left|right affinity.

## 10. Annotation targets and authorization

A source target binds its real owner, basis production SourceVersion, byte range or point affinity, expected bytes/context, and source provenance. Root-authored text belongs to the root Node; a managed included span belongs to the included Node. Unmanaged/network/synthetic multi-origin text cannot directly mint a durable exact target.

Resolution states distinguish exact, mapped, candidate, ambiguous, orphaned, and unavailable. Candidate/fuzzy/context search never authorizes a source write; repeated text with multiple candidates is ambiguous rather than “first” or “nearest.”

PDF/image regions bind exact ResourceVersion plus normalized rectangle. Audio/video uses rational timebase/ticks with `[start,end)`. Provider unavailability preserves existing raw annotation data but blocks creation of a new exact region when exact interpretation cannot be proven.

Historical quote/context reads require both Annotation disclosure and the corresponding historical-source disclosure. Bytes already lawfully delivered in a raw backup are not retroactively erased, while current online APIs continue to apply current authorization.

## 11. SourceTransform compiler and exact mapping

`PortableTransformCompilation/1` is total: representable events + afterSourceSha256 or an unavailable reason from the closed set provenance_gap, unsupported_transaction, invalid_utf8_boundary, generated_cross_anchor_edit, boundary_slot_unrepresentable, provenance_cycle, after_replay_mismatch, payload_digest_mismatch. An unavailable transform does not by itself block ordinary source save.

`SourceTransformPortableEvent/3` contains only replace and insert, all in original-before UTF-8 byte coordinates. Deletes are zero-length replacements. Generated-edit provenance folds into the originating event where representable; same-point inserts merge in transaction order; actual overlap may form a maximal island; boundary insertions remain separately ordered. Same-point order is left replacement ending at p, insert at p, right replacement starting at p.

The receiver computes deterministic generated output spans against the exact after pin, extracts payload bytes by span, verifies length/digest, and replays the events to the exact after bytes. Content search is forbidden.

For a non-zero range `[s,e)`, real replacement overlap or an insertion strictly inside the range stops exact mapping. Otherwise:

```text
s' = s + sum(delta(R), b<=s) + sum(len(I), q<=s)
e' = e + sum(delta(R), b<=e) + sum(len(I), q<e)
```

For point p, a replacement satisfying `a<=p<b` stops mapping; otherwise add deltas ending before/at p, inserts before p, and inserts at p only for right affinity. Each link in a transform chain independently revalidates source version, signature, trust cut, and result.

`CoreSourceEditPlan/2` freezes the `/3` events, exact before observation, exact after pin, and `TransformEmissionPlan/1`. Emission is either disabled with `no_exact_core_edit_plan|transform_profile_unavailable`, or required with exact profile, expectedTrustRevision, and expectedTrustKeyId. The caller cannot choose. A winning required plan cannot downgrade during recovery; a disabled plan cannot upgrade. Seal cannot recompile or reorder events.

## 12. Signed transform evidence and outbox

`SourceTransformEvidence/2` binds DecisionKey, ChangeId, owner, managed before/after SourceVersions, before/after hashes, the fixed coordinate/affinity profiles, and the exact `/3` event array. A required plan must satisfy byte-equality of D3-CJ/3(plan.edits) and D3-CJ/3(evidence.edits).

`SourceTransformSealArtifact/1` is Ed25519-signed under the `D6-Source-Transform-Seal/1` domain. The outbox key is exactly `{changeId,ownerNodeRef}` and the outbox item pins exact canonical artifact bytes as portable_metadata. A required sealed decision has exactly one item; disabled has zero; two different items for one key are an integrity conflict. Recovery republishes the exact pinned artifact by exact key and never searches by current hash/trust/source or resigns.

The transform artifact is not a PortableComponent and is not inserted into Notice/CP components. It shares the source decision's one P seal and ChangeId.

## 13. Trust profile, activation, and historical verification

A Workspace has one root/anchor. The closed seal profiles are `d6_revision_token_seal/1` and `d6_source_transform_seal/1`. Historical WorkspaceTrustDeclaration/1 remains revision-token-only. WorkspaceTrustDeclaration/2 is the current profile-discriminated successor. WorkspaceAuthorizationBundle/2 may contain a byte-exact `/1` prefix followed by a `/2` suffix; after the first `/2`, the chain never returns to `/1`.

Declaration1 activation is still derived through its original DecisionKey -> CP3/ChangeRecord evidence. Declaration2 activation is derived through its DecisionKey -> committed CP4 + ChangeRecord/1 + the policy after-image that first appends the exact declaration sequence. Same-DecisionKey declarations share the activation ChangeId and are all-in/all-out in history_at(C).

TrustConflictCarry/2 rederives its origin by loading and validating the original Declaration2 and original CP4/ChangeRecord; its stored originActivationChangeId is not self-authenticating. Carry1 retains the original Declaration1+CP3 rule. A later resolver cut never replaces the origin cut.

A normal SourceTransform receiver validates the producing ChangeRecord/CP4 and sets `C=CP4.frontierBefore` for historical key verification. Later ordinary rotation does not invalidate a historical artifact that was valid at C. Compromise uses causal order between the artifact seal ChangeId and the original compromise activation ChangeId; arrival time, current bundle, or frontierAfter are not substitutes.

DomainSealKeyHandle/1 can continue signing revision tokens when the exact domain/profile/key remains current, safe, and usable, even after Bundle2 exists. It never gains transform authority or gets silently reencoded as Handle2. New or rotated Declaration2 keys use Handle2.

Fresh bootstrap uses one root and exactly two profile declarations, revision-token then transform, with one DecisionKey/activation ChangeId and no observable intermediate prefix. Replica registration is likewise dual-profile. Continuation evaluates current(K)|none|conflicted/unproved separately for both profiles; if either is unproved, no new signing domain activates. Otherwise old revokes (in fixed profile order when present) precede new revision and transform authorizations, all under one DecisionKey/CP4.

## 14. D10 mixed-version control and execution responsibility

Current Workspace reads use D10WorkspaceReadDependencies/2 with DependencyProof/3. ControlDependencies/3, D10ControlInput/2, and ControlPrepareBinding/3 connect that evidence to the D6 current descriptor/intent family.

D10ControlRecordImage/2 is only for automation and run records whose nested schema changed. Other record kinds, including external_effect, stop, reservation, planned/external approval, activation, and supplement=none records, may remain exact Image1 current records. Versioned image/pin/range carriers explicitly retain exact V1/V2 values. Pin1 continues to authenticate `D10-Control-Record/1 || NUL || D3-CJ/3(Image1)`; Pin2 uses the `/2` domain. Historical pins are never repinned into a new domain.

D10ControlEffectPlan/2 accepts an actual Image1 or Image2 before-image and the exact current after-image, which makes Image1(Automation+Subscription1) -> Image2(Automation+Subscription2) a valid configure path without a preliminary migration transaction.

The prepare-binding mixed carrier is exactly three arms:

```text
D10VersionedControlPrepareBinding/1 =
  {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
| {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
| {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}
```

The historical `/1` value is exactly `{key,canonicalIntentBytes,allocatedControlRefs,originalCommitRequest,immutablePreview,dependencyPins:ControlDependencies/1}`. e8aa explicitly retains the exact decoder/recovery for true saved `/1` records; this does not assert universal deployment. Claims2 sorts all three by inner StableControlKey and rejects the same key across versions. The wrapper only selects a decoder; it does not migrate bytes, grant authority, create a CAS, or alter original dependencies/pins.

D10MoneyResponsibility/2, D10ExecutionClaims/2, and D10ExecutionInventory/2 use version-dispatched pins/ranges/preparations/author steps/subscriptions/occurrences/ApprovalUses. ExternalResponsibility/1, StopResponsibility/1, and StopCapacity/1 remain unchanged because their closed members do not embed the changed types, but Inventory2 must include them completely.

A legal Inventory2 may simultaneously retain old ApprovalUse1/PAB3/Subscription1/Occurrence1/Pin1/PrepareBinding1-or-2 and new ApprovalUse2/PAB4/Subscription2/Occurrence2/Pin2/PrepareBinding3 when semantic identities differ. The same DecisionKey, StableControlKey, (Automation,generation), OccurrenceKey, or same record cut cannot appear twice across versions; no LWW applies.

The D6 current successor is ExecutionResponsibilityRecord/3 + ExecutionContinuityProof/2, with exact inventory-pin domain `D6-Execution-Inventory/2`. Historical Record2/Proof1/Inventory1 remain on their own domains. A Record2 becomes Record3 only on a real responsibility mutation, checkpoint, or custody handoff, never by background schema migration. Handoff freezes admission/planning/send/schedule writers at one store barrier, captures all old and new liabilities, and proves irreversible old-holder fencing. Missing an old pin/PAB/subscription/stop/external unknown pauses takeover; it does not fabricate a partial inventory or block unrelated ordinary source work.

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

Missing a transform artifact, index row, provider, or strong proof is not a reason for a permanent generic `owner_update_required` once this coordination is present. Each affected path uses its actual unavailable/conflict error while raw source, Draft, repair, and other independent paths remain available under their existing qualifications.

## 18. Schema-version inventory

Current additions/successors include ManagedDocumentFormatProfile/1, ManagedDocumentFormatBinding/1, DocumentFormatCurrentQualification/1, ManagedDocumentSemanticQualification/1, OracleSemanticWitness/7, CoreSemanticProjection/1, DependencyKey/3, DependencyProof/3, InputDescriptor/3, PreparedIntent/3, PortableComponentKey/2, InstallationNotice/3, ContentCompletionProof/4, ChangeRecord/1, D3IdentityInput/13, D3ResolutionInputUse/2, PreparedActionBinding/4, EffectManifest/3, EffectBytes/3, PreparedEditBinding/3, ExportPlan/3, PublicationReceipt/3, PortableTransformCompilation/1, SourceTransformPortableEvent/3, CoreSourceEditPlan/2, TransformEmissionPlan/1, SourceTransformEvidence/2, SourceTransformSealArtifact/1, SourceTransformSealOutboxItem/1, WorkspaceTrustDeclaration/2, WorkspaceAuthorizationBundle/2, DomainSealKeyHandle/2, WorkspaceTrustGenesis/2, WorkspaceBootstrapPlan/4, D10WorkspaceReadDependencies/2, ControlDependencies/3, D10ControlInput/2, ControlPrepareBinding/3, D10ControlRecordImage/2, D10ControlRecordPin/2, D10ControlRange/2, D10ControlEffectPlan/2, ApprovalUse/2, D10AuthorStepResponsibility/2, ScheduleRecurrenceEvidence/2, ScheduleSubscription/2, ScheduleOccurrenceProof/2, AutomationOccurrenceRecord/2, D10MoneyResponsibility/2, D10ExecutionClaims/2, D10ExecutionInventory/2, ExecutionResponsibilityRecord/3, and ExecutionContinuityProof/2.

Key retained types include PinRef/2, ComponentImage/1, OwnerInputBinding/2, ObservationScope/2, D3DecisionCompanion/2, D4/D5 inner selector/locator types, WorkspaceTrustRootDeclaration/1, WorkspaceTrustAnchor/1, RevisionTokenSealVerificationKey/1, LeaseRunUse/1, D10ExternalResponsibility/1, D10StopResponsibility/1, and StopCapacity/1. Historical decoders are never expanded in place.

## 19. Acceptance and evidence boundary

The itemized design obligations are in `ACCEPTANCE.zh-CN.md` / `ACCEPTANCE.md` and `acceptance-matrix.json`: 437 core design oracles and 117 actual-owner coordination fixtures, 554 obligations total. They are **unexecuted design obligations**, not implementation test results.

This PR did not install or execute the Ruby oracle, Asciidork, Mermaid CLI, browser/Puppeteer, STEM/PDF providers, product SQLite/storage code, replica/crash/crypto paths, real Automation scheduling, or execution-custody handoff. Author checks are limited to document/JSON/router consistency. Independent design review must bind to the exact stopped PR head SHA.