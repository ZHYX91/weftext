---
source_language: zh-CN
translation_status: source
---

[English](CORE.md)

# Typst Core：文件、提交与运行边界

状态：**当前 Core 合同，实现与验收按切片区分**。固定旧基线的独立设计接受保持为历史事实，修订与实验边界见 [REVIEW](REVIEW.zh-CN.md) §8；接通代码不代表产品可用或完整 v0.6 验收。

## 1. 一个文件工作区与一条作者提交路径

Weftext 原生 Rust Core 供 Desktop、CLI、Server、WebUI 和未来 Mobile 使用。Desktop/CLI 本地模式调用本机 Core；远端模式与 WebUI 只调用 Server；断网可留托管 Draft，不在本机授予托管提交资格。Mobile 本地写入须有合格的平台后端。实时协作后置，未来 OT/CRDT 也只能提交明确的作者提案。

普通保存、字段编辑、创建、移动、复制、回收、恢复、批注建议、模板和业务动作统一成为 Core `Intent → Plan → OperationJournal → Receipt`。UI、转换器、Agent、同步器和索引都没有第二作者写入口。转换和外部执行使用可恢复 Job；Job 对工作区的改变仍进入同一 Plan。

持久写事务按实际冲突范围排序。多个用户可同时读、编辑、保留 Draft 和 prepare；一个物理副本的最终 durable commit coordinator 可短时串行，但不是全库长期阻塞或只允许一个用户。不同副本可离线提交普通内容，不因此复制外部执行责任。

## 2. 文件与结构的唯一权威

一个 managed Node 是 `X/X.typ`，正文、声明、字段和作者样式都在该份 Typst 源内。标题和副标题由 SOURCE 的显式作者角色决定，无标题有效；文件名只可作显示回退。资源是普通文件；独立批注为 `weftext.annotations.json`。不增加另一份正文或共享呈现数据库。

根 `.weftext-format` 精确字节为 `weftext.typst.v1\n`；一个 `.weftext-workspace.json` 记录 WorkspaceId、当前 schema 与存储合同选择。标记选择格式，不授予身份、权限或历史连续性。没有有效合同的目录作为未管理/不兼容/不完整内容处理，不猜格式或自动转换。

目录树是 live Node 的唯一 parent/name/path 权威。portable metadata 不另存可独立修改的 parent/children 列表。同级作者导航顺序依 §2.4 使用原 Settings，不改变 parent 或个人列表排序。Trash 的实际目录树决定其内部结构；回收记录可保存原位置、回收批次和恢复成员，因为这些是历史与恢复事实，不能反向覆盖 live 树。恢复目标已占用则冲突，不覆盖。

`.weftext-rules` 不继承 `.gitignore`。Core 的单一库存分别记录文件存在/可见性、Folder Node 资格及显式节点识别边界：ignored 从普通列表隐藏并排除节点发现，字节仍备份；unmanaged 可见、可展开并可打开普通文件，不获得 Node/typed Query 资格。无同名主文件的目录与非配对 `.typ` 默认标“不管理”，不因此停止向下普通枚举或发现未被显式排除的后代配对。用户显式将目录子树设为不管理时，节点发现/批量整理不重入该子树，普通文件浏览仍可进入。完整节点的 canonical `.typ` 不可独立设为不管理；显式退出节点管理须预览身份/引用后果，不自动移动或删除文件。

每次完整库存扫描在根、Workspace 与规则资格成立后，只读取一次全部已提交 Plan 的原 Birth 证据，按原顺序保留本次调用内的物理绑定观察；逐文件仍核当前受保护句柄、身份、源码声明和相关 pending。坏的后续 Birth 不能被较早的有效绑定遮蔽。此观察不跨扫描复用，不授予提交权限；下一次扫描重新读取原证据。无关 Plan 不触发目标路径的 install／commit／resolution 查询，相关 Plan 的原资格顺序与别名边界保持。

一次已接纳修订查询只枚举一次观察记录名称，并始终持有原 journal 目录 guard；每个已提交 Plan 仍通过同一受保护 reader 按观察顺序读取原记录。必需记录缺失或损坏仍拒绝，准确 stamp 与字节 hash、每 Plan 首个匹配事实、耐久提交序号及最大序号歧义规则不变。在确认修订号或确认没有修订号之前，再完整枚举一次并核对相关记录名称集合；名称变化直接拒绝，不接纳新记录替换原结论。名称与修订结果均不跨查询缓存。

节点结构动作仍拒绝显式 ignored/unmanaged 边界内的目标，或已知子树含这类排除边界的动作；普通非节点文件动作经同一安全后端独立预览确切范围，不借普通改名/移动绕过受管节点的两侧配对、已登记资源及历史责任。已接纳节点突然缺主文件或改配对是诊断/恢复事件，不能静默降为普通文件、退休身份或猜新 ID。开放树未观察新增的严格排除仍需强范围能力；§5 的迟到边界 pending 规则针对这些显式排除边界，不把普通容器缺主文件误当同一阻断条件。

受管目录提供稳定快捷目标：Node目录直接用NodeRef；非Node容器由Core `admitDirectory` 创建 `weftext.directory.json`，只记DirectoryId，不复制parent/name/path。出生/复制fresh ID/移动/duplicate/gap按同一Plan与manifest处理。快捷创建只接纳选定容器，不整改全库。unmanaged 边界不自动加 ID。用户可明确只登记目录导航身份，保留其非 Node 状态和原发现边界；这不创建主文件、Node 或 typed 业务对象。拒绝登记或后端无法证明稳定目录身份时，仅提供明确较弱的 locator 书签，不能称改名稳定快捷。根 `.weftext-tags.json` 集中登记immutable TagId与可改displayName；TagRef用于稳定快捷/导航/Query。同名不自动合并，literal标签字符串不自发登记对象。Core `createTag`可与笔记typed TagRef赋值同Plan，SOURCE负责该原生typed槽。

多级标签的 v2 父关系、操作边界、旧资料与同步接纳合同见 §2.3。该通路已接入 0.6.3-rc.1 工作树，整版验收中；原 alpha.3 的 v1 扁平登记只保留为旧候选事实。

portable 控制资料只保存无法从当前文件恢复的事实：身份创建与退休、schema 历史、已提交版本/因果关系、资源文件绑定、Trash 恢复信息。资源的稳定 Ref 通过节点内 `weftext.resources.json` 的资源 ID 与相对文件定位绑定；该文件不决定 Node parentage。普通非 canonical 文件仍可见；未登记资源不被自动授予稳定 ID，原始改名若无法证明原对象则显式重新绑定或冲突。批注 JSON 中的 annotation ID 与其 target/lifecycle 分域。

`.weftext-history/` 保存不可变已提交 manifest 与需要保留的 exact blobs。它是历史、复制与恢复证据，不能被编辑成第二套 current 源。设备/session Draft、活动 SQLite journal/WAL、运行凭据与派生索引放在同步根外。durable commit 到 manifest 发布之间，原 journal 的已提交记录和完整 blobs 是该次身份/版本/结果的真实证据；启动先恢复这些待发布记录，备份必须一致包含它们，不能因 history 尚缺就重新执行。历史快照可能与 current 文件不同；当前读取仍须观察实际文件与当前权限。索引、搜索、关系反向视图、缩略图和图可删除重建，不能重建执行责任或凭空生成 ID。

完整备份包含普通文件、ignored 字节、portable 控制/历史及一致捕获的本机未决 journal/Draft；只复制活动 SQLite 主文件不合格。可单独导出仅资料备份，但须说明它不能恢复丢失的操作/费用责任。恢复到新设备重建本地观察与派生索引；真实未决执行责任只由原日志或显式责任接管恢复。

`BackupLibrary` 保留原 v1 `library/`、`private/`、`backup.json` 树。当前文件 Draft 的首字段为必需 qualified `scope`，file-save 归属固定在同一原 Plan；actor／output 权限首部与其作者库归属分域，不另建归属账本。选择器在受保护句柄上先读有界 metadata，再读正文；文件 actor 记录使用同一原 Plan 分类。可证外库正文不读取，未知或矛盾归属明确失败，不按当前路径认领原资料或导出整个共享私有目录。

原冻结备份计划保留所选准确文件和一枚私有成员／路由摘要，执行与原事务恢复均重新检查。只有同一备份操作的类型和内容均已验证的证据，才允许其自身新增 journal 文件；文件名前缀相同不获得例外。备份只有在捕获完整原未决证据及当前树时，才可包含仍 installing 的源操作，不先解决它或认定其 committed。首个输出 effect 前，原 manifest 与唯一文件／目录须符合 64 MiB、5,000 成员输入预算，无损 ZIP 须在 80 MiB 内；编码只消费这些冻结 after-images。专用托管备份 fence 位于原 ActorOperation，独立于通用 feature／output 权限，不增加自动 restore decoder，也不允许因交付缓存丢失重放原请求。

原生内容包另用 `capture_native_export` 在一次捕获中生成包字节、manifest 与内存 `NativeExportWitness`，消费方交付前复验同一见证，不另读一份当前树来证明旧包。获权的纳管正文、登记资源、批注、Schema、设置和样式按原角色收集；未纳管、忽略和 Trash 内容只记录必要的省略路径及物理元数据，不读其正文后再过滤。当前最多 5,000 个命名空间成员与 64 MiB 保留输入，见证核实际观察到的成员及对象，不承诺任意未来原始文件的强负范围。该包不包含私有 journal、草稿或凭据，不能替代完整恢复备份。

v2 原生包包含闭合 `generatedBibliography` 角色，仅允许唯一 source 所有者同目录的 `weftext.citations.bib`，保留本次准确、合法的 UTF-8 Bib 字节。它没有 NodeId、ResourceId 或作者 revision；与真实成员、大小写别名或已报告省略的同名原件冲突时拒绝，不覆盖原件。生成必须从原包捕获的获权 Work 出版字段派生，完整库输出权不隐含文献使用／输出权。导入时该派生文件只保存在完整原始 ZIP 中，不安装为当前作者文件或资源，并报告 `generatedBibliographyNotInstalled`；应用继续从已重映射的当前 Work 生成文献输入。旧 reader 对不认识的角色明确拒绝，不宣称旧 reader 可读新增角色的包。实际产品通过范围仍以候选验收记录为准。

v2 原生内容包另包含闭合 `Tags` 角色，只允许根 `.weftext-tags.json` 的完整合法 v2 森林；导出捕获原 Core 已接纳完整树和同一见证，当前 v1 表须先显式升级，不能在导出／复制时改写原 blob。跨库导入默认通过目标 Core 为全树创建新身份并保必要祖先，在同一个 ImportNative／CreateTree Plan 的 updates 中写 Tags。以完整源 TagRef 建本次映射，统一改父级、正文 typed 引用、Query CEL 的 tagRef 字面及按 Shape 证明的 typed 参数默认值；普通 literal 原样保留。不按同名或裸 UUID 合并，不顺带改挂已有目标标签，不另存永久别名表，完整原 ZIP 继续保留。

导入 Query 的 Schema 基线仅在完整源 schemaRef／revision 与包中准确 Schema entry.revision 一致、且目标 Schema 语义一致时重绑为目标合法 revision。目标尚无 Schema 而实际 bundled Schema 相等时，在同一 Plan 初始化并固定新 revision。包未保准确的 Query Schema 历史时以 `schemaMappingRequired` 拒绝，不能用包的 latest 或目标当前定义冒原基线。托管导入依据实际计划副作用另查 tags.create／schema.write 及原读取权限；原导入槽权不隐含共享标签／Schema 写权，重试、提交及恢复仍核同一原计划。

当前原生内容包版本为 **v2**：损失和产品说明使用同一类型化消息合同，保留损失的机器代码、位置与完整并列诊断。v1 包明确以 `unsupportedVersion` 拒绝；真实旧 ZIP 保留原件，不自动升级或保留旧解析分支。这里的包版本独立于格式标记、Tag 森林及完整恢复备份版本。

### 2.1 普通文件会话

本地启动文档只作为打开定位提示。浏览器刷新时重新从 Core 读取当前正文、Base 与个人恢复草稿，不重放服务启动时缓存的编辑会话。纳管笔记沿原稳定身份解析，原改名／移动仍能定位；身份不可用时保留草稿并报告失败，不转而打开旧路径的同名文件。普通文件则按明确启动路径建立新的普通会话。

`openFile` 可打开库外或库内未管理的 `.typ`，不初始化工作区、不注入 metadata，不因文件自带 id 就接纳为 Node。会话目标为实际文件对象与授权路径范围；FileBasis 记录原字节、真实对象/路径绑定、观察世代和依赖根，与 NodeRef/RevisionId 分域。它复用 Draft、单文件保存后端、操作日志/恢复与冲突检查，不生成伪 WorkspaceId、portable manifest 或 schema admission。语法无效仍可准确保存原文并显示诊断；写权限、读权限和编译资格分别判断。

首次打开以父目录作为明确可见的默认项目根，可由用户调整。相对路径以各实际来源文件为基准，根相对路径以所选项目根为基准；依赖只读访问须在授予范围内，越界拒绝，不能把整个磁盘交给 compiler。沿用官方包解析及明确的离线/联网策略、应用字体默认；无所属库时不套用当前库样式/schema。依赖文件编辑另开普通会话。

保存比较当前文件对象和准确 Base，保外部并发稿；不能只凭 hash 声称 CAS。另存默认 create-only，已存在目标必须单独预览和取得它自己的当前写依据；来源不变。可用平台提供同一安全后端；WebUI 没有本机文件写能力时说明上传副本/下载另存的实际语义，不冒原位保存。恢复稿存于应用私有区而非在用户目录造控制文件。对已管理文件重定向回其 Node/Resource 会话，不允许绕过其合同。主动纳入库时预览新节点/必要元数据及文件安排，成功前保原件。

### 2.2 私有 Draft 与改名恢复

同一 actor 的原 `drafts` 目录内，以必需原 `FileIdentity` 与必需 `QualifiedInputScope` 派生单层主 record 键。主 record 是该准确对象／库作用域的唯一恢复权威，包括 clean、明确 discard 和 merge 后的状态。普通目录可明确属于设备输入；库输入必有已准入 WorkspaceId 与原根 FI。不同 actor／库不转授读取或写入权，旧缺 scope 记录不自动取得当前资格。

所有原 session 草稿写口，包括普通持久化、过期 Save 保稿和明确恢复决议，均使用 session 原 stamp 的文件身份与已冻结 scope，不读取旧路径现 occupant 或当前库来重新选择草稿归属。Server 配置 namespace 只承担其独立权限核验，不替代文件的原归属。

已提交的同对象 relocation 只在同一 FI＋scope 下更新主 record 的 locator；先将准确旧字节保存在既有 `draft-history`，再更新唯一主 record。不再读取旧 path-hash 槽或缺 scope 记录来合并当前提案，也不保 consumed marker／迁移协议。旧原件保留，缺必要资格明确 unsupported；归档只作保全与明确比较材料，清稿／合并后不得自动复活。主 record 损坏或资格不符不能回退另一作用域或旧历史。

同一 FI 与库作用域不证明外部变动后的当前位置。缺少原已提交位置关系的外部改名或历史坐标回退仍可查看源，但不自动恢复、迁移或消费旧 Draft；历史坐标不能冒作当前迁移证明。新打开的 session 在内存保留位置尚未确认的资格，后续 persist／Save 以 `identityPending` 明确拒绝覆盖原私有记录；编辑器保留当前输入并报告恢复草稿写入失败。正常已获资格的原 session 在已提交移动／交换后仍可按原 FI 与原位置关系续写，不能借旧路径当前其他对象的身份选写草稿。

完整 Draft record（含 Base、提案、全部 alternatives）采用一致的 16 MiB 读写预算；覆写前先校验完整拟议字节，超限或读取／持久化失败保留旧可读草稿，不截断或只检查正文长度。当前 scope 升级的定向与 HTTP／浏览器验证按本轮准确源码和产物记录；先前候选结果不自动证明新格式通过，平台和真实断电限制仍见 MVP。

Windows Compound 在取得完整命名空间保护时，一次捕获各保留目录原句柄的规范坐标。地址比较使用这份固定名称投影，包含已获资格祖先中的短文件名别名；当前源 FI、父 FI、位置阶段与后代成员仍分别必验。移动及后续目录保留不改写原投影，过期阶段不能把原句柄移动后的新地址当作预期位置。

### 2.3 标签森林、原身份与版本升级

2026-10-20 定版并接入 0.6.3-rc.1 工作树的当前合同，整版验收中。根 `.weftext-tags.json` 是标签身份声明、显示名和共享父关系的唯一当前权威；它与真实目录树分域。v2 文档闭合为 `{version: 2, tags: [{tagId, displayName, parentTagId}, ...]}`，各成员必有，根 `parentTagId: null`，其余为同表的裸 TagId。API 中的父级选择使用完整 TagRef 并核 WorkspaceId、实际登记资格及准确版本，不能把外库 UUID 或客户端父链当本库依据。保留现有 UUID、名称和 5,000 条预算，同名合法；不存独立路径、children 或祖先权威，也不设置同级硬唯一。

同一个 Core 解码／校验入口供普通读取、prepare、恢复、同步接纳、历史及交换消费者使用，按用途区分读取旧内容与接纳当前后继。完整 after 森林必须通过严格版本／成员、唯一 ID、父级存在、非自身和无环检查；父链终止于根。JSON 合法还须证明原注册表对象及每个标签的 Core 登记资格。缺父、未登记或观察缺口不把节点临时当根。从未出生的表可凭原缺席依据初始化；已经登记的表缺失或被替换属于诊断／恢复事件，不能按空表重新出生。可用迭代遍历作有界全表校验，不依赖调用栈深度；错误或超预算不安装部分树。

`createTag` 可指定合格父级，并与首次 typed TagRef 赋值在同一原 Plan 出生；`renameTag` 只改名称；新增 `moveTag` 只改变既有标签的父级，包括置 null。后代随父链派生，不批改笔记或重分配 ID。三种动作统一沿原 operationId、准确 before／after、实际文件身份、CAS、actor／授权代次及原结果恢复；预览明确所选标签和目标父级。服务端生成修改，所有能写注册表的路径按实际差异执行操作授权，不能用 rename 或通用文件写绕过 move。前像变化则按原资格重验或重新准备，不盲重试未知结果。

