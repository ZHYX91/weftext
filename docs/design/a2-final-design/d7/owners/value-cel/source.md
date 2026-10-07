---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 5bb6d1c7-f8f4-4a29-ade3-c7f12dc837c6.

Candidate status: D7 file-authority coordination repair candidate; not independently accepted, activated or implemented. Historical stage labels and bounded evidence retain only their original scope. This package now consumes the authored PL-IR-01 stable-production-address/current-Observation repair together with current D3/D6 producers, but the new exact candidate still requires independent review and does not close P1 or imply overall activation.

# D7 Value and CEL Profile

## 1. Types and wire

TypeSpec is a closed tagged object. Primitive `{kind:K}` has K in `bool|text|int64|integer|decimal|node_ref|resource_ref|annotation_ref`. text is an exact Unicode-scalar sequence. D4 nfc-for-compare uses `{kind:"text",comparison:"nfc"}`; exact omits comparison, rejecting other explicit values. `{kind:"semantic_code",scope:S}` uses complete resolved D4 CodeScope; field_local also binds full FieldId, separating equal codes from different Fields. Other forms: `{kind:"calendar_date"}`; `{kind:"zoned_instant"}`; `{kind:"quantity"}`; `{kind:"optional",item:T}`; `{kind:"list",item:T,maximum:N}`, N=0..65535; `{kind:"object",members:[{name,type}]}`, at most64 unique names following this section's rules and sorted by UTF8; `{kind:"union",variants:[{name,type}]}`,1..8 unique ordered variants. Nested optional(optional(T)) is forbidden. Objects require every member; nonrequired D4 members become Optional. No open map/any/dyn/null. Calendar/version/precision and dimension/unit come from each D4 value; they cannot be invented as static constraints from unrestricted FieldDefinition or first value. Comparisons check full compatibility or type_mismatch.

D4 date_range/instant_range maps to closed `{start:Optional<point>,end:Optional<point>}`, endExclusive→end, retaining D4's at-least-one-bound and endpoint-validity gates. Unbounded means none; wholly absent validity means outer none, distinctly. Qualifier date_range|instant_range and calendar_date|zoned_instant use date/instant union variants. bounded_set/sequence maps to list: sets retain D4 canonical authored typed-value order without Query-Equality deduplication, preserving valid NFC-equivalent raw-distinct items; sequences retain author order. Only explicit distinct/aggregate uses Query equivalence. alias_ref expands within Registry closure to structural TypeSpec, retaining Field/alias-version dependencies and rejecting recursion. external_identifier maps to `{scheme:text,value:text}`, preserving qualified-scheme validity. Calendar/quantity scope cannot be compared from labels/units alone. Other D4 validity constraints remain unchanged.

`TypedLiteral={type:T,value:V}`. V encoding: bool native boolean; text JSON string; int64/integer/decimal canonical decimal strings; refs embed corresponding complete D3 canonical wire, never bare IDs; semantic_code canonical D4 code string; calendar_date/zoned_instant original closed D4 wire; quantity complete D4 wire with same-cut Registry unitId resolving dimension; list V array; object exact schema-named members; union `{variant:name,value:V}`; optional `{state:"none"}` or `{state:"some",value:V}`. Schema supplies T, cells encode only V. Reject unknown/null/missing members. Empty text/list, none and unbound parameter are distinct.

integer/int64 lexical form is `0|-?[1-9][0-9]*`; decimal uses canonical D4 exact decimal, without exponent, plus, leading zeros, -0 or redundant trailing zeros; a noninteger ends nonzero, integer values may use integer lexical form. int64 range is [-2^63,2^63-1]. integer/decimal are mathematically arbitrary precision subject to shared resource budgets, not host double or34-digit/18-scale limits. Valid D4 values beyond processing budget yield budget_exceeded, never truncation/coercion. Protocol Counters retain their numeric lexical/range rules, distinct from arbitrary-precision authored numbers.

Object names preserve D4 lowerCamel verbatim; Query constructors also permit `[a-zA-Z_][a-zA-Z0-9_]*`, not terminal-column/parameter lower_snake grammar. Name/type encodings remain byte-budgeted. Union variants preserve D4 tokens/case, never display-label renames. Member comparison/canonicalization and schema member lists use UTF8 name order; terminal columns independently retain declared order.

