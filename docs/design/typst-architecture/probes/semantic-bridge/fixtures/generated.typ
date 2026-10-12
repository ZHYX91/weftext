#set heading(numbering: "1.1")
#for i in range(2) { heading([GeneratedHeading #i]) }

#context [#metadata(query(heading).map(it => (
  body: it.body, level: it.level,
  number: counter(heading).at(it.location()),
))) <heading-check>]
