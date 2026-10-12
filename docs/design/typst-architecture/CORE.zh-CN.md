---
source_language: zh-CN
translation_status: source
---

[English](CORE.md)

# Typst Core：文件、提交与运行边界

状态：从需求重新选择技术的作者候选，未接受、未实现。固定输入提交 `d167ae8c1d0346faf62e8e0887cf789de0bf0b89`；需求权威为[需求清单](../typst-native/REQUIREMENTS.zh-CN.md)，本套 [SOURCE](SOURCE.zh-CN.md)、[EDITOR](EDITOR.zh-CN.md)、[FEATURES](FEATURES.zh-CN.md) 与本文共同定义一套新设计。前轮的协议和有界复核保留为历史证据，不成为本套协议依赖。产品、崩溃、断电、同步、Server、安全与性能验收尚未通过。

## 1. 一个文件工作区与一条作者提交路径

Weftext 原生 Rust Core 供 Desktop、CLI、Server、WebUI 和未来 Mobile 使用。Desktop/CLI 本地模式调用本机 Core；远端模式与 WebUI 只调用 Server；断网可留托管 Draft，不在本机授予托管提交资格。Mobile 本地写入须有合格的平台后端。实时协作后置，未来 OT/CRDT 也只能提交明确的作者提案。

普通保存、字段编辑、创建、移动、复制、回收、恢复、批注建议、模板和业务动作统一成为 Core `Intent → Plan → OperationJournal → Receipt`。UI、转换器、Agent、同步器和索引都没有第二作者写入口。转换和外部执行使用可恢复 Job；Job 对工作区的改变仍进入同一 Plan。

持久写事务按实际冲突范围排序。多个用户可同时读、编辑、保留 Draft 和 prepare；一个物理副本的最终 durable commit coordinator 可短时串行，但不是全库长期阻塞或只允许一个用户。不同副本可离线提交普通内容，不因此复制外部执行责任。

## 2. 文件与结构的唯一权威

一个 managed Node 是 `X/X.typ`，正文、声明、字段和作者样式都在该份 Typst 源内。标题和副标题由 SOURCE 的显式作者角色决定，无标题有效；文件名只可作显示回退。资源是普通文件；独立批注为 `weftext.annotations.json`。不增加另一份正文或共享呈现数据库。

根 `.weftext-format` 精确字节为 `weftext.typst.v1\n`；一个 `.weftext-workspace.json` 记录 WorkspaceId、当前 schema 与存储合同选择。标记选择格式，不授予身份、权限或历史连续性。没有有效合同的目录作为未管理/不兼容/不完整内容处理，不猜格式或自动转换。

目录树是 live Node 的唯一 parent/name/path 权威。portable metadata 不另存可独立修改的 parent/children 列表。父的排序策略与子的稀疏手动 rank 只记录树无法表达的排序；自然名称顺序为默认。Trash 的实际目录树决定其内部结构；回收记录可保存原位置、回收批次和恢复成员，因为这些是历史与恢复事实，不能反向覆盖 live 树。恢复目标已占用则冲突，不覆盖。

`.weftext-rules` 显式分类 ignored、visible unmanaged 与 managed 边界，不继承 `.gitignore`。unmanaged 目录是完整不可重入边界；ignored 字节仍纳入备份。不能单独把 canonical `.typ` 分类为 unmanaged。Core 是唯一库存生产者，所有壳与 Query 消费它；操作目标在 ignored/unmanaged 内，或已知 managed subtree 内含此边界时拒绝结构改动。开放树未观察新增的严格排除是强范围能力；§5保留迟到边界的pending处理，不以扫描伪造CAS。

受管目录提供稳定快捷目标：Node目录直接用NodeRef；非Node容器由Core `admitDirectory` 创建 `weftext.directory.json`，只记DirectoryId，不复制parent/name/path。出生/复制fresh ID/移动/duplicate/gap按同一Plan与manifest处理。快捷创建只接纳选定容器，不整改全库。unmanaged边界不自动加ID；若保留unmanaged，只能有明确较弱的locator书签，不能称改名稳定快捷。根 `.weftext-tags.json` 集中登记immutable TagId与可改displayName；TagRef用于稳定快捷/导航/Query。同名不自动合并，literal标签字符串不自发登记对象。Core `createTag`可与笔记typed TagRef赋值同Plan，SOURCE负责该原生typed槽。

