---
_weftext:
  id: "e73fe605-098f-4b22-b780-76475ad863fa"
---

# D9 Worker 与导出

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


revision: D9-r04-2026-09-24；candidate。此处冻结 Weftext 自有边界；表中的候选工具不是已安装、已许可或已通过产品验收的能力。

## 1. Provider、Route 与 capability

Provider 是经安装/版本/依赖/许可证/沙箱审查的具体可执行适配器。Route 是一个有限、有序、无循环的转换流水线；每一步固定 Provider、版本、输入格式 profile、输出 profile、选项和预算。不能因文件名选择任意程序、下载依赖、执行 shell command 或尝试未列入计划的 fallback。多步 OFD→PDF→page IR 必须同时声明两步及两段损失，保留原 OFD；中间 PDF 不冒充原件。

Registry 为 host 安装配置，不是 worker 输出、模板内容或工作区可编辑普通文本。routeId/profileId 是 ASCII `[a-z][a-z0-9_.-]{0,63}`，匹配唯一已审查记录；记录更新使旧未完成准备失效。每条记录包括具名 executable 与依赖版本、平台/架构、运行参数 schema、精确输入 variants、输出类型、资源上限、隔离证据、许可分发结论与格式语料测试。只有完整满足 D1 capability 规则才 available；缺一项即 unavailable。Provider 自报支持、扩展名命中、exit0 均不是资格。

本代输入 profile ID 与 probe variant 的唯一对应：pdf_page_v1→pdf/pdf；image_page_v1→image/png或jpeg；csv_utf8_v1→csv/utf8；tsv_utf8_v1→tsv/utf8；xlsx_grid_v1→xlsx/transitional；ods_grid_v1→ods/odf13；docx_flow_v1→docx/transitional；odt_flow_v1→odt/odf13；xps_page_v1→xps/xps2005；oxps_page_v1→oxps/openxps2009；ofd_page_v1→ofd/gbt33190_2016。CAJ probe仅报告caj/caj、caj/hn或unrecognized，**无本代执行 profile**。所有其他组合或未知variant不可 convert。识别不等于可用；Strict OOXML、新ODF版本、TIFF多帧等需独立profile。恶意伪装/包混合/版本不明拒绝，不静默降级。

| 格式/路线 | 已查证事实与D9选择 | 未满足的生产门 |
|---|---|---|
| PDF | 当前repo有DoclingLitePdfAdapter，锁定docling.rs v0.52.2；其安装清单明确completeForExecution=false。保留为候选提取器；PDF渲染/提取是不同操作。 | Windows/Server具体架构、worker/ORT完整安装、OS隔离、真实语料与模型分发证据。不能宣布现有Rust crate已支持生产PDF。 |
| PNG/JPEG | page profile只承载正向单帧图像；OCR是独立已安装模型选项。 | 解码器版本/漏洞/EXIF/像素炸弹/模型语言范围；本代不联网OCR。 |
| XLSX/ODS/CSV/TSV | 首选直接安全包/XML或文本解析到grid IR，不启动Office；保留公式文字/缓存而不求值。 | 每个格式/版本全覆盖、重复/稀疏行列、limits、恶意包与独立对照。 |
| DOCX/ODT | 首选直接包/XML→flow；导出由纯模板编译器生成。LibreOffice headless仅是可选另一路线，不能当语义无损转换器。 | 实际Office互通/样式/字体/分页、LibreOffice具名版本/隔离/安装依赖/license审核。 |
| XPS/OXPS | ECMA-388定义OpenXPS；MuPDF官方列出XPS。候选MuPDF页面路线必须分开验证XPS2005/OpenXPS2009。 | MuPDF AGPL或商业授权选择及产品分发评估、具体构建支持/字体/平台/隔离；未通过不提供fallback。进程分离不是许可豁免。 |
| OFD | GB/T33190-2016；OFDRW是Apache-2.0 Java候选，导出模块提供PDF/image等。可评估明确OFD→PDF route。 | JRE、converter依赖/字体/签名保留说明、平台包、标准测试及sandbox；不能把主repo许可代替全部依赖审计。 |
| CAJ/HN | caj2pdf README区分CAJ/HN且HN不完整，依赖mutool/可选native decoder；LICENSE为GLWTPL。没有足以冻结通用CAJ安全/保真profile的证据。 | 本代convert unavailable；允许将用户明确输入作为不执行原件Resource保存。未来先变体/样本/许可/依赖/隔离评估，不能改后缀调用PDF解析器。 |
| Office/WPS自动化 | Office官方不建议/不支持一般Server无人值守自动化；Desktop安装也不证明无人交互安全。 | 不作为本代Server/默认Desktop Provider。将来明确opt-in profile另审，不能fallback到用户正在编辑的实例。 |

