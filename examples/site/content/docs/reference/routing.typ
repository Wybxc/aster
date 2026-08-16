#metadata((
  title: "Template file names define output routes.",
  section: "Reference",
  order: 30,
  summary: "This reference explains how static, parameterized, mixed, and spread templates define output routes.",
)) <aster-frontmatter>

= Template file names define output routes.

Every `.typ` file under `pages/` is an HTML page template. Every `.typ` file
under `generate/` is a generator whose `<aster-output>` metadata contains one
string or bytes result. Brackets in either template path declare route parameters.

#table(
  columns: (1.1fr, 1fr),
  table.header[
    *The template has this path.*
  ][
    *Aster writes this output.*
  ],
  [`about.typ`], [`about.html`],
  [`about/index.typ`], [`about/index.html`],
  [`journal/[slug]/index.typ`], [`journal/<slug>/index.html`],
  [`work/[year]-[slug]/index.typ`], [`work/<year>-<slug>/index.html`],
  [`docs/[...path]/index.typ`], [The output preserves the nested path below `docs/`.],
)

Dynamic templates declare their parameter sets through labelled metadata:

```typ
#metadata(
  get-collection-ids("docs").map(path => (path: path))
) <aster-route>
```

Aster validates missing parameters, extra parameters, unsafe segments, and
output collisions before rendering pages.

During the final compilation of each route, Aster exposes its preferred browser
path and parameters through native functions in `sys.inputs._aster.route`.
Shared project libraries can provide lazy accessors while retaining editor
fallbacks:

```typ
#let state = sys.inputs.at("_aster", default: none)
#let route-path() = if state == none {
  "/"
} else {
  state.route.path(default: "/")
}
#let route-param(name, default: none) = if state == none {
  default
} else {
  state.route.param(name, default: default)
}
```

Keeping the accessors as functions lets every compilation read its own route
without changing the shared Typst library. During dynamic route discovery, the
native functions return their supplied defaults.

For pages, root and nested `index.html` outputs become `/` and directory URLs;
file-shaped pages retain their `.html` suffix. Generators remove only their
final `.typ` extension and use that exact output path. The same protocol exposes
all planned page paths through `sys.inputs._aster.routes.pages()`, so generated
artifacts such as sitemaps do not need to duplicate route discovery.

== Site-root navigation

A single leading slash in navigation denotes the generated site's virtual root:

```typ
#link("/")[Home]
#link("/docs/reference/routing/")[Routing]
#html.elem("form", attrs: (action: "/search/",))[]
```

Aster rewrites these links, image-map areas, and form actions relative to each
output page. A nested page may therefore contain `../../docs/reference/routing/`
in its final HTML. The generated tree remains valid whether it is served from
the domain root or mounted below a path such as `/notes/`. Explicit relative
URLs, fragments, queries, protocol URLs, and `//` references remain unchanged.

During development, `aster dev` serves the root `404.html` with a 404 status
when a requested file does not exist. Define `pages/404.typ` to provide this
page.
