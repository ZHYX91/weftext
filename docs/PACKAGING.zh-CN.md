---
source_language: zh-CN
translation_of: null
translation_status: source
---

# Windows 候选打包

产品唯一入口仍是 `scripts/mvp.py`。Windows 桌面 `weftext.exe` 和终端 `weftext-cli.exe` 必须来自同次 Cargo 构建及同一份源码；文件改名不能代替真实 PE 子系统。两程序都承载原编译 worker 协议，Server／CLI／worker 不由 GUI wrapper 再启动第二套服务。

`build`／`check` 使用当前源码；`package` 先核完整离线材料与 SDK，再把公开产品源码冻结到实际 Cargo target 下本次独有的输入目录，后续构建和同一套检查都在该副本运行。源码归档来自这份固定字节，嵌入前端另记准确清单。不会在构建后重新扫描正在编辑的工作树来冒充同源产物。源码、前端、两个程序、依赖原许可或材料资格变化会拒绝打包；失败输出保留，不回退旧 target 或旧单程序包。

共享 target 的 Rust 编译另由同入口核内容资格：实际 Rust／Cargo、本地 path（含 vendor）源码、构建脚本和嵌入资源、命令／源码根上下文、上次成功产物及本地生成目录内容必须相符。未知或变更只强制相关本地入口重编，未变 registry 依赖继续复用；不 clean 或另建每检出冷缓存。target 内的输入提示仅为可丢弃缓存，先置未决，命令成功且输入／产物核准后才记成功；失败不冒新源码已编译。产品锁内完成双程序固定副本，再释放给同入口下一次构建。具体锁顺序和直接 Cargo 边界见 [WORKFLOW](WORKFLOW.zh-CN.md)，此机制不替代候选验收。

首次打包要求实际产品 Git 检出及 Git 工具，按 `git ls-files -c -o --exclude-standard -z` 选择已跟踪和未忽略的公开文件，读取当前工作树字节；未提交修改、新增和删除均反映到本次快照，忽略的本地材料不会被扫描读取。源码归档不包含 `.git`，解压后仍可独立运行 `build`／`check`；任意无 Git 目录重新打包不属于支持入口。

Windows 构建完成后，入口在实际 target 下新建同批产物目录，把两个最终程序及 `artifact-catalog.json` 固定在一起，后续检查和打包使用该角色映射。使用入口打印的这两个准确路径；原 mutable Cargo debug／release 输出不是本次验收角色映射。release 固定副本和最终候选属于交付产物，不纳入自动 temp 清理。清单包含 `schemaVersion`、`architecture`、`sourceSha256`、`hostContractSha256`、`buildId` 及 `roles`；desktop／cli 各记录固定文件名、准确 SHA-256／字节长度和 PE 子系统。`buildId` 为源码归档摘要、合同摘要、desktop 摘要、cli 摘要四个小写十六进制 ASCII 字符串依次以 LF 连接后的 SHA-256，末尾无 LF。宿主从系统返回的真实进程映像核对本批角色，不能信任 peer 自报身份或目录名。跨构建升级另行协调，不因目录相同而自动接管。

```text
python scripts/mvp.py build
python scripts/mvp.py check
python scripts/mvp.py package --release --windows-sdk "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64" --offline-materials "D:\Weftext-Materials" --msvc-linker "C:\Path\To\MSVC\bin\Hostx64\x64\link.exe"
```

示例材料／linker 路径需替换为本次准确输入。打包显式使用该实际 x64 MSVC linker，并要求离线 VC 版本不早于其真实文件版本。构建及依赖获取仍需已有 Node／Rust 工具链；生成阶段不下载、运行安装器、签名、安装或改信任／注册表。没有跳过角色、许可、锁或离线签名检查的开关。

CI 可用 `python scripts/prepare_windows_package.py --materials ABSOLUTE_NEW_DIRECTORY --output ABSOLUTE_NEW_JSON` 准备输入。独立获取步骤仅请求锁中的官方最终 URL，约束长度和读取，核准确 SHA-256 与原许可，再执行相同 cache-only 签名资格。每轮新建路径；失败保留部分原件与请求记录，不采用旧下载或自动重试。它选择本机已安装的 x64 SDK 工具与真实文件版本不晚于锁定 VC Runtime 的最新已装 x64 MSVC linker，记录工具摘要、路径及显式编译环境；没有兼容工具就失败，不安装新 toolset、不执行安装器、不改变锁、信任或验收阈值。规范 `package` 仍自行冻结并重新核定全部输入。

## 共同合同与材料

