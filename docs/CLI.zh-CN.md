---
source_language: zh-CN
translation_of: CLI.md
translation_status: synced
---

[English](CLI.md)

# 正式命令行

所有者连接先从原 OS 管道取得并保留进程身份，以及原程序的受保护文件句柄。身份握手只交换版本、物理资料摘要与随机挑战；同一文件句柄的程序完整资格通过后才允许业务正文、凭据或停止命令。大型程序的资格读取有独立期限并使用受跟踪的后台工作，不占用对端的身份握手期限；失败不自动重连或重发请求。

正常关闭在监听器完成信号之后等待 HTTP 服务及请求 future 实际销毁，再汇合已接纳工作并释放原 Core。本地与托管服务共用同一生命周期屏障；响应完成本身不能证明最终权限引用已经释放。

`weftext-cli` 是业务控制台入口，与 `weftext` 桌面入口共用同一个 Core 和本地所有者。命令行不直接修改库中的作者文件，也不启动第二套 Core。实现与命令说明不等于对应平台已通过验收；实际产物、平台和未验条件仍以 [MVP](MVP.zh-CN.md) 为准。

默认连接当前用户的本机资料目录；`--data-dir PATH` 明确选择另一资料目录。Windows 客户端通过受保护本地通道核对当前用户、真实物理目录及所有者实例。原请求绑定物理资料身份；同一路径被替换为新目录时，不把旧请求投给新空日志。共享所有者的退出由生命周期管理，客户端结束不强杀共享进程。

按需启动的无界面所有者保持运行，让独立的读取、外部编辑、保存命令保留同一真实会话；它不显示托盘或运行桌面提醒调度。`capabilities --output owner.json` 的结果包含经 OS 核对的 `ownerIdentity`。需要结束此所有者时，使用同一 `--data-dir PATH owner-stop --owner-identity owner.json`；它只连接已存在的准确进程，排空已接受任务并等待原进程退出，不启动新所有者或按名称结束进程。所有者重启后旧 basis 会话失效，CLI 明确拒绝，不自动重开替代。

`--data-dir PATH owner-status` 只查询现存所有者身份，不启动 Core 或调用作者 API；它可辨本地／托管角色。其 JSON 同样可作为 owner-stop 的准确身份输入。

身份查询输出 `{"schemaVersion":1,"call":"ownerStatus","ownerIdentity":{...}}`。严格身份字段为 `pid`、`created`（Windows 原始 FILETIME，不是 Unix 时间）、`incarnation`、`profileIdentity` 和 `role`（`local` 或 `hosted`）。停止命令接受此信封、capabilities 信封或原始身份对象，输入须为不超过 16 KiB 的 UTF-8 JSON 文件。成功停止输出 `{"schemaVersion":1,"call":"ownerStop","result":{"owner":{...},"exitCode":0}}`，仅在准确的原进程句柄确认退出后返回 0；closing 回执本身不算完成。所有者非零退出返回 3；停止请求已可能接受但结果不确定返回 6，不自动重试。

`--endpoint https://server.example/ --credential-env WEFTEXT_CREDENTIAL` 选择 Server 模式。环境变量参数是名称，凭据值不放命令行、请求文件或输出。地址必须是无用户信息、查询串和片段的 HTTPS 根地址；受控本机测试可显式加 `--allow-loopback-http`，仅接受字面回环 IP。客户端不使用环境代理、不跟随重定向、不自动重发效果请求。Server 接收资源选择器和原授权，不能把本机绝对路径当作 Server 能力。

## 输入、输出与语言

默认 `--format json` 向 stdout 写一份 UTF-8 JSON 信封及一个 LF，包含 `schemaVersion`、准确目标、固定 `call`、真实 `httpStatus`、原服务 `result`，有副作用时还包含原请求文件位置与摘要。服务错误响应仍保留此信封并返回非零退出码。本地输入、传输或交付失败不向 stdout 伪造替代结果；stderr 输出一行 `{"schemaVersion":1,"error":{"code":"...","exitCode":6}}`，已知时附 `operationId` 与 `requestFile`。保留输入另输出一行含 `event:"requestRetained"` 及同一定位字段的 stderr JSON；因此 JSON 模式的 stderr 是 JSON Lines。`--format human --locale zh-CN` 或 `en-US` 使用共用消息目录；机器字段名、错误代码、原始源码和作者值不翻译。

`--output PATH`、`note read --source-output PATH` 和 `export download --destination PATH` 只新建文件，不覆盖已有文件；父目录须已存在。Windows 设备名称、备用数据流及尾随点／空格别名均拒绝。请求文件、结果文件、源副本与下载位置不可互相别名，且不得落入已知作者库或 Core 私有目录。下载不重新生成工件，核对原请求编号、分块、长度和 SHA-256 后再交付。写入失败保留现场，不删除已有原件。

