#set list(marker: depth => [FuncMarker#depth])
- Body
#table(columns: 2, stroke: (paint: red, thickness: 2pt, dash: "dashed"), [DashedA], [DashedB])
Alpha #h(1em) Beta

#html.frame[
  #context [FrameTarget=#target()]
  #set list(marker: depth => [FuncMarker#depth])
  - FrameList
  #table(columns: 2, stroke: (paint: red, thickness: 2pt, dash: "dashed"), [FrameDashedA], [FrameDashedB])
]
