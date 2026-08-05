#import "/components/content.typ": aside, doc-heading

#metadata((
  title: "Routing Reference",
  description: "Map page templates and route parameters to output files.",
  section: "Reference",
  section_order: 30,
  order: 10,
  toc: (
    (id: "static-routes", title: "Static routes", level: 2),
    (id: "dynamic-routes", title: "Dynamic routes", level: 2),
    (id: "spread-parameters", title: "Spread parameters", level: 3),
    (id: "route-context", title: "Route context", level: 2),
  ),
)) <aster-frontmatter>

Aster derives output routes from Typst files under `pages/`. Files named
`index.typ` produce directory index files; other names produce exact files.

#doc-heading(id: "static-routes")[Static routes]

```text
pages/index.typ             -> /index.html
pages/about/index.typ       -> /about/index.html
pages/robots.txt.typ        -> /robots.txt
```

The development server serves exact output files. `/about/` resolves through
the generated `about/index.html`; `/about` does not imply a redirect.

#doc-heading(id: "dynamic-routes")[Dynamic routes]

Bracketed path segments declare parameters. A route template first emits
`<aster-route>` metadata during probing, then Aster compiles it once for every
parameter dictionary.

```typ
#metadata(((slug: "first-post"), (slug: "second-post"))) <aster-route>
```

#doc-heading(level: 3, id: "spread-parameters")[Spread parameters]

`[...slug]` accepts nested values such as `guides/installation`. This example
uses one spread template to render all non-root documentation entries.

#aside(kind: "caution")[
  Generated parameter values remain URL path segments. Do not use them as
  unchecked native filesystem paths.
]

#doc-heading(id: "route-context")[Route context]

The `_aster.route` dictionary exposes the current browser path and parameter
values. `_aster.routes` contains the complete planned page and endpoint sets for
navigation, feeds, and sitemaps.
