#let _state = sys.inputs.at("_aster", default: none)
#let _collections = if _state == none { (:) } else { _state.collections }
#let settings = toml("/aster.toml")
#let route-param(name, default: none) = if _state == none {
  default
} else {
  _state.route.param(name, default: default)
}
#let route-pages() = if _state == none { () } else { _state.routes.pages() }

#let get-collection(name) = {
  _collections.at(name, default: (:)).values().sorted(key: entry => entry.id)
}

#let get-entry(collection, id) = {
  _collections.at(collection, default: (:)).at(id, default: none)
}

#let posts() = {
  get-collection("posts")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.metadata.date)
    .rev()
}

#let post-url(id) = "/blog/" + id + "/"

#let date-value(value) = {
  let parts = value.split("-").map(int)
  datetime(year: parts.at(0), month: parts.at(1), day: parts.at(2))
}

#let date-label(value) = date-value(value).display("[month repr:short] [day], [year]")
