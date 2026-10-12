---
source_language: zh-CN
translation_of: SOURCE.zh-CN.md
translation_status: synced
---

[简体中文](SOURCE.zh-CN.md)

# Typst author source and ordinary business properties

Status: **coordinated design candidate; neither implemented nor independently accepted**. Fixed input: `af6737a057c7bb69763dfaad0a57da4652e085f3`; author branch: `design/typst-native-20261012`. This document owns the root declaration, native values, schema and origin projection of `typst_source/1`. [CORE](CORE.md) owns storage/transactions; [EDITOR](EDITOR.md) owns presentation/editing; [FEATURES](FEATURES.md) owns links, Query, templates, annotations and outputs. These are named interfaces of one candidate, without combining old AsciiDoc decoders. Acceptance below describes design oracles, not passed product tests.

## 1. One native source

The exact root `.weftext-format` bytes are `weftext.typst.v1\n`; the managed shape is `X/X.typ`, with one primary source per Node. `TypstProfileCarrier/1` fixes `profileId=weftext.typst.v1`, `documentLanguage=typst`, `languageBaseline=0.15.1` and `sourceContract=typst_source/1` in the same cut as CORE's format binding. Title, path, directory tree and filename do not declare identity. The body uses the complete fixed official Typst language. A title is optional; `title` and `heading` retain their separate native meanings.

```typst
#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
  types: (),
)) <weftext>

#title[Project notes]

= Research
Use native Typst for body text, math $x^2$, lists, tables and import/set/show.
```

Preserve exact bytes, including UTF-8 BOM, comments, whitespace, LF/CRLF/CR, literal spellings and unknown content. D6 strict UTF-8 decoding is the physical admission step. Official `typst-syntax` is the sole syntax parser: AST validation checks application roles and uses native token/AST decoding and ranges, without another lexer, dictionary parser or interpreter. Do not strip a header and reset body offsets.

New notes generate no YAML front matter, JSON Entry block or alternative backlink syntax. Native `json`/`yaml` loading in the body, resources, independent configuration, annotations and internal protocols retain their own permission/format contracts. There is no dual native/JSON business carrier or automatic fallback.

## 2. The unique actual root declaration

`<weftext>` is a reserved application role on an exact, case-sensitive native label. Authenticate the root source AST declaration, not a rendered element. Apply these rules in order:

1. Officially parse the exact complete root file. The declaration is a direct native `#metadata((...))` call followed by its `<weftext>` label at the same root markup level, with only whitespace/comments between them. The target is bare identifier `metadata`, with exactly one positional dictionary AST argument and no named arguments, spread, content block or extra arguments. The call cannot be wrapped in `context`, a condition, code block, loop, function, show rule or `include`. It need not be the first expression: it may precede/follow a title or follow ordinary static body text; rendered order never selects it.
2. Prove that the fixed standard library supplies the actual `metadata` binding at this location. Use the official AST to validate the preceding root lexical prefix: BOM/trivia, pure markup text/formatting/headings/raw, standard `#title[...]` with no embedded code, and standard set rules whose arguments contain only static literals/standard values and no nested call or binding (for example `#set text(size: 12pt)`) are supported. The fixed library verifies these standard calls/values. Lengths and other native values in this prefix do not extend Field `types`. Other preceding let/import/include, arbitrary calls, show/context, assignment or unknown binders make binding proof unavailable in this version; do not execute them and guess. Host inputs cannot replace standard bindings. A later legal `let metadata = ...` does not alter an already authenticated declaration or gain control authority. This prefix support set limits control proof, not the source language, and introduces no second parser.
3. Traverse actual Label nodes throughout the complete root AST: there must be exactly one `<weftext>`. A second matching label is ambiguous even with the same dictionary or in an inactive branch/function body. Neither first nor last wins. Matching text inside strings, comments and raw code is not a Label. Other labels and FEATURES' local reserved roles are not root declarations.
4. The root dictionary has direct named/keyed entries, allowing static native string keys. Decoded keys are exact and unique: no spread, dynamic keys or duplicates, including `id:` alongside `"id":`. `id` is required. Only `id` and `types` are root control keys; all others are business keys. The `weftext-` prefix is reserved for future named roles and cannot be assigned to business schema. Display labels, case folding and Unicode normalization do not merge keys.
5. `id` must be a direct native Str AST whose decoded value is an exact lowercase UUIDv4, without trimming, case repair, interpolation, variables or evaluation. Missing `types` means an empty declared set. If present, it must be a direct native Array AST of direct Str values: `()` is empty, `("person",)` is a singleton. String, `none`, concatenation, spread or duplicate expanded types do not mean an empty set.

Multiline declarations, native comments and trailing commas are supported; these rules introduce no source grammar. An ordinary legal `.typ` without this application role remains openable, editable and compilable but gains no managed Node admission. Bad controls in an existing managed source retain CORE's raw/Draft/explicit repair path, without inventing identity or deleting source.

Labels in imported/included files, metadata returned by functions, `label("weftext")` and compiler introspection results are not actual root-authored declarations. Rendered duplicate labels may produce compilation errors or ordinary diagnostics; they do not change root control data. Never authenticate identity through `query(<weftext>)`, PDF/HTML, hidden elements, compilation success or a final show result. If the complete root AST cannot exclude duplicates, for example because damaged syntax leaves structure uncertain, report diagnostic `control_unavailable` and do not guess uniqueness. Compilation-only failures such as an unknown body function do not block literal reading from a complete AST.

`TypstRootDeclaration/1` freezes the root pin/full SourceObservation, exact UTF-8 declaration/dictionary/ID/types ranges, ID, authored types order and structural qualification. Invalid output freezes diagnostics of the same source and publishes no effective control qualification. This is a derived result inside CORE's evaluation, not permanent identity evidence, separately editable metadata or a new signature token. Reading an ID does not bypass admission when controls are invalid. §8.1 defines the exact closed encoding.

## 3. Stable identity and types

NodeRef remains WorkspaceId `+` NodeId. The source ID is an authored declaration; original Core/D3/D6 authority supplies birth, file binding, tombstones, non-reuse, ACL, execution responsibility and effective-format evidence. Contradiction between source ID, protected binding and birth is conflict/incomplete, without choosing one to overwrite the other. Missing metadata, partial synchronization or a bare `.typ` never permits indexers to allocate identity again.

Creation, template instances, import as a new note and fresh copy/fork Nodes use the UUID from the original single private candidate map. Move/rename retain identity; restore consumes the actual historical binding. CORE constructs `TypstControlConstruction/1`/`TypstDocumentConstruction/1` from the same official AST and exact before pin. Existing ID edits touch only its Str span; type edits touch their array span. A missing declaration is inserted only by an explicit new/create/import/repair full preview, without adding a second declaration. Templates cannot prefill final IDs. Source, format, birth, reference mapping and SourceRevisionPlan install in one CAS/P; retry/unknown reuse frozen map, UUIDs and bytes/pins.