本地取消无需重做失败或丢响应的准备。已有原计划按准确 root／Plan 及尚未安装条件终结；尚无计划时沿原 resolution journal 持久终结原 operationId，不分配替代操作或另建取消权威。重启、迟到准备及将同号用于其它意图都不能复活已取消请求。已提交只回原事实，可能安装的操作仍保现场交原恢复；界面只有读到准确终态才清本机待决记录。

直接赋予已有 TagRef 与换父通常在业务上可交换，但不因此使旧 Plan 的文件／身份／授权依据继续有效。保留原提交条件，也不因层级新增就强迫每次直接赋值读取当前父链。依实际读集决定重验证／重准备；仅由查询或归组产生的结果不成为 writer。未裁决的 Tags 分支只限制真正依赖树的操作，不自动阻止无关笔记编辑。

继续用整体 `ObjectKey::Tags` 和准确 blobs／因果 parents／共同 base。交叉换父 A→B 与 B→A 不按边自动合并；互不相关的改名／换父也可接受整表 CAS 冲突，在确定版本上重准备并验证完整树。首版不加逐标签 CRDT、逐边历史或自动字段合并。部分副本应取得所用版本的完整、有效且合格的 Tags 对象；不新增“已接纳半棵权威树”模型。标签树完整与笔记范围完整分别判断；移动本身无需等待全部笔记同步。后代 Query 的快照及正负范围依赖由 §9 与 FEATURES §5.6 接续。

仅标签文档版本明确升 v2，不改 WorkspaceId、TagRef 或原生 Typst 格式标记；实际资料盘点决定旧支持范围，不由“尚未发布”推定没有真实资料。活动 v1 经原 Core 更新路径转换为根 parent=null，保原标签资格、注册表连续性和因果历史；不能重建同名文件或重新登记旧 UUID 冒充升级。转换准备按原 journal 核本注册表的未决安装及真实前像；旧 prepared／unknown 不改指纹、不由升级器重跑，原操作恢复也不能安装过期的 v1 after 覆盖 v2。raw 外部改动仍按 external_pending／显式接纳处理，不以 JSON 校验成功冒当前已决树。真正留存的 v1 历史／分支／base／备份按相应读取入口解码，在内存解释为全根，证据仍绑定原 v1 blob，不重写历史；未存在的旧资料不建立假想兼容层。可解码不等于可作为当前合法后继：旧离线整表不能补 null 后覆盖 v2 层级；保分支与 base，需要保留的旧意图在确定的 v2 basis 上显式重准备。无额外版本协商或迁移事实库。

当前 Native 跨库导入通过目标 Core 为完整森林新建合格身份并保必要祖先；按完整源 TagRef 建立本次映射，同一映射处理父 ID、笔记及 Query 中的 typed 引用。不静默补根、不因映射顺便移动既有目标标签、不按同名／裸 UUID 合并。将来如增加已有目标身份或仅子树选择，须另有明确选择／重新挂接预览，不据本合同声称当前 UI 已具备这些选项。先验证完整目标森林及预算再在原 Plan 提交。literal 原样保留；映射不是永久别名表。保持身份的同库恢复继续要求原恢复证据，不与跨库导入混用。

### 2.4 作者导航顺序

作者同级手动顺序使用原 `.weftext-settings.json`／`ObjectKey::Settings`，不新增顺序文件、注册表或目录名前缀。version:1 设置可含 `navigationOrders: {"<完整父 NodeRef 或 DirectoryRef>": ["<完整直接子级 NodeRef 或 DirectoryRef>", ...]}`。这里只记录已接纳容器之间的顺序，不改变真实 parent、正文、标签树或个人列表排序；普通文件和父节点自己的主文件不成为子容器。顶层其他设置成员原样保留；该成员严格检查类型、规范完整引用、同库、重复、自身及合计 5,000 项预算。通用配置读取保留准确原字节与快照供诊断，不把读取成功当作语义接纳；导航使用、写入及同步仍须通过各自的完整校验。处理设置可展示原输入警告，但不能覆盖不支持的设置对象。

读取先按当前真实目录和原身份资格取得直接子级，按已保存顺序投影仍在该父下的成员，再按默认名称顺序追加新建或移入成员。失效／移走成员只从显示投影略去，不按同名替代、不自行写回清理。扫描不完整、未接纳容器或身份有歧义时仍可浏览，但禁止作者重排。明确写入须提供准确设置 Base 和完整当前直接子级集合，展示前后顺序，并以原 UpdateFiles Plan 进行整设置 CAS；通用配置写入也复用同一校验。父级、成员及其目录／主文件／登记标记的原证明进入计划依赖，准备后移动、替换或新增成员不能使旧计划错误应用到别的父下。

准备、取消、未知结果查询、安装／恢复及设置同步沿原 Core 操作与对象链。同步传完整 Settings 对象，不按列表项自动合并；历史或部分副本的引用不授予当前导航身份。取消仅能终结未安装的准确原计划，已可能安装则保现场交原恢复；重启不分配替代操作。托管通用设置写入若实际改变该成员，除设置读写权外另需当前 `library.reorderNodes`，并须对每个变更父级有准确匹配的 `navigation.directories` 获权配置；目录成员披露不由重排权扩展到全库，也不授正文／字段／媒体读取。提交及未决操作重验两项资格，成员权撤销也清除旧预览；设置权、字段权或个人排序均不隐含作者重排权。

### 2.5 私有捕获稿与关闭确认

尚未创建的捕获输入由 Core 私有 `capture-drafts/<draftId>.json` 保存，不能伪造已有文件的 FileIdentity 塞进普通 Draft。记录只有 v2：首字段 `scope` 固定设备本人或服务端推导的 actor，以及准确库绑定或明确 `deviceUnassigned`；后者不属于当前打开的库。已选库固定 root 物理身份和 WorkspaceId，父容器保实际稳定引用及其真实目录身份摘要。路径仅用于重新定位；同路径替换库／目录不得复用旧资格，合法同对象移动须重新解析原引用。一条已绑定捕获稿不改绑另一库。

`currentInput` 与不可变 `pending` 同一记录原子替换。名称、标题、正文、位置／参数原文和所有字段控件原文使用显式 UTF-16 code units，保留 lone surrogate、NUL、组合中间输入；标识符仍为合法字符串。合法 native value 仅为可选候选，不能替代所有控件原文，DOM validity 也不能证明字符串可转 UTF-8。形成真实作者源时才严格转换，失败保原 code units，不使用替换字符。类型／字段用稳定 ID，控件地址用 FieldId、值路径和 controlRole，不用 ARIA 文案。恢复稿不附旧 Base、Schema 全文、目录／预览或权限能力。

`revision` 是整记录 CAS，`inputRevision` 单独记录输入变化和同值 ABA，均限制在 JavaScript 准确整数范围。`mutationId` 及命令指纹使最后一笔持久写丢回执后可同号查询／重放；只读恢复查询必须携已冻结的原 typed command 及原 expected revision，不能仅凭同 ID 证明另一命令已保存。同 ID 换命令或旧 revision 不覆盖新稿。负查询仅表示已被后续写替代或未知，不能证明该写从未发生。更新不存在的记录不隐式新建。

字段原文明确区分私有 exact raw 与已证实的本人片段：`CaptureRawInput.text` 是当前字段完整 UTF-16 原值，可含程序预填或保护默认值；只有独立 `author.text/provenance` 才能进入本人纯投影。provenance 固定真实 producer、inputId 和观测修订，不是权限能力。没有本人证据的 raw 不进入该投影；有证据的恢复片段标为 copyOnly，不能自动拼成旧完整字段。结构／Schema 变化在控件退休前将原地址和 UTF-16 原文移入同一 `currentInput.detachedRawInputs`，带独立 retirementId 和原因；旧 list index、union 分支或同名新字段不能继承该绑定。live 与 detached 原文共用 1024 项及原记录预算，detached 不是第二本历史账。资料库授权退休后，服务端按仍有效 actor 身份只返回本人纯投影，不返回私有完整 raw、旧 Schema 候选、模板、私有目标路径或 frozen wire；纯输入更新保留原 scope/pending，不能修改这些保护字段或产生作者效果。

顶层 name/title/body/rootInput/parentInput/parametersText 同样区分六份私有完整原值与同一记录中的 `authorTexts`。每槽保留按真实 producer/inputId 区分的本人证明片段数组，总计最多 1024 项；只更新同身份的较新修订，同修订变字节或倒退均拒绝。新窗口的证明不覆盖旧 producer，不把旧片段拼接后套上新 provenance；空文本仍是原身份的新删除证明。程序预填、DOM 采样和恢复只更新私有完整值，不造本人证明。纯 author checkpoint 只合并 own overlay；字段换 producer 时把旧证明保留为 detached，不能覆盖原私有候选。获权 hosted active 另返六份完整原值和正文模式，撤权纯投影不能自动拼回可提交正文。

字段结构只使用 `valueState = valid {value: NativeValue} | incomplete {wire: CaptureText}`。后者仅保存一棵私有原生字段树的准确 JSON/UTF-16，保留无效标量旁的 siblings、union 标记和 list 位置，不含 Schema/Base 或整个业务表单。两态均严格拒绝重复／未知原生成员；未完成标量必须匹配同 FieldId、原 ValuePath 的 live exact raw。完整树必须用 valid，提交只接 valid。恢复须重新核原 FieldId、Schema 身份和语义摘要，过期／未知地址保原件为未绑定材料，不派发作者 input/proof。旧 value/null 格式明确不支持，不保兼容双轨。

正常获权编辑用一次完整 input 原子持久化其本人证据。唯一原记录持久标明 inputCut 为 full 或 authorOnly，既有 lastMutation 明确封闭命令种类。显式本人检查点不能确认完整表单、冻结或发起新作者效果，也不能给完整表单关闭证明；目标选择和中性检查点保留原 cut。重启时若无 full cut，仅本人片段可取回为 copyOnly，不以 RAM 标记或旧私有候选恢复 formReady。完整请求响应未知时先按同一原 mutation／expectedRevision／规范完整 digest 查询，浏览器 transport 不存该候选，绝不另写 pure mutation 覆盖可能成功的结果。只有重新获权的完整输入 mutation 才建立 full。digest-only reader 必须核原命令确为 Input，其他真实命令的 digest 也不能冒充完整输入 ACK。新修订中的空本人文字仍是明确删除证据。

前端只使用一份 Records v3 纯输入恢复存储与单一 IndexedDB transaction-CAS。捕获项按 deployment／本人 actor／真实 draftId／producer 隔离，同一项分别保 current 本人 UTF-16 输入和待确认 mutation 的冻结本人 cut／expectedRevision／原号／指纹；旧 A 丢响应时的新 B 不能被 A 的 ACK 删除。它不保存 Schema、default、template、target、typed candidate、私有完整 raw 或 frozen 作者效果 wire，不另建 Capture raw ledger。浏览器 stage/flush/ACK 仅证明 transport proposal，不能冒充 Core ACK、完整表单持久化或关闭许可。真实结构选择及 typed candidate 未得到 Core ACK 时仍阻止提交／关闭。IndexedDB 故障保内存与原磁盘项；同 origin 的正常浏览器进程恢复不能自动外推不同随机端口或 Core 进程的恢复。

明确处置未知 private input mutation 必须先查原 mutation。只有同一记录 revision 仍等于其原 expectedRevision，才可用独立稳定原号做 `checkpoint_capture_input`：只 CAS 前进记录修订并耐久发布，不改 currentInput、inputRevision 或真正 frozen 创建 pending。旧 input 先提交则 checkpoint 冲突并查询真实原结果；checkpoint 先提交则迟到旧 expectedRevision 永久拒绝。checkpoint 自身丢 ACK 仍查原号，不盲换号／rebase 旧请求；浏览器项只在确认原 mutation 或 checkpoint 原记录后明确处置。该 fence 不是作者操作终结或完整 Form ACK。

显式继续下一条时，`advance_capture_submission` 必须重查同一 journal 的原 committed/resolved/cancelled 决议；unknown/prepared/installing 不能凭前端标志释放原号。缺 Plan 的本地终结还须匹配原 requestKind 和 wireDigest。只有成功提交且 current inputRevision 仍等于提交 cut，才清空消费过的输入；新输入和 detached 原文保留。捕获稿不另存 Receipt，结果重新打开始终查询原操作。名称留空时由原 operationId 确定默认碎片名称，不要求作者标题。

本地 Capture 创建提交只返回原提交事实，另行交付原结果。HTTP 与 Core 共用单一 `CaptureCreationRequest`；`open_capture_result` 严格读取同一 frozen wire，按原 canonical intent 核已有 feature Plan，再从原唯一 Birth／materialized 身份重开当前节点。它不 prepare/execute，不借同库其他 NewNode、旧 receipt.path 或原 Session 推断结果。打开或界面接纳失败保 pending 原号，下一条无需先建立文档 Session；无效原请求的终结仍只使用 wireDigest。

首次作者请求发送前可靠固定 pending 的原 operationId、requestKind、准确 JSON wire、该 wire 的 SHA-256 及输入修订；不按新 currentInput 重采原请求。读取顶层身份时只解码操作／目标字段，其余载荷用 serde 的 IgnoredAny 校验语法，原 wire 不转成 Value；因此含 escaped lone surrogate 的原被拒请求仍能保留准确摘要。原提交、原号 status／settle 和 typed resolution 继续归 journal，捕获稿不保存第二份 Receipt 账本。hosted 必须另核当前 actor／原库／批准槽，私有记录或摘要均不授予权限。

正式记录完整上限 16 MiB，每个 local／actor 域最多 128 条、记录与保留暂存合计 64 MiB；最终作者源仍受 2 MiB 限制，wire 最多 3 MiB。本地 capture HTTP 请求另限 16 MiB＋16 KiB。列表在读取正文前累计预算。未发布写位于独立 `capture-staging/<owner>/<draftId>.<stageId>.stage`，失败或中断保留原件及旧正式稿；已识别暂存只计容量、不会被当成正式稿或自动提升。原子发布期间最多额外保留一份旧记录（16 MiB），不声称实际掉电事务保证。未知版本／非闭合编码拒读，不建立旧格式回退。

完整 owner backup 同批认识 capture 路径及同一严格 validator：先在原受保护句柄读有界 scope 头，再只读所选库正文；外库或 device-unassigned 的坏／大正文不进入选中预算。actor 路径由真实 owner 重算，不能借当前 actor 或同 draftId 串户。暂存不是库作者参与者，不伪作已确认稿或提交事实。既有 64 MiB／5000 成员总备份预算及显式恢复限制不变。

`capture_flush_receipt` 只在受保护读回与期望 draftId、recordRevision、inputRevision、lastMutationId 完全一致后返回摘要和原 pending 编号。这是持久快照 ACK，**不是 native ClosePermit**。宿主必须先处理 IME 并冻结输入，再把 ACK 绑定当前 surface／epoch／本地 edit serial；任何新输入、迟到回执或持久失败均拒绝关闭。操作结果 unknown 但完整 pending 已持久时可关闭 UI 后继续恢复，不能因此宣称已提交。真实窗口生命周期和消费者按对应切片验收，不因存储 API 接通而通过 DSK02。

## 3. 身份、版本与当前读取

保留随机 UUIDv4：WorkspaceId 与各 Node/Resource/Annotation/Directory/Tag ID 使用完整类型化 `EntityRef`，解决改名稳定引用、离线创建及复制独立身份，且不把名字/路径变成身份。UUID 的价值是稳定随机标识，不是安全凭证。Directory/Tag只补导航必需身份，不成为笔记或逐值事实ID。Core 分配新身份；源码/sidecar中的 ID 是声明。已有身份须匹配已接纳创建事实、当前唯一文件绑定和 lifecycle；重复 ID、外部复制带 ID、缺元数据或 partial sync 不自动新建或重发 ID。

初始化也使用§5.3的create-only边界，不假装普通同用户staging可一次原子发布整库。先durable bootstrap operation、WorkspaceId、配置/源拟议版本和完整after blobs，再在获权目标父闭包中以§5.3直接返回原目录创建handle的原语create-only建立目标root及必要子目录并持续保留同handle；逐文件CreateNEW并持写保护构造root config、schema、根marker和全部计划源。外部可先看到partial文件，但Core发现installing/birth缺失时只返回incomplete，不按marker存在激活。原目标已存在则冲突，不接着初始化其中。只有所有计划对象/manifest/journal与current权限完整，单一commit marker才激活Workspace和creator grant，随后portable publication。

初始化失败保原ID和可恢复计划，未知新增/对象被替换不覆盖或递归清理；仅本操作归属可证的文件和实际empty目录允许条件回退。丢响应查询原operation，不能重分Workspace/Node ID或重复赋creator权限。Server初始权限来自受权issuer配置，不来自client metadata；首次初始化后不得借重启/恢复补赋creator grant。真实已有资料目录走显式import/admit，保原件/unknown/loss，不改扩展名冒导入。

本地 `create_library_with_options_and_id(path, LibraryCreateOptions, operationId)` 的闭合输入为 `LibraryCreateOptions {initialStyle?: {source,enabled}|null}`；省略／null 保留原无初始样式的建库调用。`source` 是准确 UTF-8 原生样式，须语法有效且恰有可认证的 `library-style` 入口；复杂合法规则保原文，不要求全部可由控件编辑。这项验证不承诺所有依赖或实际排版均已编译通过。提供 initialStyle 时，同一个 `CreateLibrary`／`CreateTree` Plan 创建根控制文件、普通 `Styles/Styles.typ` 节点、`Styles/library-style.typ` 资源及资源 manifest，并把 WorkspaceSettings.defaultStyle 指向该资源的 EntityRef；出生、资源接纳、版本、历史和同步发布沿原事务，不再追加第二次样式写入。语言／地区只在该原生源内，不能新增 locale 镜像。

路径、完整 options 与原 operationId 在首次网络写入前保留。重试核对同一指纹，换 options 不复用原编号；已提交结果还须证明原库根物理身份，原路径上相同字节的新目录不获得原库资格。部分初始化只能按原 Plan 检查后续写：已归本操作的原对象和准确 before／after 才可继续，未知新增或第三态保留并进入恢复，不以标记或同名文件猜归属。

