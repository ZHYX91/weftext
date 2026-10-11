---
source_language: zh-CN
translation_of: AUTHORING.zh-CN.md
translation_status: synced
---

[简体中文](AUTHORING.zh-CN.md)

# Note authoring: native identity, types and business attributes

Status: design revision from `6b13b03b8da3e9e3d14b8663e91442568c8917e0`. Bounded independent review status, scope and predecessor dispositions are in the [revision entry](REVISION.zh-CN.md). It is not implemented and the original global review remains incomplete. This file solely owns the new source mapping; D3 identity, D4 value semantics, D6 commits and D8 editing qualifications retain their original owners.

## 1. File shape and native language

A managed note remains one `.adoc` file. Note ID, types and simple business fields all use native AsciiDoc header attributes; the body retains the full native language and new notes generate no YAML front matter. Schema owns field data types, requiredness, enums, cardinality and relation constraints. Simple values require no authored JSON constructors. Native attributes are flat strings, not claimed to provide native arrays or nested objects.

```adoc
:weftext-id: 12d7c4f3-9ee6-4c36-920d-ec6af27bc654
:weftext-types: task
:status: open
:due: 2026-10-20
= Prepare the project report

Organize the research into a project report.
```

Control and business attributes may precede or follow the title or occur on both sides; legal placement must not be forcibly changed. Attributes and title obey native contiguous-header and blank-line termination rules. Titleless documents remain legal. Title, filename, directory and first paragraph do not create identity or types.

