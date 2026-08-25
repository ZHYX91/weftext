---
source_language: zh-CN
translation_status: source
---

[English](THIRD_PARTY_NOTICES.md)

# docling.rs Lite 第三方声明

经过审计的 Lite 配置将 docling.rs 固定为 v0.52.2、提交 `ca9fe7a543b55a540dfa18b88f4f44591b5a928e`（MIT）。已检查的资源锁是证据清单，不是下载器或已启用安装清单。

- RT-DETR layout Heron INT8 权重：docling-project，Apache-2.0。
- English PP-OCRv3 识别图和英文字典：PaddleOCR / RapidOCR ONNX 转换，Apache-2.0。
- PDFium 二进制分发：bblanchon/pdfium-binaries 和 Chromium PDFium，BSD-3-Clause，并附适用的 Chromium 第三方 Apache-2.0 声明。
- Microsoft ONNX Runtime 1.24.2 CPU 二进制：MIT。Rust 绑定 crate `ort`/`ort-sys` 单独固定为 2.0.0-rc.12。

每个重新分发的包都必须附带相应的完整上游许可证和声明文本。在目标工作进程二进制和 ONNX Runtime 库具备经审查的字节长度、SHA-256 摘要、许可证、声明、已安装字节一致性、封闭原生导入证据和目标操作系统沙箱证据之前，适配器保持不可用。