portable 控制资料只保存无法从当前文件恢复的事实：身份创建与退休、schema 历史、已提交版本/因果关系、资源文件绑定、Trash 恢复信息。资源的稳定 Ref 通过节点内 `weftext.resources.json` 的资源 ID 与相对文件定位绑定；该文件不决定 Node parentage。普通非 canonical 文件仍可见；未登记资源不被自动授予稳定 ID，原始改名若无法证明原对象则显式重新绑定或冲突。批注 JSON 中的 annotation ID 与其 target/lifecycle 分域。

`.weftext-history/` 保存不可变已提交 manifest 与需要保留的 exact blobs。它是历史、复制与恢复证据，不能被编辑成第二套 current 源。设备/session Draft、活动 SQLite journal/WAL、运行凭据与派生索引放在同步根外。durable commit 到 manifest 发布之间，原 journal 的已提交记录和完整 blobs 是该次身份/版本/结果的真实证据；启动先恢复这些待发布记录，备份必须一致包含它们，不能因 history 尚缺就重新执行。历史快照可能与 current 文件不同；当前读取仍须观察实际文件与当前权限。索引、搜索、关系反向视图、缩略图和图可删除重建，不能重建执行责任或凭空生成 ID。

完整备份包含普通文件、ignored 字节、portable 控制/历史及一致捕获的本机未决 journal/Draft；只复制活动 SQLite 主文件不合格。可单独导出仅资料备份，但须说明它不能恢复丢失的操作/费用责任。恢复到新设备重建本地观察与派生索引；真实未决执行责任只由原日志或显式责任接管恢复。

## 3. 身份、版本与当前读取

保留随机 UUIDv4：WorkspaceId 与各 Node/Resource/Annotation/Directory/Tag ID 使用完整类型化 `EntityRef`，解决改名稳定引用、离线创建及复制独立身份，且不把名字/路径变成身份。UUID 的价值是稳定随机标识，不是安全凭证。Directory/Tag只补导航必需身份，不成为笔记或逐值事实ID。Core 分配新身份；源码/sidecar中的 ID 是声明。已有身份须匹配已接纳创建事实、当前唯一文件绑定和 lifecycle；重复 ID、外部复制带 ID、缺元数据或 partial sync 不自动新建或重发 ID。

创建工作区在私有 staging 构造完整根配置、schema、空历史和所有目标字节，验证目标不存在后 create-only 发布；初始化失败不半激活。Server 的创建者权限只由受权服务管理行为产生，客户端 metadata 不赋权。真实已有目录采用显式 import/admit；保原件、未知字节与损失，不改扩展名冒导入。

`RevisionId` 是冻结 Plan 时私有分配的随机标识。commit 后才有效，manifest 一次关联 EntityRef、RevisionId、exact blob、因果父版本与事务。它不依赖包含自身的源码摘要，能在最终 `.typ` 中构造 self/cross 固定地址和批注锚点。计划私有 ID 不作为公开成功；真正 no-op 保持版本，外部同字节接纳是新的来源接纳事实。

`ReadBasis` 是本次 Core 的当前资格，含持久版本或明确 external observation、observer incarnation、真实文件对象与变化 generation、lifecycle、schema/授权世代以及实际依赖。hash 只校验字节完整性，不证明对象身份、权限或条件写。watcher gap、重启、替换、ABA、partial materialization 与相关依赖变化使旧 basis 失效；重建后重新读取可以获得新 basis，不能复活旧 plan/selector 的写资格。

`DependencySet` 只有一套 typed object、lifecycle、structure/range、schema、authorization、environment 与 World read 类型。完整负范围和不存在也须从当前获权枚举证明；placeholder、I/O 错误、隐藏对象或未扫索引不是不存在。无关对象改变不使一个只依赖本篇的保存失效。需要全集/约束的强动作须证明实际范围，不能因为普通保存成功而声称全集完整。

普通获权原始读取可保留合法或无效 external bytes；external 未接纳状态不获得持久 typed 写地址。普通整源保存可在短时保护窗口核对同一 external Base 并显式接纳，不要求先编译整篇。保存语义独立区分 local_valid / semantic_pending；完整强动作的跨对象条件未证不能降级成功。

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

