---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Typst 0.15.1 HTML 样式有限原型

这是明确 Html 流式阅读 / Paged 打印路线的有限技术实验，不是产品实现、完整 E1 关闭或 RQ03/RQ5.3 验收。显示出现项由同次最终 `HtmlDocument` children 选择，不能直接采用稳定轮 realization trace。完整支持表与消费合同见[证据](evidence.zh-CN.md)；精确版本、patch/lock pins 见 [version.json](version.json)，实际有限结果见 [analysis.json](executed-summary/analysis.json) 和 [results.json](executed-summary/results.json)。

## 复现

需要 Git、Rust/Cargo 和 Python 标准库。下载[固定 v0.15.1 源码](https://github.com/typst/typst/archive/refs/tags/v0.15.1.zip)及[官方 release CLI](https://github.com/typst/typst/releases/tag/v0.15.1)，按 `version.json` 校验源码 archive SHA256。使用干净的完整源码目录；不要对已打补丁的源码再次应用。

从本原型目录运行下面的 PowerShell 命令，先将三个示例路径改成实际源码根、独立 target 目录和官方 CLI。两个 `git apply` 均在源码根执行，patch 参数为原型目录的绝对路径；附带 `Cargo.lock` 覆盖源码根的 lock。runner 与 analyzer 位于本目录、与 `fixtures/` 相邻；它们根据脚本位置找 fixtures，并在本目录生成 `results/`，包含实际命令、退出码、stdout/stderr、最终原生树以及 on/off/release 输出。

```powershell
$probeDir = (Get-Location).Path
$sourceDir = (Resolve-Path 'C:\typst-0.15.1').Path
$targetDir = 'C:\typst-html-probe-target'
$officialCli = 'C:\typst-release\typst.exe'

Push-Location -LiteralPath $sourceDir
try {
  git apply --check (Join-Path $probeDir '01-inherited-observer.patch')
  git apply (Join-Path $probeDir '01-inherited-observer.patch')
  git apply --check (Join-Path $probeDir '02-html-style-adapter.patch')
  git apply (Join-Path $probeDir '02-html-style-adapter.patch')
  Copy-Item -LiteralPath (Join-Path $probeDir 'Cargo.lock') -Destination (Join-Path $sourceDir 'Cargo.lock')
} finally {
  Pop-Location
}

cargo build --manifest-path (Join-Path $sourceDir 'Cargo.toml') --target-dir $targetDir -p typst-cli --locked
python (Join-Path $probeDir 'run_style.py') --binary (Join-Path $targetDir 'debug\typst.exe') --official $officialCli
python (Join-Path $probeDir 'analyze_style.py')
```

Windows 上源码与 target 使用短路径，避免 MSVC 长路径 response-file 错误；其他平台调整 shell、路径及可执行文件名。本实验使用 Cargo/Rust 1.98.0。首次构建允许下载固定 lock 中的依赖；依赖已经缓存时才可额外加 `--offline`。继承 observer patch 隔离无关的测试资源依赖并记录 World 请求，它不是显示消费选择算法。patch 已在干净官方源码上通过 check/apply，6 个改动文件与实际执行源码相等；同一 16 例的 runner/analyzer 已从独立 fixtures 路径再次执行通过；本次收窄 oracle 后，又读取既有最终树分析通过。

## 实际范围

16 例关闭适配的 HTML 与官方 release 字节相同；开启适配的单/四线程 HTML 和最终原生树相同；Paged off/on/release PDF 字节相同。assert analyzer 验证有效字体/字号/颜色、literal marker 覆盖与深度、简单 cell stroke/align、正文结构、measure 丢弃、实际复用、table link、heading 删除与 native introspection 分域及局部原生 frame。正文结构对照只忽略有明确 producer 身份的新增 marker 和样式 carrier。

World source/file/font 调用全序列相同 15/16：unsupported marker 警告例的同 source 读取从 2 次变 3 次，font 调用均为 11 次。额外读取与诊断行为一致，但未作调用栈追踪，不能声称所有依赖均相同。

已证明的是最终原生 CSS 的有限值：字体候选、字号、纯色、有效 strong/emph、literal Content marker、简单合成 cell stroke/fill/align。孤立 SpaceElem、SmartQuote、Pre/raw 路径保持官方转换并继承容器 CSS，任意局部字体尚未完整携带。五档是五种源码显示/编辑视图，不是字号 presets 或新编译 target；本轮未验证这些 UI。

浏览器字体加载、视觉等值、viewport、AT、完整表格边界冲突/gutter/line规则和任意自由样式尚未证明。原生 frame 保留复杂 dash/function-marker 视觉输出，但完整 frame 阅读/复制语义不可用；原型未实现自动 fallback 选择。

环境开关在每个独立进程中固定，不是 memoization key。产品配置应纳入 library/routines/profile 身份或固定 worker 配置，不能在共享缓存进程中随意切换本研究开关。

本原型的 `data-probe-text` / `data-probe-marker` 是受控 fixtures 的测试识别标记，不是生产可信来源或 writer 资格。作者可以在 raw HTML 中伪造同名属性；产品需要不可由作者伪造的内部 provenance，不可据这些属性判断来源。修正后的 analyzer 仅折叠明确标记的 text-carrier span，并保留其它 span 结构及投影中的非样式属性；样式另由 CSS值检查。它还断言 stock fixtures 不含这些测试属性。此项是 oracle 修复：读取既有最终树重跑分析，未新增编译或 fixtures。

## 消费合同

同次最终 body child paths 是 flow、Query display-body、默认复制和 RenderedLink 的共同消费根。native heading level/counter 从同次文档 introspector 读取，不能由 DOM h2 推为业务 level2。复制按真实 br/role 边界做显式结构序列化，不能拼 TextItem。frame 的局部 target 是 Paged，出现项仍归 outer Html 编译；Paged 导出可走不同分支，不跨编译拼片段/counter/source。Span 不证明 writer，generated 或来源歧义不给 writer。

本包仅包含 patches、lock、fixtures、runner/analyzer、精简机器结果、版本和双语文档，不附官方源码 tree、target 或下载 cache。完整消费边界和未证明事项以[证据](evidence.zh-CN.md)为准。