## 2. Equality, canonical representation and order

Equality requires identical complete types. bool/exact text/integer/int64/decimal use literal/mathematical values; NFC text normalizes for comparison while preserving source; codes use scope+code, refs full D3 tuples, dates full calendar/version/precision and canonical date, instants exact UTC regardless of offset/display zone, quantities full dimension/unit and exact magnitude. Different dynamic date/quantity scopes compare unequal and stay in group/hash keys; ordering requires compatible scope or type_mismatch, never conversion.

none equals none, some recurses; lists are ordered, objects follow schema, unions variant then value. All legal public types support equality/hash/group/distinct. Equality canonical keys are typed value trees: NFC text normalized, instant exact UTC civil ordinal+seconds+fraction including D4 UTC years0/10000 and arbitrary precision, quantity retains unit, others recurse. D3-CJ3 canonical UTF8 includes the type descriptor. When raw values compare equal, distinct/group publishes the lexicographically least `(完整 raw value 的 CJ3 bytes)` representative, independent of enumeration/partition, not first-row wins. Composite-key representatives use the least full raw key tuple actually in each class, never a synthetic combination.

Author Orderable is only bool(false<true), text by Unicode-scalar lexicographic order (NFC uses only normalized key, not raw representation), int64/integer/decimal, same-calendar/version/precision date, instant, same-dimension/unit quantity, and Optional of these. Optional sorting explicitly selects none first/last; some uses item order. semantic_code/ref/list/object/union have no natural order and fail sort/min/max; explicitly project business sort fields. Internal canonical ties do not grant author ordering for these types.

## 3. Arithmetic and aggregation

Each CEL int64 +,-,* checks range after that operation; may_error expressions cannot be reordered into different overflow behavior. integer/decimal +,-,* is exact, without mixed arithmetic. int64→integer→decimal requires explicit functions; decimal never uses binary float. Division is only `divide(a,b,scale,rounding)`, decimal a/b, literal int64 scale0..65535, literal rounding `toward_zero|floor|ceiling|half_even`; zero divisor errors. Compute q=a/b, scale by10^scale, round by the specified mode, divide by10^scale, canonicalize. half_even rounds positive/negative ties toward the nearest even integer. `/` and `%` are absent; future integer division cannot silently truncate.

sum/avg first aggregate all present inputs in unbounded exact intermediates. int64 sum checks range once at the end: `[MAX,1,-1]`→MAX, without partition overflow. integer/decimal sums remain encoding/budget-limited. avg returns decimal with explicit scale/rounding, rounding sum/count once, never averaging partition averages. count returns integer. Resource failures are distinct from numeric_overflow. min/max requires Orderable present values, validating all dynamic bases first and choosing canonical raw representatives for equal values. NFC-equivalent raw strings satisfy x==y, !(x<y), !(y<x); min/max both choose the CJ3-minimum raw representative. sort compares every user value key before internal occurrence keys. Raw representation affects representatives, not a second CEL value order. Empty global aggregate emits one row: all counts0, sum/avg/min/max none. Empty grouped aggregate emits none.

## 4. Weftext CEL subset

CEL is the sole scalar-expression syntax, fixed profile `weftext.cel/1`. Accepted AST: literals, identifiers, static members/indexes, list/object literals, unary !/-, binary +,-,*,==,!=,<,<=,>,>=,&&,||, conditional, listed functions and bounded macros below. Forbid dyn/null/double/uint/bytes, host timestamp/duration overloads, arbitrary comprehensions, map values, reflection, regex, I/O, random, now() and user functions. No JS/SQL supplementation. Native integer literals remain int64; large integer/decimal uses explicit constructors. Engines register exact overloads without changing native int64. Parser/library selection is implementation work, not changes to this table.

Roots: `row.<columnName>` for input schema; `param.<name>`; `scalar.<id>`; `context.now`, `context.timeZone`, `context.calendarVersion`, `context.tzdbVersion` only when inferred dependencies declare them and invocation supplies them. `this` never enters SavedQuery; DynamicBlock lexical references bind call parameters, first resolved by Core to TypedLiteral. CEL performs no Field lookup, ref dereference, Query call or authorization check; operators explicitly handle these.