[`host-contract.json`](../packaging/windows/host-contract.json) 是原生宿主和清单的单一身份／路径／激活配置：Win11 x64、最低 build22000；本地包身份 `Weftext.Desktop`、Publisher `CN=Weftext`、AppId `Weftext`。Publisher 是本地签名 Subject 的要求，不是已取得证书或系统信任的声明。`persistentRelativeRoot` 为 `%LOCALAPPDATA%/Weftext-MVP` 保全范围，`coreRelativePath=core` 指向默认 Core，`hostStateRelativePath=host-state` 指向此 OS 用户的私有宿主元数据。两个目录必须独立，显式 Core 也不得等于、包含或被 host-state 包含；通知路由目录不成为第二个 Core。旧顶层 Core 原件不自动采用，需保留并显式导入／一次性转换，不保留双路径兼容。

`browserDataKnownFolder=LocalAppData` 和 `browserDataRelativeRoot=Weftext/Cache` 指定持久 WebView2 浏览器数据，配置的 UDF 路径与 profile 身份键派生保持不变。目录虽名为 Cache，UDF 不是可丢弃缓存：其中 IndexedDB 可能持有尚未被 Core 确认的唯一的本人输入副本。Core 与浏览器数据根必须保持物理隔离。

清单准确排除 `Weftext-MVP` 根（Core 和 host-state）以及此应用拥有的 `Weftext/Cache` 浏览器数据根。拒绝任意 KnownFolder、上层目录或路径穿越，不会整体关闭 AppData／注册表虚拟化。这些声明不是数据连续性的证明：实际安装、升级与卸载／重装验收须核原 Core profile／设备、保留的本机 origin 及浏览器输入。旧 MSIX 虚拟化 UDF 即使逻辑路径相同也可能具有不同物理映射；本变更不宣称已经迁移。

StartupTask 默认关闭，登录入口只传 `--login-startup`；通知 COM 使用固定 CLSID 和 `--toast-server`，激活只转发 opaque 原任务键并重新授权。CLI 的包内执行别名明确选择 console 子系统。清单资源包含同源图标和中英文 PRI；本机界面语言偏好不由包资源代替。

[`offline-materials.json`](../packaging/windows/offline-materials.json) 只接纳已核定的准确原件：类别、载荷架构、版本、官方来源／最终 URL、字节长度／SHA-256、实际微软签名者与原许可。安装器自身的 PE 架构不等于内部 payload 架构。校验持有同一只读文件句柄、禁止并发写／删除，采用 cache-only WinVerifyTrust，不声称检查了最新在线吊销状态；缓存链无法确认时拒绝完整候选。调用者不能靠 JSON 自报或把 Bootstrapper 改名为 Standalone。

当前锁已包含真实 VC 14.51.36247.0 与 WebView2 156.0.4314.42 x64 Evergreen Standalone 原安装材料。WebView2 的版本和架构来自原件中与离线清单摘要相符的签名载荷及实际 x64 Runtime，不能用外层更新器版本或 PE 架构代替。每次打包仍须核对准确原件、离线签名和全部材料锁；主机已装 Runtime、差分安装器、普通 Edge 或另一工作区的材料不能替代。原始许可与再分发条件保留，不据下载／签名通过推定公开分发资格。WebView2 的 `ExternalDependency` 只在 App Installer 路径生效，PowerShell 等安装会忽略，不能替代这里的离线资格。识别仍按现有独立 runtime／模型锁显式配置，包不冒称已包含它。

## 产物与实际验收

成功时在 `dist/` 新建候选目录，固定公共载荷分别生成 ZIP 和未签名 MSIX。原 source.zip、字体清单、原许可、实际参与构建的 Cargo／已安装 npm 依赖声明及 BUILD.json 一并保留；SDK Loader 原许可独立于 Rust crate 许可。两个容器逐文件核对完整字节和 MSIX blockmap，PACKAGE.json 记录各角色、源码、资源、工具和容器摘要。旧候选不覆盖，SDK 失败留下 INCOMPLETE.json 和原输出。

本地版本映射仅支持规范 `major.minor.patch-rc.N`（`0 ≤ N < 65535`）及正式 SemVer：RC 的第四段为 N，正式版为65535，下一 patch 高于前一正式版。拒绝其它预发布／build metadata、非规范前导零和溢出。这是本地 MSIX 策略，未声称符合 Store 的另一套发布编号政策。同包身份／版本的不同字节不能视为成功升级；实际升级验收使用更高的真实候选号。

生成只建立包结构和字节对应；签名／信任、最低系统、缺依赖离线安装、真实通知／StartupTask、带 CaptureDraft／pending／ledger 升级、升级失败恢复及卸载重装保全仍按 [WORKFLOW](WORKFLOW.zh-CN.md) 和 DSK09／DSK10 在同一最终候选实测。不能把 SDK 空白探针、单元测试或未运行项算作这些通过，也不能把当前 `0.7.0-rc.10` 字段当成 v0.7 完成。
