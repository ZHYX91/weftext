---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Core修订探针

这些有限实验只使用Python标准库，用于复核具体候选算法，不是产品后端或验收套件。

保持以下文件同目录：`namespace_probe.py`、其完整本地依赖`win32_probe.py`、独立模型`logical_cut_model.py`、完整输出`namespace-stdout.json`与`logical-cut-stdout.json`、记录脚本/输出SHA256的`MANIFEST.json`。没有其他私有依赖。

在此目录、Windows与Python 3.9以上运行：

```powershell
python namespace_probe.py > namespace-stdout.json
python logical_cut_model.py > logical-cut-stdout.json
```

namespace探针只建立脚本旁的新`namespace-run-*`实验目录并保留它，不递归清理；删除测试仅选择实验自身的known文件及实际空目录，插入unknown文件必须保全。输出仅写相对实验目录名。LogicalCut模型不访问文件系统，也可在其它Python平台运行。

记录的`2026-10-12T13:44:57.094972+00:00`本机Windows/NTFS样本：目录LIST_DIRECTORY/READ_ATTRIBUTES handle、share READ|WRITE而不shareDELETE，使外部祖先/目标父改名失败32，本操作持有source同handle的move成功；新sibling仍可插入。早先仅ATTR的handle曾不能保护祖先placement，不能把它当等价guard。

新增`NtCreateFile`案例以已持parent handle与单相对name、FILE_CREATE和FILE_DIRECTORY_FILE直接返回原LIST/ATTR/DELETE创建handle及FILE_CREATED；第二次同名创建失败`0xc0000035`，原创建handle持续持有、外部copy-root改名失败32。不从mkdir后重开推导创建归属。实验fixture搭建仍使用普通mkdir，它不是被测试的操作算法。

计划文件CreateNEW RW share0使外部write失败32；unknown目标子项可新增并保留。selected file disposition成功；有unknown子项时目录disposition失败145且unknown保全；实际空的新目录可删除。这支持固定copy成员与delete-if-empty，不支持排他subtree快照或全目录CAS。本次Win32绝对路径同handle rename成功，未执行native相对rename备用分支。

模型复现同事务前A10、后B10混读sum20；共同保留cut得到sum10，live排序变化不改变冻结分页，模型授权变化阻止继续交付。

未证明生产祖先链/alias/reparse验证、所有handle/mapping/filter、嵌套闭包容量/access升级、Node多步rename恢复、中断/断电/目录项耐久、任意外部整库原子观察、Server安全/fencing、产品retention/publication或性能。通过探针不宣布独立审查finding关闭。

该包从此前有限实验抽取完整最小依赖，补目录创建handle并重跑；不修改此前脚本/原始结果。[NtCreateFile官方说明](https://learn.microsoft.com/zh-cn/windows/win32/api/winternl/nf-winternl-ntcreatefile)、[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)、[FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info)、[SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)、[FILE_DISPOSITION_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_disposition_info) 为这些原语的官方说明。
