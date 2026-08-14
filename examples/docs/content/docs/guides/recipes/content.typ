#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Content Queries",
  description: "Build project-owned collection queries without adding a Rust-side content type.",
  section: "Guides",
  section_order: 20,
  order: 37,
)) <aster-frontmatter>

The default template exposes three small helpers. They return lazy entry
modules; reading `metadata()` or `render()` creates the corresponding source
dependency:

```typ
#import "/lib.typ": get-collection, get-entry

#let entries = get-collection("posts")
#let first-post = get-entry("posts", "hello")

#if first-post != none [
  #let data = first-post.metadata()
  #html.h1[#data.title]
  #first-post.render()
]
```

When a route only needs membership, use ids rather than metadata or content:

```typ
#import "/lib.typ": get-collection-ids

#metadata(
  get-collection-ids("posts").map(id => (slug: id))
) <aster-route>
```

This keeps the route probe independent from every entry body. Nested files keep
their path below the collection as the id:

```text
content/posts/hello.typ            -> "hello"
content/posts/guides/start.typ    -> "guides/start"
```

= Filter and sort entries

Metadata is project-defined. A publication can build a published view without
introducing a Rust-side post type:

```typ
#import "/lib.typ": get-collection

#let published-posts() = {
  let today = datetime.today().display(
    "[year]-[month padding:zero]-[day padding:zero]",
  )
  get-collection("posts")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .filter(item => not item.metadata.at("draft", default: false))
    .filter(item => item.metadata.date.slice(0, 10) <= today)
    .sorted(key: item => item.metadata.date)
    .rev()
}
```

Keep this function in the project's `lib.typ`. Other projects may use a
different draft policy, date format, or ordering key.

= Adjacent data

An ordered collection can find neighboring entries with `position`:

```typ
#let adjacent(items, id) = {
  let index = items.position(item => item.entry.id == id)
  if index == none {
    (previous: none, next: none)
  } else {
    (
      previous: if index > 0 { items.at(index - 1) } else { none },
      next: if index + 1 < items.len() { items.at(index + 1) } else { none },
    )
  }
}
```

The returned value is data. A separate HTML function can render it as links,
buttons, or another navigation pattern.

#callout(kind: "note", title: "Protocol scope")[
  `_aster.collections` transports entries and their lazy accessors. It does
  not decide what a post, draft, tag, archive, or translation is. Those rules
  belong in the project's Typst library.
]
