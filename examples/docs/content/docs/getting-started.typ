#import "/components/content.typ": callout, steps

#metadata((
  title: "Getting Started",
  description: "Create and run an Aster documentation project.",
  section: "Start Here",
  section_order: 10,
  order: 20,
)) <aster-frontmatter>

This guide covers the shortest path from a checked-out Aster repository to a
locally served documentation site.

= Prerequisites

You need a Rust toolchain capable of building the workspace. Aster includes the
development server and CSS processing pipeline, so the basic documentation
example does not require Node.js.

= Create a project

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
cargo run -- dev -p examples/docs
```

#callout(kind: "note")[
  Use `cargo run -- build -p examples/docs` when you only need the static
  output under `examples/docs/dist/`.
]

= Project structure

```text
docs/
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
