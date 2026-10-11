---
source_language: zh-CN
translation_of: D7-SEARCH.zh-CN.md
translation_status: synced
---

[简体中文](D7-SEARCH.zh-CN.md)
# A2 D7 Search Semantics and SEARCH-01–08

Status: normative D7 search design inside the current author candidate. The fixed85bdadf independent review closed the prior Registry/QuerySpec coordination findings and left only A2-D7-2F89-P2-01; this revision author-repairs that machine-oracle residual and still requires fixed-SHA independent review. D8 interaction implementation remains a later full-module batch.

## 1. Decision and one execution authority

The selected UX remains plain text by default, visual filters for ordinary structured use, and an explicitly selected shortcut-condition mode for experts. Ordinary users never need shortcut syntax. Automatically interpreting every colon as syntax remains rejected.

All successful search execution uses Core QuerySpec/2. There is no second full-text executor, hidden shell filter, search-only data store or durable search authority. When Nodes and Resources are both selected, one QuerySpec/2 builds homogeneous branches and combines their identical public search-row schema with union_all before the final sort/project.

Saving stores the canonical Query definition, not the shortcut string, visual-chip order or device state.

## 2. Search objects and current Query sources

The vocabulary remains deliberately separate:

- document title: current D2 native title, Optional;
- document subtitle: current D2 native subtitle, Optional;
- Node file name: basename derived by the QuerySpec/2 node_file_name source;
- Node relative path: exact PortableRelativePath from node_relative_path;
- Resource file name: basename from resource_file_name;
- logical hierarchy: D3 selector/placement scope, never a stable path identity;
- body: D2 semantic body_text;
- contributed text: explicitly selected current SearchContribution Field/textPath.

The exact authorization, SourceObservation/FileObjectBinding, same-cut, rename/move reset and no-identity/no-write rules for the first five built-in sources are in D7-QUERY-V2. A titleless Document remains titleless; filename/path never fabricates title. Equal titles or file names never merge distinct Refs.

## 3. Presets, empty input and source enablement

File-list filter defaults to the current selected parent, nonrecursive, live Nodes only. Its ordinary-text sources are title and node_file_name. body, subtitle, path, Resources, Annotation bodies, Trash and attachment extraction/OCR are off unless explicitly enabled. With no ordinary needle and no structured condition, this surface is browse mode and executes no search Query.

Quick Open defaults to authorized live Nodes under the selected Workspace/root scope with title, subtitle and node_file_name enabled. body and path are off. A Resources control adds the Resource domain and resource_file_name. With no needle and no structured condition, Quick Open is browse mode, not an empty-needle search.

Global text search defaults to authorized live Documents in the selected Workspace/root or explicit subtree with title, subtitle and body enabled. Node filename/path, Resources, attachment extraction/OCR, Annotation bodies and Trash are explicit source/scope controls. With no needle and no structured condition, Global Search does not enumerate the Workspace; it is an unexecuted empty draft and submit returns invalid_request.

A structured field/source condition is a valid search without a plain needle. An explicit shortcut source term also enables that source even when the preset default is off: for example @body:x in Quick Open enables body, @path:x enables Node path, and @resource-name:x enables the Resource domain and resource_file_name. The equivalent visual control changes the same compiler input. Permission/provider failure remains a Query failure; an explicit source is never silently dropped.

Source applicability is closed. title/subtitle/filename/path/body/@field are Node-only; resource-name is Resource-only. NOT preserves its child's domain set, AND intersects domain sets, and OR unions them. An AND whose domain intersection is empty is source_not_applicable rather than an accidental zero-result query. This prevents NOT @title:x from becoming an all-Resources predicate.

## 4. Match, Unicode, Boolean composition and sort

The current matching boundary is unchanged. Built-in title/subtitle/body and file metadata use their declared exact comparison. A D4 text path with normalization:"exact" is exact and case-sensitive; normalization:"nfc-for-compare" applies NFC to candidate and needle for comparison only and never rewrites source bytes. contains is exact substring on the selected comparison basis. There is no automatic case folding, fuzzy match, stemming, tokenizer, transliteration or Pinyin.

