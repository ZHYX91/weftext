---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: ae07f3ef-aee1-473b-a992-a17a530d0416.

Candidate status: D7 file-authority coordination repair candidate; not independently accepted, activated or implemented. Historical stage labels and bounded evidence retain only their original scope. This package now consumes the authored PL-IR-01 stable-production-address/current-Observation repair together with current D3/D6 producers, but the new exact candidate still requires independent review and does not close P1 or imply overall activation.

# D7 Query Algebra

## 1. Source selectors and scan

Selector is closed: `{kind:"workspace"}`; `{kind:"entities",value:S}`; `{kind:"subtree",root:S,includeRoot:bool}`. S is exactly `{kind:"literal",literal:TypedLiteral}` or `{kind:"parameter",name}`, never CEL. entities takes same-domain refs, deduplicating complete Refs into a set without duplicate rows. subtree applies to Nodes or the Node source selector for headings, with NodeRef root. Root-state disclosure precedes lookup; hidden/unavailable roots uniformly yield not_visible. Only a live authorized root permits complete structural traversal. workspace binds the request's single WorkspaceRef; cross-Workspace refs fail. D3 determines existence; path lookup cannot precede authorization.

`scan` is exactly `{id,op:"scan",domain,selector,as}`, domain=`nodes|resources|annotations|headings`. nodes includes ordinary and template, without implicitly excluding Tasks/Templates. resources/annotations produce complete owner-local Refs. headings selects Nodes, emitting `{owner:NodeRef,title:text,level:int64}` for every complete D2 section occurrence, retaining Locator/revision internally without public coordinates. Top-level document title is not a heading. people/tasks/records and plugin domains are invalid.

Output has one column named as, respectively NodeRef/ResourceRef/AnnotationRef/heading object. All scans are unordered: entity domains have at most one row per Ref; headings is an occurrence bag. Internal keys use complete K in §8, with domain, full Ref and a heading's canonical D3 locator position. Positions are valid only for that source revision; paths/titles are not identity. Source-free entity scans read D3/D6 inventory/state only. headings requires D2 full-source qualification and complete projection; invalid sources cannot be skipped.

The set contains live entities observable by the principal. Applicable Ref state-disclosure authorization precedes existence lookup; hidden objects do not count. Exact entities returns zero members for unauthorized and missing Refs alike, without per-Ref status. Public inputs grant no existence disclosure. Structural selectors require complete target-structure observation rights or fail first with not_visible; hidden parents cannot reveal descendants. Negative query-scan dependencies cover inputs, visibility-policy generation and complete inventory cut.

## 2. read: explicit data introduction

`read` is exactly `{id,op:"read",input,from,bindings}`. from is statically EntityRef CEL; bindings is nonempty, each `{name,source}`. Names cannot overwrite input columns. A batch reads only input rows, not sibling outputs. Output adds columns while preserving multiplicity, cardinality, order and lineage. Any from failure fails the whole query. read is not implicit left join: non-live/unreadable targets yield not_visible, not none pretending an absent Field. Use traverse first to follow visible relationships.

Source variants and types:

| Exact source shape | Applicable from | Output type and semantics |
|---|---|---|
| `{kind:"title"}` | NodeRef | text: original D2 title |
| `{kind:"core_kind"}` | NodeRef | TypeSpec exactly `{kind:"text"}`; complete current D2 classification's JSON string `"ordinary"` or `"template"`, only these values; no invented D4 CodeScope/SemanticCodeId |
| `{kind:"facets",scope:"declared"}` | NodeRef | list<text,32>: directly authored Facets in complete D2 classification at the same Node/sourceRevision, unique UTF8-byte order; never inferred from effective |
| `{kind:"facets",scope:"effective"}` | NodeRef | list<text,1024>: complete effective dependency closure of the same declarations/current D4 Registry, unique UTF8-byte order; not truncated to declaration limits; unknown provider is not empty |
| `{kind:"body_text"}` | NodeRef | text: recursive D2 text and explicit line breaks, below |
| `{kind:"field",fieldId,shape:"values"}` | NodeRef | list<T,8192>, complete Field author order; no Entries means [] |
| `{kind:"field",fieldId,shape:"entries"}` | NodeRef | list<object{value:T,qualifiers:Q,note:Optional<text>,authoredProvenance:list<P,16>},8192> |
| `{kind:"derived_duration",fieldId}` | NodeRef | list<DerivedDurationUnion,8192>, one per original Entry; §9 |
| `{kind:"derived_period_range",fieldId}` | NodeRef | list<DerivedPeriodRange<T>,8192>, complete value and independent boundary result per Entry; §11 |
| `{kind:"structural_parent"}` | NodeRef | Optional<NodeRef>, none at root; parent/structure observation required |
| `{kind:"resource_descriptor"}` | ResourceRef | object{mediaType:Optional<text>,byteLength:integer}, complete committed descriptor, no ByteHandle/URL/path; unavailable D2 derived.mediaType is none, never guessed; rebuildable metadata, not author Field |
| `{kind:"annotation_body"}` | AnnotationRef | text: D2 plain_text body |

