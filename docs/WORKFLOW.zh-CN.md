---
source_language: zh-CN
translation_status: source
---

[English](WORKFLOW.md)

# 开发、验收与发布

当前开发树已整合 v0.7 的既有开发切片：GUI／CLI 双入口、碎片与提醒消费者、界面语言及平台宿主基础，并统一保存／原操作交付和 Draft 基线恢复。合并开发切片的验证范围按 WORKFLOW 记录，不代表整版 v0.7 完成。Windows 原生窗口与安装交付、完整 Linux／Android／Apple 产品、真实 IME／辅助技术及性能／识别平台矩阵仍按 ROADMAP 独立验收；0.6.3 原有未验项保留。

## 当前阶段

固定旧基线已接受；本轮有界修订及实测边界见 [REVIEW](design/REVIEW.zh-CN.md) §8。当前 **0.6.0 Windows／NTFS 既定范围已完成**，本地、[受控 Server / WebUI](SERVER.zh-CN.md) 与 v0.6 同步在同一最终优化程序通过完整 HTTP、受影响浏览器及真实 NAS HTTPS 验收；尚未正式发布。使用与限制见 [MVP](MVP.zh-CN.md)，后续范围见[路线图](ROADMAP.zh-CN.md)。真实 OS IME、完整辅助技术和断电验收仍未完成；v0.7 多端及整体验收与下述隔离执行路径属于后续工作，不是已实现的产品能力。旧原型依赖问题留在归档，不使新的纯设计检查依赖旧 UI 或 npm 审计。

## 日常检查

既有 `0.6.2` 已完成 初始 Windows 本地／自托管资料导入与识别范围，提供经完整检查、实际浏览器及真实 Word 链路核验的本地交付候选。0.6.1 本轮本地交付已保留；最终候选真实 OS IME 与原撤销异常仍有原证据限制，不能因后续版本而改写。新增识别须在同一优化产物验证原件、离线模型、质量、部分范围、字段映射、Core 操作及权限／同步／撤权。模拟输入、参照软件或组件通过不代替实际产品链。具体支持与未完成项见 MVP。

0.6.3 的导航／普通写作、属性／多 Query 组织与普通根笔记预设切片沿用同一产品入口与唯一候选记录。实际客户端必须验证返回草稿／选区／Undo、取消与过期地图、权限撤销清除；最近访问／导航轨迹和保留编辑器是页面内个人状态，不成为作者内容或新权限。原 0.6.1 IME／撤销证据限制继续保留。

此前 `0.6.3-rc.4` 候选保留前三阶段能力，新增多级标签的 Core／Query／独立授权／同步及 Native 消费者、二维关系图谱、只读标题结构、作者同级顺序和按需媒体缩略图。整版验收仍在进行；各旧候选及新增定向检查的通过记录不自动升级为本次最终候选通过，具体支持与原限制见 MVP。默认在本地分支开发、运行产品检查并本地合并；远端推送、PR 和云端 CI 按用户要求使用，不作为本地交付前置。使用 PR 时核对实际提交的 CI 状态；发布与重写远端历史仍须有用户授权。沿同一候选记录汇总 TAG01–10／UX01–12、真实客户端、受影响 Office／权限／同步／恢复和产物对应，不另建完成事实库。最终候选真实 IME 与 K 候选原撤销异常仍待人工复验；固定参照插件的部分旧实测不替代整版同任务对照。

属性／Query 切片验证预设取消／原操作恢复／不重置、多个出现项独立状态、精确重排／移除及单步撤销、typed 错误输入保留、共享调用 current／pinned、直接字段独立写权与撤权，以及普通根笔记同步双程序往返。`scripts/test_query_organization.py` 接入原完整检查入口；浏览器脚本 `scripts/test_query_organization_browser.mjs` 显式接收同一候选程序和自有证据目录。源码冻结后复用同一优化程序完成 HTTP、实际浏览器及打包核对。

纯文档改动运行 `python scripts/check.py`。涉及 schema、类型/时间算法或模型时运行 `python scripts/check.py --models`。实现切片增加自己的格式、静态分析和必要测试；跨平台、文件故障、权限、编辑和 Office 输出按真实影响选场景。失败记录实际原因；不通过删断言或改期望掩盖问题。

无损表格操作和托管个人关系规则的受控浏览器组件回归分别为 `node scripts/test_table_actions_components_browser.mjs NEW_ABSOLUTE_OUTPUT`、`node scripts/test_hosted_relation_preferences_browser.mjs NEW_ABSOLUTE_OUTPUT`。它们运行当前前端和受控端口，不替代真实 Core／宿主验收；节点连续排序及托管规则刷新持久化另由既有 `test_workspace_layout_browser.mjs`、`test_hosted_knowledge_links_browser.mjs` 对同批合格 CLI 的嵌入界面验证。CLI 必须取同批 catalog／构建回执旁的角色产物，不以 target 下的中间程序或 GUI 角色替代。