CJK and RTL text are ordinary Unicode scalar text. RTL changes presentation only. Ordinary plain-search mode treats the entire input as one literal phrase; AND, OR, NOT, colon, @, URL syntax and parentheses have no special meaning there.

For default search ranking, exact title is rank 0; exact subtitle, Node/Resource file name and contribution role=name are rank 1; exact alias or exact Node relative path is rank 2; non-body substring matches are rank 3; body/content-only matches are rank 4. Only already-authorized values participate. Equal rank is followed by the explicit user sort when present, then canonical subject key and contribution/source identity for a deterministic tie. UI order and locale collation never supply a hidden tie.

## 5. Shortcut lexer and recursive grammar

Shortcut parsing is active only after the user explicitly selects shortcut mode. The input is one UTF-8 line; NUL and line breaks are invalid.

~~~text
Shortcut mode v1

shortcut-input := ws? or-expr ws?
or-expr        := and-expr (ws1 OR ws1 and-expr)*
and-expr       := unary-expr ((ws1 AND ws1 | adjacency) unary-expr)*
adjacency      := ws1
unary-expr     := NOT ws1 unary-expr | primary
primary        := field-condition | literal-term | "(" ws? or-expr ws? ")"
field-condition := builtin-field ":" value
                 | field-ref ":" value
builtin-field  := @title | @subtitle | @filename | @path | @body | @resource-name
field-ref      := @field "(" field-id ("," member-path)? ")"
value          := quoted-value | bare-token
literal-term   := quoted-value | bare-token
~~~

The lexer runs before the grammar:

- ASCII space and tab separate tokens outside quotes.
- Parentheses are structural only when unescaped and outside quotes.
- Colon is ordinary text except the one colon immediately following a recognized field operator head.
- A bare token is a nonempty sequence of Unicode scalars other than unescaped space/tab/quote/parenthesis. Backslash followed by one of backslash, quote, parenthesis, @, colon, space or tab decodes to that escaped scalar. Before any other following scalar the backslash itself is literal. A backslash that is the final scalar of a bare token is **always one complete literal backslash**, never an incomplete escape.
- quoted-value starts and ends with a double quote. Inside it, backslash escapes only double quote or backslash; every other scalar, including whitespace, colon, @, CJK and RTL text, is literal. If EOF occurs after an opening quote, the draft is incomplete. If the last scalar inside that open quote is a backslash, the unique classification is `incomplete_quoted_escape` at that backslash; otherwise it is `incomplete_quote`.
- unescaped uppercase ASCII AND, OR and NOT are keywords only as complete lexer tokens. Lowercase forms are literals. To search the uppercase words literally, quote them.
- a token beginning with an unescaped @ followed by a recognized operator head is parsed as a source term. An unknown @name: operator is unknown_shortcut_field, not a literal fallback. Escaping the leading @ makes the whole token literal.
- field-id uses the exact D4 FieldId grammar. member-path is 1..8 dot-separated lowerCamel ASCII ObjectMemberSpec names, each 1..64 bytes. It is validated against the same current Registry/Field TypeSpec used by Query. Labels and localized names never substitute.

OR has the lowest precedence, then AND/adjacency, then recursive NOT, then primary/parentheses. NOT NOT A is therefore legal and remains two explicit not nodes. A AND B AND C and adjacency A B C are legal chains.

An unmatched parenthesis, missing operand, missing field value, incomplete @field(...), `incomplete_quote`, `incomplete_quoted_escape` or other unfinished structured token is a draft-incomplete state and executes no Query. A trailing backslash in a bare token is not in this set: it is a literal scalar. On explicit submit an incomplete state returns invalid_request with the exact source span. A syntactically complete unknown field/operator/value returns its specific parse/compile error. No error path executes an alternate literal Query.

## 6. One condition AST and deterministic Query compilation

Shortcut input and visual controls both produce the same ephemeral compiler AST:

~~~text
SearchConditionAst/1 :=
    {kind:"literal",value:text}
  | {kind:"source_term",source:<closed source selector>,value:text}
  | {kind:"not",child:SearchConditionAst/1}
  | {kind:"and",children:[SearchConditionAst/1...]}
  | {kind:"or",children:[SearchConditionAst/1...]}
~~~

This AST is not persisted and is not a second query language authority. and/or nodes flatten only adjacent nodes of the same operator while preserving source order; NOT is never algebraically cancelled. Visual editing uses the same node kinds and source selectors, so switching surfaces cannot drop a condition.

Compilation is deterministic:

1. resolve the preset, explicit scope and enabled sources;
2. validate every source and Field against QuerySpec/2 and the current Registry;
3. compute each AST node's applicable subject domains using §3;
4. create the Node and, when enabled, Resource scan/read/filter/project branches;
5. project every surviving branch to one identical search-row schema with Optional<NodeRef>, Optional<ResourceRef>, Optional<text> display, rank and stable source key;
6. combine multiple branches with QuerySpec/2 union_all;
7. apply the explicit deterministic sort and final project.

Optional title/subtitle values match only in the some branch; none is never coerced to empty text. A titleless body match may therefore carry display=none, and the UI may render a non-authoritative placeholder without changing Query data. A condition outside the shortcut/visual subset remains an opaque advanced Query condition in the visual editor and is never lost. It can only be edited by the full Query editor. Saving always saves QuerySpec/2, never SearchConditionAst/1.

D7-SEARCH-FIXTURES.json is the machine oracle for parser/visual/compiler equivalence. Every positive fixture contains the actual shortcut AST, an explicit UI-neutral visual-condition input, the visual AST, the expected canonical AST, explicitly non-wire `compilerReviewDescription` metadata, an actual `expectedQuerySpec` that is strict-decodable as QuerySpec/2, and a full `expectedCanonicalGraph` serialization of the existing Query/View/Action §5 algorithm rather than an unexplained digest. `expectedQuerySpec` has exactly the normal `format/version/parameters/relations/scalars/result` top level and uses the real plural scan domains/selectors, read `input/from/bindings`, CEL filter predicates, `{name,expr}` derive/project fields, `{expr,direction,none}` sort keys, TypedLiteral/Optional parameters, result shape, and QuerySpec/2 `union_all` where required. The review description is only a readable record of preset/scope/source compiler inputs and cannot be decoded or executed as Query.

`expectedCanonicalGraph` starts at the result relation, removes author relation IDs, assigns `canonicalOrdinal` by the retained dependency order, records non-reference parameters, canonical `weftext.cel/1` ASTs, reference ordinals and declared field order, sorts ParameterSpecs by name bytes, and preserves `union_all.inputs` order. Its CEL-AST object rendering is fixture-only structural evidence for the existing profile, not a new CEL syntax, wire format or graph protocol. The mixed Node/Resource OR fixture first specializes the SearchConditionAst per legal domain, compiles distinct legal CEL predicates, projects one identical public schema, then uses the real `union_all` and final sort/project.

For a positive fixture, shortcutAst, visualAst and canonicalAst remain byte-equal under the stated canonical AST serialization; `expectedQuerySpec` and `expectedCanonicalGraph` are the expected outputs for both surfaces. To materialize the opposite-domain absent Ref without inventing a Ref literal or an overridable Optional<Ref> input, the fixtures use a version-2 parameter whose type is `list<NodeRef,0>` or `list<ResourceRef,0>`, whose only legal value is the empty list, with default `[]`; `at(param.empty_*_refs, 0)` therefore yields the required typed Optional none and cannot be changed to a present Ref by invocation arguments. Negative, incomplete and browse/no-query fixtures record both objects as null and execute no alternate Query. These are design oracles only: the product QuerySpec/2 decoder, CEL parser/compiler and Search compiler remain UNRUN.

## 7. Permission, index state, count, ranking and snippets

Authorization and minimum disclosure precede every sensitive source, Field, contribution, attachment, Annotation or index-private read. Hidden objects cannot leak through hit counts, rank gaps, snippets, completions or unavailable reasons.

