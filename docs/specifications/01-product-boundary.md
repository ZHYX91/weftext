---
source_language: zh-CN
translation_of: 01-product-boundary.zh-CN.md
translation_status: synced
---
[简体中文](01-product-boundary.zh-CN.md)

# Weftext product boundary

Status: accepted product boundary.

Weftext has one canonical managed-document format: the Weftext AsciiDoc Profile in `X/X.adoc`, selected by the required workspace marker. Desktop may also edit one explicitly opened external `.adoc` or `.asciidoc` file through a standalone single-file boundary; that file is not a workspace or node and gains no Weftext identity or workspace semantics. Markdown is an explicit import/export format, explicitly classified visible unmanaged content, or an ordinary node-owned attachment; it is not a second managed format or standalone editor mode. Repository engineering documentation may use Markdown independently. See [`15-weftext-asciidoc-profile.md`](15-weftext-asciidoc-profile.md), [`06-application-ui.md`](06-application-ui.md), and [`../architecture/14-canonical-document-metadata-and-review.md`](../architecture/14-canonical-document-metadata-and-review.md).

Weftext / 文缕 is a knowledge-workspace product. It owns workspace storage, structured-source editing and reading, node operations, links, ordering, Chrono notes, annotations, derived indexes, sync-safe recovery, a local Desktop application, an intranet Server, a browser WebUI, identity and authorization, collaboration, backup/restore, and supervised AI-agent actions over those capabilities.

Workspace storage has three Core-owned content classes: managed `X/X.adoc` nodes with UUID identity, visible unmanaged directories/files/resources without node identity, and ignored content excluded from product discovery. An optional root `.weftext-rules` file is portable workspace authority for the latter two classes. Shells and Server transports consume Core classification and may not infer node status from filename shape, document bytes, `.gitignore`, or a frontend scan.

Production document conversion, OCR, mathematical/diagram rendering, and complete structured collection views are outside the current product boundary. Import uses a Weftext-owned format-neutral boundary; no external converter schema enters Core or the workspace. External converter invocation, wire contracts, compatibility layers, and joint release receipts are also outside this boundary. See [`../architecture/15-content-intake-foundation.md`](../architecture/15-content-intake-foundation.md), [`../architecture/16-pdf-import-and-ocr.md`](../architecture/16-pdf-import-and-ocr.md), and [`../architecture/07-collections-query-and-views.md`](../architecture/07-collections-query-and-views.md).

## AI agent boundary

AI agent harnesses integrate through a versioned, harness-neutral Weftext action and context boundary. DeepSeek Harness (DSH) is the first-tier supported harness. First-tier support means a Weftext-maintained adapter, an explicit compatibility matrix, capability negotiation, local and Server integration paths, streamed session state, cancellation, approval and audit behavior, and conformance tests. It does not mean that DSH is the only supported harness.

Weftext Core remains model- and harness-neutral. DSH runtime files, plugins, session logs, model configuration, and secrets are control-plane or device state and are not workspace authority. DSH is not a Cargo dependency of Core, and its preview protocol must not leak into persistent AsciiDoc or sidecar formats.

An agent never gains a second mutation path. The first-party integration does not grant a harness raw writable access to a Weftext workspace; create, edit, rename, move, copy, Trash, restore, annotation, and link changes all use the same revision-checked Core plans or transactions as the UI and CLI. In Server mode an agent is a delegated client whose effective capabilities cannot exceed those of the authenticated human actor.

## Architecture

```text
Desktop local mode ───────> Weftext Core ───────> local workspace backend
                                  ^
                                  |  (including approved agent actions)
WebUI / Desktop / CLI ─────> Weftext Server ────> hosted workspace backend
```

All mutations are expressed as Core plans or transactions. A shell must not implement independent filesystem semantics. In hosted mode clients never write the Server workspace directly; they use an authenticated, authorized Server API. A synchronized folder is supported for file replication but is not a multi-user collaboration mechanism.

Unmanaged and ignored content do not participate in node links, backlinks, graph, Chrono, node ordering, or node transactions. Ignored content remains physical workspace data for backup purposes; ignoring never deletes or rewrites it.

The public status and direction are summarized in [`../../ROADMAP.md`](../../ROADMAP.md). Runtime boundaries are defined in [`../architecture/01-runtime-architecture.md`](../architecture/01-runtime-architecture.md).
