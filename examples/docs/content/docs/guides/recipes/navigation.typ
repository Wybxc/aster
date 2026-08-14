#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Adjacent Navigation",
  description: "Create previous and next links from an ordered content query.",
  section: "Guides",
  section_order: 20,
  order: 41,
)) <aster-frontmatter>

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

The page supplies the ordered query and current id:

```typ
#import "/lib.typ": post-url, published-posts

#let neighbors = adjacent(published-posts(), current-id)
#if neighbors.previous != none {
  html.a(href: post-url(neighbors.previous.entry.id))[Previous]
}
#if neighbors.next != none {
  html.a(href: post-url(neighbors.next.entry.id))[Next]
}
```

The returned value is data. A separate HTML function can render it as links,
buttons, breadcrumbs, or another navigation pattern. Keeping the query apart
from the HTML makes it reusable in archives and feeds.

#callout(kind: "tip", title: "Define one ordering")[
  Use the same ordering function for the index, adjacent links, Atom entries,
  and archives. If different pages sort the same collection differently, the
  meaning of "previous" becomes project-specific and should be named as such.
]