`cancel_library_creation_with_options_and_id` 接受准确原输入：已提交保持 committed；已登记但未完成的创建沿原恢复资格，只能保留可证 before 或继续核验原安装。未登记请求在原 journal 单次持久写入 `cancelPrepared` resolution，内含 typed `createLibrary` attempt 和准确原请求指纹，不制造作者 Plan 或参与者。无效原始路径只参与有界原请求指纹，不读取、写入、接纳、阻挡或发布该作者路径。重启和迟到创建识别同一决议，变输入被拒绝；状态未知或权限／I/O 错误不能冒终态。保留旧取消 Plan 的严格读取见下方原事实合同；它不是当前生产写入方式。

`RevisionId` 是冻结 Plan 时私有分配的随机标识。commit 后才有效，manifest 一次关联 EntityRef、RevisionId、exact blob、因果父版本与事务。它不依赖包含自身的源码摘要，能在最终 `.typ` 中构造 self/cross 固定地址和批注锚点。计划私有 ID 不作为公开成功；真正 no-op 保持版本，外部同字节接纳是新的来源接纳事实。

每个实际source RevisionId的admission/CommitManifest还固定 `SourceSchemaBasis=(SchemaRef, committed ConfigRevisionId)`，不是第二份parsed AST/Fields镜像。冷读取原metadata的短keys/types只在这个basis解析成永久Field/Type ID，再按当前schema明确的保ID兼容关系消费；不以current同名alias重新解释历史源。body-only或既有字段值修改默认继承原basis，只有明确schema/source migration或选择新schema的构造才重绑，并完整预览必要key/type词法变化。copy/import缺原basis不能猜同名是同一语义，须显式映射/报unknown与loss。

`ReadBasis` 是本次 Core 的当前资格，含持久版本或明确 external observation、observer incarnation、真实文件对象与变化 generation、lifecycle、schema/授权世代以及实际依赖。hash 只校验字节完整性，不证明对象身份、权限或条件写。watcher gap、重启、替换、ABA、partial materialization 与相关依赖变化使旧 basis 失效；重建后重新读取可以获得新 basis，不能复活旧 plan/selector 的写资格。

`DependencySet` 只有一套 typed object、lifecycle、structure/range、schema、authorization、environment 与 World read 类型。完整负范围和不存在也须从当前获权枚举证明；placeholder、I/O 错误、隐藏对象或未扫索引不是不存在。无关对象改变不使一个只依赖本篇的保存失效。需要全集/约束的强动作须证明实际范围，不能因为普通保存成功而声称全集完整。

普通获权原始读取可保留合法或无效 external bytes；external 未接纳状态不获得持久 typed 写地址。普通整源保存可在短时保护窗口核对同一 external Base 并显式接纳，不要求先编译整篇。保存语义独立区分 local_valid / semantic_pending；完整强动作的跨对象条件未证不能降级成功。

### 3.1 本地笔记时间线

本地笔记时间线列入当前 v0.5 本地补齐范围，按时间浏览同一笔记实际保留的已保存源码版本。Core 按稳定 NodeRef 与已提交版本事实关联历史；经 Core 管理的改名、移动仍属同一笔记，不按 path/title 合并另一笔记或外部同名文件。复用既有 Core 历史、准确字节和恢复机制，不另建版本库；未提交/未决 journal、Draft 与已保存版本分开显示，不能把恢复提案冒充已保存记录。业务 Query 的任务、事件等时间轴是另一领域。

列表显示实际保留范围、每条版本的保存时间依据和可用状态；缺保存时间明确标记“时间未知”，不把文件 mtime、扫描时间或 UUID 顺序冒充提交时间。未保留、缺失、不可读的历史准确标明，不能补取 latest 假作选中旧版，也不以列表为空声称从未保存。保留策略沿用 §§2/9.2 的历史与恢复责任，不承诺每次键入永久保存，或永久保留全部版本。

在现有提交记录中补充真实保存时间，并派生可重建的列表；旧记录缺时间则保持未知，不另设时间线账本。设备时钟只提供显示时间，同时间或时钟回拨不改变已证明的提交关系。本地范围为该库、该笔记已由 Core 保存或明确接纳且仍保留的版本；未接纳的外部修改、每次键入、未导入的其他设备历史不承诺收录。外部改名/移动须先由现有身份恢复流程证明连续性，无法证明则提示待处理，不能按路径或同名续接。

打开旧版只读，比较固定所选历史版本与本次读取的当前已保存版本。有未保存草稿时，须明确比较对象是“当前已保存版本”还是“当前草稿快照”，并保护草稿；浏览、比较不得替换编辑缓冲区或清除 Undo/Draft。历史语义读取沿用本节的原 SourceSchemaBasis；原配置或依赖缺失时报告相应预览/语义不可用，仍可只读查看可获权的准确源码。

恢复所选旧版作为一次新的普通保存提案，保留其后的历史，不回退/截断版本链或直接覆盖 current 文件。先明确保存、保留或合并未保存草稿，再以当前对象、当前 Base 和写资格冻结提案，沿用同一 Core 操作、幂等和并发变化防覆写规则。恢复响应 unknown 时查询原 operationId/原计划，不能重建操作盲目重试；历史可读不授予恢复写权限，未决恢复先按既有机制处理。

本地候选实现沿用原保存/事务提交记录中的 `savedAtUnixMs`、`commitSequence` 和原操作/版本证据派生时间线；版本键引用原保存操作，不新增历史账本。浏览器提供准确源码、与当前已保存内容或草稿快照的比较，并经原保存路径恢复；选择保护草稿时先保留当前缓冲和既有恢复稿，来源基线保持原样。恢复回执更新 Base；原脏稿或请求期间新输入继续保留，共享 Undo 不重建。所有提交后的错误均先对原 operationId 查状态，明确未登记的拒绝才可解除；回执应用/草稿持久化失败仍保留原操作。当前历史预览仅提供准确源码与文本差异，不用当前依赖冒充历史富预览，恢复后业务语义仍须重新验证。实际验收沿用 ROADMAP LHT01–06；多用户历史读取/恢复权限及同步展示留到 v0.6。

## 4. 冻结计划与准确结果

`prepare` 接受 operationId、typed Intent、Draft Base 和预算；Core 先授权实际潜在范围，再读取。计划保存实际 read/write set、完整 before、exact after blobs、身份/版本分配、结构捕获路径、约束、实际 transforms、schema 与用户可见预览。调用方 offset、ID、权限标记或摘要不成为证明。

SOURCE 以官方 AST 和原字节产生 narrow edits；EDITOR 负责 Draft 序列、选区、Undo 与安全可写子集；FEATURES 负责 Query、复制/模板 transfer 和业务后条件。Core 最终按同一 map 构造全部目标，重新 parse 完整 after，验证所有真实 typed references/固定版本/批注范围和预览一致。动态生成内容能阅读/查询不等于有可写位置。无法安全分析的 transfer 保原内容并明确不能复制相应语义，不按字符串替换未知代码。

一个对象键到最终 EntityRef 的 map 覆盖 fresh/copied/existing：每项明确 disposition 与来源/owner，不再维护三套不同结果数组。实际 change set 独立列出真正改变的源、结构、lifecycle、批注和 schema；映射到 existing 对象不证明它产生新版。fresh 的 before 是 absent，existing 字段缺席也不能伪造 absent Node。

operationId 在工作区的当前操作来源域内唯一；完整 canonical Intent 指纹固定。相同 ID/相同请求返回原状态或结果，换输入同 ID 冲突。prepare/stage 不是 saved。规划失败后保 Draft；新提案使用新操作 ID 并链接原稿，不让重试私下变换源、目标或分配。

## 5. 本地默认保存与后端合同

默认 Desktop 不要求每次 checkout。首发选择 Windows 本地 NTFS 的短时排他内容写入路径；后端能力必须按 volume/filter/path 实际验证，云占位、网络盘和别名不因同 API 名称而自动合格。现有文件内容更新与目录 namespace 操作使用不同保护方法。

**现有文件内容更新。** 后端以读写访问取得拒绝共享读/写/delete 的文件句柄，先证明 canonical containment、无 reparse/不可控硬链接别名、路径和对象绑定。通过同一个句柄读取完整 current，并与 Draft Base 和 Plan before 核对；保持句柄直到完整 after 写入、flush 和 durable commit。避免关闭保护句柄后再 hash/rename。外部编辑器已有写句柄则获得共享冲突：保 Draft、有限重试并说明文件忙；取得句柄后发现外部新稿则冲突，不覆盖。

Windows 的分享模式会检查现存访问并限制后续 write/delete，包含文档说明的可写 mapping 冲突；这使“同句柄核对后原位写入”成为可验证候选。它不能自动证明祖先目录不移动、filter 行为、目录子项不增加或真实断电耐久；必须分别覆盖。只用 byte-range/advisory 锁不足。[CreateFile 官方文档](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea)

原位完整写不是单文件崩溃原子：日志先保完整 before/after，崩溃可能留下 partial。Core reader barrier 阻止当前读取半稿，下一次启动先检查恢复；普通外部工具在 Core 中断后可能看到 partial，因此资料与恢复日志都须保留。换用 replace 的后端只有在排他保护不释放且实际 displaced bytes 可证明时合格，不能把 read-hash-rename 称 CAS。


### 5.1 目录对象的保护范围

1. 计划先固定Workspace/volume标识、source Ref/真实对象、源/目标父和basename，以及所需权限。通过已认证volume根解释相对路径，不用可变DOS drive alias重新解释。拒绝路径逃逸、未知mount、reparse/symlink和无法唯一识别的硬链接别名；目录对象也要核type。
2. 收集从受信volume/workspace根到source、source parent、target parent的全部目录链。支持的后端须说明workspace根上方定位为何稳定；不能把先前绝对路径字符串当根。每个物理目录对象只取一个guard，源/目标共享祖先复用，不重复打开相互冲突的DELETE句柄。
3. 先按路径角色算desired access，root→leaf取得。Windows目录handle必须包含实际`FILE_LIST_DIRECTORY`（read data）和`FILE_READ_ATTRIBUTES`，share=`READ|WRITE`而不shareDELETE；source将被移动时同一个guard在首次打开时包含desired DELETE。后端不能用仅属性访问假排他。不得后来重新开一个DELETE handle与自己的no-shareDELETE guard冲突；角色升级需先无副作用重规划。
4. 按相邻父handle的实际directory entry核child的volume/file object/type，再核child handle对象；核完整当前链和原 source binding。打开时`OPEN_REPARSE_POINT`并拒不获准重解析，后续操作按guard证明的实际父/名称执行。任何guard无法取得或链不等就释放全部、保Draft有限重试/冲突；不丢保护后仍沿旧字符串写。
5. 目标名称是prospective，不靠先前扫描证明空。最终create/rename使用原子create-if-absent/no-replace；名称被抢占就冲突，两边保。目录guard不阻止普通child新增；它稳定的是目录本身及其祖先定位。

完整 namespace 保护须连续保持到实际操作／验证／durable commit 完成。通过source自己有DELETE权限的同一handle执行rename，不关闭source guard后按path移动；目标父guard保留。只有已完成最终移动的 source 可以使用 §5.2 的受保护 guard 交接。若API需要目标父handle相对名称则用实际父handle；已验证的volume路径API可在整条链被保护时使用。后端需分别验证该API组合，不从SQL锁推导。

### 5.2 重叠、移动和Node改名

普通同卷move是一项已绑定目录对象的no-replace rename，basename保持；移动前新增child/外部修改随同该实际树，移动后变动是新external observation。Receipt只证明placement，不证明全树固定bytes或发source RevisionId；Trash/restore同样把原目录对象移至唯一Trash批次/获权空位置，保原树/字节和恢复元数据，不递归删内容。

destination parent等于source对象或在source后代链中时拒绝，不能递归搬进自己。source/target实际同位置是qualified no-op；同名另一对象不是no-op。共享父只一个guard。源/目标卷不同进入明确copy+有条件Trash组合，不把跨卷包装成同对象rename。

Node `A/A.typ→B/B.typ`仍两步：为目录与canonical leaf保存准确对象/名称/恢复责任，leaf rename no-replace后关闭妨碍outer rename的leaf handle；目录guard和完整祖先/目标父闭包保持。outer rename后重新取得实际canonical file guard，验证原对象/控制角色和公开声明的外部改变。新leaf/新root第三态只pending；no-replace回退绝不覆盖。这个释放leaf的窗口不持久授予旧source写资格，也不声称全树内容固定。

Windows 同目录 leaf 改名使用原文件 handle、简单 basename 与 `FILE_RENAME_INFORMATION.RootDirectory = NULL`；不能把已持有 DELETE 权限的源目录 guard 再作为相对目标，造成内部重新打开目录时的共享冲突。跨目录移动仍使用已保留的实际目标父 handle，完整目录闭包和 no-replace 保证继续有效。参见 [Windows 改名调用约定](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)。

Compound root 已到达原计划最终位置，且将作为另一参与者的目标父目录时，不再需要 DELETE 权限。在重新打开成员前，从仍持有的原 source handle 相对打开原 canonical leaf，使用普通 READ／share0 并核文件身份。保持该 leaf 与完整祖先／目标父 guards，再关闭 source DELETE handle；通过已持有的实际最终父 handle 和最终 basename，重新取得 LIST_DIRECTORY／READ_ATTRIBUTES、share READ|WRITE 且不 share DELETE 的目录 guard。核原目录身份并按捕获时的 source key 回填后，才释放 leaf。打开的 canonical 文件在交接期间阻止其父目录改名。witness 缺失、busy、身份不符或目录 guard 重取失败时立即停止后续写入，保留原 Plan 的 pending 责任；before／leaf-renamed／parked root 仍保留 DELETE。恢复对已经 final 的 root 也须在捕获文件前交接，覆盖父已 final、子仍 parked 的状态。后续完整成员、文件身份和 before／after 内容检查继续执行。

后代的历史／Query 身份续接还须有原 Relocate 内的准确成员位置证据：从已有出生／目录接纳事实选出相关成员，记录相对位置、稳定 ID、实际文件／目录对象及父对象，不扫描私有正文，也不另建位置账本。这是位置证据，不给普通 move 增加全树字节 CAS；合法晚改和新增仍随原树保留。执行及原操作恢复必须核对成员实际对象，再承认相应位置延续。只有旧路径前缀的祖先移动日志不能证明后代随行；另一对象占用旧地址后的移动不得被原后代借用，无法证明时明确身份待处理。

普通source save也使用文件的父/祖先闭包加该文件RW share0 guard，在同一文件handle核Base并写/flush/commit。可捕获的ancestor/target-parent改变不能在最后检查后绕到另一库；busy保稿重试。所有文件guard的范围有限，不要求全库锁。

### 5.3 copy与template：固定snapshot成员，不虚构私有目录隔离

默认普通Windows后端不把同用户“private文件夹”当隔离。Plan固定源snapshot成员和exact blobs、一个fresh map、所有目标目录/文件及预算。源snapshot是逻辑cut的已接纳版本或明确获权选中观察；若要求完整subtree而成员未证则coverage/pending，不能悄悄少copy。

托管 copy/template 的完整子树读取与导出由 Server 在捕获前分别授权。Core 的不透明 FeatureTreeWitness 固定准确所有者、源树文件／目录成员、物理身份、父对象、字节和原 Schema／根控制文件；成员和保留字节有界。传输 inventory 仅从该已获权 cut 和显式 inputs 的原获权上下文建立，不调用全库 inventory 读取其他正文，也不将目标父对象的命名空间证据当成引用读取授权。没有证明的普通外部路径链接保原值并报告复制损失；需要目标证明的 typed ref 缺少该上下文时拒绝 copyRef，不能仅凭稳定引用读取目标。原 Plan 仍承担保存、提交和恢复，见证本身不授予新权限。

托管目的能力由原 FeatureOperation 和 FrozenOperation 的同一个闭合 `HostedDestination` 绑定：Fixed 固定槽 ID；UniqueChild 另固定真实父目录 FileIdentity／WorkspaceId、prefix、maxMembers 和原外部 operationId。固定槽的新建／复制／模板 Birth 或改名／移动／恢复 Relocate 身份继续确认准确预留位置。UniqueChild 只接 NewNode／TemplateNode，以 prefix＋原号 no-replace 命名，不把 receive 扩给其他结构动作。其成员从同一原 Plan/Birth 派生；准备和提交都核包含 unavailable 及旧 prefix 的保留成员额度，读取另核原 Birth 的 FileIdentity。所选原 commit／Birth 证据损坏、读失败或超预算须 unavailable，不能当空集合或零成员；不建立额外成员账本、不按目录同名收养。本地 producer 没有 hosted destination，不能取得预授权；读取所有结果仍须 Server 当前独立读取授权。

安装算法：