`node scripts/test_rich_property_forms_browser.mjs NEW_ABSOLUTE_OUTPUT` 加载实际集中内置 schema 和共用值表单，经受控源码回调及真实 SourceEditor Undo 核重复联系方式用途／备注／顺序、任职组织／职务／精度同项关联、同名贡献者引用／角色／顺序、语言切换、单值开放枚举文字及自定义 schema 负例；不执行 Core 保存／恢复、权限、引用生成、原生窗口或真实 Office。`weftext-data` 的 `rich_workflows` 测试另使用已安装 schema、原生补丁 writer 和实际 CEL；Knowledge、Export 的 `rich_workflow` 过滤用例分别覆盖独立对称断言作者和文献导入／角色顺序往返。当前候选的真实表单保存／重开／原操作恢复、非对称关系写权及完整引用／Office 输出任务仍须沿原候选记录实际运行。

`node scripts/test_open_enum_consumers_browser.mjs NEW_ABSOLUTE_OUTPUT [library/publication|contacts/names]` 经真实 CaptureFields／Controller、DataPanel 和 HostedFields 核两处内置开放枚举。重挂后 Capture 字段提交门和当前候选须一致；DataPanel 刷新保当前文字，明确 Apply 后才进入共享 SourceEditor Undo。Hosted 重读／写入保当前文字，并以真实 PersonalControlInput registry、干净正文及无其他 pending 执行未改的 hosted `beforeunload` handler。这是使用受控持久化／传输的独立组件夹具，不等于整个 Hosted 页面、Core 保存／恢复或原生宿主通过。开放枚举只保一份可见文字，不维护隐藏 custom 模式或第二份可恢复值。

批量操作的受控组件回归为 `test_node_selection_browser.mjs`、`test_property_value_drag_browser.mjs`、`test_table_groups_components_browser.mjs`，均用 `node scripts/脚本名 NEW_ABSOLUTE_OUTPUT`。它们分别验证已显示选择范围、真实指针拖排及共享编辑器撤销、表格组命令与手势退休；不授予真实 Core 或原生宿主通过。`node scripts/test_table_groups_browser.mjs ABSOLUTE_CLI NEW_ABSOLUTE_OUTPUT [--hosted]` 另对同批嵌入前端和真实 Core 验成组插入／删除／移动、单次变更、准确范围及 Undo／Redo，`--dev` 只表明当前开发前端。主界面的范围选择和整批稳定链接复制也进入既有 `test_workspace_layout_browser.mjs`；所有检查保留原件与各自 owner 正常停机回执，复用同一候选记录。

本批新增的预设、跨字段与资源组件脚本包括 `test_value_presets_browser.mjs`、`test_field_transfer_components_browser.mjs`、`test_data_panel_workflows_browser.mjs` 和 `test_resource_import_batch_browser.mjs`，均以 `node scripts/脚本名 NEW_ABSOLUTE_OUTPUT` 运行当前前端及受控传输。它们分别核个人配置／原始输入保护、准确 occurrence 与坏回包、真实 DataPanel 独立对话框／staged→显式 Apply／共享 SourceEditor Undo，以及资源原操作恢复与独立正文追加；受控操作账本和 DTO 不能冒充真实 Core 文件提交。`test_hosted_resource_batch_components_browser.mjs` 另通过新的绝对 `WEFTEXT_BROWSER_REPORT` 目录及明确的 `WEFTEXT_BROWSER_EXECUTABLE`／`WEFTEXT_PLAYWRIGHT_MODULE` 运行实际 Hosted 导入控件，核批次、来源退休与个人 locator，不核真实 ACL／HTTP／导入效果。对应纯 TS 与 Rust Data／App／Core 用例仍从原产品检查入口执行。保留各次失败、准确源码图、实际退出码和自有进程正常关闭结果；这些定向结果不授予全 v0.7、原生窗口或真实系统 IME 通过。

作者输入／Draft ACK 分离沿这些原入口回归：纯 TS 核九成员身份、同输入连续 ACK、同字节新输入与 Undo／Redo、换 session／对象、撤权、原设置请求及关闭障碍；完整前端测试继续核原保存／未知操作。DataPanel 浏览器组件同时核等待中的字段 Apply 在 ACK 前进时继续、在作者／Base／库／节点／权限变化时保留提案并拒绝交付；资源组件继续核原操作恢复及真实 DocumentSession 的 ACK／ABA。受影响组件的源码清单包含共用 `source-input-basis.ts`，记录前后摘要和自有浏览器／Vite 的正常关闭结果；没有实际 Core／原生保存的受控结果不能证明其通过。

