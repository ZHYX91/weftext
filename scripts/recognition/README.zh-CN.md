---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# 独立识别运行时

`install.py` 是 Windows x64 首轮识别后端的独立安装入口；[固定清单](../../resources/recognition/runtime-lock.json) 锁定每个下载及运行文件的 SHA-256。它创建本产品自己的 Python 3.12 虚拟环境，安装 pypdfium2 5.14.0（PDFium 156.0.8076.0）、Pillow 12.1.1、Tesseract 5.5.3.20260724 和 tessdata_best 4.1.0 的指定提交，不查找其他工作区或 PATH 中的同名后端。

在产品仓库根目录，用本机独立的 **64 位、非虚拟环境 CPython 3.12** 执行：

```powershell
& 'C:\Path\Python312\python.exe' scripts/recognition/install.py --runtime-dir 'D:\Weftext\.workspace\tools\recognition' --python 'C:\Path\Python312\python.exe'
```

路径必须明确且为绝对路径。新目录由此入口写入所有权标记；已有非空、无标记的目录会被拒绝。安装不删除或接管已有文件，冲突或摘要不符立即停止并保留现场。已完成目录只允许与原固定清单相同的核验；升级使用另一个明确的独立目录，避免运行中的任务与模型混用。

只有这个显式安装入口联网下载官方固定资产。Tesseract 官方 NSIS 安装器会写注册表，并可能处理已有安装，因此本入口**不执行 Tesseract 或 7-Zip 安装器**：固定官方 `7zr` 只解出 7-Zip 控制台与 DLL，再静态解包 Tesseract，按锁定的 PE 传递依赖闭包仅复制 `tesseract.exe` 及 33 个 DLL。它不修改系统／用户 PATH、注册表、文件关联或开始菜单。运行时启动使用绝对路径；子进程 PATH 只含 Windows 系统目录，不回退寻找全局后端。

已缓存完整固定资产时可离线安装；安装后可重新核验实际文件与启动探针：

```powershell
& 'C:\Path\Python312\python.exe' scripts/recognition/install.py --runtime-dir 'D:\Weftext\.workspace\tools\recognition' --python 'C:\Path\Python312\python.exe' --offline
& 'C:\Path\Python312\python.exe' scripts/recognition/install.py --runtime-dir 'D:\Weftext\.workspace\tools\recognition' --verify
```

`--offline` 缺资产即失败，不请求网络；Python wheel 也只从已核验缓存以 `--no-index --no-deps --require-hashes` 安装。运行任务使用已完成目录，不下载缺失模型。安装探针实际创建 PDF 页面并渲染，通过全部模型组合执行英数字样 OCR，检查 Tesseract 模型列表并记录实际版本；这些探针只证明后端能启动和加载模型，不证明中文准确率或产品导入验收通过。

产物 `installation.json` 的版本为 `1`，含运行时绝对路径、基础 Python、实际版本、清单摘要、每个 Tesseract 二进制与模型摘要、许可证索引和探针结果。固定目录为：

- `venv/Scripts/python.exe`：本产品独立 Python；`venv/Lib/site-packages` 的识别依赖为两个固定 wheel，另有虚拟环境自带的 pip。
- `tesseract/tesseract.exe`：静态解包的后端及固定 DLL；`tessdata` 只含指定的 `chi_sim`、`chi_tra`、`eng` 模型。
- `downloads`：官方原分发、缓存和解包中间件；`licenses`：原版权／许可证；`work`：启动探针产物。它们均为有所有者的工具安装，不属于自动 temp 清理。

支持的显式模型组合为 `chi_sim`、`chi_tra`、`eng`、`chi_sim+eng`、`chi_tra+eng`，均用 LSTM `--oem 1`。识别语言与输出文档语言分别选择，不自动翻译或简繁转换。首次范围的预算为输入 16 MiB、100 页、300 DPI、单页 4000 万像素、总输出 32 MiB、单页 OCR 60 秒；具体任务仍须按预算处理部分失败及未处理页。