输入文件及 `--source -`、`--input -` 的 stdin 必须是完整 UTF-8；一个进程只能消费一份 stdin。源码字节不经 shell、换行归一化或编码修复。源码上限为 2 MiB；远程上传上限为 16 MiB；本机交换上传上限为 64 MiB，实际格式仍受服务预算限制。重复 JSON 键、无效 Unicode、截断及超预算输入在提交前拒绝。原生整数／十进制值仍按原生类型协议的字符串表示，不经过浮点数转换。

Shell 可以自行改变重定向或管道编码。特别是 Windows PowerShell 5.1 的重定向不承诺保留原生 UTF-8；需要准确文件时使用 CLI 自己的 `--output`、`--source-output`、`--source FILE` 和下载参数。CLI 的 BrokenPipe 是结果交付失败，不是撤销作者操作。

## 常用命令

`weftext-cli --data-dir PATH serve` 经共用所有者生命周期运行显式 HTTP 宿主。`serve --port PORT` 选择本地端口（0 表示分配可用端口）；`--open PATH --root PATH` 指定初始本地文档。`serve --server-config FILE` 选择托管 Server 配置，与本地 port/open/root 选项互斥。这些参数只由此子命令持有，`--data-dir` 仍是共用全局选项。启动成功输出一行 Ready JSON：`{"schemaVersion":1,"kind":"ready","url":"...","ownerIdentity":{...}}`，随后持续运行至优雅停止。Ready 只证明启动，结束仍须核对 `owner-stop` 与准确原进程退出。本命令拒绝客户端 `--endpoint`／凭据选项，以及业务 `--output`、`--format human`、`--allow-partial` 选项。

启动失败不输出 Ready；stderr 先输出一行经批准的固定诊断 `{"schemaVersion":1,"diagnostic":{"code":"...","message":"..."}}`，再输出标准错误行并返回 3，不回显配置内容、任意运行时错误字符串或凭据。

本地 origin 启动失败若带有安全的类型化诊断，会在同一 `diagnostic` 对象内增加 `phase`、`ioKind` 和 `osCode`。`phase` 来自固定阶段枚举，`ioKind` 仅表示标准库 `io::ErrorKind` 判别，`osCode` 是可用的数值 OS 错误码，否则为 `null`；这些字段不包含底层错误的 Display、Debug、source 链、路径或凭据。未附此诊断时，原 `code`／`message` 记录保持不变；不会新增第三行，最终标准错误仍保留原 code 和退出码 3。阶段说明失败位置，不证明原操作未产生任何效果；没有这些字段的历史失败不能由后来的诊断补认为已知原因。

以下示例中的 `C:\Notes` 是作者库，当前目录是外部工作目录，输出路径尚不存在。

```text
weftext-cli capabilities
weftext-cli library create --path C:\Notes --request-file create.weftext-request.json
weftext-cli library inspect --path C:\Notes
weftext-cli note create --root C:\Notes --parent C:\Notes --name 开始 --request-file note.weftext-request.json
weftext-cli note read --path C:\Notes\开始\开始.typ --root C:\Notes --output basis.json --source-output draft.typ
weftext-cli note write --basis basis.json --source draft.typ --request-file save.weftext-request.json --output saved.json
weftext-cli operation inspect --request save.weftext-request.json
```

`basis.json` 必须是本 CLI 读取产生的 JSON 信封；保存复用其中的准确 session/base 和目标，不隐式重新打开另一会话覆盖并发修改。再次编辑应取得新的读取结果。`note save-as` 另加 `--path`，仍由真实 Core 处理新位置。

| 命令族 | 入口及输入 |
| --- | --- |
| 库 | `library inspect/create` 为本机入口；Server 用 `library list` 读取已授权文档。建库可用 `--options FILE` 输入真实建库选项。 |
| 笔记 | `note read` 本机选 `--path`，Server 选 `--resource`；`note create --root --parent --name` 为本机真实事务；`note write/save-as` 使用读取信封与源文件。 |
| Query | `query catalog` 本机需 `--root`，Server 不接受 root；`query run --input FILE` 消费准确 Query DTO，保留覆盖范围、限制和空结果。 |
| 结构操作 | `operation prepare/commit --input FILE --request-file PATH` 消费当前真实 Plan DTO；commit 必须携带原 operationId 与 planId。Server 使用该 actor 已授权资源，不开放任意 HTTP 路由。 |
| 原结果 | `operation inspect --request FILE` 只查原号；`operation settle --request FILE --request-file NEW` 先查原结果，再由原服务终止未提交请求。已提交事实优先，待恢复 Plan 不被当作无效果。 |
| 本机恢复 | `operation recoveries`、`recovery --operation-id ID`、`resolve --operation-id ID --action ACTION --request-file PATH` 使用现有恢复入口，不直接改日志。 |
| 历史 | `history list/read/compare --basis FILE`，read/compare 另给 `--version`；`history restore --input FILE --request-file PATH` 消费当前完整恢复 DTO，保留草稿与原操作边界。 |
| 输出 | `export prepare --basis FILE --kind docx/pdf/doc/wps`；本机还支持 source。`--output-locale zh-CN/en-US` 冻结内容语言，默认 zh-CN，与展示语言 `--locale` 独立。其后 `export status/confirm/download --request FILE` 使用同一原请求，confirm 明确给 `--digest` 和必要的 `--accept-losses`。 |
| 完整输出参数 | `export prepare --input FILE --request-file PATH` 消费含必填 `outputLocale` 的固定 ExportPrepare DTO，可用真实 Query/字段 XLSX、native、文献、模板和嵌入 Query 参数。`--output-locale` 不能覆盖这份原输入。本机另需 `--basis FILE` 验证 session/base 与作者库边界。 |
| 导入 | `import inspect/prepare --file FILE --kind FORMAT`；本机 prepare 需 `--basis FILE --name NAME`，可选 `--parent`。远端原生／文献用 `--slot`，Office 用 `--resource`。commit/status/settle 只接受对应导入族的原请求文件。 |
| 同步 | `sync binding/enable/publish` 选择本机 root；`status/receive/export/history/adoption-cut/revalidation-cut/revalidation-review` 使用各自真实同步 DTO。`sync operation` 查原 Plan，`sync cancel` 用原 operationId/planId 取消准备；语义 review 不写作者事实。 |

