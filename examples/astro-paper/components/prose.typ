#let prose(body, id: none) = {
  let attrs = (:)
  if id != none {
    attrs.insert("id", id)
  }
  [
    #metadata("./prose.css") <aster-style>
    #html.elem("article", attrs: attrs)[#body]
  ]
}
