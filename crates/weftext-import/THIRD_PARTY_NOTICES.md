---
source_language: zh-CN
translation_of: THIRD_PARTY_NOTICES.zh-CN.md
translation_status: synced
---

[简体中文](THIRD_PARTY_NOTICES.zh-CN.md)

# docling.rs Lite third-party notices

The audited Lite profile pins docling.rs v0.52.2 at commit
`ca9fe7a543b55a540dfa18b88f4f44591b5a928e` (MIT). The checked asset lock is
an evidence inventory, not a downloader or an enabled installation manifest.

- RT-DETR layout Heron INT8 weights: docling-project, Apache-2.0.
- English PP-OCRv3 recognition graph and English dictionary: PaddleOCR / the
  RapidOCR ONNX conversion, Apache-2.0.
- PDFium binary distribution: bblanchon/pdfium-binaries and Chromium PDFium,
  BSD-3-Clause with applicable Chromium third-party Apache-2.0 notices.
- Microsoft ONNX Runtime 1.24.2 CPU binary, MIT. The Rust binding crates
  `ort`/`ort-sys` remain separately pinned to 2.0.0-rc.12.

Every redistributed package must ship the corresponding full upstream license
and notice texts. The adapter remains unavailable until the target worker binary
and ONNX Runtime library also have reviewed byte lengths, SHA-256 digests,
licenses, notices, matching installed bytes, closed native-import evidence, and
target operating-system sandbox evidence.
