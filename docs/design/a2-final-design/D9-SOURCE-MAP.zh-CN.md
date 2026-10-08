---
source_language: zh-CN
translation_status: source
---

[English](D9-SOURCE-MAP.md)

# A2 D9 来源映射

状态：**author-resolved-pending-independent-review**。

机器权威为 D9-SOURCE-MAP.json。

- fixed S：八份 D9 source 均 FULL 阅读，并以连续无缺口的 section/line range 从第 1 行覆盖到各文件末行。
- current D9 owner afterimage：八组中英文件均 FULL。
- Mandatory source §14 的 884–924 行：FULL。
- Mandatory source §15.5–15.8 的 1042–1141 行：对 D9 适用的 export/View 交叉为 FULL；这不批准 D7/D8 deferred layout。
- current D7 D9-binding 与 PreparedActionBinding owner 中英对：FULL。
- final AsciiDoc/Annotation successor：已完整阅读 D9 直接相关的 SPEC/SCHEMAS/ACCEPTANCE 中英文；fresh author submission 继续使用 wire13/PAB4/Effect3。fresh export 使用 final-FC ExportPlan/4，并按交付类型使用 PublicationReceipt/4 或 D9PrintReceipt/1；Plan/Receipt1–3 保留精确的记录型恢复。
- D10：只读取具名 D9 direct intersection，记为 PARTIAL；完整 D10 仍 UNREAD，留后续。

fixed8b6 独立复核已经 CLOSED D9-BAF-P2-02，但 D9-BAF-P1-01/P1-02 仍有更窄的 carrier 约束待修。本批保留既有 Annotation carrier 与六图表正向 route，并在 final-FC §6.6.1 增加关系/canonical admission：跨输入域 selection/projection 必须排他（Annotation Plan 中不得选择普通 Document 正文/参考文献），一个 inputIndex 只能对应一个 Annotation mode、一个 projection 和匹配的 target，portable_backup projection.recordPin 还必须逐字等于该 index 对应 annotation_content payload.recordPin，证据 pin 并集完整也不能替代；View scope 固定 complete_data，须校验精确 ViewSpec 的 projection/binding hash，遵守授权先于 D9 业务诊断的顺序，并通过真实受保护 print Plan 关联校验 receipt/output/loss；新增 array 明确 canonical comparator，同时保留 disclosure fragment 与 D7 result 的语义顺序。现任 D9 renderer union 只有六成员，不能靠安装 profile 扩出 network；network 在当前 route 上固定 renderer_unavailable，必须等待未来 versioned successor。此前 visible native-table Office authoring 修订保持不变：唯一 lowercase-ASCII leaf 继续使用 data.native_table.COLUMN；fresh qualified/non-ASCII 使用可见 native.table[...]::column[...] bytes，nt_/nc_ 仍只是内部 Plan key。

受保护 S49 与 docs/design/inputs.json blob 787d03c31a55496f81ed03fd54a6fdfff50a2ad4 保持 immutable。historical 有界 evidence set 57/63/59/82/90/130/12 分开记账。source coverage、count、hash 与绿色 docs check 都不是语义接受。

范围修正继续固定在已复核的 ff10 前身，并已在 fixed8b6 独立 CLOSED。current successor 映射现在指向本修复实际 blob：SCHEMAS EN §6.5 L1997–2225、§6.5.1 L2226–2262、§6.6 L2263–2634，其中 §6.6.1 为 L2559–2634；ZH §6.5 L2101–2336、§6.5.1 L2337–2375、§6.6 L2376–2747，其中 §6.6.1 为 L2672–2747。SPEC EN §8.3 L439–463、§8.4 L464–471、§16 L946–955、§16a L956–972、§17 L973–980、§18 L981–995；ZH §8.3 L449–483、§8.4 L484–491、§16 L1012–1021、§16a L1022–1038、§17 L1039–1046、§18 L1047–1153。 本次真正的现任准入入口为 FC SCHEMAS §6.6.1 EN L2559–2634 / ZH L2672–2747、FC SPEC §8.3/§16a/§17 的上述现任范围、D9-INTERFACES §4/§7 以及原 D9 workers/export owner §3a；旧版本 predecessor ranges 仅是历史来源。

协调者发现的版本预检已经在原 D9 三修 scope 内协调，不新增 finding 计数：FC §8.3 继续保留 exact Source/Resource/query_json 的 generationPolicy=none、受控名称与 canonical ordering、template/route/style、递归 pin 和 unknown-publication 义务，但 fresh unseen work 统一使用 Plan4/Receipt4 或 PrintReceipt1。SPEC §17 的 unseen-current 分派、§18 类型清单、SCHEMAS §§6.5–6.6、terminology registry、FC acceptance 与 replacement router 已一致。真实记录的 Plan/Receipt1–3 保留精确 decoder/token/bytes/pins/confirmation/saved-planned-unknown recovery，绝不迁移。

fixed6012 非作者已独立 CLOSED D9-BAF-P1-02（别名 D9-466B-P1-01）的完整八域/请求选择分派，以及 D9-466B-P2-01 的获权 selector 错误合同。fixed466b 的 D9-4DA7-MAP-P2-01 和全部旧有界 CLOSED 均保持。本批仅修两项新问题，状态均为 author-resolved-pending-independent-review：D9-6012-P1-01 撤销误加的 graph nodeDetails 强制字段（原 TerminalSchema 与 query_json 只要求完整 nodes/edges 和真实 V/列/bag/顺序；ViewSpec.network.nodeDetails? 是可选绑定，孤立 node 与零 edges 合法）；D9-6012-P2-01 只禁止请求方 View selector 借用 invalid_output，原 Host/Core 独立 IR coverage（S33，XLSX 漏 sheet/隐藏对象）、worker 输出验证、D9 Region 新签发格式/几何验证均保留原职责。八域/七 catalog、§3a 隐藏随机 token/原子保存、历史 Plan1–3 恢复、Annotation/print/View complete_data 与六图表 network 边界不变。现任映射按最终新 Git blob 刷新；ff10 predecessorRangeCorrection、D4/D5 旧 direct-read、S49 与 760 来源行只作历史资格。产品 UNRUN，完整 D10 与 fresh Pro/global A2 留后续。
