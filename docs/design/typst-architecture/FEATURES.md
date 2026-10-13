---
source_language: zh-CN
translation_of: FEATURES.zh-CN.md
translation_status: synced
---

[简体中文](FEATURES.zh-CN.md)

# New Features candidate: links, queries, navigation, annotations and output

Status: coordinated author candidate awaiting independent review of a fixed commit. This document implements the link, Query, navigation, domain-type, annotation and output requirements in [REQUIREMENTS](../typst-native/REQUIREMENTS.md), preserving Agent, automation, external-effect and cost boundaries. Shared cuts, frozen pages, schema baselines, Calendar precedence, annotation projections, citations and Html/Paged consumers are coordinated with the other owners; [REVIEW](REVIEW.md) records repairs and bounded evidence. Design acceptance is separate from product acceptance; implementation is not claimed complete.

## 1. A few shared objects and ownership

Author facts remain one `.typ` source and independent JSON annotations. Link indexes, Query results, thumbnails, inverse relations, presentation preferences and export artifacts are projections. SOURCE owns native values and centralized schema; EDITOR owns Drafts, source mapping and constructs that can be edited safely; CORE owns identity, read qualification, logical transactions and recovery. Features creates no second field database, write entry point or authorization engine.

The shared objects are `ReadBasis`, `RevisionId`, `Plan`, `OperationJournal`, `Receipt` and `CommitManifest`. A RevisionId is a private Core allocation during planning and becomes valid only at commit; a string or equal bytes proves no permission. ReadBasis binds the actual observer incarnation, file generation, schema, authorization and consumed dependencies. A new read of equal bytes cannot revive an old write qualification. `SourceSnapshot` denotes an authorized exact source snapshot; `FieldProjection` discloses only permitted fields. Neither upgrades to the other. Core can privately locate a field and apply a narrow source patch; constructing that patch does not justify returning whole-source bytes to a field-only caller.

The logical calls below consume [CORE](CORE.md), rather than defining another wire family. [SOURCE](SOURCE.md) owns values/schema, [EDITOR](EDITOR.md) owns Draft/native writing, and [REVIEW](REVIEW.md) states evidence limits:

```text
readEntity(ref, current|revision, purpose) / readScope(scope, purpose, completeness)
  -> SourceSnapshot | FieldProjection | unavailable
compile(sourceSnapshot, environment, target, budget) -> CompileSnapshot
evaluateQuery(definitionSelection, parameters, scope, budget, context) -> QueryResult
readResultPage(handle, cursor) -> Page
prepare(opId, intent, draftBase, budget) -> Plan{preview, changes, losses, dependencies}
commit(opId, planHandle) -> Receipt | conflict | rejected | unknown
operationStatus(opId) -> unknown | in-progress | pending_reconciliation
                         | committed | conflict | failed
createJob / jobStatus / stopJob / inspect / confirm / download
```

prepare freezes real before images and all changes. commit rechecks current qualification and makes one logical transaction. A transaction can change source, resources and annotations, but covers only one commit authority with the required guarantees; independent servers do not gain fictitious cross-server atomicity. Core journals recover crashes and physical installation; projections expose only decided logical state. unknown retains the original opId, Draft and plan and queries operationStatus instead of creating again or sending another external request. An operation waiting for user input must not hold a long whole-library lock against unrelated notes.

## 2. Native links and four locator domains

Body links use standard Typst `link`, labels and refs. Clear application URI names are proposed; syntax need not reuse old decoders:

```typst
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#plan")[项目计划]
#link("../资料/白云山/白云山.typ#route")[白云山]
#link("weftext-workspace://58c47256-bfce-4d29-a037-b83a3a644fb0/资料/白云山/白云山.typ#route")[白云山]
#link("file:///D:/notes/report.pdf")[报告]
```

Chinese URI text above is a readable form; generated canonical URI bytes use UTF-8 percent encoding. In-document `#link(<plan>)[View]` and `@plan` retain official semantics, including ref object/numbering requirements; neither claims cross-note identity.

| Form | Basis and guarantee |
| --- | --- |
| ID (default for creation) | Full library ID + note ID. Rename/move retains the same object; deletion never retargets to a new namesake. |
| Source-relative path | Based on the actual authored source file's directory; links inside an include use the included file. |
| Library path | Identifies the library and its root-relative path. Moving the source does not change it; moving the target may break it. |
| Local absolute path | A verified platform file URI, with a current-local-machine guarantee only. Server does not open arbitrary disk access through it. |

“Shortest valid relative path” is a computation within the source-relative domain, not a fifth protocol or title lookup. Find the longest common inventory-component prefix of source directory S and target T; emit `../` once per remaining source directory level, then the remaining target components, retaining the fragment. For the same source and target file use the exact basename. Where `.`, `..`, symlinks or case rules admit multiple interpretations, emit only a canonical form that the current Core resolver round-trips to the exact target. “The last two components are currently unique” is not path computation. Different volumes, unconnected libraries, undisclosable paths and boundary escapes yield another available form such as ID plus an accurate reason.

URI components use one UTF-8 encode/decode pass: slash is a separator, `+` is literal, `%252F` decodes once to literal `%2F`. No double decode, Unicode normalization or title-based case folding. Reject malformed escapes, control characters, encoded separators, out-of-library parents and unknown authorities. A platform library generates and rereads file URIs; do not concatenate UNC/drive syntax. Encode the resulting string through official Typst string construction while preserving the display content AST. Invalid application addresses still retain valid source and a repair entry point.

## 3. Conversion, copying and link indexes

For one current occurrence provide Convert form, Copy target address, Copy complete link and Shortest valid relative path. Literal source copying does not require the target to exist. Resolved copying requires permission to disclose the target address but no source write permission. Writing conversion requires writable source and only relevant target read/location permission.

Conversion obtains current Draft serial and ReadBasis, finds a unique authored occurrence through official AST/EDITOR mapping, resolves the exact current target and fragment, generates the requested form, rereads it through the official parser and the same Core resolver, compares identity/fragment/display content/native parameters/all unselected bytes, then produces one Draft operation/Undo and prepare/commit. A fragment must exist uniquely in exact target semantics; unavailable or duplicate labels cannot be silently discarded. Undisclosable paths cannot become location text. Rename, replacement, revocation, environment changes or later Draft edits invalidate the qualification; retain Draft and reprepare. Include/generated/shared-variable links without a unique writable argument get Source entry, not a location inferred from equal text.

Enumeration retains two origins to avoid merging repeated compiled occurrences:

```text
AuthoredLink = {sourceOwner, ReadBasis, astPath, byteRange, destinationSlot?}
RenderedLink = {compileIdentity, target, projectionGeneration, occurrenceOrdinal,
                fragments[], destination, origin: unique_source | generated | multiple}
LinkResolution = resolved(target, fragment?) | missing | fragment_missing
                 | ambiguous | unavailable
```

Official AST enumerates AuthoredLink independently of display target. Default flow RenderedLinks come from one Html compile's final tree: one logical link with its fragments, and repeated includes remain separate even with equal spans/text. Print/PDF/explicit Paged outputs consume that Paged compile's final data; equal source RevisionIds do not permit mixing targets. Each compile's origin producer must prove association, not merely Introspector/AST/span/string evidence. Generated links may navigate/export without counterfeit writers/stable asserted edges. Real Html/Paged producers still require EDITOR independent acceptance; earlier realization-trace assumptions prove no implementation.

Outgoing links expose real authored and compiled links. Incoming links provide separate ID and “currently resolved path” groups, both navigable. Typed property refs appear as Property references, without fake body links. The path index is derived within current scope and invalidates on source/target move, replacement, permission or directory changes. Potential mentions are separate and form no actual-link or integrity edge. partial never means there are no incoming links in the whole library. Complete zero is possible only for the current authorized complete scope, without hidden titles, paths or counts. There is no automatic orphan deletion or default library-wide path rewrite.

### 3.1 Unified compilation-target consumption

`CompileSnapshot` states Html/Paged target/environment/exact source-cut-World dependencies/final tree or actual output/that compile's origin map. Default flow body/tables/text copying/visibleExcerpt/potential matches consume one Html final tree. Explicit print selection/excerpts consume their Paged final output, without another compilation's edit map. Native `target()` may change body/numbers/headings/links, and equal source versions do not remove this difference.

Official Introspector native headings are a separate product. Query can read all native headings from that target's same compile, including generated/deep items, with `displayAssociation: displayed(current final-tree occurrence)|not_displayed|unavailable`. A heading without association remains read-only queryable, not displayed Html body/a writer. Displayed Html headings cannot be guessed to native items by equal text. Body search explicitly consumes displayed Html body rather than silently switching to Paged or mixing undisplayed introspection.

An official local Paged renderer inside an Html frame is a local boundary of that Html compile, with that compile's output/origin; it is not a separate whole-document Paged compile mixed into Html. Unmapped frames stay read-only/explicitly limited, without coordinate-derived root writers.

Exact-source copy/fork identity/typed links remain official authored AST/schema operations independent of display target. Displayed-text/image previews/copies/exports name and freeze the consumed target. New Html style/bridge experiments are unaccepted; report actual capabilities rather than treating Same snapshot as producer proof.

## 4. Complete potential matching and overlap algorithm

Terms come from currently disclosable note titles and schema-designated alias/name search fields, excluding the current note. Former personal names can derive candidates from name roles without another alias fact. Deduplicate the dictionary by `(target identity, exact term)`; retain all objects with equal titles. Default matching is exact Unicode `scalar` literal matching, without implicit Pinyin, fuzzy matching, case folding or normalization. Additional modes require explicit choice and a stated reason.

Separate discovery from writing: accurately extracted ordinary visible logical text may discover candidates; shortcut conversion requires a unique source range and current writable mapping. Valid generated/include content may offer read-only discovery marked with its origin. Code, math, URLs, real links, attribute declarations and annotation bodies are excluded from automatic ordinary-text scanning. Do not concatenate separate logical fragments. Soft visual wraps remain the same fragment; EDITOR explicitly maps whether source line breaks belong to the same logical flow.

For n scanned scalars and z matches:

1. Build an Aho–Corasick trie from one authorized dictionary snapshot, keeping all target identities at terminal nodes. Scan each logical fragment and return all matches in `O(n + z)`. Budget exhaustion records partial per fragment; never keep only the longest match.
2. A hit is `(flow, startScalar, endScalar, targets[])`. Group only identical ranges; targets use stable full-identity order, not title merging. Record matched terms and origins. Containment/crossing are not deduplication rules.
3. Compute interval overlap separately. Only `a.start < b.end && b.start < a.end` overlaps; touching endpoints may share a lane. Retain every semantic hit.
4. A candidate visual design uses at most two separated underline lanes. Greedily assign the lowest free lane in `(start asc,end desc)` order; groups without a lane enter their connected overlap component's More matches entry. Two lanes, default-off and 20 entries per page are replaceable design choices. The requirement is complete access. Keyboard/touch/AT lists enumerate all ranges independently of visual lanes; stable cursor pages never silently truncate.
5. Hover/click/keyboard/long press open the same list. Escape restores the original selection; background scanning does not steal IME/body focus. Only choosing a target and Convert writes. Never default to the first target.
6. Reauthorize and recheck the dictionary entry and current Draft. Both endpoints of the scalar-to-exact-byte mapping must be whole grapheme boundaries, without splitting combining/ZWJ/CRLF. Partial-grapheme hits remain read-only discovery; do not expand selection. Generate a native ID link via §3 and compare target/display text/outside bytes as one Undo.

