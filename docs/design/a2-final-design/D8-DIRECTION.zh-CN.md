---
source_language: zh-CN
translation_status: source
---

[English](D8-DIRECTION.md)
# A2 D8 Direction, Bidi, Layout and Accessibility

状态：author-resolved-pending-independent。本文承接 fixed-S Direction/Accessibility、RTL mandatory intake、D6-FA afterimage 与 current D2/D7/D8 owner。它不声称任何平台 renderer、IME、字体、AT 或本地化已经实现。

## 1. 四种能力必须分开

1. Unicode 内容保全：source bytes、logical text、Ref/Field/CEL 等标识不得因 locale/direction 改写。
2. bidi text editing：
caret、selection、hit-test、copy/delete 遵守 logical text + UAX #9。
3. RTL shell：pane/tree/menu/grid/icon/toolbar 的视觉几何与 focus path。
4. RTL localization：Arabic/Hebrew UI strings、plural、punctuation、control labels、AT names。

实现其中一项不能推导另外三项。保存 Arabic/Hebrew bytes 正确不等于 RTL shell 或翻译 PASS。

## 2. Unicode and normative data

编辑/selection/grapheme 使用 Unicode 18.0.0、UAX #9 rev52 与 UAX #29 rev49。D7 CEL 的 Unicode15.1 仅属于 weftext.cel/1 NFC 匹配 profile；D8 不借它改变 editor grapheme 或 bidi 行为。改变 Unicode/UAX/data 版本需要具名 successor、数据版本、fixture 和独立复核。

CRLF 是一个 logical EOL unit；
NEL/LS/PS 不自动成为 D2 newline。
UTF-8 byte、UTF-16 code unit、Unicode scalar、extended
grapheme 与 visual glyph offset 都不可互换。

## 3. Direction precedence

方向来源互不替代：

- D2 authored/native direction：只按真实 source/owner contract；
- D7 ViewSpec.options.textDirection：只控制 View data labels；
- D8 block/flow/session/device preference：只控制 interaction presentation；
- App locale/language：只控制 shell/localization。

在一个可应用 scope 内，显式 authored/native direction 优先；随后是对应 View/block/session explicit preference；auto 使用该 scope logical text 的 first-strong 规则；all-neutral 使用明确 inherited/fallback direction。数字本身不能选定方向。设备 preference 不写 portable content、hidden sidecar、Query 或 shared presentation policy。

run-in/separate 是 Workspace presentation policy，与 bidi direction 独立；不能把 run_in 当 RTL flag。

## 4. Caret model

```text
LogicalCaret = {
 logicalPosition,
 affinity:"upstream"|"downstream",
 layoutEpoch
}
```

一个 logical point 可以有两个真实 visual stop。affinity 只选择 visual side，不是 source offset/identity。font load、soft wrap、zoom 或 container width 改变会产生新 LayoutEpoch；旧 visual coordinate 不能跨 epoch 直接复用。

若旧 logical point 在新 layout 仍有效，按同 logical point + affinity 重建；若 affinity stop 消失，使用定义的最近同 logical stop clamp；若 source/Draft generation 已变，则必须从新 Draft map + current selection transaction 重建。DOM node/index 不是 fallback identity。

## 5. Movement and selection

- Left/Right：视觉相邻 caret stop；
- Ctrl/Meta+Left/Right：逻辑 word boundary；
- Up/Down：视觉 line navigation，保留 preferred inline coordinate 仅在同 layoutEpoch 有效；
- Home/End：当前视觉行起止；
- Ctrl/Meta+Home/End：logical document/flow edge；
- selection 保存 logical anchor/focus/affinity，reverse selection 不倒序 replacement。

Backspace/Delete 删除 logical 前/后一个 Unicode18 extended grapheme；CRLF、combining sequence、emoji ZWJ、isolate 控制序列不得拆分。删除行为作用 logical Source/Draft，不以屏幕左/右字符猜测。

## 6. Hit test and async layout

pointer/touch hit-test 返回 logical point + affinity + layoutEpoch。相同 visual x/y 在不同 epoch 不可直接重放。字体异步加载、bidi paragraph resolution、line wrap 或 high-contrast font substitution 后重新 hit-test；旧 point 若无法证明对应就不 dispatch edit。

