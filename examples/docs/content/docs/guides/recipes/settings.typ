#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Project Settings and URLs",
  description: "Read project-owned settings and construct stable page URLs.",
  section: "Guides",
  section_order: 20,
  order: 36,
)) <aster-frontmatter>

Project settings stay in `aster.toml`, including fields that Aster does not
interpret. Read them from Typst instead of adding parallel inputs:

```typ
#let settings = toml("/aster.toml")
#let site-name = settings.site.title
#let site-url = settings.site.url
```

Keep the site URL ending in `/` when recipes concatenate it with a route path.
Page paths supplied by `_aster` start with `/`:

```typ
#let absolute-url(path) = settings.site.url + path.trim("/")
#let absolute-page-url(path) = settings.site.url + path.trim("/") + "/"
```

Use the page helper only for page routes. Generator files such as
`/atom.xml` should not receive a trailing slash:

```typ
#let feed-url = settings.site.url + "atom.xml"
```

The exact joining policy is project-owned. A recipe that accepts both page and
file URLs should preserve whether the route ends in `/` instead of blindly
adding one.

= Relocatable navigation

A single leading slash in links, image-map areas, and form actions denotes the
generated site's virtual root. Aster rewrites these references relative to the
output page, so the same tree can be served at a domain root or under a path
prefix. Explicit relative URLs, protocol URLs, `//` references, fragments, and
query-only references are preserved.

For URLs that are not ordinary navigation, use a project helper rather than
assuming that a leading slash includes a deployment prefix. This matters for
feed discovery links, canonical links, and sitemap locations.

#callout(kind: "caution", title: "Keep URL policy in one function")[
  The blog and journal examples both construct absolute feed URLs from
  `settings.site.url`. Keep that operation in one project function so route
  slashes and the configured site URL cannot drift apart.
]
