// Ordinary native module. These bytes are the library's editable style authority.
#let library-style(body) = {
  set text(font: ("New Computer Modern", "Noto Serif CJK SC"), size: 11pt, lang: "zh", region: "CN")
  show raw: set text(font: ("Noto Sans Mono CJK SC", "DejaVu Sans Mono"))
  show math.equation: set text(font: ("New Computer Modern Math", "Noto Serif CJK SC"))
  set par(leading: 0.65em)
  set table(stroke: 0.5pt)
  set list(marker: [•])
  body
}
