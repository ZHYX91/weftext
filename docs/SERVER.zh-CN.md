---
source_language: zh-CN
translation_status: source
---

[English](SERVER.md)

# 受控 Server / WebUI 候选

`0.6.0` 的 Windows／NTFS 既定 Server / WebUI、多用户权限与同所有者同步范围已完成，同一最终优化程序通过完整 HTTP、受影响浏览器和真实 NAS HTTPS 验收；尚未正式发布。它复用单 writer Core 的文件保护、原操作与恢复证据，并隔离个人草稿。当前范围和限制见 [MVP](MVP.zh-CN.md)，后续版本见[路线图](ROADMAP.zh-CN.md)。

当前工作树为 `0.6.3-rc.3` 稳定化候选，整版验收仍在进行；第二阶段及早期候选的通过记录保留原产物边界。保留资料导入与识别、表格、公式、引用、注释及处理设置，并接通多级标签和知识视图。托管大稿采用同步紧凑恢复缓存，保实际基线和原未决保存请求。新增能力继续经过独立托管授权与原操作恢复，具体范围和未验项见 MVP，不由本地实现或本文合同授予通过。

## 属性与多 Query 组织

多个内嵌 Query 的重排／移除仍使用当前获权正文会话与 Draft／Undo；阅读档不显示作者控件。`/api/hosted/query/catalog` 的 `formSchema` 只包含独立字段读取权覆盖的最小字段／形状闭包，不披露未获权类型、关系或别名；没有可读字段时不返回完整 schema。结果就地编辑另查具体 resource／FieldId 的字段写权，沿原字段操作与恢复，不因正文或 Query 可读而得到写权。共享定义和 current／pinned 引用沿原定义读取／历史资格，修改调用不会改定义。撤权、退出清除保留编辑器、结果及页面内个人视图。具体控件与未验平台边界见 MVP。

## 资料导入与离线识别

0.6.2 的“资料导入与识别”使用已有 `contentImportSlots`，不接受浏览器传服务端路径。每个用户的 `contentImports.SLOT.recognition` 独立配置 `run`、`read`、`import`、`download`，默认均为 false；运行须同时有结果读取权，导入及下载也分别检查读取权。`members` 的正文／字段／历史／输出与 `resources` 的读取／下载仍分别配置，识别权不授予这些后续权利。映射到已安装 FieldId 时还须对应成员字段读写权与 schema 读取权；未映射候选只作为可编辑正文，不冒充 typed 属性。

执行端显式设置 `WEFTEXT_RECOGNITION_RUNTIME` 为本部署独立安装目录，再启动程序。安装／核验入口见 [识别运行时](../scripts/recognition/README.zh-CN.md)；程序使用内嵌的固定 worker／规则，不搜索其他工作区、PATH 后端或在线模型。缺依赖仍能保留原件。工具安装和模型单独维护，不随候选包暗中再分发未完成许可材料的二进制。

`recognition/capabilities|fields|start|status|cancel|preview|download|prepare|commit|operation|settle` 均在 `/api/hosted/` 下。原件不超过 16 MiB，PDF 不超过 100 页，单页最多 300 DPI／40 MP／60 秒 OCR；整次执行最多 15 分钟，识别与原页预览共用两个执行名额，新的保留资料受 256 MiB 预算约束。原 jobId 重试不重跑识别；原 operationId 查询／结束不重放创建。部分、未处理、失败与空白各自呈现，不能把所选范围完成当整份原件全部完成。超时／重启保留已完成页的来源检查点；缺少最后完整性检查时仅能读取，不能用接受部分或接受损失绕过，须明确新作业重跑。15 分钟确认资格过期后，仍可在当前授权、原件、规则和运行时一致时只读查询已有状态／页面文字并显示过期，不再允许原 job 启动重试、原页预览、下载或新确认；原计划提交继续检查自己的期限。未确认计划的原件、模型、规则、目标或授权变化即失效。

识别原文、核对稿与已保存原生正文分别显示。下载的原始提取文本不包含后续作者修正；随附来源／损失说明绑定准确文字及原件摘要。两项下载均重新核权。来源、资源、搜索、历史和已生成工件在撤权后不得继续交付；本地与自托管执行结果仅证明各自所测环境，不外推浏览器本机 OCR 或其他平台。

## 配置与启动

先用本地模式创建资料库及笔记，或选择已有普通 `.typ`。管理笔记须使用原有 `--data-dir` 中的身份和历史证据。关闭本地进程，再启动 Server；同一作者树只能由这一 writer 使用，不可另启不同私有状态目录的实例绕过所有权。

在资料根之外创建配置。以下例子只开放两篇已有笔记；脚本 create-only，不覆盖旧配置，也不会启动或公开服务。它输出一次性随机凭据，配置只存 SHA-256 摘要；分别保管管理员与个人凭据，不放入源码库或同步根。

```text
python scripts/server_config.py --root "D:\Notes" --out "D:\Weftext-Service\server.json" --document shared=Shared/Shared.typ --document other=Other/Other.typ --user alice --user bob
weftext.exe --server-config "D:\Weftext-Service\server.json" --data-dir "D:\Weftext-State" --no-browser
```

默认绑定 `127.0.0.1:47821`。浏览器打开服务地址，输入个人凭据；管理员凭据不用于正文登录。配置中 `root`、文件 ID / 相对路径 / 显示标题 / `editable`、明确的 `inputs` 依赖和部署 ID 由受信管理员选择。客户端只传对象 ID 或已打开的会话，不能传本机路径、依赖根或自行声明用户身份。每个用户的每个对象有分别的 `read`、`write`、`history`、`download`；缺项拒绝，禁用用户拒绝全部访问。后三项还要求 `read`；资源可以单独获权，输入依赖不继承主文档权限。

本版本不内置 TLS、密码账户、SSO、服务安装或高可用。内网部署应在受信 HTTPS 反向代理后运行，配置准确 `origin`（如 `https://notes.example.test`），代理保留该 Host，凭据只经加密连接传递。实际监听地址由 `bind` 指定；不自动改防火墙或部署代理。非回环 HTTP 只有显式 `allowInsecureHttp: true` 才启动，用于受控测试网络；WebUI 的安全上下文功能仍要求 HTTPS 或回环地址。Host 和存在的 Origin 必须完全匹配，服务不信任转发头授予权限。已验本机 HTTP / 浏览器场景不冒真实跨机 TLS 部署通过。

## 写作、冲突与断线

连接时，各面板按各自权限初始化。一个目录暂时失败会显示“未就绪”，其余获权入口继续连接；例如库配置不可用不阻断资料导入。可点击“全部文档”重试，正文草稿保留，成功后更新连接状态。身份或授权变化会停止旧连接，失败隔离不会恢复读取权。

WebUI 提供源码、带样式源码、按需语法、可视写作、阅读五档正文入口，以及显式依赖内的 Typst 预览、授权范围内的标题 / 源码文字搜索、准确历史源码查看，以及已保存源码下载。搜索范围明确为配置中获权的源码，超出预算或读取失败显示不完整；此搜索入口与下述按字段授权的 CEL Query 分开。普通 `.typ` 没有被接纳的笔记历史时明确不可用。

同篇文档的两个用户保留各自草稿。先保存者产生真实保存；另一个旧 Base 的保存被拒绝，保留原草稿和当前稿。重新读取后明确核对、编辑合并内容并确认新依据，最后按“保存”提交。确认依据本身不改正文；原冲突草稿由 Core 保留。另一篇文档不会只因这个冲突停止保存。

浏览器输入立即保存在本标签页会话存储中，并在联网空闲时保留到服务端的个人草稿。断线时可以继续输入，不获得本地提交资格，也不自动重试保存。断线期间重载页面并不保证可加载应用；恢复连接后重新打开文档可核对仍在同标签页存储中的草稿。浏览器关闭、私密模式或存储清理可能丢失尚未送到服务端的输入，存储写入失败会提示。不同用户、设备和浏览器标签不共享本页恢复存储。

显式正文、可视文字和作者面板的新增输入另存入本部署、本账号的纯输入恢复域。它仅包含实际键入或显式粘贴的新文字、来源位置／基线摘要及原保存编号，不包含旧 Base、完整保护正文、Schema、预览或凭据。注释共用“取回本人未上传输入”入口。已注册控件携带其实际正文、字段／路径、讨论／媒体、库角色或注册目标，不借当前打开正文补 owner；搜索、筛选和临时 Query 参数排除。保存成功推进准确提交原子的非撤销检查点，后续新字映射到该保存 Base；跨保存 Undo/Redo 保留原来源，已确认原生输入继续只供复制。剪贴板续接和 IME 终止事件必须保持原绑定代次。401、退出或代次变化先保已观测的新输入，再清除保护缓存；只有重新验证原账号后才能取回。来源不明的原生替换、表单历史及未应用面板文字只供独立复制，可能已应用或撤销，不冒充当前正文。源码恢复须重新获权，并核对同一稳定目标和准确基线；未知原保存先按原编号查询／落定，不能换号重发。额度或存储失败保内存原文并明确提示未可靠持久；不能据此保证关闭页面后的恢复。尚未打开文档的结构捕获稿仍属独立待实现范围。

退出会先使用原凭据释放本页持有的文档会话，再清除本页凭据、展示和缓存；最终离开页面时也尽力释放会话。浏览器保留的前进／后退缓存页面暂不关闭会话，恢复后仍须通过当前授权与会话检查。释放只移除内存会话和编辑版本，不删除作者文件、已经持久保留的个人恢复稿或原保存操作记录。退出后迟到的临时打开结果也使用原凭据清理，不借用新登录用户的凭据。

