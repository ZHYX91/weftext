#set text(font: "New Computer Modern", size: 13pt, fill: blue)
#set heading(numbering: "1.")
#show heading: set text(size: 20pt, fill: red)
#show heading: it => [PREFIX #it.body SUFFIX]
= HOne
#context [Counter=#counter(heading).display()]
#show strong: it => it.body
Stripped *StrongBody* and _Emphasized_.\
End.

#block[#show heading: it => []
= GoneHeading
]
#context [AfterGone=#counter(heading).display()]
