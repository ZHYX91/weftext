---
source_language: zh-CN
translation_status: source
---

[English](CORE.md)

# Typst Core：文件、提交与运行边界

状态：**design-refinement-not-implemented**；固定旧基线的独立接受保持为历史事实，当前修订及证据限制见 [REVIEW](REVIEW.zh-CN.md) §8。

## 1. 一个文件工作区与一条作者提交路径

Weftext 原生 Rust Core 供 Desktop、CLI、Server、WebUI 和未来 Mobile 使用。Desktop/CLI 本地模式调用本机 Core；远端模式与 WebUI 只调用 Server；断网可留托管 Draft，不在本机授予托管提交资格。Mobile 本地写入须有合格的平台后端。实时协作后置，未来 OT/CRDT 也只能提交明确的作者提案。

普通保存、字段编辑、创建、移动、复制、回收、恢复、批注建议、模板和业务动作统一成为 Core `Intent → Plan → OperationJournal → Receipt`。UI、转换器、Agent、同步器和索引都没有第二作者写入口。转换和外部执行使用可恢复 Job；Job 对工作区的改变仍进入同一 Plan。

持久写事务按实际冲突范围排序。多个用户可同时读、编辑、保留 Draft 和 prepare；一个物理副本的最终 durable commit coordinator 可短时串行，但不是全库长期阻塞或只允许一个用户。不同副本可离线提交普通内容，不因此复制外部执行责任。

## 2. 文件与结构的唯一权威

一个 managed Node 是 `X/X.typ`，正文、声明、字段和作者样式都在该份 Typst 源内。标题和副标题由 SOURCE 的显式作者角色决定，无标题有效；文件名只可作显示回退。资源是普通文件；独立批注为 `weftext.annotations.json`。不增加另一份正文或共享呈现数据库。

根 `.weftext-format` 精确字节为 `weftext.typst.v1\n`；一个 `.weftext-workspace.json` 记录 WorkspaceId、当前 schema 与存储合同选择。标记选择格式，不授予身份、权限或历史连续性。没有有效合同的目录作为未管理/不兼容/不完整内容处理，不猜格式或自动转换。

目录树是 live Node 的唯一 parent/name/path 权威。portable metadata 不另存可独立修改的 parent/children 列表。父的排序策略与子的稀疏手动 rank 只记录树无法表达的排序；自然名称顺序为默认。Trash 的实际目录树决定其内部结构；回收记录可保存原位置、回收批次和恢复成员，因为这些是历史与恢复事实，不能反向覆盖 live 树。恢复目标已占用则冲突，不覆盖。

`.weftext-rules` 不继承 `.gitignore`。Core 的单一库存分别记录文件存在/可见性、Folder Node 资格及显式节点识别边界：ignored 从普通列表隐藏并排除节点发现，字节仍备份；unmanaged 可见、可展开并可打开普通文件，不获得 Node/typed Query 资格。无同名主文件的目录与非配对 `.typ` 默认标“不管理”，不因此停止向下普通枚举或发现未被显式排除的后代配对。用户显式将目录子树设为不管理时，节点发现/批量整理不重入该子树，普通文件浏览仍可进入。完整节点的 canonical `.typ` 不可独立设为不管理；显式退出节点管理须预览身份/引用后果，不自动移动或删除文件。

节点结构动作仍拒绝显式 ignored/unmanaged 边界内的目标，或已知子树含这类排除边界的动作；普通非节点文件动作经同一安全后端独立预览确切范围，不借普通改名/移动绕过受管节点的两侧配对、已登记资源及历史责任。已接纳节点突然缺主文件或改配对是诊断/恢复事件，不能静默降为普通文件、退休身份或猜新 ID。开放树未观察新增的严格排除仍需强范围能力；§5 的迟到边界 pending 规则针对这些显式排除边界，不把普通容器缺主文件误当同一阻断条件。

