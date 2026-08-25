---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Weftext WebUI interaction prototype

This prototype validates the shared Desktop/WebUI interaction model with simulated data or one explicitly granted local Weftext workspace. It is not the Weftext Server and is not product-completion evidence.

## Local Core workspace slice

```text
weftext prototype serve D:\path\to\Workspace
```

The command binds only to `127.0.0.1`, creates a random bearer token, and prints an `openUrl`. Open the complete URL; do not copy its token into a query string.

In local mode the prototype reads exact UTF-8 AsciiDoc and revisions through Core, navigates the real node tree, keeps a controlled draft per visited node, parses the current draft for structured presentation, hides the protected identity envelope outside Source view, produces a deterministic save preview, and commits through the same Core action as the CLI. Structural actions, links, potential mentions, search, stale-revision refusal, and content-boundary rules also remain Core-owned. The browser receives no directory handle and cannot select arbitrary filesystem paths.

The prototype does not define a second document format or an additional standalone editor mode. Missing production surfaces remain governed by the public specifications.

## Validation

```text
npm run lint
npm test
```
