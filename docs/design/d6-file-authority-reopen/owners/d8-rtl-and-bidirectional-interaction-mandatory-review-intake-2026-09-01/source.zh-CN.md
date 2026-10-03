---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：7db65eaf-0892-4e64-b62f-42b7988fdc99。

# D8 RTL 与双向交互必审输入 — 2026-09-01 来源处置

候选状态：D8-FA-r01 PL-IR-01 修复消费者；完整 owner 后像，未独立接受、未激活、未实施。固定来源 S 与原 mandatory RTL intake 只保留原证据范围。本后像消费 D3/D6 稳定生产地址/current Observation 作者修复；该 exact 修复仍待独立复核，不自行关闭 P1。D3 conflict/当前 D7 effects 门保持独立。

## 目的与权威边界

本文件保留固定 S 中由总控持有的完整 RTL 必审义务及来源证据边界。原记录用于把聊天中的要求交接到独立 D8 任务；其 status=mandatory-review-input-not-frozen-decision、owner=D8-editor-and-cross-platform-interaction、d8_started=no、d4_changed=no、repos_weftext_modified=no、brand_modified=no 只是 2026-09-01 的来源状态，不是当前任务状态或执行指令。本次 D8 后像没有修改实现、品牌或上游快照，也不以该来源自行激活 RTL 模型或阿拉伯本地化。

原任务名为 Architecture Convergence and Clean-Slate Refactor - D8，相关外部控制记录未随输入发布。当前产品选择见本批主文、Direction、Interfaces 与 Acceptance Matrix；须经真实独立审查和总控裁决。本文件保存输入与处置，不能独立宣布接受。

## 实现观察与证据限度

2026-09-01 的定向检查只发现局部 RTL 内容处理，不足以声称完整 RTL UI。2026-10-02 作者对当前主工作树作只读定向刷新，未运行 UI；实际查看到：

- Desktop、Server WebUI 与原型根语言仍是 `zh-CN`，分别在 `apps/desktop/index.html:2`、`crates/weftext-server/webui/index.html:2`、`prototypes/webui/app/layout.tsx:29`。
- 用户名称等局部使用 `dir="auto"`：`crates/weftext-server/webui/app.js:269,334,1226` 和 `prototypes/webui/app/page.tsx:3864-3898`，此外部分菜单、恢复与事务预览目标也使用它。它只证明局部内容方向，不证明全局 shell。
- 原检查没有找到通过全局 `document.dir`、根 `dir="rtl"|"auto"`、`direction`、`writing-mode` 或 `unicode-bidi` 建立并传播 shell/editor RTL 状态的证据。当前定向源码检索也未建立完整机制的证明；排序变量 direction 与文本方向不是同一语义，有限检索不能证明仓库绝无其他实现。
- 原型仍有物理方向样式 `border-right`、`border-left`、`text-align:left` 和不对称圆角，代表位置仍包括 `prototypes/webui/app/globals.css:39,49,343,347,465,498,547,644,761`。局部 `text-align:start` 不能证明完整镜像。
- 固定 S 记录有阿拉伯引文 locale 的产品证据；引文呈现不能证明阿拉伯 UI 翻译或完整 RTL 编辑。数据 fixture 也不能替代 caret、IME、焦点或镜像的端到端测试。
- 原检查未在所查 UI 测试表面找到明确 RTL 布局、焦点、光标/选区、混合文本与交互端到端用例。本次限定的测试文件定向检索也没有取得这类测试证据；没有运行真实平台，不扩大为全仓库不存在的断言。

来源还记录既有公开规范要求：`docs/architecture/05-shared-navigation-information-architecture.md:29` 要求共享导航支持 RTL/混合文本；`docs/specifications/09-testing-release.md:60,64,91` 将 RTL 保留在编辑、导航、选区、IME 与命令表面的跨表面验收中。这些行号是来源定位，不是本次重验其当前行号的声明。

