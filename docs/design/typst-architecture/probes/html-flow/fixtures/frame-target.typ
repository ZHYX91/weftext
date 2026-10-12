#set heading(numbering: "1.")
= OuterHeading
#context [Outer=#target()]
#html.frame[
  #context [Inner=#target()]
  #link("https://example.org/frame")[FrameLink]
]
#context [After=#counter(heading).display()]
