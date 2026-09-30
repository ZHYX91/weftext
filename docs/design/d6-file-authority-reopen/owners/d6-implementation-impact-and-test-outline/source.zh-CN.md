---
_weftext:
  id: "3762120f-fb70-4cf7-9699-68602e9bf9fc"
---

候选状态：D6-FA-r01；partial coordinated candidate；未接受、未激活、未实现。固定 S 中 revision05/D7/D9 历史验收文字仅作来源记录。稳定文档 ID 保持。本文件只定义未来实施/验收义务，不授权代码、依赖、发布、部署或 A2。

# D6 Implementation Impact and Test Outline

## 1. 实施切片与唯一 owner

D6-FA-r01 的实现必须按逻辑 owner 分开，禁止为了方便重新合并成“一个数据库保存一切”。

| 切片 | 实施职责 | 明确禁止 |
|---|---|---|
| F1 portable file backend | .adoc/Resource current bytes、FileBinding、safe install primitive、外部变化观察 | 用P/I中的body覆盖用户文件；hash+rename冒充CAS |
| F2 portable metadata | identity、child order、lifecycle/Trash、Annotation、shared policy/trust、Change/Frontier/conflict记录 | 两份可独立写parent/order；把path/title当Ref |
| P durable control | SQLite decision/recovery/unknown/approval/claim/Money、PreparedIntent、pins、execution responsibility | 保存全库current body作为读回退；被同步器merge |
| I derived index | inventory/parser/search/OCR可删SQLite，分层coverage/checkpoint | identity/policy/receipt owner；partial index冒充全集 |
| D Draft/session | Draft/input/selection/IME、本地或Server协作transient state | 作为author revision或commit evidence |
| C coordinated consumers | D3/D4/D5/D7/D8/D9/D10新版本消费 | 半包激活、generic D6绕过原owner |

Implementation 必须有静态 owner audit：每个 current truth字段恰有一个逻辑写owner。任何兼容双写、启动时silent migration、旧authority.sqlite body mirror、portable metadata↔P双向同步current state均不合格。

## 2. 分阶段实施顺序

### S1 Portable reader + no-write open

- 打开现有文件型Workspace，读取portable metadata入口并验证路径containment/closed records。
- 当前 Document/Resource bytes从普通文件读取；index不存在也能打开指定文件。
- 外部invalid bytes可进入Source/repair inventory，不被replacement字符静默保存。
- 不允许产生新版managed commit，直到S2–S5满足。

### S2 Durable control + legacy replay

- 建立库外 control.sqlite3、WAL/SHM及pin区。
- 完整支持旧 D6 wire1、Policy/1/2、旧Prepared/saved decision原decoder/replay，不把旧记录升级。
- 新v2 schema可读写candidate fixture，但 feature gate保持not_in_release/unsupported_version，禁止产品成功路径。
- P丢失模拟不得从portable/current files合成旧receipt、ApprovalUse、Money或unknown。

### S3 Portable metadata + replica model

- 实现ReplicaRecord、CommitDomain、ChangeId/Frontier、SourceVersion/2、InstallationNotice、ContentCompletionProof、ConflictRecord。
- child order使用单一有序列表owner；move/reorder生成确定变化。
- 新设备register新ReplicaEpoch；retired epoch不可复活。
- 远端change接纳使用本域新ChangeId，不重放remote OperationId。

### S4 Safe file installation

至少实现并证明一种已有文件安装资格：conditional_replace或真正exclusive_write_window；仅advisory锁不能作为测试替身。新文件使用create_only。

故障注入必须覆盖：
- stage write前；
- stage data flush前/后；
- InstallationNotice flush前/后；
- 每个component install前/后；
- file data flush；
- directory entry flush；
- installed verification；
- P seal transaction前/中/后；
- ContentCompletionProof写/flush前/后；
- response传输丢失。

每个点都验证不丢竞争字节、不重复收费、不生成第二identity、不把unknown猜成success。

### S5 Ordinary save 与 semantic pending

- D6 source-save ordinary/complete profile；
- local typed gate与complete obligation分离；
- semantic_pending consumer deny矩阵；
- external observation epoch/ABA；
- r5 replay/current r6 分离；
- installed write set用planned poststate验证，未写dependency继续比before/cut。

D4/D5 consumer afterimage未完成前，semantic_pending只允许candidate-level测试，不打开产品capability。

### S6 Sync/conflict

- 两replica离线source/source、create/create、move/edit、move/move、Trash/edit、policy conflict、partial transport、placeholder、identity collision；
- conflictId稳定、new head supersedes prepared resolution；
- source merge保留原bytes；D3-owned placement/lifecycle resolution在D3 wire12未完成时返回owner_update_required；
- purge必须等待complete inbound + registered replica frontier或retirement。

### S7 Server multi-user