**普通同卷移动。** 默认语义是“移动已绑定的目录对象到指定父目录，basename不变”，不是复制冻结全树。Core按Ref、实际目录/父对象和边界授权，持有DELETE权限、允许read/write分享但不共享DELETE的目录句柄；durable installing先记operation、源/目标对象、定位、no-replace和恢复责任，再一次同handle rename。目标占用失败，绝不替换。busy保稿无变化有限重试，不长期锁每个child或重复制整树。移动的是同目录对象及rename瞬间其中的树；之前新增child与外部修改跟随同树，不被丢弃。之后子内容更新仍是独立external observation。

Receipt只证明placement变动，不声称移动时全树bytes固定。source RevisionId保持原版或另需接纳的external状态，路径依赖和当前ReadBasis重取；同步不得借placement的旧blob覆盖新正文。“移动指定快照且全树不变”是另一强Intent。ordinary move有可实现的同对象语义，不需先取得任意全目录CAS。

**Node改名。** `A/A.typ → B/B.typ`是两次no-replace：先按主源handle把leaf改名，再关闭会挡父rename的leaf句柄，用仍受保护的目录handle改外目录，最后重绑验证canonical角色与实际对象。任何副作用前durable保存两对象/旧新名称/步骤和回退责任，Core barrier遮中间。源ID/bytes不改，不造source新版。外部重建旧leaf、替换目标leaf、占用外目录或归属不明进入pending_reconciliation；保真实两稿/路径，仅原对象且原位置空时no-replace返回，不覆盖第三态。不凭B/B.typ存在或相同bytes造saved；两rename不对外部工具原子。

**Trash、restore与purge。** Trash/restore也移动同目录对象到唯一操作目录/获准空位置，原Tree与字节保留，批次/恢复元数据同日志提交；并发内容仍保在实际树。purge是破坏性强操作：在Core控制Trash staging取得完整关闭成员集、必要句柄和边界/责任才删明确对象；未知新增、未决writer或完整性不明暂停，不使用无限制recursive delete。ordinary move/Trash不因此全禁。

**copy、template与跨卷。** copy选明示snapshot成员及exact获权blobs，同map分配fresh ID/验证refs，源不动。目标在真正宿主管控且不处于普通sync/edit路径的staging完整构造，no-replace发布后验证归属；private名字不是隔离证明。source后来新增/改稿不删，也不称已经复制。要求完整树但snapshot成员未证时保提案/待协调，不偷少复制。template限固定完整输出，多根逐一记发布/回退，第三目标不覆盖。跨卷move先snapshot-copy，源仅在完整成员与变动资格匹配后移Trash；否则准确报copy完成而move待协调，保两树不删新稿。

已知ignored/unmanaged边界在动作前拒，扫描不重入。开放树late boundary在rename前发现则拒，之后发现则pending_reconciliation并保实际树，安全no-replace返回或明确保现位置；不覆盖旧路径第三方对象。不能从扫描宣称排除任意未观察新增。绝对要求rename瞬间不存在未知boundary/兄弟/唯一值是强negative-scope，须受控namespace。普通move的同对象合同已选定；平台目录耐久/crash资格待实测，不能因套用全树快照承诺把所有普通move永远留P0。

有限Windows probe验证：同对象no-replace move成功，先前新增child与外部修改随树保留，busy child关闭后重试成功，目标占用保两边，Node leaf/outer两步成功，旧leaf重建后no-replace回退失败且保两稿。本例child即使shareALL仍使父rename失败5，关闭后成功；directory share=0不挡child新增且挡另句柄枚举，Node路径改用允许read/write而拒delete的目录句柄。probe不证明断电/完整恢复/generation/别名/filter/Server。

**Server 与后续平台。** Server writer service 使用 OS 隔离目录，普通客户端和 worker 不直写；fence 每次实际文件动作和日志决议。该部署可实现强 namespace 与自动恢复。Linux 的普通 advisory 锁无法阻止任意同用户工具；Linux 首轮可提供受控服务目录或明确的独占宿主模式，开放本地写后端待单独证明。网络文件系统逐项认证。可选 checkout→三方导回服务于长期外部编辑或不合格后端，保留原 Base/current/proposed；不是正常 Desktop 保存的强制步骤。