受管目录提供稳定快捷目标：Node目录直接用NodeRef；非Node容器由Core `admitDirectory` 创建 `weftext.directory.json`，只记DirectoryId，不复制parent/name/path。出生/复制fresh ID/移动/duplicate/gap按同一Plan与manifest处理。快捷创建只接纳选定容器，不整改全库。unmanaged 边界不自动加 ID。用户可明确只登记目录导航身份，保留其非 Node 状态和原发现边界；这不创建主文件、Node 或 typed 业务对象。拒绝登记或后端无法证明稳定目录身份时，仅提供明确较弱的 locator 书签，不能称改名稳定快捷。根 `.weftext-tags.json` 集中登记immutable TagId与可改displayName；TagRef用于稳定快捷/导航/Query。同名不自动合并，literal标签字符串不自发登记对象。Core `createTag`可与笔记typed TagRef赋值同Plan，SOURCE负责该原生typed槽。

portable 控制资料只保存无法从当前文件恢复的事实：身份创建与退休、schema 历史、已提交版本/因果关系、资源文件绑定、Trash 恢复信息。资源的稳定 Ref 通过节点内 `weftext.resources.json` 的资源 ID 与相对文件定位绑定；该文件不决定 Node parentage。普通非 canonical 文件仍可见；未登记资源不被自动授予稳定 ID，原始改名若无法证明原对象则显式重新绑定或冲突。批注 JSON 中的 annotation ID 与其 target/lifecycle 分域。

`.weftext-history/` 保存不可变已提交 manifest 与需要保留的 exact blobs。它是历史、复制与恢复证据，不能被编辑成第二套 current 源。设备/session Draft、活动 SQLite journal/WAL、运行凭据与派生索引放在同步根外。durable commit 到 manifest 发布之间，原 journal 的已提交记录和完整 blobs 是该次身份/版本/结果的真实证据；启动先恢复这些待发布记录，备份必须一致包含它们，不能因 history 尚缺就重新执行。历史快照可能与 current 文件不同；当前读取仍须观察实际文件与当前权限。索引、搜索、关系反向视图、缩略图和图可删除重建，不能重建执行责任或凭空生成 ID。

完整备份包含普通文件、ignored 字节、portable 控制/历史及一致捕获的本机未决 journal/Draft；只复制活动 SQLite 主文件不合格。可单独导出仅资料备份，但须说明它不能恢复丢失的操作/费用责任。恢复到新设备重建本地观察与派生索引；真实未决执行责任只由原日志或显式责任接管恢复。

### 2.1 普通文件会话

`openFile` 可打开库外或库内未管理的 `.typ`，不初始化工作区、不注入 metadata，不因文件自带 id 就接纳为 Node。会话目标为实际文件对象与授权路径范围；FileBasis 记录原字节、真实对象/路径绑定、观察世代和依赖根，与 NodeRef/RevisionId 分域。它复用 Draft、单文件保存后端、操作日志/恢复与冲突检查，不生成伪 WorkspaceId、portable manifest 或 schema admission。语法无效仍可准确保存原文并显示诊断；写权限、读权限和编译资格分别判断。

首次打开以父目录作为明确可见的默认项目根，可由用户调整。相对路径以各实际来源文件为基准，根相对路径以所选项目根为基准；依赖只读访问须在授予范围内，越界拒绝，不能把整个磁盘交给 compiler。沿用官方包解析及明确的离线/联网策略、应用字体默认；无所属库时不套用当前库样式/schema。依赖文件编辑另开普通会话。

保存比较当前文件对象和准确 Base，保外部并发稿；不能只凭 hash 声称 CAS。另存默认 create-only，已存在目标必须单独预览和取得它自己的当前写依据；来源不变。可用平台提供同一安全后端；WebUI 没有本机文件写能力时说明上传副本/下载另存的实际语义，不冒原位保存。恢复稿存于应用私有区而非在用户目录造控制文件。对已管理文件重定向回其 Node/Resource 会话，不允许绕过其合同。主动纳入库时预览新节点/必要元数据及文件安排，成功前保原件。

## 3. 身份、版本与当前读取

