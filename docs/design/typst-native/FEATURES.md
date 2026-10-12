---
source_language: zh-CN
translation_of: FEATURES.zh-CN.md
translation_status: synced
---

[简体中文](FEATURES.zh-CN.md)

# Typst links, Query, templates and output

Status: **coordinated design candidate, not implemented or independently accepted**. Input: `af6737a057c7bb69763dfaad0a57da4652e085f3`; author branch: `design/typst-native-20261012`. This document owns addresses and conversion, potential links, Query author carriers, templates and import/export consumption for the new format. [SOURCE](SOURCE.md) owns language/data origins, [EDITOR](EDITOR.md) owns Draft/interaction, and [CORE](CORE.md) owns identity, authorization, transactions, closed adapters and historical dispatch. All acceptance cases are design oracles; product execution is UNRUN.

## 1. One author source and application responsibilities

`.typ` is author source. Native link, label/ref, metadata, dictionary/array, import, functions and set/show are used directly. Weftext assigns application meanings to these values; the official compiler does not understand NodeID, ACL, databases or Core Actions. Ordinary notes require neither JSON field blocks, double-bracket links nor a new query programming language. Lack of rich editing, compilation errors or missing layout resources do not cancel authorized exact-source reading or ordinary Source saving for advanced valid Typst. Complete business operations require their own semantic qualification.

Device preferences, candidate indexes, query results, runtime addresses and export artifacts are not another source of author facts. Desktop/WebUI/CLI/approved agents use the same Core; remote access and downloads pass through Server. UUID strings, URIs, compiled output, equal titles and digests grant no birth, permission, currentness or write eligibility.

## 2. Native links and closed address codecs

### 2.1 Five menu names, four locator domains

```typst
#set heading(numbering: "1.")
= Plan <plan>
See @plan. Alternatively, #link(<plan>)[Open the plan].
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#plan")[Project plan]
#link("../%E8%B5%84%E6%96%99/%E7%99%BD%E4%BA%91%E5%B1%B1/%E7%99%BD%E4%BA%91%E5%B1%B1.typ#route")[白云山]
#link("weftext-workspace://58c47256-bfce-4d29-a037-b83a3a644fb0/%E8%B5%84%E6%96%99/%E7%99%BD%E4%BA%91%E5%B1%B1/%E7%99%BD%E4%BA%91%E5%B1%B1.typ#route")[白云山]
#link("file:///D:/notes/report.pdf")[Report]
```

`@plan` and `#link(<plan>)` are native in-document references, not inter-note ID references. Ordinary Web/mailto/tel URLs retain native URL semantics. Stable body addresses use `weftext-node://W/N`, with both W and N exact lowercase UUIDv4. An optional fragment is the target author's label. Business schema `node_ref` uses the same native string **without a fragment**; that typed schema position produces the complete NodeRef. An ordinary string with the same spelling remains ordinary text. Historical n1.W.N, symbolicVersion1… and old adapter decoding domains do not change.

| Menu form | Basis and guarantee |
| --- | --- |
| ID link (device default for new links) | Complete W+N identity; same-Workspace move/rename retains the object, and deletion does not redirect to a namesake. Label rename/deletion can invalidate its fragment. |
| Source-relative path | The actual authored source file directory for this occurrence; source/target moves can change its meaning. An include is not assumed to belong to the root document directory. |
| Workspace long link | `weftext-workspace://W/<root-relative-path>`; relocating the same Workspace or moving the source preserves meaning. Moving the target requires an update. |
| Local absolute path | Verified file URI mapping on this device, without a cross-device/Server-client guarantee. |
| Shortest valid path | An operation generating the shortest canonical relative path within the current proven path domain; neither a title/basename/unique-suffix search nor a fifth protocol or sharing short URL. |

Native `@` references retain numbering/referenceable-element requirements. The example first enables heading numbering; unnumbered headings can use explicit `#link(<plan>)[...]`. Arbitrary `@label` is not promised to render. The managed target path `资料/白云山/白云山.typ` follows the sole `X/X.typ` layout; the file URI illustrates an ordinary external PDF.

A library means a Workspace. Cross-Workspace targets retain W and require a currently connected, authorized Workspace: no namesake guessing, automatic connection or sharing publication. Preferences affect subsequently created links, not existing source. A readable Node does not imply permission to disclose parent directories, library location or device paths.

### 2.2 URI and native-string encoding layers

A node URI is exactly `weftext-node://W/N` plus optional `#F`. Authority forbids userinfo/port; path is exactly one N, with no query, trailing slash or extra segment. A Workspace URI has exact scheme `weftext-workspace`, authority W and a leading slash followed by nonempty path components. Userinfo/port/query/empty paths are forbidden. Scheme case, UUID spelling and canonical representation must be exact; permissive URI repair is not accepted. An absent fragment differs from an empty fragment; application generators never emit, and application codecs reject, the latter.

For each path segment/fragment, exact UTF-8 bytes retain only URI unreserved `A-Z a-z 0-9 - . _ ~`; all others use uppercase hexadecimal percent encoding. Path slashes are separators. Receivers decode exactly once at this layer and require re-encoding to equal input. Malformed escapes, invalid UTF-8, NUL/control, backslash and encoded separator slash are rejected. Workspace path segments also reject empty, `.` and `..`. A fragment is an exact label string, not a path, so dot-segment restrictions do not apply to it. There is no Unicode normalization, case folding or plus-to-space conversion. `%252F` decodes to literal `%2F`, without another decode. Generators use only names admitted by actual inventory.

Source-relative strings use the same encoding for non-unreserved path bytes, retaining separator `/` and canonical leading `../`. Consumers permit only bounded leading parent segments followed by ordinary nonempty segments; internal `.`/`..`, duplicate slashes, backslashes, query and a leading slash reject application path interpretation. Bounded parent operations use the original authored source inventory path before verifying actual target bindings. They cannot escape the Workspace, enter ignored/unmanaged boundaries or bypass inventory through symlinks/aliases. `/x`, `C:\x` and unrecognized strings remain native links rather than acquiring Workspace-root meaning. Supported platform URI/path libraries generate and round-trip file URIs; drive/UNC spellings are not manually concatenated. Remote clients receive no Server disk URI.