facets.scope is required and closed, with no UI/data default. Both projections use the same complete current D2 classification, D4 Registry and source revision, and identical full source_read/classification-disclosure gates. Dependencies retain declared/source and effective/Registry respectively. Complete D4 validity remains; declared cannot bypass unknown Facets or facet_conflict. Task requires core_kind=ordinary and exact tasks/task explicitly in declared, not merely effective. General Facet domains use effective under D4 rules. Two explicit reads distinguish different declarations with equal effective sets without inferring/removing declarations.

Q expands the Field's D4 qualifier set into a closed object, nonrequired members Optional. P is Value§5.1's complete D4 authored-provenance union mapping; omitted provenance projects as [], while present empty arrays still fail D4. values validates complete Entry types/cardinality/relationship refs without disclosing unrequested notes. entries exposes neither occurrenceKey nor D5 selectors. Literal FieldIds use same-cut Registry typing, not first-row inference. New Fields/aliases change Registry input, not grammar.

body_text extracts D2 body: textinline uses original value; links/citations use explicit labels or empty text, never implicit titles; resources use explicit captions or empty text; inline containers concatenate in source order. Blocks append LF; sections emit title+LF then children; table cells join with TAB, rows with LF; protected literal/source uses raw payload. Saved Query/View definitions, attribute carriers and comments contribute nothing. Remove one final artificial LF, preserving payload newlines/Unicode. This is not exact author source and cannot be written back. Unknown/invalid D2 cannot be partially extracted.

read authorizes full Ref/state and each source's source_read/field_read/resource_read/annotation_read before availability/parsing. Field permission grants neither whole source nor other Fields; internal preservation/parsing reads confer no disclosure rights. Qualification must cover success/failure, D2 container validity, source versions, actually observed ref-target state, Registry and control dependencies. Ordinary authored Ref/Locator values remain complete under Value§5.1 without resolution; possession does not authorize read.from, traverse or rendering target state/content. Baseline: full source_read with no applicable Field-read deny can consume D2 source. Field-only paths require static complete output-independence proof, including D2 framing/capacity and every control dependency, or fail uniformly not_visible before business reads. Narrow success uses D7 Narrow Field Qualification's Registry-prior preservation proof and explicit metadata permissions, not automatic policy_admin. Reading first and finding no secrets cannot authorize access.

### 2.1. Complete core_kind type and consumption example

After complete D2/D4 classification and read qualification, ordinary Nodes and Templates use the same text TypeSpec. Following `read {name:"core_kind",source:{kind:"core_kind"}}`, an explicit `project` retaining only that column yields:

```json
{"kind":"rows","ordered":false,"columns":[{"columnId":"core_kind","type":{"kind":"text"}}]}
```

Ordinary cells are `["ordinary"]`, Template cells `["template"]`: full cell arrays, with managed rowHandles still transported under Execution§1. CEL `row.core_kind == "ordinary"` is valid text equality, false for Template; `row.core_kind == 1` is type_mismatch. Complete View example:

```json
{"format":"weftext.view","version":1,"inputSchema":{"kind":"rows","ordered":false,"columns":[{"columnId":"core_kind","type":{"kind":"text"}}]},"layout":"table","bindings":{"columns":["core_kind"]},"options":{"title":"Core kind","description":"当前完整分类的只读投影","labels":{},"textDirection":"auto","numberFormat":"canonical"}}
```

Replacing inputSchema with semantic_code, optional or another columnId yields schema_mismatch; the first cell cannot supply a missing type. Source `{kind:"core_kind",scope:"core"}` violates the closed shape. Core `person` or `core/ordinary` fails source classification, not a new code value. D2 Template/Core kind and Task meanings remain.

## 3. Basic relational operators

Every `input` is a relation ID; expr is one CEL string.

| Exact op members, also id/op | Schema / multiplicity / order / lineage |
|---|---|
| filter: input,predicate | bool; retain true rows; 0..N; preserve order/lineage; may_error fails completely |
| derive: input,fields:[{name,expr}] | Add-only, batch-input scope, one output per row; preserve order/lineage |
| project: input,fields:[{name,expr}] | Replace with nonempty closed schema, batch-input scope, one output per row; preserve order/internal unique lineage |
| `unnest: input,expr,as,mode` | list<T>; inner/left; inner emits each item, empty left one none; left as Optional<T>; retain input columns |
| `aggregate: input,keys:[{name,expr}],measures:[Measure]` | Keys+measures only; keys may be empty, measures nonempty; group bag; unordered; erase source/action lineage |
| distinct: input | Deduplicate equal public cells; canonical raw representative; unordered; erase source/action lineage |
| `sort: input,keys:[{expr,direction,none}]` | 1..16 keys, asc/desc; first/last none placement required even when nonoptional and ineffective; complete total order; preserve lineage |
| take: input,count | Counter 0..1,000,000; ordered input required; canonical first count; preserve lineage |