保留随机 UUIDv4：WorkspaceId 与各 Node/Resource/Annotation/Directory/Tag ID 使用完整类型化 `EntityRef`，解决改名稳定引用、离线创建及复制独立身份，且不把名字/路径变成身份。UUID 的价值是稳定随机标识，不是安全凭证。Directory/Tag只补导航必需身份，不成为笔记或逐值事实ID。Core 分配新身份；源码/sidecar中的 ID 是声明。已有身份须匹配已接纳创建事实、当前唯一文件绑定和 lifecycle；重复 ID、外部复制带 ID、缺元数据或 partial sync 不自动新建或重发 ID。

初始化也使用§5.3的create-only边界，不假装普通同用户staging可一次原子发布整库。先durable bootstrap operation、WorkspaceId、配置/源拟议版本和完整after blobs，再在获权目标父闭包中以§5.3直接返回原目录创建handle的原语create-only建立目标root及必要子目录并持续保留同handle；逐文件CreateNEW并持写保护构造root config、schema、根marker和全部计划源。外部可先看到partial文件，但Core发现installing/birth缺失时只返回incomplete，不按marker存在激活。原目标已存在则冲突，不接着初始化其中。只有所有计划对象/manifest/journal与current权限完整，单一commit marker才激活Workspace和creator grant，随后portable publication。

初始化失败保原ID和可恢复计划，未知新增/对象被替换不覆盖或递归清理；仅本操作归属可证的文件和实际empty目录允许条件回退。丢响应查询原operation，不能重分Workspace/Node ID或重复赋creator权限。Server初始权限来自受权issuer配置，不来自client metadata；首次初始化后不得借重启/恢复补赋creator grant。真实已有资料目录走显式import/admit，保原件/unknown/loss，不改扩展名冒导入。

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

所有guard保持到实际操作/验证/durable commit完成。通过source自己有DELETE权限的同一handle执行rename，不关闭source guard后按path移动；目标父guard保留。若API需要目标父handle相对名称则用实际父handle；已验证的volume路径API可在整条链被保护时使用。后端需分别验证该API组合，不从SQL锁推导。

### 5.2 重叠、移动和Node改名

普通同卷move是一项已绑定目录对象的no-replace rename，basename保持；移动前新增child/外部修改随同该实际树，移动后变动是新external observation。Receipt只证明placement，不证明全树固定bytes或发source RevisionId；Trash/restore同样把原目录对象移至唯一Trash批次/获权空位置，保原树/字节和恢复元数据，不递归删内容。

destination parent等于source对象或在source后代链中时拒绝，不能递归搬进自己。source/target实际同位置是qualified no-op；同名另一对象不是no-op。共享父只一个guard。源/目标卷不同进入明确copy+有条件Trash组合，不把跨卷包装成同对象rename。

Node `A/A.typ→B/B.typ`仍两步：为目录与canonical leaf保存准确对象/名称/恢复责任，leaf rename no-replace后关闭妨碍outer rename的leaf handle；目录guard和完整祖先/目标父闭包保持。outer rename后重新取得实际canonical file guard，验证原对象/控制角色和公开声明的外部改变。新leaf/新root第三态只pending；no-replace回退绝不覆盖。这个释放leaf的窗口不持久授予旧source写资格，也不声称全树内容固定。

普通source save也使用文件的父/祖先闭包加该文件RW share0 guard，在同一文件handle核Base并写/flush/commit。可捕获的ancestor/target-parent改变不能在最后检查后绕到另一库；busy保稿重试。所有文件guard的范围有限，不要求全库锁。

### 5.3 copy与template：固定snapshot成员，不虚构私有目录隔离

默认普通Windows后端不把同用户“private文件夹”当隔离。Plan固定源snapshot成员和exact blobs、一个fresh map、所有目标目录/文件及预算。源snapshot是逻辑cut的已接纳版本或明确获权选中观察；若要求完整subtree而成员未证则coverage/pending，不能悄悄少copy。

安装算法：

