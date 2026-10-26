---
source_language: zh-CN
translation_of: APPLE.zh-CN.md
translation_status: synced
---

[简体中文](APPLE.zh-CN.md)

# Apple native host

`platforms/apple` is a Swift host library for macOS 13/iOS 16 or later, not an installable Weftext product. The [first CI run](https://github.com/ZHYX91/weftext/actions/runs/39703298589) compiled on actual macOS 15.7.9 arm64/Xcode 16.4/Swift 6.1.2, passed five pure-policy XCTest cases and compiled against the iOS Simulator SDK for arm64/x86_64. A [later CI run](https://github.com/ZHYX91/weftext/actions/runs/39727579932) passed all twelve host tests, including real WKWebViews, and the generic iOS Simulator SDK build; the corresponding host sources are unchanged. These results cover host fixtures only, without executing an iOS Simulator or the complete Rust Core product, and cannot close LC10, IM10, IME or assistive-technology requirements.

The actual composition root must inject `AppleHostPort`, complete `app/dist` bytes from the same product build and an explicitly selected WebKit data store into `NativeWebSurface`. The library provides no default business executor, HTTP proxy, placeholder Core or replacement editor. Swift neither decodes nor reencodes business JSON and maintains no second CallKind route table. The formal adapter must still enter the same Rust Host, fixed business registry and strict original-request decoder, authorizing before body parsing. Replies preserve their original typed bytes as well.

The bridge requires the exact WebView, main frame, `weftext-bundle://app` origin and current surface generation. Late replies after retirement or reopening cannot enter another document. The JavaScript bridge also binds both Promise success and failure to the original Document identity: an old callback cannot use a WindowProxy to write into its replacement. Retirement in the still-observable original document continues to reject with `retiredSurface`. There are at most 64 in-flight deliveries, with 96 MiB input and 256 MiB replies. This gate controls UI delivery only; it proves neither Core authorization nor persistence or safe closure. Retiring a surface does not cancel accepted submissions. Window closure/reloading still requires the actual owner's author-input-cut proof. An AppKit/UIKit application can ship only when the same owner connects these contracts.

The bundled scheme reads only an immutable construction-time allowlist, with no filesystem paths, query parameters, percent-encoded paths, external network or author-directory access. Each deliberately created document consumes one exact entry-navigation admission. Reloading, back/forward, JavaScript navigation and arbitrary queries/fragments cannot recreate the page outside the actual close workflow. CSP rejects network and frames. Navigation, modules, fonts and workers still require actual WebKit tests; policy source is not runtime evidence.

The WebKit XCTest cases construct real WKWebViews and use actual script messages to verify original-byte round trips, invalid-envelope/stale-generation refusal, preservation of a live document after rejected navigation, and accepted Host work completing after UI retirement. A late reply must actually receive `retiredSurface` while the old DOM is still observable; a separate case checks isolation of the new document. Destroying the old Promise's context cannot substitute for delivery refusal. The subframe case must actually receive `invalidOrigin`; the CSP case must observe a `connect-src` violation. A network failure or an absent iframe cannot substitute for those branches. The fixture Host records bytes only and proves no Core business behavior. WebKit callbacks and waits have deadlines; the later CI run executed these bounded cases, while complete application and Simulator execution still require integration and acceptance.

## Verification entry

```text
python scripts/apple.py check
python scripts/apple.py build --state-root <absolute-owned-output-directory> --developer-dir /Applications/Xcode_16.4.app/Contents/Developer
python scripts/apple.py build --state-root <absolute-owned-output-directory> --developer-dir /Applications/Xcode_16.4.app/Contents/Developer --run-simulator-tests
```

`check` verifies source inventory and runs pure ownership-guard tests without executing Apple tools. `build` requires actual macOS and an explicit installed Xcode. It records system, architecture, Xcode/SDK/Swift/Simulator inventories, then runs macOS Swift/WebKit tests and an iOS Simulator SDK build. Sources are fixed in this run's input directory; SwiftPM `.swiftpm` state does not modify the checkout. The default build does not download SDKs, accept licenses, change global Xcode selection, start existing Simulators, sign or publish. Each command has a deadline and stops only its owned process group, retaining original logs and source/artifact hashes. Builds and unit tests do not establish Simulator product execution or installation acceptance.

Actual roles are fixed separately in the run's `roles/` directory: the macOS XCTest bundle's Mach-O executable, relocatable host objects and universal iOS Simulator host object. Each records its source path, size, SHA-256, magic and actual `file`/`lipo`/`otool` observations. Missing or ambiguous roles refuse a completed build. Failed compilations retain available roles and the original failure. The single test-bundle binary supports independent byte verification, not standalone installation or a complete runnable test bundle. Intermediate-file counts do not establish product roles.

`--run-simulator-tests` adds an OS mechanism step that is disabled by default. It requalifies the installed iOS 18.5 runtime, iPhone 16 type and actual tool help, then creates one device with this attempt's unique UUID name. A UDID present in the original inventory cannot be adopted. The original create receipt is saved before checking its UDID/name/type/runtime. Boot, testing and shutdown use only that UDID. Unknown creation is reconciled through the original unique tuple without recreating or guessing an existing device.

Boot completion is limited to 180 seconds, actual iOS XCTest to 600 and shutdown to 60. Cleanup requires a fresh `Shutdown` observation of the original tuple. Failures or unknowns cannot report clean completion; the entry performs no erase, global shutdown, runtime download, license acceptance, team configuration or signing workaround. This step compiles and runs the same twelve Swift/WebKit cases using the actual UIKit XCTest runner on iOS, without skipping or substituting zero tests. A bounded archive of the original `.xcresult` and the actual iOS test-bundle binary are fixed in `roles/` too. This Simulator step has not yet run. A pass would prove only that Simulator's host-library mechanisms, not Core/Engine, physical devices, IME/AT or the full product.

Run pure cross-platform ownership-guard tests separately with `python platforms/apple/HarnessTests/test_simulator.py`. These JSON/original-tuple negative cases launch no Apple tools and cannot replace OS receipts.

Formal Darwin/iOS Core storage qualification, the common Host C ABI, system file capabilities, macOS sandboxed XPC frozen workers, iOS WebContent/Wasm Engine, application lifecycle/input preservation, notifications and installation remain unconnected. An external picker/bookmark grants a particular file capability, not Core physical identity or a transactional backend. Preserve originals and use original-operation records for imports/exports. Undelivered capabilities must not appear available.

References: [WebKit reply messages](https://developer.apple.com/documentation/webkit/wkscriptmessagehandlerwithreply), [frame origins](https://developer.apple.com/documentation/webkit/wkframeinfo), [Apple directory capabilities](https://developer.apple.com/documentation/uikit/providing-access-to-directories), [Xcode Swift packages](https://developer.apple.com/documentation/xcode/creating-a-standalone-swift-package-with-xcode).
