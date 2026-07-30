#metadata((
  title: "Template file names define output routes.",
  section: "Reference",
  order: 30,
  summary: "This reference explains how static, parameterized, mixed, and spread templates define output routes.",
)) <frontmatter>

= Template file names define output routes.

Every `.typ` file under `src/` is a page template. Brackets in its relative path
declare route parameters.

#table(
  columns: (1.1fr, 1fr),
  table.header[
    *The template has this path.*
  ][
    *Aster writes this output.*
  ],
  [`about.typ`], [`about.html`],
  [`journal/[slug].typ`], [`journal/<slug>.html`],
  [`work/[year]-[slug].typ`], [`work/<year>-<slug>.html`],
  [`docs/[...path].typ`], [The output preserves the nested path below `docs/`.],
)

Dynamic templates declare their parameter sets through labelled metadata:

```typ
#metadata(
  get-collection-ids("docs").map(path => (path: path))
) <route>
```

Aster validates missing parameters, extra parameters, unsafe segments, and
output collisions before rendering pages.