- 多用户并行read/prepare/Draft；
- 不同文档commit可并发计算，仅必要range lock和P seal短暂串行；
- 同文档两Draft不互相覆盖；
- 权限撤销与seal竞争；
- Server failover同时fence P与author file path；
- 第二Server实例不能绕P锁直接rename托管文件。

实时协作仍not_in_release，直到后续OT/CRDT adapter及D1 release evidence；但candidate conformance要有抽象session model测试，确保receive/broadcast/checkpoint三状态不混淆。

### S8 Index/large workspace

- index删除重建、分层coverage、parser/search/OCR独立失效；
- 10k/100k/1M小文件；
- 十几/几十GB以附件为主与正文为主各一组；
- cold/warm filesystem cache；
- SSD/HDD或具名慢速磁盘，记录CPU/RAM/OS/filesystem；
- sync placeholder/按需下载；
- 重启续建、批量外部修改、watcher gap；
- exact/NFC/regex candidate回读与漏召回回退scan；
- no-body-replica检查。

## 3. 必须测量的五个时间

所有大库报告必须分别给出：

- T_first_open：Workspace入口/活动目录可响应；
- T_first_edit：活动Document Draft可输入；
- T_first_reliable_save：活动target完成安全install+P seal；
- T_full_search_ready：指定search profile/范围具备complete coverage；
- T_OCR_ready：指定附件/OCR profile完成或明确失败。

不得只给一个“启动时间”。T_first_edit不等于author save；T_first_reliable_save不等于portable published；T_full_search_ready不等于OCR ready。

还必须记录峰值RSS、index DB大小、P DB大小、protected pins、读取字节、文件数、解析吞吐、P seal延迟、portable publication延迟、重启续建工作量、单文件和批量增量代价。

没有实际数据不得声称“像Obsidian一样快”“几秒重建”或任何具名性能通过。

## 4. no-body-replica 验收

每个候选包实际创建Workspace后，扫描所有：

- durable control SQLite表/索引/FTS shadow；
- derived index表/FTS shadow；
- temp/staging/pin区；
- portable metadata；
- log/audit；
- crash recovery文件。

断言：

1. I不存在完整current Document body列或完整全库AST序列化；
2. P不存在可作为全库current source读取fallback的body镜像；
3. 每个完整source pin都有明确owner/purpose/retention/容量，可追到具体plan/conflict/history；
4. source pin达到可释放条件后可物理回收而不损坏canonical decision/receipt；
5.旧协议要求decision-lifetime pins的历史fixture仍保留；
6. F中的普通文件被外部改变后，P/I旧bytes不能自动覆盖current。

检查只证明实际schema/bytes，不允许根据表名“contentless”推断无敏感内容；token/gram/position同样纳入泄露和容量审计。

## 5. 正反例验收矩阵

下列均为待实现证据，不是本候选已通过测试。

| ID | 场景 | 必须结果 |
|---|---|---|
| FA01 | I全删，10万无关文档未解析，编辑一个D2-valid普通笔记 | 可达T_first_reliable_save，不等待无关index/OCR；全集Action仍不可用直到proof |
| FA02 | existing target只有“hash then replace”，没有conditional/exclusive primitive | commit不得报reliable；保留current+Draft/after，install_unavailable |
| FA03 | conditional replace前第三方写B，planned before=A | 不覆盖B；conflict/paused，A/B/after证据保留 |
| FA04 | 外部A→B→A且watcher gap | observationEpoch增加；旧map/locator/prepared/action evidence失效 |
| FA05 | install after成功，step10仍错误比较before | 测试必须抓出该实现；规范实现按planned after通过written target，只重验unwritten deps |
| FA06 | P seal成功，ContentCompletionProof写失败 | receipt可靠成功；portablePublication=pending；retry只补proof |
| FA07 | response丢失，r5已committed，后来r6编辑 | replay r5返回原receipt；current source仍r6，不重写/不收费 |
| FA08 | 两设备离线同笔记不同编辑 | 两heads进入source_concurrent，不LWW；原字节均可取 |
| FA09 | A move Node，B改其正文 | 可证明维度独立才组合并重验policy/structure；否则conflict |
| FA10 | A Trash，B edit | 不默认delete-wins；保留edit branch与Trash intent |
| FA11 | purge vs旧replica restore | tombstone不复活；old bytes只能fresh-copy/reconciliation |
| FA12 | child_list/Document/ContentCompletionProof分批到达 | incomplete，不mint identity、不推进Frontier |
| FA13 | placeholder size已知但bytes未下载 | not_materialized/source_unavailable，不当empty/not_found |
| FA14 | duplicate Ref但birth不同 | identity_collision；resolver不选first；resolution需D3 owner |
| FA15 | building index漏一个relation target | complete Query/Action拒绝或完整scan；不得以空范围成功 |
| FA16 | semantic_pending source进入auto Action | 明确拒绝complete consumer；Source/明确local读取仍可 |
| FA17 | P丢失、files/portable intact | 可新ReplicaEpoch处理无疑点ordinary content；旧approval/Money/unknown不可重建 |
| FA18 | I丢失、P intact | rebuild I，不重放decision、不新mint Ref、不改变收费 |
| FA19 | copied P数据库在两设备打开 | 不自动得到双execution holder；takeover需fence旧holder/continuity |
| FA20 | approval一次、网络response unknown、程序crash | restart先恢复original request/provider evidence；不新OperationId、不再consume |
| FA21 | Money work batch先charge后crash | charge保留；attempt重启不refund；预算不足保持paused |
| FA22 | Server Alice/Bob编辑不同Document | 两Draft与prepare并行；commit各一次；SQLite writer短串行不变成UI互斥 |
| FA23 | Server Alice/Bob同Document旧base | first seal成功；second保留Draft并stale/conflict，不覆写 |
| FA24 | Bob revocation与checkpoint seal并发 | 唯一线性化；revocation先赢则Bob未提交贡献不能借Alice身份提交 |
| FA25 | Server failover只fence control DB未fencefile writes | conformance fail；旧实例必须不能继续author file rename/write |
| FA26 |协作receive ack但无checkpoint | UI/API不能显示reliable save/committed |
| FA27 | IME preedit多事件+最终input | preedit不入author op；最终至多一个Draft transaction/checkpoint input |
| FA28 | protected conflict pin超容量 | 新操作受限；不能删last-reference conflict evidence继续 |
| FA29 | expired新历史effect pin | effects_unavailable；不得用current file伪造old after |
| FA30 | legacy wire1 saved decision | byte-equal旧decoder/replay，旧pin promise不被v2 retention改写 |