```text
Text: 白云山南路
[0,3) 白云山 -> A,B (two targets at one range)
[2,3) 山     -> C   (contained)
[2,5) 山南路 -> D   (crossing)
Greedy visuals: 白云山 lane0, 山南路 lane1, 山 overflow.
Complete list: three ranges, four targets, each independently selectable.
```

RTL drawing uses logical ranges and current layout mapping, not x-coordinate semantic order. Disabling hints does not change source. Scanning may prioritize the visible area and cancel. Revocation removes the target's candidates and displayed context. Completeness describes the authorized `scan` only, without a hidden-term count. An unfinished library scan does not block one independently verified local conversion.

## 5. One Query semantics with explicit states

Weftext Query accesses authorized library data; official Typst `query` remains current-compilation introspection. Plain search, navigation conditions, visual filters, saved queries and APIs produce the same typed Query AST for one evaluator. Persist structure with native dictionaries/arrays and choose a statically restricted, deterministic, side-effect-free CEL syntax profile using standard `exists/all/filter/map` comprehensions. Do not inherit old DAG wire, eight required members or contribution version trees. The CEL parser handles expression strings; the official Typst parser handles native carriers. No eval/import/arbitrary callback/network. Statically type-check and preserve advanced expressions; if the builder cannot represent them, expose a read-only advanced condition/Source rather than deleting it.

Base nodes are `scan(scope,lifecycle)`, `read(fields/products)`, `filter(expr)`, `project(columns)`, `unnest(array,binding)`, `sort(keys)`, `take(limit)`, `union_all`, `distinct(keys)`, `group(keys,aggregates)`, `related(relation)` and saved-query calls. Default displayed body/title/tables use that Html final tree; native headings/display associations are separate as §3.1. Generated/deep heading existence proves neither display nor writing. Explicit Paged products use a named target, with compileIdentity/target on each run product rather than cross-target values/occurrences. Plain full-text search creates explicit `textMatch` nodes; UI shows body inclusion/directory/descendants/library scope/conditions, without another Query DSL.

Query uses SOURCE's single Type/Value: text, bool, i64 int, exact decimal, explicit float, civil date/time/local datetime, offset-bearing instant, ref, array, record and schema-discriminated union. Money+currency, quantity+unit and precision dates are business records in that Shape system, not unregistered scalar codecs. Arrays preserve order and duplicates. Schema interprets record members; unknown native members remain read-only values without inferred types. No implicit float/decimal, date/instant, currency or unit conversion. Explicit conversion chooses an exact provider/version recorded as a dependency. Phones remain text. Schema conflicts/missing schema never infer a type from the first result row.

```text
Cell<T> = value(T) | absent | null | invalid(diagnostics) | unavailable(reason)
```

QueryResult `has` only the definition in §5.2, not another handle/schema variant here. total/aggregate belongs to that frozen terminal, without recomputation in page transport.

absent means not authored; null means schema-permitted explicit none; invalid means an actually read value violates its definition; unavailable means schema/product/required dependencies cannot be obtained. Public reasons must not disclose hidden-object existence; undisclosable objects are excluded from public scope. Empty strings/arrays and zero differ from these states. Predicates explicitly test `isAbsent/isNull/isInvalid/isUnavailable`; taking a member from a non-value produces a type/value error. Unknown/unavailable must not silently filter as false. If unavailable fields prevent deciding membership, mark partial with confirmed rows and undecided reasons. An invalid overall query returns error, never complete empty. FieldProjection consumes authorized declarations/schema under SOURCE's verified independent profile, without compiling hidden body/executing hidden conditions/disclosing hidden diagnostics. A field needing hidden computation is uniformly unavailable before execution, rather than running and exposing secret-dependent outcomes. SOURCE/Core profile specifies actual deployment capability, not a control or reader's own assertion.

Coverage and freshness are orthogonal: a complete old projection may be stale; partial current rows may still be browsed. A truncated page is not partial: it has a continuation cursor. `take(200)` is an explicit author limit, not a whole-library statistic. Only complete + current + zero rows displays complete zero for that scope. A partial count is a known lower bound; aggregates over undecided input are partial/estimated, never exact totals. Never graph a page as whole-library proportions. Authorization changes invalidate run/cursor, remove protected cached cells/snippets/counts and reexecute after authorization. Other dependency changes may display readable, explicitly stale results without writing them.

Row lineage explains origin but grants no writes. Direct-field rows may propose precise intents, and prepare reacquires current ReadBasis and narrow write permission. group/distinct/computed values do not inherit a representative writer. Results from explicitly connected authorized libraries may union without implicit cross-domain reads or cross-library transaction guarantees. Trash is an explicit authorized read-only lifecycle scope, retaining tombstone/unavailability diagnostics. Results grant no restore/purge. Every view action whose call chain includes trash is read-only.

### 5.1 Core LogicalCut: results are not assembled from per-object current reads

Counterexample: one Core transaction changes `A.amount=10,B.amount=0` to `A.amount=0,B.amount=10`. Reading old A and new B yields sum20, a state that never existed. Separate ReadBases, or proving both values were once read, does not fix this. Query is not a loop over `readEntity(current)`.

CORE produces `LogicalCut` with one authority's decided journal generation, complete causal heads, entity→exact revision/blob/lifecycle mapping, frozen structural facts, schema/rules/settings and authorization audience. One short commit/read barrier captures that decided view and registers cutHandle with retained historical blobs/views in one protected step. Release only after registration; never capture half-installed transactions. This is an invocation's historical snapshot, not independently writable current body/tree authority.

Evaluation:

1. Authorize the caller, resolve this scope/definition selection and request a Core cut. Current definition revision/current schema/invocation scope freeze at the same cut. A pinned definition is separately fixed historically while data uses this cut. All nested saved-query calls share the relevant authority cut; current callee definitions read there rather than acquiring new cuts recursively. Statically reject definition-call cycles and freeze parameters/defaults/explicit now-timeZone context once. Cross-authority calls explicitly extend a federated cut vector under call/expansion budgets.
2. scan enumerates only exact cut inventory/structure/lifecycle and complete positive/negative ranges. An index must prove its membership in this cut; missing coverage uses cut blobs or partial state, never an old index's negative range as absence proof.
3. `readEntity(ref,at=cut,purpose)` reads retained cut blobs, not latest working files. Fields/body/official compilation dependencies use this cut. Managed includes/resources/styles/config in World also resolve at the cut, without later latest assets entering native function results.
4. Lock exact immutable versions of external packages/fixed provider data/other non-workspace inputs. Negative file/member dependencies come from cut inventory or the frozen input set, not ambient filesystem checks during evaluation.
5. Reach the terminal and seal data/order/completeness, then call CORE `publishCurrentResult` rather than letting a worker announce current. Within one publication-barrier protected registration transaction, atomically validate consumed positive/negative dependencies, complete scope, schema/settings/current authorization/observer continuity and register original run/retention/change subscription continuously from capture cut. Only unchanged relevant dependencies yield `current_at(publicationCut)`; unrelated global generation changes need not reject. Validation and registration have no gap, not a validate call followed by separate handle creation. Changes without revocation may yield sealed complete/stale or bounded fresh-cut/new-run evaluation. Evaluation completion itself does not grant current. Authorization change prevents original-result publication; watcher gaps/unknown raw external changes forbid current. Post-publication changes stale through the same subscription, and authorization invalidates the whole handle without changing frozen rows. Equal bytes cannot manufacture qualification/cuts.

The example reads only old10+0 or new0+10, both sum10. Retained blobs avoid holding a whole-library write lock; new structural transactions may commit. Missing retained views return cut_unavailable or explicit partial, never per-object current fallback labeled coherent complete.

Current linearizes at the publication transaction's validation/registration decision and means relevant facts remain applicable under this mode's provable Core-admitted state. It does not promise instantaneous raw external-writer quiescence; external_pending/per-file limits remain visible. Budget/cancel/IO failure cannot yield complete. A valid sealed partial may pass the same publication gate as partial/current or partial/stale, without disguising mixed-transaction data as partial. Publication conflict retains sealed result as stale or reevaluates, never changes one row under the old run.

Context has closed explicit now(frozen Instant or null)/timeZone(explicit zone and rule version or null). Selecting workspace default zone resolves actual zone/rules from this cut's settings. Selecting Current time explicitly asks the Core/Server clock provider to generate now once for all nested callees, without repeated host-clock reads. Client now can supply Query meaning, not authorization expiry/approval/audit evidence. Result binds input/generated context selections; civil dates never gain implicit device zones.

Union across independent authorities carries `logicalCuts:[...]` and consistency `federated`: coherent within each authority, with no single instant promised between authorities. A global aggregate may explicitly describe this fixed tuple of cuts, including its lack of a single global instant in UI/export. Queries requiring single_authority or cross-domain write conditions reject rather than treating one runId as cross-server atomicity.

External edits stay separate. Unadmitted ordinary-file observations do not enter business LogicalCut. Core records external_pending/incomplete/conflict and marks affected results stale. Files remain actual current source; old cut blobs serve explicit historical results, never pretend current working files. `observed_only` browsing can show per-object exact external observations/coverage, labeled Per-file observations. It has no whole-library instant and grants no strong aggregate/negative-range/absence/all-members bulk/post-query condition. Strong evaluation first admits/merges relevant observations into normal decisions without losing external Drafts or last-arrival overwrite. Unknown responses recover the original cut/run or report expiry, never replace it with a namesake cut.

### 5.2 Frozen results and pages, not latest-reading cursors

`FrozenResult` is an immutable terminal schema/data/order/occurrence-key/coverage/cut/definition/parameter/provider dependency set. Controlled memory or read-only spool/chunks may hold it; these are disposable projections, not facts. Publish complete handles only after complete terminal evaluation and sealed indexing/order. A partial handle seals one confirmed subset and gaps; later completion does not append rows to that handle.

```text
QueryResult = {
  audienceHandle, runId, definitionSelection, schemaBinding, logicalCuts,
  inputDependencies, invocationContext, terminalSchema, frozenResultRef, publicationBasis,
  coverage: complete | partial(reasons), consistency: single_authority | federated,
  freshness: current_at(publicationCut) | stale(reasons), expiresAt
}
Page = {runId, frozenResultIdentity, startOrdinal, rows, nextCursor?}
Cursor = opaque Core record bound to
         {runId,logicalCuts,frozenResultIdentity,orderDigest,nextOrdinal,audience,expiry}
```

These specify semantic objects, not frozen transport wire. Core issues handles/cursors through protected records; caller run/ordinal claims are not permission. A page first checks current audience/all consumed authorization dependencies, then original run/cut/result retention/cursor, then reads `[nextOrdinal,nextOrdinal+pageSize)` from the frozen sequence. Repeating the cursor returns identical rows/ordinals/nextCursor. Page size cannot change membership/order/statistics. Never rerun the query/read latest for a page; one identity always refers to its original result.

