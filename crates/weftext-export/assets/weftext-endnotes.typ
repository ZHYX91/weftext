// Weftext document-end notes, native Typst module, interface version 1.
// This file is bundled with author source. It requires no package downloads.
// Import: #import "weftext-endnotes.typ": endnote, endnote-ref, endnotes
// Define: #endnote(id: "unique-key")[Author note body]
// Repeat: #endnote-ref("unique-key")
// List once, at the document end: #endnotes()
// Native footnote remains independent; these functions never change its rules.
#let weftext-endnotes-version = "1"
#let _definitions = <weftext-endnote-definition>
#let _references = <weftext-endnote-reference>
#let _bodies = <weftext-endnote-body>
#let _lists = <weftext-endnotes-list>
#let _count = counter("weftext-endnote-v1")

#let _valid-id(id) = {
  assert(type(id) == str, message: "endnote id must be a string")
  assert(id.len() > 0, message: "endnote id must not be empty")
}

#let _body-label(id) = label("weftext-endnote-body-" + id)

#let _definition(id) = {
  let found = query(_definitions).filter(note => note.value.id == id)
  assert(found.len() == 1, message: "endnote id must have exactly one definition: " + id)
  found.first()
}

#let _marker(id) = context {
  let note = _definition(id)
  let n = _count.at(note.location()).first()
  let marker = super(link(_body-label(id), str(n)))
  [#metadata((weftext: "endnote-v1", kind: "reference", id: id)) <weftext-endnote-reference>]
  if target() == "html" {
    html.elem("span", attrs: (
      "data-weftext-endnote-kind": "reference",
      "data-weftext-endnote-id": id,
    ), marker)
  } else {
    marker
  }
}

#let endnote(id: none, body) = {
  _valid-id(id)
  [#_count.step()#metadata((weftext: "endnote-v1", kind: "definition", id: id, body: body)) <weftext-endnote-definition>#_marker(id)]
}

#let endnote-ref(id) = {
  _valid-id(id)
  _marker(id)
}

#let _entry(note) = context {
  let id = note.value.id
  let n = _count.at(note.location()).first()
  let references = query(_references).filter(mark => mark.value.id == id)
  let body = if target() == "html" {
    html.elem("div", attrs: (
      "data-weftext-endnote-kind": "body",
      "data-weftext-endnote-id": id,
    ), note.value.body)
  } else {
    note.value.body
  }
  [#block[
    #metadata((weftext: "endnote-v1", kind: "body", id: id, body: note.value.body, number: n)) <weftext-endnote-body>
    #link(references.first().location(), [#n.])
    #body
  ]#_body-label(id)]
}

#let endnotes(title: [尾注]) = [
  #metadata((weftext: "endnote-v1", kind: "list")) <weftext-endnotes-list>
  #context {
  let notes = query(_definitions)
  let ids = notes.map(note => note.value.id)
  assert(ids.dedup().len() == ids.len(), message: "endnote ids must be unique")
  assert(query(_lists).len() == 1, message: "endnotes must be placed exactly once")
  let contents = [
    #if title != none { block(strong(title)) }
    #for note in notes { _entry(note) }
  ]
  if target() == "html" {
    html.elem("section", attrs: ("data-weftext-endnote-kind": "list"), contents)
  } else {
    contents
  }
  }
]
