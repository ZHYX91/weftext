---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext Docling Lite worker

This separately versioned Rust 1.98 process package is outside the main Cargo workspace. It accepts no command-line arguments. It reads one bounded `weftext.import-worker-request.v1` object from standard input, accepts only the pinned PDF/English-OCR/INT8/no-TableFormer profile, reads only `input/source.pdf`, and writes one raw DoclingDocument 1.10.0 JSON object on success. Conversion failures are typed responses; malformed requests are bounded process-level protocol errors.

The worker never downloads runtime assets. A staged Windows package places the reviewed ONNX Runtime CPU library, models, and PDFium library beside the binary according to `release-profile.json`. The pinned build uses reviewed artifacts and offline Cargo mode; release scripts verify archive and file digests, direct dependencies, runtime assets, and native import tables before generating closed build evidence.

The worker does not provide an operating-system sandbox. The adapter stays unavailable until its supervisor proves network denial, memory limits, filesystem confinement, process-tree termination, licenses, notices, and matching installed bytes on the target operating system.