## 2. 隔离执行与输入不可信

所有probe、解析、OCR、渲染和模板编译都按不可信文件执行处理，包括“纯Rust”provider。可信host只做有限原始文件复制、启动隔离环境、结构/预算检查；完整格式解析不得跑进作者事务进程。worker不得看到workspace mount、author DB、用户home、浏览器profile、SSH/API凭据、环境secret、系统剪贴板或网络。只暴露本job只读输入slots、只读已审查依赖/字体/模型和私有输出/临时目录；禁止继承额外handles。stdout仅有限协议，stderr有界并脱敏。

Windows必须实际证明受限身份/AppContainer或等价隔离、ACL、禁止网络、受控子进程树与Job Object限额；仅Job Object限内存不能证明无文件/网络访问。Linux必须实际证明只读挂载、独立namespace、网络禁止、seccomp/等价系统调用策略、cgroup与进程树清理；仅container名称或timeout不够。没有相应host证据就provider_unavailable，不以“用户信任文件”跳过。

ZIP/OPC/ODF类输入：在解压前及流式解压时同时限压缩bytes、展开bytes、entry数、层数、压缩比、XML nodes/text；拒绝路径绝对/上溯/编码别名、重复规范part名、symlink/hardlink/device/reparse point、重叠entry、加密/未知算法、DTD/entity/network解析及循环关系。ODF repeated row/cell必须按预算展开计费，不能以一个XML元素规避百万格成本。外部relationship仅检测并报告，不获取。输入包含active content未必可转换，但不能运行；Office模板按Templates整体拒绝。原件附件仍是惰性bytes，预览另走安全profile。

WorkerInvocation/1内部closed记录：`{version:1,jobToken,step,routeId,routeRevision,profileId,inputs,options,budget}`；inputs为`{slot,byteLength,sha256}`数组，slot由host分配。step是Counter；routeRevision为host签发D6 Token；job/route/input/audience均在host记录绑定，worker只能回传相等值。它没有path/URI/executable/command字段。Process I/O handles在启动层按slot配置，不由JSON控制。budget逐项复用D6 BudgetBinding；步骤共享整个job的累计扣费，不能每步重置额度。

终态结果两variant：`{version:1,jobToken,step,status:"ok",resultSlot,outputSlots}`或`{version:1,jobToken,step,status:"failed",code}`。code仅`unsupported|encrypted|unsafe|malformed|budget|cancelled|internal`；非zero exit、signal、timeout、协议截断、额外stdout、结果缺失均失败，不收partial IR。resultSlot为完整IR或该route声明的下一步格式；outputSlots为`{slot,byteLength,sha256}`；slot不重复、实际文件无别名且为regular file，host拒绝额外未声明输出，并重新读实际bytes比对。worker不能写权限、loss choices、身份、receipt或published标志。

输出先完整验证IR/格式覆盖/损失/预算，然后pin为immutable。有效字节不代表提取正确；独立验证器和语料证据是profile启用门。解析结果中的text、公式、header、HTML或错误消息从不变成命令、可执行模板或HTML。输出在生成目录内保留到host验证，不能worker自行rename到最终路径。