`node scripts/test_birthday_panel_browser.mjs NEW_ABSOLUTE_OUTPUT` 核真实 CaptureFields／Controller、DataPanel、HostedFields／PersonalControlInput 及原 Hosted 离页 handler。覆盖出生事件多选冲突、年／月精度、保来源与未知成员、明确选择与日期定位、同一 event_time 的 MonthDay 编辑、刷新／重挂、显式 Apply 与共享 Undo／Redo、invalid raw／IME 阻止结构动作、新增未知日期及自定义 schema 回退。按钮操作沿准确作者 producer 记录非文字意图，Capture 只交原 typed 草稿 owner，不进入通用个人文字库；冻结／退休／失效上下文拒绝操作，动作本身不是 ACK。此脚本使用受控传输，不能代替 Core 保存／原操作恢复、完整 Hosted 页面或原生窗口验收，也不证明保存后所有历史 producer 已结算。

`node scripts/test_property_resource_batches_browser.mjs` 另用于真实 Core／本地主界面链：设置同一构建批次 CLI 的绝对 `WEFTEXT_TEST_EXECUTABLE` 和全新绝对 `WEFTEXT_BROWSER_REPORT`，可用 `WEFTEXT_TEST_EXECUTABLE_SHA256` 限定程序摘要。脚本核相邻 catalog／receipt、物理 CLI 角色及受影响源码，直接消费内嵌 workspace，不接受 `--dev` 或 Vite 替代。范围是个人预设 staged 后显式 Apply、两个列表的准确移动与共享 Undo／Redo，以及一次资源批次提交后独立正文追加、撤销后资源／清单保留；夹具设置后不保存作者源。它保留同批实际请求和正常停机原件，结果须按实际候选执行记录，不能从脚本存在或语法检查推断通过。

`python scripts/mvp.py check` 保持完整产品入口，也可用重复的 `--group` 显式选择 `source`、`scripts`、`frontend`、`cli`、`rust`、`http`。分组只缩短开发反馈，不推断改动影响，也不授予未运行组通过：`source` 只运行源码／模型检查，不要求 Cargo 或 Node；`scripts` 运行原脚本合同；独立 `frontend` 组安装锁定依赖并运行测试与构建；后三组消费同批冻结的产品角色。默认完整入口保留全部检查、原 CLI 在较长 Rust 检查之前的次序，以及全部 40 个 HTTP 脚本。真实容量测试集中于 `reminder_device_state::integration_tests::capacity`，仍由完整 Rust 组逐项执行；64／65 成员、1024／1025 输入、合法 ledger 容量和真实 5 秒 permit 均不缩减。

Rust 组保留原 workspace／all-targets Clippy，随后依次运行：

```text
cargo test --locked --workspace --exclude weftext-app --message-format=json-render-diagnostics -- --exact --skip reminder_device_state::integration_tests::capacity::reminder_64_member_permit_checks_within_its_real_monotonic_lifetime
cargo test --locked -p weftext-core --lib --message-format=json-render-diagnostics -- reminder_device_state::integration_tests::capacity::reminder_64_member_permit_checks_within_its_real_monotonic_lifetime --exact --test-threads=1
cargo test --locked -p weftext-app --message-format=json-render-diagnostics -- --test-threads=1
```

