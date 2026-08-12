#import "/components/content.typ": callout, steps

#metadata((
  title: "Getting Started",
  description: "Create, build, and serve an Aster project.",
  section: "Start Here",
  section_order: 10,
  order: 20,
)) <aster-frontmatter>

This guide covers the shortest path from an Aster project to a locally served
site, then explains where each kind of source belongs.

= Prerequisites

You need the `aster` executable. Building this repository requires a Rust
toolchain; an installed binary has no Node.js dependency. Tailwind stylesheets
need the separate `tailwindcss` CLI, and ES modules need `esbuild`, but neither
tool is required for ordinary Typst, CSS, or classic scripts.

= Create and run a project

#steps((
  (
    title: "Create the project",
    body: [Run `aster init my-site` with an empty or nonexistent destination.],
  ),
  (
    title: "Start the development server",
    body: [Enter the project and run `aster dev`. The default address is `http://127.0.0.1:4321/`.],
  ),
  (
    title: "Edit a page",
    body: [Change `pages/index.typ`. A successful rebuild refreshes the browser automatically.],
  ),
))

```sh
aster init my-site
cd my-site
aster dev
```

#callout(kind: "note")[
  In this repository, run `cargo run -- dev -p examples/docs` to serve this
  documentation example without installing the binary first.
]

= Project structure

```text
my-site/
|-- aster.toml
|-- lib.typ
|-- assets/
|-- components/
|-- content/
|-- generate/
|-- pages/
|-- public/
|-- scripts/
|-- styles/
`-- templates/
```

Only `aster.toml` and `pages/` are required. The conventional directories have
separate roles:

- `pages/` contains Typst templates that compile to HTML pages.
- `generate/` contains Typst programs that emit exact-path non-page files.
- `content/<collection>/` contains lazily imported content entries.
- `styles/`, `scripts/`, and `assets/` contain processed project resources.
- `public/` is copied unchanged to the output root.
- `dist/` is the complete generated site and is replaced after a successful build.

This documentation site keeps route templates separate from documentation
entries. Its spread route discovers the `docs` collection and compiles one page
for every nested entry id.

= Commands

```text
aster init [path]
aster build [-p project] [-v|-vv]
aster watch [-p project] [-v|-vv]
aster dev [-p project] [--host 127.0.0.1] [--port 4321] [-v|-vv]
```

Without `-p`, build commands find the nearest ancestor containing
`aster.toml`. `build` runs once, `watch` rebuilds on dependency changes, and
`dev` adds a static server with browser reload. The server resolves `/` and
directory requests such as `/guide/` to their `index.html` files and serves a
project-provided `404.html` when present.

The default log level shows stages and rendered routes. `-v` includes detailed
build operations; `-vv` includes ordinary resource processing.

= Build guarantees

Aster collects a complete candidate output, writes it to a temporary staging
tree, runs configured postprocessors, and only then replaces `dist/`. A failed
compile, transform, generator, or postprocessor therefore preserves the last
successful output. Replacing the prior tree is the final filesystem boundary, so
an operating-system rename failure can still leave no output directory. Warnings
are reported but do not fail the build.
