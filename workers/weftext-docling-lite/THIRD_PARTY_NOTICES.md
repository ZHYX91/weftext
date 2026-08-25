---
source_language: zh-CN
translation_of: THIRD_PARTY_NOTICES.zh-CN.md
translation_status: synced
---

[简体中文](THIRD_PARTY_NOTICES.zh-CN.md)

# Weftext Docling Lite worker third-party notice entry

This package is distributed under `AGPL-3.0-or-later` as part of Weftext. A
release bundle must include the repository root `LICENSE`, this notice file,
the exact `Cargo.lock`, the generated build-evidence JSON, and the complete
redistributable upstream license/notice files for every native library, model,
dictionary, and Cargo dependency. The current Windows evidence marks the package
incomplete because those redistributed texts are not yet staged and digest-bound.

The worker directly incorporates or loads the following reviewed components:

| Component | Pinned version/artifact | License | Upstream |
| --- | --- | --- | --- |
| docling.rs | 0.52.2, commit `ca9fe7a543b55a540dfa18b88f4f44591b5a928e` | MIT | <https://github.com/docling-project/docling.rs> |
| ONNX Runtime | Microsoft CPU binary 1.24.2, dynamically linked; Rust bindings `ort`/`ort-sys` 2.0.0-rc.12 | MIT | <https://github.com/microsoft/onnxruntime> |
| PDFium | Chromium 8009 reviewed target binary | BSD-3-Clause plus bundled third-party notices | <https://github.com/bblanchon/pdfium-binaries> |
| Docling Heron INT8 layout model | `layout_heron_int8.onnx` reviewed digest | Apache-2.0 | <https://github.com/docling-project/docling.rs/releases/tag/models-v1> |
| RapidOCR / PP-OCRv3 English model | `en_PP-OCRv3_rec_infer.onnx` reviewed digest | Apache-2.0 | <https://huggingface.co/SWHL/RapidOCR> |
| PaddleOCR English dictionary | `en_dict.txt` reviewed digest | Apache-2.0 | <https://github.com/PaddlePaddle/PaddleOCR> |

`Cargo.lock` is the dependency-resolution SBOM for this isolated package. The
release pipeline must additionally generate a complete transitive license
report and ship every required license text; this short entry is not a
substitute for that generated report.
