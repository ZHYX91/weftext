---
source_language: zh-CN
translation_of: THIRD_PARTY_NOTICES.zh-CN.md
translation_status: synced
---

[简体中文](THIRD_PARTY_NOTICES.zh-CN.md)

# Active third-party dependencies

The active source directly depends on the following Rust crates and packaged UI components:

- `serde` and `serde_json` — MIT OR Apache-2.0
- `sha2` — MIT OR Apache-2.0
- `uuid` — Apache-2.0 OR MIT
- `tempfile` — MIT OR Apache-2.0
- `saphyr-parser` — MIT OR Apache-2.0
- `file-id` — MIT OR Apache-2.0
- `same-file` — Unlicense OR MIT
- `jiff` — Unlicense OR MIT
- Hayagriva and Citationberg — MIT OR Apache-2.0
- Citation Style Language styles and locales packaged by Hayagriva — Creative Commons Attribution-ShareAlike 3.0 Unported (CC BY-SA 3.0); source projects: `citation-style-language/styles` and `citation-style-language/locales`
- Ciborium, Ciborium IO, Ciborium LL, Crunchy, Half, Numerals, Paste, Quick XML, Serde YAML, Unscanny, Unicode Language Identifier, and Unsafe LibYAML — permissive MIT, Apache-2.0, or MIT OR Apache-2.0 terms as recorded by their packaged sources and `Cargo.lock`
- Axum — MIT
- Tokio and Tokio Stream — MIT
- Tauri, Tauri Build, and the official Tauri Dialog plugin — Apache-2.0 OR MIT
- React and React DOM — MIT
- CodeMirror 6 and the directly used state, view, and language modules — MIT
- Vite and its React plugin (build-time) — MIT
- Vitest, Testing Library, and jsdom (test-only) — MIT
- Microsoft Edge WebView2 Runtime (Windows system/runtime prerequisite) — distributed and serviced by Microsoft under its applicable runtime terms

Transitive dependencies and exact versions are recorded in `Cargo.lock`. Release packaging must generate notices and an SBOM from the actual resolved and packaged artifacts.
