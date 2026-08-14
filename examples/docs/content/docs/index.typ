#import "/components/content.typ": callout, card-grid

#metadata((
  title: "Welcome to Aster Docs",
  description: "Build static sites with Typst-native pages, content, and assets.",
  section: "Start Here",
  section_order: 10,
  order: 10,
  badge: "0.1",
)) <aster-frontmatter>

Aster compiles Typst-authored projects into complete static output trees. Page
routes, lazy content collections, transformed resources, exact-path generators,
and staged postprocessing share one deterministic build.

#callout(kind: "tip", title: "Start with ordinary Typst")[
  Use Typst markup for prose, scripting for data and composition, and typed HTML
  elements when browser semantics matter. Add Aster-specific labels only at the
  boundaries that need routing, metadata, resources, or generated files.
]

= Learn the workflow

#card-grid((
  (
    title: "Start building",
    href: "/getting-started/",
    body: [Create a project, run the development server, and understand the output boundary.],
  ),
  (
    title: "Author content",
    href: "/guides/authoring-content/",
    body: [Write headings, code, callouts, tabs, and semantic browser content in Typst.],
  ),
  (
    title: "Build collections",
    href: "/guides/content-collections/",
    body: [Load metadata and rendered entries lazily from collection directories.],
  ),
  (
    title: "Compose recipes",
    href: "/guides/recipes/",
    body: [Build taxonomies, pagination, navigation, feeds, and indexes from Aster primitives.],
  ),
  (
    title: "Publish resources",
    href: "/guides/assets-and-processing/",
    body: [Bundle CSS and scripts, optimize images, and colocate component resources.],
  ),
  (
    title: "Understand routing",
    href: "/reference/routing/",
    body: [Map static and dynamic templates to deterministic output paths.],
  ),
  (
    title: "Generate files",
    href: "/reference/generators-and-postprocessing/",
    body: [Produce feeds and sitemaps from rendered pages, then run external tools on staging.],
  ),
))

= Build model

Aster plans all page routes, renders and transforms each page, then evaluates
generators against the final page snapshot. The complete candidate site is
staged before optional external postprocessors run. Publication replaces the
previous output only after every required stage succeeds.

#callout(kind: "note", title: "Static and relocatable")[
  Every route is rendered during the build. Root-relative navigation is rewritten
  per output page, so the same generated tree can be served at a domain root or
  below a path prefix without rebuilding it.
]
