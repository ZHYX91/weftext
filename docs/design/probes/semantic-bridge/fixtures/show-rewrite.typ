#set heading(numbering: "1.1")
#show heading: it => [PREFIX #it.body SUFFIX]
= OriginalHeading
Ordinary body.

#context [#metadata(query(heading).map(it => (
  body: it.body, level: it.level,
  number: counter(heading).at(it.location()),
))) <heading-check>]