Derived Index remains only a candidate accelerator. building, partial, stale and unavailable are distinct from a proved complete zero-result Query. A complete search requires current authorized query_scan and all actual positive/negative source, Registry, contribution, extraction and authorization dependencies. A partial exploration cannot issue a complete ResultHandle/ActionEvidence or claim the current page is all results.

Missing selected source/Field/SearchContribution/provider/extraction data is evaluated after the ordinary disclosure gate and returns the existing unavailable/reset result. It never falls back to an empty contribution. Budget exhaustion fails the complete result; it does not return the first N rows as complete.

Snippets/highlights derive only from already-authorized semantic text. Their scalar offsets are ephemeral result-display offsets, not Locators. Opening a hit performs a fresh current resolution/read and resets when the result/source is stale.

## 8. Persistence, reopen, copy and import

New current saved searches use QuerySpec/2. The saved definition includes explicit scope, parameters, stable FieldIds, SearchContribution contributionId/version dependencies and author ordering. Shortcut text, parser cursor, source popover, recent history and device direction are interaction state only.

Reopen dispatches the recorded QuerySpec version before current qualification. QuerySpec/1 remains exact and is never reinterpreted as /2. Field deletion/type change, contribution removal/version change, permission changes and FileBinding changes follow their real Registry/currentness reset rules.

Definition Transfer maps typed Ref/DefinitionAddress slots under the version-specific Query schema; saved-definition copy/fork/import uses that D7/D3 author path and preserves the embedded QuerySpec version/bytes. D9 `query_json` is only the static export of a complete terminal result (`weftext.query-result-export/1`); it never contains QuerySpec author bytes and cannot create or migrate a saved Query. No filename/path string becomes a DefinitionAddress or identity.

## 9. Cross-surface and cross-device equivalence

For the same QuerySpec/2 bytes, parameters, scope and current dependency cut, File List, Quick Open, Global Search and a saved Query have identical Query membership/order semantics. Different surfaces may expose different preset controls, but a control that is available maps to the same source/AST rule above.

Keyboard, pointer, touch and assistive-technology operations invoke the same condition model. Focus binds logical condition/result identity rather than DOM position. IME preedit executes no Query. CJK/RTL input, bidi isolation, screen-reader labels, mobile sheets and hardware keyboard behavior remain D8 implementation obligations; this D7 design does not claim them tested.

## 10. SEARCH-01–08 acceptance

| ID | Current design obligation |
| --- | --- |
| SEARCH-01 | File List, Quick Open and Global Search use the preset scopes and empty-input behavior in §3. Recursive scope, Resources, Annotation, Trash, body and attachment/OCR are explicit. |
| SEARCH-02 | title/subtitle/Node filename/Node path/Resource filename/hierarchy are distinct; QuerySpec/2 supplies real producers; titleless remains titleless; rename/move resets through real dependencies without changing identity by text. |
| SEARCH-03 | exact versus nfc-for-compare follows §4; comparison is case-sensitive, Unicode-scalar and deterministic; no fuzzy/tokenizer/Pinyin/stemming claim. |
| SEARCH-04 | shortcut mode is explicit and the complete lexer/grammar is §5; ordinary title:, URL, drive colon and prose remain literal in plain mode; shortcut errors/incomplete drafts execute no fallback Query. |
| SEARCH-05 | visual and shortcut controls produce the same SearchConditionAst/1 and CanonicalGraph under §6; advanced conditions are retained; D8 platform interaction remains later evidence. |
| SEARCH-06 | permission precedes sensitive reads; hidden counts/ranks/snippets do not leak; index states differ from complete zero; missing sources/providers and budget have explicit failure; ordinary open/edit/save remains independent. |
| SEARCH-07 | save/reopen/copy/import persists the versioned canonical Query, not UI state; /1 and /2 dispatch separately; Field/contribution/FileBinding/permission changes requalify or reset; old result evidence never gains current authority. |
| SEARCH-08 | a hit opens through fresh current authorized resolution; snippet offsets are not Locators; identical QuerySpec/2 semantics stay identical across surfaces/devices subject only to explicitly unavailable capabilities. |

