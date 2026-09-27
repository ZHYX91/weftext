---
_weftext:
  id: "5b1582b7-f6e9-4b79-9463-001ed4ee05c9"
---

# D2 实现影响与测试轮廓

状态：final-capable D2 v2 实现影响 companion；只有与 final D2/D3/Lexicon/D3 Impact/control indexes 同世代原子切换后才成为当前实施输入。本文不证明 `repos/weftext` 已实现，也不授权代码、公开规范或D4；D4保持paused。

日期：2026-08-31。

权威决议：[D2 文档内容与领域对象](../d2-document-content-and-domain-objects/source.md)。

## 1. 实现影响图

```text
Workspace aggregate
  -> contentRootNode
      -> ordered Node tree (parent + siblingOrdinal)
          -> exactly one Document exact source
              -> closed Weftext AsciiDoc Profile v2 parser
              -> A+ reserved header control + namespace-scoped lexical carriers
              -> valid: metadata + body occurrence tree
              -> invalid: exact source + one primary diagnostic, no projection
          -> owner-local Resource logical set
          -> owner-local Annotation logical set

Task
  -> ordinary Node + exact built-in tasks/task Facet; fresh NodeRef promotion; no mirror

Template
  -> Core meta-kind; Task Template uses D9 target plan for fresh ordinary + tasks/task

Saved Query/View
  -> Document occurrence only, no ViewRef or second owner

NodeCollectionResult
  -> ordered evaluation result, no durable identity or membership ownership

D5 record domain (if chosen later)
  -> separate Workspace entrypoint/owner/identity/wire
  -> never masquerades as Document/Node or a second document authority
```

## 2. 组件影响

| 区域 | 冻结影响 | 必须删除或禁止的第二权威 |
| --- | --- | --- |
| Core domain | Node恰一Document；Resource/Annotation owner-local；Task=ordinary Node+exact `tasks/task` Facet；Template=Core meta-kind；carrier/entry及其他occurrence不是entity | folder/data Node、durable Block/field occurrence、TaskRef/mirror、workspace-owned SavedView |
| parser/profile | Profile v2 exact source唯一；A+ reserved header control、namespace-scoped carrier outer framing、raw entry/ranges、body grammar；invalid全投影unavailable | YAML/sidecar/whole-namespace blob authority、开放AsciiDoc fallback、partial/旧projection |
| wire/decoder | closed-world v2；duplicate key/unknown field/missing/illegal null 拒绝；document payload、Resource、Annotation、collection 形态固定 | 宽松 decoder、last-wins、按表面默认、locator/EntityRef coercion |
| Node tree | parent 与连续 siblingOrdinal 是 control state；heading 独立 | 从 heading、集合 membership 或路径反推 parent/order |
| Resource | bytes 是 authority；presentation 只在 occurrence；durable byte handle slot 固定 | Resource 全局 caption/alt、cross-owner direct ref、raw path/URL 当 ResourceRef |
| Annotation | purpose/reply/suggestion/target 是闭合状态；body plain text；stale/revision mismatch fail closed | Annotation inline parser、UI 推断 suggestion、跨 owner reply/target |
| Query/View | saved definition 只在 Document occurrence；NodeCollectionResult 无 identity | ViewRef、workspace control owner、持久 member snapshot |
| D5 integration | D2 API 无 Record branch；D5 仍可从零定义独立 record domain | RecordRef 送入 Node/Document decoder、Document row 与 Record 双身份/双权威 |
| D6/D4/D7 gate | final commit 是 D2 gate 与适用下游 gate 的 AND；diagnostic namespace 分离 | 下游把 D2 invalid 变 eligible、partial D2 projection 或另写作者源 |

## 3. 退役闭环

未来实现必须为每项同时给出旧 API/type/fixture 清单、replacement test、negative fixture 和全文搜索 deletion assertion：