虚拟化 item 的 focus 由 logical entity/row/condition key 持有，不以 DOM index。focused/composing item 必须 pin 到 virtualization window；无法 pin 时先明确 cancel/reset interaction，不能静默回收 DOM 并把输入落到别行。

## 7. Logical copy and technical tokens

Copy Text 始终输出 logical text order；
RTL visual display 不反转字符。
Ref、CEL、path、email、URL、citation key、ASCII/numeric
technical token 使用 direction isolation 让视觉可读，
但 source/value bytes 不改写。
Copy Source Fragment 输出 raw exact source order。

Visual selection 跨 bidi run 时，执行范围仍由 logical anchor/focus 规范化；不得按 DOM visual fragment 顺序拼接 replacement。

## 8. Shell mirroring

RTL shell 可镜像 navigation tree、pane affordance、menu/submenu、breadcrumb
geometry、drawer/sidebar、logical previous/next 等显式 directional
controls。
不得机械镜像：
- brand/logo；
- image/media content；
- math/STEM；
- chart numeric/time axes；
- graph relation direction/arrows whose direction is data；
- code/CEL/path/Ref；
- calendar chronology meaning；
- native table logical column/Field identity。

native table Tab/Shift+Tab、arrow navigation
与 structured edit 始终绑定 logical row/column；
RTL 只改变视觉位置，
不交换 FieldId/columnId 或 relationship direction。

## 9. View interaction

D7 Query/View order是语义 authority。
RTL 不能反转 category/series/legend/panel semantic order、Query
sort、network edge direction 或 calendar time。
renderer 可在屏幕几何上镜像，
但 accessible table、export、keyboard order 仍使用原 typed data/order。

chart hover/selection/focus 使用 D7 projected key/column identity；
禁止按 rendered bar index 重找 source。
隐藏 series 是可逆 device presentation state并标记 partial display，
不修改 Query/ViewSpec 或 full export claim。

## 10. Assistive technology

每个 interactive control 提供真实 role/name/state/value/checked/expanded/selected/busy/invalid。
focus order follows logical task/Query order，
不按 CSS visual reorder。
screen reader：
- Document/Source 按 logical content 朗读；
- heading level 使用 current effective heading level，不能把 H6–H9 降成 H5；
- titleless 不朗读 filename 作为 title；
- table 使用真正 row/column/header/span relation；
- Annotation rich body 提供 current semantic text；invalid R6 显示 diagnostics，不伪造 plain fallback；
- chart 同时提供同 data/order 的 accessible table；
- partial/stale/loading/complete zero/permission failure 状态明确区分；
- hidden unauthorized count/value/target 不进 accessible name/description。

IME preedit 不逐字符 live announce；composition start/update/end 有 bounded状态通知。error span 以 logical offset 与可聚焦 condition/control 报告，RTL 不改变 source position。

## 11. Keyboard, zoom and responsive

必须覆盖 keyboard-only、200% 与 400% zoom、320 CSS px、high contrast、reduced motion。focus indicator 在双向文本和 high contrast 下可见。无 hover 也能获得 tooltip 等价信息。二维图形若无法在窄屏完整呈现，可切换 D7 允许的同一完整数据 table/list alternative并明确 renderer unavailable；不能 sampling/truncate 当 support。

Mobile touch target、sheet、virtual keyboard 与 hardware keyboard 都调用相同 logical intent。
screen resize/orientation change 不改变 Draft source、selection
identity、Query semantics 或 author permission。

## 12. RTL acceptance matrix

对 Arabic-only、Hebrew-only、mixed CJK/RTL 分别覆盖：
- editor typing/save/reload/offline/conflict/recovery/direction；
- annotation edit/save/reload/offline/conflict/recovery/direction；
- list/table/board/calendar/timeline/search/chart；
- source exact copy、logical copy、paste/cut；
- IME/composition；
- bidi dual affinity、wrap、font relayout；
- screen reader/keyboard/zoom/high contrast/reduced motion。

这继承 fixed 98 RTL cases，不把两种语言或 surface 合并成“RTL smoke test”。

## 13. Nonclaims

没有真实 Desktop/WebUI/Mobile/browser/AT/IME/font-shaping test 时，状态必须是 UNRUN。设计 fixture/Unicode table/浏览器兼容列表不能证明操作系统剪贴板、VoiceOver/NVDA/TalkBack、移动键盘或真实 renderer。