These are design obligations. Product Search, GUI/IME/AT, provider, performance and cross-device execution remain UNRUN.

## 11. QuerySpec/3: lifecycle search and saved contribution bindings

This section uniquely owns fresh authoring/execution, replacing the QuerySpec/2 compiler/save clauses above and the retained Algebra §§1–2 live-only scan/read only inside a QuerySpec/3 invocation. AUTHORING and its integration still own native controls/FieldSource. Genuine QuerySpec1/2 bytes, diagnostics and recovery keep exact decoders; no defaults are injected or read-time migration performed. Old queries remain live-only and cannot claim Trash search. This is a candidate; product tests remain UNRUN.

### 11.1 Closed author shape

```text
QuerySpec/3 = {
 format:"weftext.query",version:3,
 parameters:<original ParameterSpec array>,relations:<complete QuerySpec2 DAG>,
 scalars:<original ScalarSpec array>,result:<original result union>,
 lifecycleScope:"live"|"trash"|"live_and_trash",
 searchBindings:SearchBindingSet/1
}
SearchBindingSet/1 = {
 version:1,mode:"none"|"explicit"|"all",
 selections:[{contribution:<complete SearchContribution>,
  namespaceOwner:{ownerKind:"core"|"first_party"|"publisher"|"workspace_user",ownerId:text}}...]
}
```

All eight Query members are required and objects closed. Scope is authored, never overridden by parameters/caller/UI/environment. Ordinary new queries explicitly store live and `{version:1,mode:"none",selections:[]}`. none requires []; explicit requires 1..256 selections; all permits 0..256 and freezes the complete disclosable active SearchContribution set explicitly chosen at save time, never future automatic additions. Excess is budget_exceeded, not truncation. Original five-member contribution/version/path/role rules remain; selections sort uniquely by contributionId UTF-8, rejecting duplicate IDs even at different versions. D4's exact namespace-owner kind/ID must be proved for the contribution ID, not guessed from the Field namespace.

Bindings are semantic constraints, not another FieldDefinition or accepted contribution instance. Current execution reobtains D10 admission/trust/version/permissions. A nonempty binding must be statically proved consumed by this Query's reachable Field read, matching/rank expressions and exact textPath using the existing typed CEL AST; lack of proof is invalid_request, never an unused ambient package gate. Plain Field queries with none do not depend on SearchContribution. The compiler freezes explicitly selected descriptors; all freezes its then-complete set before producing the same DAG. Trash changes only lifecycleScope, not the shortcut grammar or executor.

CanonicalGraph3 is exactly `{version:3,lifecycleScope,searchBindings,graph:CanonicalGraph}` with canonical bindings and D3-CJ/3 values. SemanticStateKey and definition closure include the full object, not merely the old DAG. Operator ordinals/K/error ordering remain unchanged; historical CanonicalGraph1/2 are not reencoded.

### 11.2 Scope and read qualification

live selects live, trash selects trashed, live_and_trash selects both, never tombstoned/never-known/conflicting/placeholder/incomplete objects. Scope applies to this invocation's scan/read, including explicit parameter Refs, but does not override QueryRef callees. DefinitionAddress lookup retains its original authorized live-source/version gate; a hidden definition is not read to discover its scope.

workspace uses the current canonical inventory and appropriate live/Trash forests. entities deduplicates complete Refs and performs minimum disclosure before existence; unauthorized/missing produce no member. subtree follows the root's current canonical forest, never its old parent/path/restore membership. A root outside scope or undisclosable is not_visible; structure_state/root disclosure precede forest reads. A live root does not regain formerly deleted descendants; search both forests with workspace or explicit entities. headings select Node owners with full D2 projection. Resources/Annotations use their own lifecycle plus independent owner/relationship disclosure; equal owner never adds independently trashed members to restore membership.