derive/project has 1..64 unique names. Any expression error fails the query; no column-order assignments. unnest as cannot overwrite inputs. inner preserves duplicate items; nonempty left wraps items in some. childKey uses §8 operator+parent+itemOrdinal, with a separate empty marker for empty left. Optional T under left is compile-time type_mismatch, avoiding nested Optional and confusion with empty lists. Ordered inputs preserve parent order then ordinal. unnest branches lineage, so cannot implicitly authorize unique-source actions; explicit EntityRef can be freshly resolved later.

Measure is exactly `{name,kind,expr?,scale?,rounding?}`, kind=count_rows/count_values/count_distinct/sum/avg/min/max. count_rows forbids expr; others require it. Only avg requires scale/rounding; others forbid them. Optional expr: count_values counts some, count_distinct distinct present values, sum/avg/min/max ignore none and return none if no present values, count_rows all occurrences. sum supports same-typed numeric/quantity; avg numeric→Optional<decimal>, quantity→Optional<quantity>. All present quantities require identical complete basis, retaining unit under Value§5.3. Other types fail. Keys must be equatable, without duplicate names or measure-name collisions.

sort compares keys sequentially, then canonical LogicalOccurrenceKey bytes. Internal tie is ascending even after a descending final user key. Evaluate keys over all inputs before sorting; top-k cannot skip later errors. Orderedness is a Query type/effect, not visible source natural order. queryRef/aggregate/distinct/traverse require new sort before take. sort preserves bags; take is semantic truncation, not paging/budget.

## 4. Relationship traversal and related

`traverse` is exactly `{id,op:"traverse",input,from,relation,direction,mode,multiplicity,targetAs,edgeAs?}`. from is NodeRef CEL; relation a complete D4 relationship FieldId or `core/structural-parent|core/document-link`; direction outgoing/incoming, mode inner/left, multiplicity per_edge/distinct_target. targetAs adds NodeRef, Optional under left. Only per_edge permits edgeAs, authorized `{relation:text,direction:text,value:FieldValue,qualifiers:Q}`; core value is object{target:NodeRef}, Q empty. left wraps Optional. Relationship kind is never omitted/generalized to parent. Symmetric direction changes perspective without creating inverse facts.

Each row makes one hop using D4 actual-owner and complete-incidence semantics. Literal targets neither match Nodes nor create inverses/graph endpoints. Non-live targets or insufficient edge/target/metadata rights hide the edge, including counts/qualifiers; unqualified source owner itself yields not_visible. distinct_target deduplicates full targetRefs per input, forbidding edgeAs because no representative is defined. per_edge preserves repeated facts/parallel document links. Unmatched left emits one none, meaning no visible-subgraph match, not no Workspace relationship. Output unordered; §8 full K includes invocation/operator and per_edge/distinct_target/left variants, not merely endpoint pairs; left has empty marker. traverse erases implicit single-source Action lineage; explicit targets still permit fresh actions.

core/structural-parent is D3 tree state, outgoing child→parent, incoming parent→children, requiring complete structure observation. core/document-link comes only from D2 NodeLink occurrences: outgoing author owner→target; incoming needs complete reference range. Distinct source occurrences are distinct edges. Neither merges with organizations/parent, tasks/dependency or people/parent.

`related` is exactly `{id,op:"related",input,from,relation,direction,quantifier,predicate}`, quantifier any/none. Predicate sees only `target` NodeRef and `edge` typed above, comparing existing values without read/dereference. It must be statically total bool, without outer row/scalar correlation; from may depend on row, predicate on fixed params. Target-title tests need explicit D4 relationship values or bounded traverse/read/filter; v1 does not fetch inside predicates. any retains input if any visible edge is true; none if none is true. Preserve order/lineage and full negative-range dependencies. This differs from left-traverse/filter. targetRef equality directly expresses no relationship to a specified node.

## 5. Scalar and QueryRef

ScalarSpec has exactly `{id,kind:"aggregate_value",relation,column}`, relation an aggregate with keys=[] and column its measure; or `{id,kind:"query_ref",definition,arguments}`, callee scalar-result. No caller row/this correlation. Evaluate once per invocation, reusable by multiple derive/filter expressions. The combined graph rejects relation→scalar→same-relation cycles. Global aggregate guarantees one row; arbitrary zero/many-row relations cannot implicitly collapse to scalar.

QueryRef relation is exactly `{id,op:"query_ref",definition,arguments}`, root-only, no input. definition is DefinitionAddress below; arguments a ValueSource object of literals/parameters only, not caller scalars/rows. Callee sees passed values, explicit same-cut context and its definition closure. Security-invoker: saver/owner rights do not propagate. Current definition visibility precedes closed-payload compilation then data authorization; readable definition does not imply readable data.

