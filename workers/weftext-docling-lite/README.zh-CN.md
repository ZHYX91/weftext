---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Weftext Docling Lite 工作进程

这是一个独立版本化、位于主 Cargo 工作区之外的 Rust 1.98 进程包。它不接受命令行参数，从标准输入读取一个有边界的 `weftext.import-worker-request.v1` 对象，只接受固定的 PDF/英文 OCR/INT8/无 TableFormer 配置，只读取 `input/source.pdf`，成功时输出一个原始 DoclingDocument 1.10.0 JSON 对象。转换失败使用类型化响应；格式错误的请求产生有边界的进程级协议错误。

工作进程不会下载运行时资源。按 `release-profile.json` 暂存的 Windows 包把经过审查的 ONNX Runtime CPU 库、模型和 PDFium 库放在二进制旁。固定构建使用经过审查的工件和 Cargo 离线模式；发布脚本在生成封闭构建证据前验证压缩包与文件摘要、直接依赖、运行时资源和原生导入表。

工作进程本身不提供操作系统沙箱。在监督进程证明目标操作系统上的网络隔离、内存上限、文件系统限制、进程树终止、许可证、声明和已安装字节一致之前，适配器保持不可用。
