---
source_language: zh-CN
translation_of: null
translation_status: source
---

[English](APPLE.md)

# Apple 原生宿主

`platforms/apple` 是面向 macOS 13／iOS 16 及以后系统的 Swift 宿主库，当前不是可安装的 Weftext 产品。[首轮 CI](https://github.com/ZHYX91/weftext/actions/runs/39703298589) 已在实际 macOS 15.7.9 arm64／Xcode 16.4／Swift 6.1.2 上通过编译与五项纯策略 XCTest，并完成 iOS Simulator SDK 的 arm64／x86_64 编译。[后续 CI](https://github.com/ZHYX91/weftext/actions/runs/39727579932) 又通过包含真实 WKWebView 的全部十二项宿主测试及 generic iOS Simulator SDK 构建；对应宿主源码未变。这些结果只覆盖宿主 fixture，没有运行 iOS Simulator 或完整 Rust Core 产品，不能关闭 LC10、IM10、输入法或辅助技术要求。

`NativeWebSurface` 必须由实际 composition root 注入 `AppleHostPort` 和同次产品构建的完整 `app/dist` 字节，且显式选择准确的 WebKit data store。库没有默认业务执行器、HTTP 代理、占位 Core 或替代编辑器。Swift 不解码或重新编码业务 JSON，不维护第二份 CallKind 路由表；正式适配器仍须进入同一个 Rust Host、固定业务注册表和严格原请求解码，获权在正文解析前。响应同样保留原 typed bytes。

桥仅接受准确 WebView 的 main frame、`weftext-bundle://app` origin 和当前 surface generation；失效或重开后的晚回包不能进入新页面。JavaScript 桥还将 Promise 的成功和失败交付同时绑定到原 Document 身份，防止旧回调借 WindowProxy 写入替换后的页面；原页面仍可观察时，退役继续明确拒绝为 `retiredSurface`。最多 64 个在途交付，输入 96 MiB、响应 256 MiB。该门只控制 UI 交付，不证明 Core 获权、持久化或关闭保稿。页面退役不取消已接受提交；窗口关闭／重载仍须实际 owner 的作者输入切面证明。只有同一个 owner 接齐这些合同后才能交付 AppKit／UIKit 应用。

包内 scheme 只读取构造时给定的不可变 allowlist，无文件路径、查询参数、百分号路径、外网或作者目录访问。每个明确创建的 document 只消费一次准确入口导航资格；reload、后退前进、JS 导航和任意 query／fragment 不能越过实际关闭流程重建页面。CSP 拒绝网络及 frame；实际 WebKit 上的导航、module／字体／worker 行为仍须测试，不以政策源码代替运行结果。

WebKit XCTest 直接创建真实 WKWebView，经实际脚本消息验证准确原始字节往返、坏 envelope／旧代次拒绝、导航拒绝后原页面保留，以及已接受 Host 工作在页面退役后继续完成。晚回包先在旧 DOM 仍可观察时必须实际收到 `retiredSurface`；另验新文档不接旧结果，不能用销毁旧 Promise 的上下文代替交付拒绝。子 frame 负例必须实际收到 `invalidOrigin`，CSP 负例必须观察 `connect-src` 违规事件，不能用网络失败或 iframe 没创建来冒充对应分支通过。fixture Host 仅记录字节，不执行或证明 Core 业务。每个 WebKit 回调与等待均有期限；上述后续 CI 已执行这些有界用例，完整应用与 Simulator 运行仍待接通和验收。

## 验证入口

```text
python scripts/apple.py check
python scripts/apple.py build --state-root <绝对独占输出目录> --developer-dir /Applications/Xcode_16.4.app/Contents/Developer
python scripts/apple.py build --state-root <绝对独占输出目录> --developer-dir /Applications/Xcode_16.4.app/Contents/Developer --run-simulator-tests
```

`check` 核源库存并运行纯 ownership guard 单测，不执行 Apple 工具。`build` 必须在真实 macOS 上使用已安装的明确 Xcode；保存系统、架构、Xcode／SDK／Swift／Simulator 清单，然后实际执行 macOS Swift／WebKit 单测和 iOS Simulator SDK 编译。源码冻结到本次输入目录，SwiftPM 的 `.swiftpm` 状态不写回检出。默认构建不下载 SDK、不接受许可、不改全局 Xcode 选择、不启动已有 Simulator，也不签名或发布。每个子命令有期限，仅停止本次独占 process group，保存原日志、源与产物摘要。构建和单测不表示 Simulator 产品运行或安装验收。

产物角色单独固定在本次 `roles/`：实际 macOS XCTest bundle 的 Mach-O 可执行文件、宿主可重定位对象和通用 iOS Simulator 宿主对象。各角色记录原路径、长度、SHA-256、magic、实际 `file`／`lipo`／`otool` 观察；缺失或歧义拒绝完成构建。编译失败也保留已存在的角色和原失败。测试 bundle 单个二进制只用于独立字节核验，不冒充可独立安装的应用或完整测试包；编译中间文件数量不证明产品角色。

`--run-simulator-tests` 是默认关闭的额外 OS 机制步骤。它重新核已安装 iOS 18.5 runtime、iPhone 16 类型和实际工具帮助，使用本次唯一 UUID 名称创建一次设备；原清单中已有的 UDID 不能接管。创建原回执先保存，再校原 UDID／名称／类型／runtime，全部 boot、测试和 shutdown 只用这个 UDID。创建结果未知时仅沿原唯一 tuple 查询，不重新创建或猜现有设备。

首次数据迁移与启动完成最多等 600 秒，实际 iOS XCTest 最多 600 秒，shutdown 最多 60 秒；结束必须重新观察原 tuple 的 `Shutdown`。失败或未知不标清理成功，不 erase、全局关闭、下载 runtime、接受许可、设置 team 或签名补救。额外步骤编译并运行同一套十二项 Swift／WebKit 用例，iOS 分支使用实际 UIKit XCTest runner，不能 skip 或用零项替代。原 `.xcresult` 有界归档及实际 iOS 测试 bundle 二进制也固定到 `roles/`。完整 Simulator 步骤是否通过以原运行回执为准；即使通过也只证明该模拟器的宿主库机制，不证明 Core／Engine、真机、IME／AT 或完整产品。

普通跨平台 ownership guard 单测可独立运行：`python platforms/apple/HarnessTests/test_simulator.py`；这些纯 JSON／原 tuple 反例不启动 Apple 工具，也不代替 OS 回执。

正式 Core 的 Darwin／iOS 存储资格、共同 Host C ABI、系统文件能力、macOS sandbox XPC frozen worker、iOS WebContent／Wasm Engine、应用生命周期与输入保全、通知及安装流程仍未接齐。外部 picker／bookmark 只能授予具体文件能力，不能冒充 Core 的物理文件身份或事务后端；原件必须保留，导入／导出需原操作记录。未交付能力不显示为可用。

依据：[WebKit 带回复消息](https://developer.apple.com/documentation/webkit/wkscriptmessagehandlerwithreply)、[页面来源](https://developer.apple.com/documentation/webkit/wkframeinfo)、[Apple 目录能力](https://developer.apple.com/documentation/uikit/providing-access-to-directories)、[Xcode 的 Swift package](https://developer.apple.com/documentation/xcode/creating-a-standalone-swift-package-with-xcode)。

iOS 18.5 Simulator 对早于 iOS 18.4 的部署目标存在已知 [WebKit Swift overlay 加载缺陷](https://bugs.webkit.org/show_bug.cgi?id=293831)。入口从原清单读取已核定 runtime bundle，要求其中已有 `System/Cryptexes/OS/usr/lib/swift/libswiftWebKit.dylib`，记录原库，并仅通过 `TEST_RUNNER_DYLD_FALLBACK_LIBRARY_PATH` 将其目录传给 XCTest runner。库缺失或已继承其他覆盖配置时拒绝执行。iOS 16 最低部署目标和全部真实 WebKit 用例保留，不修改已安装 runtime 或下载替代库。Xcode 的[测试进程环境前缀说明](https://developer.apple.com/documentation/xcode/environment-variable-reference)定义该传递方式。

CI workflow 使用标准 `macos-15-intel` runner 与 Xcode 16.4。入口记录实际 `uname` 架构与硬件资源，在 Simulator 启动前后收集虚拟内存统计。runtime 准入和 iOS 测试 bundle 角色必须匹配所观察的架构（`arm64` 或 `x86_64`）；generic SDK 构建仍要求双架构。Intel Simulator 通过不代表 ARM64 Simulator 或真机通过。

启动前通过所选 Xcode 解析一次 `simctl`、`lipo` 与 `otool`，核实工具位于该 Xcode 内并记录字节数与 SHA-256；后续模拟器和产物检查直接调用这些准确工具，不修改 SDK 或 `xcrun` 缓存。启动前后保留有时限的进程快照，只记录进程名与资源字段，不含参数或环境。工具选择不改变测试期限或原设备收尾要求。

首次 runtime 清单单独允许 180 秒冷启动预算，后续清单仍限 30 秒。runner-image 项目的 [issue 15247](https://github.com/actions/runner-images/issues/15247)记录了首次发现 runtime 可能耗时数分钟；该报告不证明任意单次运行的具体原因。每份进程快照仍限 15 秒，不可用时保留原失败命令和日志。仅在精确原进程组消失且 leader 已回收后，诊断失败才不阻断实际验证；清理未知或中断继续拒绝。SDK、Simulator 身份、XCTest 和最终 Shutdown 检查仍必须完成，不从诊断继承通过。

功能性的 boot 后设备清单仍须在原 30 秒命令期限内证明准确原 tuple 为 `Booted`。原子进程十秒后仍未回收时，仅对它的准确 PID 请求一次一秒、10 毫秒间隔且带 `-mayDie` 的 `sample`；采样器执行最多三秒，其进程组和 leader 的收尾共同受原清单的绝对期限约束。采样原日志和命令结果单独保留，采样清理未知或中断仍使本轮失败，清单 owner 始终收尾自身命令。单调阶段和 `profiled` 标记明确这是一轮诊断，不能用作无采样性能基线。参数依据为 [Apple 采样说明](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/LaunchTime/Articles/MeasuringLaunch.html)。选定工具记录保留报告／实际路径、符号链接状态和文件身份；至多 4 KiB 的原始脚本另保留只读原字节对应的有限文本，不为检查而执行该文本。

工具解析另记录已安装 CoreSimulator framework 原 `Info.plist` 的版本、摘要和身份，以及最多十六个顶层已安装 Xcode 的有限元数据。plist 最多读取 64 KiB、wrapper 最多 4 KiB；只读脚本中唯一明确的 `EXPECTED_VERSION` 做比较，不执行脚本来探测。缺失、歧义和版本不等均保留为观察，不成为运行通过或新增失败门禁；不切换 Xcode、不安装 first-launch 包、不修改 wrapper，也不放宽 Simulator 资格。
