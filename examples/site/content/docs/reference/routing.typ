#metadata((
  title: "Template file names define output routes.",
  section: "Reference",
  order: 30,
  summary: "This reference explains how static, parameterized, mixed, and spread templates define output routes.",
)) <aster-frontmatter>

= Template file names define output routes.

Every `.typ` file under `pages/` is a route template. Templates with
`<aster-endpoint>` metadata generate arbitrary files; all others generate HTML pages.
Brackets in a template's relative path declare route parameters.

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

During development, `aster dev` serves the root `404.html` with a 404 status
when a requested file does not exist. Define `pages/404.typ` to provide this
page.