Worker crash、timeout、用户取消时host先终止并确认整个进程树退出，随后清理未pin暂存；author store不变。kill无法确认则隔离目录封存、route禁用，不能重用仍可能被写的output。重试必须相同输入/版本/预算且明确剩余额度；自动重试次数有限并扣累计work，达到上限失败。不得无限fallback或换route冒充同一结果。源文件/已committed receipt永不因清理删除。

## 3. 导出域和精确快照

ExportPlan/1是Core持有的不可变准备记录，包含workspace/authority/current authorization generation、全部来源EntityTargets及source versions/Query完整结果、输入域、顺序、所需字段/资源、模板准确bytes/version、ExportContentSelection/1、binding/missing/imageSizes policies、route/version、目标格式、完整初始ExportLossReport、output预算和明确destination意图。确认记录单独保存并绑定这份不可变Plan，不在确认时改写Plan。由D6独立tag token返回；不接受客户端提供已授权snapshot，UI不能把rowHandle当持久来源。此记录不是作者intent/ledger，也不修改任何source revision。

第一代导出源域：`document`（一个准确Document及显式资源闭包）、`native_table`（同Document准确table locator、完整矩形）、`node_collection`（完整NodeRef结果、显式选定可读字段）、`query_rows`（D7完整typed rows和列schema）、`query_json`（D7原typed JSON包括合法graph/scalar）。node_collection与query rows不假造Record/永久行ID；重复value rows保留bag。无序结果必须明确sort成完整稳定次序；相等sort key允许同值重复，但不同内容不能由opaque rowHandle打破平局，需添加完整可读显式tie-break keys或拒绝。

Core在任何输出或首data chunk交付前完成Query和所有schema/值/资源资格校验。空nonterminal页继续读取；auth generation变化reset并丢弃旧未发布snapshot，不能拼接不同权限世代。即使导出只投影几列，执行仍遵循D7全部依赖读取与完整结果合同，不用export投影绕过Query权限。只有CSV/Office等渲染投影的RenderSnapshot每个binding使用Templates有限union；query_json不经过该union，完整field occurrence/qualifier不自动降为scalar；每列schema列明name/type/source domain，不把none当missing field。

导出准备按source domain分为两条已授权读取分支：

- D7ResultPin：持有原D7完整TerminalSchema、原V值代数及完整terminal数据，与原result/epoch/auth/cut和全部依赖绑定。不改变NodeRef/object/quantity/Optional/union等型，不将原V直接传入Templates；query_json直接序列化，渲染路径只可显式投影为RenderSnapshot并保留原pin证据，不重新Query取部分结果。rows的bag及顺序、scalar恰一value（包括none）、graph完整nodes/edges及其column bindings保持。未ordered rows若要求稳定文件则必须显式完整sort，不能由rowHandle排序；graph遵守原两表有序要求。
- RenderSnapshot：document/native_table/node_collection/query_rows的显式有限渲染投影，使用Templates bindings。复杂值只有Core明确投影为受支持标量/块/资源后可用，不能借原JSON支路自动字符串化。投影不改变原Query授权、complete或reset规则。

query_json文件是原结果语义的静态序列化，不是新增分页wire：exact {format:"weftext.query-result-export",version:1,schema,data}。schema逐字采用原TerminalSchema；data由schema.kind唯一决定：rows为 {rows:[[V...],...]}，scalar为 {value:V}，graph为 {nodes:[[V...],...],edges:[[V...],...]}。每V逐字用原D7 V wire decoder/serializer；列型/数量与schema完全相等，不允许unknown值或降型。序列化使用原CJ规则的精确数字/Unicode语义，不经JS double；每行occurrence保留，不dedupe。移除的只是运行时运输envelope、resultToken/cursorToken/rowHandle/epoch/分页状态，不删schema/value中的真实作者Ref。文件不是author导入或持久row identity，也不取代原D7的CLI typedJSON公共接口；该独立文件profile必须明示。

