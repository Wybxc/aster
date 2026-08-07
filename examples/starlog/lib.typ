#let _state = sys.inputs.at("_aster", default: none)
#let _collections = if _state == none { (:) } else { _state.collections }
#let _route = if _state == none { none } else { _state.at("route", default: none) }

#let settings = toml("/aster.toml")
#let route-params = if _route == none { (:) } else { _route.params }

#let get-entry(collection, id) = {
  _collections.at(collection, default: (:)).at(id, default: none)
}

#let releases() = {
  _collections.at("releases", default: (:))
    .values()
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.metadata.date)
    .rev()
}

#let date-value(value) = {
  let parts = value.split("-").map(int)
  datetime(year: parts.at(0), month: parts.at(1), day: parts.at(2))
}

#let date-label(value) = date-value(value).display("[month repr:short] [day], [year]")
