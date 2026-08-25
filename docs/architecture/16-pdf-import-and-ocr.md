---
source_language: zh-CN
translation_of: 16-pdf-import-and-ocr.zh-CN.md
translation_status: synced
---

[简体中文](16-pdf-import-and-ocr.zh-CN.md)

# PDF import and local OCR

The PDF adapter uses a pinned local Lite worker with optional reviewed enhancement. It performs text extraction, bounded rasterization, layout analysis, local OCR, and table reconstruction, then maps worker output into Weftext Import IR. The worker schema is never Core or workspace authority.

## Package and adapter boundary

The default package includes only reviewed PDF, layout, OCR, runtime, model, dictionary, and notice artifacts. Optional capabilities must be separately packaged, licensed, pinned, and accepted; a model download is explicit and never an undisclosed default. The worker has no ambient network access.

```text
PDF bytes -> local worker -> worker-internal result -> PDF adapter
-> Weftext Import IR -> validated AsciiDoc proposal
```

The adapter preserves page and bounding-box provenance, uses embedded text before OCR where usable, and selects OCR per page/region when evidence requires it. Original-PDF retention is an explicit import-plan choice.

## OCR and enhancement boundary

Recognition profiles are validated against mixed CJK/Latin, rotation, vertical text, punctuation, tables, and low-resolution scans. A model override is not supported until preprocessing, dictionaries, tensors, packaging, licenses, and quality are verified together.

Optional enhancement receives only approved page/region evidence and the relevant IR fragment. It may propose reading-order repair, heading/OCR correction, table reconstruction, formula transcription, or figure description. The patch binds target IDs and base IR revision; whole-document replacement, stale, out-of-scope, or invalid patches are rejected.

## Acceptance

The corpus covers born-digital, scanned, mixed, CJK/Latin, multi-column, table/figure-heavy, rotated, malformed, encrypted, password-required, large, and adversarial PDFs. Evidence reports semantic quality, provenance, memory, latency, cancellation, cleanup, package size, license notices, and offline startup separately.