安装目录保留下载时的完整官方分发、Tesseract AUTHORS／LICENSE、模型许可证、wheel 内全部 PDFium／Pillow 及其随附依赖许可证、7-Zip 许可证和精确 DLL 清单。该入口让用户从上游取得本机私用运行时；它不是可再分发许可。Tesseract 的官方安装包未附全部 DLL 的独立版权／对应源码材料，重新分发整套运行时前必须逐项补齐上游适用义务；本轮不声称已完成运行时再分发审核。其他系统、移动端或服务器执行端的安装验收仍按产品版本范围另行验证。

## 离线 worker 合同

用上述独立环境的 Python 执行 `worker.py --runtime <绝对安装目录>`，从 stdin 读一份 UTF-8 JSON，从 stdout 输出一份 JSON；[请求](../../resources/recognition/request.schema.json)与[结果](../../resources/recognition/result.schema.json)的版本均为 `1`。`inputPath` 只供已获权服务端选择准确原件，不能直接接受浏览器传来的本机路径。调用者须先固定原件摘要、页／区域与处理设置，并在交付结果或提交计划前复核原件、权限、规则与模型版本。产品桥已通过内嵌固定脚本接入 HTTP、Core Plan 与产品界面；worker 仍只读，中间结果不授予提交权限，持久化与交付资格由获权宿主及 Core 决定。

请求选择 1 起算且不重复的页；缺少 `regions` 表示整页。`rect` 是可见页左上角坐标的 `[左,上,右,下]`，PDF 用 pt，图片用 EXIF 朝向后的 px。显式区域不得相交。`mode` 为 `auto`、`text` 或 `ocr`；`rotation` 是仅用于识别的顺时针 0／90／180／270 度；`segmentation` 的 `auto`／`sparse`／`block`／`line` 分别固定 Tesseract PSM 3／11／6／7，不接受任意配置文件或命令。

区域的 `ocrScale` 可显式选 `1`（默认）、`0.75` 或 `0.5`，仅对派生识别图做 LANCZOS 缩小；它不更改原图、页坐标或渲染 DPI 上限。实际缩放及识别栅格尺寸保在 `ocrGeometry`，`ocrToVisible` 同时补偿整数尺寸取整、裁切、旋转与缩放。不同字号／版式可经用户核对后重试；模型分数和可选缩放均不替代固定质量门。

PNG 透明度先在派生页中合成到白色背景，再预览或识别，保原始 alpha 与文件摘要不变；避免丢弃 alpha 后把透明黑色背景误变为黑底。

`operation` 默认为 `recognize`。`inspect` 请求省略 `pages`，返回整份有界原件的 `pageCount` 与各页尺寸、朝向、裁切及坐标单位，`regions` 为空，不渲染或识别。`preview` 沿用页选择，返回所选完整页的 `preview: {mediaType: "image/png", dataBase64, width, height}` 及相同几何；区域仍以完整预览页为坐标基础，不裁掉预览上下文。这两种操作不加载或核验 OCR 模型，仍核验 PDF／图片运行时与原件；同用页数、像素、内存、时间和总输出上限。区域 ID 的 `__weftext_auto_` 前缀保留给自动派生区域；相邻区域用半开边界分配字符，避免边界中心字重复。

结果不含原件路径或后端诊断正文。它保原件 SHA、选择、准确工具／模型摘要、冻结实现摘要、逐页／区域状态、原始文字、词坐标及识别分数。识别分数不是准确率。`renderToPdf` 将识别渲染像素映射到原始 PDF 用户坐标；图片的 `visibleToRaw` 保 EXIF 映射；`ocrToVisible` 把裁切／旋转后的识别像素映回可见页。六元仿射数组 `[a,b,c,d,e,f]` 表示 `(a*x+c*y+e,b*x+d*y+f)`。