首条测试命令保留其他 Core／workspace 用例的默认并行度与全部默认目标，只把完整名称匹配的真实 64 成员许可用例交给第二条独立运行；[固定 Rust 1.99.1 的 libtest 实现](https://github.com/rust-lang/rust/blob/1.99.1/library/test/src/lib.rs#L498-L537)对 `--exact --skip` 采用完整名称相等，不排除名称带此前缀的新增用例。第二条仍使用真实单调时钟、原 5 秒硬期限和全部容量／授权断言，作为隔离时间基准；不缩减 setup、放宽期限或省略用例。第三条运行完整 App 默认目标。三条合计覆盖原 workspace 的全部默认 lib／bin／doctests，并保留当前 manifest 的默认 feature 选择；Core 当前没有可选 feature。独立 App Core／HTTP fixture 共享进程级后台 producer registry，服务退出必须排空该 registry，因此 App 用例之间串行调度。每个用例内部的 Tokio 多线程运行时、并发 HTTP、真实容量、退出排空及原截止时间保持不变。直接运行 App 测试或带名称筛选时也使用 `-- --test-threads=1`，不依赖私有环境变量。隔离时间基准通过不代表并行 I/O 压力下仍满足期限；此前并行压力失败保留为该次实际结果，不能用隔离通过改写。

Windows 构建在固定双角色批次旁生成 `build-input-receipt.json`。`check --artifacts ABSOLUTE_RECEIPT` 在该回执记录的同一源码根执行，可用于实际 Git 检出或规范打包拥有的固定源码副本；它只复用这批产物字节，所选检查每次重新执行，不读取旧 PASS。回执严格绑定准确源码根／target／profile、完整公开源码与前端、双角色和原 catalog、实际顶层 Rust／Cargo／Node／npm 与所列明确构建配置；源码中的脚本、文档或 fixture 变化也要求新批次。每组边界及最终重新核输入；源根改变、缺失或未完成回执、非法路径、角色／字节／配置不一致均拒绝。资格观察要求外部 Cargo home、明确工具／SDK 路径及工具搜索目录为绝对路径；Cargo 配置中的自定义工具须以直接环境覆盖明确选择，未解析的 include、条件 linker 与 `[env]` 工具选择均拒绝，不猜测实际工具。它不证明所有隐式 SDK、系统库及传递依赖相同，也不证明可重现构建或整版验收。普通 `build` 继续支持没有 Git 元数据的解压 `source.zip`；`package` 仍要求实际 Git 产品检出，拒绝分组和外部回执，并在固定源码副本执行原完整检查与打包。

规范 `package` 在真实构建返回后、完整检查之前，用现有 `dependency_licenses` 收集器，依据实际固定源码根、完整 metadata 和实际 built package IDs，核 Cargo 依赖与已安装 npm 包的许可原文；CI 中这一步在独立回执子进程成功后执行。缺少原文立即停止打包并保留原物。随后 `package_windows` 从同批固定输入重新收集许可，只运行一次真实 SDK 资源／MSIX 生成，并生成完整 ZIP；原完整容器与 blockmap 校验通过后，才调用原全部检查。创建产物目录即写入 `INCOMPLETE.json`；此时只有未核定载荷，尚无 `PACKAGE.json` 或 `Unsigned local candidate` 成功标记。全部检查成功后，再核源码／前端／双 PE／catalog、许可原文与声明、SDK 工具、载荷／清单及容器完整字节和原摘要；只有仍相同才新建 `PACKAGE.json`、移除未完成标记并打印成功。SDK、容器、完整检查或最终复核失败都保留本次原物与未完成状态，不重建、不重试、不沿用旧 PASS。普通 `build`、`check` 和回执验证保持不变。

Windows CI 先初始化本轮证据目录，不在这一步构建；随后调用原规范 `package --release`，只构建一次新的固定源码 release 批次。原构建返回准确产物后、原完整检查之前，用已选定的绝对 Python 3.11 从该固定源码根启动独立进程，运行原 `mvp.py check --release --group cli --artifacts ABSOLUTE_RECEIPT`；它继承该批实际构建环境，包括原显式 x64 target、linker 和 SDK，记录命令、工作目录、完整原 stdout／stderr 与退出码。子进程失败立即阻断后续检查与打包；成功后先执行打包许可预检及上述真实完整容器生成／严格校验，再让原产物继续执行全部 source／scripts／frontend／格式／Clippy／Core／App／CLI 与 40 个 HTTP 检查，保留真实容量用例及原五秒期限，不以子进程 PASS 代替任何完整检查。只有完整打包成功，才提供同批对应的 ZIP／MSIX、PACKAGE 和 BUILD；原回执／catalog、checkout／run 身份与控制／CLI 原件均绑定该实际批次及固定源码。失败控制、部分输出、SDK 日志与未打包材料原件继续保留为未核定证据，未签名复核产物按原策略保留 7 天。材料获取见 [PACKAGING](PACKAGING.zh-CN.md)，不签名、改信任、安装或发布；下载后不得改写 CI 回执源根冒本地构建。上传成功不等于检查通过。

CLI 合同入口保留业务用例与唯一报告，Win32／ConPTY、受控 relay、安全失败摘要及纯传输夹具分别放在支持模块。2048 个确定性 token 的 DOCX 夹具仍经过真实转换、多块下载和完整摘要／内容断言，不以模拟产物代替。并发输入测试在真实冻结捕获后确认原宿主的新 worker，再执行原修改并核特定后置拒绝和严格事件先后；不能用初始捕获失败、任意 409 或 worker 失败冒充撤权／依赖重验通过。

并发修改用例只测量实际变更 POST：策略读取、导入准备和保存所需的 Draft ACK 均在原总预算内先完成。确认 worker 仍运行后记录 POST 开始，要求它早于原请求交付；随后核准确回执与实际回读。客户端收到 ACK 的时点不是服务端发布时点，拒绝回包可先到达。策略核同一 generation 及完整目标值；输入序号、保存和集合成员变更各核其原身份与实际效果，保留原错误码和无泄漏断言。

校对／Office／集合成员变更的并发交付夹具构造 20,000 份原生段落内容，只投影数量及固定代表文字，使它们能在原显示树与 worker 输出预算内到达后置授权检查；整批展开超预算是容量拒绝，不能算撤权成功。真实容量／压力测试继续独立运行原规模与拒绝断言，不用这个有界交付夹具授予容量通过。

`scripts/mvp.py` 的共享 Cargo target 使用一份可丢弃的构建输入提示，不能把 Cargo `fresh` 或相对 dep-info 当成当前检出已编译的证明；[Cargo 的文件时间判据](https://doc.rust-lang.org/stable/nightly-rustc/cargo/compiler/fingerprint/index.html)并不替代源码内容身份。提示绑定实际 Rust／Cargo 程序与版本、本地／path／vendor crate 源码、构建脚本及外部嵌入资源，区分具体命令模式与源码根上下文，核原成功产物及本地构建脚本生成目录的内容摘要。未知或变化时，先核包名归属、生成记录、路径与写句柄，再精确失效当前命令所选 host／target、profile 下相关本地包及其实际依赖图反向闭包的生成式短 fingerprint，并调整当前本地入口 mtime；源码字节和旧冻结副本保留。这样覆盖 dep-info 仍指向 target 内旧副本的情形，且必须观察本地包及受影响反向依赖的实际 `compiler-artifact` 非 fresh。被本地 patch 影响的 registry 父包随之重编，避免旧 Rust 元数据引用已替换的依赖；不 clean target，无关 registry 依赖继续复用。本地 vendor 无旧同源资格也需重编当前入口；有有效内容与 unit／产物资格后保持复用，features／profile 仍由 Cargo 管理。

入口先取得同 target 的产品锁，再调用 Cargo 自有锁；固定角色副本完成前不释放产品锁。直接 Cargo 调用不受此产品锁约束，须按同 target 构建协调执行，不能据此外推任意未协作并发安全。Python 在读取输出或等待期间被中断时，仍排空自有子进程的输出，确认该确切进程退出后才释放锁，并重抛最初异常；收尾期间再次中断也不提前释放锁。每条编译命令开始前清除可用成功状态，只有命令成功、输入内容稳定及实际产物核对后才更新提示；失败、被中断或源码变化均不记新源码成功。该提示不是验收记录，不能代替源码／前端／程序冻结。现有检查内的 `scripts/test_cargo_cache.py` 使用无第三方的自有小型离线 Cargo 项目核旧 mtime、两个检出、normal／test／feature、本地 patch 经 registry 父包的重编及无关 registry 复用、失败和锁顺序；另用自有 Python 子进程核受控的仅父进程 SIGINT、输出／等待及重复中断。这些用例不冒实际产品验收或真实终端键盘行为。

宿主启动重构复用同一 `weftext` 程序和原 HTTP 检查；`cargo test --locked -p weftext-app -- --test-threads=1` 同时覆盖库中的既有消费者、持久目录／启动参数／浏览器错误和本机服务关闭回归。库的 `LocalService` 生命周期用真实监听及 Core 租约验证：重复占用拒绝，宿主停止后原端口和私有状态可重新取得；端口冲突不能留下租约。停滞交接测试实际阻塞同一交接 helper 的回调，核对超时返回、宿主运行时销毁，以及浏览器尚未完成时 HTTP 仍可用且停止可完成；它不冒充真实 Shell 扩展或浏览器验收。目录／启动器选择的纯测试只证明选择和错误语义，不作为 macOS／Linux 实机结果。继续按影响矩阵分别验真实桌面、worker 进程、权限、草稿及恢复；本轮不为结构迁移另建验收体系。

受控 Linux 编译传输另须在 Linux 上执行既有 `cargo test --locked -p weftext-worker-broker` 及 `cargo test --locked -p weftext-app compiler_worker -- --test-threads=1`。Windows 执行仅覆盖共用严格帧／回执校验，不覆盖 Linux socket、取消、匿名管道取消或终局状态映射。显式运行的 `scripts/test_linux_worker_broker.py` 要求已独立审查的 root 所有部署，不得替换为无约束进程或省略失败的内核条件。宣称产品集成前，须对 broker 准确批准的产品映像执行托管渲染／编辑／引用授权及原操作回归。

Pull Request 和 main 更新还运行 [Product checks](../.github/workflows/product.yml)：Linux 使用 Python 3.11 调用同一 `check --group frontend`，按锁执行 TypeScript 测试、类型检查和 Vite 构建；Windows 使用固定 Node.js 24.19.0、Rust 1.99.1 和 Python 3.11，在一次完整规范 release 打包的同批固定源码产物上执行上述独立进程回执检查。原 Design checks 继续独立运行，产品作业没有允许失败或跳过断言的后备路径。CI 不发布，也不代替真实桌面输入、系统 IME、辅助技术、NAS 或其他操作系统的产品验收。

Windows 作业总预算为 150 分钟，容纳顺序执行的完整 CLI、Rust 与 40 个 HTTP 脚本。CLI 合同先于较长的 Rust 检查运行；失败时立即输出有界、脱敏的原异常和本用例近期子进程／relay 诊断，并标明本轮新建的结果目录与产物目录，避免把缓存中的旧结果当成当前通过。产品操作和各用例本身的截止时间保持不变。

0.6.3-rc.2 的外部 HTML 表格互操作使用 `node scripts/test_external_html_paste_browser.mjs EXE NEW_OUTPUT`，加 `--hosted` 验托管客户端。脚本在独立 HTML 页用浏览器原生复制，经实际剪贴板及可信粘贴事件进入本产品，核取消、合并格／换行／空值、未选内容和单步 Undo／Redo；不派发合成 ClipboardEvent，也不声明 Word／Excel 生产者已通过。`--dev` 只用于开发预检，候选验收消费 EXE 内嵌前端。

托管连接的独立面板故障由 `scripts/test_hosted_content_import_browser.mjs` 注入库配置目录 503 验证：准确报告该面板失败，资料导入和普通文档仍可使用；原 12 组导入／恢复／撤权用例继续执行。连接生命周期单测另核旧身份的迟到成功／失败不继续初始化或报告到新连接。此回归不自动关闭历史重启超时的根因问题。

查询浏览器验收对自有 `Other.typ` 的物理读盘，仅在 Windows `EBUSY` 共享占用时最多等待 5 秒并记录重试；其它错误直接失败。仍逐次比较原完整文件字节，不改正文、不改原断言或浏览器期限。此观察保护不证明原占用的内部持有者或根因。


原提交／结果交付修订沿同一 journal、Core 事务和受权消费者验证：`scripts/test_commit_delivery.py EXE [NEW_EVIDENCE_DIRECTORY]` 使用自有 TEST 库与真实 HTTP，保全部原件、请求／响应及程序对应，核原号重试、旧会话丢失、新输入后的保存依据、当前磁盘改版、坏 schema 下 capture／period 重试、实际提交后打开失败、持久取消与备份、无效 Unicode 的冻结字节摘要终结、UUID 别名及未知安装证据。该脚本接入 `scripts/mvp.py check`；Core 的确定失败注入另核提交后 Draft 更新，不把受控响应替换称为内部 Core 故障。测试专用 creation helper 严格解包新回执，不为产品保留旧响应格式。

开发 profile 仅将 `sha2` 依赖以 `opt-level = 3` 编译，测试 profile 继承此设置，避免每条 CLI 命令对大型调试程序做完整身份校验时执行未优化的哈希内环。其余开发代码仍沿用原设置；SHA-256 算法、原文件句柄上的完整读取、owner 资格和所有期限均不变，不缓存或省略校验。发布 profile 独立保持原设置。

## 按影响选择验证环境

常规改动优先使用纯引擎、HTTP 和可用的无头／后台浏览器，避免占用宿主焦点。按本轮改动和实际消费者选择下表中的最小范围；仅有窄屏视口不算 Mobile，HTTP 或 DOM 赋值也不算真实输入法、触屏或辅助技术操作。

| 影响范围 | 优先执行环境 | 可证明的边界 |
| --- | --- | --- |
| 算法、Core、Query、保存协议与接口 | 仓库现有引擎测试、故障注入和实际程序 HTTP 场景；以同批固定 `weftext-cli.exe serve` 启动自有服务 | 证明所跑算法、接口及故障模型；进程中断不冒实际断电，Windows／NTFS 结果不外推其他文件系统。 |
| 浏览器布局、五档、草稿、导航与资源呈现 | 无头或独立后台浏览器，在相同候选上检查实际渲染及浏览器交互 | 记录真实浏览器与场景；前端单元测试不等于浏览器实测，浏览器模拟输入不建立 OS IME 或完整 AT 通过。 |
| Windows 安装／更新、系统文件集成、真实 IME 与辅助技术 | 按实际需要选择具备相应输入、捕获和正常关闭能力的本机桌面、Windows Sandbox 或独立 Windows VM | 本机使用独立测试资料，提前说明前台目的与时长，保护无关窗口；无需先迁移 VM。客体另验输入、产物回收和宿主焦点隔离。Sandbox 可启动不代表最小化、断开查看器或无人值守可用；重启和持久系统状态选择具备相应能力的环境。 |
| Linux／macOS 桌面及其文件后端 | 对应真实 OS 的独立测试机或合适 VM | 分别验证安装、文件语义、键盘／IME／AT、字体与关闭恢复；Windows、容器或 WSL 的命令行结果不建立该桌面通过。 |
| Android／iOS Mobile | 各自 Android 模拟器／iOS Simulator；无法由模拟环境证明的项目使用必要真机 | 分别验证系统文件访问、真实软键盘／触屏、生命周期、离线、字体及平台 AT；浏览器视口、模拟器或另一移动 OS 的通过不替代必要真机项目。 |

执行前区分三件事：**已声明**是工具或文档列出的能力，**可用**是本次实际环境、会话、输入与捕获能够工作，**实证**是指定候选的指定场景已经观察到结果。首次接入或环境能力变化时，用一个小场景确认隔离环境的输入、观察、正常关闭和结果取回，并记录宿主焦点、鼠标与剪贴板是否受影响；后续复用有效环境证据，只核对本次身份、会话与受影响能力。这只确认执行环境，不授予产品通过。当前仓库未记录上述 Sandbox、VM 或移动环境的 Weftext 产品实证。

启动失败、客体锁定、输入／捕获不可用或会话失效时停止该场景，明确记录未验证或工具错误，不静默回落宿主前台。确需前台的启动／重连或人工场景，提前告知用户所需操作与预计时长并安排短窗口；持续验收输入仍限定在目标环境。失败后只重跑受影响范围，不为维持后台执行而改变产品期望。

## 复现、回归与候选

下述浏览器／HTTP 命令的 `EXE` 均指定本次固定角色映射中的 `weftext-cli.exe` 绝对路径。浏览器脚本使用具备 JSON 整数原文及原样序列化能力的 Node.js 24 或更新工具链，以准确保留 Windows FILETIME 的 u64；缺少相应能力明确拒绝执行。共享启动器消费唯一 Ready JSON，正常关闭须核准确 OwnerIdentity 的关闭回执及原进程实际成功退出；未知结果保留现场，不作通过或自动重试。

问题复现保留原候选、环境和最小触发步骤，记录实际观察；只复现一个故障不能报整场景通过。修复后用新候选重跑反例和直接受影响的正常路径；实现、依赖与环境没有变化且身份可确认的通过结果可以复用。发布验收另核对目标平台承诺、此前未验证项及候选完整性，不能把复现或执行环境资格当成发布通过。

复用仓库的构建／检查／打包入口，每个平台的一轮候选只构建一次，再让后台、客体与必要人工场景消费同一份产物。默认 `python scripts/mvp.py check` 与 `package` 都会先构建并检查；按是否需要打包选其一。需要分组或跨进程检查时，使用本次构建打印的新回执，从回执绑定的同一源码根以 `check --artifacts ABSOLUTE_RECEIPT` 检查该批产物，不串联多个默认入口重复构建。Windows 打包参数、同次构建的 GUI／CLI 角色及严格离线材料前置见 [PACKAGING](PACKAGING.zh-CN.md)：`package` 在固定源码副本检查和生成，材料不全则拒绝完整候选。`package` 后在该打包候选上完成补充实测；源、依赖或打包内容改变则建立新候选并检查受影响范围，不沿用旧产物通过。其他平台尚未交付相应候选入口。

每个可交付切片只保留一份简短验收记录：提交或候选标识（必要时产物哈希）、目标 OS／浏览器／客体或设备版本、改动影响及场景、实际命令／手动步骤、通过／失败／未验证／工具错误、已知限制与必要证据位置。环境资格与产品结果分清；仅失败或关键操作保必要截图／片段，正常日志保摘要，不新建另一套验收事实库。用户真实输入与输出分开放置，原件不被测试覆盖。文档链接检查和有限模型不计作产品验收。

输入延迟使用 `node scripts/test_input_latency_browser.mjs EXE NEW_OUTPUT`，加 `--hosted` 测受控托管写作者。默认每条件 1000 次可信键盘事件，普通、十万行及 1 MiB 单行分别冷／热各三轮；完整源码与实际可见 DOM 同时核对。计时从 DOM keydown 到第二动画帧回执，是反馈上界，另存原始 Chrome Layout trace；浏览器传输、OS／IME、整机 steady memory、10 MiB／移动端及全套参照门不由此覆盖。`--smoke` 仅校准；`--dev` 使用当前开发前端，不作内嵌候选验收。每次保留原始结果及源码／EXE／脚本对应；失败或未跑不能记为通过。

同一输入脚本可用 `EXE NEW_OUTPUT --hosted --smoke --fixture=100k-lines --recovery-check` 检查大稿恢复：实际阻断服务端草稿交付，刷新后核完整未保存正文，关闭自有后端释放文件租约后核磁盘原件未变。该模式的少量输入只作校准，不替代完整时延矩阵；失败现场与普通退出回执分别保留。

`node scripts/test_hosted_personal_input_browser.mjs EXE NEW_OUTPUT` 在自有 TEST 库和独立无头浏览器验证未上传本人输入：仅注入后台草稿网络失败，实际撤权／401 后重新认证，核纯输入、账号分隔、准确基线恢复、共享 Undo、迟到原生回应、存储失败及原保存编号。程序赋值／IME 负向用例不授予保护文字来源。设置 `WEFTEXT_PLAYWRIGHT_MODULE` 可指定已安装的通用 Playwright 模块；不操作用户浏览器。该定向检查不代替完整候选验收或真实系统 IME。

现有工作区浏览器消费者先通过 `scripts/_workspace_browser.mjs` 选择实际任务选项卡，再进入对应命令；对话框动作仍限定于自身范围。权限负例先确认仍可读的 inspector 已实际呈现，或检查原控制器自己的 hidden 状态。注释编号任务另核独立本人副本的续编、完成与重开，始终比较原恢复记录及原操作全对象，再核另一账号无副本 DOM 或值泄漏。结构任务注入稳定引用缓存写失败时，要求零新 writer 接纳且原稿／缓存不变；解除故障后仍由同一侧栏入口重试。

`node scripts/test_capture_workspace_browser.mjs` 通过明确的 `WEFTEXT_TEST_EXECUTABLE` 消费构建批次的 CLI 及相邻目录中的 catalog／receipt，使用新的绝对 `WEFTEXT_BROWSER_REPORT` 目录、已安装的 Chrome 和 `WEFTEXT_PLAYWRIGHT_MODULE`。它验证真实 Core、Capture 界面和 IndexedDB 之间的独立草稿切换、一次创建回包丢失与重开恢复；可选 `WEFTEXT_TEST_EXECUTABLE_SHA256` 限定程序摘要，`WEFTEXT_BROWSER_FIXTURE_PARENT` 指定独立短夹具父目录，默认使用系统临时目录。实际挂载组件的退役／迟到 schema 用例另由 `scripts/test_capture_mount_browser.mjs` 以受控传输验证。两者均保留自有资料及清理记录；真实 Core 检查另留原 owner 停机回执，不授予真实原生窗口、系统 IME、辅助技术或整包通过。

`node scripts/test_capture_status_browser.mjs` 使用新的绝对 `WEFTEXT_BROWSER_REPORT` 目录，验证实际本地／托管挂载组件查询原操作后的页脚、失败诊断、换稿和迟到响应。它使用受控传输；查询呈现不改变 pending、输入或提交资格，也不替代真实 Core、原生窗口或交付验收。

`node scripts/test_workspace_layout_browser.mjs ABSOLUTE_CLI ABSOLUTE_NEW_OUTPUT [--dev]` 在自有库和独立无头 Chrome 验证单一树展开、折叠保留父节点、功能区与文档标签、实际编辑器／选区／Undo、格式操作及响应式布局。默认消费程序内嵌前端；`--dev` 仅为当前开发前端诊断。CSS 等效缩放、合成组合键和浏览器结果不替代真实缩放、系统 IME 或读屏验收。功能入口按任务选项卡到达，不通过强制点击隐藏控件维持旧测试。

`scripts/test_everyday_browser.mjs` 的本地及托管日常任务通过实际功能区进入相应命令；托管文档返回检查使用中央画布和源码编辑器各自的非零滚动位置，不以页面的 `window.scrollY` 代替画布位置。

## 发布

当前 `python scripts/mvp.py check` 检查本地实现，`package` 生成带源码和声明的本地候选，不上传发布。正式发布复用已验收的同一产物，不在发布步骤重新构建另一份。发布前明确目标版本、平台、未通过项和用户授权；发布后核对下载内容及最小运行场景。

仓库保存可独立执行的产品构建/检查/打包入口。个人工作区只负责路径与本地环境，不建立另一套发布平台。发布产物校验与关键交接使用哈希，不对每轮讨论重复计算全库哈希。只有正式交付需要标签和发布记录；当前不创建空发布。

历史交付修订另核真实 Core 结果首次读取失败后恢复可读、持续不可读时仍保 committed/receipt，以及 missing→期间提交→通用终结→typed 历史结果读取。适配器单元测试用自有目录确定移动和复位控制两次读取边界，不制造定时竞态；真实 HTTP 用例复用 test_commit_delivery.py，既有本地历史消费者由 test_v05_completion.py 核验。

同一交付入口再核原 X 正常移动后旧槽创建 Y、同 NodeID 物理替身、库根替身、另存为同字节替身、完整 Definition SaveRequest 重放及重启无旧 Session、enableSync 原 journal 状态。`scripts/test_creation_delivery_browser.mjs EXE NEW_EVIDENCE_DIRECTORY [CASE]` 覆盖真实接纳后刷新失败、原号补交的标签幂等与 Y／选区／Undo、另存为接纳及不可交付处置后沿正确 Session 继续保存、普通旧标签遇同字节物理替身、首次及查询文件保存坏回包后原号恢复、本地历史换 Session 后原号查询／终结、S 补交、后来 T 的比较门，以及真实冲突草稿处理响应等待期间输入 Y。浏览器 abort／500／延迟响应和坏包替换是明确的受控传输或交付故障，不称后端自然产生坏 DTO 或内部 Core 读盘失败。托管历史另跑实际授权矩阵，Core 内部故障点核首次 committed 与瞬时 Draft 警告，纯 TS 负例核坏 DTO／矛盾状态与重连。v0.6.1 初始化同族严格核原取消 resolution、无伪作者 Plan、准确原请求指纹与原字节、重启和迟到请求拒绝；各自证据边界分开记录。
