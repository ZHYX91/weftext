---
source_language: zh-CN
translation_of: APPLE.zh-CN.md
translation_status: synced
---

[简体中文](APPLE.zh-CN.md)

# Apple native host

`platforms/apple` contains Swift host-library source for macOS 13/iOS 16 or later. It is not an installable Weftext product. The initial slice includes WebKit surface capability gates, fixed product assets and an original-byte Host bridge; no Mac compilation or runtime result has been obtained. Swift test source cannot establish macOS/iOS, LC10, IM10, IME or assistive-technology acceptance.

The actual composition root must inject `AppleHostPort`, complete `app/dist` bytes from the same product build and an explicitly selected WebKit data store into `NativeWebSurface`. The library provides no default business executor, HTTP proxy, placeholder Core or replacement editor. Swift neither decodes nor reencodes business JSON and maintains no second CallKind route table. The formal adapter must still enter the same Rust Host, fixed business registry and strict original-request decoder, authorizing before body parsing. Replies preserve their original typed bytes as well.

The bridge requires the exact WebView, main frame, `weftext-bundle://app` origin and current surface generation. Late replies after retirement or reopening cannot enter another document. There are at most 64 in-flight deliveries, with 96 MiB input and 256 MiB replies. This gate controls UI delivery only; it proves neither Core authorization nor persistence or safe closure. Retiring a surface does not cancel accepted submissions. Window closure/reloading still requires the actual owner's author-input-cut proof. An AppKit/UIKit application can ship only when the same owner connects these contracts.

The bundled scheme reads only an immutable construction-time allowlist, with no filesystem paths, query parameters, percent-encoded paths, external network or author-directory access. Each deliberately created document consumes one exact entry-navigation admission. Reloading, back/forward, JavaScript navigation and arbitrary queries/fragments cannot recreate the page outside the actual close workflow. CSP rejects network and frames. Navigation, modules, fonts and workers still require actual WebKit tests; policy source is not runtime evidence.

## Verification entry

```text
python scripts/apple.py check
python scripts/apple.py build --state-root <absolute-owned-output-directory> --developer-dir /Applications/Xcode_16.4.app/Contents/Developer
```

`check` verifies source inventory only. `build` requires actual macOS and an explicit installed Xcode. It records system, architecture, Xcode/SDK/Swift/Simulator inventories, then actually runs macOS Swift tests and an iOS Simulator SDK build. It does not download SDKs, accept licenses, change global Xcode selection, start existing Simulators, sign or publish. Each command has a deadline and stops only its owned process group, retaining original logs and source/artifact hashes. Builds and unit tests do not establish Simulator product execution or installation acceptance.

Formal Darwin/iOS Core storage qualification, the common Host C ABI, system file capabilities, macOS sandboxed XPC frozen workers, iOS WebContent/Wasm Engine, application lifecycle/input preservation, notifications and installation remain unconnected. An external picker/bookmark grants a particular file capability, not Core physical identity or a transactional backend. Preserve originals and use original-operation records for imports/exports. Undelivered capabilities must not appear available.

References: [WebKit reply messages](https://developer.apple.com/documentation/webkit/wkscriptmessagehandlerwithreply), [frame origins](https://developer.apple.com/documentation/webkit/wkframeinfo), [Apple directory capabilities](https://developer.apple.com/documentation/uikit/providing-access-to-directories), [Xcode Swift packages](https://developer.apple.com/documentation/xcode/creating-a-standalone-swift-package-with-xcode).
