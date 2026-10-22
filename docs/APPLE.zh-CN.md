---
source_language: zh-CN
translation_of: null
translation_status: source
---

[English](APPLE.md)

# Apple 原生宿主

`platforms/apple` 是面向 macOS 13／iOS 16 及以后系统的 Swift 宿主库源码，当前不是可安装的 Weftext 产品。首个切片包含 WebKit 页面能力门、固定产品资源和原始 Host 字节桥；尚未获得 Mac 编译或运行结果。Swift 单测源码不能作为 macOS／iOS、LC10、IM10、输入法或辅助技术通过。

`NativeWebSurface` 必须由实际 composition root 注入 `AppleHostPort` 和同次产品构建的完整 `app/dist` 字节，且显式选择准确的 WebKit data store。库没有默认业务执行器、HTTP 代理、占位 Core 或替代编辑器。Swift 不解码或重新编码业务 JSON，不维护第二份 CallKind 路由表；正式适配器仍须进入同一个 Rust Host、固定业务注册表和严格原请求解码，获权在正文解析前。响应同样保留原 typed bytes。

桥仅接受准确 WebView 的 main frame、`weftext-bundle://app` origin 和当前 surface generation；失效或重开后的晚回包不能进入新页面。最多 64 个在途交付，输入 96 MiB、响应 256 MiB。该门只控制 UI 交付，不证明 Core 获权、持久化或关闭保稿。页面退役不取消已接受提交；窗口关闭／重载仍须实际 owner 的作者输入切面证明。只有同一个 owner 接齐这些合同后才能交付 AppKit／UIKit 应用。

包内 scheme 只读取构造时给定的不可变 allowlist，无文件路径、查询参数、百分号路径、外网或作者目录访问。每个明确创建的 document 只消费一次准确入口导航资格；reload、后退前进、JS 导航和任意 query／fragment 不能越过实际关闭流程重建页面。CSP 拒绝网络及 frame；实际 WebKit 上的导航、module／字体／worker 行为仍须测试，不以政策源码代替运行结果。

## 验证入口

```text
python scripts/apple.py check
python scripts/apple.py build --state-root <绝对独占输出目录> --developer-dir /Applications/Xcode_16.4.app/Contents/Developer
```

`check` 只核源库存。`build` 必须在真实 macOS 上使用已安装的明确 Xcode；保存系统、架构、Xcode／SDK／Swift／Simulator 清单，然后实际执行 macOS Swift 单测和 iOS Simulator SDK 编译。它不下载 SDK、不接受许可、不改全局 Xcode 选择、不启动已有 Simulator，也不签名或发布。每个子命令有期限，仅停止本次独占 process group，保存原日志、源与产物摘要。构建和单测不表示 Simulator 产品运行或安装验收。

正式 Core 的 Darwin／iOS 存储资格、共同 Host C ABI、系统文件能力、macOS sandbox XPC frozen worker、iOS WebContent／Wasm Engine、应用生命周期与输入保全、通知及安装流程仍未接齐。外部 picker／bookmark 只能授予具体文件能力，不能冒充 Core 的物理文件身份或事务后端；原件必须保留，导入／导出需原操作记录。未交付能力不显示为可用。

依据：[WebKit 带回复消息](https://developer.apple.com/documentation/webkit/wkscriptmessagehandlerwithreply)、[页面来源](https://developer.apple.com/documentation/webkit/wkframeinfo)、[Apple 目录能力](https://developer.apple.com/documentation/uikit/providing-access-to-directories)、[Xcode 的 Swift package](https://developer.apple.com/documentation/xcode/creating-a-standalone-swift-package-with-xcode)。
