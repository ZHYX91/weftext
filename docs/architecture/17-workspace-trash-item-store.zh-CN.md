---
source_language: zh-CN
translation_status: source
---

[English](17-workspace-trash-item-store.md)

# 工作区回收站项目存储架构

回收站是由 Core 拥有的隐藏特殊节点，具有封闭的项目存储：

```text
.weftext-trash/
├─ .weftext-trash.adoc
└─ _weftext.items/
   └─ <trashItemId>/
      ├─ _weftext.trash-item.json
      └─ payload/<原始条目>
```

该存储既不是受管节点、非受管内容、被忽略内容、资源目录，也不是派生状态。发现、搜索、链接、Chrono、任务和 Query 都不进入它；同步、备份、修订、验证和恢复包含其中的每一个字节。

## 封闭 manifest 和身份

每个项目都有新的小写 UUIDv4 `trashItemId`；一次删除操作有一个 `operationId`。永久节点 UUID 保留在节点载荷内，在恢复前保持不活跃。`_weftext.trash-item.json` 的 schema 是 `weftext.trash-item/v1`，包含唯一的封闭键、精确的种类特有来源字段和摘要/长度证据。未知来源必须显式，绝不从基名或路径推断。

节点载荷摘要绑定条目名称、目录/文件种类、长度、空目录和文件字节。资源载荷恰好包含一个普通节点所属文件。活动/已删除重复节点 UUID、不完整载荷、格式错误 manifest、碰撞或篡改均为协调证据，并安全拒绝；Core 绝不重新设定身份、合并、覆盖或猜测。

## 计划、恢复和永久删除

计划绑定工作区修订、精确源/目标清单、项目 ID、manifest 字节、载荷摘要和日志步骤。提交不覆盖、持久化、日志化且可恢复：崩溃恢复到完整提交前状态或完整项目状态，绝不会报告部分成功。

恢复计划选择精确项目 ID。`original` 需要记录的活动父项/拥有者和空闲名称；`with-ancestors` 以原子方式恢复唯一完整项目链；`existing-target` 需要明确选择活动目标，冲突时还需要显式新名称。未知来源始终使用 `existing-target`。永久删除是单独的高权限、确认绑定计划，回收站绝不是备份。

## 同步和迁移

部分到达、冲突副本、重复 ID、格式错误字节和同时恢复/删除都是类型化调和情形。完整备份包含整个存储。旧布局进入一次性迁移清单，在证据无法证明来源时创建带显式未知来源的封闭项目；不存在双重运行时权威。