## 6. 权限与非披露测试

至少覆盖：

- caller持有ConflictId但无state disclosure → not_visible，不能知道record存在；
- replica_register但无source_write → 可登记但不能编辑正文；
- conflict_resolve但无source/policy/D3实际write → prepare在对应原权限门失败；
- execution_custody_admin不授Money增额/approval新建/source read；
- phone-only Field edit不能因ordinary source profile获得body/name；
- hidden sibling不因计算ordinal被枚举；
- current source external_invalid的详细parse/bytes仅按repair/source权限交付；
- receipt/effects replay撤权遮蔽但decision不改变。

## 7. Legacy 与版本矩阵

实现测试必须同时保留：

- D3 v9/v10/v11 historical saved request/receipt/error；
- D6 wire1 commit/receipt/error；
- Policy/1/2；
- SourceVersion/1；
- D7 PreparedActionBinding/1,/2；
- D8 PreparedEditBinding/1；
- 已有 Result/ByteHandle历史token。

新 v2 fixture不能通过“把wireVersion数字改2”生成。必须按本候选closed schema独立构造，并验证old→new禁止自动upgrade、new→old禁止降级。unknown owner version返回unsupported_version/owner_update_required，不fallback旧近似路径。

旧canonical request若保存完整source/pin是旧协议历史证据，不迁移；新PreparedIntent/2使用InputDescriptor+purpose PinRef。测试须证明相同hash不同SourceVersion/owner descriptor不会被视为same input。

## 8. D4/D5/D7/D8/D9/D10 后续消费门

在下列后像完成并共同接受前，对应新路径保持 unavailable：

- D3 wire12：CommitDomain ledger、replica-local create/move/Trash、purge frontier、legacy replay；
- D4：semantic_pending local-vs-complete typed obligation矩阵；
- D5：native结构/集合/bulk的local-vs-complete门；
- D7：domain/frontier cut、PreparedActionBinding/3、EffectManifest新retention、Action evidence；
- D8：Source/Live/Read、reliable-save/portable状态、conflict与collaboration session；
- D9：ImportJob/ExportPlan新pins/version；
- D10：sourceOccurrenceKey continuity、recipient/target/payload approval、Money/claim/stop execution responsibility。

不能因D6文件存在就模拟这些consumer“已经支持”。

## 9. 文档/机器检查

候选文档自身至少检查：

- 中英文标题/章节编号与所有closed enum/key集合一致；
- terminology registry conceptId唯一、ownedNames冲突检测、旧firstFreeze逐项不变；
- replacements.json只列实际存在的后像，sourceBlob精确等于固定S；
- 不出现指向不存在owner后像的“已定义/已激活”语言；
- D6 v2 JSON例子可被严格JSON模板/人工schema检查，bool与Counter不混用；
- 新ConflictId、Frontier排序、ChangeId overflow、Policy/3能力矩阵有正反例；
- docs/design/snapshots、inputs、D10十八份在本批未变。

文档CI成功也不等于产品conformance或独立review通过。

## 10. Fresh review gate

完整联合候选必须由新的独立评审从零读取：新proposal、全部replacement owners、D10十八份、固定S49输入。作者当前16/49全文+局部范围只属于作者阅读 provenance，不能继承旧独立49/49作为本候选通过。

旧B13保持REVISE、术语/双语FAIL、P0=0/P1=3/P2=8十一OPEN。本批只提供其基础关联面，不关闭或重分类。