两分支都在prepare/inspect/确认/publish读取前检查原资格；Query auth generation变化使旧未发布结果整体reset，重新授权也不能续用旧pin。不因已生成staging而取消这个检查。

ExportBlocks只是Core的内部只读编译结果，依D2 AST已知kind穷尽匹配；不是接受外部JSON的新作者协议。每块保持完整D2源跨度与明确资源依赖。SavedQuery/View definition的literal代码可以选择导出为可读literal且有metadata损失；运行其view则必须显式选择、完整D7结果和专门已实现renderer。未冻结的TOC/index等不能自动执行。正文引用渲染必须当前目标读取授权；引用不可读时本代整体失败，不泄露label/Ref或静默drop。网络图片/外链不获取。

| 输出 | 第一代明确语义 |
|---|---|
| 原exact source/resource下载 | 原bytes、完整当前读取资格；不是格式转换。作者source只走D6原读接口，D9不把IR当exact source。 |
| D7 typed JSON 文件 | 只使用D7ResultPin和上述静态文件profile，完整原schema/V与order；无模板、无Office标量union、无分页/rowHandle；不是author snapshot。 |
| UTF-8 CSV | RFC4180逗号、CRLF、双quote；完整矩形/顺序/显式header。仅投影已选择标量，none与空text合并必须requires_choice loss。复杂对象拒绝，或用户显式Core投影为独立scalar列，不能JSON-stringify fallback。 |
| UTF-8 TSV | 无quote escape、字段不得含tab/CR/LF；否则拒绝并让用户选择CSV。零行与有header单行明确不同。 |
| XLSX/ODS | 每列显式typed值，text永远作为text cell；不把`=...`字符串写成formula。integer/decimal默认精确text，用户选择numeric只在目标格式可精确表示且预算内；Excel数字精度15位，超出强制拒绝numeric选项。date/instant默认ISO text，不自动转serial/时区；first profile无date serial输出选项。none明确blank并记录与absent/empty text的区别及loss，空text仍text。 |
| DOCX/ODT | Templates普通token/样式/重复合同；D2正文及table只按受支持结构编译。固定默认模板也用同一合同，不是绕过模板检查的另一renderer。 |
| PDF | 从受控且已验收的DOCX/ODT或page route渲染，只可宣称固定页面输出；可编辑结构、字体、分页、超链接/标签与可访问性损失独立报告。未验收route不可用。 |

CSV/TSV将被常见表格软件解释：本代对任何text字段（含header）若首scalar为`= + - @ TAB CR LF`或全角`＝ ＋ － ＠`，或去除前导Unicode White_Space后首scalar命中，整体拒绝。White_Space固定Unicode15.1集合，不用host locale变化。单纯加CSV引号不是防公式措施。第一代不提供通用sanitization承诺，用户可改用明确text cell的XLSX/ODS或typed JSON。合法negative typed integer以已验证canonical词法按numeric列输出，不误判text，但转成text后重新适用检查。标准CSV序列化必须正确quote分隔符/双quote/换行，防止内嵌separator另起cell。外部应用再次编辑/保存可能改变安全性，本profile不声称永久防护。

所有格式必须提供下节ExportLossReport/1，完整列明本次所选消费内容的实际变化和明确受权省略目录中的已知遗漏，包括Field附属信息、control/metadata/resources/annotations、复杂型投影、关系identity、精度、日期、模板空值、布局/字体和公式缓存等。尚未请求的类别以scope明示未请求，不为其制造省略、为空或已枚举的事实；已经请求的内容不能因目标不支持或无权而暗改为未请求。目标不支持时必须基于原选择产生真实损失或拒绝。拥有一个为其它读取目的取得的完整pin，不等于选择消费其中全部内容。

## 3a. 导出来源、投影、损失与确认的闭合域

