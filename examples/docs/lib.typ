#let _content-state = sys.inputs.at("_aster", default: none)
#let _collections = if _content-state == none {
  // Tinymist evaluates source files without Aster's runtime protocol.
  (:)
} else {
  assert(
    _content-state.protocol == 9,
    message: "incompatible runtime protocol with the Aster binary",
  )
  _content-state.collections
}
#let aster-version = if _content-state == none { none } else { _content-state.version }
#let route-path() = if _content-state == none {
  "/"
} else {
  _content-state.route.path(default: "/")
}
#let route-param(name, default: none) = if _content-state == none {
  default
} else {
  _content-state.route.param(name, default: default)
}
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

#let doc-url(id) = if id == "index" { "/" } else { "/" + id + "/" }

// Longest common leading path segments shared by a list of nested ids.
#let _common-prefix(ids) = {
  let segments = ids.map(id => id.split("/"))
  let prefix = ()
  for (i, segment) in segments.first().enumerate() {
    if segments.all(s => s.len() > i and s.at(i) == segment) {
      prefix.push(segment)
    } else {
      break
    }
  }
  prefix
}

// Humanize a folder segment into a sidebar label.
#let _folder-label(segment) = {
  segment.split("-").map(word => upper(word.at(0)) + word.slice(1)).join(" ")
}

// A navigation node can be a page, a folder, or both (a page whose id is also
// the parent of deeper entries, such as the Recipes overview).
#let _nav-node(page, children) = (
  label: if page == none { "" } else { page.metadata.title },
  href: if page == none { none } else { doc-url(page.id) },
  id: if page == none { none } else { page.id },
  badge: if page == none { none } else { page.metadata.at("badge", default: none) },
  order: if page == none { 0 } else { page.metadata.order },
  children: children,
)

// Build a navigation tree from entries reduced to (remaining segments, item).
#let _build-tree(entries) = {
  let groups = (:)
  let page = none
  for (segments, item) in entries {
    if segments.len() == 0 {
      page = item
    } else {
      let key = segments.first()
      if key not in groups {
        groups.insert(key, ())
      }
      groups.at(key).push((segments.slice(1), item))
    }
  }

  let children = groups.pairs()
    .map(pair => {
      let node = _build-tree(pair.last())
      if node.id != none {
        node
      } else {
        (label: _folder-label(pair.first()), href: none, id: none, badge: none, order: 0, children: node.children)
      }
    })
    .sorted(key: node => node.order)

  _nav-node(page, children)
}

// Sections are the top-level navigation groups. Within each section, entries
// nest by their id path, so content subfolders become nested branches.
#let docs-by-section() = {
  let sections = (:)
  for item in docs() {
    let name = item.metadata.section
    if name not in sections {
      sections.insert(name, ())
    }
    sections.at(name).push(item)
  }

  sections.pairs()
    .map(pair => {
      let (label, group) = pair
      let prefix = _common-prefix(group.map(item => item.id))
      let root = _build-tree(group.map(item => (item.id.split("/").slice(prefix.len()), item)))
      (label: label, order: group.first().metadata.section_order, children: root.children)
    })
    .sorted(key: section => section.order)
}

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
