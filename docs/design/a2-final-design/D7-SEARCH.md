---
source_language: zh-CN
translation_of: D7-SEARCH.zh-CN.md
translation_status: synced
---

[简体中文](D7-SEARCH.zh-CN.md)
# A2 D7 Search Semantics and SEARCH-01–08

Status: normative D7 search design inside the current author candidate. This revision repairs A2-D7-2F89-P1-02 and A2-D7-2F89-P2-01; independent review is still required. D8 interaction implementation remains a later full-module batch.

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
- A bare token is a nonempty sequence of Unicode scalars other than unescaped space/tab/quote/parenthesis. Backslash is literal unless followed by one of backslash, quote, parenthesis, @, colon, space or tab; those pairs decode to the escaped scalar. A trailing ordinary backslash is therefore a literal, while an unfinished reserved escape is an incomplete draft.
- quoted-value starts and ends with a double quote. Inside it, backslash escapes double quote or backslash; every other scalar, including whitespace, colon, @, CJK and RTL text, is literal. A missing closing quote is incomplete.
- unescaped uppercase ASCII AND, OR and NOT are keywords only as complete lexer tokens. Lowercase forms are literals. To search the uppercase words literally, quote them.
- a token beginning with an unescaped @ followed by a recognized operator head is parsed as a source term. An unknown @name: operator is unknown_shortcut_field, not a literal fallback. Escaping the leading @ makes the whole token literal.
- field-id uses the exact D4 FieldId grammar. member-path is 1..8 dot-separated lowerCamel ASCII ObjectMemberSpec names, each 1..64 bytes. It is validated against the same current Registry/Field TypeSpec used by Query. Labels and localized names never substitute.

OR has the lowest precedence, then AND/adjacency, then recursive NOT, then primary/parentheses. NOT NOT A is therefore legal and remains two explicit not nodes. A AND B AND C and adjacency A B C are legal chains.

An unmatched quote/parenthesis, missing operand, missing field value, incomplete @field(...), unfinished reserved escape or other unfinished token is a draft-incomplete state and executes no Query. On explicit submit the same state returns invalid_request with the exact source span. A syntactically complete unknown field/operator/value returns its specific parse/compile error. No error path executes an alternate literal Query.

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

D7-SEARCH-FIXTURES.json is the machine oracle for parser/visual equivalence. For every positive fixture, shortcut AST and visual AST must be byte-equal after canonical AST serialization and must compile to byte-equal CanonicalGraph descriptions. Negative/incomplete fixtures compile nothing.

## 7. Permission, index state, count, ranking and snippets

Authorization and minimum disclosure precede every sensitive source, Field, contribution, attachment, Annotation or index-private read. Hidden objects cannot leak through hit counts, rank gaps, snippets, completions or unavailable reasons.

Derived Index remains only a candidate accelerator. building, partial, stale and unavailable are distinct from a proved complete zero-result Query. A complete search requires current authorized query_scan and all actual positive/negative source, Registry, contribution, extraction and authorization dependencies. A partial exploration cannot issue a complete ResultHandle/ActionEvidence or claim the current page is all results.

Missing selected source/Field/SearchContribution/provider/extraction data is evaluated after the ordinary disclosure gate and returns the existing unavailable/reset result. It never falls back to an empty contribution. Budget exhaustion fails the complete result; it does not return the first N rows as complete.

Snippets/highlights derive only from already-authorized semantic text. Their scalar offsets are ephemeral result-display offsets, not Locators. Opening a hit performs a fresh current resolution/read and resets when the result/source is stale.

## 8. Persistence, reopen, copy and import

New current saved searches use QuerySpec/2. The saved definition includes explicit scope, parameters, stable FieldIds, SearchContribution contributionId/version dependencies and author ordering. Shortcut text, parser cursor, source popover, recent history and device direction are interaction state only.

Reopen dispatches the recorded QuerySpec version before current qualification. QuerySpec/1 remains exact and is never reinterpreted as /2. Field deletion/type change, contribution removal/version change, permission changes and FileBinding changes follow their real Registry/currentness reset rules.

Definition Transfer maps typed Ref/DefinitionAddress slots under the version-specific Query schema. D9 exact query_json copy/export/import preserves the recorded version and bytes. No filename/path string becomes a DefinitionAddress or identity.

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
