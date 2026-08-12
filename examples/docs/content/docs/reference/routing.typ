#import "/components/content.typ": callout

#metadata((
  title: "Routing Reference",
  description: "Map page templates and declared parameters to portable output paths.",
  section: "Reference",
  section_order: 30,
  order: 10,
)) <aster-frontmatter>

Every `.typ` file below `pages/` is an HTML page template. Aster removes its
final `.typ` extension and appends `.html`; there is no separate clean-URL
setting.

= Static routes

```text
pages/index.typ             → /index.html             browser path /
pages/about/index.typ       → /about/index.html       browser path /about/
pages/about.typ             → /about.html             browser path /about.html
pages/robots.txt.typ        → /robots.txt.html         browser path /robots.txt.html
```

Use explicit `index.typ` directories when a browser URL should end in `/`.
The development server serves exact files and directory indexes; `/about` does
not imply a redirect to `/about/`.

= Dynamic routes

`[name]` fills one path segment. A template containing parameters is first
compiled without route context and must emit an array of parameter dictionaries
through `<aster-route>` metadata:

```typ
#metadata((
  (year: "2025", slug: "first"),
  (year: "2026", slug: "second"),
)) <aster-route>
```

For `pages/posts/[year]-[slug]/index.typ`, these values generate
`/posts/2025-first/` and `/posts/2026-second/`. Every dictionary must provide
exactly the parameters named by the template.

== Spread parameters

A spread parameter must occupy a complete segment. `[...slug]` accepts a nested
value such as `guides/installation`; normal parameters cannot contain `/`.

```typ
#metadata(
  get-collection-ids("docs")
    .filter(id => id != "index")
    .map(slug => (slug: slug))
) <aster-route>
```

Generated segments must be nonempty, portable across common filesystems, and
cannot contain `.` or `..` path components. Route planning rejects exact,
case-insensitive, and file-versus-directory collisions before publication.

= Route context

During a concrete page compilation, `sys.inputs._aster.route` is:

```typc
(
  path: "/posts/first/",
  params: (slug: "first"),
)
```

`route` is `none` during editor evaluation and the dynamic probe. Templates
should tolerate that state when they are evaluated outside a concrete route.
`_aster.routes.pages` contains all planned page browser paths in deterministic
order; generator outputs are intentionally absent from that navigation graph.

= Navigation URLs

A single leading slash in links, image-map areas, and form actions denotes the
generated site root. Aster rewrites it relative to each page output, making the
same tree deployable below a subdirectory. Explicit relative URLs, protocol
URLs, `//` references, fragments, and query-only references are preserved.

#callout(kind: "caution")[
  Route parameters are validated URL path data, not native filesystem paths.
  Do not pass them to unrelated filesystem APIs without defining a separate
  project-specific invariant.
]