文档会话在 30 分钟没有获权请求后到期。在线页面的既有五秒授权检查同时为当前文档调用 `POST /api/hosted/session/keepalive`；单独查询身份不会续期其他文档或遗留会话。服务每分钟回收已到期会话，并在判断打开数量上限前再次回收；新请求不能复活已到期会话。网络中断或页面长时间冻结后，界面保留本页草稿与 Undo，并要求“重新读取与核对”后继续保存，不自动重开或重放写入。

每次保存先保留准确的操作编号和提交内容。丢响应时查询同一原操作；服务重启后仍可按用户查询原结果。未知 / 待恢复状态不冒成功，也不换新编号重复保存。查询为未登记 / 未提交时，先持久结束原请求，阻止迟到的原请求再提交，之后才允许用户明确提出新保存。收到保存回执后保留请求期间的新输入。服务端个人草稿与操作记录存放在原私有状态目录的 `actors/`，不放入资料树。完整恢复资料备份一致捕获所选库及相关私有记录，包括各用户与该库相关的草稿和原操作 fence；不会导出共享私有目录中不相关库的内容。独立授权入口及旧记录限制见下文。

## 撤权与交付

管理员使用 Bearer 凭据读取 `GET /api/hosted/admin/policy`，然后用 `POST` 提交 `{expectedGeneration, users}` 的完整新策略。更新以当前代次条件提交并持久化；旧代次冲突，重启读取已保存策略，不用初始配置恢复已撤销授权。此入口不支持改资料路径；初始配置中的 users 在首次启动后不再覆盖已保存策略。管理员凭据的变更通过停服修改私有配置完成。

每个用户有服务端产生的授权代次。读取、搜索、历史、保存及每块下载重新授权；预览在启动前只冻结各自获权的明确输入，编译后交付前再检查该用户代次。冻结 World 不从磁盘补缺失输入，编译器不返回服务器绝对路径诊断。撤权与保存 / 单块交付的决议通过同一短时服务边界排序；已放行的字节不能撤回，新块不再放行。失效后的旧会话和下载票据不会因重新授权而复活；其他用户未变的授权和保存会话保持有效。

下载先冻结当时的准确字节、长度和摘要，每次请求最多 64 KiB；后续保存不替换这份工件。票据绑定用户和授权代次，15 分钟后到期，服务重启失效，不能给另一用户续传。WebUI 收完后验证摘要才交付文件。接口响应均禁用浏览器缓存；在线空闲界面每 5 秒检查授权，确认变化或无权响应时清空旧展示、预览和包含保护原文的本页恢复缓存，纯本人输入按上述独立规则保留。不能撤回用户已经收到 / 另存的内容，也不声称离线浏览器立即得知撤权。

## 预算、验收与剩余范围

配置最多 128 用户、256 文件；单用户最多 32 个打开会话、服务合计 128 个。源码 2 MiB；预览最多 64 个明确输入、32 MiB 冻结文件，复用编译进程时间 / 内存预算并只允许一个活动预览。搜索累计最多 16 MiB 源码；下载暂存最多 32 件、64 MiB。耗尽返回明确错误或不完整结果，不以子集冒充全集。

`scripts/test_hosted_review.py` 继承 `test_hosted_fields.py`、`test_hosted_editor.py` 和原 `test_hosted.py` 的完整检查并接入 `scripts/mvp.py check/package`，测试真实程序的双用户冲突、独立草稿、操作幂等与重启、拒绝本地路径 API、隐藏对象不披露、历史分权、冻结编译、同一字节续下载及持久撤权。浏览器还需验证登录、丢响应、断线草稿、合并和缓存清理；实际结果随候选记录，不由文档建立通过。

以下章节描述当前托管实现合同，包括结构、知识链接、目录／标签、文献和输出格式；合同接通不等于完成候选验收。每项仍须按实际程序、HTTP、浏览器及需要的真实宿主证据确认，不能借本地 API 越过托管授权。实时协作、其他 OS 后端、Mobile / CLI 整体验收以及自动化按后续里程碑推进。

`files.<id>.editable` 限制正文、字段及恢复等写入，不再把已纳管的只读笔记排除在阅读之外。live 的 `editable: false` 对象只有在准确 `.typ` 路径上经 Core 证明原 Managed Node 身份、当前位置、`expectedNode`（若配置）及文件 witness 时才成为可打开的作者文档；JSON、二进制和未接纳的普通编译输入不会因此出现在文档、搜索或稳定导航中。原 `editable: true` 普通源码阅读能力保留。只读 Managed Note 可作为链接／反链／提及目标，并按独立权限查看历史、窄字段或输出；正文 Save、历史恢复、字段写入及 Office 导入仍须各自写权限和 editable。字段读取仍可独立于正文 read。

## 保存历史与稳定 Node 导航

历史列表、准确源码及比较复用 Core 的原保存记录。`read` 和 `history` 共同授予查看资格；恢复还需 `write`。比较明确区分当前已保存版本与当前草稿的固定快照，不保存正文，不替换编辑缓冲区或清除 Undo。仅提供原历史源码与文本差异，不以当前依赖冒充历史富预览；缺失版本明确不可用，不回退到最新版本。返回位置使用获权的资源编号，不交付服务器绝对路径或其他用户的私人草稿。

恢复把选中版本作为一次新的普通保存，保留后续历史。浏览器先持久保留完整原请求和操作编号；有未保存输入时须明确保留草稿。Core 同时保护该用户已持久化草稿与此次缓冲快照，原基线保持原样，其他用户的草稿继续独立保留。恢复提交后，原脏稿或请求期间新输入留在共享编辑器；仅仍匹配原依据的干净缓冲接受一次可撤销替换。新的保存依据来自当前已获权文档。

`/api/hosted/history/list`、`read`、`compare` 和 `compare-draft` 提供只读消费；原 `/api/hosted/history` 列表入口保留。`history/restore` 固定 actor、资源、实际 Node/FileIdentity、所选版本、Base、准确源码快照及保稿选择。`history/status` 和 `history/settle` 以原资源和原操作编号核对，不要求已过期的编辑会话。未登记请求可持久结束，阻止迟到恢复；已提交或可能已有影响的操作不能冒作取消。相同编号改变意图冲突，也不能改作普通 Save 或其他操作类别。重启或丢响应不另造恢复操作。

提交事实与结果交付分开：原操作已提交，即使配置资源后来指向另一个对象，仍可返回该用户的原提交事实，但不能交旧对象内容。原结果可读时，回执标明其依据仍为当前、已被后续保存替代或发生外部修改。同字节的另一次保存也不复活旧回执。浏览器核对当前文档后再应用；旧结果只供比较，不把旧 Base 绑定到较新的正文。已知提交但结果不可交付时，可明确结束本页回执应用，重新读取并比较当前稿；不取消 Core 提交、不自动重基，未知或待恢复操作不能使用此出口。已核对的终态记录可明确清除本页缓存，未决请求继续保留。

稳定链接使用原生 `weftext-node://` / `NodeRef`。`navigation/ref` 读取当前获权文档的稳定引用；`navigation/resolve` 仅在部署者明确配置且用户具有独立 `read` 权限的候选中解析，再通过原打开流程保护当前草稿并切换。Core 核对当前声明、真实文件身份、已证明的移动和边界，不按同名或路径猜对象，不扫描整个库补充候选。Core 移动后更新部署配置可继续解析原引用；复制产生独立身份。隐藏、缺失、歧义或资格失效目标一致不可用。源文档读取权不授予目标读取权，链接也不授予写入或历史权。

带 `#label` 的稳定引用分别解析身份和 UTF-8 片段，支持中文及百分号编码；空标签、无效编码、控制字符和多重片段拒绝。`navigation/resolve` 先校验独立目标权限与准确所有者，再从获权输入编译已保存目标。片段状态区分 found、missing、ambiguous、unavailable；只有真实编译标签唯一且有准确主稿范围才返回定位。源码中未执行的标签、生成内容无可证明范围或辅助模块标签不会被猜成主稿位置；隐藏与未知对象维持一致不可用。

`navigation/fragment` 使用完整稳定引用、sessionId、source、sourceHash 和 serial，核对目标当前会话及准确个人草稿，再复用原编译环境；交付前重验权限、所有者、源码与原依赖。范围只指向该主稿，不交付宿主路径或辅助模块源码。浏览器在定位成功前保留源编辑器及 Undo；另一个目标用临时会话核对实际本地／恢复草稿，不能套用已保存版本的范围。缺失、歧义、无法定位或原保存未决时，保留原页面并提供明确的“仅打开笔记”选择。同文档定位不重开，迟到结果不能覆盖后来输入、其他目标或撤权状态。`scripts/test_hosted_navigation_fragments.py` 纳入 check/package；实际验收范围沿用候选记录。v0.6.1 Web 深链复制另需真实部署路由与登录返回验收。

`scripts/test_hosted_history.py` 通过真实 HTTP 覆盖分权、双方草稿、准确比较、恢复与原操作重启、外部冲突、配置重绑和移动／复制导航，并纳入现有 `scripts/mvp.py check/package`。浏览器的 Undo、迟到结果、持久保留失败与撤权另由 `scripts/test_hosted_history_browser.mjs` 验证；实际通过范围仍随候选记录。


## 知识链接、字段引用与潜在提及

`POST /api/hosted/knowledge-links/read` 消费 `{sessionId, source, sourceHash, serial}` 的准确当前草稿，返回原生出链、ID／路径反链、最终编译链接、字段引用及潜在提及。候选范围明确为 `configuredAuthorizedManagedSources`：先按每个对象的独立正文 read 筛选，再资格化其原 Node 与文件 witness，不扫描环境库或按同名补候选。资源和集合另依各自权限捕获。隐藏和未知目标一致不可用；read-only Managed Note 也可成为目标。预算为 16 MiB 源码、32 MiB 保留依据、最多八个相关编译及 8 MiB 返回数据；覆盖显示 complete／partial，编译不全、歧义、未纳管对象或预算不足不被冒作完整无结果。

