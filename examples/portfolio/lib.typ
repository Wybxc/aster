#let _state = sys.inputs.at("_aster", default: none)
#let _collections = if _state == none { (:) } else { _state.collections }
#let settings = toml("/aster.toml")
#let route-param(name, default: none) = if _state == none {
  default
} else {
  _state.route.param(name, default: default)
}

#let get-collection(name) = {
  _collections.at(name, default: (:)).values().sorted(key: entry => entry.id)
}

#let get-entry(collection, id) = {
  _collections.at(collection, default: (:)).at(id, default: none)
}

#let projects() = {
  get-collection("work")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.metadata.date)
    .rev()
}

#let project-url(id) = "/work/" + id + "/"
