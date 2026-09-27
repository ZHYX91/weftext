---
_weftext:
  id: "38d4f1f1-958a-4b10-a3e0-7d1627f87f3e"
---

当前权威状态：D8 D8-r03-p2-2026-09-24，仅由总控验收（外部控制记录未随本输入发布）和committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。下面送审正文中的candidate/未激活为历史标签，不覆盖本段。架构接受不代表产品实现；D9未启动。

# D8 术语与命名

revision: D8-r03-p2-2026-09-24；candidate。沿用D1–D7既有concept/Ref/Locator/Field/Action/PreparedIntent/EffectManifest定义，不重命名其领域。新概念仅用于下表明确层级；公共wire名字列出的全集属于Editor Interfaces/Direction合同，不能把review工具、文件名或测试计数混入产品术语。

| concept | canonical English / 中文 | 所有者与命名 | 禁止混称 |
|---|---|---|---|
| weftext.term.edit_session | Edit Session / 编辑会话 | UI状态；EditSession | Workspace、Document identity、提交权威 |
| weftext.term.edit_draft | Edit Draft / 编辑草稿 | 非权威用户提案；Draft / draftSerial | 已保存作者源、sourceRevision |
| weftext.term.draft_projection | Draft Projection / 草稿投影 | Core对本次proposal的可丢弃投影；d8_draft_projection | D2 document_payload、合法D3 locator |
| weftext.term.draft_edit_map | Draft Edit Map / 草稿编辑映射 | Core输出的flow/path/segment/plainRegion/site；仅绑定一次完整proposal | D3 Locator、持久段落身份、客户端parser |
| weftext.term.prepared_edit_binding | Prepared Edit Binding / 编辑准备绑定 | Core immutable D8 record；d8_prepared_edit_binding | D7 PreparedActionBinding、ActionSpec、第三ledger |
| weftext.term.composition_transaction | Composition Transaction / 组合输入事务 | UI begin/update/commit/cancel group | D6 author transaction或操作回执 |
| weftext.term.caret_affinity | Caret Affinity / 光标亲和位置 | 同logical点的visual side，upstream/downstream | source偏移、永久定位器 |
| weftext.term.layout_epoch | Layout Epoch / 布局代 | 同次shape/折行/hit-test，客户端可丢弃 | Result epoch、auth generation、author revision |
| weftext.term.direction_preference | Direction Preference / 方向偏好 | device/session presentation，ltr/rtl/auto | locale、portable作者方向、Query排序 |

D8新请求/响应kind闭集：`d8_document_read`、`d8_document`、`d8_draft_project`、`d8_draft_projection`、`d8_draft_text_replace`、`d8_draft_text_replaced`、`d8_draft_write`、`d8_draft_written`、`d8_edit_prepare`、`d8_edit_prepared`、`d8_undo_prepare`、`d8_editor_error`。内部binding kind独立为`d8_prepared_edit_binding`。`document|annotation`只是D8 intent的局部discriminator，不能增加D3实体kind。成员与closed error集合以Editor Interfaces为唯一源。

Source / Write / Read是模式名称；“只读”是是否允许当前交互的状态，不是第四份文档。“草稿已保存于此设备”“待提交”“结果待确认”“已提交”分别使用，不把autosave Draft写作Workspace已保存。`planned`仅表示Core已存原计划，`preview`不能称receipt。选区是selection，作用于源的连续范围为source range；视觉矩形不自动成为原范围。

“RTL支持”不得作为单一空泛标签：分别列Unicode内容保存、bidi交互、RTL shell、RTL语言UI翻译。`dir=auto`只是一种具体呈现机制，不是完整能力名称。阿拉伯引文locale不等于阿拉伯UI本地化。

新旧受控名称核验范围：本候选的上述九个概念及十三个kind声明与实际使用，D7运输修订对两种Prepared Binding的名称使用，D3/D6版本号消费者。无需扫描用户正文/代码样例普通词并宣称自然语言一词一义已机械证明。独立评审需明确术语是否通过。

BodyPath、flow、segment、regionIndex及siteIndex均为Draft Edit Map局部坐标；命令splice_plain/break_plain/insert_at_site是草稿变换，不是D7 ActionSpec或D6 author transaction。inputSerial是原请求回传，projection.draftSerial是下一个proposal序号，不能混称author revision。

origins是Draft Projection内独立的只读来源成员，elements/flows及relation不授编辑权，不是新的领域身份。plainRegion是Draft Edit Map的局部raw文本区域，不是D2 paragraph；空白区域不制造D2空实体。上游D2 Inline闭集严格为text/link/resource_occurrence/node_link/citation；kind:"resource"不是合法Inline。D3 Annotation target的resource分支仍是其原协议的合法名称，不可全局替换。