DefinitionAddress is exactly `{owner:NodeRef,at:A}`, A=`{kind:"anchor",name:text}` or `{kind:"locator",locator:D3Locator}`. D3 unique-anchor rules apply without name/title fallback. For locator, owner/locator disclosure and D2 source-read authorization precede lookup, then D3 authenticates the canonical stable revision binding/sealed-outbox association, resolves the exact production SourceVersion, proves this invocation's current SourceObservation has exactly that sourceVersion at the exact Frontier, and validates the saved-definition element kind/coordinates. No definitionId/ViewRef exists. Resolve once per invocation cut, binding full production ownerVersion, this invocation's actual current Observation in sourceInputs, actual payload/occurrence and Registry closure. A different current production version is stale/unavailable rather than latest substitution; a later new Observation can qualify a new invocation but never revives an old result/action. Cycle detection canonicalizes by actual owner + complete production version + saved-definition occurrence, not token spelling.

Saved payload is complete Execution§7 SavedQueryDefinition. Invocation runs QuerySpec only, not creationPolicy as ordinary computation or this. query_ref depth≤16, total invocations≤64 including repeats. Resolve definitions to node+revision+occurrence before call-stack cycle detection; alternate address spellings cannot bypass occurrence cycles. Import only public terminal schema/cells, erasing order, Provenance, ActionEvidence and implicit lineage. Core namespaces callee occurrenceKeys under caller; duplicate rows remain separate, not cell-deduplicated. No ResultRowHandle import. Explicit subsequent sort can stably take using internal keys without exposing callee identity.

## 6. General domain projections and necessary finite specialized semantics

These compose **existing general data**; domain names are not grammar. Facet tests use `read facets→derive/filter contains(list,...)`; CEL also permits total `contains(list<T>,T)->bool` for equatable T. All Field/relationship examples use identical Registry/authorization paths.

Person/Organization/Task/Library use scan nodes→read facets/fields→filter→project. Engagement retains organization Node or literal, position/rank/department, validity and notes; unknown validity is not current. MyWorks uses library/creator and explicit currentPersonRef, not separate draft/published domains. ISBN/DOI externalIds are not Node identity. Complete Refs distinguish namesakes; display-name changes do not change sets.

Search reads title/body_text and **explicitly listed** Fields, using bounded list.exists and contains/startsWith, explicit rank, sort and project. No hidden global aliases or automatic transliteration. people/name role values alias/former/transliteration enter the same read; Organization names are Fields. Other domains register nameFields and pure-data contributions below without grammar changes. Default product search builds inspectable Query templates, not a second engine. v1 supports exact substring or explicit nfc, not automatic case folding, tokenization, pinyin, transliteration, fuzzy/full-text relevance. Boolean OR combines matches, one Node per row; explicit int64 literals/conditionals rank without hidden-field scores. Snippets come from read text, without automatic highlights/extraction. Plugin removal makes dependent templates definition_unavailable, not empty.

D7 freezes pure-data `SearchContribution`, exactly `{contributionId,version,fieldId,textPath,role}`. contributionId uses D4 namespace/local-id lexical rules, version positive Counter, fieldId from a verified available namespace. textPath is 0..8 static object members ending at text or Optional<text>; role=`name|alias|content`. No scripts/network rights/private full-text payload. D10 owns admission/installation/authenticity and same-cut complete contribution set with independent generation dependency. D7 adds no RegistrySnapshot member or alias authority. Until that producer exists, use only explicitly built-in version-fixed descriptions with identical Field validation, or explicit Query; no implemented dynamic-registration claim.

Template input is one Workspace, needle text, explicit contributionId set (or user-selected all bound to its then-complete set), and title/body booleans. Sort contributions by ID bytes into read columns; expand textPaths with the full D4→D7 bridge. Required exact text uses path, Optional exact path.orValue(''); required NFC path, Optional NFC path.orValue(nfc('')). Exact paths use exact needle, NFC paths nfc(needle); contains/equality require identical complete TypeSpec. Generate total values.exists matching per contribution, never may_error .value() or nonliteral-conditional flow inference. Ranks: title exact0, name exact1, alias exact2, substring3, body/content-only4; omit nonmatches, take minimum match rank. Derive solely from authorized values, sort(rank,title,internal tie), project NodeRef/title/rank. Empty needle is invalid_request, not undeclared enumeration. Unavailable required contributions fail the whole query; skipping unauthorized Fields cannot claim the same search. Column/AST/budget overflow is budget_exceeded, not plugin-order truncation. Extensions add descriptors/Registry facts, not grammar/package-wide rights.

Link-display Queries explicitly project labels: explicit source label first, otherwise authorized target title. Person preferred name participates only through explicitly selected people/name/qualifiers. Without target rights, show the call site's known static unavailable-link state, never hidden title. Full Ref remains; View uses projected label only.