导出使用独立 ExportLossReport/1，不使用导入 LossReport/1 或 TemplateLossLocation。下述对象均为 Core 拥有的导出控制证据，不是新的作者值、Ref、Locator、D7 wire 或任意客户端 JSON。全部 exact 成员、Counter、Unicode、重复键/索引、有限预算及当前授权按主文共同门严格验证；内部 bytes/pins 不经无界公共 base64。worker 不能建立目录、来源地址或确认记录。

每份 ExportPlan 在任何输出前原子固定 ExportInputCatalog/1 exact {version:1,items}。items 元素 exact {index,label,payload}，index 连续0起，label 是已授权显示文本而非身份；相同内容的两次输入可有不同 index。payload 仅下列变体：

- {kind:"document",sourceVersion,bytes}：准确 Node Document 的原 D6 SourceVersion 和 strict UTF-8 exact source bytes；要求完整 source_read，不能为了报告读取原本未获权的 body。
- {kind:"resource",sourceVersion,bytes}：准确 Resource 版本和原 bytes；要求完整 resource_read/owner 资格。
- {kind:"field",ownerVersion,fieldId,entries}：准确 Node SourceVersion、完整 FieldId 和原 D4 该 Field 的完整有序 Entry 数组，包含其原 occurrence/qualifier/note/provenance；依原窄 Field 读取合同及 Registry 保存，不要求或夹带隐藏的 Document bytes。该版本元数据也须具有原 Envelope 读取资格。
- {kind:"annotation_index",ownerVersion,targets}：一个 Node 的原版本和按 RefKey 排序唯一的完整 Annotation EntityTarget 数组，仅用于已授权省略目录；须在同cut证明完整 state/annotation 目录读取及正负范围依赖。未授权不得枚举或用空数组代替。它不声称持有 Annotation body。
- {kind:"template",origin,bytes}：完整 Office 模板 bytes。origin 仅 {kind:"artifact",source:SourceArtifact}、{kind:"resource",sourceVersion} 或 {kind:"route_asset",routeRevision,assetId,assetVersion}；后者是具名已验收 route 的 immutable 内置模板资产，assetId/assetVersion由该闭合安装记录验证，不可成为任意path/URL。前两者各须实际输入 lease/Resource 资格，bytes精确相等。
- {kind:"query_result",result}：result 为前述完整 D7ResultPin，保存原 TerminalSchema、所有终态 V 数据、原 result/epoch/auth/cut 和完整依赖。该 pin 也可作为 query_rows/node_collection 的渲染来源证据，只有 query_json 才直接序列化原 V。不能把运输 rowHandle/cursor 当 pin 内容身份。

ExportPlan同时冻结ExportContentSelection/1 exact {version:1,bodyInput,bibliographyInput}。两Input分别是null或本Plan ExportInputCatalog中的document项index（Counter），而非客户端可提供的快照；Core从用户明确输入选择解析并复核。首代仅document导出域允许非null；native_table/node_collection/query_rows/query_json两者必须null，不能借模板token增添整份正文消费。document域允许明确选择任意组合；非null的两者须相同index。null只表示未请求，不断言来源为空。非null必须在生成前持有该完整Document原source_read及版本；所需引用/书目目标另过各自原资格。未获资格则失败，不改null。只有完整Document pin而没有相应选择时，不能自动创建body或bibliography binding，也不能自动建立其省略义务。

公开scope由冻结的输入域、选定binding/dataset/资源、这个selection及明确省略目录生成。必须分别说明正文/书目与其它类别是已选择消费、已受权枚举省略，还是本次未请求；不泄露未请求且未授权对象的数量或label。Annotation全量省略目录是独立明确选择：请求则先证明完整目录资格后枚举并报告每个实际目标；未请求则不枚举、不报0。所有绑定派生、loss完整性和scope须在prepare中一起校验；仅地址合法、类型匹配或摘要相等不足以证明值来自该源或该项为全部必需损失。确认、任何出口和原Resource准备前都消费这份已验证完整Plan，不重新推导选择。

