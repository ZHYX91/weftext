---
source_language: zh-CN
translation_of: 19-node-template-library.zh-CN.md
translation_status: synced
---

[简体中文](19-node-template-library.zh-CN.md)

# Node Template Library

This specification defines the Template Library contract: role-aware inventory, the `weftext.template.json` profile, Template Designer, instantiation, and role-transition transactions. A caller may expose a capability only when it can preserve every contract in this document; otherwise it reports the defined unavailable diagnostic.

## Authority and roles

The workspace root's sole `weftext` envelope may contain:

```yaml
weftext:
  template_library_root: "550e8400-e29b-41d4-a716-446655440000"
```

The value is the lowercase UUIDv4 of one active managed node. Absence means no configured Template Library. Missing, duplicate, root-self, Trash, unmanaged/ignored, or non-unique targets are diagnostics; Core does not guess by path or name.

Role is derived from the current validated tree:

- the selected node is the Template Library root and is a container only;
- each direct managed child is one Template Root and one independently instantiable template; and
- every managed descendant below a Template Root is that root's Template Part, never an independent template.

Version 1 has no category/folder nesting inside the Template Library. A direct child cannot be a category containing Template Roots. The Library root, Template Roots, and Parts retain canonical `X/X.adoc`, UUID, resource, annotation, revision, synchronization, backup, Trash, and recovery rules, but they are special managed roles. They are excluded from every ordinary semantic projection: node rows, tasks, headings/outlines, citations/bibliographies, search, graph, Chrono, default links/backlinks, recents, and `from nodes|tasks|headings`. Only one Template Root row is exposed through explicit `from templates`; Parts appear only inside their authorized owning-template detail and subtree preview. When role-aware inventory is unavailable, `from templates` returns stable `domain_unavailable` rather than an empty set or path-derived guess.

## Fixed sidecar

Only a Template Root carries `weftext.template.json` adjacent to its canonical document. The filename is reserved node-local authority. It is invalid on the Library root, a Part, an ordinary node, a resource boundary, or any other location. It is not a general node manifest, YAML replacement, metadata directory, cached preview, or UI state.

The closed v1 shape is:

```json
{
  "profile": "weftext.node-template.v1",
  "version": 1,
  "parameters": [
    {
      "name": "project_name",
      "schema": { "type": "string", "nullable": false },
      "default": null,
      "examples": ["Atlas"]
    }
  ],
  "slots": [
    {
      "name": "title",
      "kind": "text",
      "scope": "11111111-1111-4111-8111-111111111111",
      "binding": "format_date(context.today, \"YYYY-MM-DD\")",
      "default": null,
      "examples": ["2026-08-25"]
    },
    {
      "name": "name",
      "kind": "node_name",
      "scope": "11111111-1111-4111-8111-111111111111",
      "binding": "input.project_name",
      "default": null,
      "examples": ["Atlas"]
    }
  ]
}
```

The sample scope UUID is the permanent Node UUID of the Template Root that owns the sidecar. Duplicate JSON keys, unknown fields, non-canonical names/order, an unsupported profile/version pair, duplicate parameter names, duplicate slot declarations with the same `(scope UUID,name)`, unknown parameter references, type mismatch, or excessive bytes/items fail closed. Profile `weftext.node-template.v1` and version `1` are one fixed paired discriminator, not two independently negotiable version axes; any other pairing is unsupported. Canonical order is parameters by name and slots by `(scope UUID,name)`; exact authored bytes remain source authority and ordinary edits are narrow.

Parameter names are ASCII snake-case. Each parameter has one recursive closed schema using the `weftext.expr.v1` types string, bool, number, date, instant, duration, UUID, list, or record plus explicit `nullable`. List schemas declare `items`; record schemas declare unique named `fields`. `default` and every `examples` value are optional typed JSON transport values validated against the same schema. A required value is one with `nullable=false` and no default. Defaults are values, not expressions.

A slot has an ASCII snake-case `name`, one `kind`, one exact `scope`, one `weftext.expr.v1` binding string, an optional typed default value, and typed examples. `scope` is always the lowercase permanent Node UUID of exactly one node in this Template Root subtree: the owning Root UUID for a Root slot or the Part UUID for a Part slot. Locators, names, paths, `root` magic strings, globs, and wildcards are invalid. Rename or move inside the same template therefore never changes slot identity. Bindings may read only the closed `input` record and explicit `context`. They cannot access `this`, query aliases, workspace inventory, filesystem, network, environment, UI, ambient time, or secrets.

## Slot source and kinds

Template source accepts:

```adoc
= slot:title[]

slot::body[]
```