Graph can use traverse edge tables retaining relationship kind, direction, asserted/derived and explicit endpoint labels. FamilyTree admits only D4 graphProjection=family kinship, not friends/professional generations. ranked suggests layout, not permanent rank/kinship proof; layered_v1 uses View forward/reverse/same/none constraints. For10k engagements, selected person→outgoing engagement organization→incoming engagement other people uses two traversals. Preserve first qualifiers, compare both explicit validities, remove self, distinct peer Ref to derive colleagues on demand, without roughly50m authored facts. Missing time is not current: require both validities, matching date/instant variant and bounded endpoints, then `startA < endB && startB < endA`, exclusive ends. Unknown time cannot claim overlap. Explicit unionValue/Optional lazy guards prevent none reads even where filter is may_error. Authored unbounded endpoints, not missing validity, allow respective absent-start/end comparisons to become true. Calendar/version/precision is dynamic basis: dateBasis(a)==dateBasis(b) must lazily guard comparison; equal TypeSpec does not imply comparable basis. Missing comparator fails at read. Mark derived, bind both facts and full negative range; asserted professional relations coexist without automatic merge. All-pairs joins remain outside v1; this selected-person two-hop needs no colleague operator.

`recurrence` is the separate temporal-expansion operator invoking frozen D4; §9/§11 remain general read adapters. Shared temporal semantics, not namespaces, define all three. Exactly `{id,op:"recurrence",input,from,horizon,limit,as}`; from NodeRef, horizon the TypedLiteral bridge below, limit1..65535, as adds `{originalStart:point,range:range,title:Optional<text>,eventStatus:Optional<code>,note:Optional<text>}`. Input needs range+recurrence Field, complete RecurrenceReadContext/1 and explicit Calendar/tzdb versions. Nonrecurring Nodes do not generate one implicit occurrence; callers filter first. Full D4 horizon/limit applies per expansion; overflow fails, not take. Unordered bag keys use §8 invocation/operator/parent+series+canonical originalStart, not display time. Duplicate parents retain separate keys; include moved-in exceptions; equal finals with different originals remain separate. No materialization/reminders/external sync; absent rule provider is unavailable.

horizon uses existing Value§1, not new date_range/instant_range primitives: exact object with UTF8-ordered end/start, both Optional<calendar_date> or both Optional<zoned_instant>, values both some. Validate TypedLiteral, map values to D4 start/endExclusive plus matching date_range/instant_range kind, then original same-basis/positive-range gate. Never send Query wire directly to D4 or leave encoding optional. Source variant must match static point type or type_mismatch. originalStart uses that point type, range the same Optional-endpoint bridge. title/note Optional<text>, eventStatus Optional<semantic_code> with exact current calendar/event-status ResolvedCodeScope. Presentation values come only from explicit D4 overrides or none. Title/Field fallbacks require another explicit read and CEL, not implicit access. No author-writeback/Event creation.

Calendar period/range/event reads defined Facets/Fields. period enters Calendar/Timeline via §11 civil boundaries, range through original Field bridge, event through explicit range. Seven-member period identity and scope/config versions are not display guesses. Frozen DerivedDuration uses the adapter below. Unfrozen holiday/workday, anniversary/solar-term algorithms cannot be added; D4 rule providers remain unavailable before D10 admission. Explicit precomputed data may display without writeback or claims of computation.

## 7. Complete example: a new domain without grammar changes

Assume the Registry validly defines `equipment/device` and `equipment/serial-number:text`. This Query uses general operators only. Substitution with people/person and people/name changes Registry-derived types, not grammar.

```json
{"format":"weftext.query","version":1,"parameters":[],"relations":[
 {"id":"n","op":"scan","domain":"nodes","selector":{"kind":"workspace"},"as":"node"},
 {"id":"f","op":"read","input":"n","from":"row.node","bindings":[{"name":"facets","source":{"kind":"facets","scope":"effective"}}]},
 {"id":"d","op":"filter","input":"f","predicate":"contains(row.facets, 'equipment/device')"},
 {"id":"v","op":"read","input":"d","from":"row.node","bindings":[{"name":"serials","source":{"kind":"field","fieldId":"equipment/serial-number","shape":"values"}}]},
 {"id":"p","op":"project","input":"v","fields":[{"name":"device","expr":"row.node"},{"name":"serials","expr":"row.serials"}]}
],"scalars":[],"result":{"kind":"rows","relation":"p"}}
```

This assumes a valid Registry, not an installed equipment plugin or real fixture. Equipment adds no domain, EntityRef or permission interface. Use at+Optional for one serial; a first row with one value cannot imply scalar schema.

## 8. Per-operator LogicalOccurrenceKey

K is an internal closed tagged tree with only these constructors. All rows use it, never host enumeration, partition number, incrementing counters, display-value hashes or page positions. Encode D3-CJ/3 canonical UTF8 and compare unsigned lexicographically. Nonnegative ordinals use D3 integers; Refs complete wire; eqKey Value§2's TypeSpec-bearing Equality canonical tree. Compare full trees, not probabilistic hashes.

`I` is invocation path, root=[]; each relation/scalar QueryRef appends caller canonicalOrdinal. Repeated calls do not share I; repeated references to one scalar remain one invocation. `o` is current Query canonicalOrdinal. `base={invocation:I,operator:o}` is mandatory in every object below. Parent/callee keys embed full trees, not text. Multiroot graphs use the main contract's unique CanonicalGraph ordinals.