Explicit sort uses SOURCE Type/Value comparison and stated missing/null order, then full run-local source/occurrence key to break ties. Without sort retain producer order: scan by complete entity key at the cut, unnest by parent order+item ordinal, union_all by declared input order, group/distinct by complete canonical typed equality keys with new row identity and no inherited writer. Hash collisions compare full values. Global sort/aggregate consumes complete logical input under budget; spool/budget failure cannot issue a fabricated complete first page. A partial aggregate's known subtotal is not a whole result, and signed subtotals are not necessarily lower bounds. An undefined estimate algorithm cannot gain an estimated label. Charts/exports consume FrozenResult, not browser pages. Late row summaries/thumbnails use that row's same-cut source and separately authorized resources; opening a note may read current, but shows its newer revision/list staleness instead of pretending a new thumbnail shares the old row basis.

Relevant authorization changes invalidate the entire handle/cursor and stop delivery. Never remove hidden rows and continue old ordinals that reveal counts/gaps. Reevaluation creates a new run/cut/result explicitly replacing the UI result, without merging old page1/new page2. Index completion/source changes/subscription updates likewise publish replacement handles. Original partial results retain their subset rather than changing coverage to complete without resealing. Expired/unavailable cuts return status; explicit refresh creates a new run without transparent content replacement. Actions use result targets as explanation only, freshly qualify basis/dependencies at prepare, freeze targets and do not rerun Query at commit to expand them.

### 5.3 Persistent schema parsing baseline for saved definitions

Short names such as `phones`/`person` plus CEL text do not persist Field/Type identity. A saved definition has required native member:

```typst
binding: (
  schema: "weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema",
  revision: "8a81e35f-ad48-4c12-af4f-3d2607102fa9",
  language: "weftext.query.cel.v1",
)
```

Example refs/revision show shape only; a positive fixture must use Core workspace initialization and its actual committed schema revision. SchemaRef is the single central schema's roleRef, internally `{workspaceId,kind:"schema"}`, natively `weftext-config://<lowercase-WorkspaceId>/schema`. No new random schema identity or string-created role. The schema baseline contains aliases, FieldId→author key/shape, Types, Relations and provider/function signatures rather than copying those definitions into Query. Persist only author expression text/native structure and one exact baseline selection; AST/name→ID maps/typed plans are rebuildable rather than independent AST facts.

Production: the builder projects a declaration officially under authorized current schema, then parses/type-checks CEL. Its static symbol table separates reserved products (self/title/body/headings), parameters, comprehension locals and fields. Lexical locals bind within scope; `p.number` resolves from p's bound record shape, not a library-wide namesake Field. Unqualified keys must be unique in baseline and not conflict with reserved names. Disambiguation uses `field("<permanent FieldId>")`, requiring a literal existing in baseline and producing a typed field-reference node at parse time rather than runtime name lookup. Ordinary UI can keep short names without manual permanent IDs. No dynamic FieldId/function-name construction. Type short names/related RelationIds/provider functions resolve through baseline too. Core verifies the reference closure against installed schema and saves binding+metadata in one source Plan. Schema change before commit requires reprepare rather than saving a caller's uninstalled schema string.

Cold reopening authorizes definition and exact schema revision, selects exact CEL profile/parser, reparses all structure/expressions, derives unique semantic IDs/Type/Value signatures through baseline, checks symbols/parameters/functions/relations, obtains current schema from this LogicalCut, joins by permanent ID and uses SOURCE compatibility rules on every consumed definition. The closure includes where/select/order/type-field reads/parameter types-defaults/view columns-expressions/export typed input/related RelationIds/definition-call arguments-constants. Callees use their own baseline under the shared data cut, never caller-schema reinterpretation. Copy/template/import enumerate these typed slots and SchemaRef/revision dependencies. Never reinterpret old source through current keys/aliases. Missing/revoked baseline/profile/IDs/incompatible meaning/shape/direction yields definition_unavailable with migration, never latest fallback.

Compatible optional record additions do not invalidate old bound members. Field-key renames/nested-member semantic changes/target direction or domain/nullable-required-enum meaning/provider-function semantics cannot automatically pass because an ID stayed equal. SOURCE decides compatibility. Required migration previews complete oldID/oldshape→newID/newshape/expression changes and explicitly updates binding via Plan. Display alias rename may retain ID; old baseline still resolves its old alias. New UI wording requires explicit rebinding/text change without drifting to a new namesake identity.

Ordinary edits preserve baseline. Explicit Rebind schema previews actual identity changes and updates it. Current definition references select current source with its own binding; pinned definitions always retain original baseline but data uses a new cut. Inline unsaved queries freeze current schema/profile and promotion saves that exact choice. Saved `library:"current"` means the definition owner's library, never ambient caller library; changing scope edits definition or supplies a declared typed scope parameter. Copy/export carries referenced baseline config revision or explicit same-map migration; losing baseline while claiming runnable Query is forbidden. Core `readConfigProjection(schemaRef,revision,authorized names/IDs,purpose)` returns minimal authorized referenced definitions; private service may retain full baseline without exposing hidden entries/names in errors/previews.

### 5.4 Closed expression-profile/schema/provider ownership

Choose one `weftext.query.cel.v1`: CEL syntax/standard bool-string-list-map-comprehensions, SOURCE Type/Value and named pure signatures, statically rejecting unknown names/dynamic invocation/eval/IO. int is checked i64; CEL float promotion cannot approximate money. Exact decimal/date/instant/ref/Cell use explicit typed adapters and only profile-defined same-type comparisons/functions. Literal constants use SOURCE valid constructors/exact string rules; expected type does not turn float into decimal.

CEL AST comes only from the author-expression parser; official Typst AST decodes native metadata/values, and CLI metadata JSON is not exact-value authority. Builder/text search/navigation produce the same normalized Query plan. Custom pure functions/relation scans/recurrence expansion use SOURCE's closed installed descriptors, not prose that unknown codecs/constraints are “registered.” Each actual function/provider is part of baseline/LogicalCut dependencies/budget. Business interfaces are §§8.2–8.3.

The Cell/CEL boundary is explicit: root fields bind SOURCE T with Cell state. Profile builtins `isValue/isAbsent/isNull/isInvalid/isUnavailable` inspect Cell directly without prior unwrap. Ordinary operations/record members/comprehensions obtain T only from value. Non-value use raises distinct `cell_absent/null/invalid/unavailable` EvaluationIssue, never a state tag consumed as CEL bool/string/false. Optional record members preserve presence; standard `has(p.label)` checks the authorized record key without accessing unauthorized fields. Standard short-circuit `&&/||` does not inspect right values when left already decides. Provider `bool-cell` adapts value to CEL bool, compareTemporal order-cell to i64(-1/0/1), and duration-cell to its fixed record signature; other states become EvaluationIssue rather than putting a whole Cell in `&&`. A missing validity period returned by containsDate is unavailable, not permanent employment.

Static checking records declared T/member presence/possible read effects and rejects cross-T coercion/invalid members. Runtime Cells preserve state distinctions. Only value(bool) filter outcomes decide membership. unavailable/invalid dependencies produce undecided rows/partial diagnostics, while illegal static schema/expression is whole-query error. Explicit state tests may select Only available values/diagnostic rows as visible author conditions, not background data dropping; undecided inventory still forbids complete. Basic UI guards known absence/optional members but does not default to excluding invalid/unavailable via isValue=false. Only explicit Only available values produces and displays that condition. Advanced source is not rewritten to false. Direct-field select exports original Cell states without unwrapping/loss. Non-value aggregate input needs explicit filtering/diagnostic policy or yields partial/error, never zero substitution.

## 6. Nested collections: same-item binding, order and representative queries

Read an array as one structure. Within current ReadBasis, distinguish items by `(owner,field,valuePath,ordinal)`; ordinal is a current locator, not permanent identity. Algebraic `any(A,x=>P(x))` computes the complete predicate on one x; `all` of empty is true and `any` of empty is false. A missing array does not become empty without explicit `orEmpty`. filter/map preserve input order and duplicates. unnest outputs parent rows in their current order then item ordinal, retaining one item binding. Multiple-array expansion requires explicit Cartesian product or actual stable-key association, never guessed index pairing.

```text
phones = [
 {number:"13800138000",label:"personal",note:"Evening"},
 {number:"010-12345678",label:"work",note:"Daytime"},
 {number:"13800138000",label:"work",note:"On call"}
]
any(phones,p => p.number == "13800138000" && p.label == "work")
  -> true, matching only item 3.
Remove item 3 -> false; item 1's number cannot pair with item 2's label.
filter(phones,p => p.number == "13800138000")
  -> authored order items 1,3, each with complete label/note.
```

The saved definition uses direct application metadata. This is a design schema sample, not an executed implementation fixture; binding's schema revision must be an actual Core committed revision. §5.3 baseline interprets field names; users may keep short names without rebinding through display labels/current namesakes:

```typst
#metadata((id: "f8597b15-6b7f-4a6a-9b52-c2732730fe41", types: ())) <weftext>
#metadata((
  kind: "weftext.query", key: "6b2b9e19-5599-48c3-b27b-6b006ec9c397",
  binding: (schema: "weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema",
    revision: "8a81e35f-ad48-4c12-af4f-3d2607102fa9", language: "weftext.query.cel.v1"),
  scope: (library: "current", lifecycle: "live"),
  types: ("person",),
  where: "isAbsent(phones) ? false : phones.exists(p, has(p.label) && p.label == 'work' && p.number.startsWith('138'))",
  select: (
    (name: "person", expr: "self"),
    (name: "contacts", expr: "phones.filter(p, has(p.label) && p.label == 'work')"),
  ),
  order: ((expr: "title", direction: "asc", missing: "last"),),
)) <weftext-query-work-phones>
```

`any(A,x=>P)` above is algebraic explanation. Persisted text uses CEL `A.exists(x,P)`; all/filter/map use `A.all(x,P)`, `A.filter(x,P)`, `A.map(x,E)`. There is one expression parser/evaluator. Text is the author representation and AST is its rebuildable projection, without an independently persisted AST fact. Custom functions such as containsDate have unique static signatures registered through schema/provider; unknown functions reject the whole query rather than executing a dynamic plugin. Parameters/defaults/view columns/export all use SOURCE Type/Value, without another TypedLiteral family.

| Use | Representative Query and meaning |
| --- | --- |
| Current appointment | `engagements.exists(e, has(e.employer.ref) && e.employer.ref == param.org && containsDate(e.during,param.day))`; employer and during belong to the same item. Literal branches do not look up targets; unknown validity is not permanent/current. |
| Organization roster | `related(inverseOf(person.engagements.employer.ref),this.node)` reads authorized personal assertions; Organization stores no duplicate roster. |
| Rank selection | One engagement's `rank.system == param.system && rank.level == param.level`; no cross-system ordering of rank text. |
| Ordered authors | `publication.contributors.map(a,a)` preserves agent(ref or text), family/given/credited_as, roles/affiliations and order. Literal branches do not create people. |
| Contacts table | `unnest(phones,p)` projects `self,p.number,p.label,p.note,p.ordinal`; repeated numbers remain separate rows. |
| Open tasks | `types has task && status == 'open' && project == param.project`; sorting civil due adds no host zone. |
| Calendar tasks | Full native date alone yields `[due,nextCivilDay(due))` deadline cards; CalendarDate(day) needs explicit provider day conversion. Year/month due displays its stated grain/uncertain day as a marked year/month band, never a fabricated first-day event/reminder. compareTemporal supports same-grain ordering and mixed-boundary undecided/partial filters. Instant due does not invent an end. |
| Publication history | `publication.publication_history.filter(s,containsDate(s.during,param.day))` may return conflicting assertions without choosing the latest. |
| Deep headings | `read(headings); filter(level >= 6); project(level,text,occurrence)` includes generated headings; 10+ levels remain actual levels. |
| Money summary | `group(currency,sum(amount))` separates currencies; invalid amounts do not silently become zero totals. |