Scope grants no permission. Order is trusted audience/Workspace → original entity_state/owner/state disclosure → structure or exact source_read/field_read/resource_read/annotation_read with deny and minimum closure → availability → read. The new explicitly read-only Trash adapter permits the same D6 current SourceObservation/FileObjectBinding/source-pin producer to observe authorized retained trashed source. It neither creates a managed version nor substitutes an old historical pin nor authorizes after reading. D2 body_text handles the positive needle case; native fields still use current FieldSource/Registry. Authored Ref/provenance values do not implicitly resolve targets. Lack of permission is not_visible; authorized missing source/proof is source_unavailable/original owner availability, never empty.

traverse/related/graph keep original live endpoint/edge qualification; a trashed origin failing that gate is not_visible and suspended edges do not revive. Ordinary live Query/Calendar/NodeCollection semantics remain. ActionEvidence, collection create/remove, bulk/all_result and writable postQuery require the entire call closure statically proven live; otherwise original action_not_applicable even if a final filter leaves live rows. Trash result grants no write. Restore/purge independently use original D3 fresh preparation/confirmation/owner proofs, never result-as-closure. Existing saved/unknown action recovery uses its recorded version.

### 11.3 Checkable coverage under the same query_scan key

Retain D6-CONTROL §3.4's key members, fifteen kinds/ranks, Proof3 and PinRef2. domain/selector remain original four domains and three selectors. The key does not encode live, while the old D7 producer covered only live. The following protected carrier distinguishes scope instead of trusting an equal key/stamp:

```text
D7QueryScanCoverage/1 = {
 kind:"d7_query_scan_coverage",version:1,
 key:<exact DependencyKey3.query_scan>,stamp:{epoch:Token,revision:Counter},
 baseFrontier:Frontier/2,lifecycleScope:"trash"|"live_and_trash",
 authorization:{key:<original authorization key>,stamp:{epoch:Token,revision:Counter}},
 inventoryEvidencePins:[PinRef/2...],
 members:[{ref:EntityRef,state:"live"|"trashed",lifecyclePin:PinRef/2}...]
}
```

Only trusted Core/D3/D6 producers create canonical D3-CJ/3 bytes in the existing protected PinRef2 record: payloadKind=portable_metadata, retentionClass=query_preview, internal payload-kind dispatch to this version1 decoder. No public pin member/tag/store is added. The internal portable_metadata decoder accepts only this exact root kind/version/closed object for the Query3 coverage consumer, never arbitrary JSON or widened old metadata. Core produces it only after Query authorization; its pin grants no public metadata read and is not a Resource ByteHandle. Lifecycle/member details remain protected under existing diagnostic disclosure. Each member consumes maxRows/work, each evidence pin maxDependencies, and actual bytes maxTemporaryBytes/maxResultBytes; checked budget precedes allocation and excess fails the whole Query. Retention never exceeds original finite result TTL/budget; query_preview release/restart follows original protected records, and author bytes store no pin. Ref/key domains and Workspace agree. Members uniquely sort by D3 RefKey (headings use Node owners), states belong to scope, and lifecyclePin proves same-cut birth/canonical/lifecycle/owner. Members contain only the complete authorized population; hidden inventory proof remains protected. Nonempty, sorted-unique inventoryEvidencePins must prove the exact potential range using real D3 current identity/forest/owner directories and continuity, including negative coverage; hit-only pins/gaps cannot produce Coverage. Full heading sources/D2 dependencies remain in original proof/sourceInputs.

Each non-live scan consumer must find one precisely matching Coverage pin in the same-key Proof3 entry.evidencePins: key/stamp/scope/baseFrontier/current authorization must all match. Multiple invocations/scopes may share one entry with one precise Coverage per scope. Strict decoding, length/digest, protected pin identity, auth key/stamp, complete inventory/lifecycle evidence and current continuity all pass; independently enumerate under the scope and compare every member Ref/state. Neither complete flags, counts/digests nor caller lists suffice. Missing/old live-only/wrong-scope evidence fails source_unavailable without complete result/ActionEvidence. Static entities=[] needs no scan/Coverage and proves no unrelated empty range.