| Operator | Exact K members beyond base |
|---|---|
| scan entity | kind=entity, domain, ref; selector already Ref-deduplicated |
| scan headings | kind=source_occurrence, owner, revision, locator: actual complete D3 position |
| read/filter/related/derive/project/sort/take | kind=preserve, parent:K; 0/1 output preserves unique input key despite projected-away columns |
| unnest inner or nonempty left | kind=unnest_item,parent:K,ordinal: original zero-based index, distinct even for equal items |
| unnest empty left | kind=unnest_empty,parent:K, distinct from ordinal=0 |
| traverse per_edge | kind=traverse_edge,parent:K,edge: exact below,direction |
| traverse distinct_target | kind=traverse_target,parent:K,target:NodeRef |
| traverse unmatched left | kind=traverse_empty,parent:K |
| aggregate | kind=group,keys: declared-order complete typed eqKey array; global keys=[] without representative input key |
| distinct | kind=distinct,cells: typed eqKey array in schema order; public raw representative follows Value |
| query_ref | kind=query_import,callee:K; caller base and independent callee I |
| recurrence | kind=recurrence,parent:K,series:NodeRef,originalStart: full typed Equality canonical tree |

edge has three closed variants: Field `{kind:"field",owner,sourceRevision,fieldId,occurrenceKey}`; D2 NodeLink `{kind:"document_link",owner,sourceRevision,locator}`; structure `{kind:"structural",child:NodeRef,parent:NodeRef,placementRevision}`. Use actual authorized facts. Both symmetric endpoints share canonical owner/source occurrence; each from emits a fact once, direction distinguishes perspective. Literal targets do not create edges. No cross-QueryRef ActionEvidence reuse. Keys establish unique bag occurrences; Provenance separately determines single-source identity/writability.

Uniqueness induction per relation: scan source uniqueness; parent preservation for1:1; parent+ordinal for unnest; parent+actual edge/deduplicated target for traverse; one output per group/distinct equivalence class; invocation path+callee uniqueness; D4 originalStart uniqueness per recurrence parent. Tags are disjoint. Unordered transport uses K order without author-natural-order claims. sort breaks complete user-key ties with ascending K, then take truncates. Error order uses failed operator's input K plus macro-ordinal path, unaffected by enumeration/worker partition.

## 9. Read-only D4 DerivedDuration adapter

`read source={kind:"derived_duration",fieldId}` requires expanded valueType date_range, instant_range or a closed union entirely of ranges; otherwise compile-time type_mismatch. fieldId remains Registry-constant, not a calendar-specific operator. Complete ordinary Field authorization, availability, Entry and qualifier/provenance/ref gates precede D4 DerivedDuration/1 per Entry. Return Value§5.3's complete union, preserving author order/count; empty Field=[]. No preferred/first/latest selection or writes. calendar/range is a common catalog input only.

Bounded date differences: ISO day Gregorian ordinals, month 12*(year-1)+month-1, year year difference; other verified calendars use calendar/version/precision comparator orderedLexemes indexes. Instants subtract unbounded UTC integer seconds plus exact fractions, canonical decimal output, not host time ranges. Open bounds yield unavailable/open_range only after all present bounds pass original provider/ordering gates. Bind Registry comparator/unit/code, tzdb contributions/actually used rule versions, Field/source revision and negative ranges. Duration adds no Calendar rule/device default.

D4-valid zoned_instants use explicit offsets for UTC, not implicit civil-time solving. Combined reads needing RecurrenceReadContext coverage must bind/prove it completely; otherwise do not invent timezone-segment reads. Provider/version changes invalidate equal-range caches; open_range cannot mask unparseable known bounds.

## 10. Value-origin evidence for asserted graph edges

Core creates internal FactOrigin={actualOwner,sourceRevision,fieldId,occurrenceKey,complete original value/qualifiers and currently visible endpoints} for validated D4 relationship Entries, neither public value nor ActionEvidence. Each read entries/values item carries its own origin, not all Field origins. Member selection, explicit Optional/union unwrap and object/list construction retain per-member/item origin trees. Literals/params/row entities create no relation origins. list.filter retains selected items; map unions only origins actually used by that iteration's expression; nested lists preserve child trees. Numeric/text conversions may retain origins without proving original relationship equality. Unknown propagation is unprovable, never reconstructed from apparent equality. unnest selects the exact item's tree while erasing implicit Action lineage: separate purposes.

For each asserted graph row, union origins of sourceRef/targetRef/relationColumn: exactly one complete real origin is required. Prove exact endpoint equality under D4 direction and relationColumn=FieldId, after current state/subject/target qualification. Literal target, mixed facts, absent origin or transformed nonmatching endpoints fail invalid_provenance. edgeLabel/layout constraints from explicit constants/read values neither change direction nor prove kinship. traverse edgeAs uses these rules; core structure/NodeLink uses complete original occurrence evidence and identical exact matching. query_ref erases origins. Mapping several relationship Fields to same-typed objects then unnesting twice can preserve each fact without implicit Field-mutation rights.