原生标题和原文链接来自正文 read。别名仅从独立 field.read 的 `core/aliases`、`contacts/names` 推导，typed refs 仅从独立获权 Field 的最小定义和值推导；必须在推导前收窄 Schema。源码里已有相同字值不等于获权私有 Schema 意义，library.schema.read 也不扩大 Field 权。没有字段权时不读取 Schema；有字段权时私下保留原 Field 权与 Schema witness，交付前复核。字段引用范围为 `explicitAuthorizedFields`，提及词典范围为 `nativeTitlesAndAuthorizedNameFields`；重叠词和同词的多个获权目标保留独立选择，不替用户猜目标。

`knowledge-links/convert` 与 `copy` 消费原完整草稿 basis、read 返回的 snapshot 和准确 occurrence／提及范围。转换只产生严格当前草稿的范围补丁，`saved: false`，由共享 SourceEditor／Undo 应用并通过原 Save 提交；只读当前源不能转换回写。ID、源相对和库内地址保持原 URI 片段编码与显示内容。original 复制允许保留原已写入的 URI 和完整链接，即使目标未解析；生成 file 地址拒绝，不能用任意 file URI 授予宿主路径。权限代次、原 owner／witness、字段权、Schema、资源／集合和原编译输入均复核；旧 serial、同字节 ABA、换会话或撤权后的迟到结果不能安装。

`scripts/test_hosted_knowledge_links.py` 提供源码范围、原草稿不保存、重叠候选、字段语义权限、片段、只读目标、配置变更及迟到撤权的 HTTP 入口。新增合同和测试源码不构成已通过 HTTP 或浏览器 Undo 的声明。

## 同所有者远程托管同步

Server 可额外托管一个明确配对的 manifest/blob 历史。`syncIdentity` 必须由部署者提供原副本的 WorkspaceId 和 genesis 摘要；`users.<id>.syncOwner: true` 明确授予该配对的完整历史传输能力，默认为 false。它与文档 read/write/history/download 分开；文档作者、窄字段客户和管理员登录凭据不会自动获得此能力。不要把完整同步凭据发给仅应读取部分文档的用户。此模式信任自己的设备和托管账户，摘要用于完整性，不声明恶意服务端或已窃取同所有者凭据的设备防护。

配置助手新增 `--sync-workspace-id ID --sync-genesis DIGEST --sync-owner USER`，其中 USER 必须同时通过 `--user` 明确创建。文档列表现在可以为空，因此可用已有空文件夹作为 root 建立独立 relay；服务不会把接收的远端记录直接写入 root。若 root 已是同一 Core 私有状态下的已配对副本，配对必须一致；服务端授权作者的真实保存会进入原副本 outbox，在完整同步读取时发布，不复制其私有草稿或策略。

`syncIdentity` 同时绑定持久策略，已存在的完整同步授权不能随启动配置悄悄转到另一配对。向原本未启用同步的部署添加配对时保留原用户策略，原启动配置中的新 syncOwner 不自动覆盖它；须通过现有管理员策略接口明确授权。撤销 syncOwner 会改变该用户授权代次，旧票据即使重新授权也不复活，其他未变用户的票据继续有效。

本机工具栏“同步 → 远程托管同步”中输入服务根地址与完整同步凭据。可上传所选本机对象，或读取远端目录、选择对象并接收；接收完后仍须通过原 clone/apply/merge 预览改变作者文件。已收到与已安装的版本分别显示。非回环地址须 HTTPS；回环字面 IP 可用于 HTTP 验收。客户端验证证书，禁止重定向和环境代理，不从 URL 查询参数或 userinfo 取凭据。凭据仅在当前界面及活动进程内存中使用，不写浏览器存储或 author tree，关窗清空输入；已启动传输可继续，停止按钮在当前块结束后停止后续请求。

传输协议按完整摘要记录幂等，单块 64 KiB。下载先冻结准确记录集，每页目录最多 128 项，每块交付重验 actor/代次/当前完整同步权；后台新提交不替换原下载。上传按同一 actor/代次/记录续接缓冲，重复相同块不重写；完整记录校验摘要和 Core manifest 规则后才入私有历史。错误字节不成为作者版本。服务重启丢弃半条内存记录和票据，完整持久记录保留；重新开始传输会跳过已验证的完整记录。传输完成表示记录已复制，不代表所有 commit 已接纳或工作树已安装。

每个 Server 最多 4 份下载快照，总计 256 MiB；上传最多 32 条缓冲/完成回执，总保留预算 256 MiB，客户端完成后关闭回执。记录继承 Core 的 64 MiB blob、4 MiB manifest 和图预算。票据 15 分钟过期，撤权时清除该代缓存。本机同时一个远端任务，最长 15 分钟；任务状态最多 64 条，保留一小时且只在当前进程有效。浏览器只保留任务编号，丢响应先查询原任务；进程重启或任务过期后，可明确开始新传输并复用完整记录。安装仍使用独立的持久原操作，不因网络传输重启产生重复作者效果。

`test_remote_sync.py` 纳入原 check/package，覆盖三个真实程序、离线冲突与无关对象前进、部分副本、同块重试/重启、分块撤权与重新授权、其他所有者不受影响、持久配对、旧授权不自动扩大、重定向和响应预算。`test_remote_sync_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` 使用明确配置的 Playwright 做实际 UI 验收。已验的回环 HTTP/无头浏览器不替代真实跨机 HTTPS、长时高延迟、真实网络故障、恶意 transport 或其他 OS 的验收。


## 五档正文的托管授权

五档共用原生 SourceEditor 与 WritingController，同一份源码和撤销栈。V02 对原源码做语法/标题装饰，V03 在准确当前构造处展开同一个编辑器，V04 只在最终编译树和官方 AST 共同证明唯一来源的文字、空内容和结构上编辑，V05 无正文修改入口。标题、副标题、格式、列表、普通表格、公式、代码和链接控件生成原生补丁；它们仍是草稿，只有既有 Save 事务能提交。模式切换不写文件；重新读取同一文档且保留原稿时保留 Undo，切换文档或退出时按原草稿流程处理。

`editor/map` 和 `editor/render` 登记本会话准确 source/serial；`editor/edit` 必须消费已登记的同一 source/serial/hash。旧 serial、同 serial 不同正文及回到同字节的旧地址均拒绝。服务端在异步检查后再次验证当前 actor、会话代次和准确稿；富编辑提案还要求文档 write。只读用户得到无写入标志的地图/编译树，不能调用编辑接口。前端再次检查当前编辑器、来源和 serial；撤权时销毁原编辑器、控件对话框、预览及本页缓存，迟到结果不能重新安装它们。

编译只使用分别获权的明确 inputs，交付前还检查依赖字节未变化。只有当前主文档的来源范围可用于编辑，返回路径为资源 ID；include 来源保持只读并移除原路径与原源范围，metadata 不经正文展示接口转交其他消费者。前端与本地编辑共享 DOM 元素、属性、图片和样式允许列表，不注入作者 HTML。随程序分发的离线字体在同源加载；失败显示原因并保留输入。编译仍受单活动任务预算控制；手动只读预览可短暂等待忙碌的编译器，保存不自动重发。

`test_hosted_editor.py` 补验准确稿/ABA、跨用户与只读拒绝、被隐藏或缺失的编译依赖、虚拟来源与 include 只读，以及编译中撤权/新稿使迟到结果失效。浏览器验收入口为 `test_hosted_browser.mjs`，覆盖五档、中文/非 BMP 输入与 Undo、原保存冲突/丢响应/断线、重开输入保护和撤权后的迟到富编辑。自动化键盘输入不冒真实操作系统 IME 验收。字段、批注与库样式的独立授权见下文。


## 窄字段与获权范围 Query

每个对象可单独配置 `fields`：例如 `"fields": {"tasks/status": {"read": true, "write": true}}`。键是已安装 Schema 中的永久 FieldId；字段 write 必须同时有该字段 read，但不要求正文 read。完整正文权限也不自动授予这些结构化接口或完整同步。`GET fields` 只列明确授权的对象 ID / FieldId；`fields/read`、`fields/write`、`fields/operation`、`fields/settle` 均位于 `/api/hosted/` 下，并逐次检查对象和字段授权。

WebUI 的“获权字段与查询”使用已有原生值表单，支持枚举、文本、数组、记录等已限定形状。字段输入在本标签页保留，包括尚未合法的表单文字；查询、切换或重载不把它替换为旧合法值。重新读取后保留输入并要求核对当前字段。保存前持久保留操作编号、原依据与值；丢响应或重载先查询原操作，即使当前字段暂时无法读取，原操作核对仍可使用。未登记 / 未提交则明确结束原请求，之后才能提出新保存；已结束的恢复状态不冒作保存成功。服务端按用户和原字段操作隔离恢复稿，不覆盖该用户的整篇正文草稿。撤权清除字段、查询结果和本页恢复缓存，迟到响应不能重建旧展示。

静态生产器使用官方 AST 检查唯一直接根声明、标准构造绑定、静态键和选中字段；只解码获权字段，不执行正文，也不校验未改动的隐藏字段。任意合法的正文函数、show 和 query 可继续存在。交付只含选中字段定义、值、可写状态和不透明依据，不含源码、范围、原始表达式、Node 控制声明、路径或完整修订。影响所改字段的约束先确定授权依赖，再读取值；缺少其他字段授权或资源 / 关系目标证据时只读，不通过隐藏值产生不同校验结果。当前资源约束及关系写入仍待接入分别授权的生产器。