Object literals have static string keys and the unique bidirectional typing rules below. Lists are homogeneous; empty lists require `emptyList(T)` with compile-time TypeSpec intrinsic T, not type strings. Static member spelling is `.name` or `['literal_name']`; runtime list indexing uses at, no dynamic object member. T denotes checker AST type arguments, not another language; portable inputs remain CEL strings plus Query/Registry typing environment.

| Expression/function | Input→output | Totality |
|---|---|---|
| true/false, text literal, int64 literal | Fixed type | total; out-of-range literal compile failure |
| integer(text literal),decimal(text literal) | Canonical literal→exact type | total; noncanonical compile failure |
| integer(int64),decimal(int64/integer) | Explicit exact widening | total; budget may fail |
| parseInteger(text),parseDecimal(text) | →Optional<exact type> | total; invalid lexical form none, budget not swallowed |
| some(x),none(T) | →Optional<T> | total; static intrinsic T |
| hasValue(optional) | →bool | total |
| value(optional) | →T | may_error, none_value |
| valueOr(optional,default T) | →T | total iff both arguments total; eager arguments |
| at(list,int64) | →Optional<T> | total; negative/out-of-range none |
| size(text/list) | →integer | total; Unicode scalars for text |
| contains/startsWith/endsWith(text,text) | →bool | total; exact scalars, no locale |
| nfc(text) | →nfc text | total; NFC, retain original display value and normalized comparison/matching key |
| contains(list<T>,T) | →bool | total; equatable T |
| codeText(semantic_code) | →text | total; original canonical code, not localized |
| quantityMagnitude(quantity) | →decimal | total; explicit unit removal, pure number without verified-unit promise |
| quantityBasis(quantity) | →object{dimensionId:text,unitId:text} | total; validated same-cut unit contribution, no conversion |
| dateBasis(calendar_date) | →`object{calendarId:text,calendarVersion:text,precision:text}` | total; complete dynamic basis |
| unionIs(union,variant text literal) | →bool | total; variant must be in static type |
| unionValue(union,variant text literal) | →Optional<variantType> | total; other variant none |
| +,-,* | Same int64/integer/decimal | int64 may_error; exact types mathematically total, budget separate |
| divide(decimal,decimal,int64 literal,text literal) | →decimal | may_error; zero division and specified rounding |
| ==,!= | Same equatable type | total iff operands total |
| <,<=,>,>= | Same nonoptional Orderable | date/quantity may_error for dynamic scope; others total iff operands total |
| !,&&,\|\| | bool→bool | Deterministic lazy semantics below |
| condition ? a : b | bool and same-type branches | Deterministic lazy semantics below |

Portable spelling for `none(T)` and `emptyList(T)` is `optional.none()` and `[]` with expected TypeSpec, legal only when the checker uniquely infers that expected type. Mathematical none(T)/emptyList(T) cannot pass TypeSpec as a CEL object argument.

**Unique constructor/expected-type rules.** The checker distinguishes synthesis from checking against a known complete TypeSpec, using one CEL AST and no runtime coercion:
1. A nonempty list without expected type synthesizes each element, requiring identical full item TypeSpec. maximum is exactly syntactic item count, at most65535, never inferred larger from runtime values/consumer budgets.
2. Expected list<T,N> admits a literal only with count≤N and each item recursively checked as T, producing that complete list<T,N>. Empty [] requires this rule. A nonliteral typed list must exactly match; no list<T,3>→list<T,8192> widening.
3. Without expected type, object literals synthesize each member and UTF8-sort schema. With expected object, member sets must match exactly and values check recursively. No implicit missing/extra/optional members or open map.
4. A conditional with expected type checks both branches against it; otherwise independently synthesized branches must be identical. If one branch is contextual []/optional.none(), or constructed only from these/context-only constructors, and the other synthesizes independently, the latter supplies expected type to the former. Reject if neither synthesizes. Never merge two synthesized list bounds; preserve lazy/totality rules.
5. optional.none() requires expected Optional<T>. optional.of(x) requires nonOptional x and returns Optional<T>. at's item and unionValue's selected variant must not already be Optional; reject at compile time, no forbidden nesting/implicit flattening. Other signatures remain; map retains receiver maximum, filter complete receiver type.
6. Expected types come only from specified function signatures/receiver inference, e.g. Optional<T>.orValue's T, declared parameters/TypedLiteral or these constructor/conditional parents. project/derive cannot infer types from labels or introduce casts. Typed expressions require complete equality. int64 literals do not coerce to expected integer/decimal, nor text to NFC. Validate full AST/totality, not samples/ranges/unexecuted branches. Actual some spelling is `optional.of`; hasValue/value/valueOr are `.hasValue()`, `.value()`, `.orValue(default)`. Other functions use exact table names; mathematical shorthand is not an alias.