目录恰覆盖实际消费或明确省略所需的已授权输入；其顺序由此次显式输入选择及 Core 确定的依赖枚举固定，不能随后按摘要合并或补入隐藏源。document/native_table 可用 document pin（table 指向准确范围）；node_collection 可用完整结果 pin 加选择的窄 field pins，或相应完整 D7 结果，不强制为一个 Field 导出扩大到 full source_read。所有源目录/标签/数量也属于受权输出。尚未选取/授权的其它 Field、资源、批注仅在公开 scope 说明“本次未请求该类内容”，不得声称枚举完整、数量为0或没有损失。要求全量省略报告时必须先取得该全量读取资格，否则本次失败。

ExportInputLocation 仅为这份目录内的位置，closed union：
- {kind:"input",inputIndex}：准确目录项整体，包括完整 Field 或批注目录的省略。
- {kind:"source_range",inputIndex,start,end}：document pin 的原 UTF-8 bytes 半开范围，边界须落在 scalar 边界，0≤start<end≤length；是报告证据位置，不是可写 Locator。
- {kind:"annotation",inputIndex,targetIndex}：annotation_index 中准确 target；不另查它的 body。
- {kind:"query_cell",inputIndex,table,row,column}：query_result 的固定 rows/nodes/edges 表，table必须与原schema.kind相容，row/column为该固定完整结果的零起ordinal并匹配原schema列。scalar只允许 {kind:"query_scalar",inputIndex}。空结果不能捏造cell，整体结果问题用 input。
- {kind:"template_range",inputIndex,part,elementPath,start,end}：template pin 中经安全包解析的准确 XML part、从根的子元素索引、所在逻辑 paragraph/cell 的半开Unicode scalar范围。必须由Templates相同profile复算、命中真实token/结构；不能跨story/cell或使用任意文件路径。整体模板/样式/包省略可用 input。

渲染分支另存 ExportProjection/1 exact {version:1,bindings,datasets}，替代自由的 binding dictionary，不增加作者 schema。bindings 元素 exact {path,value,origins}：path 为Templates允许的非data精确值path，唯一并按Unicode scalar词典序排序；value为该节 RenderSnapshot有限union；origins为非空 ExportInputLocation 数组。datasets 元素 exact {name,columns,rows}，name 为已选 SET、唯一按ASCII排序；columns exact {name,valueKind,nullable}，列名为COLUMN且唯一，valueKind为RenderSnapshot除none以外的一个kind，nullable为Boolean；rows保持已冻结完整顺序与bag，每cell exact {value,origins}，列数、非none kind及nullable一致。每个cell.origins非空且由Core证明其实际派生，不以相同值代替相同来源。外部模板输入不能自报这份 projection。

ExportBlocks 在 bindings 的 blocks值内按其D2已知结构以根顺序、子项逻辑顺序做pre-order枚举，产生报告用的 blockIndex；完整block到准确源span/Resource依赖的关系在生成时固定。不是新的作者 block ID，不能通过未知AST节点省略子项。dataset中的复杂块若该renderer没有逐项完整支持则拒绝；可用dataset_cell指整cell，不把它强制转为text。

ExportLossLocation 为 ExportInputLocation 或以下 projection 位置；各自必须在同一 Plan 的对应目录中验证：
- {kind:"binding",path}：存在的精确 scalar/blocks/resource binding（包括已证明的none）。
- {kind:"dataset_cell",set,row,column}：存在的完整 projection cell，column为列ordinal；零行无cell，重复同值行仍是不同位置。
- {kind:"block",path,blockIndex}：该blocks binding中的准确pre-order block。
每个projection位置的原始来源由上述origins追溯；报告涉及实际模板token时还列对应template_range，不能只给没有实际出现的path。禁止 sourceIndex、rowHandle、自由JSON path、未绑定Ref或跨Plan引用。整体省略可只指input；精确结构变化必须给可复算的range/block/cell，并说明完整 effect。