One recursive Type/Value equality applies everywhere: exact text; mathematical equality for exact numbers with authored spelling separately retained; finite float numeric equality including signed zero while preserving raw values; exact UTC instant; complete ref identity; order-sensitive arrays; record key order irrelevant but nested arrays ordered; tagged union branch+value. group/distinct compare full equality keys after any hash collision and choose representatives by stable whole-value order rather than a page's first row. They create new group rows without writers. Complex sorting uses only the explicit SOURCE comparator/projection rules below, with missing-value position.

Sorting uses SOURCE comparators, not another Value system. record/union/list has no default total order. Named scalar accepts same-type sortable values: exact Unicode-scalar text order without locale folding, i64/finite float/exact decimal numeric order, explicit same-domain civil date/time and exact UTC instant. `ref_identity` compares only same-kind complete canonical EntityRef tuples, without title ordering. Explicit `sequence_lexicographic` accepts same-type scalar-orderable item arrays: first differing item decides, equal prefixes put the shorter first, without reordering/deduplication. Record arrays first map a bound scalar member or choose min/max scalar projection; records/unions have no implicit lexicographic order. Quantity/money declare unit/currency groups and exact magnitude rather than implicit cross-domain comparison. Missing/null order is explicit and invalid/unavailable follows state handling rather than diagnostic-string order. SOURCE/Query/parameters/views/export share these rules.

To edit the second repeated phone, UI submits current basis and item path. Core locates that item again in the exact official before AST, changes only the selected member, reparses the whole source and validates schema, as one Undo/transaction. Reordering or source changes produces conflict; show current items for selection instead of guessing an equal number. Queries, grouping, indexes and exports never rewrite array order. A domain needing cross-version item refs may explicitly add an item ID after justifying the need; every phone does not get a default UUID.

## 7. Saved queries and navigation lists (NAV01–05)

Locate a saved Query by its author note and unique application definition key; label is its display/native locator name. Direct metadata uses `kind:"weftext.query"` and a Core-allocated lowercase UUIDv4 key. Identity is owner+key, not another NodeID. Features owns scope/types/where/select/order members; SOURCE uses official AST/binding qualification consistent with root declarations and requires unique keys/labels over the whole root. Generated/include/nested/calculated declarations gain no definition qualification. Ordinary references default to **current**: open the currently authorized definition each time, show the RevisionId used and explain/reexecute a definition update. Explicit **pinned** references fix a definition RevisionId and display Pinned definition. Unavailable history or current lack of permission yields unavailable, never silent latest fallback. pinned freezes the definition, not data: each invocation still evaluates current authorized data. Capturing exact old results requires explicit result export/history snapshot, not a pinned query pretending frozen data. Old rows/cursors and pinned definitions give no current writer.

Allocate a stable application key once for a new definition in native metadata; its label may change. Read by key rather than a new same-label object. Copy allocates a new owner/key map and rewrites typed saved-definition refs through the same copy map while retaining current/pinned choice. SOURCE coordinates key declaration, uniqueness and literal-projection qualification with root rules. Valid advanced/calculated declarations remain readable/savable source, without counterfeit application-definition qualification.

| Requirement | Candidate interaction/state |
| --- | --- |
| NAV01 | Navigation/list/body panels collapse and resize; narrow screens enter/return in layers. Session stores Query AST/scope, selected identity, scroll anchor+offset, focus and per-owner Draft/Undo. Return reauthorizes and reevaluates; disappeared selection shows removed/deleted/unavailable without reviving rows. Background previews neither focus body nor commit Draft. |
| NAV02 | Optional current authorized summary/image or PDF thumbnail/named dates/key fields and density. Missing author title falls back to a disclosable filename/path, marked as fallback, without writing title. Preview states are absent/building/ready/failed/unsupported/unavailable; missing fields are distinct. Usable rows open immediately without all-library OCR/thumbnails. |
| NAV03 | Directory/tag/type/value choices produce the same removable Query condition chips. Descendants/whole-library/body inclusion stay visible. Facet counts use authorized current `count_distinct(self)` in the same scope; each note counts once per value, missing is a separate group, partial counts mark known coverage. Changing sort does not change date filters. |
| NAV04 | Shortcuts use NodeRef, DirectoryRef, TagRef or a saved-Query key, with reorder/removal leaving targets intact. Node directories use NodeRef; CORE admitDirectory registers a non-Node managed container for DirectoryRef. CORE registers tags and SOURCE typed TagRefs agree with navigation/Query. Explicit admission affects only the selected directory, not the whole library or unmanaged boundaries. Duplicate identity, observation gaps and invalid targets are explicit, never resolved through equal names. Path-only external bookmarks explicitly lack rename stability. List pin changes only eligible current-result display order, bypassing neither filters nor permission, and differs from shortcuts. |
| NAV05 | Personal local ViewPreference is keyed by directory/tag/Query identity and saves sort/group/columns/summary/thumbnail/density with reset. Invalid field/target is shown without replacing a namesake. Settings contain no result/writer and change no source, real tree order or other user's preference. |

Device settings stay in app configuration, outside business metadata. Shortcuts/view preferences/named layouts are re-resolution entry points, not stored grants/results/writers. Personal list pins may store NodeRefs by context; first query current membership, then stable-partition pinned/unpinned rows, sorting each partition by the explicit Query sort. Query-data export keeps Query order; exporting the visible list must explicitly include and label personal order.

Table/list/task list/board/calendar/timeline/gallery/metric and already selected chart layouts consume typed terminals; absent renderers are unavailable. Views perform no hidden fetch/filter/aggregate. AT has an equivalent complete accessible table. Virtualized rows are not a complete dataset. Quick capture covers location/type/template/optional title/fields/body, with field grouping/search/pinning. Only a success Receipt opens the real object and clears Add another. unknown retains opId/Draft. Creation outside current filters is not creation failure; only a complete current query establishes non-membership, otherwise membership is unknown.

Tasks are ordinary task notes with body details and shared status/due/project fields. Checking, moving lanes or changing calendar dates shows exact field before/after and requalifies at prepare; no aggregate/generated row writes. A checklist is a body item, not an automatically created task. Period navigation finds existing notes by explicit series/period/scope and creates only on explicit user intent; Core decides uniqueness under concurrent creation. Civil dates, precision, calendars, time zones and rule dependencies remain exact. Reminder delivery, snooze, repeat-after-completion and natural-language dates remain deferred.

## 8. Rich schemas: retained business scope and Query responsibilities

Every type supports necessary multi-values, nesting, time and sources. Central schema defines types/constraints/relations, with short names for ordinary UI/source. Common fields display first and optional detail expands by group. The following reconstructs selected historical capabilities without old per-value Entry/qualifier/nominal Registry versions. SOURCE owns complete field definitions; Features owns Query/view semantics.

| Type | Retained capability and representation |
| --- | --- |
| Person | Ordered names with role/language/script/validity; repeated phone/email/address/website/account with preset/custom service, usage/label/note; profession and system-qualified rank; appointments with ordinary-node or explicit literal target, position/department/validity/status; observations with quantity/time; independent nationality/legal-sex/gender-identity temporal assertions; life events/important dates with sources/conflict/explicit choice; avatar resources. parent/guardian/manager/mentor directions are explicit; spouse/sibling/general family/social/professional assertions retain separate history and roles. Names/custom descriptions imply no direction, ancestry or current validity. |
| Organization | Multiple names/identifiers/classification/status history/site/address/contact. Business guidance/territorial administration/joint leadership/supervision/subsidiaries/brand/membership/parent/governs/owns/general relations remain distinct; folders imply no business relation. Personal appointments derive the roster without duplicate author data. |
| Project/Task | Project is an ordinary note referenced by tasks; a complete Project domain schema remains a candidate rather than invented required fields. Task status is required; start/due/priority/project/completed-at/dependency retain selected meaning. Body contains detail. Completion time comes from an explicit action rather than invented history on arbitrary status edits. Full task-status history is not already selected. Existing period navigation remains; complex HR/reminder execution is not brought forward. |
| Work/Paper | Work kinds/identifiers/ordered authors and institutions with roles/publication history/venue as a Work/Organization ref and separate container text/version relation/citation key/owner-local attachments; no independent publisher field is defined and venue never stands for publisher. Paper subtype includes useful volume/issue/pages/date/abstract/topics. A separately referenceable version explicitly creates a note; equal DOI never silently overwrites identity. |
| Calendar | Event ranges/sources/participants/series/exceptions/rescheduling/cancellation and existing-period-note navigation remain. Date/instant/precision/calendar/zone are explicit. Deferring reminder execution does not delete saved reminder intent. SOURCE/coordinating owners define complete schemas/providers. Task due is not automatically an event. |

Historical records may contain validity/status or selection/note/sources as native domain members, without another universal fact envelope. Sources may point to original attachment locations or refs, without read permission. Equal repeated values and overlapping periods may coexist. “Current” requires explicit validity/selection rules, not last-write-wins. Multiple preferred assertions are ambiguous and require choice instead of highest-confidence guessing.

Schema owns typed ref slots/direction. Store each fact once at one declared author owner and derive inverses. A symmetric assertion has one explicit owner; edit from either endpoint first requalifies that author source, otherwise remain read-only. Inverse rows retain exact `(sourceOwner,field,itemPath,ReadBasis)` and prepare checks them again rather than writing the displayed endpoint. Another independent assertion retains its provenance. A graph may merge display edges but must expose all assertion sources, without writing unknown origins. Literal person targets have no lookup/inverse/automatic person creation. Deleted targets retain authored facts marked suspended/dangling; cleanup needs an explicit authorized Plan, never implicit cross-source deletion. Personal appointments allow ordinary target notes without mandatory Organization upgrade. Other Organization relations validate schema domains. parent→child, guardian→ward, manager→direct-report, mentor→mentee and engagement→has-engagement inverses are explicit. General family/social/professional relations use SOURCE §4.1 required relationship.preset or relationship.custom in actual dedicated Shapes. Presets are extended-relative / acquaintance,classmate,friend / colleague respectively. Same-text custom stays a distinct branch; cross-domain/dual-branch/unclassified items are invalid with raw preservation. Custom cannot infer direction. Colleague views computed from overlapping appointments are derived on demand without O(n²) stored copies. Explicit professional_relations assertions remain independent and may be recorded under their actual Field rules without an engagement; the two origins are neither automatically dual-written nor merged. Cycle/rank constraints apply only where schema explicitly defines them, not to every free relation.

### 8.1 Actual algorithms for periods, ranges and events

Calendar has three selected data types: period-note/range-note/event. day/week/month/quarter/year are period kinds, not five identities. Period fields identify series/calendar/rule/period key/zone and `unique/many` scope; path/title does not define key. Navigate using the same Query over `(series,key,scope)`; partial cannot establish absence. A unique creation has Core validate a complete negative range and concurrent constraint in the same Plan; many permits independent notes.

