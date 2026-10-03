---
source_language: zh-CN
translation_status: source
---

[English](REVIEW-ENTRY.md)

# 联合候选评审入口

状态：候选，未接受、未激活、未实施。本文说明 2026-10-03 收齐的设计候选；评审必须记录实际读取的 Git 提交，不能以移动分支代替固定版本。

## 来源与当前正文

固定来源 S 为 `7e18168dad3e6d120fce0dd607dc10fa7894e252`。[原始清单](../inputs.json)的 49 份来源及清单本身保留原字节。[替换清单](replacements.json)逐项记录 45 个来源到 87 份候选文件的映射；它是候选读取路由，不代表接受或替换历史来源的授权。未映射来源仍必须按原清单读取。D10 当前正文位于[任务入口](../d10/TASK.zh-CN.md)列出的九组双语文件。

完整联合审查应读取每个相关来源、完整候选正文、中英文及机器目录；不能仅靠 diff、摘要、作者报告、文件哈希或检查通过宣布接受。D7 全部 25 件以及 D6 与 D10 联合生产端 13 件均已完成作者交付；它们已存在，不再是未来待写接口，但尚未完成整包独立接受。D7 接管后仅统一了中文“调用方”和“契约”用词。

## 已有证据与未决问题

D3 IR01–09、D4 六项、D5 两项、D8 三项修正以及 D10 历史 11 项的具名修订存在限定独立通过证据，结论只适用于实际被审版本及范围。D9 十六件完整独审结论为限定通过；其依赖的联合整合仍开放。历史 R08 的 3 个 P1、8 个 P2 不因局部修复被自动登记为全局关闭。公共历史说明若与本次完成时点不同，应按原版本解释，并在整合时逐项校准。

1. **PL-IR-01 / P1 已有作者修复候选，仍待独立复核后才能关闭。** 候选令 `RevisionTokenBinding/2` 专职稳定生产地址，仅含 `token+RevisionTokenSource/2`；每个真实 sealed managed after 都由 winning plan/seal 选出唯一 canonical binding，并由 original sealed-outbox 关联认证。接收端保持该地址不变，建立自己的 fresh `SourceObservation/1`；新的 Locator 读取只有在完整 `sourceVersion` 与已解析生产版本逐字相等，且原 coordinate/profile、授权、exact Frontier 与最终 read barrier 全通过时才成功。旧 selector、ActionEvidence、PAB、Draft/map、PreparedIntent 均不复活。相同 Counter/hash/text、真实 production ABA、不可证明 gap/rematerialization、loser/aborted token 或 external 仅 bytes 相同都不构成资格。D6 PL01–PL18 只是设计要求、未执行产品测试；本作者候选必须交给独立审查者复核，之后才能裁定是否关闭 P1。
2. **D3/D6/D7 联合审查开放。** 核对原 wire12、唯一准备描述、单决议、安装恢复、PAB3、MinimumMapping3、原生十二数组与强制完整 canonical effects 的关系，以及 D8 PB2 和 D9 Construction2 的真实消费。
3. **D6/D10 联合审查开放。** 核对完整内部控制记录、范围 CAS、两次批准计次、动态外部确认与不可变输入分离、安装后 stop、共享预算/未知发送责任、调度历史连续性、真实旧 bootstrap family 的能力。实际 portable Registry activation 使用 strict 完整安装和原 ChangeId；只改变保护的 catalog selector 才可 control_only。其余 D10 消费者必须逐项与实际生产端一致。
4. **A2 和最终全局接受开放。** 尚需自包含 D1–D10 整合、来源覆盖、术语和双语一致性检查及全新独立 Pro 全局终审。实现、互通、性能及历史模型证据不得由文档结构检查代替。

## 修订与接受

审查报告须给出固定提交、实际覆盖与缺口；每个发现须含定位、可复现的设计场景、后果与建议。作者通过独立分支及 PR 修订；同时仅一位候选作者，修改后将最终 head 交给未参与创作的审查者复核。保留真实旧决议的解码和恢复义务，不推定所有历史原型均已部署。

最终设计冻结要求完整独立阅读、零开放 P0/P1、P2 逐项处置及明确接受。设计接受不代表实现通过，也不授权自动合并、发布或部署。