D2 passes the complete file through the same native processor and consumes its attribute events, header boundary, original byte ranges, condition and source-origin evidence. No new envelope is stripped, offsets are not reset and there is no second header parser. Weftext validates reserved names and schema values above native parsing. YAML is no longer an identity/type source or fallback in the new profile. Foreign front matter retains source bytes and native processing results; managed admission requires explicit import/source-repair preview, without automatic removal or ID recovery from it. Schema, configuration and resource files keep their own existing formats. See [native headers](https://docs.asciidoctor.org/asciidoc/latest/document/header/) and [attribute names and values](https://docs.asciidoctor.org/asciidoc/latest/attributes/names-and-values/).

## 2. Native control attributes and classification

The control names are `weftext-id` and `weftext-types`, compared by native canonical name. ID requires exactly one direct literal root-header set whose value is exactly a D3 lowercase UUIDv4. Types may be absent, otherwise it also requires one direct literal set. Controls accept only one physical line, unconditional actual root declarations without attribute substitution or continuation; quotes are not string delimiters. Missing/empty/duplicate ID (including equal values and case aliases), any header unset, or duplicate/unset types makes managed-control projection unavailable while retaining authorized Source reading/repair. Business attributes retain §5 native redefinition; uniqueness is not imposed on all native attributes.

The single types string has one list convention: ASCII comma separates tokens; trim only SP/HTAB around each item. Entirely empty or absent means an empty set. Nonempty lists reject empty items, trailing commas, quotes, brackets and whitespace used instead of commas. Each item must be an existing D4 single-segment shorthand or full namespace/name; display labels never guess tokens. This is a Weftext attribute-value protocol, not native array syntax. Retain authored order/whitespace while comparing expanded sets; malformed lists never degrade to empty classification.

Control qualification comes from exact root declarations and complete origins in the same D2 header-end evaluation, not the final effective-attribute map alone. Core proves that include, conditional or external API/environment set/unset has not controlled the declaration and that its header-end value matches the direct declaration. Even equal values from those external sources cannot authenticate it; incomplete evidence is unavailable. Inactive branches produce no control declaration, and environment changes cannot select another branch's ID. Same-name body set/unset retains native rendering behavior with a diagnostic that it does not control the note; it cannot change the extracted declaration or entity identity. Complete declarations still require §3 protected metadata qualification. Validate during header processing in that same native evaluation: neither the D2 headerAttributes lexical list nor final effectiveAttributes alone proves header-end state. An old snapshot containing only a final map cannot infer or upgrade qualification. Re-evaluate complete original pins/profile for actual new qualification or report unavailable, without reviving old selectors.

`task` is a fixed built-in shorthand for `tasks/task`. `template` is the reserved D2 Core meta-kind token: no facet alias may use or rebind it, and full facet IDs are never shortened into this token. It cannot coexist with business facets such as task; D9 template recipes still declare target instance types. Other built-in or extension shorthands come from the verified table in §4; full `namespace/name` remains available. A short and full name expanding to the same facet are duplicates. Source order matters only for lossless editing, not set meaning. D4 still owns requires/conflicts/effective closure. Multiple types do not create multiple Nodes or a TaskRef.

An uninstalled/unknown legal type identifier retains its text and displays unavailable schema, never an empty type set or automatic rename. Extensions cannot rebind `task`. Full IDs, labels, translated names and shorthands differ; installation order, enablement and localization do not change stored meaning. Native attributes named `wf-kind`/`wf-facets` may still participate in native processing but have no identity/classification authority or control-attribute fallback role in the new source profile.

## 3. Note identity and external metadata

`weftext-id` is the note-carried NodeId declaration; a full NodeRef is still WorkspaceId + NodeId. D3 allocation, birth, tombstones, non-reuse, permissions and lifecycle remain proven by D3/D6 portable metadata and original decisions. A string in a file is not permission or birth evidence; metadata no longer provides another independently editable document-ID value. The original operation installs both consistently; disagreement is conflict/incomplete, never a choice to overwrite one from the other.

Creation preallocates one UUID in the original D3 plan and writes that same value to control attributes, file binding and birth. A D9 template cannot prefill a final ID. Truly new Nodes from copy/fork/import receive fresh IDs; for managed2 source the original identity map rewrites managed references and control attributes while preserving unknown/unprovable content and the original loss/confirmation rules. Move/rename preserves ID. An unseen copy/fork of actual managed1 content preserves its exact profile by default and inserts no new control attributes; only explicitly selected and previewed copy-plus-format-conversion produces managed2. Restore recovers the true historical binding without conversion. Restart/recovery uses the original frozen UUID. Externally editing an ID is not a rename, identity transfer or creation authorization.

Copying one `.adoc` carries its ID declaration and types, but cannot alone restore Workspace relations, permissions, annotations or execution responsibility. Complete backups still include portable metadata, required resources and independent `weftext.annotations.json`. Two files claiming one ID in a Workspace are a collision, never first-wins, silent merge or reissuance; an explicit import-as-new action performs fresh allocation. Missing metadata or partial sync preserves separately authorized source reading/repair with incomplete managed identity; rebuilding an index never allocates replacement identity.

## 4. Schema bindings for source names

`RegistrySnapshot/2` is this proposal's sole new registry snapshot shape: it inherits all sixteen members of D4 §2.2 `RegistrySnapshot/1` and adds required `version:2` and `authoringBindings`, retaining the original `kind`. Version is exact JSON integer 2, never Boolean. Fresh production/current consumption accepts only this explicit version; missing version never falls back to Snapshot1. Original unversioned Snapshot1 is accepted only in its real recorded typed/recovery domain or an explicit old-to-new evolution before slot; after must be Snapshot2. Version2 without authoringBindings is malformed; unknown versions reject, with no member-presence downgrade. Exact cross-module slots are in [authoring integration](AUTHORING-INTEGRATION.zh-CN.md). Other members, ownership, ordering, completeness, tombstones and migrations are unchanged. `RegistryBinding/1` still freezes generation and the digest covering the complete new object. Actual old Snapshot1 uses its original closed decoder, without ignoring new members or upgrading on read. D10 authenticates contribution owners and D4 validates schemas/bindings; missing or unknown versions are not empty configuration.

```text
authoringBindings = {
  version: 1,
  typeAliases: [{name, facetId}...],
  fields: [{attributeName, fieldId, codec, wrappers}...]
}
codec = "text" | "boolean" | "integer" | "decimal" | "semantic_code"
      | {kind:"calendar_date", calendarId, calendarVersion, precision:"day"}
wrappers = [{variant:<declared union variant>}...]
```

All objects are closed. Arrays must be actual arrays, sorted uniquely by unsigned UTF-8 `name` or `attributeName`. Shorthands are D4 single-segment tokens; attributeName is a native legal name in native canonical form and cannot occupy any fixed-baseline built-in attribute, `weftext` name or reserved `weftext-`/`wf-` prefixes. Each FieldId has at most one native binding; attributeName is also unique within the snapshot. Bindings do not duplicate FieldDefinition; fieldId resolves to the original verified definition. A type alias is declared by its verified facet owner; business-name bindings are authorized Workspace schema configuration, not injected by enabled plugins. Adding/removing/rebinding a name that affects existing sources requires a complete affected-source preview and the original schema-evolution rules, never silent reinterpretation.

`wrappers` selects declared union variants from outer to inner; an empty array means a non-union. The innermost type must match codec and Field maximum cardinality must be 1; incompatibility rejects the binding. Calendar codecs freeze calendar, version and precision and validate the original comparator/lexeme; host locale/time zone is never inferred. Boolean accepts only `true`/`false`; integer/decimal use D4 canonical decimal strings; text retains the full string and semantic_code validates the target Field's closed code domain. Types and conversions are schema facts, without repeated per-value type tags.

Initial task configuration contains `task→tasks/task`; `status→tasks/status` and `priority→tasks/priority` use semantic_code; `due→tasks/due` selects `wrappers=[{variant:"date"}]` and the selected registered Gregorian day-precision calendar codec. Its calendar ID/version must verify in the actual Workspace's same snapshot, not be guessed from an example. Task status remains required with the original catalog open/done/cancelled domain. Original priority codes, due's date/instant union and complex-value capability remain; instant values retain the §6 complex carrier. Immutable catalog bytes do not change.

Reserved controls are not business Fields: they cannot occur in authoringBindings.fields, business MapAttribute mappings, FieldSource, FieldSelector, field_set or other ordinary Field writers. Type changes retain original FacetOperationRequest3 authorization/closure gates and full Source preview; ID writes belong only to original identity production/conversion. New attribute values do not bypass those boundaries.

Logical groups such as task are constructed from existing FacetSchema.fields/relations and verified Field bindings; status, due and priority may appear in a task group. UI grouping creates no nested task value, extra empty task attribute or duplicate FieldDefinition. Fields shared by multiple facets remain one fact; device preferences may pin/collapse groups without changing types, membership or authority through prefixes or hiding. A task- name prefix is an explicit Workspace binding choice, not structural inference; rebinding retains original evolution rules.

## 5. Native attributes as business values

Only §4 bound attributes become D4 Fields; similar names do not claim ownership. D2 retains full native attribute events. D4 reads effective values and complete provenance at the end of the root document header in that same evaluation. Later body set/unset still affects native rendering but does not silently change a header Field. Repeated header assignments follow native redefinition, not array accumulation. Unset means absence; empty string differs from absence; invalid values produce explicit Field diagnostics rather than absence/zero/default. Field presence does not add a note type; facet requiredness and constraints are checked separately.

Native Field authority must originate in the root-authored header, not API hard-set, environment or included sources. An effective value controlled by those sources is typed-unavailable while retaining native rendering/Source capability. Native substitutions remain supported with frozen dependencies, origins, environment and authorization; non-root dependencies, multiple origins or generated values do not obtain narrow Field write qualification. Unbound ordinary attributes remain native metadata, neither deleted nor control-declaration errors.

The internal business projection preserves FieldId, TypedValue, permissions and relation/Calendar/Query meaning. A single header value uses a derived UUIDv5 occurrenceKey: the standard URL namespace, with the UTF-8 D3-CJ/3 encoding of `["weftext.header-entry/1",WorkspaceId,NodeId,FieldId]` as name. It is not persisted to source or a new EntityRef; use requires complete owner/Field/source-version/Observation/Registry qualification. Equal-value ABA, deletion/recreation and index rebuild never revive old selectors. The key is neither permission nor continuity evidence.

`NativeFieldOrigin/1={kind:"native_header",attributeName,rawAttributeSource,sourceRange}` is a closed internal D4 materializer origin using the same evaluation's D2 root range. It does not widen the historic Entry1 JSON decoder: block bytes use that decoder and header bytes use this codec. D4→D7→D8 typed values agree, but raw header source is never disguised as JSON rawEntrySource or patched by a JSON-only materializer. Without a uniquely writable root span, use the complete Source Draft path instead of claiming narrow-write success.

## 6. Multiple values, notes, provenance and complex fields

Complex objects, relations/refs, repeated equal assertions, qualifiers, per-value notes/provenance and non-default union branches retain the existing precise `[weftext-attributes]` Entry1 representation and internal TypedValue. This revision removes JSON from common simple values; it does not claim a simpler equivalent text representation for every complex structure.

One Node + FieldId has exactly one effective author carrier: native header or Entry stream. Both at once, even with equal values, are a carrier conflict, never precedence, merging or fallback. All complex occurrences of a Field share its Entry stream; an unbound similarly named native attribute does not impersonate that Field. Schema determines final meaning and untouched Fields retain exact bytes.

Adding per-value notes/provenance or changing to a complex value requires an explicit representation-conversion preview: exact header span to complete Entry, allocated occurrenceKey, original value/type/dependencies, removal of every same-name header assignment/unset, and all affected native references. If the attributes are still referenced by body/other values, preserve equivalent native behavior or reject this conversion while retaining Draft; deleting an assignment must not silently change other content while reporting one Field change. Reverse conversion requires cardinality≤1, no extra information, an invertible codec and complete replacement-impact validation. Never discard notes/provenance, precision, relations or duplicates to obtain shorter source.

## 7. Editing, copying and recovery

D8 property panels, quick capture and Source share one Draft, Undo and Core prepare/commit. Default edits patch only selected native value spans without whole-file formatting; preserve comments, whitespace, line endings, unknown body and placement around the title. Invalid/ambiguous controls retain Draft and repair without generating another ID. Ordinary property panels keep identity read-only; type edits require original type constraints, node_control/source write qualification and complete preview.

D4 relation/materialization, D7 copy/Field Action, D8 structured edits and D9 import/template constructors branch on actual native/header versus Entry origins and carry control-declaration/fact changes through original C/Q materialization and the single D3/D6 commit. Exact selectors, effects, template slots and control materialization are defined in [authoring integration](AUTHORING-INTEGRATION.zh-CN.md); internal control construction is not falsely treated as an existing arm of the old C/Q grammar. New syntax does not waive final read barriers, hidden fields, complete negative ranges, Field-only permissions, source transforms, inverse relations or installation recovery. Narrow Field requires proof that every change lies within authorized Fields; whole-source editing independently uses full-source authorization.

The new source profile is `weftext_managed/2`, carried by `ManagedDocumentFormatProfile/2`: languageBaseline retains its fixed value and managedProfile is exactly the new value. `ManagedDocumentFormatBinding/2` inherits Binding1's members, fixing version=2 and profile=Profile2. D6's existing document_format key/component carries the exact version without new authority/Notice/CP/CAS branches. The current qualification binding slot explicitly accepts Binding2; recorded Binding1 profile/decoder/bytes stay unchanged, without read-time upgrades.

This directly replaces the undeployed managed2 YAML candidate definition, retaining candidate Profile2/Binding2/Environment4 without managed3 or invented YAML runtime migration. Prior Git drafts and their fixed-version reviews remain design history, not installed runtime records or automatic acceptance of this definition. Actual old managed1 and recorded decoding responsibilities remain. If actual old managed1 content needs conversion, explicit format-conversion preview covers control/attribute rewrites and Binding2 together. Real source changes get a new source version and cannot use the old profile-only `sourceChanges=[]` shortcut. Original prepared/selector/preview objects are not upgraded; saved/planned/unknown recover their original versions, bytes/pins/UUID/OperationId. Missing schema still permits source reads, not empty schema substitution; bare files on a new device do not reconstruct permissions or execution responsibility.

## 8. Acceptance obligations

These are design requirements, not executed product tests. Each requires both positive and rejection/recovery paths:

| ID | Requirement |
| --- | --- |
| SRC-01 | Control/business attributes before/after/on both sides of title, and titleless source, are legal; blank lines and author/revision obey native rules; untouched bytes survive. |
| SRC-02 | Preserve direct UUID, comma type strings, native comments and BOM; duplicate/case aliases, unset, missing ID, conditional/substituted/continued controls and malformed lists reject control projection while retaining repair Draft. |
| SRC-03 | task and full name are equivalent; using both is duplicate; unknown types survive; plugin order and include/API cannot take over classification. |
| SRC-04 | Create allocates ID once; copy allocates fresh; move/restore retain original semantics; crash/retry uses the original plan without half-installed ID/birth. |
| SRC-05 | External ID edits, duplicate files, source arriving before metadata and missing-metadata index rebuild never silently merge/reissue/escalate. |
| SRC-06 | Exact schema conversion for status/due; distinguish empty, unset, invalid, body/header redefinition, API hard-set and includes; no guessed date precision/calendar. |
| SRC-07 | Reject same-generation changed digest, name collisions, built-in attributes, unknown members, incompatible codec/union and old Snapshot1 misdecoding. |
| SRC-08 | Header+Entry conflict per Field; equal phone values with different provenance remain separate; representation round trips do not lose data or silently change references. |
| SRC-09 | Query/View read the same typed value; narrow writes do not disclose whole source; revocation, ABA, gaps and format/schema changes do not revive selectors; ordinary Source saves retain their independent path. |
| SRC-10 | Template/import/copy/inverse materialization cover both carriers; source/body/ID/Binding share the original commit; actual old saved/planned/unknown bytes are not re-encoded. |
| SRC-11 | Foreign YAML grants no identity/fallback; unknown native attributes survive; note rules do not change schema/config formats or invent deployed YAML migration. |
| SRC-12 | Schema groups task properties without duplicating shared facts or creating nested objects from prefixes; every ordinary Field constructor/writer excludes controls. |