Range uses SOURCE.TemporalEndpoint for year/month/full-day or instant while retaining half-open absent-unbounded/none-invalid/at-least-one-bound rules. During remains closed with absent-unknown and none-unbounded semantics. Admission, same-grain/mixed-precision containment, overlap and grain-unit duration use the single fixed SOURCE §3.1.1 provider, **qualifying the complete authorized inputs/providers and cross-argument time basis before infinities or three-valued conjunction**. Separately valid Ranges do not prove a common domain; no days or host zones are invented. Uncertain boundaries are unavailable, not false; proven distant/interior outcomes remain usable. Event reuses Range, but present recurrence requires an exact bounded DateOrInstant template and ExactRange replacements rather than materializing partial-precision anchors. Participants/reminders are unchanged.

Recurrence uses frozen calendar/zone/provider and anchor civil phase to generate daily/weekly/monthly/yearly candidates, applying interval/byMonth/byMonthDay/byWeekday/weekStart and count or inclusive until. Skip nonexistent civil dates and use explicit DST gap/fold rules rather than host defaults. Let B be valid base occurrences,R rDates,E exDates. Deduplicate B∪R by exact original key, then apply **cancel > replace > EXDATE > template** per key, never remove EXDATE candidates before replacement. Replacement alters range/title/status/note without changing original identity to its moved time. Ordinary/window-moved-in candidates use the same decision function below, followed by actual-range filtering/sorting. Budget exhaustion is partial, never complete with missing moved-in events. Occurrences do not automatically create Nodes; explicit promote/adopt does. Series edits preview keep/remap/drop of exceptions. Per-occurrence participant/reminder overrides absent from schema preserve original imports/unsupported reports without full RFC claims.

#### 8.1.1 One original-key decision and window flow

Original key is complete civil date start or exact UTC instant start; equal instants with different offsets share a key. It is not replacement start/array ordinal. Count sorted/deduplicated base starts before EXDATE/exceptions; removal does not add the next occurrence. RDATE consumes no count and may precede anchor/exceed count-until with matching domain. until is inclusive for base starts, not explicitly valid RDATE. Unmatched EXDATE retains intent without creating occurrences. Each exception onset needs proof of B∪R membership, not a ghost event.

After SOURCE typed admission proves at most one exception per original key, `decide(o)` uses the same cut/rules/sets:

| Original key state | Unique result |
| --- | --- |
| cancel exception, with or without EXDATE | No occurrence. |
| replace exception, with or without EXDATE | Complete replacement, inheriting uncovered template fields. EXDATE cannot remove it. |
| No exception and o∈E | No occurrence. |
| No exception and o∉E | Original template occurrence. |
| Multiple exceptions at one key, including cancel+replace/different replacements | Typed series invalid; retain original/diagnostics instead of using cancel to erase conflict. |

Window evaluation enumerates possible overlapping B/R template candidates plus all finite replacement onsets, proving their B∪R membership. Merge/deduplicate original keys, decide each once, filter actual ranges by window overlap, then stable-sort by actual start/end/original key. Ordinary EXDATE-first and moved-in replacement-first paths are forbidden. Replacement enumeration requires complete coverage; otherwise partial with no complete-zero/negative proof. Title/status/note-only replacements inherit original range. Moved-out replacements do not return but retain original key.

| Unrun product oracle | Outcome |
| --- | --- |
| O=2026-10-15 in E, replacement remains in window | Return replacement, not EXDATE disappearance. |
| O in E, replacement moves from outside into window | Same decide returns one replacement with original O, sorted by actual time. |
| O in window, replacement moves outside | Neither original template nor replacement appears in that window; no ghost original card. |
| O in E with one cancel | No occurrence; cancel outranks EXDATE/template. |
| Same O has cancel and conflicting replacement | SOURCE typed admission rejects/preserves source, not two cards or silent cancellation labeled valid. |
| EXDATE absent from B∪R | Creates no occurrence/deletes no other key; retain EXDATE author value and other valid occurrences. |
| Same UTC with different-offset RDATE/EXDATE/exception spelling | One exact key; raw spelling does not bypass duplicate-exception checks. |

This preserves the selected historical rescheduling result without inheriting old wire/version wrappers. New SOURCE/provider contracts remain independently reviewed.

Calendar/holiday packs provide context and retain pack/version/source/applicability in results. Multiple packs coexist without a single isHoliday fact. ICS UID/RECURRENCE-ID/SEQUENCE are source identifiers, not EntityRefs; VFREEBUSY/VTIMEZONE create no automatic note, and VTODO/VJOURNAL require explicit preview mapping. SOURCE owns complete native Period/Range/Event schema; these are concrete Features consumption obligations, not “reuse the old service.”

Optional detail is not mandatory for common records; item IDs are neither blanket-required nor blanket-forbidden. Schema/relation migration previews originals, affected fields and unknown values. Preserve unknown source with read-only display rather than deleting it and calling the result simpler.

### 8.2 Installable relations and actual single-source producers

SOURCE's repaired closed schema adds `relations`, not another relation database. Every relation maps to a real FieldId and an exact expanded typed target path and declares source/target Type domains/direction/literal policy/source cardinality/self policy/graph projection. Inverse names are derived display names, not authored fields. The installer expands schema and verifies each member/array-each target-path component/ref shape/Type membership/max-null constraint. No arbitrary executable target-extractor expression. This is the semantic interface; SOURCE's `schema.json` owns exact JSON members.

```text
extractAssertions(sourceSnapshot, installedRelation, cut)
  -> ordered [{sourceOwner,fieldId,itemPath,sourceBasis,
               relationId,targetRef|literal,domainPayload}]
validateRelationChange(relation,before,proposed,cut,authorizedScopes,budget)
  -> complete dependency set + accepted semantic after | rejection/unavailable
readRelations(relation,scope,cut,budget)
  -> frozen source-assertion rows + coverage
```

Extraction reads only official native field projections; item ordinal is scoped to current sourceBasis. Follow Shape-discriminated ref/text unions: an absent ref path in a text branch is not an absent item. Only a relation allowing literals outputs an independent literal assertion, preserving text without lookup/inverse/target-domain/graph/namesake dependence. Both ref+text is invalid, not skipped; optional absent whole fields/members create no assertion. Source cardinality counts actual author records including permitted literals rather than filtering refs first. Text in ref-only relations is retained raw but typed invalid. Person10 and unknown Work contributors allow literal; Organization12, Work venue/version_of, Task dependency and Event participant remain ref-only.

Core resolves exact EntityRef/lifecycle for ref branches and SOURCE checks Type domains: engagement→ordinary Node, personal relations→Person, Organization12→Organization, Task dependencies→explicit Task, Work contributors→Person|Org, venue→Work|Org distinct from publisher, version_of→Work, Event participants→Person|Org. Actual SOURCE `target_path` member/each steps point to `engagements[].employer.ref`, `publication.contributors[].agent.ref`, `publication.venue.target.ref` / `publication.version_of.target.ref`; task.project is a scalar ref with empty path. Field-only rules consume declared fields/actual endpoint qualification without hidden-body reads.

All 27 historical relations need installed origins, not just prose: Person10, Organization12, Work3(contributors/venue/version_of), Task dependency and Event participant. This round adds task.project as relation28; counts are not acceptance proof. New schema installs each path/domain/direction; one installed relation does not prove every inverse available. SOURCE's closed relation members are `id/source_type/field/target_path/target_domain/target_types/direction/inverse/max_assertions/reject_self/graph_projection/cycle_policy/resolution/delete`, with exact Shapes/constraints owned there. Organization parent/Work venue/version_of max1 follows owning-field constraints, others selected-many. Display-edge merging never calculates authored cardinality.

Create/update validates complete current author container/applicable Types, extracts proposed assertions, checks required existence/Type/lifecycle/self/cardinality ranges and supplies exact narrow mutation/read dependencies to Core Plan. New refs need readable exact live endpoints. Recycled/unavailable existing targets retain facts with suspended projection, not deletion. Remove/cleanup targets author source, never inverse writes. Membership edits revalidate genuinely affected relations rather than merely changing types and leaving invalid typed edges.

Each symmetric assertion has one author owner; independent authors' assertions retain provenance. Graph merges `(fullEndpointPair,relationId)` for display with all assertion origins. A write selects one exact assertion and reauthorizes its source. Two independent assertions are not duplicate storage of one fact. Complete lists reveal no hidden source title/count; partial does not prove no relation. Copy rewrites actual target slots through the same map and validates new source, never scanning strings. External targets are preserved or explicitly lost, without counterfeit identities.

`graphProjection=general/ranked/family` describes presentation, not an automatic acyclic constraint. guardian is general, manager/mentor ranked, parent/spouse/sibling have family roles. General family/social/professional custom text creates no generational direction. Cyclic assertions remain unless an explicit business rule forbids them; ambiguous ranked layout may use general display/ambiguity labels without deleting edges or inventing rules. An installed acyclic rule verifies the proposed graph with DFS/SCC over a complete reachable closure at the cut and records traversed/negative ranges in Plan. Incomplete scope cannot accept. Writes needing global incidence/uniqueness/cycle scopes require previously declared complete constraint-scope permission, not different public errors depending on a hidden edge causing a cycle. Missing scope is uniformly unavailable. Ordinary source-only relation edits do not thereby need all-library reads.

### 8.3 Closed Calendar/quantity/provider installation and calls

SOURCE's repaired schema includes real shapes/constraints for all seven selected base Types plus paper, `providers` and `series_policies`. A provider selects an admitted exact implementation/data/precision/time-zone rule and static exported signatures, not executable code in schema. Config uses only SOURCE's `id/family/implementation/data/exports/limits`, with builtin id+abi implementation and name/inputs/result/effect:pure/query_name exports (unique public CEL name or null). Do not add undocumented digest fields. Core binds config revision/admitted ABI-build digest/complete data/runtime profile. Missing/expired versions do not choose latest or become text. Query baseline fixes signatures; LogicalCut freezes actual implementation/data. Valid shape proves no availability. Only non-null query_name whitelist service exports enter CEL; primitive periodBounds stays internal. Duplicate public query_name rejects installation; internal calls select exact provider ID+export, never array-first. SOURCE `builtin-abi.design.json` is design-signature evidence; product installers compare implementation/ABI/signatures against the trusted Rust builtin registry rather than letting config grant capability.

Minimal consumer signatures use SOURCE Type/Value, without another date/quantity V:

| Function/operation | Complete required meaning |
| --- | --- |
| `containsDate(during,point)` | Share SOURCE §5: both During members must be present; native none explicitly means unbounded, while either missing endpoint, a missing whole during or an empty record is unavailable. Bounded endpoints are inclusive and ordered; point is TemporalEndpoint with usable same-grain years/months and SOURCE §3.1.1 mixed-precision proofs, after call-level common-basis qualification of point and all finite bounds, including two-none validation. Civil/instant mixing or proved reversed bounds are invalid; uncertain boundaries or missing providers are unavailable; non-value states become EvaluationIssue. Instant uses exact UTC and date explicit calendar/precision, without a host zone or Range's absent-end rule. |
| `compareTemporal(a,b)` / `duringDuration(during)` | Fixed v2 exports follow SOURCE §3.1.1: proved temporal order or unavailable, and closed-grain counts/exact instant differences. Explicit Query calls do not create a default record order. |
| `nextCivilDay(date)` | Next civil day under the specified calendar/provider, checked for overflow. Requires complete day date, not invented year/day for year/month/month-day values. |
| `rangeOverlap(a,b)` / `rangeDuration(range)` | `range(start,end_exclusive)` is same-domain half-open with at least one bounded endpoint. Overlap first qualifies every finite endpoint/provider and common civil/UTC basis under SOURCE §3.1.1, then applies §8.1 infinities/conjunction. Same-calendar/version civil values may mix precision without ordered endpoints; cross-calendar/version requires all full days, otherwise unavailable. Civil/instant mixing is invalid; true/false cannot conceal qualification failure. Durations also qualify before unbounded/unit handling. Year/month/day duration is a same-grain years/months/days difference without claiming exact days; instant uses exact elapsed seconds. Open duration is unavailable rather than now. Missing calendar/provider produces no fabricated value. |
| `periodBounds(period)` / `resolvePeriod(period,seriesPolicy,cut)` | The former is an installed calendar/temporal pure function validating kind/calendar/version/rule/key/zone/tzdb/ISO week-year and returning exact range. The latter is Features/Core orchestration using authorized period slots plus same-cut existing/negative-range Query, not a supposedly pure provider reading the library. Actual uniqueness is series/period/scope, excluding title/folder. |
| `expandRecurrence(event,window,providers,budget)` | Complete §8.1 candidates/selectors/count or until/rDates/exDates/cancel/replace and outside-window moved-in handling. Original coordinates differ from replacement times. partial reports stopping position/gaps without complete negative scope. |
| `validateQuantity(kind,value,unit,precision)` | This logical wrapper consumes actual `units/rules` `measurement/quantity` exports, not a new uninstalled export. Height/weight use length/mass and exact decimal with matching provider units/dimensions. Unknown units retain originals but make typed business unavailable. No automatic universal units/currency/rank-system conversion. |

SOURCE installs Period/Range/Event record shapes/local constraints and real Field/Type/Relation/policy config. Features provides named pure validators/functions or bounded read-only expanders. Core freezes providers/complete scope dependencies and checks them at prepare/commit. Provider host supplies only authorized input slots/budget, without direct library/files/network/author writes/authorization signing. Scalar failure is a typed error; a global window expander may return partial. Neither becomes absent.

Recurrence admission checks more than dictionary shape: exact source anchor derives from the event's sole exact date/instant `range.start` (CalendarDate conversion is explicit preview; year/month never supplies a first day) and provider rather than a duplicate authored recurrence anchor. Validate that domain/frequency/interval/selectors/count-until exclusivity/until domain/rDates-exDates original domain/unique exception keys-valid replacement range/full zone-tzdb-rules. SOURCE closed named constraints list exact reads and invoke the fixed profile. daily/weekly/monthly/yearly use civil phase; date lengths are civil days and instant-template lengths exact elapsed. Skip nonexistent dates; explicit installed profile fixes DST gap/fold (for example gap=skip/fold=earlier), never host default. Semantic profile changes require explicit schema/rule migration and keep/remap/drop exception preview. No full RFC/all-calendar claim; retain unrepresentable originals/losses.

series_policies actually installs unique/many policy and explicit series/scope/key producer. unique reads existing and complete negative scope under authorization at the cut; prepare/commit protects the constraint to one Core decision. Missing complete scope yields uniformly unavailable instead of revealing whether hidden notes happen to conflict. Readable existing notes provide precise selection/opening. partial means membership unknown. Multiple series at one period and many notes at one key coexist. Policy/key migration is an explicit config Plan, not filename deduplication.

These installation obligations must actually close in SOURCE's complete repaired schema. Unknown constraint/function/shape rejects installation. Missing seven-Type definitions show specific unavailable capabilities, not a claim that design is executable because definitions will be generated before implementation. This is a coordinated design candidate without implementation evidence for installer/validator/provider/Query/permissions; no finding self-closes before reports and exact-candidate non-author review.

## 9. Independent JSON annotations and atomic suggestions

Use node-owned `weftext.annotations.json`. Records have annotation ID/reply relations/attribution/time/purpose (comment/mark/suggestion)/body/labels/appearance/open-resolved state. Body may use a restricted Typst inline profile with text/emphasis/math/qualified links and no import/file/network/execution. Invalid formatting retains exact text and diagnostics instead of deleting discussion. Unknown data preserves originals; edit only understood records with safe patches. A corrupt file is never overwritten by empty JSON. Offer originals/reliable backup recovery and unreadable-state explanation. Best-effort recovered discussion is marked recovered/unverified and receives no executable-suggestion qualification.

```text
AnnotationTarget = whole_document(owner)
 | source_element(owner, revision, sourceRange, elementRole)
 | source_range(owner, revision, byteRange, sourcePreimage, visibleExcerpt)
 | whole_resource(resource)
 | pdf_region(resourceRevision, pageIndex, pageBox, rectOrTextRange)
 | image_region(resourceRevision, pixelSize, rect)
 | media_interval(resourceRevision, timebase, startTick, endTick, optionalRegion)
```

Coordinates have closed units/origin/rotation/page dimensions or timebase, bounded ranges and valid endpoint order. Never mix microseconds/frame numbers/post-crop coordinates. PDF text ranges depend on the exact revision's extraction product; normalized image coordinates must round-trip to fixed dimensions; video regions combine time and frame-local region. Missing UI support cannot downgrade to whole attachment. Flow/print selection yields the same true source target. Generated/multiple-origin content permits whole-document or explicit real-source annotation; DOM coordinates are not persistent source addresses. Commit a source Draft before requalifying a new range; a failed save retains the annotation Draft.

Target resolution is independent of discussion status: exact/mapped/stale/ambiguous/orphaned/unavailable. Automatic relocation across revisions requires Core's continuous exact edit map and an unchanged range. External change without continuity yields stale/orphaned; similar-quote search never automatically reanchors. Manual reattach selects a current target and previews it; pending suggestions require reconfirmation. Invalidated ranges/elements remain in unlocated discussion lists with replies and resolve/reopen, without deleting JSON/replacing stored summaries/attaching to namesakes.

Annotation reads disclose already stored authorized contents, including authorized stored visibleExcerpt/attribution, without hidden-target reads/new screenshots/context computation. Internal sourcePreimage is location/verification data, never automatically included in annotation-read. Build an authorized projection rather than serializing an entire internal target/preimage record. Delivering raw fragments separately requires exact-source-range disclosure qualification; JSON exports/debug/bulk/download follow the same restriction rather than bypassing it through annotation permission. Current context needs separate source/resource read. Inserting a quote requires readable origin and writable destination Draft, not writable origin. The pasted quote is destination author content and proves no target currentness.

Suggestions have explicit replacement/target basis and pending/accepted/rejected state. apply specifies annotation ID/current annotation basis, not a caller's secret replacement. Core rereads the pending suggestion/current target/source read-write permission, validates unchanged range/sourcePreimage and creates one Plan with source after + annotation accepted after. User previews both exact effects; one opId commits both in one logical decision. Conflict/revocation/unknown fault retains pending state and Draft. Recovery follows OperationJournal and never calls one installed file accepted. Concurrent apply/reject share annotation before generation and have one winner. Reject modifies only annotation without reading hidden target. Reconfirm is a distinct checked preview operation; models cannot manufacture consent/attribution time. Undo is a new full inverse transaction and conflicts if source/annotation changed, not a state-only rollback.

### 9.1. JSON stores discussion; exact source revisions/ranges locate targets

Annotations remain independent in node-owned `weftext.annotations.json`. Body, replies, attribution, resolution and target history are separate from `.typ` body. Lost/unreadable/corrupt targets never justify deleting ideas or discussion. Preserve corrupt sidecar originals and diagnostics instead of replacing them with an empty array. Unverified recovery does not qualify suggestions for execution.

Body annotation identity uses full NodeRef (workspace+note identity), a valid Core RevisionId and an exact UTF-8 half-open range in that source revision. RevisionId identifies historical source, without current read/write permission. Core creates the target on a committed authorized SourceSnapshot using official AST, final origin mapping and current selection. Draft serial/DOM path/screen coordinates/layout page/equal text are not source addresses. Whole-document, element, source-range and media-range targets are distinct.

Native labels help paragraph navigation, but are not mandatory block IDs for every annotation. Annotations do not automatically label every paragraph in the library. A surviving label does not prove an old selection/revision unchanged.

### 9.2. Raw source bytes and visible excerpts are distinct

The source preimage is the exact raw range used to verify location/edit before state. The visible excerpt is the logical text seen/selected by the user, used for discussion/static quotation. They can differ; comparing plain excerpt text to raw source is not the same check.

Suppose `[128,139)` in a longer source is these 11 UTF-8 bytes:

```typst
甲*乙*丙
```

The user sees 甲乙丙 with 乙 emphasized: 9 UTF-8 bytes and 3 Unicode scalars of plain text. The source includes two `*` characters; the excerpt does not. Selecting only 乙 may map to the unique ordinary leaf `[132,135)`. Selecting all three may include emphasis syntax in the source range, but current AST/map must prove that span and operation. Never derive source end from the 9-byte excerpt length.

This JSON illustrates responsibilities, not a frozen wire/persistent schema. IDs/revision are illustrative; Core issues/verifies actual values. Equal text/summaries do not manufacture them:

```json
{
  "annotationId": "82b289ba-01ae-4c7b-8e51-779442316658",
  "target": {
    "kind": "source_range",
    "node": {
      "workspaceId": "58c47256-bfce-4d29-a037-b83a3a644fb0",
      "nodeId": "12d7c4f3-9ee6-4c36-920d-ec6af27bc654"
    },
    "revisionId": "202173ac-0e45-45cb-8998-7289b1688b0c",
    "sourceRange": {"unit": "utf8-byte", "start": 128, "endExclusive": 139},
    "sourcePreimage": {"utf8": "甲*乙*丙"},
    "visibleExcerpt": {"text": "甲乙丙", "kind": "copied-logical-text"}
  },
  "body": "Explain how 乙 relates to its surrounding text.",
  "reviewState": "open"
}
```

sourcePreimage may be Core's retained exact historical slice or a revision reference that allows exact retrieval. Any digest is auxiliary only, not independent location/auth/write revival. Stored visibleExcerpt is annotation content governed by annotation permissions; reading it does not reread target body or compute new hidden context. Raw source containing private material outside the excerpt cannot be disclosed merely because an annotation is saved. Core constructs from the authorized target scope under SOURCE field/body isolation.

The JSON shows internal storage/verification responsibility, not an annotation-read response template. Default visibleExcerpt uses logical selection text from that Html final tree. Explicit print excerpts consume their Paged final data and record target/compile origin, while source ranges require their own actual maps. Retaining excerpts does not cache full target source/private parameters.

Private-URI delivery oracle (unrun product): source is `#link("https://private.example/path?token=private")[公开文字]`, internal sourcePreimage contains raw syntax, and stored visibleExcerpt is 公开文字. Caller has annotation-read/stored-excerpt permission but no target source/URI disclosure. Correct response includes only authorized body/excerpt/attribution/target state, without sourcePreimage/private URL/token/raw source length/new context. Never serialize the whole internal target into JSON. A separate exact-raw-range read can deliver source only under its independent qualification; an excerpt grants no such permission.

### 9.3. Formatting, multiple paragraphs and generated content

Selections crossing strong/emph may have exact same-source ranges containing syntax delimiters. A suggestion needs EDITOR's verified safe operation rather than replacement of the entire raw range by visibleExcerpt. Apply validates exact source preimage and currently consumed semantics/dependencies separately, preserves outside bytes/native containers/target text meaning and reparses fully. Source mutation and accepted state share one Plan.

