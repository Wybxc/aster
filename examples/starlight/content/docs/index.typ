#import "/components/content.typ": aside, card-grid, doc-heading

#metadata((
  title: "Welcome to Aster Docs",
  description: "Build a fast documentation site with Typst-native content and static output.",
  section: "Start Here",
  section_order: 10,
  order: 10,
  badge: "New",
  toc: (
    (id: "overview", title: "Overview", level: 2),
    (id: "where-to-next", title: "Where to next?", level: 2),
  ),
)) <aster-frontmatter>

Aster combines file-based routes, lazy content collections, and Typst's content
model. The result is a static documentation site that keeps authoring and
presentation in the same language.

#aside(kind: "tip", title: "A small core")[
  Start with ordinary Typst files. Add specialized components only where their
  structure makes the document easier to read.
]

#doc-heading(id: "overview")[Overview]

Use familiar Typst markup for prose and switch to functions when a documentation
component needs structure or behavior.

#card-grid((
  (
    title: "Start building",
    href: "/getting-started/",
    body: [Create a project, run the development server, and publish the generated files.],
  ),
  (
    title: "Author content",
    href: "/guides/authoring-content/",
    body: [Write headings, code, callouts, tabs, and reusable components in Typst.],
  ),
  (
    title: "Understand routing",
    href: "/reference/routing/",
    body: [Map static and dynamic page templates to deterministic output paths.],
  ),
))

#doc-heading(id: "where-to-next")[Where to next?]

Begin with the setup guide if this is your first Aster project. If the site is
already running, continue with content authoring and component composition.

#aside(kind: "note", title: "Static by default")[
  Every documentation route is rendered during the build. The generated site
  can be deployed at a domain root or below a path prefix without rebuilding it.
]