不需要抵抗磁盘所有者刻意绕过隔离、直接损坏日志或恶意内核；普通编辑器/同步器竞态、已有句柄和 mapping 在需验证范围内。范围外行为不产生声称“任意外部写都安全”的承诺。无所需保护时保稿并返回该动作的明确边界，不暗降级到覆盖未观察稿件的 observed-only save。

## 6. 可执行提交与恢复算法

1. 查原 operation 记录，已 committed 只按当前披露权交付原 Receipt；同 ID 指纹不等拒绝。首次计划将 exact before/after、分配与有限预算计数持久 stage；Draft 留存先于破坏性写。
2. 按固定顺序取得不会改变作者 namespace 的文件保护与 Core read barrier；忙则释放全部已取保护、保留 staging 后有限重试。取得后从真实对象重读并重验实际依赖、schema、lifecycle 和当前授权；变化是 conflict/reprepare，不重新采样替换原冻结 Plan。结构动作此步只取目录对象资格，不实际rename。普通move不冻结child bytes；copy/强动作按其snapshot/完整范围要求。
3. 在活动 journal 原子保存完整 frozen Plan 和 `installing` 决议并 flush。任何要修改的 before 与 after blob 必须已经耐久，记录每个实际对象、捕获路径及安装步骤；SQLite 事务只证明此日志决议，不证明外部文件已写。
4. 持保护执行内容写或§5同对象namespace步骤；所有副作用在durable installing之后。copy/template发布固定after；ordinary move不变成copy。每步验证实际对象、写入长度和 exact bytes，并落实文件及 directory-entry 的后端耐久要求。日志记录步骤，但“步骤完成”本身不能替代当前文件归属证明。发生 I/O/磁盘满/取消/失权，停止新的步骤并进入原操作恢复责任，保 before/after/实际第三态。
5. 全部 after 在同一 Core barrier 中核对成功，原Intent结果可证（move为同目录对象到目标、rename含canonical两步、copy为snapshot全部输出、强业务须完整范围），当前授权及所需强依赖仍成立后，一次 durable journal commit 固定事务、有效 RevisionId、完整 Receipt、Job 作者步骤和待发布 manifest。该 marker 是产品 saved 的判定。不满足原请求则保原ID/恢复责任并报pending_reconciliation，不造子集成功；纯placement不发source RevisionId或全树bytes证明。再释放保护，异步更新派生索引/发布 immutable manifest；索引或同步失败不撤回 saved。
6. 响应丢失由 `operationStatus` 查询原 ID。未看到响应不重新创建、不重新抽 UUID、不重复费用或外部效果。同步状态独立为 pending/published/conflict，不拿上传成功冒保存。

这些步骤提供 Core 读者的多对象原子可见性和恢复责任，不能给普通外部工具多文件瞬时原子。Server 受控读写路径或合格后端可保证计划条件在安装窗口不变；开放目录对不受保护 namespace 的强条件必须拒绝未证明承诺。

恢复先查 durable operation，再观察实际文件；没有 commit marker 不凭 after hash 造 saved。原计划的每个对象分为 `before`、`after`、`third_state`、`unavailable`，分类要求准确字节、对象和安装归属，不是字节相等即可。连续受控后端证明只有原 writer 修改时，可继续原计划；开放本地崩溃使句柄释放，之后的 partial/替换归属不明就先保存真实残留，再呈现原 Base/恢复 Draft/第三态，由明确 repair/merge Plan 处理。不得自动覆盖，不能只把 before/after 留住就声称未知外部稿也留住。

已 committed 的历史结果不受后来 r6 使 r5 Base stale 的影响，重放只交付当时结果，当前读取仍读取当前。恢复不拿 current 改写旧 Receipt/manifest，不重签旧事务；失权可以暂停或遮蔽恢复交付，不能让既成事实变成待重执行。third_state 只阻断受影响范围，无关文档可继续。Undo 是当前 Base 上的新 Plan，不倒退全库或复用旧提交。

## 7. 同所有者同步与冲突

managed 同步传输一个 immutable `CommitManifest + exact blobs` bundle：WorkspaceId、事务、因果 parents、writer replica、所有真实出生/改动/退休、before/after RevisionId、结构/schema 改变与准确 transform。同一 manifest 的校验摘要覆盖整个对象；校验失败/缺 blob/placeholder/父历史缺失只进入 staging，不成为新 head。

