#let _content-state = sys.inputs.at("_aster", default: none)
#let _collections = if _content-state == none {
  // Tinymist evaluates source files without Aster's runtime protocol.
  (:)
} else {
  assert(
    _content-state.protocol == 7,
    message: "incompatible runtime protocol with the Aster binary",
  )
  _content-state.collections
}
#let _route = if _content-state == none {
  none
} else {
  _content-state.at("route", default: none)
}

#let aster-version = if _content-state == none { none } else { _content-state.version }
#let route-path = if _route == none { "/" } else { _route.path }
#let route-params = if _route == none { (:) } else { _route.params }
#let settings = toml("/aster.toml")

#let get-collection(name) = {
  _collections.at(name, default: (:)).values().sorted(key: entry => entry.id)
}

#let get-collection-ids(name) = {
  _collections.at(name, default: (:)).keys().sorted()
}

#let get-entry(collection, id) = {
  _collections.at(collection, default: (:)).at(id, default: none)
}

#let docs() = {
  get-collection("docs")
    .map(entry => (entry: entry, id: entry.id, metadata: entry.metadata()))
    .filter(item => not item.metadata.at("draft", default: false))
    .sorted(key: item => item.metadata.section_order * 1000 + item.metadata.order)
}

#let docs-by-section() = {
  let sections = (:)
  for item in docs() {
    let name = item.metadata.section
    sections.insert(name, sections.at(name, default: ()) + (item,))
  }
  sections.pairs().map(pair => (label: pair.first(), docs: pair.last()))
}

#let doc-url(id) = if id == "index" { "/" } else { "/" + id + "/" }

#let adjacent-docs(id) = {
  let entries = docs()
  let index = entries.position(item => item.id == id)
  if index == none {
    (previous: none, next: none)
  } else {
    (
      previous: if index > 0 { entries.at(index - 1) } else { none },
      next: if index + 1 < entries.len() { entries.at(index + 1) } else { none },
    )
  }
}