1. 在操作日志及exact blobs已durable后取得§5.1目标祖先/父闭包和Core范围reader barrier。预算预留计划文件数对应的同时handle容量；不足在安装前失败/显式改批，不先半写。
2. 一个后端调用create-only建立计划目录并返回其原guard，不采用`mkdir → reopen`。Windows候选使用[NtCreateFile](https://learn.microsoft.com/zh-cn/windows/win32/api/winternl/nf-winternl-ntcreatefile)：`RootDirectory`是已持target-parent handle，name仅单个相对目录分量；`FILE_CREATE`、`FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT`，LIST_DIRECTORY/READ_ATTRIBUTES/DELETE/SYNCHRONIZE access，share READ|WRITE而不shareDELETE。要求成功且`IO_STATUS_BLOCK.Information=FILE_CREATED`，日志登记返回对象证据并持续保留同handle加入闭包。已有同名目录冲突，不在其中合并。初始workspace root也对获权外部parent使用同一调用。普通外部工具可能先看见目录，Core reader仅见incomplete/installing。有限本机NTFS probe已验证返回创建handle、同名collision与外部root rename被拒；生产平台/耐久资格仍待验。Linux受控服务档位仅在已接纳OS隔离namespace和原writer fence连续持有时，允许`mkdirat → constrained open`作为等价原语；先同步parent，再持有新原对象并在决议前登记opaque身份。未隔离的mkdir/reopen不能伪造创建归属。
3. 逐个`CreateNEW`创建计划文件，RW share0，以创建返回的同handle写准确after、设置精确长度、flush并回读。所有这些文件handle保持到commit，不关闭再hash/replace。目标filename被外部抢占则冲突，不替换；新对象的本操作归属直接来自create-only返回handle和journal。
4. 校验全部计划目标、fresh identity/ref map与constraint。复制要求的每个snapshot对象都已创建，才能commit。纯copy的after是该固定成员集，不要求排除目标目录里其他人的新raw文件。未知新增是独立external对象，不记成本次copied result、不授ID/typed资格，也不删除；交付可以标`external_pending`，需要排他全集后条件则此次strong intent不满足。
5. durable marker固定所有计划对象/receipt后释放文件/目录guard。Core重新扫描实际树时保新增外部资料，原snapshot Receipt保持；不能因外部额外file而把缺少计划目标的copy洗成成功。

同步只运输本操作准确copied changes/births与对应blobs，不运输“recursive target tree覆盖”的指令。接收端同样create-only并验证本地已有身份/对象；外部target附加rawfile不自动进入manifest，须单独observe/admit。目标占用或identity冲突不能靠重发ID解决。

失败/rollback只针对本计划明确创建、仍归本操作的对象。文件third-state保全；目录通过handle删除也必须实际empty。未知新增令删除目录失败，则保完整实际目标树并给明确pending恢复入口，不递归清理。已创建多个root的template同规则；只部分完成不报整项成功。这个默认不依赖隐藏DACL/service权限，也不承诺外部观察多文件原子。

### 5.4 purge：固定确认集 + 删除空目录的实际原语

purge仍是获权破坏性动作；root和确认的bytes/成员固定，ignored/unmanaged边界事前拒。普通本地Tree不宣称无raw新增，采用如下保全算法，而不是要求“受控staging”凭名字成立：

1. 在实际Trash tree取得完整目录guard闭包；保存显式确认的Ref/file object/bytes及必要恢复日志。先取得全部确认file的RW|DELETE share0 guard；任何existing writer/mapping/busy失败时不删一个成员，释放/保留待重试。已知子目录身份亦在guard集中。
2. journal durable installing后，通过这些明确对象handle设置disposition并按平台合同flush/关闭；不按可能被替换的path递归删除。每个已删除对象的真实状态和原ID恢复责任记日志；commit前仍保必要before恢复资料，不把半删当成功。
3. 自底向上只对原确认目录handle执行`delete-if-empty`。新raw child使OS返回directory-not-empty，原child保在Trash实际树；未知child不进入删除集。目录guard稳定目录本身，原子empty检查由删除原语完成，而非`list empty→recursive delete`。
4. 所有原请求目录已删除、tombstone与原操作责任完整后才commit purge成功。某目录非空/busy则pending_reconciliation，报告已经处理的明确对象与实际残留，原ID不变；不报告whole-tree已清空。恢复只履行原明确集合或no-replace重建其可证明对象；未知残留不删、不覆盖。

这允许静止正常本地Trash完成purge；遇到并发新稿则保全并等待明确新决定。它没有排他全namespace承诺。真正需要原子erase全树/业务strong negative范围的部署使用有独立OS权限/fencing的Server writer，不从同用户ACL名字推导。

已知ignored/unmanaged边界仍在操作前拒且扫描不重入；开放树late boundary在rename后、commit前发现时保真实树并pending，不覆盖原路径第三对象；commit之后的新boundary是后续external observation，不能撤销原历史Receipt或重执行该move。若要求在任意raw新增竞态下瞬间不存在boundary/唯一值，该意图是强negative-scope，不能由本地目录guards冒充。普通同对象move、固定snapshot copy与安全purge各有自己的明确完成条件，不因此全不可用。

**合格文件系统元数据 v2。** 当前 Core 存储档位为 `weftext.qualified-fs.v2`，与原生导出包版本独立。物理身份与保留目录项元数据均有明确平台标签：Windows 保留真实 volume／file-index／creation 证据；Linux 使用合格文件系统实际 FSID 与 `name_to_handle_at` opaque handle，不能用 inode／ctime 元组或伪造 Windows 属性位。旧无标签物理记录明确拒绝，原件不重写。作者内容只通过显式导入／接纳进入另一物理副本；历史 Windows 证据不能授予 Linux 连续性。

**Linux 受控服务档位。** `Core::with_backend` 接收显式接纳的 `BackendContext`；普通 `Core::new` 不自动授予任意 Linux 路径写权。接纳通过无符号链接、不可由服务或其他用户写入的祖先目录，读取有界且 root 管理员拥有的 enrollment，核查专属非登录 service UID、不同的 client／worker UID、准确 real／effective／saved 身份、无 capabilities／supplementary groups、原 root opaque 身份、私有服务所有权和有界完整现存树。管理员拥有的 root 父目录阻止普通客户端／worker 替换根目录项。对真实 root FD 的锁仅协调不同私有 data-dir 的服务实例，不是 OS 写隔离机制。客户端／worker 必须以各自不同的 OS 身份运行，不能继承服务描述符或凭据；调用者自报 API 参数和同 UID 锁都不构成该部署。

Linux 的 `BoundFile` 保留 root／parent FD 与准确 basename；`openat2` 使用 BENEATH／NO_SYMLINKS／NO_MAGICLINKS／NO_XDEV，包括拒绝 bind mount。原生 ext4／XFS 须实际支持 opaque handles；不合格文件系统、非 UTF-8 名、符号链接、硬链接和特殊项明确失败。枚举真实目录 FD 并返回 complete／budget 结果。移动使用 `renameat2(RENAME_NOREPLACE)`，同步受影响的两个目录；私有替换明确声明 replace。文件同步和目录项同步为不同耐久步骤。source／read 范围在已隔离的服务权限下持有相容的进程内 lease。机制仍需逐平台产品检查，不建立真实断电、桌面、Mobile 或 macOS 验收。

托管编译 broker 绑定已准入 Core 后端持有的同一 `ValidatedEnrollment`；每次启动固定原登记路径、身份和完整摘要，另读同一路径不能替换 authority。实例的单一启动许可由已接受任务持有至原 worker 进程树终局回执，HTTP 调用方断开也不释放；占用时在启动前拒绝，启动／清理未知则关闭后续 worker 准入且不能宣称干净排空。

**Server 与后续平台。** Server writer service 使用 OS 隔离目录，普通客户端和 worker 不直写；fence 每次实际文件动作和日志决议。该部署可实现强 namespace 与自动恢复。Linux 的普通 advisory 锁无法阻止任意同用户工具；Linux 首轮可提供受控服务目录或明确的独占宿主模式，开放本地写后端待单独证明。网络文件系统逐项认证。可选 checkout→三方导回服务于长期外部编辑或不合格后端，保留原 Base/current/proposed；不是正常 Desktop 保存的强制步骤。

不需要抵抗磁盘所有者刻意绕过隔离、直接损坏日志或恶意内核；普通编辑器/同步器竞态、已有句柄和 mapping 在需验证范围内。范围外行为不产生声称“任意外部写都安全”的承诺。无所需保护时保稿并返回该动作的明确边界，不暗降级到覆盖未观察稿件的 observed-only save。

## 6. 可执行提交与恢复算法

1. 先查原operation与完整canonical Intent指纹；同ID/同请求只resume/replay，同ID换输入拒绝。durable保存固定计划、真实对象、恢复责任、必要before/after blobs和预算；Draft保全不等于saved。
2. 取得§5目录保护闭包、实际文件guards与Core作用范围reader barrier，验证绑定、权限、schema、lifecycle和真正依赖。取得失败保稿/无变化有限重试。此步不rename。ordinary move不冻结child bytes；copy固定snapshot成员，purge固定确认对象，强业务需完整范围。
3. 同journal事务保存完整frozen Plan与installing并flush，之后才执行任何rename/create/delete。SQLite只证明日志决议，不证明外部文件安装。
4. 依§5执行同对象move、Node两步rename、create-only固定输出或明确对象disposition。byte writer以持有handle写/flush/回读；namespace操作核同对象/目标与目录项耐久。步骤日志不代替归属证明。busy/磁盘满/失权/取消/第三态停止新步骤并保原ID恢复责任。
5. 在barrier内验证原Intent完整结果和当前授权/依赖，再一次durable marker保存transaction、实际有效RevisionId、完整Receipt、sourceSchemaBasis、逻辑generation/cut facts、作者Job步骤与publication outbox。move只证明placement且不发source版；copy须全部固定目标，purge须全部原请求目录实际消失；部分输出/残留不报子集成功。之后释放guards并异步index/portable publication。manifest pending不撤回saved。
6. unknown通过operationStatus查原ID，不重新创建/分配/收费。同步published、job完成、HTTP200、Draft retained与saved分别报告。

Core reader获得多对象逻辑提交原子可见性；普通外部工具可看到多步骤partial，不获多文件瞬时原子承诺。journal committed到portable publication之间，真实资格由原journal及完整blobs提供；启动/backup一致包含该窗口。

恢复先查原durable决议，再判真实对象为before/after/third/unavailable；需要准确对象、安装归属和bytes，不凭相同hash/file-ID补连续性。Server连续fenced writer可履约原计划；开放local crash释放guards后归属不明则先留current残留/before/after、明确repair/merge，不自动覆盖。copy/initialize rollback不删unknown子项，purge恢复不扩大确认集；目录只delete-if-empty或no-replace返回。已committed原Receipt不受后来r6使r5业务basis stale影响，仅当前授权过滤交付。失权不把已成事实变待重执行；无关作用范围继续。Undo是当前资格的新Plan。

## 7. 同所有者同步与冲突

实现进度（2026-10-14，alpha.1 之后的开发代码）：Core 已加入有界的 manifest/blob 因果验证与私有持久暂存。配对固定 WorkspaceId 和准确 genesis；摘要覆盖完整严格解析的 manifest。笔记源、批注、所属资源与位置组成同一冲突单元，schema/settings/tags 和目录分别有内容角色；任何角色均不承载权限或作业执行资格。接收端检查完整 parents、before revisions 和语义依赖，缺内容不接纳为 head；并发同对象保各版本与共同 base。按选中对象导出完整因果／事务闭包，保冲突双方，不要求无关对象的内容；producer 应按真实依赖选择 parents，不能让每次编辑都依赖全库 frontier。当前硬上限为 2,000 commits、每 manifest 4 MiB／5,000 文件、单 blob 64 MiB、每 store 元数据与 blob 合计 256 MiB，超限明确失败，不截成“完整”。已做内核乱序、冲突、部分选择、损坏、原请求重试和重启测试。

作者提交的待发布 manifest 与准确 blobs 现存入原 Core Plan，由原 commit marker 决定可否传输；传输失败不撤销 saved，恢复后发布原操作，不读取后来的文件冒充旧提交。初始绑定记录准确观察基线；当前已接源保存、新建、源／批注事务及已有位置改动的有界路径。新副本通过同一 CreateTree 事务只创建不存在的目标，保 WorkspaceId／NodeId／ResourceId，重新登记接收端实际文件、目录与标签资格；独立私有状态、replicaId 和草稿不从发送端复制。结构 parent 使用带类型的 Node／Directory 引用，库根书签有明确的 RootDirectory 角色。已测两个实际工作目录的独立离线保存、双方冲突保留、资源／书签／标签登记、准确计划重试、提交 marker 缺失后的原对象接纳，以及备份保留同步历史和待发布记录。

现有副本已接 `ApplySync` 内容更新与 `MergeSync` 显式解决：同一原计划冻结接收版本、实际本地 before 和准确 after，复用 Update 的全部文件／父目录 guard 与原回执恢复。源、批注和已有 Node 的新增／更新资源一起提交；未接纳过的跨对象事务保完整闭包。安装接收版本不再发布成一次新编辑，后续本地保存使用实际已安装的因果版本。预览后本地提交、外部字节或接收分支变化均拒绝旧计划；现有会话草稿继续保留。合并必须列出当时全部冲突 revisions，保双方旧 blobs；无关对象的冲突不阻塞独立内容更新。标签更新只在原受保护 registry 文件上登记接收身份，同字节替换文件仍不继承资格。同步管理计划的读取、执行和恢复均限本地所有者。

Core 的目录传输方法现可将选定完整闭包写入独立目录的 `identity.json`、`manifests/`、`blobs/`，或按显式 WorkspaceId／genesis 配对接收。准确记录只创建、重试先比较；损坏记录、错误配对、未配对用户目录，以及本机工作库／私有状态重叠目录均拒绝使用，不覆写修复。接收只进入暂存，不改作者文件；独立副本离线保存后经同一目录往返，再显式计划更新原副本，已用实际临时工作目录验证。传输不含设备草稿、权限与作业记录，也不把选择闭包声称为整库覆盖。

部分副本现在可经同一 `ApplySync` 计划补齐缺失的 Node／Directory／根书签／配置，并同时更新已有对象。原 CreateTree 增加受保护的已有库根模式：新目录与文件仍只创建不存在的目标，已存在的父目录冻结实际身份；先检查所有写入对象和缺席目标，再建立一次原操作 install 记录，最后只作一个 commit 决定。新建的原目录 handle 持续持有到提交，现有会话仅在实际受影响的路径失效。嵌套笔记与资源子目录可一起补齐；占用目标不会先写已有稿件。完整 after images 的恢复继续要求原物理对象；不重发结果未知的目录操作。同步后的实际安装集合与用户选择分别报告，仍不声称远端全库完整。

manifest 的每个文件现在必须同时携带准确 blob 摘要与 `fileRevisions` 内容修订编号。它与对象冲突单元 revision、本机 session／ReadBasis 分开。发送端冻结实际源保存回执、事务 revision 或已接纳观察编号；直接新建笔记先把编号存进原 Plan，再写文件与观察。克隆／更新端以接收编号记录新本机对象的内容观察，既有批注 JSON 原字节不改。拒绝把同一文件修订编号重新用于其他内容或所有者。普通保存回执另报 `revisionId`；无内容变更的保存保留原内容编号，不能把新打开会话的 baseId 当成一次源码修订。Query 定义保存消费者改读该内容编号。显式选择同字节分支时保所选编号，当前观察按本机 durable commitSequence 决定，不按 HashMap 或时钟。已测真实批注引擎在克隆、更新和重启后仍返回 exact，并核对资源修订与已安装源码历史读取。此字段属于未发布的同步开发格式；缺字段的旧试验记录不自动推断编号。

接收端现可用原 Relocate 事务应用单个 Node 子树的改名、移动、回收与恢复，包括携带未单独选中的本机子节点。该同一计划可同时更新正文、批注、现有资源，创建已有父目录下的新批注／资源文件，并更新事务中未移动的其他对象；不能先提交目录移动再另外保存内容。实际文件、Node／资源／目录身份、源树成员、目标父目录及未移动的依赖先核对；移动用保留的原目录 handle，提交前再从该 handle 核对完整结果并持有全部文件。接收端本机伴随文件保留，不因发送端未携带而删除；子节点的本机安装位置一起改变，不制造独立内容提交。已安装源码修订、草稿找回和历史连续保留；历史入口同时认可原 CreateLibrary 与 CloneSync 的库资格。接收回收树进入原回收站列表与 Query 范围，可以在接收端恢复后继续同步。完整 after images 的原操作恢复核对全部子树与依赖，同字节替换对象或外部第三状态不能接纳。只有原目录移动已全部成立、每个内容参与者仍是原 before／after 对象时，才允许 `resumeOriginalPlan` 补完内容；仍使用原 operationId、plan 和最终提交决定，不重发 namespace 步骤。目录尚未完成、无法确认或遇到第三版内容时保留原计划及现场。并发改名仍须显式选择全部冲突分支，历史保双方。

Core 现将多个已安装 Node／Directory 的移动、改名和内容更新组成一个内部 Compound mutation，并可组合 Node 的回收／恢复，仍沿原 `ApplySync`／`MergeSync` 的 request、Plan、install、commit 和回执。原计划可表达交换／循环重排、祖先抽取及嵌套最终位置；最终 parent 图仍须闭合且无环，原对象和中间位置必须可证明，不能把任意重叠视为已获准。可同时更新树外参与者及新增资源／批注文件。首效前核对全部原物理身份、完整成员、内容、目标资格与依赖；共享祖先 guard 去重，原 root 与重新打开的成员保护持续至提交结束；只有已完成最终移动的 parent 使用 §5.2 的连续 guard 交接。恢复按原 namespace phase 聚合证明原位、主文件改名、parking 和最终位置，沿同一 Plan 只续做剩余步骤。第三状态、同字节替换或同步基线变化保留现场并拒绝续交，已提交回执不因后来版本变化而失效。参与者与内容逐项计入有界预算；历史、草稿、安装事实及 Pending 覆盖全部参与者，普通本地单树移动的原语义不变。

该 Compound 计划进一步允许部分副本补入尚未安装的根 Rules、Schema 与 settings 文件，在移动树内外创建新 Node／普通 Directory 父节点和自有资源嵌套目录，并应用普通 Directory 的独立移动。全部新目标先证明缺席并冻结已有父目录身份；首次创建沿原 materialized 记录保留物理身份、发送端准确文件修订和 Node／Resource／Directory 身份，并接入原出生、历史与恢复消费者。创建与各树移动、内容更新仍只有一个最终提交，恢复不能把同字节的新文件当成本操作原件。曾提交后丢失的 Rules 仍不可当作首次缺席自动补建。首效前和恢复时，Pending 覆盖原位、最终位、parking 和未物化子目标；名称交叠与别名按宿主文件系统比较规则核对。

所选 after 从已安装 Node 的资源 registry 准确移除原 ResourceId，且从 after 文件集中移除其准确相对叶路径时，原 Compound 可同时保全该原叶文件；不将正文、批注、控制文件或未登记伴随文件冒作资源退役。计划冻结原 Resource 资格、文件修订、digest、FI、父目录 FI、字节与 attributes，在同一 namespace guard 中持有带 DELETE 权的原文件 handle，以 no-replace rename 移入该原 Plan 固定的成员保全位置。支持没有目录 placement 的纯资源成员退役，也可与多所有者、移动和内容更新合成；不分配新的 Node／Directory 身份，不另起删除事务，不清空字节或递归删除。旧 Ref 与历史 blob 保留，但不因此继续具有 current Resource 角色。原 source 与 retention target 均进入 Pending；恢复只接纳原 phase 下来源位置与固定保全位置恰有一处持有同一原对象，并核原父／本操作 materialized 保全父、准确字节及 attributes。两处皆有、皆无、外来占位或同字节替换均保现场拒绝。

带 live／Trash 成员依赖的原操作恢复，按原 Plan 捕获的 FI、父目录与 namespace phase 将本操作移动的准确成员逆投至 before lifecycle，再比较原成员集；不能按当前路径或 portable retired 声明猜测旧范围。只有同一 Plan 的出生记录与 materialized FI、当前原对象均匹配时，才从该 before 比较中扣除其自行新建成员。迟到外来成员不被扣除，仍使原范围失效。

同步启用后的 Native 导入沿同一 CreateTree 为外层包装目录及其普通祖先登记 Directory 身份，并在原预览中冻结全部输出文件的修订。Native Resource／Style 不得使用保留的 `weftext.directory.json` 控制文件名；旧计划中的资源也不因文件名匹配而在提交或恢复后获得导航目录资格。

同步后，已打开的编辑器保留个人草稿、撤销记录与原保存依据。同一 Node 的新位置经 Core 确认后，旧会话的预览、正文 Query 和批注刷新暂停，并清除旧派生结果；源码编辑和资料库 Query 仍可继续。已有待提交文字先按原输入状态结算，暂停读取不撤销其输入资格。用户明确比较原始版本、当前磁盘与个人草稿并应用合并后，才接纳新的文件会话并恢复这些消费者；合并本身仍不保存作者文件。

本地浏览器已接目录传输、配对、部分副本、冲突选择及原操作恢复；接收但未安装的准确历史与外部修改采纳入口也已接入，后者边界见下文。Clone 可保所选回收根与继承子树；同一原操作的多个回收根按准确 NodeId 选择恢复，缺少唯一选择时拒绝猜测。上述复杂 placement、回收与资源成员保全沿原事务接通，本轮既定 Windows／NTFS 范围已通过同一最终优化程序的 HTTP／浏览器组合验收。普通 ApplySync／MergeSync 对已变化的语义依赖仍拒绝，须转入显式 RevalidateSync，不能用确认框绕过或拆开事务冒成功；未获原角色／完整结构证明的其他文件移除仍拒绝并保接收版本。Server 消费者各按独立权限验证，所有者测试不代替其授权。新副本是选定对象及完整事务／依赖闭包的部分副本；既定 Windows／NTFS v0.6 范围已完成，尚未正式发布；未跑平台不在此接受范围。

managed 同步传输一个 immutable `CommitManifest + exact blobs` bundle：WorkspaceId、事务、因果 parents、writer replica、所有真实出生/改动/退休、before/after RevisionId、结构/schema 改变与准确 transform。同一 manifest 的校验摘要覆盖整个对象；校验失败/缺 blob/placeholder/父历史缺失只进入 staging，不成为新 head。

默认信任自己的设备与传输账户，摘要用于完整性；不声称抵抗 malicious transport。若部署承诺不可信 transport/撤销设备防护，采用一个登记设备身份与一个签名 commit 域，签完整 manifest；历史按当时有效登记/撤销因果 cut 验证，冲突撤销不能被选择另一 branch 抹除。无需两个专用签名 profile、CP/Notice 和每个 revision 的独立 seal。

同步器运输 bundle，不直接按到达顺序写 current 作者树。接收端验证完整身份、同 Workspace、因果闭包和本模式信任，再由 Core Plan 安装；本地观察与授权重新建立，不复制发送端 ReadBasis/编译许可。同步目标选择用户指定 transport 目录；运行工作树与它分离，仍是普通可编辑/备份文件。

普通云盘把整个 raw 工作树直接同步时视为外部改动，保原文件与当前已提交 blobs，partial 和重复 ID 明确诊断；不把它冒称上述 managed 因果同步，不让 provider 的覆盖决定 winner。用户可直接编辑工作树；同 owner 多设备的可靠并发应使用 bundle transport。尚未被任何设备保存/观察且已被 provider 删除的 raw bytes 无法凭空恢复，不能承诺保全。

单一因果后继按实际依赖 fast-forward；独立对象的并发更改可由确定 Plan 合成，合成 manifest 保所有 parents。涉及同源、移动/编辑、Trash/编辑、schema 或受约束范围的并发保各 head 与共同 Base，明确 merge/choose/fork；不按 mtime/到达/digest/文件名自动选胜者，不随机重发身份。冲突对象可看获权分支并保 Draft，写解决 Plan 前需重新资格；无关对象继续。

portable 文件复制绝不复制 Approval 次数、费用、provider unknown 和调度执行资格；这些只在连续 fenced Job execution store 中。即使复制完整资料库也只能开始普通内容副本，不能自动继续外部作业。

## 8. Server 认证、披露与窄字段

实现进度（2026-10-14）：`0.6.0-alpha.1` 接入受控单 writer 的源码切片，实际入口与边界见 [SERVER](../SERVER.zh-CN.md)。服务端配置选定可读写文件及明确依赖，Bearer 凭据在服务端映射 actor；按 actor 分离 Draft / 操作记录，复用同一 Core 文件提交。未知保存的持久结束记录阻止迟到原请求在后续新提案前提交；已提交 / 待恢复事实优先。用户授权代次与准确下载票据绑定，预览后重新授权，单块交付与撤权决议序列化。此进度不建立本节窄字段、全消费者、故障切换以及 §7 托管同步已实现的结论。

Server 由受信部署选择工作区、schema 与授权策略；认证 actor/session/当前授权世代在实际服务端产生。客户端源 metadata、UUID、类型、地址、指纹和历史 manifest 均不授予 hosted 权限。资源、源、字段、批注摘要、批注目标及历史分别授权，deny 优先。磁盘所有者本来能读取的本地文件不受应用 ACL 保密；共享 raw 可写目录也不能获得 Server 级隔离。

撤权与提交/交付在 Server 边界线性化：源/字段读取、Query页、缓存命中、编译输出、预览、缩略图、下载每次 chunk 与最终 commit 重验。已经交付的字节不能撤回，撤权后不继续交付。受保护审计保准确 actor/意图/结果，不向无权调用方泄漏标题、路径、数量、存在与失败差异。Receipt 已 committed 可仍不披露；不可披露不改变事实。

多个用户保各自 Draft；同文档 A 保存 r6 后，B 基于 r5 的 Plan 进入冲突并保双方稿；不同文档不因一个全局序号变化全部失效。Server failover fence 必须实际保护 journal 和每一次文件写/rename；只有 SQL lease 不够。推荐单 writer service 持有 OS 文件权限，Server 实例发带当前 fencing generation 的操作，旧实例无直接路径写权；写服务切换先隔离旧进程/卷访问，再启新 holder。

### 8.1 可决定字段资格与简单完整源路径

SOURCE的producer在后台/获权管理动作中用官方AST认证唯一direct-root作者声明、该位置标准metadata/constructor binding、selected Field静态原生值与准确range，以及实际受影响获权schema约束闭包。runtime/include/generated/nested metadata不成为控制或字段authority，body其它Label不自动成为第二作者根。源结构必须能可靠识别直接声明；可公开的控制资格与身份不按隐藏值猜测。

body允许任意合法Typst函数、query、show和动态表达式；Field-only请求不求值/编译它们，也不校验未修改隐藏Field。body读取公开Field改变隐藏派生显示，并不修改body作者bytes，不构成额外body写权。若真正的schema约束依赖未授权Field/资源，应先取得所需授权或统一不可用，不能在服务内部读秘密后让其值决定公开validation分支。

producer公布轻量FieldAccessProfile：内部绑定当前exact sourceRevision、SourceSchemaBasis/current兼容schema、声明/range/局部patch证明和allowedFields；公开仅获权Field定义、opaque basis及必要资格状态，不交body/offset/长度/trace。controlled_field_write明确允许使用这项公布资格，不由裸metadata字符串自授。服务端所有source修改经fenced Core；提交取得当前sourceRevision和profile、schema/授权，读获权Field preimage，独立解析拟议static native表达式，验证仅本字段及明示已获权输入的constraint，精确splice，非选bytes逐字保留。新值不能引入binding、spread或Label改变作者角色。只证明本次selected Field/获权constraint闭包，不把未改hidden Field的无效或pending状态洗成整篇complete，也不将其验证错误回传该请求。

资源约束只消费Core对本次显式typed ResourceRef inputs提供的获权MetaCell，先分别授权再读取metadata；不给ambient全库扫描或隐藏资源存在/owner差异。完整sourceRevision与private range是内部current-write fence，不是公开opaque FieldBasis比较键。hidden-only源变化时，fenced service内部重新认证当前direct声明并重新定位selected Field，只比较调用方获权Field前像、受影响获权约束与公开profile语义；这些未变则在当前源精确splice并保当前hidden body，不仅因full sourceRevision不同向窄调用报public stale。获权Field/语义变化才报相应公开冲突；资格撤回仅披露已获权资格/lifecycle元状态或统一不可用。不得复用旧offset，重验不执行hidden body或验证未受影响hidden Field。异步Job可吸收内部重试，不公开隐藏revision计数或重试细节；Field-only Receipt只返获权Field结果和统一operation状态，不交full manifest/源或body派生诊断。

普通full-source获权用户的字段面板仍走共享Draft/Undo与准确AST局部patch→普通same-source save，不要求额外profile、固定容量/padding、全body白名单或另一作者语言。窄Field服务也不设这些作者格式限制。源拼接/IO留在可信Server内，可异步交付而不报告size/细进度；保证hidden code/value不驱动公开语义错误/分支，不宣称普通CPU/IO完美constant-time。剩余统计侧信道按实际威胁/部署测量，不能仅为旧口号增加空间填充。

第一条窄权限安全路径是Server fenced writer对内部rebase后current sourceRevision的核验。开放local缺可信变化generation时不能由watcher未通知、同file-ID或Field bytes相同猜hidden源未变，Field-only需renewed qualification或明确不可用；full-source普通字段面板/save继续。body变化不必使Field静态数据本身无效，但旧range/source绑定不能跳过重验；重新producer只审核direct声明/字段及约束闭包，不执行隐藏body。

仅 Field-only API 的客户不获全 manifest/全库同步权限。浏览导航/Query、摘要缩略图和回收站始终遵守当前披露；Trash 获权查询只读，不授予 restore/write。结果 handle/cursor 固定 audience、定义/参数/schema、实际依赖与完整性，不成为作者权限。

## 9. 授权编译、查询与作业

`Environment` 一次绑定官方 compiler/syntax/stdlib、语义桥 build、target、冻结 date、包/字体锁、资源策略与硬预算。库默认样式按 [EDITOR](EDITOR.zh-CN.md) §8：唯一样式内容是获权普通 `.typ` module，可为真实ResourceRef或NodeRef；Core设置只选该EntityRef及启用。compile透明生成entry（import/style show/include真实note），entry无身份，真实note仍是SourceSnapshot与作者ID/字段权威。默认初始化若启用库样式，实际构造`Styles/Styles.typ` managed Node（同计划fresh NodeId/SourceSchemaBasis）与其owned Resource `Styles/library-style.typ`，在`Styles/weftext.resources.json`登记ResourceId/文件绑定；settings只选择该真实ResourceRef并enable。module是普通Typst函数文件，renderer不扫描root path认身份。root的`library-style.typ`讲解示例不能冒Node-owned ResourceRef；用户另选module须先由Core登记为真实资源/Node。workspace-settings与schema共用最小ConfigRoleRef规则（`weftext-config://WorkspaceId/workspace-settings`与`.../schema`），配置版本及module read按各自权限冻结，不增style registry。Environment冻结module/settings/entry profile与实际World依赖；style change不发note RevisionId，却失效实际preview/editmap/semantic Query，未消费样式的静态field不失效。各 consumer 引用它，不逐槽复制 carrier。Environment明确实际target/bridge；每份CompileSnapshot只关联该次target的官方语义/origin/World与输出，不把HTML/Paged不同编译混拼为同一次结果。flow具体路线由EDITOR实验资格确定，Core不硬编码Paged唯一后端。Core 提供获权 snapshot World，不把宿主目录挂给 compiler；include/import/read/数据/图片/字体/plugin 路径统一受控，正/负实际读取完整记录。环境、字体fallback、资源不存在或授权变化失效真实 consumer。

`CompileSnapshot` 保存 exact SourceSnapshot、Environment、完整实际 World dependencies、最终官方 semantic projection/origin map、输出或诊断。合法源/body 编译或投影失败仍保原源、Draft 和可用静态 fields；旧预览不能冒当前。worker 无 ambient 文件/网络/凭据/写权，有可终止的时间/内存/输出预算；无完整追踪不给强资格。编译不会创建身份、保存或隐式调用业务动作。

产品错误和计算出的状态说明使用 canonical `UiMessage`（语义 key 与有类型的命名参数），不按机器 code、英文或中文运行时文本推导翻译。`ErrorDescription` 的 `explanation`、`diagnostics`、`causes` 均必填；原始诊断保留原文，并明确标注 `os/json/typst/network/externalTool/dependency` 来源。作者正文、第三方诊断与真实历史原文不进入产品模板。Core 错误另保机器 code、获权 `currentSource` 和类型化 details；普通状态投影不重新公开受保护源码。真正因果链最多 32 项，超出时完整原结构进入既有类型化问题集合，不截断或 panic。并列校验／编译问题不折成因果链或拼接字符串；其预算由原输入及生产边界控制。聚合 details 区分 Data 的真实分类、产品问题与未满足的准确语义证据；解码证据描述不授予写入资格。

JSON 正文路由共用一个请求解析边界：授权 middleware 先于正文，先核 JSON 内容类型，再按原路由字节上限读取原请求。框架字段／语法错误保原 HTTP 状态及原诊断，并使用同一类型化错误 envelope。处理器收到请求前，唯一 Data 严格解析器检查原字节中的全部对象键，包含泛型 Value 和嵌套 JSON；不从已丢失重复键的对象声称验证成功。响应 JSON 不改作请求解析器。原提交回执与后续交付独立，交付错误携带完整获权类型化事实，不覆盖已知提交状态。

托管导入／复制审核在原 ActorOperation 中冻结闭合的安全损失语义。该授权投影排除私有路径、作者值和来源诊断，不持久化翻译后的文案。公开损失只含固定安全 code、canonical 类型化说明、原诊断与因果数组，以及必填 `productDiagnostics` 数组。切换显示语言不重新生成已审核的损失事实。旧字符串 message 损失记录明确拒绝，原字节不重写。

编译／批注 worker 的唯一内部错误协议为 **version 2**。`display_tree_budget` 保持独立机器类别，并与 `product_error` 一样必带完整 `productDiagnostics` 数组；产品失败另有类型化 description。预算与真实 World 问题可同时保留。旧协议、未知／重复／额外成员和不合形状的对象均拒绝，不从错误文案恢复类别。新操作回执只保存操作、计划、提交状态、路径与成员事实，不再生成冗余说明字符串；既有 journal 和真实历史说明保留原字节。界面语言只控制显示。确需写入 DOCX／PDF 或新批注正文的产品标签，使用单独明确的输出／内容语言意图并冻结在相应请求与原计划中；切换界面语言不重写原 Plan、已存批注、作者值或 CEL 结果。

中文默认依 [EDITOR](EDITOR.zh-CN.md) §8.6 落实。字体锁固定可用字体集合及其准确文件版本/许可与回退环境，不能只记最终已用字体而忽略集合新增改变回退选择。Desktop、Server/CLI 和 WebUI 消费同一已准入集合；宿主系统字体只有显式准入后才能参加该环境，worker 不扫描 ambient 字体。浏览器获权同源交付遵守资源授权/撤权，不增加外网权限。字体集合、库语言/样式或实际依赖变化失效受影响 compile/preview/Job 准备；静态字段和 note 原字节不因字体变化而改动。已确认下载仍交原 staged bytes，缺当前交付资格则停止而不重编译替换。

### 9.1 共同LogicalCut及current发布


#### 9.1.1 唯一producer与打开算法

Core commit marker原子更新一个journal generation及该代所有已接纳 source/config/lifecycle/placement facts；多对象事务只有一个generation。构成cut的per-entity版本视图来自这些真实committed facts/保留blobs，不是连续若干`read current`得到的集合。

`openLogicalCut(scope,purpose,budget)`：

1. 验证caller当前潜在scope与最低披露；取得短commit/read barrier，等待该scope正在installing的Core事务变成committed或明确未可用，不读取半安装state。
2. 在同一journal读事务取已决generation `g`、完整已接纳causal heads、per-entity exact RevisionId/blob/lifecycle、目录结构历史视图、schema/rules/settings版本，以及静态声明读取的完整 Tags 版本及原登记资格（FEATURES §5.6）。scope membership/负范围只能来自该`g`可证明的完整已接纳库存；building/未知raw边界不变empty。目录/config视图来自原操作事实，source版本同时包含SourceSchemaBasis，相关原配置必须被retention保护。
3. 同事务注册cutHandle的audience、授权世代、依赖/coverage、有限TTL/预算及准确blob/config/history retention roots，再释放barrier。租约注册与读取之间不能被GC删除。pending manifest发布的版本从原committed journal得到，不等待网络。
4. `readEntity(ref,at=cutHandle)`、`readScope(at=cutHandle)`、schema/World reads都只选cut固定版本。缺blob/配置返回该cut不可用/partial，不补latest。cut handle是snapshot read权上下文，不是当前write basis；Action需fresh prepare。

cut目录/配置视图是一次历史快照，对应实际已接纳树/操作，不是另一份可独立修改的current parent表。现在源/结构仍以实际文件树为current authority；未接纳raw改变使相应`currentness=external_pending/unknown`，不能用cut旧blob当当前文件已一致的证明。

#### 9.1.2 必须防的反例与允许的外部观察

Core事务从 `(A.amount=10,B.amount=0)` 一次改成 `(A.amount=0,B.amount=10)`：读者无论在两次entity读取间是否有commit，cut固定两者版本，sum只能为10；不得A旧/B新算20。相同原则覆盖schema/定义与字段语义、membership与值、lifecycle以及资源World依赖。

ordinary已接纳业务Query默认LogicalCut，可用且可复现；检测到新external稿的对象明确pending/stale，已接纳历史结果不能装当前新稿。用户可explicit打开external observed浏览/Source，它记录每对象观察与coverage，不宣称整个raw库一个瞬时cut；不授全集空/精确aggregate/强Action。如果需要fixed选中对象的当前raw bytes一致性，可以在有限对象集合取得全部file share0与祖先guards、重读后形成该集合snapshot；范围外新增仍不完整。无需全库常驻锁或等待full OCR。

#### 9.1.3 result/pagination生命周期

FEATURES求值只读取上述cut。发布run时把定义/参数/语义语言/schema/cut、完整或partial覆盖的实际rows/cells、terminal schema和排序一次冻结到受保护spool。partial是已求得subset的冻结结果，不是后续任意插入行的活列表。预算/取消造成后续新结果时开新run，不偷改旧cursor内容。

排序是完整显式Query keys + 确定tiebreaker：普通entity行用完整typed Ref；unnest追加固定input顺序及当前array ordinal；保bag重复的生成/aggregate行用同run确定producer sequence。无order时也采用固定input顺序，不靠分页时数据库/遍历当前顺序。spool顺序与schema/row-count/coverage摘要在发布时固定。

cursor引用run/cut/audience/排序版本与exact ordinal（或受保护continuation record），不只传offset到latest Query。每页当前auth/fence/TTL/retention重验；撤权整run/cursor失效并清旧缓存，不通过过滤隐藏行后继续旧ordinal泄漏数量。其它数据变化可把该run标stale但仍获权只读固定数据；需要当前结果则重跑新cut/run。complete与partial都可分页浏览，只有complete/current/获权完整scope才支持相应全集强语义。


冻结结果计算后，Core取得短publication barrier，在同journal事务重验实际consumed依赖、scope正负范围、schema/settings、当前auth和observer连续性，并登记run/retention及从cut接续的连续change订阅。只相关依赖要求不变，不以无关全库generation增长拒绝。授权未变化、仅相关数据变动时可发布sealed stale只读handle或有限重算新cut/newrun；auth变化不交付。observer gap/external unknown不能current。验证与登记同事务，不能validate→publish留窗口。发布后相关变化标stale，auth变化使整个run/cursor失效。`current_at(publicationCut)`是本模式可证明的已接纳事实，不称任意raw全库即时原子。

### 9.2 Query语义、上下文与schema绑定

Type/Value、Query/CEL/参数、View/Action和导出共用SOURCE语义，原spelling单独保存。NAV/搜索/过滤同FEATURES Query，partial/current/stale/error与complete empty明确。evaluateQuery的显式闭合context含now（冻结Instant或null）/timeZone（选定zone与规则版本或null），无ambient host clock/zone。选择“当前时间”才由Core/Server clock provider一次产生now；选择库默认时读取cut settings并freeze实际zone/rules，子调用不重取。caller提供的now只作Query数据，不代替授权期限/审计时钟。

普通保存Query引用current并冻结真实定义；显式pinned不换latest，历史结果不复活写资格。定义native binding恰(schema:SchemaRef, revision:committed ConfigRevisionId, language:"weftext.query.cel.v1")，集中唯一SchemaRef是ConfigRoleRef(role=schema)，native为`weftext-config://<WorkspaceId>/schema`（内部role ref，不另抽UUID）；workspace-settings用同一ref规则的另一个闭合role，配置role不是Node-owned Resource。初始化commit建立role。Core按当前授权读原config revision的最小定义投影；short names只在原baseline解到稳定IDs，current兼容关系明确，不猜同名。sourceRevision也各带§3的SourceSchemaBasis；没有basis不从latest重建旧typed语义。

GC保当前已接纳revision/schema facts、仍可达source/Query/trash/history binding、active cut/run、未决plan/conflict/Job责任的准确roots。cut lease登记与journal snapshot读取同事务，不能先GC再注册；TTL结束明确expired，缺旧blob/config不取latest。current文件唯一authority保持，保留cut/history blobs只服务历史/冻结读与恢复，不作为raw current fallback。当前source/Query schema baseline在仍可达期间保其原配置版本，不永久保无消费者的所有版本。

私有读取 cut 的显式释放统一回收其全部 basis，包括未绑定结构读集的可选 Tags 捕获；只回收本次读取依据，不改作者文件、接纳事实、原 Plan 或独立 witness。纯验证、临时成员比较、构建失败及已返回的未发布 Query 错误在可靠收尾处释放。强写 Plan 已拥有准确文件／目录／缺席及成员集后，可结束其临时读取；成功 Query 的观察版行打开和原 strong cut 仍有实际消费者，不能因为输出缓存只存 JSON 就提前释放。此边界不引入全局 GC 或后台回收服务。


Job 固定输入版本/输出/loss/确认/预算/actor，保存步骤、attempt/effect key、批准消费、费用结算与 stop。建议应用把目标源+annotation accepted 状态同 Plan 提交；转换输出 staged 不代表文档已保存或下载交付。下载固定确认的 exact output，续传不换内容，逐交付授权。外部请求 unknown 保存原 attempt，先查询 provider；只有被证明同 key 幂等/条件写才可安全重试，不声称跨网络 exactly-once。stop 阻止之后获权步骤，不撤回已发生效果。

保留策略保护未决操作、冲突 heads/Base、精确批注历史、固定版本引用、正在消费的结果/导出与 external unknown 的必要 blobs；其它 cache 可清理。GC 不删除幂等最小事实、tombstone 或费用责任来重新执行，也不让所有投影无限保全。新已发布合同今后升级须有版本识别、保原件、显式 staged 转换、失败恢复与真实客户端兼容表；未部署旧候选不装 runtime decoder。

2026-10-19 本地第一阶段 Query 插入：`/api/query/definition/draft` 只保留原 Core definition-reservation 的 key 预约，返回准确源码提案，不写作者文件。`/api/query/definition/pending` 依据准确当前源中的未接纳 key、实际已纳管所有者和私有原预约找回原保存编号；只恢复可证明的唯一原预约，不猜同名。没有原预约、查询无法解析或含多个待接纳定义时，不额外阻断原生源码的普通保存，也不因此接纳任何未知 key；界面说明查询仍不可用。普通 Save 消费唯一原编号后才产生该定义接纳。取消／Undo 不提交作者内容；源中不含新 key 的其它保存不消耗该预约。没有新持久协议、额外可编辑 Query 数据库或跨主体授权通道。

2026-10-19 第二阶段：`LibraryCreateOptions.initialRoot` 为可选普通根笔记选择；未选时不编码此成员，原创建指纹不变。选查询时原冻结 create-tree Plan 同时安装准确 schema revision、出生普通 Node 并沿原 birth 接纳生成的 Query keys；没有额外预约或新持久协议。同 operationId 返回原结果，不在用户清空后重建。根笔记与库 envelope 同目录时，sync 资源所有权排除三个根控制文件；内容 apply／compound 依据接收端原纳管 Node 及未变 portable placement 保留本机根位置，不从可移植字段恢复本机路径，不建立特殊根类型。相邻 Query edit 的 action 与 plan／replacement 互斥，仍检查当前 Draft 及独立权限；最小 hosted formSchema 只披露已获字段读取权的闭包。 原生渲染依赖只由该次 World 的准确派生入口 FileId 标为 derived-source／derived-file；引用／标签消费者据此排除编译器入口，形似入口名的作者文件仍按原快照验证，不以路径前缀放行。

### 9.3 识别作业与原子导入

v0.6.2 识别执行资料只保存在执行端独立 private state；它不是作者树或并行提交账本。原 jobId 固定输入摘要、目的、页／区域、语言和规则、实际目标、授权 epoch、实现及模型摘要。重试只读原结果或中断事实，不因重启、过期、取消或丢响应自动重跑 OCR。预览与识别共用执行预算；缺模型不阻断保原件，读取状态、原页、结果和下载分别复核权限。模型／规则改变使未确认计划失效，不修改已保存笔记。

`ImportRecognized` 复用原 `CreateTree`，在一个原 Core Plan 内创建原生源、登记原件／证据资源、目录／节点出生、schema 基准、历史和同步发布。不存在先写正文后补原件／字段的第二次提交。取消未安装提案沿原操作结束；提交未知后只检查原 operationId，不另分编号重放。安装中断沿原 Plan 的准确物理对象及 before／after 证据恢复；未知对象保留并拒绝覆盖。

托管入口使用既有 `contentImportSlots` 的独立 `recognition {run,read,import,download}` 权限，默认均拒绝；这些权限不隐含未来正文、字段、历史、搜索、资源或输出权。映射字段还须目标成员的准确 FieldId 读写权，schema 读取另外获权。生成成员仍由原 slot producer 的出生事实证明，新增目录资格和每个后续消费者沿既有 Core 规则；同步保原件、来源、字段及真实身份，不传执行资格或私人作业目录。

### 9.4 本设备任务提醒决定

`tasks/reminder` 是唯一作者意图。Core 从准入且当前可读的已保存 Task 重建候选，不从 Draft、通知参数或第二任务缓存取事实。共同 `device-state/reminders.json` 仅保存库选择、逐提醒 opt-in、个人处理和投递责任；复用 device-state 身份，不用 sync replica_id。复制任务不复制启用资格。固定叶 typed 完整格式拒未知／重复成员，限 16 MiB、128 库及 32768 记录，经原字节摘要和共同 device guard/CAS/flush/回读持久。库固定 WorkspaceId 与物理根身份，路径仅定位；外国设备／profile 不自动采纳。scope-first backup 仅在完整验证已知叶后排除该设备全局数据，未知文件仍拒绝。

设备记录使用 schema version 2。`failedBeforeCall` 与 `unknown` 保存类型化 `CoreDescription`，在原记录预算内保留实际因果链、原诊断来源及完整并列事实。单条投递说明另有 16 KiB 序列化预算，超过时明确拒绝，不截断。调度问题连同 owner 与 reminder 身份保留完整原始类型化 Core 错误。状态／版本／重复校验使用显式事实，不解析诊断文案。v1 与旧字符串诊断记录明确拒绝，原始字节保留。原 OS 请求中的标题／正文、执行键域及系统通知 group 属于独立外部事实，保持原值。

一次多成员捕获按库收集，仅在本次调用内复用该库的完整库存定位观察，切换库即释放；准备、原批次重试和原通知重新核权共用完整捕获路径。逐源的原物理准入、当前位置、字节、Schema 与控制见证不省略；发生决定前，在原输入 guards 持有期间按库重新完整扫描，核对固定 NodeRef 到原 source path 的唯一映射。单条订阅与有界内部批次共用一条设置写入路径：原 mutation 重取先于重新捕获，重复成员拒绝，完整资格后一次 CAS，失败不写前缀。对外仍为原单条命令，原单条 fingerprint 和禁用语义保持。

不可克隆、不可由 wire 构造的交付许可只携带原 members（至多64项），源码／控制见证先按准确路径去重再克隆。私有 journal cut 持有原 commit、Birth、observation 记录的受保护读取句柄，只纳入初始已存在且经原解码器确认的 commit。签发与调用前接纳均用一次批量检查复用普通 Core reader 的同一物理准入、placement 重放和已接纳 revision 算法，读取固定 cut 的原记录，不接纳临时新增 commit／observation。完整已知 operation、相关名称、parent／FI、stamp 与 hash 围栏仍须一致。journal 枚举沿用5000成员上限并守64MiB实读预算；原作者输入1024路径／64MiB预算保持。Debug 只报计数，不输出捕获正文。

调用前在原作者 guards 内复核当前设备／账本、原成员、选库／订阅同意及 effect key，准确原正／负作者见证、fresh 完整库存映射与该固定 journal 证明。仅正文变化、Schema／控制或 journal 证据变化也拒绝旧许可，不静默重新捕获。确认 failed-before-call 后，才按原效果号通过完整 fresh 捕获重试。完整输入资格先于新 attempt 时钟及原五秒墙钟／单调时限；最终 postcheck、目标分钟检查与一次性消费仍先于外部调用。

执行键为 `weftext.reminder-effect.v1` 与 device、NodeRef、reminderId、实际语义摘要和精确 trigger 的 canonical SHA256。先持久原键的 submitting、完整 OS request 和 attempt，才返回不可反序列化／克隆的 permit。每次系统调用前必须 `check_reminder_permit`：重验当前源、库、设备、任务和授权，复核共同源／配置正负见证闭包及全部后置检查，并最终核对目标分钟、单调 5 秒时限和墙钟未回拨；全部通过后才准入一次并消费许可。普通投递不能因资格核查耗时越过目标分钟。Native 在 Core 锁外调用 OS，再以原 permit 写回结果；最后检查不声称与之后的作者编辑或 OS 调用全局原子。Server 没有 toast producer 或投递 HTTP 接口。

submittedToSystem、可证明未调用的 failedBeforeCall 和 unknown 分开；重启后的 submitting 对外也是 unknown，成功不是已读。只有完整原 app/tag/group/activation/request 在系统历史出现才能确认 unknown；缺席、用户清除或系统限制不解锁重试。明确取消原因（库／提醒关闭、Task 关闭、定义变更）与个人 handled、外部结果正交；处理不写 Task.status。撤回使用原地址，opaque activation key 回跳逐成员重新授权，不能恢复旧读取权。

宿主可按真实单条／汇总投递键有界读取完整原通知，核对激活目录保存的 opaque key。此读取仅验证当前设备账本，不要求作者库在线，也不改投递结果；汇总内的任务成员键不是独立 OS 通知地址。后续打开仍通过实际 Core 回跳重新授权，原记录存在不授予当前作者读取权。

恢复汇总由 Core 可信扫描当前全部已启用且同意投递的订阅，调用方不能挑选前 64 项。受限或未知 scope 仅产生保留完整原因的类型化问题，不消费该范围。问题样本最多 32 项，`issue_count` 保留总数、`issues_complete` 明示样本是否完整；任何问题都不能被解释为完整可资格集合。完成、失效和已处理项排除。

全部新增错过项必须同时符合 1,024 输入、64 MiB 扫描及 16 MiB／32,768 记录的账本预算；超预算整批不发 permit、不写水位，并明确报告预算／部分范围。在预算内按 execution key 排序，一次 CAS 固定不超过 64 个实际 OS 成员和其余项的 `SummaryOverflow`。后者仅投影 `inAppOnly`，没有 OS Attempt；原通知读取、历史确认、完成和激活入口均不把它当作系统通知。汇总在扫描后的新可信准入时间固定 cutoff，`nextAllowedAt = cutoff + 900 秒`，重启保留冷却时间，墙钟回拨时拒绝新批。

仅原 Individual 的确定 `failedBeforeCall` 可以首次进入该错过阶段，并保留 prior Attempt。既有 SummaryMember／Overflow 或任何可能已有 OS 效果者不得进入新批。原汇总的确定调用前失败仅能按原 key、原 request 和原成员显式重试，cutoff、overflow 和 cooldown 均不变；不重组为另一个汇总。

最终决定前后保持正向输入共享只读 guard、核对原 parent／准确字节／身份／Pending；缺失配置保原见证并前后复验，使用唯一 bundled schema，无用户安装前置。缺失槽是观察而非未来创建锁；后验变化不给 permit，已持久 submitting／unknown 不抹去。最后检查与 OS 效果不声称对之后任意作者编辑全局原子。已有无效配置不回退。单独数据／账本测试不等于真实桌面 DSK03–08 验收。

## 10. 最小消费接口

本机设备身份、三项桌面设置及其私有记录/CAS/库备份边界见[本机设备状态](DEVICE-STATE.zh-CN.md)。这层基础不代表原生 UI、提醒或安装已经通过验收。

| 接口 / 对象 | 各 owner 的责任 |
| --- | --- |
| `openLogicalCut` / `readEntity` / `readScope` / `validate` → `SourceSnapshot, ReadBasis, DependencySet, LogicalCut` | Core 负责获权字节、当前对象和完整范围；SOURCE解释官方源/值，FEATURES消费同查询范围，EDITOR不从文件再扫描。 |
| `prepare(Intent, bases)` / `commit(operationId, planHandle)` / `operationStatus` | 所有作者动作共用计划/日志/结果；同ID精确重试。SOURCE局部patch、FEATURES业务条件、EDITOR Draft不另开提交入口。 |
| `observeExternal` / `admitExternal` / `merge` | 捕获原 bytes、原 ID 冲突和不完整状态，明确接纳/合并，重新资格。 |
| `compile(SourceSnapshot, Environment)` → `CompileSnapshot` | Core World/权限/worker；SOURCE与EDITOR消费同次最终语义和origin，不造第二解释器。 |
| `evaluateQuery(definitionSelection, parameters, scope, budget, context)` / `publishResultSnapshot` / `readResultPage` → `QueryResult` | FEATURES定义语义，Core绑定主体/版本/依赖/完整性；显示、导航和导出消费一份结果合同。 |
| `readConfigProjection(ConfigRoleRef, revision, selected IDs, purpose)` | Core按当前授权提供原schema/config最小投影；不交hidden全schema，不补latest，不产生当前writer。 |
| `createJob` / `jobStatus` / `stopJob` / protected download | 固定输入/责任与交付，作者改变回同Plan，效果unknown不盲重发。 |

这些是最小语义协议，不声称冻结每个 wire 字段。实现前以一代 strict schema 冻结 kind/version、必需/null/unknown/duplicate、canonical fingerprint、typed refs 与预算。不能以开放 JSON callback 绕过 typed Intent，也不继承十九 role adapter 与多代 decoder 祖先。

## 11. 需求、原伤害场景与验证门

RQ01–10 的源、类型、标题/混排/深标题与 Word 由 SOURCE/EDITOR/FEATURES拥有；本文提供一致 bytes/版本/授权。RQ11–20 的创建、链接、Query、任务、批注、模板与 Office 由对应 owner 消费同 Plan/结果；RQ21–24 直接落在本文三种模式与提交/同步/权限；RQ25 与 NAV01–05 的布局、摘要、快捷和列表呈现仅为个人本地偏好，恢复必须新读，不复制作者事实或旧结果。

| 原伤害 ID | 新方案位置与必须实测的失败 oracle |
| --- | --- |
| P1-01 / P2-01 / P2-14 | §4同map最终源/refs/版本/回执；复制互引、重复值、Ref-free和含资源批注预览与真正效果一致。 |
| P1-02 / P2-15 | §§2/4/6作者样式同源、完整change set同事务；保存/恢复/同步不丢混排，多效果不只完成一半。 |
| P1-03 / P1-04 | §9同次官方最终语义；深标题/生成标题可读可查，不能伪造写地址。 |
| P1-05 / P1-06 | §§8–9Trash只读、current/pinned明确、Actions重验；不能借历史结果写当前。 |
| P2-02 / P2-07 | §§5–6每步crash/磁盘满/响应丢失/第三态；原ID恢复，无伪saved或丢真实历史。 |
| P2-03 / P2-12 | §§3/7/8完整Workspace/type/ref/dependencies核对；相似bytes或错库commit不通过。 |
| P2-04 / P2-10 | 当前需求完整覆盖，原件与未知数据保全；机器计数、少字段seed或旧评审不代替需求。 |
| P2-05 / P2-06 | SOURCE统一精确值和完整合法语法；电话、decimal、日期、重复项和复杂表格原源不被损坏。 |
| P2-08 / P2-09 | §§3/7/8初始化一次发布、身份非权限；如部署签名则完整manifest/域/撤销验证，不能拼branch授权。 |
| P2-11 / P2-13 | 独立批注精确媒体范围、目标/摘要分别披露；目标失效保讨论，拒绝建议不偷读隐藏源。 |

全部场景在新方案上重新验证，未沿用旧 CLOSED。P0 必须实际完成 Windows 同句柄写/已有 mapping/filter/父rename、同目录对象move/Node两步rename/copy发布/no-replace/flush/crash恢复和第三态保全；Server 双实例旧holder fence、撤权与读/chunk/cache/commit竞争；官方 post-show/收敛语义和origin桥对照。P1 必须完成多副本乱序/partial/并发与同ID重试、预算/大库活动文档不等全scan、类型端到端/真实Office可编辑、Job unknown/费用/stop。文档自洽或短时句柄 probe 不能代替断电、文件系统、产品和性能证据。

本设计确有不可删除的复杂性：文件安装/恢复、当前资格与历史版本分开、完整负依赖、披露边界、因果冲突和外部责任。被删除的是无实际部署依据的旧协议族、重复模型与第二权威；真正旧数据若出现，先保原件，再以显式 import/recovery 工具处理具体格式。


### 本地同步消费者与原提案取消

本地 sync API 分别提供配对、启用、状态、发布、目录接收/导出和准确版本文件读取，复用原 OperationRequest / Preview / Receipt 的 clone/apply/merge 入口。状态读取不发布、不安装；已安装 portable revision 与接收图分别返回，依赖诊断比较历史依赖和当前图，不以旧提交诊断阻塞无关对象。版本文件读取必须匹配指定对象、已接纳的准确对象修订及其中的准确文件，包含不再处于当前分支或共同基础的保留祖先；返回独立的原 fileRevision，不以新版本兜底、猜测文件路径或执行文本。全部入口只允许本地所有者；hosted actor 不获得此能力。

`sync/history` 按明确对象分页返回已收到的原 commit、parents、before、revision、state 和实际 admission。每页默认 25、最多 100 项，并受序列化预算限制；提交摘要仅是分页游标，不代表时间或因果顺序。coverage 表示当前收到的对象记录是否都已接纳，不表示远端已收齐，也不表示本机已安装；尚有下一页由 nextCommit 单独表达。分页期间又收到记录时，应重新读取首页，不把跨页拼接当持久快照。缺件和拒绝记录保留诊断，不允许读其正文。浏览器历史入口只读，不能把祖先记录当作当前冲突分支直接提交。该接口的产品验收范围沿用候选记录，不能以接通代码代替实际验收。

`AdoptSyncExternal` 明确区分因果前置与当前实物：`expectedInstalled` 固定每个所选对象的准确已安装修订，`changes` 则携带原观察 snapshot base 及相同的 before/after 字节，只接纳已经存在的当前外部修改。`sync/adoption-cut` 从原本机 locators 读取明确选择，不根据收到版本的路径猜本机对象；本地 HTTP 返回的完整 JSON 最多 16 MiB，浏览器存储可能更小，持久保存失败必须发生在首次 prepare 之前。普通 Save/Apply 不因此绕过外部修改冲突。未选对象成员、当前 Schema/规则与所需类型引用依赖仍须按原范围核对；不支持的结构或语义决定保原件并明确拒绝，不用该动作顺带任意改稿。

原 cut 与 intent 可选携带 `removedMembers: [{path, baseId}]`，缺省为空，明确采纳本机已经发生的资源移除。每项必须是原 `expectedInstalled` cut 中已登记、已证明所属 Node 和物理身份的 Resource 叶文件；其原负 snapshot 须证明 `captured=None`、准确路径及同一父目录身份。仍存在的空文件不等于缺失，不能用空 `after` 表示删除；重复、路径别名、与 `changes` 竞争或 cut 外成员均拒绝。原 canonical owner 与资源 registry 仍须是原文件对象，准确观察到的 registry 必须同时省略原 ResourceId 和路径；不能将 canonical source、批注或控制文件的缺失冒充资源移除。

采纳仍为原无本地内容写入的 Update Plan：保留资源缺席／父目录负依赖，其中移除项只在 after 与原 publication 登记准确资源成员及其文件修订条目的退出，原历史修订和 blob 保留。原生验证读取真实 after World，不能为已移除成员补回旧 before 字节；重新出现的文件、父目录替换或 Pending 不能沿旧缺席资格提交或恢复。浏览器预览列出已在外部移除的资源，持久保留原列表与 bases 后按原请求／编号重试，不执行新的本地删除动作。接收端仍须在自己的原 Compound 中证明并保全旧资源实物，不能从发送端缺席获得任意删除权。该保全路径的 Core 与本轮同一最终优化程序 HTTP／浏览器组合已通过验收。

普通 Save 和事务发布同时核对原已安装文件的物理观察；重新打开得到的新 Base 不能把同字节的外部改写变成已安装事实。原 Core 提交产生的同字节写入仍沿其准确观察连续有效；缺失的旧证据不可用当前观察补齐。

外部采纳现沿原无写入 Update Plan 核对文献、保存定义及实际 Query／Provider 语义，需要原生产品时进入下述同一编译证据路径。其 current 查询固定为原 Plan 的已安装 before cut 与明确观察到的 after，不跟随较新的 received heads；真实接收图只提供准确历史／pinned 查找，未安装远端分支仍保留。对仍存在的内容，观察适配仅认可同一原文件身份、原 snapshot、before＝after 字节及原预览分配的新文件修订，不授予新的 Node／资源／定义键身份。缺依赖、外国绑定、未知定义资格、过期观察或 Pending 仍拒绝。普通未解析链接和未知作者内容可保留；不需要编译产品的普通源不强制整篇编译。下述 RevalidateSync 仍是对明确收到版本的独立语义决定，不能跳过原外部修改保护。

新 enable 的原 Plan 保留初始叶文件的物理绑定，clone/apply 沿原观察事实建立本机证据；旧计划缺少物理证据时报告 unavailable，不用当前同路径对象回填旧资格。准备后文件、父目录或依赖变化使提交失败，原请求及草稿仍保留。采纳复用原持久 Plan、提交决定、outbox 和恢复；没有第二份当前状态或新执行日志。缺省 Schema/规则必须是真实缺失且保留原负依赖；后来出现的文件不能借 bundled 默认绕过核对。成功登记的是准确当前字节与明确资源成员状态的新修订，历史分支和未安装版本不会被静默选择。

浏览器先持久保留准确请求，再发准备或确认；查询不执行，准备丢响应只重试原请求。按原编号找回入口返回 Core 保留的准确请求和当前预览，支持浏览器地址变化。新增 cancelPrepared 只适用于准确 planId 的原同步事务：原提交/已结束结果保持幂等；安装标记存在则拒绝并要求原恢复；尚未进入安装时，仅持久记录取消，不检查或写入当前文件。这样允许放弃被外部修改使之过期的提案，同时保证迟到原 execute 不能生效。它与 keepCurrent 对原参与对象的完整恢复证明不同，不能用于跳过已发生安装的恢复。


### RevalidateSync：显式语义决定与原事务证据

0.6.0 Windows／NTFS 既定 RevalidateSync 范围已接通 Core／Data、App 与浏览器，并通过同一最终优化程序的语义 HTTP 和受影响浏览器验收。实际原生产品、Native Adopt、移动 include、Style、live／Trash／混合生命周期及原号恢复沿下述同一证据链处理。准确最终检查范围见 [MVP](../MVP.zh-CN.md)；尚未正式发布，其他平台及实际断电不在本轮通过范围。

RevalidateSync 是用户可主动选择的语义决定，不要求先制造普通 ApplySync 的拒绝。普通 Save 与保留固定 Schema 绑定的保存定义不因出现 Query 语法就必然触发 `409`；是否需要特殊复验由准确当前依赖和原操作合同决定，不能把该拒绝作为每个 HTTP 场景的前提。

`sync/revalidation-cut` 只读已接纳收到图的有界对象、原依赖及准确文件描述；对象修订与 `fileRevision` 分别返回。候选清单不授予读取本机任意路径的权力，也不把尚未选择的候选当作实际语义依赖。`sync/revalidation-review` 对明确选择及所供修复字节推导实际语义读集，返回 `expectedInstalled`、`reviewedVersions`、每项 `reviewedDependencies` 和批注目标状态。请求可选带入同一 UI 动作预先保留的 `operationId`，响应原样绑定；新回收原生源需要该编号。绑定原编号的多位置／回收组合，其只读位置投影与原 writer 共用 `compound_locations`，沿准确所选 parent 图及原已安装位置推导各根和继承子树目标，不分配新身份或预留路径；需要原编号的新回收布局若缺号，或布局缺 parent 决定／无法闭合，均不能猜测。该编号不是资格：cut／review 不改作者文件、不 stage、不发布新分支，Core 不在此分配操作、Plan 或文件修订；临时快照读依据用后释放。返回的 DTO 与客户布尔确认都不是 writer 证据。

每个决定固定原对象、全部当前收到分支及准确所选基准。`preserve(revision)` 保留该版本的完整 state、原生源字节、文件修订和原绑定，不自动改键、重写 metadata 或绑定 current/latest。`repair(baseRevision)` 明确提交用户选择的完整 after state 与准确新字节；改变的文件必须取得与其所有者／摘要一致的新 `fileRevision`，不能复用原修订改字节。未改文件保原修订；改回准确原字节可保原文件修订。保留决定也可形成有新依赖及完整因果 parents 的新对象修订，不冒充正文已经重写；未知作者内容、批注和未选择文件不能被隐式清理。

正式 `RevalidateSync` 复用原 OperationRequest／Plan，提交 review 固定的准确已安装 cut、受影响事务／父级／语义读闭包中每个对象的全部当前收到分支，以及逐决定的实际依赖。Schema／Rules、typed 引用、Work 出版数据、资源、current／pinned 定义及其原 Schema 基线均按真实读取加入闭包；不能只沿旧 manifest 的依赖列表，不能给并发依赖自动选胜者。缺少准确历史源码或基线不回退 latest。新收到分支、安装集变化、依赖改变或同编号换意图必须拒绝原提案，不能在 prepare 静默升级 review。

语义检查沿原 Data 类型检查、定义预检和执行器。独立闭合的参数表达式、callee／Provider 调用必须按其准确原基线执行预检，并在需要行数据时消费原 Core 已接纳 cut 及所选 after 的真实行、最终位置和成员关系；新建／恢复／退出 live 的成员必须由同一原计划证明，不能用空行或旧 before 行冒结果。必填参数未知时保留合法的参数化定义及明确 deferred 项，不造参数值；未知参数不能豁免同定义内可独立闭合表达式、兄弟调用或固定 Provider 选择。实际 Provider／策略选择和依赖保留准确版本，校验不全或预算不足不能冒完整零结果。

实际调用需要原生产品时，Core 不透明捕获固定准确源、文件修订、所选 after 位置及可读取的 World；App 在 Core 锁外通过原隔离 Engine worker 编译，所有闭包轮次共用有界预算。最终 title／body／headings 经 Data 的同一产品转换进入原执行器及真实 Provider，不能用静态字段、空行、旧 before 或伪造成功替代。真正消费的 unavailable／歧义产品明确拒绝；不因无关坏正文而要求所有静态动作编译整库。

只读 review 将 compiler 的实际 include／资源／配置读取纳入准确对象依赖，按原接收分支与成员集求有界固定点；缺件、未明确决定的并发依赖或预算不足明确拒绝，不自动绑定 latest。prepare 必须与已审阅的实际读集一致，不能在 finish 私下补选新对象。worker 返回时重新核对已安装／已收到 cut、live 与 Trash 各自原成员集、原文件身份、正／负读取和父目录见证；原 Plan 的 include 移动仍从原位置捕获身份，按准确 after 位置编译并在恢复中重验。已由原 Plan 证明退役的准确 Resource 成员从 after World 及其 current owner 映射排除，原文件的正依赖／FI guard 仍保留；不能读保全位置或旧 before 字节填补编译输入，真实消费缺失输入仍拒绝。当前默认库 Style 还须证明原 Settings 选择、同库唯一资源绑定、实际 Styles Node 与 registry、已接纳资源身份，或同一原 Plan 的 create-only 输出；名称或相同字节不能代替角色，Style／Settings 的迟到变化不能获得旧资格。

纯复验结果仍须由原 writer 补齐六类证据：原对象接纳与位置、准确所属资源、current／pinned 定义身份及历史源码、原配置缺席、批注目标事实、实际 Query／Provider 调用。原 Core 不透明资格与文件／父目录见证和同计划创建的原出生／输出共同证明 after；客户提供的路径、元数据或相同字节不能补造接纳。正／负读依赖、Query 原成员集、准确历史输入及已履行的语义义务冻结进同一 Plan／SyncApplication，受原事务预算约束，在 stage 前、首效前与原操作恢复中重验；commit marker、outbox、状态查询与恢复责任不另起一套 writer 或账本。

将额外语义读集并入原 Plan 时，必须重新核对全部相关既有文件、缺席槽位及父目录的 Pending，执行／恢复只可排除本操作自己的准确编号。磁盘暂时缺席、只读捕获成功或 bundled 默认不能绕过另一个未决安装；新到的配置记录也使原负依赖失效。Directory 与 RootDirectory 必须按准确收到对象角色、原标记、资格见证及实际位置匹配，不能仅按 ID 拼接、路径恰在根或旧依赖列表猜角色。Node 出生容器中的标记外观成员不能获得非 Node Directory 身份；原计划重启、已提交／已知 after 的接纳消费者也须核对同一准确角色证据。上述 Pending 与 Node 根标记的实际反例回归已通过，其它未运行组合仍不得记为通过。

批注原记录、目标描述、回复、状态与未知成员保留。目标不在获权 cut、准确历史范围／资源版本缺失、编译元素来源或可信媒体事实不足时，review／预览明确列 `unavailable` 和原因，不能读取隐藏目标、删除讨论、猜测重锚或扩成整篇成功。可证明的准确目标与不可定位记录分别处理，不因讨论获准保留而产生目标读取权。

live、Trash 及混合调用按原实际生命周期快照与所选 after 判定；子节点的 effective Trash 沿准确 parent 链继承，不把 portable 子节点的原 `retired=false` 改成独立退休，也不将其列为另一个回收根。多根回收、live 移动、新出生与资源成员退役可沿同一原 Compound 布局和资格接入实际原生验证，不能用旧 Trash 位置或假 live 行代替。恢复的成员比较仍受上述原 FI／父目录／phase 的 before 投影与同 Plan 出生证明约束，外来迟到成员继续拒绝。本轮既定组合的受影响 HTTP／浏览器已通过同一最终优化程序验收；证据不足或超预算明确拒绝，原生编译成功不等于结构、写资格或整项事务已完成。

浏览器保留原 Draft／Undo。同一明确输入首次 review 预留操作编号，重复读取或只读响应丢失沿用该号；分支、修复输入或资料库／接收 cut 改变时失效旧资格与编号，迟到审阅代次不得确认。响应必须绑定本次原号；正式 prepare 的外层复用该号，intent 不夹带 review 的编号或批注展示状态。准确请求成功持久交给原 SyncFlow 后才消费临时 review，之后 prepare／commit／unknown 查询与重试只沿原请求；持久化失败且未发 HTTP 时保留用户修复输入与选择。终止或放弃后的旧 review 不可再准备同号操作。

原保存查询定义的复验性能热点已修复；本轮最终优化程序的实际定义复验与运行观察见 MVP。这些并发验收观测不作为隔离 benchmark、普遍时延保证或其他平台性能证明。既定 Windows／NTFS v0.6 功能及消费者验收已完成，尚未正式发布。


### 同所有者托管传输消费者

Server 的独立 syncOwner capability 仅授权明确 syncIdentity 的完整不可变记录传输，不继承文档或字段权，不授予原始路径、private state 或作者树安装。持久策略绑定配对，actor/epoch/票据与每块交付/接收在同一服务锁下重验；撤权清缓存、重新授权不复活旧票据。服务端已有同一物理副本的用户保存沿用原 Core outbox，relay 不因接收 manifest 改 current。上传按完整记录摘要幂等；半条记录是有界可丢内存缓存，完整记录由原 Core immutable store 保留，不能把传输进度当作者 receipt。下载冻结记录字节，不在续传时取 latest。

本地网络客户端只在准确配对与有限记录目录下复制，网络等待不持有 Core writer 锁。每条完成记录再经 Core 校验，部分或重启传输保留已收完整记录，安装继续使用已有 Plan 资格与恢复机制。此通道信任同所有者设备/账户；TLS、权限和摘要分别处理传输机密、披露资格和完整性，不构成设备签名/恶意 provider 协议。


## 原提交事实与结果交付（当前开发合同）

创建、复制、模板和恢复的交付从原事务的 canonical birth／搬移物理身份出发，核原 workspace、birth 与 materialized 证据，再投影同一物理节点的唯一当前 placement。正常移动可交付原节点的当前位置；旧路径被另一已接纳节点复用、同 NodeID 的文件替身或证据不完整时，不按旧 path 或逻辑 ID 猜对象，只保原回执并报告交付不可用。库创建同样独立核原物理库根与可选根节点；根笔记随实际文档交付，不再次按 `rootNotePath` 打开。识别导入的本地打开沿原 actor/resource/library 操作记录核同一 birth 成员，托管入口仍独立授权，不借本地全局查询。

Query 定义重放先用与 `save` 生产共用的完整 SaveRequest 指纹核 session、base、原 Plan path 与保留的最终 source，再做可选 schema／结果 IO，按原 pinned schema 重建准确源码；只读原提交事实，不调用保存写入。旧 Core Session 和界面位置缓存丢失不妨碍核原请求，变输入仍报 operationConflict。普通文件保存与 `enableSync` 是当前正式的非 transaction Plan 家族；同步启用状态从原 SyncSetup 和严格原 commit 读取固定 identity／replicaId，不用当前 binding 冒充原结果。未知 kind、互相矛盾的参与者或损坏 commit 保留为错误，没有另一份回执账本。

另存为在写入前，沿原操作既有 materialized-object 证据记新文件的物理身份；首次交付和以后查询共用同一严格身份重开，复用路径上的相同字节不能证明仍是原文件。证据缺失或普通文件移出已知定位范围时，保原提交回执并报告交付不可用。当前缓冲区可明确只结束这次界面交付等待，继续原 Session、Y 和 Undo，并保原号用于只读查询；这个处置不取消作者效果、不宣称目标已交付，也不分配新号重投原另存为。

已有普通文件标签及普通启动文档的自动刷新，只向 `/api/open` 提交原 `sessionId`。Core 在重开前后核原 Session 所持文件的物理身份；缺 Session、原件移走或同字节替身均不能静默安装新 writer。原缓冲区、Session 和 Undo 保留，可明确另存当前稿。`{path,root?}` 是独立的新路径打开意图，混合两种请求会被拒绝。此设备本地 reader 不作为托管授权入口。

Save／另存为首次响应及原号状态共用同一文件保存交付信封。消费者核原号、实际 saved／committed 状态、非空原回执 base／path、冻结源／路径，以及完整实际 document 与一致 source-match 标记，或 typed documentError。通用 committed 信封不足以确认文件 writer；畸形回执保原请求与输入，只查原号恢复，不再以缺 document 的回退分支推进 Base 或释放编号。

新建普通笔记、快速记录与周期笔记共用现有 `NewNode`／`CreateTree`，没有另一条直接文件写入算法。创建接口返回原 `OperationReceipt`，可选 `document` 与 `documentError` 单独报告打开和会话登记；已有回执不会因之后打开、刷新或下载失败变成无效果。快速记录／周期请求先核原号与完整作者输入，已存在的计划不按当前 schema 重解释。

普通保存的原 Plan 仍只保存 `Receipt`。瞬时 `SaveOutcome` 的 `recoveryIssue` 报告提交后个人 Draft 更新失败，源码提交与会话的新依据仍成立。`/api/operations/status` 从原 Plan／commit 读取保存事实，不依赖旧会话；其可选新文档有自己的实际 base。客户端只在原编辑器会话、真实 owner 和已提交源码均匹配时接纳新会话，保留等待期间的新输入、选区与 Undo。当前磁盘已是另一版时保原回执与交付待定，不能把原回执 base 与另一版的会话混用。Query 定义和历史恢复的状态也先保提交事实，再独立报告结果读取失败。

普通错误码和单次 missing 不证明无效果。客户端冻结原请求；未知结果先查原号，再经 `/api/operations/settle` 在同一原 journal 写入持久终结后才可释放编号。新全局查询／终结限本地所有者；托管内部事实读取继续由原 actor、resource、library 校验约束。操作编号统一为小写带连字符 UUID，拒绝别名，不改写原件。resolution 使用有界、严格字段和原号／文件名校验；解析或读盘错误保留为错误，不能解释成未解决或已取消。已加载 Core 中确定已提交的原事实优先于附属交付元数据。

无 Plan 的决议只能是 `cancelPrepared`。裸终结标记属于设备本身，不进入某个库的作者或同步集合。已有非安装预约须带原号及原库物理身份，按原 typed 记录核对；与预约相连的取消及预约按实际归属共同备份，外库只读有界归属首部。新取消不再制造 `cancelLibraryCreation` Plan；保留的旧取消仅按严格无作者参与者的形状读取。任何未知 suffix、孤立 install／commit、损坏决议或无法证明归属的预约都保留并拒绝宣布无效果；备份不忽略这些证据。实现与定向回归不建立整版 0.7、多端、真实 IME 或断电验收。

终结请求仅含 `operationId`、`requestKind` 和严格 64 位十六进制 `wireDigest`。客户端在首次异步等待前同步固定原 `JSON.stringify` 字节，再对这些字节计算 SHA-256；终结不重采后续输入，也不要求服务端再次解析原作者 JSON，因此被拒绝的孤立 UTF-16 代理项也能安全结束原号。摘要是同一决议的请求关联元数据，不证明未提交；后端仍先核原 Plan／commit、命名空间证据与所有者。历史恢复沿同一冻结边界，后续 CaptureDraft 直接复用此合同。

本地历史恢复的 restore 与 status 接口统一返回原操作状态信封：`operationId/status/receipt` 加可选 `result/resultError`。完整结果直接可消费；提交已确定但准确结果暂不可读时，仍保原回执、编号和 committed 状态，不能退成未知或再发作者恢复。missing 查询与终结之间可能已有提交：通用 settle 的 flat 保存事实先保留，再从 history/status 取得历史结果，不能把 flat 回执当作历史结果。不同作者输入的同号请求仍返回 operationConflict。

备份对 Plan、带作用域的 Draft／actor 首部、CaptureDraft 及无 Plan 预约，均先按明确原 workspace 身份路由，再看定位路径。A 的整物理库根保留移走、旧路径独立准入新 B 后，A 的外库记录会被排除，不读其正文，也不阻挡 B 的合法备份。同 workspace 却物理根不同，或同一物理根被矛盾 workspace 声称，仍报 scopeChanged；未知／畸形首部、有界预算和选中记录的严格 reader 继续保守拒绝。

文件保存使用原 Plan 的必需 `fileSave {version:4,sessionId,baseId?,scope}` 参与者；普通 Save 保原 session/base，SaveAs 保原 session 并在 stage 前冻结目标归属。path 与 after 只存于原 Plan，完整参与者和 scope 纳入同一指纹、同一 commit。`QualifiedInputScope` 只有明确 `deviceUnassigned` 或 `library {binding:{root,rootIdentity,workspaceId}}`，库 WorkspaceId 非可选。Open 检查真实祖先的已准入库，即便普通嵌套文件默认 UI root 只是直接父目录，也不能误归为设备输入。缺控制证明、损坏或读取失败不降为设备输入。Save 承原 Session 的 scope，SaveAs 取目标 scope；原号重试／查询不根据当前路径重算归属。

Session、当前文件 Draft 共用同一 scope，Draft 必需原 FI，唯一主键为 `(FI,scope)`。旧 Session 的后续输入仍归原库，即使旧根路径已属于另一库；不得重新标记为当前库。已提交的同对象移动只在同 scope 下更新主记录 path，先归档准确旧字节；删除旧路径槽／空归属合并和 consumed 迁移协议。旧缺 scope 的 Draft 或缺必需参与者／版本的 file-save 原件保持，报告 unsupported，不自动补证、升级或提供正常写入／备份双轨。原历史 commit 不能因此被解释为无效果。

备份、历史／修订、同步安装戳和托管原操作读取共用原 Plan 资格。普通 Save、SaveAs、未准入普通文件及未提交私有提案都按原 scope 分类；不保留路径回退或临时 Birth 推导索引。actor 的 Server 配置 namespace 是独立权限域，可是普通目录或子目录，不能当文件归属；file-save actor 记录复用同 Plan 分类与原权限记录。Capture 保留设备／已绑定生命周期和原 volume-GUID 坐标首部；其 shape 校验不等于 file-save 原 scope 证明，不能把缺失 WorkspaceId 的权限目录当成已绑定库。若消费者需要转换，须经 Core 原准入事实取得稳定 locator，不直接复用坐标字符串。

原 ActorOperation 也以必需 closed `ActorInputScope` 首部保存输入归属：`qualified {scope}` 或没有作者输入的纯 fence `noInput`。Save／field／definition／history 从原 Session 冻结，feature／navigation／backup 从已核原输入 root 冻结，发生在持久提案写入之前。已有 Core file-save Plan 时，必须与该同一参与者 scope 一致。namespace 的 `library` 权限字段仍独立，不能用它先排除作者库。因 staleBase 等普通前效果错误尚无 Plan 的托管提案，仍进入其准确原库备份；无输入设备 fence 不认领某个库。

共同原 scope reader 同时联核原 WorkspaceId、根 FI 和原准入 Plan 的稳定 locator；即使记录将被归为外库，也不能忽略矛盾 locator。首部保留有界原 JSON 字节作严格 typed decode，重复成员不能经中间通用值覆盖。Actor producer 在持久化前执行相同完整验证：普通 source／field 可以是设备输入，但非空库功能／导航提案必须已有合格库；拒绝时不留下自身 reader 无法读取的提案。纯 `noInput` fence 还须核准确 actor/operations/canonical UUID 文件名，不能隐藏实际 Plan。

Draft reader 从当前真实位置逆向遍历同 workspace 的原 committed relocation，联核当前移动目录 FI，规范叶另核文件 FI；这可恢复整目录内未单独列入事务文件清单的普通输入。旧 Session 的 writer 路径关系不能充当当前物理位置证明，外部回滚到历史旧槽不会覆盖隐藏主稿。同步 publication／local-state 读取同样要求 file-save 原参与者；新发布以原 workspace 选择原 replica，旧路径复用不会把新库保存发布到外库绑定。

普通保存、定义保存及原事务内同物理文件的搬移交付，刷新仍活动的原 Session，不另建输入 producer。刷新先核原保存 Session 或原事务捕获的文件 FI，再核当前文件 FI、Node、actor 与完整原 scope；保留原 producer、完整 Draft witness、已确认输入和冲突状态。文件实际基准未变时也保留 BaseId，重复 status 不使同一编辑器的请求失效。若实际基准已变且不匹配原事务捕获基准与原提交 observation，返回独立比较 Session，不更新原 writer 的基准或输入。保存失联后的查询显式携带冻结请求的原 Session；不带 Session 的通用回执查询及 SaveAs 仍返回独立 Session，不能从回执推导延续权。迟到输入继续属于原 writer，多窗口的独立提案不按文本合并；重启后仍走完整记录 CAS 的显式恢复。

Draft 的每条主／备选输入区分普通 `proposal` 与 `committedBaseline { operationId }`。只有真实普通保存提交后，Core 才能签发后者。原 `FileSaveData` v4 在 fingerprint 和原 commit 中绑定准确 `draftProducerId`；Plan 将 `fileSave, identity` 放在作者载荷之前。消费基线前，在同一保护句柄上先核这两项原元数据，再有界读取并重核持久 Plan 与保留事实一致，走现有唯一 commit 校验器，核 scope、FI、producer、已知 Base、准确 afterimage 及已提交搬移路径。证据缺失、损坏或不一致须失败并保留当前完整 Draft，不能把加载时缓存当作原件。

关闭回读复用同一受保护 Draft 记录读取入口，先核原 scope 与 FI，再读正文，并以原完整字节计算回执。搬移路径证明同时覆盖原 files 与 Relocate／Compound 的 leaf／members，按准确 FI 和最具体原映射核对；完整新记录通过资格后才归档。历史／比较仅合并来源已知且 producer、完整输入、角色和原号均相同的投影；未知来源及显式保留的当前缓冲区不按同文折叠。

普通 persist 只可保留同一 producer 已 ACK、仍在完整记录中且输入依据未变的既有基线角色；输入变化即回到 proposal，Undo 回到同文也不能重新获得资格，旧保存重试不重装基线。打开与合并沿同一分类规则逐条消费已验证且 producer 已不活动的基线，不转移其 producer。混合记录保留其他完整输入，必要时提升完整条目，并在交付恢复 DTO/witness 前原子更新当前记录；全是可消费基线时可暂留磁盘投影，直到首次真实 persist。角色与原号参与完整 witness、回读、恢复及预算。原作者字节和可再生基线由原 Plan/commit 保留，正常消费不写 draft-history，也不放宽 128 条／16 MiB 保护预算；未知依据、独立同文输入和活动 producer 仍保留。

所有私有 ActorOperation 读取先在同一 ParentGuard 保护句柄上逐字节读取既有顺序首部 `scope, library?, actor, resource, fingerprint, coreOperationId`（总计 8 KiB、单值 2 KiB），严格核原 scope tuple、原 Plan 和实际 actor/resource/库资格，再有界读取正文并重核元数据一致。Server 的原保存 status 必需原 resource，读取授权在进入 Core 前完成。定义枚举对已证明的其他 resource 只读首部，不读取或计入其正文预算；畸形原 tuple 不能借外域跳过。无 namespace library 绑定的原输入仍必须与当前受信库的 WorkspaceId 和根 FI 一致。纯 noInput fence 则在 4 KiB 内完整核其无输入事实，不能用未核的首部声明掩盖作者内容。

Draft 持久布局必需先 `scope, identity`，再正文；历史枚举仅在原 scope 和 FI 均匹配后计入 128 条／16 MiB 预算。准确 restore 引用若 scope 或 FI 不符直接拒绝，保原 commit 与回执，只报告结果暂不可交付。资格通过后从同一个受保护句柄有界读取、严格反序列化并联核首部；旧布局原件保留，明确 unsupported，不增加旧／新双 reader 或平行 metadata 账本。定义保存 admission sidecar 也先核其原 file-save Plan 和既有 coreOperationId/saveOperationId/actor/resource 首部，再读取预约正文。


### v0.7 作者输入顺序（开发候选）

HTTP 与原生 Draft 写入必须携带同一活动 producer 递增的 `inputRevision`，Core 在完整原稿落盘后才 ACK；旧序号重试还须重新核实当前原记录。保存冻结原已确认序号与原稿摘要，纳入 fileSave v4 参与者、指纹与 commit；Query 定义的生成稿同时绑定生成前的作者 cut。更晚 Y 已 ACK 时，迟到保存可提交 S，保留 Y 为 proposal，并将其已知 Base 接续到真正提交的 S；不得将 Y 改签为已提交 baseline。正常交付继续原 Session，丢回执和重启仍保留 Y。

未携序号的普通保存只适用于没有有序 Draft 的同步生产者（例如 CLI 的独立 Session），不能借用交互 Session 的 ACK。历史恢复另走显式原请求与完整 Draft 保护，不冒充普通作者输入。原号重试、独立 producer、Undo 与同值 ABA 保持原边界；旧 fileSave 原件保留，缺证明不静默升级。产品与故障注入结果须对应实际候选。

### v0.7 原 journal 目录与可持续原件读取（开发切片）

每项原 Plan、commit、观测、fence 与保留的未知证据按原 operation ID 前两个规范十六进制字符进入按需创建的 `journal/00..ff/` 桶中唯一物理位置，文件名与原始字节不改变。Core 独占锁下的一次性原文件重命名可恢复中断，不再保留两套提交读取。每个桶完整枚举最多 5000 项，新增事务先预留结算容量；达到额度须在作者效果前明确拒绝，不能截断当完整或删除旧原件。CommitSequence 仅在进程内从全部原 commit 重新核验重建后递增，不新建持久账本；未知发布阻断后续编号直到重新核原件。历史、同步、owner backup、提醒与恢复消费者使用相同分桶读取。涉及现有原件的一次搬迁与全量读取仍须本机故障注入验证。

分桶容量从原 pending Plan 的固定记录族与文件／目录参与者计算，并在每次私有 journal 写入时保留尚未结算的空间；不另建持久预留账本。迁移先检查所有目标容量与同名冲突，再原对象重命名。Linux 的完整准入只对明确配置的私有 profile 的准确 journal 路径使用分桶／一次迁移扫描预算，作者目录的同名路径没有扩容资格；原所有权、无符号链接与 enrollment 检查不变。

历史恢复也冻结确认稿的 `inputRevision` 与作者正文；生成稿或恢复稿若不是较新 Y 的已知 Base，Y 保持原 Base 并要求比较，不能虚构其见过生成内容。显式恢复／生成替换退休已消费序号；显式 merge/resume 也清除过期 cut 资格，已提交原号仍先对原件重试。Linux exact-path rules 使用实际 POSIX 分隔符，拒绝父目录遍历，不将 `/` 错转成普通反斜杠文件名。

正文的有序 Draft ACK 同时绑定 `inputRevision`（传输序号）、`inputId`（真实编辑代次）和编辑器实际见过的 `baseId`。输入、Undo 和待应用的 literal 都更新代次；已知已提交结果的安装保留代次，不能靠文本相等认定没有新输入。持久 Draft 的主／备选项及 FileSaveData v4 记录此身份，baseline 仅能由同代次及原提交证据延续。

旧 UI Base 的迟到 ACK 只能从同 Session、物理文件和合格 scope 的原已提交 Plan 解释；普通 S 提交可接续为 S，生成／恢复 X 不使 Y 自动变成基于 X。没有已知 Base 的缓存输入显式提交 `retainedBaseSource`，保为未知基础的 proposal 并要求比较。缺少两种基础、同时给出两种或任意未知 baseId 均拒绝。HTTP、WebUI 和原生的活动编辑路径共用此合同。独立同步 CLI 等同步生产者可在从未接受有序 ACK 的独立 Session 上无序保存；已开始有序输入的 Session 不能退回此入口。

worker 的 Started 表示已原子出生的同一受控 job 的身份，在子进程 exec setup 资格确定之前发送。SetupFailed 也必须先完成原 cgroup 空、pidfd 退出及 reap 再交付原 receipt；收不到有效终局时关闭准入。清理后可信 Timeout／Disconnected／SetupFailed 可以修正早到的管道 I/O 分类，保留原 I/O 事实；清理产生的 Cancelled 不能覆盖原预算或 I/O 原因。Query、proofread 和 Office 同族的失败 Result 经过各自既有交付授权与依赖终检后才展开。
