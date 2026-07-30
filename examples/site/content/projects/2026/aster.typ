#metadata((
  title: "Aster",
  year: "2026",
  slug: "aster",
  status: "active",
  summary: "Aster generates static sites from Typst while publishing deterministically and rebuilding pages incrementally.",
  stack: ("Rust", "Typst", "comemo", "Lightning CSS"),
)) <frontmatter>

= Aster

Aster turns Typst page templates into a complete static output tree. The build
pipeline separates route planning, compilation, document transforms, and final
publication so repeated builds remain predictable.

== Aster follows four design constraints.

- Content stays readable by Typst tools and language servers.
- Filesystem access passes through tracked project services.
- Dynamic routes are validated before any page is published.
- Generated CSS and images use content-addressed names.

#html.frame[
  #block(fill: rgb("e8f4f1"), inset: 12pt, radius: 4pt)[
    #set text(fill: rgb("17443c"))
    #set par(justify: false)
    #stack(
      dir: ttb,
      spacing: 4pt,
      [Aster compiles each Typst source through a tracked world.],
      [It then publishes the resulting static HTML.],
    )
  ]
]