字段写回重新取得当前源码和限定条件，用原生精确补丁保留所有未选中字节。隐藏正文或无关字段单独变化不使依据过期；获权前值变化，以及获权字段改走又改回的本地保存历史，会拒绝旧依据。当前文件必须有 Core 接纳的保存见证；外部改写或缺失历史不会以相同字节冒充当前。依据绑定用户代次，15 分钟到期，重启失效；持久原操作回执仍可核对。依据只包含选中形状、受影响约束及其实际 validator / provider 依赖；无关注册项变化不使普通字段过期。无法证明依赖闭包的复杂字段明确不可用，不借用整个注册表。

**写入前置**：先通过本地属性面板，或具有独立 `library.schema.write` 授权的托管配置面板，显式安装 Schema，保留同一 Core 私有状态。当前窄字段 writer 必须锁住已安装、已接纳的 `.weftext-schema.json` 直到提交决议；仅使用内置缺省 Schema 或外部未接纳 Schema 时，字段读取可用，但不发可写依据。Query 可使用内置缺省 Schema；已存在却未接纳的外部配置明确不可用。这样不会把“配置文件尚不存在”的观察假作可锁定的条件。字段权限本身不授予安装 Schema。

`POST query` 接受显式 `fields`、可选对象 ID 集合 `resources` 和原 CEL `plan`，以及 `parameters`、`products`、`relations`。省略 resources 时只查询具备请求授权的配置对象交集；类型检查前裁掉未获权字段定义，未请求字段不能通过表达式取回。复用原 CEL 的筛选、选择、排序、分组、数组展开、参数、unionAll、distinct 与相关定义调用。默认选择请求字段；对象行使用部署 ID，结果只读。默认 / 最大 500 行、64 个请求字段和 64 个请求对象；源／配置捕获最多 64 个文件（含 Schema、定义和编译输入）；显式目录最多 64 项，范围标记与控制文件见证另计实体数但共用 32 MiB 输入预算；保存定义的保护池另受 Core 64 个唯一文件上限约束；底层步骤及字节预算仍适用，限定条件不足时显示 partial，不以隐藏对象数量补总数。普通 CEL 和原生日期不需要 Provider 授权；依赖 Provider 的计算遵守下述独立授权和准确版本要求。

服务至多保留 1024 个字段依据，单用户 128 个；每对象最多 64 项明确字段授权；静态形状展开最多 20000 个节点、依赖遍历最多 200000 步，字段依据当前限定为至多 2000 个保留源码版本。`test_hosted_fields.py` 加入字段与正文分权、保留隐藏正文 / 同用户草稿、ABA、外部改写、Schema 见证、原操作重启和撤权 / 再授权，以及 CEL 隐藏字段与范围隔离。`test_hosted_fields_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` 验证原生表单、Query、丢响应、断线输入与重载、显式核对及迟到交付撤权，结果仍归入候选记录。

## Query Provider 与准确版本

用户级 `queryProviders` 分别授予 `providers` 和 `seriesPolicies` 的 ID 集合，省略即为空；正文、字段、定义和 Office 权限都不会隐含授予它们。`GET query/catalog` 仅提供获权项的安全描述、准确指纹与不可用状态，不交付 Provider 数据表、整个 Schema 或服务器路径。周期策略需要独立授权，获权历法和时区不会自动开放所有周期策略。

直接 Query 和正文内嵌运行都传入显式的 `providers`、`seriesPolicies` 映射：键为获权 ID，值为目录返回的指纹，各最多 64 项，默认均为空。指纹覆盖完整 Provider 或策略记录，包括实现标识、ABI、数据和预算。当前只使用已支持的可信内置实现；这不是动态插件安装或运行时构建摘要证明。浏览器保留本次选择及指纹；同一 ID 的数据改变后，旧选择失效并要求明确重新选择，不自动采用最新版。

依赖发现复用原 CEL 语法树，按字面值、参数形状、默认值和被调用定义的原 Schema 基线确定静态需求；动态值只在明确选择的获权子集中计算。参数的命名 Shape 仅开放本次获权字段所引用的定义，或与产品公开内置定义完全一致的形状及其依赖；不能借参数验证探测其他私有枚举或定义。准备或编辑定义及保存源码仍由原 Core 操作处理，不新增 Provider 请求账本，也不把整个获权注册表交给静态校验。current / pinned 定义继续使用各自原基线；准确版本缺失、覆盖范围不足或基线不兼容时明确不可用或 partial，不能回退到最新版本。无结果行的查询也检查与行无关的实际 Provider 调用，避免将不可计算的条件冒充完整零结果。

冻结 Query 保留原结果及实际依赖，Office 继续按原 `queryId` 准备和交付原字节；后来的配置变化不重算既有输出。Excel 结构附表保留获权字段形状和准确结果，不携带内部 Provider 数据表、执行注册表或仅供参数校验的定义。交付仍重新检查用户授权代次和 Provider / 周期策略权限；撤权后再授权不会复活旧结果或下载。`test_hosted_providers.py` 纳入产品 check/package；`test_hosted_providers_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` 检查浏览器选择、迟到结果和草稿保护。实际通过范围仍由候选记录确定。

## 明确目录与回收站 Query 范围

部署配置 `directories` 将目录编号绑定到 `path`、`title`、准确的 `reference` 和 `lifecycle`（默认 `live`）。路径仅在服务端使用，引用须为原 Core 已接纳的 NodeRef 或 DirRef；库根也须通过原目录接纳操作获得 DirRef，不能只凭工作区标记称为稳定目录。用户 `queryScopes` 分别授权 `directories` 编号集合与 `trash` 布尔值，默认空集合及 false。目录权限不授予后代正文、字段、语义产品、保存定义或 Office 权限。目录项仅返回获权编号、标题、引用、类型、生命周期及可用状态，不返回物理路径、隐藏子项或数量。

查询沿用 `plan.scope.directory/descendants/lifecycle`。目录选择为获权配置编号或其稳定引用，保存定义时规范化为原生稳定引用。current/pinned 调用、union 和 related 的实际叶查询分别核对目录与生命周期；配置生命周期须匹配，过滤依据当前真实树，live 根不是 Trash 原历史位置的替代过滤器。未知、隐藏或失去资格的范围不能产生伪完整零结果，即使 `resources: []` 也先核范围。Core 只核明确路径的真实对象、出生接纳、原移动／回收事实、当前声明、规则和 Pending 边界，不扫描隐藏子树；复制标记或另一对象占用旧位置不能延续原身份。执行结束后复检原见证与授权。

保存含范围的定义沿用原 Core 保存与依赖保护，不另建账本。新库在原初始化事务中创建默认规则文件；从未安装规则文件的旧库仍可只读查询，但保存这类定义前须由库所有者显式安装规则文件。查询准备不会隐式创建它，也不能将一次“不存在”观察冒作持续保护。浏览器的顶层范围控件只在明确应用时改变临时计划，不重写嵌套分支或作者正文；未完成输入、失效旧选择和个人 Draft 保留，迟到结果不覆盖新选择。

Trash 文件槽须显式配置准确路径、`lifecycle: "trash"` 和 `expectedNode: {workspaceId, nodeId}`，并保持 `editable/importable` 为 false。它的字段／产品读取还需独立 `queryScopes.trash`；这些槽不开放正文编辑、历史恢复、字段写入、批注写入、建议接受或导入，本入口不执行恢复或清除。部署者在实际回收后更新明确槽位；未指定目录的 Trash 查询也只使用获权配置对象，不是全回收站自动浏览。恢复后旧资格失效。结果只读，Office 使用原冻结结果并在交付时复检范围权限与授权代次，重新授权不复活旧工件。

Trash 槽可以把准确登记的图片、辅助 Typst 等资源列为显式 inputs。每个资源另须 read，且保持与所属 Trash 笔记一致的准确所有者和 lifecycle；不能假标为 live 绕过资格。若原登记叶确实缺失，且 manifest 已接纳、资源原出生证明有效，受影响行的语义产品为 unavailable，覆盖为 partial；静态字段仍可查询。所有者或 manifest 身份缺失、对象替换、撤权及预算错误仍拒绝资格。恢复笔记使旧 Trash 资格失效，Query 和媒体读取本身不恢复对象。

`scripts/test_hosted_scopes.py` 纳入产品检查入口，后台浏览器另用 `scripts/test_hosted_scopes_browser.mjs`。实际通过范围随候选记录，不由此说明建立验收结论。

## 目录接纳与标签注册表

`users.<id>.navigation` 默认拒绝，分别配置 `directories: [已配置目录 ID]`、`admitDirectories: [接纳槽 ID]`、`tags: {read, create, rename, move}`。目录读取独立于 queryScopes.directories，不授予子笔记正文、字段或 Query 范围；标签 create／rename／move 各自独立，且必须同时具备注册表 read。`Config.navigation.directoryAdmissions.<slot> = {path, title}` 固定服务端目标；空 path 明确表示库根，非空路径须为库内普通相对路径，title 必有。客户端只选择 slot，不接受 root 或 path。目录及接纳授权各最多 64 项。

`GET /api/hosted/navigation/registry` 只投影获权配置目录和接纳槽；标签注册表只在 tags.read 时读取。目录用原 `qualified_query_scope_at` 和 witness 验准确稳定身份，不使用全库发现。目录返回 complete／partial，标签返回 complete／partial／unavailable；标签不完整时不提供写 basis。公开对象只带选择器、标题、稳定 reference、可用状态及原获权条目，不交付宿主路径、未获权目录或隐藏计数。标签写 basis 为 actor／epoch／准确注册表绑定的私有有界缓存：单份最多 4 MiB、共 32 MiB、服务最多 128 份、单用户 16 份、15 分钟到期。

`navigation/registry/prepare|commit|operation|settle` 复用 Core 原 `HostedNavigationIntent`、DirectoryId／TagId 分配和 actor feature 日志。prepare 接受原 operationId 及 admitDirectory／createTag／renameTag／moveTag 意图；Tag 意图还消费原 basis，rename／move 固定原 tag reference。安全预览剔除物理路径。提交仅消费原 planId、actor、授权代次、配置绑定及未过期决议；重试、重启和丢响应不再分配身份，未知请求可结束以阻止迟到执行。复制目录标记或用同字节替换注册表不能获得原身份。

