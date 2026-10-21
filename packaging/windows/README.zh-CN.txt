Weftext Windows 本地候选

weftext.exe 是桌面入口，weftext-cli.exe 是保留真实终端输入、输出和退出码的命令行入口。两者使用同一私有状态和 Core，重复启动不能另建写入者。Server 和内部编译 worker 不进入桌面初始化。

启动需要与本候选匹配的 Microsoft WebView2 Runtime 和 VC Runtime。prerequisites/ 保存本候选核定的官方离线原安装材料及原许可；程序与打包脚本都不会自动运行它们。缺组件会明确报告。Microsoft 原安装材料的使用和分发以各自原许可为准；本候选不附带 Office。识别运行时仍需按 source.zip 内 scripts/recognition/README.md 单独明确配置，不把本机已有运行时当成包内依赖。

用户笔记库保持在所选目录。默认 Core 在 %LOCALAPPDATA%/Weftext-MVP/core；此 OS 用户的私有宿主元数据在同一保全范围下独立的 host-state。显式 Core 与 host-state 不能相同或互为祖先／子孙。旧 Weftext-MVP 顶层 Core 不自动采用；原件保留，需显式导入或一次性转换。cache 和 WebView2 用户数据仍在范围外的 %LOCALAPPDATA%/Weftext/Cache。清缓存不能替代保留笔记、恢复草稿、原操作记录和设备状态。MSIX 声明只排除准确 Weftext-MVP 保全范围的虚拟化；实际安装、升级、卸载保全仍以同一候选的真实验收为准。

本目录由生成器创建，不代表签名可信、安装成功或整版验收完成。未签名 MSIX 不是可直接安装的受信发布包，不应通过关闭系统保护来安装。BUILD.json 与外层 PACKAGE.json 记录准确源码、两个程序和材料；候选版本或文件名相同不能证明升级成功。不要覆盖旧候选或用另一次构建替换已验字节。

详细使用、构建及当前支持边界见 source.zip 内 docs/MVP.zh-CN.md、docs/PACKAGING.zh-CN.md 和 docs/WORKFLOW.zh-CN.md。