默认信任自己的设备与传输账户，摘要用于完整性；不声称抵抗 malicious transport。若部署承诺不可信 transport/撤销设备防护，采用一个登记设备身份与一个签名 commit 域，签完整 manifest；历史按当时有效登记/撤销因果 cut 验证，冲突撤销不能被选择另一 branch 抹除。无需两个专用签名 profile、CP/Notice 和每个 revision 的独立 seal。

同步器运输 bundle，不直接按到达顺序写 current 作者树。接收端验证完整身份、同 Workspace、因果闭包和本模式信任，再由 Core Plan 安装；本地观察与授权重新建立，不复制发送端 ReadBasis/编译许可。同步目标选择用户指定 transport 目录；运行工作树与它分离，仍是普通可编辑/备份文件。

普通云盘把整个 raw 工作树直接同步时视为外部改动，保原文件与当前已提交 blobs，partial 和重复 ID 明确诊断；不把它冒称上述 managed 因果同步，不让 provider 的覆盖决定 winner。用户可直接编辑工作树；同 owner 多设备的可靠并发应使用 bundle transport。尚未被任何设备保存/观察且已被 provider 删除的 raw bytes 无法凭空恢复，不能承诺保全。

单一因果后继按实际依赖 fast-forward；独立对象的并发更改可由确定 Plan 合成，合成 manifest 保所有 parents。涉及同源、移动/编辑、Trash/编辑、schema 或受约束范围的并发保各 head 与共同 Base，明确 merge/choose/fork；不按 mtime/到达/digest/文件名自动选胜者，不随机重发身份。冲突对象可看获权分支并保 Draft，写解决 Plan 前需重新资格；无关对象继续。

portable 文件复制绝不复制 Approval 次数、费用、provider unknown 和调度执行资格；这些只在连续 fenced Job execution store 中。即使复制完整资料库也只能开始普通内容副本，不能自动继续外部作业。

## 8. Server 认证、披露与窄字段

Server 由受信部署选择工作区、schema 与授权策略；认证 actor/session/当前授权世代在实际服务端产生。客户端源 metadata、UUID、类型、地址、指纹和历史 manifest 均不授予 hosted 权限。资源、源、字段、批注摘要、批注目标及历史分别授权，deny 优先。磁盘所有者本来能读取的本地文件不受应用 ACL 保密；共享 raw 可写目录也不能获得 Server 级隔离。

撤权与提交/交付在 Server 边界线性化：源/字段读取、Query页、缓存命中、编译输出、预览、缩略图、下载每次 chunk 与最终 commit 重验。已经交付的字节不能撤回，撤权后不继续交付。受保护审计保准确 actor/意图/结果，不向无权调用方泄漏标题、路径、数量、存在与失败差异。Receipt 已 committed 可仍不披露；不可披露不改变事实。

多个用户保各自 Draft；同文档 A 保存 r6 后，B 基于 r5 的 Plan 进入冲突并保双方稿；不同文档不因一个全局序号变化全部失效。Server failover fence 必须实际保护 journal 和每一次文件写/rename；只有 SQL lease 不够。推荐单 writer service 持有 OS 文件权限，Server 实例发带当前 fencing generation 的操作，旧实例无直接路径写权；写服务切换先隔离旧进程/卷访问，再启新 holder。

字段窄权通过 Server 内部 SOURCE 静态独立性合同和受保护 before/after 检查取得 opaque field qualification。只能读/写明确字段；不得先给客户端整篇/拓扑再删隐藏字段。可能影响 hidden body/其它字段或动态 World 的未知情形返回不泄密 unavailable；完整 source 授权另走 source 路径。管理员配置独立性不能由客户端 metadata 自证。字段轻量保存无需为了普通输入编译 hidden body。

仅 Field-only API 的客户不获全 manifest/全库同步权限。浏览导航/Query、摘要缩略图和回收站始终遵守当前披露；Trash 获权查询只读，不授予 restore/write。结果 handle/cursor 固定 audience、定义/参数/schema、实际依赖与完整性，不成为作者权限。

## 9. 授权编译、查询与作业