`types` is an unordered set without precedence; authored order only supports lossless editing. Central schema expands exact `aliases` to `namespace/type-name`; full names remain accepted and localized labels are not aliases. An alias and full `name` expanding to the same type are duplicates. Unknown legal tokens remain preserved with unavailable schema; they are not deleted, treated as empty or rebound by installation order. Invalid tokens are classification errors. Preserve `D4` §2's exact ASCII semantic identifier domains, owner authentication and ledger evolution. Source dictionary keys occupy a separate native key domain.

`template` is a reserved Core meta-kind token, cannot be rebound by plugins or combined with business types; authorized Recipes declare instance target types. Tasks remain ordinary Nodes with explicit `task` membership, without TaskRef, checklist mirrors or another lifecycle. ID/types are not business Fields and are unavailable to ordinary field writers, bulk `fields`, automation Field actions or import's business mapping. Type changes validate the full effective closure and actual Source/control permissions. UI grouping and prefixes cannot assign types.

## 4. The exact native value set

`TypstNativeValue/1` is an internal native-value projection domain, not tagged JSON for authors. It supports exactly the following recursive values without reducing the full language:

| Native authored value | Projection and boundary | Default authored output |
| --- | --- | --- |
| string | Official Str-decoded Unicode; empty differs from missing; no trim/NFC rewrite or numeric `phones` | Native quoted/escaped string |
| bool | true/false; no 0/1, strings or case aliases | Native keyword |
| int | Official signed i64; actual native Int literals/unary signs and qualified standard `int("-9223372036854775808")`; no bool/float coercion | Decimal Int literal when type/value round-trip, otherwise qualified int(canonical string) |
| float | Finite native binary64 literal/direct unary sign; `preserve` IEEE-754 64-bit pattern including negative zero internally, without JSON-number loss; NaN/Infinity have no Field projection | Native float literal round-tripping the same bits |
| `decimal` | Qualified standard `decimal("3.14")` or `decimal(59)`; exact native value, no float cast; out-of-range construction is an error | Native decimal constructor |
| `duration` | Qualified standard duration with unique named weeks/days/hours/minutes/seconds and direct Int constants; native constructor determines representability; calendar months/years are not seconds | Native duration constructor round-tripping exactly |
| `none` | Explicit null value, only valid in nullable schema; not missing/default | `none` |
| `datetime` | §5's standard constant constructor; date/time/local_datetime remain distinct | Same-shape `datetime` constructor |
| array | Native `()`, `(value,)`, `(value, value, ...)`; ordered, duplicates preserved; recursive supported values | Native array |
| dictionary | Native `(:)` or static key/value entries; exact unique string keys; recursive supported values; preserve authored order | Native dictionary |

Official parser/native value APIs decode literals. A unary sign applies only to a numeric literal; overflow is an error and binary expressions are not interpreted. Standard int(Str) accepts exactly one direct canonical signed-decimal string within i64, verified by the official constructor; no floats/other expressions. Official `-9223372036854775808` is a float despite its spelling. Writers verify both native type and value; i64::MIN emits `int("-9223372036854775808")`. Duration parameters accept this same qualified Int constant. Bare identifiers are not strings; `(value)` is grouping, not a singleton array. Comments/trailing commas remain native. Dynamic keys, spread, variables, user functions, arithmetic, member access, content, lengths, angles, colors, labels, regex, module, bytes, auto and values outside the table have no authored business projection in this version. Int(Str)/datetime/decimal/duration are decoded only with proved standard bindings and constant arguments through official constructors, without arbitrary function execution. Other legal constructs retain native behavior and Source access.

Project each top-level Field independently. A supported literal is `available`; violation of known schema is `invalid`; unknown schema/owner is `schema_unavailable`; a legal expression outside the static domain is `computed_unavailable`. One unsupported array/dictionary member makes its entire Field unavailable, without truncation. A computed neighbor does not prevent reading literal phones/status. A validation budget failure is unavailable, not partial successful data.

The compiler may separately produce current `derived_readonly` display values, pinned to full environment/dependencies/origins. Such display is not an authored Field or write target/default; Query/Action cannot treat computed_unavailable as verified business facts. No compilation result means unavailable. Adding computed Fields to typed Workspace Query requires a named producer/currentness contract, not final metadata introspection. This candidate retains D5/D7 computed columns/formulas and native body computation through FEATURES.

A missing key is absence. None, empty string, empty array/dictionary, zero and `false` are different actual values. Empty contacts mean zero contacts and do not satisfy non-empty constraints. Required members require actual presence; nullable only permits `none` when present. Invalid values never become absence or disappear behind a default. Defaults generate actual source values during creation/template entry only, never on reading existing source. Unsaved UI suggestions do not become Query, sorting, relation or export facts.

## 5. Dates, `time`, precision and references

