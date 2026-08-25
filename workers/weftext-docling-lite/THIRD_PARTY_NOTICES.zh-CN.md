---
source_language: zh-CN
translation_status: source
---

[English](THIRD_PARTY_NOTICES.md)

# Weftext Docling Lite 工作进程第三方声明条目

本包作为 Weftext 的一部分按 `AGPL-3.0-or-later` 分发。发布包必须包含仓库根目录 `LICENSE`、本声明文件、精确的 `Cargo.lock`、生成的构建证据 JSON，以及每个原生库、模型、字典和 Cargo 依赖的完整可再分发上游许可证/声明文件。当前 Windows 证据把包标记为不完整，因为这些重新分发文本尚未暂存并绑定摘要。

工作进程直接包含或加载以下经过审查的组件：

| 组件 | 固定版本/工件 | 许可证 | 上游 |
| --- | --- | --- | --- |
| docling.rs | 0.52.2，提交 `ca9fe7a543b55a540dfa18b88f4f44591b5a928e` | MIT | <https://github.com/docling-project/docling.rs> |
| ONNX Runtime | Microsoft CPU 二进制 1.24.2，动态链接；Rust 绑定 `ort`/`ort-sys` 2.0.0-rc.12 | MIT | <https://github.com/microsoft/onnxruntime> |
| PDFium | Chromium 8009 经审查目标二进制 | BSD-3-Clause 及附带第三方声明 | <https://github.com/bblanchon/pdfium-binaries> |
| Docling Heron INT8 布局模型 | `layout_heron_int8.onnx` 经审查摘要 | Apache-2.0 | <https://github.com/docling-project/docling.rs/releases/tag/models-v1> |
| RapidOCR / PP-OCRv3 英文模型 | `en_PP-OCRv3_rec_infer.onnx` 经审查摘要 | Apache-2.0 | <https://huggingface.co/SWHL/RapidOCR> |
| PaddleOCR 英文字典 | `en_dict.txt` 经审查摘要 | Apache-2.0 | <https://github.com/PaddlePaddle/PaddleOCR> |

`Cargo.lock` 是这个隔离包的依赖解析 SBOM。发布流水线还必须生成完整的传递许可证报告并附带全部必需许可证文本；本简要条目不能替代生成的报告。