`Environment` 一次绑定官方 compiler/syntax/stdlib、语义桥 build、target、冻结 date、包/字体锁、资源策略与硬预算。库默认样式按 [EDITOR](EDITOR.zh-CN.md) §8：唯一样式内容是获权普通 `.typ` module，可为真实ResourceRef或NodeRef；Core设置只选该EntityRef及启用。compile透明生成entry（import/style show/include真实note），entry无身份，真实note仍是SourceSnapshot与作者ID/字段权威。Environment冻结module/settings/entry profile与实际World依赖；style change不发note RevisionId，却失效实际preview/editmap/semantic Query，未消费样式的静态field不失效。各 consumer 引用它，不逐槽复制 carrier。Core 提供获权 snapshot World，不把宿主目录挂给 compiler；include/import/read/数据/图片/字体/plugin 路径统一受控，正/负实际读取完整记录。环境、字体fallback、资源不存在或授权变化失效真实 consumer。

`CompileSnapshot` 保存 exact SourceSnapshot、Environment、完整实际 World dependencies、最终官方 semantic projection/origin map、输出或诊断。合法源/body 编译或投影失败仍保原源、Draft 和可用静态 fields；旧预览不能冒当前。worker 无 ambient 文件/网络/凭据/写权，有可终止的时间/内存/输出预算；无完整追踪不给强资格。编译不会创建身份、保存或隐式调用业务动作。

Type/Value、schema、Query/CEL/参数、View/Action 和导出共享 SOURCE 的一套语义，原 spelling 与 AST bytes 单独保留。NAV/搜索/过滤与 FEATURES Query 相同；partial/stale/error 与 complete empty 分开，索引候选须回读。普通 saved query 引用默认 current 并在执行时冻结真实版本；显式 pinned 永不换 latest，历史结果永不复活写资格。完整 evaluation 成功后才发布 complete handle；获权 partial 可发布带明确 coverage/缺口/状态的 audience-bound 只读 handle 供分页浏览，不证明全集/empty/缺失，也不授全集批量或强后条件资格。分页不把页上限当全集。批量操作固定全部 target，commit 不再跑 Query 扩大集合。

Job 固定输入版本/输出/loss/确认/预算/actor，保存步骤、attempt/effect key、批准消费、费用结算与 stop。建议应用把目标源+annotation accepted 状态同 Plan 提交；转换输出 staged 不代表文档已保存或下载交付。下载固定确认的 exact output，续传不换内容，逐交付授权。外部请求 unknown 保存原 attempt，先查询 provider；只有被证明同 key 幂等/条件写才可安全重试，不声称跨网络 exactly-once。stop 阻止之后获权步骤，不撤回已发生效果。

保留策略保护未决操作、冲突 heads/Base、精确批注历史、固定版本引用、正在消费的结果/导出与 external unknown 的必要 blobs；其它 cache 可清理。GC 不删除幂等最小事实、tombstone 或费用责任来重新执行，也不让所有投影无限保全。新已发布合同今后升级须有版本识别、保原件、显式 staged 转换、失败恢复与真实客户端兼容表；未部署旧候选不装 runtime decoder。

## 10. 最小消费接口

| 接口 / 对象 | 各 owner 的责任 |
| --- | --- |
| `readEntity` / `readScope` / `validate` → `SourceSnapshot, ReadBasis, DependencySet` | Core 负责获权字节、当前对象和完整范围；SOURCE解释官方源/值，FEATURES消费同查询范围，EDITOR不从文件再扫描。 |
| `prepare(Intent, bases)` / `commit(operationId, planHandle)` / `operationStatus` | 所有作者动作共用计划/日志/结果；同ID精确重试。SOURCE局部patch、FEATURES业务条件、EDITOR Draft不另开提交入口。 |
| `observeExternal` / `admitExternal` / `merge` | 捕获原 bytes、原 ID 冲突和不完整状态，明确接纳/合并，重新资格。 |
| `compile(SourceSnapshot, Environment)` → `CompileSnapshot` | Core World/权限/worker；SOURCE与EDITOR消费同次最终语义和origin，不造第二解释器。 |
| `evaluateQuery` / `readResultPage` → `QueryResult` | FEATURES定义语义，Core绑定主体/版本/依赖/完整性；显示、导航和导出消费一份结果合同。 |
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
