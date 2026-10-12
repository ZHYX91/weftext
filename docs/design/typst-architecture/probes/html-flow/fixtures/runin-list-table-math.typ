#set heading(numbering: "1.1")
#show <runin>: it => context { strong[#counter(heading).display(it.numbering) #it.body]; text(" ") }
= OrdinaryHeading
Plain body.
== RuninHeading <runin>
Adjacent body.

- ItemOne
- ItemTwo

#table(columns: 2, [CellOne], [CellTwo], [CellThree], [CellFour])

Inline $x^2$ and display:
$ sum_(i=1)^n i $
