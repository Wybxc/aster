#import "/components/content.typ": aside, doc-heading, steps

#metadata((
  title: "Getting Started",
  description: "Create and run an Aster documentation project.",
  section: "Start Here",
  section_order: 10,
  order: 20,
  toc: (
    (id: "prerequisites", title: "Prerequisites", level: 2),
    (id: "create-a-project", title: "Create a project", level: 2),
    (id: "project-structure", title: "Project structure", level: 2),
  ),
)) <aster-frontmatter>

This guide covers the shortest path from a checked-out Aster repository to a
locally served documentation site.

#doc-heading(id: "prerequisites")[Prerequisites]

You need a Rust toolchain capable of building the workspace. Aster includes the
development server and CSS processing pipeline, so the basic documentation
example does not require Node.js.

#doc-heading(id: "create-a-project")[Create a project]

#steps((
  (
    title: "Build the Aster binary",
    body: [Run the command from the repository root.],
  ),
  (
    title: "Start the documentation site",
    body: [Point the development command at this example project.],
  ),
  (
    title: "Open the local URL",
    body: [The server rebuilds changed Typst, CSS, JavaScript, and asset dependencies.],
  ),
))

```sh
cargo build
cargo run -- dev -p examples/starlight
```

#aside(kind: "note")[
  Use `cargo run -- build -p examples/starlight` when you only need the static
  output under `examples/starlight/dist/`.
]

#doc-heading(id: "project-structure")[Project structure]

```text
starlight/
|-- aster.toml
|-- lib.typ
|-- components/
|-- content/docs/
|-- pages/
|-- scripts/
|-- styles/
`-- templates/
```

Route templates stay separate from documentation entries. The spread route
discovers entries from the `docs` collection and compiles one page for each id.
