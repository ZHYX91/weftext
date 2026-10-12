---
source_language: zh-CN
translation_of: EDITOR.zh-CN.md
translation_status: synced
---

[简体中文](EDITOR.zh-CN.md)

# Typst reading, editing and independent annotations

Status: **coordinated design candidate; not implemented or independently accepted**. Fixed input `af6737a057c7bb69763dfaad0a57da4652e085f3`, author branch `design/typst-native-20261012`. This document owns reading/editing behavior, edit eligibility and annotation interaction. [SOURCE](SOURCE.md) owns official semantics and origin projections; [CORE](CORE.md) owns closed holders, authorization, transactions and recovery; [FEATURES](FEATURES.md) §§2–4 exclusively owns addresses, link conversion and potential-match algorithms. These versions are new Typst families, not extensions to old D8/AsciiDoc decoders.

## 1. Author source, surfaces and state

Each Node's exact `.typ` bytes are its sole body authority. Source, rich editing, flowing reading, print preview and properties share one owner, Base and Draft. Switching surfaces neither commits nor discards drafts or recreates identity. Ordinary valid Typst read/save eligibility is separate from control coverage. Titles may be absent; filenames do not supply an authored `title`. Macros, imports, templates, complex tables and missing controls do not justify rejecting the whole file. Report language validity, root control/typed fields, compilation/layout availability and operation permissions separately.

A session maintains four axes: complete `SourceObservation/1` and exact Base; monotonic `draftSerial` and exact Draft; IME/input generation, inputSerial and selection; current read/write eligibility, prepare, preview and submission/recovery. Draft is an in-memory non-authoritative proposal. An encrypted local recovery draft cannot serve as current source or a cross-device permission ticket. Source, rich editing and properties share one Undo/Redo stack; synchronization, Agent and other authors' commits do not enter it.

A successful receipt proves the original decision; it cannot overwrite subsequent Draft input. Update Base only after reauthorized reading of after; mark clean only when the current Draft exactly equals after. Every Undo uses a new serial. Production versions, tokens, SourceMaps, selectors and prepared evidence never revive through equal bytes. Revocation immediately stops protected delivery, decorations and writes. Preserve the user's recovery input without redisplaying protected original content from cache before reauthorization.

## 2. Delivery backend for continuous reading

The default production target is **`flow-semantic/1`**: SOURCE's official Rust Typst **0.15.1** evaluated/compiled projection → Weftext semantic reading tree → controlled DOM/native accessible nodes and reading styles. It presents already evaluated elements, content, order and origins; it does not reinterpret source, execute functions or substitute AST text for post-`show` results. Weftext must implement this renderer; selecting it is not completion. Exact compiler, standard-library, font, resource, package, projection and renderer pins are frozen with the CORE carrier. Upgrades rerun the matrix and never use `latest`.

The production minimum matrix must deliver Chinese/English/RTL paragraphs and soft wraps; title; actual heading levels (controls for 1–9 without invalidating deeper legal levels); strong/emph/strike; native inline/display math; bullet/numbered/term lists; native links and internal references; image alt/caption; simple and header/span tables; raw/code; readable islands for content that cannot reflow. Ordinary text adapts to container width and scrolls continuously without per-page white sheets or a PDF viewer dependency. Math and complex layout may use local SVG frames from the same official paginated compiler, preserving official content boundaries, semantic descriptions/accessibility alternatives and exact source navigation. Whole-document screenshots are not flowing reading. Wide tables/diagrams may scroll locally with an accessible equivalent while body text reflows at 320 CSS px.

Official evaluation decides `show`/`set` content. Flow geometry follows reading layout; print geometry follows official pagination. Flow does not promise arbitrary page-coordinate, header/footer, float, absolute-placement or pagination-counter fidelity. Present those as local layout previews with a “print-specific” explanation. If an official element lacks trustworthy reading order/alternative content, mark its island `layout_unavailable` and offer source/print navigation rather than inventing paragraphs. Opaque official content must have a bounded island, never disappear. Missing basic matrix capabilities or whole ordinary documents reduced to Source/PDF block the flowing-reading milestone.

