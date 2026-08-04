#metadata((
  title: "An Aster project needs only a few files.",
  section: "Guides",
  order: 10,
  summary: "This guide explains how to build the example once or rebuild it while files change.",
)) <frontmatter>

= An Aster project needs only a few files.

An Aster project contains `aster.toml`, route templates under `pages/`, and
optional content, styles, assets, and public files in separate directories.

```text
site/
├── aster.toml
├── lib.typ
├── components/
│   └── navigation.typ
├── templates/
│   └── site.typ
├── assets/
├── content/
│   ├── journal/
│   ├── projects/
│   └── docs/
├── styles/
│   └── site.css
└── pages/
    ├── index.typ
    ├── rss.xml.typ
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
