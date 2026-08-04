#metadata((
  title: "Typst is the template language.",
  date: "2026-07-24",
  summary: "Typst provides markup, functions, data, mathematics, and HTML elements within one language.",
  tags: ("typst", "html", "templates"),
)) <frontmatter>

= Typst is the template language.

Aster does not place a second template language around Typst. Page modules and
content entries can use the same expressions, functions, imports, and semantic
markup.

== Typst evaluates generated content during compilation.

#let releases = (
  (version: "0.1", focus: "This release made publication deterministic."),
  (version: "0.2", focus: "This release added tracked file access."),
  (version: "0.3", focus: "This release introduced lazy content modules."),
)

#table(
  columns: (1fr, 2fr),
  table.header[
    *The release has this version.*
  ][
    *The release introduced this behavior.*
  ],
  ..releases.map(release => (
    [#release.version],
    [#release.focus],
  )).flatten(),
)

== Typst preserves mathematics and document structure.

The same source can contain inline mathematics such as
$sum_(k=1)^n k = (n(n+1))/2$, together with structured tables, figures, and raw
HTML elements.

```typ
#import "/templates/site.typ": site

#show: site.with(title: "This page contains the release notes.")

= This page contains the release notes.
The page is written with ordinary Typst markup.
```

The HTML target preserves semantic headings, lists, tables, links, and MathML.
Elements that require layout can opt into `html.frame`; explicit `html.elem`
calls are reserved for semantic boundaries that Typst markup does not express.
