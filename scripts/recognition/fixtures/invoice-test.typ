// Self-authored public synthetic fixture. All names / IDs / amounts are test data.
#set page(width: 900pt, height: 580pt, margin: 0pt)
#set text(font: "Noto Sans CJK SC", size: 14pt, lang: "zh", region: "CN")
#let at(x, y, body) = place(top + left, dx: x, dy: y, body)
#at(235pt, 22pt, text(size: 27pt, weight: "bold")[电子发票（普通发票）])
#at(590pt, 68pt)[发票号码：00123456789012345678]
#at(590pt, 94pt)[开票日期：2026年10月18日]
#at(25pt, 130pt, rect(width: 850pt, height: 136pt, stroke: 0.7pt))
#at(450pt, 130pt, line(length: 136pt, angle: 90deg, stroke: 0.7pt))
#at(38pt, 140pt, text(weight: "bold")[购买方信息])
#at(470pt, 140pt, text(weight: "bold")[销售方信息])
#at(38pt, 174pt)[名称：TEST BUYER LTD]
#at(470pt, 174pt)[名称：TEST SELLER LTD]
#at(38pt, 207pt, text(size: 12pt)[统一社会信用代码/纳税人识别号：001234567890123456])
#at(470pt, 207pt, text(size: 12pt)[统一社会信用代码/纳税人识别号：991234567890123456])
#at(25pt, 266pt, rect(width: 850pt, height: 140pt, stroke: 0.7pt))
#at(40pt, 280pt)[项目名称]
#at(600pt, 280pt)[金额]
#at(800pt, 280pt)[税额]
#at(40pt, 324pt)[TEST 测试服务]
#at(600pt, 324pt)[1,000.00]
#at(800pt, 324pt)[130.00]
#at(40pt, 375pt)[合计]
#at(600pt, 375pt)[￥1,000.00]
#at(800pt, 375pt)[￥130.00]
#at(40pt, 428pt)[币种：人民币]
#at(480pt, 428pt)[价税合计（小写）：￥1,130.00]
#at(50pt, 510pt, text(size: 22pt, weight: "bold", fill: rgb("b00000"))[TEST ONLY — 合成测试，非有效发票])