`navigation/registry/resolve` 区分已配置目录、标签和刚接纳目录。一般读者须明确获权配置的 directory reference；刚接纳目录仅原用户凭原 committed operationId、原 slot 和准确 navigationRef 继续解析，仍核当前权限、代次、配置及实际对象。不会从已占用路径建新 owner ledger 或补授权给其他用户。`scripts/test_hosted_navigation_registry.py` 提供目录／Query 分权、库根接纳、跨用户回执、原操作重启、撤权、未知 fence 和同名／复制身份反例；真实 HTTP／浏览器通过范围仍随候选记录。

### 多级标签授权

该通路已接入 0.6.3-rc.1 工作树，整版验收中。层级统一重构原导航操作及冻结查询通路，不另设一组树写接口。`navigation.tags` 使用默认拒绝的独立 `move`；既有 create／rename 含义不扩大。创建时选择初始父级需要 read+create 和完整合格树的原 basis；改变既有父级需要 read+move，改名只需 read+rename。逻辑意图的 `createTag.parentRef` 和 `moveTag.parentRef` 均明确提供、值为完整 TagRef 或 null，move 还绑定原标签；只接受获权工作区的稳定选择，不接受宿主路径、客户端树或整表 after。服务端依据原 basis 生成改动；所有注册表写路径按真实变更检查操作权，包括通用写入／恢复入口，不能以 rename 顺便修改 parent。

登记投影在 tags.read 下增加准确父级和树完整性信息，仍不披露宿主路径或隐藏笔记数量。只有完整有效且已登记的森林才提供写 basis；所用注册表版本、原文件身份、actor／epoch、TTL／预算与现有 prepare/commit/operation/settle 统一。撤权、配置变化、旧 basis 和未知操作沿原 fence／恢复；重试不重新分配 ID。move 预览和提交重新检查本人权限，无子树权限继承。

创建时可用 `assignTo` 将首次直接赋值纳入同一 Core Plan；它绑定准确获权资源、字段及原读取 basis，并独立核字段读写权、类型、源和当前资格。标签 create 不授字段 writer，字段可写也不授注册表 create／move；已有未保存输入或变化依据不能由注册表操作隐式覆盖。准备／提交／恢复保持同一原身份与完整前后像。

Query 静态读取分析识别 FEATURES §5.6 的结构谓词及完整 callee 闭包，再要求 tags.read 和原笔记／字段／Query 范围权；Core 同次 cut 提供可信树，零候选也校验根和表依赖。精确 TagRef 比较不新增 tags.read；名称／路径显示另行获权，不因此阻止纯精确结果。完整树不补完整笔记范围，缓存／分页及迟到响应仍重验当前 audience，撤权不通过旧树继续交付。本范围不新增 delete／hide 权限或假装这些操作可用；原 alpha.3 的扁平通过记录不自动覆盖新层级的整版验收。

## 高级 Query、定义与正文内嵌

每对象的 `query` 独立设置 `definitions`、`definitionHistory`、`products` 与 `relations`，默认不授权。products 的键为 `title`、`body`、`headings`、`types`、`self`，relations 是永久 RelationId 数组；正文 read 不隐含这些权限。未获权的语义产品不会因同名 CEL 表达式或原始源码可读而交付。字段值、关系断言、目标类型与实际编译输入均在求值前确定所需授权，不通过隐藏正文执行推导窄字段值。

`GET query/catalog` 列出获权配置资源、字段 / 产品 / 关系与已接纳的当前定义。某个定义来源外部改写或暂不可用时，目录显式标记 partial 和对应不可用对象，不阻塞其他获权对象。`POST query/definitions` 接受 resource，可选 selection 为原 `{"mode":"current","key":"…"}` 或 `{"mode":"pinned","key":"…","revision":"…"}`。读取固定版本另需 definitionHistory；不存在的版本不会替换为 latest。调用使用原 QueryPlan 的 `advanced.call`，HTTP owner 为配置资源 ID；原生定义里的稳定 owner 引用只在明确配置的获权对象中解析。每个被调用定义使用自己的准确 Schema 基线，不以调用方当前别名重解。服务器不向定义用户交付整篇源码或完整 Schema。

查询捕获明确授权闭包内的已接纳文件和配置；读取见证由本次请求持有，失败、取消或完成后释放，不积存在 Core 的写依据存储。可见输入副本、原始读见证、目录／节点资格见证、上下文和重复编译输入共用累计 32 MiB 捕获预算；执行时保留的输入、见证及逐项产出的语义产品仍受 32 MiB 预算约束，超限明确失败。求值与原生语义编译后复核当前授权、配置绑定及准确输入。正文、标题和标题树来自同一次原生 Html 产品，不用源码字符串冒渲染正文。关系遍历使用获权字段与可信身份 / 类型投影；参数、终点列名和 CEL 局部变量不会因为同名而取得额外字段或产品权限。冻结结果仍由原 queryId 标识，Office 使用同一份行序、列序和值；导出和每次分块交付还需完整实际依赖的 Office 权限及当前 Query 授权。

正文 `query/embedded/discover`、`run`、`edit` 复用官方 AST、原执行器和当前编辑会话。准确 source / serial / hash 防止迟到提案或同字节 ABA 重新取得写资格。构建器或源码替换只产生草稿补丁，共用五档正文和 Undo；只有原 Save 提交。只读用户无修改入口，源可读也不自动允许执行定义的数据依赖。旧结果、对象和用户切换及撤权均使迟到交付失效。

新建保存定义使用 `POST query/definition/prepare`：请求固定 resource、sessionId、baseId、source / serial / sourceHash、operationId、label 与原 QueryPlan，不接受客户端 key、路径或 Schema binding。正文 read / write 与定义读取均须获权；实际字段、产品、关系及 current / pinned 被调用定义另行校核。Core 分配 key 并一次持久保存初始已接纳 Schema 基线、完整原提案和身份；准备只返回准确 UTF16 草稿补丁，不执行结果或保存正文。界面先保留原编号和请求，再把补丁插入同一正文与 Undo。调用中的托管资源选择转换为原生稳定 Node 引用后保存。

相同原请求重发核对同一 key / 基线 / 补丁，换内容同编号冲突；旧会话已结束时，恢复结果只能用于比较，不能自动贴回当前稿。`query/definition/operation` 和 `settle` 接受 resource / operationId，`reservations` 接受当前 sessionId；重开可找到同用户、同真实所有者的待保存预约。明确结束会持久阻止迟到预约，已经安装或等待恢复的保存不能伪装成取消。Undo、删去定义或保存不含定义的正文不会取消预约。旧提案替换当前草稿须明确比较双方稿，并仍可撤销。

普通 `save` 可携带明确的 `definitionReservations` 原编号集合；该集合和完整源码固定在原保存指纹中。只接纳本次准确 after 中真实存在、属于同用户及同所有者的已预约 key。裸源码中的自造 key、另一个所有者或另一个用户的预约不会成为身份依据；合法且保留 Node 身份、但没有可接纳定义的作者源码仍可普通保存；全文语法未完整、无法确认 Node 身份时沿用原 Core 拒写并保留完整个人草稿，修复后可继续使用原预约。首次接纳前按原基线重新校核当前 Schema 兼容性、实际被调用定义和当前权限；非写入所有者的依赖保持原快照对应的保护句柄直到保存决定，所有者由原 Core 保存保护。一次保存可包含至多 32 个预约；同用户每所有者至多 128 个待处理预约。多个提案的快照须逐一核对一致才能去重，总闭包至多 64 个文件、64 MiB。界面查询原状态后移除已接纳或作废的保存编号；恢复目录中的未应用预约不会自动加入保存。丢保存响应仍查询原保存编号，不创建新操作来重放。接纳后的身份由原提交历史证明，后来撤销、保存和重做不需要重新分配 key。

所有托管恢复缓存按部署、用户和服务端授权代次隔离；同代次的原操作可恢复，旧代次不会恢复受保护正文、预览或查询输入。浏览器拒绝删除缓存时仍清除本页展示与内存，并明确提示清理站点数据，不把删除失败报告为缓存已清空。

`test_hosted_query.py` 通过真实服务验证参数 / 组合 / 去重、最小字段和语义授权、准确 current / pinned 定义、关系引用、内嵌草稿及冻结输出撤权。`test_hosted_definition.py` 检查新定义只改草稿、原预约重试 / 重启、多个定义与同稿调用、Undo / 保存 / Redo、真假身份、结束预约、未完稿保护与修复、改绑对象与保存前依赖 / 权限变化。两个 HTTP 入口均纳入产品 check/package；`test_hosted_definition_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` 验证同一编辑器的预约、显式保存、恢复与撤权。测试是否已通过及实际浏览器环境仍以候选验收记录为准，不以文档或接口存在代替运行证据。

## 库配置与原生样式

每个用户的 `library` 分别配置 `schema`、`settings`、`rules`、`style` 的 `{read, write}`，以及独立 `styleUse`，省略均拒绝。各角色 write 要求同角色 read；配置权限不授予笔记正文、字段或资源权限。`GET /api/hosted/library` 只列获权角色；`library/read` 返回角色、准确源码和不透明 basis，客户端不能选择路径。Schema 未安装时可读取内置默认值并明确安装；规则使用现有 `.weftext-rules` 格式，缺失的旧库规则须显式安装，已提交配置丢失不会静默修复。

