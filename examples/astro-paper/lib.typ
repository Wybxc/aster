#let _content-state = sys.inputs.at("_aster", default: none)
#let _collections = if _content-state == none {
  // Tinymist evaluates files without Aster's injected content protocol.
  (:)
} else {
  assert(
    _content-state.protocol == 4,
    message: "incompatible content protocol with the Aster binary",
  )
  _content-state.collections
}

#let settings = if "site" in sys.inputs {
  sys.inputs
} else {
  toml("/aster.toml")
}

#let get-collection(name) = {
  _collections.at(name, default: (:)).values().sorted(key: entry => entry.id)
}

#let get-collection-ids(name) = {
  _collections.at(name, default: (:)).keys().sorted()
}

#let get-entry(collection, id) = {
  _collections.at(collection, default: (:)).at(id, default: none)
}

#let tag-slug(tag) = lower(tag).replace(" ", "-")

#let post-url(id) = "/posts/" + id + "/"

#let canonical-url(path) = {
  if path == "/" {
    settings.site.url
  } else {
    settings.site.url + path.trim("/") + "/"
  }
}

#let date-value(value) = {
  let parts = value.slice(0, 10).split("-").map(int)
  datetime(year: parts.at(0), month: parts.at(1), day: parts.at(2))
}

#let date-label(value) = date-value(value).display("[month repr:long] [day], [year]")

#let _all-posts() = {
  get-collection("posts")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .filter(item => not item.metadata.draft)
    .sorted(key: item => item.metadata.date)
    .rev()
}

#let published-posts() = {
  let today = datetime.today().display("[year]-[month padding:zero]-[day padding:zero]")
  _all-posts().filter(item => item.metadata.date.slice(0, 10) <= today)
}

#let posts-with-tag(tag) = {
  published-posts().filter(item => item.metadata.tags.any(value => tag-slug(value) == tag))
}

#let all-tags() = {
  let tags = (:)
  for item in published-posts() {
    for tag in item.metadata.tags {
      tags.insert(tag-slug(tag), tag)
    }
  }
  tags.pairs()
    .map(pair => (
      slug: pair.first(),
      name: pair.last(),
      count: posts-with-tag(pair.first()).len(),
    ))
    .sorted(key: tag => tag.slug)
}

#let page-count(items, per-page: settings.posts.per-page) = {
  calc.ceil(items.len() / per-page)
}

#let page-items(items, page, per-page: settings.posts.per-page) = {
  let start = (page - 1) * per-page
  let end = calc.min(start + per-page, items.len())
  if start >= items.len() { () } else { items.slice(start, end) }
}

#let adjacent-posts(id) = {
  let posts = published-posts()
  let index = posts.position(item => item.entry.id == id)
  if index == none {
    (newer: none, older: none)
  } else {
    (
      newer: if index > 0 { posts.at(index - 1) } else { none },
      older: if index + 1 < posts.len() { posts.at(index + 1) } else { none },
    )
  }
}