Enabling the Coverage decoder or upgrading software does not create a continuity epoch, advance production H, or resign records. The producer retains valid original D6 key/stamp continuity at a real cut. Trash inventory/lifecycle/owner/forest negative ranges require the actual original D3 lifecycle, placement_range (applicable trash_roots/trash_children/subtree/owner directories) and authorization key/stamp/evidence entries in the same Proof3; inventoryEvidencePins must be real evidence pins of those exact entries and cover the full potential range. Coverage has no independent stamp and repeats its parent entry. Consumers revalidate every real dependency rather than treating an unchanged query_scan stamp as sufficient. Restore/purge/state transitions/independent members invalidate the affected real proofs; continuity gaps use the original D6 gate, never a client-created epoch. Query3/live and genuine Query1/2 keep original live proof rules without a Coverage/Trash-directory gate. Cache keys include scope/CanonicalGraph3, and identical bytes cannot substitute live-only proofs for non-live coverage.

### 11.4 Cold reopen and exact current D10 lookup

Author bytes store pure bindings, not activationGeneration/Catalog digest/Registry pins/Tokens/Runs. Reopen/inline/QueryRef first perform current request/definition disclosure, strict embedded-version decode and static graph/type checks. At the same AuthorizedCut, D10-CONTROL §4's current complete Catalog looks up every D7 contributionId and proves active exact version/full five-member descriptor, saved D4 namespace owner tuple, original immutable-asset digest/trust and Registry textPath qualification. D10 package-local ID/contractVersion are different domains; no same-Field/name substitute is accepted.

explicit checks only selected items; all additionally requires the current complete disclosable SearchContribution set to equal saved selections. Changes do not recompile DAG or change author bytes; the user must explicitly save a new definition revision. C2 cannot replace disabled/removed/upgraded C/v1 with the same Field/path. After disclosure, missing/version/semantic mismatch is definition_unavailable; permission failure precedes it as not_visible; authorized bad source is source_unavailable. No new error/fallback. Closure errors retain original phase/ordinal order; preexecution binding errors are deterministic by invocation path and sorted contribution ID, without public hidden ID/path/count. Definitions never lend owner rights.

Runtime dependencies retain current D10 activationGeneration+capabilityCatalogDigest+RegistryBinding and selected descriptor/owner trust evidence in the original ResultHandle closure. Catalog/auth/Registry changes reset old results; cold reopen requalifies. An unrelated connector/package change cannot make a fresh explicit saved search definition_unavailable; it may conservatively reset old runtime without altering saved semantics. none adds no D10 gate. UI hiding is not deactivation.

### 11.5 Actual producer/consumer slots and history

| Owner/slot | Current exact dispatch |
| --- | --- |
| D7-SCHEMAS / author decoder, retained Query/View/Action §4 | Eight-member QuerySpec3; exact old1/2 decoders. Core derives query.lifecycle-scope.v1 only for non-live and query.search-bindings.v1 only for non-none, never authored requires. |
| d7_query_execute.call.query.inline.spec; SavedQueryDefinition1.query; DynamicBlock1.query.inline.spec; relation/scalar QueryRef's decoded query | Explicit embedded query.version 1/2/3 dispatcher; wire2/wrapper1 other members unchanged. Recorded recovery never rewrites the embedded version. |
| Search compiler/D8 search save and builder | New QuerySpec3 scope/bindings; preserve advanced Source outside basic UI; original D2/D3/D6 save, no persisted AST/shortcut text. |
| D6 query_scan / Proof3.entries[].evidencePins / ResultHandle/cache | Non-live requires exact Coverage1 pin above; true same-cut source/lifecycle/structure/auth proofs; audience/expiry/epoch/reset gates remain. |
| D3/D6 read-only trashed-source producer / D7 read | Explicit read-only Observation/pin qualification in §11.2; no source_write or restore/purge extension, ordinary live source-save unaffected. |
| D10-CONTROL §4 Search descriptor lookup | Exact current D7 ID/version/descriptor/namespace-owner qualification/dependency; D10D7SearchDescriptor1 and ContributionKind unchanged, saved binding is not activation. |
| DefinitionTransfer / Result Q / copy/fork/import | Strict embedded-version decode; original typed Ref/DefinitionAddress slots mapped; scope/plain descriptor preserved. IDs/version/Field/path/role are not guessed Refs. Core/first_party/publisher owner and contribution revalidate at target Registry/Catalog. workspace_user.ownerId must equal target WorkspaceId; incompatible cross-Workspace binding rejects definition_unavailable, never silently rewrites; explicit new definition editing is separate. No-Ref payload still has slots=[] transfer/Q. |
| D5 collections/Actions; D9 result export; D10 Query calls | Writes require whole live closure. D9 query_json remains complete terminal-result-only. All use the same Query, no Search executor. |