`&&` skips right after false, `||` after true; conditional evaluates only the selected branch. A left error fails the expression, without CEL commutative error suppression; engines must implement this fixed profile difference. Other arguments evaluate eagerly left-to-right by AST. Totality considers all potential branches, allowing only literal-constant folding to remove unreachable ones, never sample-based claims. related predicates forbid may_error value(optional), divide, int64 arithmetic, etc.; `.orValue` can supply an explicit default.

Text match operands require equal comparison policies: exact raw, NFC normalized keys. Needle types never change implicitly; Search builder explicitly nfc(needle) on NFC paths and retains exact needle otherwise. Core `weftext.cel/1` fixes Unicode15.1 normalization data. A behavior-changing update requires new profile feature/semantic review, not device Unicode. This is an implementation/conformance obligation, not evidence of a library run.

Only bounded CEL spellings `.exists(x,p)`, `.all(x,p)`, `.filter(x,p)`, `.map(x,e)` are allowed, one variable and no index variant. Variables cannot shadow row/param/scalar/context or outer macro variables. Static item types and TypeSpec maxima apply; map retains maximum, filter type/order. p must be total bool; empty exists=false/all=true; total predicates permit short-circuiting without swallowing budgets. map e may error; evaluated items run by original ordinal, any failure fails the expression without skipped items. Error candidates include macro AST path+item ordinal internally; public diagnostics expose only static macro AST location. No infinite iterator; nested expansion uses checked precharging. Macros act only on read values, adding no source rights or subqueries.

## 5. Upstream value boundaries

Calendar dates cannot compare lexicographically across calendars; instants are not limited to nine fractional digits/host ranges; date/instant/range cannot implicitly convert. No CEL functions expose quantity conversion, date addition, anniversaries, holidays or other algorithms awaiting D10 admission. Recurrence uses its Query operator, not CEL expansion. Algebra§11 DerivedPeriodRange uses existing object/union/calendar_date types; ISO civil arithmetic is frozen inside that adapter without CEL constructors, object→date coercion or date addition. Saved computed facts may be queried with versions/origins retained; Query does not write missing facts.

D4 actual relationship endpoints retain full target read/state gates: unqualified occurrences are wholly invisible, not a cleared target retaining role/count. Ordinary nonrelation typed Refs and authored provenance follow §5.1. Read Refs never authorize automatic target titles/resource bytes.

### 5.1 D4 authored provenance and positional data

Fully validate D4 atoms using actual containing owner, Registry and original D3 decoder before structural mapping. Never repair refs/Locators, remove atoms or guess owner/revision. Existing object/union/Optional/Ref types suffice; no Locator primitive, generic target or CEL resolver. Positions are authored saved values; internal derived row Locators/Provenance do not use this outlet.

P is fixed union, ordered external,node,resource,transform; each value is the closed object below. Schema names sort by UTF8. All text is exact. D3 nonnegative integers become canonical int64 strings after original D3-domain validation, never host float.

| D4 atom→P variant | Object members |
|---|---|
| external | scheme:text,value:text,observedAt:Optional<zoned_instant>; original scheme authenticity/availability validates despite text projection |
| node | nodeRef:NodeRef,`locator:Optional<union{element:L_element,range:L_range}>`; original D3 kind selects element/range, never interchange |
| resource | resourceRef:ResourceRef,regionLocator:Optional<L_region> |
| transform | inputIndex:int64,operationId:text; preserve array order/backward indexes/valid OperationId, not a content ref |

