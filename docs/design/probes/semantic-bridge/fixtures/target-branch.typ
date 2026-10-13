#set heading(numbering: "1.1")
#context if target() == "html" { [HTMLBranch] } else { [PAGEDBranch] }

#context [#metadata(query(heading).map(it => (
  body: it.body, level: it.level,
  number: counter(heading).at(it.location()),
))) <heading-check>]
