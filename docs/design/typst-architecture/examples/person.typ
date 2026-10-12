#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654", types: ("person",),
  names: ((text: "张三", role: "ordinary", preferred: true, language: "zh-CN"),),
  phones: (
    (number: "010-12345678", label: "work", customLabel: "工作"),
    (number: "010-12345678", label: "personal", customLabel: "私人", note: "晚间联系"),
  ),
  emails: ((address: "zhang@example.org", label: "work", customLabel: "工作"),),
  birthday: (month: 5, day: 12),
  engagements: (
    (employer: (ref: "weftext-node://2fd54310-65f7-4d3c-8668-2f1e344a56c0/0efb7d4e-6248-451c-b747-692f1ae9b1cb"), position: "工程师", department: "研发",
     during: (start: datetime(year: 2022, month: 3, day: 1)),
     sources: ((reference: (url: "https://example.org/team"), locator: "研发团队",
                accessed: datetime(year: 2026, month: 10, day: 12)),)),
    (employer: (text: "过去的研究所"), position: "研究员",
     during: (start: datetime(year: 2018, month: 1, day: 1),
              end: datetime(year: 2022, month: 2, day: 28))),
  ),
)) <weftext>
#title[联系记录]
= 讨论
任职记录和多联系方式仍是原生数组和字典。