Position projections preserve only original members, not duplicate kind: L_element={owner:NodeRef,documentRevisionToken:text,elementKind:text,sourceSpan:Span}; L_range={owner:NodeRef,documentRevisionToken:text,sourceSpan:Span}; L_region={resourceRef:ResourceRef,resourceRevisionToken:text,regionToken:text}. Span={startLine:int64,startColumn:int64,endLine:int64,endColumn:int64}. Original D3/D4 gates validate elementKind, coordinate ordering, revision/region tokens and atom/Locator owner equality; visual similarity is not positional equality. Absent optionals→none, present→some. Provenance maximum16 preserves ordinals; omitted Entry provenance→[]; present must first satisfy1..16, never empty.

P/L_* are structural abbreviations, not TypeSpec kinds/schema identities/D3 aliases. Existing recursive equality/hash/group/distinct applies, no natural order. Queries may read members and Views display them. Clicking or reconstructing Action Locators requires original D3 resolver plus current D7 authorization and exact owner/kind/revision; projection does not prove present validity. Historical authored positions remain historical, never rebound latest or reissued as current ActionEvidence.

Authorized authored Ref/Locator values and current target states are different read objects. For a fully authorized Field/source, original D3/D4 closed decoding preserves complete authored Refs, historical Locators/tokens/coordinates without target Workspace/existence/lifecycle/title/content/position lookup or current-validity claims. Authority comes from the complete fact/source, not target rights. This applies to ordinary nonrelation values and existing provenance, including cross-Workspace Node and valid owner-local Resource provenance. AnnotationRef only occupies existing ordinary schema positions; no Annotation provenance atom is added. D4 domain/locality remains.

Actual endpoint domain/state, relationship visibility/incidence, graph/traversal/join and click resolution still require current target-state/content and applicable locator rights before reading. D4 distinguishes actual endpoint from ancillary provenance; a node_ref in JSON does not merge roles. Renderers cannot add resolved flags/current titles/thumbnails/states/positions. State-dependent Query/Action explicitly includes original potential scope, actual dependencies and CAS, not merely a narrow author-byte proof.

The rule spans read, Field selection/evidence, preview, effects and replay. Lost Field/source rights block disclosure under current gates. Only losing referenced-target rights preserves still-authorized authored values while preventing resolution. No stage removes Locators/atoms, substitutes none or changes transform inputIndex. Internal occurrence/source lineage stays private.

### 5.2 Complete D4 constructor and qualifier bridge

Input is a fully validated Entry/Registry binding with owner/raw bytes, not arbitrary JSON to unwrap. D4 availability/type/range/cardinality/relationship/locality/nonempty/bound/contribution gates precede mapping. Mapping defines Query types/encoding without replacing those gates. Values inherit actual positive/negative dependencies of Field/alias/QualifierSet, namespace owner, RegistryBinding, comparator/tzdb/unit/code/external scheme and recursive refs/Locators. Source/authorization dependencies continue through publication/cache/Action revalidation.

| D4 constructor/schema | Unique Query TypeSpec and V mapping |
|---|---|
| text | exact→text; nfc-for-compare→comparison=nfc; raw text V, nonEmpty input-only validation |
| boolean | bool, original Boolean |
| integer | integer, original canonical string including bounded/excluded-value schemas; never int64 downgrade |
| decimal | decimal, original canonical string including bounded schemas |
| semantic_code | semantic_code with ResolvedCodeScope below, original code V |
| calendar_date / zoned_instant / quantity | Same-named primitive, full original closed wire; Registry evidence adds no hidden members |
| date_range / instant_range | `object{end:optional(point),start:optional(point)}`, bound-by-bound; unbounded none, not absent whole range |
| `node_ref / resource_ref / annotation_ref` | Same-named primitive, complete original nodeRef/resourceRef/annotationRef member |
| external_identifier | object{scheme:text,value:text}, original members, no internal SourceBinding |
| object | Recursively bridge members; required direct, nonrequired one Optional; V from members, absent none/present some |
| union | Original ordered variant tokens, recursively bridge; V={variant,value:bridge(original.value)} |
| bounded_set / bounded_sequence | list with original schema maximum, bridge each item retaining order/multiplicity; no NFC deduplication |
| alias_ref, schema-only | D4 bounded acyclic expansion then recursive bridge; retain alias identity/version dependencies, no public kind |