ExportLossReport/1 exact {format:"weftext.export-loss",version:1,planToken,inputs,items}。inputs是上述私有目录的受权只读摘要，逐项exact {index,label,kind}，kind等于payload.kind；报告随文件携带这些摘要，使人能解释每个index，但不复制原始源bytes或运行时Query handles。label不构成身份；机器验证仍必须使用planToken所指的完整不可变目录，单靠相同摘要不能证明同源。报告地址只有这份固定Plan内的意义，不能回填author source或恢复已过期权限。

items exact {lossKey,feature,locations,effect,severity,allowedChoices}，lossKey连续0起；feature复用导入§7闭集，locations为非空ExportLossLocation数组，effect为具体纯文本。severity和allowedChoices保持唯一矩阵：notice/[]、requires_choice/[accept_loss,reject]、blocking/[reject]。没有损失时items可空。安全、资格、缺schema、未知地址、不能证明覆盖或目标含义的情况整体失败/阻断；不能用accept_loss放行。none→empty、精度/日期上下文丢失、字段/metadata/批注省略、layout/样式/bibliography relocation均按实际固定输出列明，不能用“导出成功”隐去。

Plan 的初始请求只能含投影、missing/尺寸等布局policy，不预填accept_loss。prepare先固定目录/projection和全部生成参数，生成并验证准确 dataFiles，再冻结完整初始 ExportLossReport。公开返回任何 Plan/预览前整个固定提案原子齐全；不允许边inspect边追加loss。初始报告保持requires_choice，不把接受后severity改写为notice。用户确认 exact {planToken,lossChoices}；lossChoices exact {lossKey,choice} 按key排序，恰覆盖全部requires_choice/blocking，notice不接受choice。任一reject取消本次确认，blocking不能确认成功；无选择项时显式空数组仍是发布确认的一部分。成功将确认记录耐久绑定到该Plan的完整输入、projection、route/template/policies、初始报告和准确staged bytes；不能接受客户端替换的report或文件清单。

确认不重跑Query、不重新生成文件、不改变projection、不换destination，也不因选择而重置预算。任何目录、contentSelection/scope、schema、bag顺序、模板、projection、输出bytes、loss或destination改变，都必须新Plan、新完整预览/确认；旧staging不得继续publish。损坏/丢失pin或不能证明原bytes连续性即unavailable，不以重新渲染相同摘要补救。检查当前权限/原Query epoch在inspect、确认、publish及每次交付前继续有效；Query权限换代整体reset，恢复资格不复活旧Plan。

外部bundle中 dataFiles 是按受控相对name排序的主输出及其必要静态附件；每项exact {name,byteLength,sha256}，真实bytes另pin。文件名通过host安全相对路径/别名/冲突验证，worker不决定最终路径。先完成 dataFiles，再生成包含固定 ExportLossReport 的 loss-report.json，最后生成 manifest.json，后者 exact {format:"weftext.export-bundle",version:1,planToken,dataFiles,reportFile}；reportFile是loss-report.json的name/length/真实sha256。manifest不含自身摘要，报告也不含自身或manifest摘要，不形成自引用。完整staging含这三类bytes，PublicationReceipt在bundle之外列出包括manifest与report在内的所有实际文件摘要及原报告、原确认选择。确认选择保存在控制记录/receipt，不在确认后改写已预览的bundle。Plan只在原控制记录持续可证时供机器核验；外部摘要不是授权或作者身份。

Server下载同样先提供完整报告并取得绑定同一Plan/bytes的确认，再按原逐chunk资格交付，不复用本地atomic目录声明。另存Resource也必须先确认本ExportPlan，再以固定已验证dataFile bytes进入原D7/D3完整准备与独立确认；Export确认不授write、不代表author commit。原author receipt不新增导出报告成员；报告保存在原导出控制证据中，是否另存为用户Resource是独立明确的原D3意图。Publication与Resource都不能消费另一Plan或未确认的staging。