## 11. DerivedPeriodRange: read-only boundaries for general period values

`read source={kind:"derived_period_range",fieldId}` is closed and accepts NodeRef only. Same-cut constant fieldId passes ordinary definition disclosure, type, source/Field permissions and all Entry validity. Core infers `weftext.query.derived-period-range/1`; unsupported means unsupported_feature, never string-date/renderer fallback. This first freezes shared period semantics; other Fields reuse it without new features. No author Field, content identity, transaction or Registry member is added.

### 11.1. Complete input and output types

After alias expansion, D4 valueType must exactly equal current `calendar/period-value`: required `{period:P,seriesKey:exact text}`. P has seven required members: calendarId/calendarVersion/periodKey/periodRuleId/timeZone/tzdbVersion exact text, periodKind field_local semantic_code with codes in D4 order `day,month,quarter,week,year`. Reject defaults, extra/missing/optional members, normalized text, similar names or other code scope. Structure/semantics are frozen, not FieldId calendar/period or Facet calendar/period-note requirements; other namespaces may reuse the alias/identical structure. Static mismatch is type_mismatch. Resolve original aliases and Calendar definitions at compile time even without data.

Selecting this adapter validates every value as CalendarPeriod: D4§9.4 same-cut verified Registry proves calendar/version/kind/rule tuple, kind↔keyProfile, canonical real periodKey and independent tzdbVersion/timeZone. Apply to every FieldId, not just the built-in calendar/period shortcut. Preserve complete original qualifier/note/provenance/ref/cardinality/constraint validation without outputting unrequested additions. Authorization first yields not_visible; missing disclosable definitions unknown_definition/definition_unavailable; invalid/unavailable authorized sources/dynamic providers source_unavailable, not the business unavailable values below.

T is the ordinary Field-values D7 TypeSpec under the complete current FieldId, including full field_local FieldId. `DerivedPeriodRange<T>` is notation, not primitive:

- Required UTF8-ordered object members are `range:R`, `source:T`. source preserves the complete Field-bridge value, including seven-member period and seriesKey.
- R is ordered closed union: first `bounded`, object `end:calendar_date,start:calendar_date`; then `unavailable`, object `reason:exact text`. Bounded endpoints nonoptional, strict start<end, exclusive end. Reason only `endpoint_out_of_domain|unsupported_calendar`, D7 status text without invented code scope.
- Arrays preserve author order/multiplicity, exactly one result per Entry, empty Field=[]; unavailable projection never drops Entry. Each item includes its own value/boundaries; no separately read lists joined by guessed ordinal. Maximum8192, original rows/value/work/dependency budgets checked before charge/work, no implicit limit.

Use Value§1 wire: e.g. `{"variant":"bounded","value":{"end":{"kind":"calendar_date","calendarId":"calendar/iso8601","calendarVersion":"1","precision":"day","lexeme":"2026-10-01"},"start":{"kind":"calendar_date","calendarId":"calendar/iso8601","calendarVersion":"1","precision":"day","lexeme":"2026-07-01"}}}` or `{"variant":"unavailable","value":{"reason":"endpoint_out_of_domain"}}`. Outer source is always required.

### 11.2. Five ISO profiles and boundaries

Compute Gregorian civil-day boundaries only for verified `calendarId=calendar/iso8601,calendarVersion=1`. periodRuleId can be any valid matching local ID; verified keyProfile, not spelling, selects algorithm. Other completely valid calendars/versions yield unavailable/unsupported_calendar, without applying ISO, changing identity or loading D10 code. Missing rule/provider still fails.

| periodKind / verified keyProfile | Inclusive start | Exclusive end |
|---|---|---|
| day / iso-date-v1 | Gregorian periodKey date | Next civil day |
| week / iso-week-v1 | Specified ISO week-year/week's Monday | Seven days later; week1 contains January4, Monday first |
| month / iso-month-v1 | Specified year/month's first day | Next month's first day |
| quarter / iso-quarter-v1 | First day of month `1+3*(quarter-1)` | First day three months later |
| year / iso-year-v1 | January1 of specified year | January1 next year |

Use exact integer proleptic Gregorian civil arithmetic and400-year leap rule, not device locale/clock/tzdb algorithms. Outputs retain ISO calendarId/version, precision=day, passing D4 CalendarDate/same-basis order gates; prove day-comparator availability first. This freezes adapter arithmetic without opening CEL date constructors/arbitrary date addition.

Compute true endpoints in the mathematical integer calendar, then check D4 public domain0001..9999. Any endpoint outside yields unavailable/endpoint_out_of_domain with original valid period preserved. No clipping, host exception, open-bound substitution, fake range write or author invalidation.9999-12-31,9999-W52,9999-12,9999-Q4/year9999 may be valid identities with unencodable exclusive ends;0001-W01 starts at encodable0001-01-01. Expose this complete status. Wider ordinary Calendar cells require separate D4 coordination, not a silent domain change.