`slot:name[]` is the inline form. `slot::name[]` on its own parser-confirmed block line is the block form. These macros have Template semantics only in the subtree of a valid configured Template Root and only when declared for that document's Node UUID by its Root sidecar. Source independently authored in an ordinary `.adoc` document is always inert preserved unknown-extension syntax; it does not read a nearby sidecar, ask the UI for values, or instantiate content.

That inert ordinary-source rule is not a downgrade mechanism. A node or subtree known by the validated pre-state to be a Template Root/Part cannot cross into an ordinary role through a path-only move, external-sync application, or restore-to-alternate-location and then reinterpret formerly active slots as harmless text. A reviewed role-conversion transaction must materialize or delete every active slot value and remove every template profile/sidecar before publishing the ordinary post-state, or block the whole transition. An observed out-of-band role change enters a repair-required diagnostic and is not accepted as a successful ordinary mutation.

V1 kinds are exactly:

| Kind | Use and validation |
| --- | --- |
| `text` | inline plain text encoded so it cannot introduce AsciiDoc structure |
| `attribute` | one bounded literal document-header attribute value; no expansion or continuation |
| `validated_inline_asciidoc` | inline source parsed by the constrained Weftext inline profile |
| `validated_block_asciidoc` | complete block source parsed in the exact destination context |
| `node_name` | sidecar-only portable generated basename for the scoped node |

`text`, `attribute`, and `validated_inline_asciidoc` require the inline macro. `validated_block_asciidoc` requires the block macro. One declaration is unique by `(scope UUID,name)`, but that declared content slot may occur multiple times in its scoped node source; every occurrence is replaced by the same once-evaluated value. Zero occurrences is `unused_slot`. `node_name` is the sidecar-only exception: it has no source macro, is unique for its scope, and is consumed by name resolution. There is no generic raw kind, implicit escaping mode, conditional, loop, repetition, include, Query, script, or custom renderer. A slot inside a protected range, a form/kind mismatch, undeclared occurrence, missing binding/default/input, or invalid output is a diagnostic.

Bindings are `weftext.expr.v1` expressions and return the kind's required value. A slot cannot execute Query. Query-derived generation is a two-step boundary: an authorized Query first produces a bounded typed value chosen by the caller, that value is frozen into template `input`, and instantiation evaluates only the frozen input/context.

## Compile and instantiate

Compilation inventories the complete Template Root and every Part, exact document/sidecar/resource bytes, parser ranges, internal/external node links, annotations, and drafts. It validates parameter input and explicit `context.today`, `context.now`, `context.timezone`, and `context.locale`, then evaluates every binding once under fixed limits.

The proposed output must satisfy all of the following:

- every generated node receives one fresh lowercase UUIDv4, generated and occupancy-checked once;
- each generated node has exactly one name authority: a scoped `node_name` binding when declared, otherwise the Root uses the caller's required explicit target name and a Part preserves its prototype basename;
- a Root `node_name` result is the destination name and disables caller override; every resolved name is portable and collision-free, and no suffix is silently added;
- every inline/block slot is replaced through its declared validated kind;
- no `slot:`/`slot::`, template profile, Template role, or `weftext.template.json` remains in generated output;
- all generated canonical documents parse and their `weftext.id` matches the frozen UUID map;
- internal `node:` links targeting a node inside the source template subtree are rewritten old-to-new;
- external `node:` links keep their UUID only after unique authorized resolution and boundary checks;
- every owned regular resource is copied with exact bytes/digest and rewritten reference evidence where required; and
- design-time `weftext.annotations.json` sidecars are omitted by default and the preview records their count/bytes and disposition.

No source template UUID, sidecar, or role is copied. External hidden, missing, duplicate, invalid, or forbidden link/resource targets produce non-disclosing blockers. Source or output ambiguity, protected-slot placement, residual macros, invalid `node_name`, or unsupported active content blocks the complete operation.

The authoritative preview freezes Template Root UUID, all source node/document/sidecar revisions, complete part/resource inventory, input schemas and values, evaluation context, evaluated binding values, generated UUID/path map, exact proposed sources, link rewrites, resource copies, annotation omission, target parent, resolved target name, its single caller-or-binding authority, ACL decisions, content boundaries, draft-sensitive UUIDs, counts/bytes, conflicts, and every journal step. `targetName` is the resolved value, never a second override input beside a Root `node_name`. Preview performs no write.

Commit rechecks the complete authority and draft registry and creates the full ordinary subtree in one recoverable workspace transaction. Reported success contains every generated node/resource/rewrite and no Template sidecar/role; rollback contains none. A derived-index refresh failure after a verified commit returns a warning, not permission to repeat the mutation.

## Role-boundary moves

Move into or out of the configured Template Library, between Template Roots, or between direct-child and descendant positions changes role and is a conversion transaction, not an ordinary move.