1. mixed YAML/header/sidecar metadata authority；
2. hidden Template inventory 与 ancestor role propagation；
3. task/checklist mirror identity；
4. durable Block/element entity；
5. open AsciiDoc parser、body attribute substitution、include/pass/extension；
6. Resource 全局 presentation 与 cross-owner direct ref；
7. AsciiDoc/active Annotation body；
8. persistent NodeCollection/SavedView；
9. Record 分支冒充 D2 content；只删除 D2 masquerading，不删除 D5 研究和独立模型入口；
10. citation 与 Resource 共享错误/owner 规则；
11. invalid source 的 permissive/partial/old-cache projection；
12. 无 siblingOrdinal 的 unordered Node snapshot。

## 4. 测试轮廓

1. **领域性质**：Node/Document 1:1、有序树、owner-local Resource/Annotation、Task ordinary+`tasks/task`、Template meta-kind、carrier/entry及其他occurrence无entity identity。
2. **Profile v2 fixtures**：A+ title/control/facets、carrier opener/namespace/raw entries/ranges/wrong closer、body comment/heading/list/table/protected block/anchor/escape precedence；unsupported构造全部fail closed且no fetch。
3. **Projection/diagnostic**：valid 只有 available；invalid 恰一固定 primary diagnostic、exact source visible、projection unavailable、D2 gate reject，五表面相同。
4. **闭世界 wire**：unknown version/kind/field、duplicate JSON key、missing required、illegal null、duplicate set identity 拒绝；standalone/embedded document 使用同一完整 document payload。
5. **Resource**：source defaults、explicit empty string、presentation per occurrence、byte envelope、cross-owner/raw path/URL 拒绝。
6. **Annotation**：purpose round-trip、reply same-owner/acyclic、suggestion range + expected revision、plain-text inert、stale/revision mismatch fail closed。
7. **Task/checklist/Template**：toggle只改source；promotion原子创建fresh ordinary Node、声明exact `tasks/task`、确定parent、普通Node link、无mirror；VTODO mapping同样fresh；Task Template target plan产生fresh refs且Template自身不成为Task。
8. **Citation/Bibliography**：same-Workspace Node intent、placement 属于 body、render numbering 不回写、Resource owner rule 不误用。
9. **Collection/View**：result 无 identity；saved definition 只有 NodeRef+locator；ViewRef/workspace owner/member snapshot 拒绝。
10. **D5 边界**：Document row/cell 无 RecordRef；独立 record domain 不进入 D2 content tree、不复用 NodeRef；互转必须显式 Action。
11. **Gate 合成**：D2 与 D4/D6/D7 gate 做 AND，错误 namespace 分离，任何组合都不能返回 partial/旧 D2 projection。
12. **退役门**：逐项删除旧类型/API/wire/spec/fixture，仓库搜索不存在兼容入口；公开规范只在真实实现和验收同步完成时更新。

## 5. D2 v2 原子替换增量

- authority binding：final D2 target SHA-256 `873D2487AC27C7C579D64C14CAB106215FBD462286416860225F2F404BC85137`，稳定ID不变；必须与D3 wireVersion 9 generation原子切换；所有可独立传输的D2 Annotation outer固定`wireVersion:2`。
- exact-source/artifact binding：完整source从scalar 0到EOF统一hash，A+ control、carrier、body、trivia与line endings不得拆成第二authority；copy/fork/import/export随完整bytes/hash，carrier/entry不是symbolic ref slot。
- occurrence negative：AttributeCarrierBlock/LexicalAttributeEntry只有owning Document/current revision ranges；无EntityRef、lifecycle、cross-revision continuity、locator或Annotation target。
- field target negative：本切换不新增field occurrence identity或Annotation target；未来durable field target必须正式reopen D2/D3。
- 退役：`wf-specialization`/`NodeSpecialization::Task`、TaskRef/mirror、wire v1、Profile v1、YAML/sidecar/whole-namespace blob均不得作为current decoder或authority路径保留。

## 6. 验收边界

D2 冻结只证明目标合同足够稳定，不证明当前 `repos/weftext` 已符合。未来实施必须同时更新 Core、全部调用表面、fixture、测试和中英文公开规范，并把真实证据写入 `04 Acceptance`。D3 amendment必须与本文件同代激活；D5仍保有独立Record/RecordCollection的从零选择权。D4保持paused/not started，未经用户新的明确授权不得创建或启动。