自动路由优先可用文本层，只对与已有文字完全不重叠的图像矩形识别；文本区域的 `excludedRects` 明确扣除这些 OCR 区域。有图像／文字叠层而无法证明缺区时返回 `partial`，保文本并要求明确选区，不重复写入两路文字。空 OCR 不等于空白页；非白图像却无文字属于部分结果。阅读顺序、页眉页脚和规则网格仅是待确认候选；原始块不会因建议被删除。基础完整直线网格在派生图上去除已探测规则线后执行一次 OCR，保原件及几何；复杂合并／跨页表格、手写、公式和复杂版式没有结构恢复验收。

文本层的同字同坐标重复覆盖仅输出一次；不同位置的重复词仍保留。完整直线网格也可直接使用文本层字符恢复单元格，不强制 OCR。无网格候选不代表复杂表格已恢复；原件与原始文字供人工核对。

PDFium 标记为自动生成的横向空白不属于原 PDF 的 Unicode 文字，提取时不写入正文；明确源空白和生成的换行边界仍保留。文本层跨较大同排几何间隙的片段分为独立块，以形成可核对的简单双栏顺序，不能把左右栏拼成一条横行。

每页在独立进程运行 PDFium，Windows Job 限制识别及其子进程为 1 GiB、60 秒，关闭或取消会结束 OCR 后代；输出及诊断文件也有上限。作业前后核验固定 wheel 文件、Tesseract 及 DLL、选中模型；作业不联网、不搜索 PATH 后端。失败／超限保准确失败页，与成功页共同形成 `partial`；只处理选中范围不等于整份原件完整。

Windows 页面进程先挂起启动，归入 Job 后才恢复；子进程只收到明确列出的运行环境变量。OCR 派生图片／TSV 位于该次作业私有目录，正常退出及页面超限会由父进程清理；OS 占用导致无法清理时保留自有临时文件，不让清理错误覆盖准确识别／预算结果。强制终止整个 worker 也可留下该次私有目录，按既有所有权规则处理，不能把强杀当作清理完成。

宿主内部可调用 `execute(request, runtime, progress=None)`，通过可选回调取得每个已计入页面后的隔离快照；完整检查结束时 `inspect` 回调一次。快照保完整原选择、已完成页及已知原页数；尚有选中页未计入时状态为 `partial`。消费者修改快照不会改工作中结果，回调异常向宿主传播。模块不接受客户端进度路径，也不自行维护第二份提交账本。进度是最终运行时／模型复核之前的临时恢复证据，即使最后快照状态完整，也不能替代正常返回及宿主交付前复核；宿主超时／重启须保已完成页、明确未处理范围并重新核验来源资格。

产品宿主恢复检查点时标记 `workerFinalValidation: false`，作业外层状态为 `interrupted`；已完成页可只读核对，缺页和中断损失明确列出。这类结果不能通过 `acceptPartial` 或接受损失来导入，须使用显式新 jobId 重跑后重新核对；原 jobId 查询／重试不会自动续跑或重跑。正常完成最终校验的部分结果仍按原有独立部分接受及 Core 确认合同处理。

确认资格自作业创建起有效 15 分钟，与整次执行的 15 分钟预算分别检查。资格到期后，当前授权、原件、执行实现和运行时仍一致时可只读查询状态及已有页面文字，响应标记 `confirmationExpired: true`；到期不再允许原 jobId 启动重试、页面预览、下载或创建新的确认计划，提交也继续核验原计划的有效期。只读恢复不会绕过撤权、原件替换或模型变化；进一步处理须显式创建新作业。

实际后端检查入口如下；样例由脚本生成，明确为 TEST，使用本机字体生成图片而不复制字体文件：

```powershell
& '<安装目录>\venv\Scripts\python.exe' scripts/recognition/test_worker.py --runtime '<安装目录>' --evidence '<自有证据目录>'
& '<安装目录>\venv\Scripts\python.exe' -m unittest discover -s scripts/recognition -p test_worker_boundaries.py -v
& '<安装目录>\venv\Scripts\python.exe' scripts/recognition/test_worker_progress.py --runtime '<安装目录>' --evidence '<自有进度证据目录>'
```

这些是运行时组件测试，不能替代 IM01–09、LC09、真实客户端、权限／同步／恢复、专用字段或 Office 闭环验收。
