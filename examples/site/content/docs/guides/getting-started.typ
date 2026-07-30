#metadata((
  title: "An Aster project needs only a few files.",
  section: "Guides",
  order: 10,
  summary: "This guide explains how to build the example once or rebuild it while files change.",
)) <frontmatter>

= An Aster project needs only a few files.

An Aster project contains `aster.toml`, page templates under `src/`, and
optional content collections under `content/`.

```text
site/
├── aster.toml
├── site.typ
├── content/
│   ├── journal/
│   ├── projects/
│   └── docs/
└── src/
    ├── index.typ
    ├── journal/[slug].typ
    └── docs/[...path].typ
```

The following command builds the site once:

```sh
cargo run -- build -p examples/site
```

The following command keeps the Typst session and comemo cache alive while
project files change:

```sh
cargo run -- watch -p examples/site
```

Successful publication writes a complete deterministic tree to `dist/`.
