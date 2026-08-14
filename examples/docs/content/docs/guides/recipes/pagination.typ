#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Pagination",
  description: "Split an ordered query into stable page routes and render a navigation control.",
  section: "Guides",
  section_order: 20,
  order: 39,
)) <aster-frontmatter>

Pagination is a pair of ordinary functions plus a dynamic route. Keep the
calculation independent from the HTML navigation function:

```typ
#let page-count(items, per-page) = calc.ceil(items.len() / per-page)

#let page-items(items, page, per-page) = {
  let start = (page - 1) * per-page
  let end = calc.min(start + per-page, items.len())
  if start >= items.len() { () } else { items.slice(start, end) }
}
```

For `pages/posts/[page]/index.typ`, emit the complete parameter set:

```typ
#import "/lib.typ": published-posts

#let settings = toml("/aster.toml")
#let posts = published-posts()
#let total = page-count(posts, settings.posts.per-page)

#metadata(
  range(1, total + 1).map(page => (page: str(page)))
) <aster-route>
```

Read the current page from the route context and select its items:

```typ
#import "/lib.typ": published-posts, route-params

#let settings = toml("/aster.toml")
#let posts = published-posts()
#let page = int(route-params.at("page", default: "1"))
#let items = page-items(posts, page, settings.posts.per-page)
```

= Render pagination

The query and the HTML control should remain separate. This makes the same
pagination data usable for a compact list, a full archive, or a JSON generator.

```typ
#let page-href(base, page) = if page == 1 {
  base
} else {
  base + str(page) + "/"
}

#let pagination(page, total, base) = if total > 1 [
  #html.nav(aria-label: "Pagination")[
    #if page > 1 {
      html.a(href: page-href(base, page - 1))[Previous]
    }
    #html.span[Page #page of #total]
    #if page < total {
      html.a(href: page-href(base, page + 1))[Next]
    }
  ]
]
```

The journal example adds component-owned CSS and icons to this function. Those
visual choices are not required by the pagination recipe.

#callout(kind: "caution", title: "Empty collections")[
  Decide what an empty query means before calculating the page count. A
  project may emit no route, a single empty page, or a not-found page. Keep
  that policy explicit rather than allowing a zero page count to create an
  invalid route set.
]