A selection crossing paragraphs may use one range only if both paragraphs and their separator are contiguous in the same exact authored source and AST/map proves it. The visible excerpt independently retains logical line breaks. Disjoint source fragments cannot become a bounding range that swallows unselected text. If the first stage lacks compound multi-range targets, retain annotation Draft and offer whole-document annotation or separately selected exact ranges, explicitly explaining the difference; never invent a root range.

Included text targets its actual source owner/revision, not root coordinates. Repeated include/function/loop display occurrences must first be distinguished. Equal source spans do not prove a root writer. Generated/multiple-origin text without unique origin permits whole-document annotation or explicit selection of readable real source, not a fabricated root slice. Retain historical excerpt while marking unavailable/ambiguous source targeting. A persistent generated-occurrence target requires separate design, not a hidden expansion here.

Explicitly convert UTF-8 bytes/UTF-16/scalars/graphemes/layout coordinates. Selections do not split combining/ZWJ/CRLF; copy uses logical reading order. PDF/image/audio/video annotations keep exact resource revision/page/pixel/region/timebase, without fictitious `.typ` text ranges.

### 9.4. Relocation requires actual edit mapping

Retain original revision/range/excerpt and separately report current exact/mapped/stale/ambiguous/orphaned/unavailable resolution. Core's continuous exact edit map may relocate unchanged ranges. Inserting text before a range moves its location without permanent text IDs. Overwrites/splits/ambiguity/watcher gaps/external edits without continuity/replacement/unavailable history retain discussion as awaiting relocation. Quote/hash/namesake-label matching never guesses automatic reanchoring.

Deleting/recreating equal bytes never revives old ReadBasis. Current reading/suggestion application reauthorizes; accessible exact history is not current writing. Manual reattach explicitly selects a current target, previews before/after and persists target history via Core. Pending suggestions require reconfirmation. Ordinary body/reply/resolve changes do not silently change targets.

### 9.5. Native paragraph labels and Copy paragraph link

Typst does not automatically allocate permanent IDs to every block. A real block can have a native label and a native link:

```typst
#block[
  这里是一段需要直接引用的说明。
] <para-method>

见 #link(<para-method>)[这段说明]。
```

The author may rename/delete the label. `@para-method` additionally depends on referenceable-object/numbering rules; ordinary paragraph navigation uses `#link(<para-method>)[...]`, without promised paragraph numbering. A cross-note link uses full ID plus exact fragment:

```typst
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#para-method")[方法说明]
```

Copy paragraph link:

1. Currently authorize and locate the actual paragraph/block/origin. Reuse an existing exact unique label bound to that target without renaming it. Relevant source read/address-disclosure permission suffices; no source write is required.
2. Without a label, clearly disclose that this operation adds a native target label. Only writable source and EDITOR proof of a safe block wrap/existing-element label permit a source proposal. A required wrapper is part of the reviewed change, not an allegedly harmless ID.
3. Choose an ordinary label outside reserved application-role prefixes, such as `para-<random-suffix>`. Check the complete current document/dependency label set. Unproved uniqueness/generated collision/uncertain binding denies shortcut creation. This is neither NodeID nor a global block registry.
4. Construct through official AST/source writer, reparse and compare semantics, proving the real target and preserved unselected bytes. One Draft operation/Undo enters unified prepare/commit. Cancel creates no label; conflict retains Draft and requires new preparation. A wrapper that cannot preserve original flow/layout/counters is unavailable with Source repair/existing-label entry.
5. Deliver a newly created label's link as saved only after source commit and authorized current resolution. A Draft link is explicitly not effective; retain Draft without pretending others can navigate it. unknown first queries the original opId rather than allocating/recommitting another label. Report successful save and failed clipboard separately; retain the label with Retry copy instead of fictitious rollback of committed author data.

Create/reuse a label only for the explicitly chosen paragraph, not every paragraph/document/library. Read-only unlabeled origins permit static excerpt+whole-document link or an existing readable label. Quoting does not require source write and cannot fabricate a fragment absent from saved source.

### 9.6. Current navigation, fixed historical refs and static quotation

| Form | Meaning and qualification |
| --- | --- |
| Current paragraph link | NodeRef+label resolves current authorized data. Node rename/move retains identity; label rename/deletion/duplication/generated-binding change may break it without approximate guessing. It does not guarantee unchanged wording. |
| Historical annotation target | NodeRef+RevisionId+exact source range describes the original target. Proven current mapping is separate; original history is retained. |
| Explicit fixed reference | Opens exact historical revision/range through Core's historical read/reference capabilities. Missing/revoked history is unavailable, never current fallback. External encoding/copyable historical URI needs separate specification; ordinary NodeID#label is not fixed. |
| Static quotation | Copies authorized visible text into writable destination body with source link/optional stored historical description. Text becomes destination author content and does not follow source changes. Current link content may differ; UI explains copy time/historical selection. |

Static quotation needs readable origin and writable destination Draft, not writable origin. Creating a source paragraph label is a distinct authorized source write. Reading/copying grants no label creation. Across separate authorities show source-label save and destination-edit outcomes separately, without cross-authority atomicity.

This scope covers static excerpts/source navigation/explicit historical annotation only. Live transclusion requires separate design for current reads/cache/revocation/cycles/dependencies/origin boundaries/export. This addendum neither claims it completed nor adds liveembed scope.

## 10. Templates, copies and bibliography import

Layout templates use standard Typst modules/imports/functions/set/show. Packages see frozen authorized inputs/fonts/resources, without ambient file/network/trusted identity/author writes. Application creation templates define parameter types/defaults and precise title/body/field slots; ordinary UI does not require recipe/wire authoring. Missing/null/explicit input differ and defaults apply only to missing input. Text parameters are safe literals, so `#metadata` input does not execute. Preview includes all new notes/resources/typed refs/annotation preservation or omissions/specific losses and binds the same Plan as execution.

Copy enumerates the complete authorized selection closure, allocates one fresh note/resource/annotation/definition-key/candidate-RevisionId map, traverses every typed ref/link/definition slot via official AST/schema, resolves paths in the old context and encodes them in new source locations through that same map, validates targets/fragments/owners/definition selection, freezes all after images/losses and atomically commits. Do not replace free text/unknown fields/CEL strings/UUID-like phone text. If unknown application definitions prevent complete typed-ref proof, reject full copy or offer an explicit original-preserving/loss plan. Mutual note refs, ref-free definitions and multiple instances of one source associate by full source/result keys, not array position. Resources explicitly re-own rather than share private assets because paths match. Proven annotation targets update through the map; unprovable ones retain discussion with stale state or an explicit omission choice.

Weftext-owned Rust import first retains originals, safely parses complete parts/sheets/resources/unknown data, builds exact native values/semantic structures, locates every unrepresentable feature and describes effect/options, previews and creates via Core. PDF compilation is not business fidelity; renaming extensions is not import. Formula/hidden sheet/cache results remain distinct. Macros/malicious ZIP/XML/extra worker output do not execute. Large jobs may commit bounded groups with a queryable committed prefix. Cancel stops later groups; unknown checks the original group opId without reimporting the prefix.

A paper note's `publication` record is the single editable authority for formal bibliographic data; there is no separately editable Bib database. BibTeX/CSL/RIS/DOI/provider import maps to that record, retaining original-record attachments and field source mappings. Citations refer to note identity. Bib/CSL data for Typst compilation is a frozen projection from current authorized literature snapshots. publication.title is the formal work title, distinct from document title; if used as document title, generate it from publication.title rather than writing it twice. Refresh presents a change preview and leaves author-modified fields as conflicts for choice rather than overwriting from provider latest. DOI/ISBN/arXiv identify works externally, not Nodes; repeated import offers explicit matching/choice, not blind upsert.

```typst
#metadata((
  id: "8aa10c08-3e7d-4d10-9488-92bc7f393050", types: ("paper",),
  publication: (
    kind: "article", title: "原生笔记中的结构化数据", citation_key: "chen2026native",
    contributors: (
      (agent: (ref: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654"), credited_as: "陈明", family: "陈", given: "明", roles: ("author",)),
      (agent: (text: "李华"), family: "李", given: "华", roles: ("author",)),
      (agent: (text: "原生数据研究组"), roles: ("author",)),
    ),
    identifiers: ((scheme: "doi", value: "10.0000/example"),),
    issued: (calendar: "gregorian", version: "proleptic-gregorian.v1", precision: "month", year: 2026, month: 9),
    publication_history: (
      (state: "accepted", during: (start: datetime(year: 2026, month: 7, day: 20))),
      (state: "published", during: (start: datetime(year: 2026, month: 9, day: 1))),
    ),
    container: "知识工具研究", volume: "12", issue: "3", pages: "101–118",
    abstract: "A sample paper for checking the complete chain.",
    keywords: ("Typst", "Structured notes"),
  ),
)) <weftext>
#context {
  let record = query(<weftext>).first().value
  title(record.publication.title)
}

= Reading notes
The body records questions/discussion. PDF is a resource of this note, not a second work.
```

This is fictional; DOI is not a real external record. A positive fixture must use authorized resolvable Person, optional venue(Work|Org), PDF ResourceRefs and installed paper schema. Container text is not a venue ref and creates no journal. Formal data uses SOURCE's single publication shape. The generated body title has no counterfeit writer; the form edits publication.title once, and EDITOR supplies a true field-edit entry point. agent has exactly a ref or text branch and roles is an array; unknown/corporate authors need no forced Person creation. Check contributor order in form→same-item ordered Query→CSL/Bib→citation numbering under one frozen database→DOCX bibliography. Retain unknown date precision without day 1. Unknown roles/provider members preserve originals/diagnostics rather than becoming absent.

### 10.1 Authored native citation-key binding to stable Work

SOURCE owns one direct native declaration per owner, with official AST/standard-binding qualification consistent with root. No independent citation-key identity or second Bib fact:

```typst
#metadata((
  kind: "weftext.citations",
  entries: ((key: "chen2026", work: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/8aa10c08-3e7d-4d10-9488-92bc7f393050"),),
)) <weftext-citations>

见 @chen2026；也可 #cite(<chen2026>)。
#bibliography("weftext.citations.bib")
```

Author entries bind key→complete stable Work NodeRef, including paper. Keys/fixed role label are unique in exact owner source; duplicate/calculated/include/generated/uncertain declarations never select the first. publication.citation_key only suggests a new entry key, without reverse lookup/rebinding after publication edits. Current Work data freezes in this LogicalCut/compile snapshot. No pinned-citation promise: an ordinary key is not fixed historical data.

Production officially qualifies declared entries, Core authorizes/resolves actual Work/required publication FieldProjection, freezes same-cut Type/Value/schema/publication, validates required title/kind/ordered contributors or precise unknown/loss, generates read-only virtual `weftext.citations.bib` and evaluates official `cite`/bibliography for the selected target. The virtual resource is this World's input slot, without ResourceRef/birth/author write. An actual author resource with that name conflicts explicitly instead of being overwritten. Generated data/input key-Work mapping belongs to CompileSnapshot dependencies/retention, not a separately edited Bib library. Work publication reads need neither private body nor Work write access.

Native `@key` can be a heading/other ref; equal entry names do not prove Paper citation. Application catalogs consume proven standard cite static keys or official ref→citation classification/actual same-snapshot output for `@key`. Ordinary @heading remains internal ref. Same heading/bibliography key follows official conflict diagnostics without Work association. Generated citations may navigate by same-compile occurrence but lack author writer/stable edge without a unique source slot. Missing/unreadable declaration/data preserves valid native source with application citation unavailable, never guessing from DOI/title/publication.citation_key/cache.

