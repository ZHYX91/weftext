---
source_language: zh-CN
translation_of: null
translation_status: source
---

# Windows 候选打包

产品唯一入口仍是 `scripts/mvp.py`。Windows 桌面 `weftext.exe` 和终端 `weftext-cli.exe` 必须来自同次 Cargo 构建及同一份源码；文件改名不能代替真实 PE 子系统。两程序都承载原编译 worker 协议，Server／CLI／worker 不由 GUI wrapper 再启动第二套服务。

`build`／`check` 使用当前源码；`package` 先核完整离线材料与 SDK，再把公开产品源码冻结到实际 Cargo target 下本次独有的输入目录，后续构建和同一套检查都在该副本运行。源码归档来自这份固定字节，嵌入前端另记准确清单。不会在构建后重新扫描正在编辑的工作树来冒充同源产物。源码、前端、两个程序、依赖原许可或材料资格变化会拒绝打包；失败输出保留，不回退旧 target 或旧单程序包。

首次打包要求实际产品 Git 检出及 Git 工具，按 `git ls-files -c -o --exclude-standard -z` 选择已跟踪和未忽略的公开文件，读取当前工作树字节；未提交修改、新增和删除均反映到本次快照，忽略的本地材料不会被扫描读取。源码归档不包含 `.git`，解压后仍可独立运行 `build`／`check`；任意无 Git 目录重新打包不属于支持入口。

Windows 构建完成后，入口在实际 target 下新建同批产物目录，把两个最终程序及 `artifact-catalog.json` 固定在一起，后续检查和打包使用该角色映射。使用入口打印的这两个准确路径；原 mutable Cargo debug／release 输出不是本次验收角色映射。release 固定副本和最终候选属于交付产物，不纳入自动 temp 清理。清单包含 `schemaVersion`、`architecture`、`sourceSha256`、`hostContractSha256`、`buildId` 及 `roles`；desktop／cli 各记录固定文件名、准确 SHA-256／字节长度和 PE 子系统。`buildId` 为源码归档摘要、合同摘要、desktop 摘要、cli 摘要四个小写十六进制 ASCII 字符串依次以 LF 连接后的 SHA-256，末尾无 LF。宿主从系统返回的真实进程映像核对本批角色，不能信任 peer 自报身份或目录名。跨构建升级另行协调，不因目录相同而自动接管。

```text
python scripts/mvp.py build
python scripts/mvp.py check
python scripts/mvp.py package --release --windows-sdk "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64" --offline-materials "D:\Weftext-Materials" --msvc-linker "C:\Path\To\MSVC\bin\Hostx64\x64\link.exe"
```

示例材料／linker 路径需替换为本次准确输入。打包显式使用该实际 x64 MSVC linker，并要求离线 VC 版本不早于其真实文件版本。构建及依赖获取仍需已有 Node／Rust 工具链；生成阶段不下载、运行安装器、签名、安装或改信任／注册表。没有跳过角色、许可、锁或离线签名检查的开关。

## 共同合同与材料

[`host-contract.json`](../packaging/windows/host-contract.json) 是原生宿主和清单的单一身份／路径／激活配置：Win11 x64、最低 build22000；本地包身份 `Weftext.Desktop`、Publisher `CN=Weftext`、AppId `Weftext`。Publisher 是本地签名 Subject 的要求，不是已取得证书或系统信任的声明。默认持久根保持 `%LOCALAPPDATA%/Weftext-MVP`，cache／WebView UDF 位于另一个 `Weftext/Cache` 根。清单只对准确持久子目录关闭写虚拟化，拒绝任意 KnownFolder、上层目录或路径穿越；不会整体关闭 AppData／注册表虚拟化。

StartupTask 默认关闭，登录入口只传 `--login-startup`；通知 COM 使用固定 CLSID 和 `--toast-server`，激活只转发 opaque 原任务键并重新授权。CLI 的包内执行别名明确选择 console 子系统。清单资源包含同源图标和中英文 PRI；本机界面语言偏好不由包资源代替。

[`offline-materials.json`](../packaging/windows/offline-materials.json) 只接纳已核定的准确原件：类别、载荷架构、版本、官方来源／最终 URL、字节长度／SHA-256、实际微软签名者与原许可。安装器自身的 PE 架构不等于内部 payload 架构。校验持有同一只读文件句柄、禁止并发写／删除，采用 cache-only WinVerifyTrust，不声称检查了最新在线吊销状态；缓存链无法确认时拒绝完整候选。调用者不能靠 JSON 自报或把 Bootstrapper 改名为 Standalone。

当前锁已包含真实 VC 14.51.36247.0，尚无取得资格的 WebView2 x64 Standalone 原安装材料；完整离线 Windows 候选因此明确拒绝生成。主机已装 Runtime、差分安装器、普通 Edge 或另一工作区的材料都不能补空。原始许可与再分发条件保留，不据下载／签名通过推定公开分发资格。WebView2 的 `ExternalDependency` 只在 App Installer 路径生效，PowerShell 等安装会忽略，不能替代这里的离线资格。识别仍按现有独立 runtime／模型锁显式配置，包不冒称已包含它。

## 产物与实际验收

成功时在 `dist/` 新建候选目录，固定公共载荷分别生成 ZIP 和未签名 MSIX。原 source.zip、字体清单、原许可、实际参与构建的 Cargo／已安装 npm 依赖声明及 BUILD.json 一并保留；SDK Loader 原许可独立于 Rust crate 许可。两个容器逐文件核对完整字节和 MSIX blockmap，PACKAGE.json 记录各角色、源码、资源、工具和容器摘要。旧候选不覆盖，SDK 失败留下 INCOMPLETE.json 和原输出。

本地版本映射仅支持规范 `major.minor.patch-rc.N`（`0 ≤ N < 65535`）及正式 SemVer：RC 的第四段为 N，正式版为65535，下一 patch 高于前一正式版。拒绝其它预发布／build metadata、非规范前导零和溢出。这是本地 MSIX 策略，未声称符合 Store 的另一套发布编号政策。同包身份／版本的不同字节不能视为成功升级；实际升级验收使用更高的真实候选号。

生成只建立包结构和字节对应；签名／信任、最低系统、缺依赖离线安装、真实通知／StartupTask、带 CaptureDraft／pending／ledger 升级、升级失败恢复及卸载重装保全仍按 [WORKFLOW](WORKFLOW.zh-CN.md) 和 DSK09／DSK10 在同一最终候选实测。不能把 SDK 空白探针、单元测试或未运行项算作这些通过，也不能把当前 `0.6.3-rc.4` 字段当成 v0.7 完成。