Preview must show old/new role and owner, exact sidecar creation/removal, source transformations, materialized/deleted slot occurrences, residual checks, path/UUID preservation, affected links/resources/annotations, ACL, drafts, and rollback. Moving into direct-child position requires a valid proposed Template Root sidecar and full subtree validation. Moving into a descendant position makes the branch Parts of exactly one root and forbids a nested Template Root sidecar. Moving out must safely materialize or delete every formerly active slot and prove the resulting ordinary subtree has no active or residual slot/profile/sidecar; it cannot rely on ordinary-source inertness. Focus or displayed hierarchy never changes these rules.

Generic Trash refuses the currently configured Template Library root. A distinct reviewed transaction may trash it only while atomically clearing `template_library_root` or rebinding it to another valid active container and previewing all role consequences. No path mutation alone clears or changes configuration.

Trash payload is a closed inactive byte-preserving storage class, not an active ordinary document subtree. It retains exact template documents and valid Root sidecars without evaluating or degrading their slots. Restoring a Root/Part to its proven role under the same active Library may restore exact bytes. Restore to an ordinary destination is a role conversion and must materialize/delete active slots and remove profile/sidecar or block. Restore never guesses or rebinds `template_library_root`; only an exact rollback receipt for the combined trash/configuration transaction or a new explicit configuration plan may restore that binding. Backup includes the root configuration, every template document, sidecar, resource, annotation, and Trash payload byte.

## Designer and creation UI

Template Designer has three peer views over one controlled exact subtree:

1. **Design** shows inline/block slot chips, selection-to-parameter, slash insertion for inline/block slots, parameter/schema/binding/default/example panels, and transaction-backed parameter/slot rename across sidecar and all occurrences.
2. **Preview** accepts sample typed input and context and renders the complete proposed subtree without writing.
3. **Source** shows exact canonical `.adoc` and `weftext.template.json` bytes, ranges, diagnostics, and preserved formatting.

Diagnostics cover unused, missing, type mismatch, duplicate `(scope UUID,name)` declaration, illegal form/kind, protected placement, residual slot/profile/sidecar, invalid `node_name`, unresolved link/resource, ACL, draft, stale revision, and resource ceilings. Repeated source occurrences of one content declaration are valid and all display the same value. Every Part visibly identifies its owning Template Root.

Create Node offers **Blank** and **From Template**. The template picker uses the authorized explicit template domain. Selection opens a schema-derived form, produces live no-write full-tree preview, and submits one transaction. If the Root declares `node_name`, the form displays its resolved destination name and has no override field; otherwise it requires the caller's explicit target name. The browser or Desktop never expands slots, chooses a second name, mints UUIDs, rewrites links, copies resources, or decides role independently.

## Chrono and external generators

Chrono may store/select an optional Template Root UUID and supply typed period/date values as input. Chrono's year/quarter/month/week/day spellings and hierarchy remain fixed; a template declaring any `node_name` is incompatible with Chrono and blocks selection rather than being ignored. Chrono supplies each fixed period basename as the explicit target name, and arbitrary path templates remain forbidden.

Office import/export and future generators may share the typed value transport and `weftext.expr.v1` evaluator contract. They do not gain Template slot syntax, Template Library role, or Query execution implicitly.

## Server, ACL, and non-disclosure

Authorization precedes Template Library discovery, template rows/counts, Part ownership, sidecar reads, parameter/schema display, sample preview, external-link resolution, conflict diagnostics, and instantiation. A hidden Template Root or external target is indistinguishable from unavailable. Server stores previews only for the originating actor/session, binds exact source revisions/input/context/target, expires and single-consumes them, serializes commit, and records a bounded audit event without copying source or secrets into the control plane.

Template Library configuration and sidecars are portable content; role permissions and account ACL remain Server control-plane state. Neither a sidecar nor an expression can elevate the actor, read hidden inputs, or cause external egress.

## Acceptance

Acceptance includes Library direct-child Root versus descendant Part; no nested categories; exclusion from every ordinary semantic projection; explicit `from templates` plus pre-inventory `domain_unavailable`; Root-only fixed profile/version-pair sidecar; UUID slot scope stable across rename/move; one declaration with repeated equal-value replacement and zero-occurrence diagnostics; one-authority Root/Part `node_name`; path-only role-downgrade refusal; safe materialization/removal on move out; fresh UUIDs; complete internal link rewrite and checked external links; resource copy; annotation omission; zero residuals; independently handwritten inert ordinary slots; generic AsciiDoc preservation; Designer Design/Preview/Source round-trip and exact-source preservation; parameter/slot transaction rename; schema form/live preview/one commit; Part ownership display; ACL and non-disclosure; dirty/stale conflict; configured-Library-root Trash state machine; exact-byte payload and explicit ordinary restore conversion; every crash boundary and exact rollback; and full backup/restore.