ResolvedCodeScope is D7's closed type-equality description, distinguishing `{kind:"field_local",fieldId,codes}`, `{kind:"namespace",namespaceId}`, `{kind:"contribution_set",codes}` with full original CodeScope data, and `{kind:"registry"}` only for D4 relation status qualifiers. codes retains sorted unique domain. registry means all currently verified complete SemanticCodeIds, not arbitrary text; original status owner/contribution preflight remains. Types grant no contribution availability; dependencies retain Registry binding. Every mention of resolved D4 CodeScope means exactly this wire, not implementation-added members.

QualifierSet expands to closed Q with UTF8-sorted members: `fact={selection:Optional<text>,validity:Optional<union{date:dateRange,instant:instantRange}>}`; `event_assertion={confidence:Optional<decimal>,eventTime:union{date:calendar_date,instant:zoned_instant},selection:Optional<text>}`; `observation={confidence:Optional<decimal>,observedAt:zoned_instant,selection:Optional<text>}`; `relation={status:Optional<semantic_code scope registry>,validity:Optional<union{date:dateRange,instant:instantRange}>}`. selection accepts only ordinary/preferred/deprecated, confidence0..1, time/ranges pass D4 first. Selection is text, not fabricated Field-local code. Omitted note→none, valid nonempty note→some original text; present empty remains invalid, distinct from recurrence-replacement's other-layer empty-note allowance. Provenance maps fully under §5.1.

### 5.3 Explicit quantity and date basis

Quantity basis is full dimensionId/unitId of the currently verified unit contribution; same dimension/different unit remains incompatible. `quantityBasis` and `dateBasis` inspect decoded values and bound validation descriptions, without source/network/new Registry reads. Equal quantityBasis supports grouping; equal dateBasis is the full dynamic prerequisite for comparison. One invocation has one RegistryBinding, so a basis selects one verified comparator. Different-binding results cannot mix directly.

sum/avg additionally accepts same-typed quantity→Optional<quantity>. First inspect all present bases for exact equality or type_mismatch, not first-value assumptions. sum exact magnitudes, avg rounds once by original scale/rounding; retain unitId and bound dimension. No present values→none. min/max validates basis before magnitude comparison. Explicit quantityMagnitude→numeric aggregation is valid but intentionally unitless; unitLabel text cannot promote it to verified quantity. No conversion rules/new author Fields are needed.

DerivedDuration is only the UTF8-ordered closed union: calendar_units→object{calendarId:text,calendarVersion:text,precision:text,units:integer}, D3Integer to canonical string; exact_seconds→object{seconds:decimal}; unavailable→object{reason:text}, reason only open_range. Preserve D4 algorithm/precision/unavailability; this is not writable author TypedValue. Other provider/schema failures are definition_unavailable, not open_range. D4 valid absence and Query Optional remain distinct.

## 6. Current production versions and read-only authored values

All TypeSpec/V/CEL functions, totality, arithmetic and provenance bridges above remain unchanged. SourceVersion/2, SourceObservation/1, SourceVersionRef/1 and DependencyProof/2 remain execution/preparation evidence, not new value kinds. Authored Locator/tokens remain verbatim under §5.1: displaying them consumes no current-position qualification and never mutates/reissues the author value. Actual resolution uses D3's stable-production-address/current-Observation algorithm and returns only that new read result; target state/content/profile still needs normal authorization. Such a read is not ActionEvidence and never upgrades old result/preparation inputs. Equal Counter, structure, CEL equality or bytes do not prove production-version equality.

Complete actual Registry/calendar/tzdb/unit/code/external-scheme bindings come from same-cut immutable D4 ValidatedCatalogContext/RecurrenceReadContext and enter actual D6 dependencies. Cells/type labels/context strings cannot fabricate trusted contributions. Retained old mathematical results may display only with original result's current authorization/epoch/deadline intact; display does not upgrade old inputs to new Action evidence.
