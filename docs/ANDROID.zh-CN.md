---
source_language: zh-CN
translation_status: source
---

[English](ANDROID.md)

# Android 原生宿主

`platforms/android/host` 是 Kotlin/系统 WebView 的原生宿主库。它实现平台能力边界，当前还不是能离线工作的 Weftext APK：正式 Android Core 私有存储资格、同一个 App Host 的 JNI 接口、共同 Engine 的 frozen worker JNI 以及现有 TypeScript 前端的原生传输适配尚未接齐。库没有注册占位 Launcher、空业务 handler 或第二套 Core。AAR 编译和库测试不能关闭 v0.7 Mobile、IM10、LC10 验收。

## WebView 与生命周期

`NativeHostActivity` 要求 Application 的实际 composition root 提供 `NativeSurfaceHost`。每次创建或进程恢复都产生新 surface generation 和新的消息对象，旧 WebView 历史、Core Session、授权、作者字节和结果回执不会塞入 Android Bundle。Bundle 仅保存滚动位置；草稿恢复必须来自原 Core 的持久记录。返回操作等待实际 owner 的草稿保留/关闭资格；Activity 退役只撤 UI 回包能力，不能取消 Core 已接受的作者提交。`onStop` 通知 owner，系统栏与真实 IME insets 由系统回调调整。

WebView 只加载 APK assets 的 `https://appassets.androidplatform.net/`。文件/content 访问、混合内容、任意下载、页面权限和网络请求被拒绝；外链只能由实际用户手势交还宿主。消息 listener 同时核实际 origin、实际 main-frame 和存活 generation，不相信 payload 自报。老 generation 的回包被丢弃。每个 surface 最多八个在途请求；输入 96 MiB、回复 256 MiB。字符串严格转标准 UTF-8，非法 surrogate/UTF-8 被拒绝，非 BMP、NUL 和 CRLF 不经 JNI modified UTF-8 或 shell 改写。`weftext-native-surface` 仅通知平台消息对象，不能表示 Core ready。

最终 APK 必须把**同一产品构建**的 `app/dist` 原样打包到 `assets/weftext/`，包括资源与字体，并接正式前端 native transport。当前库不嵌入替代编辑器，不把浏览器 resize 或输入字面拼音当真实移动/IME 验收。

## SAF 原件与交付

`SafDocuments` 只接受系统 `ACTION_OPEN_DOCUMENT` / `ACTION_CREATE_DOCUMENT` 回传的 `content:` 能力。仅在实际 grant flags 允许时保留持久授权，Activity 关闭不会释放授权；显式撤销才调用 `forget`。SAF URI 不是 Core 库目录、物理文件身份或发给 Server 的授权路径。

导入原件只读流入 `noBackupFilesDir/android-original-inputs/<原请求号>.original`，以 create-new/0600/no-follow 发布并同步文件、目录；16 MiB 实读预算、长度与 SHA-256 验证独立于 provider 声明。失败保留部分证据，始终不删除或覆盖原件。暂存文件是纯输入，不是第二个业务仓库。

输出只写 CREATE_DOCUMENT 得到的目标，先冻结完整字节并核摘要，再由真实 owner 持久绑定原请求号、目标 URI、artifact 长度与摘要，才允许一次写尝试。已知非空目标拒写。输出上限 64 MiB，写后读回核长度/摘要；provider 拒绝、失联或不允许读回均保留 unknown。重建后调用 `verifyDelivery` **只读核对原 URI**，不能再打开原目标重写，也不自动另建文件。Verified 只证明观察到一致字节，不冒充 provider 的断电持久承诺。SAF 交付状态不能替代 Core 原操作状态。

## 隔离 worker

具体 worker Service 必须继承 `FrozenWorkerService` 并实现真实共同 Engine codec，manifest 声明 `isolatedProcess=true`、`exported=false`。主库不注册假 Engine service。每任务 `bindIsolatedService` 新实例，最多两个在途 OS 实例；只接受 frozen 输入，无 Core、库 token 或 SAF grant。96 MiB SharedMemory 在发送前撤所有可写映射并降为只读，worker 再拒绝可写映射；回复经有界 reliable pipe 传送，核任务号、长度、SHA-256 和 EOF。共同 Engine 的原始 JSON 仍要求 `frozen_files` map。

worker→owner Binder 回调绑定原任务号，owner 读取内核 calling PID/UID；回包必须匹配该观察。启动单调时间是 worker 报告，不能冒充独立内核创建时间。取消只对原任务/原 Binder，预启动取消也保留绑定直到精确死亡观察。worker 自身 watchdog 最多 60 秒后终止自己的隔离进程；不能按进程名杀 Core/宿主。`result` 完成和 `terminated` 的精确 Binder death 是两件事，超时不伪造死亡，也不放行未知旧实例的替代进程。

`androidTest` 的 Probe service 只测隔离 UID、私有文件/网络拒绝、字节管道、取消和 watchdog，明确不是 Engine 实现或产品功能验收。

## 构建与验收

单一入口：

```text
python scripts/android.py check
python scripts/android.py build --gradle-home <绝对Gradle8.13目录> --sdk <绝对SDK目录> --java-home <绝对JDK目录> --state-root <本工作区.workspace/build下独占目录>
```

默认离线；首次依赖下载须显式加 `--allow-dependency-downloads`。要求 SDK 35、build-tools 35.0.0、JDK 21、Gradle 8.13；固定 AGP 8.13.2、Kotlin 2.3.21、AndroidX Webkit 1.14.0，minSdk 30。入口不安装 SDK/NDK/目标、不接受许可证。Gradle 用户缓存、项目缓存、Kotlin 数据、临时文件和产物归独占 state，不借其他项目环境。

实际任务为 `:host:assembleDebug`、`:host:lintDebug`、`:host:testDebugUnitTest` 与 `:host:compileDebugAndroidTestKotlin`；最后一项只编 OS instrumentation fixture，不运行或安装设备 APK。

本轮 Windows 构建以挂起进程加入独占 Job 后再启动，涵盖 Gradle 的 single-use JVM。整树 4 GiB 内存上限、25 分钟期限、宿主可用内存 4 GiB 护栏只停止本次 owned tree，核所有子进程退出后才完成。结果保存本次源摘要、准确 argv、PID、资源峰值、原日志和 AAR 摘要；不把 launcher 退出当全部 Java 已退出。

实际阶段须分别记录：源静态检查；AAR/单元/lint；自有 headless AVD 的 OS 负例；真实 Core/JNI 离线编辑、原号恢复和导入导出；系统 IME 组合/选区/撤销、键盘、旋转、返回、进程重建、权限撤销、unknown 交付；双语实际 UI 与无障碍。未运行的阶段保持未验。模拟器须本工作区独占配置，不复用其他项目 AVD。

依据：[Android 原生桥安全](https://developer.android.com/privacy-and-security/risks/insecure-webview-native-bridges)、[SAF](https://developer.android.com/training/data-storage/shared/documents-files)、[SharedMemory](https://developer.android.com/reference/kotlin/android/os/SharedMemory)、[独立隔离绑定](https://developer.android.com/reference/android/content/Context#bindIsolatedService(android.content.Intent,int,java.lang.String,java.util.concurrent.Executor,android.content.ServiceConnection))、[AGP 8.13](https://developer.android.com/build/releases/agp-8-13-0-release-notes)。