**`html-experimental/1` is an optional experimental preview, never the only production implementation or acceptance condition.** Official documentation describes incomplete HTML output unsuitable for production, primarily semantic markup rather than all CSS. Pin the same official `typst-html` version; native target-specific show rules are allowed. Display experimental status and capability/loss, and do not infer production coverage from one successful sample. Every experimental HTML fragment is validated into the reading tree rather than executing authored HTML. [Official HTML status](https://typst.app/docs/reference/html/), [pinned Rust HTML crate](https://docs.rs/typst-html/0.15.1/typst_html/).

Both backends enforce static-output security: reject scripts/event handlers, iframe/object/embed, form submission, author-active CSS/external fonts/network fetch, `javascript:` and unapproved protocols. SVG also removes scripts, foreignObject and external references. Only the renderer's fixed tags/attributes/styles are admitted. Body text is text nodes; resources use Core-authorized pins/controlled delivery addresses; link activation invokes the FEATURES resolver. WebUI uses CSP and an isolated unprivileged origin with a scriptless sandbox; interaction lives in the host overlay. Desktop native/controlled webviews similarly obtain no file/network/process capability. Report sanitization losses explicitly rather than deleting author content and claiming lossless output.

Every render binds carrier, source Observation, draftSerial when applicable, projection, resource/authorization cut and `layoutEpoch`. An explicitly opened old preview is labeled “old version”; it cannot supply current edit/annotation targets or replace newer input. Cancellation/budget failure preserves Draft; errors, partial, stale, unavailable and complete empty documents remain distinct.

### 2.1 Deliverable semantic projection artifact

The producer of `TypstSemanticProjection/1` is Core's evaluation/realization bridge inside the pinned official Rust build; consumers are the flow renderer, body Query and EditMap eligibility checks. If public APIs cannot expose post-show semantic structure/origins, implementation must add a named observation-only hook at the pinned official evaluation/realization boundary. Pin its version/digest in the build environment and compare output with the uninstrumented official compiler. Never evaluate source independently, extract text from PDF or rename pre-show AST as Content. This hook and renderer are implementation/acceptance obligations, not claims that upstream supplies a complete editor.

```text
TypstSemanticProjection/1 = {
 kind:"typst_semantic_projection",version:1,profile:TypstProfileCarrier/1,
 ownerNodeRef:NodeRef,sourceObservation:SourceObservation/1,
 environmentPin:PinRef/2,worldProofPin:PinRef/2,
 originEvidencePin:PinRef/2,root:[SemanticNode/1...],
 introspection:[SemanticIntrospectionEntry/1...],navigation:[SemanticNavigationEntry/1...],
 links:[FEATURES.TypstCompiledLinkOccurrence/1...],
 linkAssociations:[FEATURES.TypstLinkAssociation/1...]
}
SemanticNode/1 = {key:Counter,originIds:[Counter...],direction:"ltr"|"rtl",
 presentation:"block"|"inline",element:SemanticElement/1}
SemanticElement/1 =
 {kind:"text",text:text}
 |{kind:"paragraph",children:[SemanticNode/1...]}
 |{kind:"title",children:[SemanticNode/1...]}
 |{kind:"heading",level:PositiveInt,outlined:boolean,numberingText:null|text,
   children:[SemanticNode/1...]}
 |{kind:"format",style:"strong"|"emph"|"strike",children:[SemanticNode/1...]}
 |{kind:"math",display:boolean,framePin:PinRef/2,semanticText:null|text}
 |{kind:"list",style:"bullet"|"numbered"|"terms",items:[SemanticListItem/1...]}
 |{kind:"table",columns:Counter,rows:[SemanticTableRow/1...]}
 |{kind:"link",compiledIndex:Counter,destination:SemanticNativeDestination/1,
   children:[SemanticNode/1...]}
 |{kind:"resource",resourceRef:ResourceRef,bytesPin:PinRef/2,alt:null|text,
    caption:[SemanticNode/1...]}
 |{kind:"raw",text:text,language:null|text,block:boolean}
 |{kind:"island",nativeKind:text,framePin:null|PinRef/2,
    semanticText:null|text,reason:"print_specific"|"unsupported_element"|
    "reading_order_unavailable"}
SemanticListItem/1 = {key:Counter,originIds:[Counter...],
 marker:null|text,term:[SemanticNode/1...],body:[SemanticNode/1...]}
SemanticTableRow/1 = {key:Counter,originIds:[Counter...],
 section:"header"|"body"|"footer",cells:[SemanticTableCell/1...]}
SemanticTableCell/1 = {key:Counter,originIds:[Counter...],column:Counter,
 colspan:PositiveInt,rowspan:PositiveInt,children:[SemanticNode/1...]}
SemanticNativeDestination/1 = {kind:"uri",value:text}
 |{kind:"label",value:text,finalLocationKey:null|Counter}
 |{kind:"location",finalLocationKey:Counter}
 |{kind:"page_point",page:PositiveInt,xPointsBits:text,yPointsBits:text}
SemanticIntrospectionEntry/1 = {key:Counter,originIds:[Counter...],
 nativeKind:"heading"|"other",headingLevel:null|PositiveInt,
 bodyText:null|text,readingNodeKey:null|Counter}
SemanticNavigationEntry/1 = {key:Counter,readingNodeKey:null|Counter,
 page:PositiveInt,xPointsBits:text,yPointsBits:text}
```

All objects are closed. Keys are globally unique inside this artifact and assigned in official logical order, never persistent identities. root/children/items/rows/cells retain official reading order, never DOM/hash/visual-x ordering. PositiveInt is an official positive integer encoded as canonical decimal text, without the control's nine-level limit or JSON precision loss. Table columns must be positive and cell spans in bounds/nonoverlapping, matching the official grid; an unproved table becomes an island rather than invented cells. originIds refer strictly to real SOURCE origins in the same originEvidencePin, never caller ranges. Pins share owner/evaluation/environment/World cut. link destination is an official native value subsequently qualified by FEATURES, not read/write authority. framePin is controlled same-build official paginated SVG output. If official content cannot prove semanticText, use null and disclose missing alternatives rather than guessing math speech. Unknown elements become explained islands; missing basic matrix coverage still fails acceptance. CORE pins the actual official paged/html compilation target; flow-semantic selects a presentation backend only. Cross-target reuse proves equal resource cuts rather than sharing old tokens.

Capture specifically follows official evaluate → original element/effective style/scope at realize prepare/visit_show_rules → returned/realized Content from recipe/builtin execution → visit_styled and official Introspector/Frame actual text, reading occurrences, positions and origins. Publish only the final converged Introspector round of a successful compilation, never early temporary values/empty output. The bridge correlates entry/return within one trace without rerunning rules. Display payload comes only from return/Frame; semantic roles require an officially retained semantic element or individually proven transparent rule. Default heading/strong realization provides actual heading level/outlined/numberingText and strong children; unchanged ordinary text retains exact origins. `#show heading: it => [X #it.body]` must display actual official X+body with generated X. If retained heading role/level cannot be proved, it is a local island with original-heading navigation, never pre-show body/roles presented as post-show headings. `#show strong: it => it.body` never falsely remains bold. Numbering/counters/direction/inline-block placement come from final trace/resolved style; numberingText is not computed by counting headings. Unowned counter/state output becomes an island.

Production flow-semantic binds official **Paged** target by default. `#context target()` has the same official paged semantics as PDF; flow is a presentation backend, never a new Typst Target or forked standard-library target dispatch. `if target()=="html" { [网页] } else { [分页] }` yields “Paged” for default flow and print, and “Web” only for experimental HTML; independently qualify each target's World/resource trace. Flow reflows Paged semantics without secretly replacing it with Html. Qualified native run-in rules must prove semantic entry→return correspondence, actual level/numbering, body order and inline placement before emitting a heading beside its paragraph. Arbitrary show rules do not promise reversibility or retained roles. If the pinned official build cannot expose this trace, the bridge milestone remains incomplete rather than claiming completion through AST/experimental HTML. Resolved direction/placement is mandatory; font/color/page geometry follows explicit flow-loss policies, not print fidelity claims.

Official Content exists only in-process during Core's evaluation, never as a PinRef/portable JSON. CORE `TypstCompilationEvidence/1` freezes complete source/environment/world/semanticProjection/origin/output pins. Recovery can read retained artifacts without reviving old maps/tokens.

`SemanticNativeDestination/1` preserves distinct official string/label/location/page-coordinate types rather than coercing labels/locations to URLs. finalLocationKey indexes only this final official Introspector/Frame navigation trace. Missing label targets preserve value with key=null and unavailable navigation; locations cannot reference another compilation. Page points use resolved absolute pt values encoded as finite IEEE754 binary64 lowercase16-hex bit patterns with page≥1. Navigation explicitly opens the same print layout, never a source address. FEATURES requalifies application URI/current labels/permissions; semantic destinations/pins grant no permission. [Official link destination types](https://typst.app/docs/reference/model/link/).

`introspection` and display root are distinct: HeadingElem entries genuinely retained by the final official Introspector still provide actual headingLevel/official bodyText/origins when opaque show output becomes an island; readingNodeKey points to that island or null. Display limitations never erase those Query rows. bodyText=null means the column is unavailable, never empty text; other requires headingLevel=null. Heading Queries consume introspection while body reading text consumes actual returned root content. These are named heading projections, not a complete arbitrary-field introspection API; other Query columns need their owner's named projection or report unavailable. Query keys are result-epoch scoped, granting neither rich writes nor persistent locators.

navigation is a closed recoverable projection of final trace with unique keys and page/points equal to the same official Frames. readingNodeKey is nonnull only for a proven node correspondence. finalLocationKey must reference its actual entry. Out-of-scope/incomplete/unprojectable navigation is unavailable without same-label/display-text searches. Both arrays and the reading tree share one compilation evidence binding; reading retained artifacts reauthorizes/rechecks environment without reviving runtime addresses.

### 2.2 Actual link-occurrence projection slots

FEATURES §2.4 exclusively owns closed TypstAuthoredLinkKey/1, TypstAuthoredLinkCatalog/1, TypstCompiledLinkOccurrence/1 and TypstLinkAssociation/1 types/key algorithms. SOURCE origin records/reading node keys never substitute for link occurrences. Core's official AST/standard-binding read produces root-authored catalogs without entering include/function/loop outputs; independently verify exact production version, Observation, pin and complete enumeration. Current Draft writers freshly rebuild §3 maps and official AST eligibility rather than renewing new Draft source through old committed-catalog keys.

Core's final-converged realization bridge produces Projection.links/linkAssociations: one continuous index=0..N-1 per actual logical native-link trace occurrence, including all layout fragments. Repeated include/show/function output has different indices even with identical physical origins. Each index has exactly one association; unique_root requires complete AST/trace proof of one actual same-owner root key. Preserve actual include/generated/multi_origin/unavailable classifications without guessing from text or originIds counts. removed_by_show is retained but not clickable; unavailable never pretends active. Each SemanticElement.link compiledIndex references an active occurrence in this Projection, with equal destination and a node key genuinely present in readingNodeKeys; navigationKeys binds final navigation trace. Incomplete link traces cannot be marked complete: navigation/full-coverage export is unavailable rather than inventing an empty links array.

Renderer, authorized link navigation and FEATURES export-route consumers share the complete source/environment/World/current-authorization basis of this Projection and CORE compilation evidence. Bare indices cannot cross compilations. Root writers/typed transfer consume eligible authored catalogs and current EDITOR source-write proofs; compiled occurrences are derived reading/navigation/output, never stable author edges or source-write authority. Exporter revalidates every link/association against FEATURES' sole complete-occurrence-key/route-coverage rules. Duplicate/missing/wrong-basis/old-map routes fail without merging occurrences by equal destinations.

## 3. Official origins to editable regions

SOURCE produces official AST, BodyProjection and SourceMap. EDITOR consumes these into an EditMap valid only inside this Draft. Positions bind owner, Base Observation, carrier, draftSerial and exactSourceDigest; the digest is a cross-check, not exact-source/current-Observation evidence. Explicitly convert byte, UTF-16, Unicode scalar, grapheme and visual-glyph coordinates. Source ranges are half-open UTF-8 bytes; CRLF is indivisible. Navigation and write maps are separate.

| SOURCE origin | Reading/navigation | Rich-write eligibility |
| --- | --- | --- |
| `rootLiteral` | Exact current-root text/value | Writable when unique range, boundary and semantic proofs hold |
| `rootConstruct` | Current-root native strong/heading/link/table construct | Only a named adapter proving unique parameter/body ranges and the complete rewrite may write |
| `generated` | Official expression/loop/show result and known real origins | Read-only by default with “edit source”; never reverse displayed strings |
| `include` | Actual included owner/source, separately authorized | Read-only in this document; explicitly open a new session for that owner and qualify its source |
| `multiOrigin` | Multiple real origins and explanation | Read-only; never choose a default origin |

A Span or rootConstruct does not grant scalar writes to rebound standard functions, reused variables, repeated output from one source, irreversible escapes/smartquotes/text transforms, show deletion/reordering/concatenation or hidden origins. Equal displayed text has no qualifying effect. A macro also does not disable independent ordinary regions: adapters build dependency closure from official data flow and active show/set scopes. Affected/unproved regions are read-only while fully proven independent paragraphs remain editable. A global rule affecting all body content can legitimately make all rich regions read-only when proof is unavailable; Source editing remains available. Seeing an import alone is not a whole-file rejection rule.

Each EditMap flow is one logical inline slot with a projection-local unique `flowKey`, logicalText and source segments classified `exactScalar|escaped|atom|generated`. Only exactScalar or a named reversible codec may directly write. Each structural region has official AST path, exact range, constructKind, operation capabilities and explanations. A site is a parser-proven sibling insertion boundary. Array indices, table row numbers, AST paths and DOM keys are revision/Draft scoped, never persistent identities. Reading selections crossing islands may be copied; writing requires eligibility for the complete selection.

### 3.1 Actual producer for empty content and insertion positions

The Core adapter for `TypstDraftProjection/1` produces sites from the complete current official AST, independently of whether the semantic tree has text leaves. Empty body after a valid root declaration, blank-only body, body after deleting its last paragraph and exact root sibling boundaries expose at least one root_body site. Eligible existing native paragraphs/list items/content blocks with empty bodies expose empty_inline sites. Whitespace inside metadata, labels, comments or code expressions never becomes a body site. Standard constructs must be unrebound with complete scope/read-write/boundary proofs; an error that may consume a boundary prevents site issuance. Official AST navigation and exact bytes establish zero-width sourceRange: root_body belongs only to legal body sibling boundaries and empty_inline to an exact empty content slot. Callers provide no byte offsets. Metadata-only bodies without flows/regions still display an empty input position bound to this site.

```text
TypstInsertionSite/1 = {
 siteKey:Counter,kind:"root_body"|"empty_inline",
 parentPath:[Counter...],slotPath:null|[Counter...],
 sourceRange:SOURCE.TypstByteRange/1,
 capabilities:["literal"|"inline_math"|"display_math"|"list"...],
 binding:DraftMapBinding
}
TypstInsertionPosition/1 = {kind:"flow",flowKey:Counter,offsetScalar:Counter}
 |{kind:"site",siteKey:Counter}
TypstListCreationTarget/1 = {kind:"paragraph",regionKey:Counter}
 |{kind:"site",siteKey:Counter}
```

DraftMapBinding is exactly §7's existing binding. siteKey is unique in this projection; enumerate sites by actual byte position, fixed kind rank and canonical AST path. Capabilities are unique in literal/inline_math/display_math/list order. root_body requires slotPath=null; empty_inline requires an exact empty slot. Core recomputes and compares every site member when processing an operation. Disappearance, wrong kind/capability or changed owner/Base/carrier/serial/source rejects; equal byte positions cannot renew a site. Flow positions require unique reversible segments and grapheme boundaries in the same map, never generated/atom interiors.

## 4. Basic bidirectional editing and byte preservation

Deliver the following ordinary writing capabilities; advanced valid source has its own route, but these basics cannot all be deferred. UI actions create named intents. Core adapters encode valid native Typst and fully reparse with the official parser before returning a new Draft, EditMap and proven caret. Never serialize DOM `innerHTML` back to `.typ` or pretty-print the whole file.

| Control/operation | Native source and positive scope |
| --- | --- |
| Body input/delete, Enter/paragraph merge, empty body/delete-to-empty/retype | Root-authored paragraphs; native literal encoding, preserved existing EOL/whitespace and explicit document policy for generated EOL |
| Strong/emphasis/strike toggles and content editing | Native strong/emph/strike or unrebound standard calls; exact unambiguous inline selection, preserving outside comments/trivia |
| Title and heading levels 1–9, promote/demote | Native `title`/`heading` or `=` markup; title and section remain separate; font size never determines level |
| Inline/display math creation/edit | Local native math-source editor and official preview; math is a selection atom whose internals use the math-source adapter, never rendered glyphs |
| Bullet/numbered/term lists, new/split/merge/indent items | Native list/enum/terms; unique root list/item region and exact cross-item preview; pasted `-` text is not executable list syntax |
| Table cell edits, row insertion/removal/reorder, simple column insertion/removal | Unrebound standard `table`, proven fixed positive columns and complete explicit cells; rectangular simple grids without spans/header/footer expose row/column commands through native argument/content ranges |
| Headers/spans/block cells | Read official semantics; edit individually unique-origin cells; source/advanced entry for topology-changing span/header/footer actions without flattening or automatic repair |
| Link insertion/conversion | FEATURES §§2–3 native `link` encoding and same-target validation; independent source-write and target-read checks |

Each rich operation freezes complete before slice, selection, kind/parameters and map binding. Text input uses literal encoding; style/list/math/link commands use their construct codecs. Mixing them cannot inject code. Success proves every outside-selection byte unchanged; complete official AST reparse succeeds, except §6's explicitly bounded error-repair branch; target structure/value exactly matches intent; non-target control declarations, fields, AST subtrees/origins are unchanged; new scalars/constructs derive from this operation; and the caret is uniquely established in the new map. Active show transformations require fresh official evaluation and eligibility, not AST equality alone.

Selections across strong/emphasis boundaries within one paragraph use inline-tree split/merge encoding of the target region while preserving unselected leaves' exact bytes. Cross-paragraph/list deletion, splitting and text/plain paste process the whole target through one structural intent. Selections across title/heading/table/code/math/generated islands never silently flatten: logical copying is available, while writes without a named complete-selection operation explain the boundary and offer an exact Source selection. Equal table cells are distinguished by current AST cell paths.

Table reorder establishes row trivia from the official AST argument sequence: cell-internal comments move with that cell; comments/whitespace between the previous row's last cell and next row's first cell are the next row's leading trivia; trivia after the final row up to the closing delimiter stays at the table tail. Table arguments/columns/header/footer do not move with rows. Native codecs minimally regenerate commas/necessary separators while preserving comment content/EOL bytes. Cross-row expressions, spread/generated cells, unproved ownership or header/span semantic changes reject only that reorder, never all tables containing comments. SOURCE property arrays and document tables are separate. Retain six row domains: native tables, Field occurrences, NodeCollections, derived Query rows, external/import IR and Resource previews. No RecordRef or row UUID. Structured/table budgets follow FEATURES' D5 continuation; source intents cannot split batches to bypass them.

Explicit “paste into table” accepts rectangular text/plain TSV only for simple grids with complete target cells and representable literals. Parse cells once and produce one atomic Draft operation with dimension/extension preview. Budget failure or any unqualified cell leaves the entire transaction unchanged. Ordinary paste remains literal text. Column reorder, generated formula columns, automatic ragged repair and HTML/RTF rich paste need separate future mapping/loss and acceptance. They never gate legal complex-table read/save.

Creation commands are explicit new §7 operation-union arms: `insert_at_site{siteKey,text}`, `insert_math{position:TypstInsertionPosition/1,mathSource,display:boolean}` and `convert_to_list{target:TypstListCreationTarget/1,style:bullet|numbered|terms,termText:null|text}`. They carry the same TypstEditOperation binding/source, never private UI selection as missing coordinates. insert_at_site literal-encodes nonempty text; paragraph breaks produce exact ordinary paragraphs, never macros/headings/lists through `#`, `=` or `-` or escapes. For zero-width sites, Core tries nine combinations of0/1/2 prefix and suffix EOLs, choosing the valid full-official-reparse/target-proof candidate with fewest new EOLs then smallest prefix. Preserve existing EOL/comments/control and every outside-site byte. No valid candidate leaves the whole command unchanged.

insert_math inserts a standard math construct at an exact paragraph-flow scalar caret or eligible site. Inline does not split paragraphs. Display explicitly splits the target paragraph at the caret into original-text-preserving before/after paragraphs, omitting empty sides, with the display equation between them. mathSource must be fully consumed by pinned official math grammar and wrapped/encoded by the adapter; it cannot close a delimiter and append code. `甲乙` at offsetScalar=1 can produce `甲$x$乙`; display minimally generates paragraph boundaries between the two existing characters without changing their bytes and previews the intended topology change. edit_math still edits existing equations, never masquerades as insertion.

convert_to_list's paragraph target converts the complete exact ordinary paragraph to its first item, preserving body/inline leaves' bytes while generating only native markers or eligible standard list/enum/terms constructs and necessary separators. A site target creates one explicit empty item and returns that item's empty_inline site for continued typing. Empty items use officially supported explicit empty-content constructs; an unconsumed bare marker is not a list. The complete official AST must contain the actual list/item. terms requires nonnull literal-encoded termText, which may be empty; bullet/numbered requires null. Original paragraph content becomes the body without guessing a term/body split. Issue the new site only for that current native item. Reparse/evaluation inconsistent with intent retains before Draft. Each creation is one atomic Undo. Dispatch only after IME composition finishes; stale sites/flows/serials or any partial failure never insert half a construct.

Standard empty-item candidates are `#list([])`, `#enum([])` and `#terms(terms.item([], []))`. The last has separate exact AST paths for its empty term/body content slots. Paragraph conversion wraps original exact content in the same construct while encoding termText independently as a literal. Validate source position/standard bindings in the current scope, then officially reparse/compile to prove one actual empty item. Candidate strings themselves grant no eligibility. [List](https://typst.app/docs/reference/model/list/), [Enum](https://typst.app/docs/reference/model/enum/), [Terms](https://typst.app/docs/reference/model/terms/).

## 5. Input, clipboard, Undo and bidirectional navigation

Each controller has at most one transform in flight, freezing owner/Base/carrier/source/selection, generation, inputSerial, draftSerial, nativeAfter and successor input log. Late responses complete only their own generation. Reapply/requalify successor inputs or preserve unapplied native draft input rather than overwriting it. Map/official origins drive layout hit → logical selection → source range, and Source caret → official origin → current layout stop. When mapping fails, show the Source position/reason, never search equal text. Relayout updates layoutEpoch only.

Chinese IME begin/update is local preedit, unavailable for prepare, link conversion or structural commands. Composition commit/end/final input makes one Draft transaction and one Undo group. Preserve composition focus/virtualized item; Enter/save/slash cannot take ownership. Cancellation, blur or backgrounding does not manufacture a commit. There is one host composition owner. Undo/Redo creates new projection/serial. Paste, one style command, table edit, link conversion and property operation each form one visible Undo unit.

Copy Text follows logical reading order; Copy Source Fragment uses currently authorized exact source, never synthetic joins. Explicit table TSV copying does not grant resource download. Cross-block/style Copy Text uses explicit logical block separators and a chosen flavor without hidden content. Default Paste consumes only text/plain with literal encoding; HTML/RTF/scripts/macros never execute. Without a plain flavor report unavailable. Cut writes the real OS clipboard successfully before changing Draft; failure preserves selection/source. Source-mode paste is explicit source editing and may contain valid functions/macros, still subject to save gates. Identify this mode rather than treating source paste as rich literal paste.

## 6. Error partitions, Source editing and save

Official parser error nodes and scope boundaries determine error regions. Control declarations, fields and body have separate validity. Body compilation failure does not erase an intact root AST declaration or mark old body/derived values current. Authorized complete source_read always allows exact Source Draft editing and repair of preserved bytes. Malformed UTF-8 uses CORE physical repair rather than invented AST diagnostics.

Syntax-error islands offer Source repair. Other paragraphs permit rich input only with official AST proof of complete boundaries, unaffected scope/dependencies and all target/non-target invariants. The Draft remains syntax-invalid rather than pretending the whole file is valid. With syntax-valid compilation errors, direct-root AST literals remain independently editable; evaluation-dependent slots stay unavailable while ordinary regions with complete independent proofs remain editable. An error that may consume following content cannot establish a safe partition.

Whole-source save does not require every rich region to be writable, a successful renderer, whole-library Query completeness or graphical controls. It requires CORE current format/identity/applicable local typed/authorization/install gates and SOURCE language/control validation. Syntax-valid body compilation errors may be saved as author drafts with diagnostics, never described as successful compilation/export. Ordinary syntax-invalid save cannot claim validity. If CORE does not qualify an explicit repair/checkpoint, retain only a recovery Draft. Structured controls cannot bypass strong gates under a whole-source label. ordinary/complete and strict/observed_only are separate axes. observed_only follows CORE's previously selected trusted-human single-existing-Document conditions, never fallback after strict failure. Hosted writes use Server; an offline parser grants no hosted commit authority.

Prepare freezes complete proposed source/pins, Observation, carrier, typed/environment/authorization dependencies and operation. Preview shows all effects/exact bytes; a page is not the complete preview. Confirm/commit uses one OperationId, original D6 `PreparedIntent/4`, `d6_commit_request/2` and CAS/decision with CORE format parameterization, not a parallel transaction family. unknown retains the request/Draft for recorded-version recovery, never retries with a new OperationId. Committed Undo inverts only one's own single-source decision: current version and bytes exactly equal after, fresh authorization/format/Observation, complete before/after/pins and no control/other-entity effects. It creates a new inverse prepare/preview/commit; Redo similarly cannot undo another person's change.

## 7. Editor producer/consumer interfaces

CORE registers these closed shapes. This document owns behavior; SOURCE/CORE types are referenced exactly without copying their unions. Each object contains its fixed snake_case `kind` below and `version:1`. Reject unknown/missing/duplicate members, illegal null and wrong version. Counter/Token/PinRef/NodeRef/SourceObservation reuse original owners. `binding` is exactly `{ownerNodeRef,baseObservation,profile:TypstProfileCarrier/1,draftSerial,exactSourceDigest}`, cross-checked against exact request source; it is not a capability.

| Holder and complete additional members | Producer → consumer |
| --- | --- |
| `TypstEditorRead/1` (`typst_editor_read`): workspaceRef, commitDomain, ownerNodeRef, budget | UI/CLI → Core authorized source read; deliver SOURCE snapshot and complete Observation after final barrier |
| `TypstDraftProjection/1` (`typst_draft_projection`): workspaceRef, commitDomain, binding, source, budget | Editor → official SOURCE parse/project; return exact source, partition diagnostics and EditMap/eligibility, never mint persistent Locators |
| `TypstEditOperation/1` (`typst_edit_operation`): workspaceRef, commitDomain, binding, source, operation, budget | Controller → Core adapter using §4 commands; return new projection/caret, leave Draft unchanged on failure |
| `TypstEditPrepare/1` (`typst_edit_prepare`): workspaceRef, commitDomain, profile, expectedSourceObservation, proposedSource, invocationClass, saveProfile, guarantee, writeProtection, budget | Editor → original D6 `InputDescriptor/4`, `DependencyProof/4`, `PreparedIntent/4`; complete before/after pins; CORE `TypstDocumentFormatBinding/1` plus pinned `TypstDocumentQualification/1` qualify document_format |
| `TypstPreparedEditBinding/1` (`typst_prepared_edit_binding`): workspaceRef, commitDomain, profile, operationId, principalAudienceToken, inputDescriptor, sourceInputs, proposedInputs, dependencyProof, preparedIntent, preview, expiresAt | Core → preview/confirm/recovery; preview is CORE `FormatAdapterEnvelope/1(role:effect_manifest)` and bytes use role:effect_bytes with named original-D7 language-slot replacements; preparedIntent binds original `d6_commit_request/2`; all carrier/pins/cut agree exactly |

`TypstEditInput/1` is exactly `{kind:"typst_edit_input",version:1,invocationClass:"interactive_source_save"|"noninteractive",writeProtection:"strict"|"observed_only",intent:TypstPinnedEditIntent/1,origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request}}`. `TypstPinnedEditIntent/1` is closed: `{kind:"document",target:{ref:NodeRef,expectedObservation:SourceObservation/1},proposedSource:PinRef/2}`, or `{kind:"annotation",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken:AnnotationRevisionToken/1,proposedValue:PinRef/2,targetPolicy:"preserve"|"replace_current"}`, or `{kind:"annotation_reconfirm_suggestion",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken:AnnotationRevisionToken/1,proposedValue:PinRef/2}`. Annotation/reconfirm requires strict/direct; Core gates and expands complete Value before pinning proposedValue, never caller-supplied. Core creates/persists `OwnerInputBinding/2.ownerKind=intentKind="typst_edit/1"`; canonicalDescriptorBytes is CORE's complete `FormatAdapterEnvelope/1{kind:"weftext_format_adapter",version:1,role:"edit_input",profileCarrier,payload:TypstEditInput/1}`. Carriers agree; Document pins contain exact full UTF-8 source and annotation pins contain complete D3-CJ/3(TypstAnnotationValue/1). proposedInputs has exactly one corresponding entity item; sourceInputs exactly equals InputDescriptor's. Generic PreviewBinding/SourceRevisionPlan/Frontier retain their owner types. InputDescriptor/DependencyProof/PreparedIntent use CORE's generic /4 successor including compile_world, never widened /3 unions. Typst adapters only replace named payloads.

The closed `operation` union contains §4's three creation commands plus these existing commands: `replace_literal{flowKey,startScalar,endScalar,text}`, `splice_blocks{regionKeys,startScalar,endScalar,text}`, `split_paragraph{regionKey,offsetScalar}`, `merge_paragraphs{leftKey,rightKey}`, `set_inline_style{regionKey,startScalar,endScalar,style:strong|emph|strike,enabled}`, `set_title{regionKey|null,text}`, `set_heading{regionKey,level:1..9}`, `edit_math{regionKey,mathSource,display}`, `edit_list{regionKey,action:insert|remove|split|merge|indent|outdent,itemIndex,text:null|text}`, `edit_table{regionKey,action:set_cell|insert_row|remove_row|reorder_rows|insert_column|remove_column|paste_rect,arguments}` and `convert_link{regionKey,startScalar,endScalar,address,visibleText}`. `arguments` is closed by action: set_cell={row,column,text}; insert_row={at,cells:[text...]}; remove_row={at}; reorder_rows={order:[Counter...]}; insert_column={at,cells:[text...]}; remove_column={at}; paste_rect={row,column,cells:[[text...]],extend:boolean}. There is no arbitrary JSON/DOM patch arm. null is allowed only in stated slots; itemIndex/table indices address this projection. Levels/styles/layout never limit the language. Whole-source edits use prepare, not a local operation. Control/property operations remain with their owner; this union cannot edit id/types.

SourceMap/structural operations produce proposals, not save certificates. Core rechecks exact before, complete permissions and all pins. Frontier/2 keeps its causal meaning, not source-range completeness. Agents may form precise proposals or existing Core Actions without direct filesystem write authority. Flow and print share EditMap eligibility; print coordinates have a separate layout map, never a second author identity.

The author run-in control uses a **complete Source-adapter route**, not an implicit new local-operation arm: read the exact native show scope → preview complete proposedSource containing the native rule → full official reparse/evaluation and §2.1 transparent-rule proof → `TypstEditPrepare/1`. Edit only rules it can represent losslessly; retain unknown custom show with a source-required explanation. New rules cannot overwrite existing rules or change outside-scope bytes. Preview every affected heading when a rule covers multiple headings. Cancel preserves Draft; one confirmation forms one Undo. Encode/reparse native labels/quotes/brackets without requiring users to write wire objects. Failed proof preserves source rather than automatically normalizing the document.

## 8. Independent JSON annotations and quotations

Keep node-local `weftext.annotations.json`, original D3/D6 annotation owner/identity, reply/review/suggestion/attribution/permission, physical aggregate CAS and source-version semantics. New `TypstAnnotationInlineBody/1` is `{format:"typst-inline",version:1,source}`. Its pinned official parser/compiler has no files/network/packages/plugins and one fully consumed paragraph under `typst-annotation-inline/1`: text, strong/emph/strike, native math and FEATURES-qualified links. Limits are 64 KiB source, 256 KiB render, 4096 semantic nodes. Reject multiple paragraphs, headings/lists/tables, import/include/arbitrary computation/effects as `invalid_annotation_body`. Preserve exact body/diagnostics rather than fallback to another parser/plain_text.

CORE registers these closed objects without widening Value4; reject unknown/missing/duplicate members and illegal null:

```text
TypstAnnotationTarget/1 =
 {kind:"document",owner:NodeRef}
 | {kind:"document_element",locator:SOURCE.TypstElementLocator/1}
 | {kind:"document_range",locator:SOURCE.TypstRangeLocator/1}
 | {kind:"resource",resourceRef:ResourceRef}
 | {kind:"resource_region",locator:ResourceRegionLocator}
TypstAnnotationProposal/1 = {
 purpose:"comment"|"mark"|"suggestion",target:TypstAnnotationTarget/1,
 replyTo:AnnotationRef|null,body:TypstAnnotationInlineBody/1|null,
 appearance:AnnotationAppearance/1|null,labels:[text...],
 reviewState:"open"|"resolved"|"not_applicable",
 suggestion:SuggestionAuthorProposal/1|null
}
TypstAnnotationValue/1 = {
 kind:"typst_annotation_value",version:1,profile:TypstProfileCarrier/1,
 purpose,target,replyTo,body,appearance,labels,reviewState,
 suggestion:Suggestion/3|null,creator:AnnotationActorSnapshot/1,
 authoredAt:AnnotationTimeSnapshot/2,lastEditor:AnnotationActorSnapshot/1,
 editedAt:AnnotationTimeSnapshot/2
}
TypstPortableAnnotationRecord/1 = {
 kind:"typst_portable_annotation",version:1,annotationRef:AnnotationRef,
 annotationRevisionToken:AnnotationRevisionToken/1,value:TypstAnnotationValue/1
}
TypstAnnotationAggregate/1 = {
 format:"weftext.typst.annotations",version:1,ownerNodeRef:NodeRef,
 records:[TypstPortableAnnotationRecord/1...]
}
```

Value uses Proposal's purpose/target/replyTo/body/appearance/labels/reviewState types, except suggestion. Reuse D3 Suggestion/3, appearance, actor/time, same-owner acyclic replies, root/reply review constraints and operation-class gates exactly. Canonical bytes are D3-CJ/3 of complete Value/aggregate. Records are nonempty, sorted/unique by full Ref; empty sidecars are absent. Aggregate observation/install reuses D6 physical CAS types. TypstAnnotationRead/1 returns `{kind:"typst_annotation_read",version:1,workspaceRef,commitDomain,annotationRef,sourceObservation,annotationRevisionToken,value:TypstAnnotationValue/1,targetResolution:exact|mapped|candidate|ambiguous|orphaned|unavailable,body:{state:absent,exactSource:null,semanticText:""}|{state:valid,exactSource:text,semanticText:text}|{state:invalid,exactSource:text,semanticText:null,diagnostics:[...]}}`. Draft/read uses new body/profile. Annotation prepare uses CORE format envelope with TypstAnnotationProposal, exact before token/targetPolicy and unchanged generic D6 transactions; callers never supply Value attribution/evidence.

Old Value4/asciidoc-inline and recorded recovery retain their decoders without changing format on open. Keep five distinct target arms. Resource PDF/image/time regions reuse exact D9 coordinates/timebase, never wholemedia substitutes. Reading complete stored annotation values grants no target source/screenshot/media/context access.

Read/selection → annotation Draft → Core target qualification → one annotation prepare/preview/commit → reauthorized read forms the complete path. Mint range/element targets only for committed, freshly qualified exact source versions. Save an unsaved Draft first and relocate before persisting its selection; never persist draftSerial or DOM addresses. Generated/multiOrigin content may receive whole-document comments or point to real authorized source ranges, not fictitious display-as-source ranges. Flow and print produce equal source targets with different layouts. Commenting, marking, replies and resolve/reopen are independent of target exact/stale/orphaned/unavailable states. Invalidated annotations remain ungrouped without losing JSON/discussion.

Quotes/context require fresh target-source-read/Resource-read. Annotation-read grants stored expectedText digests, basis and attribution even with a hidden target, without reading that target, recomputing digests or nulling stored members. Digests/similar quotes/paths/hashes never restore old maps/tokens. Core authenticates each protected SourceTransform hop and maps only untouched ranges. Covered selections, gaps or ambiguity yield candidate/orphaned. Manual reattach explicitly selects a current target, previews changes, obtains a new token and marks pending suggestions for reconfirmation. Ordinary edits preserve target.

Insert quotation consumes an authorized source quote/locator and a writable **destination** Draft. A read-only source is sufficient; no source-write permission is required. Later revocation prevents navigation from exposing hidden context. Copied quote text is destination author content; explain this copying without treating it as current-target evidence. Apply suggestion requires current pending+confirmed, exact annotation token, target read/write, current source/range and complete dependencies/preview. One Core decision commits source mutation plus accepted state. Reject requires only annotation eligibility and cannot read hidden target. Reconfirm is separate; callers cannot manufacture actor/time/trusted/confirmation/basis. Concurrent accept/reject has one winner.

### 8.1 Closed annotation submission entries

Every new annotation edit, manual reattach, reconfirm and annotation_action create/apply/reject requires **complete + strict** operation eligibility; ordinary Document saving remains independent. AnnotationPrepare.saveProfile accepts only complete. Reject ordinary annotation requests during closed decoding and preserve Draft, without accepting missing proof as pending. After Core expands the annotation/reconfirm gate, TypstPinnedEditIntent, TypstEditInput, InputDescriptor/4, DependencyProof/4, PreparedIntent/4, protected prepared_edit backing, original commit2 and complete preview bind the same complete/strict selection, never a Document ordinary plan. annotation_action uses the original D7 action_input/prepared_action complete applicable closure for create/apply/reject without ordinary exemptions. complete covers only genuinely consumed ranges: unchanged-target comment edits and hidden-target rejection remain valid; reject never reads target or recomputes digests. Preserve static decode→original entity/record disclosure→annotation read/write/exact token→actually required target qualification→complete dependencies/before-after pins→complete preview→final authorization/CAS/P ordering. Complete does not permit earlier hidden-target reads or renamed owner errors.

`TypstAnnotationPrepare/1` is exactly `{kind:"typst_annotation_prepare",version:1,workspaceRef,commitDomain,profile:TypstProfileCarrier/1,saveProfile:"complete",guarantee:"replica_local"|"managed_atomic",writeProtection:"strict",intent,budget}`. intent is only `{kind:"annotation",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken,value:TypstAnnotationProposal/1,targetPolicy:"preserve"|"replace_current"}` or `{kind:"annotation_reconfirm_suggestion",target:{ref:AnnotationRef,expectedObservation:SourceObservation/1},expectedAnnotationRevisionToken}`. Reconfirm accepts no caller value. Manual reattach is annotation+replace_current under its separate gate and cannot combine purpose/suggestion-class transitions. Core expands before+proposal+operation class into complete Value, pins it and creates §7 TypstPinnedEditIntent/1→TypstEditInput/1→the same prepared_edit envelope/TypstPreparedEditBinding/1, never proposal-as-complete-author-value.

`TypstAnnotationAction/1` is the closed union `{kind:"create_annotation",destinationOwnerRef:NodeRef,value:TypstAnnotationProposal/1}`, `{kind:"apply_suggestion",annotation:AnnotationRef,expectedAnnotationRevisionToken:AnnotationRevisionToken/1}` and `{kind:"reject_suggestion",annotation:AnnotationRef,expectedAnnotationRevisionToken:AnnotationRevisionToken/1}`. FEATURES `TypstActionSpec/1.intent={kind:"annotation_action",action:TypstAnnotationAction/1}` exclusively wraps it. CORE action_input/prepared_action follows original D7 freezing of dependencyProof4/sourceInputs/proposedInputs/complete effects. Create uses D3's original candidate-map allocation and target qualification. Apply has target-source and accepted-Value atomic effects under one strict decision/CAS/P, never a single-entity prepared_edit. Reject has only annotation after. Apply/reject callers supply neither replacement nor targetLocator; consume the exact-token stored Suggestion/3. Complete before/after aggregates and changed annotation/source pins enter original pinDirectory/preview. Multi-entity annotation decisions do not enter single-source committed Undo.

## 9. Reading context, link hints and accessibility

Sidebars/splits open sources from Query, backlinks, search and annotations. Return restores list position, filters, selection and Draft before reauthorizing content. Outlines retain official heading semantics/actual levels, filtering/folding/current position/focused sections; focus never deletes/reorders source. Native SOURCE show/presentation supports run-in headings beside body with separate source ranges, heading role/actual aria-level, keyboard navigation and outline. Never reduce H6–H9 to H5 or turn title into a section. Author run-in controls modify native show rules only in a proven heading/show scope, preview exact full-source changes and use the same source commit. Device reading preferences only affect layout. Cancel the old portable shared presentation sidecar/bootstrap initialPresentationPolicy branch for new design; missing sidecars do not block reading. Genuine old policies/Notice4/CP5/ChangeRecord2 recover by recorded version without fictitious components. Beyond HTML h6 use role=heading with actual aria-level and the complete outline. Focus/typewriter scrolling are independent; IME, AT focus and deliberate user scrolling take precedence.

FEATURES §§3–4 exclusively owns potential-link algorithms/source encoding. Observable interaction remains required: default-off device switch with no source change; explicit selection among aliases/multiple same-range targets; preserve 白云山 `[0,3)`, 山南路 `[2,5)` and 山 `[2,3)` in 白云山南路. Use at most two separated underline levels and a complete overflow entry, stable pagination after the first 20 targets. Keyboard/touch/accessibility-list access equals hover; Escape restores selection without stealing typing focus. Real links, outgoing/incoming links and potential mentions remain distinct; hints are not graph edges. Partial combining/ZWJ scalar matches may be discovered but not expanded for conversion. Conversion freshly checks source read/write, target read, exact range/text and complete official native-link reparse, preserving visible text/outside bytes and target identity in one Undo unit. Potential targets need no write permission.

Editor segmentation retains Unicode 18.0.0, UAX #9 rev52 and UAX #29 rev49, separate from Query/CEL versions. Native authored direction takes precedence, then explicit scope device/session preferences, then first-strong auto with an explicit neutral fallback. Locale never changes source. Carets are logical point plus upstream/downstream affinity and layoutEpoch. Left/Right moves visually, modified word movement logically; Backspace/Delete removes logical graphemes with atomic CRLF. Copy is always logical. Isolate Ref/path/email/code LTR without changing bytes. RTL mirroring never reverses table identities, math/media or data direction.

Controls expose real role/name/state/selection/focus. AT follows logical reading order; tables retain header/span semantics. Virtualization never recycles active/focused/composing items; live regions do not announce each preedit scalar. Verify keyboard-only, Arabic/Hebrew/CJK mixtures, emoji/combining, 200/400% zoom, 320px, high contrast and reduced motion. Report actual Desktop/WebUI/Mobile fonts, IME and screen readers separately. A missing surface control reports capability unavailable with unchanged Core/source/advanced-route and target semantics. CLI does not claim graphical rendering.

Devices may save/select named reading/writing/review layouts or user-named layouts. Save only panel geometry, selected view types and reparsable navigation targets as device preferences, without a new business author format. Restore first preserves each open owner's Draft, Undo and selection, then restores panels and freshly authorizes/resolves/reads targets through current Core/Server. Missing, invalid-range or unreadable panels retain explicit states without cached original content. Layouts never store result sets, runtime selectors/maps/prepared evidence, permissions or replacement Drafts, and never change document order, Query sorting or shared policy. Selecting, saving another copy, overwriting or deleting a layout neither commits body content nor discards unsaved drafts; device-save failure preserves the current session. Shared/cross-device layouts and persistent multi-Query dashboards remain separate designs, never capabilities inferred from local layouts.

## 10. Performance and completion evidence

Retain D8 targets: input feedback p95≤50ms; layout feedback≤100ms; 1000 events with three cold/three warm runs; Desktop 10 MiB/100k lines, Mobile 1 MiB/10k lines, one 1 MiB line; steady editor memory Desktop≤256MiB/Mobile≤128MiB; stable refusal/degradation at 10× limits without Draft loss. CPU parse/compile/layout runs in workers. Incremental tree/error/origin must equal complete official recomputation. Full compilation need not finish for each keystroke: native preedit/local feedback can precede generation-bound official results with an explicit Draft state; unverified asynchronous results cannot be prepared for save. Virtualization never truncates source or makes one page complete.

Research proves bounded official-AST editing/display feasibility only: 32 Rust unit checks (20+12) and six whole-Text leaf edits followed by official compilation. It did not implement complete DOM selections, arbitrary-position input, IME, cross-block clipboard, row/column commands, full Undo, real Core permissions/commit or cross-device reanchoring. All product acceptance here is **UNRUN**. Implement and independently accept the flowing backend, adapters and security policies rather than claiming a complete editor from experimental HTML, AX samples, unit checks or successful print compilation.

## 11. Named disposition of previous owners

| Fixed-input actual owner/sections | New section and disposition |
| --- | --- |
| A2 D2 §§1,5–7,9; §11's §§1.1–1.4; D2 §12 projection | SOURCE replaces language/projection with official Typst; EDITOR §§1–4 replaces consumers while retaining exact-source/origin/separate-title principles; old AsciiDoc enumerations are historical |
| A2 D3 §§4.7,5.3.1,6–6.2,14.1,16.4 | CORE/SOURCE carriers/locators replace actual format slots; EDITOR §§1,6–8 retains authorization/currentness/ABA/owner and independent JSON, never Span-as-address |
| A2 D8 §§1–9,12–15; D8-INTERFACES §§1–9,11–12; D8-SCHEMAS §§1–9,11,13–14 | EDITOR §§1–10 and CORE new holders replace Typst read/draft/edit/annotation/render slots; old wire1/2/3, Value4/R6 retain recorded recovery |
| A2 D8-DIRECTION §§1–13 | EDITOR §§5,9–10 retains logical coordinates/four capabilities/actual AT/platform acceptance without copying AsciiDoc native direction semantics |
| A2 EXPERIENCE §§1–3,5–5.2,§6 table paragraph,7–8 | EDITOR §§1–10 retains context/source-rich editing/annotations/basic tables; FEATURES §§2–4 owns link algorithms/encoding; properties/Query/output consume their new owners |
| A2 D8 §§10–11,11.1–11.2; D8-INTERFACES §13; D8-SCHEMAS §12 | FEATURES explicitly continues Query/Search/View semantics; no editor Query language or persisted View state |
| AsciiDoc Annotation SCHEMAS §§6.3/6.3.1/6.4,7/7.1/8; matching A2 D3-SCHEMAS §§6.3/7/7.1/8 | CORE registers Typst holders; EDITOR §§7–8 replaces body/consumer eligibility; old inline language/holders cannot mix; SourceTransform/trust keeps its owner |

Historical snapshots remain unchanged. Previous design closure/tests do not accept the new format. Replace only listed behavior/slots, without automatically retaining old business complexity or inventing migrations.

## 12. Numbered positive/negative acceptance

Every row requires both outcomes. Design oracles are not executed product tests.

| ID | Positive | Negative/refusal and preservation |
| --- | --- | --- |
| ED-01 | Same-Draft Source↔rich↔flow↔print preserves selection/Undo; valid macros read/save | Missing controls do not reject whole files; switching never commits/discards |
| ED-02 | flow-semantic Chinese continuously reflows at320/960px with AT | PDF-only/whole-document SVG/experimental HTML cannot pass; unsupported islands explicit |
| ED-03 | Independent ordinary paragraph beside body macro writes locally | Rebound/generated/equal strings/multiOrigin/unsafe show never reverse-map; Source repair remains |
| ED-04 | Titleless, heading1–9 and same-paragraph run-in have accurate outline | Filename never authors title; deep heading never downgraded; size is not semantics |
| ED-05 | Chinese/combining/emoji/RTL input with one composition Undo | No grapheme/CRLF splitting; late transforms cannot overwrite successors |
| ED-06 | Enter/merge/delete-empty-retype/cross-style selection preserves outside bytes | Rich literals cannot inject functions/markup; no equal-text lookup on mapping failure |
| ED-07 | Native math-source edit/preview; list split/merge/indent | Glyphs/computed results are not source; failures preserve exact Draft |
| ED-08 | Edit only second equal table cell; simple rows/columns/TSV rectangle in one Undo | Never flatten span/header topology; budget/partial eligibility leaves whole transaction unchanged |
| ED-09 | Logical Copy Text/exact Source and one cross-block plain paste | HTML/RTF never executes; clipboard failure never cuts; read-only islands not deleted |
| ED-10 | Intact root data reads/repairs with body compilation error; valid source saves separately | Error-affected regions lack rich writes; syntax-invalid never falsely saved as valid |
| ED-11 | Fresh carrier/Observation/pins→complete preview→same-operation commit | Reject ABA/revocation/gaps/old maps; unknown never retries a new request |
| ED-12 | Committed Undo has new inverse plan; Draft Undo has new serial | No undo of other after/control effects; equal bytes never revive tokens |
| ED-13 | Flow/print selections qualify the same source target; JSON comments remain | Never persist Draft/DOM/Span; invalid targets retain discussion without similarity reanchor |
| ED-14 | Read-only source quotes insert into writable destination; annotation-only reads stored digests | Source write is unnecessary; hidden targets receive no extra reads/new digests |
| ED-15 | Manual reattach/reconfirm/apply follows complete gates and one decision | Ordinary edits cannot manufacture confirmation; reject cannot read target; accept/reject single winner |
| ED-16 | 白云山/山南路/山, aliases, two-level overflow and20 pagination keyboard-selectable | Longest match cannot consume other ranges; partial ZWJ match cannot expand for conversion |
| ED-17 | Explicit native-link conversion reparses same target/text/outside bytes in one Undo | Stale source/lost target read rejects; clicking hints never edits or creates graph edges |
| ED-18 | Run-in/deep heading/bidi caret/table AT/focus/zoom/high contrast | DOM order/visual x never identity; composing items never virtualized away |
| ED-19 | HTML/SVG sanitization/current resource authorization/experimental labels | Reject author JS/CSS/network/file URI; report sanitization losses |
| ED-20 | Actual per-platform performance and complete Draft on budget failure | Unit/AX/six compiles never claim IME/editor/production tests passed |
| ED-21 | Default heading/strong final realization has actual role/numbering/style; owned run-in retains heading semantics/inline placement | `show heading=>[X body]` retains actual X without invented roles; `show strong=>body` is not bold; never publish unconverged rounds |
| ED-22 | Default flow/print share official Paged target branch; experimental Html has its own qualified branch | No new semantic Typst Target; never conceal content changes by replacing Paged evaluation with Html |
| ED-23 | Device saves/restores named layouts, geometry and reparsable targets; fresh authorization preserves current Draft/Undo/selection | Never save permissions/results/runtime maps; revoked panels cannot redisplay caches, and selecting/deleting layouts never commits/discards drafts |
| ED-24 | Metadata-only→root_body site→first literal input; last-body deletion→new site→retype; full reparse and one Undo | No existing text flow required; stale site/serial, fictitious control/comment sites or failed EOL candidates leave all bytes unchanged without stealing IME |
| ED-25 | `甲乙` flow offsetScalar=1 creates inline/display math; empty-body site creates math | Existing edit_math does not substitute for insertion; reject grapheme/atom splits, delimiter escape, generated origins/old maps and preserve outside bytes |
| ED-26 | Ordinary paragraphs convert to bullet/numbered/terms; empty sites create actual native empty items and allow typing | Creation does not require an existing list; bare markers are not success; wrong term slots, rebound standards or partial failure preserve Draft |
| ED-27 | Complete+strict comment edit/reattach/reconfirm/create/apply and hidden-target reject use actual applicable closure | Reject ordinary Annotation at decoding while retaining Draft; ordinary Documents unaffected; reject has zero target reads and proof failure never downgrades |
| ED-28 | Complete direct-root catalogs and distinct compiled indices for repeated include/show; node/index/association/route share basis | Never merge equal origins or use originIds as keys; reject missing/duplicate/wrong-compilation routes; generated output grants no source writes |