1. 在操作日志及exact blobs已durable后取得§5.1目标祖先/父闭包和Core范围reader barrier。预算预留计划文件数对应的同时handle容量；不足在安装前失败/显式改批，不先半写。
2. 一个后端调用create-only建立计划目录并返回其原guard，不采用`mkdir → reopen`。Windows候选使用[NtCreateFile](https://learn.microsoft.com/zh-cn/windows/win32/api/winternl/nf-winternl-ntcreatefile)：`RootDirectory`是已持target-parent handle，name仅单个相对目录分量；`FILE_CREATE`、`FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT`，LIST_DIRECTORY/READ_ATTRIBUTES/DELETE/SYNCHRONIZE access，share READ|WRITE而不shareDELETE。要求成功且`IO_STATUS_BLOCK.Information=FILE_CREATED`，日志登记返回对象证据并持续保留同handle加入闭包。已有同名目录冲突，不在其中合并。初始workspace root也对获权外部parent使用同一调用。普通外部工具可能先看见目录，Core reader仅见incomplete/installing。有限本机NTFS probe已验证返回创建handle、同名collision与外部root rename被拒；生产平台/耐久资格仍待验。只有mkdir/reopen的后端不能伪造创建归属，必须提供真实等价原语或保持该scope pending。
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

这仍是未接浏览器入口的 Core 开发切片。现有对象的位置与回收状态变化、移除旧文件，以及语义依赖已变化的情况仍明确拒绝并保接收版本；对应结构事务与重新资格检查继续实施。外部修改的显式收编、接收但未安装的历史展示、目录传输 UI 与完整 Server 消费者也未完成。新副本明确为选定对象及完整事务／依赖闭包的部分副本。上述测试不等同于 HTTP、浏览器、多用户全消费者或完整 v0.6 验收。

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

中文默认依 [EDITOR](EDITOR.zh-CN.md) §8.6 落实。字体锁固定可用字体集合及其准确文件版本/许可与回退环境，不能只记最终已用字体而忽略集合新增改变回退选择。Desktop、Server/CLI 和 WebUI 消费同一已准入集合；宿主系统字体只有显式准入后才能参加该环境，worker 不扫描 ambient 字体。浏览器获权同源交付遵守资源授权/撤权，不增加外网权限。字体集合、库语言/样式或实际依赖变化失效受影响 compile/preview/Job 准备；静态字段和 note 原字节不因字体变化而改动。已确认下载仍交原 staged bytes，缺当前交付资格则停止而不重编译替换。

### 9.1 共同LogicalCut及current发布


#### 9.1.1 唯一producer与打开算法

Core commit marker原子更新一个journal generation及该代所有已接纳 source/config/lifecycle/placement facts；多对象事务只有一个generation。构成cut的per-entity版本视图来自这些真实committed facts/保留blobs，不是连续若干`read current`得到的集合。

`openLogicalCut(scope,purpose,budget)`：

1. 验证caller当前潜在scope与最低披露；取得短commit/read barrier，等待该scope正在installing的Core事务变成committed或明确未可用，不读取半安装state。
2. 在同一journal读事务取已决generation `g`、完整已接纳causal heads、per-entity exact RevisionId/blob/lifecycle、目录结构历史视图和schema/rules/settings版本。scope membership/负范围只能来自该`g`可证明的完整已接纳库存；building/未知raw边界不变empty。目录/config视图来自原操作事实，source版本同时包含SourceSchemaBasis，相关原配置必须被retention保护。
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


Job 固定输入版本/输出/loss/确认/预算/actor，保存步骤、attempt/effect key、批准消费、费用结算与 stop。建议应用把目标源+annotation accepted 状态同 Plan 提交；转换输出 staged 不代表文档已保存或下载交付。下载固定确认的 exact output，续传不换内容，逐交付授权。外部请求 unknown 保存原 attempt，先查询 provider；只有被证明同 key 幂等/条件写才可安全重试，不声称跨网络 exactly-once。stop 阻止之后获权步骤，不撤回已发生效果。

保留策略保护未决操作、冲突 heads/Base、精确批注历史、固定版本引用、正在消费的结果/导出与 external unknown 的必要 blobs；其它 cache 可清理。GC 不删除幂等最小事实、tombstone 或费用责任来重新执行，也不让所有投影无限保全。新已发布合同今后升级须有版本识别、保原件、显式 staged 转换、失败恢复与真实客户端兼容表；未部署旧候选不装 runtime decoder。

## 10. 最小消费接口

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