Native datetime supports `date`, time and combined datetime without an authored timezone member. [Official datetime](https://typst.app/docs/reference/foundations/datetime/)

- date: exactly year/month/day direct Int parameters, a real Gregorian date, schema range 0001..9999 and day precision, without timezone.
- time: exactly hour/minute/second, ranges 0..23/0..59/0..59 and second precision, without date/timezone.
- `local_datetime`: exactly all six parameters, the same date/time ranges and second precision; a civil time, never implicitly an instant.

Native parameter order is unrestricted, but duplicates, positional arguments, spread, expressions and omitted precision components are unsupported. §2's lexical-prefix proof and fixed library qualify standard bindings, never a user function. `datetime.today()` and arithmetic are legal computed_unavailable Fields. Official constructor validation determines real validity, without another date parser. `due: datetime(year: 2026, month: 10, day: 20)` is a date, not Shanghai midnight or inferred UTC.

An unknown `birthday` year uses `birthday: (month: 5, day: 12)`. The `month_day` dictionary semantics require Int month/day, validate month 1..12 and the month's maximum day, and permit February 29 without guessing a year. It forms a disjoint union with date. Do not fabricate a year, epoch or current-host year. Calendar view policy decides annual display of February 29 without changing source.

`instant` uses a native string and explicit RFC3339 schema: date/time plus `Z` or an explicit offset, no `-00:00`, leap second or `absent` offset, with exact fractional seconds. Preserve `D4` §4.1's precise RFC3339/UTC comparison and original offset bytes. Recurring civil events needing IANA zone/tzdb explicitly use a schema dictionary with zone and frozen version through FEATURES' temporal service, never guessing from datetime, host locale or display preferences. Other calendars, month/year precision and ranges need dedicated ordinary dictionaries, not a false day-datetime equivalence. Ordinary task due is a disjoint date/native-string instant union and retains arbitrary fractional instant precision.

Prefer native `decimal("3.14")` for ordinary exact decimal amounts. Fixed official 0.15.1 defines its precision/range; qualified constructors accept one direct string/int and reject float casts/computed parameters. Compare exact native values while preserving raw/scale for source editing, without silent rounding. Original arbitrary-precision `D4` requirements beyond native representability explicitly use `decimal_text` canonical strings and original exact comparisons; imports must not approximate them into decimal/float. [Official Decimal](https://typst.app/docs/reference/foundations/decimal/)

`duration(weeks: 1, days: 2)` represents elapsed duration, not civil months/years. Named parameters are unique Int constants and require official constructor success. First check each unit conversion and every intermediate sum in the official seconds→minutes→hours→days→weeks order; intermediate overflow rejects static Field qualification even if final cancellation would fit. Do not claim the official constructor is checked. The default writer uses duration(seconds:qualified total Int) and verifies exact round-trip. This constant-constructor subset is exact whole seconds in native time::Duration, transported as canonical signed i64 seconds; consumers must not infer internal precision from the floating-point .seconds() display method. Explicit `exact_seconds_text` handles arbitrary fractional exact elapsed values beyond this subset. Quantities, ranges and identifiers use named schema dictionaries of native values, without any/executable/JSON escape hatches. [Official Duration](https://typst.app/docs/reference/foundations/duration/)

Business references use schema `node_ref` with native string `weftext-node://<WorkspaceId>/<NodeId>`. FEATURES owns the exact URI/shared Ref decoding. Business relations have no fragment; body links may have a qualified fragment. `company: "云山公司"` remains text without automatic Organization resolution; explicit `company_ref` uses the stable URI. Core/FEATURES retain resolution, ACL, unavailable/Trash, cross-Workspace copy/fork and inbound proofs. Names do not establish relationships. Phone/email/URL validation never automatically dials or sends messages.

## 6. One central schema and multiple types

Schema is separate, portable, authorized configuration, without replication in every note. CORE's same-cut `registry` dependency carries its storage, digest, version, authentication and evolution. `TypstSchemaSnapshot/1` is a named profile decoder, not old RegistrySnapshot/1 or /2. Its members are kind, version, generation, digest, predecessor, fields, types and aliases as closed in §6.1. Digest covers the complete canonical object except itself; generation/digest bind together. Predecessor is authenticated bootstrap or the previous complete binding. Internal configuration may be JSON; authored `.typ` need not be.

Fields are unique/sorted by FieldId and define key, labels, valueType, nullable, enum and default. Types define unique TypeId, labels, FieldId/required pairs, requires and conflicts. Aliases map exact names to TypeIds. Display labels never determine keys. Unknown members, malformed containers, duplicates, missing references, cycles, asymmetric conflicts, one key assigned to different Fields and identity/digest replacement reject schema installation rather than form an empty configuration.

ValueType consists of §4/§5 scalar semantic names or recursive array, dictionary and disjoint union configuration objects. These are configuration algebra, **not new authored functions**. Dictionary members centrally define key, type, required and nullable; additional members are preserved as `unbound`, excluded from known constraints/member actions. A union needs at least two uniquely distinguishable branches (such as datetime date and native string instant); overlap is a schema conflict, never first-match. Limits, enums and non-empty requirements come from verified schema, not labels or examples.

Legal business keys may be arbitrary exact strings. Default generation uses native identifier keys such as `phones`; other keys use native quoted strings, for example `"office phone"`. Case/NFC differences do not create aliases. Reserved controls are never Fields. Unbound top-level keys remain native unbound data editable through Source, without claiming namespaces or deletion. Only the selected value token may change noncanonical native spelling.

Expand types through the verified requires DAG, without source-order precedence. Union Field sets; a FieldId has one central definition/source key, and requiredness combines by OR. Nullable/enum/default/type cannot be redefined by each consuming type. Conflicting key→FieldId/type/default/semantics contributions reject installation, without intersection, last-wins or duplicate source values. Effective closure conflicts invalidate classification; unknown declared types make relevant validation unavailable, never complete success inferred from known types. An untyped note may retain bound properties without automatically becoming a Person/Task.

UI grouping may show one Field in multiple contact/task groups, but it remains one value. Folding, hiding, language and installation order do not change values, permissions or membership. Key/alias/definition changes preview affected sources, freeze schema before/after and require authorized evolution. Retain original semantic ID immutability, owner tuples, historical retirement/tombstone and bytes obligations. Simplified fields use the new profile schema domain without changing the old 61 Field/7 Facet catalog or claiming migration.

### 6.1 Closed configuration, native values and qualification encoding

The following fixes actual internal encoding, superseding §6's parenthesized algebra shorthand. Objects are closed; reject unknown members/versions. Counter, PinRef/2, NodeRef, SourceObservation/1, SourceVersion/2 and DependencyProof/4 reuse CORE's exact original types. Text is not trimmed; Digest is 64 lowercase SHA-256 hex digits. Semantic IDs use §3's domains. Sets sort by unsigned UTF-8. Check container/count overflow and budgets before allocation.

```text
TypstNativeValue/1 =
  {kind:"string",value:text}|{kind:"bool",value:Boolean}
 |{kind:"int",value:canonical-signed-decimal-i64}
 |{kind:"float",bits:lowercase-16-hex-IEEE754-binary64}
 |{kind:"none"}
 |{kind:"decimal",negative:Boolean,coefficient:canonical-unsigned-decimal-u96,
    scale:integer(0..28)}
 |{kind:"duration",seconds:canonical-signed-decimal-i64}
 |{kind:"date",year:integer(1..9999),month:integer(1..12),day:integer(1..31)}
 |{kind:"time",hour:integer(0..23),minute:integer(0..59),second:integer(0..59)}
 |{kind:"local_datetime",year:integer(1..9999),month:integer(1..12),
    day:integer(1..31),hour:integer(0..23),minute:integer(0..59),second:integer(0..59)}
 |{kind:"array",values:[TypstNativeValue/1...]}
 |{kind:"dictionary",entries:[{key:text,value:TypstNativeValue/1}...]}

TypstValueType/1 =
  {kind:"scalar",name:"string"|"bool"|"int"|"float"|"decimal"|"duration"|
    "date"|"time"|"local_datetime"|"instant"|"decimal_text"|
    "exact_seconds_text"|"node_ref",nonEmpty:Boolean}
 |{kind:"array",items:TypstValueType/1,minItems:Counter,maxItems:Counter|null}
 |{kind:"dictionary",members:[{key:text,valueType:TypstValueType/1,
    required:Boolean,nullable:Boolean}...],additional:"preserve",semantic:"object"|"month_day"}
 |{kind:"union",variants:[TypstValueType/1...]}

TypstSchemaBinding/1={version:1,generation:Counter,digest:Digest}
TypstSchemaSnapshot/1={kind:"weftext.typst.schema",version:1,generation:Counter,
  digest:Digest,predecessor:{kind:"bootstrap"}|{kind:"previous",binding:TypstSchemaBinding/1},
  fields:[{fieldId:FieldId,key:text,labels:[{language:text,text:text}...],
    valueType:TypstValueType/1,nullable:Boolean,enum:null|[TypstNativeValue/1...],
    default:{kind:"absent"}|{kind:"literal",value:TypstNativeValue/1}}...],
  types:[{typeId:TypeId,labels:[{language:text,text:text}...],
    fields:[{fieldId:FieldId,required:Boolean}...],requires:[TypeId...],conflicts:[TypeId...]}...],
  aliases:[{name:text,typeId:TypeId}...]}
TypstSchemaEvolution/1={version:1,previousSnapshot:TypstSchemaSnapshot/1,
  currentSnapshot:TypstSchemaSnapshot/1}
TypstSchemaQualification/1={version:1,snapshot:TypstSchemaSnapshot/1,
  binding:TypstSchemaBinding/1,evolution:TypstSchemaEvolution/1|null,
  dependencyProof:DependencyProof/4}
```

Float bits encode the sign/exponent/fraction 64-bit integer in network order, most significant hex first; only finite values qualify. Keep positive/negative zero source values distinct while numeric comparisons follow native semantics. Int text is canonical `0` or signed nonzero decimal without leading zeros, within i64. Decimal coefficients are 0..79228162514264337593543950335 with native sign/scale, never JSON numbers/floats; constructors must exactly recover the value/representation. Duration only covers whole seconds from the qualified Int constructor subset, with checked native total representability; floating-point `.seconds()` is not equality evidence. Date/time transport also requires real date/native constructor validation: the wire day bound does not accept February 31. Dictionary entries keep authored order and unique keys; arrays keep duplicates. Internal tags are never emitted into `.typ` and do not reintroduce authored Entries.

nonEmpty may be `true` only on string/instant/decimal_text/exact_seconds_text/node_ref; other scalars fix false. It does not trim. Semantic string types become business values only at typed schema positions. month_day dictionaries require exactly required, nonnullable Int month/day members with additional=preserve; validate known month/day under §5 and preserve unknown members as unbound. maxItems=null means no schema quantity cap, while explicit Core budgets remain finite; min≤max. There are no executable type trees or recursive aliases. Fixed operation depth/byte budgets cause unavailable, not empty data.

Labels sort uniquely by explicit language tag; labels are never aliases. Definitions and nested members sort uniquely by ID/key; requires/conflicts are sorted sets. Enum is null or nonempty, unique and valid under full Field semantics (valueType plus nullable), allowing none in an enum/default when nullable=true. Dictionary members similarly validate against member valueType plus nullable. Literal defaults must pass all schema validation. A nullable Field permits `none` only when nullable=true and its enum is null or includes none. Union branches cannot overlap; do not guess branches under unknown schema.

Qualification matches both binding generation/digest. Bootstrap is generation=1 with evolution=null. Other generations require the complete evolution: previousSnapshot matches predecessor.binding, currentSnapshot equals snapshot, and generation increments with overflow checks. Digest is SHA-256 over D3-CJ/3 encoding excluding digest. Do not add fields to old snapshot bytes. Installation first requires CORE's real registry owner authentication/administration authority. The same-cut registry dependency proof covers the installed schema, binding/evolution and actual pins; its authorization item covers current Policy. A client-provided true or self-consistent snapshot proves no authority. Do not expand D10 signatures or old Registry decoders.

Within the new schema domain, Field key/type/nullable/enum/default and Type fields/requires/conflicts remain immutable under an ID; semantic changes require a new ID. Explicit evolution may change display labels. This version rejects definition deletion pending a named retirement/migration control design; it never truncates historical ledgers or resurrects IDs. Additions without conflicts and new aliases are supported; existing aliases cannot be rebound. Initial target-schema bootstrap is design establishment, not claimed runtime migration of prior drafts.

### 6.2 Unavailable results by operation

| Input state | Raw/Source and disjoint ordinary save | Creation or typed save touching the Field | Complete Query/Field Action/typed export |
| --- | --- | --- | --- |
| Unknown type/schema owner or schema_unavailable Field | Authorized reading; preserve untouched bytes; disjoint body edits may save with typed partial/unavailable | No complete typed creation claim; raw/import repair is separate; ordinary typed Field writes do not pass | Unavailable at actual consumption, without omitted rows/Fields presented as complete |
| Legal computed_unavailable Field | Whole-source reads/writes preserve expressions; literal neighbors remain available | Whole-source save retains legal expressions with computed state; Field actions cannot edit results | Authored Field consumer unavailable; derived columns/body computation use FEATURES' separate contracts |
| Known local invalid/requiredness failure | Draft/exact repair remain readable; untouched errors are not washed into complete | Typed ordinary creation/save rejects and retains Draft; explicit repair is not typed success | Reject/invalid diagnostics, zero business writes |
| Body compilation/resource failure with complete controls/literal Fields | Source and independent Fields remain readable/saveable; compilation marked failed | Core may save valid local Fields; missing display is not fabricated typed failure | Body-independent Query may succeed; rendering export unavailable |

Every path still checks actual permission, real versions and applicable proof obligations. This matrix neither grants hidden reading nor bypasses denies. Raw import may preserve unknown content without claiming complete typed copy/fork semantics.

## 7. Ordinary contacts and tasks

One new target `TypstSchemaSnapshot/1` domain defines internal types `people/contact`, `organizations/contact` and `tasks/task` with aliases `person`, `organization` and task. Full schema identity includes `(schema binding, semantic ID)`; identical ID bytes do not mutate an active old catalog. Authors need not write internal IDs. Shared contact Fields use central `people/contact-*` definitions, Organization-only Fields `organizations/contact-*`, and tasks `tasks/*`. Preserve the old catalog's owner/ledger/domain/bytes without claiming deployed migration.

```typst
#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
  types: ("person",),
  name: "Zhang San",
  company: "Cloud Mountain Company",
  department: "Research",
  position: "Engineer",
  birthday: datetime(year: 1990, month: 5, day: 12),
  phones: (
    (number: "010-12345678", label: "Work"),
    (number: "010-12345678", label: "Personal", note: "Evenings only"),
  ),
  emails: ((address: "zhang@example.org", label: "Work"),),
  addresses: ((address: "Haidian, Beijing", label: "Work"),),
  website: "https://example.org",
)) <weftext>

#title[Zhang San]
= Contact notes
Write conversations, history and details in the body.
```

Person has required nonempty string `name`; optional `company/department/position/website` strings; optional birthday date or month_day; `phones` is an array of dictionaries with required nonempty string number and optional label/note strings. Emails and addresses similarly require nonempty string address. Arrays may be absent/empty and retain identical values. Number need not be E.164 and is never trimmed/rewritten; leading zeros/+ remain text. Labels are ordinary text without default semantic-code identity; notes are plain remarks, not interpreted provenance/assertions. Contact name and document title are independent; creation may suggest copying one without subsequent mandatory dual writes.

```typst
#metadata((
  id: "0efb7d4e-6248-451c-b747-692f1ae9b1cb",
  types: ("organization",),
  name: "Cloud Mountain Company",
  website: "https://example.org",
  phones: ((number: "010-12345678", label: "Reception"),),
  emails: ((address: "info@example.org", label: "Public"),),
  addresses: ((address: "Haidian, Beijing", label: "Office"),),
)) <weftext>

#title[Cloud Mountain Company]
Write business information, contacts and organization notes in the body.
```

Organization requires nonempty name and has optional website, with phones/emails/addresses sharing the exact Person Field definitions. Type membership never changes array structure. The file-tree parent is D3 structure. A business `parent_ref`, when explicitly installed through dedicated schema, is a separate relationship; neither derives the other. Employment details, organizational hierarchies and graphs are not implicit constraints of company text.

```typst
#metadata((
  id: "e8ec9521-901f-4e5f-b4a3-151302967ac8",
  types: ("task",),
  status: "open",
  priority: "normal",
  due: datetime(year: 2026, month: 10, day: 20),
)) <weftext>

#title[Prepare the project report]
Organize findings into a project report.
```

Task requires `status` string enum open/done/cancelled; priority is optional string enum low/normal/high/urgent; due/start are optional date or explicit instant strings. Preserve original status/priority domains. A creation template may explicitly write status=open; it is not a read default. Types/status supply the real task-view producer. Native body checklist/list occurrences do not establish a second durable Task. Dedicated FEATURES schemas/proofs govern dependencies, relations and Calendar events; ordinary due is not a complete recurrence event.

### 7.1 Concrete base target-schema bindings

The following fixes the base configuration instead of inferring it from examples. All Fields have nullable=false; all defaults are absent except status; enum=null except task enums. `contact-string` means scalar string/nonEmpty=false, and `contact-required-string` sets nonEmpty=true. These are table abbreviations, not authored functions or additional TypeSpec aliases.

| key | FieldId in the new target schema | valueType and actual membership use |
| --- | --- | --- |
| name | people/contact-name | contact-required-string; required by Person/Organization |
| company | people/contact-company | contact-string; optional Person |
| department | people/contact-department | contact-string; optional Person |
| position | people/contact-position | contact-string; optional Person |
| website | people/contact-website | contact-string; optional Person/Organization |
| birthday | people/contact-birthday | Disjoint date/month_day union; optional Person |
| phones | people/contact-phones | array min0/maxnull; dictionary with required nonempty number and optional string label/note, additional preserve; optional Person/Organization |
| emails | people/contact-emails | Same as phones with address replacing number; optional Person/Organization |
| addresses | people/contact-addresses | Same as emails; optional Person/Organization |
| status | tasks/status | scalar string/nonEmpty=true; enum open/done/cancelled, default literal string=open; required Task |
| priority | tasks/priority | scalar string/nonEmpty=true; enum low/normal/high/urgent; optional Task |
| due | tasks/due | Disjoint date/instant union; optional Task |
| start | tasks/start | Same as due; optional Task |
| project | tasks/project | scalar node_ref/nonEmpty=true; ordinary project note, optional Task |
| aliases | core/aliases | Array of nonempty strings with minItems0/maxItemsnull; optional in all base types, also usable in untyped ordinary notes |

Aliases preserve native order and duplicate strings; FEATURES handles candidate deduplication/ACL without changing source or assigning alias identities. Project reading/copy/deletion consumes typed NodeRef and applicable inbound gates. System alias semantics require the Core-reserved core/aliases FieldId and exact binding under the same installed schema qualification; a matching key/plugin string array cannot claim this role. Project semantics recognize only tasks/project with explicit subject Task membership, actual Field/target disclosure and scope permissions, and same-cut reference proofs. Company text or another NodeRef Field is not a project relationship; no default inverse/complex fact network is added. The three base types have empty requires/conflicts and may combine explicitly. §6 merges shared key/Field use: Person+Organization still has one name/phones value. Core's template meta-kind is outside their composable business-type domain. Complete installed configuration explicitly supplies language labels and includes them in the digest, without ambient locale defaults. UI groups follow type Field use without new nested task/contact Fields. Stable company references, organization parents, specialized relations and other values require explicit new Field/type contributions, without upgrading base text implicitly.

### 7.2 The unique base-seed producer and installation

Core's built-in seed factory deterministically produces a complete Snapshot under this section, without inferring definitions from examples/old catalogs. Generation is 1 and predecessor is bootstrap. Fields/types/aliases sort under §6.1, nullable is always false, and labels are exactly empty arrays; UI may display keys as non-authoritative labels where translations have not been supplied. Administrators may explicitly install language-label configuration successors, without filling values from host locale.

Expand §7.1 as follows: contact-string is scalar string/nonEmpty=false; contact-required-string sets true. Date/instant are scalar with nonEmpty=false/true respectively. month_day is a semantic=month_day dictionary with required nonnullable Int day/month, nonEmpty=false and additional=preserve. Birthday union branch order is date then month_day; due/start is date then instant. Phones is array(minItems0,maxItemsnull) of a semantic=object dictionary with sorted label/note/number members. Label/note are optional strings, number required with nonEmpty=true, all nonnullable. Emails/addresses replace number with address and re-sort keys. All defaults are absent except status, whose literal default is NativeValue string open. Status enum sorts cancelled/done/open; priority sorts high/low/normal/urgent, all NativeValue strings; other enums are null. Project is scalar node_ref/nonEmpty=true; aliases is array(minItems0,maxItemsnull) of scalar string/nonEmpty=true. Both default to absent and are nonnullable.

Types are exactly people/contact, organizations/contact and tasks/task with empty requires/conflicts. Person uses required name and optional company/department/position/website/birthday/phones/emails/addresses/aliases. Organization uses required name and optional website/phones/emails/addresses/aliases. Task uses required status and optional priority/due/start/project/aliases. Type Fields sort by FieldId. Aliases are exactly organization→organizations/contact, person→people/contact and task→tasks/task, sorted by name. Use exact §6.1 members and real JSON strings rather than prose shorthand, keeping null/Boolean/integer domains separate. Compute the actual full-object digest under §6.1, without a placeholder.

The output is immutable configuration proposal, never automatic activation or a read default. The sole installation entry is CORE's authorized Workspace bootstrap/registry configuration transaction. An administrator selects the complete seed and freezes exact snapshot bytes/pin, TypstSchemaBinding, applicable Evolution and format registrySeedBinding; real owner/administration Policy validation precedes the original single planning CAS/P. Only an installed qualified seed can supply creation forms, templates, Field producers and Query. Source aliases never trigger automatic seed download/installation; missing registry is not an empty seed. Planned operations retain their frozen seed across software changes. Task creation writes the installed status default explicitly through an authorized form/template and revalidates same-cut schema, without reading prose as a runtime default.

## 8. Exact origins and editing eligibility

`TypstFieldProjection/1` freezes owner, full source observation/version/pin, schema generation/digest, decoded key, FieldId when bound, availability, available typed value, exact expression bytes/AST path, origin and validation dependencies. Unified origin classes are rootLiteral/rootConstruct/generated/include/multiOrigin. Paths/ranges are current origins, not durable identities. Direct literals are rootLiteral; qualified int(Str)/datetime/decimal/duration are rootConstruct. Other expressions preserve actual origins instead of matching rendered text to root literals.

The default mutation unit is the full Field value. Contact arrays contain no per-value UUID/assertion or hidden stable identity. A child control may patch a revision-local AST path (for example phones item 1 number), only while its selector freezes the whole array preimage, exact pin/version/observation, schema, authorization and selected token bytes. Deletion/reorder, external editing, ABA, gaps or `any` other revision invalidate the path and require fresh selection. Identical values cannot rebind it. Annotations likewise cannot treat array positions as permanent objects.

Narrow editing is an independent authorization result: a unique writable rootLiteral/rootConstruct span, current source/schema/format, Field read/write and entity/state/commit qualification, conservative static dependency upper bound plus actual origin/use footprint, full before/after local constraints/requiredness/closure preservation, and no disclosure of unauthorized bodies/Fields/negative ranges. Retain both static and actual narrow proofs. CORE Policy/5 adds default-deny Workspace capability `typst_dependency_state(fieldId)` checked before dependency inspection; old Policy4's AsciiDoc `native_dependency_state` does not authorize Typst.

Root metadata does not bind a variable for body use, permitting a proved isolated status positive case. But body introspection may read it; grep for status cannot prove absence of dependencies. Without complete official origin/dependency evidence, a structured plan uses independently authorized whole-source scope. Field-only clients receive aggregate unavailable, without hidden-body reasons or automatic Source grants. Lack of narrow proof never rejects an independently authorized legal Source save.

Existing scalar writes splice only selected value tokens; rootConstruct int(Str)/datetime/decimal/duration edits splice the full standard constructor or a parameter token with complete schema-decoding and same-type round-trip qualification. Adding/removing keys/items and reordering require the complete Field container and trivia boundary plan, never nonexistent selectors or guessed commas. Preserve unselected bytes; comments owned by deleted items require accurate preview instead of reformatting neighbors. Complete official reparsing proves the unique declaration, ID/types, proposed value/schema and non-target AST/byte equality. A Field write cannot inject a second label or change keys/controls.

Properties, Source and ordinary rich editing share one Draft/Undo and Core prepare/commit. The property panel retains schema grouping, search, pinned common Fields and explicit authored/readonly-computed/unavailable states. Labels do not replace FieldIds. Device-local recent-use preferences update only after a real successful commit, never cancellation, preview or hover, and supply no business authority or permission. Full legal Typst Source, including imports/functions/computation/generated content, has a separate save path from the writable property/rich subset; saving does not require every projection to be available. Missing rendering/resources/body compilation is not source-language invalidity. Bad syntax/controls/known local schema failures are not typed success: preserve Draft and CORE's explicit repair/raw states. Saved/installed/sealed/compiled preview differ; do not display stale preview as current. Annotation range displacement only uses the actual same source transform, without matching text to revive addresses.

### 8.1 Closed root/Field output and addresses

`TypstByteRange/1={start:Counter,end:Counter}` uses exact half-open UTF-8 bytes, start≤end on scalar boundaries. Declarations/values are positive width; only CORE's explicit construction insertion can be zero width. AST paths are official root child-index arrays, never derived from DOM/text/path names.

```text
TypstSourceDiagnostic/1={code:text,range:TypstByteRange/1|null,message:text}
TypstRootDeclaration/1 =
 {state:"authenticated_structure",version:1,observation:SourceObservation/1,
  sourcePin:PinRef/2,declarationRange:TypstByteRange/1,dictionaryRange:TypstByteRange/1,
  idRange:TypstByteRange/1,id:NodeId,typesRange:TypstByteRange/1|null,
  declaredTypes:[text...],diagnostics:[TypstSourceDiagnostic/1...]}
 |{state:"invalid_control",version:1,observation:SourceObservation/1,
   sourcePin:PinRef/2,diagnostics:[TypstSourceDiagnostic/1...]}
 |{state:"incomplete_source",version:1,observation:SourceObservation/1|null,
   sourcePin:PinRef/2|null,diagnostics:[TypstSourceDiagnostic/1...]}
TypstFieldProjection/1={version:1,owner:NodeRef,observation:SourceObservation/1,
  sourcePin:PinRef/2,schema:TypstSchemaQualification/1|null,
  fields:[{key:text,fieldId:FieldId|null,
    state:"available"|"absent"|"unbound"|"invalid"|"schema_unavailable"|"computed_unavailable",
    value:TypstNativeValue/1|null,expressionRange:TypstByteRange/1|null,
    astPath:[Counter...],rawExpression:text|null,
    origin:"rootLiteral"|"rootConstruct"|"generated"|"include"|"multiOrigin"|null,
    diagnostics:[TypstSourceDiagnostic/1...]}...]}
TypstElementLocator/1={version:1,owner:NodeRef,revisionToken:Token,
  path:[Counter...],range:TypstByteRange/1}
TypstRangeLocator/1={version:1,owner:NodeRef,revisionToken:Token,
  range:TypstByteRange/1}
TypstSelectedField/1={version:1,fieldId:FieldId,key:text,
  state:"available"|"absent"|"invalid"|"schema_unavailable"|"computed_unavailable",
  value:TypstNativeValue/1|null,rawFieldSource:text|null}
TypstOriginEvidence/1={version:1,evaluationDigest:Digest,
  records:[{id:Counter,kind:"rootLiteral"|"rootConstruct"|"generated"|"include"|"multiOrigin",
    sources:[{owner:NodeRef|null,pin:PinRef/2,path:[Counter...],range:TypstByteRange/1}...],
    inputs:[Counter...]}...]}
```

authenticated_structure means §2's exact unique AST/direct ID/types/binding evidence, not birth/ACL authentication. Missing parseable declarations are invalid_control. Missing physical/source qualification or syntax preventing exclusion of a second declaration is incomplete_source. `control_unavailable` is a diagnostic code, not a fourth state. Code/message are display data, never safety dispatch; only closed states dispatch. Unauthorized callers receive CORE's non-disclosing outcome before existence diagnostics.

Fields are unique/sorted by key and cover the authorized Field scope, not a whole-Workspace completeness claim. A one-Field request emits only that Field. available requires non-null value; native none is `{kind:"none"}`. absent has null value/range/raw/origin and empty path. Other states have value=null and, where an expression exists, consistent exact range/raw/path without imaginary tokens. Unbound raw data is visible only through separately authorized Source/native-data display, without typed value. schema=null means unavailable qualification, not empty schema. Owner/pins/observation/schema match CORE's evaluation cut.

`TypstFieldProjection/1` is internal: absolute ranges/paths are never returned directly to Field-only clients. CORE `TypstFieldImage/1` uses `TypstSelectedField/1`, carrying only the chosen complete native key/value and its owned comma/trivia rawFieldSource, without absolute offsets, neighbors, root dictionary or body. Value/state follow the same rules; absent has null raw/value. Opaque pins bind exact Field bytes, not synthesized JSON. Old FieldEntryImage does not decode it. Ref-like strings only decode at bound typed positions, without raw scanning.

CORE produces TypstOriginEvidence/1 from the same official evaluation/AST. Record IDs are exactly 0..n-1; unique inputs refer only to earlier IDs. Sources sort uniquely by (owner,pin,path,range), without reconstruction by matching HTML. Literal/construct has exactly one real source without synthetic dependencies; include identifies another actual unit; multiOrigin contains complete multiple sources; generated lists complete actual input edges and may have no authored source. evaluationDigest binds the same source/World/environment/target; the pin covers complete records. Evidence grants no disclosure/writing/stable identity. Static Fields need no compilation evidence. Compiled semantic originIds resolve to these records; missing evidence is readonly/unavailable.

New addresses carry revisionToken instead of future SourceVersion.changeId. Same-decision construction freezes the token from the original SourceRevisionPlan and reuses it in both passes. Seal associates the protected binding with actual full version/ChangeId without changing frozen source or creating a self-reference. Unsealed addresses are not published as readable.

These addresses do not expand old l1/l2 or `D2` locator decoders. Token is canonical 43-character base64url with tag d6_source_revision/2. Original Artifact/protected RevisionTokenBinding/2 resolves it to a genuinely sealed production SourceVersion/2. A cold read first authorizes, authenticates production binding, obtains a fresh current SourceObservation matching that version exactly, then verifies ranges/paths using the fixed AST. External, unadmitted selections stay Draft/runtime basis and publish no durable locator. Equal bytes/paths do not preserve identity across revisions. Generated/include/multiOrigin require a real unique owner source for addressing, never rendered ordinals masquerading as addresses. FEATURES/EDITOR use their new annotation/suggestion adapters; historical versions retain recorded decoders.

## 9. Explicit simplification and predecessor dispositions

This table names actual-current input files/sections and replacements. Historical files/recorded decoders/bytes are unchanged. Undeployed drafts create no invented runtime migration obligations. Body/resource conversion requires explicit import/loss preview; no Workspace migration is claimed.

| Old actual-current source/section | Named disposition | New owner/section and oracle |
| --- | --- | --- |
| `a2-final-design/AUTHORING` §§1–2 | `typst-root-authoring` supersedes AsciiDoc names/continuous header/comma types with native metadata/dictionary/array | SOURCE §§1–3; S-01–06 |
| `AUTHORING` §3 | Retain Node/birth/ACL/partial-sync principles; supersede carrier/construction | SOURCE §3, CORE; S-07 |
| `AUTHORING` §§4–5; `D4` §§2.2,3,16.1,16.3 new author carrier | `typst-schema-native-values` supersedes native bindings/codecs/header-entry UUIDv5/Entry envelope; retain schema owner/binding/ledger obligations | SOURCE §§4–6,8; S-08–14 |
| `AUTHORING` §6; D4 §§4.2,16.2 | `ordinary-business-model` supersedes root-collection prohibition, per-value Entries/qualifiers/provenance and JSON complex fallback; native arrays/dictionaries without default UUID/confidence | SOURCE §§4,7–8; S-09,15–17 |
| D4 §§10.1–10.2,17.1; `D4-IMPACT` §9 People/Organizations; `snapshots/d4-reference-catalog-registry/source.json` contact definitions | `contact-simplification` supersedes default assertions, state/measurement/profession classification, rank systems, family/social/professional/27-relation and automatic inverse/network obligations; retain common names/contact/company/position/remarks as ordinary Fields/body. Specialized relationships require explicit FEATURES installation | SOURCE §7, FEATURES; S-15–18; catalog bytes retain their original object |
| D4 §§4.1,9,10.3,16.6,17.5 | Retain exact decimal/instant/calendar/timezone/recurrence service semantics/full proofs; `native-temporal-values` replaces authored JSON constructors with schema-native string/datetime/dictionaries | SOURCE §5, FEATURES; S-12,19 |
| `AUTHORING` §7; `AUTHORING-INTEGRATION` §§2–3.1 | `typst-field-origin` supersedes header/Entry selectors/rawAttributeSources/old native adapter for this carrier; retain current cut/disclosure/static+actual narrow proofs/one Draft | SOURCE §8, EDITOR/CORE; S-20–23 |
| `AUTHORING-INTEGRATION` §§4–6.1 | `typst-format-integration` supersedes profile/registry/header-H dispatch; do not put Typst into old Result10 H/Environment4 | CORE, SOURCE §§1,3,6; S-07,24 |
| `D2` §§1,5,8–9 and §12 SourceRange/Origin/Snapshot contracts | Retain one source/strict UTF-8/real owner/origin/availability/currentness/domain boundaries; new official Typst producers supersede AsciiDoc-specific projection arms | SOURCE §§1,8, CORE/EDITOR; S-22–24 |
| D2 §§2,6–7,11–12 fixedRuby language/oracle/property profile | `official-typst-language` supersedes engine/oracle obligations, without reimplementing Typst or making static Fields the language ceiling | CORE/EDITOR, SOURCE §§1,4; S-25 |
| `AUTHORING` §8; `AUTHORING-INTEGRATION` §7 | Replace this profile's acceptance with §10; original examples retain their own versions and cannot count as acceptance here | SOURCE §10, unified ACCEPTANCE/INTEGRATION |

Removing default complex contact facts is the user's confirmed product simplification, not an implementation gap. Real historical notes/provenance/precision are not silently discarded: conversion preserves originals and explicitly maps information into plain notes/body/separate material or rejects lossy conversion. Never reconstruct old assertions from natural-language notes. Specialized graph, Library, Calendar and business Action requirements use FEATURES' named schemas without imposing them on all contacts.

## 10. Positive and negative acceptance oracles

Each row requires positive/failure/preservation and actual producer→consumer coverage. Product/ACL/sync tests remain unrun.

| ID | Positive | Negative/preservation |
| --- | --- | --- |
| S-01 | BOM/comments/multiline dictionary+label, empty types, titleless reading | Title/pure markup/static set text before metadata have qualified positives; unproved let/import bindings retain legal Source but no authenticated root |
| S-02 | Direct lowercase UUIDv4; singleton types comma | Computed/uppercase/other-version ID; string/none/spread types: control error without automatic repair |
| S-03 | Matching raw/string/comment text is not a Label | Duplicate actual root Labels/inactive branch: ambiguous, no last-wins |
| S-04 | Shadowing after declaration and legal includes/body preserve root literals | Imported/function/dynamic metadata cannot authenticate root; compile query never authenticates |
| S-05 | Identifier/quoted dictionary keys decode exactly | Duplicate id aliases, dynamic key/spread: no control qualification |
| S-06 | Unknown legal type preserved, local schema unavailable | Alias/fullname duplicates and type conflicts never become empty types |
| S-07 | Create/template/copy fresh identity with birth/format same plan; move/restore retain identity | External ID edit/collision/partial metadata/unknown seal never merges, re-signs or re-samples UUIDs |
| S-08 | Exact string/bool/int/finite float/none/array/dictionary types | No bool/int, float/decimal coercion; out-of-range/nonfinite/schema failures |
| S-09 | Empty array/dictionary, singleton comma, duplicate phones/order retained | Grouping is not array; empty/none/missing differ; no deduplication |
| S-10 | Unbound top/nested keys retain bytes; untouched Fields readable | No deletion/name guessing/empty success under unavailable schema |
| S-11 | Literal phones readable beside computed formula | Unsupported/spread array wholly unavailable, no truncation/result edits |
| S-12 | Standard date/time/localdatetime and schema instant/decimal text | Today/arithmetic lack static Fields; no inferred timezone/float precision loss |
| S-13 | Two types share one Field; required OR/one definition | Key/Field/type/default conflicts rejected during installation, no source precedence |
| S-14 | Defaults produce actual creation values only | Missing/invalid existing source never defaults; same generation/different digest rejects |
| S-15 | Flat contacts with ordinary company/position/labels/notes | No automatic assertions/value UUIDs/confidence/semantic-code labels |
| S-16 | Edit only item 1 number among duplicate phones; other bytes equal | Deleting/reordering item 0 invalidates old item 1 path, without cross-revision rebinding |
| S-17 | Add/remove/reorder use full container preimage/trivia preview | Ambiguous comma/comment ownership rejects, without whole-source formatting |
| S-18 | Company text needs no lookup; explicit company_ref uses stable NodeRef | Names/tree parent infer no business relation; unknown/ACL target is not empty |
| S-19 | Native dictionary/string calendar/decimal services preserve precision/proofs | Missing rules/tzdb/ranges never infer host defaults or complete results |
| S-20 | Current isolated status with static+actual proofs has authorized narrow write | Introspection/body reads/unknown use proof make narrow unavailable; grep cannot admit |
| S-21 | Field-only output reveals only authorized value/aggregate qualification | Hidden bodies/negative ranges cannot leak paths/offsets/reasons or grant Source |
| S-22 | One Draft/Undo; full legal import/functions save independent of rich eligibility | Generated/multiple origins may be readonly islands without rejecting Source; compilation failure cannot show stale preview |
| S-23 | Exact current token splice; reparse value/non-target bytes+AST | Injected controls/second labels/changed keys/stale observation retain Draft with zero install |
| S-24 | New carrier/profile pin; real old records recover under original decoder | No AsciiDoc H/parser reuse, read-time upgrades or invented migration |
| S-25 | All official legal bodies/computation retain Source access; extend product cases for date/float | Research allowlist never limits the language; original 32 research checks are not product acceptance |
| S-26 | Deterministic §7.2 complete seed; authorized installation supplies task/default/project/aliases to forms and Query, including explicit 白云山 alias of 白云山2026 | Aliases do not install seeds; missing schema never defaults open or mutates old catalog IDs |
| S-27 | Accurate i64::MIN int(Str); adjacent duration seconds above 2^53 remain unequal; exact decimal constructors | Float-spelled minimum integer is not Int; equal floating .seconds() does not prove equal durations; unit/intermediate overflow rejects static Fields |

## 11. Official foundations and evidence limits

The fixed [official v0.15.1 implementation](https://github.com/typst/typst/tree/v0.15.1), [AST](https://github.com/typst/typst/blob/v0.15.1/crates/typst-syntax/src/ast.rs) and [Source](https://github.com/typst/typst/blob/v0.15.1/crates/typst-syntax/src/source.rs) own syntax/data behavior. [Metadata](https://typst.app/docs/reference/introspection/metadata/), [Dictionary](https://typst.app/docs/reference/foundations/dictionary/), [Array](https://typst.app/docs/reference/foundations/array/) and [Datetime](https://typst.app/docs/reference/foundations/datetime/) explain native mechanisms. Root roles/schema/ACL/application identity are Weftext contracts.

This author ran isolated official 0.15.1 CLI compilation and native-value query checks for all four Chinese examples, plus one combined title/set-before-metadata, i64::MIN int(Str), decimal("0.30"), duration above 2^53 whole seconds and month_day example. All five compile/query calls exited 0. Query checked sample semantics only, never identity authentication; no visual PDF QA was performed.

Prior research tested official AST metadata, editing the second equal phone token, non-target equality/inverse splice, body compilation-error separation and limited body editing. That evidence does not verify this document's added float/datetime/decimal/duration/schema merge/ACL/sync/new carrier. No real-data migration, complete rich editor or implemented transaction/identity authentication is claimed.