完整参数以 `weftext-cli <命令> --help` 为准。结构、Query、历史恢复和高级输出的 JSON 是现有生产 API 的类型化输入；不能省略原始 session、base、Plan、授权选择器或损失确认，也不能把界面 DTO 换成猜测的简化格式。

`sync check/push/pull` 复用本地所有者既有远程传输作业，连接参数为 `--server URL --server-credential-env NAME --identity FILE`；本机显式测试另给 `--allow-server-loopback-http`。identity 可为准确 SyncIdentity JSON，也可为 `sync binding` 的 CLI 信封。push 另给 `--root`，pull 可用 `--objects FILE` 限定对象；两者都先保留请求文件，默认最多等待 900 秒，可用 `--wait-seconds 0` 返回当前作业状态。`sync job --request FILE` 查同号作业，`sync stop --request FILE --request-file NEW` 请求停止同号作业。凭据仅经受保护通道的独立内存段传入所有者，不进入冻结 wire。传输只复制内容寻址记录；接收后仍由真实 Plan 显式预览、应用，不直接改作者库。作业过期／所有者重启后保留 unknown，不自动再启动原传输。

PDF 和带嵌入 Query 的远端输出先通过现有 editor map 观察同一源码／serial／hash，再提交完整准确 snapshot；map 不保存作者文件。简单 `--basis` 输入使用新 CLI 会话的 serial 0。更高版本或另一个源码已经被观察时会拒绝过期 snapshot，不重置原会话绕过冲突。高级远端 native 和 selectedWorks 输出不要求额外可读文档，其资源／源码字段与普通正文输出不同，按各自 DTO 和独立权限验证。

## 原请求与退出状态

所有作者、输出和同步效果命令要求一个尚不存在的 `*.weftext-request.json`。CLI 在发送前一次编码实际 wire、生成或核对原编号、绑定真实目标，使用 create-new 写入纯输入并 `sync_all`。文件只有输入及定位信息，没有凭据、成功回执或另一套事实账本，之后不按响应更新。所有者控制使用独立的准确进程身份及真实关闭回执，不是作者操作。

回包丢失或效果不确定时，只查询同一原编号；不以新会话／新编号重做保存、导入、输出或同步。明确的 operationConflict 不会被同号旧成功覆盖。`operation inspect` 使用原输入文件，不把 notRegistered 当作无效果证明；显式 settle 使用真实原服务的终止／恢复边界。切换 actor、deployment 或物理本机资料后拒绝旧输入。

| 退出码 | 含义 |
| --- | --- |
| 0 | 本命令结果完整；显式 `--allow-partial` 也可允许部分结果退出 0，但原 partial/coverage 不变。 |
| 2 | 参数、输入、预算或输入记录位置无效，尚未提交该效果。 |
| 3 | 不支持、依赖／连接不可用或服务明确拒绝。 |
| 4 | 认证／授权失败；原请求仍保留。 |
| 5 | 冲突、失败、过期或未完成承诺的效果。 |
| 6 | 原结果未知、待执行或待恢复；保留原号和准确输入。 |
| 7 | 结果／文件／stdout 交付失败；已提交事实仍保留，不能据此重做作者操作。 |
| 8 | 部分范围、限制或恢复问题；原始覆盖说明保留。 |

CLI 只报告真实服务提供的原结果。某个 API 若还没有可查的原号事实，CLI 保持 unknown；集合状态或目录存在不能升级成单次成功回执。平台等待、stdout/stderr、退出码、后台所有者及子工作进程须使用实际产品分别验收；私有机制试验不能代替产品通过。