`library/prepare|commit|operation|settle` 沿用原 Core 操作编号与计划。配置和样式草稿独立于正文 Draft；准备先保留完整原请求，展示 before/after，再明确提交。未知结果查询或重试同一原编号，服务重启后恢复原计划；结束未安装请求阻止迟到提交。期间输入变化使旧提案只能核对，不能覆盖新输入。配置 JSON 保留未知成员；原生样式使用与本地相同的控件和源码入口，复杂源码仍可完整编辑。style.write 允许修改该样式的启用状态和原设置选择器；仅 settings.write 不能借默认样式设置绕过样式权限。

作者同级导航顺序保存在同一设置的 `navigationOrders`（CORE §2.4），以稳定父及直接子级引用表达，不改变目录结构。独立 `library.reorderNodes: true` 要求同有 settings.read/write；省略拒绝。`library/order/read` 另需准确父引用对应的 `navigation.directories`，只交付该父当前直接子级名称／引用和完整性，不暴露本机路径或正文。友好控件只改变当前设置草稿，再走同一原准备／确认。通用 settings 源码准备按实际前后差异逐父核重排及目录披露权，普通 settings.write 不能借完整 JSON 绕过。Core 对所有写口核完整当前成员和整设置 CAS；原计划提交、状态及结束操作重验当前资格，成员权撤销也清除原预览，旧计划不复活权限。配置读取只授予该角色源码，不授予所引用笔记的正文／字段或媒体读取权。

预览、富编辑呈现、实际语义 Query、校对与 Office 编译共用一次捕获的样式环境。启用库样式时须有 styleUse；静态字段查询不执行或读取样式。配置 `styleInputs` 为管理员明确指定的文件或资源 ID，每个输入另须 read。文件输入用于 Office 还须 export / download；资源输入另须自己的资源 download。编译只使用冻结输入，交付前重验原样式、设置、身份、依赖和当前授权。styleUse 不授予样式源码读取；内部编译入口与未授权的真实模块路径隔离，路径和源码诊断不向客户端交付。

样式变更刷新当前呈现和之后的查询，不重写正文、重置 Undo 或丢弃未决保存。已经完成的 Office 工件保留原字节和摘要，不因后来样式变化重算；交付仍检查原输入配置、styleUse 和当前权限代次。校对批注计划保留原扫描使用的样式依赖，重启或样式变化不能把旧扫描当作当前结果提交。`test_hosted_library.py` 纳入产品 check/package；浏览器入口为 `test_hosted_library_browser.mjs EXE NEW_EVIDENCE_DIRECTORY`。具体通过范围和未验项以候选验收记录为准。

## 文献生成与出版字段权

每个 Work 对象另配置 `citations: {use, export, download}`，默认 false；使用文献还须该对象 `fields["library/publication"].read`。这些能力不由正文 read、library.schema.read、Office、PDF 或原生包权限推定；出版字段读取不要求整篇 Work 正文 read。`GET /api/hosted/citations` 只列独立获权且经原身份／Work 类型资格核对的配置对象。`citations/propose` 消费当前 session／sourceHash／serial、准确 key 和获权 Work 选择器，给出原生静态 `weftext.citations` 声明及引用的草稿补丁；须当前正文 write 和 editable，不执行第二保存协议。

预览、Query 和输出的编译从唯一直接静态原生声明中的稳定 Work 引用生成虚拟 `weftext.citations.bib`，只使用获权 publication 切片，不扫描标题或整个库、不向工件交付 Work 私有正文或整篇正文摘要。内部保留原 Node／文件身份、已保存版本、源与 Schema／规则 witness；对当前主稿里的同一 Work 使用准确草稿字段。生成槽和显式真实输入重名时拒绝，既有资源不被覆盖。每次生成最多 64 个明确 Work、32 MiB 依据；异步编译和工件完成前复核原字段、原输入、配置和权限。工件确认／分块交付再查全部实际引用的 citations.export／download；冻结后的 bytes 不因以后改稿重算。

当前候选接入原生引用形式与选择器、关系分类、引用键和脚注等有界增强；具体支持与限制见 MVP。文献／输出 HTTP 入口为 `scripts/test_hosted_citations_outputs.py`，新增写作入口为 `scripts/test_v061_authoring.py`；脚本及文档不替代浏览器或真实 Office 验收。

## 准确资源与媒体目标

`resources.<assetId>` 配置一个已登记资源：`owner` 为文件 ID，另有 `expectedOwner: {workspaceId, nodeId}`、相对 `path`、显示 `title`、稳定 `reference` 和 `lifecycle`（live 或 trash）。资源 ID 与文件 ID 不重叠。`users.<id>.resources.<assetId>` 分别授予 read / download，省略均为 false，download 要求 read。所属笔记正文、批注与 Office 权限不隐含资源权限。`files.<id>.inputs` 和 `styleInputs` 可显式列入这些资源 ID；不通过目录扫描补齐兄弟输入，也不自动配置导入产生的资源。

`GET /api/hosted/media` 只列获权配置项，并报告 complete / partial。`media/resolve` 接收获权的稳定 ResourceRef 和可选准确版本，PDF 片段按实际页数检查。读取使用 media/read、media/chunk；复制原件使用独立的 media/download/prepare、media/download/chunk。票据绑定用户、授权代次、用途、原资源身份、配置所有者和准确 portable revision，分块至多 64 KiB，有效期 15 分钟；服务至多保留 32 张、单用户 8 张，共用 32 MiB 原始依据预算。撤权、重启或替换对象不会让旧票据续取新版本。

浏览器核对完整长度和摘要后，使用共享媒体查看器显示。PNG/JPEG/GIF/WebP、有界普通 PDF、WAV 和支持的 MP4 从真实字节取得事实；不支持的文件可以添加整资源批注，但不会虚构区域。媒体批注新建和重新定位要求资源 read，加所属笔记的批注 read/write/target，不要求正文 read。图片／PDF 区域及音视频区间按选中版本核对实际尺寸、页号和时基。原计划保留 Core 资源依据，重启后仍校验准确版本；纯讨论操作不读取目标字节，Trash 资源只读。媒体选择和稳定资源链接保留当前正文草稿；复制原件由独立 download 权限控制。

`scripts/test_hosted_media.py` 纳入 check/package，覆盖真实 HTTP 交付、媒体事实、原批注计划、明确编译输入、冻结输出及 Trash 依赖。浏览器验收另存证据目录。实际通过范围和限制以候选验收记录为准；配置资源读取不代表原生 Office 兼容性已经验收。

## 获权资源集合与原件导入

部署者可配置 `resourceCollections.<id> = {owner, expectedOwner, lifecycle, title}`，绑定一个文件 ID、准确 NodeRef 与 live／trash 生命周期。用户的同名 `resourceCollections` 分别授予 `{read, download, import}`，省略拒绝；download 要求 read，import 可以独立于资源 read 和正文权限，Trash 不允许 import。这是预先授予该准确所有者当前和未来已登记资源的集合权限，导入不会修改策略。原生 `weftext.resources.json` 仍是成员与位置的唯一依据，不扫描同目录文件补齐资源。

尚未创建的 DOCX 导入槽位可用互斥的 `fromImportSlot: true` 代替 `expectedOwner`，所属文件须是 live、importable 的原槽位。导入前集合不可用；导入后 Core 从已提交的原 ImportDocx/CreateTree 计划、原出生和实际文件身份推导准确所有者，仍核当前声明和位置。当前路径上的任意新笔记、复制标记或同字节替换不能借用这个预授权。该模式只覆盖原导入路径；经 Core 移动并改部署路径时须显式配置 `expectedOwner`。管理员可提前将该集合的 `/*` 放入导入文件的 inputs，另授集合 read／download，使原生预览与 Office 使用真实导入资源；这不会在导入时补赋权限。

`GET media` 的 `collections` 只列至少一项获权集合；其中 `status` 和总 coverage 如实报告不可用项。可读成员使用 `collectionId/ResourceId` 作为托管 asset ID，稳定 ResourceRef 保持原格式。Core 先核准确所有者、当前已接纳的 manifest 与资源出生，再读取明确选中的叶；未知、重复或未接纳记录保持原件并计为不完整。manifest 上限 2 MiB／64 条，集合依据计入原 32 MiB 消费预算。已提交 manifest 被删除或外部修改不能伪装成新空集合；真正从未初始化的空集合只产生只读的缺席见证，不能当成持久写依据。

`media/import/prepare|commit|operation|settle` 只接收集合 ID、文件名、本次上传字节和原 operationId／planId。原件至多 16 MiB，路由 JSON 限额 24 MiB。prepare 展示本次原件的名称、长度和实际媒体事实，复用原 Core ImportResource 计划；明确 commit 才以 create-only 方式同时安装原件和登记。原完整请求先留在浏览器，再发送准备；丢响应或重启查询同一编号，改意图不能重用编号，明确结束未知请求持久阻止迟到提交。原预览有效期 15 分钟；所有者、部署绑定或授权代次变化使旧计划不可提交，重新授权不复活它。import-only 用户可取得自己原操作产生的引用，但不能借回执读取集合或其他原件。

`files.<id>.inputs` 与 `styleInputs` 可明确选择单个成员或 `collectionId/*`。每次执行冻结实际成员和原集合版本，逐项校验资源 read；用于 Office 的原资源还须 download。`*` 遇到未识别成员不能报告完整编译，静态字段 Query 不展开无关集合。预览、语义 Query、校对与样式共用这一捕获；成员或已接纳 manifest 版本变化会拒绝旧执行／旧校对计划。已生成 Office 工件仍交付原字节，只检查原授权和配置，不扩展后来成员。准确 Trash 集合只读，已登记但确实缺失的输入令语义 Query 部分可用。`scripts/test_hosted_resource_collections.py` 纳入 check/package；具体测试结果沿用候选记录。

## 创建、模板与笔记结构操作