因此，局部支持与完整交互验收之间仍有待实施的差距。D8 必须明确处理，不能继承“当前 UI 已完整支持”的假定；实际产品支持仍需逐平台证据。

## 九项必答问题及当前处置

1. **能力分离：** 分别验收 Unicode/阿拉伯内容保存、双向文本交互、RTL 应用布局和阿拉伯 UI 翻译，任何一项不推导其他项。见 Direction §1/7。
2. **方向权威：** 对 shell、Document/editor、block、inline、名称、table 和 Query/View，明确 ltr/rtl/auto 的选择、scope、继承、override、持久化和可访问 API；方向不形成第二内容权威。见 Direction §2。
3. **混合内容：** 覆盖阿拉伯语、希伯来语与中日韩及拉丁文字，欧洲数字与阿拉伯数字、标点、网址、路径、邮件地址、节点与资源引用、引文、公式、代码、行内标记、表格和诊断。见 Direction §3。
4. **编辑交互：** 逐项规定视觉/逻辑光标、selection、Home/End/箭头、Backspace/Delete、word/grapheme、拖选、复制粘贴、Undo/Redo、composition/IME、工具栏、菜单、slash/palette、Annotation、reanchor 与 stale。见 Direction §4、主文 §5–9 与 Interfaces §3。
5. **布局镜像：** 明确 navigation、pane、menu/popover、table/board、icon/chevron、边框间距、scroll/focus 的镜像边界及不可镜像语义图标/数据方向；物理 CSS 不暗定语义。见 Direction §5。
6. **无障碍：** 要求键盘完整操作、稳定 role/name/state、焦点与阅读序、读屏、zoom/reflow、高对比、减弱动画；视觉与逻辑次序不同仍保持意义。见 Direction §6。
7. **跨表面一致：** Desktop、Server WebUI 与 Mobile 使用明确能力矩阵，设备只改变操作方式或真实可用性，不改变 Core plan/error/Ref/commit/内容权威。见 Direction §7。
8. **本地化边界：** 独立决定应用界面何时翻译为阿拉伯语或其他 RTL 语言；引文 locale、名称 dir=auto、成功存储都不满足本地化验收。本代不声称交付阿拉伯 UI，见 Direction §1/2/7。
9. **性能规模：** 大文档、集合、长名称、虚拟导航及快速方向变化保持确定性，不使用 locale 排序或第二 parser。见 Direction §6、Impact §2。

## 必须保留的验收场景

完整候选与独立终审包必须包含以下全部正反例：

- Arabic-only/Hebrew-only 的文档、名称、属性、单元格、Query、Annotation 和搜索结果，经过编辑、保存、重载、离线 Draft、冲突与恢复；Query/搜索仍只读，编辑/保存只指其真正作者输入或显式 Action。
- 混合 RTL/LTR 段落和 UI label，含数字、标点、URL、Ref、citation、code、formula、无效与 protected source。
- bidi 边界上的 caret、selection、删除、composition、复制粘贴、Undo/Redo、Annotation targeting 与 reanchor。
- 镜像 shell/navigation/menu/table/board，同时明确不镜像语义内容。
- Desktop、Server WebUI、Mobile 的键盘和读屏焦点/阅读序。
- 如实报告未支持的本地化与平台行为，不从局部内容支持推导。
- UI 方向变化前后 Core action/plan/commit 语义保持相同的回归证明。

本批矩阵原 160 项完整保留；新观察/保存保护/恢复与无 caret 队列义务额外加入，不替换以上场景。它们是验收义务，不是产品测试结果。

## 交接与独立接受门

后续审查必须以本文件为必审输入，刷新会漂移的实现观察，区分证据与产品权威，并把全部场景带入独立完整审查及总控裁决。固定来源提到 DeepSeek 与 GPT-5.6 Sol + Pro 5/5，是原流程出处，不是当前启动聊天、模型或代理的命令；当前执行服从人类已授权安排。没有 RTL/bidi 完整处置，或仅用引文 locale/dir=auto spot check 替代，D8 不可接受。
