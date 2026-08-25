---
source_language: zh-CN
translation_of: 06-content-io-and-rich-rendering.zh-CN.md
translation_status: synced
---

[简体中文](06-content-io-and-rich-rendering.zh-CN.md)

# Content I/O and rich rendering

Managed source, import, export, and rich rendering are separate boundaries. Managed source is canonical `X/X.adoc`, node resources, and portable sidecars. Import produces a validated proposal; export reads authorized content without mutating it; rendering produces safe derived views without becoming source authority. See [metadata](14-canonical-document-metadata-and-review.md), [intake](15-content-intake-foundation.md), and [PDF/OCR](16-pdf-import-and-ocr.md).

## Export and compatibility

Export plans freeze the authorized source set, revisions, destination profile, resources, options, unsupported constructs, and output limits. Export is not backup and does not mutate a workspace. Markdown is compatibility output rather than a second managed generation; unsupported Weftext constructs require explicit lowering or warnings, and importing exported Markdown is a new reviewed conversion.

## Mathematics and diagrams

Native `latexmath:[...]`, `[latexmath]`, `stem:[...]`, `[stem]`, and AsciiMath remain exact source. Renderers are offline-capable derived views and must not execute document classes, packages, includes, BibTeX, shell escape, or external commands. TeX intake is bounded, never executes TeX, and reports unsupported commands/macros.

The accepted `[mermaid]` literal-block extension preserves exact source. Its renderer is pinned, isolated from filesystem/network access, size/time bounded, sanitized before insertion, and paired with source, error, and accessibility fallbacks. Math and diagrams may share presentation infrastructure but stay distinct Core block types and renderer adapters.

## Publication structures and format adapters

Native AsciiDoc notes, anchors, tables, figures, equations, listings, and attributes are preferred when they express the meaning. Numbering, labels, formatted citations, and view rows are derived. Destination-specific layout belongs to export profiles and is never silently written back to source.

Adapters use the common intake boundary. Structured parsers are preferred when they preserve more meaning than layout conversion. Each adapter needs malicious/corrupt fixtures, deterministic validation, cancellation/cleanup, package/license evidence, accessible diagnostics, and an honest loss report.
