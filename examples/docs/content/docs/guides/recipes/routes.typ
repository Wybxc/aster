#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Dynamic Routes from Content",
  description: "Turn collection membership into page route parameters.",
  section: "Guides",
  section_order: 20,
  order: 38,
)) <aster-frontmatter>

The route template declares its parameter names; the page supplies the values.
For `pages/posts/[...slug]/index.typ`, a collection can provide nested slugs:

```typ
#import "/lib.typ": get-collection-ids

#metadata(
  get-collection-ids("posts").map(id => (slug: id))
) <aster-route>
```

The spread parameter is a complete path segment, so `guides/start` becomes
`/posts/guides/start/`. A normal parameter must remain one segment.

= Pagination routes

For a route such as `pages/posts/[page]/index.typ`, emit one parameter set for
every page before rendering:

```typ
#import "/lib.typ": published-posts, page-count

#let settings = toml("/aster.toml")
#let posts = published-posts()
#let total = page-count(posts, settings.posts.per-page)

#metadata(
  range(1, total + 1).map(page => (page: str(page)))
) <aster-route>
```

The page reads its current route parameter from `_aster.route.params` and
selects its slice. A project may choose a different URL convention for the
first page; the route template and link helper should use the same convention.

= Taxonomy routes

The same pattern works for tags and archives. First calculate the distinct
values in the project library, then map them to the route parameter:

```typ
#import "/lib.typ": all-tags, published-posts

#metadata(
  all-tags(published-posts()).map(tag => (tag: tag.slug))
) <aster-route>
```

Keep slug normalization in one project function so links and route parameters
cannot drift apart. Route parameters are validated URL path data, not native
filesystem paths.

#callout(kind: "tip", title: "Use ids for discovery")[
  If a route only needs collection membership, call `get-collection-ids()`.
  Calling `metadata()` during route discovery creates dependencies on each
  entry and may do more work than the route needs.
]