planToken先由Core随机分配为尚未对外可用的唯一导出token，再构造引用它的报告、manifest与完整Plan，最后原子保存记录/全部pins并返回；它不从包含自己的report bytes散列生成。准备失败不发布token，也不产生作者身份。

## 4. 生成、确认和外部发布

导出UI/CLI语义入口只有prepare→inspect→publish/state/cancel；具体IPC路由不在D9冻结为新增公共wire，host不得发明author commit。prepare固定ExportPlan、跑worker并验证整个StagedOutput，提供完整文件列表、格式、字节数、真实内容摘要、完整损失和当前授权scope。inspect提供完整预览/文件读取；每chunk重新当前资格，过期/撤权不交付partial-success。独立publish确认绑定此计划与确切output bytes；改变模板/投影/目标/损失需要新prepare。

外部destination由host先经用户选择形成私有handle与明确basename；worker不接路径。第一代**只允许create-only**新文件/新目录发布，已有目标报publication_failed；不开放覆盖、replace或原地编辑Office文件。单个主文件与loss report/manifest组成新建目录bundle；使用同卷私有临时目录完整写入、flush各文件及目录，然后一次create-only原子rename为最终bundle。消费者可另行选择主文件打开；loss report不是藏在Office自定义元数据。host文件系统无法提供被证明的create-only原子目录发布/耐久与恢复边界则该destination unavailable，不能退化逐个copy假称原子。

外部publish记录含planToken、受当前用户授权的destination handle、精确basename、staged bundle内容清单、结果状态与唯一publicationToken，属于转换输出控制区，**不产生author receipt**。状态`staged|publishing|published|failed|unknown|cancelled`，只在rename成功且耐久条件确认后published；真实PublicationReceipt/1包含publicationToken、planToken、输出逐项name/length/sha256、完整ExportLossReport及原确认lossChoices、route/profile/style bundle版本及confirmed destination display，明示external publication。它不是D3/D6 receipt。

开始publish前当前授权再检并取得绑定authorization generation的发布许可；撤权与最终rename必须由同host序列化。如果Server授权不在同锁域，需等价epoch fence，使撤权后旧许可不能发布；无法证明则该Server外部目录destination不开放。Server下载交付每chunk用D6/D7当前读取规则，撤权中止后续bytes；已交付bytes无法撤回，不能称下载整体可撤销原子发布。下载是单文件明确交付，loss report先交付并独立确认，不使用本地目录atomic claim。

crash前已存publication intent且rename结果未知时，重启只检查原destination与staged清单/本地受保护intent，绝不换名字再publish。最终bundle完整匹配且能证明本次占有/rename事件则恢复published；仍在stage且最终不存在且能证明没有并发publish可继续原intent；无法确认时unknown、停止重试，提示用户定位原输出。不能仅凭目标有同摘要文件就断言是本次发布；不可见/已被用户移走同样unknown。输出控制记录不修改任何author事实。

另存为Resource走原D3 create_resource + D7 full preview/prepare/confirm/receipt，Core重新验证stage bytes，明确owner、版本和写权；不能让“外部文件已发布”成为Resource commit证据。一次用户workflow同时要求外部文件和Resource时是两个明确阶段，各自报告结果；本代不承诺跨文件系统/author store原子事务。先后任一失败不补造另一侧receipt或自动删除已发布用户文件。

## 5. 有界证据与完整验收

本轮仅在私有研究区建立真实PDF与XLSX源fixture、CSV样例、纯模板与故障状态模型。它们检验分型IR/拒绝条件/方向和已选状态语义，不运行产品worker，不证明OS沙箱、Office应用互通、大内存输入或真实数据库crash恢复。实施必须逐profile满足Implementation Impact and Test Outline；未通过始终unavailable。所有测试输出和源码进入独立评审可读材料，不用口头“测试通过”替代证据。