Copy/fork enumerates entries.work as typed refs and maps actually copied Works or explicitly preserves external Works. Keep owner-local keys; merging owners with collisions requires explicit new keys/full proven cite-ref rewrites in Plan, not string replacement. Unproved generated/variable references reject/lose explicitly rather than incomplete rewrites. DOCX/CSL/Bib export consumes identical Work snapshots/contributor order and actual compilation key mapping. Exact-source export inserts no hidden Bib; standalone Typst bundles explicitly include generated Bib/dependencies or report unavailable offline citation.

SOURCE's limited 0.15.1 experiment proves direct-binding syntax/`@key`/`#cite` with a real Bib and official heading-Bib key conflict only. Hand-supplied Bib does not prove Core generation/permissions/copy/target output. Oracles: equal citation_key on two Works never merges identity; declared entries choose actual target; cold rebuild retains Work and publication-key edits never drift; revocation stops generated citation delivery; conditional Html/Paged citation differences qualify per target; unknown @heading/conflicting keys form no Work edge.

SOURCE citation-binding keys match complete ASCII `[A-Za-z][A-Za-z0-9_-]*`. Never silently strip/normalize publication.citation_key suggestion text. Native keys outside this profile retain source with binding repair/explicitly omitted application bridging rather than cleaning two Works into accidental identity.

Interval forms, Query parameters, import and DOCX/XLSX share SOURCE §3.1.1 native precision records and presence/none distinctions; child tables retain original members instead of converting months/years to date cells. General relationship classification follows SOURCE §4.1 Field+branch+code/custom text. Display language cannot change codes; unproved imported wording cannot guess a preset or merge preset/custom rows. Exact structural output/ordered child tables support round trips; display-only external files need confirmed mappings and loss reports.

## 11. Editable output and intranet downloads

Chinese defaults and font/language delivery follow [EDITOR](EDITOR.md) §8.6/CJK01–08. Templates, conversion and intranet workers use the same authorized frozen environment rather than public font CDNs or incidental system fallback. PDF, editable DOCX and standalone Typst projects separately verify font/language mappings, licenses and missing target-side dependencies. XLSX Chinese strings likewise preserve original values, with workbook fonts disclosed according to target capabilities. A correct browser preview does not prove a correct downloaded artifact.

Exact-source export returns original `.typ` bytes without layout/templates. Exact structured-result export returns complete schema/Cell/value/order without runtime handles/writers. PDF/SVG/PNG use available real Typst outputs; experimental capabilities stay labeled experimental. DOCX/XLSX have separate Weftext converters and actual verification routes.

The named default DOCX profile consumes one frozen Html compile's final semantic tree/same-target products/role-origin associations. It does not infer flow from Paged realization or mix Html/Paged headings/links as Same snapshot. An explicitly selected Paged conversion profile names target in preview/output and uses only that Paged final data with independent capability acceptance. Native headings without display associations do not automatically create ghost DOCX paragraphs; use explicit source-structure output/loss. Html-frame local Paged content retains its local same-Html-compile boundary without claiming whole-document Paged equivalence. New Html style/bridge remain unaccepted producer gates for conversion.

DOCX contains real paragraphs/runs/styles/tables/images/relationships/refs, not page images posing as editable text. Title/subtitle retain SOURCE roles; titleless notes gain no author title. A run-in heading defaults to the whole paragraph in `Heading N`, participating in numbering. Body runs reset inherited character properties to template body formatting while retaining explicit authored strong/emph. Preserve DOCX Heading 1–9 within the native target range; 10+ preserves source and reports unsupported/loss without collapsing to 9. A normal Word TOC may contain the whole paragraph as the stated default; title-only TOC is separately optional. Verify numbering/TOC/refs in actual Word/WPS/LibreOffice and state each application's support rather than treating valid OOXML as interaction proof.

XLSX is a real workbook/sheets/typed cells with explicit table/Query order. Phone/identifier values are text. Values exceeding numeric precision use exact text or explicit loss choice, without prior rounding. Civil date/time/local datetime adds no host zone; instant retains explicit offset or exact text. Nested arrays/records default to readable structure text plus exact subsidiary tables/structured exports; explicit unnest retains source ordinal/duplicates. Strings are string cells, so `=...` does not execute as a formula. Imported formulas use explicit permitted formula slots only. Merge/header/empty/null/missing distinctions are not silently flattened. A PDF table is not a workbook.

Offline routes cover each RenderedLink occurrence from the output's explicitly selected target/compile: actual target→unique output→actual renderer named destination. Default DOCX uses its Html semantic profile and print/PDF its Paged profile, without using an Html catalog to validate Paged output or the reverse. Do not assume labels equal Word bookmarks/HTML IDs or merge repeated includes. Every occurrence has exactly one route/loss; omitted targets are not claimed navigable offline. Routes change artifacts only, not source. External URLs retain target-format behavior without network/resource permission.

Intranet flow is inputs/format→Job generates and validates exact artifacts→inspect outputs/losses→confirm the same frozen plan/bytes→download. No external publication directory or publication receipt is required. Artifacts bind inputs/environment/schema/current delivery permission/digest/length/output name. Every request/chunk/resume checks current qualification, and resume returns the same bytes without recompilation. Revocation stops further delivery; expiry explicitly requires new generation/confirmation instead of silently changing content. Authorize before existence diagnostics. Filename/ContentType come from controlled output; extra worker files are quarantined. Generated/confirmed/delivered/printed/saved-as-Resource/published/unknown are separate outcomes. Download changes no author facts; saving a Resource is another Core transaction.

## 12. Agents, automation and external effects

Retain selected agent/automation/connector permission and outcome semantics using current Plan/Job/OperationJournal; lack of a new requirement number does not delete D10. Agent reads/author writes use identical Core scopes. Models/documents/web/email/tool output are data, never approvals. Bounded delegation states principal/scope/actions/count/time/budget. Field grants do not become body access, and package execution does not get ambient workspace/network/secret. Exact package/font/provider admission supports offline intranet without silent networking.

Automatic author changes freshly prepare complete effects and match an authorized finite rule or interactive confirmation. Out-of-rule effects stop for the user instead of widening scope. Rules are not model summaries and approvals bind exact Plans. Scheduler uses stable occurrence/run keys so restarts do not redo handled occurrences; rule changes do not replace unknown Runs. stop/revocation is checked with an atomic fence before first irreversible send/commit: if it wins, execution stops. Preserve already-sent/committed facts rather than fictitious refunds/rollback. Cancel stops unstarted work; unknown read-only reconciliation retains the original request.

External mutation first freezes exact account/target/payload/idempotency key, displays the full real request and receives authorized user confirmation. Broad grants/read permission/export confirmation do not approve sending. Distinguish prepared/submitting/succeeded/failed-no-effect/unknown. Automatically retry unknown only with reliable same-request idempotency or no-effect evidence, never a new ID. Credential rotation cannot redo an old unknown operation against another account. Compensation is a new authorized effect. Report Core author transactions and external mutations separately, without a fictitious combined atomic Receipt.

Before potentially charged model/tool/network calls, atomically reserve a bounded amount under the actual account/currency/pricing version to prevent concurrent overspend. Distinguish settled/released-no-charge/uncertain. unknown or stop does not erase potentially incurred cost; grant renewal does not reset history. Cost and author effects are separate rollback states shown separately. Secrets enter no public logs/Query/source/artifacts. Workers read limited input/output slots without direct workspace mounts/arbitrary commands. The first stage adds no automatic whiteboard/persistent multi-Query dashboard/reminder execution/complex workflow/real-time collaboration.

## 13. Coverage, prototype risks and design checks

These are unrun design oracles. This author draft inherits no old CLOSED result or implementation acceptance from AST experiments/compilation.

| ID | Positive case and critical counterexample |
| --- | --- |
| F01 / RQ12–13 | Four-form conversion preserves target/fragment/content, shortest path round-trips and literal broken-link copy works. Duplicate label/boundary/revocation/shared AST denies shortcut writes while preserving source. |
| F02 / RQ14 | 白云山/山/山南路, same-range A/B, RTL/overflow/pages are completely accessible. Partial-grapheme discovery is read-only; partial is not complete; no default target. |
| F03 / RQ08–09/15 | Item 1 number cannot pair with item 2 label; duplicates/order/notes survive. Numeric/date/currency types do not implicitly coerce; absent/null/invalid/unavailable differ from complete empty. |
| F04 / RQ06/15 | Generated deep headings remain queryable at real level and duplicate includes are distinct occurrences. Groups have no inherited writer; partial aggregates are not exact totals. |
| F05 / RQ16 | current explains updates and pinned fixes definition with current data. Missing history never falls back latest; trash is read-only; old rows have no writer. |
| F06 / NAV01–05/RQ11/25 | Wide/narrow return retains Draft/filter/anchor, asynchronous preview preserves focus, shortcuts/invalidation/pins/preferences honor their scope; field-only reads do not fetch body/counts. |
| F07 / RQ17 | Task body/fields/board/calendar agree, civil due adds no zone, periods find existing and enforce concurrent uniqueness. No generated/aggregate row actions. |
| F08 / RQ18 | Accurate PDF/image/media coordinates, corrupt JSON retained, orphaned discussion preserved, hidden targets not freshly read. Source+accepted state share one logical decision and recovery/races preserve one winner. |
| F09 / RQ19/24 | Mutual links/multiple instances/ref-free definitions/resources/annotations share one map. Unknown typed slots/loss drift block execution; unknown queries original opId. |
| F10 / RQ10 | Paper form/Query/CSL/citations/Word retain contributor order, refresh conflicts preserve author changes, external IDs are not identity and unknown precision/fields retain originals. |
| F11 / RQ04–07/20 | Actual editable DOCX, run-in body formatting/emphasis, Heading1–9 and 10+ loss; real XLSX without mistaken formulas/rounding. Validate actual Office applications. |
| F12 / RQ20/23 | Intranet download needs no publish, resumes confirmed same bytes and stops on revocation. Offline occurrence routes fully cover links and extra worker output is withheld. |
| F13 / RQ21–24/D10 | Test authorization/external approvals/cost/stop/unknown together; same-document conflicts retain both Drafts, unrelated notes continue and cross-authority atomicity is not invented. |

Prototype priorities: complete official final origin traces for links/headings without counterfeit writers; Aho–Corasick/dense overlap accessibility; nested same-item/empty-state/completeness Query and field-only authorization; navigation state preservation during revocation/Draft switches; multi-file annotation recovery; editable Word run-in headings and real XLSX precision; chunk-delivery authorization boundaries. Literal metadata projection need not wait for whole-document compilation, but declarations and shortcut writes require official AST/exact binding. Preserve source/Draft with explicit capability state where proof is absent, without a Features string parser bypass.

Ownership: this document directly handles RQ08–20 consumers and NAV01–05; SOURCE/EDITOR provide RQ01–07 language/editor/title behavior with output continued here. CORE/EDITOR and Features jointly check RQ21–25 surfaces/storage/auth/transactions/context. Stable directory/tag shortcuts, a single Type/Value, definition-key creation and full Calendar schema are explicit coordination interfaces; old protocol family names do not deliver them. There are no new compatibility decoders for undeployed candidates. Real historical data gets a separately scoped explicit import with original preservation.