timeZone/tzdbVersion remain validated/bound identity members. Civil computation neither converts midnight to instant nor reads timezone segments; DST23/25-hour days/device defaults do not affect it. Equal civil days in different zones retain different period identities. No elapsed-seconds calculation; use explicit existing instant ranges/rules. Anniversaries/workdays/holidays/solar terms/external-rule admission remain deferred.

### 11.3. Dependencies, row identity and consumption

Bind full source revision, Field and recursive alias/schema/code scope, Registry generation and actual calendar rule/comparator/tzdb contributions, authorization/control, scan/empty-Field ranges. unsupported_calendar binds its validated actual calendar/version. Source/rule/provider/authorization changes invalidate/reset caches/subscriptions. Action prepare/commit reprove dependencies even if dates equal. Resolve at the same cut, never caller-reported validation.

read stays1:1 with original LogicalOccurrenceKey/Node lineage. List items create no Node/Occurrence identity. Derived range is neither writable FieldSelector nor relationship FactOrigin. unnest uses §3 parent+ordinal and erases implicit Action lineage; only freshly resolved explicit NodeRef can open/change the Node. Move/rename/template changes preserve period identity, but source revision invalidates old action evidence.

`D7 Temporal Query View Witness v6.json` contains complete positive examples, per-stage types/counts and terminal cells: explicit Nodes→title/derived_period_range read→inner unnest→bounded filter→derive unwrap→project full key/source/start/end/title/target→sort→Calendar/Timeline same terminal. Key `{target:NodeRef,source:T}` is globally unique there because calendar/period cardinality0..1. Repeatable Fields require valid author Query keys or View duplicate_key; internal ordinals are not public identity. Month/week are viewports over dates, not periodKey reinterpretation. Clipping is explicit D8 interaction, changing neither Query results nor identity.

For unavailable, complete all-items Query+table retains every source/reason; a separate explicit bounded-only Query selects drawable ranges. Calendar cannot claim all periods: products show the selected bounded-only condition and available complete-state query, not silently drop undrawable items. D7 is not a multiterminal dashboard; one Query need not return two Views. Both Queries require separate complete success/current authorization; two cuts cannot become one snapshot. Canonical all-items export retains unavailable.

Calendar/Timeline consumes nonoptional day points only, not source.period or guessed zones, and creates no Event/reminder. Month/week views preserve NodeRef, seven-part identity and seriesKey. CalendarPeriodScopeBinding remains D6 control input; no author scope field or new unique/many decision.

## 12. Actual current scan and source proof production

Consume real DependencyProof/2 without changing algebra. Static selector/params, principalAudienceToken, Workspace/CommitDomain and authorization determine full potential scope before existence/source reads. workspace maps to `{kind:"workspace"}`. Nonempty entities are full-D3-RefKey sorted/deduplicated into `{kind:"entities",refs:[EntityRef...]}`. subtree maps to `{kind:"subtree",root:NodeRef,includeRoot:bool}`. headings still selects Node owners, allowing subtree, then expands complete source decoding without HeadingRef.

Legal D7 entities=[] is statically empty; D6 nonempty entities.refs cannot accept it. Emit no query_scan key/object reads and infer no Workspace/other-range emptiness. Retain actual request authorization/definition/Registry/rule dependencies. Every nonempty scan needs exact `QueryScanKey{kind:"query_scan",workspaceRef,principalAudienceToken,domain,selector}` complete positive/negative membership/visibility-policy proof, not only hits. domain exactly nodes/resources/annotations/headings; never title/leaf-UUID/cell sorting.

Source-free entity scans use actual inventory/lifecycle/owner and full query_scan without automatic source_read. headings/title/body/classification/Field/Annotation body and other source adapters need full SourceObservation/1, immutable source pin and actual source DependencyKey. observerDomain equals execution domain; real production domain and external/managed version remain intact. Reading authored Ref/provenance values does not fetch targets. Actual traverse/related/graph/target-state/Locator-position use adds explicit target state/range/source qualification; input Ref grants none.

Enumerate all real positive/negative graph dependencies: lifecycle binds birth/canonical/lifecycle/owner; placement_range uses actual nine StructureRange variants for exact parent/subtree/owner-membership/reply/restore scopes; authored inbound ref_inbound, D4 incidence relation_incidence, Calendar scope/config/membership calendar_scope, contributions registry, used temporal rules temporal_rules, authorization partitions authorization, full populations query_scan. foreign_binding/replica_registry/conflict_record/execution_resource use exact owner keys/ranges only when actual semantics/control requires them. Neither fill all14 with empty tokens nor omit actual usage. Source/lifecycle/Registry/Field revisions are not interchangeable; empty sets require real continuous stamps.

Post-query and collection/bulk/all_result recompute all reachable Queries, sort/take/aggregate and negative ranges by the same full algorithm at the original proposed cut. SourceStamp/SourceRevisionPlan and original candidate uniquely materialize proposed facts. Before final identity, symbolic subjects remain protected-plan internals, not public EntityRefs. Missing required owner proof fails the complete strong result; no renderer/client row completion or page-one membership closure.
