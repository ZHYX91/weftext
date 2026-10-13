#set heading(numbering: "1.1")
= OriginalHeading
Ordinary *StrongText*.

#context [#metadata(query(heading).map(it => (
  body: it.body, level: it.level,
  number: counter(heading).at(it.location()),
))) <heading-check>]