托管结构入口复用 Core 原有新建、改名、移动、完整复制、模板、回收与恢复计划。部署者配置 `structureSlots.<id> = {parent, name, types?}`；parent 指向已配置且实际纳管的目录 ID，name 固定目标名称。同 ID 的 files 项必须准确指向 `PARENT/NAME/NAME.typ`，且为 live、editable、非 importable。客户端仅提交这些选择器，不提交服务器路径。预留位置由原已提交的托管生产操作、目标槽位和当前物理身份共同确认；普通本地新建、原始复制或后来占用相同路径的其他对象不会自动获得预授权。

用户的 `structure.sources.<fileId>` 分别设置 rename、move、trash、restore、copy、template、treeRead、treeExport；`structure.destinations.<slotId>` 分别设置 create、receive，全部默认拒绝。正文 read/write 不隐含结构权限，创建也不自动获得新对象的正文、字段、批注或资源权限。改名必须同父目录，移动保持原名称；复制和注册模板另需明确的整个源子树 treeRead/treeExport 及 Schema read。这项完整子树授权包括其未单列配置的成员，必须在首次读取之前成立；实现不扫描其他库对象来补全传输引用。

`GET structure` 只列获权来源、位置、模板和可用状态。`structure/prepare|commit|operation|settle` 使用原 resource、operationId 和 planId。create/copy/template 以目标槽位为请求锚；rename/move/trash/restore 以原来源为锚。恢复须提供该用户原回收操作的准确外部编号，不提供任意回收站目录枚举。预览保留原意图、身份映射及明确损失；commit 的 acceptLosses 不得替代未展示的损失确认。未知结果查询和结束沿用原持久操作，重启不另建编号，已结束请求不能迟到生效。原计划按当前授权代次、配置和实际来源、成员、父目录及 Schema 复验。

create 可带 `capture = {title?, body?, bodyMode?, types?, fields?}`。默认 text 将正文与标题按字面原生编码；markup 只接受可证明不会执行任意代码或追加身份／字段声明的排版内容。types 必须来自该槽位的明确允许集；fields 使用准确 FieldId 和原生值，显式值、必填值与实际原生默认值分别要求字段 read/write，并按原 Schema、提供者及约束验证。没有值的可选字段不因 absent 默认而要求写权。Core 分配唯一新身份，不由表单提供。内置 native-note 模板使用字面 title/body；`structureTemplates.<id> = {source, templateId}` 则选择已登记的准确原生模板，固定原设置、来源、参数槽和完整子树。

成功回执中的 result.resource 指向准确目标槽位；canOpen 另核当前独立正文读取权。WebUI 仅在用户明确选择时打开该入口。当前编辑器受到移动或回收影响时，保留个人草稿和撤销，停止把旧会话当作新位置；结果回执不替换后来输入，也不自动换 Base。复制产生独立身份，改名、移动与恢复继续原身份。该接口接通与实际测试范围沿用同一候选记录；完整 v0.6 的剩余消费者仍须单独完成。

结构创建的资源集合可预先使用 `fromStructureSlot: true`，owner 指向准确槽位；它与 expectedOwner、fromImportSlot 互斥。集合所有者仍须通过原托管结构生产记录及当前物理身份证明，不能读取未由该槽位产生的占位对象。集合 read/download/import 继续分别授权，原生资源登记仍是成员依据；`files.<id>.inputs` 可预选该集合，创建成功后直接使用原有媒体与编译消费者。结构请求的可选 inputs 列出额外独立获权的文件选择器，固定它们的原依据以验证明确引用，不自动扫描或展开全库。

## 原生包与文献资料导入

`contentImportSlots.<id> = {parent, name, title, inputs}` 固定已配置、已接纳的 live 目录下尚不存在的目的位置。用户的 `contentImports.<id>` 分别授予 `imports: {native, bibliography}`、沿用文档权限的 `members` 和沿用集合权限的 `resources`，缺项均拒绝。准备导入还要求 `library.schema.read`。导入权本身不开放结果正文或资源，producer 不修改策略。最多 64 个槽；客户端只提交槽 ID、格式和本次原件，不传目的路径或身份。

`GET /api/hosted/import` 列出获权槽；`inspect`、`prepare`、`commit`、`operation`、`settle`、`members` 的 POST 路由沿用原操作生命周期。Prepare 接收 `{slot, operationId, format, bytesBase64}`。Native ZIP、BibTeX、CSL JSON、RIS 复用已有 Core 导入 producer，保留上传原件及原生 transfer map；需要时由 commit 明确接受已报告损失。当前文献 writer 创建无冲突的新 Work，不刷新既有 Work、不解决上传内部的出版字段冲突，也不臆造 typed 出版引用的映射。检查会报告这些限制；未解决的输入不能静默部分导入。引用键编辑增强仍单列。

Native 的闭合 Tags 角色、全源 TagRef 映射和 Query 准确 Schema 重绑遵守 CORE §2。包含新标签创建的上传／计划另需 navigation.tags.read+create，实际初始化 Schema 另需 library.schema.write；没有这些副作用的普通导入不额外索取相应写权。检查上传、准备、原操作状态、提交及恢复均核真实共享副作用，不由导入槽、正文或资源权继承。缺包内准确 Query Schema 历史以 schemaMappingRequired 拒绝，不能用 latest 换基线；旧未决计划也不借新调用绕过当前写权。

已知原生导入损失使用固定说明，例如生成文献库只保留在原件中、不安装到作者资料树；不会用原始内部路径作为说明。保留的出版字段即使尚不足以生成完整引用，也须核对能够准确比较的标准字段；无法可靠比较的复杂字段明确拒绝，不能跳过检查后覆盖外部改动。

结果选择符为 `slot/NodeId`，登记资源为 `slot/NodeId/ResourceId`。成员来自原已提交 CreateTree 和 birth 证据，再检查当前身份、声明及位置。不会扫描目录或新建成员账本补齐缺项；原成员失去资格时报告不完整。明确编译输入可选择 `slot/*` 或单个成员的 `/*`，每份实际使用的正文和资源仍须独立获权。旧路径上的无关对象不能继承槽权限，槽也不会自动跟随移出配置目的位置的对象。

运营者可用 `structure.sources["slot/*"]` 单独预授原结构操作，沿用配置笔记的源操作及树读取/导出权限；新操作必须具有原导入成员证据及当前资格。成员写权限不授予结构操作或目的权限。成功移动或回收后，原操作状态与恢复继续执行各自的原请求检查。

上传限 16 MiB，路由 JSON 限 24 MiB，原展开计划预算继续适用。预览绑定准确原件、目的位置、用户代次及 15 分钟决策期限。丢响应和重启查询同一原操作，表单后来的输入不能使旧计划重新有效。未知请求须先持久结束，才能开始替代请求。正文、字段、资源、编译和输出继续走分别授权的消费者；真实 HTTP 和浏览器通过范围按候选记录。

## 完整恢复资料备份

管理员显式配置 `ownerBackupDirectory`，指向库和私有状态之外已存在的绝对目录。用户独立的 `ownerBackup: {read, export, download}` 默认拒绝；export 要求 read，download 要求前两项。其范围包含所选库的 raw、ignored 文件、Trash、历史，以及各用户与该库相关的私有草稿和原操作记录。原生内容导出、`syncOwner`、普通文档权限、管理员凭据均不隐含此权限。

`GET /api/hosted/owner-backup` 报告可用性，`prepare`、`status`、`commit`、`settle`、`download`、`close` 的 POST 路由保留原 `requestId` 和 Core BackupLibrary 计划。客户端不能选择 source、私有状态或输出路径。原 actor record 在捕获前已持有备份 fence，备份不开放为通用 feature 意图。Commit 先验证并编码同一固定预备计划，再执行它，之后交付包含原 `library/`、`private/` 和 v1 `backup.json` 的无损 ZIP。这是完整恢复资料；没有自动跨设备 restore 入口，也不承诺原物理身份在新设备上直接有效。

新私有记录在原格式内保留可信库归属。备份先读取有界归属 metadata，再决定是否读取正文，排除可证属于外库的记录。旧事实归属未知、保留 proposal 混库、缺必要证据或成员发生变化时明确失败并保留原件；不会静默漏项、按当前配置推断历史归属，或先读外库正文再分类。最终输入含原 manifest，限 64 MiB、5,000 个唯一文件／目录；ZIP 上限 80 MiB。

下载交付固定的 64 KiB 分块及长度、摘要，每块重新检查当前用户、代次、root 和独立权限。交付缓存最多 8 件、160 MiB，提交前预留容量，15 分钟或服务重启后失效。关闭或丢失工件仍保留原 committed 操作并报告 `deliveryUnavailable`；status、prepare、重复 commit 均不重新捕获或生成。撤权后重新授予不会复活旧决策和工件。界面明确展示私有记录范围，收完核对摘要才保存。备份自身事务若仍 installing，继续按原恢复处理，不能因输出目录已存在就报告 committed。

## 批注、校对与建议事务

对象的 `annotations` 分别设置 `read`、`write`、`target`、`accept`、`proofread`，省略均为 false。当前审阅入口以批注 read 为基础；批注 write 可独立于正文权限。仅批注读取交付讨论、作者、回复和处理状态，不交付目标摘录、替换文字、源码范围或私有身份。target 允许查看该对象既有批注的目标摘录与准确位置；它不授予整篇正文或源摘要。新建正文目标、正文重新定位及重新确认建议需要正文 read、批注 write 和 target；应用建议还需要正文 write 和 accept。媒体目标使用上面的独立资源权限。校对另外需要正文 read、target、proofread；将结果加入批注还需要批注 write。服务端使用已认证用户归属，不接受客户端冒署作者。

WebUI“批注与校对”有独立的对象选择与草稿。先打开、保存并核对对应正文，再读取批注，可用当前 UTF-16 选区、插入点或整篇笔记创建批注；无法准确映射的建议不能执行。讨论用户可单独回复、解决、重新打开或拒绝建议，不因目标正文单独变化而丢失讨论写入资格。已有批注与回复使用同一 `weftext.annotations.json`，保留未知成员。应用建议经原 Core writer 同时提交准确正文补丁与 accepted 状态，原个人正文草稿仍保留，界面要求重新读取核对。

