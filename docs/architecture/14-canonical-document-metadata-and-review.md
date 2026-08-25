---
source_language: zh-CN
translation_of: 14-canonical-document-metadata-and-review.zh-CN.md
translation_status: synced
---

[简体中文](14-canonical-document-metadata-and-review.zh-CN.md)

# Canonical document, metadata, and review authority

Weftext has one managed-node shape and no managed-Markdown generation:

```text
X/
├── X.adoc
├── weftext.annotations.json   # only when needed
└── resources...
```

The root `.weftext-format` is exactly `weftext.asciidoc.v1\n`. A missing, unknown, or malformed marker fails closed or enters explicit import/adoption; extensions do not select a generation. Markdown is import/export, visible unmanaged content, or an attachment boundary only.

## Reserved names and envelope

Root controls use `.weftext-*`; closed-store children may use `_weftext.*`; portable node-local sidecars use `weftext.*`; and the system envelope is the sole top-level YAML `weftext` mapping. New names need an accepted storage/profile contract. The closed Trash store is defined in [architecture 17](17-workspace-trash-item-store.md).

Every managed document has a sole top-level `weftext` mapping. Required `id` is a lowercase UUIDv4. Optional `icon`, ordered `aliases`, child sort fields, sparse `sibling_rank`, root `adjacent_heading_body`, and root `template_library_root` have closed meanings. Path, parent, names, roles, timestamps, backlinks, search, task counts, thumbnails, and view state are derived or device/control-plane state. Envelope edits are narrow revision-checked YAML patches; whole-frontmatter reserialization is forbidden.

## AsciiDoc header and typed constructs

The document header owns title, subtitle, author, revision, language, descriptive metadata, processor configuration, and simple properties. Only bounded literal header attributes enter the Properties projection; body redefinitions are processor state. Attributes do not expand environment, paths, URIs, or processors.

Complex data uses versioned Profile constructs: native checklists and `weftext-task` task-node headers; canonical Query blocks with `weftext.expr.v1`; role-constrained Template Root `weftext.template.json`; and citation constructs. These do not create a second identity system, manifest, task database, or generic metadata store.

## Portable review sidecar

`weftext.annotations.json` is authority for non-authorial highlights, comments, threads, suggestions, and review marks. An authorial strikeout remains source; accepting a suggestion invokes a Core document transaction. Annotation targets bind base revision, UTF-8 range, quote, context, and structural evidence. Core reanchors only on a unique deterministic match; missing or ambiguous targets remain unresolved. Server serializes and authorizes hosted sidecar changes; credentials, drafts, presence, and provider state never enter it.