The result is then encoded as a Typst native string argument. Quotes, backslashes and controls use official string escaping; user text is never concatenated as executable code. Existing display content AST is preserved. New plain labels may safely use `#link("...")[#text("encoded text")]`, followed by official parser/compiler verification of text and structure. URI decoding follows native parsing into a string value, without source pre-decoding or another Typst parser. An invalid codec only disables that application address/operation, preserving source bytes and repair access. [Official Link](https://typst.app/docs/reference/model/link/) distinguishes URL strings from in-document label destinations; these URIs are Weftext application protocols.

### 2.3 Anchors, outbound/inbound links and graph

`TypstLinkTarget/1` is the closed application projection `{version:1,form:"node"|"relative"|"workspace"|"file"|"native",nativeDestination:EDITOR SemanticNativeDestination/1,nodeRef:NodeRef|null,authorLabel:text|null,sourceOccurrence:TypstAuthoredLinkKey/1|null,compiledOccurrence:TypstCompiledLinkKey/1|null}`. At least one occurrence is non-null; if both are present, §2.4's exact association is required. Only strict node URIs produce nodeRef through the authorized complete identity resolver. Transient path targets remain runtime navigation evidence, never this asserted identity arm. nativeDestination retains native string/label/location/page-coordinate types without inventing authorLabel from page coordinates. Label/ref targets belong to the current compiled document. URI fragments require an **exact unique** label in authorized current target source/dependencies; duplicates, missing or unproved generated labels make the range unavailable and disable lossless conversion. A compiled-only nodeRef is derived navigation, without acquiring a stable edge merely by being non-null.

Node identity plus author label supports this round's same-target fragment conversion, but labels are author-renamable names, **not** revision-bound Locators. No permanent anchor or automatic reanchoring is claimed. Fragment-free ID navigation does not impose target-body reading; fragment proof requires permission to read its precise source/label.

Outbound links list actual native link/ref occurrences and their address forms. Stable inbound/asserted graph consumes only the complete NodeRef of strict node URIs and SOURCE-verified root-authored origin. Generated/include/multiple-origin links may display authorized compilation/navigation results as derived, but cannot become stable author edges or identity-map rewrite slots. Path links also provide “currently resolved inbound links”: an authorized current cut builds a derived index from complete source occurrences/actual path resolution, displayed as a separate inbound-panel group. Source/target rename, move, replacement, authorization or negative-range changes invalidate it. Partial scopes never prove complete absence. These transient associations are not durable identity inbound/asserted graph, while retaining inbound navigation. Potential mentions do not enter real links, graph or reference integrity. Titles/aliases are candidate selectors and never merge objects. Link health may say “no inbound links in the whole library” only with current authorization and complete relevant scope; otherwise it says “none found in the currently visible scope”, without hidden counts or automatic deletion of isolated notes.

### 2.4 Authored and compiled link occurrences

FEATURES uniquely owns the following closed types. Their actual producers are Core's official AST/source qualification and EDITOR §2.2's final-realization bridge, without referencing a nonexistent SOURCE link producer. Counter paths apply only within bound source; pins, observations and compiled indexes grant no permission.

```text
TypstAuthoredLinkKey/1={owner:NodeRef,sourceVersion:SourceVersion/2,astPath:[Counter...]}
TypstAuthoredLinkCatalog/1={kind:"typst_authored_link_catalog",version:1,
 profile:TypstProfileCarrier/1,owner:NodeRef,observation:SourceObservation/1,
 sourcePin:PinRef/2,occurrences:[TypstAuthoredLinkOccurrence/1...],complete:true}
TypstAuthoredLinkOccurrence/1={key:TypstAuthoredLinkKey/1,
 range:SOURCE TypstByteRange/1,destinationRange:SOURCE TypstByteRange/1|null,
 syntax:"native_link"|"auto_url"|"native_ref",
 destination:EDITOR SemanticNativeDestination/1|null,
 qualification:"root_literal"|"root_construct"|"nonliteral"}
TypstCompiledLinkKey/1={owner:NodeRef,sourceVersion:SourceVersion/2,
 compilationEvidencePin:PinRef/2,projectionPin:PinRef/2,index:Counter}
TypstCompiledLinkOccurrence/1={kind:"typst_compiled_link_occurrence",version:1,
 index:Counter,destination:EDITOR SemanticNativeDestination/1,
 readingNodeKeys:[Counter...],navigationKeys:[Counter...],originIds:[Counter...],
 disposition:"active"|"removed_by_show"|"unavailable"}
TypstLinkAssociation/1={compiledIndex:Counter,
 authoredKeys:[TypstAuthoredLinkKey/1...],
 state:"unique_root"|"include"|"generated"|"multi_origin"|"unavailable"}
```

AuthoredCatalog observation/sourceVersion/pin match the same source/cut. The official AST enumerates direct markup native-link calls, automatic URLs and refs in root source order. astPath is the complete official root child-index path; range covers the whole construct's exact UTF-8 bytes, and destinationRange identifies its unique literal/constructor destination; without that qualification, destinationRange=null, destination=null and qualification=nonliteral. A string computed during compilation cannot fabricate literal origin. Bare link functions require static standard bindings; matching spellings in strings/raw/code are not links. Include/function/loop output is not a new root AST occurrence. Incomplete AST/binding enumeration is unavailable, never complete=true. Occurrences sort uniquely by range.start, range.end and numeric lexicographic astPath, with each key exactly once. Managed included files may independently obtain their own catalog, outside this root catalog. A direct native_ref without a compiled location retains its author-label value with finalLocationKey=null; unproved points/locations cannot fabricate fixed coordinates. If generated application links prevent proof of complete typed rewriting, copy/fork rejects or uses a separate explicit loss proposal, without source-string scanning.

Compiled occurrences enumerate only the final converged trace of the same successful compilation. EDITOR Projection.links uses the type above, with indexes exactly 0..N-1 in final logical link-occurrence order. Repeated include/show/function output has separate indexes even when physical spans/destinations match. Wrapping/pagination/multiple Frame fragments of one logical link share its readingNodeKeys/navigationKeys; matching strings/coordinates cannot merge distinct occurrences. Both key arrays sort uniquely and refer to actual reading/native-link nodes or navigation traces in that Projection. An active SemanticElement.link.compiledIndex points back to its item, and every actual active link node is covered. removed_by_show retains source-intent diagnostics without clickable output; unavailable cannot masquerade as empty. Origin IDs resolve only in the same origin pin, never as occurrence indexes. Projection, CompilationEvidence, World, source and authorization match completely; incomplete enumeration rejects complete outbound/route claims.

Each compiled index has exactly one association, sorted by compiledIndex. Authored keys sort uniquely by complete owner/ref, canonical sourceVersion bytes and numeric astPath. unique_root has exactly one current-root key, independently proved by the official entry→return trace and complete catalog. Include requires actual include trace: keys may identify independently managed include catalogs or be empty when no managed owner exists. Generated records actual generation trace; one template-source span does not become a root writer. multi_origin requires multiple exact source keys/origin records; unavailable cannot qualify writes. Consumers independently verify indexes, ranges, destinations and origins, without guessing associations from equal text/spans.

Root writers, stable asserted edges and typed source transfer accept only proved root_literal/root_construct authored slots. Compiled navigation/export uses actual compiled occurrences. Draft conversion separately consumes EDITOR's current mapBinding and full Draft AST/selection qualification; an old committed AuthoredKey cannot authenticate new Draft bytes. Export routes use exactly TypstCompiledLinkKey/1. After freezing the complete Projection, independently enumerate every active routable application link: each has exactly one successful route or explicit same-key loss/reject; nonapplication/removed items cannot claim routes. Duplicate/missing/extra items, wrong compilation pins or unchanged rootVersion with changed World reject. §8 separately verifies actual targets/named destinations; route arrays sort by compiled index. Source/exact-byte export does not consume compiled routes.

## 3. Conversion, copying, renaming and copied instances

Right-click, long-press and keyboard provide equivalent “convert form”, “copy address/full link” and “simplify relative path” operations on the current occurrence, with disclosable unavailable reasons. Copying exact authorized source text does not require the target to exist/be readable and gives no resolution guarantee. Converted copying validates target and address disclosure again, without source write permission, Draft/receipt creation or sharing.

The conversion sequence is closed: current disclosure/source-read → SOURCE unique occurrence and EDITOR's same Base/Draft serial → authorized full target binding and exact fragment resolution → selected URI/native-string generation → official parser plus Core resolver round-trip proving unchanged target/fragment → complete proposed Source preserving display content/applicable native parameters/unselected bytes → EDITOR `TypstEditOperation/1` closed convert_link adaptation producing complete source as one Undo unit, followed by `TypstEditPrepare/1` prepare/save → final barrier rechecking source, target, path, label, schema, environment, dependencies and permissions. Insufficient path disclosure permits only disclosable forms. ID-to-path preview explains future move behavior and stable-edge changes. If body content/label/native parameters cannot survive, shortcut conversion rejects with Source access rather than flattening rich content.

Includes, generated/shared-variable/multiple-origin links and destinations without a unique writable literal have no shortcut write eligibility. A compiler Span or equal display text does not grant it. ABA, source/target replacement, revocation, rename, label/environment changes, partial sync or Draft changes invalidate preparation and require new preparation, without silently taking latest or choosing a namesake. Cancel writes nothing; one conversion is one Undo; save/commit remains the original Core transaction.

Move preserves NodeRef and makes possible path breakage visible; there is no library-wide automatic rewriting service. To retain path meaning during copy/template/import, resolve in the authorized preimage context, then encode/verify at the frozen new source location. Mapped targets use Core's **same** candidate identity map and actual new path. Typed node-URI link slots rewrite full W+N, retain fragments and validate unique labels in the new target. Workspace URI authority changes only with explicit Workspace/target mapping, never by global UUID replacement. Ordinary text, phones, CEL strings, unknown fields and UUID-looking strings are not scanned. Complete typed link/Field/definition coverage is checked together; unprovable cases use explicit loss/rejection, and full fork cannot silently drop Nodes or unknown definitions.

## 4. The sole potential-link algorithm and interaction

The actual alias producer is installed seed Field `core/aliases` in SOURCE §§7.1–7.2, consumed with exact TypstSchemaQualification and that reserved FieldId. Its native nonempty-string array retains author order/repetitions. Similarly named keys, type-alias tables or plugin string arrays are not note aliases. Candidate deduplication affects the derived index only, never source arrays.

Device setting “show potential links” defaults off and never changes source. Terms come from current authorized live Node titles and schema-recognized aliases, excluding self. Unavailable alias sources mark scanning incomplete. Deduplication uses complete NodeRef/exact term; same-title objects remain separate. Matching is exact Unicode-scalar literal matching, without fuzzy/case-fold/NFC/Pinyin. Only ordinary visible logical text with SOURCE's unique writable root-authored mapping is scanned; terms do not join across fragments. Code, math, URL, real links, metadata/fields, annotations and generated/include/multiple-origin content are excluded. Soft wrapping permits multiple visual segments but source joins/EOL never join terms.

Matches carry `[start,end)` scalar ranges bound to the same Draft serial/flow. Before conversion, a fresh Draft map converts them to exact UTF-8 byte/EOL source ranges, with EDITOR grapheme boundaries at both ends and no split CRLF/combining sequence/emoji ZWJ. Partial-grapheme matches are nonconvertible discoveries, without selection expansion. Equal ranges group multiple targets; distinct nested/crossing ranges remain: “白云山” `[0,3)` with “山” `[2,3)`, and “白云山南路” `[0,3)` with `[2,5)`, plus “山” `[2,3)` as a third group that remains selectable in overflow. Alias “白云山2026” at the same range retains its complete target list. These numbers are scalar examples, not byte offsets.

At most two vertically separated hint lines are drawn. Sort by ascending start, then descending end for equal starts, assigning the smallest available layer; a preceding end equal to the next start permits sharing. Groups lacking a layer enter a unified dense hint and “more matches” entry, preserving every semantic match without unlimited lines or color-only meaning. Wrapped lines use actual fragments; RTL uses logical ranges and actual layout mapping rather than screen x for semantic order. A popup initially shows at most 20 targets, then pages in canonical complete NodeRef order. Titles, reasons and authorized context are shown; multiple targets require an explicit choice, never a first-item default.

Hover, click, keyboard commands/accessible lists and touch long-press all open candidates. Escape restores selection; typing/IME focus is retained. Only explicit “convert to link” edits the same Draft under §3, proving the new native link references the selected complete NodeRef, with text/unselected bytes unchanged. A readable target does not make a read-only source writable. Suggestions/scans/old evidence expire upon revocation or name/schema/Base changes. Scanning is cancellable, visible-region-first and budgeted, with truncation/incomplete states rather than whole-library claims. An individually fully verified candidate remains convertible during incomplete scanning. Candidate indexes are rebuildable, not synchronized facts.

## 5. Native author carriers for cross-library Query

### 5.1 Carrier, reading, saving and references

Typst `query` is introspection **inside a compiled document**. It cannot query cross-library ACL, issue ResultHandles or run Actions. [Official Query](https://typst.app/docs/reference/introspection/query/) and [Metadata](https://typst.app/docs/reference/introspection/metadata/) supply language/display capabilities. Weftext Query retains D7's sole Core DAG, CEL, types, scope, authorization, dependencies, paging, subscriptions, budgets and error ordering.

New saved views use direct literal root-authored metadata in `.typ` with label `weftext-view-<key>`, where key matches `[a-z][a-z0-9-]{0,63}`. This document reserves the prefix. Include/generated/shadowed declarations have no application-definition qualification. Each label is unique in exact source; duplicate/missing/unknown versions preserve valid source but make the definition unavailable, without taking the first item. Carrier `TypstFeatureDefinition/1` has exactly four members `{format:"weftext.feature-definition",version:1,query:<TypstQueryValue/1>,views:(<TypstViewValue/1>,...)}`. Views are nonempty, at most 16, in author array order. UI generates and previews complete Source; ordinary authors use filter/column/layout editors without writing DAG/JSON/DSL.

`TypstQueryValue/1` is the **member-for-member native representation** of current QuerySpec/3. All eight members are required; format/version, lifecycleScope and searchBindings remain closed. Dictionary maps to object, array to array, bool to bool, string to text. Counters require exact bounded Typst integers. D7 arbitrary integer/decimal canonical text stays string, never float. `(:)` is an empty dictionary, `()` an empty array, and singleton arrays need a comma. Typst none is forbidden where D7 forbids null; Optional.none retains D7's explicit typed Optional arm. No expression evaluation, spread, loops, imported values or JSON-string fallback exists. Schema-declared NodeRef positions use §2's fragment-free strings; Resource/Annotation refs retain their typed shapes mapped to native dictionaries. Ordinary strings are never scanned for refs. Unknown members, duplicate keys and inapplicable types reject application projection while retaining source.

The old QuerySpec3/ViewSpec1/TypedLiteral/V null prohibition belongs only to its original decoder, not the new profile. The new feature carrier's sole schema-null slot is `maxItems` in **every** `TypstQueryType/1` with `kind="typst_array"`: null maps exactly to native `none`, including Parameter.type/default.type, View.inputSchema, Domain TypedLiteral.type and the same recursively nested array/dictionary/union/nullable slot. It preserves the absence of a schema cap rather than inventing a finite N. Other Query type/value slots still reject raw null/none; an empty Optional uses `(state: "none")`, and native_nullable authored null uses an explicit state arm. Missing optional members stay absent without none placeholders. Other listed nullable slots in closed application objects—link keys/ranges, Recipe.parentIndex and SOURCE ValueType.maxItems—map only under their own decoders, without opening Query. The following complete compilable native carrier supplies all eight Query members and valid View column bindings. Actual execution requires an authorized Core cut; metadata compilation does not execute Query.

```typst
#metadata((id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654", types: ())) <weftext>
#metadata((
  format: "weftext.feature-definition", version: 1,
  query: (
    format: "weftext.query", version: 3, parameters: (),
    relations: (
      (id: "n", op: "scan", domain: "nodes", selector: (kind: "workspace"), "as": "node"),
      (id: "t", op: "read", input: "n", from: "row.node",
        bindings: ((name: "title", source: (kind: "title")),)),
      (id: "p", op: "project", input: "t", fields: (
        (name: "node", expr: "row.node"),
        (name: "title", expr: "row.title"),
      )),
    ),
    scalars: (), result: (kind: "rows", relation: "p"),
    lifecycleScope: "live",
    searchBindings: (version: 1, mode: "none", selections: ()),
  ),
  views: ((
    format: "weftext.view", version: 1,
    inputSchema: (kind: "rows", ordered: false, columns: (
      (columnId: "node", type: (kind: "node_ref")),
      (columnId: "title", type: (kind: "optional", item: (kind: "text"))),
    )),
    layout: "list", bindings: (primary: "title", target: "node"),
    options: (title: "Notes", description: "Currently authorized notes", labels: (:),
      textDirection: "auto", numberFormat: "canonical"),
  ),),
)) <weftext-view-notes>

#title[Query example]
Weftext displays results at this definition entry; Typst query does not execute library queries.
```

After definition source-read, SOURCE's official AST literal projection produces this mapping for D7's native-definition adapter, independent of whole-document layout success. Saving writes author values to the same occurrence through EDITOR's whole-source prepare/save. Current results, runtime evidence, shortcut inputs and device state are not saved. Valid advanced Query conditions beyond basic UI are retained with Source access rather than overwritten to fit controls.

`TypstDefinitionAddress/1` is exactly `{version:1,owner:NodeRef,revisionToken:Token,label:text}`, with the unique label above. revisionToken is an exact 43-character canonical base64url string with protected tag `d6_source_revision/2`; complete RevisionTokenBinding/2 stays in Core protected records rather than author values. This is a production-version address, not current authority, a new content identity or durable ViewRef. Cold read/QueryRef performs original owner disclosure, authenticates production SourceVersion/2 through the real RevisionTokenSealArtifact/1 and complete protected binding, then obtains a fresh complete SourceObservation with equal full sourceVersion, verifies unique literal definition/profile through SOURCE, then rechecks the final barrier. Equal bytes/latest same-label content cannot revive old addresses. Native QueryRef typed address slots consume this address, so `TypstQueryValue/1` has a separate profile decoder and cannot enter old QuerySpec3 JSON/DefinitionAddress. Old QueryRef keeps its decoder. Every callee owns its scope/searchBindings; callers cannot override them.

Native dictionary keys follow official keyword/identifier rules. Keywords such as `as` and keys containing spaces/nonidentifier characters require string keys, for example `"as": "node"`. Closed member comparison follows official AST decoding rather than treating keywords as identifiers.

Feature declarations have separate binding qualification rather than mechanically applying SOURCE's root-declaration prefix allowlist. The default generator inserts consecutive feature metadata after the authenticated `<weftext>` root declaration and before ordinary body/code. Its prefix permits only the complete static prefix proven by SOURCE §2 (including qualified title-before-root), trivia, authenticated root metadata and preceding qualified feature metadata. Official AST verifies every direct call is the same unshadowed standard metadata, with one literal dictionary and a direct unique label. Earlier root metadata is an admitted inert application declaration, so the example's second metadata qualifies. Elsewhere, official scope/binding evidence must statically prove the same builtin without preceding let/import/conditional computation effects; preceding code is never executed to guess the final binding. Missing proof preserves source with definition unavailable. Saving does not rearrange advanced valid documents to “repair” them into the default layout.

Persistent TypstDefinitionAddress/TemplateSourceAddress is issued only for sealed managed production tokens. Ordinary current external-source reads may create profile inline Query execution from complete SourceObservation/exact source rather than pretend to hold a managed-signed saved address. Explicit Source admission/save creates a managed version before issuing a new persistent address. External modifications make old persistent addresses stale; equal bytes/labels never forcibly make them current. Ordinary opening/source saving remains independent of this saved-reference gate.

### 5.2 Retained business execution and origins

This native Query decoder's sole Field read arm is `{kind:"field",fieldId,shape:"native_value"}`, returning the entire schema value T without old Entry `values/entries` list/qualifier/note/provenance wrappers. Contact phones is **one** array retaining repetitions and author order, never a double list. The new typed profile below gives all SOURCE everyday values positive read/project/equality/sort/exact-export paths. Old TypeSpec/V remains closed and float is never approximated as decimal.

The separate `TypstQueryType/1` decoder recursively reuses original closed D7 TypeSpec members, replacing recursive type slots with this domain, and adds only the named arms below. `TypstQueryV/1` is selected by exact schema, without any/free unknown tags. `TypstTerminalSchema/1` retains all rows/scalar/graph members, column order and key contracts, replacing only type slots. Old JSON Query retains its TypeSpec/V/TerminalSchema. Native inline/QueryRef/View/Action/export uses this profile through its complete closure, without mixing pages or guessing types across profiles.

| SOURCE valueType | Exact Type arm and V | Comparison/order and limits |
| --- | --- | --- |
| string/bool/int/node_ref | Original text/bool/int64/node_ref; V is original text/bool/canonical i64 string/complete NodeRef | Original exact rules; integers never pass through float, and refs have equality rather than name-based order. |
| decimal/decimal_text | Original decimal; V is original canonical exact decimal text computed exactly from SOURCE coefficient/scale | Original arbitrary-precision decimal comparison/order; raw scale/negative-zero origin stays in the Field pin, without changing source bytes. |
| float | `{kind:"typst_float64"}`; V is SOURCE's 16 lowercase hexadecimal IEEE754 binary64 bits | Finite only. Equality/ascending/descending order follow exact binary64 numeric values. +0/-0 compare equal while V/raw bits remain; distinct chooses the original minimum canonical raw representative. No NaN/Inf or implicit int/decimal promotion. |
| duration/exact_seconds_text | `{kind:"typst_duration"}` V is canonical signed i64 seconds; `{kind:"typst_exact_seconds"}` V is canonical exact decimal seconds | Exact elapsed-second comparison/order, without floating-point `.seconds()`, civil months/years/instants or implicit precision conversion. |
| date | `{kind:"typst_date"}`; V is exactly `{year,month,day}`, SOURCE's official valid Gregorian date, year=1..9999 | Gregorian civil-tuple comparison/order, without inferred time/zone. Calendar/Timestamp rules follow below. |
| time | `{kind:"typst_time"}`; V is exactly `{hour,minute,second}` in SOURCE's component domains | Civil-clock tuple comparison/order, never UTC instant/date or implicit midnight day rollover. |
| local_datetime | `{kind:"typst_local_datetime"}`; V is exactly `{year,month,day,hour,minute,second}` in SOURCE's valid date/time domains | Civil-tuple comparison/order without host timezone; no cross-type comparison with instants. |
| instant | `{kind:"typst_instant"}`; V is SOURCE's exact RFC3339 string | Original D4 parsing compares Gregorian UTC ordinal day/second/arbitrary-precision fractions. Raw offset/string remains, without float/date serial. Missing offsets/-00:00/leap seconds reject. |
| array | `{kind:"typst_array",item:TypstQueryType/1,minItems:Counter,maxItems:Counter|null}`; V is an ordered recursive V array | Retains repetitions/order-sensitive equality, without implicit whole-array order; explicitly unnest/select scalar keys. Null or >65535 schema maxima do not enter old list.maximum. The new arm and checked actual budgets apply; excess fails the whole operation. |
| dictionary | `{kind:"typst_dictionary",members:[{name:text,type:TypstQueryType/1,required:bool,nullable:bool}...],additional:"preserve"}`; V is exactly `{members:<declared-member value dictionary>,preserved:[{key:text,value:SOURCE TypstNativeValue/1}...]}` | Schema members use canonical key order rather than the old 64-member limit. Preserved unknown values retain exact native values/author order, without writable-member/Field-identity inference. Equality includes members/full preserved values; no implicit whole-object order. Full structure counts against budget; members are not skipped. |
| union | `{kind:"typst_union",variants:[{name:text,type:TypstQueryType/1}...]}`; V is `{variant:text,value:<branch V>}` | Names are `variant0`, `variant1`… from frozen SOURCE schema ordinal; values use the same ordinal, never display-name/first-match guessing. More than eight branches are admitted under full budgets. Equality uses variant/value; ordering explicitly projects an orderable branch, without numeric coercion across branches. |

Requiredness is OR over SOURCE effective type closure. Required nonnullable fields are T, with invalid missing values. Optional nonnullable fields use original Optional<T>, with missing encoded as `{state:"none"}`. Nullable fields use independent `{kind:"typst_nullable",item:TypstQueryType/1,optional:bool}`; V is exactly `{state:"absent"}` (optional only), `{state:"none"}` (authored none), or `{state:"value",value:<T V>}`. No nested Optional or conflation of missing/authored none occurs. Dictionary members apply these rules recursively. Native sort keys for this nullable type alone add required `nativeNullOrder:"absent_none_value"|"value_none_absent"`; nonnullable keys forbid it. Value states follow T's order; missing explicit null ordering rejects sorting. These Types/V belong to Query internals/transport, never `.typ` author values.

CEL remains the sole expression profile; original text/bool/int/decimal operations remain exact. New types support same-type `== != < <= > >=` only where the table permits order, without implicit float/int/decimal or time/date/local-datetime/instant coercion. Float arithmetic is strictly IEEE754 binary64 round-to-nearest-ties-even, rounding each primitive independently without FMA; divide-by-zero/nonfinite/overflow use original errors. Duration addition/subtraction checks i64; exact_seconds uses original exact decimal arithmetic. Date/time/local_datetime/instant provide exact comparison and explicitly bound original temporal-provider functions; undefined operations give type_mismatch/unsupported_feature rather than blocking reading/projection/sort/exact export. Existing explicit CEL member/list rules operate on typed compound values; unknown preserved values require explicit read-only native access, without dynamic Field/Action. New counters/containers/value bytes count against complete original budgets.

`TypstQueryResult/1` is exactly `{kind:"typst_query_result",version:1,resultToken:Token,cursorToken:Token,schema:TypstTerminalSchema/1}`. Core's generic ResultHandle/cursor/epoch/auth/cut/TTL remains authoritative. Native page row/scalar/graph members are exactly original D7 transport members, with cells/value decoded by that handle's TypstQueryV decoder and exact schema. Old d7_query_result/wire2 schema/V accepts no new arm. Producers/consumers/native result export explicitly dispatch, never refresh old pages into native rights.

`TypstViewValue/1` uses original ViewSpec1 members/layouts in this profile with TypstTerminalSchema type slots. It explicitly replaces only task_list status binding with **text** and full domain `open|done|cancelled`, requiring current Result's actual SOURCE Task-status Field/declared task-type proof. Similarly named strings authenticate no Tasks and renderers fetch no targets. Title still requires nonoptional text; Query explicitly derives displayTitle through `row.title.orValue('')`, without writing placeholders into title or using filenames. Calendar/Timeline admit same-type typst_date or typst_instant endpoints. Dates use SOURCE's fixed Gregorian/day domain and explicit View calendarVersion; instants retain exact UTC/explicit timeZone/tzdbVersion. Optional Task.due dates/instants may explicitly filter absence/project calendar endpoints. A due point is not a positive-duration event; an explicit Query may project date due and nextCivilDay(due) into a half-open one-day task range. This named CEL function accepts only typst_date and checks the next Gregorian civil day; beyond 9999-12-31 it gives original numeric_overflow. Instant due requires an explicitly authored positive-duration endpoint/timezone/provider rather than an invented Event/reminder. Without a positive interval, list/date-field display remains. Other View bindings/completeness/accessibility/pure-presentation limits remain unchanged.

D7 QuerySpec/3's eight members, DAG/canonical/error/bag/order, union_all, unnest/filter/project/sort/take/aggregate/distinct/query_ref/traverse/related/scalar/graph, CEL exact values/Optional, positive/negative ranges and complete-result rules remain. Generic join/window/quantile remain deferred. Field reads use SOURCE's native FieldSource/Registry: schemas specify strings/object arrays rather than per-value Entry UUID/provenance. Repeated contacts use current occurrences; array indexes are not permanent identities. Unknown/unavailable computed fields are unavailable, never absent/empty. Field-only Query reads neither body nor ref targets implicitly. Title is Optional; filenames/paths never fill it.

Body/headings/table read SOURCE's official semantic products and complete environments/dependencies. Valid generated headings from functions/loops/repeated includes remain read-only Query rows with result-epoch logical occurrence keys. Missing unique physical source cannot invent a Locator or write capability. Legal native deep/run-in heading semantics remain. Distinct/global aggregate K belongs to its operator, not a representative parent's Action lineage. Compilation/semantic-product unavailability stays explicit rather than using reparsing/string flattening as fake body projection.

Cross-library execution uses Core Query calls for explicitly connected authorized Workspaces; combination requires existing exact-schema/union rules and complete dependencies. URIs grant neither cross-library reads/writes nor cross-authority atomic transactions. Indexes accelerate candidates only. Building/partial/stale/budget shortage differs from complete zero results, and hidden count/rank/snippet must not leak. Original plain/visual/explicit-shortcut search presets and grammar in D7-SEARCH §§1–10 remain, without forcing shortcut mode.

Trash/live_and_trash retains D7-SEARCH §11's exact read-only scopes and same-query_scan Coverage/1. Tombstones/placeholders/missing proofs are not results. Scope neither resumes edges nor grants restore/purge/Action; writes reject a nonlive whole-call closure even if filtering leaves only live rows. Saved searchBindings preserve contributionId/version/full descriptor/namespaceOwner. Cold reopening checks current D10 Catalog exactly. A stopped/version-changed contribution with an intact Field remains definition_unavailable; new contributions do not enlarge all automatically, and none adds no ambient package gate to ordinary Field Query. Authorization changes reset results/cursors/subscriptions; old pages/rowHandles cannot renew rights.

### 5.3 Database views, tasks and capture

Task project is actually produced by SOURCE seed `tasks/project`: an optional nonempty fragment-free typed node_ref on explicit Task members. Reads/relations/inbound/typed copy still require the same cut and independent authorization. Targets are ordinary Nodes, without an extra ProjectFacet. Company text and other NodeRef Fields do not impersonate project fields. Missing exact installed seed/schema qualification is unavailable rather than a namesake assumption.

The new closed internal heading-scan K is `{invocation:I,operator:o,kind:"typst_product_occurrence",owner:NodeRef,sourceVersion:SourceVersion/2,entry:Counter}`. Entry is the physical index of the heading in the complete officially realized semantic introspection sequence, not a filtered ordinal. Each (I,o,root owner) binds exactly one complete TypstSourceSnapshot/1/semanticProjection pin and environment/world/all positive/negative dependencies in the protected Result. Consumers independently validate that basis before requiring the entry to be a heading. Repeated includes have distinct entries even with equal physical locators; different QueryRef invocation I never shares K. Original D3-CJ canonical-byte/unsigned UTF-8 comparison applies; full keys/scans/bases count against budgets. Missing basis/incomplete projection fails the whole operation without skipping headings. Equal sourceVersion with changed includes/environment still resets/replaces rather than reusing handles through equal entries. K is not a permanent address. Original group/distinct creates new eqKey-based K without parents; global aggregation has keys=[], inheriting no representative input write authority.

ViewSpec/1 only presents one complete Query terminal, without fetch/filter/sort/aggregate, inferred title/avatar/hidden endpoints or scripts. Its table/list/task_list/board/calendar/timeline/gallery/metric/bar/line/scatter/pie/heatmap/network closed set, explicit order, typed columns, complete accessible table, Optional/zero/empty/error distinction and CJK/RTL remain. Missing renderers are unavailable. The first 200 rows are not complete membership/semantic charts; sampling cannot claim completeness. Deferred tree/treemap/sunburst/Gantt/statistical bindings do not enter through the format change.

A task is an ordinary Node with task type. Status/due/project are SOURCE schema fields; details are the note body. Tables/boards/calendars/smart lists consume the same Query. Checking, lane dragging and due editing show exact Field changes and require a unique current writable origin through Action preparation. Aggregate/computed/hidden/stale rows cannot write. Period navigation/rule preview retains calendar/timezone/range/provider obligations. Reminder delivery, snooze, completion-triggered repetition, My Day and natural-language dates remain deferred.

Quick capture shares parent/type/template/title (optional)/fields/body. “Capture another” resets only after save succeeds; unknown preserves the original request/Draft without another create. Actual receipt resultAllocations identify the real Node. requireMembership is checked before commit through D5. Creation may succeed when membership is not required. Only a complete current Query can prove “outside the current filter”; partial/incomplete/stale results show membership unknown with access to the real object, never duplicate creation or Query modification.

### 5.4 Complete dispatch of new parameters, constants and calls

This profile selects all following objects without adding unknown types to the original D7 TypedLiteral/ParameterSpec/QueryCall decoders. Members are mandatory unless explicitly marked optional. Preserve original parameter-name syntax/uniqueness, required/default error ordering, invoker authorization, complete callee closure/call budgets and CEL scope restrictions.

```text
TypstTypedLiteral/1={type:TypstQueryType/1,value:TypstQueryV/1}
TypstParameterSpec/1={name:text,type:TypstQueryType/1,required:boolean,
 default?:TypstTypedLiteral/1}
TypstQueryCall/1={query:{kind:"inline",spec:TypstQueryValue/1}
 |{kind:"saved",definition:TypstDefinitionAddress/1},
 arguments:{parameterName:TypstTypedLiteral/1...},context:TypstQueryContext/1}
TypstQueryValueSource/1={kind:"literal",literal:TypstTypedLiteral/1}
 |{kind:"parameter",name:text}
TypstLexicalBinding/1={kind:"literal",literal:TypstTypedLiteral/1}
 |{kind:"lexical",name:"this.node"|"this.heading_title"}
TypstContextBinding/1={kind:"literal",literal:TypstTypedLiteral/1}
 |{kind:"runtime",name:"now"|"timeZone"|"calendarVersion"|"tzdbVersion"}
TypstDynamicBlock/1={format:"weftext.typst-dynamic-block",version:1,
 query:{kind:"inline",spec:TypstQueryValue/1}
 |{kind:"saved",definition:TypstDefinitionAddress/1},
 view:{kind:"inline",spec:TypstViewValue/1}
 |{kind:"saved",definition:TypstDefinitionAddress/1,viewIndex:Counter},
 arguments:{parameterName:TypstLexicalBinding/1...},
 contextBindings:{contextName:TypstContextBinding/1...}}
TypstDefinitionPayload/1=TypstFeatureDefinition/1|TypstDynamicBlock/1
```

These name→value objects accept only statically declared parameter/context names, not open maps or arbitrary Field access. Parameter.required=true forbids a default; false requires a complete same-type default. Only an unprovided optional parameter uses its default; explicit none/absent values do not trigger it. Unknown/duplicate parameters, missing required parameters and type mismatches reject the entire call under original errors. `TypstQueryContext/1` retains a statically closure-derived member set containing exactly required now/timeZone/calendarVersion/tzdbVersion: now uses the new TypedLiteral decoder but must have precisely original zoned_instant type; other members are nonempty text. No required context means `{}`. Never implicitly convert typst_instant to zoned_instant or supply host timezone/now. Lexical this.node produces a complete NodeRef TypedLiteral; this.heading_title produces text. Acquire each under actual owner/current-heading qualification before matching callee type. SavedQuery itself has no this. Runtime bindings use only explicitly supplied context, with now frozen once per invocation.

| Actual typed slot | Sole new producer→consumer |
| --- | --- |
| Query.parameters[].type/default; inline/saved/callee decoding | New ParameterSpec/TypedLiteral; parameter definitions sort canonically by name, with complete default.type=parameter.type equality. |
| QueryCall.arguments, PAB/action.canonicalCallInputs, CollectionCall.call | TypstQueryCall/new TypedLiteral decode every member; closure-wide parameters/context use one cut, never sending new arguments to old decoders. |
| Relation/scalar query_ref.arguments, selector literals, parameter defaults | ValueSource literal uses new TypedLiteral; parameter supplies only the callee's same-type value. Row/scalar/this-dependent values remain forbidden, preserving call isolation. |
| recurrence.horizon, View fixed Domain.minimum/maximum and other typed constants | Every actual original schema slot typed as TypedLiteral selects the new decoder. Original horizon/domain business-type requirements still apply; unknown types cannot bypass them. |
| DynamicBlock.arguments/contextBindings, saved View calls | The closed Lexical/ContextBinding above; viewIndex selects the exact authenticated Feature.views index, never an assumed first View. |
| DefinitionTransfer static typed roots, Result schema/V, native export, ResultPin | Recursively select this Type/V, including Refs in new defaults/arguments/lexical/context literals. Ordinary CEL/text is not scanned for identity. |

TypstDynamicBlock uses the same qualified native `#metadata(...) <weftext-view-key>` carrier, with format strictly dispatching the two arms above. Feature.query may supply a saved QueryRef; DynamicBlock itself cannot masquerade as a callee Query. TypstDefinitionInput.value selects TypstDefinitionPayload, and typed transfer enumerates both arms completely. Default UI remains an ordinary Query/View builder, without requiring handwritten internal typed objects.

Typed values in authored carriers use official native values under the exact declared type: direct float64 Float AST maps to exact finite bits; direct int64 Int or SOURCE-qualified standard int(Str) constructors map to canonical i64 text, including i64::MIN. SOURCE-qualified standard decimal/duration/date/time/local_datetime constructors map to this V under their actual builtin binding. Arrays recurse by item; dictionaries/unions/nullable recurse through declared arms. Unbounded integer/decimal_text may remain canonical strings; node_ref remains a fragment-free URI. This source mapping is separate from runtime TypedLiteral wire, which strictly accepts only §5.2 V encodings. Undeclared positions are not typed/evaluated, and arbitrary expressions are not executed. A metadata “literal declaration” admits only SOURCE-defined constant constructors in these typed leaves, without enabling computation in the Query graph/ID/type/schema.

The new CEL profile is `weftext.typst.cel/1`, not a permissive original weftext.cel/1 mode. Ordinary integer literals remain int64 and text/bool remain unchanged. Only this profile admits finite double literals in official CEL AST, producing typst_float64 with correctly parsed IEEE754 ties-even bits and negative-zero -0.0. Expected float does not turn an integer literal into float; expected decimal does not convert a double. Named `float64Bits("16hex")` and `durationSeconds("canonical i64")` accept only valid literal payloads and produce native float/duration. Invalid bits/NaN/Inf/overflow reject statically. Expected Type derives only from exact parameter/literal/declaration signatures and parent-constructor rules, never first-row values or column labels; nonliteral expressions require complete type equality. Original optional/list/object/conditional bidirectional rules recursively select this type. A native_array literal validates against complete expected min/max; maxItems=null never becomes its element count. Without expected type, a nonempty array has min=max=syntactic element count; an empty array requires expected type. Native dictionaries/unions/nullable validate closed values/states/variants only with complete expected type, without branch guessing, nullable flattening or discarded preserved values. Canonical CEL AST retains exact raw bits/types of new literals. Call/cache semantic identity preserves raw typed constants, without replacing actual input with eqKey below.

**QP-01 complete author fixture.** These two `.typ` files are review fixtures for advanced saved Queries; ordinary note/property authors still use UI rather than handwritten wire. First commit the callee and have Core issue its actual production token, then let the caller builder insert that address into the second file. The 43 A characters are lexical examples only: an authorized positive fixture substitutes the callee's genuinely authenticated token, never treating the example as a signature. Source schema has an authorized installed float Field `user/amount`, whose statically selected Field-read type is Optional<float64>. The callee reads/filters nonempty values/compares its parameter, explicitly sorts/projects and preserves an array parameter with no schema cap. Every maxItems:none below occupies a legitimate schema-null slot; actual native float/array defaults map into wire bits through the typed source mapper.

```typst
#metadata((id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654", types: ())) <weftext>
#metadata((format: "weftext.feature-definition", version: 1,
  query: (format: "weftext.query", version: 3,
    parameters: (
      (name: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none), required: false,
        default: (type: (kind: "typst_array", item: (kind: "typst_float64"),
          minItems: 0, maxItems: none), value: (0.5, -0.0))),
      (name: "threshold", type: (kind: "typst_float64"), required: false,
        default: (type: (kind: "typst_float64"), value: 0.5)),
    ),
    relations: (
      (id: "n", op: "scan", domain: "nodes", selector: (kind: "workspace"), "as": "node"),
      (id: "a", op: "read", input: "n", from: "row.node", bindings: (
        (name: "amount", source: (kind: "field", fieldId: "user/amount", shape: "native_value")),)),
      (id: "f", op: "filter", input: "a",
        predicate: "row.amount.hasValue() && row.amount.value() >= param.threshold"),
      (id: "s", op: "sort", input: "f", keys: (
        (expr: "row.amount.value()", direction: "asc", "none": "first"),)),
      (id: "p", op: "project", input: "s", fields: (
        (name: "amount", expr: "row.amount.value()"),
        (name: "samples", expr: "param.samples"),)),
    ), scalars: (), result: (kind: "rows", relation: "p"), lifecycleScope: "live",
    searchBindings: (version: 1, mode: "none", selections: ())),
  views: ((format: "weftext.view", version: 1,
    inputSchema: (kind: "rows", ordered: true, columns: (
      (columnId: "amount", type: (kind: "typst_float64")),
      (columnId: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none)),)),
    layout: "table", bindings: (columns: ("amount", "samples")),
    options: (title: "阈值查询", description: "准确浮点与数组", labels: (:),
      textDirection: "auto", numberFormat: "canonical")),),
)) <weftext-view-threshold>
```

```typst
#metadata((id: "b7a36d54-52ec-4e63-9f0d-6579e2baee7f", types: ())) <weftext>
#metadata((format: "weftext.feature-definition", version: 1,
  query: (format: "weftext.query", version: 3,
    parameters: (
      (name: "limit", type: (kind: "typst_float64"), required: false,
        default: (type: (kind: "typst_float64"), value: 0.5)),
      (name: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none), required: false,
        default: (type: (kind: "typst_array", item: (kind: "typst_float64"),
          minItems: 0, maxItems: none), value: (0.5, -0.0))),),
    relations: (
      (id: "r", op: "query_ref", definition: (version: 1,
        owner: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
        revisionToken: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        label: "weftext-view-threshold"), arguments: (
        threshold: (kind: "parameter", name: "limit"),
        samples: (kind: "parameter", name: "samples"))),
      (id: "s", op: "sort", input: "r", keys: (
        (expr: "row.amount", direction: "asc", "none": "first"),)),
    ), scalars: (), result: (kind: "rows", relation: "s"), lifecycleScope: "live",
    searchBindings: (version: 1, mode: "none", selections: ())),
  views: ((format: "weftext.view", version: 1,
    inputSchema: (kind: "rows", ordered: true, columns: (
      (columnId: "amount", type: (kind: "typst_float64")),
      (columnId: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none)),)),
    layout: "table", bindings: (columns: ("amount", "samples")),
    options: (title: "传递参数", description: "同型callee调用", labels: (:),
      textDirection: "auto", numberFormat: "canonical")),),
)) <weftext-view-caller>
```

The call fixture uses TypstQueryCall/1 with arguments={} to apply the actual limit/samples defaults above. An explicit threshold float argument uses valid runtime bits V rather than an authored Float number. Effective TypedLiteral.type passed to the callee exactly matches its parameter Type. QueryRef clears ordered and the caller sort restores it; native result export retains complete schema/bits for float columns and array V. Changing caller defaults does not rewrite the callee. Different types/NaN/raw-null/old TypedLiteral mixing/unsealed tokens reject the entire call.

### 5.5 The sole native EqualityCanonical and equivalence-class consumers

`TypstEqualityCanonical/1` has exact root `{format:"weftext.typst.equality",version:1,type:TypstQueryType/1,value:TypstEqualityTree/1}`. Type is the strictly validated complete canonical type, including scopes, bounds, nullable/member requirements and union order, not only kind. Original object members keep original canonical order; new dictionary members sort by unsigned UTF-8 name; unions retain branch order. Old/new root domains never share keys. The value trees below are closed; array order matters and numbers use canonical text rather than host floating point.

| T | Exact EqualityTree shape |
| --- | --- |
| Original D7 noncompound types | Bool=`{kind:"bool",value:boolean}`; text=`{kind:"text",value:text}`, with fixed Unicode15.1 NFC only for nfc types; int64/integer/decimal=`{kind:"number",value:text}` with original canonical mathematical values; code=`{kind:"code",value:text}` with full scope in outer type; Ref=`{kind:"ref",value:EntityRef}` with the complete D3 canonical tuple; calendar_date=`{kind:"calendar_date",calendarId:text,calendarVersion:text,precision:text,value:text}` with the original D4 comparator's exact canonical date lexeme; zoned_instant uses the instant tree below; quantity=`{kind:"quantity",dimensionId:text,unitId:text,magnitude:text}` with same-cut verified complete basis/canonical exact magnitude. Outer type strictly selects each arm; free leaves/raw values cannot masquerade as canonical values. |
| typst_float64 | `{kind:"float64",bits:text}`; preserve valid finite nonzero bits, while both zero patterns encode exactly `0000000000000000`. V remains unchanged. |
| typst_duration/typst_exact_seconds | `{kind:"duration",seconds:text}` or `{kind:"exact_seconds",seconds:text}`; exact i64 text or exact canonical decimal without trailing zeros/negative zero. The types never mix. |
| typst_date/time/local_datetime | `{kind:"date",parts:[year,month,day]}`, `{kind:"time",parts:[hour,minute,second]}` or `{kind:"local_datetime",parts:[year,month,day,hour,minute,second]}`; canonical integer components, distinct types, no zone. |
| typst_instant | `{kind:"instant",utcDay:text,secondOfDay:Counter,fraction:text}`; Gregorian 0001-01-01 has ordinal 0. Exact offset conversion to UTC may produce negative days or extend beyond displayed years; seconds are 0..86399, fraction is 0 or `0.` followed by arbitrary-precision digits without trailing zeros. No leap seconds/truncation. |
| Original optional / native nullable | `{kind:"optional",state:"none"}` or `{kind:"optional",state:"some",value:tree}`; native `{kind:"nullable",state:"absent"|"none"}` or `{kind:"nullable",state:"value",value:tree}`. Only optional permits absent; all three states remain distinct. |
| Original list/object/union and native array/union | List/array uses `{kind:"sequence",values:[tree...]}`; object uses `{kind:"object",members:[{name:text,value:tree}...]}` in schema order; union uses `{kind:"union",variant:text,value:tree}`, with native variants from frozen ordinals. The complete outer type separates domains. |
| typst_dictionary | `{kind:"dictionary",members:[{name:text,value:tree}...],preserved:[{key:text,value:TypstPreservedEqualityTree/1}...]}`; known members use schema order and preserved keys sort by exact unsigned UTF-8. **Preserved dictionary-key order does not affect equality**, while raw V retains source order. Duplicate keys reject. |

`TypstPreservedEqualityTree/1` recursively consumes only SOURCE NativeValue: string/bool/int use their native kind/exact value; none is `{kind:"none"}`; float uses the float64 tree above; decimal converts coefficient/scale/negative exactly into canonical decimal and `{kind:"decimal",value:text}`, sharing keys across equal scales/negative zero. Duration/date/time/local_datetime use the table; arrays use sequence trees preserving repetitions/order; dictionaries use `{kind:"dictionary",entries:[{key:text,value:TypstPreservedEqualityTree/1}...]}` sorted by key. Unknown preserved strings undergo no NFC, URI/Field/Ref inference. Dictionaries differing only in key order may share eqKey; changed values/array order do not. Raw representation is always preserved.

EqKey is exactly the full-root D3-CJ/3 UTF-8 bytes, compared unsigned lexicographically. Hashing only accelerates with `SHA-256(UTF8("weftext.typst.equality/1")||NUL||eqKey)`; collisions still compare complete type/tree bytes. New `==`, hash/contains, group/distinct/count_distinct, min/max equivalent representatives and aggregate/deduplication K all use this sole function rather than raw bits/offsets/key arrays. Numerical/temporal sorting retains §5.2 order, without inventing natural order from CJ bytes; equality agrees with same-type comparison. After class formation, the public raw representative is the minimum full raw V or complete key-tuple CJ3 bytes, never first row or assembled members. Both zeros form one class, normally selecting positive-zero raw representation. Equal-UTC offsets form one instant class while retaining the selected complete original offset.

Group K contains only base and complete eqKey arrays in authored key order; global keys=[]; distinct K includes eqKeys in full column order. Neither contains input parents or representative-row writers. Input ordering/partitions/workers/page boundaries cannot change class counts, representatives or K. All recursive nodes/bytes/hash/sorting consume original budgets; failures never truncate trees or sample. Equality trees never replace parameter/raw literals, actual output pins or real values in SemanticStateKey: signed zeros may be equal with different output bits, while cache still binds exact raw inputs/complete dependencies.

## 6. New feature holders and typed transfer

The separate native Query entry is `TypstQueryExecute/1={kind:"typst_query_execute",version:1,workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,expectedFrontier:Frontier/2,call:<native QueryCall>,budget:<original D7 Query budget>}`, returning TypstQueryResult1 above. Old d7_query_execute/wire2 neither guesses native source nor accepts new JSON arms. Core's protected native-result profile selects Type/V/schema page consumption; entry authorization/error order remains original, without another executor. Named CEL functions `nativePresent(x)`, `nativeIsAbsent(x)` and `nativeIsNone(x)` inspect exact nullable states. `nativeValue(x)` returns T only in value state, otherwise original none_value. `nativeVariant(x)` returns frozen variant names; `nativeCase(x,"variantN")` requires a literal admitted name and matching branch to return its T, otherwise original type_mismatch. No function coerces types; all count against original budgets. The three state tests are total, the others may error. Preserved-value access is read-only and creates no Field identity.

The following are named payload roles of CORE's `FormatAdapterEnvelope/1`, not unknown kinds inserted into old PAB4, Result9/10, ExportPlan4 or closed unions. Generic D6 SourceVersion/Observation, Frontier, InputDescriptor4/Proof4/PIntent4, sole CAS/P/receipts follow CORE; document_format carries the exact Typst profile pin. Saved/planned/unknown predecessors use original record lookup/recovery without repinning or version substitution.

`TypstDefinitionInput/1` is exactly `{version:1,address:TypstDefinitionAddress/1,observation:SourceObservation/1,sourcePin:PinRef/2,value:TypstDefinitionPayload/1}`. The protected record binds the exact occurrence, author value and actual SOURCE environment; caller addresses/values authenticate nothing. action_input/prepared_action payloads follow CORE §4's exact slot matrix: action uses the Spec below, definitionInputs uses this Input, registryInputs uses SOURCE `TypstSchemaSnapshot/1` qualification, proposedInputs uses CORE's proposed_input envelope, and constructionInput uses the template role. Request/proof/sourceInputs/preview/audience/expiry/budget matches frozen input individually. There is no new binding store/ledger or parallel TypstActionInput. All native call slots select §5.4's `TypstQueryCall/1`: inline/saved, Parameter/default, arguments/context typed literals, QueryRef and DynamicBlock each use the new decoder. Outer `{query,arguments,context}` members remain unchanged; new values never enter old D7 arguments decoding.

`TypstActionSpec/1` is exactly `{format:"weftext.typst-action",version:1,intent}` and accepts only the closed set below. `TypstActionTarget/1={ref:NodeRef,observation:SourceObservation/1}` requires full preimage/source-pin/current-cut agreement rather than a revision number. `TypstActionFieldSelector/1={target:TypstActionTarget/1,fieldId:FieldId,valuePath:[text|Counter...],preimagePin:PinRef/2}` starts at SOURCE's precise Field value root; members are schema-typed, indexes only address current bounded arrays. Core revalidates complete AST/origin/bytes rather than trusting caller paths. All values below are exact schema `TypstNativeValue/1`, with typed native refs and no JSON Entry wrapper. Optional members are only those marked `?`; missing/extra others reject.

| intent.kind / exact other members | Semantics/replacement |
| --- | --- |
| `set_title` / target,value:text | Writes the unique current native title; explicitly creates an absent title, without filename fallback. |
| `set_field` / target,fieldId,value | Sets/replaces the entire author Field value; creates its precise root key if absent. Replaces old single-value replacement/ordinary field creation. |
| `remove_field` / target,fieldId | Removes the precise root author key, preserving required/relation checks and other similarly labelled fields. |
| `set_field_member` / selector,value | Replaces an exact object/array-item member, without creating missing members or changing schema type. Narrow phone editing uses this slot. |
| `insert_field_item` / selector,at:Counter,value | Selector ends at an array, at=0..length; inserts one item while preserving other order/bytes, without a fact UUID. |
| `remove_field_item` / selector,at:Counter | Selector ends at an array, at<length; removes the current exact item, not a permanent occurrenceKey. |
| `set_types` / target,types:[text...] | Changes the complete SOURCE type set under required constraints. Old assign/remove facet uses an explicit complete types set without Entry initialEntries; cleanup is separate remove_field. |
| `native_edit` / edit:EDITOR TypstEditOperation/1 | Only EDITOR's closed basic capabilities; native cell/row/heading/link adapters, never arbitrary callback/raw patch. |
| `annotation_action` / action:EDITOR TypstAnnotationAction/1 | The sole closed Annotation create/suggestion apply/reject adapter, retaining the original source/Annotation decision, accepted state and current target qualification, without adding arbitrary kinds. |
| `d3_operation` / request:CORE identity_request envelope | Original complete mode/authority/closure, including create/move/copy/restore/purge; no private D3 mode. |
| `collection_create` / collection:<native CollectionCall>,parentPolicy:<original ParentPolicy>,source:text,ordinal:Counter,requireMembership:bool | One fresh Node with the original complete post-Query, SOURCE/CORE source admission, without a prefilled trusted ID. |
| `collection_remove` / collection:<native CollectionCall>,change:<this table's set_field or remove_field intent> | Changes the exact current target Field and proves absence in complete post-Query, without implicit Trash. |
| `bulk_field` / targets:<original D7 TargetSet with EntityTarget replaced by this Target>,fieldId,operation:{kind:"set",value}|{kind:"remove"} | One atomic operation on a complete authorized set of ≤1000 Nodes; no implicit array append, batch splitting or first-value selection. |

Beyond the native-value/source/definition/selector substitutions, original complete/deduplicated target sets, whole-call live scope, potential dependency closure, owner_fields narrow qualification, relation/type/requiredness, budget/approval/full preview and sole D3|D6 submission remain. Native checklist items have no stable Task identity. Promotion requires explicit source/D3 creation, not finding a Task by text. Old promote/checklist/Entry/initial complex-provenance wire actions do not enter this Spec. Basic native table/list operations belong to EDITOR's matrix; unselected complex facts and old origin actions retain historical decoders/explicit import disposition. Annotation suggestion actions use EDITOR's closed annotation adapter rather than adding arbitrary intents here.

`TypstDefinitionTransfer/1` is exactly `{version:1,sourceAddress:TypstDefinitionAddress/1,resultOwnerSubject:PayloadSubjectKey,resultLabel:text,slots:[{path:[TypstTransferPathSegment/1...],before:TypstTransferActualValue/1,after:<closed map/preserve/reissue_definition choice>}...]}`. The producer enumerates **every** typed slot from complete preimage official AST/native decoding, and the consumer independently enumerates/compares. Ref-free definitions still require slots=[] evidence. Paths sort by unsigned UTF-8 members/numeric indexes, unique and without ancestor overlap. Labels, CEL strings and free text are not scanned. Map/preserve follows original Core identity/locality; reissue_definition selects an actual same-map target label. Final addresses use the same new owner/frozen revisionToken rather than old addresses.

`TypstDefinitionTransferPlan/1` is exactly `{version:1,sourcePins:[{owner:NodeRef,observation:SourceObservation/1,pin:PinRef/2}...],transfers:[TypstDefinitionTransfer/1...]}`. Pins sort uniquely by full Ref; exact source pins cover every sourceAddress. The complete definition-address ordering key DefinitionAddressKey is `(version,RefKey(owner),revisionToken UTF-8 bytes,label UTF-8 bytes)`, with version strictly 1. Request transfers sort uniquely by `(DefinitionAddressKey(sourceAddress),subjectCanonicalKey(resultOwnerSubject),resultLabel UTF-8 bytes)`—the complete source-address key, original result-subject key and result label. Results are additionally unique by complete `(resultOwnerSubject,resultLabel)`. Thus one complete sourceAddress may explicitly instantiate to distinct result subjects or labels: source repetition alone cannot reject it, and different sources cannot declare the same result twice.

PayloadSubjectKey, subjectCanonicalKey, RefKey and their encodings exactly reuse original D3 §4.1.3 (fixed owner `snapshots/d3-identity-references-ownership-and-lifecycle/source.md`): existing/mapped use original ranks and full RefKey; new Nodes use original ranks and numeric ordinal; Resources/Annotations use original kind rank, numeric ordinal and recursive owner-subject key; artifacts use original digest and numeric partOrdinal. Recursive owners remain closed and acyclic. Subjects retain original D3-CJ/3 canonical encoding; sorting uses its original recursive tuples rather than JSON bytes, display names, payloads or array positions. This key neither allocates identity nor expands original Node-result-owner qualification. The after choice is exactly `{kind:"map",subject:PayloadSubjectKey}`, `{kind:"preserve"}` or `{kind:"reissue_definition",ownerSubject:PayloadSubjectKey,targetLabel:text}`. The latter requires the complete actual mapped source definition address and unique new label, never a different namesake. CORE construction transferPlanPin binds this complete exact object and source version, not free JSON or another candidate map.

`TypstDefinitionTransferEffects/1` and its slots now use the complete closed structures below, replacing an untyped array. Source/result addresses decode under their actual recorded versions; Node/Resource/Annotation Refs use complete original D3 decoders, never assembly from similarly named fields.

```text
TypstTransferPathSegment/1={kind:"member",name:text}|{kind:"index",index:Counter}
TypstTransferActualValue/1={kind:"node_ref",value:NodeRef}
 |{kind:"resource_ref",value:ResourceRef}|{kind:"annotation_ref",value:AnnotationRef}
 |{kind:"definition_address",value:TypstDefinitionAddress/1}
TypstTransferEffectSlot/1={path:[TypstTransferPathSegment/1...],
 before:TypstTransferActualValue/1,after:TypstTransferActualValue/1}
TypstDefinitionTransferEffectItem/1={sourceAddress:TypstDefinitionAddress/1,
 resultAddress:TypstDefinitionAddress/1,beforeSource:text,afterSource:text,
 slots:[TypstTransferEffectSlot/1...]}
TypstDefinitionTransferEffects/1={version:1,decisionKey:DecisionKey/2,
 transfers:[TypstDefinitionTransferEffectItem/1...]}
```

Each nonempty path identifies a real static-schema typed root in this profile. Compare members by unsigned UTF-8 and indexes numerically, with shorter prefixes first; paths are unique without ancestor overlap. Transfer/TransferPlan request paths also use this Segment, without strings in one holder and objects in another. Before selects ActualValue; after retains the original closed map/preserve/reissue choice. Effect before/after kinds must match, and definition_address covers its entire root including owner without a separate nested owner slot. No new_result_subject or caller-invented allocations may appear. Complete before/result sources match actual preimage/final-after pins byte-for-byte and contain exact unique labels. Slots equal every independently enumerated AST/type slot, including parameter/default/DynamicBlock literals; Ref-free items still have slots=[].

After materialization with the same candidate map/token, effects are re-sorted uniquely by `(DefinitionAddressKey(sourceAddress),DefinitionAddressKey(resultAddress))`—the complete source/result address keys—and complete resultAddress is independently unique. Both address keys use the complete version/Ref/token/label comparator above; request subject order never substitutes for final Ref order. Each effect uses complete sourceAddress and resultLabel to select candidate plan entries, then materializes each complete resultOwnerSubject through the same map to the exact result owner and verifies the frozen token/full resultAddress. Exactly one plan entry and the actual receipt.resultAllocations full subject/Ref must correspond. No Plan/Effects/receipt array position supplies the association, and neither source nor label alone identifies an instance when a source repeats. **Authored Query/View/array order is preserved**. Preview contains complete values/source materialized from the same plan's private candidates, without public birth authority. Committed effects equal the sole winning map/token/afterPin and receipt allocations; seal cannot change map/source. Original D7Effects2 remains historical recovery only; CORE rank4 references only this new type.

Core materialization freezes the same private candidate map, revision plan/token and labels. Pass one writes all typed refs and fixed internal placeholder versions for reissued definition addresses; official reparsing determines actual definition sets. Pass two inserts the same proposed revisionToken string/owner/label, without embedding the ChangeId created only at seal, officially reparses and validates every address, SOURCE root declaration and typed value. Addresses contain no byte coordinates, so old single-line JSON Q assumptions do not apply. Nonunique definitions, remaining placeholders, unequal coverage or version changes reject the whole operation; internal candidates are not delivered. Field/link/definition/Annotation rewrites share one map/final source. Only winning CAS/P creates production versions, ChangeId, the token seal artifact and valid receipts; seal does not rewrite after source again; losing plans borrow no other map/seal. Unknown features permit source reading/saving, but copy/fork cannot skip typed transfer silently.

## 7. Native templates, birth and import

All production addresses embedded in author values—definitions/QueryRef/recipe template and nodes.source—use the opaque revisionToken above, without unknown seal ChangeIds. Tokens are not URI fragments and are never percent-decoded; original strict 43-character token decoding rejects padding/wrong tags/noncanonical base64url. Create/copy/fork materialization privately generates/freezes a random token in the same plan. Two passes insert that same token/label into complete source before freezing afterPin. The original seal artifact binds it to actual full SourceVersion/ChangeId without rewriting source. Consumers verify real historical artifacts, identity and production bindings before fresh current Observation. Unsealed/caller-invented tokens, equal stamps/bytes with different tokens and forcibly current addresses cannot qualify. Protected internal export evidence may still retain actual full SourceVersion; evidence is not mechanically tokenized.

Layout templates use ordinary Typst modules/imports, function parameters and set/show. Users can reuse mature templates rather than only fixed title/body slots. World supplies only authorized frozen packages/fonts/resources and read-only inputs. Functions cannot read ambient Workspace data, create Nodes, generate trusted ID/birth, grant permissions, write sidecars or invoke Actions. Ordinary author metadata is independent at root; template-produced same-label metadata authenticates nothing.

Application creation templates use the complete native Recipe/parameter definitions below, replacing nonexistent “SOURCE schema parameter definitions”. This is an independent ordinary Resource's internal recipe/protected construction, without becoming ordinary note business fields. UI parameter forms generate native values; layout source still uses ordinary imports/functions.

```text
TypstTemplateSourceAddress/1={version:1,owner:NodeRef,revisionToken:Token}
TypstRecipeParameterDefinition/1={name:text,valueType:SOURCE TypstValueType/1,
 required:boolean,nullable:boolean,
 default:{kind:"absent"}|{kind:"literal",value:SOURCE TypstNativeValue/1}}
TypstRecipeParameterValue/1={name:text,value:SOURCE TypstNativeValue/1}
TypstRecipeBinding/1={parameter:text,slot:{kind:"title"}
 |{kind:"body",locator:SOURCE TypstRangeLocator/1,expectedText:text}
 |{kind:"field",fieldId:FieldId}}
TypstRecipeNode/1={source:TypstTemplateSourceAddress/1,
 parentIndex:Counter|null,types:[text...],bindings:[TypstRecipeBinding/1...]}
TypstRecipe/1={format:"weftext.typst-recipe",version:1,
 template:TypstTemplateSourceAddress/1,
 parameters:[TypstRecipeParameterDefinition/1...],nodes:[TypstRecipeNode/1...]}
TypstTemplateDestination/1={kind:"parent",parent:NodeRef,ordinal:Counter}
 |{kind:"collection",collection:TypstCollectionCall/1,parentPolicy:<D7 ParentPolicy>,
 ordinal:Counter,requireMembership:boolean}
TypstTemplateConstruct/1={version:1,template:SourceVersionRef/1,
 recipe:SourceVersionRef/1,parameters:[TypstRecipeParameterValue/1...],
 resources:[SourceVersionRef/1...],destination:TypstTemplateDestination/1,
 externalNodePolicy:"reject"|"preserve_current"}
```

All members are fixed without additional optional/null slots. Names follow original `[a-z][a-z0-9_]{0,63}`; definitions/values sort uniquely by name. Unknown/duplicate/missing-required parameters reject. Required=true fixes default=absent; false permits absent, preserving template source when omitted, or a literal validated against valueType+nullable, applied only when omitted and included in full-source preview. Explicit none requires nullable=true and qualification; it never triggers defaults. Literal defaults retain SOURCE native decimal scale/sign, float bits and exact duration seconds instead of replacing NativeValue with Query canonical values. Parameter-type mismatch or violated Field enum/requiredness rejects and cannot be accepted as loss.

nodes[0].source matches recipe.template/the requested template's authenticated complete production version, with parentIndex=null. Other parents precede children; source owners are unique and belong to the selected live template subtree. Multiple roots/cycles/repeated Nodes under different revisions reject. Types are a valid SOURCE short/full-name set expanded completely under current schema; ID/types are not Field parameters. Bindings sort uniquely by parameter, slot rank title/body/field and exact locator/FieldId. Each node has at most one title binding and one binding per FieldId. Body ranges never overlap or cross control/Field/title regions.

Title/body parameters must be nonnullable strings, never code/content. Title safely encodes text for the unique native title. A body locator identifies the same source owner/token's unique exact inert-text range. Official AST/current Source reads prove expectedText, complete byte slice and paragraph/content boundaries. Replacement uses only literal encoding; `#link`/`#metadata` text never executes or establishes links. Cross-structure/nonunique/readonly-generated sources reject this binding rather than performing whole-file replacement. Field parameters match the complete installed FieldId/valueType/nullable/enum and write the full native Field through SOURCE's container writer. Arrays preserve order/repetitions, without old field_append/Entry UUIDs. Functions/imports handle presentation; Recipes execute no arbitrary macro or permission grant.

Capture selects template → validates parameters/schema → reads full authorized sources/resources/necessary Annotation index → analysis/loss/actual Source preview → Core creates fresh Nodes/needed Resources through one candidate map → final validation and one save. Only Core assigns IDs/birth/placement/permissions, never title/filename guessing. Resources explicitly re-own. First-generation template Annotation omission requires a complete authorized index and explicit omission; it does not make every export/copy drop Annotations. An entire instance is one coupling group, without splitting around 1000 descendants/stricter budgets. New template roles carry recipe/evidence; old Recipe2/TemplateConstructionInput2 retain history. Only built-in adapters produce authorized construction.

### 7.1 Complete construction input, Resource/loss and same-map effects

```text
TypstTemplateInputPin/1={sourceVersion:SourceVersion/2,
 observation:SourceObservation/1,
 payloadKind:"exact_source_document"|"resource_bytes",pin:PinRef/2}
TypstTemplateSubjectBinding/1={sourceRef:NodeRef|ResourceRef,
 subject:<D3 new_result_subject>}
TypstTemplateLossLocation/1={kind:"template_source",sourceVersion:SourceVersion/2}
 |{kind:"template_annotation",target:D9EntityVersionAddress/2}
TypstTemplateLossChoice/1={lossKey:Counter,choice:"accept_loss"|"reject"}
TypstTemplateConstructionInput/1={version:1,
 construction:TypstTemplateConstruct/1,recipe:TypstRecipe/1,
 inputPins:[TypstTemplateInputPin/1...],
 omittedAnnotations:[D9EntityVersionAddress/2...],lossReport:LossReport/1,
 lossChoices:[TypstTemplateLossChoice/1...],
 sourceSubjectBindings:[TypstTemplateSubjectBinding/1...]}
```

Unchanged leaves reuse their original exact owner contracts: SourceVersionRef/1 is D6's current protected observation reference, not a production token. Resource/recipe selection, inputPins/sourceSubjectBindings and D9EntityVersionAddress/2 come from `d6-file-authority-reopen/owners/d9-templates/source.zh-CN.md` §6a and workers-export §3b. Construction.template is a Node; recipe is its owner-local Resource. Recipe pins are independently strictly parsed and equal constructionInput.recipe byte-for-byte. Resources sort uniquely by complete D3 RefKey and have owners in recipe.nodes. sourceToken denotes same-cut current authorized observation, never path/hash inference. Parent/collection destinations retain original D7 qualifications. TypstCollectionCall/1 is exactly `{call:TypstQueryCall/1,nodeColumn:text}`, retaining original closed CollectionCall members and replacing only call with §5.4's Call, without opening collection/ParentPolicy members.

Input pins sort uniquely by complete RefKey and exactly cover actual recipe bytes, every Node source and selected Resources. sourceVersion equals observation.sourceVersion; entity/domain/payloadKind/pin bytes match individually. Document pins retain strict UTF-8 bytes without semantic-text reordering; Resource bytes are unchanged. Node addresses resolve through the same sealed managed tokens to complete sourceVersion; internal input pins may retain real versions without embedding ChangeId in authored source. Every sourceSubjectBinding identifies a selected Node/copied Resource, with original strict same-kind D3 new_result_subject. Resource ownership mechanically identifies its selected owner's fresh Node subject. Sources/targets are individually unique and completely match request.freshSubjects, proposedInputs and the same private map. Uncopied recipes create no bindings. This map publishes no new identity; display joins it only to actual receipt.resultAllocations, never positions/titles.

omittedAnnotations is the complete same-cut authorized owner directory's D9EntityVersionAddress/2 array sorted by RefKey. Initial templates omit all Annotations without reading bodies or creating subjects; missing authorization/negative-range proof is not an empty list. LossReport/1 strictly reuses original Import IR/Mapping §7's closed `{version:1,items:[{lossKey,feature,locations,effect,severity,allowedChoices}...]}`. Feature uses the original named set, effect/locations are nonempty, and locations use only this TypstTemplateLossLocation: sourceVersion occurs in inputPins and Annotation addresses in omittedAnnotations, without Import SourceLocation. lossKeys are continuous; notice has choices=[], requires_choice has [accept_loss,reject], blocking has [reject]. lossChoices sort uniquely by key and exactly cover requires_choice/blocking, never notices. Reject cancels without an author request; blocking cannot prepare. A real constructionInput preserves the complete initial nonblocking report and accept_loss for all requires_choice items, retaining initial severity/allowedChoices. Input/parameter/Resource/directory/schema/loss drift requires new analysis/confirmation rather than accepting old keys.

Three positive fixtures specify the construction chain without making ordinary notes use internal wire. **TP-01:** an authorized single-Node template has one unique body-text slot and installed string Field `user/report-code`, with required nonnullable string parameters `body`/`code` bound to body locator/FieldId. Resources/Annotations/loss/definitionTransfers are empty. Current inputPins covers exactly recipe+Node and one sourceSubjectBinding maps the source Node to a new Node subject. Core writes fresh control ID/target types and safely replaces body/Field through the same map; two complete parse passes freeze source. Complete effects/receipt cover exactly that Node without D7 transfer effects. Body text containing `#metadata` renders as text only.

**TP-02:** two Nodes A/B in one live template tree each have an authenticated Feature definition, with typed parameter/default Refs mutually pointing to A/B. Recipe nodes are parent-before-child, bindings may be empty, Resources are empty, inputPins exactly recipe+A+B, and sourceSubjectBindings identifies two new Node subjects. Enumerate every Ref slot from actual before Feature payloads, map to A'/B' through the same map and use each plan-frozen revisionToken for production addresses. Two-pass source/Field/link/definition materialization and the new parameter decoder validate completely. Both effect items contain actual A/B→A'/B' before/after slots, joined one-to-one with actual receipt allocations. Existing template cross-references qualify copy materialization; nonexistent caller UUIDs cannot create fresh cross-references. The collection branch still rejects multiple Nodes/references requiring fresh mapping.

**TP-03:** a single Node with a valid Ref-free Feature, such as §5.1's list Query, still produces one TypstDefinitionTransferPlan.transfer and Effects item with slots=[]. Complete before/after payload/source and new result-address tokens are verified, never skipped as opaque data. All fixtures qualify only with a complete authorized cut, strict/managed_atomic, actual source/recipe/Registry pins and budgets. Wrong parameter types, repeated sources/subjects, mismatched Resource owners, stale/cross-structure body locators, omitted typed slots or false receipts reject the whole construction without allocation; no batch splitting or replacement maps repair it.

Weftext-owned Rust infrastructure remains production import authority. SourceArtifact/ImportIR coverage/mapping/loss/ImportJob remain; Office whole-sheet/hidden/formula-cache/part coverage, exact numbers/coordinates and safety validators are mandatory. Output is built as target native Typst AST, validated by official parser/compiler and complete SOURCE/Registry/typed-reference gates. Compilable PDF does not prove metadata/Query/link preservation. AsciiDoc/YAML/Markdown/JSON Entry import retains originals and maps declarations/native values/features/links explicitly. Complex fact models do not regain JSON fallback in new notes; explicit loss/rejection or ordinary attachments are required. Unknown dialects and LaTeX/Mermaid compatibility use named mature packages/providers with support/loss reporting; native math is not arbitrary LaTeX equivalence.

D6 still owns ImportJob, with sequential batches of bounded coupling groups. Committed prefixes never roll back; unknown restores original OperationId/request. Cancellation blocks only not-yet-planned later batches. Equal-digest reimport promises no upsert/deduplication. Format change invents no deployed migration and does not overwrite history on read.

## 8. Output, offline routing, intranet downloads and extensions

Exact native Query result export uses separate `{format:"weftext.typst-query-result-export",version:1,schema:TypstTerminalSchema/1,data}`: rows data is the complete V-row array, scalar is one V, and graph is exactly `{nodes,edges}` with complete V-row arrays. It retains this profile's types/V/bag/order without Query definitions/runtime handles/cursors/Action rights. Old weftext.query-result-export1 consumes only original TypeSpec/V, without native unknown arms. DOCX/XLSX render new scalars as exact text or genuinely supported typed cells: float retains round-trip display/raw-bit origin, duration uses exact seconds, and date/time/local-datetime acquires no timezone. Unsupported precision/nullable/structure is itemized loss requiring explicit choices rather than approximation or omitted Fields. Exact native result export still needs no renderer.

### 8.1 Real capabilities and closed render binding

Exact-source export delivers original `.typ` bytes with generationPolicy=none, independent of compilation/templates/renderers and without preference-based link rewrites. Exact Resource/query_json remain independent; query_json contains complete terminal schema/data/typed values, never Query definitions. Official Typst PDF/SVG/PNG provide paged/graphic output. PNG has no clickable navigation and cannot claim equivalence. HTML in 0.15.1 is experimental; flow/accessibility delivery gates belong to EDITOR. “Save anything as a webpage” cannot replace Word/workbook requirements. [Official export status](https://typst.app/docs/reference/html/), [official implementation](https://github.com/typst/typst/tree/v0.15.1).

DOCX/ODT, XLSX/ODS and CSV/TSV are named Weftext conversion capabilities, not Typst built-ins. Word needs actual paragraphs/styles/tables/resources, CJK/RTL/pagination and loss verification. XLSX needs actual workbooks/typed cells/order/formula-text safety/merges and Office/WPS/LibreOffice interoperability; a PDF table is not a workbook. D9 visible Office tokens/shortest unique native-table selectors/style/repeat admission remain, using SOURCE's table producer instead of AsciiDoc. Missing unique projections cannot guess from titles/first tables. Unsupported complex valid Typst reports loss or unavailable routes, retaining Source saving.

`TypstExportBinding/1` is exactly `{version:1,sourceInputs:[{owner:NodeRef,sourceVersion:SourceVersion/2,observation:SourceObservation/1,sourcePin:PinRef/2,snapshotPin:PinRef/2}...],environment:CORE TypstProcessorEnvironment/1,worldProof:CORE TypstWorldProof/1,presentation:"flow"|"paged",resultPins:[CORE TypstResultPin/1...],viewSpecs:[TypstViewValue/1...],route:<original D9ExportRouteBinding/1 or null>,linkRoutes:[TypstExportLinkRoute/1...],linkLosses:[TypstExportLinkLoss/1...],outputs:<original D9ExportStagedOutput/1 array>,lossReport:<original ExportLossReport/2>}`. Only actual rendering uses this binding; exact-source uses none. Flow uses EDITOR production renderer `flow-semantic/1`: official paged post-realization semantic projection to controlled DOM or native accessible nodes, with environment.target=paged rather than experimental HTML by default. Paged preview also uses the paged target. Experimental HTML uses environment.target=html only when explicitly selected and admitted by a named route, without another shared policy. Each snapshotPin binds the real CORE TypstSourceSnapshot/1/environment/world/profile. Every actually consumed source/template/font/asset/resource is independently authorized, without whole-body rights for pure Field/Query export. Actual paired resultPins/viewSpecs retain full cut/auth/epoch/schema/order. Renderers do not rerun Query, fetch extras, aggregate or sample. Semantic-chart export keeps six original layouts: metric/bar/line/scatter/pie/heatmap. A valid network source View remains unavailable on that route rather than quietly becoming data export.

This new export role combines with CORE's frozen confirmed plan and retains D9 prepare→inspect→confirm→delivery/state/cancel, controlled names/alias/prefix/reserved conflicts, complete staging/verification/budget and current authorization. It is not an unknown arm in old ExportPlan4/D9DocumentRenderBinding1. Historical Plan1–4/confirmations/receipts recover as recorded.

`TypstExportLinkLoss/1={occurrence:TypstCompiledLinkKey/1,lossKey:Counter}` links the exact item in the actual complete initial ExportLossReport/2. The key must exist and source range/output location must be independently reproducible, without another loss authority. Every §2.4 active application occurrence appears exactly once in either linkRoutes or linkLosses, without overlap. Both sort by compiled index and share binding-basis owner/sourceVersion/compilation/projection pins. Loss cannot omit links or disclose hidden targets. Routes/losses consume the complete Projection before confirmation; old Plan4 gains no such slots. `TypstExportLinkRoute/1` is exactly `{version:1,sourceOwner:NodeRef,sourceVersion:SourceVersion/2,occurrence:TypstCompiledLinkKey/1,target:NodeRef,authorLabel:text|null,outputName:text,outputFragment:text|null}`. The producer proves authorized targets/exact labels/controlled output destinations per occurrence within the frozen output set. Consumers verify unique target output and correspondence to actual renderer named destinations/HTML IDs, not assumed equality with author labels. The official renderer may change label-to-DOM-ID mappings, which must be verified. Source/target-proof/output conflicts, missing routes, duplicate labels or omitted targets cannot create apparently usable offline paths. Explicit retained application-URI/loss/reject choices apply. Routing does not rewrite author source or grant runtime permission. Only proven closed output sets promise cross-document offline links; unsupported SVG/PNG target links have itemized loss.

### 8.2 Intranet artifact downloads versus publication

WebUI intranet download needs neither external directories nor PublicationReceipt. Users choose inputs/format → complete artifacts are generated/validated → exact loss/output list is reviewed → the same frozen plan is confirmed → job/delivery state and authorized dataFile downloads are displayed. Confirmation binds identical inputs/loss choices/staged bytes; worker exit 0 or preview is not success. Tokens bind plan/confirmation/exact dataFile/current audience/bounded retention, without local paths or ambient Resource rights. Every request, chunk and resume range revalidates current authorization/retention. Revocation immediately stops subsequent delivery, without hidden file counts/names/errors. Authorization/token checks precede existence diagnostics.

“One download” means one explicit delivery operation, not lifetime permission after one token read. Retry/resume only continues **identical retained file bytes**, without recompiling. Expired/revoked/interrupted/unknown states differ. Regeneration after expiry requires a new plan/confirmation, never automatic publication reruns/new bytes. Content type/controlled filename/length bind the actual dataFile. Invalid output is isolated; arbitrary extra worker files are never delivered. Bounded jobs/cancellation do not discard author Drafts.

External bundle publication retains create-only staging/flush/provable atomic rename/receipts. Unknown restores only original intent, without renaming to fake success. Print has a print outcome. “Save as Resource” separately requires Core author-write/receipt; export confirmation grants no write capability. These three outcomes and pure download are presented separately, never substituted.

### 8.3 Packages, plugins and extensions

Intranet installs/admission explicitly freeze package/font/plugin/provider versions/digests/license/platform. World supplies authorized resources from the frozen environment without silent networking. Typst functions/WASM compilation capabilities grant no Core/Server network/secret/Action/author transaction rights. D10 authorized connectors/actions retain capability/catalog/budget/preparation/confirmation gates. D9 workers default to no network/Workspace mount/path/command access and use host-assigned immutable slots with independent declared-output validation. Unknown profiles/variants get no arbitrary fallback; Mobile conversion execution follows D1 unavailable boundaries. Malicious packages/ZIP/XML, budget exhaustion and unconfirmed process-tree exit reject/quarantine rather than using loss acceptance to bypass safety.

## 9. Named replacements and retained selections

| Fixed old owner/section | New owner/disposition |
| --- | --- |
| LINKS §§1–5, LINK-01–10; EXPERIENCE §5.2 | §§2–3 replace AsciiDoc xref/codec/source writer, retaining four domains/default ID/shortest operation/authorization/same-target/copy. Old deferred “ID has no anchor slot” becomes typed Node+authorLabel, explicitly not durable Locator. |
| LINKS §6/LINK-11; D9 §12/12a, D9-SCHEMAS §§9–14, D9-INTERFACES §§7–9 | §8/new export roles replace language-bound bindings; exact-source/query_json/typed Office/staging/delivery remain, with closed offline target/fragment routing. Old Plans do not widen. |
| EXPERIENCE §5.1, UX-10–13/17–18 | §4 uniquely retains/replaces native link encoder/Source operation. EDITOR owns actual focus/IME/RTL/AT acceptance. |
| D7 §§4/7/9/11–14/18; D7-SCHEMAS §§3–5; D7-SEARCH §11.5; d7/owners/definition-transfer §§1–6 | §§5–6 replace saved blocks/Entry producers/JSON Q/language-bound holders. Original DAG/Value/CEL/Action/completeness/history/current Contribution/Trash proofs remain. |
| D7-QUERY-V2 §§2–8, D7-SEARCH §§1–11; d7/owners/view §§1–9; EXPERIENCE §§3–4 | §5 retains search, Field/body domain boundaries, View set, scope/new carrier/cold reopening. SOURCE defines simpler native base values. |
| D3 §§6/8/11/14/21, D9 §6, D9-SCHEMAS §§5–7; EXPERIENCE §§3/6 | §§3/6/7 plus CORE construction replace source materialization slots; same map/fresh birth/owner/version/currentness/receipt allocations/sole commit remain. Old symbolic/locator/history remains unchanged. |
| D9 §§3–5/8–11/13–15, D9-INTERFACES §§1–6/10–12 | §§7–8 replace Typst generation/environment inputs, retaining IR/coverage/loss/typed values/ImportJob/Worker/regions/authorization/original recovery. Annotation reading stays with EDITOR/Core's current owner. |
| EXPERIENCE §§1–2/5/7–8 | Side-by-side writing/outline/layouts/annotation navigation remain with EDITOR. Selected Query/link/template/output behavior remains here. Deferred dashboards/complex forms/whiteboards/learning/read-aloud/sync/arbitrary scripts/library rollback/realtime cursors do not expand scope. |

The original 21 closed findings retain their original fixed-candidate evidence. New carrier/source/Field projection/typed transfer/Effects/query/read/save/download/format-binding chains are rechecked this round rather than inheriting old CLOSED acceptance. Observable semantics remain; Asciidoctor-specific syntax/types/coordinate proofs do not transfer.

## 10. Positive/negative design acceptance and limits

| ID | Positive / required rejection or explicit degradation |
| --- | --- |
| TF-01 | Four forms retain Node+label/text/content; `/x` is not Workspace-root, duplicate titles never default-select. |
| TF-02 | Space/CJK/percent/plus encode exactly once; bad UTF-8/lowercase escapes/encoded slash/backslash/parent escape/aliases/query reject the application codec while preserving source. |
| TF-03 | Node rename preserves ID target; label rename only invalidates fragment. Missing/duplicate labels/target-read permission disable lossless conversion rather than dropping fragments. |
| TF-04 | Undisclosable paths offer only disclosable ID; Server reveals no disk URI; unconnected Workspaces do not guess namesakes. |
| TF-05 | One conversion/Undo/full Source reparse; shared variables/include/ABA/revoked/stale sources never write. Exact broken-link copying works while converted copying validates targets. |
| TF-06 | Title/aliases, same-range multiple targets, nested/crossing ranges, max2/overflow/>20/keyboard/touch/RTL remain reachable; disable/cancel never changes source. |
| TF-07 | Combining/ZWJ/CRLF and scalar↔byte↔UTF16 differences do not expand selection; generated/real-link/metadata content gets no writable hints; incomplete scan is not complete. |
| TF-08 | Literal metadata definitions read independently; body compilation failure retains Source read/save. Duplicate key/label, computed Query carrier, mixed old QuerySpec/new address and none-as-null cannot execute. |
| TF-09 | Table/board/calendar share Task fields/body; computed/aggregate/stale rows cannot write; repeated contact arrays edit the precise current second item without fact UUIDs. |
| TF-10 | Generated heading rows read without source writers; distinct/global aggregate inherits no representative-row rights; partial results grant no completeness/Action/outside-membership assertion. |
| TF-11 | Contributions cold-reopen with exact ID/version/descriptor/owner; stopped C is not replaced by an intact Field; Trash coverage is complete/same-cut, hidden counts stay hidden and Trash grants no Action. |
| TF-12 | Copy/link/Field/definition share candidate map/revision/receipt; unknown definitions/full-fork missing slots reject; CEL/text UUIDs are not replaced; old Effects2/Results keep decoders. |
| TF-13 | Native import/functions reuse layout; false IDs/template metadata/packages cannot birth or grant rights; body parameters execute no arbitrary code; unknown creation is never duplicated. |
| TF-14 | Whole IR coverage/typed values/full loss/source preview; default conversion metadata/Query/xref loss cannot pass because PDF succeeds; original input remains. |
| TF-15 | Real DOCX/styles and XLSX typed cells need Office/WPS/LibreOffice checks; PDF tables/experimental HTML/missing routes do not claim equivalence; source reading/writing is independent. |
| TF-16 | Export occurrence→target→actual output-label route round-trips; missing/duplicate/omitted targets explicitly lose/reject, without guessing HTML IDs or changing source. |
| TF-17 | Intranet delivers exact confirmed dataFiles without external directory/receipt; per-chunk auth/same-byte resume/new confirmation after expiry; invalid tokens reveal no existence/path and extra worker files are not delivered. |
| TF-18 | Download/print/Resource/create-only publication have distinct outcomes; unknown restores original intent, ImportJob retains committed prefix and packages/workers have no ambient authority. |
| TF-19 / TR-01 | QP-01's two sources use a genuinely sealed callee definition/installed Field schema: float parameters/defaults, same-type QueryRef forwarding, empty arguments applying defaults, native float→exact bits, sorting/projection/exact export. Dynamic blocks' current Node/title/runtime contexts use new literal decoders. Reject old literals mixed with new types, integer-as-float, invalid bits/nonfinite values, mismatched defaults, missing required/extra parameters, QueryRef row/scalar/current-Node bindings or false revision tokens. |
| TF-20 / TR-02 | +0/-0 share eqKey while retaining both raw patterns; Z/+08:00 share a UTC-instant key; exact seconds, native decimal scales and recursive array/dictionary/nullable/union equality follow the definitions. Distinct/group/count_distinct class counts, minimum raw representatives and K remain invariant under input order/partition/page changes; global K has no parent. Reject wrong types, hash-only collision comparison, tree truncation, raw bits as EqKey or replacing actual raw cache parameters with eqKey. |
| TF-21 / TR-03 | QP-01 parameter.type/default.type/View.inputSchema at any nested typst_array.maxItems:none maps exactly to null without inventing finite N; only that schema slot permits none. Reject raw V.none/null, default:none, none placeholders for absent optional members, conflated nullable absent/none or null in unknown schema slots. |
| TF-22 / TR-06 | Two equal-looking root links have distinct AuthoredKeys; include/show duplication of one span has two compiled indexes, while one wrapped link's reading fragments share one index. Complete AST/final trace/association independently agree. Nonliteral catalog destinations may be null without a writer; old catalog qualification cannot authenticate new Draft bytes. |
| TF-23 / TR-06 | Each active compiled application key has one route or same-key loss, with actual target/fragment/output validation. Reject missing/extra/duplicate items, wrong projection/compile/World cuts, originId/DOM keys as occurrences, merged repeated includes/partial coverage claiming completeness or clickable removed_by_show output. |
| TF-24 / TR-09 | TP-01 safely materializes body+Field parameters in one Node; TP-02 reissues existing two-Node/definition cross-Refs through one map; TP-03 Ref-free transfer/effects retain slots=[]. Complete pin→construction→two-pass source/typed slots→Effects→sole receipt match individually. One complete source definition explicitly instantiates to two qualified distinct Node result subjects with the same label, producing two entries under full request-subject ordering. Actual Ref order after same-map materialization may be reversed; re-sorted Effects still associate both entries through complete source/result keys and actual receipt mappings. |
| TF-25 / TR-09 | Defaults apply only when omitted, preserving native scale/bits. Current Resource Ref/owner, recipe pins, omitted Annotation directories and initial loss/accept choices match. Executable-looking body text stays literal. Wrong parameter/valueType/nullable/FieldId, stale/cross-structure body ranges, wrong Resource owners, repeated/missing subjects, omitted effect paths, mismatched before/after kinds, invented candidate UUIDs, extra notice choices/accepted blocking/drifting loss reports or position-based receipt mapping reject the whole operation. Reject repeated complete result subject+label from equal or different sources, repeated complete effect result addresses, keys omitting source revision tokens, JSON or ordinal-string sorting, request-position association after effect reordering, and source-only/label-only matching of repeated-source instances. |

These oracles are unexecuted. Research proves only bounded official AST/value/compilation/source-splice feasibility, not ACL, library performance, cross-device behavior, IME, Office interoperability, offline routing or production HTML. Implementation must connect these cases to real Core/Server/EDITOR tests and retain D9 I01–I12 plus safety/platform/accessibility gates. Design text is not product acceptance evidence.