Existing query-spec/search-contribution concepts remain sufficient; SearchBindingSet1 and D7QueryScanCoverage1 are their closed D7 carriers, not persistent entities. No old concept ID or historical 34/13 Registry count is replaced by a global acceptance claim. Wrapper1 is an explicitly embedded-version-dispatched carrier, never an old recorded decoder ignoring unknown members.

### 11.6 Executable example and refusal/recovery oracles

```json
{"format":"weftext.query","version":3,"parameters":[],"lifecycleScope":"trash","searchBindings":{"version":1,"mode":"none","selections":[]},"relations":[{"id":"n","op":"scan","domain":"nodes","selector":{"kind":"workspace"},"as":"node"},{"id":"b","op":"read","input":"n","from":"row.node","bindings":[{"name":"body","source":{"kind":"body_text"}}]},{"id":"f","op":"filter","input":"b","predicate":"contains(row.body, 'needle')"},{"id":"p","op":"project","input":"f","fields":[{"name":"node","expr":"row.node"}]}],"scalars":[],"result":{"kind":"rows","relation":"p"}}
```

For explicit C/v1, replace searchBindings with `{version:1,mode:"explicit",selections:[{contribution:{contributionId:"people/search-name",version:1,fieldId:"people/name",textPath:["text"],role:"name"},namespaceOwner:{ownerKind:"first_party",ownerId:"weftext.people"}}]}`, read people/name values as names, and filter with `row.names.exists(v, contains(v.text, 'needle'))` under the accepted same-cut object.text schema. This member illustration is not another wire: the compiler produces a complete eight-member Query for original save/execute.

| Case | Positive and refusal/recovery oracle |
| --- | --- |
| SR-01 | Authorized retained Trash T matches needle; live does not. Tombstone/placeholder excluded without historical-pin fallback. |
| SR-02 | State/content denial precedes reading; Field-only never obtains body; hidden T adds no public count/rank/snippet/Coverage member. |
| SR-03 | Same-key old live-only proof, missing/wrong scope/auth/key/stamp/cut or negative inventory fail complete admission. Proven empty coverage succeeds; partial index is not empty. |
| SR-04 | Restore/purge/replacement/auth change resets old result/coverage. Reauthorization/byte ABA only supports new invocation, not old pages/actions. |
| SR-05 | subtree uses actual forest, not original parent; entities[] needs no scan; independent Trash resources/annotations stay distinct on owner restore. |
| SR-06 | Save C/v1 → clear I/D → reopen/QueryRef/DynamicBlock keeps full bindings. C2 cannot replace C; disabled/removed/v2/owner change while Field survives gives definition_unavailable. |
| SR-07 | Unrelated connector may reset old runtime but fresh explicit C/v1 works. none needs no Catalog. New all-set contributor never silently changes author semantics. |
| SR-08 | Same-Workspace and legal cross-Workspace first-party transfer preserve scope/bindings plus typed Ref mapping. Missing C/incompatible user owner rejects atomically, no dropped condition; empty slots still Q. |
| SR-09 | QueryRef callees retain their scope; caller live read rejects imported Trash Ref. Non-live closure all_result/collection writes reject; restore/purge independently prepare. |
| SR-10 | Old1/2 with new members, missing3 members, unknown enums/members or duplicate bindings reject. Old saved/planned/unknown bytes stay exact; query_json is not an author carrier. |

All are design oracles; product/UI/Catalog/cache/reopen/transfer/Trash/permissions remain UNRUN. This section alone closes no global finding.
