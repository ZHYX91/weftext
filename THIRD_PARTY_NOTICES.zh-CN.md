---
source_language: zh-CN
translation_status: source
---

[English](THIRD_PARTY_NOTICES.md)

# 活动第三方依赖

活动源代码直接依赖以下 Rust crate 和打包 UI 组件：

- `serde` 和 `serde_json` — MIT OR Apache-2.0
- `sha2` — MIT OR Apache-2.0
- `uuid` — Apache-2.0 OR MIT
- `tempfile` — MIT OR Apache-2.0
- `saphyr-parser` — MIT OR Apache-2.0
- `file-id` — MIT OR Apache-2.0
- `same-file` — Unlicense OR MIT
- `jiff` — Unlicense OR MIT
- Hayagriva 和 Citationberg — MIT OR Apache-2.0
- Hayagriva 打包的 Citation Style Language 样式与区域设置 — Creative Commons Attribution-ShareAlike 3.0 Unported（CC BY-SA 3.0）；源项目：`citation-style-language/styles` 和 `citation-style-language/locales`
- Ciborium、Ciborium IO、Ciborium LL、Crunchy、Half、Numerals、Paste、Quick XML、Serde YAML、Unscanny、Unicode Language Identifier 和 Unsafe LibYAML — 采用各自打包源代码与 `Cargo.lock` 记录的宽松 MIT、Apache-2.0 或 MIT OR Apache-2.0 条款
- Axum — MIT
- Tokio 和 Tokio Stream — MIT
- Tauri、Tauri Build 和官方 Tauri Dialog 插件 — Apache-2.0 OR MIT
- React 和 React DOM — MIT
- CodeMirror 6 及直接使用的 state、view、language 模块 — MIT
- Vite 及其 React 插件（构建时）— MIT
- Vitest、Testing Library 和 jsdom（仅测试）— MIT
- Microsoft Edge WebView2 Runtime（Windows 系统/运行时前置依赖）— 由 Microsoft 按适用运行时条款分发和维护

传递依赖及精确版本记录在 `Cargo.lock` 中。发布打包必须根据实际解析并打包的工件生成声明和 SBOM。
