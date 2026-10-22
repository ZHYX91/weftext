---
source_language: zh-CN
translation_of: ANDROID.zh-CN.md
translation_status: synced
---

[简体中文](ANDROID.zh-CN.md)

# Android native host

`platforms/android/host` is a Kotlin/system-WebView host library. It implements platform capability boundaries, and is not yet an offline Weftext APK: qualified Android private storage, JNI for the same App Host, the shared Engine's frozen-worker JNI and the existing TypeScript frontend's native transport remain to be connected. The library registers no placeholder Launcher, empty business handler or second Core. AAR compilation and library tests do not close v0.7 Mobile, IM10 or LC10 acceptance.

## WebView and lifecycle

`NativeHostActivity` requires the Application's real composition root to supply `NativeSurfaceHost`. Creation and process restoration each create a fresh surface generation and message object. Android Bundles never retain WebView history, Core Sessions, authorization, authored bytes or result receipts. Only scroll position is saved; draft recovery must use the original Core's durable records. Back waits for the real owner's draft-retention/close qualification. Retiring an Activity revokes UI reply delivery, without cancelling author submissions already accepted by Core. `onStop` notifies the owner; system callbacks provide system-bar and actual IME insets.

WebView loads only APK assets at `https://appassets.androidplatform.net/`. File/content access, mixed content, arbitrary downloads, page permissions and network requests are denied. Only a real user gesture can hand an external link to the host. The listener checks actual origin, actual main frame and live generation, without trusting payload claims. Replies for retired generations are discarded. Each surface permits eight in-flight calls; input is bounded to 96 MiB, replies to 256 MiB. Strict standard UTF-8 rejects malformed surrogates/UTF-8; astral characters, NUL and CRLF are not rewritten through JNI modified UTF-8 or a shell. `weftext-native-surface` announces the platform message object only; it cannot mean Core ready.

The final APK must package the **same product build's** `app/dist` unchanged under `assets/weftext/`, including resources and fonts, and connect the formal frontend native transport. This library embeds no substitute editor. Browser resizing and literal pinyin do not establish actual mobile/IME acceptance.

## SAF originals and delivery

`SafDocuments` accepts `content:` capabilities returned by system `ACTION_OPEN_DOCUMENT` / `ACTION_CREATE_DOCUMENT`. Persistence is attempted only when actual grant flags permit it. Activity closure retains grants; explicit revocation calls `forget`. A SAF URI is not a Core library directory, physical file identity or authorization path to send to Server.

Originals stream read-only into `noBackupFilesDir/android-original-inputs/<original-request-id>.original`, using create-new/0600/no-follow and synchronizing file and directory. A 16 MiB actual-read budget, length and SHA-256 are independent of provider declarations. Failures preserve partial evidence; originals are never deleted or overwritten. Staged files are pure inputs, not another business store.

Output accepts only a CREATE_DOCUMENT target. Complete bytes are frozen and verified before the real owner durably binds the original request ID, URI, artifact length and digest, permitting one write attempt. Known nonempty targets are refused. Output is bounded to 64 MiB and read back for length/digest verification. Provider rejection, loss or unavailable readback preserves unknown. After reconstruction, `verifyDelivery` **only reads the original URI**; it never rewrites that target or automatically creates another file. Verified means matching observed bytes, without claiming provider power-loss durability. SAF delivery state does not replace Core's original-operation state.

## Isolated workers

A concrete Service must extend `FrozenWorkerService`, implement the real shared Engine codec, and declare `isolatedProcess=true` / `exported=false`. The main library registers no fake Engine service. Each job uses a fresh `bindIsolatedService` instance, with at most two in-flight OS instances. It receives frozen input only, without Core, library tokens or SAF grants. The 96 MiB SharedMemory has all writable mappings removed and protection reduced to read-only before sending; the worker also refuses writable mappings. Bounded reliable-pipe replies verify job ID, length, SHA-256 and EOF. The shared Engine's original JSON still requires a `frozen_files` map.

A worker-to-owner Binder callback binds the original job ID. The owner reads kernel calling PID/UID, and the reply must match that observation. Monotonic start time is worker-reported, without claiming an independent kernel creation timestamp. Cancellation addresses only the original job/Binder. Pre-start cancellation keeps the binding until exact death is observed. The worker's own watchdog terminates its isolated process within 60 seconds; no process-name killing of Core/host is used. `result` completion and `terminated` exact Binder death are separate facts. A timeout never fabricates death or permits replacement of an unconfirmed old instance.

The `androidTest` Probe service tests isolated UID, private-file/network denial, byte pipes, cancellation and watchdog only. It is explicitly not an Engine implementation or product-feature acceptance.

## Build and acceptance

One entry:

```text
python scripts/android.py check
python scripts/android.py build --gradle-home <absolute-Gradle8.13-directory> --sdk <absolute-SDK-directory> --java-home <absolute-JDK-directory> --state-root <owned-child-of-this-workspace-.workspace/build>
```

Offline is the default; initial dependency downloads require `--allow-dependency-downloads`. Requirements: SDK 35, build-tools 35.0.0, JDK 21 and Gradle 8.13; pinned AGP 8.13.2, Kotlin 2.3.21 and AndroidX Webkit 1.14.0; minSdk 30. The entry never installs SDKs/NDKs/targets or accepts licenses. Gradle user/project caches, Kotlin state, temporary files and artifacts belong to the explicit state directory, without borrowing another project's environment.

Tasks are `:host:assembleDebug`, `:host:lintDebug`, `:host:testDebugUnitTest` and `:host:compileDebugAndroidTestKotlin`. The last only compiles OS instrumentation fixtures, without installing or running a device APK.

This Windows build admits a suspended launcher into an exclusive Job before execution, covering Gradle's single-use JVM. A 4 GiB aggregate memory limit, 25-minute deadline and 4 GiB host-free-memory guard stop only this owned tree and observe all children exiting before completion. Results retain source digests, exact argv, PID, resource peaks, raw log and AAR digests. Launcher exit alone does not establish all Java processes exited.

Record separate stages: source checks; AAR/unit/lint; OS negatives on an independently owned headless AVD; real Core/JNI offline editing, original-operation recovery and import/export; system IME composition/selection/Undo, keyboard, rotation, Back, process recreation, grant revocation and unknown delivery; actual bilingual UI and accessibility. Unrun stages remain unverified. The emulator must use this workspace's isolated configuration, without another project's AVD.

References: [Android native-bridge security](https://developer.android.com/privacy-and-security/risks/insecure-webview-native-bridges), [SAF](https://developer.android.com/training/data-storage/shared/documents-files), [SharedMemory](https://developer.android.com/reference/kotlin/android/os/SharedMemory), [isolated binding](https://developer.android.com/reference/android/content/Context#bindIsolatedService(android.content.Intent,int,java.lang.String,java.util.concurrent.Executor,android.content.ServiceConnection)), [AGP 8.13](https://developer.android.com/build/releases/agp-8-13-0-release-notes).