校对复用实际冻结编译树及原四类规则引擎，按可见正文检测错误字符、错别字、成对符号与敏感词，不能以源码正则扫描替代。编译输入逐项授权，缺失或隐藏依赖失败。每次检查返回原 scanId、命中、定位及覆盖说明；批量加入消费同一用户、对象、依据、选项和原扫描结果，提交前重验实际源与依赖，过期不自动重跑。原目标已存在的规则命中不重复创建，不覆盖既有回复和处理状态；不能准确定位的项保持只读。

个人规则可显式导出和导入，不共用本地用户的规则存储。用户可读取具有独立设置读取权限的共享库处理建议，核对来源、语言／地区、词表与版本后明确采用；这不修改文档或共享默认。规则按原文语言标记或本次明确选择及其范围执行，未知和未检查范围单独显示。扫描、待提交预览和原操作恢复持续核对原共享设置身份、版本及当前授权；词表变化或同字节替换文件也使旧资格失效，不重新捕获新设置冒充原依据。

`review/read`、`review/proofread`、`review/prepare`、`review/commit`、`review/operation` 和 `review/settle` 都在 `/api/hosted/` 下。准备先持久绑定原用户、对象、意图、操作编号和安全预览；用户明确确认原计划后才提交。网络丢响应或服务重启查询同一原操作，不新编号重试。未安装的原请求可持久结束，阻止迟到请求；已安装但待恢复的操作只报告原状态。权限变化使旧依据、扫描及可提交预览失效，重新授权不能复活旧计划。批注恢复输入与正文草稿分开保留；在线撤权清除本页展示和恢复缓存，迟到响应不能恢复旧内容。

批注依据 15 分钟到期，服务最多 128 份、单用户 32 份；扫描最多 32 份、单用户 8 份，和依据共用 64 MiB 保留预算。单批注文件至多 4 MiB、原操作预览至多 2 MiB、每批最多 200 个结果，底层 Core 文件预算仍适用。重启丢弃临时依据与扫描，持久原操作仍可核对。正文目标要求具有 Core 身份和已接纳源码版本的管理笔记；媒体目标使用上面的独立资源资格。目标不可用仍保留获权讨论。`test_hosted_review.py` 检查分别授权、准确目标、回复与未知成员、建议原子性、冲突、重启与撤权及真实编译校对；`test_hosted_review_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` 检查实际选区、讨论、建议、校对、离线输入、丢响应和迟到交付。Office 审阅交换见下文，不能把本地通过视作 Server 通过。


## Office 导入、输出与准确下载

每个文件的 `office` 分别授予 `export`、`download`、`import`，省略为 false；它们与已保存原生源码的 `download` 分开。DOCX 正文输出要求主文件和每个显式文件输入具备正文 read 与 office.export；资源输入则要求自己的资源 read 与 download。带批注或修订还要求该正文的 annotations.read 和 target；不要求 accept，也不因此接受建议。XLSX 使用已经显示的冻结字段 Query，每个实际字段来源须保有字段 read 和 office.export，不要求整篇正文 read。下载另对全部文件来源检查 office.download，对资源输入检查资源 download；正文读写或原生源码下载权不隐含授予这些权限。

WebUI“Office 导入与输出”提供 DOCX 正文／批注／待审文字修订，以及 XLSX 冻结查询结果。正文可选择当前准确草稿；带批注或修订必须先保存并核对准确正文。未能安全定位的目标作为转换损失，不猜测位置；其他笔记／媒体的目标权限不会从当前笔记继承。输出复用本地纯转换器和原生冻结编译，不补读整个资料库。XLSX 保留原列序、所有原查询行和 Cell 状态；可见结构附表只带该次获权字段的限定 Schema。旧结果不因后来改稿或改字段而重跑，完整性和覆盖仍按原结果显示。

用户可上传 DOCX 内容控件模板或 XLSX 命名范围／表格模板，先检查实际槽位，再明确绑定正文、标题、字面文本、准确 FieldId 或冻结 Query。字段绑定要求每个实际来源 field.read 和 office.export；只含字段／字面值的交付不因此要求或提供整篇正文 read。XLSX 可仅输出明确字段快照，也可复用原 Query；不能把正文／标题绑定解释为单元格字段。默认使用模板样式，也可选择源样式并核对映射损失。模板是用户本次提供的原件字节，检查不读取服务器资料或执行 Office。获权正文的准确源码与查询前 12 行供核对；工件使用全量冻结输入，超预算明确失败。

统一 `GET /api/hosted/export` 和 `export/prepare|status|confirm|chunk` 复用原 Office 工件协议，原 office/export 别名保留。新增格式的独立配置如下，均默认拒绝：

| 格式／范围 | 准备及交付权 | 准确冻结输入 |
| --- | --- | --- |
| PDF | 主正文及实际文件输入的 read + outputs.pdf.export；交付另需 outputs.pdf.download，资源依其独立 read／download | 当前 resource、sessionId、baseId、source、sourceHash、serial 和原编译环境；不接模板或批注／修订参数 |
| BibTeX／CSL-JSON／RIS | citationWorks 主文件须 read + outputs.bibliography.export／download；selectedWorks 每个所选对象须 outputs.bibliography.export／download；两者另查每个 Work 的 publication.read + citations.use／export／download | bibliographyScope 显式为 citationWorks（捕获的正文声明）或 selectedWorks（最多 64 个明确 Work ID）；两种来源不混用 |
| native | 管理员 nativeWholeLibraryExport: true 且用户 nativeWholeLibrary.read／export，交付另需 download | savedWholeLibrary 的已保存整库纳管内容和依赖，不是当前笔记或草稿；其中生成文献仍须独立 Work 文献能力 |

原生包包含已保存作者内容、原生依赖和可携的离线文献输入，不包含私有历史、个人草稿或执行日志，不能替代恢复备份。整库权限是显式的完整资料交付能力，不由当前文档／字段／同步权限推出；准备前、工件完成及每块交付持续核对原整库配置和权限。用户界面须清楚标明 savedWholeLibrary，不能把它呈现成当前笔记导出。

DOCX／PDF 可显式选择 `embeddedQueries: [{occurrenceId, queryId}]` 的已经显示结果；每份冻结 Query 记录其原 sessionId／sourceHash／serial／occurrenceId，并与本次准确正文和位置逐项匹配。最多 32 项，重复位置、其他正文的结果或旧 serial 拒绝，不重新运行 Query 或借用别处结果。模板字段、正文内嵌 Query、引用、真实输入及样式能力分别汇入原输出权和 witness 检查；同一 requestId、digest、确认损失及准确分块下载规则适用于全部格式。

`GET office`、`POST office/template` 与 `office/export/prepare|status|confirm|chunk` 都在 `/api/hosted/` 下。原 requestId 和完整冻结请求先保留在本标签页，然后准备工件；确认须匹配原 digest，并明确接受该工件报告的损失。Core 持久登记原 ID 与请求指纹；重复请求返回原工件，换内容冲突。服务重启、到期或撤权丢弃临时工件，原 ID 只返回 expired，不重新生成当前内容。未知原请求可查 notRegistered；失败原请求保持失败。生成完成前检查原权限代次、配置绑定、正文和依赖快照，每个下载块再次检查全部交付权限。确认后的准确字节不因后续编辑改变；浏览器收齐后验证长度和 SHA-256 才交付。

DOCX 导入要求管理员事先配置一个 `importable: true`、`editable: true` 的空槽位，路径固定为 `PARENT/NAME/document/document.typ`（PARENT 可为空，即库根）；例如 `Imported/document/document.typ`。用户须有该对象 read、write 和 office.import。客户端只选择对象 ID，不能传服务端路径。`office/import-targets` 列出获权槽位和是否尚未占用；父目录仍须满足原 Core 的纳管条件。`office/import/prepare|commit|operation|settle` 复用原 create-only ImportDocx 计划，创建完整原生树、转换正文和批注，并保留 DOCX 原件及图片。准备的安全损失说明与原计划持久保留，确认才能提交；同一原操作重启后继续核对，不覆盖已占用目录。结束未安装的原请求会阻止迟到提交。未知成员和恢复语义沿用 Core，不另造导入 writer。导入生成的资源默认不获得读取／输出／下载权限；部署者可逐项配置，或通过上面的 fromImportSlot 集合提前授予明确资源权限并选为 inputs。客户端不能借导入修改策略。

Office 输出最多 32 件、单用户 8 件，输入／工件分别最多 32 MiB，保留工件与预览合计 64 MiB，15 分钟到期；每块下载最多 64 KiB。模板和 DOCX 原件各最多 16 MiB，对应路由 JSON 限额 24 MiB，其他托管路由保持 3 MiB。冻结 Query 最多 32 份、单用户 8 份、数据与限定 Schema 合计 32 MiB，至多保留 15 分钟。达到容量时只淘汰同一用户最旧结果，旧 queryId 明确过期，既有已准备 Office 工件仍保留原字节；不淘汰其他用户结果或自动重跑。新结果自身或其他用户占额使其无法容纳时，拒绝新结果并保留已有缓存。浏览器存储容量可能更小；写入失败会保留本页原件和输入、提示复制／重新选择，并阻止无法保留原编号的新请求或提交。`test_hosted_office.py` 使用真实转换和 HTTP 校验独立授权、原工件、审阅交换、限定 Schema、重启、原导入及撤权；`test_hosted_office_browser.mjs EXE NEW_EVIDENCE_DIRECTORY` 是后台浏览器入口。真实 Word／WPS 等兼容性按候选记录逐项验收，不由 HTTP 或 ZIP 检查替代。
