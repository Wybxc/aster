#let _state = sys.inputs.at("_aster", default: none)
#let _collections = if _state == none { (:) } else { _state.collections }
#let _route = if _state == none { none } else { _state.at("route", default: none) }

#let settings = toml("/aster.toml")
#let route-params = if _route == none { (:) } else { _route.params }

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
